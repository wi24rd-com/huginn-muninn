//! Start ved login på macOS: programmet skriver selv login-plisten, som det skriver den daglige (`dagligt`). Samme fil og `Label`
//! som `tauri-plugin-autostart` brugte, så installationer fra før bliver ved med at virke, men med bundle-id og en XML-escapet sti.
//! Ved hver start i en bundlet build retter programmet begge plister, hvis programmet er flyttet (`ret_stier`). Windows og Linux
//! bruger stadig pluginnet. Reglerne er rene funktioner; kun `saet`, `er_til` og `ret_stier` rører maskinen, og ingen test kalder dem.
#[cfg(any(target_os = "macos", test))]
use std::fs;
#[cfg(any(target_os = "macos", test))]
use std::io;
#[cfg(any(target_os = "macos", test))]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::path::PathBuf;

#[cfg(any(target_os = "macos", test))]
use crate::dagligt::{xml, BUNDLE_ID};
#[cfg(any(target_os = "macos", test))]
use crate::VED_LOGIN;

/// Login-jobbets navn i launchd og plistens navn: det samme, som pluginnet fik som `app_name`.
#[cfg(any(target_os = "macos", test))]
const LABEL: &str = BUNDLE_ID;

/// Login-agenten: `RunAtLoad`, så launchd starter programmet ved login. Ingen `KeepAlive`: Afslut skal blive ved med at gælde. Ren funktion.
#[cfg(any(target_os = "macos", test))]
pub fn login_plist(sti: &str) -> String {
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
  <array><string>{}</string><string>{VED_LOGIN}</string></array>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#,
        xml(sti)
    )
}

/// Må selvreparationen skrive denne sti i en plist? Ikke fra en App Translocation-mappe (en usigneret app, startet fra
/// Overførsler, kører fra en tilfældig, midlertidig sti), ikke fra et diskbillede eller et andet drev under `/Volumes/`, og
/// ikke fra en debug-sti uden for en `.app`. Ren funktion.
#[cfg(any(target_os = "macos", test))]
pub fn fast_sti(sti: &str) -> bool {
    !sti.contains("/AppTranslocation/") && !sti.starts_with("/Volumes/") && sti.contains(".app/Contents/MacOS/")
}

/// Skal en plist, der peger på `gemt`, skrives igen, nu hvor programmet ligger på `ny`? Kun når stien er en anden, og den nye er fast. Ren funktion.
#[cfg(any(target_os = "macos", test))]
pub fn skal_skrive_igen(gemt: &str, ny: &str) -> bool {
    gemt != ny && fast_sti(ny)
}

/// Første `<string>` efter `<key>ProgramArguments</key>`: programmets sti, som plisten har den. Ren funktion.
#[cfg(any(target_os = "macos", test))]
pub fn gemt_sti(tekst: &str) -> Option<String> {
    let efter = &tekst[tekst.find("<key>ProgramArguments</key>")?..];
    let start = efter.find("<string>")? + "<string>".len();
    let slut = start + efter[start..].find("</string>")?;
    Some(efter[start..slut].replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&"))
}

/// Skrev pluginnet plisten? Dens `AssociatedBundleIdentifiers` er tom; programmets egen har bundle-id'et. Ren funktion.
#[cfg(any(target_os = "macos", test))]
pub fn fra_pluginnet(tekst: &str) -> bool {
    let Some(i) = tekst.find("<key>AssociatedBundleIdentifiers</key>") else {
        return true;
    };
    let efter = &tekst[i..];
    let liste = &efter[..efter.find("</array>").unwrap_or(efter.len())];
    !liste.contains(&format!("<string>{BUNDLE_ID}</string>"))
}

/// Hvad selvreparationen skriver i login-plisten. `None`: lad den være. Findes filen ikke (`gemt` er `None`), har brugeren slået
/// Start ved login fra, og så oprettes intet. Ellers skrives den igen, når programmet er flyttet, eller når pluginnet skrev den
/// (første start efter opdateringen), men aldrig fra en sti, der ikke er fast. Ren funktion.
#[cfg(any(target_os = "macos", test))]
pub fn login_ny_tekst(gemt: Option<&str>, ny: &str) -> Option<String> {
    let gemt = gemt?;
    let flyttet = skal_skrive_igen(gemt_sti(gemt).as_deref().unwrap_or_default(), ny);
    let migrer = fra_pluginnet(gemt) && fast_sti(ny);
    (flyttet || migrer).then(|| login_plist(ny))
}

/// Læser `fil` og skriver den igen, hvis `login_ny_tekst` siger det. En fil, der ikke findes, forbliver væk. Svarer, om der blev skrevet.
#[cfg(any(target_os = "macos", test))]
fn ret_login(fil: &Path, ny: &str) -> io::Result<bool> {
    let gemt = match fs::read_to_string(fil) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let Some(tekst) = login_ny_tekst(gemt.as_deref(), ny) else {
        return Ok(false);
    };
    fs::write(fil, tekst)?;
    Ok(true)
}

#[cfg(target_os = "macos")]
fn plist_fil() -> io::Result<PathBuf> {
    let hjem = std::env::var_os("HOME").ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    Ok(Path::new(&hjem).join("Library/LaunchAgents").join(format!("{LABEL}.plist")))
}

/// Programmets egen sti, som launchd skal starte. Samme regel som pluginnet: `current_exe` uden symlinks.
#[cfg(target_os = "macos")]
fn egen_sti() -> io::Result<String> {
    Ok(std::env::current_exe()?.canonicalize()?.to_string_lossy().into_owned())
}

/// Slår start ved login til eller fra. Indlæses ikke nu (`RunAtLoad` ville starte programmet en gang til); launchd læser filen
/// ved næste login, som da pluginnet skrev den. Et fravalg uden en fil er ikke en fejl.
#[cfg(target_os = "macos")]
pub fn saet(til: bool) -> io::Result<()> {
    let fil = plist_fil()?;
    if til {
        let sti = egen_sti()?;
        if let Some(mappe) = fil.parent() {
            fs::create_dir_all(mappe)?;
        }
        return fs::write(&fil, login_plist(&sti));
    }
    match fs::remove_file(&fil) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        r => r,
    }
}

/// Er start ved login slået til? Filen er sandheden, som hos pluginnet (IDEA-015: et fravalg i Login-emner ses ikke her).
#[cfg(target_os = "macos")]
pub fn er_til() -> bool {
    plist_fil().is_ok_and(|f| f.exists())
}

/// Ved hver start i en bundlet build: peger login- eller dagligt-plisten et andet sted hen end programmet, skrives de igen.
/// Login-plisten fra pluginnet skrives om i samme omgang. En plist, der ikke findes, oprettes ikke. Fejl ignoreres af kalderen.
#[cfg(target_os = "macos")]
pub fn ret_stier() -> io::Result<()> {
    let ny = egen_sti()?;
    let login = ret_login(&plist_fil()?, &ny);
    let daglig = crate::dagligt::ret_sti(&ny);
    login.and(daglig).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    const STI: &str = "/Applications/Huginn and Muninn.app/Contents/MacOS/huginn-muninn";

    /// Plisten, som `tauri-plugin-autostart` 2.7.0 / `auto-launch` 0.6.0 skrev den (kopieret fra `~/Library/LaunchAgents`, stien er byttet).
    const PLUGIN_PLIST: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
  <dict>
  <key>Label</key>
  <string>com.wi24rd.huginn-muninn</string>
  <key>AssociatedBundleIdentifiers</key>
  <array></array>
  <key>ProgramArguments</key>
  <array><string>/Applications/Huginn and Muninn.app/Contents/MacOS/huginn-muninn</string><string>--autostart</string></array>
  <key>RunAtLoad</key>
  <true/>
  </dict>
</plist>";

    /// Hver åbnende tag lukkes i rækkefølge, og der står ingen rå `&` eller `<` i teksten (samme tjek som i `dagligt`).
    fn velformet(xml: &str) -> bool {
        let krop = xml.split_once("<plist").map(|(_, r)| format!("<plist{r}")).unwrap_or_default();
        let mut stak: Vec<String> = Vec::new();
        let mut rest = krop.as_str();
        while let Some(start) = rest.find('<') {
            let tekst = &rest[..start];
            if tekst.replace("&amp;", "").replace("&lt;", "").replace("&gt;", "").contains('&') {
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

    /// Nøglerne i plistens øverste `<dict>`, i rækkefølge.
    fn noegler(p: &str) -> Vec<String> {
        p.match_indices("<key>")
            .map(|(i, _)| p[i + 5..i + 5 + p[i + 5..].find("</key>").unwrap()].to_string())
            .collect()
    }

    #[test]
    fn login_plisten_har_det_den_skal_og_intet_andet() {
        let p = login_plist(STI);
        assert!(p.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist"));
        assert_eq!(
            noegler(&p),
            ["Label", "AssociatedBundleIdentifiers", "ProgramArguments", "RunAtLoad"]
        );
        assert!(p.contains("<key>Label</key>\n  <string>com.wi24rd.huginn-muninn</string>"));
        assert!(p.contains("<key>AssociatedBundleIdentifiers</key>\n  <array><string>com.wi24rd.huginn-muninn</string></array>"));
        assert!(p.contains(&format!(
            "<key>ProgramArguments</key>\n  <array><string>{STI}</string><string>--autostart</string></array>"
        )));
        assert!(p.contains("<key>RunAtLoad</key>\n  <true/>"));
        assert!(!p.contains("KeepAlive"), "ellers genstarter launchd programmet efter Afslut");
        assert!(!p.contains("StartCalendarInterval"));
        assert!(!fra_pluginnet(&p), "programmets egen plist har bundle-id'et");
    }

    #[test]
    fn login_plisten_har_samme_navn_og_argument_som_pluginnet() {
        assert_eq!(LABEL, "com.wi24rd.huginn-muninn");
        assert!(PLUGIN_PLIST.contains(&format!("<string>{LABEL}</string>")));
        assert!(PLUGIN_PLIST.contains(&format!("<string>{VED_LOGIN}</string>")));
    }

    #[test]
    fn login_plisten_er_gyldig_xml_ogsaa_med_saere_tegn_i_stien() {
        assert!(velformet(&login_plist(STI)));
        for sti in [
            "/Users/a/A&B/Huginn and Muninn.app/Contents/MacOS/huginn-muninn",
            "/Users/a/<test>/Huginn.app/Contents/MacOS/huginn-muninn",
            "/Users/a/Mine programmer/x & y <z>/Huginn.app/Contents/MacOS/huginn-muninn",
        ] {
            let p = login_plist(sti);
            assert!(velformet(&p), "{p}");
            assert_eq!(gemt_sti(&p).as_deref(), Some(sti), "stien kommer uændret tilbage");
        }
        assert!(login_plist("/a/A&B/x").contains("<string>/a/A&amp;B/x</string>"));
        assert!(login_plist("/a/<b>/x").contains("<string>/a/&lt;b&gt;/x</string>"));
        assert!(!velformet(&PLUGIN_PLIST.replace(STI, "/a/A&B/x")), "tjekket fanger pluginnets rå &");
        assert!(!velformet("<plist><dict></plist>"), "tjekket fanger en manglende slut-tag");
    }

    #[test]
    fn faste_stier() {
        assert!(fast_sti(STI));
        assert!(fast_sti("/Users/jan/Programmer/Huginn and Muninn.app/Contents/MacOS/huginn-muninn"));
        assert!(!fast_sti(
            "/private/var/folders/xy/abc/T/AppTranslocation/0F1E2D3C-AAAA-BBBB-CCCC-123456789ABC/d/Huginn and Muninn.app/Contents/MacOS/huginn-muninn"
        ));
        assert!(!fast_sti(
            "/Volumes/Huginn and Muninn/Huginn and Muninn.app/Contents/MacOS/huginn-muninn"
        ));
        assert!(
            !fast_sti("/Users/jan/Huginn-Muninn/src-tauri/target/debug/huginn-muninn"),
            "tauri dev"
        );
    }

    #[test]
    fn skal_skrive_igen_kun_naar_programmet_er_flyttet_til_en_fast_sti() {
        let flyttet = "/Users/jan/Programmer/Huginn and Muninn.app/Contents/MacOS/huginn-muninn";
        assert!(!skal_skrive_igen(STI, STI), "samme sti");
        assert!(skal_skrive_igen(STI, flyttet), "flyttet");
        assert!(
            !skal_skrive_igen(
                STI,
                "/private/var/folders/xy/abc/T/AppTranslocation/0F1E/d/Huginn and Muninn.app/Contents/MacOS/huginn-muninn"
            ),
            "AppTranslocation"
        );
        assert!(
            !skal_skrive_igen(STI, "/Volumes/Huginn/Huginn and Muninn.app/Contents/MacOS/huginn-muninn"),
            "diskbillede"
        );
        assert!(
            !skal_skrive_igen(STI, "/Users/jan/Huginn-Muninn/src-tauri/target/debug/huginn-muninn"),
            "debug-sti uden for en .app"
        );
    }

    #[test]
    fn gemt_sti_laeser_programmets_sti() {
        assert_eq!(gemt_sti(PLUGIN_PLIST).as_deref(), Some(STI));
        assert_eq!(gemt_sti(&login_plist(STI)).as_deref(), Some(STI));
        assert_eq!(gemt_sti(&crate::dagligt::dagligt_plist(STI, 12, 0)).as_deref(), Some(STI));
        assert_eq!(gemt_sti("<plist><dict></dict></plist>"), None);
        assert_eq!(gemt_sti(""), None);
    }

    #[test]
    fn migrering_pluginnets_plist_genkendes_og_skrives_igen() {
        assert!(fra_pluginnet(PLUGIN_PLIST));
        assert_eq!(
            login_ny_tekst(Some(PLUGIN_PLIST), STI),
            Some(login_plist(STI)),
            "samme sti, men pluginnets form: skrives igen"
        );
        let flyttet = "/Users/jan/Programmer/Huginn and Muninn.app/Contents/MacOS/huginn-muninn";
        assert_eq!(login_ny_tekst(Some(PLUGIN_PLIST), flyttet), Some(login_plist(flyttet)));
        assert_eq!(
            login_ny_tekst(
                Some(PLUGIN_PLIST),
                "/private/var/folders/x/T/AppTranslocation/U/d/Huginn.app/Contents/MacOS/huginn-muninn"
            ),
            None,
            "aldrig fra AppTranslocation, heller ikke ved migrering"
        );
        let ny = login_ny_tekst(Some(PLUGIN_PLIST), STI).unwrap();
        assert_eq!(login_ny_tekst(Some(&ny), STI), None, "anden start: intet at gøre");
    }

    #[test]
    fn selvreparation_skriver_kun_naar_det_er_noedvendigt() {
        let egen = login_plist(STI);
        let flyttet = "/Users/jan/Programmer/Huginn and Muninn.app/Contents/MacOS/huginn-muninn";
        assert_eq!(login_ny_tekst(Some(&egen), STI), None, "samme sti");
        assert_eq!(login_ny_tekst(Some(&egen), flyttet), Some(login_plist(flyttet)), "flyttet");
        assert_eq!(
            login_ny_tekst(Some(&egen), "/Volumes/H/Huginn.app/Contents/MacOS/huginn-muninn"),
            None
        );
        assert_eq!(login_ny_tekst(Some(&egen), "/Users/jan/x/target/debug/huginn-muninn"), None);
        assert_eq!(
            login_ny_tekst(Some("noget helt andet"), STI),
            Some(egen.clone()),
            "en ødelagt fil under programmets navn skrives igen"
        );
    }

    #[test]
    fn fravalg_staar_ved_magt() {
        assert_eq!(login_ny_tekst(None, STI), None, "ingen fil: intet oprettes");
        let mappe = std::env::temp_dir().join(format!("hm-ved-login-{}", std::process::id()));
        fs::create_dir_all(&mappe).unwrap();
        let fil = mappe.join("com.wi24rd.huginn-muninn.plist");
        let _ = fs::remove_file(&fil);
        assert!(!ret_login(&fil, STI).unwrap());
        assert!(!fil.exists(), "selvreparationen opretter ingen plist");
        fs::write(&fil, PLUGIN_PLIST).unwrap();
        assert!(ret_login(&fil, STI).unwrap(), "pluginnets plist skrives om");
        assert_eq!(fs::read_to_string(&fil).unwrap(), login_plist(STI));
        assert!(!ret_login(&fil, STI).unwrap(), "og kun én gang");
        fs::remove_dir_all(&mappe).unwrap();
    }
}
