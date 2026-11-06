//! Cloudflare-tokenet: gemmes i OS'ets nøglering (macOS Keychain / Windows Credential Manager) og forlader aldrig Rust.
//! Ingen Tauri-kommando returnerer tokenet, og `Hemmelig` skriver kun `<TOKEN>` i `Debug` og `Display`. Mens programmet
//! kører, huskes tokenet i hukommelsen (`Huske`), så nøgleringen ikke spørger ved hver runde.
use std::fmt;
use std::sync::Mutex;

use serde::{Deserialize, Deserializer};

use crate::cf::Fejl;

pub const TJENESTE: &str = "com.wi24rd.huginn-muninn";
const KONTO: &str = "cloudflare-api-token";

/// Et token, der ikke kan komme til at stå i log, fejltekster, panics eller `Debug`-output. Ingen `Serialize`.
#[derive(Clone)]
pub struct Hemmelig(String);

impl Hemmelig {
    pub fn ny(s: impl Into<String>) -> Self {
        Hemmelig(s.into().trim().to_string())
    }
    /// Kun til `Authorization`-headeren og nøgleringen.
    pub(crate) fn afsloer(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Hemmelig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<TOKEN>")
    }
}

impl fmt::Display for Hemmelig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<TOKEN>")
    }
}

impl<'de> Deserialize<'de> for Hemmelig {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d).map(Hemmelig::ny)
    }
}

/// Et token skal kunne stå i en HTTP-header: kun synlige ASCII-tegn, intet mellemrum, rimelig længde.
pub fn tjek_form(t: &Hemmelig) -> Result<(), Fejl> {
    let s = t.afsloer();
    if s.is_empty() {
        return Err(Fejl::TokenTomt);
    }
    if s.len() > 200 || !s.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Fejl::TokenForm);
    }
    Ok(())
}

fn post() -> Result<keyring::Entry, Fejl> {
    keyring::Entry::new(TJENESTE, KONTO).map_err(noeglefejl)
}

/// Nøgleringens egen fejltekst kasseres (rå bibliotekstekst må ikke nå webvisningen); kun koden `noegle` går videre.
fn noeglefejl(_: keyring::Error) -> Fejl {
    Fejl::Noegle
}

pub fn gem(t: &Hemmelig) -> Result<(), Fejl> {
    post()?.set_password(t.afsloer()).map_err(noeglefejl)
}

pub fn hent() -> Result<Option<Hemmelig>, Fejl> {
    match post()?.get_password() {
        Ok(s) => Ok(Some(Hemmelig::ny(s))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(noeglefejl(e)),
    }
}

pub fn slet() -> Result<(), Fejl> {
    match post()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(noeglefejl(e)),
    }
}

/// Tokenet i hukommelsen, mens programmet kører: fyldes ved første vellykkede læsning af
/// nøgleringen, erstattes af `token_set` og ryddes af `token_delete`. Udgaven tælles op ved hver ændring, så en læsning
/// af nøgleringen, der startede før "Glem tokenet", ikke kan lægge det gamle token tilbage. Ingen `Debug`.
#[derive(Default)]
pub struct Huske(Mutex<(u64, Option<Hemmelig>)>);

impl Huske {
    fn laas(&self) -> std::sync::MutexGuard<'_, (u64, Option<Hemmelig>)> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Tokenet, hvis det huskes, ellers udgaven, som en læsning af nøgleringen skal give tilbage til `fyld`.
    pub fn hent(&self) -> Result<Hemmelig, u64> {
        let g = self.laas();
        g.1.clone().ok_or(g.0)
    }

    /// Et token læst fra nøgleringen. Gemmes kun, hvis intet er sat eller glemt, siden læsningen startede.
    pub fn fyld(&self, udgave: u64, t: &Hemmelig) {
        let mut g = self.laas();
        if g.0 == udgave && g.1.is_none() {
            g.1 = Some(t.clone());
        }
    }

    /// Et nyt token er gemt i nøgleringen (`token_set`): det erstatter det gamle.
    pub fn saet(&self, t: Hemmelig) {
        let mut g = self.laas();
        *g = (g.0 + 1, Some(t));
    }

    /// "Glem tokenet".
    pub fn glem(&self) {
        let mut g = self.laas();
        *g = (g.0 + 1, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAA: &str = "AbCdEfGhIjKlMnOpQrStUvWxYz0123456789_-xy";

    #[test]
    fn debug_og_display_viser_aldrig_tokenet() {
        let t = Hemmelig::ny(RAA);
        assert_eq!(format!("{t:?}"), "<TOKEN>");
        assert_eq!(format!("{t}"), "<TOKEN>");
        assert_eq!(format!("{:#?}", Some(t.clone())), "Some(\n    <TOKEN>,\n)");
        assert!(!format!("{t:?} {t}").contains(RAA));
        assert_eq!(t.afsloer(), RAA);
    }

    #[test]
    fn deserialisering_pakker_ind_og_trimmer() {
        let t: Hemmelig = serde_json::from_str(&format!("\"  {RAA}\\n\"")).unwrap();
        assert_eq!(t.afsloer(), RAA);
        assert_eq!(format!("{t:?}"), "<TOKEN>");
    }

    /// Beslutning 4: tokenet i hukommelsen fyldes ved første læsning, erstattes af token_set og ryddes af
    /// token_delete; en læsning, der var i gang under "Glem tokenet", lægger det ikke tilbage.
    #[test]
    fn huske_fyldes_erstattes_og_ryddes() {
        let h = Huske::default();
        let u = h.hent().expect_err("tom ved start");
        h.fyld(u, &Hemmelig::ny("foerste"));
        assert_eq!(h.hent().unwrap().afsloer(), "foerste", "fyldt ved første læsning");
        h.fyld(u, &Hemmelig::ny("anden"));
        assert_eq!(h.hent().unwrap().afsloer(), "foerste", "en senere læsning overskriver ikke");
        h.saet(Hemmelig::ny("nyt"));
        assert_eq!(h.hent().unwrap().afsloer(), "nyt", "token_set erstatter");
        h.glem();
        let u2 = h.hent().expect_err("token_delete rydder");
        h.fyld(u, &Hemmelig::ny("gammel"));
        assert!(h.hent().is_err(), "en læsning fra før Glem tokenet lægges ikke tilbage");
        h.fyld(u2, &Hemmelig::ny("igen"));
        assert_eq!(h.hent().unwrap().afsloer(), "igen");
        let u3 = {
            h.glem();
            h.hent().unwrap_err()
        };
        h.saet(Hemmelig::ny("sat"));
        h.fyld(u3, &Hemmelig::ny("gammel"));
        assert_eq!(h.hent().unwrap().afsloer(), "sat", "en læsning fra før token_set overskriver ikke");
    }

    #[test]
    fn formtjek() {
        assert!(tjek_form(&Hemmelig::ny(RAA)).is_ok());
        assert!(tjek_form(&Hemmelig::ny("")).is_err());
        assert!(tjek_form(&Hemmelig::ny("to dele")).is_err());
        assert!(tjek_form(&Hemmelig::ny("æøå")).is_err());
        let fejl = tjek_form(&Hemmelig::ny(format!("{RAA} x"))).unwrap_err().to_string();
        assert!(!fejl.contains(RAA), "fejlteksten gentager ikke det indsatte");
    }
}
