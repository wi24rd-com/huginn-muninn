//! Cloudflares egne sikkerhedsfund (Security Center Insights, IDEA-036, V-TOKEN.1). Kræver den valgfri rettighed "Account Security Center Insights"
//! (læs) på tokenet: uden den svarer Cloudflare 403 (målt: kode 10000 "Authentication error"), og afsnittet siger, at rettigheden kan tilføjes.
//! Resten af programmet er uberørt.
//!
//! Målt live (INVESTIGATION-V-TOKEN.1 §A3): kun kontoniveauet virker (`GET accounts/{id}/security-center/insights`); zone-niveauet giver 403
//! med samme token. Fundene fordeles derfor på `payload.zone_tag`. Kontoens id står i `account.id` i `/zones`-svaret og bruges kun i
//! adressen: det gemmes ikke. Fundets `id` indeholder zone-id'et og logges aldrig. `resolve_link` og `resolve_text` var tomme i alle målte fund, så
//! titlen er programmets egen oversættelse af `issue_class` (sprog.js), og linket er Cloudflares faste side (`DASH`).
use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use crate::cf::Fejl;

/// Cloudflares egen genvej til Security Insights for den konto, brugeren er logget ind på (developers.cloudflare.com/security/security-insights/review-insights/).
/// Ingen konto-id i adressen: `:account` udfyldes af Cloudflare.
pub const DASH: &str = "https://dash.cloudflare.com/?to=/:account/security-center";
/// Højst så ofte hentes fundene (Cloudflare skanner hver 7. dag på Free og hver 3. på Pro og Business).
pub const INTERVAL_MS: i64 = 60 * 60 * 1000;
/// `per_page`'s største værdi (API-referencen) og højst så mange sider pr. konto.
pub const PR_SIDE: usize = 1000;
pub const MAKS_SIDER: usize = 5;
/// I `indstillinger`: hvornår fundene sidst blev forsøgt hentet, og hvordan det gik (`ok`, `mangler_ret` eller `fejl:<kode>`).
pub const HENTET: &str = "sikkerhedsfund_hentet";
pub const TILSTAND: &str = "sikkerhedsfund_tilstand";
/// Alvor, der giver Hold øje. Cloudflares værdier er `Low`, `Moderate` og `Critical` (how-it-works-siden); tællesiden nævner også `high`, så
/// den tæller med, hvis den nogensinde kommer.
const HOEJ: &[&str] = &["critical", "high"];

/// Ét aktivt fund for én zone.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Fund {
    pub zone: String,
    /// Cloudflares id (indeholder zone-id'et). Kun nøgle i tabellen; sendes ikke til webvisningen.
    #[serde(skip)]
    pub id: String,
    pub alvor: String,
    pub klasse: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub siden: String,
}

fn tekst<'a>(v: &'a Value, felt: &str) -> &'a str {
    v.get(felt).and_then(Value::as_str).unwrap_or("")
}

/// Kun bogstaver, tal, `_`, `-` og mellemrum, højst 80 tegn: alt fra Cloudflare er data, aldrig tekst, der vises ukontrolleret.
fn ren(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ' ' | ':' | '.'))
        .take(80)
        .collect()
}

/// `result.issues` → aktive fund, der ikke er afvist, med en zone. Og `result.count` (alle fund, også på andre sider). Ren funktion.
pub fn fra_svar(j: &Value) -> Result<(Vec<Fund>, usize), Fejl> {
    let r = j.get("result").ok_or(Fejl::Svar)?;
    let issues = r.get("issues").and_then(Value::as_array).ok_or(Fejl::Svar)?;
    let count = r.get("count").and_then(Value::as_u64).map_or(issues.len(), |c| c as usize);
    let fund = issues
        .iter()
        .filter(|i| i.get("dismissed").and_then(Value::as_bool) != Some(true))
        .filter(|i| matches!(tekst(i, "status"), "" | "active"))
        .filter_map(|i| {
            let zone = i.pointer("/payload/zone_tag").and_then(Value::as_str)?;
            (!zone.is_empty() && zone.len() <= 64 && zone.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| Fund {
                zone: zone.to_string(),
                id: ren(tekst(i, "id")),
                alvor: ren(tekst(i, "severity")),
                klasse: ren(tekst(i, "issue_class")),
                type_: ren(tekst(i, "issue_type")),
                siden: ren(tekst(i, "since")).chars().take(10).collect(),
            })
        })
        .collect();
    Ok((fund, count))
}

/// De forskellige konti i et `/zones`-svar (`result[].account.id`). Ren funktion.
pub fn konti(j: &Value) -> Vec<String> {
    j.get("result")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|z| z.pointer("/account/id").and_then(Value::as_str))
        .filter(|id| !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric()))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Er det tid at hente igen? Højst én gang pr. `INTERVAL_MS`, regnet fra sidste forsøg. Ren funktion.
pub fn skal_hente(sidst: Option<i64>, nu: i64) -> bool {
    sidst.is_none_or(|s| nu - s >= INTERVAL_MS || s > nu)
}

/// Hvad der står i `TILSTAND` efter et forsøg. 401/403 er den manglende rettighed; alt andet er en fejl, og de gemte fund bliver stående. Ren funktion.
pub fn tilstand_efter(e: Option<&Fejl>) -> String {
    match e {
        None => "ok".into(),
        Some(Fejl::Afvist(_)) => "mangler_ret".into(),
        Some(f) => format!("fejl:{}", f.kode()),
    }
}

/// Domænets fund med høj alvor: giver mindst Hold øje, aldrig Handl (Cloudflares egne fund er ofte konfigurationsråd).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct HoejeFund {
    pub n: usize,
    /// `issue_class` for hvert fund, til teksten.
    pub klasser: Vec<String>,
}

pub fn er_hoej(alvor: &str) -> bool {
    HOEJ.contains(&alvor.to_ascii_lowercase().as_str())
}

/// Ren funktion.
pub fn hoeje(fund: &[Fund]) -> Option<HoejeFund> {
    let h: Vec<&Fund> = fund.iter().filter(|f| er_hoej(&f.alvor)).collect();
    (!h.is_empty()).then(|| HoejeFund {
        n: h.len(),
        klasser: h.iter().map(|f| f.klasse.clone()).collect(),
    })
}

/// Afsnittet "Cloudflares egne fund" på Angreb for ét domæne.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct FundUd {
    /// Højeste alvor først.
    pub fund: Vec<Fund>,
    /// `ok`, `mangler_ret`, `fejl` eller `venter` (aldrig hentet).
    pub tilstand: String,
    /// Fejlkoden ved `fejl`.
    pub fejl: Option<String>,
    pub hentet: Option<String>,
}

fn rang(alvor: &str) -> u8 {
    match alvor.to_ascii_lowercase().as_str() {
        "critical" => 0,
        "high" => 1,
        "moderate" => 2,
        "low" => 3,
        _ => 4,
    }
}

/// Ren funktion.
pub fn oversigt(mut fund: Vec<Fund>, tilstand: Option<&str>, hentet: Option<String>) -> FundUd {
    fund.sort_by(|a, b| rang(&a.alvor).cmp(&rang(&b.alvor)).then_with(|| a.klasse.cmp(&b.klasse)));
    let (tilstand, fejl) = match tilstand {
        None => ("venter".to_string(), None),
        Some(t) => match t.strip_prefix("fejl:") {
            Some(k) => ("fejl".to_string(), Some(k.to_string())),
            None => (t.to_string(), None),
        },
    };
    FundUd {
        fund,
        tilstand,
        fejl,
        hentet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Renset svar fra en målt konto (INVESTIGATION-V-TOKEN.1 §A3): zone-id → z1/z3, domæner → wi24rd.com-navne.
    fn maalt() -> Value {
        json!({"success": true, "errors": [], "messages": [], "result": {"count": 3, "page": 1, "per_page": 1000, "issues": [
            {"id": "z3-security_txt_not_enabled", "issue_class": "security_txt_not_enabled", "issue_type": "configuration_suggestion",
             "payload": {"zone_tag": "z3"}, "severity": "Low", "subject": "abe.wi24rd.com", "timestamp": "2026-11-05T13:58:23.451139Z",
             "dismissed": false, "since": "2026-04-24T23:10:01.507824Z", "resolve_link": "", "resolve_text": "", "status": "active", "source": "cloudflare"},
            {"id": "z1-always_use_https_not_enabled", "issue_class": "always_use_https_not_enabled", "issue_type": "insecure_configuration",
             "payload": {"zone_tag": "z1"}, "severity": "Critical", "subject": "wi24rd.com", "timestamp": "2026-11-05T13:57:53.721298Z",
             "dismissed": false, "since": "2026-04-24T23:10:01.108941Z", "resolve_link": "", "resolve_text": "", "status": "active", "source": "cloudflare"},
            {"id": "z1-gammel", "issue_class": "x", "issue_type": "configuration_suggestion", "payload": {"zone_tag": "z1"}, "severity": "Critical",
             "dismissed": true, "since": "2026-01-01T00:00:00Z", "status": "active"}
        ]}})
    }

    #[test]
    fn svaret_bliver_til_aktive_fund_med_zone() {
        let (fund, count) = fra_svar(&maalt()).unwrap();
        assert_eq!(count, 3);
        assert_eq!(fund.len(), 2, "et afvist fund er væk");
        assert_eq!(
            fund[0],
            Fund {
                zone: "z3".into(),
                id: "z3-security_txt_not_enabled".into(),
                alvor: "Low".into(),
                klasse: "security_txt_not_enabled".into(),
                type_: "configuration_suggestion".into(),
                siden: "2026-04-24".into(),
            }
        );
        let v = serde_json::to_value(&fund[0]).unwrap();
        assert!(v.get("id").is_none(), "id'et (med zone-id) sendes ikke til webvisningen");
        assert_eq!(v["type"], "configuration_suggestion");
        let loest = json!({"result": {"issues": [{"payload": {"zone_tag": "z1"}, "status": "resolved", "severity": "Critical"}]}});
        assert_eq!(fra_svar(&loest).unwrap().0, vec![], "løste fund tæller ikke");
        let uden_zone = json!({"result": {"issues": [{"payload": {}, "severity": "Critical"}, {"payload": {"zone_tag": "../x"}}]}});
        assert_eq!(
            fra_svar(&uden_zone).unwrap().0,
            vec![],
            "uden en gyldig zone står fundet ingen steder"
        );
        assert_eq!(fra_svar(&json!({"result": {}})), Err(Fejl::Svar));
        assert_eq!(fra_svar(&json!({})), Err(Fejl::Svar));
    }

    #[test]
    fn tekst_fra_cloudflare_renses() {
        let j = json!({"result": {"issues": [{"payload": {"zone_tag": "z1"}, "severity": "<b>Critical</b>", "issue_class": "a\"; alert(1)//", "since": "x"}]}});
        let (fund, _) = fra_svar(&j).unwrap();
        assert_eq!(fund[0].alvor, "bCriticalb");
        assert_eq!(fund[0].klasse, "a alert1");
        assert!(ren(&"a".repeat(200)).len() == 80);
    }

    #[test]
    fn konti_fra_zonelisten() {
        let j = json!({"result": [
            {"id": "z1", "account": {"id": "k1"}}, {"id": "z2", "account": {"id": "k1"}}, {"id": "z3", "account": {"id": "k2"}},
            {"id": "z4"}, {"id": "z5", "account": {"id": "../ondt"}}
        ]});
        assert_eq!(konti(&j), vec!["k1".to_string(), "k2".to_string()]);
        assert!(konti(&json!({})).is_empty());
    }

    #[test]
    fn hoejst_en_gang_i_timen() {
        let nu = 1_000_000_000_000;
        assert!(skal_hente(None, nu));
        assert!(!skal_hente(Some(nu - INTERVAL_MS + 1), nu));
        assert!(skal_hente(Some(nu - INTERVAL_MS), nu));
        assert!(
            skal_hente(Some(nu + 5_000), nu),
            "et tidspunkt i fremtiden (uret er stillet) blokerer ikke for altid"
        );
    }

    #[test]
    fn manglende_rettighed_og_fejl() {
        assert_eq!(tilstand_efter(None), "ok");
        assert_eq!(tilstand_efter(Some(&Fejl::Afvist(403))), "mangler_ret");
        assert_eq!(tilstand_efter(Some(&Fejl::Afvist(401))), "mangler_ret");
        assert_eq!(tilstand_efter(Some(&Fejl::CfNede(502))), "fejl:cf_nede");
        let o = oversigt(vec![], Some("fejl:cf_nede"), None);
        assert_eq!((o.tilstand.as_str(), o.fejl.as_deref()), ("fejl", Some("cf_nede")));
        assert_eq!(oversigt(vec![], None, None).tilstand, "venter");
        assert_eq!(oversigt(vec![], Some("mangler_ret"), None).tilstand, "mangler_ret");
    }

    #[test]
    fn kun_critical_og_high_giver_hold_oeje() {
        let (fund, _) = fra_svar(&maalt()).unwrap();
        assert_eq!(
            hoeje(&fund),
            Some(HoejeFund {
                n: 1,
                klasser: vec!["always_use_https_not_enabled".into()]
            })
        );
        let lave: Vec<Fund> = fund.iter().filter(|f| f.alvor == "Low").cloned().collect();
        assert_eq!(hoeje(&lave), None, "Low giver intet");
        assert!(er_hoej("critical") && er_hoej("High") && !er_hoej("Moderate") && !er_hoej("Low") && !er_hoej(""));
    }

    #[test]
    fn oversigten_har_hoejeste_alvor_foerst() {
        let (fund, _) = fra_svar(&maalt()).unwrap();
        let o = oversigt(fund, Some("ok"), Some("2026-11-08T09:00:00.000Z".into()));
        assert_eq!(o.fund.iter().map(|f| f.alvor.as_str()).collect::<Vec<_>>(), ["Critical", "Low"]);
        assert_eq!(o.tilstand, "ok");
    }

    #[test]
    fn linket_er_cloudflares_faste_side_uden_id() {
        let u = tauri::Url::parse(DASH).unwrap();
        assert_eq!((u.scheme(), u.host_str(), u.path()), ("https", Some("dash.cloudflare.com"), "/"));
        assert_eq!(u.query(), Some("to=/:account/security-center"));
    }
}
