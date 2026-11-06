//! Programmenuen på macOS: Huginn and Muninn (Om · Indstillinger… ⌘, · Tjenester · Skjul · Afslut), Rediger
//! (Fortryd … Vælg alt; uden den virker ⌘C/⌘V ikke i tokenfeltet), Vis (Tilbage ⌘[ · Frem ⌘] · Overblik ⌘1 · Siden sidst ⌘2
//! · Angreb ⌘3 · Robotter ⌘4 · Login ⌘5 · Land lige nu ⌘6 · Hent nu ⌘R · Større tekst ⌘+ · Mindre tekst ⌘− · Normal tekst ⌘0 · Fuld skærm) og Vindue. Teksterne følger UI'ets sprog (`sprog_saet` bygger menuen igen). Et klik
//! på en visning udsender hændelsen `visning` til webvisningen (rammen skifter visning; "Om" åbner Indstillinger → Om i
//! stedet for macOS' standard-Om-boks), Tilbage/Frem udsender `historik`, tekststørrelsen `tekst` (valget ligger i webvisningen),
//! og "Hent nu" starter køen som menulinjens punkt.
//! Et menupunkt har kun én genvej: ⌘← / ⌘→ fanger rammen med `keydown`, så de stadig flytter markøren i et tekstfelt.
//! Windows får ingen programmenu (den ville blive en menubjælke i vinduet); dér fanger rammen genvejene med `keydown`.
use crate::agent::Sprog;

/// Menupunkternes id'er. Andre end menulinjeikonets (`agent::AABN` …), for den globale menuhåndtering får begge slags.
pub const OM: &str = "pm-om";
pub const INDSTILLINGER: &str = "pm-indstillinger";
pub const TILBAGE: &str = "pm-tilbage";
pub const FREM: &str = "pm-frem";
pub const OVERBLIK: &str = "pm-overblik";
pub const ROBOTTER: &str = "pm-robotter";
pub const LOGIN: &str = "pm-login";
pub const LAND: &str = "pm-land";
pub const SIDEN: &str = "pm-siden";
pub const ANGREB: &str = "pm-angreb";
pub const HENT: &str = "pm-hent";
pub const TEKST_STOERRE: &str = "pm-tekst-stoerre";
pub const TEKST_MINDRE: &str = "pm-tekst-mindre";
pub const TEKST_NORMAL: &str = "pm-tekst-normal";

/// Hvad et klik i programmenuen gør.
#[derive(Debug, PartialEq, Eq)]
pub enum Handling {
    /// Skift til visningen (rammens navn: "overblik", "siden-sidst", "vaernet", "robotter", "login", "land", "indstillinger"; "om" = Indstillinger → Om).
    Visning(&'static str),
    /// Et trin i webvisningens historik ("tilbage" | "frem").
    Historik(&'static str),
    /// Et trin i tekststørrelsen ("stoerre" | "mindre" | "normal").
    Tekst(&'static str),
    HentNu,
}

/// Menupunktets id → handling; `None` for alt andet (fx menulinjeikonets punkter). Ren funktion.
pub fn handling(id: &str) -> Option<Handling> {
    match id {
        OM => Some(Handling::Visning("om")),
        INDSTILLINGER => Some(Handling::Visning("indstillinger")),
        TILBAGE => Some(Handling::Historik("tilbage")),
        FREM => Some(Handling::Historik("frem")),
        OVERBLIK => Some(Handling::Visning("overblik")),
        SIDEN => Some(Handling::Visning("siden-sidst")),
        ANGREB => Some(Handling::Visning("vaernet")),
        ROBOTTER => Some(Handling::Visning("robotter")),
        LOGIN => Some(Handling::Visning("login")),
        LAND => Some(Handling::Visning("land")),
        HENT => Some(Handling::HentNu),
        TEKST_STOERRE => Some(Handling::Tekst("stoerre")),
        TEKST_MINDRE => Some(Handling::Tekst("mindre")),
        TEKST_NORMAL => Some(Handling::Tekst("normal")),
        _ => None,
    }
}

/// Genvejene (Tauris accelerator-syntaks).
pub const GENVEJ_INDSTILLINGER: &str = "CmdOrCtrl+,";
pub const GENVEJ_TILBAGE: &str = "CmdOrCtrl+[";
pub const GENVEJ_FREM: &str = "CmdOrCtrl+]";
pub const GENVEJ_OVERBLIK: &str = "CmdOrCtrl+1";
pub const GENVEJ_SIDEN: &str = "CmdOrCtrl+2";
pub const GENVEJ_ANGREB: &str = "CmdOrCtrl+3";
pub const GENVEJ_ROBOTTER: &str = "CmdOrCtrl+4";
pub const GENVEJ_LOGIN: &str = "CmdOrCtrl+5";
pub const GENVEJ_LAND: &str = "CmdOrCtrl+6";
pub const GENVEJ_HENT: &str = "CmdOrCtrl+R";
/// På macOS giver NumpadAdd tegnet + i menuen (muda 0.20), så ⌘+ virker på et dansk tastatur. Equal ville være tegnet =.
pub const GENVEJ_TEKST_STOERRE: &str = "CmdOrCtrl+NumpadAdd";
pub const GENVEJ_TEKST_MINDRE: &str = "CmdOrCtrl+Minus";
pub const GENVEJ_TEKST_NORMAL: &str = "CmdOrCtrl+0";

/// Alle synlige tekster i programmenuen på ét sprog.
#[derive(Debug)]
pub struct Tekster {
    pub om: &'static str,
    pub indstillinger: &'static str,
    pub tjenester: &'static str,
    pub skjul: &'static str,
    pub skjul_andre: &'static str,
    pub vis_alle: &'static str,
    pub afslut: &'static str,
    pub rediger: &'static str,
    pub fortryd: &'static str,
    pub gentag: &'static str,
    pub klip: &'static str,
    pub kopier: &'static str,
    pub saet_ind: &'static str,
    pub vaelg_alt: &'static str,
    pub vis: &'static str,
    pub tilbage: &'static str,
    pub frem: &'static str,
    pub overblik: &'static str,
    pub siden: &'static str,
    pub angreb: &'static str,
    pub robotter: &'static str,
    pub login: &'static str,
    pub land: &'static str,
    pub hent: &'static str,
    pub tekst_stoerre: &'static str,
    pub tekst_mindre: &'static str,
    pub tekst_normal: &'static str,
    pub fuld_skaerm: &'static str,
    pub vindue: &'static str,
    pub minimer: &'static str,
    pub zoom: &'static str,
    pub luk: &'static str,
}

/// Programmets navn i menuen (macOS viser bundlens navn som overskrift; teksterne nævner det).
pub const APP: &str = "Huginn and Muninn";

pub fn tekster(sprog: Sprog) -> Tekster {
    match sprog {
        Sprog::Da => Tekster {
            om: "Om Huginn and Muninn",
            indstillinger: "Indstillinger…",
            tjenester: "Tjenester",
            skjul: "Skjul Huginn and Muninn",
            skjul_andre: "Skjul andre",
            vis_alle: "Vis alle",
            afslut: "Afslut Huginn and Muninn",
            rediger: "Rediger",
            fortryd: "Fortryd",
            gentag: "Gentag",
            klip: "Klip",
            kopier: "Kopiér",
            saet_ind: "Sæt ind",
            vaelg_alt: "Vælg alt",
            vis: "Vis",
            tilbage: "Tilbage",
            frem: "Frem",
            overblik: "Overblik",
            siden: "Siden sidst",
            angreb: "Angreb",
            robotter: "Robotter",
            login: "Login",
            land: "Land lige nu",
            hent: "Hent nu",
            tekst_stoerre: "Større tekst",
            tekst_mindre: "Mindre tekst",
            tekst_normal: "Normal tekst",
            fuld_skaerm: "Fuld skærm",
            vindue: "Vindue",
            minimer: "Minimér",
            zoom: "Zoom",
            luk: "Luk vindue",
        },
        Sprog::En => Tekster {
            om: "About Huginn and Muninn",
            indstillinger: "Settings…",
            tjenester: "Services",
            skjul: "Hide Huginn and Muninn",
            skjul_andre: "Hide Others",
            vis_alle: "Show All",
            afslut: "Quit Huginn and Muninn",
            rediger: "Edit",
            fortryd: "Undo",
            gentag: "Redo",
            klip: "Cut",
            kopier: "Copy",
            saet_ind: "Paste",
            vaelg_alt: "Select All",
            vis: "View",
            tilbage: "Back",
            frem: "Forward",
            overblik: "Overview",
            siden: "Since last time",
            angreb: "Attacks",
            robotter: "Robots",
            login: "Login",
            land: "Country now",
            hent: "Fetch now",
            tekst_stoerre: "Larger Text",
            tekst_mindre: "Smaller Text",
            tekst_normal: "Normal Text",
            fuld_skaerm: "Enter Full Screen",
            vindue: "Window",
            minimer: "Minimize",
            zoom: "Zoom",
            luk: "Close Window",
        },
    }
}

/// Bygger programmenuen på sproget.
#[cfg(target_os = "macos")]
pub fn byg(app: &tauri::AppHandle, sprog: Sprog) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem as P, Submenu};
    let t = tekster(sprog);
    let punkt = |id: &str, tekst: &str, genvej: &str| MenuItem::with_id(app, id, tekst, true, Some(genvej));
    let program = Submenu::with_items(
        app,
        APP,
        true,
        &[
            &MenuItem::with_id(app, OM, t.om, true, None::<&str>)?,
            &P::separator(app)?,
            &punkt(INDSTILLINGER, t.indstillinger, GENVEJ_INDSTILLINGER)?,
            &P::separator(app)?,
            &P::services(app, Some(t.tjenester))?,
            &P::separator(app)?,
            &P::hide(app, Some(t.skjul))?,
            &P::hide_others(app, Some(t.skjul_andre))?,
            &P::show_all(app, Some(t.vis_alle))?,
            &P::separator(app)?,
            &P::quit(app, Some(t.afslut))?,
        ],
    )?;
    let rediger = Submenu::with_items(
        app,
        t.rediger,
        true,
        &[
            &P::undo(app, Some(t.fortryd))?,
            &P::redo(app, Some(t.gentag))?,
            &P::separator(app)?,
            &P::cut(app, Some(t.klip))?,
            &P::copy(app, Some(t.kopier))?,
            &P::paste(app, Some(t.saet_ind))?,
            &P::select_all(app, Some(t.vaelg_alt))?,
        ],
    )?;
    let vis = Submenu::with_items(
        app,
        t.vis,
        true,
        &[
            &punkt(TILBAGE, t.tilbage, GENVEJ_TILBAGE)?,
            &punkt(FREM, t.frem, GENVEJ_FREM)?,
            &P::separator(app)?,
            &punkt(OVERBLIK, t.overblik, GENVEJ_OVERBLIK)?,
            &punkt(SIDEN, t.siden, GENVEJ_SIDEN)?,
            &punkt(ANGREB, t.angreb, GENVEJ_ANGREB)?,
            &punkt(ROBOTTER, t.robotter, GENVEJ_ROBOTTER)?,
            &punkt(LOGIN, t.login, GENVEJ_LOGIN)?,
            &punkt(LAND, t.land, GENVEJ_LAND)?,
            &P::separator(app)?,
            &punkt(HENT, t.hent, GENVEJ_HENT)?,
            &P::separator(app)?,
            &punkt(TEKST_STOERRE, t.tekst_stoerre, GENVEJ_TEKST_STOERRE)?,
            &punkt(TEKST_MINDRE, t.tekst_mindre, GENVEJ_TEKST_MINDRE)?,
            &punkt(TEKST_NORMAL, t.tekst_normal, GENVEJ_TEKST_NORMAL)?,
            &P::separator(app)?,
            &P::fullscreen(app, Some(t.fuld_skaerm))?,
        ],
    )?;
    let vindue = Submenu::with_items(
        app,
        t.vindue,
        true,
        &[
            &P::minimize(app, Some(t.minimer))?,
            &P::maximize(app, Some(t.zoom))?,
            &P::separator(app)?,
            &P::close_window(app, Some(t.luk))?,
        ],
    )?;
    Menu::with_items(app, &[&program, &rediger, &vis, &vindue])
}

/// Webvisningen får platformen som `<html data-platform>` (og `window.__HM_PLATFORM__`), så rammen kan reservere plads
/// til trafiklysene og vælge hover og genveje. Sat af Rust, ikke gættet ud fra userAgent. Init-scriptet kører, før siden
/// har et `<html>`-element; derfor også ved `readystatechange`.
pub fn platform_skript(os: &str) -> String {
    let os = match os {
        "macos" | "windows" | "linux" => os,
        _ => "andet",
    };
    format!(
        "(function(){{var p='{os}';Object.defineProperty(window,'__HM_PLATFORM__',{{value:p}});\
         function s(){{if(document.documentElement)document.documentElement.setAttribute('data-platform',p);}}\
         s();document.addEventListener('readystatechange',s);}})();"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menutekster_paa_dansk_og_engelsk() {
        let da = tekster(Sprog::Da);
        let en = tekster(Sprog::En);
        assert_eq!((da.indstillinger, en.indstillinger), ("Indstillinger…", "Settings…"));
        assert_eq!(
            (da.vis, da.overblik, da.siden, da.angreb, da.hent),
            ("Vis", "Overblik", "Siden sidst", "Angreb", "Hent nu")
        );
        assert_eq!(
            (en.vis, en.overblik, en.siden, en.angreb, en.hent),
            ("View", "Overview", "Since last time", "Attacks", "Fetch now")
        );
        assert_eq!(
            (da.rediger, da.kopier, da.saet_ind, da.vaelg_alt),
            ("Rediger", "Kopiér", "Sæt ind", "Vælg alt")
        );
        assert_eq!(
            (en.rediger, en.kopier, en.saet_ind, en.vaelg_alt),
            ("Edit", "Copy", "Paste", "Select All")
        );
        assert_eq!((da.afslut, en.afslut), ("Afslut Huginn and Muninn", "Quit Huginn and Muninn"));
        assert_eq!((da.vindue, en.vindue), ("Vindue", "Window"));
        assert_eq!((da.tilbage, da.frem), ("Tilbage", "Frem"));
        assert_eq!((en.tilbage, en.frem), ("Back", "Forward"));
        assert_eq!((da.om, en.om), ("Om Huginn and Muninn", "About Huginn and Muninn"));
        assert_eq!(
            (da.tekst_stoerre, da.tekst_mindre, da.tekst_normal),
            ("Større tekst", "Mindre tekst", "Normal tekst")
        );
        assert_eq!(
            (en.tekst_stoerre, en.tekst_mindre, en.tekst_normal),
            ("Larger Text", "Smaller Text", "Normal Text")
        );
    }

    /// Klarsprog: "Værnet"/"Defences" står ingen steder i menuen.
    #[test]
    fn menuen_naevner_ikke_vaernet() {
        for s in [Sprog::Da, Sprog::En] {
            let alt = format!("{:?}", tekster(s));
            assert!(!alt.contains("Værn") && !alt.contains("Defence"), "{alt}");
        }
    }

    /// Navnet på visningen: kort på engelsk, så det kan stå på én linje.
    #[test]
    fn land_i_menuen() {
        assert_eq!(tekster(Sprog::Da).land, "Land lige nu");
        assert_eq!(tekster(Sprog::En).land, "Country now");
    }

    #[test]
    fn handlinger_og_genveje() {
        assert_eq!(handling(INDSTILLINGER), Some(Handling::Visning("indstillinger")));
        assert_eq!(handling(OVERBLIK), Some(Handling::Visning("overblik")));
        assert_eq!(handling(ROBOTTER), Some(Handling::Visning("robotter")));
        assert_eq!(handling(LOGIN), Some(Handling::Visning("login")));
        assert_eq!(handling(LAND), Some(Handling::Visning("land")));
        assert_eq!(handling(SIDEN), Some(Handling::Visning("siden-sidst")));
        assert_eq!(handling(ANGREB), Some(Handling::Visning("vaernet")));
        assert_eq!(handling(HENT), Some(Handling::HentNu));
        assert_eq!(handling(TEKST_STOERRE), Some(Handling::Tekst("stoerre")));
        assert_eq!(handling(TEKST_MINDRE), Some(Handling::Tekst("mindre")));
        assert_eq!(handling(TEKST_NORMAL), Some(Handling::Tekst("normal")));
        // "Om" åbner Indstillinger → Om, ikke macOS' standard-Om-boks.
        assert_eq!(handling(OM), Some(Handling::Visning("om")));
        assert_eq!(handling(TILBAGE), Some(Handling::Historik("tilbage")));
        assert_eq!(handling(FREM), Some(Handling::Historik("frem")));
        // Menulinjeikonets punkter håndteres af ikonet selv, ikke her.
        for id in [
            crate::agent::AABN,
            crate::agent::SIDEN,
            crate::agent::HENT,
            crate::agent::AFSLUT,
            "",
            "pm-",
        ] {
            assert_eq!(handling(id), None, "{id}");
        }
        assert_eq!(
            [
                GENVEJ_INDSTILLINGER,
                GENVEJ_TILBAGE,
                GENVEJ_FREM,
                GENVEJ_OVERBLIK,
                GENVEJ_SIDEN,
                GENVEJ_ANGREB,
                GENVEJ_ROBOTTER,
                GENVEJ_LOGIN,
                GENVEJ_LAND,
                GENVEJ_HENT,
                GENVEJ_TEKST_STOERRE,
                GENVEJ_TEKST_MINDRE,
                GENVEJ_TEKST_NORMAL
            ],
            [
                "CmdOrCtrl+,",
                "CmdOrCtrl+[",
                "CmdOrCtrl+]",
                "CmdOrCtrl+1",
                "CmdOrCtrl+2",
                "CmdOrCtrl+3",
                "CmdOrCtrl+4",
                "CmdOrCtrl+5",
                "CmdOrCtrl+6",
                "CmdOrCtrl+R",
                "CmdOrCtrl+NumpadAdd",
                "CmdOrCtrl+Minus",
                "CmdOrCtrl+0"
            ]
        );
    }

    #[test]
    fn platform_skript_kun_kendte_vaerdier() {
        assert!(platform_skript("macos").contains("var p='macos'"));
        assert!(platform_skript("windows").contains("var p='windows'"));
        assert!(platform_skript("linux").contains("var p='linux'"));
        assert!(
            platform_skript("x');alert(1);('").contains("var p='andet'"),
            "intet fremmed i scriptet"
        );
        assert!(platform_skript("macos").contains("setAttribute('data-platform',p)"));
    }
}
