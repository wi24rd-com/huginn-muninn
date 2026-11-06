//! Robotter: hvem der læser sitet, ud fra det, cachen allerede har (`bot`, `falsk`, `alle`) og de ekstra tal (`stoppet`).
//! `fil_art` og `bygg_crawlere` er en port af W `worker/src/features/vagt.js` (`filArt`, `byggCrawlere`; samme ophavsmand); testen
//! kører mod W's egen JavaScript (`robotter/paritet.json`). Grupperne er Cloudflares `verifiedBotCategory`, så der er ingen liste
//! at vedligeholde: en ukendt eller ny kategori (fx fra inddelingen af 1. juli 2026) lander i Ukendt, aldrig i en fejl.
//! Labyrinten er ikke en mitigering (Cloudflare), så den står for sig som "lokket ind", aldrig som stoppet. Perioderne er hele
//! UTC-døgn som i dyk, og et klik viser samme tal i dyk (`Valg`).
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Serialize;

use crate::adgang::vinduet;
use crate::cf::{Fejl, Zone};
use crate::dyk::{periode_ud, spaend, top_af, Periode, PeriodeUd, Spoergsmaal, VaerdiN};
use crate::overblik::{gruppe as handling_gruppe, BLOKERET, LABYRINT, UDFORDRET};
use crate::store::{Raekke, Store};
use crate::vaern::{dag_ms, dag_noegle, er_faerdig, iso, StiN, Taeller, DOEGN_MS};

/// Grupperne i den rækkefølge, de vises (J1).
pub const AI_TRAENING: &str = "ai-traening";
pub const AI_SOEGNING: &str = "ai-soegning";
pub const SOEGEMASKINER: &str = "soegemaskiner";
pub const SOCIALE: &str = "sociale";
pub const OVERVAAGNING: &str = "overvaagning";
pub const SEO: &str = "seo";
pub const UKENDT: &str = "ukendt";
pub const GRUPPER: &[&str] = &[AI_TRAENING, AI_SOEGNING, SOEGEMASKINER, SOCIALE, SEO, OVERVAAGNING, UKENDT];
const ARTER: &[&str] = &["alle", "bot", "falsk", "stoppet"];
/// Navne, der kun er Cloudflares kategori: de siger ikke, at en bestemt ny robot er kommet.
const UDEN_NAVN: &[&str] = &["AI-assistent i en browser", "Ukendt bot"];
/// Højst så mange linjer om nye robotter i Siden sidst.
const NYE_MAKS: usize = 5;

/// Cloudflares kategori → gruppe (J1). Alt, vi ikke kender, er Ukendt.
pub fn robot_gruppe(kategori: &str) -> &'static str {
    match kategori {
        "AI Crawler" => AI_TRAENING,
        "AI Search" | "AI Assistant" => AI_SOEGNING,
        "Search Engine Crawler" => SOEGEMASKINER,
        "Page Preview" | "Social Media Marketing" | "Feed Fetcher" => SOCIALE,
        "Monitoring & Analytics" | "Security" | "Accessibility" | "Webhooks" => OVERVAAGNING,
        "Search Engine Optimization" => SEO,
        _ => UKENDT,
    }
}

/// Hvad filteret `robot` i dyk peger på. `alle`, et navn, eller en slags (`udklaedt`, `uverificeret`, `stoppet`, `labyrint`) med
/// et muligt `:navn`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Valg<'a> {
    Alle,
    Bot(&'a str),
    Falsk(&'static str, Option<&'a str>),
    Stoppet(Option<&'a str>),
    Labyrint(Option<&'a str>),
}

pub fn valg(v: &str) -> Valg<'_> {
    let (slags, navn) = v.split_once(':').map_or((v, None), |(s, n)| (s, Some(n)));
    match slags {
        "alle" if navn.is_none() => Valg::Alle,
        "udklaedt" => Valg::Falsk("efterligning", navn),
        "uverificeret" => Valg::Falsk("uverificeret", navn),
        "stoppet" => Valg::Stoppet(navn),
        "labyrint" => Valg::Labyrint(navn),
        _ => Valg::Bot(v),
    }
}

/// En række af art `bot`, `falsk` eller `stoppet` i læsbar form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Linje<'a> {
    pub navn: &'a str,
    pub kategori: &'a str,
    pub sti: &'a str,
    pub status: &'a str,
}

/// `Some`, hvis rækken er en robotrække, der tæller som stoppet ved Edge eller i labyrinten; ellers `None`.
fn stoppet_slags(noegle: &str) -> Option<(&str, &'static str)> {
    let mut f = noegle.split('\t');
    let navn = f.next()?;
    let handling = f.nth(2)?;
    match handling_gruppe(handling) {
        BLOKERET | UDFORDRET => Some((navn, "stoppet")),
        LABYRINT => Some((navn, LABYRINT)),
        _ => None,
    }
}

/// Passer rækken til filtrene `robot` og `robotgruppe`? Stien filtreres af kalderen, for kun `bot` har en.
pub fn passer<'a>(r: &'a Raekke, robot: Option<&str>, gruppe: Option<&str>) -> Option<Linje<'a>> {
    let v = robot.map(valg);
    match r.art.as_str() {
        "bot" => {
            let mut f = r.noegle.split('\t');
            let (navn, kategori) = (f.next()?, f.nth(1)?);
            let (sti, status) = (f.next().unwrap_or(""), f.next().unwrap_or(""));
            let ok = match v {
                None | Some(Valg::Alle) => true,
                Some(Valg::Bot(n)) => n == navn,
                _ => false,
            };
            (ok && gruppe.is_none_or(|g| g == robot_gruppe(kategori))).then_some(Linje {
                navn,
                kategori,
                sti,
                status,
            })
        }
        "falsk" => {
            let (navn, slags) = r.noegle.split_once('\t')?;
            let ok = match v {
                Some(Valg::Falsk(s, n)) => s == slags && n.is_none_or(|x| x == navn),
                None => true,
                _ => false,
            };
            (ok && gruppe.is_none_or(|g| g == UKENDT) && (robot.is_some() || gruppe.is_some())).then_some(Linje {
                navn,
                kategori: "",
                sti: "",
                status: "",
            })
        }
        "stoppet" => {
            let (navn, slags) = stoppet_slags(&r.noegle)?;
            let ok = match v {
                Some(Valg::Stoppet(n)) => slags == "stoppet" && n.is_none_or(|x| x == navn),
                Some(Valg::Labyrint(n)) => slags == LABYRINT && n.is_none_or(|x| x == navn),
                _ => false,
            };
            (ok && gruppe.is_none()).then_some(Linje {
                navn,
                kategori: "",
                sti: "",
                status: "",
            })
        }
        _ => None,
    }
}

// ── Port af W ──────────────────────────────────────────────────────────────────────────────────────────────────────
fn er_asset(sti: &str) -> bool {
    const ENDELSER: &[&str] = &[
        ".woff2",
        ".woff",
        ".js",
        ".css",
        ".png",
        ".jpeg",
        ".jpg",
        ".webp",
        ".avif",
        ".gif",
        ".svg",
        ".ico",
        ".webmanifest",
        ".mp3",
        ".pcm",
    ];
    let lille = sti.to_ascii_lowercase();
    sti.starts_with("/assets/") || sti.starts_with("/cf-fonts/") || ENDELSER.iter().any(|e| lille.ends_with(e))
}

/// Hvad en sti er for en slags fil — W `filArt`.
pub fn fil_art(sti: &str) -> &'static str {
    match sti {
        "/robots.txt" => "robots",
        "/sitemap.xml" => "sitemap",
        "/llms.txt" => "llms",
        "/.well-known/security.txt" => "security",
        _ if sti.ends_with("/index.md") => "markdown",
        _ if er_asset(sti) => "assets",
        _ if sti.starts_with("/api/") => "api",
        _ => "sider",
    }
}

/// Felternes navne er W's, så paritetstesten kan sammenligne JSON. "7" er periodens tal her: W's periode er 7 døgn.
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IAlt {
    pub bots7: i64,
    pub bots30: i64,
    pub alle7: i64,
    pub bot_navne: usize,
    pub falske7: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct KatN {
    pub kat: String,
    pub n: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DagKat {
    pub dag: String,
    pub kat: BTreeMap<String, i64>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct NavnN {
    pub navn: String,
    pub n: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct BotUd {
    pub navn: String,
    pub firma: String,
    pub kat: String,
    pub n7: i64,
    pub sider: Vec<StiN>,
    pub status: BTreeMap<String, i64>,
    pub filer: BTreeMap<String, i64>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Crawlere {
    pub genereret: String,
    pub i_alt: IAlt,
    pub kategorier: Vec<KatN>,
    pub dage: Vec<DagKat>,
    pub bots: Vec<BotUd>,
    pub filer: BTreeMap<String, i64>,
    pub status: BTreeMap<String, i64>,
    pub falske: Vec<NavnN>,
    pub uverificerede: Vec<NavnN>,
}

/// Én bot undervejs.
struct Bot {
    navn: String,
    firma: String,
    kat: String,
    n7: i64,
    sider: Taeller,
    status: BTreeMap<String, i64>,
    filer: BTreeMap<String, i64>,
}

fn navne(t: &Taeller, n: usize) -> Vec<NavnN> {
    t.top(n).into_iter().map(|(navn, n)| NavnN { navn, n }).collect()
}

/// `byggCrawlere`, med perioden `fra`…`til` i stedet for W's faste 7 døgn og `dage` som søjlernes dage. Ren funktion.
pub fn bygg(rows: &[Raekke], fra: &str, til: &str, dage: Vec<String>, genereret: String) -> Crawlere {
    let mut dag_kat: HashMap<String, BTreeMap<String, i64>> = dage.iter().map(|d| (d.clone(), BTreeMap::new())).collect();
    let (mut kat, mut falske, mut uverif) = (Taeller::default(), Taeller::default(), Taeller::default());
    let (mut filer, mut status) = (BTreeMap::<String, i64>::new(), BTreeMap::<String, i64>::new());
    let mut bots: Vec<Bot> = Vec::new();
    let mut plads: HashMap<String, usize> = HashMap::new();
    let mut i_alt = IAlt::default();
    for r in rows.iter().filter(|r| r.dag.as_str() <= til) {
        let i_perioden = r.dag.as_str() >= fra;
        if r.art == "alle" {
            if i_perioden {
                i_alt.alle7 += r.n;
            }
            continue;
        }
        if r.art == "falsk" {
            if i_perioden {
                let mut f = r.noegle.split('\t');
                let (navn, slags) = (f.next().unwrap_or(""), f.next().unwrap_or(""));
                if slags == "efterligning" { &mut falske } else { &mut uverif }.inc(navn, r.n);
            }
            continue;
        }
        if r.art != "bot" {
            continue;
        }
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        let (navn, firma, k, sti, st, n) = (felt(0), felt(1), felt(2), felt(3), felt(4), r.n);
        i_alt.bots30 += n;
        if let Some(dk) = dag_kat.get_mut(&r.dag) {
            *dk.entry(k.to_string()).or_default() += n;
        }
        if !i_perioden {
            continue;
        }
        let art = fil_art(sti);
        i_alt.bots7 += n;
        kat.inc(k, n);
        *filer.entry(art.to_string()).or_default() += n;
        *status.entry(st.to_string()).or_default() += n;
        let i = *plads.entry(navn.to_string()).or_insert_with(|| {
            bots.push(Bot {
                navn: navn.to_string(),
                firma: firma.to_string(),
                kat: k.to_string(),
                n7: 0,
                sider: Taeller::default(),
                status: BTreeMap::new(),
                filer: BTreeMap::new(),
            });
            bots.len() - 1
        });
        let b = &mut bots[i];
        b.n7 += n;
        if art == "sider" {
            b.sider.inc(sti, n);
        }
        *b.status.entry(st.to_string()).or_default() += n;
        *b.filer.entry(art.to_string()).or_default() += n;
    }
    i_alt.bot_navne = bots.len();
    i_alt.falske7 = falske.orden.iter().map(|x| x.1).sum();
    bots.sort_by_key(|b| std::cmp::Reverse(b.n7));
    bots.truncate(24);
    Crawlere {
        genereret,
        i_alt,
        kategorier: kat.top(20).into_iter().map(|(kat, n)| KatN { kat, n }).collect(),
        dage: dage
            .into_iter()
            .map(|dag| {
                let kat = dag_kat.remove(&dag).unwrap_or_default();
                DagKat { dag, kat }
            })
            .collect(),
        bots: bots
            .into_iter()
            .map(|b| BotUd {
                sider: b.sider.top(3).into_iter().map(|(sti, n)| StiN { sti, n }).collect(),
                navn: b.navn,
                firma: b.firma,
                kat: b.kat,
                n7: b.n7,
                status: b.status,
                filer: b.filer,
            })
            .collect(),
        filer,
        status,
        falske: navne(&falske, 10),
        uverificerede: navne(&uverif, 10),
    }
}

/// W `byggCrawlere(rows, nu)`: de seneste 7 døgn inkl. i dag, med søjler for 30.
#[cfg(test)]
pub fn bygg_crawlere(rows: &[Raekke], nu: i64) -> Crawlere {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    bygg(rows, &d(6), &d(0), (0..crate::vaern::DAGE).rev().map(d).collect(), iso(nu))
}

// ── Visningen ──────────────────────────────────────────────────────────────────────────────────────────────────────
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GruppeUd {
    pub id: &'static str,
    pub n: i64,
}

/// Det, de ekstra tal siger om stoppede robotter.
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Stoppet {
    pub sum: i64,
    pub labyrint: i64,
    pub robotter: Vec<VaerdiN>,
    pub labyrint_robotter: Vec<VaerdiN>,
    /// Dage i perioden, hvor de ekstra tal endnu ikke er hentet for mindst ét domæne.
    pub mangler_dage: usize,
    /// Domæner, Cloudflare ikke gav de ekstra tal for.
    pub ikke_tilgaengelig: Vec<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct RobotterUd {
    pub periode: PeriodeUd,
    pub crawlere: Crawlere,
    pub grupper: Vec<GruppeUd>,
    pub sider: Vec<VaerdiN>,
    pub antal_sider: usize,
    pub stoppet: Stoppet,
}

/// Det, `regn` skal bruge fra databasen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Laest {
    pub rows: Vec<Raekke>,
    pub hentet: HashSet<String>,
    pub ekstra: HashMap<String, HashSet<String>>,
    pub ikke_tilgaengelig: Vec<String>,
}

/// Domæner uden ekstra tal ved seneste forsøg, med navn (id'et, hvis det ikke står i zonelisten).
pub fn uden_ekstra(lager: &Store, zoner: &[String]) -> Result<Vec<String>, Fejl> {
    let navne: HashMap<String, String> = lager.zoner()?.0.into_iter().map(|z: Zone| (z.id, z.navn)).collect();
    let mut ud = Vec::new();
    for z in zoner {
        if lager.adgang_fejl(z)?.is_some() {
            ud.push(navne.get(z).cloned().unwrap_or_else(|| "…".into()));
        }
    }
    Ok(ud)
}

pub fn laes(lager: &Store, s: &Spoergsmaal, _nu: i64) -> Result<Laest, Fejl> {
    let mut hentet = HashSet::new();
    let mut ekstra = HashMap::new();
    for z in &s.zoner {
        hentet.extend(lager.hentet(z)?.into_keys());
        ekstra.insert(z.clone(), lager.hentet_ekstra(z)?.into_keys().collect());
    }
    Ok(Laest {
        rows: lager
            .raekker_art(&s.zoner, ARTER, "0000-00-00")?
            .into_iter()
            .map(|(_, r)| r)
            .collect(),
        hentet,
        ekstra,
        ikke_tilgaengelig: uden_ekstra(lager, &s.zoner)?,
    })
}

/// Dagene `fra`…`til`.
pub fn dage_i(fra: &str, til: &str) -> Vec<String> {
    let (a, b) = (dag_ms(fra).unwrap_or(0), dag_ms(til).unwrap_or(0));
    (0..=(b - a).max(0) / DOEGN_MS).map(|i| dag_noegle(a + i * DOEGN_MS)).collect()
}

/// Dage i perioden, hvor mindst ét domæne stadig venter på de ekstra tal. Kun døgn, køen kan hente: færdige døgn i hentevinduet.
/// Ældre døgn har Cloudflare ikke længere, så dem venter ingen på.
pub fn mangler_dage(dage: &[String], ekstra: &HashMap<String, HashSet<String>>, nu: i64) -> usize {
    let vindue = vinduet(nu);
    dage.iter()
        .filter(|d| vindue.contains(*d) && er_faerdig(d, &iso(nu)) && ekstra.values().any(|h| !h.contains(*d)))
        .count()
}

/// Svaret ud fra det læste. Ren funktion.
pub fn regn(periode: &Periode, l: &Laest, nu: i64) -> RobotterUd {
    let (fra, til) = spaend(periode, &l.hentet, nu);
    let dage = dage_i(&fra, &til);
    let i_perioden = |r: &&Raekke| r.dag >= fra && r.dag <= til;
    let (mut grupper, mut sider) = (HashMap::<&str, i64>::new(), HashMap::<&str, i64>::new());
    let (mut stoppet, mut labyrint) = (HashMap::<&str, i64>::new(), HashMap::<&str, i64>::new());
    for r in l.rows.iter().filter(i_perioden) {
        match r.art.as_str() {
            "bot" => {
                let Some(x) = passer(r, None, None) else { continue };
                *grupper.entry(robot_gruppe(x.kategori)).or_default() += r.n;
                if fil_art(x.sti) != "assets" {
                    *sider.entry(x.sti).or_default() += r.n;
                }
            }
            "falsk" => *grupper.entry(UKENDT).or_default() += r.n,
            "stoppet" => {
                if let Some((navn, slags)) = stoppet_slags(&r.noegle) {
                    *if slags == LABYRINT { &mut labyrint } else { &mut stoppet }
                        .entry(navn)
                        .or_default() += r.n;
                }
            }
            _ => {}
        }
    }
    let sum = |t: &HashMap<&str, i64>| t.values().sum();
    RobotterUd {
        periode: periode_ud(&fra, &til, &l.hentet, nu),
        grupper: GRUPPER
            .iter()
            .filter_map(|g| grupper.get(g).map(|n| GruppeUd { id: g, n: *n }))
            .collect(),
        antal_sider: sider.len(),
        sider: top_af(&sider),
        stoppet: Stoppet {
            sum: sum(&stoppet),
            labyrint: sum(&labyrint),
            robotter: top_af(&stoppet),
            labyrint_robotter: top_af(&labyrint),
            mangler_dage: mangler_dage(&dage, &l.ekstra, nu),
            ikke_tilgaengelig: l.ikke_tilgaengelig.clone(),
        },
        crawlere: bygg(&l.rows, &fra, &til, dage, iso(nu)),
    }
}

/// En AI-robot, der er dukket op på et domæne.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct NyRobot {
    pub zone: String,
    pub domaene: String,
    pub robot: String,
    pub foerst: String,
}

/// AI-robotter set første gang fra og med `fra`, på et domæne med tal fra før det: uden den betingelse ville alt være nyt, første
/// gang domænet hentes. Kategorinavne tæller ikke. Ren funktion.
pub fn nye_ai(rows: &[Raekke], fra: &str, foerste_hentet: &str) -> Vec<(String, String)> {
    let mut foerst: BTreeMap<&str, &str> = BTreeMap::new();
    for r in rows {
        let Some(x) = passer(r, None, None).filter(|_| r.art == "bot") else {
            continue;
        };
        if matches!(robot_gruppe(x.kategori), AI_TRAENING | AI_SOEGNING)
            && !UDEN_NAVN.contains(&x.navn)
            && !x.navn.starts_with("Verificeret (")
        {
            let d = foerst.entry(x.navn).or_insert(r.dag.as_str());
            *d = r.dag.as_str().min(*d);
        }
    }
    foerst
        .into_iter()
        .filter(|(_, d)| *d >= fra && *d > foerste_hentet)
        .map(|(n, d)| (n.to_string(), d.to_string()))
        .collect()
}

/// Nye AI-robotter siden kortet sidst blev vist (ellers de seneste 7 døgn). Kun hele døgn, som resten af Siden sidst: en robot, der først er set i dag, kommer i morgen, så klikket
/// fører til tal og ikke til 0. Højst `NYE_MAKS`, nyeste først.
pub fn nye_robotter(lager: &Store, vist: Option<&str>, nu: i64) -> Result<Vec<NyRobot>, Fejl> {
    let fra = vist.map_or_else(|| dag_noegle(nu - 7 * DOEGN_MS), |v| v.chars().take(10).collect());
    let i_dag = dag_noegle(nu);
    let mut ud = Vec::new();
    for z in lager.zoner_til()? {
        let Some(foerste) = lager.hentet(&z.id)?.into_keys().min() else {
            continue;
        };
        let rows: Vec<Raekke> = lager
            .raekker_art(std::slice::from_ref(&z.id), &["bot"], "0000-00-00")?
            .into_iter()
            .map(|(_, r)| r)
            .filter(|r| r.dag < i_dag)
            .collect();
        ud.extend(nye_ai(&rows, &fra, &foerste).into_iter().map(|(robot, foerst)| NyRobot {
            zone: z.id.clone(),
            domaene: z.navn.clone(),
            robot,
            foerst,
        }));
    }
    ud.sort_by(|a, b| {
        b.foerst
            .cmp(&a.foerst)
            .then_with(|| (&a.domaene, &a.robot).cmp(&(&b.domaene, &b.robot)))
    });
    ud.truncate(NYE_MAKS);
    Ok(ud)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dyk::{regn as dyk_regn, tjek, Dim, Filter, Laest as DykLaest, Omfang};
    use serde_json::{json, Value};
    use std::sync::LazyLock;

    /// 2026-11-04T08:00:00Z.
    const NU: i64 = 1_793_779_200_000;

    static ORAKEL: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(include_str!("robotter/paritet.json")).unwrap());

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

    fn bot(dag: &str, navn: &str, kat: &str, sti: &str, n: i64) -> Raekke {
        rk(dag, "bot", &[navn, "", kat, sti, "200"].join("\t"), n)
    }

    #[test]
    fn paritet_fil_art() {
        for p in ORAKEL["filArt"].as_array().unwrap() {
            assert_eq!(fil_art(p[0].as_str().unwrap()), p[1].as_str().unwrap(), "filArt({})", p[0]);
        }
        assert!(ORAKEL["filArt"].as_array().unwrap().len() > 20);
    }

    #[test]
    fn paritet_byggcrawlere() {
        let rows: Vec<Raekke> = ORAKEL["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                rk(
                    r["dag"].as_str().unwrap(),
                    r["art"].as_str().unwrap(),
                    r["noegle"].as_str().unwrap(),
                    r["n"].as_i64().unwrap(),
                )
            })
            .collect();
        assert!(rows.len() > 200, "oraklet dækker mange rækker ({})", rows.len());
        // Oraklets NU: 2026-10-29T05:00:00Z.
        let nu = dag_ms("2026-10-29").unwrap() + 5 * 3_600_000;
        let c = bygg_crawlere(&rows, nu);
        let mut mig = serde_json::to_value(&c).unwrap();
        let mut w = ORAKEL["byggCrawlere"].clone();
        // W skriver `genereret` med Date#toISOString: det samme format som iso().
        assert_eq!(mig["genereret"], w["genereret"]);
        for v in [&mut mig, &mut w] {
            v.as_object_mut().unwrap().remove("genereret");
        }
        assert_eq!(mig, w, "byggCrawlere");
    }

    #[test]
    fn grupper_og_ukendte_kategorier() {
        for (k, g) in [
            ("AI Crawler", AI_TRAENING),
            ("AI Search", AI_SOEGNING),
            ("AI Assistant", AI_SOEGNING),
            ("Search Engine Crawler", SOEGEMASKINER),
            ("Page Preview", SOCIALE),
            ("Social Media Marketing", SOCIALE),
            ("Feed Fetcher", SOCIALE),
            ("Monitoring & Analytics", OVERVAAGNING),
            ("Security", OVERVAAGNING),
            ("Accessibility", OVERVAAGNING),
            ("Webhooks", OVERVAAGNING),
            ("Search Engine Optimization", SEO),
        ] {
            assert_eq!(robot_gruppe(k), g, "{k}");
        }
        for k in ["Agent", "Training", "Search", "Noget helt nyt", ""] {
            assert_eq!(robot_gruppe(k), UKENDT, "ny kategori er ikke en fejl: {k}");
        }
        assert_eq!(GRUPPER.len(), 7, "J1");
    }

    fn laest() -> Laest {
        Laest {
            rows: vec![
                bot(&d(1), "GPTBot", "AI Crawler", "/", 5),
                bot(&d(2), "GPTBot", "AI Crawler", "/a.css", 2),
                bot(&d(2), "Googlebot", "Search Engine Crawler", "/", 3),
                bot(&d(2), "Ny", "Agent", "/x", 1),
                bot(&d(20), "GPTBot", "AI Crawler", "/gammel", 100),
                bot(&d(0), "GPTBot", "AI Crawler", "/i-dag", 50),
                rk(&d(1), "falsk", "Googlebot\tefterligning", 7),
                rk(&d(1), "falsk", "FooBot\tuverificeret", 4),
                rk(&d(1), "alle", "", 1000),
                rk(&d(1), "stoppet", "GPTBot\tOpenAI\tbotFight\tmanaged_challenge", 6),
                rk(&d(1), "stoppet", "GPTBot\tOpenAI\tfirewallManaged\tblock", 2),
                rk(&d(1), "stoppet", "ClaudeBot\tAnthropic\tlinkMaze\tlink_maze_visited", 9),
                rk(&d(1), "stoppet", "ClaudeBot\tAnthropic\tlinkMaze\tlink_maze_injected", 90),
            ],
            hentet: (0..=31).map(d).collect(),
            ekstra: HashMap::from([("z1".to_string(), (1..=6).map(d).collect())]),
            ikke_tilgaengelig: vec![],
        }
    }

    #[test]
    fn perioden_grupperne_og_labyrinten_for_sig() {
        let u = regn(&Periode::D7, &laest(), NU);
        assert_eq!(u.crawlere.i_alt.bots7, 11, "hele døgn: i dag og d-20 er ikke med");
        assert_eq!(u.crawlere.i_alt.alle7, 1000);
        assert_eq!(u.crawlere.i_alt.falske7, 7);
        let g: Vec<(&str, i64)> = u.grupper.iter().map(|x| (x.id, x.n)).collect();
        assert_eq!(
            g,
            [(AI_TRAENING, 7), (SOEGEMASKINER, 3), (UKENDT, 1 + 7 + 4)],
            "falsk og ny kategori i Ukendt"
        );
        assert_eq!(u.stoppet.sum, 8, "blokeret og udfordret");
        assert_eq!(u.stoppet.labyrint, 9, "labyrinten står for sig, sider med fælde-links tæller ikke");
        assert_eq!(u.stoppet.robotter[0].v, "GPTBot");
        assert_eq!(u.stoppet.labyrint_robotter[0].v, "ClaudeBot");
        assert_eq!(u.stoppet.mangler_dage, 1, "d-7 er ikke hentet endnu");
        assert_eq!(
            u.sider.iter().map(|x| (x.v.as_str(), x.n)).collect::<Vec<_>>(),
            [("/", 8), ("/x", 1)],
            "uden assets"
        );
        let u = regn(&Periode::Alt, &laest(), NU);
        assert_eq!(u.crawlere.i_alt.bots7, 111);
    }

    #[test]
    fn kun_doegn_koeen_kan_hente_mangler() {
        let ekstra = HashMap::from([("z1".to_string(), HashSet::from([d(2)]))]);
        let dage = |a: i64, b: i64| dage_i(&d(a), &d(b));
        assert_eq!(mangler_dage(&dage(3, 1), &ekstra, NU), 2, "d-3 og d-1");
        assert_eq!(mangler_dage(&dage(40, 31), &ekstra, NU), 0, "Cloudflare har dem ikke længere");
        assert_eq!(mangler_dage(&dage(0, 0), &ekstra, NU), 0, "i dag hentes ikke");
        let lige_efter = dag_ms(&d(0)).unwrap() + 1_800_000; // 00:30: i går er ikke færdigt endnu
        assert_eq!(mangler_dage(&dage(1, 1), &ekstra, lige_efter), 0);
    }

    /// Tallet på Robotter er tallet i dyk.
    #[test]
    fn tallene_er_ens_i_robotter_og_dyk() {
        let l = laest();
        let rows: Vec<(String, Raekke)> = l.rows.iter().map(|r| ("z1".to_string(), r.clone())).collect();
        let dl = DykLaest {
            rows,
            hentet: l.hentet.clone(),
            ..Default::default()
        };
        let dyk = |dim: Dim, v: &str| {
            let s = tjek(Omfang::Zone("z1".into()), vec![Filter { dim, vaerdi: v.into() }], Periode::D7, None).unwrap();
            dyk_regn(&s, &dl, NU).sum
        };
        let u = regn(&Periode::D7, &l, NU);
        assert_eq!(dyk(Dim::Robot, "alle"), u.crawlere.i_alt.bots7);
        assert_eq!(dyk(Dim::Robot, "udklaedt"), u.crawlere.i_alt.falske7);
        assert_eq!(dyk(Dim::Robot, "stoppet"), u.stoppet.sum);
        assert_eq!(dyk(Dim::Robot, "labyrint"), u.stoppet.labyrint);
        assert_eq!(dyk(Dim::Robot, "stoppet:GPTBot"), u.stoppet.robotter[0].n);
        assert_eq!(dyk(Dim::Robot, "labyrint:ClaudeBot"), u.stoppet.labyrint_robotter[0].n);
        for b in &u.crawlere.bots {
            assert_eq!(dyk(Dim::Robot, &b.navn), b.n7, "{}", b.navn);
        }
        for g in &u.grupper {
            assert_eq!(dyk(Dim::RobotGruppe, g.id), g.n, "{}", g.id);
        }
    }

    #[test]
    fn valg_af_robot() {
        assert_eq!(valg("alle"), Valg::Alle);
        assert_eq!(valg("GPTBot"), Valg::Bot("GPTBot"));
        assert_eq!(valg("udklaedt"), Valg::Falsk("efterligning", None));
        assert_eq!(valg("uverificeret:FooBot"), Valg::Falsk("uverificeret", Some("FooBot")));
        assert_eq!(valg("stoppet:GPTBot"), Valg::Stoppet(Some("GPTBot")));
        assert_eq!(valg("labyrint"), Valg::Labyrint(None));
        assert_eq!(valg("Verificeret (AI Crawler)"), Valg::Bot("Verificeret (AI Crawler)"));
    }

    #[test]
    fn nye_ai_robotter_kun_med_historik_og_kun_med_navn() {
        let rows = [
            bot(&d(1), "NyBot", "AI Crawler", "/", 1),
            bot(&d(2), "NyBot", "AI Crawler", "/", 1),
            bot(&d(20), "GammelBot", "AI Search", "/", 1),
            bot(&d(1), "GammelBot", "AI Search", "/", 1),
            bot(&d(1), "Googlebot", "Search Engine Crawler", "/", 1),
            bot(&d(1), "AI-assistent i en browser", "AI Assistant", "/", 1),
            bot(&d(1), "Verificeret (AI Crawler)", "AI Crawler", "/", 1),
            rk(&d(1), "falsk", "FalskBot\tuverificeret", 1),
        ];
        assert_eq!(nye_ai(&rows, &d(3), &d(25)), [("NyBot".to_string(), d(2))]);
        assert!(nye_ai(&rows, &d(3), &d(2)).is_empty(), "første hentning: intet er nyt");
        assert!(nye_ai(&rows, &d(1), &d(25)).is_empty(), "set før sidste visning");
    }

    /// D2: en ny robot på et domæne, der er slået fra, står ikke i Siden sidst.
    #[test]
    fn nye_robotter_uden_de_fravalgte() {
        let z = |id: &str| Zone {
            id: id.into(),
            navn: format!("{id}.wi24rd.com"),
        };
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(&[z("a"), z("b")], "t").unwrap();
        for id in ["a", "b"] {
            s.gem_doegn(id, &d(25), &[], "t").unwrap();
            s.gem_doegn(id, &d(1), &[("bot".to_string(), "NyBot\t\tAI Crawler\t/\t200".to_string(), 3)], "t")
                .unwrap();
        }
        s.gem_valgte(&["a".into(), "b".into()]).unwrap();
        assert_eq!(nye_robotter(&s, None, NU).unwrap().len(), 2);
        s.gem_valgte(&["a".into()]).unwrap();
        let ud = nye_robotter(&s, None, NU).unwrap();
        assert_eq!(ud.iter().map(|r| r.zone.as_str()).collect::<Vec<_>>(), ["a"]);
    }

    /// Siden sidst bygger på hele døgn. En robot, der først er set i dag, kommer i morgen, så klikket aldrig fører til 0.
    #[test]
    fn ny_robot_kun_paa_hele_doegn() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(
            &[Zone {
                id: "z1".into(),
                navn: "a.wi24rd.com".into(),
            }],
            "t",
        )
        .unwrap();
        let bot = || vec![("bot".to_string(), "NyBot\t\tAI Crawler\t/\t200".to_string(), 1)];
        s.gem_doegn("z1", &d(25), &[("alle".into(), String::new(), 1)], "t").unwrap();
        s.gem_doegn("z1", &d(0), &bot(), "t").unwrap();
        assert_eq!(nye_robotter(&s, None, NU).unwrap(), [], "kun set i dag");
        s.gem_doegn("z1", &d(1), &bot(), "t").unwrap();
        assert_eq!(nye_robotter(&s, None, NU).unwrap().len(), 1, "set på et helt døgn");
    }

    #[test]
    fn nye_robotter_fra_lageret_efter_sidste_visning() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(
            &[Zone {
                id: "z1".into(),
                navn: "a.wi24rd.com".into(),
            }],
            "t",
        )
        .unwrap();
        s.gem_doegn("z1", &d(25), &[("alle".into(), String::new(), 1)], "t").unwrap();
        s.gem_doegn("z1", &d(1), &[("bot".into(), "NyBot\t\tAI Crawler\t/\t200".into(), 3)], "t")
            .unwrap();
        let ny = nye_robotter(&s, None, NU).unwrap();
        assert_eq!(
            (ny.len(), ny[0].robot.as_str(), ny[0].domaene.as_str()),
            (1, "NyBot", "a.wi24rd.com")
        );
        assert_eq!(nye_robotter(&s, Some(&iso(NU)), NU).unwrap(), [], "allerede vist");
    }

    #[test]
    fn json_formen_har_w_navne() {
        let v = serde_json::to_value(regn(&Periode::D7, &laest(), NU)).unwrap();
        assert_eq!(v["crawlere"]["iAlt"]["botNavne"], json!(3));
    }
}
