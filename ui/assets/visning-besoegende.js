/* Huginn and Muninn — Besøgende: hvor mange der besøger sitet, og hvorfra. #besoegende?zone=<id>&navn=… eller #besoegende?samlet=1, perioden
   periode=30|aar (ingen = 7 hele dage) eller fra=…&til=… (højst et år), og land=XX som filter (grafen viser så besøg fra landet pr. dag).
   Tallene regnes i Rust (besoegende, kun fra cachen): unikke besøgende er årstallene (unikke IP-adresser pr. dag), besøg pr. land er Cloudflares
   estimat ud fra en stikprøve og findes en måned tilbage plus det, programmet har gemt. Et klik på en dag i grafen åbner "Trafik den …" på Angreb,
   et klik på et land lægger det som filter her (chip med ×), og fliserne sætter perioden. Dage uden tal er huller, ikke 0. Intet uden data bag er
   klikbart. Tegnes igen, når køen eller de ekstra tal melder nyt, højst hvert 3. sekund; svar, der kommer for sent, kasseres (gen). Bygges med createElement. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, B = T.bes, R = window.hmRamme, V = window.hmTegn;
  const $ = id => document.getElementById(id);
  const { el, fmt, invoke } = R, tal = S.tal;
  const TOP = 10;
  const rod = $('besoegende-rod');

  let params = new URLSearchParams(), domaene = '…', sidste = 'samlet=1';
  const samlet = () => params.get('samlet') === '1';
  const interval = () => params.has('fra') && params.has('til');
  const land = () => (/^[A-Z0-9]{2}$/.test(params.get('land') || '') ? params.get('land') : null);
  const periodeArg = () => (interval() ? { spaend: { fra: params.get('fra'), til: params.get('til') } } : { 30: 'd30', aar: 'alt' }[params.get('periode')] || 'd7');
  const omfangQ = () => (samlet() ? { samlet: '1' } : { zone: params.get('zone') || '', navn: domaene });
  /* Adresserne er forespørgsler; gaa() gør dem til et mål i visningen. */
  const kopi = fn => { const p = new URLSearchParams(params); fn(p); return p.toString(); };
  const gaa = q => ({ gaa: 'besoegende?' + q });
  const medPeriode = v => kopi(p => { ['periode', 'fra', 'til'].forEach(k => p.delete(k)); if (v) p.set('periode', v); });
  const medInterval = (a, b) => kopi(p => { ['periode', 'fra', 'til'].forEach(k => p.delete(k)); p.set('fra', a); p.set('til', b); });
  const medLand = k => kopi(p => { if (k) p.set('land', k); else p.delete('land'); });
  const periodeNavn = () => (interval() ? V.interval(params.get('fra'), params.get('til')) : { d7: T.dykP7, d30: T.dykP30, alt: B.aar }[periodeArg()]);
  /* "Trafik den …" på Angreb for dagen (data-dyk: dagen er perioden). */
  const dagQ = dag => new URLSearchParams({ ...omfangQ(), dag, trafik: '1' }).toString();
  const procent = (n, i) => (i ? (100 * n / i).toLocaleString(S.lang === 'da' ? 'da-DK' : 'en-GB', { maximumFractionDigits: 1 }) + ' %' : '–');

  /* En flise uden tal er ikke 0 og fører ingen steder hen. */
  function flise(sum, tekst, maal, aktiv, nul) {
    if (sum === null || (sum.med === 0)) return el('div', { className: 'vg-flise' }, el('b', { textContent: '–' }), el('span', { textContent: tekst }), el('small', { textContent: nul || B.ingenTal }));
    const sub = sum.med < sum.dage ? fmt(T.dagMed, { m: tal(sum.med), n: tal(sum.dage) }) : '';
    return V.flise(sum.n, tekst, sub, gaa(maal), { aktiv, label: fmt(B.periodeAabn, { navn: tekst, n: tal(sum.n) }) });
  }
  function fliser(d) {
    const p = periodeArg(), fast = !interval(), boks = el('div', { className: 'vg-fliser besoeg-fliser' });
    const iGaar = d.i_gaar === null ? null : { n: d.i_gaar, med: 1, dage: 1 };
    boks.append(flise(iGaar, B.fIGaar, medInterval(d.i_gaar_dag, d.i_gaar_dag), interval() && params.get('fra') === d.i_gaar_dag && params.get('til') === d.i_gaar_dag));
    boks.append(flise(d.d7, B.f7, medPeriode(), fast && p === 'd7'), flise(d.d30, B.f30, medPeriode('30'), fast && p === 'd30'), flise(d.aar, B.fAar, medPeriode('aar'), fast && p === 'alt'));
    boks.append(flise(d.besoeg30, B.fBesoeg30, medPeriode('30'), false, d.ikke_tilgaengelig.length ? B.ikkeTilgKort : B.ingenTal));
    return boks;
  }
  function graf(d) {
    const k = land(), dage = d.dage.map(x => ({ dag: x.dag, n: x.n || 0, hentet: x.n !== null }));
    const h = k ? fmt(B.grafLandH, { land: V.land(k) }) : B.grafH;
    const dele = [V.soejler(dage, { titel: h, dyk: dagQ })];
    if (d.periode.med < d.periode.dage) dele.push(el('p', { className: 'vg-note hul-note', textContent: fmt(B.hulNote, { m: tal(d.periode.med), n: tal(d.periode.dage) }) }));
    const s = V.blok(h, B.grafP, ...dele);
    s.id = 'besoeg-graf';
    return s;
  }
  function tabel(rk, i_alt) {
    const th = (t, k) => { const e = el('th', { textContent: t, className: k || '' }); e.setAttribute('scope', 'col'); return e; };
    const raekker = rk.map(x => {
      const navn = V.land(x.kode), c = el('th', {}, V.knap(navn, gaa(medLand(x.kode)), 'link dyk-celle', fmt(B.landAabn, { navn, n: tal(x.n) })));
      c.setAttribute('scope', 'row');
      return el('tr', {}, c, el('td', { className: 'tal-kol', textContent: tal(x.n) }), el('td', { className: 'tal-kol', textContent: procent(x.n, i_alt) }));
    });
    return el('table', { className: 'vg-tabel besoeg-lande' }, el('thead', {}, el('tr', {}, th(B.thLand), th(B.thBesoeg, 'tal-kol'), th(B.thAndel, 'tal-kol'))), el('tbody', {}, ...raekker));
  }
  function lande(d) {
    const dele = [], ikke = d.ikke_tilgaengelig;
    if (ikke.length) dele.push(el('p', { className: 'vg-advarsel vg-gul besoeg-ikke', textContent: samlet() ? fmt(B.ikkeTilgFlere, { d: ikke.join(', ') }) : B.ikkeTilg }));
    if (!d.lande.length) {
      if (!ikke.length || d.besoeg_fra) dele.push(el('p', { className: 'vg-status dyk-tom', textContent: d.besoeg_fra ? B.landeIngen : B.landeIkkeHentet }));
    } else {
      if (d.besoeg_fra && d.besoeg_fra > d.periode.fra) dele.push(el('p', { className: 'vg-note', textContent: fmt(B.landeFra, { d: V.datoLang(d.besoeg_fra) }) }));
      const rul = el('div', { className: 'vg-tabel-rul' }, tabel(d.lande.slice(0, TOP), d.lande_i_alt));
      dele.push(rul);
      if (d.lande.length > TOP) {
        const flere = el('details', { className: 'dyk-flere' }, el('summary', { textContent: fmt(B.visAlle, { n: tal(d.lande.length) }) }), el('div', { className: 'vg-tabel-rul' }, tabel(d.lande.slice(TOP), d.lande_i_alt)));
        flere.dataset.noegle = 'besoeg-lande';
        dele.push(flere);
      }
    }
    const s = V.blok(B.landeH, B.landeP, ...dele);
    s.id = 'besoeg-lande';
    return s;
  }
  function tegn(d) {
    $('besoegende-under').textContent = domaene;
    const p = periodeArg();
    $('besoegende-periode').replaceChildren(V.segment('besoegende-periode', T.dykPeriode, [[T.dykP7, medPeriode(), !interval() && p === 'd7'], [T.dykP30, medPeriode('30'), !interval() && p === 'd30'], [B.aar, medPeriode('aar'), !interval() && p === 'alt']], 'besoegende'));
    const genaabn = V.husk(rod), dele = [V.datoVaelger({ visning: 'besoegende', periode: d.periode, nulstil: medPeriode('aar'), gaa: (a, b) => R.gaaTil('besoegende', medInterval(a, b)) })];
    const chips = [];
    if (interval()) { const n = periodeNavn(); chips.push(V.knap(`${n} ×`, gaa(medPeriode()), 'sti-chip', fmt(T.dykFjern, { navn: n }))); }
    if (land()) { const n = V.land(land()); chips.push(V.knap(`${n} ×`, gaa(medLand(null)), 'sti-chip besoeg-land-chip', fmt(T.dykFjern, { navn: n }))); }
    if (chips.length) dele.push(el('div', { className: 'login-stier besoeg-chips' }, ...chips));
    dele.push(fliser(d), graf(d), lande(d));
    dele.push(el('p', { className: 'vg-note besoeg-note', textContent: B.note + (samlet() ? ' ' + B.noteSamlet : '') }));
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(...dele);
    genaabn();
  }
  function besked(tekst, farve = 'roed') {
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(el('p', { className: `vg-advarsel vg-${farve}`, textContent: tekst }));
    $('besoegende-periode').replaceChildren();
  }

  let gen = 0, aktiv = null, afmeld = [];
  function stop() {
    gen++;
    aktiv = null;
    for (const f of afmeld.splice(0)) f();
  }
  async function start(p) {
    stop();
    const g = gen;
    aktiv = p.toString();
    sidste = aktiv;
    params = p;
    domaene = samlet() ? '…' : (p.get('navn') || '').slice(0, 253) || '…';
    S.saetDomaene(domaene, samlet());
    R.titel();
    R.visVaelger();
    rod.setAttribute('aria-busy', 'true');
    rod.replaceChildren(el('p', { className: 'vg-status', textContent: T.dykHenter }));
    if (!invoke) return besked(T.mangler);
    let zoner = null;
    try { if (samlet()) zoner = await invoke('valg_hent'); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
    if (g !== gen) return;
    if (samlet()) {
      if (!zoner.length) { besked(''); rod.replaceChildren(window.hmVelkomst.intetValgt()); return; }
      domaene = T.domaener(zoner.length);
      S.saetDomaene(domaene, true);
      R.titel();
      R.visVaelger();
    }
    const omfang = samlet() ? { zoner } : { zone: p.get('zone') || '' };
    async function hent() {
      let d;
      try { d = await invoke('besoegende', { omfang, periode: periodeArg(), land: land() }); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
      if (g === gen) tegn(d);
    }
    await hent();
    if (g !== gen) return;
    if (!R.genbesoeg()) $('besoegende-h').focus();
    let sidstHentet = Date.now(), ventende = null;
    const snart = () => {
      if (ventende) return;
      ventende = setTimeout(() => { ventende = null; sidstHentet = Date.now(); hent(); }, Math.max(0, sidstHentet + 3000 - Date.now()));
    };
    afmeld.push(R.paa('hentekoe', snart), R.paa('ekstra', snart), () => clearTimeout(ventende));
  }
  R.registrer('besoegende', {
    domaener: true,
    hvad: () => ({ samlet: samlet(), zone: params.get('zone') || '', navn: domaene }),
    /* Et andet domæne: samme periode og samme land. */
    skift(q) {
      const ny = new URLSearchParams(q);
      for (const k of ['periode', 'fra', 'til', 'land']) if (params.has(k)) ny.set(k, params.get(k));
      R.gaaTil('besoegende', ny);
    },
    vis(p, { samme } = {}) {
      const q = p.toString() ? p : new URLSearchParams(sidste);
      if (samme && aktiv === q.toString()) return;
      start(q);
    },
    skjul: stop,
    sidste: () => sidste,
  });
})();
