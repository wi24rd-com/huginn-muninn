//! Den daglige start: launchd (macOS) eller Opgavestyring (Windows) starter programmet med `--dagligt` én gang i døgnet, så
//! historikken ikke får huller, når brugeren har valgt Afslut, eller maskinen ikke har været logget ud i ugevis. Kører programmet
//! allerede, henter det bare (`anden_start`). Ellers starter det uden vindue, henter og lukker igen (`skal_afslutte`). Følger
//! kontakten Start ved login (`saet`). Linux: intet i v1. Reglerne er rene funktioner; kun `saet` og `er_til` rører maskinen,
//! og ingen test kalder dem.
#[cfg(target_os = "macos")]
use std::fs;
use std::io;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::koe::Fase;

/// Den planlagte start giver programmet dette argument.
pub const DAGLIGT: &str = "--dagligt";
/// Jobbets navn i launchd, og plistens navn. Login-agenten hedder det samme uden `.dagligt`.
#[cfg(any(target_os = "macos", test))]
const LABEL: &str = "com.wi24rd.huginn-muninn.dagligt";
/// Programmets bundle-id: så står jobbet under programmet i Login-emner.
#[cfg(any(target_os = "macos", test))]
const BUNDLE_ID: &str = "com.wi24rd.huginn-muninn";
/// Opgavens navn i Opgavestyring.
#[cfg(any(windows, test))]
const OPGAVE: &str = "Wi24rd Huginn and Muninn dagligt";
/// Tidspunktet, lokal tid. Vil du prøve det: sæt det få minutter frem, byg, og slå Start ved login fra og til.
#[cfg(any(target_os = "macos", windows))]
const TIME: u8 = 12;
#[cfg(any(target_os = "macos", windows))]
const MINUT: u8 = 0;
/// En daglig start lukker senest efter så lang tid på vægur.
pub const MAKS_MS: i64 = 30 * 60_000;
/// Så ofte ser vagten på køen.
pub const TJEK: Duration = Duration::from_secs(5);
/// Så længe en zone, der er i gang, får til at blive færdig, efter at køen er bedt om at stoppe.
pub const STOP_VENT: Duration = Duration::from_secs(60);
/// Notifikationer sendes på en baggrundsopgave: så længe får de, før programmet lukker.
pub const EFTERSLAEB: Duration = Duration::from_secs(10);

/// Startede den planlagte start programmet? Første argument er programmets sti. Ren funktion.
pub fn startet_dagligt(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().skip(1).any(|a| a == DAGLIGT)
}

/// Hvad en start gør, når programmet allerede kører.
#[derive(Debug, PartialEq, Eq)]
pub enum AndenStart {
    Vindue,
    Hent,
}

/// En daglig start må aldrig åbne vinduet; alle andre starter viser det. `args` er den anden starts argumenter. Ren funktion.
pub fn anden_start(args: impl IntoIterator<Item = String>) -> AndenStart {
    if startet_dagligt(args) {
        AndenStart::Hent
    } else {
        AndenStart::Vindue
    }
}

/// Skal en daglig start lukke nu? Ja, når hele runder (`Koe::hele_runder`) er gjort, for `Fase::Faerdig` kommer før de ekstra tal.
/// Ja, når køen står: den er stoppet, eller en fejl venter på en bruger, som ikke er der. Og ja efter `MAKS_MS`. Ren funktion.
pub fn skal_afslutte(koerer: bool, fase: Option<Fase>, hele_runder: usize, forloebet_ms: i64) -> bool {
    hele_runder > 0 || !koerer || fase == Some(Fase::Fejl) || forloebet_ms >= MAKS_MS
}

/// Første afgørelse: `None`: der er intet at gøre (`tauri dev`, eller den er truffet før). `Some(true)`: slå til og notér. `Some(false)`: notér kun, for
/// er login-agenten fra, har brugeren valgt det, og så slår programmet ikke noget til. Dækker også programmer, der er installeret før den daglige start. Ren funktion.
pub fn standard(dev: bool, anvendt: Option<&str>, login_til: bool) -> Option<bool> {
    (!dev && anvendt.is_none()).then_some(login_til)
}

#[cfg(any(target_os = "macos", test))]
fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// LaunchAgenten. Uden `RunAtLoad` (ellers starter den også ved login og ved indlæsning) og uden `KeepAlive` (programmet lukker selv). Ren funktion.
#[cfg(any(target_os = "macos", test))]
pub fn dagligt_plist(sti: &str, time: u8, minut: u8) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>AssociatedBundleIdentifiers</key>
  <array><string>{BUNDLE_ID}</string></array>
  <key>ProgramArguments</key>
  <array><string>{}</string><string>{DAGLIGT}</string></array>
  <key>StartCalendarInterval</key>
  <dict>
    <key>Hour</key>
    <integer>{time}</integer>
    <key>Minute</key>
    <integer>{minut}</integer>
  </dict>
</dict>
</plist>
"#,
        xml(sti)
    )
}

/// Argumenterne til `launchctl`, der indlæser plisten nu. Ellers læses den først ved næste login. Ren funktion.
#[cfg(any(target_os = "macos", test))]
fn bootstrap_args(uid: u32, fil: &str) -> [String; 3] {
    ["bootstrap".into(), format!("gui/{uid}"), fil.into()]
}

/// Argumenterne til `launchctl`, der fjerner jobbet. Det hjælper ikke at slette filen: et indlæst job kører til næste logout. Ren funktion.
#[cfg(any(target_os = "macos", test))]
fn bootout_args(uid: u32) -> [String; 2] {
    ["bootout".into(), format!("gui/{uid}/{LABEL}")]
}

/// Argumenterne til `schtasks`, der opretter opgaven. `Command` citerer selv, så anførselstegnene om stien når frem. Ren funktion.
#[cfg(any(windows, test))]
fn schtasks_opret(sti: &str, time: u8, minut: u8) -> Vec<String> {
    let kommando = format!("\"{sti}\" {DAGLIGT}");
    let kl = format!("{time:02}:{minut:02}");
    [
        "/create", "/tn", OPGAVE, "/tr", &kommando, "/sc", "daily", "/st", &kl, "/rl", "LIMITED", "/f",
    ]
    .map(String::from)
    .to_vec()
}

/// Argumenterne til `schtasks`, der fjerner opgaven. Ren funktion.
#[cfg(any(windows, test))]
fn schtasks_slet() -> Vec<String> {
    ["/delete", "/tn", OPGAVE, "/f"].map(String::from).to_vec()
}

#[cfg(target_os = "macos")]
fn plist_fil() -> io::Result<PathBuf> {
    let hjem = std::env::var_os("HOME").ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    Ok(Path::new(&hjem).join("Library/LaunchAgents").join(format!("{LABEL}.plist")))
}

/// Fejl ignoreres: `bootstrap` fejler, hvis jobbet allerede er indlæst, og filen læses under alle omstændigheder ved næste login.
#[cfg(target_os = "macos")]
fn launchctl(args: &[String]) {
    use std::process::{Command, Stdio};
    let _ = Command::new("/bin/launchctl")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Lægger eller fjerner den daglige start. Stien findes som hos autostart-pluginnet.
#[cfg(target_os = "macos")]
pub fn saet(til: bool) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let fil = plist_fil()?;
    if til {
        let sti = std::env::current_exe()?.canonicalize()?;
        if let Some(mappe) = fil.parent() {
            fs::create_dir_all(mappe)?;
        }
        fs::write(&fil, dagligt_plist(&sti.to_string_lossy(), TIME, MINUT))?;
        launchctl(&bootstrap_args(fs::metadata(&fil)?.uid(), &fil.to_string_lossy()));
        return Ok(());
    }
    let uid = match fs::metadata(&fil) {
        Ok(m) => m.uid(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    // Er det jobbet selv, der kører, kan `bootout` lukke programmet under brugeren. Så bliver jobbet til næste logout, og `er_til` stopper det.
    if !startet_dagligt(std::env::args()) {
        launchctl(&bootout_args(uid));
    }
    fs::remove_file(&fil)
}

/// Er den daglige start slået til? Filen er sandheden: et job, launchd stadig har, efter at filen er væk, skal ikke gøre noget.
#[cfg(target_os = "macos")]
pub fn er_til() -> bool {
    plist_fil().is_ok_and(|f| f.exists())
}

/// Ikke prøvet på Windows. `/delete` afbryder ikke et kørende program, så der er ingen vagt som på macOS.
#[cfg(windows)]
pub fn saet(til: bool) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    /// CREATE_NO_WINDOW: ingen konsol, der blinker.
    const UDEN_KONSOL: u32 = 0x0800_0000;
    let args = if til {
        schtasks_opret(&std::env::current_exe()?.to_string_lossy(), TIME, MINUT)
    } else {
        schtasks_slet()
    };
    let status = Command::new("schtasks")
        .args(&args)
        .creation_flags(UDEN_KONSOL)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    // Fravalg uden en opgave er ikke en fejl.
    if til && !status.success() {
        return Err(io::Error::other("schtasks"));
    }
    Ok(())
}

#[cfg(windows)]
pub fn er_til() -> bool {
    true
}

/// Linux: intet i v1.
#[cfg(not(any(target_os = "macos", windows)))]
pub fn saet(_til: bool) -> io::Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn er_til() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn startet_dagligt_kun_med_argumentet() {
        assert!(startet_dagligt(args(&["huginn-muninn", "--dagligt"])));
        assert!(startet_dagligt(args(&["huginn-muninn", "--noget", "--dagligt"])));
        assert!(!startet_dagligt(args(&["huginn-muninn"])));
        assert!(!startet_dagligt(args(&["huginn-muninn", "--autostart"])));
        assert!(!startet_dagligt(args(&["--dagligt"])), "programmets sti tæller ikke");
        assert!(!startet_dagligt(args(&["huginn-muninn", "--dagligt=1"])));
    }

    #[test]
    fn anden_start_aabner_aldrig_vinduet_ved_dagligt() {
        assert_eq!(anden_start(args(&["huginn-muninn", "--dagligt"])), AndenStart::Hent);
        assert_eq!(anden_start(args(&["huginn-muninn"])), AndenStart::Vindue);
        assert_eq!(anden_start(args(&["huginn-muninn", "--autostart"])), AndenStart::Vindue);
    }

    #[test]
    fn afslutter_naar_koeen_er_faerdig_eller_tiden_er_gaaet() {
        for fase in [Fase::Henter, Fase::Pause, Fase::VenterKvote] {
            assert!(!skal_afslutte(true, Some(fase), 0, 0), "{fase:?}: køen arbejder");
        }
        assert!(!skal_afslutte(true, None, 0, 0), "før første melding");
        assert!(!skal_afslutte(true, Some(Fase::Faerdig), 0, 60_000), "de ekstra tal hentes stadig");
        assert!(skal_afslutte(true, Some(Fase::Faerdig), 1, 60_000), "runde og ekstra tal er gjort");
        assert!(skal_afslutte(true, Some(Fase::Fejl), 0, 0), "fejlen venter på en bruger");
        assert!(skal_afslutte(false, Some(Fase::Fejl), 0, 0), "intet token: køen er stoppet");
        assert!(skal_afslutte(false, None, 0, 0));
        assert!(!skal_afslutte(true, Some(Fase::Henter), 0, MAKS_MS - 1));
        assert!(skal_afslutte(true, Some(Fase::Henter), 0, MAKS_MS), "højst 30 minutter");
        assert_eq!(MAKS_MS, 1_800_000);
    }

    #[test]
    fn afgoeres_een_gang_og_slaas_kun_til_med_login_agenten() {
        assert_eq!(
            standard(false, None, true),
            Some(true),
            "ny installation og opgradering med login til"
        );
        assert_eq!(
            standard(false, None, false),
            Some(false),
            "brugeren har valgt login fra: kun noteret"
        );
        assert_eq!(standard(true, None, true), None, "tauri dev");
        assert_eq!(standard(false, Some("1"), true), None, "afgjort før: brugerens valg gælder");
    }

    #[test]
    fn plisten_har_det_den_skal_og_intet_andet() {
        let p = dagligt_plist("/Programmer/Huginn and Muninn.app/Contents/MacOS/huginn-muninn", 12, 0);
        assert!(p.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist"));
        assert!(p.contains("<key>Label</key>\n  <string>com.wi24rd.huginn-muninn.dagligt</string>"));
        assert!(p.contains(
            "<array><string>/Programmer/Huginn and Muninn.app/Contents/MacOS/huginn-muninn</string><string>--dagligt</string></array>"
        ));
        assert!(p.contains("<key>StartCalendarInterval</key>"));
        assert!(p.contains("<key>Hour</key>\n    <integer>12</integer>"));
        assert!(p.contains("<key>Minute</key>\n    <integer>0</integer>"));
        assert!(p.contains("<array><string>com.wi24rd.huginn-muninn</string></array>"));
        assert!(!p.contains("RunAtLoad"), "ellers starter den også ved login");
        assert!(!p.contains("KeepAlive"), "ellers genstarter launchd programmet, når det lukker");
    }

    /// Hver åbnende tag lukkes i rækkefølge, og der står ingen rå `&` eller `<` i teksten.
    fn velformet(xml: &str) -> bool {
        let krop = xml.split_once("<plist").map(|(_, r)| format!("<plist{r}")).unwrap_or_default();
        let mut stak: Vec<String> = Vec::new();
        let mut rest = krop.as_str();
        while let Some(start) = rest.find('<') {
            if rest[..start].contains('&') && !rest[..start].contains("&amp;") {
                return false;
            }
            let Some(slut) = rest[start..].find('>') else { return false };
            let tag = &rest[start + 1..start + slut];
            if tag.contains('<') {
                return false;
            }
            rest = &rest[start + slut + 1..];
            if let Some(navn) = tag.strip_prefix('/') {
                if stak.pop().as_deref() != Some(navn) {
                    return false;
                }
            } else if !tag.ends_with('/') {
                stak.push(tag.split_whitespace().next().unwrap_or_default().to_string());
            }
        }
        stak.is_empty() && !krop.is_empty()
    }

    #[test]
    fn plisten_er_gyldig_xml_ogsaa_med_saere_tegn_i_stien() {
        assert!(velformet(&dagligt_plist("/Programmer/Huginn.app/huginn-muninn", 12, 0)));
        let p = dagligt_plist("/Brugere/A&B <test>/Huginn.app/huginn-muninn", 12, 0);
        assert!(velformet(&p), "{p}");
        assert!(p.contains("/Brugere/A&amp;B &lt;test&gt;/Huginn.app/huginn-muninn"));
        assert!(
            !velformet("<plist><dict><string>A&B</string></dict></plist>"),
            "tjekket fanger en rå &"
        );
        assert!(!velformet("<plist><dict></plist>"), "tjekket fanger en manglende slut-tag");
    }

    #[test]
    fn launchctl_argumenter() {
        assert_eq!(
            bootstrap_args(501, "/Users/x/Library/LaunchAgents/a.plist"),
            ["bootstrap", "gui/501", "/Users/x/Library/LaunchAgents/a.plist"]
        );
        assert_eq!(bootout_args(501), ["bootout", "gui/501/com.wi24rd.huginn-muninn.dagligt"]);
    }

    #[test]
    fn schtasks_argumenter() {
        assert_eq!(
            schtasks_opret(r"C:\Program Files\Huginn\huginn-muninn.exe", 12, 0),
            [
                "/create",
                "/tn",
                "Wi24rd Huginn and Muninn dagligt",
                "/tr",
                r#""C:\Program Files\Huginn\huginn-muninn.exe" --dagligt"#,
                "/sc",
                "daily",
                "/st",
                "12:00",
                "/rl",
                "LIMITED",
                "/f"
            ]
        );
        assert_eq!(schtasks_opret("x", 7, 5)[8], "07:05", "HH:mm");
        assert_eq!(schtasks_slet(), ["/delete", "/tn", "Wi24rd Huginn and Muninn dagligt", "/f"]);
    }
}
