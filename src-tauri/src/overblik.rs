//! Overblikket: tal pr. zone fra cachen alene (ingen nye kald), med hele UTC-døgn, fordeling efter Cloudflares handling,
//! angreb skilt fra rutine og en grøn/gul/rød/grå status. Samme hele-døgn-tal står ved siden af W's felter i Værnet.
//!
//! Hele døgn: "i går" = sidste hele UTC-døgn (d-1); 7 døgn = d-7…d-1; 30 døgn = d-30…d-1; ugen før = d-14…d-8.
//! Statusdøgnet er i går, eller døgnet før (d-2) om natten, indtil i går er færdighentet (`grundlag`). Angrebstallene i
//! overblikket (status, årsag, "Angreb, uge") regnes alle fra statusdøgnet.
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Serialize;

use crate::cf::Zone;
use crate::dmarc::{self, DmarcFund};
use crate::dns;
use crate::store::Raekke;
use crate::teknologi::{self, Profil, TekBesked, TekN, Uventet};
use crate::vaern::{dag_ms, dag_noegle, er_faerdig, parse_iso, regel, sidst_hentet, DOEGN_MS, HENT_DAGE};

// ── Statusreglen: alle konstanter ét sted ─────────────────────────────────────────────
/// Cloudflares automatiske bot-tjek. Det er rutine, ikke angreb, og kan aldrig give gul eller rød.
pub const RUTINE: &[&str] = &["linkMaze", "botFight", "bic"];
/// Rød: angreb i går ≥ max(3 · snit, snit + 25).
pub const ROED_FAKTOR: f64 = 3.0;
pub const ROED_TILLAEG: f64 = 25.0;
/// Gul: angreb i går ≥ max(1,5 · snit, snit + 10).
pub const GUL_FAKTOR: f64 = 1.5;
pub const GUL_TILLAEG: f64 = 10.0;
/// Snittet tages over d-8…d-2 (7 døgn før i går).
pub const SNIT_FRA: i64 = 8;
pub const SNIT_TIL: i64 = 2;
/// Tallene er gamle, når "sidst hentet" er ældre end dette.
pub const GAMMEL_MS: i64 = 26 * 3_600_000;
/// Natten: fra 00:00 til 03:00 UTC er i går ofte ikke færdighentet endnu (et døgn er færdigt 1 t efter
/// midnat, og køen kører hver time). Så bygger status på døgnet før (d-2) og står som "opdateres …", ikke gul.
pub const NAT_MS: i64 = 3 * 3_600_000;
/// Så mange angrebstyper står under status.
pub const TOP_ANGREB: usize = 3;
/// Kilden i firewallEventsAdaptive, der groft svarer til Radars `mitigationProduct=WAF`.
pub const WAF_KILDE: &str = "firewallManaged";
/// Følger den globale tendens: verden steg mindst så meget (r_ref), og mit domæne højst `TENDENS_MIG_FAKTOR` gange så meget.
pub const TENDENS_VERDEN_STEG: f64 = 1.5;
pub const TENDENS_MIG_FAKTOR: f64 = 2.0;
/// Stigningen er din egen: verden steg under dette, og dit døgn er mindst `TENDENS_EGEN` gange dit snit. Målt over 30 døgn var verden flad
/// hver dag, så "flad" alene sagde ingenting.
pub const TENDENS_FLAD: f64 = 1.3;
pub const TENDENS_EGEN: f64 = 3.0;
/// "Usædvanlig meget trafik": et døgn med mindst så mange forespørgsler og mindst `TRAFIK_FAKTOR` gange domænets median for 30
/// døgn, på et domæne med forespørgsler på mindst `TRAFIK_DAGE` af dagene. Gulvet holder små domæner ude: 300 forespørgsler mod 20 er ikke værd
/// at nævne, og uden en normal at holde døgnet op imod er medianen 0. Det giver Hold øje: om det er en overbelastning, en løbsk robot eller en fejl, kan tallene ikke sige.
pub const TRAFIK_MIN: i64 = 1000;
pub const TRAFIK_FAKTOR: f64 = 10.0;
pub const TRAFIK_DAGE: usize = 7;
/// "Din server svarer ikke": mindst så mange procent af døgnets svar er `SERVER_KODER`, og døgnet har mindst `SERVER_MIN_SVAR`
/// svar, så en enkelt fejl på et stille domæne ikke giver Hold øje. Procent som heltal, så grænsen er præcis.
pub const SERVER_PROCENT: i64 = 20;
pub const SERVER_MIN_SVAR: i64 = 50;
/// Cloudflare kunne ikke få et brugbart svar fra serveren. Cloudflares fejlside beskriver 520–526; 527 står med, så hele rækken er dækket. 530
/// (V-DRIFT.1 B2, D4): Cloudflares side "Error 530" siger, at Cloudflare ikke kan slå serverens værtsnavn op og derfor ikke kan få forbindelse til
/// serveren. 528 og 529 er ikke med: Cloudflares oversigt over 5xx-fejl nævner dem ikke.
pub struct ServerKoder;

impl ServerKoder {
    pub fn contains(&self, kode: &i64) -> bool {
        (520..=527).contains(kode) || *kode == 530
    }
}

pub const SERVER_KODER: ServerKoder = ServerKoder;
/// "Ligner rekognoscering" (R1): døgnet har mindst så mange forskellige stier med 404 og mindst `REKOG_FAKTOR` gange medianen af de `REKOG_BASIS` døgn før.
pub const REKOG_MIN: i64 = 100;
pub const REKOG_FAKTOR: f64 = 3.0;
pub const REKOG_BASIS: i64 = 7;
/// Et nyt mærke er en ny episode: først efter så mange stille døgn.
pub const REKOG_STILLE: i64 = 7;
/// Under så mange procent stier, der kun ses her i de `REKOG_EGNE_DOEGN` seneste døgn, er det en almindelig scanner.
pub const REKOG_EGNE_PROCENT: i64 = 30;
pub const REKOG_EGNE_DOEGN: i64 = 8;
/// En sti, der passer til en "Bruger vi"-teknologi, tæller så mange gange i R1.
pub const REKOG_PASSER_VAEGT: i64 = 2;
/// "Ligner målrettet" (Q5, M4): ét netværk står for mindst så mange procent af døgnets angreb.
pub const MAAL_NET_PROCENT: i64 = 60;
/// Mindst så mange valgte domæner med tal, det selv medregnet. Færre er ingen sammenligning: M1 alene skelner ikke (Q5), og Radar frasorterede intet på de målte data.
pub const MAAL_MIN_DOMAENER: usize = 3;
/// Højst så mange andre domæner må være gul/rød samme dag ±1.
pub const MAAL_ANDRE_MAKS: usize = 1;
/// "Målrettet": netværket må gerne røre de andre valgte domæner ±1, men højst så mange procent af det, det gjorde her. 1.800 på ét domæne og 5 på de andre dagen efter er stadig rettet mod det ene.
pub const MAAL_NET_ANDRE_PROCENT: i64 = 10;
/// Når Radar har tal: verden steg under dette.
pub const MAAL_VERDEN: f64 = TENDENS_FLAD;
/// "Stigningen er din egen" gælder ikke, når så mange af de andre domæner også steg samme dag ±1.
pub const BOELGE_ANDRE: usize = 2;

/// Sorteringsrækkefølgen er rød, gul, grøn, grå.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Roed,
    Gul,
    Groen,
    Graa,
}

/// Rød, gul eller grøn ud fra angreb i går (`a`) og snittet pr. døgn før (`s`). Ren funktion.
pub fn status_for_angreb(a: i64, s: f64) -> Status {
    let a = a as f64;
    if a >= (ROED_FAKTOR * s).max(s + ROED_TILLAEG) {
        Status::Roed
    } else if a >= (GUL_FAKTOR * s).max(s + GUL_TILLAEG) {
        Status::Gul
    } else {
        Status::Groen
    }
}

/// Hvilket døgn statusreglen bygger på, eller hvorfor den ikke kan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grundlag {
    /// Intet hentet: grå.
    Intet,
    /// Sidst hentet > 26 t, eller i går er ikke færdighentet uden for natten: gul "tallene er gamle".
    Gammel,
    /// I går (d-1) er færdighentet.
    IGaar,
    /// Natten: i går er ikke færdighentet endnu, men døgnet før (d-2) er. Status bygger på d-2 ("opdateres …").
    Forgaars,
}

impl Grundlag {
    /// Hvor mange døgn statusdøgnet ligger før i går.
    pub fn forskyd(self) -> i64 {
        i64::from(self == Grundlag::Forgaars)
    }
}

/// Statusdøgnet for en zone ud fra `hentet` og `nu`. Ren funktion.
pub fn grundlag(hentet: &HashMap<String, String>, nu: i64) -> Grundlag {
    let Some(sidst) = sidst_hentet(hentet, nu) else {
        return Grundlag::Intet;
    };
    if parse_iso(&sidst).is_none_or(|ms| nu - ms > GAMMEL_MS) {
        return Grundlag::Gammel;
    }
    let faerdig = |i: i64| {
        let d = dag_noegle(nu - i * DOEGN_MS);
        hentet.get(&d).is_some_and(|h| er_faerdig(&d, h))
    };
    if faerdig(1) {
        return Grundlag::IGaar;
    }
    let efter_midnat = dag_ms(&dag_noegle(nu)).map_or(i64::MAX, |m| nu - m);
    if efter_midnat < NAT_MS && faerdig(2) {
        Grundlag::Forgaars
    } else {
        Grundlag::Gammel
    }
}

/// Status og `gammel` (gul, fordi tallene er gamle, ikke på grund af angreb) for et grundlag, angreb i statusdøgnet
/// (`a`) og snittet før (`s`). Ren funktion.
pub fn status(g: Grundlag, a: i64, s: f64) -> (Status, bool) {
    match g {
        Grundlag::Intet => (Status::Graa, false),
        Grundlag::Gammel => (Status::Gul, true),
        Grundlag::IGaar | Grundlag::Forgaars => (status_for_angreb(a, s), false),
    }
}

// ── Hele døgn og fordeling ─────────────────────────────────────────────────────────────────────────────────────────
/// Handlingens gruppe (J9). `link_maze_injected` betyder, at siden blev leveret med skjulte fælde-links (Cloudflares AI
/// Labyrinth-dokumentation): intet blev afvist. Først `link_maze_visited`, en crawler, der fulgte et fælde-link, er
/// labyrinten. Dyk filtrerer på grupperne, ikke på de rå koder.
pub fn gruppe(handling: &str) -> &'static str {
    match handling {
        "block" => BLOKERET,
        "managed_challenge" | "challenge" | "js_challenge" => UDFORDRET,
        "link_maze_visited" => LABYRINT,
        "link_maze_injected" => FAELDER,
        _ => ANDET,
    }
}

pub const BLOKERET: &str = "blokeret";
pub const UDFORDRET: &str = "udfordret";
pub const LABYRINT: &str = "labyrint";
pub const FAELDER: &str = "faelder";
pub const ANDET: &str = "andet";

/// "Afvist" = blokeret + udfordret + lokket ind i labyrinten (J9). Sider med fælde-links og andre handlinger (fx `log`) er
/// ikke afvist.
pub fn er_afvist(handling: &str) -> bool {
    matches!(gruppe(handling), BLOKERET | UDFORDRET | LABYRINT)
}

/// Hvad Cloudflare gjorde ved det, den så.
#[derive(Serialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct Fordeling {
    /// `block`
    pub blokeret: i64,
    /// `managed_challenge`, `challenge`, `js_challenge`
    pub udfordret: i64,
    /// `link_maze_visited` (lokket ind i labyrinten)
    pub labyrint: i64,
    pub andet: i64,
    /// `link_maze_injected`: sider leveret med skjulte fælde-links. Tæller ikke som afvist.
    pub faelder: i64,
}

impl Fordeling {
    pub fn tael(&mut self, handling: &str, n: i64) {
        let felt = match gruppe(handling) {
            BLOKERET => &mut self.blokeret,
            UDFORDRET => &mut self.udfordret,
            LABYRINT => &mut self.labyrint,
            FAELDER => &mut self.faelder,
            _ => &mut self.andet,
        };
        *felt += n;
    }
}

#[derive(Serialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct Fordelinger {
    pub d30: Fordeling,
    pub i_gaar: Fordeling,
}

/// De hele døgn ("afvist", se `er_afvist`), ved siden af W's felter i Værnet og i overblikket. `fordeling` har også det,
/// der ikke er afvist (`faelder`, `andet`).
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Hele {
    pub i_gaar: i64,
    pub i_dag_hidtil: i64,
    pub hele7: i64,
    pub hele30: i64,
    pub ugen_foer: i64,
    /// Alle forespørgsler d-7…d-1 (til "1 af hver N").
    pub alle_hele7: i64,
    pub fordeling: Fordelinger,
}

/// Første døgn, der skal læses fra cachen for at have alle hele døgn (d-30).
pub fn hele_fra(nu: i64) -> String {
    dag_noegle(nu - HENT_DAGE * DOEGN_MS)
}

/// Rækker (flere zoner må gerne blandes) → hele døgn. Ren funktion.
pub fn hele_doegn(rows: &[Raekke], nu: i64) -> Hele {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    let (d0, d1, d7, d8, d14, d30) = (d(0), d(1), d(7), d(8), d(14), d(30));
    let mellem = |dag: &str, fra: &str, til: &str| dag >= fra && dag <= til;
    let mut h = Hele::default();
    for r in rows {
        let dag = r.dag.as_str();
        if r.art == "alle" && mellem(dag, &d7, &d1) {
            h.alle_hele7 += r.n;
        }
        if r.art != "fw" {
            continue;
        }
        let handling = r.noegle.split('\t').nth(7).unwrap_or("");
        if dag == d1 {
            h.fordeling.i_gaar.tael(handling, r.n);
        }
        if mellem(dag, &d30, &d1) {
            h.fordeling.d30.tael(handling, r.n);
        }
        if !er_afvist(handling) {
            continue;
        }
        if dag == d0 {
            h.i_dag_hidtil += r.n;
        }
        if dag == d1 {
            h.i_gaar += r.n;
        }
        if mellem(dag, &d7, &d1) {
            h.hele7 += r.n;
        }
        if mellem(dag, &d30, &d1) {
            h.hele30 += r.n;
        }
        if mellem(dag, &d14, &d8) {
            h.ugen_foer += r.n;
        }
    }
    h
}

// ── Angreb (ikke rutine), regnet fra statusdøgnet ──────────────────────────────────────────────────────────────────
/// En angrebstype og hvor mange gange, den blev stoppet i statusdøgnet. `egen`: brugerens egen regel (kilden er en af
/// `EGNE_KILDER`); UI'et viser den som "din egen regel »navn«" i stedet for regelens beskrivelse. `regler`: de fulde navne bag
/// det korte navn (det, dyk filtrerer på).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct AngrebType {
    pub navn: String,
    pub n: i64,
    pub egen: bool,
    pub regler: Vec<String>,
    /// Teknologien bag reglen (`teknologi::af_fw`), så råd om noget, brugeren ikke kører, kan udelades.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tek: Option<String>,
}

/// Kilder (`source` i firewallEventsAdaptive) for brugerens egne regler. Kun de to, der er set i rigtige data:
/// `firewallCustom` (W's paritetsdata) og `ratelimit` (en egen rate limit). Andre kilder står som "andet" i UI'et.
pub const EGNE_KILDER: &[&str] = &["firewallCustom", "ratelimit"];

/// En regels art ud fra kilden: "rutine" (grå), "egen" eller "angreb". Regelkort og dyk viser den.
pub fn regel_art(kilde: &str) -> &'static str {
    if RUTINE.contains(&kilde) {
        "rutine"
    } else if EGNE_KILDER.contains(&kilde) {
        "egen"
    } else {
        "angreb"
    }
}

/// Angreb i forhold til statusdøgnet `s` (i går, eller d-2 om natten): `i_gaar` = s; snittet = s-7…s-1;
/// `uge` = s-6…s; `uge_foer` = s-13…s-7.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Angreb {
    pub i_gaar: i64,
    /// I alt i de 7 døgn før statusdøgnet (til snittet).
    pub foer: i64,
    /// De største angrebstyper i statusdøgnet (højst `TOP_ANGREB`).
    pub top: Vec<AngrebType>,
    /// Hvad Cloudflare gjorde ved angrebene i statusdøgnet.
    pub fordeling: Fordeling,
    pub uge: i64,
    pub uge_foer: i64,
}

impl Angreb {
    pub fn snit(&self) -> f64 {
        self.foer as f64 / (SNIT_FRA - SNIT_TIL + 1) as f64
    }
}

/// Regelnavnet, som Værnets regelkort viser det: produkt og type med CVE ("React RCE (CVE-2025-55182)"), ellers kun
/// produktet ("WordPress"); regler uden " - "-opdeling (egne regler, browsertjekket) med deres fulde navn. Produktet staves som i kataloget, men en egen regel beholder det navn, brugeren gav den.
pub fn kort_regelnavn(navn: &str, egen: bool) -> String {
    let bic = navn == "Browser Integrity Check";
    let r = regel(if bic { "" } else { navn }, if bic { "bic" } else { "" });
    let produkt = if egen {
        r.produkt.clone()
    } else {
        teknologi::vist_produkt(&r.produkt)
    };
    match (&r.cve, r.produkt.is_empty()) {
        (_, true) => r.navn,
        (Some(cve), false) if !r.type_.is_empty() => format!("{produkt} {} ({cve})", r.type_),
        (Some(cve), false) => format!("{produkt} ({cve})"),
        (None, false) => produkt,
    }
}

/// Rækker → angreb i forhold til `nu` (statusdøgnet = døgnet før `nu`). Ren funktion.
pub fn angreb(rows: &[Raekke], nu: i64) -> Angreb {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    let (d1, d7, d8, d14) = (d(1), d(7), d(8), d(14));
    let (snit_fra, snit_til) = (d(SNIT_FRA), d(SNIT_TIL));
    let mellem = |dag: &str, fra: &str, til: &str| dag >= fra && dag <= til;
    let mut a = Angreb::default();
    let mut typer: Vec<AngrebType> = Vec::new();
    for r in rows.iter().filter(|r| r.art == "fw") {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        if RUTINE.contains(&felt(6)) {
            continue;
        }
        let dag = r.dag.as_str();
        if dag == d1 {
            a.i_gaar += r.n;
            a.fordeling.tael(felt(7), r.n);
            let egen = EGNE_KILDER.contains(&felt(6));
            let navn = kort_regelnavn(felt(0), egen);
            let fuldt = felt(0).to_string();
            match typer.iter_mut().find(|t| t.navn == navn) {
                Some(t) => {
                    t.n += r.n;
                    t.egen |= egen;
                    if !t.regler.contains(&fuldt) {
                        t.regler.push(fuldt);
                    }
                }
                None => typer.push(AngrebType {
                    navn,
                    n: r.n,
                    egen,
                    tek: teknologi::af_fw(felt(0), felt(3), felt(6)),
                    regler: vec![fuldt],
                }),
            }
        }
        if mellem(dag, &snit_fra, &snit_til) {
            a.foer += r.n;
        }
        if mellem(dag, &d7, &d1) {
            a.uge += r.n;
        }
        if mellem(dag, &d14, &d8) {
            a.uge_foer += r.n;
        }
    }
    typer.sort_by(|x, y| y.n.cmp(&x.n).then_with(|| x.navn.cmp(&y.navn)));
    typer.truncate(TOP_ANGREB);
    a.top = typer;
    a
}

// ── Sammenligningen med verden (Q4) ─────────────────────────────────────────────────────────────────────────────────
/// Hvad sammenligningen siger om et gult eller rødt døgn. Den siger kun noget om WAF-angreb generelt, aldrig om en bestemt regel,
/// og den ændrer aldrig status.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tendens {
    Foelger,
    SaerligtDig,
}

/// Mit tal i statusdøgnet ÷ snittet af de 7 døgn før, kun for `WAF_KILDE`; `nu` som i `angreb`. Uden WAF-angreb i døgnet er der intet at
/// sammenligne (`None`), og uden nogen før er forholdet uendeligt. Ren funktion.
pub fn r_mig(rows: &[Raekke], nu: i64) -> Option<f64> {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    let (d1, fra, til) = (d(1), d(SNIT_FRA), d(SNIT_TIL));
    let (mut doegn, mut foer) = (0, 0);
    for r in rows
        .iter()
        .filter(|r| r.art == "fw" && r.noegle.split('\t').nth(6) == Some(WAF_KILDE))
    {
        if r.dag == d1 {
            doegn += r.n;
        } else if r.dag >= fra && r.dag <= til {
            foer += r.n;
        }
    }
    if doegn == 0 {
        return None;
    }
    let snit = foer as f64 / (SNIT_FRA - SNIT_TIL + 1) as f64;
    Some(if snit == 0.0 { f64::INFINITY } else { doegn as f64 / snit })
}

/// Q4-tabellen. Kun gul og rød kan få en sætning: små tal når ikke dertil (`status_for_angreb`). Ren funktion.
pub fn tendens(status: Status, gammel: bool, r_mig: Option<f64>, r_ref: Option<f64>) -> Option<Tendens> {
    if gammel || !matches!(status, Status::Roed | Status::Gul) {
        return None;
    }
    let (mig, verden) = (r_mig?, r_ref?);
    if verden >= TENDENS_VERDEN_STEG && mig <= TENDENS_MIG_FAKTOR * verden {
        Some(Tendens::Foelger)
    } else if verden < TENDENS_FLAD && mig >= TENDENS_EGEN {
        Some(Tendens::SaerligtDig)
    } else {
        None
    }
}

// ── Mønstre: "Ligner rekognoscering" og "Ligner målrettet" ──────────────────────────────────────────────────────
/// Et mærke oven på status (J5): status måler mængde, mærket mønster, så en række kan være Roligt og have mærket. Altid "ligner", aldrig en dom.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MoensterSlags {
    Rekognoscering,
    DnsRekognoscering,
    Maalrettet,
}

impl MoensterSlags {
    /// "Ligner rekognoscering" over HTTP eller DNS: samme notifikation.
    pub fn er_rekognoscering(self) -> bool {
        matches!(self, MoensterSlags::Rekognoscering | MoensterSlags::DnsRekognoscering)
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Moenster {
    pub slags: MoensterSlags,
    pub dag: String,
    /// Rekognoscering: forskellige stier i døgnet og medianen før, uden "Bruger vi ikke". DNS: tallet, der slog til, og snittet før.
    pub stier: i64,
    pub normalt: f64,
    /// DNS: tallet er forskellige navne, ikke opslag.
    pub dns_navne: bool,
    /// Under `REKOG_EGNE_PROCENT` stier kun her: det ligner en almindelig scanner.
    pub scanner: bool,
    /// Første døgn i episoden. Kun det kan give besked.
    pub ny: bool,
    /// Målrettet: det ene netværk og dets andel af døgnets angreb.
    pub netvaerk: Option<String>,
    pub procent: i64,
    /// Radar har tal for døgnet: "resten af verden" kan siges. Ellers er det kun dine andre domæner.
    pub verden: bool,
    /// Den største "Bruger vi"-teknologi, døgnet ramte.
    pub tek: Option<String>,
    /// Målrettet: døgnets angreb og det, Cloudflare stoppede. Mærket kan handle om døgnet før statusdøgnet.
    pub angreb: i64,
    pub stoppet: i64,
    /// Målrettet: døgnet efter er ikke hentet færdigt, så "±1" er ikke afgjort. Giver ingen besked.
    pub foreloebig: bool,
}

/// Så mange døgn bag statusdøgnet ser en vurdering højst tilbage, når programmet har været slukket.
pub const MAERKE_BAGUD: i64 = 7;

/// Det, en række viser, og det, der er afgjort og kan give besked.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Moenstre {
    pub vis: Option<Moenster>,
    pub afgjort: Option<Moenster>,
    pub efterslaeb: Vec<Moenster>,
    pub vurderet: Option<String>,
}

/// Det, ét valgt domæne bidrager med til de andres sammenligning. Altid alle angreb uden hensyn til dets egne valg: en bølge er en bølge, også på et domæne, der ikke bruger teknologien.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Signal {
    /// Domænet, så boksen "Vurdering" kan sige hvilket.
    pub zone: String,
    pub navn: String,
    /// Dage med gul/rød.
    stiger: HashSet<String>,
    /// Angrebene pr. netværk og stierne med 404 pr. dag.
    net: BTreeMap<String, HashMap<String, i64>>,
    stier: BTreeMap<String, HashSet<String>>,
    /// Dage med tal, og dage, der er hentet færdigt.
    tal: HashSet<String>,
    faerdige: HashSet<String>,
}

/// Netværket i en `fw`-række, der er et angreb. Ren funktion.
fn angreb_net(r: &Raekke) -> Option<&str> {
    if r.art != "fw" {
        return None;
    }
    let f: Vec<&str> = r.noegle.split('\t').collect();
    (!RUTINE.contains(&f.get(6).copied().unwrap_or(""))).then(|| f.get(2).copied().unwrap_or(""))
}

pub fn dag_foer(dag: &str, i: i64) -> String {
    dag_ms(dag).map_or_else(String::new, |ms| dag_noegle(ms - i * DOEGN_MS))
}

/// Ét domænes rækker og `hentet` → dets signal. Ren funktion.
pub fn signal(rows: &[Raekke], hentet: &HashMap<String, String>) -> Signal {
    let mut s = Signal {
        faerdige: hentet.iter().filter(|(d, h)| er_faerdig(d, h)).map(|(d, _)| d.clone()).collect(),
        ..Signal::default()
    };
    let mut pr_dag: BTreeMap<&str, i64> = BTreeMap::new();
    for r in rows {
        if !s.tal.contains(&r.dag) {
            s.tal.insert(r.dag.clone());
        }
        if let Some(net) = angreb_net(r) {
            *pr_dag.entry(&r.dag).or_default() += r.n;
            if !net.is_empty() {
                *s.net.entry(r.dag.clone()).or_default().entry(net.to_string()).or_default() += r.n;
            }
        } else if r.art == "probe" {
            s.stier.entry(r.dag.clone()).or_default().insert(r.noegle.clone());
        }
    }
    for (dag, a) in &pr_dag {
        let foer: i64 = (1..=SNIT_FRA - SNIT_TIL + 1)
            .map(|i| pr_dag.get(dag_foer(dag, i).as_str()).copied().unwrap_or(0))
            .sum();
        if status_for_angreb(*a, foer as f64 / (SNIT_FRA - SNIT_TIL + 1) as f64) != Status::Groen {
            s.stiger.insert(dag.to_string());
        }
    }
    s
}

/// Dagen og dens naboer: "samme dag ±1".
fn naboer(dag: &str) -> [String; 3] {
    [dag_foer(dag, 1), dag.to_string(), dag_foer(dag, -1)]
}

/// Så mange af de andre domæner steg samme dag ±1. Ren funktion.
pub fn stiger_ogsaa(andre: &[&Signal], dag: &str) -> usize {
    let dage = naboer(dag);
    andre.iter().filter(|s| dage.iter().any(|d| s.stiger.contains(d))).count()
}

/// Mindst `MAAL_MIN_DOMAENER` valgte domæner med tal for dagen, domænet selv medregnet. Ren funktion.
pub fn kan_sammenligne(andre: &[&Signal], dag: &str) -> bool {
    1 + andre.iter().filter(|s| s.tal.contains(dag)).count() >= MAAL_MIN_DOMAENER
}

/// Bølge-reglen i Hvorfor: stiger de andre domæner også, er stigningen ikke din egen, uanset hvad verden gjorde. Ren funktion.
pub fn efter_boelgen(t: Option<Tendens>, andre: &[&Signal], dag: &str) -> Option<Tendens> {
    t.filter(|t| *t != Tendens::SaerligtDig || stiger_ogsaa(andre, dag) < BOELGE_ANDRE)
}

/// Står statusdøgnet to eller flere lokale dage tilbage? Så er det "i forgårs", også når det er sidste hele døgn i UTC. Ren funktion.
pub fn er_forgaars(status_dag: &str, lokal_dag: &str) -> bool {
    lokal_dag > dag_foer(status_dag, -1).as_str()
}

/// Forskellige stier i døgnet, uden "Bruger vi ikke": (stierne, de vægtede). Stier, der passer til en "Bruger vi"-teknologi, tæller `REKOG_PASSER_VAEGT` gange.
fn dagens_stier<'a>(rows: &'a [Raekke], dag: &str, p: &Profil) -> (HashSet<&'a str>, i64) {
    let (mut stier, mut vaegt) = (HashSet::new(), 0);
    for r in rows.iter().filter(|r| r.art == "probe" && r.dag == dag) {
        let valg = teknologi::af_sti(&r.noegle).and_then(|t| p.get(t)).map(|v| v.valg);
        if valg != Some(teknologi::Valg::BrugerIkke) && stier.insert(r.noegle.as_str()) {
            vaegt += if valg == Some(teknologi::Valg::Bruger) {
                REKOG_PASSER_VAEGT
            } else {
                1
            };
        }
    }
    (stier, vaegt)
}

/// R1's to betingelser for ét døgn, på de vægtede stier.
fn r1_vurder(rows: &[Raekke], dag: &str, p: &Profil) -> Vec<Betingelse> {
    let vaegt = |d: &str| dagens_stier(rows, d, p).1;
    let mut foer: Vec<i64> = (1..=REKOG_BASIS).map(|i| vaegt(&dag_foer(dag, i))).collect();
    let (nu, m) = (vaegt(dag), median(&mut foer));
    vec![
        Betingelse {
            id: "stier",
            ok: Some(nu >= REKOG_MIN),
            n: nu as f64,
            m: REKOG_MIN as f64,
            ..Betingelse::default()
        },
        Betingelse {
            id: "faktor",
            ok: Some(nu as f64 >= REKOG_FAKTOR * m),
            n: nu as f64,
            m: REKOG_FAKTOR * m,
            uden_normal: m == 0.0,
            ..Betingelse::default()
        },
    ]
}

/// R1 for ét døgn: tallet og medianen før, hvis døgnet slår til.
fn r1(rows: &[Raekke], dag: &str, p: &Profil) -> Option<(i64, f64)> {
    opfyldt(&r1_vurder(rows, dag, p)).then(|| {
        let mut raa: Vec<i64> = (1..=REKOG_BASIS)
            .map(|i| dagens_stier(rows, &dag_foer(dag, i), p).0.len() as i64)
            .collect();
        (dagens_stier(rows, dag, p).0.len() as i64, median(&mut raa))
    })
}

/// Den største "Bruger vi"-teknologi i døgnets angreb og stier.
fn passer_tek(rows: &[Raekke], dag: &str, p: &Profil) -> Option<String> {
    let mut sum: BTreeMap<String, i64> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.dag == dag) {
        let t = match r.art.as_str() {
            "fw" => {
                let f: Vec<&str> = r.noegle.split('\t').collect();
                let felt = |i: usize| f.get(i).copied().unwrap_or("");
                teknologi::af_fw(felt(0), felt(3), felt(6))
            }
            "probe" => teknologi::af_sti(&r.noegle).map(str::to_string),
            _ => None,
        };
        if let Some(t) = t.filter(|t| p.get(t).is_some_and(|v| v.valg == teknologi::Valg::Bruger)) {
            *sum.entry(t).or_default() += r.n;
        }
    }
    sum.into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|x| x.0)
}

/// En betingelse i boksen "Vurdering". Mærket afgøres af de samme tal (`m4_vurder`, `r1_vurder`), så boksen kan ikke sige noget andet end mærket. `ok` er `None`, når den ikke kan vurderes (Radar har
/// ikke dagen), og så tæller den ikke imod.
#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct Betingelse {
    pub id: &'static str,
    pub ok: Option<bool>,
    pub n: f64,
    pub m: f64,
    pub navn: Option<String>,
    pub domaener: Vec<DomaeneTal>,
    /// Der er intet normalt at sammenligne med (det normale er 0, eller der er for få dage). Ændrer kun visningen: `ok` afgør stadig mærket.
    pub uden_normal: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DomaeneTal {
    pub zone: String,
    pub navn: String,
    pub n: i64,
}

/// Hvorfor mærket står eller ikke står for én dag. `foreloebig`: "Ligner målrettet" er opfyldt, men døgnet efter mangler.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Vurdering {
    pub dag: String,
    pub maal: Vec<Betingelse>,
    pub maal_ja: bool,
    pub foreloebig: bool,
    pub rekog: Vec<Betingelse>,
    pub rekog_ja: bool,
    /// "Ligner DNS-rekognoscering": én af linjerne er nok.
    pub dns: Vec<Betingelse>,
    pub dns_ja: bool,
}

fn opfyldt(b: &[Betingelse]) -> bool {
    b.iter().all(|x| x.ok != Some(false))
}

/// M4 for én dag: stigning + ét netværk ≥ 60 % + netværket næsten ikke på andre valgte domæner ±1 + højst ét andet domæne gul/rød ±1, og når Radar findes: verden steg under 1,3. Kræver
/// `MAAL_MIN_DOMAENER` valgte domæner. Stigningen regnes på de relevante angreb, som status gør det.
fn m4_vurder(dag: &str, rows: &[Raekke], p: &Profil, andre: &[&Signal], r_ref: Option<f64>) -> (Vec<Betingelse>, Option<Moenster>) {
    let valgte = Betingelse {
        id: "valgte",
        ok: Some(kan_sammenligne(andre, dag)),
        n: (1 + andre.iter().filter(|s| s.tal.contains(dag)).count()) as f64,
        m: MAAL_MIN_DOMAENER as f64,
        ..Betingelse::default()
    };
    let radar = Betingelse {
        id: "radar",
        ok: r_ref.map(|v| v < MAAL_VERDEN),
        n: r_ref.unwrap_or(0.0),
        m: MAAL_VERDEN,
        ..Betingelse::default()
    };
    let Some(ms) = dag_ms(dag) else {
        return (vec![valgte, radar], None);
    };
    let fra = dag_foer(dag, SNIT_FRA);
    let relevante: Vec<Raekke> = rows
        .iter()
        .filter(|x| x.art == "fw" && x.dag >= fra && x.dag.as_str() <= dag && !teknologi::udeladt(x, p))
        .cloned()
        .collect();
    let a = angreb(&relevante, ms + DOEGN_MS);
    let mut net = BTreeMap::<&str, i64>::new();
    for x in relevante.iter().filter(|x| x.dag == dag) {
        if let Some(n) = angreb_net(x) {
            *net.entry(n).or_default() += x.n;
        }
    }
    let (top, n) = net
        .into_iter()
        .max_by(|x, y| x.1.cmp(&y.1).then_with(|| y.0.cmp(x.0)))
        .unwrap_or(("", 0));
    let procent = if a.i_gaar > 0 {
        (n as f64 * 100.0 / a.i_gaar as f64).round() as i64
    } else {
        0
    };
    let dage = naboer(dag);
    let domaene = |s: &Signal, n: i64| DomaeneTal {
        zone: s.zone.clone(),
        navn: s.navn.clone(),
        n,
    };
    let paa_andre: Vec<DomaeneTal> = andre
        .iter()
        .filter_map(|s| {
            let k: i64 = dage.iter().filter_map(|d| s.net.get(d)?.get(top).copied()).sum();
            (k > 0).then(|| domaene(s, k))
        })
        .collect();
    let andre_i_alt: i64 = paa_andre.iter().map(|x| x.n).sum();
    let gule: Vec<DomaeneTal> = andre
        .iter()
        .filter(|s| dage.iter().any(|d| s.stiger.contains(d)))
        .map(|s| domaene(s, 0))
        .collect();
    let b = vec![
        Betingelse {
            id: "stigning",
            ok: Some(status_for_angreb(a.i_gaar, a.snit()) != Status::Groen),
            n: a.i_gaar as f64,
            m: a.snit(),
            ..Betingelse::default()
        },
        Betingelse {
            id: "netvaerk",
            ok: Some(!top.is_empty() && n * 100 >= MAAL_NET_PROCENT * a.i_gaar),
            n: procent as f64,
            m: MAAL_NET_PROCENT as f64,
            navn: Some(top.to_string()).filter(|x| !x.is_empty()),
            ..Betingelse::default()
        },
        Betingelse {
            id: "net_andre",
            ok: Some(andre_i_alt * 100 <= MAAL_NET_ANDRE_PROCENT * n),
            n: andre_i_alt as f64,
            m: n as f64,
            navn: Some(top.to_string()).filter(|x| !x.is_empty()),
            domaener: paa_andre,
            ..Betingelse::default()
        },
        Betingelse {
            id: "gule",
            ok: Some(gule.len() <= MAAL_ANDRE_MAKS),
            n: gule.len() as f64,
            m: MAAL_ANDRE_MAKS as f64,
            domaener: gule,
            ..Betingelse::default()
        },
        valgte,
        radar,
    ];
    let m = opfyldt(&b).then(|| Moenster {
        slags: MoensterSlags::Maalrettet,
        dns_navne: false,
        dag: dag.to_string(),
        stier: 0,
        normalt: 0.0,
        scanner: false,
        ny: true,
        netvaerk: Some(top.to_string()),
        procent,
        verden: r_ref.is_some(),
        tek: passer_tek(rows, dag, p),
        angreb: a.i_gaar,
        stoppet: a.fordeling.blokeret + a.fordeling.udfordret + a.fordeling.labyrint,
        foreloebig: false,
    });
    (b, m)
}

fn m4(dag: &str, rows: &[Raekke], p: &Profil, andre: &[&Signal], r_ref: Option<f64>) -> Option<Moenster> {
    m4_vurder(dag, rows, p, andre, r_ref).1
}

/// Boksen "Vurdering" for én dag. Ren funktion.
pub fn vurdering(dag: &str, rows: &[Raekke], p: &Profil, egen: &Signal, andre: &[&Signal], r_ref: Option<f64>) -> Vurdering {
    let (mut maal, m) = m4_vurder(dag, rows, p, andre, r_ref);
    let efter = dag_foer(dag, -1);
    let venter: Vec<DomaeneTal> = std::iter::once(egen)
        .chain(andre.iter().copied().filter(|s| s.tal.contains(dag)))
        .filter(|s| !s.faerdige.contains(&efter))
        .map(|s| DomaeneTal {
            zone: s.zone.clone(),
            navn: s.navn.clone(),
            n: 0,
        })
        .collect();
    let klar = afgjort_dag(dag, egen, andre);
    maal.push(Betingelse {
        id: "efter",
        ok: Some(klar),
        domaener: venter,
        ..Betingelse::default()
    });
    let rekog = r1_vurder(rows, dag, p);
    let dns = dns::vurder(rows, dag);
    Vurdering {
        dag: dag.to_string(),
        maal_ja: m.is_some(),
        foreloebig: m.is_some() && !klar,
        maal,
        rekog_ja: opfyldt(&rekog),
        rekog,
        dns_ja: dns.iter().any(|b| b.ok == Some(true)),
        dns,
    }
}

/// Er "±1" afgjort? Kun når døgnet efter er hentet færdigt for domænet og de valgte domæner, der har tal for dagen.
fn afgjort_dag(dag: &str, egen: &Signal, andre: &[&Signal]) -> bool {
    let efter = dag_foer(dag, -1);
    egen.faerdige.contains(&efter) && andre.iter().all(|s| !s.tal.contains(dag) || s.faerdige.contains(&efter))
}

/// R1 for statusdøgnet. Målt mod de valgte domæner.
fn rekognoscering(dag: &str, rows: &[Raekke], p: &Profil, andre: &[&Signal]) -> Option<Moenster> {
    let (stier, normalt) = r1(rows, dag, p)?;
    let ny = !(1..=REKOG_STILLE).any(|i| r1(rows, &dag_foer(dag, i), p).is_some());
    let (mine, _) = dagens_stier(rows, dag, p);
    let hos_andre = |sti: &str| {
        andre
            .iter()
            .any(|s| (0..REKOG_EGNE_DOEGN).any(|i| s.stier.get(&dag_foer(dag, i)).is_some_and(|x| x.contains(sti))))
    };
    let kun_her = mine.iter().filter(|x| !hos_andre(x)).count() as i64;
    Some(Moenster {
        slags: MoensterSlags::Rekognoscering,
        dag: dag.to_string(),
        stier,
        normalt,
        dns_navne: false,
        scanner: !andre.is_empty() && kun_her * 100 < REKOG_EGNE_PROCENT * mine.len() as i64,
        ny,
        netvaerk: None,
        procent: 0,
        verden: false,
        tek: passer_tek(rows, dag, p),
        angreb: 0,
        stoppet: 0,
        foreloebig: false,
    })
}

/// "Ligner DNS-rekognoscering" for statusdøgnet. Som R1 er kun første døgn i en episode nyt.
fn dns_rekognoscering(dag: &str, rows: &[Raekke]) -> Option<Moenster> {
    let (stier, normalt, dns_navne) = dns::signal(rows, dag)?;
    Some(Moenster {
        slags: MoensterSlags::DnsRekognoscering,
        dag: dag.to_string(),
        stier,
        normalt,
        dns_navne,
        scanner: false,
        ny: !(1..=REKOG_STILLE).any(|i| dns::signal(rows, &dag_foer(dag, i)).is_some()),
        netvaerk: None,
        procent: 0,
        verden: false,
        tek: None,
        angreb: 0,
        stoppet: 0,
        foreloebig: false,
    })
}

/// Mærkerne for en række. "Bruger vi ikke" tæller ikke i signalerne. Statusdøgnets "Ligner målrettet" er foreløbigt, indtil døgnet efter er hentet: det står, men melder ikke. Døgnet før
/// statusdøgnet er afgjort, så det er dér, beskeden kommer fra, og er mærket væk nu, kommer der ingen. Målrettet går forud for rekognoscering. Døgn efter `vurderet` (højst `MAERKE_BAGUD`), som
/// programmet ikke nåede, fordi det var slukket, står i `efterslaeb`. `vurderet` i svaret er det seneste døgn, hvor alt frem til er afgjort: et døgn, der venter, ses igen næste gang. Ren funktion.
pub fn moenstre(
    r: &Raekke1,
    rows: &[Raekke],
    p: &Profil,
    egen: &Signal,
    andre: &[&Signal],
    r_ref: &dyn Fn(&str) -> Option<f64>,
    vurderet: Option<&str>,
) -> Moenstre {
    if r.gammel || r.status == Status::Graa {
        return Moenstre::default();
    }
    let dag = r.status_dag.as_str();
    let maal = |d: &str| {
        m4(d, rows, p, andre, r_ref(d)).map(|m| Moenster {
            foreloebig: !afgjort_dag(d, egen, andre),
            ..m
        })
    };
    let nu = maal(dag);
    let afgjort = nu
        .clone()
        .filter(|m| !m.foreloebig)
        .or_else(|| maal(&dag_foer(dag, 1)).filter(|m| !m.foreloebig));
    let vis = nu
        .or_else(|| rekognoscering(dag, rows, p, andre))
        .or_else(|| dns_rekognoscering(dag, rows))
        .or_else(|| afgjort.clone());
    // Døgnene efter `vurderet`, ældste først. Første gang er det kun døgnet før statusdøgnet, så en opdatering eller et nyvalgt domæne ikke melder gammelt nyt.
    let start = vurderet.map_or_else(|| dag_foer(dag, 1), |v| dag_foer(v, -1));
    let (mut efterslaeb, mut naaet, mut i_traek) = (Vec::new(), None, true);
    for d in (0..=MAERKE_BAGUD).rev().map(|i| dag_foer(dag, i)).filter(|d| *d >= start) {
        let klar = afgjort_dag(&d, egen, andre);
        i_traek = i_traek && klar;
        if i_traek {
            naaet = Some(d.clone());
        }
        if klar && afgjort.as_ref().is_none_or(|m| m.dag != d) {
            efterslaeb.extend(maal(&d).filter(|m| !m.foreloebig));
        }
    }
    Moenstre {
        vis,
        afgjort,
        efterslaeb,
        vurderet: naaet,
    }
}

// ── Er sitet oppe? ───────────────────────────────────────────────────────────────────────────────────────
/// Midterværdien; tom liste giver 0. Ren funktion.
pub fn median(v: &mut [i64]) -> f64 {
    v.sort_unstable();
    match v.len() {
        0 => 0.0,
        n if n % 2 == 1 => v[n / 2] as f64,
        n => (v[n / 2 - 1] + v[n / 2]) as f64 / 2.0,
    }
}

/// Disse svar fra `leakedCredentialCheckResult` betyder, at kodeordet stod i et kendt læk. `username_leaked` siger kun noget om brugernavnet, og `username_password_similar` om et lignende par, så de tæller ikke i status.
pub const LAEK_KODEORD: &[&str] = &["password_leaked", "username_and_password_leaked"];

/// Login-forsøg med et kodeord fra et kendt læk i ét døgn: alle, og dem, serveren svarede 2xx eller 302 på. Ingen sti: den må ikke nå en notifikation.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct LaekDag {
    pub dag: String,
    pub n: i64,
    pub naaet: i64,
}

/// Art `laek`: "sti\tresultat\torigin". Ren funktion.
pub fn laek_i(rows: &[Raekke], dag: &str) -> Option<LaekDag> {
    let (mut n, mut naaet) = (0, 0);
    for r in rows.iter().filter(|r| r.art == "laek" && r.dag == dag) {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        if LAEK_KODEORD.contains(&f.get(1).copied().unwrap_or("")) {
            n += r.n;
            if f.get(2).and_then(|x| x.parse().ok()).is_some_and(teknologi::er_svar) {
                naaet += r.n;
            }
        }
    }
    (n > 0).then(|| LaekDag {
        dag: dag.to_string(),
        n,
        naaet,
    })
}

/// Noget ved domænet selv hos Cloudflare: `slags` er pause, inaktiv, phishing eller udvikler. `kl`: hvornår udviklertilstanden slutter, i lokal tid.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TilstandFund {
    pub slags: &'static str,
    pub status: Option<String>,
    pub kl: Option<String>,
}

/// Zonen, som den sidst blev hentet, set fra `nu`. Udviklertilstanden slutter af sig selv, så den tælles ned fra hentetidspunktet. Ren funktion.
pub fn tilstand_fund(z: &crate::cf::ZoneStand, hentet_ms: i64, nu: i64, kl: &dyn Fn(i64) -> String) -> Vec<TilstandFund> {
    let fund = |slags, status, kl| TilstandFund { slags, status, kl };
    let mut ud = Vec::new();
    if z.paused {
        ud.push(fund("pause", None, None));
    }
    if z.status != "active" {
        ud.push(fund("inaktiv", Some(z.status.clone()), None));
    }
    if z.phishing {
        ud.push(fund("phishing", None, None));
    }
    let slut = hentet_ms + z.dev_sek * 1000;
    if z.dev_sek > 0 && slut > nu {
        ud.push(fund("udvikler", None, Some(kl(slut))));
    }
    ud
}

/// Pause, inaktiv og phishing er Handl: Cloudflare beskytter ikke domænet, eller det er mistænkt. Udviklertilstand er mindst Hold øje. Status bliver aldrig lavere end før. Ren funktion.
pub fn med_tilstand(r: &mut Raekke1, fund: Vec<TilstandFund>) {
    if fund.iter().any(|f| f.slags != "udvikler") {
        r.status = Status::Roed;
    } else if !fund.is_empty() && r.status == Status::Groen {
        r.status = Status::Gul;
    }
    r.tilstand = fund;
}

/// Cloudflares egne fund for domænet: Critical giver mindst Hold øje, aldrig Handl, og status bliver aldrig lavere end før. Ren funktion.
pub fn med_fund(r: &mut Raekke1, fund: &[crate::sikkerhed::Fund]) {
    r.cf_fund = crate::sikkerhed::hoeje(fund);
    if r.cf_fund.is_some() && r.status == Status::Groen {
        r.status = Status::Gul;
    }
}

/// Et døgn med usædvanlig meget trafik: forespørgslerne, domænets median for de 30 døgn og det, Cloudflare stoppede (`er_afvist`).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TrafikDag {
    pub dag: String,
    pub n: i64,
    pub median: f64,
    pub stoppet: i64,
}

/// De hele døgn d-30…d-1 for ét domæne, der opfylder reglen, ældste først. Kun ét domænes rækker: medianen er domænets egen. Ren funktion.
pub fn trafik_spring(rows: &[Raekke], nu: i64) -> Vec<TrafikDag> {
    let (fra, til) = (dag_noegle(nu - HENT_DAGE * DOEGN_MS), dag_noegle(nu - DOEGN_MS));
    let mut dage: Vec<(&str, i64)> = rows
        .iter()
        .filter(|r| r.art == "alle" && r.dag >= fra && r.dag <= til)
        .map(|r| (r.dag.as_str(), r.n))
        .collect();
    dage.sort_unstable();
    if dage.iter().filter(|d| d.1 > 0).count() < TRAFIK_DAGE {
        return Vec::new();
    }
    let median = median(&mut dage.iter().map(|d| d.1).collect::<Vec<_>>());
    let stoppet = |dag: &str| -> i64 {
        rows.iter()
            .filter(|r| r.art == "fw" && r.dag == dag && er_afvist(r.noegle.split('\t').nth(7).unwrap_or("")))
            .map(|r| r.n)
            .sum()
    };
    dage.into_iter()
        .filter(|(_, n)| *n >= TRAFIK_MIN && *n as f64 >= TRAFIK_FAKTOR * median)
        .map(|(dag, n)| TrafikDag {
            dag: dag.to_string(),
            n,
            median,
            stoppet: stoppet(dag),
        })
        .collect()
}

/// Et døgn, hvor Cloudflare ikke kunne nå serveren: fejlene, alle svar og fejlenes andel i hele procent.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ServerFejl {
    pub dag: String,
    pub fejl: i64,
    pub svar: i64,
    pub procent: i64,
    /// V-DRIFT.1 B2: døgnets længste spænd af timer i træk med fejl (`drift::mellem`), i brugerens lokale tid. `None`: intet spænd, eller timetallene
    /// er ikke hentet. Sættes af `byg_overblik`; ændrer aldrig status.
    pub mellem: Option<Mellem>,
}

/// "mellem kl. {fra} og {til}": klokkeslæt som programmets øvrige ("02.00"), lokal tid.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Mellem {
    pub fra: String,
    pub til: String,
}

/// Art `status` er svarkoderne fra Cloudflares edge (`adgang::ADGANG_QUERY`). Noget af det er svaret fra cachen eller stoppet undervejs og
/// har aldrig været hos serveren, så andelen er snarere for lav end for høj. Ren funktion.
pub fn server_fejl(rows: &[Raekke], dag: &str) -> Option<ServerFejl> {
    let (mut fejl, mut svar) = (0, 0);
    for r in rows.iter().filter(|r| r.art == "status" && r.dag == dag) {
        svar += r.n;
        if r.noegle.parse().is_ok_and(|k: i64| SERVER_KODER.contains(&k)) {
            fejl += r.n;
        }
    }
    (svar >= SERVER_MIN_SVAR && fejl * 100 >= SERVER_PROCENT * svar).then(|| ServerFejl {
        dag: dag.to_string(),
        fejl,
        svar,
        procent: (fejl as f64 * 100.0 / svar as f64).round() as i64,
        mellem: None,
    })
}

/// En kendt forstyrrelse samme dag som `ServerFejl`, i det land brugeren har valgt (`radar::forstyrrelse`).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Forstyrrelse {
    pub land: String,
    pub netvaerk: Option<String>,
}

// ── Én række i overblikket ─────────────────────────────────────────────────────────────────────────────────────────
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Raekke1 {
    pub id: String,
    pub navn: String,
    pub status: Status,
    /// Gul, fordi tallene er gamle (ikke på grund af angreb).
    pub gammel: bool,
    /// Natten: status bygger på døgnet før i går, fordi i går ikke er færdighentet endnu ("opdateres …").
    pub opdateres: bool,
    /// Dagene bag tallene, til Overblikkets klik: i går (d-1), statusdøgnet, første dag i "denne uge" (til og med statusdøgnet)
    /// og ugen før. Natten flytter alle på nær `i_gaar_dag` (`Grundlag::forskyd`).
    pub i_gaar_dag: String,
    pub status_dag: String,
    pub uge_fra: String,
    pub ugen_foer_fra: String,
    pub ugen_foer_til: String,
    #[serde(flatten)]
    pub hele: Hele,
    /// Ændring i procent for angreb, 7 hele døgn mod ugen før; `null`, hvis ugen før var 0 og ugen ikke er det.
    pub aendring: Option<i64>,
    /// Angreb i statusdøgnet ("i går").
    pub angreb_i_gaar: i64,
    /// De op til 3 største angrebstyper i statusdøgnet.
    pub angreb_i_gaar_top: Vec<AngrebType>,
    /// Hvad Cloudflare gjorde ved angrebene i statusdøgnet.
    pub angreb_fordeling: Fordeling,
    /// Snit pr. døgn af angreb i de 7 døgn før statusdøgnet, med én decimal.
    pub angreb_snit: f64,
    /// Angreb i 7 hele døgn til og med statusdøgnet, og i ugen før.
    pub angreb_uge: i64,
    pub angreb_ugen_foer: i64,
    pub sidst_hentet: Option<String>,
    /// Døgn i hentevinduet (i dag + 30), der er færdighentet, og i alt.
    pub doegn_faerdige: usize,
    pub doegn_i_alt: usize,
    /// Sammenligningen med verden, kun når Radar har tal for døgnet (`radar::tendens_for`).
    pub tendens: Option<Tendens>,
    /// Statusdøgnet havde usædvanlig meget trafik. Giver mindst Hold øje.
    pub trafik: Option<TrafikDag>,
    /// Cloudflare kunne ikke nå serveren i statusdøgnet. Giver mindst Hold øje.
    pub server: Option<ServerFejl>,
    /// En kendt forstyrrelse samme dag som `server`.
    pub forstyrrelse: Option<Forstyrrelse>,
    /// Det, angrebene alene gav, uden "Bruger vi ikke"-typer: sammenligningen med verden og notifikationens titel siger noget om angreb, ikke om serveren eller trafikken.
    pub angreb_status: Status,
    /// Angreb i statusdøgnet på typer, brugeren ikke bruger. De står fuldt i tallene, men tæller ikke i status.
    pub ikke_brugt: Vec<TekN>,
    /// Sikkerhedsnettet: serveren svarede på en sti af en type, brugeren ikke bruger. Giver mindst Hold øje.
    pub uventet: Option<Uventet>,
    /// Angreb på en "Bruger vi"-type nåede serveren. Giver Handl.
    pub naaet: Option<TekN>,
    /// "Giv mig besked": typer, der ses mere end normalt. Notificeres uanset niveau og ændrer ikke status.
    pub tek_besked: Vec<TekBesked>,
    /// Den største type uden et valg, til linket i Hvorfor.
    pub tek_spoerg: Option<String>,
    /// Hentede døgn af de sidste 7 og 30 hele døgn: et døgn uden tal er et hul, ikke 0.
    pub med7: i64,
    pub med30: i64,
    /// Login-forsøg med et kodeord fra et kendt læk i statusdøgnet. Mindst Hold øje, og Handl, hvis serveren svarede 2xx eller 302.
    pub laek: Option<LaekDag>,
    /// Mails i domænets navn, der fejlede DMARC, fra en ny afsender eller i en markant større andel. Mindst Hold øje.
    pub dmarc: Option<DmarcFund>,
    /// Pause, ikke aktiv, phishing eller udviklertilstand hos Cloudflare (`med_tilstand`).
    pub tilstand: Vec<TilstandFund>,
    /// En følsom fil (fx `/.env`) svarede 200 i statusdøgnet, og svaret ligner filen, ikke sidens almindelige side (`sti200::fund`). Giver Handl.
    pub sti200: Option<crate::sti200::Sti200Dag>,
    /// Cloudflares egne fund med alvor Critical (Security Center Insights, `med_fund`). Giver mindst Hold øje, aldrig Handl.
    pub cf_fund: Option<crate::sikkerhed::HoejeFund>,
    /// "Ligner rekognoscering" eller "Ligner målrettet". Ændrer aldrig status (J5).
    pub moenster: Option<Moenster>,
    /// "Ligner målrettet", der er afgjort: døgnet efter er hentet. Kun den giver besked.
    pub afgjort: Option<Moenster>,
    /// Afgjorte mærker fra døgn, der ikke er vurderet endnu, ældste først: programmet var slukket. Giver besked som `afgjort`.
    #[serde(skip)]
    pub efterslaeb: Vec<Moenster>,
    /// Det seneste døgn, hvor alt frem til er afgjort (`Store::gem_vurderet`).
    #[serde(skip)]
    pub vurderet: Option<String>,
    /// Statusdøgnet står to lokale dage tilbage: teksten siger "i forgårs", også uden `opdateres`.
    pub forgaars: bool,
    /// Der er valgte domæner nok til "Ligner målrettet".
    pub kan_sammenligne: bool,
}

pub fn aendring(uge: i64, foer: i64) -> Option<i64> {
    match (uge, foer) {
        (0, 0) => Some(0),
        (_, 0) => None,
        _ => Some((((uge - foer) as f64 / foer as f64) * 100.0).round() as i64),
    }
}

/// Én zones række uden valg af teknologi.
#[cfg(test)]
pub fn raekke(zone: &Zone, hentet: &HashMap<String, String>, rows: &[Raekke], nu: i64) -> Raekke1 {
    raekke_med(zone, hentet, rows, nu, &Profil::new())
}

/// Hentede døgn blandt de sidste `dage` hele døgn. Ren funktion.
pub fn med_data(hentet: &HashMap<String, String>, nu: i64, dage: i64) -> i64 {
    (1..=dage).filter(|i| hentet.contains_key(&dag_noegle(nu - i * DOEGN_MS))).count() as i64
}

/// Én zones række ud fra dens `hentet`, rækker fra `hele_fra(nu)` og valg af teknologi. Ren funktion.
pub fn raekke_med(zone: &Zone, hentet: &HashMap<String, String>, rows: &[Raekke], nu: i64, profil: &Profil) -> Raekke1 {
    let g = grundlag(hentet, nu);
    let s = nu - g.forskyd() * DOEGN_MS;
    let dag = |i: i64| dag_noegle(s - i * DOEGN_MS);
    let a = angreb(rows, s);
    let snit = a.snit();
    let aktuel = matches!(g, Grundlag::IGaar | Grundlag::Forgaars);
    let vurdering = if aktuel {
        teknologi::vurder(rows, profil, s)
    } else {
        teknologi::Vurdering::default()
    };
    // "Bruger vi ikke" tæller hverken i døgnet eller i snittet. Tallene i rækken er stadig alle angreb.
    let (angreb_status, gammel) = if vurdering.ikke_brugt.is_empty() {
        status(g, a.i_gaar, snit)
    } else {
        let relevante: Vec<Raekke> = rows.iter().filter(|r| !teknologi::udeladt(r, profil)).cloned().collect();
        let b = angreb(&relevante, s);
        status(g, b.i_gaar, b.snit())
    };
    let server = aktuel.then(|| server_fejl(rows, &dag(1))).flatten();
    let trafik = aktuel
        .then(|| trafik_spring(rows, nu).into_iter().find(|t| t.dag == dag(1)))
        .flatten();
    let laek = aktuel.then(|| laek_i(rows, &dag(1))).flatten();
    let dmarc = aktuel.then(|| dmarc::fund(rows, &dag(1))).flatten();
    let sti200 = aktuel.then(|| crate::sti200::fund(rows, &dag(1))).flatten();
    // Hold øje, men aldrig mindre, end angrebene alene gav. Noget, der nåede serveren, og en følsom fil, der ligner filen, er altid Handl.
    let status = if vurdering.naaet.is_some() || laek.as_ref().is_some_and(|l| l.naaet > 0) || sti200.is_some() {
        Status::Roed
    } else if angreb_status == Status::Groen
        && (server.is_some() || trafik.is_some() || vurdering.uventet.is_some() || laek.is_some() || dmarc.is_some())
    {
        Status::Gul
    } else {
        angreb_status
    };
    let vindue: Vec<String> = (0..=HENT_DAGE).map(|i| dag_noegle(nu - i * DOEGN_MS)).collect();
    let doegn_faerdige = vindue.iter().filter(|d| hentet.get(*d).is_some_and(|h| er_faerdig(d, h))).count();
    Raekke1 {
        id: zone.id.clone(),
        navn: zone.navn.clone(),
        status,
        gammel,
        opdateres: g == Grundlag::Forgaars,
        i_gaar_dag: dag_noegle(nu - DOEGN_MS),
        status_dag: dag(1),
        uge_fra: dag(7),
        ugen_foer_fra: dag(14),
        ugen_foer_til: dag(8),
        hele: hele_doegn(rows, nu),
        aendring: aendring(a.uge, a.uge_foer),
        angreb_i_gaar: a.i_gaar,
        angreb_snit: (snit * 10.0).round() / 10.0,
        angreb_uge: a.uge,
        angreb_ugen_foer: a.uge_foer,
        angreb_fordeling: a.fordeling,
        angreb_i_gaar_top: a.top,
        sidst_hentet: sidst_hentet(hentet, nu),
        doegn_faerdige,
        doegn_i_alt: vindue.len(),
        tendens: None,
        trafik,
        server,
        forstyrrelse: None,
        angreb_status,
        ikke_brugt: vurdering.ikke_brugt,
        uventet: vurdering.uventet,
        naaet: vurdering.naaet,
        tek_besked: vurdering.besked,
        tek_spoerg: vurdering.spoerg,
        med7: med_data(hentet, nu, 7),
        med30: med_data(hentet, nu, HENT_DAGE),
        laek,
        dmarc,
        tilstand: vec![],
        sti200,
        cf_fund: None,
        moenster: None,
        afgjort: None,
        efterslaeb: vec![],
        vurderet: None,
        forgaars: false,
        kan_sammenligne: true,
    }
}

/// Rød, gul, grøn, grå; derefter navn.
pub fn sorter(rk: &mut [Raekke1]) {
    rk.sort_by(|a, b| a.status.cmp(&b.status).then_with(|| a.navn.cmp(&b.navn)));
}

/// Testdata til `dyk`: `hentet` for en zone, der står på i går, eller om natten på døgnet før.
#[cfg(test)]
pub fn status_dag_test(nu: i64, nat: bool) -> HashMap<String, String> {
    let i = 1 + i64::from(nat);
    let dag = dag_noegle(nu - i * DOEGN_MS);
    let hentet = crate::vaern::iso(dag_ms(&dag).unwrap() + DOEGN_MS + 2 * 3_600_000);
    let hentet = if hentet.as_str() > crate::vaern::iso(nu).as_str() {
        crate::vaern::iso(nu)
    } else {
        hentet
    };
    HashMap::from([(dag, hentet)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaern::iso;

    /// 2026-11-03T08:08:00Z.
    const NU: i64 = 1_793_693_280_000;

    fn d(i: i64) -> String {
        dag_noegle(NU - i * DOEGN_MS)
    }

    fn kl(t: &str) -> i64 {
        parse_iso(t).unwrap()
    }

    fn zone(navn: &str) -> Zone {
        Zone {
            id: "z".into(),
            navn: navn.into(),
        }
    }

    /// En `fw`-række som `doegn_raekker` laver den (8 felter, regel = felt 0, kilde = felt 6, handling = felt 7).
    fn fw_regel(dag: &str, regel: &str, kilde: &str, handling: &str, n: i64) -> Raekke {
        Raekke {
            dag: dag.into(),
            art: "fw".into(),
            noegle: [regel, "US", "AS", "/", "CPH", "GET", kilde, handling].join("\t"),
            n,
        }
    }

    fn fw(dag: &str, kilde: &str, handling: &str, n: i64) -> Raekke {
        fw_regel(dag, "Regel", kilde, handling, n)
    }

    /// Alle døgn i hentevinduet hentet færdigt for en time siden.
    fn hentet_alt() -> HashMap<String, String> {
        (0..=HENT_DAGE).map(|i| (d(i), iso(NU - 3_600_000))).collect()
    }

    /// Alle døgn i hentevinduet set fra `nu`, hentet kl. `t`.
    fn hentet_kl(nu: i64, t: &str) -> HashMap<String, String> {
        (0..=HENT_DAGE).map(|i| (dag_noegle(nu - i * DOEGN_MS), iso(kl(t)))).collect()
    }

    fn typer(a: &[AngrebType]) -> Vec<(&str, i64)> {
        a.iter().map(|t| (t.navn.as_str(), t.n)).collect()
    }

    fn alle(i: i64, n: i64) -> Raekke {
        Raekke {
            dag: d(i),
            art: "alle".into(),
            noegle: String::new(),
            n,
        }
    }

    fn svarkode(i: i64, kode: &str, n: i64) -> Raekke {
        Raekke {
            dag: d(i),
            art: "status".into(),
            noegle: kode.into(),
            n,
        }
    }

    // ── Mærkerne ──
    fn net(i: i64, netvaerk: &str, n: i64) -> Raekke {
        Raekke {
            dag: d(i),
            art: "fw".into(),
            noegle: ["Regel", "US", netvaerk, "/", "CPH", "GET", "firewallManaged", "block"].join("\t"),
            n,
        }
    }

    fn sti(i: i64, s: &str) -> Raekke {
        Raekke {
            dag: d(i),
            art: "probe".into(),
            noegle: s.into(),
            n: 1,
        }
    }

    fn stier(i: i64, foran: &str, n: i64) -> Vec<Raekke> {
        (0..n).map(|k| sti(i, &format!("/{foran}{k}"))).collect()
    }

    fn sig(rows: &[Raekke]) -> Signal {
        signal(rows, &hentet_alt())
    }

    /// Et valgt domæne, hvor intet sker.
    fn stille() -> Signal {
        sig(&[net(1, "Stille AS", 1)])
    }

    fn moenstre_for(rows: &[Raekke], p: &Profil, andre: &[&Signal], r_ref: Option<f64>) -> Moenstre {
        moenstre(
            &raekke_med(&zone("a.wi24rd.com"), &hentet_alt(), rows, NU, p),
            rows,
            p,
            &sig(rows),
            andre,
            &|_| r_ref,
            None,
        )
    }

    /// Mærket med to stille domæner mere, så sammenligningen altid har nok.
    fn maerke(rows: &[Raekke], p: &Profil, andre: &[&Signal], r_ref: Option<f64>) -> Option<Moenster> {
        let fyld = [stille(), stille()];
        let alle: Vec<&Signal> = andre.iter().copied().chain(&fyld).collect();
        moenstre_for(rows, p, &alle, r_ref).vis
    }

    /// En gul dag: 40 angreb mod 0 før, heraf `top` fra ét netværk.
    fn gul_dag(top: i64) -> Vec<Raekke> {
        vec![net(1, "Mal AS", top), net(1, "Andet AS", 40 - top)]
    }

    #[test]
    fn maalrettet_er_stigning_et_netvaerk_og_ingen_boelge() {
        let (ro, p) = (stille(), Profil::new());
        let m = maerke(&gul_dag(30), &p, &[&ro], Some(1.0)).unwrap();
        assert_eq!(
            (m.slags, m.netvaerk.as_deref(), m.procent, m.verden, m.ny),
            (MoensterSlags::Maalrettet, Some("Mal AS"), 75, true, true)
        );
        assert_eq!(maerke(&gul_dag(24), &p, &[&ro], None).unwrap().procent, 60, "præcis 60 %");
        assert!(
            !maerke(&gul_dag(24), &p, &[&ro], None).unwrap().verden,
            "uden Radar: kun de andre domæner"
        );
        assert_eq!(maerke(&gul_dag(23), &p, &[&ro], None), None, "57 %");
        assert_eq!(maerke(&gul_dag(30), &p, &[&ro], Some(1.3)), None, "verden steg 1,3");
        assert!(maerke(&gul_dag(30), &p, &[&ro], Some(1.29)).is_some());
        assert_eq!(maerke(&[net(1, "Mal AS", 9)], &p, &[&ro], None), None, "ingen stigning");
        assert_eq!(maerke(&[net(1, "", 40)], &p, &[&ro], None), None, "ukendt netværk");
    }

    #[test]
    fn maalrettet_falder_bort_ved_en_boelge() {
        let (ro, p) = (stille(), Profil::new());
        for i in [0, 1, 2] {
            let ogsaa = sig(&[net(i, "Mal AS", 4)]);
            assert_eq!(
                maerke(&gul_dag(30), &p, &[&ro, &ogsaa], None),
                None,
                "netværket på et andet domæne, dag {i}"
            );
        }
        let lidt = sig(&[net(0, "Mal AS", 3)]);
        assert!(
            maerke(&gul_dag(30), &p, &[&ro, &lidt], None).is_some(),
            "10 % af det, netværket gjorde her, er ikke en bølge"
        );
        let delt = (sig(&[net(1, "Mal AS", 2)]), sig(&[net(2, "Mal AS", 2)]));
        assert_eq!(maerke(&gul_dag(30), &p, &[&delt.0, &delt.1], None), None, "domænerne lægges sammen");
        let fjern = sig(&[net(3, "Mal AS", 1)]);
        assert!(maerke(&gul_dag(30), &p, &[&fjern], None).is_some(), "to dage væk tæller ikke");
        let gul = |i: i64| sig(&[net(i, "Andre AS", 40)]);
        let (a, b) = (gul(1), gul(2));
        assert!(maerke(&gul_dag(30), &p, &[&a], None).is_some(), "højst ét andet domæne gul");
        assert_eq!(maerke(&gul_dag(30), &p, &[&a, &b], None), None, "to andre domæner gul");
    }

    /// Beslutning 2: mindst 3 valgte domæner med tal, det selv medregnet. Rekognoscering kræver det ikke.
    #[test]
    fn maalrettet_kraever_tre_valgte_domaener_med_tal() {
        let (ro, p) = (stille(), Profil::new());
        let (a, b) = (stille(), stille());
        assert!(moenstre_for(&gul_dag(30), &p, &[&a, &b], None).vis.is_some(), "præcis 3");
        assert_eq!(moenstre_for(&gul_dag(30), &p, &[&a], None).vis, None, "2 domæner, også uden Radar");
        assert_eq!(moenstre_for(&gul_dag(30), &p, &[&a], Some(1.0)).vis, None, "Radar er ikke nok");
        assert_eq!(moenstre_for(&gul_dag(30), &p, &[], Some(1.0)).vis, None);
        let uden_tal = sig(&[net(5, "Stille AS", 1)]);
        assert_eq!(
            moenstre_for(&gul_dag(30), &p, &[&ro, &uden_tal], None).vis,
            None,
            "et domæne uden tal for dagen tæller ikke"
        );
        assert!(
            moenstre_for(&stier(1, "a", 100), &p, &[&ro], None).vis.is_some(),
            "rekognoscering som før"
        );
        assert!(!kan_sammenligne(&[&ro], &d(1)) && kan_sammenligne(&[&a, &b], &d(1)));
    }

    /// Boksen bruger de tal, mærket afgøres af. Hver betingelse fejler for sig, og ✓ står kun ved den, der er opfyldt.
    #[test]
    fn vurderingen_siger_det_samme_som_maerket() {
        let (ro, ro2, p) = (stille(), stille(), Profil::new());
        let egen = sig(&gul_dag(30));
        let v = vurdering(&d(1), &gul_dag(30), &p, &egen, &[&ro, &ro2], Some(1.0));
        let ok = |v: &Vurdering, id: &str| v.maal.iter().chain(&v.rekog).find(|b| b.id == id).unwrap().ok;
        let tal = |v: &Vurdering, id: &str| {
            let b = v.maal.iter().find(|b| b.id == id).unwrap().clone();
            (b.n, b.m)
        };
        assert!(v.maal_ja && v.foreloebig, "alt er opfyldt, men døgnet efter mangler");
        assert_eq!(
            (ok(&v, "stigning"), ok(&v, "efter"), ok(&v, "radar")),
            (Some(true), Some(false), Some(true))
        );
        assert_eq!(tal(&v, "stigning").0, 40.0);
        assert_eq!(tal(&v, "netvaerk"), (75.0, 60.0));
        assert_eq!(v.maal.iter().find(|b| b.id == "netvaerk").unwrap().navn.as_deref(), Some("Mal AS"));
        assert_eq!(tal(&v, "valgte"), (3.0, 3.0));
        let ingen_radar = vurdering(&d(1), &gul_dag(30), &p, &egen, &[&ro, &ro2], None);
        assert_eq!(ok(&ingen_radar, "radar"), None, "kan ikke vurderes og tæller ikke imod");
        assert!(ingen_radar.maal_ja);
        let mut z = sig(&[net(0, "Mal AS", 4)]);
        (z.zone, z.navn) = ("z2".into(), "b.wi24rd.com".into());
        let v = vurdering(&d(1), &gul_dag(30), &p, &egen, &[&ro, &z], None);
        assert!(!v.maal_ja && !v.foreloebig);
        assert_eq!(ok(&v, "net_andre"), Some(false));
        let b = v.maal.iter().find(|b| b.id == "net_andre").unwrap();
        assert_eq!(
            (b.n, b.m, b.domaener[0].navn.as_str(), b.domaener[0].n),
            (4.0, 30.0, "b.wi24rd.com", 4)
        );
        let v = vurdering(&d(1), &gul_dag(30), &p, &egen, &[&ro], Some(1.5));
        assert_eq!((ok(&v, "valgte"), ok(&v, "radar"), v.maal_ja), (Some(false), Some(false), false));
        let spredt = vec![net(1, "A", 20), net(1, "B", 20)];
        let v = vurdering(&d(1), &spredt, &p, &sig(&spredt), &[&ro, &ro2], None);
        assert_eq!((ok(&v, "netvaerk"), tal(&v, "netvaerk").0, v.maal_ja), (Some(false), 50.0, false));
        let rolig = vec![net(1, "A", 1)];
        let v = vurdering(&d(1), &rolig, &p, &sig(&rolig), &[&ro, &ro2], None);
        assert_eq!((ok(&v, "stigning"), v.maal_ja), (Some(false), false));
        let scan = stier(1, "a", 200);
        let v = vurdering(&d(1), &scan, &p, &sig(&scan), &[&ro], None);
        assert!(v.rekog_ja);
        assert_eq!((ok(&v, "stier"), ok(&v, "faktor")), (Some(true), Some(true)));
        let v = vurdering(&d(1), &stier(1, "a", 20), &p, &sig(&stier(1, "a", 20)), &[&ro], None);
        assert_eq!((ok(&v, "stier"), v.rekog_ja), (Some(false), false));
    }

    fn laek(i: i64, sti: &str, resultat: &str, origin: i64, n: i64) -> Raekke {
        Raekke {
            dag: d(i),
            art: "laek".into(),
            noegle: format!("{sti}\t{resultat}\t{origin}"),
            n,
        }
    }

    /// `none` er aldrig et læk, hverken uden sti eller på en gammel række med sti og origin. Status er uændret.
    #[test]
    fn none_taeller_aldrig_som_laek() {
        let med = |rows: Vec<Raekke>| raekke(&zone("a.wi24rd.com"), &hentet_alt(), &rows, NU);
        let mut uden_sti = laek(1, "", "none", 0, 1234);
        uden_sti.noegle = "\tnone\t".into();
        let r = med(vec![uden_sti, laek(1, "/", "none", 200, 40), laek(1, "/login", "ukendt", 200, 3)]);
        assert_eq!((r.status, r.laek), (Status::Groen, None));
    }

    fn dns_doegn(i: i64, nx: i64, navne: i64) -> Vec<Raekke> {
        let rk = |art: &str, noegle: &str, n: i64| Raekke {
            dag: d(i),
            art: art.into(),
            noegle: noegle.into(),
            n,
        };
        vec![rk("dns_kode", "NXDOMAIN", nx), rk("dns_navne", "", navne)]
    }

    /// Mærket står på de samme tal som boksen "Vurdering", og kun første døgn i en episode er nyt.
    #[test]
    fn dns_maerket_og_vurderingen_siger_det_samme() {
        let ro: Vec<Raekke> = (2..=8).flat_map(|i| dns_doegn(i, 5, 2)).collect();
        let hop = [ro.clone(), dns_doegn(1, 900, 300)].concat();
        let m = dns_rekognoscering(&d(1), &hop).unwrap();
        assert_eq!(
            (m.slags, m.stier, m.dns_navne, m.ny),
            (MoensterSlags::DnsRekognoscering, 300, true, true)
        );
        assert!(m.slags.er_rekognoscering() && !MoensterSlags::Maalrettet.er_rekognoscering());
        let (p, s) = (Profil::new(), Signal::default());
        let v = vurdering(&d(1), &hop, &p, &s, &[], None);
        assert!(v.dns_ja && v.dns.iter().all(|b| b.ok == Some(true)), "{:?}", v.dns);
        let stille = [ro.clone(), dns_doegn(1, 5, 2)].concat();
        assert_eq!(dns_rekognoscering(&d(1), &stille), None);
        let v = vurdering(&d(1), &stille, &p, &s, &[], None);
        assert!(!v.dns_ja && v.dns.iter().all(|b| b.ok == Some(false)));
        let to_dage = [ro, dns_doegn(2, 900, 300), dns_doegn(1, 9000, 3000)].concat();
        assert!(!dns_rekognoscering(&d(1), &to_dage).unwrap().ny, "dagen før slog også til");
    }

    /// Mails, der fejler DMARC fra en ny afsender, giver Hold øje, aldrig Handl, og status er uændret uden.
    #[test]
    fn dmarc_giver_hold_oeje() {
        let rk = |i: i64, org: &str, res: &str, n: i64| Raekke {
            dag: d(i),
            art: "dmarc".into(),
            noegle: format!("{org}\tDK\t{res}\t{res}"),
            n,
        };
        let med = |rows: Vec<Raekke>| raekke(&zone("a.wi24rd.com"), &hentet_alt(), &rows, NU);
        let kendte: Vec<Raekke> = (5..=12).map(|i| rk(i, "Kendt", "pass", 40)).collect();
        assert_eq!(med(kendte.clone()).status, Status::Groen);
        let r = med([kendte, vec![rk(1, "Ny", "fail", 2)]].concat());
        assert_eq!(r.status, Status::Gul);
        assert_eq!(r.dmarc.unwrap().nye, ["Ny"]);
    }

    /// Kun kodeord fra et kendt læk tæller. Hold øje ved ét forsøg, Handl, når serveren svarede 2xx eller 302.
    #[test]
    fn laekkede_kodeord_giver_hold_oeje_og_handl() {
        let med = |rows: Vec<Raekke>| raekke(&zone("a.wi24rd.com"), &hentet_alt(), &rows, NU);
        let r = med(vec![
            laek(1, "/wp-login.php", "password_leaked", 403, 2),
            laek(1, "/wp-login.php", "username_leaked", 200, 9),
        ]);
        assert_eq!((r.status, r.laek.as_ref().map(|l| (l.n, l.naaet))), (Status::Gul, Some((2, 0))));
        for origin in [200, 204, 302] {
            let r = med(vec![laek(1, "/login", "username_and_password_leaked", origin, 1)]);
            assert_eq!((r.status, r.laek.unwrap().naaet), (Status::Roed, 1), "{origin}");
        }
        assert_eq!(med(vec![laek(1, "/login", "password_leaked", 401, 1)]).status, Status::Gul);
        assert_eq!(
            med(vec![laek(1, "/login", "ukendt", 200, 5)]).laek,
            None,
            "ukendte værdier tæller ikke"
        );
        assert_eq!(
            med(vec![laek(2, "/login", "password_leaked", 200, 5)]).laek,
            None,
            "kun statusdøgnet"
        );
    }

    /// En følsom sti med antal og snit (bytes) i døgn `i` før `NU`.
    fn sti200(i: i64, sti: &str, n: i64, snit: i64) -> [Raekke; 2] {
        let r = |art: &str, n: i64| Raekke {
            dag: d(i),
            art: art.into(),
            noegle: sti.into(),
            n,
        };
        [r(crate::sti200::ART, n), r(crate::sti200::ART_BYTES, n * snit)]
    }

    /// V-TOKEN.1: en lille fil, der ikke ligner forsiden, giver Handl. Det, der ligner sidens almindelige side (målt på 3 af 13 domæner), ændrer intet.
    #[test]
    fn foelsom_fil_med_svar_200_giver_handl_kun_naar_den_ligner_filen() {
        let med = |dele: Vec<[Raekke; 2]>| {
            let rows: Vec<Raekke> = dele.into_iter().flatten().collect();
            raekke(&zone("a.wi24rd.com"), &hentet_alt(), &rows, NU)
        };
        let r = med(vec![sti200(1, "/", 40, 30_000), sti200(1, "/.env", 2, 900)]);
        assert_eq!(r.status, Status::Roed);
        assert_eq!(
            r.sti200.as_ref().map(|f| (f.n, f.stier.clone())),
            Some((2, vec!["/.env".to_string()]))
        );
        let side = med(vec![sti200(1, "/", 40, 1_675), sti200(1, "/.env", 27, 1_647)]);
        assert_eq!((side.status, side.sti200), (Status::Groen, None), "som forsiden");
        let stor = med(vec![sti200(1, "/", 40, 10_015), sti200(1, "/.git/HEAD", 2, 20_778)]);
        assert_eq!((stor.status, stor.sti200), (Status::Groen, None), "en side, ikke en fil");
        let gammel = med(vec![sti200(1, "/", 40, 30_000), sti200(2, "/.env", 2, 900)]);
        assert_eq!(gammel.sti200, None, "kun statusdøgnet");
        // Mutation: uden reglen ville den troværdige træffer ikke give Handl.
        assert_ne!(r.angreb_status, Status::Roed, "det er filen, ikke angrebene, der giver Handl");
    }

    #[test]
    fn cloudflares_kritiske_fund_giver_hold_oeje_aldrig_handl_og_aldrig_lavere() {
        let f = |alvor: &str| crate::sikkerhed::Fund {
            zone: "a".into(),
            id: "a-x".into(),
            alvor: alvor.into(),
            klasse: "always_use_https_not_enabled".into(),
            type_: "insecure_configuration".into(),
            siden: "2026-04-24".into(),
        };
        let groen = raekke(&zone("a.wi24rd.com"), &hentet_alt(), &[], NU);
        assert_eq!(groen.status, Status::Groen);
        let mut r = groen.clone();
        med_fund(&mut r, &[f("Critical")]);
        assert_eq!((r.status, r.cf_fund.as_ref().map(|c| c.n)), (Status::Gul, Some(1)));
        let mut r = groen.clone();
        med_fund(&mut r, &[f("Low"), f("Moderate")]);
        assert_eq!((r.status, r.cf_fund), (Status::Groen, None), "Low og Moderate giver intet");
        let mut r = Raekke1 {
            status: Status::Roed,
            ..groen.clone()
        };
        med_fund(&mut r, &[f("Critical")]);
        assert_eq!(r.status, Status::Roed, "aldrig lavere end før");
        let mut r = Raekke1 {
            status: Status::Graa,
            ..groen
        };
        med_fund(&mut r, &[f("Critical")]);
        assert_eq!(r.status, Status::Graa, "uden tal er der intet at sige");
    }

    fn stand(status: &str, paused: bool, dev_sek: i64, phishing: bool) -> crate::cf::ZoneStand {
        crate::cf::ZoneStand {
            status: status.into(),
            paused,
            dev_sek,
            phishing,
        }
    }

    /// Hver tilstand giver sin slags og sin status. Udviklertilstanden løber ud af sig selv.
    #[test]
    fn zonens_tilstand_giver_rigtig_status() {
        let kl = |_: i64| "14.30".to_string();
        let fund = |z: &crate::cf::ZoneStand, nu: i64| tilstand_fund(z, NU, nu, &kl);
        let slags = |f: &[TilstandFund]| f.iter().map(|x| x.slags).collect::<Vec<_>>();
        assert!(fund(&stand("active", false, 0, false), NU).is_empty());
        assert!(
            fund(&stand("active", false, -500, false), NU).is_empty(),
            "negativt: den sluttede for længst"
        );
        assert_eq!(slags(&fund(&stand("active", true, 0, false), NU)), ["pause"]);
        let f = fund(&stand("pending", false, 0, false), NU);
        assert_eq!((slags(&f), f[0].status.as_deref()), (vec!["inaktiv"], Some("pending")));
        assert_eq!(slags(&fund(&stand("active", false, 0, true), NU)), ["phishing"]);
        let f = fund(&stand("active", false, 3600, false), NU);
        assert_eq!((slags(&f), f[0].kl.as_deref()), (vec!["udvikler"], Some("14.30")));
        assert!(
            fund(&stand("active", false, 3600, false), NU + 3_600_001).is_empty(),
            "tiden er gået"
        );
        let groen = || raekke(&zone("a.wi24rd.com"), &hentet_alt(), &[], NU);
        for z in [
            stand("active", true, 0, false),
            stand("moved", false, 0, false),
            stand("active", false, 0, true),
        ] {
            let mut r = groen();
            med_tilstand(&mut r, fund(&z, NU));
            assert_eq!(r.status, Status::Roed, "{z:?}");
        }
        let mut r = groen();
        med_tilstand(&mut r, fund(&stand("active", false, 60, false), NU));
        assert_eq!(r.status, Status::Gul);
        let mut r = Raekke1 {
            status: Status::Roed,
            ..groen()
        };
        med_tilstand(&mut r, fund(&stand("active", false, 60, false), NU));
        assert_eq!(r.status, Status::Roed, "aldrig lavere end før");
    }

    /// Et mærke på et døgn, programmet ikke nåede, er ikke tabt. Højst `MAERKE_BAGUD` døgn tilbage, og første gang ser det kun til døgnet før statusdøgnet.
    #[test]
    fn afgjorte_maerker_fra_slukkede_doegn_kommer_med() {
        let p = Profil::new();
        let dage = || sig(&(1..=12).map(|i| net(i, "Stille AS", 1)).collect::<Vec<_>>());
        let (a, b) = (dage(), dage());
        let paa = |i: i64| -> Vec<Raekke> { gul_dag(30).into_iter().map(|r| Raekke { dag: d(i), ..r }).collect() };
        let kald = |rows: &[Raekke], egen: &Signal, v: Option<&str>| {
            moenstre(
                &raekke_med(&zone("a.wi24rd.com"), &hentet_alt(), rows, NU, &p),
                rows,
                &p,
                egen,
                &[&a, &b],
                &|_| None,
                v,
            )
        };
        let dage_i = |m: &Moenstre| m.efterslaeb.iter().map(|x| x.dag.clone()).collect::<Vec<_>>();
        let rows = paa(4);
        let m = kald(&rows, &sig(&rows), Some(&d(6)));
        assert_eq!(dage_i(&m), [d(4)], "døgnet efter det vurderede");
        assert!(!m.efterslaeb[0].foreloebig && m.afgjort.is_none());
        assert_eq!(m.vurderet, Some(d(2)), "statusdøgnet venter på døgnet efter");
        assert!(dage_i(&kald(&rows, &sig(&rows), Some(&d(4)))).is_empty(), "vurderet er vurderet");
        let foerste = kald(&rows, &sig(&rows), None);
        assert!(dage_i(&foerste).is_empty(), "en opdatering melder ikke gammelt nyt");
        assert_eq!(foerste.vurderet, Some(d(2)));
        let rows = paa(8);
        assert_eq!(dage_i(&kald(&rows, &sig(&rows), Some(&d(20)))), [d(8)], "syv døgn før statusdøgnet");
        let rows = paa(9);
        assert!(dage_i(&kald(&rows, &sig(&rows), Some(&d(20)))).is_empty(), "ikke otte");
        let rows: Vec<Raekke> = [paa(3), paa(5)].concat();
        let mut hul = hentet_alt();
        hul.remove(&d(4));
        let m = kald(&rows, &signal(&rows, &hul), Some(&d(6)));
        assert_eq!(dage_i(&m), [d(3)], "et senere døgn venter ikke på det, der mangler");
        assert_eq!(m.vurderet, None, "og det ventende døgn ses igen næste gang");
    }

    /// Beslutning 1: statusdøgnet er foreløbigt, og døgnet før er afgjort, når statusdøgnet er hentet færdigt.
    #[test]
    fn maalrettet_er_foreloebig_til_doegnet_efter_er_hentet() {
        let p = Profil::new();
        let to_dage = || sig(&[net(1, "Stille AS", 1), net(2, "Stille AS", 1)]);
        let (a, b) = (to_dage(), to_dage());
        let m = moenstre_for(&gul_dag(30), &p, &[&a, &b], None);
        assert!(m.vis.as_ref().unwrap().foreloebig, "i dag er ikke færdig");
        assert_eq!(m.afgjort, None, "ingen besked endnu");
        let i_gaar: Vec<Raekke> = gul_dag(30).into_iter().map(|r| Raekke { dag: d(2), ..r }).collect();
        let m = moenstre_for(&i_gaar, &p, &[&a, &b], None);
        let afgjort = m.afgjort.unwrap();
        assert_eq!(
            (afgjort.dag.as_str(), afgjort.foreloebig, afgjort.angreb, afgjort.stoppet),
            (d(2).as_str(), false, 40, 40)
        );
        assert!(!m.vis.unwrap().foreloebig, "mærket står videre, nu afgjort");
        let ogsaa = sig(&[net(1, "Mal AS", 30), net(2, "Stille AS", 1)]);
        let m = moenstre_for(&i_gaar, &p, &[&a, &ogsaa], None);
        assert_eq!(
            (m.vis, m.afgjort),
            (None, None),
            "netværket dukkede op på et andet domæne dagen efter: mærket er væk, og der kommer ingen besked"
        );
        let bagud = signal(
            &[net(2, "Stille AS", 1)],
            &HashMap::from([(d(1), iso(dag_ms(&d(1)).unwrap() + 3_600_000))]),
        );
        let m = moenstre_for(&i_gaar, &p, &[&a, &bagud], None);
        assert_eq!(
            (m.vis, m.afgjort),
            (None, None),
            "et andet domæne mangler døgnet efter: ikke afgjort endnu"
        );
    }

    #[test]
    fn forgaars_foelger_den_lokale_dag() {
        assert!(!er_forgaars("2026-11-02", "2026-11-03"));
        assert!(er_forgaars("2026-11-02", "2026-11-04"));
        assert!(!er_forgaars("2026-11-02", "2026-11-02"), "vest for UTC");
    }

    /// Uden noget normalt er betingelsen uden indhold i visningen, men mærket afgøres som før.
    #[test]
    fn normalt_nul_er_ingen_sammenligning_og_aendrer_ikke_maerket() {
        let (ro, p) = (stille(), Profil::new());
        let faktor = |v: &Vurdering| v.rekog.iter().find(|b| b.id == "faktor").unwrap().clone();
        let scan = stier(1, "a", 100);
        let v = vurdering(&d(1), &scan, &p, &sig(&scan), &[&ro], None);
        let b = faktor(&v);
        assert_eq!((b.uden_normal, b.ok, b.m, v.rekog_ja), (true, Some(true), 0.0, true));
        assert!(maerke(&scan, &p, &[&ro], None).is_some(), "mærket står uændret");
        let tomt: Vec<Raekke> = Vec::new();
        let v = vurdering(&d(1), &tomt, &p, &sig(&tomt), &[&ro], None);
        assert_eq!((faktor(&v).uden_normal, faktor(&v).ok, v.rekog_ja), (true, Some(true), false));
        let med: Vec<Raekke> = (2..=8).flat_map(|i| stier(i, "b", 40)).chain(stier(1, "a", 120)).collect();
        let v = vurdering(&d(1), &med, &p, &sig(&med), &[&ro], None);
        assert_eq!((faktor(&v).uden_normal, faktor(&v).m), (false, 120.0));
    }

    #[test]
    fn rekognoscering_taersklerne_og_episoder() {
        let (ro, p) = (stille(), Profil::new());
        let m = maerke(&stier(1, "a", 100), &p, &[&ro], None).unwrap();
        assert_eq!((m.slags, m.stier, m.normalt, m.ny), (MoensterSlags::Rekognoscering, 100, 0.0, true));
        assert_eq!(maerke(&stier(1, "a", 99), &p, &[&ro], None), None, "99 stier");
        let basis: Vec<Raekke> = (2..=8).flat_map(|i| stier(i, "b", 40)).collect();
        let med = |n: i64| [basis.clone(), stier(1, "a", n)].concat();
        assert_eq!(maerke(&med(120), &p, &[&ro], None).unwrap().normalt, 40.0, "præcis 3 × medianen");
        assert_eq!(maerke(&med(119), &p, &[&ro], None), None, "under 3 × medianen");
        let igen = |foer: i64| [stier(foer, "c", 100), stier(1, "a", 100)].concat();
        assert!(!maerke(&igen(8), &p, &[&ro], None).unwrap().ny, "6 stille døgn: samme episode");
        assert!(maerke(&igen(9), &p, &[&ro], None).unwrap().ny, "7 stille døgn: ny episode");
    }

    #[test]
    fn scanner_er_stier_der_ogsaa_ses_andre_steder() {
        let p = Profil::new();
        let rows = stier(1, "a", 100);
        let m = |andre: &[&Signal]| maerke(&rows, &p, andre, None).unwrap().scanner;
        assert!(!m(&[]), "uden andre domæner kan det ikke afgøres");
        let (ens70, ens71) = (sig(&stier(3, "a", 70)), sig(&stier(3, "a", 71)));
        assert!(!m(&[&ens70]), "30 % kun her");
        assert!(m(&[&ens71]), "29 % kun her");
        let gammel = sig(&stier(9, "a", 100));
        assert!(!m(&[&gammel]), "for længe siden");
    }

    /// De tre rækker fra baglæns-kørslen: en bølge, en scanner og et stille domæne.
    #[test]
    fn baglaens_faste_data() {
        let p = Profil::new();
        let boelge = gul_dag(30);
        let [a, b] = [sig(&gul_dag(30)), sig(&gul_dag(30))];
        assert_eq!(maerke(&boelge, &p, &[&a, &b], None), None, "bølge");
        let scanner = stier(1, "a", 150);
        let c = sig(&stier(2, "a", 150));
        assert!(maerke(&scanner, &p, &[&c], None).unwrap().scanner, "scanner");
        assert_eq!(maerke(&[net(1, "Stille AS", 3)], &p, &[&c], None), None, "stille");
    }

    #[test]
    fn maerket_aendrer_aldrig_status_og_kraever_friske_tal() {
        let rows = stier(1, "a", 200);
        let ro = stille();
        let m = |r: &Raekke1| moenstre(r, &rows, &Profil::new(), &sig(&rows), &[&ro], &|_| None, None).vis;
        let r = raekke(&zone("a.wi24rd.com"), &hentet_alt(), &rows, NU);
        assert_eq!(r.status, Status::Groen);
        assert!(m(&r).is_some(), "mærket står på en grøn række");
        let gamle = hentet_kl(NU, "2026-10-20T00:00:00Z");
        assert_eq!(m(&raekke(&zone("a.wi24rd.com"), &gamle, &rows, NU)), None);
    }

    #[test]
    fn teknologi_valgene_styrer_signalerne() {
        let ro = stille();
        // "Bruger vi ikke": WordPress-angreb og -stier tæller ikke.
        let angreb = [fw_regel(&d(1), WP, "firewallManaged", "block", 40)];
        let wp: Vec<Raekke> = angreb
            .iter()
            .map(|r| Raekke {
                noegle: r.noegle.replace("\tAS\t", "\tMal AS\t"),
                ..r.clone()
            })
            .collect();
        assert!(maerke(&wp, &Profil::new(), &[&ro], None).is_some());
        let ikke = tek("wordpress", teknologi::Valg::BrugerIkke);
        assert_eq!(maerke(&wp, &ikke, &[&ro], None), None, "angreb");
        let stier_wp: Vec<Raekke> = (0..100).map(|k| sti(1, &format!("/wp-content/x{k}.zip"))).collect();
        assert_eq!(teknologi::af_sti("/wp-content/x1.zip"), Some("wordpress"));
        assert!(maerke(&stier_wp, &Profil::new(), &[&ro], None).is_some());
        assert_eq!(maerke(&stier_wp, &ikke, &[&ro], None), None, "stier");
        // "Bruger vi": stierne tæller dobbelt og nævnes.
        let bruger = tek("wordpress", teknologi::Valg::Bruger);
        let halvt: Vec<Raekke> = (0..50).map(|k| sti(1, &format!("/wp-content/x{k}.zip"))).collect();
        assert_eq!(maerke(&halvt, &Profil::new(), &[&ro], None), None, "50 stier");
        let m = maerke(&halvt, &bruger, &[&ro], None).unwrap();
        assert_eq!(
            (m.stier, m.tek.as_deref()),
            (50, Some("wordpress")),
            "vægtet til 100, men tallet er stierne"
        );
        assert_eq!(maerke(&wp, &bruger, &[&ro], None).unwrap().tek.as_deref(), Some("wordpress"));
    }

    /// Status er stadig gul af de relevante angreb: "Bruger vi ikke" må heller ikke flytte netværket eller dets andel.
    #[test]
    fn bruger_vi_ikke_taeller_ikke_i_netvaerkets_andel() {
        let (ro, ikke) = (stille(), tek("wordpress", teknologi::Valg::BrugerIkke));
        let wp = |netvaerk: &str, n: i64| Raekke {
            dag: d(1),
            art: "fw".into(),
            noegle: [WP, "US", netvaerk, "/", "CPH", "GET", "firewallManaged", "block"].join("\t"),
            n,
        };
        let rows = [gul_dag(30), vec![wp("WP AS", 100)]].concat();
        assert_eq!(
            maerke(&rows, &Profil::new(), &[&ro], None).map(|m| (m.netvaerk, m.procent)),
            Some((Some("WP AS".to_string()), 71)),
            "uden valg er WordPress-netværket størst"
        );
        assert_eq!(
            maerke(&rows, &ikke, &[&ro], None).map(|m| (m.netvaerk, m.procent)),
            Some((Some("Mal AS".to_string()), 75)),
            "med valget tæller kun de relevante"
        );
        let spredt = [vec![net(1, "A AS", 20), net(1, "B AS", 20)], vec![wp("WP AS", 100)]].concat();
        assert!(maerke(&spredt, &Profil::new(), &[&ro], None).is_some(), "71 % fra ét netværk");
        assert_eq!(maerke(&spredt, &ikke, &[&ro], None), None, "de relevante er spredt: 50 %");
    }

    #[test]
    fn boelge_reglen_i_hvorfor() {
        let gul = |i: i64| sig(&[net(i, "AS", 40)]);
        let (a, b, ro) = (gul(1), gul(2), stille());
        let dag = d(1);
        let egen = Some(Tendens::SaerligtDig);
        assert_eq!(efter_boelgen(egen, &[&a, &ro], &dag), egen, "ét andet domæne");
        assert_eq!(efter_boelgen(egen, &[&a, &b], &dag), None, "to andre domæner ±1");
        assert_eq!(efter_boelgen(Some(Tendens::Foelger), &[&a, &b], &dag), Some(Tendens::Foelger));
        let fjern = gul(4);
        assert_eq!(efter_boelgen(egen, &[&a, &fjern], &dag), egen, "tre dage væk");
    }

    #[test]
    fn trafik_spring_taersklerne() {
        let doegn = |top: i64, ellers: i64| -> Vec<Raekke> { (1..=30).map(|i| alle(i, if i == 3 { top } else { ellers })).collect() };
        assert_eq!(
            trafik_spring(&doegn(2000, 200), NU),
            [TrafikDag {
                dag: d(3),
                n: 2000,
                median: 200.0,
                stoppet: 0
            }],
            "præcis 10 × medianen"
        );
        assert!(trafik_spring(&doegn(1999, 200), NU).is_empty(), "under 10 × medianen");
        assert_eq!(trafik_spring(&doegn(1000, 50), NU).len(), 1, "præcis 1.000");
        assert!(trafik_spring(&doegn(999, 50), NU).is_empty(), "under 1.000, selv om det er 20 ×");
        let mut rows = doegn(5000, 100);
        rows.push(alle(0, 900_000));
        rows.push(alle(31, 900_000));
        assert_eq!(trafik_spring(&rows, NU).len(), 1, "kun de 30 hele døgn: ikke i dag og ikke d-31");
        assert!(trafik_spring(&[], NU).is_empty());
        // Uden en normal at holde døgnet op imod kommer mærket ikke.
        let faa = |dage: i64| -> Vec<Raekke> {
            (1..=30)
                .map(|i| {
                    alle(
                        i,
                        if i == 3 {
                            5000
                        } else if i <= dage {
                            10
                        } else {
                            0
                        },
                    )
                })
                .collect()
        };
        assert_eq!(trafik_spring(&faa(7), NU).len(), 1, "præcis 7 dage med forespørgsler");
        assert!(trafik_spring(&faa(6), NU).is_empty(), "6 dage: medianen er 0, og det siger intet");
        let mut rows = doegn(5000, 100);
        rows.push(Raekke {
            dag: d(3),
            ..fw(&d(3), "firewallManaged", "block", 67)
        });
        rows.push(fw(&d(3), "linkMaze", "link_maze_injected", 900));
        rows.push(fw(&d(4), "firewallManaged", "block", 5));
        assert_eq!(trafik_spring(&rows, NU)[0].stoppet, 67, "kun det afviste den dag");
    }

    #[test]
    fn server_fejl_taersklerne() {
        let dag = d(1);
        let med = |fejl: i64, ok: i64| vec![svarkode(1, "522", fejl), svarkode(1, "200", ok)];
        let s = server_fejl(&med(10, 40), &dag).unwrap();
        assert_eq!((s.fejl, s.svar, s.procent), (10, 50, 20), "præcis 20 % af præcis 50");
        assert_eq!(server_fejl(&med(9, 41), &dag), None, "18 %");
        assert_eq!(server_fejl(&med(20, 80 + 1), &dag), None, "under 20 %");
        assert_eq!(server_fejl(&med(10, 39), &dag), None, "49 svar, selv om andelen er over 20 %");
        // V-DRIFT.1 B2 (D4): 530 er med; 528 og 529 er ikke.
        for kode in ["520", "521", "523", "524", "525", "526", "527", "530"] {
            assert!(server_fejl(&[svarkode(1, kode, 60)], &dag).is_some(), "{kode}");
        }
        for kode in ["519", "528", "529", "531", "500", "502", "404", "x"] {
            assert_eq!(server_fejl(&[svarkode(1, kode, 60)], &dag), None, "{kode}");
        }
        assert_eq!(server_fejl(&[svarkode(2, "522", 60)], &dag), None, "et andet døgn");
    }

    /// V-DRIFT.1 B2 (D4): 530 alene, 530 sammen med 52x, og 528/529 tæller ikke som fejl (men som svar). Tærsklerne er de samme.
    #[test]
    fn server_fejl_med_530() {
        let dag = d(1);
        let alene = server_fejl(&[svarkode(1, "530", 10), svarkode(1, "200", 40)], &dag).unwrap();
        assert_eq!(
            (alene.fejl, alene.svar, alene.procent),
            (10, 50, 20),
            "530 alene, præcis på begge tærskler"
        );
        assert_eq!(server_fejl(&[svarkode(1, "530", 9), svarkode(1, "200", 41)], &dag), None, "18 %");
        assert_eq!(
            server_fejl(&[svarkode(1, "530", 10), svarkode(1, "200", 39)], &dag),
            None,
            "49 svar"
        );
        // Hverken 522 eller 530 når 20 % alene; sammen gør de.
        let sammen = [svarkode(1, "522", 6), svarkode(1, "530", 6), svarkode(1, "200", 48)];
        assert_eq!(server_fejl(&sammen[..1], &dag), None);
        let s = server_fejl(&sammen, &dag).unwrap();
        assert_eq!((s.fejl, s.svar, s.procent), (12, 60, 20));
        // 528 og 529 er svar i nævneren, aldrig fejl i tælleren.
        let med = [svarkode(1, "522", 10), svarkode(1, "528", 20), svarkode(1, "529", 20)];
        let s = server_fejl(&med, &dag).unwrap();
        assert_eq!((s.fejl, s.svar, s.procent), (10, 50, 20));
        assert_eq!(server_fejl(&[svarkode(1, "528", 30), svarkode(1, "529", 30)], &dag), None);
        assert!(SERVER_KODER.contains(&530) && !SERVER_KODER.contains(&528) && !SERVER_KODER.contains(&529));
        assert_eq!(s.mellem, None, "spændet sættes kun af byg_overblik");
        // Status er den samme som for 52x: Hold øje, aldrig Handl.
        let z = zone("a.wi24rd.com");
        let ro: Vec<Raekke> = (2..=30).map(|i| alle(i, 100)).collect();
        let nede = [ro, vec![svarkode(1, "530", 30), svarkode(1, "200", 70)]].concat();
        assert_eq!(raekke(&z, &hentet_alt(), &nede, NU).status, Status::Gul);
    }

    /// Et trafikspring kan være et overbelastningsforsøg, Cloudflare lod passere: det giver Hold øje. 52x giver også Hold øje, og ingen af dem sænker rød.
    #[test]
    fn trafik_og_52x_giver_hold_oeje() {
        let z = zone("a.wi24rd.com");
        let ro: Vec<Raekke> = (2..=30).map(|i| alle(i, 100)).collect();
        let med = |n: i64| [ro.clone(), vec![alle(1, n)]].concat();
        let spring = raekke(&z, &hentet_alt(), &med(50_000), NU);
        assert_eq!((spring.status, spring.angreb_status), (Status::Gul, Status::Groen));
        assert_eq!(spring.trafik.as_ref().map(|t| t.n), Some(50_000));
        assert_eq!(raekke(&z, &hentet_alt(), &med(100), NU).status, Status::Groen);
        assert_eq!(raekke(&z, &hentet_alt(), &med(100), NU).trafik, None);
        let faa: Vec<Raekke> = (1..=5).map(|i| alle(i, if i == 1 { 50_000 } else { 100 })).collect();
        assert_eq!(raekke(&z, &hentet_alt(), &faa, NU).status, Status::Groen, "under 7 dage med data");

        let nede = [ro.clone(), vec![svarkode(1, "522", 30), svarkode(1, "200", 70)]].concat();
        let r = raekke(&z, &hentet_alt(), &nede, NU);
        assert_eq!(
            (r.status, r.gammel, r.server.as_ref().map(|s| s.procent)),
            (Status::Gul, false, Some(30))
        );

        let angreb: Vec<Raekke> = (2..=8)
            .map(|i| fw(&d(i), "firewallManaged", "block", 10))
            .chain([fw(&d(1), "firewallManaged", "block", 100)])
            .collect();
        let begge = [nede.clone(), angreb].concat();
        assert_eq!(raekke(&z, &hentet_alt(), &begge, NU).status, Status::Roed, "rød bliver rød");
        let med_spring = [begge.clone(), vec![alle(1, 50_000)]].concat();
        assert_eq!(
            raekke(&z, &hentet_alt(), &med_spring, NU).status,
            Status::Roed,
            "også med et spring"
        );

        let gamle = hentet_kl(NU, "2026-10-20T00:00:00Z");
        let r = raekke(&z, &gamle, &nede, NU);
        assert!(r.gammel && r.server.is_none(), "gamle tal siger intet om serveren");
        assert_eq!(raekke(&z, &HashMap::new(), &nede, NU).status, Status::Graa);
    }

    fn tek(tek: &str, valg: teknologi::Valg) -> Profil {
        Profil::from([(tek.to_string(), teknologi::TekValg { valg, besked: false })])
    }

    const WP: &str = "Wordpress - Broken Access Control, File Inclusion";

    /// "Bruger vi ikke" ændrer status, men ingen af tallene i rækken, og de tæller hverken i døgnet eller i snittet.
    #[test]
    fn bruger_vi_ikke_aendrer_status_men_ikke_tallene() {
        let z = zone("a.wi24rd.com");
        let mut rows: Vec<Raekke> = (2..=8).map(|i| fw_regel(&d(i), "Andet", "firewallManaged", "block", 1)).collect();
        rows.push(fw_regel(&d(1), WP, "firewallManaged", "block", 40));
        rows.push(fw_regel(&d(1), "Andet", "firewallManaged", "block", 1));
        let foer = raekke(&z, &hentet_alt(), &rows, NU);
        assert_eq!(foer.status, Status::Roed);
        let efter = raekke_med(&z, &hentet_alt(), &rows, NU, &tek("wordpress", teknologi::Valg::BrugerIkke));
        assert_eq!(efter.status, Status::Groen);
        assert_eq!(
            (
                efter.angreb_i_gaar,
                efter.angreb_snit,
                efter.angreb_uge,
                &efter.angreb_i_gaar_top,
                efter.hele.i_gaar
            ),
            (
                foer.angreb_i_gaar,
                foer.angreb_snit,
                foer.angreb_uge,
                &foer.angreb_i_gaar_top,
                foer.hele.i_gaar
            ),
            "tallene er de samme"
        );
        assert_eq!(
            efter.ikke_brugt,
            [TekN {
                tek: "wordpress".into(),
                n: 40
            }]
        );
        let ved_ikke = raekke_med(&z, &hentet_alt(), &rows, NU, &tek("wordpress", teknologi::Valg::VedIkke));
        assert_eq!(ved_ikke.status, Status::Roed, "standarden er som i dag");
        assert_eq!(ved_ikke.tek_spoerg.as_deref(), Some("wordpress"));
    }

    #[test]
    fn sikkerhedsnettet_giver_gul_og_naaet_serveren_handl() {
        let z = zone("a.wi24rd.com");
        let svar = Raekke {
            dag: d(1),
            art: "login".into(),
            noegle: ["/wp-login.php", "POST", "200", "200", "US", ""].join("\t"),
            n: 1,
        };
        let rows = vec![fw_regel(&d(1), WP, "firewallManaged", "block", 40), svar];
        let r = raekke_med(&z, &hentet_alt(), &rows, NU, &tek("wordpress", teknologi::Valg::BrugerIkke));
        assert_eq!(r.status, Status::Gul, "sikkerhedsnettet");
        assert_eq!(r.uventet.as_ref().map(|u| u.sti.as_str()), Some("/wp-login.php"));
        let bruger = tek("wordpress", teknologi::Valg::Bruger);
        assert_eq!(raekke_med(&z, &hentet_alt(), &rows[..1], NU, &bruger).naaet, None);
        let igennem = vec![fw_regel(&d(1), WP, "firewallManaged", "log", 1)];
        let r = raekke_med(&z, &hentet_alt(), &igennem, NU, &bruger);
        assert_eq!(
            (r.status, r.naaet.map(|x| x.n)),
            (Status::Roed, Some(1)),
            "et enkelt angreb, der nåede serveren, er Handl"
        );
        let gamle = hentet_kl(NU, "2026-10-20T00:00:00Z");
        let r = raekke_med(&z, &gamle, &igennem, NU, &bruger);
        assert!(r.gammel && r.naaet.is_none() && r.status == Status::Gul, "gamle tal siger intet");
    }

    #[test]
    fn med_data_taeller_hentede_doegn() {
        let mut h = hentet_alt();
        h.remove(&d(3));
        h.remove(&d(20));
        h.remove(&d(0));
        assert_eq!((med_data(&h, NU, 7), med_data(&h, NU, 30)), (6, 28));
        let r = raekke(&zone("a.wi24rd.com"), &h, &[], NU);
        assert_eq!((r.med7, r.med30), (6, 28));
    }

    #[test]
    fn statusregel_taersklerne() {
        assert_eq!(iso(NU), "2026-11-03T08:08:00.000Z");
        assert_eq!(status_for_angreb(0, 0.0), Status::Groen);
        assert_eq!(status_for_angreb(9, 0.0), Status::Groen, "gulvet: 0 → 9 er ikke gult");
        assert_eq!(status_for_angreb(10, 0.0), Status::Gul);
        assert_eq!(status_for_angreb(24, 0.0), Status::Gul);
        assert_eq!(status_for_angreb(25, 0.0), Status::Roed, "0 → 25 er rødt");
        // Ved store snit tæller faktoren: s = 100 → gul fra 150, rød fra 300.
        assert_eq!(status_for_angreb(149, 100.0), Status::Groen);
        assert_eq!(status_for_angreb(150, 100.0), Status::Gul);
        assert_eq!(status_for_angreb(299, 100.0), Status::Gul);
        assert_eq!(status_for_angreb(300, 100.0), Status::Roed);
        // xyz.wi24rd.com: 28 mod 23,1 → grøn.
        assert_eq!(status_for_angreb(28, 23.1), Status::Groen);
    }

    /// Eksempel: wi24rd.com i går 1.260 stoppet, heraf 1.200 linkMaze. Kun angreb tæller: gul, ikke rød.
    #[test]
    fn wi24rd_com_gul_ikke_roed_naar_labyrinten_er_rutine() {
        let mut rows = vec![
            fw(&d(1), "linkMaze", "link_maze_injected", 1200),
            fw(&d(1), "botFight", "managed_challenge", 30),
            fw(&d(1), "firewallManaged", "block", 28),
            fw(&d(1), "ratelimit", "block", 2),
            fw(&d(0), "firewallManaged", "block", 14),
        ];
        // d-8…d-2: 87 angreb i alt (snit 12,4) og masser af rutine.
        for (i, n) in [(2, 12), (3, 13), (4, 12), (5, 12), (6, 13), (7, 12), (8, 13)] {
            rows.push(fw(&d(i), "firewallManaged", "block", n));
            rows.push(fw(&d(i), "linkMaze", "link_maze_injected", 400));
        }
        rows.push(fw(&d(9), "firewallManaged", "block", 1000)); // uden for snittets vindue
        let h = hele_doegn(&rows, NU);
        let a = angreb(&rows, NU);
        assert_eq!(h.i_gaar, 60, "J9: de 1.200 sider med fælde-links blev leveret, ikke afvist");
        assert_eq!(a.i_gaar, 30);
        assert_eq!(a.foer, 87);
        assert!((a.snit() - 12.43).abs() < 0.01);
        assert_eq!(h.i_dag_hidtil, 14);
        assert_eq!(
            h.fordeling.i_gaar,
            Fordeling {
                blokeret: 30,
                udfordret: 30,
                labyrint: 0,
                andet: 0,
                faelder: 1200
            }
        );
        let r = raekke(&zone("wi24rd.com"), &hentet_alt(), &rows, NU);
        assert_eq!((r.status, r.gammel, r.opdateres), (Status::Gul, false, false));
        assert_eq!(r.angreb_snit, 12.4);
        // Til sammenligning: hvis rutine talte med, ville samme regel give rødt (1.260 mod snit ~412).
        let alle_foer: i64 = rows.iter().filter(|x| x.dag >= d(8) && x.dag <= d(2)).map(|x| x.n).sum();
        let alle_i_gaar: i64 = rows.iter().filter(|x| x.dag == d(1)).map(|x| x.n).sum();
        assert_eq!(status_for_angreb(alle_i_gaar, alle_foer as f64 / 7.0), Status::Roed);
    }

    #[test]
    fn rutine_alene_giver_aldrig_gul_eller_roed() {
        let rows: Vec<Raekke> = RUTINE.iter().map(|k| fw(&d(1), k, "managed_challenge", 50_000)).collect();
        let (h, a) = (hele_doegn(&rows, NU), angreb(&rows, NU));
        assert_eq!((h.i_gaar, a.i_gaar), (150_000, 0));
        assert!(a.top.is_empty());
        assert_eq!(status(grundlag(&hentet_alt(), NU), a.i_gaar, a.snit()), (Status::Groen, false));
    }

    #[test]
    fn graa_og_gamle_tal() {
        assert_eq!(grundlag(&HashMap::new(), NU), Grundlag::Intet);
        assert_eq!(status(Grundlag::Intet, 1000, 0.0), (Status::Graa, false), "intet hentet");
        // I går hentet, før døgnet var slut (+1 t), og klokken er over 03:00: gul "gamle", også med mange angreb.
        let mut h = hentet_alt();
        h.insert(d(1), iso(dag_ms(&d(0)).unwrap() + 1_800_000));
        assert_eq!(grundlag(&h, NU), Grundlag::Gammel);
        assert_eq!(status(Grundlag::Gammel, 1000, 0.0), (Status::Gul, true));
        // Alt hentet for 27 timer siden.
        let h: HashMap<String, String> = (0..=HENT_DAGE).map(|i| (d(i), iso(NU - 27 * 3_600_000))).collect();
        assert_eq!(grundlag(&h, NU), Grundlag::Gammel);
        // Friske tal: angrebene afgør.
        assert_eq!(grundlag(&hentet_alt(), NU), Grundlag::IGaar);
        assert_eq!(status(Grundlag::IGaar, 1000, 0.0), (Status::Roed, false));
    }

    #[test]
    fn hele_doegn_vinduer_og_ugen_foer() {
        let mut rows = Vec::new();
        for i in 0..=31 {
            rows.push(fw(&d(i), "firewallManaged", "block", 1));
            rows.push(Raekke {
                dag: d(i),
                art: "alle".into(),
                noegle: String::new(),
                n: 100,
            });
        }
        rows.push(fw(&d(3), "x", "js_challenge", 5));
        rows.push(fw(&d(3), "x", "log", 2));
        let h = hele_doegn(&rows, NU);
        assert_eq!((h.i_dag_hidtil, h.i_gaar), (1, 1));
        assert_eq!(h.hele7, 7 + 5, "d-7…d-1");
        assert_eq!(h.hele30, 30 + 5, "d-30…d-1 (d-31 og i dag tæller ikke; `log` er ikke afvist)");
        assert_eq!(h.ugen_foer, 7, "d-14…d-8");
        assert_eq!(h.alle_hele7, 700);
        assert_eq!(
            h.fordeling.d30,
            Fordeling {
                blokeret: 30,
                udfordret: 5,
                labyrint: 0,
                andet: 2,
                faelder: 0
            }
        );
        let a = angreb(&rows, NU);
        assert_eq!((a.uge, a.uge_foer), (7 + 7, 7), "angreb: samme vinduer som de hele døgn");
        assert_eq!(aendring(14, 7), Some(100));
        assert_eq!(aendring(3408, 2400), Some(42), "wi24rd.com +42 %");
        assert_eq!(aendring(479, 774), Some(-38));
        assert_eq!((aendring(0, 0), aendring(5, 0)), (Some(0), None));
        assert_eq!(hele_fra(NU), d(30));
    }

    // ── P3: årsagen ved status ──
    #[test]
    fn egne_regler_kendes_paa_kilden() {
        let rows = vec![
            fw_regel(
                &d(1),
                "min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)",
                "ratelimit",
                "block",
                2,
            ),
            fw_regel(&d(1), "React - RCE - CVE:CVE-2025-55182", "firewallManaged", "block", 1),
        ];
        let a = angreb(&rows, NU);
        assert_eq!(
            a.top.iter().map(|t| (t.navn.as_str(), t.egen)).collect::<Vec<_>>(),
            [
                ("min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)", true),
                ("React RCE (CVE-2025-55182)", false)
            ]
        );
        assert_eq!(EGNE_KILDER, ["firewallCustom", "ratelimit"]);
    }

    /// J9: "afvist" er blokeret + udfordret + lokket ind; siderne med fælde-links står for sig; statusreglen er uændret.
    #[test]
    fn afvist_uden_sider_med_faelde_links() {
        for (h, g, afvist) in [
            ("block", BLOKERET, true),
            ("managed_challenge", UDFORDRET, true),
            ("challenge", UDFORDRET, true),
            ("js_challenge", UDFORDRET, true),
            ("link_maze_visited", LABYRINT, true),
            ("link_maze_injected", FAELDER, false),
            ("log", ANDET, false),
            ("", ANDET, false),
        ] {
            assert_eq!((gruppe(h), er_afvist(h)), (g, afvist), "{h}");
        }
        let rows = vec![
            fw(&d(1), "firewallManaged", "block", 3),
            fw(&d(1), "botFight", "managed_challenge", 4),
            fw(&d(1), "linkMaze", "link_maze_visited", 5),
            fw(&d(1), "linkMaze", "link_maze_injected", 1000),
            fw(&d(1), "firewallCustom", "log", 7),
            fw(&d(0), "linkMaze", "link_maze_injected", 50),
            fw(&d(0), "linkMaze", "link_maze_visited", 2),
        ];
        let h = hele_doegn(&rows, NU);
        assert_eq!((h.i_gaar, h.hele7, h.hele30, h.i_dag_hidtil), (12, 12, 12, 2));
        let vent = Fordeling {
            blokeret: 3,
            udfordret: 4,
            labyrint: 5,
            andet: 7,
            faelder: 1000,
        };
        assert_eq!((h.fordeling.i_gaar, h.fordeling.d30), (vent, vent));
        let a = angreb(&rows, NU);
        assert_eq!(a.i_gaar, 3 + 7, "statusreglen: alt uden for rutine, uændret");
    }

    #[test]
    fn regel_art_efter_kilde() {
        for k in RUTINE {
            assert_eq!(regel_art(k), "rutine");
        }
        for k in EGNE_KILDER {
            assert_eq!(regel_art(k), "egen");
        }
        assert_eq!(regel_art("firewallManaged"), "angreb");
    }

    /// "Se de N angreb" — dagens angreb som statusreglen; de gemte enkelthændelser uden rutine; reglerne bag.
    #[test]
    fn angreb_dage_i_gaar_uden_rutine() {
        let senest = |dag: &str, tid: &str, regel: &str, cve: &str, n: i64| Raekke {
            dag: dag.into(),
            art: "seneste".into(),
            noegle: [tid, regel, cve, "US", "/", "EWR", "GET"].join("\t"),
            n,
        };
        let i_gaar = d(1);
        let rows = vec![
            fw_regel(&i_gaar, "React - RCE - CVE:CVE-2025-55182", "firewallManaged", "block", 20),
            fw_regel(&i_gaar, "Wordpress - Broken Access Control", "firewallManaged", "block", 10),
            fw_regel(&i_gaar, "linkMaze", "linkMaze", "link_maze_injected", 500),
            fw_regel(&d(2), "Wordpress - Broken Access Control", "firewallManaged", "block", 4),
            senest(
                &i_gaar,
                &format!("{i_gaar}T23:00:00Z"),
                "React - RCE - CVE:CVE-2025-55182",
                "CVE-2025-55182",
                2,
            ),
            senest(&i_gaar, &format!("{i_gaar}T22:00:00Z"), "linkMaze", "", 9),
            senest(&i_gaar, &format!("{i_gaar}T21:00:00Z"), "Browser Integrity Check", "", 1),
            senest(
                &d(0),
                &format!("{}T01:00:00Z", d(0)),
                "React - RCE - CVE:CVE-2025-55182",
                "CVE-2025-55182",
                3,
            ),
        ];
        let ad = crate::vaern::angreb_dage(&rows, NU);
        assert_eq!(ad.len(), 2);
        assert_eq!(
            (ad[0].dag.as_str(), ad[0].angreb),
            (i_gaar.as_str(), 30),
            "som statusreglen: rutine tæller ikke"
        );
        assert_eq!(ad[0].seneste.len(), 2, "kun i går, rutine (labyrint, browsertjek) sorteret fra");
        assert!(ad[0].seneste.iter().all(|s| s.cve.as_deref() == Some("CVE-2025-55182")));
        assert_eq!(
            ad[0].regler.iter().map(|r| (r.navn.as_str(), r.n)).collect::<Vec<_>>(),
            [("React - RCE - CVE:CVE-2025-55182", 20), ("Wordpress - Broken Access Control", 10)]
        );
        assert_eq!(
            (ad[1].dag.as_str(), ad[1].angreb, ad[1].seneste.len()),
            (d(2).as_str(), 4, 0),
            "i forgårs"
        );
    }

    #[test]
    fn kort_regelnavn_som_regelkortet() {
        assert_eq!(
            kort_regelnavn("React - RCE - CVE:CVE-2025-55182", false),
            "React RCE (CVE-2025-55182)"
        );
        assert_eq!(
            kort_regelnavn("Wordpress - Broken Access Control, File Inclusion", false),
            "WordPress"
        );
        assert_eq!(
            kort_regelnavn("Wordpress - Remote Code Execution - CVE:CVE-2026-63030", false),
            "WordPress Remote Code Execution (CVE-2026-63030)"
        );
        assert_eq!(
            kort_regelnavn("Apache Struts - CVE:CVE-2017-5638", false),
            "Apache Struts (CVE-2017-5638)"
        );
        // Egne regler uden " - " og browsertjekket står med det fulde navn.
        let egen = "min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)";
        assert_eq!(kort_regelnavn(egen, true), egen);
        assert_eq!(kort_regelnavn("wordpress", true), "wordpress", "en egen regel beholder sit navn");
        assert_eq!(kort_regelnavn("Browser Integrity Check", false), "Browser Integrity Check");
    }

    /// Eksempel, i går: 35 React, 49 + 3 + 1 Wordpress, 2 min-regel, 1.200 labyrint (rutine).
    #[test]
    fn aarsag_top3_og_fordeling_for_angreb_i_gaar() {
        let rows = vec![
            fw_regel(
                &d(1),
                "Wordpress - Broken Access Control, File Inclusion",
                "firewallManaged",
                "block",
                49,
            ),
            fw_regel(&d(1), "React - RCE - CVE:CVE-2025-55182", "firewallManaged", "block", 35),
            fw_regel(
                &d(1),
                "Wordpress - Remote Code Execution - CVE:CVE-2026-63030",
                "firewallManaged",
                "block",
                3,
            ),
            fw_regel(
                &d(1),
                "Wordpress - SQL Injection - CVE:CVE-2026-60137",
                "firewallManaged",
                "block",
                1,
            ),
            fw_regel(&d(1), "Wordpress - Dangerous File Upload", "firewallManaged", "block", 4),
            fw_regel(
                &d(1),
                "min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)",
                "ratelimit",
                "block",
                2,
            ),
            fw_regel(&d(1), "linkMaze", "linkMaze", "link_maze_injected", 1200),
            fw_regel(&d(2), "React - RCE - CVE:CVE-2025-55182", "firewallManaged", "block", 500),
        ];
        let a = angreb(&rows, NU);
        assert_eq!(a.i_gaar, 49 + 35 + 3 + 1 + 4 + 2);
        assert_eq!(
            typer(&a.top),
            [
                ("WordPress", 53),
                ("React RCE (CVE-2025-55182)", 35),
                ("WordPress Remote Code Execution (CVE-2026-63030)", 3)
            ],
            "samme korte navn lægges sammen; rutine og andre døgn tæller ikke; højst 3"
        );
        assert_eq!(
            a.fordeling,
            Fordeling {
                blokeret: 94,
                ..Fordeling::default()
            },
            "alle blokeret"
        );
        // Udfordret og labyrint skilles ad, og ved lige mange står navnet først.
        let rows = vec![
            fw_regel(&d(1), "B - x", "firewallManaged", "block", 5),
            fw_regel(&d(1), "A - x", "firewallManaged", "managed_challenge", 5),
            fw_regel(&d(1), "C - x", "firewallCustom", "link_maze_visited", 1),
        ];
        let a = angreb(&rows, NU);
        assert_eq!(typer(&a.top), [("A", 5), ("B", 5), ("C", 1)]);
        assert_eq!(
            a.top.iter().map(|t| t.egen).collect::<Vec<_>>(),
            [false, false, true],
            "firewallCustom er en egen regel, firewallManaged ikke"
        );
        assert_eq!(
            a.fordeling,
            Fordeling {
                blokeret: 5,
                udfordret: 5,
                labyrint: 1,
                andet: 0,
                faelder: 0
            }
        );
        let r = raekke(&zone("a.wi24rd.com"), &hentet_alt(), &rows, NU);
        assert_eq!((r.angreb_i_gaar, r.angreb_fordeling.udfordret), (11, 5));
        let j = serde_json::to_value(&r).unwrap();
        assert_eq!(
            j["angreb_i_gaar_top"][0],
            serde_json::json!({"navn": "A", "n": 5, "egen": false, "regler": ["A - x"], "tek": "a"})
        );
        assert_eq!(j["angreb_fordeling"]["blokeret"], 5);
    }

    // ── P4: "Ugen før" på samme grundlag som status ──
    /// alfa.wi24rd.com: labyrinten voksede voldsomt (+1.490 % stoppet), angrebene ikke. Kolonnen viser angreb.
    #[test]
    fn ugen_foer_taeller_angreb_ikke_rutine() {
        let mut rows = Vec::new();
        for i in 1..=7 {
            rows.push(fw(&d(i), "linkMaze", "link_maze_injected", 1000));
            rows.push(fw(&d(i), "firewallManaged", "block", 2));
        }
        for i in 8..=14 {
            rows.push(fw(&d(i), "linkMaze", "link_maze_injected", 60));
            rows.push(fw(&d(i), "firewallManaged", "block", 3));
        }
        let r = raekke(&zone("alfa.wi24rd.com"), &hentet_alt(), &rows, NU);
        assert_eq!((r.angreb_uge, r.angreb_ugen_foer, r.aendring), (14, 21, Some(-33)));
        assert_eq!(r.status, Status::Groen);
        // Afvist i går / 7 / 30: kun blokeringerne her, for siderne med fælde-links blev leveret (J9).
        assert_eq!((r.hele.i_gaar, r.hele.hele7, r.hele.hele30, r.hele.ugen_foer), (2, 14, 14 + 21, 21));
        assert_eq!(
            aendring(7014, 441),
            Some(1490),
            "det gamle grundlag (alt i `fw`) ville vise +1.490 %"
        );
    }

    // ── P5: ingen natlig gul ──
    #[test]
    fn natten_bygger_paa_doegnet_foer_og_er_ikke_gul() {
        let nat = kl("2026-11-03T01:30:00Z");
        // Køen hentede alt kl. 00:40: i går (11-02) er ikke færdig (det bliver den først kl. 01:00), døgnet før er.
        let hentet = hentet_kl(nat, "2026-11-03T00:40:00Z");
        assert_eq!(grundlag(&hentet, nat), Grundlag::Forgaars);
        let mut rows = vec![
            fw_regel("2026-11-02", "React - RCE - CVE:CVE-2025-55182", "firewallManaged", "block", 3),
            fw_regel("2026-11-01", "React - RCE - CVE:CVE-2025-55182", "firewallManaged", "block", 40),
        ];
        for dag in [
            "2026-10-25",
            "2026-10-26",
            "2026-10-27",
            "2026-10-28",
            "2026-10-29",
            "2026-10-30",
            "2026-10-31",
        ] {
            rows.push(fw(dag, "firewallManaged", "block", 2));
        }
        let r = raekke(&zone("a.wi24rd.com"), &hentet, &rows, nat);
        assert_eq!((r.status, r.gammel, r.opdateres), (Status::Roed, false, true));
        assert_eq!(
            (r.angreb_i_gaar, r.angreb_snit),
            (40, 2.0),
            "statusdøgnet er 11-01, snittet 10-25…10-31"
        );
        assert_eq!(typer(&r.angreb_i_gaar_top), [("React RCE (CVE-2025-55182)", 40)]);
        assert_eq!(
            (r.angreb_uge, r.angreb_ugen_foer),
            (40 + 6 * 2, 2),
            "ugen 10-26…11-01, ugen før 10-19…10-25"
        );
        assert_eq!(r.hele.i_gaar, 3, "\"Stoppet i går\" er stadig i går");

        // Klokken 02:59:59 er det stadig nat; fra 03:00 er tallene gamle.
        assert_eq!(grundlag(&hentet, kl("2026-11-03T02:59:59Z")), Grundlag::Forgaars);
        assert_eq!(grundlag(&hentet, kl("2026-11-03T03:00:00Z")), Grundlag::Gammel);
        let r = raekke(&zone("a.wi24rd.com"), &hentet, &rows, kl("2026-11-03T03:30:00Z"));
        assert_eq!((r.status, r.gammel, r.opdateres), (Status::Gul, true, false));

        // Natten, men i går er færdighentet (01:10): i går gælder.
        assert_eq!(grundlag(&hentet_kl(nat, "2026-11-03T01:10:00Z"), nat), Grundlag::IGaar);
        // Natten, men døgnet før er heller ikke færdigt: gul.
        assert_eq!(grundlag(&hentet_kl(nat, "2026-11-02T00:30:00Z"), nat), Grundlag::Gammel);
        // Natten, men sidst hentet for over 26 t siden: gul.
        assert_eq!(grundlag(&hentet_kl(nat, "2026-11-01T23:00:00Z"), nat), Grundlag::Gammel);
        // Lige efter midnat er i går aldrig færdig: ingen gul, så længe døgnet før er hentet.
        let midnat = kl("2026-11-03T00:00:00Z");
        assert_eq!(grundlag(&hentet_kl(midnat, "2026-11-02T23:30:00Z"), midnat), Grundlag::Forgaars);
        assert_eq!(
            (NAT_MS, Grundlag::Forgaars.forskyd(), Grundlag::IGaar.forskyd()),
            (3 * 3_600_000, 1, 0)
        );
    }

    /// Q4: tærsklerne, og at kun gul og rød kan få en sætning.
    #[test]
    fn tendens_foelger_tabellen() {
        let t = |s, m, v| tendens(s, false, Some(m), Some(v));
        assert_eq!(t(Status::Gul, 2.0, 1.5), Some(Tendens::Foelger));
        assert_eq!(t(Status::Roed, 3.0, 1.5), Some(Tendens::Foelger), "præcis 2 · r_ref");
        assert_eq!(t(Status::Gul, 3.1, 1.5), None, "steg mere end verden, men verden er ikke flad");
        assert_eq!(t(Status::Gul, 5.0, 1.29), Some(Tendens::SaerligtDig));
        assert_eq!(t(Status::Gul, 5.0, 1.3), None, "mellem de to: ingen sætning");
        assert_eq!(t(Status::Gul, 3.0, 1.0), Some(Tendens::SaerligtDig), "præcis 3 × snit");
        assert_eq!(t(Status::Gul, 2.9, 1.0), None, "verden er flad, men stigningen er for lille");
        assert_eq!(t(Status::Groen, 5.0, 1.0), None, "små tal og grønne døgn: ingen sætning");
        assert_eq!(t(Status::Graa, 5.0, 1.0), None);
        assert_eq!(tendens(Status::Gul, true, Some(5.0), Some(1.0)), None, "gamle tal");
        assert_eq!(tendens(Status::Gul, false, None, Some(1.0)), None, "ingen WAF-angreb");
        assert_eq!(tendens(Status::Gul, false, Some(5.0), None), None, "ingen Radar-tal");
        assert_eq!(t(Status::Gul, f64::INFINITY, 1.0), Some(Tendens::SaerligtDig), "ingen angreb før");
    }

    /// r_mig tæller kun `firewallManaged`, og snittet er de 7 døgn før statusdøgnet.
    #[test]
    fn r_mig_kun_waf_kilden() {
        let rows = [
            fw(&d(1), "firewallManaged", "block", 30),
            fw(&d(1), "botFight", "block", 900),
            fw(&d(1), "firewallCustom", "block", 900),
            fw(&d(2), "firewallManaged", "block", 70),
            fw(&d(9), "firewallManaged", "block", 700),
        ];
        assert_eq!(r_mig(&rows, NU), Some(3.0));
        assert_eq!(r_mig(&rows[3..], NU), None, "ingen WAF-angreb i døgnet");
        assert_eq!(r_mig(&rows[..1], NU), Some(f64::INFINITY));
    }

    #[test]
    fn overblik_sorteres_roed_gul_groen_graa_og_navn() {
        let r = |navn: &str, status| Raekke1 {
            id: navn.into(),
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
            angreb_i_gaar: 0,
            angreb_i_gaar_top: Vec::new(),
            angreb_fordeling: Fordeling::default(),
            angreb_snit: 0.0,
            angreb_uge: 0,
            angreb_ugen_foer: 0,
            sidst_hentet: None,
            doegn_faerdige: 0,
            doegn_i_alt: 31,
            tendens: None,
            trafik: None,
            server: None,
            forstyrrelse: None,
            angreb_status: status,
            ikke_brugt: Vec::new(),
            uventet: None,
            naaet: None,
            tek_besked: Vec::new(),
            tek_spoerg: None,
            med7: 7,
            med30: 30,
            laek: None,
            dmarc: None,
            tilstand: vec![],
            sti200: None,
            cf_fund: None,
            moenster: None,
            afgjort: None,
            efterslaeb: vec![],
            vurderet: None,
            forgaars: false,
            kan_sammenligne: true,
        };
        let mut v = vec![
            r("b.wi24rd.com", Status::Graa),
            r("c.wi24rd.com", Status::Groen),
            r("a.wi24rd.com", Status::Graa),
            r("d.wi24rd.com", Status::Roed),
            r("e.wi24rd.com", Status::Gul),
            r("a.wi24rd.com", Status::Groen),
        ];
        sorter(&mut v);
        let ud: Vec<(&str, Status)> = v.iter().map(|x| (x.navn.as_str(), x.status)).collect();
        assert_eq!(
            ud,
            [
                ("d.wi24rd.com", Status::Roed),
                ("e.wi24rd.com", Status::Gul),
                ("a.wi24rd.com", Status::Groen),
                ("c.wi24rd.com", Status::Groen),
                ("a.wi24rd.com", Status::Graa),
                ("b.wi24rd.com", Status::Graa)
            ]
        );
        let j = serde_json::to_value(&v[0]).unwrap();
        assert_eq!(j["status"], "roed");
        assert!(
            j.get("i_gaar").is_some() && j.get("fordeling").is_some(),
            "hele døgn står fladt i rækken"
        );
        for k in [
            "opdateres",
            "angreb_i_gaar_top",
            "angreb_fordeling",
            "angreb_uge",
            "angreb_ugen_foer",
        ] {
            assert!(j.get(k).is_some(), "{k}");
        }
        assert!(j.get("angreb_foer").is_none());
    }

    #[test]
    fn faerdige_doegn_taelles_i_hentevinduet() {
        let z = zone("a.wi24rd.com");
        let r = raekke(&z, &hentet_alt(), &[], NU);
        assert_eq!((r.doegn_faerdige, r.doegn_i_alt), (30, 31), "i dag er aldrig færdigt");
        let tom = raekke(&z, &HashMap::new(), &[], NU);
        assert_eq!((tom.status, tom.doegn_faerdige, tom.sidst_hentet), (Status::Graa, 0, None));
    }
}
