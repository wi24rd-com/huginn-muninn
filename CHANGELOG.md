# Changelog

Udgivelser er endnu ikke tagget. Posterne står efter dato, nyeste først.

## [Unreleased]

### 2026-11-11 23:30 — Wi24rd UI Kit 2: farverne og temaet
#### Ændret
- **Farverne kommer fra Wi24rd UI Kit 2** (pakke 2.1.0, kopieret uændret til `ui/assets/kit2/`). En bro (`ui/assets/kit2-bro.css`) sætter programmets farvevariabler til kittets: nordlysgrøn accent, "ok" som sin egen farve, angreb i `--danger-mark`, besøg i `--info-mark`, og kortets fem trin fra kittets skala. Rammen (båndet, sidebjælken) og skriften er som før; de følger i de næste trin.
- **Udseende (Lys, Mørk, System)** styres af kittets `tema.js`: `<html data-tema="lys | moerk | system">`. Et valg gemt før skiftet oversættes én gang ved start. Tekststørrelsen er uændret og ligger nu i `tekst.js`.
- Kontrasten måles efter kittets krav (WCAG AA): tekst mindst 4,5:1, stor tekst, mærker, serier og UI mindst 3:1. Før var kravet til tekst 7:1.
#### Tilføjet
- `npm run kit:tjek`: fejler, hvis en kopi af kittet ikke er byte for byte som pakken.

### 2026-11-10 23:30 — Besøgende og Angreb efter Jans live-test: fliser øverst, et år på én bredde, sider, der svarer med forsiden
#### Tilføjet
- **Svarede med sidens almindelige side** på Besøgende › Mest besøgte sider (under 404-listen): stier, hvis svar var lige så store som forsiden (højst 25 % fra, samme regel som Følsomme filer), som ikke er forsiden, og som ingen kom til udefra. Sitet svarer med forsiden på adresser, det ikke kender (catch-all), så de kom aldrig i 404-listen. Et skøn ud fra størrelsen; hvert domæne sammenlignes med sin egen forside, og uden forsiden i perioden deles intet (en linje siger det). Sidekaldet henter nu også `sum { edgeResponseBytes }`, gemt som `b_side_bytes`; første start efter opdateringen henter siderne for den seneste måned én gang til, ét kald pr. domæne.
#### Ændret
- **Besøgende:** de faste fliser står øverst under overskriften "De seneste dage", så vælgeren (7/30/Et år og Fra/Til), og under den "Valgt periode: 11. okt. – 9. nov. 2026" over alt, der følger perioden. Ingen flise er markeret som valgt længere; et klik sætter stadig perioden.
- **Et år pr. uge ses på én gang:** uge-søjlerne er mindst 14 px brede (dag-søjler stadig 24 px), så 53 uger fylder bredden uden vandret rulning ved et vindue på 1200 px, og aksen viser månederne.
- **Angreb › De seneste:** Forrige og Næste bytter kun tabellen ud. Siden springer ikke til toppen, fokus bliver på knappen (eller "Side N af M"), og `side=N` i adressen erstattes, så tilbage og frem ikke får et trin pr. side.
- **Landevælgeren** (båndet, Indstillinger › Land og "Skift til …") viser kun lande, der har været blandt Radars 100 mest ramte mål på mindst 15 af de seneste 30 dage. Ingen undtagelser.
- 404-listens tekst lover ikke længere, at alle forsøg står der.
#### Rettet
- Menulinjen siger ved netfejl "Ingen forbindelse til Cloudflare · prøver igen om 2 min." som båndet, i stedet for "1 domæne fejlede".
- Manualen (Siden sidst): "Hele dage i UTC".

### 2026-11-09 23:30 — Besøgende efter live-testen: blå søjler, uger, robotter og sider
#### Tilføjet
- **Kendte robotter** på Besøgende: besøg fra robotter, Cloudflare har verificeret, i Robotter-visningens grupper, et klik på en gruppe viser dens besøg pr. dag, og flisen **Besøg på 30 dage** siger "heraf N fra kendte robotter (x %)". Hentes med et eget kald (`httpRequestsAdaptiveGroups`, `count` + `sum { visits }`, `date verifiedBotCategory`, `requestSource: "eyeball"`) for de samme dage som besøgene og gemmes som `b_robot`.
- **IP-klasser** (`ipClassMap { ipType requests }`) hentes med årstallene og gemmes som `h_ipklasse`: "x % af trafikken det seneste år kom fra kendte robotter og scannere" (søgemaskiner, overvågning, scannere, Tor og kendte dårlige adresser). Første start efter opdateringen henter året én gang til for de dage, der allerede er hentet; kun klasserne skrives.
- **Mest besøgte sider** på Besøgende: sti, visninger, besøg udefra og andel, de 10 største fremme og resten bag **Vis alle**, og **Sider, der ikke findes** (404) for sig. Et klik på en sti viser visningerne af siden pr. dag, og flisen Besøg følger stien. Eget kald (html-svar, `date clientRequestPath edgeResponseStatus`), top 50 pr. dag, stien renset som i Angreb (uden forespørgselsstreng, lange koder skjult, højst 90 tegn); gemmes som `b_side`/`b_side_b`.
- Afviser Cloudflare robotterne eller siderne, står "ikke tilgængelig" kun ved det afsnit; resten virker, og der spørges igen ved næste start. Databasen får skema 18 med `hentet_robot` og `hentet_side`; **Nulstil historik** sletter dem.
#### Ændret
- **Besøgendes søjler er blå** som båndet øverst (ny token `--board-visitors`, lys `blue-720`, mørk `blue-400`, ≥ 3:1 mod tavlen), så de ikke læses som angreb. Angrebs søjler er uændret røde.
- **Et år pr. uge:** perioder over 90 dage vises med én søjle pr. uge (mandag–søndag, UTC), med "N af 7 dage" i værktøjstippet, når der mangler dage. Et klik på en uge viser ugen. 7 og 30 dage er uændrede.
- **Flisen Besøg følger filtret:** "Besøg fra Danmark på 30 dage" med et land, og på stien med en sti. Unikke besøgende kan ikke deles; de fliser siger "alle lande" eller "alle sider". Højst ét filter ad gangen.
- **Land lige nu:** landevælgeren i båndet, Indstillinger › Land og forslaget "Skift til …" viser kun lande, Cloudflare Radar har haft tal for som mål de seneste 30 dage. Overskrifterne "Dine domæner og hele verden" og "Hvor angreb mod dine domæner kom fra (alle lande)" siger, at de afsnit ikke afhænger af landet.
- Datovælgerens note siger "Hele dage i UTC. Tidspunktet på dagen gemmes ikke."
#### Rettet
- **Ingen forbindelse efter dvale:** et domæne, der ikke kunne hentes uden net, gav "1 fejlede" og "Der opstod en fejl i programmet" i op til en time, fordi et senere domænes net-pause slettede fejlen. Nu står "Ingen forbindelse til Cloudflare, så … kunne ikke hentes. Prøver igen af sig selv." (pillen: "Ingen forbindelse · prøver igen"), og køen starter selv en ny runde efter 2 minutter i stedet for en time.

### 2026-11-09 23:30 — Besøgende: hvor mange, og hvorfra
#### Tilføjet
- **Ny visning Besøgende** (IDEA-046, ⌘7, Windows Ctrl+7, også i menuen **Vis** og i hjælpens genveje): unikke besøgende i går, på 7 dage, 30 dage og et år, besøg på 30 dage, en graf pr. dag (7 dage, 30 dage, et år eller Fra/Til op til et år) og en tabel over, hvor besøgene kom fra. Ét domæne eller Samlet. Et klik på en dag åbner "Trafik den …" på Angreb, et klik på et land viser besøgene fra landet pr. dag (chip med ×), og fliserne vælger perioden. Dage uden tal er huller, ikke 0. Ingen notifikationer, ingen status.
- **Unikke besøgende** hentes med årstallene (`httpRequests1dGroups` → `uniq { uniques }`, unikke IP-adresser pr. dag, kun et tal). Første start efter opdateringen henter året én gang til for de dage, der allerede er hentet, ét kald pr. domæne; kun det tal skrives.
- **Besøg pr. land** (`httpRequestsAdaptiveGroups` → `sum { visits }`, `requestSource: "eyeball"`) hentes til sidst i den langsomme runde for de sidste 30 færdige dage, ét kald pr. domæne, og gemmes, så historikken vokser. Afviser Cloudflare dem, står "ikke tilgængelig" ved domænet, resten kører videre, og der spørges igen ved næste start. Ikke målt live på Free endnu.
- Databasen får skema 17 med tabellen `hentet_besoeg`. **Nulstil historik** sletter også den. Ingen nye rettigheder på API Tokenet.
#### Rettet
- "Trafik den …" for en dag ældre end en måned lagde alle årstallets nøgler sammen som forespørgsler; nu kun forespørgslerne (ellers ville unikke besøgende tælle med).

### 2026-11-08 23:30 — Start ved login, der overlever en flytning, og det rigtige token-trin
#### Ændret
- **Start ved login skrives af programmet selv på macOS** (IDEA-044), som den daglige start. Samme fil og navn som før (`~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist`, `com.wi24rd.huginn-muninn`), nu med programmets bundle-id, så Login-emner viser jobbet under programmet, og med stien XML-escapet, så en mappe med `&` i navnet ikke giver en ugyldig fil. Første start efter opdateringen skriver den eksisterende fil om. Et fravalg står ved magt. Windows og Linux bruger stadig `tauri-plugin-autostart`, uændret.
- **Flyttes programmet, retter det selv** login- og dagligt-plisten ved næste start, og den daglige indlæses igen. Det sker aldrig, når programmet kører fra en midlertidig mappe (App Translocation, når en usigneret app startes fra Overførsler), fra et diskbillede under `/Volumes/` eller under `npm run tauri dev`. Er Start ved login slået fra, oprettes intet.
- **Token-trinnet til Cloudflares egne fund** (IDEA-045): programmet siger nu det samme som manualen. Rettigheden er *Account Security Center Insights › Read* i en ny politik (**Add new policy › Add new Account policy**, Include · All accounts), og opsummeringen viser *Account Security Center Insights: Read* under All accounts. *Zone Security Center Insights* virker ikke. Mangler rettigheden, siger afsnittet **Cloudflares egne fund**, at den tilføjes som kontorettighed.

### 2026-11-08 23:30 — Afviste som Cloudflares estimat, ikke som antal rækker
#### Ændret
- **Afviste tælles som Cloudflares estimat** (IDEA-009): hovedhentningen henter feltet `sampleInterval` på hver afvist forespørgsel og lægger det sammen i stedet for at tælle rækker. Det er det samme tal, Cloudflares `firewallEventsAdaptiveGroups` ville give, men virker på Free-planen og koster ikke et ekstra kald. Målt: på travle døgn stiger tallet 11–56 %; stille domæner er uændrede. Overblik, Angreb, kortet, listerne, status, mærkerne og notifikationerne følger med. "De seneste" og de enkelte angreb tæller stadig hændelser.
- **De sidste 30 dage hentes igen én gang** efter opdateringen, i køens normale tempo, så snit og mærker ikke sammenligner estimater med rækketal. Ældre dage beholder de talte rækker; går en periode på Angreb over grænsen, siger en note det.
- **10.000-grænsen** afgøres nu af antallet af hændelser i Cloudflares svar (gemt for sig), ikke af det estimerede tal. Står en dag på grænsen, siger Angreb "Der er mindst …". Et svar på grænsen erstatter aldrig et højere tal, der allerede er gemt.
- Teksterne om stikprøver (fodnoten på Angreb, "Det, der ikke gemmes" og manualen) siger nu, at tallet er Cloudflares estimat ud fra en stikprøve, i stedet for "kan være samplet og er derfor omtrentlige".
- HAM og wi24rd.com viser forskellige tal på travle domæner: wi24rd.com tæller rækker.

### 2026-11-08 23:30 — Følsomme filer, der svarede 200, og Cloudflares egne fund
#### Tilføjet
- **Følsomme filer, der svarede 200** (IDEA-011): hovedhentningen spørger nu også, om 39 faste stier, der aldrig bør svare 200 (fx `/.env`, `/.git/config`, `/.aws/credentials`), fik svar 200, og hvor store svarene var. Det koster ikke et ekstra kald. Afsnittet står på Angreb for ét domæne med sti, antal, snitstørrelse og vurdering, og hver sti filtrerer Angreb på stien.
- Et svar, der ligner selve filen, giver **Handl** med teksten "En følsom fil (…) svarede 200 i går – den kan ligge åbent" og et råd bag **Hvad gør jeg?**. Det ligner filen, når det er højst 8 KB, mere end 25 % fra forsidens størrelse, og domænet ikke svarer 200 på mere end 6 af stierne. Målt på 13 domæner svarede 3 med deres almindelige side på næsten alle stierne; de står på Angreb som "Ligner sidens almindelige side" og ændrer ikke status.
- **Cloudflares egne fund** (IDEA-036, Security Center Insights): afsnittet på Angreb for ét domæne viser alvor, titel og dato, og hvert fund åbner Cloudflares side for Security Insights. Fundene hentes højst én gang i timen efter den normale runde. Et kritisk fund giver **Hold øje**, aldrig Handl.
- Kræver én valgfri ekstra rettighed på API Tokenet, *Security Center Insights › Read* (Cloudflares dokumentation: *Account Security Center Insights*). Uden den siger afsnittet, at rettigheden kan tilføjes, og resten af programmet virker. Token-trinene i programmet og manualen har trinnet som valgfrit.
- Databasen får skema 16 med tabellen `sikkerhedsfund`. **Nulstil historik** sletter også den.
#### Ikke med
- Sampling-justerede totaler (IDEA-009) er ikke bygget: `firewallEventsAdaptiveGroups` er slået fra på Free-planen (målt). Se INVESTIGATION-V-TOKEN.1 for en vej uden den.

### 2026-11-07 23:30 — Huginn starter selv én gang om dagen
#### Tilføjet
- Daglig start kl. 12 lokal tid, så historikken ikke får huller, når programmet er afsluttet, eller maskinen ikke har været logget ud i ugevis. macOS: LaunchAgenten `com.wi24rd.huginn-muninn.dagligt`, indlæst med det samme. Sover Mac'en, sker det, når den vågner.
- Kørte programmet ikke, starter det uden vindue og Dock-ikon, henter hele runden med de ekstra tal og lukker igen, senest efter 30 minutter. Åbner du vinduet undervejs, bliver programmet kørende.
- Kører programmet, åbner den daglige start ikke vinduet. Den starter en hentning som **Hent nu**.
- Windows: en planlagt opgave med `schtasks`. Koden er skrevet, men ikke prøvet eller oversat. Linux: intet endnu.
#### Ændret
- **Start ved login** slår både login-agenten og den daglige start til og fra. Der er ingen ny kontakt. Installationer med Start ved login slået til får den daglige start én gang; et fravalg står ved magt.

### 2026-11-07 23:30 — Kortet i mørkt tema: egen ringfarve og tydeligere trin
#### Ændret
- Kortet har sin egen ringfarve i lyst og mørkt tema, så ringen ikke falder sammen med det første trin. I mørkt tema er ringen gul.
- Mørkt tema: nabotrin i landenes fyld står mindst 1,3 fra hinanden, og det øverste trin er hvidt. Havet er en lille smule mørkere, så det første trin stadig står 3:1 mod havet.
- Konturen om et land ved hover og fokus har sin egen farve og kan ses på alle trin.

### 2026-11-07 23:30 — Angreb på én side pr. domæne
#### Ændret
- Angreb og dyk er samlet på én side. Overskriften siger fx "Angreb · wi24rd.com", og siden har altid datovælger, **7 dage** (standard) / **30 dage**, **Vis** og filtre som chips med × og **Nulstil**.
- Et klik på et tal, et land, en regel eller en sti forudindstiller perioden og filtret på samme side, og tilbage og frem virker som før.
- Alle afsnit fra begge visninger følger perioden og filtrene. Det gælder også "Hvad de ledte efter", mails og tabellen under søjlerne. Et afsnit, der ikke kan følge et filter, står ikke, og en linje siger hvilke.
- Dage med usædvanlig meget trafik er afsnittet "Trafik den …" på samme side, under **Vis**.
- Domænevælgeren beholder filtre og periode, når du skifter domæne.
- Gamle `#dyk?…`-adresser lander på samme sted i Angreb.
- README har et eget afsnit om Angreb, og manualens kapitel er skrevet om.
#### Fjernet
- "Udsnit" og "← Hele oversigten" er væk.

### 2026-11-07 23:30 — Knappen Åbn vejledningen
#### Tilføjet
- **Åbn vejledningen** øverst i hjælpen (**?**) og under **Indstillinger → Om** åbner manualens forside i din browser.
- Adressen følger sproget og står kun i Rust. Webvisningen sender kun sproget, og programmet henter intet derfra.

### 2026-11-07 23:30 — Trinene til API Token følger Cloudflares nye side
#### Ændret
- "Sådan opretter du et API Token" har nu seks trin med Cloudflares ord: Create Token, Policies, Zone › Read, Analytics › Read, Include · All zones, No expiration og Continue to summary. Knappen åbner formularen færdigudfyldt, så de overflødige trin er væk.
- En linje under trinene siger, at felterne er de samme, hvis man selv starter fra Create Token.
- Fejlteksterne om manglende rettigheder bruger de samme ord: Zone › Read, Analytics › Read og Include · All zones eller Specific zone. Fejlfinding i manualen citerer dem.
- Manualens kapitel om opsætningen siger ikke længere, at programmets liste bruger ældre ord.

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
