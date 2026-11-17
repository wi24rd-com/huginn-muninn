//! V-SUBDOM.1: trafik og angreb pr. subdomæne. Værten er Cloudflares `clientRequestHTTPHost` på `httpRequestsAdaptiveGroups` og `firewallEventsAdaptive`
//! (målt på Free i fase A: summen over værter er zonens total). Værtsnavnet er delvis angriberstyret (klienten skriver `Host`), så det renses (`ren_vaert`),
//! før det gemmes: kun labelen foran zonen (`""` for domænet selv, `www`, `shop`, `a.b`), og alt andet som en kategori i skarpe parenteser (`[ip]`, `[anden]`,
//! `[ugyldig]`). Højst `TOP_VAERTER` værter pr. domæne og døgn; resten lægges sammen som `[andre]`. Arterne i `vagt_dag`:
//! - `v_alle` (vært → forespørgsler fra alle kilder) og `v_besoeg` (vært → besøg, eyeball): `VAERT_QUERY`, ét kald pr. domæne i `koe::ekstra_runde` for de
//!   færdige døgn, der mangler i `hentet_vaert`.
//! - `v_fw` ("vært\tkilde\thandling" → hændelser, summen af `sampleInterval`): fra hovedhentningens `fw` (`vaern::opdater`), og for døgn hentet før V-SUBDOM.1
//!   fra `VAERT_FW_QUERY` (døgn, der mangler i `hentet_vaert_fw`). Angreb og afviste regnes som i Overblik (`RUTINE`, `er_afvist`).
//! - `v_afskaaret` og `v_fw_afskaaret` (nøgle "", n = loftet): Cloudflare sendte 10.000 rækker for døgnet, så der er ingen tal pr. vært for det (hellere intet
//!   tal end et for lavt).
//! - `v_land` ("vært\tland" → besøg, eyeball) og `v_robot` ("vært\tkategori" → besøg fra en kendt robot, eyeball) (V-SUBDOM.1 FIX 1): to aliaser mere i
//!   `VAERT_QUERY`, så det er samme kald. Står et af dem på loftet, gemmes ingen tal for den art for døgnene, kun mærket `v_land_afskaaret`/`v_robot_afskaaret`.
//!   Døgn hentet før FIX 1 har ingen (`store::VAERT_LAND_V1`: fra den dag har døgnene i `hentet_vaert` dem).
//! - `v_fejl` ("vært\tkant" → svar med kant ≥ 500, eyeball) (V-DRIFT.1): aliaset `vf` i `VAERT_QUERY`, samme kald og samme top 20 som `v_alle`. På loftet kun
//!   mærket `v_fejl_afskaaret`; afviser Cloudflare aliaset, hentes resten uden det, og døgnene får mærket `v_fejl_afvist`. Døgn hentet før V-DRIFT.1 har ingen
//!   (`store::VAERT_5XX_V1`).
//!
//! Ingen af arterne læses af Overblik, Angreb eller notifikationerne (`Store::raekker` udelader dem), og et værtsnavn står aldrig i en `Fejl`. Visningens tal
//! regnes her (`laes`, `regn`), kun fra cachen.
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::besoeg::AAR_DAGE;
use crate::cf::Fejl;
use crate::dyk::LOFT;
use crate::overblik::{er_afvist, RUTINE};
use crate::robotter::dage_i;
use crate::store::{Raekke, Store, SIDE_AFSKAARET, VAERT_FEJL, VAERT_FW_FEJL};
use crate::vaern::{dag_ms, dag_noegle, doegn_fra_svar, parse_iso, tom_ved_null, Fw, DOEGN_MS};

/// En IP-adresse som vært. Adressen gemmes aldrig.
pub const IP: &str = "[ip]";
/// En vært uden for zonen (fx en anden zone på samme IP, eller en forfalsket `Host`).
pub const ANDEN: &str = "[anden]";
/// Ulovlige tegn eller længder, eller tom.
pub const UGYLDIG: &str = "[ugyldig]";
/// Værterne efter de `TOP_VAERTER` største i døgnet, lagt sammen.
pub const ANDRE: &str = "[andre]";
/// Så mange værter gemmes pr. domæne og døgn (efter forespørgsler; for `v_fw` efter hændelser). Wildcard-DNS med tilfældige navne giver aldrig flere rækker.
pub const TOP_VAERTER: usize = 20;

pub const V_ALLE: &str = "v_alle";
pub const V_BESOEG: &str = "v_besoeg";
pub const V_AFSKAARET: &str = "v_afskaaret";
pub const V_FW: &str = "v_fw";
pub const V_FW_AFSKAARET: &str = "v_fw_afskaaret";
/// Besøg pr. vært og land, og pr. vært og robotkategori (V-SUBDOM.1 FIX 1), med deres mærker for loftet.
pub const V_LAND: &str = "v_land";
pub const V_ROBOT: &str = "v_robot";
pub const V_LAND_AFSKAARET: &str = "v_land_afskaaret";
pub const V_ROBOT_AFSKAARET: &str = "v_robot_afskaaret";
/// Fejl pr. vært (V-DRIFT.1): svar med kant ≥ 500, og mærkerne for loftet og for et afslag på aliaset (begge: ingen tal for døgnet).
pub const V_FEJL: &str = "v_fejl";
pub const V_FEJL_AFSKAARET: &str = "v_fejl_afskaaret";
pub const V_FEJL_AFVIST: &str = "v_fejl_afvist";
/// Arterne fra `VAERT_QUERY`: dem, `Store::gem_vaert` erstatter.
pub const ARTER: &[&str] = &[
    V_ALLE,
    V_BESOEG,
    V_AFSKAARET,
    V_LAND,
    V_ROBOT,
    V_LAND_AFSKAARET,
    V_ROBOT_AFSKAARET,
    V_FEJL,
    V_FEJL_AFSKAARET,
    V_FEJL_AFVIST,
];
/// Arterne til fejl pr. vært i `drift` (V-DRIFT.1).
pub const FEJL_ARTER: &[&str] = &[V_AFSKAARET, V_FEJL, V_FEJL_AFSKAARET, V_FEJL_AFVIST];
/// Arterne, tabellen "Subdomæner" regner med (`laes`).
pub const TABEL_ARTER: &[&str] = &[V_ALLE, V_BESOEG, V_AFSKAARET, V_FW, V_FW_AFSKAARET];
/// Arterne til lande og robotter pr. vært i Besøgende (`laes_lr`).
pub const LR_ARTER: &[&str] = &[V_AFSKAARET, V_LAND, V_ROBOT, V_LAND_AFSKAARET, V_ROBOT_AFSKAARET];
/// Arterne for angreb pr. vært: dem, `Store::gem_vaert_fw` erstatter, og som står i `store::DOEGN_ARTER`.
pub const FW_ARTER: &[&str] = &[V_FW, V_FW_AFSKAARET];
/// Alle arterne pr. vært (`Store::raekker` udelader dem).
pub const ALLE_ARTER: &[&str] = &[
    V_ALLE,
    V_BESOEG,
    V_AFSKAARET,
    V_FW,
    V_FW_AFSKAARET,
    V_LAND,
    V_ROBOT,
    V_LAND_AFSKAARET,
    V_ROBOT_AFSKAARET,
    V_FEJL,
    V_FEJL_AFSKAARET,
    V_FEJL_AFVIST,
];

/// Trafik pr. dag og vært: `v` = besøg udefra (eyeball; `count` og `sum { visits }`), `va` = forespørgsler fra alle kilder (`count`, samme tal som art `alle`).
/// V-SUBDOM.1 FIX 1: `vl` = besøg pr. vært og land og `vr` = besøg pr. vært og robotkategori (eyeball, som `besoeg::BESOEG_QUERY` og `besoeg::ROBOT_QUERY`
/// med værten; målt i fase A, INVESTIGATION Q3). Samme kald: ingen ekstra kald pr. dag.
/// V-DRIFT.1: `vf` = svar med kant ≥ 500 pr. dag, vært og kode (eyeball). Samme kald.
/// Kun zone og tider er variabler (`besoeg::variabler`). Ingen IP-adresser, stier eller brugeragenter.
pub const VAERT_QUERY: &str = r#"query Vaert($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    v: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball"}, orderBy:[count_DESC]) { count sum { visits } dimensions { date clientRequestHTTPHost } }
    va: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until}, orderBy:[count_DESC]) { count dimensions { date clientRequestHTTPHost } }
    vl: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball"}) { sum { visits } dimensions { date clientRequestHTTPHost clientCountryName } }
    vr: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball"}) { count sum { visits } dimensions { date clientRequestHTTPHost verifiedBotCategory } }
    vf: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball", edgeResponseStatus_geq:500}, orderBy:[count_DESC]) { count dimensions { date clientRequestHTTPHost edgeResponseStatus } }
  } } }"#;

/// `VAERT_QUERY` uden `vf`: tegn for tegn forespørgslen fra før V-DRIFT.1.
pub fn uden_vf(q: &str) -> String {
    q.lines()
        .filter(|l| !l.trim_start().starts_with("vf: "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Henter med `VAERT_QUERY`. `uden`: domænet har afvist `vf` tidligere i kørslen. Ellers: fejler kaldet med en GraphQL-fejl (`vaertsfelt_maaske_afvist`),
/// prøves det én gang uden `vf`. Lykkes det, er det andet tal `true` (den, der kalder, husker domænet), og svaret har intet `vf` (`trafik_raekker` giver så
/// mærket `v_fejl_afvist`). Fejler også det, er svaret den første fejl. Et afslag på `vf` koster altså aldrig de andre tal pr. vært. Ren bortset fra `graphql`.
pub async fn hent_med_vf<G, Fut>(uden: bool, graphql: G) -> (Result<Value, Fejl>, bool)
where
    G: Fn(String) -> Fut,
    Fut: Future<Output = Result<Value, Fejl>>,
{
    if uden {
        return (graphql(uden_vf(VAERT_QUERY)).await, false);
    }
    match graphql(VAERT_QUERY.to_string()).await {
        Err(e) if vaertsfelt_maaske_afvist(&e) => match graphql(uden_vf(VAERT_QUERY)).await {
            Ok(v) => (Ok(v), true),
            Err(_) => (Err(e), false),
        },
        svar => (svar, false),
    }
}

/// Kun værten, vægten, kilden og handlingen for hændelserne: til døgn, der blev hentet uden værten (før V-SUBDOM.1). Skriver kun art `v_fw`, aldrig `fw`.
pub const VAERT_FW_QUERY: &str = r#"query VaertFw($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    fw: firewallEventsAdaptive(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until}, orderBy:[datetime_DESC]) { clientRequestHTTPHost sampleInterval source action datetime }
  } } }"#;

/// (dag, art, nøgle, n), som `Store::gem_vaert` og `Store::gem_vaert_fw` gemmer dem.
pub type Raekke4 = (String, String, String, i64);

// ── Sikringen af hovedhentningen (B2, trin 0) ──────────────────────────────────────────────────────────────────────
/// Hovedforespørgslen `q` uden værtsfeltet på `fw`: som før V-SUBDOM.1. Feltet står sidst på `fw` (`vaern::DOEGN_QUERY`).
pub fn uden_vaertsfelt(q: &str) -> String {
    q.replacen(" clientRequestHTTPHost }", " }", 1)
}

/// Skal hovedforespørgslen prøves igen uden værtsfeltet? Et ukendt eller afvist felt er en valideringsfejl for hele forespørgslen (`Fejl::GraphQl`), og
/// Cloudflares tekst nævner ikke altid feltet, så alle GraphQL-fejl tæller, undtagen et døgn uden for Cloudflares opbevaring (det svar er det samme uden feltet).
pub fn vaertsfelt_maaske_afvist(e: &Fejl) -> bool {
    matches!(e, Fejl::GraphQl { .. }) && !e.uden_for_opbevaring()
}

/// Gælder noteringen "domænet afviste værtsfeltet" (noteret `noteret`, ms) stadig ved `nu`? Kun til næste UTC-døgn og højst 24 timer (V-SUBDOM.1 FIX 1, V1):
/// en forbigående fejl må ikke koste tallene pr. vært, så længe programmet kører i menulinjen. Ren funktion.
pub fn uden_gaelder(noteret: i64, nu: i64) -> bool {
    nu >= noteret && nu - noteret < DOEGN_MS && dag_noegle(noteret) == dag_noegle(nu)
}

/// Henter ét døgn med hovedforespørgslen `q`. `uden`: domænet har afvist værtsfeltet i dette UTC-døgn (`uden_gaelder`), så der spørges uden det. Ellers: fejler
/// kaldet med en GraphQL-fejl (`vaertsfelt_maaske_afvist`), prøves det én gang uden feltet. Lykkes det, er det andet tal `true`, og den, der kalder, noterer
/// domænet. Fejler også det, er svaret den første fejl, som før sikringen. Ren bortset fra `graphql`.
pub async fn hent_hoved<G, Fut>(q: &str, uden: bool, graphql: G) -> (Result<Value, Fejl>, bool)
where
    G: Fn(String) -> Fut,
    Fut: Future<Output = Result<Value, Fejl>>,
{
    if uden {
        return (graphql(uden_vaertsfelt(q)).await, false);
    }
    match graphql(q.to_string()).await {
        Err(e) if vaertsfelt_maaske_afvist(&e) => match graphql(uden_vaertsfelt(q)).await {
            Ok(v) => (Ok(v), true),
            Err(_) => (Err(e), false),
        },
        svar => (svar, false),
    }
}

// ── Rensning ───────────────────────────────────────────────────────────────────────────────────────────────────────
fn er_port(p: &str) -> bool {
    (1..=5).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit())
}

/// Kun `a-z 0-9 - _` i labels af 1–63 tegn, højst 253 i alt. Punycode (`xn--…`) er det samme.
pub(crate) fn gyldigt_navn(v: &str) -> bool {
    v.len() <= 253
        && v.split('.').all(|l| {
            (1..=63).contains(&l.len())
                && l.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        })
}

/// Værtens nøgle under zonen `zone` (zonens navn, fx `wi24rd.com`): små bogstaver, uden afsluttende punktum og `:port`. `""` er domænet selv, ellers labelen
/// foran zonen (`www`, `shop`, `a.b`). En IP som vært er `[ip]` (adressen gemmes aldrig), en vært uden for zonen `[anden]`, og ulovlige tegn, længder eller
/// en tom vært `[ugyldig]`. Punycode bliver punycode. Ren funktion.
pub fn ren_vaert(vaert: &str, zone: &str) -> String {
    let v = vaert.to_ascii_lowercase();
    if let Some(rest) = v.strip_prefix('[') {
        let ip = rest.split_once(']').is_some_and(|(adr, efter)| {
            adr.parse::<Ipv6Addr>().is_ok() && (efter.is_empty() || efter.strip_prefix(':').is_some_and(er_port))
        });
        return if ip { IP } else { UGYLDIG }.to_string();
    }
    if v.parse::<Ipv6Addr>().is_ok() {
        return IP.to_string();
    }
    let v = match v.rsplit_once(':') {
        Some((navn, port)) if er_port(port) => navn,
        Some(_) => return UGYLDIG.to_string(),
        None => v.as_str(),
    };
    let v = v.strip_suffix('.').unwrap_or(v);
    if v.is_empty() {
        return UGYLDIG.to_string();
    }
    if v.parse::<Ipv4Addr>().is_ok() {
        return IP.to_string();
    }
    let zone = zone.to_ascii_lowercase();
    let zone = zone.strip_suffix('.').unwrap_or(&zone);
    let label = if !zone.is_empty() && v == zone {
        ""
    } else {
        match v.strip_suffix(zone).and_then(|x| x.strip_suffix('.')).filter(|_| !zone.is_empty()) {
            Some(l) => l,
            None => return ANDEN.to_string(),
        }
    };
    if !gyldigt_navn(v) {
        return UGYLDIG.to_string();
    }
    label.to_string()
}

/// Vært → nøglen, den gemmes under: de `TOP_VAERTER` med størst `n` beholder deres egen, resten bliver `[andre]`. Uafgjort efter navn. Ren funktion.
fn top(tal: &HashMap<String, i64>) -> HashMap<String, String> {
    let mut v: Vec<(&String, &i64)> = tal.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    v.into_iter()
        .enumerate()
        .map(|(i, (k, _))| (k.clone(), if i < TOP_VAERTER { k.clone() } else { ANDRE.to_string() }))
        .collect()
}

// ── Trafik pr. vært ────────────────────────────────────────────────────────────────────────────────────────────────
#[derive(Deserialize, Default, Debug, Clone)]
struct VDim {
    #[serde(default, deserialize_with = "tom_ved_null")]
    date: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientRequestHTTPHost")]
    vaert: String,
    /// Kun i `vl` (tomt land er `XX`, som `besoeg::raekker`).
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientCountryName")]
    land: String,
    /// Kun i `vr` (tom = ikke en kendt robot; den gemmes ikke).
    #[serde(default, deserialize_with = "tom_ved_null", rename = "verifiedBotCategory")]
    kategori: String,
    /// Kun i `vf` (V-DRIFT.1).
    #[serde(default, deserialize_with = "tom_ved_null", rename = "edgeResponseStatus")]
    status: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct VSum {
    #[serde(default, deserialize_with = "tom_ved_null")]
    visits: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct VGruppe {
    #[serde(default, deserialize_with = "tom_ved_null")]
    count: i64,
    #[serde(default, deserialize_with = "tom_ved_null")]
    sum: VSum,
    #[serde(default, deserialize_with = "tom_ved_null")]
    dimensions: VDim,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct VSvar {
    #[serde(default, deserialize_with = "tom_ved_null")]
    v: Vec<VGruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    va: Vec<VGruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    vl: Vec<VGruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    vr: Vec<VGruppe>,
    /// V-DRIFT.1: `None`, når forespørgslen var uden `vf` (`hent_med_vf`); så mærker `med_vf_afvist` døgnene.
    #[serde(default)]
    vf: Option<Vec<VGruppe>>,
}

/// `VAERT_QUERY`-svaret som (dag, `v_alle`/`v_besoeg`, vært, n) for døgnene `dage` (kun n > 0), og om `v` eller `va` står på loftet (så er rækkerne ikke
/// til at stole på, og `hent_opdelt` spørger døgn for døgn). Værter, der bliver ens efter rensningen, lægges sammen; top 20 efter forespørgsler. Summen
/// over værterne er altid svarets sum. V-SUBDOM.1 FIX 1: `v_land` ("vært\tland") og `v_robot` ("vært\tkategori") med samme top 20 som `v_alle`. Står `vl`
/// eller `vr` på loftet (regel S4), får hvert døgn i `dage` kun mærket for den art, ingen tal; de andre arter er uberørte, og der spørges ikke igen. Ren funktion.
pub fn trafik_raekker(data: &Value, dage: &[String], zone: &str) -> Result<(Vec<Raekke4>, bool), Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    let svar = VSvar::deserialize(z).map_err(|_| Fejl::Svar)?;
    let afskaaret = svar.v.len() as i64 >= LOFT || svar.va.len() as i64 >= LOFT;
    let (land_loft, robot_loft) = (svar.vl.len() as i64 >= LOFT, svar.vr.len() as i64 >= LOFT);
    let dag_af = |g: &VGruppe| dage.iter().find(|d| **d == g.dimensions.date).map(String::as_str);
    // dag → vært → (forespørgsler, besøg)
    let mut tal: BTreeMap<&str, HashMap<String, (i64, i64)>> = BTreeMap::new();
    for (g, besoeg) in svar.va.iter().map(|g| (g, false)).chain(svar.v.iter().map(|g| (g, true))) {
        let Some(dag) = dag_af(g) else {
            continue;
        };
        let x = tal.entry(dag).or_default().entry(ren_vaert(&g.dimensions.vaert, zone)).or_default();
        if besoeg {
            x.1 += g.sum.visits;
        } else {
            x.0 += g.count;
        }
    }
    // dag → (vært, land eller kategori) → besøg. Værterne kommer med i top 20-opgørelsen (med 0, hvis `va` ikke har dem).
    let mut pr_land: BTreeMap<&str, HashMap<(String, String), i64>> = BTreeMap::new();
    let mut pr_robot: BTreeMap<&str, HashMap<(String, String), i64>> = BTreeMap::new();
    for (g, land) in svar.vl.iter().map(|g| (g, true)).chain(svar.vr.iter().map(|g| (g, false))) {
        let Some(dag) = dag_af(g) else {
            continue;
        };
        if (land && land_loft) || (!land && (robot_loft || g.dimensions.kategori.is_empty())) || g.sum.visits <= 0 {
            continue;
        }
        let vaert = ren_vaert(&g.dimensions.vaert, zone);
        tal.entry(dag).or_default().entry(vaert.clone()).or_default();
        let (m, k) = if land {
            let l = if g.dimensions.land.is_empty() {
                "XX"
            } else {
                g.dimensions.land.as_str()
            };
            (&mut pr_land, l.to_string())
        } else {
            (&mut pr_robot, g.dimensions.kategori.clone())
        };
        *m.entry(dag).or_default().entry((vaert, k)).or_default() += g.sum.visits;
    }
    // V-DRIFT.1: dag → (vært, kant) → svar med kant ≥ 500. Værterne lægges ikke i `tal`, så top 20 og de andre arter er præcis som før; en vært, der ikke er
    // blandt døgnets værter (kan kun ske, hvis `va` mangler den), tæller som `[andre]`.
    let fejl_loft = svar.vf.as_ref().is_some_and(|v| v.len() as i64 >= LOFT);
    let mut pr_fejl: BTreeMap<&str, BTreeMap<(String, i64), i64>> = BTreeMap::new();
    for g in svar.vf.iter().flatten().filter(|_| !fejl_loft) {
        if let Some(dag) = dag_af(g).filter(|_| g.count > 0) {
            *pr_fejl
                .entry(dag)
                .or_default()
                .entry((ren_vaert(&g.dimensions.vaert, zone), g.dimensions.status))
                .or_default() += g.count;
        }
    }
    let mut ud = Vec::new();
    for (dag, vaerter) in tal {
        let noegle = top(&vaerter.iter().map(|(k, x)| (k.clone(), x.0)).collect());
        let mut fejl: BTreeMap<String, i64> = BTreeMap::new();
        for ((v, kode), n) in pr_fejl.remove(dag).unwrap_or_default() {
            let k = noegle.get(&v).map_or(ANDRE, String::as_str);
            *fejl.entry(format!("{k}\t{kode}")).or_default() += n;
        }
        let mut samlet: BTreeMap<String, (i64, i64)> = BTreeMap::new();
        for (k, (a, b)) in vaerter {
            let x = samlet.entry(noegle[&k].clone()).or_default();
            x.0 += a;
            x.1 += b;
        }
        for (k, (a, b)) in samlet {
            if a > 0 {
                ud.push((dag.to_string(), V_ALLE.to_string(), k.clone(), a));
            }
            if b > 0 {
                ud.push((dag.to_string(), V_BESOEG.to_string(), k, b));
            }
        }
        for (art, m) in [(V_LAND, &mut pr_land), (V_ROBOT, &mut pr_robot)] {
            let mut samlet: BTreeMap<String, i64> = BTreeMap::new();
            for ((v, k), n) in m.remove(dag).unwrap_or_default() {
                *samlet.entry(format!("{}\t{k}", noegle[&v])).or_default() += n;
            }
            ud.extend(samlet.into_iter().map(|(k, n)| (dag.to_string(), art.to_string(), k, n)));
        }
        ud.extend(fejl.into_iter().map(|(k, n)| (dag.to_string(), V_FEJL.to_string(), k, n)));
    }
    // Fejl på et døgn uden andre tal pr. vært: alle som `[andre]`.
    for (dag, m) in pr_fejl {
        let mut fejl: BTreeMap<String, i64> = BTreeMap::new();
        for ((_, kode), n) in m {
            *fejl.entry(format!("{ANDRE}\t{kode}")).or_default() += n;
        }
        ud.extend(fejl.into_iter().map(|(k, n)| (dag.to_string(), V_FEJL.to_string(), k, n)));
    }
    for (loft, maerke) in [(land_loft, V_LAND_AFSKAARET), (robot_loft, V_ROBOT_AFSKAARET)] {
        if loft {
            ud.extend(dage.iter().map(|d| (d.clone(), maerke.to_string(), String::new(), LOFT)));
        }
    }
    // V-DRIFT.1: loftet (S4) giver kun mærket, aldrig 0. Et svar uden `vf` giver intet her; det mærker `med_vf_afvist`, når forespørgslen var uden det.
    if fejl_loft {
        ud.extend(dage.iter().map(|d| (d.clone(), V_FEJL_AFSKAARET.to_string(), String::new(), LOFT)));
    }
    Ok((ud, afskaaret))
}

/// V-DRIFT.1: blev mindst ét af kaldene for døgnene `dage` sendt uden `vf` (`uden`), får hvert døgn uden fejltal og uden loftmærke mærket `v_fejl_afvist`
/// (S4: et døgn uden `v_fejl` er ellers "hentet og nul"). Ren funktion.
pub fn med_vf_afvist(mut rk: Vec<Raekke4>, dage: &[String], uden: bool) -> Vec<Raekke4> {
    if uden {
        let med: HashSet<String> = rk
            .iter()
            .filter(|r| r.1 == V_FEJL || r.1 == V_FEJL_AFSKAARET)
            .map(|r| r.0.clone())
            .collect();
        rk.extend(
            dage.iter()
                .filter(|d| !med.contains(*d))
                .map(|d| (d.clone(), V_FEJL_AFVIST.to_string(), String::new(), 0)),
        );
    }
    rk
}

// ── Angreb pr. vært ────────────────────────────────────────────────────────────────────────────────────────────────
/// Hændelserne → art `v_fw` med nøglen "vært\tkilde\thandling" og n = summen af `Fw::antal` (som art `fw`). Top 20 værter efter hændelser. Ren funktion.
fn fw_samlet<'a>(fw: impl Iterator<Item = &'a Fw>, zone: &str) -> Vec<(String, String, i64)> {
    let mut pr: HashMap<String, HashMap<(String, String), i64>> = HashMap::new();
    for e in fw {
        let v = ren_vaert(e.vaert.as_deref().unwrap_or(""), zone);
        *pr.entry(v).or_default().entry((e.source.clone(), e.action.clone())).or_default() += e.antal();
    }
    let noegle = top(&pr.iter().map(|(k, m)| (k.clone(), m.values().sum())).collect());
    let mut samlet: BTreeMap<String, i64> = BTreeMap::new();
    for (v, m) in pr {
        for ((kilde, handling), n) in m {
            *samlet.entry(format!("{}\t{kilde}\t{handling}", noegle[&v])).or_default() += n;
        }
    }
    samlet.into_iter().map(|(k, n)| (V_FW.to_string(), k, n)).collect()
}

/// Ét døgns `fw` fra hovedhentningen → art `v_fw`. `None`, når svaret har hændelser, men ingen af dem har værten (W's forespørgsel, eller Cloudflare sendte
/// ikke feltet): så er der intet tal pr. vært for døgnet, og det noteres ikke som hentet. Summen er altid art `fw`'s sum. Ren funktion.
pub fn fw_raekker(fw: &[Fw], zone: &str) -> Option<Vec<(String, String, i64)>> {
    if !fw.is_empty() && fw.iter().all(|e| e.vaert.is_none()) {
        return None;
    }
    Some(fw_samlet(fw.iter(), zone))
}

/// Det, `vaern::opdater` gemmer pr. vært for et døgn. Under loftet: `nye`. På loftet (S4, hellere intet tal end et for lavt): de gemte `v_fw`, når de gemte
/// `fw`-rækker blev beholdt (`vaern::behold_fuldt_fw`) og har dem, for så kom de fra et fuldt svar; ellers kun mærket `v_fw_afskaaret`. Ren funktion.
pub fn fw_med_loft(
    nye: Vec<(String, String, i64)>,
    loft: bool,
    beholdt: bool,
    gemt_v_fw: &[(String, i64)],
    haendelser: usize,
) -> Vec<(String, String, i64)> {
    if !loft {
        return nye;
    }
    if beholdt && !gemt_v_fw.is_empty() {
        return gemt_v_fw.iter().map(|(k, n)| (V_FW.to_string(), k.clone(), *n)).collect();
    }
    vec![(V_FW_AFSKAARET.to_string(), String::new(), haendelser as i64)]
}

/// `VAERT_FW_QUERY`-svaret som (dag, `v_fw`, nøgle, n) for døgnene `dage`, delt på døgn efter `datetime`, og om svaret står på loftet. En hændelse uden
/// læsbart tidspunkt gør svaret ubrugeligt (`Fejl::Svar`): ellers ville et døgn tælle for lavt. Ren funktion.
pub fn fw_dag_raekker(data: &Value, dage: &[String], zone: &str) -> Result<(Vec<Raekke4>, bool), Fejl> {
    let z = doegn_fra_svar(data)?;
    let afskaaret = z.fw.len() as i64 >= LOFT;
    let mut pr_dag: BTreeMap<String, Vec<&Fw>> = BTreeMap::new();
    for e in &z.fw {
        let dag = parse_iso(&e.datetime).map(dag_noegle).ok_or(Fejl::Svar)?;
        if dage.contains(&dag) {
            pr_dag.entry(dag).or_default().push(e);
        }
    }
    let mut ud = Vec::new();
    for (dag, fw) in pr_dag {
        ud.extend(
            fw_samlet(fw.into_iter(), zone)
                .into_iter()
                .map(|(art, k, n)| (dag.clone(), art, k, n)),
        );
    }
    Ok((ud, afskaaret))
}

/// Henter `fra`…`til` i ét kald. Står svaret på loftet, hentes døgnene hver for sig, og et døgn, der stadig står på loftet, får kun mærket `maerke` (n =
/// loftet), aldrig et tal. Returnerer døgnene, svarene dækker, og rækkerne. Højst 1 + antal døgn kald. Ren bortset fra `hent`.
pub async fn hent_opdelt<F, Fut, R>(fra: &str, til: &str, hent: F, regn: R, maerke: &str) -> Result<(Vec<String>, Vec<Raekke4>), Fejl>
where
    F: Fn(String, String) -> Fut,
    Fut: Future<Output = Result<Value, Fejl>>,
    R: Fn(&Value, &[String]) -> Result<(Vec<Raekke4>, bool), Fejl>,
{
    let dage = dage_i(fra, til);
    let svar = hent(fra.to_string(), til.to_string()).await?;
    let (rk, afskaaret) = regn(&svar, &dage)?;
    if !afskaaret {
        return Ok((dage, rk));
    }
    let mut ud = Vec::new();
    for dag in &dage {
        let (rk, afskaaret) = if dage.len() == 1 {
            (Vec::new(), true)
        } else {
            let svar = hent(dag.clone(), dag.clone()).await?;
            regn(&svar, std::slice::from_ref(dag))?
        };
        if afskaaret {
            ud.push((dag.clone(), maerke.to_string(), String::new(), LOFT));
        } else {
            ud.extend(rk);
        }
    }
    Ok((dage, ud))
}

// ── Visningen (B2): kommandoen `subdomaener` ───────────────────────────────────────────────────────────────────────
/// Én dag for én vært. `None`: ingen tal for dagen (ikke hentet, eller Cloudflare sendte 10.000 rækker), og det er noget andet end 0. Et tal står, når døgnet er
/// hentet, eller når der ligger gemte tal for det, selv om køen skal hente det igen (V-SUBDOM.1 FIX 2, F3).
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct VaertDag {
    pub dag: String,
    pub foresp: Option<i64>,
    pub besoeg: Option<i64>,
    pub angreb: Option<i64>,
    pub afvist: Option<i64>,
}

/// Én vært i perioden: `vaert` er nøglen (`""` = domænet selv, `www`, `[andre]` …); visningen sætter zonens navn på. Summerne er over dagene med tal.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct VaertUd {
    pub vaert: String,
    pub foresp: i64,
    pub besoeg: i64,
    pub angreb: i64,
    pub afvist: i64,
    pub dage: Vec<VaertDag>,
}

/// Svaret til visningen. Ingen zone-id og intet konto-id.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SubdomaenerUd {
    pub fra: String,
    pub til: String,
    /// Værter med tal i perioden: rigtige navne først efter forespørgsler (så angreb, så navn), kategorierne (`[andre]`, `[ip]` …) sidst.
    pub vaerter: Vec<VaertUd>,
    /// Den første dag med trafik og med angreb pr. vært ("Pr. subdomæne fra …").
    pub trafik_fra: Option<String>,
    pub angreb_fra: Option<String>,
    /// Dage i perioden, hvor Cloudflare sendte 10.000 rækker: ingen tal pr. vært for dem.
    pub trafik_afskaaret: Vec<String>,
    pub angreb_afskaaret: Vec<String>,
    /// Mest besøgte sider: et svar i perioden stod på 10.000 rækker, så kun de mest besøgte er med.
    pub sider_afskaaret: bool,
    /// Cloudflare afviste forespørgslen pr. vært for domænet (trafik / angreb). Domænets egne tal er uberørte.
    pub trafik_afvist: bool,
    pub angreb_afvist: bool,
    /// V-SUBDOM.1 FIX 2 (F3): dage i perioden uden tal pr. vært, som køen stadig har til gode at hente (færdige døgn i hentevinduet `besoeg::hentevindue`,
    /// ikke afvist, ikke på loftet). Antallet er længden. Ældre dage er ikke med (dem dækker `trafik_fra`/`angreb_fra`).
    pub trafik_mangler: Vec<String>,
    pub angreb_mangler: Vec<String>,
    /// Antal dage i perioden med tal pr. vært (0: intet i perioden er hentet endnu, og visningen viser "–", ikke 0).
    pub trafik_dage: usize,
    pub angreb_dage: usize,
}

/// Det, `regn` skal bruge fra databasen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Laest {
    /// Arterne `ALLE_ARTER` fra og med periodens første dag.
    pub raekker: Vec<Raekke>,
    pub hentet: HashSet<String>,
    pub hentet_fw: HashSet<String>,
    /// Døgnene i det seneste afskårne sidesvar (`store::SIDE_AFSKAARET`).
    pub sider_afskaaret: Option<(String, String)>,
    pub trafik_afvist: bool,
    pub angreb_afvist: bool,
    /// Hovedhentningen afviste værtsfeltet for domænet i dag (`uden_gaelder`, sat af kommandoen). Det giver kun `angreb_afvist`, når perioden ingen `v_fw` har.
    pub uden_vaert: bool,
    /// Uret (ms, sat af kommandoen) til hentevinduet for `trafik_mangler`/`angreb_mangler`. 0: intet vindue, så intet mangler.
    pub nu: i64,
}

pub fn laes(lager: &Store, zone: &str, fra: &str) -> Result<Laest, Fejl> {
    Ok(Laest {
        uden_vaert: false,
        nu: 0,
        raekker: lager
            .raekker_art(&[zone.to_string()], TABEL_ARTER, fra)?
            .into_iter()
            .map(|(_, r)| r)
            .collect(),
        hentet: lager.hentet_vaert(zone)?.into_keys().collect(),
        hentet_fw: lager.hentet_vaert_fw(zone)?.into_keys().collect(),
        sider_afskaaret: lager
            .afsnit_fejl(SIDE_AFSKAARET, zone)?
            .and_then(|v| v.split_once(' ').map(|(a, b)| (a.to_string(), b.to_string()))),
        trafik_afvist: lager.afsnit_fejl(VAERT_FEJL, zone)?.is_some(),
        angreb_afvist: lager.afsnit_fejl(VAERT_FW_FEJL, zone)?.is_some(),
    })
}

/// `fra` og `til` er gyldige datoer, `fra` ≤ `til`, og perioden er højst et år.
pub fn tjek_periode(fra: &str, til: &str) -> Result<(), Fejl> {
    let (a, b) = (dag_ms(fra).ok_or(Fejl::UgyldigDato)?, dag_ms(til).ok_or(Fejl::UgyldigDato)?);
    if a > b || b - a >= AAR_DAGE * DOEGN_MS {
        return Err(Fejl::UgyldigDato);
    }
    Ok(())
}

/// Tallene pr. vært for `fra`…`til`. Angreb er hændelser uden rutinekilderne (`RUTINE`), afviste er hændelser med en afvisende handling (`er_afvist`),
/// som Overblik og Angreb regner dem. Ren funktion.
pub fn regn(l: &Laest, fra: &str, til: &str) -> SubdomaenerUd {
    let dage = dage_i(fra, til);
    let i_perioden: Vec<&Raekke> = l
        .raekker
        .iter()
        .filter(|r| r.dag.as_str() >= fra && r.dag.as_str() <= til)
        .collect();
    let afskaaret = |art: &str| -> BTreeSet<&str> { i_perioden.iter().filter(|r| r.art == art).map(|r| r.dag.as_str()).collect() };
    let (trafik_afsk, fw_afsk) = (afskaaret(V_AFSKAARET), afskaaret(V_FW_AFSKAARET));
    // vært → dag → [forespørgsler, besøg, angreb, afviste]
    let mut t: BTreeMap<&str, HashMap<&str, [i64; 4]>> = BTreeMap::new();
    for r in &i_perioden {
        let mut felt = r.noegle.split('\t');
        let vaert = felt.next().unwrap_or("");
        let tal = match r.art.as_str() {
            V_ALLE => [r.n, 0, 0, 0],
            V_BESOEG => [0, r.n, 0, 0],
            V_FW => {
                let (kilde, handling) = (felt.next().unwrap_or(""), felt.next().unwrap_or(""));
                let angreb = if RUTINE.contains(&kilde) { 0 } else { r.n };
                [0, 0, angreb, if er_afvist(handling) { r.n } else { 0 }]
            }
            _ => continue,
        };
        let x = t.entry(vaert).or_default().entry(r.dag.as_str()).or_default();
        for (i, n) in tal.into_iter().enumerate() {
            x[i] += n;
        }
    }
    // V-SUBDOM.1 FIX 2 (F3): gemte tal tæller, også når døgnet venter på en genhentning (ikke i `hentet_*`, fx efter `store::vaert_land_en_gang`); `hentet_*`
    // styrer kun, hvad køen henter. Et døgn uden gemte tal er hentet med 0, når det står i `hentet_*`. S4: et døgn med mærket for loftet tæller aldrig.
    let gemt = |arter: &[&str]| -> BTreeSet<&str> {
        i_perioden
            .iter()
            .filter(|r| arter.contains(&r.art.as_str()))
            .map(|r| r.dag.as_str())
            .collect()
    };
    let (trafik_gemt, fw_gemt) = (gemt(&[V_ALLE, V_BESOEG]), gemt(&[V_FW]));
    let trafik = |d: &str| (l.hentet.contains(d) || trafik_gemt.contains(d)) && !trafik_afsk.contains(d);
    let angreb = |d: &str| (l.hentet_fw.contains(d) || fw_gemt.contains(d)) && !fw_afsk.contains(d);
    let mut vaerter: Vec<VaertUd> = t
        .into_iter()
        .map(|(vaert, pr_dag)| {
            let dage: Vec<VaertDag> = dage
                .iter()
                .map(|d| {
                    let x = pr_dag.get(d.as_str()).copied().unwrap_or_default();
                    VaertDag {
                        dag: d.clone(),
                        foresp: trafik(d).then_some(x[0]),
                        besoeg: trafik(d).then_some(x[1]),
                        angreb: angreb(d).then_some(x[2]),
                        afvist: angreb(d).then_some(x[3]),
                    }
                })
                .collect();
            let sum = |f: fn(&VaertDag) -> Option<i64>| dage.iter().filter_map(f).sum();
            VaertUd {
                vaert: vaert.to_string(),
                foresp: sum(|d| d.foresp),
                besoeg: sum(|d| d.besoeg),
                angreb: sum(|d| d.angreb),
                afvist: sum(|d| d.afvist),
                dage,
            }
        })
        .filter(|v| v.foresp + v.besoeg + v.angreb + v.afvist > 0)
        .collect();
    vaerter.sort_by_key(|v| (v.vaert.starts_with('['), Reverse(v.foresp), Reverse(v.angreb), v.vaert.clone()));
    let i_perioden_dage = |m: &BTreeSet<&str>| m.iter().map(|d| d.to_string()).collect::<Vec<_>>();
    // V-SUBDOM.1 FIX 1 (V1): noteringen fra hovedhentningen giver kun et afslag, når perioden ingen tal pr. vært har for angrebene; har den, vises de.
    let angreb_afvist = l.angreb_afvist || (l.uden_vaert && !i_perioden.iter().any(|r| r.art == V_FW));
    // Det, køen stadig henter: færdige døgn i hentevinduet og perioden uden tal, ikke på loftet, og ikke for et domæne, der har afvist forespørgslen.
    let vindue: BTreeSet<String> = if l.nu > 0 {
        crate::besoeg::hentevindue(l.nu).into_iter().collect()
    } else {
        BTreeSet::new()
    };
    let mangler = |afvist: bool, med: &dyn Fn(&str) -> bool, afsk: &BTreeSet<&str>| -> Vec<String> {
        if afvist {
            return Vec::new();
        }
        dage.iter()
            .filter(|d| vindue.contains(*d) && !med(d) && !afsk.contains(d.as_str()))
            .cloned()
            .collect()
    };
    let (trafik_mangler, angreb_mangler) = (
        mangler(l.trafik_afvist, &trafik, &trafik_afsk),
        mangler(angreb_afvist, &angreb, &fw_afsk),
    );
    let (trafik_dage, angreb_dage) = (dage.iter().filter(|d| trafik(d)).count(), dage.iter().filter(|d| angreb(d)).count());
    // Den første dag med tal pr. vært: hentet, eller med gemte tal (FIX 2).
    let foerste = |hentet: &HashSet<String>, arter: &[&str]| -> Option<String> {
        let gemt = l.raekker.iter().filter(|r| arter.contains(&r.art.as_str())).map(|r| r.dag.as_str());
        hentet.iter().map(String::as_str).chain(gemt).min().map(str::to_string)
    };
    SubdomaenerUd {
        fra: fra.to_string(),
        til: til.to_string(),
        vaerter,
        trafik_fra: foerste(&l.hentet, &[V_ALLE, V_BESOEG]),
        angreb_fra: foerste(&l.hentet_fw, &[V_FW]),
        trafik_afskaaret: i_perioden_dage(&trafik_afsk),
        angreb_afskaaret: i_perioden_dage(&fw_afsk),
        sider_afskaaret: l
            .sider_afskaaret
            .as_ref()
            .is_some_and(|(a, b)| a.as_str() <= til && b.as_str() >= fra),
        trafik_afvist: l.trafik_afvist,
        angreb_afvist,
        trafik_mangler,
        angreb_mangler,
        trafik_dage,
        angreb_dage,
    }
}

// ── Lande og robotter pr. vært i Besøgende (V-SUBDOM.1 FIX 1) ──────────────────────────────────────────────────────────
/// Én værts lande og robotter: rækkerne med nøglen uden værten (`land` eller `kategori`) som `b_land` og `b_robot`, og dagene med tal for hver.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct VaertLr {
    pub land: Vec<Raekke>,
    pub robot: Vec<Raekke>,
    pub land_hentet: HashSet<String>,
    pub robot_hentet: HashSet<String>,
}

/// Lande og robotter for værten `vaert` (labelen; `""` = domænet selv) ud fra rækkerne `rk` (`LR_ARTER`), de hentede døgn `hentet` (`hentet_vaert`) og den
/// første dag med lande og robotter pr. vært (`fra`, `store::VAERT_LAND_V1`; `None`: ingen). Et døgn har tal for en art, når det er hentet fra og med `fra`
/// eller har gemte tal for arten (V-SUBDOM.1 FIX 2: også når køen skal hente det igen), og hverken hele svaret (`v_afskaaret`) eller artens alias
/// (`v_land_afskaaret`/`v_robot_afskaaret`) stod på loftet. Ren funktion.
pub fn lr(rk: &[Raekke], hentet: &HashSet<String>, fra: Option<&str>, vaert: &str) -> VaertLr {
    let hentet_fra: BTreeSet<&str> = hentet.iter().map(String::as_str).filter(|d| fra.is_some_and(|f| *d >= f)).collect();
    let maerket = |art: &str| -> HashSet<&str> {
        rk.iter()
            .filter(|r| r.art == art || r.art == V_AFSKAARET)
            .map(|r| r.dag.as_str())
            .collect()
    };
    let (uden_land, uden_robot) = (maerket(V_LAND_AFSKAARET), maerket(V_ROBOT_AFSKAARET));
    let med = |art: &str, uden: &HashSet<&str>| {
        let gemt = rk.iter().filter(|r| r.art == art).map(|r| r.dag.as_str());
        hentet_fra
            .iter()
            .copied()
            .chain(gemt)
            .filter(|d| !uden.contains(d))
            .map(str::to_string)
            .collect::<HashSet<_>>()
    };
    let (land_hentet, robot_hentet) = (med(V_LAND, &uden_land), med(V_ROBOT, &uden_robot));
    let af = |art: &str, hentet: &HashSet<String>| -> Vec<Raekke> {
        rk.iter()
            .filter(|r| r.art == art && hentet.contains(&r.dag))
            .filter_map(|r| {
                let (v, k) = r.noegle.split_once('\t')?;
                (v == vaert).then(|| Raekke {
                    dag: r.dag.clone(),
                    art: art.to_string(),
                    noegle: k.to_string(),
                    n: r.n,
                })
            })
            .collect()
    };
    VaertLr {
        land: af(V_LAND, &land_hentet),
        robot: af(V_ROBOT, &robot_hentet),
        land_hentet,
        robot_hentet,
    }
}

pub fn laes_lr(lager: &Store, zone: &str, vaert: &str) -> Result<VaertLr, Fejl> {
    let rk: Vec<Raekke> = lager
        .raekker_art(&[zone.to_string()], LR_ARTER, "0000-00-00")?
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    let hentet: HashSet<String> = lager.hentet_vaert(zone)?.into_keys().collect();
    Ok(lr(&rk, &hentet, lager.vaert_land_fra()?.as_deref(), vaert))
}

#[cfg(test)]
mod tests;
