# Installation

> **Status i dag:** Programmet er ikke udgivet. Der findes endnu ingen færdige installationspakker at hente, og det er kun bygget og prøvet på macOS. Dette kapitel beskriver, hvad der gælder nu, og hvad der er planlagt. Kapitlet opdateres, når den første udgave udgives.

## macOS

### I dag: byg programmet selv

Du skal bruge Rust (installeret med rustup) og Node 20 eller nyere. Derefter, i en kopi af kildekoden:

```
npm install
npm run tauri build
```

Resultatet er programmet **Huginn and Muninn.app**. Træk det til din Programmer-mappe (`/Applications`), og åbn det som ethvert andet program.

Vil du kun prøve programmet uden at bygge en færdig udgave, kan du køre `npm run tauri dev`. Så starter det i udviklingstilstand; i den tilstand slår programmet ikke **Start ved login** til (se nedenfor).

### Planlagt: en færdig pakke

Når programmet udgives, bliver det som en pakke, du henter og åbner. Signering hos Apple (notarisering) er planlagt, men ikke på plads endnu. Indtil da vil macOS advare, hvis du åbner en pakke, der er hentet fra nettet, fordi den ikke er signeret.

## Windows

Programmet er bygget til at kunne køre på Windows (de samme genveje med Ctrl, notifikationsområdet i stedet for menulinjen, Credential Manager i stedet for Nøgleringen), men der findes endnu ingen Windows-udgave, og den er ikke prøvet. Signering på Windows er planlagt.

## Linux

En Linux-udgave er planlagt, men findes ikke endnu.

## Hvad der sker, første gang du åbner programmet

1. Programmet viser **Forbind med Cloudflare**, fordi der ikke er gemt noget API Token. Kun **Indstillinger** kan åbnes i sidebjælken, indtil du har forbundet. Se [Opsætning](30-opsaetning.md).
2. Første gang en færdigbygget udgave starter, slår programmet **Start ved login** til. Det betyder, at programmet starter uden vindue, kun i menulinjen, når du logger ind på computeren, og henter dine tal i baggrunden. Du kan slå det fra under **Indstillinger → Generelt → Start ved login**; programmet slår det aldrig til igen af sig selv.
3. Programmet opretter sin datamappe og den lokale database. Se [Data og historik](70-data.md).
4. Første gang programmet viser en notifikation, spørger macOS, om det må. Se [Notifikationer og baggrund](60-notifikationer.md).

Programmet kører kun én gang ad gangen. Starter du det igen, vises det vindue, der allerede findes.

## Hvad der bliver lagt på din computer

| Hvad | Hvor (macOS) | Hvornår |
|---|---|---|
| Programmet | `/Applications/Huginn and Muninn.app` (hvis du lægger det der) | Ved installation |
| Datamappe med database | `~/Library/Application Support/com.wi24rd.huginn-muninn/` | Første start |
| API Token | macOS Nøglering, posten `com.wi24rd.huginn-muninn` | Når du forbinder |
| Start ved login | `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist` | Første start af en færdigbygget udgave |
| Daglig start | `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.dagligt.plist` | Sammen med Start ved login |

Hvordan du fjerner det hele igen, står i [Afinstallation](90-afinstallation.md).
