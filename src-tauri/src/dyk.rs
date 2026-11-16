//! Dyk: de gemte tal for ét filter (regel, land, netværk, sti, datacenter, metode, handling, forsøg, robot eller robotgruppe), kun fra cachen.
//! `fw`-rækken har alle otte felter, så enhver kombination af filtre er præcis. "Afvist" er som i overblikket (`er_afvist`,
//! J9), og perioderne er hele UTC-døgn (J1), så tallet er det samme før og efter klikket. Dyk læser under låsen (`laes`) og
//! regner bagefter (`regn`, ren funktion). `fw` og `probe` lægges aldrig sammen: om det er samme forespørgsel, kan ikke
//! afgøres. Stier står i renset form (J2); `[kategori]` fra Angreb virker også som filter. Robotter tælles fra `bot`, `falsk` og
//! `stoppet` (`robotter::passer`), som hverken har regel, land eller metode: de kan kun kombineres med sti, ligesom forsøg er for sig.
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::cf::Fejl;
use crate::overblik::{
    er_afvist, gruppe, median, regel_art, trafik_spring, Fordeling, TrafikDag, ANDET, BLOKERET, EGNE_KILDER, FAELDER, LABYRINT, RUTINE,
    SERVER_KODER, UDFORDRET,
};
use crate::robotter::{self, GRUPPER};
use crate::store::{Raekke, Store, FW_VAEGT};
use crate::teknologi;
use crate::vaern::{
    angreb_fra_raekker, dag_ms, dag_noegle, forsoeg_kategori, offentlig_sti, regel, regelkort, seneste_fra_raekker, tjek_zone, Bue,
    Kategori, KortSum, Regel, Seneste, StiN, ANGREB_MAKS, DOEGN_MS, FW_HAENDELSER, RUTINE_NAVNE,
};

/// Så mange rækker pr. top-liste sendes til visningen (de 5 første står fremme, resten bag "Vis top 20").
pub const TOP: usize = 20;
/// Så mange stier står under hver kategori i "Hvad de ledte efter".
const FORSOEG_STIER: usize = 6;
/// Så mange buer tegnes, som i Angreb.
const BUER: usize = 60;
/// Højst så mange tegn i en filterværdi (Cloudflares felter er kortere).
pub const MAKS_VAERDI: usize = 200;
/// `limit` i forespørgslen: et zone-døgn, der står på præcis så mange, er afskåret (de ældste timer mangler).
pub const LOFT: i64 = 10_000;
const ARTER: &[&str] = &["alle", "fw", "probe", "seneste", "bot", "falsk", "stoppet"];
/// "De seneste" viser så mange ad gangen.
pub const SIDE: usize = 25;
/// `forsoeg=alle`: alle stier med 404, som mærket "Ligner rekognoscering" åbner dem.
pub const ALLE_FORSOEG: &str = "alle";

/// Ét domæne, eller de valgte tilsammen.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Omfang {
    Zone(String),
    Zoner(Vec<String>),
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Dim {
    Regel,
    Land,
    Netvaerk,
    Sti,
    Colo,
    Metode,
    Handling,
    Forsoeg,
    Robot,
    #[serde(rename = "robotgruppe")]
    RobotGruppe,
    Teknologi,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Filter {
    pub dim: Dim,
    pub vaerdi: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Periode {
    D7,
    D30,
    Alt,
    Dag(String),
    Spaend { fra: String, til: String },
}

/// Ingen = alt (J4).
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Slags {
    Angreb,
    Rutine,
}

/// Et godkendt spørgsmål (`tjek`).
#[derive(Debug, Clone, PartialEq)]
pub struct Spoergsmaal {
    pub zoner: Vec<String>,
    pub filtre: Vec<Filter>,
    pub periode: Periode,
    pub slags: Option<Slags>,
    /// Side i "De seneste", fra 0.
    pub side: usize,
}

/// Afviser ugyldige zoner, datoer og filtre, før databasen spørges: højst én værdi pr. dimension, kendte handlinger og
/// grupper, og forsøg (art `probe`) og robotter kan ikke kombineres med filtre på `fw` (robotter dog med sti).
pub fn tjek(omfang: Omfang, filtre: Vec<Filter>, periode: Periode, slags: Option<Slags>) -> Result<Spoergsmaal, Fejl> {
    let zoner = match omfang {
        Omfang::Zone(z) => vec![tjek_zone(&z)?],
        Omfang::Zoner(zs) => zs.iter().map(|z| tjek_zone(z)).collect::<Result<_, _>>()?,
    };
    let mut set = HashSet::new();
    for f in &filtre {
        let tegn = f.vaerdi.chars().count();
        if tegn == 0 || tegn > MAKS_VAERDI || !set.insert(f.dim) {
            return Err(Fejl::UgyldigtFilter);
        }
        if f.dim == Dim::Handling && ![BLOKERET, UDFORDRET, LABYRINT, FAELDER, ANDET].contains(&f.vaerdi.as_str()) {
            return Err(Fejl::UgyldigtFilter);
        }
        if f.dim == Dim::RobotGruppe && !GRUPPER.contains(&f.vaerdi.as_str()) {
            return Err(Fejl::UgyldigtFilter);
        }
        if f.dim == Dim::Teknologi && !teknologi::kendt(&f.vaerdi) && f.vaerdi != teknologi::ANDET {
            return Err(Fejl::UgyldigtFilter);
        }
    }
    if set.contains(&Dim::Forsoeg) && set.len() > 1 {
        return Err(Fejl::UgyldigtFilter);
    }
    let robot = set.contains(&Dim::Robot) || set.contains(&Dim::RobotGruppe);
    if robot && set.iter().any(|d| !matches!(d, Dim::Robot | Dim::RobotGruppe | Dim::Sti)) {
        return Err(Fejl::UgyldigtFilter);
    }
    let dage: Vec<&String> = match &periode {
        Periode::Dag(d) => vec![d],
        Periode::Spaend { fra, til } => vec![fra, til],
        _ => Vec::new(),
    };
    let ms = dage
        .iter()
        .map(|d| dag_ms(d).ok_or(Fejl::UgyldigDato))
        .collect::<Result<Vec<_>, _>>()?;
    if ms.windows(2).any(|p| p[0] > p[1]) {
        return Err(Fejl::UgyldigDato);
    }
    Ok(Spoergsmaal {
        zoner,
        filtre,
        periode,
        slags,
        side: 0,
    })
}

/// Det, `regn` skal bruge fra databasen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Laest {
    /// Årstallene (art `h_total`) for omfanget, og de dage, der har dem (`Store::trafik_dage`).
    pub aar: Vec<(String, Raekke)>,
    pub trafik_dage: HashSet<String>,
    /// Teknologi-valgene pr. domæne i omfanget. Kun mærket i "Hvad angrebene var rettet mod".
    pub profiler: Vec<teknologi::Profil>,
    /// Omfanget over hele historikken (til første og sidst set).
    pub rows: Vec<(String, Raekke)>,
    /// Enkelthændelserne fra periodens start, for omfanget.
    pub angreb: Vec<(String, Raekke)>,
    /// De andre zoner, kun i perioden.
    pub andre: Vec<(String, Raekke)>,
    /// Dage, hvor mindst én zone i omfanget er hentet.
    pub hentet: HashSet<String>,
    pub navne: HashMap<String, String>,
    /// Antal hændelser pr. zone-døgn i perioden (art `vaern::FW_HAENDELSER`), til loftet. Døgn hentet før V-SAMPLE.1 har ingen.
    pub haendelser: Vec<(String, Raekke)>,
    /// Første dag, hvis `fw`-tal er Cloudflares estimat (`store::FW_VAEGT`). Ældre døgn er talte rækker.
    pub vaegt_fra: Option<String>,
}

/// Første og sidste dag i perioden. Hele historikken går fra første hentede dag til i går.
pub fn spaend(periode: &Periode, hentet: &HashSet<String>, nu: i64) -> (String, String) {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    match periode {
        Periode::D7 => (d(7), d(1)),
        Periode::D30 => (d(30), d(1)),
        Periode::Alt => {
            let i_gaar = d(1);
            let foerste = hentet.iter().min().filter(|f| **f <= i_gaar).cloned();
            (foerste.unwrap_or_else(|| i_gaar.clone()), i_gaar)
        }
        Periode::Dag(dag) => (dag.clone(), dag.clone()),
        Periode::Spaend { fra, til } => (fra.clone(), til.clone()),
    }
}

/// Læser det, der skal tælles. Omfanget læses over hele historikken, de andre zoner kun i perioden: så er ét domæne hurtigt
/// uanset alder.
pub fn laes(lager: &Store, s: &Spoergsmaal, nu: i64) -> Result<Laest, Fejl> {
    let navne: HashMap<String, String> = lager.zoner()?.0.into_iter().map(|z| (z.id, z.navn)).collect();
    let mut hentet = HashSet::new();
    for z in &s.zoner {
        hentet.extend(lager.hentet(z)?.into_keys());
    }
    let trafik_dage = lager.trafik_dage(&s.zoner)?;
    let (fra, _) = spaend(&s.periode, &hentet.union(&trafik_dage).cloned().collect(), nu);
    let andre: Vec<String> = lager.valg()?.into_iter().filter(|z| !s.zoner.contains(z)).collect();
    Ok(Laest {
        rows: lager.raekker_art(&s.zoner, ARTER, "0000-00-00")?,
        angreb: lager.raekker_art(&s.zoner, &["angreb"], &fra)?,
        andre: lager.raekker_art(&andre, ARTER, &fra)?,
        aar: lager.raekker_art(&s.zoner, &["h_total"], &fra)?,
        trafik_dage,
        profiler: s.zoner.iter().map(|z| teknologi::profil(lager, z)).collect::<Result<_, _>>()?,
        hentet,
        navne,
        haendelser: lager.raekker_art(&s.zoner, &[FW_HAENDELSER], &fra)?,
        vaegt_fra: lager.indstilling(FW_VAEGT)?,
    })
}

// ── Svaret ─────────────────────────────────────────────────────────────────────────────────────────────────────────
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PeriodeUd {
    pub fra: String,
    pub til: String,
    pub dage: i64,
    /// Dage i perioden med tal. Resten er huller: de er ikke hentet, og det er noget andet end 0.
    pub med: i64,
    /// Første og sidste dag med tal i omfanget, og i dag: grænserne i datovælgeren.
    pub data_fra: Option<String>,
    pub data_til: Option<String>,
    pub i_dag: String,
    /// Første dag med sikkerhedstal. Før den findes kun trafiktal (`trafik_fra`, årstallene).
    pub sikkerhed_fra: Option<String>,
    pub trafik_fra: Option<String>,
}

/// Perioden med dens huller og datovælgerens grænser. Ren funktion.
pub fn periode_ud(fra: &str, til: &str, hentet: &HashSet<String>, nu: i64) -> PeriodeUd {
    let dage = robotter::dage_i(fra, til);
    PeriodeUd {
        fra: fra.to_string(),
        til: til.to_string(),
        dage: dage.len() as i64,
        med: dage.iter().filter(|d| hentet.contains(*d)).count() as i64,
        data_fra: hentet.iter().min().cloned(),
        data_til: hentet.iter().max().cloned(),
        i_dag: dag_noegle(nu),
        sikkerhed_fra: hentet.iter().min().cloned(),
        trafik_fra: None,
    }
}

/// Altid samme tal uanset den valgte periode. `alt` er hele dage til og med i går.
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Noegletal {
    pub i_gaar: i64,
    pub i_dag_hidtil: i64,
    pub hele7: i64,
    pub hele30: i64,
    pub alt: i64,
    /// Hentede døgn af de 7 og 30 hele døgn, så en sum med huller kan sige det.
    pub med7: i64,
    pub med30: i64,
}

/// `hentet = false` er en dag uden tal, ikke en dag med 0.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DagUd {
    pub dag: String,
    pub n: i64,
    pub hentet: bool,
    /// Alle forespørgsler den dag, til tabellen under søjlerne.
    pub alle: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct VaerdiN {
    pub v: String,
    pub n: i64,
    /// Kun i `top.regel`: reglens art ud fra kilden (`regel_art`), så en egen regel kan stå med kort navn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art: Option<String>,
}

#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Top {
    pub regel: Vec<VaerdiN>,
    pub land: Vec<VaerdiN>,
    pub netvaerk: Vec<VaerdiN>,
    pub sti: Vec<VaerdiN>,
    pub colo: Vec<VaerdiN>,
    pub metode: Vec<VaerdiN>,
    pub handling: Vec<VaerdiN>,
    pub robot: Vec<VaerdiN>,
    pub status: Vec<VaerdiN>,
}

/// "Hvad de ledte efter" for perioden: alle forsøg og dem, der hører til en kategori.
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct ForsoegUd {
    pub n: i64,
    pub kategorier: Vec<Kategori>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct RegelHoved {
    pub navn: String,
    pub produkt: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub cve: Option<String>,
    pub art: String,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DomaeneN {
    pub zone: String,
    pub navn: String,
    pub n: i64,
}

/// Tallene bag vælgeren Alt / Kun angreb / Kun rutine, i perioden.
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct SlagsTal {
    pub angreb: i64,
    pub rutine: i64,
}

/// Forespørgslerne pr. dag i perioden: de daglige tal, hvor de findes, ellers årstallene. Ren funktion.
fn trafik_pr_dag<'a>(l: &'a Laest, fra: &str, til: &str) -> BTreeMap<&'a str, i64> {
    let i = |d: &str| d >= fra && d <= til;
    let mut pr: BTreeMap<(&str, &str), i64> = BTreeMap::new();
    for (z, r) in l.rows.iter().filter(|(_, r)| r.art == "alle" && i(&r.dag)) {
        *pr.entry((z, &r.dag)).or_default() += r.n;
    }
    for (z, r) in l.aar.iter().filter(|(_, r)| r.noegle == "foresp" && i(&r.dag)) {
        pr.entry((z, &r.dag)).or_insert(r.n);
    }
    let mut ud = BTreeMap::new();
    for ((_, dag), n) in pr {
        *ud.entry(dag).or_default() += n;
    }
    ud
}

/// Alle forespørgsler i perioden og dagene med usædvanlig meget trafik. For flere domæner er tallene summer, også "normalt": hvert domæne har sin egen median.
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct TrafikPeriode {
    pub i_alt: i64,
    pub dage: Vec<TrafikDag>,
}

/// Dagens tal for et udsnit uden hele døgn. `t`: tidspunktet, hvis det er én enkelt hændelse med et gemt tidspunkt.
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct KunIDag {
    pub stier: Vec<VaerdiN>,
    pub lande: Vec<VaerdiN>,
    pub t: Option<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DykUd {
    pub periode: PeriodeUd,
    /// "Hvad de ledte efter" for perioden. 404-svar har ingen land, regel eller slags, så filtrene gælder ikke her.
    pub forsoeg: ForsoegUd,
    /// "Hvad angrebene var rettet mod" i udsnittet.
    pub rettet_mod: Vec<RettetMod>,
    /// I går (d-1), til flisen "Afvist i går", og i dag, til "i dag hidtil".
    pub i_gaar: String,
    pub i_dag: String,
    pub sum: i64,
    pub noegletal: Noegletal,
    /// Over hele historikken, også uden for perioden.
    pub foerst: Option<String>,
    pub sidst: Option<String>,
    pub dage: Vec<DagUd>,
    pub fordeling: Fordeling,
    pub top: Top,
    /// Forskellige værdier pr. liste ("Vis top 20 (N)").
    pub antal: BTreeMap<&'static str, usize>,
    pub buer: Vec<Bue>,
    pub buer_i_alt: usize,
    pub regel: Option<RegelHoved>,
    pub domaener: Vec<DomaeneN>,
    pub enkelte: Vec<Seneste>,
    /// Sat, når `enkelte` er de gemte angreb. Ellers er det de 12 seneste af alle slags.
    pub enkelte_info: Option<EnkelteInfo>,
    pub slags: SlagsTal,
    /// Et zone-døgn i perioden står på loftet: Cloudflare sendte `LOFT` hændelser, så de ældste timer mangler, og tallene er mindst det, der står.
    pub loft: bool,
    /// Perioden krydser `Laest::vaegt_fra`: før dagen er `fw`-tallene talte rækker, fra den Cloudflares estimat. Kun for afviste (ikke forsøg og robotter).
    pub vaegt_fra: Option<String>,
    /// Forespørgslerne i perioden, så angrebene kan holdes op imod dem.
    pub trafik: TrafikPeriode,
    /// Forespørgslerne dag for dag, når perioden går længere tilbage end sikkerhedstallene. Ellers tom.
    pub trafik_serie: Vec<DagUd>,
    /// Det, der er set i dag, når udsnittet kun har tal i dag.
    pub kun_i_dag: Option<KunIDag>,
    /// Regelkortene for udsnittet: `h30` er tallet i perioden, og stier, lande og datoer følger udsnittet. Tom i et regel-dyk, hvor
    /// reglen allerede er hovedet, og for forsøg og robotter, som ingen regel har.
    pub regler: Vec<Regel>,
}

// ── Optællingen ────────────────────────────────────────────────────────────────────────────────────────────────────
fn vaerdi(s: &Spoergsmaal, dim: Dim) -> Option<&str> {
    s.filtre.iter().find(|f| f.dim == dim).map(|f| f.vaerdi.as_str())
}

/// Sti i renset form, eller `[kategori]` som i Angreb.
fn sti_passer(sti: &str, v: &str) -> bool {
    sti == v || (v.starts_with('[') && offentlig_sti(sti) == v)
}

/// En `fw`-række (otte felter) mod filtrene.
fn fw_passer(s: &Spoergsmaal, f: &[&str]) -> bool {
    let felt = |i: usize| f.get(i).copied().unwrap_or("");
    s.filtre.iter().all(|x| match x.dim {
        Dim::Regel => felt(0) == x.vaerdi,
        Dim::Land => felt(1) == x.vaerdi,
        Dim::Netvaerk => felt(2) == x.vaerdi,
        Dim::Sti => sti_passer(felt(3), &x.vaerdi),
        Dim::Colo => felt(4) == x.vaerdi,
        Dim::Metode => felt(5) == x.vaerdi,
        Dim::Handling => gruppe(felt(7)) == x.vaerdi,
        Dim::Forsoeg | Dim::Robot | Dim::RobotGruppe => false,
        Dim::Teknologi => rettet_mod(felt(0), felt(3), felt(6)).is_some_and(|t| t == x.vaerdi),
    })
}

/// Linjen i "Hvad angrebene var rettet mod": teknologien, ellers "Andet". Rutine og egne regler er ikke rettet mod noget produkt.
fn rettet_mod(regel: &str, sti: &str, kilde: &str) -> Option<String> {
    (!RUTINE.contains(&kilde) && !EGNE_KILDER.contains(&kilde))
        .then(|| teknologi::af_fw(regel, sti, kilde).unwrap_or_else(|| teknologi::ANDET.to_string()))
}

/// Samme linje for en gemt enkelthændelse. Rutine og egne regler er ikke gemt som angreb.
fn tek_i_angreb(regel: &str, sti: &str) -> String {
    teknologi::af_regel(regel)
        .or_else(|| teknologi::af_sti(sti).map(str::to_string))
        .unwrap_or_else(|| teknologi::ANDET.to_string())
}

fn slags_passer(s: &Spoergsmaal, rutine: bool) -> bool {
    s.slags.is_none_or(|k| (k == Slags::Rutine) == rutine)
}

/// En `fw`-række, der tæller: afvist, den handling, der er filtreret på (fx siderne med fælde-links), eller under "Kun angreb"
/// alt uden for rutine, som statusreglen tæller det (`overblik::angreb`), også fx `log` fra egne regler.
fn taeller_med(s: &Spoergsmaal, handling: &str, rutine: bool) -> bool {
    vaerdi(s, Dim::Handling).is_some() || er_afvist(handling) || (!rutine && s.slags == Some(Slags::Angreb))
}

/// En `probe`-række: stien, dens kategori eller alle.
fn probe_passer(sti: &str, v: &str) -> bool {
    v == ALLE_FORSOEG || sti == v || forsoeg_kategori(sti) == Some(v) || offentlig_sti(sti) == v
}

/// En robotrække, der passer til filtrene (`tjek` har sikret, at kun `robot`, `robotgruppe` og `sti` kan stå sammen).
fn robot_linje<'a>(s: &Spoergsmaal, r: &'a Raekke) -> Option<robotter::Linje<'a>> {
    let x = robotter::passer(r, vaerdi(s, Dim::Robot), vaerdi(s, Dim::RobotGruppe))?;
    vaerdi(s, Dim::Sti).is_none_or(|v| sti_passer(x.sti, v)).then_some(x)
}

/// De `TOP` største, størst først; lige tal efter navn.
pub fn top_af(t: &HashMap<&str, i64>) -> Vec<VaerdiN> {
    top(t)
}

fn top(t: &HashMap<&str, i64>) -> Vec<VaerdiN> {
    let mut v: Vec<(&str, i64)> = t.iter().map(|(k, n)| (*k, *n)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    v.truncate(TOP);
    v.into_iter()
        .map(|(v, n)| VaerdiN {
            v: v.to_string(),
            n,
            art: None,
        })
        .collect()
}

/// De gemte enkelthændelser med samme filtre. Netværk og handling står ikke i dem, så med de filtre er listen tom.
fn enkelte(s: &Spoergsmaal, l: &Laest, fra: &str, til: &str) -> Vec<Seneste> {
    let passer = |r: &Raekke| {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        s.filtre.iter().all(|x| match x.dim {
            Dim::Regel => felt(1) == x.vaerdi,
            Dim::Land => felt(3) == x.vaerdi,
            Dim::Sti => felt(4) == x.vaerdi || felt(4) == offentlig_sti(&x.vaerdi),
            Dim::Colo => felt(5) == x.vaerdi,
            Dim::Metode => felt(6) == x.vaerdi,
            Dim::Teknologi => tek_i_angreb(felt(1), felt(4)) == x.vaerdi,
            _ => false,
        }) && slags_passer(s, RUTINE_NAVNE.contains(&felt(1)))
    };
    let mut pr_zone: BTreeMap<&str, Vec<Raekke>> = BTreeMap::new();
    for (z, r) in &l.rows {
        if r.art == "seneste" && r.dag.as_str() >= fra && r.dag.as_str() <= til && passer(r) {
            pr_zone.entry(z).or_default().push(r.clone());
        }
    }
    let flere = s.zoner.len() > 1;
    let mut ud: Vec<Seneste> = Vec::new();
    for (z, rk) in pr_zone {
        let domaene = flere.then(|| l.navne.get(z).cloned().unwrap_or_else(|| "…".into()));
        ud.extend(seneste_fra_raekker(&rk).into_iter().map(|e| Seneste {
            domaene: domaene.clone(),
            ..e
        }));
    }
    ud.sort_by(|a, b| b.t.cmp(&a.t));
    ud.truncate(12);
    ud
}

/// "Viser N af M angreb": `vist` er de gemte hændelser, der passer, og `loft`, at et døgn har mere, end der gemmes.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct EnkelteInfo {
    pub vist: i64,
    pub i_alt: i64,
    pub side: usize,
    pub sider: usize,
    pub loft: bool,
}

/// En `angreb`-række mod filtrene. En række fra før skema 12 har ikke netværket og passer ikke til det filter.
fn angreb_passer(s: &Spoergsmaal, f: &[&str]) -> bool {
    let felt = |i: usize| f.get(i).copied().unwrap_or("");
    s.filtre.iter().all(|x| match x.dim {
        Dim::Regel => felt(1) == x.vaerdi,
        Dim::Land => felt(3) == x.vaerdi,
        Dim::Sti => felt(4) == x.vaerdi || felt(4) == offentlig_sti(&x.vaerdi),
        Dim::Colo => felt(5) == x.vaerdi,
        Dim::Metode => felt(6) == x.vaerdi,
        Dim::Handling => gruppe(felt(8)) == x.vaerdi,
        Dim::Netvaerk => felt(9) == x.vaerdi,
        Dim::Teknologi => tek_i_angreb(felt(1), felt(4)) == x.vaerdi,
        _ => false,
    })
}

/// De gemte angreb i udsnittet, side for side. `None`: udsnittet er rutine, eller angrebene er ikke hentet endnu, og så viser "De seneste" det gamle. Ren funktion.
fn enkelte_angreb(s: &Spoergsmaal, l: &Laest, fra: &str, til: &str, i_alt: i64, loft: bool) -> Option<(Vec<Seneste>, EnkelteInfo)> {
    if s.slags == Some(Slags::Rutine) || vaerdi(s, Dim::Regel).is_some_and(|r| RUTINE_NAVNE.contains(&r)) {
        return None;
    }
    let flere = s.zoner.len() > 1;
    let mut ud: Vec<Seneste> = Vec::new();
    for (z, r) in l.angreb.iter().filter(|(_, r)| r.dag.as_str() >= fra && r.dag.as_str() <= til) {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        if angreb_passer(s, &f) {
            let domaene = flere.then(|| l.navne.get(z).cloned().unwrap_or_else(|| "…".into()));
            ud.extend(angreb_fra_raekker(std::slice::from_ref(r)).into_iter().map(|e| Seneste {
                domaene: domaene.clone(),
                ..e
            }));
        }
    }
    if ud.is_empty() {
        return None;
    }
    ud.sort_by(|a, b| b.t.cmp(&a.t).then_with(|| a.regel.cmp(&b.regel)));
    let vist = ud.iter().map(|e| e.n.unwrap_or(1)).sum();
    let sider = ud.len().div_ceil(SIDE);
    let side = s.side.min(sider - 1);
    let info = EnkelteInfo {
        vist,
        i_alt,
        side: side + 1,
        sider,
        loft,
    };
    Some((ud.into_iter().skip(side * SIDE).take(SIDE).collect(), info))
}

/// Forespørgslerne i perioden og de dage, trafikken sprang. `trafik_spring` regnes pr. domæne: medianen er domænets egen. Ren funktion.
fn trafik_i(l: &Laest, fra: &str, til: &str, nu: i64) -> TrafikPeriode {
    let mut pr_zone: BTreeMap<&str, Vec<Raekke>> = BTreeMap::new();
    let i_alt = trafik_pr_dag(l, fra, til).values().sum();
    for (z, r) in l.rows.iter().filter(|(_, r)| r.art == "alle" || r.art == "fw") {
        pr_zone.entry(z).or_default().push(r.clone());
    }
    let mut dage: BTreeMap<String, TrafikDag> = BTreeMap::new();
    for rows in pr_zone.values() {
        for t in trafik_spring(rows, nu)
            .into_iter()
            .filter(|t| t.dag.as_str() >= fra && t.dag.as_str() <= til)
        {
            dage.entry(t.dag.clone())
                .and_modify(|x| {
                    x.n += t.n;
                    x.median += t.median;
                    x.stoppet += t.stoppet;
                })
                .or_insert(t);
        }
    }
    TrafikPeriode {
        i_alt,
        dage: dage.into_values().collect(),
    }
}

/// En linje i "Hvad angrebene var rettet mod". `valg`: valget på omfangets domæner, `blandet`, når de er forskellige. Hverken "Bruger ikke" eller en skjult kolonne fjerner en linje.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct RettetMod {
    pub tek: String,
    pub n: i64,
    pub valg: &'static str,
}

/// De mest angrebne først. Ren funktion.
fn rettet_mod_liste(t: HashMap<String, i64>, profiler: &[teknologi::Profil]) -> Vec<RettetMod> {
    let valg = |tek: &str| {
        let mut v = profiler.iter().map(|p| p.get(tek).copied().unwrap_or_default().valg.kode());
        let foerste = v.next().unwrap_or("ved_ikke");
        if v.all(|x| x == foerste) {
            foerste
        } else {
            "blandet"
        }
    };
    let mut ud: Vec<RettetMod> = t.into_iter().map(|(tek, n)| RettetMod { valg: valg(&tek), tek, n }).collect();
    ud.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.tek.cmp(&b.tek)));
    ud
}

/// Svaret ud fra det læste. Ren funktion.
/// Stierne med 404 i perioden sorteret i kategorier, de største først. Stierne står renset, og ens stier efter rensningen lægges sammen.
fn forsoeg_ud(pr_sti: &HashMap<&str, i64>) -> ForsoegUd {
    let mut kat: HashMap<&'static str, (i64, HashMap<String, i64>)> = HashMap::new();
    for (sti, n) in pr_sti {
        let Some(k) = forsoeg_kategori(sti) else { continue };
        let x = kat.entry(k).or_default();
        x.0 += n;
        *x.1.entry(offentlig_sti(sti)).or_default() += n;
    }
    let mut kategorier: Vec<Kategori> = kat
        .into_iter()
        .map(|(id, (n, stier))| {
            let mut stier: Vec<StiN> = stier.into_iter().map(|(sti, n)| StiN { sti, n }).collect();
            stier.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.sti.cmp(&b.sti)));
            stier.truncate(FORSOEG_STIER);
            Kategori { id: id.into(), n, stier }
        })
        .collect();
    kategorier.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.id.cmp(&b.id)));
    ForsoegUd {
        n: pr_sti.values().sum(),
        kategorier,
    }
}

pub fn regn(s: &Spoergsmaal, l: &Laest, nu: i64) -> DykUd {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    let (d0, d1, d7, d30) = (d(0), d(1), d(7), d(30));
    let alle_dage: HashSet<String> = l.hentet.union(&l.trafik_dage).cloned().collect();
    let (fra, til) = spaend(&s.periode, &alle_dage, nu);
    let i_perioden = |dag: &str| dag >= fra.as_str() && dag <= til.as_str();
    let forsoeg = vaerdi(s, Dim::Forsoeg);
    let robot = vaerdi(s, Dim::Robot).is_some() || vaerdi(s, Dim::RobotGruppe).is_some();

    let (mut sum, mut noegletal, mut fordeling, mut slags) = (0, Noegletal::default(), Fordeling::default(), SlagsTal::default());
    let (mut foerst, mut sidst): (Option<&str>, Option<&str>) = (None, None);
    let mut per_dag: HashMap<&str, i64> = HashMap::new();
    let (mut t_robot, mut t_status) = (HashMap::new(), HashMap::new());
    let (mut d_sti, mut d_land): (HashMap<&str, i64>, HashMap<&str, i64>) = (HashMap::new(), HashMap::new());
    let (mut t_regel, mut t_land, mut t_net, mut t_sti, mut t_colo, mut t_metode, mut t_handling) = (
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    );
    let mut par: HashMap<(&str, &str), i64> = HashMap::new();
    let mut t_probe: HashMap<&str, i64> = HashMap::new();
    let mut t_tek: HashMap<String, i64> = HashMap::new();
    let mut zone_dag: HashMap<(&str, &str), i64> = HashMap::new();
    let mut angreb_dag: HashMap<(&str, &str), i64> = HashMap::new();
    let mut kilde_regel: Option<&str> = None;
    let mut kilder: HashMap<&str, &str> = HashMap::new();
    let (mut kort, mut kort_plads): (Vec<KortSum>, HashMap<&str, usize>) = (Vec::new(), HashMap::new());

    let mut tael = |dag: &'_ str, n: i64| {
        if dag == d0 {
            noegletal.i_dag_hidtil += n;
        } else if dag < d0.as_str() {
            noegletal.alt += n;
        }
        if dag == d1 {
            noegletal.i_gaar += n;
        }
        if dag >= d7.as_str() && dag <= d1.as_str() {
            noegletal.hele7 += n;
        }
        if dag >= d30.as_str() && dag <= d1.as_str() {
            noegletal.hele30 += n;
        }
    };
    for (z, r) in &l.rows {
        let dag = r.dag.as_str();
        let n = r.n;
        if r.art == "probe" && i_perioden(dag) {
            *t_probe.entry(r.noegle.as_str()).or_default() += n;
        }
        if robot {
            let Some(x) = robot_linje(s, r) else { continue };
            tael(dag, n);
            if dag == d0 && !x.sti.is_empty() {
                *d_sti.entry(x.sti).or_default() += n;
            }
            foerst = Some(foerst.map_or(dag, |y| y.min(dag)));
            sidst = Some(sidst.map_or(dag, |y| y.max(dag)));
            if i_perioden(dag) {
                sum += n;
                *per_dag.entry(dag).or_default() += n;
                *t_robot.entry(x.navn).or_default() += n;
                if !x.sti.is_empty() {
                    *t_sti.entry(x.sti).or_default() += n;
                }
                if !x.status.is_empty() {
                    *t_status.entry(x.status).or_default() += n;
                }
            }
            continue;
        }
        if let Some(v) = forsoeg {
            if r.art != "probe" || !probe_passer(&r.noegle, v) {
                continue;
            }
            tael(dag, n);
            if dag == d0 {
                *d_sti.entry(r.noegle.as_str()).or_default() += n;
            }
            foerst = Some(foerst.map_or(dag, |x| x.min(dag)));
            sidst = Some(sidst.map_or(dag, |x| x.max(dag)));
            if i_perioden(dag) {
                sum += n;
                *per_dag.entry(dag).or_default() += n;
                *t_sti.entry(r.noegle.as_str()).or_default() += n;
            }
            continue;
        }
        if r.art != "fw" {
            continue;
        }
        if i_perioden(dag) {
            *zone_dag.entry((z.as_str(), dag)).or_default() += n;
        }
        let f: Vec<&str> = r.noegle.split('\t').collect();
        if i_perioden(dag) && !RUTINE.contains(&f.get(6).copied().unwrap_or("")) {
            *angreb_dag.entry((z.as_str(), dag)).or_default() += n;
        }
        if !fw_passer(s, &f) {
            continue;
        }
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        let (kilde, handling) = (felt(6), felt(7));
        kilde_regel.get_or_insert(kilde);
        let rutine = RUTINE.contains(&kilde);
        let med = taeller_med(s, handling, rutine);
        if i_perioden(dag) {
            if !rutine {
                slags.angreb += n;
            } else if med {
                slags.rutine += n;
            }
        }
        if !slags_passer(s, rutine) {
            continue;
        }
        if i_perioden(dag) {
            fordeling.tael(handling, n);
        }
        if !med {
            continue;
        }
        tael(dag, n);
        if dag == d0 {
            *d_sti.entry(felt(3)).or_default() += n;
            *d_land.entry(felt(1)).or_default() += n;
        }
        foerst = Some(foerst.map_or(dag, |x| x.min(dag)));
        sidst = Some(sidst.map_or(dag, |x| x.max(dag)));
        if !i_perioden(dag) {
            continue;
        }
        sum += n;
        *per_dag.entry(dag).or_default() += n;
        if let Some(t) = rettet_mod(felt(0), felt(3), kilde) {
            *t_tek.entry(t).or_default() += n;
        }
        *t_regel.entry(felt(0)).or_default() += n;
        kilder.insert(felt(0), kilde);
        let i = *kort_plads.entry(felt(0)).or_insert_with(|| {
            kort.push(KortSum {
                navn: felt(0).to_string(),
                kilde: kilde.to_string(),
                ..KortSum::default()
            });
            kort.len() - 1
        });
        let k = &mut kort[i];
        k.h30 += n;
        k.stier.inc(felt(3), n);
        k.lande.inc(felt(1), n);
        if k.foerst.is_empty() || dag < k.foerst.as_str() {
            k.foerst = dag.to_string();
        }
        if dag > k.sidst.as_str() {
            k.sidst = dag.to_string();
        }
        *t_land.entry(felt(1)).or_default() += n;
        *t_sti.entry(felt(3)).or_default() += n;
        *t_handling.entry(gruppe(handling)).or_default() += n;
        if !felt(2).is_empty() {
            *t_net.entry(felt(2)).or_default() += n;
        }
        if !felt(4).is_empty() {
            *t_colo.entry(felt(4)).or_default() += n;
            *par.entry((felt(1), felt(4))).or_default() += n;
        }
        if !felt(5).is_empty() {
            *t_metode.entry(felt(5)).or_default() += n;
        }
    }

    // De andre domæner: samme filtre i perioden.
    let mut andre: HashMap<&str, i64> = HashMap::new();
    for (z, r) in l.andre.iter().filter(|(_, r)| i_perioden(&r.dag)) {
        let passer = match forsoeg {
            _ if robot => robot_linje(s, r).is_some(),
            Some(v) => r.art == "probe" && probe_passer(&r.noegle, v),
            None if r.art == "fw" => {
                let f: Vec<&str> = r.noegle.split('\t').collect();
                let felt = |i: usize| f.get(i).copied().unwrap_or("");
                let rutine = RUTINE.contains(&felt(6));
                fw_passer(s, &f) && taeller_med(s, felt(7), rutine) && slags_passer(s, rutine)
            }
            None => false,
        };
        if passer {
            *andre.entry(z.as_str()).or_default() += r.n;
        }
    }
    let mut domaener: Vec<DomaeneN> = andre
        .into_iter()
        .filter_map(|(z, n)| {
            l.navne.get(z).map(|navn| DomaeneN {
                zone: z.to_string(),
                navn: navn.clone(),
                n,
            })
        })
        .collect();
    domaener.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.navn.cmp(&b.navn)));

    let (fra_ms, til_ms) = (dag_ms(&fra).unwrap_or(0), dag_ms(&til).unwrap_or(0));
    let alle_pr_dag = trafik_pr_dag(l, &fra, &til);
    let dage: Vec<DagUd> = (0..=(til_ms - fra_ms) / DOEGN_MS)
        .map(|i| {
            let dag = dag_noegle(fra_ms + i * DOEGN_MS);
            DagUd {
                n: per_dag.get(dag.as_str()).copied().unwrap_or(0),
                hentet: l.hentet.contains(&dag),
                alle: alle_pr_dag.get(dag.as_str()).copied().unwrap_or(0),
                dag,
            }
        })
        .collect();

    let mut buer: Vec<Bue> = par
        .into_iter()
        .map(|((land, colo), n)| Bue {
            land: land.into(),
            colo: colo.into(),
            n,
        })
        .collect();
    buer.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| (&a.land, &a.colo).cmp(&(&b.land, &b.colo))));
    let buer_i_alt = buer.len();
    buer.truncate(BUER);

    let regel_hoved = vaerdi(s, Dim::Regel).map(|navn| {
        let bic = navn == "Browser Integrity Check";
        let r = regel(if bic { "" } else { navn }, if bic { "bic" } else { "" });
        let art = match kilde_regel {
            Some(k) => regel_art(k),
            None if RUTINE_NAVNE.contains(&navn) => "rutine",
            None => "angreb",
        };
        RegelHoved {
            navn: navn.to_string(),
            produkt: teknologi::vist_produkt(&r.produkt),
            type_: r.type_,
            cve: r.cve,
            art: art.to_string(),
        }
    });
    let mut top_regel = top(&t_regel);
    for x in &mut top_regel {
        x.art = kilder.get(x.v.as_str()).map(|k| regel_art(k).to_string());
    }
    let antal = BTreeMap::from([
        ("regel", t_regel.len()),
        ("land", t_land.len()),
        ("netvaerk", t_net.len()),
        ("sti", t_sti.len()),
        ("colo", t_colo.len()),
        ("metode", t_metode.len()),
        ("handling", t_handling.len()),
        ("robot", t_robot.len()),
        ("status", t_status.len()),
    ]);
    let med = |a: &str| robotter::dage_i(a, &d1).iter().filter(|d| l.hentet.contains(*d)).count() as i64;
    noegletal.med7 = med(&d7);
    noegletal.med30 = med(&d30);
    let kun_i_dag = (noegletal.alt == 0 && noegletal.i_dag_hidtil > 0).then(|| KunIDag {
        stier: top(&d_sti),
        lande: top(&d_land),
        t: (noegletal.i_dag_hidtil == 1)
            .then(|| enkelte(s, l, &d0, &d0).first().and_then(|e| e.t.clone()))
            .flatten(),
    });
    let loft_dag = angreb_dag.values().any(|n| *n > ANGREB_MAKS as i64);
    let (liste, enkelte_info) = match enkelte_angreb(s, l, &fra, &til, slags.angreb, loft_dag) {
        Some((v, info)) => (v, Some(info)),
        None => (enkelte(s, l, &fra, &til), None),
    };
    DykUd {
        forsoeg: forsoeg_ud(&t_probe),
        periode: PeriodeUd {
            data_fra: alle_dage.iter().min().cloned(),
            trafik_fra: l.trafik_dage.iter().min().cloned(),
            ..periode_ud(&fra, &til, &l.hentet, nu)
        },
        rettet_mod: rettet_mod_liste(t_tek, &l.profiler),
        trafik_serie: l
            .hentet
            .iter()
            .min()
            .filter(|f| fra < **f)
            .map(|_| {
                let pr_dag = trafik_pr_dag(l, &fra, &til);
                robotter::dage_i(&fra, &til)
                    .into_iter()
                    .map(|dag| {
                        let n = pr_dag.get(dag.as_str()).copied().unwrap_or(0);
                        DagUd {
                            n,
                            hentet: pr_dag.contains_key(dag.as_str()),
                            alle: n,
                            dag,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
        i_gaar: d1.clone(),
        i_dag: d0.clone(),
        sum,
        noegletal,
        foerst: foerst.map(str::to_string),
        sidst: sidst.map(str::to_string),
        dage,
        fordeling,
        top: Top {
            regel: top_regel,
            land: top(&t_land),
            netvaerk: top(&t_net),
            sti: top(&t_sti),
            colo: top(&t_colo),
            metode: top(&t_metode),
            handling: top(&t_handling),
            robot: top(&t_robot),
            status: top(&t_status),
        },
        antal,
        buer,
        buer_i_alt,
        regel: regel_hoved,
        domaener,
        enkelte: liste,
        enkelte_info,
        slags,
        loft: paa_loftet(&zone_dag, l, &fra, &til),
        vaegt_fra: if robot || forsoeg.is_some() {
            None
        } else {
            vaegt_graense(l, &fra, &til)
        },
        trafik: trafik_i(l, &fra, &til, nu),
        kun_i_dag,
        regler: if vaerdi(s, Dim::Regel).is_some() {
            Vec::new()
        } else {
            regelkort(kort)
        },
    }
}

/// Står et zone-døgn i perioden på loftet? Med antal hændelser (`FW_HAENDELSER`) afgør det; et døgn uden (hentet før V-SAMPLE.1, `fw`
/// er talte rækker) afgøres af `fw`-summen som før. `fw` alene kan ikke bruges på nye døgn: estimatet passerer 10.000 længe før svaret.
fn paa_loftet(zone_dag: &HashMap<(&str, &str), i64>, l: &Laest, fra: &str, til: &str) -> bool {
    let haendelser: HashMap<(&str, &str), i64> = l
        .haendelser
        .iter()
        .filter(|(_, r)| r.art == FW_HAENDELSER && r.dag.as_str() >= fra && r.dag.as_str() <= til)
        .map(|(z, r)| ((z.as_str(), r.dag.as_str()), r.n))
        .collect();
    haendelser.values().any(|n| *n >= LOFT) || zone_dag.iter().any(|(k, n)| !haendelser.contains_key(k) && *n >= LOFT)
}

/// Grænsen mellem talte rækker og estimater, hvis perioden krydser den: dagen ligger i perioden, og mindst én hentet dag i perioden
/// ligger før den. Uden gamle døgn (ny installation, eller efter Nulstil historik) er der ingen grænse at vise.
fn vaegt_graense(l: &Laest, fra: &str, til: &str) -> Option<String> {
    let g = l.vaegt_fra.as_deref()?;
    (fra < g && g <= til && l.hentet.iter().any(|d| d.as_str() >= fra && d.as_str() < g)).then(|| g.to_string())
}

// ── Trafik-dyk ─────────────────────────────────────────────────────────────────────────────────────────────────────
/// Et land i trafik-dyk. `angreb`: det, Cloudflare stoppede derfra den dag. Kun dem har noget at åbne.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TrafikLand {
    pub v: String,
    pub n: i64,
    pub angreb: i64,
}

/// Hvor en dags trafik kom fra, set fra forespørgslerne og ikke fra angrebene. Lande og svarkoder er de ekstra tal (`adgang::ADGANG_QUERY`, de 20 største lande og 50 største
/// svarkoder), og robotterne er de verificerede (`bot`).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TrafikUd {
    pub dag: String,
    /// Sidste dag: samme som `dag`, medmindre det er en periode.
    pub til: String,
    pub i_alt: i64,
    /// Domænets median for de 30 døgn før; `null` for flere domæner, for medianen er det enkelte domænes.
    pub median: Option<f64>,
    pub stoppet: i64,
    pub lande: Vec<TrafikLand>,
    pub lande_i_alt: i64,
    pub status: Vec<VaerdiN>,
    pub status_i_alt: i64,
    /// Svar 520–527: Cloudflare kunne ikke få svar fra serveren.
    pub server_fejl: i64,
    pub robotter: Vec<VaerdiN>,
    /// De ekstra tal er hentet for dagen.
    pub ekstra: bool,
    /// Perioden har daglige tal: `stoppet` siger noget. Ellers er det kun årstallene, med Cloudflares `trusler` og `cachet`.
    pub sikkerhed: bool,
    pub trusler: i64,
    pub cachet: i64,
}

const TRAFIK_ARTER: &[&str] = &["alle", "fw", "land", "status", "bot", "h_total", "h_land", "h_status"];

/// Rækkerne fra de 30 døgn før den første dag og frem, for omfanget.
pub fn trafik_laes(lager: &Store, s: &Spoergsmaal, dag: &str, til: &str) -> Result<(Vec<Raekke>, bool), Fejl> {
    let fra = dag_ms(dag).map_or_else(String::new, |ms| dag_noegle(ms - 30 * DOEGN_MS));
    let rows = lager
        .raekker_art(&s.zoner, TRAFIK_ARTER, &fra)?
        .into_iter()
        .map(|(_, r)| r)
        .filter(|r| r.dag.as_str() <= til)
        .collect();
    let mut ekstra = false;
    for z in &s.zoner {
        ekstra |= lager.hentet_ekstra(z)?.keys().any(|d| d.as_str() >= dag && d.as_str() <= til);
        ekstra |= lager.hentet_historik(z)?.keys().any(|d| d.as_str() >= dag && d.as_str() <= til);
    }
    Ok((rows, ekstra))
}

/// Svaret for dagen eller perioden `dag`…`til`. Ren funktion.
pub fn trafik_regn(rows: &[Raekke], dag: &str, til: &str, en_zone: bool, ekstra: bool) -> TrafikUd {
    let (mut lande, mut status) = (HashMap::<&str, i64>::new(), HashMap::<&str, i64>::new());
    let (mut bots, mut fw_land) = (HashMap::<&str, i64>::new(), HashMap::<&str, i64>::new());
    let (mut i_alt, mut stoppet, mut server_fejl) = (0, 0, 0);
    let (mut trusler, mut cachet) = (0, 0);
    // Årstallene bruges kun for døgn uden de daglige tal, ældre end sikkerhedstallene.
    let har = |art: &str, d: &str| rows.iter().any(|r| r.art == art && r.dag == d);
    let mut sikkerhed = false;
    for r in rows.iter().filter(|r| r.dag.as_str() >= dag && r.dag.as_str() <= til) {
        match r.art.as_str() {
            "alle" => {
                i_alt += r.n;
                sikkerhed = true;
            }
            "h_total" if r.noegle == "trusler" => trusler += r.n,
            "h_total" if r.noegle == "cachet" => cachet += r.n,
            "h_total" if r.noegle == "foresp" && !har("alle", &r.dag) => i_alt += r.n,
            "h_land" if !har("land", &r.dag) => *lande.entry(&r.noegle).or_default() += r.n,
            "h_status" if !har("status", &r.dag) => {
                *status.entry(&r.noegle).or_default() += r.n;
                if r.noegle.parse().is_ok_and(|k: i64| SERVER_KODER.contains(&k)) {
                    server_fejl += r.n;
                }
            }
            "land" => *lande.entry(&r.noegle).or_default() += r.n,
            "status" => {
                *status.entry(&r.noegle).or_default() += r.n;
                if r.noegle.parse().is_ok_and(|k: i64| SERVER_KODER.contains(&k)) {
                    server_fejl += r.n;
                }
            }
            "bot" => {
                if let Some(x) = robotter::passer(r, None, None) {
                    *bots.entry(x.navn).or_default() += r.n;
                }
            }
            "fw" => {
                let f: Vec<&str> = r.noegle.split('\t').collect();
                if er_afvist(f.get(7).copied().unwrap_or("")) {
                    stoppet += r.n;
                    *fw_land.entry(f.get(1).copied().unwrap_or("")).or_default() += r.n;
                }
            }
            _ => {}
        }
    }
    let mut foer: Vec<i64> = rows
        .iter()
        .filter(|r| r.art == "alle" && r.dag.as_str() < dag)
        .map(|r| r.n)
        .collect();
    let lande_i_alt = lande.values().sum();
    let status_i_alt = status.values().sum();
    TrafikUd {
        dag: dag.to_string(),
        til: til.to_string(),
        i_alt,
        median: (en_zone && !foer.is_empty()).then(|| median(&mut foer)),
        stoppet,
        lande: top(&lande)
            .into_iter()
            .map(|x| TrafikLand {
                angreb: fw_land.get(x.v.as_str()).copied().unwrap_or(0),
                v: x.v,
                n: x.n,
            })
            .collect(),
        lande_i_alt,
        status: top(&status),
        status_i_alt,
        server_fejl,
        robotter: top(&bots),
        ekstra,
        sikkerhed,
        trusler,
        cachet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overblik::hele_doegn;
    use crate::vaern::iso;

    fn laest_b3(rows: Vec<(&str, Raekke)>, dage: &[String]) -> Laest {
        Laest {
            rows: rows.into_iter().map(|(z, r)| (z.to_string(), r)).collect(),
            hentet: dage.iter().cloned().collect(),
            ..Laest::default()
        }
    }

    fn raekke_b3(dag: String, art: &str, noegle: &str, n: i64) -> Raekke {
        Raekke {
            dag,
            art: art.into(),
            noegle: noegle.into(),
            n,
        }
    }

    /// "Hvad de ledte efter" følger perioden, står uden for filtrene og har rensede stier.
    #[test]
    fn forsoeg_foelger_perioden_og_ikke_filtrene() {
        let dage = vec![
            dag_noegle(NU - DOEGN_MS),
            dag_noegle(NU - 2 * DOEGN_MS),
            dag_noegle(NU - 20 * DOEGN_MS),
        ];
        let rows = vec![
            ("z", raekke_b3(dage[0].clone(), "probe", "/.env", 5)),
            ("z", raekke_b3(dage[1].clone(), "probe", "/wp-login.php", 7)),
            ("z", raekke_b3(dage[1].clone(), "probe", "/favicon.ico", 2)),
            ("z", raekke_b3(dage[2].clone(), "probe", "/.env", 100)),
        ];
        let l = laest_b3(rows, &dage);
        let syv = regn(&spoerg_b3(&["z"], Vec::new(), Periode::D7), &l, NU);
        assert_eq!(syv.forsoeg.n, 14);
        assert_eq!(
            syv.forsoeg.kategorier.iter().map(|k| (k.id.as_str(), k.n)).collect::<Vec<_>>(),
            [("wordpress", 7), ("hemmeligheder", 5)]
        );
        assert_eq!(syv.forsoeg.kategorier[1].stier[0].sti, offentlig_sti("/.env"));
        let tredive = regn(&spoerg_b3(&["z"], Vec::new(), Periode::D30), &l, NU);
        assert_eq!(
            (tredive.forsoeg.n, tredive.forsoeg.kategorier[0].id.as_str()),
            (114, "hemmeligheder")
        );
        let dag = regn(&spoerg_b3(&["z"], Vec::new(), Periode::Dag(dage[0].clone())), &l, NU);
        assert_eq!(dag.forsoeg.n, 5);
        let land = vec![Filter {
            dim: Dim::Land,
            vaerdi: "PL".into(),
        }];
        assert_eq!(regn(&spoerg_b3(&["z"], land, Periode::D7), &l, NU).forsoeg.n, 14);
    }

    /// Tabellen under søjlerne har alle forespørgsler pr. dag.
    #[test]
    fn dagene_har_alle_forespoergsler() {
        let dag = dag_noegle(NU - DOEGN_MS);
        let l = laest_b3(vec![("z", raekke_b3(dag.clone(), "alle", "", 4200))], std::slice::from_ref(&dag));
        let u = regn(&spoerg_b3(&["z"], Vec::new(), Periode::D7), &l, NU);
        assert_eq!(u.dage.iter().find(|d| d.dag == dag).map(|d| d.alle), Some(4200));
        assert!(u.dage.iter().filter(|d| d.dag != dag).all(|d| d.alle == 0));
    }

    fn spoerg_b3(zoner: &[&str], filtre: Vec<Filter>, periode: Periode) -> Spoergsmaal {
        Spoergsmaal {
            zoner: zoner.iter().map(|z| z.to_string()).collect(),
            filtre,
            periode,
            slags: None,
            side: 0,
        }
    }

    /// Forespørgslerne i udsnittet og dagene med usædvanlig meget trafik, også summeret over domæner.
    #[test]
    fn forespoergsler_i_alt_og_spring_i_perioden() {
        const NU2: i64 = 1_793_779_680_000; // 2026-11-04T08:08:00Z
        let d = |i: i64| dag_noegle(NU2 - i * DOEGN_MS);
        let mut rows: Vec<(&str, Raekke)> = Vec::new();
        for z in ["a", "b"] {
            for i in 1..=30 {
                rows.push((z, raekke_b3(d(i), "alle", "", if i == 3 { 5000 } else { 100 })));
            }
        }
        let l = laest_b3(rows, &[]);
        let s = spoerg_b3(&["a", "b"], Vec::new(), Periode::Spaend { fra: d(4), til: d(2) });
        let u = regn(&s, &l, NU2);
        assert_eq!(u.trafik.i_alt, 2 * (100 + 5000 + 100));
        assert_eq!(
            u.trafik.dage,
            [TrafikDag {
                dag: d(3),
                n: 10_000,
                median: 200.0,
                stoppet: 0
            }],
            "hvert domæne regnes for sig, og tallene lægges sammen"
        );
        let uden = spoerg_b3(&["a"], Vec::new(), Periode::Spaend { fra: d(2), til: d(1) });
        assert!(regn(&uden, &l, NU2).trafik.dage.is_empty(), "uden for perioden");
    }

    /// Et udsnit, der kun har tal i dag, siger det med dagens tal.
    #[test]
    fn kun_i_dag_viser_dagens_tal() {
        const NU2: i64 = 1_793_779_680_000;
        let d = |i: i64| dag_noegle(NU2 - i * DOEGN_MS);
        let bot = |i: i64, n| {
            (
                "a",
                raekke_b3(d(i), "bot", &["GoogleOther", "Google", "AI Search", "/x", "200"].join("\t"), n),
            )
        };
        let f = vec![Filter {
            dim: Dim::Robot,
            vaerdi: "GoogleOther".into(),
        }];
        let u = regn(
            &spoerg_b3(&["a"], f.clone(), Periode::Alt),
            &laest_b3(vec![bot(0, 1)], &[d(1)]),
            NU2,
        );
        assert_eq!((u.noegletal.alt, u.noegletal.i_dag_hidtil), (0, 1));
        assert_eq!(
            u.kun_i_dag.map(|k| k.stier.into_iter().map(|x| (x.v, x.n)).collect::<Vec<_>>()),
            Some(vec![("/x".to_string(), 1)])
        );
        let u = regn(
            &spoerg_b3(&["a"], f, Periode::Alt),
            &laest_b3(vec![bot(0, 1), bot(2, 1)], &[d(1)]),
            NU2,
        );
        assert_eq!(u.kun_i_dag, None, "der er også hele døgn");
    }

    #[test]
    fn forsoeg_alle_er_alle_stier_med_404() {
        const NU2: i64 = 1_793_779_680_000;
        let d = |i: i64| dag_noegle(NU2 - i * DOEGN_MS);
        let l = laest_b3(
            vec![
                ("a", raekke_b3(d(1), "probe", "/.env", 2)),
                ("a", raekke_b3(d(1), "probe", "/x.php", 3)),
            ],
            &[d(1)],
        );
        let f = vec![Filter {
            dim: Dim::Forsoeg,
            vaerdi: ALLE_FORSOEG.into(),
        }];
        assert_eq!(regn(&spoerg_b3(&["a"], f, Periode::Dag(d(1))), &l, NU2).sum, 5);
    }

    /// Alle gemte angreb for udsnittet, 25 ad gangen, med N af M.
    #[test]
    fn de_seneste_viser_alle_gemte_angreb_side_for_side() {
        const NU2: i64 = 1_793_779_680_000;
        let d = |i: i64| dag_noegle(NU2 - i * DOEGN_MS);
        let dag = d(2);
        let noegle = |k: i64, land: &str| {
            let net = if k < 5 { "Mal AS" } else { "Andet AS" };
            [
                &format!("{dag}T{:02}:00:00Z", k % 24),
                "Regel",
                "",
                land,
                &format!("/s{k}"),
                "CPH",
                "GET",
                "firewallManaged",
                "block",
                net,
            ]
            .join("\t")
        };
        let fw = ["Regel", "US", "AS", "/", "CPH", "GET", "firewallManaged", "block"].join("\t");
        let mut l = laest_b3(vec![("a", raekke_b3(dag.clone(), "fw", &fw, 300))], std::slice::from_ref(&dag));
        l.angreb = (0..60)
            .map(|k| {
                (
                    "a".to_string(),
                    raekke_b3(dag.clone(), "angreb", &noegle(k, if k < 10 { "PL" } else { "US" }), 2),
                )
            })
            .collect();
        let mut s = spoerg_b3(&["a"], Vec::new(), Periode::Dag(dag.clone()));
        let u = regn(&s, &l, NU2);
        assert_eq!(u.enkelte.len(), SIDE);
        assert_eq!(
            u.enkelte_info,
            Some(EnkelteInfo {
                vist: 120,
                i_alt: 300,
                side: 1,
                sider: 3,
                loft: true
            })
        );
        assert_eq!(u.enkelte[0].n, Some(2));
        s.side = 2;
        assert_eq!(regn(&s, &l, NU2).enkelte.len(), 10, "sidste side");
        s.side = 9;
        assert_eq!(regn(&s, &l, NU2).enkelte_info.unwrap().side, 3, "en side for meget er den sidste");
        s.side = 0;
        s.filtre = vec![Filter {
            dim: Dim::Land,
            vaerdi: "PL".into(),
        }];
        assert_eq!(regn(&s, &l, NU2).enkelte_info.unwrap().vist, 20, "filteret gælder");
        s.filtre = vec![Filter {
            dim: Dim::Netvaerk,
            vaerdi: "Mal AS".into(),
        }];
        assert_eq!(
            regn(&s, &l, NU2).enkelte_info.unwrap().vist,
            10,
            "klik på Ligner målrettet: netværket står i nøglen"
        );
        let gammel = (0..3)
            .map(|i| noegle(i, "PL").rsplit_once('\t').unwrap().0.to_string())
            .collect::<Vec<_>>();
        let mut uden = l.clone();
        uden.angreb = gammel
            .iter()
            .map(|k| ("a".to_string(), raekke_b3(dag.clone(), "angreb", k, 2)))
            .collect();
        assert_eq!(
            regn(&s, &uden, NU2).enkelte_info,
            None,
            "en række fra før skema 12 har intet netværk"
        );
        s.filtre.clear();
        s.slags = Some(Slags::Rutine);
        assert_eq!(regn(&s, &l, NU2).enkelte_info, None, "rutine: de 12 seneste som før");
        s.slags = None;
        s.periode = Periode::Dag(d(3));
        assert_eq!(regn(&s, &l, NU2).enkelte_info, None, "uden for perioden, eller ikke hentet endnu");
        let lille = laest_b3(vec![("a", raekke_b3(dag.clone(), "fw", &fw, 200))], std::slice::from_ref(&dag));
        let mut l2 = lille.clone();
        l2.angreb = l.angreb.clone();
        let s2 = spoerg_b3(&["a"], Vec::new(), Periode::Dag(dag.clone()));
        assert!(!regn(&s2, &l2, NU2).enkelte_info.unwrap().loft, "præcis 200");
    }

    #[test]
    fn trafik_regn_taeller_hele_perioden() {
        let rows = vec![
            raekke_b3("2026-10-27".into(), "alle", "", 10),
            raekke_b3("2026-10-28".into(), "alle", "", 400),
            raekke_b3("2026-10-29".into(), "alle", "", 500),
            raekke_b3("2026-10-30".into(), "alle", "", 20),
        ];
        let u = trafik_regn(&rows, "2026-10-28", "2026-10-29", true, true);
        assert_eq!(
            (u.i_alt, u.median, u.dag.as_str(), u.til.as_str()),
            (900, Some(10.0), "2026-10-28", "2026-10-29")
        );
    }

    /// 2026-11-04T08:00:00Z.
    const NU: i64 = 1_793_779_200_000;
    const REACT: &str = "React - RCE - CVE:CVE-2025-55182";

    fn d(i: i64) -> String {
        dag_noegle(NU - i * DOEGN_MS)
    }

    /// En `fw`-række: regel, land, netværk, sti, colo, metode, kilde, handling.
    fn fw(dag: &str, f: [&str; 8], n: i64) -> (String, Raekke) {
        fw_i("z1", dag, f, n)
    }

    fn fw_i(z: &str, dag: &str, f: [&str; 8], n: i64) -> (String, Raekke) {
        (
            z.into(),
            Raekke {
                dag: dag.into(),
                art: "fw".into(),
                noegle: f.join("\t"),
                n,
            },
        )
    }

    fn react(land: &str, sti: &str, colo: &str) -> [String; 8] {
        [
            REACT.into(),
            land.into(),
            "Nordlys Hosting".into(),
            sti.into(),
            colo.into(),
            "POST".into(),
            "firewallManaged".into(),
            "block".into(),
        ]
    }

    fn r(dag: &str, f: [String; 8], n: i64) -> (String, Raekke) {
        fw(dag, f.each_ref().map(String::as_str), n)
    }

    fn art(z: &str, dag: &str, a: &str, noegle: &str, n: i64) -> (String, Raekke) {
        (
            z.into(),
            Raekke {
                dag: dag.into(),
                art: a.into(),
                noegle: noegle.into(),
                n,
            },
        )
    }

    fn filter(dim: Dim, v: &str) -> Filter {
        Filter { dim, vaerdi: v.into() }
    }

    fn spoerg(filtre: Vec<Filter>, periode: Periode, slags: Option<Slags>) -> Spoergsmaal {
        tjek(Omfang::Zone("z1".into()), filtre, periode, slags).unwrap()
    }

    /// Kendte tællerækker: React fra PL og US, Wordpress fra PL, labyrinten og botFight, plus tal uden for perioden.
    fn laest() -> Laest {
        let rows = vec![
            r(&d(0), react("PL", "/", "WAW"), 2),
            r(&d(1), react("PL", "/", "WAW"), 5),
            r(&d(1), react("US", "/api", "IAD"), 1),
            r(&d(3), react("PL", "/wp-config%2ephp", "FRA"), 4),
            r(&d(7), react("PL", "/", "WAW"), 10),
            r(&d(8), react("US", "/", "IAD"), 20),
            r(&d(30), react("US", "/", "IAD"), 40),
            r(&d(31), react("US", "/", "IAD"), 80),
            fw(
                &d(2),
                [
                    "Wordpress - SQL Injection",
                    "PL",
                    "Eksempel Net",
                    "/a",
                    "WAW",
                    "GET",
                    "firewallManaged",
                    "block",
                ],
                3,
            ),
            fw(
                &d(2),
                [
                    "botFight",
                    "PL",
                    "Eksempel Net",
                    "/a",
                    "WAW",
                    "GET",
                    "botFight",
                    "managed_challenge",
                ],
                6,
            ),
            fw(
                &d(2),
                ["linkMaze", "DE", "Nordlys Net", "/b", "FRA", "GET", "linkMaze", "link_maze_visited"],
                7,
            ),
            fw(
                &d(2),
                [
                    "linkMaze",
                    "DE",
                    "Nordlys Net",
                    "/b",
                    "FRA",
                    "GET",
                    "linkMaze",
                    "link_maze_injected",
                ],
                900,
            ),
            fw(
                &d(2),
                ["min-regel: max 100", "DK", "Fjord Net", "/c", "CPH", "GET", "ratelimit", "block"],
                1,
            ),
        ];
        let hentet = (0..=31).map(d).collect();
        Laest {
            aar: Vec::new(),
            trafik_dage: HashSet::new(),
            profiler: Vec::new(),
            angreb: Vec::new(),
            rows,
            andre: vec![
                fw_i("z2", &d(1), react("PL", "/", "WAW").each_ref().map(String::as_str), 9),
                fw_i("z3", &d(1), react("PL", "/", "WAW").each_ref().map(String::as_str), 30),
                fw_i("z4", &d(1), react("PL", "/", "WAW").each_ref().map(String::as_str), 99),
            ],
            hentet,
            navne: HashMap::from([
                ("z1".into(), "eksempel.wi24rd.com".into()),
                ("z2".into(), "b.wi24rd.com".into()),
                ("z3".into(), "a.wi24rd.com".into()),
            ]),
            ..Laest::default()
        }
    }

    fn vn(v: &[VaerdiN]) -> Vec<(&str, i64)> {
        v.iter().map(|x| (x.v.as_str(), x.n)).collect()
    }

    fn robotrader() -> Laest {
        let bot =
            |dag: &str, navn: &str, kat: &str, sti: &str, st: &str, n: i64| art("z1", dag, "bot", &[navn, "", kat, sti, st].join("\t"), n);
        Laest {
            aar: Vec::new(),
            trafik_dage: HashSet::new(),
            profiler: Vec::new(),
            angreb: Vec::new(),
            rows: vec![
                bot(&d(1), "GPTBot", "AI Crawler", "/", "200", 5),
                bot(&d(2), "GPTBot", "AI Crawler", "/priser/", "404", 2),
                bot(&d(0), "GPTBot", "AI Crawler", "/", "200", 9),
                bot(&d(2), "Googlebot", "Search Engine Crawler", "/", "200", 3),
                bot(&d(2), "Ny", "Agent", "/", "200", 1),
                art("z1", &d(1), "falsk", "Googlebot\tefterligning", 7),
                art("z1", &d(1), "falsk", "FooBot\tuverificeret", 4),
                art("z1", &d(1), "stoppet", "GPTBot\tOpenAI\tbotFight\tmanaged_challenge", 6),
                art("z1", &d(1), "stoppet", "ClaudeBot\tAnthropic\tlinkMaze\tlink_maze_visited", 8),
                r(&d(1), react("PL", "/", "WAW"), 100),
            ],
            andre: vec![art("z2", &d(1), "bot", "GPTBot\t\tAI Crawler\t/\t200", 11)],
            hentet: (0..=31).map(d).collect(),
            navne: HashMap::from([("z1".into(), "eksempel.wi24rd.com".into()), ("z2".into(), "b.wi24rd.com".into())]),
            ..Laest::default()
        }
    }

    #[test]
    fn robot_og_gruppe_taeller_kun_robotter() {
        let l = robotrader();
        let tal = |f: Vec<Filter>| regn(&spoerg(f, Periode::D30, None), &l, NU);
        for (dim, v, sum) in [
            (Dim::Robot, "alle", 11),
            (Dim::Robot, "GPTBot", 7),
            (Dim::Robot, "udklaedt", 7),
            (Dim::Robot, "udklaedt:Googlebot", 7),
            (Dim::Robot, "uverificeret", 4),
            (Dim::Robot, "stoppet", 6),
            (Dim::Robot, "labyrint", 8),
            (Dim::Robot, "findes-ikke", 0),
            (Dim::RobotGruppe, "ai-traening", 7),
            (Dim::RobotGruppe, "soegemaskiner", 3),
            (Dim::RobotGruppe, "ukendt", 1 + 7 + 4),
        ] {
            let u = tal(vec![filter(dim, v)]);
            assert_eq!(u.sum, sum, "{dim:?} {v}");
            assert_eq!(u.dage.iter().map(|x| x.n).sum::<i64>(), sum, "søjlerne {dim:?} {v}");
            assert!(u.top.regel.is_empty() && u.fordeling == Fordeling::default(), "fw tæller ikke med");
        }
        let u = tal(vec![filter(Dim::Robot, "GPTBot"), filter(Dim::Sti, "/priser/")]);
        assert_eq!(u.sum, 2, "robot og sti");
        let u = tal(vec![filter(Dim::Robot, "alle")]);
        assert_eq!(vn(&u.top.robot), [("GPTBot", 7), ("Googlebot", 3), ("Ny", 1)]);
        assert_eq!(vn(&u.top.status), [("200", 9), ("404", 2)]);
        assert_eq!(u.noegletal.i_dag_hidtil, 9, "i dag står for sig");
        assert_eq!(u.domaener.len(), 1, "samme filter på de andre domæner");
        assert_eq!((u.domaener[0].navn.as_str(), u.domaener[0].n), ("b.wi24rd.com", 11));
        assert!(u.enkelte.is_empty(), "enkelthændelser har ingen robot");
        assert_eq!(
            tal(vec![filter(Dim::Robot, "udklaedt"), filter(Dim::Sti, "/")]).sum,
            0,
            "falsk har ingen sti"
        );
    }

    #[test]
    fn robot_kan_ikke_kombineres_med_fw_filtre() {
        let t = |f: Vec<Filter>| tjek(Omfang::Zone("z1".into()), f, Periode::D30, None).err();
        for dim in [Dim::Regel, Dim::Land, Dim::Netvaerk, Dim::Colo, Dim::Metode, Dim::Forsoeg] {
            assert_eq!(
                t(vec![filter(Dim::Robot, "alle"), filter(dim, "x")]),
                Some(Fejl::UgyldigtFilter),
                "{dim:?}"
            );
            assert_eq!(
                t(vec![filter(Dim::RobotGruppe, "seo"), filter(dim, "x")]),
                Some(Fejl::UgyldigtFilter),
                "{dim:?}"
            );
        }
        assert_eq!(
            t(vec![filter(Dim::Robot, "alle"), filter(Dim::Handling, "blokeret")]),
            Some(Fejl::UgyldigtFilter)
        );
        assert_eq!(t(vec![filter(Dim::RobotGruppe, "ny-gruppe")]), Some(Fejl::UgyldigtFilter));
        assert_eq!(t(vec![filter(Dim::Robot, "alle"), filter(Dim::Sti, "/")]), None);
        assert_eq!(t(vec![filter(Dim::Robot, "alle"), filter(Dim::RobotGruppe, "seo")]), None);
    }

    #[test]
    fn hver_dimension_alene_summer_rigtigt() {
        let l = laest();
        for (dim, v, sum) in [
            (Dim::Regel, REACT, 5 + 1 + 4 + 10 + 20 + 40),
            (Dim::Land, "PL", 5 + 4 + 10 + 3 + 6),
            (Dim::Netvaerk, "Nordlys Net", 7),
            (Dim::Sti, "/", 5 + 10 + 20 + 40),
            (Dim::Colo, "WAW", 5 + 10 + 3 + 6),
            (Dim::Metode, "GET", 3 + 6 + 7 + 1),
            (Dim::Handling, "blokeret", 80 + 3 + 1),
            (Dim::Handling, "udfordret", 6),
            (Dim::Handling, "labyrint", 7),
            (Dim::Handling, "faelder", 900),
        ] {
            let u = regn(&spoerg(vec![filter(dim, v)], Periode::D30, None), &l, NU);
            assert_eq!(u.sum, sum, "{dim:?} {v}");
            assert_eq!(u.dage.iter().map(|x| x.n).sum::<i64>(), sum, "søjlerne {dim:?} {v}");
        }
    }

    #[test]
    fn to_og_tre_filtre_sammen() {
        let l = laest();
        let u = regn(
            &spoerg(vec![filter(Dim::Regel, REACT), filter(Dim::Land, "PL")], Periode::D30, None),
            &l,
            NU,
        );
        assert_eq!(u.sum, 5 + 4 + 10);
        let u = regn(
            &spoerg(
                vec![filter(Dim::Regel, REACT), filter(Dim::Land, "PL"), filter(Dim::Colo, "FRA")],
                Periode::D30,
                None,
            ),
            &l,
            NU,
        );
        assert_eq!(u.sum, 4);
        assert_eq!(vn(&u.top.sti), [("/wp-config%2ephp", 4)], "J2: renset sti");
    }

    #[test]
    fn top_sorteret_stabil_hoejst_20_og_antal() {
        let mut l = Laest::default();
        for i in 0..25 {
            let land = format!("L{i:02}");
            l.rows.push(r(&d(1), react(&land, "/", "WAW"), if i < 2 { 5 } else { 1 }));
        }
        let u = regn(&spoerg(vec![], Periode::D30, None), &l, NU);
        assert_eq!(u.top.land.len(), 20);
        assert_eq!(vn(&u.top.land[..3]), [("L00", 5), ("L01", 5), ("L02", 1)], "lige tal efter navn");
        assert_eq!(u.antal["land"], 25, "det rigtige antal, ikke afskåret");
    }

    #[test]
    fn slags_skiller_rutine_og_egne_paa_kilden() {
        let l = laest();
        let land = |k| regn(&spoerg(vec![filter(Dim::Land, "PL")], Periode::D30, k), &l, NU);
        assert_eq!(land(None).sum, 28);
        assert_eq!(land(Some(Slags::Angreb)).sum, 22);
        assert_eq!(land(Some(Slags::Rutine)).sum, 6);
        assert_eq!(land(None).slags, SlagsTal { angreb: 22, rutine: 6 });
        assert_eq!(
            land(Some(Slags::Rutine)).slags,
            SlagsTal { angreb: 22, rutine: 6 },
            "tallene står, uanset valget"
        );
        let alt = regn(&spoerg(vec![], Periode::D30, Some(Slags::Angreb)), &l, NU);
        assert!(
            alt.top.regel.iter().any(|x| x.v == "min-regel: max 100"),
            "en egen regel er ikke rutine"
        );
    }

    #[test]
    fn perioder_og_graensedage() {
        let l = laest();
        let regel = vec![filter(Dim::Regel, REACT)];
        let u = regn(&spoerg(regel.clone(), Periode::D7, None), &l, NU);
        assert_eq!(
            (u.periode.fra.as_str(), u.periode.til.as_str(), u.periode.dage),
            (d(7).as_str(), d(1).as_str(), 7)
        );
        assert_eq!(u.sum, 5 + 1 + 4 + 10, "d-7 med, i dag og d-8 ikke");
        let u = regn(&spoerg(regel.clone(), Periode::D30, None), &l, NU);
        assert_eq!((u.sum, u.dage.len()), (80, 30), "d-30 med, d-31 ikke");
        let u = regn(&spoerg(regel.clone(), Periode::Alt, None), &l, NU);
        assert_eq!((u.sum, u.periode.fra.as_str(), u.dage.len()), (160, d(31).as_str(), 31));
        let u = regn(&spoerg(regel.clone(), Periode::Dag(d(0)), None), &l, NU);
        assert_eq!((u.sum, u.dage.len()), (2, 1), "i dag kan vælges som dag");
        let u = regn(&spoerg(regel.clone(), Periode::Spaend { fra: d(8), til: d(7) }, None), &l, NU);
        assert_eq!((u.sum, u.dage.len()), (30, 2));
        assert_eq!(
            u.noegletal,
            Noegletal {
                i_gaar: 6,
                i_dag_hidtil: 2,
                hele7: 20,
                hele30: 80,
                alt: 160,
                med7: u.noegletal.med7,
                med30: u.noegletal.med30
            },
            "nøgletallene er ens i alle perioder"
        );
    }

    /// J1 og J9: dyk uden filter giver præcis fliserne og Overblikket (`hele_doegn`).
    #[test]
    fn samme_tal_som_fliserne_og_overblikket() {
        let l = laest();
        let rows: Vec<Raekke> = l.rows.iter().map(|(_, r)| r.clone()).collect();
        let h = hele_doegn(&rows, NU);
        let u = regn(&spoerg(vec![], Periode::D30, None), &l, NU);
        assert_eq!(
            (u.noegletal.i_gaar, u.noegletal.i_dag_hidtil, u.noegletal.hele7, u.noegletal.hele30),
            (h.i_gaar, h.i_dag_hidtil, h.hele7, h.hele30)
        );
        assert_eq!(u.sum, h.hele30);
        assert_eq!(u.fordeling, h.fordeling.d30, "også siderne med fælde-links");
        assert_eq!(regn(&spoerg(vec![], Periode::D7, None), &l, NU).sum, h.hele7);
        assert_eq!(regn(&spoerg(vec![], Periode::Dag(d(1)), None), &l, NU).sum, h.i_gaar);
    }

    /// J1: regelkort, lister og søjler i Angreb står på samme tal som dyk, før og efter klikket.
    #[test]
    fn angreb_og_dyk_viser_samme_tal() {
        let l = laest();
        let rows: Vec<Raekke> = l.rows.iter().map(|(_, r)| r.clone()).collect();
        let v = crate::vaern::oversigt_ud(&rows, NU, None);
        assert_eq!(v.lister.h_dage.len(), 30);
        assert_eq!(
            (v.lister.h_dage[0].dag.as_str(), v.lister.h_dage[29].dag.as_str()),
            (d(30).as_str(), d(1).as_str())
        );
        assert_eq!(
            v.lister.h_dage.iter().map(|x| x.n).sum::<i64>(),
            v.hele.hele30,
            "Edge · 30 dage = flisen"
        );
        let kort = |navn: &str| v.lister.h_regler.iter().find(|r| r.navn == navn).unwrap();
        for rg in &v.lister.h_regler {
            let u = regn(&spoerg(vec![filter(Dim::Regel, &rg.navn)], Periode::D30, None), &l, NU);
            assert_eq!((rg.h30, rg.h7), (Some(u.noegletal.hele30), Some(u.noegletal.hele7)), "{}", rg.navn);
            assert_eq!(rg.faelder30, Some(u.fordeling.faelder), "{}", rg.navn);
            assert_eq!(rg.art.as_deref(), u.regel.as_ref().map(|h| h.art.as_str()), "{}", rg.navn);
        }
        assert_eq!(
            (kort("linkMaze").h30, kort("linkMaze").faelder30),
            (Some(7), Some(900)),
            "lokket ind og fælde-links"
        );
        assert!(v.lister.h_regler.windows(2).all(|p| p[0].h30 >= p[1].h30), "størst først");
        for x in &v.lister.h_lande {
            assert_eq!(
                regn(&spoerg(vec![filter(Dim::Land, &x.kode)], Periode::D30, None), &l, NU).sum,
                x.n,
                "{}",
                x.kode
            );
        }
        for x in &v.lister.h_netvaerk {
            assert_eq!(
                regn(&spoerg(vec![filter(Dim::Netvaerk, &x.navn)], Periode::D30, None), &l, NU).sum,
                x.n,
                "{}",
                x.navn
            );
        }
        for x in &v.lister.h_colos {
            assert_eq!(
                regn(&spoerg(vec![filter(Dim::Colo, &x.kode)], Periode::D30, None), &l, NU).sum,
                x.n,
                "{}",
                x.kode
            );
        }
        assert!(
            v.vaern.regler.iter().all(|r| r.h30.is_none() && r.art.is_none()),
            "W's liste er urørt"
        );
        assert!(serde_json::to_value(&v.vaern).unwrap()["regler"][0].get("h30").is_none());
    }

    fn raekker(l: &Laest) -> Vec<Raekke> {
        l.rows.iter().map(|(_, r)| r.clone()).collect()
    }

    /// "Først set" på regelkortet er det samme som i dyk: hele historikken, kun det afviste.
    #[test]
    fn foerst_set_er_ens_i_kort_og_dyk() {
        let mut l = laest();
        l.rows.push(r(&d(200), react("PL", "/", "WAW"), 1));
        l.rows.push(fw(
            &d(150),
            [
                "linkMaze",
                "DE",
                "Nordlys Net",
                "/b",
                "FRA",
                "GET",
                "linkMaze",
                "link_maze_injected",
            ],
            5,
        ));
        let rows = raekker(&l);
        let hele: Vec<Raekke> = rows.iter().filter(|x| x.dag >= d(30)).cloned().collect();
        let mut v = crate::vaern::oversigt_ud(&hele, NU, None);
        let kort = |v: &crate::vaern::VaernUd, navn: &str| v.lister.h_regler.iter().find(|r| r.navn == navn).unwrap().clone();
        assert_eq!(kort(&v, REACT).foerst, d(30), "uden historikken: kun vinduet");
        let mut lager = Store::i_hukommelse().unwrap();
        for dag in rows.iter().map(|x| x.dag.clone()).collect::<HashSet<_>>() {
            let paa_dagen: Vec<(String, String, i64)> = rows
                .iter()
                .filter(|x| x.dag == dag)
                .map(|x| (x.art.clone(), x.noegle.clone(), x.n))
                .collect();
            lager.gem_doegn("z1", &dag, &paa_dagen, "2026-11-04T00:00:00Z").unwrap();
        }
        lager
            .gem_doegn("z1", &d(40), &[("probe".into(), "/kun-probe".into(), 1)], "2026-11-04T00:00:00Z")
            .unwrap();
        let datoer = lager.fw_datoer(&["z1".to_string()]).unwrap();
        assert!(datoer.iter().all(|(n, _, _)| n.split('\t').count() == 8), "kun art fw");
        v.lister.med_historik(&datoer);
        for navn in [REACT, "linkMaze", "min-regel: max 100"] {
            let u = regn(&spoerg(vec![filter(Dim::Regel, navn)], Periode::D30, None), &l, NU);
            let k = kort(&v, navn);
            assert_eq!((Some(k.foerst), Some(k.sidst)), (u.foerst, u.sidst), "{navn}");
        }
        assert_eq!(kort(&v, REACT).foerst, d(200), "ældre end vinduet");
        assert_eq!(kort(&v, REACT).sidst, d(0), "i dag er med");
        assert_eq!(
            kort(&v, "linkMaze").foerst,
            d(2),
            "fælde-links er ikke afvist, så d-150 tæller ikke"
        );
    }

    /// Hver regel har flere nøgler (land, sti …): datoerne er den tidligste og seneste over dem alle, i enhver rækkefølge.
    #[test]
    fn historikken_tager_min_og_maks_over_alle_noegler() {
        let noegle = |land: &str| ["R", land, "N", "/", "WAW", "GET", "firewallManaged", "block"].join("\t");
        let linje = |land: &str, a: &str, b: &str| (noegle(land), a.to_string(), b.to_string());
        let rows = vec![fw(&d(2), ["R", "PL", "N", "/", "WAW", "GET", "firewallManaged", "block"], 1).1];
        for datoer in [
            vec![linje("PL", "2026-02-01", "2026-03-01"), linje("US", "2026-01-01", "2026-04-01")],
            vec![linje("US", "2026-01-01", "2026-04-01"), linje("PL", "2026-02-01", "2026-03-01")],
            vec![linje("PL", "2026-01-01", "2026-03-01"), linje("US", "2026-02-01", "2026-04-01")],
            vec![linje("US", "2026-02-01", "2026-04-01"), linje("PL", "2026-01-01", "2026-03-01")],
        ] {
            let mut v = crate::vaern::oversigt_ud(&rows, NU, None);
            v.lister.med_historik(&datoer);
            let k = &v.lister.h_regler[0];
            assert_eq!((k.foerst.as_str(), k.sidst.as_str()), ("2026-01-01", "2026-04-01"), "{datoer:?}");
        }
    }

    /// Kortene bygges af hele døgn. En regel set kun i dag får sit kort; en regel set kun på d-30 får også; ingen top 12.
    #[test]
    fn regelkort_af_hele_doegn_ikke_w_top_12() {
        let mut rows: Vec<Raekke> = (0..15)
            .map(|i| {
                let navn = format!("Regel {i:02}");
                fw(&d(2), [&navn, "PL", "N", "/", "WAW", "GET", "firewallManaged", "block"], 100 + i).1
            })
            .collect();
        rows.push(fw(&d(0), ["Ny i dag", "PL", "N", "/x", "WAW", "GET", "firewallManaged", "block"], 3).1);
        rows.push(fw(&d(30), ["Kun d-30", "SE", "N", "/y", "ARN", "GET", "firewallManaged", "block"], 1).1);
        rows.push(fw(&d(31), ["For gammel", "SE", "N", "/z", "ARN", "GET", "firewallManaged", "block"], 9).1);
        rows.push(
            fw(
                &d(0),
                ["Fælde i dag", "SE", "N", "/z", "ARN", "GET", "linkMaze", "link_maze_injected"],
                9,
            )
            .1,
        );
        let v = crate::vaern::oversigt_ud(&rows, NU, None);
        assert_eq!(v.vaern.regler.len(), 12, "W's liste er stadig 12");
        let navne: Vec<&str> = v.lister.h_regler.iter().map(|r| r.navn.as_str()).collect();
        assert_eq!(navne.len(), 17, "{navne:?}");
        assert_eq!(navne[0], "Regel 14", "størst først");
        assert_eq!(&navne[15..], ["Kun d-30", "Ny i dag"], "{navne:?}");
        let ny = v.lister.h_regler.iter().find(|r| r.navn == "Ny i dag").unwrap();
        assert_eq!((ny.h30, ny.h7, ny.idag), (Some(0), Some(0), Some(3)));
        assert_eq!(ny.stier, vec![], "stier og lande er de hele døgns");
        assert_eq!(v.lister.h_idag, d(0));
        let enkelt = regn(
            &spoerg(vec![filter(Dim::Regel, "Ny i dag")], Periode::Dag(d(0)), None),
            &laest_af(&rows),
            NU,
        );
        assert_eq!(enkelt.sum, 3, "kortet åbner dagen i dag, hvor tallet står");
        assert_eq!(enkelt.noegletal.i_dag_hidtil, ny.idag.unwrap());
    }

    fn laest_af(rows: &[Raekke]) -> Laest {
        Laest {
            aar: Vec::new(),
            trafik_dage: HashSet::new(),
            profiler: Vec::new(),
            angreb: Vec::new(),
            rows: rows.iter().map(|x| ("z1".to_string(), x.clone())).collect(),
            hentet: (0..=31).map(d).collect(),
            ..Laest::default()
        }
    }

    /// Stier står i offentlig form på kortet, og dyk kan filtrere på dem.
    #[test]
    fn regelkortets_stier_er_offentlige_og_virker_i_dyk() {
        let l = laest();
        let v = crate::vaern::oversigt_ud(&raekker(&l), NU, None);
        let rg = v.lister.h_regler.iter().find(|r| r.navn == REACT).unwrap();
        for s in &rg.stier {
            let u = regn(
                &spoerg(vec![filter(Dim::Regel, REACT), filter(Dim::Sti, &s.sti)], Periode::D30, None),
                &l,
                NU,
            );
            assert_eq!(u.sum, s.n, "{}", s.sti);
        }
        assert!(rg.stier.iter().all(|s| !s.sti.contains("%2e")), "{:?}", rg.stier);
    }

    /// Alle lande og datacentre står i Angreb, ikke kun top 15 og top 20; buerne er begrænset, men tallet er kendt.
    #[test]
    fn alle_lande_og_datacentre_i_angreb() {
        let kode = |i: usize| format!("{}{}", (b'A' + (i / 26) as u8) as char, (b'A' + (i % 26) as u8) as char);
        let rows: Vec<Raekke> = (0..44)
            .flat_map(|i| {
                let (land, colo) = (kode(i), format!("C{}", kode(i)));
                let f = ["R", land.as_str(), "N", "/", colo.as_str(), "GET", "firewallManaged", "block"];
                [
                    fw(&d(2), f, 2).1,
                    fw(&d(3), ["R", land.as_str(), "N", "/", "WAW", "GET", "firewallManaged", "block"], 1).1,
                ]
            })
            .chain([fw(&d(2), ["R", "T1", "N", "/", "UDI", "GET", "firewallManaged", "block"], 1).1])
            .collect();
        let v = crate::vaern::oversigt_ud(&rows, NU, None);
        assert_eq!(v.vaern.lande.len(), 15, "W's liste er urørt");
        assert_eq!(v.vaern.colos.len(), 20, "W's liste er urørt");
        assert_eq!(v.lister.h_lande.len(), 45);
        assert_eq!(v.lister.h_colos.len(), 46, "44 + WAW + UDI");
        assert!(v.lister.h_lande.iter().any(|x| x.kode == "T1") && v.lister.h_colos.iter().any(|x| x.kode == "UDI"));
        assert_eq!(v.lister.h_buer.len(), 60);
        assert_eq!(v.lister.h_buer_i_alt, 89);
        assert_eq!(v.lister.h_lande.iter().map(|x| x.n).sum::<i64>(), v.hele.hele30, "summen er flisen");
    }

    /// "Hvad de ledte efter" i Angreb har samme tal som dyk: 7 hele døgn, kategorier og stier.
    #[test]
    fn forsoeg_i_angreb_er_samme_tal_som_dyk() {
        let mut l = laest();
        for (dag, sti, n) in [
            (0, "/wp-login.php", 50),
            (1, "/wp-login.php", 3),
            (7, "/wp-admin", 2),
            (8, "/wp-login.php", 40),
            (2, "/.env", 7),
            (3, "/wp-config%2ephp", 4),
        ] {
            l.rows.push(art("z1", &d(dag), "probe", sti, n));
        }
        let v = crate::vaern::oversigt_ud(&raekker(&l), NU, None);
        let f = &v.lister.h_forsoeg;
        assert_eq!(f.i7, 16, "hverken i dag (50) eller d-8 (40)");
        for k in &f.kategorier {
            let dyk = |x: &str| regn(&spoerg(vec![filter(Dim::Forsoeg, x)], Periode::D7, None), &l, NU).sum;
            assert_eq!(dyk(&k.id), k.n, "{}", k.id);
            for s in &k.stier {
                assert_eq!(dyk(&s.sti), s.n, "{}", s.sti);
            }
        }
        assert!(f.kategorier.iter().any(|k| k.id == "wordpress"));
        assert!(
            f.kategorier.iter().flat_map(|k| &k.stier).all(|s| s.sti == offentlig_sti(&s.sti)),
            "offentlig form"
        );
    }

    /// "Kun angreb" er statusreglens `angreb()`, også for `log` fra egne regler, så Overblikkets ugetal er dykkets tal.
    #[test]
    fn kun_angreb_er_statusreglens_angreb() {
        let mut l = laest();
        l.rows.push(fw(
            &d(2),
            ["min-regel: max 100", "DK", "Fjord Net", "/c", "CPH", "GET", "ratelimit", "log"],
            4,
        ));
        l.rows.push(fw(
            &d(9),
            ["min-regel: max 100", "DK", "Fjord Net", "/c", "CPH", "GET", "ratelimit", "log"],
            6,
        ));
        let a = crate::overblik::angreb(&raekker(&l), NU);
        let spaend = |fra: i64, til: i64, k| regn(&spoerg(vec![], Periode::Spaend { fra: d(fra), til: d(til) }, k), &l, NU);
        let uge = spaend(7, 1, Some(Slags::Angreb));
        assert_eq!(uge.sum, a.uge, "denne uge");
        assert_eq!(spaend(14, 8, Some(Slags::Angreb)).sum, a.uge_foer, "ugen før");
        assert_eq!(uge.slags.angreb, a.uge, "tallet på knappen er det samme");
        assert_eq!(
            spaend(7, 1, None).slags.angreb,
            a.uge,
            "også når Alt er valgt: knappen lover det tal, den åbner"
        );
        assert_eq!(uge.fordeling.andet, 4, "log står i fordelingen");
        let rutine = spaend(7, 1, Some(Slags::Rutine));
        assert_eq!(
            spaend(7, 1, None).sum,
            a.uge - 4 + rutine.sum,
            "Alt tæller kun det afviste: log ud, rutinen ind"
        );
        assert_eq!(
            regn(&spoerg(vec![], Periode::Dag(d(2)), Some(Slags::Angreb)), &l, NU).sum,
            crate::overblik::angreb(&raekker(&l), NU - DOEGN_MS).i_gaar,
            "statusdøgnet om natten"
        );
        assert_eq!(rutine.fordeling.faelder, 900, "fælde-links står for sig");
        assert!(rutine.sum < 900, "og er ude af rutinens tal");
    }

    /// Legenden kan sige, hvor mange buer der er udeladt: tallet er før afskæringen.
    #[test]
    fn buer_i_alt_er_foer_afskaeringen() {
        let rows: Vec<(String, Raekke)> = (0..70)
            .map(|i| {
                let colo = format!("C{i:02}");
                fw(&d(2), ["R", "PL", "N", "/", &colo, "GET", "firewallManaged", "block"], 1)
            })
            .collect();
        let u = regn(
            &spoerg(vec![], Periode::D30, None),
            &Laest {
                rows,
                hentet: (0..=31).map(d).collect(),
                ..Laest::default()
            },
            NU,
        );
        assert_eq!((u.buer.len(), u.buer_i_alt), (60, 70));
    }

    /// Egne regler i listen kan kendes på arten (kilden), rutine og angreb også. De andre lister har ingen art.
    #[test]
    fn kilden_er_med_i_top_regel() {
        let u = regn(&spoerg(vec![], Periode::D30, None), &laest(), NU);
        assert_eq!(
            (u.i_gaar.as_str(), u.i_dag.as_str()),
            (d(1).as_str(), d(0).as_str()),
            "flisernes dage"
        );
        let art = |navn: &str| u.top.regel.iter().find(|x| x.v == navn).and_then(|x| x.art.as_deref());
        assert_eq!(art("min-regel: max 100"), Some("egen"));
        assert_eq!(art(REACT), Some("angreb"));
        assert_eq!(art("linkMaze"), Some("rutine"));
        assert!(u.top.land.iter().all(|x| x.art.is_none()));
        let json = serde_json::to_value(&u).unwrap();
        assert!(json["top"]["land"][0].get("art").is_none(), "felter uden art står ikke i svaret");
    }

    /// Overblikkets klik: dagene bag tallene, også om natten, hvor statusdøgnet er d-2.
    #[test]
    fn overblikkets_dage_svarer_til_ugetallene() {
        use crate::overblik::{raekke, status_dag_test};
        let mut l = laest();
        l.rows.push(r(&d(14), react("PL", "/", "WAW"), 3));
        l.rows.push(r(&d(15), react("PL", "/", "WAW"), 5));
        let rows = raekker(&l);
        let zone = crate::cf::Zone {
            id: "z1".into(),
            navn: "eksempel.wi24rd.com".into(),
        };
        for (nu, nat) in [(NU, false), (NU - 7 * 3_600_000, true)] {
            let r = raekke(&zone, &status_dag_test(nu, nat), &rows, nu);
            assert_eq!(r.opdateres, nat);
            let dag = |i: i64| dag_noegle(nu - i * DOEGN_MS);
            assert_eq!(r.i_gaar_dag, dag(1), "i går flytter sig ikke om natten");
            let forskyd = i64::from(nat);
            assert_eq!(r.status_dag, dag(1 + forskyd));
            let spaend = |fra: &str, til: &str| Periode::Spaend {
                fra: fra.into(),
                til: til.into(),
            };
            let tal = |p| regn(&spoerg(vec![], p, Some(Slags::Angreb)), &l, nu).sum;
            assert_eq!(tal(spaend(&r.uge_fra, &r.status_dag)), r.angreb_uge, "denne uge");
            assert_eq!(tal(spaend(&r.ugen_foer_fra, &r.ugen_foer_til)), r.angreb_ugen_foer, "sidste uge");
            assert_eq!(tal(Periode::Dag(r.status_dag.clone())), r.angreb_i_gaar, "Hvorfor");
            assert_eq!(
                regn(&spoerg(vec![], Periode::Dag(r.i_gaar_dag.clone()), None), &l, nu).sum,
                r.hele.i_gaar,
                "Afvist i går"
            );
        }
    }

    /// Guiden kan pege på en regel: det fulde navn står ved det korte.
    #[test]
    fn angrebstyper_har_de_fulde_navne() {
        let rows: Vec<Raekke> = ["Wordpress - SQL Injection", "Wordpress - Broken Access Control", REACT]
            .iter()
            .map(|n| fw(&d(1), [n, "PL", "N", "/", "WAW", "GET", "firewallManaged", "block"], 2).1)
            .collect();
        let a = crate::overblik::angreb(&rows, NU);
        let wp = a.top.iter().find(|t| t.navn == "WordPress").unwrap();
        assert_eq!(wp.regler, ["Wordpress - SQL Injection", "Wordpress - Broken Access Control"]);
        assert_eq!(a.top.iter().find(|t| t.regler == [REACT]).unwrap().n, 2);
    }

    /// J9: siderne med fælde-links tæller kun, når der filtreres på dem.
    #[test]
    fn faelder_uden_for_afvist_men_kan_vaelges() {
        let l = laest();
        let u = regn(&spoerg(vec![filter(Dim::Regel, "linkMaze")], Periode::D30, None), &l, NU);
        assert_eq!((u.sum, u.fordeling.labyrint, u.fordeling.faelder), (7, 7, 900));
        assert_eq!(vn(&u.top.handling), [("labyrint", 7)]);
        let u = regn(
            &spoerg(
                vec![filter(Dim::Regel, "linkMaze"), filter(Dim::Handling, "faelder")],
                Periode::D30,
                None,
            ),
            &l,
            NU,
        );
        assert_eq!((u.sum, u.fordeling.labyrint, u.fordeling.faelder), (900, 0, 900));
    }

    #[test]
    fn dage_skiller_ikke_hentet_fra_nul() {
        let mut l = laest();
        l.hentet.remove(&d(5));
        let u = regn(&spoerg(vec![filter(Dim::Regel, REACT)], Periode::D7, None), &l, NU);
        let dag = |i: i64| u.dage.iter().find(|x| x.dag == d(i)).unwrap();
        assert_eq!((dag(5).n, dag(5).hentet), (0, false));
        assert_eq!((dag(4).n, dag(4).hentet), (0, true));
        assert_eq!(u.dage.len(), 7, "alle dage i perioden");
    }

    #[test]
    fn foerst_og_sidst_ligger_uden_for_perioden() {
        let l = laest();
        let u = regn(&spoerg(vec![filter(Dim::Regel, REACT)], Periode::D7, None), &l, NU);
        assert_eq!((u.foerst, u.sidst), (Some(d(31)), Some(d(0))));
        let u = regn(&spoerg(vec![filter(Dim::Regel, "findes ikke")], Periode::D7, None), &l, NU);
        assert_eq!((u.foerst, u.sidst, u.sum), (None, None, 0));
    }

    #[test]
    fn sti_matcher_renset_og_offentlig_form() {
        let l = laest();
        let sti = |v: &str| regn(&spoerg(vec![filter(Dim::Sti, v)], Periode::D30, None), &l, NU).sum;
        assert_eq!(sti("/wp-config%2ephp"), 4, "renset");
        assert_eq!(sti("[hemmeligheder]"), 4, "offentlig form fra Angreb");
        assert_eq!(sti("/findes-ikke"), 0);
        assert_eq!(sti("[wordpress]"), 0);
    }

    #[test]
    fn forsoeg_kun_fra_probe_og_aldrig_blandet_med_fw() {
        let mut l = laest();
        l.rows.push(art("z1", &d(1), "probe", "/.env", 11));
        l.rows.push(art("z1", &d(2), "probe", "/wp-config.php", 3));
        l.rows.push(art("z1", &d(2), "probe", "/wp-login.php", 5));
        l.rows.push(r(&d(1), react("PL", "/.env", "WAW"), 1000));
        l.andre.push(art("z2", &d(1), "probe", "/.env", 2));
        let u = regn(&spoerg(vec![filter(Dim::Forsoeg, "hemmeligheder")], Periode::D30, None), &l, NU);
        assert_eq!(u.sum, 14, "kun probe; fw-rækken med samme sti tæller ikke");
        assert_eq!(vn(&u.top.sti), [("/.env", 11), ("/wp-config.php", 3)]);
        assert!(u.top.land.is_empty() && u.buer.is_empty() && u.enkelte.is_empty());
        assert_eq!(u.domaener[0].n, 2);
        let u = regn(&spoerg(vec![filter(Dim::Forsoeg, "/wp-login.php")], Periode::D30, None), &l, NU);
        assert_eq!(u.sum, 5, "en enkelt sti");
        assert_eq!(
            regn(&spoerg(vec![filter(Dim::Sti, "/.env")], Periode::D30, None), &l, NU).sum,
            1000,
            "fw ser ikke probe"
        );
        let andet = regn(&spoerg(vec![filter(Dim::Handling, "andet")], Periode::D30, None), &l, NU);
        assert_eq!(andet.sum, 0, "en probe-række er ikke en fw-række uden handling");
    }

    #[test]
    fn domaener_uden_omfanget_og_uden_ukendte_zoner() {
        let l = laest();
        let u = regn(&spoerg(vec![filter(Dim::Regel, REACT)], Periode::D30, None), &l, NU);
        let v: Vec<(&str, i64)> = u.domaener.iter().map(|x| (x.navn.as_str(), x.n)).collect();
        assert_eq!(v, [("a.wi24rd.com", 30), ("b.wi24rd.com", 9)], "z1 er omfanget, z4 har intet navn");
        let u = regn(&spoerg(vec![filter(Dim::Land, "US")], Periode::D30, None), &l, NU);
        assert!(u.domaener.is_empty(), "ingen tal på de andre");
    }

    /// V-SAMPLE.1: `fw` er et estimat og kan passere 10.000, uden at svaret er afkortet. Loftet afgøres af antal hændelser (`fwn`); et
    /// gammelt døgn uden `fwn` (talte rækker) af `fw` som før.
    #[test]
    fn loft_efter_antal_haendelser_ikke_estimatet() {
        let s = spoerg(vec![], Periode::D30, None);
        let mut l = Laest::default();
        l.rows.push(r(&d(1), react("PL", "/", "WAW"), 14_000));
        l.haendelser.push(art("z1", &d(1), FW_HAENDELSER, "", 6_000));
        assert!(!regn(&s, &l, NU).loft, "14.000 estimeret af 6.000 hændelser er et helt døgn");
        l.haendelser[0].1.n = LOFT;
        assert!(regn(&s, &l, NU).loft, "10.000 hændelser: afkortet");
        let u = regn(&spoerg(vec![], Periode::Dag(d(2)), None), &l, NU);
        assert!(!u.loft, "kun dage i perioden");
        let mut gammel = Laest {
            hentet: (1..=40).map(d).collect(),
            ..Laest::default()
        };
        gammel.rows.push(r(&d(40), react("PL", "/", "WAW"), LOFT));
        assert!(
            regn(&spoerg(vec![], Periode::Alt, None), &gammel, NU).loft,
            "gammelt døgn uden fwn: rækketallet afgør"
        );
    }

    /// Noten "Før X er tallene talt rækker": kun når perioden krydser grænsen, og der er hentede dage før den.
    #[test]
    fn vaegt_graensen_vises_kun_naar_perioden_krydser_den() {
        let mut l = Laest {
            hentet: (0..=40).map(d).collect(),
            vaegt_fra: Some(d(20)),
            ..Laest::default()
        };
        l.rows.push(r(&d(25), react("PL", "/", "WAW"), 3));
        l.rows.push(r(&d(5), react("PL", "/", "WAW"), 3));
        let g = |p: Periode, f: Vec<Filter>, l: &Laest| regn(&spoerg(f, p, None), l, NU).vaegt_fra;
        assert_eq!(g(Periode::D30, vec![], &l), Some(d(20)), "30 dage krydser grænsen");
        assert_eq!(g(Periode::Alt, vec![], &l), Some(d(20)));
        assert_eq!(g(Periode::D7, vec![], &l), None, "7 dage ligger efter grænsen");
        assert_eq!(
            g(Periode::Spaend { fra: d(40), til: d(25) }, vec![], &l),
            None,
            "helt før grænsen krydser den ikke"
        );
        assert_eq!(
            g(Periode::Spaend { fra: d(21), til: d(20) }, vec![], &l),
            Some(d(20)),
            "grænsedagen er den første med estimater"
        );
        assert_eq!(g(Periode::Spaend { fra: d(20), til: d(19) }, vec![], &l), None);
        assert_eq!(
            g(Periode::D30, vec![filter(Dim::Forsoeg, ALLE_FORSOEG)], &l),
            None,
            "forsøg (404) er ikke vægtet"
        );
        assert_eq!(
            g(Periode::D30, vec![filter(Dim::Robot, "alle")], &l),
            None,
            "robotter er ikke vægtet"
        );
        let ny = Laest {
            hentet: (0..=20).map(d).collect(),
            ..l.clone()
        };
        assert_eq!(
            g(Periode::D30, vec![], &ny),
            None,
            "ingen hentede dage før grænsen (ny installation)"
        );
        let uden = Laest { vaegt_fra: None, ..l };
        assert_eq!(g(Periode::D30, vec![], &uden), None);
    }

    #[test]
    fn loft_ved_10000_i_et_zone_doegn() {
        let mut l = Laest::default();
        l.rows.push(r(&d(1), react("PL", "/", "WAW"), 9_999));
        let s = spoerg(vec![filter(Dim::Land, "US")], Periode::D30, None);
        assert!(!regn(&s, &l, NU).loft);
        l.rows.push(r(&d(1), react("US", "/", "IAD"), 1));
        assert!(regn(&s, &l, NU).loft, "hele zone-døgnet tæller, ikke kun filteret");
        let u = regn(&spoerg(vec![], Periode::Dag(d(2)), None), &l, NU);
        assert!(!u.loft, "kun dage i perioden");
    }

    #[test]
    fn enkelte_med_samme_filtre() {
        let senest = |tid: &str, regel: &str, land: &str, sti: &str, n| {
            art(
                "z1",
                &d(1),
                "seneste",
                &[&format!("{}T{tid}:00:00Z", d(1)), regel, "", land, sti, "WAW", "POST"].join("\t"),
                n,
            )
        };
        let mut l = laest();
        l.rows.push(senest("21", REACT, "PL", "/", 1));
        l.rows.push(senest("20", REACT, "US", "[hemmeligheder]", 2));
        l.rows.push(senest("19", "linkMaze", "PL", "/", 1));
        let kald = |f, k| regn(&spoerg(f, Periode::D7, k), &l, NU).enkelte;
        assert_eq!(kald(vec![filter(Dim::Regel, REACT)], None).len(), 3);
        assert_eq!(kald(vec![filter(Dim::Land, "PL")], None).len(), 2);
        assert_eq!(kald(vec![filter(Dim::Land, "PL")], Some(Slags::Rutine)).len(), 1);
        assert_eq!(
            kald(vec![filter(Dim::Sti, "/wp-config%2ephp")], None).len(),
            2,
            "renset sti mod gemt kategori"
        );
        assert!(
            kald(vec![filter(Dim::Netvaerk, "Eksempel Net")], None).is_empty(),
            "netværk står ikke i de gemte"
        );
        assert!(kald(vec![filter(Dim::Handling, "blokeret")], None).is_empty());
    }

    /// Regelkortets og dyk-hovedets etiket er det samme navn som i teknologilisten, selv om Cloudflare staver det anderledes.
    #[test]
    fn regelkortets_etiket_er_navnet_fra_teknologilisten() {
        let mut l = laest();
        l.rows = ["Wordpress - X", "Magento 2 - Y", "Typo3 - Z"]
            .iter()
            .map(|r| fw(&d(2), [r, "US", "N", "/", "CPH", "GET", "firewallManaged", "block"], 5))
            .collect();
        let u = regn(&spoerg(Vec::new(), Periode::D30, None), &l, NU);
        assert_eq!(u.regler.len(), 3);
        for rg in &u.regler {
            let liste = teknologi::navn(&teknologi::af_regel(&rg.navn).unwrap());
            assert_eq!(rg.produkt, liste, "{}", rg.navn);
            assert!(u.rettet_mod.iter().any(|x| teknologi::navn(&x.tek) == rg.produkt), "{}", rg.navn);
            let hoved = regn(&spoerg(vec![filter(Dim::Regel, &rg.navn)], Periode::D30, None), &l, NU)
                .regel
                .unwrap();
            assert_eq!(hoved.produkt, liste, "{}", rg.navn);
        }
    }

    #[test]
    fn regelhoved_med_art_og_cve() {
        let l = laest();
        let hoved = |v: &str| {
            regn(&spoerg(vec![filter(Dim::Regel, v)], Periode::D30, None), &l, NU)
                .regel
                .unwrap()
        };
        let h = hoved(REACT);
        assert_eq!(
            (h.produkt.as_str(), h.type_.as_str(), h.cve.as_deref(), h.art.as_str()),
            ("React", "RCE", Some("CVE-2025-55182"), "angreb")
        );
        assert_eq!(hoved("linkMaze").art, "rutine");
        assert_eq!(hoved("min-regel: max 100").art, "egen");
        assert_eq!(hoved("Browser Integrity Check").art, "rutine", "uden tal: navnet afgør");
        assert!(regn(&spoerg(vec![], Periode::D30, None), &l, NU).regel.is_none());
    }

    #[test]
    fn afviser_ugyldigt() {
        let z = || Omfang::Zone("z1".into());
        let to = vec![filter(Dim::Land, "PL"), filter(Dim::Land, "US")];
        assert_eq!(tjek(z(), to, Periode::D30, None), Err(Fejl::UgyldigtFilter));
        let langt = "x".repeat(MAKS_VAERDI + 1);
        assert_eq!(
            tjek(z(), vec![filter(Dim::Sti, &langt)], Periode::D30, None),
            Err(Fejl::UgyldigtFilter)
        );
        assert!(
            tjek(z(), vec![filter(Dim::Sti, &"æ".repeat(MAKS_VAERDI))], Periode::D30, None).is_ok(),
            "tegn, ikke bytes"
        );
        assert_eq!(
            tjek(z(), vec![filter(Dim::Land, "")], Periode::D30, None),
            Err(Fejl::UgyldigtFilter)
        );
        assert_eq!(
            tjek(z(), vec![filter(Dim::Handling, "block")], Periode::D30, None),
            Err(Fejl::UgyldigtFilter)
        );
        let blandet = vec![filter(Dim::Forsoeg, "php"), filter(Dim::Land, "PL")];
        assert_eq!(tjek(z(), blandet, Periode::D30, None), Err(Fejl::UgyldigtFilter));
        assert_eq!(tjek(z(), vec![], Periode::Dag("2026-13-01".into()), None), Err(Fejl::UgyldigDato));
        let omvendt = Periode::Spaend {
            fra: "2026-11-02".into(),
            til: "2026-11-01".into(),
        };
        assert_eq!(tjek(z(), vec![], omvendt, None), Err(Fejl::UgyldigDato));
        assert_eq!(tjek(Omfang::Zone("a b".into()), vec![], Periode::D30, None), Err(Fejl::UgyldigZone));
        assert_eq!(
            tjek(Omfang::Zoner(vec!["ok".into(), "'".into()]), vec![], Periode::D30, None),
            Err(Fejl::UgyldigZone)
        );
    }

    #[test]
    fn ingen_ip_adresser_i_svaret() {
        let u = regn(&spoerg(vec![filter(Dim::Regel, REACT)], Periode::Alt, None), &laest(), NU);
        let ip = regex::Regex::new(r"\d+\.\d+\.\d+\.\d+").unwrap();
        assert!(!ip.is_match(&serde_json::to_string(&u).unwrap()), "ingen IP-adresser");
    }

    /// Argumenterne fra webvisningen og hele vejen gennem databasen.
    #[test]
    fn json_ind_og_laes_fra_databasen() {
        let omfang: Omfang = serde_json::from_value(serde_json::json!({"zone": "z1"})).unwrap();
        let filtre: Vec<Filter> = serde_json::from_value(serde_json::json!([{"dim": "land", "vaerdi": "PL"}])).unwrap();
        let periode: Periode = serde_json::from_value(serde_json::json!("d30")).unwrap();
        let slags: Option<Slags> = serde_json::from_value(serde_json::json!("rutine")).unwrap();
        assert_eq!(serde_json::from_value::<Periode>(serde_json::json!("alt")).unwrap(), Periode::Alt);
        assert_eq!(
            serde_json::from_value::<Periode>(serde_json::json!({"dag": "2026-11-03"})).unwrap(),
            Periode::Dag("2026-11-03".into())
        );
        assert_eq!(
            serde_json::from_value::<Periode>(serde_json::json!({"spaend": {"fra": "a", "til": "b"}})).unwrap(),
            Periode::Spaend {
                fra: "a".into(),
                til: "b".into()
            }
        );
        assert_eq!(
            serde_json::from_value::<Omfang>(serde_json::json!({"zoner": ["a"]})).unwrap(),
            Omfang::Zoner(vec!["a".into()])
        );
        let s = tjek(omfang, filtre, periode, slags).unwrap();

        let mut lager = Store::i_hukommelse().unwrap();
        let t = iso(NU);
        let rk = |n: i64, h: &str| {
            vec![(
                "fw".to_string(),
                ["botFight", "PL", "Eksempel Net", "/a", "WAW", "GET", "botFight", h].join("\t"),
                n,
            )]
        };
        lager.gem_doegn("z1", &d(1), &rk(4, "managed_challenge"), &t).unwrap();
        lager.gem_doegn("z1", &d(40), &rk(1, "managed_challenge"), &t).unwrap();
        lager.gem_doegn("z2", &d(1), &rk(9, "managed_challenge"), &t).unwrap();
        lager.gem_doegn("z2", &d(40), &rk(100, "managed_challenge"), &t).unwrap();
        let zone = |id: &str, navn: &str| crate::cf::Zone {
            id: id.into(),
            navn: navn.into(),
        };
        lager
            .gem_zoner(1, &[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], &t)
            .unwrap();
        lager.gem_valgte(&["z1".into(), "z2".into()]).unwrap();
        let l = laes(&lager, &s, NU).unwrap();
        assert_eq!(l.andre.len(), 1, "de andre zoner kun i perioden");
        let u = regn(&s, &l, NU);
        assert_eq!((u.sum, u.noegletal.alt, u.foerst.as_deref()), (4, 5, Some(d(40).as_str())));
        assert_eq!(u.domaener[0].n, 9);
        let j = serde_json::to_value(&u).unwrap();
        assert_eq!(j["top"]["land"][0], serde_json::json!({"v": "PL", "n": 4}));
        assert_eq!(j["periode"]["dage"], 30);
    }

    /// D2: "på dine andre domæner" har ikke et domæne, der er slået fra.
    #[test]
    fn andre_domaener_uden_de_fravalgte() {
        let z = |id: &str| crate::cf::Zone {
            id: id.into(),
            navn: format!("{id}.wi24rd.com"),
        };
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(1, &[z("a"), z("b"), z("c")], "t").unwrap();
        for id in ["a", "b", "c"] {
            s.gem_doegn(id, &dag_noegle(NU - DOEGN_MS), &[("fw".to_string(), "k".to_string(), 1)], "t")
                .unwrap();
        }
        s.gem_valgte(&["a".into(), "b".into()]).unwrap();
        let q = tjek(Omfang::Zone("a".into()), Vec::new(), Periode::D30, None).unwrap();
        let l = laes(&s, &q, NU).unwrap();
        assert_eq!(l.andre.iter().map(|(z, _)| z.as_str()).collect::<Vec<_>>(), ["b"]);
    }

    fn fw_kort(dag: &str, regel: &str, land: &str, sti: &str, kilde: &str, n: i64) -> (String, Raekke) {
        (
            "z".to_string(),
            Raekke {
                dag: dag.into(),
                art: "fw".into(),
                noegle: [regel, land, "AS", sti, "CPH", "GET", kilde, "block"].join("\t"),
                n,
            },
        )
    }

    fn sp(filtre: Vec<Filter>, periode: Periode, slags: Option<Slags>) -> Spoergsmaal {
        Spoergsmaal {
            zoner: vec!["z".into()],
            filtre,
            periode,
            slags,
            side: 0,
        }
    }

    fn kort_laest() -> Laest {
        Laest {
            aar: Vec::new(),
            trafik_dage: HashSet::new(),
            profiler: Vec::new(),
            angreb: Vec::new(),
            rows: vec![
                fw_kort(&d(1), REACT, "AU", "/a", "firewallManaged", 5),
                fw_kort(&d(1), REACT, "US", "/b", "firewallManaged", 7),
                fw_kort(&d(3), REACT, "AU", "/c", "firewallManaged", 2),
                fw_kort(&d(40), REACT, "AU", "/gammel", "firewallManaged", 900),
                fw_kort(&d(1), "Wordpress", "US", "/wp", "firewallManaged", 3),
                fw_kort(&d(1), "botFight", "AU", "/", "botFight", 4),
            ],
            hentet: (0..=45).map(d).collect(),
            ..Laest::default()
        }
    }

    /// Kortene følger udsnittet: tal, stier, lande og datoer er kun landets og periodens, og den anden regel uden Australien står ikke.
    #[test]
    fn regelkort_foelger_udsnittet() {
        let land = vec![Filter {
            dim: Dim::Land,
            vaerdi: "AU".into(),
        }];
        let v = regn(&sp(land.clone(), Periode::D30, None), &kort_laest(), NU);
        let navne: Vec<&str> = v.regler.iter().map(|r| r.navn.as_str()).collect();
        assert_eq!(navne, [REACT, "botFight"], "størst først, kun det, der ramte Australien");
        let k = &v.regler[0];
        assert_eq!(k.h30, Some(7), "ikke de 12 fra USA og ikke d-40");
        assert_eq!(k.lande.iter().map(|l| l.kode.as_str()).collect::<Vec<_>>(), ["AU"]);
        assert_eq!(k.stier.len(), 2);
        assert_eq!((k.foerst.as_str(), k.sidst.as_str()), (d(3).as_str(), d(1).as_str()));
        assert_eq!(k.art.as_deref(), Some("angreb"));
        assert_eq!(v.regler[1].art.as_deref(), Some("rutine"));

        let dag = regn(&sp(land, Periode::Dag(d(1)), None), &kort_laest(), NU);
        assert_eq!(dag.regler[0].h30, Some(5), "kun den dag");
        assert_eq!(dag.regler[0].foerst, d(1));
    }

    /// Kun angreb har angrebskortene, Kun rutine rutinekortene.
    #[test]
    fn regelkort_foelger_slags() {
        let navne = |s: Option<Slags>| -> Vec<String> {
            regn(&sp(Vec::new(), Periode::D30, s), &kort_laest(), NU)
                .regler
                .into_iter()
                .map(|r| r.navn)
                .collect()
        };
        assert_eq!(navne(Some(Slags::Angreb)), [REACT, "Wordpress"]);
        assert_eq!(navne(Some(Slags::Rutine)), ["botFight"]);
        assert_eq!(navne(None).len(), 3);
    }

    /// I et regel-dyk er reglen hovedet: ingen kort, heller ikke for andre regler.
    #[test]
    fn regel_dyk_har_ingen_regelkort() {
        let regel = vec![Filter {
            dim: Dim::Regel,
            vaerdi: REACT.into(),
        }];
        let v = regn(&sp(regel, Periode::D30, None), &kort_laest(), NU);
        assert!(v.sum > 0 && v.regel.is_some());
        assert!(v.regler.is_empty());
    }

    #[test]
    fn huller_taeller_i_perioden_og_graenserne_er_dagene_med_tal() {
        let mut l = Laest::default();
        for i in [1, 2, 4, 40] {
            l.hentet.insert(d(i));
        }
        let u = regn(&spoerg(Vec::new(), Periode::D7, None), &l, NU);
        assert_eq!((u.periode.dage, u.periode.med), (7, 3));
        assert_eq!((u.noegletal.med7, u.noegletal.med30), (3, 3));
        assert_eq!((u.periode.data_fra, u.periode.data_til), (Some(d(40)), Some(d(1))));
        assert_eq!(u.periode.i_dag, d(0));
        assert!(
            !u.dage.iter().find(|x| x.dag == d(3)).unwrap().hentet,
            "et hul er ikke en dag med 0"
        );
    }

    /// Alle angrebne teknologier står, sorteret efter antal, også "Bruger ikke", og det, regelnavnet intet siger om, er "Andet". Rutine og egne regler er ikke rettet mod noget produkt.
    #[test]
    fn rettet_mod_har_alt_og_kun_valget_som_maerke() {
        let regel = |r: &'static str, kilde: &'static str, n| fw(&d(2), [r, "US", "N", "/", "CPH", "GET", kilde, "block"], n);
        let mut l = laest();
        l.rows = vec![
            regel("Wordpress - X", "firewallManaged", 9),
            regel("React - RCE", "firewallManaged", 30),
            regel("Generic Rules - X", "firewallManaged", 4),
            regel("min regel", "firewallCustom", 50),
            regel("linkMaze", "linkMaze", 70),
        ];
        let tek = teknologi::TekValg {
            valg: teknologi::Valg::BrugerIkke,
            besked: false,
        };
        l.profiler = vec![teknologi::Profil::from([("wordpress".to_string(), tek)])];
        let u = regn(&spoerg(Vec::new(), Periode::D30, None), &l, NU);
        assert_eq!(
            u.rettet_mod.iter().map(|x| (x.tek.as_str(), x.n, x.valg)).collect::<Vec<_>>(),
            [("react", 30, "ved_ikke"), ("wordpress", 9, "bruger_ikke"), ("andet", 4, "ved_ikke")]
        );
        l.profiler.push(teknologi::Profil::new());
        let u = regn(&spoerg(Vec::new(), Periode::D30, None), &l, NU);
        assert_eq!(u.rettet_mod[1].valg, "blandet", "to domæner, to valg");
        let u = regn(&spoerg(vec![filter(Dim::Teknologi, "andet")], Periode::D30, None), &l, NU);
        assert_eq!(u.sum, 4, "Andet kan åbnes i dyk");
        assert_eq!(tek_i_angreb("Generic Rules - X", "/"), "andet", "også de enkelte angreb");
        assert_eq!(tek_i_angreb("React - RCE", "/"), "react");
        assert!(tjek(
            Omfang::Zone("a".repeat(32)),
            vec![filter(Dim::Teknologi, "magento")],
            Periode::D7,
            None
        )
        .is_ok());
    }

    #[test]
    fn teknologi_filter_taeller_som_regel_og_sti() {
        let wp = ["Wordpress - X", "US", "N", "/", "CPH", "GET", "firewallManaged", "block"];
        let generel = [
            "Generic Rules - X",
            "US",
            "N",
            "/wp-login.php",
            "CPH",
            "GET",
            "firewallManaged",
            "block",
        ];
        let andet = ["Generic Rules - X", "US", "N", "/", "CPH", "GET", "firewallManaged", "block"];
        let l = Laest {
            rows: vec![fw(&d(1), wp, 3), fw(&d(1), generel, 2), fw(&d(1), andet, 9)],
            ..Laest::default()
        };
        let s = spoerg(vec![filter(Dim::Teknologi, "wordpress")], Periode::D7, None);
        assert_eq!(regn(&s, &l, NU).sum, 5);
        assert!(tjek(
            Omfang::Zone("z1".into()),
            vec![filter(Dim::Teknologi, "hemmeligheder")],
            Periode::D7,
            None
        )
        .is_err());
    }

    /// Hele historikken går et år tilbage for trafikken, sikkerhedstallene bliver ved deres egne dage, og et døgn uden tal er et hul.
    #[test]
    fn hele_historikken_gaar_et_aar_tilbage_for_trafik() {
        let sikker = d(3);
        let gammel = d(200);
        let mut l = laest_b3(
            vec![("a", raekke_b3(sikker.clone(), "alle", "", 100))],
            std::slice::from_ref(&sikker),
        );
        l.aar = vec![
            ("a".to_string(), raekke_b3(gammel.clone(), "h_total", "foresp", 40)),
            ("a".to_string(), raekke_b3(sikker.clone(), "h_total", "foresp", 99)),
            ("a".to_string(), raekke_b3(gammel.clone(), "h_total", "trusler", 7)),
        ];
        l.trafik_dage = [gammel.clone(), d(199), sikker.clone()].into_iter().collect();
        let u = regn(&spoerg(vec![], Periode::Alt, None), &l, NU);
        assert_eq!(u.periode.fra, gammel, "året");
        assert_eq!(
            (u.periode.sikkerhed_fra.as_deref(), u.periode.trafik_fra.as_deref()),
            (Some(sikker.as_str()), Some(gammel.as_str()))
        );
        assert_eq!(u.periode.data_fra.as_deref(), Some(gammel.as_str()), "datovælgeren når året");
        assert_eq!(u.periode.med, 1, "sikkerhedstal: én dag");
        assert_eq!(u.trafik.i_alt, 140, "de daglige tal går forud for årstallene");
        let dag = |x: &str| u.trafik_serie.iter().find(|y| y.dag == x).unwrap().clone();
        assert_eq!((dag(&gammel).n, dag(&gammel).hentet), (40, true));
        assert_eq!((dag(&d(100)).n, dag(&d(100)).hentet), (0, false), "et hul, ikke 0");
        assert_eq!(dag(&sikker).n, 100);
        assert!(
            !regn(&spoerg(vec![], Periode::D7, None), &l, NU).trafik_serie.is_empty(),
            "syv dage når før"
        );
        let u = regn(&spoerg(vec![], Periode::Dag(sikker.clone()), None), &l, NU);
        assert!(u.trafik_serie.is_empty(), "kun når perioden når før sikkerhedstallene");
    }

    /// Et døgn ældre end en måned har kun årstallene: lande og svarkoder kommer derfra, og `stoppet` siger intet.
    #[test]
    fn trafik_dyk_for_et_gammelt_doegn() {
        let dag = d(200);
        let rows = vec![
            art("z", &dag, "h_total", "foresp", 500).1,
            art("z", &dag, "h_total", "trusler", 12).1,
            art("z", &dag, "h_total", "cachet", 300).1,
            art("z", &dag, "h_land", "DK", 400).1,
            art("z", &dag, "h_status", "522", 90).1,
            art("z", &dag, "h_total", "unikke", 37).1,
        ];
        let u = trafik_regn(&rows, &dag, &dag, true, true);
        assert_eq!(
            (u.i_alt, u.trusler, u.cachet, u.sikkerhed, u.server_fejl),
            (500, 12, 300, false, 90),
            "unikke besøgende er ikke forespørgsler"
        );
        assert_eq!(u.lande[0].v, "DK");
        let mut nyere = rows.clone();
        nyere.push(art("z", &dag, "alle", "", 510).1);
        nyere.push(art("z", &dag, "land", "SE", 510).1);
        let u = trafik_regn(&nyere, &dag, &dag, true, true);
        assert_eq!((u.i_alt, u.sikkerhed, u.lande.len()), (510, true, 1), "de daglige tal går forud");
    }

    #[test]
    fn trafik_dyk_viser_forespoergslerne_ikke_angrebene() {
        let dag = d(3);
        let mut rows: Vec<Raekke> = vec![
            art("z", &dag, "alle", "", 1000).1,
            art("z", &dag, "land", "US", 920).1,
            art("z", &dag, "land", "DE", 80).1,
            art("z", &dag, "status", "522", 840).1,
            art("z", &dag, "status", "200", 160).1,
            art(
                "z",
                &dag,
                "bot",
                &["Googlebot", "Google", "Search Engine Crawler", "/", "200"].join("\t"),
                700,
            )
            .1,
            r(&dag, react("US", "/", "CPH"), 67).1,
            art("z", &d(4), "land", "CN", 5000).1,
        ];
        rows.extend((4..=13).map(|i| art("z", &d(i), "alle", "", 100).1));
        let u = trafik_regn(&rows, &dag, &dag, true, true);
        assert_eq!((u.i_alt, u.stoppet, u.median), (1000, 67, Some(100.0)));
        assert_eq!(
            u.lande,
            [
                TrafikLand {
                    v: "US".into(),
                    n: 920,
                    angreb: 67
                },
                TrafikLand {
                    v: "DE".into(),
                    n: 80,
                    angreb: 0
                }
            ]
        );
        assert_eq!((u.lande_i_alt, u.status_i_alt, u.server_fejl), (1000, 1000, 840));
        assert_eq!(u.status[0].v, "522");
        assert_eq!((u.robotter[0].v.as_str(), u.robotter[0].n), ("Googlebot", 700));
        assert_eq!(trafik_regn(&rows, &dag, &dag, false, true).median, None, "medianen er ét domænes");
    }
}
