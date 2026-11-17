# Notifikationer og baggrund

## Programmet kører videre, når vinduet er lukket

Lukker du vinduet, kører programmet videre i **menulinjen** (Windows: notifikationsområdet) og henter dine tal i baggrunden. På macOS forsvinder Dock-ikonet, til vinduet åbnes igen. Det er sådan, historikken bliver uden huller: programmet skal bare være i gang.

Menuen ved ravne-ikonet i menulinjen:

| Punkt | Gør |
|---|---|
| **Åbn Huginn** | Åbner vinduet. |
| **Siden sidst** | Åbner vinduet på [Siden sidst](40-visninger/siden-sidst.md). |
| **Hent nu** | Henter nye tal med det samme. |
| Statuslinjen | Fx "Henter 3/16" eller "Hentet for 12 min. siden". Kan ikke klikkes. |
| **Afslut** | Lukker programmet helt. Så hentes der ikke, før du starter det igen. |

**⌘Q**, mens vinduet er åbent, lukker også programmet helt. Et klik på det røde lukkeikon lukker kun vinduet.

**Start ved login** (under **Indstillinger** i gruppen **Start**) gør, at programmet starter uden vindue, kun i menulinjen, når du logger ind på computeren. Det slås til første gang en færdigbygget udgave starter, og programmet slår det aldrig til igen af sig selv, hvis du slår det fra. Så længe det er til, starter programmet også selv én gang om dagen, kl. 12, uden vindue, henter og lukker igen, hvis det ikke kørte i forvejen. Er det fra, henter programmet kun, mens det er åbent; åbn det mindst hver tredje uge, ellers kommer der huller i historikken, fordi Cloudflare kun gemmer en måned.

## Hvor notifikationerne kommer fra

Notifikationerne kommer fra din egen computers notifikationstjeneste. Aldrig fra en server, en mail eller en webhook, for der er ingen server. Det betyder også, at der kun kommer notifikationer, når computeren er tændt, og programmet kører.

macOS spørger om lov første gang, en notifikation vises. Siger du nej, kan du slå dem til igen under **Systemindstillinger → Notifikationer → Huginn and Muninn**. Samme sted slår du dem helt fra.

Et klik på en notifikation åbner **Siden sidst**.

## Hvornår du får besked

Det samlede niveau vælger du i opsætningen og under **Indstillinger** i gruppen **Notifikationer**:

| Niveau | Hvornår |
|---|---|
| **Kun alvorligt, højst én gang om dagen** (Rolig) | Én samlet besked om dagen, kun ved **Handl**, og ikke før kl. 03 UTC, når nattens tal er på plads. |
| **Ved alvorligt (anbefalet)** (Standard) | Besked efter hver hentning, hvor et domæne er gået til **Handl**. |
| **Også når et domæne skal holdes øje med** (Alt) | Som Standard, og også ved **Hold øje**. Går et domæne fra Hold øje til Handl samme døgn, kommer én besked mere. |

Pr. domæne kan du vælge **Følg standard**, **Alvorligt og hold øje** (som Alt), **Kun alvorligt** (som Standard) eller **Aldrig**. **Giv besked, når et domæne kræver handling** kan slås fra, hvis du kun vil se Handl i programmet.

Reglerne bag:

- **Handl** betyder usædvanligt mange angreb i går i forhold til ugen før, eller at noget slap igennem. Cloudflares rutine (labyrinten, Bot Fight Mode, browsertjekket) og gamle tal giver aldrig besked.
- Hvert domæne meldes **højst én gang pr. statusdøgn**, så natten (hvor status bruger døgnet før) aldrig melder det samme igen.
- Et domæne uden tal (Ukendt) giver aldrig besked.

## Hvad notifikationerne kan sige

Teksten er den samme, som står under **Hvorfor** i Overblik, på programmets sprog og i dit datoformat. Stier og kodeord står aldrig i en notifikation, og IP-adresser findes ikke i programmet.

| Besked om | Hvornår | Niveau |
|---|---|---|
| **Usædvanligt mange angreb** | Angreb i går mod ugen før, med hvad de gik efter. | Handl / Hold øje |
| **Ligner målrettet** | Angrebene steg her, men ikke på dine andre domæner, og en stor del kom fra ét netværk. Sammenligner kun med dine valgte domæner, kræver mindst 3, og giver først besked, når døgnet efter er hentet. Til da står det som foreløbigt. Kommer også på et roligt domæne. | Standard og Alt |
| **Ligner rekognoscering** og **Ligner DNS-rekognoscering** | Mange forskellige stier eller mange opslag på navne, der ikke findes, på én dag. Kun første døgn i en episode. | Kun hvis **Giv besked ved »Ligner rekognoscering«** er slået til |
| **Din server svarer ikke** | En stor del af svarene i går var fejl, fordi Cloudflare ikke kunne nå eller finde din server (fejl 520–527 og 530). Havde tre eller flere timer i træk fejl, nævner beskeden tidsrummet i din lokale tid: "mellem kl. 01.00 og 05.00". | Følger status |
| **Usædvanlig meget trafik** | Langt flere forespørgsler end normalt på én dag. | Følger status |
| **Mails i dit domænes navn** | En ny afsender fejlede DMARC, eller andelen steg markant. Kræver DMARC Management hos Cloudflare. | Hold øje |
| **Kodeord fra kendte læk** | Login-forsøg med lækkede kodeord. Mindst Hold øje; Handl, hvis serveren svarede 2xx eller 302. | Følger status |
| **Domænets tilstand** | Domænet er sat på pause, ikke aktivt, mistænkt for phishing eller i udviklertilstand. | Følger status |
| **Flere angreb på en teknologi** | Kun hvis du har valgt **Giv mig besked** på teknologien under [Teknologi pr. domæne](40-visninger/teknologi.md). Melder uanset niveau, men ikke for et domæne på Aldrig. | Uanset niveau |

## Hvad skal jeg gøre, når …

- **… du får for mange beskeder?** Vælg **Kun alvorligt, højst én gang om dagen**, eller sæt enkelte domæner til **Kun alvorligt** eller **Aldrig**.
- **… du ingen beskeder får?** Tjek, at programmet kører (ravne-ikonet i menulinjen), at **Start ved login** er til, at macOS tillader notifikationer for programmet, og at domænet ikke står på **Aldrig**. Roligt giver aldrig besked; det er meningen.
- **… du hellere vil have en mail?** Det kan programmet ikke. Det har ingen server, og det er med vilje: intet forlader din computer.
