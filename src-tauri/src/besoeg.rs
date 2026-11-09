//! Besøgende (V-BESOEG.1): unikke besøgende pr. dag et år tilbage og besøg pr. land. Unikke besøgende er årstallene (`historik`, art `h_total`, nøgle
//! `unikke`, Cloudflares `uniq { uniques }`: unikke IP-adresser, kun en total pr. dag). Besøg er `sum { visits }` på `httpRequestsAdaptiveGroups`
//! med `requestSource:"eyeball"` (Cloudflares egen definition: en sidevisning, der kom fra et andet site eller et direkte link), pr. dag og land.
//! Cloudflare har dem i ca. en måned; programmet gemmer dagene (arterne `b_land` og `b_total`, `hentet_besoeg`), så historikken vokser derfra.
//! Hentes i `koe::Koe::ekstra_runde` efter årstallene: ét kald pr. domæne for de færdige døgn, der mangler. Afviser Cloudflare forespørgslen, noteres det
//! ved domænet (`store::BESOEG_FEJL`, "ikke tilgængelig"), og der spørges ikke igen, før programmet starter igen. Ingen IP-adresser, stier eller
//! brugeragenter: kun dag, land og et tal. Visningens tal regnes her (`laes`, `regn`), kun fra cachen.
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cf::Fejl;
use crate::dyk::{periode_ud, Periode, PeriodeUd};
use crate::historik::UNIKKE;
use crate::robotter::dage_i;
use crate::store::{Raekke, Store, BESOEG_FEJL};
use crate::vaern::{dag_ms, dag_noegle, er_faerdig, iso, tom_ved_null, DOEGN_MS};

/// Så mange døgn tilbage hentes besøgene. `httpRequestsAdaptiveGroups` har `notOlderThan` 31 dage og `maxDuration` 30 dage på Free og Pro (bilag A),
/// så døgnet for 31 dage siden er altid delvis for gammelt: d-30…d-1 er det, ét kald kan dække.
pub const BESOEG_DAGE: i64 = 30;
/// Så mange dage dækker "et år" i visningen, og så langt kan Fra/Til spænde.
pub const AAR_DAGE: i64 = 365;

/// Kun zone og tider er variabler. Ingen `orderBy`: 30 døgn gange alle lande er under grænsen, og svaret lægges sammen her.
pub const BESOEG_QUERY: &str = r#"query Besoeg($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    besoeg: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball"}) { sum { visits } dimensions { date clientCountryName } }
  } } }"#;

/// De færdige døgn i hentevinduet, der mangler: (ældste, nyeste). Ét kald dækker alt derimellem. Ren funktion.
pub fn mangler(hentet: &HashMap<String, String>, nu: i64) -> Option<(String, String)> {
    let dage: Vec<String> = (1..=BESOEG_DAGE)
        .map(|i| dag_noegle(nu - i * DOEGN_MS))
        .filter(|d| er_faerdig(d, &iso(nu)) && !hentet.contains_key(d))
        .collect();
    Some((dage.last()?.clone(), dage.first()?.clone()))
}

/// Variablerne til døgnene `fra`…`til` (hele UTC-døgn).
pub fn variabler(zone: &str, fra: &str, til: &str) -> Result<Value, Fejl> {
    dag_ms(fra).ok_or(Fejl::UgyldigDato)?;
    let slut = dag_ms(til).ok_or(Fejl::UgyldigDato)? + DOEGN_MS;
    Ok(json!({ "zoneTag": zone, "since": format!("{fra}T00:00:00Z"), "until": format!("{}T00:00:00Z", dag_noegle(slut)) }))
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Sum {
    #[serde(default, deserialize_with = "tom_ved_null")]
    visits: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Dim {
    #[serde(default, deserialize_with = "tom_ved_null")]
    date: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientCountryName")]
    land: String,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Gruppe {
    #[serde(default, deserialize_with = "tom_ved_null")]
    sum: Sum,
    #[serde(default, deserialize_with = "tom_ved_null")]
    dimensions: Dim,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Svar {
    #[serde(default, deserialize_with = "tom_ved_null")]
    besoeg: Vec<Gruppe>,
}

/// Svaret som (dag, art, nøgle, n) for døgnene `dage`: `b_land` pr. land med besøg (tomt land er `XX`, som resten af programmet) og `b_total`
/// (`besoeg`) for hvert døgn, også 0, når Cloudflare ingen besøg havde. Rækker for andre døgn end `dage` tæller ikke. Ren funktion.
pub fn raekker(data: &Value, dage: &[String]) -> Result<Vec<(String, String, String, i64)>, Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    let svar = Svar::deserialize(z).map_err(|_| Fejl::Svar)?;
    let mut lande: BTreeMap<(&str, String), i64> = BTreeMap::new();
    for g in &svar.besoeg {
        let Some(dag) = dage.iter().find(|d| **d == g.dimensions.date) else {
            continue;
        };
        let land = if g.dimensions.land.is_empty() {
            "XX"
        } else {
            g.dimensions.land.as_str()
        };
        *lande.entry((dag.as_str(), land.to_string())).or_default() += g.sum.visits;
    }
    let mut ud = Vec::new();
    for dag in dage {
        let mut i_alt = 0;
        for ((_, land), n) in lande
            .range((dag.as_str(), String::new())..)
            .take_while(|((d, _), _)| *d == dag.as_str())
        {
            if *n > 0 {
                ud.push((dag.clone(), "b_land".to_string(), land.clone(), *n));
                i_alt += n;
            }
        }
        ud.push((dag.clone(), "b_total".to_string(), "besoeg".to_string(), i_alt));
    }
    Ok(ud)
}

// ── Visningen ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// Det, `regn` skal bruge fra databasen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Laest {
    /// Art `h_total` med nøglen `unikke` for omfanget.
    pub unikke: Vec<Raekke>,
    /// Arterne `b_total` og `b_land` for omfanget.
    pub besoeg: Vec<Raekke>,
    /// Dage, hvor mindst ét domæne i omfanget har fået besøgene hentet.
    pub besoeg_hentet: HashSet<String>,
    /// Domæner i omfanget, hvor Cloudflare ikke svarede på besøgene.
    pub ikke_tilgaengelig: Vec<String>,
}

pub fn laes(lager: &Store, zoner: &[String]) -> Result<Laest, Fejl> {
    let navne: HashMap<String, String> = lager.zoner()?.0.into_iter().map(|z| (z.id, z.navn)).collect();
    let mut besoeg_hentet = HashSet::new();
    let mut ikke_tilgaengelig = Vec::new();
    for z in zoner {
        besoeg_hentet.extend(lager.hentet_besoeg(z)?.into_keys());
        if lager.afsnit_fejl(BESOEG_FEJL, z)?.is_some() {
            ikke_tilgaengelig.push(navne.get(z).cloned().unwrap_or_else(|| z.clone()));
        }
    }
    ikke_tilgaengelig.sort();
    let raekker = |arter: &[&str]| -> Result<Vec<Raekke>, Fejl> {
        Ok(lager.raekker_art(zoner, arter, "0000-00-00")?.into_iter().map(|(_, r)| r).collect())
    };
    Ok(Laest {
        unikke: raekker(&["h_total"])?.into_iter().filter(|r| r.noegle == UNIKKE).collect(),
        besoeg: raekker(&["b_total", "b_land"])?,
        besoeg_hentet,
        ikke_tilgaengelig,
    })
}

/// En flise: summen af dagstallene i dagene, hvor mange dage der har tal (`med`), og hvor mange dage den dækker.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Sum7 {
    pub n: i64,
    pub med: i64,
    pub dage: i64,
}

/// Én dag i grafen. `n: None`: ingen tal for dagen (ikke hentet), og det er noget andet end 0.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DagUd {
    pub dag: String,
    pub n: Option<i64>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct LandUd {
    pub kode: String,
    pub n: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct BesoegUd {
    /// Perioden med dens huller: unikke besøgende, eller besøgene, når der er et land som filter.
    pub periode: PeriodeUd,
    pub i_gaar_dag: String,
    /// Unikke besøgende i går; `None`, når dagen ikke har tal endnu.
    pub i_gaar: Option<i64>,
    pub d7: Sum7,
    pub d30: Sum7,
    pub aar: Sum7,
    /// Besøg de seneste 30 dage.
    pub besoeg30: Sum7,
    /// Grafen: unikke besøgende pr. dag, eller besøg fra landet pr. dag.
    pub dage: Vec<DagUd>,
    pub land: Option<String>,
    /// Besøg pr. land i perioden, flest først, og summen.
    pub lande: Vec<LandUd>,
    pub lande_i_alt: i64,
    /// Dage i perioden med besøg hentet, og den første dag med besøg.
    pub lande_med: i64,
    pub besoeg_fra: Option<String>,
    pub ikke_tilgaengelig: Vec<String>,
}

/// Første og sidste dag i perioden. "Hele" er et år; et spænd er højst et år, regnet bagud fra `til`. Ren funktion.
pub fn spaend(periode: &Periode, nu: i64) -> (String, String) {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    match periode {
        Periode::D7 => (d(7), d(1)),
        Periode::D30 => (d(30), d(1)),
        Periode::Alt => (d(AAR_DAGE), d(1)),
        Periode::Dag(dag) => (dag.clone(), dag.clone()),
        Periode::Spaend { fra, til } => {
            let graense = dag_ms(til).map_or_else(String::new, |t| dag_noegle(t - (AAR_DAGE - 1) * DOEGN_MS));
            (fra.clone().max(graense), til.clone())
        }
    }
}

/// Et land i adressen: to tegn, store bogstaver eller tal (Cloudflares koder, også `T1` og `XX`).
pub fn tjek_land(land: Option<String>) -> Result<Option<String>, Fejl> {
    match land {
        None => Ok(None),
        Some(l) if l.len() == 2 && l.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) => Ok(Some(l)),
        Some(_) => Err(Fejl::UgyldigtFilter),
    }
}

/// Svaret ud fra det læste. Ren funktion.
pub fn regn(periode: &Periode, land: Option<&str>, l: &Laest, nu: i64) -> BesoegUd {
    let mut unikke: HashMap<&str, i64> = HashMap::new();
    for r in &l.unikke {
        *unikke.entry(r.dag.as_str()).or_default() += r.n;
    }
    let (mut besoeg, mut fra_land): (HashMap<&str, i64>, HashMap<&str, i64>) = (HashMap::new(), HashMap::new());
    for r in &l.besoeg {
        match r.art.as_str() {
            "b_total" => *besoeg.entry(r.dag.as_str()).or_default() += r.n,
            "b_land" if Some(r.noegle.as_str()) == land => *fra_land.entry(r.dag.as_str()).or_default() += r.n,
            _ => {}
        }
    }
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    let sum = |tal: &HashMap<&str, i64>, hentet: &dyn Fn(&str) -> bool, dage: i64| {
        let i = (1..=dage).map(d).filter(|x| hentet(x));
        let (n, med) = i.fold((0, 0), |(n, med), x| (n + tal.get(x.as_str()).copied().unwrap_or(0), med + 1));
        Sum7 { n, med, dage }
    };
    let har_unikke = |x: &str| unikke.contains_key(x);
    let har_besoeg = |x: &str| l.besoeg_hentet.contains(x);
    let (fra, til) = spaend(periode, nu);
    let dage = dage_i(&fra, &til);
    let (serie, hentet): (&HashMap<&str, i64>, HashSet<String>) = match land {
        Some(_) => (&fra_land, l.besoeg_hentet.clone()),
        None => (&unikke, unikke.keys().map(|x| x.to_string()).collect()),
    };
    let mut periode_ud = periode_ud(&fra, &til, &hentet, nu);
    // Fra/Til kan højst nå et år tilbage.
    let foerste = d(AAR_DAGE);
    periode_ud.data_fra = periode_ud.data_fra.map(|f| f.max(foerste));
    let mut lande: HashMap<&str, i64> = HashMap::new();
    let mut lande_i_alt = 0;
    for r in l.besoeg.iter().filter(|r| r.dag >= fra && r.dag <= til) {
        match r.art.as_str() {
            "b_land" => *lande.entry(r.noegle.as_str()).or_default() += r.n,
            "b_total" => lande_i_alt += r.n,
            _ => {}
        }
    }
    let mut lande: Vec<LandUd> = lande
        .into_iter()
        .filter(|x| x.1 > 0)
        .map(|(kode, n)| LandUd { kode: kode.to_string(), n })
        .collect();
    lande.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.kode.cmp(&b.kode)));
    BesoegUd {
        i_gaar_dag: d(1),
        i_gaar: unikke.get(d(1).as_str()).copied(),
        d7: sum(&unikke, &har_unikke, 7),
        d30: sum(&unikke, &har_unikke, 30),
        aar: sum(&unikke, &har_unikke, AAR_DAGE),
        besoeg30: sum(&besoeg, &har_besoeg, 30),
        dage: dage
            .iter()
            .map(|x| DagUd {
                dag: x.clone(),
                n: hentet.contains(x).then(|| serie.get(x.as_str()).copied().unwrap_or(0)),
            })
            .collect(),
        land: land.map(str::to_string),
        lande,
        lande_i_alt,
        lande_med: dage.iter().filter(|x| har_besoeg(x)).count() as i64,
        besoeg_fra: l.besoeg_hentet.iter().min().cloned(),
        ikke_tilgaengelig: l.ikke_tilgaengelig.clone(),
        periode: periode_ud,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NU: i64 = 1_793_955_600_000; // 2026-11-06T09:00:00Z

    fn rk(dag: &str, art: &str, noegle: &str, n: i64) -> Raekke {
        Raekke {
            dag: dag.into(),
            art: art.into(),
            noegle: noegle.into(),
            n,
        }
    }

    #[test]
    fn forespoergslen_har_kun_de_kendte_felter_og_filtret() {
        for felt in [
            "httpRequestsAdaptiveGroups",
            "sum { visits }",
            "dimensions { date clientCountryName }",
            r#"requestSource:"eyeball""#,
            "datetime_geq:$since",
            "datetime_lt:$until",
        ] {
            assert!(BESOEG_QUERY.contains(felt), "{felt}");
        }
        for felt in ["clientIP", "userAgent", "clientRequestPath", "clientAsn"] {
            assert!(!BESOEG_QUERY.contains(felt), "{felt}");
        }
    }

    /// Højst 30 dage tilbage, kun færdige døgn, og bagefter kun de nye.
    #[test]
    fn mangler_er_hoejst_30_dage_og_saa_kun_det_nye() {
        let (fra, til) = mangler(&HashMap::new(), NU).unwrap();
        assert_eq!((fra.as_str(), til.as_str()), ("2026-10-07", "2026-11-05"));
        let v = variabler("z", &fra, &til).unwrap();
        assert_eq!(
            (v["since"].as_str(), v["until"].as_str()),
            (Some("2026-10-07T00:00:00Z"), Some("2026-11-06T00:00:00Z"))
        );
        let spaend_s = (dag_ms("2026-11-06").unwrap() - dag_ms("2026-10-07").unwrap()) / 1000;
        assert!(spaend_s <= 2_592_000, "maxDuration 30 dage: {spaend_s}");
        let mut hentet: HashMap<String, String> = (2..=BESOEG_DAGE)
            .map(|i| (dag_noegle(NU - i * DOEGN_MS), "t".to_string()))
            .collect();
        assert_eq!(mangler(&hentet, NU), Some(("2026-11-05".into(), "2026-11-05".into())));
        hentet.insert("2026-11-05".into(), "t".into());
        assert_eq!(mangler(&hentet, NU), None);
        let lige_efter = 1_793_923_200_000 + 60_000; // 2026-11-06T00:01Z: i går er ikke færdig før kl. 01
        assert_eq!(mangler(&HashMap::new(), lige_efter).unwrap().1, "2026-11-04");
        assert!(variabler("z", "x", "2026-11-01").is_err());
    }

    /// Fixture med Cloudflares form: lande lægges sammen pr. dag, tomt land er XX, 0 gemmes ikke pr. land, og et døgn uden besøg er 0.
    #[test]
    fn svaret_bliver_til_b_land_og_b_total() {
        let data = json!({"viewer": {"zones": [{"besoeg": [
            {"sum": {"visits": 40}, "dimensions": {"date": "2026-11-01", "clientCountryName": "DK"}},
            {"sum": {"visits": 2}, "dimensions": {"date": "2026-11-01", "clientCountryName": "DK"}},
            {"sum": {"visits": 7}, "dimensions": {"date": "2026-11-01", "clientCountryName": "US"}},
            {"sum": {"visits": 3}, "dimensions": {"date": "2026-11-01", "clientCountryName": ""}},
            {"sum": {"visits": 0}, "dimensions": {"date": "2026-11-01", "clientCountryName": "SE"}},
            {"sum": null, "dimensions": {"date": "2026-11-02", "clientCountryName": "DE"}},
            {"sum": {"visits": 9}, "dimensions": {"date": "2026-10-01", "clientCountryName": "DK"}}
        ]}]}});
        let dage = vec!["2026-11-01".to_string(), "2026-11-02".to_string(), "2026-11-03".to_string()];
        let r = raekker(&data, &dage).unwrap();
        let tal = |dag: &str, art: &str, n: &str| r.iter().find(|x| x.0 == dag && x.1 == art && x.2 == n).map(|x| x.3);
        assert_eq!(
            (
                tal("2026-11-01", "b_land", "DK"),
                tal("2026-11-01", "b_land", "US"),
                tal("2026-11-01", "b_land", "XX")
            ),
            (Some(42), Some(7), Some(3))
        );
        assert_eq!(tal("2026-11-01", "b_land", "SE"), None, "0 gemmes ikke");
        assert_eq!(tal("2026-11-01", "b_total", "besoeg"), Some(52));
        assert_eq!(
            (tal("2026-11-02", "b_total", "besoeg"), tal("2026-11-03", "b_total", "besoeg")),
            (Some(0), Some(0))
        );
        assert!(r.iter().all(|x| x.0 != "2026-10-01"), "kun de spurgte døgn");
        assert_eq!(raekker(&json!({"viewer": {"zones": []}}), &dage), Err(Fejl::ZoneUkendt));
        assert_eq!(raekker(&json!({"viewer": {"zones": [{"besoeg": "x"}]}}), &dage), Err(Fejl::Svar));
    }

    fn laest() -> Laest {
        let d = |i: i64| dag_noegle(NU - i * DOEGN_MS);
        // Unikke: 100 pr. dag for alle dage i året undtagen d-3 og d-200, og 5 pr. dag før året.
        let mut unikke: Vec<Raekke> = (1..=AAR_DAGE)
            .filter(|i| ![3, 200].contains(i))
            .map(|i| rk(&d(i), "h_total", "unikke", 100))
            .collect();
        unikke.push(rk(&d(400), "h_total", "unikke", 5));
        // Besøg: d-1…d-10 er hentet, d-4 uden besøg.
        let mut besoeg = Vec::new();
        for i in (1..=10).filter(|i| *i != 4) {
            besoeg.push(rk(&d(i), "b_land", "DK", 8));
            besoeg.push(rk(&d(i), "b_land", "US", 2));
            besoeg.push(rk(&d(i), "b_total", "besoeg", 10));
        }
        besoeg.push(rk(&d(4), "b_total", "besoeg", 0));
        Laest {
            unikke,
            besoeg,
            besoeg_hentet: (1..=10).map(d).collect(),
            ikke_tilgaengelig: Vec::new(),
        }
    }

    /// Summerne for 7, 30 og 365 dage tæller kun dage med tal og siger, hvor mange det er.
    #[test]
    fn summer_for_7_30_og_365_dage_med_manglende_doegn() {
        let u = regn(&Periode::D7, None, &laest(), NU);
        assert_eq!(u.i_gaar, Some(100));
        assert_eq!(u.d7, Sum7 { n: 600, med: 6, dage: 7 });
        assert_eq!(
            u.d30,
            Sum7 {
                n: 2900,
                med: 29,
                dage: 30
            }
        );
        assert_eq!(
            u.aar,
            Sum7 {
                n: 36300,
                med: 363,
                dage: 365
            },
            "d-400 er uden for året"
        );
        assert_eq!(u.besoeg30, Sum7 { n: 90, med: 10, dage: 30 });
        assert_eq!((u.periode.dage, u.periode.med), (7, 6));
        assert_eq!(
            u.dage.iter().find(|x| x.dag == dag_noegle(NU - 3 * DOEGN_MS)).unwrap().n,
            None,
            "et hul er ikke 0"
        );
        assert_eq!(
            u.periode.data_fra,
            Some(dag_noegle(NU - AAR_DAGE * DOEGN_MS)),
            "Fra/Til højst et år tilbage"
        );
        let tom = regn(&Periode::D7, None, &Laest::default(), NU);
        assert_eq!((tom.i_gaar, tom.d7.med, tom.besoeg30.n), (None, 0, 0));
    }

    /// Landene i perioden, flest først, og grafen med et land som filter: besøg fra landet, huller for dage uden hentede besøg.
    #[test]
    fn lande_og_filter() {
        let u = regn(&Periode::D7, None, &laest(), NU);
        assert_eq!(
            u.lande,
            vec![LandUd { kode: "DK".into(), n: 48 }, LandUd { kode: "US".into(), n: 12 }]
        );
        assert_eq!((u.lande_i_alt, u.lande_med), (60, 7));
        assert_eq!(u.besoeg_fra, Some(dag_noegle(NU - 10 * DOEGN_MS)));
        let f = regn(&Periode::D30, Some("DK"), &laest(), NU);
        assert_eq!((f.periode.dage, f.periode.med), (30, 10), "dækningen er besøgenes");
        let n = |i: i64| f.dage.iter().find(|x| x.dag == dag_noegle(NU - i * DOEGN_MS)).unwrap().n;
        assert_eq!((n(1), n(4), n(20)), (Some(8), Some(0), None));
        assert_eq!(f.land.as_deref(), Some("DK"));
    }

    #[test]
    fn perioden_er_hoejst_et_aar() {
        assert_eq!(
            spaend(&Periode::Alt, NU),
            (dag_noegle(NU - 365 * DOEGN_MS), "2026-11-05".to_string())
        );
        let (fra, til) = spaend(
            &Periode::Spaend {
                fra: "2024-01-01".into(),
                til: "2026-11-05".into(),
            },
            NU,
        );
        assert_eq!(
            (fra.as_str(), til.as_str(), dage_i(&fra, &til).len()),
            ("2025-11-06", "2026-11-05", 365)
        );
        assert_eq!(
            spaend(
                &Periode::Spaend {
                    fra: "2026-11-01".into(),
                    til: "2026-11-03".into()
                },
                NU
            )
            .0,
            "2026-11-01"
        );
        assert_eq!(tjek_land(Some("DK".into())), Ok(Some("DK".into())));
        assert_eq!(tjek_land(Some("T1".into())), Ok(Some("T1".into())));
        for x in ["dk", "DNK", "", "D'"] {
            assert_eq!(tjek_land(Some(x.into())), Err(Fejl::UgyldigtFilter), "{x}");
        }
    }

    /// Lageret: besøgene står for sig (ikke i `raekker`), et døgn uden rækker er hentet, og "Nulstil historik" sletter dem og tilstanden.
    #[test]
    fn lageret_gemmer_og_nulstiller() {
        let mut s = Store::i_hukommelse().unwrap();
        let dage = vec!["2026-11-01".to_string(), "2026-11-02".to_string()];
        let data = json!({"viewer": {"zones": [{"besoeg": [{"sum": {"visits": 4}, "dimensions": {"date": "2026-11-01", "clientCountryName": "DK"}}]}]}});
        s.gem_besoeg("z", &dage, &raekker(&data, &dage).unwrap(), "t").unwrap();
        assert_eq!(s.hentet_besoeg("z").unwrap().len(), 2);
        assert!(
            s.raekker("z", "0000-00-00").unwrap().is_empty(),
            "blander sig ikke i Angreb og Overblik"
        );
        s.gem_afsnit_fejl(BESOEG_FEJL, "z", Some("afvist")).unwrap();
        let l = laes(&s, &["z".to_string()]).unwrap();
        assert_eq!(
            (l.besoeg.len(), l.besoeg_hentet.len(), l.ikke_tilgaengelig.clone()),
            (3, 2, vec!["z".to_string()])
        );
        s.nulstil_historik().unwrap();
        let l = laes(&s, &["z".to_string()]).unwrap();
        assert!(l.besoeg.is_empty() && l.besoeg_hentet.is_empty() && l.ikke_tilgaengelig.is_empty());
    }

    /// Unikke for de gamle døgn: én gang, kun nøglen `unikke`, kun døgn med årstal, og et domæne, der hentes bagefter, venter ikke.
    #[test]
    fn unikke_en_gang_roerer_ingen_andre_tal() {
        let mut s = Store::i_hukommelse().unwrap();
        let dage = vec!["2026-01-01".to_string(), "2026-01-02".to_string()];
        let aar = vec![
            ("2026-01-01".to_string(), "h_total".to_string(), "foresp".to_string(), 90),
            ("2026-01-01".to_string(), "h_land".to_string(), "DK".to_string(), 50),
        ];
        s.gem_historik("z1", &dage, &aar, "t").unwrap();
        assert!(s.unikke_en_gang("2026-11-06").unwrap());
        assert!(!s.unikke_en_gang("2026-11-07").unwrap(), "kun én gang");
        assert_eq!(s.indstilling(crate::store::UNIKKE_V1).unwrap().as_deref(), Some("2026-11-06"));
        s.gem_historik("z2", &dage, &[], "t").unwrap();
        assert!(
            s.unikke_mangler("z1").unwrap() && !s.unikke_mangler("z2").unwrap(),
            "z2 kom efter: får tallet med det samme"
        );
        s.gem_unikke("z1", &[("2026-01-01".to_string(), 17), ("2026-01-05".to_string(), 3)])
            .unwrap();
        assert!(!s.unikke_mangler("z1").unwrap());
        let mut rk: Vec<(String, String, String, i64)> = s
            .raekker_art(&["z1".to_string()], &["h_total", "h_land", "h_status"], "0000-00-00")
            .unwrap()
            .into_iter()
            .map(|(_, r)| (r.dag, r.art, r.noegle, r.n))
            .collect();
        rk.sort();
        assert_eq!(
            rk,
            vec![
                ("2026-01-01".into(), "h_land".into(), "DK".into(), 50),
                ("2026-01-01".into(), "h_total".into(), "foresp".into(), 90),
                ("2026-01-01".into(), "h_total".into(), "unikke".into(), 17),
            ],
            "intet andet tal ændret, og 5. jan. uden årstal fik ingen række"
        );
        assert_eq!(
            s.hentet_historik("z1").unwrap().get("2026-01-01").map(String::as_str),
            Some("t"),
            "hentet-tiden står"
        );
    }
}
