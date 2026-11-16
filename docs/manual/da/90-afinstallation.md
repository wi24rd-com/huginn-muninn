# Afinstallation

Programmet lægger fem ting på din computer: programmet selv, to autostart-poster, et API Token pr. forbindelse i nøgleringen og en datamappe. Følg trinene i rækkefølge, så intet bliver liggende.

## macOS

### 1. Slå Start ved login fra i programmet

Åbn programmet, og slå **Start ved login** fra under **Indstillinger** (tandhjulet) i gruppen **Start**. Det fjerner filerne `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist` og `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.dagligt.plist`.

Glemmer du det, bliver filerne liggende og peger på et program, der ikke findes længere. De kan også slettes i hånden:

```
rm ~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist
rm ~/Library/LaunchAgents/com.wi24rd.huginn-muninn.dagligt.plist
```

### 2. Glem alle forbindelser

Vælg **Indstillinger** › **Forbindelse** › **Glem API Token…** og bekræft med **Glem**. Det sletter API Tokenet fra macOS Nøglering.

Har du flere forbindelser, klikker du **Glem…** i hver række og bekræfter med **Glem**, til der er én tilbage. Den sidste glemmer du med **Glem API Token…**. Hver gang slettes forbindelsens API Token fra nøgleringen.

Vil du gøre det i hånden: åbn macOS' nøgleringsprogram (Keychain Access), søg efter `com.wi24rd.huginn-muninn`, og slet alle posterne. Den første forbindelse bruger posten `cloudflare-api-token`, de næste `cloudflare-api-token-2`, `cloudflare-api-token-3` og så videre.

Overvej også at **tilbagekalde hvert API Token hos Cloudflare**: log ind, åbn **My Profile › API Tokens**, og slet tokenet med det navn, du gav det. Så kan det ikke bruges af nogen, uanset hvad.

### 3. Afslut programmet

Vælg **Afslut** i menuen ved ravne-ikonet i menulinjen, eller ⌘Q, mens vinduet er åbent. Ravne-ikonet skal være væk.

### 4. Slet programmet

Træk **Huginn and Muninn.app** fra Programmer-mappen til papirkurven.

### 5. Slet datamappen

Historikken ligger i datamappen og slettes ikke af sig selv. Vil du af med den:

```
rm -r ~/Library/Application\ Support/com.wi24rd.huginn-muninn
```

Vil du beholde historikken til senere, så lad mappen ligge. Installerer du programmet igen, finder det den.

### 6. Notifikationer

macOS husker din tilladelse til notifikationer under **Systemindstillinger → Notifikationer**. Posten forsvinder, når programmet er slettet, eller kan fjernes der.

## Windows

Der findes endnu ingen Windows-udgave. Når den kommer, er stederne:

| Hvad | Hvor |
|---|---|
| Autostart | Værdien under `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` (slå **Start ved login** fra i programmet først) |
| Daglig start | Den planlagte opgave `Wi24rd Huginn and Muninn dagligt` (slå **Start ved login** fra i programmet først) |
| API Tokens | Windows Credential Manager, posterne under `com.wi24rd.huginn-muninn`: `cloudflare-api-token`, `cloudflare-api-token-2` og så videre, én pr. forbindelse |
| Datamappe | `%APPDATA%\com.wi24rd.huginn-muninn\` |

## Linux

Der findes endnu ingen Linux-udgave. Datamappen vil ligge i `~/.local/share/com.wi24rd.huginn-muninn/`.

## Tjekliste

- [ ] Start ved login slået fra (ingen `.plist` i `~/Library/LaunchAgents/`)
- [ ] Alle forbindelser glemt i programmet (og API Tokens tilbagekaldt hos Cloudflare)
- [ ] Programmet afsluttet (intet ravne-ikon i menulinjen)
- [ ] Programmet slettet fra Programmer
- [ ] Datamappen slettet, eller bevidst beholdt
