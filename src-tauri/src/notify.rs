//! Notifikationer og "Siden sidst". Reglen er en ren funktion oven på overblikkets rækker
//! (`overblik::raekke`), så samme tal står i overblikket, på kortet og i notifikationen. Kun den lokale
//! notifikationstjeneste; teksterne bygges her på UI'ets sprog (`indstillinger.sprog`), som menulinjen.
//!
//! Forudindstillinger: Rolig = højst én samlet besked om dagen, kun rødt, ikke før kl. 03 UTC (efter natten) · Standard
//! (forvalgt) = rødt efter hver runde · Alt = rødt og gult (gul → rød samme døgn giver én ekstra). Pr. domæne: følg
//! forudindstillingen, Altid (som Alt), Kun rødt (som Standard) eller Aldrig. "Giv besked ved rødt" kan slås fra.
//! Gamle tal og grå er aldrig en alarm, og rutine tæller aldrig som angreb (`overblik::RUTINE`). Højst én besked pr.
//! domæne pr. statusdøgn (`meldt`), så natten (status på døgnet før) aldrig melder det samme igen. "Giv mig besked" på en teknologi (`Raekke1::tek_besked`) melder uanset
//! forudindstillingen, men et domæne på Aldrig forbliver tavst. Mærket (J6): "Ligner målrettet" melder under Standard og Alt, også på en grøn række, men først når det er afgjort
//! (`Raekke1::afgjort`: døgnet efter er hentet), også når programmet var slukket den dag (`Raekke1::efterslaeb`, `vurderede`); "Ligner rekognoscering" kun, når `REKOG` er slået til, og kun første døgn i en episode. `meldt` holder mærket for sig ("gul+maerke") på mærkets
//! eget døgn, så et mærke aldrig undertrykker en rød. "Første gang i døgnet" og Rolig følger den lokale dag (`Tidszone`); tallene er stadig hele døgn i UTC, og "efter natten" handler om dem.
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::agent::Sprog;
use crate::cf::Fejl;
use crate::overblik::{Moenster, MoensterSlags, Raekke1, Status, Tendens, NAT_MS};
use crate::store::Store;
use crate::teknologi;
use crate::vaern::{dag_ms, dag_noegle, parse_iso, DOEGN_MS};

// Nøgler i `indstillinger`.
pub const NIVEAU: &str = "notifikation_niveau";
pub const ROED: &str = "notifikation_roed";
/// "Ligner rekognoscering" giver besked (fra som standard).
pub const REKOG: &str = "notifikation_rekognoscering";
/// Datoformatet: "sprog" (standard), "system" eller "iso", og systemets mønster (fx "d/M/yyyy"), som kun webvisningen kan læse.
/// Tidsskiftene fra webvisningen (`Tidszone`).
pub const TIDSZONE: &str = "tidszone";
pub const DATO_FORMAT: &str = "dato_format";
pub const DATO_MOENSTER: &str = "dato_moenster";
/// Hvornår "Siden sidst" sidst blev vist (ISO, UTC), og hvad der stod på det (`aftryk`).
pub const VIST: &str = "siden_sidst_vist";
pub const VIST_AFTRYK: &str = "siden_sidst_aftryk";
/// Døgnet (UTC) for Roligs ene besked.
pub const ROLIG_SIDST: &str = "rolig_sidst";
/// Det meldte huskes så mange døgn; dedup behøver kun statusdøgnet.
pub const MELDT_DOEGN: i64 = 40;
/// Så mange domæner naevnes i en samlet besked.
const NAEVN: usize = 3;

/// Den lokale tid: (fra ms, minutter øst for UTC), ældste først. Rust kan ikke selv læse tidszonen uden en ny afhængighed, så webvisningen sender skiftene for de næste 13 måneder (`tidsskift` i
/// ramme.js), og de virker, også når vinduet er lukket. Tom = UTC.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Tidszone(pub Vec<(i64, i64)>);

impl Tidszone {
    fn forskel_ms(&self, ms: i64) -> i64 {
        self.0
            .iter()
            .rev()
            .find(|x| x.0 <= ms)
            .or(self.0.first())
            .map_or(0, |x| x.1 * 60_000)
    }

    /// Den lokale dag.
    pub fn dag(&self, ms: i64) -> String {
        dag_noegle(ms + self.forskel_ms(ms))
    }

    /// Lokal time og minut.
    pub fn kl(&self, ms: i64) -> (i64, i64) {
        let inde = (ms + self.forskel_ms(ms)).rem_euclid(DOEGN_MS) / 60_000;
        (inde / 60, inde % 60)
    }
}

pub fn tidszone(lager: &Store) -> Result<Tidszone, Fejl> {
    Ok(Tidszone(
        lager
            .indstilling(TIDSZONE)?
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or_default(),
    ))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Niveau {
    Rolig,
    #[default]
    Standard,
    Alt,
}

impl Niveau {
    pub fn fra(s: &str) -> Option<Niveau> {
        match s {
            "rolig" => Some(Niveau::Rolig),
            "standard" => Some(Niveau::Standard),
            "alt" => Some(Niveau::Alt),
            _ => None,
        }
    }

    pub fn kode(self) -> &'static str {
        match self {
            Niveau::Rolig => "rolig",
            Niveau::Standard => "standard",
            Niveau::Alt => "alt",
        }
    }
}

/// Ét domænes valg. `Standard` = følg forudindstillingen (gemmes ikke).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ZoneValg {
    Standard,
    Alt,
    Roed,
    Fra,
}

impl ZoneValg {
    pub fn fra(s: &str) -> Option<ZoneValg> {
        match s {
            "standard" => Some(ZoneValg::Standard),
            "alt" => Some(ZoneValg::Alt),
            "roed" => Some(ZoneValg::Roed),
            "fra" => Some(ZoneValg::Fra),
            _ => None,
        }
    }

    pub fn kode(self) -> &'static str {
        match self {
            ZoneValg::Standard => "standard",
            ZoneValg::Alt => "alt",
            ZoneValg::Roed => "roed",
            ZoneValg::Fra => "fra",
        }
    }
}

/// Notifikationsindstillingerne, som `beslut` bruger dem.
#[derive(Debug, Clone, PartialEq)]
pub struct Indst {
    pub niveau: Niveau,
    /// "Giv besked ved rødt" (forvalgt til).
    pub roed: bool,
    pub zoner: HashMap<String, ZoneValg>,
    pub rolig_sidst: Option<String>,
    pub rekognoscering: bool,
    pub tz: Tidszone,
}

impl Default for Indst {
    fn default() -> Self {
        Indst {
            niveau: Niveau::Standard,
            roed: true,
            zoner: HashMap::new(),
            rolig_sidst: None,
            rekognoscering: false,
            tz: Tidszone::default(),
        }
    }
}

/// Indstillingerne fra SQLite; ukendte værdier tæller som standarden.
pub fn indst(lager: &Store) -> Result<Indst, Fejl> {
    Ok(Indst {
        niveau: lager.indstilling(NIVEAU)?.as_deref().and_then(Niveau::fra).unwrap_or_default(),
        roed: lager.indstilling(ROED)?.as_deref() != Some("0"),
        zoner: lager
            .notifikation_zoner()?
            .into_iter()
            .filter_map(|(z, v)| ZoneValg::fra(&v).map(|v| (z, v)))
            .collect(),
        rolig_sidst: lager.indstilling(ROLIG_SIDST)?,
        rekognoscering: lager.indstilling(REKOG)?.as_deref() == Some("1"),
        tz: tidszone(lager)?,
    })
}

/// Hvad et domæne kan udløse, når valget pr. domæne og forudindstillingen er lagt sammen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Regel {
    /// Kun rødt, højst én samlet besked om dagen, ikke om natten.
    Rolig,
    /// Rødt efter hver runde.
    Roed,
    /// Rødt og gult.
    Alt,
}

fn regel(ind: &Indst, zone: &str) -> Option<Regel> {
    match ind.zoner.get(zone).copied().unwrap_or(ZoneValg::Standard) {
        ZoneValg::Fra => None,
        ZoneValg::Alt => Some(Regel::Alt),
        ZoneValg::Roed => Some(Regel::Roed),
        ZoneValg::Standard => Some(match ind.niveau {
            Niveau::Rolig => Regel::Rolig,
            Niveau::Standard => Regel::Roed,
            Niveau::Alt => Regel::Alt,
        }),
    }
}

/// Statusdøgnet for en række: i går, eller døgnet før om natten (`opdateres`).
pub fn statusdoegn(r: &Raekke1, nu: i64) -> String {
    dag_noegle(nu - (1 + i64::from(r.opdateres)) * DOEGN_MS)
}

pub fn status_kode(s: Status) -> &'static str {
    match s {
        Status::Roed => "roed",
        Status::Gul => "gul",
        Status::Groen => "groen",
        Status::Graa => "graa",
    }
}

/// Rød eller gul på grund af angreb (ikke "tallene er gamle").
fn usaedvanlig(r: &Raekke1) -> bool {
    // Et domæne på pause giver ingen nye tal, så tilstanden tæller, også når tallene er gamle.
    (!r.gammel || !r.tilstand.is_empty()) && matches!(r.status, Status::Roed | Status::Gul)
}

/// Noget, Siden sidst skal nævne: rød, gul eller et mærke på friske tal.
fn naevnes(r: &Raekke1) -> bool {
    usaedvanlig(r) || (!r.gammel && r.moenster.is_some())
}

/// Et domæne i en besked.
#[derive(Debug, Clone, PartialEq)]
pub struct Fund {
    pub dag: String,
    /// Meldt under Rolig (sætter `rolig_sidst`).
    pub rolig: bool,
    pub r: Raekke1,
    /// Det mærke, der er meldt nu.
    pub maerke: Option<Moenster>,
    /// Det, `meldt` skal huske: (døgn, det højeste niveau og mærket). Mærket står på sit eget døgn.
    pub meldes: Vec<(String, String)>,
}

impl Fund {
    /// Det mærke, beskeden er om.
    pub fn moenster(&self) -> Option<&Moenster> {
        self.maerke.as_ref()
    }
}

/// Én samlet besked; domænerne står rød før gul, derefter efter navn.
#[derive(Debug, Clone, PartialEq)]
pub struct Besked {
    pub fund: Vec<Fund>,
    pub rolig: bool,
}

impl Besked {
    /// Det, der skal skrives i `meldt`: (zone, statusdøgn, niveau).
    pub fn meldte(&self) -> Vec<(String, String, String)> {
        self.fund
            .iter()
            .flat_map(|f| f.meldes.iter().map(|(dag, niveau)| (f.r.id.clone(), dag.clone(), niveau.clone())))
            .collect()
    }
}

/// Mærket i `meldt` står for sig efter niveauet.
const MAERKE: &str = "+maerke";

fn rang(niveau: &str) -> u8 {
    match niveau {
        "roed" => 2,
        "gul" => 1,
        _ => 0,
    }
}

/// Det mærke, der skal meldes nu: det ældste "Ligner målrettet", der er afgjort og ikke meldt (ét pr. runde, så hvert mærke får sin egen besked), ellers en ny episode af "Ligner rekognoscering",
/// hvis det er slået til. Aldrig under Rolig: den er kun rødt, højst én om dagen.
fn maerke_melder<'a>(r: &'a Raekke1, ind: &Indst, regel: Regel, meldt: &Meldt) -> Option<&'a Moenster> {
    if regel == Regel::Rolig {
        return None;
    }
    let er_meldt = |m: &&Moenster| meldt.get(&(r.id.clone(), m.dag.clone())).is_some_and(|x| laes_meldt(x).1);
    let rekog = r
        .moenster
        .as_ref()
        .filter(|m| r.afgjort.is_none() && m.slags.er_rekognoscering() && m.ny && ind.rekognoscering);
    r.efterslaeb.iter().chain(&r.afgjort).chain(rekog).find(|m| !er_meldt(m))
}

/// Niveauet i `meldt` uden mærket, og om mærket er meldt.
fn laes_meldt(x: &str) -> (&str, bool) {
    x.strip_suffix(MAERKE).map_or((x, false), |n| (n, true))
}

/// (zone, statusdøgn) → det højeste meldte niveau, måske med mærket.
pub type Meldt = HashMap<(String, String), String>;

/// Det, der kan noteres som vurderet efter en runde: (zone, døgn). Er der et mærke tilbage at melde, står domænet stille til næste runde. Ren funktion.
pub fn vurderede(rk: &[Raekke1], ind: &Indst, meldt: &Meldt) -> Vec<(String, String)> {
    rk.iter()
        .filter(|r| regel(ind, &r.id).is_none_or(|g| maerke_melder(r, ind, g, meldt).is_none()))
        .filter_map(|r| r.vurderet.clone().map(|d| (r.id.clone(), d)))
        .collect()
}

/// Er der noget at sige? Ren funktion.
pub fn beslut(rk: &[Raekke1], ind: &Indst, meldt: &Meldt, nu: i64) -> Option<Besked> {
    let efter_natten = dag_ms(&dag_noegle(nu)).is_some_and(|m| nu - m >= NAT_MS);
    let rolig_tilladt = efter_natten && ind.rolig_sidst.as_deref() != Some(ind.tz.dag(nu).as_str());
    let mut fund = Vec::new();
    for r in rk
        .iter()
        .filter(|r| usaedvanlig(r) || !r.tek_besked.is_empty() || (!r.gammel && (r.moenster.is_some() || !r.efterslaeb.is_empty())))
    {
        let Some(regel) = regel(ind, &r.id) else { continue };
        let udloeser = usaedvanlig(r)
            && match r.status {
                Status::Roed => ind.roed,
                Status::Gul => regel == Regel::Alt,
                _ => false,
            };
        let bedt = !r.tek_besked.is_empty();
        let tilladt = udloeser && (regel != Regel::Rolig || rolig_tilladt);
        let maerke_ny = maerke_melder(r, ind, regel, meldt);
        if !bedt && !tilladt && maerke_ny.is_none() {
            continue;
        }
        let dag = statusdoegn(r, nu);
        let foer = |d: &str| meldt.get(&(r.id.clone(), d.to_string())).map(|x| laes_meldt(x));
        let status_ny = match foer(&dag).map(|x| x.0) {
            None => tilladt || bedt,
            Some("groen") => tilladt,
            Some("gul") => r.status == Status::Roed && tilladt,
            Some(_) => false,
        };
        if !status_ny && maerke_ny.is_none() {
            continue;
        }
        let hoejest = |a: &'static str, b: Option<&str>| b.filter(|n| rang(n) > rang(a)).map_or(a.to_string(), str::to_string);
        let nu_niveau = status_kode(r.status);
        let mut meldes = Vec::new();
        if status_ny || maerke_ny.is_some_and(|m| m.dag == dag) {
            let (niveau, har_maerke) = foer(&dag).map_or((None, false), |x| (Some(x.0), x.1));
            let med = har_maerke || maerke_ny.is_some_and(|m| m.dag == dag);
            meldes.push((
                dag.clone(),
                format!("{}{}", hoejest(nu_niveau, niveau), if med { MAERKE } else { "" }),
            ));
        }
        if let Some(m) = maerke_ny.filter(|m| m.dag != dag) {
            let niveau = foer(&m.dag).map(|x| x.0);
            meldes.push((m.dag.clone(), format!("{}{MAERKE}", hoejest("groen", niveau))));
        }
        fund.push(Fund {
            dag,
            rolig: regel == Regel::Rolig && !bedt && status_ny,
            r: r.clone(),
            maerke: maerke_ny.cloned(),
            meldes,
        });
    }
    if fund.is_empty() {
        return None;
    }
    fund.sort_by(|a, b| a.r.status.cmp(&b.r.status).then_with(|| a.r.navn.cmp(&b.r.navn)));
    let rolig = fund.iter().any(|f| f.rolig);
    Some(Besked { fund, rolig })
}

// ── Tekster (da/en) ────────────────────────────────────────────────────────────────────────────────────────────────
/// Et tal som `toLocaleString` i webvisningen (da-DK / en-GB): tusindtalsskille og højst én decimal.
pub fn tal(x: f64, sprog: Sprog) -> String {
    let (tusind, komma) = match sprog {
        Sprog::Da => ('.', ','),
        Sprog::En => (',', '.'),
    };
    let tiendedele = (x.abs() * 10.0).round() as i64;
    let hele = (tiendedele / 10).to_string();
    let mut ud = String::new();
    for (i, c) in hele.chars().enumerate() {
        if i > 0 && (hele.len() - i).is_multiple_of(3) {
            ud.push(tusind);
        }
        ud.push(c);
    }
    if tiendedele % 10 != 0 {
        ud.push(komma);
        ud.push_str(&(tiendedele % 10).to_string());
    }
    if x < 0.0 && tiendedele != 0 {
        ud.insert(0, '−');
    }
    ud
}

/// Ental eller flertal efter et tal.
pub fn flertal<'a>(n: i64, en: &'a str, flere: &'a str) -> &'a str {
    if n == 1 {
        en
    } else {
        flere
    }
}

fn n(x: i64, sprog: Sprog) -> String {
    tal(x as f64, sprog)
}

/// Regelnavnet uden CVE-nummer til sidst (" (CVE-2025-55182)"), som `udenCve` i visning-overblik.js.
fn uden_cve(navn: &str) -> &str {
    match navn.rfind(" (CVE-") {
        Some(i) if navn.ends_with(')') && !navn[i + 2..navn.len() - 1].contains(')') => &navn[..i],
        _ => navn,
    }
}

/// Ordlisten: Cloudflares regelnavne (uden CVE-nummer) → klarsprog. (id, mønster, dansk, engelsk, råd da, råd en).
/// Samme poster og rækkefølge som `ORDLISTE` i ui/assets/sprog.js; UI-røgtesten sammenligner dem post for post.
/// Første træf vinder; ukendte navne står, som de er (id "andet").
pub const ORDLISTE: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "react",
        "(?i)react",
        "et kendt hul i React",
        "a known hole in React",
        "Bed din webmand tjekke, at siden er opdateret.",
        "Ask your web person to check that the site is up to date.",
    ),
    (
        "react",
        "(?i)next\\.?js",
        "et kendt hul i Next.js",
        "a known hole in Next.js",
        "Bed din webmand tjekke, at siden er opdateret.",
        "Ask your web person to check that the site is up to date.",
    ),
    (
        "log4j",
        "(?i)log4j",
        "et kendt hul i Log4j (Java)",
        "a known hole in Log4j (Java)",
        "Bed din webmand tjekke, at Java-programmerne er opdateret.",
        "Ask your web person to check that Java software is up to date.",
    ),
    (
        "wordpress",
        "(?i)wordpress|\\bwp[-_ ]",
        "angreb på WordPress",
        "attacks on WordPress",
        "Bed din webmand tjekke, at WordPress og plugins er opdateret.",
        "Ask your web person to check that WordPress and its plugins are up to date.",
    ),
    (
        "browsertjek",
        "(?i)browser integrity",
        "browsertjekket",
        "the browser check",
        "",
        "",
    ),
    (
        "sqli",
        "(?i)sql ?injection|sqli",
        "forsøg på at snige databasekommandoer ind",
        "attempts to sneak in database commands",
        "",
        "",
    ),
    (
        "xss",
        "(?i)xss|cross.site",
        "forsøg på at plante kode i andres browser",
        "attempts to plant code in other people’s browsers",
        "",
        "",
    ),
    (
        "traversal",
        "(?i)path traversal|file inclusion|\\blfi\\b",
        "forsøg på at åbne filer, de ikke må",
        "attempts to open files they should not",
        "",
        "",
    ),
    (
        "rce",
        "(?i)remote code execution|\\brce\\b",
        "forsøg på at køre kode på serveren",
        "attempts to run code on the server",
        "",
        "",
    ),
];

static MOENSTRE: LazyLock<Vec<Regex>> = LazyLock::new(|| ORDLISTE.iter().map(|o| Regex::new(o.1).expect("ordlistens mønster")).collect());

/// Et regelnavn i klarsprog: kategori (til handlingsbogen i UI'et), tekst og evt. råd.
#[derive(Debug, PartialEq)]
pub struct Klar {
    pub id: &'static str,
    pub tekst: String,
    pub raad: &'static str,
}

/// En egen regels korte navn: op til første ":" eller " – ", højst 24 tegn (ellers 23 + "…"). Resten står kun i title.
pub fn kort_egen(navn: &str) -> String {
    let slut = [navn.find(':'), navn.find(" – ")].into_iter().flatten().min().unwrap_or(navn.len());
    let kort = navn[..slut].trim();
    if kort.chars().count() > 24 {
        format!("{}…", kort.chars().take(23).collect::<String>())
    } else {
        kort.to_string()
    }
}

/// Regelnavn → klarsprog, som `klar` i sprog.js. Egne regler først (deres navn kan ligne en kendt regel).
pub fn klar(navn: &str, egen: bool, sprog: Sprog) -> Klar {
    let da = sprog == Sprog::Da;
    if egen {
        let k = kort_egen(navn);
        let tekst = if da {
            format!("din egen regel »{k}«")
        } else {
            format!("your own rule “{k}”")
        };
        return Klar {
            id: "egen",
            tekst,
            raad: "",
        };
    }
    match ORDLISTE.iter().zip(MOENSTRE.iter()).find(|(_, m)| m.is_match(navn)) {
        Some(((id, _, d, e, rd, re), _)) => Klar {
            id,
            tekst: (if da { *d } else { *e }).to_string(),
            raad: if da { rd } else { re },
        },
        None => Klar {
            id: "andet",
            tekst: navn.to_string(),
            raad: "",
        },
    }
}

/// De største angrebstyper i klarsprog; typer med samme tekst lægges sammen (fx to WordPress-regler), sorteret efter
/// summen (stabilt). Ren funktion.
pub fn flet(r: &Raekke1, sprog: Sprog) -> Vec<(Klar, i64)> {
    let mut ud: Vec<(Klar, i64)> = Vec::new();
    for t in &r.angreb_i_gaar_top {
        let k = klar(uden_cve(&t.navn), t.egen, sprog);
        match ud.iter_mut().find(|(x, _)| x.tekst == k.tekst) {
            Some((_, n)) => *n += t.n,
            None => ud.push((k, t.n)),
        }
    }
    ud.sort_by_key(|x| std::cmp::Reverse(x.1));
    ud
}

/// Teksten siger "i forgårs": statusdøgnet er rykket om natten, eller det står to lokale dage tilbage.
fn forgaars(r: &Raekke1) -> bool {
    r.opdateres || r.forgaars
}

/// "Din server svarer ikke" som i Overblikket (`server` i visning-overblik.js), med mellemrum foran; tom uden fejl. Forstyrrelsen står kun i
/// programmet: landets navn kendes kun i webvisningen.
fn server_tekst(r: &Raekke1, sprog: Sprog) -> String {
    let Some(s) = &r.server else { return String::new() };
    let (p, svar) = (n(s.procent, sprog), n(s.svar, sprog));
    match (sprog == Sprog::Da, forgaars(r)) {
        (true, nat) => format!(
            " Din server svarer ikke: {p} % af {svar} svar {} var fejl 520–527, så Cloudflare kunne ikke få svar fra din server. Se »Hvad gør jeg?«.",
            if nat { "i forgårs" } else { "i går" }
        ),
        (false, nat) => format!(
            " Your server is not answering: {p}% of {svar} responses {} were errors 520–527, so Cloudflare could not get an answer from your server. See “What do I do?”.",
            if nat { "the day before yesterday" } else { "yesterday" }
        ),
    }
}

/// "Usædvanlig meget trafik" som i Overblikket (`trafik` i visning-overblik.js), med mellemrum foran; tom uden spring. De stoppede er angreb; det er de øvrige, der ikke tæller som angreb. Sætningen siger,
/// hvad tallene kan bære: Cloudflares manglende reaktion viser kun, at Cloudflare ikke behandlede dem som angreb. Stoppede Cloudflare mindst halvdelen, siger den det.
fn trafik_tekst(r: &Raekke1, sprog: Sprog) -> String {
    let Some(t) = &r.trafik else { return String::new() };
    let rest = t.n - t.stoppet;
    let (x, m, s, o) = (
        n(t.n, sprog),
        n(t.median.round() as i64, sprog),
        n(t.stoppet, sprog),
        n(rest, sprog),
    );
    let mest = t.stoppet * 2 >= t.n;
    match (sprog == Sprog::Da, mest) {
        (true, true) => format!(
            " Usædvanlig meget trafik: {x} {} (normalt {m}). Det meste af trafikspringet var angreb, som Cloudflare stoppede. Det var {s}. {} {o} slap igennem og tæller ikke som angreb. Se, hvor trafikken kom fra.",
            flertal(t.n, "forespørgsel", "forespørgsler"),
            flertal(rest, "Den sidste", "De øvrige")
        ),
        (true, false) => format!(
            " Usædvanlig meget trafik: {x} {} (normalt {m}). Cloudflare stoppede {s} som angreb. {} {o} slap igennem og tæller ikke som angreb — det kan være et forsøg på at overbelaste siden (DDoS), en robot, der løber løbsk, eller en fejl. Se, hvor trafikken kom fra.",
            flertal(t.n, "forespørgsel", "forespørgsler"),
            flertal(rest, "Den sidste", "De øvrige")
        ),
        (false, true) => format!(
            " Unusually high traffic: {x} {} (normally {m}). Most of the traffic spike was attacks that Cloudflare stopped. That was {s}. The other {o} got through and {}. See where the traffic came from.",
            flertal(t.n, "request", "requests"),
            flertal(rest, "does not count as an attack", "do not count as attacks")
        ),
        (false, false) => format!(
            " Unusually high traffic: {x} {} (normally {m}). Cloudflare stopped {s} as {}. The other {o} got through and {} — it may be an attempt to overload the site (DDoS), a robot running wild, or a fault. See where the traffic came from.",
            flertal(t.n, "request", "requests"),
            flertal(t.stoppet, "an attack", "attacks"),
            flertal(rest, "does not count as an attack", "do not count as attacks")
        ),
    }
}

/// DMARC (`dmarc` i visning-overblik.js), med mellemrum foran. Aldrig en IP-adresse.
fn dmarc_tekst(r: &Raekke1, sprog: Sprog) -> String {
    let Some(f) = &r.dmarc else { return String::new() };
    let da = sprog == Sprog::Da;
    let mut ud = String::new();
    if !f.nye.is_empty() {
        let liste = f.nye.join(", ");
        ud += &if da {
            format!(" Mails i dit domænes navn fejlede DMARC fra en ny afsender: {liste}.")
        } else {
            format!(" Mail in your domain's name failed DMARC from a new sender: {liste}.")
        };
    }
    if f.stiger {
        ud += &if da {
            format!(
                " {} % af mails i dit domænes navn fejlede DMARC (normalt {} %).",
                n(f.procent, sprog),
                n(f.normalt, sprog)
            )
        } else {
            format!(
                " {}% of mail in your domain's name failed DMARC (usually {}%).",
                n(f.procent, sprog),
                n(f.normalt, sprog)
            )
        };
    }
    ud
}

/// Lækkede kodeord i statusdøgnet (`laek` i visning-overblik.js), med mellemrum foran. Aldrig stien.
fn laek_tekst(r: &Raekke1, sprog: Sprog) -> String {
    let Some(l) = &r.laek else { return String::new() };
    let (a, b) = (n(l.n, sprog), n(l.naaet, sprog));
    let dag = match (sprog == Sprog::Da, forgaars(r)) {
        (true, false) => "i går",
        (true, true) => "i forgårs",
        (false, false) => "yesterday",
        (false, true) => "the day before yesterday",
    };
    match (sprog == Sprog::Da, l.naaet > 0) {
        (true, false) => format!(" {a} {} brugte et kodeord fra et kendt læk {dag}.", flertal(l.n, "login-forsøg", "login-forsøg")),
        (true, true) => format!(
            " {a} {} brugte et kodeord fra et kendt læk {dag}, og {b} af dem nåede din server og fik et svar, der kan ligne et login.",
            flertal(l.n, "login-forsøg", "login-forsøg")
        ),
        (false, false) => format!(" {a} login {} used a password from a known leak {dag}.", flertal(l.n, "attempt", "attempts")),
        (false, true) => format!(
            " {a} login {} used a password from a known leak {dag}, and {b} of them reached your server and got an answer that can look like a login.",
            flertal(l.n, "attempt", "attempts")
        ),
    }
}

/// Domænets tilstand hos Cloudflare (`tilstand` i visning-overblik.js), uden mellemrum foran.
fn tilstand_tekst(r: &Raekke1, sprog: Sprog) -> String {
    let da = sprog == Sprog::Da;
    let linje = |f: &crate::overblik::TilstandFund| match (f.slags, da) {
        ("pause", true) => "Domænet er sat på pause — Cloudflare beskytter det ikke.".to_string(),
        ("pause", false) => "The domain is paused — Cloudflare is not protecting it.".to_string(),
        ("phishing", true) => "Cloudflare mistænker phishing på domænet.".to_string(),
        ("phishing", false) => "Cloudflare suspects phishing on the domain.".to_string(),
        ("udvikler", true) => format!(
            "Udviklertilstand er slået til — cachen er slået fra, til kl. {}.",
            f.kl.as_deref().unwrap_or("")
        ),
        ("udvikler", false) => format!("Development Mode is on — the cache is off until {}.", f.kl.as_deref().unwrap_or("")),
        (_, true) => format!(
            "Domænet er ikke aktivt hos Cloudflare (status: {}).",
            f.status.as_deref().unwrap_or("")
        ),
        (_, false) => format!(
            "The domain is not active at Cloudflare (status: {}).",
            f.status.as_deref().unwrap_or("")
        ),
    };
    r.tilstand.iter().map(linje).collect::<Vec<_>>().join(" ")
}

/// Det, valgene under "Dit sites teknologi" gør ved "Hvorfor" (`teknologi` i visning-overblik.js), med mellemrum foran.
fn tek_tekst(r: &Raekke1, sprog: Sprog) -> String {
    let da = sprog == Sprog::Da;
    let mut t = String::new();
    if !r.ikke_brugt.is_empty() {
        // Flere teknologier står i én sætning, som `teknologi` i visning-overblik.js.
        let sum: i64 = r.ikke_brugt.iter().map(|x| x.n).sum();
        let navne: Vec<String> = r.ikke_brugt.iter().map(|x| teknologi::navn(&x.tek)).collect();
        let (a, og) = (n(sum, sprog), if da { " og " } else { " and " });
        let navn = match navne.split_last() {
            Some((sidste, foran)) if !foran.is_empty() => format!("{}{og}{sidste}", foran.join(", ")),
            _ => navne.join(""),
        };
        t.push_str(&if da {
            format!(" {a} angreb på {navn}, som du ikke bruger — ufarlige for dig.")
        } else {
            format!(
                " {a} {} on {navn}, which you do not use — harmless to you.",
                flertal(sum, "attack", "attacks")
            )
        });
    }
    if let Some(u) = &r.uventet {
        let navn = teknologi::navn(&u.tek);
        t.push_str(&if da {
            format!(" Uventet: din server svarede på en {navn}-sti. Bruger du alligevel {navn}?")
        } else {
            format!(" Unexpected: your server answered on a {navn} path. Do you use {navn} after all?")
        });
    }
    if let Some(x) = &r.naaet {
        let (a, navn) = (n(x.n, sprog), teknologi::navn(&x.tek));
        t.push_str(&if da {
            format!(" {a} angreb på {navn} nåede din server, og du har sagt, at du bruger {navn}.")
        } else {
            format!(
                " {a} {} on {navn} reached your server, and you have said you use {navn}.",
                flertal(x.n, "attack", "attacks")
            )
        });
    }
    for b in &r.tek_besked {
        let (a, navn, s) = (n(b.n, sprog), teknologi::navn(&b.tek), tal(b.snit, sprog));
        t.push_str(&if da {
            format!(" Flere angreb på {navn} end normalt: {a} (snit {s}).")
        } else {
            format!(" More attacks on {navn} than usual: {a} (average {s}).")
        });
    }
    t
}

/// Hvad DNS-mærkets tal tæller (`moensterDns` og `moensterDnsNavne` i sprog.js).
fn dns_ord(m: &Moenster, da: bool) -> String {
    match (m.dns_navne, da) {
        (true, true) => format!("{}, der ikke findes,", flertal(m.stier, "navn", "forskellige navne")),
        (false, true) => "opslag på navne, der ikke findes,".to_string(),
        (true, false) => format!("different non-existent {}", flertal(m.stier, "name", "names")),
        (false, false) => format!("{} of non-existent names", flertal(m.stier, "lookup", "lookups")),
    }
}

/// Mærket som i Overblikket (`moensterTekst` i sprog.js). Altid "ligner"/"looks": tallene kan ikke mere. Ren funktion.
pub fn moenster_tekst(m: &Moenster, sprog: Sprog) -> String {
    let da = sprog == Sprog::Da;
    let tek = m.tek.as_deref().map(teknologi::navn);
    let foreloebig = match (m.foreloebig, da) {
        (false, _) => "",
        (true, true) => " Foreløbig — afgøres, når i morgen er hentet.",
        (true, false) => " Provisional — decided when tomorrow is fetched.",
    };
    let slut = match (tek, da) {
        (Some(t), true) => format!(", og angrebene ramte {t}, som dit site bruger"),
        (Some(t), false) => format!(", and the attacks hit {t}, which your site uses"),
        (None, _) => String::new(),
    };
    match (m.slags, da) {
        (MoensterSlags::Rekognoscering, true) => format!(
            "Ligner rekognoscering: {} forskellige stier på én dag (normalt {}){}{slut}.",
            n(m.stier, sprog),
            n(m.normalt.round() as i64, sprog),
            if m.scanner { ", som en almindelig scanner" } else { "" }
        ),
        (MoensterSlags::Rekognoscering, false) => format!(
            "Looks like reconnaissance: {} different paths in one day (usually {}){}{slut}.",
            n(m.stier, sprog),
            n(m.normalt.round() as i64, sprog),
            if m.scanner { ", like an ordinary scanner" } else { "" }
        ),
        (MoensterSlags::DnsRekognoscering, true) => format!(
            "Ligner DNS-rekognoscering: {} {} på én dag (normalt {}).",
            n(m.stier, sprog),
            dns_ord(m, true),
            n(m.normalt.round() as i64, sprog)
        ),
        (MoensterSlags::DnsRekognoscering, false) => format!(
            "Looks like DNS reconnaissance: {} {} in one day (usually {}).",
            n(m.stier, sprog),
            dns_ord(m, false),
            n(m.normalt.round() as i64, sprog)
        ),
        (MoensterSlags::Maalrettet, true) => format!(
            "Ligner målrettet: angrebene steg her, men ikke {}, og {} % kom fra ét netværk{slut}.{foreloebig}",
            if m.verden {
                "i resten af verden"
            } else {
                "på dine andre domæner"
            },
            n(m.procent, sprog)
        ),
        (MoensterSlags::Maalrettet, false) => format!(
            "Looks targeted: attacks rose here but not {}, and {}% came from one network{slut}.{foreloebig}",
            if m.verden { "elsewhere" } else { "on your other domains" },
            n(m.procent, sprog)
        ),
    }
}

/// Notifikationen for et mærke (Q5). "Cloudflare har stoppet dem" står kun, når det dækker hvert angreb.
fn moenster_besked(r: &Raekke1, m: &Moenster, sprog: Sprog, dato_fmt: &DatoFormat) -> (String, String) {
    let da = sprog == Sprog::Da;
    let (k, a) = (m.stoppet, m.angreb);
    match m.slags {
        MoensterSlags::Maalrettet => {
            let hvor = match (m.verden, da) {
                (true, true) => "i resten af verden",
                (false, true) => "på dine andre domæner",
                (true, false) => "elsewhere",
                (false, false) => "on your other domains",
            };
            if da {
                let stoppet = if k == a {
                    "Cloudflare har stoppet dem.".to_string()
                } else {
                    format!("Cloudflare har stoppet {} af {}.", n(k, sprog), n(a, sprog))
                };
                (
                    "Ligner et målrettet angreb".into(),
                    format!(
                        "{}: angrebene steg her den {}, men ikke {hvor}. {stoppet} Det er ikke sikkert, at nogen sigter på dig.",
                        r.navn,
                        dato(&m.dag, dato_fmt, sprog)
                    ),
                )
            } else {
                let stoppet = if k == a {
                    "Cloudflare stopped them.".to_string()
                } else {
                    format!("Cloudflare stopped {} of {}.", n(k, sprog), n(a, sprog))
                };
                (
                    "Looks like a targeted attack".into(),
                    format!(
                        "{}: attacks rose here on {} but not {hvor}. {stoppet} It is not certain that anyone is aiming at you.",
                        r.navn,
                        dato(&m.dag, dato_fmt, sprog)
                    ),
                )
            }
        }
        MoensterSlags::DnsRekognoscering => {
            let (x, y, d) = (n(m.stier, sprog), n(m.normalt.round() as i64, sprog), dato(&m.dag, dato_fmt, sprog));
            if da {
                (
                    "Ligner DNS-rekognoscering".into(),
                    format!(
                        "{}: {x} {} den {d} (normalt {y}). Nogen gætter på, hvad der findes under domænet.",
                        r.navn,
                        dns_ord(m, true)
                    ),
                )
            } else {
                (
                    "Looks like DNS reconnaissance".into(),
                    format!(
                        "{}: {x} {} on {d} (usually {y}). Someone is guessing what exists under the domain.",
                        r.navn,
                        dns_ord(m, false)
                    ),
                )
            }
        }
        MoensterSlags::Rekognoscering => {
            let (x, y, d) = (n(m.stier, sprog), n(m.normalt.round() as i64, sprog), dato(&m.dag, dato_fmt, sprog));
            if da {
                (
                    "Ligner rekognoscering".into(),
                    format!(
                        "{}: {x} forskellige stier den {d} (normalt {y}). Det er oftest en automatisk scanner.",
                        r.navn
                    ),
                )
            } else {
                (
                    "Looks like reconnaissance".into(),
                    format!(
                        "{}: {x} different paths on {d} (usually {y}). It is usually an automated scanner.",
                        r.navn
                    ),
                )
            }
        }
    }
}

/// "Hvorfor" — samme tekst som i Overblikket (`hvorfor` i visning-overblik.js), fx "30 angreb i går (snit 12,4) – alle
/// stoppet. Mest et kendt hul i React (20) og angreb på WordPress (7). Bed din webmand …". Uden historik "(første
/// gang)". Stoppet = blokeret, udfordret eller lokket ind i labyrinten (J9: `labyrint` er kun `link_maze_visited`); "alle
/// stoppet" kun, når det dækker hvert angreb. Ren funktion.
pub fn aarsag(r: &Raekke1, sprog: Sprog) -> String {
    let (foran, resten) = (tilstand_tekst(r, sprog), aarsag_tal(r, sprog));
    if foran.is_empty() {
        resten
    } else {
        format!("{foran} {resten}")
    }
}

fn aarsag_tal(r: &Raekke1, sprog: Sprog) -> String {
    let da = sprog == Sprog::Da;
    let opdateres = if da { "Opdateres …" } else { "Updating …" };
    if r.status == Status::Graa {
        return (if da {
            "Ingen tal endnu – henter."
        } else {
            "No figures yet – fetching."
        })
        .into();
    }
    if r.gammel {
        return (if da {
            "Tallene er gamle – henter nye."
        } else {
            "The figures are old – fetching new ones."
        })
        .into();
    }
    let (f, a) = (&r.angreb_fordeling, r.angreb_i_gaar);
    let maerke = r
        .moenster
        .as_ref()
        .map_or_else(String::new, |m| format!(" {}", moenster_tekst(m, sprog)));
    let server = format!(
        "{}{}{}{}{}{maerke}",
        tek_tekst(r, sprog),
        laek_tekst(r, sprog),
        dmarc_tekst(r, sprog),
        trafik_tekst(r, sprog),
        server_tekst(r, sprog)
    );
    let hale = if r.opdateres { format!(" {opdateres}") } else { String::new() };
    if a == 0 {
        return match (da, forgaars(r)) {
            (true, false) => format!("Ingen angreb i går.{server}{hale}"),
            (false, false) => format!("No attacks yesterday.{server}{hale}"),
            (true, true) => format!("Ingen angreb i forgårs.{server}{hale}"),
            (false, true) => format!("No attacks the day before yesterday.{server}{hale}"),
        };
    }
    let snit = match (r.angreb_snit == 0.0, da) {
        (true, true) => "første gang".to_string(),
        (true, false) => "first time".to_string(),
        (false, true) => format!("snit {}", tal(r.angreb_snit, sprog)),
        (false, false) => format!("average {}", tal(r.angreb_snit, sprog)),
    };
    let dag = match (da, forgaars(r)) {
        (true, false) => "angreb i går",
        (true, true) => "angreb i forgårs",
        (false, false) => flertal(a, "attack yesterday", "attacks yesterday"),
        (false, true) => flertal(a, "attack the day before yesterday", "attacks the day before yesterday"),
    };
    let alle = if da {
        flertal(a, "stoppet", "alle stoppet")
    } else {
        flertal(a, "stopped", "all stopped")
    };
    let stoppet = if f.blokeret == a {
        alle.to_string()
    } else {
        let mut dele = vec![
            format!("{} {}", n(f.blokeret, sprog), if da { "blokeret" } else { "blocked" }),
            format!("{} {}", n(f.udfordret, sprog), if da { "udfordret" } else { "challenged" }),
        ];
        if f.labyrint != 0 {
            dele.push(format!(
                "{} {}",
                n(f.labyrint, sprog),
                if da { "i labyrinten" } else { "in the labyrinth" }
            ));
        }
        if f.andet + f.faelder != 0 {
            dele.push(format!("{} {}", n(f.andet + f.faelder, sprog), if da { "andet" } else { "other" }));
        }
        let k = f.blokeret + f.udfordret + f.labyrint;
        let hoved = match (k == a, da) {
            (true, _) => alle.to_string(),
            (false, true) => format!("{} af {} stoppet", n(k, sprog), n(a, sprog)),
            (false, false) => format!("{} of {} stopped", n(k, sprog), n(a, sprog)),
        };
        format!("{hoved} ({})", dele.join(", "))
    };
    let mut t = format!("{} {dag} ({snit}) – {stoppet}.", n(a, sprog));
    // Står før reglerne, så den ikke læses som en påstand om en bestemt regel: Radar kender kun WAF-angreb generelt.
    match (r.tendens, da, forgaars(r)) {
        (Some(Tendens::Foelger), true, _) => t.push_str(" Følger den globale tendens."),
        (Some(Tendens::Foelger), false, _) => t.push_str(" Follows the global trend."),
        (Some(Tendens::SaerligtDig), true, false) => t.push_str(" Verden var normal i går, så stigningen er din egen."),
        (Some(Tendens::SaerligtDig), true, true) => t.push_str(" Verden var normal i forgårs, så stigningen er din egen."),
        (Some(Tendens::SaerligtDig), false, false) => t.push_str(" The world was normal yesterday, so the rise is yours."),
        (Some(Tendens::SaerligtDig), false, true) => t.push_str(" The world was normal the day before yesterday, so the rise is yours."),
        (None, ..) => {}
    }
    let typer = flet(r, sprog);
    if !typer.is_empty() {
        let dele: Vec<String> = typer.iter().map(|(k, x)| format!("{} ({})", k.tekst, n(*x, sprog))).collect();
        let liste = match dele.split_last() {
            Some((sidste, foer)) if !foer.is_empty() => format!("{}{}{sidste}", foer.join(", "), if da { " og " } else { " and " }),
            _ => dele.join(""),
        };
        t.push_str(&if da {
            format!(" Mest {liste}.")
        } else {
            format!(" Mostly {liste}.")
        });
        let bruges = !r.ikke_brugt.iter().any(|x| {
            r.angreb_i_gaar_top
                .iter()
                .any(|t| t.tek.as_ref() == Some(&x.tek) && klar(uden_cve(&t.navn), t.egen, sprog).tekst == typer[0].0.tekst)
        });
        if matches!(r.status, Status::Roed | Status::Gul) && !typer[0].0.raad.is_empty() && bruges {
            t.push(' ');
            t.push_str(typer[0].0.raad);
        }
    }
    t.push_str(&server);
    if r.opdateres {
        t.push(' ');
        t.push_str(opdateres);
    }
    t
}

/// Datoformatet til notifikationer (`DATO_FORMAT`). `moenster` er systemets, som webvisningen har læst.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DatoFormat {
    pub format: String,
    pub moenster: String,
}

pub fn dato_format(lager: &Store) -> Result<DatoFormat, Fejl> {
    Ok(DatoFormat {
        format: lager.indstilling(DATO_FORMAT)?.unwrap_or_default(),
        moenster: lager.indstilling(DATO_MOENSTER)?.unwrap_or_default(),
    })
}

/// Samme korte måneder som `MND` i ui/assets/sprog.js.
const MND_DA: [&str; 12] = [
    "jan.", "feb.", "mar.", "apr.", "maj", "jun.", "jul.", "aug.", "sep.", "okt.", "nov.", "dec.",
];
const MND_EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// En dag ("2026-11-04") som `S.dato` i sprog.js: da "4. nov. 2026", en "4 Nov 2026", ISO som den er, og systemets efter mønstret (d, dd, M, MM, yy, yyyy). Ren funktion.
pub fn dato(dag: &str, f: &DatoFormat, sprog: Sprog) -> String {
    let mut dele = dag.split('-');
    let (Some(a), Some(m), Some(d)) = (dele.next(), dele.next(), dele.next()) else {
        return dag.to_string();
    };
    let (Ok(mi), Ok(di)) = (m.parse::<usize>(), d.parse::<usize>()) else {
        return dag.to_string();
    };
    if f.format == "iso" || !(1..=12).contains(&mi) {
        return dag.to_string();
    }
    if f.format == "system" && !f.moenster.is_empty() {
        return f
            .moenster
            .replace("yyyy", a)
            .replace("yy", a.get(2..).unwrap_or(a))
            .replace("MM", m)
            .replace('M', &mi.to_string())
            .replace("dd", d)
            .replace('d', &di.to_string());
    }
    match sprog {
        Sprog::Da => format!("{di}. {} {a}", MND_DA[mi - 1]),
        Sprog::En => format!("{di} {} {a}", MND_EN[mi - 1]),
    }
}

/// Notifikationens titel og tekst. Ét domæne: navnet og linjen; flere: antallet og de første `NAEVN`. Ren funktion.
pub fn tekst(b: &Besked, sprog: Sprog, dato_fmt: &DatoFormat) -> (String, String) {
    let da = sprog == Sprog::Da;
    if let [f] = b.fund.as_slice() {
        let r = &f.r;
        if let Some(m) = f.moenster() {
            return moenster_besked(r, m, sprog, dato_fmt);
        }
        // Titlen siger, hvad domænet er på listen for: gul fra serveren, trafikken eller sikkerhedsnettet er ikke flere angreb.
        let titel = match (r.status, r.angreb_status, da) {
            _ if !r.tilstand.is_empty() && da => format!("{}: domænet hos Cloudflare", r.navn),
            _ if !r.tilstand.is_empty() => format!("{}: the domain at Cloudflare", r.navn),
            _ if r.laek.is_some() && da => format!("{}: login med lækket kodeord", r.navn),
            _ if r.laek.is_some() => format!("{}: login with a leaked password", r.navn),
            (Status::Gul, Status::Groen, true) if r.dmarc.is_some() => format!("{}: mails i dit navn fejlede DMARC", r.navn),
            (Status::Gul, Status::Groen, false) if r.dmarc.is_some() => format!("{}: mail in your name failed DMARC", r.navn),
            (Status::Roed, _, _) if r.naaet.is_some() && da => format!("{}: et angreb nåede din server", r.navn),
            (Status::Roed, _, _) if r.naaet.is_some() => format!("{}: an attack reached your server", r.navn),
            (Status::Roed, _, true) => format!("{}: usædvanligt mange angreb", r.navn),
            (Status::Roed, _, false) => format!("{}: unusually many attacks", r.navn),
            (_, Status::Gul, true) => format!("{}: flere angreb end normalt", r.navn),
            (_, Status::Gul, false) => format!("{}: more attacks than usual", r.navn),
            _ if r.uventet.is_some() && da => format!("{}: din server svarede uventet", r.navn),
            _ if r.uventet.is_some() => format!("{}: your server answered unexpectedly", r.navn),
            _ if r.trafik.is_some() && da => format!("{}: usædvanlig meget trafik", r.navn),
            _ if r.trafik.is_some() => format!("{}: unusually high traffic", r.navn),
            _ if r.server.is_some() && da => format!("{}: din server svarer ikke", r.navn),
            _ if r.server.is_some() => format!("{}: your server is not answering", r.navn),
            _ if da => format!(
                "{}: flere angreb på {} end normalt",
                r.navn,
                teknologi::navn(r.tek_besked.first().map_or("", |b| &b.tek))
            ),
            _ => format!(
                "{}: more attacks on {} than usual",
                r.navn,
                teknologi::navn(r.tek_besked.first().map_or("", |b| &b.tek))
            ),
        };
        return (titel, aarsag(&f.r, sprog));
    }
    let antal = b.fund.len();
    let titel = if da {
        format!("{antal} domæner kræver opmærksomhed")
    } else {
        format!("{antal} domains need attention")
    };
    let mut dele: Vec<String> = b
        .fund
        .iter()
        .take(NAEVN)
        .map(|f| {
            let a = f.r.angreb_i_gaar;
            let ord = if da { "angreb" } else { flertal(a, "attack", "attacks") };
            let maerke = match (f.moenster().map(|m| m.slags), da) {
                (Some(MoensterSlags::Maalrettet), true) => " (ligner målrettet)",
                (Some(MoensterSlags::Maalrettet), false) => " (looks targeted)",
                (Some(MoensterSlags::Rekognoscering), true) => " (ligner rekognoscering)",
                (Some(MoensterSlags::Rekognoscering), false) => " (looks like reconnaissance)",
                (Some(MoensterSlags::DnsRekognoscering), true) => " (ligner DNS-rekognoscering)",
                (Some(MoensterSlags::DnsRekognoscering), false) => " (looks like DNS reconnaissance)",
                (None, _) => "",
            };
            format!("{} {} {ord}{maerke}", f.r.navn, n(a, sprog))
        })
        .collect();
    if antal > NAEVN {
        let k = antal - NAEVN;
        dele.push(if da { format!("+ {k} flere") } else { format!("+ {k} more") });
    }
    (titel, dele.join(" · "))
}

// ── "Siden sidst" ──────────────────────────────────────────────────────────────────────────────────────────────────
/// Skal kortet vise sig selv, når forsiden åbnes? Rolig: aldrig. Standard: første gang i døgnet (UTC), hvis der er rødt
/// eller gult. Alt: første gang i døgnet. Ren funktion.
pub fn vis_selv(niveau: Niveau, vist: Option<&str>, rk: &[Raekke1], nu: i64, tz: &Tidszone) -> bool {
    if niveau == Niveau::Rolig {
        return false;
    }
    if vist.and_then(parse_iso).is_some_and(|ms| tz.dag(ms) == tz.dag(nu)) {
        return false;
    }
    niveau == Niveau::Alt || rk.iter().any(|r| usaedvanlig(r) || (!r.gammel && r.afgjort.is_some()))
}

fn aftryk_linje(r: &Raekke1, nu: i64) -> String {
    let maerke = match r.moenster.as_ref().map(|m| (m.slags, m.foreloebig)) {
        Some((MoensterSlags::Maalrettet, true)) => "\tforeloebig",
        Some((MoensterSlags::Maalrettet, false)) => "\tmaalrettet",
        Some((MoensterSlags::Rekognoscering, _)) => "\trekognoscering",
        Some((MoensterSlags::DnsRekognoscering, _)) => "\tdns",
        None => "",
    };
    format!("{}\t{}\t{}{maerke}", r.id, statusdoegn(r, nu), status_kode(r.status))
}

/// Hvad der står på kortet (røde og gule domæner og mærker med statusdøgn): "zone\tdag\tstatus" og måske mærket pr. linje. Ren funktion.
pub fn aftryk(rk: &[Raekke1], nu: i64) -> String {
    let mut linjer: Vec<String> = rk.iter().filter(|r| naevnes(r)).map(|r| aftryk_linje(r, nu)).collect();
    linjer.sort();
    linjer.join("\n")
}

/// Domænerne, der er nye siden kortet sidst blev vist: røde/gule, der ikke stod på kortet med samme statusdøgn og
/// status. Er kortet aldrig vist, er alle røde/gule nye. Ren funktion.
pub fn nye(rk: &[Raekke1], forrige: Option<&str>, nu: i64) -> Vec<String> {
    let set: HashSet<&str> = forrige.map(|a| a.lines().collect()).unwrap_or_default();
    rk.iter()
        .filter(|r| naevnes(r))
        .filter(|r| !set.contains(aftryk_linje(r, nu).as_str()))
        .map(|r| r.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cf::Zone;
    use crate::overblik::{raekke, AngrebType, Fordeling, Hele, ServerFejl, TrafikDag};
    use crate::store::Raekke;
    use crate::teknologi::{TekBesked, TekN, Uventet};
    use crate::vaern::{iso, HENT_DAGE};

    /// 2026-11-03T08:08:00Z, som overblik.rs' tests.
    const NU: i64 = 1_793_693_280_000;
    const T: i64 = 3_600_000;

    fn kl(t: &str) -> i64 {
        parse_iso(t).unwrap()
    }

    fn r(id: &str, navn: &str, status: Status, a: i64, snit: f64) -> Raekke1 {
        Raekke1 {
            id: id.into(),
            navn: navn.into(),
            status,
            gammel: false,
            opdateres: false,
            i_gaar_dag: String::new(),
            status_dag: String::new(),
            uge_fra: String::new(),
            ugen_foer_fra: String::new(),
            ugen_foer_til: String::new(),
            hele: Hele::default(),
            aendring: None,
            angreb_i_gaar: a,
            angreb_i_gaar_top: vec![],
            angreb_fordeling: Fordeling {
                blokeret: a,
                ..Fordeling::default()
            },
            angreb_snit: snit,
            angreb_uge: 0,
            angreb_ugen_foer: 0,
            sidst_hentet: Some(iso(NU - T)),
            doegn_faerdige: 31,
            doegn_i_alt: 31,
            tendens: None,
            trafik: None,
            server: None,
            forstyrrelse: None,
            angreb_status: status,
            ikke_brugt: vec![],
            uventet: None,
            naaet: None,
            tek_besked: vec![],
            tek_spoerg: None,
            med7: 7,
            med30: 30,
            laek: None,
            dmarc: None,
            tilstand: vec![],
            moenster: None,
            afgjort: None,
            efterslaeb: vec![],
            vurderet: None,
            forgaars: false,
            kan_sammenligne: true,
        }
    }

    fn roed(id: &str) -> Raekke1 {
        r(id, &format!("{id}.wi24rd.com"), Status::Roed, 45, 12.0)
    }

    fn gul(id: &str) -> Raekke1 {
        r(id, &format!("{id}.wi24rd.com"), Status::Gul, 18, 5.0)
    }

    fn ind(niveau: Niveau) -> Indst {
        Indst {
            niveau,
            ..Indst::default()
        }
    }

    fn ids(b: &Option<Besked>) -> Vec<String> {
        b.as_ref()
            .map(|b| b.fund.iter().map(|f| f.r.id.clone()).collect())
            .unwrap_or_default()
    }

    fn intet() -> HashMap<(String, String), String> {
        HashMap::new()
    }

    /// `meldt` efter en besked, som `vurder` i lib.rs skriver det.
    fn efter(b: &Option<Besked>, mut meldt: HashMap<(String, String), String>) -> HashMap<(String, String), String> {
        for (z, d, n) in b.as_ref().map(Besked::meldte).unwrap_or_default() {
            meldt.insert((z, d), n);
        }
        meldt
    }

    fn maerke(slags: MoensterSlags) -> Moenster {
        Moenster {
            slags,
            dag: dag_noegle(NU - DOEGN_MS),
            stier: 240,
            normalt: 12.0,
            dns_navne: false,
            scanner: false,
            ny: true,
            netvaerk: Some("Mal AS".into()),
            procent: 75,
            verden: true,
            tek: None,
            angreb: 3,
            stoppet: 3,
            foreloebig: false,
        }
    }

    fn med(mut x: Raekke1, slags: MoensterSlags) -> Raekke1 {
        x.moenster = Some(maerke(slags));
        if slags == MoensterSlags::Maalrettet {
            x.afgjort = x.moenster.clone();
        }
        x
    }

    /// DNS-mærket er "Ligner rekognoscering" for notifikationen: fra som standard og kun første døgn i en episode.
    #[test]
    fn dns_maerket_folger_rekognoscering() {
        let r = med(groen("a"), MoensterSlags::DnsRekognoscering);
        let meldt = Meldt::new();
        let (til, fra) = (
            Indst {
                rekognoscering: true,
                ..Indst::default()
            },
            Indst::default(),
        );
        assert!(!fra.rekognoscering, "fra som standard");
        assert!(maerke_melder(&r, &fra, Regel::Alt, &meldt).is_none());
        assert!(maerke_melder(&r, &til, Regel::Alt, &meldt).is_some());
        let mut gammel = r.clone();
        gammel.moenster.as_mut().unwrap().ny = false;
        assert!(maerke_melder(&gammel, &til, Regel::Alt, &meldt).is_none());
        assert_eq!(
            moenster_tekst(r.moenster.as_ref().unwrap(), Sprog::Da),
            "Ligner DNS-rekognoscering: 240 opslag på navne, der ikke findes, på én dag (normalt 12)."
        );
        let m = Moenster {
            dns_navne: true,
            ..r.moenster.clone().unwrap()
        };
        assert_eq!(
            moenster_tekst(&m, Sprog::En),
            "Looks like DNS reconnaissance: 240 different non-existent names in one day (usually 12)."
        );
    }

    /// DMARC i Hvorfor-sætningen på begge sprog, uden en IP-adresse.
    #[test]
    fn dmarc_i_teksten() {
        let mut r = groen("a");
        assert_eq!(dmarc_tekst(&r, Sprog::Da), "");
        r.dmarc = Some(crate::dmarc::DmarcFund {
            dag: "2026-11-05".into(),
            nye: vec!["Ny A/S".into()],
            stiger: true,
            procent: 40,
            normalt: 10,
        });
        assert_eq!(
            dmarc_tekst(&r, Sprog::Da),
            " Mails i dit domænes navn fejlede DMARC fra en ny afsender: Ny A/S. 40 % af mails i dit domænes navn fejlede DMARC (normalt 10 %)."
        );
        assert!(dmarc_tekst(&r, Sprog::En).starts_with(" Mail in your domain's name failed DMARC from a new sender: Ny A/S."));
    }

    /// De stoppede er angreb, de øvrige er ikke. Stoppede Cloudflare mindst halvdelen, siger teksten det.
    #[test]
    fn trafikteksten_er_aerlig_om_de_stoppede() {
        let tekst = |n, s, sprog| {
            let x = Raekke1 {
                trafik: Some(TrafikDag {
                    dag: "2026-11-05".into(),
                    n,
                    median: 13.0,
                    stoppet: s,
                }),
                ..r("t", "t.wi24rd.com", Status::Roed, 0, 0.0)
            };
            trafik_tekst(&x, sprog)
        };
        assert_eq!(
            tekst(3200, 1800, Sprog::Da),
            " Usædvanlig meget trafik: 3.200 forespørgsler (normalt 13). Det meste af trafikspringet var angreb, som Cloudflare stoppede. Det var 1.800. De øvrige 1.400 slap igennem og tæller ikke som angreb. Se, hvor trafikken kom fra."
        );
        assert_eq!(
            tekst(3200, 1800, Sprog::En),
            " Unusually high traffic: 3,200 requests (normally 13). Most of the traffic spike was attacks that Cloudflare stopped. That was 1,800. The other 1,400 got through and do not count as attacks. See where the traffic came from."
        );
        assert!(tekst(3000, 1000, Sprog::Da)
            .contains("Cloudflare stoppede 1.000 som angreb. De øvrige 2.000 slap igennem og tæller ikke som angreb — det kan være"));
        assert!(tekst(100, 50, Sprog::Da).contains("Det meste"), "præcis halvdelen");
        assert!(!tekst(100, 49, Sprog::Da).contains("Det meste"));
        assert!(tekst(2, 1, Sprog::Da).contains("Den sidste 1 slap igennem"));
        assert!(tekst(2, 1, Sprog::En).contains("The other 1 got through and does not count as an attack"));
        assert!(tekst(3, 1, Sprog::En).contains("stopped 1 as an attack"));
        for s in [tekst(3200, 1800, Sprog::Da), tekst(3000, 1000, Sprog::Da)] {
            assert!(!s.contains("så de tæller ikke som angreb"));
        }
    }

    /// Stien når aldrig en notifikation, og tilstanden melder, også når tallene er gamle.
    #[test]
    fn laek_og_tilstand_i_beskeden() {
        use crate::overblik::{LaekDag, TilstandFund};
        let fmt = DatoFormat::default();
        let besked = |r: &Raekke1| beslut(std::slice::from_ref(r), &ind(Niveau::Standard), &intet(), NU);
        let l = Raekke1 {
            laek: Some(LaekDag {
                dag: "2026-11-02".into(),
                n: 3,
                naaet: 1,
            }),
            ..r("l", "l.wi24rd.com", Status::Roed, 0, 0.0)
        };
        let (titel, t) = tekst(besked(&l).as_ref().unwrap(), Sprog::Da, &fmt);
        assert_eq!(titel, "l.wi24rd.com: login med lækket kodeord");
        assert!(
            t.contains("3 login-forsøg brugte et kodeord fra et kendt læk i går, og 1 af dem nåede din server"),
            "{t}"
        );
        assert!(!t.contains('/'), "ingen sti: {t}");
        let en = aarsag(&l, Sprog::En);
        assert!(en.contains("3 login attempts used a password from a known leak yesterday"), "{en}");
        let gul = Raekke1 {
            laek: Some(LaekDag {
                dag: "2026-11-02".into(),
                n: 1,
                naaet: 0,
            }),
            ..r("l", "l.wi24rd.com", Status::Gul, 0, 0.0)
        };
        assert_eq!(besked(&gul), None, "Hold øje følger niveauet");
        assert!(beslut(std::slice::from_ref(&gul), &ind(Niveau::Alt), &intet(), NU).is_some());
        assert!(aarsag(&gul, Sprog::Da).contains("1 login-forsøg brugte et kodeord fra et kendt læk i går."));
        let fund = |slags, status: Option<&str>, kl: Option<&str>| TilstandFund {
            slags,
            status: status.map(str::to_string),
            kl: kl.map(str::to_string),
        };
        let pause = Raekke1 {
            gammel: true,
            tilstand: vec![fund("pause", None, None)],
            ..r("p", "p.wi24rd.com", Status::Roed, 0, 0.0)
        };
        let b = besked(&pause).expect("en pause giver ingen nye tal, men skal meldes");
        let (titel, t) = tekst(&b, Sprog::Da, &fmt);
        assert_eq!(titel, "p.wi24rd.com: domænet hos Cloudflare");
        assert!(t.starts_with("Domænet er sat på pause — Cloudflare beskytter det ikke."), "{t}");
        let tekst_for = |f: TilstandFund, sprog| {
            aarsag(
                &Raekke1 {
                    tilstand: vec![f],
                    ..r("p", "p.wi24rd.com", Status::Gul, 0, 0.0)
                },
                sprog,
            )
        };
        assert!(tekst_for(fund("inaktiv", Some("pending"), None), Sprog::Da)
            .starts_with("Domænet er ikke aktivt hos Cloudflare (status: pending)."));
        assert!(tekst_for(fund("phishing", None, None), Sprog::En).starts_with("Cloudflare suspects phishing on the domain."));
        assert!(tekst_for(fund("udvikler", None, Some("14.30")), Sprog::Da)
            .starts_with("Udviklertilstand er slået til — cachen er slået fra, til kl. 14.30."));
    }

    /// Mærker fra døgn, programmet ikke nåede, melder ét ad gangen, ældste først, med deres egen dato. Domænet står stille, til det sidste er meldt.
    #[test]
    fn maerker_fra_slukkede_doegn_faar_hver_sin_besked() {
        let (std, fmt) = (ind(Niveau::Standard), DatoFormat::default());
        let paa = |dag: &str| Moenster {
            dag: dag.into(),
            ..maerke(MoensterSlags::Maalrettet)
        };
        let r = Raekke1 {
            efterslaeb: vec![paa("2026-10-29"), paa("2026-10-31")],
            vurderet: Some("2026-11-01".into()),
            ..groen("a")
        };
        let rk = std::slice::from_ref(&r);
        let b = beslut(rk, &std, &intet(), NU);
        assert_eq!(b.as_ref().unwrap().fund[0].moenster().unwrap().dag, "2026-10-29");
        assert!(tekst(b.as_ref().unwrap(), Sprog::Da, &fmt).1.contains("29. okt"));
        let meldt = efter(&b, intet());
        assert_eq!(meldt[&("a".to_string(), "2026-10-29".to_string())], "groen+maerke");
        assert!(vurderede(rk, &std, &meldt).is_empty(), "et mærke mangler stadig");
        let b = beslut(rk, &std, &meldt, NU + T);
        assert_eq!(b.as_ref().unwrap().fund[0].moenster().unwrap().dag, "2026-10-31");
        let meldt = efter(&b, meldt);
        assert_eq!(beslut(rk, &std, &meldt, NU + 2 * T), None, "én besked pr. mærke");
        assert_eq!(vurderede(rk, &std, &meldt), [("a".to_string(), "2026-11-01".to_string())]);
    }

    /// Rolig og "Aldrig" melder ikke mærket, og det vurderede rykker alligevel, så et senere valg ikke melder gammelt nyt.
    #[test]
    fn vurderet_rykker_ogsaa_uden_besked() {
        let r = Raekke1 {
            efterslaeb: vec![maerke(MoensterSlags::Maalrettet)],
            vurderet: Some("2026-11-01".into()),
            ..groen("a")
        };
        let rk = std::slice::from_ref(&r);
        let rolig = ind(Niveau::Rolig);
        assert_eq!(beslut(rk, &rolig, &intet(), NU), None);
        assert_eq!(vurderede(rk, &rolig, &intet()).len(), 1);
        let aldrig = Indst {
            zoner: HashMap::from([("a".into(), ZoneValg::Fra)]),
            ..ind(Niveau::Standard)
        };
        assert_eq!(beslut(rk, &aldrig, &intet(), NU), None);
        assert_eq!(vurderede(rk, &aldrig, &intet()).len(), 1);
        let uden = Raekke1 {
            vurderet: None,
            ..r.clone()
        };
        assert!(vurderede(&[uden], &rolig, &intet()).is_empty(), "uden et nyt døgn intet at notere");
    }

    /// Beslutning 1: foreløbigt mærke giver ingen besked; det afgjorte melder på sit eget døgn og uden at røre statusdøgnet.
    #[test]
    fn foreloebig_melder_ikke_og_afgjort_melder_paa_sit_eget_doegn() {
        let std = ind(Niveau::Alt);
        let mut g = med(groen("a"), MoensterSlags::Maalrettet);
        g.afgjort = None;
        g.moenster.as_mut().unwrap().foreloebig = true;
        assert_eq!(beslut(std::slice::from_ref(&g), &std, &intet(), NU), None, "foreløbigt");
        assert!(moenster_tekst(g.moenster.as_ref().unwrap(), Sprog::Da).ends_with("Foreløbig — afgøres, når i morgen er hentet."));
        assert!(moenster_tekst(g.moenster.as_ref().unwrap(), Sprog::En).ends_with("Provisional — decided when tomorrow is fetched."));
        assert!(
            !vis_selv(Niveau::Standard, None, std::slice::from_ref(&g), NU, &Tidszone::default()),
            "åbner ikke kortet"
        );
        let gaar = dag_noegle(NU - 2 * DOEGN_MS);
        let mut g = groen("a");
        g.afgjort = Some(Moenster {
            dag: gaar.clone(),
            ..maerke(MoensterSlags::Maalrettet)
        });
        g.moenster = g.afgjort.clone();
        let b = beslut(std::slice::from_ref(&g), &std, &intet(), NU);
        assert_eq!(
            b.as_ref().unwrap().meldte(),
            [("a".to_string(), gaar.clone(), "groen+maerke".to_string())]
        );
        let meldt = efter(&b, intet());
        assert_eq!(beslut(std::slice::from_ref(&g), &std, &meldt, NU + T), None, "én gang");
        // Rød på statusdøgnet i samme runde: to rækker i `meldt`, og den rødeste på mærkets døgn bevares.
        let mut x = roed("a");
        x.afgjort = g.afgjort.clone();
        let foer = HashMap::from([(("a".to_string(), gaar.clone()), "roed".to_string())]);
        let b = beslut(&[x], &std, &foer, NU).unwrap();
        assert_eq!(
            b.meldte(),
            [
                ("a".to_string(), dag_noegle(NU - DOEGN_MS), "roed".to_string()),
                ("a".to_string(), gaar, "roed+maerke".to_string())
            ]
        );
    }

    #[test]
    fn lokal_tid_over_midnat_og_sommertid() {
        // København 2026: sommertid til 25. okt. kl. 01:00 UTC, derefter vintertid.
        let skift = kl("2026-10-25T01:00:00Z");
        let tz = Tidszone(vec![(0, 120), (skift, 60)]);
        assert_eq!(tz.dag(kl("2026-10-24T21:59:00Z")), "2026-10-24");
        assert_eq!(tz.dag(kl("2026-10-24T22:00:00Z")), "2026-10-25", "midnat i sommertid");
        assert_eq!(tz.dag(kl("2026-10-25T22:59:00Z")), "2026-10-25");
        assert_eq!(tz.dag(kl("2026-10-25T23:00:00Z")), "2026-10-26", "midnat i vintertid");
        assert_eq!(tz.kl(kl("2026-10-25T00:59:00Z")), (2, 59));
        assert_eq!(tz.kl(kl("2026-10-25T01:00:00Z")), (2, 0), "uret går en time tilbage");
        assert_eq!(Tidszone::default().dag(kl("2026-10-24T23:30:00Z")), "2026-10-24", "uden skift: UTC");
        assert_eq!(
            Tidszone(vec![(i64::MAX, 60)]).dag(kl("2026-10-24T23:30:00Z")),
            "2026-10-25",
            "før første skift"
        );
        assert_eq!(
            Tidszone(vec![(0, -300)]).dag(kl("2026-10-25T03:00:00Z")),
            "2026-10-24",
            "vest for UTC"
        );
        // Kortet: kl. 00:30 dansk tid er en ny dag, selv om det stadig er i går i UTC.
        let tz = Tidszone(vec![(0, 60)]);
        let (vist, nu) = (iso(kl("2026-11-03T20:00:00Z")), kl("2026-11-03T23:30:00Z"));
        assert!(vis_selv(Niveau::Alt, Some(&vist), &[], nu, &tz), "ny lokal dag");
        assert!(!vis_selv(Niveau::Alt, Some(&vist), &[], nu, &Tidszone::default()), "samme UTC-døgn");
        // Rolig: én om dagen efter den lokale dag; "efter natten" er stadig UTC.
        let rolig = Indst {
            rolig_sidst: Some("2026-11-03".into()),
            tz: tz.clone(),
            ..ind(Niveau::Rolig)
        };
        let samme = kl("2026-11-03T21:00:00Z");
        assert_eq!(beslut(&[roed("a")], &rolig, &intet(), samme), None, "samme lokale dag");
        assert_eq!(ids(&beslut(&[roed("a")], &rolig, &intet(), nu)), ["a"], "ny lokal dag");
    }

    #[test]
    fn i_forgaars_efter_den_lokale_dag() {
        let x = Raekke1 {
            forgaars: true,
            ..r("a", "a.wi24rd.com", Status::Groen, 2, 2.0)
        };
        assert!(aarsag(&x, Sprog::Da).starts_with("2 angreb i forgårs"), "{}", aarsag(&x, Sprog::Da));
        assert!(!aarsag(&x, Sprog::Da).contains("Opdateres"), "kun om natten");
        let nul = Raekke1 {
            forgaars: true,
            ..r("a", "a.wi24rd.com", Status::Groen, 0, 0.0)
        };
        assert_eq!(aarsag(&nul, Sprog::En), "No attacks the day before yesterday.");
    }

    fn groen(id: &str) -> Raekke1 {
        r(id, &format!("{id}.wi24rd.com"), Status::Groen, 3, 2.0)
    }

    #[test]
    fn maalrettet_melder_som_standard_ogsaa_paa_groen() {
        let g = med(groen("a"), MoensterSlags::Maalrettet);
        for niveau in [Niveau::Standard, Niveau::Alt] {
            assert_eq!(
                ids(&beslut(std::slice::from_ref(&g), &ind(niveau), &intet(), NU)),
                ["a"],
                "{niveau:?}"
            );
        }
        assert_eq!(
            beslut(std::slice::from_ref(&g), &ind(Niveau::Rolig), &intet(), NU),
            None,
            "Rolig er kun rødt"
        );
        let aldrig = Indst {
            zoner: HashMap::from([("a".into(), ZoneValg::Fra)]),
            ..ind(Niveau::Alt)
        };
        assert_eq!(beslut(std::slice::from_ref(&g), &aldrig, &intet(), NU), None, "Aldrig");
        let gammel = Raekke1 { gammel: true, ..g.clone() };
        assert_eq!(beslut(&[gammel], &ind(Niveau::Alt), &intet(), NU), None, "gamle tal");
        let b = beslut(std::slice::from_ref(&g), &ind(Niveau::Standard), &intet(), NU);
        let meldt = efter(&b, intet());
        assert_eq!(
            beslut(std::slice::from_ref(&g), &ind(Niveau::Standard), &meldt, NU + T),
            None,
            "højst én pr. døgn"
        );
    }

    #[test]
    fn rekognoscering_melder_kun_naar_det_er_slaaet_til() {
        let g = med(groen("a"), MoensterSlags::Rekognoscering);
        for niveau in [Niveau::Rolig, Niveau::Standard, Niveau::Alt] {
            assert_eq!(
                beslut(std::slice::from_ref(&g), &ind(niveau), &intet(), NU),
                None,
                "{niveau:?}: fra som standard"
            );
        }
        assert!(!Indst::default().rekognoscering);
        let til = Indst {
            rekognoscering: true,
            ..ind(Niveau::Standard)
        };
        assert_eq!(ids(&beslut(std::slice::from_ref(&g), &til, &intet(), NU)), ["a"]);
        let mut gammel_episode = g.clone();
        gammel_episode.moenster.as_mut().unwrap().ny = false;
        assert_eq!(beslut(&[gammel_episode], &til, &intet(), NU), None, "kun første døgn i episoden");
    }

    /// Q5: bliver mærket gemt som et niveau, undertrykker det en rød samme døgn.
    #[test]
    fn maerket_undertrykker_aldrig_en_roed() {
        let std = ind(Niveau::Standard);
        for start in [groen("a"), gul("a")] {
            let b = beslut(&[med(start, MoensterSlags::Maalrettet)], &std, &intet(), NU);
            let meldt = efter(&b, intet());
            let dag = (String::from("a"), dag_noegle(NU - DOEGN_MS));
            assert!(meldt[&dag].ends_with("+maerke"), "{}", meldt[&dag]);
            let roed_nu = med(roed("a"), MoensterSlags::Maalrettet);
            let b2 = beslut(std::slice::from_ref(&roed_nu), &std, &meldt, NU + T);
            assert_eq!(ids(&b2), ["a"], "rød samme døgn melder");
            assert!(b2.as_ref().unwrap().fund[0].maerke.is_none(), "mærket meldes ikke igen");
            assert_eq!(
                tekst(b2.as_ref().unwrap(), Sprog::Da, &DatoFormat::default()).0,
                "a.wi24rd.com: usædvanligt mange angreb"
            );
            let meldt = efter(&b2, meldt);
            assert_eq!(meldt[&dag], "roed+maerke");
            assert_eq!(beslut(&[roed_nu], &std, &meldt, NU + 2 * T), None, "og derefter er der ro");
        }
        // Rød først, så mærket: det kommer stadig, og det rødeste huskes.
        let b = beslut(&[roed("a")], &std, &intet(), NU);
        let meldt = efter(&b, intet());
        let b2 = beslut(&[med(gul("a"), MoensterSlags::Maalrettet)], &std, &meldt, NU + T);
        assert_eq!(ids(&b2), ["a"]);
        assert_eq!(b2.unwrap().meldte()[0].2, "roed+maerke");
    }

    #[test]
    fn maerkets_tekster_siger_altid_ligner() {
        let fmt = DatoFormat::default();
        let besked = |r: Raekke1| Besked {
            fund: vec![Fund {
                dag: "d".into(),
                rolig: false,
                maerke: r.afgjort.clone().or(r.moenster.clone()),
                r,
                meldes: Vec::new(),
            }],
            rolig: false,
        };
        let m = med(groen("a"), MoensterSlags::Maalrettet);
        assert_eq!(
            tekst(&besked(m.clone()), Sprog::Da, &fmt),
            (
                "Ligner et målrettet angreb".to_string(),
                "a.wi24rd.com: angrebene steg her den 2. nov. 2026, men ikke i resten af verden. Cloudflare har stoppet dem. Det er ikke sikkert, at nogen sigter på dig.".to_string()
            )
        );
        assert_eq!(
            tekst(&besked(m.clone()), Sprog::En, &fmt),
            (
                "Looks like a targeted attack".to_string(),
                "a.wi24rd.com: attacks rose here on 2 Nov 2026 but not elsewhere. Cloudflare stopped them. It is not certain that anyone is aiming at you."
                    .to_string()
            )
        );
        let mut delvist = m.clone();
        delvist.moenster.as_mut().unwrap().stoppet = 1;
        delvist.moenster.as_mut().unwrap().verden = false;
        delvist.afgjort = delvist.moenster.clone();
        assert_eq!(
            tekst(&besked(delvist), Sprog::Da, &fmt).1,
            "a.wi24rd.com: angrebene steg her den 2. nov. 2026, men ikke på dine andre domæner. Cloudflare har stoppet 1 af 3. Det er ikke sikkert, at nogen sigter på dig."
        );
        let rk = med(groen("a"), MoensterSlags::Rekognoscering);
        assert_eq!(
            tekst(&besked(rk.clone()), Sprog::Da, &fmt),
            (
                "Ligner rekognoscering".to_string(),
                "a.wi24rd.com: 240 forskellige stier den 2. nov. 2026 (normalt 12). Det er oftest en automatisk scanner.".to_string()
            )
        );
        let iso = DatoFormat {
            format: "iso".into(),
            moenster: String::new(),
        };
        assert!(tekst(&besked(rk), Sprog::En, &iso).1.contains("paths on 2026-11-02 (usually 12)"));
        let mut x = maerke(MoensterSlags::Maalrettet);
        x.tek = Some("wordpress".into());
        assert_eq!(
            moenster_tekst(&x, Sprog::Da),
            "Ligner målrettet: angrebene steg her, men ikke i resten af verden, og 75 % kom fra ét netværk, og angrebene ramte WordPress, som dit site bruger."
        );
        assert_eq!(
            moenster_tekst(&x, Sprog::En),
            "Looks targeted: attacks rose here but not elsewhere, and 75% came from one network, and the attacks hit WordPress, which your site uses."
        );
        let mut y = maerke(MoensterSlags::Rekognoscering);
        y.scanner = true;
        assert_eq!(
            moenster_tekst(&y, Sprog::Da),
            "Ligner rekognoscering: 240 forskellige stier på én dag (normalt 12), som en almindelig scanner."
        );
        for slags in [MoensterSlags::Maalrettet, MoensterSlags::Rekognoscering] {
            for verden in [true, false] {
                let m = Moenster { verden, ..maerke(slags) };
                assert!(moenster_tekst(&m, Sprog::Da).starts_with("Ligner "));
                assert!(moenster_tekst(&m, Sprog::En).starts_with("Looks "));
            }
        }
    }

    #[test]
    fn aarsag_og_siden_sidst_naevner_maerket() {
        let g = med(groen("a"), MoensterSlags::Rekognoscering);
        assert!(
            aarsag(&g, Sprog::Da).ends_with("Ligner rekognoscering: 240 forskellige stier på én dag (normalt 12)."),
            "{}",
            aarsag(&g, Sprog::Da)
        );
        assert_eq!(nye(std::slice::from_ref(&g), None, NU), ["a"], "et mærke på en grøn række er nyt");
        let a = aftryk(std::slice::from_ref(&g), NU);
        assert!(a.ends_with("\tgroen\trekognoscering"), "{a}");
        assert!(nye(std::slice::from_ref(&g), Some(&a), NU).is_empty());
        assert!(
            !vis_selv(Niveau::Standard, None, std::slice::from_ref(&g), NU, &Tidszone::default()),
            "rekognoscering åbner ikke kortet"
        );
        let m = med(groen("a"), MoensterSlags::Maalrettet);
        assert!(vis_selv(Niveau::Standard, None, &[m], NU, &Tidszone::default()), "målrettet gør");
    }

    #[test]
    fn dato_foelger_formatet() {
        let f = |format: &str, moenster: &str| DatoFormat {
            format: format.into(),
            moenster: moenster.into(),
        };
        assert_eq!(dato("2026-11-04", &f("sprog", ""), Sprog::Da), "4. nov. 2026");
        assert_eq!(dato("2026-11-04", &f("", ""), Sprog::En), "4 Nov 2026");
        assert_eq!(dato("2026-05-14", &f("sprog", ""), Sprog::Da), "14. maj 2026");
        assert_eq!(dato("2026-11-04", &f("iso", ""), Sprog::Da), "2026-11-04");
        assert_eq!(dato("2026-11-04", &f("system", "M/d/yyyy"), Sprog::Da), "11/4/2026");
        assert_eq!(dato("2026-11-04", &f("system", "dd.MM.yy"), Sprog::En), "04.11.26");
        assert_eq!(
            dato("2026-11-04", &f("system", ""), Sprog::Da),
            "4. nov. 2026",
            "uden mønster: sproget"
        );
        assert_eq!(dato("ikke en dag", &f("sprog", ""), Sprog::Da), "ikke en dag");
    }

    #[test]
    fn hvorfor_entals_og_flertalsform_for_0_1_og_2() {
        let med = |a: i64| {
            let mut x = r("a", "a.wi24rd.com", Status::Roed, a, 2.9);
            x.angreb_fordeling.blokeret = a;
            x
        };
        assert_eq!(aarsag(&med(0), Sprog::En), "No attacks yesterday.");
        assert_eq!(aarsag(&med(0), Sprog::Da), "Ingen angreb i går.");
        assert!(
            aarsag(&med(1), Sprog::En).starts_with("1 attack yesterday (average 2.9) – stopped."),
            "{}",
            aarsag(&med(1), Sprog::En)
        );
        assert!(
            aarsag(&med(1), Sprog::Da).starts_with("1 angreb i går (snit 2,9) – stoppet."),
            "{}",
            aarsag(&med(1), Sprog::Da)
        );
        assert!(aarsag(&med(2), Sprog::En).starts_with("2 attacks yesterday (average 2.9) – all stopped."));
        assert!(aarsag(&med(2), Sprog::Da).starts_with("2 angreb i går (snit 2,9) – alle stoppet."));
        let natten = Raekke1 { opdateres: true, ..med(1) };
        assert!(aarsag(&natten, Sprog::En).starts_with("1 attack the day before yesterday"));
        let en = |a: i64| Besked {
            fund: vec![
                Fund {
                    dag: "d".into(),
                    rolig: false,
                    r: med(a),
                    maerke: None,
                    meldes: Vec::new(),
                },
                Fund {
                    dag: "d".into(),
                    rolig: false,
                    r: med(2),
                    maerke: None,
                    meldes: Vec::new(),
                },
            ],
            rolig: false,
        };
        assert_eq!(
            tekst(&en(1), Sprog::En, &DatoFormat::default()).1,
            "a.wi24rd.com 1 attack · a.wi24rd.com 2 attacks"
        );
        assert_eq!(
            tekst(&en(0), Sprog::En, &DatoFormat::default()).1,
            "a.wi24rd.com 0 attacks · a.wi24rd.com 2 attacks"
        );
        assert_eq!(
            tekst(&en(1), Sprog::Da, &DatoFormat::default()).1,
            "a.wi24rd.com 1 angreb · a.wi24rd.com 2 angreb"
        );
    }

    #[test]
    fn en_notifikation_viser_aldrig_en_sti() {
        let en = |r: Raekke1| Besked {
            fund: vec![Fund {
                dag: "2026-11-03".into(),
                rolig: false,
                r,
                maerke: None,
                meldes: Vec::new(),
            }],
            rolig: false,
        };
        let to = Besked {
            fund: en(roed("a")).fund.into_iter().chain(en(gul("b")).fund).collect(),
            rolig: false,
        };
        for b in [en(roed("a")), en(gul("b")), en(wi24rd()), to] {
            for sprog in [Sprog::Da, Sprog::En] {
                let (titel, tekst) = tekst(&b, sprog, &DatoFormat::default());
                let ord = format!("{titel} {tekst}");
                assert!(!ord.split_whitespace().any(|o| o.starts_with('/')), "{ord}");
            }
        }
    }

    #[test]
    fn beslut_roed_giver_besked_i_standard_og_rolig_ikke_gul() {
        let rk = [roed("a"), gul("b"), r("c", "c.wi24rd.com", Status::Groen, 3, 2.0)];
        for niveau in [Niveau::Standard, Niveau::Rolig] {
            let b = beslut(&rk, &ind(niveau), &intet(), NU);
            assert_eq!(ids(&b), ["a"], "{niveau:?}: kun rødt");
            assert_eq!(b.unwrap().rolig, niveau == Niveau::Rolig);
        }
        assert_eq!(beslut(&[gul("b")], &ind(Niveau::Standard), &intet(), NU), None, "gul alene: intet");
        assert_eq!(Indst::default().niveau, Niveau::Standard, "Standard er forvalgt");
        assert!(Indst::default().roed, "rødt er forvalgt til");
    }

    #[test]
    fn beslut_alt_giver_gul_og_igen_ved_gul_til_roed_samme_doegn() {
        let alt = ind(Niveau::Alt);
        let b = beslut(&[gul("b")], &alt, &intet(), NU);
        assert_eq!(ids(&b), ["b"]);
        let meldt = efter(&b, intet());
        assert_eq!(meldt[&("b".into(), dag_noegle(NU - DOEGN_MS))], "gul");
        assert_eq!(beslut(&[gul("b")], &alt, &meldt, NU + T), None, "gul igen samme døgn: intet");
        let b2 = beslut(&[roed("b")], &alt, &meldt, NU + T);
        assert_eq!(ids(&b2), ["b"], "gul → rød samme døgn: én ekstra");
        let meldt = efter(&b2, meldt);
        assert_eq!(beslut(&[roed("b")], &alt, &meldt, NU + 2 * T), None, "rød igen: intet");
        // Standard melder aldrig gult, så gul → rød er den første besked.
        assert_eq!(ids(&beslut(&[roed("b")], &ind(Niveau::Standard), &intet(), NU)), ["b"]);
    }

    #[test]
    fn beslut_hoejst_een_pr_domaene_pr_statusdoegn() {
        let std = ind(Niveau::Standard);
        let b = beslut(&[roed("a")], &std, &intet(), NU);
        let meldt = efter(&b, intet());
        assert_eq!(beslut(&[roed("a")], &std, &meldt, NU + T), None, "næste runde samme døgn");
        assert_eq!(beslut(&[roed("a")], &std, &meldt, NU + 15 * T), None, "kl. 23:08 samme døgn");
        assert_eq!(
            ids(&beslut(&[roed("a")], &std, &meldt, NU + DOEGN_MS)),
            ["a"],
            "nyt statusdøgn: ny besked"
        );
        // Et andet domæne meldes stadig, selvom det første er meldt.
        assert_eq!(ids(&beslut(&[roed("a"), roed("b")], &std, &meldt, NU + T)), ["b"]);
    }

    #[test]
    fn beslut_natten_giver_ikke_ny_besked_paa_forgaars() {
        let std = ind(Niveau::Standard);
        // I går (2026-11-02, status på 11-02) blev a meldt rødt.
        let meldt = efter(&beslut(&[roed("a")], &std, &intet(), NU), intet());
        // Næste nat kl. 01:30: i går (11-03) er ikke færdighentet, så status bygger på 11-02 igen ("opdateres …").
        let nat = kl("2026-11-04T01:30:00Z");
        let paa_forgaars = Raekke1 {
            opdateres: true,
            ..roed("a")
        };
        assert_eq!(statusdoegn(&paa_forgaars, nat), "2026-11-02");
        assert_eq!(
            beslut(std::slice::from_ref(&paa_forgaars), &std, &meldt, nat),
            None,
            "det samme døgn meldes ikke igen"
        );
        // Efter natten, når 11-03 er hentet og stadig rødt: ny besked for 11-03.
        let morgen = kl("2026-11-04T03:10:00Z");
        assert_eq!(ids(&beslut(&[roed("a")], &std, &meldt, morgen)), ["a"]);
    }

    #[test]
    fn beslut_gamle_tal_og_graa_er_aldrig_en_alarm() {
        let gammel = Raekke1 {
            gammel: true,
            ..r("g", "g.wi24rd.com", Status::Gul, 999, 0.0)
        };
        let graa = r("x", "x.wi24rd.com", Status::Graa, 0, 0.0);
        for niveau in [Niveau::Rolig, Niveau::Standard, Niveau::Alt] {
            assert_eq!(
                beslut(&[gammel.clone(), graa.clone()], &ind(niveau), &intet(), NU),
                None,
                "{niveau:?}"
            );
        }
        let altid = Indst {
            zoner: HashMap::from([("g".into(), ZoneValg::Alt), ("x".into(), ZoneValg::Alt)]),
            ..ind(Niveau::Alt)
        };
        assert_eq!(beslut(&[gammel, graa], &altid, &intet(), NU), None, "heller ikke med Altid");
    }

    /// Eksempel: 1.200 i labyrinten + 30 botFight i går og ingen angreb er grønt og melder intet, heller ikke i Alt.
    #[test]
    fn beslut_rutine_alene_kan_ikke_udloese() {
        let fw = |dag: String, kilde: &str, handling: &str, n: i64| Raekke {
            dag,
            art: "fw".into(),
            noegle: ["Regel", "US", "AS", "/", "CPH", "GET", kilde, handling].join("\t"),
            n,
        };
        let d = |i: i64| dag_noegle(NU - i * DOEGN_MS);
        let rows = vec![
            fw(d(1), "linkMaze", "link_maze_injected", 1200),
            fw(d(1), "botFight", "managed_challenge", 30),
            fw(d(1), "bic", "managed_challenge", 400),
        ];
        let hentet: HashMap<String, String> = (0..=HENT_DAGE).map(|i| (d(i), iso(NU - T))).collect();
        let zone = Zone {
            id: "w".into(),
            navn: "wi24rd.com".into(),
        };
        let rk = raekke(&zone, &hentet, &rows, NU);
        assert_eq!((rk.status, rk.angreb_i_gaar), (Status::Groen, 0));
        assert_eq!(
            rk.hele.i_gaar, 430,
            "afvist tæller rutine med, men ikke siderne med fælde-links (J9)"
        );
        assert_eq!(beslut(&[rk], &ind(Niveau::Alt), &intet(), NU), None);
    }

    #[test]
    fn beslut_pr_domaene_aldrig_altid_kun_roed() {
        let rk = [roed("a"), gul("b"), roed("c"), gul("d")];
        let i = Indst {
            zoner: HashMap::from([
                ("a".into(), ZoneValg::Fra),
                ("b".into(), ZoneValg::Alt),
                ("c".into(), ZoneValg::Roed),
                ("d".into(), ZoneValg::Roed),
            ]),
            ..ind(Niveau::Rolig)
        };
        let b = beslut(&rk, &i, &intet(), NU).unwrap();
        assert_eq!(
            b.fund.iter().map(|f| f.r.id.as_str()).collect::<Vec<_>>(),
            ["c", "b"],
            "Aldrig, Altid (gul), Kun rødt"
        );
        assert!(!b.rolig, "ingen af dem følger Rolig");
        // Kun rødt følger ikke Roligs "én om dagen".
        let i2 = Indst {
            rolig_sidst: Some(dag_noegle(NU)),
            ..i.clone()
        };
        assert_eq!(ids(&beslut(&[roed("c")], &i2, &intet(), NU)), ["c"]);
        // Ukendte koder.
        assert_eq!((ZoneValg::fra("altid"), Niveau::fra("Standard")), (None, None));
        for v in [ZoneValg::Standard, ZoneValg::Alt, ZoneValg::Roed, ZoneValg::Fra] {
            assert_eq!(ZoneValg::fra(v.kode()), Some(v));
        }
        for v in [Niveau::Rolig, Niveau::Standard, Niveau::Alt] {
            assert_eq!(Niveau::fra(v.kode()), Some(v));
        }
    }

    #[test]
    fn beslut_roed_slaaet_fra() {
        let fra = |niveau| Indst {
            roed: false,
            ..ind(niveau)
        };
        assert_eq!(beslut(&[roed("a")], &fra(Niveau::Standard), &intet(), NU), None);
        let altid = Indst {
            zoner: HashMap::from([("a".into(), ZoneValg::Alt)]),
            ..fra(Niveau::Standard)
        };
        assert_eq!(beslut(&[roed("a")], &altid, &intet(), NU), None, "rødt fra gælder også Altid");
        assert_eq!(
            ids(&beslut(&[roed("a"), gul("b")], &fra(Niveau::Alt), &intet(), NU)),
            ["b"],
            "Alt: gult stadig"
        );
    }

    #[test]
    fn beslut_rolig_hoejst_een_om_dagen_og_ikke_foer_kl_03_utc() {
        let rolig = ind(Niveau::Rolig);
        assert_eq!(
            beslut(&[roed("a")], &rolig, &intet(), kl("2026-11-03T02:59:00Z")),
            None,
            "før kl. 03 UTC"
        );
        let b = beslut(&[roed("a")], &rolig, &intet(), kl("2026-11-03T03:00:00Z"));
        assert_eq!(ids(&b), ["a"]);
        assert!(b.unwrap().rolig);
        let efter_een = Indst {
            rolig_sidst: Some("2026-11-03".into()),
            ..rolig.clone()
        };
        assert_eq!(beslut(&[roed("b")], &efter_een, &intet(), NU), None, "anden besked samme døgn");
        assert_eq!(ids(&beslut(&[roed("b")], &efter_een, &intet(), NU + DOEGN_MS)), ["b"], "næste døgn");
    }

    #[test]
    fn beslut_samler_flere_domaener_i_een_besked() {
        let rk = [
            gul("e"),
            roed("c"),
            gul("a"),
            roed("b"),
            r("z", "z.wi24rd.com", Status::Groen, 0, 0.0),
        ];
        let b = beslut(&rk, &ind(Niveau::Alt), &intet(), NU).unwrap();
        assert_eq!(
            b.fund.iter().map(|f| f.r.id.as_str()).collect::<Vec<_>>(),
            ["b", "c", "a", "e"],
            "rød før gul, så navn"
        );
        assert_eq!(
            b.meldte(),
            [("b", "roed"), ("c", "roed"), ("a", "gul"), ("e", "gul")].map(|(z, n)| (
                z.to_string(),
                "2026-11-02".to_string(),
                n.to_string()
            ))
        );
    }

    /// Typer med samme klarsprogstekst lægges sammen (sum, sorteret efter sum); egne regler med kort navn.
    #[test]
    fn aarsag_fletter_samme_tekst_og_viser_egne_regler_kort() {
        let typ = |navn: &str, n: i64, egen: bool| AngrebType {
            navn: navn.into(),
            n,
            egen,
            regler: Vec::new(),
            tek: None,
        };
        let wp = Raekke1 {
            angreb_i_gaar_top: vec![
                typ("Wordpress", 12, false),
                typ("React RCE (CVE-2025-55182)", 9, false),
                typ("Wordpress Remote Code Execution (CVE-2026-63030)", 3, false),
            ],
            ..r("a", "abe.wi24rd.com", Status::Gul, 24, 5.0)
        };
        assert_eq!(
            aarsag(&wp, Sprog::Da),
            "24 angreb i går (snit 5) – alle stoppet. Mest angreb på WordPress (15) og et kendt hul i React (9). Bed din webmand tjekke, at WordPress og plugins er opdateret."
        );
        assert_eq!(
            flet(&wp, Sprog::En).iter().map(|(k, n)| (k.id, *n)).collect::<Vec<_>>(),
            [("wordpress", 15), ("react", 9)]
        );
        assert_eq!(kort_egen("min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)"), "min-regel");
        assert_eq!(kort_egen("Bloker admin – kun fra kontoret"), "Bloker admin");
        assert_eq!(
            kort_egen("En meget lang regel uden kolon og tankestreg"),
            "En meget lang regel ude…"
        );
        assert_eq!(kort_egen("En meget lang regel uden kolon og tankestreg").chars().count(), 24);
        assert_eq!(
            kort_egen("abcdefghijklmnopqrstuvwxyz"),
            "abcdefghijklmnopqrstuvw…",
            "26 tegn kortes"
        );
        assert_eq!(kort_egen("abcdefghijklmnopqrstuvwx"), "abcdefghijklmnopqrstuvwx", "24 tegn står");
        assert_eq!(
            klar("WordPress login", true, Sprog::Da).tekst,
            "din egen regel »WordPress login«",
            "egen før ordlisten"
        );
        assert_eq!(
            klar("Ukendt", false, Sprog::Da),
            Klar {
                id: "andet",
                tekst: "Ukendt".into(),
                raad: ""
            }
        );
        let foerste = Raekke1 {
            angreb_i_gaar_top: vec![typ("SQLi - x", 3, false)],
            ..r("n", "ny.wi24rd.com", Status::Gul, 3, 0.0)
        };
        assert_eq!(
            aarsag(&foerste, Sprog::Da),
            "3 angreb i går (første gang) – alle stoppet. Mest forsøg på at snige databasekommandoer ind (3)."
        );
        assert_eq!(
            aarsag(&foerste, Sprog::En),
            "3 attacks yesterday (first time) – all stopped. Mostly attempts to sneak in database commands (3)."
        );
    }

    fn wi24rd() -> Raekke1 {
        Raekke1 {
            angreb_i_gaar_top: vec![
                AngrebType {
                    navn: "React RCE (CVE-2025-55182)".into(),
                    n: 20,
                    egen: false,
                    regler: Vec::new(),
                    tek: None,
                },
                AngrebType {
                    navn: "Wordpress".into(),
                    n: 7,
                    egen: false,
                    regler: Vec::new(),
                    tek: None,
                },
                AngrebType {
                    navn: "min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)".into(),
                    n: 2,
                    egen: true,
                    regler: Vec::new(),
                    tek: None,
                },
            ],
            ..r("z1", "wi24rd.com", Status::Gul, 30, 12.4)
        }
    }

    fn andet_domaene() -> Raekke1 {
        Raekke1 {
            angreb_i_gaar_top: vec![
                AngrebType {
                    navn: "React RCE (CVE-2025-55182)".into(),
                    n: 25,
                    egen: false,
                    regler: Vec::new(),
                    tek: None,
                },
                AngrebType {
                    navn: "Browser Integrity Check".into(),
                    n: 3,
                    egen: false,
                    regler: Vec::new(),
                    tek: None,
                },
            ],
            angreb_fordeling: Fordeling {
                blokeret: 20,
                udfordret: 8,
                labyrint: 0,
                andet: 0,
                faelder: 0,
            },
            ..r("z2", "xyz.wi24rd.com", Status::Roed, 28, 23.1)
        }
    }

    /// Samme eksempler og strenge som UI-røgtesten (P3) for `hvorfor` i visning-overblik.js.
    /// Q4: sætningen står før reglerne og ændrer intet andet, heller ikke status.
    #[test]
    fn aarsag_med_sammenligningen() {
        let uden = wi24rd();
        let foelger = Raekke1 {
            tendens: Some(Tendens::Foelger),
            ..wi24rd()
        };
        let dig = Raekke1 {
            tendens: Some(Tendens::SaerligtDig),
            ..wi24rd()
        };
        let t = aarsag(&foelger, Sprog::Da);
        assert!(
            t.starts_with("30 angreb i går (snit 12,4) – alle stoppet. Følger den globale tendens. Mest "),
            "{t}"
        );
        assert_eq!(t.replace(" Følger den globale tendens.", ""), aarsag(&uden, Sprog::Da));
        assert!(aarsag(&foelger, Sprog::En).contains(" Follows the global trend. Mostly "));
        assert!(aarsag(&dig, Sprog::Da).contains(" Verden var normal i går, så stigningen er din egen. Mest "));
        assert!(aarsag(&dig, Sprog::En).contains(" The world was normal yesterday, so the rise is yours. Mostly "));
        let nat = Raekke1 { opdateres: true, ..dig };
        assert!(aarsag(&nat, Sprog::Da).contains(" Verden var normal i forgårs, så stigningen er din egen."));
        assert!(aarsag(&nat, Sprog::En).contains(" The world was normal the day before yesterday, so the rise is yours."));
        assert_eq!(foelger.status, uden.status);
    }

    /// 52x giver Hold øje og følger notifikationsreglerne: kun Alt melder gult, ligesom ved angreb.
    #[test]
    fn server_svarer_ikke_foelger_notifikationsreglerne() {
        let nede = Raekke1 {
            server: Some(ServerFejl {
                dag: "2026-11-02".into(),
                fejl: 40,
                svar: 120,
                procent: 33,
            }),
            ..r("s", "nede.wi24rd.com", Status::Gul, 0, 0.0)
        };
        let rk = [nede.clone()];
        assert!(
            beslut(&rk, &ind(Niveau::Standard), &intet(), NU).is_none(),
            "gul melder ikke på Standard"
        );
        assert_eq!(ids(&beslut(&rk, &ind(Niveau::Alt), &intet(), NU)), ["s"]);
        assert_eq!(
            aarsag(&nede, Sprog::Da),
            "Ingen angreb i går. Din server svarer ikke: 33 % af 120 svar i går var fejl 520–527, så Cloudflare kunne ikke få svar fra din server. Se »Hvad gør jeg?«."
        );
        assert_eq!(
            aarsag(&nede, Sprog::En),
            "No attacks yesterday. Your server is not answering: 33% of 120 responses yesterday were errors 520–527, so Cloudflare could not get an answer from your server. See “What do I do?”."
        );
        let nat = Raekke1 {
            opdateres: true,
            ..nede.clone()
        };
        assert_eq!(
            aarsag(&nat, Sprog::Da),
            "Ingen angreb i forgårs. Din server svarer ikke: 33 % af 120 svar i forgårs var fejl 520–527, så Cloudflare kunne ikke få svar fra din server. Se »Hvad gør jeg?«. Opdateres …"
        );
        let med_angreb = Raekke1 {
            status: Status::Roed,
            ..wi24rd()
        };
        let med_angreb = Raekke1 {
            server: nede.server.clone(),
            ..med_angreb
        };
        assert!(aarsag(&med_angreb, Sprog::Da).ends_with("Bed din webmand tjekke, at siden er opdateret. Din server svarer ikke: 33 % af 120 svar i går var fejl 520–527, så Cloudflare kunne ikke få svar fra din server. Se »Hvad gør jeg?«."));
    }

    fn trafik(n: i64, stoppet: i64) -> Option<TrafikDag> {
        Some(TrafikDag {
            dag: "2026-11-02".into(),
            n,
            median: 1500.0,
            stoppet,
        })
    }

    /// Vi kan ikke vide, at det ikke er et angreb, og teksten påstår det ikke.
    #[test]
    fn trafikspring_er_ikke_bevist_harmloest() {
        let x = Raekke1 {
            trafik: trafik(410_000, 67),
            angreb_status: Status::Groen,
            ..r("t", "t.wi24rd.com", Status::Gul, 0, 0.0)
        };
        assert_eq!(
            aarsag(&x, Sprog::Da),
            "Ingen angreb i går. Usædvanlig meget trafik: 410.000 forespørgsler (normalt 1.500). Cloudflare stoppede 67 som angreb. De øvrige 409.933 slap igennem og tæller ikke som angreb — det kan være et forsøg på at overbelaste siden (DDoS), en robot, der løber løbsk, eller en fejl. Se, hvor trafikken kom fra."
        );
        assert_eq!(
            aarsag(&x, Sprog::En),
            "No attacks yesterday. Unusually high traffic: 410,000 requests (normally 1,500). Cloudflare stopped 67 as attacks. The other 409,933 got through and do not count as attacks — it may be an attempt to overload the site (DDoS), a robot running wild, or a fault. See where the traffic came from."
        );
        for sprog in [Sprog::Da, Sprog::En] {
            let t = aarsag(&x, sprog).to_lowercase();
            assert!(!t.contains("ikke et angreb") && !t.contains("not an attack"), "{t}");
        }
        assert!(
            beslut(std::slice::from_ref(&x), &ind(Niveau::Standard), &intet(), NU).is_none(),
            "gult melder kun ved Alt"
        );
        let b = beslut(std::slice::from_ref(&x), &ind(Niveau::Alt), &intet(), NU).unwrap();
        assert_eq!(
            tekst(&b, Sprog::Da, &DatoFormat::default()).0,
            "t.wi24rd.com: usædvanlig meget trafik",
            "det er ikke flere angreb"
        );
        assert_eq!(
            tekst(&b, Sprog::En, &DatoFormat::default()).0,
            "t.wi24rd.com: unusually high traffic"
        );
    }

    fn tek(t: &str, n: i64) -> TekN {
        TekN { tek: t.into(), n }
    }

    #[test]
    fn hvorfor_siger_hvad_valgene_goer() {
        let wp = Raekke1 {
            ikke_brugt: vec![tek("wordpress", 47)],
            angreb_i_gaar_top: vec![AngrebType {
                navn: "Wordpress".into(),
                n: 47,
                egen: false,
                regler: Vec::new(),
                tek: Some("wordpress".into()),
            }],
            angreb_status: Status::Groen,
            ..r("w", "w.wi24rd.com", Status::Gul, 47, 40.0)
        };
        assert_eq!(
            aarsag(&wp, Sprog::Da),
            "47 angreb i går (snit 40) – alle stoppet. Mest angreb på WordPress (47). 47 angreb på WordPress, som du ikke bruger — ufarlige for dig.",
            "ingen råd om noget, sitet ikke kører"
        );
        assert!(aarsag(&wp, Sprog::En).ends_with("47 attacks on WordPress, which you do not use — harmless to you."));
        let to = Raekke1 {
            ikke_brugt: vec![tek("wordpress", 9), tek("react", 6)],
            ..wp.clone()
        };
        assert!(
            aarsag(&to, Sprog::Da).ends_with("15 angreb på WordPress og React, som du ikke bruger — ufarlige for dig."),
            "flere teknologier står i én sætning"
        );
        assert!(aarsag(&to, Sprog::En).ends_with("15 attacks on WordPress and React, which you do not use — harmless to you."));
        let u = Raekke1 {
            uventet: Some(Uventet {
                tek: "wordpress".into(),
                sti: "/wp-login.php".into(),
                n: 2,
            }),
            angreb_status: Status::Groen,
            ..r("u", "u.wi24rd.com", Status::Gul, 0, 0.0)
        };
        assert_eq!(
            aarsag(&u, Sprog::Da),
            "Ingen angreb i går. Uventet: din server svarede på en WordPress-sti. Bruger du alligevel WordPress?"
        );
        let b = beslut(std::slice::from_ref(&u), &ind(Niveau::Alt), &intet(), NU).unwrap();
        assert_eq!(
            tekst(&b, Sprog::Da, &DatoFormat::default()).0,
            "u.wi24rd.com: din server svarede uventet"
        );
        let n = Raekke1 {
            naaet: Some(tek("wordpress", 4)),
            ..r("n", "n.wi24rd.com", Status::Roed, 4, 0.0)
        };
        assert!(aarsag(&n, Sprog::Da).ends_with(" 4 angreb på WordPress nåede din server, og du har sagt, at du bruger WordPress."));
        let b = beslut(std::slice::from_ref(&n), &ind(Niveau::Standard), &intet(), NU).unwrap();
        assert_eq!(
            tekst(&b, Sprog::Da, &DatoFormat::default()).0,
            "n.wi24rd.com: et angreb nåede din server"
        );
    }

    /// "Giv mig besked" melder uanset niveau, én gang pr. statusdøgn, og Aldrig er stadig aldrig.
    #[test]
    fn giv_mig_besked_uanset_niveau() {
        let x = Raekke1 {
            tek_besked: vec![TekBesked {
                tek: "wordpress".into(),
                n: 30,
                snit: 4.0,
            }],
            ..r("b", "b.wi24rd.com", Status::Groen, 30, 25.0)
        };
        let rk = std::slice::from_ref(&x);
        for niveau in [Niveau::Rolig, Niveau::Standard, Niveau::Alt] {
            assert_eq!(ids(&beslut(rk, &ind(niveau), &intet(), NU)), ["b"], "{niveau:?}");
        }
        let nat = kl("2026-11-03T01:00:00Z");
        assert_eq!(
            ids(&beslut(rk, &ind(Niveau::Rolig), &intet(), nat)),
            ["b"],
            "Roligs nat gælder ikke"
        );
        let b = beslut(rk, &ind(Niveau::Standard), &intet(), NU);
        assert_eq!(
            tekst(b.as_ref().unwrap(), Sprog::Da, &DatoFormat::default()).0,
            "b.wi24rd.com: flere angreb på WordPress end normalt"
        );
        assert!(tekst(b.as_ref().unwrap(), Sprog::Da, &DatoFormat::default())
            .1
            .ends_with(" Flere angreb på WordPress end normalt: 30 (snit 4)."));
        assert!(!b.as_ref().unwrap().rolig, "sætter ikke Roligs ene besked");
        let meldt = efter(&b, intet());
        assert_eq!(beslut(rk, &ind(Niveau::Standard), &meldt, NU + T), None, "én gang pr. døgn");
        let senere = Raekke1 {
            status: Status::Roed,
            ..x.clone()
        };
        assert_eq!(
            ids(&beslut(&[senere], &ind(Niveau::Standard), &meldt, NU + T)),
            ["b"],
            "rødt melder stadig bagefter"
        );
        let aldrig = Indst {
            zoner: HashMap::from([("b".into(), ZoneValg::Fra)]),
            ..ind(Niveau::Alt)
        };
        assert_eq!(beslut(rk, &aldrig, &intet(), NU), None);
    }

    #[test]
    fn aarsag_rust_svarer_til_overblikkets_hvorfor() {
        assert_eq!(
            aarsag(&wi24rd(), Sprog::Da),
            "30 angreb i går (snit 12,4) – alle stoppet. Mest et kendt hul i React (20), angreb på WordPress (7) og din egen regel »min-regel« (2). Bed din webmand tjekke, at siden er opdateret."
        );
        assert_eq!(
            aarsag(&andet_domaene(), Sprog::Da),
            "28 angreb i går (snit 23,1) – alle stoppet (20 blokeret, 8 udfordret). Mest et kendt hul i React (25) og browsertjekket (3). Bed din webmand tjekke, at siden er opdateret."
        );
        assert_eq!(
            aarsag(&wi24rd(), Sprog::En),
            "30 attacks yesterday (average 12.4) – all stopped. Mostly a known hole in React (20), attacks on WordPress (7) and your own rule “min-regel” (2). Ask your web person to check that the site is up to date."
        );
        let nul = r("n", "nul.wi24rd.com", Status::Groen, 0, 0.0);
        assert_eq!(aarsag(&nul, Sprog::Da), "Ingen angreb i går.", "nul angreb");
        let labyrint = Raekke1 {
            angreb_i_gaar_top: vec![AngrebType {
                navn: "Egen regel".into(),
                n: 12,
                egen: false,
                regler: Vec::new(),
                tek: None,
            }],
            angreb_fordeling: Fordeling {
                blokeret: 2,
                udfordret: 0,
                labyrint: 9,
                andet: 1,
                faelder: 0,
            },
            ..r("l", "labyrint.wi24rd.com", Status::Gul, 12, 1.0)
        };
        assert_eq!(
            aarsag(&labyrint, Sprog::Da),
            "12 angreb i går (snit 1) – 11 af 12 stoppet (2 blokeret, 0 udfordret, 9 i labyrinten, 1 andet). Mest Egen regel (12)."
        );
        assert_eq!(
            aarsag(&labyrint, Sprog::En),
            "12 attacks yesterday (average 1) – 11 of 12 stopped (2 blocked, 0 challenged, 9 in the labyrinth, 1 other). Mostly Egen regel (12)."
        );
        // J9: en side leveret med fælde-links er hverken stoppet eller "i labyrinten" (her fra en egen regel).
        let faelder = Raekke1 {
            angreb_fordeling: Fordeling {
                blokeret: 2,
                faelder: 10,
                ..Fordeling::default()
            },
            ..labyrint.clone()
        };
        assert_eq!(
            aarsag(&faelder, Sprog::Da),
            "12 angreb i går (snit 1) – 2 af 12 stoppet (2 blokeret, 0 udfordret, 10 andet). Mest Egen regel (12)."
        );
        assert_eq!(
            aarsag(&faelder, Sprog::En),
            "12 attacks yesterday (average 1) – 2 of 12 stopped (2 blocked, 0 challenged, 10 other). Mostly Egen regel (12)."
        );
        assert_eq!(uden_cve("A (CVE-1) (x)"), "A (CVE-1) (x)", "kun et CVE-nummer helt til sidst");
        assert_eq!(uden_cve("Wordpress (CVE-2024-1)"), "Wordpress");
        assert_eq!(
            (
                tal(1260.0, Sprog::Da),
                tal(1260.0, Sprog::En),
                tal(12.4, Sprog::Da),
                tal(2.0, Sprog::Da)
            ),
            ("1.260".into(), "1,260".into(), "12,4".into(), "2".into())
        );
        assert_eq!(
            (tal(1_234_567.0, Sprog::Da), tal(23.1, Sprog::En), tal(0.0, Sprog::Da)),
            ("1.234.567".into(), "23.1".into(), "0".into())
        );
    }

    #[test]
    fn tekst_da_og_en_een_og_flere_domaener_alle_blokeret_og_fordelt() {
        let een = |r: Raekke1| Besked {
            fund: vec![Fund {
                dag: "2026-11-02".into(),
                rolig: false,
                r,
                maerke: None,
                meldes: Vec::new(),
            }],
            rolig: false,
        };
        let roed_alle = Raekke1 {
            status: Status::Roed,
            ..wi24rd()
        };
        assert_eq!(
            tekst(&een(roed_alle.clone()), Sprog::Da, &DatoFormat::default()),
            (
                "wi24rd.com: usædvanligt mange angreb".into(),
                "30 angreb i går (snit 12,4) – alle stoppet. Mest et kendt hul i React (20), angreb på WordPress (7) og din egen regel »min-regel« (2). Bed din webmand tjekke, at siden er opdateret.".into()
            )
        );
        assert_eq!(
            tekst(&een(andet_domaene()), Sprog::En, &DatoFormat::default()),
            (
                "xyz.wi24rd.com: unusually many attacks".into(),
                "28 attacks yesterday (average 23.1) – all stopped (20 blocked, 8 challenged). Mostly a known hole in React (25) and the browser check (3). Ask your web person to check that the site is up to date.".into()
            )
        );
        assert_eq!(
            tekst(&een(wi24rd()), Sprog::Da, &DatoFormat::default()).0,
            "wi24rd.com: flere angreb end normalt",
            "gul"
        );
        assert_eq!(
            tekst(&een(wi24rd()), Sprog::En, &DatoFormat::default()).0,
            "wi24rd.com: more attacks than usual"
        );
        let nat = Raekke1 {
            opdateres: true,
            ..andet_domaene()
        };
        assert!(tekst(&een(nat.clone()), Sprog::Da, &DatoFormat::default())
            .1
            .starts_with("28 angreb i forgårs (snit 23,1) – alle stoppet"));
        assert!(tekst(&een(nat.clone()), Sprog::En, &DatoFormat::default())
            .1
            .starts_with("28 attacks the day before yesterday"));
        assert!(
            tekst(&een(nat), Sprog::Da, &DatoFormat::default()).1.ends_with(" Opdateres …"),
            "som i Overblikket"
        );

        let mange = |rk: Vec<Raekke1>| Besked {
            fund: rk
                .into_iter()
                .map(|r| Fund {
                    dag: "2026-11-02".into(),
                    rolig: false,
                    r,
                    maerke: None,
                    meldes: Vec::new(),
                })
                .collect(),
            rolig: false,
        };
        let to = mange(vec![andet_domaene(), roed_alle]);
        assert_eq!(
            tekst(&to, Sprog::Da, &DatoFormat::default()),
            (
                "2 domæner kræver opmærksomhed".into(),
                "xyz.wi24rd.com 28 angreb · wi24rd.com 30 angreb".into()
            )
        );
        let fem = mange(vec![
            roed("a"),
            roed("b"),
            gul("c"),
            gul("d"),
            r("e", "e.wi24rd.com", Status::Gul, 1, 0.0),
        ]);
        assert_eq!(
            tekst(&fem, Sprog::Da, &DatoFormat::default()),
            (
                "5 domæner kræver opmærksomhed".into(),
                "a.wi24rd.com 45 angreb · b.wi24rd.com 45 angreb · c.wi24rd.com 18 angreb · + 2 flere".into()
            )
        );
        let fire_en = mange(vec![
            roed("a"),
            r("e", "e.wi24rd.com", Status::Gul, 1, 0.0),
            gul("c"),
            r("t", "t.wi24rd.com", Status::Gul, 1_500, 0.0),
        ]);
        assert_eq!(
            tekst(&fire_en, Sprog::En, &DatoFormat::default()),
            (
                "4 domains need attention".into(),
                "a.wi24rd.com 45 attacks · e.wi24rd.com 1 attack · c.wi24rd.com 18 attacks · + 1 more".into()
            )
        );
    }

    /// Beslutning 6: ved start vurderes fra cachen, men kun domæner med friske tal (grundlag i går eller døgnet før om
    /// natten); "tallene er gamle" melder intet.
    #[test]
    fn cache_vurdering_kun_ved_igaar_eller_forgaars() {
        let d = |nu: i64, i: i64| dag_noegle(nu - i * DOEGN_MS);
        let blok = |dag: String, n: i64| Raekke {
            dag,
            art: "fw".into(),
            noegle: [
                "React - RCE - CVE:CVE-2025-55182",
                "US",
                "AS",
                "/",
                "CPH",
                "POST",
                "firewallManaged",
                "block",
            ]
            .join("\t"),
            n,
        };
        let zone = Zone {
            id: "a".into(),
            navn: "a.wi24rd.com".into(),
        };
        let hentet = |nu: i64, t: i64| -> HashMap<String, String> { (0..=HENT_DAGE).map(|i| (d(nu, i), iso(t))).collect() };
        let rows = |nu: i64, i_gaar: i64| -> Vec<Raekke> {
            let mut v = vec![blok(d(nu, 1), i_gaar)];
            v.extend((2..=8).map(|i| blok(d(nu, i), 2)));
            v
        };
        let std = ind(Niveau::Standard);
        // I går: friske tal, rødt → besked.
        let r1 = raekke(&zone, &hentet(NU, NU - T), &rows(NU, 60), NU);
        assert_eq!((r1.status, r1.gammel), (Status::Roed, false));
        assert_eq!(ids(&beslut(&[r1], &std, &intet(), NU)), ["a"]);
        // Gamle tal (hentet for 27 t siden): gul "gamle", intet, også med mange angreb.
        let r2 = raekke(&zone, &hentet(NU, NU - 27 * T), &rows(NU, 600), NU);
        assert_eq!((r2.status, r2.gammel), (Status::Gul, true));
        assert_eq!(beslut(&[r2], &ind(Niveau::Alt), &intet(), NU), None);
        // Natten: i går er ikke færdig, døgnet før er; status på d-2.
        let nat = kl("2026-11-03T01:30:00Z");
        let mut v = vec![blok(d(nat, 2), 60)];
        v.extend((3..=9).map(|i| blok(d(nat, i), 2)));
        let r3 = raekke(&zone, &hentet(nat, kl("2026-11-03T00:40:00Z")), &v, nat);
        assert_eq!((r3.status, r3.gammel, r3.opdateres), (Status::Roed, false, true));
        let b = beslut(&[r3], &std, &intet(), nat).unwrap();
        assert_eq!(b.fund[0].dag, "2026-11-01", "statusdøgnet er døgnet før i går");
        // Intet hentet: grå, intet.
        let r4 = raekke(&zone, &HashMap::new(), &[], NU);
        assert_eq!(r4.status, Status::Graa);
        assert_eq!(beslut(&[r4], &std, &intet(), NU), None);
    }

    /// Indstillingerne fra SQLite: standarden uden gemte værdier, de gemte værdier, og ukendte koder tæller som standarden.
    #[test]
    fn indst_fra_lageret_standard_og_gemte_vaerdier() {
        let mut s = Store::i_hukommelse().unwrap();
        assert_eq!(indst(&s).unwrap(), Indst::default(), "intet gemt: Standard, rødt til");
        s.gem_indstilling(NIVEAU, "alt").unwrap();
        s.gem_indstilling(ROED, "0").unwrap();
        s.gem_indstilling(ROLIG_SIDST, "2026-11-03").unwrap();
        s.gem_notifikation_zone("z1", Some("fra")).unwrap();
        s.gem_notifikation_zone("z2", Some("ukendt")).unwrap();
        let i = indst(&s).unwrap();
        assert_eq!(
            (i.niveau, i.roed, i.rolig_sidst.as_deref()),
            (Niveau::Alt, false, Some("2026-11-03"))
        );
        assert_eq!(
            i.zoner,
            HashMap::from([("z1".to_string(), ZoneValg::Fra)]),
            "ukendt kode springes over"
        );
        s.gem_indstilling(NIVEAU, "noget").unwrap();
        s.gem_indstilling(ROED, "ja").unwrap();
        let i = indst(&s).unwrap();
        assert_eq!((i.niveau, i.roed), (Niveau::Standard, true), "ukendte værdier: standarden");
    }

    #[test]
    fn siden_sidst_vis_selv_og_nye() {
        let rk = [roed("a"), gul("b"), r("c", "c.wi24rd.com", Status::Groen, 0, 0.0)];
        let groen = [r("c", "c.wi24rd.com", Status::Groen, 0, 0.0)];
        let i_dag = iso(NU - 2 * T);
        let i_gaar = iso(NU - DOEGN_MS);
        assert!(
            !vis_selv(Niveau::Rolig, None, &rk, NU, &Tidszone::default()),
            "Rolig: aldrig af sig selv"
        );
        assert!(vis_selv(Niveau::Standard, None, &rk, NU, &Tidszone::default()));
        assert!(
            vis_selv(Niveau::Standard, Some(&i_gaar), &rk, NU, &Tidszone::default()),
            "første gang i døgnet"
        );
        assert!(
            !vis_selv(Niveau::Standard, Some(&i_dag), &rk, NU, &Tidszone::default()),
            "allerede vist i dag"
        );
        assert!(
            !vis_selv(Niveau::Standard, None, &groen, NU, &Tidszone::default()),
            "Standard: kun ved rødt/gult"
        );
        assert!(
            vis_selv(Niveau::Alt, Some(&i_gaar), &groen, NU, &Tidszone::default()),
            "Alt: første gang i døgnet"
        );
        assert!(!vis_selv(Niveau::Alt, Some(&i_dag), &groen, NU, &Tidszone::default()));
        let gammel = Raekke1 { gammel: true, ..gul("g") };
        assert!(
            !vis_selv(Niveau::Standard, None, std::slice::from_ref(&gammel), NU, &Tidszone::default()),
            "gamle tal tæller ikke"
        );

        assert_eq!(nye(&rk, None, NU), ["a", "b"], "aldrig vist: alle røde/gule");
        let a = aftryk(&rk, NU);
        assert_eq!(a, "a\t2026-11-02\troed\nb\t2026-11-02\tgul");
        assert!(nye(&rk, Some(&a), NU).is_empty(), "intet nyt siden");
        let b_roed = [roed("a"), roed("b"), gammel];
        assert_eq!(nye(&b_roed, Some(&a), NU), ["b"], "gul → rød er nyt");
        assert_eq!(nye(&rk, Some(&a), NU + DOEGN_MS), ["a", "b"], "nyt statusdøgn er nyt");
    }
}
