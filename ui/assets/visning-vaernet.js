/* Huginn and Muninn — Angreb (Værnet): hvad Cloudflares edge holdt ude fra dine zoner. Én visning i rammen:
   #vaernet?zone=<id>&navn=… for ét domæne, #vaernet?samlet=1 for de valgte tilsammen; domænevælgeren i båndet skifter (et valg er ét trin i historikken).
   Tegningen er en port af W assets/vagt.js (MIT, samme ophavsmand); kort, søjler og lister står nu i vg-tegn.js og deles med dyk.
   Tallene kommer fra programmets Rust-del (vaern_cache = de gemte tal med det samme, vaern_oversigt = hent nye fra Cloudflare;
   samlet: vaern_samlet_cache) og kortdata ligeså. Webvisningen henter intet over netværket selv, og der er ingen eksempeldata.
   Ingen IP-adresser; stierne er renset i Rust. Fejl er koder fra Rust, oversat af hmSprog.fejl. Uden forbindelse prøver
   visningen selv igen (genforsoeg.js). Fliser, søjler, lister, kort og regelkort viser hele UTC-dage (i går, 7 og 30 hele dage: felterne
   h_*) og kun det afviste, så tallet er det samme i dyk; siderne med skjulte fælde-links står på deres egen linje. Alt med data bag
   åbner dyk (data-dyk, rammens lytter): fliser, fordeling, kort, søjler, lister, regelkort, "Hvad de ledte efter" og "De seneste".
   vaern() tegner skelettet; alt, der kan klikkes på, sættes ind i dets pladser af fyld() med createElement.
   Det samlede Angreb tegnes igen, når køen melder nyt (rammens 'hentekoe'), højst hvert 3. sekund. Ét domæne har afsnittet "Dit sites teknologi" (teknologi_hent), og
   #vaernet?…&vis=teknologi ruller dertil. Valgene sættes på #teknologi. "Hvad angrebene var rettet mod" står både for ét domæne og samlet. Dage i uhentet er huller: de tegnes ikke som 0, og summer siger "N af M dage".
   start(params)/stop(): et skift af domæne eller visning stopper alt fra det forrige (lyttere, ure, genforsøg), og svar,
   der kommer for sent, kasseres (gen). De valgte domæner står som etiketter under titlen (visValgte, createElement). */
(function () {
  'use strict';
  const rod = document.getElementById('vagt');
  const R = window.hmRamme, Sp = window.hmSprog, V = window.hmTegn;
  const da = document.documentElement.lang !== 'en', loc = da ? 'da-DK' : 'en-GB';
  const invoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
  let zoneId = '', samlet = false, domaene = '…', vis = null, samletValg = [];
  const esc = s => String(s == null ? '' : s).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const tal = n => Number(n || 0).toLocaleString(loc);
  const fmt = Sp.fmt;
  const fejltekst = window.hmSprog.fejl, erNet = window.hmSprog.erNet;
  const landNavn = V.land, coloNavn = V.colo, datoLang = V.datoLang, { el } = R;

  const T = da ? {
    henter: 'Henter tallene fra Cloudflare …', opdateret: 'Hentet fra Cloudflare {t}', opdaterer: 'Viser de gemte tal fra {t} · henter nye fra Cloudflare …',
    fejl: 'Tallene kunne ikke hentes: {f}', fejlGemte: 'Nye tal kunne ikke hentes: {f} Visningen viser de senest gemte tal fra {t}.', ingenZone: 'Intet domæne er valgt. Vælg et i listen.',
    mangler: 'Programmets forbindelse til Rust-delen mangler. Start det med npm run tauri dev.',
    d1: 'Afvist i går', d1Sub: 'i dag hidtil: {n}', d7: 'Afvist på 7 dage', d30: 'Afvist på 30 dage', andel: '1 af hver {n} {n|forespørgsel|forespørgsler}', forsoeg: 'Snusere, der intet fandt (7 dage)', forsoegSub: 'forespørgsler efter filer, der ikke findes',
    fordI: 'I går', ford30: '30 dage', ford: 'heraf {b} blokeret · {u} udfordret · {l} lokket ind i labyrinten', fordAndet: ' · og {a} andet, som ikke tæller som afvist',
    fordNote: 'Cloudflare stopper på flere måder: blokerer, udfordrer (et browsertjek, som en robot ikke klarer) eller lokker robotten ind i en labyrint af falske sider. Sider med skjulte fælde-links blev leveret og tæller ikke som afvist; først når en robot følger et link, er den lokket ind. Hele dage regnes i UTC.',
    kortH: 'Hvor angrebene kom fra', kortP: 'Hver bue går fra det land, angrebet kom fra, til det datacenter i Cloudflares edge, der stoppede det. Det sker i det datacenter, der ligger nærmest angriberen, længe før noget når dit site.',
    kortTitel: 'Verdenskort over afviste angreb de seneste 30 dage', kortDesc: 'Flest kom fra {lande}. Flest blev stoppet i {colos}.',
    verden: 'Verden', europa: 'Europa', leg: { land: 'Land, angrebet kom fra', colo: 'Datacenter, der stoppede det', bue: 'Et angreb på vej', samme: '◆ i en ring = stoppet i samme land, som angrebet kom fra' },
    lande: 'Lande', netvaerk: 'Netværk', colos: 'Afvist i',
    dageH: 'Edge · 30 dage', dageP: 'Første forsvarslinje: Cloudflares edge, som al trafik møder, før den når din server. Afviste forespørgsler pr. dag (UTC).', dageTitel: 'Søjlediagram over afviste forespørgsler pr. dag de seneste 30 dage', ialt: 'i alt', maks: 'flest: {n} den {d}',
    vis: 'Vis tallene', dag: 'Dag', antal: 'Afvist', trafik: 'Alle forespørgsler',
    reglerH: 'Reglerne, der slog til', reglerP: 'Cloudflares forvaltede regelsæt kender angrebsmønstre mod almindelig software. Her er de regler, der stoppede noget på {domaene}.',
    reglerNote: 'Om et angreb ville have virket, afhænger af, hvad sitet kører. Cloudflare stoppede det alligevel, før det nåede frem.',
    n30: 'på 30 dage', n7: '{n} på 7 dage',
    forsoegH: 'Hvad de ledte efter', forsoegP: 'Ikke alt bliver blokeret. Mange robotter går bare rundt og prøver dørhåndtag: de spørger efter filer, som andre steder ved et uheld ligger fremme. Her svarede {domaene} 404: filen fandtes ikke.',
    forsoegPP: 'Ikke alt bliver blokeret. Mange robotter går bare rundt og prøver dørhåndtag: de spørger efter filer, som andre steder ved et uheld ligger fremme. Her svarede dine {domaene} 404: filen fandtes ikke.',
    ingenTalSamlet: 'Ingen af dine domæner har tal endnu. De hentes i baggrunden, og siden tegnes, når de kommer.',
    mAfN: '{m} af {n} {n|domæne|domæner} har tal endnu. Resten hentes i baggrunden.', valgteH: 'Dine domæner', flere: '+ {n} flere', udenTal: 'ingen tal endnu', opdateretSamlet: 'Hentet fra Cloudflare {t} (det ældste af de valgte)', opdatererSamlet: 'Hentet fra Cloudflare {t} (det ældste af de valgte) · henter i baggrunden …',
    senesteH: 'De seneste, der blev afvist', tid: 'Tidspunkt (UTC, hel time)', domaeneKol: 'Domæne', regel: 'Regel', land: 'Land', sti: 'Sti', colo: 'Datacenter', metode: 'Metode',
  } : {
    henter: 'Fetching the figures from Cloudflare …', opdateret: 'Fetched from Cloudflare {t}', opdaterer: 'Showing the saved figures from {t} · fetching new ones from Cloudflare …',
    fejl: 'The figures could not be fetched: {f}', fejlGemte: 'New figures could not be fetched: {f} The view shows the last saved figures from {t}.', ingenZone: 'No domain is chosen. Choose one in the list.',
    mangler: 'The connection to the program’s Rust part is missing. Start it with npm run tauri dev.',
    d1: 'Refused yesterday', d1Sub: 'today so far: {n}', d7: 'Refused in 7 days', d30: 'Refused in 30 days', andel: '1 in every {n} {n|request|requests}', forsoeg: 'Snoopers who found nothing (7 days)', forsoegSub: 'requests for files that do not exist',
    fordI: 'Yesterday', ford30: '30 days', ford: 'of which {b} blocked · {u} challenged · {l} lured into the labyrinth', fordAndet: ' · and {a} other, not counted as refused',
    fordNote: 'Cloudflare stops in several ways: it blocks, challenges (a browser check a robot cannot pass) or lures the robot into a labyrinth of fake pages. Pages with hidden trap links were served and do not count as refused; only when a robot follows a link has it been lured in. Whole days are counted in UTC.',
    kortH: 'Where the attacks came from', kortP: 'Each arc runs from the country an attack came from to the data centre in Cloudflare’s edge that stopped it. That happens in the data centre nearest the attacker, long before anything reaches your site.',
    kortTitel: 'World map of refused attacks in the last 30 days', kortDesc: 'Most came from {lande}. Most were stopped in {colos}.',
    verden: 'World', europa: 'Europe', leg: { land: 'Country the attack came from', colo: 'Data centre that stopped it', bue: 'An attack under way', samme: '◆ inside a ring = stopped in the country the attack came from' },
    lande: 'Countries', netvaerk: 'Networks', colos: 'Refused in',
    dageH: 'Edge · 30 days', dageP: 'First line of defence: Cloudflare’s edge, which all traffic meets before it reaches your server. Refused requests per day (UTC).', dageTitel: 'Bar chart of refused requests per day over the last 30 days', ialt: 'in total', maks: 'most: {n} on {d}',
    vis: 'Show the figures', dag: 'Day', antal: 'Refused', trafik: 'All requests',
    reglerH: 'The rules that fired', reglerP: 'Cloudflare’s managed ruleset knows attack patterns against common software. These are the rules that stopped something on {domaene}.',
    reglerNote: 'Whether an attack would have worked depends on what the site runs. Cloudflare stopped it anyway, before it arrived.',
    n30: 'in 30 days', n7: '{n} in 7 days',
    forsoegH: 'What they were looking for', forsoegP: 'Not everything is blocked. Many robots simply walk around trying door handles: they ask for files that are sometimes left out by accident elsewhere. Here {domaene} answered 404: the file did not exist.',
    forsoegPP: 'Not everything is blocked. Many robots simply walk around trying door handles: they ask for files that are sometimes left out by accident elsewhere. Here your {domaene} answered 404: the file did not exist.',
    ingenTalSamlet: 'None of your domains has figures yet. They are fetched in the background, and the page is drawn when they arrive.',
    mAfN: '{m} of {n} {n|domain|domains} {m|has|have} figures so far. The rest are fetched in the background.', valgteH: 'Your domains', flere: '+ {n} more', udenTal: 'no figures yet', opdateretSamlet: 'Fetched from Cloudflare {t} (the oldest of the chosen)', opdatererSamlet: 'Fetched from Cloudflare {t} (the oldest of the chosen) · fetching in the background …',
    senesteH: 'The latest that were refused', tid: 'Time (UTC, whole hour)', domaeneKol: 'Domain', regel: 'Rule', land: 'Country', sti: 'Path', colo: 'Data centre', metode: 'Method',
  };
  const kl = Sp.kl;

  /* ── fælles byggeklodser ── */
  const blok = (id, h, p, indhold) => `<section class="vg-blok" id="${id}" aria-labelledby="${id}-h"><h2 id="${id}-h">${esc(h)}</h2>${p ? `<p class="vg-intro">${esc(p)}</p>` : ''}${indhold}</section>`;

  /* ── Værnet: skelettet ── */
  function vaern(v) {
    const kortBlok = `<div class="vg-tavle"><div class="vg-zoom" role="group" aria-label="${esc(T.kortH)}"><button type="button" data-vis="verden" aria-pressed="true">${esc(T.verden)}</button><button type="button" data-vis="europa" aria-pressed="false">${esc(T.europa)}</button></div><div class="vg-kortramme" role="region" aria-label="${esc(T.kortTitel)}"></div>
      <ul class="vg-legende" data-plads="legende"><li><i class="vg-leg-land"></i>${esc(T.leg.land)}</li><li><i class="vg-leg-colo"></i>${esc(T.leg.colo)}</li><li><i class="vg-leg-bue"></i>${esc(T.leg.bue)}</li><li>${esc(T.leg.samme)}</li></ul></div>
      <div class="vg-tre"><div><h3>${esc(T.lande)}</h3><div data-plads="lande"></div></div><div><h3>${esc(T.netvaerk)}</h3><div data-plads="netvaerk"></div></div><div><h3>${esc(T.colos)}</h3><div data-plads="colos"></div></div></div>`;
    const dageBlok = `<div class="vg-tavle vg-tavle-pad"><p class="vg-tavle-top" data-plads="dage-top"></p><div class="vg-soejler-rul" role="region" aria-label="${esc(T.dageTitel)}"></div></div><details class="vg-tal" data-noegle="dage"><summary>${esc(T.vis)}</summary><div data-plads="dage-tabel"></div></details>`;
    const regler = `<div class="vg-regler" data-plads="regler"></div><p class="vg-note">${esc(T.reglerNote)}</p>`;
    const vurd = samlet ? '' : '<div data-plads="vurdering"></div>';
    const tek = samlet ? '' : '<section class="vg-blok" id="vg-teknologi" aria-labelledby="vg-teknologi-h"><h2 id="vg-teknologi-h"></h2><div data-plads="teknologi"></div></section>';
    const sen = (v.seneste || []).length ? blok('vg-seneste', T.senesteH, '', '<div data-plads="seneste"></div>') : '';
    return `<div data-plads="fliser"></div><ul class="vg-trafik" data-plads="trafik"></ul><ul class="vg-fordeling" data-plads="fordeling"></ul><p class="vg-note">${esc(T.fordNote)}</p>${vurd}` + blok('vg-kort', T.kortH, T.kortP, kortBlok) + blok('vg-dage', T.dageH, T.dageP, dageBlok) + blok('vg-regler', T.reglerH, fmt(T.reglerP, { domaene }), regler) + tek + '<div data-plads="rettet"></div>' + (samlet ? '' : '<div data-plads="email"></div>') + blok('vg-forsoeg', T.forsoegH, fmt(samlet ? T.forsoegPP : T.forsoegP, { domaene }), '<ol class="vg-protokol" data-plads="forsoeg"></ol>') + sen;
  }

  /* Forespørgslen til dyk: domænet (eller det samlede) og filtrene. */
  const dykTil = x => { const q = new URLSearchParams(samlet ? { samlet: '1' } : { zone: zoneId, navn: domaene }); for (const [k, val] of Object.entries(x)) q.set(k, val); return q.toString(); };
  const knap = V.knap;
  /* i går = sidste søjle (d-1). */
  const iGaar = v => (v.h_dage.length ? v.h_dage[v.h_dage.length - 1].dag : '');
  /* Hentede døgn af de sidste 7 og 30: en sum med huller siger det. */
  const huller = v => new Set(v.uhentet || []);
  const daekning = (v, n) => { const h = huller(v), m = v.h_dage.slice(-n).filter(d => !h.has(d.dag)).length; return m < n ? fmt(Sp.T.dagMed, { m: tal(m), n: tal(n) }) : ''; };
  function fliser(v) {
    const d7 = v.hele7, alle7 = v.alle_hele7, f = v.h_forsoeg || {}, andel = d7 && alle7 ? fmt(T.andel, { n: tal(Math.round(alle7 / d7)) }) : '';
    return el('div', { className: 'vg-fliser' },
      V.flise(v.i_gaar, T.d1, fmt(T.d1Sub, { n: tal(v.i_dag_hidtil) }), dykTil({ dag: iGaar(v) }), { ekstra: { n: v.i_dag_hidtil, dyk: dykTil({ dag: v.h_idag }) } }),
      V.flise(d7, T.d7, [andel, daekning(v, 7)].filter(Boolean).join(' · '), dykTil({ periode: '7' }), { klasse: 'vg-hoved' }),
      V.flise(v.hele30, T.d30, daekning(v, 30), dykTil({})),
      V.flise(f.i7, T.forsoeg, T.forsoegSub, { rul: 'vg-forsoeg' }, { klasse: 'vg-graa', label: fmt(Sp.T.dykRul, { navn: T.forsoeg, n: tal(f.i7) }) }));
  }
  /* Usædvanlig meget trafik: hver dag åbner dagens forespørgsler i trafik-dyk. Tallene kan ikke sige, hvad det var, så det står for sig. */
  function trafikLinjer(v) {
    const dage = v.trafik || [];
    if (!dage.length) return [];
    return [...dage.map(t => el('li', {}, knap(fmt(2 * t.stoppet >= t.n ? Sp.T.trafikDagMest : Sp.T.trafikDag, { d: datoLang(t.dag), n: tal(t.n), s: tal(t.stoppet), o: tal(t.n - t.stoppet) }), dykTil({ dag: t.dag, trafik: '1' }), 'trafik-maerke'))), el('li', { className: 'vg-trafik-note', textContent: fmt(Sp.T.trafikNote, { m: tal(Math.round(dage[0].median)) }) })];
  }
  /* "heraf N blokeret · N udfordret · N lokket ind i labyrinten" for én periode; hver del åbner dyk på den handling. Tallet 0 fører ingen steder hen. */
  const FORD = { b: 'blokeret', u: 'udfordret', l: 'labyrint' };
  function fordelingLinjer(navn, f, periode) {
    if (!f) return [];
    const li = el('li', {}, el('b', { textContent: `${navn}:` }), ' '), maal = handling => dykTil({ ...periode, handling });
    T.ford.split(' · ').forEach((del, i) => {
      const [, foer, k] = /^(.*?)\{(\w)\}/.exec(del), tekst = fmt(del.slice(foer.length), { [k]: tal(f[FORD[k]]) });
      li.append(...(i ? [' · '] : []), foer, f[FORD[k]] ? knap(tekst, maal(FORD[k])) : tekst);
    });
    if (f.andet) li.append(' · ', knap(fmt(T.fordAndet, { a: tal(f.andet) }).replace(/^ · /, ''), maal('andet')));
    const ud = [li];
    if (f.faelder) ud.push(el('li', { className: 'vg-faelder' }, el('b', { textContent: `${navn}:` }), ' ', knap(Sp.T.faelderLinje(tal(f.faelder)), maal('faelder'))));
    return ud;
  }
  /* En regel, der kun er set i dag, åbner dagen i dag, hvor tallet står. */
  function regelKort(rg, v) {
    const vis = V.regelVis(rg), labyrint = rg.navn === 'linkMaze', kunIDag = !rg.h30 && rg.idag > 0, omfang = kunIDag ? { dag: v.h_idag } : {};
    return V.regelKort(rg, {
      dyk: dykTil({ regel: rg.navn, ...omfang }), n: rg.h30, enhed: labyrint ? Sp.T.lokketInd : T.n30,
      label: kunIDag ? fmt(Sp.T.dykAabnRegelIDag, { navn: vis.overskrift, n: tal(rg.idag) }) : fmt(Sp.T.dykAabnRegel, { navn: vis.overskrift, n: tal(rg.h30) }),
      under: kunIDag ? fmt(T.d1Sub, { n: tal(rg.idag) }) : labyrint ? fmt(Sp.T.faelderSat, { n: tal(rg.faelder30) }) : fmt(T.n7, { n: tal(rg.h7) }),
      sti: x => dykTil({ regel: rg.navn, sti: x }), land: k => dykTil({ regel: rg.navn, land: k }),
    });
  }
  /* "Edge · 30 dage": i alt, den travleste dag og tabellen bag "Vis tallene". */
  function dageTop(v) {
    const sumD = v.h_dage.reduce((s, d) => s + d.n, 0), mx = v.h_dage.reduce((a, d) => (d.n > a.n ? d : a), v.h_dage[0] || { n: 0 });
    const b = el('b', { textContent: tal(sumD) }), ud = [sumD ? knap(b, dykTil({}), 'link dyk-celle', fmt(Sp.T.dykAabn, { navn: T.ialt, n: tal(sumD) })) : b, ' ', T.ialt];
    if (mx && mx.n) ud.push(' · ', knap(fmt(T.maks, { n: tal(mx.n), d: datoLang(mx.dag) }), dykTil({ dag: mx.dag })));
    if (daekning(v, 30)) ud.push(' · ', daekning(v, 30));
    return ud;
  }
  function dageTabel(v) {
    const kol = (tekst, klasse) => { const th = el('th', { textContent: tekst, className: klasse || '' }); th.setAttribute('scope', 'col'); return th; };
    const uhentet = huller(v), raekker = v.h_dage.slice().reverse().map(d => {
      if (uhentet.has(d.dag)) return el('tr', { className: 'hul-raekke' }, el('th', { scope: 'row', textContent: datoLang(d.dag) }), el('td', { className: 'tal-kol', textContent: Sp.T.hulIngen }), el('td', { className: 'tal-kol', textContent: Sp.T.hulIngen }));
      const dag = datoLang(d.dag), q = dykTil({ dag: d.dag }), th = el('th', { scope: 'row' }, d.n ? knap(dag, q, 'link dyk-celle', fmt(Sp.T.dykDag, { d: dag, n: tal(d.n) })) : dag);
      return el('tr', {}, th, el('td', { className: 'tal-kol' }, d.n ? knap(tal(d.n), q, 'link dyk-celle', fmt(Sp.T.dykDag, { d: dag, n: tal(d.n) })) : tal(d.n)), el('td', { className: 'tal-kol', textContent: tal(d.alle) }));
    });
    const rul = el('div', { className: 'vg-tabel-rul', tabIndex: 0 }, el('table', { className: 'vg-tabel' }, el('thead', {}, el('tr', {}, kol(T.dag), kol(T.antal, 'tal-kol'), kol(T.trafik, 'tal-kol'))), el('tbody', {}, ...raekker)));
    rul.setAttribute('role', 'region');
    rul.setAttribute('aria-label', T.vis);
    return rul;
  }
  /* "Hvad de ledte efter": 7 hele dage, som flisen; en kategori og en sti åbner forsøget i dyk med samme periode. */
  function forsoegPunkt(k, i, m) {
    const [navn, forkl] = Sp.T.kat[k.id] || [k.id, ''], q = x => dykTil({ forsoeg: x, periode: '7' });
    const li = el('li', {}, el('div', { className: 'vg-p-hoved' }, el('span', { className: 'vg-p-nr', textContent: `§ ${i + 1}` }), el('h3', {}, knap(navn, q(k.id), 'link dyk-celle', fmt(Sp.T.dykAabn, { navn, n: tal(k.n) }))), el('b', { textContent: tal(k.n) })),
      el('i', { className: V.bredde(100 * k.n / m) }), el('p', { textContent: forkl }));
    if (k.stier && k.stier.length) li.append(el('p', { className: 'vg-p-stier' }, ...k.stier.slice(0, 5).flatMap(s => [knap(V.sti(s.sti), q(s.sti), 'link dyk-celle', fmt(Sp.T.dykAabn, { navn: s.sti, n: tal(s.n) })), ' '])));
    return li;
  }
  /* Domænet i det samlede "De seneste" åbner domænets Angreb. Kendes det ikke (mere), står det som tekst. */
  function domaeneCelle(navn) {
    const r = R.tilstand.raekker.find(x => x.navn === navn), td = el('td');
    if (!r) td.append(navn); else td.append(knap(navn, { gaa: 'vaernet?' + new URLSearchParams({ zone: r.id, navn: r.navn }) }, 'link dyk-celle', fmt(Sp.T.aabnFor, { navn })));
    return td;
  }
  /* Kort, søjler, lister og regelkort sættes ind i de pladser, vaern() har lavet. */
  const kortOpt = v => ({
    titel: `${T.kortTitel}. ${Sp.T.tastatur}`, id: 'vg-kort-d', beskrivelse: fmt(T.kortDesc, { lande: v.h_lande.slice(0, 3).map(l => landNavn(l.kode)).join(', '), colos: v.h_colos.slice(0, 3).map(c => coloNavn(c.kode)).join(', ') }),
    dyk: { land: k => dykTil({ land: k }), colo: k => dykTil({ colo: k }), bue: (l, c) => dykTil({ land: l, colo: c }) },
  });
  /* "Dit sites teknologi": kun valgene og forslaget (hmTegn.tekKort); de sættes på #teknologi. De generiske slags har intet valg og står for sig. */
  async function tegnTeknologi() {
    const plads = rod.querySelector('[data-plads="teknologi"]'), g = gen, S = Sp.T;
    if (!plads) return;
    rod.querySelector('#vg-teknologi-h').textContent = S.tekH;
    let d;
    try { d = await invoke('teknologi_hent', { zone: zoneId }); } catch (e) { return; }
    if (g !== gen || !plads.isConnected) return;
    const saet = async (tek, valg) => { try { await invoke('teknologi_saet', { zone: zoneId, tek, valg, besked: false }); await tegnTeknologi(); R.hentOverblik(); } catch (e) { /* valget står */ } };
    const dele = V.tekKort({ zone: zoneId, navn: domaene, linjer: d.linjer, saet });
    if (d.generiske.length) {
      const liste = el('ul', { className: 'tek-generiske' }, ...d.generiske.map(x => { const n = (S.kat[x.id] || [x.id])[0]; return el('li', {}, knap(`${n}: ${fmt(S.tekForsoeg, { n: tal(x.forsoeg30) })}`, dykTil({ forsoeg: x.id }))); }));
      dele.push(el('h3', { textContent: S.tekGeneriskH }), el('p', { className: 'vg-note', textContent: S.tekGeneriskP }), liste);
    }
    plads.replaceChildren(...dele);
    if (vis === 'teknologi') {
      /* Tallene fra cachen tegnes, mens de friske er på vej. Først de friske er sidste tegning. */
      if (!rod.classList.contains('vg-henter')) vis = null;
      const s = rod.querySelector('#vg-teknologi');
      if (s && s.scrollIntoView) s.scrollIntoView({ block: 'start' });
    }
  }
  /* "Hvad angrebene var rettet mod" for de 30 hele dage: samme tal som dyk, og kun angreb. Fejler opslaget, står afsnittet ikke. */
  async function tegnRettet() {
    const plads = rod.querySelector('[data-plads="rettet"]'), g = gen;
    if (!plads) return;
    let d;
    try { d = await invoke('dyk', { omfang: samlet ? { zoner: samletValg } : { zone: zoneId }, filtre: [], periode: 'd30', slags: 'angreb' }); } catch (e) { return; }
    if (g !== gen || !plads.isConnected || !d || !(d.rettet_mod || []).length) return;
    plads.replaceChildren(V.blok(Sp.T.rettetH, Sp.T.rettetP, V.rettetMod(d.rettet_mod, t => dykTil({ teknologi: t, slags: 'angreb' }))));
  }
  /* "Mails sendt i dit domænes navn", kun fra cachen. Fejler opslaget, står afsnittet ikke. Rækkerne har ikke mere bag sig, så intet her er klikbart ud over guiden. */
  function emailBlok(d) {
    const T = Sp.T, fod = [el('p', { className: 'vg-note', textContent: T.emailNote })];
    const guide = () => { const b = Sp.bog('dmarc_til'); return el('details', { className: 'dyk-guide' }, el('summary', { textContent: T.emailGuide }), el('p', { textContent: b.alle }), el('ol', {}, ...b.ja.map(t => el('li', { textContent: t })))); };
    if (!d.mails) {
      const linje = d.fejl ? fmt(T.emailFejl, { f: d.fejl }) : d.hentede_dage ? T.emailTom : T.emailVenter;
      return V.blok(T.emailH, '', el('p', { className: 'email-tom', textContent: linje }), ...(d.hentede_dage && !d.fejl ? [guide()] : []));
    }
    const top = el('p', { className: 'email-linje', textContent: d.fejlet ? fmt(T.emailLinje, { n: tal(d.mails), m: tal(d.fejlet), k: d.afsendere }) : fmt(T.emailLinjeOk, { n: tal(d.mails) }) });
    if (!d.liste.length) return V.blok(T.emailH, T.emailP, top);
    const th = (t, k) => { const e = el('th', { textContent: t, className: k || 'v' }); e.setAttribute('scope', 'col'); return e; };
    const rk = d.liste.map(x => el('tr', {}, el('td', { className: 'v', textContent: x.afsender || T.emailUkendt }), el('td', { className: 'v', textContent: x.land || '–' }), el('td', { className: 'tal-kol', textContent: tal(x.fejlet) }), el('td', { className: 'tal-kol', textContent: tal(x.mails) })));
    const rul = el('div', { className: 'vg-tabel-rul', tabIndex: 0 }, el('table', { className: 'vg-tabel email-tabel' }, el('thead', {}, el('tr', {}, th(T.emailAfsender), th(T.emailLand), th(T.emailFejlet, 'tal-kol'), th(T.emailMails, 'tal-kol'))), el('tbody', {}, ...rk)));
    rul.setAttribute('role', 'region');
    rul.setAttribute('aria-label', T.emailH);
    return V.blok(T.emailH, T.emailP, top, rul, ...fod);
  }
  async function tegnEmail() {
    const plads = rod.querySelector('[data-plads="email"]'), g = gen;
    if (!plads) return;
    let d;
    try { d = await invoke('email_oversigt', { zone: zoneId }); } catch (e) { return; }
    if (g !== gen || !plads.isConnected || !d) return;
    plads.replaceChildren(emailBlok(d));
  }
  const kortData = v => ({ lande: v.h_lande, colos: v.h_colos, buer: v.h_buer });
  const LISTE = 8;
  /* Mærket "Ligner …" øverst med forklaringen: klik åbner dagen, og linket guiden. */
  function tegnMoenster() {
    rod.querySelector('.moenster-linje')?.remove();
    rod.querySelector('.moenster-note')?.remove();
    if (R.tilstand.raekker.some(x => x.kan_sammenligne === false)) rod.prepend(el('p', { className: 'vg-note moenster-note', textContent: Sp.T.maalMin }));
    const r = samlet ? null : R.tilstand.raekker.find(x => x.id === zoneId), b = r && V.moensterKnap(r);
    if (!b) return;
    const guide = el('button', { type: 'button', className: 'link', textContent: Sp.T.maerkeGuide });
    guide.addEventListener('click', () => R.guide(r.id));
    rod.prepend(el('p', { className: `moenster-linje moenster-linje-${r.moenster.slags}` }, b, ' ', Sp.moensterTekst(r.moenster), ' ', guide));
  }
  function fyld(v) {
    const plads = id => rod.querySelector(`[data-plads="${id}"]`), f = v.fordeling || {}, fk = (v.h_forsoeg && v.h_forsoeg.kategorier) || [], fm = Math.max(1, ...fk.map(k => k.n));
    plads('fliser').replaceChildren(fliser(v));
    tegnMoenster();
    if (!samlet) tegnTeknologi();
    tegnRettet();
    tegnEmail();
    if (!samlet && iGaar(v)) plads('vurdering').replaceChildren(V.vurderingBoks({ zone: zoneId, navn: domaene, dag: iGaar(v) }));
    plads('trafik').replaceChildren(...trafikLinjer(v));
    plads('fordeling').replaceChildren(...fordelingLinjer(T.fordI, f.i_gaar, { dag: iGaar(v) }), ...fordelingLinjer(T.ford30, f.d30, {}));
    rod.querySelector('.vg-kortramme').replaceChildren(V.kort(kortData(v), 'verden', kortOpt(v)));
    const uden = V.udenPlads(kortData(v)), legende = plads('legende'), skala = V.kortSkala(kortData(v).lande);
    legende.querySelector('.vg-skala')?.remove();
    if (skala) legende.prepend(skala);
    if (uden.length) legende.append(el('li', { textContent: fmt(Sp.T.kortUdenPlads, { liste: uden.join(', ') }) }));
    if (v.h_buer_i_alt > v.h_buer.length) legende.append(el('li', { textContent: fmt(Sp.T.kortBuer, { n: tal(v.h_buer.length), m: tal(v.h_buer_i_alt) }) }));
    plads('dage-top').replaceChildren(...dageTop(v));
    rod.querySelector('.vg-soejler-rul').replaceChildren(V.soejler(v.h_dage.map(d => ({ ...d, hentet: !huller(v).has(d.dag) })), { titel: `${T.dageTitel}. ${Sp.T.tastatur}`, dyk: dag => dykTil({ dag }) }));
    plads('dage-tabel').replaceChildren(dageTabel(v));
    plads('lande').replaceChildren(...V.listeFlere(v.h_lande.map(l => ({ navn: landNavn(l.kode), n: l.n, dyk: dykTil({ land: l.kode }) })), LISTE, 'lande', fmt(Sp.T.listeAlle, { n: tal(v.h_lande.length) })));
    plads('netvaerk').replaceChildren(V.liste(v.h_netvaerk.slice(0, LISTE).map(l => ({ navn: l.navn, n: l.n, dyk: dykTil({ netvaerk: l.navn }) }))));
    plads('colos').replaceChildren(...V.listeFlere(v.h_colos.map(c => ({ navn: coloNavn(c.kode), n: c.n, dyk: dykTil({ colo: c.kode }) })), LISTE, 'colos', fmt(Sp.T.listeAlle, { n: tal(v.h_colos.length) })));
    plads('regler').replaceChildren(...v.h_regler.map(rg => regelKort(rg, v)));
    plads('forsoeg').replaceChildren(...fk.map((k, i) => forsoegPunkt(k, i, fm)));
    const sen = plads('seneste'), kilde = new Map(v.h_regler.map(rg => [rg.navn, rg.art]));
    if (sen) sen.replaceChildren(V.senesteTabel(v.seneste, { til: (dim, val) => dykTil({ [dim]: val }), domaene: domaeneCelle, cve: true, art: x => kilde.get(x) }));
  }

  /* ── "Se de N angreb" (vis=angreb-i-gaar): "De seneste" kun med dagens egentlige angreb ── */
  let filter = false, sidsteTegning = null, filterNat = null;
  /* Dagen er statusdagen: i går, eller om natten i forgårs (som Overblikket). null = intet at filtrere (fx samlet). */
  function filterDag(data) {
    if (!filter || samlet || !Array.isArray(data.angreb_dage) || !data.angreb_dage.length) return null;
    const r = R.tilstand.raekker.find(x => x.id === zoneId), nat = Boolean(r && r.opdateres);
    filterNat = nat;
    const d = data.angreb_dage[nat ? 1 : 0];
    return d ? { ...d, ord: nat ? Sp.T.dagIForgaars : Sp.T.dagIGaar } : null;
  }
  /* Data til tegningen og chippens tekst. Har cachen ingen enkelte angreb for dagen, vises reglerne bag dem i stedet. */
  function filtrer(data, d) {
    const k = d.seneste.length, n = d.angreb;
    if (k) return { data: { ...data, seneste: d.seneste }, maal: 'vg-seneste', tekst: fmt(k < n ? Sp.T.fNogle : Sp.T.fAlle, { k, n: tal(n), dag: d.ord }) };
    const navne = new Set(d.regler.map(x => x.navn));
    if (navne.size) return { data: { ...data, seneste: [], h_regler: data.h_regler.filter(rg => navne.has(rg.navn)) }, maal: 'vg-regler', tekst: fmt(Sp.T.fRegler, { n: tal(n), dag: d.ord }) };
    return { data: { ...data, seneste: [] }, maal: 'vg-regler', tekst: fmt(Sp.T.fIngen, { dag: d.ord }) };
  }
  function visFilterChip(f) {
    const maal = document.getElementById(f.maal);
    if (!maal) return;
    const chip = document.createElement('p'), tekst = document.createElement('span'), x = document.createElement('button');
    chip.className = 'vg-filter';
    chip.id = 'vg-filter';
    chip.setAttribute('role', 'status');
    tekst.textContent = f.tekst;
    x.type = 'button';
    x.className = 'luk';
    x.textContent = '×';
    x.setAttribute('aria-label', Sp.T.fFjern);
    x.title = Sp.T.fFjern;
    x.addEventListener('click', () => R.gaaTil('vaernet', new URLSearchParams({ zone: zoneId, navn: domaene })));
    chip.append(tekst, ' ', x);
    maal.before(chip);
    if (maal.scrollIntoView && !R.genbesoeg()) maal.scrollIntoView({ block: 'start' }); // tilbage/frem gendanner selv rullepositionen
  }

  /* ── tegning og start ── */
  function besked(tekst, farve = 'roed') {
    rod.removeAttribute('aria-busy');
    rod.innerHTML = `<p class="vg-advarsel vg-${farve}" role="alert">${esc(tekst)}</p>`;
  }
  /* advarsel: [tekst, 'gul' | 'roed'] eller null. Gul prik, når tallene ikke er friske (henter, eller hentningen fejlede). */
  function tegn(alle, status, henter, advarsel) {
    sidsteTegning = [alle, status, henter, advarsel];
    const d = filterDag(alle), f = d ? filtrer(alle, d) : null, data = f ? f.data : alle, genaabn = V.husk(rod);
    rod.innerHTML = `<div class="vg-bjaelke"><p class="vg-status">${esc(status)}</p></div>${advarsel ? `<p class="vg-advarsel vg-${advarsel[1]}" role="alert">${esc(advarsel[0])}</p>` : ''}` + vaern(data);
    fyld(data);
    genaabn();
    rod.querySelectorAll('.vg-soejler-rul').forEach(r => { r.scrollLeft = r.scrollWidth; });
    rod.classList.toggle('vg-henter', henter || Boolean(advarsel));
    if (henter) rod.setAttribute('aria-busy', 'true'); else rod.removeAttribute('aria-busy');
    const ramme = rod.querySelector('.vg-kortramme');
    const midt = () => { if (ramme.scrollWidth > ramme.clientWidth) ramme.scrollLeft = (ramme.scrollWidth - ramme.clientWidth) * 0.5; };
    midt();
    rod.querySelectorAll('.vg-zoom button').forEach(b => b.addEventListener('click', () => {
      rod.querySelectorAll('.vg-zoom button').forEach(x => x.setAttribute('aria-pressed', String(x === b)));
      ramme.replaceChildren(V.kort(kortData(data), b.getAttribute('data-vis'), kortOpt(data))); midt();
    }));
    if (f) visFilterChip(f);
  }

  /* ── etiketterne ── */
  /* De valgte domæner som etiketter (P2): de første 8, resten bag "+ N flere". Hver etiket åbner domænets eget Angreb.
     Domæner uden tal står stiplet. */
  const VALGTE_SYNLIGE = 8;
  function visValgte(valgte) {
    const boks = document.getElementById('valgte');
    boks.replaceChildren();
    boks.hidden = !valgte.length;
    const liste = rk => {
      const ul = document.createElement('ul');
      ul.className = 'hm-valgte-liste';
      for (const v of rk) {
        const a = document.createElement('a');
        a.href = '#vaernet?' + new URLSearchParams({ zone: v.id, navn: v.navn });
        a.textContent = v.navn;
        const li = document.createElement('li');
        if (!v.med_tal) {
          li.className = 'hm-valgt-uden';
          const sr = document.createElement('span');
          sr.className = 'sr-only';
          sr.textContent = ` (${T.udenTal})`;
          a.append(sr);
        }
        li.append(a);
        ul.append(li);
      }
      return ul;
    };
    const synlige = liste(valgte.slice(0, VALGTE_SYNLIGE));
    synlige.setAttribute('aria-label', T.valgteH);
    boks.append(synlige);
    const rest = valgte.slice(VALGTE_SYNLIGE);
    if (rest.length) {
      const flere = document.createElement('details'), sum = document.createElement('summary');
      flere.className = 'hm-valgte-flere';
      sum.textContent = fmt(T.flere, { n: rest.length });
      flere.append(sum, liste(rest));
      boks.append(flere);
    }
  }
  function visDomaene(tekst, flertal) {
    domaene = tekst;
    Sp.saetDomaene(tekst, flertal);
    document.getElementById('vaernet-under').textContent = samlet && !flertal ? '' : tekst;
    R.titel();
  }

  /* ── start og stop ── */
  let gen = 0, aktiv = null, sidste = 'samlet=1', afmeld = [];
  function stop() {
    gen++;
    aktiv = null;
    for (const f of afmeld.splice(0)) f();
  }
  async function start(params) {
    stop();
    const g = gen;
    aktiv = params.toString();
    sidste = aktiv;
    zoneId = params.get('zone') || '';
    samlet = params.get('samlet') === '1';
    filter = params.get('vis') === 'angreb-i-gaar';
    vis = params.get('vis');
    sidsteTegning = null;
    filterNat = null;
    afmeld.push(R.paa('raekker', () => { if (rod.querySelector('[data-plads="fliser"]')) tegnMoenster(); }));
    // Overblikket (og dermed "opdateres") kan komme efter første tegning: tegn igen, hvis statusdagen skifter.
    if (filter) afmeld.push(R.paa('raekker', () => {
      const r = R.tilstand.raekker.find(x => x.id === zoneId);
      if (sidsteTegning && filterNat !== null && Boolean(r && r.opdateres) !== filterNat) tegn(...sidsteTegning);
    }));
    visDomaene(samlet ? '…' : (params.get('navn') || '').slice(0, 253) || '…', samlet);
    R.visVaelger();
    document.getElementById('valgte').hidden = true;
    document.getElementById('valgte').replaceChildren();
    rod.setAttribute('aria-busy', 'true');
    rod.innerHTML = `<p class="vg-status">${esc(T.henter)}</p>`;
    if (!invoke) return besked(T.mangler);
    if (!samlet && !zoneId) return besked(T.ingenZone);
    try { await V.klar(); } catch (e) { if (g === gen) besked(fmt(T.fejl, { f: fejltekst(e) })); return; }
    if (g !== gen) return;
    if (samlet) return startSamlet(g);
    const prioriter = () => invoke('koe_prioritet', { zone: zoneId }).catch(() => { });
    prioriter(); // zonen på skærmen kommer først i køen
    window.addEventListener('online', prioriter);
    afmeld.push(() => window.removeEventListener('online', prioriter));
    const zone = zoneId;
    const gemte = () => invoke('vaern_cache', { zoneId: zone }).catch(() => null);
    const foer = await gemte();
    if (g !== gen) return;
    if (foer) tegn(foer, fmt(T.opdaterer, { t: kl(foer.genereret) }), true);
    /* Ét forsøg på at hente nye tal: 'ok', 'net' (prøv igen) eller 'stop'. Svar til et domæne, der ikke vises mere, kasseres. */
    async function hent() {
      try {
        const ny = await invoke('vaern_oversigt', { zoneId: zone });
        if (g !== gen) return 'stop';
        tegn(ny, fmt(T.opdateret, { t: kl(ny.genereret) }), false);
        return 'ok';
      } catch (e) {
        if (g !== gen) return 'stop';
        const net = erNet(e), efter = await gemte(); // dage, der nåede at komme hjem før fejlen, er gemt
        if (g !== gen) return 'stop';
        if (efter) tegn(efter, fmt(T.opdateret, { t: kl(efter.genereret) }), false, [fmt(T.fejlGemte, { f: fejltekst(e), t: kl(efter.genereret) }), net ? 'gul' : 'roed']);
        else besked(fmt(T.fejl, { f: fejltekst(e) }));
        return net ? 'net' : 'stop';
      }
    }
    const gf = window.hmGenforsoeg(document.getElementById('vg-genforsoeg'), hent);
    afmeld.push(gf.fjern);
    await gf.start();
  }
  /* Det samlede Angreb: valget fra Rust, tallene fra cachen; køen henter, og visningen tegnes igen, når den melder nyt. */
  async function startSamlet(g) {
    let valg;
    try { valg = await invoke('valg_hent'); } catch (e) { if (g === gen) besked(fmt(T.fejl, { f: fejltekst(e) })); return; }
    if (g !== gen) return;
    samletValg = valg;
    if (!valg.length) { visDomaene(Sp.T.domaener(0), true); rod.removeAttribute('aria-busy'); rod.replaceChildren(window.hmVelkomst.intetValgt()); return; }
    visDomaene(Sp.T.domaener(valg.length), true);
    async function tegnSamlet() {
      let s, k = null;
      try { s = await invoke('vaern_samlet_cache', { zoner: valg }); } catch (e) { if (g === gen) besked(fmt(T.fejl, { f: fejltekst(e) })); return; }
      try { k = await invoke('koe_status'); } catch (e) { k = null; }
      if (g !== gen) return;
      visValgte(s.valgte || []);
      const henter = Boolean(k && (k.tilstand === 'henter' || k.tilstand === 'pause' || k.tilstand === 'venter_kvote'));
      if (!s.vaern) return besked(T.ingenTalSamlet, 'gul');
      const advarsel = s.med_tal < s.domaener ? [fmt(T.mAfN, { m: s.med_tal, n: s.domaener }), 'gul'] : null;
      tegn(s.vaern, fmt(henter ? T.opdatererSamlet : T.opdateretSamlet, { t: kl(s.vaern.genereret) }), henter, advarsel);
    }
    await tegnSamlet();
    if (g !== gen) return;
    let sidst = Date.now(), ventende = null;
    const snart = () => {
      if (ventende) return;
      ventende = setTimeout(() => { ventende = null; sidst = Date.now(); tegnSamlet(); }, Math.max(0, sidst + 3000 - Date.now()));
    };
    afmeld.push(R.paa('hentekoe', p => { p = p || {}; if (!p.zone || valg.includes(p.zone)) snart(); }), () => clearTimeout(ventende));
  }
  R.registrer('vaernet', {
    domaener: true,
    hvad: () => ({ samlet, zone: zoneId, navn: domaene }),
    skift: q => R.gaaTil('vaernet', q),
    /* #vaernet uden parametre: det sidst viste (eller det samlede). Samme adresse igen (fx efter "Glem"/nyt token): bliv. */
    vis(params, { samme } = {}) {
      const p = params.toString() ? params : new URLSearchParams(sidste);
      if (samme && aktiv === p.toString()) return;
      start(p);
    },
    skjul: stop,
    sidste: () => sidste,
    aktiv: () => aktiv,
  });
})();
