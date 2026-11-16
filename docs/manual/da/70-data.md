# Data og historik

## Hvor tallene ligger

Alt, programmet gemmer, ligger i én SQLite-fil, `huginn-muninn.sqlite`, i programmets datamappe på din computer:

| System | Datamappe |
|---|---|
| macOS | `~/Library/Application Support/com.wi24rd.huginn-muninn/` |
| Windows | `%APPDATA%\com.wi24rd.huginn-muninn\` |
| Linux | `~/.local/share/com.wi24rd.huginn-muninn/` (eller `$XDG_DATA_HOME`, hvis den er sat) |

API Tokens ligger **ikke** i filen. De ligger i din computers nøglering (macOS Nøglering eller Windows Credential Manager) under navnet `com.wi24rd.huginn-muninn`, ét pr. forbindelse. Posternes navne står i [Afinstallation](90-afinstallation.md).

Størrelsen står under **Indstillinger** › **Data** › **Lokal database**.

## Hvad der gemmes

- **Hver færdig dag** pr. valgt domæne som optællinger: afviste forespørgsler fordelt på regel, land, netværk, sti, datacenter, handling og robot; trafiktal; robotter; login-tal; DMARC-tal; DNS-tal; unikke besøgende (ét tal pr. dag, ingen IP-adresser); forespørgsler pr. IP-klasse; besøg pr. land; besøg pr. kendt robot (Cloudflares kategori); de 50 mest viste sider pr. dag (stien uden forespørgselsstreng, med svarkode, visninger og besøg, og subdomænet, når det ikke er domænet selv); forespørgsler, besøg og angreb pr. subdomæne.
- **Dine indstillinger**: sprog, datoformat, land, udseende, tekststørrelse, notifikationsvalg, domænevalg, login-stier, teknologi-valg.
- **Dine forbindelser**: kontoens navn hos Cloudflare, dit eget navn, hvor mange konti API Tokenet ser, hvornår forbindelsen blev oprettet, og hvilke domæner hver forbindelse ser. Aldrig selve API Tokenet.
- **Hvad der er meldt**: hvilke domæner der har fået en notifikation for hvilket døgn, så det samme ikke meldes to gange. Den liste rydder sig selv.
- **Radar-tal** for dit land og verden.

**Subdomænerne** gemmes som navnet under domænet, fx `www` eller `shop`, i din egen database på din computer, som domænenavnene. Navnet er det, besøgende skrev, så programmet renser det først: højst 20 navne pr. domæne og dag (resten lægges sammen som "andre"), og et opslag med en IP-adresse, et navn uden for domænet eller et ugyldigt navn gemmes kun som en af de tre slags, aldrig selve adressen eller navnet. Subdomænerne står aldrig i notifikationer, i menulinjen eller i Overblik.

Det, der **ikke** gemmes: IP-adresser (hentes aldrig), forespørgselsdelen af stier, lange koder i stier, fulde brugeragenter (kun robotnavnet), kodeord og tidspunkt på dagen (alt er hele døgn i UTC).

## Hvor langt tilbage

- Cloudflare gemmer **sikkerhedshændelser i en måned** og **trafiktal i et år**. Det er det, programmet kan hente.
- Programmet gemmer **alt, det har hentet, så længe du vil**. Intet slettes af sig selv. Så efter et år har du et års sikkerhedshændelser, selv om Cloudflare kun har en måned.
- **Hvad der findes** under Indstillinger › Data siger, hvor langt tilbage hver slags tal rækker, og **Ældste dag** er den første dag med tal.

## Huller

Et hul er en dag, programmet aldrig har hentet, typisk fordi computeren var slukket. Huller tegnes aldrig som 0, og summer siger "N af M dage".

- Var programmet slukket **kortere end en måned**, henter det de manglende dage selv, når det starter igen. Overblik siger "Huginn var ikke tændt fra … til …. Cloudflare havde stadig tallene, så intet mangler."
- Var det slukket **længere**, er de ældste dage væk hos Cloudflare og kan ikke hentes. Overblik siger, hvor mange dage der mangler for altid.
- **Indstillinger** › **Data** › **Huller** viser antallet, og et klik viser listen.

Derfor er **Start ved login** slået til som standard. Med den starter programmet også selv én gang om dagen og henter, så der ikke kommer huller, selv om du har valgt **Afslut**. Og derfor siger programmet, at du bør åbne det mindst hver tredje uge, hvis du slår det fra.

## Hentningen

- Programmet spørger om i dag og i går, én dag ad gangen, så Cloudflare ikke skal sample over en lang periode. Ældre dage hentes kun, hvis de mangler.
- Antallet af afviste er Cloudflares estimat: hver hændelse i stikprøven oplyser, hvor mange den står for, og programmet lægger det sammen. Første gang programmet starter efter opdateringen 8. november 2026, henter det de sidste 30 dage igen i det normale tempo, så tallene kan sammenlignes. Ældre dage kan Cloudflare ikke levere igen; de beholder de talte rækker.
- Højst to kald ad gangen og mindst 1,1 sekund imellem, for alle domæner tilsammen. Beder Cloudflare det vente, holder programmet pause og siger "Prøver igen kl. …" i fodlinjen nederst i vinduet.
- Tallene til Login og "stoppet ved Edge" pr. robot hentes bagefter i langsomt tempo og kun for færdige dage.
- Til **Besøgende** hentes unikke besøgende med årets trafiktal og besøg pr. land til sidst, ét kald pr. domæne. Besøg pr. land findes en måned tilbage hos Cloudflare; programmet gemmer dagene, så historikken vokser derfra. Første gang programmet starter efter opdateringen 9. november 2026, henter det året én gang til for de dage, det allerede har, så de får unikke besøgende. Kun det tal skrives; de andre tal for dagene ændres ikke. Første start efter opdateringen med Kendte robotter gør det samme én gang for IP-klasserne (art `h_ipklasse`); kun de skrives. Kendte robotter (art `b_robot`) og de mest besøgte sider (arterne `b_side`, `b_side_b` og `b_side_bytes`: visninger, besøg og svarenes bytes i alt pr. dag og sti) hentes hver med sit eget kald for de samme dage som besøgene, også for dage, hvis besøg allerede er hentet. Bytes bruges kun til at skille stier, der svarer med forsiden, fra rigtige sider. Første start efter opdateringen 10. november 2026 henter siderne for den seneste måned én gang til, så de får bytes; ældre dage beholder deres tal uden. Afviser Cloudflare et af kaldene, står det som "ikke tilgængelig" ved det afsnit, og resten virker.
- **Subdomæner**: forespørgsler og besøg pr. subdomæne hentes til sidst i samme langsomme tempo, ét kald pr. domæne for de færdige dage, der mangler. Angreb pr. subdomæne kommer med i det kald, der i forvejen henter angrebene, så det koster intet ekstra. Første start efter opdateringen med subdomæner henter de sidste ca. 30 dage én gang (tre kald pr. domæne: trafik, angreb og siderne pr. subdomæne), så historikken pr. subdomæne starter ca. 30 dage før opdateringen. Ældre dage har ingen tal pr. subdomæne. Afviser Cloudflare navnet i angrebene, henter programmet angrebene uden det, til det starter igen, og domænets tal er de samme.
- **Kvote hos Cloudflare** under Indstillinger › Data viser, hvor mange kald Cloudflare tillader, og hvor mange programmet brugte i sidste runde.
- **Hent nu** (⌘R, Windows Ctrl+R eller F5) starter en hentning med det samme.

## Eksport

Programmet har ingen eksportfunktion endnu. Databasen er en almindelig SQLite-fil, som kan åbnes med ethvert SQLite-værktøj, mens programmet ikke kører.

## Nulstil historik

**Indstillinger** › **Data** › **Nulstil historik…** sletter alle tal på din computer og henter dem forfra. Programmet spørger først og siger, hvilke datoer der forsvinder for altid, og hvor mange dage det er: Cloudflare har kun sikkerhedshændelser for den seneste måned og trafiktal for det seneste år, så alt ældre kan ikke hentes igen.

Det, der bliver: dine indstillinger, zonelisten, domænevalget, notifikationsvalget pr. domæne, login-stierne og teknologi-valgene. Filen selv bliver også; pladsen frigives.

## Glem en forbindelse

Med én forbindelse: **Indstillinger** › **Forbindelse** › **Glem API Token…** sletter API Tokenet fra nøgleringen og dine valg pr. domæne (domænevalg, notifikationer pr. domæne, login-stier, teknologi-valg). Historikken bliver, så du kan forbinde igen senere uden at miste den.

Med flere forbindelser: **Indstillinger** › **Forbindelser** › **Glem…** i forbindelsens række sletter kun dens API Token fra nøgleringen. Kun domæner, som ingen anden forbindelse ser, mister deres valg. De andre forbindelser henter videre, og historikken bliver. Den sidste forbindelse glemmer du med **Glem API Token…** som ovenfor.

## Nyere database

Åbner du en database, der er lavet af en nyere udgave af programmet, siger programmet "Den lokale database er fra en nyere version af programmet (skema …)". Opdatér programmet; filen røres ikke.

Udgaven med flere forbindelser og subdomæner opgraderer databasen første gang, den starter: først til skema 19 (forbindelserne), så til skema 20 (to nye tabeller over, hvilke dage der er hentet pr. subdomæne; ingen gemte tal ændres). Bagefter kan version 1.0.0 ikke åbne den og siger "(skema 20)". Vil du kunne gå tilbage, så tag en kopi af filen først; se [Opdatering](85-opdatering.md).
