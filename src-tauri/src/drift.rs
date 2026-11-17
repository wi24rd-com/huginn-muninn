//! V-DRIFT.1 B1: fejl pr. dag og time, fejl pr. subdomæne og cache, regnet fra cachen til kommandoen `drift` (fanen Drift kommer i B2). Tallene hentes som
//! arterne `adgang::DRIFT_ARTER` (de ekstra tal, `hentet_drift`) og `vaert::V_FEJL` (trafikken pr. vært, `hentet_vaert` fra `store::VAERT_5XX_V1`).
//!
//! Reglerne fra V-SUBDOM.1: hellere intet tal end et for lavt (S4), og "0 betyder hentet og nul; ikke hentet er aldrig 0" (`None`). Intet her ændrer
//! Overblik, notifikationerne eller `overblik::server_fejl`: de læser ingen af arterne (`Store::raekker` udelader dem). Svaret har ingen id'er og intet
//! værtsnavn ud over den label, visningen selv har valgt.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::RangeInclusive;

use serde::Serialize;

use crate::adgang::DRIFT_ARTER;
use crate::cf::Fejl;
use crate::overblik::SERVER_KODER;
use crate::robotter::dage_i;
use crate::store::{Raekke, Store};
use crate::vaern::{dag_ms, dag_noegle, DOEGN_MS};
use crate::vaert::{FEJL_ARTER, V_AFSKAARET, V_FEJL, V_FEJL_AFSKAARET, V_FEJL_AFVIST};

/// "Kunne ikke nås": Cloudflare fik intet brugbart svar fra serveren (INVESTIGATION Q2). 530 tælles for sig i svaret (`kunne_ikke_530`); fanen lægger
/// de to sammen, og reglen for en time med fejl (`overblik::SERVER_KODER`) har begge med (B2, D4).
pub const KUNNE_IKKE: RangeInclusive<i64> = 520..=527;
pub const KODE_530: i64 = 530;
/// "Serveren svarede med en fejl": origin 500–599. Ændrer aldrig status (D5).
pub const SERVER_FEJL: RangeInclusive<i64> = 500..=599;
/// En time med fejl: mindst så mange svar med `overblik::SERVER_KODER` (520–527 og 530, samme regel som "Din server svarer ikke") og mindst så mange
/// procent af timens svar (INVESTIGATION Q2).
pub const TIME_MIN_SVAR: i64 = 5;
pub const TIME_PROCENT: i64 = 20;
/// Så mange timer i træk er et spænd ("mellem kl. A og B").
pub const SPAEND_MIN: usize = 3;
const TIME_MS: i64 = 3_600_000;

// ── Svaret ─────────────────────────────────────────────────────────────────────────────────────────────────────────────
/// Svar og data i de tre grupper fra INVESTIGATION Q3, plus `andet` for en cache-status, Cloudflare ikke har dokumenteret, så grupperne altid summer.
#[derive(Serialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Grupper {
    /// Fra Cloudflares lager.
    pub lager: i64,
    /// Svaret af Cloudflare selv (`none`, `unknown`).
    pub selv: i64,
    /// Hentet hos din server.
    pub server: i64,
    pub andet: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct CacheStatus {
    pub status: String,
    pub svar: i64,
    pub bytes: i64,
    /// Gruppen for forespørgsler og for data: "lager", "selv", "server" eller "andet" (B2: fanens fliser filtrerer tabellen på dem).
    pub gruppe: &'static str,
    pub gruppe_data: &'static str,
}

#[derive(Serialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheUd {
    /// Pr. cache-status, flest svar først.
    pub status: Vec<CacheStatus>,
    /// Forespørgsler: `revalidated` er hentet hos serveren. Data: `revalidated` er fra lageret (Cloudflares Cache Analytics).
    pub svar: Grupper,
    pub data: Grupper,
    /// `stale`: gamle kopier vist, fordi serveren ikke svarede. Også med i `svar.lager`.
    pub stale: i64,
}

/// Én dag for domænet (eller de valgte tilsammen). `None`: ikke hentet (eller afskåret for den art), aldrig 0.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DriftDag {
    pub dag: String,
    /// Alle domæner i omfanget har dagen i `hentet_drift`.
    pub hentet: bool,
    pub svar: Option<i64>,
    /// Kant 520–527.
    pub kunne_ikke: Option<i64>,
    /// Kant 530, talt for sig.
    pub kunne_ikke_530: Option<i64>,
    /// Origin 500–599.
    pub server_fejl: Option<i64>,
    /// B2: alle svar med kant eller origin ≥ 500 (hvert svar én gang), til grafen pr. dag.
    pub fejl: Option<i64>,
    /// Svarede forsiden (`/`) med kant ≥ 500 mindst én gang?
    pub forside_fejl: Option<bool>,
    /// Timer (UTC, "HH") med fejl efter `TIME_MIN_SVAR`/`TIME_PROCENT` for mindst ét domæne.
    pub timer_med_fejl: Vec<String>,
    pub cache: Option<CacheUd>,
    /// Arter, der stod på loftet dette døgn (`d_time`, `d_fejl`, `d_forside`, `d_cache`).
    pub afskaaret: Vec<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct KodePar {
    pub kant: i64,
    pub origin: i64,
    pub svar: i64,
}

/// Én time (UTC) for en valgt dag (`fra` = `til`).
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TimeUd {
    pub time: String,
    pub svar: Option<i64>,
    /// Svar med kant eller origin ≥ 500 pr. kode-par.
    pub fejl: Vec<KodePar>,
    pub forside: Option<i64>,
    pub med_fejl: bool,
}

/// Mindst `SPAEND_MIN` timer i træk med fejl for ét domæne (UTC, "YYYY-MM-DDTHH", begge med). Kan gå hen over midnat.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Spaend {
    pub fra: String,
    pub til: String,
    pub timer: usize,
    /// Fejlede forsiden også i spændet? Kun så siger B2 "Dit website svarede ikke".
    pub forside_fejlede: bool,
}

/// Fejl for én vært (`v_fejl`, kun kant-koder; ingen timer og ingen origin pr. vært).
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct VaertDag {
    pub dag: String,
    pub hentet: bool,
    pub kunne_ikke: Option<i64>,
    pub kunne_ikke_530: Option<i64>,
    /// Andre svar med kant ≥ 500.
    pub andre_5xx: Option<i64>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct VaertUd {
    /// Labelen under domænet (`""` = domænet selv).
    pub vaert: String,
    pub dage: Vec<VaertDag>,
    /// Den første dag med fejl pr. vært.
    pub fra: Option<String>,
    pub afskaaret: Vec<String>,
    pub afvist: Vec<String>,
    /// Cloudflare har afvist aliaset i denne kørsel.
    pub afvist_nu: bool,
}

/// Svaret til B2. Ingen zone-id, intet konto-id.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DriftUd {
    pub fra: String,
    pub til: String,
    pub dage: Vec<DriftDag>,
    /// Kun når perioden er én dag.
    pub timer: Vec<TimeUd>,
    pub spaend: Vec<Spaend>,
    /// Cachen for perioden (kun dage med cachetal). `None`: ingen.
    pub cache: Option<CacheUd>,
    /// Cachen er domænets: med en vært valgt gælder den stadig hele domænet, aldrig værten.
    pub cache_hele_domaenet: bool,
    /// Den første dag med fejl- og cachetal.
    pub foerste_dag: Option<String>,
    /// Dage i perioden, hvor en art stod på loftet.
    pub afskaaret: Vec<String>,
    /// Cloudflare afviste aliaserne for et domæne i omfanget i denne kørsel.
    pub afvist: bool,
    pub vaert: Option<VaertUd>,
}

// ── Det, der læses ─────────────────────────────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Laest {
    pub zoner: Vec<String>,
    /// `DRIFT_ARTER` fra og med periodens første dag, med zonen.
    pub raekker: Vec<(String, Raekke)>,
    /// zone → døgn i `hentet_drift`.
    pub hentet: HashMap<String, HashSet<String>>,
    /// (zone, dag) med `d_udenfor` (hele historikken): ikke hentet.
    pub udenfor: HashSet<(String, String)>,
    pub vaert: Option<String>,
    /// `vaert::FEJL_ARTER` for domænet fra og med periodens første dag.
    pub vaert_raekker: Vec<Raekke>,
    pub vaert_hentet: HashSet<String>,
    /// `store::VAERT_5XX_V1`.
    pub vaert_fra: Option<String>,
    /// Sat af kommandoen.
    pub afvist: bool,
    pub vaert_afvist_nu: bool,
}

pub fn laes(lager: &Store, zoner: &[String], fra: &str, vaert: Option<&str>) -> Result<Laest, Fejl> {
    let mut l = Laest {
        zoner: zoner.to_vec(),
        raekker: lager.raekker_art(zoner, DRIFT_ARTER, fra)?,
        udenfor: lager
            .raekker_art(zoner, &["d_udenfor"], "0000-00-00")?
            .into_iter()
            .map(|(z, r)| (z, r.dag))
            .collect(),
        vaert: vaert.map(str::to_string),
        ..Laest::default()
    };
    for z in zoner {
        l.hentet.insert(z.clone(), lager.hentet_drift(z)?.into_keys().collect());
    }
    // B2: også uden en valgt vært, så `regn_fejl` kan liste værterne med fejl. `regn` bruger dem kun med en valgt vært.
    if let [z] = zoner {
        l.vaert_raekker = lager
            .raekker_art(std::slice::from_ref(z), FEJL_ARTER, fra)?
            .into_iter()
            .map(|(_, r)| r)
            .collect();
        l.vaert_hentet = lager.hentet_vaert(z)?.into_keys().collect();
        l.vaert_fra = lager.vaert_5xx_fra()?;
    }
    Ok(l)
}

// ── Regningen ──────────────────────────────────────────────────────────────────────────────────────────────────────────
/// Ét domænes ene døgn.
#[derive(Default)]
struct Zd {
    timer: [i64; 24],
    /// (time, kant, origin) → svar.
    fejl: BTreeMap<(usize, i64, i64), i64>,
    forside: [i64; 24],
    cache: BTreeMap<String, (i64, i64)>,
    afsk: BTreeSet<String>,
}

impl Zd {
    fn har(&self, art: &str) -> bool {
        !self.afsk.contains(art)
    }
    fn fejl_sum(&self, p: impl Fn(i64, i64) -> bool) -> i64 {
        self.fejl.iter().filter(|((_, k, o), _)| p(*k, *o)).map(|(_, n)| n).sum()
    }
    fn kunne_ikke_time(&self, t: usize) -> i64 {
        self.fejl
            .iter()
            .filter(|((h, k, _), _)| *h == t && SERVER_KODER.contains(k))
            .map(|(_, n)| n)
            .sum()
    }
    /// Timens regel: kun når både svar og fejl er hentet.
    fn time_med_fejl(&self, t: usize) -> bool {
        if !self.har("d_time") || !self.har("d_fejl") {
            return false;
        }
        let k = self.kunne_ikke_time(t);
        k >= TIME_MIN_SVAR && k * 100 >= TIME_PROCENT * self.timer[t]
    }
}

fn time_nr(hh: &str) -> Option<usize> {
    hh.parse::<usize>().ok().filter(|t| *t < 24)
}

/// Gruppen for forespørgsler og for data (Cloudflares Cache Analytics; `revalidated` skifter side). `unknown` er dokumenteret sammen med `none`.
fn grupper(status: &str) -> (usize, usize) {
    match status {
        "hit" | "stale" | "updating" => (0, 0),
        "revalidated" => (2, 0),
        "none" | "unknown" => (1, 1),
        "miss" | "expired" | "bypass" | "dynamic" => (2, 2),
        _ => (3, 3),
    }
}

const GRUPPE_NAVNE: [&str; 4] = ["lager", "selv", "server", "andet"];

fn laeg_til(g: &mut Grupper, i: usize, n: i64) {
    match i {
        0 => g.lager += n,
        1 => g.selv += n,
        2 => g.server += n,
        _ => g.andet += n,
    }
}

fn cache_ud(m: &BTreeMap<String, (i64, i64)>) -> CacheUd {
    let mut c = CacheUd::default();
    for (s, (n, b)) in m {
        let (gs, gd) = grupper(s);
        laeg_til(&mut c.svar, gs, *n);
        laeg_til(&mut c.data, gd, *b);
        if s == "stale" {
            c.stale += n;
        }
        c.status.push(CacheStatus {
            status: s.clone(),
            svar: *n,
            bytes: *b,
            gruppe: GRUPPE_NAVNE[gs],
            gruppe_data: GRUPPE_NAVNE[gd],
        });
    }
    c.status.sort_by(|a, b| b.svar.cmp(&a.svar).then_with(|| a.status.cmp(&b.status)));
    c
}

/// Spændene (≥ `SPAEND_MIN` timer i træk) i de timer `t` (ms for timens start), med forsiden. Ren funktion.
fn spaend_af(timer: &BTreeMap<i64, bool>) -> Vec<Spaend> {
    let mut ud = Vec::new();
    let mut loeb: Vec<(i64, bool)> = Vec::new();
    let mut slut = |loeb: &mut Vec<(i64, bool)>| {
        if loeb.len() >= SPAEND_MIN {
            let tekst = |ms: i64| format!("{}T{:02}", dag_noegle(ms), (ms.rem_euclid(DOEGN_MS)) / TIME_MS);
            ud.push(Spaend {
                fra: tekst(loeb[0].0),
                til: tekst(loeb[loeb.len() - 1].0),
                timer: loeb.len(),
                forside_fejlede: loeb.iter().any(|x| x.1),
            });
        }
        loeb.clear();
    };
    for (&t, &forside) in timer {
        if loeb.last().is_some_and(|x| x.0 + TIME_MS != t) {
            slut(&mut loeb);
        }
        loeb.push((t, forside));
    }
    slut(&mut loeb);
    ud
}

/// Døgnet er noteret i `hentet_drift` og ikke mærket uden for Cloudflares opbevaring.
fn er_hentet(l: &Laest, z: &str, d: &str) -> bool {
    l.hentet.get(z).is_some_and(|h| h.contains(d)) && !l.udenfor.contains(&(z.to_string(), d.to_string()))
}

/// (zone, dag) → døgnets tal for `fra`…`til`.
fn doegn<'a>(l: &'a Laest, fra: &str, til: &str) -> HashMap<(&'a str, &'a str), Zd> {
    let mut zd: HashMap<(&str, &str), Zd> = HashMap::new();
    for (z, r) in l.raekker.iter().filter(|(_, r)| r.dag.as_str() >= fra && r.dag.as_str() <= til) {
        let d = zd.entry((z.as_str(), r.dag.as_str())).or_default();
        let mut f = r.noegle.split('\t');
        let t = f.next().and_then(time_nr);
        let mut tal = || f.next().and_then(|x| x.parse::<i64>().ok());
        match r.art.as_str() {
            "d_time" => {
                if let Some(t) = t {
                    d.timer[t] += r.n;
                }
            }
            "d_fejl" => {
                if let (Some(t), Some(k), Some(o)) = (t, tal(), tal()) {
                    *d.fejl.entry((t, k, o)).or_default() += r.n;
                }
            }
            "d_forside" => {
                if let Some(t) = t {
                    d.forside[t] += r.n;
                }
            }
            "d_cache" => d.cache.entry(r.noegle.clone()).or_default().0 += r.n,
            "d_cache_b" => d.cache.entry(r.noegle.clone()).or_default().1 += r.n,
            "d_afskaaret" => {
                d.afsk.insert(r.noegle.clone());
            }
            _ => {}
        }
    }
    zd
}

/// Tallene for `fra`…`til`. Ren funktion.
pub fn regn(l: &Laest, fra: &str, til: &str) -> DriftUd {
    let dage = dage_i(fra, til);
    let zd = doegn(l, fra, til);
    let tom = Zd::default();
    let hentet = |z: &str, d: &str| er_hentet(l, z, d);
    let mut ud_dage = Vec::new();
    let mut periode_cache: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    let mut cache_dage = 0;
    let mut afskaaret_dage = BTreeSet::new();
    // zone → timens start (ms) → fejlede forsiden i timen.
    let mut fejltimer: BTreeMap<&str, BTreeMap<i64, bool>> = BTreeMap::new();
    for dag in &dage {
        let alle = !l.zoner.is_empty() && l.zoner.iter().all(|z| hentet(z, dag));
        let tal: Vec<(&str, &Zd)> = l
            .zoner
            .iter()
            .filter(|z| hentet(z, dag))
            .map(|z| (z.as_str(), zd.get(&(z.as_str(), dag.as_str())).unwrap_or(&tom)))
            .collect();
        let afsk: BTreeSet<String> = tal.iter().flat_map(|(_, d)| d.afsk.iter().cloned()).collect();
        if !afsk.is_empty() {
            afskaaret_dage.insert(dag.clone());
        }
        let med = |art: &str| alle && !afsk.contains(art);
        let sum = |art: &str, f: &dyn Fn(&Zd) -> i64| med(art).then(|| tal.iter().map(|(_, d)| f(d)).sum::<i64>());
        let start = dag_ms(dag).unwrap_or(0);
        let mut timer_med_fejl = BTreeSet::new();
        for (z, d) in &tal {
            for t in (0..24).filter(|t| d.time_med_fejl(*t)) {
                timer_med_fejl.insert(format!("{t:02}"));
                fejltimer
                    .entry(z)
                    .or_default()
                    .insert(start + t as i64 * TIME_MS, d.har("d_forside") && d.forside[t] > 0);
            }
        }
        let cache = med("d_cache").then(|| {
            let mut m: BTreeMap<String, (i64, i64)> = BTreeMap::new();
            for (_, d) in &tal {
                for (s, (n, b)) in &d.cache {
                    let x = m.entry(s.clone()).or_default();
                    x.0 += n;
                    x.1 += b;
                }
            }
            for (s, (n, b)) in &m {
                let x = periode_cache.entry(s.clone()).or_default();
                x.0 += n;
                x.1 += b;
            }
            cache_dage += 1;
            cache_ud(&m)
        });
        ud_dage.push(DriftDag {
            dag: dag.clone(),
            hentet: alle,
            svar: sum("d_time", &|d| d.timer.iter().sum()),
            kunne_ikke: sum("d_fejl", &|d| d.fejl_sum(|k, _| KUNNE_IKKE.contains(&k))),
            kunne_ikke_530: sum("d_fejl", &|d| d.fejl_sum(|k, _| k == KODE_530)),
            server_fejl: sum("d_fejl", &|d| d.fejl_sum(|_, o| SERVER_FEJL.contains(&o))),
            fejl: sum("d_fejl", &|d| d.fejl_sum(|_, _| true)),
            forside_fejl: sum("d_forside", &|d| d.forside.iter().sum()).map(|n| n > 0),
            timer_med_fejl: timer_med_fejl.into_iter().collect(),
            cache,
            afskaaret: afsk.into_iter().collect(),
        });
    }
    // Timerne for én valgt dag, lagt sammen over domænerne.
    let timer = match (dage.as_slice(), ud_dage.first()) {
        ([dag], Some(dd)) if dd.hentet => {
            let tal: Vec<&Zd> = l
                .zoner
                .iter()
                .map(|z| zd.get(&(z.as_str(), dag.as_str())).unwrap_or(&tom))
                .collect();
            (0..24)
                .map(|t| {
                    let mut par: BTreeMap<(i64, i64), i64> = BTreeMap::new();
                    for d in &tal {
                        for ((h, k, o), n) in &d.fejl {
                            if *h == t {
                                *par.entry((*k, *o)).or_default() += n;
                            }
                        }
                    }
                    TimeUd {
                        time: format!("{t:02}"),
                        svar: dd.svar.map(|_| tal.iter().map(|d| d.timer[t]).sum()),
                        fejl: if dd.kunne_ikke.is_some() {
                            par.into_iter()
                                .map(|((kant, origin), svar)| KodePar { kant, origin, svar })
                                .collect()
                        } else {
                            Vec::new()
                        },
                        forside: dd.forside_fejl.map(|_| tal.iter().map(|d| d.forside[t]).sum()),
                        med_fejl: dd.timer_med_fejl.contains(&format!("{t:02}")),
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    };
    let mut spaend: Vec<Spaend> = fejltimer.values().flat_map(spaend_af).collect();
    spaend.sort();
    spaend.dedup();
    let foerste_dag = l
        .zoner
        .iter()
        .flat_map(|z| l.hentet.get(z).into_iter().flatten().filter(move |d| hentet(z, d)))
        .min()
        .cloned();
    DriftUd {
        fra: fra.to_string(),
        til: til.to_string(),
        dage: ud_dage,
        timer,
        spaend,
        cache: (cache_dage > 0).then(|| cache_ud(&periode_cache)),
        cache_hele_domaenet: l.vaert.is_some(),
        foerste_dag,
        afskaaret: afskaaret_dage.into_iter().collect(),
        afvist: l.afvist,
        vaert: l.vaert.as_ref().map(|v| regn_vaert(l, v, &dage)),
    }
}

fn vaert_maerket<'a>(l: &'a Laest, art: &str) -> BTreeSet<&'a str> {
    l.vaert_raekker.iter().filter(|r| r.art == art).map(|r| r.dag.as_str()).collect()
}

/// Har døgnet fejltal pr. vært (reglen i `regn_vaert`)?
fn vaert_med(l: &Laest, d: &str) -> bool {
    let har = |art: &str| l.vaert_raekker.iter().any(|r| r.art == art && r.dag == d);
    let hentet = l.vaert_hentet.contains(d) && l.vaert_fra.as_deref().is_some_and(|f| d >= f);
    (hentet || har(V_FEJL)) && !har(V_AFSKAARET) && !har(V_FEJL_AFSKAARET) && !har(V_FEJL_AFVIST)
}

/// Fejl for værten `vaert`. Et døgn har tal, når det er hentet fra og med markøren (`store::VAERT_5XX_V1`) eller har gemte `v_fejl` (som V-SUBDOM.1 FIX 2),
/// og hverken hele svaret (`v_afskaaret`), aliaset (`v_fejl_afskaaret`) eller et afslag (`v_fejl_afvist`) har mærket det. Ren funktion.
fn regn_vaert(l: &Laest, vaert: &str, dage: &[String]) -> VaertUd {
    let maerket = |art: &str| vaert_maerket(l, art);
    let (hele, loft, afvist) = (maerket(V_AFSKAARET), maerket(V_FEJL_AFSKAARET), maerket(V_FEJL_AFVIST));
    let gemt = maerket(V_FEJL);
    let med = |d: &str| vaert_med(l, d);
    let ud_dage = dage
        .iter()
        .map(|d| {
            let koder: Vec<(i64, i64)> = l
                .vaert_raekker
                .iter()
                .filter(|r| r.art == V_FEJL && r.dag == *d)
                .filter_map(|r| {
                    let (v, k) = r.noegle.split_once('\t')?;
                    (v == vaert).then(|| k.parse().ok().map(|k: i64| (k, r.n)))?
                })
                .collect();
            let sum = |p: &dyn Fn(i64) -> bool| med(d).then(|| koder.iter().filter(|(k, _)| p(*k)).map(|x| x.1).sum());
            VaertDag {
                dag: d.clone(),
                hentet: med(d),
                kunne_ikke: sum(&|k| KUNNE_IKKE.contains(&k)),
                kunne_ikke_530: sum(&|k| k == KODE_530),
                andre_5xx: sum(&|k| k >= 500 && k != KODE_530 && !KUNNE_IKKE.contains(&k)),
            }
        })
        .collect();
    let i_perioden = |m: &BTreeSet<&str>| {
        m.iter()
            .filter(|d| dage.iter().any(|x| x == *d))
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    };
    let afskaaret: BTreeSet<&str> = hele.union(&loft).copied().collect();
    let fra = l
        .vaert_hentet
        .iter()
        .map(String::as_str)
        .filter(|d| l.vaert_fra.as_deref().is_some_and(|f| *d >= f))
        .chain(gemt.iter().copied())
        .min()
        .map(str::to_string);
    VaertUd {
        vaert: vaert.to_string(),
        dage: ud_dage,
        fra,
        afskaaret: i_perioden(&afskaaret),
        afvist: i_perioden(&afvist),
        afvist_nu: l.vaert_afvist_nu,
    }
}

// ── B2: tabellen i fanen Drift ─────────────────────────────────────────────────────────────────────────────────────────
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TimeN {
    /// Timen (UTC, "HH").
    pub time: String,
    pub svar: i64,
}

/// Én række i tabellen "Svarede dit website?". `slags`: "kode" (ét par af kant- og origin-kode for et domæne, med timer), "forside" (svar på `/` med
/// kant ≥ 500) eller "vaert" (én kant-kode for den valgte vært; kun dage, for timer findes ikke pr. vært).
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FejlRaekke {
    /// Domænets nøgle hos programmet (som i Overblikkets rækker); visningen slår navnet op.
    pub zone: String,
    pub slags: &'static str,
    pub kant: i64,
    pub origin: i64,
    /// Kanten er en af `overblik::SERVER_KODER`: "kunne ikke nås".
    pub naa: bool,
    /// Origin 500–599: "din server svarede med en fejl".
    pub server: bool,
    pub svar: i64,
    /// Så mange af svarene faldt i en time med fejl (`TIME_MIN_SVAR`/`TIME_PROCENT`).
    pub i_fejltimer: i64,
    /// Første og sidste time ("YYYY-MM-DDTHH", UTC); for "vaert" første og sidste dag ("YYYY-MM-DD").
    pub foerste: String,
    pub sidste: String,
    /// Svar pr. time, kun når perioden er én dag.
    pub timer: Vec<TimeN>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ZoneSpaend {
    pub zone: String,
    pub fra: String,
    pub til: String,
    pub timer: usize,
    pub forside_fejlede: bool,
}

/// En vært med svar med kant ≥ 500 i perioden (labelen under domænet, eller en kategori i skarpe parenteser).
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct VaertSum {
    pub vaert: String,
    pub svar: i64,
    pub kunne_ikke: i64,
}

/// Svaret fra kommandoen `drift_fejl`. Værtsnavnene er de rensede labels (`vaert::ren_vaert`), som i `subdomaener`.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct FejlUd {
    pub raekker: Vec<FejlRaekke>,
    pub spaend: Vec<ZoneSpaend>,
    /// Kun for ét domæne: værterne med fejl, flest først.
    pub vaerter: Vec<VaertSum>,
}

#[derive(Default)]
struct Saml {
    svar: i64,
    i_fejltimer: i64,
    foerste: Option<String>,
    sidste: Option<String>,
    timer: BTreeMap<usize, i64>,
}

impl Saml {
    fn laeg(&mut self, hvornaar: String, n: i64) {
        self.svar += n;
        if self.foerste.as_ref().is_none_or(|f| hvornaar < *f) {
            self.foerste = Some(hvornaar.clone());
        }
        if self.sidste.as_ref().is_none_or(|s| hvornaar > *s) {
            self.sidste = Some(hvornaar);
        }
    }
}

/// Tabellen, spændene pr. domæne og værterne med fejl for `fra`…`til`. Kun døgn, der er hentet, og kun arter, der ikke stod på loftet: en række, der
/// mangler, er ikke et 0 (fliserne siger "–"). Ren funktion.
pub fn regn_fejl(l: &Laest, fra: &str, til: &str) -> FejlUd {
    let dage = dage_i(fra, til);
    let en_dag = dage.len() == 1;
    let zd = doegn(l, fra, til);
    let mut rk: BTreeMap<(&str, &'static str, i64, i64), Saml> = BTreeMap::new();
    let mut spaend = Vec::new();
    for z in &l.zoner {
        let mut fejltimer: BTreeMap<i64, bool> = BTreeMap::new();
        for dag in dage.iter().filter(|d| er_hentet(l, z, d)) {
            let Some(d) = zd.get(&(z.as_str(), dag.as_str())) else { continue };
            if d.har("d_fejl") {
                for ((t, k, o), n) in &d.fejl {
                    let a = rk.entry((z.as_str(), "kode", *k, *o)).or_default();
                    a.laeg(format!("{dag}T{t:02}"), *n);
                    if d.time_med_fejl(*t) {
                        a.i_fejltimer += n;
                    }
                    if en_dag {
                        *a.timer.entry(*t).or_default() += n;
                    }
                }
            }
            if d.har("d_forside") {
                for t in (0..24).filter(|t| d.forside[*t] > 0) {
                    let a = rk.entry((z.as_str(), "forside", 0, 0)).or_default();
                    a.laeg(format!("{dag}T{t:02}"), d.forside[t]);
                    if d.time_med_fejl(t) {
                        a.i_fejltimer += d.forside[t];
                    }
                    if en_dag {
                        *a.timer.entry(t).or_default() += d.forside[t];
                    }
                }
            }
            let start = dag_ms(dag).unwrap_or(0);
            for t in (0..24).filter(|t| d.time_med_fejl(*t)) {
                fejltimer.insert(start + t as i64 * TIME_MS, d.har("d_forside") && d.forside[t] > 0);
            }
        }
        spaend.extend(spaend_af(&fejltimer).into_iter().map(|s| ZoneSpaend {
            zone: z.clone(),
            fra: s.fra,
            til: s.til,
            timer: s.timer,
            forside_fejlede: s.forside_fejlede,
        }));
    }
    // Pr. vært: kun kant-koder og kun dage.
    let mut vaerter: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    if let [z] = l.zoner.as_slice() {
        for r in l
            .vaert_raekker
            .iter()
            .filter(|r| r.art == V_FEJL && dage.contains(&r.dag) && vaert_med(l, &r.dag))
        {
            let Some((v, k)) = r.noegle.split_once('\t') else { continue };
            let Ok(k) = k.parse::<i64>() else { continue };
            let x = vaerter.entry(v).or_default();
            x.0 += r.n;
            if SERVER_KODER.contains(&k) {
                x.1 += r.n;
            }
            if l.vaert.as_deref() == Some(v) {
                rk.entry((z.as_str(), "vaert", k, 0)).or_default().laeg(r.dag.clone(), r.n);
            }
        }
    }
    let mut raekker: Vec<FejlRaekke> = rk
        .into_iter()
        .filter(|((_, slags, ..), _)| (*slags == "vaert") == l.vaert.is_some())
        .map(|((zone, slags, kant, origin), a)| FejlRaekke {
            zone: zone.to_string(),
            slags,
            kant,
            origin,
            naa: slags != "forside" && SERVER_KODER.contains(&kant),
            server: slags == "kode" && SERVER_FEJL.contains(&origin),
            svar: a.svar,
            i_fejltimer: a.i_fejltimer,
            foerste: a.foerste.unwrap_or_default(),
            sidste: a.sidste.unwrap_or_default(),
            timer: a
                .timer
                .into_iter()
                .map(|(t, svar)| TimeN {
                    time: format!("{t:02}"),
                    svar,
                })
                .collect(),
        })
        .collect();
    raekker.sort_by(|a, b| {
        b.svar
            .cmp(&a.svar)
            .then_with(|| (&a.zone, a.slags, a.kant, a.origin).cmp(&(&b.zone, b.slags, b.kant, b.origin)))
    });
    let mut vaerter: Vec<VaertSum> = vaerter
        .into_iter()
        .filter(|(_, (n, _))| *n > 0)
        .map(|(v, (svar, kunne_ikke))| VaertSum {
            vaert: v.to_string(),
            svar,
            kunne_ikke,
        })
        .collect();
    vaerter.sort_by(|a, b| b.svar.cmp(&a.svar).then_with(|| a.vaert.cmp(&b.vaert)));
    FejlUd { raekker, spaend, vaerter }
}

/// V-DRIFT.1 B2: "mellem kl. {a} og {b}" for statusdøgnet `dag` i lokal tid: døgnets længste spænd (ved lige mange timer det første). Slutningen er
/// timen efter spændets sidste time. `None` uden spænd. Ren funktion.
pub fn mellem_af(spaend: &[Spaend], tz: &crate::notify::Tidszone) -> Option<crate::overblik::Mellem> {
    let ms = |t: &str| -> Option<i64> {
        let (dag, hh) = t.split_once('T')?;
        Some(dag_ms(dag)? + time_nr(hh)? as i64 * TIME_MS)
    };
    let kl = |ms: i64| {
        let (t, m) = tz.kl(ms);
        format!("{t:02}.{m:02}")
    };
    let mut bedst: Option<&Spaend> = None;
    for s in spaend {
        if bedst.is_none_or(|b| s.timer > b.timer) {
            bedst = Some(s);
        }
    }
    let s = bedst?;
    Some(crate::overblik::Mellem {
        fra: kl(ms(&s.fra)?),
        til: kl(ms(&s.til)? + TIME_MS),
    })
}

/// Spændet for ét domænes statusdøgn, kun fra cachen. Er timetallene ikke hentet eller afskåret, er der intet spænd (`regn`), og teksten er som uden.
pub fn mellem(lager: &Store, zone: &str, dag: &str, tz: &crate::notify::Tidszone) -> Result<Option<crate::overblik::Mellem>, Fejl> {
    let l = laes(lager, &[zone.to_string()], dag, None)?;
    Ok(mellem_af(&regn(&l, dag, dag).spaend, tz))
}

#[cfg(test)]
mod tests;
