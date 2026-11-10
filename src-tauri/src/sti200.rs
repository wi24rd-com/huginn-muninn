//! Følsomme filer, der svarede 200 (IDEA-011, V-TOKEN.1). En fast liste over stier, der aldrig bør svare 200, hentes med i hovedhentningen
//! (`hoved_query`: `vaern::DOEGN_QUERY`, der er W's forespørgsel ordret, plus to aliaser), og svarene bliver til art `sti200` (antal) og `sti200b`
//! (bytes i alt) pr. sti. Forsiden (`/`) står i de samme arter til sammenligning: den er aldrig på listen.
//!
//! **200 beviser ikke, at filen findes.** Målt live (INVESTIGATION-V-TOKEN.1 §A2) svarede 3 af 13 domæner 200 på 31–35 af 39 stier med
//! sidens almindelige side (en "catch-all"). Derfor er en træffer kun troværdig (`troevaerdig`), når svaret er lille, ikke ligner forsiden,
//! og domænet ikke svarer 200 på mange af stierne på én gang. Kun troværdige træffere ændrer status (Handl). De andre står på Angreb som
//! "ligner sidens almindelige side".
use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{json, Value};

use crate::cf::Fejl;
use crate::store::Raekke;
use crate::vaern::{dag_ms, dag_noegle, doegn_variabler, Gruppe, DOEGN_MS, DOEGN_QUERY};

/// Stier, der aldrig bør svare 200: valgt blandt dem, angribere faktisk prøvede på målte domæner (kategorierne `hemmeligheder` og `kildekode`
/// i `vaern::forsoeg_kategori`). Ikke med: `/wp-config.php` og `/config.php` (PHP-filer svarer 200 med en tom side, når de køres) og
/// `/config.json`, `/.env.example`, `/.env.dist` (kan ligge åbent med vilje).
pub const STIER: &[&str] = &[
    "/.env",
    "/.env.local",
    "/.env.production",
    "/.env.prod",
    "/.env.development",
    "/.env.dev",
    "/.env.backup",
    "/.env.bak",
    "/.env.old",
    "/.env.save",
    "/.env.staging",
    "/.env.docker",
    "/.env.swp",
    "/.env~",
    "/api/.env",
    "/app/.env",
    "/backend/.env",
    "/public/.env",
    "/laravel/.env",
    "/config/.env",
    "/.npmrc",
    "/credentials.json",
    "/secrets.json",
    "/.aws/credentials",
    "/.aws/config",
    "/docker-compose.yml",
    "/.htpasswd",
    "/wp-config.php.bak",
    "/wp-config.php.old",
    "/wp-config.php~",
    "/.ssh/id_rsa",
    "/id_rsa",
    "/.git/config",
    "/.git/HEAD",
    "/.git/index",
    "/.svn/entries",
    "/.svn/wc.db",
    "/.hg/hgrc",
    "/.DS_Store",
];

/// Forsiden i arterne, til sammenligning.
pub const FORSIDE: &str = "/";
/// Arterne. Begge hører til hovedhentningen (`store::DOEGN_ARTER`).
pub const ART: &str = "sti200";
pub const ART_BYTES: &str = "sti200b";
/// Et svar over så mange bytes i snit er en side, ikke en hemmelighed (målt: catch-all-sider på 20–97 KB, en rigtig `/.git/HEAD` er ca. 23 bytes plus headere).
pub const MAKS_BYTES: i64 = 8192;
/// Et svar, der ligger inden for så stor en andel af forsidens størrelse, er forsiden (målt: −9 % til +15 % på et catch-all-domæne).
pub const SOM_FORSIDEN: f64 = 0.25;
/// Svarer domænet 200 på flere end så mange af stierne, er det en fælles side (målt: 31 og 35 af 39).
pub const MAKS_STIER: usize = 6;
/// Forsiden og bredden ses over så mange døgn til og med statusdøgnet.
const VINDUE: i64 = 7;
/// Højst så mange stier nævnes i teksten.
pub const NAEVN: usize = 3;

/// `DOEGN_QUERY` med de to aliaser og variablen `$sti200`. W-delen er uændret (testet).
pub fn hoved_query() -> String {
    let aliaser = r#"    sti200: httpRequestsAdaptiveGroups(limit:100, filter:{datetime_geq:$since, datetime_lt:$until, edgeResponseStatus:200, clientRequestPath_in:$sti200}, orderBy:[count_DESC]) { count sum { edgeResponseBytes } dimensions { clientRequestPath } }
    forside: httpRequestsAdaptiveGroups(limit:1, filter:{datetime_geq:$since, datetime_lt:$until, edgeResponseStatus:200, clientRequestPath:"/"}, orderBy:[count_DESC]) { count sum { edgeResponseBytes } dimensions { clientRequestPath } }
  } } }"#;
    let hoved = DOEGN_QUERY.replacen("$until: Time!)", "$until: Time!, $sti200: [string!]!)", 1);
    match hoved.strip_suffix("  } } }") {
        Some(krop) => format!("{krop}{aliaser}"),
        None => hoved,
    }
}

/// Variablerne til `hoved_query`: `doegn_variabler` plus listen.
pub fn variabler(zone: &str, dag: &str) -> Result<Value, Fejl> {
    let mut v = doegn_variabler(zone, dag)?;
    v["sti200"] = json!(STIER);
    Ok(v)
}

/// Svarene → rækker. Kun stier fra listen, og forsiden. Ren funktion.
pub fn raekker(sti200: &[Gruppe], forside: &[Gruppe]) -> Vec<(String, String, i64)> {
    let mut ud: BTreeMap<(&str, &str), i64> = BTreeMap::new();
    let mut tael = |sti: &'static str, g: &Gruppe| {
        *ud.entry((ART, sti)).or_default() += g.count;
        *ud.entry((ART_BYTES, sti)).or_default() += g.sum.bytes;
    };
    for g in sti200 {
        if let Some(sti) = STIER.iter().find(|s| **s == g.dimensions.sti) {
            tael(sti, g);
        }
    }
    for g in forside.iter().filter(|g| g.dimensions.sti == FORSIDE) {
        tael(FORSIDE, g);
    }
    ud.into_iter()
        .filter(|(_, n)| *n > 0)
        .map(|((art, sti), n)| (art.to_string(), sti.to_string(), n))
        .collect()
}

/// Én sti i en periode.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct StiSvar {
    pub sti: String,
    pub n: i64,
    /// Gennemsnitlig svarstørrelse i bytes.
    pub snit: i64,
    /// Ligner svaret filen (lille og ikke forsiden), og svarer domænet ikke 200 på mange af stierne?
    pub troevaerdig: bool,
}

/// Antal og bytes pr. sti for dagene `fra`…`til`.
fn summer<'a>(rows: &'a [Raekke], fra: &str, til: &str) -> BTreeMap<&'a str, (i64, i64)> {
    let mut m: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.dag.as_str() >= fra && r.dag.as_str() <= til) {
        let e = m.entry(r.noegle.as_str());
        if r.art == ART {
            e.or_default().0 += r.n;
        } else if r.art == ART_BYTES {
            e.or_default().1 += r.n;
        }
    }
    m
}

/// Ligner et snit på `snit` bytes forsidens snit `forside` (inden for `SOM_FORSIDEN`)? En forside på 0 bytes ligner intet. Ren funktion; bruges også af
/// Besøgende › Mest besøgte sider (`besoeg::forside_deling`).
pub fn som_forsiden(snit: i64, forside: i64) -> bool {
    forside > 0 && (snit - forside).abs() as f64 <= SOM_FORSIDEN * forside as f64
}

/// Er et snit på `snit` bytes troværdigt mod forsidens snit, når `stier` forskellige stier svarede 200? Ren funktion.
pub fn troevaerdig(snit: i64, forside: Option<i64>, stier: usize) -> bool {
    let Some(f) = forside.filter(|f| *f > 0) else {
        return false; // uden forsiden kan en fælles side ikke udelukkes
    };
    snit <= MAKS_BYTES && !som_forsiden(snit, f) && stier <= MAKS_STIER
}

/// Stierne i perioden med deres vurdering, flest først. `forside`: forsidens snit i perioden. Ren funktion.
pub fn i_perioden(rows: &[Raekke], fra: &str, til: &str) -> (Vec<StiSvar>, Option<i64>) {
    let m = summer(rows, fra, til);
    let forside = m.get(FORSIDE).filter(|(n, _)| *n > 0).map(|(n, b)| b / n);
    let stier: Vec<(&str, i64, i64)> = m
        .iter()
        .filter(|(s, (n, _))| **s != FORSIDE && *n > 0)
        .map(|(s, (n, b))| (*s, *n, b / n))
        .collect();
    let bredde = stier.len();
    let mut ud: Vec<StiSvar> = stier
        .into_iter()
        .map(|(sti, n, snit)| StiSvar {
            sti: sti.to_string(),
            n,
            snit,
            troevaerdig: troevaerdig(snit, forside, bredde),
        })
        .collect();
    ud.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.sti.cmp(&b.sti)));
    (ud, forside)
}

/// Statusdøgnets troværdige træffere: giver Handl.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Sti200Dag {
    pub n: i64,
    /// De troværdige stier, flest først.
    pub stier: Vec<String>,
}

/// Troværdige træffere i `dag`. Forsiden og bredden tages fra de `VINDUE` døgn til og med `dag`, så et stille døgn ikke ligner noget andet,
/// end det gør. Ren funktion.
pub fn fund(rows: &[Raekke], dag: &str) -> Option<Sti200Dag> {
    let fra = dag_ms(dag).map(|ms| dag_noegle(ms - (VINDUE - 1) * DOEGN_MS))?;
    let (uge, forside) = i_perioden(rows, &fra, dag);
    let bredde = uge.len();
    let (doegn, doegn_forside) = i_perioden(rows, dag, dag);
    let forside = doegn_forside.or(forside);
    let stier: Vec<&StiSvar> = doegn
        .iter()
        .filter(|s| troevaerdig(s.snit, forside, bredde.max(doegn.len())))
        .collect();
    (!stier.is_empty()).then(|| Sti200Dag {
        n: stier.iter().map(|s| s.n).sum(),
        stier: stier.iter().map(|s| s.sti.clone()).collect(),
    })
}

/// Afsnittet på Angreb for ét domæne og en periode.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Sti200Ud {
    pub stier: Vec<StiSvar>,
    /// Forsidens snit i perioden (bytes), til teksten om sammenligningen.
    pub forside: Option<i64>,
    /// Døgn i perioden, hvor hovedhentningen har kørt: uden dem kan "ingen" ikke siges.
    pub hentede_dage: usize,
    /// Så mange stier er på listen.
    pub liste: usize,
}

pub fn oversigt(rows: &[Raekke], fra: &str, til: &str, hentede_dage: usize) -> Sti200Ud {
    let (stier, forside) = i_perioden(rows, fra, til);
    Sti200Ud {
        stier,
        forside,
        hentede_dage,
        liste: STIER.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaern::{forsoeg_kategori, Doegn};

    fn r(dag: &str, art: &str, sti: &str, n: i64) -> Raekke {
        Raekke {
            dag: dag.into(),
            art: art.into(),
            noegle: sti.into(),
            n,
        }
    }
    /// En sti med antal og snit på en dag.
    fn sti(dag: &str, s: &str, n: i64, snit: i64) -> [Raekke; 2] {
        [r(dag, ART, s, n), r(dag, ART_BYTES, s, n * snit)]
    }
    fn rows(dele: &[[Raekke; 2]]) -> Vec<Raekke> {
        dele.iter().flat_map(|d| d.iter().cloned()).collect()
    }

    /// Renset svar fra et målt domæne (INVESTIGATION-V-TOKEN.1 §A2, domæne-C): tre stier, ens store, og større end forsiden.
    fn domaene_c() -> Doegn {
        serde_json::from_value(serde_json::json!({
            "forside": [{"count": 45, "dimensions": {"clientRequestPath": "/"}, "sum": {"edgeResponseBytes": 450672}}],
            "sti200": [
                {"count": 2, "dimensions": {"clientRequestPath": "/.git/HEAD"}, "sum": {"edgeResponseBytes": 41556}},
                {"count": 1, "dimensions": {"clientRequestPath": "/.env"}, "sum": {"edgeResponseBytes": 20745}},
                {"count": 1, "dimensions": {"clientRequestPath": "/.git/config"}, "sum": {"edgeResponseBytes": 20811}}
            ]
        }))
        .unwrap()
    }

    /// Renset svar fra et målt catch-all-domæne (domæne-M): 35 af 39 stier, alle på størrelse med forsiden.
    fn domaene_m() -> Doegn {
        serde_json::from_str(include_str!("sti200/domaene-m.json")).unwrap()
    }

    #[test]
    fn listen_er_hemmeligheder_og_kildekode_og_hoejst_40() {
        assert!(STIER.len() <= 40, "højst 40 stier");
        for s in STIER {
            assert!(
                matches!(forsoeg_kategori(s), Some("hemmeligheder" | "kildekode")),
                "{s} er ikke i hemmeligheder/kildekode"
            );
            assert!(s.starts_with('/') && !s.contains('?'), "{s}: præcis sti");
        }
        let mut u: Vec<&&str> = STIER.iter().collect();
        u.sort();
        u.dedup();
        assert_eq!(u.len(), STIER.len(), "ingen dubletter");
        assert!(!STIER.contains(&FORSIDE));
        for ikke in ["/wp-config.php", "/config.php", "/config.json", "/.env.example"] {
            assert!(!STIER.contains(&ikke), "{ikke} svarer 200 helt lovligt");
        }
    }

    #[test]
    fn hoved_query_er_doegn_query_plus_to_aliaser() {
        let q = hoved_query();
        assert!(q.starts_with("query Vaernet($zoneTag: string!, $since: Time!, $until: Time!, $sti200: [string!]!)"));
        let w_krop = DOEGN_QUERY.split_once(')').unwrap().1.strip_suffix("  } } }").unwrap();
        assert!(q.contains(w_krop), "W-delen er ordret med");
        assert!(q.contains("sti200: httpRequestsAdaptiveGroups(") && q.contains("clientRequestPath_in:$sti200"));
        assert!(q.contains("forside: httpRequestsAdaptiveGroups(") && q.contains("clientRequestPath:\"/\""));
        assert!(q.trim_end().ends_with("} } }"));
        assert_eq!(q.matches("edgeResponseStatus:200").count(), 2);
        assert!(!q.to_lowercase().contains("clientip"), "ingen IP-adresser");
        assert!(!q.contains("mutation"));
        let v = variabler("abc", "2026-11-07").unwrap();
        assert_eq!(v["sti200"].as_array().unwrap().len(), STIER.len());
        assert_eq!(
            (v["since"].as_str(), v["until"].as_str()),
            (Some("2026-11-07T00:00:00Z"), Some("2026-11-08T00:00:00Z"))
        );
        assert!(!q.contains("/.env"), "listen er en variabel, ikke en del af teksten");
    }

    #[test]
    fn raekker_kun_fra_listen_og_forsiden() {
        let z: Doegn = serde_json::from_value(serde_json::json!({
            "sti200": [
                {"count": 3, "dimensions": {"clientRequestPath": "/.env"}, "sum": {"edgeResponseBytes": 900}},
                {"count": 5, "dimensions": {"clientRequestPath": "/ikke-paa-listen"}, "sum": {"edgeResponseBytes": 5}}
            ],
            "forside": [{"count": 10, "dimensions": {"clientRequestPath": "/"}, "sum": {"edgeResponseBytes": 300000}}]
        }))
        .unwrap();
        assert_eq!(
            raekker(&z.sti200, &z.forside),
            vec![
                (ART.into(), "/".into(), 10),
                (ART.into(), "/.env".into(), 3),
                (ART_BYTES.into(), "/".into(), 300000),
                (ART_BYTES.into(), "/.env".into(), 900),
            ]
        );
        assert_eq!(raekker(&[], &[]), vec![], "intet svar, ingen rækker");
        let uden_sum: Doegn =
            serde_json::from_value(serde_json::json!({"sti200": [{"count": 1, "dimensions": {"clientRequestPath": "/.env"}}]})).unwrap();
        assert_eq!(
            raekker(&uden_sum.sti200, &[]),
            vec![(ART.into(), "/.env".into(), 1)],
            "uden sum: kun antallet"
        );
    }

    #[test]
    fn maalte_catch_all_domaener_giver_ikke_handl() {
        for (navn, z) in [("domæne-C", domaene_c()), ("domæne-M", domaene_m())] {
            let rk: Vec<Raekke> = raekker(&z.sti200, &z.forside)
                .into_iter()
                .map(|(art, sti, n)| r("2026-11-07", &art, &sti, n))
                .collect();
            assert!(rk.iter().any(|x| x.art == ART && x.noegle != FORSIDE), "{navn} har træffere");
            assert_eq!(fund(&rk, "2026-11-07"), None, "{navn}: ligner forsiden eller en side");
            let (stier, _) = i_perioden(&rk, "2026-11-07", "2026-11-07");
            assert!(stier.iter().all(|s| !s.troevaerdig), "{navn}: ingen troværdige på Angreb");
        }
        assert_eq!(i_perioden(&domaene_m_rows(), "2026-11-07", "2026-11-07").0.len(), 35);
    }

    fn domaene_m_rows() -> Vec<Raekke> {
        let z = domaene_m();
        raekker(&z.sti200, &z.forside)
            .into_iter()
            .map(|(art, sti, n)| r("2026-11-07", &art, &sti, n))
            .collect()
    }

    #[test]
    fn en_lille_fil_der_ikke_ligner_forsiden_giver_handl() {
        let d = "2026-11-07";
        let rk = rows(&[sti(d, "/", 40, 30_000), sti(d, "/.env", 2, 900), sti(d, "/.git/HEAD", 1, 700)]);
        assert_eq!(
            fund(&rk, d),
            Some(Sti200Dag {
                n: 3,
                stier: vec!["/.env".into(), "/.git/HEAD".into()]
            })
        );
        // Kun forsiden fra tidligere i ugen: den bruges.
        let rk = rows(&[sti("2026-11-04", "/", 40, 30_000), sti(d, "/.env", 1, 900)]);
        assert_eq!(fund(&rk, d).map(|f| f.n), Some(1));
        // Forsiden for længe siden: kan ikke afgøres, intet Handl.
        let rk = rows(&[sti("2026-10-01", "/", 40, 30_000), sti(d, "/.env", 1, 900)]);
        assert_eq!(fund(&rk, d), None);
        // Et andet døgn tæller ikke i statusdøgnet.
        let rk = rows(&[sti(d, "/", 40, 30_000), sti("2026-11-06", "/.env", 1, 900)]);
        assert_eq!(fund(&rk, d), None);
    }

    #[test]
    fn grænserne_for_troevaerdig() {
        assert!(troevaerdig(900, Some(30_000), 1));
        assert!(troevaerdig(MAKS_BYTES, Some(30_000), 1), "præcis grænsen tæller med");
        assert!(!troevaerdig(MAKS_BYTES + 1, Some(30_000), 1), "for stor: en side");
        assert!(!troevaerdig(1_647, Some(1_675), 1), "som forsiden");
        assert!(!troevaerdig(1_675 + 418, Some(1_675), 1), "under 25 % fra forsiden");
        assert!(troevaerdig(1_675 + 419, Some(1_675), 1), "over 25 % fra forsiden");
        assert!(!troevaerdig(900, None, 1), "uden forsiden kan intet afgøres");
        assert!(!troevaerdig(900, Some(0), 1));
        assert!(troevaerdig(900, Some(30_000), MAKS_STIER));
        assert!(
            !troevaerdig(900, Some(30_000), MAKS_STIER + 1),
            "200 på mange af stierne: en fælles side"
        );
    }

    #[test]
    fn mange_stier_i_ugen_goer_doegnets_ene_utrovaerdig() {
        let d = "2026-11-07";
        let mut dele = vec![sti(d, "/", 40, 30_000), sti(d, "/.env", 1, 900)];
        for s in &STIER[1..=MAKS_STIER] {
            dele.push(sti("2026-11-05", s, 1, 900));
        }
        assert_eq!(fund(&rows(&dele), d), None, "{} stier i ugen", MAKS_STIER + 1);
        dele.pop();
        assert!(fund(&rows(&dele), d).is_some(), "{MAKS_STIER} stier i ugen er stadig troværdigt");
    }

    #[test]
    fn oversigten_sorterer_og_tæller_perioden() {
        let rk = rows(&[
            sti("2026-11-01", "/", 10, 30_000),
            sti("2026-11-01", "/.env", 1, 900),
            sti("2026-11-03", "/.env", 2, 900),
            sti("2026-11-03", "/.git/config", 5, 40_000),
            sti("2026-11-09", "/.npmrc", 9, 100),
        ]);
        let o = oversigt(&rk, "2026-11-01", "2026-11-07", 7);
        assert_eq!(o.forside, Some(30_000));
        assert_eq!(o.liste, STIER.len());
        assert_eq!(
            o.stier,
            vec![
                StiSvar {
                    sti: "/.git/config".into(),
                    n: 5,
                    snit: 40_000,
                    troevaerdig: false
                },
                StiSvar {
                    sti: "/.env".into(),
                    n: 3,
                    snit: 900,
                    troevaerdig: true
                },
            ]
        );
    }
}
