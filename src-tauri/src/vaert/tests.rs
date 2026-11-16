//! V-SUBDOM.1 B1. Kun opdigtede fixtures under `wi24rd.com`; ingen kald til Cloudflare.
use std::cell::RefCell;
use std::sync::Mutex;

use serde_json::json;

use super::*;
use crate::cf::Zone;
use crate::vaern::{angreb_raekker, doegn_raekker, haendelser_raekke, opdater, seneste_raekker, Doegn, HENT_DAGE, TEST_NU};

const Z: &str = "wi24rd.com";

/// Testenes ur (`vaern::TEST_NU`).
fn nu() -> i64 {
    parse_iso(TEST_NU).unwrap()
}

fn doegn(v: Value) -> Doegn {
    serde_json::from_value(v).unwrap()
}

fn sum(rk: &[(String, String, i64)], art: &str) -> i64 {
    rk.iter().filter(|r| r.0 == art).map(|r| r.2).sum()
}

fn sum4(rk: &[Raekke4], art: &str) -> i64 {
    rk.iter().filter(|r| r.1 == art).map(|r| r.3).sum()
}

/// En hændelse som Cloudflare sender den, med eller uden værten.
fn haendelse(i: usize, vaert: Option<&str>, vaegt: i64) -> Value {
    let (handling, kilde) = (
        if i.is_multiple_of(3) { "managed_challenge" } else { "block" },
        ["firewallManaged", "bic", "waf", "linkMaze"][i % 4],
    );
    let mut e = json!({ "action": handling, "source": kilde,
        "description": format!("Regel {}", i % 5), "clientCountryName": "US", "clientRequestPath": format!("/s{}", i % 7), "clientRequestHTTPMethodName": "GET",
        "clientASNDescription": "AS", "edgeColoName": "CPH", "datetime": format!("2026-11-05T{:02}:00:00Z", 23 - i % 24), "sampleInterval": vaegt });
    if let Some(v) = vaert {
        e["clientRequestHTTPHost"] = json!(v);
    }
    e
}

/// Værter med port, punktum, store bogstaver, IP og 25 subdomæner (wildcard): flere end `TOP_VAERTER`.
fn vaerter() -> Vec<String> {
    let mut v: Vec<String> = [
        "wi24rd.com",
        "wi24rd.com:443",
        "WI24RD.com.",
        "www.wi24rd.com",
        "shop.wi24rd.com",
        "shop.wi24rd.com:8443",
        "192.0.2.7",
        "[2001:db8::1]:443",
        "",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    v.extend((1..=25).map(|i| format!("sub{i}.wi24rd.com")));
    v
}

// ── ren_vaert ──────────────────────────────────────────────────────────────────────────────────────────────────────
#[test]
fn ren_vaert_renser_som_aftalt() {
    let lang = |n: usize| format!("{}.{Z}", "a".repeat(n));
    let for_langt = format!("{}.{Z}", ["b".repeat(60), "c".repeat(60), "d".repeat(60), "e".repeat(60)].join("."));
    let tilfaelde: Vec<(String, &str)> = vec![
        // apex, www og to niveauer
        (Z.into(), ""),
        ("www.wi24rd.com".into(), "www"),
        ("shop.wi24rd.com".into(), "shop"),
        ("a.b.wi24rd.com".into(), "a.b"),
        ("_dmarc.wi24rd.com".into(), "_dmarc"),
        // port, afsluttende punktum, store bogstaver
        ("wi24rd.com:443".into(), ""),
        ("shop.wi24rd.com:8443".into(), "shop"),
        ("wi24rd.com.".into(), ""),
        ("www.wi24rd.com.:80".into(), "www"),
        ("WWW.Wi24rd.COM".into(), "www"),
        ("wi24rd.com:abc".into(), UGYLDIG),
        ("wi24rd.com:123456".into(), UGYLDIG),
        // IP som vært: adressen gemmes aldrig
        ("192.0.2.7".into(), IP),
        ("192.0.2.7:8080".into(), IP),
        ("[2001:db8::1]".into(), IP),
        ("[2001:db8::1]:443".into(), IP),
        ("2001:db8::1".into(), IP),
        ("[2001:db8::zz]".into(), UGYLDIG),
        ("[2001:db8::1".into(), UGYLDIG),
        // punycode bliver punycode; rå unicode er ugyldig
        ("xn--bcher-kva.wi24rd.com".into(), "xn--bcher-kva"),
        ("bücher.wi24rd.com".into(), UGYLDIG),
        // ulovlige tegn, tomme labels, længder
        ("".into(), UGYLDIG),
        (":443".into(), UGYLDIG),
        ("a..wi24rd.com".into(), UGYLDIG),
        (".wi24rd.com".into(), UGYLDIG),
        ("<b>.wi24rd.com".into(), UGYLDIG),
        ("a b.wi24rd.com".into(), UGYLDIG),
        ("a\tb.wi24rd.com".into(), UGYLDIG),
        (lang(63), "x"),
        (lang(64), UGYLDIG),
        (for_langt.clone(), UGYLDIG),
    ];
    for (ind, ud) in &tilfaelde {
        let ud = if *ud == "x" { "a".repeat(63) } else { ud.to_string() };
        assert_eq!(ren_vaert(ind, Z), ud, "{ind:?}");
    }
    assert!(
        for_langt.len() > 253 && for_langt.split('.').all(|l| l.len() <= 63),
        "kun den samlede længde er for lang"
    );
    // Uden for zonen (zonen er her et subdomæne, så alle navne i testen er under wi24rd.com).
    for ind in ["wi24rd.com", "blog.wi24rd.com", "xshop.wi24rd.com", "wi24rd"] {
        assert_eq!(ren_vaert(ind, "shop.wi24rd.com"), ANDEN, "{ind}");
    }
    assert_eq!(ren_vaert("api.shop.wi24rd.com", "shop.wi24rd.com"), "api");
    assert_eq!(ren_vaert("www.wi24rd.com", "WI24RD.com."), "www", "zonens navn renses også");
    assert_eq!(ren_vaert("www.wi24rd.com", ""), ANDEN, "uden zone er alt uden for");
    // Intet resultat kan ligne en rå adresse eller indeholde andet end det tilladte.
    for (ind, _) in &tilfaelde {
        let ud = ren_vaert(ind, Z);
        assert!(
            ud.starts_with('[')
                || ud
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b)),
            "{ud:?}"
        );
        assert!(!ud.contains("192") && !ud.contains("2001"), "{ud:?}");
    }
}

// ── Trafik pr. vært ────────────────────────────────────────────────────────────────────────────────────────────────
/// Rå tal pr. (vært, land): forespørgsler fra alle kilder, forespørgsler og besøg udefra. Cloudflare lægger dem sammen pr. gruppe.
fn raa_trafik() -> Vec<(String, &'static str, i64, i64, i64)> {
    vaerter()
        .into_iter()
        .enumerate()
        .flat_map(|(i, v)| {
            let i = i as i64;
            [(v.clone(), "DK", 100 + i * 7, 60 + i * 3, 10 + i), (v, "US", 40 + i, 20 + i, i % 4)]
        })
        .collect()
}

fn vaert_svar(dag: &str) -> Value {
    let mut va: BTreeMap<String, i64> = BTreeMap::new();
    let mut v: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for (h, _, alle, eyeball, besoeg) in raa_trafik() {
        *va.entry(h.clone()).or_default() += alle;
        let x = v.entry(h).or_default();
        x.0 += eyeball;
        x.1 += besoeg;
    }
    json!({ "viewer": { "zones": [{
        "va": va.into_iter().map(|(h, n)| json!({ "count": n, "dimensions": { "date": dag, "clientRequestHTTPHost": h } })).collect::<Vec<_>>(),
        "v": v.into_iter().map(|(h, (n, b))| json!({ "count": n, "sum": { "visits": b }, "dimensions": { "date": dag, "clientRequestHTTPHost": h } })).collect::<Vec<_>>(),
    }] } })
}

/// Summen over værterne er zonens `alle` (hovedhentningen) og `b_total` (besøgene) for døgnet, også når værter lægges sammen (port, punktum, store bogstaver)
/// og de mindste bliver til `[andre]`.
#[test]
fn summen_over_vaerter_er_zonens_alle_og_b_total() {
    let dag = "2026-11-05".to_string();
    let (rk, afskaaret) = trafik_raekker(&vaert_svar(&dag), std::slice::from_ref(&dag), Z).unwrap();
    assert!(!afskaaret);
    let raa = raa_trafik();
    let alle: i64 = raa.iter().map(|r| r.2).sum();
    let zone_alle = doegn(json!({ "alle": [{ "count": alle, "dimensions": { "date": dag } }] }));
    assert_eq!(sum4(&rk, V_ALLE), sum(&doegn_raekker(&zone_alle), "alle"), "v_alle = alle");
    let mut lande: BTreeMap<&str, i64> = BTreeMap::new();
    for r in &raa {
        *lande.entry(r.1).or_default() += r.4;
    }
    let besoeg = json!({ "viewer": { "zones": [{ "besoeg": lande.iter().map(|(l, n)| json!({ "sum": { "visits": n }, "dimensions": { "date": dag, "clientCountryName": l } })).collect::<Vec<_>>() }] } });
    let b_total: i64 = crate::besoeg::raekker(&besoeg, std::slice::from_ref(&dag))
        .unwrap()
        .iter()
        .filter(|r| r.1 == "b_total")
        .map(|r| r.3)
        .sum();
    assert_eq!(sum4(&rk, V_BESOEG), b_total, "v_besoeg = b_total");
    // Rensningen lægger sammen: apex med port og punktum er én række; IP og tom vært er kategorier.
    let noegler: HashSet<&str> = rk.iter().filter(|r| r.1 == V_ALLE).map(|r| r.2.as_str()).collect();
    for k in ["", "shop", IP, ANDRE] {
        assert!(noegler.contains(k), "{k:?} i {noegler:?}");
    }
    assert!(
        !noegler.iter().any(|k| k.contains(':') || k.contains("wi24rd")),
        "aldrig rå værter: {noegler:?}"
    );
}

/// Top 20 pr. domæne pr. døgn efter forespørgsler; resten er én række `[andre]`, og intet tal forsvinder.
#[test]
fn top_20_og_andre() {
    let dag = "2026-11-05".to_string();
    let (rk, _) = trafik_raekker(&vaert_svar(&dag), std::slice::from_ref(&dag), Z).unwrap();
    let alle: Vec<&Raekke4> = rk.iter().filter(|r| r.1 == V_ALLE).collect();
    assert_eq!(alle.len(), TOP_VAERTER + 1, "20 værter + [andre]");
    let andre = alle.iter().find(|r| r.2 == ANDRE).unwrap().3;
    let mindst_beholdt = alle.iter().filter(|r| r.2 != ANDRE).map(|r| r.3).min().unwrap();
    assert!(andre > 0 && mindst_beholdt > 0);
    // De mindste rå værter (sub1, sub2, …: i = 9, 10, …) er i [andre], de største beholdt.
    let navne: HashSet<&str> = alle.iter().map(|r| r.2.as_str()).collect();
    assert!(navne.contains("sub25") && !navne.contains("sub1"), "{navne:?}");
    assert!(rk.iter().filter(|r| r.1 == V_BESOEG).count() <= TOP_VAERTER + 1);
    // Et døgn uden for `dage` tæller ikke; to døgn holdes hver for sig.
    let (intet, _) = trafik_raekker(&vaert_svar(&dag), &["2026-11-04".to_string()], Z).unwrap();
    assert!(intet.is_empty());
}

#[test]
fn trafiksvar_paa_loftet_er_afskaaret() {
    let dag = "2026-11-05";
    let mange: Vec<Value> = (0..LOFT)
        .map(|i| json!({ "count": 1, "dimensions": { "date": dag, "clientRequestHTTPHost": format!("x{i}.wi24rd.com") } }))
        .collect();
    let svar = json!({ "viewer": { "zones": [{ "va": mange, "v": [] }] } });
    let (rk, afskaaret) = trafik_raekker(&svar, &[dag.to_string()], Z).unwrap();
    assert!(afskaaret);
    assert_eq!(
        rk.iter().filter(|r| r.1 == V_ALLE).count(),
        TOP_VAERTER + 1,
        "og aldrig mere end 21 rækker"
    );
}

// ── Angreb pr. vært ────────────────────────────────────────────────────────────────────────────────────────────────
fn fw_doegn(med_vaert: bool) -> Doegn {
    let v = vaerter();
    let fw: Vec<Value> = (0..300)
        .map(|i| haendelse(i, med_vaert.then(|| v[i % v.len()].as_str()), 1 + (i % 3) as i64))
        .collect();
    doegn(json!({ "fw": fw, "alle": [{ "count": 5000, "dimensions": { "date": "2026-11-05" } }] }))
}

/// `v_fw` summer til `fw` (Cloudflares estimat), også med `[andre]`, IP og tom vært.
#[test]
fn v_fw_summer_til_fw() {
    let z = fw_doegn(true);
    let v = fw_raekker(&z.fw, Z).expect("værten er i svaret");
    assert_eq!(sum(&v, V_FW), sum(&doegn_raekker(&z), "fw"));
    let vaerter: HashSet<&str> = v.iter().map(|r| r.1.split('\t').next().unwrap()).collect();
    assert_eq!(vaerter.len(), TOP_VAERTER + 1, "20 værter + [andre]: {vaerter:?}");
    for r in &v {
        assert_eq!(r.1.split('\t').count(), 3, "vært, kilde, handling: {:?}", r.1);
    }
    assert_eq!(fw_raekker(&[], Z), Some(Vec::new()), "ingen hændelser: 0, og døgnet er hentet");
}

/// `fw`, `fwn`, `angreb` og `seneste` er bit for bit de samme med og uden værten i svaret, og uden værten er der intet `v_fw`.
#[test]
fn fw_fwn_angreb_seneste_uaendrede_med_og_uden_vaert() {
    let (med, uden) = (fw_doegn(true), fw_doegn(false));
    assert_eq!(doegn_raekker(&med), doegn_raekker(&uden));
    assert_eq!(angreb_raekker(&med), angreb_raekker(&uden));
    assert_eq!(seneste_raekker(&med), seneste_raekker(&uden));
    assert_eq!(haendelser_raekke(&med), haendelser_raekke(&uden));
    assert_eq!(fw_raekker(&uden.fw, Z), None, "W's forespørgsel: intet tal pr. vært");
    assert!(doegn_raekker(&med).iter().all(|r| !r.0.starts_with("v_")), "doegn_raekker er W's");
}

/// På loftet (S4): under loftet de nye; på loftet de gemte, hvis de gemte `fw`-rækker blev beholdt og har `v_fw`; ellers kun mærket.
#[test]
fn fw_paa_loftet_giver_aldrig_et_for_lavt_tal() {
    let nye = vec![(V_FW.to_string(), "\tfirewallManaged\tblock".to_string(), 7)];
    let gemt = vec![("shop\twaf\tblock".to_string(), 12_000)];
    assert_eq!(fw_med_loft(nye.clone(), false, false, &gemt, 9), nye);
    assert_eq!(
        fw_med_loft(nye.clone(), true, true, &gemt, 10_000),
        vec![(V_FW.to_string(), "shop\twaf\tblock".to_string(), 12_000)]
    );
    let maerke = vec![(V_FW_AFSKAARET.to_string(), String::new(), 10_000)];
    assert_eq!(
        fw_med_loft(nye.clone(), true, false, &gemt, 10_000),
        maerke,
        "det nye svar er afkortet"
    );
    assert_eq!(fw_med_loft(nye, true, true, &[], 10_000), maerke, "de gemte havde ingen værter");
}

fn fw_svar(n: usize, dag: &str) -> Value {
    let fw: Vec<Value> = (0..n)
        .map(|i| {
            let mut e = haendelse(i, Some(["shop.wi24rd.com", "wi24rd.com"][i % 2]), 1);
            e["datetime"] = json!(format!("{dag}T{:02}:00:00Z", i % 24));
            e
        })
        .collect();
    json!({ "viewer": { "zones": [{ "fw": fw }] } })
}

/// Genhentningen: et vindue på loftet deles i døgn, og et døgn, der stadig står på loftet, får intet `v_fw`, kun mærket.
#[tokio::test]
async fn genhentning_paa_loftet_giver_intet_v_fw() {
    let kald: RefCell<Vec<(String, String)>> = RefCell::default();
    let hent = |fra: String, til: String| {
        kald.borrow_mut().push((fra.clone(), til.clone()));
        let svar = if fra != til {
            fw_svar(LOFT as usize, "2026-11-03")
        } else {
            match fra.as_str() {
                "2026-11-03" => fw_svar(5, &fra),
                "2026-11-04" => fw_svar(LOFT as usize, &fra),
                _ => fw_svar(0, &fra),
            }
        };
        async move { Ok(svar) }
    };
    let (dage, rk) = hent_opdelt(
        "2026-11-03",
        "2026-11-05",
        hent,
        |d, dage| fw_dag_raekker(d, dage, Z),
        V_FW_AFSKAARET,
    )
    .await
    .unwrap();
    assert_eq!(dage, ["2026-11-03", "2026-11-04", "2026-11-05"]);
    assert_eq!(kald.borrow().len(), 4, "vinduet og så hvert døgn");
    let paa = |dag: &str, art: &str| rk.iter().filter(|r| r.0 == dag && r.1 == art).map(|r| r.3).sum::<i64>();
    assert_eq!(paa("2026-11-03", V_FW), 5);
    assert_eq!(
        (paa("2026-11-04", V_FW), paa("2026-11-04", V_FW_AFSKAARET)),
        (0, LOFT),
        "intet tal, kun mærket"
    );
    assert!(rk.iter().all(|r| r.1 == V_FW || r.1 == V_FW_AFSKAARET), "aldrig art fw");
    assert!(
        !rk.iter().any(|r| r.0 == "2026-11-05"),
        "et døgn uden hændelser: ingen rækker, men hentet"
    );
    // Under loftet: ét kald.
    kald.borrow_mut().clear();
    let hent = |fra: String, _til: String| {
        kald.borrow_mut().push((fra.clone(), fra.clone()));
        async move { Ok(fw_svar(30, &fra)) }
    };
    let (_, rk) = hent_opdelt(
        "2026-11-03",
        "2026-11-05",
        hent,
        |d, dage| fw_dag_raekker(d, dage, Z),
        V_FW_AFSKAARET,
    )
    .await
    .unwrap();
    assert_eq!(kald.borrow().len(), 1);
    assert_eq!(sum4(&rk, V_FW), 30);
    // En hændelse uden tidspunkt gør svaret ubrugeligt, hellere end at tælle for lavt.
    let mut d = fw_svar(2, "2026-11-03");
    d["viewer"]["zones"][0]["fw"][0]["datetime"] = json!("i går");
    assert_eq!(fw_dag_raekker(&d, &["2026-11-03".to_string()], Z), Err(Fejl::Svar));
}

// ── Hovedhentningen gemmer v_fw ────────────────────────────────────────────────────────────────────────────────────
fn lager_med_zone(navn: Option<&str>) -> Mutex<Store> {
    let mut s = Store::i_hukommelse().unwrap();
    if let Some(navn) = navn {
        s.gem_zoner(
            1,
            &[Zone {
                id: "z1".into(),
                navn: navn.into(),
            }],
            "t",
        )
        .unwrap();
    }
    Mutex::new(s)
}

/// `vaern::opdater` gemmer `v_fw` fra samme svar og noterer døgnet i `hentet_vaert_fw`; `raekker` (Overblik, Angreb, notifikationerne) ser dem ikke. Uden
/// zonens navn gemmes intet pr. vært, og resten er det samme.
#[tokio::test]
async fn hovedhentningen_gemmer_v_fw_og_overblikket_ser_dem_ikke() {
    let z = fw_doegn(true);
    let hent = |_: String| {
        let z = z.clone();
        async move { Ok(z) }
    };
    let (med, uden) = (lager_med_zone(Some(Z)), lager_med_zone(None));
    let antal = (HENT_DAGE + 1) as usize;
    assert_eq!(opdater(&med, "z1", nu(), &hent, |_, _| {}).await.unwrap(), antal);
    assert_eq!(opdater(&uden, "z1", nu(), &hent, |_, _| {}).await.unwrap(), antal);
    let (m, u) = (med.lock().unwrap(), uden.lock().unwrap());
    let dag = dag_noegle(nu() - DOEGN_MS);
    let v_fw: i64 = m.doegn_art("z1", &dag, V_FW).unwrap().iter().map(|r| r.1).sum();
    let fw: i64 = m.doegn_art("z1", &dag, "fw").unwrap().iter().map(|r| r.1).sum();
    assert_eq!(v_fw, fw);
    assert_eq!(m.hentet_vaert_fw("z1").unwrap().len(), antal);
    assert!(u.hentet_vaert_fw("z1").unwrap().is_empty() && u.doegn_art("z1", &dag, V_FW).unwrap().is_empty());
    assert_eq!(
        m.raekker("z1", "0000-00-00").unwrap(),
        u.raekker("z1", "0000-00-00").unwrap(),
        "Overblik og Angreb er uændrede"
    );
    assert_eq!(
        m.raekker_for(&["z1".to_string()], "0000-00-00").unwrap(),
        u.raekker_for(&["z1".to_string()], "0000-00-00").unwrap(),
        "samlet Værn er uændret"
    );
    assert!(m.raekker("z1", "0000-00-00").unwrap().iter().all(|r| !r.art.starts_with("v_")));
}

/// Kommer et svar uden værten for et døgn, der havde tal pr. vært, er døgnet ikke længere hentet pr. vært: intet tal, ikke 0.
#[tokio::test]
async fn et_svar_uden_vaert_glemmer_doegnet_pr_vaert() {
    let (med, uden) = (fw_doegn(true), fw_doegn(false));
    let lager = lager_med_zone(Some(Z));
    let hent = |_: String| {
        let z = med.clone();
        async move { Ok(z) }
    };
    opdater(&lager, "z1", nu(), &hent, |_, _| {}).await.unwrap();
    let dag = dag_noegle(nu() - DOEGN_MS);
    assert!(lager.lock().unwrap().hentet_vaert_fw("z1").unwrap().contains_key(&dag));
    let hent = |_: String| {
        let z = uden.clone();
        async move { Ok(z) }
    };
    opdater(&lager, "z1", nu(), &hent, |_, _| {}).await.unwrap();
    let s = lager.lock().unwrap();
    assert!(
        !s.hentet_vaert_fw("z1").unwrap().contains_key(&dag),
        "i dag og i går hentes igen: ikke længere pr. vært"
    );
    assert!(s.doegn_art("z1", &dag, V_FW).unwrap().is_empty());
    let l = laes(&s, "z1", &dag).unwrap();
    assert!(
        !l.hentet_fw.contains(&dag) && regn(&l, &dag, &dag).vaerter.is_empty(),
        "intet tal, ikke 0"
    );
}

/// På loftet i hovedhentningen: kun mærket, og døgnet er hentet (det spørges ikke om igen).
#[tokio::test]
async fn hovedhentningen_paa_loftet_giver_kun_maerket() {
    let dag = dag_noegle(nu() - 2 * DOEGN_MS);
    let fuld = Doegn::deserialize(&fw_svar(LOFT as usize, &dag)["viewer"]["zones"][0]).unwrap();
    let hent = |d: String| {
        let z = if d == dag { fuld.clone() } else { Doegn::default() };
        async move { Ok(z) }
    };
    let lager = lager_med_zone(Some(Z));
    opdater(&lager, "z1", nu(), &hent, |_, _| {}).await.unwrap();
    let s = lager.lock().unwrap();
    assert!(s.doegn_art("z1", &dag, V_FW).unwrap().is_empty());
    assert_eq!(s.doegn_art("z1", &dag, V_FW_AFSKAARET).unwrap(), [(String::new(), LOFT)]);
    assert!(s.hentet_vaert_fw("z1").unwrap().contains_key(&dag));
}

// ── Kommandoen ─────────────────────────────────────────────────────────────────────────────────────────────────────
fn rk(dag: &str, art: &str, noegle: &str, n: i64) -> Raekke4 {
    (dag.into(), art.into(), noegle.into(), n)
}

fn lager_til_visning() -> Store {
    let mut s = Store::i_hukommelse().unwrap();
    let zone = "0123456789abcdef0123456789abcdef";
    s.gem_zoner(
        1,
        &[Zone {
            id: zone.into(),
            navn: Z.into(),
        }],
        "t",
    )
    .unwrap();
    let dage: Vec<String> = ["2026-11-03", "2026-11-04", "2026-11-05"].iter().map(|d| d.to_string()).collect();
    s.gem_vaert(
        zone,
        &dage,
        &[
            rk("2026-11-03", V_ALLE, "", 900),
            rk("2026-11-03", V_BESOEG, "", 80),
            rk("2026-11-03", V_ALLE, "www", 300),
            rk("2026-11-03", V_ALLE, "shop", 50),
            rk("2026-11-03", V_BESOEG, "shop", 5),
            rk("2026-11-04", V_AFSKAARET, "", LOFT),
            rk("2026-11-05", V_ALLE, "", 700),
            rk("2026-11-05", V_ALLE, ANDRE, 3),
        ],
        "t",
    )
    .unwrap();
    s.gem_vaert_fw(
        zone,
        &dage[..2],
        &[
            rk("2026-11-03", V_FW, "\tfirewallManaged\tblock", 40),
            rk("2026-11-03", V_FW, "\tbic\tmanaged_challenge", 9),
            rk("2026-11-03", V_FW, "shop\twaf\tlog", 4),
            rk("2026-11-04", V_FW, "shop\tfirewallManaged\tblock", 6),
        ],
        "t",
    )
    .unwrap();
    s
}

/// Pr. vært og dag: forespørgsler, besøg, angreb (uden rutine) og afviste; et afskåret eller ikke hentet døgn er `None`, ikke 0.
#[test]
fn subdomaener_regner_pr_vaert_og_dag() {
    let s = lager_til_visning();
    let zone = "0123456789abcdef0123456789abcdef";
    let u = regn(&laes(&s, zone, "2026-11-03").unwrap(), "2026-11-03", "2026-11-05");
    let navne: Vec<&str> = u.vaerter.iter().map(|v| v.vaert.as_str()).collect();
    assert_eq!(navne, ["", "www", "shop", ANDRE], "efter forespørgsler, kategorierne sidst");
    let apex = &u.vaerter[0];
    assert_eq!((apex.foresp, apex.besoeg, apex.angreb, apex.afvist), (1600, 80, 40, 49));
    assert_eq!(apex.dage[1].foresp, None, "afskåret: intet tal");
    assert_eq!(apex.dage[2].angreb, None, "angreb ikke hentet for døgnet");
    let shop = u.vaerter.iter().find(|v| v.vaert == "shop").unwrap();
    assert_eq!(
        (shop.foresp, shop.besoeg, shop.angreb, shop.afvist),
        (50, 5, 10, 6),
        "log er et angreb, men ikke afvist"
    );
    assert_eq!(shop.dage[2].foresp, Some(0), "hentet, ingen trafik: 0");
    assert_eq!(
        (u.trafik_fra.as_deref(), u.angreb_fra.as_deref()),
        (Some("2026-11-03"), Some("2026-11-03"))
    );
    assert_eq!(u.trafik_afskaaret, ["2026-11-04"]);
    assert!(u.angreb_afskaaret.is_empty() && !u.sider_afskaaret && !u.trafik_afvist && !u.angreb_afvist);
    assert!(tjek_periode("2026-11-05", "2026-11-03").is_err() && tjek_periode("2025-11-01", "2026-11-05").is_err());
    assert!(tjek_periode("2026-11-03", "2026-11-05").is_ok() && tjek_periode("2026-11-31", "2026-12-01").is_err());
}

/// Kommandoens svar har ingen id'er: hverken zone-id, 32-tegns hex eller et felt, der hedder noget med id.
#[test]
fn subdomaenernes_svar_har_ingen_ider() {
    let mut s = lager_til_visning();
    let zone = "0123456789abcdef0123456789abcdef";
    s.gem_afsnit_fejl(VAERT_FW_FEJL, zone, Some("afvist")).unwrap();
    s.noter_side_afskaaret(zone, "2026-11-01", "2026-11-04", true).unwrap();
    let u = regn(&laes(&s, zone, "2026-11-03").unwrap(), "2026-11-03", "2026-11-05");
    assert!(u.angreb_afvist && u.sider_afskaaret);
    let tekst = serde_json::to_string(&u).unwrap();
    assert!(
        !tekst.contains(zone) && !tekst.contains("\"id\"") && !tekst.contains("zone"),
        "{tekst}"
    );
    let hex32 = regex::Regex::new("[0-9a-fA-F]{32}").unwrap();
    assert!(!hex32.is_match(&tekst));
}

/// Mærket for afskårne sider lægges sammen og ryddes kun, når et svar under loftet dækker alle de afskårne døgn.
#[test]
fn sidernes_afskaaret_maerke() {
    let mut s = Store::i_hukommelse().unwrap();
    s.noter_side_afskaaret("z", "2026-10-07", "2026-11-05", true).unwrap();
    s.noter_side_afskaaret("z", "2026-11-06", "2026-11-06", true).unwrap();
    assert_eq!(
        s.afsnit_fejl(SIDE_AFSKAARET, "z").unwrap().as_deref(),
        Some("2026-10-07 2026-11-06")
    );
    s.noter_side_afskaaret("z", "2026-11-07", "2026-11-07", false).unwrap();
    assert!(
        s.afsnit_fejl(SIDE_AFSKAARET, "z").unwrap().is_some(),
        "et nyt døgn dækker ikke de gamle"
    );
    s.noter_side_afskaaret("z", "2026-10-07", "2026-11-07", false).unwrap();
    assert_eq!(s.afsnit_fejl(SIDE_AFSKAARET, "z").unwrap(), None);
}

/// En `Fejl` fra rensningen og læsningen af svarene nævner aldrig en vært: kun koden.
#[test]
fn en_fejl_indeholder_aldrig_en_vaert() {
    let vaert = "hemmelig.wi24rd.com";
    let daarlige = [
        json!({ "viewer": { "zones": [{ "va": [{ "count": "x", "dimensions": { "date": "2026-11-05", "clientRequestHTTPHost": vaert } }] }] } }),
        json!({ "viewer": { "zones": [{ "fw": [{ "clientRequestHTTPHost": vaert, "datetime": vaert }] }] } }),
        json!({ "viewer": { "zones": [] }, "vaert": vaert }),
    ];
    let dage = ["2026-11-05".to_string()];
    for d in &daarlige {
        for e in [trafik_raekker(d, &dage, Z).err(), fw_dag_raekker(d, &dage, Z).err()]
            .into_iter()
            .flatten()
        {
            let t = format!("{e} {e:?} {}", serde_json::to_string(&e).unwrap_or_default());
            assert!(!t.contains("hemmelig"), "{t}");
        }
    }
    assert!(trafik_raekker(&daarlige[0], &dage, Z).is_err() && fw_dag_raekker(&daarlige[1], &dage, Z).is_err());
}

// ── B2, trin 0: sikringen af hovedhentningen ────────────────────────────────────────────────────────────────────────
/// Et helt svar på hovedforespørgslen uden værtsfeltet (som før V-SUBDOM.1): samme hændelser som `fw_doegn(false)`.
fn hoved_svar_uden_vaert() -> Value {
    let fw: Vec<Value> = (0..300).map(|i| haendelse(i, None, 1 + (i % 3) as i64)).collect();
    json!({ "viewer": { "zones": [{ "fw": fw, "alle": [{ "count": 5000, "dimensions": { "date": "2026-11-05" } }] }] } })
}

/// Cloudflares valideringsfejl for et felt, zonen ikke må spørge om (som `cf.rs` læser den: `Fejl::GraphQl`).
fn valideringsfejl() -> Fejl {
    Fejl::GraphQl {
        dag: None,
        detalje: "unknown field \"clientRequestHTTPHost\" on type \"ZoneFirewallEventsAdaptive\"".into(),
    }
}

/// Uden værtsfeltet er hovedforespørgslen præcis den fra før V-SUBDOM.1: feltet står kun én gang, sidst på `fw`.
#[test]
fn hovedforespoergslen_uden_vaertsfelt() {
    let q = crate::sti200::hoved_query();
    let uden = uden_vaertsfelt(&q);
    assert_eq!(q.matches("clientRequestHTTPHost").count(), 1);
    assert!(!uden.contains("clientRequestHTTPHost"));
    assert_eq!(uden, q.replace(" clientRequestHTTPHost", ""));
    assert!(uden.contains("datetime sampleInterval }"), "resten af fw er uændret");
}

/// Hvornår der prøves igen: kun ved en GraphQL-fejl, der ikke er "uden for opbevaring"; og hvad der kommer ud.
#[tokio::test]
async fn hent_hoved_proever_en_gang_uden_feltet() {
    let q = crate::sti200::hoved_query();
    let kald: RefCell<Vec<bool>> = RefCell::default();
    // Med feltet: valideringsfejl. Uden: svaret.
    let afviser = |s: String| {
        let med = s.contains("clientRequestHTTPHost");
        kald.borrow_mut().push(med);
        async move {
            if med {
                Err(valideringsfejl())
            } else {
                Ok(hoved_svar_uden_vaert())
            }
        }
    };
    let (svar, afvist) = hent_hoved(&q, false, &afviser).await;
    assert!(svar.is_ok() && afvist);
    assert_eq!(*kald.borrow(), vec![true, false], "først med feltet, så én gang uden");
    kald.borrow_mut().clear();
    let (svar, afvist) = hent_hoved(&q, true, &afviser).await;
    assert!(svar.is_ok() && !afvist);
    assert_eq!(*kald.borrow(), vec![false], "et noteret domæne spørges kun uden feltet");
    // Andre fejl prøves ikke igen: ikke en GraphQL-fejl, og et døgn uden for opbevaringen.
    for f in [
        Fejl::Svar,
        Fejl::GraphQl {
            dag: None,
            detalje: "cannot request data older than 31 days".into(),
        },
    ] {
        kald.borrow_mut().clear();
        let fejler = |s: String| {
            kald.borrow_mut().push(s.contains("clientRequestHTTPHost"));
            let f = f.clone();
            async move { Err::<Value, Fejl>(f) }
        };
        let (svar, afvist) = hent_hoved(&q, false, fejler).await;
        assert!(svar.is_err() && !afvist);
        assert_eq!(*kald.borrow(), vec![true], "{f}");
    }
    // Fejler også forsøget uden feltet, er svaret den første fejl, og domænet noteres ikke.
    kald.borrow_mut().clear();
    let begge = |s: String| {
        let med = s.contains("clientRequestHTTPHost");
        kald.borrow_mut().push(med);
        async move {
            Err::<Value, Fejl>(if med {
                valideringsfejl()
            } else {
                Fejl::GraphQl {
                    dag: None,
                    detalje: "andet".into(),
                }
            })
        }
    };
    let (svar, afvist) = hent_hoved(&q, false, begge).await;
    assert_eq!(svar.unwrap_err().to_string(), valideringsfejl().to_string());
    assert!(!afvist);
    assert_eq!(*kald.borrow(), vec![true, false]);
}

/// Promptens test: et svar med valideringsfejl på feltet giver domænets `fw`, `fwn`, `angreb` og `seneste` som uden feltet, og intet `v_fw`.
/// Domænet noteres, så de næste døgn kun spørges uden feltet.
#[tokio::test]
async fn valideringsfejl_paa_vaertsfeltet_giver_tallene_som_uden_feltet() {
    let q = crate::sti200::hoved_query();
    let noteret = RefCell::new(false);
    let kald: RefCell<Vec<bool>> = RefCell::default();
    let graphql = |s: String| {
        let med = s.contains("clientRequestHTTPHost");
        kald.borrow_mut().push(med);
        async move {
            if med {
                Err(valideringsfejl())
            } else {
                Ok(hoved_svar_uden_vaert())
            }
        }
    };
    let hent = |_: String| {
        let (q, graphql, noteret) = (&q, &graphql, &noteret);
        async move {
            let uden = *noteret.borrow();
            let (svar, afvist) = hent_hoved(q, uden, graphql).await;
            if afvist {
                *noteret.borrow_mut() = true;
            }
            crate::vaern::doegn_fra_svar(&svar?)
        }
    };
    let gammel = |_: String| async { crate::vaern::doegn_fra_svar(&hoved_svar_uden_vaert()) };
    let (sikret, foer) = (lager_med_zone(Some(Z)), lager_med_zone(Some(Z)));
    let antal = (HENT_DAGE + 1) as usize;
    assert_eq!(
        opdater(&sikret, "z1", nu(), &hent, |_, _| {}).await.unwrap(),
        antal,
        "alle døgn gemmes"
    );
    assert_eq!(opdater(&foer, "z1", nu(), &gammel, |_, _| {}).await.unwrap(), antal);
    assert!(*noteret.borrow(), "afslaget er noteret ved domænet");
    let med_felt = kald.borrow().iter().filter(|x| **x).count();
    assert!(
        (1..=crate::vaern::SAMTIDIGE).contains(&med_felt) && kald.borrow().len() == antal + med_felt,
        "kun de første samtidige spørger med feltet: {:?}",
        kald.borrow()
    );
    let (s, f) = (sikret.lock().unwrap(), foer.lock().unwrap());
    let dag = dag_noegle(nu() - DOEGN_MS);
    for art in ["fw", "fwn", "angreb", "seneste"] {
        let a = s.doegn_art("z1", &dag, art).unwrap();
        assert!(!a.is_empty() || art == "fwn", "{art} har tal i fixturen");
        assert_eq!(a, f.doegn_art("z1", &dag, art).unwrap(), "{art}");
    }
    assert_eq!(
        s.raekker("z1", "0000-00-00").unwrap(),
        f.raekker("z1", "0000-00-00").unwrap(),
        "alle døgnets tal er som uden feltet"
    );
    assert!(
        s.doegn_art("z1", &dag, V_FW).unwrap().is_empty() && s.hentet_vaert_fw("z1").unwrap().is_empty(),
        "intet v_fw"
    );
}

// ── FIX 1: lande og robotter pr. vært, noteringens udløb ────────────────────────────────────────────────────────────
/// Kendte robotter pr. rå vært: (vært, kategori, besøg). Tom kategori = ikke en kendt robot.
fn raa_robot() -> Vec<(String, &'static str, i64)> {
    vaerter()
        .into_iter()
        .enumerate()
        .flat_map(|(i, v)| {
            let i = i as i64;
            [
                (v.clone(), "Search Engine Crawler", 3 + i % 5),
                (v.clone(), "AI Crawler", 1 + i % 2),
                (v, "", 9 + i),
            ]
        })
        .collect()
}

/// `vaert_svar` med `vl` (vært × land, fra `raa_trafik`) og `vr` (vært × kategori, fra `raa_robot`).
fn vaert_svar_lr(dag: &str) -> Value {
    let mut s = vaert_svar(dag);
    let z = &mut s["viewer"]["zones"][0];
    z["vl"] = json!(raa_trafik()
        .into_iter()
        .map(|(h, l, _, _, b)| json!({ "sum": { "visits": b }, "dimensions": { "date": dag, "clientRequestHTTPHost": h, "clientCountryName": l } }))
        .collect::<Vec<_>>());
    z["vr"] = json!(raa_robot()
        .into_iter()
        .map(|(h, k, b)| json!({ "count": b * 3, "sum": { "visits": b }, "dimensions": { "date": dag, "clientRequestHTTPHost": h, "verifiedBotCategory": k } }))
        .collect::<Vec<_>>());
    s
}

/// Forespørgslen har de to nye aliaser med eyeball, værten og landet/kategorien, og stadig kun zone og tider som variabler.
#[test]
fn vaertsforespoergslen_har_lande_og_robotter() {
    for (alias, felt) in [("vl:", "clientCountryName"), ("vr:", "verifiedBotCategory")] {
        let linje = VAERT_QUERY.lines().find(|l| l.trim_start().starts_with(alias)).unwrap();
        assert!(
            linje.contains("requestSource:\"eyeball\"") && linje.contains("clientRequestHTTPHost") && linje.contains(felt),
            "{linje}"
        );
        assert!(linje.contains("sum { visits }") && linje.contains("limit:10000"), "{linje}");
    }
    assert!(VAERT_QUERY.contains("vr: httpRequestsAdaptiveGroups") && VAERT_QUERY.matches("httpRequestsAdaptiveGroups").count() == 4);
    for f in ["clientIP", "userAgent", "clientRequestPath", "$zone:", "$dag"] {
        assert!(!VAERT_QUERY.contains(f), "{f}");
    }
}

/// Summen over værterne pr. land er domænets `b_land` for døgnet, og pr. kategori domænets `b_robot` (uden "ikke en kendt robot"); `v_land` og `v_robot`
/// har samme værter som `v_alle` (top 20 og `[andre]`), og `v_alle`/`v_besoeg` er bit for bit som uden de nye aliaser.
#[test]
fn lande_og_robotter_pr_vaert_summer_til_domaenets() {
    let dag = "2026-11-05".to_string();
    let dage = std::slice::from_ref(&dag);
    let (rk, afskaaret) = trafik_raekker(&vaert_svar_lr(&dag), dage, Z).unwrap();
    assert!(!afskaaret);
    let (uden, _) = trafik_raekker(&vaert_svar(&dag), dage, Z).unwrap();
    let kun = |rk: &[Raekke4], arter: &[&str]| rk.iter().filter(|r| arter.contains(&r.1.as_str())).cloned().collect::<Vec<_>>();
    assert_eq!(kun(&rk, &[V_ALLE, V_BESOEG]), uden, "v_alle og v_besoeg er uændrede");
    // Domænets egne tal for samme døgn (besoeg::BESOEG_QUERY og ROBOT_QUERY lægger værterne sammen).
    let mut lande: BTreeMap<&str, i64> = BTreeMap::new();
    for r in raa_trafik() {
        *lande.entry(r.1).or_default() += r.4;
    }
    let besoeg = json!({ "viewer": { "zones": [{ "besoeg": lande.iter().map(|(l, n)| json!({ "sum": { "visits": n }, "dimensions": { "date": dag, "clientCountryName": l } })).collect::<Vec<_>>() }] } });
    let b_land: BTreeMap<String, i64> = crate::besoeg::raekker(&besoeg, dage)
        .unwrap()
        .into_iter()
        .filter(|r| r.1 == "b_land")
        .map(|r| (r.2, r.3))
        .collect();
    let mut kat: BTreeMap<&str, i64> = BTreeMap::new();
    for r in raa_robot() {
        *kat.entry(r.1).or_default() += r.2;
    }
    let robot = json!({ "viewer": { "zones": [{ "robot": kat.iter().map(|(k, n)| json!({ "count": n, "sum": { "visits": n }, "dimensions": { "date": dag, "verifiedBotCategory": k } })).collect::<Vec<_>>() }] } });
    let b_robot: BTreeMap<String, i64> = crate::besoeg::robot_raekker(&robot, dage)
        .unwrap()
        .into_iter()
        .filter(|r| !r.2.is_empty())
        .map(|r| (r.2, r.3))
        .collect();
    let pr = |art: &str| -> BTreeMap<String, i64> {
        let mut m = BTreeMap::new();
        for r in rk.iter().filter(|r| r.1 == art) {
            *m.entry(r.2.split_once('\t').unwrap().1.to_string()).or_default() += r.3;
        }
        m
    };
    assert_eq!(pr(V_LAND), b_land, "summen pr. land = b_land");
    assert_eq!(pr(V_ROBOT), b_robot, "summen pr. kategori = b_robot (kendte robotter)");
    let alle: HashSet<&str> = rk.iter().filter(|r| r.1 == V_ALLE).map(|r| r.2.as_str()).collect();
    for r in rk.iter().filter(|r| r.1 == V_LAND || r.1 == V_ROBOT) {
        let (v, k) = r.2.split_once('\t').unwrap();
        assert!(alle.contains(v), "{v:?} har ingen v_alle");
        assert!(!k.is_empty() && !v.contains("wi24rd") && !v.contains(':'), "{:?}", r.2);
    }
    assert!(
        rk.iter().any(|r| r.1 == V_LAND && r.2.starts_with("[andre]\t")),
        "de mindste værter er [andre]"
    );
    assert!(!rk.iter().any(|r| r.1.ends_with("_afskaaret")));
}

/// Regel S4: står `vl` eller `vr` på loftet, gemmes ingen tal for den art for døgnene, kun mærket; de andre arter er uberørte, og der spørges ikke døgn
/// for døgn (ingen ekstra kald).
#[test]
fn alias_paa_loftet_giver_ingen_tal_for_arten() {
    let dage: Vec<String> = ["2026-11-04", "2026-11-05"].iter().map(|d| d.to_string()).collect();
    let mange = |felt: &str| -> Vec<Value> {
        (0..LOFT)
            .map(|i| json!({ "count": 1, "sum": { "visits": 1 }, "dimensions": { "date": "2026-11-05", "clientRequestHTTPHost": "wi24rd.com", felt: format!("K{i}") } }))
            .collect()
    };
    for (alias, felt, art, maerke, anden) in [
        ("vl", "clientCountryName", V_LAND, V_LAND_AFSKAARET, V_ROBOT),
        ("vr", "verifiedBotCategory", V_ROBOT, V_ROBOT_AFSKAARET, V_LAND),
    ] {
        let mut svar = vaert_svar_lr("2026-11-05");
        svar["viewer"]["zones"][0][alias] = json!(mange(felt));
        let (rk, afskaaret) = trafik_raekker(&svar, &dage, Z).unwrap();
        assert!(!afskaaret, "{alias}: ikke døgn for døgn");
        assert!(!rk.iter().any(|r| r.1 == art), "{alias}: intet tal for arten");
        let m: Vec<&Raekke4> = rk.iter().filter(|r| r.1 == maerke).collect();
        assert_eq!(m.len(), 2, "{alias}: mærket for hvert døgn");
        assert!(m.iter().all(|r| r.2.is_empty() && r.3 == LOFT));
        assert!(rk.iter().any(|r| r.1 == anden), "{alias}: den anden art er uberørt");
        let (uden, _) = trafik_raekker(&vaert_svar("2026-11-05"), &dage, Z).unwrap();
        assert_eq!(
            rk.iter().filter(|r| r.1 == V_ALLE || r.1 == V_BESOEG).cloned().collect::<Vec<_>>(),
            uden
        );
    }
}

/// `b_land`, `b_robot` og `b_total` og hele Besøgende uden filter pr. vært er de samme med og uden de nye arter i databasen, og ingen af arterne pr. vært
/// når Overblik, Angreb og notifikationerne (`Store::raekker`).
#[test]
fn domaenets_besoeg_er_uaendrede_med_vaertsarterne() {
    let z = "0123456789abcdef0123456789abcdef";
    let d = |i: i64| dag_noegle(nu() - i * DOEGN_MS);
    let dage: Vec<String> = (1..=5).map(d).collect();
    let byg = |med_v: bool| {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(
            1,
            &[Zone {
                id: z.into(),
                navn: Z.into(),
            }],
            "t",
        )
        .unwrap();
        let b: Vec<Raekke4> = dage
            .iter()
            .flat_map(|x| [rk(x, "b_land", "DK", 7), rk(x, "b_land", "US", 3), rk(x, "b_total", "besoeg", 10)])
            .collect();
        s.gem_besoeg(z, &dage, &b, "t").unwrap();
        s.gem_robot(
            z,
            &dage,
            &dage.iter().map(|x| rk(x, "b_robot", "AI Crawler", 2)).collect::<Vec<_>>(),
            "t",
        )
        .unwrap();
        if med_v {
            let v: Vec<Raekke4> = dage
                .iter()
                .flat_map(|x| {
                    [
                        rk(x, V_ALLE, "shop", 50),
                        rk(x, V_LAND, "shop\tDK", 4),
                        rk(x, V_ROBOT, "shop\tAI Crawler", 1),
                        rk(x, V_LAND_AFSKAARET, "", LOFT),
                        rk(x, V_ROBOT_AFSKAARET, "", LOFT),
                    ]
                })
                .collect();
            s.gem_vaert(z, &dage, &v, "t").unwrap();
        }
        s
    };
    let (uden, med) = (byg(false), byg(true));
    let zoner = [z.to_string()];
    for art in ["b_land", "b_robot", "b_total"] {
        assert_eq!(
            uden.raekker_art(&zoner, &[art], "0000-00-00").unwrap(),
            med.raekker_art(&zoner, &[art], "0000-00-00").unwrap(),
            "{art}"
        );
    }
    let regn_b = |s: &Store| {
        let mut l = crate::besoeg::laes(s, &zoner).unwrap();
        l.med_vaert = true;
        crate::besoeg::regn(&crate::dyk::Periode::D7, None, &l, nu())
    };
    assert_eq!(regn_b(&uden), regn_b(&med), "Besøgende uden filter pr. vært");
    assert!(
        med.raekker(z, "0000-00-00").unwrap().is_empty(),
        "ingen art pr. vært i Store::raekker"
    );
    assert_eq!(med.raekker_art(&zoner, ALLE_ARTER, "0000-00-00").unwrap().len(), 25);
}

/// Genhentningen én gang (`VAERT_LAND_V1`): døgnene i hentevinduet glemmes i `hentet_vaert` (ét kald pr. domæne: `besoeg::mangler` dækker dem i ét
/// spænd), ældre døgn og alle tal bliver, og det sker kun én gang. Den første dag med lande gemmes.
#[test]
fn lande_og_robotter_hentes_igen_en_gang() {
    let mut s = Store::i_hukommelse().unwrap();
    let d = |i: i64| dag_noegle(nu() - i * DOEGN_MS);
    let alle: Vec<String> = (1..=40).map(d).collect();
    s.gem_vaert("z1", &alle, &alle.iter().map(|x| rk(x, V_ALLE, "", 9)).collect::<Vec<_>>(), "t")
        .unwrap();
    assert_eq!(s.vaert_land_fra().unwrap(), None);
    let fra = d(HENT_DAGE);
    assert!(s.vaert_land_en_gang(&fra).unwrap());
    let hentet = s.hentet_vaert("z1").unwrap();
    assert_eq!(hentet.len(), 10, "kun døgn før vinduet står som hentet");
    assert!(hentet.keys().all(|x| *x < fra));
    assert_eq!(
        s.raekker_art(&["z1".into()], &[V_ALLE], "0000-00-00").unwrap().len(),
        40,
        "tallene står"
    );
    assert_eq!(s.vaert_land_fra().unwrap().as_deref(), Some(fra.as_str()));
    assert_eq!(
        crate::besoeg::mangler(&hentet, nu()),
        Some((d(crate::besoeg::BESOEG_DAGE), d(1))),
        "ét spænd = ét kald pr. domæne"
    );
    s.gem_vaert("z1", &[d(1)], &[], "t").unwrap();
    assert!(!s.vaert_land_en_gang(&d(HENT_DAGE - 1)).unwrap(), "kun én gang");
    assert!(s.hentet_vaert("z1").unwrap().contains_key(&d(1)));
    assert_eq!(s.vaert_land_fra().unwrap().as_deref(), Some(fra.as_str()));
}

/// Værtens lande og robotter: kun dage hentet fra og med genhentningen, ikke dage på loftet (hele svaret eller artens alias), og kun værtens rækker.
#[test]
fn lande_og_robotter_for_en_vaert() {
    let r = |dag: &str, art: &str, k: &str, n: i64| Raekke {
        dag: dag.into(),
        art: art.into(),
        noegle: k.into(),
        n,
    };
    let rk = vec![
        r("2026-10-01", V_LAND, "shop\tDK", 99),
        r("2026-11-01", V_LAND, "shop\tDK", 5),
        r("2026-11-01", V_LAND, "\tDK", 70),
        r("2026-11-01", V_ROBOT, "shop\tAI Crawler", 2),
        r("2026-11-02", V_LAND_AFSKAARET, "", LOFT),
        r("2026-11-02", V_ROBOT, "shop\tAI Crawler", 3),
        r("2026-11-03", V_AFSKAARET, "", LOFT),
        r("2026-11-04", V_LAND, "shop\tSE", 1),
    ];
    let hentet: HashSet<String> = ["2026-10-01", "2026-11-01", "2026-11-02", "2026-11-03", "2026-11-04"]
        .iter()
        .map(|d| d.to_string())
        .collect();
    let v = lr(&rk, &hentet, Some("2026-10-20"), "shop");
    let dage = |m: &HashSet<String>| m.iter().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        dage(&v.land_hentet),
        ["2026-11-01", "2026-11-04"].iter().map(|d| d.to_string()).collect()
    );
    assert_eq!(
        dage(&v.robot_hentet),
        ["2026-11-01", "2026-11-02", "2026-11-04"].iter().map(|d| d.to_string()).collect()
    );
    assert_eq!(v.land, [r("2026-11-01", V_LAND, "DK", 5), r("2026-11-04", V_LAND, "SE", 1)]);
    assert_eq!(v.robot.iter().map(|x| x.n).sum::<i64>(), 5);
    assert_eq!(
        lr(&rk, &hentet, Some("2026-10-20"), "").land,
        [r("2026-11-01", V_LAND, "DK", 70)],
        "domænet selv"
    );
    assert_eq!(lr(&rk, &hentet, None, "shop"), VaertLr::default(), "uden genhentning: ingen dage");
}

/// Besøgende med filter pr. vært: lande, robotter og filtrene land og robot er værtens; IP-klasserne og domænets noter er væk.
#[test]
fn besoegende_med_vaert_har_vaertens_lande_og_robotter() {
    use crate::besoeg::{regn as b_regn, Filter, Laest as BLaest};
    let d = |i: i64| dag_noegle(nu() - i * DOEGN_MS);
    let r = |dag: &str, art: &str, k: &str, n: i64| Raekke {
        dag: dag.into(),
        art: art.into(),
        noegle: k.into(),
        n,
    };
    let mut l = BLaest {
        besoeg: (1..=7)
            .flat_map(|i| {
                [
                    r(&d(i), "b_land", "DK", 100),
                    r(&d(i), "b_total", "besoeg", 100),
                    r(&d(i), "b_robot", "AI Crawler", 9),
                ]
            })
            .collect(),
        besoeg_hentet: (1..=7).map(d).collect(),
        robot_hentet: (1..=7).map(d).collect(),
        ipklasse: vec![r(&d(1), "h_ipklasse", "x", 5)],
        ikke_tilgaengelig: vec![Z.into()],
        ..BLaest::default()
    };
    let v = VaertLr {
        land: vec![r(&d(1), V_LAND, "SE", 4), r(&d(1), V_LAND, "DK", 6), r(&d(2), V_LAND, "SE", 2)],
        robot: vec![r(&d(2), V_ROBOT, "Search Engine Crawler", 3)],
        land_hentet: [d(1), d(2), d(3)].into_iter().collect(),
        robot_hentet: [d(2)].into_iter().collect(),
    };
    crate::besoeg::med_vaert_lr(&mut l, v);
    l.med_vaert = true;
    l.vaert = Some("shop".into());
    let u = b_regn(&crate::dyk::Periode::D7, None, &l, nu());
    assert_eq!(
        u.lande.iter().map(|x| (x.kode.as_str(), x.n)).collect::<Vec<_>>(),
        [("SE", 6), ("DK", 6)]
            .iter()
            .map(|(k, n)| (*k, *n))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
    );
    assert_eq!((u.lande_i_alt, u.lande_med), (12, 3), "andelene er af værtens lande");
    assert_eq!(u.robotter.len(), 1);
    assert_eq!((u.robot_med, u.ipklasse.clone(), u.ikke_tilgaengelig.len()), (1, None, 0));
    let f = b_regn(&crate::dyk::Periode::D7, Some(&Filter::Land("SE".into())), &l, nu());
    let n = |i: i64| f.dage.iter().find(|x| x.dag == d(i)).unwrap().n;
    assert_eq!(
        (n(1), n(2), n(3), n(4)),
        (Some(4), Some(2), Some(0), None),
        "landet pr. dag for værten; en dag uden tal er et hul"
    );
    assert_eq!(f.besoeg30.n, 6);
    let g = b_regn(&crate::dyk::Periode::D7, Some(&Filter::Robot("soeg".into())), &l, nu());
    assert_eq!(g.periode.med, 1, "robotgruppen har værtens dage");
}

/// V1: noteringen gælder til næste UTC-døgn, højst 24 timer.
#[test]
fn noteringen_udloeber_ved_doegnskift() {
    let t = parse_iso("2026-11-05T22:00:00Z").unwrap();
    assert!(uden_gaelder(t, t) && uden_gaelder(t, t + 3_599_999));
    assert!(!uden_gaelder(t, t + 2 * 3_600_000), "næste UTC-døgn");
    let morgen = parse_iso("2026-11-05T00:10:00Z").unwrap();
    assert!(uden_gaelder(morgen, morgen + 23 * 3_600_000));
    assert!(!uden_gaelder(morgen, morgen + DOEGN_MS), "højst 24 timer");
    assert!(!uden_gaelder(t, t - 1), "uret gik baglæns");
}

/// V1: noteringen giver kun `angreb_afvist`, når perioden ingen `v_fw` har; har den tal, vises de. Det gemte afslag (`VAERT_FW_FEJL`) gælder som før.
#[test]
fn angreb_afvist_kun_uden_tal_pr_vaert() {
    let s = lager_til_visning();
    let zone = "0123456789abcdef0123456789abcdef";
    let mut l = laes(&s, zone, "2026-11-01").unwrap();
    l.uden_vaert = true;
    let med_tal = regn(&l, "2026-11-03", "2026-11-05");
    assert!(!med_tal.angreb_afvist && med_tal.vaerter.iter().any(|v| v.angreb > 0));
    assert!(regn(&l, "2026-11-05", "2026-11-05").angreb_afvist, "ingen v_fw i perioden");
    l.uden_vaert = false;
    assert!(!regn(&l, "2026-11-05", "2026-11-05").angreb_afvist, "uden notering");
    l.angreb_afvist = true;
    assert!(regn(&l, "2026-11-03", "2026-11-05").angreb_afvist, "det gemte afslag");
}

/// Fejl i de nye aliaser nævner aldrig værten.
#[test]
fn en_fejl_i_lande_og_robotter_indeholder_aldrig_en_vaert() {
    let dage = ["2026-11-05".to_string()];
    for alias in ["vl", "vr"] {
        let d = json!({ "viewer": { "zones": [{ alias: [{ "sum": { "visits": "x" }, "dimensions": { "date": "2026-11-05", "clientRequestHTTPHost": "hemmelig.wi24rd.com" } }] }] } });
        let e = trafik_raekker(&d, &dage, Z).unwrap_err();
        let t = format!("{e} {e:?} {}", serde_json::to_string(&e).unwrap_or_default());
        assert!(!t.contains("hemmelig"), "{t}");
    }
}
