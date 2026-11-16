# Besøgende

Genvej: ⌘7 (Windows: Ctrl+7)

## Hvad viser den?

Besøgende viser, hvor mange der besøger dit site, hvorfra, hvor mange af besøgene der er kendte robotter, og hvilke sider der bliver set. Du ser ét domæne ad gangen eller alle samlet, for 7 dage, 30 dage, et år eller en periode, du selv vælger (højst et år).

Øverst står fliserne under overskriften **De seneste dage**. De er faste: de viser altid i går, de sidste 7 og 30 dage og det sidste år, uanset hvilken periode du har valgt. Perioden vælger du i båndet (**7 dage**, **30 dage**, **Et år**, eller **Fra** og **Til** under **Egne datoer**), og under fliserne står overskriften **Valgt periode: 11. okt. – 9. nov. 2026**. Alt under den overskrift følger perioden: grafen, landene, robotterne og siderne.

Fliserne:

| Flise | Betyder |
|---|---|
| **Unikke besøgende i går** | Antal forskellige IP-adresser, der besøgte sitet i går. |
| **Unikke besøgende på 7 dage** | Summen af dagstallene for de sidste 7 dage. |
| **Unikke besøgende på 30 dage** | Summen af dagstallene for de sidste 30 dage. |
| **Unikke besøgende på et år** | Summen af dagstallene for det sidste år. |
| **Besøg på 30 dage** | Cloudflares besøg for de sidste 30 dage. Linjen under siger, hvor mange af dem der kom fra kendte robotter. |

En sum over flere dage tæller den samme besøgende én gang pr. dag: kommer den samme person tilbage fem dage, tæller den fem gange. Mangler der dage, står der fx "6 af 7 dage" under tallet.

Med et land som filter hedder flisen **Besøg fra Danmark på 30 dage** (og med en side **Besøg på /produkter på 30 dage**) og tæller kun dem. Unikke besøgende kan ikke deles på lande eller sider; de fliser får linjen **alle lande** eller **alle sider**.

**Unikke besøgende pr. dag** er en graf over perioden. Søjlerne er blå, så de ikke ligner angreb. Ved **Et år** (og perioder over 90 dage) er der én søjle pr. uge, mandag til søndag, og hele året ses på én gang med månederne på aksen; mangler der dage i en uge, siger værktøjstippet fx "3 af 7 dage". En dag eller uge uden tal står tom; den er ikke 0, den er bare ikke hentet.

**Hvor besøgene kom fra** er en tabel med land, besøg og andel i perioden. De 10 største står fremme, resten bag **Vis alle**.

**Kendte robotter** viser besøg fra robotter, Cloudflare har verificeret, fordelt på de samme grupper som [Robotter](robotter.md) (søgemaskiner, AI-træning, overvågning …). Over tabellen står, hvor stor en del af trafikken det seneste år der kom fra kendte robotter og scannere.

**Mest besøgte sider** viser siderne (html) i perioden: sti, visninger, besøg udefra og andel. De 10 største står fremme, resten bag **Vis alle**. Under den står **Sider, der ikke findes**: stier, der svarede 404. Ikke alle forsøg står der: mange sites svarer med deres almindelige side på adresser, de ikke kender.

Derfor står der under 404-listen **Svarede med sidens almindelige side**: stier som `//crm/.env`, hvor svaret var lige så stort som forsiden (højst 25 % fra), og ingen kom udefra. Siden findes sandsynligvis ikke; sitet svarer bare med forsiden. Det er et skøn ud fra svarenes størrelse, samme regel som Følsomme filer på [Angreb](vaernet.md). Forsiden selv står aldrig der. Er forsiden ikke blandt siderne i perioden, deles listen ikke, og en linje siger det. Et site, der svarer 404 på ukendte adresser, får ingen sådan liste.

### Subdomæner

Har domænet trafik på mindst to navne i perioden, fx `wi24rd.com`, `www.wi24rd.com` og `shop.wi24rd.com`, står afsnittet **Subdomæner** under grafen. Det står kun, når ét domæne er valgt, og kun uden filter. Et domæne uden subdomæner ser ud som før.

Tabellen har kolonnerne **Subdomæne**, **Besøg**, **Forespørgsler**, **Angreb** og **Andel** (andelen af forespørgslerne). Domænet selv står med sit eget navn, `www` som `www.<domæne>`. Har domænet flere end 20 navne, står resten i en sidste række **Andre (n)**, og navnene kan åbnes under **Vis n subdomæner mere**. Tre rækker er ikke navne og kan ikke åbnes: **Kaldt med IP-adresse** (nogen kaldte sitet med IP-adressen i stedet for navnet), **Andet navn** (et navn uden for domænet) og **Ugyldigt navn**.

Tabellen følger den valgte periode. Programmet har tal pr. subdomæne fra ca. 30 dage før opdateringen til denne version. Når perioden går længere tilbage, står **Pr. subdomæne fra {dato}.** under tabellen, og tallene gælder kun dagene derfra. Under tabellen står altid: **Kun trafik gennem Cloudflare. Et subdomæne, der er «DNS only», har ingen tal her.** Et navn, hvor skyen er grå i Cloudflares DNS, går ikke gennem Cloudflare, så Cloudflare har ingen tal for det.

Klik på et subdomæne (navnet, **Besøg** eller **Forespørgsler**), og Besøgende viser kun det. Øverst står chippen **Subdomæne: shop.wi24rd.com**. Fliserne og graferne viser besøg og forespørgsler pr. dag på subdomænet. **Hvor besøgene kom fra** og **Kendte robotter** viser subdomænets egne tal, med samme tabel og klik som for domænet: et land viser besøgene fra det land pr. dag på subdomænet. **Mest besøgte sider** viser kun subdomænets sider. Har subdomænet ingen tal for et afsnit i perioden, står afsnittet ikke.

Unikke besøgende og IP-klasserne findes ikke pr. subdomæne hos Cloudflare. De står slet ikke, aldrig med hele domænets tal. En linje under chippen siger det: **Unikke besøgende findes ikke pr. subdomæne. Se dem for hele wi24rd.com.** Et klik på **hele wi24rd.com** fjerner filteret, ligesom **×** på chippen.

Tallet under **Angreb** i tabellen åbner [Angreb](vaernet.md) for subdomænet med samme periode (**Et år** bliver **Hele historikken**). Et tal, der er 0, kan ikke klikkes.

Lande og kendte robotter pr. subdomæne findes fra opdateringen til denne version og ca. 30 dage tilbage. Programmet henter de dage én gang igen pr. domæne. Lige efter en opdatering kommer tallene pr. subdomæne i løbet af de første minutter: indtil da står **–** (ikke hentet endnu) i stedet for et tal, og under tabellen står **Henter besøg og forespørgsler pr. subdomæne …** eller **Henter {n} dage endnu.**

I **Mest besøgte sider** er samme sti på to subdomæner to rækker. Rækken for et subdomæne har navnet ved stien, fx `/` med `shop.wi24rd.com`. Dage, der er gemt før opdateringen og er ældre end ca. 30 dage, kan ikke skilles ad og tæller som domænet selv. Sendte Cloudflare det højeste antal rækker for en dag, står noten **Kun de mest besøgte sider er med: Cloudflare sendte højst 10.000 rækker.**

## Hvad skal jeg gøre, når …

- **… tallet springer op en enkelt dag?** Klik på dagen i grafen. Den åbner trafikken den dag på [Angreb](vaernet.md), hvor du kan se, om det var robotter, et angreb eller rigtige besøgende.
- **… et land, du ikke venter, står højt?** Klik på landet. Grafen viser så besøgene fra det land pr. dag. Husk, at landet er det, Cloudflare så: en VPN tæller i VPN'ens land.
- **… besøgene er mange flere end de unikke besøgende?** Se linjen under **Besøg på 30 dage** og afsnittet **Kendte robotter**. Robotter, der udgiver sig for at være en almindelig browser, kan ikke ses og tæller som besøgende.
- **… /admin eller /wp-login.php står øverst under Sider, der ikke findes?** Det er næsten altid scannere, der prøver kendte adresser. Cloudflare svarede 404; du behøver ikke gøre noget.
- **… `.env`-filer eller `wp-config.php` står under Svarede med sidens almindelige side?** Det er scannere, der prøver kendte adresser, og sitet svarede med forsiden, ikke med filen. Vil du være sikker, så åbn adressen i din browser: ser du forsiden, er filen der ikke.
- **… der står "ikke tilgængelige"?** Cloudflare svarede ikke på de tal for det domæne. Resten af siden virker. Programmet spørger igen, næste gang det starter.
- **… et subdomæne har mange angreb eller forespørgsler?** Klik på det under **Subdomæner**. Så ser du det pr. dag, hvor besøgene kom fra, og hvilke sider der blev set. Klik på tallet under **Angreb**, så ser du angrebene pr. dag på [Angreb](vaernet.md) for samme subdomæne.
- **… mit subdomæne står der ikke?** Er det «DNS only» (grå sky) i Cloudflares DNS, har Cloudflare ingen tal for det. Ellers havde det ingen trafik i perioden. Se [Fejlfinding](../80-fejlfinding.md).
- **… der står "Ingen tal endnu" eller "ikke hentet endnu"?** Programmet henter i baggrunden efter de andre tal. Første gang efter opdateringen kan det tage lidt tid.

## Sådan gør du

1. Klik på fanen **Besøgende** (eller tryk ⌘7, Windows Ctrl+7).
2. Vælg domæne i listen **Domæner** til venstre, eller **Alle domæner** for alle samlet.
3. Vælg **7 dage**, **30 dage** eller **Et år** i båndet, eller vælg **Fra** og **Til**. **Nulstil** viser et år. Overskriften **Valgt periode** viser datoerne.

## Klik videre

- En **flise** vælger perioden: i går, 7 dage, 30 dage eller et år. Fliserne følger ikke perioden, så ingen af dem står markeret.
- En **dag i grafen** åbner **Trafik den …** på Angreb for den dag. En **uge i grafen** viser ugen (Fra og Til sættes til mandag og søndag).
- Et **land i tabellen** lægger et filter på Besøgende: grafen viser besøg fra det land pr. dag, og landet står som en chip øverst.
- En **gruppe under Kendte robotter** lægger et filter: grafen viser besøg fra den gruppe pr. dag.
- En **sti under Mest besøgte sider**, **Sider, der ikke findes** eller **Svarede med sidens almindelige side** lægger et filter: grafen viser visninger af siden pr. dag, og flisen Besøg følger siden.
- Et **subdomæne under Subdomæner** (navnet, **Besøg** eller **Forespørgsler**) viser Besøgende for det subdomæne, med chippen **Subdomæne: …**. Tallet under **Angreb** viser Angreb for subdomænet med samme periode. Filteret står i adressen, så tilbage og frem virker, og det nulstilles, når du vælger et andet domæne.
- En **sti på et subdomæne** under Mest besøgte sider viser visningerne af stien på det subdomæne. Med et subdomæne valgt kan et land, en robotgruppe eller en sti stå sammen med det; tallene er subdomænets.
- **hele {domæne}** i linjen under chippen fjerner filteret pr. subdomæne.
- Der er ellers højst ét filter ad gangen. **×** på chippen fjerner det.
- En flise uden tal, en dag uden tal eller med 0, tallene og andelene i tabellerne, procentlinjerne og noterne fører ingen steder hen og ser ikke klikbare ud. Det gælder også **Andel**, **Andre (n)**, rækkerne **Kaldt med IP-adresse**, **Andet navn** og **Ugyldigt navn**, et tal under Subdomæner, der er 0, og noterne under Subdomæner.

## Tal og begreber

- **Unikke besøgende** er Cloudflares unikke IP-adresser pr. dag. Robotter tæller med, og flere personer bag samme router tæller som én. Tallet findes et år tilbage hos Cloudflare, og programmet henter hele året.
- **Besøg** er Cloudflares egen definition: en sidevisning, der kom fra et andet site eller et direkte link. Det er det samme tal som "Visits" i Cloudflares dashboard. Det er Cloudflares estimat ud fra en stikprøve.
- **Kendte robotter** er robotter, Cloudflare har verificeret (fx Googlebot). Linjen "heraf … fra kendte robotter" tæller deres besøg de sidste 30 dage på de dage, hvor både besøg og robotter er hentet.
- **Kendte robotter og scannere** (andelen af trafikken det seneste år) bygger på Cloudflares [IP-klasser](../99-ordliste.md): søgemaskiner, overvågning, scannere, Tor og kendte dårlige adresser. Det er forespørgsler, ikke besøg.
- **Visninger** er antallet af html-svar for en sti. **Besøg udefra** er Cloudflares besøg på stien. Robotter tæller med. Stien vises uden forespørgselsstreng, og lange koder er skjult. De 50 mest viste sider gemmes pr. dag.
- Besøg pr. land, kendte robotter og sider findes en måned tilbage hos Cloudflare. Programmet gemmer dagene, så historikken vokser derfra.
- **Subdomæner**: besøg og forespørgsler pr. subdomæne er Cloudflares tal for navnet, som besøgende skrev det. Summen over alle rækker er domænets tal. **Angreb** tælles som på Angreb. Et subdomæne, der er «DNS only», har ingen tal.
- **Alle domæner** lægger domænerne sammen, så den samme besøgende på to domæner tæller to gange.
- Tallene er pr. dag i UTC, fra de tal, der er gemt på din computer. Ingen IP-adresser eller brugeragenter gemmes, kun tallene.
- Siden giver ingen notifikationer og ændrer ikke status i Overblik.
