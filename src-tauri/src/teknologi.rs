//! Domænets teknologi: brugeren siger pr. domæne og teknologi "Bruger vi", "Bruger vi ikke" eller "Ved ikke" (standard). Vi gætter aldrig
//! selv (J2): data kan kun give et forslag. "Bruger vi ikke" ændrer kun status og advarsler; angrebene står fuldt i Angreb, tal, kort og dyk.
//! Teknologien er produktdelen før første " - " i Cloudflares regelnavn ("Magento 2 - Unrestricted file upload"), så hvert produkt, Cloudflare navngiver, kommer med af sig selv (`af_regel`). `KATALOG` giver
//! pæne navne, aliaser og grupper. Id'et er det normaliserede navn, så valg står ved et nyt produkt uden en migrering, og de syv gamle id'er er uændrede. Ellers kortlægges fra stien (`vaern::forsoeg_kategori`).
//! Egne regler og rutine har ingen teknologi, og de generiske slags (hemmeligheder, kildekode, adgang) og regeltyper (`GENERISKE_REGLER`) er altid relevante og får intet valg. Hvad der vises som
//! kolonne på `#teknologi`, står i `tek_vis` og ændrer aldrig status, tal eller "Hvad angrebene var rettet mod".
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::cf::Fejl;
use crate::overblik::{er_afvist, EGNE_KILDER, RUTINE, SNIT_FRA, SNIT_TIL};
use crate::store::{Raekke, Store};
use crate::vaern::{dag_noegle, forsoeg_kategori, DOEGN_MS, HENT_DAGE};

/// (id, mønster, navn). Bruges, når produktdelen ikke selv siger det: regler uden " - ", generiske regeltyper og produkter med teknologien i navnet. Første træf vinder: en WordPress-regel om PHP er WordPress.
pub const TEKNOLOGIER: &[(&str, &str, &str)] = &[
    ("react", "(?i)react", "React"),
    ("nextjs", "(?i)next\\.?js", "Next.js"),
    ("log4j", "(?i)log4j|log4shell|struts|\\bjava\\b", "Java/Log4j"),
    ("wordpress", "(?i)wordpress|\\bwp[-_ ]", "WordPress"),
    ("joomla", "(?i)joomla", "Joomla"),
    ("drupal", "(?i)drupal", "Drupal"),
    ("php", "(?i)\\bphp", "PHP"),
];
/// (id, navn, gruppe, begyndelser på produktdelen i små bogstaver). Første træf vinder, så "apache struts" står før andre Apache-produkter, og "next" aldrig er React. Samme navne som `TEK` i ui/assets/sprog.js.
pub const KATALOG: &[(&str, &str, &str, &[&str])] = &[
    ("wordpress", "WordPress", "cms", &["wordpress", "wp "]),
    ("joomla", "Joomla", "cms", &["joomla"]),
    ("drupal", "Drupal", "cms", &["drupal"]),
    ("magento", "Magento", "cms", &["magento", "adobe commerce"]),
    ("ghost", "Ghost", "cms", &["ghost"]),
    ("plone", "Plone", "cms", &["plone"]),
    ("typo3", "TYPO3", "cms", &["typo3"]),
    ("prestashop", "PrestaShop", "cms", &["prestashop"]),
    ("nextjs", "Next.js", "ramme", &["next.js", "nextjs"]),
    ("react", "React", "ramme", &["react"]),
    ("laravel", "Laravel", "ramme", &["laravel"]),
    ("symfony", "Symfony", "ramme", &["symfony"]),
    ("django", "Django", "ramme", &["django"]),
    ("rails", "Ruby on Rails", "ramme", &["ruby on rails", "rails"]),
    ("spring", "Spring", "ramme", &["spring"]),
    ("nodejs", "Node.js", "ramme", &["node.js", "nodejs"]),
    (
        "log4j",
        "Java/Log4j",
        "server",
        &["apache struts", "struts", "log4j", "log4shell", "java"],
    ),
    ("php", "PHP", "server", &["php"]),
    ("tomcat", "Apache Tomcat", "server", &["apache tomcat", "tomcat"]),
    ("apache camel", "Apache Camel", "server", &["apache camel"]),
    ("nginx", "nginx", "server", &["nginx"]),
    ("mongodb", "MongoDB", "server", &["mongodb"]),
    ("redis", "Redis", "server", &["redis"]),
    ("oracle", "Oracle", "server", &["oracle"]),
    ("exchange", "Microsoft Exchange", "samarbejde", &["microsoft exchange", "exchange"]),
    (
        "sharepoint",
        "Microsoft SharePoint",
        "samarbejde",
        &["microsoft sharepoint", "sharepoint"],
    ),
    (
        "confluence",
        "Atlassian Confluence",
        "samarbejde",
        &["atlassian confluence", "confluence"],
    ),
    ("jira", "Atlassian Jira", "samarbejde", &["atlassian jira", "jira"]),
    ("zimbra", "Zimbra", "samarbejde", &["zimbra"]),
    ("roundcube", "Roundcube", "samarbejde", &["roundcube"]),
    ("fortinet", "Fortinet", "netvaerk", &["fortinet", "forti"]),
    ("ivanti", "Ivanti", "netvaerk", &["ivanti"]),
    ("citrix", "Citrix", "netvaerk", &["citrix"]),
    ("cisco", "Cisco", "netvaerk", &["cisco"]),
    ("palo alto", "Palo Alto Networks", "netvaerk", &["palo alto"]),
    ("sonicwall", "SonicWall", "netvaerk", &["sonicwall"]),
    ("f5", "F5 BIG-IP", "netvaerk", &["f5 "]),
    ("gitlab", "GitLab", "andet", &["gitlab"]),
    ("jenkins", "Jenkins", "andet", &["jenkins"]),
    ("grafana", "Grafana", "andet", &["grafana"]),
    ("solarwinds", "SolarWinds", "andet", &["solarwinds"]),
    ("splunk", "Splunk", "andet", &["splunk"]),
];
/// Regeltyper, der ligner et produkt, men ikke er det: produktdelen er angrebsformen. Begyndelser i små bogstaver.
pub const GENERISKE_REGLER: &[&str] = &[
    "generic",
    "anomaly",
    "sqli",
    "sql injection",
    "xss",
    "command injection",
    "code injection",
    "remote code execution",
    "rce",
    "ssrf",
    "file inclusion",
    "lfi",
    "rfi",
    "path traversal",
    "directory traversal",
    "information disclosure",
    "http ",
    "cloudflare",
];
/// Linjen i "Hvad angrebene var rettet mod" for det, regelnavnet ikke siger noget om. Kan ikke vælges.
pub const ANDET: &str = "andet";
/// Længste id: en produktdel, der er længere, er ikke et produktnavn.
const MAKS_TEK: usize = 40;
/// De slags fra `forsoeg_kategori`, der aldrig får et valg.
pub const GENERISKE: &[&str] = &["hemmeligheder", "kildekode", "adgang"];
/// "Giv mig besked": døgnet har mindst så mange angreb af typen og mindst `BESKED_FAKTOR` gange snittet af de 7 døgn før.
pub const BESKED_MIN: i64 = 5;
pub const BESKED_FAKTOR: f64 = 3.0;
/// Forslaget kræver mindst så mange forsøg på 30 dage: tre tilfældige 404 siger intet om sitet.
pub const FORSLAG_MIN: i64 = 20;

static MOENSTRE: LazyLock<Vec<Regex>> =
    LazyLock::new(|| TEKNOLOGIER.iter().map(|t| Regex::new(t.1).expect("teknologiens mønster")).collect());

/// Små bogstaver, ét mellemrum, og uden et løbenummer til sidst ("SonicWall SSLVPN 2", "Magento 2").
fn normaliser(tekst: &str) -> String {
    let ord: Vec<String> = tekst.split_whitespace().map(str::to_lowercase).collect();
    let n = ord.len() - usize::from(ord.len() > 1 && ord.last().is_some_and(|o| o.chars().all(|c| c.is_ascii_digit())));
    ord[..n].join(" ")
}

/// Et id, der kan gemmes og vælges. Hverken generiske slags eller "Andet". Ren funktion.
pub fn kendt(tek: &str) -> bool {
    !tek.is_empty()
        && tek.chars().count() <= MAKS_TEK
        && tek != ANDET
        && !GENERISKE.contains(&tek)
        && tek.chars().all(|c| c.is_alphanumeric() || " ./+_-".contains(c))
}

/// Id'et for et produktnavn: aliaset, ellers det normaliserede navn. `None`: det er en angrebsform, eller det kan ikke være et navn. Ren funktion.
pub fn id_for(produkt: &str) -> Option<String> {
    let n = normaliser(produkt);
    let starter = |p: &&str| n.starts_with(*p) || n == p.trim_end();
    if let Some(k) = KATALOG.iter().find(|k| k.3.iter().any(starter)) {
        return Some(k.0.to_string());
    }
    (!GENERISKE_REGLER.iter().any(starter) && kendt(&n)).then_some(n)
}

fn gammelt_moenster(tekst: &str) -> Option<String> {
    TEKNOLOGIER
        .iter()
        .zip(MOENSTRE.iter())
        .find(|(_, m)| m.is_match(tekst))
        .map(|(t, _)| t.0.to_string())
}

/// Teknologien bag et regelnavn: produktdelen før første " - ". Uden produkt, eller ved en generisk regeltype, afgør mønstrene over hele navnet. Ren funktion.
pub fn af_regel(navn: &str) -> Option<String> {
    let produkt = match navn.split_once(" - ") {
        Some((p, _)) if !p.trim().is_empty() => p,
        _ => return gammelt_moenster(navn),
    };
    match id_for(produkt) {
        Some(id) => Some(
            gammelt_moenster(produkt)
                .filter(|_| !KATALOG.iter().any(|k| k.0 == id))
                .unwrap_or(id),
        ),
        None => gammelt_moenster(navn),
    }
}

/// Teknologien bag en sti. `wp-config` er en hemmelighed og altså generisk. Ren funktion.
pub fn af_sti(sti: &str) -> Option<&'static str> {
    match forsoeg_kategori(sti) {
        Some("wordpress") => Some("wordpress"),
        Some("php") => Some("php"),
        _ => None,
    }
}

/// Teknologien bag en `fw`-række: reglen, ellers stien. Rutine og egne regler har ingen. Ren funktion.
pub fn af_fw(regel: &str, sti: &str, kilde: &str) -> Option<String> {
    if RUTINE.contains(&kilde) || EGNE_KILDER.contains(&kilde) {
        return None;
    }
    af_regel(regel).or_else(|| af_sti(sti).map(str::to_string))
}

/// Navnet til teksterne: katalogets, ellers id'et med stort begyndelsesbogstav i hvert ord (samme regel som `tekNavn` i ui/assets/sprog.js).
pub fn navn(tek: &str) -> String {
    KATALOG.iter().find(|k| k.0 == tek).map_or_else(
        || {
            tek.split(' ')
                .map(|o| {
                    let mut c = o.chars();
                    c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
                })
                .collect::<Vec<_>>()
                .join(" ")
        },
        |k| k.1.to_string(),
    )
}

/// Gruppen i "Alle teknologier". Alt uden for kataloget er "andet".
pub fn gruppe(tek: &str) -> &'static str {
    KATALOG.iter().find(|k| k.0 == tek).map_or("andet", |k| k.2)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Valg {
    #[default]
    VedIkke,
    Bruger,
    BrugerIkke,
}

impl Valg {
    pub fn fra(s: &str) -> Option<Valg> {
        match s {
            "ved_ikke" => Some(Valg::VedIkke),
            "bruger" => Some(Valg::Bruger),
            "bruger_ikke" => Some(Valg::BrugerIkke),
            _ => None,
        }
    }

    pub fn kode(self) -> &'static str {
        match self {
            Valg::VedIkke => "ved_ikke",
            Valg::Bruger => "bruger",
            Valg::BrugerIkke => "bruger_ikke",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TekValg {
    pub valg: Valg,
    pub besked: bool,
}

/// Ét domænes valg. En teknologi uden post er "Ved ikke".
pub type Profil = HashMap<String, TekValg>;

pub fn profil(lager: &Store, zone: &str) -> Result<Profil, Fejl> {
    Ok(lager
        .teknologi(zone)?
        .into_iter()
        .filter_map(|(t, v, besked)| Valg::fra(&v).map(|valg| (t, TekValg { valg, besked })))
        .collect())
}

/// Gemmer ét valg. "Giv mig besked" findes kun ved "Bruger vi".
pub fn saet(lager: &mut Store, zone: &str, tek: &str, valg: Valg, besked: bool, sat_utc: &str) -> Result<(), Fejl> {
    if !kendt(tek) {
        return Err(Fejl::UgyldigtFilter);
    }
    lager.gem_teknologi(zone, tek, valg.kode(), besked && valg == Valg::Bruger, sat_utc)
}

/// Felterne i en række med svar fra serveren: (sti, origin-status). Art `login` og `post` er de eneste med `originResponseStatus` (`adgang::raekker`).
fn svar_fra_server(r: &Raekke) -> Option<(&str, i64)> {
    let plads = match r.art.as_str() {
        "login" => 3,
        "post" => 2,
        _ => return None,
    };
    let mut f = r.noegle.split('\t');
    let sti = f.next()?;
    Some((sti, f.nth(plads - 1)?.parse().ok()?))
}

/// Sikkerhedsnettet: "200/302". En 301 er oftest hele sitets omdirigering (http til https, www), ikke siden: målt baglæns gav den wi24rd.com gult på 10 af 30 døgn
/// uden en eneste 2xx.
pub fn er_svar(origin: i64) -> bool {
    (200..300).contains(&origin) || origin == 302
}

/// Forslaget siger "alle fik 404", så her tæller enhver 2xx og 3xx imod.
fn modsiger_404(origin: i64) -> bool {
    (200..400).contains(&origin)
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TekN {
    pub tek: String,
    pub n: i64,
}

/// Et svar 2xx/3xx fra serveren på en sti af en type, brugeren siger ikke bruges.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Uventet {
    pub tek: String,
    pub sti: String,
    pub n: i64,
}

/// "Giv mig besked": typen ses mere end normalt.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TekBesked {
    pub tek: String,
    pub n: i64,
    pub snit: f64,
}

/// Hvad valgene betyder for statusdøgnet.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Vurdering {
    /// Angreb på "Bruger vi ikke"-typer: de tæller ikke i status, og Hvorfor nævner dem.
    pub ikke_brugt: Vec<TekN>,
    /// Sikkerhedsnettet.
    pub uventet: Option<Uventet>,
    /// Angreb på en "Bruger vi"-type, som Cloudflare ikke stoppede, så de nåede serveren. Giver Handl.
    pub naaet: Option<TekN>,
    pub besked: Vec<TekBesked>,
    /// Den største type i døgnet, brugeren ikke har taget stilling til: linket "Bruger du ikke React?".
    pub spoerg: Option<String>,
}

/// Skal rækken holdes ude af status? Kun angreb på en "Bruger vi ikke"-type. Ren funktion.
pub fn udeladt(r: &Raekke, p: &Profil) -> bool {
    if r.art != "fw" {
        return false;
    }
    let mut f = r.noegle.split('\t');
    let (regel, sti, kilde) = (f.next().unwrap_or(""), f.nth(2).unwrap_or(""), f.nth(2).unwrap_or(""));
    af_fw(regel, sti, kilde).is_some_and(|t| p.get(&t).is_some_and(|v| v.valg == Valg::BrugerIkke))
}

/// Den største; ved lige mange den første efter navn.
fn stoerst<'a>(m: impl Iterator<Item = (&'a str, i64)>) -> Option<(&'a str, i64)> {
    m.max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
}

/// Statusdøgnet er døgnet før `nu`, som i `overblik::angreb`. Ren funktion.
pub fn vurder(rows: &[Raekke], p: &Profil, nu: i64) -> Vurdering {
    let d = |i: i64| dag_noegle(nu - i * DOEGN_MS);
    let (dag, fra, til) = (d(1), d(SNIT_FRA), d(SNIT_TIL));
    let (mut i_dag, mut foer) = (BTreeMap::<String, i64>::new(), BTreeMap::<String, i64>::new());
    let mut naaet = BTreeMap::<String, i64>::new();
    for r in rows.iter().filter(|r| r.art == "fw") {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        let Some(t) = af_fw(felt(0), felt(3), felt(6)) else { continue };
        if r.dag == dag {
            *i_dag.entry(t.clone()).or_default() += r.n;
            if !er_afvist(felt(7)) {
                *naaet.entry(t).or_default() += r.n;
            }
        } else if r.dag >= fra && r.dag <= til {
            *foer.entry(t).or_default() += r.n;
        }
    }
    let valg = |t: &str| p.get(t).copied().unwrap_or_default();
    let mut ikke_brugt: Vec<TekN> = i_dag
        .iter()
        .filter(|(t, _)| valg(t).valg == Valg::BrugerIkke)
        .map(|(t, n)| TekN { tek: t.clone(), n: *n })
        .collect();
    ikke_brugt.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.tek.cmp(&b.tek)));
    let mut svar: BTreeMap<(&str, &str), i64> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.dag == dag) {
        if let Some((sti, _)) = svar_fra_server(r).filter(|x| er_svar(x.1)) {
            if let Some(t) = af_sti(sti).filter(|t| valg(t).valg == Valg::BrugerIkke) {
                *svar.entry((t, sti)).or_default() += r.n;
            }
        }
    }
    let uventet = svar
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|((t, sti), n)| Uventet {
            tek: t.to_string(),
            sti: sti.to_string(),
            n,
        });
    let snit = |t: &str| foer.get(t).copied().unwrap_or(0) as f64 / (SNIT_FRA - SNIT_TIL + 1) as f64;
    Vurdering {
        ikke_brugt,
        uventet,
        naaet: stoerst(
            naaet
                .iter()
                .filter(|(t, _)| valg(t).valg == Valg::Bruger)
                .map(|(t, n)| (t.as_str(), *n)),
        )
        .map(|(t, n)| TekN { tek: t.to_string(), n }),
        besked: i_dag
            .iter()
            .filter(|(t, n)| valg(t).besked && **n >= BESKED_MIN && **n as f64 >= BESKED_FAKTOR * snit(t))
            .map(|(t, n)| TekBesked {
                tek: t.clone(),
                n: *n,
                snit: (snit(t) * 10.0).round() / 10.0,
            })
            .collect(),
        spoerg: stoerst(
            i_dag
                .iter()
                .filter(|(t, _)| valg(t).valg == Valg::VedIkke)
                .map(|(t, n)| (t.as_str(), *n)),
        )
        .map(|(t, _)| t.to_string()),
    }
}

// ── Afsnittet "Dit sites teknologi" ────────────────────────────────────────────────────────────────────────────────
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Linje {
    pub tek: String,
    /// Angreb i de 30 hele døgn (som dyk "Alt" uden rutine) og forsøg, der fik 404.
    pub angreb30: i64,
    pub forsoeg30: i64,
    /// Over hele historikken.
    pub foerst: Option<String>,
    pub sidst: Option<String>,
    pub valg: &'static str,
    pub besked: bool,
    /// Alle `forsoeg30` fik 404, og intet svar fra serveren modsiger det. Kun ved "Ved ikke", og der sættes aldrig noget af sig selv.
    pub forslag: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Generisk {
    pub id: &'static str,
    pub forsoeg30: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
pub struct TeknologiUd {
    pub linjer: Vec<Linje>,
    pub generiske: Vec<Generisk>,
}

#[derive(Default)]
struct Sum<'a> {
    angreb30: i64,
    forsoeg30: i64,
    foerst: Option<&'a str>,
    sidst: Option<&'a str>,
    svar: bool,
}

impl<'a> Sum<'a> {
    fn set(&mut self, dag: &'a str) {
        self.foerst = Some(self.foerst.map_or(dag, |x| x.min(dag)));
        self.sidst = Some(self.sidst.map_or(dag, |x| x.max(dag)));
    }
}

/// Bot-rækkens sti og svarkode: navn, firma, kategori, sti, status (`vaern::doegn_raekker`). Svaret kommer fra edge, så kun 2xx tæller her: en 301 kan være Cloudflares egen.
fn bot_svar(noegle: &str) -> Option<(&str, i64)> {
    let mut f = noegle.split('\t');
    let sti = f.nth(3)?;
    Some((sti, f.next()?.parse().ok()?))
}

/// De teknologier, der er set på domænet eller har et valg, de mest angrebne først, og de generiske slags. Ren funktion.
pub fn ud(rows: &[Raekke], p: &Profil, nu: i64) -> TeknologiUd {
    let (fra, til) = (dag_noegle(nu - HENT_DAGE * DOEGN_MS), dag_noegle(nu - DOEGN_MS));
    let i_vinduet = |d: &str| d >= fra.as_str() && d <= til.as_str();
    let (mut s, mut g): (BTreeMap<String, Sum>, BTreeMap<&str, i64>) = Default::default();
    for r in rows {
        match r.art.as_str() {
            "fw" => {
                let f: Vec<&str> = r.noegle.split('\t').collect();
                let felt = |i: usize| f.get(i).copied().unwrap_or("");
                if let Some(t) = af_fw(felt(0), felt(3), felt(6)) {
                    let x = s.entry(t).or_default();
                    x.set(&r.dag);
                    if i_vinduet(&r.dag) {
                        x.angreb30 += r.n;
                    }
                }
            }
            "probe" => {
                if let Some(t) = af_sti(&r.noegle) {
                    let x = s.entry(t.to_string()).or_default();
                    x.set(&r.dag);
                    if i_vinduet(&r.dag) {
                        x.forsoeg30 += r.n;
                    }
                } else if let Some(k) = forsoeg_kategori(&r.noegle).filter(|k| GENERISKE.contains(k)) {
                    if i_vinduet(&r.dag) {
                        *g.entry(k).or_default() += r.n;
                    }
                }
            }
            _ => {
                let sti = svar_fra_server(r).filter(|x| modsiger_404(x.1)).or_else(|| {
                    (r.art == "bot")
                        .then(|| bot_svar(&r.noegle))
                        .flatten()
                        .filter(|x| (200..300).contains(&x.1))
                });
                if let Some(t) = sti.filter(|_| i_vinduet(&r.dag)).and_then(|x| af_sti(x.0)) {
                    s.entry(t.to_string()).or_default().svar = true;
                }
            }
        }
    }
    for t in p.keys().filter(|t| kendt(t)) {
        s.entry(t.clone()).or_default();
    }
    let mut linjer: Vec<Linje> = s
        .into_iter()
        .filter(|(t, x)| x.foerst.is_some() || p.contains_key(t))
        .map(|(t, x)| {
            let v = p.get(&t).copied().unwrap_or_default();
            Linje {
                tek: t,
                angreb30: x.angreb30,
                forsoeg30: x.forsoeg30,
                foerst: x.foerst.map(str::to_string),
                sidst: x.sidst.map(str::to_string),
                valg: v.valg.kode(),
                besked: v.besked,
                forslag: v.valg == Valg::VedIkke && x.forsoeg30 >= FORSLAG_MIN && !x.svar,
            }
        })
        .collect();
    linjer.sort_by(|a, b| {
        (b.angreb30 + b.forsoeg30)
            .cmp(&(a.angreb30 + a.forsoeg30))
            .then_with(|| a.tek.cmp(&b.tek))
    });
    TeknologiUd {
        linjer,
        generiske: GENERISKE
            .iter()
            .filter_map(|k| g.get(k).map(|n| Generisk { id: k, forsoeg30: *n }))
            .collect(),
    }
}

/// Art, afsnittet læser: reglerne, forsøgene og svarene med en sti.
pub const ARTER: &[&str] = &["fw", "probe", "login", "post", "bot"];

// ── Siden #teknologi ──────────────────────────────────────────────────────────────────────────────────────────────
/// Et valgt domæne: teknologi → valg og "Giv mig besked", og teknologi → angreb i de 30 hele døgn. Står en teknologi ikke i `antal`, er den ikke set på domænet.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct SideDomaene {
    pub zone: String,
    pub navn: String,
    pub valg: BTreeMap<String, (&'static str, bool)>,
    pub antal: BTreeMap<String, i64>,
}

/// En linje i "Alle teknologier". `vis`: kolonne i tabellen. `ny`: set første gang efter, at siden blev åbnet første gang, og ikke kvitteret.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct KatalogLinje {
    pub tek: String,
    pub navn: String,
    pub gruppe: &'static str,
    pub angreb: i64,
    pub set: bool,
    pub vis: bool,
    pub ny: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
pub struct SideUd {
    pub domaener: Vec<SideDomaene>,
    pub katalog: Vec<KatalogLinje>,
}

/// (tek, vis, ny, egen) fra `tek_vis`.
pub type VisRaekke = (String, bool, bool, bool);

/// Hvad der skal gemmes i `tek_vis`: hver set eller valgt teknologi uden række får en. Er tabellen tom, er det første gang, og intet er "nyt". En teknologi, brugeren har fravalgt, har en række og røres ikke. Ren funktion.
pub fn nye_vis(linjer: &[SideDomaene], gemt: &[VisRaekke]) -> Vec<VisRaekke> {
    let kendte: BTreeSet<&str> = gemt.iter().map(|g| g.0.as_str()).collect();
    let foerste_gang = gemt.is_empty();
    let mut ud = BTreeMap::new();
    for d in linjer {
        for t in d.antal.keys().filter(|t| !kendte.contains(t.as_str())) {
            ud.insert(t.clone(), !foerste_gang);
        }
        for t in d.valg.keys().filter(|t| !kendte.contains(t.as_str())) {
            ud.entry(t.clone()).or_insert(false);
        }
    }
    ud.into_iter().map(|(t, ny)| (t, true, ny, false)).collect()
}

/// Kataloget: det faste, alt set på domænerne, alle valg og egne tilføjelser. Sorteret efter gruppe i katalogets orden og derefter navn. Ren funktion.
pub fn katalog(linjer: &[SideDomaene], gemt: &[VisRaekke]) -> Vec<KatalogLinje> {
    let mut alle: BTreeSet<String> = KATALOG.iter().map(|k| k.0.to_string()).collect();
    alle.extend(gemt.iter().map(|g| g.0.clone()));
    for d in linjer {
        alle.extend(d.antal.keys().chain(d.valg.keys()).cloned());
    }
    let mut ud: Vec<KatalogLinje> = alle
        .into_iter()
        .map(|tek| {
            let g = gemt.iter().find(|g| g.0 == tek);
            KatalogLinje {
                angreb: linjer.iter().filter_map(|d| d.antal.get(&tek)).sum(),
                set: linjer.iter().any(|d| d.antal.contains_key(&tek)),
                vis: g.is_some_and(|g| g.1),
                ny: g.is_some_and(|g| g.2),
                navn: navn(&tek),
                gruppe: gruppe(&tek),
                tek,
            }
        })
        .collect();
    ud.sort_by_key(|k| k.navn.to_lowercase());
    ud
}

/// Domænets linje i tabellen.
pub fn side_domaene(zone: &str, navn: &str, rows: &[Raekke], p: &Profil, nu: i64) -> SideDomaene {
    let linjer = ud(rows, p, nu).linjer;
    SideDomaene {
        zone: zone.to_string(),
        navn: navn.to_string(),
        valg: p
            .iter()
            .filter(|(t, v)| kendt(t) && v.valg != Valg::VedIkke)
            .map(|(t, v)| (t.clone(), (v.valg.kode(), v.besked)))
            .collect(),
        antal: linjer
            .into_iter()
            .filter(|l| l.foerst.is_some())
            .map(|l| (l.tek, l.angreb30))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-11-03T08:08:00Z
    const NU: i64 = 1_793_693_280_000;

    fn d(i: i64) -> String {
        dag_noegle(NU - i * DOEGN_MS)
    }

    fn fw(i: i64, regel: &str, sti: &str, kilde: &str, handling: &str, n: i64) -> Raekke {
        Raekke {
            dag: d(i),
            art: "fw".into(),
            noegle: [regel, "US", "AS", sti, "CPH", "GET", kilde, handling].join("\t"),
            n,
        }
    }

    fn art(i: i64, a: &str, noegle: &str, n: i64) -> Raekke {
        Raekke {
            dag: d(i),
            art: a.into(),
            noegle: noegle.into(),
            n,
        }
    }

    fn valg(tek: &str, valg: Valg, besked: bool) -> Profil {
        HashMap::from([(tek.to_string(), TekValg { valg, besked })])
    }

    const WP: &str = "Wordpress - SQL Injection - CVE:CVE-2026-60137";

    #[test]
    fn kortlaegningen() {
        for (regel, tek) in [
            ("React - RCE - CVE:CVE-2025-55182", "react"),
            ("Next.js - Auth Bypass", "nextjs"),
            ("Log4j Headers", "log4j"),
            ("Apache Struts - CVE:CVE-2017-5638", "log4j"),
            (WP, "wordpress"),
            ("Wordpress - PHP Object Injection", "wordpress"),
            ("Joomla - SQLi", "joomla"),
            ("Drupal - RCE", "drupal"),
            ("PHP - Code Injection", "php"),
        ] {
            assert_eq!(af_regel(regel).as_deref(), Some(tek), "{regel}");
        }
        assert_eq!(af_regel("Generic Rules - Anomalous Execution - 6"), None);
        assert_eq!(af_regel("Reactive").as_deref(), Some("react"), "som ordlisten");
        assert_eq!(af_sti("/wp-login.php"), Some("wordpress"));
        assert_eq!(af_sti("/x.php"), Some("php"));
        for generisk in ["/.env", "/.git/config", "/admin", "/wp-config.php"] {
            assert_eq!(af_sti(generisk), None, "{generisk}");
        }
    }

    /// Rigtige regelnavne fra Cloudflares WAF-changelog.
    #[test]
    fn regelnavn_bliver_til_teknologi() {
        for (regel, tek) in [
            ("Magento 2 - Unrestricted file upload", "magento"),
            ("Fortinet FortiCloud SSO - Authentication Bypass - CVE:CVE-2025-59718", "fortinet"),
            ("FortiOS - Auth Bypass - CVE:CVE-2024-55591", "fortinet"),
            ("Ivanti EPMM - Auth Bypass - CVE:CVE-2025-4428, CVE:CVE-2025-4427", "ivanti"),
            ("SonicWall SSLVPN 2 - Auth Bypass - CVE:CVE-2024-53704", "sonicwall"),
            (
                "Citrix Content Collaboration ShareFile - Remote Code Execution - CVE:CVE-2023-24489",
                "citrix",
            ),
            ("Cisco IOS XE - Information Disclosure - CVE:CVE-2023-20198", "cisco"),
            ("F5 Big IP - Remote Code Execution - CVE:CVE-2025-31644", "f5"),
            ("Microsoft SharePoint - Deserialization - CVE:CVE-2025-53770", "sharepoint"),
            ("Apache Tomcat - DoS - CVE:CVE-2025-31650", "tomcat"),
            ("Apache Camel - Code Injection - CVE:CVE-2025-27636", "apache camel"),
            ("Palo Alto Networks - Auth Bypass - CVE:CVE-2025-0108", "palo alto"),
            ("GitLab - Auth Bypass - CVE:CVE-2023-7028", "gitlab"),
            ("Oracle PeopleSoft - Remote Code Execution - CVE:CVE-2023-22047", "oracle"),
            ("Splunk Enterprise - Remote Code Execution - CVE:CVE-2025-20229", "splunk"),
            ("Yeti Platform - Auth Bypass - CVE:CVE-2024-46507", "yeti platform"),
            ("BerriAI - SSRF - CVE:CVE-2024-6587", "berriai"),
            ("Next.js React Server - RCE", "nextjs"),
            ("Apache Struts - CVE:CVE-2017-5638", "log4j"),
            ("Magento - PHP Object Injection", "magento"),
        ] {
            assert_eq!(af_regel(regel).as_deref(), Some(tek), "{regel}");
        }
        for generisk in [
            "Generic Rules - Parameter Pollution - Body",
            "SQLi - UNION - 2 - URI",
            "XSS, HTML Injection - Link Tag - URI",
            "Command Injection - Obfuscation",
            "Anomaly:Header:User-Agent - Fake Bing or MSN Bot",
            "Browser Integrity Check",
            "",
            " - ",
        ] {
            assert_eq!(af_regel(generisk), None, "{generisk}");
        }
        assert_eq!(
            af_regel(&format!("{} - RCE", "x".repeat(50))),
            None,
            "for langt til at være et navn"
        );
        assert_eq!(af_regel("Generic Rules - PHP Code Injection").as_deref(), Some("php"), "som før");
        assert_eq!(navn("yeti platform"), "Yeti Platform");
        assert_eq!(navn("log4j"), "Java/Log4j");
        assert_eq!(gruppe("yeti platform"), "andet");
        assert_eq!(gruppe("fortinet"), "netvaerk");
    }

    /// De syv gamle id'er er uændrede: et gemt valg (php, react, wordpress) betyder det samme.
    #[test]
    fn gamle_valg_beholder_betydningen() {
        for t in TEKNOLOGIER {
            assert!(KATALOG.iter().any(|k| k.0 == t.0 && k.1 == t.2), "{}", t.0);
            assert_eq!(af_regel(&format!("{} - RCE", t.2)).as_deref(), Some(t.0), "{}", t.0);
        }
        for k in KATALOG {
            assert_eq!(id_for(k.1).as_deref(), Some(k.0), "{}", k.1);
            assert!(kendt(k.0), "{}", k.0);
        }
        let sprog = include_str!("../../ui/assets/sprog.js");
        for k in KATALOG {
            assert!(sprog.contains(&format!("'{}'", k.1)), "{} mangler i sprog.js", k.1);
        }
    }

    #[test]
    fn id_og_fri_tekst() {
        assert_eq!(id_for("Wordpress").as_deref(), Some("wordpress"));
        assert_eq!(id_for("  magento ").as_deref(), Some("magento"));
        assert_eq!(id_for("Min Butik").as_deref(), Some("min butik"));
        assert_eq!(
            id_for("Apache Struts").as_deref(),
            Some("log4j"),
            "fri tekst går samme vej som reglerne"
        );
        for nej in ["", "   ", "generic rules", "<script>", "a;b"] {
            assert_eq!(id_for(nej), None, "{nej}");
        }
        assert!(!kendt(ANDET));
    }

    fn sd(zone: &str, antal: &[(&str, i64)], valg: &[&str]) -> SideDomaene {
        SideDomaene {
            zone: zone.into(),
            navn: zone.into(),
            antal: antal.iter().map(|(t, n)| (t.to_string(), *n)).collect(),
            valg: valg.iter().map(|t| (t.to_string(), ("bruger_ikke", false))).collect(),
        }
    }

    /// Første gang er intet "nyt"; bagefter er det, og en fravalgt teknologi kommer ikke tilbage af sig selv.
    #[test]
    fn nye_teknologier_afkrydses_og_meldes() {
        let d = [sd("a", &[("react", 4)], &["php"])];
        let v = |t: &str, ny| (t.to_string(), true, ny, false);
        assert_eq!(nye_vis(&d, &[]), [v("php", false), v("react", false)], "første gang");
        let gemt = [("react".to_string(), false, false, false), ("php".to_string(), true, false, false)];
        assert!(nye_vis(&d, &gemt).is_empty(), "fravalget står");
        let med_ny = [sd("a", &[("react", 4), ("magento", 1)], &[])];
        assert_eq!(nye_vis(&med_ny, &gemt), [v("magento", true)]);
    }

    #[test]
    fn kataloget_har_det_faste_det_sete_og_det_egne() {
        let gemt = [
            ("min butik".to_string(), true, false, true),
            ("react".to_string(), false, false, false),
        ];
        let k = katalog(
            &[sd("a", &[("react", 4), ("yeti platform", 2)], &[]), sd("b", &[("react", 6)], &[])],
            &gemt,
        );
        let f = |t: &str| k.iter().find(|x| x.tek == t).unwrap();
        assert_eq!(
            (f("react").angreb, f("react").set, f("react").vis),
            (10, true, false),
            "skjult, men talt"
        );
        assert!(f("min butik").vis && !f("min butik").set);
        assert_eq!(f("yeti platform").gruppe, "andet");
        assert!(!f("magento").vis && !f("magento").set, "kendt, men ikke set");
        assert!(k.windows(2).all(|w| w[0].navn.to_lowercase() <= w[1].navn.to_lowercase()));
    }

    /// Valg, der er skjult i tabellen, gælder stadig: profilen læser ikke `tek_vis`.
    #[test]
    fn side_domaene_har_valg_og_sete() {
        let rk = [fw(2, WP, "/", "firewallManaged", "block", 7)];
        let d = side_domaene("z", "a.wi24rd.com", &rk, &valg("react", Valg::BrugerIkke, false), NU);
        assert_eq!(d.antal.get("wordpress"), Some(&7));
        assert_eq!(d.antal.get("react"), None, "valgt, men ikke set");
        assert_eq!(d.valg.get("react"), Some(&("bruger_ikke", false)));
    }

    /// Generiske slags, egne regler og rutine kan aldrig holdes ude af status, heller ikke på en WordPress-sti.
    #[test]
    fn generiske_typer_har_intet_valg() {
        let p = valg("wordpress", Valg::BrugerIkke, false);
        assert!(udeladt(&fw(1, WP, "/", "firewallManaged", "block", 1), &p));
        assert!(
            udeladt(&fw(1, "Generic Rules - X", "/wp-login.php", "firewallManaged", "block", 1), &p),
            "stien afgør"
        );
        assert!(!udeladt(&fw(1, "Generic Rules - X", "/.env", "firewallManaged", "block", 1), &p));
        assert!(
            !udeladt(&fw(1, "wp login", "/wp-login.php", "firewallCustom", "block", 1), &p),
            "egen regel"
        );
        assert!(
            !udeladt(&fw(1, "linkMaze", "/wp-login.php", "linkMaze", "link_maze_injected", 1), &p),
            "rutine"
        );
        for g in GENERISKE {
            assert!(!kendt(g), "{g} kan ikke vælges");
        }
        let mut l = Store::i_hukommelse().unwrap();
        assert!(saet(&mut l, "z", "hemmeligheder", Valg::BrugerIkke, false, "t").is_err());
        let u = ud(
            &[art(1, "probe", "/.env", 4), art(1, "probe", "/.git/config", 2)],
            &Profil::new(),
            NU,
        );
        assert!(u.linjer.is_empty());
        assert_eq!(
            u.generiske,
            [
                Generisk {
                    id: "hemmeligheder",
                    forsoeg30: 4
                },
                Generisk {
                    id: "kildekode",
                    forsoeg30: 2
                }
            ]
        );
    }

    #[test]
    fn sikkerhedsnettet_ser_kun_svar_2xx_og_302_fra_serveren() {
        let p = valg("wordpress", Valg::BrugerIkke, false);
        let login = |o: &str| art(1, "login", &["/wp-login.php", "POST", "200", o, "US", "AS1"].join("\t"), 3);
        let post = |o: &str| art(1, "post", &["/xmlrpc.php", "200", o, "US", "AS1"].join("\t"), 2);
        let u = |r: Raekke| vurder(&[r], &p, NU).uventet;
        assert_eq!(
            u(login("200")),
            Some(Uventet {
                tek: "wordpress".into(),
                sti: "/wp-login.php".into(),
                n: 3
            })
        );
        assert!(u(login("302")).is_some());
        assert!(u(post("204")).is_some(), "post har origin i felt 3");
        for o in ["199", "300", "301", "303", "308", "400", "404", "500", "0", ""] {
            assert_eq!(u(login(o)), None, "origin {o}");
        }
        assert_eq!(vurder(&[login("200")], &Profil::new(), NU).uventet, None, "kun ved Bruger vi ikke");
        let mut gammel = login("200");
        gammel.dag = d(2);
        assert_eq!(u(gammel), None, "kun statusdøgnet");
    }

    #[test]
    fn bruger_vi_og_naaet_serveren() {
        let p = valg("wordpress", Valg::Bruger, false);
        let rk = [
            fw(1, WP, "/", "firewallManaged", "log", 4),
            fw(1, WP, "/", "firewallManaged", "block", 9),
            fw(1, "linkMaze", "/", "linkMaze", "log", 9),
        ];
        assert_eq!(
            vurder(&rk, &p, NU).naaet,
            Some(TekN {
                tek: "wordpress".into(),
                n: 4
            }),
            "kun det, Cloudflare ikke stoppede"
        );
        assert_eq!(vurder(&rk[1..], &p, NU).naaet, None, "alt stoppet");
        assert_eq!(vurder(&rk, &Profil::new(), NU).naaet, None, "Ved ikke");
        assert_eq!(vurder(&rk, &valg("wordpress", Valg::BrugerIkke, false), NU).naaet, None);
    }

    #[test]
    fn besked_reglen() {
        let p = valg("wordpress", Valg::Bruger, true);
        let med = |n: i64, foer: i64| -> Vec<Raekke> {
            let mut v = vec![fw(1, WP, "/", "firewallManaged", "block", n)];
            v.extend((2..=8).map(|i| fw(i, WP, "/", "firewallManaged", "block", foer)));
            v
        };
        assert_eq!(vurder(&med(6, 2), &p, NU).besked[0].snit, 2.0);
        assert_eq!(vurder(&med(6, 2), &p, NU).besked.len(), 1, "præcis 3 × snit");
        assert!(vurder(&med(5, 2), &p, NU).besked.is_empty(), "under 3 × snit");
        assert_eq!(vurder(&med(5, 0), &p, NU).besked.len(), 1, "præcis 5");
        assert!(vurder(&med(4, 0), &p, NU).besked.is_empty(), "under 5");
        assert!(
            vurder(&med(50, 0), &valg("wordpress", Valg::Bruger, false), NU).besked.is_empty(),
            "ikke bedt om"
        );
    }

    #[test]
    fn forslag_kraever_kun_404_og_saetter_intet() {
        let probe = |n: i64| art(3, "probe", "/wp-login.php", n);
        let forslag = |rk: &[Raekke], p: &Profil| ud(rk, p, NU).linjer.first().map(|l| (l.forsoeg30, l.forslag));
        assert_eq!(forslag(&[probe(312)], &Profil::new()), Some((312, true)));
        assert_eq!(forslag(&[probe(19)], &Profil::new()), Some((19, false)), "for få");
        let svar = art(5, "login", &["/wp-login.php", "POST", "200", "200", "US", ""].join("\t"), 1);
        assert_eq!(forslag(&[probe(312), svar], &Profil::new()), Some((312, false)), "serveren svarede");
        let flyttet = art(5, "post", &["/xmlrpc.php", "301", "301", "US", ""].join("\t"), 1);
        assert_eq!(
            forslag(&[probe(312), flyttet], &Profil::new()),
            Some((312, false)),
            "ikke alle fik 404"
        );
        let bot = art(5, "bot", &["Googlebot", "Google", "Search", "/wp-login.php", "200"].join("\t"), 1);
        assert_eq!(
            forslag(&[probe(312), bot], &Profil::new()),
            Some((312, false)),
            "en robot fik siden"
        );
        let p = valg("wordpress", Valg::BrugerIkke, false);
        assert_eq!(forslag(&[probe(312)], &p), Some((312, false)), "der er taget stilling");
        let l = &ud(&[probe(312)], &Profil::new(), NU).linjer[0];
        assert_eq!(l.valg, "ved_ikke", "et forslag sætter intet");
    }

    #[test]
    fn afsnittet_taeller_30_hele_doegn_og_husker_valg_uden_data() {
        let rk = [
            fw(0, WP, "/", "firewallManaged", "block", 100),
            fw(2, WP, "/", "firewallManaged", "block", 7),
            fw(31, WP, "/", "firewallManaged", "block", 50),
            fw(2, "React - RCE", "/", "firewallManaged", "block", 9),
        ];
        let u = ud(&rk, &valg("drupal", Valg::Bruger, false), NU);
        assert_eq!(
            u.linjer.iter().map(|l| (l.tek.as_str(), l.angreb30)).collect::<Vec<_>>(),
            [("react", 9), ("wordpress", 7), ("drupal", 0)]
        );
        assert_eq!((u.linjer[1].foerst.clone(), u.linjer[1].sidst.clone()), (Some(d(31)), Some(d(0))));
    }

    #[test]
    fn spoerg_er_den_stoerste_uden_valg() {
        let rk = [
            fw(1, WP, "/", "firewallManaged", "block", 3),
            fw(1, "React - RCE", "/", "firewallManaged", "block", 9),
        ];
        assert_eq!(vurder(&rk, &Profil::new(), NU).spoerg.as_deref(), Some("react"));
        assert_eq!(
            vurder(&rk, &valg("react", Valg::BrugerIkke, false), NU).spoerg.as_deref(),
            Some("wordpress")
        );
    }
}
