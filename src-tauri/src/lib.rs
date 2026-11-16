//! Wi24rd Huginn and Muninn — for Cloudflare®. Rust-delen: tokenet i nøgleringen, kald til Cloudflare (kun læsning),
//! lokal cache i SQLite og Værnet. Webvisningen får aldrig tokenet; den kalder kun kommandoerne herunder. Programmet
//! bliver i menulinjen, når vinduet lukkes (`agent`), og kan starte ved login uden vindue (autostart). Usædvanligt mange
//! angreb meldes med en lokal notifikation, og visningen "Siden sidst" åbner (`notify`). På macOS har programmet sin egen
//! programmenu på dansk/engelsk (`programmenu`); vinduet har ingen titellinje, men et blåt bånd med trafiklysene. Cloudflare Radar
//! hentes uden for køen (`radar`).
mod adgang;
mod agent;
mod besoeg;
mod cf;
mod dagligt;
mod dmarc;
mod dns;
mod dyk;
mod historik;
mod huller;
mod koe;
mod login;
mod notify;
mod overblik;
mod programmenu;
mod radar;
mod robotter;
mod sikkerhed;
mod sti200;
mod store;
mod teknologi;
mod token;
mod vaern;
mod vaert;
mod ved_login;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, Url};
#[cfg(not(target_os = "macos"))]
use tauri_plugin_autostart::ManagerExt;

use cf::{Fejl, Klient, Zone};
use koe::{Henter, Koe, KoeStatus};
use store::Store;
use token::Hemmelig;
use vaern::Doegnudfald;

struct Tilstand {
    lager: Mutex<Store>,
    klient: Klient,
    laase: Hentelaase,
    koe: Koe,
    /// Døgn, Cloudflare ikke længere har data for (zone, dag), med fejlen: de spørges der ikke om igen, mens programmet kører.
    udenfor: Mutex<HashMap<(String, String), Fejl>>,
    /// Tokenet i hukommelsen, så nøgleringen ikke spørger ved hver runde (aldrig i log, fejltekst eller IPC-svar).
    token: token::Huske,
    /// Menuen eller en notifikation har bedt om "Siden sidst": forsiden viser kortet ved næste `siden_sidst`.
    siden_sidst: AtomicBool,
    /// Domæner, hvor Cloudflare afviste et felt i `ADGANG_QUERY`: de hentes uden `clientAsn`, mens programmet kører.
    uden_asn: Mutex<HashSet<String>>,
    /// Domæner, hvor Cloudflare afviste hovedforespørgslen med værtsfeltet (V-SUBDOM.1 B2, trin 0), med tidspunktet (ms): de hentes uden
    /// `clientRequestHTTPHost` på `fw` til næste UTC-døgn, højst 24 timer (`vaert::uden_gaelder`, FIX 1). Domænets egne tal er som før; døgnene får intet `v_fw`,
    /// og `subdomaener` melder `angreb_afvist`, når perioden ingen tal pr. vært har for angrebene.
    uden_vaert: Mutex<HashMap<String, i64>>,
    /// Cloudflare har afvist filteret i `LAEK_QUERY`: resten af kørslen hentes uden det.
    laek_uden_filter: AtomicBool,
    /// Startet af den daglige start og lukker selv, når køen er færdig. Et vindue slår det fra (`aabn_vindue`): så bliver programmet.
    dagligt: AtomicBool,
    /// Forbindelser, hvis zoneliste er hentet, mens programmet kører (`zones`), også når den var tom (V-KONTI.1 FIX 1, F2). Se `ForbindelseUd::hentet`.
    zoner_hentet: Mutex<HashSet<i64>>,
}

/// Fejl til webvisningen er en fejlkode (`{kode, status?, dag?, detalje?}`, se `cf::Fejl`); teksten vælges i UI'et.
type Svar<T> = Result<T, Fejl>;

/// Nøglen til zonelistens lås. Zone-id'er er kun ASCII-bogstaver og -tal (`vaern::tjek_zone`), så den kan ikke kollidere.
const ZONELISTEN: &str = "*zoner";
/// Nøglen til nøgleringens lås: én læsning ad gangen.
const NOEGLERINGEN: &str = "*noeglering";
/// Nøglen til Radars lås: én opdatering ad gangen.
const RADAR: &str = "*radar";
/// Databasefilen i app-datamappen.
const DATABASE: &str = "huginn-muninn.sqlite";

/// Én lås pr. zone (og én for zonelisten), så der aldrig kører to hentninger af det samme på én gang: en ny hentning
/// venter, til den igangværende er færdig, og starter først derefter. Forskellige zoner blokerer ikke hinanden.
#[derive(Default)]
struct Hentelaase(Mutex<HashMap<String, Arc<futures_util::lock::Mutex<()>>>>);

impl Hentelaase {
    async fn koer<T>(&self, noegle: &str, f: impl Future<Output = T>) -> T {
        let laas = {
            let mut m = self.0.lock().unwrap_or_else(|e| e.into_inner());
            m.entry(noegle.to_string()).or_default().clone()
        };
        let _vagt = laas.lock().await;
        f.await
    }
}

fn nu_ms() -> i64 {
    vaern::ur_ms()
}

/// Nøgleringen kan blokere (og på macOS spørge brugeren), så den kaldes uden for async-tråden.
async fn blokerende<T: Send + 'static>(f: impl FnOnce() -> Result<T, Fejl> + Send + 'static) -> Result<T, Fejl> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|_| Fejl::Noegle)?
}

/// Forbindelsernes id'er, laveste først. Uden forbindelser er det `[1]`: nøgleringens oprindelige post kan have et token fra før skema 19 uden en
/// zoneliste (hullet), og den almindelige læsning finder det (`gemt_token` opretter så forbindelse 1).
fn forbindelse_ider(t: &Tilstand) -> Result<Vec<i64>, Fejl> {
    Ok(ider_eller_1(laas(t)?.forbindelse_ider()?))
}

/// Se `forbindelse_ider`. Ren funktion.
fn ider_eller_1(ider: Vec<i64>) -> Vec<i64> {
    if ider.is_empty() {
        vec![1]
    } else {
        ider
    }
}

/// Forbindelsens token fra hukommelsen, ellers fra nøgleringen (og så huskes det). Nøgleringen læses af én ad gangen, også på tværs af forbindelserne:
/// spørger køen og siden samtidig (ved start), venter den ene og får tokenet fra hukommelsen, så der kun kommer én dialog pr. post. Er tokenet glemt eller
/// erstattet, mens nøgleringen blev læst, gælder det nye: det læste token bruges ikke. Finder læsningen af forbindelse 1 et token, og er der ingen
/// forbindelser, oprettes forbindelse 1 (hullet fra skema 19); der læses ikke ekstra.
async fn gemt_token(t: &Tilstand, forbindelse: i64) -> Result<Hemmelig, Fejl> {
    if let Ok(h) = t.token.hent(forbindelse) {
        return Ok(h);
    }
    t.laase
        .koer(NOEGLERINGEN, async {
            let udgave = match t.token.hent(forbindelse) {
                Ok(h) => return Ok(h),
                Err(u) => u,
            };
            let h = blokerende(move || token::hent(forbindelse)).await?.ok_or(Fejl::IntetToken)?;
            if forbindelse == 1 {
                laas(t)?.sikr_forbindelse_1(&vaern::iso(nu_ms()))?;
            }
            t.token.fyld(forbindelse, udgave, &h);
            t.token.hent(forbindelse).map_err(|_| Fejl::IntetToken)
        })
        .await
}

/// Det første token i `ider`, der kan læses; ellers den første fejl, der ikke er `IntetToken` (nøgleringen), og så `IntetToken`.
async fn foerste_token(t: &Tilstand, ider: Vec<i64>) -> Result<Hemmelig, Fejl> {
    let mut fejl = None;
    for id in ider {
        match gemt_token(t, id).await {
            Ok(h) => return Ok(h),
            Err(Fejl::IntetToken) => {}
            Err(e) => {
                fejl.get_or_insert(e);
            }
        }
    }
    Err(fejl.unwrap_or(Fejl::IntetToken))
}

/// Et vilkårligt gyldigt token (Radar hører ikke til en konto): den første forbindelse, der ikke er afvist.
async fn et_token(t: &Tilstand) -> Result<Hemmelig, Fejl> {
    let afviste = t.koe.afviste();
    foerste_token(t, forbindelse_ider(t)?.into_iter().filter(|f| !afviste.contains(f)).collect()).await
}

/// Tokenet til zonen (D3): den forbindelse med lavest id blandt dem, der ser zonen, hvis token ikke er afvist og kan læses.
async fn token_til(t: &Tilstand, zone: &str) -> Result<Hemmelig, Fejl> {
    let afviste = t.koe.afviste();
    let ider = laas(t)?.forbindelser_for(zone)?;
    foerste_token(t, ider.into_iter().filter(|f| !afviste.contains(f)).collect()).await
}

/// Alle forbindelsers tokens ved rundens start. En forbindelse, hvis token ikke kan læses, springes over; kan intet læses, er det den første fejl,
/// der ikke er `IntetToken` (køen venter på "Hent nu"), ellers `IntetToken` (køen stopper), som i 1.0.0.
async fn alle_tokens(t: &Tilstand) -> Result<BTreeMap<i64, Hemmelig>, Fejl> {
    let (mut ud, mut fejl) = (BTreeMap::new(), None);
    for id in forbindelse_ider(t)? {
        match gemt_token(t, id).await {
            Ok(h) => {
                ud.insert(id, h);
            }
            Err(Fejl::IntetToken) => {}
            Err(e) => {
                fejl.get_or_insert(e);
            }
        }
    }
    if ud.is_empty() {
        return Err(fejl.unwrap_or(Fejl::IntetToken));
    }
    Ok(ud)
}

#[derive(Serialize)]
struct TokenStatus {
    sat: bool,
}

/// Gemmer et godkendt token i forbindelsens post. Den nye forbindelse (`ny`) fjernes igen, hvis nøgleringen fejler. Rydder en afvisning og starter køen.
async fn gem_token(tilstand: &Tilstand, app: &AppHandle, id: i64, ny: bool, token: Hemmelig) -> Result<(), Fejl> {
    let gem = token.clone();
    if let Err(e) = blokerende(move || token::gem(id, &gem)).await {
        if ny {
            let _ = laas(tilstand).and_then(|mut l| l.fortryd_forbindelse(id));
        }
        return Err(e);
    }
    tilstand.token.saet(id, token);
    tilstand.koe.ryd_afvist(id);
    start_koe(app);
    Ok(())
}

/// Er der mindst én forbindelse med et token? En nøgleringsfejl meldes kun, hvis ingen forbindelse kunne læses.
#[tauri::command]
async fn token_status(tilstand: State<'_, Tilstand>) -> Svar<TokenStatus> {
    match foerste_token(&tilstand, forbindelse_ider(&tilstand)?).await {
        Ok(_) => Ok(TokenStatus { sat: true }),
        Err(Fejl::IntetToken) => Ok(TokenStatus { sat: false }),
        Err(e) => Err(e),
    }
}

/// Glem den sidste forbindelse (`forbindelse_glem`), som "Glem tokenet" i 1.0.0: stopper køen (menulinjen: "intet token") og glemmer forbindelserne én ad
/// gangen (`glem_en`), så nøgleringen og databasen passer sammen, også hvis en sletning fejler midt i. Værnets tal og indstillingerne bliver.
async fn glem_alle(tilstand: &Tilstand, app: &AppHandle) -> Result<(), Fejl> {
    tilstand.koe.stop_med(Fejl::IntetToken, &|s| melder(app, s));
    for id in forbindelse_ider(tilstand)? {
        glem_en(tilstand, id).await?;
    }
    laas(tilstand)?.slet_alle()
}

/// Glemmer én forbindelse: først nøgleringen, så hukommelsen, så databasen (`Store::slet_zoner`). En læsning af nøgleringen, der startede før `glem`, har den
/// gamle udgave, og `fyld` afviser den; en læsning efter `slet` finder intet. Fejler `slet`, huskes tokenet stadig, og fejlen returneres (det er ikke glemt).
async fn glem_en(tilstand: &Tilstand, id: i64) -> Result<(), Fejl> {
    blokerende(move || token::slet(id)).await?;
    tilstand.token.glem(id);
    tilstand.koe.ryd_afvist(id);
    if let Ok(mut h) = tilstand.zoner_hentet.lock() {
        h.remove(&id);
    }
    laas(tilstand)?.slet_zoner(id)
}

/// En forbindelse til webvisningen: id, Cloudflares navn, brugerens navn, antal konti og domæner, om tokenet er afvist, og om zonelisten er hentet. Aldrig
/// et token, et konto-id eller et token-id. Det viste navn dannes i UI'et (D5).
#[derive(Serialize)]
struct ForbindelseUd {
    #[serde(flatten)]
    f: store::Forbindelse,
    afvist: bool,
    /// Zonelisten er hentet: forbindelsen ser domæner (gemt), eller `zones` har hentet dens liste, mens programmet kører, også en tom (F2). `false`: en ny
    /// forbindelse, hvis liste ikke er hentet endnu (UI'et skriver "Domænerne står her, når zonelisten er hentet.").
    hentet: bool,
}

/// Se `ForbindelseUd`. Ren funktion.
fn forbindelser_ud(fs: Vec<store::Forbindelse>, afviste: &HashSet<i64>, hentede: &HashSet<i64>) -> Vec<ForbindelseUd> {
    fs.into_iter()
        .map(|f| ForbindelseUd {
            afvist: afviste.contains(&f.id),
            hentet: f.domaener > 0 || hentede.contains(&f.id),
            f,
        })
        .collect()
}

#[tauri::command]
async fn forbindelser_hent(tilstand: State<'_, Tilstand>) -> Svar<Vec<ForbindelseUd>> {
    let afviste = tilstand.koe.afviste();
    let hentede = tilstand.zoner_hentet.lock().map(|h| h.clone()).unwrap_or_default();
    Ok(forbindelser_ud(laas(&tilstand)?.forbindelser()?, &afviste, &hentede))
}

/// V1 (V-KONTI.1 FIX 1): den forbindelse, der ser præcis de samme domæner som `nye` (samme, ikke-tomme mængde zone-id'er); laveste id først. En tom liste
/// og en del- eller overmængde er ikke den samme (IT-firmaet: ét API Token pr. kunde på samme konto).
fn samme_domaener(lager: &Store, nye: &[Zone]) -> Result<Option<i64>, Fejl> {
    if nye.is_empty() {
        return Ok(None);
    }
    let nye: HashSet<&str> = nye.iter().map(|z| z.id.as_str()).collect();
    for id in lager.forbindelse_ider()? {
        let egne = lager.zoner_for(id)?;
        if egne.len() == nye.len() && egne.iter().all(|z| nye.contains(z.as_str())) {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// En ny forbindelse med et godkendt token; dens id. Verificerer tokenet hos Cloudflare og gemmer det kun, hvis det er gyldigt (`Klient::godkend`).
/// Returnerer aldrig tokenet. Den første forbindelse er id 1 (nøgleringens oprindelige post). Zonelisten hentes af `zones` bagefter.
/// Findes der forbindelser, hentes det nye tokens zoneliste først: ser det præcis de samme domæner som en af dem, oprettes intet, og intet gemmes i
/// nøgleringen (`Fejl::AlleredeForbundet` med den forbindelses id, V1). En fejl i den hentning stopper også tilføjelsen.
#[tauri::command]
async fn forbindelse_tilfoej(token: Hemmelig, tilstand: State<'_, Tilstand>, app: AppHandle) -> Svar<i64> {
    token::tjek_form(&token)?;
    tilstand.klient.godkend(&token, nu_ms()).await?;
    let findes = !laas(&tilstand)?.forbindelse_ider()?.is_empty();
    if findes {
        let l = tilstand.klient.zoner(&token).await?;
        let samme = samme_domaener(&*laas(&tilstand)?, &l.zoner)?;
        if let Some(id) = samme {
            return Err(Fejl::AlleredeForbundet(id));
        }
    }
    let id = laas(&tilstand)?.ny_forbindelse(&vaern::iso(nu_ms()))?;
    gem_token(&tilstand, &app, id, true, token).await?;
    Ok(id)
}

fn findes(tilstand: &Tilstand, id: i64) -> Result<(), Fejl> {
    if laas(tilstand)?.forbindelse_ider()?.contains(&id) {
        Ok(())
    } else {
        Err(Fejl::Intern)
    }
}

/// Et nyt token på en forbindelse ("Skift API Token…"). Rydder afvisningen og vækker køen.
#[tauri::command]
async fn forbindelse_skift(id: i64, token: Hemmelig, tilstand: State<'_, Tilstand>, app: AppHandle) -> Svar<()> {
    findes(&tilstand, id)?;
    token::tjek_form(&token)?;
    tilstand.klient.godkend(&token, nu_ms()).await?;
    gem_token(&tilstand, &app, id, false, token).await
}

/// Brugerens navn på forbindelsen (renset som Cloudflares, `cf::rens_kontonavn`); tomt eller `null` viser Cloudflares igen (D5).
#[tauri::command]
async fn forbindelse_omdoeb(id: i64, navn: Option<String>, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let navn = navn.as_deref().and_then(cf::rens_kontonavn);
    laas(&tilstand)?.omdoeb(id, navn.as_deref())
}

/// "Glem" én forbindelse (D4). Er det den sidste, er det "Glem tokenet" fra 1.0.0 (`glem_alle`).
#[tauri::command]
async fn forbindelse_glem(id: i64, tilstand: State<'_, Tilstand>, app: AppHandle) -> Svar<()> {
    findes(&tilstand, id)?;
    if laas(&tilstand)?.forbindelse_ider()?.len() == 1 {
        return glem_alle(&tilstand, &app).await;
    }
    glem_en(&tilstand, id).await?;
    tilstand.koe.prioriter(None);
    Ok(())
}

fn laas(tilstand: &Tilstand) -> Result<std::sync::MutexGuard<'_, Store>, Fejl> {
    tilstand.lager.lock().map_err(|_| Fejl::Lager)
}

/// Zonerne fra Cloudflare, forbindelse for forbindelse i id-orden. Hver forbindelses liste erstatter dens gemte (`Store::gem_zoner`), og kontoens navn
/// gemmes. En fejl på én stopper ikke de andre; fejler alle, er det den første fejl. Svaret er den samlede liste (`zones_cache`), og køen får besked.
#[tauri::command]
async fn zones(tilstand: State<'_, Tilstand>) -> Svar<Vec<Zone>> {
    let z = tilstand
        .laase
        .koer(ZONELISTEN, async {
            let (mut nogen, mut fejl) = (false, None);
            for id in forbindelse_ider(&tilstand)? {
                let svar = async {
                    let t = gemt_token(&tilstand, id).await?;
                    tilstand.klient.zoner(&t).await
                }
                .await;
                match svar {
                    Ok(l) => {
                        let mut lager = laas(&tilstand)?;
                        lager.gem_zoner(id, &l.zoner, &vaern::iso(nu_ms()))?;
                        lager.gem_cf_navn(id, l.konto_navn.as_deref(), l.konti)?;
                        if let Ok(mut h) = tilstand.zoner_hentet.lock() {
                            h.insert(id);
                        }
                        nogen = true;
                    }
                    Err(e) => {
                        fejl.get_or_insert(e);
                    }
                }
            }
            match (nogen, fejl) {
                (false, Some(e)) => Err(e),
                _ => Ok::<_, Fejl>(laas(&tilstand)?.zoner()?.0),
            }
        })
        .await?;
    tilstand.koe.prioriter(None);
    Ok(z)
}

#[derive(Serialize)]
struct ZoneCache {
    zoner: Vec<Zone>,
    /// Hvornår listen blev hentet (ISO, UTC); `null`, hvis der ikke er gemt nogen.
    hentet: Option<String>,
}

/// Den senest hentede zoneliste fra cachen (intet netværk), så forsiden kan vise zonerne med det samme og uden net.
#[tauri::command]
async fn zones_cache(tilstand: State<'_, Tilstand>) -> Svar<ZoneCache> {
    let (zoner, hentet) = laas(&tilstand)?.zoner()?;
    Ok(ZoneCache { zoner, hentet })
}

fn byg(tilstand: &Tilstand, zone: &str, nu: i64) -> Result<Option<vaern::VaernUd>, Fejl> {
    let lager = laas(tilstand)?;
    let Some(sidst) = vaern::sidst_hentet(&lager.hentet(zone)?, nu) else {
        return Ok(None);
    };
    let rows = lager.raekker(zone, &overblik::hele_fra(nu))?;
    let mut ud = vaern::oversigt_ud(&rows, nu, Some(sidst));
    ud.lister.med_historik(&lager.fw_datoer(&[zone.to_string()])?);
    ud.med_huller(lager.hentet(zone)?.keys(), nu);
    Ok(Some(ud))
}

/// Værnet ud fra cachen alene (intet netværk), så siden kan vise de gemte tal med det samme. `null` hvis intet er hentet.
#[tauri::command]
async fn vaern_cache(zone_id: String, tilstand: State<'_, Tilstand>) -> Svar<Option<vaern::VaernUd>> {
    let zone = vaern::tjek_zone(&zone_id)?;
    byg(&tilstand, &zone, nu_ms())
}

/// Henter zonens manglende døgn (i dag, i går og huller) under zonens lås og gemmer hvert døgn. Fælles for
/// `vaern_oversigt` og køen, så der aldrig hentes dobbelt; `nu` beregnes, når låsen er taget. Døgn uden for Cloudflares
/// opbevaring noteres og spørges der ikke om igen.
async fn hent_zone(
    tilstand: &Tilstand,
    token: &Hemmelig,
    zone: &str,
    paa_doegn: &mut (dyn FnMut(&str, Doegnudfald<'_>) + Send),
) -> Result<usize, Fejl> {
    tilstand
        .laase
        .koer(zone, async {
            let nu = nu_ms();
            let (klient, udenfor, uden_vaert) = (&tilstand.klient, &tilstand.udenfor, &tilstand.uden_vaert);
            let hent = move |dag: String| async move {
                let kendt = udenfor
                    .lock()
                    .map_err(|_| Fejl::Lager)?
                    .get(&(zone.to_string(), dag.clone()))
                    .cloned();
                if let Some(f) = kendt {
                    return Err(f);
                }
                // V-SUBDOM.1 B2 (trin 0): afviser Cloudflare værtsfeltet, hentes domænets tal uden det, og domænet noteres til næste UTC-døgn (FIX 1, V1).
                let variabler = sti200::variabler(zone, &dag)?;
                let uden = uden_vaert
                    .lock()
                    .map_err(|_| Fejl::Lager)?
                    .get(zone)
                    .is_some_and(|t| vaert::uden_gaelder(*t, nu_ms()));
                let (svar, afvist) = vaert::hent_hoved(&sti200::hoved_query(), uden, |q: String| {
                    let v = variabler.clone();
                    async move { klient.graphql(token, &q, v).await }
                })
                .await;
                if afvist {
                    uden_vaert.lock().map_err(|_| Fejl::Lager)?.insert(zone.to_string(), nu_ms());
                }
                vaern::doegn_fra_svar(&svar.map_err(|e| e.for_dag(&dag))?)
            };
            vaern::opdater(&tilstand.lager, zone, nu, hent, |dag, u| {
                if let Doegnudfald::UdenFor(f) = u {
                    if let Ok(mut m) = udenfor.lock() {
                        m.insert((zone.to_string(), dag.to_string()), f.clone());
                    }
                }
                paa_doegn(dag, u);
            })
            .await
        })
        .await
}

/// Henter og gemmer de ekstra tal for ét færdigt døgn. Afviser Cloudflare et felt, prøves der én gang til uden `clientAsn`, og
/// domænet huskes, så det ikke koster to kald pr. døgn.
async fn hent_ekstra(tilstand: &Tilstand, token: &Hemmelig, zone: &str, dag: &str) -> Result<(), Fejl> {
    let stier = adgang::stier_til_hentning(&laas(tilstand)?.login_stier(zone)?);
    let variabler = adgang::variabler(zone, dag, &stier)?;
    let uden = tilstand.uden_asn.lock().map_err(|_| Fejl::Lager)?.contains(zone);
    let mut svar = if uden {
        tilstand.klient.graphql(token, &adgang::query_uden_asn(), variabler.clone()).await
    } else {
        tilstand.klient.graphql(token, adgang::ADGANG_QUERY, variabler.clone()).await
    };
    if !uden && svar.as_ref().err().is_some_and(adgang::felt_afvist) {
        svar = tilstand.klient.graphql(token, &adgang::query_uden_asn(), variabler).await;
        if svar.is_ok() {
            tilstand.uden_asn.lock().map_err(|_| Fejl::Lager)?.insert(zone.to_string());
        }
    }
    let raekker = adgang::raekker(&adgang::fra_svar(&svar.map_err(|e| e.for_dag(dag))?)?);
    laas(tilstand)?.gem_ekstra(zone, dag, &raekker, &vaern::iso(nu_ms()))
}

/// Enkelthændelserne for et gammelt døgn: ét kald med `vaern::ANGREB_QUERY`, kun for færdige døgn og i `ekstra_runde`s tempo.
async fn hent_angreb(tilstand: &Tilstand, token: &Hemmelig, zone: &str, dag: &str) -> Result<(), Fejl> {
    let svar = tilstand
        .klient
        .graphql(token, vaern::ANGREB_QUERY, vaern::doegn_variabler(zone, dag)?)
        .await
        .map_err(|e| e.for_dag(dag))?;
    let raekker = vaern::angreb_raekker(&vaern::doegn_fra_svar(&svar)?);
    laas(tilstand)?.gem_angreb(zone, dag, &raekker, &vaern::iso(nu_ms()))
}

/// DNS-tallene for ét færdigt døgn: ét kald med `dns::DNS_QUERY`.
async fn hent_dns(tilstand: &Tilstand, token: &Hemmelig, zone: &str, dag: &str) -> Result<(), Fejl> {
    let svar = tilstand
        .klient
        .graphql(token, dns::DNS_QUERY, dns::variabler(zone, dag)?)
        .await
        .map_err(|e| e.for_dag(dag))?;
    laas(tilstand)?.gem_dns(zone, dag, &dns::raekker(&svar)?, &vaern::iso(nu_ms()))
}

/// DMARC-tallene for ét færdigt døgn. Felterne er ikke bekræftet af Cloudflare, så afviser Cloudflare forespørgslen, prøves den enkle én gang.
async fn hent_dmarc(tilstand: &Tilstand, token: &Hemmelig, zone: &str, dag: &str) -> Result<(), Fejl> {
    let v = dmarc::variabler(zone, dag)?;
    let svar = match tilstand.klient.graphql(token, dmarc::DMARC_QUERY, v.clone()).await {
        Err(e) if dmarc::afvist(&e) => tilstand.klient.graphql(token, &dmarc::query_enkel(), v).await,
        svar => svar,
    }
    .map_err(|e| e.for_dag(dag))?;
    laas(tilstand)?.gem_dmarc(zone, dag, &dmarc::raekker(&svar)?, &vaern::iso(nu_ms()))
}

/// Lækkede kodeord for ét færdigt døgn: ét kald med `adgang::LAEK_QUERY`. Afviser Cloudflare filteret, hentes der uden, resten af kørslen.
async fn hent_laek(tilstand: &Tilstand, token: &Hemmelig, zone: &str, dag: &str) -> Result<(), Fejl> {
    let v = vaern::doegn_variabler(zone, dag)?;
    let foerst = if tilstand.laek_uden_filter.load(Ordering::SeqCst) {
        adgang::LAEK_QUERY_UDEN_FILTER
    } else {
        adgang::LAEK_QUERY
    };
    let svar = match tilstand.klient.graphql(token, foerst, v.clone()).await {
        Err(e) if foerst == adgang::LAEK_QUERY && adgang::laek_filter_afvist(&e) => {
            tilstand.laek_uden_filter.store(true, Ordering::SeqCst);
            tilstand.klient.graphql(token, adgang::LAEK_QUERY_UDEN_FILTER, v).await
        }
        svar => svar,
    }
    .map_err(|e| e.for_dag(dag))?;
    laas(tilstand)?.gem_laek(zone, dag, &adgang::laek_raekker(&svar)?, &vaern::iso(nu_ms()))
}

/// Årstallene for døgnene `fra`…`til`: ét kald med `historik::HISTORIK_QUERY`, uanset hvor mange døgn der mangler.
async fn hent_historik(tilstand: &Tilstand, token: &Hemmelig, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
    let svar = tilstand
        .klient
        .graphql(token, historik::HISTORIK_QUERY, historik::variabler(zone, fra, til))
        .await?;
    let dage = robotter::dage_i(fra, til);
    laas(tilstand)?.gem_historik(zone, &dage, &historik::raekker(&svar)?, &vaern::iso(nu_ms()))
}

/// Unikke besøgende for døgn, der blev hentet før tallet kom med: ét kald med `historik::HISTORIK_QUERY`, og kun `unikke` gemmes (`Store::gem_unikke`).
async fn hent_unikke(tilstand: &Tilstand, token: &Hemmelig, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
    let svar = tilstand
        .klient
        .graphql(token, historik::HISTORIK_QUERY, historik::variabler(zone, fra, til))
        .await?;
    laas(tilstand)?.gem_unikke(zone, &historik::unikke_raekker(&svar)?)
}

/// Besøg pr. land for døgnene `fra`…`til`: ét kald med `besoeg::BESOEG_QUERY`.
async fn hent_besoeg(tilstand: &Tilstand, token: &Hemmelig, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
    let svar = tilstand
        .klient
        .graphql(token, besoeg::BESOEG_QUERY, besoeg::variabler(zone, fra, til)?)
        .await?;
    let dage = robotter::dage_i(fra, til);
    laas(tilstand)?.gem_besoeg(zone, &dage, &besoeg::raekker(&svar, &dage)?, &vaern::iso(nu_ms()))
}

/// IP-klasserne for døgn, der blev hentet før de kom med: ét kald med `historik::HISTORIK_QUERY`, og kun `h_ipklasse` gemmes (`Store::gem_ipklasse`).
async fn hent_ipklasse(tilstand: &Tilstand, token: &Hemmelig, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
    let svar = tilstand
        .klient
        .graphql(token, historik::HISTORIK_QUERY, historik::variabler(zone, fra, til))
        .await?;
    laas(tilstand)?.gem_ipklasse(zone, &historik::ipklasse_raekker(&svar)?)
}

/// Kendte robotter for døgnene `fra`…`til`: ét kald med `besoeg::ROBOT_QUERY`.
async fn hent_robot(tilstand: &Tilstand, token: &Hemmelig, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
    let svar = tilstand
        .klient
        .graphql(token, besoeg::ROBOT_QUERY, besoeg::variabler(zone, fra, til)?)
        .await?;
    let dage = robotter::dage_i(fra, til);
    laas(tilstand)?.gem_robot(zone, &dage, &besoeg::robot_raekker(&svar, &dage)?, &vaern::iso(nu_ms()))
}

/// Mest besøgte sider for døgnene `fra`…`til`: ét kald med `besoeg::SIDE_QUERY`. Værten renses under zonens navn (V-SUBDOM.1), og står svaret på 10.000
/// rækker, noteres det ved domænet (`store::SIDE_AFSKAARET`).
async fn hent_side(tilstand: &Tilstand, token: &Hemmelig, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
    let navn = laas(tilstand)?.zone_navn(zone)?.ok_or(Fejl::ZoneUkendt)?;
    let svar = tilstand
        .klient
        .graphql(token, besoeg::SIDE_QUERY, besoeg::variabler(zone, fra, til)?)
        .await?;
    let dage = robotter::dage_i(fra, til);
    let mut l = laas(tilstand)?;
    l.gem_side(zone, &dage, &besoeg::side_raekker(&svar, &dage, &navn)?, &vaern::iso(nu_ms()))?;
    l.noter_side_afskaaret(zone, fra, til, besoeg::side_afskaaret(&svar)?)
}

/// Trafik pr. vært for døgnene `fra`…`til` (V-SUBDOM.1): ét kald med `vaert::VAERT_QUERY`; står svaret på loftet, ét pr. døgn (`vaert::hent_opdelt`).
async fn hent_vaert(tilstand: &Tilstand, token: &Hemmelig, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
    let navn = laas(tilstand)?.zone_navn(zone)?.ok_or(Fejl::ZoneUkendt)?;
    let hent = |f: String, t: String| async move {
        tilstand
            .klient
            .graphql(token, vaert::VAERT_QUERY, besoeg::variabler(zone, &f, &t)?)
            .await
    };
    let (dage, rk) = vaert::hent_opdelt(fra, til, hent, |d, dage| vaert::trafik_raekker(d, dage, &navn), vaert::V_AFSKAARET).await?;
    laas(tilstand)?.gem_vaert(zone, &dage, &rk, &vaern::iso(nu_ms()))
}

/// Angreb pr. vært for døgn, der blev hentet uden værten (V-SUBDOM.1): ét kald med `vaert::VAERT_FW_QUERY`; står svaret på loftet, ét pr. døgn, og et døgn,
/// der stadig står på loftet, får intet tal. Skriver kun `v_fw`, aldrig `fw`.
async fn hent_vaert_fw(tilstand: &Tilstand, token: &Hemmelig, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
    let navn = laas(tilstand)?.zone_navn(zone)?.ok_or(Fejl::ZoneUkendt)?;
    let hent = |f: String, t: String| async move {
        tilstand
            .klient
            .graphql(token, vaert::VAERT_FW_QUERY, besoeg::variabler(zone, &f, &t)?)
            .await
    };
    let (dage, rk) = vaert::hent_opdelt(
        fra,
        til,
        hent,
        |d, dage| vaert::fw_dag_raekker(d, dage, &navn),
        vaert::V_FW_AFSKAARET,
    )
    .await?;
    laas(tilstand)?.gem_vaert_fw(zone, &dage, &rk, &vaern::iso(nu_ms()))
}

/// Zonens tilstand: GET /zones/{id}, Zone Read, som tokenet har i forvejen.
async fn hent_tilstand(tilstand: &Tilstand, token: &Hemmelig, zone: &str) -> Result<(), Fejl> {
    let z = tilstand.klient.zone_stand(token, zone).await?;
    laas(tilstand)?.gem_zone_tilstand(zone, &z, &vaern::iso(nu_ms()))
}

/// Cloudflares egne sikkerhedsfund for én forbindelse: højst én gang i timen pr. forbindelse (`sikkerhed::skal_hente`, regnet fra sidste forsøg), efter zonernes
/// tilstand i `ekstra_runde`. Kun fund for de zoner, forbindelsen ejer (`Store::ejede`, D3), gemmes, og kun dens fund erstattes. Uden rettigheden noteres det,
/// forbindelsens gamle fund slettes (de kan ikke længere holdes ajour), og resten af programmet virker. En anden fejl lader de gamle fund stå.
async fn hent_fund(tilstand: &Tilstand, forbindelse: i64, token: &Hemmelig) {
    let nu = nu_ms();
    let (hentet_noegle, tilstand_noegle) = (sikkerhed::hentet_noegle(forbindelse), sikkerhed::tilstand_noegle(forbindelse));
    let sidst = laas(tilstand)
        .ok()
        .and_then(|l| l.indstilling(&hentet_noegle).ok().flatten())
        .and_then(|s| vaern::parse_iso(&s));
    if !sikkerhed::skal_hente(sidst, nu) {
        return;
    }
    let svar = tilstand.klient.sikkerhedsfund(token).await;
    let afviste = tilstand.koe.afviste();
    let Ok(mut l) = laas(tilstand) else { return };
    let tid = vaern::iso(nu);
    let _ = l.gem_indstilling(&hentet_noegle, &tid);
    let _ = l.gem_indstilling(&tilstand_noegle, &sikkerhed::tilstand_efter(svar.as_ref().err()));
    let ejede = l.ejede(forbindelse, &afviste).unwrap_or_default();
    match svar {
        Ok(fund) => {
            let fund: Vec<sikkerhed::Fund> = fund.into_iter().filter(|f| ejede.contains(&f.zone)).collect();
            let _ = l.gem_sikkerhedsfund(&ejede, &fund, &tid);
        }
        Err(Fejl::Afvist(_)) => {
            let _ = l.gem_sikkerhedsfund(&ejede, &[], &tid);
        }
        Err(_) => {}
    }
}

/// Henter i dag, i går og manglende døgn fra Cloudflare, gemmer dem og returnerer Værnet (W's felter som `data` i W's
/// /api/maskine/vaern + hele døgn). Fejler hentningen, er de døgn, der nåede at komme hjem, alligevel gemt. Kører der
/// allerede en hentning for zonen (fra køen), venter denne, til den er færdig (lås pr. zone).
#[tauri::command]
async fn vaern_oversigt(zone_id: String, tilstand: State<'_, Tilstand>) -> Svar<vaern::VaernUd> {
    let zone = vaern::tjek_zone(&zone_id)?;
    // Kun valgte domæner hentes, heller ikke fra en gammel adresse.
    if laas(&tilstand)?.valgte()?.contains(&zone) {
        let token = token_til(&tilstand, &zone).await?;
        hent_zone(&tilstand, &token, &zone, &mut |_, _| {}).await?;
    }
    byg(&tilstand, &zone, nu_ms())?.ok_or(Fejl::Svar)
}

fn signal_for(z: &Zone, rows: &[store::Raekke], hentet: &HashMap<String, String>) -> overblik::Signal {
    let mut s = overblik::signal(rows, hentet);
    (s.zone, s.navn) = (z.id.clone(), z.navn.clone());
    s
}

/// Boksen "Vurdering" for ét domæne og én dag, mod de valgte domæner.
fn vurdering_for(lager: &Store, zone: &str, dag: &str, nu: i64) -> Result<overblik::Vurdering, Fejl> {
    let ms = vaern::dag_ms(dag).ok_or(Fejl::UgyldigDato)?;
    let fra = overblik::hele_fra(nu).min(vaern::dag_noegle(ms - 10 * vaern::DOEGN_MS));
    let waf = radar::verden_serie(&lager.radar_raekker("", &fra)?, radar::L7_WAF);
    let navne: HashMap<String, String> = lager.zoner()?.0.into_iter().map(|z| (z.id, z.navn)).collect();
    let egen_zone = Zone {
        id: zone.to_string(),
        navn: navne.get(zone).cloned().unwrap_or_default(),
    };
    let rows = lager.raekker_tal(zone, &fra)?;
    let egen = signal_for(&egen_zone, &rows, &lager.hentet(zone)?);
    let mut signaler = Vec::new();
    for z in lager.zoner_til()?.iter().filter(|z| z.id != zone) {
        signaler.push(signal_for(z, &lager.raekker_tal(&z.id, &fra)?, &lager.hentet(&z.id)?));
    }
    let andre: Vec<&overblik::Signal> = signaler.iter().collect();
    Ok(overblik::vurdering(
        dag,
        &rows,
        &teknologi::profil(lager, zone)?,
        &egen,
        &andre,
        radar::forhold(&waf, dag),
    ))
}

/// "Mails sendt i dit domænes navn" for de 30 hele dage, kun fra cachen.
#[tauri::command]
async fn email_oversigt(zone: String, fra: Option<String>, til: Option<String>, tilstand: State<'_, Tilstand>) -> Svar<dmarc::EmailUd> {
    let zone = vaern::tjek_zone(&zone)?;
    if [&fra, &til].into_iter().flatten().any(|d| vaern::dag_ms(d).is_none()) {
        return Err(Fejl::UgyldigDato);
    }
    let lager = laas(&tilstand)?;
    let fra = fra.unwrap_or_else(|| overblik::hele_fra(nu_ms()));
    let rows: Vec<store::Raekke> = lager
        .raekker_art(std::slice::from_ref(&zone), &["dmarc"], &fra)?
        .into_iter()
        .map(|(_, r)| r)
        .filter(|r| til.as_ref().is_none_or(|t| r.dag <= *t))
        .collect();
    let fejl = lager.afsnit_fejl(store::DMARC_FEJL, &zone)?;
    Ok(dmarc::oversigt(&rows, lager.hentet_dmarc(&zone)?.len(), fejl))
}

/// "Følsomme filer, der svarede 200" på Angreb for ét domæne og en periode, kun fra cachen (`sti200.rs`).
#[tauri::command]
async fn sti200_oversigt(zone: String, fra: Option<String>, til: Option<String>, tilstand: State<'_, Tilstand>) -> Svar<sti200::Sti200Ud> {
    let zone = vaern::tjek_zone(&zone)?;
    if [&fra, &til].into_iter().flatten().any(|d| vaern::dag_ms(d).is_none()) {
        return Err(Fejl::UgyldigDato);
    }
    let lager = laas(&tilstand)?;
    let nu = nu_ms();
    let fra = fra.unwrap_or_else(|| vaern::dag_noegle(nu - 7 * vaern::DOEGN_MS));
    let til = til.unwrap_or_else(|| vaern::dag_noegle(nu - vaern::DOEGN_MS));
    let rows: Vec<store::Raekke> = lager
        .raekker_art(std::slice::from_ref(&zone), &[sti200::ART, sti200::ART_BYTES], &fra)?
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    // Døgn hentet med hovedhentningen i perioden. Døgn fra før V-TOKEN.1 har ingen af arterne, men står som hentet: teksten siger det.
    let hentede = lager.hentet(&zone)?.keys().filter(|d| **d >= fra && **d <= til).count();
    Ok(sti200::oversigt(&rows, &fra, &til, hentede))
}

/// "Cloudflares egne fund" på Angreb for ét domæne, kun fra cachen (`sikkerhed.rs`).
#[tauri::command]
async fn sikkerhedsfund_oversigt(zone: String, tilstand: State<'_, Tilstand>) -> Svar<sikkerhed::FundUd> {
    let zone = vaern::tjek_zone(&zone)?;
    let afviste = tilstand.koe.afviste();
    let lager = laas(&tilstand)?;
    // Tilstanden og tidspunktet er zonens forbindelses (D3).
    let f = match lager.ejere(&afviste)?.get(&zone) {
        Some(f) => *f,
        None => lager.forbindelser_for(&zone)?.first().copied().unwrap_or(1),
    };
    Ok(sikkerhed::oversigt(
        lager.sikkerhedsfund(Some(&zone))?,
        lager.indstilling(&sikkerhed::tilstand_noegle(f))?.as_deref(),
        lager.indstilling(&sikkerhed::hentet_noegle(f))?,
    ))
}

/// Et klik på et af Cloudflares egne fund: Cloudflares faste side for Security Insights i brugerens browser (`sikkerhed::DASH`). Webvisningen kan ikke vælge en anden adresse.
#[tauri::command]
async fn aabn_sikkerhedsfund() -> Svar<()> {
    tauri::async_runtime::spawn_blocking(|| aabn_i_browser(sikkerhed::DASH))
        .await
        .map_err(|_| Fejl::Intern)?
        .map_err(|_| Fejl::Intern)
}

#[tauri::command]
async fn vurdering(zone: String, dag: String, tilstand: State<'_, Tilstand>) -> Svar<overblik::Vurdering> {
    let zone = vaern::tjek_zone(&zone)?;
    vurdering_for(&*laas(&tilstand)?, &zone, &dag, nu_ms())
}

/// Én række pr. valgt domæne, kun fra cachen (intet netværk). Sorteret rød, gul, grøn, grå. Samme rækker
/// til overblikket, "Siden sidst" og notifikationerne, så et domæne, der ikke er valgt, er væk alle tre steder.
fn byg_overblik(lager: &Store, nu: i64) -> Result<Vec<overblik::Raekke1>, Fejl> {
    let zoner = lager.zoner_til()?;
    let fra = overblik::hele_fra(nu);
    let waf = radar::verden_serie(&lager.radar_raekker("", &fra)?, radar::L7_WAF);
    let land = lager.indstilling(radar::LAND)?.unwrap_or_default();
    // Uden et valgt land er der intet at sige: rækkerne uden land er hele verdens forstyrrelser.
    let udfald = if land.is_empty() {
        Vec::new()
    } else {
        lager.radar_raekker(&land, &fra)?
    };
    let lokal_dag = notify::tidszone(lager)?.dag(nu);
    let vurderet = lager.vurderet()?;
    let (tz, stande) = (notify::tidszone(lager)?, lager.zone_tilstande()?);
    let alle_fund = lager.sikkerhedsfund(None)?;
    let mut data = Vec::with_capacity(zoner.len());
    for z in &zoner {
        let rows = lager.raekker_tal(&z.id, &fra)?;
        let profil = teknologi::profil(lager, &z.id)?;
        let hentet = lager.hentet(&z.id)?;
        let signal = signal_for(z, &rows, &hentet);
        data.push((overblik::raekke_med(z, &hentet, &rows, nu, &profil), rows, profil, signal));
    }
    let mut ud = Vec::with_capacity(data.len());
    for (i, (r, rows, profil, signal)) in data.iter().enumerate() {
        // Kun valgte domæner : fravalgte hentes ikke, og deres tal bliver gamle.
        let andre: Vec<&overblik::Signal> = data.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, x)| &x.3).collect();
        let mut r = r.clone();
        r.tendens = overblik::efter_boelgen(radar::tendens_for(&r, rows, &waf), &andre, &r.status_dag);
        r.forstyrrelse = r.server.as_ref().and_then(|s| radar::forstyrrelse(&udfald, &land, &s.dag));
        let m = overblik::moenstre(
            &r,
            rows,
            profil,
            signal,
            &andre,
            &|d| radar::forhold(&waf, d),
            vurderet.get(&r.id).map(String::as_str),
        );
        (r.moenster, r.afgjort, r.efterslaeb, r.vurderet) = (m.vis, m.afgjort, m.efterslaeb, m.vurderet);
        r.forgaars = overblik::er_forgaars(&r.status_dag, &lokal_dag);
        r.kan_sammenligne = overblik::kan_sammenligne(&andre, &r.status_dag);
        if let Some((z, hentet)) = stande.get(&r.id) {
            let kl = |ms: i64| {
                let (t, m) = tz.kl(ms);
                format!("{t:02}.{m:02}")
            };
            let fund = overblik::tilstand_fund(z, vaern::parse_iso(hentet).unwrap_or(nu), nu, &kl);
            overblik::med_tilstand(&mut r, fund);
        }
        let fund: Vec<sikkerhed::Fund> = alle_fund.iter().filter(|f| f.zone == r.id).cloned().collect();
        overblik::med_fund(&mut r, &fund);
        ud.push(r);
    }
    overblik::sorter(&mut ud);
    Ok(ud)
}

#[tauri::command]
async fn overblik(tilstand: State<'_, Tilstand>) -> Svar<Vec<overblik::Raekke1>> {
    let lager = laas(&tilstand)?;
    byg_overblik(&lager, nu_ms())
}

/// De valgte domæner (id'er).
#[tauri::command]
async fn valg_hent(tilstand: State<'_, Tilstand>) -> Svar<Vec<String>> {
    laas(&tilstand)?.valg()
}

#[derive(Serialize, Debug, PartialEq)]
struct DomaeneValg {
    id: String,
    navn: String,
    til: bool,
    /// Nyt på kontoen og venter på et svar (linjen i Overblik).
    ny: bool,
    /// Den forbindelse, domænet hører til (D3; V-KONTI.1). `null`, hvis kun afviste forbindelser ser det.
    forbindelse: Option<i64>,
    /// Alle de forbindelser, der ser domænet, laveste id først, også afviste (V-KONTI.1 FIX 1, F2): Domæner viser det i hver af deres grupper. Kun id'er.
    forbindelser: Vec<i64>,
}

/// Hele zonelisten med kontakten pr. domæne, ejeren og alle, der ser det (Indstillinger → Domæner). Ren funktion.
fn domaener_ud(
    zoner: Vec<Zone>,
    valgte: &HashSet<String>,
    nye: &HashSet<String>,
    ejere: &HashMap<String, i64>,
    seere: &HashMap<String, Vec<i64>>,
) -> Vec<DomaeneValg> {
    zoner
        .into_iter()
        .map(|z| DomaeneValg {
            til: valgte.contains(&z.id),
            ny: nye.contains(&z.id),
            forbindelse: ejere.get(&z.id).copied(),
            forbindelser: seere.get(&z.id).cloned().unwrap_or_default(),
            id: z.id,
            navn: z.navn,
        })
        .collect()
}

#[tauri::command]
async fn domaener_hent(tilstand: State<'_, Tilstand>) -> Svar<Vec<DomaeneValg>> {
    let afviste = tilstand.koe.afviste();
    let lager = laas(&tilstand)?;
    Ok(domaener_ud(
        lager.zoner()?.0,
        &lager.valgte()?,
        &lager.nye()?,
        &lager.ejere(&afviste)?,
        &lager.seere()?,
    ))
}

/// Det første domæne, der er kommet til: det står forrest i køen. Ren funktion.
fn nyvalgt(foer: &HashSet<String>, nu: &[String]) -> Option<String> {
    nu.iter().filter(|z| !foer.contains(*z)).min().cloned()
}

/// Gemmer de valgte domæner. Tallene for et fravalgt domæne bliver liggende. Kommer et domæne til, vækkes køen, som henter det, der mangler.
#[tauri::command]
async fn domaener_saet(valgte: Vec<String>, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let valgte = valgte.iter().map(|z| vaern::tjek_zone(z)).collect::<Result<Vec<_>, _>>()?;
    let foerst = {
        let mut lager = laas(&tilstand)?;
        let foer = lager.valgte()?;
        lager.gem_valgte(&valgte)?;
        nyvalgt(&foer, &valgte)
    };
    if foerst.is_some() {
        tilstand.koe.prioriter(foerst);
    }
    Ok(())
}

/// "Ikke nu" til nye domæner: linjen i Overblik kommer ikke igen for dem.
#[tauri::command]
async fn domaene_afvis(zoner: Vec<String>, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let zoner = zoner.iter().map(|z| vaern::tjek_zone(z)).collect::<Result<Vec<_>, _>>()?;
    laas(&tilstand)?.afvis_nye(&zoner)
}

#[derive(Serialize)]
struct Samlet {
    /// Antal valgte domæner, og hvor mange af dem der har tal i cachen.
    domaener: usize,
    med_tal: usize,
    /// De valgte domæner efter navn (til etiketterne i heroen).
    valgte: Vec<Valgt>,
    /// `null`, hvis ingen af de valgte har tal endnu.
    vaern: Option<vaern::VaernUd>,
}

#[derive(Serialize, Debug, PartialEq)]
struct Valgt {
    id: String,
    /// Navnet fra den gemte zoneliste ("…", hvis zonen ikke står i den).
    navn: String,
    med_tal: bool,
}

/// De valgte zoner med navn, sorteret efter navn. Ren funktion.
fn valgte(zoner: &[String], navne: &HashMap<String, String>, med_tal: &[String]) -> Vec<Valgt> {
    let mut v: Vec<Valgt> = zoner
        .iter()
        .map(|z| Valgt {
            id: z.clone(),
            navn: navne.get(z).cloned().unwrap_or_else(|| "…".into()),
            med_tal: med_tal.contains(z),
        })
        .collect();
    v.sort_by(|a, b| a.navn.cmp(&b.navn).then_with(|| a.id.cmp(&b.id)));
    v
}

/// Det samlede Værn for de valgte zoner, kun fra cachen (intet netværk; køen henter). "Hentet" = det ældste blandt dem.
#[tauri::command]
async fn vaern_samlet_cache(zoner: Vec<String>, tilstand: State<'_, Tilstand>) -> Svar<Samlet> {
    let zoner = zoner.iter().map(|z| vaern::tjek_zone(z)).collect::<Result<Vec<_>, _>>()?;
    let nu = nu_ms();
    let lager = laas(&tilstand)?;
    let navne: HashMap<String, String> = lager.zoner()?.0.into_iter().map(|z| (z.id, z.navn)).collect();
    let mut med_tal = Vec::new();
    let mut aeldste: Option<String> = None;
    for z in &zoner {
        if let Some(s) = vaern::sidst_hentet(&lager.hentet(z)?, nu) {
            med_tal.push(z.clone());
            aeldste = Some(aeldste.map_or(s.clone(), |a| a.min(s)));
        }
    }
    let vaern = match aeldste {
        Some(genereret) => {
            let rows = lager.raekker_for(&med_tal, &overblik::hele_fra(nu))?;
            let mut ud = vaern::samlet(&rows, &navne, nu, genereret);
            ud.lister.med_historik(&lager.fw_datoer(&med_tal)?);
            let mut hentet = HashSet::new();
            for z in &med_tal {
                hentet.extend(lager.hentet(z)?.into_keys());
            }
            ud.med_huller(hentet.iter(), nu);
            Some(ud)
        }
        None => None,
    };
    Ok(Samlet {
        domaener: zoner.len(),
        med_tal: med_tal.len(),
        valgte: valgte(&zoner, &navne, &med_tal),
        vaern,
    })
}

/// Dyk ned i de gemte tal: ét filter pr. dimension, kun fra cachen (intet netværk). Der læses under låsen og regnes bagefter,
/// så køen ikke venter på optællingen.
#[tauri::command]
async fn dyk(
    omfang: dyk::Omfang,
    filtre: Vec<dyk::Filter>,
    periode: dyk::Periode,
    slags: Option<dyk::Slags>,
    side: Option<usize>,
    tilstand: State<'_, Tilstand>,
) -> Svar<dyk::DykUd> {
    let mut spoerg = dyk::tjek(omfang, filtre, periode, slags)?;
    spoerg.side = side.unwrap_or(1).saturating_sub(1);
    let nu = nu_ms();
    let laest = dyk::laes(&*laas(&tilstand)?, &spoerg, nu)?;
    Ok(dyk::regn(&spoerg, &laest, nu))
}

/// Trafik-dyk: hvor en dags forespørgsler kom fra, kun fra cachen (intet netværk). Noget andet end `dyk`, der viser det, Cloudflare stoppede.
#[tauri::command]
async fn trafik_dag(omfang: dyk::Omfang, dag: String, til: Option<String>, tilstand: State<'_, Tilstand>) -> Svar<dyk::TrafikUd> {
    let til = til.unwrap_or_else(|| dag.clone());
    let periode = dyk::Periode::Spaend {
        fra: dag.clone(),
        til: til.clone(),
    };
    let spoerg = dyk::tjek(omfang, Vec::new(), periode, None)?;
    let (rows, ekstra) = dyk::trafik_laes(&*laas(&tilstand)?, &spoerg, &dag, &til)?;
    Ok(dyk::trafik_regn(&rows, &dag, &til, spoerg.zoner.len() == 1, ekstra))
}

/// "Dit sites teknologi" for ét domæne, kun fra cachen (intet netværk).
#[tauri::command]
async fn teknologi_hent(zone: String, tilstand: State<'_, Tilstand>) -> Svar<teknologi::TeknologiUd> {
    let zone = vaern::tjek_zone(&zone)?;
    let lager = laas(&tilstand)?;
    let rows: Vec<store::Raekke> = lager
        .raekker_art(std::slice::from_ref(&zone), teknologi::ARTER, "0000-00-00")?
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    Ok(teknologi::ud(&rows, &teknologi::profil(&lager, &zone)?, nu_ms()))
}

/// Siden `#teknologi`: de valgte domæner og kataloget, kun fra cachen. Nye sete teknologier afkrydses her, for det er her, de vises.
fn teknologi_side_for(lager: &mut Store, nu: i64) -> Result<teknologi::SideUd, Fejl> {
    let mut domaener = Vec::new();
    for z in lager.zoner_til()? {
        let rows: Vec<store::Raekke> = lager
            .raekker_art(std::slice::from_ref(&z.id), teknologi::ARTER, "0000-00-00")?
            .into_iter()
            .map(|(_, r)| r)
            .collect();
        domaener.push(teknologi::side_domaene(
            &z.id,
            &z.navn,
            &rows,
            &teknologi::profil(lager, &z.id)?,
            nu,
        ));
    }
    let nye = teknologi::nye_vis(&domaener, &lager.tek_vis()?);
    lager.tilfoej_tek_vis(&nye)?;
    Ok(teknologi::SideUd {
        katalog: teknologi::katalog(&domaener, &lager.tek_vis()?),
        domaener,
    })
}

#[tauri::command]
async fn teknologi_side(tilstand: State<'_, Tilstand>) -> Svar<teknologi::SideUd> {
    teknologi_side_for(&mut *laas(&tilstand)?, nu_ms())
}

/// "Vis i tabellen". Ændrer kun kolonnerne, aldrig et valg.
#[tauri::command]
async fn teknologi_vis_saet(tek: String, vis: bool, tilstand: State<'_, Tilstand>) -> Svar<()> {
    if !teknologi::kendt(&tek) {
        return Err(Fejl::UgyldigtFilter);
    }
    laas(&tilstand)?.saet_tek_vis(&tek, vis)
}

/// Samme valg på alle valgte domæner: "Bruger ikke på alle" og "Nulstil kolonnen".
#[tauri::command]
async fn teknologi_kolonne(tek: String, valg: String, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let v = teknologi::Valg::fra(&valg).ok_or(Fejl::UgyldigtFilter)?;
    let mut lager = laas(&tilstand)?;
    for z in lager.valg()? {
        teknologi::saet(&mut lager, &z, &tek, v, false, &vaern::iso(nu_ms()))?;
    }
    Ok(())
}

/// "Tilføj teknologi": et navn fra listen eller fri tekst. Svaret er id'et.
#[tauri::command]
async fn teknologi_tilfoej(navn: String, tilstand: State<'_, Tilstand>) -> Svar<String> {
    let tek = teknologi::id_for(&navn).ok_or(Fejl::UgyldigtFilter)?;
    let mut lager = laas(&tilstand)?;
    lager.tilfoej_tek_vis(&[(tek.clone(), true, false, true)])?;
    lager.saet_tek_vis(&tek, true)?;
    Ok(tek)
}

/// Gemmer ét valg. Status og Hvorfor følger med ved næste `overblik`.
#[tauri::command]
async fn teknologi_saet(zone: String, tek: String, valg: String, besked: bool, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let zone = vaern::tjek_zone(&zone)?;
    let v = teknologi::Valg::fra(&valg).ok_or(Fejl::UgyldigtFilter)?;
    teknologi::saet(&mut *laas(&tilstand)?, &zone, &tek, v, besked, &vaern::iso(nu_ms()))
}

/// "Anvend på alle domæner": domænets valg lægges på de andre valgte domæner. Svaret er, hvor mange der fik dem.
#[tauri::command]
async fn teknologi_alle(zone: String, tilstand: State<'_, Tilstand>) -> Svar<usize> {
    let zone = vaern::tjek_zone(&zone)?;
    let mut lager = laas(&tilstand)?;
    let andre: Vec<String> = lager.valg()?.into_iter().filter(|z| *z != zone).collect();
    lager.kopier_teknologi(&zone, &andre, &vaern::iso(nu_ms()))?;
    Ok(andre.len())
}

/// Linjen under Indstillinger → Domæner: antal valg pr. valgt domæne, og om der står et valg nogen steder, også på et fravalgt domæne.
#[derive(Serialize, Debug, PartialEq)]
struct TekOversigt {
    zoner: HashMap<String, usize>,
    valg: bool,
}

fn tek_oversigt(lager: &Store) -> Result<TekOversigt, Fejl> {
    let mut zoner = HashMap::new();
    for zone in lager.valg()? {
        let n = lager.teknologi(&zone)?.len();
        zoner.insert(zone, n);
    }
    let valg = !lager.teknologi_valg()?.is_empty();
    Ok(TekOversigt { zoner, valg })
}

#[tauri::command]
async fn teknologi_oversigt(tilstand: State<'_, Tilstand>) -> Svar<TekOversigt> {
    tek_oversigt(&*laas(&tilstand)?)
}

/// Sætter alt til "Ved ikke": for ét domæne, eller for alle uden `zone`.
#[tauri::command]
async fn teknologi_nulstil(zone: Option<String>, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let zone = zone.map(|z| vaern::tjek_zone(&z)).transpose()?;
    laas(&tilstand)?.nulstil_teknologi(zone.as_deref())
}

/// Huller i historikken, kun fra cachen.
#[tauri::command]
async fn huller(tilstand: State<'_, Tilstand>) -> Svar<huller::HullerUd> {
    huller::laes(&*laas(&tilstand)?, nu_ms())
}

/// Linjen om et hul er lukket: den kommer ikke igen for det hul.
#[tauri::command]
async fn hul_luk(noegle: String, tilstand: State<'_, Tilstand>) -> Svar<()> {
    huller::luk(&mut *laas(&tilstand)?, &noegle)
}

/// Robotter: de gemte tal for ét domæne eller de valgte tilsammen, kun fra cachen (intet netværk).
#[tauri::command]
async fn robotter(omfang: dyk::Omfang, periode: dyk::Periode, tilstand: State<'_, Tilstand>) -> Svar<robotter::RobotterUd> {
    let spoerg = dyk::tjek(omfang, Vec::new(), periode, None)?;
    let nu = nu_ms();
    let laest = robotter::laes(&*laas(&tilstand)?, &spoerg, nu)?;
    Ok(robotter::regn(&spoerg.periode, &laest, nu))
}

/// Besøgende: unikke besøgende pr. dag, besøg pr. land, kendte robotter og mest besøgte sider for ét domæne eller de valgte tilsammen, kun fra cachen (intet
/// netværk). Højst ét filter: `land` (besøg fra landet), `sti` (visninger af siden) eller `robot` (besøg fra robotgruppen); grafen viser filtrets tal.
#[tauri::command]
async fn besoegende(
    omfang: dyk::Omfang,
    periode: dyk::Periode,
    land: Option<String>,
    sti: Option<String>,
    robot: Option<String>,
    vaert: Option<String>,
    tilstand: State<'_, Tilstand>,
) -> Svar<besoeg::BesoegUd> {
    let spoerg = dyk::tjek(omfang, Vec::new(), periode, None)?;
    let filter = besoeg::tjek_filter(land, sti, robot)?;
    // V-SUBDOM.1 B2: siderne pr. vært, kun for ét domæne; `vaert` (labelen under domænet) tager kun den værts sider.
    let vaert = besoeg::tjek_vaert(vaert)?;
    if vaert.is_some() && spoerg.zoner.len() != 1 {
        return Err(Fejl::UgyldigtFilter);
    }
    let mut laest = {
        let lager = laas(&tilstand)?;
        let mut l = besoeg::laes(&lager, &spoerg.zoner)?;
        // V-SUBDOM.1 FIX 1: med et filter pr. vært er lande og robotter værtens (`v_land`, `v_robot`), aldrig domænets.
        if let (Some(v), [zone]) = (vaert.as_deref(), spoerg.zoner.as_slice()) {
            besoeg::med_vaert_lr(&mut l, vaert::laes_lr(&lager, zone, v)?);
        }
        l
    };
    laest.med_vaert = spoerg.zoner.len() == 1;
    laest.vaert = vaert;
    Ok(besoeg::regn(&spoerg.periode, filter.as_ref(), &laest, nu_ms()))
}

/// Subdomæner (V-SUBDOM.1, til B2): trafik og angreb pr. vært for ét domæne i `fra`…`til` (højst et år), pr. dag, kun fra cachen (intet netværk). Svaret har
/// ingen id'er; værterne er nøglerne under zonen (`""` = domænet selv).
#[tauri::command]
async fn subdomaener(zone: String, fra: String, til: String, tilstand: State<'_, Tilstand>) -> Svar<vaert::SubdomaenerUd> {
    let zone = vaern::tjek_zone(&zone)?;
    vaert::tjek_periode(&fra, &til)?;
    let mut laest = vaert::laes(&*laas(&tilstand)?, &zone, &fra)?;
    // Hovedhentningen afviste værtsfeltet for domænet i dag (trin 0, FIX 1): uden tal pr. vært for angrebene i perioden er de ikke tilgængelige.
    laest.uden_vaert = tilstand
        .uden_vaert
        .lock()
        .map_err(|_| Fejl::Lager)?
        .get(&zone)
        .is_some_and(|t| vaert::uden_gaelder(*t, nu_ms()));
    Ok(vaert::regn(&laest, &fra, &til))
}

/// Login: de gemte tal for domænets login-stier, kun fra cachen (intet netværk).
#[tauri::command]
async fn login(omfang: dyk::Omfang, periode: dyk::Periode, filter: login::Filter, tilstand: State<'_, Tilstand>) -> Svar<login::LoginUd> {
    let spoerg = dyk::tjek(omfang, Vec::new(), periode, None)?;
    let filter = login::tjek(filter)?;
    let nu = nu_ms();
    let laest = login::laes(&*laas(&tilstand)?, &spoerg, nu)?;
    Ok(login::regn(&spoerg.periode, &filter, &laest, nu))
}

/// Indstillingen af login-stier for ét domæne: startlisten, egne stier og forslag fra data.
#[tauri::command]
async fn login_stier(zone: String, tilstand: State<'_, Tilstand>) -> Svar<login::StierUd> {
    let zone = vaern::tjek_zone(&zone)?;
    login::stier(&*laas(&tilstand)?, &zone)
}

/// Slår en login-sti til eller fra. En ny sti uden for startlisten er ikke i de hentede døgn: de hentes igen i baggrunden.
#[tauri::command]
async fn login_sti_saet(zone: String, sti: String, til: bool, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let zone = vaern::tjek_zone(&zone)?;
    login::saet_sti(&mut *laas(&tilstand)?, &zone, &sti, til)?;
    tilstand.koe.prioriter(None);
    Ok(())
}

#[tauri::command]
async fn login_sti_fjern(zone: String, sti: String, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let zone = vaern::tjek_zone(&zone)?;
    login::fjern_sti(&mut *laas(&tilstand)?, &zone, &sti)
}

#[derive(Serialize)]
struct LandValg {
    /// Det gemte land; `null`, hvis der ikke er valgt noget.
    land: Option<String>,
    /// Et andet land, kun når det er tydeligt, at mindst halvdelen af de besøgende kommer derfra (`radar::forslag`).
    forslag: Option<String>,
    /// Landene, Radar har haft tal for som mål på mindst 15 af de seneste 30 dage (`radar::lande_med_tal`). Kun dem kan vælges (V-BESOEG.2 FIX 1).
    med_tal: Vec<String>,
}

#[tauri::command]
async fn land_hent(tilstand: State<'_, Tilstand>) -> Svar<LandValg> {
    let nu = nu_ms();
    let lager = laas(&tilstand)?;
    let rows = lager.raekker_art(&lager.valg()?, &["land", "alle", "bot"], &overblik::hele_fra(nu))?;
    let land = lager.indstilling(radar::LAND)?;
    let med_tal = radar::lande_med_tal(&lager, nu)?;
    Ok(LandValg {
        forslag: radar::forslag(&rows, land.as_deref()).filter(|f| med_tal.contains(f)),
        land,
        med_tal,
    })
}

#[tauri::command]
async fn land_saet(kode: String, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let kode = radar::tjek_land(&kode)?;
    laas(&tilstand)?.gem_indstilling(radar::LAND, &kode)
}

/// <Land> lige nu og Verden, kun fra cachen (intet netværk; `radar_opdater` henter). `waf`: verden som `mitigationProduct=WAF`.
#[tauri::command]
async fn land_visning(kode: String, waf: bool, tilstand: State<'_, Tilstand>) -> Svar<radar::LandUd> {
    let kode = radar::tjek_land(&kode)?;
    let nu = nu_ms();
    let laest = radar::laes(&*laas(&tilstand)?, &kode, nu)?;
    Ok(radar::regn(&kode, waf, &laest, nu))
}

struct RadarKilde<'a> {
    klient: &'a Klient,
    token: &'a Hemmelig,
}

impl radar::Kilde for RadarKilde<'_> {
    async fn hent(&self, sti: &str) -> Result<Value, Fejl> {
        self.klient.radar(self.token, sti).await
    }
}

/// Opdaterer Radar under sin egen lås og melder `radar`, hvis der blev spurgt. Kører uden for køen: en fejl her når aldrig køen.
async fn radar_runde(app: &AppHandle, token: &Hemmelig, kode: Option<&str>, interval: i64) -> Result<(), Fejl> {
    let t = app.state::<Tilstand>();
    let kilde = RadarKilde { klient: &t.klient, token };
    let spurgt = t
        .laase
        .koer(RADAR, radar::koer(&t.lager, &kilde, kode, nu_ms(), interval, radar::TEMPO))
        .await?;
    if spurgt {
        let _ = app.emit("radar", ());
    }
    Ok(())
}

/// Visningen er åbnet: det, der er over 1 time gammelt, hentes. Svaret kommer som hændelsen `radar`.
#[tauri::command]
async fn radar_opdater(kode: String, tilstand: State<'_, Tilstand>, app: AppHandle) -> Svar<()> {
    let kode = radar::tjek_land(&kode)?;
    let token = et_token(&tilstand).await?;
    radar_runde(&app, &token, Some(&kode), radar::AABEN_MS).await
}

/// Hver 6. time, tjekket hvert `koe::TIK`, så dvale ikke flytter den. Kun med et token i hukommelsen: baggrunden må ikke spørge
/// nøgleringen af sig selv.
fn start_radar(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(koe::TIK).await;
            let t = app.state::<Tilstand>();
            let afviste = t.koe.afviste();
            let ider: Vec<i64> = forbindelse_ider(&t)
                .unwrap_or_default()
                .into_iter()
                .filter(|f| !afviste.contains(f))
                .collect();
            let Some(token) = t.token.et_af(&ider) else { continue };
            let land = laas(&t).ok().and_then(|l| l.indstilling(radar::LAND).ok().flatten());
            let _ = radar_runde(&app, &token, land.as_deref(), radar::OPDATER_MS).await;
        }
    });
}

/// Zonen på skærmen hentes først (`null`: ingen bestemt); vækker køen til en ny runde (også ved `online`).
#[tauri::command]
async fn koe_prioritet(zone: Option<String>, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let zone = zone.map(|z| vaern::tjek_zone(&z)).transpose()?;
    tilstand.koe.prioriter(zone);
    Ok(())
}

/// Køens seneste tilstand (hændelser afspilles ikke igen, når siden skiftes). `null` før første runde.
#[tauri::command]
async fn koe_status(tilstand: State<'_, Tilstand>) -> Svar<Option<KoeStatus>> {
    Ok(tilstand.koe.status())
}

/// "Hent nu" i båndet, ⌘R/Ctrl+R og programmenuen: præcis som menulinjens "Hent nu" (`start_koe`), så en kø, der er
/// stoppet efter et afvist token, også startes igen. `koe_prioritet` (ved `online`) vækker kun en sovende kø.
#[tauri::command]
async fn hent_nu(tilstand: State<'_, Tilstand>, app: AppHandle) -> Svar<HentNu> {
    start_hentning(&tilstand, &app)
}

/// "Hent nu" og en daglig start, mens programmet kører: starter køen, hvis der er noget at hente, og den ikke kører.
fn start_hentning(tilstand: &Tilstand, app: &AppHandle) -> Svar<HentNu> {
    let nu = nu_ms();
    let alt_friskt = {
        let lager = laas(tilstand)?;
        let mut friskt = true;
        for z in lager.zoner_til()? {
            friskt &= koe::er_frisk(&lager.hentet(&z.id)?, nu);
        }
        friskt
    };
    let status = tilstand.koe.status();
    let udfald = koe::hent_udfald(status.as_ref(), tilstand.koe.koerer.load(Ordering::SeqCst), alt_friskt);
    if udfald == koe::HentUdfald::Startet {
        start_koe(app);
    }
    let (faerdige, i_alt) = status.map_or((0, 0), |s| (s.zoner_faerdige, s.zoner_i_alt));
    Ok(HentNu { udfald, faerdige, i_alt })
}

/// Svaret på "Hent nu": hvad der skete, og hvor langt en kørende runde er nået.
#[derive(Serialize)]
struct HentNu {
    udfald: koe::HentUdfald,
    faerdige: usize,
    i_alt: usize,
}

// ── Køens driver ───────────────────────────────────────────────────────────────────────────────────────────────────
/// Rundens henter: tokens for alle forbindelser, der kunne læses ved rundens start (V-KONTI.1). Tokenet slås op pr. zone (D3) ved hvert kald,
/// så en forbindelse, der er afvist eller glemt undervejs, ikke bruges igen.
struct KoeHenter<'a> {
    tilstand: &'a Tilstand,
    tokens: BTreeMap<i64, Hemmelig>,
}

impl KoeHenter<'_> {
    fn token(&self, zone: &str) -> Result<&Hemmelig, Fejl> {
        Henter::forbindelse(self, zone)
            .and_then(|f| self.tokens.get(&f))
            .ok_or(Fejl::IntetToken)
    }
}

impl Henter for KoeHenter<'_> {
    async fn zone(&self, zone: &str, paa_doegn: &mut (dyn FnMut(&str, Doegnudfald<'_>) + Send)) -> Result<usize, Fejl> {
        hent_zone(self.tilstand, self.token(zone)?, zone, paa_doegn).await
    }
    fn pause_til(&self) -> Option<i64> {
        self.tilstand.klient.pause_til_ms()
    }
    async fn ekstra(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
        hent_ekstra(self.tilstand, self.token(zone)?, zone, dag).await
    }
    async fn angreb(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
        hent_angreb(self.tilstand, self.token(zone)?, zone, dag).await
    }
    async fn laek(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
        hent_laek(self.tilstand, self.token(zone)?, zone, dag).await
    }
    async fn dns(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
        hent_dns(self.tilstand, self.token(zone)?, zone, dag).await
    }
    async fn dmarc(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
        hent_dmarc(self.tilstand, self.token(zone)?, zone, dag).await
    }
    async fn historik(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
        hent_historik(self.tilstand, self.token(zone)?, zone, fra, til).await
    }
    async fn unikke(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
        hent_unikke(self.tilstand, self.token(zone)?, zone, fra, til).await
    }
    async fn besoeg(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
        hent_besoeg(self.tilstand, self.token(zone)?, zone, fra, til).await
    }
    async fn ipklasse(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
        hent_ipklasse(self.tilstand, self.token(zone)?, zone, fra, til).await
    }
    async fn robot(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
        hent_robot(self.tilstand, self.token(zone)?, zone, fra, til).await
    }
    async fn side(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
        hent_side(self.tilstand, self.token(zone)?, zone, fra, til).await
    }
    async fn vaert(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
        hent_vaert(self.tilstand, self.token(zone)?, zone, fra, til).await
    }
    async fn vaert_fw(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
        hent_vaert_fw(self.tilstand, self.token(zone)?, zone, fra, til).await
    }
    async fn tilstand(&self, zone: &str) -> Result<(), Fejl> {
        hent_tilstand(self.tilstand, self.token(zone)?, zone).await
    }
    async fn fund(&self) {
        let afviste = self.tilstand.koe.afviste();
        for (id, t) in self.tokens.iter().filter(|(id, _)| !afviste.contains(*id)) {
            // En forbindelse, der er glemt undervejs i runden, hentes ikke (dens nøgler er slettet).
            let findes = laas(self.tilstand).is_ok_and(|l| l.forbindelse_ider().is_ok_and(|v| v.contains(id)));
            if findes {
                hent_fund(self.tilstand, *id, t).await;
            }
        }
    }
    fn forbindelse(&self, zone: &str) -> Option<i64> {
        let afviste = self.tilstand.koe.afviste();
        let ider = laas(self.tilstand).ok()?.forbindelser_for(zone).ok()?;
        ider.into_iter().find(|f| !afviste.contains(f) && self.tokens.contains_key(f))
    }
    fn forbindelser(&self) -> Vec<i64> {
        self.tokens.keys().copied().collect()
    }
}

/// Køen melder: hændelsen `hentekoe` til webvisningen og statuslinjen i menuen. Er runden færdig, vurderes
/// notifikationerne.
fn melder(app: &AppHandle, s: &KoeStatus) {
    let _ = app.emit("hentekoe", s);
    if let Some(a) = app.try_state::<agent::Agent>() {
        // Forbindelsernes navne til teksten ved en afvisning. `try_lock`: melderen må aldrig vente på lageret (en kalder kan holde det).
        if let Some(f) = app
            .try_state::<Tilstand>()
            .and_then(|t| t.lager.try_lock().ok().and_then(|l| l.forbindelser().ok()))
        {
            a.saet_forbindelser(f);
        }
        a.meld(s, nu_ms());
    }
    if s.tilstand == koe::Fase::Faerdig {
        vurder(app);
    }
}

/// Starter køen (eller vækker den, hvis den kører, også når den venter på nøgleringen): "Hent nu", et nyt token og
/// programstart. Den kører, til tokenet mangler, afvises eller glemmes.
fn start_koe(app: &AppHandle) {
    let t = app.state::<Tilstand>();
    t.koe.fortsaet();
    if t.koe.koerer.swap(true, Ordering::SeqCst) {
        t.koe.prioriter(None);
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let tilstand = app.state::<Tilstand>();
        let t: &Tilstand = &tilstand;
        let meld = &|s: &KoeStatus| melder(&app, s);
        let paa_gemt = &|zone: &str| {
            let _ = app.emit("ekstra", zone);
        };
        let runde = move || async move {
            let tokens = alle_tokens(t).await?;
            t.klient.ny_runde();
            let h = KoeHenter { tilstand: t, tokens };
            let slut = t.koe.runde(&t.lager, &h, meld, &nu_ms).await;
            if slut == koe::Slut::Faerdig && t.koe.ekstra_runde(&t.lager, &h, &nu_ms, koe::EKSTRA_TEMPO, paa_gemt).await {
                return Ok(koe::Slut::Vaekket);
            }
            Ok(slut)
        };
        // Ved hvert tik: "Hentet for N min. siden" i menuen.
        let paa_tik = || {
            if let Some(a) = app.try_state::<agent::Agent>() {
                a.vis(t.koe.status().as_ref(), nu_ms());
            }
        };
        t.koe.koer(runde, meld, &nu_ms, &paa_tik).await;
    });
}

/// Den daglige start, når programmet ikke kørte: lukker, når `dagligt::skal_afslutte` siger ja. Køen bedes om at stoppe først, så en
/// zone i gang bliver hentet færdig (højst `STOP_VENT`). Åbner brugeren vinduet undervejs, er det et almindeligt kørende program.
fn vagt_dagligt(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let t = app.state::<Tilstand>();
        let start = nu_ms();
        loop {
            tokio::time::sleep(dagligt::TJEK).await;
            if !t.dagligt.load(Ordering::SeqCst) {
                return;
            }
            let (koerer, fase) = (t.koe.koerer.load(Ordering::SeqCst), t.koe.status().map(|s| s.tilstand));
            if dagligt::skal_afslutte(koerer, fase, t.koe.hele_runder.load(Ordering::SeqCst), nu_ms() - start) {
                break;
            }
        }
        t.koe.stop();
        let _ = tokio::time::timeout(dagligt::STOP_VENT, t.koe.slut()).await;
        tokio::time::sleep(dagligt::EFTERSLAEB).await;
        if t.dagligt.load(Ordering::SeqCst) {
            app.exit(0);
        } else {
            start_koe(&app);
        }
    });
}

#[derive(Serialize)]
struct DataStatus {
    /// Databasefilens størrelse i bytes.
    bytes: u64,
    /// Ældste dag pr. art for de valgte domæner (`Store::aeldste`), `null`, hvis der ingen er endnu.
    trafik_fra: Option<String>,
    sikkerhed_fra: Option<String>,
    /// Ældste og nyeste dag med tal, uanset art.
    foerste: Option<String>,
    sidste: Option<String>,
    /// Domæner med tal.
    domaener: i64,
    /// Hvad "Nulstil historik" koster.
    nulstil: historik::NulstilTab,
    /// Cloudflares kvote, hvis den er meldt.
    kvote: Option<cf::KvoteUd>,
}

/// Indstillinger → Data: databasefilens størrelse og hvad den rummer. Kun læsning; intet netværk.
#[tauri::command]
async fn data_status(tilstand: State<'_, Tilstand>, app: AppHandle) -> Svar<DataStatus> {
    let sti = app.path().app_data_dir().map_err(|_| Fejl::Lager)?.join(DATABASE);
    let bytes = std::fs::metadata(sti).map_err(|_| Fejl::Lager)?.len();
    let lager = laas(&tilstand)?;
    let (a, (_, _, domaener)) = (lager.aeldste(&lager.valg()?)?, lager.data_status()?);
    Ok(DataStatus {
        bytes,
        foerste: a.trafik_fra.iter().chain(a.sikkerhed_fra.iter()).min().cloned(),
        nulstil: historik::nulstil_tab(&a, nu_ms()),
        trafik_fra: a.trafik_fra,
        sikkerhed_fra: a.sikkerhed_fra,
        sidste: a.sidste,
        domaener,
        kvote: tilstand.klient.kvote_ud(),
    })
}

/// "Nulstil historik": køen stoppes, og tallene slettes (`Store::nulstil_historik`). Køen startes først igen, når den gamle
/// løkke er slut: ellers ville en runde med den gamle plan køre videre og melde "færdig" på halve tal, og `vurder` kunne
/// sende en notifikation ud fra dem. Dage, en zone i gang gemmer efter sletningen, er gyldige og hentes ikke igen.
#[tauri::command]
async fn nulstil_historik(tilstand: State<'_, Tilstand>, app: AppHandle) -> Svar<()> {
    tilstand.koe.stop();
    let svar = laas(&tilstand).and_then(|mut l| l.nulstil_historik());
    tauri::async_runtime::spawn(async move {
        app.state::<Tilstand>().koe.slut().await;
        start_koe(&app);
    });
    svar
}

/// Er start ved login slået til? macOS: login-plisten findes (`ved_login`, som programmet selv skriver). Windows og Linux: pluginnet
/// (Run-nøglen / autostart-filen). Fejltekster (med stier) kasseres; webvisningen får kun koden `autostart`.
#[cfg(target_os = "macos")]
fn login_til(_app: &AppHandle) -> Result<bool, Fejl> {
    Ok(ved_login::er_til())
}

#[cfg(not(target_os = "macos"))]
fn login_til(app: &AppHandle) -> Result<bool, Fejl> {
    app.autolaunch().is_enabled().map_err(|_| Fejl::Autostart)
}

/// Slår start ved login til eller fra; et fravalg uden noget at fjerne er ikke en fejl.
#[cfg(target_os = "macos")]
fn login_saet(_app: &AppHandle, til: bool) -> Result<(), Fejl> {
    ved_login::saet(til).map_err(|_| Fejl::Autostart)
}

#[cfg(not(target_os = "macos"))]
fn login_saet(app: &AppHandle, til: bool) -> Result<(), Fejl> {
    let al = app.autolaunch();
    let svar = if til { al.enable() } else { al.disable() };
    svar.map_err(|_| Fejl::Autostart)
}

/// Start ved login: OS'et er sandheden (LaunchAgent-plist / Run-nøglen), så et fravalg i Systemindstillinger slår igennem.
#[tauri::command]
async fn autostart_hent(app: AppHandle) -> Svar<bool> {
    login_til(&app)
}

/// Slår start ved login og den daglige start til eller fra (én kontakt) og svarer med OS'ets tilstand bagefter. Begge forsøges, også hvis den ene fejler.
#[tauri::command]
async fn autostart_saet(til: bool, app: AppHandle) -> Svar<bool> {
    let login = login_saet(&app, til);
    let daglig = dagligt::saet(til);
    if login.is_err() || daglig.is_err() {
        return Err(Fejl::Autostart);
    }
    login_til(&app)
}

/// UI'ets sprog ("da"/"en") til menuen i menulinjen, programmenuen (macOS) og notifikationerne; gemt, så de er på det
/// sprog, også når programmet startede ved login.
#[tauri::command]
async fn sprog_saet(sprog: String, tilstand: State<'_, Tilstand>, app: AppHandle) -> Svar<()> {
    let s = agent::Sprog::fra(&sprog).ok_or(Fejl::Intern)?;
    laas(&tilstand)?.gem_indstilling(SPROG, s.kode())?;
    if let Some(a) = app.try_state::<agent::Agent>() {
        a.saet_sprog(s, tilstand.koe.status().as_ref(), nu_ms());
    }
    #[cfg(target_os = "macos")]
    if let Ok(menu) = programmenu::byg(&app, s) {
        let _ = app.set_menu(menu);
    }
    let _ = vis_handling(&app, s);
    Ok(())
}

// ── Notifikationer og "Siden sidst" ───────────────────────────────────────────────────────────────────
/// Det fælles domænevalg: "samlet" eller en zone. Fravalgt eller væk afgør rammen, som kender listen.
const DOMAENE_VALG: &str = "domaene_valg";
const SAMLET: &str = "samlet";

#[tauri::command]
async fn domaene_valg_hent(tilstand: State<'_, Tilstand>) -> Svar<String> {
    Ok(laas(&tilstand)?.indstilling(DOMAENE_VALG)?.unwrap_or_else(|| SAMLET.into()))
}

#[tauri::command]
async fn domaene_valg_saet(valg: String, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let v = if valg == SAMLET { valg } else { vaern::tjek_zone(&valg)? };
    laas(&tilstand)?.gem_indstilling(DOMAENE_VALG, &v)
}

/// Tidsskiftene fra webvisningen: (fra ms, minutter øst for UTC). Højst 8: to om året for 13 måneder og lidt til.
#[tauri::command]
async fn tidszone_saet(skift: Vec<(i64, i64)>, tilstand: State<'_, Tilstand>) -> Svar<()> {
    if skift.len() > 8 || skift.iter().any(|x| x.1.abs() > 14 * 60) {
        return Err(Fejl::Intern);
    }
    let vaerdi = serde_json::to_string(&skift).map_err(|_| Fejl::Intern)?;
    laas(&tilstand)?.gem_indstilling(notify::TIDSZONE, &vaerdi)
}

/// Datoformatet ("sprog" | "system" | "iso") og systemets mønster, som kun webvisningen kan læse. Bruges af notifikationerne.
#[tauri::command]
async fn dato_format_saet(format: String, moenster: String, tilstand: State<'_, Tilstand>) -> Svar<()> {
    if !matches!(format.as_str(), "sprog" | "system" | "iso") || moenster.len() > 32 {
        return Err(Fejl::Intern);
    }
    let mut lager = laas(&tilstand)?;
    lager.gem_indstilling(notify::DATO_FORMAT, &format)?;
    lager.gem_indstilling(notify::DATO_MOENSTER, &moenster)
}

#[derive(Serialize)]
struct NotifikationValg {
    niveau: &'static str,
    roed: bool,
    rekognoscering: bool,
    /// Zone → "alt" | "roed" | "fra"; domæner uden valg følger forudindstillingen.
    zoner: HashMap<String, &'static str>,
}

/// Forudindstillingen, "Giv besked ved rødt" og valget pr. domæne.
#[tauri::command]
async fn notifikation_hent(tilstand: State<'_, Tilstand>) -> Svar<NotifikationValg> {
    let lager = laas(&tilstand)?;
    let i = notify::indst(&lager)?;
    Ok(NotifikationValg {
        niveau: i.niveau.kode(),
        roed: i.roed,
        rekognoscering: i.rekognoscering,
        zoner: i.zoner.into_iter().map(|(z, v)| (z, v.kode())).collect(),
    })
}

/// Gemmer forudindstillingen ("rolig" | "standard" | "alt") og "Giv besked ved rødt".
#[tauri::command]
async fn notifikation_saet(niveau: String, roed: bool, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let n = notify::Niveau::fra(&niveau).ok_or(Fejl::Intern)?;
    let mut lager = laas(&tilstand)?;
    lager.gem_indstilling(notify::NIVEAU, n.kode())?;
    lager.gem_indstilling(notify::ROED, if roed { "1" } else { "0" })
}

/// Giv besked ved "Ligner rekognoscering" (fra som standard).
#[tauri::command]
async fn notifikation_rekog_saet(til: bool, tilstand: State<'_, Tilstand>) -> Svar<()> {
    laas(&tilstand)?.gem_indstilling(notify::REKOG, if til { "1" } else { "0" })
}

/// Gemmer ét domænes valg ("standard" | "alt" | "roed" | "fra").
#[tauri::command]
async fn notifikation_zone_saet(zone: String, valg: String, tilstand: State<'_, Tilstand>) -> Svar<()> {
    let zone = vaern::tjek_zone(&zone)?;
    let v = notify::ZoneValg::fra(&valg).ok_or(Fejl::Intern)?;
    laas(&tilstand)?.gem_notifikation_zone(&zone, (v != notify::ZoneValg::Standard).then(|| v.kode()))
}

#[derive(Serialize)]
struct SidenSidst {
    /// Skal kortet vises nu (menuen/notifikationen bad om det, eller forudindstillingen siger det)?
    vis: bool,
    /// Hvornår kortet sidst blev vist (ISO, UTC).
    vist: Option<String>,
    /// Røde/gule domæner, der er nye siden da.
    nye: Vec<String>,
    raekker: Vec<overblik::Raekke1>,
    /// AI-robotter, der er dukket op siden sidst (linjer i visningen; status og kolonner er uændrede).
    nye_robotter: Vec<robotter::NyRobot>,
}

/// "Siden sidst": overblikkets rækker, hvornår kortet sidst blev vist, og hvad der er nyt siden. Kun fra cachen.
#[tauri::command]
async fn siden_sidst(tilstand: State<'_, Tilstand>) -> Svar<SidenSidst> {
    let nu = nu_ms();
    let lager = laas(&tilstand)?;
    let raekker = byg_overblik(&lager, nu)?;
    let niveau = notify::indst(&lager)?.niveau;
    let vist = lager.indstilling(notify::VIST)?;
    let nye = notify::nye(&raekker, lager.indstilling(notify::VIST_AFTRYK)?.as_deref(), nu);
    let nye_robotter = robotter::nye_robotter(&lager, vist.as_deref(), nu)?;
    let bedt = tilstand.siden_sidst.swap(false, Ordering::SeqCst);
    Ok(SidenSidst {
        vis: bedt || notify::vis_selv(niveau, vist.as_deref(), &raekker, nu, &notify::tidszone(&lager)?),
        vist,
        nye,
        raekker,
        nye_robotter,
    })
}

/// Kortet er vist: tidspunktet og det, der stod på det, gemmes (til "du så kortet sidst …" og "ny").
#[tauri::command]
async fn siden_sidst_set(tilstand: State<'_, Tilstand>, app: AppHandle) -> Svar<()> {
    let nu = nu_ms();
    let mut lager = laas(&tilstand)?;
    let aftryk = notify::aftryk(&byg_overblik(&lager, nu)?, nu);
    lager.gem_indstilling(notify::VIST, &vaern::iso(nu))?;
    lager.gem_indstilling(notify::VIST_AFTRYK, &aftryk)?;
    if let Some(a) = app.try_state::<agent::Agent>() {
        a.saet_nye(0, None);
    }
    Ok(())
}

/// Handlingstypen med knappen "Vis"/"Show": uden en handling melder macOS-stien (NSUserNotification) ikke klik tilbage
///. Registreres igen, når sproget skiftes (en type med samme id erstatter den forrige).
const VIS_TYPE: &str = "huginn-vis";
const VIS: &str = "vis";

fn vis_handling(app: &AppHandle, sprog: agent::Sprog) -> Result<(), tauri_plugin_notification::Error> {
    use tauri_plugin_notification::{Action, ActionType, NotificationExt};
    let titel = match sprog {
        agent::Sprog::Da => "Vis",
        agent::Sprog::En => "Show",
    };
    app.notification().register_action_types(vec![ActionType::builder(VIS_TYPE)
        .actions(vec![Action::builder(VIS, titel).build()])
        .build()])
}

/// Klik på notifikationen (`tap`) eller på "Vis" åbner vinduet på "Siden sidst". Handleren kører på en baggrundstråd;
/// vinduet åbnes på hovedtråden.
fn notifikation_klik(app: &AppHandle) -> Result<(), tauri_plugin_notification::Error> {
    use tauri_plugin_notification::NotificationExt;
    let h = app.clone();
    app.notification().on_action(move |handling| {
        if matches!(handling.action_id(), "tap" | VIS) {
            let a = h.clone();
            let _ = h.run_on_main_thread(move || vis_siden_sidst(&a));
        }
    })
}

/// Menuen "Siden sidst" og klik på en notifikation: vinduet åbnes på visningen "Siden sidst". Et nyt vindue spørger selv
/// (`siden_sidst` → `vis`); et åbent får hændelsen `siden_sidst`, og rammen skifter visning (uden at genindlæse).
fn vis_siden_sidst(app: &AppHandle) {
    app.state::<Tilstand>().siden_sidst.store(true, Ordering::SeqCst);
    let _ = aabn_vindue(app);
    let _ = app.emit("siden_sidst", ());
}

fn sprog_fra(lager: &Store) -> agent::Sprog {
    lager
        .indstilling(SPROG)
        .ok()
        .flatten()
        .as_deref()
        .and_then(agent::Sprog::fra)
        .unwrap_or(agent::Sprog::Da)
}

/// Beslutter og noterer: det meldte, og hvor langt mærkerne er vurderet. Også en runde uden besked rykker det vurderede frem, så et senere valg ikke melder gammelt nyt.
fn naeste_besked(lager: &mut Store, rk: &[overblik::Raekke1], ind: &notify::Indst, nu: i64) -> Result<Option<notify::Besked>, Fejl> {
    let b = notify::beslut(rk, ind, &lager.meldt()?, nu);
    if let Some(b) = &b {
        let glem_foer = vaern::dag_noegle(nu - notify::MELDT_DOEGN * vaern::DOEGN_MS);
        lager.gem_meldt(&b.meldte(), &vaern::iso(nu), &glem_foer)?;
    }
    lager.gem_vurderet(&notify::vurderede(rk, ind, &lager.meldt()?))?;
    Ok(b)
}

/// Er der noget at sige? Ved start (fra cachen alene; `beslut` melder kun domæner med friske tal) og efter hver færdig
/// runde. Det meldte noteres, før notifikationen sendes, så den samme aldrig kommer to gange. Det kan ikke flyttes til efter `show()`: pluginnet starter en tråd og svarer `Ok`,
/// før notifikationen er vist (tauri-plugin-notification 2.8.0, desktop.rs), så et svar ville ikke sige noget om visningen. Venter køen på nøgleringen,
/// kommer der ingen runde og derfor ingen notifikation (kun menulinjen).
fn vurder(app: &AppHandle) {
    use tauri_plugin_notification::NotificationExt;
    let t = app.state::<Tilstand>();
    let nu = nu_ms();
    let Ok(mut lager) = laas(&t) else { return };
    let mut besked = || -> Result<Option<(String, String)>, Fejl> {
        let rk = byg_overblik(&lager, nu)?;
        if let Some(a) = app.try_state::<agent::Agent>() {
            a.saet_maerker(agent::Maerker::af(&rk));
            let vist = lager.indstilling(notify::VIST)?;
            let nye = notify::nye(&rk, lager.indstilling(notify::VIST_AFTRYK)?.as_deref(), nu).len()
                + robotter::nye_robotter(&lager, vist.as_deref(), nu)?.len();
            let tz = notify::tidszone(&lager)?;
            a.saet_nye(nye, vist.as_deref().and_then(vaern::parse_iso).map(|ms| tz.kl(ms)));
        }
        let ind = notify::indst(&lager)?;
        let Some(b) = naeste_besked(&mut lager, &rk, &ind, nu)? else {
            return Ok(None);
        };
        if b.rolig {
            lager.gem_indstilling(notify::ROLIG_SIDST, &ind.tz.dag(nu))?;
        }
        Ok(Some(notify::tekst(&b, sprog_fra(&lager), &notify::dato_format(&lager)?)))
    };
    let besked = besked();
    drop(lager);
    if let Ok(Some((titel, tekst))) = besked {
        let _ = app
            .notification()
            .builder()
            .title(titel)
            .body(tekst)
            .action_type_id(VIS_TYPE)
            .show();
    }
}

// ── Vinduet, menuen og start ved login ─────────────────────────────────────────────────────────────────────────────
/// Start ved login giver programmet dette argument (i login-plisten fra `ved_login` / Run-nøglen fra pluginnet).
const VED_LOGIN: &str = "--autostart";
const SPROG: &str = "sprog";
/// Sat, når start ved login er slået til første gang; derefter er brugerens valg (i OS'et) sandheden.
const AUTOSTART_STANDARD: &str = "autostart_standard";
/// Sat, når den daglige start er afgjort første gang, også på en installation, der er ældre end den.
const DAGLIGT_STANDARD: &str = "dagligt_standard";

/// Startede programmet ved login? Første argument er programmets sti. Ren funktion.
fn startet_ved_login(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().skip(1).any(|a| a == VED_LOGIN)
}

/// Startede programmet uden vindue og uden Dock-ikon? Ved login og ved den daglige start. Ren funktion.
fn uden_vindue(args: &[String]) -> bool {
    startet_ved_login(args.iter().cloned()) || dagligt::startet_dagligt(args.iter().cloned())
}

/// Skal start ved login slås til nu? Kun i en bundlet build (ikke `tauri dev`) og kun den første gang. Ren funktion.
fn skal_slaa_autostart_til(dev: bool, anvendt: Option<&str>) -> bool {
    !dev && anvendt.is_none()
}

fn autostart_standard(app: &AppHandle) -> Result<(), Fejl> {
    let t = app.state::<Tilstand>();
    let anvendt = laas(&t)?.indstilling(AUTOSTART_STANDARD)?;
    if skal_slaa_autostart_til(tauri::is_dev(), anvendt.as_deref()) {
        login_saet(app, true)?;
        laas(&t)?.gem_indstilling(AUTOSTART_STANDARD, "1")?;
    }
    // Den daglige start følger login-agenten og har sin egen markør, så installationer fra før den også får den. Én gang: et fravalg står ved magt.
    let anvendt = laas(&t)?.indstilling(DAGLIGT_STANDARD)?;
    let login_til = login_til(app)?;
    let Some(slaa_til) = dagligt::standard(tauri::is_dev(), anvendt.as_deref(), login_til) else {
        return Ok(());
    };
    if slaa_til {
        dagligt::saet(true).map_err(|_| Fejl::Autostart)?;
    }
    let mut lager = laas(&t)?;
    lager.gem_indstilling(DAGLIGT_STANDARD, "1")
}

/// Viser vinduet: det eksisterende, eller et nyt fra konfigurationen. Det bygges her ("create": false i tauri.conf.json),
/// så vinduets titel følger sidens <title> ("Angreb · 3 domæner · …"); det gør Tauri ikke af sig selv. Init-scriptet giver
/// siden platformen (`<html data-platform>`). På macOS kommer Dock-ikonet frem først (det er væk, mens programmet kun står
/// i menulinjen).
fn aabn_vindue(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(t) = app.try_state::<Tilstand>() {
        t.dagligt.store(false, Ordering::SeqCst);
    }
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Regular)?;
    let vindue = match app.get_webview_window("main") {
        Some(v) => v,
        None => {
            let konfig = app
                .config()
                .app
                .windows
                .first()
                .cloned()
                .ok_or("vinduet mangler i tauri.conf.json")?;
            tauri::WebviewWindowBuilder::from_config(app, &konfig)?
                .initialization_script(programmenu::platform_skript(std::env::consts::OS))
                .on_document_title_changed(|w, titel| {
                    let _ = w.set_title(&titel);
                })
                .build()?
        }
    };
    vindue.show()?;
    vindue.set_focus()?;
    Ok(())
}

/// Programmenuen (macOS): en visning → hændelsen `visning` til rammen (vinduet åbnes først); Tilbage/Frem → hændelsen
/// `historik`; tekststørrelsen → hændelsen `tekst`; "Hent nu" → køen. Den globale menuhåndtering får også menulinjeikonets punkter; dem kender
/// `programmenu::handling` ikke.
fn paa_programmenu(app: &AppHandle, id: &str) {
    match programmenu::handling(id) {
        Some(programmenu::Handling::Visning(v)) => {
            let _ = aabn_vindue(app);
            let _ = app.emit("visning", v);
        }
        Some(programmenu::Handling::Historik(retning)) => {
            let _ = app.emit("historik", retning);
        }
        Some(programmenu::Handling::Tekst(trin)) => {
            let _ = app.emit("tekst", trin);
        }
        // Med et vindue går ⌘R samme vej som knappen, så brugeren ser udfaldet.
        Some(programmenu::Handling::HentNu) if app.get_webview_window("main").is_none() || app.emit("hent_nu", ()).is_err() => {
            start_koe(app)
        }
        Some(programmenu::Handling::HentNu) | None => {}
    }
}

fn paa_menu(app: &AppHandle, id: &str) {
    match id {
        agent::AABN => {
            let _ = aabn_vindue(app);
        }
        agent::SIDEN => vis_siden_sidst(app),
        agent::STATUS => {
            let _ = aabn_vindue(app);
            let _ = app.emit("visning", "overblik");
        }
        agent::HENT => start_koe(app),
        agent::AFSLUT => app.exit(0),
        _ => {}
    }
}

static KORTDATA: LazyLock<Value> = LazyLock::new(|| {
    json!({
        "verden": serde_json::from_str::<Value>(include_str!("../../ui/assets/verden.json")).unwrap_or(Value::Null),
        "colo": serde_json::from_str::<Value>(include_str!("../../ui/assets/colo.json")).unwrap_or(Value::Null),
        "tidszoner": serde_json::from_str::<Value>(include_str!("../../ui/assets/tidszoner.json")).unwrap_or(Value::Null),
    })
});

/// Landkonturer og datacentre til kortet og tidszonerne til første valg af land (indlejret ved bygning; webvisningen henter intet selv).
#[tauri::command]
fn kortdata() -> Value {
    KORTDATA.clone()
}

/// CVE-numre: præcis "CVE-", fire cifre, "-", fire til syv cifre (kun ASCII-cifre; ikke Unicodes `\d`).
static CVE_ID: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new("^CVE-[0-9]{4}-[0-9]{4,7}$").expect("CVE-mønster"));

/// Et CVE-nummer → cve.org-adressen; alt andet afvises (`Fejl::Intern`). Adressen bygges kun her. Ren funktion.
fn cve_adresse(id: &str) -> Result<String, Fejl> {
    if !CVE_ID.is_match(id) {
        return Err(Fejl::Intern);
    }
    Ok(format!("https://www.cve.org/CVERecord?id={id}"))
}

/// Åbner adressen i systemets standardbrowser: et fast program med adressen som ét argument (ingen shell), og der ventes
/// på, at programmet har givet adressen videre. Programmet selv kontakter ikke cve.org.
fn aabn_i_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut kommando = std::process::Command::new("/usr/bin/open");
    #[cfg(target_os = "windows")]
    let mut kommando = std::process::Command::new("explorer.exe");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut kommando = std::process::Command::new("xdg-open");
    kommando.arg(url).status().map(|_| ())
}

/// Et klik på et CVE-nummer: åbner CVE'ens side på cve.org i brugerens browser. Kun gyldige CVE-numre.
#[tauri::command]
async fn aabn_cve(id: String) -> Svar<()> {
    let url = cve_adresse(&id)?;
    tauri::async_runtime::spawn_blocking(move || aabn_i_browser(&url))
        .await
        .map_err(|_| Fejl::Intern)?
        .map_err(|_| Fejl::Intern)
}

/// Kildelinjen under Radar-tal (CC BY-NC 4.0 kræver kilden): Radars forside i brugerens browser. Adressen står kun her.
#[tauri::command]
async fn aabn_radar() -> Svar<()> {
    tauri::async_runtime::spawn_blocking(|| aabn_i_browser("https://radar.cloudflare.com/"))
        .await
        .map_err(|_| Fejl::Intern)?
        .map_err(|_| Fejl::Intern)
}

/// Cloudflares template-URL til et bruger-API Token med navn og de to rettigheder udfyldt på forhånd: `[{"key":"analytics","type":"read"},{"key":"zone","type":"read"}]`
/// (developers.cloudflare.com/fundamentals/api/how-to/account-owned-token-template/). Brugeren skal selv oprette tokenet.
const API_TOKEN_SIDE: &str = "https://dash.cloudflare.com/profile/api-tokens?permissionGroupKeys=%5B%7B%22key%22%3A%22analytics%22%2C%22type%22%3A%22read%22%7D%2C%7B%22key%22%3A%22zone%22%2C%22type%22%3A%22read%22%7D%5D&accountId=%2A&zoneId=all&name=Huginn%20and%20Muninn";

/// "Opret API Token hos Cloudflare": åbner siden i brugerens browser. Et fast mål, og webvisningen kan ikke vælge et andet.
#[tauri::command]
async fn aabn_api_token() -> Svar<()> {
    tauri::async_runtime::spawn_blocking(|| aabn_i_browser(API_TOKEN_SIDE))
        .await
        .map_err(|_| Fejl::Intern)?
        .map_err(|_| Fejl::Intern)
}

/// Manualen på wi24rd.com: forsiden på hvert sprog. Programmet henter intet derfra; adressen åbnes i brugerens browser.
const VEJLEDNING_DA: &str = "https://wi24rd.com/dokumentation/huginn-muninn/";
const VEJLEDNING_EN: &str = "https://wi24rd.com/en/documentation/huginn-muninn/";
/// Adressen på manualens forside for sproget. Et andet sprog end `en` er dansk, som overalt i programmet. Sproget vælger kun mellem de to faste adresser.
fn vejledning_adresse(sprog: &str) -> &'static str {
    if sprog == "en" {
        VEJLEDNING_EN
    } else {
        VEJLEDNING_DA
    }
}

/// "Åbn vejledningen": manualens forside i brugerens browser. Faste adresser; webvisningen vælger kun sproget.
#[tauri::command]
async fn aabn_vejledning(sprog: String) -> Svar<()> {
    let url = vejledning_adresse(&sprog);
    tauri::async_runtime::spawn_blocking(move || aabn_i_browser(url))
        .await
        .map_err(|_| Fejl::Intern)?
        .map_err(|_| Fejl::Intern)
}

/// Webvisningen må kun vise appens egne sider: Tauris egen protokol, og under `tauri dev` den indbyggede dev-server.
fn tilladt_adresse(url: &Url) -> bool {
    let app = matches!(
        (url.scheme(), url.host_str()),
        ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost"))
    );
    let dev_server = cfg!(dev) && url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
    app || dev_server
}

fn navigationsvagt<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("navigationsvagt")
        .on_navigation(|_, url| tilladt_adresse(url))
        .build()
}

pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    // Slået fra, men launchd har stadig jobbet (det var det, der kørte, da kontakten blev slået fra, eller filen er slettet i hånden): intet at gøre.
    if dagligt::startet_dagligt(args.iter().cloned()) && !dagligt::er_til() {
        return;
    }
    let setup_args = args.clone();
    let builder = tauri::Builder::default()
        // Skal være det første plugin: en anden start overdrager til denne (som viser vinduet eller, ved en daglig start, henter)
        // og afslutter, før dens eget setup åbner SQLite (plugins sættes op i `Builder::build`, vores setup først ved `RunEvent::Ready`).
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            match dagligt::anden_start(args) {
                dagligt::AndenStart::Vindue => {
                    let _ = aabn_vindue(app);
                }
                dagligt::AndenStart::Hent => {
                    if let Some(t) = app.try_state::<Tilstand>() {
                        let _ = start_hentning(&t, app);
                    }
                }
            }
        }));
    // Windows og Linux: Run-nøglen / autostart-filen fra pluginnet, uændret. macOS: login-plisten skrives af `ved_login`.
    #[cfg(not(target_os = "macos"))]
    let builder = builder.plugin(
        tauri_plugin_autostart::Builder::new()
            .app_name("com.wi24rd.huginn-muninn")
            .args([VED_LOGIN])
            .build(),
    );
    let app = builder
        .plugin(navigationsvagt())
        .plugin(tauri_plugin_notification::init())
        .on_menu_event(|app, ev| paa_programmenu(app, ev.id().as_ref()))
        .setup(move |app| {
            let mappe = app.path().app_data_dir()?;
            std::fs::create_dir_all(&mappe)?;
            let mut lager = Store::aaben(&mappe.join(DATABASE))?;
            // V-SAMPLE.1: én gang hentes de færdige døgn i hentevinduet igen med Cloudflares estimat. Fejler det, prøves der igen ved næste start.
            let _ = vaern::fw_vaegt_en_gang(&mut lager, nu_ms());
            // V-BESOEG.1: én gang hentes unikke besøgende for de døgn i året, der allerede er hentet. Fejler det, prøves der igen ved næste start.
            let _ = lager.unikke_en_gang(&vaern::dag_noegle(nu_ms()));
            // V-BESOEG.2: én gang hentes IP-klasserne (`h_ipklasse`) for de døgn i året, der allerede er hentet. Kun den art skrives.
            let _ = lager.ipklasse_en_gang(&vaern::dag_noegle(nu_ms()));
            // V-BESOEG.2 FIX 1: én gang hentes sidernes døgn i hentevinduet igen, nu med `b_side_bytes`. Ét kald pr. domæne.
            let _ = lager.side_bytes_en_gang(
                &vaern::dag_noegle(nu_ms()),
                &vaern::dag_noegle(nu_ms() - besoeg::BESOEG_DAGE * vaern::DOEGN_MS),
            );
            // V-SUBDOM.1: én gang hentes sidernes døgn i hentevinduet igen, nu med værten i nøglen. Ét kald pr. domæne.
            let _ = lager.side_vaert_en_gang(
                &vaern::dag_noegle(nu_ms()),
                &vaern::dag_noegle(nu_ms() - besoeg::BESOEG_DAGE * vaern::DOEGN_MS),
            );
            // V-SUBDOM.1 FIX 1: én gang hentes trafikkens døgn pr. vært i hentevinduet igen, nu med lande og robotter pr. vært. Ét kald pr. domæne.
            let _ = lager.vaert_land_en_gang(&vaern::dag_noegle(nu_ms() - besoeg::BESOEG_DAGE * vaern::DOEGN_MS));
            let sprog = sprog_fra(&lager);
            app.manage(Tilstand {
                lager: Mutex::new(lager),
                klient: Klient::ny()?,
                laase: Hentelaase::default(),
                koe: Koe::default(),
                udenfor: Mutex::default(),
                token: token::Huske::default(),
                siden_sidst: AtomicBool::new(false),
                uden_asn: Mutex::default(),
                uden_vaert: Mutex::default(),
                laek_uden_filter: AtomicBool::new(false),
                dagligt: AtomicBool::new(dagligt::startet_dagligt(setup_args.iter().cloned())),
                zoner_hentet: Mutex::default(),
            });
            app.manage(agent::Agent::byg(app.handle(), sprog, paa_menu)?);
            #[cfg(target_os = "macos")]
            app.set_menu(programmenu::byg(app.handle(), sprog)?)?;
            vis_handling(app.handle(), sprog)?;
            notifikation_klik(app.handle())?;
            // Ved start: notifikation fra cachen, hvis tallene er friske nok (beslutning 6); ellers efter første runde.
            vurder(app.handle());
            // Er programmet flyttet, peger login- og dagligt-plisten det gamle sted hen: de skrives igen (aldrig under `tauri dev`).
            #[cfg(target_os = "macos")]
            if !tauri::is_dev() {
                let _ = ved_login::ret_stier();
            }
            // Første start af en bundlet build: start ved login slås til. Lykkes det ikke, prøves der igen ved næste start.
            let _ = autostart_standard(app.handle());
            // Køen starter, hvis der er et token (ellers melder den "intet token" og startes af forbindelse_tilfoej).
            start_koe(app.handle());
            start_radar(app.handle());
            // Daglig start: lukker selv igen, når køen er færdig.
            if dagligt::startet_dagligt(setup_args.iter().cloned()) {
                vagt_dagligt(app.handle());
            }
            // Ved login og ved den daglige start: intet vindue, kun ikonet i menulinjen (politikken Accessory er sat, før programmet kørte).
            if !uden_vindue(&setup_args) {
                aabn_vindue(app.handle())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            token_status,
            forbindelser_hent,
            forbindelse_tilfoej,
            forbindelse_skift,
            forbindelse_omdoeb,
            forbindelse_glem,
            zones,
            zones_cache,
            vaern_cache,
            vaern_oversigt,
            kortdata,
            overblik,
            valg_hent,
            domaener_hent,
            domaener_saet,
            domaene_afvis,
            vaern_samlet_cache,
            dyk,
            robotter,
            besoegende,
            subdomaener,
            login,
            login_stier,
            login_sti_saet,
            login_sti_fjern,
            trafik_dag,
            teknologi_hent,
            teknologi_saet,
            teknologi_alle,
            teknologi_side,
            teknologi_vis_saet,
            teknologi_kolonne,
            teknologi_tilfoej,
            email_oversigt,
            sti200_oversigt,
            sikkerhedsfund_oversigt,
            aabn_sikkerhedsfund,
            huller,
            hul_luk,
            land_hent,
            land_saet,
            domaene_valg_hent,
            domaene_valg_saet,
            dato_format_saet,
            notifikation_rekog_saet,
            tidszone_saet,
            teknologi_oversigt,
            teknologi_nulstil,
            vurdering,
            aabn_api_token,
            aabn_vejledning,
            land_visning,
            radar_opdater,
            aabn_radar,
            koe_prioritet,
            koe_status,
            hent_nu,
            data_status,
            nulstil_historik,
            aabn_cve,
            autostart_hent,
            autostart_saet,
            sprog_saet,
            notifikation_hent,
            notifikation_saet,
            notifikation_zone_saet,
            siden_sidst,
            siden_sidst_set
        ])
        .build(tauri::generate_context!())
        .expect("Huginn and Muninn kunne ikke starte");
    // Startet ved login eller af den daglige start: intet Dock-ikon fra starten.
    #[cfg(target_os = "macos")]
    let app = {
        let mut app = app;
        if uden_vindue(&args) {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
        app
    };
    app.run(|app, ev| match ev {
        // Det sidste vindue er lukket (rødt kryds, ⌘W): programmet bliver i menulinjen, og Dock-ikonet forsvinder.
        // "Afslut" i menuen (app.exit: code Some) og ⌘Q (RunEvent::Exit) afslutter.
        RunEvent::ExitRequested { code: None, api, .. } => {
            api.prevent_exit();
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
        // Klik på Dock-ikonet.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => {
            let _ = aabn_vindue(app);
        }
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_kun_til_appens_egne_sider() {
        for ok in [
            "tauri://localhost/index.html",
            "tauri://localhost/index.html#vaernet?zone=a&navn=a.wi24rd.com",
            "http://tauri.localhost/index.html",
        ] {
            assert!(tilladt_adresse(&Url::parse(ok).unwrap()), "{ok}");
        }
        for nej in [
            "https://www.cve.org/CVERecord?id=CVE-2025-55182",
            "https://wi24rd.com/",
            "https://api.cloudflare.com/",
            "file:///etc/passwd",
            "tauri://evil/",
        ] {
            assert!(!tilladt_adresse(&Url::parse(nej).unwrap()), "{nej}");
        }
        // Den indbyggede dev-server (tauri dev) kun når tauri-build har sat cfg `dev`.
        assert_eq!(tilladt_adresse(&Url::parse("http://localhost:1430/index.html").unwrap()), cfg!(dev));
        assert!(!tilladt_adresse(&Url::parse("https://localhost:1430/").unwrap()));
    }

    /// F3: to hentninger af samme zone kører aldrig samtidig; to forskellige zoner gør gerne.
    #[tokio::test]
    async fn hentelaas_pr_zone() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let laase = Hentelaase::default();
        let (i_gang, flest) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let arbejde = || async {
            let n = i_gang.fetch_add(1, Ordering::SeqCst) + 1;
            flest.fetch_max(n, Ordering::SeqCst);
            for _ in 0..3 {
                tokio::task::yield_now().await;
            }
            i_gang.fetch_sub(1, Ordering::SeqCst);
        };
        futures_util::future::join3(laase.koer("a", arbejde()), laase.koer("a", arbejde()), laase.koer("a", arbejde())).await;
        assert_eq!(flest.load(Ordering::SeqCst), 1, "samme zone: én ad gangen");
        flest.store(0, Ordering::SeqCst);
        futures_util::future::join(laase.koer("a", arbejde()), laase.koer("b", arbejde())).await;
        assert_eq!(flest.load(Ordering::SeqCst), 2, "forskellige zoner blokerer ikke hinanden");
        flest.store(0, Ordering::SeqCst);
        futures_util::future::join(laase.koer(ZONELISTEN, arbejde()), laase.koer(ZONELISTEN, arbejde())).await;
        assert_eq!(flest.load(Ordering::SeqCst), 1, "zonelisten har sin egen lås");
        assert!(vaern::tjek_zone(ZONELISTEN).is_err(), "nøglen kan ikke være et zone-id");
        assert!(vaern::tjek_zone(NOEGLERINGEN).is_err(), "nøgleringens nøgle heller ikke");
        assert!(vaern::tjek_zone(RADAR).is_err(), "Radars nøgle heller ikke");
    }

    fn zone(id: &str, navn: &str) -> Zone {
        Zone {
            id: id.into(),
            navn: navn.into(),
        }
    }

    /// T1 og D2: et domæne, der ikke er valgt, er væk fra rækkerne, som Overblik, Siden sidst og notifikationerne deler. En ny zone er ikke valgt.
    #[test]
    fn kun_valgte_domaener_er_med() {
        const NU: i64 = 1_793_693_280_000;
        let mut s = Store::i_hukommelse().unwrap();
        let dag = vaern::dag_noegle(NU - vaern::DOEGN_MS);
        s.gem_zoner(1, &[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t")
            .unwrap();
        let rk = [(
            "fw".to_string(),
            ["R", "US", "A", "/", "CPH", "GET", "firewallManaged", "block"].join("\t"),
            900,
        )];
        for z in ["z1", "z2"] {
            s.gem_doegn(z, &dag, &rk, &vaern::iso(NU)).unwrap();
        }
        let navne = |s: &Store| byg_overblik(s, NU).unwrap().into_iter().map(|r| r.navn).collect::<Vec<_>>();
        let meldes = |s: &Store| {
            notify::beslut(&byg_overblik(s, NU).unwrap(), &notify::indst(s).unwrap(), &s.meldt().unwrap(), NU).map(|b| b.meldte().len())
        };
        assert!(navne(&s).is_empty(), "intet er valgt fra start");
        assert_eq!(meldes(&s), None, "og intet giver en notifikation");
        s.gem_valgte(&["z1".into(), "z2".into()]).unwrap();
        assert_eq!(navne(&s), ["a.wi24rd.com", "b.wi24rd.com"]);
        assert_eq!(meldes(&s), Some(2));
        s.gem_valgte(&["z1".into()]).unwrap();
        assert_eq!(navne(&s), ["a.wi24rd.com"]);
        assert_eq!(meldes(&s), Some(1), "det fravalgte giver ingen notifikation");
        assert_eq!(s.raekker("z2", &dag).unwrap().len(), 1, "tallene bliver liggende");
        s.gem_zoner(
            1,
            &[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com"), zone("z3", "c.wi24rd.com")],
            "t",
        )
        .unwrap();
        assert_eq!(s.valg().unwrap(), ["z1"], "T3: en ny zone er ikke valgt");
        assert_eq!(meldes(&s), Some(1), "og giver ingen notifikation");
        let ud = domaener_ud(
            s.zoner().unwrap().0,
            &s.valgte().unwrap(),
            &s.nye().unwrap(),
            &s.ejere(&HashSet::new()).unwrap(),
            &s.seere().unwrap(),
        );
        assert_eq!(
            ud.iter().map(|d| (d.til, d.ny, d.forbindelse)).collect::<Vec<_>>(),
            [(true, false, Some(1)), (false, false, Some(1)), (false, true, Some(1))],
            "V-KONTI.1: hvert domæne har sin forbindelse"
        );
    }

    /// D7 er udløbet: `token_set` og `token_delete` er væk. Det, de gjorde med én forbindelse, gør `forbindelse_tilfoej` og `forbindelse_glem` nu: den første
    /// forbindelse er id 1 (den oprindelige post i nøgleringen), og uden forbindelser læses den oprindelige post (`[1]`). Glem af den sidste (`glem_alle`:
    /// forbindelse for forbindelse, så alle) efterlader lageret som `slet_zoner` i 1.0.0: ingen forbindelser, zoner, valg eller notifikationsvalg, men tallene og
    /// indstillingerne. Glem af én af to rører ikke den anden.
    #[test]
    fn forbindelse_tilfoej_og_glem_virker_som_foer_med_een_forbindelse() {
        let mut s = Store::i_hukommelse().unwrap();
        assert_eq!(
            ider_eller_1(s.forbindelse_ider().unwrap()),
            [1],
            "hullet: den oprindelige post læses"
        );
        assert_eq!(s.ny_forbindelse("t").unwrap(), 1, "forbindelse_tilfoej: den første er 1");
        assert_eq!(token::konto(1), "cloudflare-api-token", "samme post som 1.0.0");
        s.gem_zoner(1, &[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t")
            .unwrap();
        s.gem_valgte(&["z1".into()]).unwrap();
        s.gem_notifikation_zone("z1", Some("fra")).unwrap();
        s.gem_indstilling("sprog", "da").unwrap();
        s.gem_doegn("z1", "2026-11-02", &[], "t").unwrap();
        // forbindelse_glem på den sidste = glem_alle: hver forbindelse for sig, så det hele.
        assert_eq!(s.forbindelse_ider().unwrap().len(), 1, "den sidste");
        for id in ider_eller_1(s.forbindelse_ider().unwrap()) {
            s.slet_zoner(id).unwrap();
        }
        s.slet_alle().unwrap();
        assert!(s.forbindelser().unwrap().is_empty());
        assert_eq!(s.zoner().unwrap(), (vec![], None));
        assert!(s.valgte().unwrap().is_empty() && s.notifikation_zoner().unwrap().is_empty());
        assert_eq!(s.hentet("z1").unwrap().len(), 1, "tallene bliver");
        assert_eq!(s.indstilling("sprog").unwrap().as_deref(), Some("da"));
        assert_eq!(s.ny_forbindelse("t").unwrap(), 1, "næste API Token er igen forbindelse 1");
        // To forbindelser: forbindelse_tilfoej giver 2, og forbindelse_glem(1) rører ikke 2.
        let to = s.ny_forbindelse("t").unwrap();
        assert_eq!(to, 2);
        assert_eq!(token::konto(to), "cloudflare-api-token-2");
        s.gem_zoner(1, &[zone("z1", "a.wi24rd.com")], "t").unwrap();
        s.gem_zoner(to, &[zone("z2", "b.wi24rd.com")], "t").unwrap();
        s.gem_valgte(&["z1".into(), "z2".into()]).unwrap();
        assert!(s.forbindelse_ider().unwrap().len() > 1, "ikke den sidste: kun glem_en");
        s.slet_zoner(1).unwrap();
        assert_eq!(s.forbindelse_ider().unwrap(), [to]);
        assert_eq!(s.valg().unwrap(), ["z2"]);
    }

    /// V1 (V-KONTI.1 FIX 1): `forbindelse_tilfoej` afviser et API Token, der ser præcis de samme domæner som en forbindelse, der findes (samme, ikke-tomme
    /// mængde), med den forbindelses id. En delmængde (IT-firmaet: ét API Token til ét af kontoens domæner) og en tom liste tilføjes.
    #[test]
    fn samme_api_token_to_gange_afvises_delmaengde_og_tom_tilfoejes() {
        let mut s = Store::i_hukommelse().unwrap();
        assert_eq!(s.ny_forbindelse("t").unwrap(), 1);
        s.gem_zoner(1, &[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t")
            .unwrap();
        // Ens mængde, i en anden rækkefølge: afvist med forbindelsens id; fejlen har kun id'et.
        let ens = [zone("z2", "b.wi24rd.com"), zone("z1", "a.wi24rd.com")];
        assert_eq!(samme_domaener(&s, &ens).unwrap(), Some(1), "samme domæner som forbindelse 1");
        let f = Fejl::AlleredeForbundet(1);
        assert_eq!(
            serde_json::to_value(&f).unwrap(),
            serde_json::json!({ "kode": "allerede_forbundet", "detalje": "1" })
        );
        // Delmængde: tilføjes.
        assert_eq!(samme_domaener(&s, &[zone("z2", "b.wi24rd.com")]).unwrap(), None, "delmængde");
        // Overmængde: tilføjes også (ikke "præcis de samme").
        let mere = [zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com"), zone("z3", "c.wi24rd.com")];
        assert_eq!(samme_domaener(&s, &mere).unwrap(), None, "overmængde");
        // Tom liste: tilføjes (den sammenlignes aldrig, heller ikke med en forbindelse uden domæner).
        let to = s.ny_forbindelse("t").unwrap();
        assert_eq!(samme_domaener(&s, &[]).unwrap(), None, "tom liste, også når en forbindelse er tom");
        // Med to forbindelser findes den, der ser det samme; ens med den nye delmængde er nu forbindelse 2.
        s.gem_zoner(to, &[zone("z2", "b.wi24rd.com")], "t").unwrap();
        assert_eq!(samme_domaener(&s, &[zone("z2", "b.wi24rd.com")]).unwrap(), Some(to));
        assert_eq!(samme_domaener(&s, &ens).unwrap(), Some(1));
    }

    /// F2 (V-KONTI.1 FIX 1): et delt domæne har alle de forbindelser, der ser det (også afviste), og ejeren er stadig den laveste ikke-afviste (D3).
    #[test]
    fn domaener_hent_giver_alle_forbindelser_for_et_delt_domaene() {
        let mut s = Store::i_hukommelse().unwrap();
        let (a, b) = (s.ny_forbindelse("t").unwrap(), s.ny_forbindelse("t").unwrap());
        s.gem_zoner(a, &[zone("z1", "a.wi24rd.com"), zone("z2", "b.wi24rd.com")], "t")
            .unwrap();
        s.gem_zoner(b, &[zone("z2", "b.wi24rd.com")], "t").unwrap();
        s.gem_valgte(&["z2".into()]).unwrap();
        let ud = |afviste: &HashSet<i64>| {
            domaener_ud(
                s.zoner().unwrap().0,
                &s.valgte().unwrap(),
                &s.nye().unwrap(),
                &s.ejere(afviste).unwrap(),
                &s.seere().unwrap(),
            )
            .into_iter()
            .map(|d| (d.id, d.til, d.forbindelse, d.forbindelser))
            .collect::<Vec<_>>()
        };
        assert_eq!(
            ud(&HashSet::new()),
            [
                ("z1".to_string(), false, Some(a), vec![a]),
                ("z2".to_string(), true, Some(a), vec![a, b]),
            ],
            "det delte domæne har begge; ét valg"
        );
        assert_eq!(
            ud(&HashSet::from([a]))[1],
            ("z2".to_string(), true, Some(b), vec![a, b]),
            "afvist ejer: den næste ejer det, og begge står stadig"
        );
        assert_eq!(s.forbindelser_for("z2").unwrap(), [a, b], "samme som forbindelser_for");
    }

    /// F2: `hentet` er sand, når forbindelsen ser domæner, eller når dens liste er hentet (også tom); ellers falsk.
    #[test]
    fn forbindelsen_er_hentet_med_domaener_eller_efter_zones() {
        let mut s = Store::i_hukommelse().unwrap();
        let (a, b, c) = (
            s.ny_forbindelse("t").unwrap(),
            s.ny_forbindelse("t").unwrap(),
            s.ny_forbindelse("t").unwrap(),
        );
        s.gem_zoner(a, &[zone("z1", "a.wi24rd.com")], "t").unwrap();
        s.gem_zoner(b, &[], "t").unwrap();
        let ud = forbindelser_ud(s.forbindelser().unwrap(), &HashSet::new(), &HashSet::from([b]));
        assert_eq!(
            ud.iter().map(|f| (f.f.id, f.f.domaener, f.hentet)).collect::<Vec<_>>(),
            [(a, 1, true), (b, 0, true), (c, 0, false)]
        );
        let j = serde_json::to_value(&ud[2]).unwrap();
        assert_eq!(j["hentet"], false);
        assert!(j.get("token").is_none() && j.get("konto").is_none(), "ingen hemmeligheder");
    }

    #[test]
    fn nyvalgt_finder_det_der_er_kommet_til() {
        let foer = HashSet::from(["a".to_string()]);
        assert_eq!(nyvalgt(&foer, &["a".into(), "c".into(), "b".into()]).as_deref(), Some("b"));
        assert_eq!(nyvalgt(&foer, &["a".into()]), None, "intet nyt");
        assert_eq!(nyvalgt(&foer, &[]), None, "kun fravalgt");
    }

    /// P2: de valgte domæner efter navn, med "har tal"; en zone uden for den gemte liste står som "…".
    #[test]
    fn valgte_domaener_efter_navn() {
        let navne = HashMap::from([
            ("a".to_string(), "wi24rd.com".to_string()),
            ("b".to_string(), "abe.wi24rd.com".to_string()),
        ]);
        let v = valgte(&["a".into(), "b".into(), "c".into()], &navne, &["a".into()]);
        let ud: Vec<(&str, &str, bool)> = v.iter().map(|x| (x.id.as_str(), x.navn.as_str(), x.med_tal)).collect();
        assert_eq!(ud, [("b", "abe.wi24rd.com", false), ("a", "wi24rd.com", true), ("c", "…", false)]);
    }

    #[test]
    fn startet_ved_login_kun_med_argumentet() {
        let a = |v: &[&str]| startet_ved_login(v.iter().map(|s| s.to_string()));
        assert!(a(&[
            "/Applications/Huginn and Muninn.app/Contents/MacOS/huginn-muninn",
            "--autostart"
        ]));
        assert!(
            !a(&["/Applications/Huginn and Muninn.app/Contents/MacOS/huginn-muninn"]),
            "dobbeltklik"
        );
        assert!(!a(&["--autostart"]), "programmets sti tæller ikke");
        assert!(!a(&["huginn-muninn", "--autostart=1"]));
        assert!(!a(&[]));
    }

    #[test]
    fn uden_vindue_ved_login_og_ved_daglig_start() {
        let a = |v: &[&str]| uden_vindue(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(a(&["huginn-muninn", "--autostart"]));
        assert!(a(&["huginn-muninn", "--dagligt"]));
        assert!(!a(&["huginn-muninn"]), "en almindelig start viser vinduet");
    }

    /// Autostart slås kun til af sig selv i en bundlet build, og kun én gang (et fravalg holder).
    #[test]
    fn autostart_standard_kun_bundlet_og_kun_een_gang() {
        assert!(skal_slaa_autostart_til(false, None), "første start af en bundlet build");
        assert!(!skal_slaa_autostart_til(true, None), "tauri dev");
        assert!(!skal_slaa_autostart_til(false, Some("1")), "anvendt før: brugerens valg gælder");
    }

    /// Kun gyldige CVE-numre bliver til en adresse, og adressen er præcis cve.org's.
    /// Beslutning 2: en bølge, der kun ses på et fravalgt domæne, tæller ikke, og det fravalgte tæller heller ikke med i de 3.
    #[test]
    fn kun_valgte_domaener_i_sammenligningen() {
        const NU: i64 = 1_793_693_280_000; // 2026-11-03T08:08:00Z
        let dag = |i: i64| vaern::dag_noegle(NU - i * vaern::DOEGN_MS);
        let hentet = vaern::iso(NU - 3_600_000);
        let rk = |netvaerk: &str, n: i64| {
            let noegle = ["Regel", "US", netvaerk, "/", "CPH", "GET", "firewallManaged", "block"].join("\t");
            vec![("fw".to_string(), noegle, n)]
        };
        let med = |valgte: &[&str]| {
            let mut s = Store::i_hukommelse().unwrap();
            let zoner: Vec<Zone> = ["a", "b", "c", "d"].iter().map(|z| zone(z, &format!("{z}.wi24rd.com"))).collect();
            s.gem_zoner(1, &zoner, "t").unwrap();
            s.gem_valgte(&valgte.iter().map(|z| z.to_string()).collect::<Vec<_>>()).unwrap();
            for (z, net, n) in [
                ("a", "Mal AS", 40),
                ("b", "Stille AS", 1),
                ("c", "Stille AS", 1),
                ("d", "Mal AS", 40),
            ] {
                s.gem_doegn(z, &dag(1), &rk(net, n), &hentet).unwrap();
            }
            byg_overblik(&s, NU).unwrap()
        };
        let a = |rk: Vec<overblik::Raekke1>| rk.into_iter().find(|r| r.id == "a").unwrap();
        let r = a(med(&["a", "b", "c"]));
        assert!(
            r.moenster.is_some() && r.kan_sammenligne,
            "netværket på det fravalgte d tæller ikke"
        );
        assert!(a(med(&["a", "b", "c", "d"])).moenster.is_none(), "valgt tæller det");
        let r = a(med(&["a", "b"]));
        assert!(r.moenster.is_none() && !r.kan_sammenligne, "2 valgte: det fravalgte fylder ikke op");
    }

    /// Et mærke afgøres, mens programmet er slukket. Det melder ved næste start, én gang, og det vurderede rykker frem.
    #[test]
    fn maerke_afgjort_mens_programmet_var_slukket_melder_senere() {
        const NU: i64 = 1_793_693_280_000; // 2026-11-03T08:08:00Z
        const FOER: i64 = NU - 4 * vaern::DOEGN_MS;
        let dag = |i: i64| vaern::dag_noegle(NU - i * vaern::DOEGN_MS);
        let rk = |netvaerk: &str, n: i64| {
            let noegle = ["Regel", "US", netvaerk, "/", "CPH", "GET", "firewallManaged", "block"].join("\t");
            vec![("fw".to_string(), noegle, n)]
        };
        let mut s = Store::i_hukommelse().unwrap();
        let zoner: Vec<Zone> = ["a", "b", "c"].iter().map(|z| zone(z, &format!("{z}.wi24rd.com"))).collect();
        s.gem_zoner(1, &zoner, "t").unwrap();
        s.gem_valgte(&["a".to_string(), "b".to_string(), "c".to_string()]).unwrap();
        for z in ["a", "b", "c"] {
            for i in 5..=9 {
                s.gem_doegn(z, &dag(i), &rk("Stille AS", 1), &vaern::iso(FOER - 3_600_000)).unwrap();
            }
        }
        let runde = |s: &mut Store, nu: i64| {
            let (rk, ind) = (byg_overblik(s, nu).unwrap(), notify::indst(s).unwrap());
            naeste_besked(s, &rk, &ind, nu).unwrap()
        };
        assert_eq!(runde(&mut s, FOER), None);
        assert_eq!(s.vurderet().unwrap()["a"], dag(6), "før den slukkede periode");
        for z in ["a", "b", "c"] {
            for i in 1..=4 {
                let tal = if z == "a" && i == 4 { rk("Mal AS", 40) } else { rk("Stille AS", 1) };
                s.gem_doegn(z, &dag(i), &tal, &vaern::iso(NU - 3_600_000)).unwrap();
            }
        }
        let b = runde(&mut s, NU).expect("mærket er ikke tabt");
        assert_eq!(b.fund.len(), 1);
        assert_eq!(
            (b.fund[0].r.id.as_str(), b.fund[0].moenster().unwrap().dag.as_str()),
            ("a", dag(4).as_str())
        );
        assert_eq!(s.vurderet().unwrap()["a"], dag(2), "statusdøgnet venter på døgnet efter");
        assert_eq!(runde(&mut s, NU + 3_600_000), None, "én besked pr. mærke");
    }

    /// V-SUBDOM.1 (S2): tallene pr. vært ændrer hverken Overblik eller notifikationen, og et værtsnavn står aldrig i en notifikation, på dansk eller engelsk.
    #[test]
    fn vaerter_naar_aldrig_overblik_eller_notifikationer() {
        const NU: i64 = 1_793_693_280_000; // 2026-11-03T08:08:00Z
        let dag = |i: i64| vaern::dag_noegle(NU - i * vaern::DOEGN_MS);
        let fw = |n: i64| {
            let noegle = ["Regel", "US", "AS", "/", "CPH", "GET", "firewallManaged", "block"].join("\t");
            ("fw".to_string(), noegle, n)
        };
        let byg = |med_vaerter: bool| {
            let mut s = Store::i_hukommelse().unwrap();
            let zoner: Vec<Zone> = ["a", "b", "c"].iter().map(|z| zone(z, &format!("{z}.wi24rd.com"))).collect();
            s.gem_zoner(1, &zoner, "t").unwrap();
            s.gem_valgte(&["a".to_string(), "b".to_string(), "c".to_string()]).unwrap();
            for z in ["a", "b", "c"] {
                for i in 1..=9 {
                    let n = if z == "a" && i == 1 { 400 } else { 1 };
                    let mut rk = vec![fw(n)];
                    if med_vaerter {
                        rk.push(("v_fw".into(), "hemmelig\tfirewallManaged\tblock".into(), n));
                        rk.push(("v_fw_afskaaret".into(), String::new(), 10_000));
                    }
                    s.gem_doegn(z, &dag(i), &rk, &vaern::iso(NU - 3_600_000)).unwrap();
                    if med_vaerter {
                        let v = |art: &str, n: i64| (dag(i), art.to_string(), "hemmelig".to_string(), n);
                        s.gem_vaert(z, &[dag(i)], &[v("v_alle", 99), v("v_besoeg", 9)], "t").unwrap();
                        s.noter_vaert_fw(z, &dag(i), "t").unwrap();
                    }
                }
            }
            s
        };
        let (mut med, mut uden) = (byg(true), byg(false));
        let (rm, ru) = (byg_overblik(&med, NU).unwrap(), byg_overblik(&uden, NU).unwrap());
        assert_eq!(
            serde_json::to_value(&rm).unwrap(),
            serde_json::to_value(&ru).unwrap(),
            "Overblik er det samme"
        );
        assert!(!serde_json::to_string(&rm).unwrap().contains("hemmelig"));
        let (im, iu) = (notify::indst(&med).unwrap(), notify::indst(&uden).unwrap());
        let bm = naeste_besked(&mut med, &rm, &im, NU).unwrap();
        let bu = naeste_besked(&mut uden, &ru, &iu, NU).unwrap();
        assert_eq!(bm, bu, "samme notifikation");
        let b = bm.expect("a er rød: der er en notifikation at se på");
        for sprog in [agent::Sprog::Da, agent::Sprog::En] {
            let (titel, tekst) = notify::tekst(&b, sprog, &notify::dato_format(&med).unwrap());
            assert!(!titel.contains("hemmelig") && !tekst.contains("hemmelig"), "{titel} / {tekst}");
        }
    }

    /// Kun valgte domæner, hver med sine linjer; "Ved ikke" står med, og et valg på et fravalgt domæne tæller til knappen.
    #[test]
    fn teknologi_oversigten_taeller_valg_pr_valgt_domaene() {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(
            1,
            &[zone("a", "a.wi24rd.com"), zone("b", "b.wi24rd.com"), zone("c", "c.wi24rd.com")],
            "t",
        )
        .unwrap();
        s.gem_valgte(&["a".to_string(), "b".to_string()]).unwrap();
        let o = tek_oversigt(&s).unwrap();
        assert_eq!(o.zoner["a"], 0);
        assert!(!o.zoner.contains_key("c") && !o.valg);
        s.gem_teknologi("b", "php", "bruger_ikke", false, "t").unwrap();
        assert_eq!(tek_oversigt(&s).unwrap().zoner["b"], 1);
        s.nulstil_teknologi(None).unwrap();
        s.gem_teknologi("c", "php", "bruger", false, "t").unwrap();
        let o = tek_oversigt(&s).unwrap();
        assert!(o.valg && o.zoner["b"] == 0, "knappen nulstiller også fravalgte domæner");
    }

    /// Siden: de valgte domæner, første gang intet "nyt", og et nyt angreb på en ny teknologi afkrydses med en linje.
    #[test]
    fn teknologi_siden_afkrydser_det_sete() {
        const NU: i64 = 1_793_693_280_000;
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(1, &[zone("a", "a.wi24rd.com"), zone("c", "c.wi24rd.com")], "t")
            .unwrap();
        s.gem_valgte(&["a".to_string()]).unwrap();
        let dag = vaern::dag_noegle(NU - vaern::DOEGN_MS);
        let regel = |r: &str| {
            (
                "fw".to_string(),
                [r, "US", "N", "/", "CPH", "GET", "firewallManaged", "block"].join("\t"),
                3,
            )
        };
        s.gem_doegn("a", &dag, &[regel("React - RCE")], &vaern::iso(NU)).unwrap();
        let u = teknologi_side_for(&mut s, NU).unwrap();
        assert_eq!(u.domaener.iter().map(|d| d.navn.as_str()).collect::<Vec<_>>(), ["a.wi24rd.com"]);
        let r = u.katalog.iter().find(|k| k.tek == "react").unwrap();
        assert!(r.vis && r.set && !r.ny);
        s.gem_doegn("a", &dag, &[regel("React - RCE"), regel("Yeti Platform - RCE")], &vaern::iso(NU))
            .unwrap();
        let u = teknologi_side_for(&mut s, NU).unwrap();
        let y = u.katalog.iter().find(|k| k.tek == "yeti platform").unwrap();
        assert!(y.vis && y.ny && y.navn == "Yeti Platform");
    }

    #[test]
    fn api_token_siden_er_cloudflares_template_med_de_to_laeserettigheder() {
        let u = Url::parse(API_TOKEN_SIDE).unwrap();
        assert_eq!((u.host_str(), u.path()), (Some("dash.cloudflare.com"), "/profile/api-tokens"));
        let q: HashMap<String, String> = u.query_pairs().into_owned().collect();
        assert_eq!(
            q["permissionGroupKeys"],
            r#"[{"key":"analytics","type":"read"},{"key":"zone","type":"read"}]"#
        );
        assert_eq!(
            (q["accountId"].as_str(), q["zoneId"].as_str(), q["name"].as_str()),
            ("*", "all", "Huginn and Muninn")
        );
    }

    #[test]
    fn vejledningen_aabner_forsiden_paa_sproget() {
        assert_eq!(vejledning_adresse("da"), "https://wi24rd.com/dokumentation/huginn-muninn/");
        assert_eq!(vejledning_adresse("en"), "https://wi24rd.com/en/documentation/huginn-muninn/");
    }

    #[test]
    fn vejledningens_vaert_er_altid_wi24rd_com() {
        for sprog in ["da", "en", "", "xx", "../evil", "https://evil.example/"] {
            let u = Url::parse(vejledning_adresse(sprog)).unwrap();
            assert_eq!((u.scheme(), u.host_str()), ("https", Some("wi24rd.com")), "{sprog}");
        }
    }

    #[test]
    fn vejledningen_bruger_ikke_sproget_i_adressen() {
        for sprog in ["xx", "../evil", "40-visninger/vaernet"] {
            assert_eq!(vejledning_adresse(sprog), VEJLEDNING_DA, "{sprog}");
        }
    }

    #[test]
    fn cve_adresse_kun_for_gyldige_numre() {
        assert_eq!(
            cve_adresse("CVE-2025-55182").unwrap(),
            "https://www.cve.org/CVERecord?id=CVE-2025-55182"
        );
        assert_eq!(
            cve_adresse("CVE-2017-5638").unwrap(),
            "https://www.cve.org/CVERecord?id=CVE-2017-5638"
        );
        assert_eq!(
            cve_adresse("CVE-2026-1234567").unwrap(),
            "https://www.cve.org/CVERecord?id=CVE-2026-1234567"
        );
        for nej in [
            "CVE-2025-55182; rm",
            "CVE-2025-55182 && open /",
            "javascript:alert(1)",
            "",
            "CVE-2025-55182\n",
            "cve-2025-55182",
            "CVE-25-55182",
            "CVE-2025-123",
            "CVE-2025-12345678",
            "CVE-٢٠٢٥-55182",
            "https://www.cve.org/CVERecord?id=CVE-2025-55182",
        ] {
            assert_eq!(cve_adresse(nej), Err(Fejl::Intern), "{nej:?}");
        }
    }

    #[test]
    fn kortdata_er_indlejret_og_gyldig() {
        assert!(KORTDATA["verden"]["lande"].is_object());
        assert!(KORTDATA["colo"].is_object());
        assert_eq!(KORTDATA["tidszoner"]["zoner"]["Europe/Copenhagen"], "DK");
    }
}
