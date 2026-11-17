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

/// Origin 5xx uden kant-fejl er "websitet svarede med en fejl": ingen time med fejl, intet "kunne ikke nås". 530 tælles for sig i svaret, og fra B2 (D4)
/// er den med i reglen for en time med fejl. FIX 1: 520 med origin 503 er "kunne ikke nås" og tælles ikke én gang til som websitets fejl.
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
    assert_eq!(d.website_fejl, Some(40), "kun 200/503: 520/503 er allerede talt som kunne ikke nås");
    assert_eq!(d.fejl, Some(73), "hvert svar én gang");
    assert_eq!(d.kunne_ikke, Some(3));
    assert_eq!(d.kunne_ikke_530, Some(30));
    assert_eq!(
        d.timer_med_fejl,
        ["06"],
        "530 er i SERVER_KODER fra B2 (D4); origin-fejl ændrer intet"
    );
    assert!(SERVER_KODER.contains(&530) && !SERVER_KODER.contains(&528) && !SERVER_KODER.contains(&529));
}

/// B2 (D4): timens regel med 530 alene, 530 sammen med 52x, og 528/529, der ikke tæller.
#[test]
fn timer_med_fejl_foelger_reglen_med_530() {
    let r = vec![
        rk("d_time", "01", 10),
        rk("d_fejl", "01\t530\t0", 5), // 530 alene: ja
        rk("d_time", "02", 20),
        rk("d_fejl", "02\t522\t0", 2),
        rk("d_fejl", "02\t530\t0", 2), // 4 svar i alt: nej
        rk("d_time", "03", 20),
        rk("d_fejl", "03\t522\t0", 3),
        rk("d_fejl", "03\t530\t0", 2), // 5 svar og 25 %: ja
        rk("d_time", "04", 20),
        rk("d_fejl", "04\t528\t0", 10),
        rk("d_fejl", "04\t529\t0", 10), // 528/529: nej
        rk("d_time", "05", 20),
        rk("d_fejl", "05\t522\t0", 4),
        rk("d_fejl", "05\t528\t0", 16), // kun de 4 tæller: nej
    ];
    let s = lager(&[("z", "2026-11-03", r)]);
    let u = regn_for(&s, &["z"], "2026-11-03", "2026-11-03");
    assert_eq!(u.dage[0].timer_med_fejl, ["01", "03"]);
    assert_eq!((u.dage[0].kunne_ikke, u.dage[0].kunne_ikke_530), (Some(9), Some(9)));
}

/// FIX 1: fordelingen i de to fliser. En klassisk servers 502 (origin 502), et website hos Cloudflare, der fejler (kant 500, origin 0), en fejl, hvor
/// Cloudflare ikke fik et brugbart svar (kant 520, origin 0), en fejl fra serveren, som den besøgende ikke fik som fejl (kant 200, origin 503), og 530.
/// De to flisers sum er dagens svar med fejl, og intet svar tælles to gange. Timer med fejl og spænd følger stadig kun "kunne ikke nås".
#[test]
fn fordelingen_i_de_to_fliser() {
    let r = vec![
        rk("d_time", "03", 100),
        rk("d_fejl", "03\t502\t502", 11), // klassisk server: websitet
        rk("d_fejl", "03\t500\t0", 30),   // Worker eller Pages, der fejler: websitet
        rk("d_fejl", "03\t520\t0", 7),    // kunne ikke nås
        rk("d_fejl", "03\t200\t503", 5),  // serveren fejlede, kanten svarede 200: websitet
        rk("d_fejl", "03\t530\t0", 3),    // kunne ikke nås (530)
        rk("d_time", "04", 40),
        rk("d_fejl", "04\t500\t0", 40), // hele timen fejlede med 500: stadig ingen time med fejl
        rk("d_time", "05", 40),
        rk("d_fejl", "05\t500\t0", 40),
        rk("d_time", "06", 40),
        rk("d_fejl", "06\t500\t0", 40),
        rk("d_forside", "04\t500", 12),
    ];
    let s = lager(&[("z", "2026-11-03", r)]);
    let u = regn_for(&s, &["z"], "2026-11-03", "2026-11-03");
    let d = &u.dage[0];
    assert_eq!((d.kunne_ikke, d.kunne_ikke_530), (Some(7), Some(3)));
    assert_eq!(d.website_fejl, Some(11 + 30 + 5 + 120));
    assert_eq!(d.fejl, Some(176));
    assert_eq!(
        d.kunne_ikke.unwrap() + d.kunne_ikke_530.unwrap() + d.website_fejl.unwrap(),
        d.fejl.unwrap(),
        "de to flisers sum er dagens svar med fejl"
    );
    assert_eq!(d.forside_fejl, Some(true));
    assert!(
        d.timer_med_fejl.is_empty() && u.spaend.is_empty(),
        "websitets egne fejl giver hverken timer med fejl eller spænd (D5)"
    );
    // Rækkerne: hver i præcis én flise, og summerne er flisernes.
    let f = fejl_for(&s, &["z"], "2026-11-03", "2026-11-03", None);
    let koder: Vec<_> = f.raekker.iter().filter(|r| r.slags == "kode").collect();
    assert_eq!(koder.len(), 5);
    assert!(koder.iter().all(|r| r.naa != r.website));
    let sum = |p: &dyn Fn(&FejlRaekke) -> bool| koder.iter().filter(|r| p(r)).map(|r| r.svar).sum::<i64>();
    assert_eq!((sum(&|r| r.naa), sum(&|r| r.website)), (10, 166));
    let par = |k: i64, o: i64| koder.iter().find(|r| r.kant == k && r.origin == o).unwrap();
    assert_eq!((par(502, 502).website, par(502, 502).server), (true, true));
    assert_eq!((par(500, 0).website, par(500, 0).server, par(500, 0).svar), (true, false, 150));
    assert_eq!((par(520, 0).naa, par(520, 0).website), (true, false));
    assert_eq!((par(200, 503).website, par(200, 503).server), (true, true));
    assert_eq!((par(530, 0).naa, par(530, 0).website), (true, false));
    assert!(f.spaend.is_empty());
    // Ikke hentet er aldrig 0.
    let tom = regn_for(&s, &["z"], "2026-11-04", "2026-11-04");
    assert_eq!(tom.dage[0].website_fejl, None);
}

/// B2: spændet i lokal tid. Hen over midnat, ved skiftet til vintertid og til sommertid, det længste spænd vinder, og uden timetal er der intet.
#[test]
fn mellem_i_lokal_tid() {
    use crate::notify::Tidszone;
    use crate::overblik::Mellem;
    let m = |a: &str, b: &str| {
        Some(Mellem {
            fra: a.into(),
            til: b.into(),
        })
    };
    let sp = |fra: &str, til: &str, timer: usize| Spaend {
        fra: fra.into(),
        til: til.into(),
        timer,
        forside_fejlede: false,
    };
    let ms = |iso: &str| crate::vaern::parse_iso(iso).unwrap();
    // Vintertid (UTC+1): 21, 22 og 23 UTC er kl. 22.00 til 01.00 næste lokale dag.
    let vinter = Tidszone(vec![(0, 60)]);
    assert_eq!(mellem_af(&[sp("2026-11-02T21", "2026-11-02T23", 3)], &vinter), m("22.00", "01.00"));
    // Uden tidsskift: UTC.
    assert_eq!(
        mellem_af(&[sp("2026-11-02T21", "2026-11-02T23", 3)], &Tidszone::default()),
        m("21.00", "00.00")
    );
    // København 2026: vintertid fra 25. okt. kl. 01:00 UTC. 00, 01 og 02 UTC er 02.00 (sommertid) til 04.00 (vintertid).
    let efteraar = Tidszone(vec![(0, 120), (ms("2026-10-25T01:00:00Z"), 60)]);
    assert_eq!(
        mellem_af(&[sp("2026-10-25T00", "2026-10-25T02", 3)], &efteraar),
        m("02.00", "04.00")
    );
    // Sommertid fra 29. marts kl. 01:00 UTC: 00, 01 og 02 UTC er 01.00 (vintertid) til 05.00 (sommertid).
    let foraar = Tidszone(vec![(0, 60), (ms("2026-03-29T01:00:00Z"), 120)]);
    assert_eq!(mellem_af(&[sp("2026-03-29T00", "2026-03-29T02", 3)], &foraar), m("01.00", "05.00"));
    // En zone med halve timer.
    assert_eq!(
        mellem_af(&[sp("2026-11-02T10", "2026-11-02T12", 3)], &Tidszone(vec![(0, 330)])),
        m("15.30", "18.30")
    );
    // Det længste spænd; ved lige mange det første.
    let to = [sp("2026-11-02T01", "2026-11-02T03", 3), sp("2026-11-02T10", "2026-11-02T14", 5)];
    assert_eq!(mellem_af(&to, &Tidszone::default()), m("10.00", "15.00"));
    let lige = [sp("2026-11-02T01", "2026-11-02T03", 3), sp("2026-11-02T10", "2026-11-02T12", 3)];
    assert_eq!(mellem_af(&lige, &Tidszone::default()), m("01.00", "04.00"));
    assert_eq!(mellem_af(&[], &vinter), None);
    assert_eq!(
        mellem_af(&[sp("ugyldig", "2026-11-02T03", 3)], &vinter),
        None,
        "kan tiden ikke læses, er der intet spænd"
    );

    // Fra lageret: tre timer i træk giver et spænd; to gør ikke; ikke hentet og afskåret gør ikke.
    let nat: Vec<_> = [time("21", 10, 5), time("22", 10, 5), time("23", 10, 5)].concat();
    let s = lager(&[("z", "2026-11-02", nat.clone()), ("to", "2026-11-02", nat[..4].to_vec())]);
    assert_eq!(mellem(&s, "z", "2026-11-02", &vinter).unwrap(), m("22.00", "01.00"));
    assert_eq!(mellem(&s, "to", "2026-11-02", &vinter).unwrap(), None, "to timer er ikke et spænd");
    assert_eq!(mellem(&s, "z", "2026-11-01", &vinter).unwrap(), None, "et andet døgn");
    assert_eq!(mellem(&s, "ukendt", "2026-11-02", &vinter).unwrap(), None, "ikke hentet");
    let afsk = lager(&[("z", "2026-11-02", [nat, vec![rk("d_afskaaret", "d_fejl", 1)]].concat())]);
    assert_eq!(
        mellem(&afsk, "z", "2026-11-02", &vinter).unwrap(),
        None,
        "afskåret: teksten er som uden"
    );
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
    assert!(d.hentet && d.svar == Some(10) && d.kunne_ikke.is_none() && d.website_fejl.is_none());
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

// ── B2: tabellen, spændene pr. domæne og værterne med fejl ──────────────────────────────────────────────────────────
fn fejl_for(s: &Store, zoner: &[&str], fra: &str, til: &str, vaert: Option<&str>) -> FejlUd {
    let zoner: Vec<String> = zoner.iter().map(|z| z.to_string()).collect();
    regn_fejl(&laes(s, &zoner, fra, vaert).unwrap(), fra, til)
}

/// Én række pr. domæne og kode-par med første og sidste time; forsiden for sig; svarene i timer med fejl; timerne kun for én dag. Summen af rækkerne
/// er dagens `fejl`, "kunne ikke nås" er rækkerne med `naa`, og "websitet svarede med en fejl" er rækkerne med `website` (FIX 1: resten, hver række i
/// præcis én af de to).
#[test]
fn tabellen_har_et_kodepar_pr_domaene_med_foerste_og_sidste_time() {
    let a3: Vec<_> = [
        time("02", 10, 6),
        time("03", 10, 6),
        time("04", 10, 6),
        vec![
            rk("d_fejl", "04\t520\t503", 2),
            rk("d_fejl", "09\t200\t500", 4),
            rk("d_time", "09", 50),
            rk("d_fejl", "09\t502\t0", 1),
            rk("d_forside", "03\t522", 2),
        ],
    ]
    .concat();
    let s = lager(&[
        ("a", "2026-11-03", a3),
        ("a", "2026-11-04", time("23", 10, 1)),
        ("b", "2026-11-03", time("01", 10, 0)),
        ("b", "2026-11-04", [time("05", 10, 5), vec![rk("d_fejl", "05\t530\t0", 3)]].concat()),
    ]);
    let u = fejl_for(&s, &["a", "b"], "2026-11-03", "2026-11-04", None);
    let r = |zone: &str, slags: &str, kant: i64, origin: i64| {
        u.raekker
            .iter()
            .find(|r| r.zone == zone && r.slags == slags && r.kant == kant && r.origin == origin)
            .unwrap_or_else(|| panic!("{zone} {slags} {kant}/{origin}"))
    };
    let x = r("a", "kode", 522, 0);
    assert_eq!((x.svar, x.i_fejltimer, x.naa, x.server, x.website), (19, 18, true, false, false));
    assert_eq!((x.foerste.as_str(), x.sidste.as_str()), ("2026-11-03T02", "2026-11-04T23"));
    assert!(x.timer.is_empty(), "flere dage: ingen timer");
    let x = r("a", "kode", 520, 503);
    assert_eq!(
        (x.svar, x.i_fejltimer, x.naa, x.server, x.website),
        (2, 2, true, true, false),
        "kunne ikke nås, og serveren svarede 503: teksten nævner begge, men svaret tælles kun som kunne ikke nås"
    );
    let x = r("a", "kode", 200, 500);
    assert_eq!((x.svar, x.i_fejltimer, x.naa, x.server, x.website), (4, 0, false, true, true));
    let x = r("a", "kode", 502, 0);
    assert_eq!(
        (x.naa, x.server, x.website),
        (false, false, true),
        "en fejl uden svar fra en server: websitets fejl"
    );
    let x = r("a", "forside", 0, 0);
    assert_eq!(
        (x.svar, x.i_fejltimer, x.foerste.as_str(), x.sidste.as_str()),
        (2, 2, "2026-11-03T03", "2026-11-03T03")
    );
    assert!(!x.naa && !x.server && !x.website);
    assert!(r("b", "kode", 530, 0).naa);
    assert!(
        u.raekker.iter().filter(|r| r.slags == "kode").all(|r| r.naa != r.website),
        "hver række i præcis én flise"
    );
    assert_eq!(u.raekker[0].svar, 19, "flest svar først");
    // Summerne passer med `regn`.
    let d = regn_for(&s, &["a", "b"], "2026-11-03", "2026-11-04");
    let sum = |f: &dyn Fn(&FejlRaekke) -> bool| u.raekker.iter().filter(|r| r.slags == "kode" && f(r)).map(|r| r.svar).sum::<i64>();
    let dage = |f: &dyn Fn(&DriftDag) -> Option<i64>| d.dage.iter().map(|x| f(x).unwrap()).sum::<i64>();
    assert_eq!(sum(&|_| true), dage(&|x| x.fejl));
    assert_eq!(sum(&|r| r.naa), dage(&|x| Some(x.kunne_ikke? + x.kunne_ikke_530?)));
    assert_eq!(sum(&|r| r.website), dage(&|x| x.website_fejl));
    assert_eq!(
        sum(&|r| r.naa) + sum(&|r| r.website),
        dage(&|x| x.fejl),
        "de to flisers sum er svarene med fejl"
    );
    assert_eq!(d.dage[0].fejl, Some(25));
    // Spændet har domænet med.
    assert_eq!(
        u.spaend,
        [ZoneSpaend {
            zone: "a".into(),
            fra: "2026-11-03T02".into(),
            til: "2026-11-03T04".into(),
            timer: 3,
            forside_fejlede: true
        }]
    );
    // Én dag: timerne står på rækken.
    let en = fejl_for(&s, &["a"], "2026-11-03", "2026-11-03", None);
    let x = en.raekker.iter().find(|r| r.kant == 522).unwrap();
    let timer: Vec<(&str, i64)> = x.timer.iter().map(|t| (t.time.as_str(), t.svar)).collect();
    assert_eq!(timer, [("02", 6), ("03", 6), ("04", 6)]);
    assert!(en.vaerter.is_empty() && fejl_for(&s, &[], "2026-11-03", "2026-11-04", None).raekker.is_empty());
}

/// Et døgn, der ikke er hentet, eller hvor fejlene stod på loftet, giver ingen rækker (aldrig et for lavt tal), og cachegrupperne står på hver status.
#[test]
fn tabellen_udelader_ikke_hentet_og_afskaaret() {
    let mut s = lager(&[
        (
            "a",
            "2026-11-03",
            [
                time("02", 10, 6),
                vec![rk("d_afskaaret", "d_fejl", 1), rk("d_forside", "02\t522", 1)],
            ]
            .concat(),
        ),
        (
            "a",
            "2026-11-04",
            [
                time("02", 10, 6),
                vec![
                    rk("d_cache", "hit", 3),
                    rk("d_cache", "revalidated", 2),
                    rk("d_cache", "nyt_ord", 1),
                ],
            ]
            .concat(),
        ),
    ]);
    s.gem_drift("a", "2026-11-05", &[rk("d_udenfor", "", 1)], "t").unwrap();
    let u = fejl_for(&s, &["a"], "2026-11-02", "2026-11-05", None);
    let par: Vec<(&str, i64, i64, &str)> = u.raekker.iter().map(|r| (r.slags, r.kant, r.svar, r.foerste.as_str())).collect();
    assert_eq!(par, [("kode", 522, 6, "2026-11-04T02"), ("forside", 0, 1, "2026-11-03T02")]);
    let d = regn_for(&s, &["a"], "2026-11-02", "2026-11-05");
    assert_eq!(
        d.dage.iter().map(|x| x.fejl).collect::<Vec<_>>(),
        [None, None, Some(6), None],
        "ikke hentet, afskåret og uden for opbevaringen er aldrig 0"
    );
    let c = d.cache.unwrap();
    let grupper: Vec<(&str, &str, &str)> = c.status.iter().map(|x| (x.status.as_str(), x.gruppe, x.gruppe_data)).collect();
    assert_eq!(
        grupper,
        [
            ("hit", "lager", "lager"),
            ("revalidated", "server", "lager"),
            ("nyt_ord", "andet", "andet")
        ]
    );
    assert_eq!(
        c.svar.lager + c.svar.selv + c.svar.server + c.svar.andet,
        6,
        "grupperne summer til alle svar"
    );
}

/// Værterne med fejl for ét domæne (flest først, kategorierne med), og rækkerne for den valgte vært: én pr. kant-kode med første og sidste dag. Med flere
/// domæner står ingen værter. `drift` selv nævner stadig aldrig en anden vært.
#[test]
fn vaerterne_med_fejl_og_den_valgte_vaerts_raekker() {
    let mut s = lager(&[("z", "2026-11-05", time("05", 100, 30)), ("y", "2026-11-05", time("05", 100, 30))]);
    s.vaert_5xx_en_gang("2026-11-04").unwrap();
    let v = |dag: &str, art: &str, k: &str, n: i64| (dag.to_string(), art.to_string(), k.to_string(), n);
    let dage: Vec<String> = ["2026-11-04", "2026-11-05", "2026-11-06", "2026-11-07"]
        .iter()
        .map(|d| d.to_string())
        .collect();
    s.gem_vaert(
        "z",
        &dage,
        &[
            v("2026-11-04", V_FEJL, "shop\t522", 5),
            v("2026-11-05", V_FEJL, "shop\t522", 20),
            v("2026-11-05", V_FEJL, "shop\t530", 2),
            v("2026-11-05", V_FEJL, "shop\t502", 1),
            v("2026-11-05", V_FEJL, "www\t522", 9),
            v("2026-11-05", V_FEJL, "[andre]\t500", 4),
            v("2026-11-06", V_FEJL, "www\t522", 99),
            v("2026-11-06", V_FEJL_AFSKAARET, "", 10_000),
        ],
        "t",
    )
    .unwrap();
    let u = fejl_for(&s, &["z"], "2026-11-04", "2026-11-07", None);
    let vaerter: Vec<(&str, i64, i64)> = u.vaerter.iter().map(|x| (x.vaert.as_str(), x.svar, x.kunne_ikke)).collect();
    assert_eq!(
        vaerter,
        [("shop", 28, 27), ("www", 9, 9), ("[andre]", 4, 0)],
        "døgnet på loftet tæller ikke"
    );
    assert!(u.raekker.iter().all(|r| r.slags != "vaert"), "uden filter: domænets rækker");
    let m = fejl_for(&s, &["z"], "2026-11-04", "2026-11-07", Some("shop"));
    let rk: Vec<(&str, i64, i64, bool, &str, &str)> = m
        .raekker
        .iter()
        .map(|r| (r.slags, r.kant, r.svar, r.naa, r.foerste.as_str(), r.sidste.as_str()))
        .collect();
    assert_eq!(
        rk,
        [
            ("vaert", 522, 25, true, "2026-11-04", "2026-11-05"),
            ("vaert", 530, 2, true, "2026-11-05", "2026-11-05"),
            ("vaert", 502, 1, false, "2026-11-05", "2026-11-05"),
        ]
    );
    assert!(
        fejl_for(&s, &["z", "y"], "2026-11-04", "2026-11-07", None).vaerter.is_empty(),
        "flere domæner: ingen værter"
    );
    let drift = serde_json::to_string(&regn_for(&s, &["z"], "2026-11-04", "2026-11-07")).unwrap();
    assert!(!drift.contains("shop") && !drift.contains("www"), "{drift}");
}
