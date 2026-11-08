//! Værnet: hvad Cloudflares edge stoppede på brugerens zone de seneste 30 døgn.
//! Port af W `worker/src/features/vagt.js` (MIT, samme ophavsmand): rensning, klassifikation, døgnrækker, byggVaern og
//! offentligVaern. Hvert UTC-døgn hentes for sig (korte perioder samples ikke), i dag og i går hver gang, ældre døgn kun
//! når de mangler i den lokale cache. Ingen IP-adresser hentes eller gemmes; stier renses, før de gemmes.
//!
//! Regulære udtryk bygges med `unicode(false)`, så `\w`, `\d`, `\s` og `\b` er ASCII som i JavaScript uden u-flaget.
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::sync::{LazyLock, Mutex};

use futures_util::stream::{self, StreamExt};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};

use crate::cf::Fejl;
use crate::overblik::{angreb, er_afvist, gruppe, hele_doegn, regel_art, trafik_spring, Hele, TrafikDag, FAELDER, RUTINE};
use crate::store::{Raekke, Store};

pub const DAGE: i64 = 30;
/// Hentevinduet: i dag + de sidste 30 hele UTC-døgn.
pub const HENT_DAGE: i64 = 30;
pub const SAMTIDIGE: usize = 4;
pub const DOEGN_MS: i64 = 86_400_000;
/// Et ældre døgn regnes for færdighentet, når det er hentet mindst så længe efter døgnets slutning.
const FAERDIG_MARGIN_MS: i64 = 3_600_000;

fn re(p: &str) -> Regex {
    RegexBuilder::new(p).unicode(false).build().expect(p)
}

// ── Tid (UTC) ──────────────────────────────────────────────────────────────────────────────────────────────────────
fn dage_fra_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_fra_dage(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

/// `YYYY-MM-DD` (UTC) for et tidspunkt i ms — W `dagNoegle`.
pub fn dag_noegle(ms: i64) -> String {
    let (y, m, d) = civil_fra_dage(ms.div_euclid(DOEGN_MS));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Testenes faste ur, når `HM_NU` ikke er sat. Samme tidspunkt som UI-testene (`tests/ui/ui-roeg.mjs`).
#[cfg(test)]
pub const TEST_NU: &str = "2026-11-06T12:00:00Z";

/// Uret i ms (UTC). I test er det `HM_NU` eller `TEST_NU`, så ingen test afhænger af dagens dato. Programmet læser aldrig `HM_NU`: linjerne findes kun i testbygget.
pub fn ur_ms() -> i64 {
    #[cfg(test)]
    {
        let v = std::env::var("HM_NU").unwrap_or_else(|_| TEST_NU.to_string());
        parse_iso(&v).expect("HM_NU skal have formen 2026-11-06T12:00:00Z")
    }
    #[cfg(not(test))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64)
    }
}

/// ISO 8601 med millisekunder, som JavaScripts `toISOString()`.
pub fn iso(ms: i64) -> String {
    let (y, m, d) = civil_fra_dage(ms.div_euclid(DOEGN_MS));
    let r = ms.rem_euclid(DOEGN_MS);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        r / 3_600_000,
        r / 60_000 % 60,
        r / 1000 % 60,
        r % 1000
    )
}

fn tal(s: &str) -> Option<i64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Midnat (UTC) for `YYYY-MM-DD` i ms; `None` hvis datoen ikke findes.
pub fn dag_ms(dag: &str) -> Option<i64> {
    let b = dag.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (tal(&dag[0..4])?, tal(&dag[5..7])?, tal(&dag[8..10])?);
    let z = dage_fra_civil(y, m, d);
    (civil_fra_dage(z) == (y, m, d)).then_some(z * DOEGN_MS)
}

/// `YYYY-MM-DDTHH:MM:SS[.sss]Z` → ms. Smallere end JavaScripts `Date.parse`, men det er formatet, Cloudflare svarer med.
pub fn parse_iso(t: &str) -> Option<i64> {
    let t = t.strip_suffix('Z')?;
    let (dato, kl) = t.split_once('T')?;
    let (kl, brok) = kl.split_once('.').unwrap_or((kl, ""));
    let mut dele = kl.split(':');
    let (h, mi, s) = (tal(dele.next()?)?, tal(dele.next()?)?, tal(dele.next()?)?);
    if dele.next().is_some() || kl.len() != 8 || h > 23 || mi > 59 || s > 59 {
        return None;
    }
    let ms = if brok.is_empty() {
        0
    } else {
        tal(brok)?;
        brok.chars().chain("00".chars()).take(3).collect::<String>().parse().ok()?
    };
    Some(dag_ms(dato)? + h * 3_600_000 + mi * 60_000 + s * 1000 + ms)
}

/// Tidsstempel rundet ned til hel time (ISO, UTC) — W `tilTimen`.
pub fn til_timen(t: &str) -> Option<String> {
    let ms = parse_iso(t)?;
    Some(iso(ms - ms.rem_euclid(3_600_000)).replace(".000Z", "Z"))
}

// ── Tekst som i JavaScript ─────────────────────────────────────────────────────────────────────────────────────────
/// JavaScripts `\s` (WhiteSpace + LineTerminator).
fn js_mellemrum(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(js_mellemrum)
}

/// `s.slice(0, n)` målt i UTF-16-enheder som i JavaScript (et tegnpar, der ville blive delt, udelades).
pub(crate) fn js_slice(s: &str, n: usize) -> String {
    let mut brugt = 0;
    s.chars()
        .take_while(|c| {
            brugt += c.len_utf16();
            brugt <= n
        })
        .collect()
}

// ── Rensning ───────────────────────────────────────────────────────────────────────────────────────────────────────
static LANGT: LazyLock<Regex> = LazyLock::new(|| re(r"[A-Za-z0-9_\-]{32,}"));
static SIKKER_STI: LazyLock<Regex> = LazyLock::new(|| re(r"^/[\w.\-/]{0,60}$"));
static KATEGORI_STI: LazyLock<Regex> = LazyLock::new(|| re(r"^\[[a-z]+\]$"));

/// En sti uden forespørgselsstreng, uden styretegn, med lange token-agtige stykker skjult, højst 90 tegn — W `renSti`.
pub fn ren_sti(s: &str) -> String {
    let foer = s.split('?').next().unwrap_or("");
    let mut p = String::with_capacity(foer.len());
    let mut i_mellemrum = false;
    for c in foer.chars().filter(|&c| c > '\u{1f}' && c != '\u{7f}') {
        if js_mellemrum(c) {
            if !i_mellemrum {
                p.push(' ');
            }
            i_mellemrum = true;
        } else {
            p.push(c);
            i_mellemrum = false;
        }
    }
    let p = LANGT.replace_all(&p, "…");
    let p = if p.starts_with('/') { p.into_owned() } else { format!("/{p}") };
    js_slice(&p, 90)
}

/// En angribers sti vises kun, hvis den er kort og ufarlig; ellers som kategori i skarpe parenteser — W `offentligSti`.
pub fn offentlig_sti(s: &str) -> String {
    if SIKKER_STI.is_match(s) || KATEGORI_STI.is_match(s) {
        return s.to_string();
    }
    format!("[{}]", forsoeg_kategori(s).unwrap_or("andet"))
}

// ── Bots: navn og firma ud fra brugeragenten ───────────────────────────────────────────────────────────────────────
type BotRegel = (Regex, &'static str, &'static str);
static BOTS: LazyLock<Vec<BotRegel>> = LazyLock::new(|| {
    [
        ("Googlebot-Image", "Googlebot-Image", "Google"),
        ("Googlebot", "Googlebot", "Google"),
        ("Google-InspectionTool", "Google-InspectionTool", "Google"),
        ("GoogleOther", "GoogleOther", "Google"),
        ("Storebot-Google", "Storebot-Google", "Google"),
        ("AdsBot-Google", "AdsBot-Google", "Google"),
        ("Google-Read-Aloud", "Google Read Aloud", "Google"),
        ("Google-Extended", "Google-Extended", "Google"),
        ("APIs-Google|Google-Safety", "Google (tjenester)", "Google"),
        ("bingbot", "Bingbot", "Microsoft"),
        ("BingPreview", "BingPreview", "Microsoft"),
        ("msnbot", "MSNBot", "Microsoft"),
        ("DuckAssistBot", "DuckAssistBot", "DuckDuckGo"),
        ("DuckDuckBot", "DuckDuckBot", "DuckDuckGo"),
        ("YandexBot|YandexImages", "YandexBot", "Yandex"),
        ("Baiduspider", "Baiduspider", "Baidu"),
        ("Applebot-Extended", "Applebot-Extended", "Apple"),
        ("Applebot", "Applebot", "Apple"),
        ("PetalBot", "PetalBot", "Huawei"),
        ("SeznamBot", "SeznamBot", "Seznam"),
        ("Sogou", "Sogou", "Sogou"),
        ("Qwantbot|Qwantify", "Qwantbot", "Qwant"),
        ("Claude-SearchBot", "Claude-SearchBot", "Anthropic"),
        ("Claude-User", "Claude-User", "Anthropic"),
        ("ClaudeBot|anthropic-ai", "ClaudeBot", "Anthropic"),
        ("OAI-SearchBot", "OAI-SearchBot", "OpenAI"),
        ("ChatGPT-User", "ChatGPT-User", "OpenAI"),
        ("OAI-AdsBot", "OAI-AdsBot", "OpenAI"),
        ("GPTBot", "GPTBot", "OpenAI"),
        ("Perplexity-User", "Perplexity-User", "Perplexity"),
        ("PerplexityBot", "PerplexityBot", "Perplexity"),
        ("MistralAI-User", "MistralAI-User", "Mistral"),
        ("Amazonbot", "Amazonbot", "Amazon"),
        ("CCBot", "CCBot", "Common Crawl"),
        ("Bytespider", "Bytespider", "ByteDance"),
        ("TikTokSpider", "TikTok Spider", "ByteDance"),
        ("meta-externalagent", "Meta-ExternalAgent", "Meta"),
        ("meta-externalfetcher", "Meta-ExternalFetcher", "Meta"),
        ("facebookexternalhit|FacebookBot", "Facebook", "Meta"),
        ("Qwenbot", "Qwenbot", "Alibaba"),
        ("YouBot", "YouBot", "You"),
        ("cohere-ai", "Cohere", "Cohere"),
        ("Timpibot", "Timpibot", "Timpi"),
        ("Twitterbot", "Twitterbot", "X"),
        ("LinkedInBot", "LinkedInBot", "LinkedIn"),
        ("Slackbot|Slack-ImgProxy", "Slackbot", "Slack"),
        ("Discordbot", "Discordbot", "Discord"),
        ("TelegramBot", "TelegramBot", "Telegram"),
        ("WhatsApp", "WhatsApp", "Meta"),
        ("Pinterestbot", "Pinterestbot", "Pinterest"),
        ("AhrefsBot|AhrefsSiteAudit", "AhrefsBot", "Ahrefs"),
        ("SemrushBot|SiteAuditBot", "SemrushBot", "Semrush"),
        ("MJ12bot", "MJ12bot", "Majestic"),
        ("DotBot", "DotBot", "Moz"),
        ("rogerbot", "Rogerbot", "Moz"),
        ("serpstatbot", "Serpstatbot", "Serpstat"),
        ("DataForSeoBot", "DataForSeoBot", "DataForSEO"),
        (r"archive\.org_bot|ia_archiver", "archive.org_bot", "Internet Archive"),
        ("UptimeRobot", "UptimeRobot", "UptimeRobot"),
        ("Pingdom", "Pingdom", "SolarWinds"),
        (
            "Cloudflare-Healthchecks|Cloudflare-Traffic-Manager|CloudflareObservatory|Cloudflare-Radar",
            "Cloudflare",
            "Cloudflare",
        ),
        ("HeadlessChrome", "Headless Chrome", ""),
        ("Lighthouse|Chrome-Lighthouse", "Lighthouse", "Google"),
    ]
    .into_iter()
    .map(|(p, navn, firma)| (re(&format!("(?i){p}")), navn, firma))
    .collect()
});

/// Kendte navne, som Cloudflare verificerer: kalder noget sig dét uden at være verificeret, udgiver det sig for at være det.
const VERIFICERBARE: &[&str] = &[
    "Googlebot",
    "Googlebot-Image",
    "Google-InspectionTool",
    "GoogleOther",
    "Storebot-Google",
    "AdsBot-Google",
    "Bingbot",
    "BingPreview",
    "DuckDuckBot",
    "DuckAssistBot",
    "YandexBot",
    "Baiduspider",
    "Applebot",
    "Applebot-Extended",
    "PetalBot",
    "ClaudeBot",
    "Claude-User",
    "Claude-SearchBot",
    "GPTBot",
    "OAI-SearchBot",
    "ChatGPT-User",
    "OAI-AdsBot",
    "PerplexityBot",
    "Perplexity-User",
    "Amazonbot",
    "CCBot",
    "Bytespider",
    "Meta-ExternalAgent",
    "Facebook",
    "AhrefsBot",
    "SemrushBot",
    "archive.org_bot",
];
static UKENDT_BOT: LazyLock<Regex> = LazyLock::new(|| re(r"([A-Za-z][\w.-]{1,30}(?:bot|Bot|BOT|crawler|spider)[\w.-]{0,20})(?:/[\d.]+)?"));

#[derive(Debug, Clone, PartialEq)]
pub struct Bot {
    pub navn: String,
    pub firma: String,
}

/// Navn og firma for en brugeragent; `kat` er Cloudflares verificerede kategori ('' = ikke verificeret) — W `botNavn`.
pub fn bot_navn(ua: &str, kat: &str) -> Bot {
    if let Some((_, navn, firma)) = BOTS.iter().find(|(r, _, _)| r.is_match(ua)) {
        return Bot {
            navn: (*navn).into(),
            firma: (*firma).into(),
        };
    }
    if !kat.is_empty() {
        let navn = if kat == "AI Assistant" {
            "AI-assistent i en browser".into()
        } else {
            format!("Verificeret ({kat})")
        };
        return Bot {
            navn,
            firma: String::new(),
        };
    }
    let navn = UKENDT_BOT
        .captures(ua)
        .map_or_else(|| "Ukendt bot".to_string(), |m| m[1].to_string());
    Bot {
        navn,
        firma: String::new(),
    }
}

/// Er en ikke-verificeret brugeragent en efterligning af en kendt bot — W `efterligning`.
pub fn efterligning(navn: &str) -> bool {
    VERIFICERBARE.contains(&navn)
}

// ── Regler: "React - RCE - CVE:CVE-2025-55182" → produkt, type, CVE ─────────────────────────────────────────────────
static CVE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)CVE-\d{4}-\d{4,7}"));
static CVE_DEL: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\s*-?\s*CVE:CVE-\d{4}-\d{4,7}"));
static SKILLE: LazyLock<Regex> = LazyLock::new(|| re(r"\s+-\s+"));

#[derive(Debug, Clone, PartialEq)]
pub struct RegelInfo {
    pub navn: String,
    pub produkt: String,
    pub type_: String,
    pub cve: Option<String>,
}

/// W `regel`.
pub fn regel(description: &str, source: &str) -> RegelInfo {
    let d = js_trim(description);
    if d.is_empty() {
        let bic = source == "bic";
        let navn = if bic {
            "Browser Integrity Check"
        } else if source.is_empty() {
            "Ukendt regel"
        } else {
            source
        };
        return RegelInfo {
            navn: navn.into(),
            produkt: String::new(),
            type_: if bic { "bic".into() } else { String::new() },
            cve: None,
        };
    }
    let cve = CVE.find(d).map(|m| m.as_str().to_uppercase());
    let uden = CVE_DEL.replacen(d, 1, "");
    let dele: Vec<&str> = SKILLE.split(&uden).map(js_trim).filter(|x| !x.is_empty()).collect();
    RegelInfo {
        navn: js_slice(d, 90),
        produkt: dele.first().map(|s| s.to_string()).unwrap_or_default(),
        type_: dele.get(1..).map(|r| r.join(" – ")).unwrap_or_default(),
        cve,
    }
}

// ── Forsøg, der bare fik 404: hvad ledte de efter? ─────────────────────────────────────────────────────────────────
static UFARLIG: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^/(favicon\.ico|apple-touch-icon[\w.-]*\.png|robots\.txt|sitemap\.xml|ads\.txt|browserconfig\.xml)$"));
static K_HEMMELIG: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"\.env|appsettings|secrets?|credentials|config\.(json|ya?ml|php|js)|\.aws|\.ssh|id_rsa|\.npmrc|docker-compose|web\.config|\.htpasswd|wp-config",
    )
});
static K_KILDE: LazyLock<Regex> = LazyLock::new(|| re(r"\.git\b|\.git/|\.svn|\.hg/|\.ds_store"));
static K_WP: LazyLock<Regex> = LazyLock::new(|| re(r"wp-|wordpress|xmlrpc|/wp/"));
static K_PHP: LazyLock<Regex> = LazyLock::new(|| re(r"phpinfo|_profiler|phpmyadmin|adminer|\.php\b"));
static K_ADGANG: LazyLock<Regex> = LazyLock::new(|| re(r"admin|login|signin|manager|console|actuator|cgi-bin|/shell|boaform|hnap|\.cgi"));

/// W `forsoegKategori`: `None` for ufarlige 404 (favicon, robots.txt …).
pub fn forsoeg_kategori(sti: &str) -> Option<&'static str> {
    let s = sti.to_lowercase();
    if UFARLIG.is_match(&s) {
        return None;
    }
    Some(if K_HEMMELIG.is_match(&s) {
        "hemmeligheder"
    } else if K_KILDE.is_match(&s) {
        "kildekode"
    } else if K_WP.is_match(&s) {
        "wordpress"
    } else if K_PHP.is_match(&s) {
        "php"
    } else if K_ADGANG.is_match(&s) {
        "adgang"
    } else {
        "andet"
    })
}

// ── GraphQL: ét UTC-døgn pr. forespørgsel, alle værdier som variabler ───────────────────────────────────────────────
/// Samme aliaser, grænser, filtre og felter som W `doegnForespoergsel` (vagt.js:121-134), plus `sampleInterval` på `fw` (V-SAMPLE.1); kun zone og tider er variabler.
/// HAM tæller Cloudflares estimat (summen af `sampleInterval`); W tæller rækker. Tallene afviger derfor på travle domæner.
pub const DOEGN_QUERY: &str = r#"query Vaernet($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    fw: firewallEventsAdaptive(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until}, orderBy:[datetime_DESC]) { action source description clientCountryName clientRequestPath clientRequestHTTPMethodName clientASNDescription edgeColoName datetime sampleInterval }
    alle: httpRequestsAdaptiveGroups(limit:1, filter:{datetime_geq:$since, datetime_lt:$until}, orderBy:[count_DESC]) { count dimensions { date } }
    probes: httpRequestsAdaptiveGroups(limit:400, filter:{datetime_geq:$since, datetime_lt:$until, edgeResponseStatus:404}, orderBy:[count_DESC]) { count dimensions { clientRequestPath } }
    bots: httpRequestsAdaptiveGroups(limit:1500, filter:{datetime_geq:$since, datetime_lt:$until, verifiedBotCategory_neq:""}, orderBy:[count_DESC]) { count dimensions { verifiedBotCategory userAgent clientRequestPath edgeResponseStatus } }
    f1: httpRequestsAdaptiveGroups(limit:300, filter:{datetime_geq:$since, datetime_lt:$until, verifiedBotCategory:"", userAgent_like:"%bot%"}, orderBy:[count_DESC]) { count dimensions { userAgent } }
    f2: httpRequestsAdaptiveGroups(limit:300, filter:{datetime_geq:$since, datetime_lt:$until, verifiedBotCategory:"", userAgent_like:"%Bot%"}, orderBy:[count_DESC]) { count dimensions { userAgent } }
    f3: httpRequestsAdaptiveGroups(limit:200, filter:{datetime_geq:$since, datetime_lt:$until, verifiedBotCategory:"", userAgent_like:"%crawler%"}, orderBy:[count_DESC]) { count dimensions { userAgent } }
  } } }"#;

/// Variablerne til ét døgn: `{zoneTag, since, until}`.
pub fn doegn_variabler(zone: &str, dag: &str) -> Result<Value, Fejl> {
    let fra = dag_ms(dag).ok_or(Fejl::UgyldigDato)?;
    Ok(json!({ "zoneTag": zone, "since": format!("{dag}T00:00:00Z"), "until": format!("{}T00:00:00Z", dag_noegle(fra + DOEGN_MS)) }))
}

/// Zone-id'et sendes kun som variabel, men tjekkes alligevel: 1–64 ASCII-bogstaver og -tal.
pub fn tjek_zone(id: &str) -> Result<String, Fejl> {
    if !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        Ok(id.to_string())
    } else {
        Err(Fejl::UgyldigZone)
    }
}

pub(crate) fn tom_ved_null<'de, D: Deserializer<'de>, T: Deserialize<'de> + Default>(d: D) -> Result<T, D::Error> {
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

/// `sampleInterval` som heltal. Et decimaltal rundes; alt andet (tekst, null) er 0, så ét mærkeligt felt aldrig koster hele døgnets svar.
fn mild_vaegt<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let v = Value::deserialize(d)?;
    Ok(v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)).unwrap_or(0))
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Fw {
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub action: String,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub source: String,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub description: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientCountryName")]
    pub land: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientRequestPath")]
    pub sti: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientRequestHTTPMethodName")]
    pub metode: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientASNDescription")]
    pub asn: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "edgeColoName")]
    pub colo: String,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub datetime: String,
    /// Cloudflares `sampleInterval`: hændelsen står for så mange (1 = talt én gang). Kun i `DOEGN_QUERY`; mangler det, er det 0.
    #[serde(default, deserialize_with = "mild_vaegt", rename = "sampleInterval")]
    pub vaegt: i64,
}

impl Fw {
    /// Hvor mange hændelser denne ene række står for: `sampleInterval`, eller 1, hvis feltet mangler eller er 0.
    pub fn antal(&self) -> i64 {
        self.vaegt.max(1)
    }
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Dim {
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientRequestPath")]
    pub sti: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "verifiedBotCategory")]
    pub bot_kategori: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "userAgent")]
    pub ua: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "edgeResponseStatus")]
    pub status: i64,
}

/// `sum { edgeResponseBytes }`: kun på aliaserne i `sti200::hoved_query`.
#[derive(Deserialize, Default, Debug, Clone)]
pub struct Sum {
    #[serde(default, deserialize_with = "tom_ved_null", rename = "edgeResponseBytes")]
    pub bytes: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Gruppe {
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub count: i64,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub dimensions: Dim,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub sum: Sum,
}

/// Ét døgns svar for zonen (`data.viewer.zones[0]`).
#[derive(Deserialize, Default, Debug, Clone)]
pub struct Doegn {
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub fw: Vec<Fw>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub alle: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub probes: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub bots: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub f1: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub f2: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub f3: Vec<Gruppe>,
    /// Følsomme filer med svar 200 og forsiden (`sti200::hoved_query`). Tomme i W's forespørgsel.
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub sti200: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub forside: Vec<Gruppe>,
}

pub fn doegn_fra_svar(data: &Value) -> Result<Doegn, Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    Doegn::deserialize(z).map_err(|_| Fejl::Svar)
}

// ── Tæller med JavaScripts Map-rækkefølge ──────────────────────────────────────────────────────────────────────────
/// Som en JS `Map<string, number>`: nøgler beholder pladsen fra første gang, de blev sat. `top` sorterer stabilt.
#[derive(Default, Debug, Clone)]
pub(crate) struct Taeller {
    pub(crate) orden: Vec<(String, i64)>,
    indeks: HashMap<String, usize>,
}

impl Taeller {
    fn plads(&mut self, k: &str) -> &mut i64 {
        let i = match self.indeks.get(k) {
            Some(&i) => i,
            None => {
                self.indeks.insert(k.to_string(), self.orden.len());
                self.orden.push((k.to_string(), 0));
                self.orden.len() - 1
            }
        };
        &mut self.orden[i].1
    }
    pub(crate) fn inc(&mut self, k: &str, n: i64) {
        *self.plads(k) += n;
    }
    fn saet(&mut self, k: &str, n: i64) {
        *self.plads(k) = n;
    }
    pub(crate) fn top(&self, n: usize) -> Vec<(String, i64)> {
        let mut v = self.orden.clone();
        v.sort_by_key(|x| Reverse(x.1));
        v.truncate(n);
        v
    }
}

// ── Døgnrækker ─────────────────────────────────────────────────────────────────────────────────────────────────────
/// Et døgns svar → rækker (art, nøgle, n) — W `doegnRaekker`, men art `fw` tæller Cloudflares estimat (`Fw::antal`, summen af
/// `sampleInterval`) og ikke rækker. Uden `sampleInterval` i svaret er tallene W's. Ren funktion.
pub fn doegn_raekker(z: &Doegn) -> Vec<(String, String, i64)> {
    let mut rk = Taeller::default();
    let mut add = |art: &str, noegle: &str, n: i64| rk.inc(&format!("{art}\u{1}{noegle}"), n);
    for e in &z.fw {
        let r = regel(&e.description, &e.source);
        let land = if e.land.is_empty() { "XX" } else { &e.land };
        let noegle = [
            r.navn.as_str(),
            land,
            &js_slice(&e.asn, 60),
            &ren_sti(&e.sti),
            &e.colo,
            &e.metode,
            &e.source,
            &e.action,
        ]
        .join("\t");
        add("fw", &noegle, e.antal());
    }
    add("alle", "", z.alle.first().map_or(0, |g| g.count));
    for g in &z.probes {
        let s = ren_sti(&g.dimensions.sti);
        if forsoeg_kategori(&s).is_some() {
            add("probe", &s, g.count);
        }
    }
    for g in &z.bots {
        let d = &g.dimensions;
        let b = bot_navn(&d.ua, &d.bot_kategori);
        add(
            "bot",
            &[b.navn, b.firma, d.bot_kategori.clone(), ren_sti(&d.sti), d.status.to_string()].join("\t"),
            g.count,
        );
    }
    let mut set = Taeller::default(); // samme agent kan stå i både %bot% og %Bot%
    for g in z.f1.iter().chain(&z.f2).chain(&z.f3) {
        set.saet(&g.dimensions.ua, g.count);
    }
    for (ua, n) in &set.orden {
        let b = bot_navn(ua, "");
        let slags = if efterligning(&b.navn) { "efterligning" } else { "uverificeret" };
        add("falsk", &format!("{}\t{slags}", b.navn), *n);
    }
    rk.orden
        .into_iter()
        .map(|(k, n)| {
            let (art, noegle) = k.split_once('\u{1}').unwrap_or((&k, ""));
            (art.to_string(), noegle.to_string(), n)
        })
        .collect()
}

/// Så mange egentlige angreb gemmes pr. domæne og døgn.
pub const ANGREB_MAKS: usize = 200;

/// Kun firewall-delen af `DOEGN_QUERY`: samme felter, filter og grænse, til de gamle døgn, der mangler enkelthændelser. Ingen IP-adresse og ingen brugeragent.
pub const ANGREB_QUERY: &str = r#"query Angreb($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    fw: firewallEventsAdaptive(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until}, orderBy:[datetime_DESC]) { action source description clientCountryName clientRequestPath clientRequestHTTPMethodName clientASNDescription edgeColoName datetime }
  } } }"#;

/// De `ANGREB_MAKS` seneste egentlige angreb i døgnet som rækker med art `angreb`: tid (hel time), regel, CVE, land, sti (renset, J2), datacenter, metode, kilde, handling og netværk (som i art `fw`). Hændelser med samme
/// nøgle lægges sammen. `z.fw` står nyeste først (`orderBy`). Rutine er ikke med, og der er aldrig IP-adresser eller brugeragent.
pub fn angreb_raekker(z: &Doegn) -> Vec<(String, String, i64)> {
    let mut t = Taeller::default();
    for e in z.fw.iter().filter(|e| !RUTINE.contains(&e.source.as_str())).take(ANGREB_MAKS) {
        let r = regel(&e.description, &e.source);
        let land = if e.land.is_empty() { "XX" } else { &e.land };
        let tid = til_timen(&e.datetime).unwrap_or_default();
        let sti = offentlig_sti(&ren_sti(&e.sti));
        t.inc(
            &[
                tid.as_str(),
                &r.navn,
                r.cve.as_deref().unwrap_or(""),
                land,
                &sti,
                &e.colo,
                &e.metode,
                &e.source,
                &e.action,
                &js_slice(&e.asn, 60),
            ]
            .join("\t"),
            1,
        );
    }
    t.orden.into_iter().map(|(k, n)| ("angreb".to_string(), k, n)).collect()
}

/// De 12 seneste blokeringer i døgnet som rækker med art `seneste`, allerede i offentlig form (sti renset, tid rundet ned
/// til hel time). W gemmer dem i KV; her ligger de i cachen, så de også kan vises ved næste start.
pub fn seneste_raekker(z: &Doegn) -> Vec<(String, String, i64)> {
    let mut t = Taeller::default();
    for e in z.fw.iter().take(12) {
        let r = regel(&e.description, &e.source);
        let land = if e.land.is_empty() { "XX" } else { &e.land };
        let tid = til_timen(&e.datetime).unwrap_or_default();
        let sti = offentlig_sti(&ren_sti(&e.sti));
        t.inc(
            &[
                tid.as_str(),
                &r.navn,
                r.cve.as_deref().unwrap_or(""),
                land,
                &sti,
                &e.colo,
                &e.metode,
            ]
            .join("\t"),
            1,
        );
    }
    t.orden.into_iter().map(|(k, n)| ("seneste".to_string(), k, n)).collect()
}

/// Art med antallet af hændelser (rækker) i døgnets `fw`-svar, nøgle "". Kun til 10.000-loftet (`crate::dyk::LOFT`): art `fw` er
/// Cloudflares estimat og kan passere 10.000, uden at svaret er afkortet. Døgn hentet før V-SAMPLE.1 har den ikke.
pub const FW_HAENDELSER: &str = "fwn";

/// Rækken med antal hændelser i døgnets svar (art `FW_HAENDELSER`).
pub fn haendelser_raekke(z: &Doegn) -> (String, String, i64) {
    (FW_HAENDELSER.to_string(), String::new(), z.fw.len() as i64)
}

/// "Overskriv aldrig fulde tal med reducerede": står det nye svar på loftet (`crate::dyk::LOFT` hændelser, de ældste timer
/// mangler), og er dets `fw`-sum lavere end den gemte, beholdes de gemte `fw`-rækker. Returnerer, om de blev beholdt. Ren funktion.
pub fn behold_fuldt_fw(raekker: &mut Vec<(String, String, i64)>, gemt_fw: &[(String, i64)], haendelser: usize) -> bool {
    if (haendelser as i64) < crate::dyk::LOFT {
        return false;
    }
    let ny: i64 = raekker.iter().filter(|r| r.0 == "fw").map(|r| r.2).sum();
    let gammel: i64 = gemt_fw.iter().map(|r| r.1).sum();
    if gammel <= ny {
        return false;
    }
    raekker.retain(|r| r.0 != "fw");
    raekker.extend(gemt_fw.iter().map(|(k, n)| ("fw".to_string(), k.clone(), *n)));
    true
}

/// V-SAMPLE.1, én gang: de færdige døgn i hentevinduet (fra `HENT_DAGE` dage siden til i forgårs) hentes igen i køens normale
/// tempo, så snit og median ikke sammenligner Cloudflares estimat med rækketal. I dag og i går hentes alligevel. Ældre døgn kan
/// Cloudflare ikke levere igen; de beholder rækketallene, og den første dag med estimater gemmes (`store::FW_VAEGT`).
/// Returnerer, om det skete nu.
pub fn fw_vaegt_en_gang(lager: &mut Store, nu: i64) -> Result<bool, Fejl> {
    lager.fw_vaegt_en_gang(&dag_noegle(nu - HENT_DAGE * DOEGN_MS), &dag_noegle(nu - DOEGN_MS))
}

// ── Oversigten (samme JSON-form som `data` i W's /api/maskine/vaern) ───────────────────────────────────────────────
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Perioder {
    pub d1: i64,
    pub d7: i64,
    pub d30: i64,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Alle {
    pub d7: i64,
    pub d30: i64,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Dag {
    pub dag: String,
    pub n: i64,
    pub alle: i64,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct StiN {
    pub sti: String,
    pub n: i64,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct KodeN {
    pub kode: String,
    pub n: i64,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct NavnN {
    pub navn: String,
    pub n: i64,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Bue {
    pub land: String,
    pub colo: String,
    pub n: i64,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Regel {
    pub navn: String,
    pub produkt: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub cve: Option<String>,
    pub n7: i64,
    pub n30: i64,
    pub foerst: String,
    pub sidst: String,
    pub stier: Vec<StiN>,
    pub lande: Vec<KodeN>,
    /// De næste fem findes ikke hos W: de sættes kun i regelkortenes egen liste (`HeleLister::h_regler`), aldrig i
    /// `Vaern::regler`, og står derfor ikke i W's JSON (som `Seneste.domaene`). "rutine", "egen" eller "angreb".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art: Option<String>,
    /// Afvist på 7 og 30 hele døgn, samme tal som i dyk (J1, J9).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h7: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h30: Option<i64>,
    /// Sider leveret med skjulte fælde-links på 30 hele døgn (J9); de tæller ikke i `h30`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faelder30: Option<i64>,
    /// Afvist i dag hidtil; tæller ikke i `h7` og `h30`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idag: Option<i64>,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Kategori {
    pub id: String,
    pub n: i64,
    pub stier: Vec<StiN>,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Forsoeg {
    pub i7: i64,
    pub kategorier: Vec<Kategori>,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Seneste {
    pub t: Option<String>,
    pub regel: String,
    pub cve: Option<String>,
    pub land: String,
    pub sti: String,
    pub colo: String,
    pub metode: String,
    /// Kun i det samlede Værn: hvilket domæne der blev ramt (tilføjes ved bygning; cachens nøgleformat er uændret).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domaene: Option<String>,
    /// Kun enkelthændelser fra art `angreb`: hvor mange med samme nøgle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i64>,
}
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Vaern {
    pub genereret: String,
    pub perioder: Perioder,
    pub alle: Alle,
    pub dage: Vec<Dag>,
    pub regler: Vec<Regel>,
    pub lande: Vec<KodeN>,
    pub netvaerk: Vec<NavnN>,
    pub stier: Vec<StiN>,
    pub colos: Vec<KodeN>,
    pub buer: Vec<Bue>,
    pub metoder: BTreeMap<String, i64>,
    pub kilder: BTreeMap<String, i64>,
    pub forsoeg: Forsoeg,
    pub seneste: Vec<Seneste>,
}

fn sti_n(v: Vec<(String, i64)>) -> Vec<StiN> {
    v.into_iter().map(|(sti, n)| StiN { sti, n }).collect()
}
fn kode_n(v: Vec<(String, i64)>) -> Vec<KodeN> {
    v.into_iter().map(|(kode, n)| KodeN { kode, n }).collect()
}

struct RegelSum {
    info: RegelInfo,
    navn: String,
    n7: i64,
    n30: i64,
    foerst: String,
    sidst: String,
    stier: Taeller,
    lande: Taeller,
}

/// W `byggVaern`. Ren funktion.
pub fn bygg_vaern(rows: &[Raekke], nu: i64, seneste: Vec<Seneste>) -> Vaern {
    let d1 = dag_noegle(nu);
    let d7 = dag_noegle(nu - 6 * DOEGN_MS);
    let dage_liste: Vec<String> = (0..DAGE).map(|i| dag_noegle(nu - (DAGE - 1 - i) * DOEGN_MS)).collect();
    let mut per_dag: HashMap<&str, i64> = dage_liste.iter().map(|d| (d.as_str(), 0)).collect();
    let mut alle_dag = per_dag.clone();
    let mut regler: Vec<RegelSum> = Vec::new();
    let mut regel_i: HashMap<String, usize> = HashMap::new();
    let (mut lande, mut net, mut stier, mut colos, mut par, mut probes) = (
        Taeller::default(),
        Taeller::default(),
        Taeller::default(),
        Taeller::default(),
        Taeller::default(),
        Taeller::default(),
    );
    let (mut metoder, mut kilder) = (BTreeMap::<String, i64>::new(), BTreeMap::<String, i64>::new());
    let (mut p1, mut p7, mut p30, mut probe7, mut alle7, mut alle30) = (0, 0, 0, 0, 0, 0);
    for r in rows {
        let dag = r.dag.as_str();
        match r.art.as_str() {
            "alle" => {
                if let Some(v) = alle_dag.get_mut(dag) {
                    *v += r.n; // én række pr. zone pr. døgn: for én zone som W, for flere zoner summen
                }
                alle30 += r.n;
                if dag >= d7.as_str() {
                    alle7 += r.n;
                }
                continue;
            }
            "probe" => {
                if dag >= d7.as_str() {
                    probes.inc(&r.noegle, r.n);
                    probe7 += r.n;
                }
                continue;
            }
            "fw" => {}
            _ => continue,
        }
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        let (navn, land, asn, sti, colo, metode, kilde) = (felt(0), felt(1), felt(2), felt(3), felt(4), felt(5), felt(6));
        let n = r.n;
        p30 += n;
        if dag >= d7.as_str() {
            p7 += n;
        }
        if dag == d1 {
            p1 += n;
        }
        if let Some(v) = per_dag.get_mut(dag) {
            *v += n;
        }
        let i = *regel_i.entry(navn.to_string()).or_insert_with(|| {
            let bic = navn == "Browser Integrity Check";
            regler.push(RegelSum {
                info: regel(if bic { "" } else { navn }, if bic { "bic" } else { "" }),
                navn: navn.to_string(),
                n7: 0,
                n30: 0,
                foerst: r.dag.clone(),
                sidst: r.dag.clone(),
                stier: Taeller::default(),
                lande: Taeller::default(),
            });
            regler.len() - 1
        });
        let rg = &mut regler[i];
        rg.n30 += n;
        if dag >= d7.as_str() {
            rg.n7 += n;
        }
        if dag < rg.foerst.as_str() {
            rg.foerst = r.dag.clone();
        }
        if dag > rg.sidst.as_str() {
            rg.sidst = r.dag.clone();
        }
        rg.stier.inc(sti, n);
        rg.lande.inc(land, n);
        lande.inc(land, n);
        if !asn.is_empty() {
            net.inc(asn, n);
        }
        stier.inc(sti, n);
        if !colo.is_empty() {
            colos.inc(colo, n);
            par.inc(&format!("{land}>{colo}"), n);
        }
        if !metode.is_empty() {
            *metoder.entry(metode.into()).or_default() += n;
        }
        *kilder.entry(if kilde.is_empty() { "?" } else { kilde }.into()).or_default() += n;
    }
    let mut kat: Vec<(&'static str, i64, Taeller)> = Vec::new();
    for (sti, n) in &probes.orden {
        let Some(k) = forsoeg_kategori(sti) else { continue };
        let i = kat.iter().position(|(id, _, _)| *id == k).unwrap_or_else(|| {
            kat.push((k, 0, Taeller::default()));
            kat.len() - 1
        });
        kat[i].1 += n;
        kat[i].2.orden.push((sti.clone(), *n)); // W skubber [sti, n] på en liste (ingen sammenlægning)
    }
    regler.sort_by_key(|r| Reverse(r.n30));
    kat.sort_by_key(|k| Reverse(k.1));
    Vaern {
        genereret: iso(nu),
        perioder: Perioder { d1: p1, d7: p7, d30: p30 },
        alle: Alle { d7: alle7, d30: alle30 },
        dage: dage_liste
            .iter()
            .map(|d| Dag {
                dag: d.clone(),
                n: per_dag[d.as_str()],
                alle: alle_dag[d.as_str()],
            })
            .collect(),
        regler: regler
            .into_iter()
            .take(12)
            .map(|r| Regel {
                navn: r.navn,
                produkt: r.info.produkt,
                type_: r.info.type_,
                cve: r.info.cve,
                n7: r.n7,
                n30: r.n30,
                foerst: r.foerst,
                sidst: r.sidst,
                stier: sti_n(r.stier.top(3)),
                lande: kode_n(r.lande.top(3)),
                ..Regel::default()
            })
            .collect(),
        lande: kode_n(lande.top(15)),
        netvaerk: net.top(8).into_iter().map(|(navn, n)| NavnN { navn, n }).collect(),
        stier: sti_n(stier.top(12)),
        colos: kode_n(colos.top(20)),
        buer: par
            .top(60)
            .into_iter()
            .map(|(k, n)| {
                let mut d = k.split('>');
                Bue {
                    land: d.next().unwrap_or("").into(),
                    colo: d.next().unwrap_or("").into(),
                    n,
                }
            })
            .collect(),
        metoder,
        kilder,
        forsoeg: Forsoeg {
            i7: probe7,
            kategorier: kat
                .into_iter()
                .map(|(id, n, s)| Kategori {
                    id: id.into(),
                    n,
                    stier: sti_n(s.top(6)),
                })
                .collect(),
        },
        seneste: seneste.into_iter().take(12).collect(),
    }
}

/// [{sti, n}] med offentlige stier; ens stier efter rensningen lægges sammen — W `stiListe`.
fn sti_liste(liste: Vec<StiN>) -> Vec<StiN> {
    let mut t = Taeller::default();
    for x in liste {
        t.inc(&offentlig_sti(&x.sti), x.n);
    }
    sti_n(t.orden)
}

/// Værnets oversigt med angribernes stier i sikker form og tider rundet ned til hel time — W `offentligVaern`. Idempotent.
pub fn offentlig_vaern(mut v: Vaern) -> Vaern {
    for r in &mut v.regler {
        r.stier = sti_liste(std::mem::take(&mut r.stier));
    }
    v.stier = sti_liste(std::mem::take(&mut v.stier));
    for k in &mut v.forsoeg.kategorier {
        k.stier = sti_liste(std::mem::take(&mut k.stier));
    }
    for s in &mut v.seneste {
        s.sti = offentlig_sti(&s.sti);
        s.t = s.t.as_deref().and_then(til_timen);
    }
    v
}

/// Rækker med art `seneste` → de 12 nyeste blokeringer (nyeste først).
pub fn seneste_fra_raekker(rows: &[Raekke]) -> Vec<Seneste> {
    let mut ud: Vec<Seneste> = Vec::new();
    for r in rows.iter().filter(|r| r.art == "seneste") {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("").to_string();
        let s = Seneste {
            t: Some(felt(0)).filter(|t| !t.is_empty()),
            regel: felt(1),
            cve: Some(felt(2)).filter(|c| !c.is_empty()),
            land: felt(3),
            sti: felt(4),
            colo: felt(5),
            metode: felt(6),
            domaene: None,
            n: None,
        };
        for _ in 0..r.n.clamp(0, 12) {
            ud.push(s.clone());
        }
    }
    ud.sort_by(|a, b| b.t.cmp(&a.t));
    ud.truncate(12);
    ud
}

/// Oversigten, som `vaern_oversigt` returnerer: byggVaern + offentligVaern over cachen; `genereret` = sidst hentet.
pub fn oversigt(rows: &[Raekke], nu: i64, sidst_hentet: Option<String>) -> Vaern {
    let mut v = offentlig_vaern(bygg_vaern(rows, nu, seneste_fra_raekker(rows)));
    if let Some(h) = sidst_hentet {
        v.genereret = h;
    }
    v
}

/// Første dag i oversigtens vindue (30 døgn inkl. i dag, som W).
pub fn vindue_fra(nu: i64) -> String {
    dag_noegle(nu - (DAGE - 1) * DOEGN_MS)
}

/// Værnet til webvisningen: W's felter uændrede (paritetstestet) og de hele døgn ved siden af (`i_gaar`, `i_dag_hidtil`,
/// `hele7`, `hele30`, `fordeling` …), plus angrebene i går og i forgårs til guidens "Se de N angreb" (`angreb_dage`).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct VaernUd {
    #[serde(flatten)]
    pub vaern: Vaern,
    #[serde(flatten)]
    pub hele: Hele,
    #[serde(flatten)]
    pub lister: HeleLister,
    pub angreb_dage: Vec<AngrebDag>,
    /// Døgn med usædvanlig meget trafik. Medianen er ét domænes egen, så det samlede Angreb har ingen.
    pub trafik: Vec<TrafikDag>,
    /// De 30 hele døgn, der ikke er hentet for nogen af zonerne: huller, ikke 0. Sættes af `med_huller`.
    pub uhentet: Vec<String>,
}

impl VaernUd {
    /// `hentet`: dagene, mindst én af zonerne har tal for.
    pub fn med_huller<'a>(&mut self, hentet: impl Iterator<Item = &'a String>, nu: i64) {
        let set: std::collections::HashSet<&String> = hentet.collect();
        self.uhentet = (1..=HENT_DAGE)
            .rev()
            .map(|i| dag_noegle(nu - i * DOEGN_MS))
            .filter(|d| !set.contains(d))
            .collect();
    }
}

/// Søjler, lister og kort i Angreb over de samme hele døgn og det samme "afvist" som dyk (J1, J9), ved siden af W's felter,
/// der tæller d-29…i dag og alt i `fw`. Så er tallet ens før og efter klikket. Lande og datacentre er alle, ikke W's top 15 og
/// top 20, så kortet ikke skjuler noget; buerne er stadig de 60 største af `h_buer_i_alt`.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct HeleLister {
    /// Regelkortene: alle regler, der slog til på de hele døgn eller i dag, størst først.
    pub h_regler: Vec<Regel>,
    pub h_dage: Vec<Dag>,
    pub h_lande: Vec<KodeN>,
    pub h_netvaerk: Vec<NavnN>,
    pub h_colos: Vec<KodeN>,
    pub h_buer: Vec<Bue>,
    pub h_buer_i_alt: usize,
    /// "Hvad de ledte efter" over 7 hele døgn i offentlig form, så kategorier og stier har samme tal i dyk.
    pub h_forsoeg: Forsoeg,
    /// I dag: en regel set kun i dag åbner dyk på den dag.
    pub h_idag: String,
}

impl HeleLister {
    /// "Først set" og "sidst set" på regelkortene over hele historikken, som i dyk. `datoer`: se `Store::fw_datoer`.
    pub fn med_historik(&mut self, datoer: &[(String, String, String)]) {
        let mut pr_regel: HashMap<&str, (&str, &str)> = HashMap::new();
        for (noegle, foerst, sidst) in datoer {
            let f: Vec<&str> = noegle.split('\t').collect();
            if !er_afvist(f.get(7).copied().unwrap_or("")) {
                continue;
            }
            let e = pr_regel.entry(f[0]).or_insert((foerst, sidst));
            e.0 = e.0.min(foerst);
            e.1 = e.1.max(sidst);
        }
        for rg in &mut self.h_regler {
            if let Some((foerst, sidst)) = pr_regel.get(rg.navn.as_str()) {
                rg.foerst = foerst.to_string();
                rg.sidst = sidst.to_string();
            }
        }
    }
}

/// `rows` skal gå fra og med `hele_fra(nu)`. Samme optælling som W's `bygg_vaern`, men over d-30…d-1 og kun det afviste.
pub fn hele_lister(rows: &[Raekke], nu: i64) -> HeleLister {
    let (fra, til) = (dag_noegle(nu - HENT_DAGE * DOEGN_MS), dag_noegle(nu - DOEGN_MS));
    let afvist = |r: &&Raekke| r.art != "fw" || er_afvist(r.noegle.split('\t').nth(7).unwrap_or(""));
    let hele: Vec<Raekke> = rows
        .iter()
        .filter(|r| r.dag >= fra && r.dag <= til)
        .filter(afvist)
        .cloned()
        .collect();
    let v = offentlig_vaern(bygg_vaern(&hele, nu - DOEGN_MS, Vec::new()));
    let (mut lande, mut colos, mut par) = (Taeller::default(), Taeller::default(), Taeller::default());
    for r in hele.iter().filter(|r| r.art == "fw") {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        lande.inc(felt(1), r.n);
        if !felt(4).is_empty() {
            colos.inc(felt(4), r.n);
            par.inc(&format!("{}>{}", felt(1), felt(4)), r.n);
        }
    }
    HeleLister {
        h_regler: hele_regler(rows, nu),
        h_dage: v.dage,
        h_lande: kode_n(lande.top(usize::MAX)),
        h_netvaerk: v.netvaerk,
        h_colos: kode_n(colos.top(usize::MAX)),
        h_buer: v.buer,
        h_buer_i_alt: par.orden.len(),
        h_forsoeg: v.forsoeg,
        h_idag: dag_noegle(nu),
    }
}

#[derive(Default)]
pub(crate) struct KortSum {
    pub(crate) navn: String,
    pub(crate) kilde: String,
    pub(crate) h7: i64,
    pub(crate) h30: i64,
    pub(crate) faelder30: i64,
    pub(crate) idag: i64,
    pub(crate) foerst: String,
    pub(crate) sidst: String,
    pub(crate) stier: Taeller,
    pub(crate) lande: Taeller,
}

/// Regelkortene bygges af de hele døgn (J1) og dagen i dag, ikke af W's top 12: afvist på 7 og 30 hele døgn, siderne med
/// fælde-links og i dag hidtil. En regel, der kun er set i dag, får sit kort. Størst først. `foerst` og `sidst` dækker kun
/// rækkerne her; `HeleLister::med_historik` sætter dem over hele historikken. Ren funktion.
fn hele_regler(rows: &[Raekke], nu: i64) -> Vec<Regel> {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    let (d1, d7, d30) = (d(1), d(7), d(HENT_DAGE));
    let mut kort: Vec<KortSum> = Vec::new();
    let mut plads: HashMap<&str, usize> = HashMap::new();
    for r in rows.iter().filter(|r| r.art == "fw" && r.dag >= d30) {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        let (dag, hel, afvist) = (r.dag.as_str(), r.dag <= d1, er_afvist(felt(7)));
        if !hel && !afvist {
            continue;
        }
        let i = *plads.entry(felt(0)).or_insert_with(|| {
            kort.push(KortSum {
                navn: felt(0).to_string(),
                kilde: felt(6).to_string(),
                ..KortSum::default()
            });
            kort.len() - 1
        });
        let k = &mut kort[i];
        if hel && gruppe(felt(7)) == FAELDER {
            k.faelder30 += r.n;
        }
        if !afvist {
            continue;
        }
        if k.foerst.is_empty() || dag < k.foerst.as_str() {
            k.foerst = r.dag.clone();
        }
        if dag > k.sidst.as_str() {
            k.sidst = r.dag.clone();
        }
        if !hel {
            k.idag += r.n;
            continue;
        }
        k.h30 += r.n;
        if r.dag >= d7 {
            k.h7 += r.n;
        }
        k.stier.inc(felt(3), r.n);
        k.lande.inc(felt(1), r.n);
    }
    regelkort(kort)
}

/// Summerne → kortene, størst først. Dyk tæller selv sit udsnit og bruger samme kort.
pub(crate) fn regelkort(mut kort: Vec<KortSum>) -> Vec<Regel> {
    kort.sort_by(|a, b| (b.h30, b.idag).cmp(&(a.h30, a.idag)).then_with(|| a.navn.cmp(&b.navn)));
    kort.into_iter()
        .map(|k| {
            let bic = k.navn == "Browser Integrity Check";
            let info = regel(if bic { "" } else { &k.navn }, if bic { "bic" } else { "" });
            Regel {
                produkt: crate::teknologi::vist_produkt(&info.produkt),
                type_: info.type_,
                cve: info.cve,
                n7: k.h7,
                n30: k.h30,
                stier: sti_liste(sti_n(k.stier.top(3))),
                lande: kode_n(k.lande.top(3)),
                art: Some(regel_art(&k.kilde).to_string()),
                h7: Some(k.h7),
                h30: Some(k.h30),
                faelder30: Some(k.faelder30),
                idag: Some(k.idag),
                navn: k.navn,
                foerst: k.foerst,
                sidst: k.sidst,
            }
        })
        .collect()
}

/// En regel, der stoppede egentlige angreb en bestemt dag (fuldt navn som regelkortenes `navn`).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct RegelDag {
    pub navn: String,
    pub n: i64,
}

/// Rækker med art `angreb` → enkelthændelser, nyeste først.
pub fn angreb_fra_raekker(rows: &[Raekke]) -> Vec<Seneste> {
    let mut ud: Vec<Seneste> = rows
        .iter()
        .filter(|r| r.art == "angreb")
        .map(|r| {
            let f: Vec<&str> = r.noegle.split('\t').collect();
            let felt = |i: usize| f.get(i).copied().unwrap_or("").to_string();
            Seneste {
                t: Some(felt(0)).filter(|t| !t.is_empty()),
                regel: felt(1),
                cve: Some(felt(2)).filter(|c| !c.is_empty()),
                land: felt(3),
                sti: felt(4),
                colo: felt(5),
                metode: felt(6),
                domaene: None,
                n: Some(r.n),
            }
        })
        .collect();
    ud.sort_by(|a, b| b.t.cmp(&a.t));
    ud
}

/// Egentlige angreb (ikke rutine) én UTC-dag, som statusreglen tæller dem (`overblik::angreb`). `seneste` er de enkelte
/// angreb, cachen har gemt for dagen: højst 12 hændelser pr. dag gemmes (`seneste_raekker`, også rutine), så listen kan
/// være kortere end `angreb`; rutine sorteres fra på navnet (`RUTINE_NAVNE`), fordi de gemte hændelser ikke har kilden.
/// `regler`: reglerne bag dagens angreb, størst først (til visningen, når ingen enkelte angreb er gemt).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct AngrebDag {
    pub dag: String,
    pub angreb: i64,
    pub seneste: Vec<Seneste>,
    pub regler: Vec<RegelDag>,
}

/// Rutinens navne i de gemte enkelthændelser: `regel(beskrivelse, kilde)` giver en tom beskrivelse kildens navn
/// ("linkMaze", "botFight") og `bic` navnet "Browser Integrity Check". Samme tre kilder som `overblik::RUTINE`.
pub const RUTINE_NAVNE: &[&str] = &["linkMaze", "botFight", "Browser Integrity Check"];

/// Angrebene i går (d-1) og i forgårs (d-2) ud fra cachens rækker. Ren funktion.
pub fn angreb_dage(rows: &[Raekke], nu: i64) -> Vec<AngrebDag> {
    (1..=2)
        .map(|i| {
            let dag = dag_noegle(nu - i * DOEGN_MS);
            let a = angreb(rows, nu - (i - 1) * DOEGN_MS);
            let paa_dagen: Vec<Raekke> = rows.iter().filter(|r| r.dag == dag).cloned().collect();
            // De gemte angreb, hvis de er hentet; ellers de 12 seneste af alle slags uden rutine.
            let gemte = angreb_fra_raekker(&paa_dagen);
            let seneste = if gemte.is_empty() {
                seneste_fra_raekker(&paa_dagen)
                    .into_iter()
                    .filter(|s| !RUTINE_NAVNE.contains(&s.regel.as_str()))
                    .collect()
            } else {
                gemte
            };
            let mut t = Taeller::default();
            for r in paa_dagen.iter().filter(|r| r.art == "fw") {
                let f: Vec<&str> = r.noegle.split('\t').collect();
                if !RUTINE.contains(&f.get(6).copied().unwrap_or("")) {
                    t.inc(f.first().copied().unwrap_or(""), r.n);
                }
            }
            let mut regler: Vec<RegelDag> = t.orden.into_iter().map(|(navn, n)| RegelDag { navn, n }).collect();
            regler.sort_by_key(|r| Reverse(r.n));
            AngrebDag {
                dag,
                angreb: a.i_gaar,
                seneste,
                regler,
            }
        })
        .collect()
}

/// Én zone: `rows` fra og med `hele_fra(nu)` (d-30). W's oversigt får kun vinduet fra `vindue_fra(nu)` (d-29), som før.
pub fn oversigt_ud(rows: &[Raekke], nu: i64, sidst_hentet: Option<String>) -> VaernUd {
    let fra = vindue_fra(nu);
    let vindue: Vec<Raekke> = rows.iter().filter(|r| r.dag >= fra).cloned().collect();
    let vaern = oversigt(&vindue, nu, sidst_hentet);
    VaernUd {
        lister: hele_lister(rows, nu),
        vaern,
        hele: hele_doegn(rows, nu),
        angreb_dage: angreb_dage(rows, nu),
        trafik: trafik_spring(rows, nu),
        uhentet: Vec::new(),
    }
}

/// Det samlede Værn for flere zoner: rækkerne (med zone, fra `hele_fra(nu)`) lægges sammen i `bygg_vaern`; de seneste
/// blokeringer får domænenavnet; `genereret` er det ældste "sidst hentet" blandt zonerne med tal. Ren funktion.
pub fn samlet(rows: &[(String, Raekke)], navne: &HashMap<String, String>, nu: i64, genereret: String) -> VaernUd {
    let fra = vindue_fra(nu);
    let mut pr_zone: BTreeMap<&str, Vec<Raekke>> = BTreeMap::new();
    for (z, r) in rows.iter().filter(|(_, r)| r.art == "seneste" && r.dag >= fra) {
        pr_zone.entry(z.as_str()).or_default().push(r.clone());
    }
    let mut seneste: Vec<Seneste> = Vec::new();
    for (z, rk) in pr_zone {
        let navn = navne.get(z).cloned().unwrap_or_else(|| "…".into());
        seneste.extend(seneste_fra_raekker(&rk).into_iter().map(|s| Seneste {
            domaene: Some(navn.clone()),
            ..s
        }));
    }
    seneste.sort_by(|a, b| b.t.cmp(&a.t));
    seneste.truncate(12);
    let alle: Vec<Raekke> = rows.iter().map(|(_, r)| r.clone()).collect();
    let vindue: Vec<Raekke> = alle.iter().filter(|r| r.dag >= fra).cloned().collect();
    let mut v = offentlig_vaern(bygg_vaern(&vindue, nu, seneste));
    v.genereret = genereret;
    VaernUd {
        lister: hele_lister(&alle, nu),
        vaern: v,
        hele: hele_doegn(&alle, nu),
        angreb_dage: angreb_dage(&alle, nu),
        trafik: Vec::new(),
        uhentet: Vec::new(),
    }
}

// ── Cachen: hvad skal hentes? ──────────────────────────────────────────────────────────────────────────────────────
/// I dag og i går altid; ældre døgn i hentevinduet kun hvis de mangler — eller kun blev hentet, før døgnet var slut
/// (+1 time), for så er de ufuldstændige. Ren funktion (testes).
pub fn dage_at_hente(nu: i64, hentet: &HashMap<String, String>) -> Vec<String> {
    let dage: Vec<String> = (0..=HENT_DAGE).map(|i| dag_noegle(nu - i * DOEGN_MS)).collect();
    let mut ud = vec![dage[0].clone(), dage[1].clone()];
    for d in &dage[2..] {
        if !hentet.get(d).is_some_and(|h| er_faerdig(d, h)) {
            ud.push(d.clone());
        }
    }
    ud
}

/// Er døgnet færdighentet: hentet mindst `FAERDIG_MARGIN_MS` efter døgnets slutning?
pub fn er_faerdig(dag: &str, hentet_utc: &str) -> bool {
    let faerdig_efter = dag_ms(dag).map(|ms| iso(ms + DOEGN_MS + FAERDIG_MARGIN_MS)).unwrap_or_default();
    hentet_utc >= faerdig_efter.as_str()
}

fn laas(lager: &Mutex<Store>) -> Result<std::sync::MutexGuard<'_, Store>, Fejl> {
    lager.lock().map_err(|_| Fejl::Lager)
}

/// Hvad der skete med ét døgn i `opdater`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Doegnudfald<'a> {
    Gemt,
    /// Cloudflare har ikke længere data for døgnet; det springes over (ikke en fejl for zonen).
    UdenFor(&'a Fejl),
    Fejl(&'a Fejl),
}

/// Henter de døgn, cachen mangler (højst `SAMTIDIGE` ad gangen; den fælles begrænser i `cf` styrer tempoet), og gemmer
/// hvert døgn, så snart det er hentet. `paa_doegn` kaldes for hvert døgn. Fejler et døgn, fortsætter resten; den
/// første fejl returneres til sidst (døgn uden for Cloudflares opbevaring tæller ikke som fejl). Returnerer antal gemte.
pub async fn opdater<F, Fut>(
    lager: &Mutex<Store>,
    zone: &str,
    nu: i64,
    hent: F,
    mut paa_doegn: impl FnMut(&str, Doegnudfald<'_>),
) -> Result<usize, Fejl>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<Doegn, Fejl>>,
{
    let hentet = laas(lager)?.hentet(zone)?;
    let tid = iso(nu);
    let mut stroem = stream::iter(dage_at_hente(nu, &hentet))
        .map(|dag| {
            let f = hent(dag.clone());
            async move { (dag, f.await) }
        })
        .buffer_unordered(SAMTIDIGE);
    let (mut antal, mut foerste_fejl) = (0, None);
    while let Some((dag, svar)) = stroem.next().await {
        match svar {
            Ok(z) => {
                let mut raekker = doegn_raekker(&z);
                raekker.extend(seneste_raekker(&z));
                raekker.extend(angreb_raekker(&z));
                raekker.extend(crate::sti200::raekker(&z.sti200, &z.forside));
                raekker.push(haendelser_raekke(&z));
                let mut l = laas(lager)?;
                if z.fw.len() as i64 >= crate::dyk::LOFT {
                    let gemt = l.doegn_art(zone, &dag, "fw")?;
                    behold_fuldt_fw(&mut raekker, &gemt, z.fw.len());
                }
                l.gem_doegn(zone, &dag, &raekker, &tid)?;
                antal += 1;
                paa_doegn(&dag, Doegnudfald::Gemt);
            }
            Err(e) if e.uden_for_opbevaring() => paa_doegn(&dag, Doegnudfald::UdenFor(&e)),
            Err(e) => {
                paa_doegn(&dag, Doegnudfald::Fejl(&e));
                foerste_fejl.get_or_insert(e);
            }
        }
    }
    foerste_fejl.map_or(Ok(antal), Err)
}

/// Seneste `hentet_utc` blandt døgnene i oversigtens vindue.
pub fn sidst_hentet(hentet: &HashMap<String, String>, nu: i64) -> Option<String> {
    let fra = vindue_fra(nu);
    hentet.iter().filter(|(d, _)| **d >= fra).map(|(_, h)| h.clone()).max()
}

#[cfg(test)]
mod tests;
