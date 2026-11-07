# Fejlfinding

Programmet viser kun sine egne fejltekster i klart sprog, aldrig rå tekniske fejl. Her står hver tekst, hvad den betyder, og hvad du gør.

## Fejltekster om forbindelsen

| Teksten | Betyder | Gør |
|---|---|---|
| **Ingen forbindelse til Cloudflare. Tjek din internetforbindelse.** | Computeren kan ikke nå `api.cloudflare.com`. | Tjek nettet. Programmet prøver selv igen og viser **Prøv igen nu**. |
| **Cloudflare svarer ikke lige nu.** | Cloudflare svarede ikke, eller svarede med en serverfejl. | Vent. Programmet prøver igen af sig selv. |
| **Cloudflare beder os vente lidt (for mange forespørgsler).** | Cloudflare har en grænse for, hvor mange kald der må komme. | Ingenting. Programmet holder pause og siger "Prøver igen kl. …" i statuspillen. |
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
| **Cloudflare afviste API Tokenet. Det kan være udløbet eller mangle rettigheder.** | Cloudflare kender ikke tokenet, eller det er udløbet. | Opret et nyt API Token efter [Opsætning](30-opsaetning.md), og indsæt det under **Indstillinger → Forbindelse → Skift API Token…**. |
| **API Tokenet er ikke aktivt (status: …). Opret et nyt API Token, der kun kan læse.** | Tokenet er deaktiveret eller udløbet hos Cloudflare. | Opret et nyt. |
| **API Tokenet kan ikke se nogen domæner: det mangler rettigheden Zone › Zone › Read, eller domænet står ikke under Zone Resources.** | Tokenet må ikke se zonelisten, eller ingen zoner er inkluderet. | Ret tokenet hos Cloudflare: **Permissions** skal have `Zone › Zone › Read`, og **Zone Resources** skal inkludere domænet (eller All zones). |
| **API Tokenet mangler rettigheden Zone › Analytics › Read, så programmet kan ikke læse tallene.** | Tokenet må se zoner, men ikke tal. | Tilføj `Zone › Analytics › Read` hos Cloudflare. |
| **Der er ikke gemt noget API Token. Indsæt dit API Token under Forbind med Cloudflare.** | Nøgleringen har ikke noget token (fx efter **Glem API Token**). | Forbind igen. |
| **Zonen findes ikke, eller API Tokenet har ikke adgang til den.** | Et domæne er fjernet fra kontoen, eller tokenet ser det ikke længere. | Fravælg domænet under **Indstillinger → Domæner**, eller ret tokenet. |

## Fejltekster om computeren

| Teksten | Betyder | Gør |
|---|---|---|
| **Nøgleringen kunne ikke bruges.** | Programmet kunne ikke læse eller skrive i macOS Nøglering / Credential Manager. | Prøv igen. Spørger macOS om adgang til nøgleringen, så tillad det. |
| **Den lokale database kunne ikke bruges.** | Filen i datamappen kunne ikke åbnes eller skrives. | Tjek, at der er plads på disken, og at datamappen ikke er skrivebeskyttet. Se [Data og historik](70-data.md). |
| **Den lokale database er fra en nyere version af programmet (skema …).** | Du har åbnet en ældre udgave af programmet med en nyere database. | Opdatér programmet. Filen røres ikke. |
| **Start ved login kunne ikke læses eller ændres.** | Styresystemet afviste at oprette eller fjerne autostart-posten. | Prøv igen. På macOS er posten filen `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist`; se [Afinstallation](90-afinstallation.md). |
| **Der opstod en fejl i programmet.** | En fejl, programmet ikke selv kan forklare. | Prøv igen. Sker det igen, er det værd at melde som fejl. |

## Andre tekster

| Teksten | Betyder |
|---|---|
| **Ugyldigt zone-id.** / **Ugyldig dato.** / **Filteret i adressen kan ikke bruges.** | Adressen i tilbage/frem-historikken var forkert. Gå til Overblik og klik dig frem igen. |
| **Du kan højst have 20 egne login-stier pr. domæne. Fjern en, før du tilføjer en ny.** | Grænsen for egne login-stier er nået. |
| **»Fra« skal ligge før eller på »Til«.** | Datovælgeren fik en periode bagvendt. |
| **ikke tilgængelig** | Din Cloudflare-plan har ikke felterne til netop de tal (fx Login eller "stoppet ved Edge" pr. robot). Resten virker. |

## Det er ikke en fejl

- **Status "Ukendt"** eller **"Ingen tal endnu – henter"**: domænet er lige valgt. Vent på hentningen.
- **"opdateres …"** ved status om natten (til kl. 03 UTC): i går er ikke hentet færdigt endnu.
- **Tallene passer ikke med Cloudflares dashboard**: dashboardet viser samplede estimater og lokal tid; programmet viser hele døgn i UTC og kun det, der er hentet. Store forskelle over lange perioder skyldes typisk sampling.
- **En gul linje om et hul**: programmet var slukket. Se [Data og historik](70-data.md).

## Meld en fejl

Programmet er open source. Fejl meldes i kildekodens repo (linket står under **Indstillinger → Om**, når det er udgivet). Skriv aldrig dit API Token, account-id eller zone-id i en fejlmelding.
