//! Login: hvad der skete på domænets login-stier. Tre kilder, der aldrig lægges sammen, for om det er samme forespørgsel, kan
//! ikke afgøres: stoppet ved Edge (`fw`), fik 404 (`probe`) og nåede din server (`login`, kun rækker med origin-status).
//! Svar 200 og 302 siger ikke, at nogen kom ind: det kan lige så godt være en fejlside, og teksten skal sige det. Stierne vælger
//! brugeren pr. domæne (J2): en fast startliste plus forslag fra data, aldrig et gæt på teknologi. Ingen IP-adresser.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use regex::{Regex, RegexSet};
use serde::{Deserialize, Serialize};

use crate::adgang::LAEK_IKKE_TJEKKET;
use crate::cf::Fejl;
use crate::dyk::{periode_ud, spaend, top_af, Periode, PeriodeUd, Spoergsmaal, VaerdiN, MAKS_VAERDI};
use crate::overblik::{gruppe, BLOKERET, LAEK_KODEORD, RUTINE, UDFORDRET};
use crate::robotter::{dage_i, mangler_dage, uden_ekstra};
use crate::store::{Raekke, Store};
use crate::teknologi::er_svar;

/// Stierne, brugeren kan slå til med ét klik (J2).
pub const STARTLISTE: &[&str] = &[
    "/wp-login.php",
    "/wp-admin/",
    "/xmlrpc.php",
    "/admin/",
    "/administrator/",
    "/user/login",
    "/login",
    "/signin",
];
/// Højst så mange egne stier pr. domæne, ud over startlisten.
pub const MAKS_EGNE_STIER: usize = 20;
const ARTER: &[&str] = &["fw", "probe", "login", "asn"];
/// Et forslag kræver POST med et af disse svar på så mange forskellige dage.
const FORSLAG_DAGE: usize = 3;
const FORSLAG_STATUS: &[&str] = &["200", "302", "401", "403"];
const FORSLAG_MAKS: usize = 8;
/// Ord for at logge ind (ikke ud). En sti ligner et login, når et af dem står som helt ord, også i flertal og med et tal efter, uanset store og små bogstaver: `/login.php` og `/wp-login.php`, men ikke `/badminton`. En omdøbt
/// login-side findes i stedet på, at lækket-tjekket har set et kodeord (`forslag`).
pub const LOGIN_ORD: &[&str] = &[
    "login",
    "signin",
    "sign-in",
    "logon",
    "auth",
    "authenticate",
    "authorize",
    "oauth",
    "token",
    "sso",
    "saml",
    "2fa",
    "mfa",
    "otp",
    "password",
    "passwd",
    "reset",
    "session",
    "account",
    "admin",
    "administrator",
    "user",
];
static LOGIN_MOENSTER: LazyLock<Regex> = LazyLock::new(|| {
    let ord: Vec<String> = LOGIN_ORD.iter().map(|o| regex::escape(o)).collect();
    Regex::new(&format!(r"(^|[^\p{{L}}\p{{N}}])({})s?\d*($|[^\p{{L}}\p{{N}}])", ord.join("|"))).expect("mønsteret for login-ordene")
});
/// Stier, der ligner et login på navnet, men ikke er det (mønstre på stien i små bogstaver): WordPress' AJAX- og formularindgang, forfatterarkivet, hvor brugernavnet er en del af stien, og REST-ruterne
/// for brugere. Gælder kun forslag fra navnet: en sti, hvor tjekket har set et kodeord, kan stadig foreslås (`forslag`).
pub const IKKE_LOGIN: &[&str] = &[
    r"/wp-admin/admin-ajax\.php$",
    r"/wp-admin/admin-post\.php$",
    r"(^|/)authors?(/|$)",
    r"/wp-json/(.*/)?users(/|$)",
];
static IKKE_LOGIN_MOENSTRE: LazyLock<RegexSet> =
    LazyLock::new(|| RegexSet::new(IKKE_LOGIN).expect("mønstrene for det, der ikke er et login"));

/// En sti: `/`, højst `MAKS_VAERDI` tegn, uden mellemrum og styretegn, og uden forespørgsel (`clientRequestPath` har ingen).
pub fn tjek_sti(sti: &str) -> Result<String, Fejl> {
    let ok = sti.starts_with('/')
        && sti.chars().count() <= MAKS_VAERDI
        && !sti.contains('?')
        && !sti.chars().any(|c| c.is_control() || c.is_whitespace());
    if ok {
        Ok(sti.to_string())
    } else {
        Err(Fejl::UgyldigtFilter)
    }
}

/// Slår en sti til eller fra, eller tilføjer en egen sti. En egen sti, der slås til, er ny i `clientRequestPath_in`, så domænets
/// ekstra døgn hentes igen i baggrunden; startlisten er altid med i forespørgslen og har allerede sin historik.
pub fn saet_sti(lager: &mut Store, zone: &str, sti: &str, til: bool) -> Result<(), Fejl> {
    let sti = tjek_sti(sti)?;
    let start = STARTLISTE.contains(&sti.as_str());
    if !start {
        let gemte = lager.login_stier(zone)?;
        let egne = gemte.iter().filter(|(s, _)| !STARTLISTE.contains(&s.as_str())).count();
        if egne >= MAKS_EGNE_STIER && !gemte.iter().any(|(s, _)| *s == sti) {
            return Err(Fejl::ForMangeStier);
        }
    }
    lager.gem_login_sti(zone, &sti, til)?;
    if til && !start {
        lager.glem_hentet_ekstra(zone)?;
    }
    Ok(())
}

/// Fjerner en egen sti. Startlistens stier kan kun slås fra. Tallene bliver, men stien tæller ikke længere med i Login.
pub fn fjern_sti(lager: &mut Store, zone: &str, sti: &str) -> Result<(), Fejl> {
    let sti = tjek_sti(sti)?;
    if STARTLISTE.contains(&sti.as_str()) {
        return Err(Fejl::UgyldigtFilter);
    }
    lager.slet_login_sti(zone, &sti)
}

/// Hvad et svar fra din server betyder. Klarsproget står i webvisningen; 200 og 302 er `svar`, ikke et login.
pub fn slags(origin: i64) -> &'static str {
    match origin {
        401 | 403 => AFVIST,
        200 | 302 => SVAR,
        _ => ANDET,
    }
}

pub const EDGE: &str = "edge";
pub const IKKE_FUNDET: &str = "ikke_fundet";
pub const SERVER: &str = "server";
pub const AFVIST: &str = "afvist";
pub const SVAR: &str = "svar";
pub const ANDET: &str = "andet";

/// Filtrene i `#login`.
#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Filter {
    #[serde(default)]
    pub udfald: Option<String>,
    #[serde(default)]
    pub sti: Option<String>,
    #[serde(default)]
    pub metode: Option<String>,
    #[serde(default)]
    pub land: Option<String>,
    #[serde(default)]
    pub netvaerk: Option<String>,
}

pub fn tjek(f: Filter) -> Result<Filter, Fejl> {
    let kendt = f
        .udfald
        .as_deref()
        .is_none_or(|u| [EDGE, IKKE_FUNDET, SERVER, AFVIST, SVAR].contains(&u));
    let korte = [&f.sti, &f.metode, &f.land, &f.netvaerk]
        .iter()
        .all(|v| v.as_ref().is_none_or(|x| !x.is_empty() && x.chars().count() <= MAKS_VAERDI));
    if kendt && korte {
        Ok(f)
    } else {
        Err(Fejl::UgyldigtFilter)
    }
}

/// Tre kilder og fordelingen af den tredje: `afvist` + `svar` + `andet` = `server`. Hverken `edge` eller `ikke_fundet` indgår.
#[derive(Serialize, Default, Debug, Clone, PartialEq)]
pub struct Tal {
    pub edge: i64,
    pub ikke_fundet: i64,
    pub server: i64,
    pub afvist: i64,
    pub svar: i64,
    pub andet: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DagUd {
    pub dag: String,
    pub edge: i64,
    pub ikke_fundet: i64,
    pub server: i64,
    pub hentet: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct StiUd {
    pub sti: String,
    /// Mest brugte metode først.
    pub metoder: Vec<String>,
    pub tal: Tal,
}

/// Login-forsøg med et kodeord fra et kendt læk pr. dag og sti: alle, og dem, serveren svarede 2xx eller 302 på. Domænet kun, når flere er med.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct LaekUd {
    pub dag: String,
    pub sti: String,
    pub n: i64,
    pub naaet: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct LoginUd {
    pub periode: PeriodeUd,
    pub laek: Vec<LaekUd>,
    /// Det øvrige fra tjekket: Cloudflares andre svar og `ukendt`. Tæller ikke i status.
    pub laek_andet: Vec<VaerdiN>,
    /// Døgn i perioden, hvor tjekket er hentet.
    pub laek_dage: usize,
    /// Stierne, der er slået til (ét domæne), eller summen af dem for de valgte.
    pub stier: Vec<String>,
    pub tal: Tal,
    pub dage: Vec<DagUd>,
    pub raekker: Vec<StiUd>,
    pub metoder_edge: Vec<VaerdiN>,
    pub metoder_server: Vec<VaerdiN>,
    pub lande_edge: Vec<VaerdiN>,
    pub lande_server: Vec<VaerdiN>,
    pub net_edge: Vec<VaerdiN>,
    pub net_server: Vec<VaerdiN>,
    pub mangler_dage: usize,
    pub ikke_tilgaengelig: Vec<String>,
}

/// Det, `regn` skal bruge fra databasen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Laest {
    pub rows: Vec<(String, Raekke)>,
    pub laek: Vec<Raekke>,
    pub laek_hentet: HashSet<String>,
    pub stier: HashMap<String, HashSet<String>>,
    pub hentet: HashSet<String>,
    pub ekstra: HashMap<String, HashSet<String>>,
    pub ikke_tilgaengelig: Vec<String>,
}

pub fn laes(lager: &Store, s: &Spoergsmaal, _nu: i64) -> Result<Laest, Fejl> {
    let (mut hentet, mut ekstra, mut stier) = (HashSet::new(), HashMap::new(), HashMap::new());
    let mut laek_hentet = HashSet::new();
    for z in &s.zoner {
        hentet.extend(lager.hentet(z)?.into_keys());
        laek_hentet.extend(lager.hentet_laek(z)?.into_keys());
        ekstra.insert(z.clone(), lager.hentet_ekstra(z)?.into_keys().collect());
        let til = lager.login_stier(z)?.into_iter().filter(|x| x.1).map(|x| x.0).collect();
        stier.insert(z.clone(), til);
    }
    Ok(Laest {
        rows: lager.raekker_art(&s.zoner, ARTER, "0000-00-00")?,
        laek: lager
            .raekker_art(&s.zoner, &["laek"], "0000-00-00")?
            .into_iter()
            .map(|x| x.1)
            .collect(),
        laek_hentet,
        stier,
        hentet,
        ekstra,
        ikke_tilgaengelig: uden_ekstra(lager, &s.zoner)?,
    })
}

/// Hvilken kilde en hændelse kommer fra.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kilde {
    Edge,
    IkkeFundet,
    /// Med origin-status.
    Server(i64),
}

/// Én talt række i fælles form. `land`, `net` og `metode` er tomme for `probe`, som ikke har dem.
struct Haendelse<'a> {
    dag: &'a str,
    sti: &'a str,
    metode: &'a str,
    land: &'a str,
    net: String,
    kilde: Kilde,
    n: i64,
}

fn asn_navn(asn: &str, navne: &HashMap<&str, &str>) -> String {
    match (asn, navne.get(asn)) {
        ("", _) => String::new(),
        (_, Some(n)) => n.to_string(),
        (a, None) => format!("AS{a}"),
    }
}

/// `fw`, `probe` og `login` for de stier, der er slået til. Fra `fw` tæller kun blokeret og udfordret: en side med fælde-links er
/// ikke et forsøg, og labyrinten er ikke en mitigering. En `login`-række uden origin-status nåede ikke serveren.
fn haendelser<'a>(l: &'a Laest, navne: &HashMap<&'a str, &'a str>) -> Vec<Haendelse<'a>> {
    let mut ud = Vec::new();
    for (z, r) in &l.rows {
        let Some(stier) = l.stier.get(z) else { continue };
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        let dag = r.dag.as_str();
        match r.art.as_str() {
            "fw" if stier.contains(felt(3)) && matches!(gruppe(felt(7)), BLOKERET | UDFORDRET) => ud.push(Haendelse {
                dag,
                sti: felt(3),
                metode: felt(5),
                land: felt(1),
                net: felt(2).to_string(),
                kilde: Kilde::Edge,
                n: r.n,
            }),
            "probe" if stier.contains(r.noegle.as_str()) => ud.push(Haendelse {
                dag,
                sti: &r.noegle,
                metode: "",
                land: "",
                net: String::new(),
                kilde: Kilde::IkkeFundet,
                n: r.n,
            }),
            "login" if stier.contains(felt(0)) => {
                let origin: i64 = felt(3).parse().unwrap_or(0);
                if origin > 0 {
                    ud.push(Haendelse {
                        dag,
                        sti: felt(0),
                        metode: felt(1),
                        land: felt(4),
                        net: asn_navn(felt(5), navne),
                        kilde: Kilde::Server(origin),
                        n: r.n,
                    });
                }
            }
            _ => {}
        }
    }
    ud
}

impl Tal {
    fn tael(&mut self, k: Kilde, n: i64) {
        match k {
            Kilde::Edge => self.edge += n,
            Kilde::IkkeFundet => self.ikke_fundet += n,
            Kilde::Server(o) => {
                self.server += n;
                match slags(o) {
                    AFVIST => self.afvist += n,
                    SVAR => self.svar += n,
                    _ => self.andet += n,
                }
            }
        }
    }
}

/// Passer kilden til filteret `udfald`?
fn udfald_passer(u: Option<&str>, k: Kilde) -> bool {
    match (u, k) {
        (None, _) | (Some(EDGE), Kilde::Edge) | (Some(IKKE_FUNDET), Kilde::IkkeFundet) | (Some(SERVER), Kilde::Server(_)) => true,
        (Some(x), Kilde::Server(o)) => x == slags(o),
        _ => false,
    }
}

/// Svaret ud fra det læste. Fliserne står altid uden `udfald`, så de kan vælges imellem; alt andet følger filteret. Ren funktion.
pub fn regn(periode: &Periode, f: &Filter, l: &Laest, nu: i64) -> LoginUd {
    let (fra, til) = spaend(periode, &l.hentet, nu);
    let dage_liste = dage_i(&fra, &til);
    let navne: HashMap<&str, &str> = l
        .rows
        .iter()
        .filter(|(_, r)| r.art == "asn")
        .filter_map(|(_, r)| r.noegle.split_once('\t'))
        .collect();
    let (mut tal, mut per_dag) = (Tal::default(), HashMap::<&str, Tal>::new());
    let mut per_sti: BTreeMap<&str, (Tal, HashMap<&str, i64>)> = BTreeMap::new();
    let (mut m_edge, mut m_server) = (HashMap::<&str, i64>::new(), HashMap::<&str, i64>::new());
    let (mut l_edge, mut l_server) = (HashMap::<&str, i64>::new(), HashMap::<&str, i64>::new());
    let (mut n_edge, mut n_server) = (HashMap::<String, i64>::new(), HashMap::<String, i64>::new());
    let alle = haendelser(l, &navne);
    let sted = f.land.is_some() || f.netvaerk.is_some() || f.metode.is_some();
    for h in &alle {
        if h.dag < fra.as_str() || h.dag > til.as_str() || f.sti.as_deref().is_some_and(|s| s != h.sti) {
            continue;
        }
        // `probe` kender hverken land, netværk eller metode, så med de filtre står den ikke.
        let forkert = |v: &Option<String>, x: &str| v.as_deref().is_some_and(|y| y != x);
        if sted
            && (h.kilde == Kilde::IkkeFundet || forkert(&f.land, h.land) || forkert(&f.netvaerk, &h.net) || forkert(&f.metode, h.metode))
        {
            continue;
        }
        tal.tael(h.kilde, h.n);
        if !udfald_passer(f.udfald.as_deref(), h.kilde) {
            continue;
        }
        per_dag.entry(h.dag).or_default().tael(h.kilde, h.n);
        let s = per_sti.entry(h.sti).or_default();
        s.0.tael(h.kilde, h.n);
        if !h.metode.is_empty() {
            *s.1.entry(h.metode).or_default() += h.n;
        }
        let (m, la, ne) = match h.kilde {
            Kilde::Edge => (&mut m_edge, &mut l_edge, &mut n_edge),
            Kilde::Server(_) => (&mut m_server, &mut l_server, &mut n_server),
            Kilde::IkkeFundet => continue,
        };
        *m.entry(h.metode).or_default() += h.n;
        *la.entry(h.land).or_default() += h.n;
        if !h.net.is_empty() {
            *ne.entry(h.net.clone()).or_default() += h.n;
        }
    }
    let top_net = |t: &HashMap<String, i64>| top_af(&t.iter().map(|(k, n)| (k.as_str(), *n)).collect());
    let mut stier: Vec<String> = l.stier.values().flatten().cloned().collect::<HashSet<_>>().into_iter().collect();
    stier.sort();
    let mut raekker: Vec<StiUd> = per_sti
        .into_iter()
        .map(|(sti, (tal, m))| {
            let mut metoder: Vec<(&str, i64)> = m.into_iter().collect();
            metoder.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
            StiUd {
                sti: sti.to_string(),
                metoder: metoder.into_iter().map(|x| x.0.to_string()).collect(),
                tal,
            }
        })
        .collect();
    raekker.sort_by_key(|r| std::cmp::Reverse(r.tal.edge.max(r.tal.ikke_fundet).max(r.tal.server)));
    let (mut laek, mut andet) = (BTreeMap::<(&str, &str), (i64, i64)>::new(), HashMap::<&str, i64>::new());
    for r in l.laek.iter().filter(|r| r.dag >= fra && r.dag <= til) {
        let d: Vec<&str> = r.noegle.split('\t').collect();
        let (sti, resultat, origin) = (
            d.first().copied().unwrap_or(""),
            d.get(1).copied().unwrap_or(""),
            d.get(2).copied().unwrap_or(""),
        );
        if f.sti.as_deref().is_some_and(|s| s != sti) {
            continue;
        }
        if LAEK_KODEORD.contains(&resultat) {
            let x = laek.entry((&r.dag, sti)).or_default();
            x.0 += r.n;
            if origin.parse().is_ok_and(er_svar) {
                x.1 += r.n;
            }
        } else {
            *andet.entry(resultat).or_default() += r.n;
        }
    }
    let mut laek: Vec<LaekUd> = laek
        .into_iter()
        .map(|((dag, sti), (n, naaet))| LaekUd {
            dag: dag.to_string(),
            sti: sti.to_string(),
            n,
            naaet,
        })
        .collect();
    laek.sort_by(|a, b| b.dag.cmp(&a.dag).then_with(|| b.n.cmp(&a.n)));
    LoginUd {
        periode: periode_ud(&fra, &til, &l.hentet, nu),
        laek,
        laek_andet: top_af(&andet),
        laek_dage: dage_liste.iter().filter(|d| l.laek_hentet.contains(*d)).count(),
        stier,
        tal,
        dage: dage_liste
            .iter()
            .map(|d| {
                let t = per_dag.get(d.as_str()).cloned().unwrap_or_default();
                DagUd {
                    dag: d.clone(),
                    edge: t.edge,
                    ikke_fundet: t.ikke_fundet,
                    server: t.server,
                    hentet: l.hentet.contains(d),
                }
            })
            .collect(),
        raekker,
        metoder_edge: top_af(&m_edge),
        metoder_server: top_af(&m_server),
        lande_edge: top_af(&l_edge),
        lande_server: top_af(&l_server),
        net_edge: top_net(&n_edge),
        net_server: top_net(&n_server),
        mangler_dage: mangler_dage(&dage_liste, &l.ekstra, nu),
        ikke_tilgaengelig: l.ikke_tilgaengelig.clone(),
    }
}

/// Ligner stien et login på navnet? Ren funktion.
fn ligner_login(sti: &str) -> bool {
    let sti = sti.to_lowercase();
    !IKKE_LOGIN_MOENSTRE.is_match(&sti) && LOGIN_MOENSTER.is_match(&sti)
}

/// Stier, hvor lækket-tjekket har set et kodeord: alt andet end `none` og det ukendte. Ren funktion.
fn kodeord_set(rows: &[Raekke]) -> HashSet<&str> {
    let mut ud = HashSet::new();
    for r in rows {
        if r.art == "laek_ren" {
            ud.insert(r.noegle.as_str());
        } else if r.art == "laek" {
            let mut f = r.noegle.split('\t');
            let (sti, resultat) = (f.next().unwrap_or(""), f.next().unwrap_or(""));
            if !sti.is_empty() && resultat != LAEK_IKKE_TJEKKET && resultat != "ukendt" {
                ud.insert(sti);
            }
        }
    }
    ud
}

/// Forslag til login-stier: POST med 200, 302, 401 eller 403 på mindst `FORSLAG_DAGE` forskellige dage, som ikke mest svarer 404, og som enten ligner et login (`LOGIN_ORD`) eller har fået et kodeord tjekket af
/// Cloudflare. Forsiden er aldrig et login, og en sti, der optræder i hændelser fra en kendt regel, er et mål for angreb, hvad den end hedder. Ingen gæt på teknologi. Ren funktion.
pub fn forslag(rows: &[Raekke], kendte: &HashSet<&str>) -> Vec<String> {
    let (mut dage, mut alle, mut ikke_fundet) = (
        HashMap::<&str, HashSet<&str>>::new(),
        HashMap::<&str, i64>::new(),
        HashMap::<&str, i64>::new(),
    );
    let mut angrebet = HashSet::<&str>::new();
    for r in rows.iter().filter(|r| r.art == "fw") {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let felt = |i: usize| f.get(i).copied().unwrap_or("");
        if !RUTINE.contains(&felt(6)) {
            angrebet.insert(felt(3));
        }
    }
    let kodeord = kodeord_set(rows);
    for r in rows.iter().filter(|r| r.art == "post") {
        let f: Vec<&str> = r.noegle.split('\t').collect();
        let (sti, status) = (f.first().copied().unwrap_or(""), f.get(1).copied().unwrap_or(""));
        *alle.entry(sti).or_default() += r.n;
        if status == "404" {
            *ikke_fundet.entry(sti).or_default() += r.n;
        }
        if FORSLAG_STATUS.contains(&status) {
            dage.entry(sti).or_default().insert(r.dag.as_str());
        }
    }
    let mut ud: Vec<(&str, i64)> = dage
        .into_iter()
        .filter(|(sti, d)| {
            let (n, ikke) = (alle[sti], ikke_fundet.get(sti).copied().unwrap_or(0));
            let login = ligner_login(sti) || kodeord.contains(sti);
            d.len() >= FORSLAG_DAGE
                && ikke * 2 <= n
                && login
                && !angrebet.contains(sti)
                && *sti != "/"
                && !kendte.contains(sti)
                && tjek_sti(sti).is_ok()
        })
        .map(|(sti, _)| (sti, alle[sti]))
        .collect();
    ud.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    ud.into_iter().take(FORSLAG_MAKS).map(|x| x.0.to_string()).collect()
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct StiValg {
    pub sti: String,
    pub til: bool,
    /// Står på startlisten (en egen sti kan slås fra, men forsvinder ikke).
    pub start: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct StierUd {
    pub stier: Vec<StiValg>,
    pub forslag: Vec<String>,
}

/// Indstillingen for ét domæne: startlisten først, så egne stier, og forslag fra data.
pub fn stier(lager: &Store, zone: &str) -> Result<StierUd, Fejl> {
    let gemte = lager.login_stier(zone)?;
    let til = |s: &str| gemte.iter().any(|(x, t)| x == s && *t);
    let mut ud: Vec<StiValg> = STARTLISTE
        .iter()
        .map(|s| StiValg {
            sti: s.to_string(),
            til: til(s),
            start: true,
        })
        .collect();
    for (sti, t) in &gemte {
        if !STARTLISTE.contains(&sti.as_str()) {
            ud.push(StiValg {
                sti: sti.clone(),
                til: *t,
                start: false,
            });
        }
    }
    let rows: Vec<Raekke> = lager
        .raekker_art(&[zone.to_string()], &["post", "fw", "laek", "laek_ren"], "0000-00-00")?
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    let kendte: HashSet<&str> = ud.iter().map(|s| s.sti.as_str()).collect();
    let forslag = forslag(&rows, &kendte);
    Ok(StierUd { stier: ud, forslag })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaern::{dag_noegle, DOEGN_MS};

    fn med_ekstra() -> Store {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_ekstra("z1", "2026-11-02", &[], "t").unwrap();
        s
    }

    #[test]
    fn egen_sti_gemmes_og_hentes_igen() {
        let mut s = med_ekstra();
        saet_sti(&mut s, "z1", "/min-login/", true).unwrap();
        assert_eq!(s.login_stier("z1").unwrap(), vec![("/min-login/".to_string(), true)]);
        assert!(s.hentet_ekstra("z1").unwrap().is_empty(), "de 30 dage skal hentes igen");
        assert!(stier(&s, "z1")
            .unwrap()
            .stier
            .iter()
            .any(|x| x.sti == "/min-login/" && x.til && !x.start));
    }

    #[test]
    fn ugyldig_egen_sti_afvises_og_gemmes_ikke() {
        let mut s = med_ekstra();
        for nej in ["", "min-login", "/min login/", "/a?b=1", "/a\tb"] {
            assert_eq!(saet_sti(&mut s, "z1", nej, true), Err(Fejl::UgyldigtFilter), "{nej:?}");
        }
        assert!(s.login_stier("z1").unwrap().is_empty());
        assert_eq!(s.hentet_ekstra("z1").unwrap().len(), 1, "intet skal hentes igen");
    }

    #[test]
    fn slaas_en_sti_fra_eller_er_den_paa_startlisten_hentes_intet_igen() {
        let mut s = med_ekstra();
        saet_sti(&mut s, "z1", "/wp-login.php", true).unwrap();
        saet_sti(&mut s, "z1", "/min-login/", false).unwrap();
        assert_eq!(s.hentet_ekstra("z1").unwrap().len(), 1);
        saet_sti(&mut s, "z1", "/min-login/", true).unwrap();
        assert!(
            s.hentet_ekstra("z1").unwrap().is_empty(),
            "slået til igen: den var ikke med i forespørgslen"
        );
    }

    #[test]
    fn hoejst_tyve_egne_stier_pr_domaene() {
        let mut s = Store::i_hukommelse().unwrap();
        saet_sti(&mut s, "z1", "/login", true).unwrap();
        for i in 0..MAKS_EGNE_STIER {
            saet_sti(&mut s, "z1", &format!("/s{i}"), true).unwrap();
        }
        assert_eq!(saet_sti(&mut s, "z1", "/en-for-meget", true), Err(Fejl::ForMangeStier));
        saet_sti(&mut s, "z1", "/s3", false).unwrap();
        saet_sti(&mut s, "z1", "/admin/", true).unwrap();
        saet_sti(&mut s, "z2", "/andet-domaene", true).unwrap();
        fjern_sti(&mut s, "z1", "/s3").unwrap();
        saet_sti(&mut s, "z1", "/nu-er-der-plads", true).unwrap();
    }

    #[test]
    fn kun_egne_stier_kan_fjernes() {
        let mut s = med_ekstra();
        saet_sti(&mut s, "z1", "/wp-login.php", true).unwrap();
        saet_sti(&mut s, "z1", "/min-login/", true).unwrap();
        assert_eq!(fjern_sti(&mut s, "z1", "/wp-login.php"), Err(Fejl::UgyldigtFilter));
        fjern_sti(&mut s, "z1", "/min-login/").unwrap();
        assert_eq!(s.login_stier("z1").unwrap(), vec![("/wp-login.php".to_string(), true)]);
        assert!(!stier(&s, "z1").unwrap().stier.iter().any(|x| x.sti == "/min-login/"));
    }

    /// 2026-11-04T08:00:00Z.
    const NU: i64 = 1_793_779_200_000;

    fn d(i: i64) -> String {
        dag_noegle(NU - i * DOEGN_MS)
    }

    fn rk(dag: &str, art: &str, noegle: &str, n: i64) -> (String, Raekke) {
        (
            "z1".into(),
            Raekke {
                dag: dag.into(),
                art: art.into(),
                noegle: noegle.into(),
                n,
            },
        )
    }

    fn fw(dag: &str, sti: &str, land: &str, net: &str, handling: &str, n: i64) -> (String, Raekke) {
        rk(
            dag,
            "fw",
            &["R", land, net, sti, "WAW", "POST", "firewallManaged", handling].join("\t"),
            n,
        )
    }

    fn login(dag: &str, sti: &str, status: i64, origin: i64, land: &str, asn: &str, n: i64) -> (String, Raekke) {
        rk(
            dag,
            "login",
            &[sti, "POST", &status.to_string(), &origin.to_string(), land, asn].join("\t"),
            n,
        )
    }

    /// Pr. dag og sti, med det, serveren svarede 2xx eller 302 på. Resten står for sig og tæller ikke.
    #[test]
    fn laekkede_kodeord_pr_dag_og_sti() {
        let r = |dag: &str, noegle: &str, n| Raekke {
            dag: dag.into(),
            art: "laek".into(),
            noegle: noegle.into(),
            n,
        };
        let mut l = laest();
        l.laek = vec![
            r(&d(1), "/login\tpassword_leaked\t200", 2),
            r(&d(1), "/login\tpassword_leaked\t403", 3),
            r(&d(1), "/andet\tusername_and_password_leaked\t302", 1),
            r(&d(2), "/login\tusername_leaked\t200", 9),
            r(&d(2), "/login\tukendt\t200", 1),
            r(&d(2), "\tnone\t", 1234),
            r(&d(90), "/login\tpassword_leaked\t200", 5),
        ];
        l.laek_hentet = [d(1), d(2)].into_iter().collect();
        let x = regn(&Periode::D30, &Filter::default(), &l, NU);
        assert_eq!(
            x.laek
                .iter()
                .map(|y| (y.dag.as_str(), y.sti.as_str(), y.n, y.naaet))
                .collect::<Vec<_>>(),
            [(d(1).as_str(), "/login", 5, 2), (d(1).as_str(), "/andet", 1, 1)]
        );
        assert_eq!(
            x.laek_andet.iter().map(|y| (y.v.as_str(), y.n)).collect::<Vec<_>>(),
            [("none", 1234), ("username_leaked", 9), ("ukendt", 1)],
            "`none` er en optælling, aldrig et fund"
        );
        assert_eq!(x.laek_dage, 2);
        let f = Filter {
            sti: Some("/andet".into()),
            ..Filter::default()
        };
        assert_eq!(regn(&Periode::D30, &f, &l, NU).laek.len(), 1, "stien filtrerer");
    }

    fn laest() -> Laest {
        Laest {
            laek: Vec::new(),
            laek_hentet: HashSet::new(),
            rows: vec![
                fw(&d(1), "/login", "PL", "Eksempel Net", "block", 10),
                fw(&d(2), "/login", "US", "AWS", "managed_challenge", 5),
                fw(&d(2), "/login", "US", "AWS", "link_maze_injected", 700),
                fw(&d(2), "/login", "US", "AWS", "link_maze_visited", 80),
                fw(&d(2), "/andet", "US", "AWS", "block", 600),
                rk(&d(1), "probe", "/login", 3),
                rk(&d(1), "probe", "/andet", 500),
                login(&d(1), "/login", 200, 200, "PL", "64500", 4),
                login(&d(1), "/login", 302, 302, "PL", "64500", 2),
                login(&d(2), "/login", 401, 401, "DE", "64501", 6),
                login(&d(2), "/login", 500, 500, "DE", "64501", 1),
                login(&d(2), "/login", 403, 0, "DE", "64501", 90),
                login(&d(2), "/andet", 200, 200, "DE", "64501", 400),
                rk(&d(1), "asn", "64500\tEksempel Net", 2),
            ],
            stier: HashMap::from([("z1".to_string(), HashSet::from(["/login".to_string()]))]),
            hentet: (0..=31).map(d).collect(),
            ekstra: HashMap::from([("z1".to_string(), (1..=30).map(d).collect())]),
            ikke_tilgaengelig: vec![],
        }
    }

    fn u(f: Filter) -> LoginUd {
        regn(&Periode::D30, &f, &laest(), NU)
    }

    /// Mutation: læg `fw` og den nye node sammen, og denne test falder.
    #[test]
    fn de_tre_tal_laegges_aldrig_sammen() {
        let t = u(Filter::default()).tal;
        assert_eq!(t.edge, 15, "kun blokeret og udfordret; fælde-links og labyrinten er ikke forsøg");
        assert_eq!(t.ikke_fundet, 3);
        assert_eq!(t.server, 13, "kun rækker med origin-status");
        assert_eq!((t.svar, t.afvist, t.andet), (6, 6, 1));
        assert_eq!(t.svar + t.afvist + t.andet, t.server, "fordelingen er af serveren alene");
        let s = serde_json::to_value(&t).unwrap();
        assert_eq!(s.as_object().unwrap().len(), 6, "der findes ingen sum af kilderne");
    }

    #[test]
    fn kun_stier_der_er_slaaet_til() {
        let mut l = laest();
        l.stier.insert("z1".into(), HashSet::new());
        let x = regn(&Periode::D30, &Filter::default(), &l, NU);
        assert_eq!(x.tal, Tal::default());
        assert!(x.raekker.is_empty());
    }

    #[test]
    fn dage_og_stier_holder_kilderne_adskilt() {
        let x = u(Filter::default());
        let dag1 = x.dage.iter().find(|y| y.dag == d(1)).unwrap();
        assert_eq!((dag1.edge, dag1.ikke_fundet, dag1.server), (10, 3, 6));
        let r = &x.raekker[0];
        assert_eq!((r.sti.as_str(), r.tal.edge, r.tal.ikke_fundet, r.tal.server), ("/login", 15, 3, 13));
        assert_eq!(r.metoder, ["POST"]);
    }

    #[test]
    fn lande_og_netvaerk_pr_kilde_med_asn_navn() {
        let x = u(Filter::default());
        let par = |v: &[VaerdiN]| v.iter().map(|y| (y.v.clone(), y.n)).collect::<Vec<_>>();
        assert_eq!(par(&x.lande_edge), [("PL".to_string(), 10), ("US".to_string(), 5)]);
        assert_eq!(par(&x.lande_server), [("DE".to_string(), 7), ("PL".to_string(), 6)]);
        assert_eq!(
            par(&x.net_server),
            [("AS64501".to_string(), 7), ("Eksempel Net".to_string(), 6)],
            "navnet fra firewall-rækkerne, ellers nummeret"
        );
    }

    #[test]
    fn filtre_passer_med_fliserne() {
        let f = |udfald: &str| Filter {
            udfald: Some(udfald.into()),
            ..Default::default()
        };
        let x = u(f("svar"));
        assert_eq!(x.tal.edge, 15, "fliserne bliver stående");
        assert_eq!(x.dage.iter().map(|y| y.server).sum::<i64>(), x.tal.svar, "søjlerne følger filteret");
        assert_eq!(x.dage.iter().map(|y| y.edge).sum::<i64>(), 0);
        assert_eq!(u(f("edge")).raekker[0].tal.edge, 15);
        let x = u(Filter {
            land: Some("PL".into()),
            ..Default::default()
        });
        assert_eq!((x.tal.edge, x.tal.server, x.tal.ikke_fundet), (10, 6, 0), "404 kender ikke land");
        let x = u(Filter {
            netvaerk: Some("Eksempel Net".into()),
            ..Default::default()
        });
        assert_eq!((x.tal.edge, x.tal.server), (10, 6), "samme navn i begge kilder");
        let x = u(Filter {
            metode: Some("GET".into()),
            ..Default::default()
        });
        assert_eq!(x.tal, Tal::default(), "ingen GET i tallene, og 404 kender ikke metode");
        let x = regn(
            &Periode::Dag(d(1)),
            &Filter {
                sti: Some("/login".into()),
                ..Default::default()
            },
            &laest(),
            NU,
        );
        assert_eq!((x.tal.edge, x.dage.len()), (10, 1));
    }

    #[test]
    fn svar_200_og_302_er_svar_ikke_login() {
        assert_eq!(slags(200), SVAR);
        assert_eq!(slags(302), SVAR);
        assert_eq!(slags(401), AFVIST);
        assert_eq!(slags(403), AFVIST);
        assert_eq!(slags(404), ANDET);
        assert_eq!(slags(500), ANDET);
    }

    #[test]
    fn ukendte_filtre_afvises() {
        let f = |u: &str| Filter {
            udfald: Some(u.into()),
            ..Default::default()
        };
        assert!(tjek(f("edge")).is_ok() && tjek(f("svar")).is_ok() && tjek(Filter::default()).is_ok());
        assert_eq!(tjek(f("lykket")).err(), Some(Fejl::UgyldigtFilter));
        let lang = Filter {
            sti: Some("/".repeat(MAKS_VAERDI + 1)),
            ..Default::default()
        };
        assert_eq!(tjek(lang).err(), Some(Fejl::UgyldigtFilter));
    }

    #[test]
    fn stier_tjekkes() {
        assert_eq!(tjek_sti("/login").unwrap(), "/login");
        for s in ["login", "", "/a b", "/a\nb", "/a?x=1", &format!("/{}", "a".repeat(MAKS_VAERDI))] {
            assert_eq!(tjek_sti(s).err(), Some(Fejl::UgyldigtFilter), "{s:?}");
        }
    }

    /// Forsiden foreslås aldrig, og en sti, hvor en kendt regel stoppede flertallet af POST, er et angreb.
    #[test]
    fn forslag_aldrig_forsiden_eller_angreb() {
        let p = |dag: &str, sti: &str, n: i64| rk(dag, "post", &[sti, "200", "200", "PL", ""].join("\t"), n).1;
        let tre = |sti: &str, n: i64| vec![p(&d(1), sti, n), p(&d(2), sti, n), p(&d(3), sti, n)];
        let stop = |sti: &str, metode: &str, kilde: &str, handling: &str, n: i64| {
            rk(&d(1), "fw", &["R", "US", "N", sti, "CPH", metode, kilde, handling].join("\t"), n).1
        };
        let mut rows = [
            tre("/", 500),
            tre("/login-angrebet", 10),
            tre("/login-get", 10),
            tre("/login-log", 10),
            tre("/login2", 10),
            tre("/login-rutine", 10),
            tre("/wp-json/batch/v1", 10),
            tre("/api", 10),
            tre("/app", 10),
        ]
        .concat();
        rows.push(stop("/login-angrebet", "POST", "firewallManaged", "block", 1));
        rows.push(stop("/login-get", "GET", "firewallManaged", "block", 1));
        rows.push(stop("/login-log", "POST", "firewallManaged", "log", 1));
        rows.push(stop("/login-rutine", "POST", "linkMaze", "block", 500));
        rows.push(stop("/wp-json/batch/v1", "POST", "firewallManaged", "block", 3));
        assert_eq!(forslag(&rows, &HashSet::new()), ["/login-rutine", "/login2"]);
    }

    /// `/api`, `/app` og angrebsstien kommer ikke med, heller ikke selvom Cloudflare stoppede ingenting. En omdøbt login kommer med, når tjekket har set et kodeord, men ikke på `none` eller det ukendte.
    #[test]
    fn forslag_kraever_login_ord_eller_kodeord_set() {
        let p = |dag: &str, sti: &str| rk(dag, "post", &[sti, "200", "200", "PL", ""].join("\t"), 3).1;
        let tre = |sti: &str| vec![p(&d(1), sti), p(&d(2), sti), p(&d(3), sti)];
        let mut rows = [
            tre("/api"),
            tre("/app"),
            tre("/wp-json/batch/v1"),
            tre("/indgang"),
            tre("/portal"),
            tre("/doer"),
            tre("/none"),
            tre("/Sign-In/Ny"),
            tre("/mit-logon"),
        ]
        .concat();
        rows.push(rk(&d(1), "laek_ren", "/indgang", 2).1);
        rows.push(rk(&d(1), "laek", "/portal\tpassword_leaked\t200", 1).1);
        rows.push(rk(&d(1), "laek", "/none\tnone\t200", 90).1);
        rows.push(rk(&d(1), "laek", "/doer\tukendt\t200", 90).1);
        rows.push(rk(&d(1), "laek", "\tnone\t", 1234).1);
        assert_eq!(
            forslag(&rows, &HashSet::new()),
            ["/Sign-In/Ny", "/indgang", "/mit-logon", "/portal"]
        );
    }

    #[test]
    fn stier_der_ligner_et_login_paa_navnet() {
        for sti in [
            "/auth",
            "/auth/",
            "/AUTH",
            "/api/auth/session",
            "/auth.php",
            "/jwt-auth/v1/token",
            "/user/login",
            "/wp-login.php",
            "/wp-admin/",
            "/my-account",
            "/blog/wp-login.php",
            "/login",
            "/logins",
            "/login.php",
            "/signin",
            "/sign-in",
            "/Sign-In/Ny",
            "/logon",
            "/users",
            "/user_profile",
            "/account",
            "/accounts",
            "/admin",
            "/administrator/",
            "/sessions",
            "/authenticate",
            "/api/authorize",
            "/oauth/token",
            "/oauth2/authorize",
            "/oauth2",
            "/login2",
            "/api/tokens",
            "/sso",
            "/saml/acs",
            "/2fa",
            "/mfa/verify",
            "/otp",
            "/password/reset",
            "/passwords",
            "/passwd",
            "/reset",
        ] {
            assert!(ligner_login(sti), "{sti}");
        }
    }

    #[test]
    fn stier_der_ikke_er_et_login() {
        for sti in [
            "/wp-admin/admin-ajax.php",
            "/WP-ADMIN/admin-ajax.php",
            "/blog/wp-admin/admin-ajax.php",
            "/wp-admin/admin-post.php",
            "/author",
            "/author/",
            "/author/admin",
            "/authors",
            "/authors/admin/",
            "/da/author/admin",
            "/wp-json/wp/v2/users",
            "/wp-json/wp/v2/users/1",
            "/wp-json/wp/v2/users/me",
            "/wp-json/users",
            "/authority",
            "/coauthored",
            "/badminton",
            "/accounting",
            "/obsession",
            "/superuser",
            "/usercontent",
            "/blogin",
            "/logout",
            "/presets",
            "/tokenize",
            "/lotpris",
        ] {
            assert!(!ligner_login(sti), "{sti}");
        }
    }

    /// Hvert mønster fanger noget, og de rammer ikke indgangene på startlisten.
    #[test]
    fn undtagelserne_rammer_ikke_startlisten() {
        assert_eq!(IKKE_LOGIN.len(), IKKE_LOGIN_MOENSTRE.len());
        for sti in STARTLISTE {
            assert!(!IKKE_LOGIN_MOENSTRE.is_match(sti), "{sti}");
        }
    }

    /// Undtagelsen gælder kun navnet: har tjekket set et kodeord på stien, kan den stadig foreslås.
    #[test]
    fn undtagelsen_gaelder_kun_forslag_fra_navnet() {
        let p = |dag: &str, sti: &str| rk(dag, "post", &[sti, "200", "200", "PL", ""].join("\t"), 3).1;
        let tre = |sti: &str| vec![p(&d(1), sti), p(&d(2), sti), p(&d(3), sti)];
        let mut rows = [tre("/wp-admin/admin-ajax.php"), tre("/authors/admin"), tre("/api/auth/session")].concat();
        assert_eq!(forslag(&rows, &HashSet::new()), ["/api/auth/session"]);
        rows.push(rk(&d(1), "laek_ren", "/wp-admin/admin-ajax.php", 2).1);
        assert_eq!(forslag(&rows, &HashSet::new()), ["/api/auth/session", "/wp-admin/admin-ajax.php"]);
    }

    #[test]
    fn forslag_kraever_tre_dage_og_ikke_mest_404() {
        let p = |dag: &str, sti: &str, status: &str, n: i64| rk(dag, "post", &[sti, status, status, "PL", ""].join("\t"), n).1;
        let rows = vec![
            p(&d(1), "/api/auth/signin", "200", 5),
            p(&d(2), "/api/auth/signin", "401", 5),
            p(&d(3), "/api/auth/signin", "302", 5),
            p(&d(1), "/login-kun-to", "200", 5),
            p(&d(2), "/login-kun-to", "200", 5),
            p(&d(1), "/login-mest-404", "200", 1),
            p(&d(2), "/login-mest-404", "200", 1),
            p(&d(3), "/login-mest-404", "200", 1),
            p(&d(3), "/login-mest-404", "404", 9),
            p(&d(1), "/login-kendt", "200", 1),
            p(&d(2), "/login-kendt", "200", 1),
            p(&d(3), "/login-kendt", "200", 1),
            p(&d(1), "/login-fejl", "500", 9),
            p(&d(2), "/login-fejl", "500", 9),
            p(&d(3), "/login-fejl", "500", 9),
        ];
        assert_eq!(forslag(&rows, &HashSet::from(["/login-kendt"])), ["/api/auth/signin"]);
    }

    #[test]
    fn indstillingen_har_startlisten_egne_stier_og_forslag() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_login_sti("z1", "/login", true).unwrap();
        s.gem_login_sti("z1", "/mit", false).unwrap();
        for i in 1..=3 {
            s.gem_ekstra("z1", &d(i), &[("post".into(), "/signin2\t200\t200\tPL\t".into(), 2)], "t")
                .unwrap();
        }
        let u = stier(&s, "z1").unwrap();
        assert_eq!(u.stier.len(), STARTLISTE.len() + 1);
        assert!(u.stier.iter().find(|x| x.sti == "/login").unwrap().til);
        assert!(
            !u.stier.iter().find(|x| x.sti == "/admin/").unwrap().til,
            "intet er slået til af sig selv"
        );
        let mit = u.stier.iter().find(|x| x.sti == "/mit").unwrap();
        assert!(!mit.til && !mit.start);
        assert_eq!(u.forslag, ["/signin2"]);
    }
}
