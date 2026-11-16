//! Cloudflare-tokens: gemmes i OS'ets nøglering (macOS Keychain / Windows Credential Manager) og forlader aldrig Rust.
//! Ingen Tauri-kommando returnerer et token, og `Hemmelig` skriver kun `<TOKEN>` i `Debug` og `Display`. Mens programmet
//! kører, huskes tokenerne i hukommelsen (`Huske`), så nøgleringen ikke spørger ved hver runde.
//!
//! Én post pr. forbindelse (V-KONTI.1): forbindelse 1 bruger den oprindelige post `cloudflare-api-token` uændret (den flyttes og omdøbes aldrig, så en
//! opgradering fra 1.0.0 hverken skal have tokenet igen eller giver en ny dialog), og forbindelse n ≥ 2 bruger `cloudflare-api-token-<n>`. Nøgleringen kan
//! ikke liste sine poster; listen er tabellen `forbindelse` (`store.rs`).
use std::collections::HashMap;
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

/// Postens navn (keyrings `user`) for forbindelsen. Ren funktion.
pub fn konto(forbindelse: i64) -> String {
    if forbindelse == 1 {
        KONTO.to_string()
    } else {
        format!("{KONTO}-{forbindelse}")
    }
}

fn post(forbindelse: i64) -> Result<keyring::Entry, Fejl> {
    keyring::Entry::new(TJENESTE, &konto(forbindelse)).map_err(noeglefejl)
}

/// Nøgleringens egen fejltekst kasseres (rå bibliotekstekst må ikke nå webvisningen); kun koden `noegle` går videre.
fn noeglefejl(_: keyring::Error) -> Fejl {
    Fejl::Noegle
}

pub fn gem(forbindelse: i64, t: &Hemmelig) -> Result<(), Fejl> {
    post(forbindelse)?.set_password(t.afsloer()).map_err(noeglefejl)
}

pub fn hent(forbindelse: i64) -> Result<Option<Hemmelig>, Fejl> {
    match post(forbindelse)?.get_password() {
        Ok(s) => Ok(Some(Hemmelig::ny(s))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(noeglefejl(e)),
    }
}

pub fn slet(forbindelse: i64) -> Result<(), Fejl> {
    match post(forbindelse)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(noeglefejl(e)),
    }
}

/// Tokenerne i hukommelsen pr. forbindelse, mens programmet kører: fyldes ved første vellykkede læsning af nøgleringen, erstattes
/// af et nyt token og ryddes af "Glem". Udgaven tælles op pr. forbindelse ved hver ændring, så en læsning af nøgleringen, der startede
/// før "Glem", ikke kan lægge det gamle token tilbage. "Glem" beholder forbindelsens tæller (uden token), så den aldrig starter forfra. Ingen `Debug`.
#[derive(Default)]
pub struct Huske(Mutex<HashMap<i64, (u64, Option<Hemmelig>)>>);

impl Huske {
    fn laas(&self) -> std::sync::MutexGuard<'_, HashMap<i64, (u64, Option<Hemmelig>)>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Forbindelsens token, hvis det huskes, ellers udgaven, som en læsning af nøgleringen skal give tilbage til `fyld`.
    pub fn hent(&self, forbindelse: i64) -> Result<Hemmelig, u64> {
        let g = self.laas();
        match g.get(&forbindelse) {
            Some((_, Some(t))) => Ok(t.clone()),
            Some((u, None)) => Err(*u),
            None => Err(0),
        }
    }

    /// Et token læst fra nøgleringen. Gemmes kun, hvis intet er sat eller glemt for forbindelsen, siden læsningen startede.
    pub fn fyld(&self, forbindelse: i64, udgave: u64, t: &Hemmelig) {
        let mut g = self.laas();
        let post = g.entry(forbindelse).or_insert((0, None));
        if post.0 == udgave && post.1.is_none() {
            post.1 = Some(t.clone());
        }
    }

    /// Et nyt token er gemt i nøgleringen for forbindelsen: det erstatter det gamle.
    pub fn saet(&self, forbindelse: i64, t: Hemmelig) {
        let mut g = self.laas();
        let post = g.entry(forbindelse).or_insert((0, None));
        *post = (post.0 + 1, Some(t));
    }

    /// "Glem" forbindelsen: tokenet væk, tælleren op og bevaret.
    pub fn glem(&self, forbindelse: i64) {
        let mut g = self.laas();
        let post = g.entry(forbindelse).or_insert((0, None));
        *post = (post.0 + 1, None);
    }

    /// Et token i hukommelsen fra en af `forbindelser` (den første i rækkefølgen, der huskes), uden at spørge nøgleringen.
    pub fn et_af(&self, forbindelser: &[i64]) -> Option<Hemmelig> {
        let g = self.laas();
        forbindelser.iter().find_map(|f| g.get(f).and_then(|p| p.1.clone()))
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

    /// Forbindelse 1 er den oprindelige post fra 1.0.0; de andre har deres eget nummer.
    #[test]
    fn posterne_pr_forbindelse() {
        assert_eq!(konto(1), "cloudflare-api-token");
        assert_eq!(konto(2), "cloudflare-api-token-2");
        assert_eq!(konto(17), "cloudflare-api-token-17");
    }

    /// Beslutning 4 (nu pr. forbindelse): tokenet i hukommelsen fyldes ved første læsning, erstattes af et nyt token og ryddes af
    /// "Glem"; en læsning, der var i gang under "Glem", lægger det ikke tilbage.
    #[test]
    fn huske_fyldes_erstattes_og_ryddes() {
        let h = Huske::default();
        let u = h.hent(1).expect_err("tom ved start");
        h.fyld(1, u, &Hemmelig::ny("foerste"));
        assert_eq!(h.hent(1).unwrap().afsloer(), "foerste", "fyldt ved første læsning");
        h.fyld(1, u, &Hemmelig::ny("anden"));
        assert_eq!(h.hent(1).unwrap().afsloer(), "foerste", "en senere læsning overskriver ikke");
        h.saet(1, Hemmelig::ny("nyt"));
        assert_eq!(h.hent(1).unwrap().afsloer(), "nyt", "et nyt token erstatter");
        h.glem(1);
        let u2 = h.hent(1).expect_err("Glem rydder");
        h.fyld(1, u, &Hemmelig::ny("gammel"));
        assert!(h.hent(1).is_err(), "en læsning fra før Glem lægges ikke tilbage");
        h.fyld(1, u2, &Hemmelig::ny("igen"));
        assert_eq!(h.hent(1).unwrap().afsloer(), "igen");
        let u3 = {
            h.glem(1);
            h.hent(1).unwrap_err()
        };
        h.saet(1, Hemmelig::ny("sat"));
        h.fyld(1, u3, &Hemmelig::ny("gammel"));
        assert_eq!(
            h.hent(1).unwrap().afsloer(),
            "sat",
            "en læsning fra før et nyt token overskriver ikke"
        );
    }

    /// V-KONTI.1: hver forbindelse har sin egen tæller. "Glem" én lægger ikke en andens læsning væk, og en læsning fra før "Glem"
    /// lægges ikke tilbage, heller ikke efter at forbindelsen er glemt to gange.
    #[test]
    fn huske_pr_forbindelse_laesning_foer_glem_laegges_ikke_tilbage() {
        let h = Huske::default();
        let (u1, u2) = (h.hent(1).unwrap_err(), h.hent(2).unwrap_err());
        h.glem(2);
        h.fyld(1, u1, &Hemmelig::ny("en"));
        assert_eq!(h.hent(1).unwrap().afsloer(), "en", "Glem 2 rører ikke en læsning af 1");
        h.fyld(2, u2, &Hemmelig::ny("to-gammel"));
        assert!(h.hent(2).is_err(), "læsningen af 2 startede før Glem");
        let u2b = h.hent(2).unwrap_err();
        h.glem(2);
        h.fyld(2, u2b, &Hemmelig::ny("to-gammel"));
        assert!(h.hent(2).is_err(), "tælleren starter ikke forfra efter Glem");
        h.saet(2, Hemmelig::ny("to"));
        assert_eq!(h.hent(2).unwrap().afsloer(), "to");
        assert_eq!(h.et_af(&[3, 2, 1]).unwrap().afsloer(), "to", "den første huskede i rækkefølgen");
        h.glem(1);
        h.glem(2);
        assert!(h.et_af(&[1, 2]).is_none());
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
