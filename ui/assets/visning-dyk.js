/* Huginn and Muninn — dyk: de gemte tal for det, der blev klikket på. #dyk?zone=<id>&navn=… eller #dyk?samlet=1, plus filtrene regel,
   land, netvaerk, sti, colo, metode, handling, forsoeg, robot, robotgruppe og teknologi (højst ét pr. dimension, J3; rækkefølgen i adressen er
   rækkefølgen i chippene; robotter kan kun kombineres med sti), slags (angreb|rutine, ellers alt, J4) og perioden: periode=7|alt (ingen = 30 hele dage), dag=… eller fra=…&til=…. forsoeg=alle er alle stier med 404. side=N er siden i "De seneste". trafik=1 med dag=… eller fra=…&til=… er trafik-dyk: forespørgslerne pr. land, svarkode og robot (trafik_dag), ikke det, Cloudflare stoppede.
   Tallene regnes i Rust (dyk, kun fra cachen). Et klik på en værdi lægger et filter oveni (rammens data-dyk), og hvert skift er ét
   trin i tilbage/frem. Listen for en filtreret dimension står ikke. Stier står renset (J2) og sættes altid som tekst.
   Tegnes igen, når køen melder nyt, højst hvert 3. sekund; svar, der kommer for sent, kasseres (gen). Bygges med createElement. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, R = window.hmRamme, V = window.hmTegn;
  const $ = id => document.getElementById(id);
  const { el, fmt, invoke } = R, tal = S.tal;
  const FILTRE = ['regel', 'land', 'netvaerk', 'sti', 'colo', 'metode', 'handling', 'forsoeg', 'robot', 'robotgruppe', 'teknologi'];
  const PERIODE = ['periode', 'dag', 'fra', 'til'];
  const TOP_SYNLIGE = 5;
  const rod = $('dyk-rod');

  /* ── adressen ── */
  let params = new URLSearchParams(), domaene = '…', antal = 0;
  const samlet = () => params.get('samlet') === '1';
  const filtre = () => [...params.keys()].filter(k => FILTRE.includes(k)).map(dim => ({ dim, vaerdi: params.get(dim) }));
  const har = dim => params.has(dim);
  const erRobot = () => har('robot') || har('robotgruppe');
  const erTrafik = () => har('trafik') && (har('dag') || (har('fra') && har('til')));
  function periodeArg() {
    if (har('dag')) return { dag: params.get('dag') };
    if (har('fra') && har('til')) return { spaend: { fra: params.get('fra'), til: params.get('til') } };
    return { 7: 'd7', alt: 'alt' }[params.get('periode')] || 'd30';
  }
  /* Et nyt udsnit starter på første side i "De seneste". */
  const kopi = fn => { const p = new URLSearchParams(params); p.delete('side'); fn(p); return p.toString(); };
  const medFilter = (dim, v) => kopi(p => p.set(dim, v));
  const udenFilter = dim => kopi(p => p.delete(dim));
  const medPeriode = (k, v) => kopi(p => { PERIODE.forEach(x => p.delete(x)); if (k) p.set(k, v); });
  const medSlags = v => kopi(p => { if (v) p.set('slags', v); else p.delete('slags'); });
  const omfangQ = () => (samlet() ? { samlet: '1' } : { zone: params.get('zone'), navn: domaene });
  const oversigt = () => `${erRobot() ? 'robotter' : 'vaernet'}?${new URLSearchParams(omfangQ())}`;
  const hvor = () => (samlet() ? fmt(T.dykDine, { n: antal }) : domaene);

  /* ── navne ── */
  function filterNavn(f, d) {
    switch (f.dim) {
      case 'regel': return d.regel ? V.regelVis(d.regel).titel : V.regelTitel(f.vaerdi);
      case 'land': return V.land(f.vaerdi);
      case 'colo': return V.colo(f.vaerdi);
      case 'handling': return T.dykHandling[f.vaerdi] || f.vaerdi;
      case 'forsoeg': return (T.kat[f.vaerdi] || [f.vaerdi])[0];
      case 'robot': return V.robotValg(f.vaerdi);
      case 'robotgruppe': return (T.robotGrupper[f.vaerdi] || [f.vaerdi])[0];
      case 'teknologi': return S.tekNavn(f.vaerdi);
      default: return f.vaerdi;
    }
  }
  const periodeTekst = () => (har('dag') ? V.datoLang(params.get('dag')) : har('fra') ? V.interval(params.get('fra'), params.get('til')) : params.get('periode') === '7' ? T.dykP7 : params.get('periode') === 'alt' ? T.dykPAlt : T.dykP30);
  /* Kun angreb tæller som statusreglen, også det, der ikke blev afvist (fx log fra egne regler). */
  const ord = () => (erRobot() ? (/^(stoppet|labyrint)/.test(params.get('robot') || '') ? T.dykHaendelser : T.dykBesoeg) : ['faelder', 'andet'].includes(params.get('handling')) || har('forsoeg') ? T.dykHaendelser : params.get('slags') === 'angreb' ? T.dykAngrebOrd : T.dykAfvist);

  /* ── byggeklodser ── */
  const knap = (tekst, query, klasse, label) => V.knap(tekst, query, klasse || 'link', label);
  const segment = (navn, etiket, valg) => V.segment(navn, etiket, valg, 'dyk');
  const blok = V.blok;
  const flise = (n, tekst, sub, query, aktiv) => V.flise(n, tekst, sub, query, { aktiv });

  /* ── dele af visningen ── */
  /* Knappen til hele oversigten: en fast adresse, ikke historikken, for man kan være kommet ind ad et link. */
  function tilbage() {
    const tekst = fmt(erRobot() ? T.dykTilbageRobot : T.dykTilbage, { d: hvor() });
    $('dyk-tilbage').replaceChildren(V.knap(`← ${tekst}`, { gaa: oversigt() }, 'knap', tekst));
  }
  /* Overskriften siger det første filter, så dyk ikke kan forveksles med Angreb. Dagen tæller med; perioden og slags står i vælgerne. */
  function overskrift(d, liste) {
    const H = T.dykOverskrift, f = liste[0], dag = har('dag') ? V.datoLang(params.get('dag')) : '', hvad = ord();
    const o = params.get('slags') === 'rutine' ? T.dykHaendelser : hvad !== T.dykAfvist ? hvad : d.regel && d.regel.art === 'rutine' ? T.dykHaendelser : T.dykAngrebOrd, x = { o, d: hvor() };
    if (!f) return fmt(dag ? H.dag : H.alle, { ...x, v: dag });
    if (f.dim === 'robotgruppe' || V.ROBOT_SLAGS.includes(f.vaerdi)) return fmt(H.gruppe, { ...x, v: filterNavn(f, d) });
    return fmt(H[f.dim], { ...x, v: f.dim === 'robot' ? V.robotNavn(f.vaerdi.slice(f.vaerdi.indexOf(':') + 1)) : filterNavn(f, d) });
  }
  /* Ét filter pr. dimension (J3). × fjerner kun det; er det det sidste, er der intet udsnit tilbage, og man står i oversigten. */
  function chips(d, liste) {
    const led = liste.map(f => ({ navn: filterNavn(f, d), fjern: udenFilter(f.dim) }));
    if (har('dag')) led.push({ navn: V.datoLang(params.get('dag')), fjern: medPeriode() });
    if (har('fra') && har('til')) led.push({ navn: V.interval(params.get('fra'), params.get('til')), fjern: medPeriode() });
    const ul = el('ul');
    for (const l of led) {
      const indhold = el('span', { className: 'dyk-filter-tekst' }, el('span', { textContent: l.navn }), el('span', { className: 'dyk-filter-x', textContent: '×' }));
      ul.append(el('li', {}, V.knap(indhold, led.length === 1 ? { gaa: oversigt() } : l.fjern, 'dyk-filter', fmt(T.dykFjern, { navn: l.navn }))));
    }
    const nav = $('dyk-filtre');
    nav.setAttribute('aria-label', T.dykFiltre);
    nav.hidden = !led.length;
    nav.replaceChildren(ul);
  }
  /* Hovedet efter filter: regel (mærke, CVE, Cloudflares navn og guiden) og noter til de andre filtre. */
  function hoved(d) {
    const ud = [];
    if (d.regel) {
      const r = d.regel, vis = V.regelVis(r), top = el('div', { className: 'dyk-hoved-top' }, el('span', { className: 'vg-produkt', textContent: vis.maerke }));
      if (r.cve) top.append(el('span', { className: 'vg-cve' }, V.cveKnap(r.cve, `${r.cve} ↗`)));
      const boks = el('div', { className: 'dyk-hoved' + (r.art === 'rutine' ? ' dyk-rutine' : '') }, top);
      const type = V.typeTekst(r.navn, r.type);
      if (r.art === 'angreb' && type) boks.append(el('p', { className: 'dyk-type', textContent: type }));
      boks.append(el('p', { className: 'dyk-navn' }, T.dykCloudflareNavn, ' ', el('code', { textContent: r.navn })));
      boks.append(el('details', { className: 'dyk-guide' }, el('summary', { textContent: T.gTitel }), ...V.bogAfsnit(vis.id)));
      ud.push(boks);
    }
    const note = t => ud.push(el('p', { className: 'vg-note dyk-note', textContent: t }));
    if (har('netvaerk')) note(T.dykNetNote);
    if (har('sti')) { const k = /^\[([a-z]+)\]$/.exec(params.get('sti')); if (k && T.kat[k[1]]) note(T.kat[k[1]][1]); }
    if (har('forsoeg')) { const k = T.kat[params.get('forsoeg')]; note(k ? `${k[1]} ${T.dykForsoegNote}` : T.dykForsoegNote); }
    if (har('robotgruppe') && T.robotGrupper[params.get('robotgruppe')]) note(T.robotGrupper[params.get('robotgruppe')][2]);
    if (/^labyrint/.test(params.get('robot') || '')) note(T.robotLabyrintNote);
    return ud;
  }
  function slagsVaelger(d) {
    const nu = params.get('slags'), boks = el('div', { className: 'dyk-slags' }, el('span', { textContent: T.dykSlagsH }));
    boks.append(segment('dyk-slags', T.dykSlagsH, [[T.dykSAlt, medSlags(''), !nu], [fmt(T.dykSAngreb, { n: tal(d.slags.angreb) }), medSlags('angreb'), nu === 'angreb'], [fmt(T.dykSRutine, { n: tal(d.slags.rutine) }), medSlags('rutine'), nu === 'rutine']]));
    return boks;
  }
  function fliser(d) {
    const k = d.noegletal, o = ord(), p = params.get('periode'), iDag = fmt(T.dykFIDag, { n: tal(k.i_dag_hidtil) });
    const boks = el('div', { className: 'vg-fliser' }), med = (m, n, ellers) => (m < n ? fmt(T.dagMed, { m: tal(m), n: tal(n) }) : ellers);
    if (har('dag')) boks.append(flise(d.sum, fmt(T.dykFDag, { o, d: V.datoLang(params.get('dag')) }), '', '', true));
    if (har('fra')) { const [a, b] = S.datoPar(params.get('fra'), params.get('til')); boks.append(flise(d.sum, fmt(T.dykFPeriode, { o, a, b }), med(d.periode.med, d.periode.dage, ''), '', true)); }
    /* Tal kun i dag: flisen siger begge dele, så den ikke modsiger "Først set". */
    const altSub = d.kun_i_dag ? fmt(T.dykHeleIDag, { h: tal(k.alt), n: tal(k.i_dag_hidtil) }) : med(d.periode.med, d.periode.dage, '');
    if (p === 'alt') boks.append(flise(k.alt, fmt(T.dykFAlt, { o }), altSub, medPeriode('periode', 'alt'), true));
    const fast = !har('dag') && !har('fra') && p !== 'alt';
    boks.append(flise(k.hele30, fmt(T.dykF30, { o }), med(k.med30, 30, T.dykF30Sub), medPeriode(), fast && p !== '7'), flise(k.hele7, fmt(T.dykF7, { o }), med(k.med7, 7, ''), medPeriode('periode', '7'), fast && p === '7'));
    boks.append(V.flise(k.i_gaar, fmt(T.dykFIGaar, { o }), iDag, medPeriode('dag', d.i_gaar), { ekstra: { n: k.i_dag_hidtil, dyk: medPeriode('dag', d.i_dag) } }));
    const dato = (h, dag) => el('div', { className: 'dyk-dato' }, el('span', { textContent: h }), knap(V.datoLang(dag), medPeriode('dag', dag), 'link', fmt(T.dykAabn, { navn: h, n: V.datoLang(dag) })));
    if (d.foerst) boks.append(el('div', { className: 'vg-flise dyk-datoer' }, dato(T.dykFoerst, d.foerst), dato(T.dykSidst, d.sidst)));
    boks.append(...forespoergsler(d));
    return boks;
  }
  /* Adressen til trafik-dyk for en periode. */
  const trafikQ = (a, b) => kopi(p => { PERIODE.forEach(x => p.delete(x)); FILTRE.forEach(x => p.delete(x)); p.delete('slags'); if (a === b) p.set('dag', a); else { p.set('fra', a); p.set('til', b); } p.set('trafik', '1'); });
  /* Forholdet mellem det, der står her, og alle forespørgsler. Kun uden filtre: et filter gælder ikke forespørgslerne. Robotter tæller besøg, og forsøg er for sig. */
  function forespoergsler(d) {
    const antal = erRobot() ? d.sum : d.slags.angreb, ren = erRobot() ? !har('sti') : !filtre().length;
    if (!d.trafik.i_alt || !ren) return [];
    const sub = antal ? fmt(T.dykForespSub, { o: erRobot() ? T.dykBesoeg : T.dykAngrebOrd, n: tal(antal), m: tal(Math.round(d.trafik.i_alt / antal)) }) : '';
    return [V.flise(d.trafik.i_alt, T.dykForesp, sub, trafikQ(d.periode.fra, d.periode.til), { label: fmt(T.dykForespAabn, { n: tal(d.trafik.i_alt) }) })];
  }
  /* En periode med usædvanlig meget trafik siger det øverst, for angrebene alene ser små ud. */
  function trafikLinje(d) {
    const dage = d.trafik.dage;
    if (!dage.length || filtre().length) return [];
    const a = dage[0].dag, b = dage[dage.length - 1].dag, sum = k => dage.reduce((x, t) => x + t[k], 0), n = tal(sum('n'));
    const tekst = fmt(T.dykTrafikLinje, { t: T.trafikMaerke, p: V.interval(a, b), n, m: tal(Math.round(dage[0].median)), stop: S.trafikStop(sum('n'), sum('stoppet')) });
    return [el('p', { className: 'vg-advarsel vg-gul dyk-trafik-linje' }, tekst, ' ', knap(T.dykSeTrafikken, trafikQ(a, b), 'link', T.dykSeTrafikken))];
  }
  /* Et udsnit, der kun har tal i dag, må ikke ligne 0. */
  function kunIDag(d, liste) {
    const k = d.kun_i_dag;
    if (!k) return [];
    const n = d.noegletal.i_dag_hidtil, flere = ord().toLowerCase(), o = n === 1 ? ({ hændelser: 'hændelse', visits: 'visit', attacks: 'attack', events: 'event' }[flere] || flere) : flere;
    const navn = liste.length ? filterNavn(liste[0], d) : T.dykKunIDagHer, boks = el('div', { className: 'vg-advarsel dyk-kun-i-dag' }, el('p', { textContent: fmt(T.dykKunIDag, { navn, n: tal(n), o }) }));
    const linje = (h, rk) => { if (rk.length) boks.append(el('p', {}, el('b', { textContent: `${h}: ` }), rk.join(', '))); };
    linje(T.dykKunStier, k.stier.slice(0, 3).map(x => `${x.v} (${tal(x.n)})`));
    linje(T.dykKunLande, k.lande.slice(0, 3).map(x => `${V.land(x.v)} (${tal(x.n)})`));
    if (k.t) linje(T.dykKunTid, [S.datoTid(k.t)]);
    boks.append(el('p', {}, knap(T.dykKunAabn, medPeriode('dag', d.i_dag), 'link', T.dykKunAabn)));
    return [boks];
  }
  function fordeling(d) {
    if (har('handling') || har('forsoeg') || erRobot()) return [];
    const f = d.fordeling, ul = el('ul', { className: 'vg-fordeling' }), linje = el('li', {}, el('b', { textContent: `${periodeTekst()}:` }), ' ');
    const dele = [['blokeret', T.nBlokeret], ['udfordret', T.nUdfordret], ['labyrint', T.nLabyrint]].filter(([k]) => f[k]);
    dele.forEach(([k, t], i) => { if (i) linje.append(' · '); linje.append(knap(fmt(t, { n: tal(f[k]) }), medFilter('handling', k), 'link dyk-celle')); });
    if (dele.length) ul.append(linje);
    if (f.faelder) ul.append(el('li', { className: 'vg-faelder' }, knap(T.faelderLinje(tal(f.faelder)), medFilter('handling', 'faelder'), 'link dyk-celle')));
    if (f.andet) ul.append(el('li', {}, knap(fmt(T.nAndet, { n: tal(f.andet) }), medFilter('handling', 'andet'), 'link dyk-celle')));
    return ul.childElementCount ? [ul] : [];
  }
  /* Går perioden længere tilbage end sikkerhedshændelserne, står trafikken for hele perioden, og angrebene kun, hvor de findes. */
  function soejleBlok(d) {
    if (har('dag')) return [];
    const fra = d.periode.sikkerhed_fra, sikre = d.trafik_serie.length ? d.dage.filter(x => x.dag >= fra) : d.dage;
    const tavle = el('div', { className: 'vg-tavle vg-tavle-pad' }, el('div', { className: 'vg-soejler-rul' }, V.soejler(sikre, { titel: T.dykSoejleTitel, dyk: dag => medPeriode('dag', dag) })));
    const trafik = d.trafik_serie.length ? [blok(T.dykTrafikDage, T.dykTrafikDageP, el('div', { className: 'vg-tavle vg-tavle-pad' }, el('div', { className: 'vg-soejler-rul' }, V.soejler(d.trafik_serie, { titel: T.dykTrafikSoejle, hul: T.dykTrafikHul, dyk: dag => trafikQ(dag, dag) }))))] : [];
    return [...trafik, blok(fmt(erRobot() ? T.dykRobotDage : T.dykEdge, { p: periodeTekst() }), T.dykEdgeP, tavle)];
  }
  /* Sikkerhedstal over en måned gamle findes ikke hos Cloudflare: det skal stå tydeligt, når perioden når længere tilbage. */
  function kunTrafik(d) {
    const fra = d.periode.sikkerhed_fra;
    return fra && d.periode.fra < fra ? [el('p', { className: 'vg-advarsel vg-gul dyk-kun-trafik', textContent: fmt(T.dykKunTrafik, { d: V.datoLang(fra) }) })] : [];
  }
  function kortBlok(d) {
    const v = { lande: d.top.land.map(x => ({ kode: x.v, n: x.n })), colos: d.top.colo.map(x => ({ kode: x.v, n: x.n })), buer: d.buer };
    if (har('forsoeg') || !v.lande.length) return [];
    const dyk = {
      land: k => medFilter('land', k),
      colo: k => medFilter('colo', k),
      bue: (l, c) => kopi(p => { p.set('land', l); p.set('colo', c); }),
    };
    const legende = el('ul', { className: 'vg-legende' }, el('li', {}, el('i', { className: 'vg-leg-land' }), T.dykLegLand), el('li', {}, el('i', { className: 'vg-leg-colo' }), T.dykLegColo), el('li', { textContent: T.dykLegSamme }));
    const uden = V.udenPlads(v), flereLande = d.antal.land - v.lande.length, flereColos = d.antal.colo - v.colos.length;
    if (uden.length) legende.append(el('li', { textContent: fmt(T.kortUdenPlads, { liste: uden.join(', ') }) }));
    if (flereLande > 0) legende.append(el('li', { textContent: fmt(T.kortFaaLande, { n: tal(flereLande) }) }));
    if (flereColos > 0) legende.append(el('li', { textContent: fmt(T.kortFaaColos, { n: tal(flereColos) }) }));
    if (d.buer_i_alt > d.buer.length) legende.append(el('li', { textContent: fmt(T.kortBuer, { n: tal(d.buer.length), m: tal(d.buer_i_alt) }) }));
    return [blok(T.dykKortH, T.dykKortP, el('div', { className: 'vg-tavle' }, V.kort(v, 'verden', { titel: T.dykKortTitel, tal: n => tal(n), dyk }), legende))];
  }
  const RAEKKER = {
    land: x => ({ navn: V.land(x.v) }),
    netvaerk: x => ({ navn: x.v }),
    sti: x => ({ navn: x.v, kode: true }),
    colo: x => ({ navn: V.colo(x.v) }),
    metode: x => ({ navn: x.v }),
    handling: x => ({ navn: T.dykHandling[x.v] || x.v }),
    robot: x => ({ navn: V.robotNavn(x.v) }),
    status: x => ({ navn: x.v }),
  };
  /* "Reglerne, der slog til" for udsnittet, med samme kort som Angreb. Et klik kæder reglen på. Et regel-dyk får ingen kort: dér er reglen hovedet. */
  function reglerBlok(d, liste) {
    if (!d.regler.length) return [];
    const u = [...liste.map(f => filterNavn(f, d)), ...(har('dag') ? [V.datoLang(params.get('dag'))] : [])].join(' · ') || periodeTekst();
    const kort = d.regler.map(rg => {
      const navn = V.regelVis(rg).overskrift;
      return V.regelKort(rg, { dyk: medFilter('regel', rg.navn), n: rg.h30, enhed: T.dykKortN, label: fmt(T.dykAabn, { navn, n: tal(rg.h30) }), sti: x => kopi(p => { p.set('regel', rg.navn); p.set('sti', x); }), land: k => kopi(p => { p.set('regel', rg.navn); p.set('land', k); }) });
    });
    return [blok(fmt(T.dykReglerH, { u }), '', el('div', { className: 'vg-regler' }, ...kort))];
  }
  /* Top 5 fremme, resten bag "Vis top 20". Et filter er sat for dimensionen: listen står ikke. Reglerne står som kort (reglerBlok). */
  function lister(d) {
    const boks = el('div', { className: 'vg-tre dyk-lister' });
    for (const dim of erRobot() ? ['robot', 'sti', 'status'] : ['land', 'netvaerk', 'sti', 'colo', 'metode', 'handling']) {
      if ((har(dim) && !(dim === 'robot' && V.ROBOT_SLAGS.includes(params.get('robot')))) || (har('forsoeg') && dim !== 'sti')) continue;
      /* Svarkoden er ikke et filter: den tal fører ingen steder hen. */
      const maal = x => (dim === 'status' ? {} : { dyk: har('forsoeg') ? medForsoegSti(x.v) : dim === 'robot' ? medFilter('robot', robotFilter(x.v)) : medFilter(dim, x.v) });
      const rk = d.top[dim].map(x => ({ ...RAEKKER[dim](x), n: x.n, ...maal(x) }));
      if (!rk.length) continue;
      boks.append(el('div', {}, el('h3', { textContent: T.dykLister[dim] }), ...V.listeFlere(rk, TOP_SYNLIGE, dim, fmt(T.dykFlere, { n: tal(d.antal[dim]) }))));
    }
    return boks.childElementCount ? [blok(T.dykListerH, T.dykListerP, boks)] : [];
  }
  const medForsoegSti = sti => medFilter('forsoeg', sti);
  /* Et navn i listen over fx de udklædte: samme slags, nu kun den robot. */
  const robotFilter = navn => { const v = params.get('robot'); return !v || v === 'alle' ? navn : `${v}:${navn}`; };
  function andre(d, liste) {
    if (!d.domaener.length) return [];
    const h = har('regel') && liste.length === 1 ? T.dykAndreRegel : liste.length ? fmt(T.dykAndre, { navn: liste.map(f => filterNavn(f, d)).join(' · ') }) : T.dykAndreAlt;
    const rk = d.domaener.map(x => {
      const q = kopi(p => { p.delete('samlet'); p.set('zone', x.zone); p.set('navn', x.navn); });
      return { navn: x.navn, n: x.n, dyk: q, domaene: true, label: fmt(T.dykSkift, { navn: x.navn, n: tal(x.n) }) };
    });
    return [blok(h, '', el('div', { className: 'dyk-andre' }, V.liste(rk)))];
  }
  function enkelte(d) {
    if (!d.enkelte.length) return [];
    const kilde = new Map(d.top.regel.map(x => [x.v, x.art]));
    /* Domænet i det samlede dyk: samme filter på det domæne. */
    const domaeneCelle = navn => {
      const r = R.tilstand.raekker.find(x => x.navn === navn), td = el('td');
      if (!r) td.append(navn); else td.append(V.knap(navn, kopi(p => { p.delete('samlet'); p.set('zone', r.id); p.set('navn', r.navn); })));
      return td;
    };
    const til = (dim, v) => (dim === 'dag' ? (har('dag') ? null : medPeriode('dag', v)) : har(dim) ? null : medFilter(dim, v));
    const tabel = V.senesteTabel(d.enkelte, { til, domaene: domaeneCelle, uden: har('regel') ? ['regel'] : [], art: v => kilde.get(v) }), e = d.enkelte_info;
    if (!e) return [blok(T.dykSenesteH, T.dykSenesteP, tabel)];
    const tekst = [fmt(T.dykVisNafM, { n: tal(e.vist), m: tal(e.i_alt) }), ...(e.loft ? [T.dykLoft200] : [])].join(' ');
    const side = n => kopi(p => { if (n > 1) p.set('side', n); });
    const nav = el('nav', { className: 'dyk-sider' }, ...(e.side > 1 ? [knap(T.dykForrige, side(e.side - 1), 'knap')] : []), el('span', { textContent: fmt(T.dykSideN, { a: tal(e.side), b: tal(e.sider) }) }), ...(e.side < e.sider ? [knap(T.dykNaeste, side(e.side + 1), 'knap')] : []));
    nav.setAttribute('aria-label', T.dykSiderH);
    return [blok(T.dykSenesteH, tekst, tabel, ...(e.sider > 1 ? [nav] : []))];
  }

  /* "Dit sites teknologi" for ét domæne uden filtre: chips og knappen (hmTegn.tekKort). Pladsen fyldes bagefter: valgene er et lokalt opslag, og tallene skal ikke vente på dem. */
  function tekBlok(g) {
    if (samlet() || filtre().length || !params.get('zone')) return [];
    const plads = el('div'), zone = params.get('zone');
    const fyld = async () => {
      let t;
      try { t = await invoke('teknologi_hent', { zone }); } catch (e) { return; }
      if (g !== gen || !plads.isConnected) return;
      const saet = async (tek, valg) => { try { await invoke('teknologi_saet', { zone, tek, valg, besked: false }); await R.hentOverblik(); fyld(); } catch (e) { /* valget står */ } };
      plads.replaceChildren(V.blok(T.tekH, '', ...V.tekKort({ zone, navn: domaene, linjer: t.linjer, saet })));
    };
    queueMicrotask(fyld);
    return [plads];
  }
  /* "Hvad angrebene var rettet mod" for udsnittet. Står ikke, når udsnittet allerede er én teknologi. */
  const rettetBlok = d => (d.rettet_mod.length && !har('teknologi') && !erRobot() && !har('forsoeg') ? [V.blok(T.rettetH, T.rettetP, V.rettetMod(d.rettet_mod, t => medFilter('teknologi', t)))] : []);

  /* "Vurdering": for én dag på ét domæne. */
  const vurdering = () => (har('dag') && !samlet() && params.get('zone') ? [V.vurderingBoks({ zone: params.get('zone'), navn: domaene, dag: params.get('dag') })] : []);

  /* ── trafik-dyk ── */
  const pct = (a, b) => `${tal(Math.round(100 * a / b))}${S.lang === 'da' ? ' %' : '%'}`;
  /* Sætninger i klarsprog, kun når én ting fylder mere end halvdelen. Tallene siger, hvor trafikken kom fra, ikke hvad den var. */
  function trafikOrd(d) {
    const ud = [], [land] = d.lande, [kode] = d.status, [robot] = d.robotter, stor = (n, i) => i > 0 && 2 * n > i;
    if (land && stor(land.n, d.lande_i_alt)) ud.push(fmt(T.trafikEtLand, { p: pct(land.n, d.lande_i_alt), land: V.land(land.v) }));
    if (stor(d.server_fejl, d.status_i_alt)) ud.push(fmt(T.trafik52, { p: pct(d.server_fejl, d.status_i_alt) }));
    else if (kode && stor(kode.n, d.status_i_alt)) ud.push(fmt(T.trafikKode, { p: pct(kode.n, d.status_i_alt), k: kode.v }));
    if (robot && stor(robot.n, d.i_alt)) ud.push(fmt(T.trafikRobot, { r: V.robotNavn(robot.v) }));
    return ud;
  }
  /* Et land kan kun åbnes, hvis Cloudflare stoppede noget derfra den dag: forespørgslerne selv har vi kun som tal. Svarkoderne er rene tal. */
  function tegnTrafik(d) {
    const dag = V.interval(d.dag, d.til), uden = fn => kopi(p => { p.delete('trafik'); fn(p); });
    S.saetDyk(T.trafikMaerke);
    R.titel();
    $('dyk-h').textContent = fmt(T.dykTrafikH, { d: hvor(), v: dag });
    $('dyk-under').textContent = samlet() ? domaene : '';
    tilbage();
    $('dyk-filtre').hidden = true;
    $('dyk-periode').replaceChildren();
    const normalt = d.median === null ? '' : fmt(T.trafikNormalt, { m: tal(Math.round(d.median)) });
    /* Lande står, hvor kortet står i et almindeligt dyk: lige efter fliserne. */
    const dele = [el('div', { className: 'vg-fliser' }, flise(d.i_alt, T.trafikIAlt, normalt, '', true), ...(d.sikkerhed ? [flise(d.stoppet, T.trafikStoppet, '', uden(() => { }), false)] : [flise(d.trusler, T.trafikTrusler, '', '', true), flise(d.cachet, T.trafikCachet, '', '', true)]))];
    if (!d.sikkerhed) dele.push(el('p', { className: 'vg-note', textContent: T.trafikKunAar }));
    const ord = trafikOrd(d);
    if (ord.length) dele.push(el('ul', { className: 'vg-fordeling' }, ...ord.map(t => el('li', { textContent: t }))));
    if (!d.ekstra) dele.push(el('p', { className: 'vg-note', textContent: T.trafikIngenEkstra }));
    const kol = (h, rk) => (rk.length ? [el('div', {}, el('h3', { textContent: h }), V.liste(rk))] : []);
    const lande = kol(T.trafikLande, d.lande.map(x => ({ navn: V.land(x.v), n: x.n, ...(x.angreb ? { dyk: uden(p => p.set('land', x.v)) } : {}) })));
    if (lande.length) dele.push(el('div', { className: 'dyk-lister dyk-trafik-lande' }, ...lande));
    const boks = el('div', { className: 'vg-tre dyk-lister' },
      ...kol(T.trafikStatus, d.status.map(x => ({ navn: x.v, n: x.n }))),
      ...kol(T.trafikRobotter, d.robotter.map(x => ({ navn: V.robotNavn(x.v), n: x.n, dyk: uden(p => p.set('robot', x.v)) }))));
    if (boks.childElementCount) dele.push(V.blok(T.dykListerH, T.dykTrafikP, boks));
    else if (d.ekstra && !lande.length) dele.push(el('p', { className: 'vg-status dyk-tom', textContent: T.trafikTomt }));
    dele.push(el('details', { className: 'dyk-guide' }, el('summary', { textContent: T.gTrafikH }), ...V.bogAfsnit('trafik'), el('h4', { textContent: T.gDnsH }), ...V.bogAfsnit('dns')));
    dele.push(el('p', { className: 'vg-note', textContent: T.dykNote }));
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(...dele);
  }

  /* ── tegning ── */
  /* Det valg, der havde fokus, da siden skiftede: "Henter …" fjerner det, og det skal have fokus igen bagefter. */
  let valgtFokus = null;
  const fokusNu = () => (document.activeElement && document.activeElement.dataset ? document.activeElement.dataset.fokus : null);
  function tegn(d) {
    const liste = filtre(), navne = liste.map(f => filterNavn(f, d));
    S.saetDyk(navne.length ? navne.join(' · ') : fmt(T.dykAlle, { domaene }));
    R.titel();
    $('dyk-h').textContent = overskrift(d, liste);
    $('dyk-under').textContent = samlet() ? domaene : '';
    tilbage();
    chips(d, liste);
    const p = params.get('periode'), fast = !har('dag') && !har('fra');
    $('dyk-periode').replaceChildren(segment('dyk-periode', T.dykPeriode, [[T.dykP7, medPeriode('periode', '7'), fast && p === '7'], [T.dykP30, medPeriode(), fast && p !== '7' && p !== 'alt']]));
    const genaabn = V.husk(rod);
    const fokus = fokusNu() || valgtFokus;
    const vaelger = V.datoVaelger({ visning: 'dyk', periode: d.periode, nulstil: medPeriode('periode', 'alt'), gaa: (a, b) => R.gaaTil('dyk', kopi(q => { PERIODE.forEach(x => q.delete(x)); q.set('fra', a); q.set('til', b); })) });
    /* Samme opbygning som Angreb: fliser → fordeling → kort → søjler og regler → lister, og "De seneste" sidst. */
    const dele = [...kunIDag(d, liste), ...trafikLinje(d), vaelger, ...kunTrafik(d), ...hoved(d), ...(har('regel') || har('forsoeg') || erRobot() ? [] : [slagsVaelger(d)]), fliser(d), ...(kunTrafik(d).length ? [] : V.daekning(d.periode)), ...vurdering(), ...fordeling(d)];
    if (d.loft) dele.push(el('p', { className: 'vg-advarsel vg-gul', textContent: T.dykLoft }));
    if (d.sum) dele.push(...kortBlok(d), ...soejleBlok(d), ...reglerBlok(d, liste), ...tekBlok(gen), ...rettetBlok(d), ...lister(d), ...andre(d, liste), ...enkelte(d));
    else dele.push(el('p', { className: 'vg-status dyk-tom', textContent: T.dykTomt }), ...andre(d, liste));
    dele.push(el('p', { className: 'vg-note', textContent: T.dykNote }));
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(...dele);
    genaabn();
    rod.querySelectorAll('.vg-soejler-rul').forEach(r => { r.scrollLeft = r.scrollWidth; });
    const igen = fokus ? [...document.querySelectorAll('[data-fokus]')].find(x => x.dataset.fokus === fokus) : null;
    if (igen) igen.focus();
    valgtFokus = null;
  }
  function besked(tekst, farve = 'roed') {
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(el('p', { className: `vg-advarsel vg-${farve}`, textContent: tekst }));
    $('dyk-filtre').hidden = true;
    $('dyk-periode').replaceChildren();
  }

  /* ── start og stop ── */
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
    params = p;
    valgtFokus = fokusNu() || null;
    domaene = samlet() ? '…' : (p.get('navn') || '').slice(0, 253) || '…';
    S.saetDomaene(domaene, samlet());
    R.visVaelger();
    tilbage();
    rod.setAttribute('aria-busy', 'true');
    rod.replaceChildren(el('p', { className: 'vg-status', textContent: T.dykHenter }));
    if (!invoke) return besked(T.mangler);
    let zoner;
    try {
      await V.klar();
      zoner = samlet() ? await invoke('valg_hent') : null;
    } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
    if (g !== gen) return;
    if (samlet()) {
      if (!zoner.length) { besked(''); rod.replaceChildren(window.hmVelkomst.intetValgt()); return; }
      domaene = T.domaener(zoner.length);
      antal = zoner.length;
      S.saetDomaene(domaene, true);
      R.visVaelger();
    }
    const omfang = samlet() ? { zoner } : { zone: p.get('zone') || '' };
    async function hent() {
      let d;
      const trafik = erTrafik();
      try { d = await (trafik ? invoke('trafik_dag', { omfang, dag: p.get('dag') || p.get('fra'), til: p.get('til') }) : invoke('dyk', { omfang, filtre: filtre(), periode: periodeArg(), slags: p.get('slags'), side: Number(p.get('side')) || undefined })); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
      if (g === gen) (trafik ? tegnTrafik : tegn)(d);
    }
    await hent();
    if (g !== gen) return;
    /* Knappen først, så Tab går videre til chips og vælgere. */
    if (!R.genbesoeg() && !fokusNu()) $('dyk-tilbage').querySelector('button').focus();
    let sidst = Date.now(), ventende = null;
    const snart = () => {
      if (ventende) return;
      ventende = setTimeout(() => { ventende = null; sidst = Date.now(); hent(); }, Math.max(0, sidst + 3000 - Date.now()));
    };
    afmeld.push(R.paa('hentekoe', k => { k = k || {}; if (!k.zone || (samlet() ? zoner.includes(k.zone) : k.zone === p.get('zone'))) snart(); }), () => clearTimeout(ventende));
  }
  R.registrer('dyk', {
    domaener: true,
    hvad: () => ({ samlet: samlet(), zone: params.get('zone') || '', navn: domaene }),
    /* Et andet domæne: samme filtre og periode. */
    skift(q) {
      const ny = new URLSearchParams(q);
      for (const [k, v] of params) if (!['zone', 'navn', 'samlet'].includes(k)) ny.append(k, v);
      R.gaaTil('dyk', ny);
    },
    vis(p, { samme } = {}) {
      if (samme && aktiv === p.toString()) return;
      start(p);
    },
    skjul: stop,
  });
})();
