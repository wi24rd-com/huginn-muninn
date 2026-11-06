/* Huginn and Muninn — programrammen. Én side med hash-navigation: #overblik, #siden-sidst, #vaernet?zone=<id>&navn=…
   (eller ?samlet=1), #dyk?zone=<id>&navn=…&regel=… (visning-dyk.js), #robotter?zone=…, #login?zone=…, #land?kode=DK (visning-land.js) og #indstillinger. Uden token står opsætningen i indholdsområdet, og visningerne er slået fra (kun
   Indstillinger kan åbnes). Rammen ejer den delte tilstand (token, zoner, overblikkets rækker, valget, køens status) og er
   den eneste, der lytter på Rust-hændelserne: 'hentekoe' (køen), 'siden_sidst' (menulinjen/en notifikation), 'visning'
   (programmenuen på macOS: ⌘1–6, ⌘, og Om), 'historik' (Vis → Tilbage/Frem), 'ekstra' (de ekstra tal for et domæne er gemt) og 'radar' (nye tal fra Radar). Visningerne (visning-*.js) registrerer sig med
   vis()/skjul() og abonnerer på rammens egne beskeder (paa → afmeld), så et skift aldrig efterlader lyttere. domaener: true
   på en visning viser domænevælgeren i båndet; visningen svarer med hvad() og skift(). Domænevalget er fælles (tilstand.domaeneValg, gemt i Rust som domaene_valg): enhver adresse med zone eller samlet=1 sætter det, og sidebjælken, ⌘ og
   programmenuen åbner visningen med det. Historikken beholder sin egen adresse. land: true viser landevælgeren, som visningen selv fylder. data-dyk, data-gaa, data-cve, data-radar og data-rul på et
   element gør det klikbart: én lytter her (klik, Enter, mellemrum), så det overlever, at visningerne tegner om. Et mål i hjælpeskuffen
   lukker skuffen. I en tegning (SVG) flytter piletasterne mellem mærkerne, så tegningen kun er ét Tab-stop; .dyk-hit er den usynlige,
   større klikflade under et mærke og tæller ikke med. Genveje: på macOS står de i programmenuen (Rust), på Windows/Linux fanger
   rammen Ctrl+1–6, Ctrl+, og Ctrl+R/F5 (Hent nu). Tilbage/frem: ‹ › i båndet, ⌘[ ⌘] (menuen), ⌘← ⌘→ og Alt+← Alt+→ (keydown).
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

  /* ── visninger og router ── */
  const VISNINGER = ['overblik', 'siden-sidst', 'vaernet', 'dyk', 'robotter', 'login', 'land', 'teknologi', 'indstillinger'];
  const visninger = {};
  const registrer = (navn, v) => { visninger[navn] = v; };
  const sektion = navn => document.querySelector(`section[data-visning="${navn}"]`);
  /* Ingen decodeURIComponent her: URLSearchParams afkoder selv. To gange ødelægger rigtige værdier (/wp-config%2ephp, AT&T). */
  function laesHash() {
    const h = location.hash.replace(/^#/, ''), i = h.indexOf('?');
    return { navn: i < 0 ? h : h.slice(0, i), params: new URLSearchParams(i < 0 ? '' : h.slice(i + 1)) };
  }
  /* Skift visning. Er adressen allerede den samme, tegnes den igen (hashchange kommer ikke). */
  function gaaTil(navn, query) {
    const q = query ? String(query) : '';
    const ny = '#' + navn + (q ? '?' + q : '');
    hist.genbesoeg = false;
    if (location.hash === ny) route(); else location.hash = ny;
  }
  function route() {
    let { navn, params } = laesHash();
    if (!VISNINGER.includes(navn)) {
      if (tilstand.visning && navn) return; // fx #indhold (spring til indhold): bliv
      navn = 'overblik';
    }
    if (!tilstand.sat && navn !== 'indstillinger') navn = 'opsaetning';
    /* Et domæne, der ikke er valgt, er væk overalt, også fra en gammel adresse. Adressen erstattes, så tilbage-knappen ikke sidder fast. */
    if (tilstand.uvalgt.has(params.get('zone'))) {
      const ny = { vaernet: '#vaernet?samlet=1', robotter: '#robotter?samlet=1', login: '#login?samlet=1', teknologi: '#teknologi', indstillinger: '#indstillinger' }[navn] || '#overblik';
      history.replaceState(history.state, '', ny);
      return route();
    }
    if (visninger[navn] && visninger[navn].domaener) huskDomaene(params);
    const foer = tilstand.visning, foerParams = tilstand.params.toString();
    if (foer && foer !== navn && visninger[foer] && visninger[foer].skjul) visninger[foer].skjul();
    tilstand.visning = navn;
    tilstand.params = params;
    document.querySelectorAll('section[data-visning]').forEach(s => { s.hidden = s.getAttribute('data-visning') !== navn; });
    const punkt = navn === 'teknologi' ? 'indstillinger' : navn !== 'dyk' ? navn : params.has('robot') || params.has('robotgruppe') ? 'robotter' : 'vaernet';
    document.querySelectorAll('#side a[data-visning]').forEach(a => {
      if (a.getAttribute('data-visning') === punkt) a.setAttribute('aria-current', 'page'); else a.removeAttribute('aria-current');
    });
    document.querySelectorAll('#hjaelp .hjaelp-del').forEach(d => { d.hidden = d.getAttribute('data-hjaelp') !== navn; });
    vaelgerSynlig();
    $('band-land').hidden = !(visninger[navn] && visninger[navn].land);
    opdaterTitel();
    if (visninger[navn] && visninger[navn].vis) visninger[navn].vis(params, { samme: foer === navn && foerParams === params.toString() });
  }
  function opdaterTitel() { document.title = S.titel(tilstand.visning); }
  function navTilstand() {
    document.querySelectorAll('#side a[data-visning]').forEach(a => {
      const fri = tilstand.sat || a.getAttribute('data-visning') === 'indstillinger';
      if (fri) a.removeAttribute('aria-disabled'); else a.setAttribute('aria-disabled', 'true');
    });
  }
  // Altid gennem gaaTil: et klik på den adresse, der allerede står, ville ellers rydde history.state (trinets nummer).
  document.querySelectorAll('#side a[data-visning]').forEach(a => a.addEventListener('click', ev => {
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

  /* ── domænevælgeren i båndet: visningen siger, hvad den viser (hvad), og hvad et skift betyder (skift) ── */
  const vaelger = $('vg-zone');
  /* Er intet domæne valgt, er der intet at vælge imellem: ingen "Samlet (0)". Skjult, så Tab springer den over. */
  function vaelgerSynlig() {
    const v = visninger[tilstand.visning];
    $('band-zone').hidden = !(v && v.domaener && tilstand.raekker.length);
  }
  function visVaelger() {
    vaelgerSynlig();
    const v = visninger[tilstand.visning];
    if (!v || !v.domaener) return;
    const { samlet, zone, navn } = v.hvad(), zoner = [...tilstand.raekker].sort((a, b) => a.navn.localeCompare(b.navn));
    vaelger.replaceChildren();
    const opt = (value, tekst) => vaelger.append(el('option', { value, textContent: tekst }));
    opt('samlet', fmt(T.samletValg, { n: zoner.length }));
    for (const r of zoner) opt(r.id, r.navn);
    if (!samlet && zone && !zoner.some(r => r.id === zone)) opt(zone, navn);
    vaelger.value = samlet ? 'samlet' : zone;
  }
  vaelger.addEventListener('change', () => {
    const v = visninger[tilstand.visning];
    if (!v || !v.domaener) return;
    if (vaelger.value === 'samlet') v.skift('samlet=1');
    else v.skift(new URLSearchParams({ zone: vaelger.value, navn: vaelger.options[vaelger.selectedIndex].textContent }));
  });
  paa('raekker', visVaelger);

  /* ── klikbart med data bag: data-dyk = forespørgslen til #dyk, data-gaa = en anden visning ("vaernet?zone=…"), data-cve = cve.org,
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
    if (!gaa) return gaaTil('dyk', e.getAttribute('data-dyk'));
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
  const GENVEJE = [['1', 'overblik', 'navOverblik'], ['2', 'siden-sidst', 'navSiden'], ['3', 'vaernet', 'navVaernet'], ['4', 'robotter', 'navRobotter'], ['5', 'login', 'navLogin'], ['6', 'land', 'navLand'], [',', 'indstillinger', 'navIndst']];
  const genvejTekst = tast => (mac ? '⌘' : 'Ctrl+') + (tast === 'r' ? 'R' : tast);
  const histGenvej = [mac ? '⌘[' : 'Alt+←', mac ? '⌘]' : 'Alt+→'];
  for (const [id, tekst, genvej] of [['hist-tilbage', T.tilbage, histGenvej[0]], ['hist-frem', T.frem, histGenvej[1]]]) {
    $(id).title = `${tekst} (${genvej})`;
    $(id).setAttribute('aria-label', tekst);
  }
  document.querySelectorAll('#side kbd[data-genvej]').forEach(k => { k.textContent = genvejTekst(k.getAttribute('data-genvej')); });
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
    // Tekststørrelse. '=' er + uden Skift på et amerikansk tastatur, og programmenuens ⌘+ fanger kun tegnet +.
    const tekstTrin = { '+': 1, '=': 1, '-': -1, 0: 0 }[ev.key];
    if (tekstTrin !== undefined && !ev.altKey && (mac ? ev.metaKey && !ev.ctrlKey : ev.ctrlKey && !ev.metaKey)) { ev.preventDefault(); window.hmTekst.trin(tekstTrin); return; }
    if (mac) return; // macOS: programmenuen (Rust) har genvejene
    if (ev.key === 'F5' || (ev.ctrlKey && !ev.altKey && (ev.key === 'r' || ev.key === 'R'))) { ev.preventDefault(); hentNu(); return; }
    if (!ev.ctrlKey || ev.altKey || ev.metaKey) return;
    const g = GENVEJE.find(([tast]) => tast === ev.key);
    if (!g) return;
    ev.preventDefault();
    if (g[1] === 'indstillinger' || tilstand.sat) aabnGenvej(g[1]);
  });

  /* ── hjælpeskuffen: den generelle forklaring ("?") eller en guide (Overblikkets "Hvad gør jeg?": { titel: nøglen i T, indhold }) ── */
  function hjaelp(aaben, guide) {
    const g = $('hjaelp-guide');
    if (aaben) {
      g.replaceChildren(...(guide ? [guide.indhold] : []));
      g.hidden = !guide;
      $('hjaelp-generelt').hidden = Boolean(guide);
      $('hjaelp-h').dataset.t = guide ? guide.titel : 'hjaelpH'; // data-t, så sprog.js' anvend() ikke skriver en anden tekst tilbage
      $('hjaelp-h').textContent = T[$('hjaelp-h').dataset.t];
    }
    $('hjaelp').hidden = !aaben;
    $('hjaelp-knap').setAttribute('aria-expanded', String(aaben));
    if (aaben) $('hjaelp-luk').focus(); else $('hjaelp-knap').focus();
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
      t = k.zoner_fejl ? fmt(T.koeFaerdigFejl, { t: S.kl(sidst), n: T.domaener(k.zoner_fejl), f: S.fejl(k.fejl) }) : fmt(T.koeFaerdig, { t: S.kl(sidst) });
      klasse = k.zoner_fejl ? 'gul' : 'ok';
      k1 = fmt(k.zoner_fejl ? T.pilleOpdateretFejl : T.pilleOpdateret, { t: S.tid(sidst || new Date().toISOString()), n: k.zoner_fejl });
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
    const aabn = el('button', { type: 'button', className: 'knap', textContent: T.tokenAabn });
    aabn.addEventListener('click', () => { if (invoke) invoke('aabn_api_token').catch(() => { }); });
    const trin = el('details', { className: 'hm-hjaelp token-trin' }, el('summary', { textContent: T.hjaelpTokenH }), el('ol', {}, ...T.tokenTrin.map(t => el('li', { textContent: t }))));
    plads.append(aabn, el('p', { className: 'note', textContent: T.tokenAabnNote }), trin);
  });
  $('token-form').addEventListener('submit', ev => { ev.preventDefault(); gemNoegle($('token'), $('token-knap'), $('token-status')); });

  /* Et klik på et CVE-nummer: Rust åbner cve.org i brugerens browser (aabn_cve validerer igen). */
  const CVE = /^CVE-[0-9]{4}-[0-9]{4,7}$/;
  const aabnCve = id => (invoke && CVE.test(id) ? invoke('aabn_cve', { id }).catch(() => { }) : null);

  window.hmRamme = {
    aabnCve, erCve: id => CVE.test(String(id)), visVaelger,
    tilstand, paa, udsend, antalAbonnenter, registrer, gaaTil, route, titel: opdaterTitel, gemValgte, afvisNye, status, el, fmt, vis, hjaelp,
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
      listen('tekst', e => { window.hmTekst.trin({ stoerre: 1, mindre: -1 }[e.payload] || 0); });
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
