# Opdatering

> **Status i dag:** Programmet har ingen indbygget opdatering endnu. Det tjekker ikke selv efter nye udgaver og henter intet fra Wi24rd. Automatisk opdatering fra et offentligt repo, hvor alt, der hentes, signeres og tjekkes, er planlagt.

## Sådan opdaterer du nu

1. Vælg **Afslut** i menuen ved ravne-ikonet (eller ⌘Q, mens vinduet er åbent), så programmet er lukket helt.
2. Hent den nye udgave, og erstat **Huginn and Muninn** i din Programmer-mappe.
3. Åbn programmet igen.

Din historik, dine indstillinger og dit API Token bliver, hvor de er. Selve udskiftningen rører hverken databasen eller nøgleringen.

## Databasen opgraderes ved første start

Udgaven med flere forbindelser opgraderer den lokale database af sig selv, første gang den starter. Intet går tabt: dit API Token bliver forbindelse 1 med alle dine valg.

Bagefter kan version 1.0.0 ikke åbne databasen. Den siger "Den lokale database er fra en nyere version af programmet (skema 19)." og lader filen være.

**Vil du måske tilbage til 1.0.0?** Så tag en kopi af databasefilen `huginn-muninn.sqlite`, før du åbner den nye udgave første gang. Programmet skal være lukket, mens du kopierer. Hvor filen ligger, står i [Data og historik](70-data.md).

## Hvis databasen er nyere end programmet

Åbner du en ældre udgave af programmet efter at have kørt en nyere, siger programmet "Den lokale database er fra en nyere version af programmet". Så skal du tilbage til den nyere udgave; filen røres ikke.

## Hvilken udgave har jeg?

Knappen **i** øverst til højre (**Om Huginn and Muninn**) viser versionen. Den står også nederst til højre i vinduet. Udgivelser står i `CHANGELOG.md` i kildekoden.
