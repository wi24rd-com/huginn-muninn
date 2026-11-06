//! DNS-rekognoscering: `dnsAnalyticsAdaptiveGroups` for færdige døgn. Node og felter står i bilag A, §3 (zone-noden, Free, 31 dage, målt live). Forespørgslen er sin egen, så en fejl her kun koster
//! dette afsnit. `sourceIP` er en dimension på noden og hentes aldrig: der gemmes kun svarkoder, typer og navne. Antal forskellige navne er antal grupper på `queryName` (højst `NAVNE_GRAENSE`), for en tæller
//! (`uniq`) på noden er ikke verificeret.
use serde::Deserialize;
use serde_json::{json, Value};

use crate::cf::Fejl;
use crate::overblik::{dag_foer, Betingelse};
use crate::store::Raekke;
use crate::vaern::{dag_ms, js_slice, tom_ved_null};

/// `date_geq`/`date_leq` er formen fra Cloudflares DNS Analytics-side. Kun zone og dag er variabler.
pub const DNS_QUERY: &str = r#"query Dns($zoneTag: string!, $dag: Date!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    koder: dnsAnalyticsAdaptiveGroups(limit:50, filter:{date_geq:$dag, date_leq:$dag}, orderBy:[count_DESC]) { count dimensions { responseCode } }
    typer: dnsAnalyticsAdaptiveGroups(limit:50, filter:{date_geq:$dag, date_leq:$dag}, orderBy:[count_DESC]) { count dimensions { queryType } }
    navne: dnsAnalyticsAdaptiveGroups(limit:1000, filter:{date_geq:$dag, date_leq:$dag, responseCode:"NXDOMAIN"}, orderBy:[count_DESC]) { count dimensions { queryName } }
  } } }"#;

/// Arterne, `Store::gem_dns` erstatter.
pub const ARTER: &[&str] = &["dns_kode", "dns_type", "dns_navne", "dns_navn"];
/// Så mange forskellige navne kan tælles på ét døgn. Grænsen i `DNS_QUERY`.
#[cfg(test)]
pub const NAVNE_GRAENSE: i64 = 1000;
/// De mest efterspurgte navne, der ikke findes, gemmes som eksempler til boksen "Vurdering".
const EKSEMPLER_GEMT: usize = 10;
const EKSEMPLER_VIST: usize = 3;
/// Et DNS-navn er højst 253 tegn.
const NAVN_MAKS: usize = 253;
/// "Ligner DNS-rekognoscering": mindst så meget og mindst `DNS_FAKTOR` gange snittet af de `DNS_BASIS` døgn før.
pub const DNS_MIN: i64 = 100;
pub const DNS_FAKTOR: f64 = 10.0;
pub const DNS_BASIS: i64 = 7;
/// Så mange af døgnene før skal have tal: uden en normal at holde døgnet op imod siger det ingenting.
pub const DNS_BASIS_MIN: usize = 3;
const NXDOMAIN: &str = "NXDOMAIN";

/// Variablerne til ét døgn.
pub fn variabler(zone: &str, dag: &str) -> Result<Value, Fejl> {
    dag_ms(dag).ok_or(Fejl::UgyldigDato)?;
    Ok(json!({ "zoneTag": zone, "dag": dag }))
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Gruppe {
    #[serde(default, deserialize_with = "tom_ved_null")]
    count: i64,
    #[serde(default)]
    dimensions: Value,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Svar {
    #[serde(default, deserialize_with = "tom_ved_null")]
    koder: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    typer: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    navne: Vec<Gruppe>,
}

/// En dimension som tekst: Cloudflares DNS-logs har koden som tal, GraphQL-eksemplet som tekst.
fn felt(g: &Gruppe, navn: &str) -> String {
    match g.dimensions.get(navn) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Rækkerne for ét døgn: (art, nøgle, n). Ren funktion.
pub fn raekker(data: &Value) -> Result<Vec<(String, String, i64)>, Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    let svar = Svar::deserialize(z).map_err(|_| Fejl::Svar)?;
    let mut ud = Vec::new();
    for (grupper, navn, art) in [(&svar.koder, "responseCode", "dns_kode"), (&svar.typer, "queryType", "dns_type")] {
        ud.extend(grupper.iter().filter_map(|g| {
            let v = felt(g, navn).to_uppercase();
            (!v.is_empty()).then(|| (art.to_string(), js_slice(&v, 40), g.count))
        }));
    }
    let navne: Vec<(String, i64)> = svar
        .navne
        .iter()
        .filter_map(|g| {
            let v = felt(g, "queryName");
            (!v.is_empty()).then(|| (js_slice(&v, NAVN_MAKS), g.count))
        })
        .collect();
    if !navne.is_empty() {
        ud.push(("dns_navne".to_string(), String::new(), navne.len() as i64));
    }
    ud.extend(navne.into_iter().take(EKSEMPLER_GEMT).map(|(v, n)| ("dns_navn".to_string(), v, n)));
    Ok(ud)
}

fn har_tal(rows: &[Raekke], dag: &str) -> bool {
    rows.iter().any(|r| r.art == "dns_kode" && r.dag == dag)
}

/// Opslag på navne, der ikke findes, i døgnet.
fn nx(rows: &[Raekke], dag: &str) -> i64 {
    rows.iter()
        .filter(|r| r.art == "dns_kode" && r.dag == dag && r.noegle == NXDOMAIN)
        .map(|r| r.n)
        .sum()
}

/// Forskellige navne, der ikke findes, i døgnet.
fn navne(rows: &[Raekke], dag: &str) -> i64 {
    rows.iter().filter(|r| r.art == "dns_navne" && r.dag == dag).map(|r| r.n).sum()
}

/// (døgnets tal, snittet før, slår det til?, er der dage nok til et snit?) for ét mål. Ren funktion.
fn maal(rows: &[Raekke], dag: &str, f: &dyn Fn(&[Raekke], &str) -> i64) -> (i64, f64, bool, bool) {
    let foer: Vec<i64> = (1..=DNS_BASIS)
        .map(|i| dag_foer(dag, i))
        .filter(|d| har_tal(rows, d))
        .map(|d| f(rows, &d))
        .collect();
    let snit = if foer.is_empty() {
        0.0
    } else {
        foer.iter().sum::<i64>() as f64 / foer.len() as f64
    };
    let nu = f(rows, dag);
    let nok = foer.len() >= DNS_BASIS_MIN;
    let ok = har_tal(rows, dag) && nok && nu >= DNS_MIN && nu as f64 >= DNS_FAKTOR * snit;
    (nu, snit, ok, nok)
}

/// De to betingelser i boksen "Vurdering": én af dem er nok. Mærket afgøres af de samme tal. Ren funktion.
pub fn vurder(rows: &[Raekke], dag: &str) -> Vec<Betingelse> {
    let linje = |id: &'static str, (nu, snit, ok, nok): (i64, f64, bool, bool), navn: Option<String>| Betingelse {
        id,
        ok: Some(ok),
        n: nu as f64,
        m: (DNS_FAKTOR * snit).max(DNS_MIN as f64),
        navn,
        uden_normal: !nok,
        ..Betingelse::default()
    };
    let eksempler: Vec<&str> = rows
        .iter()
        .filter(|r| r.art == "dns_navn" && r.dag == dag)
        .take(EKSEMPLER_VIST)
        .map(|r| r.noegle.as_str())
        .collect();
    vec![
        linje("dns_nx", maal(rows, dag, &nx), None),
        linje(
            "dns_navne",
            maal(rows, dag, &navne),
            (!eksempler.is_empty()).then(|| eksempler.join(", ")),
        ),
    ]
}

/// Mærket for ét døgn: (tallet, snittet før, er det forskellige navne?). Forskellige navne går forud: det er dem, der siger, at nogen gætter. Ren funktion.
pub fn signal(rows: &[Raekke], dag: &str) -> Option<(i64, f64, bool)> {
    let (a, b) = (maal(rows, dag, &navne), maal(rows, dag, &nx));
    if a.2 {
        Some((a.0, a.1, true))
    } else {
        b.2.then_some((b.0, b.1, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaern::{dag_noegle, DOEGN_MS};

    const NU: i64 = 1_793_000_000_000;

    fn d(i: i64) -> String {
        dag_noegle(NU - i * DOEGN_MS)
    }
    fn rk(dag: &str, art: &str, noegle: &str, n: i64) -> Raekke {
        Raekke {
            dag: dag.into(),
            art: art.into(),
            noegle: noegle.into(),
            n,
        }
    }
    /// Et døgn med `nx` opslag på `navne` forskellige navne.
    fn doegn(i: i64, nx: i64, navne: i64) -> Vec<Raekke> {
        let dag = d(i);
        vec![
            rk(&dag, "dns_kode", "NOERROR", 5000),
            rk(&dag, "dns_kode", NXDOMAIN, nx),
            rk(&dag, "dns_navne", "", navne),
        ]
    }
    fn uge(nx: i64, navne: i64) -> Vec<Raekke> {
        (2..=8).flat_map(|i| doegn(i, nx, navne)).collect()
    }

    /// Ingen IP-adresser: `sourceIP` og `destinationIP` findes på noden, men spørges der aldrig om.
    #[test]
    fn forespoergslen_har_ingen_ip_felter() {
        for felt in ["sourceIP", "destinationIP", "upstreamIP", "IP"] {
            assert!(!DNS_QUERY.contains(felt), "{felt}");
        }
        assert!(DNS_QUERY.contains(&format!("limit:{NAVNE_GRAENSE}")));
    }

    #[test]
    fn variabler_tjekker_datoen() {
        assert_eq!(
            variabler("z", "2026-11-05").unwrap(),
            json!({ "zoneTag": "z", "dag": "2026-11-05" })
        );
        assert!(variabler("z", "ikke-en-dato").is_err());
    }

    /// Koden kan være tekst eller tal, og tomme værdier gemmes ikke. Antallet af navne er antal grupper, og kun de største gemmes som eksempler.
    #[test]
    fn raekker_fra_svaret() {
        let navne: Vec<Value> = (0..12)
            .map(|i| json!({ "count": 20 - i, "dimensions": { "queryName": format!("n{i}.a.wi24rd.com") } }))
            .collect();
        let data = json!({ "viewer": { "zones": [{
            "koder": [{ "count": 900, "dimensions": { "responseCode": "NOERROR" } }, { "count": 80, "dimensions": { "responseCode": "nxdomain" } }, { "count": 1, "dimensions": { "responseCode": 2 } }, { "count": 9, "dimensions": {} }],
            "typer": [{ "count": 700, "dimensions": { "queryType": "A" } }],
            "navne": navne,
        }] } });
        let r = raekker(&data).unwrap();
        let hent = |art: &str, k: &str| r.iter().find(|x| x.0 == art && x.1 == k).map(|x| x.2);
        assert_eq!(hent("dns_kode", NXDOMAIN), Some(80));
        assert_eq!(hent("dns_kode", "2"), Some(1));
        assert_eq!(hent("dns_type", "A"), Some(700));
        assert_eq!(hent("dns_navne", ""), Some(12));
        assert_eq!(r.iter().filter(|x| x.0 == "dns_navn").count(), EKSEMPLER_GEMT);
        assert_eq!(r.iter().filter(|x| x.0 == "dns_kode").count(), 3, "den tomme kode er væk");
        assert!(r.iter().all(|x| ARTER.contains(&x.0.as_str())));
    }

    /// Begge grænser skal være nået: 10 × snittet og 100.
    #[test]
    fn signalet_kraever_baade_faktor_og_gulv() {
        let med = |nx: i64, navne: i64, snit: i64| [uge(snit, snit), doegn(1, nx, navne)].concat();
        assert_eq!(signal(&med(100, 0, 10), &d(1)), Some((100, 10.0, false)), "lige på grænsen");
        assert_eq!(signal(&med(99, 0, 9), &d(1)), None, "under gulvet");
        assert_eq!(signal(&med(199, 0, 20), &d(1)), None, "under 10 × snit");
        assert_eq!(signal(&med(500, 200, 10), &d(1)), Some((200, 10.0, true)), "navnene går forud");
        assert_eq!(signal(&med(0, 100, 10), &d(1)), Some((100, 10.0, true)), "navnene alene er nok");
    }

    /// Uden en normal siger døgnet ingenting, og et døgn uden tal slår ikke til.
    #[test]
    fn signalet_kraever_en_normal() {
        assert_eq!(signal(&doegn(1, 9000, 900), &d(1)), None);
        let to: Vec<Raekke> = (2..=3).flat_map(|i| doegn(i, 1, 1)).chain(doegn(1, 900, 900)).collect();
        assert_eq!(signal(&to, &d(1)), None, "to døgn er for lidt");
        let tre: Vec<Raekke> = (2..=4).flat_map(|i| doegn(i, 1, 1)).chain(doegn(1, 900, 900)).collect();
        assert!(signal(&tre, &d(1)).is_some(), "tre døgn er nok");
        assert_eq!(signal(&uge(0, 0), &d(1)), None);
    }

    /// Boksen siger det samme som mærket, og grænsen er den højeste af de to.
    #[test]
    fn vurderingen_viser_tallene() {
        let mut rows = [uge(40, 2), doegn(1, 500, 150)].concat();
        rows.push(rk(&d(1), "dns_navn", "vpn.a.wi24rd.com", 9));
        rows.push(rk(&d(1), "dns_navn", "mail2.a.wi24rd.com", 7));
        let v = vurder(&rows, &d(1));
        assert_eq!(
            v.iter().map(|b| (b.id, b.ok, b.n, b.m)).collect::<Vec<_>>(),
            [("dns_nx", Some(true), 500.0, 400.0), ("dns_navne", Some(true), 150.0, 100.0)]
        );
        assert_eq!(v[1].navn.as_deref(), Some("vpn.a.wi24rd.com, mail2.a.wi24rd.com"));
        let nej = vurder(&[uge(40, 2), doegn(1, 41, 3)].concat(), &d(1));
        assert_eq!(nej.iter().map(|b| b.ok).collect::<Vec<_>>(), [Some(false), Some(false)]);
        assert!(v.iter().chain(&nej).all(|b| !b.uden_normal));
    }

    /// For få dage til et snit: ingen sammenligning i visningen, og mærket slår stadig ikke til.
    #[test]
    fn for_faa_dage_er_ingen_sammenligning() {
        let rows = [doegn(3, 10, 2), doegn(2, 10, 2), doegn(1, 500, 150)].concat();
        let v = vurder(&rows, &d(1));
        assert_eq!(
            v.iter().map(|b| (b.id, b.ok, b.uden_normal)).collect::<Vec<_>>(),
            [("dns_nx", Some(false), true), ("dns_navne", Some(false), true)]
        );
        assert_eq!(signal(&rows, &d(1)), None);
    }

    /// Normalt 0 med dage nok: gulvet er stadig en rigtig grænse, så linjen beholder sit tegn.
    #[test]
    fn normalt_nul_med_dage_nok_beholder_gulvet() {
        let v = vurder(&[uge(0, 0), doegn(1, 500, 150)].concat(), &d(1));
        assert_eq!(
            v.iter().map(|b| (b.ok, b.m, b.uden_normal)).collect::<Vec<_>>(),
            [(Some(true), 100.0, false), (Some(true), 100.0, false)]
        );
    }
}
