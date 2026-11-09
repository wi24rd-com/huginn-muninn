# Besøgende

Genvej: ⌘7 (Windows: Ctrl+7)

## Hvad viser den?

Besøgende viser, hvor mange der besøger dit site, og hvorfra. Du ser ét domæne ad gangen eller alle samlet, for 7 dage, 30 dage, et år eller en periode, du selv vælger (højst et år).

Fliserne:

| Flise | Betyder |
|---|---|
| **Unikke besøgende i går** | Antal forskellige IP-adresser, der besøgte sitet i går. |
| **Unikke besøgende på 7 dage** | Summen af dagstallene for de sidste 7 dage. |
| **Unikke besøgende på 30 dage** | Summen af dagstallene for de sidste 30 dage. |
| **Unikke besøgende på et år** | Summen af dagstallene for det sidste år. |
| **Besøg på 30 dage** | Cloudflares besøg for de sidste 30 dage. |

En sum over flere dage tæller den samme besøgende én gang pr. dag: kommer den samme person tilbage fem dage, tæller den fem gange. Mangler der dage, står der fx "6 af 7 dage" under tallet.

**Unikke besøgende pr. dag** er en graf over perioden. En dag uden tal står tom; den er ikke 0, den er bare ikke hentet.

**Hvor besøgene kom fra** er en tabel med land, besøg og andel i perioden. De 10 største står fremme, resten bag **Vis alle**.

## Hvad skal jeg gøre, når …

- **… tallet springer op en enkelt dag?** Klik på dagen i grafen. Den åbner trafikken den dag på [Angreb](vaernet.md), hvor du kan se, om det var robotter, et angreb eller rigtige besøgende.
- **… et land, du ikke venter, står højt?** Klik på landet. Grafen viser så besøgene fra det land pr. dag. Husk, at landet er det, Cloudflare så: en VPN tæller i VPN'ens land.
- **… der står "Besøg pr. land er ikke tilgængelige"?** Cloudflare svarede ikke på besøgene for det domæne. Resten af siden virker. Programmet spørger igen, næste gang det starter.
- **… der står "Ingen tal endnu"?** Programmet henter året i baggrunden efter de andre tal. Første gang efter opdateringen kan det tage lidt tid.

## Sådan gør du

1. Klik på **Besøgende** i sidebjælken (eller tryk ⌘7, Windows Ctrl+7).
2. Vælg domæne i vælgeren øverst, eller **Samlet** for alle.
3. Vælg **7 dage**, **30 dage** eller **Et år**, eller vælg **Fra** og **Til**. **Nulstil** viser et år.

## Klik videre

- En **flise** vælger perioden: i går, 7 dage, 30 dage eller et år.
- En **dag i grafen** åbner **Trafik den …** på Angreb for den dag.
- Et **land i tabellen** lægger et filter på Besøgende: grafen viser besøg fra det land pr. dag, og landet står som en chip øverst. **×** på chippen fjerner filtret.
- En flise uden tal, en dag uden tal eller med 0, tallene i tabellen og andelen fører ingen steder hen og ser ikke klikbare ud.

## Tal og begreber

- **Unikke besøgende** er Cloudflares unikke IP-adresser pr. dag. Robotter tæller med, og flere personer bag samme router tæller som én. Tallet findes et år tilbage hos Cloudflare, og programmet henter hele året.
- **Besøg** er Cloudflares egen definition: en sidevisning, der kom fra et andet site eller et direkte link. Det er det samme tal som "Visits" i Cloudflares dashboard. Det er Cloudflares estimat ud fra en stikprøve.
- Besøg pr. land findes en måned tilbage hos Cloudflare. Programmet gemmer dagene, så historikken vokser derfra.
- **Samlet** lægger domænerne sammen, så den samme besøgende på to domæner tæller to gange.
- Tallene er pr. dag i UTC, fra de tal, der er gemt på din computer. Ingen IP-adresser gemmes, kun tallet.
- Siden giver ingen notifikationer og ændrer ikke status i Overblik.
