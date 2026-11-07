# Changelog

Udgivelser er endnu ikke tagget. Posterne står efter dato, nyeste først.

## [Unreleased]

### 2026-11-07 23:30 — Verdenskort med farver og ét Tab-stop pr. domæne
#### Tilføjet
- Verdenskortet i Angreb og dyk har hav, land og landegrænser som et rigtigt kort i lyst og mørkt tema.
- Lande fyldes i fem trin efter antal angreb. Trinene fordeles efter de viste tal, så ét land med alt ikke gør de andre usynlige.
- Forklaringen under kortet viser skalaen med tal ved enderne og siger, at landet er det, Cloudflare så angrebet komme fra, og at det kan være en VPN eller proxy.
- Et land med angreb kan klikkes med samme mål som ringen og viser land og tal, når musen står på det.
#### Ændret
- Cloudflares datacentre står som svage blå prikker, tydeligt svagere end det datacenter, der stoppede angrebet.
- Mørkt tema: havet er lysere og blåere, så hav og land skilles klart. Skalaen, buerne, prikkerne og gitteret følger med, og her betyder lysere rød flere angreb.
- Rækkens handlinger på `#teknologi` står bag én knap "⋯": ét Tab-stop pr. domæne. Piletasterne flytter i menuen, også i kolonnerne, og Esc lukker.
- Glorien om ringene er en smal streg, der ikke vokser ved hover eller fokus.
#### Rettet
- Kortets usynlige klikflader males ikke længere af musen. Den store mørke skive om ringen og det brede bånd langs buen er væk, og mærket lyser op i stedet.

### 2026-11-07 23:30 — Klart sprog i Vurdering og bedre login-forslag
#### Ændret
- Vurdering hedder "Hvorfor fik dagen (ikke) et mærke?" og samler alle tre mærker med ja, nej eller "ja, foreløbigt".
- Grænser står som hele tal, rundet op. Er der intet at sammenligne med, står betingelsen uden ✓ eller ✗. Mærkernes logik er uændret.
- `none` fra Cloudflares tjek står som "Ikke et login-forsøg (intet kodeord sendt)" og tæller stadig aldrig som læk.
- Login-ordene matcher som hele ord og stidele, også i flertal, så `/badminton` og `/accounting` ikke foreslås.
- Ord for login-handlinger er tilføjet: `authenticate`, `authorize`, `oauth`, `token`, `sso`, `saml`, `2fa`, `mfa`, `otp`, `password`, `passwd` og `reset`.
- Det foreløbige mærke har samme tekst overalt: "Foreløbig — afgøres, når dagen efter er hentet".
- Teknologinavne staves ens på regelkortet, i listen, på `#teknologi` og i Overblik.
- Linjen under "Mails sendt i dit domænes navn" gentager ikke overskriften.
#### Rettet
- Stier, der kun ligner et login på navnet, foreslås ikke: `/wp-admin/admin-ajax.php`, `/wp-admin/admin-post.php`, `/author`, `/authors` og `/wp-json/…/users`. Har tjekket set et kodeord på stien, kan den stadig foreslås.
- Statuslinjen i menuen skelner "Ligner DNS-rekognoscering" fra "Ligner rekognoscering".

### 2026-11-06 23:30 — DNS, DMARC og lækkede kodeord
#### Tilføjet
- "Mails sendt i dit domænes navn" i Angreb, fra `dmarcReportsSourcesAdaptiveGroups`: tal, afsendere, der fejlede DMARC, og en tom tilstand med guide. Hold øje ved en ny afsender, der fejler, eller en markant større andel. Ingen IP-adresser.
- Mærket "Ligner DNS-rekognoscering" fra `dnsAnalyticsAdaptiveGroups`, med ✓/✗ i Vurdering. Følger notifikationen for "Ligner rekognoscering".
#### Ændret
- Login-forslag kræver, at stien ligner et login, eller at Cloudflares tjek har set et kodeord. En sti i hændelser fra en kendt regel foreslås aldrig.
- Lækkede kodeord hentes uden krav om POST, så Basic auth med GET tæller, og fundene deler ikke længere grænse med andre svar. Skema 15 henter døgnene igen.
- Kolonnehandlingerne på `#teknologi` står bag én knap "⋯": ét Tab-stop pr. kolonne.
#### Rettet
- `none` fra Cloudflares tjek står ikke længere som "ukendt". Tomme svar har deres egen linje. Ingen af dem tæller som læk.
- Testene kører på et fast ur (`HM_NU`) og afhænger ikke af dagens dato.

### 2026-11-06 23:30 — Teknologi-siden, Hent nu og Cloudflares kvote
#### Tilføjet
- Siden "Teknologi pr. domæne" (`#teknologi`): en tabel med en radiogruppe pr. domæne og teknologi, kolonne- og rækkehandlinger, søgning og fanen "Alle teknologier", hvor man vælger, hvad der vises, og tilføjer teknologier på forhånd.
- "Hvad angrebene var rettet mod" i Angreb og dyk: alle angrebne teknologier med valget som mærke. Hverken "Bruger ikke" eller en skjult kolonne fjerner noget.
- Teknologier hentes fra Cloudflares regelnavne, så hvert produkt, Cloudflare navngiver, kommer med af sig selv.
- Tempoet følger Cloudflares `Ratelimit`- og `Ratelimit-Policy`-headere, og Indstillinger → Data viser kvoten.
#### Ændret
- "Hent nu" siger straks "Henter …" og bagefter, hvad der skete.
- Hele rækken i Overblik er ét mål: et klik åbner Angreb. Kun tal og små links går andre steder hen. Store flader får knap-look uden understregning.
- Lækkede kodeord hentes uden sti-filter, og blokken står altid i Login. Skema 14 henter de 30 døgn igen.
- "Dit sites teknologi" i Angreb og dyk er kort: valgene, ét forslag og knappen "Ret teknologier".
- "Nulstil historik" siger med datoer, hvad der forsvinder for altid.
#### Rettet
- "Trafik fra" og "Hele historikken" regnes fra dage med trafiktal, ikke fra hentede dage. Datoer i et andet år står med året.
- Forsiden og stier, hvor en kendt regel stoppede flertallet af POST, foreslås ikke længere som login-sti.

### 2026-11-06 23:30 — Vurdering, et års trafik og domænets tilstand
#### Tilføjet
- "Vurdering" i Angreb og i dyk for én dag: hver betingelse for "Ligner målrettet" og "Ligner rekognoscering" med ✓ eller ✗ og tallet.
- Et års trafik fra `httpRequests1dGroups`: "Hele historikken" og datovælgeren går et år tilbage for trafiktal, og sikkerhedstal siger tydeligt, hvor langt de går. Indstillinger → Data viser begge datoer.
- Login viser forsøg med kodeord fra kendte læk pr. dag og sti. Mindst Hold øje, og Handl, hvis serveren svarede 2xx eller 302. Stien når aldrig en notifikation.
- Overblik og notifikationer siger, når et domæne er på pause, ikke er aktivt, er mistænkt for phishing eller står i udviklertilstand, med guide i Cloudflares ord.
#### Ændret
- "Ligner målrettet": netværket må røre de andre valgte domæner ±1 dag med højst 10 % af det, det gjorde her. Ét enkelt hit andre steder fjernede før mærket.
#### Rettet
- Trafikteksten sagde, at de stoppede ikke tæller som angreb. De stoppede er angreb; det er de øvrige, der ikke tæller. Har Cloudflare stoppet mindst halvdelen, står det.

### 2026-11-05 23:30 — Teknologi pr. domæne og stram CSP
#### Tilføjet
- Tabellen "Teknologi pr. domæne" under Indstillinger → Domæner: et klik på et felt åbner "Dit sites teknologi" med teknologien i fokus. Den erstatter linjen under hvert domæne.
- Klik på "Ligner målrettet" viser nu de gemte angreb: netværket står i enkelthændelserne. Døgn, Cloudflare stadig har, hentes igen i baggrunden.
#### Ændret
- Knapperne i "Dit sites teknologi" står ens i alle rækker.
- Overblik ruller i sin egen boks i smalle vinduer, med domænet fastlåst, og hele vinduet ruller ikke længere med.
- "Huginn gemmer de 200 seneste angreb pr. dag" i stedet for at give Cloudflare æren.
#### Rettet
- Et afgjort "Ligner målrettet" går ikke tabt, hvis programmet var slukket den dag: op til 7 døgn tilbage vurderes, med én besked pr. mærke.
#### Sikkerhed
- CSP uden `'unsafe-inline'`: `style-src 'self'`.

### 2026-11-05 23:30 — Teknologivalg og gemte angreb
#### Tilføjet
- Teknologi-valg kan ses og fortrydes: linje pr. domæne under Indstillinger → Domæner, "Nulstil" pr. teknologi, pr. domæne og for alle, og "som du ikke bruger" i Hvorfor er et link.
- Op til 200 egentlige angreb gemmes pr. domæne og døgn; "De seneste" i dyk viser dem 25 ad gangen med "Viser N af M angreb". Gamle døgn hentes i baggrunden, kun for valgte domæner. Ingen IP-adresser og ingen brugeragent.
- Menulinjen siger "3 nye siden kl. 23.30".
#### Ændret
- "Ligner målrettet" sammenligner kun med valgte domæner og kræver mindst 3. Beskeden kommer først, når døgnet efter er hentet; indtil da står mærket som foreløbigt.
- "Første gang i døgnet", Rolig og "i går" følger din lokale tid; tallene er stadig hele døgn i UTC.
- Hvorfor står på højst 3 linjer, resten bag "mere".

### 2026-11-05 23:30 — Mærker, datoformat og API Token
#### Tilføjet
- Mærkerne "Ligner rekognoscering" og "Ligner målrettet" oven på status, i Overblik, Siden sidst, Angreb og menulinjen, med guide og notifikation.
- Datoformat under Indstillinger → Generelt (Følg sproget, Følg systemet, ISO) og en kalender, der følger det.
- "Opret API Token hos Cloudflare" med navn og rettigheder udfyldt, trin for trin, og fejl, der siger, hvilken rettighed der mangler.
- "Forespørgsler i alt" og en gul linje ved usædvanlig meget trafik i dyk.
#### Ændret
- Ét fælles domænevalg for Angreb, Robotter, Login og dyk.
- Dyk bygges som Angreb: fliser, fordeling, kort, søjler og regler, lister og "De seneste" sidst. Fra og Til viser altid udsnittet.
- "Ny AI-robot" bygger på hele døgn; et udsnit med tal kun i dag siger det.
- Start ved login slået fra står med rødt banner og en knap.

### 2026-11-05 23:30 — Dit sites teknologi og trafik-dyk
#### Tilføjet
- "Dit sites teknologi" pr. domæne: Bruger vi, Bruger vi ikke, Ved ikke og Giv mig besked, med sikkerhedsnet og forslag ud fra data.
- Trafik-dyk: hvor en dags forespørgsler kom fra. Datovælger i dyk, Robotter og Login.
- Huller i historikken vises som huller, aldrig som 0, med linje i Overblik og Siden sidst.
- Kilder til angreb mod landet og forstyrrelser for verden.
#### Ændret
- Usædvanlig meget trafik giver Hold øje og påstår ikke længere, at det ikke er et angreb.

### 2026-11-05 23:30 — Er sitet oppe?
#### Tilføjet
- "Er sitet oppe?": "Din server svarer ikke" og "Usædvanlig meget trafik" med guide om DNS-poster.
- Regelkort i dyk.
#### Ændret
- <Land> lige nu ser landet fra målsiden.

### 2026-11-04 23:30 til 2026-11-05 23:30 — Velkomst, domænevalg og egne login-stier
#### Tilføjet
- Velkomst med logo, hvad du kan forvente, sikkerheden og Wi24rds filosofi.
- Domæner er et tilvalg under Indstillinger → Domæner; nye domæner venter på et svar.
- Egne login-stier.
#### Ændret
- Ét land overalt med forslag ud fra tidszonen. "API Token" er ordet alle steder. "Kommer snart" er væk.
- Dyk kan ikke forveksles med Angreb: egen overskrift, knap tilbage og mærket Udsnit.
#### Rettet
- Kontrast, margen, ental og flertal, båndets farve og knapgrupper.

### 2026-11-04 23:30 — Land og Verden fra Cloudflare Radar
#### Tilføjet
- <Land> lige nu og Verden fra Cloudflare Radar, med sammenligningen i Hvorfor: "følger den globale tendens" eller "stigningen er din egen".

### 2026-11-04 23:30 — Robotter og Login
#### Tilføjet
- Robotter og Login som visninger, med de ekstra tal, der hentes i baggrunden for færdige døgn.

### 2026-11-04 23:30 — Rettelser i dyk
#### Rettet
- Startrækkefølge, lange domænenavne og stier.

### 2026-11-04 23:30 — Dyk-visningen
#### Tilføjet
- Dyk-visningen: klik på en regel, et land, et netværk, en sti, et datacenter, en handling eller en dag og se fordeling, forløb pr. dag, lille kort, top 5/20 og samme filter på andre domæner; filtre kædes frit; periode 7/30 hele dage eller hele historikken; Alt / Kun angreb / Kun rutine.
- Alt med data bag er klikbart (Overblik, Siden sidst, Angreb, dyk, menulinjen), med hover, tastatur (ét Tab-stop + piletaster i kort og søjler) og tilbage/frem.
- Nyt logo: to ravne på en pulslinje (bånd, Om, opsætning, app-ikon, menulinje).
- Overblik-tabellen med overskrifterne Afvist og Angreb.
#### Ændret
- "Afvist" tæller ikke længere sider, der blev leveret med skjulte fælde-links (AI Labyrinth `link_maze_injected`); de vises som egen linje.
- "30 dage" betyder 30 hele dage overalt; kortet i Angreb viser alle lande og datacentre.
- Regelkort: rutine i klarsprog og grå, egen regel som »navn«, ingen dobbelt CVE; "Værnet" fjernet fra brugervendt tekst.
- Tekst i kort og søjler følger Tekststørrelse.
#### Rettet
- Stier med `%` og netværk med `&` blev ødelagt i adressen.

### 2026-11-04 23:30 — Tekststørrelse og Nulstil historik
#### Tilføjet
- Tekststørrelse Lille / Normal (15 px, standard) / Stor under Indstillinger → Generelt og med ⌘+ / ⌘− / ⌘0; én tekstskala i tokens.
- Nulstil historik under Indstillinger → Data med dialog, der siger, hvad der går tabt.
- "Edge – første forsvarslinje" i Angreb og hjælpen; kortets legende forklarer "◆ i en ring".
#### Ændret
- Valg pr. domæne: Følg standard · Alvorligt og hold øje · Kun alvorligt · Aldrig.
- Indstillinger-rækker ombrydes i stedet for at klemme etiketten.
- README: "adgangsnøgle" i stedet for "token".
#### Rettet
- "Glem nøglen…" viste dialogens tekst, når flere domæner var valgt (flertalsmekanismen).
- Tooltip midt på kortet fjernet.

### 2026-11-04 23:30 — Programramme, Indstillinger og Overblik
#### Tilføjet
- Programramme: blåt Wi24rd-bånd som titel- og værktøjslinje (trafiklys, statuspille, Hent nu, hjælp, tilbage/frem, domænevælger i Angreb), sidebjælke (Overblik, Siden sidst, Angreb, Kommer snart), egen programmenu på macOS (da/en) med genveje.
- Indstillinger-siden: Generelt (sprog, udseende System/Lys/Mørk, start ved login), Notifikationer, Forbindelse (skift/glem adgangsnøglen med bekræftelse), Data (`data_status`), Om.
- Overblik med domslinje, status Handl/Hold øje/Roligt/Ukendt, "Hvorfor" i klarsprog og handlingsguiden "Hvad gør jeg?".
- CVE-numre åbner cve.org i browseren (`aabn_cve`); "Se de N angreb fra i går" filtrerer Angreb.
- Wi24rd UI-kit: `design/wi24rd-tokens.json` → `tokens.css` + XAML; WCAG-kontrast testet; lys og mørk visning.
- UI-røgtesten ligger i `tests/ui` (`npm run test:ui`).
#### Ændret
- Én side med hash-navigation i stedet for to HTML-sider; systemfont; Værnet hedder Angreb.
- "Angreb denne uge" farves kun ved stor stigning eller stort fald.
- Notifikationerne bruger samme tekst som Overblikket; menulinjen siger "dage".
#### Sikkerhed
- `core:menu:default` fjernet; kun `core:window:allow-start-dragging`, `allow-hent-nu`, `allow-aabn-cve`, `allow-data-status` tilføjet. CSP uændret.

### 2026-11-03 23:30 — Lokale notifikationer
#### Tilføjet
- Lokale notifikationer ved rødt (og gult under "Alt"), én samlet besked pr. runde og højst én pr. domæne pr. døgn. Knappen "Vis" åbner vinduet på "Siden sidst".
- Kortet "Siden sidst" øverst på forsiden og i menulinjen: røde og gule domæner med årsag i klart sprog, markeret "ny" siden sidst.
- Forudindstillingerne Rolig / Standard (forvalgt) / Alt i opsætningen og under Indstillinger, valg pr. domæne og "Giv besked ved rødt".
- Lokal database skema 5 (`notifikation_zone`, `meldt`).
#### Sikkerhed
- Webvisningen har ingen `notification:*`-rettigheder; kun Rust sender beskeder.

### 2026-11-03 23:30 — Menulinjen og start ved login
#### Tilføjet
- Ikon i menulinjen med Åbn Huginn · Siden sidst · Hent nu · statuslinje · Afslut (da/en).
- Programmet bliver i menulinjen, når vinduet lukkes; ⌘Q og Afslut afslutter.
- Start ved login (slået til én gang ved første start af en bundlet build; kan slås fra under Indstillinger).
- Kun én kørende kopi ad gangen.
- Sektionen Indstillinger; "Glem tokenet" er flyttet hertil.
#### Ændret
- Hentekøen tikker hvert minut på vægur og henter straks efter dvale.
- Tokenet huskes i hukommelsen, så nøgleringen ikke spørger hver time.
#### Rettet
- En nøgleringsfejl stoppede køen uden besked; nu står den i menulinjen, og "Hent nu" prøver igen.

### 2026-11-03 23:30 — Samlet Værn og årsager i Overblik
#### Tilføjet
- Det samlede Værn viser de valgte domæner som etiketter (8 + "+ N flere"); klik åbner domænets eget Værn.
- Status i overblikket viser årsagen: de største angrebstyper i går og hvordan de blev stoppet.
- Kolonnen "Angreb, uge" (angreb 7 døgn mod ugen før).
#### Ændret
- Topbjælke og hero flugter med indholdet på brede vinduer.
- Vinduets titel følger sidens titel (vinduet bygges i `setup`).
#### Rettet
- Ingen natlig gul: før i går er færdighentet (00–03 UTC), bygger status på døgnet før og står som "opdateres …".

### 2026-11-03 23:30 — Hentekø og Overblik
#### Tilføjet
- Hentekø i baggrunden for alle zoner, med zonen på skærmen først og fremskridt i UI'et.
- Fælles begrænser for alle GraphQL-kald (2 samtidige, 1,1 s mellem starter, `Retry-After`, kvote- og 5xx-pauser).
- Forsiden er et overblik: status grøn/gul/rød/grå (kun angreb tæller; link-labyrint, Bot Fight og browsertjek er rutine), afkrydsning gemt i SQLite (skema 3).
- Samlet Værn for valgte domæner.
#### Ændret
- Værnet siger "Stoppet" med fordeling (blokeret, udfordret, sendt i labyrinten) og bruger hele UTC-døgn.

### 2026-11-03 23:30 — Første version
#### Tilføjet
- Tauri 2-skrivebordsprogram: læse-token i OS'ets nøglering, Cloudflare-klient kun mod `api.cloudflare.com`, lokal SQLite-cache.
- Værnet porteret fra wi24rd.com, paritetstestet mod den oprindelige JavaScript.
- Virker uden net (zoner og tal fra cachen) og kobler selv på igen.
- Fejl som lukkede koder, oversat i UI'et (dansk/engelsk).
- MIT-licens og tredjepartslicenser for skrifter og kortdata.
