/* Huginn and Muninn — sprog. Dansk først, engelsk med samme nøgler. Indstillingen huskes i webvisningens localStorage
   ('hm-sprog' = auto | da | en); 'auto' følger systemsproget. Scriptet sætter <html lang> og udfylder alle [data-t]-elementer,
   før de andre scripts kører (alle er defer, i den rækkefølge). Programmet er én side, så alle visningers tekster
   er ét sæt. Angreb-visningen (Værnet) kalder saetDomaene('3 domæner', true), når den ved, hvad den viser: så udfyldes
   teksterne igen med {domaene}, med flertalsvarianten (nøgle + '__fl'), hvor der er en. Vinduets titel sætter rammen (titel()).
   Det afgjorte sprog (da/en) sendes til Rust (sprog_saet) ved start og ved skift, så menulinjen og programmenuen følger med.
   Ved skift genindlæses siden; adressens #visning bevares. saetLand('Danmark') giver teksterne {land} (visningens navn følger landet). Ordlisten (klar) oversætter regelnavne fra Cloudflare til
   klarsprog i Overblikkets "Hvorfor". Noter, der skifter tekst (fx på macOS), får data-t sat af visningen, så anvend() ikke
   skriver dem tilbage. */
(function () {
  'use strict';
  let valg = 'auto';
  try { valg = localStorage.getItem('hm-sprog') || 'auto'; } catch (e) { }
  const system = (navigator.languages && navigator.languages[0]) || navigator.language || 'da';
  const afgjort = v => (v === 'da' || v === 'en' ? v : (/^da\b/i.test(system) ? 'da' : 'en'));
  const lang = afgjort(valg);
  const tilRust = l => {
    const invoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    if (invoke) invoke('sprog_saet', { sprog: l }).catch(() => { });
  };
  tilRust(lang);
  /* De pæne navne på kataloget: samme som `KATALOG` i src-tauri/src/teknologi.rs. En teknologi uden for kataloget får stort begyndelsesbogstav i hvert ord (tekNavn, som `teknologi::navn`). */
  const TEK = { wordpress: 'WordPress', joomla: 'Joomla', drupal: 'Drupal', magento: 'Magento', ghost: 'Ghost', plone: 'Plone', typo3: 'TYPO3', prestashop: 'PrestaShop', nextjs: 'Next.js', react: 'React', laravel: 'Laravel', symfony: 'Symfony', django: 'Django', rails: 'Ruby on Rails', spring: 'Spring', nodejs: 'Node.js', log4j: 'Java/Log4j', php: 'PHP', tomcat: 'Apache Tomcat', 'apache camel': 'Apache Camel', nginx: 'nginx', mongodb: 'MongoDB', redis: 'Redis', oracle: 'Oracle', exchange: 'Microsoft Exchange', sharepoint: 'Microsoft SharePoint', confluence: 'Atlassian Confluence', jira: 'Atlassian Jira', zimbra: 'Zimbra', roundcube: 'Roundcube', fortinet: 'Fortinet', ivanti: 'Ivanti', citrix: 'Citrix', cisco: 'Cisco', 'palo alto': 'Palo Alto Networks', sonicwall: 'SonicWall', f5: 'F5 BIG-IP', gitlab: 'GitLab', jenkins: 'Jenkins', grafana: 'Grafana', solarwinds: 'SolarWinds', splunk: 'Splunk' };
  let domaene = '', landNavn = '';
  /* Datoformatet (Indstillinger → Generelt): sprog (standard), system eller iso. Huskes som sproget, og Rust får det til notifikationerne. */
  let datoValg = 'sprog';
  try { datoValg = localStorage.getItem('hm-dato') || 'sprog'; } catch (e) { }

  /* Ental eller flertal efter et tal; tallet kan være færdig tekst (tal() giver "1"). */
  const flertal = (n, en, flere) => (String(n) === '1' ? en : flere);
  /* {n} er værdien; {n|side|sider} vælger ordet efter samme værdi, og ordene må selv indeholde {n}. */
  const fmt = (s, v) => String(s).replace(/\{(\w+)\|((?:[^|{}]|\{\w+\})*)\|((?:[^|{}]|\{\w+\})*)\}/g, (m, k, en, flere) => (k in v ? flertal(v[k], en, flere) : m)).replace(/\{(\w+)\}/g, (m, k) => (k in v ? v[k] : m));
  const TEKST = {
    da: {
      skip: 'Gå til indhold', under: 'for Cloudflare®', sprog: 'Sprog', sprogAuto: 'Systemets sprog', appNavn: 'Wi24rd Huginn and Muninn',
      titel: { teknologi: 'Teknologi pr. domæne', overblik: 'Overblik', 'siden-sidst': 'Siden sidst', vaernet: 'Angreb · {domaene}', robotter: 'Robotter · {domaene}', login: 'Login · {domaene}', land: '{land}', besoegende: 'Besøgende · {domaene}', indstillinger: 'Indstillinger', opsaetning: 'Forbind med Cloudflare' },
      navH: 'Visninger', navUdenToken: 'Kræver et API Token.', traeH: 'Domæner', traeAria: 'Vælg domæne', traeAlle: 'Alle domæner', traeTom: 'Ingen domæner er valgt endnu.', baandAria: 'Kommandoer', gData: 'Data', gLand: 'Land', gHjaelp: 'Hjælp', gProgram: 'Program', baandMere: 'Mere', gVis: 'Vis', gDomaener: 'Domæner', gPeriode: 'Periode', gDatoer: 'Egne datoer', gFiltre: 'Filtre', gTeknologi: 'Teknologi', gStier: 'Stier', baandVaelgDom: 'Vælg domæner', baandRetTek: 'Ret teknologier', baandRobotAngreb: 'Angreb fra robotter', baandNulstil: 'Nulstil', baandNulstilIntet: 'Der er ingen filtre at nulstille.', vejledningKort: 'Vejledning', landSoeg: 'Søg efter land', landSoegPh: 'Søg', landIngen: 'Intet land passer.', fodLokalt: 'Kun på denne maskine', navOverblik: 'Overblik', navSiden: 'Siden sidst', navVaernet: 'Angreb', navIndst: 'Indstillinger',
      navRobotter: 'Robotter', navLogin: 'Login', navLand: 'Land lige nu',
      maalMin: 'Huginn kan kun sige »Ligner målrettet«, når du har mindst 3 domæner valgt at sammenligne med.', moensterForeloebig: 'Foreløbig — afgøres, når dagen efter er hentet.', maerkeForeloebig: 'foreløbig',
      vurdSpoerg: 'Hvorfor fik dagen (ikke) et mærke?', vurdIntro: 'Hver betingelse med det tal, mærket afgøres af, for {d}.', vurdJa: 'Opfyldt', vurdNej: 'Ikke opfyldt', vurdIkke: 'Tæller ikke med', vurdStatusJa: '{m}: ja', vurdStatusForeloebig: '{m}: ja, foreløbigt — afgøres, når dagen efter er hentet', vurdStatusNej: '{m}: nej',
      vurd: { dns_nx: 'Opslag på navne, der ikke findes: {n} (skal være mindst {m})', dns_navne: 'Forskellige navne, der ikke findes: {n} (skal være mindst {m})', dns_navneEks: 'Forskellige navne, der ikke findes: {n} (skal være mindst {m}), fx {navn}', stigning: 'Stigning: {n} angreb mod snit {m}', netvaerk: 'Ét netværk står for mindst {m} %: {navn} {n} %', netvaerkIngen: 'Ét netværk står for mindst {m} %: ingen', net_andre: 'Netværket næsten kun her: {n} på andre valgte domæner ±1 dag mod {m} her', gule: 'Højst {m} {m|andet domæne|andre domæner} med en stigning ±1 dag: {n}', valgte: 'Mindst {m} valgte domæner med tal: {n}', radar: 'Verden steg ikke: {n} (skal være under {m})', radarIngen: 'Verden steg ikke: Radar har ikke tal for dagen', efter: 'Dagen efter er hentet færdig', efterNej: 'Dagen efter er ikke hentet færdig', stier: 'Mindst {m} forskellige stier med 404: {n}', faktor: 'Mindst 3 gange det normale: {n} (grænsen er {m})', normalNul: 'Det normale: 0 (ingen sammenligning)', dns_nxUden: 'Opslag på navne, der ikke findes: {n}. Det normale: for få dage (ingen sammenligning)', dns_navneUden: 'Forskellige navne, der ikke findes: {n}. Det normale: for få dage (ingen sammenligning)' }, vurdAabnDag: 'Åbn dagens angreb', vurdAabnNet: 'Åbn angrebene fra {navn}', vurdAabnDomaene: 'Åbn {navn}', vurdAabnValgte: 'Åbn dine valgte domæner', vurdAabnVerden: 'Åbn Verden', vurdAabnStier: 'Åbn stierne',
      hvDmarcNy: 'Mails i dit domænes navn fejlede DMARC fra en ny afsender: {l}.', hvDmarcAndel: '{p} % af mails i dit domænes navn fejlede DMARC (normalt {m} %).', hvIGaarOrd: 'i går', hvForgaarsOrd: 'i forgårs', hvLaek: '{n} login-forsøg brugte et kodeord fra et kendt læk {d}.', hvLaekNaaet: '{n} login-forsøg brugte et kodeord fra et kendt læk {d}, og {k} af dem nåede din server og fik et svar, der kan ligne et login.',
      hvTilstand: { pause: 'Domænet er sat på pause — Cloudflare beskytter det ikke.', inaktiv: 'Domænet er ikke aktivt hos Cloudflare (status: {s}).', phishing: 'Cloudflare mistænker phishing på domænet.', udvikler: 'Udviklertilstand er slået til — cachen er slået fra, til kl. {k}.' }, gTilstandH: { pause: 'Hvad gør jeg, når domænet er sat på pause?', inaktiv: 'Hvad gør jeg, når domænet ikke er aktivt?', phishing: 'Hvad gør jeg ved mistanke om phishing?', udvikler: 'Hvad gør jeg ved udviklertilstand?' }, gLaekH: 'Hvad gør jeg ved lækkede kodeord?', hvSti200: 'En følsom fil ({s}) svarede 200 {d} – den kan ligge åbent.', hvSti200Flere: 'Følsomme filer ({s}) svarede 200 {d} – de kan ligge åbent.', hvCfFund: 'Cloudflare har et kritisk sikkerhedsfund for domænet.', hvCfFundFlere: 'Cloudflare har {n} kritiske sikkerhedsfund for domænet.', gSti200H: 'Hvad gør jeg ved en følsom fil, der svarede 200?', gCfFundH: 'Hvad er Cloudflares egne fund?',
      dykKunTrafik: 'Cloudflare gemmer sikkerhedshændelser i en måned — før {d} findes kun trafiktal.', dykTrafikDage: 'Forespørgsler dag for dag', dykTrafikDageP: 'Fra Cloudflares dagstal. Hver dag åbner dagens trafik.', dykTrafikSoejle: 'Forespørgsler pr. dag', dykTrafikHul: '{d}: ingen tal fra Cloudflare', trafikKunAar: 'Dagen er ældre end sikkerhedshændelserne: tallene er Cloudflares dagstal, og »trusler« er Cloudflares egen optælling.', trafikTrusler: 'Trusler (Cloudflares dagstal)', trafikCachet: 'Fra Cloudflares cache', dataKilder: 'Hvad der findes', dataKilderTekst: 'Trafik fra {a} · sikkerhedshændelser fra {b}',
      loginLaekH: 'Kodeord fra kendte læk', loginLaekP: 'Cloudflare sammenligner kodeordet i login-forsøg med kendte læk. Et match siger kun, at kodeordet findes i et læk, ikke at det var en rigtig bruger, eller at nogen kom ind. Hverken Cloudflare eller Huginn gemmer kodeordet.', loginLaekIngen: 'Kodeord fra kendte læk: ingen forsøg i perioden.', loginLaekVenter: 'Tjekket er ikke hentet for perioden endnu.', loginLaekLinje: '{n} login-forsøg brugte et kodeord fra et kendt læk', loginLaekThDag: 'Dag', loginLaekThSti: 'Sti', loginLaekThN: 'Forsøg', loginLaekThNaaet: 'Nåede serveren', loginLaekAndet: 'Andre svar fra tjekket: {l}.', laekVaerdi: { username_leaked: 'kun brugernavnet stod i et læk', username_password_similar: 'et lignende brugernavn og kodeord', ukendt: 'ukendt' }, loginLaekNone: 'Ikke et login-forsøg (intet kodeord sendt): {n}', loginLaekUdenSvar: 'Uden svar fra tjekket: {n}.', loginLaekAabn: 'Vis kun {sti}', loginLaekDag: 'Vis kun {d}',
      hentHenter: 'Henter …', hentFaerdig: 'Hentet kl. {t} — {n} opdateret', hentAlt: 'Alt er hentet — intet nyt siden kl. {t}', hentAltUden: 'Alt er hentet.', hentAllerede: 'Henter allerede ({a} af {b}) …', hentFejl: 'Kunne ikke hente: {f}',
      loginLaekSlaaetTil: 'Tjekket er slået til som standard på Free-planen. På en betalt plan skal det være slået til i Cloudflare under Security → Settings, ellers står her intet.',
      nulstilSikkerhed: 'Du har sikkerhedstal fra {fra}. Efter nulstilling kan kun tal fra {g} hentes igen. {n} {n|dag|dage} forsvinder for altid.', nulstilTrafik: 'Du har trafiktal fra {fra}. Efter nulstilling kan kun tal fra {g} hentes igen. {n} {n|dag|dage} forsvinder for altid.', nulstilUkendt: 'Det, der er ældre end det, Cloudflare stadig har, forsvinder for altid.',
      dataTrafikHentes: 'Trafiktal hentes … · sikkerhedshændelser fra {b}', dataKvoteH: 'Kvote hos Cloudflare', dataKvote: 'Cloudflare tillader {n} kald pr. {m} {m|minut|minutter}; Huginn brugte {k} i sidste runde.',
      tekRet: 'Ret teknologier', tekRetAabn: 'Ret teknologierne for {navn}', tekIngenValg: 'Du har ikke sagt, hvad sitet bruger.', tekChipBruger: 'Bruger: {l}', tekChipIkke: 'Bruger ikke: {l}', tekForslagKort: 'Alle {n} {t}-forsøg fik 404 —', tekAndet: 'Andet',
      emailH: 'Mails sendt i dit domænes navn', emailP: '{p}, fra de rapporter, modtagerne sender til Cloudflare. Rapporterne kommer en dag eller mere bagud.', emailLinje: '{n} {n|mail|mails} · {m} ikke godkendt fra {k} {k|afsender|afsendere}', emailLinjeOk: '{n} {n|mail|mails} · alle godkendt', emailAfsender: 'Afsender', emailLand: 'Land', emailFejlet: 'Fejlede DMARC', emailMails: 'Mails i alt', emailUkendt: 'Ukendt afsender', emailNote: 'Regnet ud fra DKIM og SPF: en mail består DMARC, hvis en af dem består. Listen viser kun afsendere, der fejlede. En afsender, du selv bruger, kan fejle, fordi den ikke er sat op endnu.', emailTom: 'Cloudflare har ingen DMARC-rapporter for domænet — DMARC Management er måske ikke slået til.', emailVenter: 'DMARC-tallene er ikke hentet endnu.', emailFejl: 'Cloudflare svarede ikke på DMARC-tallene: {f}', emailGuide: 'Sådan slår du DMARC Management til', sti200H: 'Følsomme filer, der svarede 200', sti200P: '{p}: {n} {n|sti|stier}, der aldrig bør svare 200, fx /.env og /.git/config. 200 betyder ikke altid, at filen findes: mange sites svarer med deres almindelige side på alle adresser.', sti200Ingen: 'Ingen af stierne på listen ({n}) svarede 200 i perioden.', sti200Venter: 'Tallene er ikke hentet for perioden endnu.', sti200Fil: 'Ligner filen', sti200Side: 'Ligner sidens almindelige side', sti200ThSti: 'Sti', sti200ThN: 'Svar 200', sti200ThSnit: 'Snit pr. svar', sti200ThVurd: 'Vurdering', sti200Aabn: 'Vis angreb på {sti}', sti200Note: 'Et svar ligner filen, når det er højst 8 KB, mere end 25 % fra forsidens størrelse ({f}), og domænet ikke svarer 200 på mere end 6 af stierne. Kun da står domænet på Handl. Tallene findes kun for dage, der er hentet efter opdateringen 8. november 2026.', sti200UdenForside: 'ukendt', sti200Guide: 'Hvad gør jeg?', cfH: 'Cloudflares egne fund', cfP: 'Cloudflare skanner selv domænets opsætning (Security Insights: hver 7. dag på Free, hver 3. på Pro og Business). Et klik åbner fundene hos Cloudflare.', cfIngen: 'Cloudflare har ingen aktive fund for domænet.', cfVenter: 'Fundene er ikke hentet endnu.', cfManglerRet: 'Kræver én rettighed mere på API Tokenet, som kontorettighed: Account Security Center Insights › Read i en ny politik (Add new policy › Add new Account policy). Zone Security Center Insights virker ikke. Den er valgfri: resten af programmet virker uden.', cfFejl: 'Cloudflare svarede ikke på fundene ({f}). De senest hentede står.', cfAlvor: { critical: 'Kritisk', high: 'Høj', moderate: 'Middel', low: 'Lav' }, cfKlasse: { security_txt_not_enabled: 'security.txt er ikke sat op', always_use_https_not_enabled: 'Always Use HTTPS er ikke slået til' }, cfAabn: 'Åbn fundet hos Cloudflare: {t}', cfSiden: 'siden {d}', cfHentet: 'Hentet {t}.', rettetH: 'Hvad angrebene var rettet mod', rettetP: 'Alle teknologier, der blev angrebet i perioden. Dine valg ændrer kun status og advarsler, aldrig denne liste.', rettetUfarlig: 'ufarligt for dig', rettetBlandet: 'Forskelligt pr. domæne', rettetAabn: '{n} angreb på {t}: åbn dem',
      tekKolMenu: 'Handlinger for {t}', tekRkMenu: 'Handlinger for {navn}', tekSideH: 'Teknologi pr. domæne', tekFanePr: 'Pr. domæne', tekFaneAlle: 'Alle teknologier', tekFaner: 'Visning', tekSoeg: 'Søg efter teknologi', tekForkl: '✓ Bruger · ✕ Bruger ikke · ? Ved ikke. Et lysere felt er ikke set på domænet, men kan stadig sættes. Tal åbner angrebene.', tekKortBruger: 'Bruger', tekKortIkke: 'Bruger ikke', tekIkkeAlle: 'Bruger ikke på alle', tekIkkeAlleFor: '{t}: Bruger ikke på alle domæner', tekNulKolonne: 'Nulstil kolonnen', tekNulKolonneFor: 'Nulstil {t} på alle domæner', tekNulDomaene: 'Nulstil domænet', tekNulDomaeneFor: 'Nulstil alle valg for {navn}', tekKopier: 'Brug på alle domæner', tekKopierFor: 'Brug valgene for {navn} på alle domæner', tekCelleAntal: '{n} angreb på {t} på {navn}: åbn dem', tekIngenKolonner: 'Ingen teknologier vises i tabellen. Vælg dem under Alle teknologier.', tekIngenTraef: 'Ingen teknologi passer til søgningen.', tekNy: 'Ny teknologi set: {t} — vis i tabellen?', tekNyJa: 'Ja', tekNyNej: 'Nej', tekNyJaFor: 'Vis {t} i tabellen', tekNyNejFor: 'Vis ikke {t} i tabellen', tekVis: 'Vis i tabellen', tekVisFor: 'Vis {t} i tabellen', tekKontoAntal: '{n} angreb på kontoen', tekIkkeSetKonto: 'Ikke set', tekTilfoej: 'Tilføj teknologi', tekTilfoejKnap: 'Tilføj', tekTilfoejNote: 'Vælg et navn fra listen eller skriv selv, så du kan sige »Bruger ikke«, før første angreb.', tekTilfoejet: '{t} er tilføjet.', tekTilfoejFejl: 'Navnet kan ikke bruges. Det må højst være 40 tegn og må ikke være en angrebsform.', tekGruppeNote: 'Skjuler du en teknologi her, gælder domænernes valg stadig, og angrebene står stadig i tal, lister og på kortet.', tekIndeGruppe: 'Piletasterne vælger nu i feltet. Esc går tilbage til tabellen.', tekGemt: 'Gemt.', tekGruppe: { cms: 'CMS', ramme: 'Webrammeværk', server: 'Server og sprog', samarbejde: 'E-mail og samarbejde', netvaerk: 'Netværk og VPN', andet: 'Andet' },
      domTekAabn: 'Teknologi pr. domæne →', domTekUden: '{n} uden valg', domTekValg: '{navn}: {n} {n|valg|valg}',
      hvMere: 'mere', hvMindre: 'mindre', hvIkkeBrugtOrd: 'som du ikke bruger', hvIkkeBrugtAabn: 'Se og fortryd teknologi-valgene for {navn}',
      sidenUtc: 'Døgn i UTC, vist i din tid.', sidenVistNat: 'Status for i forgårs. Du kiggede her sidst {t}.', sidenFoersteNat: 'Status for i forgårs på alle dine domæner.',
      dykVisNafM: 'Viser {n} af {m} angreb.', dykLoft200: 'Huginn gemmer de 200 seneste angreb pr. dag.', dykForrige: '‹ Forrige', dykNaeste: 'Næste ›', dykSideN: 'Side {a} af {b}', dykSiderH: 'Sider', dykSideAabn: 'Vis side {a} af {b} af de seneste',
      
      domTekH: 'Teknologi pr. domæne', domTekAlle: 'Nulstil for alle domæner', 
      bekraeftTekH: 'Nulstil teknologi-valg?', bekraeftTekP: 'Alle valg for {d} sættes til »Ved ikke«. Status regnes igen, som før du valgte.', bekraeftTekAlleP: 'Alle valg på alle domæner sættes til »Ved ikke«. Status regnes igen, som før du valgte.', bekraeftJa: 'Nulstil', tekNulstillet: 'Teknologi-valgene er nulstillet.',
      datoFormat: 'Datoformat', datoFormatNote: 'Gælder alle datoer i programmet og notifikationerne.', datoFormatSprog: 'Følg sproget', datoFormatSystem: 'Følg systemet', datoFormatIso: 'ISO (2026-11-04)',
      kalForrige: 'Forrige måned', kalNaeste: 'Næste måned', kalAabn: '{felt}: {d}. Vælg en anden dato', kalAabnTom: '{felt}: vælg en dato',
      dykKunIDag: '{navn} er kun set i dag ({n} {o} indtil nu). Hele døgn tælles fra i morgen.', dykKunIDagHer: 'Det her', dykKunStier: 'Stier', dykKunLande: 'Lande', dykKunTid: 'Tidspunkt (UTC, hel time)', dykKunAabn: 'Se dagens tal',
      dykHeleIDag: '{h} hele døgn · {n} i dag', dykForesp: 'Forespørgsler i alt', dykForespSub: '{o}: {n} — 1 af hver {m}', dykForespAabn: 'Forespørgsler i alt: {n}. Åbn trafikken',
      dykTrafikLinje: '{t} {p}: {n} {n|forespørgsel|forespørgsler} (normalt {m} om dagen). {stop}', dykSeTrafikken: 'Se trafikken →',
      autostartBanner: 'Programmet starter ikke ved login.', autostartSlaaTil: 'Slå Start ved login til', autostartHulKort: 'Slået fra: der kan komme huller i historikken.',
      maerkeRekog: 'Ligner rekognoscering', maerkeRekogKort: 'Recon', maerkeDnsKort: 'DNS-recon', maerkeDns: 'Ligner DNS-rekognoscering', maerkeMaal: 'Ligner målrettet', maerkeAabn: '{m} på {navn} den {d}: åbn dagen', maerkeGuide: 'Hvad betyder det? →',
      moensterRekog: 'Ligner rekognoscering: {n} forskellige stier på én dag (normalt {m}){s}{t}.', moensterDns: 'Ligner DNS-rekognoscering: {n} opslag på navne, der ikke findes, på én dag (normalt {m}).', moensterDnsNavne: 'Ligner DNS-rekognoscering: {n} {n|navn|forskellige navne}, der ikke findes, på én dag (normalt {m}).', moensterScanner: ', som en almindelig scanner',
      moensterMaal: 'Ligner målrettet: angrebene steg her, men ikke {v}, og {p} % kom fra ét netværk{t}.', moensterVerden: 'i resten af verden', moensterAndre: 'på dine andre domæner', moensterTek: ', og angrebene ramte {t}, som dit site bruger',
      gMaalH: 'Hvad betyder »Ligner målrettet«?', gRekogH: 'Hvad er rekognoscering?', gDnsRekogH: 'Hvad er DNS-rekognoscering?', gDmarcH: 'Nogen sender mails i dit domænes navn',
      notifRekog: 'Giv besked ved »Ligner rekognoscering«', notifRekogNote: 'Fra som standard: de fleste er automatiske scannere. Gælder også »Ligner DNS-rekognoscering«. Mærket vises altid i Overblik og Siden sidst.',
      tokenAabn: 'Opret API Token hos Cloudflare', tokenAabnNote: 'Knappen åbner Cloudflares side med navn og de to rettigheder udfyldt. Tjek dem, og opret API Tokenet dér.', tokenItFirma: 'Har du fået et API Token af dit IT-firma? Indsæt det her.', tokenSeTrin: 'Se trinene',
      tokenTrin: ['Knappen ovenfor åbner Create Token hos Cloudflare med navn og rettigheder udfyldt. Log ind først, hvis Cloudflare beder om det.', 'Tjek under Policies, at DNS & Zones viser 1 valgt (Zone › Read), og Analytics & Logs 1 valgt (Analytics › Read). Alle andre grupper skal stå på 0, og intet må stå på Edit: programmet læser kun.', 'Valgfrit: vil du se Cloudflares egne sikkerhedsfund under Angreb, så vælg Add new policy › Add new Account policy. Lad Include · All accounts stå, søg efter Security Center, og vælg Account Security Center Insights › Read. Rettigheden hører til kontoen: Zone Security Center Insights i den første politik virker ikke. Programmet virker uden.', 'Lad Include · All zones stå. Skal API Tokenet kun se ét domæne, vælger du Specific zone og domænet.', 'Lad Token expiration stå på No expiration, og lad IP-filteret (Client IP address filtering) være tomt.', 'Vælg Continue to summary. Der må kun stå Analytics: Read og Zone: Read (og Account Security Center Insights: Read under All accounts, hvis du tilføjede den). Vælg Create Token.', 'Kopiér API Tokenet, og indsæt det i feltet. Cloudflare viser det kun én gang.'], tokenTrinNote: 'Starter du selv fra Create Token, er felterne de samme.',
      slogan: 'Sikkerhed er ikke noget, man køber. Det er noget, man forstår.',
      forventH: 'Det her kan du forvente', velkomstLand: 'dit land',
      forvent: [
        ['Huginn holder øje med sikkerheden', ' på alle dine Cloudflare-domæner: angreb, robotter og login-forsøg, samlet på én skærm med en dom i klart sprog: {groen}, {gul} eller {roed}.'],
        ['Du får besked, når det betyder noget:', ' notifikationer fra din egen computer, når et domæne kræver handling — du bestemmer hvornår.'],
        ['Dyk ned i det, der skete:', ' klik på et tal og se reglerne, landene, netværkene, stierne og robotterne bag — og hvordan {land} og verden har det.'],
        ['Muninn husker:', ' tallene gemmes på din computer, også efter Cloudflares måned.'],
        ['Kun læse, ingen telemetri:', ' programmet kan ikke ændre noget i Cloudflare, og intet sendes til Wi24rd.'],
      ],
      filosofi: {
        h: 'Wi24rd — vores filosofi',
        afsnit: [
          'Vi mener, at cybersikkerhed skal være tilgængelig for alle – ikke kun for store virksomheder med dyre værktøjer og egne sikkerhedsteams.',
          'Ingen falske løfter. Ingen unødig kompleksitet. Bare praktiske værktøjer, klare indsigter og den viden, der skal til for at træffe bedre sikkerhedsbeslutninger.',
          'Lavet til at styrke dig. Ikke til at gøre dig afhængig.',
        ],
      },
      landLigeNu: '{land} lige nu', landVaelg: 'Land', landVerdenFane: 'Verden', landFaner: 'Land eller verden', landRadar: 'Cloudflare Radar · de seneste 30 dage', landVaelgP: 'Vælg det land, du vil sammenligne med.', landForslag: 'Skift til {land}', landForslagNote: 'Mindst halvdelen af dine besøgende kommer derfra.', landTilbage: 'Tilbage til {land}', landUkendt: 'Vælg land …', landIndstNote: 'Bruges i {land} lige nu.',
      landHenter: 'Tallene fra Cloudflare Radar hentes. Det tager op til et minut første gang.', landFejl: 'Tallene fra Cloudflare Radar er ikke opdateret: {f} Dine egne tal virker stadig.', landHentet: 'Hentet {t}',
      landKurveH: 'Dine domæner og hele verden', landKurveP: 'Begge kurver er regnet om, så 100 er periodens gennemsnit. De viser stigninger og fald, ikke antal.', landKurveTitel: 'Angreb som indeks, 100 er gennemsnittet for perioden. Piletaster flytter mellem dagene, Enter åbner dagen.', landMit: 'Mine domæner', landVerden: 'Verden, alle angreb', landVerdenWaf: 'Verden, WAF', landAkse: '100 = gennemsnit',
      landDag: '{d}: mine domæner {m} (indeks {i}), verden indeks {v}. Åbn dagen.', landDagUden: '{d}: mine domæner {m}, verden indeks {v}', landWafFjern: 'Vis alle angreb i verden',
      landFoelgerH: 'Dine domæner følger den globale tendens', landFoelgerP: '{n} angreb den {d} (snit {s}). WAF-angreb steg også i resten af verden.', landDigH: 'Stigningen er din egen', landDigP: '{n} angreb den {d} (snit {s}). Verden var normal den dag: WAF-angreb steg ikke i resten af verden.', landTendensHvad: 'Hvad betyder det? →',
      landAndelH: '{land} som mål', landAndelP: 'Hvor stor en del af de angreb, Cloudflare så i verden, der var rettet mod landet. Målet er zonens faktureringsland.', landAndel: '{land} var mål for {p} af angrebene {d}.', landUdenfor: '{land} var ikke blandt de 100 mest ramte lande {d}.', landAndelIngen: 'Ingen tal for {land} endnu.', landAndelDage: 'Dag for dag', landUdenforKort: 'ikke i top 100', dagDen: 'den {d}',
      landModH: 'De største kilder til angreb mod {land}', landModMineH: 'Hvor angreb mod dine domæner kom fra (alle lande)', landModBlokH: 'Hvor angrebene kom fra', landModP: 'Til venstre de største kilder til angreb mod {land} {p}, som andel af alle angreb mod landet, hvor målet er zonens faktureringsland. Til højre angreb mod de domæner, du har valgt, i samme periode.', landMod7Tekst: 'de seneste 7 dage', landMod28Tekst: 'de seneste 4 uger', landModRest: 'Andre lande (ikke oplyst af Radar)', landModPeriode: 'Periode', landMod7: '7 dage', landMod28: '4 uger', landModNote: 'Angriberens land bestemmes af IP-adressen og kan skyldes VPN eller lejede servere. Det gælder både for {land} og for dine domæner, så sammenligningen er fair, men landet siger ikke, hvem angriberen er.', landModIngen: 'Ingen tal for angreb mod {land} endnu.', landModIkkeOplyst: 'Cloudflare Radar oplyser ikke, hvor angrebene mod {land} kom fra (landet er for lille i statistikken).', landSerieFejl: 'Cloudflare Radar gav ikke tal til dette afsnit ved seneste opdatering. Resten af siden er ikke berørt.', landMineIngen: 'Ingen angreb på dine domæner i perioden.', landSaerligt: 'Særligt hos dig',
      landAndelKurve: 'Andel af alle angreb, Cloudflare så, dag for dag', landAndelDag: '{d}: {p} af alle angreb, Cloudflare så', landAndelDagUden: '{d}: ikke blandt de 100 mest ramte',
      landUdfaldH: 'Internetforstyrrelser i {land}', landUdfaldP: 'Det, Cloudflare Radar har registreret de seneste 30 dage.', landUdfaldIngen: 'Ingen kendte forstyrrelser i {land} de seneste 30 dage', landUdfaldVenter: 'Forstyrrelserne er ikke hentet endnu.', landUdfaldSeVerden: 'Se forstyrrelser i verden', landUdfaldVerdenH: 'Internetforstyrrelser i verden', landUdfaldVerdenP: 'De seneste, Cloudflare Radar har registreret de seneste 30 dage.', landUdfaldVerdenIngen: 'Ingen kendte forstyrrelser i verden de seneste 30 dage', landUdfaldNet: 'Berørte netværk: {liste}', landUdfaldUdenSlut: 'ingen slutdato oplyst', landUdfaldMin: '{n} {n|minut|minutter}', landUdfaldTimer: '{n} {n|time|timer}', landUdfaldDage: '{n} {n|dag|dage}', landUdfaldUkendt: 'Ukendt årsag',
      landUdfaldAarsag: { BLOCKING: 'Internettet blev blokeret', CABLE_CUT: 'Et kabel blev kappet', CYBERATTACK: 'Et cyberangreb', DNS: 'En fejl i DNS', FIRE: 'En brand', GOVERNMENT_DIRECTED: 'Beordret af myndighederne', MAINTENANCE: 'Vedligeholdelse', MECHANICAL: 'En mekanisk fejl', MILITARY_ACTION: 'Militære handlinger', MISCONFIGURATION: 'En fejl i opsætningen', NATURAL_DISASTER: 'En naturkatastrofe', NETWORK_PROBLEM: 'Et netværksproblem', POWER_OUTAGE: 'Strømsvigt', SOFTWARE: 'En softwarefejl', TECHNICAL_PROBLEM: 'Et teknisk problem', UNKNOWN: 'Ukendt årsag', WEATHER: 'Vejret' },
      landUdfaldSlags: { NATIONWIDE: 'hele landet', REGIONAL: 'en region', NETWORK: 'ét netværk', PLATFORM: 'en platform' },
      landKvanteH: 'Kvantesikre forbindelser', landKvanteSub: 'af HTTPS-forespørgslerne', landAiNavnH: 'AI-robotter efter navn', landRobotNote: 'Radar opdeler robotter og forbindelser efter, hvor trafikken kommer fra, ikke hvor sitet står. Derfor står de kun her og ikke under et land.',
      landMaalH: 'Mest ramte lande', landKildeH: 'Flest angreb fra', landProduktH: 'Hvad der stoppede dem', landBotH: 'Robottrafik', landBotSub: 'af al HTTP-trafik', landHttpH: 'HTTP-trafik', landHttpSub: 'indeks, 100 = 30 dages gennemsnit', landAiH: 'AI-robotter efter formål', landVerdenH: 'Angreb i verden', landVerdenP: 'Radar viser procenter og indeks, ikke antal. Tallene er for {d}.', landVerdenRobotH: 'Robotter i verden', landTom: 'Ingen tal fra Radar endnu.',
      landProdukt: { DDOS: 'DDoS-beskyttelse', WAF: 'WAF', BOT_MANAGEMENT: 'Bot-beskyttelse', ACCESS_RULES: 'Adgangsregler', IP_REPUTATION: 'IP-omdømme', API_SHIELD: 'API Shield', DATA_LOSS_PREVENTION: 'Data Loss Prevention' },
      landFormaal: { 'Mixed Purpose': 'Blandet formål', Training: 'Træning', Search: 'Søgning', 'User Action': 'Brugerhandling', Undeclared: 'Ikke oplyst' },
      landKilde: 'Kilde: Cloudflare Radar, CC BY-NC 4.0', landKildeAabn: 'Åbn Cloudflare Radar (åbner din browser)', hjLand: 'Her står dine egne angreb ved siden af verden fra Cloudflare Radar. Radar viser procenter og indeks, ikke antal, så kurverne siger noget om stigninger og fald. Angreb vises fra målsiden: hvor angreb mod landet kom fra, ved siden af hvor angreb mod dine domæner kom fra. Et kildeland alene kan ikke stoles på.',
      robotVerificeret: 'Verificeret ({k})', robotAssistent: 'AI-assistent i en browser', robotUkendt: 'Ukendt robot eller browser',
      robotSlags: { alle: 'Kendte robotter', udklaedt: 'Udklædte', uverificeret: 'Uverificerede', stoppet: 'Stoppet ved Edge', labyrint: 'Lokket ind i labyrinten' },
      robotGrupper: {
        'ai-traening': ['AI-træning', 'Samler tekst til at træne sprogmodeller.', 'Blokerer du dem, kan dine tekster undgå at blive brugt til træning. Nogle robotter bruges til både træning og søgning, så tjek først.'],
        'ai-soegning': ['AI-søgning og assistenter', 'Henter en side, fordi nogen spurgte en AI.', 'Blokerer du dem, kan dit site mangle i AI-svar.'],
        soegemaskiner: ['Søgemaskiner', 'Googlebot, Bingbot og andre indekserer dit site.', 'Blokerer du dem, kan du forsvinde fra søgeresultaterne.'],
        sociale: ['Sociale medier og link-previews', 'Henter titel og billede, når et link deles.', 'Blokerer du fx Meta, kan delte links miste billede og tekst, og annoncer kan blive påvirket.'],
        seo: ['SEO-værktøjer', 'Ahrefs, Semrush og lignende.', 'Blokerer du dem, kan de værktøjer miste data om dit site.'],
        overvaagning: ['Overvågning', 'Oppetid, sikkerhed og tilgængelighed.', 'Blokerer du dem, kan dine egne målinger stoppe.'],
        ukendt: ['Ukendt', 'Robotter, Cloudflare ikke har sat i en gruppe, og dem, der ikke er verificeret.', 'Det kan være nye robotter eller nogen, der kalder sig noget, de ikke er.'],
      },
      robotBesoeg: 'Besøg fra kendte robotter', robotAndel: 'Andel af al trafik', robotUdklaedtSub: 'kalder sig fx Googlebot uden at være det', robotStoppetSub: 'udfordret eller afvist af en regel', robotLabyrintSub: 'falske sider, som spilder robottens tid',
      robotHvemH: 'Hvem læser dit site', robotHvemP: 'Grupperne kommer fra Cloudflares verificerede kategorier. Blokering laver du selv hos Cloudflare: Huginn og Muninn læser kun.',
      robotMestAktive: 'Mest aktive robotter', robotMestLaeste: 'Mest crawlede sider', robotTomt: 'Ingen robotter i perioden.',
      robotIkkeTilg: 'Stoppet ved Edge er ikke tilgængelig for: {d}. Resten af visningen virker.', ekstraMangler: 'De ekstra tal hentes i baggrunden: {n} {n|dag|dage} mangler stadig.',
      robotNote: 'Cloudflare kan bruge stikprøver, så tallene kan være estimater. Tal pr. dag i UTC, fra de tal, der er gemt på din computer.', robotLabyrintNote: 'Labyrinten stopper ikke robotten. Den viser falske sider, så robotten spilder sin tid.',
      dykBesoeg: 'Besøg', dykRobotDage: 'Pr. dag · {p}', sidenNyRobot: 'Ny AI-robot på {domaene}: {robot}',
      hjRobotter: 'Robotter viser, hvem der læser dit site, og hvad der kan ske, hvis du blokerer dem. Blokering laver du selv hos Cloudflare.', hjLogin: 'Login viser, hvad der skete på de login-stier, du selv har valgt. De tre tal kommer fra hver sin kilde og lægges ikke sammen.',
      loginStierH: 'Login-stier pr. domæne', loginStierP: 'Slå de stier til, som er login på dit site. Vi gætter ikke på, hvad sitet kører.', loginDineStier: 'Dine login-stier:', loginRet: 'Ret stier', loginFiltre: 'Viser kun:',
      loginEgenH: 'Tilføj din egen login-sti', loginEgenEksempel: '/min-login/', loginEgenTilfoej: 'Tilføj', loginEgenFjern: 'Fjern', loginEgenFjernNavn: 'Fjern {sti}', loginStiUgyldig: 'Stien skal starte med / og må ikke have mellemrum eller ?',
      loginForslagH: 'Forslag fra dine data:', loginForslagTilfoej: 'Tilføj {sti} som login-sti', loginForslagNote: 'Foreslået, fordi stien fik POST med svar 200, 302, 401 eller 403 på mindst 3 forskellige dage.',
      loginIngenStier: 'Ingen login-stier er slået til for dette domæne. Vælg dem her, så tæller vi forsøgene.', loginVaelgDomaene: 'Stierne vælges pr. domæne: vælg et domæne øverst for at rette dem.',
      loginIkkeTilg: '"Nåede din server" er ikke tilgængelig for: {d}. Tallene fra Edge og 404 virker stadig.',
      loginUdfald: { edge: 'Stoppet ved Edge', ikke_fundet: 'Fik 404', server: 'Nåede din server', afvist: 'Afvist af din server', svar: 'Svar 200 eller 302' },
      loginEdgeSub: 'nåede aldrig din server', loginIkkeFundetSub: 'stien findes ikke', loginServerSub: 'din server svarede', loginAfvistSub: 'svar 401 eller 403', loginSvarSub: 'kan være lykket, kan være en fejlside',
      loginDag: '{d}: {e} stoppet ved Edge, {f} fik 404, {s} nåede din server', loginSoejleTitel: 'Forsøg pr. dag, tre søjler pr. dag. Piletaster flytter mellem dagene, Enter åbner.', loginDageH: 'Forsøg pr. dag', loginDageP: 'Tre tal fra hver sin kilde, side om side. De lægges ikke sammen: det kan være de samme forsøg.',
      loginTabelH: 'Stier og svar', loginCelle: '{sti}, {navn}: {n}. Åbn.', loginBetyder: 'Hvad det betyder', loginKortEdge: 'Edge', loginKort404: '404', loginKortServer: 'Din server', loginKortAfvist: 'Afvist', loginKortSvar: '200/302',
      loginSvarForkl: 'Svar 200 eller 302 siger ikke, om nogen kom ind. Det kan være login-siden, der vises igen.', loginForkl: { '/xmlrpc.php': 'Gammel fjernadgang til WordPress. Bruger du den ikke, kan du lukke den.' },
      loginHvorH: 'Hvor forsøgene kom fra', loginHvorP: 'Hver kilde for sig. Netværk fra din server står som nummer, hvis navnet ikke er set ved Edge.', loginLandeEdge: 'Lande, stoppet ved Edge', loginLandeServer: 'Lande, nåede din server', loginNetEdge: 'Netværk, stoppet ved Edge', loginNetServer: 'Netværk, nåede din server', loginMetodeEdge: 'Metoder, stoppet ved Edge', loginMetodeServer: 'Metoder, nåede din server',
      loginNote: 'Vi kan ikke se IP-adresser eller hvem der loggede ind, og tallene kan være stikprøver. Svar 200 eller 302 er ikke bevis på et login. Dit eget login tæller med, så vis ikke disse tal offentligt.',
      hentNu: 'Hent nu', tilbage: 'Tilbage', frem: 'Frem', hjaelpKnap: 'Hjælp', hjaelpH: 'Hjælp', hjaelpLuk: 'Luk hjælpen', genvejeH: 'Genveje',
      sidenTal: n => flertal(n, '1 domæne kræver handling', `${n} domæner kræver handling`),
      pilleHenter: 'Henter {z} af {Z}', pilleOpdateret: 'Opdateret {t}', pilleOpdateretFejl: 'Opdateret {t} · {n} fejlede', pilleVenter: 'Venter på Cloudflare',
      pillePause: 'Prøver igen kl. {kl}', pilleFejl: 'Hentning stoppet', pilleAabn: '{t} Åbn Overblik.',
      hjOm: 'Version, licenser og varemærke står bag Om-knappen (i) øverst til højre.',
      navBesoeg: 'Besøgende',
      hjBesoeg: 'Her ser du, hvor mange der besøger dine sites, og hvorfra. Unikke besøgende går et år tilbage. Besøg pr. land går en måned tilbage hos Cloudflare, og programmet gemmer dagene, så historikken vokser derfra.',
      hjBesoeg2: 'Et klik på en flise vælger perioden, et klik på en dag åbner trafikken den dag på Angreb, og et klik på et land, en robotgruppe eller en side viser dens tal pr. dag.',
      bes: {
        aar: 'Et år', fIGaar: 'Unikke besøgende i går', f7: 'Unikke besøgende på 7 dage', f30: 'Unikke besøgende på 30 dage', fAar: 'Unikke besøgende på et år', fBesoeg30: 'Besøg på 30 dage',
        ingenTal: 'Ingen tal endnu', ikkeTilgKort: 'Ikke tilgængelig', periodeAabn: '{navn}: vis perioden ({n})',
        grafH: 'Unikke besøgende pr. dag', grafLandH: 'Besøg fra {land} pr. dag', grafP: 'Et klik på en dag åbner trafikken den dag på Angreb.',
        hulNote: 'Grafen dækker {m} af {n} {n|dag|dage}. Dage uden tal er ikke 0: de er ikke hentet.',
        landeH: 'Hvor besøgene kom fra', landeP: 'Besøg pr. land i perioden. Et klik på et land viser besøgene fra det land pr. dag.',
        thLand: 'Land', thBesoeg: 'Besøg', thAndel: 'Andel', visAlle: 'Vis alle {n} {n|land|lande}', landAabn: 'Vis besøg fra {navn} pr. dag ({n} besøg)',
        landeFra: 'Besøg pr. land findes fra {d}. Cloudflare har dem i en måned, og programmet gemmer dagene derfra.',
        landeIngen: 'Ingen besøg i perioden.', landeIkkeHentet: 'Besøg pr. land er ikke hentet endnu. De hentes i baggrunden efter de andre tal.',
        ikkeTilg: 'Besøg pr. land er ikke tilgængelige for dette domæne hos Cloudflare. Resten af siden virker.', ikkeTilgFlere: 'Besøg pr. land er ikke tilgængelige hos Cloudflare for {d}. Resten af siden virker.',
        note: 'Unikke besøgende er unikke IP-adresser pr. dag: robotter tæller med, og flere personer bag samme router tæller som én. En sum over flere dage tæller den samme besøgende én gang pr. dag. Landet er det, Cloudflare så, og kan være en VPN. Besøg er Cloudflares estimat ud fra en stikprøve: en sidevisning, der kom fra et andet site eller et direkte link. Tal pr. dag i UTC, fra de tal, der er gemt på din computer.',
        noteSamlet: 'Alle domæner lægges sammen, så den samme besøgende på to domæner tæller to gange.',
        fBesoegLand: 'Besøg fra {land} på 30 dage', fBesoegSti: 'Besøg på {sti} på 30 dage', alleLande: 'alle lande', alleSider: 'alle sider',
        robotLinje: 'heraf {n} fra kendte robotter ({p})',
        grafUgeH: 'Unikke besøgende pr. uge', grafLandUgeH: 'Besøg fra {land} pr. uge', grafStiH: 'Visninger af {sti} pr. dag', grafStiUgeH: 'Visninger af {sti} pr. uge',
        grafRobotH: 'Besøg fra {gruppe} pr. dag', grafRobotUgeH: 'Besøg fra {gruppe} pr. uge',
        grafUgeP: 'Én søjle pr. uge, mandag til søndag. Et klik på en uge viser ugen.',
        uge: 'Ugen fra {a} til {b}: {n}', ugeDele: 'Ugen fra {a} til {b}: {n} ({m} af 7 dage)', ugeHul: 'Ugen fra {a} til {b}: ingen tal', ugeAabn: 'Vis ugen fra {a} til {b} ({n})',
        robotH: 'Kendte robotter', robotP: 'Besøg fra robotter, Cloudflare har verificeret, i perioden, i Robotter-visningens grupper. Et klik på en gruppe viser dens besøg pr. dag.',
        thGruppe: 'Gruppe', gruppeAabn: 'Vis besøg fra {navn} pr. dag ({n} besøg)',
        robotIngen: 'Ingen besøg fra kendte robotter i perioden.', robotIkkeHentet: 'Kendte robotter er ikke hentet endnu. De hentes i baggrunden efter besøgene.',
        robotIkkeTilg: 'Kendte robotter er ikke tilgængelige hos Cloudflare for {d}. Resten af siden virker.',
        klasseLinje: '{p} af trafikken det seneste år kom fra kendte robotter og scannere.', klasseDage: '({m} af 365 dage)',
        robotNote: 'Kendte robotter er dem, Cloudflare har verificeret, fx Googlebot. Robotter, der udgiver sig for at være en almindelig browser, kan ikke ses her og tæller som besøgende. Andelen af trafikken bygger på Cloudflares IP-klasser: søgemaskiner, overvågning, scannere, Tor og kendte dårlige adresser.',
        sideH: 'Mest besøgte sider', sideP: 'Sider (html) i perioden: visninger, besøg udefra og andel af visningerne. Et klik på en sti viser visningerne af siden pr. dag.',
        thSti: 'Side', thVisninger: 'Visninger', thBesoegUde: 'Besøg udefra', stiAabn: 'Vis visninger af {sti} pr. dag ({n} {n|visning|visninger})',
        visAlleSider: 'Vis alle {n} {n|side|sider}', side404H: 'Sider, der ikke findes', side404P: 'Stier, der svarede 404 (ikke fundet), ofte robotter, der prøver kendte adresser. Ikke alle forsøg står her: svarer sitet med sin almindelige side, kommer stien ikke på denne liste.',
        sideForsideH: 'Svarede med sidens almindelige side', sideForsideP: 'Siden findes sandsynligvis ikke: sitet svarer med forsiden på adresser, det ikke kender.',
        sideForsideNote: 'Et skøn ud fra størrelsen: svaret var lige så stort som forsiden (højst 25 % fra), og ingen kom udefra.',
        sideUdenForside: 'Forsiden er ikke blandt siderne i perioden, så stier, der svarer med forsiden, kan ikke skilles fra.',
        fasteH: 'De seneste dage', valgtPeriode: 'Valgt periode: {p}',
        sideIngen: 'Ingen sidevisninger i perioden.', sideIkkeHentet: 'De mest besøgte sider er ikke hentet endnu. De hentes i baggrunden efter besøgene.',
        sideIkkeTilg: 'De mest besøgte sider er ikke tilgængelige hos Cloudflare for {d}. Resten af siden virker.', sideFra: 'Siderne findes fra {d}.',
        sideNote: 'Robotter tæller med. En side som /admin besøges næsten altid af scannere, ikke af mennesker. Stierne vises uden forespørgselsstreng, og lange koder er skjult. De 50 mest viste sider gemmes pr. dag.',
      },
      sub: {
        h: 'Subdomæner', pBes: 'Besøg, forespørgsler og angreb pr. subdomæne i perioden. Et klik på et subdomæne viser Besøgende for det.', pAng: 'Besøg, forespørgsler og angreb pr. subdomæne i perioden. Et klik på et subdomæne viser Angreb for det.',
        th: 'Subdomæne', thBesoeg: 'Besøg', thForesp: 'Forespørgsler', thAngreb: 'Angreb', thAndel: 'Andel', andre: 'Andre ({n})', visAndre: 'Vis {n} {n|subdomæne|subdomæner} mere',
        filter: 'Subdomæne: {vaert}', ariaBes: 'Vis Besøgende for {vaert}', ariaAng: 'Vis Angreb for {vaert}',
        fra: 'Pr. subdomæne fra {dato}.', dnsOnly: 'Kun trafik gennem Cloudflare. Et subdomæne, der er «DNS only», har ingen tal her.',
        ikke: 'Ikke pr. subdomæne', ikkeTilg: 'Cloudflare svarede ikke på subdomænerne for dette domæne.', afskaaret: 'Kun de mest besøgte sider er med: Cloudflare sendte højst 10.000 rækker.',
        ip: 'Kaldt med IP-adresse', anden: 'Andet navn', ugyldig: 'Ugyldigt navn',
        fBesoeg: 'Besøg på {vaert} på 30 dage', fForesp: 'Forespørgsler til {vaert} på 30 dage',
        grafBesoegH: 'Besøg på {vaert} pr. dag', grafBesoegUgeH: 'Besøg på {vaert} pr. uge', grafForespH: 'Forespørgsler til {vaert} pr. dag', grafForespUgeH: 'Forespørgsler til {vaert} pr. uge', grafP: 'Et klik på en dag viser dagen.',
        fAngreb: 'Angreb på {vaert}', fAfvist: 'Afviste på {vaert}', rulDage: '{navn}: {n}. Gå til dagene.',
        dageH: 'Angreb og afviste på {vaert} pr. dag', dageP: 'Angreb og afviste regnes som i resten af Angreb. Et klik på en dag viser dagen.', dag: '{d}: {a} angreb, {b} afviste', legAngreb: 'Angreb', legAfvist: 'Afviste',
        hRegler: 'Reglerne, der slog til', stiVaert: 'Vis visninger af {sti} på {vaert} pr. dag ({n} {n|visning|visninger})',
      },
      ang: {
    henter: 'Henter tallene fra Cloudflare …', opdateret: 'Hentet fra Cloudflare {t}', opdaterer: 'Viser de gemte tal fra {t} · henter nye fra Cloudflare …',
    fejl: 'Tallene kunne ikke hentes: {f}', fejlGemte: 'Nye tal kunne ikke hentes: {f} Visningen viser de senest gemte tal fra {t}.', ingenZone: 'Intet domæne er valgt. Vælg et i listen.',
    mangler: 'Programmets forbindelse til Rust-delen mangler. Start det med npm run tauri dev.',
    d1Sub: 'i dag hidtil: {n}', andel: '1 af hver {n} {n|forespørgsel|forespørgsler}', forsoeg: 'Snusere, der intet fandt ({p})', forsoegSub: 'forespørgsler efter filer, der ikke findes',
    fordI: 'I går', ford: 'heraf {b} blokeret · {u} udfordret · {l} lokket ind i labyrinten', fordAndet: ' · og {a} andet, som ikke tæller som afvist',
    fordNote: 'Cloudflare stopper på flere måder: blokerer, udfordrer (et browsertjek, som en robot ikke klarer) eller lokker robotten ind i en labyrint af falske sider. Sider med skjulte fælde-links blev leveret og tæller ikke som afvist; først når en robot følger et link, er den lokket ind. Hele dage regnes i UTC.',
    kortH: 'Hvor angrebene kom fra', kortP: 'Hver bue går fra det land, angrebet kom fra, til det datacenter i Cloudflares edge, der stoppede det. Det sker i det datacenter, der ligger nærmest angriberen, længe før noget når dit site.',
    kortTitel: 'Verdenskort over afviste angreb · {p}', kortDesc: 'Flest kom fra {lande}. Flest blev stoppet i {colos}.',
    verden: 'Verden', europa: 'Europa', leg: { land: 'Land, angrebet kom fra', colo: 'Datacenter, der stoppede det', bue: 'Et angreb på vej', samme: '◆ i en ring = stoppet i samme land, som angrebet kom fra' },
        dageP: 'Første forsvarslinje: Cloudflares edge, som al trafik møder, før den når din server. Afviste forespørgsler pr. dag (UTC).', dageTitel: 'Søjlediagram over afviste forespørgsler pr. dag · {p}', ialt: 'i alt', maks: 'flest: {n} den {d}',
    vis: 'Vis tallene', dag: 'Dag', antal: 'Afvist', trafik: 'Alle forespørgsler',
    reglerP: 'Cloudflares forvaltede regelsæt kender angrebsmønstre mod almindelig software. Her er de regler, der stoppede noget på {domaene}.',
    reglerNote: 'Om et angreb ville have virket, afhænger af, hvad sitet kører. Cloudflare stoppede det alligevel, før det nåede frem.',
        forsoegH: 'Hvad de ledte efter', forsoegP: 'Ikke alt bliver blokeret. Mange robotter går bare rundt og prøver dørhåndtag: de spørger efter filer, som andre steder ved et uheld ligger fremme. Her svarede {domaene} 404: filen fandtes ikke.',
    forsoegPP: 'Ikke alt bliver blokeret. Mange robotter går bare rundt og prøver dørhåndtag: de spørger efter filer, som andre steder ved et uheld ligger fremme. Her svarede dine {domaene} 404: filen fandtes ikke.',
    ingenTalSamlet: 'Ingen af dine domæner har tal endnu. De hentes i baggrunden, og siden tegnes, når de kommer.',
    mAfN: '{m} af {n} {n|domæne|domæner} har tal endnu. Resten hentes i baggrunden.', valgteH: 'Dine domæner', flere: '+ {n} flere', udenTal: 'ingen tal endnu', opdateretSamlet: 'Hentet fra Cloudflare {t} (det ældste af de valgte)', opdatererSamlet: 'Hentet fra Cloudflare {t} (det ældste af de valgte) · henter i baggrunden …',
    nulstil: 'Nulstil', nulstilLabel: 'Nulstil alle filtre og perioden', trafikChip: 'Trafik', skjult: 'Skjult med dette filter, fordi tallene ikke kan filtreres sådan: {liste}.', vurderingNavn: 'Vurdering', soejlerNavn: 'Søjlerne pr. dag',
      },
      vejledningAabn: 'Åbn vejledningen', vejledningLabel: 'Åbn vejledningen i din browser',
      omVersion: 'Version {v}', omLinje: 'Apache-2.0-licens · Ingen telemetri · Intet sendes til Wi24rd',
      om2: 'Cloudflare® er et registreret varemærke tilhørende Cloudflare, Inc. Wi24rd Huginn and Muninn er ikke tilknyttet Cloudflare, Inc.',
      om3: 'Skrifter: Inter og JetBrains Mono (SIL Open Font License 1.1). Kortdata: world-countries (ODbL) og Natural Earth (public domain).',
      omKilde: 'Kildekode: {url}',
      proeverOm: 'Prøver igen om {s} s', proeverNu: 'Prøver igen …', proevNu: 'Prøv igen nu',
      domaener: n => flertal(n, '1 domæne', `${n} domæner`),
      fejl: {
        forbindelse: 'Ingen forbindelse til Cloudflare. Tjek din internetforbindelse.',
        afvist: 'Cloudflare afviste API Tokenet. Det kan være udløbet eller mangle rettigheder.',
        vent: 'Cloudflare beder os vente lidt (for mange forespørgsler).',
        cf_nede: 'Cloudflare svarer ikke lige nu.',
        http: 'Cloudflare svarede med HTTP {status}.',
        api: 'Cloudflare svarede med en fejl: {detalje}',
        radar: 'Cloudflare Radar svarer ikke lige nu.',
        graphql: 'Cloudflare kunne ikke levere tallene for {dag}.', graphqlUdenDag: 'Cloudflare kunne ikke levere tallene.',
        svar: 'Uventet svar fra Cloudflare.',
        zone_ukendt: 'Zonen findes ikke, eller API Tokenet har ikke adgang til den.',
        intet_token: 'Der er ikke gemt noget API Token. Indsæt dit API Token under Forbind med Cloudflare.',
        token_tomt: 'Feltet er tomt. Indsæt dit API Token.',
        token_form: 'Det indsatte ligner ikke et API Token fra Cloudflare (kun bogstaver, tal og tegn uden mellemrum).',
        token_inaktiv: 'API Tokenet er ikke aktivt (status: {detalje}). Opret et nyt API Token, der kun kan læse.',
        mangler_zone: 'API Tokenet kan ikke se nogen domæner: det mangler rettigheden Zone › Read, eller domænet står ikke under Include · All zones eller Specific zone.',
        mangler_analytics: 'API Tokenet mangler rettigheden Analytics › Read, så programmet kan ikke læse tallene.',
        ugyldig_zone: 'Ugyldigt zone-id.', ugyldig_dato: 'Ugyldig dato.', ugyldigt_filter: 'Filteret i adressen kan ikke bruges.', for_mange_stier: 'Du kan højst have 20 egne login-stier pr. domæne. Fjern en, før du tilføjer en ny.',
        noegle: 'Nøgleringen kunne ikke bruges.',
        lager: 'Den lokale database kunne ikke bruges.',
        nyere_database: 'Den lokale database er fra en nyere version af programmet (skema {detalje}).',
        autostart: 'Start ved login kunne ikke læses eller ændres.',
        allerede_forbundet: 'Dette API Token ser de samme domæner som «{navn}». Det er ikke tilføjet.',
        intern: 'Der opstod en fejl i programmet.',
      },
      // Opsætning
      tokenH: 'Forbind med Cloudflare',
      tokenP: 'Indsæt et API Token, der kun kan læse. Det gemmes i din computers nøglering (macOS Nøglering eller Windows Credential Manager) og forlader aldrig programmets Rust-del. Webvisningen kan ikke hente det tilbage.',
      tokenFelt: 'API Token (kun læsning)', tokenKnap: 'Gem og forbind',
      hjaelpTokenH: 'Sådan opretter du et API Token',
      gemmer: 'Tjekker API Tokenet hos Cloudflare …', gemt: 'API Tokenet er gyldigt og gemt i nøgleringen.', glemt: 'API Tokenet er slettet fra nøgleringen.',
      mangler: 'Programmets forbindelse til Rust-delen mangler. Start det med npm run tauri dev.',
      // Overblik
      zonerH: 'Overblik',
      hjOverblik1: 'De domæner, du har valgt under Vælg domæner, hentes i baggrunden, mens programmet kører, også når vinduet er lukket, og det kun står i menulinjen.',
      hjOverblik2: 'Status sammenligner angreb i går med snittet for ugen før: Handl, Hold øje, Roligt eller Ukendt (ingen tal endnu).',
      hjOverblik3: 'Om natten (til kl. 03 UTC), før i går er hentet færdigt, bruger status dagen før og står som "opdateres …".',
      hjOverblik4: 'Cloudflares automatiske bot-tjek (labyrinten, Bot Fight Mode og browsertjekket) er rutine og giver aldrig Hold øje eller Handl. "Afvist" tæller det, Cloudflare blokerede, udfordrede eller lokkede ind i labyrinten; status tæller kun egentlige angreb.',
      hjOverblik5: 'Handl: Cloudflare har allerede stoppet angrebene. Bed den, der passer dit website, om at tjekke, at software og plugins er opdateret. Klik på en række for at se angrebene, eller se dem alle samlet.',
      domHjaelp: 'Hvad gør jeg? →', domHjaelpRoligt: 'Hvorfor er alt roligt? →', domLuk: 'Skjul',
      domRoed: n => flertal(n, '1 domæne kræver handling', `${n} domæner kræver handling`),
      domGul: n => flertal(n, '1 domæne skal holdes øje med', `${n} domæner skal holdes øje med`), domIntet: 'Intet at gøre', domUkendt: 'Ingen tal endnu',
      domHoldOeje: ': hold øje.', domRoligt: '{n}: alt roligt.', domGraa: '{n}: ingen tal endnu.', domGamle: '{n}: tallene er gamle.', domAltRoligt: 'Alt er roligt på {n} i går.', domHentes: 'Tallene hentes i baggrunden.',
      samletKnap: 'Angreb for alle', fravalgtNote: n => flertal(n, '1 domæne er ikke valgt — Vælg domæner', `${n} domæner er ikke valgt — Vælg domæner`), intetValgtH: 'Du har ikke valgt nogen domæner endnu', intetValgtP: 'Vælg de domæner, Huginn skal holde øje med. Kun dem hentes og vises.', intetValgtKnap: 'Vælg domæner',
      nyeLinje: n => flertal(n, '1 nyt domæne på din Cloudflare-konto:', `${n} nye domæner på din Cloudflare-konto`), nyVaelg: 'Vælg', nyVaelgFor: 'Vælg {navn}', nyIkkeNu: 'Ikke nu', nyIkkeNuFor: 'Ikke nu: {navn}', nyeSe: 'Se dem', nyeSeLabel: 'Se de nye domæner under Vælg domæner',
      domaenerH: 'Domæner', domaenerP: 'Vælg de domæner, Huginn skal hente og vise. Et domæne, du fravælger, hentes ikke længere; dets tal bliver liggende.', domaenerVis: 'Vis i Huginn', domaenerFor: 'Vis {navn} i Huginn', domaenerAlleTil: 'Vælg alle', domaenerAlleFra: 'Fravælg alle', domaenerSer: n => flertal(n, 'API Tokenet ser 1 domæne', `API Tokenet ser ${n} domæner`), domaenerGemt: 'Domænerne er gemt.',
      tabelTitel: 'Dine domæner med tal fra den lokale cache', thDomaene: 'Domæne', thStatus: 'Status', thHvorfor: 'Hvorfor', thAfvist: 'Afvist', thAngreb: 'Angreb', thIGaar: 'i går',
      th7: '7 dage', th30: '30 dage', thUgen: 'denne uge', thSidsteUge: 'sidste uge', thAabn: 'Åbn',
      ovIGaar: 'afvist i går', ov7: 'afvist på 7 dage', ov30: 'afvist på 30 dage', ovUge: 'angreb denne uge', ovSidste: 'angreb sidste uge',
      status: { roed: 'Handl', gul: 'Hold øje', groen: 'Roligt', graa: 'Ukendt' },
      hvServer: 'Din server svarer ikke: {p} af {n} svar i går var fejl 520–527, så Cloudflare kunne ikke få svar fra din server. Se »Hvad gør jeg?«.', hvServerNat: 'Din server svarer ikke: {p} af {n} svar i forgårs var fejl 520–527, så Cloudflare kunne ikke få svar fra din server. Se »Hvad gør jeg?«.', hvForstyrrelse: 'Samme dag var der en forstyrrelse i {land}.', hvForstyrrelseNet: 'Samme dag var der en forstyrrelse hos {net} i {land}.',
      serverLinje: 'Din server svarer ikke:', serverAabn: 'Hvad gør jeg ved {navn}?', gServerH: 'Din server svarer ikke', gDnsH: 'Din DNS peger måske på en IP-adresse, du ikke ejer længere',
      trafikMaerke: 'Usædvanlig meget trafik', trafikAabn: 'Usædvanlig meget trafik på {navn} den {d}: {n} {n|forespørgsel|forespørgsler}. Åbn dagen.', trafikDag: 'Usædvanlig meget trafik den {d}: {n} {n|forespørgsel|forespørgsler}. Cloudflare stoppede {s} som angreb, og {o} slap igennem', trafikDagMest: 'Usædvanlig meget trafik den {d}: {n} {n|forespørgsel|forespørgsler}. Det meste var angreb, som Cloudflare stoppede: {s}, og {o} slap igennem', trafikNote: 'Det, Cloudflare ikke stoppede, tæller ikke som angreb. Det kan være et forsøg på at overbelaste siden (DDoS), en robot, der løber løbsk, eller en fejl. En normal dag har {m} {m|forespørgsel|forespørgsler}.',
      hvTrafik: 'Usædvanlig meget trafik: {n} {n|forespørgsel|forespørgsler} (normalt {m}). {stop} Se, hvor trafikken kom fra.', trafikStop: 'Cloudflare stoppede {s} som angreb. {o|Den sidste|De øvrige} {o} slap igennem og tæller ikke som angreb — det kan være et forsøg på at overbelaste siden (DDoS), en robot, der løber løbsk, eller en fejl.', trafikStopMest: 'Det meste af trafikspringet var angreb, som Cloudflare stoppede. Det var {s}. {o|Den sidste|De øvrige} {o} slap igennem og tæller ikke som angreb.', gTrafikH: 'Hvad gør jeg ved usædvanlig meget trafik?',
      dykTrafikH: 'Trafik den {v}', dykTrafikP: 'Her står forespørgslerne, ikke det, Cloudflare stoppede.', trafikIAlt: 'Forespørgsler', trafikNormalt: 'normalt {m}', trafikStoppet: 'Cloudflare stoppede', trafikLande: 'Besøgende pr. land', trafikStatus: 'Svarkoder', trafikRobotter: 'Robotter', trafikEtLand: '{p} kom fra ét land ({land}).', trafik52: '{p} fik 52x (din server svarede ikke).', trafikKode: '{p} fik svaret {k}.', trafikRobot: 'De fleste var robotten {r}.', trafikIngenEkstra: 'Tal for lande og svarkoder er ikke hentet for dagen endnu.', trafikTomt: 'Ingen tal for dagen.',
      tek: TEK,
      hvIkkeBrugt: '{n} angreb på {t}, som du ikke bruger — ufarlige for dig.', hvUventet: 'Uventet: din server svarede på en {t}-sti. Bruger du alligevel {t}?', hvNaaet: '{n} angreb på {t} nåede din server, og du har sagt, at du bruger {t}.', hvBesked: 'Flere angreb på {t} end normalt: {n} (snit {s}).', hvSpoerg: 'Bruger du ikke {t}? Sig det her',
      tekH: 'Dit sites teknologi', tekP: 'Angreb på noget, sitet ikke kører, kan ikke ramme det. Sig, hvad du bruger, så vejer angrebene rigtigt i status og advarsler. Tallene ændrer sig ikke: alle angreb står stadig i Angreb og på kortet.', tekVedIkke: 'Ved ikke', tekBesked: 'Giv mig besked', tekBeskedNote: 'Når der er flere angreb på {t} end normalt, uanset niveauet for notifikationer.', tekForsoeg: '{n} {n|forsøg|forsøg} på 30 dage', tekForslagJa: 'sæt til »Bruger ikke«?', tekGeneriskH: 'Altid relevant', tekGeneriskP: 'Hemmeligheder, kildekode og login er ikke en teknologi, du kan fravælge.',
      hulLinje: 'Huginn var ikke tændt fra {a} til {b}. Cloudflare gemmer kun en måned, så {n} {n|dag|dage} mangler for altid i historikken.', hulLinjeIntet: 'Huginn var ikke tændt fra {a} til {b}. Cloudflare havde stadig tallene, så intet mangler.', hulLuk: 'Luk', hulLukLabel: 'Luk linjen om hullet fra {a} til {b}', dagMed: '{m} af {n} {n|dag|dage}', hulNote: 'Tallene dækker {m} af {n} {n|dag|dage}. Resten er ikke hentet: ingen data — Huginn var slukket.', hulIngen: 'ingen data', hulListeH: 'Huller i historikken', hulListeP: 'Dage uden tal. De er ikke 0: de er aldrig hentet.',
      datoFra: 'Fra', datoTil: 'Til', datoNulstil: 'Nulstil', datoNulstilLabel: 'Nulstil til hele historikken', datoNote: 'Hele dage i UTC. Tidspunktet på dagen gemmes ikke.', datoFejl: '»Fra« skal ligge før eller på »Til«.',
      regelStier: 'Bankede på', regelFra: 'Fra', regelFoerst: 'Først set {d}, sidst {s}', dykReglerH: 'Reglerne, der slog til — {u}', dykKortN: 'i perioden',
      hvIngenTal: 'Ingen tal endnu – henter.', hvGammel: 'Tallene er gamle – henter nye.', hvIngen: 'Ingen angreb i går.', hvIngenNat: 'Ingen angreb i forgårs.',
      hvAngreb: '{a} angreb i går ({s})', hvAngrebNat: '{a} angreb i forgårs ({s})', hvSnit: 'snit {s}', hvFoerste: 'første gang', egenRegel: 'din egen regel »{navn}«',
      gTitel: 'Hvad gør jeg?', gRoligtTitel: 'Hvorfor er alt roligt?', gIntro: 'Rådene er generelle. Er du i tvivl, så spørg den, der passer dit website.',
      gHvad: 'Hvad skete der:', gRammer: 'Rammer det dig?', gHvisJa: 'Hvis ja, gør dette:', gGoer: 'Gør dette:', gHvisNej: 'Hvis nej:',
      gAlleStoppet: 'Cloudflare stoppede {n|det|alle {n}}.', gKomIgennem: '{k} kom igennem.', gKomIgennemTrin: 'Åbn domænets angreb, og vis dem til din webmand.',
      gSe: n => flertal(n, 'Se angrebet fra i går →', `Se de ${n} angreb fra i går →`), gSeNat: n => flertal(n, 'Se angrebet fra i forgårs →', `Se de ${n} angreb fra i forgårs →`),
      cveAabn: 'Åbn {id} på cve.org (åbner din browser)', cveLaes: 'Læs om {id} ↗',
      hjCve: 'Et klik på et CVE-nummer (fx CVE-2025-55182) åbner CVE’ens side på cve.org i din browser. Programmet selv henter intet derfra.',
      
      dagIGaar: 'i går', 
      gRoligt1: 'Cloudflare afviste {n} forsøg i går på {d}, og {n|det var ikke usædvanligt|ingen af dem var usædvanlige}.', gRoligt2: 'Det er normalt: robotter prøver hele tiden kendte huller på alle sites, også dine. Status bliver først Hold øje eller Handl, når et domæne får usædvanligt mange angreb i forhold til ugen før.',
      hvFoelger: 'Følger den globale tendens.', hvSaerligtDig: 'Verden var normal i går, så stigningen er din egen.', hvSaerligtDigNat: 'Verden var normal i forgårs, så stigningen er din egen.',
      hvAlleStoppet: '{a|stoppet|alle stoppet}', hvNafM: '{n} af {a} stoppet', hvMest: 'Mest {liste}.', hvOg: ' og ', hvOpdateres: 'Opdateres …',
      nBlokeret: '{n} blokeret', nUdfordret: '{n} udfordret', nLabyrint: '{n} i labyrinten', nAndet: '{n} andet',
      henterKort: 'henter …', hentetTitel: 'Hentet {t}', ugeTitel: '{p} mod ugen før', aabnFor: 'Åbn Angreb for {navn}',
      fodnote: '"Afvist" tæller det, Cloudflare blokerede, udfordrede eller lokkede ind i labyrinten, også rutine (Bot Fight Mode, browsertjek). Sider, der blev leveret med skjulte fælde-links, tæller ikke med. Status tæller kun egentlige angreb. Klik på et tal for at se det nærmere, eller på resten af rækken for at åbne domænets angreb.',
      koeHenter: 'Henter i baggrunden: {z} af {Z} {Z|domæne|domæner} · {d} af {D} {D|dag|dage}', koeNu: 'nu: {navn}',
      koeKvote: 'Cloudflare beder os vente (kvoten er brugt) – fortsætter kl. {kl}.',
      koePause: 'Cloudflare svarer ikke lige nu – prøver igen kl. {kl}.', koePauseNet: 'Ingen forbindelse til Cloudflare – prøver igen kl. {kl}.',
      koeFaerdig: 'Alle domæner er hentet (sidst {t}).', koeFaerdigFejl: 'Hentet (sidst {t}). {n} kunne ikke hentes: {f}',
      koeNet: 'Ingen forbindelse til Cloudflare, så {d} kunne ikke hentes. Prøver igen af sig selv.', pilleNet: 'Ingen forbindelse · prøver igen',
      koeFejl: 'Hentningen i baggrunden er stoppet: {f}', koeFremskridt: 'Fremskridt for hentningen i baggrunden',
      henterZoner: 'Henter dine zoner …', ingenZoner: 'API Tokenet kan ikke se nogen zoner. Tjek, at det har rettigheden Zone › Read.',
      gemteZonerNet: 'Ingen forbindelse til Cloudflare – viser dine gemte zoner fra {t}', gemteZoner: '{f} Viser dine gemte zoner fra {t}.',
      // Siden sidst
      sidenH: 'Siden sidst', sidenVist: 'Status for i går. Du kiggede her sidst {t}.', sidenFoerste: 'Status for i går på alle dine domæner.',
      sidenGroenne: '{n}: intet usædvanligt', sidenRoligt: 'Alt er roligt: {n}, {a} angreb i går{b}.', sidenAlleBlok: ', alle blokeret',
      sidenGamle: '{n}: tallene er gamle – Huginn henter', sidenGraa: '{n}: ingen tal endnu', ny: 'ny', sidenSkjul: 'Til overblikket',
      hjSiden: 'Siden sidst viser de domæner, der kræver handling eller skal holdes øje med, og om de er nye, siden du sidst kiggede. Visningen åbner selv, når du klikker på en notifikation eller vælger Siden sidst i menulinjen.',
      // Angreb (Værnet)
      vaernetH: 'Angreb', vaelgDomaene: 'Domæne', samletValg: 'Samlet ({n})',
      lead: 'Hver dag banker robotter på {domaene} med kendte angreb mod software, som sitet måske slet ikke kører. Cloudflares edge stopper dem i det datacenter, der ligger nærmest angriberen, før de når frem. Her er, hvad Cloudflare har set de seneste 30 dage: hvor angrebene kom fra, hvilke regler der slog til, og hvad de ledte efter. Ingen IP-adresser bliver hentet eller vist.',
      lead__fl: 'Hver dag banker robotter på dine {domaene} med kendte angreb mod software, som siderne måske slet ikke kører. Cloudflares edge stopper dem i det datacenter, der ligger nærmest angriberen, før de når frem. Her er, hvad Cloudflare har set de seneste 30 dage på dem alle tilsammen: hvor angrebene kom fra, hvilke regler der slog til, og hvad de ledte efter. Ingen IP-adresser bliver hentet eller vist.',
      edgeH: 'Edge – første forsvarslinje', edgeP: 'Edge er Cloudflares netværk af datacentre. Dit site svarer fra dem alle (anycast), så et angreb stoppes i det datacenter, der ligger nærmest angriberen, længe før det når din server. Det gælder kun poster med orange sky. Kender en angriber serverens egen IP-adresse, kan den gå uden om edge, medmindre serveren kun tager imod trafik fra Cloudflare.',
      metodeH: 'Sådan tælles der',
      k1: 'GraphQL Analytics API', k2: 'Programmets Rust-del', k3: 'SQLite på din computer', k4: 'Dette vindue',
      m1H: 'Kilden', m1: 'Cloudflares GraphQL Analytics API for zonen {domaene}: firewall-hændelser, alle forespørgsler og verificerede bots. API Tokenet kan kun læse.',
      m1__fl: 'Cloudflares GraphQL Analytics API for hver af dine zoner ({domaene}), lagt sammen på din computer: firewall-hændelser, alle forespørgsler og verificerede bots. API Tokenet kan kun læse.',
      m2H: 'Når du åbner visningen', m2: 'Programmet spørger om i dag og i går, én dag ad gangen, så Cloudflare ikke skal sample over en lang periode. Ældre dage hentes kun, hvis de mangler. Højst to kald ad gangen og mindst 1,1 sekund imellem, for alle zoner tilsammen; beder Cloudflare os vente, holder programmet pause.',
      m2__fl: 'En kø henter alle dine zoner i baggrunden, én dag ad gangen, så Cloudflare ikke skal sample over en lang periode. Ældre dage hentes kun, hvis de mangler. Højst to kald ad gangen og mindst 1,1 sekund imellem; beder Cloudflare os vente, holder køen pause. Visningen tegnes igen, når der kommer nye tal.',
      m3H: 'Hukommelsen', m3: 'Dagene gemmes som optællinger i en SQLite-fil i programmets datamappe på din computer. Intet slettes af sig selv, og næste gang vises de gemte tal med det samme, mens nye hentes.',
      m4H: 'Det, der ikke gemmes', m4: 'IP-adresser bliver hverken hentet eller gemt. Stier gemmes uden forespørgselsdel, lange koder skjules, og brugeragenter gemmes kun som botnavn. Antallet af afviste er Cloudflares estimat ud fra en stikprøve: Cloudflare gemmer ikke hver hændelse, men oplyser for hver, hvor mange den står for, og programmet lægger det sammen.',
      kilder: 'Landenes midtpunkter: world-countries (ODbL). Landkonturer: Natural Earth (public domain).',
      // Dyk
      hjDyk1: 'Siden viser én periode ad gangen: 7 dage, 30 dage eller de datoer, du vælger. Alle afsnit følger perioden og filtrene. Et filter viser de gemte tal og henter intet fra Cloudflare.',
      hjDyk2: 'Klik på en værdi for at lægge et filter på. Filtrene står under overskriften: × fjerner ét, og Nulstil fjerner dem alle. Alle perioder er hele dage i UTC; i dag står for sig, indtil dagen er slut.',
      rutine: 'Rutine', nAfvist: '{n} afvist', lokketInd: 'lokket ind', faelderSat: '{n} {n|fælde-link|fælde-links} sat ind',
      faelderLinje: n => flertal(n, '1 side blev leveret med skjulte fælde-links til robotter', `${n} sider blev leveret med skjulte fælde-links til robotter`),
      dykFiltre: 'Filtre', dykFjern: 'Fjern filtret {navn}', 
       dykHenter: 'Henter tallene …', dykAabn: '{navn}: {n}. Åbn.', dykAabnTekst: '{navn}. Åbn.', dykRul: '{navn}: {n}. Gå til listen.', dykSkift: '{navn}: {n}. Skift domæne.', dykAabnRegelIDag: 'Åbn {navn}: {n} i dag hidtil',
      dykPeriode: 'Periode', dykP7: '7 dage', dykP30: '30 dage', dykPAlt: 'Hele historikken', dykSlagsH: 'Vis', dykSAlt: 'Alt', dykSAngreb: 'Kun angreb ({n})', dykSRutine: 'Kun rutine ({n})',
      dykAfvist: 'Afvist', dykAngrebOrd: 'Angreb', dykHaendelser: 'Hændelser', dykF30: '{o} på 30 dage', dykF30Sub: 'hele dage til og med i går', dykF7: '{o} på 7 dage', dykFAlt: '{o} i hele historikken', dykFIGaar: '{o} i går', dykFIDag: 'i dag hidtil: {n}', dykFDag: '{o} den {d}', dykFPeriode: '{o} {a} – {b}',
      dykFoerst: 'Først set', dykSidst: 'Sidst set', dykEdge: 'Edge · {p}', dykEdgeP: 'Antal pr. dag (UTC). Klik på en dag for at se netop den.', dykDag: '{d}: {n}. Åbn dagen.', dykIkkeHentet: '{d}: ingen data — Huginn var slukket',
      dykBue: '{a} til {b}: {n}. Åbn.',
      dykListerH: 'Fordelt på', dykListerP: 'Klik for at lægge et filter mere på.', dykFlere: 'Vis top 20 ({n})', listeAlle: 'Vis alle ({n})', tastatur: 'Piletaster flytter, Enter åbner.',
      kortUdenPlads: 'Uden plads på kortet: {liste}. De står i listerne.', kortBuer: '{n|Buen er den største|Buerne er de {n} største} af {m}.', kortFaaLande: '+ {n} {n|land|lande} med få angreb står ikke på kortet.', kortFaaColos: '+ {n} {n|datacenter|datacentre} med få angreb står ikke på kortet.',
      dykLister: { land: 'Lande', netvaerk: 'Netværk', sti: 'Stier', colo: 'Datacentre', metode: 'Metoder', handling: 'Handling', robot: 'Robotter', status: 'Svar' },
      dykHandling: { blokeret: 'Blokeret', udfordret: 'Udfordret', labyrint: 'Lokket ind i labyrinten', faelder: 'Sider med fælde-links', andet: 'Andet' },
      dykAndreRegel: 'Samme regel på dine andre domæner', dykAndre: '{navn} på dine andre domæner', dykAndreAlt: 'På dine andre domæner',
      kortSkala: 'Landet, Cloudflare så angrebet komme fra (kan være en VPN eller proxy)', 
      dykSenesteH: 'De seneste, der blev afvist', dykTid: 'Tidspunkt (UTC, hel time)', dykDomaene: 'Domæne', dykRegel: 'Regel', dykLand: 'Land', dykSti: 'Sti', dykColo: 'Datacenter', dykMetode: 'Metode',
      dykSenesteP: 'Enkelte hændelser findes kun for de 12 seneste pr. dag.', dykTomt: 'Ingen tal i perioden.', dykNote: 'Tal pr. dag i UTC, fra de tal, der er gemt på din computer. Tidspunkt på dagen og IP-adresser gemmes ikke. Tallene er Cloudflares estimat ud fra en stikprøve.', dykVaegtGraense: 'Før {d} er tallene talte rækker, derefter Cloudflares estimat.',
      dykLoft: 'Mindst én dag i perioden står på Cloudflares grænse på 10.000 hændelser, så de ældste timer den dag mangler. Der er mindst {n}.', dykCloudflareNavn: 'Cloudflares navn:', dykNetNote: 'Kun netværkets navn er kendt, ikke dets nummer, så to netværk med samme navn står sammen.', dykForsoegNote: 'Spørgsmål efter filer, der ikke fandtes (svar 404). De tælles for sig og lægges ikke sammen med det afviste.',
      dykTor: 'Tor-netværket', dykUkendtLand: 'Ukendt land',
      kat: {
        alle: ['Alle stier med 404', 'Alle de adresser, der blev prøvet og ikke fandtes.'],
        hemmeligheder: ['Hemmeligheder', 'Filer med adgangskoder og nøgler, som nogen har glemt at skjule (.env, appsettings).'],
        kildekode: ['Kildekode', 'Git-mapper og backupfiler, som kan afsløre et sites kode.'],
        wordpress: ['WordPress', 'Login- og adminsider på WordPress.'],
        php: ['PHP-værktøjer', 'Fejlfindingssider som phpinfo, der viser serverens opsætning.'],
        adgang: ['Kontrolpaneler', 'Admin-, status- og styringssider til andre systemer.'],
        andet: ['Andet', 'Alt det andet, der blev spurgt efter og ikke fandtes.'],
      },
      type: [[/remote code execution|\brce\b/i, 'Fjernkørsel af kode: få serveren til at køre angriberens egen kode'], [/sql injection|sqli/i, 'SQL-injektion: snige databasekommandoer ind ad bagdøren'], [/broken access control/i, 'Brudt adgangskontrol: nå filer eller funktioner uden lov'], [/file inclusion|lfi|path traversal/i, 'Filinklusion: få serveren til at åbne en fil, den ikke burde'], [/xss|cross.site/i, 'Cross-site scripting: plante kode, der kører i andres browser'], [/anomal/i, 'Usædvanlig adfærd, der ligner et angreb'], [/browser integrity/i, 'Lignede ikke en rigtig browser (manglende eller falske headere)']],
      // Indstillinger
      indstH: 'Indstillinger', autostart: 'Start ved login', glem: 'Glem API Token…',
      grpGenerelt: 'Generelt', grpForbindelse: 'Forbindelse', grpData: 'Data', grpOm: 'Om',
      /* Popuppen Indstillinger, dialogerne Domæner og Om og guiden ved første start (V-INDST.1) */
      luk: 'Luk', omKnap: 'Om Huginn and Muninn', omH: 'Om Huginn and Muninn', grpUdseende: 'Udseende og sprog', grpDato: 'Dato og land', grpStart: 'Start', grpFoerste: 'Første start',
      guideIgenH: 'Guiden', guideIgenP: 'De fire trin fra første start: velkomsten, dit API Token, dine domæner og besked.', guideIgen: 'Vis guiden igen',
      guideTrin1: 'Velkommen', guideTrin2: 'Dit API Token', guideTrin3: 'Dine domæner', guideTrin4: 'Besked',
      guideH1: 'Velkommen til Huginn and Muninn', guideH2: 'Forbind med Cloudflare', guideH3: 'Vælg dine domæner', guideH4: 'Besked og start',
      guideP1: 'Fire trin, så er du i gang. Du kan springe over og åbne guiden igen under Indstillinger.',
      guideP2: 'Indsæt et API Token, der kun kan læse. Det gemmes i din computers nøglering (macOS Nøglering eller Windows Credential Manager) og forlader aldrig programmets Rust-del. Webvisningen kan ikke hente det tilbage.',
      guideP3: 'Vælg de domæner, Huginn skal hente og vise. Du kan ændre det senere på Overblik under Vælg domæner.',
      guideP4: 'To valg, så er du færdig.',
      baandBesked: 'Notifikationer', gBesked: 'Besked', beskedH: 'Notifikationer pr. domæne', domBeskedP: 'Hvornår du vil have besked om hvert valgt domæne. »Følg standard« bruger valget under Indstillinger › Notifikationer.',
      guideNaeste: 'Næste', guideTilbage: 'Tilbage', guideSlut: 'Færdig', guideSpring: 'Spring over',
      guideTokenFoerst: 'Gem et API Token, der virker, før du går videre.', guideDomVenter: 'Dine domæner hentes hos Cloudflare …',
      opsaetTomH: 'Der er ikke gemt et API Token', opsaetTomP: 'Guiden viser, hvordan du opretter et API Token hos Cloudflare, der kun kan læse, og forbinder programmet.', opsaetTomKnap: 'Åbn guiden',
      baandTilAngreb: 'Tilbage til Angreb', tekNulstilIntet: 'Der er ingen valg at nulstille.',
      tema: 'Udseende', temaNote: 'Følger systemet, eller vælg selv.', temaSystem: 'System', temaLys: 'Lys', temaMoerk: 'Mørk',
      tekst: 'Tekststørrelse', tekstNote: 'Gælder teksten i hele programmet.', tekstLille: 'Lille', tekstMellem: 'Normal', tekstStor: 'Stor',
      tekstStoerre: 'Større tekst', tekstMindre: 'Mindre tekst', tekstNormal: 'Normal tekst',
      autostartKort: 'Programmet starter i menulinjen og henter i baggrunden. Huginn starter også selv én gang om dagen og henter, hvis det ikke kører. Slå det fra, før du sletter programmet.',
      forbStatus: 'Status', forbNote: 'API Tokenet ligger i din computers nøglering.', forbNoteMac: 'API Tokenet ligger i macOS Nøglering.',
      forbOk: 'Forbundet · {n}', forbAfvist: 'Afvist', forbIngen: 'Intet API Token',
      forbNoegle: 'API Token til Cloudflare', forbNoegleNote: 'Kan kun læse, aldrig ændre noget.', skift: 'Skift API Token…', annuller: 'Annullér',
      glemH: 'Glem API Tokenet?', glemP: 'Programmet holder op med at hente, og dine valg pr. domæne slettes. Historikken bliver.', glemJa: 'Glem',
      /* Flere forbindelser (V-KONTI.1): én forbindelse er ét API Token og et navn. Det viste navn følger agent::vist_navn (D5). */
      grpForbindelser: 'Forbindelser', forbTilfoej: 'Tilføj API Token…', forbOmdoeb: 'Omdøb…', forbOmdoebH: 'Omdøb forbindelsen', forbNavnFelt: 'Navn',
      forbNavnNote: 'Tomt felt: navnet fra Cloudflare.', forbNavnGem: 'Gem', forbNavnGemt: 'Navnet er gemt.', forbStandard: 'Forbindelse {n}', forbFlereKonti: '{navn} + {n} {n|anden|andre}',
      forbForbundet: 'Forbundet', forbDomaenerAria: 'Vis domænerne for {navn}', forbOmdoebAria: 'Omdøb {navn}', forbSkiftAria: 'Skift API Token for {navn}',
      forbGlem: 'Glem…', forbGlemAria: 'Glem {navn}', forbListe: 'Dine forbindelser til Cloudflare', forbFlereH: 'Flere Cloudflare-konti', forbFlereNote: 'Ét API Token pr. konto. Domænerne vises samlet.',
      forbSkiftNote: 'Har du en konto mere? Brug Tilføj API Token… i stedet. Skift erstatter API Tokenet på denne forbindelse.',
      glemForbH: 'Glem forbindelsen «{navn}»?', glemForbP: 'Programmet holder op med at hente dens domæner, og dine valg for dem slettes. Historikken bliver.',
      afvistForb: 'Cloudflare afviste API Tokenet «{navn}». Dens domæner hentes ikke.', afvistFlere: 'Cloudflare afviste {n} af {m} API Tokens.', pilleAfvist: '{n|1 API Token|{n} API Tokens} afvist',
      domGruppeAlle: 'Vælg alle', domGruppeIngen: 'Ingen', domGruppeAlleAria: 'Vælg alle domæner for {navn}', domGruppeIngenAria: 'Fravælg alle domæner for {navn}',
      domaenerSerFlere: '{m} API Tokens ser {n} {n|domæne|domæner} i alt',
      guideFlere: 'Har du flere Cloudflare-konti? Tilføj dem bagefter under Indstillinger → Forbindelser.',
      /* V-KONTI.1 FIX 1: et delt domæne står under hver forbindelse, der ser det (F2); ens navne (F1). */
      domHentesMed: 'Hentes med «{navn}»', domIngenSer: 'Dette API Token ser ingen domæner.',
      forbNavnEns: 'To forbindelser hedder det samme. Giv denne et navn, så du kan kende dem fra hinanden.',
      dataDb: 'Lokal database', dataHist: 'Historik', dataHistNote: 'Programmet gemmer al historik på din computer og sletter intet af sig selv.', dataHistNoteMac: 'Programmet gemmer al historik på din Mac og sletter intet af sig selv.',
      dataDomaener: 'Domæner med tal', dataAeldste: 'Ældste dag', dataHuller: 'Huller', dataHullerN: '{n} {n|dag|dage}', dataNafM: '{n} af {m}', dataHent: 'Hent nye tal nu', dataIngen: 'Ingen endnu',
      nulstil: 'Nulstil historik…', nulstilNote: 'Sletter alle tal på din computer og henter dem forfra. Cloudflare har kun sikkerhedshændelser for den seneste måned og trafiktal for det seneste år — alt ældre er væk for altid.', nulstilNoteMac: 'Sletter alle tal på din Mac og henter dem forfra. Cloudflare har kun sikkerhedshændelser for den seneste måned og trafiktal for det seneste år — alt ældre er væk for altid.', nulstilH: 'Nulstil historikken?', nulstilJa: 'Slet og hent forfra',
      nulstilIntetTab: 'Alt kan hentes igen fra Cloudflare.', nulstilt: 'Historikken er nulstillet. Programmet henter tallene forfra.',
      autostartNote: 'Programmet starter i menulinjen, når du logger ind, og henter dine tal i baggrunden. Huginn starter også selv én gang om dagen og henter, hvis det ikke kører. Det bliver i menulinjen, også når vinduet er lukket; vælg Afslut i dets menu for at lukke det helt. Slå Start ved login fra her, før du sletter programmet.',
      autostartTil: 'Programmet starter ved login.', autostartFra: 'Programmet starter ikke ved login.', autostartHul: 'Så henter Huginn kun, når programmet er åbent. Åbn det mindst hver tredje uge, ellers kommer der huller i historikken.',
      niveauH: 'Notifikationer', niveauSp: 'Hvornår vil du have besked?', niveauSpNote: 'Beskeden kommer fra din egen computer, aldrig fra en server.',
      opsaetNiveauP: 'Hvornår vil du have besked? Du kan ændre det senere under Indstillinger (tandhjulet øverst til højre).',
      niveauRolig: 'Kun alvorligt, højst én gang om dagen', niveauStandard: 'Ved alvorligt (anbefalet)', niveauAlt: 'Også når et domæne skal holdes øje med',
      notifSystem: 'Du kan også slå notifikationerne helt fra i Systemindstillinger → Notifikationer → Huginn and Muninn.',
      notifRoed: 'Giv besked, når et domæne kræver handling', prDomaene: 'Pr. domæne', prDomaeneTom: 'Domænerne står her, når zonelisten er hentet.',
      prDomaeneFor: 'Notifikationer for {navn}',
      zv: { standard: 'Følg standard', alt: 'Alvorligt og hold øje', roed: 'Kun alvorligt', fra: 'Aldrig' },
      notifNote: 'Handl betyder usædvanligt mange angreb i går i forhold til ugen før. Cloudflares rutine (labyrinten, Bot Fight Mode og browsertjekket) og gamle tal giver aldrig besked, og hvert domæne meldes højst én gang om dagen. Klik på en notifikation, eller vælg Siden sidst i menulinjen, for at åbne Siden sidst. Du kan også slå notifikationerne helt fra i Systemindstillinger → Notifikationer → Huginn and Muninn.',
      notifGemt: 'Notifikationerne er gemt.',
    },
    en: {
      skip: 'Skip to content', under: 'for Cloudflare®', sprog: 'Language', sprogAuto: 'System language', appNavn: 'Wi24rd Huginn and Muninn',
      titel: { teknologi: 'Technology by domain', overblik: 'Overview', 'siden-sidst': 'Since last time', vaernet: 'Attacks · {domaene}', robotter: 'Robots · {domaene}', login: 'Login · {domaene}', land: '{land}', besoegende: 'Visitors · {domaene}', indstillinger: 'Settings', opsaetning: 'Connect to Cloudflare' },
      navH: 'Views', navUdenToken: 'Needs an API Token.', traeH: 'Domains', traeAria: 'Choose domain', traeAlle: 'All domains', traeTom: 'No domains are chosen yet.', baandAria: 'Commands', gData: 'Data', gLand: 'Country', gHjaelp: 'Help', gProgram: 'App', baandMere: 'More', gVis: 'Show', gDomaener: 'Domains', gPeriode: 'Period', gDatoer: 'Own dates', gFiltre: 'Filters', gTeknologi: 'Technology', gStier: 'Paths', baandVaelgDom: 'Choose domains', baandRetTek: 'Edit technologies', baandRobotAngreb: 'Attacks from robots', baandNulstil: 'Reset', baandNulstilIntet: 'There are no filters to reset.', vejledningKort: 'Guide', landSoeg: 'Search for a country', landSoegPh: 'Search', landIngen: 'No country matches.', fodLokalt: 'Only on this machine', navOverblik: 'Overview', navSiden: 'Since last time', navVaernet: 'Attacks', navIndst: 'Settings',
      navRobotter: 'Robots', navLogin: 'Login', navLand: 'Country now',
      maalMin: 'Huginn can only say “Looks targeted” when you have at least 3 domains chosen to compare with.', moensterForeloebig: 'Preliminary — decided when the next day is fetched.', maerkeForeloebig: 'preliminary',
      vurdSpoerg: 'Why did the day (not) get a label?', vurdIntro: 'Each condition with the figure that decides the mark, for {d}.', vurdJa: 'Met', vurdNej: 'Not met', vurdIkke: 'Does not count', vurdStatusJa: '{m}: yes', vurdStatusForeloebig: '{m}: yes, preliminary — decided when the next day is fetched', vurdStatusNej: '{m}: no',
      vurd: { dns_nx: 'Lookups of names that do not exist: {n} (must be at least {m})', dns_navne: 'Different names that do not exist: {n} (must be at least {m})', dns_navneEks: 'Different names that do not exist: {n} (must be at least {m}), e.g. {navn}', stigning: 'Rise: {n} {n|attack|attacks} against an average of {m}', netvaerk: 'One network accounts for at least {m}%: {navn} {n}%', netvaerkIngen: 'One network accounts for at least {m}%: none', net_andre: 'The network almost only here: {n} on other chosen domains ±1 day against {m} here', gule: 'At most {m} other {m|domain|domains} with a rise ±1 day: {n}', valgte: 'At least {m} chosen domains with figures: {n}', radar: 'The world did not rise: {n} (must be under {m})', radarIngen: 'The world did not rise: Radar has no figures for the day', efter: 'The next day is fully fetched', efterNej: 'The next day is not fully fetched', stier: 'At least {m} different paths with 404: {n}', faktor: 'At least 3 times normal: {n} (the limit is {m})', normalNul: 'Normal: 0 (no comparison)', dns_nxUden: 'Lookups of names that do not exist: {n}. Normal: too few days (no comparison)', dns_navneUden: 'Different names that do not exist: {n}. Normal: too few days (no comparison)' }, vurdAabnDag: 'Open the day’s attacks', vurdAabnNet: 'Open the attacks from {navn}', vurdAabnDomaene: 'Open {navn}', vurdAabnValgte: 'Open your chosen domains', vurdAabnVerden: 'Open World', vurdAabnStier: 'Open the paths',
      hvDmarcNy: 'Mail in your domain’s name failed DMARC from a new sender: {l}.', hvDmarcAndel: '{p}% of mail in your domain’s name failed DMARC (usually {m}%).', hvIGaarOrd: 'yesterday', hvForgaarsOrd: 'the day before yesterday', hvLaek: '{n} login {n|attempt|attempts} used a password from a known leak {d}.', hvLaekNaaet: '{n} login {n|attempt|attempts} used a password from a known leak {d}, and {k} of them reached your server and got an answer that can look like a login.',
      hvTilstand: { pause: 'The domain is paused — Cloudflare is not protecting it.', inaktiv: 'The domain is not active at Cloudflare (status: {s}).', phishing: 'Cloudflare suspects phishing on the domain.', udvikler: 'Development Mode is on — the cache is off until {k}.' }, gTilstandH: { pause: 'What do I do when the domain is paused?', inaktiv: 'What do I do when the domain is not active?', phishing: 'What do I do about suspected phishing?', udvikler: 'What do I do about Development Mode?' }, gLaekH: 'What do I do about leaked passwords?', hvSti200: 'A sensitive file ({s}) answered 200 {d} – it may be exposed.', hvSti200Flere: 'Sensitive files ({s}) answered 200 {d} – they may be exposed.', hvCfFund: 'Cloudflare has a critical security finding for the domain.', hvCfFundFlere: 'Cloudflare has {n} critical security findings for the domain.', gSti200H: 'What do I do about a sensitive file that answered 200?', gCfFundH: 'What are Cloudflare’s own findings?',
      dykKunTrafik: 'Cloudflare keeps security events for a month — before {d} there are only traffic figures.', dykTrafikDage: 'Requests day by day', dykTrafikDageP: 'From Cloudflare’s daily figures. Each day opens that day’s traffic.', dykTrafikSoejle: 'Requests per day', dykTrafikHul: '{d}: no figures from Cloudflare', trafikKunAar: 'The day is older than the security events: the figures are Cloudflare’s daily figures, and “threats” is Cloudflare’s own count.', trafikTrusler: 'Threats (Cloudflare’s daily figures)', trafikCachet: 'From Cloudflare’s cache', dataKilder: 'What is available', dataKilderTekst: 'Traffic from {a} · security events from {b}',
      loginLaekH: 'Passwords from known leaks', loginLaekP: 'Cloudflare compares the password in login attempts with known leaks. A match only says that the password is in a leak, not that it was a real user or that anyone got in. Neither Cloudflare nor Huginn stores the password.', loginLaekIngen: 'Passwords from known leaks: no attempts in the period.', loginLaekVenter: 'The check has not been fetched for the period yet.', loginLaekLinje: '{n} login {n|attempt|attempts} used a password from a known leak', loginLaekThDag: 'Day', loginLaekThSti: 'Path', loginLaekThN: 'Attempts', loginLaekThNaaet: 'Reached the server', loginLaekAndet: 'Other answers from the check: {l}.', laekVaerdi: { username_leaked: 'only the username was in a leak', username_password_similar: 'a similar username and password', ukendt: 'unknown' }, loginLaekNone: 'Not a login attempt (no password sent): {n}', loginLaekUdenSvar: 'No answer from the check: {n}.', loginLaekAabn: 'Show only {sti}', loginLaekDag: 'Show only {d}',
      hentHenter: 'Fetching …', hentFaerdig: 'Fetched at {t} — {n} updated', hentAlt: 'All fetched — nothing new since {t}', hentAltUden: 'All fetched.', hentAllerede: 'Already fetching ({a} of {b}) …', hentFejl: 'Could not fetch: {f}',
      loginLaekSlaaetTil: 'The check is on by default on the Free plan. On a paid plan it must be turned on in Cloudflare under Security → Settings, otherwise nothing shows here.',
      nulstilSikkerhed: 'You have security figures from {fra}. After a reset, only figures from {g} can be fetched again. {n} {n|day is|days are} lost for good.', nulstilTrafik: 'You have traffic figures from {fra}. After a reset, only figures from {g} can be fetched again. {n} {n|day is|days are} lost for good.', nulstilUkendt: 'Anything older than what Cloudflare still has is lost for good.',
      dataTrafikHentes: 'Traffic figures are being fetched … · security events from {b}', dataKvoteH: 'Quota at Cloudflare', dataKvote: 'Cloudflare allows {n} {n|call|calls} per {m} {m|minute|minutes}; Huginn used {k} in the last round.',
      tekRet: 'Edit technologies', tekRetAabn: 'Edit the technologies for {navn}', tekIngenValg: 'You have not said what the site uses.', tekChipBruger: 'Uses: {l}', tekChipIkke: 'Does not use: {l}', tekForslagKort: 'All {n} {t} {n|attempt|attempts} got 404 —', tekAndet: 'Other',
      emailH: 'Mail sent in your domain’s name', emailP: '{p}, from the reports receivers send to Cloudflare. The reports run a day or more behind.', emailLinje: '{n} {n|email|emails} · {m} not approved from {k} {k|sender|senders}', emailLinjeOk: '{n} {n|email|emails} · all approved', emailAfsender: 'Sender', emailLand: 'Country', emailFejlet: 'Failed DMARC', emailMails: 'All mail', emailUkendt: 'Unknown sender', emailNote: 'Worked out from DKIM and SPF: a message passes DMARC if one of them passes. The list only shows senders that failed. A sender you use yourself can fail because it has not been set up yet.', emailTom: 'Cloudflare has no DMARC reports for the domain — DMARC Management may not be turned on.', emailVenter: 'The DMARC figures have not been fetched yet.', emailFejl: 'Cloudflare did not answer for the DMARC figures: {f}', emailGuide: 'How to turn on DMARC Management', sti200H: 'Sensitive files that answered 200', sti200P: '{p}: {n} {n|path|paths} that should never answer 200, for example /.env and /.git/config. 200 does not always mean the file exists: many sites answer with their ordinary page at every address.', sti200Ingen: 'None of the paths on the list ({n}) answered 200 in the period.', sti200Venter: 'The figures have not been fetched for the period yet.', sti200Fil: 'Looks like the file', sti200Side: 'Looks like the site’s ordinary page', sti200ThSti: 'Path', sti200ThN: '200 answers', sti200ThSnit: 'Average per answer', sti200ThVurd: 'Assessment', sti200Aabn: 'Show attacks on {sti}', sti200Note: 'An answer looks like the file when it is at most 8 KB, more than 25% away from the front page’s size ({f}), and the domain does not answer 200 on more than 6 of the paths. Only then does the domain show Act. The figures only exist for days fetched after the update of 8 November 2026.', sti200UdenForside: 'unknown', sti200Guide: 'What do I do?', cfH: 'Cloudflare’s own findings', cfP: 'Cloudflare scans the domain’s setup itself (Security Insights: every 7 days on Free, every 3 on Pro and Business). A click opens the findings at Cloudflare.', cfIngen: 'Cloudflare has no active findings for the domain.', cfVenter: 'The findings have not been fetched yet.', cfManglerRet: 'Requires one more permission on the API Token, as an account permission: Account Security Center Insights › Read in a new policy (Add new policy › Add new Account policy). Zone Security Center Insights does not work. It is optional: the rest of the program works without it.', cfFejl: 'Cloudflare did not answer about the findings ({f}). The most recently fetched are shown.', cfAlvor: { critical: 'Critical', high: 'High', moderate: 'Moderate', low: 'Low' }, cfKlasse: { security_txt_not_enabled: 'security.txt is not set up', always_use_https_not_enabled: 'Always Use HTTPS is not turned on' }, cfAabn: 'Open the finding at Cloudflare: {t}', cfSiden: 'since {d}', cfHentet: 'Fetched {t}.', rettetH: 'What the attacks targeted', rettetP: 'Every technology attacked in the period. Your choices only change status and warnings, never this list.', rettetUfarlig: 'harmless to you', rettetBlandet: 'Differs by domain', rettetAabn: '{n} {n|attack|attacks} on {t}: open them',
      tekKolMenu: 'Actions for {t}', tekRkMenu: 'Actions for {navn}', tekSideH: 'Technology by domain', tekFanePr: 'By domain', tekFaneAlle: 'All technologies', tekFaner: 'View', tekSoeg: 'Search for a technology', tekForkl: '✓ Uses · ✕ Does not use · ? Don’t know. A lighter cell has not been seen on the domain but can still be set. Numbers open the attacks.', tekKortBruger: 'Uses', tekKortIkke: 'Does not use', tekIkkeAlle: 'Not used anywhere', tekIkkeAlleFor: '{t}: not used on any domain', tekNulKolonne: 'Reset column', tekNulKolonneFor: 'Reset {t} on all domains', tekNulDomaene: 'Reset domain', tekNulDomaeneFor: 'Reset all choices for {navn}', tekKopier: 'Use on all domains', tekKopierFor: 'Use the choices for {navn} on all domains', tekCelleAntal: '{n} {n|attack|attacks} on {t} on {navn}: open them', tekIngenKolonner: 'No technologies are shown in the table. Choose them under All technologies.', tekIngenTraef: 'No technology matches the search.', tekNy: 'New technology seen: {t} — show in the table?', tekNyJa: 'Yes', tekNyNej: 'No', tekNyJaFor: 'Show {t} in the table', tekNyNejFor: 'Do not show {t} in the table', tekVis: 'Show in table', tekVisFor: 'Show {t} in the table', tekKontoAntal: '{n} {n|attack|attacks} on the account', tekIkkeSetKonto: 'Not seen', tekTilfoej: 'Add technology', tekTilfoejKnap: 'Add', tekTilfoejNote: 'Pick a name from the list or type your own, so you can say “Does not use” before the first attack.', tekTilfoejet: '{t} has been added.', tekTilfoejFejl: 'That name cannot be used. It may be at most 40 characters and may not be a type of attack.', tekGruppeNote: 'If you hide a technology here, the domains’ choices still apply, and the attacks still stand in figures, lists and on the map.', tekIndeGruppe: 'The arrow keys now choose within the cell. Esc goes back to the table.', tekGemt: 'Saved.', tekGruppe: { cms: 'CMS', ramme: 'Web frameworks', server: 'Servers and languages', samarbejde: 'E-mail and collaboration', netvaerk: 'Network and VPN', andet: 'Other' },
      domTekAabn: 'Technology by domain →', domTekUden: '{n} without choices', domTekValg: '{navn}: {n} {n|choice|choices}',
      hvMere: 'more', hvMindre: 'less', hvIkkeBrugtOrd: 'which you do not use', hvIkkeBrugtAabn: 'See and undo the technology choices for {navn}',
      sidenUtc: 'Days in UTC, shown in your time.', sidenVistNat: 'Status for the day before yesterday. You last looked here {t}.', sidenFoersteNat: 'Status for the day before yesterday for all your domains.',
      dykVisNafM: 'Showing {n} of {m} {m|attack|attacks}.', dykLoft200: 'Huginn keeps the 200 latest attacks per day.', dykForrige: '‹ Previous', dykNaeste: 'Next ›', dykSideN: 'Page {a} of {b}', dykSiderH: 'Pages', dykSideAabn: 'Show page {a} of {b} of the latest',
      
      domTekH: 'Technology by domain', domTekAlle: 'Reset for all domains', 
      bekraeftTekH: 'Reset technology choices?', bekraeftTekP: 'All choices for {d} are set to “Don’t know”. The status is worked out again, as before you chose.', bekraeftTekAlleP: 'All choices on all domains are set to “Don’t know”. The status is worked out again, as before you chose.', bekraeftJa: 'Reset', tekNulstillet: 'The technology choices have been reset.',
      datoFormat: 'Date format', datoFormatNote: 'Applies to all dates in the program and the notifications.', datoFormatSprog: 'Follow the language', datoFormatSystem: 'Follow the system', datoFormatIso: 'ISO (2026-11-04)',
      kalForrige: 'Previous month', kalNaeste: 'Next month', kalAabn: '{felt}: {d}. Choose another date', kalAabnTom: '{felt}: choose a date',
      dykKunIDag: '{navn} has only been seen today ({n} {o} so far). Whole days are counted from tomorrow.', dykKunIDagHer: 'This', dykKunStier: 'Paths', dykKunLande: 'Countries', dykKunTid: 'Time (UTC, whole hour)', dykKunAabn: 'See today’s figures',
      dykHeleIDag: '{h} whole days · {n} today', dykForesp: 'Requests in total', dykForespSub: '{o}: {n} — 1 in every {m}', dykForespAabn: 'Requests in total: {n}. Open the traffic',
      dykTrafikLinje: '{t} {p}: {n} {n|request|requests} (normally {m} a day). {stop}', dykSeTrafikken: 'See the traffic →',
      autostartBanner: 'The program does not start at login.', autostartSlaaTil: 'Turn on Start at login', autostartHulKort: 'Turned off: gaps may appear in the history.',
      maerkeRekog: 'Looks like reconnaissance', maerkeRekogKort: 'Recon', maerkeDnsKort: 'DNS recon', maerkeDns: 'Looks like DNS reconnaissance', maerkeMaal: 'Looks targeted', maerkeAabn: '{m} on {navn} on {d}: open the day', maerkeGuide: 'What does it mean? →',
      moensterRekog: 'Looks like reconnaissance: {n} different paths in one day (usually {m}){s}{t}.', moensterDns: 'Looks like DNS reconnaissance: {n} {n|lookup|lookups} of non-existent names in one day (usually {m}).', moensterDnsNavne: 'Looks like DNS reconnaissance: {n} different non-existent {n|name|names} in one day (usually {m}).', moensterScanner: ', like an ordinary scanner',
      moensterMaal: 'Looks targeted: attacks rose here but not {v}, and {p}% came from one network{t}.', moensterVerden: 'elsewhere', moensterAndre: 'on your other domains', moensterTek: ', and the attacks hit {t}, which your site uses',
      gMaalH: 'What does “Looks targeted” mean?', gRekogH: 'What is reconnaissance?', gDnsRekogH: 'What is DNS reconnaissance?', gDmarcH: 'Someone is sending mail in your domain’s name',
      notifRekog: 'Notify me of “Looks like reconnaissance”', notifRekogNote: 'Off by default: most are automated scanners. Also applies to “Looks like DNS reconnaissance”. The label always shows in Overview and Since last time.',
      tokenAabn: 'Create API Token at Cloudflare', tokenAabnNote: 'The button opens Cloudflare’s page with the name and the two permissions filled in. Check them, and create the API Token there.', tokenItFirma: 'Did your IT company give you an API Token? Paste it here.', tokenSeTrin: 'See the steps',
      tokenTrin: ['The button above opens Create Token at Cloudflare with the name and permissions filled in. Log in first if Cloudflare asks you to.', 'Check under Policies that DNS & Zones shows 1 selected (Zone › Read) and Analytics & Logs 1 selected (Analytics › Read). All other groups must show 0, and nothing may be set to Edit: the program only reads.', 'Optional: if you want to see Cloudflare’s own security findings under Attacks, choose Add new policy › Add new Account policy. Leave Include · All accounts as it is, search for Security Center, and choose Account Security Center Insights › Read. The permission belongs to the account: Zone Security Center Insights in the first policy does not work. The program works without it.', 'Leave Include · All zones as it is. If the API Token should see only one domain, choose Specific zone and the domain.', 'Leave Token expiration on No expiration, and leave the IP filter (Client IP address filtering) empty.', 'Choose Continue to summary. Only Analytics: Read and Zone: Read may be listed (and Account Security Center Insights: Read under All accounts, if you added it). Choose Create Token.', 'Copy the API Token and paste it into the field. Cloudflare shows it only once.'], tokenTrinNote: 'If you start from Create Token yourself, the fields are the same.',
      slogan: 'Security isn\'t something you buy. It\'s something you understand.',
      forventH: 'What to expect', velkomstLand: 'your country',
      forvent: [
        ['Huginn watches the security', ' of all your Cloudflare domains: attacks, robots and login attempts, on one screen with a plain-language verdict: {groen}, {gul} or {roed}.'],
        ['You are told when it matters:', ' notifications from your own computer when a domain needs action — you decide when.'],
        ['Dig into what happened:', ' click a number to see the rules, countries, networks, paths and robots behind it — and how {land} and the world are doing.'],
        ['Muninn remembers:', " the numbers are kept on your computer, beyond Cloudflare's month."],
        ['Read-only, no telemetry:', ' the program cannot change anything in Cloudflare, and nothing is sent to Wi24rd.'],
      ],
      filosofi: {
        h: 'Wi24rd — our philosophy',
        afsnit: [
          'We believe cybersecurity should be accessible to everyone — not just enterprises with expensive tools and dedicated security teams.',
          'No false promises. No unnecessary complexity. Just practical tools, clear insights, and the knowledge to make better security decisions.',
          'Built to empower. Not to create dependency.',
        ],
      },
      landLigeNu: '{land} now', landVaelg: 'Country', landVerdenFane: 'World', landFaner: 'Country or world', landRadar: 'Cloudflare Radar · the last 30 days', landVaelgP: 'Choose the country you want to compare with.', landForslag: 'Switch to {land}', landForslagNote: 'At least half of your visitors come from there.', landTilbage: 'Back to {land}', landUkendt: 'Choose a country …', landIndstNote: 'Used in {land} now.',
      landHenter: 'The figures from Cloudflare Radar are being fetched. The first time takes up to a minute.', landFejl: 'The figures from Cloudflare Radar are not up to date: {f} Your own figures still work.', landHentet: 'Fetched {t}',
      landKurveH: 'Your domains and the whole world', landKurveP: 'Both curves are converted so that 100 is the average of the period. They show rises and falls, not numbers.', landKurveTitel: 'Attacks as an index, 100 is the average of the period. Arrow keys move between the days, Enter opens the day.', landMit: 'My domains', landVerden: 'World, all attacks', landVerdenWaf: 'World, WAF', landAkse: '100 = average',
      landDag: '{d}: my domains {m} (index {i}), world index {v}. Open the day.', landDagUden: '{d}: my domains {m}, world index {v}', landWafFjern: 'Show all attacks in the world',
      landFoelgerH: 'Your domains follow the global trend', landFoelgerP: '{n} {n|attack|attacks} on {d} (average {s}). WAF attacks also rose in the rest of the world.', landDigH: 'The rise is yours', landDigP: '{n} {n|attack|attacks} on {d} (average {s}). The world was normal that day: WAF attacks did not rise in the rest of the world.', landTendensHvad: 'What does it mean? →',
      landAndelH: '{land} as a target', landAndelP: 'How large a part of the attacks Cloudflare saw in the world was aimed at the country. The target is the zone’s billing country.', landAndel: '{land} was the target of {p} of the attacks {d}.', landUdenfor: '{land} was not among the 100 most targeted countries {d}.', landAndelIngen: 'No figures for {land} yet.', landAndelDage: 'Day by day', landUdenforKort: 'not in the top 100', dagDen: 'on {d}',
      landModH: 'The largest sources of attacks on {land}', landModMineH: 'Where attacks on your domains came from (all countries)', landModBlokH: 'Where the attacks came from', landModP: 'On the left the largest sources of attacks on {land} {p}, as a share of all attacks on the country, where the target is the zone’s billing country. On the right attacks on the domains you have chosen, in the same period.', landMod7Tekst: 'in the last 7 days', landMod28Tekst: 'in the last 4 weeks', landModRest: 'Other countries (not given by Radar)', landModPeriode: 'Period', landMod7: '7 days', landMod28: '4 weeks', landModNote: 'The attacker’s country is determined by the IP address and can be due to a VPN or rented servers. That applies both to {land} and to your domains, so the comparison is fair, but the country does not say who the attacker is.', landModIngen: 'No figures for attacks on {land} yet.', landModIkkeOplyst: 'Cloudflare Radar does not say where the attacks on {land} came from (the country is too small in the statistics).', landSerieFejl: 'Cloudflare Radar gave no figures for this section at the last update. The rest of the page is not affected.', landMineIngen: 'No attacks on your domains in the period.', landSaerligt: 'Especially you',
      landAndelKurve: 'Share of all attacks Cloudflare saw, day by day', landAndelDag: '{d}: {p} of all attacks Cloudflare saw', landAndelDagUden: '{d}: not among the 100 most targeted',
      landUdfaldH: 'Internet disruptions in {land}', landUdfaldP: 'What Cloudflare Radar has recorded in the last 30 days.', landUdfaldIngen: 'No known disruptions in {land} in the last 30 days', landUdfaldVenter: 'The disruptions have not been fetched yet.', landUdfaldSeVerden: 'See disruptions in the world', landUdfaldVerdenH: 'Internet disruptions in the world', landUdfaldVerdenP: 'The latest Cloudflare Radar has recorded in the last 30 days.', landUdfaldVerdenIngen: 'No known disruptions in the world in the last 30 days', landUdfaldNet: 'Networks affected: {liste}', landUdfaldUdenSlut: 'no end date given', landUdfaldMin: '{n} {n|minute|minutes}', landUdfaldTimer: '{n} {n|hour|hours}', landUdfaldDage: '{n} {n|day|days}', landUdfaldUkendt: 'Unknown cause',
      landUdfaldAarsag: { BLOCKING: 'The internet was blocked', CABLE_CUT: 'A cable was cut', CYBERATTACK: 'A cyberattack', DNS: 'A DNS fault', FIRE: 'A fire', GOVERNMENT_DIRECTED: 'Ordered by the authorities', MAINTENANCE: 'Maintenance', MECHANICAL: 'A mechanical fault', MILITARY_ACTION: 'Military action', MISCONFIGURATION: 'A configuration fault', NATURAL_DISASTER: 'A natural disaster', NETWORK_PROBLEM: 'A network problem', POWER_OUTAGE: 'A power cut', SOFTWARE: 'A software fault', TECHNICAL_PROBLEM: 'A technical problem', UNKNOWN: 'Unknown cause', WEATHER: 'The weather' },
      landUdfaldSlags: { NATIONWIDE: 'the whole country', REGIONAL: 'a region', NETWORK: 'one network', PLATFORM: 'a platform' },
      landKvanteH: 'Quantum-safe connections', landKvanteSub: 'of HTTPS requests', landAiNavnH: 'AI robots by name', landRobotNote: 'Radar divides robots and connections by where the traffic comes from, not where the site is. That is why they stand here and not under a country.',
      landMaalH: 'Most targeted countries', landKildeH: 'Most attacks from', landProduktH: 'What stopped them', landBotH: 'Robot traffic', landBotSub: 'of all HTTP traffic', landHttpH: 'HTTP traffic', landHttpSub: 'index, 100 = 30-day average', landAiH: 'AI robots by purpose', landVerdenH: 'Attacks in the world', landVerdenP: 'Radar shows percentages and indexes, not numbers. The figures are for {d}.', landVerdenRobotH: 'Robots in the world', landTom: 'No figures from Radar yet.',
      landProdukt: { DDOS: 'DDoS protection', WAF: 'WAF', BOT_MANAGEMENT: 'Bot protection', ACCESS_RULES: 'Access rules', IP_REPUTATION: 'IP reputation', API_SHIELD: 'API Shield', DATA_LOSS_PREVENTION: 'Data Loss Prevention' },
      landFormaal: { 'Mixed Purpose': 'Mixed purpose', Training: 'Training', Search: 'Search', 'User Action': 'User action', Undeclared: 'Undeclared' },
      landKilde: 'Source: Cloudflare Radar, CC BY-NC 4.0', landKildeAabn: 'Open Cloudflare Radar (opens your browser)', hjLand: 'Here your own attacks stand next to the world from Cloudflare Radar. Radar shows percentages and indexes, not numbers, so the curves say something about rises and falls. Attacks are shown from the target’s side: where attacks on the country came from, next to where attacks on your domains came from. A source country alone cannot be trusted.',
      robotVerificeret: 'Verified ({k})', robotAssistent: 'AI assistant in a browser', robotUkendt: 'Unknown robot or browser',
      robotSlags: { alle: 'Known robots', udklaedt: 'In disguise', uverificeret: 'Unverified', stoppet: 'Stopped at the Edge', labyrint: 'Lured into the labyrinth' },
      robotGrupper: {
        'ai-traening': ['AI training', 'Collects text to train language models.', 'If you block them, your texts may avoid being used for training. Some robots are used for both training and search, so check first.'],
        'ai-soegning': ['AI search and assistants', 'Fetches a page because someone asked an AI.', 'If you block them, your site may be missing from AI answers.'],
        soegemaskiner: ['Search engines', 'Googlebot, Bingbot and others index your site.', 'If you block them, you may disappear from the search results.'],
        sociale: ['Social media and link previews', 'Fetches the title and image when a link is shared.', 'If you block Meta, for example, shared links may lose their image and text, and ads may be affected.'],
        seo: ['SEO tools', 'Ahrefs, Semrush and the like.', 'If you block them, those tools may lose data about your site.'],
        overvaagning: ['Monitoring', 'Uptime, security and accessibility.', 'If you block them, your own measurements may stop.'],
        ukendt: ['Unknown', 'Robots Cloudflare has not put in a group, and those that are not verified.', 'They may be new robots, or someone calling themselves something they are not.'],
      },
      robotBesoeg: 'Visits from known robots', robotAndel: 'Share of all traffic', robotUdklaedtSub: 'call themselves Googlebot, say, without being it', robotStoppetSub: 'challenged or refused by a rule', robotLabyrintSub: 'fake pages that waste the robot’s time',
      robotHvemH: 'Who reads your site', robotHvemP: 'The groups come from Cloudflare’s verified categories. You do any blocking yourself at Cloudflare: Huginn and Muninn only read.',
      robotMestAktive: 'Most active robots', robotMestLaeste: 'Most crawled pages', robotTomt: 'No robots in the period.',
      robotIkkeTilg: 'Stopped at the Edge is not available for: {d}. The rest of the view works.', ekstraMangler: 'The extra figures are being fetched in the background: {n} {n|day is|days are} still missing.',
      robotNote: 'Cloudflare may use samples, so the figures can be estimates. Figures per day in UTC, from the figures saved on your computer.', robotLabyrintNote: 'The labyrinth does not stop the robot. It shows fake pages so the robot wastes its time.',
      dykBesoeg: 'Visits', dykRobotDage: 'Per day · {p}', sidenNyRobot: 'New AI robot on {domaene}: {robot}',
      hjRobotter: 'Robots shows who reads your site and what may happen if you block them. You do any blocking yourself at Cloudflare.', hjLogin: 'Login shows what happened on the login paths you chose yourself. The three figures come from separate sources and are not added up.',
      loginStierH: 'Login paths per domain', loginStierP: 'Turn on the paths that are logins on your site. We do not guess what the site runs.', loginDineStier: 'Your login paths:', loginRet: 'Edit paths', loginFiltre: 'Showing only:',
      loginEgenH: 'Add your own login path', loginEgenEksempel: '/my-login/', loginEgenTilfoej: 'Add', loginEgenFjern: 'Remove', loginEgenFjernNavn: 'Remove {sti}', loginStiUgyldig: 'The path must start with / and cannot contain spaces or ?',
      loginForslagH: 'Suggestions from your data:', loginForslagTilfoej: 'Add {sti} as a login path', loginForslagNote: 'Suggested because the path got POST with response 200, 302, 401 or 403 on at least 3 different days.',
      loginIngenStier: 'No login paths are turned on for this domain. Choose them here and we will count the attempts.', loginVaelgDomaene: 'Paths are chosen per domain: choose a domain at the top to edit them.',
      loginIkkeTilg: '"Reached your server" is not available for: {d}. The Edge and 404 figures still work.',
      loginUdfald: { edge: 'Stopped at the Edge', ikke_fundet: 'Got 404', server: 'Reached your server', afvist: 'Refused by your server', svar: 'Response 200 or 302' },
      loginEdgeSub: 'never reached your server', loginIkkeFundetSub: 'the path does not exist', loginServerSub: 'your server answered', loginAfvistSub: 'response 401 or 403', loginSvarSub: 'may have worked, may be an error page',
      loginDag: '{d}: {e} stopped at the Edge, {f} got 404, {s} reached your server', loginSoejleTitel: 'Attempts per day, three bars per day. Arrow keys move between the days, Enter opens.', loginDageH: 'Attempts per day', loginDageP: 'Three figures from separate sources, side by side. They are not added up: they may be the same attempts.',
      loginTabelH: 'Paths and responses', loginCelle: '{sti}, {navn}: {n}. Open.', loginBetyder: 'What it means', loginKortEdge: 'Edge', loginKort404: '404', loginKortServer: 'Your server', loginKortAfvist: 'Refused', loginKortSvar: '200/302',
      loginSvarForkl: 'Response 200 or 302 does not say whether anyone got in. It may be the login page shown again.', loginForkl: { '/xmlrpc.php': 'Old remote access to WordPress. If you do not use it, you can close it.' },
      loginHvorH: 'Where the attempts came from', loginHvorP: 'Each source on its own. Networks from your server appear as a number if the name has not been seen at the Edge.', loginLandeEdge: 'Countries, stopped at the Edge', loginLandeServer: 'Countries, reached your server', loginNetEdge: 'Networks, stopped at the Edge', loginNetServer: 'Networks, reached your server', loginMetodeEdge: 'Methods, stopped at the Edge', loginMetodeServer: 'Methods, reached your server',
      loginNote: 'We cannot see IP addresses or who logged in, and the figures may be samples. Response 200 or 302 is not proof of a login. Your own login counts too, so do not show these figures in public.',
      hentNu: 'Fetch now', tilbage: 'Back', frem: 'Forward', hjaelpKnap: 'Help', hjaelpH: 'Help', hjaelpLuk: 'Close help', genvejeH: 'Shortcuts',
      sidenTal: n => flertal(n, '1 domain needs action', `${n} domains need action`),
      pilleHenter: 'Fetching {z} of {Z}', pilleOpdateret: 'Updated {t}', pilleOpdateretFejl: 'Updated {t} · {n} failed', pilleVenter: 'Waiting for Cloudflare',
      pillePause: 'Trying again at {kl}', pilleFejl: 'Fetching stopped', pilleAabn: '{t} Open the Overview.',
      hjOm: 'Version, licences and trademark are behind the About button (i) at the top right.',
      navBesoeg: 'Visitors',
      hjBesoeg: 'Here you see how many people visit your sites, and from where. Unique visitors go back a year. Visits per country go back a month at Cloudflare, and the program saves the days, so the history grows from there.',
      hjBesoeg2: 'A click on a tile chooses the period, a click on a day opens that day’s traffic on Attacks, and a click on a country, a robot group or a page shows its figures per day.',
      bes: {
        aar: 'One year', fIGaar: 'Unique visitors yesterday', f7: 'Unique visitors in 7 days', f30: 'Unique visitors in 30 days', fAar: 'Unique visitors in a year', fBesoeg30: 'Visits in 30 days',
        ingenTal: 'No figures yet', ikkeTilgKort: 'Not available', periodeAabn: '{navn}: show the period ({n})',
        grafH: 'Unique visitors per day', grafLandH: 'Visits from {land} per day', grafP: 'A click on a day opens that day’s traffic on Attacks.',
        hulNote: 'The chart covers {m} of {n} {n|day|days}. Days without figures are not 0: they have not been fetched.',
        landeH: 'Where the visits came from', landeP: 'Visits per country in the period. A click on a country shows the visits from that country per day.',
        thLand: 'Country', thBesoeg: 'Visits', thAndel: 'Share', visAlle: 'Show all {n} {n|country|countries}', landAabn: 'Show visits from {navn} per day ({n} {n|visit|visits})',
        landeFra: 'Visits per country exist from {d}. Cloudflare keeps them for a month, and the program saves the days from then on.',
        landeIngen: 'No visits in the period.', landeIkkeHentet: 'Visits per country have not been fetched yet. They are fetched in the background after the other figures.',
        ikkeTilg: 'Visits per country are not available for this domain at Cloudflare. The rest of the page works.', ikkeTilgFlere: 'Visits per country are not available at Cloudflare for {d}. The rest of the page works.',
        note: 'Unique visitors are unique IP addresses per day: robots count, and several people behind the same router count as one. A sum over several days counts the same visitor once per day. The country is the one Cloudflare saw and can be a VPN. Visits are Cloudflare’s estimate from a sample: a page view that came from another site or a direct link. Figures per day in UTC, from the figures saved on your computer.',
        noteSamlet: 'All domains are added together, so the same visitor on two domains counts twice.',
        fBesoegLand: 'Visits from {land} in 30 days', fBesoegSti: 'Visits to {sti} in 30 days', alleLande: 'all countries', alleSider: 'all pages',
        robotLinje: 'of which {n} from known robots ({p})',
        grafUgeH: 'Unique visitors per week', grafLandUgeH: 'Visits from {land} per week', grafStiH: 'Views of {sti} per day', grafStiUgeH: 'Views of {sti} per week',
        grafRobotH: 'Visits from {gruppe} per day', grafRobotUgeH: 'Visits from {gruppe} per week',
        grafUgeP: 'One bar per week, Monday to Sunday. A click on a week shows the week.',
        uge: 'The week from {a} to {b}: {n}', ugeDele: 'The week from {a} to {b}: {n} ({m} of 7 days)', ugeHul: 'The week from {a} to {b}: no figures', ugeAabn: 'Show the week from {a} to {b} ({n})',
        robotH: 'Known robots', robotP: 'Visits from robots Cloudflare has verified, in the period, in the groups of the Robots view. A click on a group shows its visits per day.',
        thGruppe: 'Group', gruppeAabn: 'Show visits from {navn} per day ({n} {n|visit|visits})',
        robotIngen: 'No visits from known robots in the period.', robotIkkeHentet: 'Known robots have not been fetched yet. They are fetched in the background after the visits.',
        robotIkkeTilg: 'Known robots are not available at Cloudflare for {d}. The rest of the page works.',
        klasseLinje: '{p} of the traffic in the last year came from known robots and scanners.', klasseDage: '({m} of 365 days)',
        robotNote: 'Known robots are the ones Cloudflare has verified, for example Googlebot. Robots that pretend to be an ordinary browser cannot be seen here and count as visitors. The share of the traffic is based on Cloudflare’s IP classes: search engines, monitoring, scanners, Tor and known bad addresses.',
        sideH: 'Most visited pages', sideP: 'Pages (html) in the period: views, visits from outside and share of the views. A click on a path shows the views of the page per day.',
        thSti: 'Page', thVisninger: 'Views', thBesoegUde: 'Visits from outside', stiAabn: 'Show views of {sti} per day ({n} {n|view|views})',
        visAlleSider: 'Show all {n} {n|page|pages}', side404H: 'Pages that do not exist', side404P: 'Paths that answered 404 (not found), often robots trying well-known addresses. Not every attempt is here: when the site answers with its ordinary page, the path does not end up on this list.',
        sideForsideH: 'Answered with the site’s ordinary page', sideForsideP: 'The page probably does not exist: the site answers with its front page at addresses it does not know.',
        sideForsideNote: 'An estimate from the size: the answer was as large as the front page (within 25 %), and nobody came from outside.',
        sideUdenForside: 'The front page is not among the pages in the period, so paths that answer with the front page cannot be told apart.',
        fasteH: 'The last days', valgtPeriode: 'Selected period: {p}',
        sideIngen: 'No page views in the period.', sideIkkeHentet: 'The most visited pages have not been fetched yet. They are fetched in the background after the visits.',
        sideIkkeTilg: 'The most visited pages are not available at Cloudflare for {d}. The rest of the page works.', sideFra: 'The pages exist from {d}.',
        sideNote: 'Robots count. A page like /admin is almost always visited by scanners, not people. Paths are shown without a query string, and long codes are hidden. The 50 most viewed pages are saved per day.',
      },
      sub: {
        h: 'Subdomains', pBes: 'Visits, requests and attacks per subdomain in the period. Clicking a subdomain shows Visitors for it.', pAng: 'Visits, requests and attacks per subdomain in the period. Clicking a subdomain shows Attacks for it.',
        th: 'Subdomain', thBesoeg: 'Visits', thForesp: 'Requests', thAngreb: 'Attacks', thAndel: 'Share', andre: 'Other ({n})', visAndre: 'Show {n} more {n|subdomain|subdomains}',
        filter: 'Subdomain: {vaert}', ariaBes: 'Show Visitors for {vaert}', ariaAng: 'Show Attacks for {vaert}',
        fra: 'Per subdomain from {dato}.', dnsOnly: 'Only traffic through Cloudflare. A subdomain set to “DNS only” has no numbers here.',
        ikke: 'Not per subdomain', ikkeTilg: 'Cloudflare did not return subdomains for this domain.', afskaaret: 'Only the most visited pages are included: Cloudflare sent at most 10,000 rows.',
        ip: 'Called by IP address', anden: 'Other name', ugyldig: 'Invalid name',
        fBesoeg: 'Visits to {vaert} in 30 days', fForesp: 'Requests to {vaert} in 30 days',
        grafBesoegH: 'Visits to {vaert} per day', grafBesoegUgeH: 'Visits to {vaert} per week', grafForespH: 'Requests to {vaert} per day', grafForespUgeH: 'Requests to {vaert} per week', grafP: 'Clicking a day shows the day.',
        fAngreb: 'Attacks on {vaert}', fAfvist: 'Refused on {vaert}', rulDage: '{navn}: {n}. Go to the days.',
        dageH: 'Attacks and refused on {vaert} per day', dageP: 'Attacks and refused are counted as in the rest of Attacks. Clicking a day shows the day.', dag: '{d}: {a} {a|attack|attacks}, {b} refused', legAngreb: 'Attacks', legAfvist: 'Refused',
        hRegler: 'The rules that fired', stiVaert: 'Show views of {sti} on {vaert} per day ({n} {n|view|views})',
      },
      ang: {
    henter: 'Fetching the figures from Cloudflare …', opdateret: 'Fetched from Cloudflare {t}', opdaterer: 'Showing the saved figures from {t} · fetching new ones from Cloudflare …',
    fejl: 'The figures could not be fetched: {f}', fejlGemte: 'New figures could not be fetched: {f} The view shows the last saved figures from {t}.', ingenZone: 'No domain is chosen. Choose one in the list.',
    mangler: 'The connection to the program’s Rust part is missing. Start it with npm run tauri dev.',
    d1Sub: 'today so far: {n}', andel: '1 in every {n} {n|request|requests}', forsoeg: 'Snoopers who found nothing ({p})', forsoegSub: 'requests for files that do not exist',
    fordI: 'Yesterday', ford: 'of which {b} blocked · {u} challenged · {l} lured into the labyrinth', fordAndet: ' · and {a} other, not counted as refused',
    fordNote: 'Cloudflare stops in several ways: it blocks, challenges (a browser check a robot cannot pass) or lures the robot into a labyrinth of fake pages. Pages with hidden trap links were served and do not count as refused; only when a robot follows a link has it been lured in. Whole days are counted in UTC.',
    kortH: 'Where the attacks came from', kortP: 'Each arc runs from the country an attack came from to the data centre in Cloudflare’s edge that stopped it. That happens in the data centre nearest the attacker, long before anything reaches your site.',
    kortTitel: 'World map of refused attacks · {p}', kortDesc: 'Most came from {lande}. Most were stopped in {colos}.',
    verden: 'World', europa: 'Europe', leg: { land: 'Country the attack came from', colo: 'Data centre that stopped it', bue: 'An attack under way', samme: '◆ inside a ring = stopped in the country the attack came from' },
        dageP: 'First line of defence: Cloudflare’s edge, which all traffic meets before it reaches your server. Refused requests per day (UTC).', dageTitel: 'Bar chart of refused requests per day · {p}', ialt: 'in total', maks: 'most: {n} on {d}',
    vis: 'Show the figures', dag: 'Day', antal: 'Refused', trafik: 'All requests',
    reglerP: 'Cloudflare’s managed ruleset knows attack patterns against common software. These are the rules that stopped something on {domaene}.',
    reglerNote: 'Whether an attack would have worked depends on what the site runs. Cloudflare stopped it anyway, before it arrived.',
        forsoegH: 'What they were looking for', forsoegP: 'Not everything is blocked. Many robots simply walk around trying door handles: they ask for files that are sometimes left out by accident elsewhere. Here {domaene} answered 404: the file did not exist.',
    forsoegPP: 'Not everything is blocked. Many robots simply walk around trying door handles: they ask for files that are sometimes left out by accident elsewhere. Here your {domaene} answered 404: the file did not exist.',
    ingenTalSamlet: 'None of your domains has figures yet. They are fetched in the background, and the page is drawn when they arrive.',
    mAfN: '{m} of {n} {n|domain|domains} {m|has|have} figures so far. The rest are fetched in the background.', valgteH: 'Your domains', flere: '+ {n} more', udenTal: 'no figures yet', opdateretSamlet: 'Fetched from Cloudflare {t} (the oldest of the chosen)', opdatererSamlet: 'Fetched from Cloudflare {t} (the oldest of the chosen) · fetching in the background …',
    nulstil: 'Reset', nulstilLabel: 'Reset all filters and the period', trafikChip: 'Traffic', skjult: 'Hidden with this filter, because the figures cannot be filtered that way: {liste}.', vurderingNavn: 'Assessment', soejlerNavn: 'The bars per day',
      },
      vejledningAabn: 'Open the guide', vejledningLabel: 'Open the guide in your browser',
      omVersion: 'Version {v}', omLinje: 'Apache-2.0 licence · No telemetry · Nothing is sent to Wi24rd',
      om2: 'Cloudflare® is a registered trademark of Cloudflare, Inc. Wi24rd Huginn and Muninn is not affiliated with Cloudflare, Inc.',
      om3: 'Fonts: Inter and JetBrains Mono (SIL Open Font License 1.1). Map data: world-countries (ODbL) and Natural Earth (public domain).',
      omKilde: 'Source code: {url}',
      proeverOm: 'Trying again in {s} s', proeverNu: 'Trying again …', proevNu: 'Try again now',
      domaener: n => flertal(n, '1 domain', `${n} domains`),
      fejl: {
        forbindelse: 'No connection to Cloudflare. Check your internet connection.',
        afvist: 'Cloudflare rejected the API Token. It may have expired or lack permissions.',
        vent: 'Cloudflare asks us to wait a little (too many requests).',
        cf_nede: 'Cloudflare is not answering right now.',
        http: 'Cloudflare answered with HTTP {status}.',
        api: 'Cloudflare answered with an error: {detalje}',
        radar: 'Cloudflare Radar is not answering right now.',
        graphql: 'Cloudflare could not deliver the figures for {dag}.', graphqlUdenDag: 'Cloudflare could not deliver the figures.',
        svar: 'Unexpected answer from Cloudflare.',
        zone_ukendt: 'The zone does not exist, or the API Token has no access to it.',
        intet_token: 'No API Token is saved. Paste your API Token under Connect to Cloudflare.',
        token_tomt: 'The field is empty. Paste your API Token.',
        token_form: 'What was pasted does not look like a Cloudflare API Token (only letters, digits and symbols without spaces).',
        token_inaktiv: 'The API Token is not active (status: {detalje}). Create a new read-only API Token.',
        mangler_zone: 'The API Token cannot see any domains: it lacks the permission Zone › Read, or the domain is not under Include · All zones or Specific zone.',
        mangler_analytics: 'The API Token lacks the permission Analytics › Read, so the program cannot read the figures.',
        ugyldig_zone: 'Invalid zone ID.', ugyldig_dato: 'Invalid date.', ugyldigt_filter: 'The filter in the address cannot be used.', for_mange_stier: 'You can have at most 20 login paths of your own per domain. Remove one before adding a new one.',
        noegle: 'The keychain could not be used.',
        lager: 'The local database could not be used.',
        nyere_database: 'The local database is from a newer version of the program (schema {detalje}).',
        autostart: 'Start at login could not be read or changed.',
        allerede_forbundet: 'This API Token sees the same domains as “{navn}”. It was not added.',
        intern: 'An error occurred in the program.',
      },
      tokenH: 'Connect to Cloudflare',
      tokenP: 'Paste an API Token that can only read. It is stored in your computer’s keychain (macOS Keychain or Windows Credential Manager) and never leaves the program’s Rust part. The web view cannot read it back.',
      tokenFelt: 'API Token (read only)', tokenKnap: 'Save and connect',
      hjaelpTokenH: 'How to create an API Token',
      gemmer: 'Checking the API Token with Cloudflare …', gemt: 'The API Token is valid and stored in the keychain.', glemt: 'The API Token has been deleted from the keychain.',
      mangler: 'The connection to the program’s Rust part is missing. Start it with npm run tauri dev.',
      zonerH: 'Overview',
      hjOverblik1: 'The domains you have chosen under Choose domains are fetched in the background while the program runs, also when the window is closed and it only sits in the menu bar.',
      hjOverblik2: 'The status compares yesterday’s attacks with the average of the week before: Act, Watch, Calm or Unknown (no figures yet).',
      hjOverblik3: 'At night (until 03:00 UTC), before yesterday is fully fetched, the status uses the day before and shows "updating …".',
      hjOverblik4: 'Cloudflare’s automatic bot checks (the labyrinth, Bot Fight Mode and the browser check) are routine and never give Watch or Act. "Refused" counts what Cloudflare blocked, challenged or lured into the labyrinth; the status only counts real attacks.',
      hjOverblik5: 'Act: Cloudflare has already stopped the attacks. Ask whoever looks after your website to check that software and plugins are up to date. Click a row to see the attacks, or see them all combined.',
      domHjaelp: 'What do I do? →', domHjaelpRoligt: 'Why is everything calm? →', domLuk: 'Hide',
      domRoed: n => flertal(n, '1 domain needs action', `${n} domains need action`),
      domGul: n => flertal(n, '1 domain needs watching', `${n} domains need watching`), domIntet: 'Nothing to do', domUkendt: 'No figures yet',
      domHoldOeje: ': keep an eye on it.', domRoligt: '{n}: all quiet.', domGraa: '{n}: no figures yet.', domGamle: '{n}: the figures are old.', domAltRoligt: 'All quiet on {n} yesterday.', domHentes: 'The figures are being fetched in the background.',
      samletKnap: 'Attacks for all', fravalgtNote: n => flertal(n, '1 domain is not chosen — Choose domains', `${n} domains are not chosen — Choose domains`), intetValgtH: 'You haven\'t chosen any domains yet', intetValgtP: 'Choose the domains Huginn should watch. Only those are fetched and shown.', intetValgtKnap: 'Choose domains',
      nyeLinje: n => flertal(n, '1 new domain on your Cloudflare account:', `${n} new domains on your Cloudflare account`), nyVaelg: 'Choose', nyVaelgFor: 'Choose {navn}', nyIkkeNu: 'Not now', nyIkkeNuFor: 'Not now: {navn}', nyeSe: 'See them', nyeSeLabel: 'See the new domains under Choose domains',
      domaenerH: 'Domains', domaenerP: 'Choose the domains Huginn should fetch and show. A domain you clear is no longer fetched; its figures stay.', domaenerVis: 'Show in Huginn', domaenerFor: 'Show {navn} in Huginn', domaenerAlleTil: 'Choose all', domaenerAlleFra: 'Clear all', domaenerSer: n => flertal(n, 'The API Token can see 1 domain', `The API Token can see ${n} domains`), domaenerGemt: 'The domains are saved.',
      tabelTitel: 'Your domains with figures from the local cache', thDomaene: 'Domain', thStatus: 'Status', thHvorfor: 'Why', thAfvist: 'Refused', thAngreb: 'Attacks', thIGaar: 'yesterday',
      th7: '7 days', th30: '30 days', thUgen: 'this week', thSidsteUge: 'last week', thAabn: 'Open',
      ovIGaar: 'refused yesterday', ov7: 'refused in 7 days', ov30: 'refused in 30 days', ovUge: 'attacks this week', ovSidste: 'attacks last week',
      status: { roed: 'Act', gul: 'Watch', groen: 'Calm', graa: 'Unknown' },
      hvServer: 'Your server is not answering: {p} of {n} {n|response|responses} yesterday were errors 520–527, so Cloudflare could not get an answer from your server. See “What do I do?”.', hvServerNat: 'Your server is not answering: {p} of {n} {n|response|responses} the day before yesterday were errors 520–527, so Cloudflare could not get an answer from your server. See “What do I do?”.', hvForstyrrelse: 'The same day there was a disruption in {land}.', hvForstyrrelseNet: 'The same day there was a disruption at {net} in {land}.',
      serverLinje: 'Your server is not answering:', serverAabn: 'What do I do about {navn}?', gServerH: 'Your server is not answering', gDnsH: 'Your DNS may point to an IP address you no longer own',
      trafikMaerke: 'Unusually high traffic', trafikAabn: 'Unusually high traffic on {navn} on {d}: {n} {n|request|requests}. Open the day.', trafikDag: 'Unusually high traffic on {d}: {n} {n|request|requests}. Cloudflare stopped {s} as {s|an attack|attacks}, and {o} got through', trafikDagMest: 'Unusually high traffic on {d}: {n} {n|request|requests}. Most of it was attacks that Cloudflare stopped: {s}, and {o} got through', trafikNote: 'What Cloudflare did not stop does not count as attacks. It may be an attempt to overload the site (DDoS), a robot running wild, or a fault. A normal day has {m} {m|request|requests}.',
      hvTrafik: 'Unusually high traffic: {n} {n|request|requests} (normally {m}). {stop} See where the traffic came from.', trafikStop: 'Cloudflare stopped {s} as {s|an attack|attacks}. The other {o} got through and {o|does not count as an attack|do not count as attacks} — it may be an attempt to overload the site (DDoS), a robot running wild, or a fault.', trafikStopMest: 'Most of the traffic spike was attacks that Cloudflare stopped. That was {s}. The other {o} got through and {o|does not count as an attack|do not count as attacks}.', gTrafikH: 'What do I do about unusually high traffic?',
      dykTrafikH: 'Traffic on {v}', dykTrafikP: 'These are the requests, not what Cloudflare stopped.', trafikIAlt: 'Requests', trafikNormalt: 'normally {m}', trafikStoppet: 'Stopped by Cloudflare', trafikLande: 'Visitors by country', trafikStatus: 'Response codes', trafikRobotter: 'Robots', trafikEtLand: '{p} came from one country ({land}).', trafik52: '{p} got 52x (your server did not answer).', trafikKode: '{p} got the response {k}.', trafikRobot: 'Most were the robot {r}.', trafikIngenEkstra: 'Figures for countries and response codes have not been fetched for the day yet.', trafikTomt: 'No figures for the day.',
      tek: TEK,
      hvIkkeBrugt: '{n} {n|attack|attacks} on {t}, which you do not use — harmless to you.', hvUventet: 'Unexpected: your server answered on a {t} path. Do you use {t} after all?', hvNaaet: '{n} {n|attack|attacks} on {t} reached your server, and you have said you use {t}.', hvBesked: 'More attacks on {t} than usual: {n} (average {s}).', hvSpoerg: 'Not using {t}? Say so here',
      tekH: 'Your site’s technology', tekP: 'Attacks on something the site does not run cannot hit it. Say what you use, so the attacks weigh correctly in status and warnings. The figures do not change: all attacks still stand in Attacks and on the map.', tekVedIkke: 'Don’t know', tekBesked: 'Notify me', tekBeskedNote: 'When there are more attacks on {t} than usual, whatever the notification level.', tekForsoeg: '{n} {n|attempt|attempts} in 30 days', tekForslagJa: 'set to “Does not use”?', tekGeneriskH: 'Always relevant', tekGeneriskP: 'Secrets, source code and login are not a technology you can opt out of.',
      hulLinje: 'Huginn was not on from {a} to {b}. Cloudflare keeps only a month, so {n} {n|day is|days are} missing for good from the history.', hulLinjeIntet: 'Huginn was not on from {a} to {b}. Cloudflare still had the figures, so nothing is missing.', hulLuk: 'Close', hulLukLabel: 'Close the line about the gap from {a} to {b}', dagMed: '{m} of {n} {n|day|days}', hulNote: 'The figures cover {m} of {n} {n|day|days}. The rest were not fetched: no data — Huginn was off.', hulIngen: 'no data', hulListeH: 'Gaps in the history', hulListeP: 'Days without figures. They are not 0: they were never fetched.',
      datoFra: 'From', datoTil: 'To', datoNulstil: 'Reset', datoNulstilLabel: 'Reset to all history', datoNote: 'Whole days in UTC. The time of day is not saved.', datoFejl: '“From” must be on or before “To”.',
      regelStier: 'Knocked on', regelFra: 'From', regelFoerst: 'First seen {d}, last {s}', dykReglerH: 'The rules that fired — {u}', dykKortN: 'in this view',
      hvIngenTal: 'No figures yet – fetching.', hvGammel: 'The figures are old – fetching new ones.', hvIngen: 'No attacks yesterday.', hvIngenNat: 'No attacks the day before yesterday.',
      hvAngreb: '{a} {a|attack|attacks} yesterday ({s})', hvAngrebNat: '{a} {a|attack|attacks} the day before yesterday ({s})', hvSnit: 'average {s}', hvFoerste: 'first time', egenRegel: 'your own rule “{navn}”',
      gTitel: 'What do I do?', gRoligtTitel: 'Why is everything calm?', gIntro: 'The advice is general. If in doubt, ask whoever looks after your website.',
      gHvad: 'What happened:', gRammer: 'Does it affect you?', gHvisJa: 'If yes, do this:', gGoer: 'Do this:', gHvisNej: 'If no:',
      gAlleStoppet: 'Cloudflare stopped {n|it|all {n}}.', gKomIgennem: '{k} got through.', gKomIgennemTrin: 'Open the domain’s attacks and show them to your web person.',
      gSe: n => flertal(n, 'See the attack from yesterday →', `See the ${n} attacks from yesterday →`), gSeNat: n => flertal(n, 'See the attack from the day before yesterday →', `See the ${n} attacks from the day before yesterday →`),
      cveAabn: 'Open {id} on cve.org (opens your browser)', cveLaes: 'Read about {id} ↗',
      hjCve: 'Clicking a CVE number (e.g. CVE-2025-55182) opens the CVE’s page on cve.org in your browser. The program itself fetches nothing from there.',
      
      dagIGaar: 'yesterday', 
      gRoligt1: 'Cloudflare refused {n} {n|attempt|attempts} yesterday on {d}, and {n|it was not unusual|none of them were unusual}.', gRoligt2: 'That is normal: robots constantly try known holes on every site, including yours. The status only becomes Watch or Act when a domain gets unusually many attacks compared with the week before.',
      hvFoelger: 'Follows the global trend.', hvSaerligtDig: 'The world was normal yesterday, so the rise is yours.', hvSaerligtDigNat: 'The world was normal the day before yesterday, so the rise is yours.',
      hvAlleStoppet: '{a|stopped|all stopped}', hvNafM: '{n} of {a} stopped', hvMest: 'Mostly {liste}.', hvOg: ' and ', hvOpdateres: 'Updating …',
      nBlokeret: '{n} blocked', nUdfordret: '{n} challenged', nLabyrint: '{n} in the labyrinth', nAndet: '{n} other',
      henterKort: 'fetching …', hentetTitel: 'Fetched {t}', ugeTitel: '{p} compared with the week before', aabnFor: 'Open Attacks for {navn}',
      fodnote: '"Refused" counts what Cloudflare blocked, challenged or lured into the labyrinth, including routine (Bot Fight Mode, browser check). Pages served with hidden trap links are not counted. The status only counts real attacks. Click a figure to look closer, or the rest of the row to open the domain’s attacks.',
      koeHenter: 'Fetching in the background: {z} of {Z} {Z|domain|domains} · {d} of {D} {D|day|days}', koeNu: 'now: {navn}',
      koeKvote: 'Cloudflare asks us to wait (the quota is used up) – continuing at {kl}.',
      koePause: 'Cloudflare is not answering right now – trying again at {kl}.', koePauseNet: 'No connection to Cloudflare – trying again at {kl}.',
      koeFaerdig: 'All domains are fetched (last {t}).', koeFaerdigFejl: 'Fetched (last {t}). {n} could not be fetched: {f}',
      koeNet: 'No connection to Cloudflare, so {d} could not be fetched. Trying again by itself.', pilleNet: 'No connection · trying again',
      koeFejl: 'Fetching in the background has stopped: {f}', koeFremskridt: 'Progress of the background fetching',
      henterZoner: 'Fetching your zones …', ingenZoner: 'The API Token cannot see any zones. Check that it has the Zone › Read permission.',
      gemteZonerNet: 'No connection to Cloudflare – showing your saved zones from {t}', gemteZoner: '{f} Showing your saved zones from {t}.',
      sidenH: 'Since last time', sidenVist: 'Yesterday’s status. You last looked here {t}.', sidenFoerste: 'Yesterday’s status for all your domains.',
      sidenGroenne: '{n}: nothing unusual', sidenRoligt: 'All quiet: {n}, {a} {a|attack|attacks} yesterday{b}.', sidenAlleBlok: ', all blocked',
      sidenGamle: '{n}: the figures are old – Huginn is fetching', sidenGraa: '{n}: no figures yet', ny: 'new', sidenSkjul: 'To the overview',
      hjSiden: 'Since last time shows the domains that need action or should be watched, and whether they are new since you last looked. The view opens by itself when you click a notification or choose Since last time in the menu bar.',
      vaernetH: 'Attacks', vaelgDomaene: 'Domain', samletValg: 'Combined ({n})',
      lead: 'Every day robots knock on {domaene} with known attacks against software the site may not even run. Cloudflare’s edge stops them in the data centre nearest the attacker, before they arrive. Here is what Cloudflare has seen in the last 30 days: where the attacks came from, which rules fired, and what they were looking for. No IP addresses are fetched or shown.',
      lead__fl: 'Every day robots knock on your {domaene} with known attacks against software the sites may not even run. Cloudflare’s edge stops them in the data centre nearest the attacker, before they arrive. Here is what Cloudflare has seen in the last 30 days on all of them together: where the attacks came from, which rules fired, and what they were looking for. No IP addresses are fetched or shown.',
      edgeH: 'Edge – first line of defence', edgeP: 'The edge is Cloudflare’s network of data centres. Your site answers from all of them (anycast), so an attack is stopped in the data centre nearest the attacker, long before it reaches your server. This only applies to records with the orange cloud. If an attacker knows your server’s own IP address, they can go around the edge unless the server only accepts traffic from Cloudflare.',
      metodeH: 'How it is counted',
      k1: 'GraphQL Analytics API', k2: 'The program’s Rust part', k3: 'SQLite on your computer', k4: 'This window',
      m1H: 'The source', m1: 'Cloudflare’s GraphQL Analytics API for the zone {domaene}: firewall events, all requests and verified bots. The API Token can only read.',
      m1__fl: 'Cloudflare’s GraphQL Analytics API for each of your zones ({domaene}), added up on your computer: firewall events, all requests and verified bots. The API Token can only read.',
      m2H: 'When you open the view', m2: 'The program asks for today and yesterday, one day at a time, so Cloudflare does not have to sample a long period. Older days are only fetched if they are missing. At most two calls at a time and at least 1.1 seconds apart, for all zones together; if Cloudflare asks us to wait, the program pauses.',
      m2__fl: 'A queue fetches all your zones in the background, one day at a time, so Cloudflare does not have to sample a long period. Older days are only fetched if they are missing. At most two calls at a time and at least 1.1 seconds apart; if Cloudflare asks us to wait, the queue pauses. The view is redrawn when new figures arrive.',
      m3H: 'The memory', m3: 'The days are stored as counts in an SQLite file in the program’s data folder on your computer. Nothing is deleted by itself, and next time the saved figures are shown at once while new ones are fetched.',
      m4H: 'What is not stored', m4: 'IP addresses are neither fetched nor stored. Paths are stored without the query part, long codes are hidden, and user agents are only stored as a bot name. The number refused is Cloudflare’s estimate from a sample: Cloudflare does not keep every event, but states for each how many it stands for, and the program adds that up.',
      kilder: 'Country centroids: world-countries (ODbL). Country outlines: Natural Earth (public domain).',
      hjDyk1: 'The page shows one period at a time: 7 days, 30 days or the dates you choose. All sections follow the period and the filters. A filter shows the saved figures and fetches nothing from Cloudflare.',
      hjDyk2: 'Click a value to add a filter. The filters are under the heading: × removes one, and Reset removes them all. All periods are whole days in UTC; today stands apart until the day is over.',
      rutine: 'Routine', nAfvist: '{n} refused', lokketInd: 'lured in', faelderSat: '{n} trap {n|link|links} inserted',
      faelderLinje: n => flertal(n, '1 page was served with hidden trap links for robots', `${n} pages were served with hidden trap links for robots`),
      dykFiltre: 'Filters', dykFjern: 'Remove the filter {navn}', 
       dykHenter: 'Fetching the figures …', dykAabn: '{navn}: {n}. Open.', dykAabnTekst: '{navn}. Open.', dykRul: '{navn}: {n}. Go to the list.', dykSkift: '{navn}: {n}. Change domain.', dykAabnRegelIDag: 'Open {navn}: {n} today so far',
      dykPeriode: 'Period', dykP7: '7 days', dykP30: '30 days', dykPAlt: 'All history', dykSlagsH: 'Show', dykSAlt: 'All', dykSAngreb: 'Attacks only ({n})', dykSRutine: 'Routine only ({n})',
      dykAfvist: 'Refused', dykAngrebOrd: 'Attacks', dykHaendelser: 'Events', dykF30: '{o} in 30 days', dykF30Sub: 'whole days up to and including yesterday', dykF7: '{o} in 7 days', dykFAlt: '{o} in all history', dykFIGaar: '{o} yesterday', dykFIDag: 'today so far: {n}', dykFDag: '{o} on {d}', dykFPeriode: '{o} {a} – {b}',
      dykFoerst: 'First seen', dykSidst: 'Last seen', dykEdge: 'Edge · {p}', dykEdgeP: 'Number per day (UTC). Click a day to see just that day.', dykDag: '{d}: {n}. Open the day.', dykIkkeHentet: '{d}: no data — Huginn was off',
      dykBue: '{a} to {b}: {n}. Open.',
      dykListerH: 'Broken down by', dykListerP: 'Click to add one more filter.', dykFlere: 'Show top 20 ({n})', listeAlle: 'Show all ({n})', tastatur: 'Arrow keys move, Enter opens.',
      kortUdenPlads: 'No place on the map: {liste}. They are in the lists.', kortBuer: '{n|The arc is the largest|The arcs are the {n} largest} of {m}.', kortFaaLande: '+ {n} {n|country|countries} with few attacks {n|is|are} not on the map.', kortFaaColos: '+ {n} data {n|centre|centres} with few attacks {n|is|are} not on the map.',
      dykLister: { land: 'Countries', netvaerk: 'Networks', sti: 'Paths', colo: 'Data centres', metode: 'Methods', handling: 'Action', robot: 'Robots', status: 'Responses' },
      dykHandling: { blokeret: 'Blocked', udfordret: 'Challenged', labyrint: 'Lured into the labyrinth', faelder: 'Pages with trap links', andet: 'Other' },
      dykAndreRegel: 'The same rule on your other domains', dykAndre: '{navn} on your other domains', dykAndreAlt: 'On your other domains',
      kortSkala: 'Country Cloudflare saw the attack come from (may be a VPN or proxy)', 
      dykSenesteH: 'The latest that were refused', dykTid: 'Time (UTC, whole hour)', dykDomaene: 'Domain', dykRegel: 'Rule', dykLand: 'Country', dykSti: 'Path', dykColo: 'Data centre', dykMetode: 'Method',
      dykSenesteP: 'Single events only exist for the latest 12 per day.', dykTomt: 'No figures in the period.', dykNote: 'Figures per day in UTC, from the figures saved on your computer. Time of day and IP addresses are not stored. The figures are Cloudflare’s estimate from a sample.', dykVaegtGraense: 'Before {d} the figures are counted rows, after that Cloudflare’s estimate.',
      dykLoft: 'At least one day in the period is at Cloudflare’s limit of 10,000 events, so the oldest hours of that day are missing. There are at least {n}.', dykCloudflareNavn: 'Cloudflare’s name:', dykNetNote: 'Only the network’s name is known, not its number, so two networks with the same name stand together.', dykForsoegNote: 'Requests for files that did not exist (answer 404). They are counted separately and never added to what was refused.',
      dykTor: 'The Tor network', dykUkendtLand: 'Unknown country',
      kat: {
        alle: ['All paths with 404', 'All the addresses that were tried and did not exist.'],
        hemmeligheder: ['Secrets', 'Files with passwords and keys someone forgot to hide (.env, appsettings).'],
        kildekode: ['Source code', 'Git folders and backup files that can reveal a site’s code.'],
        wordpress: ['WordPress', 'WordPress login and admin pages.'],
        php: ['PHP tools', 'Debug pages such as phpinfo that show the server’s setup.'],
        adgang: ['Control panels', 'Admin, status and management pages for other systems.'],
        andet: ['Other', 'Everything else that was asked for and did not exist.'],
      },
      type: [[/remote code execution|\brce\b/i, 'Remote code execution: making the server run the attacker’s own code'], [/sql injection|sqli/i, 'SQL injection: sneaking database commands in through the back door'], [/broken access control/i, 'Broken access control: reaching files or functions without permission'], [/file inclusion|lfi|path traversal/i, 'File inclusion: making the server open a file it should not'], [/xss|cross.site/i, 'Cross-site scripting: planting code that runs in other people’s browsers'], [/anomal/i, 'Unusual behaviour that looks like an attack'], [/browser integrity/i, 'Did not look like a real browser (missing or fake headers)']],
      indstH: 'Settings', autostart: 'Start at login', glem: 'Forget API Token…',
      grpGenerelt: 'General', grpForbindelse: 'Connection', grpData: 'Data', grpOm: 'About',
      luk: 'Close', omKnap: 'About Huginn and Muninn', omH: 'About Huginn and Muninn', grpUdseende: 'Appearance and language', grpDato: 'Date and country', grpStart: 'Start-up', grpFoerste: 'First start',
      guideIgenH: 'The guide', guideIgenP: 'The four steps from the first start: the welcome, your API Token, your domains and notifications.', guideIgen: 'Show the guide again',
      guideTrin1: 'Welcome', guideTrin2: 'Your API Token', guideTrin3: 'Your domains', guideTrin4: 'Notifications',
      guideH1: 'Welcome to Huginn and Muninn', guideH2: 'Connect to Cloudflare', guideH3: 'Choose your domains', guideH4: 'Notifications and start-up',
      guideP1: 'Four steps and you are up and running. You can skip and open the guide again under Settings.',
      guideP2: 'Paste an API Token that can only read. It is stored in your computer’s keychain (macOS Keychain or Windows Credential Manager) and never leaves the program’s Rust part. The web view cannot read it back.',
      guideP3: 'Choose the domains Huginn should fetch and show. You can change it later on Overview under Choose domains.',
      guideP4: 'Two choices and you are done.',
      baandBesked: 'Notifications', gBesked: 'Notify', beskedH: 'Notifications per domain', domBeskedP: 'When you want to be told about each chosen domain. “Follow default” uses the choice under Settings › Notifications.',
      guideNaeste: 'Next', guideTilbage: 'Back', guideSlut: 'Done', guideSpring: 'Skip',
      guideTokenFoerst: 'Save an API Token that works before you continue.', guideDomVenter: 'Your domains are being fetched from Cloudflare …',
      opsaetTomH: 'No API Token is saved', opsaetTomP: 'The guide shows how to create an API Token at Cloudflare that can only read, and connects the program.', opsaetTomKnap: 'Open the guide',
      baandTilAngreb: 'Back to Attacks', tekNulstilIntet: 'There are no choices to reset.',
      tema: 'Appearance', temaNote: 'Follows the system, or choose for yourself.', temaSystem: 'System', temaLys: 'Light', temaMoerk: 'Dark',
      tekst: 'Text size', tekstNote: 'Applies to the text in the whole program.', tekstLille: 'Small', tekstMellem: 'Normal', tekstStor: 'Large',
      tekstStoerre: 'Larger text', tekstMindre: 'Smaller text', tekstNormal: 'Normal text',
      autostartKort: 'The program starts in the menu bar and fetches in the background. Huginn also starts by itself once a day and fetches if it is not running. Turn it off before you delete the program.',
      forbStatus: 'Status', forbNote: 'The API Token is in your computer’s keychain.', forbNoteMac: 'The API Token is in the macOS Keychain.',
      forbOk: 'Connected · {n}', forbAfvist: 'Rejected', forbIngen: 'No API Token',
      forbNoegle: 'API Token for Cloudflare', forbNoegleNote: 'Can only read, never change anything.', skift: 'Change API Token…', annuller: 'Cancel',
      glemH: 'Forget the API Token?', glemP: 'The program stops fetching, and your per-domain choices are deleted. The history stays.', glemJa: 'Forget',
      /* More than one connection (V-KONTI.1): a connection is one API Token and a name. The shown name follows agent::vist_navn (D5). */
      grpForbindelser: 'Connections', forbTilfoej: 'Add API Token…', forbOmdoeb: 'Rename…', forbOmdoebH: 'Rename the connection', forbNavnFelt: 'Name',
      forbNavnNote: 'Empty: the name from Cloudflare.', forbNavnGem: 'Save', forbNavnGemt: 'The name is saved.', forbStandard: 'Connection {n}', forbFlereKonti: '{navn} + {n} more',
      forbForbundet: 'Connected', forbDomaenerAria: 'Show the domains for {navn}', forbOmdoebAria: 'Rename {navn}', forbSkiftAria: 'Change API Token for {navn}',
      forbGlem: 'Forget…', forbGlemAria: 'Forget {navn}', forbListe: 'Your connections to Cloudflare', forbFlereH: 'More Cloudflare accounts', forbFlereNote: 'One API Token per account. The domains are shown together.',
      forbSkiftNote: 'Do you have another account? Use Add API Token… instead. Change replaces the API Token on this connection.',
      glemForbH: 'Forget the connection “{navn}”?', glemForbP: 'The program stops fetching its domains, and your choices for them are deleted. The history stays.',
      afvistForb: 'Cloudflare rejected the API Token “{navn}”. Its domains are not fetched.', afvistFlere: 'Cloudflare rejected {n} of {m} API Tokens.', pilleAfvist: '{n|1 API Token|{n} API Tokens} rejected',
      domGruppeAlle: 'Select all', domGruppeIngen: 'None', domGruppeAlleAria: 'Select all domains for {navn}', domGruppeIngenAria: 'Clear all domains for {navn}',
      domaenerSerFlere: '{m} API Tokens can see {n} {n|domain|domains} in total',
      guideFlere: 'More than one Cloudflare account? Add them afterwards under Settings → Connections.',
      domHentesMed: 'Fetched with “{navn}”', domIngenSer: 'This API Token sees no domains.',
      forbNavnEns: 'Two connections have the same name. Give this one a name, so you can tell them apart.',
      dataDb: 'Local database', dataHist: 'History', dataHistNote: 'The program keeps all history on your computer and deletes nothing by itself.', dataHistNoteMac: 'The program keeps all history on your Mac and deletes nothing by itself.',
      dataDomaener: 'Domains with figures', dataAeldste: 'Oldest day', dataHuller: 'Gaps', dataHullerN: '{n} {n|day|days}', dataNafM: '{n} of {m}', dataHent: 'Fetch new figures now', dataIngen: 'None yet',
      nulstil: 'Reset history…', nulstilNote: 'Deletes all figures on your computer and fetches them again. Cloudflare only has security events for the last month and traffic figures for the last year — anything older is gone for good.', nulstilNoteMac: 'Deletes all figures on your Mac and fetches them again. Cloudflare only has security events for the last month and traffic figures for the last year — anything older is gone for good.', nulstilH: 'Reset the history?', nulstilJa: 'Delete and fetch again',
      nulstilIntetTab: 'Everything can be fetched again from Cloudflare.', nulstilt: 'The history has been reset. The program is fetching the figures again.',
      autostartNote: 'The program starts in the menu bar when you log in and fetches your figures in the background. Huginn also starts by itself once a day and fetches if it is not running. It stays in the menu bar, also when the window is closed; choose Quit in its menu to close it completely. Turn Start at login off here before you delete the program.',
      autostartTil: 'The program starts at login.', autostartFra: 'The program does not start at login.', autostartHul: 'Then Huginn only fetches while the program is open. Open it at least every three weeks, or gaps will appear in the history.',
      niveauH: 'Notifications', niveauSp: 'When do you want to be told?', niveauSpNote: 'The message comes from your own computer, never from a server.',
      opsaetNiveauP: 'When do you want to be told? You can change it later under Settings (the gear at the top right).',
      niveauRolig: 'Only when serious, at most once a day', niveauStandard: 'When serious (recommended)', niveauAlt: 'Also when a domain should be watched',
      notifSystem: 'You can also turn notifications off completely in System Settings → Notifications → Huginn and Muninn.',
      notifRoed: 'Notify me when a domain needs action', prDomaene: 'Per domain', prDomaeneTom: 'The domains appear here once the zone list is fetched.',
      prDomaeneFor: 'Notifications for {navn}',
      zv: { standard: 'Follow default', alt: 'Serious and watch', roed: 'Serious only', fra: 'Never' },
      notifNote: 'Act means unusually many attacks yesterday compared with the week before. Cloudflare’s routine (the labyrinth, Bot Fight Mode and the browser check) and old figures never notify you, and each domain is reported at most once a day. Click a notification, or choose Since last time in the menu bar, to open Since last time. You can also turn notifications off completely in System Settings → Notifications → Huginn and Muninn.',
      notifGemt: 'The notifications are saved.',
    },
  };
  /* Ordlisten: Cloudflares regelnavne (uden CVE-nummer) → klarsprog i "Hvorfor". Samme poster og rækkefølge som ORDLISTE i
     src-tauri/src/notify.rs (notifikationerne); UI-røgtesten sammenligner dem post for post. Første træf vinder; ukendte navne
     står, som de er (id 'andet'). Egne regler (Rust: egen = true) kommer før ordlisten og vises med kort navn. raad er et
     konkret næste skridt, vist ved Handl og Hold øje. id'et peger ind i handlingsbogen. */
  const RAAD_SITE = ['Bed din webmand tjekke, at siden er opdateret.', 'Ask your web person to check that the site is up to date.'];
  const ORDLISTE = [
    { id: 'react', re: /react/i, da: 'et kendt hul i React', en: 'a known hole in React', raadDa: RAAD_SITE[0], raadEn: RAAD_SITE[1] },
    { id: 'react', re: /next\.?js/i, da: 'et kendt hul i Next.js', en: 'a known hole in Next.js', raadDa: RAAD_SITE[0], raadEn: RAAD_SITE[1] },
    { id: 'log4j', re: /log4j/i, da: 'et kendt hul i Log4j (Java)', en: 'a known hole in Log4j (Java)', raadDa: 'Bed din webmand tjekke, at Java-programmerne er opdateret.', raadEn: 'Ask your web person to check that Java software is up to date.' },
    { id: 'wordpress', re: /wordpress|\bwp[-_ ]/i, da: 'angreb på WordPress', en: 'attacks on WordPress', raadDa: 'Bed din webmand tjekke, at WordPress og plugins er opdateret.', raadEn: 'Ask your web person to check that WordPress and its plugins are up to date.' },
    { id: 'browsertjek', re: /browser integrity/i, da: 'browsertjekket', en: 'the browser check', raadDa: '', raadEn: '' },
    { id: 'sqli', re: /sql ?injection|sqli/i, da: 'forsøg på at snige databasekommandoer ind', en: 'attempts to sneak in database commands', raadDa: '', raadEn: '' },
    { id: 'xss', re: /xss|cross.site/i, da: 'forsøg på at plante kode i andres browser', en: 'attempts to plant code in other people’s browsers', raadDa: '', raadEn: '' },
    { id: 'traversal', re: /path traversal|file inclusion|\blfi\b/i, da: 'forsøg på at åbne filer, de ikke må', en: 'attempts to open files they should not', raadDa: '', raadEn: '' },
    { id: 'rce', re: /remote code execution|\brce\b/i, da: 'forsøg på at køre kode på serveren', en: 'attempts to run code on the server', raadDa: '', raadEn: '' },
  ];
  /* Rutinens navne i klarsprog (J7). Står uden for ORDLISTE, som skal være ens med notify.rs: de bruges kun på regelkort og i dyk,
     og rutine kan ikke stå i "Hvorfor". */
  const RUTINE = {
    linkMaze: { da: 'Labyrinten (falske sider til robotter)', en: 'The labyrinth (fake pages for robots)' },
    botFight: { da: 'Bot Fight Mode (kendte bot-mønstre)', en: 'Bot Fight Mode (known bot patterns)' },
    'Browser Integrity Check': { da: 'Browsertjekket (lignede ikke en rigtig browser)', en: 'The browser check (did not look like a real browser)' },
  };
  const rutineNavn = navn => (RUTINE[navn] ? RUTINE[navn][lang] : String(navn));
  /* En egen regels korte navn: op til første ":" eller " – ", højst 24 tegn (ellers 23 + "…"); som kort_egen i notify.rs. */
  function kortEgen(navn) {
    const s = String(navn), i = [s.indexOf(':'), s.indexOf(' – ')].filter(x => x >= 0);
    const kort = (i.length ? s.slice(0, Math.min(...i)) : s).trim(), tegn = [...kort];
    return tegn.length > 24 ? tegn.slice(0, 23).join('') + '…' : kort;
  }
  function klar(navn, egen) {
    if (egen) return { id: 'egen', tekst: TEKST[lang].egenRegel.replace('{navn}', kortEgen(navn)), raad: '' };
    for (const o of ORDLISTE) if (o.re.test(navn)) return { id: o.id, tekst: o[lang], raad: lang === 'da' ? o.raadDa : o.raadEn };
    return { id: 'andet', tekst: String(navn), raad: '' };
  }
  /* Handlingsbogen ("Hvad gør jeg?"): én post pr. kategori (id fra ordlisten, plus 'egen' og 'andet'). spoerg = et ja/nej-
     spørgsmål, brugeren selv kan svare på; ja = 2–4 korte trin; nej = hvad det betyder, hvis svaret er nej. Uden spørgsmål
     står alle (hvorfor det gælder alle) og trinene. Generelle råd ("spørg din webmand"), aldrig "dit site er sårbart". */
  const BOG = {
    react: {
      da: { spoerg: 'Er sitet bygget med React eller Next.js? (Spørg din webmand.)', ja: ['Opdatér React og Next.js til nyeste version.', 'Tjek, at serveren ikke stadig kører en gammel udgave af sitet.'], nej: 'Intet. Cloudflare stoppede dem, og de kan ikke ramme et site uden React eller Next.js.' },
      en: { spoerg: 'Is the site built with React or Next.js? (Ask your web person.)', ja: ['Update React and Next.js to the latest version.', 'Check that the server is not still running an old version of the site.'], nej: 'Nothing. Cloudflare stopped them, and they cannot hit a site without React or Next.js.' },
    },
    wordpress: {
      da: { spoerg: 'Kører sitet WordPress?', ja: ['Opdatér WordPress, tema og plugins.', 'Slet plugins, I ikke bruger.', 'Brug stærke adgangskoder og totrinslogin for administratorer.'], nej: 'Intet. Cloudflare stoppede dem, og de kan ikke ramme et site uden WordPress.' },
      en: { spoerg: 'Does the site run WordPress?', ja: ['Update WordPress, the theme and the plugins.', 'Delete plugins you do not use.', 'Use strong passwords and two-step login for administrators.'], nej: 'Nothing. Cloudflare stopped them, and they cannot hit a site without WordPress.' },
    },
    log4j: {
      da: { spoerg: 'Kører sitet Java-programmer?', ja: ['Bed din webmand tjekke, at Log4j er opdateret.'], nej: 'Intet. Cloudflare stoppede dem, og de kan ikke ramme et site uden Java.' },
      en: { spoerg: 'Does the site run Java software?', ja: ['Ask your web person to check that Log4j is up to date.'], nej: 'Nothing. Cloudflare stopped them, and they cannot hit a site without Java.' },
    },
    sqli: {
      da: { spoerg: 'Har sitet formularer eller søgning, der taler med en database?', ja: ['Bed din webmand tjekke, at formularer og søgning er opdateret.', 'Tjek, at sitet ikke viser tekniske fejltekster til besøgende.'], nej: 'Intet. Cloudflare stoppede dem, og de kan ikke ramme et site uden en database bag formularerne.' },
      en: { spoerg: 'Does the site have forms or search that talk to a database?', ja: ['Ask your web person to check that forms and search are up to date.', 'Check that the site does not show technical error messages to visitors.'], nej: 'Nothing. Cloudflare stopped them, and they cannot hit a site without a database behind its forms.' },
    },
    xss: {
      da: { spoerg: 'Kan besøgende skrive noget, der vises for andre (kommentarer, formularer)?', ja: ['Opdatér sitet.', 'Bed din webmand tjekke formularerne.'], nej: 'Intet. Cloudflare stoppede dem, og de kan ikke ramme et site, hvor besøgende ikke kan skrive noget, der vises for andre.' },
      en: { spoerg: 'Can visitors write something that is shown to others (comments, forms)?', ja: ['Update the site.', 'Ask your web person to check the forms.'], nej: 'Nothing. Cloudflare stopped them, and they cannot hit a site where visitors cannot write anything shown to others.' },
    },
    traversal: {
      da: { alle: 'Det kan ramme alle sites.', ja: ['Opdatér sitet.', 'Læg ingen backup- eller konfigurationsfiler i webmappen.'] },
      en: { alle: 'It can affect any site.', ja: ['Update the site.', 'Keep no backup or configuration files in the web folder.'] },
    },
    rce: {
      da: { alle: 'Det kan ramme alle sites, der kører software på serveren.', ja: ['Opdatér sitet og den software, det kører på.', 'Bed din webmand tjekke, at intet er forældet.'] },
      en: { alle: 'It can affect any site that runs software on the server.', ja: ['Update the site and the software it runs on.', 'Ask your web person to check that nothing is outdated.'] },
    },
    browsertjek: {
      da: { alle: 'Nej. Det er Cloudflares rutinetjek af trafik, der ikke ligner en rigtig browser.', ja: ['Intet. Tjekket gjorde sit arbejde.'] },
      en: { alle: 'No. It is Cloudflare’s routine check of traffic that does not look like a real browser.', ja: ['Nothing. The check did its job.'] },
    },
    rutine: {
      da: { alle: 'Nej. Det er Cloudflares rutine, som kører af sig selv på al trafik.', ja: ['Intet. Rutinen gjorde sit arbejde.'] },
      en: { alle: 'No. It is Cloudflare’s routine, which runs by itself on all traffic.', ja: ['Nothing. The routine did its job.'] },
    },
    robot: {
      da: { ord: 'Robot', alle: 'Et program, der henter sider af sig selv: søgemaskiner, AI-tjenester, overvågning og meget andet.', ja: ['Intet. De fleste robotter er helt almindelige.'] },
      en: { ord: 'Robot', alle: 'A program that fetches pages by itself: search engines, AI services, monitoring and much else.', ja: ['Nothing. Most robots are perfectly ordinary.'] },
    },
    verificeret: {
      da: { ord: 'Verificeret', alle: 'Cloudflare har tjekket, at robotten er den, den siger, den er.', ja: ['Intet. Vil du ikke have den, blokerer du den selv hos Cloudflare.'] },
      en: { ord: 'Verified', alle: 'Cloudflare has checked that the robot is who it says it is.', ja: ['Nothing. If you do not want it, you block it yourself at Cloudflare.'] },
    },
    udklaedt: {
      da: { ord: 'Udklædt', alle: 'Noget, der kalder sig en kendt robot som Googlebot, men som Cloudflare ikke kan verificere.', ja: ['Intet lige nu. Stiger tallet, kan du stramme reglerne hos Cloudflare.'] },
      en: { ord: 'In disguise', alle: 'Something that calls itself a known robot such as Googlebot, but which Cloudflare cannot verify.', ja: ['Nothing right now. If the figure rises, you can tighten the rules at Cloudflare.'] },
    },
    linkpreview: {
      da: { ord: 'Link-preview', alle: 'Når et link deles, henter tjenesten titel og billede, så det kan vises i opslaget.', ja: ['Intet. Blokerer du dem, kan delte links miste billede og tekst.'] },
      en: { ord: 'Link preview', alle: 'When a link is shared, the service fetches the title and image so they can be shown in the post.', ja: ['Nothing. If you block them, shared links may lose their image and text.'] },
    },
    labyrint: {
      da: { ord: 'Labyrinten', alle: 'Cloudflare viser robotter, der ikke følger reglerne, falske sider, så de spilder deres tid. Den stopper dem ikke.', ja: ['Intet. Labyrinten kører af sig selv.'] },
      en: { ord: 'The labyrinth', alle: 'Cloudflare shows robots that do not follow the rules fake pages, so they waste their time. It does not stop them.', ja: ['Nothing. The labyrinth runs by itself.'] },
    },
    loginsti: {
      da: { ord: 'Login-sti', alle: 'Adressen på sitet, hvor man logger ind, fx /wp-login.php. Du vælger selv, hvilke der gælder for dit domæne.', ja: ['Slå dine egne stier til under Login (Ret stier).'] },
      en: { ord: 'Login path', alle: 'The address on the site where you log in, such as /wp-login.php. You choose which ones apply to your domain.', ja: ['Turn on your own paths under Login (Edit paths).'] },
    },
    naaede: {
      da: { ord: 'Nåede din server', alle: 'Cloudflare stoppede ikke forsøget, og din server svarede. Det siger ikke, om nogen kom ind.', ja: ['Sørg for stærke adgangskoder og totrinslogin.', 'Stiger tallet, kan du stramme reglerne for stien hos Cloudflare.'] },
      en: { ord: 'Reached your server', alle: 'Cloudflare did not stop the attempt, and your server answered. It does not say whether anyone got in.', ja: ['Use strong passwords and two-step login.', 'If the figure rises, you can tighten the rules for the path at Cloudflare.'] },
    },
    radar: {
      da: { ord: 'Cloudflare Radar', alle: 'Cloudflares offentlige tal for hele internettet. Tallene er procenter og indeks, ikke antal, og de er ikke dine egne data.', ja: ['Intet. Brug dem til at se, om det, der sker hos dig, også sker andre steder.'] },
      en: { ord: 'Cloudflare Radar', alle: 'Cloudflare’s public figures for the whole internet. They are percentages and indexes, not numbers, and they are not your own data.', ja: ['Nothing. Use them to see whether what happens to you also happens elsewhere.'] },
    },
    indeks: {
      da: { ord: 'Indeks', alle: 'Tallene er regnet om, så 100 er periodens gennemsnit. 200 er dobbelt så meget som normalt, uanset om det er 10 eller 10.000 angreb.', ja: ['Intet. Se på, om kurverne stiger samtidig.'] },
      en: { ord: 'Index', alle: 'The figures are converted so that 100 is the average of the period. 200 is twice as much as usual, whether it is 10 or 10,000 attacks.', ja: ['Nothing. Look at whether the curves rise at the same time.'] },
    },
    kildemaal: {
      da: { ord: 'Kilde og mål', alle: 'Kilden er det land, angrebet kom fra. Målet er det land, zonen under angreb faktureres i. Et land kan være begge dele, og de to tal siger noget forskelligt.', ja: ['Intet. Læs overskriften: "Hvor angreb mod … kom fra" er kilderne, "som mål" er målet.'] },
      en: { ord: 'Source and target', alle: 'The source is the country an attack came from. The target is the country the attacked zone is billed in. A country can be both, and the two figures say different things.', ja: ['Nothing. Read the heading: "Where attacks on … came from" is the sources, "as a target" is the target.'] },
    },
    server: {
      da: { alle: 'Ja. Fejl 520–527 er Cloudflares svar, når din server ikke svarer, afviser forbindelsen eller svarer med noget, Cloudflare ikke kan bruge. Besøgende ser en fejlside i stedet for dit site.', ja: ['Tjek, at serveren kører og kan nås.', 'Spørg din udbyder, om der er en driftsforstyrrelse.', 'Tjek, at din DNS peger på den rigtige server (se nedenfor).', 'Fortsætter det, så vis det til din webmand.'] },
      en: { alle: 'Yes. Errors 520–527 are Cloudflare’s answer when your server does not respond, refuses the connection or replies with something Cloudflare cannot use. Visitors see an error page instead of your site.', ja: ['Check that the server is running and can be reached.', 'Ask your provider whether there is a disruption.', 'Check that your DNS points to the right server (see below).', 'If it goes on, show it to your web person.'] },
    },
    /* Cloudflares ord fra dokumentationen (Pause Cloudflare, Zone status og Zone stuck in Pending Nameserver Update, Development Mode, Leaked credentials detection). */
    pause: {
      da: { alle: 'Ja. Mens domænet er sat på pause, går trafikken uden om Cloudflare, så hverken regler eller cache virker, og besøgende rammer din server direkte.', ja: ['Log ind hos Cloudflare, og vælg domænet.', 'Find »Pause Cloudflare«, og slå pausen fra.', 'Har du selv sat pausen for at fejlsøge, så husk at slå den fra igen, når du er færdig.'] },
      en: { alle: 'Yes. While the domain is paused, traffic goes around Cloudflare, so neither rules nor the cache work, and visitors reach your server directly.', ja: ['Log in to Cloudflare and choose the domain.', 'Find “Pause Cloudflare” and turn the pause off.', 'If you paused it yourself to troubleshoot, remember to turn it off when you are done.'] },
    },
    inaktiv: {
      da: { alle: 'Ja. Cloudflare beskytter kun et domæne med status »Active«. Står det på »Pending«, kan Cloudflare ikke se, at domænet peger på de navneservere, det har fået. Et Free-domæne, der står på Pending i mere end 28 dage, bliver slettet.', ja: ['Log ind hos Cloudflare, vælg domænet, og åbn siden Overview. Her står de navneservere, domænet har fået.', 'Tjek hos den, du har købt domænet af (registrator), at navneserverne er præcis de samme.', 'Fjern gamle DS-poster hos registratoren, hvis DNSSEC var slået til.', 'Bed om et nyt tjek fra siden Overview.'] },
      en: { alle: 'Yes. Cloudflare only protects a domain with the status “Active”. If it says “Pending”, Cloudflare cannot see that the domain points to the nameservers it was assigned. A Free domain that stays Pending for more than 28 days is deleted.', ja: ['Log in to Cloudflare, choose the domain and open the Overview page. It shows the nameservers assigned to the domain.', 'Check at your registrar that the nameservers are exactly the same.', 'Remove old DS records at the registrar if DNSSEC was on.', 'Request a new check from the Overview page.'] },
    },
    phishing: {
      da: { alle: 'Det kan det. Mistanken kan komme af sider, nogen har lagt på dit site uden at spørge, og besøgende kan få en advarsel, indtil den er afklaret.', ja: ['Log ind hos Cloudflare, vælg domænet, og læs Cloudflares besked om mistanken.', 'Tjek sitet for sider og filer, du ikke selv har lagt der, og skift adgangskoderne.', 'Følg Cloudflares vejledning for at få mistanken gennemgået, når sitet er rent.'] },
      en: { alle: 'It can. The suspicion may come from pages someone has put on your site without asking, and visitors may get a warning until it is resolved.', ja: ['Log in to Cloudflare, choose the domain and read Cloudflare’s message about the suspicion.', 'Check the site for pages and files you did not put there, and change the passwords.', 'Follow Cloudflare’s instructions to have the suspicion reviewed once the site is clean.'] },
    },
    udvikler: {
      da: { alle: 'Kun lidt. Development Mode slår cachen fra i tre timer, så alle forespørgsler går til din server. Det er nyttigt, mens du retter noget, men serveren får mere at lave.', ja: ['Er du færdig med at rette, så log ind hos Cloudflare, vælg domænet, og slå Development Mode fra.', 'Ellers skal du ikke gøre noget: den slår selv fra.'] },
      en: { alle: 'Only a little. Development Mode turns the cache off for three hours, so all requests go to your server. It is useful while you fix something, but the server gets more to do.', ja: ['If you are done, log in to Cloudflare, choose the domain and turn Development Mode off.', 'Otherwise do nothing: it turns itself off.'] },
    },
    laek: {
      da: { alle: 'Måske. Et kodeord fra et læk kan være en angriber, der prøver lækkede kodeord af, eller en rigtig bruger, der genbruger et gammelt kodeord. Tallene kan ikke skelne.', ja: ['Åbn Login, og se hvilke stier og dage det gælder.', 'Nåede forsøget din server, så tjek om nogen er logget ind, og bed brugerne skifte kodeord.', 'Slå to-trins-login til, hvis dit system kan det.', 'Cloudflare har en skabelon til en rate limiting-regel, der bremser mange forsøg med lækkede kodeord: se Security rules.'] },
      en: { alle: 'Perhaps. A password from a leak can be an attacker trying leaked passwords, or a real user reusing an old password. The figures cannot tell them apart.', ja: ['Open Login and see which paths and days it concerns.', 'If the attempt reached your server, check whether anyone logged in and ask users to change their passwords.', 'Turn on two-step login if your system supports it.', 'Cloudflare has a template for a rate limiting rule that slows down many attempts with leaked passwords: see Security rules.'] },
    },
    sti200: {
      da: { alle: 'Måske. Svaret var lille og ikke på størrelse med forsiden, så det kan være selve filen. Det kan stadig være en fejlside, men tjek det med det samme.', ja: ['Åbn adressen i din browser (domænet efterfulgt af stien, fx /.env), og se, hvad der kommer.', 'Ligger filen der, så fjern den fra serveren, og skift alle kodeord og hemmeligheder, der stod i den: de skal regnes for lækket.', 'Bed din webmand sørge for, at serveren aldrig udleverer filer, der begynder med punktum (.env, .git).', 'Indtil da kan du lave en regel hos Cloudflare under Security rules, der blokerer stien.'] },
      en: { alle: 'Perhaps. The answer was small and not the size of the front page, so it may be the file itself. It can still be an error page, but check it right away.', ja: ['Open the address in your browser (the domain followed by the path, for example /.env) and see what comes back.', 'If the file is there, remove it from the server and change every password and secret that was in it: treat them as leaked.', 'Ask your web person to make sure the server never hands out files that start with a dot (.env, .git).', 'Until then you can make a rule at Cloudflare under Security rules that blocks the path.'] },
    },
    cf_fund: {
      da: { alle: 'Det er Cloudflares egen skanning af domænets opsætning (Security Insights). Et kritisk fund er vigtigt, men de fleste fund er råd om indstillinger, ikke tegn på et angreb.', ja: ['Åbn Angreb for domænet, og klik på fundet under »Cloudflares egne fund«. Det åbner Security Insights hos Cloudflare, hvor fundet er forklaret.', 'Følg Cloudflares vejledning ved fundet, eller arkivér det hos Cloudflare, hvis det ikke gælder for dig.', 'Fundet forsvinder herfra, når Cloudflare har skannet igen (hver 7. dag på Free).'] },
      en: { alle: 'It is Cloudflare’s own scan of the domain’s setup (Security Insights). A critical finding matters, but most findings are advice about settings, not signs of an attack.', ja: ['Open Attacks for the domain and click the finding under “Cloudflare’s own findings”. It opens Security Insights at Cloudflare, where the finding is explained.', 'Follow Cloudflare’s instructions for the finding, or archive it at Cloudflare if it does not apply to you.', 'The finding disappears from here once Cloudflare has scanned again (every 7 days on Free).'] },
    },
    /* Cloudflares ord fra dokumentationen (Under Attack mode, HTTP DDoS attack protection, Create rate limiting rules). */
    trafik: {
      da: { alle: 'Det kan vi ikke sige om det, der slap igennem. Et forsøg på at overbelaste siden, en robot, der løber løbsk, og en fejl ser ens ud her.', ja: ['Åbn dagen, og se, om det meste kom fra ét land eller én robot, og om din server svarede.', 'Cloudflares beskyttelse mod HTTP-angreb (HTTP DDoS attack protection) er altid slået til. Du kan justere den på siden Security rules, fanen DDoS protection.', 'Er sitet nede lige nu, så slå Under Attack Mode til under Quick Actions på domænets oversigtsside. Besøgende får en mellemside, så det er en sidste udvej.', 'Bliver én sti hamret, så lav en regel på siden Security rules med Create rule → Rate limiting rules.', 'Tjek, at din DNS-post peger på den rigtige server (se nedenfor).'] },
      en: { alle: 'The figures cannot say what got through. An attempt to overload the site, a robot running wild and a fault look the same here.', ja: ['Open the day and see whether most came from one country or one robot, and whether your server answered.', 'Cloudflare’s protection against HTTP attacks (HTTP DDoS attack protection) is always on. You can adjust it on the Security rules page, the DDoS protection tab.', 'If the site is down right now, turn on Under Attack Mode under Quick Actions on the domain’s overview page. Visitors get an interstitial page, so it is a last resort.', 'If one path is being hammered, make a rule on the Security rules page with Create rule → Rate limiting rules.', 'Check that your DNS record points to the right server (see below).'] },
    },
    dns: {
      da: { alle: 'Det kan det, hvis du har skiftet server eller udbyder. En gammel DNS-post kan stadig pege på den tidligere IP-adresse. Er adressen givet videre, sendes din trafik videre til den, der har adressen nu.', ja: ['Log ind på Cloudflare, og vælg domænet.', 'Gå til DNS → Records.', 'Find posterne af typen A eller AAAA med den gamle adresse, og vælg Edit.', 'Ret Content til din nuværende adresse, og vælg Save.', 'Bruger du ikke posten mere, så vælg Edit og Delete, og bekræft med Delete.'] },
      en: { alle: 'It can, if you have changed server or provider. An old DNS record can still point to the former IP address. If the address has been passed on, your traffic is sent on to whoever has the address now.', ja: ['Log in to Cloudflare and choose the domain.', 'Go to DNS → Records.', 'Find the records of type A or AAAA with the old address and select Edit.', 'Change Content to your current address and select Save.', 'If you no longer use the record, select Edit and Delete, and confirm with Delete.'] },
    },
    tendens: {
      da: { alle: 'Sammenligningen ser kun på WAF-angreb generelt, ikke på en bestemt regel eller et bestemt hul. Følger du tendensen, steg den slags angreb også i resten af verden. Er stigningen din egen, steg dine angreb mindst tre gange så meget som normalt, mens verden var normal. Status ændres ikke af det.', ja: ['Følg rådene for det, der ramte dig, under Overblik.'] },
      en: { alle: 'The comparison only looks at WAF attacks in general, not at a particular rule or hole. If you follow the trend, that kind of attack also rose in the rest of the world. If the rise is yours, your attacks were at least three times the usual while the world was normal. The status is not changed by it.', ja: ['Follow the advice for what hit you under Overview.'] },
    },
    egen: {
      da: { alle: 'Det er din egen regel.', ja: ['Din regel virkede. Ingen handling.', 'Overvej kun at stramme den, hvis tallet stiger flere dage i træk.'] },
      en: { alle: 'It is your own rule.', ja: ['Your rule worked. No action.', 'Only consider tightening it if the number rises several days in a row.'] },
    },
    maalrettet: {
      da: { alle: 'Angrebene steg her, men ikke andre steder, og de fleste kom fra ét netværk. Det ligner nogen, der sigter på netop dig, men tallene kan ikke sige det sikkert.', ja: ['Tjek, at din server kun tager imod trafik fra Cloudflare, så ingen kan gå uden om.', 'Stram reglerne for de stier, der blev ramt.', 'Overvej Under Attack Mode, mens det står på.'] },
      en: { alle: 'Attacks rose here but not elsewhere, and most came from one network. It looks like someone aiming at you in particular, but the figures cannot say for certain.', ja: ['Check that your server only accepts traffic from Cloudflare, so nobody can go around it.', 'Tighten the rules for the paths that were hit.', 'Consider Under Attack Mode while it lasts.'] },
    },
    rekognoscering: {
      da: { alle: 'Nogen har prøvet mange adresser for at se, hvad der findes. De fleste er automatiske scannere.', ja: ['Se, hvilke stier de ledte efter.', 'Tjek, at der ikke ligger noget fremme, der ikke skal være der (fx sikkerhedskopier og .env-filer).'] },
      en: { alle: 'Someone tried many addresses to see what exists. Most are automated scanners.', ja: ['See which paths they looked for.', 'Check that nothing is left out that should not be there (for example backups and .env files).'] },
    },
    dns_rekognoscering: {
      da: { alle: 'Nogen har spurgt efter mange navne under dit domæne, der ikke findes, for at se, hvad der gør. De fleste er automatiske scannere. Tallene siger ikke, hvem det var.', ja: ['Se navnene i boksen »Vurdering« for dagen.', 'Tjek under DNS → Records hos Cloudflare, at der ikke ligger gamle navne, du har glemt (fx test. og dev.).'] },
      en: { alle: 'Someone asked for many names under your domain that do not exist, to see what does. Most are automated scanners. The figures cannot say who it was.', ja: ['See the names in the “Assessment” box for the day.', 'Check under DNS → Records at Cloudflare that no old names are left that you have forgotten (for example test. and dev.).'] },
    },
    dmarc: {
      da: { alle: 'Måske. Mails, der fejler DMARC, kan være nogen, der udgiver sig for dig, eller en tjeneste, du selv bruger, men ikke har sat op endnu. Tallene kan ikke skelne.', ja: ['Åbn Angreb for domænet, og se afsenderne under »Mails sendt i dit domænes navn«.', 'Kender du afsenderen, så sæt den op, så den består. Kender du den ikke, er det en grund til at stramme din DMARC-politik.'] },
      en: { alle: 'Perhaps. Mail that fails DMARC can be someone pretending to be you, or a service you use yourself but have not set up yet. The figures cannot tell them apart.', ja: ['Open Attacks for the domain and see the senders under “Mail sent in your domain’s name”.', 'If you know the sender, set it up so it passes. If you do not, that is a reason to tighten your DMARC policy.'] },
    },
    /* Cloudflares ord fra DMARC Management: Enable DMARC Management. */
    dmarc_til: {
      da: { alle: 'DMARC Management samler de rapporter, modtagerne sender om mails i dit domænes navn. Det virker kun på hoveddomæner (wi24rd.com, ikke blog.wi24rd.com).', ja: ['Log ind på Cloudflare, og vælg domænet.', 'Gå til Email → DMARC Management.', 'Vælg Enable DMARC Management.', 'Det kan tage op til 24 timer, før den første rapport er der.'] },
      en: { alle: 'DMARC Management collects the reports receivers send about mail in your domain’s name. It only works on apex domains (wi24rd.com, not blog.wi24rd.com).', ja: ['Log in to Cloudflare and choose the domain.', 'Go to Email → DMARC Management.', 'Select Enable DMARC Management.', 'It may take up to 24 hours to receive the first report.'] },
    },
    andet: {
      da: { alle: 'Det kan regelnavnet alene ikke afgøre.', ja: ['Følg med i Angreb-visningen.', 'Står domænet på Handl flere dage i træk, så vis det til din webmand.'] },
      en: { alle: 'The rule name alone cannot tell.', ja: ['Keep an eye on the Attacks view.', 'If the domain shows Act several days in a row, show it to your web person.'] },
    },
  };
  const bog = id => (BOG[id] || BOG.andet)[lang];

  const T = TEKST[lang];
  const udfyld = s => String(s).replace(/\{domaene\}/g, domaene || '…').replace(/\{land\}/g, landNavn || '…');
  document.documentElement.lang = lang;
  /* Flertalsnøgler slutter på __fl. Et almindeligt "P" duede ikke: glem + glemP (P for paragraf) blev læst som flertal. */
  const FLERTAL = '__fl';
  let flereDomaener = false;
  /* Wi24rd UI Kit 2 (kit.js) sætter også data-t-aria, data-t-title og data-t-ph på det, den bygger: de udfyldes her som aria-label, title og placeholder. */
  const ATTRIBUTTER = [['data-t-aria', 'aria-label'], ['data-t-title', 'title'], ['data-t-ph', 'placeholder']];
  function anvend(rod = document) {
    const opslag = k => { const kk = flereDomaener && (k + FLERTAL) in T ? k + FLERTAL : k; return kk in T && typeof T[kk] === 'string' ? udfyld(T[kk]) : null; };
    const alle = (vaelger, fn) => { if (rod.matches && rod.matches(vaelger)) fn(rod); rod.querySelectorAll(vaelger).forEach(fn); };
    alle('[data-t]', el => { const t = opslag(el.getAttribute('data-t')); if (t !== null) el.textContent = t; });
    for (const [fra, til] of ATTRIBUTTER) alle(`[${fra}]`, el => { const t = opslag(el.getAttribute(fra)); if (t !== null) el.setAttribute(til, t); });
  }
  anvend();
  function saetDomaene(tekst, fl) { domaene = String(tekst).slice(0, 253); flereDomaener = Boolean(fl); anvend(); }
  /* Hvad dyk-visningen viser ("Polen"), til vinduets titel. */
  /* Visningens navn ("Danmark lige nu"), til vinduets titel. */
  const saetLand = tekst => { landNavn = String(tekst); };
  /* Vinduets titel for en visning ("Angreb · 3 domæner · Wi24rd Huginn and Muninn"); Rust følger document.title. */
  const titel = visning => (T.titel[visning] ? udfyld(T.titel[visning]) + ' · ' : '') + T.appNavn;
  const vaelger = document.getElementById('sprog');
  if (vaelger) {
    vaelger.value = valg;
    vaelger.addEventListener('change', () => { try { localStorage.setItem('hm-sprog', vaelger.value); } catch (e) { } tilRust(afgjort(vaelger.value)); location.reload(); });
  }
  const datoVaelgerEl = document.getElementById('dato-format');
  if (datoVaelgerEl) {
    datoVaelgerEl.value = datoValg;
    datoVaelgerEl.addEventListener('change', () => { try { localStorage.setItem('hm-dato', datoVaelgerEl.value); } catch (e) { } datoValg = datoVaelgerEl.value; datoTilRust(); location.reload(); });
  }
  /* Fejl fra Rust-delen er en kode ({kode, status?, dag?, detalje?}); teksten vælges her efter sprogvalget. Alt andet
     (fx en rå tekst fra Tauri eller JavaScript) vises aldrig, kun den generelle fejltekst. */
  const loc = lang === 'da' ? 'da-DK' : 'en-GB';
  /* Alle datoer går gennem disse, så Datoformat gælder overalt. dato: "4. nov. 2026", datoLang: "4. november" i brødtekst og overskrifter, datoKort: aksernes "4/11", datoTid: "De seneste", kl: et tidspunkt.
     Følg systemet giver webvisningens egen sprogopsætning, som på en dansk Mac kan være en-US, selv om regionen er Danmark. Derfor er sproget standard. Samme korte måneder som `MND_DA` og `MND_EN` i notify.rs. */
  const MND = { da: ['jan.', 'feb.', 'mar.', 'apr.', 'maj', 'jun.', 'jul.', 'aug.', 'sep.', 'okt.', 'nov.', 'dec.'], en: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'] };
  const erDag = d => /^\d{4}-\d{2}-\d{2}$/.test(String(d));
  const dele = d => String(d).split('-').map(Number);
  const iSystem = (d, opt) => new Intl.DateTimeFormat(undefined, { timeZone: 'UTC', ...opt }).format(new Date(`${d}T00:00:00Z`));
  const dato = d => {
    if (!erDag(d) || datoValg === 'iso') return String(d);
    if (datoValg === 'system') return iSystem(d, { dateStyle: 'medium' });
    const [y, m, dd] = dele(d);
    return lang === 'da' ? `${dd}. ${MND.da[m - 1]} ${y}` : `${dd} ${MND.en[m - 1]} ${y}`;
  };
  /* Året står, når det ikke er i år: et års historik gør "7. november" tvetydigt. */
  const aarNu = () => String(new Date().getUTCFullYear());
  const datoLang = (d, medAar) => {
    if (!erDag(d) || datoValg === 'iso') return String(d);
    if (datoValg === 'system') return iSystem(d, { dateStyle: 'long' });
    return new Date(`${d}T00:00:00Z`).toLocaleDateString(loc, { day: 'numeric', month: 'long', timeZone: 'UTC', ...(medAar || d.slice(0, 4) !== aarNu() ? { year: 'numeric' } : {}) });
  };
  /* To datoer i samme sætning: går de over et årsskifte, står året ved begge. */
  const datoPar = (a, b) => { const aar = String(a).slice(0, 4) !== String(b).slice(0, 4); return [datoLang(a, aar), datoLang(b, aar)]; };
  const datoKort = d => {
    if (!erDag(d)) return String(d);
    if (datoValg === 'iso') return String(d).slice(5);
    if (datoValg === 'system') return iSystem(d, { day: 'numeric', month: 'numeric' });
    const [, m, dd] = dele(d);
    return lang === 'da' ? `${dd}/${m}` : `${dd} ${MND.en[m - 1]}`;
  };
  /* Månedens korte navn til en akse (uger på Besøgende): "nov." / "Nov", i ISO "2026-11". */
  const maaned = d => {
    if (!erDag(d)) return String(d);
    if (datoValg === 'iso') return String(d).slice(0, 7);
    if (datoValg === 'system') return iSystem(d, { month: 'short' });
    return MND[lang === 'da' ? 'da' : 'en'][dele(d)[1] - 1];
  };
  const datoTid = t => { const s = String(t), d = s.slice(0, 10); return erDag(d) ? `${dato(d)} ${s.slice(11, 16)}`.trim() : s; };
  const to = n => String(n).padStart(2, '0');
  const kl = iso => {
    const t = new Date(iso);
    if (!iso || isNaN(t)) return '';
    if (datoValg === 'iso') return `${t.getFullYear()}-${to(t.getMonth() + 1)}-${to(t.getDate())} ${to(t.getHours())}:${to(t.getMinutes())}`;
    if (datoValg === 'system') return t.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
    return t.toLocaleString(loc, { day: 'numeric', month: 'long', hour: '2-digit', minute: '2-digit' });
  };
  /* Systemets rækkefølge til notifikationerne: Rust kan ikke selv læse den. Dagen 4. januar viser, om dag og måned har foranstillet 0. */
  function systemMoenster() {
    try {
      const k = { day: v => (v.length > 1 ? 'dd' : 'd'), month: v => (v.length > 1 ? 'MM' : 'M'), year: v => (v.length > 2 ? 'yyyy' : 'yy') };
      return new Intl.DateTimeFormat(undefined, { dateStyle: 'short', timeZone: 'UTC' }).formatToParts(new Date(Date.UTC(2026, 0, 4))).map(p => (k[p.type] ? k[p.type](p.value) : p.value)).join('').slice(0, 32);
    } catch (e) { return ''; }
  }
  function datoTilRust() {
    const invoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    if (invoke) invoke('dato_format_saet', { format: datoValg, moenster: systemMoenster() }).catch(() => { });
  }
  datoTilRust();
  const tid = iso => { const t = new Date(iso); return iso && !isNaN(t) ? t.toLocaleTimeString(loc, { hour: '2-digit', minute: '2-digit' }) : ''; };
  const tal = n => Number(n || 0).toLocaleString(loc);
  /* Filstørrelse (decimale enheder, som Finder) og en periode af hele dage: "4. okt. – 2. nov. 2026". */
  const stoerrelse = b => (b < 1e6 ? `${(Number(b || 0) / 1e3).toLocaleString(loc, { maximumFractionDigits: 0 })} kB` : `${(b / 1e6).toLocaleString(loc, { maximumFractionDigits: 1 })} MB`);
  /* To datoer: "4. okt. – 5. nov. 2026". Året står kun én gang, hvis det er det samme. */
  const periode = (a, b) => (a === b ? dato(b) : `${datoValg === 'sprog' && a.slice(0, 4) === b.slice(0, 4) ? dato(a).replace(/ \d{4}$/, '') : dato(a)} – ${dato(b)}`);
  /* Mærket "Ligner …" (notify::moenster_tekst). Altid "ligner": tallene kan ikke mere. */
  /* Mærkets navn: i Overblik, Siden sidst og Angreb. */
  /* kort: Overblikkets smalle statuskolonne skriver »recon«; alle andre steder står det fulde ord. */
  const maerkeNavn = (m, kort) => ({ maalrettet: T.maerkeMaal, dns_rekognoscering: kort ? T.maerkeDnsKort : T.maerkeDns }[m.slags] || (kort ? T.maerkeRekogKort : T.maerkeRekog)) + (m.foreloebig ? ` · ${T.maerkeForeloebig}` : '');
  function moensterTekst(m) {
    const t = m.tek ? fmt(T.moensterTek, { t: tekNavn(m.tek) }) : '';
    if (m.slags === 'dns_rekognoscering') return fmt(m.dns_navne ? T.moensterDnsNavne : T.moensterDns, { n: tal(m.stier), m: tal(Math.round(m.normalt)) });
    if (m.slags === 'maalrettet') return fmt(T.moensterMaal, { v: m.verden ? T.moensterVerden : T.moensterAndre, p: tal(m.procent), t }) + (m.foreloebig ? ` ${T.moensterForeloebig}` : '');
    return fmt(T.moensterRekog, { n: tal(m.stier), m: tal(Math.round(m.normalt)), s: m.scanner ? T.moensterScanner : '', t });
  }
  const erNet = e => Boolean(e && e.kode === 'forbindelse');
  function fejl(e) {
    const k = e && typeof e === 'object' && typeof e.kode === 'string' && e.kode in T.fejl ? e.kode : 'intern';
    const skabelon = k === 'graphql' && !e.dag ? T.fejl.graphqlUdenDag : T.fejl[k];
    const t = skabelon.replace(/\{(\w+)\}/g, (m, n) => (n === 'dag' ? dato(e.dag) : String(e[n] == null ? '' : e[n])));
    return k === 'graphql' && e.detalje ? `${t} (${e.detalje})` : t;
  }
  const ordliste = ORDLISTE.map(o => [o.id, (o.re.flags.includes('i') ? '(?i)' : '') + o.re.source, o.da, o.en, o.raadDa, o.raadEn]);
  /* Det, Cloudflare stoppede, er angreb; det øvrige er ikke (notify::trafik_tekst). Stoppede den mindst halvdelen, siger sætningen det. */
  const trafikStop = (n, s) => fmt(2 * s >= n ? T.trafikStopMest : T.trafikStop, { s: tal(s), o: tal(n - s) });
  const tekNavn = t => (t === 'andet' ? T.tekAndet : TEK[t] || String(t).split(' ').map(o => o.charAt(0).toUpperCase() + o.slice(1)).join(' '));
  window.hmSprog = { lang, T, anvend, flertal, fmt, trafikStop, tekNavn, datoPar, get domaene() { return domaene; }, fejl, erNet, dato, datoLang, datoKort, maaned, datoTid, moensterTekst, maerkeNavn, get datoValg() { return datoValg; }, kl, tid, tal, stoerrelse, periode, saetDomaene, saetLand, titel, klar, rutineNavn, kortEgen, bog, ordliste, bogIds: Object.keys(BOG) };
})();
