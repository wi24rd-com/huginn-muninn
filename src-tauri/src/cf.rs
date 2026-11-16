//! HTTP-klienten mod Cloudflare. Kun https://api.cloudflare.com/client/v4/ (HTTPS, ingen omdirigeringer), kun læsning.
//! GraphQL-værdier sendes altid som variabler, aldrig flettet ind i forespørgselsteksten. Alle GraphQL-kald (siden og
//! hentekøen) går gennem én fælles begrænser, så programmet holder sig inden for Cloudflares kvote.
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde::ser::SerializeMap;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::{Semaphore, SemaphorePermit};
use tokio::time::Instant;

use crate::token::Hemmelig;
use crate::vaern::{dag_noegle, doegn_variabler, DOEGN_MS};

pub const BASIS: &str = "https://api.cloudflare.com/client/v4/";
const MAKS_ZONESIDER: u32 = 20;

// ── Begrænserens tempo (Cloudflare: 300 GraphQL-forespørgsler pr. 5 min pr. bruger) ──────────────────────────────────
/// Højst så mange GraphQL-kald i luften på én gang, for alle kaldere tilsammen.
pub const MAKS_SAMTIDIGE: usize = 2;
/// Mindst så lang tid mellem to kaldstarter (≈ 270 kald pr. 5 min) …
pub const MELLEMRUM: Duration = Duration::from_millis(1100);
/// … og efter Cloudflare første gang har bedt os vente, resten af kørslen (1 kald pr. 7,5 s).
pub const MELLEMRUM_LANGSOM: Duration = Duration::from_millis(7500);
/// Pause ved 429 eller tom kvote uden `Retry-After`: 300 s + 0–30 s tilfældigt.
pub const KVOTEPAUSE: Duration = Duration::from_secs(300);
const KVOTEPAUSE_TILFAELDIG_MS: u64 = 30_000;
/// Pauser ved 5xx (også 503 "too many queries in progress"), i sekunder; derefter den sidste igen.
pub const NEDE_TRIN: [u64; 4] = [15, 30, 60, 120];
/// Er der mindre end så mange procent tilbage af kvoten i `Ratelimit`, går begrænseren i det langsomme tempo, til vinduet nulstilles.
pub const LAV_KVOTE_PROCENT: i64 = 10;

// ── Fejl ───────────────────────────────────────────────────────────────────────────────────────────────────────────
/// Alle fejl, der kan nå webvisningen: en lukket liste. Webvisningen får en fejlkode (`kode`) og oversætter den selv til
/// dansk eller engelsk efter brugerens sprogvalg. Bibliotekernes egne fejltekster (reqwest, rusqlite, keyring, serde)
/// gemmes aldrig. Det eneste fritekstfelt (`detalje`) er Cloudflares egen besked og passerer `rens_fejl` i både
/// `Serialize`, `Display` og `Debug`.
#[derive(Clone, PartialEq)]
pub enum Fejl {
    /// Forbindelse, DNS, TLS, afbrudt forbindelse eller timeout.
    Forbindelse,
    /// HTTP 401/403.
    Afvist(u16),
    /// HTTP 429.
    Vent,
    /// HTTP 5xx.
    CfNede(u16),
    /// Anden HTTP-status uden besked fra Cloudflare.
    Http(u16),
    /// REST-svar med Cloudflares egen fejlbesked.
    Api(String),
    /// Cloudflare Radar svarede ikke (HTTP-status, hvis der kom et svar). Rammer kun Radar-visningen: det er aldrig `Afvist`, for
    /// et afvist Radar-kald siger intet om adgangsnøglen til brugerens egne tal.
    Radar(Option<u16>),
    /// GraphQL-fejl; `dag` sættes af Værnet for det døgn, der fejlede.
    GraphQl {
        dag: Option<String>,
        detalje: String,
    },
    /// Svaret kunne ikke læses (ikke JSON, manglende eller forkerte felter).
    Svar,
    ZoneUkendt,
    IntetToken,
    TokenTomt,
    TokenForm,
    /// `result.status` fra /user/tokens/verify, fx `disabled`.
    TokenInaktiv(String),
    /// Tokenet er aktivt, men `Zone › Zone › Read` mangler, eller det ser ingen domæner (`mangler_ret_zoner`).
    ManglerZone,
    /// Tokenet ser domæner, men `Zone › Analytics › Read` mangler (`mangler_ret_analytics`).
    ManglerAnalytics,
    UgyldigZone,
    UgyldigDato,
    /// `dyk`: filteret er for langt, ukendt eller står to gange.
    UgyldigtFilter,
    /// Domænet har allerede `login::MAKS_EGNE_STIER` egne login-stier.
    ForMangeStier,
    Noegle,
    Lager,
    NyereDatabase(i64),
    /// Start ved login kunne ikke læses eller ændres (LaunchAgent / Run-nøglen).
    Autostart,
    Intern,
}

impl Fejl {
    pub fn kode(&self) -> &'static str {
        match self {
            Fejl::Forbindelse => "forbindelse",
            Fejl::Afvist(_) => "afvist",
            Fejl::Vent => "vent",
            Fejl::CfNede(_) => "cf_nede",
            Fejl::Http(_) => "http",
            Fejl::Api(_) => "api",
            Fejl::Radar(_) => "radar",
            Fejl::GraphQl { .. } => "graphql",
            Fejl::Svar => "svar",
            Fejl::ZoneUkendt => "zone_ukendt",
            Fejl::IntetToken => "intet_token",
            Fejl::TokenTomt => "token_tomt",
            Fejl::TokenForm => "token_form",
            Fejl::TokenInaktiv(_) => "token_inaktiv",
            Fejl::ManglerZone => "mangler_zone",
            Fejl::ManglerAnalytics => "mangler_analytics",
            Fejl::UgyldigZone => "ugyldig_zone",
            Fejl::UgyldigDato => "ugyldig_dato",
            Fejl::UgyldigtFilter => "ugyldigt_filter",
            Fejl::ForMangeStier => "for_mange_stier",
            Fejl::Noegle => "noegle",
            Fejl::Lager => "lager",
            Fejl::NyereDatabase(_) => "nyere_database",
            Fejl::Autostart => "autostart",
            Fejl::Intern => "intern",
        }
    }

    fn status(&self) -> Option<u16> {
        match self {
            Fejl::Afvist(s) | Fejl::CfNede(s) | Fejl::Http(s) => Some(*s),
            Fejl::Radar(s) => *s,
            _ => None,
        }
    }

    fn detalje(&self) -> Option<String> {
        match self {
            Fejl::Api(d) | Fejl::GraphQl { detalje: d, .. } | Fejl::TokenInaktiv(d) => Some(rens_fejl(d)),
            Fejl::NyereDatabase(v) => Some(v.to_string()),
            _ => None,
        }
    }

    /// Cloudflare har ikke længere data for døgnet (uden for opbevaringen, fx "cannot request data older than …"). Det
    /// døgn springes over; det er ikke en fejl for hele zonen.
    pub fn uden_for_opbevaring(&self) -> bool {
        match self {
            Fejl::GraphQl { detalje: d, .. } | Fejl::Api(d) => {
                let d = d.to_lowercase();
                d.contains("older than") || d.contains("retention")
            }
            _ => false,
        }
    }

    /// GraphQL-fejlen får det døgn, den gælder; andre fejl er uændrede.
    pub fn for_dag(self, d: &str) -> Fejl {
        match self {
            Fejl::GraphQl { detalje, .. } => Fejl::GraphQl {
                dag: Some(d.to_string()),
                detalje,
            },
            f => f,
        }
    }
}

/// HTTP-status → fejl (`None` for 2xx). `besked` er Cloudflares `errors[0].message`, hvis svaret havde en.
pub fn status_fejl(status: u16, besked: Option<String>) -> Option<Fejl> {
    match status {
        200..=299 => None,
        401 | 403 => Some(Fejl::Afvist(status)),
        429 => Some(Fejl::Vent),
        500..=599 => Some(Fejl::CfNede(status)),
        _ => Some(match besked {
            Some(m) => Fejl::Api(format!("HTTP {status}: {m}")),
            None => Fejl::Http(status),
        }),
    }
}

/// Svaret på et Radar-kald → kroppen eller `Fejl::Radar`. Går bevidst uden om `status_fejl`: 401/403 må ikke blive `Afvist` og stoppe
/// hentekøen (`koe.rs`), og 429/5xx må ikke sætte den fælles begrænser på pause. Ren funktion.
pub fn tolk_radar(status: u16, krop: Option<Value>) -> Result<Value, Fejl> {
    if !(200..=299).contains(&status) {
        return Err(Fejl::Radar(Some(status)));
    }
    let j = krop.ok_or(Fejl::Radar(None))?;
    if j.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(Fejl::Radar(None));
    }
    Ok(j)
}

/// Til webvisningen: `{kode, status?, dag?, detalje?}`.
impl Serialize for Fejl {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("kode", self.kode())?;
        if let Some(st) = self.status() {
            m.serialize_entry("status", &st)?;
        }
        if let Fejl::GraphQl { dag: Some(d), .. } = self {
            m.serialize_entry("dag", d)?;
        }
        if let Some(d) = self.detalje() {
            m.serialize_entry("detalje", &d)?;
        }
        m.end()
    }
}

/// Teknisk form (kode, status, døgn, renset detalje). Teksterne til brugeren ligger i webvisningen (`sprog.js`).
impl fmt::Display for Fejl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.kode())?;
        if let Some(st) = self.status() {
            write!(f, " {st}")?;
        }
        if let Fejl::GraphQl { dag: Some(d), .. } = self {
            write!(f, " {d}")?;
        }
        match self.detalje() {
            Some(d) => write!(f, ": {d}"),
            None => Ok(()),
        }
    }
}

impl fmt::Debug for Fejl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fejl({self})")
    }
}

impl std::error::Error for Fejl {}

static BEARER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)bearer\s+\S+").unwrap());
static TOKENAGTIG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z0-9_\-]{32,}").unwrap());

/// Fjerner styretegn, `Bearer …` og alt, der ligner et token (32+ tegn af bogstaver, tal, _ og -), og afkorter.
pub fn rens_fejl(s: &str) -> String {
    let s: String = s.chars().filter(|c| !c.is_control()).collect();
    let s = BEARER.replace_all(&s, "Bearer <TOKEN>");
    let s = TOKENAGTIG.replace_all(&s, "<TOKEN>");
    s.chars().take(300).collect()
}

// ── Begrænseren ────────────────────────────────────────────────────────────────────────────────────────────────────
/// Cloudflares egne tal for kvoten: `Ratelimit: "list_zones";r=1200;t=1` og `Ratelimit-Policy: "list_zones";q=1201;w=300`. Dokumentationen lover dem kun på REST, ikke på GraphQL; de læses på alle svar og bruges, hvis de er der.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kvote {
    /// Kald tilbage i vinduet.
    pub rest: i64,
    /// Sekunder, til vinduet nulstilles.
    pub nulstil_s: u64,
    /// Kald pr. vindue.
    pub kvote: i64,
    /// Vinduets længde i sekunder.
    pub vindue_s: u64,
}

/// `"navn";a=1;b=2, "andet";a=3` → (navn, nøgle → tal).
fn kvote_poster(h: &str) -> Vec<(String, std::collections::HashMap<String, i64>)> {
    h.split(',')
        .filter_map(|post| {
            let mut dele = post.split(';').map(str::trim);
            let navn = dele.next()?.trim_matches('"').to_string();
            let tal = dele
                .filter_map(|d| {
                    let (k, v) = d.split_once('=')?;
                    Some((k.trim().to_string(), v.trim().parse().ok()?))
                })
                .collect();
            Some((navn, tal))
        })
        .collect()
}

/// De to headere → den strammeste kvote, altså den med mindst tilbage i forhold til sin størrelse. Mangler eller forstås en af dem ikke: `None`, som i dag. Ren funktion.
pub fn laes_kvote(ratelimit: Option<&str>, politik: Option<&str>) -> Option<Kvote> {
    let (rate, pol) = (kvote_poster(ratelimit?), kvote_poster(politik?));
    rate.iter()
        .filter_map(|(navn, r)| {
            let p = &pol.iter().find(|x| x.0 == *navn)?.1;
            let k = Kvote {
                rest: *r.get("r")?,
                nulstil_s: u64::try_from(*r.get("t")?).ok()?,
                kvote: *p.get("q")?,
                vindue_s: u64::try_from(*p.get("w")?).ok()?,
            };
            (k.kvote > 0 && k.vindue_s > 0).then_some(k)
        })
        .min_by_key(|k| k.rest * 1000 / k.kvote)
}

fn kvote_af(h: &reqwest::header::HeaderMap) -> Option<Kvote> {
    let tekst = |navn: &str| h.get(navn).and_then(|v| v.to_str().ok());
    laes_kvote(tekst("ratelimit"), tekst("ratelimit-policy"))
}

fn retry_after_af(h: &reqwest::header::HeaderMap) -> Option<u64> {
    h.get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse().ok())
}

/// Et GraphQL-svar, før det er tolket: status, `Retry-After` (sekunder), kvoten og kroppen som JSON, hvis den var det.
#[derive(Debug, Clone)]
pub struct Raasvar {
    pub status: u16,
    pub retry_after: Option<u64>,
    pub kvote: Option<Kvote>,
    pub krop: Option<Value>,
}

/// Har Cloudflare meldt tom kvote i kroppen? Account-based: `extensions.code == "budget"`; ellers beskeden
/// "rate limiter budget depleted" (kan også komme i et 200-svar).
fn kvote_brugt(j: &Value) -> bool {
    j.get("errors").and_then(Value::as_array).into_iter().flatten().any(|e| {
        e.pointer("/extensions/code").and_then(Value::as_str) == Some("budget")
            || e.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.to_lowercase().contains("rate limiter budget depleted"))
    })
}

/// Raasvar → `data` eller en fejl. 429 og tom kvote er `Vent`; øvrige statusser som `status_fejl`. Ren funktion.
pub fn tolk_graphql(r: &Raasvar) -> Result<Value, Fejl> {
    if r.status == 429 || r.krop.as_ref().is_some_and(kvote_brugt) {
        return Err(Fejl::Vent);
    }
    if let Some(f) = status_fejl(r.status, r.krop.as_ref().and_then(foerste_fejl)) {
        return Err(f);
    }
    let j = r.krop.as_ref().ok_or(Fejl::Svar)?;
    if let Some(m) = foerste_fejl(j) {
        return Err(Fejl::GraphQl { dag: None, detalje: m });
    }
    match j.get("data") {
        Some(d) if !d.is_null() => Ok(d.clone()),
        _ => Err(Fejl::Svar),
    }
}

/// Et nyt tokens dom (V-KONTI.1, Q8). Svarer `user/tokens/verify` med en status, der ikke er `active`, er tokenet afvist som før. Bliver selve kaldet
/// afvist (`Afvist` eller `Api`: et Account API Token kan måske ikke verificeres dér; det er ikke målt), afgør rettighederne det: kun et svar, der bekræfter
/// begge (`Ok(true)`), godkender tokenet, og ellers gælder den oprindelige fejl. Uden for det er det som i 1.0.0: verificeret, og rettighederne afgør
/// en manglende rettighed. `accounts/{id}/tokens/verify` kaldes ikke (heller ikke målt). Ren funktion af de to svar.
pub async fn efter_verificering<F, Fut>(v: Result<(), Fejl>, rettigheder: F) -> Result<(), Fejl>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<bool, Fejl>>,
{
    match v {
        Ok(()) => rettigheder().await.map(|_| ()),
        Err(e @ (Fejl::Afvist(_) | Fejl::Api(_))) => match rettigheder().await {
            Ok(true) => Ok(()),
            _ => Err(e),
        },
        Err(e) => Err(e),
    }
}

/// Zonerne fra Cloudflare og kontoens navn til forbindelsen (V-KONTI.1, D5). `konto_navn` er den første konto efter navn (renset, `rens_kontonavn`),
/// `konti` antallet af forskellige konti i svaret. Kontoernes id'er bruges kun til at tælle dem og forlader aldrig `zoner`.
#[derive(Debug, Clone, PartialEq)]
pub struct Zoneliste {
    pub zoner: Vec<Zone>,
    pub konto_navn: Option<String>,
    pub konti: i64,
}

/// Højst så mange tegn af kontoens navn gemmes og vises.
pub const KONTONAVN_MAKS: usize = 100;

/// Kontoens navn, som det må gemmes: kontroltegn væk, mellemrum i enderne væk, højst `KONTONAVN_MAKS` tegn; bogstaver uden for ASCII (æøå) og en
/// e-mailadresse bliver stående (D5). Tomt bliver `None`. Ren funktion.
pub fn rens_kontonavn(s: &str) -> Option<String> {
    let s: String = s.chars().filter(|c| !c.is_control()).collect();
    let s: String = s.trim().chars().take(KONTONAVN_MAKS).collect();
    let s = s.trim_end().to_string();
    (!s.is_empty()).then_some(s)
}

/// Én side af /zones: zonerne (med id og navn) til `alle` og kontoerne (id → renset navn) til `konti`. Ren funktion.
pub fn zoneside(j: &Value, alle: &mut Vec<Zone>, konti: &mut BTreeMap<String, Option<String>>) {
    for z in j.get("result").and_then(Value::as_array).into_iter().flatten() {
        if let Some(k) = z.pointer("/account/id").and_then(Value::as_str).filter(|k| !k.is_empty()) {
            let navn = z.pointer("/account/name").and_then(Value::as_str).and_then(rens_kontonavn);
            let post = konti.entry(k.to_string()).or_insert(None);
            if post.is_none() {
                *post = navn;
            }
        }
        let (Some(id), Some(navn)) = (z.get("id").and_then(Value::as_str), z.get("name").and_then(Value::as_str)) else {
            continue;
        };
        alle.push(Zone {
            id: id.into(),
            navn: navn.into(),
        });
    }
}

/// Zonerne sorteret efter navn, den første konto efter navn og antallet af konti. Ren funktion.
pub fn zoneliste(mut alle: Vec<Zone>, konti: &BTreeMap<String, Option<String>>) -> Zoneliste {
    alle.sort_by(|a, b| a.navn.cmp(&b.navn));
    Zoneliste {
        zoner: alle,
        konto_navn: konti.values().flatten().min().cloned(),
        konti: konti.len() as i64,
    }
}

/// Den mindste forespørgsel, der kræver `Zone › Analytics › Read`.
const RET_QUERY: &str = "query Ret($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) { httpRequestsAdaptiveGroups(limit:1, filter:{datetime_geq:$since, datetime_lt:$until}) { count } } } }";

/// Svaret fra /zones: den første zone, `None`, hvis svaret ikke siger noget om rettigheden, eller `ManglerZone`. Tokenet er netop verificeret som aktivt, så en afvisning her handler om rettigheden.
/// Cloudflares dokumentation siger ikke, om svaret er 403 eller en tom liste, så begge tæller. Ren funktion.
pub fn mangler_ret_zoner(svar: Result<Option<String>, Fejl>) -> Result<Option<String>, Fejl> {
    match svar {
        Ok(Some(z)) => Ok(Some(z)),
        Ok(None) | Err(Fejl::Afvist(_)) => Err(Fejl::ManglerZone),
        Err(Fejl::Api(m)) if afvist_tekst(&m) => Err(Fejl::ManglerZone),
        Err(_) => Ok(None),
    }
}

/// Svaret fra `RET_QUERY`. `does not have access` er det, Cloudflare svarede ved et felt uden adgang (`adgang::felt_afvist`); ordlyden ved en manglende rettighed er ikke dokumenteret. Ren funktion.
pub fn mangler_ret_analytics(svar: &Result<Value, Fejl>) -> Result<(), Fejl> {
    match svar {
        Err(Fejl::Afvist(_)) => Err(Fejl::ManglerAnalytics),
        Err(Fejl::GraphQl { detalje, .. }) if afvist_tekst(detalje) => Err(Fejl::ManglerAnalytics),
        _ => Ok(()),
    }
}

fn afvist_tekst(m: &str) -> bool {
    let m = m.to_lowercase();
    ["does not have access", "not authorized", "unauthorized", "authz"]
        .iter()
        .any(|x| m.contains(x))
}

struct Takt {
    naeste_start: Instant,
    pause_til: Option<Instant>,
    /// Samme tidspunkt som `pause_til`, i ms (UTC), til "fortsætter kl. …" i UI'et.
    pause_til_ms: Option<i64>,
    langsom: bool,
    /// Antal 5xx i træk.
    nede: usize,
    /// Den seneste kvote, Cloudflare har meldt, og hvor længe tempoet er sat ned af den.
    kvote: Option<Kvote>,
    lav_til: Option<Instant>,
    /// Kald i runden, der er i gang, og i den sidste hele runde.
    brugt: u64,
    sidste_runde: Option<u64>,
}

/// Indstillinger → Data: "Cloudflare tillader N kald pr. M minutter; Huginn brugte K i sidste runde". Kun, når Cloudflare har meldt kvoten.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct KvoteUd {
    pub kald: i64,
    pub minutter: u64,
    pub brugt: u64,
}

/// Én fælles begrænser for alle GraphQL-kald: højst `MAKS_SAMTIDIGE` i luften, mindst `MELLEMRUM` mellem starterne, og
/// en fælles pause, når Cloudflare beder os vente (429/kvote) eller ikke svarer (5xx). Pausen gælder alle kaldere.
pub struct Begraenser {
    pladser: Semaphore,
    takt: Mutex<Takt>,
}

fn ms_nu() -> i64 {
    crate::vaern::ur_ms()
}

/// 0–30 s tilfældigt, så mange klienter ikke starter igen i samme sekund. Uret er nok til det; ingen ny afhængighed.
fn tilfaeldig_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::from(d.subsec_nanos()))
        % (KVOTEPAUSE_TILFAELDIG_MS + 1)
}

impl Default for Begraenser {
    fn default() -> Self {
        Begraenser {
            pladser: Semaphore::new(MAKS_SAMTIDIGE),
            takt: Mutex::new(Takt {
                naeste_start: Instant::now(),
                pause_til: None,
                pause_til_ms: None,
                langsom: false,
                nede: 0,
                kvote: None,
                lav_til: None,
                brugt: 0,
                sidste_runde: None,
            }),
        }
    }
}

impl Begraenser {
    fn takt(&self) -> std::sync::MutexGuard<'_, Takt> {
        self.takt.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Venter på en plads og på sin tur (mellemrum og eventuel pause). Pladsen holdes, til kaldet er færdigt.
    async fn tur(&self) -> SemaphorePermit<'_> {
        let plads = self.pladser.acquire().await.expect("begrænserens pladser lukkes aldrig");
        loop {
            let vent = {
                let mut t = self.takt();
                let nu = Instant::now();
                let tidligst = t.pause_til.map_or(t.naeste_start, |p| p.max(t.naeste_start));
                if tidligst <= nu {
                    let lav = t.lav_til.is_some_and(|x| x > nu);
                    t.naeste_start = nu + if t.langsom || lav { MELLEMRUM_LANGSOM } else { MELLEMRUM };
                    None
                } else {
                    Some(tidligst)
                }
            };
            match vent {
                None => return plads,
                Some(i) => tokio::time::sleep_until(i).await,
            }
        }
    }

    fn pause(t: &mut Takt, varighed: Duration) {
        let til = Instant::now() + varighed;
        if t.pause_til.is_none_or(|p| p < til) {
            t.pause_til = Some(til);
            t.pause_til_ms = Some(ms_nu() + varighed.as_millis() as i64);
        }
    }

    /// Husker Cloudflares kvote og sætter tempoet ned, til vinduet nulstilles, når der er under `LAV_KVOTE_PROCENT` tilbage. Uden tal er alt som før.
    fn noter_kvote(t: &mut Takt, k: Option<Kvote>) {
        let Some(k) = k else { return };
        t.kvote = Some(k);
        t.lav_til = (k.rest * 100 < k.kvote * LAV_KVOTE_PROCENT).then(|| Instant::now() + Duration::from_secs(k.nulstil_s.max(1)));
    }

    /// Justerer tempoet efter et svar: 429/kvote → pause (`Retry-After`, ellers `t` fra `Ratelimit`, ellers 300 s + 0–30 s) og langsomt tempo resten af
    /// kørslen; 5xx → 15/30/60/120 s; svar fra Cloudflare uden 5xx nulstiller 5xx-trinnet.
    fn efter(&self, resultat: &Result<Value, Fejl>, retry_after: Option<u64>, kvote: Option<Kvote>) {
        let mut t = self.takt();
        Self::noter_kvote(&mut t, kvote);
        match resultat {
            Err(Fejl::Vent) => {
                let sekunder = retry_after.or(kvote.map(|k| k.nulstil_s).filter(|s| *s > 0));
                let varighed = sekunder.map_or_else(|| KVOTEPAUSE + Duration::from_millis(tilfaeldig_ms()), Duration::from_secs);
                Self::pause(&mut t, varighed);
                t.langsom = true;
            }
            Err(Fejl::CfNede(_)) => {
                let s = NEDE_TRIN[t.nede.min(NEDE_TRIN.len() - 1)];
                t.nede += 1;
                Self::pause(&mut t, Duration::from_secs(s));
            }
            Err(Fejl::Forbindelse) => {}
            _ => t.nede = 0,
        }
    }

    /// Ét GraphQL-kald gennem begrænseren: vent på tur, send (`send`), tolk svaret og justér tempoet.
    pub async fn kald<F, Fut>(&self, send: F) -> Result<Value, Fejl>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Raasvar, Fejl>>,
    {
        let _plads = self.tur().await;
        self.talt();
        let (resultat, retry_after, kvote) = match send().await {
            Ok(r) => (tolk_graphql(&r), r.retry_after, r.kvote),
            Err(e) => (Err(e), None, None),
        };
        self.efter(&resultat, retry_after, kvote);
        resultat
    }

    /// Ét kald mere i runden.
    fn talt(&self) {
        self.takt().brugt += 1;
    }

    /// Et REST-svar: kvoten er fælles, så en 429 her sætter også GraphQL på pause. Radar går uden om, som altid.
    fn efter_rest(&self, status: u16, retry_after: Option<u64>, kvote: Option<Kvote>) {
        let resultat = if status == 429 { Err(Fejl::Vent) } else { Ok(Value::Null) };
        self.efter(&resultat, retry_after, kvote);
    }

    /// En ny runde begynder: den forrige tælles færdig.
    pub fn ny_runde(&self) {
        let mut t = self.takt();
        if t.brugt > 0 {
            t.sidste_runde = Some(t.brugt);
        }
        t.brugt = 0;
    }

    /// Kvoten til Indstillinger → Data.
    pub fn kvote_ud(&self) -> Option<KvoteUd> {
        let t = self.takt();
        t.kvote.map(|k| KvoteUd {
            kald: k.kvote,
            minutter: k.vindue_s.div_ceil(60),
            brugt: t.sidste_runde.unwrap_or(t.brugt),
        })
    }

    /// Hvornår pausen slutter (ms, UTC), hvis begrænseren holder pause lige nu.
    pub fn pause_til_ms(&self) -> Option<i64> {
        let t = self.takt();
        t.pause_til.filter(|p| *p > Instant::now()).and(t.pause_til_ms)
    }

    #[cfg(test)]
    fn langsom(&self) -> bool {
        self.takt().langsom
    }

    #[cfg(test)]
    fn lav(&self) -> bool {
        self.takt().lav_til.is_some_and(|x| x > Instant::now())
    }
}

// ── Klienten ───────────────────────────────────────────────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct Klient {
    http: reqwest::Client,
    begraenser: Arc<Begraenser>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Zone {
    pub id: String,
    pub navn: String,
}

/// Zonens tilstand fra GET /zones/{id} (Zone Read). `dev_sek`: sekunder tilbage af udviklertilstanden; 0 er slået fra, og et negativt tal er, hvornår den sidst sluttede.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ZoneStand {
    pub status: String,
    pub paused: bool,
    pub dev_sek: i64,
    pub phishing: bool,
}

/// `result` i svaret. Mangler et felt, er det det ufarlige svar: uden `status` kan intet siges. Ren funktion.
pub fn zone_stand(j: &Value) -> Result<ZoneStand, Fejl> {
    let r = j.get("result").ok_or(Fejl::Svar)?;
    Ok(ZoneStand {
        status: r.get("status").and_then(Value::as_str).ok_or(Fejl::Svar)?.to_string(),
        paused: r.get("paused").and_then(Value::as_bool).unwrap_or(false),
        dev_sek: r.get("development_mode").and_then(Value::as_i64).unwrap_or(0),
        phishing: r.pointer("/meta/phishing_detected").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// Selve POST-body'en til /graphql. Ren funktion (testes).
pub fn graphql_body(query: &str, variables: Value) -> Value {
    json!({ "query": query, "variables": variables })
}

fn url(sti: &str) -> String {
    debug_assert!(!sti.starts_with('/') && !sti.contains("://"), "kun stier under BASIS");
    format!("{BASIS}{sti}")
}

/// Alt, der går galt på vejen til og fra Cloudflare (DNS, forbindelse, TLS, timeout, afbrudt svar), er "ingen
/// forbindelse". reqwests egen tekst ("error sending request …") kasseres her.
fn netfejl(_: reqwest::Error) -> Fejl {
    Fejl::Forbindelse
}

/// Første fejlbesked i et Cloudflare-svar (`errors[0].message`), både REST og GraphQL.
fn foerste_fejl(j: &Value) -> Option<String> {
    j.get("errors")?.as_array()?.first().map(|e| {
        e.get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| e.to_string())
    })
}

impl Klient {
    pub fn ny() -> Result<Self, Fejl> {
        let http = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("huginn-muninn/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| Fejl::Intern)?;
        Ok(Klient {
            http,
            begraenser: Arc::default(),
        })
    }

    /// Hvornår begrænseren lader GraphQL-kald gå igen (ms, UTC), hvis den holder pause.
    pub fn pause_til_ms(&self) -> Option<i64> {
        self.begraenser.pause_til_ms()
    }

    async fn svar(r: reqwest::Response) -> Result<Value, Fejl> {
        let status = r.status().as_u16();
        let krop = r.bytes().await.map_err(netfejl)?; // afbrydes forbindelsen midt i svaret, er det også "ingen forbindelse"
        let j: Option<Value> = serde_json::from_slice(&krop).ok();
        if let Some(f) = status_fejl(status, j.as_ref().and_then(foerste_fejl)) {
            return Err(f);
        }
        j.ok_or(Fejl::Svar)
    }

    async fn get(&self, t: &Hemmelig, sti: &str) -> Result<Value, Fejl> {
        self.begraenser.talt();
        let r = self.http.get(url(sti)).bearer_auth(t.afsloer()).send().await.map_err(netfejl)?;
        self.begraenser
            .efter_rest(r.status().as_u16(), retry_after_af(r.headers()), kvote_af(r.headers()));
        let j = Self::svar(r).await?;
        if j.get("success").and_then(Value::as_bool) != Some(true) {
            return Err(Fejl::Api(foerste_fejl(&j).unwrap_or_else(|| "success er ikke true".into())));
        }
        Ok(j)
    }

    /// GET under radar/. Sin egen funktion ved siden af `get`: al fejl bliver `Fejl::Radar` (`tolk_radar`), og kaldet går uden om den
    /// fælles begrænser, som kun gælder GraphQL-kvoten.
    pub async fn radar(&self, t: &Hemmelig, sti: &str) -> Result<Value, Fejl> {
        debug_assert!(sti.starts_with("radar/"), "kun Radar");
        self.begraenser.talt();
        let r = self
            .http
            .get(url(sti))
            .bearer_auth(t.afsloer())
            .send()
            .await
            .map_err(|_| Fejl::Radar(None))?;
        let status = r.status().as_u16();
        let krop = r.bytes().await.map_err(|_| Fejl::Radar(None))?;
        tolk_radar(status, serde_json::from_slice(&krop).ok())
    }

    /// GET /user/tokens/verify — afviser tokens, Cloudflare ikke melder som aktive.
    pub async fn verificer(&self, t: &Hemmelig) -> Result<(), Fejl> {
        let j = self.get(t, "user/tokens/verify").await?;
        match j.pointer("/result/status").and_then(Value::as_str) {
            Some("active") | None => Ok(()),
            Some(s) => Err(Fejl::TokenInaktiv(s.into())),
        }
    }

    /// Har tokenet de to rettigheder? Ét kald til /zones og én lille GraphQL-forespørgsel. `Err` er en manglende rettighed; `Ok(true)`: Cloudflare bekræftede
    /// begge; `Ok(false)`: Cloudflare sagde noget andet (net, 5xx, kvote), og tokenet står ikke til doms her (køen melder selv fejlen). Se `efter_verificering`.
    async fn rettigheder(&self, t: &Hemmelig, nu: i64) -> Result<bool, Fejl> {
        let svar = self.get(t, "zones?per_page=1").await.map(|j| {
            j.get("result")
                .and_then(Value::as_array)
                .and_then(|z| z.first())
                .and_then(|z| z.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string)
        });
        let Some(zone) = mangler_ret_zoner(svar)? else {
            return Ok(false);
        };
        let svar = self
            .graphql(t, RET_QUERY, doegn_variabler(&zone, &dag_noegle(nu - DOEGN_MS))?)
            .await;
        mangler_ret_analytics(&svar)?;
        Ok(svar.is_ok())
    }

    /// Et nyt token, før det gemmes: verificeret og med de to rettigheder (`forbindelse_tilfoej`, `forbindelse_skift`). Se `efter_verificering`.
    pub async fn godkend(&self, t: &Hemmelig, nu: i64) -> Result<(), Fejl> {
        let v = self.verificer(t).await;
        efter_verificering(v, || self.rettigheder(t, nu)).await
    }

    /// GET /zones?per_page=50 (side 2+ hvis der er flere), sorteret efter navn, med kontoens navn (`Zoneliste`).
    pub async fn zoner(&self, t: &Hemmelig) -> Result<Zoneliste, Fejl> {
        let mut alle = Vec::new();
        let mut konti = BTreeMap::new();
        for side in 1..=MAKS_ZONESIDER {
            let j = self.get(t, &format!("zones?per_page=50&page={side}")).await?;
            zoneside(&j, &mut alle, &mut konti);
            let sider = j.pointer("/result_info/total_pages").and_then(Value::as_u64).unwrap_or(1);
            if u64::from(side) >= sider {
                break;
            }
        }
        Ok(zoneliste(alle, &konti))
    }

    /// Cloudflares egne sikkerhedsfund (`sikkerhed.rs`): kontoerne fra GET /zones, og for hver GET accounts/{id}/security-center/insights. Kontoens id bruges kun i
    /// adressen. Følger tempoet som `zone_stand`. 401/403 er `Fejl::Afvist`: den valgfri rettighed mangler.
    pub async fn sikkerhedsfund(&self, t: &Hemmelig) -> Result<Vec<crate::sikkerhed::Fund>, Fejl> {
        let mut konti = Vec::new();
        for side in 1..=MAKS_ZONESIDER {
            let _plads = self.begraenser.tur().await;
            let j = self.get(t, &format!("zones?per_page=50&page={side}")).await?;
            for k in crate::sikkerhed::konti(&j) {
                if !konti.contains(&k) {
                    konti.push(k);
                }
            }
            let sider = j.pointer("/result_info/total_pages").and_then(Value::as_u64).unwrap_or(1);
            if u64::from(side) >= sider {
                break;
            }
        }
        // En konto uden rettigheden springes over, så de andre stadig vises. Mangler den på alle, er det `Afvist`.
        let (mut alle, mut svarede, mut afvist) = (Vec::new(), 0, None);
        'konti: for k in &konti {
            let mut set = 0;
            for side in 1..=crate::sikkerhed::MAKS_SIDER {
                let _plads = self.begraenser.tur().await;
                let j = match self
                    .get(
                        t,
                        &format!(
                            "accounts/{k}/security-center/insights?per_page={}&page={side}",
                            crate::sikkerhed::PR_SIDE
                        ),
                    )
                    .await
                {
                    Err(e @ Fejl::Afvist(_)) => {
                        afvist = Some(e);
                        continue 'konti;
                    }
                    svar => svar?,
                };
                let n = j.pointer("/result/issues").and_then(Value::as_array).map_or(0, Vec::len);
                let (fund, count) = crate::sikkerhed::fra_svar(&j)?;
                alle.extend(fund);
                set += n;
                if n == 0 || set >= count {
                    break;
                }
            }
            svarede += 1;
        }
        match afvist {
            Some(e) if svarede == 0 => Err(e),
            _ => Ok(alle),
        }
    }

    /// GET /zones/{id}: zonens tilstand. Køens eneste REST-kald i runden, så det følger tempoet og venter en pause ud.
    pub async fn zone_stand(&self, t: &Hemmelig, zone: &str) -> Result<ZoneStand, Fejl> {
        let _plads = self.begraenser.tur().await;
        zone_stand(&self.get(t, &format!("zones/{zone}")).await?)
    }

    /// Se `Begraenser::ny_runde`.
    pub fn ny_runde(&self) {
        self.begraenser.ny_runde();
    }

    /// Se `Begraenser::kvote_ud`.
    pub fn kvote_ud(&self) -> Option<KvoteUd> {
        self.begraenser.kvote_ud()
    }

    /// POST /graphql med variabler, gennem den fælles begrænser. Returnerer `data`.
    pub async fn graphql(&self, t: &Hemmelig, query: &str, variables: Value) -> Result<Value, Fejl> {
        let body = graphql_body(query, variables);
        self.begraenser
            .kald(|| async {
                let r = self
                    .http
                    .post(url("graphql"))
                    .bearer_auth(t.afsloer())
                    .json(&body)
                    .send()
                    .await
                    .map_err(netfejl)?;
                let status = r.status().as_u16();
                let (retry_after, kvote) = (retry_after_af(r.headers()), kvote_af(r.headers()));
                let krop = r.bytes().await.map_err(netfejl)?;
                Ok(Raasvar {
                    status,
                    retry_after,
                    kvote,
                    krop: serde_json::from_slice(&krop).ok(),
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAA: &str = "AbCdEfGhIjKlMnOpQrStUvWxYz0123456789_-xy";

    fn som_json(f: &Fejl) -> Value {
        serde_json::to_value(f).unwrap()
    }

    #[test]
    fn manglende_rettigheder_siger_hvilken() {
        let zoner = |s| mangler_ret_zoner(s);
        assert_eq!(zoner(Ok(Some("z".into()))), Ok(Some("z".into())));
        assert_eq!(zoner(Ok(None)), Err(Fejl::ManglerZone), "tom liste");
        assert_eq!(zoner(Err(Fejl::Afvist(403))), Err(Fejl::ManglerZone));
        assert_eq!(
            zoner(Err(Fejl::Api("Unauthorized to access requested resource".into()))),
            Err(Fejl::ManglerZone)
        );
        for andet in [Fejl::Forbindelse, Fejl::CfNede(500), Fejl::Vent, Fejl::Api("Noget andet".into())] {
            assert_eq!(zoner(Err(andet)), Ok(None), "siger intet om rettigheden");
        }
        let gql = |d: &str| {
            Err(Fejl::GraphQl {
                dag: None,
                detalje: d.into(),
            })
        };
        assert_eq!(mangler_ret_analytics(&Ok(json!({}))), Ok(()));
        assert_eq!(mangler_ret_analytics(&Err(Fejl::Afvist(403))), Err(Fejl::ManglerAnalytics));
        assert_eq!(
            mangler_ret_analytics(&gql("zone does not have access to the path")),
            Err(Fejl::ManglerAnalytics)
        );
        assert_eq!(
            mangler_ret_analytics(&gql("not authorized for that account")),
            Err(Fejl::ManglerAnalytics)
        );
        assert_eq!(mangler_ret_analytics(&gql("unknown field")), Ok(()));
        assert_eq!(mangler_ret_analytics(&Err(Fejl::Vent)), Ok(()));
        assert_eq!(som_json(&Fejl::ManglerZone), json!({ "kode": "mangler_zone" }));
        assert_eq!(som_json(&Fejl::ManglerAnalytics), json!({ "kode": "mangler_analytics" }));
    }

    #[test]
    fn fejltekster_renses_for_tokens() {
        assert_eq!(
            rens_fejl(&format!("Authorization: Bearer {RAA} afvist")),
            "Authorization: Bearer <TOKEN> afvist"
        );
        assert_eq!(rens_fejl(&format!("zone {RAA}")), "zone <TOKEN>");
        assert_eq!(
            rens_fejl("httpRequestsAdaptiveGroups: ukendt felt"),
            "httpRequestsAdaptiveGroups: ukendt felt",
            "feltnavne bevares"
        );
        assert_eq!(rens_fejl("a\u{0}b\nc"), "abc");
        assert_eq!(rens_fejl(&"x ".repeat(400)).chars().count(), 300);
        for f in [
            Fejl::Api(format!("bad token {RAA}")),
            Fejl::GraphQl {
                dag: Some("2026-10-11".into()),
                detalje: format!("Bearer {RAA}"),
            },
            Fejl::TokenInaktiv(RAA.into()),
        ] {
            assert!(!f.to_string().contains(RAA));
            assert!(!format!("{f:?}").contains(RAA));
            assert!(!som_json(&f).to_string().contains(RAA), "heller ikke i det, webvisningen får");
        }
    }

    /// Radar: ingen status bliver `Afvist`, `Vent` eller `CfNede`, så hverken køen eller begrænseren mærker det.
    #[test]
    fn radar_fejl_er_altid_radar() {
        for s in [400, 401, 403, 404, 422, 429, 500, 503] {
            assert_eq!(
                tolk_radar(s, Some(json!({ "errors": [{ "message": "nej" }] }))),
                Err(Fejl::Radar(Some(s))),
                "{s}"
            );
        }
        assert_eq!(tolk_radar(200, None), Err(Fejl::Radar(None)), "ikke JSON");
        assert_eq!(tolk_radar(200, Some(json!({ "success": false }))), Err(Fejl::Radar(None)));
        assert_eq!(tolk_radar(200, Some(json!({ "success": true, "result": 1 }))).unwrap()["result"], 1);
        assert_eq!(som_json(&Fejl::Radar(Some(403))), json!({ "kode": "radar", "status": 403 }));
    }

    #[test]
    fn http_status_kortlaegges_til_en_lukket_liste() {
        assert_eq!(status_fejl(200, None), None);
        assert_eq!(status_fejl(204, Some("x".into())), None);
        assert_eq!(status_fejl(401, None), Some(Fejl::Afvist(401)));
        assert_eq!(status_fejl(403, Some("Authentication error".into())), Some(Fejl::Afvist(403)));
        assert_eq!(status_fejl(429, None), Some(Fejl::Vent));
        assert_eq!(status_fejl(500, None), Some(Fejl::CfNede(500)));
        assert_eq!(status_fejl(503, Some("x".into())), Some(Fejl::CfNede(503)));
        assert_eq!(status_fejl(404, None), Some(Fejl::Http(404)));
        assert_eq!(status_fejl(400, Some("nej".into())), Some(Fejl::Api("HTTP 400: nej".into())));
    }

    #[test]
    fn webvisningen_faar_kode_og_kun_cloudflares_rensede_detalje() {
        assert_eq!(som_json(&Fejl::Forbindelse), json!({ "kode": "forbindelse" }));
        assert_eq!(som_json(&Fejl::Afvist(403)), json!({ "kode": "afvist", "status": 403 }));
        assert_eq!(som_json(&Fejl::Vent), json!({ "kode": "vent" }));
        assert_eq!(som_json(&Fejl::CfNede(502)), json!({ "kode": "cf_nede", "status": 502 }));
        assert_eq!(som_json(&Fejl::Svar), json!({ "kode": "svar" }));
        assert_eq!(som_json(&Fejl::Lager), json!({ "kode": "lager" }));
        let g = Fejl::GraphQl {
            dag: None,
            detalje: "cannot request data older than 30d".into(),
        }
        .for_dag("2026-10-11");
        assert_eq!(
            som_json(&g),
            json!({ "kode": "graphql", "dag": "2026-10-11", "detalje": "cannot request data older than 30d" })
        );
        assert_eq!(Fejl::Vent.for_dag("2026-10-11"), Fejl::Vent, "kun GraphQL-fejl får et døgn");
        assert_eq!(g.to_string(), "graphql 2026-10-11: cannot request data older than 30d");
    }

    /// En rigtig reqwest-fejl (forbindelse nægtet på den lokale maskine; intet kald til Cloudflare): kun koden slipper ud.
    #[tokio::test]
    async fn bibliotekets_raa_tekst_slipper_aldrig_igennem() {
        let e = reqwest::Client::new().get("http://127.0.0.1:1/").send().await.unwrap_err();
        let raa = e.to_string();
        assert!(
            raa.contains("error sending request"),
            "forudsætning: reqwests egen tekst er {raa:?}"
        );
        let f = netfejl(e);
        assert_eq!(f, Fejl::Forbindelse);
        for ud in [som_json(&f).to_string(), f.to_string(), format!("{f:?}")] {
            for forbudt in ["error sending request", "127.0.0.1", "refused", "tcp", "client error"] {
                assert!(!ud.to_lowercase().contains(forbudt), "{forbudt:?} i {ud:?}");
            }
        }
    }

    /// Felterne, Cloudflare svarer med. Uden `status` kan intet siges, og det er en fejl, ikke et grønt svar.
    #[test]
    fn zone_stand_laeser_felterne() {
        let j = json!({"result": {"status": "active", "paused": true, "development_mode": 7200, "meta": {"phishing_detected": true}}});
        assert_eq!(
            zone_stand(&j).unwrap(),
            ZoneStand {
                status: "active".into(),
                paused: true,
                dev_sek: 7200,
                phishing: true
            }
        );
        let kort = zone_stand(&json!({"result": {"status": "pending"}})).unwrap();
        assert_eq!((kort.paused, kort.dev_sek, kort.phishing), (false, 0, false));
        assert_eq!(zone_stand(&json!({"result": {}})), Err(Fejl::Svar));
    }

    #[test]
    fn graphql_body_har_query_og_variabler() {
        let b = graphql_body("query Q($zoneTag: string!) { x }", json!({ "zoneTag": "abc" }));
        assert_eq!(b["variables"]["zoneTag"], "abc");
        assert_eq!(b["query"], "query Q($zoneTag: string!) { x }");
    }

    #[test]
    fn kun_cloudflares_api_vaert() {
        assert_eq!(url("graphql"), "https://api.cloudflare.com/client/v4/graphql");
        assert_eq!(
            url("zones?per_page=50&page=2"),
            "https://api.cloudflare.com/client/v4/zones?per_page=50&page=2"
        );
    }

    #[test]
    fn foerste_fejl_fra_svar() {
        assert_eq!(foerste_fejl(&json!({"errors": [{"message": "nej"}]})).as_deref(), Some("nej"));
        assert_eq!(foerste_fejl(&json!({"errors": []})), None);
        assert_eq!(foerste_fejl(&json!({"data": {}})), None);
    }

    // ── Begrænseren: falsk klient (Raasvar uden net) og falsk ur (tokios pausede tid, der springer frem ved sleep) ──
    fn ok() -> Raasvar {
        Raasvar {
            status: 200,
            retry_after: None,
            kvote: None,
            krop: Some(json!({ "data": { "viewer": {} } })),
        }
    }
    fn svar(status: u16, retry_after: Option<u64>, krop: Value) -> Raasvar {
        Raasvar {
            status,
            retry_after,
            kvote: None,
            krop: Some(krop),
        }
    }
    fn kvote(rest: i64, nulstil_s: u64) -> Option<Kvote> {
        Some(Kvote {
            rest,
            nulstil_s,
            kvote: 1200,
            vindue_s: 300,
        })
    }

    #[test]
    fn kvoten_laeses_af_cloudflares_headere() {
        assert_eq!(
            laes_kvote(Some(r#""list_zones";r=1200;t=1"#), Some(r#""list_zones";q=1201;w=300"#)),
            Some(Kvote {
                rest: 1200,
                nulstil_s: 1,
                kvote: 1201,
                vindue_s: 300
            })
        );
        let k = laes_kvote(
            Some(r#""default";r=900;t=30, "burst";r=5;t=9"#),
            Some(r#""default";q=1000;w=300, "burst";q=100;w=60"#),
        );
        assert_eq!(k.map(|k| (k.rest, k.vindue_s)), Some((5, 60)), "den strammeste");
        for (a, b) in [
            (None, Some(r#""x";q=1;w=1"#)),
            (Some(r#""x";r=1;t=1"#), None),
            (Some("vrøvl"), Some("vrøvl")),
            (Some(r#""x";r=1;t=1"#), Some(r#""y";q=1;w=1"#)),
            (Some(r#""x";r=1;t=1"#), Some(r#""x";q=0;w=300"#)),
        ] {
            assert_eq!(laes_kvote(a, b), None, "{a:?} {b:?}");
        }
    }

    /// Lav kvote: det langsomme tempo, til vinduet nulstilles, og så tilbage. Ingen fast antagelse om kvotens størrelse.
    #[tokio::test(start_paused = true)]
    async fn lav_kvote_saenker_tempoet_til_vinduet_er_nulstillet() {
        let b = Begraenser::default();
        let t0 = Instant::now();
        let r = |k| Raasvar { kvote: k, ..ok() };
        let _ = send(&b, t0, r(kvote(600, 1))).await;
        assert!(!b.lav(), "halvdelen er ikke lavt");
        let _ = send(&b, t0, r(kvote(120, 1))).await;
        assert!(!b.lav(), "præcis 10 % er ikke under");
        let _ = send(&b, t0, r(kvote(119, 100))).await;
        assert!(b.lav());
        let (foer, _) = send(&b, t0, ok()).await;
        let (efter, _) = send(&b, t0, ok()).await;
        assert_eq!(efter - foer, MELLEMRUM_LANGSOM.as_millis(), "langsomt");
        tokio::time::sleep(Duration::from_secs(100)).await;
        assert!(!b.lav() && !b.langsom(), "vinduet er nulstillet, og lav kvote er ikke en 429");
        let (a, _) = send(&b, t0, ok()).await;
        let (c, _) = send(&b, t0, ok()).await;
        assert_eq!(c - a, MELLEMRUM.as_millis(), "normalt tempo igen");
    }

    /// 429: `Retry-After` først, ellers `t`, ellers den faste pause.
    #[tokio::test(start_paused = true)]
    async fn kvotefejl_venter_paa_retry_after_eller_t() {
        let n429 = |ra, k| Raasvar {
            kvote: k,
            ..svar(429, ra, json!({}))
        };
        for (ra, k, ventet) in [(Some(40), kvote(0, 20), 40_000), (None, kvote(0, 20), 20_000)] {
            let (b, t0) = (Begraenser::default(), Instant::now());
            let _ = send(&b, t0, n429(ra, k)).await;
            assert_eq!(send(&b, t0, ok()).await.0, ventet, "{ra:?}");
        }
        let (c, t1) = (Begraenser::default(), Instant::now());
        let _ = send(&c, t1, n429(None, None)).await;
        assert!(send(&c, t1, ok()).await.0 >= KVOTEPAUSE.as_millis(), "uden headere som i dag");
    }

    /// Uden headere er tempoet det samme som før, og Indstillinger siger intet.
    #[tokio::test(start_paused = true)]
    async fn manglende_headere_aendrer_intet() {
        let b = Begraenser::default();
        let t0 = Instant::now();
        let (a, _) = send(&b, t0, ok()).await;
        let (c, _) = send(&b, t0, ok()).await;
        assert_eq!(c - a, MELLEMRUM.as_millis());
        assert_eq!(b.kvote_ud(), None);
    }

    #[test]
    fn rest_429_saetter_alt_paa_pause() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
        rt.block_on(async {
            let b = Begraenser::default();
            b.efter_rest(200, None, kvote(1000, 5));
            assert_eq!(b.pause_til_ms(), None);
            b.efter_rest(429, Some(30), None);
            assert!(b.pause_til_ms().is_some());
            assert!(b.langsom());
        });
    }

    #[tokio::test(start_paused = true)]
    async fn kaldene_taelles_pr_runde() {
        let b = Begraenser::default();
        let t0 = Instant::now();
        for _ in 0..3 {
            let _ = send(
                &b,
                t0,
                Raasvar {
                    kvote: kvote(1000, 5),
                    ..ok()
                },
            )
            .await;
        }
        let ud = b.kvote_ud().unwrap();
        assert_eq!((ud.kald, ud.minutter, ud.brugt), (1200, 5, 3), "første runde er i gang");
        b.ny_runde();
        let _ = send(&b, t0, ok()).await;
        assert_eq!(b.kvote_ud().unwrap().brugt, 3, "den sidste hele runde");
    }

    /// Sender ét kald med et fast svar og returnerer (start i ms efter `t0`, resultat).
    async fn send(b: &Begraenser, t0: Instant, r: Raasvar) -> (u128, Result<Value, Fejl>) {
        let mut start = 0;
        let res = b
            .kald(|| {
                start = (Instant::now() - t0).as_millis();
                async move { Ok(r) }
            })
            .await;
        (start, res)
    }

    #[test]
    fn graphql_svar_tolkes() {
        assert!(tolk_graphql(&ok()).is_ok());
        assert_eq!(tolk_graphql(&svar(429, Some(5), json!({}))), Err(Fejl::Vent));
        let budget = json!({ "data": null, "errors": [{ "message": "Zone <ZONE_TAG> has exceeded its rate limit.", "extensions": { "code": "budget" } }] });
        assert_eq!(tolk_graphql(&svar(200, None, budget)), Err(Fejl::Vent), "budget i et 200-svar");
        let udtoemt = json!({ "errors": [{ "message": "rate limiter budget depleted, try again after 5 minutes" }] });
        assert_eq!(tolk_graphql(&svar(200, None, udtoemt)), Err(Fejl::Vent));
        assert_eq!(
            tolk_graphql(&svar(
                503,
                None,
                json!({ "errors": [{ "message": "too many queries in progress, please try again later" }] })
            )),
            Err(Fejl::CfNede(503))
        );
        assert_eq!(tolk_graphql(&svar(401, None, json!({}))), Err(Fejl::Afvist(401)));
        let gammel = tolk_graphql(&svar(
            200,
            None,
            json!({ "errors": [{ "message": "cannot request data older than 2678400s" }] }),
        ));
        assert!(gammel.as_ref().unwrap_err().uden_for_opbevaring());
        let gammel400 = tolk_graphql(&svar(
            400,
            None,
            json!({ "errors": [{ "message": "query time range is older than retention" }] }),
        ));
        assert!(gammel400.as_ref().unwrap_err().uden_for_opbevaring(), "{gammel400:?}");
        assert!(!Fejl::GraphQl {
            dag: None,
            detalje: "unknown field".into()
        }
        .uden_for_opbevaring());
        assert!(!Fejl::Vent.uden_for_opbevaring());
    }

    #[tokio::test(start_paused = true)]
    async fn begraenser_hoejst_to_samtidige_og_mindst_1_1_s_mellem_starter() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let b = Begraenser::default();
        let t0 = Instant::now();
        let (i_luften, flest) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let starter = Mutex::new(Vec::new());
        let et_kald = || async {
            b.kald(|| async {
                starter.lock().unwrap().push((Instant::now() - t0).as_millis());
                let n = i_luften.fetch_add(1, Ordering::SeqCst) + 1;
                flest.fetch_max(n, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_secs(5)).await; // et langsomt svar
                i_luften.fetch_sub(1, Ordering::SeqCst);
                Ok(ok())
            })
            .await
        };
        let svar = futures_util::future::join_all((0..5).map(|_| et_kald())).await;
        assert!(svar.iter().all(Result::is_ok));
        assert_eq!(flest.load(Ordering::SeqCst), MAKS_SAMTIDIGE, "højst to i luften");
        let mut s = starter.lock().unwrap().clone();
        s.sort();
        assert_eq!(
            s,
            [0, 1100, 5000, 6100, 10_000],
            "1,1 s mellem starter, og den tredje venter på en plads"
        );
        for w in s.windows(2) {
            assert!(w[1] - w[0] >= 1100, "{s:?}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn begraenser_429_med_retry_after_og_derefter_langsomt_tempo() {
        let b = Begraenser::default();
        let t0 = Instant::now();
        assert_eq!(send(&b, t0, ok()).await.0, 0);
        let (s, r) = send(&b, t0, svar(429, Some(42), json!({}))).await;
        assert_eq!((s, r), (1100, Err(Fejl::Vent)));
        assert!(b.langsom());
        let pause = b.pause_til_ms().expect("pause meldes til UI'et") - ms_nu();
        assert!((41_000..=42_000).contains(&pause), "{pause} ms");
        let (s, r) = send(&b, t0, ok()).await;
        assert!(r.is_ok());
        assert_eq!(s, 1100 + 42_000, "næste kald venter Retry-After");
        assert_eq!(send(&b, t0, ok()).await.0, 43_100 + 7500, "derefter 1 kald pr. 7,5 s");
        assert_eq!(send(&b, t0, ok()).await.0, 50_600 + 7500, "også efter et vellykket kald");
        assert_eq!(b.pause_til_ms(), None);
    }

    #[tokio::test(start_paused = true)]
    async fn begraenser_tom_kvote_uden_retry_after_pauser_300_til_330_s() {
        let b = Begraenser::default();
        let t0 = Instant::now();
        let budget = json!({ "errors": [{ "message": "x", "extensions": { "code": "budget" } }] });
        assert_eq!(send(&b, t0, svar(200, None, budget)).await.1, Err(Fejl::Vent));
        let (s, _) = send(&b, t0, ok()).await;
        assert!((300_000..=330_000).contains(&s), "pause {s} ms");
        let (s2, _) = send(&b, t0, ok()).await;
        assert_eq!(s2 - s, 7500, "langsomt tempo");
        // 429 uden header: samme pause.
        let b = Begraenser::default();
        let t0 = Instant::now();
        assert_eq!(send(&b, t0, svar(429, None, json!({}))).await.1, Err(Fejl::Vent));
        let (s, _) = send(&b, t0, ok()).await;
        assert!((300_000..=330_000).contains(&s), "pause {s} ms");
    }

    #[tokio::test(start_paused = true)]
    async fn begraenser_5xx_venter_15_30_60_120_og_nulstilles_ved_svar() {
        let b = Begraenser::default();
        let t0 = Instant::now();
        let nede = || {
            svar(
                503,
                None,
                json!({ "errors": [{ "message": "too many queries in progress, please try again later" }] }),
            )
        };
        let mut starter = vec![send(&b, t0, nede()).await.0];
        for _ in 0..4 {
            let (s, r) = send(&b, t0, nede()).await;
            assert_eq!(r, Err(Fejl::CfNede(503)));
            starter.push(s);
        }
        let mellem: Vec<u128> = starter.windows(2).map(|w| w[1] - w[0]).collect();
        assert_eq!(mellem, [15_000, 30_000, 60_000, 120_000]);
        let (s, r) = send(&b, t0, svar(502, None, json!({}))).await;
        assert_eq!((s - starter[4], r), (120_000, Err(Fejl::CfNede(502))), "derefter 120 s igen");
        let (s1, r) = send(&b, t0, ok()).await;
        assert!(r.is_ok());
        assert_eq!(s1 - s, 120_000);
        let (s2, _) = send(&b, t0, nede()).await;
        let (s3, _) = send(&b, t0, ok()).await;
        assert_eq!((s2 - s1, s3 - s2), (1100, 15_000), "et svar nulstiller trinnet");
        assert!(!b.langsom(), "5xx giver ikke langsomt tempo");
    }

    #[tokio::test(start_paused = true)]
    async fn begraenser_401_giver_afvist_uden_pause() {
        let b = Begraenser::default();
        let t0 = Instant::now();
        assert_eq!(send(&b, t0, svar(401, None, json!({}))).await.1, Err(Fejl::Afvist(401)));
        assert_eq!(send(&b, t0, svar(403, None, json!({}))).await, (1100, Err(Fejl::Afvist(403))));
        assert_eq!(b.pause_til_ms(), None);
        let r = b.kald(|| async { Err::<Raasvar, _>(Fejl::Forbindelse) }).await;
        assert_eq!(r, Err(Fejl::Forbindelse));
        assert_eq!(
            send(&b, t0, ok()).await.0,
            3300,
            "ingen forbindelse: ingen pause i begrænseren (køen venter selv)"
        );
    }

    /// V-KONTI.1 (Q8): en status, der ikke er `active`, afviser som før; et afvist kald til `user/tokens/verify` afgøres af rettighederne, og kun et
    /// bekræftende svar godkender. Ellers gælder den oprindelige fejl. Med en verificering, der lykkes, er det som i 1.0.0.
    #[tokio::test]
    async fn verificer_afvist_afgoeres_af_rettighederne() {
        let ret = |r: Result<bool, Fejl>| move || async move { r };
        let inaktiv = Fejl::TokenInaktiv("disabled".into());
        assert_eq!(
            efter_verificering(Err(inaktiv.clone()), ret(Ok(true))).await,
            Err(inaktiv),
            "inaktiv afvises uden at spørge"
        );
        assert_eq!(
            efter_verificering(Err(Fejl::Afvist(401)), ret(Ok(true))).await,
            Ok(()),
            "kontotoken: rettighederne bekræfter"
        );
        assert_eq!(efter_verificering(Err(Fejl::Api("x".into())), ret(Ok(true))).await, Ok(()));
        assert_eq!(
            efter_verificering(Err(Fejl::Afvist(401)), ret(Err(Fejl::ManglerZone))).await,
            Err(Fejl::Afvist(401)),
            "afviser rettighederne også, gælder den oprindelige fejl"
        );
        assert_eq!(
            efter_verificering(Err(Fejl::Afvist(403)), ret(Ok(false))).await,
            Err(Fejl::Afvist(403)),
            "intet svar om rettighederne er ingen godkendelse"
        );
        assert_eq!(
            efter_verificering(Err(Fejl::Forbindelse), ret(Ok(true))).await,
            Err(Fejl::Forbindelse),
            "net: ingen dom"
        );
        assert_eq!(
            efter_verificering(Ok(()), ret(Ok(false))).await,
            Ok(()),
            "som i 1.0.0: intet svar står ikke til doms"
        );
        assert_eq!(
            efter_verificering(Ok(()), ret(Err(Fejl::ManglerAnalytics))).await,
            Err(Fejl::ManglerAnalytics)
        );
    }

    /// D5: kontoens navn renses (kontroltegn, længde) og bruges ellers uændret, også med æøå. Det står kun i `Zoneliste`, aldrig i en fejl.
    #[test]
    fn kontonavn_renses_og_ender_aldrig_i_fejl() {
        assert_eq!(rens_kontonavn("  Ærø\u{0007} Rådgivning\n "), Some("Ærø Rådgivning".into()));
        assert_eq!(rens_kontonavn("\u{0000}\t "), None);
        let langt = "å".repeat(150);
        assert_eq!(rens_kontonavn(&langt).unwrap().chars().count(), KONTONAVN_MAKS);
        let navn = "Hemmeligt Kontonavn ApS";
        let side = json!({ "result": [
            { "id": "z2", "name": "b.wi24rd.com", "account": { "id": "k2", "name": "Øvrig Konto" } },
            { "id": "z1", "name": "a.wi24rd.com", "account": { "id": "k1", "name": format!("{navn}\u{0001}") } },
            { "id": "z3", "name": "c.wi24rd.com", "account": { "id": "k1", "name": navn } },
            { "name": "uden-id.wi24rd.com", "account": { "id": "k3" } }
        ] });
        let (mut alle, mut konti) = (Vec::new(), BTreeMap::new());
        zoneside(&side, &mut alle, &mut konti);
        let l = zoneliste(alle, &konti);
        assert_eq!(
            l.zoner.iter().map(|z| z.id.as_str()).collect::<Vec<_>>(),
            ["z1", "z2", "z3"],
            "efter navn; uden id springes over"
        );
        assert_eq!(l.konto_navn.as_deref(), Some(navn), "den første konto efter navn");
        assert_eq!(l.konti, 3, "tre forskellige konti (id'erne tælles, gemmes ikke)");
        let tom = zoneliste(Vec::new(), &BTreeMap::new());
        assert_eq!((tom.konto_navn, tom.konti), (None, 0));
        for f in [Fejl::Svar, Fejl::Api(rens_fejl("bad")), Fejl::Afvist(403), Fejl::ManglerZone] {
            let ud = format!("{f} {f:?} {}", som_json(&f));
            assert!(!ud.contains(navn) && !ud.contains("k1"), "{ud}");
        }
    }

    /// D5: et navn med en e-mailadresse (Cloudflares standard) bruges uændret; der foreslås intet andet.
    #[test]
    fn kontonavn_med_e_mail_bruges_uaendret() {
        let navn = "test@wi24rd.com's Account";
        assert_eq!(rens_kontonavn(navn).as_deref(), Some(navn));
        let side = json!({ "result": [{ "id": "z1", "name": "a.wi24rd.com", "account": { "id": "k1", "name": navn } }] });
        let (mut alle, mut konti) = (Vec::new(), BTreeMap::new());
        zoneside(&side, &mut alle, &mut konti);
        assert_eq!(zoneliste(alle, &konti).konto_navn.as_deref(), Some(navn));
    }
}
