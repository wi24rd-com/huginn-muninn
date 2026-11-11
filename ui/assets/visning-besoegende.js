/* Huginn and Muninn — Besøgende: hvor mange der besøger sitet, hvorfra, hvilke robotter og hvilke sider. #besoegende?zone=<id>&navn=… eller #besoegende?samlet=1,
   perioden periode=30|aar (ingen = 7 hele dage) eller fra=…&til=… (højst et år), og højst ét filter: land=XX (besøg fra landet pr. dag), sti=/… (visninger af
   siden pr. dag) eller robot=<gruppe> (besøg fra robotgruppen pr. dag). Tallene regnes i Rust (besoegende, kun fra cachen): unikke besøgende er årstallene
   (unikke IP-adresser pr. dag), besøg, kendte robotter og sider er Cloudflares estimat ud fra en stikprøve og findes en måned tilbage plus det, programmet har
   gemt. Perioder over 90 dage vises pr. uge (mandag–søndag, UTC); et klik på en uge viser ugen. Et klik på en dag åbner "Trafik den …" på Angreb, et klik på et
   land, en robotgruppe eller en sti lægger det som filter her (chip med ×), og fliserne sætter perioden. Fliserne er faste og står øverst, over vælgeren;
   under vælgeren står "Valgt periode: …" over alt, der følger perioden. Stier, der svarede med sidens almindelige side, står i en egen liste. Dage uden tal er huller, ikke 0. Intet uden data bag er
   klikbart. Søjlerne er blå (--board-visitors), så de ikke læses som angreb. Tegnes igen, når køen eller de ekstra tal melder nyt, højst hvert 3. sekund; svar,
   der kommer for sent, kasseres (gen). Bygges med createElement. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, B = T.bes, R = window.hmRamme, V = window.hmTegn;
  const $ = id => document.getElementById(id);
  const { el, fmt, invoke } = R, tal = S.tal;
  const TOP = 10;
  const FILTRE = ['land', 'sti', 'robot'];
  const rod = $('besoegende-rod');

  let params = new URLSearchParams(), domaene = '…', sidste = 'samlet=1';
  const samlet = () => params.get('samlet') === '1';
  const interval = () => params.has('fra') && params.has('til');
  const land = () => (/^[A-Z0-9]{2}$/.test(params.get('land') || '') ? params.get('land') : null);
  const sti = () => (params.get('sti') || '').startsWith('/') ? params.get('sti') : null;
  const robot = () => (T.robotGrupper[params.get('robot')] ? params.get('robot') : null);
  const gruppeNavn = id => T.robotGrupper[id][0];
  const periodeArg = () => (interval() ? { spaend: { fra: params.get('fra'), til: params.get('til') } } : { 30: 'd30', aar: 'alt' }[params.get('periode')] || 'd7');
  const omfangQ = () => (samlet() ? { samlet: '1' } : { zone: params.get('zone') || '', navn: domaene });
  /* Adresserne er forespørgsler; gaa() gør dem til et mål i visningen. */
  const kopi = fn => { const p = new URLSearchParams(params); fn(p); return p.toString(); };
  const gaa = q => ({ gaa: 'besoegende?' + q });
  const medPeriode = v => kopi(p => { ['periode', 'fra', 'til'].forEach(k => p.delete(k)); if (v) p.set('periode', v); });
  const medInterval = (a, b) => kopi(p => { ['periode', 'fra', 'til'].forEach(k => p.delete(k)); p.set('fra', a); p.set('til', b); });
  /* Højst ét filter: tallene kommer fra hvert sit kald hos Cloudflare og kan ikke deles på hinanden. */
  const medFilter = (navn, v) => kopi(p => { FILTRE.forEach(k => p.delete(k)); if (v) p.set(navn, v); });
  const periodeNavn = () => (interval() ? V.interval(params.get('fra'), params.get('til')) : { d7: T.dykP7, d30: T.dykP30, alt: B.aar }[periodeArg()]);
  /* "Trafik den …" på Angreb for dagen (data-dyk: dagen er perioden). */
  const dagQ = dag => new URLSearchParams({ ...omfangQ(), dag, trafik: '1' }).toString();
  const procent = (n, i) => (i ? (100 * n / i).toLocaleString(S.lang === 'da' ? 'da-DK' : 'en-GB', { maximumFractionDigits: 1 }) + ' %' : '–');

  /* En flise uden tal er ikke 0 og fører ingen steder hen. Fliserne er faste (de seneste dage) og står over vælgeren; ingen er markeret som valgt, for de
     følger ikke perioden (V-BESOEG.2 FIX 1). Et klik sætter stadig perioden. */
  function flise(sum, tekst, maal, nul) {
    if (sum === null || (sum.med === 0)) return V.fliseTekst('–', tekst, nul || B.ingenTal);
    const sub = sum.med < sum.dage ? fmt(T.dagMed, { m: tal(sum.med), n: tal(sum.dage) }) : '';
    return V.flise(sum.n, tekst, sub, gaa(maal), { label: fmt(B.periodeAabn, { navn: tekst, n: tal(sum.n) }) });
  }
  /* En lille linje under flisen (ren tekst: ingen data bag). */
  const linje = (f, tekst, klasse) => { f.append(el('span', { className: 'sflise-under besoeg-linje ' + klasse, textContent: tekst })); return f; };
  function fliser(d) {
    const boks = el('div', { className: 'sfliser vg-fliser besoeg-fliser' });
    /* Unikke besøgende kan ikke deles på lande eller sider: med et filter siger flisen, at den gælder alt. */
    const alle = land() ? B.alleLande : sti() ? B.alleSider : null;
    const unik = f => (alle ? linje(f, alle, 'besoeg-alle') : f);
    const iGaar = d.i_gaar === null ? null : { n: d.i_gaar, med: 1, dage: 1 };
    boks.append(unik(flise(iGaar, B.fIGaar, medInterval(d.i_gaar_dag, d.i_gaar_dag))));
    boks.append(unik(flise(d.d7, B.f7, medPeriode())), unik(flise(d.d30, B.f30, medPeriode('30'))), unik(flise(d.aar, B.fAar, medPeriode('aar'))));
    const tekst = land() ? fmt(B.fBesoegLand, { land: V.land(land()) }) : sti() ? fmt(B.fBesoegSti, { sti: sti() }) : B.fBesoeg30;
    const ikke = sti() ? d.side_ikke : d.ikke_tilgaengelig;
    const b30 = flise(d.besoeg30, tekst, medPeriode('30'), ikke.length ? B.ikkeTilgKort : B.ingenTal);
    if (d.robot30 && d.robot30.i_alt) linje(b30, fmt(B.robotLinje, { n: tal(d.robot30.n), p: procent(d.robot30.n, d.robot30.i_alt) }), 'besoeg-robot-linje');
    boks.append(b30);
    const s = el('section', { className: 'besoeg-faste' }, el('h2', { className: 'besoeg-faste-h', textContent: B.fasteH }), boks);
    s.setAttribute('aria-label', B.fasteH);
    return s;
  }
  function filterH() {
    if (land()) return [fmt(B.grafLandH, { land: V.land(land()) }), fmt(B.grafLandUgeH, { land: V.land(land()) })];
    if (sti()) return [fmt(B.grafStiH, { sti: sti() }), fmt(B.grafStiUgeH, { sti: sti() })];
    if (robot()) return [fmt(B.grafRobotH, { gruppe: gruppeNavn(robot()) }), fmt(B.grafRobotUgeH, { gruppe: gruppeNavn(robot()) })];
    return [B.grafH, B.grafUgeH];
  }
  function graf(d) {
    const [hDag, hUge] = filterH();
    let h, soejler;
    if (d.uger) {
      /* Én søjle pr. uge: værktøjstippet siger, hvor mange af ugens dage der har tal, og et klik viser ugen. */
      h = hUge;
      const uger = new Map(d.uger.map(u => [u.dag, u]));
      const tekst = x => {
        const u = uger.get(x.dag), v = { a: V.datoLang(u.fra), b: V.datoLang(u.til), n: tal(u.n || 0), m: tal(u.med) };
        return u.n === null ? fmt(B.ugeHul, v) : fmt(u.med < 7 ? B.ugeDele : B.uge, v);
      };
      const dyk = dag => { const u = uger.get(dag); return gaa(medInterval(u.fra, u.til)); };
      /* Et år skal kunne ses på én gang: smallere søjler end dagenes (klikfladen er højere end bred), og aksen viser måneder. */
      soejler = V.soejler(d.uger.map(u => ({ dag: u.dag, n: u.n || 0, hentet: u.n !== null })), { titel: h, dyk, tekst, klasse: 'besoeg-soejle', hit: V.HIT_UGE, maaneder: true });
    } else {
      h = hDag;
      soejler = V.soejler(d.dage.map(x => ({ dag: x.dag, n: x.n || 0, hentet: x.n !== null })), { titel: h, dyk: dagQ, klasse: 'besoeg-soejle' });
    }
    const dele = [soejler];
    if (d.periode.med < d.periode.dage) dele.push(el('p', { className: 'vg-note hul-note', textContent: fmt(B.hulNote, { m: tal(d.periode.med), n: tal(d.periode.dage) }) }));
    const s = V.blok(h, d.uger ? B.grafUgeP : B.grafP, ...dele);
    s.id = 'besoeg-graf';
    return s;
  }
  const th = (t, k) => { const e = el('th', { textContent: t, className: k || '' }); e.setAttribute('scope', 'col'); return e; };
  /* En tabel: første kolonne er et link (filter), resten er tal uden data bag. */
  function tabel(klasse, hoved, rk) {
    const raekker = rk.map(x => {
      const c = el('th', {}, V.knap(x.navn, x.maal, 'link dyk-celle' + (x.kode ? ' besoeg-sti' : ''), x.label));
      c.setAttribute('scope', 'row');
      return el('tr', {}, c, ...x.tal.map(t => el('td', { className: 'tal-kol', textContent: t })));
    });
    return el('table', { className: 'vg-tabel ' + klasse }, el('thead', {}, el('tr', {}, th(hoved[0]), ...hoved.slice(1).map(t => th(t, 'tal-kol')))), el('tbody', {}, ...raekker));
  }
  /* De 10 største fremme, resten bag "Vis alle". */
  function tiFremme(klasse, hoved, rk, noegle, visAlle) {
    const dele = [el('div', { className: 'vg-tabel-rul' }, tabel(klasse, hoved, rk.slice(0, TOP)))];
    if (rk.length > TOP) {
      const flere = el('details', { className: 'dyk-flere' }, el('summary', { textContent: visAlle }), el('div', { className: 'vg-tabel-rul' }, tabel(klasse, hoved, rk.slice(TOP))));
      flere.dataset.noegle = noegle;
      dele.push(flere);
    }
    return dele;
  }
  const ikkeLinje = (liste, en, flere, klasse) => (liste.length ? [el('p', { className: `vg-advarsel vg-gul ${klasse}`, textContent: samlet() ? fmt(flere, { d: liste.join(', ') }) : en })] : []);
  function lande(d) {
    const dele = [], ikke = d.ikke_tilgaengelig;
    dele.push(...ikkeLinje(ikke, B.ikkeTilg, B.ikkeTilgFlere, 'besoeg-ikke'));
    if (!d.lande.length) {
      if (!ikke.length || d.besoeg_fra) dele.push(el('p', { className: 'vg-status dyk-tom', textContent: d.besoeg_fra ? B.landeIngen : B.landeIkkeHentet }));
    } else {
      if (d.besoeg_fra && d.besoeg_fra > d.periode.fra) dele.push(el('p', { className: 'vg-note', textContent: fmt(B.landeFra, { d: V.datoLang(d.besoeg_fra) }) }));
      const rk = d.lande.map(x => { const navn = V.land(x.kode); return { navn, maal: gaa(medFilter('land', x.kode)), label: fmt(B.landAabn, { navn, n: tal(x.n) }), tal: [tal(x.n), procent(x.n, d.lande_i_alt)] }; });
      dele.push(...tiFremme('besoeg-lande', [B.thLand, B.thBesoeg, B.thAndel], rk, 'besoeg-lande', fmt(B.visAlle, { n: tal(d.lande.length) })));
    }
    const s = V.blok(B.landeH, B.landeP, ...dele);
    s.id = 'besoeg-lande';
    return s;
  }
  function robotter(d) {
    const dele = [...ikkeLinje(d.robot_ikke, fmt(B.robotIkkeTilg, { d: domaene }), B.robotIkkeTilg, 'besoeg-robot-ikke')];
    if (d.ipklasse && d.ipklasse.i_alt) {
      const tekst = fmt(B.klasseLinje, { p: procent(d.ipklasse.n, d.ipklasse.i_alt) }) + (d.ipklasse.med < 365 ? ' ' + fmt(B.klasseDage, { m: tal(d.ipklasse.med) }) : '');
      dele.push(el('p', { className: 'besoeg-klasse', textContent: tekst }));
    }
    if (d.robotter.length) {
      const i_alt = d.robotter.reduce((a, x) => a + x.n, 0);
      const rk = d.robotter.map(x => { const navn = gruppeNavn(x.id); return { navn, maal: gaa(medFilter('robot', x.id)), label: fmt(B.gruppeAabn, { navn, n: tal(x.n) }), tal: [tal(x.n), procent(x.n, i_alt)] }; });
      dele.push(el('div', { className: 'vg-tabel-rul' }, tabel('besoeg-robotter', [B.thGruppe, B.thBesoeg, B.thAndel], rk)));
    } else if (!d.robot_ikke.length || d.robot_fra) dele.push(el('p', { className: 'vg-status dyk-tom', textContent: d.robot_fra ? B.robotIngen : B.robotIkkeHentet }));
    dele.push(el('p', { className: 'vg-note besoeg-robot-note', textContent: B.robotNote }));
    const s = V.blok(B.robotH, B.robotP, ...dele);
    s.id = 'besoeg-robotter';
    return s;
  }
  function sider(d) {
    const dele = [...ikkeLinje(d.side_ikke, fmt(B.sideIkkeTilg, { d: domaene }), B.sideIkkeTilg, 'besoeg-side-ikke')];
    const raekke = x => ({ navn: x.sti, kode: true, maal: gaa(medFilter('sti', x.sti)), label: fmt(B.stiAabn, { sti: x.sti, n: tal(x.n) }), tal: [tal(x.n), tal(x.besoeg), procent(x.n, d.sider_i_alt)] });
    const stiRk = x => ({ navn: x.sti, kode: true, maal: gaa(medFilter('sti', x.sti)), label: fmt(B.stiAabn, { sti: x.sti, n: tal(x.n) }), tal: [tal(x.n)] });
    const forside = d.sider_forside || [];
    if (d.sider.length || d.sider_404.length || forside.length) {
      if (d.side_fra && d.side_fra > d.periode.fra) dele.push(el('p', { className: 'vg-note', textContent: fmt(B.sideFra, { d: V.datoLang(d.side_fra) }) }));
      if (d.sider.length) dele.push(...tiFremme('besoeg-sider', [B.thSti, B.thVisninger, B.thBesoegUde, B.thAndel], d.sider.map(raekke), 'besoeg-sider', fmt(B.visAlleSider, { n: tal(d.sider.length) })));
      if (d.sider_404.length) {
        const blok = el('div', { className: 'besoeg-404' }, el('h3', { textContent: B.side404H }), el('p', { className: 'vg-note', textContent: B.side404P }), ...tiFremme('besoeg-sider-404', [B.thSti, B.thVisninger], d.sider_404.map(stiRk), 'besoeg-404', fmt(B.visAlleSider, { n: tal(d.sider_404.length) })));
        dele.push(blok);
      }
      /* Stier, der svarede med sidens almindelige side (catch-all): et skøn ud fra svarenes størrelse, regnet i Rust (besoeg::forside_deling). */
      if (forside.length) {
        const blok = el('div', { className: 'besoeg-forside' }, el('h3', { textContent: B.sideForsideH }), el('p', { className: 'vg-note', textContent: B.sideForsideP }), ...tiFremme('besoeg-sider-forside', [B.thSti, B.thVisninger], forside.map(stiRk), 'besoeg-forside', fmt(B.visAlleSider, { n: tal(forside.length) })), el('p', { className: 'vg-note besoeg-forside-note', textContent: B.sideForsideNote }));
        dele.push(blok);
      }
      if (d.sider_uden_forside) dele.push(el('p', { className: 'vg-note besoeg-uden-forside', textContent: B.sideUdenForside }));
    } else if (!d.side_ikke.length || d.side_fra) dele.push(el('p', { className: 'vg-status dyk-tom', textContent: d.side_fra ? B.sideIngen : B.sideIkkeHentet }));
    dele.push(el('p', { className: 'vg-note besoeg-side-note', textContent: B.sideNote }));
    const s = V.blok(B.sideH, B.sideP, ...dele);
    s.id = 'besoeg-sider';
    return s;
  }
  function tegn(d) {
    $('besoegende-under').textContent = domaene;
    const p = periodeArg();
    /* Øverst de faste fliser, så vælgeren (7/30/Et år og Fra/Til), og under den en overskrift for alt, der følger perioden (V-BESOEG.2 FIX 1). */
    const segment = V.segment('besoegende-periode', T.dykPeriode, [[T.dykP7, medPeriode(), !interval() && p === 'd7'], [T.dykP30, medPeriode('30'), !interval() && p === 'd30'], [B.aar, medPeriode('aar'), !interval() && p === 'alt']], 'besoegende');
    const vaelger = el('div', { className: 'besoeg-valg' }, segment, V.datoVaelger({ visning: 'besoegende', periode: d.periode, nulstil: medPeriode('aar'), gaa: (a, b) => R.gaaTil('besoegende', medInterval(a, b)) }));
    const valgt = el('h2', { className: 'besoeg-valgt-h', textContent: fmt(B.valgtPeriode, { p: V.interval(d.periode.fra, d.periode.til) }) });
    const genaabn = V.husk(rod), dele = [fliser(d), vaelger, valgt];
    const chips = [], chip = (n, klasse) => chips.push(V.knap(`${n} ×`, gaa(medFilter()), 'sti-chip ' + klasse, fmt(T.dykFjern, { navn: n })));
    if (interval()) { const n = periodeNavn(); chips.push(V.knap(`${n} ×`, gaa(medPeriode()), 'sti-chip', fmt(T.dykFjern, { navn: n }))); }
    if (land()) chip(V.land(land()), 'besoeg-land-chip');
    if (sti()) chip(sti(), 'besoeg-sti-chip');
    if (robot()) chip(gruppeNavn(robot()), 'besoeg-robot-chip');
    if (chips.length) dele.push(el('div', { className: 'login-stier besoeg-chips' }, ...chips));
    dele.push(graf(d), lande(d), robotter(d), sider(d));
    dele.push(el('p', { className: 'vg-note besoeg-note', textContent: B.note + (samlet() ? ' ' + B.noteSamlet : '') }));
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(...dele);
    genaabn();
  }
  function besked(tekst, farve = 'roed') {
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(el('p', { className: `vg-advarsel vg-${farve}`, textContent: tekst }));
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
      try { d = await invoke('besoegende', { omfang, periode: periodeArg(), land: land(), sti: sti(), robot: robot() }); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
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
    /* Et andet domæne: samme periode og samme filter. */
    skift(q) {
      const ny = new URLSearchParams(q);
      for (const k of ['periode', 'fra', 'til', ...FILTRE]) if (params.has(k)) ny.set(k, params.get(k));
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
