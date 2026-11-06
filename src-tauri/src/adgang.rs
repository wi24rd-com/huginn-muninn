//! De ekstra tal: det, `vaern::DOEGN_QUERY` ikke giver, og som Robotter og Login skal bruge. `ADGANG_QUERY` er sin egen
//! forespørgsel, så en plan, der mangler et felt, kun koster de ekstra tal: Angreb og Overblik virker stadig. Den hentes kun
//! for færdige døgn og efter den normale runde (`koe::Koe::ekstra_runde`), og hvert døgn står i `hentet_ekstra`, ikke `hentet`.
//! Ingen IP-adresser. `clientASNDescription` findes ikke på `httpRequestsAdaptiveGroups` (målt live: authz-fejl på alle
//! domæner), så netværket er `clientAsn` der, og navnet slås op i `firewallEventsAdaptive` (art `asn`). Brugeragenter gemmes
//! aldrig: de bliver til et robotnavn med det samme.
use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};

use crate::cf::Fejl;
use crate::login::STARTLISTE;
use crate::overblik::er_afvist;
use crate::vaern::{bot_navn, dag_noegle, doegn_variabler, er_faerdig, iso, js_slice, ren_sti, tom_ved_null, Taeller, DOEGN_MS, HENT_DAGE};

/// Kun zone, tider og stier er variabler. `requestSource:"eyeball"` udelader Cloudflares interne trafik.
pub const ADGANG_QUERY: &str = r#"query Adgang($zoneTag: string!, $since: Time!, $until: Time!, $stier: [string!]!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    post: httpRequestsAdaptiveGroups(limit:500, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball", clientRequestHTTPMethodName:"POST"}, orderBy:[count_DESC]) { count dimensions { clientRequestPath edgeResponseStatus originResponseStatus clientCountryName clientAsn } }
    status: httpRequestsAdaptiveGroups(limit:50, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball"}, orderBy:[count_DESC]) { count dimensions { edgeResponseStatus } }
    lande: httpRequestsAdaptiveGroups(limit:20, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball"}, orderBy:[count_DESC]) { count dimensions { clientCountryName } }
    login: httpRequestsAdaptiveGroups(limit:500, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball", clientRequestPath_in:$stier}, orderBy:[count_DESC]) { count dimensions { clientRequestPath clientRequestHTTPMethodName edgeResponseStatus originResponseStatus clientCountryName clientAsn } }
    stoppet: firewallEventsAdaptive(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until}, orderBy:[datetime_DESC]) { action source userAgent clientAsn clientASNDescription }
  } } }"#;

/// Lækkede kodeord. Sin egen forespørgsel, så `ADGANG_QUERY` er uændret. Hverken kodeord, brugernavne eller IP-adresser findes i svaret. Uden sti-filter (Cloudflare afgør selv, hvad der er et login), og
/// uden krav om POST: siden 20. aug. 2026 skanner Cloudflare også `Authorization: Basic`, som kan være GET. Tre aliaser, så et rigtigt fund aldrig drukner i de andre svar:
/// `laek` er fundene med deres sti, `ren` er stier, hvor tjekket fandt et kodeord uden at finde det i et læk (`login::forslag`), og `ikke` er ét tal for `none`, kun for POST: på GET ville det bare være al trafikken.
/// `_notin` er en dokumenteret operator, men ikke prøvet på netop dette felt: afviser Cloudflare den, bruges `LAEK_QUERY_UDEN_FILTER`.
pub const LAEK_QUERY: &str = r#"query Laek($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    laek: httpRequestsAdaptiveGroups(limit:2000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball", leakedCredentialCheckResult_notin:["clean","none"]}, orderBy:[count_DESC]) { count dimensions { clientRequestPath originResponseStatus leakedCredentialCheckResult } }
    ren: httpRequestsAdaptiveGroups(limit:500, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball", leakedCredentialCheckResult:"clean"}, orderBy:[count_DESC]) { count dimensions { clientRequestPath leakedCredentialCheckResult } }
    ikke: httpRequestsAdaptiveGroups(limit:1, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball", clientRequestHTTPMethodName:"POST", leakedCredentialCheckResult:"none"}, orderBy:[count_DESC]) { count dimensions { leakedCredentialCheckResult } }
  } } }"#;

/// Fald tilbage uden filter på resultatet: `laek_raekker` sorterer selv, og metoden står med, så `none` stadig kun tæller POST. Grænsen er Cloudflares største side, for her kan fundene ligge bag al den øvrige trafik.
pub const LAEK_QUERY_UDEN_FILTER: &str = r#"query Laek($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    laek: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball"}, orderBy:[count_DESC]) { count dimensions { clientRequestPath clientRequestHTTPMethodName originResponseStatus leakedCredentialCheckResult } }
  } } }"#;

/// Afviste Cloudflare filteret? Så prøves `LAEK_QUERY_UDEN_FILTER`.
pub fn laek_filter_afvist(e: &Fejl) -> bool {
    matches!(e, Fejl::GraphQl { detalje, .. } if detalje.contains("leakedCredentialCheckResult"))
}

/// En værdi, Cloudflare ikke har dokumenteret, gemmes, som den er, hvis den ligner et nøgleord: uden den rå værdi kan det ikke afgøres, hvad en `ukendt`-række var. Alt andet, også et tomt svar, er `ukendt`. Ren funktion.
fn laek_vaerdi(v: &str) -> &str {
    if !v.is_empty() && v.len() <= 40 && v.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
        v
    } else {
        "ukendt"
    }
}

/// Samme forespørgsel uden `clientAsn` på `httpRequestsAdaptiveGroups`, til domæner, hvor feltet afvises. Spiken prøvede kun
/// `clientASNDescription`, så det er først det rigtige svar, der viser, om `clientAsn` er tilladt.
pub fn query_uden_asn() -> String {
    ADGANG_QUERY.replace(" clientAsn }", " }")
}

/// Afviste Cloudflare et felt? Så prøves `query_uden_asn` for domænet, før det opgives.
pub fn felt_afvist(e: &Fejl) -> bool {
    matches!(e, Fejl::GraphQl { detalje, .. } if detalje.contains("clientAsn") || detalje.contains("does not have access"))
}

/// Stierne til `clientRequestPath_in`: hele startlisten og det, brugeren selv har slået til. Startlisten er altid med, så en
/// sti, der slås til senere, allerede har sin historik.
pub fn stier_til_hentning(valgte: &[(String, bool)]) -> Vec<String> {
    let mut ud: Vec<String> = STARTLISTE.iter().map(|s| s.to_string()).collect();
    for (sti, til) in valgte {
        if *til && !ud.contains(sti) {
            ud.push(sti.clone());
        }
    }
    ud
}

/// Variablerne til ét døgn.
pub fn variabler(zone: &str, dag: &str, stier: &[String]) -> Result<Value, Fejl> {
    let mut v = doegn_variabler(zone, dag)?;
    v["stier"] = json!(stier);
    Ok(v)
}

/// Mangler de ekstra tal for døgnet? Kun færdige døgn: i dag og døgn, der ikke er slut (+1 time), hentes ikke. Ren funktion.
pub fn mangler(dag: &str, hentet: &HashMap<String, String>, nu: i64) -> bool {
    er_faerdig(dag, &iso(nu)) && !hentet.contains_key(dag)
}

/// Døgnene i hentevinduet, nyeste først: i går til d-30.
pub fn vinduet(nu: i64) -> Vec<String> {
    (1..=HENT_DAGE).map(|i| dag_noegle(nu - i * DOEGN_MS)).collect()
}

/// `clientAsn` kan komme som tekst eller tal; 0 er ukendt.
fn asn_tekst<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(match Option::<Value>::deserialize(d)? {
        Some(Value::String(s)) if s != "0" => s,
        Some(Value::Number(n)) if n.as_i64() != Some(0) => n.to_string(),
        _ => String::new(),
    })
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Dim {
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientRequestPath")]
    pub sti: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientRequestHTTPMethodName")]
    pub metode: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "edgeResponseStatus")]
    pub status: i64,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "originResponseStatus")]
    pub origin: i64,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientCountryName")]
    pub land: String,
    #[serde(default, deserialize_with = "asn_tekst", rename = "clientAsn")]
    pub asn: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "leakedCredentialCheckResult")]
    pub laek: String,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Gruppe {
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub count: i64,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub dimensions: Dim,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Stoppet {
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub action: String,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub source: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "userAgent")]
    pub ua: String,
    #[serde(default, deserialize_with = "asn_tekst", rename = "clientAsn")]
    pub asn: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientASNDescription")]
    pub asn_navn: String,
}

/// Ét døgns svar for zonen (`data.viewer.zones[0]`).
#[derive(Deserialize, Default, Debug, Clone)]
pub struct Adgang {
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub post: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub status: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub lande: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub login: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    pub stoppet: Vec<Stoppet>,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct LaekSvar {
    #[serde(default, deserialize_with = "tom_ved_null")]
    laek: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    ren: Vec<Gruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    ikke: Vec<Gruppe>,
}

/// Cloudflares svar `none`. Står ikke i dokumentationen og er aldrig et læk: den tæller kun som en optælling uden sti.
pub const LAEK_IKKE_TJEKKET: &str = "none";

/// Rækkerne fra `LAEK_QUERY` og `LAEK_QUERY_UDEN_FILTER`. Art `laek`: "sti\tresultat\torigin", og `none` som én række uden sti. Art `laek_ren`: stien, hvor tjekket så et kodeord, der ikke var lækket.
/// En ukendt værdi er aldrig en fejl. Ren funktion.
pub fn laek_raekker(data: &Value) -> Result<Vec<(String, String, i64)>, Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    let svar = LaekSvar::deserialize(z).map_err(|_| Fejl::Svar)?;
    let (mut laek, mut ren) = (Taeller::default(), Taeller::default());
    let ikke_noegle = format!("\t{LAEK_IKKE_TJEKKET}\t");
    for g in svar.laek.iter().chain(&svar.ren).chain(&svar.ikke) {
        let d = &g.dimensions;
        let sti = js_slice(&ren_sti(&d.sti), 200);
        match d.laek.as_str() {
            "clean" => ren.inc(&sti, g.count),
            LAEK_IKKE_TJEKKET if d.metode.is_empty() || d.metode == "POST" => laek.inc(&ikke_noegle, g.count),
            LAEK_IKKE_TJEKKET => {}
            v => laek.inc(&format!("{sti}\t{}\t{}", laek_vaerdi(v), d.origin), g.count),
        }
    }
    let art = |t: Taeller, a: &str| t.orden.into_iter().map(|(k, n)| (a.to_string(), k, n)).collect::<Vec<_>>();
    Ok([art(laek, "laek"), art(ren, "laek_ren")].concat())
}

pub fn fra_svar(data: &Value) -> Result<Adgang, Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    Adgang::deserialize(z).map_err(|_| Fejl::Svar)
}

fn land_kode(land: &str) -> &str {
    if land.is_empty() {
        "XX"
    } else {
        land
    }
}

/// Et døgns svar → rækker (art, nøgle, n). `stoppet` tæller kun det afviste (`er_afvist`): en side med fælde-links er ikke
/// stoppet. Ren funktion.
pub fn raekker(a: &Adgang) -> Vec<(String, String, i64)> {
    let mut rk = Taeller::default();
    let mut add = |art: &str, noegle: &str, n: i64| rk.inc(&format!("{art}\u{1}{noegle}"), n);
    for g in &a.post {
        let d = &g.dimensions;
        let noegle = [
            ren_sti(&d.sti),
            d.status.to_string(),
            d.origin.to_string(),
            land_kode(&d.land).to_string(),
            d.asn.clone(),
        ];
        add("post", &noegle.join("\t"), g.count);
    }
    for g in &a.status {
        add("status", &g.dimensions.status.to_string(), g.count);
    }
    for g in &a.lande {
        add("land", land_kode(&g.dimensions.land), g.count);
    }
    for g in &a.login {
        let d = &g.dimensions;
        let noegle = [
            ren_sti(&d.sti),
            d.metode.clone(),
            d.status.to_string(),
            d.origin.to_string(),
            land_kode(&d.land).to_string(),
            d.asn.clone(),
        ];
        add("login", &noegle.join("\t"), g.count);
    }
    for e in &a.stoppet {
        if !e.asn.is_empty() && !e.asn_navn.is_empty() {
            add("asn", &format!("{}\t{}", e.asn, js_slice(&e.asn_navn, 60)), 1);
        }
        if er_afvist(&e.action) {
            let b = bot_navn(&e.ua, "");
            add("stoppet", &[b.navn, b.firma, e.source.clone(), e.action.clone()].join("\t"), 1);
        }
    }
    rk.orden
        .into_iter()
        .map(|(k, n)| {
            let (art, noegle) = k.split_once('\u{1}').unwrap_or((&k, ""));
            (art.to_string(), noegle.to_string(), n)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alias<'a>(q: &'a str, navn: &str) -> &'a str {
        let fra = q.find(&format!("{navn}: httpRequests")).unwrap();
        q[fra..].lines().next().unwrap()
    }

    /// Ingen sti-filter, og fundene kræver ikke POST, for Basic auth kan være GET.
    #[test]
    fn laek_query_uden_sti_og_uden_post_krav() {
        for q in [LAEK_QUERY, LAEK_QUERY_UDEN_FILTER] {
            assert!(!q.contains("$stier") && !q.contains("clientRequestPath_in"));
            assert!(
                !alias(q, "laek").contains(r#"clientRequestHTTPMethodName:"POST""#),
                "POST-kravet er væk"
            );
        }
        assert!(alias(LAEK_QUERY, "laek").contains(r#"leakedCredentialCheckResult_notin:["clean","none"]"#));
        assert!(alias(LAEK_QUERY, "ren").contains(r#"leakedCredentialCheckResult:"clean""#));
        assert!(alias(LAEK_QUERY, "ikke").contains(r#"clientRequestHTTPMethodName:"POST", leakedCredentialCheckResult:"none""#));
        assert!(!LAEK_QUERY_UDEN_FILTER.contains("_notin"));
        let afvist = |d: &str| Fejl::GraphQl {
            dag: None,
            detalje: d.into(),
        };
        assert!(laek_filter_afvist(&afvist("unknown field leakedCredentialCheckResult_notin")));
        assert!(!laek_filter_afvist(&afvist("noget andet")));
        assert!(!laek_filter_afvist(&Fejl::Vent));
    }

    /// Fundene deler ikke grænse med andre svar, og faldet tilbage bruger hele siden.
    #[test]
    fn laek_graensen_taber_ikke_et_rigtigt_fund() {
        assert!(alias(LAEK_QUERY, "laek").contains("limit:2000"));
        assert!(alias(LAEK_QUERY_UDEN_FILTER, "laek").contains("limit:10000"));
    }

    fn g(sti: &str, metode: &str, v: &str, n: i64) -> Value {
        json!({ "count": n, "dimensions": { "clientRequestPath": sti, "clientRequestHTTPMethodName": metode, "originResponseStatus": 0, "leakedCredentialCheckResult": v } })
    }

    /// Uden filteret sorteres der her: `clean` bliver en sti med kodeord, `none` ét tal, tomt `ukendt`, og en udokumenteret værdi gemmes rå, hvis den ligner et nøgleord.
    #[test]
    fn laek_raekker_sorterer_svarene() {
        let data = json!({ "viewer": { "zones": [{ "laek": [g("/a", "POST", "clean", 9), g("/a", "POST", "", 9), g("/b", "POST", "password_leaked", 2), g("/c", "POST", "lookup_timeout", 3), g("/d", "POST", "Ikke Et Ord!", 1), g("/", "POST", "none", 7), g("/x", "POST", "none", 1)] }] } });
        let rk = laek_raekker(&data).unwrap();
        assert_eq!(
            rk.iter().map(|r| (r.0.as_str(), r.1.as_str(), r.2)).collect::<Vec<_>>(),
            [
                ("laek", "/a\tukendt\t0", 9),
                ("laek", "/b\tpassword_leaked\t0", 2),
                ("laek", "/c\tlookup_timeout\t0", 3),
                ("laek", "/d\tukendt\t0", 1),
                ("laek", "\tnone\t", 8),
                ("laek_ren", "/a", 9),
            ]
        );
    }

    /// GET med Basic auth tæller som fund, mens `none` kun tæller POST: på GET er det al trafikken.
    #[test]
    fn laek_get_taeller_og_none_kun_paa_post() {
        let data = json!({ "viewer": { "zones": [{ "laek": [g("/beskyttet", "GET", "password_leaked", 4), g("/", "GET", "none", 90_000), g("/", "POST", "none", 5)] }] } });
        let rk = laek_raekker(&data).unwrap();
        assert_eq!(
            rk.iter().map(|r| (r.1.as_str(), r.2)).collect::<Vec<_>>(),
            [("/beskyttet\tpassword_leaked\t0", 4), ("\tnone\t", 5)]
        );
    }

    /// De tre aliaser lægges sammen, og `ikke` har ingen sti.
    #[test]
    fn laek_raekker_fra_de_tre_aliaser() {
        let u = |sti: &str, v: &str, n: i64| json!({ "count": n, "dimensions": { "clientRequestPath": sti, "leakedCredentialCheckResult": v } });
        let data = json!({ "viewer": { "zones": [{ "laek": [u("/login", "password_leaked", 2)], "ren": [u("/portal", "clean", 6)], "ikke": [{ "count": 1234, "dimensions": { "leakedCredentialCheckResult": "none" } }] }] } });
        let rk = laek_raekker(&data).unwrap();
        assert_eq!(
            rk.iter().map(|r| (r.0.as_str(), r.1.as_str(), r.2)).collect::<Vec<_>>(),
            [
                ("laek", "/login\tpassword_leaked\t0", 2),
                ("laek", "\tnone\t", 1234),
                ("laek_ren", "/portal", 6)
            ]
        );
    }

    /// Kun sti, resultat og origin-status, og en ukendt værdi er `ukendt`.
    #[test]
    fn laek_gemmer_intet_foelsomt() {
        let g = |sti: &str, res: &str, o: i64, n: i64| serde_json::json!({"count": n, "dimensions": {"clientRequestPath": sti, "originResponseStatus": o, "leakedCredentialCheckResult": res}});
        let data = serde_json::json!({"viewer": {"zones": [{"laek": [
            g("/wp-login.php", "password_leaked", 200, 3), g("/wp-login.php", "clean", 200, 500), g("/login", "", 0, 9),
            g("/login", "username_and_password_leaked", 403, 1), g("/login", "noget_nyt", 200, 2), g("/login", "username_password_similar", 200, 4),
        ]}]}});
        let r = laek_raekker(&data).unwrap();
        let tal = |k: &str| r.iter().find(|x| x.1 == k).map(|x| x.2);
        assert_eq!(r.len(), 6, "{r:?}");
        assert!(r.iter().all(|x| x.0.starts_with("laek")));
        assert_eq!(
            r.iter().find(|x| x.0 == "laek_ren").map(|x| (x.1.as_str(), x.2)),
            Some(("/wp-login.php", 500))
        );
        assert_eq!(tal("/wp-login.php\tpassword_leaked\t200"), Some(3));
        assert_eq!(tal("/login\tusername_and_password_leaked\t403"), Some(1));
        assert_eq!(tal("/login\tnoget_nyt\t200"), Some(2));
        assert_eq!(tal("/login\tusername_password_similar\t200"), Some(4));
        for q in [LAEK_QUERY, LAEK_QUERY_UDEN_FILTER] {
            for felt in ["clientIP", "userAgent", "username", "password"] {
                assert!(!q.replace("leakedCredentialCheckResult", "").contains(felt), "{felt}");
            }
        }
    }

    #[test]
    fn adgang_query_er_uden_laek() {
        assert!(!ADGANG_QUERY.contains("leakedCredentialCheckResult"));
    }
    use crate::store::EKSTRA_ARTER;
    use crate::vaern::{dag_ms, DOEGN_QUERY};

    /// 2026-11-04T08:00:00Z.
    const NU: i64 = 1_793_779_200_000;

    /// Nodernes tekst: hver linje i forespørgslen er én node.
    fn noder(q: &str) -> Vec<&str> {
        q.lines().filter(|l| l.contains("Adaptive")).collect()
    }

    #[test]
    fn ingen_ip_og_intet_asn_navn_paa_http_noden() {
        for q in [ADGANG_QUERY, query_uden_asn().as_str(), DOEGN_QUERY] {
            assert!(!q.to_lowercase().contains("clientip"), "beder aldrig om IP-adresser");
        }
        let alle = noder(ADGANG_QUERY);
        assert_eq!(alle.len(), 5, "højst 6 noder");
        let http: Vec<&&str> = alle.iter().filter(|l| l.contains("httpRequestsAdaptiveGroups")).collect();
        assert_eq!(http.len(), 4);
        for l in http {
            assert!(!l.contains("clientASNDescription"), "authz-fejl: {l}");
        }
        assert!(!noder(DOEGN_QUERY)
            .iter()
            .any(|l| l.contains("httpRequests") && l.contains("clientASNDescription")));
    }

    #[test]
    fn query_uden_asn_beholder_navnet_paa_firewall_noden() {
        let q = query_uden_asn();
        assert!(noder(&q)
            .iter()
            .filter(|l| l.contains("httpRequests"))
            .all(|l| !l.contains("clientAsn")));
        assert!(
            q.contains("userAgent clientAsn clientASNDescription"),
            "firewallEventsAdaptive: målt live"
        );
        assert_eq!(q.matches("originResponseStatus").count(), 2);
    }

    #[test]
    fn kun_afvist_felt_giver_nyt_forsoeg() {
        let g = |d: &str| Fejl::GraphQl {
            dag: None,
            detalje: d.into(),
        };
        assert!(felt_afvist(&g("zone does not have access to the field clientAsn")));
        assert!(felt_afvist(&g("unknown field clientAsn")));
        assert!(!felt_afvist(&g("cannot request data older than 2678400s")));
        assert!(!felt_afvist(&Fejl::Afvist(403)));
    }

    #[test]
    fn startlisten_er_altid_med_og_egne_stier_kun_naar_de_er_slaaet_til() {
        let v = [
            ("/mit-login".to_string(), true),
            ("/fra".to_string(), false),
            ("/login".to_string(), true),
        ];
        let s = stier_til_hentning(&v);
        assert_eq!(s.len(), STARTLISTE.len() + 1, "{s:?}");
        assert!(s.contains(&"/mit-login".to_string()) && !s.contains(&"/fra".to_string()));
        let var = variabler("abc", "2026-11-03", &s).unwrap();
        assert_eq!(var["stier"].as_array().unwrap().len(), s.len());
        assert_eq!(var["until"], "2026-11-04T00:00:00Z");
    }

    #[test]
    fn kun_faerdige_doegn_der_mangler() {
        let dag = dag_noegle(NU - DOEGN_MS);
        let ingen = HashMap::new();
        assert!(mangler(&dag, &ingen, NU), "i går er slut for mere end en time siden");
        assert!(!mangler(&dag_noegle(NU), &ingen, NU), "i dag aldrig");
        let lige_efter = dag_ms(&dag_noegle(NU)).unwrap() + 1_800_000; // 00:30: i går er ikke færdigt endnu
        assert!(!mangler(&dag, &ingen, lige_efter));
        let hentet = HashMap::from([(dag.clone(), "t".to_string())]);
        assert!(!mangler(&dag, &hentet, NU), "hentet: ikke igen");
        let v = vinduet(NU);
        assert_eq!((v.len(), v[0].as_str()), (30, dag.as_str()), "nyeste først, 30 døgn");
    }

    #[test]
    fn svaret_bliver_til_rensede_raekker() {
        let data = json!({ "viewer": { "zones": [{
            "post": [{ "count": 4, "dimensions": { "clientRequestPath": "/login?u=hemmelig", "edgeResponseStatus": 200, "originResponseStatus": 200, "clientCountryName": "PL", "clientAsn": "64500" } }],
            "status": [{ "count": 90, "dimensions": { "edgeResponseStatus": 404 } }],
            "lande": [{ "count": 80, "dimensions": { "clientCountryName": "" } }],
            "login": [{ "count": 3, "dimensions": { "clientRequestPath": "/wp-login.php", "clientRequestHTTPMethodName": "POST", "edgeResponseStatus": 403, "originResponseStatus": 0, "clientCountryName": "DE", "clientAsn": 64501 } }],
            "stoppet": [
                { "action": "block", "source": "firewallManaged", "userAgent": "GPTBot/1.2", "clientAsn": "64500", "clientASNDescription": "Eksempel Net" },
                { "action": "block", "source": "firewallManaged", "userAgent": "GPTBot/1.3", "clientAsn": "64500", "clientASNDescription": "Eksempel Net" },
                { "action": "link_maze_injected", "source": "linkMaze", "userAgent": "ClaudeBot/1.0", "clientAsn": "0", "clientASNDescription": "" },
                { "action": "link_maze_visited", "source": "linkMaze", "userAgent": "Mozilla/5.0", "clientAsn": null, "clientASNDescription": null },
            ],
        }] } });
        let rk = raekker(&fra_svar(&data).unwrap());
        let n = |art: &str, k: &str| rk.iter().find(|r| r.0 == art && r.1 == k).map(|r| r.2);
        assert_eq!(n("post", "/login\t200\t200\tPL\t64500"), Some(4), "{rk:?}");
        assert_eq!(n("status", "404"), Some(90));
        assert_eq!(n("land", "XX"), Some(80), "tomt land er ukendt");
        assert_eq!(n("login", "/wp-login.php\tPOST\t403\t0\tDE\t64501"), Some(3), "tal som ASN");
        assert_eq!(n("stoppet", "GPTBot\tOpenAI\tfirewallManaged\tblock"), Some(2), "tælles pr. robot");
        assert_eq!(n("stoppet", "Ukendt bot\t\tlinkMaze\tlink_maze_visited"), Some(1));
        assert!(
            !rk.iter().any(|r| r.1.contains("ClaudeBot")),
            "en side med fælde-links er ikke stoppet"
        );
        assert_eq!(n("asn", "64500\tEksempel Net"), Some(2));
        assert!(
            !rk.iter().any(|r| r.1.contains("Mozilla") || r.1.contains("hemmelig")),
            "ingen brugeragent, ingen query"
        );
        assert!(
            rk.iter().all(|r| EKSTRA_ARTER.contains(&r.0.as_str())),
            "alle arter er ekstra-arter"
        );
    }

    #[test]
    fn manglende_noder_er_ikke_en_fejl() {
        assert!(raekker(&fra_svar(&json!({ "viewer": { "zones": [{}] } })).unwrap()).is_empty());
        assert_eq!(fra_svar(&json!({ "viewer": { "zones": [] } })).err(), Some(Fejl::ZoneUkendt));
    }
}
