//! Menulinjeikonet: programmet bliver i menulinjen (macOS) eller notifikationsområdet (Windows), når vinduet
//! er lukket, og køen henter videre. Menuen: Åbn Huginn · Siden sidst · Hent nu · statuslinjen (åbner Overblik) · Afslut.
//! Statuslinjen og tooltippen viser køens tilstand (`menulinje_tekst`, ren funktion); de opdateres, når køen melder, og
//! ved hvert tik (den relative tid). Ikonet bygges i Rust, så webvisningen får ingen `core:tray:*`-rettigheder. Sproget
//! er UI'ets valg (`sprog_saet`, gemt i SQLite), så menuen også er på det rigtige sprog, når programmet startede ved login.
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Wry};

use crate::cf::Fejl;
use crate::koe::{Fase, KoeStatus};
use crate::notify::flertal;
use crate::overblik::{MoensterSlags, Raekke1};
use crate::store::Forbindelse;
use crate::vaern::parse_iso;

pub const AABN: &str = "aabn";
pub const SIDEN: &str = "siden";
pub const HENT: &str = "hent";
pub const AFSLUT: &str = "afslut";
pub const STATUS: &str = "status";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sprog {
    Da,
    En,
}

impl Sprog {
    /// UI'ets afgjorte sprog ("da" eller "en"; "auto" er afgjort i webvisningen).
    pub fn fra(s: &str) -> Option<Sprog> {
        match s {
            "da" => Some(Sprog::Da),
            "en" => Some(Sprog::En),
            _ => None,
        }
    }

    pub fn kode(self) -> &'static str {
        match self {
            Sprog::Da => "da",
            Sprog::En => "en",
        }
    }
}

/// Menupunkternes tekster: Åbn, Siden sidst, Hent nu, Afslut.
fn punkter(sprog: Sprog) -> [&'static str; 4] {
    match sprog {
        Sprog::Da => ["Åbn Huginn", "Siden sidst", "Hent nu", "Afslut"],
        Sprog::En => ["Open Huginn", "Since last time", "Fetch now", "Quit"],
    }
}

/// "for 12 min. siden" / "12 min ago"; under et minut "lige nu", over to timer i timer, over to døgn i dage.
fn siden(ms: i64, da: bool) -> String {
    let min = ms.max(0) / 60_000;
    match (min, da) {
        (0, true) => "lige nu".into(),
        (0, false) => "just now".into(),
        (1..120, true) => format!("for {min} min. siden"),
        (1..120, false) => format!("{min} min ago"),
        (120..2880, true) => format!("for {} t. siden", min / 60),
        (120..2880, false) => format!("{} h ago", min / 60),
        (_, true) => format!("for {} dage siden", min / 1440),
        (_, false) => format!("{} days ago", min / 1440),
    }
}

/// "om 3 min." / "in 3 min"; uden tidspunkt "snart".
fn om(kl: Option<&str>, nu: i64, da: bool) -> String {
    let Some(ms) = kl.and_then(parse_iso) else {
        return if da { "snart" } else { "soon" }.into();
    };
    let min = ((ms - nu).max(0) as u64).div_ceil(60_000);
    match (min, da) {
        (0..=1, true) => "om under 1 min.".into(),
        (0..=1, false) => "in under a minute".into(),
        (_, true) => format!("om {min} min."),
        (_, false) => format!("in {min} min"),
    }
}

/// Domænerne med et mærke i statuslinjen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Maerker {
    pub maalrettet: Vec<String>,
    pub rekognoscering: Vec<String>,
    pub dns: Vec<String>,
}

impl Maerker {
    /// Rækkernes mærker: "Ligner målrettet" først, når det er afgjort. Ren funktion.
    pub fn af(rk: &[Raekke1]) -> Maerker {
        let navne = |f: &dyn Fn(&Raekke1) -> bool| rk.iter().filter(|r| f(r)).map(|r| r.navn.clone()).collect();
        let slags = |s: MoensterSlags| move |r: &Raekke1| r.moenster.as_ref().is_some_and(|m| m.slags == s);
        Maerker {
            maalrettet: navne(&|r| r.afgjort.is_some()),
            rekognoscering: navne(&slags(MoensterSlags::Rekognoscering)),
            dns: navne(&slags(MoensterSlags::DnsRekognoscering)),
        }
    }
}

/// Mærket i statuslinjen med det første navn eller antallet. Målrettet står forrest og DNS sidst, som på rækkerne. Tom uden mærker. Ren funktion.
pub fn maerke_tekst(m: &Maerker, sprog: Sprog) -> String {
    let da = sprog == Sprog::Da;
    let (ord, liste) = if !m.maalrettet.is_empty() {
        (if da { "Ligner målrettet" } else { "Looks targeted" }, &m.maalrettet)
    } else if !m.rekognoscering.is_empty() {
        (
            if da { "Ligner rekognoscering" } else { "Looks like reconnaissance" },
            &m.rekognoscering,
        )
    } else if !m.dns.is_empty() {
        (
            if da {
                "Ligner DNS-rekognoscering"
            } else {
                "Looks like DNS reconnaissance"
            },
            &m.dns,
        )
    } else {
        return String::new();
    };
    match liste.as_slice() {
        [navn] => format!(" · {ord}: {navn}"),
        _ if da => format!(" · {ord}: {} domæner", liste.len()),
        _ => format!(" · {ord}: {} domains", liste.len()),
    }
}

/// "· 3 nye siden kl. 08.10": nyt i Siden sidst siden sidste åbning. Tom uden noget nyt eller uden en åbning at regne fra. Ren funktion.
pub fn nye_tekst(n: usize, kl: Option<(i64, i64)>, sprog: Sprog) -> String {
    let Some((t, m)) = kl.filter(|_| n > 0) else {
        return String::new();
    };
    match sprog {
        Sprog::Da => format!(" · {n} {} siden kl. {t:02}.{m:02}", if n == 1 { "ny" } else { "nye" }),
        Sprog::En => format!(" · {n} new since {t:02}:{m:02}"),
    }
}

/// Forbindelsens viste navn (V-KONTI.1, D5): brugerens, ellers Cloudflares kontonavn (med "+ N andre", når tokenet ser flere konti). `None` uden navn. Ren funktion.
pub fn vist_navn(f: &Forbindelse, sprog: Sprog) -> Option<String> {
    if let Some(eget) = f.eget_navn.as_deref().filter(|n| !n.is_empty()) {
        return Some(eget.to_string());
    }
    let cf = f.cf_navn.as_deref().filter(|n| !n.is_empty())?;
    Some(match (f.konti - 1, sprog) {
        (n, Sprog::Da) if n > 0 => format!("{cf} + {n} {}", if n == 1 { "anden" } else { "andre" }),
        (n, Sprog::En) if n > 0 => format!("{cf} + {n} more"),
        _ => cf.to_string(),
    })
}

/// Teksten, når Cloudflare har afvist en eller flere forbindelsers token: én med navn → navnet, én uden navn → som i 1.0.0, flere → "n af m". `None`, når
/// ingen er afvist. Kontoens navn står kun her, i databasen og i `forbindelser_hent` (V-KONTI.1). Ren funktion.
pub fn afvist_tekst(afviste: &[i64], forbindelser: &[Forbindelse], sprog: Sprog) -> Option<String> {
    let da = sprog == Sprog::Da;
    match afviste {
        [] => None,
        [id] => Some(
            match (forbindelser.iter().find(|f| f.id == *id).and_then(|f| vist_navn(f, sprog)), da) {
                (Some(n), true) => format!("Cloudflare afviste API Tokenet «{n}»"),
                (Some(n), false) => format!("Cloudflare rejected the API Token “{n}”"),
                (None, true) => "Cloudflare afviste API Tokenet".into(),
                (None, false) => "Cloudflare rejected the API Token".into(),
            },
        ),
        _ => {
            let (n, m) = (afviste.len(), forbindelser.len().max(afviste.len()));
            Some(match da {
                true => format!("Cloudflare afviste {n} af {m} API Tokens"),
                false => format!("Cloudflare rejected {n} of {m} API Tokens"),
            })
        }
    }
}

/// Statuslinjen i menuen (og tooltippen) ud fra køens seneste status. `faerdig_kl` er, hvornår den seneste runde sluttede
/// (ms, UTC), og `nu` er uret; begge på vægur, så teksten passer efter dvale. `forbindelser` giver navnene ved en afvisning. Ren funktion.
pub fn menulinje_tekst(status: Option<&KoeStatus>, faerdig_kl: Option<i64>, nu: i64, sprog: Sprog, forbindelser: &[Forbindelse]) -> String {
    let da = sprog == Sprog::Da;
    let Some(s) = status else {
        return if da { "Starter …" } else { "Starting …" }.into();
    };
    let kl = s.fortsaetter_kl.as_deref();
    match s.tilstand {
        Fase::Henter if da => format!("Henter {}/{}", s.zoner_faerdige, s.zoner_i_alt),
        Fase::Henter => format!("Fetching {}/{}", s.zoner_faerdige, s.zoner_i_alt),
        Fase::VenterKvote if da => format!("Cloudflare beder os vente – fortsætter {}", om(kl, nu, da)),
        Fase::VenterKvote => format!("Cloudflare asks us to wait – continuing {}", om(kl, nu, da)),
        Fase::Pause => match (s.fejl == Some(Fejl::Forbindelse), da) {
            (true, true) => format!("Ingen forbindelse – prøver igen {}", om(kl, nu, da)),
            (true, false) => format!("No connection – trying again {}", om(kl, nu, da)),
            (false, true) => format!("Cloudflare svarer ikke – prøver igen {}", om(kl, nu, da)),
            (false, false) => format!("Cloudflare is not answering – trying again {}", om(kl, nu, da)),
        },
        // Som båndet (V-BESOEG.2 FIX 1): uden forbindelse siges det i klart sprog, og køen prøver selv igen.
        Fase::Faerdig if s.zoner_fejl > 0 && s.fejl == Some(Fejl::Forbindelse) => match da {
            true => format!("Ingen forbindelse til Cloudflare · prøver igen {}", om(kl, nu, da)),
            false => format!("No connection to Cloudflare · trying again {}", om(kl, nu, da)),
        },
        Fase::Faerdig => {
            let mut t = match (faerdig_kl, da) {
                (Some(f), true) => format!("Hentet {}", siden(nu - f, da)),
                (Some(f), false) => format!("Fetched {}", siden(nu - f, da)),
                (None, true) => "Hentet".into(),
                (None, false) => "Fetched".into(),
            };
            match (s.zoner_fejl, da) {
                (0, _) => {}
                (n, true) => t.push_str(&format!(" · {n} {} fejlede", flertal(n as i64, "domæne", "domæner"))),
                (n, false) => t.push_str(&format!(" · {n} {} failed", flertal(n as i64, "domain", "domains"))),
            }
            // V-KONTI.1: andre forbindelser blev hentet, men en er afvist.
            if let Some(a) = afvist_tekst(&s.afviste, forbindelser, sprog) {
                t.push_str(" · ");
                t.push_str(&a);
            }
            t
        }
        Fase::Fejl => match (&s.fejl, da) {
            (Some(Fejl::Noegle), true) => "Venter på nøgleringen – vælg Hent nu".into(),
            (Some(Fejl::Noegle), false) => "Waiting for the keychain – choose Fetch now".into(),
            (Some(Fejl::IntetToken), true) => "Intet API Token – åbn Huginn".into(),
            (Some(Fejl::IntetToken), false) => "No API Token – open Huginn".into(),
            (Some(Fejl::Afvist(_)), _) => afvist_tekst(&s.afviste, forbindelser, sprog).unwrap_or_else(|| {
                if da {
                    "Cloudflare afviste API Tokenet"
                } else {
                    "Cloudflare rejected the API Token"
                }
                .into()
            }),
            (_, true) => "Hentningen er stoppet".into(),
            (_, false) => "Fetching has stopped".into(),
        },
    }
}

/// Ikonet og menuen (managed state). Menupunkterne gemmes, så teksterne kan skiftes uden at bygge menuen igen.
pub struct Agent {
    ikon: TrayIcon,
    /// Åbn, Siden sidst, Hent nu, Afslut (samme rækkefølge som `punkter`).
    punkter: [MenuItem<Wry>; 4],
    status: MenuItem<Wry>,
    sprog: Mutex<Sprog>,
    faerdig_kl: Mutex<Option<i64>>,
    /// Domænerne med et mærke.
    maerker: Mutex<Maerker>,
    /// Nyt siden Siden sidst blev åbnet: antal og hvornår (lokal time og minut).
    nye: Mutex<(usize, Option<(i64, i64)>)>,
    /// Forbindelserne (navnene ved en afvisning).
    forbindelser: Mutex<Vec<Forbindelse>>,
}

impl Agent {
    /// Bygger ikonet og menuen. `paa_menu` får menupunktets id (`AABN`, `SIDEN`, `HENT`, `STATUS`, `AFSLUT`).
    pub fn byg(app: &AppHandle, sprog: Sprog, paa_menu: fn(&AppHandle, &str)) -> tauri::Result<Agent> {
        let [aabn, siden, hent, afslut] = punkter(sprog);
        let punkter = [
            MenuItem::with_id(app, AABN, aabn, true, None::<&str>)?,
            MenuItem::with_id(app, SIDEN, siden, true, None::<&str>)?,
            MenuItem::with_id(app, HENT, hent, true, None::<&str>)?,
            MenuItem::with_id(app, AFSLUT, afslut, true, None::<&str>)?,
        ];
        let tekst = menulinje_tekst(None, None, 0, sprog, &[]);
        let status = MenuItem::with_id(app, STATUS, &tekst, true, None::<&str>)?;
        let menu = Menu::with_items(
            app,
            &[
                &punkter[0],
                &punkter[1],
                &punkter[2],
                &status,
                &PredefinedMenuItem::separator(app)?,
                &punkter[3],
            ],
        )?;
        // macOS: sort + alfa som template-ikon, så systemet farver det til lys og mørk menulinje. Windows: det farvede ikon.
        #[cfg(target_os = "macos")]
        let billede = tauri::include_image!("icons/menulinje.png");
        #[cfg(not(target_os = "macos"))]
        let billede = tauri::include_image!("icons/32x32.png");
        let ikon = TrayIconBuilder::with_id("huginn")
            .icon(billede)
            .icon_as_template(true)
            .tooltip(&tekst)
            .menu(&menu)
            .show_menu_on_left_click(true)
            .on_menu_event(move |app, ev| paa_menu(app, ev.id().as_ref()))
            .build(app)?;
        Ok(Agent {
            ikon,
            punkter,
            status,
            sprog: Mutex::new(sprog),
            faerdig_kl: Mutex::new(None),
            maerker: Mutex::new(Maerker::default()),
            nye: Mutex::new((0, None)),
            forbindelser: Mutex::new(Vec::new()),
        })
    }

    fn sprog(&self) -> Sprog {
        *self.sprog.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Viser køens status i statuslinjen og tooltippen.
    pub fn vis(&self, status: Option<&KoeStatus>, nu: i64) {
        let faerdig = *self.faerdig_kl.lock().unwrap_or_else(|e| e.into_inner());
        let maerker = self.maerker.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let sprog = self.sprog();
        let (nye, kl) = *self.nye.lock().unwrap_or_else(|e| e.into_inner());
        let forbindelser = self.forbindelser.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let tekst = format!(
            "{}{}{}",
            menulinje_tekst(status, faerdig, nu, sprog, &forbindelser),
            maerke_tekst(&maerker, sprog),
            nye_tekst(nye, kl, sprog)
        );
        let _ = self.status.set_text(&tekst);
        let _ = self.ikon.set_tooltip(Some(&tekst));
    }

    /// Forbindelserne, som de står i databasen (navnene ved en afvisning).
    pub fn saet_forbindelser(&self, f: Vec<Forbindelse>) {
        *self.forbindelser.lock().unwrap_or_else(|e| e.into_inner()) = f;
    }

    /// Domænerne med et mærke.
    pub fn saet_maerker(&self, maerker: Maerker) {
        *self.maerker.lock().unwrap_or_else(|e| e.into_inner()) = maerker;
    }

    /// Nyt i Siden sidst, og hvornår kortet sidst blev åbnet (lokal time og minut).
    pub fn saet_nye(&self, n: usize, kl: Option<(i64, i64)>) {
        *self.nye.lock().unwrap_or_else(|e| e.into_inner()) = (n, kl);
    }

    /// Køen har meldt: en færdig runde noteres (til "Hentet for N min. siden").
    pub fn meld(&self, status: &KoeStatus, nu: i64) {
        if status.tilstand == Fase::Faerdig {
            *self.faerdig_kl.lock().unwrap_or_else(|e| e.into_inner()) = Some(nu);
        }
        self.vis(Some(status), nu);
    }

    pub fn saet_sprog(&self, sprog: Sprog, status: Option<&KoeStatus>, nu: i64) {
        *self.sprog.lock().unwrap_or_else(|e| e.into_inner()) = sprog;
        for (punkt, tekst) in self.punkter.iter().zip(punkter(sprog)) {
            let _ = punkt.set_text(tekst);
        }
        self.vis(status, nu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaern::iso;

    /// 2026-11-03T08:08:00Z
    const NU: i64 = 1_793_693_280_000;
    const MIN: i64 = 60_000;

    fn st(tilstand: Fase) -> KoeStatus {
        KoeStatus {
            zone: None,
            navn: None,
            doegn_faerdige: 87,
            doegn_i_alt: 229,
            zoner_faerdige: 3,
            zoner_i_alt: 16,
            zoner_fejl: 0,
            zoner_opdateret: 0,
            tilstand,
            fejl: None,
            fortsaetter_kl: None,
            fejl_navn: None,
            afviste: Vec::new(),
        }
    }

    fn begge(s: Option<&KoeStatus>, faerdig: Option<i64>) -> (String, String) {
        begge_med(s, faerdig, &[])
    }

    fn begge_med(s: Option<&KoeStatus>, faerdig: Option<i64>, f: &[Forbindelse]) -> (String, String) {
        (
            menulinje_tekst(s, faerdig, NU, Sprog::Da, f),
            menulinje_tekst(s, faerdig, NU, Sprog::En, f),
        )
    }

    fn forb(id: i64, cf: Option<&str>, eget: Option<&str>, konti: i64) -> Forbindelse {
        Forbindelse {
            id,
            cf_navn: cf.map(str::to_string),
            eget_navn: eget.map(str::to_string),
            konti,
            domaener: 1,
        }
    }

    /// V-KONTI.1: én afvist forbindelse nævnes ved sit viste navn (brugerens, ellers Cloudflares, "+ N andre" ved flere konti), flere som "n af m".
    /// Med én forbindelse uden navn er teksten som i 1.0.0. Andre forbindelser blev hentet: teksten står efter "Hentet".
    #[test]
    fn afvist_med_forbindelsens_navn() {
        let t = |a: &str, b: &str| (a.to_string(), b.to_string());
        let fejl = |afviste: Vec<i64>| KoeStatus {
            fejl: Some(Fejl::Afvist(403)),
            afviste,
            ..st(Fase::Fejl)
        };
        assert_eq!(
            begge_med(Some(&fejl(vec![1])), None, &[forb(1, None, None, 0)]),
            t("Cloudflare afviste API Tokenet", "Cloudflare rejected the API Token")
        );
        let navnet = [forb(1, Some("test@wi24rd.com's Account"), None, 1)];
        assert_eq!(
            begge_med(Some(&fejl(vec![1])), None, &navnet),
            t(
                "Cloudflare afviste API Tokenet «test@wi24rd.com's Account»",
                "Cloudflare rejected the API Token “test@wi24rd.com's Account”"
            ),
            "D5: kontoens navn uændret"
        );
        let flere = [
            forb(1, Some("Eksempel ApS"), Some("Kunden"), 1),
            forb(2, Some("Anden Konto"), None, 3),
        ];
        assert_eq!(
            begge_med(Some(&fejl(vec![1])), None, &flere),
            t(
                "Cloudflare afviste API Tokenet «Kunden»",
                "Cloudflare rejected the API Token “Kunden”"
            ),
            "brugerens navn vinder"
        );
        assert_eq!(
            begge_med(Some(&fejl(vec![1, 2])), None, &flere),
            t("Cloudflare afviste 2 af 2 API Tokens", "Cloudflare rejected 2 of 2 API Tokens")
        );
        let delvis = KoeStatus {
            afviste: vec![2],
            ..st(Fase::Faerdig)
        };
        assert_eq!(
            begge_med(Some(&delvis), Some(NU - 2 * MIN), &flere),
            t(
                "Hentet for 2 min. siden · Cloudflare afviste API Tokenet «Anden Konto + 2 andre»",
                "Fetched 2 min ago · Cloudflare rejected the API Token “Anden Konto + 2 more”"
            )
        );
        assert_eq!(
            vist_navn(&forb(3, Some("A"), Some(""), 2), Sprog::Da).as_deref(),
            Some("A + 1 anden"),
            "tomt eget navn: Cloudflares"
        );
        assert_eq!(vist_navn(&forb(3, None, None, 0), Sprog::En), None);
    }

    #[test]
    fn nye_i_statuslinjen() {
        assert_eq!(nye_tekst(3, Some((8, 10)), Sprog::Da), " · 3 nye siden kl. 08.10");
        assert_eq!(nye_tekst(3, Some((8, 10)), Sprog::En), " · 3 new since 08:10");
        assert_eq!(nye_tekst(1, Some((21, 5)), Sprog::Da), " · 1 ny siden kl. 21.05");
        assert_eq!(nye_tekst(0, Some((8, 10)), Sprog::Da), "", "intet nyt");
        assert_eq!(nye_tekst(3, None, Sprog::Da), "", "aldrig åbnet");
    }

    #[test]
    fn maerket_i_statuslinjen() {
        let (a, b) = (
            vec!["a.wi24rd.com".to_string()],
            vec!["b.wi24rd.com".to_string(), "c.wi24rd.com".to_string()],
        );
        let m = |maalrettet: &[String], rekognoscering: &[String], dns: &[String]| Maerker {
            maalrettet: maalrettet.to_vec(),
            rekognoscering: rekognoscering.to_vec(),
            dns: dns.to_vec(),
        };
        assert_eq!(maerke_tekst(&m(&[], &[], &[]), Sprog::Da), "");
        assert_eq!(maerke_tekst(&m(&a, &b, &b), Sprog::Da), " · Ligner målrettet: a.wi24rd.com");
        assert_eq!(maerke_tekst(&m(&[], &b, &a), Sprog::Da), " · Ligner rekognoscering: 2 domæner");
        assert_eq!(maerke_tekst(&m(&a, &[], &[]), Sprog::En), " · Looks targeted: a.wi24rd.com");
        assert_eq!(maerke_tekst(&m(&[], &b, &[]), Sprog::En), " · Looks like reconnaissance: 2 domains");
    }

    #[test]
    fn statuslinjen_skelner_dns_fra_rekognoscering() {
        let (a, b) = (
            vec!["a.wi24rd.com".to_string()],
            vec!["b.wi24rd.com".to_string(), "c.wi24rd.com".to_string()],
        );
        let dns = |dns: &[String]| Maerker {
            dns: dns.to_vec(),
            ..Maerker::default()
        };
        assert_eq!(maerke_tekst(&dns(&a), Sprog::Da), " · Ligner DNS-rekognoscering: a.wi24rd.com");
        assert_eq!(maerke_tekst(&dns(&b), Sprog::Da), " · Ligner DNS-rekognoscering: 2 domæner");
        assert_eq!(maerke_tekst(&dns(&a), Sprog::En), " · Looks like DNS reconnaissance: a.wi24rd.com");
        assert_eq!(maerke_tekst(&dns(&b), Sprog::En), " · Looks like DNS reconnaissance: 2 domains");
    }

    #[test]
    fn menulinje_tekst_alle_tilstande_paa_dansk_og_engelsk() {
        let t = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(begge(None, None), t("Starter …", "Starting …"), "før første runde");
        assert_eq!(begge(Some(&st(Fase::Henter)), None), t("Henter 3/16", "Fetching 3/16"));

        let faerdig = st(Fase::Faerdig);
        assert_eq!(
            begge(Some(&faerdig), Some(NU - 12 * MIN - 59_000)),
            t("Hentet for 12 min. siden", "Fetched 12 min ago")
        );
        assert_eq!(begge(Some(&faerdig), Some(NU - 30_000)), t("Hentet lige nu", "Fetched just now"));
        assert_eq!(
            begge(Some(&faerdig), Some(NU - 119 * MIN)),
            t("Hentet for 119 min. siden", "Fetched 119 min ago")
        );
        assert_eq!(
            begge(Some(&faerdig), Some(NU - 7 * 60 * MIN)),
            t("Hentet for 7 t. siden", "Fetched 7 h ago")
        );
        assert_eq!(
            begge(Some(&faerdig), Some(NU - 3 * 1440 * MIN)),
            t("Hentet for 3 dage siden", "Fetched 3 days ago")
        );
        assert_eq!(
            begge(Some(&faerdig), Some(NU + 5 * MIN)),
            t("Hentet lige nu", "Fetched just now"),
            "uret stillet tilbage"
        );
        assert_eq!(begge(Some(&faerdig), None), t("Hentet", "Fetched"));
        let fejlede = KoeStatus {
            zoner_fejl: 2,
            zoner_opdateret: 0,
            ..faerdig.clone()
        };
        assert_eq!(
            begge(Some(&fejlede), Some(NU - 3 * MIN)),
            t(
                "Hentet for 3 min. siden · 2 domæner fejlede",
                "Fetched 3 min ago · 2 domains failed"
            )
        );
        let en = KoeStatus {
            zoner_fejl: 1,
            ..faerdig.clone()
        };
        assert_eq!(
            begge(Some(&en), Some(NU - 3 * MIN)),
            t("Hentet for 3 min. siden · 1 domæne fejlede", "Fetched 3 min ago · 1 domain failed")
        );
        // Netfejl: som båndet, ikke "1 domæne fejlede".
        let uden_net = KoeStatus {
            zoner_fejl: 1,
            fejl: Some(Fejl::Forbindelse),
            fortsaetter_kl: Some(iso(NU + 2 * MIN)),
            ..faerdig.clone()
        };
        assert_eq!(
            begge(Some(&uden_net), Some(NU - 3 * MIN)),
            t(
                "Ingen forbindelse til Cloudflare · prøver igen om 2 min.",
                "No connection to Cloudflare · trying again in 2 min"
            )
        );
        let anden = KoeStatus {
            fejl: Some(Fejl::Svar),
            ..uden_net
        };
        assert_eq!(
            begge(Some(&anden), Some(NU - 3 * MIN)),
            t("Hentet for 3 min. siden · 1 domæne fejlede", "Fetched 3 min ago · 1 domain failed"),
            "andre fejl er uændrede"
        );

        let kvote = KoeStatus {
            fortsaetter_kl: Some(iso(NU + 13 * MIN + 1)),
            ..st(Fase::VenterKvote)
        };
        assert_eq!(
            begge(Some(&kvote), None),
            t(
                "Cloudflare beder os vente – fortsætter om 14 min.",
                "Cloudflare asks us to wait – continuing in 14 min"
            )
        );
        let net = KoeStatus {
            fejl: Some(Fejl::Forbindelse),
            fortsaetter_kl: Some(iso(NU + 15_000)),
            ..st(Fase::Pause)
        };
        assert_eq!(
            begge(Some(&net), None),
            t(
                "Ingen forbindelse – prøver igen om under 1 min.",
                "No connection – trying again in under a minute"
            )
        );
        let nede = KoeStatus {
            fortsaetter_kl: None,
            ..st(Fase::Pause)
        };
        assert_eq!(
            begge(Some(&nede), None),
            t(
                "Cloudflare svarer ikke – prøver igen snart",
                "Cloudflare is not answering – trying again soon"
            )
        );

        let fejl = |f: Fejl| KoeStatus {
            fejl: Some(f),
            ..st(Fase::Fejl)
        };
        assert_eq!(
            begge(Some(&fejl(Fejl::Noegle)), None),
            t(
                "Venter på nøgleringen – vælg Hent nu",
                "Waiting for the keychain – choose Fetch now"
            )
        );
        assert_eq!(
            begge(Some(&fejl(Fejl::IntetToken)), None),
            t("Intet API Token – åbn Huginn", "No API Token – open Huginn")
        );
        assert_eq!(
            begge(Some(&fejl(Fejl::Afvist(403))), None),
            t("Cloudflare afviste API Tokenet", "Cloudflare rejected the API Token")
        );
        assert_eq!(
            begge(Some(&fejl(Fejl::Lager)), None),
            t("Hentningen er stoppet", "Fetching has stopped")
        );
    }

    #[test]
    fn sprog_kun_da_og_en() {
        assert_eq!(Sprog::fra("da"), Some(Sprog::Da));
        assert_eq!(Sprog::fra("en"), Some(Sprog::En));
        for nej in ["auto", "", "DA", "de", "da-DK"] {
            assert_eq!(Sprog::fra(nej), None, "{nej}");
        }
        assert_eq!(Sprog::fra(Sprog::En.kode()), Some(Sprog::En));
        assert_eq!(punkter(Sprog::Da)[2], "Hent nu");
        assert_eq!(punkter(Sprog::En)[3], "Quit");
    }
}
