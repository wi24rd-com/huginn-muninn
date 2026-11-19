# Før du starter

## Hvad programmet gør

Huginn and Muninn henter tal fra din egen Cloudflare-konto og viser dem som et overblik over alle dine domæner: hvor mange angreb Cloudflare stoppede, hvilke robotter der læser dit site, hvad der skete på dine login-sider, hvem der besøger dit site, om dit website svarede, og hvordan det ser ud i resten af dit land og i verden. Hvert domæne får en status i klart sprog: **Roligt**, **Hold øje**, **Handl** eller **Ukendt** (ingen tal endnu).

Hver færdig dag gemmes på din computer. Cloudflare gemmer kun sikkerhedshændelser for den seneste måned, fejl pr. time og cache i ca. 30 dage og trafiktal for det seneste år; programmet gemmer, så længe du vil.

## Hvad programmet ikke gør

- Det **ændrer aldrig noget** hos Cloudflare. API Tokenet kan kun læse. Vil du blokere en robot eller lave en regel, gør du det selv i Cloudflares dashboard.
- Det **sender intet til Wi24rd**. Der er ingen telemetri, ingen konto hos Wi24rd og ingen server imellem. Programmet taler kun med `api.cloudflare.com`.
- Det **henter og gemmer ingen IP-adresser**. Stier gemmes uden forespørgselsdel, lange koder skjules, og brugeragenter gemmes kun som robotnavn.
- Det giver ingen garanti for, at alt bliver opdaget. Tallene er Cloudflares tal, og Cloudflare kan bruge stikprøver (sampling) over lange perioder, så tal kan være omtrentlige.

## Hvem det er til

Alle, der har et eller flere domæner hos Cloudflare og vil vide, om de skal gøre noget, uden at skulle klikke sig igennem Cloudflares dashboard domæne for domæne. Du behøver ikke være tekniker: standardvisningerne svarer på "skal jeg gøre noget?" af sig selv. Vil du vide mere, kan alt med tal bag sig klikkes.

Et IT-firma kan oprette ét API Token pr. kunde, der kun ser kundens domæne, og give kunden det. Så ser kunden kun sine egne domæner.

## Det kræver

| | |
|---|---|
| **Computer** | macOS 11 eller nyere på en Mac med Apple silicon. Version 1.1.0 er signeret og notariseret af Apple. Windows-udgaven er ikke udgivet endnu, og der findes ingen Linux-udgave; se [Installation](20-installation.md). |
| **Cloudflare** | En Cloudflare-konto med mindst ét domæne (en zone). Alle planer, også Free, kan bruges. |
| **Rettigheder** | Et API Token med to påkrævede rettigheder, der kun kan læse: **Zone › Read** og **Analytics › Read**. Én rettighed mere er valgfri: **Account Security Center Insights › Read**, hvis du vil se Cloudflares egne fund under Angreb. Programmet beder ikke om mere. |
| **Internet** | Programmet henter fra Cloudflare, mens det kører. Det kører kun, når computeren er tændt. |
| **Plads** | Den lokale database er lille (få megabyte efter en måned med få domæner). Du kan se størrelsen under **Indstillinger** › **Data**. |

Nogle tal kræver mere hos Cloudflare: tallene for **Mails sendt i dit domænes navn** kræver, at DMARC Management er slået til i Cloudflare, og nogle felter findes ikke på alle planer. Mangler de, står der "ikke tilgængelig", og resten virker.

## Privatliv i én tabel

| Hvad | Hvor | Hvem ser det |
|---|---|---|
| API Tokens (ét pr. forbindelse) | Din computers nøglering (macOS Nøglering eller Windows Credential Manager), én post pr. forbindelse | Kun programmets Rust-del. Vinduet kan ikke hente dem tilbage. |
| Tallene (historikken) | En SQLite-fil i programmets datamappe på din computer, se [Data og historik](70-data.md) | Kun dig |
| Dine valg (domæner, notifikationer, sprog) | Samme database | Kun dig |
| Notifikationer | Din egen computers notifikationstjeneste | Kun dig. Ingen mail, ingen webhook, ingen server. |
| Trafik ud af computeren | Kun til `api.cloudflare.com` (og Cloudflare Radar for landevisningen) | Cloudflare |

Et klik på et CVE-nummer åbner cve.org i din browser; programmet selv henter intet derfra.
