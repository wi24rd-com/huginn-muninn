//! Tests for Værnet. Del A er porteret fra W `worker/test/vagt.test.js` (testnavnene står i kommentarerne). Del B er
//! differentiel: `paritet.json` er genereret af W's egen vagt.js på faste input, og Rust-porten skal give det samme.
//! Del C tester det, der er nyt i programmet: variabler i GraphQL, de seneste blokeringer i cachen og hentelogikken.
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::*;
use crate::cf::graphql_body;
use crate::store::{Raekke, Store};

const NU: i64 = 1_793_250_000_000; // 2026-10-29T05:00:00Z

fn doegn(v: Value) -> Doegn {
    serde_json::from_value(v).unwrap()
}

fn som_rows(dag: &str, rk: &[(String, String, i64)]) -> Vec<Raekke> {
    rk.iter()
        .map(|(art, noegle, n)| Raekke {
            dag: dag.into(),
            art: art.clone(),
            noegle: noegle.clone(),
            n: *n,
        })
        .collect()
}

/// Testdøgnet fra W `vagt.test.js` (DOEGN), som JSON.
fn w_doegn() -> Doegn {
    doegn(json!({
      "fw": [
        { "action": "block", "source": "firewallManaged", "description": "React - RCE - CVE:CVE-2025-55182", "clientCountryName": "US", "clientRequestPath": "/", "clientRequestHTTPMethodName": "POST", "clientASNDescription": "Google LLC", "edgeColoName": "EWR", "datetime": "2026-10-29T03:17:28Z" },
        { "action": "block", "source": "firewallManaged", "description": "React - RCE - CVE:CVE-2025-55182", "clientCountryName": "US", "clientRequestPath": "/", "clientRequestHTTPMethodName": "POST", "clientASNDescription": "Google LLC", "edgeColoName": "EWR", "datetime": "2026-10-29T03:17:27Z" },
        { "action": "block", "source": "firewallManaged", "description": "Wordpress - Broken Access Control, File Inclusion", "clientCountryName": "NL", "clientRequestPath": "/wp-config.php.bak", "clientRequestHTTPMethodName": "GET", "clientASNDescription": "AS QualityNetwork", "edgeColoName": "AMS", "datetime": "2026-10-29T01:00:00Z" }
      ],
      "alle": [{ "count": 32749, "dimensions": { "date": "2026-10-29" } }],
      "probes": [{ "count": 37, "dimensions": { "clientRequestPath": "/.env" } }, { "count": 146, "dimensions": { "clientRequestPath": "/favicon.ico" } }, { "count": 46, "dimensions": { "clientRequestPath": "/.git/config" } }],
      "bots": [
        { "count": 47, "dimensions": { "verifiedBotCategory": "Search Engine Crawler", "userAgent": "Mozilla/5.0 (compatible; Googlebot/2.1; +https://wi24rd.com/bot.html)", "clientRequestPath": "/robots.txt", "edgeResponseStatus": 200 } },
        { "count": 21, "dimensions": { "verifiedBotCategory": "Search Engine Crawler", "userAgent": "Mozilla/5.0 (compatible; Googlebot/2.1; +https://wi24rd.com/bot.html)", "clientRequestPath": "/software/", "edgeResponseStatus": 200 } },
        { "count": 50, "dimensions": { "verifiedBotCategory": "AI Crawler", "userAgent": "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; ClaudeBot/1.0; +claudebot@wi24rd.com)", "clientRequestPath": "/sitemap.xml", "edgeResponseStatus": 200 } },
        { "count": 3, "dimensions": { "verifiedBotCategory": "AI Crawler", "userAgent": "Mozilla/5.0 (compatible; GoogleOther)", "clientRequestPath": "/om/index.md", "edgeResponseStatus": 200 } },
        { "count": 9, "dimensions": { "verifiedBotCategory": "AI Crawler", "userAgent": "Mozilla/5.0 (compatible; GoogleOther)", "clientRequestPath": "/cf-fonts/s/spectral/x.woff2", "edgeResponseStatus": 200 } }
      ],
      "f1": [{ "count": 102, "dimensions": { "userAgent": "Mozilla/5.0 (compatible; Googlebot/2.1; +https://wi24rd.com/bot.html)" } }, { "count": 11, "dimensions": { "userAgent": "Mozilla/5.0 (compatible; DeepSeekBot/1.0)" } }],
      "f2": [{ "count": 11, "dimensions": { "userAgent": "Mozilla/5.0 (compatible; DeepSeekBot/1.0)" } }],
      "f3": []
    }))
}

// ════ A. Porteret fra W worker/test/vagt.test.js ════════════════════════════════════════════════════════════════

fn haendelse(source: &str, n: usize) -> Fw {
    Fw {
        action: "block".into(),
        source: source.into(),
        description: format!("Regel {n}"),
        land: "US".into(),
        sti: format!("/s{n}?hemmelig=1"),
        metode: "GET".into(),
        asn: "AS".into(),
        colo: "CPH".into(),
        datetime: "2026-11-03T05:17:00Z".into(),
        vaegt: 0,
        vaert: None,
    }
}

/// De 200 seneste angreb, uden rutine, med kilde og handling og uden andet end felterne.
#[test]
fn angreb_raekker_200_seneste_uden_rutine() {
    let mut fw: Vec<Fw> = (0..3).map(|n| haendelse("linkMaze", n)).collect();
    fw.extend((0..250).map(|n| haendelse("firewallManaged", n)));
    let z = Doegn { fw, ..Doegn::default() };
    let r = angreb_raekker(&z);
    assert_eq!(r.iter().map(|x| x.2).sum::<i64>(), ANGREB_MAKS as i64);
    assert!(r.iter().all(|x| x.0 == "angreb" && !x.1.contains("linkMaze")), "rutine er ikke med");
    let f: Vec<&str> = r[0].1.split('\t').collect();
    assert_eq!(f.len(), 10);
    assert_eq!(
        (f[0], f[3], f[5], f[6], f[7], f[8], f[9]),
        (
            "2026-11-03T05:00:00Z",
            "US",
            "CPH",
            "GET",
            "firewallManaged",
            "block",
            z.fw[3].asn.as_str()
        )
    );
    assert!(!f[9].is_empty(), "netværket står i nøglen, som dyk filtrerer på");
    assert!(!r[0].1.contains('?'), "stien er renset");
    let sammen = Doegn {
        fw: vec![haendelse("firewallManaged", 1), haendelse("firewallManaged", 1)],
        ..Doegn::default()
    };
    assert_eq!(angreb_raekker(&sammen)[0].2, 2, "samme nøgle lægges sammen");
}

/// Ingen IP-adresse og ingen brugeragent i forespørgslen, og den er `DOEGN_QUERY`s firewall-del ordret, bortset fra `sampleInterval`:
/// enkelthændelserne (art `angreb`) tæller hændelser, ikke vægt (V-SAMPLE.1).
#[test]
fn angreb_query_er_firewall_delen_uden_ip_og_brugeragent() {
    let fw = ANGREB_QUERY.lines().nth(1).unwrap();
    assert!(DOEGN_QUERY
        .replacen(" datetime sampleInterval clientRequestHTTPHost }", " datetime }", 1)
        .contains(fw));
    assert!(!ANGREB_QUERY.contains("sampleInterval"), "angreb tæller hændelser");
    for felt in ["clientIP", "userAgent", "clientIp"] {
        assert!(!ANGREB_QUERY.contains(felt), "{felt}");
    }
}

#[test]
fn w_rensning_stier() {
    // 'rensning: stier og brugeragenter, der må stå på en offentlig side' (renUa er ikke porteret: bruges kun i W's test)
    assert_eq!(ren_sti("/wp-config.php?x=hemmelig"), "/wp-config.php");
    assert_eq!(ren_sti("admin\u{0}/x"), "/admin/x");
    assert_eq!(
        ren_sti(&format!("/.well-known/acme-challenge/{}", "a".repeat(43))),
        "/.well-known/acme-challenge/…"
    );
    assert!(ren_sti(&format!("/{}", "x/".repeat(100))).chars().count() <= 90);
}

#[test]
fn w_bots() {
    // 'bots: navn og firma ud fra brugeragenten, verificeret eller ej'
    let b = bot_navn(
        "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; ClaudeBot/1.0; +claudebot@wi24rd.com)",
        "AI Crawler",
    );
    assert_eq!(
        b,
        Bot {
            navn: "ClaudeBot".into(),
            firma: "Anthropic".into()
        }
    );
    let b = bot_navn("Mozilla/5.0 (compatible; Googlebot/2.1; +https://wi24rd.com/bot.html)", "");
    assert_eq!(
        b,
        Bot {
            navn: "Googlebot".into(),
            firma: "Google".into()
        }
    );
    assert_eq!(
        bot_navn("Mozilla/5.0 (Windows NT 10.0) Chrome/141", "AI Assistant").navn,
        "AI-assistent i en browser"
    );
    assert_eq!(
        bot_navn("Mozilla/5.0 (X11) Chrome/141", "Monitoring & Analytics").navn,
        "Verificeret (Monitoring & Analytics)"
    );
    assert_eq!(bot_navn("Mozilla/5.0 (compatible; DeepSeekBot/1.0)", "").navn, "DeepSeekBot");
    assert_eq!(bot_navn("curl/8.0", "").navn, "Ukendt bot");
    assert!(efterligning("Googlebot"));
    assert!(!efterligning("DeepSeekBot"));
}

#[test]
fn w_regler() {
    // 'regler: produkt, type og CVE ud af WAF-beskrivelsen'
    let r = regel("React - RCE - CVE:CVE-2025-55182", "firewallManaged");
    assert_eq!(
        r,
        RegelInfo {
            navn: "React - RCE - CVE:CVE-2025-55182".into(),
            produkt: "React".into(),
            type_: "RCE".into(),
            cve: Some("CVE-2025-55182".into())
        }
    );
    let r = regel("Wordpress - Broken Access Control, File Inclusion", "");
    assert_eq!(
        r,
        RegelInfo {
            navn: "Wordpress - Broken Access Control, File Inclusion".into(),
            produkt: "Wordpress".into(),
            type_: "Broken Access Control, File Inclusion".into(),
            cve: None
        }
    );
    assert_eq!(regel("", "bic").navn, "Browser Integrity Check");
}

#[test]
fn w_forsoeg() {
    // 'forsøg: hvad 404-stierne ledte efter'
    assert_eq!(forsoeg_kategori("/.env.production"), Some("hemmeligheder"));
    assert_eq!(forsoeg_kategori("/appsettings.Production.json"), Some("hemmeligheder"));
    assert_eq!(forsoeg_kategori("/.git/config"), Some("kildekode"));
    assert_eq!(forsoeg_kategori("/wp-login.php"), Some("wordpress"));
    assert_eq!(forsoeg_kategori("/_profiler/phpinfo"), Some("php"));
    assert_eq!(forsoeg_kategori("/actuator/env"), Some("adgang"));
    assert_eq!(forsoeg_kategori("/favicon.ico"), None);
    assert_eq!(forsoeg_kategori("/blog/"), Some("andet"));
}

#[test]
fn w_doegnraekker_og_oversigt_vaernet() {
    // 'døgnrækker og oversigter: Værnet'
    let rows = som_rows("2026-10-29", &doegn_raekker(&w_doegn()));
    assert!(
        !rows.iter().any(|r| r.noegle.contains("favicon")),
        "ufarlige 404 tælles ikke som forsøg"
    );
    let v = bygg_vaern(&rows, NU, vec![]);
    assert_eq!(v.perioder, Perioder { d1: 3, d7: 3, d30: 3 });
    assert_eq!(v.dage.len(), 30);
    assert_eq!(v.dage[29].n, 3);
    assert_eq!(v.dage[29].alle, 32749);
    assert_eq!(v.regler[0].cve.as_deref(), Some("CVE-2025-55182"));
    assert_eq!(v.regler[0].n7, 2);
    assert_eq!(v.regler[0].stier[0].sti, "/");
    assert_eq!(
        v.buer.iter().find(|b| b.land == "US"),
        Some(&Bue {
            land: "US".into(),
            colo: "EWR".into(),
            n: 2
        })
    );
    assert_eq!(v.netvaerk[0].navn, "Google LLC");
    let mut ids: Vec<&str> = v.forsoeg.kategorier.iter().map(|k| k.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, ["hemmeligheder", "kildekode"]);
    let ip = regex::Regex::new(r"\d+\.\d+\.\d+\.\d+").unwrap();
    assert!(!ip.is_match(&serde_json::to_string(&v).unwrap()), "ingen IP-adresser");
}

#[test]
fn w_forespoergsel() {
    // 'forespørgsel og aktivering' — W tjekker datoerne i forespørgselsteksten; her er de variabler.
    let vars = doegn_variabler("abc", "2026-10-28").unwrap();
    assert_eq!(vars["since"], "2026-10-28T00:00:00Z");
    assert_eq!(vars["until"], "2026-10-29T00:00:00Z");
    assert!(!DOEGN_QUERY.to_lowercase().contains("clientip"), "beder aldrig om IP-adresser");
    assert_eq!(dag_noegle(NU), "2026-10-29");
}

#[test]
fn w_offentlig_vaern() {
    // 'værnet: angriberens stier vises kun i en sikker form, ellers som kategori; tider rundes ned til hel time'
    assert_eq!(offentlig_sti("/wp-admin/install.php"), "/wp-admin/install.php");
    assert_eq!(offentlig_sti("/.env"), "/.env");
    assert_eq!(
        offentlig_sti("/wp-admin/RING-WI24RD-SUPPORT-+45-12 34 56 78"),
        "[wordpress]",
        "tekst med mellemrum og + bliver til kategorien"
    );
    assert_eq!(offentlig_sti(&format!("/{}", "a".repeat(61))), "[andet]", "for lang");
    assert_eq!(offentlig_sti("/ø-ring-mig"), "[andet]");
    assert_eq!(offentlig_sti(&offentlig_sti("/x y")), "[andet]", "idempotent");
    assert_eq!(til_timen("2026-11-02T05:17:34Z").as_deref(), Some("2026-11-02T05:00:00Z"));
    assert_eq!(til_timen("ikke en tid"), None);
    let sti = |s: &str, n| StiN { sti: s.into(), n };
    let v = offentlig_vaern(Vaern {
        perioder: Perioder {
            d1: 1,
            ..Default::default()
        },
        stier: vec![sti("/a b", 2), sti("/c d", 3), sti("/ok", 1)],
        regler: vec![Regel {
            navn: "R".into(),
            stier: vec![sti("/wp-login.php?x", 1)],
            ..Default::default()
        }],
        forsoeg: Forsoeg {
            i7: 1,
            kategorier: vec![Kategori {
                id: "hemmeligheder".into(),
                n: 1,
                stier: vec![sti("/.env bak", 1)],
            }],
        },
        seneste: vec![Seneste {
            t: Some("2026-11-02T05:17:34Z".into()),
            sti: "/RING +45".into(),
            regel: "R".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    assert_eq!(v.stier, vec![sti("[andet]", 5), sti("/ok", 1)], "ens kategorier lægges sammen");
    assert_eq!(v.regler[0].stier[0].sti, "[wordpress]");
    assert_eq!(v.forsoeg.kategorier[0].stier[0].sti, "[hemmeligheder]");
    assert_eq!(v.seneste[0].t.as_deref(), Some("2026-11-02T05:00:00Z"));
    assert_eq!(v.seneste[0].sti, "[andet]");
    assert_eq!(v.seneste[0].regel, "R");
    assert_eq!(v.perioder.d1, 1);
}

// ════ B. Differentielt mod W's egen JavaScript (paritet.json) ═══════════════════════════════════════════════════

static ORAKEL: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| serde_json::from_str(include_str!("paritet.json")).unwrap());

fn par(navn: &str) -> &'static Vec<Value> {
    ORAKEL[navn].as_array().unwrap()
}

#[test]
fn paritet_rensning_og_kategorier() {
    for p in par("renSti") {
        assert_eq!(ren_sti(p[0].as_str().unwrap()), p[1].as_str().unwrap(), "renSti({})", p[0]);
    }
    for p in par("offentligSti") {
        assert_eq!(
            offentlig_sti(p[0].as_str().unwrap()),
            p[1].as_str().unwrap(),
            "offentligSti({})",
            p[0]
        );
    }
    for p in par("forsoegKategori") {
        assert_eq!(forsoeg_kategori(p[0].as_str().unwrap()), p[1].as_str(), "forsoegKategori({})", p[0]);
    }
    for p in par("tilTimen") {
        assert_eq!(til_timen(p[0].as_str().unwrap()).as_deref(), p[1].as_str(), "tilTimen({})", p[0]);
    }
    for p in par("dagNoegle") {
        assert_eq!(dag_noegle(p[0].as_f64().unwrap() as i64), p[1].as_str().unwrap());
    }
}

#[test]
fn paritet_bots_og_regler() {
    for p in par("botNavn") {
        let b = bot_navn(p[0].as_str().unwrap(), p[1].as_str().unwrap());
        assert_eq!(
            (b.navn.as_str(), b.firma.as_str()),
            (p[2]["navn"].as_str().unwrap(), p[2]["firma"].as_str().unwrap()),
            "botNavn({}, {})",
            p[0],
            p[1]
        );
    }
    for p in par("efterligning") {
        assert_eq!(efterligning(p[0].as_str().unwrap()), p[1].as_bool().unwrap());
    }
    for p in par("regel") {
        let r = regel(p[0].as_str().unwrap(), p[1].as_str().unwrap());
        let ud = json!({ "navn": r.navn, "produkt": r.produkt, "type": r.type_, "cve": r.cve });
        assert_eq!(ud, p[2], "regel({}, {})", p[0], p[1]);
    }
}

#[test]
fn paritet_forespoergslen_har_samme_felter_som_w() {
    let ingen_mellemrum = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let w = ingen_mellemrum(ORAKEL["forespoergsel"].as_str().unwrap())
        .replace("\"abc\"", "$zoneTag")
        .replace("\"2026-10-28T00:00:00Z\"", "$since")
        .replace("\"2026-10-29T00:00:00Z\"", "$until");
    // V-SAMPLE.1: W's forespørgsel + feltet `sampleInterval` på `fw` (HAM tæller Cloudflares estimat, W tæller rækker). V-SUBDOM.1: + `clientRequestHTTPHost`
    // på `fw` (kun til art `v_fw`; `doegn_raekker` bruger det ikke). Alt andet er ordret.
    let gammel = "clientASNDescriptionedgeColoNamedatetime}";
    assert_eq!(w.matches(gammel).count(), 1, "W har én fw-node");
    let w = w.replacen(
        gammel,
        "clientASNDescriptionedgeColoNamedatetimesampleIntervalclientRequestHTTPHost}",
        1,
    );
    let mig = ingen_mellemrum(DOEGN_QUERY);
    let mig = mig
        .strip_prefix("queryVaernet($zoneTag:string!,$since:Time!,$until:Time!)")
        .expect("hoved med tre variabler");
    assert_eq!(
        mig, w,
        "samme aliaser, grænser, filtre og felter plus sampleInterval; kun zone og tider er variabler"
    );
    assert_eq!(DOEGN_QUERY.matches("sampleInterval").count(), 1, "kun på fw");
    assert_eq!(DOEGN_QUERY.matches("clientRequestHTTPHost").count(), 1, "værten kun på fw");
}

#[test]
fn paritet_doegnraekker_byggvaern_og_offentligvaern() {
    let mut rows = Vec::new();
    for (dag, z) in ORAKEL["doegn"].as_object().unwrap() {
        let rk = doegn_raekker(&doegn(z.clone()));
        let forventet: Vec<(String, String, i64)> = serde_json::from_value(ORAKEL["raekker"][dag].clone()).unwrap();
        assert_eq!(rk, forventet, "doegnRaekker for {dag}");
        rows.extend(som_rows(dag, &rk));
    }
    assert!(rows.len() > 100, "oraklet dækker mange rækker ({})", rows.len());
    let v = bygg_vaern(&rows, NU, vec![]);
    assert_eq!(serde_json::to_value(&v).unwrap(), ORAKEL["byggVaern"], "byggVaern");
    assert_eq!(
        serde_json::to_value(offentlig_vaern(v)).unwrap(),
        ORAKEL["offentligVaern"],
        "offentligVaern"
    );
}

// ════ C. Programmets egne dele ══════════════════════════════════════════════════════════════════════════════════

#[test]
fn datoer_og_tider() {
    assert_eq!(iso(NU), "2026-10-29T05:00:00.000Z");
    assert_eq!(parse_iso("2026-10-29T05:00:00Z"), Some(NU));
    assert_eq!(parse_iso("2026-10-29T05:00:00.123Z"), Some(NU + 123));
    assert_eq!(dag_ms("2024-02-29"), parse_iso("2024-02-29T00:00:00Z"));
    assert_eq!(dag_ms("2026-02-29"), None);
    assert_eq!(dag_ms("2026-13-01"), None);
    assert_eq!(parse_iso("2026-10-29T24:00:00Z"), None);
    assert_eq!(parse_iso("2026-10-29 05:00:00Z"), None);
    assert_eq!(dag_noegle(dag_ms("2026-12-31").unwrap() + DOEGN_MS), "2027-01-01");
    assert_eq!(vindue_fra(NU), "2026-09-30");
}

#[test]
fn graphql_body_har_variabler_og_intet_interpoleret_zone_id() {
    let zone = "0123456789abcdef0123456789abcdef";
    let body = graphql_body(DOEGN_QUERY, doegn_variabler(zone, "2026-10-28").unwrap());
    let query = body["query"].as_str().unwrap();
    assert!(!query.contains(zone), "zone-id'et står ikke i forespørgselsteksten");
    assert!(!query.contains("2026-10-2"), "datoerne står ikke i forespørgselsteksten");
    assert!(query.contains("zones(filter:{zoneTag:$zoneTag})"));
    assert!(query.contains("datetime_geq:$since") && query.contains("datetime_lt:$until"));
    assert_eq!(
        body["variables"],
        json!({ "zoneTag": zone, "since": "2026-10-28T00:00:00Z", "until": "2026-10-29T00:00:00Z" })
    );
    assert!(doegn_variabler(zone, "28-10-2026").is_err());
}

#[test]
fn zone_id_tjekkes() {
    assert!(tjek_zone("0123456789abcdef0123456789abcdef").is_ok());
    for daarlig in ["", "abc\"}", "a b", "x".repeat(65).as_str(), "æøå"] {
        assert!(tjek_zone(daarlig).is_err(), "{daarlig}");
    }
}

#[test]
fn svar_uden_zone_giver_fejl() {
    assert_eq!(doegn_fra_svar(&json!({ "viewer": { "zones": [] } })).unwrap_err(), Fejl::ZoneUkendt);
    // serdes egen tekst ("invalid type: string …") kasseres; webvisningen får kun koden.
    let f = doegn_fra_svar(&json!({ "viewer": { "zones": [{ "fw": "x" }] } })).unwrap_err();
    assert_eq!(f, Fejl::Svar);
    assert_eq!(serde_json::to_string(&f).unwrap(), r#"{"kode":"svar"}"#);
    let z = doegn_fra_svar(&json!({ "viewer": { "zones": [{ "fw": null, "alle": [{ "count": 5, "dimensions": null }] }] } })).unwrap();
    assert!(z.fw.is_empty());
    assert_eq!(z.alle[0].count, 5);
}

#[test]
fn seneste_gemmes_i_offentlig_form_og_vises_nyeste_foerst() {
    let mut z = w_doegn();
    z.fw[2].sti = "/wp-admin/RING MIG +45".into();
    let rk = seneste_raekker(&z);
    assert_eq!(rk.len(), 2, "de to ens React-hændelser (samme time) lægges sammen");
    assert_eq!(
        rk[0],
        (
            "seneste".into(),
            "2026-10-29T03:00:00Z\tReact - RCE - CVE:CVE-2025-55182\tCVE-2025-55182\tUS\t/\tEWR\tPOST".into(),
            2
        )
    );
    assert!(rk[1].1.contains("\t[wordpress]\t"), "angriberens tekst bliver til en kategori");
    assert!(
        !rk.iter().any(|r| r.1.contains(":17:")),
        "intet minut- eller sekundtidsstempel gemmes"
    );
    let mut rows = som_rows(
        "2026-10-28",
        &[("seneste".into(), "2026-10-28T22:00:00Z\tX\t\tDK\t/a\tCPH\tGET".into(), 1)],
    );
    rows.extend(som_rows("2026-10-29", &rk));
    let s = seneste_fra_raekker(&rows);
    let tider: Vec<&str> = s.iter().map(|x| x.t.as_deref().unwrap()).collect();
    assert_eq!(
        tider,
        [
            "2026-10-29T03:00:00Z",
            "2026-10-29T03:00:00Z",
            "2026-10-29T01:00:00Z",
            "2026-10-28T22:00:00Z"
        ]
    );
    assert_eq!(s[3].cve, None);
    let v = oversigt(&rows, NU, Some("2026-10-29T04:59:00.000Z".into()));
    assert_eq!(v.seneste.len(), 4);
    assert_eq!(v.genereret, "2026-10-29T04:59:00.000Z", "genereret = sidst hentet");
}

fn alle_hentet_faerdige(nu: i64) -> HashMap<String, String> {
    (0..=HENT_DAGE).map(|i| (dag_noegle(nu - i * DOEGN_MS), iso(nu))).collect()
}

#[test]
fn cache_aeldre_doegn_hentes_ikke_igen_i_dag_og_i_gaar_goer() {
    let foerste = dage_at_hente(NU, &HashMap::new());
    assert_eq!(foerste.len(), 31, "i dag + 30 hele døgn");
    assert_eq!(&foerste[..2], ["2026-10-29", "2026-10-28"]);
    assert_eq!(foerste.last().unwrap(), "2026-09-29");
    assert_eq!(dage_at_hente(NU, &alle_hentet_faerdige(NU)), ["2026-10-29", "2026-10-28"]);
    // Et ældre døgn, der kun blev hentet, mens det stadig var i gang, er ufuldstændigt og hentes igen.
    let mut h = alle_hentet_faerdige(NU);
    h.insert("2026-10-20".into(), "2026-10-20T18:00:00.000Z".into());
    h.insert("2026-10-19".into(), "2026-10-20T00:30:00.000Z".into());
    h.insert("2026-10-18".into(), "2026-10-19T01:00:00.000Z".into());
    assert_eq!(dage_at_hente(NU, &h), ["2026-10-29", "2026-10-28", "2026-10-20", "2026-10-19"]);
}

#[tokio::test]
async fn opdater_gemmer_hvert_doegn_og_henter_kun_det_noedvendige_naeste_gang() {
    let lager = Mutex::new(Store::i_hukommelse().unwrap());
    let kald = Mutex::new(Vec::<String>::new());
    let (nu_i_gang, flest) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let hent = |dag: String| {
        kald.lock().unwrap().push(dag);
        let (nu_i_gang, flest) = (nu_i_gang.clone(), flest.clone());
        async move {
            let n = nu_i_gang.fetch_add(1, Ordering::SeqCst) + 1;
            flest.fetch_max(n, Ordering::SeqCst);
            for _ in 0..3 {
                tokio::task::yield_now().await;
            }
            nu_i_gang.fetch_sub(1, Ordering::SeqCst);
            Ok(w_doegn())
        }
    };
    assert_eq!(opdater(&lager, "zone", NU, &hent, |_, _| {}).await.unwrap(), 31);
    assert_eq!(kald.lock().unwrap().iter().collect::<HashSet<_>>().len(), 31);
    assert_eq!(
        flest.load(Ordering::SeqCst),
        SAMTIDIGE,
        "højst fire samtidige kald (og fire bliver brugt)"
    );
    let rows = lager.lock().unwrap().raekker("zone", &vindue_fra(NU)).unwrap();
    let v = oversigt(&rows, NU, None);
    assert_eq!(
        v.perioder,
        Perioder { d1: 3, d7: 21, d30: 90 },
        "30 døgn i vinduet, hvert med tre blokeringer"
    );
    assert_eq!(v.seneste.len(), 12);

    kald.lock().unwrap().clear();
    let en_time_senere = NU + 3_600_000;
    assert_eq!(opdater(&lager, "zone", en_time_senere, &hent, |_, _| {}).await.unwrap(), 2);
    assert_eq!(*kald.lock().unwrap(), ["2026-10-29", "2026-10-28"], "kun i dag og i går");
}

#[tokio::test]
async fn et_fejlende_doegn_stopper_ikke_de_andre() {
    let lager = Mutex::new(Store::i_hukommelse().unwrap());
    let hent = |dag: String| async move {
        if dag == "2026-10-10" {
            Err(Fejl::GraphQl {
                dag: None,
                detalje: "for gammelt".into(),
            })
        } else {
            Ok(w_doegn())
        }
    };
    let fejl = opdater(&lager, "zone", NU, hent, |_, _| {}).await.unwrap_err();
    assert_eq!(
        fejl,
        Fejl::GraphQl {
            dag: None,
            detalje: "for gammelt".into()
        }
    );
    let hentet = lager.lock().unwrap().hentet("zone").unwrap();
    assert_eq!(hentet.len(), 30);
    assert!(!hentet.contains_key("2026-10-10"), "det fejlede døgn står stadig som manglende");
}

// ════ D. Samlet Værn, hele døgn ved siden af W's felter, døgn uden for opbevaringen ══════════════════

/// Rækker for en zone: W's testdøgn på `dag` med `alle` sat til `alle` og `fw` gentaget `gange` gange.
fn zone_rows(zone: &str, dag: &str, gange: usize, alle: i64) -> Vec<(String, Raekke)> {
    let mut z = w_doegn();
    let fw = z.fw.clone();
    for _ in 1..gange {
        z.fw.extend(fw.clone());
    }
    z.alle[0].count = alle;
    let mut rk = doegn_raekker(&z);
    rk.extend(seneste_raekker(&z));
    som_rows(dag, &rk).into_iter().map(|r| (zone.to_string(), r)).collect()
}

#[test]
fn samlet_vaern_for_tre_zoner_laegger_sammen() {
    let mut rows = zone_rows("z1", "2026-10-28", 1, 100);
    rows.extend(zone_rows("z2", "2026-10-28", 2, 1000));
    rows.extend(zone_rows("z3", "2026-10-29", 1, 10));
    rows.extend(zone_rows("z1", "2026-10-29", 1, 1));
    let navne: HashMap<String, String> = [("z1", "a.wi24rd.com"), ("z2", "b.wi24rd.com"), ("z3", "c.wi24rd.com")]
        .into_iter()
        .map(|(a, b)| (a.into(), b.into()))
        .collect();
    let v = samlet(&rows, &navne, NU, "2026-10-29T01:00:00.000Z".into());
    // 3 + 6 blokeringer den 28., 3 + 3 den 29.
    assert_eq!(v.vaern.perioder, Perioder { d1: 6, d7: 15, d30: 15 });
    let dag = |d: &str| v.vaern.dage.iter().find(|x| x.dag == d).unwrap().clone();
    assert_eq!(
        (dag("2026-10-28").n, dag("2026-10-28").alle),
        (9, 1100),
        "alle pr. døgn summeres (+=)"
    );
    assert_eq!((dag("2026-10-29").n, dag("2026-10-29").alle), (6, 11));
    assert_eq!(v.vaern.alle.d30, 1111, "og passer med totalen");
    assert_eq!(v.vaern.regler[0].n30, 10, "samme regel på flere domæner lægges sammen");
    assert_eq!(
        v.vaern.genereret, "2026-10-29T01:00:00.000Z",
        "ældste 'sidst hentet' blandt de valgte"
    );
    assert_eq!(v.vaern.seneste.len(), 12, "z1 3+3, z2 6, z3 3 = 15 flettet og afkortet til 12");
    let domaener: std::collections::BTreeSet<_> = v.vaern.seneste.iter().map(|s| s.domaene.clone().unwrap()).collect();
    assert_eq!(
        domaener.into_iter().collect::<Vec<_>>(),
        ["a.wi24rd.com", "b.wi24rd.com", "c.wi24rd.com"],
        "hver seneste har sit domæne"
    );
    let tider: Vec<_> = v.vaern.seneste.iter().map(|s| s.t.clone()).collect();
    assert!(tider.windows(2).all(|w| w[0] >= w[1]), "nyeste først på tværs af domæner");
    let j = serde_json::to_value(&v).unwrap();
    assert_eq!(j["seneste"][0]["domaene"].as_str().map(|d| d.ends_with(".wi24rd.com")), Some(true));
    assert_eq!(j["i_gaar"], 9, "hele døgn står ved siden af W's felter");
    assert_eq!(j["i_dag_hidtil"], 6);
    assert_eq!(j["perioder"]["d1"], 6, "W's felt er uændret (i dag hidtil)");
}

#[test]
fn samlet_for_en_zone_er_det_samme_som_zonens_eget_vaern() {
    let rows = zone_rows("z1", "2026-10-28", 1, 100);
    let navne = HashMap::from([("z1".to_string(), "a.wi24rd.com".to_string())]);
    let mut en = samlet(&rows, &navne, NU, "g".into());
    let egen = oversigt_ud(&rows.iter().map(|(_, r)| r.clone()).collect::<Vec<_>>(), NU, Some("g".into()));
    assert!(en.vaern.seneste.iter().all(|s| s.domaene.as_deref() == Some("a.wi24rd.com")));
    en.vaern.seneste.iter_mut().for_each(|s| s.domaene = None);
    assert_eq!(en, egen);
    assert!(
        !serde_json::to_string(&egen).unwrap().contains("domaene"),
        "én zone: intet domænefelt (W-formen)"
    );
}

#[test]
fn oversigt_ud_holder_w_vinduet_og_tilfoejer_hele_doegn() {
    // Et døgn på d-30 (2026-09-29) er med i hele30, men ikke i W's 30-døgnsvindue (d-29…d0).
    let mut rows = som_rows("2026-09-29", &doegn_raekker(&w_doegn()));
    rows.extend(som_rows("2026-10-28", &doegn_raekker(&w_doegn())));
    let v = oversigt_ud(&rows, NU, None);
    assert_eq!(v.vaern.perioder.d30, 3, "W-vinduet er uændret");
    assert_eq!(v.vaern, oversigt(&rows[rows.len() / 2..], NU, None));
    assert_eq!((v.hele.hele30, v.hele.i_gaar, v.hele.i_dag_hidtil), (6, 3, 0));
    assert_eq!(v.hele.fordeling.i_gaar.blokeret, 3);
}

#[tokio::test]
async fn doegn_uden_for_opbevaringen_springes_over_uden_fejl() {
    let lager = Mutex::new(Store::i_hukommelse().unwrap());
    let hent = |dag: String| async move {
        if dag.as_str() <= "2026-09-30" {
            Err(Fejl::GraphQl {
                dag: None,
                detalje: "cannot request data older than 2678400s".into(),
            })
        } else {
            Ok(w_doegn())
        }
    };
    let udfald = Mutex::new(Vec::new());
    let antal = opdater(&lager, "zone", NU, hent, |dag, u| {
        udfald.lock().unwrap().push((dag.to_string(), matches!(u, Doegnudfald::UdenFor(_))))
    })
    .await
    .unwrap();
    assert_eq!(antal, 29);
    let u = udfald.into_inner().unwrap();
    assert_eq!(u.len(), 31, "tilbagekald for hvert døgn");
    let uden: Vec<&str> = u
        .iter()
        .filter(|x| x.1)
        .map(|x| x.0.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(uden.len(), 2, "d-29 og d-30 sprunget over: {uden:?}");
    assert!(!lager.lock().unwrap().hentet("zone").unwrap().contains_key("2026-09-30"));
}

/// Testene kører på `HM_NU` eller `TEST_NU`, aldrig på dagens dato.
#[test]
fn testenes_ur_er_fast() {
    let vaerdi = std::env::var("HM_NU").unwrap_or_else(|_| TEST_NU.to_string());
    assert_eq!(ur_ms(), parse_iso(&vaerdi).unwrap());
    assert_eq!(ur_ms(), ur_ms(), "uret går ikke i testene");
}

// ════ D. V-SAMPLE.1: Cloudflares estimat (`sampleInterval`) i stedet for rækker ══════════════════════════════════════

/// Fordelingen målt på domæne-C 5. nov. (INVESTIGATION-V-TOKEN.1 §A1): `sampleInterval` → antal rækker.
const FORDELING_C: [(i64, usize); 7] = [(1, 1094), (2, 612), (4, 91), (6, 12), (3, 16), (10, 2), (20, 2)];

/// Et døgns svar som JSON, med `sampleInterval` fra fordelingen. Tre regler og to lande, så nøglerne er flere end én.
fn vaegtet_doegn(fordeling: &[(i64, usize)]) -> Doegn {
    let mut fw = Vec::new();
    for &(vaegt, antal) in fordeling {
        for i in 0..antal {
            fw.push(json!({ "action": "block", "source": "firewallManaged", "description": format!("Regel {}", i % 3), "clientCountryName": if i % 2 == 0 { "US" } else { "NL" }, "clientRequestPath": "/", "clientRequestHTTPMethodName": "GET", "clientASNDescription": "AS", "edgeColoName": "CPH", "datetime": "2026-11-05T12:00:00Z", "sampleInterval": vaegt }));
        }
    }
    doegn(json!({ "fw": fw }))
}

fn fw_sum(rk: &[(String, String, i64)]) -> i64 {
    rk.iter().filter(|r| r.0 == "fw").map(|r| r.2).sum()
}

#[test]
fn fordelingen_giver_2862_af_1829_raekker() {
    let z = vaegtet_doegn(&FORDELING_C);
    assert_eq!(z.fw.len(), 1829, "rækker");
    let rk = doegn_raekker(&z);
    assert_eq!(fw_sum(&rk), 2862, "Cloudflares estimat (summen af sampleInterval)");
    assert_eq!(
        haendelser_raekke(&z),
        (FW_HAENDELSER.to_string(), String::new(), 1829),
        "antal hændelser gemmes for sig"
    );
    let v = bygg_vaern(&som_rows("2026-10-29", &rk), NU, vec![]);
    assert_eq!(v.perioder.d1, 2862, "Overblik/Angreb bygger på art fw og får estimatet");
}

#[test]
fn manglende_0_null_og_maerkelige_vaegte_taeller_1() {
    let haendelse = |si: Option<Value>| {
        let mut e = json!({ "action": "block", "source": "firewallManaged", "description": "R", "clientCountryName": "US", "clientRequestPath": "/", "datetime": "2026-11-05T12:00:00Z" });
        if let Some(v) = si {
            e["sampleInterval"] = v;
        }
        e
    };
    let tal = |si: Option<Value>| fw_sum(&doegn_raekker(&doegn(json!({ "fw": [haendelse(si)] }))));
    assert_eq!(tal(None), 1, "uden feltet (W's forespørgsel)");
    assert_eq!(tal(Some(json!(0))), 1, "0");
    assert_eq!(tal(Some(Value::Null)), 1, "null");
    assert_eq!(tal(Some(json!(-3))), 1, "negativ");
    assert_eq!(tal(Some(json!("x"))), 1, "tekst vælter ikke døgnet");
    assert_eq!(tal(Some(json!(2.0))), 2, "decimaltal rundes");
    assert_eq!(tal(Some(json!(7))), 7);
}

#[test]
fn seneste_og_angreb_taeller_haendelser_ikke_vaegt() {
    let z = vaegtet_doegn(&[(10, 300)]);
    assert_eq!(fw_sum(&doegn_raekker(&z)), 3000);
    let sum = |rk: Vec<(String, String, i64)>| rk.iter().map(|r| r.2).sum::<i64>();
    assert_eq!(sum(seneste_raekker(&z)), 12, "de 12 seneste hændelser, ikke 120");
    assert_eq!(sum(angreb_raekker(&z)), ANGREB_MAKS as i64, "de 200 seneste angreb, ikke 2000");
}

/// W-paritet er kun opgivet for selve tællingen: med `sampleInterval` = 3 på hver hændelse er nøgler, rækkefølge og alle andre
/// arter som `paritet.json`, og kun `fw`-tallene er 3 gange W's.
#[test]
fn paritet_kun_taellingen_afviger() {
    for (dag, z) in ORAKEL["doegn"].as_object().unwrap() {
        let mut z = z.clone();
        for e in z["fw"].as_array_mut().into_iter().flatten() {
            e["sampleInterval"] = json!(3);
        }
        let rk = doegn_raekker(&doegn(z));
        let w: Vec<(String, String, i64)> = serde_json::from_value(ORAKEL["raekker"][dag].clone()).unwrap();
        assert_eq!(rk.len(), w.len(), "{dag}");
        for (mig, w) in rk.iter().zip(&w) {
            assert_eq!((&mig.0, &mig.1), (&w.0, &w.1), "samme rensning, kategorier og robotter ({dag})");
            assert_eq!(mig.2, if w.0 == "fw" { 3 * w.2 } else { w.2 }, "{dag}: {}", w.1);
        }
    }
}

#[test]
fn afkortet_svar_erstatter_aldrig_et_fuldt_doegn() {
    let gemt = vec![("a".to_string(), 7000), ("b".to_string(), 5000)];
    let nyt = |n: i64| vec![("fw".to_string(), "a".to_string(), n), ("alle".to_string(), String::new(), 1)];
    let mut rk = nyt(10_000);
    assert!(behold_fuldt_fw(&mut rk, &gemt, 10_000), "afkortet og lavere: det gemte bliver");
    assert_eq!(fw_sum(&rk), 12_000);
    assert!(rk.contains(&("alle".to_string(), String::new(), 1)), "kun fw røres");
    let mut rk = nyt(500);
    assert!(
        !behold_fuldt_fw(&mut rk, &gemt, 500),
        "ikke afkortet: det nye svar er rigtigt, også når det er lavere"
    );
    assert_eq!(fw_sum(&rk), 500);
    let mut rk = nyt(13_000);
    assert!(!behold_fuldt_fw(&mut rk, &gemt, 10_000), "afkortet, men højere: det nye bliver");
    assert_eq!(fw_sum(&rk), 13_000);
}

#[tokio::test]
async fn opdater_beholder_det_fulde_doegn_ved_afkortet_svar() {
    let lager = Mutex::new(Store::i_hukommelse().unwrap());
    let dag = dag_noegle(NU - DOEGN_MS); // i går: hentes igen ved hver runde
    lager
        .lock()
        .unwrap()
        .gem_doegn(
            "zone",
            &dag,
            &[("fw".into(), "gammel".into(), 12_000), ("fwn".into(), String::new(), 9_000)],
            &format!("{dag}T12:00:00.000Z"), // hentet, mens døgnet var i gang: hentes igen
        )
        .unwrap();
    // Kun testdøgnet får det store svar; de andre døgn i vinduet er tomme, så testen er hurtig.
    let svar = |n: usize, vaegt: i64| {
        let (z, maal) = (vaegtet_doegn(&[(vaegt, n)]), dag.clone());
        move |d: String| {
            let z = if d == maal { z.clone() } else { Doegn::default() };
            async move { Ok(z) }
        }
    };
    let gemt = |l: &Mutex<Store>| {
        let s = l.lock().unwrap();
        (s.doegn_art("zone", &dag, "fw").unwrap(), s.doegn_art("zone", &dag, "fwn").unwrap())
    };
    opdater(&lager, "zone", NU, &svar(10_000, 1), |_, _| {}).await.unwrap();
    let (fw, fwn) = gemt(&lager);
    assert_eq!(
        fw,
        [("gammel".to_string(), 12_000)],
        "10.000 afkortede hændelser erstatter ikke 12.000"
    );
    assert_eq!(fwn, [(String::new(), 10_000)], "døgnet er markeret som afkortet");
    opdater(&lager, "zone", NU, &svar(10_000, 2), |_, _| {}).await.unwrap();
    assert_eq!(
        gemt(&lager).0.iter().map(|x| x.1).sum::<i64>(),
        20_000,
        "afkortet, men højere: det nye bliver"
    );
    opdater(&lager, "zone", NU, &svar(100, 1), |_, _| {}).await.unwrap();
    assert_eq!(
        gemt(&lager).0.iter().map(|x| x.1).sum::<i64>(),
        100,
        "et fuldt svar erstatter altid"
    );
}

#[test]
fn genhentning_af_30_doegn_sker_en_gang() {
    let mut s = Store::i_hukommelse().unwrap();
    let dag = |i: i64| dag_noegle(NU - i * DOEGN_MS);
    for i in 0..=40 {
        s.gem_doegn("z1", &dag(i), &[("fw".into(), "k".into(), 1)], &iso(NU - 3_600_000))
            .unwrap();
    }
    s.gem_ekstra("z1", &dag(5), &[("stoppet".into(), "k".into(), 1)], &iso(NU)).unwrap();
    let foer = s.hentet("z1").unwrap();
    assert!(foer.iter().all(|(d, h)| er_faerdig(d, h) || *d >= dag(1)));

    assert!(fw_vaegt_en_gang(&mut s, NU).unwrap(), "første start efter opdateringen");
    let efter = s.hentet("z1").unwrap();
    assert_eq!(efter.len(), foer.len(), "ingen døgn forsvinder (ingen huller)");
    assert_eq!(
        s.indstilling(crate::store::FW_VAEGT).unwrap(),
        Some(dag(HENT_DAGE)),
        "første dag med estimater"
    );
    let igen = dage_at_hente(NU, &efter);
    assert_eq!(igen.len(), 31, "i dag, i går og de 29 færdige døgn i vinduet");
    assert_eq!((igen.first(), igen.last()), (Some(&dag(0)), Some(&dag(HENT_DAGE))));
    for i in [0, 1, 31, 40] {
        assert_eq!(efter[&dag(i)], foer[&dag(i)], "dag {i} er urørt");
    }
    assert_eq!(s.hentet_ekstra("z1").unwrap()[&dag(5)], iso(NU), "de ekstra tal røres ikke");
    assert_eq!(
        s.doegn_art("z1", &dag(5), "fw").unwrap(),
        [("k".to_string(), 1)],
        "ingen tal slettes"
    );

    // Køen henter døgnet igen; næste start rører det ikke.
    s.gem_doegn("z1", &dag(5), &[("fw".into(), "k".into(), 2)], &iso(NU)).unwrap();
    assert!(!fw_vaegt_en_gang(&mut s, NU + 3 * DOEGN_MS).unwrap(), "kun én gang");
    assert_eq!(s.hentet("z1").unwrap()[&dag(5)], iso(NU));
    assert_eq!(
        s.indstilling(crate::store::FW_VAEGT).unwrap(),
        Some(dag(HENT_DAGE)),
        "grænsen flytter sig ikke"
    );
}
