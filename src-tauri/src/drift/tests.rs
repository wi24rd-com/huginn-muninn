//! V-DRIFT.1 B1. Kun opdigtede fixtures under `wi24rd.com`; ingen kald til Cloudflare.
use super::*;

/// Et døgns rækker for ét domæne, som `Store::gem_drift` gemmer dem.
fn rk(art: &str, noegle: &str, n: i64) -> (String, String, i64) {
    (art.to_string(), noegle.to_string(), n)
}

/// (zone, dag, rækker), som `Store::gem_drift` får dem.
type Doegn<'a> = (&'a str, &'a str, Vec<(String, String, i64)>);

/// Et lager med de givne døgn (alle noteret i `hentet_drift`).
fn lager(doegn: &[Doegn]) -> Store {
    let mut s = Store::i_hukommelse().unwrap();
    for (z, dag, r) in doegn {
        s.gem_drift(z, dag, r, "t").unwrap();
    }
    s
}

fn regn_for(s: &Store, zoner: &[&str], fra: &str, til: &str) -> DriftUd {
    let zoner: Vec<String> = zoner.iter().map(|z| z.to_string()).collect();
    regn(&laes(s, &zoner, fra, None).unwrap(), fra, til)
}

/// En time med `svar` svar, hvoraf `fejl` er 522.
fn time(hh: &str, svar: i64, fejl: i64) -> Vec<(String, String, i64)> {
    let mut v = vec![rk("d_time", hh, svar)];
    if fejl > 0 {
        v.push(rk("d_fejl", &format!("{hh}\t522\t0"), fejl));
    }
    v
}

// ── Timer med fejl ──────────────────────────────────────────────────────────────────────────────────────────────────
/// Grænsen: 4 svar er for få, 5 er nok; 19 % er for lidt, 20 % er nok.
#[test]
fn timer_med_fejl_paa_graensen() {
    let r: Vec<_> = [
        time("01", 10, 4),   // 4 svar: nej
        time("02", 10, 5),   // 5 af 10: ja
        time("03", 100, 19), // 19 %: nej
        time("04", 100, 20), // 20 %: ja
        time("05", 25, 5),   // 5 og 20 %: ja
        time("06", 26, 5),   // 5 og 19,2 %: nej
    ]
    .concat();
    let s = lager(&[("z", "2026-11-03", r)]);
    let u = regn_for(&s, &["z"], "2026-11-03", "2026-11-03");
    assert_eq!(u.dage[0].timer_med_fejl, ["02", "04", "05"]);
    assert_eq!(u.timer.len(), 24, "én dag: timerne med");
    assert!(u.timer[2].med_fejl && !u.timer[1].med_fejl && !u.timer[3].med_fejl);
    assert_eq!(
        u.timer[2].fejl,
        [KodePar {
            kant: 522,
            origin: 0,
            svar: 5
        }]
    );
    assert_eq!(u.timer[0].svar, Some(0), "hentet og nul");
}

/// To timer i træk er ikke et spænd, tre er; og et spænd går hen over midnat UTC.
#[test]
fn spaend_paa_to_og_tre_timer_og_over_midnat() {
    let s = lager(&[
        (
            "z",
            "2026-11-03",
            [
                time("02", 10, 9),
                time("03", 10, 9),
                time("10", 10, 9),
                time("11", 10, 9),
                time("12", 10, 9),
                time("22", 10, 9),
                time("23", 10, 9),
            ]
            .concat(),
        ),
        (
            "z",
            "2026-11-04",
            [time("00", 10, 9), time("01", 10, 9), time("05", 10, 1)].concat(),
        ),
    ]);
    let u = regn_for(&s, &["z"], "2026-11-03", "2026-11-04");
    assert_eq!(
        u.spaend,
        [
            Spaend {
                fra: "2026-11-03T10".into(),
                til: "2026-11-03T12".into(),
                timer: 3,
                forside_fejlede: false
            },
            Spaend {
                fra: "2026-11-03T22".into(),
                til: "2026-11-04T01".into(),
                timer: 4,
                forside_fejlede: false
            },
        ],
        "02–03 er kun to timer"
    );
    assert!(u.timer.is_empty(), "flere dage: ingen timer");
    // Kun den ene side af midnat i perioden: spændet er kortere.
    let u = regn_for(&s, &["z"], "2026-11-04", "2026-11-04");
    assert!(u.spaend.is_empty(), "00–01 er kun to timer");
}

/// Forsiden er vidnet: fejlede den i spændet, står det på spændet og dagen; en robot på én ødelagt sti giver fejl, men forsiden virker.
#[test]
fn forsiden_som_vidne() {
    let mut r = [time("10", 10, 9), time("11", 10, 9), time("12", 10, 9)].concat();
    let s_uden = lager(&[("z", "2026-11-03", r.clone())]);
    r.push(rk("d_forside", "11\t522", 2));
    let s_med = lager(&[("z", "2026-11-03", r)]);
    let (med, uden) = (
        regn_for(&s_med, &["z"], "2026-11-03", "2026-11-03"),
        regn_for(&s_uden, &["z"], "2026-11-03", "2026-11-03"),
    );
    assert!(med.spaend[0].forside_fejlede && !uden.spaend[0].forside_fejlede);
    assert_eq!((med.dage[0].forside_fejl, uden.dage[0].forside_fejl), (Some(true), Some(false)));
    assert_eq!(med.timer[11].forside, Some(2));
    assert_eq!(uden.timer[11].forside, Some(0), "hentet og nul");
}

/// Origin 5xx uden kant-fejl er "serveren svarede med en fejl": ingen time med fejl, intet "kunne ikke nås". 530 tælles for sig og ændrer ikke reglen.
#[test]
fn origin_5xx_uden_kant_fejl_og_530_for_sig() {
    let r = vec![
        rk("d_time", "05", 100),
        rk("d_fejl", "05\t200\t503", 40),
        rk("d_fejl", "05\t520\t503", 3),
        rk("d_fejl", "06\t530\t0", 30),
        rk("d_time", "06", 30),
    ];
    let s = lager(&[("z", "2026-11-03", r)]);
    let d = &regn_for(&s, &["z"], "2026-11-03", "2026-11-03").dage[0];
    assert_eq!(d.server_fejl, Some(43), "origin 500–599, uanset kant");
    assert_eq!(d.kunne_ikke, Some(3));
    assert_eq!(d.kunne_ikke_530, Some(30));
    assert!(
        d.timer_med_fejl.is_empty(),
        "530 er ikke i SERVER_KODER i B1, og origin-fejl ændrer intet"
    );
    assert!(!SERVER_KODER.contains(&530), "B1 ændrer ikke reglen (D4)");
}

/// "Ikke hentet" er aldrig 0: et døgn uden `hentet_drift`, et døgn med `d_udenfor`, og alle domæner skal have døgnet, før det samlede tal står.
#[test]
fn ikke_hentet_er_aldrig_0() {
    let mut s = lager(&[("a", "2026-11-03", time("05", 10, 0)), ("b", "2026-11-04", time("05", 10, 0))]);
    s.gem_drift("a", "2026-11-04", &[rk("d_udenfor", "", 0)], "t").unwrap();
    let u = regn_for(&s, &["a"], "2026-11-02", "2026-11-04");
    let d = |i: usize| &u.dage[i];
    assert!(!d(0).hentet && d(0).svar.is_none() && d(0).kunne_ikke.is_none() && d(0).cache.is_none() && d(0).forside_fejl.is_none());
    assert!(d(1).hentet && d(1).svar == Some(10) && d(1).kunne_ikke == Some(0), "hentet og nul");
    assert!(!d(2).hentet && d(2).svar.is_none(), "uden for opbevaringen er ikke hentet");
    assert_eq!(u.foerste_dag.as_deref(), Some("2026-11-03"));
    let begge = regn_for(&s, &["a", "b"], "2026-11-03", "2026-11-04");
    assert!(
        begge.dage.iter().all(|d| !d.hentet && d.svar.is_none()),
        "hellere intet tal end et for lavt (S4)"
    );
    // Afskåret for én art: kun den art mangler.
    let s = lager(&[(
        "a",
        "2026-11-03",
        [time("05", 10, 2), vec![rk("d_afskaaret", "d_fejl", 10_000)]].concat(),
    )]);
    let d = &regn_for(&s, &["a"], "2026-11-03", "2026-11-03").dage[0];
    assert!(d.hentet && d.svar == Some(10) && d.kunne_ikke.is_none() && d.server_fejl.is_none());
    assert_eq!(d.afskaaret, ["d_fejl"]);
}

/// En tom periode og et omfang uden domæner.
#[test]
fn tom_periode() {
    let s = lager(&[]);
    let u = regn_for(&s, &["a"], "2026-11-03", "2026-11-03");
    assert!(!u.dage[0].hentet && u.spaend.is_empty() && u.cache.is_none() && u.foerste_dag.is_none() && u.timer.is_empty());
    let u = regn_for(&s, &[], "2026-11-03", "2026-11-04");
    assert_eq!(u.dage.len(), 2);
    assert!(u.dage.iter().all(|d| !d.hentet && d.svar.is_none()));
}

// ── Cache ──────────────────────────────────────────────────────────────────────────────────────────────────────────
/// Grupperne summer til dagens svar og bytes; `revalidated` er hentet hos serveren i forespørgsler og fra lageret i data; `stale` for sig; en ukendt
/// status står i `andet`.
#[test]
fn cachegrupperne_summer_og_revalidated_skifter_side() {
    let status = [
        ("hit", 30, 3000),
        ("stale", 2, 200),
        ("updating", 1, 100),
        ("revalidated", 4, 400),
        ("none", 10, 50),
        ("dynamic", 40, 8000),
        ("miss", 5, 500),
        ("expired", 1, 10),
        ("bypass", 2, 20),
        ("[ukendt]", 3, 30),
    ];
    let r: Vec<_> = status
        .iter()
        .flat_map(|(s, n, b)| [rk("d_cache", s, *n), rk("d_cache_b", s, *b)])
        .collect();
    let s = lager(&[("z", "2026-11-03", r.clone()), ("z", "2026-11-04", r)]);
    let u = regn_for(&s, &["z"], "2026-11-03", "2026-11-04");
    let c = u.dage[0].cache.as_ref().unwrap();
    let (svar, bytes): (i64, i64) = status.iter().fold((0, 0), |a, x| (a.0 + x.1, a.1 + x.2));
    let g = |g: &Grupper| g.lager + g.selv + g.server + g.andet;
    assert_eq!((g(&c.svar), g(&c.data)), (svar, bytes), "summer til dagens svar og bytes");
    assert_eq!(
        c.svar,
        Grupper {
            lager: 33,
            selv: 10,
            server: 52,
            andet: 3
        },
        "revalidated hos serveren"
    );
    assert_eq!(
        c.data,
        Grupper {
            lager: 3700,
            selv: 50,
            server: 8530,
            andet: 30
        },
        "revalidated fra lageret"
    );
    assert_eq!(c.stale, 2);
    assert_eq!(c.status[0].status, "dynamic", "flest svar først");
    let p = u.cache.unwrap();
    assert_eq!((g(&p.svar), g(&p.data)), (2 * svar, 2 * bytes), "perioden");
    assert!(!u.cache_hele_domaenet);
}

// ── Fejl pr. vært ─────────────────────────────────────────────────────────────────────────────────────────────────────
/// Markøren skiller gamle døgn fra nye: før den er værtens døgn "ikke hentet" (aldrig 0), fra den er de hentet; loft og afslag giver intet tal. Med en
/// vært valgt er cachen markeret som hele domænets, og summen over værterne er aldrig over domænets fejl.
#[test]
fn fejl_pr_vaert_og_markoeren() {
    let mut s = lager(&[(
        "z",
        "2026-11-05",
        [
            time("05", 100, 30),
            vec![rk("d_cache", "dynamic", 100), rk("d_cache_b", "dynamic", 1000)],
        ]
        .concat(),
    )]);
    let dage: Vec<String> = ["2026-11-03", "2026-11-04", "2026-11-05", "2026-11-06", "2026-11-07"]
        .iter()
        .map(|d| d.to_string())
        .collect();
    s.gem_vaert("z", &dage[..1], &[], "t").unwrap(); // hentet før V-DRIFT.1
    assert!(s.vaert_5xx_en_gang("2026-11-04").unwrap());
    let v = |dag: &str, art: &str, k: &str, n: i64| (dag.to_string(), art.to_string(), k.to_string(), n);
    s.gem_vaert(
        "z",
        &dage[1..],
        &[
            v("2026-11-05", V_FEJL, "shop\t522", 20),
            v("2026-11-05", V_FEJL, "shop\t530", 2),
            v("2026-11-05", V_FEJL, "shop\t502", 1),
            v("2026-11-05", V_FEJL, "www\t522", 9),
            v("2026-11-06", V_FEJL_AFSKAARET, "", 10_000),
            v("2026-11-07", V_FEJL_AFVIST, "", 0),
        ],
        "t",
    )
    .unwrap();
    let l = laes(&s, &["z".to_string()], "2026-11-03", Some("shop")).unwrap();
    let u = regn(&l, "2026-11-03", "2026-11-07");
    let vu = u.vaert.as_ref().unwrap();
    let d = |i: usize| &vu.dage[i];
    assert!(!d(0).hentet && d(0).kunne_ikke.is_none(), "før markøren: ikke hentet, aldrig 0");
    assert!(d(1).hentet && d(1).kunne_ikke == Some(0), "efter markøren: hentet og nul");
    assert_eq!((d(2).kunne_ikke, d(2).kunne_ikke_530, d(2).andre_5xx), (Some(20), Some(2), Some(1)));
    assert!(
        !d(3).hentet && d(3).kunne_ikke.is_none() && !d(4).hentet,
        "loft og afslag: intet tal"
    );
    assert_eq!(
        (vu.afskaaret.clone(), vu.afvist.clone()),
        (vec!["2026-11-06".to_string()], vec!["2026-11-07".to_string()])
    );
    assert_eq!(vu.fra.as_deref(), Some("2026-11-04"));
    assert!(u.cache_hele_domaenet && u.cache.is_some(), "cachen er domænets og siger det");
    // Summen over værterne (kant 520–527) er ikke over domænets.
    let alle_vaerter: i64 = ["shop", "www"]
        .iter()
        .map(|h| {
            let l = laes(&s, &["z".to_string()], "2026-11-05", Some(h)).unwrap();
            regn(&l, "2026-11-05", "2026-11-05").vaert.unwrap().dage[0].kunne_ikke.unwrap()
        })
        .sum();
    assert!(alle_vaerter <= u.dage[2].kunne_ikke.unwrap(), "{alle_vaerter}");
}

// ── Sikkerhed ──────────────────────────────────────────────────────────────────────────────────────────────────────────
/// Svaret har ingen id'er og intet værtsnavn ud over den valgte label, og en `Fejl` fra fejl- og cachetallene indeholder aldrig en vært.
#[test]
fn svaret_har_ingen_ider_og_ingen_vaerter() {
    let zone = "testzonetestzonetestzonetestzone";
    let mut s = lager(&[(
        zone,
        "2026-11-05",
        [time("05", 100, 30), vec![rk("d_cache", "hit", 1), rk("d_cache_b", "hit", 1)]].concat(),
    )]);
    s.vaert_5xx_en_gang("2026-11-01").unwrap();
    let v = |k: &str| ("2026-11-05".to_string(), V_FEJL.to_string(), k.to_string(), 3);
    s.gem_vaert(zone, &["2026-11-05".into()], &[v("hemmelig\t522"), v("shop\t522")], "t")
        .unwrap();
    for vaert in [None, Some("shop")] {
        let l = laes(&s, &[zone.to_string()], "2026-11-01", vaert).unwrap();
        let tekst = serde_json::to_string(&regn(&l, "2026-11-01", "2026-11-05")).unwrap();
        assert!(
            !tekst.contains(zone) && !tekst.contains("hemmelig") && !tekst.contains("wi24rd"),
            "{tekst}"
        );
    }
    // En fejl fra svaret: ingen vært og intet indhold fra svaret.
    let skrald = serde_json::json!({ "viewer": { "zones": [{ "d_time": [{ "count": "hemmelig.wi24rd.com" }] }] } });
    let f = crate::adgang::drift_fra_svar(&skrald, "2026-11-05").unwrap_err();
    assert!(!f.to_string().contains("hemmelig") && !serde_json::to_string(&f).unwrap().contains("hemmelig"));
    let f = crate::vaert::trafik_raekker(
        &serde_json::json!({ "viewer": { "zones": [{ "vf": [{ "count": "x", "dimensions": { "clientRequestHTTPHost": "hemmelig.wi24rd.com" } }] }] } }),
        &["2026-11-05".to_string()],
        "wi24rd.com",
    )
    .unwrap_err();
    assert!(!f.to_string().contains("hemmelig"));
}

/// Ingen forespørgsel i programmet beder om `originIP` eller `clientIP` (eller nogen anden IP-adresse).
#[test]
fn ingen_forespoergsel_har_originip_eller_clientip() {
    let alle: Vec<String> = vec![
        crate::adgang::ADGANG_QUERY.to_string(),
        crate::adgang::query(true, true),
        crate::adgang::query(true, false),
        crate::adgang::query(false, true),
        crate::adgang::drift_query(),
        crate::adgang::LAEK_QUERY.to_string(),
        crate::adgang::LAEK_QUERY_UDEN_FILTER.to_string(),
        crate::besoeg::BESOEG_QUERY.to_string(),
        crate::besoeg::ROBOT_QUERY.to_string(),
        crate::besoeg::SIDE_QUERY.to_string(),
        crate::dmarc::DMARC_QUERY.to_string(),
        crate::dns::DNS_QUERY.to_string(),
        crate::historik::HISTORIK_QUERY.to_string(),
        crate::sti200::hoved_query(),
        crate::vaern::DOEGN_QUERY.to_string(),
        crate::vaern::ANGREB_QUERY.to_string(),
        crate::vaert::VAERT_QUERY.to_string(),
        crate::vaert::uden_vf(crate::vaert::VAERT_QUERY),
        crate::vaert::VAERT_FW_QUERY.to_string(),
        crate::cf::RET_QUERY.to_string(),
    ];
    for q in &alle {
        let l = q.to_lowercase();
        assert!(!l.contains("originip") && !l.contains("clientip"), "{q}");
    }
    assert_eq!(alle.len(), 20);
}
