//! Et års trafik: `httpRequests1dGroups` har 365 dage på Free, hvor sikkerhedshændelser kun har en måned. Daglige totaler, de største lande og svarkoder, ét kald pr. domæne for hele året og
//! derefter ét kald, når der er nye døgn. Arterne i `vagt_dag` er `h_land`, `h_status` og `h_total` med nøglerne `foresp`, `cachet`, `trusler` og `unikke`. Hentes sidst i `koe::Koe::ekstra_runde`, efter alt andet. Felterne er dem, bilag A og spiken bekræfter. Ingen IP-adresser, stier eller brugeragenter findes i noden.
//! `h_ipklasse` er `ipClassMap { ipType requests }` (V-BESOEG.2): forespørgsler pr. IP-klasse pr. døgn; døgn hentet før det får det én gang (`store::IPKLASSE_V1`), og kun den art skrives.
//! `unikke` er `uniq { uniques }`: antallet af unikke IP-adresser for døgnet, et tal, ikke adresserne (V-BESOEG.1). Døgn hentet før det fik tallet én gang (`store::UNIKKE_V1`, `unikke_spaend`): kun nøglen `unikke` skrives, intet andet tal ændres.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cf::Fejl;
use crate::store::Aeldste;
use crate::vaern::{dag_ms, dag_noegle, er_faerdig, iso, tom_ved_null, DOEGN_MS};

/// `notOlderThan` er 365 dage og en time regnet fra nu, og et døgn tæller fra midnat, så det ældste hele døgn, der altid kan spørges om, er d-364.
pub const HISTORIK_DAGE: i64 = 364;
/// Så mange dage tilbage kan tal hentes igen efter en nulstilling: sikkerhedshændelser og trafik.
pub const SIKKERHED_GRAENSE: i64 = 31;
pub const TRAFIK_GRAENSE: i64 = 365;
/// Så mange lande og svarkoder gemmes pr. døgn.
const TOP: usize = 10;

pub const HISTORIK_QUERY: &str = r#"query Historik($zoneTag: string!, $fra: Date!, $til: Date!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    dage: httpRequests1dGroups(limit:400, filter:{date_geq:$fra, date_leq:$til}, orderBy:[date_ASC]) { dimensions { date } sum { requests cachedRequests threats countryMap { clientCountryName requests } responseStatusMap { edgeResponseStatus requests } ipClassMap { ipType requests } } uniq { uniques } }
  } } }"#;

/// De færdige døgn i året, der mangler: (ældste, nyeste). Ét kald dækker alt derimellem. Ren funktion.
pub fn mangler(hentet: &HashMap<String, String>, nu: i64) -> Option<(String, String)> {
    let dage: Vec<String> = (1..=HISTORIK_DAGE)
        .map(|i| dag_noegle(nu - i * DOEGN_MS))
        .filter(|d| er_faerdig(d, &iso(nu)) && !hentet.contains_key(d))
        .collect();
    Some((dage.last()?.clone(), dage.first()?.clone()))
}

/// De færdige døgn i året, der er hentet før `unikke`: (ældste, nyeste), som ét kald til `HISTORIK_QUERY` dækker. `None`: intet at hente igen. Ren funktion.
pub fn unikke_spaend(hentet: &HashMap<String, String>, nu: i64) -> Option<(String, String)> {
    let dage: Vec<String> = (1..=HISTORIK_DAGE)
        .map(|i| dag_noegle(nu - i * DOEGN_MS))
        .filter(|d| er_faerdig(d, &iso(nu)) && hentet.contains_key(d))
        .collect();
    Some((dage.last()?.clone(), dage.first()?.clone()))
}

/// Kun `unikke` fra svaret: (dag, n). Til genhentningen, der ikke må ændre andre tal. Ren funktion.
pub fn unikke_raekker(data: &Value) -> Result<Vec<(String, i64)>, Fejl> {
    Ok(raekker(data)?
        .into_iter()
        .filter(|r| r.1 == "h_total" && r.2 == UNIKKE)
        .map(|r| (r.0, r.3))
        .collect())
}

/// Nøglen i art `h_total` for unikke besøgende.
pub const UNIKKE: &str = "unikke";
/// Art for forespørgsler pr. IP-klasse (`ipClassMap`, nøglen er Cloudflares `ipType`, fx `searchEngine`, `scan`).
pub const IPKLASSE: &str = "h_ipklasse";
/// IP-klasserne, der tæller som kendte robotter og scannere i "x % af trafikken det seneste år" (V-BESOEG.2).
pub const ROBOT_KLASSER: &[&str] = &["searchEngine", "monitoringService", "scan", "tor", "badHost"];

/// Kun `h_ipklasse` fra svaret: (dag, art, nøgle, n). Til genhentningen, der ikke må ændre andre tal. Ren funktion.
pub fn ipklasse_raekker(data: &Value) -> Result<Vec<(String, String, String, i64)>, Fejl> {
    Ok(raekker(data)?.into_iter().filter(|r| r.1 == IPKLASSE).collect())
}

/// Tabet for én slags tal: den ældste dag, der findes nu, den ældste, der kan hentes igen, og dagene imellem.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub fra: String,
    pub graense: String,
    pub dage: i64,
}

/// Hvad "Nulstil historik" koster. `None`: den slags tal findes ikke.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct NulstilTab {
    pub sikkerhed: Option<Tab>,
    pub trafik: Option<Tab>,
}

fn tab(fra: Option<&str>, dage_tilbage: i64, nu: i64) -> Option<Tab> {
    let fra = fra?;
    let graense = dag_noegle(nu - dage_tilbage * DOEGN_MS);
    let dage = dag_ms(&graense).zip(dag_ms(fra)).map_or(0, |(g, f)| ((g - f) / DOEGN_MS).max(0));
    Some(Tab {
        fra: fra.to_string(),
        graense,
        dage,
    })
}

/// Ren funktion.
pub fn nulstil_tab(a: &Aeldste, nu: i64) -> NulstilTab {
    NulstilTab {
        sikkerhed: tab(a.sikkerhed_fra.as_deref(), SIKKERHED_GRAENSE, nu),
        trafik: tab(a.trafik_fra.as_deref(), TRAFIK_GRAENSE, nu),
    }
}

pub fn variabler(zone: &str, fra: &str, til: &str) -> Value {
    json!({ "zoneTag": zone, "fra": fra, "til": til })
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Land {
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientCountryName")]
    land: String,
    #[serde(default, deserialize_with = "tom_ved_null")]
    requests: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Svar {
    #[serde(default, deserialize_with = "tom_ved_null", rename = "edgeResponseStatus")]
    status: i64,
    #[serde(default, deserialize_with = "tom_ved_null")]
    requests: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Klasse {
    #[serde(default, deserialize_with = "tom_ved_null", rename = "ipType")]
    klasse: String,
    #[serde(default, deserialize_with = "tom_ved_null")]
    requests: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Sum {
    #[serde(default, deserialize_with = "tom_ved_null")]
    requests: i64,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "cachedRequests")]
    cached: i64,
    #[serde(default, deserialize_with = "tom_ved_null")]
    threats: i64,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "countryMap")]
    lande: Vec<Land>,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "responseStatusMap")]
    status: Vec<Svar>,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "ipClassMap")]
    klasser: Vec<Klasse>,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Uniq {
    #[serde(default, deserialize_with = "tom_ved_null")]
    uniques: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Dim {
    #[serde(default, deserialize_with = "tom_ved_null")]
    date: String,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Dag {
    #[serde(default, deserialize_with = "tom_ved_null")]
    dimensions: Dim,
    #[serde(default, deserialize_with = "tom_ved_null")]
    sum: Sum,
    #[serde(default, deserialize_with = "tom_ved_null")]
    uniq: Uniq,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Zone {
    #[serde(default, deserialize_with = "tom_ved_null")]
    dage: Vec<Dag>,
}

/// Svaret som (dag, art, nøgle, n). Ren funktion.
pub fn raekker(data: &Value) -> Result<Vec<(String, String, String, i64)>, Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    let z = Zone::deserialize(z).map_err(|_| Fejl::Svar)?;
    let mut ud = Vec::new();
    for d in z.dage.into_iter().filter(|d| !d.dimensions.date.is_empty()) {
        let dag = d.dimensions.date;
        let mut add = |art: &str, noegle: String, n: i64| ud.push((dag.clone(), art.to_string(), noegle, n));
        add("h_total", "foresp".into(), d.sum.requests);
        add("h_total", "cachet".into(), d.sum.cached);
        add("h_total", "trusler".into(), d.sum.threats);
        add("h_total", UNIKKE.into(), d.uniq.uniques);
        let mut lande = d.sum.lande;
        lande.sort_by_key(|l| std::cmp::Reverse(l.requests));
        for l in lande.into_iter().filter(|l| !l.land.is_empty()).take(TOP) {
            add("h_land", l.land, l.requests);
        }
        let mut status = d.sum.status;
        status.sort_by_key(|s| std::cmp::Reverse(s.requests));
        for s in status.into_iter().filter(|s| s.status != 0).take(TOP) {
            add("h_status", s.status.to_string(), s.requests);
        }
        let mut klasser: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        for k in d.sum.klasser.into_iter().filter(|k| !k.klasse.is_empty() && k.requests > 0) {
            *klasser.entry(k.klasse).or_default() += k.requests;
        }
        for (k, n) in klasser {
            add(IPKLASSE, k, n);
        }
    }
    Ok(ud)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 6. nov. minus 31 dage er 6. okt., så data fra 4. okt. mister 2 dage. Eksemplet fra fundet.
    #[test]
    fn nulstil_siger_hvad_der_forsvinder() {
        const NU: i64 = 1_793_955_600_000; // 2026-11-06T09:00:00Z
        let a = Aeldste {
            sikkerhed_fra: Some("2026-10-04".into()),
            trafik_fra: Some("2025-11-01".into()),
            sidste: None,
        };
        let t = nulstil_tab(&a, NU);
        let s = t.sikkerhed.unwrap();
        assert_eq!((s.fra.as_str(), s.graense.as_str(), s.dage), ("2026-10-04", "2026-10-06", 2));
        let f = t.trafik.unwrap();
        assert_eq!((f.graense.as_str(), f.dage), ("2025-11-06", 5));
        let nyt = Aeldste {
            sikkerhed_fra: Some("2026-11-01".into()),
            ..Aeldste::default()
        };
        let t = nulstil_tab(&nyt, NU);
        assert_eq!(t.sikkerhed.unwrap().dage, 0, "alt kan hentes igen");
        assert_eq!(t.trafik, None);
    }

    const NU: i64 = 1_793_693_280_000; // 2026-11-03T08:08:00Z

    #[test]
    fn mangler_er_aaret_og_saa_kun_det_nye() {
        let tom = HashMap::new();
        let (fra, til) = mangler(&tom, NU).unwrap();
        assert_eq!(
            (til.as_str(), fra.as_str()),
            ("2026-11-02", dag_noegle(NU - 364 * DOEGN_MS).as_str())
        );
        let mut hentet: HashMap<String, String> = (2..=HISTORIK_DAGE)
            .map(|i| (dag_noegle(NU - i * DOEGN_MS), "t".to_string()))
            .collect();
        assert_eq!(
            mangler(&hentet, NU),
            Some(("2026-11-02".to_string(), "2026-11-02".to_string())),
            "ét nyt døgn"
        );
        hentet.insert("2026-11-02".into(), "t".into());
        assert_eq!(mangler(&hentet, NU), None);
    }

    #[test]
    fn et_doegn_der_ikke_er_faerdigt_hentes_ikke() {
        let lige_efter = 1_793_664_000_000 + 60_000; // 2026-11-03T00:01:00Z: i går er ikke faerdig før +1 time
        let (_, til) = mangler(&HashMap::new(), lige_efter).unwrap();
        assert_eq!(til, "2026-11-01");
    }

    #[test]
    fn raekker_med_top_og_uden_tomme_felter() {
        let lande: Vec<Value> = (0..12)
            .map(|i| json!({"clientCountryName": format!("L{i}"), "requests": i}))
            .collect();
        let data = json!({"viewer": {"zones": [{"dage": [
            {"dimensions": {"date": "2026-01-02"}, "sum": {"requests": 90, "cachedRequests": 40, "threats": 3, "countryMap": lande, "responseStatusMap": [{"edgeResponseStatus": 200, "requests": 80}, {"edgeResponseStatus": 0, "requests": 2}]}, "uniq": {"uniques": 17}},
            {"dimensions": {"date": "2026-01-03"}, "sum": null}
        ]}]}});
        let r = raekker(&data).unwrap();
        let tal = |art: &str, n: &str| r.iter().find(|x| x.0 == "2026-01-02" && x.1 == art && x.2 == n).map(|x| x.3);
        assert_eq!(
            (tal("h_total", "foresp"), tal("h_total", "cachet"), tal("h_total", "trusler")),
            (Some(90), Some(40), Some(3))
        );
        assert_eq!(tal("h_total", "unikke"), Some(17));
        assert_eq!(r.iter().filter(|x| x.1 == "h_land").count(), 10);
        assert_eq!(tal("h_land", "L0"), None, "kun de største");
        assert_eq!((tal("h_status", "200"), tal("h_status", "0")), (Some(80), None));
        assert_eq!(
            r.iter().filter(|x| x.0 == "2026-01-03" && x.1 == "h_total").count(),
            4,
            "et døgn uden tal er 0, ikke væk"
        );
        assert_eq!(
            unikke_raekker(&data).unwrap(),
            vec![("2026-01-02".to_string(), 17), ("2026-01-03".to_string(), 0)]
        );
    }

    /// Genhentningen dækker de hentede døgn i året, ét kald fra den ældste til den nyeste, og kun færdige døgn.
    #[test]
    fn unikke_spaend_er_de_hentede_doegn() {
        assert_eq!(unikke_spaend(&HashMap::new(), NU), None, "intet hentet: intet at hente igen");
        let mut hentet: HashMap<String, String> = [5, 40, 300]
            .iter()
            .map(|i| (dag_noegle(NU - i * DOEGN_MS), "t".to_string()))
            .collect();
        hentet.insert(dag_noegle(NU - 400 * DOEGN_MS), "t".into());
        hentet.insert(dag_noegle(NU), "t".into());
        assert_eq!(
            unikke_spaend(&hentet, NU),
            Some((dag_noegle(NU - 300 * DOEGN_MS), dag_noegle(NU - 5 * DOEGN_MS))),
            "ældre end året og i dag tæller ikke"
        );
    }

    #[test]
    fn forespoergslen_har_kun_kendte_felter() {
        for felt in [
            "httpRequests1dGroups",
            "requests",
            "cachedRequests",
            "threats",
            "countryMap",
            "responseStatusMap",
            "uniq { uniques }",
            "ipClassMap { ipType requests }",
        ] {
            assert!(HISTORIK_QUERY.contains(felt), "{felt}");
        }
        for felt in ["clientIP", "userAgent"] {
            assert!(!HISTORIK_QUERY.contains(felt), "{felt}");
        }
    }

    /// `ipClassMap` bliver til `h_ipklasse` pr. klasse; tomme klasser og 0 gemmes ikke, og genhentningen tager kun den art.
    #[test]
    fn ipklasser_bliver_til_h_ipklasse() {
        let data = json!({"viewer": {"zones": [{"dage": [
            {"dimensions": {"date": "2026-01-02"}, "sum": {"requests": 90, "ipClassMap": [
                {"ipType": "searchEngine", "requests": 20}, {"ipType": "scan", "requests": 5}, {"ipType": "noRecord", "requests": 60},
                {"ipType": "", "requests": 3}, {"ipType": "tor", "requests": 0}
            ]}, "uniq": {"uniques": 1}}
        ]}]}});
        let r = ipklasse_raekker(&data).unwrap();
        assert_eq!(
            r,
            vec![
                ("2026-01-02".into(), IPKLASSE.into(), "noRecord".into(), 60),
                ("2026-01-02".into(), IPKLASSE.into(), "scan".into(), 5),
                ("2026-01-02".into(), IPKLASSE.into(), "searchEngine".into(), 20),
            ]
        );
        assert!(raekker(&data).unwrap().iter().any(|x| x.1 == "h_total"), "årstallene har det hele");
        assert!(
            ipklasse_raekker(&data).unwrap().iter().all(|x| x.1 == IPKLASSE),
            "genhentningen kun h_ipklasse"
        );
    }
}
