# Data og historik

## Hvor tallene ligger

Alt, programmet gemmer, ligger i én SQLite-fil, `huginn-muninn.sqlite`, i programmets datamappe på din computer:

| System | Datamappe |
|---|---|
| macOS | `~/Library/Application Support/com.wi24rd.huginn-muninn/` |
| Windows | `%APPDATA%\com.wi24rd.huginn-muninn\` |
| Linux | `~/.local/share/com.wi24rd.huginn-muninn/` (eller `$XDG_DATA_HOME`, hvis den er sat) |

API Tokenet ligger **ikke** i filen. Det ligger i din computers nøglering (macOS Nøglering eller Windows Credential Manager) under navnet `com.wi24rd.huginn-muninn`.

Størrelsen står under **Indstillinger → Data → Lokal database**.

## Hvad der gemmes

- **Hver færdig dag** pr. valgt domæne som optællinger: afviste forespørgsler fordelt på regel, land, netværk, sti, datacenter, handling og robot; trafiktal; robotter; login-tal; DMARC-tal; DNS-tal.
- **Dine indstillinger**: sprog, datoformat, land, udseende, tekststørrelse, notifikationsvalg, domænevalg, login-stier, teknologi-valg.
- **Hvad der er meldt**: hvilke domæner der har fået en notifikation for hvilket døgn, så det samme ikke meldes to gange. Den liste rydder sig selv.
- **Radar-tal** for dit land og verden.

Det, der **ikke** gemmes: IP-adresser (hentes aldrig), forespørgselsdelen af stier, lange koder i stier, fulde brugeragenter (kun robotnavnet), kodeord og tidspunkt på dagen (alt er hele døgn i UTC).

## Hvor langt tilbage

- Cloudflare gemmer **sikkerhedshændelser i en måned** og **trafiktal i et år**. Det er det, programmet kan hente.
- Programmet gemmer **alt, det har hentet, så længe du vil**. Intet slettes af sig selv. Så efter et år har du et års sikkerhedshændelser, selv om Cloudflare kun har en måned.
- **Hvad der findes** under Indstillinger → Data siger, hvor langt tilbage hver slags tal rækker, og **Ældste dag** er den første dag med tal.

## Huller

Et hul er en dag, programmet aldrig har hentet, typisk fordi computeren var slukket. Huller tegnes aldrig som 0, og summer siger "N af M dage".

- Var programmet slukket **kortere end en måned**, henter det de manglende dage selv, når det starter igen. Overblik siger "Huginn var ikke tændt fra … til …. Cloudflare havde stadig tallene, så intet mangler."
- Var det slukket **længere**, er de ældste dage væk hos Cloudflare og kan ikke hentes. Overblik siger, hvor mange dage der mangler for altid.
- **Indstillinger → Data → Huller** viser antallet, og et klik viser listen.

Derfor er **Start ved login** slået til som standard, og derfor siger programmet, at du bør åbne det mindst hver tredje uge, hvis du slår det fra.

## Hentningen

- Programmet spørger om i dag og i går, én dag ad gangen, så Cloudflare ikke skal sample over en lang periode. Ældre dage hentes kun, hvis de mangler.
- Højst to kald ad gangen og mindst 1,1 sekund imellem, for alle domæner tilsammen. Beder Cloudflare det vente, holder programmet pause og siger "Prøver igen kl. …" i statuspillen.
- Tallene til Login og "stoppet ved Edge" pr. robot hentes bagefter i langsomt tempo og kun for færdige dage.
- **Kvote hos Cloudflare** under Indstillinger → Data viser, hvor mange kald Cloudflare tillader, og hvor mange programmet brugte i sidste runde.
- **Hent nu** (⌘R, Windows Ctrl+R eller F5) starter en hentning med det samme.

## Eksport

Programmet har ingen eksportfunktion endnu. Databasen er en almindelig SQLite-fil, som kan åbnes med ethvert SQLite-værktøj, mens programmet ikke kører.

## Nulstil historik

**Indstillinger → Data → Nulstil historik…** sletter alle tal på din computer og henter dem forfra. Programmet spørger først og siger, hvilke datoer der forsvinder for altid, og hvor mange dage det er: Cloudflare har kun sikkerhedshændelser for den seneste måned og trafiktal for det seneste år, så alt ældre kan ikke hentes igen.

Det, der bliver: dine indstillinger, zonelisten, domænevalget, notifikationsvalget pr. domæne, login-stierne og teknologi-valgene. Filen selv bliver også; pladsen frigives.

## Glem API Tokenet

**Indstillinger → Forbindelse → Glem API Token…** sletter API Tokenet fra nøgleringen og dine valg pr. domæne (domænevalg, notifikationer pr. domæne, login-stier, teknologi-valg). Historikken bliver, så du kan forbinde igen senere uden at miste den.

## Nyere database

Åbner du en database, der er lavet af en nyere udgave af programmet, siger programmet "Den lokale database er fra en nyere version af programmet". Opdatér programmet; filen røres ikke.
