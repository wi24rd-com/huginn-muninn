//! Besøgende (V-BESOEG.1): unikke besøgende pr. dag et år tilbage og besøg pr. land. Unikke besøgende er årstallene (`historik`, art `h_total`, nøgle
//! `unikke`, Cloudflares `uniq { uniques }`: unikke IP-adresser, kun en total pr. dag). Besøg er `sum { visits }` på `httpRequestsAdaptiveGroups`
//! med `requestSource:"eyeball"` (Cloudflares egen definition: en sidevisning, der kom fra et andet site eller et direkte link), pr. dag og land.
//! Cloudflare har dem i ca. en måned; programmet gemmer dagene (arterne `b_land` og `b_total`, `hentet_besoeg`), så historikken vokser derfra.
//! Hentes i `koe::Koe::ekstra_runde` efter årstallene: ét kald pr. domæne for de færdige døgn, der mangler. Afviser Cloudflare forespørgslen, noteres det
//! ved domænet (`store::BESOEG_FEJL`, "ikke tilgængelig"), og der spørges ikke igen, før programmet starter igen. Ingen IP-adresser, stier eller
//! brugeragenter: kun dag, land og et tal. Visningens tal regnes her (`laes`, `regn`), kun fra cachen.
//! V-BESOEG.2: to kald mere for de samme døgn, hver med sin egen `hentet_*`-tabel og fejl: besøg pr. kendt robot (`ROBOT_QUERY`, `verifiedBotCategory`, art `b_robot`,
//! tom kategori er "ikke en kendt robot") og de mest besøgte sider (`SIDE_QUERY`, html-svar, arterne `b_side` = visninger og `b_side_b` = besøg, nøglen er
//! "<svarkode> <sti>", stien renset som i Angreb, top `TOP_SIDER` pr. døgn). Perioder over `UGE_GRAENSE` dage vises pr. uge (mandag–søndag, UTC).
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cf::Fejl;
use crate::dyk::{periode_ud, Periode, PeriodeUd};
use crate::historik::UNIKKE;
use crate::historik::{IPKLASSE, ROBOT_KLASSER};
use crate::robotter::{dage_i, robot_gruppe, GRUPPER};
use crate::store::{Raekke, Store, BESOEG_ARTER, BESOEG_FEJL, ROBOT_FEJL, SIDE_FEJL};
use crate::vaern::{dag_ms, dag_noegle, er_faerdig, iso, ren_sti, tom_ved_null, DOEGN_MS};

/// Så mange døgn tilbage hentes besøgene. `httpRequestsAdaptiveGroups` har `notOlderThan` 31 dage og `maxDuration` 30 dage på Free og Pro (bilag A),
/// så døgnet for 31 dage siden er altid delvis for gammelt: d-30…d-1 er det, ét kald kan dække.
pub const BESOEG_DAGE: i64 = 30;
/// Så mange dage dækker "et år" i visningen, og så langt kan Fra/Til spænde.
pub const AAR_DAGE: i64 = 365;

/// Kun zone og tider er variabler. Ingen `orderBy`: 30 døgn gange alle lande er under grænsen, og svaret lægges sammen her.
pub const BESOEG_QUERY: &str = r#"query Besoeg($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    besoeg: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball"}) { sum { visits } dimensions { date clientCountryName } }
  } } }"#;

/// Kendte robotter: besøg og forespørgsler pr. dag og Cloudflares kategori for verificerede robotter (tom = ikke en kendt robot). Eget kald, så besøg pr. land
/// ikke rammer grænsen. `count` hentes med, men kun besøgene gemmes.
pub const ROBOT_QUERY: &str = r#"query Robot($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    robot: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball"}) { count sum { visits } dimensions { date verifiedBotCategory } }
  } } }"#;

/// Mest besøgte sider: html-svar pr. dag, sti, svarkode og vært, flest først. Kun stien (aldrig forespørgselsstrengen, som `clientRequestPath` ikke har).
/// `edgeResponseBytes` (V-BESOEG.2 FIX 1, som `sti200::hoved_query`) giver svarenes størrelse, så stier, der svarer med forsiden, kan skilles fra.
/// `clientRequestHTTPHost` (V-SUBDOM.1) skiller samme sti på to værter ad (`side_noegle_vaert`).
pub const SIDE_QUERY: &str = r#"query Side($zoneTag: string!, $since: Time!, $until: Time!) { viewer { zones(filter:{zoneTag:$zoneTag}) {
    side: httpRequestsAdaptiveGroups(limit:10000, filter:{datetime_geq:$since, datetime_lt:$until, requestSource:"eyeball", edgeResponseContentTypeName:"html"}, orderBy:[count_DESC]) { count sum { visits edgeResponseBytes } dimensions { date clientRequestPath edgeResponseStatus clientRequestHTTPHost } }
  } } }"#;

/// Art: bytes i alt pr. døgn, svarkode og sti (`SIDE_QUERY`).
pub const SIDE_BYTES: &str = "b_side_bytes";
/// Så mange sider (sti og svarkode) gemmes pr. døgn, flest visninger først.
pub const TOP_SIDER: usize = 50;
/// Perioder med flere dage end dette vises med én søjle pr. uge.
pub const UGE_GRAENSE: usize = 90;

/// De færdige døgn i hentevinduet, der mangler: (ældste, nyeste). Ét kald dækker alt derimellem. Ren funktion.
pub fn mangler(hentet: &HashMap<String, String>, nu: i64) -> Option<(String, String)> {
    let dage: Vec<String> = (1..=BESOEG_DAGE)
        .map(|i| dag_noegle(nu - i * DOEGN_MS))
        .filter(|d| er_faerdig(d, &iso(nu)) && !hentet.contains_key(d))
        .collect();
    Some((dage.last()?.clone(), dage.first()?.clone()))
}

/// Variablerne til døgnene `fra`…`til` (hele UTC-døgn).
pub fn variabler(zone: &str, fra: &str, til: &str) -> Result<Value, Fejl> {
    dag_ms(fra).ok_or(Fejl::UgyldigDato)?;
    let slut = dag_ms(til).ok_or(Fejl::UgyldigDato)? + DOEGN_MS;
    Ok(json!({ "zoneTag": zone, "since": format!("{fra}T00:00:00Z"), "until": format!("{}T00:00:00Z", dag_noegle(slut)) }))
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Sum {
    #[serde(default, deserialize_with = "tom_ved_null")]
    visits: i64,
    /// Kun i `SIDE_QUERY`.
    #[serde(default, deserialize_with = "tom_ved_null", rename = "edgeResponseBytes")]
    bytes: i64,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Dim {
    #[serde(default, deserialize_with = "tom_ved_null")]
    date: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientCountryName")]
    land: String,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Gruppe {
    #[serde(default, deserialize_with = "tom_ved_null")]
    sum: Sum,
    #[serde(default, deserialize_with = "tom_ved_null")]
    dimensions: Dim,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Svar {
    #[serde(default, deserialize_with = "tom_ved_null")]
    besoeg: Vec<Gruppe>,
}

/// Svaret som (dag, art, nøgle, n) for døgnene `dage`: `b_land` pr. land med besøg (tomt land er `XX`, som resten af programmet) og `b_total`
/// (`besoeg`) for hvert døgn, også 0, når Cloudflare ingen besøg havde. Rækker for andre døgn end `dage` tæller ikke. Ren funktion.
pub fn raekker(data: &Value, dage: &[String]) -> Result<Vec<(String, String, String, i64)>, Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    let svar = Svar::deserialize(z).map_err(|_| Fejl::Svar)?;
    let mut lande: BTreeMap<(&str, String), i64> = BTreeMap::new();
    for g in &svar.besoeg {
        let Some(dag) = dage.iter().find(|d| **d == g.dimensions.date) else {
            continue;
        };
        let land = if g.dimensions.land.is_empty() {
            "XX"
        } else {
            g.dimensions.land.as_str()
        };
        *lande.entry((dag.as_str(), land.to_string())).or_default() += g.sum.visits;
    }
    let mut ud = Vec::new();
    for dag in dage {
        let mut i_alt = 0;
        for ((_, land), n) in lande
            .range((dag.as_str(), String::new())..)
            .take_while(|((d, _), _)| *d == dag.as_str())
        {
            if *n > 0 {
                ud.push((dag.clone(), "b_land".to_string(), land.clone(), *n));
                i_alt += n;
            }
        }
        ud.push((dag.clone(), "b_total".to_string(), "besoeg".to_string(), i_alt));
    }
    Ok(ud)
}

#[derive(Deserialize, Default, Debug, Clone)]
struct RobotDim {
    #[serde(default, deserialize_with = "tom_ved_null")]
    date: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "verifiedBotCategory")]
    kategori: String,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct RobotGruppe {
    #[serde(default, deserialize_with = "tom_ved_null")]
    sum: Sum,
    #[serde(default, deserialize_with = "tom_ved_null")]
    dimensions: RobotDim,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct SideDim {
    #[serde(default, deserialize_with = "tom_ved_null")]
    date: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "clientRequestPath")]
    sti: String,
    #[serde(default, deserialize_with = "tom_ved_null", rename = "edgeResponseStatus")]
    status: i64,
    /// Rå `clientRequestHTTPHost`; renses af `vaert::ren_vaert`. `None`: feltet var ikke med (svar fra før V-SUBDOM.1 i testene) = domænet selv.
    #[serde(default, rename = "clientRequestHTTPHost")]
    vaert: Option<String>,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct SideGruppe {
    #[serde(default, deserialize_with = "tom_ved_null")]
    count: i64,
    #[serde(default, deserialize_with = "tom_ved_null")]
    sum: Sum,
    #[serde(default, deserialize_with = "tom_ved_null")]
    dimensions: SideDim,
}

#[derive(Deserialize, Default, Debug, Clone)]
struct Svar2 {
    #[serde(default, deserialize_with = "tom_ved_null")]
    robot: Vec<RobotGruppe>,
    #[serde(default, deserialize_with = "tom_ved_null")]
    side: Vec<SideGruppe>,
}

fn svar2(data: &Value) -> Result<Svar2, Fejl> {
    let z = data.pointer("/viewer/zones/0").ok_or(Fejl::ZoneUkendt)?;
    Svar2::deserialize(z).map_err(|_| Fejl::Svar)
}

/// Robotsvaret som (dag, `b_robot`, kategori, besøg) for døgnene `dage`: kun > 0, og den tomme kategori (ikke en kendt robot) med. Ren funktion.
pub fn robot_raekker(data: &Value, dage: &[String]) -> Result<Vec<(String, String, String, i64)>, Fejl> {
    let mut tal: BTreeMap<(String, String), i64> = BTreeMap::new();
    for g in svar2(data)?.robot.into_iter().filter(|g| dage.contains(&g.dimensions.date)) {
        *tal.entry((g.dimensions.date, g.dimensions.kategori)).or_default() += g.sum.visits;
    }
    Ok(tal
        .into_iter()
        .filter(|x| x.1 > 0)
        .map(|((dag, kat), n)| (dag, "b_robot".to_string(), kat, n))
        .collect())
}

/// Nøglen for en side: svarkoden og den rensede sti.
pub fn side_noegle(status: i64, sti: &str) -> String {
    format!("{status} {sti}")
}

/// Nøglen for en side på en vært (V-SUBDOM.1): som `side_noegle` for domænet selv (`vaert` = ""), ellers "<svarkode> <sti>\t<vært>". Tabulatoren kan ikke
/// stå i en renset sti (`ren_sti` fjerner styretegn), så nøglen kan ikke misforstås. Et domæne uden subdomæner får præcis de samme nøgler som før.
pub fn side_noegle_vaert(status: i64, sti: &str, vaert: &str) -> String {
    if vaert.is_empty() {
        side_noegle(status, sti)
    } else {
        format!("{status} {sti}\t{vaert}")
    }
}

/// En sidenøgle → (svarkode, sti, vært). Nøgler uden vært (domænet selv og døgn hentet før V-SUBDOM.1) har værten "". Ren funktion.
pub fn side_dele(noegle: &str) -> Option<(&str, &str, &str)> {
    let (kode, rest) = noegle.split_once(' ')?;
    let (sti, vaert) = rest.split_once('\t').unwrap_or((rest, ""));
    Some((kode, sti, vaert))
}

/// Står sidesvaret på loftet (10.000 rækker)? Så mangler de mindst besøgte sider (`orderBy:[count_DESC]`). Ren funktion.
pub fn side_afskaaret(data: &Value) -> Result<bool, Fejl> {
    Ok(svar2(data)?.side.len() as i64 >= crate::dyk::LOFT)
}

/// Sidesvaret som (dag, art, nøgle, n) for døgnene `dage`: `b_side` = visninger, `b_side_b` = besøg og `b_side_bytes` = bytes i alt (altid,
/// også 0, så et døgn med bytes kan kendes fra et døgn hentet før de kom med). Nøglen er `side_noegle_vaert` med værten renset under zonen `zone`
/// (`vaert::ren_vaert`). Stien renses som i Angreb
/// (`vaern::ren_sti`: uden forespørgselsstreng, token-agtige stykker skjult, højst 90 tegn), stier, der bliver ens, lægges sammen, og kun de `TOP_SIDER`
/// med flest visninger pr. døgn gemmes. Ren funktion.
pub fn side_raekker(data: &Value, dage: &[String], zone: &str) -> Result<Vec<(String, String, String, i64)>, Fejl> {
    let mut tal: BTreeMap<String, HashMap<String, (i64, i64, i64)>> = BTreeMap::new();
    for g in svar2(data)?.side.into_iter().filter(|g| dage.contains(&g.dimensions.date)) {
        let vaert = g
            .dimensions
            .vaert
            .as_deref()
            .map_or_else(String::new, |v| crate::vaert::ren_vaert(v, zone));
        let noegle = side_noegle_vaert(g.dimensions.status, &ren_sti(&g.dimensions.sti), &vaert);
        let x = tal.entry(g.dimensions.date).or_default().entry(noegle).or_default();
        x.0 += g.count;
        x.1 += g.sum.visits;
        x.2 += g.sum.bytes;
    }
    let mut ud = Vec::new();
    for (dag, sider) in tal {
        let mut sider: Vec<(String, (i64, i64, i64))> = sider.into_iter().filter(|x| x.1 .0 > 0).collect();
        sider.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then_with(|| a.0.cmp(&b.0)));
        for (noegle, (visninger, besoeg, bytes)) in sider.into_iter().take(TOP_SIDER) {
            ud.push((dag.clone(), "b_side".to_string(), noegle.clone(), visninger));
            ud.push((dag.clone(), SIDE_BYTES.to_string(), noegle.clone(), bytes.max(0)));
            if besoeg > 0 {
                ud.push((dag.clone(), "b_side_b".to_string(), noegle, besoeg));
            }
        }
    }
    Ok(ud)
}

// ── Visningen ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// Det, `regn` skal bruge fra databasen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Laest {
    /// Art `h_total` med nøglen `unikke` for omfanget.
    pub unikke: Vec<Raekke>,
    /// Arterne `b_total`, `b_land`, `b_robot`, `b_side` og `b_side_b` for omfanget.
    pub besoeg: Vec<Raekke>,
    /// Art `h_ipklasse` for omfanget.
    pub ipklasse: Vec<Raekke>,
    /// Dage, hvor mindst ét domæne i omfanget har fået besøgene hentet.
    pub besoeg_hentet: HashSet<String>,
    /// Dage med kendte robotter og med sider hentet.
    pub robot_hentet: HashSet<String>,
    pub side_hentet: HashSet<String>,
    /// Arterne `b_side`, `b_side_b` og `b_side_bytes` med domænet: forsiden er hvert domænes egen (`forside_deling`).
    pub sider_z: Vec<(String, Raekke)>,
    /// Domæner i omfanget, hvor Cloudflare ikke svarede på besøgene, robotterne og siderne.
    pub ikke_tilgaengelig: Vec<String>,
    pub robot_ikke: Vec<String>,
    pub side_ikke: Vec<String>,
}

pub fn laes(lager: &Store, zoner: &[String]) -> Result<Laest, Fejl> {
    let navne: HashMap<String, String> = lager.zoner()?.0.into_iter().map(|z| (z.id, z.navn)).collect();
    let mut l = Laest::default();
    for z in zoner {
        l.besoeg_hentet.extend(lager.hentet_besoeg(z)?.into_keys());
        l.robot_hentet.extend(lager.hentet_robot(z)?.into_keys());
        l.side_hentet.extend(lager.hentet_side(z)?.into_keys());
        let navn = || navne.get(z).cloned().unwrap_or_else(|| z.clone());
        for (praefiks, liste) in [
            (BESOEG_FEJL, &mut l.ikke_tilgaengelig),
            (ROBOT_FEJL, &mut l.robot_ikke),
            (SIDE_FEJL, &mut l.side_ikke),
        ] {
            if lager.afsnit_fejl(praefiks, z)?.is_some() {
                liste.push(navn());
            }
        }
    }
    for liste in [&mut l.ikke_tilgaengelig, &mut l.robot_ikke, &mut l.side_ikke] {
        liste.sort();
    }
    let raekker = |arter: &[&str]| -> Result<Vec<Raekke>, Fejl> {
        Ok(lager.raekker_art(zoner, arter, "0000-00-00")?.into_iter().map(|(_, r)| r).collect())
    };
    l.unikke = raekker(&["h_total"])?.into_iter().filter(|r| r.noegle == UNIKKE).collect();
    l.besoeg = raekker(BESOEG_ARTER)?;
    l.sider_z = lager.raekker_art(zoner, &["b_side", "b_side_b", SIDE_BYTES], "0000-00-00")?;
    l.ipklasse = raekker(&[IPKLASSE])?;
    Ok(l)
}

/// En flise: summen af dagstallene i dagene, hvor mange dage der har tal (`med`), og hvor mange dage den dækker.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Sum7 {
    pub n: i64,
    pub med: i64,
    pub dage: i64,
}

/// Én dag i grafen. `n: None`: ingen tal for dagen (ikke hentet), og det er noget andet end 0.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DagUd {
    pub dag: String,
    pub n: Option<i64>,
}

/// Én uge i grafen (perioder over `UGE_GRAENSE` dage): mandag–søndag i UTC, skåret til perioden. `n`: summen af ugens dage med tal, `None`, hvis ingen har.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct UgeUd {
    /// Ugens mandag (også når perioden begynder senere).
    pub dag: String,
    pub fra: String,
    pub til: String,
    pub n: Option<i64>,
    /// Dage med tal, og dage af ugen i perioden.
    pub med: i64,
    pub dage: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct LandUd {
    pub kode: String,
    pub n: i64,
}

/// Besøg fra kendte robotter: `n` af `i_alt` besøg på de dage, der har både besøg og robotter (`med`).
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct RobotAndel {
    pub n: i64,
    pub i_alt: i64,
    pub med: i64,
}

/// Forespørgsler fra IP-klasser, der er kendte robotter og scannere (`ROBOT_KLASSER`), af alle, det seneste år, og hvor mange dage der har tal.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct KlasseAndel {
    pub n: i64,
    pub i_alt: i64,
    pub med: i64,
}

/// En robotgruppe (Robotter-visningens grupper ud fra Cloudflares kategori) og dens besøg i perioden.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct GruppeUd {
    pub id: String,
    pub n: i64,
}

/// En side i perioden: visninger og besøg udefra.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SideUd {
    pub sti: String,
    pub n: i64,
    pub besoeg: i64,
}

/// Filteret i adressen: højst ét ad gangen, for tallene kommer fra hvert sit kald og kan ikke deles på hinanden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter {
    Land(String),
    Sti(String),
    Robot(String),
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct BesoegUd {
    /// Perioden med dens huller: unikke besøgende, eller filtrets tal.
    pub periode: PeriodeUd,
    pub i_gaar_dag: String,
    /// Unikke besøgende i går; `None`, når dagen ikke har tal endnu.
    pub i_gaar: Option<i64>,
    pub d7: Sum7,
    pub d30: Sum7,
    pub aar: Sum7,
    /// Besøg de seneste 30 dage: alle, fra landet eller på stien, når der er et af dem som filter.
    pub besoeg30: Sum7,
    /// Heraf fra kendte robotter (kun uden filter, og kun når der er hentet robotter).
    pub robot30: Option<RobotAndel>,
    /// Grafen: unikke besøgende pr. dag, eller filtrets tal pr. dag.
    pub dage: Vec<DagUd>,
    /// Det samme pr. uge, når perioden er over `UGE_GRAENSE` dage.
    pub uger: Option<Vec<UgeUd>>,
    pub land: Option<String>,
    pub sti: Option<String>,
    pub robot: Option<String>,
    /// Besøg pr. land i perioden, flest først, og summen.
    pub lande: Vec<LandUd>,
    pub lande_i_alt: i64,
    /// Dage i perioden med besøg hentet, og den første dag med besøg.
    pub lande_med: i64,
    pub besoeg_fra: Option<String>,
    pub ikke_tilgaengelig: Vec<String>,
    /// Kendte robotter i perioden pr. gruppe, flest først; dage med tal, og den første dag med robotter.
    pub robotter: Vec<GruppeUd>,
    pub robot_med: i64,
    pub robot_fra: Option<String>,
    pub robot_ikke: Vec<String>,
    /// IP-klasserne det seneste år (årstallene).
    pub ipklasse: Option<KlasseAndel>,
    /// Mest besøgte sider i perioden (alle svarkoder undtagen 404), flest visninger først, og summen af visningerne; sider, der svarede 404.
    pub sider: Vec<SideUd>,
    pub sider_i_alt: i64,
    pub sider_404: Vec<SideUd>,
    /// Stier, der svarede med sidens almindelige side (`forside_deling`): taget ud af `sider`. Et skøn ud fra svarenes størrelse.
    pub sider_forside: Vec<SideUd>,
    /// Et domæne har sider med størrelse i perioden, men ikke forsiden: dets stier er ikke delt.
    pub sider_uden_forside: bool,
    pub side_med: i64,
    pub side_fra: Option<String>,
    pub side_ikke: Vec<String>,
}

/// Første og sidste dag i perioden. "Hele" er et år; et spænd er højst et år, regnet bagud fra `til`. Ren funktion.
pub fn spaend(periode: &Periode, nu: i64) -> (String, String) {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    match periode {
        Periode::D7 => (d(7), d(1)),
        Periode::D30 => (d(30), d(1)),
        Periode::Alt => (d(AAR_DAGE), d(1)),
        Periode::Dag(dag) => (dag.clone(), dag.clone()),
        Periode::Spaend { fra, til } => {
            let graense = dag_ms(til).map_or_else(String::new, |t| dag_noegle(t - (AAR_DAGE - 1) * DOEGN_MS));
            (fra.clone().max(graense), til.clone())
        }
    }
}

/// Et land i adressen: to tegn, store bogstaver eller tal (Cloudflares koder, også `T1` og `XX`).
pub fn tjek_land(land: Option<String>) -> Result<Option<String>, Fejl> {
    match land {
        None => Ok(None),
        Some(l) if l.len() == 2 && l.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) => Ok(Some(l)),
        Some(_) => Err(Fejl::UgyldigtFilter),
    }
}

/// Filteret fra adressen: land, sti (en renset sti, som visningen selv har vist: `/`, højst 90 tegn, intet `?`) eller robotgruppe (`robotter::GRUPPER`).
/// Højst ét. Ren funktion.
pub fn tjek_filter(land: Option<String>, sti: Option<String>, robot: Option<String>) -> Result<Option<Filter>, Fejl> {
    let land = tjek_land(land)?;
    let sti = match sti {
        None => None,
        Some(x) if x.starts_with('/') && x.chars().count() <= 90 && !x.contains('?') && !x.chars().any(char::is_control) => Some(x),
        Some(_) => return Err(Fejl::UgyldigtFilter),
    };
    let robot = match robot {
        None => None,
        Some(x) if GRUPPER.contains(&x.as_str()) => Some(x),
        Some(_) => return Err(Fejl::UgyldigtFilter),
    };
    match (land, sti, robot) {
        (None, None, None) => Ok(None),
        (Some(l), None, None) => Ok(Some(Filter::Land(l))),
        (None, Some(s), None) => Ok(Some(Filter::Sti(s))),
        (None, None, Some(r)) => Ok(Some(Filter::Robot(r))),
        _ => Err(Fejl::UgyldigtFilter),
    }
}

/// Ugens mandag for en dag (UTC). 1970-01-01 var en torsdag. Ren funktion.
pub fn mandag(dag: &str) -> Option<String> {
    let ms = dag_ms(dag)?;
    let ugedag = (ms / DOEGN_MS + 3).rem_euclid(7); // 0 = mandag
    Some(dag_noegle(ms - ugedag * DOEGN_MS))
}

/// Dagene i uger, mandag–søndag i UTC: summen af dagene med tal, og "N af M dage". Den første og den sidste uge kan være skåret af perioden. Ren funktion.
pub fn uger(dage: &[DagUd]) -> Vec<UgeUd> {
    let mut ud: Vec<UgeUd> = Vec::new();
    for d in dage {
        let man = mandag(&d.dag).unwrap_or_else(|| d.dag.clone());
        if ud.last().is_none_or(|u| u.dag != man) {
            ud.push(UgeUd {
                dag: man,
                fra: d.dag.clone(),
                til: d.dag.clone(),
                n: None,
                med: 0,
                dage: 0,
            });
        }
        let u = ud.last_mut().expect("lige lagt i");
        u.til = d.dag.clone();
        u.dage += 1;
        if let Some(n) = d.n {
            u.n = Some(u.n.unwrap_or(0) + n);
            u.med += 1;
        }
    }
    ud
}

/// Sti → (visninger, besøg).
type Sider = HashMap<String, (i64, i64)>;

/// "Svarede med sidens almindelige side" (V-BESOEG.2 FIX 1): mange sites svarer 200 med forsiden på adresser, de ikke kender (catch-all), så de kommer
/// aldrig i 404-listen. Et skøn ud fra svarenes størrelse.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ForsideDeling {
    /// Stier, der i hvert domæne, hvor de står, svarede 200 i snit på forsidens størrelse (`sti200::som_forsiden`), ikke er forsiden og har 0 besøg udefra.
    pub som_forsiden: HashSet<String>,
    /// Et domæne har sider med størrelse i perioden, men forsiden (`200 /`) mangler: dets stier deles ikke.
    pub uden_forside: bool,
}

/// Delingen for perioden `fra`…`til` ud fra (domæne, række) med arterne `b_side`, `b_side_b` og `b_side_bytes`. Pr. domæne og vært (V-SUBDOM.1): snittet pr.
/// visning af 200-svarene (kun døgn, hvor sidens bytes er hentet) sammenlignes med samme værts forside, som Følsomme filer gør. 404 og andre svarkoder end 200
/// tæller ikke med i snittet; besøg udefra tæller alle svarkoder undtagen 404. En vært uden sin egen forside deles ikke; et domæne, hvor ingen vært har en
/// forside, er `uden_forside`. Ren funktion.
pub fn forside_deling(rk: &[(String, Raekke)], fra: &str, til: &str) -> ForsideDeling {
    let i_perioden: Vec<&(String, Raekke)> = rk
        .iter()
        .filter(|(_, r)| r.dag.as_str() >= fra && r.dag.as_str() <= til && !r.noegle.starts_with("404 "))
        .collect();
    let bytes: HashMap<(&str, &str, &str), i64> = i_perioden
        .iter()
        .filter(|(_, r)| r.art == SIDE_BYTES)
        .map(|(z, r)| ((z.as_str(), r.dag.as_str(), r.noegle.as_str()), r.n))
        .collect();
    // (domæne, vært, sti) → (visninger med 200 og bytes, bytes, besøg udefra).
    let mut m: HashMap<(&str, &str, &str), (i64, i64, i64)> = HashMap::new();
    for (z, r) in &i_perioden {
        let Some((kode, sti, vaert)) = side_dele(&r.noegle) else {
            continue;
        };
        let x = m.entry((z.as_str(), vaert, sti)).or_default();
        match r.art.as_str() {
            "b_side" if kode == "200" => {
                if let Some(b) = bytes.get(&(z.as_str(), r.dag.as_str(), r.noegle.as_str())) {
                    x.0 += r.n;
                    x.1 += b;
                }
            }
            "b_side_b" => x.2 += r.n,
            _ => {}
        }
    }
    let snit = |x: &(i64, i64, i64)| (x.0 > 0).then(|| x.1 / x.0);
    let forside = |z: &str, v: &str| m.get(&(z, v, crate::sti200::FORSIDE)).and_then(snit).filter(|f| *f > 0);
    let med_bytes: HashSet<&str> = bytes.keys().map(|k| k.0).collect();
    let med_forside: HashSet<&str> = m.keys().filter(|k| forside(k.0, k.1).is_some()).map(|k| k.0).collect();
    let uden_forside = med_bytes.iter().any(|z| !med_forside.contains(z));
    // En sti står kun i den nye liste, når den gør det på alle domæner og værter, hvor den står.
    let mut alle: HashMap<&str, bool> = HashMap::new();
    for ((z, v, sti), x) in &m {
        let ja = *sti != crate::sti200::FORSIDE
            && x.2 == 0
            && matches!((snit(x), forside(z, v)), (Some(s), Some(f)) if crate::sti200::som_forsiden(s, f));
        *alle.entry(sti).or_insert(true) &= ja;
    }
    ForsideDeling {
        som_forsiden: alle.into_iter().filter(|x| x.1).map(|x| x.0.to_string()).collect(),
        uden_forside,
    }
}

/// Svaret ud fra det læste. Ren funktion.
pub fn regn(periode: &Periode, filter: Option<&Filter>, l: &Laest, nu: i64) -> BesoegUd {
    let mut unikke: HashMap<&str, i64> = HashMap::new();
    for r in &l.unikke {
        *unikke.entry(r.dag.as_str()).or_default() += r.n;
    }
    let (land, sti, robot) = match filter {
        Some(Filter::Land(x)) => (Some(x.as_str()), None, None),
        Some(Filter::Sti(x)) => (None, Some(x.as_str()), None),
        Some(Filter::Robot(x)) => (None, None, Some(x.as_str())),
        None => (None, None, None),
    };
    // Indtil B2 (V-SUBDOM.1) lægges samme sti på flere værter sammen i visningen, som før: værten i nøglen vises ikke.
    let side_sti = |noegle: &str| side_dele(noegle).map_or("", |x| x.1).to_string();
    let side_404 = |noegle: &str| noegle.starts_with("404 ");
    // Pr. dag: alle besøg, besøg fra landet, visninger og besøg på stien, besøg fra robotgruppen og fra alle kendte robotter.
    let mut besoeg: HashMap<&str, i64> = HashMap::new();
    let (mut fra_land, mut sti_vis, mut sti_bes, mut fra_gruppe, mut fra_robot) = (
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::<&str, i64>::new(),
    );
    for r in &l.besoeg {
        let dag = r.dag.as_str();
        match r.art.as_str() {
            "b_total" => *besoeg.entry(dag).or_default() += r.n,
            "b_land" if Some(r.noegle.as_str()) == land => *fra_land.entry(dag).or_default() += r.n,
            "b_side" if sti.is_some_and(|s| side_sti(&r.noegle) == s) => *sti_vis.entry(dag).or_default() += r.n,
            "b_side_b" if sti.is_some_and(|s| side_sti(&r.noegle) == s) => *sti_bes.entry(dag).or_default() += r.n,
            "b_robot" if !r.noegle.is_empty() => {
                *fra_robot.entry(dag).or_default() += r.n;
                if Some(robot_gruppe(&r.noegle)) == robot {
                    *fra_gruppe.entry(dag).or_default() += r.n;
                }
            }
            _ => {}
        }
    }
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    let sum = |tal: &HashMap<&str, i64>, hentet: &dyn Fn(&str) -> bool, dage: i64| {
        let i = (1..=dage).map(d).filter(|x| hentet(x));
        let (n, med) = i.fold((0, 0), |(n, med), x| (n + tal.get(x.as_str()).copied().unwrap_or(0), med + 1));
        Sum7 { n, med, dage }
    };
    let har_unikke = |x: &str| unikke.contains_key(x);
    let har_besoeg = |x: &str| l.besoeg_hentet.contains(x);
    let har_side = |x: &str| l.side_hentet.contains(x);
    let (fra, til) = spaend(periode, nu);
    let dage = dage_i(&fra, &til);
    let (serie, hentet): (&HashMap<&str, i64>, HashSet<String>) = match filter {
        Some(Filter::Land(_)) => (&fra_land, l.besoeg_hentet.clone()),
        Some(Filter::Sti(_)) => (&sti_vis, l.side_hentet.clone()),
        Some(Filter::Robot(_)) => (&fra_gruppe, l.robot_hentet.clone()),
        None => (&unikke, unikke.keys().map(|x| x.to_string()).collect()),
    };
    let mut periode_ud = periode_ud(&fra, &til, &hentet, nu);
    // Fra/Til kan højst nå et år tilbage.
    let foerste = d(AAR_DAGE);
    periode_ud.data_fra = periode_ud.data_fra.map(|f| f.max(foerste));
    let i_perioden = |r: &&Raekke| r.dag >= fra && r.dag <= til;
    // Landene i perioden.
    let mut lande: HashMap<&str, i64> = HashMap::new();
    let mut lande_i_alt = 0;
    for r in l.besoeg.iter().filter(i_perioden) {
        match r.art.as_str() {
            "b_land" => *lande.entry(r.noegle.as_str()).or_default() += r.n,
            "b_total" => lande_i_alt += r.n,
            _ => {}
        }
    }
    let mut lande: Vec<LandUd> = lande
        .into_iter()
        .filter(|x| x.1 > 0)
        .map(|(kode, n)| LandUd { kode: kode.to_string(), n })
        .collect();
    lande.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.kode.cmp(&b.kode)));
    // Robotgrupperne i perioden.
    let mut grupper: HashMap<&str, i64> = HashMap::new();
    for r in l
        .besoeg
        .iter()
        .filter(i_perioden)
        .filter(|r| r.art == "b_robot" && !r.noegle.is_empty())
    {
        *grupper.entry(robot_gruppe(&r.noegle)).or_default() += r.n;
    }
    let mut robotter: Vec<GruppeUd> = grupper
        .into_iter()
        .filter(|x| x.1 > 0)
        .map(|(id, n)| GruppeUd { id: id.to_string(), n })
        .collect();
    robotter.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.id.cmp(&b.id)));
    // Siderne i perioden: visninger og besøg pr. sti, 404 for sig.
    let (mut sider, mut sider_404): (Sider, Sider) = (HashMap::new(), HashMap::new());
    for r in l
        .besoeg
        .iter()
        .filter(i_perioden)
        .filter(|r| r.art == "b_side" || r.art == "b_side_b")
    {
        let m = if side_404(&r.noegle) { &mut sider_404 } else { &mut sider };
        let x = m.entry(side_sti(&r.noegle)).or_default();
        if r.art == "b_side" {
            x.0 += r.n;
        } else {
            x.1 += r.n;
        }
    }
    let liste = |m: Sider| {
        let mut v: Vec<SideUd> = m
            .into_iter()
            .filter(|x| x.1 .0 > 0)
            .map(|(sti, (n, besoeg))| SideUd { sti, n, besoeg })
            .collect();
        v.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.sti.cmp(&b.sti)));
        v
    };
    let deling = forside_deling(&l.sider_z, &fra, &til);
    let (sider_forside, sider): (Vec<SideUd>, Vec<SideUd>) = liste(sider).into_iter().partition(|x| deling.som_forsiden.contains(&x.sti));
    let sider_404 = liste(sider_404);
    let sider_i_alt = sider.iter().map(|x| x.n).sum();
    // Besøg fra kendte robotter de seneste 30 dage, på dage med både besøg og robotter. Kun uden filter.
    let robot30 = filter.is_none().then(|| {
        let dage30: Vec<String> = (1..=30)
            .map(d)
            .filter(|x| l.besoeg_hentet.contains(x) && l.robot_hentet.contains(x))
            .collect();
        let tael = |m: &HashMap<&str, i64>| dage30.iter().map(|x| m.get(x.as_str()).copied().unwrap_or(0)).sum();
        RobotAndel {
            n: tael(&fra_robot),
            i_alt: tael(&besoeg),
            med: dage30.len() as i64,
        }
    });
    // IP-klasserne det seneste år.
    let aar_fra = d(AAR_DAGE);
    let klasse_dage: HashSet<&str> = l.ipklasse.iter().filter(|r| r.dag >= aar_fra).map(|r| r.dag.as_str()).collect();
    let ipklasse = (!klasse_dage.is_empty()).then(|| {
        let (n, i_alt) = l.ipklasse.iter().filter(|r| r.dag >= aar_fra).fold((0, 0), |(n, i), r| {
            (n + if ROBOT_KLASSER.contains(&r.noegle.as_str()) { r.n } else { 0 }, i + r.n)
        });
        KlasseAndel {
            n,
            i_alt,
            med: klasse_dage.len() as i64,
        }
    });
    let besoeg30 = match filter {
        Some(Filter::Land(_)) => sum(&fra_land, &har_besoeg, 30),
        Some(Filter::Sti(_)) => sum(&sti_bes, &har_side, 30),
        _ => sum(&besoeg, &har_besoeg, 30),
    };
    let dage_ud: Vec<DagUd> = dage
        .iter()
        .map(|x| DagUd {
            dag: x.clone(),
            n: hentet.contains(x).then(|| serie.get(x.as_str()).copied().unwrap_or(0)),
        })
        .collect();
    BesoegUd {
        i_gaar_dag: d(1),
        i_gaar: unikke.get(d(1).as_str()).copied(),
        d7: sum(&unikke, &har_unikke, 7),
        d30: sum(&unikke, &har_unikke, 30),
        aar: sum(&unikke, &har_unikke, AAR_DAGE),
        besoeg30,
        robot30: robot30.filter(|r| r.med > 0),
        uger: (dage_ud.len() > UGE_GRAENSE).then(|| uger(&dage_ud)),
        dage: dage_ud,
        land: land.map(str::to_string),
        sti: sti.map(str::to_string),
        robot: robot.map(str::to_string),
        lande,
        lande_i_alt,
        lande_med: dage.iter().filter(|x| har_besoeg(x)).count() as i64,
        besoeg_fra: l.besoeg_hentet.iter().min().cloned(),
        ikke_tilgaengelig: l.ikke_tilgaengelig.clone(),
        robotter,
        robot_med: dage.iter().filter(|x| l.robot_hentet.contains(*x)).count() as i64,
        robot_fra: l.robot_hentet.iter().min().cloned(),
        robot_ikke: l.robot_ikke.clone(),
        ipklasse,
        sider,
        sider_i_alt,
        sider_404,
        sider_forside,
        sider_uden_forside: deling.uden_forside,
        side_med: dage.iter().filter(|x| har_side(x)).count() as i64,
        side_fra: l.side_hentet.iter().min().cloned(),
        side_ikke: l.side_ikke.clone(),
        periode: periode_ud,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NU: i64 = 1_793_955_600_000; // 2026-11-06T09:00:00Z

    fn rk(dag: &str, art: &str, noegle: &str, n: i64) -> Raekke {
        Raekke {
            dag: dag.into(),
            art: art.into(),
            noegle: noegle.into(),
            n,
        }
    }

    #[test]
    fn forespoergslen_har_kun_de_kendte_felter_og_filtret() {
        for felt in [
            "httpRequestsAdaptiveGroups",
            "sum { visits }",
            "dimensions { date clientCountryName }",
            r#"requestSource:"eyeball""#,
            "datetime_geq:$since",
            "datetime_lt:$until",
        ] {
            assert!(BESOEG_QUERY.contains(felt), "{felt}");
        }
        for felt in ["clientIP", "userAgent", "clientRequestPath", "clientAsn"] {
            assert!(!BESOEG_QUERY.contains(felt), "{felt}");
        }
    }

    /// Højst 30 dage tilbage, kun færdige døgn, og bagefter kun de nye.
    #[test]
    fn mangler_er_hoejst_30_dage_og_saa_kun_det_nye() {
        let (fra, til) = mangler(&HashMap::new(), NU).unwrap();
        assert_eq!((fra.as_str(), til.as_str()), ("2026-10-07", "2026-11-05"));
        let v = variabler("z", &fra, &til).unwrap();
        assert_eq!(
            (v["since"].as_str(), v["until"].as_str()),
            (Some("2026-10-07T00:00:00Z"), Some("2026-11-06T00:00:00Z"))
        );
        let spaend_s = (dag_ms("2026-11-06").unwrap() - dag_ms("2026-10-07").unwrap()) / 1000;
        assert!(spaend_s <= 2_592_000, "maxDuration 30 dage: {spaend_s}");
        let mut hentet: HashMap<String, String> = (2..=BESOEG_DAGE)
            .map(|i| (dag_noegle(NU - i * DOEGN_MS), "t".to_string()))
            .collect();
        assert_eq!(mangler(&hentet, NU), Some(("2026-11-05".into(), "2026-11-05".into())));
        hentet.insert("2026-11-05".into(), "t".into());
        assert_eq!(mangler(&hentet, NU), None);
        let lige_efter = 1_793_923_200_000 + 60_000; // 2026-11-06T00:01Z: i går er ikke færdig før kl. 01
        assert_eq!(mangler(&HashMap::new(), lige_efter).unwrap().1, "2026-11-04");
        assert!(variabler("z", "x", "2026-11-01").is_err());
    }

    /// Fixture med Cloudflares form: lande lægges sammen pr. dag, tomt land er XX, 0 gemmes ikke pr. land, og et døgn uden besøg er 0.
    #[test]
    fn svaret_bliver_til_b_land_og_b_total() {
        let data = json!({"viewer": {"zones": [{"besoeg": [
            {"sum": {"visits": 40}, "dimensions": {"date": "2026-11-01", "clientCountryName": "DK"}},
            {"sum": {"visits": 2}, "dimensions": {"date": "2026-11-01", "clientCountryName": "DK"}},
            {"sum": {"visits": 7}, "dimensions": {"date": "2026-11-01", "clientCountryName": "US"}},
            {"sum": {"visits": 3}, "dimensions": {"date": "2026-11-01", "clientCountryName": ""}},
            {"sum": {"visits": 0}, "dimensions": {"date": "2026-11-01", "clientCountryName": "SE"}},
            {"sum": null, "dimensions": {"date": "2026-11-02", "clientCountryName": "DE"}},
            {"sum": {"visits": 9}, "dimensions": {"date": "2026-10-01", "clientCountryName": "DK"}}
        ]}]}});
        let dage = vec!["2026-11-01".to_string(), "2026-11-02".to_string(), "2026-11-03".to_string()];
        let r = raekker(&data, &dage).unwrap();
        let tal = |dag: &str, art: &str, n: &str| r.iter().find(|x| x.0 == dag && x.1 == art && x.2 == n).map(|x| x.3);
        assert_eq!(
            (
                tal("2026-11-01", "b_land", "DK"),
                tal("2026-11-01", "b_land", "US"),
                tal("2026-11-01", "b_land", "XX")
            ),
            (Some(42), Some(7), Some(3))
        );
        assert_eq!(tal("2026-11-01", "b_land", "SE"), None, "0 gemmes ikke");
        assert_eq!(tal("2026-11-01", "b_total", "besoeg"), Some(52));
        assert_eq!(
            (tal("2026-11-02", "b_total", "besoeg"), tal("2026-11-03", "b_total", "besoeg")),
            (Some(0), Some(0))
        );
        assert!(r.iter().all(|x| x.0 != "2026-10-01"), "kun de spurgte døgn");
        assert_eq!(raekker(&json!({"viewer": {"zones": []}}), &dage), Err(Fejl::ZoneUkendt));
        assert_eq!(raekker(&json!({"viewer": {"zones": [{"besoeg": "x"}]}}), &dage), Err(Fejl::Svar));
    }

    fn laest() -> Laest {
        let d = |i: i64| dag_noegle(NU - i * DOEGN_MS);
        // Unikke: 100 pr. dag for alle dage i året undtagen d-3 og d-200, og 5 pr. dag før året.
        let mut unikke: Vec<Raekke> = (1..=AAR_DAGE)
            .filter(|i| ![3, 200].contains(i))
            .map(|i| rk(&d(i), "h_total", "unikke", 100))
            .collect();
        unikke.push(rk(&d(400), "h_total", "unikke", 5));
        // Besøg: d-1…d-10 er hentet, d-4 uden besøg.
        let mut besoeg = Vec::new();
        for i in (1..=10).filter(|i| *i != 4) {
            besoeg.push(rk(&d(i), "b_land", "DK", 8));
            besoeg.push(rk(&d(i), "b_land", "US", 2));
            besoeg.push(rk(&d(i), "b_total", "besoeg", 10));
        }
        besoeg.push(rk(&d(4), "b_total", "besoeg", 0));
        Laest {
            unikke,
            besoeg,
            besoeg_hentet: (1..=10).map(d).collect(),
            ..Laest::default()
        }
    }

    /// Summerne for 7, 30 og 365 dage tæller kun dage med tal og siger, hvor mange det er.
    #[test]
    fn summer_for_7_30_og_365_dage_med_manglende_doegn() {
        let u = regn(&Periode::D7, None, &laest(), NU);
        assert_eq!(u.i_gaar, Some(100));
        assert_eq!(u.d7, Sum7 { n: 600, med: 6, dage: 7 });
        assert_eq!(
            u.d30,
            Sum7 {
                n: 2900,
                med: 29,
                dage: 30
            }
        );
        assert_eq!(
            u.aar,
            Sum7 {
                n: 36300,
                med: 363,
                dage: 365
            },
            "d-400 er uden for året"
        );
        assert_eq!(u.besoeg30, Sum7 { n: 90, med: 10, dage: 30 });
        assert_eq!((u.periode.dage, u.periode.med), (7, 6));
        assert_eq!(
            u.dage.iter().find(|x| x.dag == dag_noegle(NU - 3 * DOEGN_MS)).unwrap().n,
            None,
            "et hul er ikke 0"
        );
        assert_eq!(
            u.periode.data_fra,
            Some(dag_noegle(NU - AAR_DAGE * DOEGN_MS)),
            "Fra/Til højst et år tilbage"
        );
        let tom = regn(&Periode::D7, None, &Laest::default(), NU);
        assert_eq!((tom.i_gaar, tom.d7.med, tom.besoeg30.n), (None, 0, 0));
    }

    /// Landene i perioden, flest først, og grafen med et land som filter: besøg fra landet, huller for dage uden hentede besøg.
    #[test]
    fn lande_og_filter() {
        let u = regn(&Periode::D7, None, &laest(), NU);
        assert_eq!(
            u.lande,
            vec![LandUd { kode: "DK".into(), n: 48 }, LandUd { kode: "US".into(), n: 12 }]
        );
        assert_eq!((u.lande_i_alt, u.lande_med), (60, 7));
        assert_eq!(u.besoeg_fra, Some(dag_noegle(NU - 10 * DOEGN_MS)));
        let f = regn(&Periode::D30, Some(&Filter::Land("DK".into())), &laest(), NU);
        assert_eq!((f.periode.dage, f.periode.med), (30, 10), "dækningen er besøgenes");
        let n = |i: i64| f.dage.iter().find(|x| x.dag == dag_noegle(NU - i * DOEGN_MS)).unwrap().n;
        assert_eq!((n(1), n(4), n(20)), (Some(8), Some(0), None));
        assert_eq!(f.land.as_deref(), Some("DK"));
    }

    #[test]
    fn perioden_er_hoejst_et_aar() {
        assert_eq!(
            spaend(&Periode::Alt, NU),
            (dag_noegle(NU - 365 * DOEGN_MS), "2026-11-05".to_string())
        );
        let (fra, til) = spaend(
            &Periode::Spaend {
                fra: "2024-01-01".into(),
                til: "2026-11-05".into(),
            },
            NU,
        );
        assert_eq!(
            (fra.as_str(), til.as_str(), dage_i(&fra, &til).len()),
            ("2025-11-06", "2026-11-05", 365)
        );
        assert_eq!(
            spaend(
                &Periode::Spaend {
                    fra: "2026-11-01".into(),
                    til: "2026-11-03".into()
                },
                NU
            )
            .0,
            "2026-11-01"
        );
        assert_eq!(tjek_land(Some("DK".into())), Ok(Some("DK".into())));
        assert_eq!(tjek_land(Some("T1".into())), Ok(Some("T1".into())));
        for x in ["dk", "DNK", "", "D'"] {
            assert_eq!(tjek_land(Some(x.into())), Err(Fejl::UgyldigtFilter), "{x}");
        }
    }

    /// Lageret: besøgene står for sig (ikke i `raekker`), et døgn uden rækker er hentet, og "Nulstil historik" sletter dem og tilstanden.
    #[test]
    fn lageret_gemmer_og_nulstiller() {
        let mut s = Store::i_hukommelse().unwrap();
        let dage = vec!["2026-11-01".to_string(), "2026-11-02".to_string()];
        let data = json!({"viewer": {"zones": [{"besoeg": [{"sum": {"visits": 4}, "dimensions": {"date": "2026-11-01", "clientCountryName": "DK"}}]}]}});
        s.gem_besoeg("z", &dage, &raekker(&data, &dage).unwrap(), "t").unwrap();
        assert_eq!(s.hentet_besoeg("z").unwrap().len(), 2);
        assert!(
            s.raekker("z", "0000-00-00").unwrap().is_empty(),
            "blander sig ikke i Angreb og Overblik"
        );
        s.gem_afsnit_fejl(BESOEG_FEJL, "z", Some("afvist")).unwrap();
        let l = laes(&s, &["z".to_string()]).unwrap();
        assert_eq!(
            (l.besoeg.len(), l.besoeg_hentet.len(), l.ikke_tilgaengelig.clone()),
            (3, 2, vec!["z".to_string()])
        );
        s.nulstil_historik().unwrap();
        let l = laes(&s, &["z".to_string()]).unwrap();
        assert!(l.besoeg.is_empty() && l.besoeg_hentet.is_empty() && l.ikke_tilgaengelig.is_empty());
    }

    /// Unikke for de gamle døgn: én gang, kun nøglen `unikke`, kun døgn med årstal, og et domæne, der hentes bagefter, venter ikke.
    #[test]
    fn unikke_en_gang_roerer_ingen_andre_tal() {
        let mut s = Store::i_hukommelse().unwrap();
        let dage = vec!["2026-01-01".to_string(), "2026-01-02".to_string()];
        let aar = vec![
            ("2026-01-01".to_string(), "h_total".to_string(), "foresp".to_string(), 90),
            ("2026-01-01".to_string(), "h_land".to_string(), "DK".to_string(), 50),
        ];
        s.gem_historik("z1", &dage, &aar, "t").unwrap();
        assert!(s.unikke_en_gang("2026-11-06").unwrap());
        assert!(!s.unikke_en_gang("2026-11-07").unwrap(), "kun én gang");
        assert_eq!(s.indstilling(crate::store::UNIKKE_V1).unwrap().as_deref(), Some("2026-11-06"));
        s.gem_historik("z2", &dage, &[], "t").unwrap();
        assert!(
            s.unikke_mangler("z1").unwrap() && !s.unikke_mangler("z2").unwrap(),
            "z2 kom efter: får tallet med det samme"
        );
        s.gem_unikke("z1", &[("2026-01-01".to_string(), 17), ("2026-01-05".to_string(), 3)])
            .unwrap();
        assert!(!s.unikke_mangler("z1").unwrap());
        let mut rk: Vec<(String, String, String, i64)> = s
            .raekker_art(&["z1".to_string()], &["h_total", "h_land", "h_status"], "0000-00-00")
            .unwrap()
            .into_iter()
            .map(|(_, r)| (r.dag, r.art, r.noegle, r.n))
            .collect();
        rk.sort();
        assert_eq!(
            rk,
            vec![
                ("2026-01-01".into(), "h_land".into(), "DK".into(), 50),
                ("2026-01-01".into(), "h_total".into(), "foresp".into(), 90),
                ("2026-01-01".into(), "h_total".into(), "unikke".into(), 17),
            ],
            "intet andet tal ændret, og 5. jan. uden årstal fik ingen række"
        );
        assert_eq!(
            s.hentet_historik("z1").unwrap().get("2026-01-01").map(String::as_str),
            Some("t"),
            "hentet-tiden står"
        );
    }

    /// Robot- og sideforespørgslerne: de kendte felter og filtre, og aldrig IP, brugeragent eller forespørgselsstreng.
    #[test]
    fn robot_og_side_forespoergslerne() {
        for felt in [
            "httpRequestsAdaptiveGroups",
            "count sum { visits }",
            "dimensions { date verifiedBotCategory }",
            r#"requestSource:"eyeball""#,
        ] {
            assert!(ROBOT_QUERY.contains(felt), "{felt}");
        }
        assert!(!ROBOT_QUERY.contains("edgeResponseBytes"));
        for felt in [
            "httpRequestsAdaptiveGroups",
            "count sum { visits edgeResponseBytes }",
            "dimensions { date clientRequestPath edgeResponseStatus clientRequestHTTPHost }",
            r#"requestSource:"eyeball""#,
            r#"edgeResponseContentTypeName:"html""#,
            "orderBy:[count_DESC]",
        ] {
            assert!(SIDE_QUERY.contains(felt), "{felt}");
        }
        for q in [ROBOT_QUERY, SIDE_QUERY] {
            for felt in ["clientIP", "userAgent", "clientRequestQuery", "clientAsn"] {
                assert!(!q.contains(felt), "{felt}");
            }
            assert!(q.contains("datetime_geq:$since") && q.contains("datetime_lt:$until"));
        }
        assert!(!ROBOT_QUERY.contains("clientRequestPath"));
    }

    /// Robotsvaret: kategorier pr. dag lægges sammen, tom kategori står for sig, 0 og andre døgn gemmes ikke.
    #[test]
    fn robotsvaret_bliver_til_b_robot() {
        let data = json!({"viewer": {"zones": [{"robot": [
            {"count": 50, "sum": {"visits": 30}, "dimensions": {"date": "2026-11-01", "verifiedBotCategory": "Search Engine Crawler"}},
            {"count": 5, "sum": {"visits": 4}, "dimensions": {"date": "2026-11-01", "verifiedBotCategory": "Search Engine Crawler"}},
            {"count": 900, "sum": {"visits": 700}, "dimensions": {"date": "2026-11-01", "verifiedBotCategory": ""}},
            {"count": 2, "sum": {"visits": 0}, "dimensions": {"date": "2026-11-01", "verifiedBotCategory": "AI Crawler"}},
            {"count": 9, "sum": {"visits": 9}, "dimensions": {"date": "2026-10-01", "verifiedBotCategory": "AI Crawler"}}
        ]}]}});
        let r = robot_raekker(&data, &["2026-11-01".to_string()]).unwrap();
        assert_eq!(
            r,
            vec![
                ("2026-11-01".into(), "b_robot".into(), "".into(), 700),
                ("2026-11-01".into(), "b_robot".into(), "Search Engine Crawler".into(), 34),
            ]
        );
        assert_eq!(robot_raekker(&json!({"viewer": {"zones": [{"robot": 3}]}}), &[]), Err(Fejl::Svar));
    }

    /// Sidesvaret: stien renses (ingen forespørgselsstreng, lange token-agtige stykker skjult), ens stier lægges sammen, svarkoden er i nøglen, og kun
    /// de 50 med flest visninger pr. døgn gemmes.
    #[test]
    fn sidesvaret_bliver_til_b_side_top_50() {
        let mut grupper = vec![
            json!({"count": 100, "sum": {"visits": 60}, "dimensions": {"date": "2026-11-01", "clientRequestPath": "/", "edgeResponseStatus": 200}}),
            json!({"count": 7, "sum": {"visits": 7}, "dimensions": {"date": "2026-11-01", "clientRequestPath": "/admin", "edgeResponseStatus": 404}}),
            json!({"count": 3, "sum": {"visits": 1}, "dimensions": {"date": "2026-11-01", "clientRequestPath": "/login?u=hemmelig", "edgeResponseStatus": 200}}),
            json!({"count": 2, "sum": {"visits": 0}, "dimensions": {"date": "2026-11-01", "clientRequestPath": "/login", "edgeResponseStatus": 200}}),
            json!({"count": 4, "sum": {"visits": 4}, "dimensions": {"date": "2026-11-01", "clientRequestPath": "/n/abcdefghijklmnopqrstuvwxyz0123456789ABCD", "edgeResponseStatus": 200}}),
        ];
        for i in 0..60 {
            grupper.push(json!({"count": 1, "sum": {"visits": 1}, "dimensions": {"date": "2026-11-02", "clientRequestPath": format!("/s{i:02}"), "edgeResponseStatus": 200}}));
        }
        let data = json!({"viewer": {"zones": [{"side": grupper}]}});
        let dage = vec!["2026-11-01".to_string(), "2026-11-02".to_string()];
        let r = side_raekker(&data, &dage, "wi24rd.com").unwrap();
        let tal = |art: &str, n: &str| r.iter().find(|x| x.0 == "2026-11-01" && x.1 == art && x.2 == n).map(|x| x.3);
        assert_eq!((tal("b_side", "200 /"), tal("b_side_b", "200 /")), (Some(100), Some(60)));
        assert_eq!(tal("b_side", "404 /admin"), Some(7));
        assert_eq!(
            (tal("b_side", "200 /login"), tal("b_side_b", "200 /login")),
            (Some(5), Some(1)),
            "forespørgselsstrengen væk, og stierne lagt sammen"
        );
        assert_eq!(tal("b_side", "200 /n/…"), Some(4), "token-agtigt stykke skjult");
        assert!(r.iter().all(|x| !x.2.contains('?') && !x.2.contains("hemmelig")));
        assert_eq!(
            r.iter().filter(|x| x.0 == "2026-11-02" && x.1 == "b_side").count(),
            50,
            "top 50 pr. døgn"
        );
        let lang = ren_sti(&format!("/{}", "a/".repeat(80)));
        assert!(lang.chars().count() <= 90);
    }

    /// Bytes (V-BESOEG.2 FIX 1): summen pr. døgn og nøgle som `b_side_bytes`, altid (også 0), og kun for de sider, der gemmes.
    #[test]
    fn sidesvaret_har_bytes_pr_side() {
        let mut grupper = vec![
            json!({"count": 10, "sum": {"visits": 6, "edgeResponseBytes": 200_000}, "dimensions": {"date": "2026-11-01", "clientRequestPath": "/", "edgeResponseStatus": 200}}),
            json!({"count": 2, "sum": {"visits": 0, "edgeResponseBytes": 41_000}, "dimensions": {"date": "2026-11-01", "clientRequestPath": "//crm/.env", "edgeResponseStatus": 200}}),
            json!({"count": 1, "sum": {"visits": 0, "edgeResponseBytes": 20_000}, "dimensions": {"date": "2026-11-01", "clientRequestPath": "//crm/.env?x=1", "edgeResponseStatus": 200}}),
            json!({"count": 3, "sum": {"visits": 3}, "dimensions": {"date": "2026-11-01", "clientRequestPath": "/om", "edgeResponseStatus": 304}}),
        ];
        for i in 0..55 {
            grupper.push(json!({"count": 1, "sum": {"visits": 1, "edgeResponseBytes": 5}, "dimensions": {"date": "2026-11-02", "clientRequestPath": format!("/s{i:02}"), "edgeResponseStatus": 200}}));
        }
        let data = json!({"viewer": {"zones": [{"side": grupper}]}});
        let r = side_raekker(&data, &["2026-11-01".to_string(), "2026-11-02".to_string()], "wi24rd.com").unwrap();
        let tal = |art: &str, n: &str| r.iter().find(|x| x.0 == "2026-11-01" && x.1 == art && x.2 == n).map(|x| x.3);
        assert_eq!(tal(SIDE_BYTES, "200 /"), Some(200_000));
        assert_eq!(tal(SIDE_BYTES, "200 //crm/.env"), Some(61_000), "stierne lagt sammen, også bytes");
        assert_eq!(tal(SIDE_BYTES, "304 /om"), Some(0), "uden bytes i svaret: 0, men rækken står");
        assert_eq!(
            r.iter().filter(|x| x.0 == "2026-11-02" && x.1 == SIDE_BYTES).count(),
            50,
            "kun de 50 gemte"
        );
    }

    fn zr(zone: &str, dag: &str, art: &str, noegle: &str, n: i64) -> (String, Raekke) {
        (zone.into(), rk(dag, art, noegle, n))
    }

    /// Siderne til delingen: forsiden 20 KB; en catch-all-sti på forsidens størrelse uden besøg; en rigtig side på samme størrelse med besøg; en lille fil.
    fn sider_z(zone: &str, dag: &str) -> Vec<(String, Raekke)> {
        vec![
            zr(zone, dag, "b_side", "200 /", 100),
            zr(zone, dag, SIDE_BYTES, "200 /", 2_000_000),
            zr(zone, dag, "b_side_b", "200 /", 40),
            zr(zone, dag, "b_side", "200 //crm/.env", 3),
            zr(zone, dag, SIDE_BYTES, "200 //crm/.env", 3 * 23_000), // +15 %
            zr(zone, dag, "b_side", "200 /platform/", 10),
            zr(zone, dag, SIDE_BYTES, "200 /platform/", 10 * 20_500),
            zr(zone, dag, "b_side_b", "200 /platform/", 4),
            zr(zone, dag, "b_side", "200 /robots.txt", 5),
            zr(zone, dag, SIDE_BYTES, "200 /robots.txt", 5 * 400),
            zr(zone, dag, "b_side", "404 /.env", 7),
            zr(zone, dag, SIDE_BYTES, "404 /.env", 7 * 20_000),
        ]
    }

    /// Delingen: ligner forsiden + ikke forsiden + 0 besøg udefra → ny liste; forsiden selv aldrig; uden forside ingen deling; alle domæner skal være enige.
    #[test]
    fn forside_deling_ligner_forsiden_og_0_besoeg() {
        let (fra, til) = ("2026-11-01", "2026-11-05");
        let mut rk = sider_z("a", "2026-11-02");
        rk.push(zr("a", "2026-11-02", "b_side", "200 /gammel", 4)); // døgn uden bytes: kan ikke vurderes
        rk.push(zr("a", "2026-11-02", "b_side", "301 /flyttet", 4));
        rk.push(zr("a", "2026-11-02", SIDE_BYTES, "301 /flyttet", 4 * 20_000)); // kun 200 tæller
        rk.push(zr("a", "2026-10-01", "b_side", "200 /udenfor", 1));
        rk.push(zr("a", "2026-10-01", SIDE_BYTES, "200 /udenfor", 20_000));
        let d = forside_deling(&rk, fra, til);
        assert_eq!(d.som_forsiden, HashSet::from(["//crm/.env".to_string()]));
        assert!(!d.uden_forside);
        // Grænsen: +25 % er med, +26 % er ikke.
        for (bytes, med) in [(25_000, true), (25_200, false), (15_000, true), (14_800, false)] {
            let mut rk = sider_z("a", "2026-11-02");
            rk.retain(|x| !(x.1.art == SIDE_BYTES && x.1.noegle == "200 //crm/.env"));
            rk.push(zr("a", "2026-11-02", SIDE_BYTES, "200 //crm/.env", 3 * bytes));
            assert_eq!(forside_deling(&rk, fra, til).som_forsiden.contains("//crm/.env"), med, "{bytes}");
        }
        // Forsiden selv står aldrig i listen, heller ikke uden besøg.
        let mut uden_besoeg = sider_z("a", "2026-11-02");
        uden_besoeg.retain(|x| x.1.art != "b_side_b");
        assert!(!forside_deling(&uden_besoeg, fra, til).som_forsiden.contains("/"));
        // Ingen forside i perioden: ingen deling, og det siges.
        let mut uden = sider_z("a", "2026-11-02");
        uden.retain(|x| !x.1.noegle.ends_with(" /"));
        let d = forside_deling(&uden, fra, til);
        assert!(d.som_forsiden.is_empty() && d.uden_forside);
        // Ingen bytes hentet endnu: ingen deling og ingen linje.
        let mut ingen: Vec<_> = sider_z("a", "2026-11-02");
        ingen.retain(|x| x.1.art != SIDE_BYTES);
        assert_eq!(forside_deling(&ingen, fra, til), ForsideDeling::default());
        // To domæner: på b er stien en rigtig side (besøg), så den bliver stående.
        let mut to = sider_z("a", "2026-11-02");
        to.extend(sider_z("b", "2026-11-03"));
        to.push(zr("b", "2026-11-03", "b_side_b", "200 //crm/.env", 1));
        assert!(forside_deling(&to, fra, til).som_forsiden.is_empty());
        // Hvert domæne har sin egen forside: b's sider er 2 KB, a's forside 20 KB.
        let mut egen = sider_z("a", "2026-11-02");
        egen.push(zr("b", "2026-11-02", "b_side", "200 /", 10));
        egen.push(zr("b", "2026-11-02", SIDE_BYTES, "200 /", 10 * 2_000));
        egen.push(zr("b", "2026-11-02", "b_side", "200 /ukendt", 2));
        egen.push(zr("b", "2026-11-02", SIDE_BYTES, "200 /ukendt", 2 * 2_100));
        egen.push(zr("b", "2026-11-02", "b_side", "200 /stor", 2));
        egen.push(zr("b", "2026-11-02", SIDE_BYTES, "200 /stor", 2 * 20_000));
        let d = forside_deling(&egen, fra, til);
        assert!(d.som_forsiden.contains("/ukendt") && !d.som_forsiden.contains("/stor"), "{d:?}");
    }

    /// Visningen: stien, der ligner forsiden, står i `sider_forside` og ikke i `sider`, og andelen regnes af de rigtige sider.
    #[test]
    fn mest_besoegte_sider_uden_catch_all() {
        let mut l = Laest::default();
        let dag = dag_noegle(NU - DOEGN_MS);
        l.side_hentet.insert(dag.clone());
        l.sider_z = sider_z("z", &dag);
        l.besoeg = l.sider_z.iter().map(|x| x.1.clone()).collect();
        let u = regn(&Periode::D7, None, &l, NU);
        assert_eq!(
            u.sider.iter().map(|x| x.sti.as_str()).collect::<Vec<_>>(),
            ["/", "/platform/", "/robots.txt"]
        );
        assert_eq!(
            u.sider_forside,
            vec![SideUd {
                sti: "//crm/.env".into(),
                n: 3,
                besoeg: 0
            }]
        );
        assert_eq!((u.sider_i_alt, u.sider_404.len(), u.sider_uden_forside), (115, 1, false));
        l.sider_z.retain(|x| !x.1.noegle.ends_with(" /"));
        let u = regn(&Periode::D7, None, &l, NU);
        assert!(
            u.sider_forside.is_empty() && u.sider_uden_forside && u.sider.len() == 4,
            "uden forside: alt står som før"
        );
    }

    /// V-SUBDOM.1: nøglen for domænet selv er uændret ("<svarkode> <sti>"), og samme sti på to værter bliver to nøgler ("…\t<vært>"). Værten renses (port,
    /// store bogstaver, IP), og et svar uden værtsfeltet giver nøglerne fra før.
    #[test]
    fn sidenoeglen_har_vaerten_kun_uden_for_domaenet_selv() {
        let g = |vaert: Option<&str>, sti: &str, n: i64| {
            let mut d = json!({"date": "2026-11-01", "clientRequestPath": sti, "edgeResponseStatus": 200});
            if let Some(v) = vaert {
                d["clientRequestHTTPHost"] = json!(v);
            }
            json!({"count": n, "sum": {"visits": n, "edgeResponseBytes": n * 10}, "dimensions": d})
        };
        let data = json!({"viewer": {"zones": [{"side": [
            g(Some("wi24rd.com"), "/", 9), g(Some("wi24rd.com:443"), "/", 1), g(Some("SHOP.wi24rd.com"), "/", 5), g(Some("shop.wi24rd.com"), "/om", 2),
            g(Some("192.0.2.7"), "/", 1),
        ]}]}});
        let r = side_raekker(&data, &["2026-11-01".to_string()], "wi24rd.com").unwrap();
        let noegler: Vec<&str> = r.iter().filter(|x| x.1 == "b_side").map(|x| x.2.as_str()).collect();
        assert_eq!(noegler, ["200 /", "200 /\tshop", "200 /om\tshop", "200 /\t[ip]"]);
        let tal = |n: &str| r.iter().find(|x| x.1 == "b_side" && x.2 == n).unwrap().3;
        assert_eq!((tal("200 /"), tal("200 /\tshop")), (10, 5), "apex med port er apex");
        assert_eq!(side_dele("200 /om\tshop"), Some(("200", "/om", "shop")));
        assert_eq!(side_dele("200 /a b"), Some(("200", "/a b", "")), "en gammel nøgle har ingen vært");
        let uden = json!({"viewer": {"zones": [{"side": [g(None, "/", 3)]}]}});
        assert_eq!(
            side_raekker(&uden, &["2026-11-01".to_string()], "wi24rd.com").unwrap()[0].2,
            "200 /",
            "som før V-SUBDOM.1"
        );
        assert!(!side_afskaaret(&data).unwrap());
        let mange: Vec<Value> = (0..crate::dyk::LOFT).map(|i| g(Some("wi24rd.com"), &format!("/s{i}"), 1)).collect();
        assert!(side_afskaaret(&json!({"viewer": {"zones": [{"side": mange}]}})).unwrap());
    }

    /// Indtil B2 lægges samme sti på flere værter sammen i visningen, som før: ingen tabulator og ingen vært i `sider`, og stifilteret rammer alle værter.
    #[test]
    fn visningen_laegger_vaerterne_sammen_indtil_b2() {
        let mut l = Laest::default();
        let dag = dag_noegle(NU - DOEGN_MS);
        l.side_hentet.insert(dag.clone());
        l.besoeg = vec![
            rk(&dag, "b_side", "200 /", 10),
            rk(&dag, "b_side_b", "200 /", 4),
            rk(&dag, "b_side", "200 /\tshop", 5),
            rk(&dag, "b_side_b", "200 /\tshop", 2),
            rk(&dag, "b_side", "404 /x\tshop", 1),
        ];
        let u = regn(&Periode::D7, None, &l, NU);
        assert_eq!(
            u.sider,
            vec![SideUd {
                sti: "/".into(),
                n: 15,
                besoeg: 6
            }]
        );
        assert_eq!(u.sider_404[0].sti, "/x");
        let f = regn(&Periode::D7, Some(&Filter::Sti("/".into())), &l, NU);
        assert_eq!(f.dage.last().unwrap().n, Some(15), "stifilteret tæller alle værter");
    }

    /// `forside_deling` pr. vært: et subdomæne med sin egen (lille) forside sammenlignes med den, ikke med domænets. Med to forsider deles stien på den vært,
    /// hvor den ligner værtens forside, og ikke på den anden; en vært uden forside deles ikke, og domænet er ikke `uden_forside`, når en vært har en.
    #[test]
    fn forside_deling_med_to_forsider() {
        let (fra, til) = ("2026-11-01", "2026-11-05");
        let d = "2026-11-02";
        let mut rk = sider_z("a", d); // domænet selv: forsiden 20 KB, "//crm/.env" ligner den
        for (art, n) in [("b_side", 10), (SIDE_BYTES, 10 * 2_000)] {
            rk.push(zr("a", d, art, "200 /\tshop", n)); // shop: forsiden 2 KB
        }
        for (art, n) in [("b_side", 2), (SIDE_BYTES, 2 * 2_100)] {
            rk.push(zr("a", d, art, "200 /ukendt\tshop", n));
        }
        for (art, n) in [("b_side", 2), (SIDE_BYTES, 2 * 20_000)] {
            rk.push(zr("a", d, art, "200 /stor\tshop", n)); // på domænets størrelse, men ikke på shops
        }
        for (art, n) in [("b_side", 2), (SIDE_BYTES, 2 * 2_000)] {
            rk.push(zr("a", d, art, "200 /ingen\tapi", n)); // api har ingen forside
        }
        let deling = forside_deling(&rk, fra, til);
        assert!(
            deling.som_forsiden.contains("//crm/.env") && deling.som_forsiden.contains("/ukendt"),
            "{deling:?}"
        );
        assert!(
            !deling.som_forsiden.contains("/stor") && !deling.som_forsiden.contains("/ingen"),
            "{deling:?}"
        );
        assert!(!deling.som_forsiden.contains("/"), "forsiden selv aldrig");
        assert!(!deling.uden_forside, "domænet har en forside");
        // Samme sti på to værter: deles kun, når den ligner forsiden på begge.
        for (art, n) in [("b_side", 3), (SIDE_BYTES, 3 * 2_000)] {
            rk.push(zr("a", d, art, "200 //crm/.env\tshop", n));
        }
        assert!(
            forside_deling(&rk, fra, til).som_forsiden.contains("//crm/.env"),
            "ligner begge forsider"
        );
        rk.push(zr("a", d, "b_side_b", "200 //crm/.env\tshop", 1));
        assert!(
            !forside_deling(&rk, fra, til).som_forsiden.contains("//crm/.env"),
            "besøg på shop: en rigtig side"
        );
    }

    /// Sidernes døgn i hentevinduet hentes én gang igen med bytes: kun `hentet_side` fra og med vinduet røres, og tallene står.
    #[test]
    fn side_bytes_en_gang_glemmer_kun_vinduets_hentet_side() {
        let mut s = Store::i_hukommelse().unwrap();
        let dage = vec!["2026-09-01".to_string(), "2026-11-01".to_string()];
        let rk = vec![("2026-11-01".to_string(), "b_side".to_string(), "200 /".to_string(), 9)];
        s.gem_side("z", &dage, &rk, "t").unwrap();
        s.gem_robot("z", &dage, &[], "t").unwrap();
        assert!(s.side_bytes_en_gang("2026-11-10", "2026-10-11").unwrap());
        assert!(!s.side_bytes_en_gang("2026-11-11", "2026-10-12").unwrap(), "kun én gang");
        assert_eq!(s.hentet_side("z").unwrap().into_keys().collect::<Vec<_>>(), ["2026-09-01"]);
        assert_eq!(s.hentet_robot("z").unwrap().len(), 2, "robotterne røres ikke");
        assert_eq!(
            laes(&s, &["z".to_string()]).unwrap().besoeg.len(),
            1,
            "tallene står, til de nye erstatter dem"
        );
        s.gem_side(
            "z",
            &dage[1..],
            &[("2026-11-01".into(), SIDE_BYTES.into(), "200 /".into(), 0)],
            "t2",
        )
        .unwrap();
        let l = laes(&s, &["z".to_string()]).unwrap();
        assert_eq!(
            (l.besoeg.len(), l.sider_z.len()),
            (1, 1),
            "erstattet for døgnet, b_side_bytes også med domænet"
        );
        assert!(
            s.raekker("z", "0000-00-00").unwrap().is_empty(),
            "blander sig ikke i Angreb og Overblik"
        );
    }

    /// Uger: mandag–søndag i UTC, skåret af perioden, summen af dagene med tal, og "N af M dage".
    #[test]
    fn uger_mandag_til_soendag() {
        assert_eq!(mandag("2026-11-09").as_deref(), Some("2026-11-09"), "en mandag");
        assert_eq!(mandag("2026-11-08").as_deref(), Some("2026-11-02"), "en søndag");
        assert_eq!(mandag("2026-11-04").as_deref(), Some("2026-11-02"));
        let dage: Vec<DagUd> = dage_i("2026-10-29", "2026-11-10")
            .into_iter()
            .map(|d| DagUd {
                n: (d != "2026-11-03" && d != "2026-11-04").then_some(10),
                dag: d,
            })
            .collect();
        let u = uger(&dage);
        assert_eq!(u.len(), 3);
        assert_eq!(
            u[0],
            UgeUd {
                dag: "2026-10-26".into(),
                fra: "2026-10-29".into(),
                til: "2026-11-01".into(),
                n: Some(40),
                med: 4,
                dage: 4
            }
        );
        assert_eq!(
            u[1],
            UgeUd {
                dag: "2026-11-02".into(),
                fra: "2026-11-02".into(),
                til: "2026-11-08".into(),
                n: Some(50),
                med: 5,
                dage: 7
            }
        );
        assert_eq!((u[2].fra.as_str(), u[2].til.as_str(), u[2].dage), ("2026-11-09", "2026-11-10", 2));
        let tom = uger(&[DagUd {
            dag: "2026-11-02".into(),
            n: None,
        }]);
        assert_eq!((tom[0].n, tom[0].med), (None, 0), "en uge uden tal er et hul");
        // Et år i visningen er uger; 30 dage er dage.
        assert!(regn(&Periode::Alt, None, &laest(), NU)
            .uger
            .is_some_and(|u| (52..=54).contains(&u.len())));
        assert!(regn(&Periode::D30, None, &laest(), NU).uger.is_none());
    }

    fn med_robot_og_side() -> Laest {
        let d = |i: i64| dag_noegle(NU - i * DOEGN_MS);
        let mut l = laest();
        for i in 1..=5 {
            l.besoeg.push(rk(&d(i), "b_robot", "Search Engine Crawler", 2));
            l.besoeg.push(rk(&d(i), "b_robot", "AI Crawler", 1));
            l.besoeg.push(rk(&d(i), "b_robot", "", 7));
            l.besoeg.push(rk(&d(i), "b_side", "200 /", 20));
            l.besoeg.push(rk(&d(i), "b_side_b", "200 /", 5));
            l.besoeg.push(rk(&d(i), "b_side", "301 /", 1));
            l.besoeg.push(rk(&d(i), "b_side", "200 /om", 4));
            l.besoeg.push(rk(&d(i), "b_side", "404 /admin", 3));
        }
        l.robot_hentet = (1..=5).map(d).collect();
        l.side_hentet = (1..=6).map(d).collect();
        l.ipklasse = vec![
            rk(&d(10), IPKLASSE, "searchEngine", 30),
            rk(&d(10), IPKLASSE, "scan", 10),
            rk(&d(10), IPKLASSE, "noRecord", 60),
            rk(&d(400), IPKLASSE, "scan", 1000),
        ];
        l
    }

    /// Fliserne følger filtret: Besøg på 30 dage fra landet eller på stien; robot-andelen kun uden filter; IP-klasserne det seneste år.
    #[test]
    fn flisen_besoeg_foelger_land_og_sti_og_robotandelen() {
        let l = med_robot_og_side();
        let u = regn(&Periode::D30, None, &l, NU);
        assert_eq!(u.besoeg30, Sum7 { n: 90, med: 10, dage: 30 });
        assert_eq!(
            u.robot30,
            Some(RobotAndel { n: 15, i_alt: 40, med: 5 }),
            "3 kendte robotbesøg pr. dag på de 5 dage med begge (d-4 havde 0 besøg); tom kategori tæller ikke"
        );
        assert_eq!(
            u.ipklasse,
            Some(KlasseAndel { n: 40, i_alt: 100, med: 1 }),
            "d-400 er uden for året"
        );
        let land = regn(&Periode::D30, Some(&Filter::Land("DK".into())), &l, NU);
        assert_eq!((land.besoeg30.n, land.besoeg30.med), (72, 10), "besøg fra Danmark");
        assert_eq!(land.robot30, None);
        let sti = regn(&Periode::D30, Some(&Filter::Sti("/".into())), &l, NU);
        assert_eq!((sti.besoeg30.n, sti.besoeg30.med), (25, 6), "besøg på stien, dage med sider hentet");
        let n = |u: &BesoegUd, i: i64| u.dage.iter().find(|x| x.dag == dag_noegle(NU - i * DOEGN_MS)).unwrap().n;
        assert_eq!(
            (n(&sti, 1), n(&sti, 6), n(&sti, 7)),
            (Some(21), Some(0), None),
            "visninger af siden pr. dag (alle svarkoder), hul uden sider"
        );
        assert_eq!((sti.periode.med, sti.sti.as_deref()), (6, Some("/")));
        let rob = regn(&Periode::D7, Some(&Filter::Robot(crate::robotter::SOEGEMASKINER.into())), &l, NU);
        assert_eq!((n(&rob, 1), n(&rob, 6)), (Some(2), None), "besøg fra gruppen pr. dag");
    }

    /// Kendte robotter pr. gruppe (Robotter-visningens navne), og siderne med 404 for sig.
    #[test]
    fn robotgrupper_og_sider() {
        let u = regn(&Periode::D7, None, &med_robot_og_side(), NU);
        assert_eq!(
            u.robotter,
            vec![
                GruppeUd {
                    id: crate::robotter::SOEGEMASKINER.into(),
                    n: 10
                },
                GruppeUd {
                    id: crate::robotter::AI_TRAENING.into(),
                    n: 5
                },
            ]
        );
        assert_eq!((u.robot_med, u.side_med), (5, 6));
        assert_eq!(
            u.sider,
            vec![
                SideUd {
                    sti: "/".into(),
                    n: 105,
                    besoeg: 25
                },
                SideUd {
                    sti: "/om".into(),
                    n: 20,
                    besoeg: 0
                }
            ]
        );
        assert_eq!(u.sider_i_alt, 125);
        assert_eq!(
            u.sider_404,
            vec![SideUd {
                sti: "/admin".into(),
                n: 15,
                besoeg: 0
            }]
        );
    }

    #[test]
    fn filtret_er_hoejst_et_og_kendt() {
        assert_eq!(tjek_filter(None, None, None), Ok(None));
        assert_eq!(tjek_filter(Some("DK".into()), None, None), Ok(Some(Filter::Land("DK".into()))));
        assert_eq!(
            tjek_filter(None, Some("/om/os".into()), None),
            Ok(Some(Filter::Sti("/om/os".into())))
        );
        assert_eq!(tjek_filter(None, None, Some("seo".into())), Ok(Some(Filter::Robot("seo".into()))));
        for (l, st, r) in [
            (Some("DK"), Some("/"), None),
            (None, Some("om"), None),
            (None, Some("/a?b=1"), None),
            (None, None, Some("Search Engine Crawler")),
        ] {
            assert_eq!(
                tjek_filter(l.map(Into::into), st.map(Into::into), r.map(Into::into)),
                Err(Fejl::UgyldigtFilter),
                "{l:?} {st:?} {r:?}"
            );
        }
        assert_eq!(
            tjek_filter(None, Some(format!("/{}", "x".repeat(90))), None),
            Err(Fejl::UgyldigtFilter)
        );
    }

    /// Lageret: robotter og sider står for sig (ikke i `raekker`), hver med sin `hentet_*` og fejl, og "Nulstil historik" sletter dem.
    #[test]
    fn lageret_robot_og_side() {
        let mut s = Store::i_hukommelse().unwrap();
        let dage = vec!["2026-11-01".to_string()];
        s.gem_robot("z", &dage, &[("2026-11-01".into(), "b_robot".into(), "AI Crawler".into(), 3)], "t")
            .unwrap();
        s.gem_side(
            "z",
            &dage,
            &[
                ("2026-11-01".into(), "b_side".into(), "200 /".into(), 9),
                ("2026-11-01".into(), "b_robot".into(), "x".into(), 1),
            ],
            "t",
        )
        .unwrap();
        assert!(
            s.raekker("z", "0000-00-00").unwrap().is_empty(),
            "blander sig ikke i Angreb og Overblik"
        );
        s.gem_afsnit_fejl(SIDE_FEJL, "z", Some("afvist")).unwrap();
        let l = laes(&s, &["z".to_string()]).unwrap();
        assert_eq!(l.besoeg.len(), 2, "gem_side gemmer kun sine egne arter");
        assert_eq!((l.robot_hentet.len(), l.side_hentet.len(), l.besoeg_hentet.len()), (1, 1, 0));
        assert_eq!((l.side_ikke.clone(), l.robot_ikke.len()), (vec!["z".to_string()], 0));
        s.nulstil_historik().unwrap();
        let l = laes(&s, &["z".to_string()]).unwrap();
        assert!(l.besoeg.is_empty() && l.robot_hentet.is_empty() && l.side_hentet.is_empty() && l.side_ikke.is_empty());
    }

    /// IP-klasserne for de gamle døgn: én gang, kun art `h_ipklasse`, kun døgn med årstal; intet andet tal og ingen hentet-tid ændres.
    #[test]
    fn ipklasse_en_gang_roerer_ingen_andre_tal() {
        let mut s = Store::i_hukommelse().unwrap();
        let dage = vec!["2026-01-01".to_string()];
        let aar = vec![("2026-01-01".to_string(), "h_total".to_string(), "foresp".to_string(), 90)];
        s.gem_historik("z1", &dage, &aar, "t").unwrap();
        assert!(s.ipklasse_en_gang("2026-11-06").unwrap());
        assert!(!s.ipklasse_en_gang("2026-11-07").unwrap());
        assert!(s.ipklasse_mangler("z1").unwrap());
        s.gem_ipklasse(
            "z1",
            &[
                ("2026-01-01".into(), IPKLASSE.into(), "scan".into(), 4),
                ("2026-01-05".into(), IPKLASSE.into(), "scan".into(), 1),
                ("2026-01-01".into(), "h_total".into(), "foresp".into(), 1),
            ],
        )
        .unwrap();
        assert!(!s.ipklasse_mangler("z1").unwrap());
        let mut rk: Vec<(String, String, String, i64)> = s
            .raekker_art(&["z1".to_string()], &["h_total", IPKLASSE], "0000-00-00")
            .unwrap()
            .into_iter()
            .map(|(_, r)| (r.dag, r.art, r.noegle, r.n))
            .collect();
        rk.sort();
        assert_eq!(
            rk,
            vec![
                ("2026-01-01".into(), IPKLASSE.into(), "scan".into(), 4),
                ("2026-01-01".into(), "h_total".into(), "foresp".into(), 90),
            ]
        );
        assert_eq!(s.hentet_historik("z1").unwrap().get("2026-01-01").map(String::as_str), Some("t"));
    }
}
