# Changelog

Posterne står efter dato, nyeste først.

## [Unreleased]

### 2026-11-16 23:30 — Subdomæner i Besøgende og Angreb
#### Tilføjet
- **Subdomæner i Besøgende og Angreb:** har et domæne trafik på mindst to navne (fx `wi24rd.com`, `www.wi24rd.com` og `shop.wi24rd.com`), står afsnittet **Subdomæner** med besøg, forespørgsler, angreb og andel pr. subdomæne i den valgte periode. Det står kun, når ét domæne er valgt; et domæne uden subdomæner ser ud som før. Flere end 20 navne samles i **Andre (n)**, og opslag med en IP-adresse, et andet navn eller et ugyldigt navn står for sig og kan ikke åbnes.
- **Filter pr. subdomæne:** et klik på et subdomæne viser samme visning for det, med chippen **Subdomæne: …**. Besøgende viser besøg og forespørgsler pr. dag, hvor besøgene kom fra, kendte robotter og subdomænets mest besøgte sider; Angreb viser angreb og afviste pr. dag. Det, der ikke findes pr. subdomæne (unikke besøgende, IP-klasserne, regler, lande og stier i Angreb, De seneste, Cloudflares egne fund), står ikke; en linje under chippen siger det og fører til hele domænet. Et afsnit uden tal for subdomænet i perioden står heller ikke. Filteret står i adressen og nulstilles, når et andet domæne vælges.
- **Hvert tal i tabellen Subdomæner åbner det, det tæller:** Angreb åbner Angreb for subdomænet, Besøg og Forespørgsler Besøgende, med samme periode. Navnet åbner den visning, tabellen står i. Et tal, der er 0, kan ikke klikkes.
- **Lande og kendte robotter pr. subdomæne** hentes i samme kald som trafikken pr. subdomæne (intet ekstra kald pr. dag). De sidste ca. 30 dage hentes én gang igen pr. domæne ved første start.
- Noterne **Pr. subdomæne fra …**, **Kun trafik gennem Cloudflare. Et subdomæne, der er «DNS only», har ingen tal her.**, **Cloudflare svarede ikke på subdomænerne for dette domæne.** og **{n} dage er ikke med: Cloudflare sendte flere rækker, end programmet kan hente på én gang.**
- **Et tal pr. subdomæne, der ikke er hentet endnu, står som –, aldrig som 0:** de gemte tal vises, mens dagene hentes igen, og en linje under tabellen siger **Henter besøg og forespørgsler pr. subdomæne …** eller **Henter {n} dage endnu.**, indtil køen er færdig. Det gælder også det filtrerede og de første minutter efter en opdatering.
#### Ændret
- **Den lokale database opgraderes endnu et trin** (skema 20) ved første start: to nye tabeller over, hvilke dage der er hentet pr. subdomæne. Ingen gemte tal ændres. Version 1.0.0 siger bagefter "(skema 20)".
- **Mest besøgte sider skelner mellem subdomæner:** samme sti på to subdomæner står som to rækker, med subdomænet ved stien, og et klik viser stien på det subdomæne. Står svaret på Cloudflares loft, siger en note, at kun de mest besøgte sider er med.
- **Historik pr. subdomæne fra ca. 30 dage før opgraderingen:** første start henter de sidste ca. 30 dage én gang pr. domæne. Ældre dage har ingen tal pr. subdomæne.
- Afviser Cloudflare subdomænets navn i hentningen af angrebene, hentes domænets tal uden det, som før, resten af døgnet (UTC); næste døgn prøves der med navnet igen.
- Vejledningen beskriver subdomænerne i Besøgende, Angreb, Data og historik, Fejlfinding og ordlisten. API Tokenet skal ikke have flere rettigheder.

### 2026-11-16 23:30 — Flere API Tokens side om side
#### Tilføjet
- **Flere Cloudflare-konti:** under Indstillinger › Forbindelse(r) tilføjer du ét API Token pr. konto med **Tilføj API Token…**. Domænerne fra alle forbindelser vises og hentes samlet, og et domæne, to API Tokens ser, står kun én gang. Hver forbindelse har en række med navn, antal domæner og tilstand, og kan skiftes eller glemmes for sig. Med én forbindelse ser kassen ud som før.
- **Navnet på en forbindelse** er kontoens navn hos Cloudflare, også når det er en e-mailadresse. **Omdøb…** giver den dit eget navn; et tomt felt giver Cloudflares navn igen.
- **Domæner** viser med flere forbindelser én liste pr. forbindelse, hver med sit eget Vælg alle / Ingen.
- **Et afvist API Token** stopper kun sin egen forbindelse. Overblik viser et banner med forbindelsens navn og **Skift API Token…**, og fodlinjen og menulinjen nævner det. Er alle afvist, stopper hentningen som før.
- Guiden siger i trinnet Dit API Token, at flere konti tilføjes bagefter under Indstillinger.
- **Et domæne, flere forbindelser ser,** står i Domæner under hver af dem, med "Hentes med «navn»" under dem, der ikke henter det. Det er ét valg: kontakten følger med alle steder. Overblik og de andre visninger viser det stadig én gang.
- **To forbindelser med samme navn:** hedder en ny forbindelse det samme som en anden, åbner Omdøb med en note, så du kan give den et navn.
- **Samme API Token to gange** tilføjes ikke. Ser et nyt API Token præcis de samme domæner som en forbindelse, du har, siger formularen det med forbindelsens navn. Et API Token til færre domæner på samme konto kan godt tilføjes.
#### Ændret
- **Den lokale database opgraderes** ved første start. Intet går tabt: et gemt API Token bliver den første forbindelse med alle sine valg. Version 1.0.0 kan ikke åbne databasen bagefter; tag en kopi af den først, hvis du vil kunne gå tilbage.
- Hver forbindelse har sin egen post i nøgleringen. Den første bruger samme post som 1.0.0.
- Efter **Omdøb** står "Navnet er gemt." i forbindelsens række, og fokus går tilbage til **Omdøb…**.
- Vejledningens kapitel om data beskriver nu flere forbindelser: Glem en forbindelse, nøgleringen og databasen efter opgraderingen.
#### Rettet
- Et nyt API Token fra en anden Cloudflare-konto fjernede de første domæner, deres valg og teknologisvar. Nu tilføjes det som en ny forbindelse, og **Skift API Token…** siger i formularen, at en konto mere skal tilføjes.
- En forbindelse, hvis domæner alle også ses af en ældre forbindelse, havde en tom liste i Domæner med teksten "Domænerne står her, når zonelisten er hentet." Den tekst står nu kun, før listen er hentet; ser API Tokenet ingen domæner, står der det.
- **Skift API Token…** fra banneret i Overblik viser feltet med fokus, også i et lavt vindue.

### 2026-11-15 23:30 — Repoet på GitHub: CI, sikkerhedspolitik og skabeloner
#### Tilføjet
- **CI** (`.github/workflows/ci.yml`): Rust-testene, UI-røgtesten, testen af signeringsscriptet og kit-tjekket kører på GitHub ved hvert push til `main` og ved pull requests. Kun læserettigheder, ingen hemmeligheder, og de brugte actions er låst til en bestemt commit. Signering og notarisering sker stadig kun lokalt.
- **Dependabot** (`.github/dependabot.yml`): én samlet pull request om måneden for Cargo, npm og GitHub Actions. Intet flettes automatisk.
- **SECURITY.md**, **SUPPORT.md**, **CONTRIBUTING.md** og **RELEASING.md** (på engelsk): hvordan en sårbarhed meldes privat, hvor man finder hjælp, hvordan man bidrager og tester, og hvordan en udgivelse laves med kladde først og kontrolsum.
- **Skabeloner** til fejl, ønsker og pull requests, som beder om programmets version og macOS-version og advarer mod at indsætte API Token, zone-id eller domæner.
#### Ændret
- **README** (dansk og engelsk) har fået fire mærker øverst og to nye afsnit: hvad programmet sender og hvorhen, og hvordan man tjekker kontrolsum, signatur og notarisering på det, man har hentet.

### 2026-11-14 23:30 — Vejledningen og README følger version 1.0.0
#### Ændret
- **Vejledningen** (`docs/manual/`, dansk og engelsk) beskriver nu programmet, som det er: guiden ved første start, Indstillinger bag tandhjulet, dialogerne Domæner, Notifikationer pr. domæne og Om, fanerne, båndet og fodlinjen. Installationen beskriver den signerede og notariserede udgave.
- Alle skærmbilleder i vejledningen er taget på ny, og der er kommet billeder af guidens fire trin og de tre dialoger.
- **README** (dansk og engelsk) har status 1.0.0, et afsnit om installation og link til vejledningen.
- **README** har øverst et tydeligt link til at hente den nyeste udgave.
- **README** har fået et banner øverst (`docs/banner.jpg`) og siger tydeligt, at Windows-udgaven ikke er udgivet endnu.
- Skærmbillederne i vejledningen viser nu programmets egen version i fodlinjen (før et fast prøvetal), så de passer til den udgave, de beskriver.

## [1.0.0] — 2026-11-14

Første offentlige udgave til macOS: signeret med Developer ID og notariseret af Apple.

### 2026-11-14 23:30 — Programmet kan signeres og notariseres til macOS
#### Tilføjet
- **Signering til Apple:** `tools/signer.sh` signerer programmet med Developer ID, sender det til Apples notarisering og hæfter svaret på. Udgivelser signeres kun i et fast tidsvindue. Intet ændrer sig i programmet selv; en signeret udgave viser udviklerens navn i stedet for »unidentified developer«.
- `npm run test:signer` prøver scriptet med stubbe, uden at signere noget.

### 2026-11-13 23:30 — Programikonet er grønt på mørk bund
#### Ændret
- **Programikonet** (Dock, Finder, Windows): ravnene står i kittets grønne på en mørk flise, som de andre Wi24rd-programmer. Før var de hvide på blå. Ikonet i menulinjen er uændret.

### 2026-11-13 23:30 — Indstillinger er en popup, og første start er en guide
#### Ændret
- **Indstillinger er en popup bag tandhjulet** øverst til højre (⌘, / Ctrl+,, og Indstillinger… i programmenuen). Den åbner oven på den visning, du står i, og har grupperne Udseende og sprog, Dato og land, Start, Notifikationer, Forbindelse, Data og Første start. Fanen Indstillinger er væk; de syv visninger er tilbage som de eneste faner.
- **Om har sin egen knap** (i) ved siden af tandhjulet: version, licens, skrifter, kortdata og varemærkelinjen. Programmenuens Om åbner den samme dialog.
- **Vælg domæner** på Overblik åbner dialogen Domæner: en kontakt pr. domæne.
- **Notifikationer** (klokken) på Overblik åbner sin egen dialog: hvornår du vil have besked om hvert valgt domæne. Linjerne om fravalgte og nye domæner åbner den samme dialog.
- **Teknologi pr. domæne hører til Angreb:** fanen Angreb er markeret, og båndet har Tilbage til Angreb og Nulstil for alle domæner (før under Indstillinger › Domæner).
- **Login-stier** rettes ét sted: Login › Ret stier. Afsnittet under Indstillinger er væk.

- **Mærket i Overblik er kortere:** »Recon« (og »DNS-recon«), så det står på én linje. Det fulde ord står alle andre steder, også i mærkets forklaring.

#### Tilføjet
- **En guide ved første start** med fire trin: Velkommen, Dit API Token, Dine domæner og Besked. Næste er først fri på trin 2, når dit API Token er gemt og virker. Spring over efterlader en rolig side med knappen Åbn guiden, og guiden kan altid åbnes igen under Indstillinger › Første start.

#### Rettet
- Gamle adresser (`#indstillinger`, også med `afsnit=om`, `afsnit=domaener` og `afsnit=login`) åbner det rigtige sted i stedet for en side, der ikke findes længere.

### 2026-11-13 23:30 — Store knapper og tydelige faner, som skitsen
#### Ændret
- **Periode og Vis er store knapper med ikon** i båndet (7 dage, 30 dage, Hele historikken; Alt, Kun angreb, Kun rutine; Danmark, Verden; 7 dage, 4 uger), ikke små sammenhængende knapper. Den valgte har grøn flade og kant.
- **Hent nu** er en fyldt, grøn knap.
- **Fanerne er større** og nemmere at ramme.
- **Egne datoer** (Fra og Til) står sidst i båndet.

### 2026-11-13 23:30 — Mærket står til venstre i fuld skærm
#### Rettet
- **macOS i fuld skærm:** ravnene og navnet i titellinjen står nu helt ude til venstre. Før holdt titellinjen plads til de tre vinduesknapper, selv om macOS skjuler dem i fuld skærm.
- I et almindeligt vindue står mærket lidt tættere på vinduesknapperne end før.

### 2026-11-13 23:30 — Faner i båndet, og båndet viser kun det, der hører til fanen
#### Ændret
- **De syv visninger er faner øverst i båndet** (Overblik, Siden sidst, Angreb, Robotter, Login, Danmark lige nu, Besøgende), og **Indstillinger** står for sig til højre. Genvejene ⌘1–⌘7 og ⌘, virker som før.
- **Båndet under fanerne viser kun det, der hører til fanen.** "Hent nu" står forrest i alle.
- **Periode, Fra og Til, Vis og Nulstil står nu i båndet** i Angreb, Robotter, Login og Besøgende, ikke længere nede i siden. De aktive filtre står stadig som mærker under overskriften.
- **Fra og Til er tomme, så længe en fast periode er valgt.** Vælger du en dato, slipper den faste periode markeringen; vælger du kun Fra, går perioden frem til i går.
- **"Hele historikken"** er et valg i Periode i Angreb, Robotter og Login (før: linket "Nulstil" ved datoerne).
- **Nulstil** i Angreb står altid i båndet og er slået fra, når der ingen filtre er.
- **Danmark lige nu:** Danmark/Verden, 7 dage/4 uger og landevælgeren står i båndet.
- **"Angreb for alle" og "Vælg domæner"** (Overblik), **"Til overblikket"** (Siden sidst), **"Ret teknologier"** (Angreb), **"Angreb fra robotter"** (Robotter) og **"Ret stier"** (Login) er knapper i båndet.
- **Vejledning** står i titellinjen ved siden af "?".

### 2026-11-13 23:30 — Wi24rd UI Kit 2, pakke 2.8.0
#### Ændret
- Pakke 2.8.0 ligger uændret i `ui/assets/kit2/`. Programmet bruger ikke den nye mulighed (visningerne som båndets egne faner) og beholder de store knapper i båndet.

### 2026-11-13 23:30 — Wi24rd UI Kit 2, pakke 2.7.0: visningerne i båndet er kittets
#### Ændret
- **Visningernes knapper i båndet** bygges nu af Wi24rd UI Kit 2 (pakke 2.7.0) i stedet for af programmet selv. De står og virker som før.
- **Den åbne visning** har fået en streg forneden ud over den grønne flade og den fede skrift.
- **Genvejen og "Kræver et API Token."** står i kittets tip, som også kan læses med tastaturet.
- **Smalt vindue eller Stor tekst:** båndet står altid på én række. Det, der ikke er plads til, samles under "Mere"; visningerne og landevælgeren bliver stående.
- Pakke 2.7.0 ligger uændret i `ui/assets/kit2/`.

### 2026-11-12 23:30 — Visningerne står i båndet
#### Ændret
- **De syv visninger** (Overblik, Siden sidst, Angreb, Robotter, Login, Danmark lige nu, Besøgende) står nu som store knapper først i båndet, på samme række som "Hent nu" og "Åbn vejledningen". Den åbne visning er markeret.
- **Sidebjælken** har kun domænerne og bruger hele højden til dem.
- **Genvejene** (⌘1–⌘7, Ctrl+1–7) virker som før og står nu i knappens tip.
- **Uden API Token** siger knappens tip, hvorfor den er slået fra.
- **Mindste vindue** er nu 1100 px bredt (før 900), så båndet kan stå på én række.

### Ændret
- **De syv visninger** (Overblik, Siden sidst, Angreb, Robotter, Login, Danmark lige nu, Besøgende) står nu som faner under titellinjen, som i Wi24rd Grimnir. Båndet står under fanerne.
- **Sidebjælken** har kun domænerne og bruger hele højden til dem.
- **Genvejene** (⌘1–⌘7, Ctrl+1–7) virker som før og står nu i fanens tip i stedet for ved siden af navnet.
- **Uden API Token** siger fanens tip, hvorfor den er slået fra.

### 2026-11-12 23:30 — Wi24rd UI Kit 2, pakke 2.6.0
#### Ændret
- Pakke 2.6.0 ligger uændret i `ui/assets/kit2/`. Programmet bruger ikke den nye mulighed (visningerne som faner øverst) og ser ud som før.
- En valgt række i en tabel viser nu valget, også når musen står på den.

### 2026-11-12 23:30 — Indstillinger i båndet, og vinduesknapperne på plads
#### Ændret
- **Indstillinger** står nu i båndet ved siden af "Åbn vejledningen" (gruppen Program) i stedet for som tandhjul i titellinjen. Knappen er markeret, mens Indstillinger er åben.
#### Rettet
- **macOS:** de tre vinduesknapper (luk, minimér, zoom) sad for lavt og ragede ned over titellinjens kant. De står nu midt i titellinjen.

### 2026-11-12 23:30 — Wi24rd UI Kit 2: oprydning (Kit 1.0, Outfit og broen er væk)
#### Ændret
- **Farver og skrifter har én kilde:** Wi24rd UI Kit 2 i `ui/assets/kit2/`. Programmets egen CSS bruger kittets navne direkte (`--text`, `--surface`, `--accent`, `--danger-mark` …).
- **"Om" i Indstillinger** nævner nu Inter og JetBrains Mono.
- Programmets egne mål (kontrolhøjde, tekststørrelser, fokusring) står ét sted øverst i `ui/assets/app.css`.
#### Fjernet
- Wi24rd UI kit 1.0: `design/wi24rd-tokens.json`, `design/build-tokens.mjs`, `design/out/Wi24rdTokens.xaml` og den genererede `ui/assets/tokens.css`, med deres tests.
- Broen `ui/assets/kit2-bro.css`.
- Skriften Outfit og programmets egen kopi af JetBrains Mono (`ui/assets/fonts/`); kittet har JetBrains Mono med.

### 2026-11-12 23:30 — Wi24rd UI Kit 2: fodlinjen, filtrene og fliserne (pakke 2.5.0)
#### Ændret
- **Hentningens status i fodlinjen** er kittets: mærke eller ring, kort tekst og en tynd fremdriftslinje; et klik åbner Overblik.
- **Aktive filtre** (fx "Polen", "3. november") er kittets filterchips med ×; teksten streges over, når musen står på dem.
- **Den valgte flise**, flisen med to mål ("Afvist i går" og "i dag hidtil") og datoflisen er kittets.
- **Tallet ved Siden sidst** og visninger, der er slået fra uden API Token, er kittets.
- Pakke 2.5.0 ligger uændret i `ui/assets/kit2/`.

### 2026-11-12 23:30 — Wi24rd UI Kit 2: tabellerne (pakke 2.4.0)
#### Ændret
- **Overblik er kittets tabel:** et klik, Enter eller mellemrum på en række åbner domænets Angreb, piletasterne flytter mellem rækkerne, og domænet står fast, når tabellen ruller vandret, ved alle vinduesbredder.
- **De øvrige tabeller** (De seneste, trafik pr. dag, DMARC, følsomme filer, Login, Besøgende) er kittets tabel uden rækkevalg.
- **Røde statuschips viser ×** foran ordet (før ?).
- Pakke 2.4.0 ligger uændret i `ui/assets/kit2/`.

### 2026-11-11 23:30 — Wi24rd UI Kit 2: sideskift og tom tilstand
#### Ændret
- **Sideskiftet i "De seneste"** på Angreb er kittets: Forrige og Næste står der altid og er slået fra på første og sidste side.
- **Tomme afsnit** (ingen DMARC-rapporter, ingen fund, ingen teknologi valgt) bruger kittets tomme tilstand.

### 2026-11-11 23:30 — Wi24rd UI Kit 2: dialoger, kasser og fliser
#### Ændret
- **Bekræftelserne** (Glem API Token, Nulstil historik, Nulstil teknologi-valg) er kittets dialog. Esc svarer nej.
- **Grupperne i Indstillinger** er kittets kasser med overskriften i kassens hoved.
- **Fliserne** er kittets: navnet står øverst, så tallet, så underteksten.
- **Dine login-stier** på Login er kittets filterchips med flueben ved den valgte.

### 2026-11-11 23:30 — Wi24rd UI Kit 2: knapper, chips og valg
#### Ændret
- **Knapper, statuschips, kontakter, afkrydsninger, radiogrupper, valglister og segmentknapper er kittets** (pakke 2.3.0). Programmets egne udgaver er væk fra `app.css`.
- **Udseende og Tekststørrelse** i Indstillinger er radioknapper i stedet for tre brede knapper.
- **Status** (Handl, Hold øje, Roligt, Ukendt) har kittets symbol foran ordet, så status aldrig hviler på farven alene.
- **7 dage / 30 dage, Alt / Kun angreb / Kun rutine og fanerne** på Land og Teknologi er kittets segmentknap.
- `npm run test:ui` kører med 8 GB til Node (testen løb tør ved 4 GB).

### 2026-11-11 23:30 — Wi24rd UI Kit 2: tekststørrelsen
#### Ændret
- **Tekststørrelsen (Lille, Normal, Stor) er kittets** (pakke 2.3.0, kopieret uændret til `ui/assets/kit2/`): `<html data-tekst="lille | stor">`, og hele fladen skaleres samlet (tekst, knapper, ikoner og luft), ikke kun skriften. ⌘+ ⌘− ⌘0, programmenuen og Indstillinger virker som før. Et valg gemt før skiftet oversættes én gang ved start.
- `tekst.js` er afløst af `gamle-valg.js`, som kun oversætter gamle valg (udseende og tekststørrelse). Skalaen `data-density` er ude af `design/wi24rd-tokens.json`; målene i `tokens.css` er faste.

### 2026-11-11 23:30 — Wi24rd UI Kit 2: skallen
#### Ændret
- **Programmet har fået Wi24rd UI Kit 2's opbygning** (pakke 2.2.0): mørk titellinje med tilbage, frem, Hjælp og Indstillinger; et bånd med Hent nu, landevælgeren (på Land lige nu) og Åbn vejledningen; en navigationsrude med domænerne øverst og de syv visninger nederst; en fodlinje med antal domæner, hentningens status, "Kun på denne maskine" og versionen. Det blå bånd, bølgerne og sidebjælken er væk.
- **Domænet vælges i træet** i navigationsruden ("Alle domæner" og hvert domæne) i stedet for i en liste i båndet. På Overblik, Siden sidst og Land åbner et klik på et domæne dets Angreb.
- **Hjælpen** er kittets sidepanel og ligger over indholdet som før.
- **Skriften** er Inter (kittets); Outfit bruges ikke længere.

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
