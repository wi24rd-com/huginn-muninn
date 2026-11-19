# Installation

> **Status i dag:** Version 1.1.0 findes til macOS på en Mac med Apple silicon. Der findes endnu ingen udgave til Windows eller Linux.

## macOS

Kræver macOS 11 eller nyere og en Mac med Apple silicon.

1. Hent `Huginn-Muninn-1.1.0.zip` fra udgivelsen på GitHub.
2. Åbn zip-filen. Den pakker **Huginn and Muninn** ud.
3. Træk **Huginn and Muninn** til din Programmer-mappe (`/Applications`).
4. Åbn programmet.

Programmet er signeret med Developer ID og notariseret af Apple. macOS åbner det derfor uden advarslen om en ukendt udvikler. Første gang kan macOS spørge, om du vil åbne et program, der er hentet fra internettet.

### Byg programmet selv

Du kan også bygge programmet fra kildekoden. Du skal bruge Rust (installeret med rustup) og Node 20 eller nyere. I en kopi af kildekoden:

```
npm install
npm run tauri build
```

Resultatet er **Huginn and Muninn.app**. En udgave, du selv bygger, er ikke signeret, medmindre du signerer den med din egen Developer ID.

Vil du kun prøve programmet, kan du køre `npm run tauri dev`. Så starter det i udviklingstilstand; i den tilstand slår programmet ikke **Start ved login** til (se nedenfor).

## Windows

Programmet er bygget til at kunne køre på Windows (de samme genveje med Ctrl, notifikationsområdet i stedet for menulinjen, Credential Manager i stedet for Nøgleringen), men der findes endnu ingen Windows-udgave, og den er ikke prøvet.

## Linux

Der findes ingen Linux-udgave.

## Hvad der sker, første gang du åbner programmet

1. En guide med fire trin åbner: **Velkommen**, **Dit API Token**, **Dine domæner** og **Besked**. Fanerne øverst kan først bruges, når et API Token er gemt. Se [Opsætning](30-opsaetning.md).
2. Første gang en færdigbygget udgave starter, slår programmet **Start ved login** til. Det betyder, at programmet starter uden vindue, kun i menulinjen, når du logger ind på computeren, og henter dine tal i baggrunden. Du kan slå det fra i guidens trin 4 eller under **Indstillinger** (tandhjulet øverst til højre) i gruppen **Start**; programmet slår det aldrig til igen af sig selv.
3. Programmet opretter sin datamappe og den lokale database. Se [Data og historik](70-data.md).
4. Første gang programmet viser en notifikation, spørger macOS, om det må. Se [Notifikationer og baggrund](60-notifikationer.md).

Programmet kører kun én gang ad gangen. Starter du det igen, vises det vindue, der allerede findes.

## Hvad der bliver lagt på din computer

| Hvad | Hvor (macOS) | Hvornår |
|---|---|---|
| Programmet | `/Applications/Huginn and Muninn.app` (hvis du lægger det der) | Ved installation |
| Datamappe med database | `~/Library/Application Support/com.wi24rd.huginn-muninn/` | Første start |
| API Tokens | macOS Nøglering under `com.wi24rd.huginn-muninn`, én post pr. forbindelse | Når du forbinder eller tilføjer et API Token |
| Start ved login | `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist` | Første start af en færdigbygget udgave |
| Daglig start | `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.dagligt.plist` | Sammen med Start ved login |

Hvordan du fjerner det hele igen, står i [Afinstallation](90-afinstallation.md).
