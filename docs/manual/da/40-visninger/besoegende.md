# Besøgende

Genvej: ⌘7 (Windows: Ctrl+7)

## Hvad viser den?

Besøgende viser, hvor mange der besøger dit site, hvorfra, hvor mange af besøgene der er kendte robotter, og hvilke sider der bliver set. Du ser ét domæne ad gangen eller alle samlet, for 7 dage, 30 dage, et år eller en periode, du selv vælger (højst et år).

Øverst står fliserne under overskriften **De seneste dage**. De er faste: de viser altid i går, de sidste 7 og 30 dage og det sidste år, uanset hvilken periode du har valgt. Under dem står vælgeren (7 dage, 30 dage, Et år, Fra og Til), og under den overskriften **Valgt periode: 11. okt. – 9. nov. 2026**. Alt under den overskrift følger perioden: grafen, landene, robotterne og siderne.

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

## Hvad skal jeg gøre, når …

- **… tallet springer op en enkelt dag?** Klik på dagen i grafen. Den åbner trafikken den dag på [Angreb](vaernet.md), hvor du kan se, om det var robotter, et angreb eller rigtige besøgende.
- **… et land, du ikke venter, står højt?** Klik på landet. Grafen viser så besøgene fra det land pr. dag. Husk, at landet er det, Cloudflare så: en VPN tæller i VPN'ens land.
- **… besøgene er mange flere end de unikke besøgende?** Se linjen under **Besøg på 30 dage** og afsnittet **Kendte robotter**. Robotter, der udgiver sig for at være en almindelig browser, kan ikke ses og tæller som besøgende.
- **… /admin eller /wp-login.php står øverst under Sider, der ikke findes?** Det er næsten altid scannere, der prøver kendte adresser. Cloudflare svarede 404; du behøver ikke gøre noget.
- **… `.env`-filer eller `wp-config.php` står under Svarede med sidens almindelige side?** Det er scannere, der prøver kendte adresser, og sitet svarede med forsiden, ikke med filen. Vil du være sikker, så åbn adressen i din browser: ser du forsiden, er filen der ikke.
- **… der står "ikke tilgængelige"?** Cloudflare svarede ikke på de tal for det domæne. Resten af siden virker. Programmet spørger igen, næste gang det starter.
- **… der står "Ingen tal endnu" eller "ikke hentet endnu"?** Programmet henter i baggrunden efter de andre tal. Første gang efter opdateringen kan det tage lidt tid.

## Sådan gør du

1. Klik på **Besøgende** i sidebjælken (eller tryk ⌘7, Windows Ctrl+7).
2. Vælg domæne i vælgeren øverst, eller **Samlet** for alle.
3. Vælg **7 dage**, **30 dage** eller **Et år** under fliserne, eller vælg **Fra** og **Til**. **Nulstil** viser et år. Overskriften **Valgt periode** viser datoerne.

## Klik videre

- En **flise** vælger perioden: i går, 7 dage, 30 dage eller et år. Fliserne følger ikke perioden, så ingen af dem står markeret.
- En **dag i grafen** åbner **Trafik den …** på Angreb for den dag. En **uge i grafen** viser ugen (Fra og Til sættes til mandag og søndag).
- Et **land i tabellen** lægger et filter på Besøgende: grafen viser besøg fra det land pr. dag, og landet står som en chip øverst.
- En **gruppe under Kendte robotter** lægger et filter: grafen viser besøg fra den gruppe pr. dag.
- En **sti under Mest besøgte sider**, **Sider, der ikke findes** eller **Svarede med sidens almindelige side** lægger et filter: grafen viser visninger af siden pr. dag, og flisen Besøg følger siden.
- Der er højst ét filter ad gangen. **×** på chippen fjerner det.
- En flise uden tal, en dag uden tal eller med 0, tallene og andelene i tabellerne, procentlinjerne og noterne fører ingen steder hen og ser ikke klikbare ud.

## Tal og begreber

- **Unikke besøgende** er Cloudflares unikke IP-adresser pr. dag. Robotter tæller med, og flere personer bag samme router tæller som én. Tallet findes et år tilbage hos Cloudflare, og programmet henter hele året.
- **Besøg** er Cloudflares egen definition: en sidevisning, der kom fra et andet site eller et direkte link. Det er det samme tal som "Visits" i Cloudflares dashboard. Det er Cloudflares estimat ud fra en stikprøve.
- **Kendte robotter** er robotter, Cloudflare har verificeret (fx Googlebot). Linjen "heraf … fra kendte robotter" tæller deres besøg de sidste 30 dage på de dage, hvor både besøg og robotter er hentet.
- **Kendte robotter og scannere** (andelen af trafikken det seneste år) bygger på Cloudflares [IP-klasser](../99-ordliste.md): søgemaskiner, overvågning, scannere, Tor og kendte dårlige adresser. Det er forespørgsler, ikke besøg.
- **Visninger** er antallet af html-svar for en sti. **Besøg udefra** er Cloudflares besøg på stien. Robotter tæller med. Stien vises uden forespørgselsstreng, og lange koder er skjult. De 50 mest viste sider gemmes pr. dag.
- Besøg pr. land, kendte robotter og sider findes en måned tilbage hos Cloudflare. Programmet gemmer dagene, så historikken vokser derfra.
- **Samlet** lægger domænerne sammen, så den samme besøgende på to domæner tæller to gange.
- Tallene er pr. dag i UTC, fra de tal, der er gemt på din computer. Ingen IP-adresser eller brugeragenter gemmes, kun tallene.
- Siden giver ingen notifikationer og ændrer ikke status i Overblik.
