//! Cloudflare Radar: verden og landene ved siden af brugerens egne tal. Kaldene ligger under `api.cloudflare.com/client/v4/radar/`, går
//! gennem `cf::Klient::radar` og kører uden for hentekøen og uden om den fælles begrænser: hver 6. time og, når visningen åbnes, hvis det
//! er over 1 time siden. Al fejl er `Fejl::Radar` og rammer kun Radar-visningen, aldrig køen eller et domænes status.
//!
//! Tidsserierne hentes med `MIN0_MAX`, som Radars dokumentation beskriver som "a proportion of the maximum value in the entire
//! response": proportional, så indekset (100 = periodens gennemsnit) er rigtigt, men kun inden for ét svar. Derfor hentes hele vinduet på
//! 30 hele UTC-døgn i ét kald hver gang, og kun tal med samme `hentet_utc` regnes sammen (`verden_serie`). Ældre døgn bliver stående med
//! deres egen skala. Top-lister og fordelinger er procenter og hentes pr. døgn.
//!
//! Kilde og mål: angreb vises fra målsiden, for et kildeland kan skyldes VPN eller lejede servere. `top/locations/target`
//! er målet (zonens faktureringsland). `top/locations/origin` ignorerer `location` (målt live), så kilderne til angreb mod et land kommer fra angrebsparrene
//! `top/attacks` med `limitDirection=TARGET` og `limitPerLocation` (målt live): de største kilder pr. mål, sorteret globalt og afskåret ved 200 par. Værdierne er procent af
//! alle angreb i verden, så de deles med landets egen andel som mål for samme `dateRange` og bliver til en andel af angrebene mod landet (`mod_landet`).
//! Alle andre angrebs-endpoints filtrerer på kilden og bruges ikke med `location`. Der findes ingen tidsserie for angreb mod et land, men
//! landets andel af målene hentes pr. døgn og kan tegnes som en målt andel, aldrig som et antal.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::cf::Fejl;
use crate::overblik::{self, median, Forstyrrelse, Raekke1, Status, Tendens};
use crate::store::{RadarRaekke, Raekke, Store};
use crate::vaern::{dag_ms, dag_noegle, er_faerdig, iso, js_slice, parse_iso, DOEGN_MS, HENT_DAGE};

// Serierne i `radar_dag`.
pub const L7: &str = "l7";
pub const L7_WAF: &str = "l7_waf";
pub const HTTP: &str = "http";
pub const MAAL: &str = "maal";
pub const KILDE: &str = "kilde";
pub const PRODUKT: &str = "produkt";
pub const BOT: &str = "bot";
pub const AI: &str = "ai";
pub const AI_NAVN: &str = "ai_navn";
pub const KVANTE: &str = "kvante";
/// Kilder til angreb mod landet som andel af angrebene mod det. En tom nøgle betyder, at der er spurgt.
pub const PAR7: &str = "par7";
pub const PAR28: &str = "par28";
/// Internetforstyrrelser i landet, eller i verden uden land. Nøglen er felterne fra `udfald`; en tom nøgle betyder, at der er spurgt.
pub const UDFALD: &str = "udfald";
/// Dokumenteret proportional (se øverst). Tal med en anden normalisering indgår ikke i et indeks.
pub const NORMALISERING: &str = "MIN0_MAX";
pub const PROCENT: &str = "PERCENTAGE";
// Nøgler i `indstillinger`.
pub const LAND: &str = "land";
pub const SIDST: &str = "radar_sidst";
pub const FEJL: &str = "radar_fejl";
/// Serier, der ikke kunne bruges ved seneste forsøg, som JSON. Med `:<land>` for landets. De koster kun deres eget afsnit.
pub const SERIE_FEJL: &str = "radar_serier";
/// Baggrunden opdaterer så ofte, og visningen, når den åbnes og tallene er så gamle.
pub const OPDATER_MS: i64 = 6 * 3_600_000;
pub const AABEN_MS: i64 = 3_600_000;
/// Mellem to Radar-kald. Radars egen grænse er ikke oplyst.
pub const TEMPO: Duration = Duration::from_millis(1100);
/// `top/locations/target`: står landet ikke blandt så mange, kan dets andel ikke vises.
pub const TOP_MAAL: usize = 100;
/// `top/attacks`: det højeste `limit`, Radar tager (målt live).
pub const TOP_PAR: usize = 200;
/// `limitPerLocation`: 3 giver flest kilder pr. land for de lande, der er med. Mangler landet, prøves 2 og 1, som rækker længere ned (målt live).
pub const PR_MAAL: [usize; 3] = [3, 2, 1];
/// `dateRange` i de to perioder. Radar kender 7d og 28d, så HAM's "30 dage" er 4 uger her.
pub const UGE: &str = "7d";
pub const FIRE_UGER: &str = "28d";
pub const FIRE_UGER_DAGE: i64 = 28;
/// Så mange lande vises.
const TOP_LANDE: usize = 10;
/// Så mange forstyrrelser hentes.
const TOP_UDFALD: usize = 25;
/// "Særligt hos dig": landet fylder mindst så mange gange mere hos dig end for det valgte land, og mindst så mange procent hos dig.
pub const SAERLIGT_FAKTOR: f64 = 2.0;
pub const SAERLIGT_MIN: f64 = 5.0;
/// `forslag`: et døgn med så mange gange medianen er et trafikspring, og så stor en del skal ét land have.
pub const FORSLAG_SPRING: f64 = 10.0;
pub const FORSLAG_ANDEL: f64 = 0.5;
/// r_ref: snittet af så mange døgn før.
const SNIT_DOEGN: i64 = overblik::SNIT_FRA - overblik::SNIT_TIL + 1;

/// Det, opdateringen skal bruge udefra: ét GET under radar/. Testene svarer selv.
pub trait Kilde: Sync {
    fn hent<'a>(&'a self, sti: &'a str) -> impl Future<Output = Result<Value, Fejl>> + Send + 'a;
}

// ── Stierne ────────────────────────────────────────────────────────────────────────────────────────────────────────
fn periode(fra: &str, til: &str) -> String {
    format!("dateStart={fra}T00:00:00Z&dateEnd={til}T00:00:00Z&format=json")
}

/// Tidsserierne for hele vinduet: `fra` til og med dagen før `til`.
fn tidsserier(fra: &str, til: &str) -> [(&'static str, String); 3] {
    let p = format!("aggInterval=1d&normalization={NORMALISERING}&{}", periode(fra, til));
    [
        (L7, format!("radar/attacks/layer7/timeseries?{p}")),
        (L7_WAF, format!("radar/attacks/layer7/timeseries?mitigationProduct=WAF&{p}")),
        (HTTP, format!("radar/http/timeseries?{p}")),
    ]
}

/// Mål er `target`, kilde er `origin`.
fn sti_maal(dag: &str, til: &str) -> String {
    format!("radar/attacks/layer7/top/locations/target?limit={TOP_MAAL}&{}", periode(dag, til))
}

const ORIGIN: &str = "originCountryAlpha2";

fn sti_kilde(dag: &str, til: &str) -> String {
    format!("radar/attacks/layer7/top/locations/origin?limit={TOP_LANDE}&{}", periode(dag, til))
}

/// Dagens fordelinger. `bot_class` og `post_quantum` er mærket forældede til fordel for `radar/http/summary/{dimension}`, men kun de faste
/// stier har en dokumenteret svarform. Uden `location`: dér er det brugerens land, ikke sitets, så de hører til Verden.
fn fordelinger(dag: &str, til: &str) -> [(&'static str, String); 5] {
    let p = periode(dag, til);
    [
        (PRODUKT, format!("radar/attacks/layer7/summary/mitigation_product?{p}")),
        (BOT, format!("radar/http/summary/bot_class?{p}")),
        (AI, format!("radar/ai/bots/summary/CRAWL_PURPOSE?{p}")),
        (AI_NAVN, format!("radar/ai/bots/summary/USER_AGENT?{p}")),
        (KVANTE, format!("radar/http/summary/post_quantum?{p}")),
    ]
}

/// Angrebsparrene: de `pr_maal` største kilder pr. mål.
fn sti_par(range: &str, pr_maal: usize) -> String {
    format!(
        "radar/attacks/layer7/top/attacks?limit={TOP_PAR}&limitDirection=TARGET&limitPerLocation={pr_maal}&dateRange={range}&format=json"
    )
}

/// Landenes andel som mål for hele perioden, med samme `dateRange` som parrene.
fn sti_maal_range(range: &str) -> String {
    format!("radar/attacks/layer7/top/locations/target?limit={TOP_MAAL}&dateRange={range}&format=json")
}

/// Forstyrrelserne de seneste 30 dage, som W henter dem: i landet, eller uden land i hele verden.
fn sti_udfald(kode: Option<&str>) -> String {
    let land = kode.map_or_else(String::new, |k| format!("location={k}&"));
    format!("radar/annotations/outages?{land}dateRange={HENT_DAGE}d&limit={TOP_UDFALD}&format=json")
}

// ── Svarene ────────────────────────────────────────────────────────────────────────────────────────────────────────
/// Radars tal står som tekst ("4.323214") eller tal.
fn tal(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .filter(|x: &f64| x.is_finite())
}

/// Et tidsstempel → HAM's døgn. Kun præcis midnat UTC: et punkt midt i et døgn eller med en anden tidszone er ikke et helt UTC-døgn, og
/// det gættes der ikke på. Ren funktion.
pub fn utc_dag(t: &str) -> Option<String> {
    parse_iso(t).filter(|ms| ms.rem_euclid(DOEGN_MS) == 0).map(dag_noegle)
}

/// En tidsserie → normaliseringens navn og (dag, værdi) for de døgn, der er bedt om. Et ufærdigt døgn står ikke i `dage`. Ren funktion.
pub fn punkter(j: &Value, dage: &[String]) -> Option<(String, Vec<(String, f64)>)> {
    let s = j.pointer("/result/serie_0")?;
    let (t, v) = (s.get("timestamps")?.as_array()?, s.get("values")?.as_array()?);
    let ud = t
        .iter()
        .zip(v)
        .filter_map(|(t, v)| Some((utc_dag(t.as_str()?)?, tal(v)?)))
        .filter(|(d, _)| dage.contains(d))
        .collect();
    let norm = j
        .pointer("/result/meta/normalization")
        .and_then(Value::as_str)
        .unwrap_or(NORMALISERING);
    Some((norm.to_string(), ud))
}

/// `result.top_0` → (landekode, procent). `felt` afgør, om det er målet eller kilden. Ren funktion.
pub fn top(j: &Value, felt: &str) -> Option<Vec<(String, f64)>> {
    let liste = j.pointer("/result/top_0")?.as_array()?;
    Some(
        liste
            .iter()
            .filter_map(|x| Some((x.get(felt)?.as_str()?.to_string(), tal(x.get("value")?)?)))
            .collect(),
    )
}

/// `result.top_0` i angrebsparrene → (kildeland, procent af alle angreb) for de par, der har landet som mål. Ren funktion.
pub fn par_mod(j: &Value, kode: &str) -> Option<Vec<(String, f64)>> {
    let liste = j.pointer("/result/top_0")?.as_array()?;
    Some(
        liste
            .iter()
            .filter(|x| x.get("targetCountryAlpha2").and_then(Value::as_str) == Some(kode))
            .filter_map(|x| Some((x.get(ORIGIN)?.as_str()?.to_string(), tal(x.get("value")?)?)))
            .collect(),
    )
}

/// `summary_0` → (navn, procent). Dokumentationen viser den både med og uden `result`. Ren funktion.
pub fn fordeling(j: &Value) -> Option<Vec<(String, f64)>> {
    let m = j.pointer("/result/summary_0").or_else(|| j.get("summary_0"))?.as_object()?;
    Some(m.iter().filter_map(|(k, v)| Some((k.clone(), tal(v)?))).collect())
}

/// Én forstyrrelse som nøgle i `radar_dag`: start, slut, årsag, type, netværk og lande, skilt med tabulator.
fn udfald_noegle(a: &Value) -> Option<(String, String)> {
    let felt = |f: &str| a.get(f).and_then(Value::as_str).unwrap_or("").replace('\t', " ");
    let start = felt("startDate");
    let dag = start.get(..10).filter(|d| dag_ms(d).is_some())?.to_string();
    let udfald = |f: &str| {
        a.pointer(&format!("/outage/{f}"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .replace('\t', " ")
    };
    let navne = a.get("asnsDetails").and_then(Value::as_array).into_iter().flatten();
    let mut net: Vec<String> = navne
        .filter_map(|x| x.get("name")?.as_str())
        .map(|n| js_slice(&n.replace(['\t', ','], " "), 60))
        .collect();
    if net.is_empty() {
        let numre = a.get("asns").and_then(Value::as_array).into_iter().flatten();
        net = numre.map(|n| format!("AS{}", n.to_string().trim_matches('"'))).collect();
    }
    let lande = a.get("locations").and_then(Value::as_array).into_iter().flatten();
    let lande: Vec<&str> = lande.filter_map(Value::as_str).collect();
    let dele = [
        start,
        felt("endDate"),
        udfald("outageCause"),
        udfald("outageType"),
        net.join(", "),
        lande.join(","),
    ];
    Some((dag, dele.join("\t")))
}

/// `annotations` → (startdag, nøgle). Står landet ikke i `locations`, handler forstyrrelsen ikke om det; uden land gælder alle. Ren funktion.
pub fn udfald(j: &Value, kode: Option<&str>) -> Option<Vec<(String, String)>> {
    let liste = j.pointer("/result/annotations")?.as_array()?;
    let her = |a: &&Value| {
        kode.is_none()
            || a.get("locations")
                .and_then(Value::as_array)
                .is_none_or(|l| l.is_empty() || l.iter().any(|x| x.as_str() == kode))
    };
    Some(liste.iter().filter(her).filter_map(udfald_noegle).collect())
}

// ── Opdateringen ───────────────────────────────────────────────────────────────────────────────────────────────────
/// Tid til en opdatering? Stilles uret tilbage, venter den. Ren funktion.
pub fn skal_opdatere(sidst: Option<&str>, nu: i64, interval: i64) -> bool {
    sidst.and_then(parse_iso).is_none_or(|t| nu.saturating_sub(t) >= interval)
}

/// De 30 hele døgn til og med i går, ældste først.
pub fn vindue(nu: i64) -> Vec<String> {
    (1..=HENT_DAGE).rev().map(|i| dag_noegle(nu - i * DOEGN_MS)).collect()
}

/// Døgn uden tal, eller hvor tallene blev hentet, før døgnet var færdigt. Ren funktion.
pub fn mangler(dage: &[String], hentet: &HashMap<String, String>) -> Vec<String> {
    dage.iter()
        .filter(|d| !hentet.get(*d).is_some_and(|h| er_faerdig(d, h)))
        .cloned()
        .collect()
}

fn laas(lager: &Mutex<Store>) -> Result<std::sync::MutexGuard<'_, Store>, Fejl> {
    lager.lock().map_err(|_| Fejl::Lager)
}

/// Gemmer ét døgns tal med nøgler.
fn gem_dag(lager: &Mutex<Store>, serie: &str, land: &str, dag: &str, tal: Vec<(String, f64)>, nu: i64) -> Result<(), Fejl> {
    let rk: Vec<_> = tal.into_iter().map(|(k, v)| (dag.to_string(), k, v)).collect();
    laas(lager)?.gem_radar(serie, land, &[dag.to_string()], &rk, PROCENT, &iso(nu))
}

/// Svaret, eller `None`, hvis stien eller dens parametre blev afvist (400, 404): det rammer kun den serie, hvor 401, 403, 429 og 5xx rammer
/// alle kald og stopper runden.
async fn svar<K: Kilde>(k: &K, sti: &str) -> Result<Option<Value>, Fejl> {
    match k.hent(sti).await {
        Err(Fejl::Radar(Some(400 | 404))) => Ok(None),
        x => x.map(Some),
    }
}

/// Gemmer forstyrrelserne for landet eller (tomt) verden. En forstyrrelse, der begyndte før vinduet, står på første døgn. Den tomme nøgle viser, at der er spurgt.
fn gem_udfald(lager: &Mutex<Store>, land: &str, u: Vec<(String, String)>, nu: i64) -> Result<(), Fejl> {
    let mut dage = vindue(nu);
    dage.push(dag_noegle(nu));
    let rk: Vec<_> = u
        .into_iter()
        .map(|(d, n)| (d.max(dage[0].clone()), n, 0.0))
        .chain([(dag_noegle(nu - DOEGN_MS), String::new(), 0.0)])
        .collect();
    laas(lager)?.gem_radar(UDFALD, land, &dage, &rk, "", &iso(nu))
}

/// Verden: tidsserierne, i går, de døgn af landenes andel, der mangler, og forstyrrelserne. Svarer Radar med en fejl, der rammer alle kald, stopper den, og resten kommer
/// næste gang. Et svar, der ikke kan læses, koster kun den serie: formen er ikke dokumenteret for alle stier, og de andre skal stadig med. Svaret er de serier, der mangler.
pub async fn opdater<K: Kilde>(lager: &Mutex<Store>, k: &K, nu: i64, tempo: Duration) -> Result<Vec<&'static str>, Fejl> {
    let dage = vindue(nu);
    let (i_dag, i_gaar) = (dag_noegle(nu), dag_noegle(nu - DOEGN_MS));
    let mut ulaeselige = Vec::new();
    for (serie, sti) in tidsserier(&dage[0], &i_dag) {
        match svar(k, &sti).await?.and_then(|j| punkter(&j, &dage)).filter(|(_, p)| !p.is_empty()) {
            Some((norm, pkt)) => {
                let rk: Vec<_> = pkt.into_iter().map(|(d, v)| (d, String::new(), v)).collect();
                laas(lager)?.gem_radar(serie, "", &dage, &rk, &norm, &iso(nu))?;
            }
            None => ulaeselige.push(serie),
        }
        tokio::time::sleep(tempo).await;
    }
    let ny = |serie: &str| -> Result<bool, Fejl> {
        Ok(!mangler(std::slice::from_ref(&i_gaar), &laas(lager)?.radar_hentet(serie, "")?).is_empty())
    };
    for (serie, sti) in fordelinger(&i_gaar, &i_dag) {
        if ny(serie)? {
            match svar(k, &sti).await?.as_ref().and_then(fordeling) {
                Some(f) => gem_dag(lager, serie, "", &i_gaar, f, nu)?,
                None => ulaeselige.push(serie),
            }
            tokio::time::sleep(tempo).await;
        }
    }
    if ny(KILDE)? {
        match svar(k, &sti_kilde(&i_gaar, &i_dag)).await?.and_then(|j| top(&j, ORIGIN)) {
            Some(t) => gem_dag(lager, KILDE, "", &i_gaar, t, nu)?,
            None => ulaeselige.push(KILDE),
        }
        tokio::time::sleep(tempo).await;
    }
    match svar(k, &sti_udfald(None)).await?.and_then(|j| udfald(&j, None)) {
        Some(u) => gem_udfald(lager, "", u, nu)?,
        None => ulaeselige.push(UDFALD),
    }
    tokio::time::sleep(tempo).await;
    let hentet = laas(lager)?.radar_hentet(MAAL, "")?;
    for dag in mangler(&dage, &hentet).iter().rev() {
        let til = dag_noegle(dag_ms(dag).ok_or(Fejl::Radar(None))? + DOEGN_MS);
        match svar(k, &sti_maal(dag, &til)).await?.and_then(|j| top(&j, "targetCountryAlpha2")) {
            Some(t) => gem_dag(lager, MAAL, "", dag, t, nu)?,
            None if ulaeselige.contains(&MAAL) => {}
            None => ulaeselige.push(MAAL),
        }
        tokio::time::sleep(tempo).await;
    }
    Ok(ulaeselige)
}

/// Første døgn i de sidste `dage` hele døgn.
fn foer(nu: i64, dage: i64) -> String {
    dag_noegle(nu - dage * DOEGN_MS)
}

/// De største kilder til angreb mod landet som procent af angrebene mod det, størst først, plus en tom nøgle som tegn på, at der er spurgt. Rækker landet ikke ind på listen,
/// er det kun den tomme nøgle: Radar oplyser det ikke. `None`: et svar kunne ikke bruges.
async fn mod_landet<K: Kilde>(k: &K, kode: &str, range: &str, tempo: Duration) -> Result<Option<Vec<(String, f64)>>, Fejl> {
    let spurgt = (String::new(), 0.0);
    let Some(maal) = svar(k, &sti_maal_range(range)).await?.and_then(|j| top(&j, "targetCountryAlpha2")) else {
        return Ok(None);
    };
    tokio::time::sleep(tempo).await;
    let Some(andel) = maal.iter().find(|x| x.0 == kode).map(|x| x.1).filter(|v| *v > 0.0) else {
        return Ok(Some(vec![spurgt]));
    };
    for n in PR_MAAL {
        let Some(par) = svar(k, &sti_par(range, n)).await?.and_then(|j| par_mod(&j, kode)) else {
            return Ok(None);
        };
        tokio::time::sleep(tempo).await;
        if !par.is_empty() {
            let mut ud: Vec<_> = par.into_iter().map(|(l, v)| (l, (100.0 * v / andel).min(100.0))).collect();
            ud.push(spurgt);
            return Ok(Some(ud));
        }
    }
    Ok(Some(vec![spurgt]))
}

/// Ét land: hvor angreb mod landet kom fra (7 dage og 4 uger, gemt på i går) og forstyrrelserne. En serie, der ikke kan bruges, koster kun sit eget afsnit.
pub async fn opdater_land<K: Kilde>(lager: &Mutex<Store>, k: &K, kode: &str, nu: i64, tempo: Duration) -> Result<Vec<&'static str>, Fejl> {
    let i_gaar = dag_noegle(nu - DOEGN_MS);
    let mut ulaeselige = Vec::new();
    for (serie, range) in [(PAR7, UGE), (PAR28, FIRE_UGER)] {
        match mod_landet(k, kode, range, tempo).await? {
            Some(t) => gem_dag(lager, serie, kode, &i_gaar, t, nu)?,
            None => ulaeselige.push(serie),
        }
    }
    match svar(k, &sti_udfald(Some(kode))).await?.and_then(|j| udfald(&j, Some(kode))) {
        Some(u) => gem_udfald(lager, kode, u, nu)?,
        None => ulaeselige.push(UDFALD),
    }
    Ok(ulaeselige)
}

/// Opdaterer det, der er for gammelt efter `interval`, og noterer tidspunkt og fejl, så en fejl ikke gentages hvert minut. Fejlen når
/// kun Radar-visningen: en fejl, der stoppede runden, står i `LandUd::fejl`, og serier, der ikke kunne bruges, i `LandUd::serie_fejl`. `true`: der blev spurgt.
pub async fn koer<K: Kilde>(
    lager: &Mutex<Store>,
    k: &K,
    kode: Option<&str>,
    nu: i64,
    interval: i64,
    tempo: Duration,
) -> Result<bool, Fejl> {
    let mut spurgt = false;
    let noter = |noegle: &str, serier: &str, svar: Result<Vec<&'static str>, Fejl>| -> Result<(), Fejl> {
        let mut s = laas(lager)?;
        s.gem_indstilling(noegle, &iso(nu))?;
        s.gem_indstilling(
            serier,
            &serde_json::to_string(svar.as_ref().map_or(&[][..], Vec::as_slice)).unwrap_or_default(),
        )?;
        s.gem_indstilling(
            FEJL,
            &svar
                .err()
                .map_or_else(String::new, |e| serde_json::to_string(&e).unwrap_or_default()),
        )
    };
    let sidst = laas(lager)?.indstilling(SIDST)?;
    if skal_opdatere(sidst.as_deref(), nu, interval) {
        spurgt = true;
        noter(SIDST, SERIE_FEJL, opdater(lager, k, nu, tempo).await)?;
    }
    if let Some(kode) = kode {
        let noegle = format!("{SIDST}:{kode}");
        let sidst = laas(lager)?.indstilling(&noegle)?;
        if skal_opdatere(sidst.as_deref(), nu, interval) {
            spurgt = true;
            noter(
                &noegle,
                &format!("{SERIE_FEJL}:{kode}"),
                opdater_land(lager, k, kode, nu, tempo).await,
            )?;
        }
    }
    Ok(spurgt)
}

// ── Indeks og sammenligning ────────────────────────────────────────────────────────────────────────────────────────
/// Indeks: 100 er gennemsnittet af de døgn, der har tal. En serie uden tal eller kun med 0 har intet indeks. Ren funktion.
pub fn indeks(v: &[Option<f64>]) -> Vec<Option<f64>> {
    let tal: Vec<f64> = v.iter().flatten().copied().collect();
    let snit = tal.iter().sum::<f64>() / tal.len().max(1) as f64;
    if snit <= 0.0 {
        return vec![None; v.len()];
    }
    v.iter().map(|x| x.map(|x| (1000.0 * x / snit).round() / 10.0)).collect()
}

/// Én tidsserie på én skala: tallene fra den nyeste hentning med den proportionale normalisering. Ren funktion.
pub fn verden_serie(rows: &[RadarRaekke], serie: &str) -> BTreeMap<String, f64> {
    let rk: Vec<_> = rows
        .iter()
        .filter(|r| r.serie == serie && r.normalisering == NORMALISERING)
        .collect();
    let nyeste = rk.iter().map(|r| r.hentet_utc.as_str()).max().unwrap_or("");
    rk.iter()
        .filter(|r| r.hentet_utc == nyeste)
        .map(|r| (r.dag.clone(), r.vaerdi))
        .collect()
}

/// r_ref: værdien i døgnet ÷ snittet af de 7 døgn før. Mangler ét af dem, er der intet at sammenligne med. Ren funktion.
pub fn forhold(serie: &BTreeMap<String, f64>, dag: &str) -> Option<f64> {
    let ms = dag_ms(dag)?;
    let foer = (1..=SNIT_DOEGN)
        .map(|i| serie.get(&dag_noegle(ms - i * DOEGN_MS)).copied())
        .collect::<Option<Vec<f64>>>()?;
    let snit = foer.iter().sum::<f64>() / SNIT_DOEGN as f64;
    (snit > 0.0).then(|| serie.get(dag).map(|v| v / snit))?
}

/// Sammenligningen for én række i overblikket. `waf`: `verden_serie` for `L7_WAF`.
pub fn tendens_for(r: &Raekke1, rows: &[Raekke], waf: &BTreeMap<String, f64>) -> Option<Tendens> {
    let nu = dag_ms(&r.status_dag)? + DOEGN_MS;
    // Gul på grund af serveren eller trafikken siger intet om angrebene: sammenligningen gælder kun det, angrebene alene gav.
    overblik::tendens(r.angreb_status, r.gammel, overblik::r_mig(rows, nu), forhold(waf, &r.status_dag))
}

/// En forstyrrelse, der daekker dagen. Uden slutdato tæller kun startdagen: dokumentationen siger ikke, hvad en tom slutdato betyder. Ren funktion.
pub fn forstyrrelse(rows: &[RadarRaekke], land: &str, dag: &str) -> Option<Forstyrrelse> {
    rows.iter().filter(|r| r.serie == UDFALD).find_map(|r| {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let (start, slut) = (f.first()?.get(..10)?, f.get(1)?.get(..10).unwrap_or(""));
        let daekker = if slut.is_empty() {
            start == dag
        } else {
            start <= dag && dag <= slut
        };
        daekker.then(|| Forstyrrelse {
            land: land.to_string(),
            netvaerk: f
                .get(4)
                .filter(|n| !n.is_empty())
                .map(|n| n.split(", ").next().unwrap_or(n).to_string()),
        })
    })
}

// ── Land ───────────────────────────────────────────────────────────────────────────────────────────────────────────
/// To store bogstaver. Andet er ikke en landekode, Radar kender.
pub fn tjek_land(kode: &str) -> Result<String, Fejl> {
    if kode.len() == 2 && kode.bytes().all(|b| b.is_ascii_uppercase()) {
        Ok(kode.to_string())
    } else {
        Err(Fejl::UgyldigtFilter)
    }
}

/// Et andet land end det valgte, kun når det er tydeligt, ellers intet: en forkert knap er værre end ingen. Art `land` er filtreret på
/// `requestSource:"eyeball"` (`adgang::ADGANG_QUERY`), men det udelader kun Cloudflares egen trafik, ikke robotter. Derfor:
/// - døgn, hvor domænets forespørgsler er mindst `FORSLAG_SPRING` gange medianen, tæller ikke (et trafikspring siger intet om besøgende),
/// - art `bot` har intet land, så døgnets robotter trækkes fra døgnets største land. Det kan kun give færre forslag, aldrig flere,
/// - ét land skal have mindst `FORSLAG_ANDEL` af det, der er tilbage. Ukendt (XX), Tor (T1) og andet, der ikke er en landekode, kan ikke
///   foreslås, men tæller med i helheden. Ren funktion.
pub fn forslag(rows: &[(String, Raekke)], valgt: Option<&str>) -> Option<String> {
    let mut alle: HashMap<&str, HashMap<&str, i64>> = HashMap::new();
    let mut robotter: HashMap<(&str, &str), i64> = HashMap::new();
    let mut lande: BTreeMap<(&str, &str), Vec<(&str, i64)>> = BTreeMap::new();
    for (z, r) in rows {
        let dag = (z.as_str(), r.dag.as_str());
        match r.art.as_str() {
            "alle" => *alle.entry(dag.0).or_default().entry(dag.1).or_default() += r.n,
            "bot" => *robotter.entry(dag).or_default() += r.n,
            "land" => lande.entry(dag).or_default().push((&r.noegle, r.n)),
            _ => {}
        }
    }
    let gyldig = |k: &str| k != "XX" && tjek_land(k).is_ok();
    let medianer: HashMap<&str, f64> = alle
        .iter()
        .map(|(z, d)| (*z, median(&mut d.values().copied().collect::<Vec<_>>())))
        .collect();
    let (mut n, mut i_alt): (BTreeMap<&str, i64>, i64) = (BTreeMap::new(), 0);
    for ((z, dag), mut l) in lande {
        let trafik = alle.get(z).and_then(|d| d.get(dag)).copied().unwrap_or(0) as f64;
        if trafik >= FORSLAG_SPRING * medianer.get(z).copied().unwrap_or(0.0) {
            continue;
        }
        if let Some(stoerst) = l.iter_mut().filter(|x| gyldig(x.0)).max_by_key(|x| x.1) {
            stoerst.1 = (stoerst.1 - robotter.get(&(z, dag)).copied().unwrap_or(0)).max(0);
        }
        for (k, v) in l {
            i_alt += v;
            if gyldig(k) {
                *n.entry(k).or_default() += v;
            }
        }
    }
    let (land, antal) = n.into_iter().max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))?;
    (antal > 0 && antal as f64 >= FORSLAG_ANDEL * i_alt as f64 && Some(land) != valgt).then(|| land.to_string())
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DagUd {
    pub dag: String,
    /// Angreb på de valgte domæner; `null`, hvis døgnet ikke er hentet.
    pub mit: Option<i64>,
    pub mit_indeks: Option<f64>,
    pub verden_indeks: Option<f64>,
}

/// Landets andel af målene ét døgn; `null` = ikke blandt de 100 mest ramte.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Andel {
    pub dag: String,
    pub vaerdi: Option<f64>,
}

/// Der står kun noget i ord, når døgnet opfylder gul-reglen.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Ord {
    pub dag: String,
    pub angreb: i64,
    pub snit: f64,
    pub tendens: Option<Tendens>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct NavnPct {
    pub navn: String,
    pub vaerdi: f64,
}

/// Et land i kolonnen med dine domæner.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct MitLand {
    pub navn: String,
    pub vaerdi: f64,
    pub n: i64,
    pub saerligt: bool,
}

/// To kolonner for samme periode: hvor angreb mod landet kom fra, og hvor angreb mod dine domæner kom fra. Procent af hele perioden, så en
/// kolonne summerer til højst 100.
#[derive(Serialize, Debug, Clone, PartialEq, Default)]
pub struct Sammenligning {
    /// De største kilder, Radar oplyser, som andel af angrebene mod landet.
    pub land: Vec<NavnPct>,
    /// Resten: andre lande, Radar ikke oplyser. `null` uden kilder.
    pub rest: Option<f64>,
    /// Der er spurgt: en tom `land` betyder, at Radar ikke oplyser det for landet.
    pub spurgt: bool,
    pub mine: Vec<MitLand>,
    pub mine_i_alt: i64,
    /// Dine domæners periode, til dyk.
    pub fra: String,
    pub til: String,
}

/// Én forstyrrelse. `minutter`: `null`, hvis slutdatoen mangler. Årsag og type er Radars koder, og `lande` er landekoderne.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Udfald {
    pub start: String,
    pub slut: Option<String>,
    pub minutter: Option<i64>,
    pub aarsag: String,
    pub slags: String,
    pub netvaerk: Vec<String>,
    pub lande: Vec<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct LandUd {
    pub kode: String,
    pub i_gaar: String,
    pub dage: Vec<DagUd>,
    /// Nyeste først, kun hentede døgn.
    pub andel: Vec<Andel>,
    pub ord: Option<Ord>,
    pub sammen7: Sammenligning,
    pub sammen28: Sammenligning,
    pub udfald: Vec<Udfald>,
    /// Er der spurgt? Så betyder en tom `udfald`, at der ingen kendte forstyrrelser er.
    pub udfald_hentet: bool,
    /// Forstyrrelserne i hele verden, nyeste først.
    pub udfald_verden: Vec<Udfald>,
    pub udfald_verden_hentet: bool,
    pub maal: Vec<NavnPct>,
    pub kilde: Vec<NavnPct>,
    pub produkt: Vec<NavnPct>,
    pub ai: Vec<NavnPct>,
    pub ai_navne: Vec<NavnPct>,
    /// Robotternes andel af HTTP-trafikken i procent.
    pub bot: Option<f64>,
    /// Kvantesikre HTTPS-forespørgsler i procent.
    pub kvante: Option<f64>,
    /// HTTP-trafikken i går som indeks.
    pub http_indeks: Option<f64>,
    /// Døgnet bag listerne under Verden.
    pub verden_dag: Option<String>,
    pub hentet: Option<String>,
    /// Fejlen fra seneste forsøg, som `cf::Fejl` sendes til webvisningen.
    pub fejl: Option<Value>,
    /// Serier, der ikke kunne bruges: en note ved deres afsnit, ikke det fælles banner.
    pub serie_fejl: Vec<String>,
}

/// Det, `regn` skal bruge.
#[derive(Debug, Default)]
pub struct Laest {
    pub verden: Vec<RadarRaekke>,
    pub land: Vec<RadarRaekke>,
    pub fw: Vec<Raekke>,
    /// Døgn, mindst ét af de valgte domæner har tal for.
    pub hentede: HashSet<String>,
    pub fejl: Option<String>,
    pub serie_fejl: Vec<String>,
}

/// Læser under låsen; `regn` regner bagefter. Omfanget er de valgte domæner.
pub fn laes(lager: &Store, kode: &str, nu: i64) -> Result<Laest, Fejl> {
    let zoner = lager.valg()?;
    let fra = overblik::hele_fra(nu);
    let mut hentede = HashSet::new();
    for z in &zoner {
        hentede.extend(lager.hentet(z)?.into_keys());
    }
    let serier = |noegle: String| -> Result<Vec<String>, Fejl> {
        Ok(lager
            .indstilling(&noegle)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default())
    };
    let serie_fejl = [serier(SERIE_FEJL.to_string())?, serier(format!("{SERIE_FEJL}:{kode}"))?].concat();
    Ok(Laest {
        verden: lager.radar_raekker("", &fra)?,
        land: lager.radar_raekker(kode, &fra)?,
        fw: lager.raekker_art(&zoner, &["fw"], &fra)?.into_iter().map(|(_, r)| r).collect(),
        hentede,
        fejl: lager.indstilling(FEJL)?.filter(|f| !f.is_empty()),
        serie_fejl,
    })
}

/// Det nyeste døgns rækker af serien, størst først.
fn nyeste<'a>(rows: &'a [RadarRaekke], serie: &str) -> Vec<&'a RadarRaekke> {
    let dag = rows.iter().filter(|r| r.serie == serie).map(|r| r.dag.as_str()).max();
    let mut rk: Vec<_> = rows.iter().filter(|r| r.serie == serie && Some(r.dag.as_str()) == dag).collect();
    rk.sort_by(|a, b| b.vaerdi.total_cmp(&a.vaerdi).then_with(|| a.noegle.cmp(&b.noegle)));
    rk
}

fn navne(rows: &[&RadarRaekke], antal: usize) -> Vec<NavnPct> {
    rows.iter()
        .take(antal)
        .map(|r| NavnPct {
            navn: r.noegle.clone(),
            vaerdi: r.vaerdi,
        })
        .collect()
}

/// Angreb pr. døgn på de valgte domæner, som `overblik::angreb` tæller dem (uden rutine).
fn angreb_pr_dag(fw: &[Raekke]) -> HashMap<&str, i64> {
    let mut n: HashMap<&str, i64> = HashMap::new();
    for r in fw {
        if !r.noegle.split('\t').nth(6).is_some_and(|k| overblik::RUTINE.contains(&k)) {
            *n.entry(&r.dag).or_default() += r.n;
        }
    }
    n
}

/// Landets kolonne: kilderne, størst først, og om der er spurgt. Ren funktion.
pub fn mod_land(land: &[RadarRaekke], serie: &str) -> (Vec<NavnPct>, bool) {
    let rk = nyeste(land, serie);
    let kilder: Vec<&RadarRaekke> = rk.iter().copied().filter(|r| !r.noegle.is_empty()).collect();
    (navne(&kilder, TOP_LANDE), !rk.is_empty())
}

/// `land`: landets procent i Radars kolonne. Står det ikke på listen over de største kilder, er den højst som den sidste på listen. Ren funktion.
pub fn saerligt(mit: f64, land: Option<f64>) -> bool {
    land.is_some_and(|v| mit >= SAERLIGT_MIN && mit >= SAERLIGT_FAKTOR * v)
}

/// Dine angreb pr. kildeland over de sidste `dage` hele døgn, som dyk med "Kun angreb" tæller dem, ved siden af landets kolonne. Ren funktion.
pub fn sammenlign(fw: &[Raekke], land: Vec<NavnPct>, spurgt: bool, nu: i64, dage: i64) -> Sammenligning {
    let (fra, til) = (foer(nu, dage), dag_noegle(nu - DOEGN_MS));
    let mut n: BTreeMap<&str, i64> = BTreeMap::new();
    for r in fw.iter().filter(|r| r.dag >= fra && r.dag <= til) {
        let mut f = r.noegle.split('\t');
        let kode = f.nth(1).unwrap_or("");
        if !f.nth(4).is_some_and(|k| overblik::RUTINE.contains(&k)) {
            *n.entry(kode).or_default() += r.n;
        }
    }
    let i_alt: i64 = n.values().sum();
    let bund = land.last().map(|x| x.vaerdi);
    let vaerdi = |k: &str| land.iter().find(|x| x.navn == k).map(|x| x.vaerdi).or(bund).unwrap_or(0.0);
    let mut mine: Vec<MitLand> = n
        .into_iter()
        .map(|(k, n)| {
            let pct = 100.0 * n as f64 / i_alt as f64;
            MitLand {
                navn: k.to_string(),
                vaerdi: pct,
                n,
                saerligt: !land.is_empty() && k != "XX" && tjek_land(k).is_ok() && saerligt(pct, Some(vaerdi(k))),
            }
        })
        .collect();
    mine.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.navn.cmp(&b.navn)));
    Sammenligning {
        rest: (!land.is_empty()).then(|| (100.0 - land.iter().map(|x| x.vaerdi).sum::<f64>()).max(0.0)),
        land,
        spurgt,
        mine,
        mine_i_alt: i_alt,
        fra,
        til,
    }
}

/// Kun når døgnet opfylder gul-reglen (`status_for_angreb`), må der stå noget i ord: 3 angreb i snit giver vilde indeks. Ren funktion.
pub fn ord(fw: &[Raekke], waf: &BTreeMap<String, f64>, dag: &str) -> Option<Ord> {
    let nu = dag_ms(dag)? + DOEGN_MS;
    let a = overblik::angreb(fw, nu);
    let status = overblik::status_for_angreb(a.i_gaar, a.snit());
    (status != Status::Groen).then(|| Ord {
        dag: dag.to_string(),
        angreb: a.i_gaar,
        snit: (a.snit() * 10.0).round() / 10.0,
        tendens: overblik::tendens(status, false, overblik::r_mig(fw, nu), forhold(waf, dag)),
    })
}

/// Forstyrrelserne i rækkerne, nyeste først. Ren funktion.
fn udfald_liste(rows: &[RadarRaekke]) -> Vec<Udfald> {
    let mut ud: Vec<Udfald> = rows
        .iter()
        .filter(|r| r.serie == UDFALD && !r.noegle.is_empty())
        .map(|r| {
            let f: Vec<&str> = r.noegle.split('\t').collect();
            let felt = |i: usize| f.get(i).copied().unwrap_or("").to_string();
            let slut = Some(felt(1)).filter(|s| !s.is_empty());
            let (net, lande) = (felt(4), felt(5));
            Udfald {
                minutter: slut
                    .as_deref()
                    .and_then(parse_iso)
                    .zip(parse_iso(&felt(0)))
                    .map(|(b, a)| (b - a) / 60_000),
                start: felt(0),
                slut,
                aarsag: felt(2),
                slags: felt(3),
                netvaerk: net.split(", ").filter(|n| !n.is_empty()).map(str::to_string).collect(),
                lande: lande.split(',').filter(|n| !n.is_empty()).map(str::to_string).collect(),
            }
        })
        .collect();
    ud.sort_by(|a, b| b.start.cmp(&a.start));
    ud
}

/// `waf`: verden er `mitigationProduct=WAF` i stedet for alle L7-angreb. Ren funktion.
pub fn regn(kode: &str, waf: bool, l: &Laest, nu: i64) -> LandUd {
    let dage = vindue(nu);
    let i_gaar = dag_noegle(nu - DOEGN_MS);
    let verden = verden_serie(&l.verden, if waf { L7_WAF } else { L7 });
    let waf_serie = verden_serie(&l.verden, L7_WAF);
    let n = angreb_pr_dag(&l.fw);
    let mit: Vec<Option<i64>> = dage
        .iter()
        .map(|d| l.hentede.contains(d).then(|| n.get(d.as_str()).copied().unwrap_or(0)))
        .collect();
    let mit_i = indeks(&mit.iter().map(|x| x.map(|v| v as f64)).collect::<Vec<_>>());
    let verden_i = indeks(&dage.iter().map(|d| verden.get(d).copied()).collect::<Vec<_>>());
    let ud: Vec<DagUd> = dage
        .iter()
        .enumerate()
        .map(|(i, d)| DagUd {
            dag: d.clone(),
            mit: mit[i],
            mit_indeks: mit_i[i],
            verden_indeks: verden_i[i],
        })
        .collect();
    let mut hentet_maal: Vec<&str> = l.verden.iter().filter(|r| r.serie == MAAL).map(|r| r.dag.as_str()).collect();
    hentet_maal.sort_unstable();
    hentet_maal.dedup();
    let andel = hentet_maal
        .iter()
        .rev()
        .map(|d| Andel {
            dag: d.to_string(),
            vaerdi: l
                .verden
                .iter()
                .find(|r| r.serie == MAAL && r.dag == *d && r.noegle == kode)
                .map(|r| r.vaerdi),
        })
        .collect();
    let ord_dag = waf_serie
        .keys()
        .next_back()
        .map_or(i_gaar.as_str(), String::as_str)
        .min(i_gaar.as_str());
    let http = verden_serie(&l.verden, HTTP);
    let http_i = indeks(&dage.iter().map(|d| http.get(d).copied()).collect::<Vec<_>>());
    let sammen = |serie, dage| {
        let (land, spurgt) = mod_land(&l.land, serie);
        sammenlign(&l.fw, land, spurgt, nu, dage)
    };
    let maal = nyeste(&l.verden, MAAL);
    LandUd {
        kode: kode.to_string(),
        ord: ord(&l.fw, &waf_serie, ord_dag),
        i_gaar,
        dage: ud,
        andel,
        sammen7: sammen(PAR7, 7),
        sammen28: sammen(PAR28, FIRE_UGER_DAGE),
        udfald: udfald_liste(&l.land),
        udfald_hentet: l.land.iter().any(|r| r.serie == UDFALD),
        udfald_verden: udfald_liste(&l.verden),
        udfald_verden_hentet: l.verden.iter().any(|r| r.serie == UDFALD),
        verden_dag: maal.first().map(|r| r.dag.clone()),
        maal: navne(&maal, TOP_LANDE),
        kilde: navne(&nyeste(&l.verden, KILDE), TOP_LANDE),
        produkt: navne(&nyeste(&l.verden, PRODUKT), TOP_LANDE),
        ai: navne(&nyeste(&l.verden, AI), TOP_LANDE),
        ai_navne: navne(&nyeste(&l.verden, AI_NAVN), TOP_LANDE),
        bot: nyeste(&l.verden, BOT).iter().find(|r| r.noegle == "bot").map(|r| r.vaerdi),
        kvante: nyeste(&l.verden, KVANTE).iter().find(|r| r.noegle == "SUPPORTED").map(|r| r.vaerdi),
        http_indeks: http_i.iter().rev().flatten().next().copied(),
        hentet: l.verden.iter().map(|r| r.hentet_utc.as_str()).max().map(str::to_string),
        fejl: l.fejl.as_deref().and_then(|f| serde_json::from_str(f).ok()),
        serie_fejl: l.serie_fejl.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cf::{tolk_radar, Zone};
    use crate::koe::Koe;
    use crate::overblik::{raekke, status_dag_test};
    use serde_json::json;

    /// 2026-11-03T08:08:00Z
    const NU: i64 = 1_793_693_280_000;
    const INGEN: Duration = Duration::ZERO;

    fn d(i: i64) -> String {
        dag_noegle(NU - i * DOEGN_MS)
    }

    /// Svarer efter stien, som Radar ville; `fejl`: hvert kald får den status (gennem den rigtige `tolk_radar`).
    #[derive(Default)]
    struct Falsk {
        fejl: Option<u16>,
        efter: Option<usize>,
        /// Stier, der indeholder dette, svarer med en form, der ikke kan læses.
        ulaeseligt: Option<&'static str>,
        /// Stier, der indeholder dette, afvises med HTTP-status.
        afvist: Option<(&'static str, u16)>,
        /// Det højeste `limitPerLocation`, hvor DK når med i parrene. 0: aldrig. `None`: 3.
        dk_ved: Option<usize>,
        /// DK står ikke blandt målene for en hel periode.
        uden_maal: bool,
        kald: Mutex<Vec<String>>,
        norm: Option<&'static str>,
    }

    impl Falsk {
        fn svar(&self, sti: &str) -> Value {
            let dag = sti.split("dateStart=").nth(1).map_or("", |s| &s[..10]).to_string();
            if sti.contains("/timeseries") {
                let t: Vec<String> = vindue(NU).iter().map(|d| format!("{d}T00:00:00Z")).collect();
                let v: Vec<String> = (1..=30).map(|i| format!("0.{i:02}")).collect();
                return json!({ "success": true, "result": { "meta": { "normalization": self.norm.unwrap_or(NORMALISERING) }, "serie_0": { "timestamps": t, "values": v } } });
            }
            if sti.contains("top/locations/target") {
                let dk = if dag == d(2) || (self.uden_maal && sti.contains("dateRange")) {
                    "SE"
                } else {
                    "DK"
                };
                return json!({ "success": true, "result": { "top_0": [
                    { "rank": 1, "targetCountryAlpha2": "US", "value": "31.5" },
                    { "rank": 2, "targetCountryAlpha2": dk, "value": "0.4" },
                ] } });
            }
            if sti.contains("top/attacks") {
                let n: usize = sti.split("limitPerLocation=").nth(1).map_or(0, |s| s[..1].parse().unwrap());
                let mut top = vec![json!({ "originCountryAlpha2": "CN", "targetCountryAlpha2": "US", "value": "5" })];
                if n <= self.dk_ved.unwrap_or(3) {
                    let dk = |o: &str, v: &str| json!({ "originCountryAlpha2": o, "targetCountryAlpha2": "DK", "value": v });
                    top.extend([dk("US", "0.2"), dk("NL", "0.1")]);
                }
                return json!({ "success": true, "result": { "top_0": top } });
            }
            if sti.contains("top/locations/origin") {
                return json!({ "success": true, "result": { "top_0": [{ "rank": 1, "originCountryAlpha2": "CN", "value": "13" }] } });
            }
            if sti.contains("annotations/outages") {
                return json!({ "success": true, "result": { "annotations": [
                    { "startDate": format!("{}T04:00:00Z", d(3)), "endDate": format!("{}T06:30:00Z", d(3)), "locations": ["DK"],
                      "asns": [64502], "asnsDetails": [{ "asn": "64502", "name": "Fjord Net", "location": { "code": "DK", "name": "Denmark" } }],
                      "outage": { "outageCause": "CABLE_CUT", "outageType": "NETWORK" } },
                    { "startDate": format!("{}T04:00:00Z", d(2)), "endDate": null, "locations": ["SE"], "outage": { "outageCause": "FIRE" } },
                    { "startDate": format!("{}T01:00:00Z", d(40)), "endDate": null, "locations": ["DK"], "asns": [64500], "outage": {} },
                ] } });
            }
            if sti.contains("post_quantum") {
                return json!({ "success": true, "result": { "summary_0": { "NOT_SUPPORTED": "62", "SUPPORTED": "38" } } });
            }
            if sti.contains("USER_AGENT") {
                return json!({ "success": true, "result": { "summary_0": { "GPTBot": "30", "Amazonbot": "10" } } });
            }
            if sti.contains("bot_class") {
                return json!({ "success": true, "result": { "summary_0": { "bot": "38.5", "human": "61.5" } } });
            }
            if sti.contains("mitigation_product") {
                return json!({ "success": true, "result": { "summary_0": { "DDOS": "54", "WAF": "33" } } });
            }
            json!({ "success": true, "result": { "summary_0": { "Training": "40", "Search": "60" } } })
        }
    }

    impl Kilde for Falsk {
        async fn hent(&self, sti: &str) -> Result<Value, Fejl> {
            let n = {
                let mut k = self.kald.lock().unwrap();
                k.push(sti.to_string());
                k.len()
            };
            if self.ulaeseligt.is_some_and(|u| sti.contains(u)) {
                return tolk_radar(200, Some(json!({ "success": true, "result": { "meta": {} } })));
            }
            if let Some((_, s)) = self.afvist.filter(|(a, _)| sti.contains(a)) {
                return tolk_radar(s, None);
            }
            match self.fejl.filter(|_| self.efter.is_none_or(|e| n > e)) {
                Some(s) => tolk_radar(s, Some(json!({ "success": false, "errors": [{ "message": "nej" }] }))),
                None => tolk_radar(200, Some(self.svar(sti))),
            }
        }
    }

    fn lager() -> Mutex<Store> {
        Mutex::new(Store::i_hukommelse().unwrap())
    }

    fn fw(dag: &str, kilde: &str, n: i64) -> Raekke {
        Raekke {
            dag: dag.into(),
            art: "fw".into(),
            noegle: ["Regel", "US", "AS", "/", "CPH", "GET", kilde, "block"].join("\t"),
            n,
        }
    }

    fn rr(serie: &str, dag: &str, noegle: &str, vaerdi: f64, norm: &str, hentet: &str) -> RadarRaekke {
        RadarRaekke {
            serie: serie.into(),
            dag: dag.into(),
            noegle: noegle.into(),
            vaerdi,
            normalisering: norm.into(),
            hentet_utc: hentet.into(),
        }
    }

    #[test]
    fn indeks_gennemsnit_er_100() {
        assert_eq!(indeks(&[Some(1.0), Some(3.0)]), [Some(50.0), Some(150.0)]);
        let v = indeks(&[Some(0.2), Some(0.4), Some(0.9)]);
        let snit = v.iter().flatten().sum::<f64>() / 3.0;
        assert!((snit - 100.0).abs() < 0.1, "{snit}");
        assert!(indeks(&[]).is_empty(), "tom serie");
        assert_eq!(indeks(&[None, None]), [None, None], "ingen tal");
        assert_eq!(indeks(&[Some(0.0), Some(0.0)]), [None, None], "kun 0 har intet indeks");
        assert_eq!(
            indeks(&[Some(0.0), Some(2.0)]),
            [Some(0.0), Some(200.0)],
            "et 0 tæller med i snittet"
        );
        assert_eq!(
            indeks(&[Some(1.0), None, Some(3.0)]),
            [Some(50.0), None, Some(150.0)],
            "et hul tæller ikke"
        );
    }

    #[test]
    fn tidsstempler_passer_til_hams_doegn() {
        assert_eq!(utc_dag("2026-11-03T00:00:00Z").as_deref(), Some("2026-11-03"));
        assert_eq!(utc_dag("2026-11-03T00:00:00.000Z").as_deref(), Some("2026-11-03"));
        assert_eq!(utc_dag("2026-11-03T12:00:00Z"), None, "ikke et helt døgn");
        assert_eq!(utc_dag("2026-11-03T00:00:00+01:00"), None, "anden tidszone");
        assert_eq!(utc_dag("2026-11-03"), None);
        let dage = vec!["2026-11-02".to_string(), "2026-11-03".to_string()];
        let j = json!({ "result": { "meta": { "normalization": "MIN0_MAX" }, "serie_0": {
            "timestamps": ["2026-11-01T00:00:00Z", "2026-11-02T00:00:00Z", "2026-11-03T00:00:00Z", "2026-11-04T00:00:00Z"],
            "values": ["0.1", "0.2", 0.3, "0.4"] } } });
        let (norm, p) = punkter(&j, &dage).unwrap();
        assert_eq!(norm, "MIN0_MAX");
        assert_eq!(
            p,
            [("2026-11-02".to_string(), 0.2), ("2026-11-03".to_string(), 0.3)],
            "kun døgnene i vinduet; det ufærdige i dag står ikke"
        );
    }

    #[tokio::test]
    async fn opdatering_gemmer_serier_og_landenes_andel() {
        let (l, k) = (lager(), Falsk::default());
        opdater(&l, &k, NU, INGEN).await.unwrap();
        let s = l.lock().unwrap();
        let rk = s.radar_raekker("", "2000-01-01").unwrap();
        assert_eq!(verden_serie(&rk, L7).len(), 30, "backfill 30 døgn");
        assert_eq!(verden_serie(&rk, L7_WAF).len(), 30);
        assert_eq!(s.radar_hentet(MAAL, "").unwrap().len(), 30, "ét kald pr. døgn første gang");
        let kald = k.kald.lock().unwrap();
        assert_eq!(kald.len(), 3 + 5 + 1 + 1 + 30);
        assert!(
            kald[1].contains("mitigationProduct=WAF") && kald[0].contains("normalization=MIN0_MAX") && kald[0].contains("aggInterval=1d")
        );
        assert!(kald[0].contains(&format!("dateStart={}T00:00:00Z", d(30))) && kald[0].contains(&format!("dateEnd={}T00:00:00Z", d(0))));
        assert!(kald.iter().all(|s| s.starts_with("radar/")), "kun Radar");
        assert!(
            !kald.iter().any(|s| s.contains("top/locations") && s.contains("location=")),
            "top/locations har ingen location"
        );
    }

    /// Kilde og mål må ikke byttes: `target` er målet, `origin` er kilden.
    #[tokio::test]
    async fn maal_er_target_og_kilde_er_origin() {
        let (l, k) = (lager(), Falsk::default());
        opdater(&l, &k, NU, INGEN).await.unwrap();
        let laest = laes(&l.lock().unwrap(), "DK", NU).unwrap();
        let u = regn("DK", false, &laest, NU);
        assert_eq!(u.maal[0].navn, "US");
        assert!(u.maal.iter().any(|x| x.navn == "DK"), "mål");
        assert_eq!(u.kilde.iter().map(|x| x.navn.as_str()).collect::<Vec<_>>(), ["CN"], "kilde");
        assert!(sti_maal("2026-11-02", "2026-11-03").contains("/target?") && sti_kilde("2026-11-02", "2026-11-03").contains("/origin?"));
    }

    /// Verden og landet, som `koer` henter dem.
    async fn hentet(k: &Falsk) -> LandUd {
        let l = lager();
        opdater(&l, k, NU, INGEN).await.ok();
        opdater_land(&l, k, "DK", NU, INGEN).await.ok();
        let laest = laes(&l.lock().unwrap(), "DK", NU).unwrap();
        regn("DK", false, &laest, NU)
    }

    /// Kilderne er angrebsparrene med landet som mål, delt med landets andel som mål for samme `dateRange`.
    #[tokio::test]
    async fn mod_landet_er_par_delt_med_maalandelen() {
        let k = Falsk::default();
        let u = hentet(&k).await;
        let kald = k.kald.lock().unwrap();
        let med = |d: &str| {
            kald.iter()
                .filter(|s| s.contains(&format!("dateRange={d}")) && !s.contains("annotations"))
                .collect::<Vec<_>>()
        };
        for range in ["7d", "28d"] {
            let s = med(range);
            assert_eq!(s.len(), 2, "{s:?}");
            assert_eq!(
                s[0],
                &format!("radar/attacks/layer7/top/locations/target?limit=100&dateRange={range}&format=json")
            );
            assert_eq!(
                s[1],
                &format!(
                    "radar/attacks/layer7/top/attacks?limit=200&limitDirection=TARGET&limitPerLocation=3&dateRange={range}&format=json"
                )
            );
        }
        assert!(!kald.iter().any(|s| s.contains("origin?location")), "Radar ignorerer location dér");
        assert!(!kald.iter().any(|s| s.contains("top/locations") && s.contains("location=")));
        let navne = |s: &Sammenligning| s.land.iter().map(|x| (x.navn.clone(), x.vaerdi)).collect::<Vec<_>>();
        let vent = [("US".to_string(), 50.0), ("NL".to_string(), 25.0)];
        assert_eq!(navne(&u.sammen7), vent, "0,2 ÷ 0,4 og 0,1 ÷ 0,4");
        assert_eq!(navne(&u.sammen28), vent);
        assert_eq!((u.sammen7.rest, u.sammen7.spurgt), (Some(25.0), true));
        assert_eq!((u.sammen7.fra.as_str(), u.sammen28.fra.as_str()), (d(7).as_str(), d(28).as_str()));
    }

    #[tokio::test]
    async fn landet_mangler_saa_proeves_faerre_kilder() {
        for (ved, kald_par) in [(3, 1), (2, 2), (1, 3)] {
            let k = Falsk {
                dk_ved: Some(ved),
                ..Falsk::default()
            };
            let u = hentet(&k).await;
            let kald = k.kald.lock().unwrap();
            let par: Vec<_> = kald
                .iter()
                .filter(|s| s.contains("top/attacks") && s.contains("dateRange=7d"))
                .collect();
            assert_eq!(par.len(), kald_par, "{par:?}");
            for (s, n) in par.iter().zip([3, 2, 1]) {
                assert!(s.contains(&format!("limitPerLocation={n}&")), "{s}");
            }
            assert_eq!(u.sammen7.land.len(), 2);
        }
    }

    /// Rang over 200: Radar oplyser det ikke, og det er noget andet end ikke at have spurgt.
    #[tokio::test]
    async fn landet_er_for_lille_i_statistikken() {
        let u = hentet(&Falsk {
            dk_ved: Some(0),
            ..Falsk::default()
        })
        .await;
        assert!(u.sammen7.land.is_empty() && u.sammen7.spurgt);
        assert_eq!(u.sammen7.rest, None);
        assert!(u.serie_fejl.is_empty(), "det er ikke en fejl");
        let k = Falsk {
            uden_maal: true,
            ..Falsk::default()
        };
        let u = hentet(&k).await;
        assert!(
            u.sammen28.land.is_empty() && u.sammen28.spurgt,
            "uden landets andel kan der ikke deles"
        );
        assert!(
            !k.kald.lock().unwrap().iter().any(|s| s.contains("top/attacks")),
            "så spørges der ikke"
        );
        assert!(!regn("DK", false, &Laest::default(), NU).sammen7.spurgt);
    }

    #[test]
    fn andelen_kan_ikke_overstige_hele() {
        let h = "2026-11-03T02:00:00.000Z";
        let land = [
            rr(PAR7, &d(1), "DE", 80.0, PROCENT, h),
            rr(PAR7, &d(1), "NL", 40.0, PROCENT, h),
            rr(PAR7, &d(1), "", 0.0, PROCENT, h),
        ];
        let (l, spurgt) = mod_land(&land, PAR7);
        assert_eq!((l.len(), spurgt), (2, true));
        assert_eq!(sammenlign(&[], l, spurgt, NU, 7).rest, Some(0.0), "resten er aldrig under 0");
        assert_eq!(mod_land(&[], PAR7), (Vec::new(), false));
    }

    /// 400 og 404 koster kun den ene serie; alle andre afsnit står.
    #[tokio::test]
    async fn fejl_i_een_serie_skjuler_kun_dens_afsnit() {
        for s in [400, 404] {
            let u = hentet(&Falsk {
                afvist: Some(("top/attacks", s)),
                ..Falsk::default()
            })
            .await;
            assert!(u.sammen7.land.is_empty() && !u.sammen7.spurgt, "{s}");
            assert!(u.udfald_hentet && !u.udfald.is_empty(), "{s}: forstyrrelserne står");
            assert!(u.kvante.is_some() && u.bot.is_some(), "{s}: Verden står");
            let u = hentet(&Falsk {
                afvist: Some(("post_quantum", s)),
                ..Falsk::default()
            })
            .await;
            assert_eq!(u.kvante, None, "{s}");
            assert!(
                u.bot.is_some() && !u.ai_navne.is_empty() && !u.sammen7.land.is_empty(),
                "{s}: resten står"
            );
            let u = hentet(&Falsk {
                afvist: Some(("annotations/outages", s)),
                ..Falsk::default()
            })
            .await;
            assert!(
                !u.udfald_hentet && !u.sammen7.land.is_empty(),
                "{s}: afsnittet kan ikke sige, at der intet var"
            );
        }
    }

    /// En serie, der ikke kan bruges, er en note ved afsnittet, ikke det fælles banner. En fejl, der stopper runden, er stadig banneret.
    #[tokio::test]
    async fn fejl_i_een_serie_er_ikke_det_faelles_banner() {
        let (l, k) = (
            lager(),
            Falsk {
                afvist: Some(("post_quantum", 400)),
                ..Falsk::default()
            },
        );
        koer(&l, &k, Some("DK"), NU, OPDATER_MS, INGEN).await.unwrap();
        let u = regn("DK", false, &laes(&l.lock().unwrap(), "DK", NU).unwrap(), NU);
        assert_eq!((u.fejl, u.serie_fejl), (None, vec![KVANTE.to_string()]));
        let (l, k) = (
            lager(),
            Falsk {
                afvist: Some(("top/attacks", 404)),
                ..Falsk::default()
            },
        );
        koer(&l, &k, Some("DK"), NU, OPDATER_MS, INGEN).await.unwrap();
        let u = regn("DK", false, &laes(&l.lock().unwrap(), "DK", NU).unwrap(), NU);
        assert_eq!((u.fejl, u.serie_fejl), (None, vec![PAR7.to_string(), PAR28.to_string()]));
        let (l, k) = (
            lager(),
            Falsk {
                fejl: Some(403),
                ..Falsk::default()
            },
        );
        koer(&l, &k, Some("DK"), NU, OPDATER_MS, INGEN).await.unwrap();
        assert!(regn("DK", false, &laes(&l.lock().unwrap(), "DK", NU).unwrap(), NU).fejl.is_some());
    }

    /// Verdens forstyrrelser står for sig, med landene, og blandes ikke med landets.
    #[tokio::test]
    async fn forstyrrelserne_i_verden() {
        let k = Falsk::default();
        let u = hentet(&k).await;
        assert!(k
            .kald
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.starts_with("radar/annotations/outages?dateRange=30d&limit=25")));
        assert!(u.udfald_verden_hentet);
        assert_eq!(u.udfald_verden.len(), 3, "også den svenske");
        assert_eq!(u.udfald_verden[0].lande, ["SE"], "nyeste først");
        assert_eq!(u.udfald.len(), 2);
        assert!(!regn("DK", false, &Laest::default(), NU).udfald_verden_hentet);
    }

    #[tokio::test]
    async fn verden_har_robotter_og_kvante_uden_location() {
        let k = Falsk::default();
        let u = hentet(&k).await;
        assert_eq!(u.kvante, Some(38.0));
        assert_eq!(u.bot, Some(38.5));
        assert_eq!(u.ai_navne[0].navn, "GPTBot");
        let kald = k.kald.lock().unwrap();
        for del in ["post_quantum", "bot_class", "USER_AGENT", "CRAWL_PURPOSE"] {
            let s = kald.iter().find(|s| s.contains(del)).unwrap();
            assert!(!s.contains("location="), "{del}: brugerens land er ikke sitets");
        }
    }

    /// Forstyrrelser: klarsprog kommer fra koderne, et andet land står ikke med, og mangler navnene, står AS-nummeret.
    #[tokio::test]
    async fn forstyrrelserne_i_landet() {
        let k = Falsk::default();
        let u = hentet(&k).await;
        assert!(k
            .kald
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.starts_with("radar/annotations/outages?location=DK&dateRange=30d")));
        assert!(u.udfald_hentet);
        assert_eq!(u.udfald.len(), 2, "svenske står ikke med: {:?}", u.udfald);
        let x = &u.udfald[0];
        assert_eq!(
            (x.minutter, x.aarsag.as_str(), x.slags.as_str()),
            (Some(150), "CABLE_CUT", "NETWORK")
        );
        assert_eq!(x.netvaerk, ["Fjord Net"]);
        let gammel = &u.udfald[1];
        assert_eq!((gammel.minutter, gammel.slut.as_deref()), (None, None), "uden slutdato");
        assert_eq!(gammel.netvaerk, ["AS64500"]);
    }

    /// Spurgt uden fund er noget andet end ikke spurgt.
    #[tokio::test]
    async fn ingen_forstyrrelser_er_ikke_det_samme_som_ikke_hentet() {
        struct Tom;
        impl Kilde for Tom {
            async fn hent(&self, _: &str) -> Result<Value, Fejl> {
                Ok(json!({ "success": true, "result": { "annotations": [] } }))
            }
        }
        let l = lager();
        opdater_land(&l, &Tom, "DK", NU, INGEN).await.ok();
        let u = regn("DK", false, &laes(&l.lock().unwrap(), "DK", NU).unwrap(), NU);
        assert!(u.udfald_hentet && u.udfald.is_empty());
        assert!(!regn("DK", false, &Laest::default(), NU).udfald_hentet);
    }

    #[test]
    fn forstyrrelse_daekker_dagen() {
        let h = "x";
        let raekke = |start: &str, slut: &str| {
            rr(
                UDFALD,
                "d",
                &format!("{start}\t{slut}\tFIRE\tNETWORK\tFjord Net, Bakke Net"),
                0.0,
                "",
                h,
            )
        };
        let m = raekke("2026-11-01T04:00:00Z", "2026-11-03T06:00:00Z");
        assert_eq!(
            forstyrrelse(std::slice::from_ref(&m), "DK", "2026-11-02"),
            Some(Forstyrrelse {
                land: "DK".into(),
                netvaerk: Some("Fjord Net".into())
            })
        );
        assert_eq!(forstyrrelse(std::slice::from_ref(&m), "DK", "2026-11-04"), None);
        let aaben = raekke("2026-11-01T04:00:00Z", "");
        assert!(forstyrrelse(std::slice::from_ref(&aaben), "DK", "2026-11-01").is_some());
        assert_eq!(forstyrrelse(&[aaben], "DK", "2026-11-02"), None, "uden slutdato kun startdagen");
        assert_eq!(
            forstyrrelse(&[rr(UDFALD, "d", "", 0.0, "", h)], "DK", "2026-11-02"),
            None,
            "den tomme nøgle er ingen forstyrrelse"
        );
    }

    fn fw_land(dag: &str, land: &str, kilde: &str, n: i64) -> Raekke {
        Raekke {
            dag: dag.into(),
            art: "fw".into(),
            noegle: ["Regel", land, "AS", "/", "CPH", "GET", kilde, "block"].join("\t"),
            n,
        }
    }

    fn np(navn: &str, vaerdi: f64) -> NavnPct {
        NavnPct { navn: navn.into(), vaerdi }
    }

    /// Procent af hele perioden: kolonnen summerer til 100, rutine tæller ikke, og 7 dage er ikke 30.
    #[test]
    fn dine_procenter_summerer_og_foelger_perioden() {
        let fw_ = vec![
            fw_land(&d(1), "US", "firewallManaged", 60),
            fw_land(&d(2), "PL", "firewallManaged", 30),
            fw_land(&d(3), "XX", "firewallManaged", 10),
            fw_land(&d(3), "CN", "botFight", 900),
            fw_land(&d(8), "FR", "firewallManaged", 100),
            fw_land(&d(0), "DE", "firewallManaged", 500),
        ];
        let s = sammenlign(&fw_, vec![np("US", 50.0)], false, NU, 7);
        assert_eq!(s.mine_i_alt, 100);
        assert_eq!(s.mine.iter().map(|x| x.navn.as_str()).collect::<Vec<_>>(), ["US", "PL", "XX"]);
        assert!((s.mine.iter().map(|x| x.vaerdi).sum::<f64>() - 100.0).abs() < 1e-9);
        let s = sammenlign(&fw_, vec![np("US", 50.0)], false, NU, 30);
        assert_eq!(s.mine_i_alt, 200, "30 dage har Frankrig med");
        assert!(
            sammenlign(&[], Vec::new(), false, NU, 7).mine.is_empty(),
            "ingen angreb, ingen deling med 0"
        );
    }

    #[test]
    fn saerligt_hos_dig_taersklerne() {
        assert!(saerligt(10.0, Some(5.0)), "præcis 2 ×");
        assert!(!saerligt(9.9, Some(5.0)), "under 2 ×");
        assert!(saerligt(5.0, Some(0.0)), "præcis 5 %");
        assert!(!saerligt(4.9, Some(0.0)), "under 5 %, selv om landet har 0");
        assert!(!saerligt(50.0, None), "intet at sammenligne med");
        let mit = |land: &str, n| fw_land(&d(1), land, "firewallManaged", n);
        let fw_ = vec![mit("US", 80), mit("PL", 16), mit("XX", 4)];
        let hvem = |s: &Sammenligning| s.mine.iter().filter(|x| x.saerligt).map(|x| x.navn.clone()).collect::<Vec<_>>();
        assert_eq!(
            hvem(&sammenlign(&fw_, vec![np("US", 50.0), np("DE", 8.0)], false, NU, 7)),
            ["PL"],
            "kun 80 % mod 50 % er ikke særligt"
        );
        assert!(
            hvem(&sammenlign(&fw_, Vec::new(), false, NU, 7)).is_empty(),
            "uden Radars kolonne ingen mærker"
        );
        let ukendt = vec![mit("XX", 80), mit("T1", 20)];
        assert!(
            hvem(&sammenlign(&ukendt, vec![np("US", 50.0)], false, NU, 7)).is_empty(),
            "XX og Tor er ikke lande"
        );
        // Står landet ikke blandt de største kilder, er dets andel højst den sidstes.
        let fuld: Vec<NavnPct> = (0..3).map(|i| np(&format!("A{}", (b'A' + i) as char), 8.0)).collect();
        assert!(
            hvem(&sammenlign(&fw_, fuld.clone(), false, NU, 7)).contains(&"PL".to_string()),
            "16 % ≥ 2 × 8 %"
        );
        let fw_lav = vec![mit("US", 85), mit("PL", 15)];
        assert!(
            !hvem(&sammenlign(&fw_lav, fuld, false, NU, 7)).contains(&"PL".to_string()),
            "15 % < 2 × 8 %"
        );
    }

    /// Landet uden for top 100 er ikke 0 %: det har ingen værdi. Et døgn, der ikke er hentet, står slet ikke.
    #[tokio::test]
    async fn land_uden_for_top_100() {
        let (l, k) = (lager(), Falsk::default());
        opdater(&l, &k, NU, INGEN).await.unwrap();
        let u = regn("DK", false, &laes(&l.lock().unwrap(), "DK", NU).unwrap(), NU);
        assert_eq!(u.andel.len(), 30);
        assert_eq!(
            u.andel[0],
            Andel {
                dag: d(1),
                vaerdi: Some(0.4)
            },
            "nyeste først"
        );
        assert_eq!(u.andel[1], Andel { dag: d(2), vaerdi: None }, "ikke i listen den dag");
        let ingen = regn("FO", false, &Laest::default(), NU);
        assert!(ingen.andel.is_empty(), "intet hentet er ikke det samme som uden for top 100");
    }

    #[tokio::test]
    async fn anden_gang_hentes_kun_det_ufaerdige() {
        let (l, k) = (lager(), Falsk::default());
        opdater(&l, &k, NU, INGEN).await.unwrap();
        k.kald.lock().unwrap().clear();
        opdater(&l, &k, NU + 2 * 3_600_000, INGEN).await.unwrap();
        let kald = k.kald.lock().unwrap();
        assert_eq!(
            kald.iter().filter(|s| s.contains("/timeseries")).count(),
            3,
            "hele vinduet igen: ét svar, én skala"
        );
        assert_eq!(
            kald.iter().filter(|s| s.contains("/target")).count(),
            0,
            "i går var færdig ved første hentning"
        );
    }

    /// Enhver Radar-fejl rammer kun visningen: køen står ikke, hverken status eller tal ændres, og fejlen står ved Radar.
    #[tokio::test]
    async fn radar_fejl_stopper_hverken_koeen_eller_aendrer_status() {
        let z = Zone {
            id: "z".into(),
            navn: "a.wi24rd.com".into(),
        };
        let rows: Vec<Raekke> = (2..=8)
            .map(|i| fw(&d(i), "firewallManaged", 10))
            .chain([fw(&d(1), "firewallManaged", 100)])
            .collect();
        let hentet = status_dag_test(NU, false);
        for status in [401, 403, 429, 500] {
            let (l, koe) = (lager(), Koe::default());
            let k = Falsk {
                fejl: Some(status),
                ..Falsk::default()
            };
            let foer = raekke(&z, &hentet, &rows, NU);
            let svar = koer(&l, &k, Some("DK"), NU, OPDATER_MS, INGEN).await;
            assert_eq!(svar, Ok(true), "{status}: fejlen noteres, den kastes ikke videre");
            assert!(!koe.stoppet(), "{status}");
            assert_eq!(koe.status(), None, "{status}: køen hører intet");
            let u = regn("DK", false, &laes(&l.lock().unwrap(), "DK", NU).unwrap(), NU);
            assert_eq!(u.fejl, Some(json!({ "kode": "radar", "status": status })), "{status}");
            let mut efter = raekke(&z, &hentet, &rows, NU);
            efter.tendens = tendens_for(
                &efter,
                &rows,
                &verden_serie(&l.lock().unwrap().radar_raekker("", "2000-01-01").unwrap(), L7_WAF),
            );
            assert_eq!(efter, foer, "{status}: status og tal er de samme, og der står ingen sammenligning");
            assert_eq!(
                k.kald.lock().unwrap().len(),
                2,
                "{status}: verden stopper ved første fejl, landet prøves for sig"
            );
        }
    }

    /// En fejl midt i gemmer det, der nåede frem, og næste forsøg fjerner noten. Et svar, der ikke kan læses, er også kun Radars fejl.
    #[tokio::test]
    async fn fejl_midt_i_og_naeste_forsoeg() {
        let l = lager();
        let k = Falsk {
            fejl: Some(500),
            efter: Some(1),
            ..Falsk::default()
        };
        koer(&l, &k, None, NU, OPDATER_MS, INGEN).await.unwrap();
        assert_eq!(
            verden_serie(&l.lock().unwrap().radar_raekker("", "2000-01-01").unwrap(), L7).len(),
            30
        );
        assert_eq!(
            koer(&l, &k, None, NU + 60_000, OPDATER_MS, INGEN).await,
            Ok(false),
            "ingen nye forsøg hvert minut"
        );
        assert_eq!(
            koer(&l, &Falsk::default(), None, NU + OPDATER_MS, OPDATER_MS, INGEN).await,
            Ok(true)
        );
        assert!(
            regn("DK", false, &laes(&l.lock().unwrap(), "DK", NU).unwrap(), NU).fejl.is_none(),
            "noten er væk"
        );
        struct Tom;
        impl Kilde for Tom {
            async fn hent(&self, _: &str) -> Result<Value, Fejl> {
                Ok(json!({ "success": true, "result": {} }))
            }
        }
        assert_eq!(opdater(&lager(), &Tom, NU, INGEN).await.map(|v| v.len()), Ok(3 + 5 + 1 + 1 + 1));
    }

    /// Et svar, der ikke kan læses, koster kun den serie: HTTP-svarets form står ikke i dokumentationen.
    #[tokio::test]
    async fn en_ulaeselig_serie_stopper_ikke_de_andre() {
        let (l, k) = (
            lager(),
            Falsk {
                ulaeseligt: Some("radar/http/timeseries"),
                ..Falsk::default()
            },
        );
        assert_eq!(opdater(&l, &k, NU, INGEN).await, Ok(vec![HTTP]), "den står stadig som mangler");
        let s = l.lock().unwrap();
        assert_eq!(verden_serie(&s.radar_raekker("", "2000-01-01").unwrap(), L7_WAF).len(), 30);
        assert_eq!(s.radar_hentet(MAAL, "").unwrap().len(), 30);
        assert!(s.radar_hentet(HTTP, "").unwrap().is_empty());
    }

    #[test]
    fn hver_6_time_og_1_time_ved_aabning() {
        let t = iso(NU);
        assert!(skal_opdatere(None, NU, OPDATER_MS), "aldrig hentet");
        assert!(skal_opdatere(Some("noget"), NU, OPDATER_MS));
        assert!(!skal_opdatere(Some(&t), NU + OPDATER_MS - 1, OPDATER_MS));
        assert!(skal_opdatere(Some(&t), NU + OPDATER_MS, OPDATER_MS));
        assert!(!skal_opdatere(Some(&t), NU + AABEN_MS - 1, AABEN_MS));
        assert!(skal_opdatere(Some(&t), NU + AABEN_MS + 1, AABEN_MS));
        assert!(!skal_opdatere(Some(&t), NU - 5000, AABEN_MS), "uret stillet tilbage");
    }

    /// `MIN0_MAX` gælder pr. svar: to hentninger blandes ikke, og anden normalisering indgår ikke i et indeks.
    #[test]
    fn kun_een_skala() {
        let rk = [
            rr(L7, "2026-10-01", "", 1.0, NORMALISERING, "2026-10-02T00:00:00.000Z"),
            rr(L7, "2026-10-03", "", 0.5, NORMALISERING, "2026-10-04T00:00:00.000Z"),
            rr(L7, "2026-10-04", "", 0.25, NORMALISERING, "2026-10-04T00:00:00.000Z"),
            rr(L7, "2026-10-05", "", 90.0, "PERCENTAGE_CHANGE", "2026-10-05T00:00:00.000Z"),
            rr(L7_WAF, "2026-10-05", "", 1.0, NORMALISERING, "2026-10-05T00:00:00.000Z"),
        ];
        let s = verden_serie(&rk, L7);
        assert_eq!(s.keys().map(String::as_str).collect::<Vec<_>>(), ["2026-10-03", "2026-10-04"]);
    }

    #[test]
    fn forhold_er_doegnet_mod_de_7_foer() {
        let mut s: BTreeMap<String, f64> = (2..=8).map(|i| (d(i), 0.5)).collect();
        assert_eq!(forhold(&s, &d(1)), None, "døgnet mangler");
        s.insert(d(1), 1.0);
        assert_eq!(forhold(&s, &d(1)), Some(2.0));
        s.remove(&d(5));
        assert_eq!(forhold(&s, &d(1)), None, "et hul i ugen før");
        let nul: BTreeMap<String, f64> = (1..=8).map(|i| (d(i), 0.0)).collect();
        assert_eq!(forhold(&nul, &d(1)), None);
    }

    /// Hvorfor: sætningen kommer kun, når Radar har døgnet, og den ændrer ikke status.
    #[test]
    fn tendens_kun_med_radar_og_uden_ny_status() {
        let z = Zone {
            id: "z".into(),
            navn: "a.wi24rd.com".into(),
        };
        let rows: Vec<Raekke> = (2..=8)
            .map(|i| fw(&d(i), "firewallManaged", 10))
            .chain([fw(&d(1), "firewallManaged", 30)])
            .collect();
        let r = raekke(&z, &status_dag_test(NU, false), &rows, NU);
        assert_eq!(r.status, Status::Gul);
        let mut waf: BTreeMap<String, f64> = (2..=8).map(|i| (d(i), 0.5)).collect();
        assert_eq!(tendens_for(&r, &rows, &waf), None, "Radar mangler døgnet");
        waf.insert(d(1), 1.0);
        assert_eq!(tendens_for(&r, &rows, &waf), Some(Tendens::Foelger));
        waf.insert(d(1), 0.55);
        assert_eq!(tendens_for(&r, &rows, &waf), Some(Tendens::SaerligtDig));
    }

    /// Gul på grund af serveren siger intet om angrebene, så der kommer ingen sammenligning.
    #[test]
    fn gul_fra_serveren_giver_ingen_tendens() {
        let z = Zone {
            id: "z".into(),
            navn: "a.wi24rd.com".into(),
        };
        let status = |n| Raekke {
            dag: d(1),
            art: "status".into(),
            noegle: "522".into(),
            n,
        };
        let rows: Vec<Raekke> = (2..=8)
            .map(|i| fw(&d(i), "firewallManaged", 10))
            .chain([fw(&d(1), "firewallManaged", 11), status(60)])
            .collect();
        let r = raekke(&z, &status_dag_test(NU, false), &rows, NU);
        assert_eq!(r.status, Status::Gul);
        let waf: BTreeMap<String, f64> = (2..=8).map(|i| (d(i), 0.5)).chain([(d(1), 1.0)]).collect();
        assert_eq!(tendens_for(&r, &rows, &waf), None);
    }

    /// Små tal: kurven er der, men der står intet i ord.
    #[test]
    fn smaa_tal_giver_ingen_saetning() {
        let waf: BTreeMap<String, f64> = (1..=8).map(|i| (d(i), 0.5)).collect();
        let lille: Vec<Raekke> = (2..=8)
            .map(|i| fw(&d(i), "firewallManaged", 3))
            .chain([fw(&d(1), "firewallManaged", 9)])
            .collect();
        assert_eq!(ord(&lille, &waf, &d(1)), None, "tre gange så mange, men kun 9");
        let stor: Vec<Raekke> = (2..=8)
            .map(|i| fw(&d(i), "firewallManaged", 10))
            .chain([fw(&d(1), "firewallManaged", 40)])
            .collect();
        let o = ord(&stor, &waf, &d(1)).unwrap();
        assert_eq!((o.angreb, o.snit, o.tendens), (40, 10.0, Some(Tendens::SaerligtDig)));
        let laest = Laest {
            fw: lille,
            hentede: (1..=30).map(d).collect(),
            ..Laest::default()
        };
        let u = regn("DK", false, &laest, NU);
        assert!(u.ord.is_none());
        assert!(u.dage.iter().any(|x| x.mit_indeks.is_some()), "kurven tegnes stadig");
    }

    #[test]
    fn mine_tal_er_summen_uden_rutine_og_huller_er_huller() {
        let fw_ = vec![
            fw(&d(1), "firewallManaged", 5),
            fw(&d(1), "firewallManaged", 7),
            fw(&d(1), "botFight", 900),
            fw(&d(3), "firewallCustom", 2),
        ];
        let laest = Laest {
            fw: fw_,
            hentede: [d(1), d(2), d(3)].into_iter().collect(),
            ..Laest::default()
        };
        let u = regn("DK", false, &laest, NU);
        let dag = |i: i64| u.dage.iter().find(|x| x.dag == d(i)).unwrap();
        assert_eq!(u.dage.len(), 30);
        assert_eq!(dag(1).mit, Some(12), "summen af domænerne, uden rutine");
        assert_eq!(dag(2).mit, Some(0), "hentet uden angreb");
        assert_eq!(dag(4).mit, None, "ikke hentet");
        assert_eq!(dag(4).mit_indeks, None);
    }

    #[test]
    fn verden_kan_vaere_waf() {
        let h = "2026-11-03T02:00:00.000Z";
        let verden = (1..=3)
            .flat_map(|i| {
                [
                    rr(L7, &d(i), "", i as f64, NORMALISERING, h),
                    rr(L7_WAF, &d(i), "", 1.0, NORMALISERING, h),
                ]
            })
            .collect();
        let l = Laest {
            verden,
            ..Laest::default()
        };
        assert_eq!(regn("DK", false, &l, NU).dage[29].verden_indeks, Some(50.0));
        assert_eq!(regn("DK", true, &l, NU).dage[29].verden_indeks, Some(100.0));
    }

    fn rk(zone: &str, i: i64, art: &str, noegle: &str, n: i64) -> (String, Raekke) {
        (
            zone.into(),
            Raekke {
                dag: d(i),
                art: art.into(),
                noegle: noegle.into(),
                n,
            },
        )
    }

    /// Almindelige døgn på ét domæne: `dk` besøgende fra Danmark og `us` fra USA, heraf `bot` robotter.
    fn doegn(dk: i64, us: i64, bot: i64) -> Vec<(String, Raekke)> {
        (1..=7)
            .flat_map(|i| {
                [
                    rk("z", i, "alle", "", dk + us),
                    rk("z", i, "land", "DK", dk),
                    rk("z", i, "land", "US", us),
                    rk("z", i, "bot", "x", bot),
                ]
            })
            .collect()
    }

    /// Ét land med mindst halvdelen, og ikke det valgte.
    #[test]
    fn forslag_kraever_halvdelen() {
        assert_eq!(forslag(&doegn(60, 40, 0), None).as_deref(), Some("DK"));
        assert_eq!(
            forslag(&doegn(50, 50, 0), None).as_deref(),
            Some("DK"),
            "præcis 50 %, ens: efter kode"
        );
        assert_eq!(forslag(&doegn(60, 40, 0), Some("DK")), None, "det valgte foreslås ikke");
        assert_eq!(forslag(&doegn(60, 40, 0), Some("SE")).as_deref(), Some("DK"));
        let spredt: Vec<_> = (1..=7)
            .flat_map(|i| {
                [
                    rk("z", i, "alle", "", 100),
                    rk("z", i, "land", "US", 49),
                    rk("z", i, "land", "DK", 26),
                    rk("z", i, "land", "FR", 25),
                ]
            })
            .collect();
        assert_eq!(forslag(&spredt, None), None, "49 % er ikke tydeligt");
    }

    /// Flest USA-forespørgsler, men robotterne står for dem. Art `bot` har intet land, så de trækkes fra det største.
    #[test]
    fn forslag_er_ikke_drevet_af_robotter() {
        assert_eq!(
            forslag(&doegn(20, 80, 0), Some("DK")).as_deref(),
            Some("US"),
            "uden robotter er USA tydeligt"
        );
        assert_eq!(forslag(&doegn(20, 80, 70), Some("DK")), None, "kun 10 af de 80 er ikke robotter");
        assert_eq!(forslag(&doegn(20, 80, 900), Some("DK")), None, "aldrig under 0");
    }

    /// Et trafikspring tæller ikke, og det vejer ellers tungest.
    #[test]
    fn forslag_uden_trafikspring() {
        let mut rows = doegn(60, 40, 0);
        rows.extend([
            rk("z", 8, "alle", "", 1_000_000),
            rk("z", 8, "land", "US", 999_000),
            rk("z", 8, "land", "DK", 1000),
        ]);
        assert_eq!(forslag(&rows, None).as_deref(), Some("DK"));
        rows.push(rk("z", 9, "alle", "", 999));
        rows.push(rk("z", 9, "land", "US", 999));
        assert_eq!(forslag(&rows, None).as_deref(), Some("US"), "under 10 × medianen tæller døgnet");
    }

    /// Medianen er pr. domæne: et stort domæne gør ikke et lille til et spring, og omvendt.
    #[test]
    fn forslag_median_pr_domaene() {
        let mut rows = doegn(60, 40, 0);
        for i in 1..=7 {
            rows.extend([rk("stor", i, "alle", "", 5000), rk("stor", i, "land", "DK", 5000)]);
        }
        assert_eq!(forslag(&rows, Some("US")).as_deref(), Some("DK"));
    }

    /// Ukendte koder er aldrig en fejl i forslaget: de kan ikke foreslås, men tæller i helheden.
    #[test]
    fn forslag_og_landekoder() {
        let med = |k: &str, n| {
            vec![
                rk("z", 1, "alle", "", 100),
                rk("z", 2, "alle", "", 100),
                rk("z", 3, "alle", "", 100),
                rk("z", 1, "land", k, n),
                rk("z", 1, "land", "DK", 100 - n),
            ]
        };
        assert_eq!(forslag(&med("XX", 90), None), None, "ukendt fylder 90 %");
        assert_eq!(forslag(&med("T1", 90), None), None, "Tor");
        assert_eq!(forslag(&med("", 90), None), None);
        assert_eq!(forslag(&[rk("z", 1, "alle", "", 9), rk("z", 1, "land", "XX", 9)], None), None);
        assert_eq!(forslag(&[], None), None);
        assert_eq!(median(&mut []), 0.0);
        assert_eq!(median(&mut [3, 1, 2]), 2.0);
        assert_eq!(median(&mut [4, 1, 2, 3]), 2.5);
        assert_eq!(tjek_land("DK"), Ok("DK".to_string()));
        for nej in ["dk", "D", "DKK", "419", "", "D1", "Æ"] {
            assert_eq!(tjek_land(nej), Err(Fejl::UgyldigtFilter), "{nej}");
        }
    }

    #[test]
    fn mangler_er_uhentet_eller_ufaerdigt() {
        let dage = vec![d(2), d(1)];
        let hentet = HashMap::from([(d(2), iso(NU)), (d(1), iso(NU - 9 * 3_600_000))]);
        assert_eq!(mangler(&dage, &hentet), [d(1)], "hentet før døgnet var slut");
        assert_eq!(mangler(&dage, &HashMap::new()), dage);
    }
}
