/* Huginn and Muninn — Robotter: hvem der læser sitet. #robotter?zone=<id>&navn=… eller #robotter?samlet=1 og periode=30|alt (ingen = 7
   hele dage) eller fra=…&til=…. Tallene regnes i Rust (robotter, kun fra cachen), og et klik åbner samme tal i dyk (robot, robotgruppe, sti). Grupperne er
   Cloudflares verificerede kategorier; noterne om blokering er vores egne og siger "kan". Labyrinten står for sig og kaldes aldrig
   blokeret: Cloudflare regner den ikke for en mitigering. Tegnes igen, når køen eller de ekstra tal melder nyt, højst hvert 3. sekund;
   svar, der kommer for sent, kasseres (gen). Bygges med createElement. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, R = window.hmRamme, V = window.hmTegn;
  const $ = id => document.getElementById(id);
  const { el, fmt, invoke } = R, tal = S.tal;
  const TOP_SYNLIGE = 5;
  const rod = $('robotter-rod');

  let params = new URLSearchParams(), domaene = '…', sidste = 'samlet=1';
  const samlet = () => params.get('samlet') === '1';
  const interval = () => params.has('fra') && params.has('til');
  const periodeArg = () => (interval() ? { spaend: { fra: params.get('fra'), til: params.get('til') } } : { 30: 'd30', alt: 'alt' }[params.get('periode')] || 'd7');
  const omfangQ = () => (samlet() ? { samlet: '1' } : { zone: params.get('zone'), navn: domaene });
  /* Samme periode i dyk, hvor ingen periode betyder 30 dage. */
  const dykPeriode = () => (interval() ? { fra: params.get('fra'), til: params.get('til') } : { d7: { periode: '7' }, d30: {}, alt: { periode: 'alt' } }[periodeArg()]);
  const dyk = x => new URLSearchParams({ ...omfangQ(), ...x, ...dykPeriode() }).toString();
  const medPeriode = v => { const p = new URLSearchParams(params); ['periode', 'fra', 'til'].forEach(k => p.delete(k)); if (v) p.set('periode', v); return p.toString(); };
  const medInterval = (a, b) => { const p = new URLSearchParams(medPeriode()); p.set('fra', a); p.set('til', b); return p.toString(); };
  const periodeNavn = () => (interval() ? V.interval(params.get('fra'), params.get('til')) : { d7: T.dykP7, d30: T.dykP30, alt: T.dykPAlt }[periodeArg()]);

  function fliser(d) {
    const i = d.crawlere.iAlt, boks = el('div', { className: 'sfliser vg-fliser' }), p = periodeNavn();
    boks.append(V.flise(i.bots7, T.robotBesoeg, p, dyk({ robot: 'alle' }), { aktiv: true }));
    /* En andel har ingen rækker bag sig: den er ikke klikbar. */
    const andel = i.alle7 ? (100 * i.bots7 / i.alle7).toLocaleString(S.lang === 'da' ? 'da-DK' : 'en-GB', { maximumFractionDigits: 1 }) + ' %' : '–';
    boks.append(V.fliseTekst(andel, T.robotAndel, p));
    boks.append(V.flise(i.falske7, T.robotSlags.udklaedt, T.robotUdklaedtSub, dyk({ robot: 'udklaedt' })));
    boks.append(V.flise(d.stoppet.sum, T.robotSlags.stoppet, T.robotStoppetSub, dyk({ robot: 'stoppet' })));
    boks.append(V.flise(d.stoppet.labyrint, T.robotSlags.labyrint, T.robotLabyrintSub, dyk({ robot: 'labyrint' })));
    return boks;
  }
  function grupper(d) {
    const ul = el('ul', { className: 'hm-grupper' });
    for (const g of d.grupper) {
      const [navn, hvad, konsekvens] = T.robotGrupper[g.id];
      const b = V.knap('', dyk({ robotgruppe: g.id }), 'gruppe-kort', fmt(T.dykAabn, { navn, n: tal(g.n) }));
      b.append(el('b', { className: 'gr-navn', textContent: navn }), el('b', { className: 'gr-n', textContent: tal(g.n) }), el('small', { textContent: hvad }), el('span', { className: 'konsekvens', textContent: konsekvens }));
      ul.append(el('li', {}, b));
    }
    return V.blok(T.robotHvemH, T.robotHvemP, ul);
  }
  function liste(h, noegle, rk, antal) {
    return el('div', {}, el('h3', { textContent: h }), ...V.listeFlere(rk, TOP_SYNLIGE, noegle, fmt(T.dykFlere, { n: tal(antal) })));
  }
  function lister(d) {
    const c = d.crawlere, boks = el('div', { className: 'vg-tre' });
    const navn = (slags, x) => ({ navn: V.robotNavn(x.v), n: x.n, dyk: dyk({ robot: `${slags}:${x.v}` }) });
    const bots = c.bots.map(b => ({ navn: V.robotNavn(b.navn) + (b.firma ? ` · ${b.firma}` : ''), n: b.n7, dyk: dyk({ robot: b.navn }) }));
    if (bots.length) boks.append(liste(T.robotMestAktive, 'bots', bots, c.iAlt.botNavne));
    if (d.sider.length) boks.append(liste(T.robotMestLaeste, 'sider', d.sider.map(x => ({ navn: x.v, kode: true, n: x.n, dyk: dyk({ robot: 'alle', sti: x.v }) })), d.antal_sider));
    if (d.stoppet.robotter.length) boks.append(liste(T.robotSlags.stoppet, 'stoppet', d.stoppet.robotter.map(x => navn('stoppet', x)), d.stoppet.robotter.length));
    if (d.stoppet.labyrint_robotter.length) boks.append(liste(T.robotSlags.labyrint, 'labyrint', d.stoppet.labyrint_robotter.map(x => navn('labyrint', x)), d.stoppet.labyrint_robotter.length));
    const falske = (slags, rk) => rk.map(x => ({ navn: V.robotNavn(x.navn), n: x.n, dyk: dyk({ robot: `${slags}:${x.navn}` }) }));
    if (c.falske.length) boks.append(liste(T.robotSlags.udklaedt, 'udklaedt', falske('udklaedt', c.falske), c.falske.length));
    if (c.uverificerede.length) boks.append(liste(T.robotSlags.uverificeret, 'uverificeret', falske('uverificeret', c.uverificerede), c.uverificerede.length));
    return boks.childElementCount ? [V.blok(T.dykListerH, '', boks)] : [];
  }
  /* "Angreb fra robotter" i båndet åbner de stoppede robotter i Angreb for samme domæne og periode. */
  let robotAngreb = null;
  $('robot-angreb').addEventListener('click', () => R.gaaTil('vaernet', robotAngreb || dyk({ robot: 'stoppet' })));
  function tegn(d) {
    $('robotter-under').textContent = domaene;
    const p = periodeArg();
    R.baandValg('robotter-periode', [[T.dykP7, medPeriode(), p === 'd7'], [T.dykP30, medPeriode('30'), p === 'd30'], [T.dykPAlt, medPeriode('alt'), p === 'alt']], 'robotter');
    robotAngreb = dyk({ robot: 'stoppet' });
    const genaabn = V.husk(rod), dele = [V.datoVaelger({ visning: 'robotter', plads: 'robotter-datoer', tom: !interval(), periode: d.periode, gaa: (a, b) => R.gaaTil('robotter', medInterval(a, b)) })];
    if (interval()) { const n = periodeNavn(); dele.push(el('div', { className: 'login-stier' }, V.fjernChip(n, { gaa: 'robotter?' + medPeriode() }))); }
    dele.push(fliser(d), ...V.daekning(d.periode));
    if (d.stoppet.ikke_tilgaengelig.length) dele.push(el('p', { className: 'vg-advarsel vg-gul', textContent: fmt(T.robotIkkeTilg, { d: d.stoppet.ikke_tilgaengelig.map(V.dato).join(', ') }) }));
    else if (d.stoppet.mangler_dage) dele.push(el('p', { className: 'vg-note', textContent: fmt(T.ekstraMangler, { n: tal(d.stoppet.mangler_dage) }) }));
    if (d.grupper.length) dele.push(grupper(d), ...lister(d));
    else dele.push(el('p', { className: 'vg-status dyk-tom', textContent: T.robotTomt }));
    dele.push(el('p', { className: 'vg-note', textContent: T.robotNote }));
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(...dele);
    genaabn();
  }
  function besked(tekst, farve = 'roed') {
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(el('p', { className: `vg-advarsel vg-${farve}`, textContent: tekst }));
    R.baandValg('robotter-periode', null); R.iBaand('robotter-datoer');
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
      try { d = await invoke('robotter', { omfang, periode: periodeArg() }); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
      if (g === gen) tegn(d);
    }
    await hent();
    if (g !== gen) return;
    if (!R.genbesoeg()) $('robotter-h').focus();
    let sidstHentet = Date.now(), ventende = null;
    const snart = () => {
      if (ventende) return;
      ventende = setTimeout(() => { ventende = null; sidstHentet = Date.now(); hent(); }, Math.max(0, sidstHentet + 3000 - Date.now()));
    };
    afmeld.push(R.paa('hentekoe', snart), R.paa('ekstra', snart), () => clearTimeout(ventende));
  }
  const ord = $('ord-robotter');
  for (const id of ['robot', 'verificeret', 'udklaedt', 'linkpreview', 'labyrint']) ord.append(el('dt', { textContent: S.bog(id).ord }), el('dd', { textContent: S.bog(id).alle }));
  R.registrer('robotter', {
    domaener: true,
    hvad: () => ({ samlet: samlet(), zone: params.get('zone') || '', navn: domaene }),
    /* Et andet domæne: samme periode. */
    skift(q) {
      const ny = new URLSearchParams(q);
      for (const k of ['periode', 'fra', 'til']) if (params.has(k)) ny.set(k, params.get(k));
      R.gaaTil('robotter', ny);
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
