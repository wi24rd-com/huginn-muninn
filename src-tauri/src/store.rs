//! Lokal cache i SQLite (app-datamappen). Kun aggregater pr. døgn, som D1-tabellen vagt_dag i W, plus hvornår hvert
//! døgn sidst blev hentet, den senest hentede zoneliste (så forsiden virker uden net) og de domæner, brugeren har valgt
//! (`valgt`; tom = ingen), plus de nye domæner, der venter på et svar (`ny_zone`, `afvist_ny`), plus programmets egne indstillinger (`indstillinger`), notifikationsvalget pr. domæne og hvad der er
//! meldt (`notifikation_zone`, `meldt`), login-stierne pr. domæne (`login_sti`) og hvornår de ekstra tal for et døgn sidst blev
//! hentet (`hentet_ekstra`) og de daglige tal fra Cloudflare Radar (`radar_dag`). Intet af Værnets tal slettes efter 95 døgn: Muninn husker. `vagt_dag` har to slags arter, som hver
//! sin hentning ejer: `DOEGN_ARTER` (`gem_doegn`, hentes igen hver time) og `EKSTRA_ARTER` (`gem_ekstra`, kun færdige døgn).
use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::{params, params_from_iter, Connection};
use serde::Serialize;

use crate::cf::{Fejl, Zone};

/// 1: vagt_dag + hentet. 2: + zoner. 3: + valg. 4: + indstillinger. 5: + notifikation_zone, meldt. 6: + login_sti, hentet_ekstra.
/// 7: + radar_dag. 8: valg → fravalgt. 9: fravalgt → valgt, + afvist_ny, ny_zone. 10: + teknologi, Radar-serier uden brug væk.
/// 11: + hentet_angreb (art `angreb`). 12: + vurderet, hentet_angreb tømt, så angrebene hentes igen med netværket. 13: + hentet_historik, hentet_laek, zone_tilstand.
/// 14: + tek_vis. 15: + hentet_dns, hentet_dmarc. 16: + sikkerhedsfund. 17: + hentet_besoeg. 18: + hentet_robot, hentet_side.
const SKEMA_VERSION: i64 = 18;
const SKEMA_1: &str = "
CREATE TABLE IF NOT EXISTS vagt_dag (zone TEXT NOT NULL, dag TEXT NOT NULL, art TEXT NOT NULL, noegle TEXT NOT NULL, n INTEGER NOT NULL, PRIMARY KEY (zone, dag, art, noegle));
CREATE TABLE IF NOT EXISTS hentet (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
";
const SKEMA_2: &str = "
CREATE TABLE IF NOT EXISTS zoner (id TEXT PRIMARY KEY, navn TEXT NOT NULL, hentet_utc TEXT NOT NULL);
";
const SKEMA_3: &str = "
CREATE TABLE IF NOT EXISTS valg (zone TEXT PRIMARY KEY);
";
const SKEMA_4: &str = "
CREATE TABLE IF NOT EXISTS indstillinger (noegle TEXT PRIMARY KEY, vaerdi TEXT NOT NULL);
";
/// Pr. domæne: 'alt' | 'roed' | 'fra' (ingen række = følg forudindstillingen). `meldt`: højst én pr. domæne pr. statusdøgn,
/// med det højeste meldte niveau ('groen' | 'gul' | 'roed'), og '+maerke', når et mærke er meldt (`notify::beslut`).
const SKEMA_5: &str = "
CREATE TABLE IF NOT EXISTS notifikation_zone (zone TEXT PRIMARY KEY, valg TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS meldt (zone TEXT NOT NULL, dag TEXT NOT NULL, niveau TEXT NOT NULL, kl TEXT NOT NULL, PRIMARY KEY (zone, dag));
";
/// `login_sti`: en sti brugeren har slået til (`til` = 1) eller fra pr. domæne. `hentet_ekstra`: som `hentet`, men for `EKSTRA_ARTER`.
const SKEMA_6: &str = "
CREATE TABLE IF NOT EXISTS login_sti (zone TEXT NOT NULL, sti TEXT NOT NULL, til INTEGER NOT NULL, PRIMARY KEY (zone, sti));
CREATE TABLE IF NOT EXISTS hentet_ekstra (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
";
/// `radar_dag`: ét tal pr. serie, land, døgn og nøgle, som Radar svarede, med normaliseringens navn og hvornår det blev hentet.
/// `land` er tomt for verden. `MIN0_MAX` er en andel af det største tal i ét svar, så kun tal med samme `hentet_utc` står på samme
/// skala (`radar::verden_serie`).
const SKEMA_7: &str = "
CREATE TABLE IF NOT EXISTS radar_dag (serie TEXT NOT NULL, land TEXT NOT NULL, dag TEXT NOT NULL, noegle TEXT NOT NULL, vaerdi REAL NOT NULL, normalisering TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (serie, land, dag, noegle));
";
/// `fravalgt`: domæner, brugeren har slået fra. Listen er vendt om fra `valg`, som gemte de valgte, så en ny zone er til uden videre. Et gemt
/// valg bliver til det modsatte: det, der stod i `zoner`, men ikke i `valg`. Uden markøren '*' var intet gemt, og et tomt valg skjulte
/// ingen domæner, så i begge tilfælde er intet slået fra.
const SKEMA_8: &str = "
CREATE TABLE IF NOT EXISTS fravalgt (zone TEXT PRIMARY KEY);
INSERT OR IGNORE INTO fravalgt (zone) SELECT id FROM zoner
  WHERE EXISTS (SELECT 1 FROM valg WHERE zone = '*') AND EXISTS (SELECT 1 FROM valg WHERE zone <> '*') AND id NOT IN (SELECT zone FROM valg);
DROP TABLE IF EXISTS valg;
";
/// Domæner er et tilvalg: `valgt` er dem, brugeren vil se (tom = ingen). `ny_zone` er domæner, der dukkede op i en senere zoneliste, og som venter på et
/// svar: i næste zoneliste er de kendte, så uden den ville linjen i Overblik forsvinde af sig selv. `afvist_ny` er dem, brugeren har sagt "Ikke nu" til.
/// Eksisterende installationer beholder det, de ser: alt i `zoner`, der ikke stod i `fravalgt`, bliver valgt.
const SKEMA_9: &str = "
CREATE TABLE IF NOT EXISTS valgt (zone TEXT PRIMARY KEY);
CREATE TABLE IF NOT EXISTS afvist_ny (zone TEXT PRIMARY KEY);
CREATE TABLE IF NOT EXISTS ny_zone (zone TEXT PRIMARY KEY);
INSERT OR IGNORE INTO valgt (zone) SELECT id FROM zoner WHERE id NOT IN (SELECT zone FROM fravalgt);
DROP TABLE IF EXISTS fravalgt;
";
/// `teknologi`: brugerens valg pr. domæne og teknologi (`teknologi::Valg`). Ingen række = "Ved ikke". Radar-serierne `par`, `mod7`, `mod30`, `kilde7` og `kilde30` bruges
/// ikke længere: `mod*` viste hele verdens liste, fordi Radar ignorerer `location` på `top/locations/origin` (målt live). Værnets tal røres ikke.
const SKEMA_10: &str = "
CREATE TABLE IF NOT EXISTS teknologi (zone TEXT NOT NULL, tek TEXT NOT NULL, valg TEXT NOT NULL, besked INTEGER NOT NULL, sat_utc TEXT NOT NULL, PRIMARY KEY (zone, tek));
DELETE FROM radar_dag WHERE serie IN ('par', 'mod7', 'mod30', 'kilde7', 'kilde30');
";
/// `hentet_angreb`: døgn, hvis enkelthændelser (art `angreb`) er hentet. Et nyt døgn får dem med i `gem_doegn`; de gamle hentes i langsomt tempo af `koe::ekstra_runde`. Ingen eksisterende række ændres.
const SKEMA_11: &str = "
CREATE TABLE IF NOT EXISTS hentet_angreb (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
";
/// `vurderet`: det seneste døgn pr. domæne, hvis mærker er vurderet til en besked (`notify::vurderede`). `hentet_angreb` tømmes, så `koe::ekstra_runde` henter de døgn, Cloudflare stadig har, igen
/// med netværket i nøglen. De gemte angreb bliver stående, til deres døgn er hentet igen. Ingen anden række ændres.
const SKEMA_12: &str = "
CREATE TABLE IF NOT EXISTS vurderet (zone TEXT PRIMARY KEY, dag TEXT NOT NULL);
DELETE FROM hentet_angreb;
";
/// `hentet_historik`: døgn, hvis årstal (arterne `h_*`, `historik`) er hentet. `hentet_laek`: døgn, hvis lækkede kodeord (art `laek`) er hentet. `zone_tilstand`: zonen hos Cloudflare, som den sidst blev
/// hentet. Ingen eksisterende række ændres.
const SKEMA_13: &str = "
CREATE TABLE IF NOT EXISTS hentet_historik (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
CREATE TABLE IF NOT EXISTS hentet_laek (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
CREATE TABLE IF NOT EXISTS zone_tilstand (zone TEXT PRIMARY KEY, status TEXT NOT NULL, paused INTEGER NOT NULL, dev_sek INTEGER NOT NULL, phishing INTEGER NOT NULL, hentet_utc TEXT NOT NULL);
";
/// `tek_vis`: kolonnerne på `#teknologi` (`vis`), en ny teknologi, der ikke er kvitteret (`ny`), og egne tilføjelser (`egen`). `hentet_laek` tømmes, så `koe::ekstra_runde` henter de døgn, Cloudflare stadig har,
/// igen med `LAEK_QUERY` uden sti-filter. De gemte rækker erstattes døgn for døgn. Ingen anden række ændres, heller ikke `teknologi`: id'erne er de samme.
const SKEMA_14: &str = "
CREATE TABLE IF NOT EXISTS tek_vis (tek TEXT PRIMARY KEY, vis INTEGER NOT NULL, ny INTEGER NOT NULL, egen INTEGER NOT NULL);
DELETE FROM hentet_laek;
";
/// `hentet_dns` og `hentet_dmarc`: døgn, hvis DNS-tal (arterne i `dns::ARTER`) og DMARC-tal (art `dmarc`) er hentet. `hentet_laek` tømmes, så `koe::ekstra_runde` henter de døgn, Cloudflare stadig har, igen uden krav om POST
/// og med `clean` pr. sti. De gemte rækker erstattes døgn for døgn. Ingen anden række ændres.
const SKEMA_15: &str = "
CREATE TABLE IF NOT EXISTS hentet_dns (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
CREATE TABLE IF NOT EXISTS hentet_dmarc (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
DELETE FROM hentet_laek;
";
/// `sikkerhedsfund`: Cloudflares egne aktive fund (Security Center Insights, `sikkerhed.rs`) for zonerne, som de sidst blev hentet. Ingen konto-id, intet domænenavn
/// og ingen IP-adresse: kun zone, Cloudflares id, alvor, klasse, type og dato. Tabellen erstattes helt ved hver hentning. Ingen eksisterende række ændres.
const SKEMA_16: &str = "
CREATE TABLE IF NOT EXISTS sikkerhedsfund (zone TEXT NOT NULL, id TEXT NOT NULL, alvor TEXT NOT NULL, klasse TEXT NOT NULL, type TEXT NOT NULL, siden TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, id));
";
/// `hentet_besoeg`: døgn, hvis besøg pr. land (arterne `b_land` og `b_total`, `besoeg.rs`) er hentet. Ingen eksisterende række ændres.
const SKEMA_17: &str = "
CREATE TABLE IF NOT EXISTS hentet_besoeg (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
";
/// `hentet_robot` og `hentet_side`: døgn, hvis besøg pr. kendt robot (art `b_robot`) og mest besøgte sider (arterne `b_side` og `b_side_b`) er hentet
/// (V-BESOEG.2). Hver sin tabel, så de allerede hentede besøgsdøgn også får dem. Ingen eksisterende række ændres.
const SKEMA_18: &str = "
CREATE TABLE IF NOT EXISTS hentet_robot (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
CREATE TABLE IF NOT EXISTS hentet_side (zone TEXT NOT NULL, dag TEXT NOT NULL, hentet_utc TEXT NOT NULL, PRIMARY KEY (zone, dag));
";
/// Arterne fra hovedhentningen (`vaern::DOEGN_QUERY` og `sti200::hoved_query`): dem, `gem_doegn` erstatter. `fwn` er antal hændelser
/// i `fw`-svaret (`vaern::FW_HAENDELSER`, kun til loftet).
pub const DOEGN_ARTER: &[&str] = &[
    "alle", "angreb", "bot", "falsk", "fw", "fwn", "probe", "seneste", "sti200", "sti200b",
];
/// `indstillinger`: sat, når de færdige døgn i hentevinduet én gang er sat til at blive hentet igen med Cloudflares estimat
/// (`vaern::fw_vaegt_en_gang`). Værdien er den første dag, hvis `fw`-tal er estimater; ældre døgn er talte rækker.
pub const FW_VAEGT: &str = "fw_vaegt_v1";
/// `indstillinger`: sat, når domænerne med årstal én gang er sat til at få unikke besøgende hentet for de døgn, de allerede har (`unikke_en_gang`). Værdien er dagen, det skete.
pub const UNIKKE_V1: &str = "unikke_v1";
/// Foran zone-id'et i `indstillinger`: domænets hentede døgn i året venter på `unikke` (`unikke_en_gang`). Slettes, når de har fået det (`gem_unikke`).
const UNIKKE_MANGLER: &str = "unikke_mangler:";
/// Foran zone-id'et i `indstillinger`: Cloudflare svarede ikke på besøgene for domænet (værdien er den rensede fejl). Det koster kun besøgene.
pub const BESOEG_FEJL: &str = "besoeg_fejl:";
/// Som `BESOEG_FEJL` for de kendte robotter og de mest besøgte sider (V-BESOEG.2). Hver koster kun sit eget afsnit.
pub const ROBOT_FEJL: &str = "robot_fejl:";
pub const SIDE_FEJL: &str = "side_fejl:";
/// `indstillinger`: sat, når domænerne med årstal én gang er sat til at få IP-klasserne (`h_ipklasse`) hentet for de døgn, de allerede har (`ipklasse_en_gang`).
pub const IPKLASSE_V1: &str = "ipklasse_v1";
/// Foran zone-id'et i `indstillinger`: domænets hentede døgn i året venter på `h_ipklasse`. Slettes af `gem_ipklasse`.
const IPKLASSE_MANGLER: &str = "ipklasse_mangler:";
/// Besøgsarterne, der hentes for sig (`besoeg.rs`): de står uden for `raekker`, så de ikke blander sig i Angreb og Overblik.
pub const BESOEG_ARTER: &[&str] = &["b_total", "b_land", "b_robot", "b_side", "b_side_b"];
/// Arterne fra `adgang::ADGANG_QUERY`: dem, `gem_ekstra` erstatter.
pub const EKSTRA_ARTER: &[&str] = &["post", "status", "land", "login", "stoppet", "asn"];
/// Foran zone-id'et i `indstillinger`: Cloudflare svarede ikke på de ekstra tal for domænet (værdien er den rensede fejl).
const ADGANG_FEJL: &str = "adgang_fejl:";
/// Foran zone-id'et i `indstillinger`: Cloudflare svarede ikke på DNS- eller DMARC-tallene for domænet. Det koster kun det afsnit.
pub const DNS_FEJL: &str = "dns_fejl:";
pub const DMARC_FEJL: &str = "dmarc_fejl:";
/// Foran `indstillinger`-nøglerne for Radar (`radar::SIDST`, `radar::FEJL`): de hører til de hentede tal, ikke til brugerens valg.
pub const RADAR_PRAEFIKS: &str = "radar_";
/// Foran `indstillinger`-nøglerne for sikkerhedsfundene (`sikkerhed::HENTET`, `sikkerhed::TILSTAND`): de hører til de hentede tal.
pub const SIKKERHED_PRAEFIKS: &str = "sikkerhedsfund_";

/// Én række i `radar_dag`.
#[derive(Debug, Clone, PartialEq)]
pub struct RadarRaekke {
    pub serie: String,
    pub dag: String,
    pub noegle: String,
    pub vaerdi: f64,
    pub normalisering: String,
    pub hentet_utc: String,
}

/// Ældste dag pr. art og nyeste dag for en række zoner (`Store::aeldste`).
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Aeldste {
    pub trafik_fra: Option<String>,
    pub sikkerhed_fra: Option<String>,
    pub sidste: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Raekke {
    pub dag: String,
    pub art: String,
    pub noegle: String,
    pub n: i64,
}

pub struct Store {
    db: Connection,
}

/// rusqlites egen fejltekst kasseres; webvisningen får kun koden `lager`.
fn lagerfejl(_: rusqlite::Error) -> Fejl {
    Fejl::Lager
}

impl Store {
    pub fn aaben(sti: &Path) -> Result<Self, Fejl> {
        Self::klar(Connection::open(sti).map_err(lagerfejl)?)
    }

    #[cfg(test)]
    pub fn i_hukommelse() -> Result<Self, Fejl> {
        Self::klar(Connection::open_in_memory().map_err(lagerfejl)?)
    }

    /// Opretter eller migrerer skemaet trin for trin (0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10) i én transaktion.
    fn klar(mut db: Connection) -> Result<Self, Fejl> {
        let v: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(lagerfejl)?;
        if v > SKEMA_VERSION {
            return Err(Fejl::NyereDatabase(v));
        }
        let tx = db.transaction().map_err(lagerfejl)?;
        if v < 1 {
            tx.execute_batch(SKEMA_1).map_err(lagerfejl)?;
        }
        if v < 2 {
            tx.execute_batch(SKEMA_2).map_err(lagerfejl)?;
        }
        if v < 3 {
            tx.execute_batch(SKEMA_3).map_err(lagerfejl)?;
        }
        if v < 4 {
            tx.execute_batch(SKEMA_4).map_err(lagerfejl)?;
        }
        if v < 5 {
            tx.execute_batch(SKEMA_5).map_err(lagerfejl)?;
        }
        if v < 6 {
            tx.execute_batch(SKEMA_6).map_err(lagerfejl)?;
        }
        if v < 7 {
            tx.execute_batch(SKEMA_7).map_err(lagerfejl)?;
        }
        if v < 8 {
            tx.execute_batch(SKEMA_8).map_err(lagerfejl)?;
        }
        if v < 9 {
            tx.execute_batch(SKEMA_9).map_err(lagerfejl)?;
        }
        if v < 10 {
            tx.execute_batch(SKEMA_10).map_err(lagerfejl)?;
        }
        if v < 11 {
            tx.execute_batch(SKEMA_11).map_err(lagerfejl)?;
        }
        if v < 12 {
            tx.execute_batch(SKEMA_12).map_err(lagerfejl)?;
        }
        if v < 13 {
            tx.execute_batch(SKEMA_13).map_err(lagerfejl)?;
        }
        if v < 14 {
            tx.execute_batch(SKEMA_14).map_err(lagerfejl)?;
        }
        if v < 15 {
            tx.execute_batch(SKEMA_15).map_err(lagerfejl)?;
        }
        if v < 16 {
            tx.execute_batch(SKEMA_16).map_err(lagerfejl)?;
        }
        if v < 17 {
            tx.execute_batch(SKEMA_17).map_err(lagerfejl)?;
        }
        if v < 18 {
            tx.execute_batch(SKEMA_18).map_err(lagerfejl)?;
        }
        tx.pragma_update(None, "user_version", SKEMA_VERSION).map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)?;
        Ok(Store { db })
    }

    #[cfg(test)]
    pub fn skema_version(&self) -> Result<i64, Fejl> {
        self.db.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(lagerfejl)
    }

    /// dag → hentet_utc for zonen.
    pub fn hentet(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT dag, hentet_utc FROM hentet WHERE zone = ?1")
            .map_err(lagerfejl)?;
        let rk = st.query_map([zone], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// dag → hentet_utc for de ekstra tal (`gem_ekstra`).
    pub fn hentet_angreb(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT dag, hentet_utc FROM hentet_angreb WHERE zone = ?1")
            .map_err(lagerfejl)?;
        let rk = st.query_map([zone], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    pub fn hentet_ekstra(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT dag, hentet_utc FROM hentet_ekstra WHERE zone = ?1")
            .map_err(lagerfejl)?;
        let rk = st.query_map([zone], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Erstatter de givne arter for ét døgn og noterer, hvornår døgnet blev hentet, i `tabel`. Én transaktion.
    fn gem_arter(
        &mut self,
        arter: &[&str],
        tabel: &str,
        zone: &str,
        dag: &str,
        raekker: &[(String, String, i64)],
        hentet_utc: &str,
    ) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        let slet = format!(
            "DELETE FROM vagt_dag WHERE zone = ?1 AND dag = ?2 AND art IN ({})",
            vec!["?"; arter.len()].join(", ")
        );
        tx.execute(&slet, params_from_iter([zone, dag].into_iter().chain(arter.iter().copied())))
            .map_err(lagerfejl)?;
        {
            let mut ind = tx
                .prepare("INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES (?1, ?2, ?3, ?4, ?5)")
                .map_err(lagerfejl)?;
            for (art, noegle, n) in raekker {
                ind.execute(params![zone, dag, art, noegle, n]).map_err(lagerfejl)?;
            }
        }
        tx.execute(
            &format!("INSERT OR REPLACE INTO {tabel} (zone, dag, hentet_utc) VALUES (?1, ?2, ?3)"),
            params![zone, dag, hentet_utc],
        )
        .map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)
    }

    /// Erstatter ét døgns `DOEGN_ARTER` for zonen og noterer, hvornår det blev hentet. Andre arter bliver stående: i dag og i
    /// går hentes hver time, og de ekstra tal hentes kun én gang. Én transaktion.
    pub fn gem_doegn(&mut self, zone: &str, dag: &str, raekker: &[(String, String, i64)], hentet_utc: &str) -> Result<(), Fejl> {
        self.gem_arter(DOEGN_ARTER, "hentet", zone, dag, raekker, hentet_utc)?;
        self.db
            .execute(
                "INSERT OR REPLACE INTO hentet_angreb (zone, dag, hentet_utc) VALUES (?1, ?2, ?3)",
                [zone, dag, hentet_utc],
            )
            .map(|_| ())
            .map_err(lagerfejl)
    }

    /// Én arts rækker (nøgle, n) for ét døgn, i den rækkefølge de blev gemt.
    pub fn doegn_art(&self, zone: &str, dag: &str, art: &str) -> Result<Vec<(String, i64)>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT noegle, n FROM vagt_dag WHERE zone = ?1 AND dag = ?2 AND art = ?3 ORDER BY rowid")
            .map_err(lagerfejl)?;
        let rk = st.query_map([zone, dag, art], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Én gang (`FW_VAEGT`): døgnene fra `fra` til før `i_gaar` markeres som hentet, før døgnet var slut, så hovedhentningen
    /// henter dem igen i sit normale tempo. Kun `hentet`: de ekstra tal, trafikken og alt andet røres ikke, og ingen tal slettes.
    /// `fra` gemmes som den første dag med estimater. Én transaktion. Returnerer, om det skete nu.
    pub fn fw_vaegt_en_gang(&mut self, fra: &str, i_gaar: &str) -> Result<bool, Fejl> {
        if self.indstilling(FW_VAEGT)?.is_some() {
            return Ok(false);
        }
        let tx = self.db.transaction().map_err(lagerfejl)?;
        tx.execute(
            "UPDATE hentet SET hentet_utc = dag || 'T00:00:00.000Z' WHERE dag >= ?1 AND dag < ?2",
            [fra, i_gaar],
        )
        .map_err(lagerfejl)?;
        tx.execute(
            "INSERT OR REPLACE INTO indstillinger (noegle, vaerdi) VALUES (?1, ?2)",
            [FW_VAEGT, fra],
        )
        .map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)?;
        Ok(true)
    }

    /// Erstatter ét døgns enkelthændelser og noterer det i `hentet_angreb`. Én transaktion.
    pub fn gem_angreb(&mut self, zone: &str, dag: &str, raekker: &[(String, String, i64)], hentet_utc: &str) -> Result<(), Fejl> {
        self.gem_arter(&["angreb"], "hentet_angreb", zone, dag, raekker, hentet_utc)
    }

    pub fn hentet_laek(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        self.hentet_i("hentet_laek", zone)
    }

    pub fn hentet_dns(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        self.hentet_i("hentet_dns", zone)
    }

    pub fn hentet_dmarc(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        self.hentet_i("hentet_dmarc", zone)
    }

    /// Erstatter ét døgns DNS-tal og noterer det i `hentet_dns`. Én transaktion.
    pub fn gem_dns(&mut self, zone: &str, dag: &str, raekker: &[(String, String, i64)], hentet_utc: &str) -> Result<(), Fejl> {
        self.gem_arter(crate::dns::ARTER, "hentet_dns", zone, dag, raekker, hentet_utc)
    }

    /// Erstatter ét døgns DMARC-tal og noterer det i `hentet_dmarc`. Én transaktion.
    pub fn gem_dmarc(&mut self, zone: &str, dag: &str, raekker: &[(String, String, i64)], hentet_utc: &str) -> Result<(), Fejl> {
        self.gem_arter(&["dmarc"], "hentet_dmarc", zone, dag, raekker, hentet_utc)
    }

    pub fn hentet_historik(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        self.hentet_i("hentet_historik", zone)
    }

    fn hentet_i(&self, tabel: &str, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        let mut st = self
            .db
            .prepare(&format!("SELECT dag, hentet_utc FROM {tabel} WHERE zone = ?1"))
            .map_err(lagerfejl)?;
        let rk = st.query_map([zone], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Erstatter ét døgns lækkede kodeord for zonen og noterer det i `hentet_laek`. Én transaktion.
    pub fn gem_laek(&mut self, zone: &str, dag: &str, raekker: &[(String, String, i64)], hentet_utc: &str) -> Result<(), Fejl> {
        self.gem_arter(&["laek", "laek_ren"], "hentet_laek", zone, dag, raekker, hentet_utc)
    }

    /// Årstallene for døgnene `dage`, som svaret dækker: (dag, art, nøgle, n). Et døgn uden rækker er også hentet. Én transaktion.
    pub fn gem_historik(
        &mut self,
        zone: &str,
        dage: &[String],
        raekker: &[(String, String, String, i64)],
        hentet_utc: &str,
    ) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut slet = tx
                .prepare("DELETE FROM vagt_dag WHERE zone = ?1 AND dag = ?2 AND art IN ('h_total', 'h_land', 'h_status', 'h_ipklasse')")
                .map_err(lagerfejl)?;
            let mut noter = tx
                .prepare("INSERT OR REPLACE INTO hentet_historik (zone, dag, hentet_utc) VALUES (?1, ?2, ?3)")
                .map_err(lagerfejl)?;
            for dag in dage {
                slet.execute(params![zone, dag]).map_err(lagerfejl)?;
                noter.execute(params![zone, dag, hentet_utc]).map_err(lagerfejl)?;
            }
            let mut ind = tx
                .prepare("INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES (?1, ?2, ?3, ?4, ?5)")
                .map_err(lagerfejl)?;
            for (dag, art, noegle, n) in raekker.iter().filter(|r| dage.contains(&r.0)) {
                ind.execute(params![zone, dag, art, noegle, n]).map_err(lagerfejl)?;
            }
        }
        tx.commit().map_err(lagerfejl)
    }

    /// Én gang (`UNIKKE_V1`): hvert domæne med hentede årstal noteres som ventende på `unikke` (`UNIKKE_MANGLER`), så køen henter året igen
    /// for de døgn, det allerede har. Ingen tal ændres her. Et domæne, der først hentes bagefter, får tallet med det samme og noteres ikke. Én transaktion.
    /// Returnerer, om det skete nu.
    pub fn unikke_en_gang(&mut self, i_dag: &str) -> Result<bool, Fejl> {
        if self.indstilling(UNIKKE_V1)?.is_some() {
            return Ok(false);
        }
        let tx = self.db.transaction().map_err(lagerfejl)?;
        tx.execute(
            "INSERT OR REPLACE INTO indstillinger (noegle, vaerdi) SELECT DISTINCT ?1 || zone, '1' FROM hentet_historik",
            [UNIKKE_MANGLER],
        )
        .map_err(lagerfejl)?;
        tx.execute(
            "INSERT OR REPLACE INTO indstillinger (noegle, vaerdi) VALUES (?1, ?2)",
            [UNIKKE_V1, i_dag],
        )
        .map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)?;
        Ok(true)
    }

    /// Venter domænet på `unikke` for de døgn, det allerede har (`unikke_en_gang`)?
    pub fn unikke_mangler(&self, zone: &str) -> Result<bool, Fejl> {
        Ok(self.indstilling(&format!("{UNIKKE_MANGLER}{zone}"))?.is_some())
    }

    /// Gemmer `unikke` (art `h_total`) for de døgn, domænet allerede har årstal for, og noterer, at domænet ikke venter mere. Kun den nøgle
    /// skrives: de andre tal for døgnet røres ikke, og et døgn uden årstal får ingen række (så det ikke ligner et døgn med trafik). Én transaktion.
    pub fn gem_unikke(&mut self, zone: &str, raekker: &[(String, i64)]) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut ind = tx
                .prepare(
                    "INSERT OR REPLACE INTO vagt_dag (zone, dag, art, noegle, n) SELECT ?1, ?2, 'h_total', ?3, ?4 \
                     WHERE EXISTS (SELECT 1 FROM hentet_historik WHERE zone = ?1 AND dag = ?2)",
                )
                .map_err(lagerfejl)?;
            for (dag, n) in raekker {
                ind.execute(params![zone, dag, crate::historik::UNIKKE, n]).map_err(lagerfejl)?;
            }
        }
        tx.execute("DELETE FROM indstillinger WHERE noegle = ?1", [format!("{UNIKKE_MANGLER}{zone}")])
            .map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)
    }

    /// Som `unikke_en_gang` for IP-klasserne (`IPKLASSE_V1`): hvert domæne med hentede årstal noteres som ventende på `h_ipklasse`. Ingen tal ændres. Én transaktion.
    pub fn ipklasse_en_gang(&mut self, i_dag: &str) -> Result<bool, Fejl> {
        if self.indstilling(IPKLASSE_V1)?.is_some() {
            return Ok(false);
        }
        let tx = self.db.transaction().map_err(lagerfejl)?;
        tx.execute(
            "INSERT OR REPLACE INTO indstillinger (noegle, vaerdi) SELECT DISTINCT ?1 || zone, '1' FROM hentet_historik",
            [IPKLASSE_MANGLER],
        )
        .map_err(lagerfejl)?;
        tx.execute(
            "INSERT OR REPLACE INTO indstillinger (noegle, vaerdi) VALUES (?1, ?2)",
            [IPKLASSE_V1, i_dag],
        )
        .map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)?;
        Ok(true)
    }

    /// Venter domænet på `h_ipklasse` for de døgn, det allerede har (`ipklasse_en_gang`)?
    pub fn ipklasse_mangler(&self, zone: &str) -> Result<bool, Fejl> {
        Ok(self.indstilling(&format!("{IPKLASSE_MANGLER}{zone}"))?.is_some())
    }

    /// Gemmer `h_ipklasse` for de døgn, domænet allerede har årstal for, og noterer, at domænet ikke venter mere. Kun den art skrives (den erstattes for
    /// døgnet); de andre tal og `hentet_utc` røres ikke. Én transaktion.
    pub fn gem_ipklasse(&mut self, zone: &str, raekker: &[(String, String, String, i64)]) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut slet = tx
                .prepare("DELETE FROM vagt_dag WHERE zone = ?1 AND dag = ?2 AND art = 'h_ipklasse'")
                .map_err(lagerfejl)?;
            let dage: HashSet<&str> = raekker.iter().map(|r| r.0.as_str()).collect();
            for dag in dage {
                slet.execute(params![zone, dag]).map_err(lagerfejl)?;
            }
            let mut ind = tx
                .prepare(
                    "INSERT INTO vagt_dag (zone, dag, art, noegle, n) SELECT ?1, ?2, 'h_ipklasse', ?3, ?4 \
                     WHERE EXISTS (SELECT 1 FROM hentet_historik WHERE zone = ?1 AND dag = ?2)",
                )
                .map_err(lagerfejl)?;
            for (dag, _, noegle, n) in raekker.iter().filter(|r| r.1 == crate::historik::IPKLASSE) {
                ind.execute(params![zone, dag, noegle, n]).map_err(lagerfejl)?;
            }
        }
        tx.execute("DELETE FROM indstillinger WHERE noegle = ?1", [format!("{IPKLASSE_MANGLER}{zone}")])
            .map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)
    }

    pub fn hentet_robot(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        self.hentet_i("hentet_robot", zone)
    }

    pub fn hentet_side(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        self.hentet_i("hentet_side", zone)
    }

    /// Kendte robotter for døgnene `dage` (art `b_robot`). Et døgn uden rækker er også hentet. Én transaktion.
    pub fn gem_robot(
        &mut self,
        zone: &str,
        dage: &[String],
        raekker: &[(String, String, String, i64)],
        hentet_utc: &str,
    ) -> Result<(), Fejl> {
        self.gem_besoeg_i("hentet_robot", &["b_robot"], zone, dage, raekker, hentet_utc)
    }

    /// Mest besøgte sider for døgnene `dage` (arterne `b_side` og `b_side_b`). Et døgn uden rækker er også hentet. Én transaktion.
    pub fn gem_side(
        &mut self,
        zone: &str,
        dage: &[String],
        raekker: &[(String, String, String, i64)],
        hentet_utc: &str,
    ) -> Result<(), Fejl> {
        self.gem_besoeg_i("hentet_side", &["b_side", "b_side_b"], zone, dage, raekker, hentet_utc)
    }

    fn gem_besoeg_i(
        &mut self,
        tabel: &str,
        arter: &[&str],
        zone: &str,
        dage: &[String],
        raekker: &[(String, String, String, i64)],
        hentet_utc: &str,
    ) -> Result<(), Fejl> {
        let liste = arter.iter().map(|a| format!("'{a}'")).collect::<Vec<_>>().join(", ");
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut slet = tx
                .prepare(&format!("DELETE FROM vagt_dag WHERE zone = ?1 AND dag = ?2 AND art IN ({liste})"))
                .map_err(lagerfejl)?;
            let mut noter = tx
                .prepare(&format!(
                    "INSERT OR REPLACE INTO {tabel} (zone, dag, hentet_utc) VALUES (?1, ?2, ?3)"
                ))
                .map_err(lagerfejl)?;
            for dag in dage {
                slet.execute(params![zone, dag]).map_err(lagerfejl)?;
                noter.execute(params![zone, dag, hentet_utc]).map_err(lagerfejl)?;
            }
            let mut ind = tx
                .prepare("INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES (?1, ?2, ?3, ?4, ?5)")
                .map_err(lagerfejl)?;
            for (dag, art, noegle, n) in raekker.iter().filter(|r| dage.contains(&r.0) && arter.contains(&r.1.as_str())) {
                ind.execute(params![zone, dag, art, noegle, n]).map_err(lagerfejl)?;
            }
        }
        tx.commit().map_err(lagerfejl)
    }

    pub fn hentet_besoeg(&self, zone: &str) -> Result<HashMap<String, String>, Fejl> {
        self.hentet_i("hentet_besoeg", zone)
    }

    /// Besøgene for døgnene `dage`, som svaret dækker: (dag, art, nøgle, n), arterne `b_land` og `b_total`. Et døgn uden rækker er også hentet. Én transaktion.
    pub fn gem_besoeg(
        &mut self,
        zone: &str,
        dage: &[String],
        raekker: &[(String, String, String, i64)],
        hentet_utc: &str,
    ) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut slet = tx
                .prepare("DELETE FROM vagt_dag WHERE zone = ?1 AND dag = ?2 AND art IN ('b_total', 'b_land')")
                .map_err(lagerfejl)?;
            let mut noter = tx
                .prepare("INSERT OR REPLACE INTO hentet_besoeg (zone, dag, hentet_utc) VALUES (?1, ?2, ?3)")
                .map_err(lagerfejl)?;
            for dag in dage {
                slet.execute(params![zone, dag]).map_err(lagerfejl)?;
                noter.execute(params![zone, dag, hentet_utc]).map_err(lagerfejl)?;
            }
            let mut ind = tx
                .prepare("INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES (?1, ?2, ?3, ?4, ?5)")
                .map_err(lagerfejl)?;
            for (dag, art, noegle, n) in raekker.iter().filter(|r| dage.contains(&r.0)) {
                ind.execute(params![zone, dag, art, noegle, n]).map_err(lagerfejl)?;
            }
        }
        tx.commit().map_err(lagerfejl)
    }

    /// Dage med trafiktal for zonerne: `h_total`, ikke `hentet_historik`, for et hentet døgn uden trafik har ingen tal. Den ene beregning bag "Trafik fra", "Hele historikken" og kalenderens første dag.
    pub fn trafik_dage(&self, zoner: &[String]) -> Result<HashSet<String>, Fejl> {
        Ok(self
            .raekker_art(zoner, &["h_total"], "0000-00-00")?
            .into_iter()
            .map(|(_, r)| r.dag)
            .collect())
    }

    /// Dage med sikkerhedstal for zonerne.
    fn sikkerhed_dage(&self, zoner: &[String]) -> Result<HashSet<String>, Fejl> {
        let mut ud = HashSet::new();
        for z in zoner {
            ud.extend(self.hentet(z)?.into_keys());
        }
        Ok(ud)
    }

    /// Ældste dag pr. art og nyeste dag for zonerne.
    pub fn aeldste(&self, zoner: &[String]) -> Result<Aeldste, Fejl> {
        let (trafik, sikkerhed) = (self.trafik_dage(zoner)?, self.sikkerhed_dage(zoner)?);
        Ok(Aeldste {
            trafik_fra: trafik.iter().min().cloned(),
            sikkerhed_fra: sikkerhed.iter().min().cloned(),
            sidste: trafik.iter().chain(sikkerhed.iter()).max().cloned(),
        })
    }

    /// `#teknologi`: (tek, vis, ny, egen), sorteret efter id.
    pub fn tek_vis(&self) -> Result<Vec<crate::teknologi::VisRaekke>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT tek, vis, ny, egen FROM tek_vis ORDER BY tek")
            .map_err(lagerfejl)?;
        let rk = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Nye rækker. En række, der findes, røres ikke: et fravalg står.
    pub fn tilfoej_tek_vis(&mut self, rk: &[crate::teknologi::VisRaekke]) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        for (tek, vis, ny, egen) in rk {
            tx.execute(
                "INSERT OR IGNORE INTO tek_vis (tek, vis, ny, egen) VALUES (?1, ?2, ?3, ?4)",
                params![tek, vis, ny, egen],
            )
            .map_err(lagerfejl)?;
        }
        tx.commit().map_err(lagerfejl)
    }

    /// "Vis i tabellen": valgene på domænerne røres ikke, og en ny teknologi er hermed kvitteret.
    pub fn saet_tek_vis(&mut self, tek: &str, vis: bool) -> Result<(), Fejl> {
        self.db
            .execute(
                "INSERT INTO tek_vis (tek, vis, ny, egen) VALUES (?1, ?2, 0, 0) ON CONFLICT(tek) DO UPDATE SET vis = ?2, ny = 0",
                params![tek, vis],
            )
            .map(|_| ())
            .map_err(lagerfejl)
    }

    /// Zonerne, som de sidst blev hentet: zone → (tilstand, hentet).
    pub fn zone_tilstande(&self) -> Result<HashMap<String, (crate::cf::ZoneStand, String)>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT zone, status, paused, dev_sek, phishing, hentet_utc FROM zone_tilstand")
            .map_err(lagerfejl)?;
        let rk = st
            .query_map([], |r| {
                let z = crate::cf::ZoneStand {
                    status: r.get(1)?,
                    paused: r.get(2)?,
                    dev_sek: r.get(3)?,
                    phishing: r.get(4)?,
                };
                Ok((r.get(0)?, (z, r.get(5)?)))
            })
            .map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Erstatter alle gemte sikkerhedsfund med `fund`. Én transaktion.
    pub fn gem_sikkerhedsfund(&mut self, fund: &[crate::sikkerhed::Fund], hentet_utc: &str) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        tx.execute("DELETE FROM sikkerhedsfund", []).map_err(lagerfejl)?;
        {
            let mut ind = tx
                .prepare("INSERT OR REPLACE INTO sikkerhedsfund (zone, id, alvor, klasse, type, siden, hentet_utc) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
                .map_err(lagerfejl)?;
            for f in fund {
                ind.execute(params![f.zone, f.id, f.alvor, f.klasse, f.type_, f.siden, hentet_utc])
                    .map_err(lagerfejl)?;
            }
        }
        tx.commit().map_err(lagerfejl)
    }

    /// De gemte sikkerhedsfund for én zone (`None`: alle zoner).
    pub fn sikkerhedsfund(&self, zone: Option<&str>) -> Result<Vec<crate::sikkerhed::Fund>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT zone, id, alvor, klasse, type, siden FROM sikkerhedsfund WHERE ?1 IS NULL OR zone = ?1 ORDER BY zone, id")
            .map_err(lagerfejl)?;
        let rk = st
            .query_map(params![zone], |r| {
                Ok(crate::sikkerhed::Fund {
                    zone: r.get(0)?,
                    id: r.get(1)?,
                    alvor: r.get(2)?,
                    klasse: r.get(3)?,
                    type_: r.get(4)?,
                    siden: r.get(5)?,
                })
            })
            .map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    pub fn gem_zone_tilstand(&mut self, zone: &str, z: &crate::cf::ZoneStand, hentet_utc: &str) -> Result<(), Fejl> {
        self.db
            .execute(
                "INSERT OR REPLACE INTO zone_tilstand (zone, status, paused, dev_sek, phishing, hentet_utc) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![zone, z.status, z.paused, z.dev_sek, z.phishing, hentet_utc],
            )
            .map(|_| ())
            .map_err(lagerfejl)
    }

    /// Døgnet er uden for Cloudflares opbevaring: det noteres som hentet, og det, der allerede er gemt, bliver stående.
    pub fn noter_angreb(&mut self, zone: &str, dag: &str, hentet_utc: &str) -> Result<(), Fejl> {
        self.db
            .execute(
                "INSERT OR REPLACE INTO hentet_angreb (zone, dag, hentet_utc) VALUES (?1, ?2, ?3)",
                params![zone, dag, hentet_utc],
            )
            .map(|_| ())
            .map_err(lagerfejl)
    }

    /// Erstatter ét døgns `EKSTRA_ARTER` for zonen og noterer det i `hentet_ekstra`. Én transaktion.
    pub fn gem_ekstra(&mut self, zone: &str, dag: &str, raekker: &[(String, String, i64)], hentet_utc: &str) -> Result<(), Fejl> {
        self.gem_arter(EKSTRA_ARTER, "hentet_ekstra", zone, dag, raekker, hentet_utc)
    }

    /// Erstatter Radar-tallene for serien, landet og de døgn, svaret dækker (`dage`), og gemmer `raekker` (dag, nøgle, værdi). Én
    /// transaktion, så et svar aldrig står halvt: det er ét svars tal, der deler skala.
    pub fn gem_radar(
        &mut self,
        serie: &str,
        land: &str,
        dage: &[String],
        raekker: &[(String, String, f64)],
        normalisering: &str,
        hentet_utc: &str,
    ) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut slet = tx
                .prepare("DELETE FROM radar_dag WHERE serie = ?1 AND land = ?2 AND dag = ?3")
                .map_err(lagerfejl)?;
            for dag in dage {
                slet.execute(params![serie, land, dag]).map_err(lagerfejl)?;
            }
            let mut ind = tx
                .prepare("INSERT OR REPLACE INTO radar_dag (serie, land, dag, noegle, vaerdi, normalisering, hentet_utc) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
                .map_err(lagerfejl)?;
            for (dag, noegle, vaerdi) in raekker {
                ind.execute(params![serie, land, dag, noegle, vaerdi, normalisering, hentet_utc])
                    .map_err(lagerfejl)?;
            }
        }
        tx.commit().map_err(lagerfejl)
    }

    /// Radar-tallene for landet (tomt = verden) fra og med `fra_dag`, alle serier.
    pub fn radar_raekker(&self, land: &str, fra_dag: &str) -> Result<Vec<RadarRaekke>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT serie, dag, noegle, vaerdi, normalisering, hentet_utc FROM radar_dag WHERE land = ?1 AND dag >= ?2")
            .map_err(lagerfejl)?;
        let rk = st
            .query_map(params![land, fra_dag], |r| {
                Ok(RadarRaekke {
                    serie: r.get(0)?,
                    dag: r.get(1)?,
                    noegle: r.get(2)?,
                    vaerdi: r.get(3)?,
                    normalisering: r.get(4)?,
                    hentet_utc: r.get(5)?,
                })
            })
            .map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// dag → hentet_utc for én Radar-serie.
    /// Lande, Radar har tal for som mål fra og med `fra_dag`: blandt de største mål (`maal`, verden) med en andel over 0, eller med en kildeliste
    /// (`par7`/`par28` for landet med mindst én kilde). Sorteret. Kun det, programmet allerede har hentet.
    pub fn radar_lande_med_tal(&self, fra_dag: &str) -> Result<Vec<String>, Fejl> {
        let mut st = self
            .db
            .prepare(
                "SELECT noegle FROM radar_dag WHERE serie = ?1 AND land = '' AND noegle != '' AND vaerdi > 0 AND dag >= ?2 \
                 UNION SELECT land FROM radar_dag WHERE serie IN (?3, ?4) AND land != '' AND noegle != '' AND dag >= ?2 ORDER BY 1",
            )
            .map_err(lagerfejl)?;
        let rk = st
            .query_map(params![crate::radar::MAAL, fra_dag, crate::radar::PAR7, crate::radar::PAR28], |r| {
                r.get(0)
            })
            .map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    pub fn radar_hentet(&self, serie: &str, land: &str) -> Result<HashMap<String, String>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT dag, MAX(hentet_utc) FROM radar_dag WHERE serie = ?1 AND land = ?2 GROUP BY dag")
            .map_err(lagerfejl)?;
        let rk = st.query_map([serie, land], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Domænets valg: (teknologi, valg, giv besked).
    pub fn teknologi(&self, zone: &str) -> Result<Vec<(String, String, bool)>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT tek, valg, besked FROM teknologi WHERE zone = ?1 ORDER BY tek")
            .map_err(lagerfejl)?;
        let rk = st.query_map([zone], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Alle valg: (zone, teknologi, valg). "Ved ikke" står ikke i tabellen.
    pub fn teknologi_valg(&self) -> Result<Vec<(String, String, String)>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT zone, tek, valg FROM teknologi ORDER BY zone, tek")
            .map_err(lagerfejl)?;
        let rk = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Sætter alt til "Ved ikke": for ét domæne, eller for alle uden `zone`.
    pub fn nulstil_teknologi(&mut self, zone: Option<&str>) -> Result<(), Fejl> {
        match zone {
            Some(z) => self.db.execute("DELETE FROM teknologi WHERE zone = ?1", [z]),
            None => self.db.execute("DELETE FROM teknologi", []),
        }
        .map(|_| ())
        .map_err(lagerfejl)
    }

    /// Gemmer ét valg. "Ved ikke" er standarden og sletter rækken.
    pub fn gem_teknologi(&mut self, zone: &str, tek: &str, valg: &str, besked: bool, sat_utc: &str) -> Result<(), Fejl> {
        if valg == "ved_ikke" {
            self.db
                .execute("DELETE FROM teknologi WHERE zone = ?1 AND tek = ?2", [zone, tek])
                .map(|_| ())
                .map_err(lagerfejl)
        } else {
            self.db
                .execute(
                    "INSERT OR REPLACE INTO teknologi (zone, tek, valg, besked, sat_utc) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![zone, tek, valg, besked, sat_utc],
                )
                .map(|_| ())
                .map_err(lagerfejl)
        }
    }

    /// "Anvend på alle domæner": domænets valg lægges på de andre. Teknologier, domænet ikke har taget stilling til, røres ikke. Én transaktion.
    pub fn kopier_teknologi(&mut self, fra: &str, til: &[String], sat_utc: &str) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut ind = tx
                .prepare("INSERT OR REPLACE INTO teknologi (zone, tek, valg, besked, sat_utc) SELECT ?1, tek, valg, besked, ?2 FROM teknologi WHERE zone = ?3")
                .map_err(lagerfejl)?;
            for z in til.iter().filter(|z| z.as_str() != fra) {
                ind.execute(params![z, sat_utc, fra]).map_err(lagerfejl)?;
            }
        }
        tx.commit().map_err(lagerfejl)
    }

    /// Hvert døgn med hvornår det sidst blev hentet, for alle domæner (`huller`).
    pub fn alle_hentet(&self) -> Result<Vec<(String, String)>, Fejl> {
        let mut st = self.db.prepare("SELECT DISTINCT dag, hentet_utc FROM hentet").map_err(lagerfejl)?;
        let rk = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Stierne, brugeren har taget stilling til for domænet: (sti, slået til), sorteret efter sti.
    pub fn login_stier(&self, zone: &str) -> Result<Vec<(String, bool)>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT sti, til FROM login_sti WHERE zone = ?1 ORDER BY sti")
            .map_err(lagerfejl)?;
        let rk = st.query_map([zone], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    pub fn gem_login_sti(&mut self, zone: &str, sti: &str, til: bool) -> Result<(), Fejl> {
        self.db
            .execute(
                "INSERT OR REPLACE INTO login_sti (zone, sti, til) VALUES (?1, ?2, ?3)",
                params![zone, sti, til],
            )
            .map(|_| ())
            .map_err(lagerfejl)
    }

    pub fn slet_login_sti(&mut self, zone: &str, sti: &str) -> Result<(), Fejl> {
        self.db
            .execute("DELETE FROM login_sti WHERE zone = ?1 AND sti = ?2", params![zone, sti])
            .map(|_| ())
            .map_err(lagerfejl)
    }

    /// Domænets ekstra døgn hentes igen, fordi en ny sti skal med i forespørgslen. Tallene bliver, til hvert døgn er erstattet.
    pub fn glem_hentet_ekstra(&mut self, zone: &str) -> Result<(), Fejl> {
        self.db
            .execute("DELETE FROM hentet_ekstra WHERE zone = ?1", [zone])
            .map(|_| ())
            .map_err(lagerfejl)
    }

    /// Den rensede fejl, hvis de ekstra tal ikke kunne hentes for domænet ved seneste forsøg.
    pub fn adgang_fejl(&self, zone: &str) -> Result<Option<String>, Fejl> {
        self.indstilling(&format!("{ADGANG_FEJL}{zone}"))
    }

    /// Noterer eller (`None`) rydder fejlen for de ekstra tal.
    pub fn gem_adgang_fejl(&mut self, zone: &str, fejl: Option<&str>) -> Result<(), Fejl> {
        let noegle = format!("{ADGANG_FEJL}{zone}");
        match fejl {
            Some(f) => self.gem_indstilling(&noegle, f),
            None => self
                .db
                .execute("DELETE FROM indstillinger WHERE noegle = ?1", [noegle])
                .map(|_| ())
                .map_err(lagerfejl),
        }
    }

    /// Fejlen for et afsnit (`DNS_FEJL`, `DMARC_FEJL`) ved seneste forsøg.
    pub fn afsnit_fejl(&self, praefiks: &str, zone: &str) -> Result<Option<String>, Fejl> {
        self.indstilling(&format!("{praefiks}{zone}"))
    }

    /// Noterer eller (`None`) rydder fejlen for et afsnit.
    pub fn gem_afsnit_fejl(&mut self, praefiks: &str, zone: &str, fejl: Option<&str>) -> Result<(), Fejl> {
        let noegle = format!("{praefiks}{zone}");
        match fejl {
            Some(f) => self.gem_indstilling(&noegle, f),
            None => self
                .db
                .execute("DELETE FROM indstillinger WHERE noegle = ?1", [noegle])
                .map(|_| ())
                .map_err(lagerfejl),
        }
    }

    /// Som `raekker`, uden enkelthændelserne: overblikket regnes hvert 2. sekund, mens køen henter, og tæller dem ikke.
    pub fn raekker_tal(&self, zone: &str, fra_dag: &str) -> Result<Vec<Raekke>, Fejl> {
        Ok(self.raekker(zone, fra_dag)?.into_iter().filter(|r| r.art != "angreb").collect())
    }

    /// Alle rækker for zonen fra og med `fra_dag`, i den rækkefølge de blev gemt (som D1 i W).
    pub fn raekker(&self, zone: &str, fra_dag: &str) -> Result<Vec<Raekke>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT dag, art, noegle, n FROM vagt_dag WHERE zone = ?1 AND dag >= ?2 AND art NOT IN ('h_total', 'h_land', 'h_status', 'h_ipklasse', 'b_total', 'b_land', 'b_robot', 'b_side', 'b_side_b') ORDER BY rowid")
            .map_err(lagerfejl)?;
        let rk = st
            .query_map(params![zone, fra_dag], |r| {
                Ok(Raekke {
                    dag: r.get(0)?,
                    art: r.get(1)?,
                    noegle: r.get(2)?,
                    n: r.get(3)?,
                })
            })
            .map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Rækkerne for flere zoner fra og med `fra_dag`, hver med sin zone, i den rækkefølge de blev gemt (samlet Værn).
    pub fn raekker_for(&self, zoner: &[String], fra_dag: &str) -> Result<Vec<(String, Raekke)>, Fejl> {
        if zoner.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT zone, dag, art, noegle, n FROM vagt_dag WHERE dag >= ?1 AND zone IN ({}) ORDER BY rowid",
            vec!["?"; zoner.len()].join(", ")
        );
        let mut st = self.db.prepare(&sql).map_err(lagerfejl)?;
        let vaerdier = std::iter::once(fra_dag).chain(zoner.iter().map(String::as_str));
        let rk = st
            .query_map(params_from_iter(vaerdier), |r| {
                Ok((
                    r.get(0)?,
                    Raekke {
                        dag: r.get(1)?,
                        art: r.get(2)?,
                        noegle: r.get(3)?,
                        n: r.get(4)?,
                    },
                ))
            })
            .map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Første og sidste dag for hver `fw`-nøgle over hele historikken (regelkortenes "Først set", som i dyk).
    pub fn fw_datoer(&self, zoner: &[String]) -> Result<Vec<(String, String, String)>, Fejl> {
        if zoner.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT noegle, MIN(dag), MAX(dag) FROM vagt_dag WHERE art = 'fw' AND zone IN ({}) GROUP BY noegle",
            vec!["?"; zoner.len()].join(", ")
        );
        let mut st = self.db.prepare(&sql).map_err(lagerfejl)?;
        let rk = st
            .query_map(params_from_iter(zoner.iter()), |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(lagerfejl)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(lagerfejl)?;
        Ok(rk)
    }

    /// Rækkerne af de givne arter for flere zoner fra og med `fra_dag`, hver med sin zone (dyk). Uden rækkefølge: dyk tæller
    /// og sorterer selv, og primærnøglen dækker `zone`, `dag` og `art`.
    pub fn raekker_art(&self, zoner: &[String], arter: &[&str], fra_dag: &str) -> Result<Vec<(String, Raekke)>, Fejl> {
        if zoner.is_empty() || arter.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT zone, dag, art, noegle, n FROM vagt_dag WHERE dag >= ?1 AND zone IN ({}) AND art IN ({})",
            vec!["?"; zoner.len()].join(", "),
            vec!["?"; arter.len()].join(", ")
        );
        let mut st = self.db.prepare(&sql).map_err(lagerfejl)?;
        let vaerdier = std::iter::once(fra_dag)
            .chain(zoner.iter().map(String::as_str))
            .chain(arter.iter().copied());
        let rk = st
            .query_map(params_from_iter(vaerdier), |r| {
                Ok((
                    r.get(0)?,
                    Raekke {
                        dag: r.get(1)?,
                        art: r.get(2)?,
                        noegle: r.get(3)?,
                        n: r.get(4)?,
                    },
                ))
            })
            .map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    fn zone_saet(&self, sql: &str) -> Result<HashSet<String>, Fejl> {
        let mut st = self.db.prepare(sql).map_err(lagerfejl)?;
        let rk = st.query_map([], |r| r.get(0)).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// De domæner, brugeren har valgt.
    pub fn valgte(&self) -> Result<HashSet<String>, Fejl> {
        self.zone_saet("SELECT zone FROM valgt")
    }

    /// De nye domæner, der venter på et svar (linjen i Overblik).
    pub fn nye(&self) -> Result<HashSet<String>, Fejl> {
        self.zone_saet("SELECT zone FROM ny_zone")
    }

    /// Erstatter listen over valgte domæner. Et valgt domæne venter ikke længere på et svar. Tallene for et fravalgt domæne bliver liggende. Én transaktion.
    pub fn gem_valgte(&mut self, zoner: &[String]) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        tx.execute("DELETE FROM valgt", []).map_err(lagerfejl)?;
        {
            let mut ind = tx.prepare("INSERT OR IGNORE INTO valgt (zone) VALUES (?1)").map_err(lagerfejl)?;
            let mut ny = tx.prepare("DELETE FROM ny_zone WHERE zone = ?1").map_err(lagerfejl)?;
            for z in zoner {
                ind.execute([z]).map_err(lagerfejl)?;
                ny.execute([z]).map_err(lagerfejl)?;
            }
        }
        tx.commit().map_err(lagerfejl)
    }

    /// "Ikke nu": domænerne venter ikke længere på et svar og kommer ikke igen som nye. Én transaktion.
    pub fn afvis_nye(&mut self, zoner: &[String]) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut ind = tx
                .prepare("INSERT OR IGNORE INTO afvist_ny (zone) VALUES (?1)")
                .map_err(lagerfejl)?;
            let mut ny = tx.prepare("DELETE FROM ny_zone WHERE zone = ?1").map_err(lagerfejl)?;
            for z in zoner {
                ind.execute([z]).map_err(lagerfejl)?;
                ny.execute([z]).map_err(lagerfejl)?;
            }
        }
        tx.commit().map_err(lagerfejl)
    }

    /// Zonelisten med kun de valgte domæner: det, køen henter, og det, programmet viser.
    pub fn zoner_til(&self) -> Result<Vec<Zone>, Fejl> {
        let valgte = self.valgte()?;
        Ok(self.zoner()?.0.into_iter().filter(|z| valgte.contains(&z.id)).collect())
    }

    /// Id'erne på de valgte domæner, sorteret.
    pub fn valg(&self) -> Result<Vec<String>, Fejl> {
        let mut ud: Vec<String> = self.zoner_til()?.into_iter().map(|z| z.id).collect();
        ud.sort();
        Ok(ud)
    }

    /// Erstatter hele den gemte zoneliste med en frisk fra Cloudflare og holder valget ajour. Er det første forbindelse, eller ser nøglen en helt anden
    /// konto (ingen fælles zoner), starter alt forfra: intet er nyt, og intet er valgt, medmindre nøglen ser præcis ét domæne (T2: IT-firmaets
    /// kunde med et token til sin egen zone). Ellers er en zone, der ikke var kendt før og hverken er valgt eller afvist, ny (T3); den vælges aldrig af sig selv.
    /// En tom liste rører ikke valget: den siger ikke, at kontoen er en anden. Tallene bliver. Én transaktion.
    pub fn gem_zoner(&mut self, zoner: &[Zone], hentet_utc: &str) -> Result<(), Fejl> {
        let foer: HashSet<String> = self.zone_saet("SELECT id FROM zoner")?;
        let (valgte, afviste) = (self.valgte()?, self.zone_saet("SELECT zone FROM afvist_ny")?);
        let forfra = !zoner.is_empty() && !zoner.iter().any(|z| foer.contains(&z.id));
        let tx = self.db.transaction().map_err(lagerfejl)?;
        tx.execute("DELETE FROM zoner", []).map_err(lagerfejl)?;
        {
            let mut ind = tx
                .prepare("INSERT OR REPLACE INTO zoner (id, navn, hentet_utc) VALUES (?1, ?2, ?3)")
                .map_err(lagerfejl)?;
            for z in zoner {
                ind.execute(params![z.id, z.navn, hentet_utc]).map_err(lagerfejl)?;
            }
        }
        if forfra {
            tx.execute_batch("DELETE FROM valgt; DELETE FROM afvist_ny; DELETE FROM ny_zone; DELETE FROM teknologi;")
                .map_err(lagerfejl)?;
            if let [eneste] = zoner {
                tx.execute("INSERT INTO valgt (zone) VALUES (?1)", [&eneste.id])
                    .map_err(lagerfejl)?;
            }
        } else {
            let mut ny = tx.prepare("INSERT OR IGNORE INTO ny_zone (zone) VALUES (?1)").map_err(lagerfejl)?;
            for z in zoner
                .iter()
                .filter(|z| !foer.contains(&z.id) && !valgte.contains(&z.id) && !afviste.contains(&z.id))
            {
                ny.execute([&z.id]).map_err(lagerfejl)?;
            }
        }
        tx.execute("DELETE FROM ny_zone WHERE zone NOT IN (SELECT id FROM zoner)", [])
            .map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)
    }

    /// Den gemte zoneliste (sorteret efter navn) og hvornår den blev hentet (`None`, hvis der ikke er gemt nogen).
    pub fn zoner(&self) -> Result<(Vec<Zone>, Option<String>), Fejl> {
        let mut st = self.db.prepare("SELECT id, navn FROM zoner ORDER BY navn, id").map_err(lagerfejl)?;
        let zoner = st
            .query_map([], |r| {
                Ok(Zone {
                    id: r.get(0)?,
                    navn: r.get(1)?,
                })
            })
            .map_err(lagerfejl)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(lagerfejl)?;
        let hentet = self
            .db
            .query_row("SELECT max(hentet_utc) FROM zoner", [], |r| r.get::<_, Option<String>>(0))
            .map_err(lagerfejl)?;
        Ok((zoner, hentet))
    }

    /// En af programmets indstillinger (`None`, hvis den aldrig er gemt).
    pub fn indstilling(&self, noegle: &str) -> Result<Option<String>, Fejl> {
        let mut st = self
            .db
            .prepare("SELECT vaerdi FROM indstillinger WHERE noegle = ?1")
            .map_err(lagerfejl)?;
        let mut rk = st.query_map([noegle], |r| r.get(0)).map_err(lagerfejl)?;
        rk.next().transpose().map_err(lagerfejl)
    }

    pub fn gem_indstilling(&mut self, noegle: &str, vaerdi: &str) -> Result<(), Fejl> {
        self.db
            .execute(
                "INSERT OR REPLACE INTO indstillinger (noegle, vaerdi) VALUES (?1, ?2)",
                params![noegle, vaerdi],
            )
            .map(|_| ())
            .map_err(lagerfejl)
    }

    /// Notifikationsvalget pr. domæne (zone → 'alt' | 'roed' | 'fra'); domæner uden række følger forudindstillingen.
    pub fn notifikation_zoner(&self) -> Result<HashMap<String, String>, Fejl> {
        let mut st = self.db.prepare("SELECT zone, valg FROM notifikation_zone").map_err(lagerfejl)?;
        let rk = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Gemmer ét domænes valg; `None` (følg forudindstillingen) sletter rækken.
    pub fn gem_notifikation_zone(&mut self, zone: &str, valg: Option<&str>) -> Result<(), Fejl> {
        match valg {
            Some(v) => self.db.execute(
                "INSERT OR REPLACE INTO notifikation_zone (zone, valg) VALUES (?1, ?2)",
                params![zone, v],
            ),
            None => self.db.execute("DELETE FROM notifikation_zone WHERE zone = ?1", [zone]),
        }
        .map(|_| ())
        .map_err(lagerfejl)
    }

    /// Som skema 12 gør det.
    #[cfg(test)]
    pub fn db_ryd_hentet_angreb(&mut self) {
        self.db.execute("DELETE FROM hentet_angreb", []).unwrap();
    }

    /// Det seneste vurderede døgn pr. domæne.
    pub fn vurderet(&self) -> Result<HashMap<String, String>, Fejl> {
        let mut st = self.db.prepare("SELECT zone, dag FROM vurderet").map_err(lagerfejl)?;
        let rk = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Noterer (zone, døgn). Et døgn går aldrig tilbage. Én transaktion.
    pub fn gem_vurderet(&mut self, dage: &[(String, String)]) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut ind = tx
                .prepare("INSERT INTO vurderet (zone, dag) VALUES (?1, ?2) ON CONFLICT (zone) DO UPDATE SET dag = MAX(dag, excluded.dag)")
                .map_err(lagerfejl)?;
            for (zone, dag) in dage {
                ind.execute(params![zone, dag]).map_err(lagerfejl)?;
            }
        }
        tx.commit().map_err(lagerfejl)
    }

    /// Hvad der er meldt: (zone, statusdøgn) → det højeste meldte niveau.
    pub fn meldt(&self) -> Result<HashMap<(String, String), String>, Fejl> {
        let mut st = self.db.prepare("SELECT zone, dag, niveau FROM meldt").map_err(lagerfejl)?;
        let rk = st.query_map([], |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?))).map_err(lagerfejl)?;
        rk.collect::<Result<_, _>>().map_err(lagerfejl)
    }

    /// Noterer meldte domæner (zone, statusdøgn, niveau) kl. `kl` og glemmer dem, der er ældre end `foer_dag`. Én transaktion.
    pub fn gem_meldt(&mut self, meldte: &[(String, String, String)], kl: &str, foer_dag: &str) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        {
            let mut ind = tx
                .prepare("INSERT OR REPLACE INTO meldt (zone, dag, niveau, kl) VALUES (?1, ?2, ?3, ?4)")
                .map_err(lagerfejl)?;
            for (zone, dag, niveau) in meldte {
                ind.execute(params![zone, dag, niveau, kl]).map_err(lagerfejl)?;
            }
        }
        tx.execute("DELETE FROM meldt WHERE dag < ?1", [foer_dag]).map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)
    }

    /// Til Indstillinger → Data: ældste dag, nyeste dag og antal domæner med tal (alt, hvad der er hentet; også domæner, der
    /// ikke længere er på zonelisten). Ingen dage: `(None, None, 0)`.
    pub fn data_status(&self) -> Result<(Option<String>, Option<String>, i64), Fejl> {
        self.db
            .query_row("SELECT MIN(dag), MAX(dag), COUNT(DISTINCT zone) FROM hentet", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .map_err(lagerfejl)
    }

    /// "Glem tokenet": zonelisten, de valgte, de afviste og de nye domæner, notifikationsvalget pr. domæne, login-stierne, teknologierne og det meldte slettes (zonerne kan
    /// høre til en anden konto næste gang); Værnets tal (vagt_dag, hentet, hentet_ekstra) og indstillingerne bliver. Én transaktion.
    pub fn slet_zoner(&mut self) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        tx.execute_batch(
            "DELETE FROM zoner; DELETE FROM valgt; DELETE FROM afvist_ny; DELETE FROM ny_zone; DELETE FROM notifikation_zone; DELETE FROM login_sti; DELETE FROM meldt; DELETE FROM vurderet; DELETE FROM zone_tilstand; DELETE FROM teknologi; DELETE FROM tek_vis; DELETE FROM indstillinger WHERE noegle = 'domaene_valg';",
        )
        .map_err(lagerfejl)?;
        tx.commit().map_err(lagerfejl)
    }

    /// "Nulstil historik": alle hentede tal (vagt_dag, hentet, hentet_ekstra, hentet_besoeg, radar_dag, sikkerhedsfund og fejlene og tidspunkterne for dem) slettes i én transaktion;
    /// indstillingerne, zonelisten, valget, notifikationsvalget pr. domæne, login-stierne og det meldte bliver (J6: `meldt` er en log over beskeder, ikke hentede tal, og uden
    /// den kunne en besked for i går komme igen; den rydder sig selv efter `notify::MELDT_DOEGN`). `VACUUM` bagefter frigiver
    /// pladsen (det kan ikke stå i en transaktion). Filen selv bliver.
    pub fn nulstil_historik(&mut self) -> Result<(), Fejl> {
        let tx = self.db.transaction().map_err(lagerfejl)?;
        tx.execute_batch(
            "DELETE FROM vagt_dag; DELETE FROM hentet; DELETE FROM hentet_ekstra; DELETE FROM hentet_angreb; DELETE FROM hentet_laek; DELETE FROM hentet_dns; DELETE FROM hentet_dmarc; DELETE FROM hentet_historik; DELETE FROM hentet_besoeg; DELETE FROM hentet_robot; DELETE FROM hentet_side; DELETE FROM radar_dag; DELETE FROM sikkerhedsfund;",
        )
        .map_err(lagerfejl)?;
        for praefiks in [
            ADGANG_FEJL,
            DNS_FEJL,
            DMARC_FEJL,
            RADAR_PRAEFIKS,
            SIKKERHED_PRAEFIKS,
            BESOEG_FEJL,
            ROBOT_FEJL,
            SIDE_FEJL,
            UNIKKE_MANGLER,
            IPKLASSE_MANGLER,
        ] {
            tx.execute("DELETE FROM indstillinger WHERE noegle LIKE ?1", [format!("{praefiks}%")])
                .map_err(lagerfejl)?;
        }
        tx.commit().map_err(lagerfejl)?;
        self.db.execute_batch("VACUUM").map_err(lagerfejl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rk(art: &str, noegle: &str, n: i64) -> (String, String, i64) {
        (art.into(), noegle.into(), n)
    }

    fn zone(id: &str, navn: &str) -> Zone {
        Zone {
            id: id.into(),
            navn: navn.into(),
        }
    }

    /// Skema 15: to nye tabeller, og `hentet_laek` tømmes, så døgnene hentes igen uden krav om POST. Rækkerne står, til deres døgn er hentet igen, og intet andet ændres.
    #[test]
    fn skema_15_henter_laek_igen_og_roerer_intet_andet() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_laek("z", "2026-11-01", &[rk("laek", "/a\tpassword_leaked\t200", 2)], "t")
            .unwrap();
        s.gem_ekstra("z", "2026-11-01", &[rk("post", "/a\t200\t200\tDK\t", 1)], "t")
            .unwrap();
        s.db.execute_batch("PRAGMA user_version = 14; DROP TABLE hentet_dns; DROP TABLE hentet_dmarc;")
            .unwrap();
        let s = Store::klar(std::mem::replace(&mut s.db, Connection::open_in_memory().unwrap())).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert!(s.hentet_laek("z").unwrap().is_empty());
        assert_eq!(s.hentet_ekstra("z").unwrap().len(), 1);
        assert_eq!(s.raekker("z", "0000-00-00").unwrap().len(), 2, "rækkerne står");
        assert!(s.hentet_dns("z").unwrap().is_empty() && s.hentet_dmarc("z").unwrap().is_empty());
    }

    /// Skema 16: tabellen `sikkerhedsfund` kommer til, og intet andet ændres. Fundene erstattes helt ved hver hentning.
    #[test]
    fn skema_16_tilfoejer_sikkerhedsfund_og_roerer_intet_andet() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_doegn("z", "2026-11-01", &[rk("fw", "a", 3)], "t").unwrap();
        s.gem_indstilling("x", "y").unwrap();
        s.db.execute_batch("PRAGMA user_version = 15; DROP TABLE sikkerhedsfund;").unwrap();
        let mut s = Store::klar(std::mem::replace(&mut s.db, Connection::open_in_memory().unwrap())).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert_eq!(s.raekker("z", "0000-00-00").unwrap().len(), 1, "rækkerne står");
        assert_eq!(s.indstilling("x").unwrap().as_deref(), Some("y"));
        assert!(s.sikkerhedsfund(None).unwrap().is_empty());
        let f = |zone: &str, id: &str, alvor: &str| crate::sikkerhed::Fund {
            zone: zone.into(),
            id: id.into(),
            alvor: alvor.into(),
            klasse: "k".into(),
            type_: "t".into(),
            siden: "2026-04-24".into(),
        };
        s.gem_sikkerhedsfund(&[f("z", "z-a", "Low"), f("y", "y-a", "Critical")], "t1")
            .unwrap();
        assert_eq!(s.sikkerhedsfund(Some("z")).unwrap(), vec![f("z", "z-a", "Low")]);
        assert_eq!(s.sikkerhedsfund(None).unwrap().len(), 2);
        s.gem_sikkerhedsfund(&[f("y", "y-b", "Low")], "t2").unwrap();
        assert_eq!(s.sikkerhedsfund(None).unwrap(), vec![f("y", "y-b", "Low")], "erstattes helt");
        let sql: String =
            s.db.query_row("SELECT sql FROM sqlite_master WHERE name = 'sikkerhedsfund'", [], |r| r.get(0))
                .unwrap();
        assert!(
            !sql.to_lowercase().contains("konto") && !sql.to_lowercase().contains("account") && !sql.contains("navn"),
            "{sql}"
        );
        s.gem_indstilling(crate::sikkerhed::HENTET, "t2").unwrap();
        s.gem_indstilling(crate::sikkerhed::TILSTAND, "ok").unwrap();
        s.nulstil_historik().unwrap();
        assert!(s.sikkerhedsfund(None).unwrap().is_empty(), "nulstil sletter fundene");
        assert_eq!(s.indstilling(crate::sikkerhed::HENTET).unwrap(), None);
        assert_eq!(s.indstilling(crate::sikkerhed::TILSTAND).unwrap(), None);
        assert_eq!(s.indstilling("x").unwrap().as_deref(), Some("y"), "indstillingerne bliver");
    }

    /// Ingen kolonne til en IP-adresse: DNS og DMARC ligger i `vagt_dag`, og tabellerne har kun zone, dag og tidspunkt.
    #[test]
    fn dns_og_dmarc_har_ingen_ip_i_skemaet() {
        let s = Store::i_hukommelse().unwrap();
        for t in ["hentet_dns", "hentet_dmarc", "vagt_dag"] {
            let sql: String =
                s.db.query_row("SELECT sql FROM sqlite_master WHERE name = ?1", [t], |r| r.get(0))
                    .unwrap();
            assert!(!sql.to_lowercase().contains("ip"), "{t}: {sql}");
        }
    }

    /// `gem_dns` og `gem_dmarc` erstatter kun deres egne arter, og "Nulstil historik" tager dem med.
    #[test]
    fn dns_og_dmarc_gemmes_for_sig() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_laek(
            "z",
            "2026-11-01",
            &[rk("laek", "/a\tpassword_leaked\t200", 2), rk("laek_ren", "/a", 3)],
            "t",
        )
        .unwrap();
        s.gem_dns("z", "2026-11-01", &[rk("dns_kode", "NXDOMAIN", 9)], "t").unwrap();
        s.gem_dns("z", "2026-11-01", &[rk("dns_kode", "NOERROR", 1)], "t2").unwrap();
        s.gem_dmarc("z", "2026-11-01", &[rk("dmarc", "A\tDK\tfail\tfail", 4)], "t").unwrap();
        let arter = |s: &Store| {
            let mut a: Vec<(String, i64)> = s.raekker("z", "0000-00-00").unwrap().into_iter().map(|r| (r.art, r.n)).collect();
            a.sort();
            a
        };
        assert_eq!(
            arter(&s),
            [
                ("dmarc".to_string(), 4),
                ("dns_kode".to_string(), 1),
                ("laek".to_string(), 2),
                ("laek_ren".to_string(), 3)
            ]
        );
        assert_eq!(s.hentet_dns("z").unwrap()["2026-11-01"], "t2");
        s.gem_laek("z", "2026-11-01", &[], "t").unwrap();
        assert_eq!(arter(&s).len(), 2, "`laek_ren` erstattes med `laek`");
        s.gem_afsnit_fejl(DMARC_FEJL, "z", Some("nej")).unwrap();
        assert_eq!(s.afsnit_fejl(DMARC_FEJL, "z").unwrap().as_deref(), Some("nej"));
        assert_eq!(s.afsnit_fejl(DNS_FEJL, "z").unwrap(), None);
        s.nulstil_historik().unwrap();
        assert!(s.hentet_dns("z").unwrap().is_empty() && s.hentet_dmarc("z").unwrap().is_empty());
        assert_eq!(s.afsnit_fejl(DMARC_FEJL, "z").unwrap(), None);
    }

    /// Skema 11: kun en ny tabel. Et nyt døgn markeres, og `gem_angreb` rører kun art `angreb`.
    #[test]
    fn enkelthaendelser_hentes_og_nulstilles() {
        let mut s = Store::i_hukommelse().unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        let rk = |art: &str| (art.to_string(), "x".to_string(), 1);
        s.gem_doegn("z", "2026-11-01", &[rk("fw"), rk("angreb")], "t").unwrap();
        assert!(s.hentet_angreb("z").unwrap().contains_key("2026-11-01"));
        s.gem_angreb("z", "2026-11-01", &[("angreb".into(), "y".into(), 2)], "t").unwrap();
        let alle = s.raekker("z", "2026-01-01").unwrap();
        assert_eq!(alle.len(), 2, "fw står");
        assert!(alle.iter().any(|r| r.art == "angreb" && r.noegle == "y"));
        assert_eq!(s.raekker_tal("z", "2026-01-01").unwrap().len(), 1, "overblikket læser dem ikke");
        s.gem_angreb("z", "2026-10-01", &[], "t").unwrap();
        assert_eq!(s.hentet("z").unwrap().len(), 1, "et gammelt døgn er ikke hentet af den grund");
        s.nulstil_historik().unwrap();
        assert!(s.hentet_angreb("z").unwrap().is_empty());
    }

    #[test]
    fn nulstil_teknologi_for_et_domaene_eller_alle() {
        let mut s = Store::i_hukommelse().unwrap();
        for (z, t) in [("a", "react"), ("a", "php"), ("b", "react")] {
            s.gem_teknologi(z, t, "bruger_ikke", false, "t").unwrap();
        }
        s.nulstil_teknologi(Some("a")).unwrap();
        assert_eq!(
            s.teknologi_valg().unwrap(),
            [("b".to_string(), "react".to_string(), "bruger_ikke".to_string())]
        );
        s.gem_teknologi("a", "php", "bruger", false, "t").unwrap();
        s.nulstil_teknologi(None).unwrap();
        assert!(s.teknologi_valg().unwrap().is_empty());
    }

    #[test]
    fn skema_og_version() {
        let s = Store::i_hukommelse().unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert!(s.hentet("z").unwrap().is_empty());
        assert_eq!(s.zoner().unwrap(), (vec![], None));
        assert!(s.valg().unwrap().is_empty());
    }

    #[test]
    fn migrering_fra_version_1_bevarer_vaernets_tal() {
        // En database præcis som version 1 lavede den (skema + user_version = 1 + data).
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(SKEMA_1).unwrap();
        db.pragma_update(None, "user_version", 1).unwrap();
        db.execute(
            "INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-10-11', 'fw', 'k', 31)",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO hentet (zone, dag, hentet_utc) VALUES ('z1', '2026-10-11', '2026-11-03T06:55:55.180Z')",
            [],
        )
        .unwrap();
        let har_zoner = |db: &Connection| -> i64 {
            db.query_row("SELECT count(*) FROM sqlite_master WHERE name = 'zoner'", [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(har_zoner(&db), 0);

        let mut s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION, "1 → 2 → … → 10 i én omgang");
        assert_eq!(har_zoner(&s.db), 1);
        assert_eq!(s.raekker("z1", "2026-10-01").unwrap()[0].n, 31, "Værnets tal er urørte");
        assert_eq!(s.hentet("z1").unwrap()["2026-10-11"], "2026-11-03T06:55:55.180Z");
        s.gem_zoner(&[zone("z1", "a.wi24rd.com")], "2026-11-03T08:00:00.000Z").unwrap();
        assert_eq!(s.zoner().unwrap().0, vec![zone("z1", "a.wi24rd.com")]);

        // Åbnes igen (allerede version 9): intet sker.
        let db = s.db;
        let s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert_eq!(s.zoner().unwrap().0.len(), 1);
    }

    /// 9 → 10: teknologi kommer til, de gamle Radar-serier går, og Værnets tal, de andre Radar-tal og valget bliver.
    #[test]
    fn migrering_fra_version_9_tilfoejer_teknologi() {
        let db = Connection::open_in_memory().unwrap();
        for skema in [SKEMA_1, SKEMA_2, SKEMA_3, SKEMA_4, SKEMA_5, SKEMA_6, SKEMA_7, SKEMA_8, SKEMA_9] {
            db.execute_batch(skema).unwrap();
        }
        db.pragma_update(None, "user_version", 9).unwrap();
        db.execute_batch(
            "INSERT INTO valgt (zone) VALUES ('z1');
             INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-11-02', 'fw', 'k', 7);
             INSERT INTO radar_dag (serie, land, dag, noegle, vaerdi, normalisering, hentet_utc) VALUES
               ('mod7', 'DK', '2026-11-02', 'US', 1, 'P', 't'), ('kilde30', '', '2026-11-02', 'US', 1, 'P', 't'), ('par', 'DK', '2026-11-02', 'US', 1, 'P', 't'),
               ('maal', '', '2026-11-02', 'DK', 1, 'P', 't'), ('udfald', 'DK', '2026-11-02', '', 0, '', 't');",
        )
        .unwrap();
        let s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert!(s.teknologi("z1").unwrap().is_empty());
        assert_eq!(s.raekker("z1", "2026-11-01").unwrap().len(), 1);
        let serier: Vec<String> = {
            let mut st = s.db.prepare("SELECT serie FROM radar_dag ORDER BY serie").unwrap();
            let rk = st.query_map([], |r| r.get(0)).unwrap();
            rk.map(Result::unwrap).collect()
        };
        assert_eq!(serier, ["maal", "udfald"]);
    }

    /// 12 → 13: tre nye tabeller, og ingen række ændres.
    #[test]
    fn migrering_fra_version_12_tilfoejer_og_roerer_intet() {
        let db = Connection::open_in_memory().unwrap();
        for skema in [
            SKEMA_1, SKEMA_2, SKEMA_3, SKEMA_4, SKEMA_5, SKEMA_6, SKEMA_7, SKEMA_8, SKEMA_9, SKEMA_10, SKEMA_11, SKEMA_12,
        ] {
            db.execute_batch(skema).unwrap();
        }
        db.pragma_update(None, "user_version", 12).unwrap();
        db.execute_batch(
            "INSERT INTO hentet_angreb (zone, dag, hentet_utc) VALUES ('z1', '2026-11-02', 't');
             INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-11-02', 'fw', 'k', 9);",
        )
        .unwrap();
        let s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert_eq!(s.hentet_angreb("z1").unwrap().len(), 1);
        assert_eq!(s.raekker("z1", "2026-11-01").unwrap().len(), 1);
        assert!(
            s.hentet_laek("z1").unwrap().is_empty()
                && s.hentet_historik("z1").unwrap().is_empty()
                && s.zone_tilstande().unwrap().is_empty()
        );
    }

    /// Årstallene står for sig: de blander sig ikke i `raekker` og forsvinder med "Nulstil historik". Døgn uden rækker er også hentet.
    #[test]
    fn aarstallene_gemmes_uden_at_blande_sig() {
        let mut s = Store::i_hukommelse().unwrap();
        let dage = ["2026-01-01".to_string(), "2026-01-02".to_string()];
        let rk = |n| vec![("2026-01-01".to_string(), "h_total".to_string(), "foresp".to_string(), n)];
        s.gem_historik("z1", &dage, &rk(5), "t").unwrap();
        s.gem_historik("z1", &dage, &rk(7), "t2").unwrap();
        assert_eq!(s.hentet_historik("z1").unwrap().len(), 2);
        let z = ["z1".to_string()];
        assert_eq!(s.aeldste(&z).unwrap().trafik_fra.as_deref(), Some("2026-01-01"));
        let aar = s.raekker_art(&["z1".to_string()], &["h_total"], "0000-00-00").unwrap();
        assert_eq!((aar.len(), aar[0].1.n), (1, 7), "erstattes");
        assert!(s.raekker("z1", "0000-00-00").unwrap().is_empty());
        s.nulstil_historik().unwrap();
        assert!(s.hentet_historik("z1").unwrap().is_empty() && s.aeldste(&z).unwrap().trafik_fra.is_none());
    }

    /// Et hentet døgn uden trafik tæller ikke, og fravalgte domæner tæller ikke.
    #[test]
    fn aeldste_dag_er_for_de_valgte_og_kun_med_tal() {
        let mut s = Store::i_hukommelse().unwrap();
        let dage: Vec<String> = ["2025-11-07", "2026-01-01", "2026-03-05"].map(String::from).to_vec();
        let rk = |dag: &str| vec![(dag.to_string(), "h_total".to_string(), "foresp".to_string(), 5)];
        let to = [rk("2026-03-05"), rk("2026-03-06")].concat();
        s.gem_historik("z1", &[dage.clone(), vec!["2026-03-06".to_string()]].concat(), &to, "t")
            .unwrap();
        s.gem_historik("z2", &dage, &rk("2026-01-01"), "t").unwrap();
        s.gem_doegn(
            "z1",
            "2026-10-04",
            &[("alle".to_string(), "x".to_string(), 1)],
            "2026-11-06T00:00:00Z",
        )
        .unwrap();
        let a = s.aeldste(&["z1".to_string()]).unwrap();
        assert_eq!(
            a.trafik_fra.as_deref(),
            Some("2026-03-05"),
            "ikke 7. nov. 2025, det ældste hentede døgn"
        );
        assert_eq!(a.sikkerhed_fra.as_deref(), Some("2026-10-04"));
        assert_eq!(a.sidste.as_deref(), Some("2026-10-04"));
        assert_eq!(s.aeldste(&["z2".to_string()]).unwrap().trafik_fra.as_deref(), Some("2026-01-01"));
        assert_eq!(s.aeldste(&[]).unwrap(), Aeldste::default());
    }

    /// 13 → 14: `tek_vis` er ny, `hentet_laek` tømmes, og valgene og tallene står.
    #[test]
    fn migrering_fra_version_13_henter_laek_igen_og_roerer_ikke_valg() {
        let db = Connection::open_in_memory().unwrap();
        for skema in [
            SKEMA_1, SKEMA_2, SKEMA_3, SKEMA_4, SKEMA_5, SKEMA_6, SKEMA_7, SKEMA_8, SKEMA_9, SKEMA_10, SKEMA_11, SKEMA_12, SKEMA_13,
        ] {
            db.execute_batch(skema).unwrap();
        }
        db.pragma_update(None, "user_version", 13).unwrap();
        db.execute_batch(
            "INSERT INTO hentet_laek (zone, dag, hentet_utc) VALUES ('z1', '2026-11-02', 't');
             INSERT INTO teknologi (zone, tek, valg, besked, sat_utc) VALUES ('z1', 'react', 'bruger_ikke', 0, 't');
             INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-11-02', 'laek', 'k', 9);",
        )
        .unwrap();
        let s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert!(s.hentet_laek("z1").unwrap().is_empty());
        assert_eq!(
            s.teknologi("z1").unwrap(),
            [("react".to_string(), "bruger_ikke".to_string(), false)]
        );
        assert_eq!(s.raekker_art(&["z1".to_string()], &["laek"], "0000-00-00").unwrap().len(), 1);
        assert!(s.tek_vis().unwrap().is_empty());
    }

    /// "Vis i tabellen" følger kontoen, rører aldrig et valg, og et fravalg står.
    #[test]
    fn tek_vis_gemmes_pr_konto_og_roerer_ikke_valg() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_teknologi("z1", "react", "bruger_ikke", false, "t").unwrap();
        s.tilfoej_tek_vis(&[("react".to_string(), true, true, false)]).unwrap();
        s.saet_tek_vis("react", false).unwrap();
        assert_eq!(
            s.tek_vis().unwrap(),
            [("react".to_string(), false, false, false)],
            "kvitteret og skjult"
        );
        s.tilfoej_tek_vis(&[("react".to_string(), true, true, false)]).unwrap();
        assert!(!s.tek_vis().unwrap()[0].1, "et fravalg står");
        assert_eq!(s.teknologi("z1").unwrap().len(), 1, "valget står");
        s.nulstil_historik().unwrap();
        assert_eq!(s.tek_vis().unwrap().len(), 1, "Nulstil historik er ikke Glem");
        s.slet_zoner().unwrap();
        assert!(s.tek_vis().unwrap().is_empty());
    }

    #[test]
    fn zonens_tilstand_gemmes_og_ryddes_med_kontoen() {
        let mut s = Store::i_hukommelse().unwrap();
        let z = crate::cf::ZoneStand {
            status: "active".into(),
            paused: true,
            dev_sek: 60,
            phishing: true,
        };
        s.gem_zone_tilstand("z1", &z, "t").unwrap();
        assert_eq!(s.zone_tilstande().unwrap()["z1"], (z, "t".to_string()));
        s.slet_zoner().unwrap();
        assert!(s.zone_tilstande().unwrap().is_empty());
    }

    /// 11 → 12: `vurderet` kommer til, og `hentet_angreb` tømmes, så døgnene hentes igen med netværket. De gemte angreb bliver stående, til det er sket.
    #[test]
    fn migrering_fra_version_11_henter_angrebene_igen_og_bevarer_tallene() {
        let db = Connection::open_in_memory().unwrap();
        for skema in [
            SKEMA_1, SKEMA_2, SKEMA_3, SKEMA_4, SKEMA_5, SKEMA_6, SKEMA_7, SKEMA_8, SKEMA_9, SKEMA_10, SKEMA_11,
        ] {
            db.execute_batch(skema).unwrap();
        }
        db.pragma_update(None, "user_version", 11).unwrap();
        db.execute_batch(
            "INSERT INTO hentet_angreb (zone, dag, hentet_utc) VALUES ('z1', '2026-11-02', 't');
             INSERT INTO hentet_ekstra (zone, dag, hentet_utc) VALUES ('z1', '2026-11-02', 't');
             INSERT INTO meldt (zone, dag, niveau, kl) VALUES ('z1', '2026-11-02', 'gul', 't');
             INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-11-02', 'angreb', 'k', 7), ('z1', '2026-11-02', 'fw', 'k', 9);",
        )
        .unwrap();
        let s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert!(s.hentet_angreb("z1").unwrap().is_empty());
        assert!(s.vurderet().unwrap().is_empty());
        assert_eq!(s.hentet_ekstra("z1").unwrap().len(), 1);
        assert_eq!(s.meldt().unwrap().len(), 1);
        assert_eq!(s.raekker("z1", "2026-11-01").unwrap().len(), 2, "også art angreb bliver");
    }

    #[test]
    fn vurderet_gaar_aldrig_tilbage_og_ryddes_med_kontoen() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_vurderet(&[("z1".into(), "2026-11-02".into())]).unwrap();
        s.gem_vurderet(&[("z1".into(), "2026-10-20".into()), ("z2".into(), "2026-11-01".into())])
            .unwrap();
        let v = s.vurderet().unwrap();
        assert_eq!((v["z1"].as_str(), v["z2"].as_str()), ("2026-11-02", "2026-11-01"));
        s.slet_zoner().unwrap();
        assert!(s.vurderet().unwrap().is_empty());
    }

    /// Cloudflare har ikke døgnet længere: det, der er gemt, bliver stående.
    #[test]
    fn noter_angreb_sletter_intet() {
        let mut s = Store::i_hukommelse().unwrap();
        let rk = [("angreb".to_string(), "k".to_string(), 3)];
        s.gem_angreb("z1", "2026-10-01", &rk, "t").unwrap();
        s.db.execute("DELETE FROM hentet_angreb", []).unwrap();
        s.noter_angreb("z1", "2026-10-01", "t2").unwrap();
        assert_eq!(s.hentet_angreb("z1").unwrap()["2026-10-01"], "t2");
        assert_eq!(s.raekker("z1", "2026-10-01").unwrap().len(), 1);
    }

    #[test]
    fn teknologi_ryddes_med_kontoen_og_bliver_ved_fravalg() {
        let mut s = Store::i_hukommelse().unwrap();
        let valg = |s: &Store, z: &str| s.teknologi(z).unwrap();
        s.gem_zoner(&[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t").unwrap();
        s.gem_teknologi("z1", "wordpress", "bruger_ikke", false, "t").unwrap();
        s.gem_teknologi("z1", "php", "bruger", true, "t").unwrap();
        assert_eq!(
            valg(&s, "z1"),
            [
                ("php".to_string(), "bruger".to_string(), true),
                ("wordpress".to_string(), "bruger_ikke".to_string(), false)
            ]
        );
        s.gem_teknologi("z2", "react", "bruger", false, "t").unwrap();
        s.gem_teknologi("z2", "php", "bruger_ikke", false, "t").unwrap();
        s.kopier_teknologi("z1", &["z1".to_string(), "z2".to_string()], "t2").unwrap();
        assert_eq!(valg(&s, "z2").len(), 3, "kun det, z1 har taget stilling til, ændres");
        assert_eq!(valg(&s, "z2")[0], ("php".to_string(), "bruger".to_string(), true));
        s.gem_teknologi("z1", "php", "ved_ikke", false, "t").unwrap();
        assert_eq!(valg(&s, "z1").len(), 1, "Ved ikke er ingen række");
        s.gem_valgte(&["z2".to_string()]).unwrap();
        assert_eq!(valg(&s, "z1").len(), 1, "fravalg af domænet bevarer valget");
        s.nulstil_historik().unwrap();
        assert_eq!(valg(&s, "z1").len(), 1, "Nulstil historik sletter tal, ikke valg");
        s.gem_zoner(&[zone("z9", "anden.wi24rd.com")], "t").unwrap();
        assert!(valg(&s, "z1").is_empty(), "en anden konto starter forfra");
        s.gem_zoner(&[zone("z1", "a.wi24rd.com")], "t").unwrap();
        s.gem_teknologi("z1", "php", "bruger", false, "t").unwrap();
        s.slet_zoner().unwrap();
        assert!(valg(&s, "z1").is_empty(), "Glem API Token");
    }

    #[test]
    fn nyere_database_afvises() {
        let db = Connection::open_in_memory().unwrap();
        db.pragma_update(None, "user_version", SKEMA_VERSION + 1).unwrap();
        assert_eq!(Store::klar(db).err(), Some(Fejl::NyereDatabase(SKEMA_VERSION + 1)));
    }

    /// En database præcis som version 2 lavede den (med tal og zoneliste) får tabellen `valg`; intet andet røres.
    #[test]
    fn migrering_fra_version_2_til_3_tilfoejer_valg() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(SKEMA_1).unwrap();
        db.execute_batch(SKEMA_2).unwrap();
        db.pragma_update(None, "user_version", 2).unwrap();
        db.execute_batch(
            "INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-11-02', 'fw', 'k', 1260);
             INSERT INTO hentet (zone, dag, hentet_utc) VALUES ('z1', '2026-11-02', '2026-11-03T08:01:00.000Z');
             INSERT INTO zoner (id, navn, hentet_utc) VALUES ('z1', 'a.wi24rd.com', '2026-11-03T08:00:00.000Z'), ('z2', 'b.wi24rd.com', '2026-11-03T08:00:00.000Z');",
        )
        .unwrap();
        let har_valg = |db: &Connection| -> i64 {
            db.query_row("SELECT count(*) FROM sqlite_master WHERE name = 'valg'", [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(har_valg(&db), 0);
        let mut s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert_eq!(har_valg(&s.db), 0, "valg er blevet til valgt");
        assert_eq!(s.raekker("z1", "2026-11-01").unwrap()[0].n, 1260, "Værnets tal er urørte");
        assert_eq!(s.hentet("z1").unwrap().len(), 1);
        assert_eq!(s.zoner().unwrap().0.len(), 2, "zonelisten er urørt");
        assert_eq!(s.valg().unwrap(), ["z1", "z2"], "standard ved første start: alle valgt");
        s.gem_valgte(&["z2".into()]).unwrap();
        let s = Store::klar(s.db).unwrap();
        assert_eq!(s.valg().unwrap(), ["z2"], "valget overlever en genåbning");
    }

    /// En database præcis som version 3 lavede den får tabellen `indstillinger`; intet andet røres.
    #[test]
    fn migrering_fra_version_3_til_4_tilfoejer_indstillinger() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(SKEMA_1).unwrap();
        db.execute_batch(SKEMA_2).unwrap();
        db.execute_batch(SKEMA_3).unwrap();
        db.pragma_update(None, "user_version", 3).unwrap();
        db.execute_batch(
            "INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-11-02', 'fw', 'k', 1260);
             INSERT INTO hentet (zone, dag, hentet_utc) VALUES ('z1', '2026-11-02', '2026-11-03T08:01:00.000Z');
             INSERT INTO zoner (id, navn, hentet_utc) VALUES ('z1', 'a.wi24rd.com', '2026-11-03T08:00:00.000Z');
             INSERT INTO valg (zone) VALUES ('*'), ('z1');",
        )
        .unwrap();
        let har = |db: &Connection| -> i64 {
            db.query_row("SELECT count(*) FROM sqlite_master WHERE name = 'indstillinger'", [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(har(&db), 0);
        let mut s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert_eq!(har(&s.db), 1);
        assert_eq!(s.raekker("z1", "2026-11-01").unwrap()[0].n, 1260, "Værnets tal er urørte");
        assert_eq!(s.zoner().unwrap().0.len(), 1, "zonelisten er urørt");
        assert_eq!(s.valg().unwrap(), ["z1"], "valget er urørt");
        assert_eq!(s.indstilling("autostart_standard").unwrap(), None, "intet gemt endnu");
        s.gem_indstilling("autostart_standard", "1").unwrap();
        s.gem_indstilling("sprog", "da").unwrap();
        s.gem_indstilling("sprog", "en").unwrap();
        let s = Store::klar(s.db).unwrap();
        assert_eq!(
            s.indstilling("autostart_standard").unwrap().as_deref(),
            Some("1"),
            "overlever en genåbning"
        );
        assert_eq!(s.indstilling("sprog").unwrap().as_deref(), Some("en"), "erstattes, ikke dubleret");
    }

    /// En database præcis som version 4 lavede den får `notifikation_zone` og `meldt`; intet andet røres.
    #[test]
    fn skema_4_til_5_bevarer_data() {
        let db = Connection::open_in_memory().unwrap();
        for skema in [SKEMA_1, SKEMA_2, SKEMA_3, SKEMA_4] {
            db.execute_batch(skema).unwrap();
        }
        db.pragma_update(None, "user_version", 4).unwrap();
        db.execute_batch(
            "INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-11-02', 'fw', 'k', 1260);
             INSERT INTO hentet (zone, dag, hentet_utc) VALUES ('z1', '2026-11-02', '2026-11-03T08:01:00.000Z');
             INSERT INTO zoner (id, navn, hentet_utc) VALUES ('z1', 'a.wi24rd.com', '2026-11-03T08:00:00.000Z');
             INSERT INTO valg (zone) VALUES ('*'), ('z1');
             INSERT INTO indstillinger (noegle, vaerdi) VALUES ('sprog', 'en'), ('autostart_standard', '1');",
        )
        .unwrap();
        let har = |db: &Connection, navn: &str| -> i64 {
            db.query_row("SELECT count(*) FROM sqlite_master WHERE name = ?1", [navn], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(har(&db, "meldt") + har(&db, "notifikation_zone"), 0);
        let mut s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert_eq!(har(&s.db, "meldt"), 1);
        assert_eq!(har(&s.db, "notifikation_zone"), 1);
        assert_eq!(s.raekker("z1", "2026-11-01").unwrap()[0].n, 1260, "Værnets tal er urørte");
        assert_eq!(s.hentet("z1").unwrap().len(), 1);
        assert_eq!(s.zoner().unwrap().0.len(), 1, "zonelisten er urørt");
        assert_eq!(s.valg().unwrap(), ["z1"], "valget er urørt");
        assert_eq!(s.indstilling("sprog").unwrap().as_deref(), Some("en"), "indstillingerne er urørte");
        assert!(s.notifikation_zoner().unwrap().is_empty() && s.meldt().unwrap().is_empty());
        s.gem_notifikation_zone("z1", Some("fra")).unwrap();
        s.gem_notifikation_zone("z1", Some("alt")).unwrap();
        s.gem_notifikation_zone("z2", Some("roed")).unwrap();
        s.gem_notifikation_zone("z2", None).unwrap();
        let s = Store::klar(s.db).unwrap();
        assert_eq!(
            s.notifikation_zoner().unwrap(),
            HashMap::from([("z1".into(), "alt".into())]),
            "erstattes; None sletter"
        );
    }

    /// En database præcis som version 5 lavede den får `login_sti` og `hentet_ekstra`; intet andet røres.
    #[test]
    fn skema_5_til_6_bevarer_data() {
        let db = Connection::open_in_memory().unwrap();
        for skema in [SKEMA_1, SKEMA_2, SKEMA_3, SKEMA_4, SKEMA_5] {
            db.execute_batch(skema).unwrap();
        }
        db.pragma_update(None, "user_version", 5).unwrap();
        db.execute_batch(
            "INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-11-02', 'fw', 'k', 1260), ('z1', '2026-11-02', 'bot', 'b', 7);
             INSERT INTO hentet (zone, dag, hentet_utc) VALUES ('z1', '2026-11-02', '2026-11-03T08:01:00.000Z');
             INSERT INTO zoner (id, navn, hentet_utc) VALUES ('z1', 'a.wi24rd.com', '2026-11-03T08:00:00.000Z');
             INSERT INTO meldt (zone, dag, niveau, kl) VALUES ('z1', '2026-11-02', 'roed', '2026-11-03T08:00:00.000Z');
             INSERT INTO notifikation_zone (zone, valg) VALUES ('z1', 'fra');",
        )
        .unwrap();
        let mut s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert_eq!(s.raekker("z1", "2026-11-01").unwrap().len(), 2, "Værnets tal er urørte");
        assert_eq!(s.hentet("z1").unwrap().len(), 1);
        assert_eq!(s.meldt().unwrap().len(), 1);
        assert_eq!(s.notifikation_zoner().unwrap()["z1"], "fra");
        assert!(s.hentet_ekstra("z1").unwrap().is_empty(), "alt skal hentes første gang");
        assert!(s.login_stier("z1").unwrap().is_empty(), "ingen stier er slået til af sig selv");
        s.gem_login_sti("z1", "/login", true).unwrap();
        s.gem_login_sti("z1", "/login", false).unwrap();
        s.gem_login_sti("z1", "/admin/", true).unwrap();
        assert_eq!(
            s.login_stier("z1").unwrap(),
            [("/admin/".to_string(), true), ("/login".to_string(), false)],
            "erstattes, ikke dubleret"
        );
    }

    /// `gem_doegn` kører hver time for i dag og i går: de ekstra tal må ikke forsvinde, og omvendt.
    #[test]
    fn doegn_og_ekstra_erstatter_kun_egne_arter() {
        let mut s = Store::i_hukommelse().unwrap();
        let arter = |s: &Store| {
            let mut a: Vec<String> = s.raekker("z1", "2026-11-01").unwrap().into_iter().map(|r| r.art).collect();
            a.sort();
            a
        };
        s.gem_doegn("z1", "2026-11-02", &[rk("fw", "a", 1)], "t1").unwrap();
        s.gem_ekstra("z1", "2026-11-02", &[rk("login", "x", 2), rk("stoppet", "y", 3)], "t2")
            .unwrap();
        assert_eq!(arter(&s), ["fw", "login", "stoppet"]);
        s.gem_doegn("z1", "2026-11-02", &[rk("alle", "", 9)], "t3").unwrap();
        assert_eq!(arter(&s), ["alle", "login", "stoppet"], "gem_doegn bevarer de ekstra arter");
        s.gem_ekstra("z1", "2026-11-02", &[rk("post", "p", 4)], "t4").unwrap();
        assert_eq!(arter(&s), ["alle", "post"], "gem_ekstra erstatter sine egne og bevarer døgnets");
        assert_eq!(s.hentet("z1").unwrap()["2026-11-02"], "t3", "hver sin notering");
        assert_eq!(s.hentet_ekstra("z1").unwrap()["2026-11-02"], "t4");
        s.glem_hentet_ekstra("z1").unwrap();
        assert!(s.hentet_ekstra("z1").unwrap().is_empty());
        assert_eq!(arter(&s), ["alle", "post"], "tallene bliver, til døgnet er hentet igen");
    }

    #[test]
    fn arterne_overlapper_ikke() {
        assert!(DOEGN_ARTER.iter().all(|a| !EKSTRA_ARTER.contains(a)));
    }

    #[test]
    fn adgang_fejl_noteres_og_ryddes() {
        let mut s = Store::i_hukommelse().unwrap();
        assert_eq!(s.adgang_fejl("z1").unwrap(), None);
        s.gem_adgang_fejl("z1", Some("graphql")).unwrap();
        assert_eq!(s.adgang_fejl("z1").unwrap().as_deref(), Some("graphql"));
        s.gem_adgang_fejl("z1", None).unwrap();
        assert_eq!(s.adgang_fejl("z1").unwrap(), None);
    }

    /// "Nulstil historik": de ekstra tal skal hentes igen, men stierne er et valg og bliver. "Glem tokenet" er omvendt.
    #[test]
    fn nulstil_og_glem_ved_ekstra_tal_og_stier() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_ekstra("z1", "2026-11-02", &[rk("login", "x", 2)], "t").unwrap();
        s.gem_login_sti("z1", "/login", true).unwrap();
        s.gem_adgang_fejl("z2", Some("graphql")).unwrap();
        s.gem_indstilling("sprog", "da").unwrap();
        s.nulstil_historik().unwrap();
        assert!(s.hentet_ekstra("z1").unwrap().is_empty());
        assert_eq!(s.adgang_fejl("z2").unwrap(), None);
        assert_eq!(s.indstilling("sprog").unwrap().as_deref(), Some("da"), "kun fejlene ryger");
        assert_eq!(s.login_stier("z1").unwrap().len(), 1);
        s.gem_ekstra("z1", "2026-11-02", &[rk("login", "x", 2)], "t").unwrap();
        s.slet_zoner().unwrap();
        assert!(s.login_stier("z1").unwrap().is_empty());
        assert_eq!(s.hentet_ekstra("z1").unwrap().len(), 1, "tallene bliver som Værnets");
    }

    #[test]
    fn meldt_ryddes_efter_40_doegn() {
        let mut s = Store::i_hukommelse().unwrap();
        let m = |z: &str, d: &str, n: &str| (z.to_string(), d.to_string(), n.to_string());
        s.gem_meldt(
            &[m("z1", "2026-09-20", "roed"), m("z1", "2026-09-24", "gul")],
            "2026-09-25T06:00:00.000Z",
            "2026-08-16",
        )
        .unwrap();
        assert_eq!(s.meldt().unwrap().len(), 2);
        // 40 døgn før 2026-11-03 er 2026-09-24: det ældre døgn glemmes, det på grænsen bliver.
        s.gem_meldt(
            &[m("z1", "2026-09-24", "roed"), m("z2", "2026-11-02", "roed")],
            "2026-11-03T06:00:00.000Z",
            "2026-09-24",
        )
        .unwrap();
        let meldt = s.meldt().unwrap();
        assert_eq!(meldt.len(), 2, "{meldt:?}");
        assert_eq!(
            meldt[&("z1".into(), "2026-09-24".into())],
            "roed",
            "gul → rød erstatter, dubleres ikke"
        );
        assert_eq!(meldt[&("z2".into(), "2026-11-02".into())], "roed");
    }

    #[test]
    fn glem_tokenet_sletter_notifikationsvalg() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_notifikation_zone("z1", Some("fra")).unwrap();
        s.gem_meldt(
            &[("z1".into(), "2026-11-02".into(), "roed".into())],
            "2026-11-03T06:00:00.000Z",
            "2026-09-24",
        )
        .unwrap();
        s.gem_indstilling("notifikation_niveau", "alt").unwrap();
        s.gem_doegn("z1", "2026-11-02", &[rk("fw", "k", 1)], "2026-11-03T06:00:00.000Z")
            .unwrap();
        s.slet_zoner().unwrap();
        assert!(s.notifikation_zoner().unwrap().is_empty(), "pr.-domæne-valget er glemt");
        assert!(s.meldt().unwrap().is_empty(), "det meldte er glemt");
        assert_eq!(
            s.indstilling("notifikation_niveau").unwrap().as_deref(),
            Some("alt"),
            "forudindstillingen bliver"
        );
        assert_eq!(s.raekker("z1", "2026-11-01").unwrap().len(), 1, "Værnets tal bliver");
    }

    #[test]
    fn indstillinger_overlever_glem_tokenet() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_indstilling("autostart_standard", "1").unwrap();
        s.slet_zoner().unwrap();
        assert_eq!(
            s.indstilling("autostart_standard").unwrap().as_deref(),
            Some("1"),
            "Glem tokenet slår ikke autostart til igen ved næste start"
        );
    }

    #[test]
    fn valgte_gemmes_og_glemmes_med_tokenet() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(
            &[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com"), zone("z3", "c.wi24rd.com")],
            "2026-11-03T08:00:00.000Z",
        )
        .unwrap();
        assert!(s.valg().unwrap().is_empty(), "T1: intet er valgt fra start");
        assert!(s.nye().unwrap().is_empty(), "første forbindelse: intet er nyt");
        s.gem_valgte(&["z1".into(), "z3".into(), "z3".into()]).unwrap();
        assert_eq!(s.valg().unwrap(), ["z1", "z3"], "dubletter tæller én gang");
        assert_eq!(s.zoner_til().unwrap(), [zone("z1", "a.wi24rd.com"), zone("z3", "c.wi24rd.com")]);
        s.gem_valgte(&[]).unwrap();
        assert!(s.valg().unwrap().is_empty(), "alle fra er også et valg");
        s.gem_valgte(&["z1".into()]).unwrap();
        s.afvis_nye(&["z2".into()]).unwrap();
        s.gem_doegn("z1", "2026-11-02", &[rk("alle", "", 5)], "2026-11-03T08:00:00.000Z")
            .unwrap();
        s.slet_zoner().unwrap();
        assert_eq!(s.zoner().unwrap(), (vec![], None));
        assert!(s.valgte().unwrap().is_empty(), "Glem tokenet sletter de valgte");
        let n: i64 = s.db.query_row("SELECT count(*) FROM afvist_ny", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "og de afviste");
        assert_eq!(s.raekker("z1", "2026-11-01").unwrap().len(), 1, "Værnets tal bliver");
    }

    #[test]
    fn raekker_for_flere_zoner_med_zone_og_i_gemt_raekkefoelge() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_doegn("z1", "2026-11-01", &[rk("fw", "a", 1)], "t").unwrap();
        s.gem_doegn("z2", "2026-11-01", &[rk("fw", "a", 2)], "t").unwrap();
        s.gem_doegn("z3", "2026-11-01", &[rk("fw", "a", 4)], "t").unwrap();
        s.gem_doegn("z1", "2026-11-02", &[rk("alle", "", 8)], "t").unwrap();
        s.gem_doegn("z2", "2026-10-01", &[rk("fw", "gammel", 16)], "t").unwrap();
        let r = s.raekker_for(&["z2".into(), "z1".into()], "2026-10-15").unwrap();
        let kort: Vec<(&str, &str, i64)> = r.iter().map(|(z, x)| (z.as_str(), x.dag.as_str(), x.n)).collect();
        assert_eq!(kort, [("z1", "2026-11-01", 1), ("z2", "2026-11-01", 2), ("z1", "2026-11-02", 8)]);
        assert!(s.raekker_for(&[], "2026-10-01").unwrap().is_empty());
        let en = s.raekker_for(&["z1".into()], "2026-10-01").unwrap();
        assert_eq!(
            en.into_iter().map(|(_, x)| x).collect::<Vec<_>>(),
            s.raekker("z1", "2026-10-01").unwrap(),
            "samme som raekker for én zone"
        );
    }

    #[test]
    fn zonelisten_erstattes_helt_og_glemmes_uden_vaernets_tal() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_doegn("z1", "2026-10-29", &[rk("alle", "", 5)], "2026-10-29T10:00:00.000Z")
            .unwrap();
        s.gem_zoner(
            &[zone("z2", "b.wi24rd.com"), zone("z1", "a.wi24rd.com")],
            "2026-11-03T07:00:00.000Z",
        )
        .unwrap();
        assert_eq!(
            s.zoner().unwrap(),
            (
                vec![zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")],
                Some("2026-11-03T07:00:00.000Z".into())
            ),
            "sorteret efter navn"
        );
        s.gem_zoner(&[zone("z3", "c.wi24rd.com")], "2026-11-03T08:00:00.000Z").unwrap();
        assert_eq!(
            s.zoner().unwrap(),
            (vec![zone("z3", "c.wi24rd.com")], Some("2026-11-03T08:00:00.000Z".into())),
            "hele listen erstattes"
        );
        s.slet_zoner().unwrap();
        assert_eq!(s.zoner().unwrap(), (vec![], None));
        assert_eq!(s.raekker("z1", "2026-10-01").unwrap().len(), 1, "Værnets tal bliver");
        assert_eq!(s.hentet("z1").unwrap().len(), 1);
    }

    #[test]
    fn et_doegn_erstattes_helt_og_zoner_holdes_adskilt() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_doegn(
            "z1",
            "2026-10-29",
            &[rk("alle", "", 5), rk("fw", "a", 1)],
            "2026-10-29T10:00:00.000Z",
        )
        .unwrap();
        s.gem_doegn("z2", "2026-10-29", &[rk("alle", "", 9)], "2026-10-29T10:00:00.000Z")
            .unwrap();
        s.gem_doegn("z1", "2026-10-29", &[rk("alle", "", 7)], "2026-10-30T02:00:00.000Z")
            .unwrap();
        let r = s.raekker("z1", "2026-10-01").unwrap();
        assert_eq!(
            r,
            vec![Raekke {
                dag: "2026-10-29".into(),
                art: "alle".into(),
                noegle: "".into(),
                n: 7
            }]
        );
        assert_eq!(s.hentet("z1").unwrap()["2026-10-29"], "2026-10-30T02:00:00.000Z");
        assert_eq!(s.raekker("z2", "2026-10-01").unwrap()[0].n, 9);
        assert!(s.raekker("z1", "2026-10-30").unwrap().is_empty(), "fra_dag filtrerer");
    }

    /// Indstillinger → Data: ældste og nyeste dag og antal domæner; "Glem" rører ikke historikken.
    #[test]
    fn data_status_tom_og_med_tal() {
        let mut s = Store::i_hukommelse().unwrap();
        assert_eq!(s.data_status().unwrap(), (None, None, 0));
        for (zone, dag) in [("z1", "2026-11-02"), ("z1", "2026-10-04"), ("z2", "2026-10-20")] {
            s.gem_doegn(zone, dag, &[rk("alle", "", 1)], "2026-11-03T02:00:00.000Z").unwrap();
        }
        let ventet = (Some("2026-10-04".to_string()), Some("2026-11-02".to_string()), 2);
        assert_eq!(s.data_status().unwrap(), ventet);
        s.slet_zoner().unwrap();
        assert_eq!(s.data_status().unwrap(), ventet);
    }

    /// Skema 8 og 9: et gemt valg fra før tilvalg bliver ved med at vise det samme (T4).
    #[test]
    fn migrering_fra_version_7_beholder_det_viste() {
        let v7 = |valg: &str| {
            let db = Connection::open_in_memory().unwrap();
            for skema in [SKEMA_1, SKEMA_2, SKEMA_3, SKEMA_4, SKEMA_5, SKEMA_6, SKEMA_7] {
                db.execute_batch(skema).unwrap();
            }
            db.pragma_update(None, "user_version", 7).unwrap();
            db.execute_batch(
                "INSERT INTO zoner (id, navn, hentet_utc) VALUES ('z1', 'a.wi24rd.com', 't'), ('z2', 'b.wi24rd.com', 't'), ('z3', 'c.wi24rd.com', 't');",
            )
            .unwrap();
            db.execute_batch("INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z3', '2026-11-02', 'fw', 'k', 7);")
                .unwrap();
            db.execute_batch(valg).unwrap();
            Store::klar(db).unwrap()
        };
        let alle = ["z1", "z2", "z3"];
        assert_eq!(
            v7("INSERT INTO valg (zone) VALUES ('*'), ('z1'), ('z2'), ('z3');").valg().unwrap(),
            alle,
            "alle valgt"
        );
        assert_eq!(v7("").valg().unwrap(), alle, "aldrig gemt: alle");
        assert_eq!(
            v7("INSERT INTO valg (zone) VALUES ('*');").valg().unwrap(),
            alle,
            "tomt valg skjulte intet"
        );
        let et = v7("INSERT INTO valg (zone) VALUES ('*'), ('z1'), ('z2'), ('gammel');");
        assert_eq!(et.valg().unwrap(), ["z1", "z2"]);
        assert_eq!(et.raekker("z3", "2026-11-01").unwrap().len(), 1, "tallene bliver");
        assert_eq!(et.skema_version().unwrap(), SKEMA_VERSION);
    }

    /// T4: migreringen 8 → 9 vælger alle kendte zoner minus de fravalgte, og intet står som nyt.
    #[test]
    fn migrering_fra_version_8_vaelger_det_viste() {
        let db = Connection::open_in_memory().unwrap();
        for skema in [SKEMA_1, SKEMA_2, SKEMA_3, SKEMA_4, SKEMA_5, SKEMA_6, SKEMA_7, SKEMA_8] {
            db.execute_batch(skema).unwrap();
        }
        db.pragma_update(None, "user_version", 8).unwrap();
        db.execute_batch(
            "INSERT INTO zoner (id, navn, hentet_utc) VALUES ('z1', 'a.wi24rd.com', 't'), ('z2', 'b.wi24rd.com', 't'), ('z3', 'c.wi24rd.com', 't');
             INSERT INTO fravalgt (zone) VALUES ('z2');
             INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z2', '2026-11-02', 'fw', 'k', 7);",
        )
        .unwrap();
        let s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert_eq!(s.valg().unwrap(), ["z1", "z3"]);
        assert!(s.nye().unwrap().is_empty());
        assert_eq!(s.raekker("z2", "2026-11-01").unwrap().len(), 1, "tallene bliver");
        let n: i64 =
            s.db.query_row("SELECT count(*) FROM sqlite_master WHERE name = 'fravalgt'", [], |r| r.get(0))
                .unwrap();
        assert_eq!(n, 0, "den gamle tabel er væk");
    }

    /// T2: præcis ét domæne ved første forbindelse vælges. Flere gør ikke.
    #[test]
    fn eet_domaene_vaelges_ved_foerste_forbindelse() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(&[zone("z1", "a.wi24rd.com")], "t").unwrap();
        assert_eq!(s.valg().unwrap(), ["z1"]);
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(&[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t").unwrap();
        assert!(s.valg().unwrap().is_empty(), "to domæner: intet vælges");
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(&[], "t").unwrap();
        assert!(s.valg().unwrap().is_empty(), "ingen zoner");
    }

    /// T3: en zone, der dukker op senere, er ny og vælges ikke, heller ikke hvis den er den eneste nye. Den bliver ny, til der er svaret.
    #[test]
    fn nye_zoner_venter_paa_svar() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(&[zone("z1", "a.wi24rd.com")], "t").unwrap();
        s.gem_zoner(&[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t").unwrap();
        assert_eq!(s.valg().unwrap(), ["z1"], "T2 slår ikke til for en ny zone senere");
        assert_eq!(s.nye().unwrap(), HashSet::from(["z2".to_string()]));
        s.gem_zoner(&[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t").unwrap();
        assert_eq!(s.nye().unwrap().len(), 1, "den er stadig ny i næste zoneliste");
        s.gem_zoner(
            &[
                zone("z1", "a.wi24rd.com"),
                zone("z2", "b.wi24rd.com"),
                zone("z3", "c.wi24rd.com"),
                zone("z4", "d.wi24rd.com"),
            ],
            "t",
        )
        .unwrap();
        assert_eq!(s.nye().unwrap().len(), 3);
        s.afvis_nye(&["z2".into()]).unwrap();
        s.gem_valgte(&["z1".into(), "z3".into()]).unwrap();
        assert_eq!(
            s.nye().unwrap(),
            HashSet::from(["z4".to_string()]),
            "afvist og valgt er væk fra linjen"
        );
        s.gem_zoner(
            &[zone("z1", "a.wi24rd.com"), zone("z3", "c.wi24rd.com"), zone("z4", "d.wi24rd.com")],
            "t",
        )
        .unwrap();
        s.gem_zoner(
            &[
                zone("z1", "a.wi24rd.com"),
                zone("z2", "b.wi24rd.com"),
                zone("z3", "c.wi24rd.com"),
                zone("z4", "d.wi24rd.com"),
            ],
            "t",
        )
        .unwrap();
        assert_eq!(
            s.nye().unwrap(),
            HashSet::from(["z4".to_string()]),
            "\"Ikke nu\" huskes, også hvis domænet var væk et stykke tid"
        );
        s.gem_zoner(&[zone("z1", "a.wi24rd.com"), zone("z3", "c.wi24rd.com")], "t").unwrap();
        assert!(s.nye().unwrap().is_empty(), "et domæne, nøglen ikke ser, venter ikke på svar");
    }

    /// Et domæne, brugeren fravælger, er ikke nyt: det var kendt.
    #[test]
    fn fravalgt_er_ikke_nyt() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(&[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t").unwrap();
        s.gem_valgte(&["z1".into(), "z2".into()]).unwrap();
        s.gem_valgte(&["z1".into()]).unwrap();
        s.gem_zoner(&[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t").unwrap();
        assert!(s.nye().unwrap().is_empty());
    }

    /// En nøgle til en anden konto (ingen fælles zoner) starter forfra og anvender T2. Fælles zoner er samme konto. Historikken bliver.
    #[test]
    fn anden_konto_starter_forfra() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(
            &[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com"), zone("z3", "c.wi24rd.com")],
            "t",
        )
        .unwrap();
        s.gem_valgte(&["z1".into(), "z2".into()]).unwrap();
        s.afvis_nye(&["z3".into()]).unwrap();
        s.gem_doegn("z2", "2026-11-02", &[rk("alle", "", 5)], "t").unwrap();
        s.gem_zoner(&[zone("z2", "b.wi24rd.com"), zone("z9", "x.wi24rd.com")], "t").unwrap();
        assert_eq!(s.valg().unwrap(), ["z2"], "samme konto: valget bliver");
        assert_eq!(s.nye().unwrap(), HashSet::from(["z9".to_string()]));
        s.gem_zoner(&[zone("n1", "ny.wi24rd.com"), zone("n2", "ny2.wi24rd.com")], "t")
            .unwrap();
        assert!(s.valgte().unwrap().is_empty(), "en anden konto: intet er valgt");
        assert!(s.nye().unwrap().is_empty(), "og intet er nyt");
        let n: i64 = s.db.query_row("SELECT count(*) FROM afvist_ny", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "de afviste er ryddet");
        s.gem_zoner(&[zone("m1", "en.wi24rd.com")], "t").unwrap();
        assert_eq!(s.valg().unwrap(), ["m1"], "T2 gælder også en anden konto");
        assert_eq!(s.raekker("z2", "2026-11-01").unwrap().len(), 1, "historikken bliver");
        s.gem_zoner(&[], "t").unwrap();
        assert_eq!(s.valgte().unwrap().len(), 1, "en tom liste er ikke en anden konto");
    }

    /// Skema 7: en database fra version 6 får `radar_dag`, og alle tal bliver stående.
    #[test]
    fn migrering_fra_version_6_til_7_tilfoejer_radar_dag() {
        let db = Connection::open_in_memory().unwrap();
        for skema in [SKEMA_1, SKEMA_2, SKEMA_3, SKEMA_4, SKEMA_5, SKEMA_6] {
            db.execute_batch(skema).unwrap();
        }
        db.pragma_update(None, "user_version", 6).unwrap();
        db.execute_batch(
            "INSERT INTO vagt_dag (zone, dag, art, noegle, n) VALUES ('z1', '2026-11-02', 'fw', 'k', 1260), ('z1', '2026-11-02', 'land', 'DK', 7);
             INSERT INTO indstillinger (noegle, vaerdi) VALUES ('sprog', 'da');",
        )
        .unwrap();
        let s = Store::klar(db).unwrap();
        assert_eq!(s.skema_version().unwrap(), SKEMA_VERSION);
        assert_eq!(s.raekker("z1", "2026-10-01").unwrap().len(), 2);
        assert_eq!(s.indstilling("sprog").unwrap().as_deref(), Some("da"));
        assert!(s.radar_raekker("", "2000-01-01").unwrap().is_empty());
    }

    /// Et nyt svar erstatter de døgn, det dækker, for serien og landet, og intet andet. Nulstil rydder tal og tidspunkter, ikke landet.
    #[test]
    fn radar_erstatter_kun_sine_doegn_og_nulstilles() {
        let mut s = Store::i_hukommelse().unwrap();
        let t = |d: &str, k: &str, v| (d.to_string(), k.to_string(), v);
        let dage = ["2026-11-01".to_string(), "2026-11-02".to_string()];
        s.gem_radar(
            "l7",
            "",
            &dage,
            &[t("2026-11-01", "", 0.5), t("2026-11-02", "", 1.0)],
            "MIN0_MAX",
            "h1",
        )
        .unwrap();
        s.gem_radar("maal", "", &dage[..1], &[t("2026-11-01", "DK", 0.4)], "PERCENTAGE", "h1")
            .unwrap();
        s.gem_radar("par", "DK", &dage[..1], &[t("2026-11-01", "DK\tUS", 9.0)], "PERCENTAGE", "h1")
            .unwrap();
        s.gem_radar("l7", "", &dage[1..], &[t("2026-11-02", "", 0.7)], "MIN0_MAX", "h2")
            .unwrap();
        let rk = s.radar_raekker("", "2026-11-01").unwrap();
        assert_eq!(rk.len(), 3);
        assert_eq!(rk.iter().find(|r| r.dag == "2026-11-02").unwrap().vaerdi, 0.7);
        assert_eq!(
            rk.iter().find(|r| r.serie == "l7" && r.dag == "2026-11-01").unwrap().hentet_utc,
            "h1"
        );
        assert_eq!(s.radar_raekker("DK", "2026-11-01").unwrap().len(), 1, "landet står for sig");
        assert_eq!(s.radar_hentet("l7", "").unwrap()["2026-11-02"], "h2");
        assert!(s.radar_raekker("", "2026-11-03").unwrap().is_empty(), "fra og med `fra_dag`");
        s.gem_indstilling("land", "DK").unwrap();
        s.gem_indstilling("radar_sidst", "h2").unwrap();
        s.gem_indstilling("radar_sidst:DK", "h2").unwrap();
        s.gem_indstilling("radar_fejl", "{}").unwrap();
        s.nulstil_historik().unwrap();
        assert!(s.radar_raekker("", "2000-01-01").unwrap().is_empty());
        assert!(s.radar_raekker("DK", "2000-01-01").unwrap().is_empty());
        assert_eq!(s.indstilling("land").unwrap().as_deref(), Some("DK"), "landet er et valg");
        for k in ["radar_sidst", "radar_sidst:DK", "radar_fejl"] {
            assert_eq!(s.indstilling(k).unwrap(), None, "{k}");
        }
    }

    /// "Nulstil historik": tallene er væk; indstillinger, zoner, valg, notifikationsvalg og det meldte bliver (J6).
    #[test]
    fn nulstil_historik_sletter_tal_og_beholder_valg() {
        let mut s = Store::i_hukommelse().unwrap();
        let mange: Vec<_> = (0..2000).map(|i| rk("fw", &format!("noegle-{i}"), 3)).collect();
        s.gem_doegn("z1", "2026-11-02", &mange, "2026-11-03T02:00:00.000Z").unwrap();
        s.gem_doegn("z2", "2026-10-04", &[rk("alle", "", 9)], "2026-11-03T02:00:00.000Z")
            .unwrap();
        s.gem_meldt(
            &[("z1".into(), "2026-11-02".into(), "roed".into())],
            "2026-11-03T08:00:00.000Z",
            "2026-10-01",
        )
        .unwrap();
        s.gem_zoner(
            &[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")],
            "2026-11-03T02:00:00.000Z",
        )
        .unwrap();
        s.gem_valgte(&["z1".to_string()]).unwrap();
        s.gem_indstilling("niveau", "alt").unwrap();
        s.gem_notifikation_zone("z2", Some("fra")).unwrap();

        s.nulstil_historik().unwrap();

        assert_eq!(s.data_status().unwrap(), (None, None, 0));
        for z in ["z1", "z2"] {
            assert!(s.raekker(z, "2000-01-01").unwrap().is_empty(), "vagt_dag {z}");
            assert!(s.hentet(z).unwrap().is_empty(), "hentet {z}");
        }
        assert_eq!(
            s.meldt().unwrap().len(),
            1,
            "J6: samme besked må ikke komme igen efter en nulstilling"
        );
        let frie: i64 = s.db.query_row("PRAGMA freelist_count", [], |r| r.get(0)).unwrap();
        assert_eq!(frie, 0, "VACUUM har frigivet pladsen");
        assert_eq!(s.zoner().unwrap().0.len(), 2);
        assert_eq!(s.valg().unwrap(), ["z1"]);
        assert_eq!(s.indstilling("niveau").unwrap().as_deref(), Some("alt"));
        assert_eq!(s.notifikation_zoner().unwrap()["z2"], "fra");
        s.gem_doegn("z1", "2026-11-02", &[rk("fw", "x", 1)], "2026-11-04T02:00:00.000Z")
            .unwrap();
        assert_eq!(s.raekker("z1", "2000-01-01").unwrap()[0].n, 1, "databasen virker efter VACUUM");
    }
}
