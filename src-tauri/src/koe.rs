//! Hentekøen: henter alle zoners døgn i baggrunden, så længe programmet kører, også når vinduet er lukket, og programmet
//! kun står i menulinjen. Rækkefølge: zonen på skærmen først, derefter zoner uden færdige døgn, derefter
//! ældste "sidst hentet". Ny runde, når der er gået en time på vægur, straks efter dvale (`skal_hente`, tjekket hvert
//! `TIK`) og straks, når siden eller menuen beder om det (`koe_prioritet`, "Hent nu", `online`). Hver zone hentes med den samme
//! funktion og under den samme lås pr. zone som `vaern_oversigt`, så der aldrig hentes dobbelt. Tempoet styres af den
//! fælles begrænser i `cf`; køen melder fremskridt som hændelsen `hentekoe` og husker den seneste (`koe_status`). Efter en
//! færdig runde henter `ekstra_runde` de ekstra tal (`adgang`, årstallene og til sidst besøgene) for færdige døgn, ét kald ad gangen i langsomt tempo. Den har
//! lavest prioritet: enhver anmodning om en ny runde afbryder den, og en fejl i den rører hverken zonen eller køen. Kun valgte domæner hentes
//! (`Store::zoner_til`, og igen ved turen, for et domæne kan være fravalgt undervejs), så uden valgte domæner henter køen intet. Et nyvalgt domæne
//! får det, der mangler i hentevinduet (`dage_at_hente`), altså højst de seneste 30 dage, og står forrest (`prioriter`).
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use futures_util::future::Either;
use serde::Serialize;
use tokio::sync::Notify;

use crate::adgang;
use crate::besoeg;
use crate::cf::{Fejl, Zone, MELLEMRUM_LANGSOM};
use crate::dmarc;
use crate::historik;
use crate::store::{Store, BESOEG_FEJL, DMARC_FEJL, DNS_FEJL};
use crate::vaern::{dag_noegle, dage_at_hente, er_faerdig, iso, sidst_hentet, Doegnudfald, DOEGN_MS, HENT_DAGE};

/// Ny runde så ofte (og straks ved `koe_prioritet`).
pub const RUNDE: Duration = Duration::from_secs(3600);
const RUNDE_MS: i64 = RUNDE.as_millis() as i64;
/// Løkken sover så længe ad gangen og spørger `skal_hente`. tokios ur tæller ikke dvale på macOS (`CLOCK_UPTIME_RAW`),
/// så en søvn på en hel `RUNDE` kunne komme op til en time for sent efter opvågning; vægurets spring opdager dvalen.
pub const TIK: Duration = Duration::from_secs(60);
const TIK_MS: i64 = TIK.as_millis() as i64;
/// Kommer et tik så meget senere end ventet på vægur, har maskinen sovet: ny runde straks. 5 min, så App Naps
/// forsinkede timere (et tik, der kommer lidt for sent) ikke giver en runde hvert tik.
pub const SPRING_MS: i64 = 300_000;
/// En zone, hvis døgn alle er hentet inden for så mange ms, springes over i runden (den er lige hentet af siden).
pub const FRISK_MS: i64 = 600_000;
/// Ingen forbindelse: nyt forsøg efter 15/30/60 s, derefter hvert 2. minut (som genforsoeg.js).
pub const NET_TRIN: [u64; 4] = [15, 30, 60, 120];
/// Så mange forsøg pr. zone pr. runde ved forbigående fejl (kvote, 5xx, ingen forbindelse).
pub const MAKS_FORSOEG: usize = 4;
/// Mellem to kald i `ekstra_runde`: 30 døgn for 10 domæner tager knap 40 minutter.
pub const EKSTRA_TEMPO: Duration = MELLEMRUM_LANGSOM;

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Fase {
    Henter,
    Pause,
    VenterKvote,
    Faerdig,
    Fejl,
}

/// Hændelsen `hentekoe` (og svaret fra `koe_status`). Døgn og zoner tælles for hele runden.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct KoeStatus {
    pub zone: Option<String>,
    pub navn: Option<String>,
    pub doegn_faerdige: usize,
    pub doegn_i_alt: usize,
    pub zoner_faerdige: usize,
    pub zoner_i_alt: usize,
    /// Zoner, der ikke kunne hentes i runden (fejl, der ikke går over af sig selv).
    pub zoner_fejl: usize,
    /// Zoner, der fik mindst ét døgn i runden ("Hentet kl. … — N domæner opdateret").
    pub zoner_opdateret: usize,
    pub tilstand: Fase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fejl: Option<Fejl>,
    /// Hvornår køen fortsætter (ISO, UTC) ved `pause` og `venter_kvote`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fortsaetter_kl: Option<String>,
}

/// Det, køen skal bruge udefra: hent én zone (fælles med `vaern_oversigt`) og begrænserens pause.
pub trait Henter: Sync {
    fn zone<'a>(
        &'a self,
        zone: &'a str,
        paa_doegn: &'a mut (dyn FnMut(&str, Doegnudfald<'_>) + Send),
    ) -> impl Future<Output = Result<usize, Fejl>> + Send + 'a;
    /// Hvornår begrænseren lader kald gå igen (ms, UTC), hvis den holder pause.
    fn pause_til(&self) -> Option<i64>;
    /// Henter og gemmer de ekstra tal for ét færdigt døgn.
    fn ekstra<'a>(&'a self, zone: &'a str, dag: &'a str) -> impl Future<Output = Result<(), Fejl>> + Send + 'a;
    /// Henter og gemmer enkelthændelserne for et gammelt døgn, der mangler dem.
    fn angreb<'a>(&'a self, zone: &'a str, dag: &'a str) -> impl Future<Output = Result<(), Fejl>> + Send + 'a;
    /// Henter og gemmer lækkede kodeord for ét færdigt døgn.
    fn laek<'a>(&'a self, zone: &'a str, dag: &'a str) -> impl Future<Output = Result<(), Fejl>> + Send + 'a;
    /// Henter og gemmer DNS-tallene for ét færdigt døgn.
    fn dns<'a>(&'a self, zone: &'a str, dag: &'a str) -> impl Future<Output = Result<(), Fejl>> + Send + 'a;
    /// Henter og gemmer DMARC-tallene for ét færdigt døgn.
    fn dmarc<'a>(&'a self, zone: &'a str, dag: &'a str) -> impl Future<Output = Result<(), Fejl>> + Send + 'a;
    /// Henter og gemmer årstallene for døgnene `fra`…`til`.
    fn historik<'a>(&'a self, zone: &'a str, fra: &'a str, til: &'a str) -> impl Future<Output = Result<(), Fejl>> + Send + 'a;
    /// Henter `unikke` igen for døgnene `fra`…`til`, som zonen allerede har årstal for, og gemmer kun det tal (`Store::gem_unikke`).
    fn unikke<'a>(&'a self, _zone: &'a str, _fra: &'a str, _til: &'a str) -> impl Future<Output = Result<(), Fejl>> + Send + 'a {
        async { Ok(()) }
    }
    /// Henter og gemmer besøgene pr. land for døgnene `fra`…`til` (`besoeg::BESOEG_QUERY`).
    fn besoeg<'a>(&'a self, _zone: &'a str, _fra: &'a str, _til: &'a str) -> impl Future<Output = Result<(), Fejl>> + Send + 'a {
        async { Ok(()) }
    }
    /// Henter og gemmer zonens tilstand hos Cloudflare.
    fn tilstand<'a>(&'a self, zone: &'a str) -> impl Future<Output = Result<(), Fejl>> + Send + 'a;
    /// Henter og gemmer Cloudflares egne sikkerhedsfund for kontoen, højst én gang i timen (`sikkerhed::skal_hente`). Udfaldet noteres af henteren selv.
    fn fund(&self) -> impl Future<Output = ()> + Send + '_ {
        async {}
    }
}

/// DNS og DMARC: så mange døgn tilbage hentes de, når de mangler. Cloudflare gemmer dem i 31 og 32 dage (målt live). Er det ældste døgn for gammelt, noteres det som tomt.
pub const AFSNIT_DAGE: i64 = 31;

/// Afsnit af de ekstra tal, der hentes ens: en fejl koster kun afsnittet for domænet, og teksten står ved afsnittet.
#[derive(Clone, Copy)]
enum Afsnit {
    Dns,
    Dmarc,
}

impl Afsnit {
    fn mangler(self, s: &Store, zone: &str, dag: &str, nu: i64) -> bool {
        match self {
            Afsnit::Dns => s.hentet_dns(zone).is_ok_and(|h| adgang::mangler(dag, &h, nu)),
            Afsnit::Dmarc => s.hentet_dmarc(zone).is_ok_and(|h| dmarc::mangler(dag, &h, nu)),
        }
    }
    fn gem_tomt(self, s: &mut Store, zone: &str, dag: &str, nu: i64) -> Result<(), Fejl> {
        match self {
            Afsnit::Dns => s.gem_dns(zone, dag, &[], &iso(nu)),
            Afsnit::Dmarc => s.gem_dmarc(zone, dag, &[], &iso(nu)),
        }
    }
    fn fejl_praefiks(self) -> &'static str {
        match self {
            Afsnit::Dns => DNS_FEJL,
            Afsnit::Dmarc => DMARC_FEJL,
        }
    }
}

/// En zone i rundens plan.
#[derive(Debug, Clone)]
pub struct Plan {
    pub zone: Zone,
    /// Antal færdighentede døgn i hentevinduet.
    pub faerdige: usize,
    pub sidst_hentet: Option<String>,
    /// Døgn, der mangler i runden (til fremskridtet).
    pub rest: usize,
    pub forsoeg: usize,
}

impl Plan {
    fn ny(zone: Zone, hentet: &HashMap<String, String>, nu: i64) -> Plan {
        let faerdige = (0..=HENT_DAGE)
            .map(|i| dag_noegle(nu - i * DOEGN_MS))
            .filter(|d| hentet.get(d).is_some_and(|h| er_faerdig(d, h)))
            .count();
        Plan {
            zone,
            faerdige,
            sidst_hentet: sidst_hentet(hentet, nu),
            rest: dage_at_hente(nu, hentet).len(),
            forsoeg: 0,
        }
    }
}

/// Hvilken zone står for tur: prioriteten (zonen på skærmen), derefter zoner uden færdige døgn, derefter ældste "sidst
/// hentet" (aldrig hentet først); ens: efter navn. Ren funktion.
pub fn naeste(rest: &[Plan], prioritet: Option<&str>) -> Option<usize> {
    if let Some(i) = prioritet.and_then(|p| rest.iter().position(|x| x.zone.id == p)) {
        return Some(i);
    }
    (0..rest.len()).min_by(|&a, &b| {
        let noegle = |p: &Plan| (p.faerdige > 0, p.sidst_hentet.clone(), p.zone.navn.clone());
        noegle(&rest[a]).cmp(&noegle(&rest[b]))
    })
}

/// Er alle døgn hentet for nylig (inden for `FRISK_MS`)? Så springes zonen over i runden.
pub fn er_frisk(hentet: &HashMap<String, String>, nu: i64) -> bool {
    let graense = iso(nu - FRISK_MS);
    let dage = dage_at_hente(nu, hentet);
    dage.iter().all(|d| hentet.get(d).is_some_and(|h| *h >= graense))
}

/// Hvad "Hent nu" gjorde, så UI'et ikke skal gætte.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HentUdfald {
    Startet,
    KoererAllerede,
    IntetAtHente,
}

/// "Hent nu": kører en runde allerede, sker der intet nyt; er alt friskt, og køen er sund, er der intet at hente; ellers startes den. En kø, der står efter en fejl, startes altid. Ren funktion.
pub fn hent_udfald(status: Option<&KoeStatus>, koerer: bool, alt_friskt: bool) -> HentUdfald {
    let fase = status.map(|s| s.tilstand);
    if koerer && matches!(fase, Some(Fase::Henter | Fase::Pause | Fase::VenterKvote)) {
        HentUdfald::KoererAllerede
    } else if koerer && alt_friskt && fase != Some(Fase::Fejl) {
        HentUdfald::IntetAtHente
    } else {
        HentUdfald::Startet
    }
}

/// Ny runde? En time på vægur, siden den sidste runde sluttede, eller tikket kom mere end `SPRING_MS` for sent (uret
/// sprang: maskinen har sovet). Stilles uret tilbage (NTP), bliver forskellene negative: ingen runde. Ren funktion.
pub fn skal_hente(nu: i64, sidste_runde_slut: i64, forrige_tik: i64) -> bool {
    nu.saturating_sub(sidste_runde_slut) >= RUNDE_MS || nu.saturating_sub(forrige_tik) > TIK_MS + SPRING_MS
}

/// Er domænet stadig valgt? Kan listen ikke læses, gælder det, der stod ved rundens start.
fn slaaet_til(lager: &Mutex<Store>, zone: &str) -> bool {
    lager.lock().ok().and_then(|s| s.valgte().ok()).is_none_or(|v| v.contains(zone))
}

fn forbigaaende(e: &Fejl) -> bool {
    matches!(e, Fejl::Vent | Fejl::CfNede(_) | Fejl::Forbindelse)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Slut {
    Faerdig,
    /// Tokenet blev afvist (401/403) eller glemt: køen står, til et nyt token er gemt.
    Stoppet,
    /// `ekstra_runde` blev afbrudt af en anmodning om en ny runde: den kører straks.
    Vaekket,
}

#[derive(Default)]
pub struct Koe {
    status: Mutex<Option<KoeStatus>>,
    prioritet: Mutex<Option<String>>,
    vaek: Notify,
    stop: AtomicBool,
    /// Hvorfor køen er stoppet (`stop_med`): meldes, når løkken er slut.
    grund: Mutex<Option<Fejl>>,
    /// Kører køens løkke? Kun én ad gangen.
    pub koerer: AtomicBool,
    /// Runder, der er gjort helt færdige, ekstra tal med (`Fase::Faerdig` kommer før dem). Den daglige start lukker på den.
    pub hele_runder: AtomicUsize,
    /// Domæner, hvor Cloudflare ikke svarede på besøgene: der spørges ikke igen, før programmet starter igen.
    besoeg_opgivet: Mutex<HashSet<String>>,
}

impl Koe {
    pub fn status(&self) -> Option<KoeStatus> {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Zonen på skærmen kommer først (ved næste zone i en runde, der kører); vækker køen til en ny runde.
    pub fn prioriter(&self, zone: Option<String>) {
        if zone.is_some() {
            *self.prioritet.lock().unwrap_or_else(|e| e.into_inner()) = zone;
        }
        self.vaek.notify_one();
    }

    fn prioritet(&self) -> Option<String> {
        self.prioritet.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.vaek.notify_one();
    }

    /// Stopper køen med en fejl (fx `IntetToken` fra "Glem tokenet"). Den meldes, når løkken er slut, så en zone, der
    /// er ved at blive hentet, ikke når at skrive "henter" over den; kører løkken ikke, meldes den med det samme.
    pub fn stop_med(&self, e: Fejl, meld: &(dyn Fn(&KoeStatus) + Sync)) {
        *self.grund.lock().unwrap_or_else(|e| e.into_inner()) = Some(e);
        self.stop();
        if !self.koerer.load(Ordering::SeqCst) {
            self.meld_grund(meld);
        }
    }

    fn meld_grund(&self, meld: &(dyn Fn(&KoeStatus) + Sync)) {
        let grund = self.grund.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(e) = grund {
            self.fejl(meld, e);
        }
    }

    pub fn fortsaet(&self) {
        self.grund.lock().unwrap_or_else(|e| e.into_inner()).take();
        self.stop.store(false, Ordering::SeqCst);
    }

    pub fn stoppet(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Venter, til løkken er slut efter `stop`. En zone, der er i gang, hentes færdig først; det kan tage et stykke tid.
    pub async fn slut(&self) {
        while self.koerer.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Sover `d`, men vågner straks ved `prioriter` eller `stop`. `true`, hvis den blev vækket.
    pub async fn sov(&self, d: Duration) -> bool {
        let (sov, vaek) = (pin!(tokio::time::sleep(d)), pin!(self.vaek.notified()));
        matches!(futures_util::future::select(sov, vaek).await, Either::Right(_))
    }

    /// Venter uden tidsgrænse på `prioriter` eller `stop`.
    async fn vent(&self) {
        self.vaek.notified().await;
    }

    /// Køen kan ikke hente, fordi tokenet ikke kunne læses (eller er glemt): fejlen meldes, og der hentes ikke.
    pub fn fejl(&self, meld: &(dyn Fn(&KoeStatus) + Sync), e: Fejl) {
        self.meld(meld, |s| {
            s.zone = None;
            s.navn = None;
            s.tilstand = Fase::Fejl;
            s.fejl = Some(e);
            s.fortsaetter_kl = None;
        });
    }

    /// Køens løkke: én runde med det samme, derefter en ny, når `skal_hente` siger ja (spurgt hvert `TIK`), eller straks
    /// ved `prioriter`. `runde` læser tokenet og kører `Koe::runde`. Kan tokenet ikke læses, meldes fejlen, og løkken
    /// venter på `prioriter` ("Hent nu" eller et nyt token) uden selv at prøve igen; mangler tokenet, stopper køen.
    /// `paa_tik` kaldes ved hvert tik (menulinjens "for N min. siden"). Returnerer, når køen er stoppet.
    pub async fn koer<F, Fut>(
        &self,
        mut runde: F,
        meld: &(dyn Fn(&KoeStatus) + Sync),
        nu: &(dyn Fn() -> i64 + Sync),
        paa_tik: &(dyn Fn() + Sync),
    ) where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<Slut, Fejl>>,
    {
        loop {
            if self.stoppet() {
                self.koerer.store(false, Ordering::SeqCst);
                // Et nyt token kan være gemt imens; så fortsætter denne løkke i stedet for at starte en ny.
                if self.stoppet() || self.koerer.swap(true, Ordering::SeqCst) {
                    self.meld_grund(meld);
                    return;
                }
            }
            match runde().await {
                Ok(Slut::Faerdig) => {
                    self.hele_runder.fetch_add(1, Ordering::SeqCst);
                    self.vent_paa_runde(nu, paa_tik).await
                }
                Ok(Slut::Stoppet | Slut::Vaekket) => {}
                Err(e) => {
                    let intet = e == Fejl::IntetToken;
                    self.fejl(meld, e);
                    if intet {
                        self.stop();
                    } else {
                        self.vent().await;
                    }
                }
            }
        }
    }

    /// Sover et `TIK` ad gangen, til `skal_hente` siger ja, eller køen bliver vækket.
    async fn vent_paa_runde(&self, nu: &(dyn Fn() -> i64 + Sync), paa_tik: &(dyn Fn() + Sync)) {
        let mut sidste = nu();
        let mut forrige = sidste;
        loop {
            if self.sov(TIK).await {
                return;
            }
            let n = nu();
            paa_tik();
            // Er uret stillet tilbage, regnes timen fra nu: ingen runde for hvert tik, og ingen, der venter i timevis.
            sidste = sidste.min(n);
            if skal_hente(n, sidste, forrige) {
                return;
            }
            forrige = n;
        }
    }

    /// De ekstra tal efter en færdig runde: døgn for døgn, nyeste først, for alle zoner, og kun hvor `hentet_ekstra` mangler.
    /// `tempo` mellem kaldene. Fejl stopper hverken køen eller andre zoner: en forbigående fejl (kvote, 5xx, net) gemmer resten
    /// til næste runde, og alt andet, også 401/403, noteres ved zonen, som er færdig for denne gang (fx et felt, planen ikke har).
    /// Det er kun den normale runde, der kan afgøre, at tokenet er afvist. `true`: en ny runde blev bedt om undervejs.
    pub async fn ekstra_runde<H: Henter>(
        &self,
        lager: &Mutex<Store>,
        h: &H,
        nu: &(dyn Fn() -> i64 + Sync),
        tempo: Duration,
        paa_gemt: &(dyn Fn(&str) + Sync),
    ) -> bool {
        let zoner = lager.lock().ok().and_then(|s| s.zoner_til().ok()).unwrap_or_default();
        // Zonens tilstand: ét lille kald pr. valgt domæne og runde. En fejl her stopper ingenting, og tilstanden fra sidst står.
        for z in &zoner {
            if self.stoppet() {
                return false;
            }
            let _ = h.tilstand(&z.id).await;
        }
        // Cloudflares egne fund: ét kald pr. konto, højst én gang i timen, og kun med valgte domæner. En fejl stopper ingenting.
        if !zoner.is_empty() {
            if self.stoppet() {
                return false;
            }
            h.fund().await;
        }
        let mut opgivet: HashSet<&str> = HashSet::new();
        for dag in adgang::vinduet(nu()) {
            for z in &zoner {
                if self.stoppet() {
                    return false;
                }
                let hentet = lager.lock().ok().and_then(|s| s.hentet_ekstra(&z.id).ok()).unwrap_or_default();
                if opgivet.contains(z.id.as_str()) || !slaaet_til(lager, &z.id) || !adgang::mangler(&dag, &hentet, nu()) {
                    continue;
                }
                let noter = |fejl: Option<&str>| {
                    if let Ok(mut s) = lager.lock() {
                        let _ = s.gem_adgang_fejl(&z.id, fejl);
                    }
                };
                match h.ekstra(&z.id, &dag).await {
                    Ok(()) => {
                        noter(None);
                        paa_gemt(&z.id);
                    }
                    Err(e) if e.uden_for_opbevaring() => {
                        if let Ok(mut s) = lager.lock() {
                            let _ = s.gem_ekstra(&z.id, &dag, &[], &iso(nu()));
                        }
                    }
                    Err(e) if forbigaaende(&e) => return false,
                    Err(e) => {
                        noter(Some(&e.to_string()));
                        opgivet.insert(&z.id);
                    }
                }
                if self.sov(tempo).await {
                    return true;
                }
            }
        }
        // Enkelthændelser for de gamle døgn, bagefter og i samme tempo. Nye døgn får dem med i den normale runde, så det her er kun efter en opgradering (skema 11 og 12).
        for dag in adgang::vinduet(nu()) {
            for z in &zoner {
                if self.stoppet() {
                    return false;
                }
                let hentet = lager.lock().ok().and_then(|s| s.hentet_angreb(&z.id).ok()).unwrap_or_default();
                if opgivet.contains(z.id.as_str()) || !slaaet_til(lager, &z.id) || !adgang::mangler(&dag, &hentet, nu()) {
                    continue;
                }
                match h.angreb(&z.id, &dag).await {
                    Ok(()) => paa_gemt(&z.id),
                    Err(e) if e.uden_for_opbevaring() => {
                        if let Ok(mut s) = lager.lock() {
                            let _ = s.noter_angreb(&z.id, &dag, &iso(nu()));
                        }
                    }
                    Err(e) if forbigaaende(&e) => return false,
                    Err(_) => {
                        opgivet.insert(&z.id);
                    }
                }
                if self.sov(tempo).await {
                    return true;
                }
            }
        }
        // Lækkede kodeord, samme tempo. Fejler feltet på et domæne, er det kun det, der mangler.
        let mut opgivet: HashSet<&str> = HashSet::new();
        for dag in adgang::vinduet(nu()) {
            for z in &zoner {
                if self.stoppet() {
                    return false;
                }
                let hentet = lager.lock().ok().and_then(|s| s.hentet_laek(&z.id).ok()).unwrap_or_default();
                if opgivet.contains(z.id.as_str()) || !slaaet_til(lager, &z.id) || !adgang::mangler(&dag, &hentet, nu()) {
                    continue;
                }
                match h.laek(&z.id, &dag).await {
                    Ok(()) => paa_gemt(&z.id),
                    Err(e) if e.uden_for_opbevaring() => {
                        if let Ok(mut s) = lager.lock() {
                            let _ = s.gem_laek(&z.id, &dag, &[], &iso(nu()));
                        }
                    }
                    Err(e) if forbigaaende(&e) => return false,
                    Err(_) => {
                        opgivet.insert(&z.id);
                    }
                }
                if self.sov(tempo).await {
                    return true;
                }
            }
        }
        // DNS og DMARC, samme tempo. En fejl her skjuler kun afsnittet for domænet: hverken andre domæner, det andet afsnit eller årstallene mærker det. Kun en forbigående fejl gemmer resten til næste runde.
        let dage: Vec<String> = (1..=AFSNIT_DAGE).map(|i| dag_noegle(nu() - i * DOEGN_MS)).collect();
        for afsnit in [Afsnit::Dns, Afsnit::Dmarc] {
            let mut opgivet: HashSet<&str> = HashSet::new();
            for dag in &dage {
                for z in &zoner {
                    if self.stoppet() {
                        return false;
                    }
                    let mangler = lager.lock().ok().is_some_and(|s| afsnit.mangler(&s, &z.id, dag, nu()));
                    if opgivet.contains(z.id.as_str()) || !slaaet_til(lager, &z.id) || !mangler {
                        continue;
                    }
                    let noter = |fejl: Option<&str>| {
                        if let Ok(mut s) = lager.lock() {
                            let _ = s.gem_afsnit_fejl(afsnit.fejl_praefiks(), &z.id, fejl);
                        }
                    };
                    let svar = match afsnit {
                        Afsnit::Dns => h.dns(&z.id, dag).await,
                        Afsnit::Dmarc => h.dmarc(&z.id, dag).await,
                    };
                    match svar {
                        Ok(()) => {
                            noter(None);
                            paa_gemt(&z.id);
                        }
                        Err(e) if e.uden_for_opbevaring() => {
                            if let Ok(mut s) = lager.lock() {
                                let _ = afsnit.gem_tomt(&mut s, &z.id, dag, nu());
                            }
                        }
                        Err(e) if forbigaaende(&e) => return false,
                        Err(e) => {
                            noter(Some(&e.to_string()));
                            opgivet.insert(&z.id);
                        }
                    }
                    if self.sov(tempo).await {
                        return true;
                    }
                }
            }
        }
        // Årstallene sidst, når alt andet er hentet: ét kald pr. domæne dækker året, og bagefter kun de nye døgn.
        for z in &zoner {
            if self.stoppet() {
                return false;
            }
            let hentet = lager.lock().ok().and_then(|s| s.hentet_historik(&z.id).ok()).unwrap_or_default();
            let Some((fra, til)) = historik::mangler(&hentet, nu()).filter(|_| slaaet_til(lager, &z.id)) else {
                continue;
            };
            match h.historik(&z.id, &fra, &til).await {
                Ok(()) => paa_gemt(&z.id),
                Err(e) if forbigaaende(&e) => return false,
                Err(_) => {}
            }
            if self.sov(tempo).await {
                return true;
            }
        }
        // Unikke besøgende for de døgn, der blev hentet før tallet kom med (`Store::unikke_en_gang`): ét kald pr. domæne for året, én gang. Kun det tal skrives.
        for z in &zoner {
            if self.stoppet() {
                return false;
            }
            if !lager.lock().ok().is_some_and(|s| s.unikke_mangler(&z.id).unwrap_or(false)) || !slaaet_til(lager, &z.id) {
                continue;
            }
            let hentet = lager.lock().ok().and_then(|s| s.hentet_historik(&z.id).ok()).unwrap_or_default();
            let Some((fra, til)) = historik::unikke_spaend(&hentet, nu()) else {
                if let Ok(mut s) = lager.lock() {
                    let _ = s.gem_unikke(&z.id, &[]);
                }
                continue;
            };
            match h.unikke(&z.id, &fra, &til).await {
                Ok(()) => paa_gemt(&z.id),
                Err(e) if forbigaaende(&e) => return false,
                Err(_) => {}
            }
            if self.sov(tempo).await {
                return true;
            }
        }
        // Besøg pr. land, efter årstallene: ét kald pr. domæne for de færdige døgn, der mangler. Afviser Cloudflare det, står det ved domænet som
        // "ikke tilgængelig", resten kører videre, og der spørges ikke igen før næste start. Kun en forbigående fejl gemmer resten til næste runde.
        for z in &zoner {
            if self.stoppet() {
                return false;
            }
            let opgivet = self.besoeg_opgivet.lock().unwrap_or_else(|e| e.into_inner()).contains(&z.id);
            let hentet = lager.lock().ok().and_then(|s| s.hentet_besoeg(&z.id).ok()).unwrap_or_default();
            let Some((fra, til)) = besoeg::mangler(&hentet, nu()).filter(|_| !opgivet && slaaet_til(lager, &z.id)) else {
                continue;
            };
            let opgiv = || {
                self.besoeg_opgivet.lock().unwrap_or_else(|e| e.into_inner()).insert(z.id.clone());
            };
            let noter = |fejl: Option<&str>| {
                if let Ok(mut s) = lager.lock() {
                    let _ = s.gem_afsnit_fejl(BESOEG_FEJL, &z.id, fejl);
                }
            };
            match h.besoeg(&z.id, &fra, &til).await {
                Ok(()) => {
                    noter(None);
                    paa_gemt(&z.id);
                }
                Err(e) if forbigaaende(&e) => return false,
                // Det ældste døgn var for gammelt: det er ikke domænets skyld, og næste start prøver igen.
                Err(e) if e.uden_for_opbevaring() => opgiv(),
                Err(e) => {
                    noter(Some(&e.to_string()));
                    opgiv();
                }
            }
            if self.sov(tempo).await {
                return true;
            }
        }
        false
    }

    /// Ændrer status, gemmer den og melder den.
    fn meld(&self, meld: &(dyn Fn(&KoeStatus) + Sync), f: impl FnOnce(&mut KoeStatus)) {
        let s = {
            let mut g = self.status.lock().unwrap_or_else(|e| e.into_inner());
            let s = g.get_or_insert_with(|| KoeStatus {
                zone: None,
                navn: None,
                doegn_faerdige: 0,
                doegn_i_alt: 0,
                zoner_faerdige: 0,
                zoner_i_alt: 0,
                zoner_fejl: 0,
                zoner_opdateret: 0,
                tilstand: Fase::Henter,
                fejl: None,
                fortsaetter_kl: None,
            });
            f(s);
            s.clone()
        };
        meld(&s);
    }

    /// Én runde over alle zoner i den gemte zoneliste. `nu` er uret (ms, UTC); det læses pr. zone.
    pub async fn runde<H: Henter>(
        &self,
        lager: &Mutex<Store>,
        h: &H,
        meld: &(dyn Fn(&KoeStatus) + Sync),
        nu: &(dyn Fn() -> i64 + Sync),
    ) -> Slut {
        // Kan cachen ikke læses, behandles zonen som uhentet (den hentes så bare).
        let hentet_for = |id: &str| lager.lock().ok().and_then(|s| s.hentet(id).ok()).unwrap_or_default();
        let zoner = lager.lock().ok().and_then(|s| s.zoner_til().ok()).unwrap_or_default();
        let t = nu();
        let mut rest = Vec::new();
        let mut friske = 0;
        for z in zoner.iter().cloned() {
            let hentet = hentet_for(&z.id);
            if er_frisk(&hentet, t) {
                friske += 1;
            } else {
                rest.push(Plan::ny(z, &hentet, t));
            }
        }
        self.meld(meld, |s| {
            *s = KoeStatus {
                zone: None,
                navn: None,
                doegn_faerdige: 0,
                doegn_i_alt: rest.iter().map(|p| p.rest).sum(),
                zoner_faerdige: friske,
                zoner_i_alt: zoner.len(),
                zoner_fejl: 0,
                zoner_opdateret: 0,
                tilstand: Fase::Henter,
                fejl: None,
                fortsaetter_kl: None,
            }
        });
        let mut net_trin = 0;
        while let Some(i) = naeste(&rest, self.prioritet().as_deref()) {
            if self.stoppet() {
                return Slut::Stoppet;
            }
            let mut p = rest.remove(i);
            if !slaaet_til(lager, &p.zone.id) {
                self.meld(meld, |s| {
                    s.zoner_faerdige += 1;
                    s.doegn_i_alt -= p.rest.min(s.doegn_i_alt);
                });
                continue;
            }
            // Planen genberegnes ved turen: siden kan have hentet zonen imens, og tiden går.
            let t = nu();
            let hentet = hentet_for(&p.zone.id);
            let nye = dage_at_hente(t, &hentet).len();
            let (id, navn) = (p.zone.id.clone(), p.zone.navn.clone());
            self.meld(meld, |s| {
                s.doegn_i_alt = s.doegn_i_alt + nye - p.rest.min(s.doegn_i_alt);
                s.zone = Some(id.clone());
                s.navn = Some(navn.clone());
                s.tilstand = Fase::Henter;
                s.fortsaetter_kl = None;
            });
            p.rest = nye;
            p.forsoeg += 1;
            let mut gjort = 0;
            let mut paa_doegn = |_: &str, u: Doegnudfald<'_>| match u {
                Doegnudfald::Gemt | Doegnudfald::UdenFor(_) => {
                    gjort += 1;
                    self.meld(meld, |s| {
                        s.doegn_faerdige += 1;
                        s.tilstand = Fase::Henter;
                        s.fortsaetter_kl = None;
                    });
                }
                Doegnudfald::Fejl(e @ (Fejl::Vent | Fejl::CfNede(_))) => {
                    let fase = if *e == Fejl::Vent { Fase::VenterKvote } else { Fase::Pause };
                    let kl = h.pause_til().map(iso);
                    self.meld(meld, |s| {
                        s.tilstand = fase;
                        s.fortsaetter_kl = kl;
                    });
                }
                Doegnudfald::Fejl(_) => {}
            };
            let resultat = h.zone(&p.zone.id, &mut paa_doegn).await;
            p.rest = p.rest.saturating_sub(gjort);
            match resultat {
                Ok(_) => {
                    net_trin = 0;
                    self.meld(meld, |s| {
                        s.zoner_faerdige += 1;
                        s.zoner_opdateret += usize::from(gjort > 0);
                    });
                }
                Err(e @ Fejl::Afvist(_)) => {
                    self.stop.store(true, Ordering::SeqCst);
                    self.meld(meld, |s| {
                        s.tilstand = Fase::Fejl;
                        s.fejl = Some(e);
                        s.fortsaetter_kl = None;
                    });
                    return Slut::Stoppet;
                }
                Err(e) if forbigaaende(&e) && p.forsoeg < MAKS_FORSOEG => {
                    if e == Fejl::Forbindelse {
                        let sek = NET_TRIN[net_trin.min(NET_TRIN.len() - 1)];
                        net_trin += 1;
                        self.meld(meld, |s| {
                            s.tilstand = Fase::Pause;
                            s.fejl = Some(Fejl::Forbindelse);
                            s.fortsaetter_kl = Some(iso(nu() + sek as i64 * 1000));
                        });
                        self.sov(Duration::from_secs(sek)).await;
                        self.meld(meld, |s| s.fejl = None);
                    }
                    // Kvote og 5xx: begrænseren holder selv pausen; zonen prøves igen ved næste tur.
                    rest.push(p);
                }
                Err(e) => {
                    self.meld(meld, |s| {
                        s.zoner_faerdige += 1;
                        s.zoner_fejl += 1;
                        s.doegn_i_alt -= p.rest.min(s.doegn_i_alt);
                        s.fejl.get_or_insert(e);
                    });
                }
            }
        }
        self.meld(meld, |s| {
            s.zone = None;
            s.navn = None;
            s.tilstand = Fase::Faerdig;
            s.fortsaetter_kl = None;
        });
        Slut::Faerdig
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaern::dag_ms;

    /// 2026-11-03T08:08:00Z
    const NU: i64 = 1_793_693_280_000;

    fn zone(id: &str, navn: &str) -> Zone {
        Zone {
            id: id.into(),
            navn: navn.into(),
        }
    }

    fn plan(id: &str, faerdige: usize, sidst: Option<&str>) -> Plan {
        Plan {
            zone: zone(id, &format!("{id}.wi24rd.com")),
            faerdige,
            sidst_hentet: sidst.map(str::to_string),
            rest: 0,
            forsoeg: 0,
        }
    }

    fn status_med(tilstand: Fase) -> KoeStatus {
        KoeStatus {
            zone: None,
            navn: None,
            doegn_faerdige: 0,
            doegn_i_alt: 0,
            zoner_faerdige: 2,
            zoner_i_alt: 4,
            zoner_fejl: 0,
            zoner_opdateret: 0,
            tilstand,
            fejl: None,
            fortsaetter_kl: None,
        }
    }

    /// Hvad "Hent nu" svarer.
    #[test]
    fn hent_nu_siger_hvad_der_skete() {
        use HentUdfald::*;
        let u = |f: Option<Fase>, koerer, friskt| hent_udfald(f.map(status_med).as_ref(), koerer, friskt);
        for f in [Fase::Henter, Fase::Pause, Fase::VenterKvote] {
            assert_eq!(u(Some(f), true, false), KoererAllerede, "{f:?}");
            assert_eq!(u(Some(f), true, true), KoererAllerede, "{f:?}");
        }
        assert_eq!(u(Some(Fase::Faerdig), true, true), IntetAtHente);
        assert_eq!(u(Some(Fase::Faerdig), true, false), Startet);
        assert_eq!(u(Some(Fase::Fejl), true, true), Startet, "en kø, der står efter en fejl, startes");
        assert_eq!(u(Some(Fase::Faerdig), false, true), Startet, "løkken kører ikke");
        assert_eq!(u(None, false, true), Startet);
    }

    /// Efter et hul står zonen med det ældste "sidst hentet" forrest, og der hentes højst de 31 døgn, Cloudflare stadig har.
    #[test]
    fn efter_et_hul_hentes_forrest_og_hoejst_31_doegn() {
        let nu = crate::vaern::parse_iso("2026-11-03T08:08:00Z").unwrap();
        let dag = |i: i64| dag_noegle(nu - i * DOEGN_MS);
        let zone = |id: &str| Zone {
            id: id.into(),
            navn: id.into(),
        };
        let frisk: HashMap<String, String> = (0..=HENT_DAGE).map(|i| (dag(i), iso(nu - 3_600_000))).collect();
        let hul: HashMap<String, String> = (28..=HENT_DAGE).map(|i| (dag(i), iso(nu - 27 * DOEGN_MS))).collect();
        let rest = [Plan::ny(zone("frisk"), &frisk, nu), Plan::ny(zone("hul"), &hul, nu)];
        assert_eq!(naeste(&rest, None), Some(1));
        assert_eq!(rest[1].rest, 28, "kun det, der mangler");
        assert!(dage_at_hente(nu, &HashMap::new()).len() as i64 <= HENT_DAGE + 1);
    }

    #[test]
    fn raekkefoelge_prioritet_derefter_uden_faerdige_derefter_aeldste() {
        let rest = vec![
            plan("c", 30, Some("2026-11-03T07:00:00.000Z")),
            plan("a", 30, Some("2026-11-03T06:00:00.000Z")),
            plan("e", 0, None),
            plan("b", 0, Some("2026-11-03T08:00:00.000Z")), // hentet, men intet færdigt
            plan("d", 30, None),
        ];
        let orden = |prio: Option<&str>| {
            let mut r = rest.clone();
            let mut ud = Vec::new();
            while let Some(i) = naeste(&r, prio) {
                ud.push(r.remove(i).zone.id);
            }
            ud
        };
        assert_eq!(
            orden(None),
            ["e", "b", "d", "a", "c"],
            "uden færdige døgn først; så ældste sidst hentet"
        );
        assert_eq!(orden(Some("c")), ["c", "e", "b", "d", "a"], "zonen på skærmen først");
        assert_eq!(orden(Some("ukendt")), ["e", "b", "d", "a", "c"]);
        assert_eq!(naeste(&[], Some("a")), None);
    }

    #[test]
    fn frisk_zone_springes_over() {
        let alle =
            |h: &str| -> HashMap<String, String> { (0..=HENT_DAGE).map(|i| (dag_noegle(NU - i * DOEGN_MS), h.to_string())).collect() };
        assert!(er_frisk(&alle(&iso(NU - 60_000)), NU), "hentet for et minut siden");
        assert!(
            !er_frisk(&alle(&iso(NU - 3_600_000)), NU),
            "i dag og i går skal hentes igen efter en time"
        );
        assert!(!er_frisk(&HashMap::new(), NU));
    }

    // ── En hel runde med en falsk henter (ingen net) ──
    struct Falsk<'a> {
        lager: Mutex<Vec<String>>,
        /// zone-id → svar i rækkefølge (det sidste gentages); `None` = alt går godt.
        svar: HashMap<String, Vec<Result<(), Fejl>>>,
        pause: Option<i64>,
        /// Sættes, når zonen hentes (prioritet midt i runden).
        prioritet_ved: Option<(String, String)>,
        koe: &'a Koe,
        /// `ekstra`: kaldene, svar pr. zone (ingen = det går godt), lageret, der gemmes i, og om kaldet beder om en ny runde.
        ekstra_kald: Mutex<Vec<(String, String)>>,
        angreb_kald: Mutex<Vec<(String, String)>>,
        ekstra_svar: HashMap<String, Result<(), Fejl>>,
        angreb_svar: Result<(), Fejl>,
        laek_kald: Mutex<Vec<(String, String)>>,
        /// `dns`, `dmarc`: kaldene og svar pr. zone (ingen = det går godt).
        dns_kald: Mutex<Vec<(String, String)>>,
        dmarc_kald: Mutex<Vec<(String, String)>>,
        dns_svar: HashMap<String, Result<(), Fejl>>,
        dmarc_svar: HashMap<String, Result<(), Fejl>>,
        historik_kald: Mutex<Vec<(String, String, String)>>,
        /// `unikke`, `besoeg`: kaldene (zone, fra, til) og svar pr. zone (ingen = det går godt).
        unikke_kald: Mutex<Vec<(String, String, String)>>,
        besoeg_kald: Mutex<Vec<(String, String, String)>>,
        besoeg_svar: HashMap<String, Result<(), Fejl>>,
        tilstand_kald: Mutex<Vec<String>>,
        tilstand_svar: Result<(), Fejl>,
        /// Rækkefølgen af kaldene: zone, ekstra, angreb, laek, historik, tilstand.
        log: Mutex<Vec<&'static str>>,
        ekstra_lager: Option<&'a Mutex<Store>>,
        ekstra_vaek: bool,
    }

    impl Henter for Falsk<'_> {
        async fn zone(&self, zone: &str, paa: &mut (dyn FnMut(&str, Doegnudfald<'_>) + Send)) -> Result<usize, Fejl> {
            let n = {
                let mut l = self.lager.lock().unwrap();
                l.push(zone.to_string());
                l.iter().filter(|z| *z == zone).count()
            };
            if let Some((ved, prio)) = &self.prioritet_ved {
                if ved == zone {
                    self.koe.prioriter(Some(prio.clone()));
                }
            }
            let svar = self.svar.get(zone).map(|v| v[(n - 1).min(v.len() - 1)].clone()).unwrap_or(Ok(()));
            match svar {
                Ok(()) => {
                    paa("2026-11-03", Doegnudfald::Gemt);
                    paa("2026-11-02", Doegnudfald::Gemt);
                    paa("2026-10-03", Doegnudfald::UdenFor(&Fejl::Svar));
                    Ok(2)
                }
                Err(e) => {
                    paa("2026-11-03", Doegnudfald::Gemt);
                    paa("2026-11-02", Doegnudfald::Fejl(&e));
                    Err(e)
                }
            }
        }
        fn pause_til(&self) -> Option<i64> {
            self.pause
        }
        async fn angreb(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
            self.angreb_kald.lock().unwrap().push((zone.to_string(), dag.to_string()));
            self.log.lock().unwrap().push("angreb");
            self.angreb_svar.clone()?;
            if let Some(l) = self.ekstra_lager {
                l.lock().unwrap().gem_angreb(zone, dag, &[], "t").unwrap();
            }
            Ok(())
        }
        async fn laek(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
            self.laek_kald.lock().unwrap().push((zone.to_string(), dag.to_string()));
            self.log.lock().unwrap().push("laek");
            if let Some(l) = self.ekstra_lager {
                l.lock().unwrap().gem_laek(zone, dag, &[], "t").unwrap();
            }
            Ok(())
        }
        async fn dns(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
            self.dns_kald.lock().unwrap().push((zone.to_string(), dag.to_string()));
            self.log.lock().unwrap().push("dns");
            let svar = self.dns_svar.get(zone).cloned().unwrap_or(Ok(()));
            if let (Ok(()), Some(l)) = (&svar, self.ekstra_lager) {
                l.lock().unwrap().gem_dns(zone, dag, &[], "9999").unwrap();
            }
            svar
        }
        async fn dmarc(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
            self.dmarc_kald.lock().unwrap().push((zone.to_string(), dag.to_string()));
            self.log.lock().unwrap().push("dmarc");
            let svar = self.dmarc_svar.get(zone).cloned().unwrap_or(Ok(()));
            if let (Ok(()), Some(l)) = (&svar, self.ekstra_lager) {
                l.lock().unwrap().gem_dmarc(zone, dag, &[], &iso(NU)).unwrap();
            }
            svar
        }
        async fn historik(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
            self.historik_kald
                .lock()
                .unwrap()
                .push((zone.to_string(), fra.to_string(), til.to_string()));
            self.log.lock().unwrap().push("historik");
            if let Some(l) = self.ekstra_lager {
                let dage: Vec<String> = (1..=historik::HISTORIK_DAGE).map(|i| dag_noegle(NU - i * DOEGN_MS)).collect();
                l.lock().unwrap().gem_historik(zone, &dage, &[], "t").unwrap();
            }
            Ok(())
        }
        async fn unikke(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
            self.unikke_kald
                .lock()
                .unwrap()
                .push((zone.to_string(), fra.to_string(), til.to_string()));
            self.log.lock().unwrap().push("unikke");
            if let Some(l) = self.ekstra_lager {
                l.lock().unwrap().gem_unikke(zone, &[]).unwrap();
            }
            Ok(())
        }
        async fn besoeg(&self, zone: &str, fra: &str, til: &str) -> Result<(), Fejl> {
            self.besoeg_kald
                .lock()
                .unwrap()
                .push((zone.to_string(), fra.to_string(), til.to_string()));
            self.log.lock().unwrap().push("besoeg");
            let svar = self.besoeg_svar.get(zone).cloned().unwrap_or(Ok(()));
            if let (Ok(()), Some(l)) = (&svar, self.ekstra_lager) {
                l.lock()
                    .unwrap()
                    .gem_besoeg(zone, &crate::robotter::dage_i(fra, til), &[], &iso(NU))
                    .unwrap();
            }
            svar
        }
        async fn tilstand(&self, zone: &str) -> Result<(), Fejl> {
            self.tilstand_kald.lock().unwrap().push(zone.to_string());
            self.log.lock().unwrap().push("tilstand");
            self.tilstand_svar.clone()
        }
        async fn fund(&self) {
            self.log.lock().unwrap().push("fund");
        }

        async fn ekstra(&self, zone: &str, dag: &str) -> Result<(), Fejl> {
            self.ekstra_kald.lock().unwrap().push((zone.to_string(), dag.to_string()));
            let svar = self.ekstra_svar.get(zone).cloned().unwrap_or(Ok(()));
            if svar.is_ok() {
                if let Some(l) = self.ekstra_lager {
                    l.lock().unwrap().gem_ekstra(zone, dag, &[], "t").unwrap();
                }
            }
            if self.ekstra_vaek {
                self.koe.prioriter(None);
            }
            svar
        }
    }

    fn lager_med(zoner: &[(&str, &str, Option<&str>)]) -> Mutex<Store> {
        let mut s = Store::i_hukommelse().unwrap();
        s.gem_zoner(&zoner.iter().map(|(id, navn, _)| zone(id, navn)).collect::<Vec<_>>(), "t")
            .unwrap();
        s.gem_valgte(&zoner.iter().map(|z| z.0.to_string()).collect::<Vec<_>>()).unwrap();
        for (id, _, hentet) in zoner {
            if let Some(h) = hentet {
                for i in 0..=HENT_DAGE {
                    s.gem_doegn(id, &dag_noegle(NU - i * DOEGN_MS), &[], h).unwrap();
                }
            }
        }
        Mutex::new(s)
    }

    fn falsk(koe: &Koe) -> Falsk<'_> {
        Falsk {
            lager: Mutex::new(Vec::new()),
            svar: HashMap::new(),
            pause: None,
            prioritet_ved: None,
            koe,
            ekstra_kald: Mutex::default(),
            angreb_kald: Mutex::default(),
            ekstra_svar: HashMap::new(),
            angreb_svar: Ok(()),
            laek_kald: Mutex::default(),
            dns_kald: Mutex::default(),
            dmarc_kald: Mutex::default(),
            dns_svar: HashMap::new(),
            dmarc_svar: HashMap::new(),
            historik_kald: Mutex::default(),
            unikke_kald: Mutex::default(),
            besoeg_kald: Mutex::default(),
            besoeg_svar: HashMap::new(),
            tilstand_kald: Mutex::default(),
            tilstand_svar: Ok(()),
            log: Mutex::default(),
            ekstra_lager: None,
            ekstra_vaek: false,
        }
    }

    async fn koer(koe: &Koe, lager: &Mutex<Store>, f: &Falsk<'_>) -> (Slut, Vec<KoeStatus>) {
        let haendelser = Mutex::new(Vec::new());
        let meld = |s: &KoeStatus| haendelser.lock().unwrap().push(s.clone());
        let slut = koe.runde(lager, f, &meld, &|| NU).await;
        (slut, haendelser.into_inner().unwrap())
    }

    #[tokio::test(start_paused = true)]
    async fn runde_henter_alle_i_raekkefoelge_og_melder_fremskridt() {
        let gammel = iso(NU - 30 * 3_600_000);
        let frisk = iso(NU - 60_000);
        let lager = lager_med(&[
            ("a", "a.wi24rd.com", Some(&gammel)),
            ("b", "b.wi24rd.com", None),
            ("c", "c.wi24rd.com", Some(&frisk)),
            ("d", "d.wi24rd.com", None),
        ]);
        let koe = Koe::default();
        let mut f = falsk(&koe);
        f.prioritet_ved = Some(("b".into(), "a".into())); // brugeren åbner a.wi24rd.com, mens b hentes
        let (slut, h) = koer(&koe, &lager, &f).await;
        assert_eq!(slut, Slut::Faerdig);
        assert_eq!(
            *f.lager.lock().unwrap(),
            ["b", "a", "d"],
            "uden færdige døgn først (b, d efter navn), men a blev prioriteret; c er frisk"
        );
        let sidst = h.last().unwrap();
        assert_eq!(sidst.tilstand, Fase::Faerdig);
        assert_eq!((sidst.zoner_faerdige, sidst.zoner_i_alt, sidst.zoner_fejl), (4, 4, 0));
        assert_eq!(sidst.zone, None);
        assert_eq!(h[0].doegn_i_alt, 31 + 2 + 31, "a: i dag + i går; b og d: hele vinduet");
        assert_eq!(
            sidst.doegn_faerdige, 9,
            "tre døgn pr. zone fra den falske henter (inkl. et uden for opbevaringen)"
        );
        assert!(h
            .iter()
            .any(|s| s.navn.as_deref() == Some("b.wi24rd.com") && s.tilstand == Fase::Henter));
        let j = serde_json::to_value(sidst).unwrap();
        assert_eq!(j["tilstand"], "faerdig");
        assert!(j.get("fejl").is_none() && j.get("fortsaetter_kl").is_none());
        assert_eq!(koe.status().as_ref(), Some(sidst), "koe_status = seneste hændelse");
    }

    #[tokio::test(start_paused = true)]
    async fn afvist_token_stopper_koeen() {
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let koe = Koe::default();
        let mut f = falsk(&koe);
        f.svar.insert("a".into(), vec![Err(Fejl::Afvist(403))]);
        let (slut, h) = koer(&koe, &lager, &f).await;
        assert_eq!(slut, Slut::Stoppet);
        assert!(koe.stoppet());
        assert_eq!(*f.lager.lock().unwrap(), ["a"], "b hentes ikke");
        let sidst = h.last().unwrap();
        assert_eq!((sidst.tilstand, sidst.fejl.clone()), (Fase::Fejl, Some(Fejl::Afvist(403))));
        assert_eq!(serde_json::to_value(sidst).unwrap()["fejl"]["kode"], "afvist");
        koe.fortsaet();
        assert!(!koe.stoppet(), "et nyt token starter køen igen");
    }

    /// "Nulstil historik" stopper køen midt i en zone: runden må ikke melde "færdig" (det ville sætte notifikationerne i
    /// gang på halve tal), og de øvrige zoner hentes ikke med den gamle plan.
    #[tokio::test(start_paused = true)]
    async fn stop_midt_i_en_zone_melder_ikke_faerdig() {
        struct StopIZone<'a>(&'a Koe, Mutex<Vec<String>>);
        impl Henter for StopIZone<'_> {
            async fn zone(&self, zone: &str, _: &mut (dyn FnMut(&str, Doegnudfald<'_>) + Send)) -> Result<usize, Fejl> {
                self.1.lock().unwrap().push(zone.to_string());
                self.0.stop();
                Ok(0)
            }
            fn pause_til(&self) -> Option<i64> {
                None
            }
            async fn angreb(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn laek(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn dns(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn dmarc(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn historik(&self, _: &str, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn tilstand(&self, _: &str) -> Result<(), Fejl> {
                Ok(())
            }

            async fn ekstra(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
        }
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let koe = Koe::default();
        let h = StopIZone(&koe, Mutex::new(Vec::new()));
        let meldt = Mutex::new(Vec::new());
        let slut = koe
            .runde(&lager, &h, &|s: &KoeStatus| meldt.lock().unwrap().push(s.tilstand), &|| NU)
            .await;
        assert_eq!(slut, Slut::Stoppet);
        assert_eq!(h.1.lock().unwrap().len(), 1, "kun zonen i gang");
        assert!(!meldt.lock().unwrap().contains(&Fase::Faerdig));
    }

    /// `slut` venter på løkken; er den ikke i gang, returnerer den med det samme.
    #[tokio::test(start_paused = true)]
    async fn slut_venter_til_loekken_er_stoppet() {
        let koe = Koe::default();
        let start = tokio::time::Instant::now();
        koe.slut().await;
        assert_eq!(start.elapsed(), Duration::ZERO);
        koe.koerer.store(true, Ordering::SeqCst);
        let slip = async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            koe.koerer.store(false, Ordering::SeqCst);
        };
        let vent = async {
            koe.slut().await;
            assert!(!koe.koerer.load(Ordering::SeqCst), "slut må ikke returnere, mens løkken kører");
        };
        tokio::join!(vent, slip);
    }

    #[tokio::test(start_paused = true)]
    async fn kvote_melder_fortsaetter_kl_og_proever_zonen_igen() {
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let koe = Koe::default();
        let mut f = falsk(&koe);
        let kl = dag_ms("2026-11-03").unwrap() + 9 * 3_600_000 + 21 * 60_000;
        f.pause = Some(kl);
        f.svar.insert("a".into(), vec![Err(Fejl::Vent), Ok(())]);
        f.svar.insert("b".into(), vec![Err(Fejl::CfNede(503)), Ok(())]);
        let (slut, h) = koer(&koe, &lager, &f).await;
        assert_eq!(slut, Slut::Faerdig);
        let kvote = h.iter().find(|s| s.tilstand == Fase::VenterKvote).expect("venter_kvote meldes");
        assert_eq!(kvote.fortsaetter_kl.as_deref(), Some("2026-11-03T09:21:00.000Z"));
        assert_eq!(kvote.navn.as_deref(), Some("a.wi24rd.com"));
        assert!(
            h.iter()
                .any(|s| s.tilstand == Fase::Pause && s.navn.as_deref() == Some("b.wi24rd.com")),
            "5xx: pause"
        );
        let hentet = f.lager.lock().unwrap().clone();
        assert_eq!(hentet.iter().filter(|z| *z == "a").count(), 2, "a prøves igen");
        assert_eq!(hentet.iter().filter(|z| *z == "b").count(), 2, "b prøves igen");
        assert_eq!(h.last().unwrap().zoner_fejl, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn ingen_forbindelse_pauser_15_s_og_anden_fejl_springer_zonen_over() {
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let koe = Koe::default();
        let mut f = falsk(&koe);
        f.svar.insert("a".into(), vec![Err(Fejl::Forbindelse), Ok(())]);
        f.svar.insert("b".into(), vec![Err(Fejl::Svar)]);
        let t0 = tokio::time::Instant::now();
        let (slut, h) = koer(&koe, &lager, &f).await;
        assert_eq!(slut, Slut::Faerdig);
        assert_eq!(t0.elapsed(), Duration::from_secs(15), "én pause på 15 s (falsk ur)");
        let pause = h.iter().find(|s| s.tilstand == Fase::Pause).unwrap();
        assert_eq!(pause.fejl, Some(Fejl::Forbindelse));
        assert_eq!(pause.fortsaetter_kl.as_deref(), Some(iso(NU + 15_000).as_str()));
        assert_eq!(*f.lager.lock().unwrap(), ["a", "a", "b"], "a prøves igen efter pausen");
        let sidst = h.last().unwrap();
        assert_eq!((sidst.zoner_faerdige, sidst.zoner_fejl), (2, 1));
        assert_eq!(sidst.fejl, Some(Fejl::Svar));
        assert_eq!(sidst.tilstand, Fase::Faerdig);
    }

    /// Q5 a–d: en time på uret, dvale (tikket kom for sent) og uret stillet tilbage.
    #[test]
    fn skal_hente_vaegur_og_spring() {
        let min = 60_000;
        assert!(!skal_hente(NU + 59 * min, NU, NU + 58 * min), "a: 59 min → nej");
        assert!(skal_hente(NU + 60 * min, NU, NU + 59 * min), "b: 60 min → ja");
        assert!(
            skal_hente(NU + 7 * 60 * min, NU + 7 * 60 * min - 5 * min, NU),
            "c: tikket kom 7 timer for sent (dvale) → ja, selvom runden kun er 5 min gammel"
        );
        assert!(!skal_hente(NU + 6 * min, NU, NU + 5 * min), "et almindeligt tik");
        assert!(!skal_hente(NU + 6 * min, NU, NU), "6 min for sent er inden for springet (App Nap)");
        assert!(skal_hente(NU + 6 * min + 1, NU, NU), "lige over springet");
        assert!(!skal_hente(NU - 2 * 60 * min, NU, NU), "d: uret stillet 2 timer tilbage → nej");
        assert!(!skal_hente(i64::MIN, i64::MAX, i64::MAX), "d: ingen panik ved ekstreme værdier");
        assert!(skal_hente(i64::MAX, i64::MIN, i64::MIN));
    }

    /// Løkken med falsk ur: en runde ved start, ingen efter 30 min, en efter 60 min, en straks efter et vægurspring, og
    /// ingen runde-storm, når uret stilles tilbage.
    #[tokio::test(start_paused = true)]
    async fn loekken_tikker_paa_vaegur() {
        use std::sync::atomic::{AtomicI64, AtomicUsize};
        let koe = Koe::default();
        koe.koerer.store(true, Ordering::SeqCst);
        let (runder, tik, forskyd) = (AtomicUsize::new(0), AtomicUsize::new(0), AtomicI64::new(0));
        let t0 = tokio::time::Instant::now();
        let nu = || NU + t0.elapsed().as_millis() as i64 + forskyd.load(Ordering::SeqCst);
        let r = &runder;
        let runde = move || async move {
            r.fetch_add(1, Ordering::SeqCst);
            Ok(Slut::Faerdig)
        };
        let paa_tik = || {
            tik.fetch_add(1, Ordering::SeqCst);
        };
        let antal = || runder.load(Ordering::SeqCst);
        let til = |s: u64| tokio::time::sleep_until(t0 + Duration::from_secs(s));
        futures_util::future::join(koe.koer(runde, &|_| {}, &nu, &paa_tik), async {
            til(1).await;
            assert_eq!(antal(), 1, "en runde ved start");
            til(1830).await;
            assert_eq!(antal(), 1, "ingen runde efter 30 min");
            assert_eq!(tik.load(Ordering::SeqCst), 30, "et tik i minuttet");
            til(3630).await;
            assert_eq!(antal(), 2, "en runde efter 60 min");
            forskyd.fetch_add(10 * 60_000, Ordering::SeqCst); // maskinen har sovet 10 min
            til(3690).await;
            assert_eq!(antal(), 3, "en runde ved første tik efter vægurspringet");
            forskyd.fetch_sub(3 * 3_600_000, Ordering::SeqCst); // NTP stiller uret 3 timer tilbage
            til(7230).await;
            assert_eq!(antal(), 3, "ingen runde-storm, når uret går baglæns");
            til(7350).await;
            assert_eq!(antal(), 4, "en time efter, regnet fra det tilbagestillede ur");
            koe.stop();
        })
        .await;
        assert!(!koe.koerer.load(Ordering::SeqCst), "løkken er slut");
    }

    /// Kan nøgleringen ikke læses, meldes fejl/noegle, og der prøves ikke igen af sig selv; "Hent nu" prøver igen.
    #[tokio::test(start_paused = true)]
    async fn noeglefejl_melder_og_venter_paa_hent_nu() {
        use std::sync::atomic::AtomicUsize;
        let koe = Koe::default();
        koe.koerer.store(true, Ordering::SeqCst);
        let runder = AtomicUsize::new(0);
        let r = &runder;
        let runde = move || async move {
            match r.fetch_add(1, Ordering::SeqCst) {
                0 => Err(Fejl::Noegle),
                _ => Ok(Slut::Faerdig),
            }
        };
        let haendelser = Mutex::new(Vec::new());
        let meld = |s: &KoeStatus| haendelser.lock().unwrap().push(s.clone());
        futures_util::future::join(koe.koer(runde, &meld, &|| NU, &|| {}), async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let s = koe.status().expect("fejlen er gemt til koe_status");
            assert_eq!((s.tilstand, s.fejl.clone()), (Fase::Fejl, Some(Fejl::Noegle)));
            assert_eq!(serde_json::to_value(&s).unwrap()["fejl"]["kode"], "noegle");
            assert_eq!(haendelser.lock().unwrap().len(), 1, "meldt som hentekoe");
            tokio::time::sleep(Duration::from_secs(5 * 3600)).await;
            assert_eq!(runder.load(Ordering::SeqCst), 1, "ingen nye forsøg hver time");
            koe.prioriter(None); // Hent nu
            tokio::time::sleep(Duration::from_secs(1)).await;
            assert_eq!(runder.load(Ordering::SeqCst), 2, "Hent nu prøver igen");
            koe.stop();
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn hele_runder_taeller_kun_runder_der_er_gjort_faerdige() {
        let koe = Koe::default();
        koe.koerer.store(true, Ordering::SeqCst);
        let gang = AtomicUsize::new(0);
        let g = &gang;
        let runde = move || async move {
            match g.fetch_add(1, Ordering::SeqCst) {
                0 => Ok(Slut::Vaekket),
                _ => Ok(Slut::Faerdig),
            }
        };
        futures_util::future::join(koe.koer(runde, &|_| {}, &|| NU, &|| {}), async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            assert_eq!(gang.load(Ordering::SeqCst), 2, "en afbrudt og en hel runde");
            assert_eq!(koe.hele_runder.load(Ordering::SeqCst), 1, "den afbrudte tæller ikke");
            koe.stop();
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn intet_token_stopper_koeen_og_melder_det() {
        let koe = Koe::default();
        koe.koerer.store(true, Ordering::SeqCst);
        let meld = |_: &KoeStatus| {};
        let slut = tokio::time::timeout(RUNDE, koe.koer(|| async { Err(Fejl::IntetToken) }, &meld, &|| NU, &|| {})).await;
        assert!(slut.is_ok(), "løkken slutter (venter ikke)");
        assert!(
            koe.stoppet() && !koe.koerer.load(Ordering::SeqCst),
            "køen står, til et token er gemt"
        );
        assert_eq!(koe.status().unwrap().fejl, Some(Fejl::IntetToken));
    }

    /// "Glem tokenet" midt i en runde: zonen, der hentes, melder færdig, men statussen ender som intet_token.
    #[tokio::test(start_paused = true)]
    async fn glemt_token_midt_i_runden_ender_som_intet_token() {
        let koe = Koe::default();
        koe.koerer.store(true, Ordering::SeqCst);
        let haendelser = Mutex::new(Vec::new());
        let meld = |s: &KoeStatus| haendelser.lock().unwrap().push(s.clone());
        let (k, m) = (&koe, &meld);
        let runde = move || async move {
            k.meld(m, |s| s.tilstand = Fase::Henter);
            tokio::time::sleep(Duration::from_secs(30)).await; // zonen hentes færdig
            k.meld(m, |s| s.tilstand = Fase::Henter);
            Ok(if k.stoppet() { Slut::Stoppet } else { Slut::Faerdig })
        };
        futures_util::future::join(koe.koer(runde, &meld, &|| NU, &|| {}), async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            koe.stop_med(Fejl::IntetToken, &meld);
            assert_eq!(koe.status().unwrap().tilstand, Fase::Henter, "endnu ikke: zonen hentes stadig");
        })
        .await;
        let s = koe.status().unwrap();
        assert_eq!((s.tilstand, s.fejl), (Fase::Fejl, Some(Fejl::IntetToken)));
        assert_eq!(
            haendelser.lock().unwrap().last().unwrap().tilstand,
            Fase::Fejl,
            "meldt som hentekoe"
        );
        // Kører løkken ikke, meldes det med det samme.
        koe.stop_med(Fejl::IntetToken, &meld);
        assert_eq!(haendelser.lock().unwrap().len(), 4);
        // Et nyt token: grunden glemmes.
        koe.fortsaet();
        assert!(koe.grund.lock().unwrap().is_none());
    }

    // ── De ekstra tal ──
    async fn ekstra(koe: &Koe, lager: &Mutex<Store>, f: &Falsk<'_>) -> bool {
        koe.ekstra_runde(lager, f, &|| NU, Duration::ZERO, &|_| {}).await
    }

    fn kald_for(f: &Falsk<'_>, zone: &str) -> usize {
        f.ekstra_kald.lock().unwrap().iter().filter(|(z, _)| z == zone).count()
    }

    #[tokio::test(start_paused = true)]
    async fn enkelthaendelser_hentes_bagefter_kun_hvor_de_mangler_og_kun_for_valgte() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let d = |i: i64| dag_noegle(NU - i * DOEGN_MS);
        lager.lock().unwrap().gem_valgte(&["a".to_string()]).unwrap();
        lager.lock().unwrap().gem_doegn("a", &d(3), &[], "t").unwrap();
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        assert!(!ekstra(&koe, &lager, &f).await);
        let kald = f.angreb_kald.lock().unwrap().clone();
        assert!(kald.iter().all(|(z, _)| z == "a"), "kun valgte domæner");
        assert!(!kald.iter().any(|(_, dag)| *dag == d(3)), "et døgn fra den normale runde har dem");
        assert_eq!(kald.len(), HENT_DAGE as usize - 1);
        assert_eq!(kald[0].1, d(1), "nyeste først");
        f.angreb_kald.lock().unwrap().clear();
        assert!(!ekstra(&koe, &lager, &f).await);
        assert!(f.angreb_kald.lock().unwrap().is_empty(), "hentes kun én gang");
    }

    #[tokio::test(start_paused = true)]
    async fn ekstra_henter_kun_hvor_hentet_ekstra_mangler() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let d = |i: i64| dag_noegle(NU - i * DOEGN_MS);
        for i in 1..=HENT_DAGE {
            if i != 5 {
                lager.lock().unwrap().gem_ekstra("a", &d(i), &[], "t").unwrap();
            }
        }
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        assert!(!ekstra(&koe, &lager, &f).await);
        let kald = f.ekstra_kald.lock().unwrap().clone();
        assert_eq!(
            kald.iter()
                .filter(|(z, _)| z == "a")
                .map(|(_, dag)| dag.clone())
                .collect::<Vec<_>>(),
            [d(5)],
            "kun hullet"
        );
        assert_eq!(kald.iter().filter(|(z, _)| z == "b").count(), 30, "hele vinduet, ikke i dag");
        assert_eq!(kald[0], ("b".to_string(), d(1)), "nyeste døgn først");
        assert!(!kald.iter().any(|(_, dag)| *dag == d(0)), "i dag er ikke færdigt");
        f.ekstra_kald.lock().unwrap().clear();
        assert!(!ekstra(&koe, &lager, &f).await);
        assert!(f.ekstra_kald.lock().unwrap().is_empty(), "alt er hentet: ingen kald");
    }

    fn hentede(f: &Falsk<'_>) -> Vec<String> {
        f.lager.lock().unwrap().clone()
    }

    /// Et domæne, der er slået fra, hentes hverken i runden eller med de ekstra tal, og det tæller ikke med.
    #[tokio::test(start_paused = true)]
    async fn fravalgt_zone_hentes_ikke() {
        let koe = Koe::default();
        let lager = lager_med(&[
            ("a", "a.wi24rd.com", None),
            ("b", "b.wi24rd.com", None),
            ("c", "c.wi24rd.com", None),
        ]);
        lager.lock().unwrap().gem_valgte(&["a".into(), "c".into()]).unwrap();
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        assert_eq!(koe.runde(&lager, &f, &|_: &KoeStatus| {}, &|| NU).await, Slut::Faerdig);
        assert!(!hentede(&f).contains(&"b".to_string()), "{:?}", hentede(&f));
        assert_eq!(hentede(&f).len(), 2);
        assert_eq!(koe.status().unwrap().zoner_i_alt, 2, "menulinjen tæller kun dem, der er til");
        assert!(!ekstra(&koe, &lager, &f).await);
        assert_eq!(kald_for(&f, "b"), 0, "ingen ekstra tal");
        assert_eq!(kald_for(&f, "a"), 30);
    }

    /// T1: uden valgte domæner henter køen ingenting, heller ikke de ekstra tal.
    #[tokio::test(start_paused = true)]
    async fn intet_valgt_intet_hentet() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        lager.lock().unwrap().gem_valgte(&[]).unwrap();
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        assert_eq!(koe.runde(&lager, &f, &|_: &KoeStatus| {}, &|| NU).await, Slut::Faerdig);
        assert!(hentede(&f).is_empty(), "{:?}", hentede(&f));
        assert_eq!(koe.status().unwrap().zoner_i_alt, 0);
        assert!(!ekstra(&koe, &lager, &f).await);
        assert_eq!(kald_for(&f, "a"), 0);
    }

    /// Slås et domæne fra midt i en runde, hentes det ikke, når turen kommer.
    #[tokio::test(start_paused = true)]
    async fn slaaet_fra_undervejs_hentes_ikke() {
        struct Slaa<'a>(&'a Mutex<Store>, Mutex<Vec<String>>);
        impl Henter for Slaa<'_> {
            async fn angreb(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn laek(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn dns(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn dmarc(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn historik(&self, _: &str, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
            async fn tilstand(&self, _: &str) -> Result<(), Fejl> {
                Ok(())
            }

            async fn zone(&self, zone: &str, _: &mut (dyn FnMut(&str, Doegnudfald<'_>) + Send)) -> Result<usize, Fejl> {
                self.1.lock().unwrap().push(zone.to_string());
                self.0.lock().unwrap().gem_valgte(&["a".into()]).unwrap();
                Ok(0)
            }
            fn pause_til(&self) -> Option<i64> {
                None
            }
            async fn ekstra(&self, _: &str, _: &str) -> Result<(), Fejl> {
                Ok(())
            }
        }
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let h = Slaa(&lager, Mutex::new(Vec::new()));
        assert_eq!(koe.runde(&lager, &h, &|_: &KoeStatus| {}, &|| NU).await, Slut::Faerdig);
        assert_eq!(*h.1.lock().unwrap(), ["a"]);
        assert_eq!(koe.status().unwrap().zoner_faerdige, 2, "runden når sin ende");
    }

    /// Slås et domæne til igen, står det forrest, og hullet er højst hentevinduet, uanset hvor længe det var fra.
    #[tokio::test(start_paused = true)]
    async fn genvalg_staar_forrest_og_hullet_er_hoejst_30_dage() {
        let koe = Koe::default();
        let lager = lager_med(&[
            ("a", "a.wi24rd.com", None),
            ("b", "b.wi24rd.com", None),
            ("c", "c.wi24rd.com", None),
        ]);
        koe.prioriter(Some("c".into()));
        let f = falsk(&koe);
        koe.runde(&lager, &f, &|_: &KoeStatus| {}, &|| NU).await;
        assert_eq!(hentede(&f)[0], "c");
        let gammel: HashMap<String, String> = (40..=100)
            .map(|i| (dag_noegle(NU - i * DOEGN_MS), iso(NU - 39 * DOEGN_MS)))
            .collect();
        let dage = dage_at_hente(NU, &gammel);
        assert_eq!(dage.len(), 31, "i dag og 30 døgn tilbage");
        assert_eq!(
            dage.iter().min().unwrap(),
            &dag_noegle(NU - 30 * DOEGN_MS),
            "aldrig ældre end 30 dage"
        );
    }

    /// Radars svar 401/403/429/500 kan ikke stoppe køen, selv hvis det nåede frem til den: det er aldrig `Fejl::Afvist`.
    #[tokio::test(start_paused = true)]
    async fn radar_fejl_stopper_aldrig_koeen() {
        for status in [401, 403, 429, 500] {
            let fejl = crate::cf::tolk_radar(status, None).unwrap_err();
            let koe = Koe::default();
            let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
            let mut f = falsk(&koe);
            f.svar.insert("a".into(), vec![Err(fejl)]);
            assert_eq!(koe.runde(&lager, &f, &|_: &KoeStatus| {}, &|| NU).await, Slut::Faerdig, "{status}");
            assert!(!koe.stoppet(), "{status}");
            assert!(hentede(&f).contains(&"b".to_string()), "{status}: de andre domæner hentes");
        }
    }

    /// Mutation: lad fejlen stoppe køen, og denne test falder.
    #[tokio::test(start_paused = true)]
    async fn fejl_i_ekstra_stopper_hverken_zonen_koeen_eller_de_andre() {
        for fejl in [
            Fejl::GraphQl {
                dag: None,
                detalje: "does not have access to the field".into(),
            },
            Fejl::Afvist(403),
            Fejl::Svar,
        ] {
            let koe = Koe::default();
            let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
            let mut f = falsk(&koe);
            f.ekstra_lager = Some(&lager);
            f.ekstra_svar.insert("a".into(), Err(fejl.clone()));
            assert!(!ekstra(&koe, &lager, &f).await, "{fejl:?}");
            assert!(!koe.stoppet(), "køen står ikke stille: {fejl:?}");
            assert_eq!(kald_for(&f, "a"), 1, "ét forsøg pr. runde");
            assert_eq!(kald_for(&f, "b"), 30, "de andre domæner hentes");
            let noteret = lager.lock().unwrap().adgang_fejl("a").unwrap().unwrap();
            assert_eq!(noteret, fejl.to_string());
            assert_eq!(lager.lock().unwrap().adgang_fejl("b").unwrap(), None);
            assert!(
                lager.lock().unwrap().hentet_ekstra("a").unwrap().is_empty(),
                "prøves igen næste runde"
            );
            // Næste runde virker det: noten forsvinder.
            f.ekstra_svar.clear();
            assert!(!ekstra(&koe, &lager, &f).await);
            assert_eq!(lager.lock().unwrap().adgang_fejl("a").unwrap(), None);
            assert_eq!(lager.lock().unwrap().hentet_ekstra("a").unwrap().len(), 30);
        }
    }

    /// Selve kørslen: en fejl i de ekstra tal må ikke ændre køens status.
    fn afvist() -> Fejl {
        Fejl::GraphQl {
            dag: None,
            detalje: "does not have access to the field".into(),
        }
    }

    /// DNS og DMARC hentes for alle døgn og begge domæner, efter de lækkede kodeord og før årstallene.
    #[tokio::test(start_paused = true)]
    async fn dns_og_dmarc_hentes_i_baggrunden_foer_aarstallene() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        assert!(!ekstra(&koe, &lager, &f).await);
        for kald in [&f.dns_kald, &f.dmarc_kald] {
            let k = kald.lock().unwrap();
            assert_eq!(k.len(), 2 * AFSNIT_DAGE as usize);
            assert_eq!(k[0], ("a".to_string(), dag_noegle(NU - DOEGN_MS)), "nyeste først");
        }
        let log = f.log.lock().unwrap();
        let foerst = |n: &str| log.iter().position(|x| *x == n).unwrap();
        assert!(foerst("laek") < foerst("dns") && foerst("dns") < foerst("dmarc") && foerst("dmarc") < foerst("historik"));
    }

    /// En fejl i DMARC stopper hverken køen, de andre domæner, DNS eller årstallene. Den står ved domænet, og næste svar rydder den.
    #[tokio::test(start_paused = true)]
    async fn fejl_i_dmarc_koster_kun_afsnittet() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        f.dmarc_svar.insert("a".into(), Err(afvist()));
        assert!(!ekstra(&koe, &lager, &f).await);
        assert_eq!(kald_i(&f.dmarc_kald, "a"), 1, "domænet opgives efter første fejl");
        assert_eq!(kald_i(&f.dmarc_kald, "b"), AFSNIT_DAGE as usize);
        assert_eq!(kald_i(&f.dns_kald, "a"), AFSNIT_DAGE as usize);
        assert_eq!(f.historik_kald.lock().unwrap().len(), 2);
        assert!(lager
            .lock()
            .unwrap()
            .afsnit_fejl(DMARC_FEJL, "a")
            .unwrap()
            .is_some_and(|t| t.contains("does not have access")));
        assert_eq!(lager.lock().unwrap().afsnit_fejl(DNS_FEJL, "a").unwrap(), None);
        assert!(lager.lock().unwrap().hentet_ekstra("a").is_ok());
        f.dmarc_svar.clear();
        ekstra(&koe, &lager, &f).await;
        assert_eq!(lager.lock().unwrap().afsnit_fejl(DMARC_FEJL, "a").unwrap(), None);
    }

    /// Teksten vises i afsnittet, så et id eller token i Cloudflares svar må ikke stå i den.
    #[tokio::test(start_paused = true)]
    async fn fejlen_ved_afsnittet_er_renset() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        let id = "0123456789abcdef0123456789abcdef";
        f.dmarc_svar.insert(
            "a".into(),
            Err(Fejl::GraphQl {
                dag: None,
                detalje: format!("zone {id} does not have access"),
            }),
        );
        ekstra(&koe, &lager, &f).await;
        let t = lager.lock().unwrap().afsnit_fejl(DMARC_FEJL, "a").unwrap().unwrap();
        assert!(!t.contains(id) && t.contains("<TOKEN>"), "{t}");
    }

    /// Samme for DNS: DMARC hentes stadig.
    #[tokio::test(start_paused = true)]
    async fn fejl_i_dns_koster_kun_afsnittet() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        f.dns_svar.insert("a".into(), Err(afvist()));
        assert!(!ekstra(&koe, &lager, &f).await);
        assert_eq!(kald_i(&f.dns_kald, "a"), 1);
        assert_eq!(kald_i(&f.dmarc_kald, "a"), AFSNIT_DAGE as usize);
        assert!(lager.lock().unwrap().afsnit_fejl(DNS_FEJL, "a").unwrap().is_some());
    }

    /// Et døgn uden for Cloudflares opbevaring noteres som tomt, så der ikke spørges igen. Det er ikke en fejl.
    #[tokio::test(start_paused = true)]
    async fn dns_uden_for_opbevaringen_spoerges_der_ikke_om_igen() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.dns_svar.insert(
            "a".into(),
            Err(Fejl::GraphQl {
                dag: None,
                detalje: "cannot request data older than 2678400s".into(),
            }),
        );
        assert!(!ekstra(&koe, &lager, &f).await);
        assert_eq!(kald_i(&f.dns_kald, "a"), AFSNIT_DAGE as usize);
        assert_eq!(lager.lock().unwrap().hentet_dns("a").unwrap().len(), AFSNIT_DAGE as usize);
        assert_eq!(lager.lock().unwrap().afsnit_fejl(DNS_FEJL, "a").unwrap(), None);
    }

    fn kald_i(k: &Mutex<Vec<(String, String)>>, zone: &str) -> usize {
        k.lock().unwrap().iter().filter(|x| x.0 == zone).count()
    }

    #[tokio::test(start_paused = true)]
    async fn fejl_i_ekstra_roerer_ikke_koeens_status() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.ekstra_svar.insert("a".into(), Err(Fejl::Afvist(403)));
        let (slut, _) = koer(&koe, &lager, &f).await;
        assert_eq!(slut, Slut::Faerdig);
        ekstra(&koe, &lager, &f).await;
        let s = koe.status().unwrap();
        assert_eq!((s.tilstand, s.fejl), (Fase::Faerdig, None));
    }

    #[tokio::test(start_paused = true)]
    async fn forbigaaende_fejl_gemmer_resten_til_naeste_runde() {
        for fejl in [Fejl::Vent, Fejl::CfNede(503), Fejl::Forbindelse] {
            let koe = Koe::default();
            let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
            let mut f = falsk(&koe);
            f.ekstra_svar.insert("a".into(), Err(fejl.clone()));
            assert!(!ekstra(&koe, &lager, &f).await);
            assert_eq!(f.ekstra_kald.lock().unwrap().len(), 1, "ingen flere kald lige nu: {fejl:?}");
            assert_eq!(lager.lock().unwrap().adgang_fejl("a").unwrap(), None, "det er ikke domænets skyld");
            assert!(!koe.stoppet());
        }
    }

    #[tokio::test(start_paused = true)]
    async fn doegn_uden_for_opbevaringen_spoerges_der_ikke_om_igen() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.ekstra_svar.insert(
            "a".into(),
            Err(Fejl::GraphQl {
                dag: None,
                detalje: "cannot request data older than 2678400s".into(),
            }),
        );
        assert!(!ekstra(&koe, &lager, &f).await);
        assert_eq!(lager.lock().unwrap().hentet_ekstra("a").unwrap().len(), 30);
        assert_eq!(lager.lock().unwrap().adgang_fejl("a").unwrap(), None);
    }

    /// Årstallene hentes sidst i den langsomme runde, ét kald for hele året, og kun nye døgn bagefter. Den normale runde rører dem ikke.
    #[tokio::test(start_paused = true)]
    async fn aaret_hentes_sidst_og_kun_en_gang() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        assert_eq!(koe.runde(&lager, &f, &|_: &KoeStatus| {}, &|| NU).await, Slut::Faerdig);
        assert!(f.historik_kald.lock().unwrap().is_empty(), "ikke i den normale runde");
        f.log.lock().unwrap().clear();
        assert!(!ekstra(&koe, &lager, &f).await);
        let log = f.log.lock().unwrap().clone();
        assert_eq!(
            log[..3],
            ["tilstand", "tilstand", "fund"],
            "tilstanden først: den er lille, og så fundene, én gang for alle domæner"
        );
        assert_eq!(
            log[log.len() - 4..],
            ["historik", "historik", "besoeg", "besoeg"],
            "årstallene og så besøgene sidst"
        );
        assert!(log.iter().filter(|x| **x == "historik").count() == 2 && log.contains(&"laek") && log.contains(&"angreb"));
        let kald = f.historik_kald.lock().unwrap().clone();
        assert_eq!(kald[0].1, dag_noegle(NU - historik::HISTORIK_DAGE * DOEGN_MS));
        assert_eq!(kald[0].2, dag_noegle(NU - DOEGN_MS));
        f.log.lock().unwrap().clear();
        assert!(!ekstra(&koe, &lager, &f).await);
        assert_eq!(
            *f.log.lock().unwrap(),
            ["tilstand", "tilstand", "fund"],
            "alt er hentet: kun tilstanden og fundene (som selv holder timen)"
        );
    }

    /// Besøgene: ét kald pr. domæne for de 30 færdige døgn, bagefter kun det nye. Afviser Cloudflare dem for et domæne, står det ved domænet,
    /// de andre domæner og køen kører videre, og der spørges ikke igen, før programmet starter igen (en ny `Koe`). Et svar rydder tilstanden.
    #[tokio::test(start_paused = true)]
    async fn afvist_besoeg_giver_tilstand_og_stopper_ikke_koeen() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        f.besoeg_svar.insert(
            "a".into(),
            Err(Fejl::GraphQl {
                dag: None,
                detalje: "unknown field visits".into(),
            }),
        );
        assert!(!ekstra(&koe, &lager, &f).await);
        assert!(!koe.stoppet());
        assert!(
            lager.lock().unwrap().afsnit_fejl(BESOEG_FEJL, "a").unwrap().is_some(),
            "ikke tilgængelig"
        );
        assert_eq!(lager.lock().unwrap().afsnit_fejl(BESOEG_FEJL, "b").unwrap(), None);
        assert_eq!(
            lager.lock().unwrap().hentet_besoeg("b").unwrap().len(),
            besoeg::BESOEG_DAGE as usize
        );
        let b = f.besoeg_kald.lock().unwrap().iter().find(|k| k.0 == "b").cloned().unwrap();
        assert_eq!(
            (b.1, b.2),
            (dag_noegle(NU - besoeg::BESOEG_DAGE * DOEGN_MS), dag_noegle(NU - DOEGN_MS)),
            "ét kald for de 30 døgn"
        );
        assert!(
            lager.lock().unwrap().hentet_historik("a").unwrap().len() > 300,
            "årstallene for a er hentet alligevel"
        );
        ekstra(&koe, &lager, &f).await;
        assert_eq!(f.besoeg_kald.lock().unwrap().len(), 2, "ikke igen før næste start, og b har alt");
        // Næste start: der spørges igen, og et svar rydder tilstanden.
        let ny = Koe::default();
        f.besoeg_svar.clear();
        ekstra(&ny, &lager, &f).await;
        assert_eq!(f.besoeg_kald.lock().unwrap().len(), 3);
        assert_eq!(lager.lock().unwrap().afsnit_fejl(BESOEG_FEJL, "a").unwrap(), None);
        // En forbigående fejl er ikke domænets skyld.
        let lager2 = lager_med(&[("c", "c.wi24rd.com", None)]);
        f.ekstra_lager = Some(&lager2);
        f.besoeg_svar.insert("c".into(), Err(Fejl::Forbindelse));
        assert!(!ekstra(&ny, &lager2, &f).await);
        assert_eq!(lager2.lock().unwrap().afsnit_fejl(BESOEG_FEJL, "c").unwrap(), None);
        f.besoeg_svar.clear();
        ekstra(&ny, &lager2, &f).await;
        assert_eq!(
            lager2.lock().unwrap().hentet_besoeg("c").unwrap().len(),
            besoeg::BESOEG_DAGE as usize,
            "næste runde henter dem"
        );
    }

    /// Unikke for de døgn, der var hentet før opdateringen: ét kald pr. domæne for de hentede døgn, én gang, og et domæne uden årstal spørges ikke.
    #[tokio::test(start_paused = true)]
    async fn unikke_hentes_igen_en_gang() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        let gamle: Vec<String> = (3..=200).map(|i| dag_noegle(NU - i * DOEGN_MS)).collect();
        lager.lock().unwrap().gem_historik("a", &gamle, &[], "t").unwrap();
        assert!(lager.lock().unwrap().unikke_en_gang("2026-11-03").unwrap());
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        assert!(!ekstra(&koe, &lager, &f).await);
        let kald = f.unikke_kald.lock().unwrap().clone();
        assert_eq!(kald.len(), 1, "kun a, og kun én gang: {kald:?}");
        assert_eq!(
            (kald[0].1.as_str(), kald[0].2.as_str()),
            (dag_noegle(NU - 364 * DOEGN_MS).as_str(), dag_noegle(NU - DOEGN_MS).as_str()),
            "året, efter at de manglende døgn er hentet"
        );
        let log = f.log.lock().unwrap().clone();
        let i = |x: &str| log.iter().position(|y| *y == x).unwrap();
        assert!(i("historik") < i("unikke") && i("unikke") < i("besoeg"), "{log:?}");
        ekstra(&koe, &lager, &f).await;
        assert_eq!(f.unikke_kald.lock().unwrap().len(), 1, "én gang");
        assert!(!lager.lock().unwrap().unikke_mangler("a").unwrap());
    }

    /// En fejl i kaldet stopper ikke resten, og kun valgte domæner spørges.
    #[tokio::test(start_paused = true)]
    async fn en_fejl_i_zonens_tilstand_stopper_ikke_koeen() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None), ("b", "b.wi24rd.com", None)]);
        lager.lock().unwrap().gem_valgte(&["a".to_string()]).unwrap();
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        f.tilstand_svar = Err(Fejl::Svar);
        assert!(!ekstra(&koe, &lager, &f).await);
        assert_eq!(*f.tilstand_kald.lock().unwrap(), ["a"]);
        assert!(!f.ekstra_kald.lock().unwrap().is_empty() && !f.historik_kald.lock().unwrap().is_empty());
    }

    /// Skema 12 henter angrebene igen. Har Cloudflare ikke døgnet længere, bliver det gemte stående, og der spørges ikke igen.
    #[tokio::test(start_paused = true)]
    async fn angreb_uden_for_opbevaringen_sletter_ikke_det_gemte() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None)]);
        let dag = dag_noegle(NU - 20 * DOEGN_MS);
        lager.lock().unwrap().gem_valgte(&["a".to_string()]).unwrap();
        let gemt = [("angreb".to_string(), "k".to_string(), 3)];
        lager.lock().unwrap().gem_angreb("a", &dag, &gemt, "t").unwrap();
        lager.lock().unwrap().db_ryd_hentet_angreb();
        let mut f = falsk(&koe);
        f.angreb_svar = Err(Fejl::GraphQl {
            dag: None,
            detalje: "cannot request data older than 2678400s".into(),
        });
        assert!(!ekstra(&koe, &lager, &f).await);
        let s = lager.lock().unwrap();
        assert_eq!(s.raekker("a", &dag).unwrap().len(), 1);
        assert!(s.hentet_angreb("a").unwrap().contains_key(&dag));
    }

    #[tokio::test(start_paused = true)]
    async fn ny_runde_afbryder_de_ekstra_tal_og_tempoet_holdes() {
        let koe = Koe::default();
        let lager = lager_med(&[("a", "a.wi24rd.com", None)]);
        let mut f = falsk(&koe);
        f.ekstra_lager = Some(&lager);
        f.ekstra_vaek = true;
        assert!(
            koe.ekstra_runde(&lager, &f, &|| NU, EKSTRA_TEMPO, &|_| {}).await,
            "vækket: en ny runde kører straks"
        );
        assert_eq!(f.ekstra_kald.lock().unwrap().len(), 1);
        f.ekstra_vaek = false;
        let t0 = tokio::time::Instant::now();
        assert!(!koe.ekstra_runde(&lager, &f, &|| NU, EKSTRA_TEMPO, &|_| {}).await);
        assert_eq!(
            t0.elapsed(),
            EKSTRA_TEMPO * (29 + 30 + 30 + 31 + 31 + 1 + 1),
            "de 29 døgn, der var tilbage, de 30 med enkelthændelser, de 30 med lækkede kodeord, de 31 med DNS, de 31 med DMARC, årstallene og besøgene"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn sov_vaagner_ved_prioritet() {
        let koe = Koe::default();
        let t0 = tokio::time::Instant::now();
        let (vaekket, _) = futures_util::future::join(koe.sov(RUNDE), async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            koe.prioriter(Some("a".into()));
        })
        .await;
        assert_eq!(t0.elapsed(), Duration::from_secs(5), "ny runde straks, ikke efter en time");
        assert!(vaekket);
        assert!(!koe.sov(TIK).await, "et tik uden prioritet: ikke vækket");
        assert_eq!(koe.prioritet().as_deref(), Some("a"));
    }
}
