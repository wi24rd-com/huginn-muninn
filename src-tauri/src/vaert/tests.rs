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
