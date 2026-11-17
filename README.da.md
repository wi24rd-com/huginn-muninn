![Wi24rd Huginn and Muninn](docs/banner.jpg)
# Wi24rd Huginn and Muninn
<sub>for Cloudflare®</sub>

*Sikkerhed er ikke noget, man køber. Det er noget, man forstår.*

[![CI](https://github.com/wi24rd-com/huginn-muninn/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/wi24rd-com/huginn-muninn/actions/workflows/ci.yml)
[![Nyeste udgivelse](https://img.shields.io/github/v/release/wi24rd-com/huginn-muninn?label=udgivelse)](https://github.com/wi24rd-com/huginn-muninn/releases/latest)
[![Licens: Apache-2.0](https://img.shields.io/badge/licens-Apache--2.0-blue)](LICENSE)
![macOS på Apple silicon](https://img.shields.io/badge/macOS-Apple%20silicon-lightgrey)

### [⬇ Hent Huginn and Muninn 1.1.0 til macOS](https://github.com/wi24rd-com/huginn-muninn/releases/latest)
Signeret og notariseret af Apple · Apple silicon · macOS 11 eller nyere · åbn zip-filen, og træk programmet til Programmer

**Windows:** ikke udgivet endnu. En Windows-udgave er på vej.

Et skrivebordsprogram (Tauri), der viser, hvad der faktisk sker mod **dine egne** Cloudflare-domæner — i klart sprog, på tværs af alle dine zoner og med en lokal historik, der rækker længere end Cloudflares 31 dage.

- **Huginn (tanke):** det, der sker nu — angrebene, hvad der blev stoppet og hvordan.
- **Muninn (hukommelse):** det, der er sket — hver færdig dag gemmes lokalt.

**Privatliv:** programmet taler kun med `api.cloudflare.com` med dit eget API Token, der kun kan læse (ét pr. Cloudflare-konto, hvis du har flere), gemt i styresystemets nøglering. Intet sendes til Wi24rd. Ingen telemetri.

**Status:** version 1.1.0 til macOS (Apple silicon), signeret med Developer ID og notariseret af Apple. Hent den under [Releases](https://github.com/wi24rd-com/huginn-muninn/releases/latest). Windows-udgaven er ikke udgivet endnu, og der findes ingen Linux-udgave.

**Brugervejledning:** [Dansk](docs/manual/da/00-index.md) · [English](docs/manual/en/00-index.md)

## Programmet
- **Ét vindue:** øverst en titellinje med programmets navn, tilbage og frem, **Vejledning**, **?** (hjælp: forklaringerne til visningen og genvejene), tandhjulet (**Indstillinger**) og **i** (Om). Under den står fanerne og et bånd med knapperne til den visning, du står i, med **Hent nu** først. Fodlinjen nederst viser status ("Henter 3 af 7", "Opdateret 08.01") og versionen.
- **Fanerne:** **Overblik** · **Siden sidst** · **Angreb** · **Robotter** · **Login** · **Danmark lige nu** (navnet følger dit land) · **Besøgende**. Til venstre står listen **Domæner**, hvor du vælger ét domæne eller **Alle domæner**. Første gang åbner en guide med fire trin (Velkommen, Dit API Token, Dine domæner, Besked); uden API Token kan fanerne ikke bruges.
- **Indstillinger** er et vindue bag tandhjulet (⌘,): Udseende og sprog, Dato og land, Start, Notifikationer, Forbindelse, Data og Første start.
- **Flere Cloudflare-konti:** under **Indstillinger** › **Forbindelse** tilføjer du ét API Token pr. konto med **Tilføj API Token…**, side om side. Domænerne fra dem alle vises samlet. Hver forbindelse har kontoens navn hos Cloudflare; du kan **Omdøb…**, **Skift API Token…** og **Glem…** hver for sig uden at røre de andre. Afviser Cloudflare ét API Token, henter de andre videre, og Overblik siger, hvilket der blev afvist.
- **Domæner** er et tilvalg. Når du har forbundet, er ingen domæner valgt, medmindre API Tokenet ser præcis ét, som så vælges for dig. Med **Vælg domæner** i båndet på Overblik vælger du dem, Huginn skal hente og vise. Et domæne, du fravælger, hentes ikke længere, vises ingen steder og giver ingen notifikationer; dets tal bliver liggende, og vælger du det igen, hentes det, der mangler, højst de seneste 30 dage. Et nyt domæne på kontoen er ikke valgt: en diskret linje i Overblik tilbyder **Vælg** eller **Ikke nu**.
- **Overblik:** en domslinje øverst ("1 domæne kræver handling" / "Intet at gøre") og én række pr. domæne med status (**Handl**, **Hold øje**, **Roligt**, **Ukendt**), hvorfor, afviste forespørgsler (i går, 7 og 30 dage) og angreb denne og sidste uge. Klik på en række (eller Enter) for at se domænets angreb; **Angreb for alle** viser dem samlet.
- **Besøgende:** unikke besøgende et år tilbage (pr. uge, når perioden er over 90 dage), besøg pr. land, kendte robotter og mest besøgte sider (404'erne for sig), med blå søjler og højst ét filter ad gangen.
- **Subdomæner:** Besøgende og Angreb viser for ét domæne besøg, forespørgsler og angreb pr. subdomæne (fx `www` og `shop`), og et klik viser visningen for det subdomæne. Kun trafik gennem Cloudflare, og API Tokenet skal ikke have flere rettigheder.
- **Drift** (⌘8): om dit website svarede, med fejl pr. dag og pr. time ("mellem kl. 01.00 og 05.00 kunne Cloudflare ikke nå din server"), og hvor meget Cloudflare klarede fra sin cache uden at spørge din server. Der kræves ikke flere rettigheder.
- **Teknologi pr. domæne** (**Ret teknologier** i båndet på Angreb) er en tabel med et valg pr. domæne og teknologi: Bruger, Bruger ikke eller Ved ikke. Hver kolonne og hvert domæne har sine handlinger bag én knap **⋯**. Du kan søge, og under **Alle teknologier** vælger du, hvad der vises, og tilføjer teknologier på forhånd.
- **Genveje:** macOS ⌘1 Overblik · ⌘2 Siden sidst · ⌘3 Angreb · ⌘4 Robotter · ⌘5 Login · ⌘6 Danmark lige nu · ⌘7 Besøgende · ⌘, Indstillinger · ⌘R Hent nu (også i menuen **Vis**). Windows: de samme med Ctrl, og F5 = Hent nu.
- **CVE-numre** (fx CVE-2025-55182 i Angreb og i "Hvad gør jeg?") kan klikkes: de åbner CVE'ens side på cve.org i din browser. Programmet selv henter intet derfra.
- **Lys og mørk** følger systemet, eller du vælger under **Indstillinger**. Farver og skrifter kommer fra Wi24rd UI Kit 2 (`ui/assets/kit2/`).
- **Tekststørrelse:** Lille, Normal eller Stor under **Indstillinger**, eller ⌘+ ⌘− ⌘0 (Windows: Ctrl). Valget huskes.
- **Nulstil historik** under **Indstillinger** › **Data** sletter alle tal på din computer og henter dem forfra. Cloudflare har kun sikkerhedshændelser for den seneste måned og trafiktal for det seneste år — alt ældre er væk for altid; dialogen siger hvilke datoer, og hvor mange dage der forsvinder.

## Angreb
- **Afviste som Cloudflares estimat:** antallet af afviste er summen af Cloudflares stikprøve-vægt, ikke antallet af rækker, så det kan være højere end på wi24rd.com.
- **Én side pr. domæne:** alt om angrebene står på samme side, og overskriften siger fx "Angreb · wi24rd.com". Siden viser 7 hele dage som standard. Vælg **30 dage**, **Hele historikken** eller **Fra** og **Til** i båndet.
- **Filtre:** et klik på et tal, et land, en regel eller en sti forudindstiller perioden og filtret på samme side. Filtrene står som chips under overskriften: × fjerner ét, og **Nulstil** i båndet fjerner dem alle. **Vis** skifter mellem **Alt**, **Kun angreb** og **Kun rutine**. Tilbage og frem går ét trin ad gangen.
- **Alle afsnit følger perioden og filtrene.** Et afsnit, der ikke kan filtreres sådan, står ikke, og en linje siger hvilke. Med et filter vises de gemte tal, og intet hentes fra Cloudflare.
- **Trafik den …:** en dag med usædvanlig meget trafik åbner et afsnit øverst på siden med forespørgslerne pr. land, svarkode og robot.
- **Verdenskortet** viser hav og land og fylder hvert land i fem trin efter antal angreb, med skalaen under kortet. Landet er det, Cloudflare så angrebet komme fra, og det kan være en VPN eller proxy. En bue går fra landet til det datacenter, der stoppede angrebet, og Cloudflares øvrige datacentre står som svage blå prikker. Klik på et land, en ring eller en bue for at se angrebene bag. De mindste lande har intet omrids på kortet og får kun en ring.
- **Mails sendt i dit domænes navn:** Angreb viser for ét domæne, hvor mange mails der blev sendt i domænets navn, hvor mange der fejlede DMARC, og fra hvilke afsendere. Det kræver, at DMARC Management er slået til i Cloudflare. En ny afsender, der fejler, eller en markant større andel giver Hold øje. Der gemmes aldrig IP-adresser.
- **Følsomme filer, der svarede 200:** for ét domæne 39 faste stier, der aldrig bør svare 200 (fx `/.env` og `/.git/config`), med svarenes størrelse. Et svar, der ligner filen, giver Handl; et, der ligner sidens almindelige side (mange sites svarer med den på alle adresser), ændrer intet. Intet ekstra kald til Cloudflare.
- **Cloudflares egne fund:** for ét domæne det, Cloudflares egen skanning af opsætningen har fundet (Security Insights), med alvor. Et klik åbner det hos Cloudflare. Det kræver én valgfri rettighed mere på API Tokenet, *Account Security Center Insights › Read*, i en ny politik for kontoen (Add new Account policy); uden den virker resten. Et kritisk fund giver Hold øje.
- **Ligner DNS-rekognoscering:** et mærke, når nogen spørger efter mange navne under domænet, der ikke findes. Det følger notifikationen for "Ligner rekognoscering".
- **Vurdering:** for én dag kan du folde **Hvorfor fik dagen (ikke) et mærke?** ud. Hvert af de tre mærker siger ja, nej eller "ja, foreløbigt", med hver betingelse og tallet. Grænser står som hele tal, og en betingelse uden noget at sammenligne med får hverken ✓ eller ✗.

## Dit land lige nu og Verden
- **Danmark lige nu** (⌘6; navnet følger det land, du vælger) viser angrebene på dine domæner ved siden af verden fra Cloudflare Radar, som et indeks, hvor 100 er periodens gennemsnit. Kurverne viser stigninger og fald, ikke antal.
- Angreb vises fra målsiden, for et kildeland kan skyldes VPN eller lejede servere. **De største kilder til angreb mod Danmark** står som andel af angrebene mod landet ved siden af **Hvor angreb mod dine domæner kom fra** for de seneste 7 dage eller 4 uger, og lande, der fylder meget mere hos dig, er mærket "Særligt hos dig".
- **Danmark som mål** viser landets andel af alle angreb, Cloudflare så, som tal og dag for dag. Det er en målt andel, ikke et antal.
- **Et års trafik:** Huginn henter et års daglige trafiktal for hvert valgt domæne. **Hele historikken** og datovælgeren i Angreb går så langt tilbage for trafik. Cloudflare gemmer sikkerhedshændelser i en måned, og det står tydeligt, hvor de begynder.
- **Domænets tilstand:** Overblik og notifikationerne siger det, hvis et domæne er sat på pause, ikke er aktivt, er mistænkt for phishing eller står i udviklertilstand.
- **Internetforstyrrelser** viser det, Radar har registreret i landet de seneste 30 dage, og fanen Verden viser dem for hele verden.
- **Verden** er en fane med de mest ramte lande, flest angreb fra, hvad der stoppede dem, robottrafik, kvantesikre forbindelser og AI-robotter efter formål og navn.
- Landet tages fra din tidszone første gang, derefter fra systemets sprog, og du kan skifte det i båndet eller under Indstillinger. Kommer mindst halvdelen af dine besøgende tydeligt fra et andet land, tilbyder Indstillinger **Skift til** det land; robotter og trafikspring tæller ikke med.
- I Overblik kan **Hvorfor** sige "Følger den globale tendens" eller "Verden var normal i går, så stigningen er din egen" (kun når dit døgn er mindst tre gange dit snit). Det sammenligner WAF-angreb generelt, aldrig en bestemt regel, og det ændrer ikke status.
- Radar kræver ingen ekstra rettighed, og en fejl hos Radar rammer kun disse visninger. Kilde: Cloudflare Radar, CC BY-NC 4.0.

## Robotter og Login
- **Robotter** (⌘4) viser, hvem der læser dit site, i syv grupper fra Cloudflares verificerede kategorier: AI-træning, AI-søgning og assistenter, Søgemaskiner, Sociale medier og link-previews, SEO-værktøjer, Overvågning og Ukendt. Hver gruppe siger, hvad der kan ske, hvis du blokerer den. Blokering laver du selv hos Cloudflare: programmet læser kun.
- Du ser også de mest crawlede sider, de udklædte (noget, der kalder sig fx Googlebot uden at være det), hvilke robotter der blev stoppet ved Edge, og hvilke der blev lokket ind i labyrinten. Labyrinten stopper ikke robotten; den viser falske sider.
- **Login** (⌘5) viser, hvad der skete på de login-stier, du selv slår til pr. domæne, med **Ret stier** i båndet. Der er en fast startliste og forslag fra dine egne data, men programmet gætter aldrig på, hvad sitet kører. En sti foreslås kun, hvis navnet har et login-ord som helt ord (`login`, `oauth`, `password` og lignende), eller Cloudflares tjek har set et kodeord på den. Stier, der bare ligner et login, fx `admin-ajax.php`, tæller ikke på navnet, og en sti i hændelser fra en kendt regel foreslås aldrig.
- **Kodeord fra kendte læk:** Login viser pr. dag og sti, hvor mange login-forsøg der brugte et kodeord fra et kendt læk. Det giver mindst Hold øje, og Handl, hvis din server svarede 2xx eller 302. Kodeord gemmes aldrig, og stien står aldrig i en notifikation. En forespørgsel uden kodeord står som "Ikke et login-forsøg (intet kodeord sendt)" og tæller aldrig som læk.
- Login har tre tal fra hver sin kilde, som aldrig lægges sammen: **stoppet ved Edge**, **fik 404** og **nåede din server**. Svar 200 eller 302 fra din server kan være et login, men kan lige så godt være en fejlside. Programmet ser ingen IP-adresser, og Cloudflare kan bruge stikprøver.
- Tallene til Login og "stoppet ved Edge" pr. robot hentes særskilt, i langsomt tempo efter den normale hentning, og kun for færdige dage. Første gang tager det op til en time for 30 dage. Har en plan ikke felterne, står der "ikke tilgængelig", og resten virker.

## I menulinjen og ved login
- **Lukker du vinduet, kører programmet videre** i menulinjen (Windows: notifikationsområdet) og henter dine tal i baggrunden. Dock-ikonet forsvinder, til vinduet åbnes igen.
- **Menuen ved ravne-ikonet:** Åbn Huginn · Siden sidst · Hent nu · en statuslinje ("Henter 3/16", "Hentet for 12 min. siden") · Afslut.
- **Afslut** i menuen (eller ⌘Q, mens vinduet er åbent) lukker programmet helt.
- **Start ved login** slås til første gang, en færdigbygget udgave starter (ikke under `npm run tauri dev`). Programmet starter så uden vindue, kun i menulinjen. Slå det fra under **Indstillinger** i programmet. Programmet slår det aldrig til igen af sig selv. Flytter du programmet til en anden mappe, retter det selv Start ved login og den daglige start, næste gang det starter (ikke når macOS kører det fra en midlertidig kopi eller fra et diskbillede).
- **Daglig start:** så længe Start ved login er til, starter programmet også selv én gang om dagen, kl. 12 lokal tid. Sover Mac'en, sker det, når den vågner. Kører programmet ikke, starter det uden vindue, henter og lukker igen, senest efter 30 minutter. Kører det, henter det bare. Så får historikken ikke huller, selv om du har valgt **Afslut**. Der er ingen særskilt kontakt: Start ved login slår begge til og fra. Windows: en planlagt opgave, som endnu ikke er prøvet.
- **Før du sletter programmet:** slå Start ved login fra i Indstillinger. Ellers bliver filerne `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist` og `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.dagligt.plist` liggende (Windows: værdien under `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` og den planlagte opgave `Wi24rd Huginn and Muninn dagligt`) og peger på et program, der ikke findes længere. De kan også slettes i hånden.
- Programmet kører kun én gang ad gangen; starter du det igen, vises det vindue, der allerede findes.

## "Siden sidst" og notifikationer
- **Siden sidst** er en visning: status for i går på alle dine domæner (Handl, Hold øje, Roligt) med én linje hver, og hvad der er nyt, siden du kiggede sidst. Den åbner af sig selv fra menulinjen (**Siden sidst**) eller ved et klik på en notifikation.
- **Notifikationerne kommer fra din egen computer** (systemets lokale notifikationstjeneste), aldrig fra en server, mail eller webhook. Vælg under **Indstillinger** (og når du forbinder API Tokenet): **Rolig** (højst én besked om dagen, kun når et domæne kræver handling), **Standard** (besked, når et domæne kræver handling, forvalgt) eller **Alt** (også ved Hold øje). Hvert domæne kan også sættes (**Notifikationer**, klokken i båndet på Overblik) til Følg standard, Alvorligt og hold øje, Kun alvorligt eller Aldrig, og **Giv besked, når et domæne kræver handling** kan slås fra.
- **Ligner målrettet, Ligner rekognoscering og Ligner DNS-rekognoscering** er mærker oven på status: de siger noget om mønsteret, ikke om mængden, og altid som "ligner", aldrig som en sikker dom. "Ligner målrettet" giver besked som standard; "Ligner rekognoscering", og med det DNS-mærket, kan slås til under **Indstillinger** › **Notifikationer**. I Overblik står de to kort som **Recon** og **DNS-recon**. "Ligner målrettet" sammenligner kun med dine valgte domæner, kræver mindst 3 og giver først besked, når døgnet efter er hentet. Indtil da står det som foreløbigt.
- **Datoformat** vælges under **Indstillinger**: Følg sproget, Følg systemet eller ISO. Det gælder alle datoer, også i notifikationerne.
- Handl betyder usædvanligt mange angreb i går i forhold til ugen før. Cloudflares rutine (labyrinten, Bot Fight Mode, browsertjekket) og gamle tal giver aldrig besked, og hvert domæne meldes højst én gang om dagen.
- macOS spørger om lov første gang, en notifikation vises. **Slå dem helt fra:** Systemindstillinger → Notifikationer → Huginn and Muninn.

## Installation
Hent `Huginn-Muninn-1.1.0.zip` under [Releases](https://github.com/wi24rd-com/huginn-muninn/releases/latest), åbn den, og træk **Huginn and Muninn** til din Programmer-mappe. Kræver macOS 11 eller nyere og en Mac med Apple silicon. Se [Installation](docs/manual/da/20-installation.md) og [Afinstallation](docs/manual/da/90-afinstallation.md).


## Hvad programmet sender, og hvorhen
- **Til Cloudflare og ingen andre steder:** programmet laver selv kun netværkskald til én vært, `api.cloudflare.com`. Det dækker dine zoner, tallene for dine valgte domæner, Cloudflares egne fund og Cloudflare Radar. Hvert kald har dit API Token og navnet `huginn-muninn/<version>` med.
- **Dit API Token** gemmes i styresystemets nøglering, aldrig i en fil, og sendes kun til Cloudflare. Med flere API Tokens har hvert sin post i nøgleringen, og hvert sendes kun med kaldene for sin egen kontos domæner. Giv det kun læserettigheder; vejledningen viser [de to rettigheder, det skal have](docs/manual/da/31-cloudflare.md).
- **Intet til Wi24rd:** ingen telemetri, ingen fejlrapporter, ingen konto og intet tjek for opdateringer. Programmet leder ikke selv efter nye udgaver.
- **Links åbner i din browser, og kun når du klikker:** et CVE-nummer (cve.org), kildelinjen under Radar-tal (radar.cloudflare.com), **Opret API Token hos Cloudflare** og Cloudflares egne fund (dash.cloudflare.com) og **Vejledning** (wi24rd.com). Programmet henter ikke selv noget fra de sider.
- **På din computer:** tallene ligger i én SQLite-fil i `~/Library/Application Support/com.wi24rd.huginn-muninn/`. Se [Data og historik](docs/manual/da/70-data.md).
- **Tjek det selv:** den eneste adresse i koden står i `src-tauri/src/cf.rs`, og vinduets content security policy i `src-tauri/tauri.conf.json` lader siden tale med programmet selv og intet andet. En netværksovervågning som Little Snitch eller LuLu skal vise `api.cloudflare.com` og intet andet.

## Verificér det, du har hentet
Du kan tjekke, at den fil, du har hentet, er den, der blev udgivet, og at programmet er signeret af udvikleren og notariseret af Apple. Kør kommandoerne i Terminal.

**1. Zip-filens kontrolsum**
```
shasum -a 256 ~/Downloads/Huginn-Muninn-1.1.0.zip
```
Linjen skal begynde med den SHA-256, der står i [udgivelsesteksten](https://github.com/wi24rd-com/huginn-muninn/releases/latest). GitHub viser samme værdi ved filen under **Assets**. For 1.0.0 er den:
```
f599bac0d5c61ad4a9355880979d583c8bea246712327f2ca35b0c342a5bb70b
```

**2. Programmets signatur og notarisering**
```
codesign --verify --deep --strict --verbose=2 "/Applications/Huginn and Muninn.app"
spctl -a -vv "/Applications/Huginn and Muninn.app"
```
Den første skal svare `valid on disk` og `satisfies its Designated Requirement`. Den anden skal svare `accepted`, `source=Notarized Developer ID` og `origin=Developer ID Application: Jan Tingsvad (5DAVNFA355)`.

Er kontrolsummen eller udviklerens navn anderledes, så åbn ikke programmet, og meld det gerne (se [SECURITY.md](SECURITY.md)).

## Byg (udvikling)
```
npm install
npm run tauri dev
(cd src-tauri && cargo test)  # Rust-testene
npm run test:ui              # UI-røgtesten (jsdom)
npm run kit:tjek             # kopierne i ui/assets/kit2/ er som pakken Wi24rd UI Kit 2
npm run test:signer          # signeringsscriptet prøvet med stubbe (macOS; signerer intet)
sh tools/signer.sh proeve    # macOS: debug-bygning signeret med din egen Developer ID, uden notarisering
```
Kræver Rust (rustup) og Node 20+.


## Hjælp, sikkerhed og bidrag
- **Hjælp:** [SUPPORT.md](SUPPORT.md). Projektet kommer uden support; brugervejledningen og issues er stederne at kigge.
- **Sikkerhedsproblemer** meldes privat: [SECURITY.md](SECURITY.md).
- **Bidrag:** [CONTRIBUTING.md](CONTRIBUTING.md). Sådan laves en udgivelse: [RELEASING.md](RELEASING.md).

SECURITY.md, SUPPORT.md, CONTRIBUTING.md og RELEASING.md er på engelsk.

## Licens
Apache-2.0 — se [LICENSE](LICENSE) og [NOTICE](NOTICE). Tredjepartskomponenter: [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

Cloudflare is a registered trademark of Cloudflare, Inc. Huginn and Muninn er ikke tilknyttet Cloudflare, Inc.

[English](README.md)
