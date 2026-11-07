# Ordliste

**Afvist** · Alt, Cloudflare stoppede: blokerede, udfordrede eller lokkede ind i labyrinten, også rutine. Status tæller kun egentlige angreb.

**Angreb** · Forespørgsler, Cloudflares regler stoppede, fordi de lignede et angreb: kendte huller, dine egne regler, DDoS. Rutine (bot-tjek) tæller ikke med.

**API Token** · En nøgle, du opretter i din Cloudflare-konto, og som giver et program lov til bestemte ting. Programmets API Token kan kun læse: `Zone › Analytics › Read` og `Zone › Zone › Read`.

**Blokeret** · Cloudflare afviste forespørgslen uden videre.

**Bot Fight Mode** · Cloudflares automatiske beskyttelse mod robotter. Rutine; giver aldrig Hold øje eller Handl.

**Browsertjek (udfordret)** · Cloudflare bad den besøgende bevise, at den er en rigtig browser. En robot klarer det ikke.

**CVE** · Et nummer for et kendt sikkerhedshul, fx CVE-2025-55182. Et klik åbner hullets side på cve.org.

**Datacenter** · Et af Cloudflares mange steder i verden, hvor dit site svarer fra. Angreb stoppes i det datacenter, der ligger nærmest angriberen.

**DMARC** · En regel for, hvem der må sende mails i dit domænes navn. Mails, der fejler DMARC, kan være nogen, der udgiver sig for dig, eller en tjeneste, du selv bruger, men ikke har sat op.

**DNS-rekognoscering** · Nogen spørger efter mange navne under dit domæne, der ikke findes, for at finde servere og tjenester.

**Dyk** · Det gamle navn for filtrene i Angreb: de gemte tal for netop det, du klikkede på.

**Edge – første forsvarslinje** · Cloudflares netværk af datacentre. Dit site svarer fra dem alle (anycast), så et angreb stoppes nær angriberen, længe før det når din server. Gælder kun DNS-poster med orange sky.

**Handl** · Status, når der var usædvanligt mange angreb i går i forhold til ugen før, eller noget slap igennem. Giver besked som standard.

**Hold øje** · Status, når mønsteret er usædvanligt, men alt er stoppet.

**Hul** · En dag, programmet aldrig har hentet, typisk fordi computeren var slukket. Tegnes aldrig som 0.

**Huginn** · Tanke. Det, der sker nu: angreb, robotter, login.

**Labyrinten** · Cloudflare viser robotten falske sider, der spilder dens tid. Stopper ikke robotten og regnes ikke som en blokering.

**Ligner målrettet** · Mærke, når angrebene steg hos dig, men ikke på dine andre domæner, og en stor del kom fra ét netværk. Kræver mindst 3 valgte domæner og afgøres først, når døgnet efter er hentet.

**Ligner rekognoscering** · Mærke, når én dag har mange forskellige stier, som når en scanner leder efter, hvad dit site kører.

**Muninn** · Hukommelse. Det, der er sket: historikken på din computer.

**Netværk** · Den udbyder (et autonomt system), en forespørgsel kom fra. Programmet ser netværket, aldrig IP-adressen.

**Nøglering** · Det sted, dit styresystem gemmer adgangskoder sikkert: macOS Nøglering eller Windows Credential Manager. Her ligger API Tokenet.

**Radar** · Cloudflare Radar, Cloudflares offentlige tal for hele internettet. Bruges i dit lands visning. Kræver ingen rettighed.

**Roligt** · Status, når intet er usædvanligt.

**Rutine** · Cloudflares automatiske bot-tjek: labyrinten, Bot Fight Mode og browsertjekket. Tæller som afvist, men aldrig som angreb.

**Sampling (stikprøver)** · Over lange perioder tæller Cloudflare ikke alt, men regner fra en stikprøve. Tallene er derfor omtrentlige. Programmet henter én dag ad gangen for at mindske det.

**Snusere** · Forespørgsler efter filer, der ikke findes: scannere, der leder efter kendte huller.

**Sti** · Delen af en adresse efter domænet, fx `/wp-login.php`. Gemmes uden forespørgselsdel.

**Udklædt** · En robot, der kalder sig fx Googlebot uden at være det.

**Ukendt** · Status, når der ingen tal er endnu.

**UTC** · Verdenstid. Alle dage i programmet er hele døgn i UTC, så "i går" slutter kl. 01 eller 02 dansk tid.

**Zone** · Cloudflares ord for et domæne på din konto.
