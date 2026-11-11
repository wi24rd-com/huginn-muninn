/* Huginn and Muninn — Angreb: hvad Cloudflares edge holdt ude fra dine zoner, på én side pr. domæne.
   #vaernet?zone=<id>&navn=… eller #vaernet?samlet=1, plus filtrene regel, land, netvaerk, sti, colo, metode, handling, forsoeg, robot,
   robotgruppe og teknologi (højst ét pr. dimension; rækkefølgen i adressen er rækkefølgen i chippene; robotter kan kun kombineres med sti),
   slags (angreb|rutine, ellers alt) og perioden: ingen eller periode=7 er 7 hele dage, periode=30, periode=alt, dag=… eller fra=…&til=….
   side=N er siden i "De seneste". trafik=1 med dag=… eller fra=…&til=… lægger afsnittet "Trafik den …" øverst (trafik_dag). vis=teknologi og vis=seneste ruller dertil første gang.
   Et klik på et tal, et land, en regel eller en sti forudindstiller perioden og filtrene på samme side, og hvert skift er ét trin i tilbage/frem.
   Sidens egne links har altid perioden med, for rammen læser data-dyk uden periode som 30 dage (fraDyk). Gamle #dyk?…-adresser lander her.
   Alle tal, der følger perioden, regnes i Rust (dyk, kun fra cachen). vaern_* giver statuslinjen og "I går" i fordelingen, og vaern_oversigt henter nye tal fra Cloudflare,
   men kun på siden uden parametre: et filter læser kun det gemte. Det samlede henter ikke selv; køen gør, og siden tegnes igen, når den melder nyt, højst hvert 3. sekund.
   Afsnit, der ikke kan følge et filter, står ikke og nævnes i én linje (skjultLinje), så intet står tomt. Kort, søjler, lister og regelkort står i vg-tegn.js.
   Ingen IP-adresser; stierne er renset i Rust og sættes altid som tekst. Bygges med createElement. Dage i uhentet er huller: de tegnes ikke som 0.
   start(params)/stop(): et skift stopper alt fra det forrige (lyttere, ure, genforsøg), og svar, der kommer for sent, kasseres (gen). */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, L = T.ang, R = window.hmRamme, V = window.hmTegn;
  const $ = id => document.getElementById(id);
  const { el, fmt, invoke } = R, tal = S.tal, kl = S.kl;
  const FILTRE = ['regel', 'land', 'netvaerk', 'sti', 'colo', 'metode', 'handling', 'forsoeg', 'robot', 'robotgruppe', 'teknologi'];
  const PERIODE = ['periode', 'dag', 'fra', 'til'];
  const TOP_SYNLIGE = 5;
  const rod = $('vagt');

  /* ── adressen ── */
  let params = new URLSearchParams(), domaene = '…', omfang = null, vaern = null, stat = { tekst: '', henter: false, advarsel: null };
  const samlet = () => params.get('samlet') === '1';
  const zone = () => params.get('zone') || '';
  const filtre = () => [...params.keys()].filter(k => FILTRE.includes(k)).map(dim => ({ dim, vaerdi: params.get(dim) }));
  const har = dim => params.has(dim);
  const erRobot = () => har('robot') || har('robotgruppe');
  const erTrafik = () => har('trafik') && (har('dag') || (har('fra') && har('til')));
  /* Siden uden parametre er den eneste, der henter fra Cloudflare. */
  const ren = () => ![...params.keys()].some(k => !['zone', 'navn', 'samlet', 'vis'].includes(k));
  function periodeArg() {
    if (har('dag')) return { dag: params.get('dag') };
    if (har('fra') && har('til')) return { spaend: { fra: params.get('fra'), til: params.get('til') } };
    return { 30: 'd30', alt: 'alt' }[params.get('periode')] || 'd7';
  }
  /* En ny adresse starter på første side i "De seneste" og har altid perioden med. */
  const kopi = fn => { const p = new URLSearchParams(params); p.delete('side'); p.delete('vis'); fn(p); if (!PERIODE.some(k => p.has(k))) p.set('periode', '7'); return p.toString(); };
  const medFilter = (dim, v) => kopi(p => p.set(dim, v));
  const udenFilter = dim => kopi(p => p.delete(dim));
  const medPeriode = (k, v) => kopi(p => { PERIODE.forEach(x => p.delete(x)); if (k) p.set(k, v); });
  const medSlags = v => kopi(p => { if (v) p.set('slags', v); else p.delete('slags'); });
  /* Til adresser, der ikke går gennem data-dyk: standardperioden står ikke i adressen. */
  const udenStandard = q => { const p = new URLSearchParams(q); if (p.get('periode') === '7') p.delete('periode'); return p.toString(); };
  const omfangQ = () => new URLSearchParams(samlet() ? { samlet: '1' } : { zone: zone(), navn: domaene }).toString();

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
  const periodeTekst = () => (har('dag') ? V.datoLang(params.get('dag')) : har('fra') ? V.interval(params.get('fra'), params.get('til')) : params.get('periode') === '30' ? T.dykP30 : params.get('periode') === 'alt' ? T.dykPAlt : T.dykP7);
  /* Kun angreb tæller som statusreglen, også det, der ikke blev afvist (fx log fra egne regler). */
  const ord = () => (erRobot() ? (/^(stoppet|labyrint)/.test(params.get('robot') || '') ? T.dykHaendelser : T.dykBesoeg) : ['faelder', 'andet'].includes(params.get('handling')) || har('forsoeg') ? T.dykHaendelser : params.get('slags') === 'angreb' ? T.dykAngrebOrd : T.dykAfvist);

  /* ── byggeklodser ── */
  const knap = (tekst, query, klasse, label) => V.knap(tekst, query, klasse || 'link', label);
  const segment = (navn, etiket, valg) => V.segment(navn, etiket, valg.map(([t, q, valgt]) => [t, udenStandard(q), valgt]), 'vaernet');
  const flise = (n, tekst, sub, query, aktiv) => V.flise(n, tekst, sub, query, { aktiv });
  /* Et afsnit med fast id, så det kan nås med data-rul og fra tests. */
  function afsnit(id, h, p, ...indhold) {
    const s = V.blok(h, p, ...indhold), h2 = s.querySelector('h2');
    s.id = id;
    h2.id = `${id}-h`;
    s.setAttribute('aria-labelledby', h2.id);
    return s;
  }

  /* ── overskrift, chips og vælgere ── */
  function visOverskrift() {
    $('vaernet-h').textContent = fmt(erRobot() ? T.titel.robotter : T.titel.vaernet, { domaene });
  }
  /* Tilbage til oversigten: robotterne har deres egen. */
  const oversigt = q => (erRobot() ? `robotter?${omfangQ()}` : `vaernet?${udenStandard(q)}`);
  /* Ét filter pr. dimension. × fjerner kun det; er det det sidste, er man tilbage på siden uden filtre. Nulstil fjerner alt. */
  function chips(d, liste) {
    const led = liste.map(f => ({ navn: filterNavn(f, d), fjern: udenFilter(f.dim) }));
    if (erTrafik()) led.push({ navn: L.trafikChip, fjern: kopi(p => p.delete('trafik')) });
    if (har('dag')) led.push({ navn: V.datoLang(params.get('dag')), fjern: kopi(p => { PERIODE.forEach(x => p.delete(x)); p.delete('trafik'); }) });
    if (har('fra') && har('til')) led.push({ navn: V.interval(params.get('fra'), params.get('til')), fjern: kopi(p => { PERIODE.forEach(x => p.delete(x)); p.delete('trafik'); }) });
    const ul = el('ul');
    for (const l of led) {
      const indhold = el('span', { className: 'dyk-filter-tekst' }, el('span', { textContent: l.navn }), el('span', { className: 'dyk-filter-x', textContent: '×' }));
      ul.append(el('li', {}, V.knap(indhold, led.length === 1 ? { gaa: oversigt(l.fjern) } : l.fjern, 'dyk-filter', fmt(T.dykFjern, { navn: l.navn }))));
    }
    const nav = $('dyk-filtre'), noget = led.length || har('slags');
    nav.setAttribute('aria-label', T.dykFiltre);
    nav.hidden = !noget;
    nav.replaceChildren(ul, ...(noget ? [V.knap(L.nulstil, { gaa: `vaernet?${omfangQ()}` }, 'link dyk-nulstil', L.nulstilLabel)] : []));
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

  /* ── fliser ── */
  function fliser(d) {
    const k = d.noegletal, o = ord(), p = params.get('periode'), iDag = fmt(T.dykFIDag, { n: tal(k.i_dag_hidtil) }), liste = filtre();
    const boks = el('div', { className: 'vg-fliser' }), med = (m, n, ellers) => (m < n ? fmt(T.dagMed, { m: tal(m), n: tal(n) }) : ellers);
    if (har('dag')) boks.append(flise(d.sum, fmt(T.dykFDag, { o, d: V.datoLang(params.get('dag')) }), '', '', true));
    if (har('fra')) { const [a, b] = S.datoPar(params.get('fra'), params.get('til')); boks.append(flise(d.sum, fmt(T.dykFPeriode, { o, a, b }), med(d.periode.med, d.periode.dage, ''), '', true)); }
    /* Tal kun i dag: flisen siger begge dele, så den ikke modsiger "Først set". */
    const altSub = d.kun_i_dag ? fmt(T.dykHeleIDag, { h: tal(k.alt), n: tal(k.i_dag_hidtil) }) : med(d.periode.med, d.periode.dage, '');
    if (p === 'alt') boks.append(flise(k.alt, fmt(T.dykFAlt, { o }), altSub, medPeriode('periode', 'alt'), true));
    const fast = !har('dag') && !har('fra') && p !== 'alt', andel = !liste.length && vaern && k.hele7 && vaern.alle_hele7 ? fmt(L.andel, { n: tal(Math.round(vaern.alle_hele7 / k.hele7)) }) : '';
    boks.append(V.flise(k.i_gaar, fmt(T.dykFIGaar, { o }), iDag, medPeriode('dag', d.i_gaar), { ekstra: { n: k.i_dag_hidtil, dyk: medPeriode('dag', d.i_dag) } }));
    boks.append(flise(k.hele7, fmt(T.dykF7, { o }), [andel, med(k.med7, 7, '')].filter(Boolean).join(' · '), medPeriode('periode', '7'), fast && p !== '30'), flise(k.hele30, fmt(T.dykF30, { o }), med(k.med30, 30, T.dykF30Sub), medPeriode('periode', '30'), fast && p === '30'));
    const dato = (h, dag) => el('div', { className: 'dyk-dato' }, el('span', { textContent: h }), knap(V.datoLang(dag), medPeriode('dag', dag), 'link', fmt(T.dykAabn, { navn: h, n: V.datoLang(dag) })));
    if (d.foerst) boks.append(el('div', { className: 'vg-flise dyk-datoer' }, dato(T.dykFoerst, d.foerst), dato(T.dykSidst, d.sidst)));
    boks.append(...forespoergsler(d));
    if (visForsoeg(d)) { const navn = fmt(L.forsoeg, { p: periodeTekst() }); boks.append(V.flise(d.forsoeg.n, navn, L.forsoegSub, { rul: 'vg-forsoeg' }, { klasse: 'vg-graa', label: fmt(T.dykRul, { navn, n: tal(d.forsoeg.n) }) })); }
    return boks;
  }
  /* Adressen til afsnittet "Trafik den …" for en periode. */
  const trafikQ = (a, b) => kopi(p => { PERIODE.forEach(x => p.delete(x)); FILTRE.forEach(x => p.delete(x)); p.delete('slags'); if (a === b) p.set('dag', a); else { p.set('fra', a); p.set('til', b); } p.set('trafik', '1'); });
  /* Forholdet mellem det, der står her, og alle forespørgsler. Kun uden filtre: et filter gælder ikke forespørgslerne. Robotter tæller besøg, og forsøg er for sig. */
  function forespoergsler(d) {
    const antal = erRobot() ? d.sum : d.slags.angreb, rene = erRobot() ? !har('sti') : !filtre().length;
    if (!d.trafik.i_alt || !rene) return [];
    const sub = antal ? fmt(T.dykForespSub, { o: erRobot() ? T.dykBesoeg : T.dykAngrebOrd, n: tal(antal), m: tal(Math.round(d.trafik.i_alt / antal)) }) : '';
    return [V.flise(d.trafik.i_alt, T.dykForesp, sub, trafikQ(d.periode.fra, d.periode.til), { label: fmt(T.dykForespAabn, { n: tal(d.trafik.i_alt) }) })];
  }

  /* ── linjerne øverst ── */
  /* En periode med usædvanlig meget trafik siger det øverst, for angrebene alene ser små ud. Flere dage får en samlet linje; hver dag åbner sin trafik. Tallene kan ikke sige, hvad det var. */
  function trafikLinjer(d) {
    const dage = d.trafik.dage;
    if (!dage.length || filtre().length || erTrafik()) return [];
    const ud = [];
    if (dage.length > 1) {
      const a = dage[0].dag, b = dage[dage.length - 1].dag, sum = k => dage.reduce((x, t) => x + t[k], 0), n = tal(sum('n'));
      const tekst = fmt(T.dykTrafikLinje, { t: T.trafikMaerke, p: V.interval(a, b), n, m: tal(Math.round(dage[0].median)), stop: S.trafikStop(sum('n'), sum('stoppet')) });
      ud.push(el('p', { className: 'vg-advarsel vg-gul dyk-trafik-linje' }, tekst, ' ', knap(T.dykSeTrafikken, trafikQ(a, b), 'link', T.dykSeTrafikken)));
    }
    ud.push(el('ul', { className: 'vg-trafik' }, ...dage.map(t => el('li', {}, V.knap(fmt(2 * t.stoppet >= t.n ? T.trafikDagMest : T.trafikDag, { d: V.datoLang(t.dag), n: tal(t.n), s: tal(t.stoppet), o: tal(t.n - t.stoppet) }), trafikQ(t.dag, t.dag), 'trafik-maerke'))), el('li', { className: 'vg-trafik-note', textContent: fmt(T.trafikNote, { m: tal(Math.round(dage[0].median)) }) })));
    return ud;
  }
  /* Et filter, der kun har tal i dag, må ikke ligne 0. */
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
  /* Sikkerhedstal over en måned gamle findes ikke hos Cloudflare: det skal stå tydeligt, når perioden når længere tilbage. */
  function kunTrafik(d) {
    const fra = d.periode.sikkerhed_fra;
    return fra && d.periode.fra < fra ? [el('p', { className: 'vg-advarsel vg-gul dyk-kun-trafik', textContent: fmt(T.dykKunTrafik, { d: V.datoLang(fra) }) })] : [];
  }
  /* Mærket "Ligner …" øverst med forklaringen: klik åbner dagen, og linket guiden. */
  function tegnMoenster() {
    rod.querySelector('.moenster-linje')?.remove();
    rod.querySelector('.moenster-note')?.remove();
    if (R.tilstand.raekker.some(x => x.kan_sammenligne === false)) rod.prepend(el('p', { className: 'vg-note moenster-note', textContent: T.maalMin }));
    const r = samlet() ? null : R.tilstand.raekker.find(x => x.id === zone()), b = r && V.moensterKnap(r);
    if (!b) return;
    const guide = el('button', { type: 'button', className: 'link', textContent: T.maerkeGuide });
    guide.addEventListener('click', () => R.guide(r.id));
    rod.prepend(el('p', { className: `moenster-linje moenster-linje-${r.moenster.slags}` }, b, ' ', S.moensterTekst(r.moenster), ' ', guide));
  }

  /* ── fordelingen ── */
  /* "heraf N blokeret · N udfordret · N lokket ind i labyrinten" for én periode; hver del lægger handlingen på som filter. Tallet 0 fører ingen steder hen. */
  const FORD = { b: 'blokeret', u: 'udfordret', l: 'labyrint' };
  function fordelingLinjer(navn, f, maal) {
    if (!f) return [];
    const li = el('li', {}, el('b', { textContent: `${navn}:` }), ' ');
    L.ford.split(' · ').forEach((del, i) => {
      const [, foer, k] = /^(.*?)\{(\w)\}/.exec(del), tekst = fmt(del.slice(foer.length), { [k]: tal(f[FORD[k]]) });
      li.append(...(i ? [' · '] : []), foer, f[FORD[k]] ? knap(tekst, maal(FORD[k]), 'link dyk-celle') : tekst);
    });
    if (f.andet) li.append(' · ', knap(fmt(L.fordAndet, { a: tal(f.andet) }).replace(/^ · /, ''), maal('andet'), 'link dyk-celle'));
    const ud = [li];
    if (f.faelder) ud.push(el('li', { className: 'vg-faelder' }, el('b', { textContent: `${navn}:` }), ' ', knap(T.faelderLinje(tal(f.faelder)), maal('faelder'), 'link dyk-celle')));
    return ud;
  }
  function fordeling(d) {
    if (har('handling') || har('forsoeg') || erRobot()) return [];
    const iGaar = !filtre().length && !har('slags') && !har('dag') && vaern && vaern.fordeling ? fordelingLinjer(L.fordI, vaern.fordeling.i_gaar, h => kopi(p => { PERIODE.forEach(x => p.delete(x)); p.set('dag', d.i_gaar); p.set('handling', h); })) : [];
    const linjer = [...iGaar, ...fordelingLinjer(periodeTekst(), d.fordeling, h => medFilter('handling', h))];
    return linjer.length ? [el('ul', { className: 'vg-fordeling' }, ...linjer), el('p', { className: 'vg-note', textContent: L.fordNote })] : [];
  }

  /* ── kortet ── */
  function kortBlok(d) {
    const v = { lande: d.top.land.map(x => ({ kode: x.v, n: x.n })), colos: d.top.colo.map(x => ({ kode: x.v, n: x.n })), buer: d.buer };
    if (har('forsoeg') || !v.lande.length) return [];
    const opt = {
      titel: `${fmt(L.kortTitel, { p: periodeTekst() })}. ${T.tastatur}`, id: 'vg-kort-d', ...(filtre().length || har('slags') ? { tal: n => tal(n) } : {}), beskrivelse: fmt(L.kortDesc, { lande: v.lande.slice(0, 3).map(l => V.land(l.kode)).join(', '), colos: v.colos.slice(0, 3).map(c => V.colo(c.kode)).join(', ') }),
      dyk: { land: k => medFilter('land', k), colo: k => medFilter('colo', k), bue: (l, c) => kopi(p => { p.set('land', l); p.set('colo', c); }) },
    };
    const leg = (klasse, tekst) => el('li', {}, el('i', { className: klasse }), tekst);
    const legende = el('ul', { className: 'vg-legende' }, leg('vg-leg-land', L.leg.land), leg('vg-leg-colo', L.leg.colo), leg('vg-leg-bue', L.leg.bue), el('li', { textContent: L.leg.samme }));
    const uden = V.udenPlads(v), flereLande = d.antal.land - v.lande.length, flereColos = d.antal.colo - v.colos.length, skala = V.kortSkala(v.lande);
    if (skala) legende.prepend(skala);
    if (uden.length) legende.append(el('li', { textContent: fmt(T.kortUdenPlads, { liste: uden.join(', ') }) }));
    if (flereLande > 0) legende.append(el('li', { textContent: fmt(T.kortFaaLande, { n: tal(flereLande) }) }));
    if (flereColos > 0) legende.append(el('li', { textContent: fmt(T.kortFaaColos, { n: tal(flereColos) }) }));
    if (d.buer_i_alt > d.buer.length) legende.append(el('li', { textContent: fmt(T.kortBuer, { n: tal(d.buer.length), m: tal(d.buer_i_alt) }) }));
    const ramme = el('div', { className: 'vg-kortramme' }, V.kort(v, 'verden', opt));
    ramme.setAttribute('role', 'region');
    ramme.setAttribute('aria-label', fmt(L.kortTitel, { p: periodeTekst() }));
    const midt = () => { if (ramme.scrollWidth > ramme.clientWidth) ramme.scrollLeft = (ramme.scrollWidth - ramme.clientWidth) * 0.5; };
    const zoom = el('div', { className: 'vg-zoom' });
    zoom.setAttribute('role', 'group');
    zoom.setAttribute('aria-label', L.kortH);
    for (const [navn, tekst] of [['verden', L.verden], ['europa', L.europa]]) {
      const b = el('button', { type: 'button', textContent: tekst });
      b.dataset.vis = navn;
      b.setAttribute('aria-pressed', String(navn === 'verden'));
      b.addEventListener('click', () => {
        zoom.querySelectorAll('button').forEach(x => x.setAttribute('aria-pressed', String(x === b)));
        ramme.replaceChildren(V.kort(v, navn, opt));
        midt();
      });
      zoom.append(b);
    }
    queueMicrotask(midt);
    return [afsnit('vg-kort', L.kortH, L.kortP, el('div', { className: 'vg-tavle' }, zoom, ramme, legende))];
  }

  /* ── søjlerne og tabellen ── */
  function dageTop(dage) {
    const sumD = dage.reduce((s, d) => s + d.n, 0), mx = dage.reduce((a, d) => (d.n > a.n ? d : a), dage[0] || { n: 0 });
    const b = el('b', { textContent: tal(sumD) }), ud = [b, ' ', L.ialt];
    if (mx && mx.n) ud.push(' · ', knap(fmt(L.maks, { n: tal(mx.n), d: V.datoLang(mx.dag) }), medPeriode('dag', mx.dag), 'link dyk-celle'));
    return ud;
  }
  function dageTabel(dage) {
    const kol = (tekst, klasse) => { const th = el('th', { textContent: tekst, className: klasse || '' }); th.setAttribute('scope', 'col'); return th; };
    const raekker = dage.slice().reverse().map(d => {
      if (d.hentet === false) return el('tr', { className: 'hul-raekke' }, el('th', { scope: 'row', textContent: V.datoLang(d.dag) }), el('td', { className: 'tal-kol', textContent: T.hulIngen }), el('td', { className: 'tal-kol', textContent: T.hulIngen }));
      const dag = V.datoLang(d.dag), q = medPeriode('dag', d.dag), label = fmt(T.dykDag, { d: dag, n: tal(d.n) });
      return el('tr', {}, el('th', { scope: 'row' }, d.n ? knap(dag, q, 'link dyk-celle', label) : dag), el('td', { className: 'tal-kol' }, d.n ? knap(tal(d.n), q, 'link dyk-celle', label) : tal(d.n)), el('td', { className: 'tal-kol', textContent: tal(d.alle) }));
    });
    const rul = el('div', { className: 'vg-tabel-rul', tabIndex: 0 }, el('table', { className: 'vg-tabel' }, el('thead', {}, el('tr', {}, kol(L.dag), kol(ord() === T.dykAfvist ? L.antal : ord(), 'tal-kol'), kol(L.trafik, 'tal-kol'))), el('tbody', {}, ...raekker)));
    rul.setAttribute('role', 'region');
    rul.setAttribute('aria-label', L.vis);
    return rul;
  }
  /* Går perioden længere tilbage end sikkerhedshændelserne, står trafikken for hele perioden, og angrebene kun, hvor de findes. Én dag har ingen søjler. */
  function soejleBlok(d) {
    if (har('dag')) return [];
    const fra = d.periode.sikkerhed_fra, sikre = d.trafik_serie.length ? d.dage.filter(x => x.dag >= fra) : d.dage, h = fmt(erRobot() ? T.dykRobotDage : T.dykEdge, { p: periodeTekst() });
    const rul = el('div', { className: 'vg-soejler-rul' }, V.soejler(sikre, { titel: `${fmt(L.dageTitel, { p: periodeTekst() })}. ${T.tastatur}`, dyk: dag => medPeriode('dag', dag) }));
    rul.setAttribute('role', 'region');
    rul.setAttribute('aria-label', fmt(L.dageTitel, { p: periodeTekst() }));
    const top = el('p', { className: 'vg-tavle-top' }, ...dageTop(sikre)), tavle = el('div', { className: 'vg-tavle vg-tavle-pad' }, top, rul);
    const tabel = el('details', { className: 'vg-tal' }, el('summary', { textContent: L.vis }), dageTabel(sikre));
    tabel.dataset.noegle = 'dage';
    const trafik = d.trafik_serie.length ? [V.blok(T.dykTrafikDage, T.dykTrafikDageP, el('div', { className: 'vg-tavle vg-tavle-pad' }, el('div', { className: 'vg-soejler-rul' }, V.soejler(d.trafik_serie, { titel: T.dykTrafikSoejle, hul: T.dykTrafikHul, dyk: dag => trafikQ(dag, dag) }))))] : [];
    return [...trafik, afsnit('vg-dage', h, filtre().length ? T.dykEdgeP : L.dageP, tavle, tabel)];
  }

  /* ── lister, regler og de andre afsnit ── */
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
  /* "Reglerne, der slog til": et klik lægger reglen på som filter. Med et regelfilter er reglen hovedet og får intet kort. */
  function reglerBlok(d, liste) {
    /* En regel, der kun er set i dag, er ikke i de hele dage: uden filter og valgt dag står den alligevel og åbner dagen i dag. */
    const nye = !liste.length && !har('dag') && !har('fra') && !har('slags') && vaern && vaern.h_regler ? vaern.h_regler.filter(rg => !rg.h30 && rg.idag > 0) : [];
    const regler = d.sum ? d.regler : [];
    if (!regler.length && !nye.length) return [];
    const u = [...liste.map(f => filterNavn(f, d)), ...(har('dag') ? [V.datoLang(params.get('dag'))] : [])].join(' · ') || periodeTekst();
    const kort = regler.map(rg => {
      const navn = V.regelVis(rg).overskrift, labyrint = rg.navn === 'linkMaze';
      return V.regelKort(rg, { dyk: medFilter('regel', rg.navn), n: rg.h30, enhed: labyrint ? T.lokketInd : T.dykKortN, label: fmt(T.dykAabn, { navn, n: tal(rg.h30) }), ...(labyrint ? { under: fmt(T.faelderSat, { n: tal(rg.faelder30) }) } : {}), sti: x => kopi(p => { p.set('regel', rg.navn); p.set('sti', x); }), land: k => kopi(p => { p.set('regel', rg.navn); p.set('land', k); }) });
    });
    for (const rg of nye) {
      const omDagen = fn => kopi(p => { PERIODE.forEach(x => p.delete(x)); p.set('regel', rg.navn); p.set('dag', d.i_dag); fn(p); });
      kort.push(V.regelKort(rg, { dyk: omDagen(() => { }), n: rg.h30, enhed: T.dykKortN, label: fmt(T.dykAabnRegelIDag, { navn: V.regelVis(rg).overskrift, n: tal(rg.idag) }), under: fmt(L.d1Sub, { n: tal(rg.idag) }), sti: x => omDagen(p => p.set('sti', x)), land: k => omDagen(p => p.set('land', k)) }));
    }
    return [afsnit('vg-regler', fmt(T.dykReglerH, { u }), fmt(L.reglerP, { domaene }), el('div', { className: 'vg-regler' }, ...kort), el('p', { className: 'vg-note', textContent: L.reglerNote }))];
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
    return boks.childElementCount ? [V.blok(T.dykListerH, T.dykListerP, boks)] : [];
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
    return [V.blok(h, '', el('div', { className: 'dyk-andre' }, V.liste(rk)))];
  }
  function enkelte(d) {
    if (!d.enkelte.length) return [];
    const kilde = new Map(d.top.regel.map(x => [x.v, x.art]));
    /* Domænet i det samlede: samme filter på det domæne. */
    const domaeneCelle = navn => {
      const r = R.tilstand.raekker.find(x => x.navn === navn), td = el('td');
      if (!r) td.append(navn); else td.append(V.knap(navn, ren() ? { gaa: 'vaernet?' + new URLSearchParams({ zone: r.id, navn: r.navn }) } : kopi(p => { p.delete('samlet'); p.set('zone', r.id); p.set('navn', r.navn); }), 'link dyk-celle', fmt(T.aabnFor, { navn })));
      return td;
    };
    const til = (dim, v) => (dim === 'dag' ? (har('dag') ? null : medPeriode('dag', v)) : har(dim) ? null : medFilter(dim, v));
    const tabel = V.senesteTabel(d.enkelte, { til, domaene: domaeneCelle, cve: true, uden: har('regel') ? ['regel'] : [], art: v => kilde.get(v) }), e = d.enkelte_info;
    if (!e) return [afsnit('vg-seneste', T.dykSenesteH, T.dykSenesteP, tabel)];
    const tekst = [fmt(T.dykVisNafM, { n: tal(e.vist), m: tal(e.i_alt) }), ...(e.loft ? [T.dykLoft200] : [])].join(' ');
    /* Forrige/Næste henter kun den nye side og bytter afsnittet ud på stedet (skiftSide): ingen ny tegning, rullepositionen bliver, og adressen
       erstattes, så tilbage/frem ikke får et trin pr. side (V-BESOEG.2 FIX 1). */
    const sideKnap = (tekst, n, hvilken) => {
      const b = el('button', { type: 'button', className: 'btn dyk-side-knap', textContent: tekst });
      b.dataset.side = hvilken;
      b.setAttribute('aria-label', fmt(T.dykSideAabn, { a: tal(n), b: tal(e.sider) }));
      b.addEventListener('click', () => skiftSide(n, hvilken));
      return b;
    };
    const nr = el('span', { className: 'dyk-side-nr', textContent: fmt(T.dykSideN, { a: tal(e.side), b: tal(e.sider) }), tabIndex: -1 });
    const nav = el('nav', { className: 'dyk-sider' }, ...(e.side > 1 ? [sideKnap(T.dykForrige, e.side - 1, 'forrige')] : []), nr, ...(e.side < e.sider ? [sideKnap(T.dykNaeste, e.side + 1, 'naeste')] : []));
    nav.setAttribute('aria-label', T.dykSiderH);
    return [afsnit('vg-seneste', T.dykSenesteH, tekst, tabel, ...(e.sider > 1 ? [nav] : []))];
  }
  /* "Hvad de ledte efter": kun uden filtre, for et 404-svar har hverken land, regel eller slags. En kategori og en sti lægger forsøget på som filter. */
  const visForsoeg = d => Boolean(d.forsoeg && d.forsoeg.kategorier.length) && !filtre().length && !har('slags');
  function forsoegPunkt(k, i, m) {
    const [navn, forkl] = T.kat[k.id] || [k.id, ''];
    const li = el('li', {}, el('div', { className: 'vg-p-hoved' }, el('span', { className: 'vg-p-nr', textContent: `§ ${i + 1}` }), el('h3', {}, knap(navn, medFilter('forsoeg', k.id), 'link dyk-celle', fmt(T.dykAabn, { navn, n: tal(k.n) }))), el('b', { textContent: tal(k.n) })),
      el('i', { className: V.bredde(100 * k.n / m) }), el('p', { textContent: forkl }));
    if (k.stier && k.stier.length) li.append(el('p', { className: 'vg-p-stier' }, ...k.stier.slice(0, 5).flatMap(s => [knap(V.sti(s.sti), medFilter('forsoeg', s.sti), 'link dyk-celle', fmt(T.dykAabn, { navn: s.sti, n: tal(s.n) })), ' '])));
    return li;
  }
  function forsoegBlok(d) {
    if (!visForsoeg(d)) return [];
    const fk = d.forsoeg.kategorier, fm = Math.max(1, ...fk.map(k => k.n));
    return [afsnit('vg-forsoeg', L.forsoegH, fmt(samlet() ? L.forsoegPP : L.forsoegP, { domaene }), el('ol', { className: 'vg-protokol' }, ...fk.map((k, i) => forsoegPunkt(k, i, fm))))];
  }

  /* "Dit sites teknologi" for ét domæne uden filtre: valgene og forslaget (hmTegn.tekKort), og de generiske slags, som ikke har et valg. Valgene sættes på #teknologi og er ikke tal for perioden. Pladsen fyldes bagefter: tallene skal ikke vente. */
  function tekBlok(g) {
    let plads = el('div');
    const z = zone(), foerst = plads;
    const fyld = async () => {
      let t;
      try { t = await invoke('teknologi_hent', { zone: z }); } catch (e) { return; }
      if (g !== gen || !plads.isConnected) return;
      const saet = async (tek, valg) => { try { await invoke('teknologi_saet', { zone: z, tek, valg, besked: false }); await R.hentOverblik(); fyld(); } catch (e) { /* valget står */ } };
      const dele = V.tekKort({ zone: z, navn: domaene, linjer: t.linjer, saet });
      if (t.generiske.length) {
        const liste = el('ul', { className: 'tek-generiske' }, ...t.generiske.map(x => { const n = (T.kat[x.id] || [x.id])[0]; return el('li', {}, V.knap(`${n}: ${fmt(T.tekForsoeg, { n: tal(x.forsoeg30) })}`, kopi(p => { PERIODE.forEach(k => p.delete(k)); p.set('periode', '30'); p.set('forsoeg', x.id); }))); }));
        dele.push(el('h3', { textContent: T.tekGeneriskH }), el('p', { className: 'vg-note', textContent: T.tekGeneriskP }), liste);
      }
      const ny = afsnit('vg-teknologi', T.tekH, '', ...dele);
      plads.replaceWith(ny);
      plads = ny;
      if (params.get('vis') === 'teknologi' && !stat.henter) {
        /* Tallene fra cachen tegnes, mens de friske er på vej. Først de friske er sidste tegning. */
        params.delete('vis');
        const s = $('vg-teknologi');
        if (s && s.scrollIntoView) s.scrollIntoView({ block: 'start' });
      }
    };
    queueMicrotask(fyld);
    return [foerst];
  }
  /* "Hvad angrebene var rettet mod". Står ikke, når der allerede er filtreret på én teknologi. */
  const rettetBlok = d => (d.rettet_mod.length && !har('teknologi') && !erRobot() && !har('forsoeg') ? [V.blok(T.rettetH, T.rettetP, V.rettetMod(d.rettet_mod, t => kopi(p => { p.set('teknologi', t); p.set('slags', 'angreb'); })))] : []);
  /* "Mails sendt i dit domænes navn", kun fra cachen og for perioden. Fejler opslaget, står afsnittet ikke. Rækkerne har ikke mere bag sig, så intet her er klikbart ud over guiden. */
  function emailIndhold(m) {
    const fod = [el('p', { className: 'vg-note', textContent: T.emailNote })], intro = fmt(T.emailP, { p: periodeTekst() });
    const guide = () => { const b = S.bog('dmarc_til'); return el('details', { className: 'dyk-guide' }, el('summary', { textContent: T.emailGuide }), el('p', { textContent: b.alle }), el('ol', {}, ...b.ja.map(t => el('li', { textContent: t })))); };
    if (!m.mails) {
      const linje = m.fejl ? fmt(T.emailFejl, { f: m.fejl }) : m.hentede_dage ? T.emailTom : T.emailVenter;
      return V.blok(T.emailH, '', el('p', { className: 'email-tom', textContent: linje }), ...(m.hentede_dage && !m.fejl ? [guide()] : []));
    }
    const top = el('p', { className: 'email-linje', textContent: m.fejlet ? fmt(T.emailLinje, { n: tal(m.mails), m: tal(m.fejlet), k: m.afsendere }) : fmt(T.emailLinjeOk, { n: tal(m.mails) }) });
    if (!m.liste.length) return V.blok(T.emailH, intro, top);
    const th = (t, k) => { const e = el('th', { textContent: t, className: k || 'v' }); e.setAttribute('scope', 'col'); return e; };
    const rk = m.liste.map(x => el('tr', {}, el('td', { className: 'v', textContent: x.afsender || T.emailUkendt }), el('td', { className: 'v', textContent: x.land || '–' }), el('td', { className: 'tal-kol', textContent: tal(x.fejlet) }), el('td', { className: 'tal-kol', textContent: tal(x.mails) })));
    const rul = el('div', { className: 'vg-tabel-rul', tabIndex: 0 }, el('table', { className: 'vg-tabel email-tabel' }, el('thead', {}, el('tr', {}, th(T.emailAfsender), th(T.emailLand), th(T.emailFejlet, 'tal-kol'), th(T.emailMails, 'tal-kol'))), el('tbody', {}, ...rk)));
    rul.setAttribute('role', 'region');
    rul.setAttribute('aria-label', T.emailH);
    return V.blok(T.emailH, intro, top, rul, ...fod);
  }
  function emailBlok(d, g) {
    const plads = el('div'), z = zone();
    plads.dataset.plads = 'email';
    queueMicrotask(async () => {
      let m;
      try { m = await invoke('email_oversigt', { zone: z, fra: d.periode.fra, til: d.periode.til }); } catch (e) { return; }
      if (g === gen && plads.isConnected && m) plads.replaceChildren(emailIndhold(m));
    });
    return [plads];
  }
  /* "Følsomme filer, der svarede 200" (sti200.rs), kun fra cachen og for perioden. Hver sti åbner Angreb filtreret på stien. Antal, snit og vurdering har intet mere bag sig. */
  const bytes = b => (b < 1024 ? `${tal(b)} B` : `${tal(Math.round(b / 102.4) / 10)} KB`);
  function sti200Indhold(o) {
    const intro = fmt(T.sti200P, { p: periodeTekst(), n: tal(o.liste) });
    if (!o.stier.length) return V.blok(T.sti200H, intro, el('p', { className: 'sti200-tom', textContent: o.hentede_dage ? fmt(T.sti200Ingen, { n: tal(o.liste) }) : T.sti200Venter }));
    const th = (t, k) => { const e = el('th', { textContent: t, className: k || 'v' }); e.setAttribute('scope', 'col'); return e; };
    const rk = o.stier.map(x => {
      const b = knap(x.sti, medFilter('sti', x.sti), 'link dyk-celle', fmt(T.sti200Aabn, { sti: x.sti }));
      b.dataset.fokus = 'sti200:' + x.sti;
      return el('tr', { className: x.troevaerdig ? 'sti200-fil' : '' }, el('td', { className: 'v' }, b), el('td', { className: 'tal-kol', textContent: tal(x.n) }), el('td', { className: 'tal-kol', textContent: bytes(x.snit) }), el('td', { className: 'v', textContent: x.troevaerdig ? T.sti200Fil : T.sti200Side }));
    });
    const rul = el('div', { className: 'vg-tabel-rul', tabIndex: 0 }, el('table', { className: 'vg-tabel sti200-tabel' }, el('thead', {}, el('tr', {}, th(T.sti200ThSti), th(T.sti200ThN, 'tal-kol'), th(T.sti200ThSnit, 'tal-kol'), th(T.sti200ThVurd))), el('tbody', {}, ...rk)));
    rul.setAttribute('role', 'region');
    rul.setAttribute('aria-label', T.sti200H);
    const fod = [el('p', { className: 'vg-note', textContent: fmt(T.sti200Note, { f: o.forside ? bytes(o.forside) : T.sti200UdenForside }) })];
    if (o.stier.some(x => x.troevaerdig)) { const b = S.bog('sti200'); fod.push(el('details', { className: 'dyk-guide' }, el('summary', { textContent: T.sti200Guide }), el('p', { textContent: b.alle }), el('ol', {}, ...b.ja.map(t => el('li', { textContent: t }))))); }
    return V.blok(T.sti200H, intro, rul, ...fod);
  }
  function sti200Blok(d, g) {
    const plads = el('div'), z = zone();
    plads.dataset.plads = 'sti200';
    queueMicrotask(async () => {
      let o;
      try { o = await invoke('sti200_oversigt', { zone: z, fra: d.periode.fra, til: d.periode.til }); } catch (e) { return; }
      if (g === gen && plads.isConnected && o) plads.replaceChildren(sti200Indhold(o));
    });
    return [plads];
  }
  /* "Cloudflares egne fund" (sikkerhed.rs), kun fra cachen. Hvert fund åbner Cloudflares faste side for Security Insights (aabn_sikkerhedsfund). Uden rettigheden står kun teksten om den. */
  const ALVOR_CHIP = { critical: 'roed', high: 'roed', moderate: 'gul', low: 'graa' };
  function cfIndhold(f) {
    const linjer = [];
    if (f.tilstand === 'mangler_ret') linjer.push(el('p', { className: 'cf-ret', textContent: T.cfManglerRet }));
    else if (f.tilstand === 'venter') linjer.push(el('p', { className: 'cf-tom', textContent: T.cfVenter }));
    else if (f.tilstand === 'fejl') linjer.push(el('p', { className: 'vg-advarsel vg-gul', textContent: fmt(T.cfFejl, { f: f.fejl || '' }) }));
    if (f.tilstand !== 'mangler_ret' && f.fund.length) {
      linjer.push(el('ul', { className: 'cf-liste' }, ...f.fund.map(x => {
        const alvor = String(x.alvor || '').toLowerCase(), titel = T.cfKlasse[x.klasse] || String(x.klasse || '').replace(/_/g, ' ');
        const b = el('button', { type: 'button', className: 'link cf-fund', textContent: titel });
        b.setAttribute('aria-label', fmt(T.cfAabn, { t: titel }));
        b.dataset.fokus = 'cf:' + x.klasse;
        b.addEventListener('click', () => { if (invoke) invoke('aabn_sikkerhedsfund').catch(() => { }); });
        return el('li', {}, V.chip(ALVOR_CHIP[alvor] || 'graa', T.cfAlvor[alvor] || x.alvor), ' ', b, x.siden ? el('small', { className: 'cf-siden', textContent: ' · ' + fmt(T.cfSiden, { d: V.datoLang(x.siden) }) }) : '');
      })));
    } else if (f.tilstand === 'ok') linjer.push(el('p', { className: 'cf-tom', textContent: T.cfIngen }));
    if (f.hentet && f.tilstand !== 'venter') linjer.push(el('p', { className: 'vg-note', textContent: fmt(T.cfHentet, { t: V.datoLang(f.hentet.slice(0, 10)) }) }));
    return V.blok(T.cfH, T.cfP, ...linjer);
  }
  function cfBlok(g) {
    const plads = el('div'), z = zone();
    plads.dataset.plads = 'cf';
    queueMicrotask(async () => {
      let f;
      try { f = await invoke('sikkerhedsfund_oversigt', { zone: z }); } catch (e) { return; }
      if (g === gen && plads.isConnected && f) plads.replaceChildren(cfIndhold(f));
    });
    return [plads];
  }
  /* "Vurdering": for én dag på ét domæne. Uden en valgt dag er det i går. */
  const vurdering = d => [V.vurderingBoks({ zone: zone(), navn: domaene, dag: har('dag') ? params.get('dag') : d.i_gaar })];

  /* ── afsnittet "Trafik den …" ── */
  const pct = (a, b) => `${tal(Math.round(100 * a / b))}${S.lang === 'da' ? ' %' : '%'}`;
  /* Sætninger i klarsprog, kun når én ting fylder mere end halvdelen. Tallene siger, hvor trafikken kom fra, ikke hvad den var. */
  function trafikOrd(t) {
    const ud = [], [land] = t.lande, [kode] = t.status, [robot] = t.robotter, stor = (n, i) => i > 0 && 2 * n > i;
    if (land && stor(land.n, t.lande_i_alt)) ud.push(fmt(T.trafikEtLand, { p: pct(land.n, t.lande_i_alt), land: V.land(land.v) }));
    if (stor(t.server_fejl, t.status_i_alt)) ud.push(fmt(T.trafik52, { p: pct(t.server_fejl, t.status_i_alt) }));
    else if (kode && stor(kode.n, t.status_i_alt)) ud.push(fmt(T.trafikKode, { p: pct(kode.n, t.status_i_alt), k: kode.v }));
    if (robot && stor(robot.n, t.i_alt)) ud.push(fmt(T.trafikRobot, { r: V.robotNavn(robot.v) }));
    return ud;
  }
  /* Et land kan kun åbnes, hvis Cloudflare stoppede noget derfra den dag: forespørgslerne selv har vi kun som tal. Svarkoderne er rene tal. */
  function trafikAfsnit(t) {
    const uden = fn => kopi(p => { p.delete('trafik'); fn(p); });
    const normalt = t.median === null ? '' : fmt(T.trafikNormalt, { m: tal(Math.round(t.median)) });
    const dele = [el('div', { className: 'vg-fliser' }, flise(t.i_alt, T.trafikIAlt, normalt, '', true), ...(t.sikkerhed ? [flise(t.stoppet, T.trafikStoppet, '', uden(() => { }), false)] : [flise(t.trusler, T.trafikTrusler, '', '', true), flise(t.cachet, T.trafikCachet, '', '', true)]))];
    if (!t.sikkerhed) dele.push(el('p', { className: 'vg-note', textContent: T.trafikKunAar }));
    const tekst = trafikOrd(t);
    if (tekst.length) dele.push(el('ul', { className: 'vg-fordeling' }, ...tekst.map(x => el('li', { textContent: x }))));
    if (!t.ekstra) dele.push(el('p', { className: 'vg-note', textContent: T.trafikIngenEkstra }));
    const kol = (h, rk) => (rk.length ? [el('div', {}, el('h3', { textContent: h }), V.liste(rk))] : []);
    const lande = kol(T.trafikLande, t.lande.map(x => ({ navn: V.land(x.v), n: x.n, ...(x.angreb ? { dyk: uden(p => p.set('land', x.v)) } : {}) })));
    if (lande.length) dele.push(el('div', { className: 'dyk-lister dyk-trafik-lande' }, ...lande));
    const boks = el('div', { className: 'vg-tre dyk-lister' },
      ...kol(T.trafikStatus, t.status.map(x => ({ navn: x.v, n: x.n }))),
      ...kol(T.trafikRobotter, t.robotter.map(x => ({ navn: V.robotNavn(x.v), n: x.n, dyk: uden(p => p.set('robot', x.v)) }))));
    if (boks.childElementCount) dele.push(V.blok(T.dykListerH, T.dykTrafikP, boks));
    else if (t.ekstra && !lande.length) dele.push(el('p', { className: 'vg-status dyk-tom', textContent: T.trafikTomt }));
    dele.push(el('details', { className: 'dyk-guide' }, el('summary', { textContent: T.gTrafikH }), ...V.bogAfsnit('trafik'), el('h4', { textContent: T.gDnsH }), ...V.bogAfsnit('dns')));
    return afsnit('vg-trafik-dag', fmt(T.dykTrafikH, { v: V.interval(t.dag, t.til) }), '', ...dele);
  }

  /* ── tegning ── */
  /* Det valg, der havde fokus, da siden skiftede: "Henter …" fjerner det, og det skal have fokus igen bagefter. */
  let valgtFokus = null;
  const fokusNu = () => (document.activeElement && document.activeElement.dataset ? document.activeElement.dataset.fokus : null);
  function tegn(d, trafik) {
    const liste = filtre(), g = gen, skjult = [], et = !samlet() && Boolean(zone());
    S.saetDomaene(domaene, samlet());
    R.titel();
    visOverskrift();
    chips(d, liste);
    const p = params.get('periode'), fast = !har('dag') && !har('fra');
    $('dyk-periode').replaceChildren(segment('dyk-periode', T.dykPeriode, [[T.dykP7, medPeriode('periode', '7'), fast && p !== '30' && p !== 'alt'], [T.dykP30, medPeriode('periode', '30'), fast && p === '30']]));
    const genaabn = V.husk(rod), fokus = fokusNu() || valgtFokus;
    const vaelger = V.datoVaelger({ visning: 'vaernet', periode: d.periode, nulstil: medPeriode('periode', 'alt'), gaa: (a, b) => R.gaaTil('vaernet', kopi(q => { PERIODE.forEach(x => q.delete(x)); q.set('fra', a); q.set('til', b); })) });
    const bjaelke = el('div', { className: 'vg-bjaelke' }, el('p', { className: 'vg-status', textContent: stat.tekst }));
    const advarsel = stat.advarsel ? [el('p', { className: `vg-advarsel vg-${stat.advarsel[1]}`, textContent: stat.advarsel[0] })] : [];
    advarsel.forEach(a => a.setAttribute('role', 'alert'));
    const filter = liste.length > 0, vurd = et && (har('dag') || !filter);
    if (et && !vurd) skjult.push(L.vurderingNavn);
    if (har('dag') && d.sum) skjult.push(L.soejlerNavn);
    /* Samme rækkefølge som altid: datovælger → Vis → trafik og hoved → fliser → fordeling → kort → lister → søjler → regler → resten, og "De seneste" sidst. */
    const dele = [bjaelke, ...advarsel, ...kunIDag(d, liste), ...trafikLinjer(d), vaelger, ...(har('regel') || har('forsoeg') || erRobot() ? [] : [slagsVaelger(d)]), ...kunTrafik(d), ...(trafik ? [trafikAfsnit(trafik)] : []), ...hoved(d), fliser(d), ...(kunTrafik(d).length ? [] : V.daekning(d.periode)), ...fordeling(d), ...(vurd ? vurdering(d) : [])];
    if (d.loft) dele.push(el('p', { className: 'vg-advarsel vg-gul vg-loft', textContent: fmt(T.dykLoft, { n: tal(d.sum) }) }));
    /* V-SAMPLE.1: ældre døgn er talte rækker, nyere Cloudflares estimat. Kun når perioden krydser grænsen. */
    if (d.vaegt_fra) dele.push(el('p', { className: 'vg-note vg-vaegt', textContent: fmt(T.dykVaegtGraense, { d: V.datoLang(d.vaegt_fra) }) }));
    if (d.sum) dele.push(...kortBlok(d), ...lister(d), ...soejleBlok(d), ...reglerBlok(d, liste));
    else dele.push(el('p', { className: 'vg-status dyk-tom', textContent: T.dykTomt }), ...reglerBlok(d, liste));
    if (et && !filter) dele.push(...tekBlok(g)); else if (et) skjult.push(T.tekH);
    if (d.sum) dele.push(...rettetBlok(d));
    if (et && !filter) dele.push(...emailBlok(d, g)); else if (et) skjult.push(T.emailH);
    if (et && !filter) dele.push(...sti200Blok(d, g), ...cfBlok(g)); else if (et) skjult.push(T.sti200H, T.cfH);
    if (visForsoeg(d)) dele.push(...forsoegBlok(d)); else if (d.forsoeg && d.forsoeg.kategorier.length && !har('forsoeg')) skjult.push(L.forsoegH);
    dele.push(...andre(d, liste));
    if (d.sum) dele.push(...enkelte(d));
    if (skjult.length && (filter || har('slags') || har('dag'))) dele.push(el('p', { className: 'vg-note ang-skjult', textContent: fmt(L.skjult, { liste: skjult.join(', ') }) }));
    dele.push(el('p', { className: 'vg-note', textContent: T.dykNote }));
    rod.replaceChildren(...dele);
    tegnMoenster();
    genaabn();
    rod.querySelectorAll('.vg-soejler-rul').forEach(r => { r.scrollLeft = r.scrollWidth; });
    rod.classList.toggle('vg-henter', stat.henter || Boolean(stat.advarsel));
    if (stat.henter) rod.setAttribute('aria-busy', 'true'); else rod.removeAttribute('aria-busy');
    const igen = fokus ? [...document.querySelectorAll('[data-fokus]')].find(x => x.dataset.fokus === fokus) : null;
    if (igen) igen.focus();
    valgtFokus = null;
    /* "Se de N angreb": første gang rulles der til listen, eller til reglerne, når de enkelte angreb ikke er gemt. Tilbage/frem gendanner selv rullepositionen. */
    if (params.get('vis') === 'seneste') {
      params.delete('vis');
      const maal = $('vg-seneste') || $('vg-regler');
      if (maal && maal.scrollIntoView && !R.genbesoeg()) maal.scrollIntoView({ block: 'start' });
    }
  }
  function besked(tekst, farve = 'roed') {
    rod.removeAttribute('aria-busy');
    const p = el('p', { className: `vg-advarsel vg-${farve}`, textContent: tekst });
    p.setAttribute('role', 'alert');
    rod.replaceChildren(p);
    $('dyk-filtre').hidden = true;
    $('dyk-periode').replaceChildren();
  }

  /* ── etiketterne ── */
  /* De valgte domæner som etiketter: de første 8, resten bag "+ N flere". Hver etiket åbner domænets eget Angreb. Domæner uden tal står stiplet. */
  const VALGTE_SYNLIGE = 8;
  function visValgte(valgte) {
    const boks = $('valgte');
    boks.replaceChildren();
    boks.hidden = !valgte.length;
    const liste = rk => {
      const ul = el('ul', { className: 'hm-valgte-liste' });
      for (const v of rk) {
        const a = el('a', { href: '#vaernet?' + new URLSearchParams({ zone: v.id, navn: v.navn }), textContent: v.navn }), li = el('li');
        if (!v.med_tal) {
          li.className = 'hm-valgt-uden';
          a.append(el('span', { className: 'sr-only', textContent: ` (${L.udenTal})` }));
        }
        li.append(a);
        ul.append(li);
      }
      return ul;
    };
    const synlige = liste(valgte.slice(0, VALGTE_SYNLIGE));
    synlige.setAttribute('aria-label', L.valgteH);
    boks.append(synlige);
    const rest = valgte.slice(VALGTE_SYNLIGE);
    if (rest.length) boks.append(el('details', { className: 'hm-valgte-flere' }, el('summary', { textContent: fmt(L.flere, { n: rest.length }) }), liste(rest)));
  }
  function visDomaene(tekst, flertal) {
    domaene = tekst;
    S.saetDomaene(tekst, flertal);
    visOverskrift();
    R.titel();
  }

  /* ── start og stop ── */
  let gen = 0, aktiv = null, sidste = 'samlet=1', afmeld = [];
  function stop() {
    gen++;
    aktiv = null;
    for (const f of afmeld.splice(0)) f();
  }
  /* Tallene for perioden og filtrene, kun fra cachen. */
  async function hentTal(g) {
    let d, t = null;
    const trafik = erTrafik();
    try {
      [d, t] = await Promise.all([
        invoke('dyk', { omfang, filtre: filtre(), periode: periodeArg(), slags: params.get('slags'), side: Number(params.get('side')) || undefined }),
        trafik ? invoke('trafik_dag', { omfang, dag: params.get('dag') || params.get('fra'), til: params.get('til') }) : null,
      ]);
    } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
    if (g === gen) tegn(d, t);
  }
  /* Én side i "De seneste": samme kald som hentTal med den nye side, men kun afsnittet byttes ud. Fokus bliver på knappen, der blev trykket, eller på
     "Side N af M", når den knap ikke er der mere (første/sidste side). */
  async function skiftSide(n, hvilken) {
    const g = gen;
    let d;
    try { d = await invoke('dyk', { omfang, filtre: filtre(), periode: periodeArg(), slags: params.get('slags'), side: n }); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
    if (g !== gen) return;
    if (n > 1) params.set('side', String(n)); else params.delete('side');
    aktiv = params.toString();
    R.erstatAdresse('vaernet', aktiv);
    const gammel = $('vg-seneste'), [ny] = d.sum ? enkelte(d) : [];
    if (!gammel || !ny) return hentTal(g);
    gammel.replaceWith(ny);
    (ny.querySelector(`.dyk-side-knap[data-side="${hvilken}"]`) || ny.querySelector('.dyk-side-nr')).focus({ preventScroll: true });
  }
  /* Tegn igen, når køen melder nyt, højst hvert 3. sekund. */
  function lytPaaKoe(gaelder, igen) {
    let sidst = Date.now(), ventende = null;
    const snart = () => {
      if (ventende) return;
      ventende = setTimeout(() => { ventende = null; sidst = Date.now(); igen(); }, Math.max(0, sidst + 3000 - Date.now()));
    };
    afmeld.push(R.paa('hentekoe', k => { k = k || {}; if (!k.zone || gaelder(k.zone)) snart(); }), () => clearTimeout(ventende));
  }
  /* Filtrene og "Hent nu" ændrer ikke, hvor man står, når man kommer tilbage ad sidebjælken: kun domænet og perioden huskes. */
  const husket = p => { const q = new URLSearchParams(); for (const [k, v] of p) if (['zone', 'navn', 'samlet', 'periode'].includes(k)) q.set(k, v); return q.toString(); };
  async function start(p) {
    stop();
    const g = gen;
    aktiv = p.toString();
    sidste = husket(p);
    params = new URLSearchParams(p);
    valgtFokus = fokusNu() || null;
    vaern = null;
    stat = { tekst: '', henter: false, advarsel: null };
    omfang = null;
    afmeld.push(R.paa('raekker', () => { if (rod.querySelector('.vg-bjaelke')) tegnMoenster(); }));
    visDomaene(samlet() ? '…' : (p.get('navn') || '').slice(0, 253) || '…', samlet());
    R.visVaelger();
    $('valgte').hidden = true;
    $('valgte').replaceChildren();
    rod.setAttribute('aria-busy', 'true');
    rod.replaceChildren(el('p', { className: 'vg-status', textContent: L.henter }));
    if (!invoke) return besked(L.mangler);
    if (!samlet() && !zone()) return besked(L.ingenZone);
    try { await V.klar(); } catch (e) { if (g === gen) besked(fmt(L.fejl, { f: S.fejl(e) })); return; }
    if (g !== gen) return;
    if (samlet()) await startSamlet(g); else await startZone(g);
    if (g !== gen) return;
    /* Et klik på et filter fjerner det, der blev klikket på: fokus går til overskriften, så Tab fortsætter ved chips og vælgere. */
    if (!ren() && !R.genbesoeg() && !fokusNu()) $('vaernet-h').focus({ preventScroll: true });
  }
  async function startZone(g) {
    const z = zone();
    omfang = { zone: z };
    const prioriter = () => invoke('koe_prioritet', { zone: z }).catch(() => { });
    prioriter(); // zonen på skærmen kommer først i køen
    window.addEventListener('online', prioriter);
    afmeld.push(() => window.removeEventListener('online', prioriter));
    const gemte = () => invoke('vaern_cache', { zoneId: z }).catch(() => null);
    vaern = await gemte();
    if (g !== gen) return;
    if (!ren()) {
      /* Et filter eller en periode: kun det gemte. Køen kan stadig melde nyt. */
      stat = { tekst: vaern ? fmt(L.opdateret, { t: kl(vaern.genereret) }) : '', henter: false, advarsel: null };
      await hentTal(g);
      if (g === gen) lytPaaKoe(x => x === z, () => hentTal(g));
      return;
    }
    if (vaern) {
      stat = { tekst: fmt(L.opdaterer, { t: kl(vaern.genereret) }), henter: true, advarsel: null };
      await hentTal(g);
      if (g !== gen) return;
    }
    /* Ét forsøg på at hente nye tal: 'ok', 'net' (prøv igen) eller 'stop'. Svar til et domæne, der ikke vises mere, kasseres. */
    async function hent() {
      try {
        const ny = await invoke('vaern_oversigt', { zoneId: z });
        if (g !== gen) return 'stop';
        vaern = ny;
        stat = { tekst: fmt(L.opdateret, { t: kl(ny.genereret) }), henter: false, advarsel: null };
        await hentTal(g);
        return 'ok';
      } catch (e) {
        if (g !== gen) return 'stop';
        const net = S.erNet(e), efter = await gemte(); // dage, der nåede at komme hjem før fejlen, er gemt
        if (g !== gen) return 'stop';
        if (efter) {
          vaern = efter;
          stat = { tekst: fmt(L.opdateret, { t: kl(efter.genereret) }), henter: false, advarsel: [fmt(L.fejlGemte, { f: S.fejl(e), t: kl(efter.genereret) }), net ? 'gul' : 'roed'] };
          await hentTal(g);
        } else besked(fmt(L.fejl, { f: S.fejl(e) }));
        return net ? 'net' : 'stop';
      }
    }
    const gf = window.hmGenforsoeg($('vg-genforsoeg'), hent);
    afmeld.push(gf.fjern);
    await gf.start();
  }
  /* Det samlede: valget fra Rust, tallene fra cachen; køen henter, og siden tegnes igen, når den melder nyt. */
  async function startSamlet(g) {
    let valg;
    try { valg = await invoke('valg_hent'); } catch (e) { if (g === gen) besked(fmt(L.fejl, { f: S.fejl(e) })); return; }
    if (g !== gen) return;
    if (!valg.length) {
      visDomaene(T.domaener(0), true);
      rod.removeAttribute('aria-busy');
      $('dyk-filtre').hidden = true;
      $('dyk-periode').replaceChildren();
      rod.replaceChildren(window.hmVelkomst.intetValgt());
      return;
    }
    omfang = { zoner: valg };
    visDomaene(T.domaener(valg.length), true);
    R.visVaelger();
    async function tegnSamlet() {
      let s, k = null;
      try { s = await invoke('vaern_samlet_cache', { zoner: valg }); } catch (e) { if (ren()) { if (g === gen) besked(fmt(L.fejl, { f: S.fejl(e) })); return; } s = {}; }
      try { k = await invoke('koe_status'); } catch (e) { k = null; }
      if (g !== gen) return;
      visValgte(s.valgte || []);
      const henter = Boolean(k && (k.tilstand === 'henter' || k.tilstand === 'pause' || k.tilstand === 'venter_kvote'));
      if (!s.vaern && ren()) return besked(L.ingenTalSamlet, 'gul');
      vaern = s.vaern || null;
      stat = { tekst: vaern ? fmt(henter ? L.opdatererSamlet : L.opdateretSamlet, { t: kl(vaern.genereret) }) : '', henter, advarsel: s.med_tal < s.domaener ? [fmt(L.mAfN, { m: s.med_tal, n: s.domaener }), 'gul'] : null };
      await hentTal(g);
    }
    await tegnSamlet();
    if (g === gen) lytPaaKoe(x => valg.includes(x), tegnSamlet);
  }
  R.registrer('vaernet', {
    domaener: true,
    hvad: () => ({ samlet: samlet(), zone: zone(), navn: domaene }),
    titel: () => (erRobot() ? 'robotter' : 'vaernet'),
    /* Et andet domæne: samme filtre og periode. */
    skift(q) {
      const ny = new URLSearchParams(q);
      for (const [k, v] of params) if (!['zone', 'navn', 'samlet', 'side'].includes(k)) ny.append(k, v);
      R.gaaTil('vaernet', ny);
    },
    /* #vaernet uden parametre: det sidst viste (eller det samlede). Samme adresse igen (fx efter "Glem"/nyt token): bliv. */
    vis(p, { samme } = {}) {
      const q = p.toString() ? p : new URLSearchParams(sidste);
      if (samme && aktiv === q.toString()) return;
      start(q);
    },
    skjul: stop,
    sidste: () => sidste,
    aktiv: () => aktiv,
  });
})();
