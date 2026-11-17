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

/// "Kunne ikke nås": Cloudflare fik intet brugbart svar fra serveren (INVESTIGATION Q2). 530 tælles for sig, så B2 kan vise den, uden at B1 ændrer reglen (D4).
pub const KUNNE_IKKE: RangeInclusive<i64> = 520..=527;
pub const KODE_530: i64 = 530;
/// "Serveren svarede med en fejl": origin 500–599. Ændrer aldrig status (D5).
pub const SERVER_FEJL: RangeInclusive<i64> = 500..=599;
/// En time med fejl: mindst så mange svar med `overblik::SERVER_KODER` (i dag 520–527, samme regel som "Din server svarer ikke") og mindst så mange
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
    if let (Some(_), [z]) = (vaert, zoner) {
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

/// Tallene for `fra`…`til`. Ren funktion.
pub fn regn(l: &Laest, fra: &str, til: &str) -> DriftUd {
    let dage = dage_i(fra, til);
    // (zone, dag) → døgnets tal.
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
    let tom = Zd::default();
    let hentet = |z: &str, d: &str| l.hentet.get(z).is_some_and(|h| h.contains(d)) && !l.udenfor.contains(&(z.to_string(), d.to_string()));
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

/// Fejl for værten `vaert`. Et døgn har tal, når det er hentet fra og med markøren (`store::VAERT_5XX_V1`) eller har gemte `v_fejl` (som V-SUBDOM.1 FIX 2),
/// og hverken hele svaret (`v_afskaaret`), aliaset (`v_fejl_afskaaret`) eller et afslag (`v_fejl_afvist`) har mærket det. Ren funktion.
fn regn_vaert(l: &Laest, vaert: &str, dage: &[String]) -> VaertUd {
    let maerket = |art: &str| -> BTreeSet<&str> { l.vaert_raekker.iter().filter(|r| r.art == art).map(|r| r.dag.as_str()).collect() };
    let (hele, loft, afvist) = (maerket(V_AFSKAARET), maerket(V_FEJL_AFSKAARET), maerket(V_FEJL_AFVIST));
    let gemt = maerket(V_FEJL);
    let med = |d: &str| {
        let hentet = l.vaert_hentet.contains(d) && l.vaert_fra.as_deref().is_some_and(|f| d >= f);
        (hentet || gemt.contains(d)) && !hele.contains(d) && !loft.contains(d) && !afvist.contains(d)
    };
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

#[cfg(test)]
mod tests;
