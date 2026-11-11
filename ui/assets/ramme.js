/* Huginn and Muninn — programrammen. Én side med hash-navigation: #overblik, #siden-sidst, #vaernet?zone=<id>&navn=…
   (eller ?samlet=1; perioden og filtrene står i samme adresse), #robotter?zone=…, #login?zone=…, #land?kode=DK (visning-land.js), #besoegende?zone=… og #indstillinger. Uden token står opsætningen i indholdsområdet, og visningerne er slået fra (kun
   Indstillinger kan åbnes). Rammen ejer den delte tilstand (token, zoner, overblikkets rækker, valget, køens status) og er
   den eneste, der lytter på Rust-hændelserne: 'hentekoe' (køen), 'siden_sidst' (menulinjen/en notifikation), 'visning'
   (programmenuen på macOS: ⌘1–7, ⌘, og Om), 'historik' (Vis → Tilbage/Frem), 'ekstra' (de ekstra tal for et domæne er gemt) og 'radar' (nye tal fra Radar). Visningerne (visning-*.js) registrerer sig med
   vis()/skjul() og abonnerer på rammens egne beskeder (paa → afmeld), så et skift aldrig efterlader lyttere. domaener: true
   på en visning lader domænetræet i navigationsruden styre den; visningen svarer med hvad() og skift(). Domænevalget er fælles (tilstand.domaeneValg, gemt i Rust som domaene_valg): enhver adresse med zone eller samlet=1 sætter det, og sidebjælken, ⌘ og
   programmenuen åbner visningen med det. Historikken beholder sin egen adresse. land: true viser landevælgeren, som visningen selv fylder. data-dyk, data-gaa, data-cve, data-radar og data-rul på et
   element gør det klikbart: én lytter her (klik, Enter, mellemrum), så det overlever, at visningerne tegner om. Et mål i hjælpeskuffen
   lukker skuffen. I en tegning (SVG) flytter piletasterne mellem mærkerne, så tegningen kun er ét Tab-stop; .dyk-hit er den usynlige,
   større klikflade under et mærke og tæller ikke med. Genveje: på macOS står de i programmenuen (Rust), på Windows/Linux fanger
   rammen Ctrl+1–7, Ctrl+, og Ctrl+R/F5 (Hent nu). Tilbage/frem: ‹ › i båndet, ⌘[ ⌘] (menuen), ⌘← ⌘→ og Alt+← Alt+→ (keydown).
   Tekststørrelse: ⌘+ ⌘− ⌘0 (Ctrl på Windows) i keydown på alle platforme, og i programmenuen (hændelsen 'tekst').
   <html data-platform> sættes af Rust (init-script).
   Vinduets titel følger visningen (document.title; Rust sætter vinduets titel ud fra den). Alt bygges med createElement/
   textContent: ingen innerHTML, ingen style-attributter. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T;
  const $ = id => document.getElementById(id);
  const invoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
  const listen = window.__TAURI__ && window.__TAURI__.event && window.__TAURI__.event.listen;
  const fmt = S.fmt;
  const el = (navn, egenskaber, ...boern) => { const e = Object.assign(document.createElement(navn), egenskaber || {}); e.append(...boern); return e; };
  const status = (e, tekst, klasse) => { e.className = 'r6-status' + (klasse ? ' ' + klasse : ''); e.textContent = tekst || ''; };

  const platform = window.__HM_PLATFORM__ || document.documentElement.getAttribute('data-platform') || '';
  if (platform) document.documentElement.setAttribute('data-platform', platform);
  const mac = platform === 'macos';

  /* ── delt tilstand og beskeder ── */
  const tilstand = { sat: false, gemte: null, raekker: [], domaener: [], uvalgt: new Set(), koe: null, siden: null, huller: null, visning: null, params: new URLSearchParams(), domaeneValg: 'samlet' };
  const abonnenter = {};
  const paa = (navn, fn) => { (abonnenter[navn] ||= new Set()).add(fn); return () => abonnenter[navn].delete(fn); };
  const udsend = (navn, data) => { for (const fn of [...(abonnenter[navn] || [])]) { try { fn(data); } catch (e) { console.error(e); } } };
  const antalAbonnenter = navn => (abonnenter[navn] ? abonnenter[navn].size : 0);

  /* ── skallen (Wi24rd UI Kit 2, kit.js): bånd uden faner, visningsskift, landevælger og sidepanel. Kittet bygger elementerne;
     rammen giver dem de id'er, resten af programmet kender (hent-nu, band-land, nav-land, siden-tal, hjaelp-luk). ── */
  const Kit = window.Kit;
  Kit.registrerIkoner({
    'hm-overblik': [['rect', { x: 3, y: 3, width: 7, height: 7, rx: 1.5 }], ['rect', { x: 14, y: 3, width: 7, height: 7, rx: 1.5 }], ['rect', { x: 3, y: 14, width: 7, height: 7, rx: 1.5 }], ['rect', { x: 14, y: 14, width: 7, height: 7, rx: 1.5 }]],
    'hm-siden': [['circle', { cx: 12, cy: 12, r: 9 }], ['path', { d: 'M12 7v5l3 2' }]],
    'hm-robot': [['rect', { x: 5, y: 8, width: 14, height: 11, rx: 2 }], ['path', { d: 'M12 4v4M9 13h.01M15 13h.01' }]],
    'hm-klode': [['circle', { cx: 12, cy: 12, r: 9 }], ['path', { d: 'M3 12h18M12 3c2.5 2.5 3.8 5.5 3.8 9s-1.3 6.5-3.8 9c-2.5-2.5-3.8-5.5-3.8-9S9.5 5.5 12 3z' }]],
    'hm-besoeg': [['circle', { cx: 9, cy: 8, r: 3.5 }], ['path', { d: 'M2.5 20c0-3.6 2.9-6.5 6.5-6.5s6.5 2.9 6.5 6.5' }], ['path', { d: 'M15.5 4.8a3.5 3.5 0 0 1 0 6.4M17.5 14c2.4.8 4 3.1 4 6' }]],
    'hm-hent': [['path', { d: 'M20 11a8 8 0 1 0-2.3 5.7' }], ['path', { d: 'M20 4v7h-7' }]],
  });
  /* De syv visninger i navigationsruden: tast, visning, tekstnøgle, ikon. Indstillinger står i titellinjen (tandhjulet). */
  const SKIFT = [['1', 'overblik', 'navOverblik', 'hm-overblik'], ['2', 'siden-sidst', 'navSiden', 'hm-siden'], ['3', 'vaernet', 'navVaernet', 'skjold'], ['4', 'robotter', 'navRobotter', 'hm-robot'], ['5', 'login', 'navLogin', 'laas'], ['6', 'land', 'navLand', 'hm-klode'], ['7', 'besoegende', 'navBesoeg', 'hm-besoeg']];
  const skift = Kit.visningsskift($('skift'), SKIFT.map(([tast, id, t, ikon]) => ({ id, t, ikon, genvej: (mac ? '⌘' : 'Ctrl+') + tast, genvejAria: (mac ? 'Meta+' : 'Control+') + tast })), { ariaT: 'navH' });
  for (const a of skift.links) {
    const navn = a.querySelector('span'), k = a.querySelector('kbd'), v = a.dataset.visning;
    if (k) k.dataset.genvej = SKIFT.find(x => x[1] === v)[0];
    if (v === 'land') navn.id = 'nav-land';
    if (v === 'siden-sidst') a.insertBefore(el('span', { className: 'tal', id: 'siden-tal', hidden: true }), k);
  }
  /* Landevælgeren står i båndet og vises kun på visninger med land: true; visningen fylder den og lytter på 'landValg'. */
  const landVaelger = Kit.vaelger({ etiket: { t: 'landVaelg' }, soeg: { ph: 'landSoegPh', ariaT: 'landSoeg' }, ingen: { t: 'landIngen' }, punkter: [], vedValg: id => udsend('landValg', id) });
  const baand = Kit.commandBar($('baand'), { ariaT: 'baandAria', udenFaner: true, faner: [{ id: 'start', t: 'baandAria', grupper: [
    { t: 'gData', knapper: [{ id: 'hent', t: 'hentNu', ikon: 'hm-hent', pri: true }] },
    { t: 'gLand', knapper: [{ id: 'land', element: landVaelger.rod }] },
    { t: 'gHjaelp', knapper: [{ id: 'vejledning', t: 'vejledningAabn', ikon: 'bog', ved: () => aabnVejledning() }] },
  ] }] });
  baand.knap('hent').id = 'hent-nu';
  baand.knap('vejledning').id = 'baand-vejledning';
  const landGruppe = landVaelger.rod.closest('.grp');
  landGruppe.id = 'band-land';
  landGruppe.hidden = true;
  /* Hjælpen er kittets sidepanel og ligger over arbejdsfladen, som skuffen gjorde. */
  const hjPanel = Kit.sidepanel($('hjaelp'), { t: 'hjaelpH', lukAriaT: 'hjaelpLuk', indhold: $('hjaelp-indhold'), overlaeg: true });
  hjPanel.rod.querySelector('.sp-luk').id = 'hjaelp-luk';
  $('fod-lokalt').prepend(Kit.ikon('laas'));
  S.anvend();

  /* ── visninger og router ── */
  const VISNINGER = ['overblik', 'siden-sidst', 'vaernet', 'robotter', 'login', 'land', 'besoegende', 'teknologi', 'indstillinger'];
  const visninger = {};
  const registrer = (navn, v) => { visninger[navn] = v; };
  const sektion = navn => document.querySelector(`section[data-visning="${navn}"]`);
  /* Ingen decodeURIComponent her: URLSearchParams afkoder selv. To gange ødelægger rigtige værdier (/wp-config%2ephp, AT&T). */
  function laesHash() {
    const h = location.hash.replace(/^#/, ''), i = h.indexOf('?');
    return { navn: i < 0 ? h : h.slice(0, i), params: new URLSearchParams(i < 0 ? '' : h.slice(i + 1)) };
  }
  /* #dyk?… er en gammel adresse, og data-dyk er stadig dens forespørgsel: den lander på Angreb med samme parametre. Dyk uden periode var 30 hele dage, Angreb uden periode er 7, så tallet, der blev klikket på, skal have sin periode med. */
  const PERIODE = ['periode', 'dag', 'fra', 'til'];
  function fraDyk(q) {
    const p = new URLSearchParams(q);
    if (!PERIODE.some(k => p.has(k))) p.set('periode', '30');
    return p.toString();
  }
  /* vis=angreb-i-gaar er en gammel adresse: nu er det statusdagen med "Kun angreb". Kendes rækken ikke endnu, er det i går. */
  function fraIGaar(params) {
    const p = new URLSearchParams(params), r = tilstand.raekker.find(x => x.id === p.get('zone'));
    p.set('vis', 'seneste');
    p.set('dag', (r && r.status_dag) || new Date(Date.now() - 864e5).toISOString().slice(0, 10));
    p.set('slags', 'angreb');
    return p.toString();
  }
  /* Skift visning. Er adressen allerede den samme, tegnes den igen (hashchange kommer ikke). */
  function gaaTil(navn, query) {
    const q = query ? String(query) : '';
    const ny = '#' + navn + (q ? '?' + q : '');
    hist.genbesoeg = false;
    if (location.hash === ny) route(); else location.hash = ny;
  }
  /* Adressen for det, der står, uden at tegne noget og uden et nyt trin i tilbage/frem (fx sideskift i "De seneste", V-BESOEG.2 FIX 1). Trinets nummer bevares. */
  function erstatAdresse(navn, query) {
    const q = query ? String(query) : '';
    history.replaceState(history.state, '', '#' + navn + (q ? '?' + q : ''));
    tilstand.params = new URLSearchParams(q);
  }
  function route() {
    let { navn, params } = laesHash();
    /* Adressen erstattes: det er stadig ét trin i historikken. */
    if (navn === 'dyk' || (navn === 'vaernet' && params.get('vis') === 'angreb-i-gaar')) {
      history.replaceState(history.state, '', '#vaernet?' + (navn === 'dyk' ? fraDyk(params) : fraIGaar(params)));
      return route();
    }
    if (!VISNINGER.includes(navn)) {
      if (tilstand.visning && navn) return; // fx #indhold (spring til indhold): bliv
      navn = 'overblik';
    }
    if (!tilstand.sat && navn !== 'indstillinger') navn = 'opsaetning';
    /* Et domæne, der ikke er valgt, er væk overalt, også fra en gammel adresse. Adressen erstattes, så tilbage-knappen ikke sidder fast. */
    if (tilstand.uvalgt.has(params.get('zone'))) {
      const ny = { vaernet: '#vaernet?samlet=1', robotter: '#robotter?samlet=1', login: '#login?samlet=1', besoegende: '#besoegende?samlet=1', teknologi: '#teknologi', indstillinger: '#indstillinger' }[navn] || '#overblik';
      history.replaceState(history.state, '', ny);
      return route();
    }
    if (visninger[navn] && visninger[navn].domaener) huskDomaene(params);
    const foer = tilstand.visning, foerParams = tilstand.params.toString();
    if (foer && foer !== navn && visninger[foer] && visninger[foer].skjul) visninger[foer].skjul();
    tilstand.visning = navn;
    tilstand.params = params;
    document.querySelectorAll('section[data-visning]').forEach(s => { s.hidden = s.getAttribute('data-visning') !== navn; });
    const punkt = navn === 'teknologi' ? 'indstillinger' : navn === 'vaernet' && (params.has('robot') || params.has('robotgruppe')) ? 'robotter' : navn;
    skift.markér(punkt);
    if (punkt === 'indstillinger') $('indst-knap').setAttribute('aria-current', 'page'); else $('indst-knap').removeAttribute('aria-current');
    document.querySelectorAll('#hjaelp .hjaelp-del').forEach(d => { d.hidden = d.getAttribute('data-hjaelp') !== navn; });
    $('band-land').hidden = !(visninger[navn] && visninger[navn].land);
    opdaterTitel();
    if (visninger[navn] && visninger[navn].vis) visninger[navn].vis(params, { samme: foer === navn && foerParams === params.toString() });
    visVaelger();
  }
  /* En visning kan låne en anden visnings titel (titel()): Angreb med robotfilter hedder Robotter. */
  function opdaterTitel() { const v = visninger[tilstand.visning]; document.title = S.titel((v && v.titel && v.titel()) || tilstand.visning); }
  function navTilstand() {
    document.querySelectorAll('a[data-visning]').forEach(a => {
      const fri = tilstand.sat || a.getAttribute('data-visning') === 'indstillinger';
      if (fri) a.removeAttribute('aria-disabled'); else a.setAttribute('aria-disabled', 'true');
    });
  }
  // Altid gennem gaaTil: et klik på den adresse, der allerede står, ville ellers rydde history.state (trinets nummer).
  document.querySelectorAll('a[data-visning]').forEach(a => a.addEventListener('click', ev => {
    ev.preventDefault();
    if (a.getAttribute('aria-disabled') !== 'true') aabnGenvej(a.getAttribute('data-visning'));
  }));

  /* ── den lokale tid til Rust: uden en ny afhængighed kan Rust ikke læse tidszonen, så skiftene for de næste 13 måneder sendes herfra. De gælder også, når vinduet er lukket. ── */
  function tidsskift(nu = Date.now()) {
    const dag = 864e5, forskel = ms => -new Date(ms).getTimezoneOffset(), start = nu - 40 * dag, ud = [[start, forskel(start)]];
    for (let t = start; t < nu + 400 * dag; t += dag) {
      if (forskel(t + dag) === forskel(t)) continue;
      let a = t, b = t + dag;
      while (b - a > 1000) { const m = Math.floor((a + b) / 2); if (forskel(m) === forskel(a)) a = m; else b = m; }
      const skift = Math.round(b / 6e4) * 6e4;
      ud.push([skift, forskel(skift)]);
    }
    return ud;
  }
  /* Spørger i dialogen og svarer true eller false. */
  function bekraeft(tekst) {
    const d = $('bekraeft-dialog');
    $('bekraeft-p').textContent = tekst;
    d.showModal();
    return new Promise(svar => {
      const luk = v => { $('bekraeft-ja').removeEventListener('click', ja); $('bekraeft-nej').removeEventListener('click', nej); d.close(); svar(v); };
      const ja = () => luk(true), nej = () => luk(false);
      $('bekraeft-ja').addEventListener('click', ja);
      $('bekraeft-nej').addEventListener('click', nej);
    });
  }

  /* ── det fælles domænevalg ── */
  function huskDomaene(params) {
    const valg = params.get('samlet') === '1' ? 'samlet' : params.get('zone');
    if (!valg || valg === tilstand.domaeneValg) return;
    tilstand.domaeneValg = valg;
    if (invoke) invoke('domaene_valg_saet', { valg }).catch(() => { });
  }
  /* Adressen for en visning med det fælles valg: visningens egen periode, det fælles domæne. Fravalgt eller væk er Samlet. */
  function medDomaene(q) {
    const p = new URLSearchParams(q), r = tilstand.raekker.find(x => x.id === tilstand.domaeneValg), hvorfra = p.get('zone') || 'samlet';
    ['zone', 'navn', 'samlet'].forEach(k => p.delete(k));
    if (hvorfra !== (r ? r.id : 'samlet')) p.delete('vis');
    if (r) { p.set('zone', r.id); p.set('navn', r.navn); } else p.set('samlet', '1');
    return p.toString();
  }

  /* ── domænetræet i navigationsruden (Kit.navTrae): "Alle domæner" med hvert valgt domæne under. Ét valg. En visning med
     domaener: true siger, hvad den viser (hvad), og hvad et skift betyder (skift); på de andre visninger åbner et valg Angreb
     for domænet. Træet bygges i et nyt <ul> hver gang, så kittets lyttere ikke lægger sig oven på hinanden; kittet melder
     også det valg, det selv sætter, så stille er sat imens. ── */
  let traeStille = false;
  function traeValg(id, navn) {
    const v = visninger[tilstand.visning], q = id === 'samlet' ? 'samlet=1' : new URLSearchParams({ zone: id, navn });
    if (v && v.domaener) { v.skift(q); return; }
    tilstand.domaeneValg = id;
    if (invoke) invoke('domaene_valg_saet', { valg: id }).catch(() => { });
    aabnGenvej('vaernet');
  }
  function visVaelger() {
    const v = visninger[tilstand.visning], zoner = [...tilstand.raekker].sort((a, b) => a.navn.localeCompare(b.navn));
    const ny = el('ul', { id: 'trae' }), hadeFokus = $('trae').contains(document.activeElement);
    $('trae').replaceWith(ny);
    /* Er intet domæne valgt, er der intet at vælge imellem: ingen "Alle domæner (0)". */
    $('trae-tom').hidden = zoner.length > 0;
    if (!zoner.length) return;
    const boern = zoner.map(r => ({ id: r.id, tekst: r.navn }));
    let valgt = zoner.some(r => r.id === tilstand.domaeneValg) ? tilstand.domaeneValg : 'samlet';
    if (v && v.domaener) {
      const { samlet, zone, navn } = v.hvad();
      if (!samlet && zone && !zoner.some(r => r.id === zone)) boern.push({ id: zone, tekst: navn });
      valgt = samlet ? 'samlet' : zone;
    }
    const navne = Object.fromEntries(boern.map(b => [b.id, b.tekst]));
    traeStille = true;
    Kit.navTrae(ny, [{ id: 'samlet', tekst: T.traeAlle, antal: zoner.length, boern }], { ariaT: 'traeAria', valgt, vedValg: id => { if (!traeStille) traeValg(id, navne[id]); } });
    traeStille = false;
    S.anvend(ny);
    if (hadeFokus) { const b = ny.querySelector('.tr.on'); if (b) b.focus(); }
  }
  paa('raekker', visVaelger);

  /* ── klikbart med data bag: data-dyk = en forespørgsel til Angreb (uden periode: 30 dage, se fraDyk), data-gaa = en anden visning ("vaernet?zone=…"), data-cve = cve.org,
     data-radar = Radars forside (kildelinjen), data-rul = et afsnit i samme visning ── */
  const maal = t => (t && t.closest ? t.closest('[data-dyk], [data-gaa], [data-cve], [data-radar], [data-rul]') : null);
  function rulTil(id) {
    const afsnit = $(id), h = afsnit && afsnit.querySelector('h2');
    if (!afsnit) return;
    if (afsnit.scrollIntoView) afsnit.scrollIntoView({ block: 'start' });
    if (h) { h.tabIndex = -1; h.focus({ preventScroll: true }); }
  }
  function aabnMaal(e) {
    const gaa = e.getAttribute('data-gaa'), cve = e.getAttribute('data-cve'), rul = e.getAttribute('data-rul');
    if (cve) return aabnCve(cve);
    if (e.hasAttribute('data-radar')) return invoke ? invoke('aabn_radar').catch(() => { }) : null;
    if (rul) return rulTil(rul);
    if (e.closest('#hjaelp')) hjaelp(false);
    if (!gaa) return gaaTil('vaernet', fraDyk(e.getAttribute('data-dyk')));
    const i = gaa.indexOf('?');
    return i < 0 ? gaaTil(gaa) : gaaTil(gaa.slice(0, i), gaa.slice(i + 1));
  }
  document.addEventListener('click', ev => { const e = maal(ev.target); if (e) { ev.preventDefault(); aabnMaal(e); } });
  document.addEventListener('keydown', ev => {
    const e = maal(ev.target);
    if (!e || ev.metaKey || ev.ctrlKey || ev.altKey) return;
    if (ev.key === 'Enter' || ev.key === ' ') {
      if (e.tagName !== 'BUTTON') { ev.preventDefault(); aabnMaal(e); } // en knap klikker selv
      return;
    }
    const skridt = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[ev.key], tegning = e.closest('svg');
    if (!skridt || !tegning) return;
    ev.preventDefault();
    const alle = [...tegning.querySelectorAll('[data-dyk]:not(.dyk-hit), [data-gaa]:not(.dyk-hit)')], ny = alle[(alle.indexOf(e) + skridt + alle.length) % alle.length];
    alle.forEach(x => x.setAttribute('tabindex', x === ny ? '0' : '-1'));
    ny.focus();
  });

  /* ── tilbage og frem: hvert hash-skift er ét trin i webvisningens egen historik ──
     history.state har trinets nummer; et hashchange uden nummer er et nyt trin, med nummer er det tilbage/frem. Rullepositionen
     læses pr. trin i selve hashchange (dér står den gamle visning stadig; en scroll-hændelse ville komme for sent). maks (det sidste trin, frem kan nå) og rullepositionerne overlever sprogskiftets genindlæsning i
     sessionStorage. Køens opdateringer rører ikke adressen og laver derfor aldrig et trin. */
  const hist = { trin: 0, maks: 0, rul: {}, genbesoeg: false, ur: null };
  const HIST = 'hm-historik', hoved = $('indhold');
  function gemHist() { try { sessionStorage.setItem(HIST, JSON.stringify({ maks: hist.maks, rul: hist.rul })); } catch (e) { } }
  function histKnapper() {
    $('hist-tilbage').disabled = hist.trin <= 0;
    $('hist-frem').disabled = hist.trin >= hist.maks;
  }
  const stopRul = () => { clearTimeout(hist.ur); hist.ur = null; };
  /* Visningen kan tegne sit indhold lidt efter (Angreb henter fra cachen): prøv igen, til det er der. Brugeren vinder. */
  function gendanRul(top) {
    stopRul();
    let n = 0;
    const saet = () => { hoved.scrollTop = top; hist.ur = ++n < 15 ? setTimeout(saet, 100) : null; };
    saet();
  }
  const huskRul = () => { if (hist.ur === null) hist.rul[hist.trin] = hoved.scrollTop; };
  for (const t of ['wheel', 'touchstart', 'pointerdown', 'keydown']) hoved.addEventListener(t, stopRul, { passive: true });
  function nytHash() {
    const s = history.state, n = s && Number.isInteger(s.hmTrin) ? s.hmTrin : null;
    huskRul();
    stopRul();
    hist.genbesoeg = n !== null;
    if (n === null) {
      hist.trin++;
      hist.maks = hist.trin;
      for (const k of Object.keys(hist.rul)) if (Number(k) >= hist.trin) delete hist.rul[k];
      history.replaceState({ hmTrin: hist.trin }, '');
      hoved.scrollTop = 0;
    } else hist.trin = n;
    gemHist();
    histKnapper();
    route();
    if (n !== null) gendanRul(hist.rul[n] || 0);
  }
  window.addEventListener('hashchange', nytHash);
  window.addEventListener('pagehide', () => { huskRul(); gemHist(); });
  const iTekstfelt = () => {
    const a = document.activeElement;
    return Boolean(a && (a.isContentEditable || a.tagName === 'TEXTAREA' || (a.tagName === 'INPUT' && !/^(checkbox|radio|button|submit|reset|range|color|file)$/.test(a.type))));
  };
  function gaaHist(skridt) {
    const maal = hist.trin + skridt;
    if (maal < 0 || maal > hist.maks || document.querySelector('dialog[open]')) return;
    history.go(skridt);
  }
  $('hist-tilbage').addEventListener('click', () => gaaHist(-1));
  $('hist-frem').addEventListener('click', () => gaaHist(1));
  /* Trinets nummer ved start. Efter en genindlæsning (sprogskift) står nummeret stadig i history.state. Adressen får altid en
     visning, så et klik på Overblik ved start ikke laver et trin, der ser ud som det samme. */
  function klarHist() {
    if (!location.hash) history.replaceState(history.state, '', '#overblik');
    const s = history.state;
    if (s && Number.isInteger(s.hmTrin)) {
      let gemt = null;
      try { gemt = JSON.parse(sessionStorage.getItem(HIST)); } catch (e) { }
      hist.trin = s.hmTrin;
      hist.maks = Math.max(hist.trin, (gemt && gemt.maks) || 0);
      hist.rul = (gemt && gemt.rul) || {};
      hist.genbesoeg = true;
    } else history.replaceState({ hmTrin: 0 }, '');
    histKnapper();
  }

  /* ── genveje ── */
  const GENVEJE = [['1', 'overblik', 'navOverblik'], ['2', 'siden-sidst', 'navSiden'], ['3', 'vaernet', 'navVaernet'], ['4', 'robotter', 'navRobotter'], ['5', 'login', 'navLogin'], ['6', 'land', 'navLand'], ['7', 'besoegende', 'navBesoeg'], [',', 'indstillinger', 'navIndst']];
  const genvejTekst = tast => (mac ? '⌘' : 'Ctrl+') + (tast === 'r' ? 'R' : tast);
  const histGenvej = [mac ? '⌘[' : 'Alt+←', mac ? '⌘]' : 'Alt+→'];
  for (const [id, tekst, genvej] of [['hist-tilbage', T.tilbage, histGenvej[0]], ['hist-frem', T.frem, histGenvej[1]]]) {
    $(id).title = `${tekst} (${genvej})`;
    $(id).setAttribute('aria-label', tekst);
  }
  {
    const dl = $('genveje');
    for (const [tast, , noegle] of GENVEJE) dl.append(el('dt', {}, el('kbd', { textContent: genvejTekst(tast) })), el('dd', { textContent: T[noegle] }));
    dl.append(el('dt', {}, el('kbd', { textContent: genvejTekst('r') })), el('dd', { textContent: T.hentNu }));
    dl.append(el('dt', {}, el('kbd', { textContent: histGenvej[0] })), el('dd', { textContent: T.tilbage }));
    dl.append(el('dt', {}, el('kbd', { textContent: histGenvej[1] })), el('dd', { textContent: T.frem }));
    for (const [tast, tekst] of [['+', T.tekstStoerre], ['−', T.tekstMindre], ['0', T.tekstNormal]]) dl.append(el('dt', {}, el('kbd', { textContent: genvejTekst(tast) })), el('dd', { textContent: tekst }));
    dl.append(el('dt', {}, el('kbd', { textContent: 'Esc' })), el('dd', { textContent: T.hjaelpLuk }));
  }
  function aabnGenvej(maal) {
    if (visninger[maal] && visninger[maal].sidste) gaaTil(maal, medDomaene(visninger[maal].sidste()));
    else if (maal === 'om') gaaTil('indstillinger', 'afsnit=om');
    else gaaTil(maal);
  }
  document.addEventListener('keydown', ev => {
    if (ev.key === 'Escape' && !$('hjaelp').hidden) { hjaelp(false); return; }
    // Tilbage/frem. ⌘← ⌘→ er ikke i programmenuen: et menupunkt har kun én genvej, og i et tekstfelt flytter de markøren.
    const pil = ev.key === 'ArrowLeft' ? -1 : ev.key === 'ArrowRight' ? 1 : 0;
    if (pil && !ev.shiftKey && !ev.ctrlKey && (mac ? ev.metaKey && !ev.altKey : ev.altKey && !ev.metaKey)) {
      if (!iTekstfelt()) { ev.preventDefault(); gaaHist(pil); }
      return;
    }
    // Tekststørrelse (kittets Tekst: større, mindre, nulstil). '=' er + uden Skift på et amerikansk tastatur, og programmenuens ⌘+ fanger kun tegnet +.
    const tekstTrin = { '+': 'større', '=': 'større', '-': 'mindre', 0: 'nulstil' }[ev.key];
    if (tekstTrin !== undefined && !ev.altKey && (mac ? ev.metaKey && !ev.ctrlKey : ev.ctrlKey && !ev.metaKey)) { ev.preventDefault(); window.Tekst[tekstTrin](); return; }
    if (mac) return; // macOS: programmenuen (Rust) har genvejene
    if (ev.key === 'F5' || (ev.ctrlKey && !ev.altKey && (ev.key === 'r' || ev.key === 'R'))) { ev.preventDefault(); hentNu(); return; }
    if (!ev.ctrlKey || ev.altKey || ev.metaKey) return;
    const g = GENVEJE.find(([tast]) => tast === ev.key);
    if (!g) return;
    ev.preventDefault();
    if (g[1] === 'indstillinger' || tilstand.sat) aabnGenvej(g[1]);
  });

  /* ── hjælpen (kittets sidepanel, se øverst): den generelle forklaring ("?") eller en guide (Overblikkets "Hvad gør jeg?": { titel: nøglen i T, indhold }) ── */
  function hjaelp(aaben, guide) {
    const g = $('hjaelp-guide');
    if (aaben) {
      g.replaceChildren(...(guide ? [guide.indhold] : []));
      g.hidden = !guide;
      $('hjaelp-generelt').hidden = Boolean(guide);
      $('hjaelp-h').dataset.t = guide ? guide.titel : 'hjaelpH'; // data-t, så sprog.js' anvend() ikke skriver en anden tekst tilbage
      $('hjaelp-h').textContent = T[$('hjaelp-h').dataset.t];
    }
    if (aaben) { hjPanel.aabn(); $('hjaelp-luk').focus(); } else { hjPanel.luk(); $('hjaelp-knap').focus(); }
  }
  // "?" åbner (eller skifter fra en guide til) den generelle forklaring; står den allerede, lukker "?".
  $('hjaelp-knap').addEventListener('click', () => hjaelp($('hjaelp').hidden || !$('hjaelp-guide').hidden));
  $('hjaelp-luk').addEventListener('click', () => hjaelp(false));
  $('skip').addEventListener('click', ev => { ev.preventDefault(); $('indhold').focus(); });

  /* ── køen: pillen i båndet og en linje i indholdet, når noget kræver opmærksomhed ── */
  function visKoe(k) {
    tilstand.koe = k;
    hentKoe(k);
    const pille = $('koe'), kort = $('koe-kort'), tekst = $('koe-tekst'), bar = $('koe-fremskridt'), besked = $('koe-besked');
    if (!k || !k.zoner_i_alt) { pille.hidden = true; status(besked, ''); udsend('koe', k); return; }
    pille.hidden = false;
    bar.max = Math.max(1, k.doegn_i_alt);
    bar.value = Math.min(k.doegn_faerdige, bar.max);
    bar.setAttribute('aria-label', T.koeFremskridt);
    let t, klasse = 'henter', k1;
    if (k.tilstand === 'venter_kvote') { t = fmt(T.koeKvote, { kl: S.tid(k.fortsaetter_kl) }); klasse = 'gul'; k1 = T.pilleVenter; }
    else if (k.tilstand === 'pause') { t = fmt(k.fejl && k.fejl.kode === 'forbindelse' ? T.koePauseNet : T.koePause, { kl: S.tid(k.fortsaetter_kl) }); klasse = 'gul'; k1 = fmt(T.pillePause, { kl: S.tid(k.fortsaetter_kl) }); }
    else if (k.tilstand === 'fejl') { t = fmt(T.koeFejl, { f: S.fejl(k.fejl) }); klasse = 'roed'; k1 = T.pilleFejl; }
    else if (k.tilstand === 'faerdig') {
      const sidst = tilstand.raekker.reduce((a, r) => (r.sidst_hentet && r.sidst_hentet > a ? r.sidst_hentet : a), '');
      /* Uden forbindelse (fx lige efter dvale) prøver køen selv igen om lidt: det siges i klart sprog, med domænet, når det er ét. */
      const net = k.zoner_fejl && S.erNet(k.fejl);
      t = net ? fmt(T.koeNet, { d: k.zoner_fejl === 1 && k.fejl_navn ? k.fejl_navn : T.domaener(k.zoner_fejl) })
        : k.zoner_fejl ? fmt(T.koeFaerdigFejl, { t: S.kl(sidst), n: T.domaener(k.zoner_fejl), f: S.fejl(k.fejl) }) : fmt(T.koeFaerdig, { t: S.kl(sidst) });
      klasse = k.zoner_fejl ? 'gul' : 'ok';
      k1 = net ? T.pilleNet : fmt(k.zoner_fejl ? T.pilleOpdateretFejl : T.pilleOpdateret, { t: S.tid(sidst || new Date().toISOString()), n: k.zoner_fejl });
      bar.value = bar.max;
    } else {
      t = fmt(T.koeHenter, { z: k.zoner_faerdige, Z: k.zoner_i_alt, d: S.tal(k.doegn_faerdige), D: S.tal(k.doegn_i_alt) });
      if (k.navn) t += ' · ' + fmt(T.koeNu, { navn: k.navn });
      k1 = fmt(T.pilleHenter, { z: k.zoner_faerdige, Z: k.zoner_i_alt });
    }
    pille.className = 'pille ' + klasse;
    pille.title = t;
    pille.setAttribute('aria-label', fmt(T.pilleAabn, { t }));
    kort.textContent = k1;
    tekst.textContent = t;
    status(besked, klasse === 'gul' || klasse === 'roed' ? t : '', klasse);
    udsend('koe', k);
  }
  /* "Hent nu": knappen og teksten ved siden af siger straks "Henter …". Rust svarer, hvad der skete (startet, kører allerede, intet at hente), så her gættes ikke: en startet runde er færdig, når køen melder det.
     Samme tilstand i båndet og under Indstillinger → Data, og ⌘R (hændelsen 'hent_nu') og Ctrl+R/F5 går samme vej. Svaret står i 4 sekunder. */
  const HENT_KNAPPER = ['hent-nu', 'data-hent'], HENT_TEKSTER = ['hent-status', 'data-hent-status'], HENT_VIST_MS = 4000, HENT_VAGT_MS = 5000;
  const hent = { aktiv: false, set: false, ur: null, vagt: null };
  function hentTekst(tekst, klasse) {
    for (const id of HENT_TEKSTER) { const e = $(id); e.textContent = tekst; e.title = tekst; e.className = 'hent-status' + (klasse ? ' ' + klasse : ''); }
  }
  function hentKnapper(aktiv) {
    for (const id of HENT_KNAPPER) {
      const b = $(id), t = b.querySelector('[data-t]');
      b.disabled = aktiv || !tilstand.sat;
      b.classList.toggle('henter', aktiv);
      if (aktiv) b.setAttribute('aria-busy', 'true'); else b.removeAttribute('aria-busy');
      t.dataset.t = aktiv ? 'hentHenter' : b.dataset.hvile; // data-t, så sprog.js' anvend() ikke skriver en anden tekst tilbage
      t.textContent = T[t.dataset.t];
    }
  }
  for (const id of HENT_KNAPPER) $(id).dataset.hvile = $(id).querySelector('[data-t]').dataset.t;
  function hentSlut(tekst, klasse) {
    hent.aktiv = false;
    clearTimeout(hent.ur);
    clearTimeout(hent.vagt);
    hentKnapper(false);
    hentTekst(tekst, klasse);
    if (klasse !== 'roed') hent.ur = setTimeout(() => hentTekst(''), HENT_VIST_MS); // en fejl står, til der trykkes igen
  }
  const sidstHentet = () => tilstand.raekker.reduce((a, r) => (r.sidst_hentet && r.sidst_hentet > a ? r.sidst_hentet : a), '');
  const hentIntet = () => { const t = sidstHentet(); return t ? fmt(T.hentAlt, { t: S.tid(t) }) : T.hentAltUden; };
  /* Køen melder: en startet runde er slut, når den har været i gang og står færdig. */
  function hentKoe(k) {
    if (!hent.aktiv || !k) return;
    if (k.tilstand === 'henter') hent.set = true;
    if (k.tilstand === 'fejl') hentSlut(fmt(T.hentFejl, { f: S.fejl(k.fejl) }), 'roed');
    else if (k.tilstand === 'faerdig' && hent.set) hentSlut(k.zoner_opdateret ? fmt(T.hentFaerdig, { t: S.tid(new Date().toISOString()), n: T.domaener(k.zoner_opdateret) }) : hentIntet(), 'ok');
  }
  async function hentNu() {
    if (!invoke || hent.aktiv) return;
    hent.aktiv = true;
    hent.set = false;
    clearTimeout(hent.ur);
    hentKnapper(true);
    hentTekst(T.hentHenter, 'sr-only'); // knappen siger det; her er det til skærmlæsere
    try {
      const u = await invoke('hent_nu');
      if (u.udfald === 'intet_at_hente') hentSlut(hentIntet(), 'ok');
      else if (u.udfald === 'koerer_allerede') hentSlut(fmt(T.hentAllerede, { a: S.tal(u.faerdige), b: S.tal(u.i_alt) }));
      /* En runde kan nå at slutte, før svaret er her. Så kommer der ingen melding mere. */
      else hent.vagt = setTimeout(async () => { let k = null; try { k = await invoke('koe_status'); } catch (e) { /* køen melder selv */ } if (hent.aktiv && !hent.set && k && k.tilstand === 'faerdig') { hent.set = true; hentKoe(k); } }, HENT_VAGT_MS);
    } catch (e) { hentSlut(fmt(T.hentFejl, { f: S.fejl(e) }), 'roed'); }
  }
  $('hent-nu').addEventListener('click', hentNu);

  /* ── overblikkets rækker og domænerne (delt af Overblik, Siden sidst, Angreb og Indstillinger) ── */
  /* Hele zonelisten med kontakten pr. domæne. Overblikkets rækker har kun dem, der er slået til. */
  async function hentDomaener() {
    try { tilstand.domaener = await invoke('domaener_hent'); } catch (e) { tilstand.domaener = []; }
    tilstand.uvalgt = new Set(tilstand.domaener.filter(d => !d.til).map(d => d.id));
    udsend('domaener');
    if (tilstand.uvalgt.has(tilstand.params.get('zone'))) route();
  }
  /* Gemmer de valgte domæner og tegner alt om, der viser domæner. */
  async function gemValgte(ids) {
    await invoke('domaener_saet', { valgte: [...ids] });
    await hentDomaener();
    await hentOverblik();
  }
  /* "Ikke nu" til nye domæner. */
  async function afvisNye(ids) {
    await invoke('domaene_afvis', { zoner: [...ids] });
    await hentDomaener();
  }
  /* Fodlinjen: antal valgte domæner til venstre (køens status står ved siden af), versionen til højre. */
  function fodAntal() { $('fod-antal').textContent = tilstand.raekker.length ? T.domaener(tilstand.raekker.length) : ''; }
  paa('raekker', fodAntal);
  if (window.__TAURI__ && window.__TAURI__.app) window.__TAURI__.app.getVersion().then(v => { $('fod-version').textContent = fmt(T.omVersion, { v }); }).catch(() => { });
  function sidenTal() {
    const n = tilstand.raekker.filter(r => r.status === 'roed' && !r.gammel).length, b = $('siden-tal');
    b.hidden = !n;
    b.textContent = n ? String(n) : '';
    b.title = n ? T.sidenTal(n) : '';
  }
  async function hentOverblik() {
    try { tilstand.raekker = await invoke('overblik'); } catch (e) { tilstand.raekker = []; status($('zone-status'), S.fejl(e), 'fejl'); }
    try { tilstand.huller = await invoke('huller'); } catch (e) { tilstand.huller = null; }
    udsend('huller');
    sidenTal();
    udsend('raekker', tilstand.raekker);
  }
  /* Højst hvert 2. sekund, mens køen henter; straks når en runde er færdig. */
  let sidstTegnet = 0, ventende = null;
  function snartOverblik(straks) {
    if (ventende) { if (!straks) return; clearTimeout(ventende); }
    const vent = straks ? 0 : Math.max(0, sidstTegnet + 2000 - Date.now());
    ventende = setTimeout(() => { ventende = null; sidstTegnet = Date.now(); hentOverblik(); }, vent);
  }

  /* Ét forsøg på at hente friske zoner: 'ok', 'net' (prøv igen) eller 'stop'. */
  async function hentZoner() {
    const st = $('zone-status');
    status(st, T.henterZoner, 'venter');
    try {
      const z = await invoke('zones');
      tilstand.gemte = { zoner: z, hentet: new Date().toISOString() };
      await hentDomaener();
      await hentOverblik();
      status(st, z.length ? '' : T.ingenZoner, z.length ? '' : 'fejl');
      return 'ok';
    } catch (e) {
      const net = S.erNet(e);
      if (tilstand.gemte && tilstand.gemte.zoner.length) {
        const t = S.kl(tilstand.gemte.hentet);
        status(st, net ? fmt(T.gemteZonerNet, { t }) : fmt(T.gemteZoner, { f: S.fejl(e), t }), 'advarsel');
      } else status(st, S.fejl(e), 'fejl');
      return net ? 'net' : 'stop';
    }
  }
  const genforsoeg = invoke ? window.hmGenforsoeg($('genforsoeg'), hentZoner) : null;

  async function zoner() {
    try { tilstand.gemte = await invoke('zones_cache'); } catch (e) { tilstand.gemte = null; }
    await hentDomaener();
    await hentOverblik();
    try { tilstand.domaeneValg = (await invoke('domaene_valg_hent')) || 'samlet'; } catch (e) { /* Samlet står */ }
    if (visninger[tilstand.visning] && visninger[tilstand.visning].domaener) huskDomaene(tilstand.params);
    try { visKoe(await invoke('koe_status')); } catch (e) { visKoe(null); }
    await genforsoeg.start();
  }

  /* "Siden sidst": Rust afgør, om visningen skal åbne af sig selv (vis: menuen, en notifikation eller forudindstillingen);
     tving = hændelsen fra menulinjen/notifikationen. Visningen tegner det hentede og markerer det som set. */
  async function sidenSidst(tving) {
    let d;
    try { d = await invoke('siden_sidst'); } catch (e) { return; }
    if (!d) return;
    tilstand.siden = d;
    udsend('siden', d);
    if (d.vis || tving) gaaTil('siden-sidst');
  }

  /* Tokenet afgør, om rammen viser opsætningen eller visningerne. Kaldes ved start, efter "Gem og forbind" og "Glem". */
  async function vis() {
    let sat = false;
    try { sat = (await invoke('token_status')).sat; } catch (e) { status($('token-status'), S.fejl(e), 'fejl'); }
    tilstand.sat = sat;
    $('hent-nu').hidden = !sat;
    $('hent-nu').closest('.grp').hidden = !sat; // uden API Token er der intet at hente: hele gruppen væk, ikke kun knappen
    navTilstand();
    udsend('token', sat);
    if (sat) { route(); await zoner(); await sidenSidst(false); }
    else { genforsoeg.afbryd(); tilstand.raekker = []; tilstand.domaener = []; tilstand.uvalgt = new Set(); tilstand.domaeneValg = 'samlet'; udsend('domaener'); sidenTal(); udsend('raekker', []); visKoe(null); route(); $('token').focus({ preventScroll: true }); } // velkomsten over feltet skal ses først
  }

  /* Fælles for opsætningen og Indstillinger → Skift nøgle. true, når nøglen er gemt. */
  async function gemNoegle(felt, knap, st) {
    status(st, T.gemmer, 'venter'); knap.disabled = true;
    try {
      await invoke('token_set', { token: felt.value });
      felt.value = '';
      status(st, T.gemt, 'ok');
      await vis();
      return true;
    } catch (e) {
      status(st, S.fejl(e), 'fejl');
      /* En manglende rettighed: fejlen siger hvilken, og trinene står lige ved siden af. */
      if (e && /^mangler_/.test(e.kode)) { const trin = felt.closest('section').querySelector('.token-trin'); if (trin) st.append(' ', visTrin(trin)); }
      return false;
    } finally { knap.disabled = false; }
  }
  function visTrin(trin) {
    const b = el('button', { type: 'button', className: 'link', textContent: T.tokenSeTrin });
    b.addEventListener('click', () => { trin.open = true; trin.querySelector('summary').focus(); });
    return b;
  }
  /* "Opret API Token hos Cloudflare" og trinene: i opsætningen og under Indstillinger → Forbindelse. Rust åbner én fast adresse. */
  document.querySelectorAll('[data-token-hjaelp]').forEach(plads => {
    const aabn = el('button', { type: 'button', className: 'btn', textContent: T.tokenAabn });
    aabn.addEventListener('click', () => { if (invoke) invoke('aabn_api_token').catch(() => { }); });
    const trin = el('details', { className: 'hm-hjaelp token-trin' }, el('summary', { textContent: T.hjaelpTokenH }), el('ol', {}, ...T.tokenTrin.map(t => el('li', { textContent: t }))), el('p', { className: 'note', textContent: T.tokenTrinNote }));
    plads.append(aabn, el('p', { className: 'note', textContent: T.tokenAabnNote }), trin);
  });
  $('token-form').addEventListener('submit', ev => { ev.preventDefault(); gemNoegle($('token'), $('token-knap'), $('token-status')); });

  /* "Åbn vejledningen": Rust åbner manualens forside på wi24rd.com i brugerens browser. Herfra sendes kun sproget. */
  const aabnVejledning = () => (invoke ? invoke('aabn_vejledning', { sprog: S.lang }).catch(() => { }) : null);
  $('hjaelp-vejledning').setAttribute('aria-label', T.vejledningLabel);
  $('hjaelp-vejledning').addEventListener('click', () => aabnVejledning());

  /* Et klik på et CVE-nummer: Rust åbner cve.org i brugerens browser (aabn_cve validerer igen). */
  const CVE = /^CVE-[0-9]{4}-[0-9]{4,7}$/;
  const aabnCve = id => (invoke && CVE.test(id) ? invoke('aabn_cve', { id }).catch(() => { }) : null);

  window.hmRamme = {
    aabnCve, erCve: id => CVE.test(String(id)), visVaelger, landVaelger, aabnVejledning,
    tilstand, paa, udsend, antalAbonnenter, registrer, gaaTil, erstatAdresse, route, titel: opdaterTitel, gemValgte, afvisNye, status, el, fmt, vis, hjaelp,
    platform, mac, invoke, listen, hentNu, henterNu: () => hent.aktiv, hentOverblik, gemNoegle, tidsskift, bekraeft, genbesoeg: () => hist.genbesoeg, glemt: () => status($('token-status'), T.glemt, 'ok'),
  };

  function start() {
    klarHist();
    if (!invoke) { tilstand.visning = null; route(); status($('token-status'), T.mangler, 'fejl'); return; }
    if (listen) {
      listen('hentekoe', e => { visKoe(e.payload); snartOverblik(e.payload && e.payload.tilstand !== 'henter'); udsend('hentekoe', e.payload); });
      listen('hent_nu', () => { hentNu(); });
      listen('siden_sidst', () => { sidenSidst(true); });
      listen('ekstra', e => { udsend('ekstra', e.payload); });
      listen('radar', () => { udsend('radar'); snartOverblik(true); });
      listen('visning', e => { if (e.payload === 'om' || (VISNINGER.includes(e.payload) && (tilstand.sat || e.payload === 'indstillinger'))) aabnGenvej(e.payload); });
      listen('historik', e => { if (!iTekstfelt()) gaaHist(e.payload === 'frem' ? 1 : -1); });
      listen('tekst', e => { window.Tekst[{ stoerre: 'større', mindre: 'mindre' }[e.payload] || 'nulstil'](); });
    }
    window.addEventListener('online', () => { invoke('koe_prioritet', { zone: null }).catch(() => { }); });
    invoke('tidszone_saet', { skift: tidsskift() }).catch(() => { });
    vis();
    if (hist.genbesoeg) gendanRul(hist.rul[hist.trin] || 0); // efter en genindlæsning
  }
  // Visningernes scripts kommer efter dette (defer, i rækkefølge): start, når de alle har registreret sig. defer kører ved readyState
  // 'interactive', så først 'complete' eller DOMContentLoaded siger, at de sidste også har kørt.
  if (document.readyState === 'complete') setTimeout(start, 0); else document.addEventListener('DOMContentLoaded', start);
})();
