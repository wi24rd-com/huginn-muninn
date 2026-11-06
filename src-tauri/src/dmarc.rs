//! "Mails sendt i dit domænes navn": `dmarcReportsSourcesAdaptiveGroups` for færdige døgn. Noden er slået til på Free (målt live, 32 dage), men dens felter står kun i bilag A: hverken Cloudflares
//! dokumentation eller målingen nævner dem. Derfor så få felter som muligt, et fald tilbage, og Cloudflares fejl vises i afsnittet (`Store::afsnit_fejl`). `sum.dmarc` bruges ikke, for hvad den tæller, står ingen steder.
//! DMARC-resultatet regnes i stedet fra `dkim` og `spf` efter RFC 7489: en mail består, hvis en af dem består.
//! Aldrig IP-adresser: de står ikke i forespørgslen, og `ren_afsender` fjerner en adresse, der kommer som navn. Der gemmes kun afsender, land, antal og resultat.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cf::Fejl;
use crate::overblik::dag_foer;
use crate::store::Raekke;
use crate::vaern::{dag_ms, er_faerdig, iso, js_slice, parse_iso, tom_ved_null, DOEGN_MS};

pub const DMARC_QUERY: &str = r#"query Dmarc($zoneTag: string!, $dag: Date!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    dmarc: dmarcReportsSourcesAdaptiveGroups(limit:1000, filter:{date_geq:$dag, date_leq:$dag}, orderBy:[sum_totalMatchingMessages_DESC]) { sum { totalMatchingMessages } dimensions { sourceOrgName sourceLocationName dkim spf } }
  } } }"#;

/// Fald tilbage, hvis Cloudflare afviser sorteringen eller landet.
pub fn query_enkel() -> String {
    DMARC_QUERY
        .replace(", orderBy:[sum_totalMatchingMessages_DESC]", "")
        .replace(" sourceLocationName", "")
}

/// Afviste Cloudflare forespørgslen? Så prøves `query_enkel`.
pub fn afvist(e: &Fejl) -> bool {
    matches!(e, Fejl::GraphQl { .. }) && !e.uden_for_opbevaring()
}

/// Rapporter kommer dagen efter eller senere, så et døgn hentes igen, til det er `SENT_MS` gammelt, og højst hver `GENHENT_MS`.
pub const SENT_MS: i64 = 3 * DOEGN_MS;
pub const GENHENT_MS: i64 = 6 * 3_600_000;
/// De seneste døgn, signalet ser på: rapporterne for i går er sjældent komplette.
const SENESTE: i64 = 3;
/// Så mange døgn med rapporter før `SENESTE`, før noget kan kaldes nyt: ellers er alle afsendere nye.
pub const BASIS_DAGE: usize = 7;
/// Andelen, der fejler, er steget markant: mindst så mange mails, mindst så mange procent, og mindst `ANDEL_FAKTOR` gange det normale.
pub const ANDEL_MIN_MAILS: i64 = 10;
pub const ANDEL_MIN_PROCENT: i64 = 20;
pub const ANDEL_FAKTOR: i64 = 2;
/// Højst så mange nye afsendere nævnes i teksten, og højst så mange står i listen.
const NYE_VIST: usize = 3;
const LISTE_MAKS: usize = 50;
const NAVN_MAKS: usize = 120;

/// Variablerne til ét døgn.
pub fn variabler(zone: &str, dag: &str) -> Result<Value, Fejl> {
    dag_ms(dag).ok_or(Fejl::UgyldigDato)?;
    Ok(json!({ "zoneTag": zone, "dag": dag }))
}

/// Skal døgnet hentes (igen)? Ren funktion.
pub fn mangler(dag: &str, hentet: &HashMap<String, String>, nu: i64) -> bool {
    if !er_faerdig(dag, &iso(nu)) {
        return false;
    }
    let (Some(h), Some(start)) = (hentet.get(dag).and_then(|h| parse_iso(h)), dag_ms(dag)) else {
        return !hentet.contains_key(dag);
    };
    h < start + SENT_MS && nu - h >= GENHENT_MS
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Sum {
    #[serde(default, deserialize_with = "tom_ved_null", rename = "totalMatchingMessages")]
    mails: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Gruppe {
    #[serde(default, deserialize_with = "tom_ved_null")]
    sum: Sum,
    #[serde(default)]
    dimensions: Value,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Svar {
    #[serde(default, deserialize_with = "tom_ved_null")]
    dmarc: Vec<Gruppe>,
}

fn felt(g: &Gruppe, navn: &str) -> String {
    match g.dimensions.get(navn) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// Afsenderens navn: tabulatorer væk, og en IP-adresse som navn gemmes aldrig. Ren funktion.
pub fn ren_afsender(s: &str) -> String {
    let s = s.replace(['\t', '\n', '\r'], " ");
    let s = s.trim();
    if s.parse::<IpAddr>().is_ok() {
        String::new()
    } else {
        js_slice(s, NAVN_MAKS)
    }
}

/// Art `dmarc`: "afsender\tland\tdkim\tspf". Ren funktion.
pub fn raekker(data: &Value) -> Result<Vec<(String, String, i64)>, Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    let svar = Svar::deserialize(z).map_err(|_| Fejl::Svar)?;
    let mut ud: BTreeMap<String, i64> = BTreeMap::new();
    for g in svar.dmarc.iter().filter(|g| g.sum.mails > 0) {
        let k = format!(
            "{}\t{}\t{}\t{}",
            ren_afsender(&felt(g, "sourceOrgName")),
            ren_afsender(&felt(g, "sourceLocationName")),
            felt(g, "dkim").to_lowercase(),
            felt(g, "spf").to_lowercase()
        );
        *ud.entry(k).or_default() += g.sum.mails;
    }
    Ok(ud.into_iter().map(|(k, n)| ("dmarc".to_string(), k, n)).collect())
}

/// Bestod mailen? RFC 7489: det er nok, at DKIM eller SPF består. Uden noget resultat kan det ikke kaldes en fejl. Ren funktion.
fn fejlede(dkim: &str, spf: &str) -> bool {
    let bestaaet = |v: &str| matches!(v, "pass" | "true" | "1");
    !(dkim.is_empty() && spf.is_empty()) && !bestaaet(dkim) && !bestaaet(spf)
}

/// Én nøgle: (afsender, land, fejlede?).
fn del(noegle: &str) -> (&str, &str, bool) {
    let mut f = noegle.split('\t');
    let (org, land) = (f.next().unwrap_or(""), f.next().unwrap_or(""));
    let (dkim, spf) = (f.next().unwrap_or(""), f.next().unwrap_or(""));
    (org, land, fejlede(dkim, spf))
}

/// Mails og fejlede for rækkerne.
fn tal<'a>(rows: impl Iterator<Item = &'a Raekke>) -> (i64, i64) {
    rows.fold((0, 0), |(m, f), r| (m + r.n, f + if del(&r.noegle).2 { r.n } else { 0 }))
}

/// "Hold øje" for domænet. Ingen sti, ingen IP-adresse.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DmarcFund {
    pub dag: String,
    /// Afsendere, der ikke er set før, og som fejlede.
    pub nye: Vec<String>,
    /// Andelen, der fejlede, steg markant: procent og det normale.
    pub stiger: bool,
    pub procent: i64,
    pub normalt: i64,
}

/// Ny afsender eller en markant stigning i de seneste `SENESTE` døgn, målt mod døgnene før. Ren funktion.
pub fn fund(rows: &[Raekke], dag: &str) -> Option<DmarcFund> {
    let graense = dag_foer(dag, SENESTE - 1);
    let (seneste, foer): (Vec<&Raekke>, Vec<&Raekke>) = rows
        .iter()
        .filter(|r| r.art == "dmarc" && r.dag.as_str() <= dag)
        .partition(|r| r.dag >= graense);
    if foer.iter().map(|r| r.dag.as_str()).collect::<HashSet<_>>().len() < BASIS_DAGE {
        return None;
    }
    let kendte: HashSet<&str> = foer.iter().map(|r| del(&r.noegle).0).collect();
    let mut nye: Vec<String> = seneste
        .iter()
        .map(|r| del(&r.noegle))
        .filter(|(org, _, fejl)| *fejl && !org.is_empty() && !kendte.contains(org))
        .map(|(org, _, _)| org.to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    nye.sort();
    nye.truncate(NYE_VIST);
    let ((m, f), (m0, f0)) = (tal(seneste.iter().copied()), tal(foer.iter().copied()));
    let (procent, normalt) = (f * 100 / m.max(1), f0 * 100 / m0.max(1));
    let stiger = m >= ANDEL_MIN_MAILS && procent >= ANDEL_MIN_PROCENT && procent >= ANDEL_FAKTOR * normalt && procent > normalt;
    (stiger || !nye.is_empty()).then(|| DmarcFund {
        dag: dag.to_string(),
        nye,
        stiger,
        procent,
        normalt,
    })
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct AfsenderUd {
    pub afsender: String,
    pub land: String,
    pub mails: i64,
    pub fejlet: i64,
}

/// Visningen for ét domæne. `fejl`: Cloudflare svarede ikke på DMARC-tallene ved seneste forsøg.
#[derive(Serialize, Debug, Clone, PartialEq, Default)]
pub struct EmailUd {
    pub mails: i64,
    pub fejlet: i64,
    /// Forskellige afsendere, der fejlede.
    pub afsendere: usize,
    /// De, der fejlede, flest først.
    pub liste: Vec<AfsenderUd>,
    /// Hentede døgn. 0: der er ikke spurgt endnu, så tomheden siger ingenting.
    pub hentede_dage: usize,
    pub fejl: Option<String>,
}

/// Rækkerne samlet pr. afsender. Ren funktion.
pub fn oversigt(rows: &[Raekke], hentede_dage: usize, fejl: Option<String>) -> EmailUd {
    let mut pr: HashMap<(&str, &str), (i64, i64)> = HashMap::new();
    let (mut mails, mut fejlet) = (0, 0);
    for r in rows.iter().filter(|r| r.art == "dmarc") {
        let (org, land, fejl) = del(&r.noegle);
        let x = pr.entry((org, land)).or_default();
        x.0 += r.n;
        mails += r.n;
        if fejl {
            x.1 += r.n;
            fejlet += r.n;
        }
    }
    let mut liste: Vec<AfsenderUd> = pr
        .into_iter()
        .filter(|(_, x)| x.1 > 0)
        .map(|((org, land), (m, f))| AfsenderUd {
            afsender: org.to_string(),
            land: land.to_string(),
            mails: m,
            fejlet: f,
        })
        .collect();
    liste.sort_by(|a, b| {
        b.fejlet
            .cmp(&a.fejlet)
            .then_with(|| a.afsender.cmp(&b.afsender))
            .then_with(|| a.land.cmp(&b.land))
    });
    let afsendere = liste.iter().map(|x| x.afsender.as_str()).collect::<HashSet<_>>().len();
    liste.truncate(LISTE_MAKS);
    EmailUd {
        mails,
        fejlet,
        afsendere,
        liste,
        hentede_dage,
        fejl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaern::dag_noegle;

    const NU: i64 = 1_793_000_000_000;

    fn d(i: i64) -> String {
        dag_noegle(NU - i * DOEGN_MS)
    }
    fn rk(dag: &str, org: &str, dkim: &str, spf: &str, n: i64) -> Raekke {
        Raekke {
            dag: dag.into(),
            art: "dmarc".into(),
            noegle: format!("{org}\tDK\t{dkim}\t{spf}"),
            n,
        }
    }
    /// Otte døgn med rapporter før de seneste tre.
    fn historik(org: &str, ok: i64, fejl: i64) -> Vec<Raekke> {
        (5..=12)
            .flat_map(|i| [rk(&d(i), org, "pass", "pass", ok), rk(&d(i), org, "fail", "fail", fejl)])
            .collect()
    }
    fn svar(g: Value) -> Value {
        json!({ "viewer": { "zones": [{ "dmarc": g }] } })
    }

    /// Ingen IP-adresser i forespørgslen, heller ikke i faldet tilbage, og `sum.dmarc` bruges ikke.
    #[test]
    fn forespoergslen_har_ingen_ip_felter() {
        for q in [DMARC_QUERY.to_string(), query_enkel()] {
            let felter = q.to_lowercase();
            for f in ["sourceip", "ipaddress", "ip_address", "clientip", "ipcount"] {
                assert!(!felter.contains(f), "{f}");
            }
            assert!(q.contains("sum { totalMatchingMessages }"));
        }
        let enkel = query_enkel();
        assert!(!enkel.contains("orderBy") && !enkel.contains("sourceLocationName") && enkel.contains("sourceOrgName dkim spf"));
    }

    /// En IP-adresse, der kommer som afsender eller land, gemmes aldrig, og felter, Cloudflare skulle finde på at sende, kommer ikke med.
    #[test]
    fn en_ip_adresse_gemmes_aldrig() {
        assert_eq!(ren_afsender("192.0.2.7"), "");
        assert_eq!(ren_afsender("2001:db8::1"), "");
        assert_eq!(ren_afsender(" Google\tLLC "), "Google LLC");
        let r = raekker(&svar(json!([
            { "sum": { "totalMatchingMessages": 4 }, "dimensions": { "sourceOrgName": "198.51.100.9", "sourceLocationName": "203.0.113.5", "sourceIP": "192.0.2.1", "dkim": "fail", "spf": "fail" } },
        ])))
        .unwrap();
        assert_eq!(r, [("dmarc".to_string(), "\t\tfail\tfail".to_string(), 4)]);
        assert!(!format!("{r:?}").contains("192.0.2"));
    }

    #[test]
    fn raekker_laegger_sammen_og_springer_tomme_over() {
        let g = |org: &str, n: i64| json!({ "sum": { "totalMatchingMessages": n }, "dimensions": { "sourceOrgName": org, "sourceLocationName": "DK", "dkim": "PASS", "spf": "fail" } });
        let r = raekker(&svar(json!([g("A", 2), g("A", 3), g("B", 0)]))).unwrap();
        assert_eq!(r, [("dmarc".to_string(), "A\tDK\tpass\tfail".to_string(), 5)]);
        assert_eq!(raekker(&svar(json!([]))).unwrap(), []);
    }

    /// RFC 7489: det er nok, at en af dem består, og uden resultat er det ikke en fejl.
    #[test]
    fn dmarc_bestaas_af_dkim_eller_spf() {
        assert!(!fejlede("pass", "fail") && !fejlede("fail", "pass") && !fejlede("pass", "pass"));
        assert!(fejlede("fail", "fail") && fejlede("none", "softfail") && fejlede("", "fail"));
        assert!(!fejlede("", ""));
    }

    /// Et døgn hentes igen, til rapporterne er kommet, men ikke hver time.
    #[test]
    fn doegn_hentes_igen_mens_rapporterne_kommer() {
        let dag = d(1);
        let h = |ms: i64| HashMap::from([(dag.clone(), iso(ms))]);
        let start = dag_ms(&dag).unwrap();
        assert!(mangler(&dag, &HashMap::new(), start + 2 * DOEGN_MS), "aldrig hentet");
        assert!(!mangler(&dag, &HashMap::new(), start + DOEGN_MS / 2), "døgnet er ikke slut");
        assert!(
            !mangler(&dag, &h(start + 2 * DOEGN_MS), start + 2 * DOEGN_MS + 3_600_000),
            "hentet for en time siden"
        );
        assert!(
            mangler(&dag, &h(start + 2 * DOEGN_MS), start + 2 * DOEGN_MS + GENHENT_MS),
            "seks timer senere"
        );
        assert!(!mangler(&dag, &h(start + SENT_MS), start + 10 * DOEGN_MS), "sent nok er endeligt");
    }

    #[test]
    fn ny_afsender_der_fejler_giver_fund() {
        let mut rows = historik("Kendt", 50, 0);
        rows.push(rk(&d(1), "Ny Afsender", "fail", "fail", 3));
        let f = fund(&rows, &d(1)).unwrap();
        assert_eq!((f.nye.as_slice(), f.stiger), (["Ny Afsender".to_string()].as_slice(), false));
    }

    /// En kendt afsender, en ny, der består, og en uden navn er ikke nye afsendere, der fejler.
    #[test]
    fn kendt_eller_godkendt_afsender_giver_intet() {
        let mut rows = historik("Kendt", 50, 1);
        rows.push(rk(&d(1), "Kendt", "fail", "fail", 1));
        rows.push(rk(&d(1), "Ny Men Godkendt", "pass", "fail", 40));
        rows.push(rk(&d(1), "", "fail", "fail", 1));
        assert_eq!(fund(&rows, &d(1)), None);
    }

    /// Uden en historik er alle afsendere nye, så der siges ingenting.
    #[test]
    fn uden_historik_siges_intet() {
        let mut rows: Vec<Raekke> = (4..=9).map(|i| rk(&d(i), "Kendt", "pass", "pass", 50)).collect();
        rows.push(rk(&d(1), "Ny", "fail", "fail", 500));
        assert_eq!(fund(&rows, &d(1)), None, "seks døgn før er for lidt");
        rows.push(rk(&d(10), "Kendt", "pass", "pass", 50));
        assert!(fund(&rows, &d(1)).is_some(), "syv døgn før er nok");
    }

    #[test]
    fn andelen_skal_stige_markant() {
        let med = |ok: i64, fejl: i64| {
            let mut rows = historik("Kendt", 90, 10);
            rows.push(rk(&d(1), "Kendt", "pass", "pass", ok));
            rows.push(rk(&d(1), "Kendt", "fail", "fail", fejl));
            fund(&rows, &d(1))
        };
        let f = med(60, 40).unwrap();
        assert_eq!((f.stiger, f.procent, f.normalt), (true, 40, 10));
        assert_eq!(med(85, 15), None, "15 % er under 20 %");
        assert_eq!(med(5, 4), None, "for få mails");
        let mut rows = historik("Kendt", 60, 40);
        rows.push(rk(&d(1), "Kendt", "pass", "pass", 50));
        rows.push(rk(&d(1), "Kendt", "fail", "fail", 50));
        assert_eq!(fund(&rows, &d(1)), None, "50 % mod 40 % er ikke en fordobling");
    }

    #[test]
    fn oversigten_viser_dem_der_fejlede() {
        let rows = vec![
            rk(&d(1), "Gode", "pass", "pass", 100),
            rk(&d(1), "Blandet", "pass", "fail", 10),
            rk(&d(1), "Blandet", "fail", "fail", 3),
            rk(&d(2), "Ond", "fail", "fail", 7),
        ];
        let o = oversigt(&rows, 2, None);
        assert_eq!((o.mails, o.fejlet, o.afsendere, o.hentede_dage), (120, 10, 2, 2));
        assert_eq!(
            o.liste.iter().map(|x| (x.afsender.as_str(), x.mails, x.fejlet)).collect::<Vec<_>>(),
            [("Ond", 7, 7), ("Blandet", 13, 3)]
        );
        assert_eq!(oversigt(&[], 0, Some("x".into())).fejl.as_deref(), Some("x"));
    }
}
