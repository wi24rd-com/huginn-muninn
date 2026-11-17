# Drift

Genvej: ⌘8 (Windows: Ctrl+8)

## Hvad viser den?

Drift svarer på to spørgsmål: svarede dit website, og hvor meget klarede Cloudflare uden at spørge din server? Du ser ét domæne ad gangen eller alle samlet, for 7 dage, 30 dage, et år eller en periode, du selv vælger (højst et år). Perioden vælger du i båndet (**7 dage**, **30 dage**, **Et år**, eller **Fra** og **Til** under **Egne datoer**), og overskriften **Valgt periode: …** viser datoerne. Perioden går til og med i går: tallene findes kun for hele dage.

Fanen har to afsnit: **Svarede dit website?** og **Hvor meget Cloudflare klarede selv**.

### Svarede dit website?

Her står de svar, der var fejl, som Cloudflare så dem. Kun besøgendes forespørgsler er talt med.

Fliserne:

| Flise | Betyder |
|---|---|
| **Kunne ikke nås** | Svar, hvor Cloudflare ikke kunne nå eller finde din server (Cloudflares fejl 520–527 og 530). Linjen under siger, hvor stor en del af alle svar det var. |
| **Din server svarede med en fejl** | Din server var oppe, men svarede selv med en fejl (svar 500–599). Det er oftest et program på serveren, der fejler, ikke et nedbrud. |
| **Timer med fejl** | Antal timer, hvor mindst 5 svar ikke kunne nås, og de var mindst 20 % af timens svar. |
| **Forsiden svarede ikke** | **Ja**, hvis forsiden (`/`) svarede med en fejl mindst én gang i perioden; ellers **Nej**. |

En flise med et tal viser sine rækker i tabellen, og et klik mere viser alle fejl igen. Et **0**, et **Nej** og et **–** kan ikke klikkes. Mangler der dage, står der fx "Tal for 6 af 7 dage" under tallet.

Under fliserne kan der stå en gul linje: **Mellem kl. 01.00 og 05.00 kunne Cloudflare ikke nå din server.** Den står, når tre eller flere timer i træk havde fejl. Fejlede forsiden også i de timer, siger linjen i stedet **Mellem kl. 01.00 og 05.00 svarede dit website ikke: Cloudflare kunne ikke nå din server.** Klokkeslættene er din lokale tid. Med flere dage eller flere domæner står dagen og domænet foran. Var der fejl, men ingen timer i træk, og forsiden virkede, står der **Nogle sider svarede med fejl, men forsiden virkede.**

**Svar med fejl pr. dag** er en graf over perioden. Søjlerne er gule, så de ikke ligner angreb. Et klik på en dag viser dagens 24 timer i samme graf (**Svar med fejl pr. time den …**), med klokkeslæt i din lokale tid. Et klik på en time viser timens fejl i tabellen. **Tilbage til perioden** over grafen fører tilbage, og **Se trafikken den dag på Angreb** åbner **Trafik den …** på [Angreb](vaernet.md). Ved **Et år** (og perioder over 90 dage) er der én søjle pr. uge, mandag til søndag, og et klik på en uge viser ugen. En dag uden tal står tom; den er ikke 0, den er bare ikke hentet.

Tabellen har kolonnerne **Domæne**, **Hvad skete der**, **Svar** og **Første og sidste time**. Hver række er én slags fejl på ét domæne, flest svar først, og fejlen står i klart sprog med svarkoden i parentes, fx "Cloudflare kunne ikke få forbindelse til din server i tide (svar 522)". Med **Alle domæner** åbner domænet Drift for det domæne. De 10 største står fremme, resten bag **Vis alle … rækker**.

Nederst i afsnittet står altid: **Programmet ser kun trafik, der går gennem Cloudflare. Svarer Cloudflare med en gammel kopi, ser den besøgende siden, selv om din server er nede.**

### Subdomæner med fejl

Har ét domæne fejl på mindst to navne i perioden, fx `wi24rd.com` og `www.wi24rd.com`, står tabellen **Subdomæner med fejl** med kolonnerne **Subdomæne**, **Svar med fejl** og **Kunne ikke nås**. Den står kun, når ét domæne er valgt. Rækkerne **Kaldt med IP-adresse**, **Andet navn**, **Ugyldigt navn** og **Andre subdomæner** er ikke navne og kan ikke åbnes.

Klik på et subdomæne, og Drift viser kun dets fejl. Øverst står chippen **Subdomæne: www.wi24rd.com**. Pr. subdomæne findes kun **Kunne ikke nås** og fejlene pr. dag; tabellen viser **Første og sidste dag** i stedet for timer. De tre andre fliser viser **–** med **Findes ikke pr. subdomæne**, og en linje siger: **Timer med fejl, forsiden og din servers egne fejl findes ikke pr. subdomæne. Se dem for hele wi24rd.com.** **×** på chippen eller **hele wi24rd.com** fjerner filteret, og det nulstilles, når du vælger et andet domæne.

### Hvor meget Cloudflare klarede selv

Cloudflare gemmer kopier af dine filer og kan svare uden at spørge din server. Afsnittet viser, hvor ofte det skete.

Fliserne:

| Flise | Betyder |
|---|---|
| **Fra Cloudflares lager** | Andelen af forespørgslerne, Cloudflare svarede på med en kopi fra sit lager. |
| **Data, din server slap for at sende** | Mængden af data, der kom fra lageret, i læselige enheder (kB, MB, GB). |
| **Svaret af Cloudflare selv** | Forespørgsler, Cloudflare svarede på uden at spørge din server, fx en omdirigering. |
| **Hentet hos din server** | Forespørgsler, hvor svaret blev hentet hos din server. |

En flise viser sine rækker i tabellen. Under fliserne står andelene på én linje; de summer altid til 100 %.

Grafen viser forespørgsler pr. dag med to søjler side om side: **Fra Cloudflares lager** (grøn) og **Hentet hos din server** (blå). Et klik på en dag viser dagen.

Tabellen **Hvad Cloudflare gjorde** viser, hvad der skete med forespørgslerne, med antal forespørgsler og data:

| Teksten | Betyder |
|---|---|
| **Fundet i Cloudflares lager** | Cloudflare havde en kopi og svarede med den. |
| **Ikke i lageret endnu, så svaret blev hentet hos din server** | Filen kan gemmes, men lå der ikke endnu. |
| **Kopien i lageret var for gammel, så svaret blev hentet hos din server** | Kopien var udløbet. |
| **En gammel kopi fra lageret, fordi din server ikke svarede** | Cloudflare kunne ikke få en ny og viste den gamle. |
| **Svaret måtte ikke gemmes, så det blev hentet hos din server** | Din server sagde, at svaret ikke må gemmes. |
| **Din server bekræftede, at kopien i lageret stadig gælder** | Cloudflare spurgte kort og brugte sin kopi. |
| **En gammel kopi fra lageret, mens Cloudflare henter en ny** | Den besøgende fik kopien med det samme. |
| **Sider, der hentes hos din server hver gang** | Cloudflare gemmer dem ikke. Det gælder de fleste sider. |
| **Svaret af Cloudflare selv, uden at spørge din server** | Fx en omdirigering til https. |
| **Andet** | En værdi, Cloudflare ikke har beskrevet. |

Har der været gamle kopier, fordi serveren ikke svarede, står linjen **Gamle kopier vist, fordi din server ikke svarede: …**.

Under tabellen står to noter. Den første: **De fleste sider på små websites hentes hos din server hver gang. Det er Cloudflares standard: billeder, CSS og JavaScript gemmes, selve siderne ikke. Det betyder ikke, at noget er i stykker.** Den anden: **Data er det, Cloudflare sendte til den besøgende. Det kan være mindre end det, din server ville have sendt.**

Cachen gælder altid hele domænet. Med et subdomæne valgt står der kun **Cache gælder hele domænet.**, aldrig domænets tal under subdomænets navn.

## Hvad skal jeg gøre, når …

- **… der står en gul linje med et tidsrum?** I de timer kunne Cloudflare ikke nå din server. Spørg din udbyder, om der var en driftsforstyrrelse, og se "Hvad gør jeg?" på [Overblik](overblik.md), hvis domænet også står på Hold øje.
- **… Din server svarede med en fejl har et tal?** Serveren var oppe, men et program på den fejlede. Vis rækkerne i tabellen, og giv din webmand tidspunkterne.
- **… der står "Nogle sider svarede med fejl, men forsiden virkede."?** Det er ofte robotter, der rammer en adresse, som fejler. Dit website var ikke nede.
- **… næsten alt er Hentet hos din server?** Det er normalt for små websites; se noten under tabellen. Programmet giver ingen råd, der kræver ændringer hos Cloudflare.
- **… der står "–" eller "Tallene er ikke hentet for perioden endnu."?** Programmet henter i baggrunden efter de andre tal. Første gang efter opdateringen tager det lidt tid. Se [Fejlfinding](../80-fejlfinding.md).
- **… der står "Fejl pr. time fra …"?** Perioden går længere tilbage, end tallene findes. Fejl pr. time findes ca. 30 dage tilbage hos Cloudflare, og programmet gemmer dagene, så historikken vokser derfra.

## Sådan gør du

1. Klik på fanen **Drift** (eller tryk ⌘8, Windows Ctrl+8).
2. Vælg domæne i listen **Domæner** til venstre, eller **Alle domæner** for alle samlet.
3. Vælg **7 dage**, **30 dage** eller **Et år** i båndet, eller vælg **Fra** og **Til**.

## Klik videre

- En **flise med et tal** viser sine rækker i tabellen; chippen **Kun: …** siger, hvad der er valgt, og **×** fjerner det.
- En **dag i grafen** viser dagens timer. En **time** viser timens fejl i tabellen. En **uge** viser ugen.
- **Tilbage til perioden** fører tilbage fra en dag, og **Se trafikken den dag på Angreb** åbner Trafik den dag.
- Et **domæne i tabellen** (med Alle domæner) åbner Drift for domænet med samme periode.
- Et **subdomæne under Subdomæner med fejl** viser fejlene for det subdomæne.
- En **dag i cachegrafen** viser dagen.
- Linjen **Din server svarer ikke** på [Overblik](overblik.md) og linket i [Siden sidst](siden-sidst.md) åbner Drift for domænet og dagen.
- Alt står i adressen, så tilbage og frem virker.
- En flise med 0, Nej eller –, en dag uden fejl, tallene i tabellerne, cachetabellen, linjerne og noterne fører ingen steder hen og ser ikke klikbare ud.

## Tal og begreber

- **Kunne ikke nås** er Cloudflares fejl 520–527 og 530: din server svarede ikke, afviste forbindelsen, var for længe om at svare, havde et problem med den sikre forbindelse, eller Cloudflare kunne ikke slå serverens navn op. De enkelte koder står i [ordlisten](../99-ordliste.md).
- **Din server svarede med en fejl** er svar 500–599 fra din egen server.
- **En time med fejl** har mindst 5 svar, der ikke kunne nås, og de er mindst 20 % af timens svar. Tre eller flere timer i træk giver linjen med tidsrummet.
- Tallene er Cloudflares estimat ud fra en stikprøve, og kun besøgendes forespørgsler tæller med. Det, Cloudflare selv blokerede, er ikke fejl.
- Programmet ser kun trafik, der går gennem Cloudflare. Et subdomæne, der er «DNS only», og besøg direkte på serverens IP-adresse ses ikke. Viser Cloudflare en gammel kopi, mens serveren er nede, ser den besøgende siden, og det tæller ikke som en fejl.
- Fejl pr. time og cache findes ca. 30 dage tilbage hos Cloudflare. Programmet gemmer dagene, så historikken vokser derfra. Fejl pr. subdomæne findes fra ca. 30 dage før opdateringen.
- Et tal, der ikke er hentet, står som **–** eller "Ikke hentet", aldrig som 0.
- Dagene er hele døgn i UTC; klokkeslættene vises i din lokale tid.
- **Alle domæner** lægger domænerne sammen. Et samlet tal står kun for de dage, hvor alle valgte domæner er hentet.
- Drift giver ingen notifikationer og ændrer ikke status. Kun **Din server svarer ikke** i [Overblik](overblik.md) kan give Hold øje (aldrig Handl), og den regel er den samme som før, nu med fejl 530. Din servers egne fejl og "nogle sider svarede med fejl" ændrer aldrig status og giver aldrig en besked.
- Der kræves ingen ny rettighed på dit API Token.
