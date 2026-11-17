# Drift

Genvej: ⌘8 (Windows: Ctrl+8)

## Hvad viser den?

Drift svarer på to spørgsmål: svarede dit website, og hvor meget klarede Cloudflare selv? Du ser ét domæne ad gangen eller alle samlet, for 7 dage, 30 dage, et år eller en periode, du selv vælger (højst et år). Perioden vælger du i båndet (**7 dage**, **30 dage**, **Et år**, eller **Fra** og **Til** under **Egne datoer**), og overskriften **Valgt periode: …** viser datoerne. Perioden går til og med i går: tallene findes kun for hele dage.

Fanen har to afsnit: **Svarede dit website?** og **Hvor meget Cloudflare klarede selv**.

### Svarede dit website?

Her står de svar, der var fejl, som Cloudflare så dem. Kun besøgendes forespørgsler er talt med.

Fanen siger **dit website**, fordi der er to slags. Har du din egen server (hos en udbyder eller hos dig selv), er dit website den server, og Cloudflare står foran den. Ligger dit website hos Cloudflare selv (Workers eller Pages), er der ingen server bag: så er det Cloudflare, der svarer. Begge slags læses rigtigt, og samme note står i fanen.

Fliserne:

| Flise | Betyder |
|---|---|
| **Kunne ikke nås** | Svar, hvor Cloudflare ikke kunne nå eller finde din server (Cloudflares fejl 520–527 og 530). Linjen under siger, hvor stor en del af alle svar det var. De fejl kræver en server: ligger dit website hos Cloudflare, er tallet 0. |
| **Dit website svarede med en fejl** | Alle andre svar med fejl: din server var oppe, men svarede selv med en fejl (svar 500–599), eller dit website hos Cloudflare (Workers eller Pages) fejlede. Det er oftest et program, der fejler, ikke et nedbrud. Hvert svar tælles én gang, så de to første fliser tilsammen er alle svar med fejl. |
| **Timer med fejl** | Antal timer, hvor mindst 5 svar ikke kunne nås, og de var mindst 20 % af timens svar. |
| **Forsiden svarede ikke** | **Ja**, hvis forsiden (`/`) svarede med en fejl mindst én gang i perioden; ellers **Nej**. |

En flise med et tal viser sine rækker i tabellen, og et klik mere viser alle fejl igen. Et **0**, et **Nej** og et **–** kan ikke klikkes. Mangler der dage, står der fx "Tal for 6 af 7 dage" under tallet.

Under fliserne kan der stå en gul linje: **Mellem kl. 01.00 og 05.00 kunne Cloudflare ikke nå din server.** Den står, når tre eller flere timer i træk havde fejl. Fejlede forsiden også i de timer, siger linjen i stedet **Mellem kl. 01.00 og 05.00 svarede dit website ikke: Cloudflare kunne ikke nå din server.** Klokkeslættene er din lokale tid. Med flere dage eller flere domæner står dagen og domænet foran. Svarede forsiden med en fejl uden for sådan et tidsrum, står der i stedet **Dit website svarede ikke hver gang: forsiden svarede med en fejl … gange.** Det er linjen, et website hos Cloudflare får, når det fejler: den siger ikke, at Cloudflare ikke kunne nå en server, for der er ingen. Var der fejl, men ingen timer i træk, og forsiden virkede, står der **Nogle sider svarede med fejl, men forsiden virkede.**

**Svar med fejl pr. dag** er en graf over perioden. Søjlerne er gule, så de ikke ligner angreb. Et klik på en dag viser dagens 24 timer i samme graf (**Svar med fejl pr. time den …**), med klokkeslæt i din lokale tid. Et klik på en time viser timens fejl i tabellen. **Tilbage til perioden** over grafen fører tilbage, og **Se trafikken den dag på Angreb** åbner **Trafik den …** på [Angreb](vaernet.md). Ved **Et år** (og perioder over 90 dage) er der én søjle pr. uge, mandag til søndag, og et klik på en uge viser ugen. En dag uden tal står tom; den er ikke 0, den er bare ikke hentet.

Tabellen har kolonnerne **Domæne**, **Hvad skete der**, **Svar** og **Første og sidste time**. Hver række er én slags fejl på ét domæne, flest svar først, og fejlen står i klart sprog med svarkoden i parentes, fx "Cloudflare kunne ikke få forbindelse til din server i tide (svar 522)" eller "Din server svarede med en fejl (svar 502)". Et almindeligt fejlsvar, hvor ingen server svarede, står som "Ingen server svarede: dit website hos Cloudflare fejlede, eller Cloudflare selv svarede med en fejl (svar 500)". Andre koder står som "Den besøgende fik en fejl" med koden. Med **Alle domæner** åbner domænet Drift for det domæne. De 10 største står fremme, resten bag **Vis alle … rækker**.

Nederst i afsnittet står altid to noter. Den første: **Har du din egen server, er »dit website« den. Ligger dit website hos Cloudflare (Workers eller Pages), er der ingen server at nå: Cloudflare svarer selv, og derfor er »Kunne ikke nås« 0.** Den anden: **Programmet ser kun trafik, der går gennem Cloudflare. Svarer Cloudflare med en gammel kopi, ser den besøgende siden, selv om dit website er nede.**

### Subdomæner med fejl

Har ét domæne fejl på mindst to navne i perioden, fx `wi24rd.com` og `www.wi24rd.com`, står tabellen **Subdomæner med fejl** med kolonnerne **Subdomæne**, **Svar med fejl** og **Kunne ikke nås**. Den står kun, når ét domæne er valgt. Rækkerne **Kaldt med IP-adresse**, **Andet navn**, **Ugyldigt navn** og **Andre subdomæner** er ikke navne og kan ikke åbnes.

Klik på et subdomæne, og Drift viser kun dets fejl. Øverst står chippen **Subdomæne: www.wi24rd.com**. Pr. subdomæne findes kun **Kunne ikke nås** og fejlene pr. dag; tabellen viser **Første og sidste dag** i stedet for timer. De tre andre fliser viser **–** med **Findes ikke pr. subdomæne**, og en linje siger: **Timer med fejl, forsiden og websitets egne fejl findes ikke pr. subdomæne. Se dem for hele wi24rd.com.** **×** på chippen eller **hele wi24rd.com** fjerner filteret, og det nulstilles, når du vælger et andet domæne.

### Hvor meget Cloudflare klarede selv

Cloudflare gemmer kopier af dine filer og kan svare uden at spørge dit website. Afsnittet viser, hvor ofte det skete.

Fliserne:

| Flise | Betyder |
|---|---|
| **Fra Cloudflares lager** | Andelen af forespørgslerne, Cloudflare svarede på med en kopi fra sit lager. |
| **Data, dit website slap for at sende** | Mængden af data, der kom fra lageret, i læselige enheder (kB, MB, GB). |
| **Svaret af Cloudflare selv** | Forespørgsler, Cloudflare selv svarede på, fx en omdirigering. Svar fra et website, der kører som en Worker hos Cloudflare, står også her. |
| **Hentet hos dit website** | Forespørgsler, hvor svaret blev hentet hos din server. |

En flise viser sine rækker i tabellen. Under fliserne står andelene på én linje; de summer altid til 100 %.

Grafen viser forespørgsler pr. dag med to søjler side om side: **Fra Cloudflares lager** (grøn) og **Hentet hos dit website** (blå). Et klik på en dag viser dagen.

Tabellen **Hvad Cloudflare gjorde** viser, hvad der skete med forespørgslerne, med antal forespørgsler og data:

| Teksten | Betyder |
|---|---|
| **Fundet i Cloudflares lager** | Cloudflare havde en kopi og svarede med den. |
| **Ikke i lageret endnu, så svaret blev hentet hos dit website** | Filen kan gemmes, men lå der ikke endnu. |
| **Kopien i lageret var for gammel, så svaret blev hentet hos dit website** | Kopien var udløbet. |
| **En gammel kopi fra lageret, fordi dit website ikke svarede** | Cloudflare kunne ikke få en ny og viste den gamle. |
| **Svaret måtte ikke gemmes, så det blev hentet hos dit website** | Svaret selv sagde, at det ikke må gemmes. |
| **Dit website bekræftede, at kopien i lageret stadig gælder** | Cloudflare spurgte kort og brugte sin kopi. |
| **En gammel kopi fra lageret, mens Cloudflare henter en ny** | Den besøgende fik kopien med det samme. |
| **Sider, der hentes hos dit website hver gang** | Cloudflare gemmer dem ikke. Det gælder de fleste sider. |
| **Svaret af Cloudflare selv: fx en omdirigering, eller et website, der ligger hos Cloudflare (Workers)** | Cloudflare lavede selv svaret. Workers kører foran lageret, så deres svar står her. |
| **Andet** | En værdi, Cloudflare ikke har beskrevet. |

Har der været gamle kopier, fordi websitet ikke svarede, står linjen **Gamle kopier vist, fordi dit website ikke svarede: …**.

Under tabellen står tre noter. Den første: **Har du din egen server, er »dit website« den. Ligger dit website hos Cloudflare (Workers eller Pages), er der ingen server at hente noget hos: svar fra en Worker står under »Svaret af Cloudflare selv«.** Den anden: **Har du din egen server, hentes de fleste sider hos den hver gang. Det er Cloudflares standard: billeder, CSS og JavaScript gemmes, selve siderne ikke. Det betyder ikke, at noget er i stykker.** Den tredje: **Data er det, Cloudflare sendte til den besøgende. Det kan være mindre end det, dit website ville have sendt.**

Cachen gælder altid hele domænet. Med et subdomæne valgt står der kun **Cache gælder hele domænet.**, aldrig domænets tal under subdomænets navn.

## Hvad skal jeg gøre, når …

- **… der står en gul linje med et tidsrum?** I de timer kunne Cloudflare ikke nå din server. Spørg din udbyder, om der var en driftsforstyrrelse, og se "Hvad gør jeg?" på [Overblik](overblik.md), hvis domænet også står på Hold øje.
- **… Dit website svarede med en fejl har et tal?** Websitet var der, men et program fejlede: på din server eller, ligger websitet hos Cloudflare, i din Worker eller Pages-funktion. Vis rækkerne i tabellen, og giv din webmand tidspunkterne.
- **… der står "Dit website svarede ikke hver gang: forsiden svarede med en fejl …"?** Forsiden fejlede for nogle besøgende. Vis rækkerne med flisen **Forsiden svarede ikke**, og giv din webmand tidspunkterne.
- **… Kunne ikke nås er 0, selv om websitet har haft fejl?** Det er rigtigt for et website hos Cloudflare (Workers eller Pages): der er ingen server at nå. Fejlene står under **Dit website svarede med en fejl**.
- **… der står "Nogle sider svarede med fejl, men forsiden virkede."?** Det er ofte robotter, der rammer en adresse, som fejler. Dit website var ikke nede.
- **… næsten alt er Hentet hos dit website?** Det er normalt for små websites med egen server; se noten under tabellen. Programmet giver ingen råd, der kræver ændringer hos Cloudflare.
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
- **Dit website svarede med en fejl** er alle svar med fejl, der ikke er Kunne ikke nås: svar 500–599 fra din egen server, og fejl fra et website hos Cloudflare (Workers eller Pages), hvor ingen server svarede. Hvert svar tælles én gang.
- **En time med fejl** har mindst 5 svar, der ikke kunne nås, og de er mindst 20 % af timens svar. Tre eller flere timer i træk giver linjen med tidsrummet.
- Tallene er Cloudflares estimat ud fra en stikprøve, og kun besøgendes forespørgsler tæller med. Det, Cloudflare selv blokerede, er ikke fejl.
- Programmet ser kun trafik, der går gennem Cloudflare. Et subdomæne, der er «DNS only», og besøg direkte på serverens IP-adresse ses ikke. Viser Cloudflare en gammel kopi, mens serveren er nede, ser den besøgende siden, og det tæller ikke som en fejl.
- Fejl pr. time og cache findes ca. 30 dage tilbage hos Cloudflare. Programmet gemmer dagene, så historikken vokser derfra. Fejl pr. subdomæne findes fra ca. 30 dage før opdateringen.
- Et tal, der ikke er hentet, står som **–** eller "Ikke hentet", aldrig som 0.
- Dagene er hele døgn i UTC; klokkeslættene vises i din lokale tid.
- **Alle domæner** lægger domænerne sammen. Et samlet tal står kun for de dage, hvor alle valgte domæner er hentet.
- Drift giver ingen notifikationer og ændrer ikke status. Kun **Din server svarer ikke** i [Overblik](overblik.md) kan give Hold øje (aldrig Handl), og den regel er den samme som før, nu med fejl 530. Dit websites egne fejl, også en forside, der fejler på et website uden egen server, og "nogle sider svarede med fejl" ændrer aldrig status og giver aldrig en besked.
- Der kræves ingen ny rettighed på dit API Token.
