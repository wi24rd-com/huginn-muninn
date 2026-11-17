# Ordliste

**Afvist** · Alt, Cloudflare stoppede: blokerede, udfordrede eller lokkede ind i labyrinten, også rutine. Status tæller kun egentlige angreb.

**Angreb** · Forespørgsler, Cloudflares regler stoppede, fordi de lignede et angreb: kendte huller, dine egne regler, DDoS. Rutine (bot-tjek) tæller ikke med.

**API Token** · En nøgle, du opretter i din Cloudflare-konto, og som giver et program lov til bestemte ting. Programmets API Token kan kun læse: **Zone › Read** og **Analytics › Read**.

**Besøg** · Cloudflares egen definition: en sidevisning, der kom fra et andet site eller et direkte link. Det samme tal som "Visits" i Cloudflares dashboard, og Cloudflares estimat ud fra en stikprøve. Står på Besøgende pr. land.

**Blokeret** · Cloudflare afviste forespørgslen uden videre.

**Bot Fight Mode** · Cloudflares automatiske beskyttelse mod robotter. Rutine; giver aldrig Hold øje eller Handl.

**Browsertjek (udfordret)** · Cloudflare bad den besøgende bevise, at den er en rigtig browser. En robot klarer det ikke.

**Cache (Cloudflares lager)** · Cloudflare gemmer kopier af dine filer (billeder, CSS, JavaScript) og kan svare med dem uden at spørge dit website. Selve siderne gemmes som standard ikke. [Drift](40-visninger/drift.md) viser, hvor ofte svaret kom fra lageret. Cachen gælder hele domænet.

**CVE** · Et nummer for et kendt sikkerhedshul, fx CVE-2025-55182. Et klik åbner hullets side på cve.org.

**Datacenter** · Et af Cloudflares mange steder i verden, hvor dit site svarer fra. Angreb stoppes i det datacenter, der ligger nærmest angriberen.

**DMARC** · En regel for, hvem der må sende mails i dit domænes navn. Mails, der fejler DMARC, kan være nogen, der udgiver sig for dig, eller en tjeneste, du selv bruger, men ikke har sat op.

**DNS-rekognoscering** · Nogen spørger efter mange navne under dit domæne, der ikke findes, for at finde servere og tjenester.

**Dyk** · Det gamle navn for filtrene i Angreb: de gemte tal for netop det, du klikkede på.

**Edge – første forsvarslinje** · Cloudflares netværk af datacentre. Dit site svarer fra dem alle (anycast), så et angreb stoppes nær angriberen, længe før det når din server. Gælder kun DNS-poster med orange sky.

**Fejl 520–527 og 530** · Cloudflares egne svarkoder, når Cloudflare ikke kunne nå eller finde din server; programmet kalder dem "Kunne ikke nås". 520: serveren svarede, men svaret kunne ikke bruges. 521: serveren afviste Cloudflare. 522: Cloudflare kunne ikke få forbindelse i tide. 523: Cloudflare kunne ikke finde vej til serveren. 524: serveren var for længe om at svare. 525: den sikre forbindelse til serveren kunne ikke oprettes. 526: serverens certifikat er ikke gyldigt. 527 står ikke på Cloudflares side over fejlene, men tæller med. 530: Cloudflare kunne ikke slå serverens navn op. Er de en stor del af svarene på en dag, står der **Din server svarer ikke** i Overblik. De kræver en server: ligger dit website hos Cloudflare (Workers eller Pages), er der ingen at nå, og tallet er 0. Svar 500–599 fra serveren selv er noget andet: serveren var oppe, men et program på den fejlede. Dem, og fejl fra et website hos Cloudflare, tæller [Drift](40-visninger/drift.md) under **Dit website svarede med en fejl**.

**Forbindelse** · Ét API Token og et navn i programmet, én pr. Cloudflare-konto. Navnet er kontoens navn hos Cloudflare, indtil du giver forbindelsen dit eget med **Omdøb…**. Domænerne fra alle forbindelser vises samlet.

**Handl** · Status, når der var usædvanligt mange angreb i går i forhold til ugen før, eller noget slap igennem. Giver besked som standard.

**Hold øje** · Status, når mønsteret er usædvanligt, men alt er stoppet.

**Hul** · En dag, programmet aldrig har hentet, typisk fordi computeren var slukket. Tegnes aldrig som 0.

**Huginn** · Tanke. Det, der sker nu: angreb, robotter, login.

**IP-klasse** · Cloudflares grov inddeling af de IP-adresser, trafikken kom fra, fx søgemaskine, overvågning, scanner, Tor eller kendt dårlig adresse. Programmet gemmer kun antallet af forespørgsler pr. klasse og dag, aldrig adresserne.

**Kendte robotter** · Robotter, Cloudflare har verificeret, fx Googlebot. Robotter, der udgiver sig for at være en almindelig browser, er ikke med.

**Labyrinten** · Cloudflare viser robotten falske sider, der spilder dens tid. Stopper ikke robotten og regnes ikke som en blokering.

**Ligner målrettet** · Mærke, når angrebene steg hos dig, men ikke på dine andre domæner, og en stor del kom fra ét netværk. Kræver mindst 3 valgte domæner og afgøres først, når døgnet efter er hentet.

**Ligner rekognoscering** · Mærke, når én dag har mange forskellige stier, som når en scanner leder efter, hvad dit site kører.

**Muninn** · Hukommelse. Det, der er sket: historikken på din computer.

**Netværk** · Den udbyder (et autonomt system), en forespørgsel kom fra. Programmet ser netværket, aldrig IP-adressen.

**Nøglering** · Det sted, dit styresystem gemmer adgangskoder sikkert: macOS Nøglering eller Windows Credential Manager. Her ligger API Tokenet, ét pr. forbindelse.

**Radar** · Cloudflare Radar, Cloudflares offentlige tal for hele internettet. Bruges i dit lands visning. Kræver ingen rettighed.

**Roligt** · Status, når intet er usædvanligt.

**Rutine** · Cloudflares automatiske bot-tjek: labyrinten, Bot Fight Mode og browsertjekket. Tæller som afvist, men aldrig som angreb.

**Sampling (stikprøver)** · Cloudflare gemmer ikke hver hændelse, men en stikprøve, hvor hver hændelse oplyser, hvor mange den står for. Programmet lægger det sammen, så antallet af afviste er Cloudflares estimat, ikke et antal rækker. Det kan derfor være højere end antallet af enkelte hændelser. Programmet henter én dag ad gangen, så stikprøven dækker så kort tid som muligt.

**Snusere** · Forespørgsler efter filer, der ikke findes: scannere, der leder efter kendte huller.

**Sti** · Delen af en adresse efter domænet, fx `/wp-login.php`. Gemmes uden forespørgselsdel.

**Subdomæne** · Et navn under dit domæne, fx `www.wi24rd.com` eller `shop.wi24rd.com` under `wi24rd.com`. Domænet selv (uden noget foran) kaldes også *apex*. Programmet viser trafik og angreb pr. subdomæne i [Besøgende](40-visninger/besoegende.md) og [Angreb](40-visninger/vaernet.md), når trafikken går gennem Cloudflare (orange sky), ikke når navnet er «DNS only».

**Udklædt** · En robot, der kalder sig fx Googlebot uden at være det.

**Ukendt** · Status, når der ingen tal er endnu.

**Unikke besøgende** · Antal forskellige IP-adresser, der besøgte sitet en dag. Robotter tæller med, og flere personer bag samme router tæller som én. En sum over flere dage tæller den samme besøgende én gang pr. dag.

**UTC** · Verdenstid. Alle dage i programmet er hele døgn i UTC, så "i går" slutter kl. 01 eller 02 dansk tid.

**Zone** · Cloudflares ord for et domæne på din konto.
