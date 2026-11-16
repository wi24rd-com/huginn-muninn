# Fejlfinding

Programmet viser kun sine egne fejltekster i klart sprog, aldrig rå tekniske fejl. Her står hver tekst, hvad den betyder, og hvad du gør.

## Fejltekster om forbindelsen

| Teksten | Betyder | Gør |
|---|---|---|
| **Ingen forbindelse til Cloudflare. Tjek din internetforbindelse.** | Computeren kan ikke nå `api.cloudflare.com`. | Tjek nettet. Programmet prøver selv igen og viser **Prøv igen nu**. |
| **Ingen forbindelse til Cloudflare, så … kunne ikke hentes. Prøver igen af sig selv.** (i båndet: **Ingen forbindelse · prøver igen**) | Et eller flere domæner kunne ikke hentes, fordi der ikke var net, fx lige efter, at Mac'en har sovet. | Ingenting. Programmet prøver en ny runde af sig selv efter 2 minutter, og straks når det opdager, at maskinen har sovet. **Hent nu** prøver med det samme. |
| **Cloudflare svarer ikke lige nu.** | Cloudflare svarede ikke, eller svarede med en serverfejl. | Vent. Programmet prøver igen af sig selv. |
| **Cloudflare beder os vente lidt (for mange forespørgsler).** | Cloudflare har en grænse for, hvor mange kald der må komme. | Ingenting. Programmet holder pause og siger "Prøver igen kl. …" i fodlinjen nederst i vinduet. |
| **Cloudflare Radar svarer ikke lige nu.** | Kun landevisningen er ramt. | Dine egne tal virker stadig. Programmet prøver igen. |
| **Cloudflare svarede med HTTP …** / **Cloudflare svarede med en fejl: …** / **Uventet svar fra Cloudflare.** | Et svar, programmet ikke kunne bruge. | Prøv **Hent nu**. Sker det igen og igen, er det værd at melde som fejl. |
| **Cloudflare kunne ikke levere tallene for …** | Én dag kunne ikke hentes. | Dagen bliver et hul, og programmet prøver igen senere, så længe Cloudflare stadig har dagen. |
| **Tallene fra Cloudflare Radar er ikke opdateret: … Dine egne tal virker stadig.** | Radar svarede ikke. | Ingenting. |
| **Hentningen i baggrunden er stoppet: …** | Hentningen stoppede på en fejl. | Læs fejlen i teksten; typisk et API Token, der er udløbet. |

## Fejltekster om API Tokenet

| Teksten | Betyder | Gør |
|---|---|---|
| **Feltet er tomt. Indsæt dit API Token.** | Du klikkede Gem og forbind uden noget i feltet. | Indsæt API Tokenet. |
| **Det indsatte ligner ikke et API Token fra Cloudflare (kun bogstaver, tal og tegn uden mellemrum).** | Der kom noget andet med, fx et mellemrum eller et linjeskift. | Kopiér API Tokenet igen fra Cloudflare, uden andet omkring. |
| **Cloudflare afviste API Tokenet. Det kan være udløbet eller mangle rettigheder.** | Cloudflare kender ikke tokenet, eller det er udløbet. Afviser Cloudflare alle dine forbindelser, er det som med én: hentningen stopper, og status er rød. | Opret et nyt API Token efter [Opsætning](30-opsaetning.md), og indsæt det under **Indstillinger** (tandhjulet) › **Forbindelse** › **Skift API Token…**. |
| **Cloudflare afviste API Tokenet «navn». Dens domæner hentes ikke.** (rød linje på Overblik med knappen **Skift API Token…**; i fodlinjen: **1 API Token afvist**) | Du har flere forbindelser, og Cloudflare afviser API Tokenet på den forbindelse, der står i teksten. Det er slettet, udløbet, eller rettighederne er skåret ned. De andre forbindelser henter videre. Menuen ved ravne-ikonet nævner det også. | Opret et nyt API Token hos Cloudflare efter [Opsætning](30-opsaetning.md). Klik **Skift API Token…** i den røde linje; det åbner **Indstillinger** ved forbindelsens række. Indsæt det nye API Token, og klik **Gem og forbind**. |
| **Dette API Token ser de samme domæner som «navn». Det er ikke tilføjet.** | Du brugte **Tilføj API Token…** (eller guiden) med et API Token, der ser præcis de samme domæner som forbindelsen «navn», typisk det samme API Token en gang til. | Intet. Forbindelsen findes allerede. Skal API Tokenet erstatte det gamle, så brug **Skift API Token…** i forbindelsens række. |
| **API Tokenet er ikke aktivt (status: …). Opret et nyt API Token, der kun kan læse.** | Tokenet er deaktiveret eller udløbet hos Cloudflare. | Opret et nyt. |
| **API Tokenet kan ikke se nogen domæner: det mangler rettigheden Zone › Read, eller domænet står ikke under Include · All zones eller Specific zone.** | Tokenet må ikke se zonelisten, eller ingen zoner er inkluderet. | Ret tokenet hos Cloudflare: under **Policies** skal **DNS & Zones** have **Zone › Read**, og **Include** skal dække domænet (**All zones** eller **Specific zone**). |
| **API Tokenet mangler rettigheden Analytics › Read, så programmet kan ikke læse tallene.** | Tokenet må se zoner, men ikke tal. | Tilføj **Analytics › Read** under **Analytics & Logs** hos Cloudflare. |
| **Der er ikke gemt noget API Token. Indsæt dit API Token under Forbind med Cloudflare.** | Nøgleringen har ikke noget token (fx efter **Glem API Token**). | Forbind igen. |
| **Zonen findes ikke, eller API Tokenet har ikke adgang til den.** | Et domæne er fjernet fra kontoen, eller tokenet ser det ikke længere. | Fravælg domænet med **Vælg domæner** på Overblik, eller ret tokenet. |

## Fejltekster om computeren

| Teksten | Betyder | Gør |
|---|---|---|
| **Nøgleringen kunne ikke bruges.** | Programmet kunne ikke læse eller skrive i macOS Nøglering / Credential Manager. | Prøv igen. Spørger macOS om adgang til nøgleringen, så tillad det. |
| **Den lokale database kunne ikke bruges.** | Filen i datamappen kunne ikke åbnes eller skrives. | Tjek, at der er plads på disken, og at datamappen ikke er skrivebeskyttet. Se [Data og historik](70-data.md). |
| **Den lokale database er fra en nyere version af programmet (skema …).** | Du har åbnet en ældre udgave af programmet med en nyere database. | Opdatér programmet. Filen røres ikke. Se [Opdatering](85-opdatering.md). |
| **Start ved login kunne ikke læses eller ændres.** | Styresystemet afviste at oprette eller fjerne autostart-posten. | Prøv igen. På macOS er posten filen `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist`; se [Afinstallation](90-afinstallation.md). |
| **Der opstod en fejl i programmet.** | En fejl, programmet ikke selv kan forklare. | Prøv igen. Sker det igen, er det værd at melde som fejl. |

## Andre tekster

| Teksten | Betyder |
|---|---|
| **Ugyldigt zone-id.** / **Ugyldig dato.** / **Filteret i adressen kan ikke bruges.** | Adressen i tilbage/frem-historikken var forkert. Gå til Overblik og klik dig frem igen. |
| **Du kan højst have 20 egne login-stier pr. domæne. Fjern en, før du tilføjer en ny.** | Grænsen for egne login-stier er nået. |
| **»Fra« skal ligge før eller på »Til«.** | Datovælgeren fik en periode bagvendt. |
| **ikke tilgængelig** | Din Cloudflare-plan har ikke felterne til netop de tal (fx Login eller "stoppet ved Edge" pr. robot). Resten virker. |

## Mine domæner forsvandt, da jeg satte et nyt API Token ind

I version 1.0.0 kunne programmet kun have ét API Token. Brugte du **Skift API Token…** med et API Token til en anden Cloudflare-konto, erstattede det det første. Så forsvandt den første kontos domæner og dine valg for dem. Tal, der allerede var hentet, blev aldrig slettet.

Nu får hver konto sin egen forbindelse. Til en konto mere bruger du **Tilføj API Token…** under **Indstillinger** › **Forbindelse**. **Skift API Token…** erstatter stadig API Tokenet på én forbindelse, og formularen siger det: "Har du en konto mere? Brug Tilføj API Token… i stedet."

Har du allerede skiftet, tilføjer du den første konto igen med **Tilføj API Token…**. Har du ikke API Tokenet længere, opretter du et nyt hos Cloudflare. Vælg derefter domænerne i dialogen **Domæner**. Deres tal er der stadig.

## Et domæne står under to forbindelser i Domæner

Har du et API Token til hele kontoen og et til ét af dens domæner, ser begge forbindelser det domæne. I dialogen **Domæner** står det under begge, og under den nyeste står "Hentes med «navn»": det hentes kun med den ældste forbindelses API Token. Det er ét valg; slår du det fra under den ene, er det fra under begge.

Hedder de to forbindelser det samme (begge har kontoens navn hos Cloudflare), åbner programmet **Omdøb forbindelsen**, når den nye er tilføjet. Giv den et navn, du kan kende den på, fx kundens.

## Subdomænerne

| Teksten | Betyder | Gør |
|---|---|---|
| **Cloudflare svarede ikke på subdomænerne for dette domæne.** (under **Subdomæner** i Besøgende eller Angreb) | Cloudflare afviste forespørgslen pr. subdomæne for domænet, fx fordi planen ikke har feltet. Domænets egne tal hentes og vises som før, bare ikke pr. subdomæne. | Ingenting. Afviste Cloudflare navnet i hovedhentningen af angrebene, spørger programmet igen uden navnet, til det starter næste gang. |

**Mit subdomæne står der ikke.** Afsnittet **Subdomæner** viser kun navne med trafik i den valgte periode, og kun trafik, der går gennem Cloudflare. Tjek i Cloudflares DNS, om navnet har orange sky. Står det som «DNS only» (grå sky), går trafikken direkte til serveren, og Cloudflare har ingen tal for det. Har domænet kun trafik på ét navn, står afsnittet slet ikke. Programmet har tal pr. subdomæne fra ca. 30 dage før opdateringen; ældre dage har ingen tal pr. subdomæne, og så står der **Pr. subdomæne fra …**.

## Det er ikke en fejl

- **Status "Ukendt"** eller **"Ingen tal endnu – henter"**: domænet er lige valgt. Vent på hentningen.
- **"opdateres …"** ved status om natten (til kl. 03 UTC): i går er ikke hentet færdigt endnu.
- **Tallene passer ikke med Cloudflares dashboard**: dashboardet viser samplede estimater og lokal tid; programmet viser hele døgn i UTC og kun det, der er hentet. Store forskelle over lange perioder skyldes typisk sampling.
- **En gul linje om et hul**: programmet var slukket. Se [Data og historik](70-data.md).

## Meld en fejl

Programmet er open source. Fejl meldes i kildekodens repo på GitHub. Skriv aldrig dit API Token, account-id eller zone-id i en fejlmelding.
