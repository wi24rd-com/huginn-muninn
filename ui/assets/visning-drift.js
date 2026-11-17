/* Huginn and Muninn — Drift (V-DRIFT.1 B2): svarede dit website, og hvor meget klarede Cloudflare selv. #drift?zone=<id>&navn=… eller #drift?samlet=1,
   perioden periode=30|aar (ingen = 7 hele dage) eller fra=…&til=… (højst et år), dag=YYYY-MM-DD (dagens 24 timer), time=HH (timen i UTC; vist i lokal
   tid), vis=naa|server|timer|forside (fejlfliserne filtrerer tabellen), cache=lager|data|selv|server (cachefliserne filtrerer cachetabellen) og
   vaert=<navn> (filteret pr. subdomæne, kun for ét domæne). Tallene regnes i Rust (drift og drift_fejl, kun fra cachen): fejl pr. dag og time, spændene
   ("mellem kl. … og …"), forsiden som vidne, cache pr. status og, for ét domæne, værterne med fejl. Et tal, kommandoen svarer null på, er "–" eller
   "ikke hentet", aldrig 0. Cachen gælder altid hele domænet: med et filter pr. subdomæne står kun den sætning. Perioder over 90 dage vises pr. uge
   (mandag–søndag, UTC). Intet uden data bag er klikbart. Værtsnavne sættes kun som tekst. Tegnes igen, når køen eller de ekstra tal melder nyt, højst
   hvert 3. sekund; svar, der kommer for sent, kasseres (gen). Bygges med createElement. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, D = T.drift, B = T.bes, U = T.sub, R = window.hmRamme, V = window.hmTegn;
  const $ = id => document.getElementById(id);
  const { el, fmt, invoke } = R, tal = S.tal;
  const TOP = 10, MAKS_SPAEND = 3, UGE_FRA = 90;
  const VIS = ['naa', 'server', 'timer', 'forside'], CACHE = ['lager', 'data', 'selv', 'server'];
  const rod = $('drift-rod');
  const loc = S.lang === 'da' ? 'da-DK' : 'en-GB';

  let params = new URLSearchParams(), domaene = '…', sidste = 'samlet=1';
  const erDag = d => /^\d{4}-\d{2}-\d{2}$/.test(d || '') && !isNaN(Date.parse(`${d}T00:00:00Z`));
  const samlet = () => params.get('samlet') === '1';
  const interval = () => erDag(params.get('fra')) && erDag(params.get('til')) && params.get('fra') <= params.get('til');
  const dagen = () => (erDag(params.get('dag')) ? params.get('dag') : null);
  const timen = () => (dagen() && /^([01]\d|2[0-3])$/.test(params.get('time') || '') ? params.get('time') : null);
  const vis = () => (VIS.includes(params.get('vis')) ? params.get('vis') : null);
  const cacheValg = () => (CACHE.includes(params.get('cache')) ? params.get('cache') : null);
  const vaertHost = () => (samlet() || V.subNoegle(params.get('vaert'), domaene) === null ? null : params.get('vaert').toLowerCase());
  const vaertN = () => (vaertHost() === null ? null : V.subNoegle(vaertHost(), domaene));
  const flyt = (dag, n) => new Date(Date.parse(`${dag}T00:00:00Z`) + n * 864e5).toISOString().slice(0, 10);
  const iDag = () => new Date().toISOString().slice(0, 10);
  /* Perioden: de seneste hele dage til og med i går (tallene findes kun for færdige døgn), eller Fra–Til. */
  function periode() {
    const til = flyt(iDag(), -1);
    if (interval()) return { fra: params.get('fra'), til: params.get('til') };
    return { fra: flyt(til, -({ 30: 29, aar: 364 }[params.get('periode')] || 6)), til };
  }
  /* Det, der vises: dagen, når en dag er valgt, ellers perioden. */
  const udsnit = () => (dagen() ? { fra: dagen(), til: dagen() } : periode());
  const periodeNavn = () => (interval() ? V.interval(params.get('fra'), params.get('til')) : { 30: T.dykP30, aar: B.aar }[params.get('periode')] || T.dykP7);
  const kopi = fn => { const p = new URLSearchParams(params); fn(p); return p.toString(); };
  const gaa = q => ({ gaa: 'drift?' + q });
  const UNDER = ['dag', 'time', 'vis', 'cache'];
  const medPeriode = v => kopi(p => { ['periode', 'fra', 'til', ...UNDER].forEach(k => p.delete(k)); if (v) p.set('periode', v); });
  const medInterval = (a, b) => kopi(p => { ['periode', 'fra', 'til', ...UNDER].forEach(k => p.delete(k)); p.set('fra', a); p.set('til', b); });
  const medDag = dag => kopi(p => { p.delete('time'); p.set('dag', dag); });
  const medTime = hh => kopi(p => { p.set('time', hh); });
  const uden = (...k) => kopi(p => k.forEach(x => p.delete(x)));
  const medValg = (navn, v) => kopi(p => { if (p.get(navn) === v) p.delete(navn); else p.set(navn, v); });
  const medVaert = host => kopi(p => { ['time', 'vis'].forEach(k => p.delete(k)); p.set('vaert', host); });
  /* "Trafik den …" på Angreb for dagen (data-dyk: dagen er perioden). */
  const angrebQ = dag => new URLSearchParams({ ...(samlet() ? { samlet: '1' } : { zone: params.get('zone') || '', navn: domaene }), dag, trafik: '1' }).toString();
  const to = n => String(n).padStart(2, '0');
  /* Klokkeslæt som programmets øvrige ("02.00"), i brugerens lokale tid. t er timens start i UTC ("YYYY-MM-DDTHH"); plus = timer senere. */
  const lokal = (t, plus = 0) => new Date(Date.parse(`${t}:00:00Z`) + plus * 36e5);
  const kl = (t, plus) => { const x = lokal(t, plus); return `${to(x.getHours())}.${to(x.getMinutes())}`; };
  const lokalDag = t => { const x = lokal(t); return `${x.getFullYear()}-${to(x.getMonth() + 1)}-${to(x.getDate())}`; };
  const procent = (n, i) => (i ? (100 * n / i).toLocaleString(loc, { maximumFractionDigits: 1 }) + ' %' : '–');
  const ENHEDER = ['B', 'kB', 'MB', 'GB', 'TB'];
  /* Data i læselige enheder (decimale, som resten af programmet). */
  function data(b) {
    let v = Number(b || 0), i = 0;
    while (v >= 1000 && i < ENHEDER.length - 1) { v /= 1000; i++; }
    return `${v.toLocaleString(loc, { maximumFractionDigits: i ? 1 : 0 })} ${ENHEDER[i]}`;
  }
  /* Andelene i tiendedele procent, så de altid summer til 100 (de største rester rundes op). */
  function andele(tal4) {
    const sum = tal4.reduce((a, x) => a + x, 0);
    if (!sum) return tal4.map(() => 0);
    const raa = tal4.map(x => 1000 * x / sum), ned = raa.map(Math.floor);
    let rest = 1000 - ned.reduce((a, x) => a + x, 0);
    for (const i of raa.map((x, j) => [x - ned[j], j]).sort((a, b) => b[0] - a[0] || a[1] - b[1]).map(x => x[1])) { if (rest-- <= 0) break; ned[i]++; }
    return ned.map(x => x / 10);
  }
  const pct = v => v.toLocaleString(loc, { minimumFractionDigits: 1, maximumFractionDigits: 1 }) + ' %';

  /* ── fliser: kittets statusflise. Et tal med rækker bag filtrerer tabellen (og fjerner filteret igen); 0, "–" og Nej er ikke klikbare. ── */
  const intet = () => { };
  function flise(o) {
    const klik = Boolean(o.maal), label = klik ? fmt(o.aktiv ? D.fliseLuk : D.fliseAabn, { navn: o.tekst, n: o.vaerdi }) : undefined;
    const f = window.Kit.statusflise({ tal: o.vaerdi, tekst: o.tekst, under: o.under || undefined, ved: klik ? intet : undefined, aria: label, valgt: (klik && o.aktiv) || undefined });
    f.classList.add('drift-flise', ...(o.klasse ? [o.klasse] : []));
    if (klik) V.maal(f.matches('button') ? f : f.querySelector('.sflise-maal'), o.maal).classList.add('dyk-flise');
    return f;
  }
  /* Summen af et felt over dagene: null, når ingen dag har tallet (ikke hentet er aldrig 0). */
  function sum(dage, felt) {
    const med = dage.filter(x => felt(x) !== null && felt(x) !== undefined);
    return med.length ? { n: med.reduce((a, x) => a + felt(x), 0), med: med.length, dage: dage.length } : null;
  }
  const daekning = s => (s && s.med < s.dage ? fmt(D.dagMed, { m: tal(s.med), n: tal(s.dage) }) : '');
  const talFlise = (s, tekst, noegle, klasse, under) => (s === null
    ? flise({ vaerdi: '–', tekst, under: D.ikkeHentet, klasse })
    : flise({ vaerdi: tal(s.n), tekst, under: [under, daekning(s)].filter(Boolean).join(' · '), klasse, maal: s.n ? gaa(medValg('vis', noegle)) : null, aktiv: vis() === noegle }));
  function fejlFliser(d) {
    const boks = el('div', { className: 'sfliser vg-fliser drift-fliser' });
    if (vaertHost() !== null) {
      const vd = d.vaert ? d.vaert.dage : [];
      const s = sum(vd, x => (x.kunne_ikke === null || x.kunne_ikke_530 === null ? null : x.kunne_ikke + x.kunne_ikke_530));
      boks.append(talFlise(s, D.kunneIkke, 'naa', 'drift-f-naa'));
      for (const [t, k] of [[D.serverFejl, 'server'], [D.timer, 'timer'], [D.forside, 'forside']]) boks.append(flise({ vaerdi: '–', tekst: t, under: D.ikkePrSub, klasse: 'drift-f-' + k }));
      return boks;
    }
    const naa = x => (x.kunne_ikke === null || x.kunne_ikke_530 === null ? null : x.kunne_ikke + x.kunne_ikke_530);
    const sNaa = sum(d.dage, naa), begge = d.dage.filter(x => naa(x) !== null && x.svar !== null), svar = begge.reduce((a, x) => a + x.svar, 0);
    boks.append(talFlise(sNaa, D.kunneIkke, 'naa', 'drift-f-naa', sNaa && svar ? fmt(D.kunneIkkeUnder, { p: procent(begge.reduce((a, x) => a + naa(x), 0), svar) }) : ''));
    boks.append(talFlise(sum(d.dage, x => x.website_fejl), D.serverFejl, 'server', 'drift-f-server'));
    const hentet = d.dage.filter(x => x.hentet && !x.afskaaret.includes('d_time') && !x.afskaaret.includes('d_fejl'));
    boks.append(talFlise(hentet.length ? { n: hentet.reduce((a, x) => a + x.timer_med_fejl.length, 0), med: hentet.length, dage: d.dage.length } : null, D.timer, 'timer', 'drift-f-timer'));
    const fs = d.dage.filter(x => x.forside_fejl !== null), ja = fs.some(x => x.forside_fejl);
    boks.append(!fs.length ? flise({ vaerdi: '–', tekst: D.forside, under: D.ikkeHentet, klasse: 'drift-f-forside' })
      : flise({ vaerdi: ja ? D.ja : D.nej, tekst: D.forside, under: daekning({ med: fs.length, dage: d.dage.length }), klasse: 'drift-f-forside', maal: ja ? gaa(medValg('vis', 'forside')) : null, aktiv: vis() === 'forside' }));
    return boks;
  }

  /* ── uger (perioder over UGE_FRA dage): mandag–søndag i UTC. felter lægges sammen; en uge uden en eneste dag med tal er ikke hentet. ── */
  function uger(dage, felter) {
    const ud = new Map();
    for (const x of dage) {
      const t = new Date(`${x.dag}T00:00:00Z`), man = flyt(x.dag, -((t.getUTCDay() + 6) % 7));
      const u = ud.get(man) || ud.set(man, { dag: man, fra: x.dag, til: x.dag, hentet: false, ...Object.fromEntries(felter.map(f => [f, 0])) }).get(man);
      u.til = x.dag;
      if (x.hentet !== false) { u.hentet = true; for (const f of felter) u[f] += x[f] || 0; }
    }
    return [...ud.values()];
  }

  /* ── grafen over fejl: pr. dag (eller uge), og for en valgt dag dagens 24 timer i lokal tid. ── */
  function fejlGraf(d) {
    const dag = dagen(), vaert = vaertHost() !== null, dele = [];
    if (dag) {
      const tilbage = V.knap(D.tilbage, gaa(uden('dag', 'time')), 'link drift-tilbage', fmt(D.tilbageAria, { p: periodeNavn() }));
      const angreb = V.knap(D.angrebDag, angrebQ(dag), 'link drift-angreb', fmt(D.angrebDagAria, { d: V.datoLang(dag) }));
      dele.push(el('p', { className: 'drift-veje' }, tilbage, ' · ', angreb));
      if (vaert) dele.push(el('p', { className: 'vg-note drift-timer-ikke', textContent: D.timerIkkeSub }));
      else if (!d.timer.length) dele.push(el('p', { className: 'vg-status dyk-tom drift-ikke-hentet', textContent: D.ikkeHentetP }));
      else {
        const rk = d.timer.map(t => ({ dag: t.time, n: t.fejl.reduce((a, x) => a + x.svar, 0), hentet: t.svar !== null && d.dage[0].fejl !== null }));
        const v = x => ({ a: kl(`${dag}T${x.dag}`), b: kl(`${dag}T${x.dag}`, 1), n: tal(x.n) });
        const tekst = x => fmt(x.hentet === false ? D.timeIkke : x.n ? D.time : D.timeNul, v(x));
        const h = fmt(D.grafTimeH, { d: V.datoLang(dag) });
        dele.unshift(el('h3', { className: 'drift-graf-h', textContent: h }), el('p', { className: 'vg-intro', textContent: D.grafTimeP }));
        dele.push(V.soejler(rk, { titel: `${h}. ${T.tastatur}`, dyk: hh => gaa(medTime(hh)), tekst, klasse: 'drift-soejle', akse: (x, i) => (i % 3 === 0 ? kl(`${dag}T${x.dag}`) : '') }));
      }
      return el('div', { className: 'drift-graf', id: 'drift-graf' }, ...dele);
    }
    const raa = vaert
      ? (d.vaert ? d.vaert.dage : []).map(x => ({ dag: x.dag, n: x.hentet ? x.kunne_ikke + x.kunne_ikke_530 + x.andre_5xx : 0, hentet: x.hentet }))
      : d.dage.map(x => ({ dag: x.dag, n: x.fejl || 0, hentet: x.fejl !== null }));
    const pr_uge = raa.length > UGE_FRA, h = pr_uge ? D.grafUgeH : D.grafH;
    dele.push(el('h3', { className: 'drift-graf-h', textContent: h }), el('p', { className: 'vg-intro', textContent: pr_uge ? D.grafUgeP : D.grafP }));
    if (pr_uge) {
      const ug = uger(raa, ['n']), pr = new Map(ug.map(u => [u.dag, u]));
      const tekst = x => { const u = pr.get(x.dag); return fmt(!u.hentet ? D.ugeIkke : u.n ? D.uge : D.ugeNul, { a: V.datoLang(u.fra), b: V.datoLang(u.til), n: tal(u.n) }); };
      dele.push(V.soejler(ug, { titel: `${h}. ${T.tastatur}`, dyk: m => gaa(medInterval(pr.get(m).fra, pr.get(m).til)), tekst, klasse: 'drift-soejle', hit: V.HIT_UGE, maaneder: true }));
    } else {
      const tekst = x => fmt(x.hentet === false ? D.dagIkke : x.n ? D.dag : D.dagNul, { d: V.datoLang(x.dag), n: tal(x.n) });
      dele.push(V.soejler(raa, { titel: `${h}. ${T.tastatur}`, dyk: x => gaa(medDag(x)), tekst, klasse: 'drift-soejle' }));
    }
    return el('div', { className: 'drift-graf', id: 'drift-graf' }, ...dele);
  }

  /* ── tabellen: ét kode-par pr. domæne (eller én kode pr. valgt subdomæne). Domænet åbner Drift for domænet, når flere står sammen. ── */
  const th = (t, k) => { const e = el('th', { textContent: t, className: k || '' }); e.setAttribute('scope', 'col'); return e; };
  function hvad(r) {
    if (r.slags === 'forside') return D.forsideRk;
    if (r.naa) { const t = fmt(D.kodeTal, { t: D.kode[r.kant] || D.kodeNaa, kode: r.kant }); return r.server ? fmt(D.ogServer, { t, kode: r.origin }) : t; }
    /* FIX 1: uden "kunne ikke nås". En server bag Cloudflare svarede selv med en fejl (origin 500–599); eller ingen server svarede (origin 0) med et
       almindeligt fejlsvar (500–511): et website hos Cloudflare, der fejlede, eller Cloudflares egen fejl. Alt andet står med koden alene. */
    if (r.server) return fmt(D.kodeServer, { kode: r.origin });
    return fmt(r.origin === 0 && r.kant >= 500 && r.kant <= 511 ? D.kodeUden : D.kodeAnden, { kode: r.kant });
  }
  function hvornaar(r) {
    const en = t => (r.slags === 'vaert' ? V.datoKort(t) : fmt(D.kl, { d: V.datoKort(lokalDag(t)), t: kl(t) }));
    return r.foerste === r.sidste ? en(r.foerste) : `${en(r.foerste)} – ${en(r.sidste)}`;
  }
  function raekker(f) {
    const hh = timen(), v = vis(), vaert = vaertHost() !== null;
    let rk = f.raekker.filter(r => (vaert ? r.slags === 'vaert' : v === 'forside' ? r.slags === 'forside' : r.slags === 'kode'));
    if (v === 'naa') rk = rk.filter(r => r.naa);
    if (!vaert && v === 'server') rk = rk.filter(r => r.website);
    if (!vaert && v === 'timer') rk = rk.filter(r => r.i_fejltimer > 0);
    const n = r => (hh && !vaert ? (r.timer.find(t => t.time === hh) || { svar: 0 }).svar : v === 'timer' && !vaert ? r.i_fejltimer : r.svar);
    return rk.map(r => ({ r, n: n(r) })).filter(x => x.n > 0).sort((a, b) => b.n - a.n);
  }
  function tabel(rk) {
    const vaert = vaertHost() !== null, navne = new Map(R.tilstand.raekker.map(x => [x.id, x.navn]));
    const tr = ({ r, n }) => {
      let celle;
      if (vaert) celle = el('th', {}, V.domaene(vaertHost()));
      else {
        const navn = navne.get(r.zone) || (samlet() ? '–' : domaene);
        /* Med flere domæner åbner navnet Drift for domænet; med ét er det allerede det, der vises. */
        celle = el('th', {}, samlet() && navne.has(r.zone) ? V.knap(V.domaene(navn), gaa(kopi(p => { ['samlet', 'vaert'].forEach(k => p.delete(k)); p.set('zone', r.zone); p.set('navn', navn); })), 'link dyk-celle drift-dom', fmt(D.domAabn, { navn })) : V.domaene(navn));
      }
      celle.setAttribute('scope', 'row');
      const e = el('tr', { className: 'drift-raekke' }, celle, el('td', { className: 'drift-hvad', textContent: hvad(r) }), el('td', { className: 'tal-kol', textContent: tal(n) }), el('td', { className: 'drift-hvornaar', textContent: hvornaar(r) }));
      e.dataset.kode = r.slags === 'forside' ? 'forside' : `${r.kant}/${r.origin}`;
      return e;
    };
    const hoved = () => [th(vaert ? D.thSub : D.thDomaene), th(D.thHvad), th(D.thSvar, 'tal-kol'), th(vaert ? D.thDage : D.thTimer)];
    const dele = [V.tabelRul(V.laeseTabel('drift-tabel', hoved(), rk.slice(0, TOP).map(tr)), D.tabelH)];
    if (rk.length > TOP) {
      const flere = el('details', { className: 'dyk-flere' }, el('summary', { textContent: fmt(D.visAlle, { n: tal(rk.length) }) }), V.tabelRul(V.laeseTabel('drift-tabel', hoved(), rk.slice(TOP).map(tr)), D.tabelH));
      flere.dataset.noegle = 'drift-tabel';
      dele.push(flere);
    }
    return dele;
  }
  /* Linjen fra INVESTIGATION Q2: spændet, eller at forsiden virkede. Med flere domæner eller flere dage står domænet og dagen foran. */
  function linjer(d, f) {
    if (vaertHost() !== null) return [];
    const navne = new Map(R.tilstand.raekker.map(x => [x.id, x.navn])), ud = [];
    for (const s of f.spaend.slice(0, MAKS_SPAEND)) {
      const t = fmt(s.forside_fejlede ? D.mellemForside : D.mellem, { a: kl(s.fra), b: kl(s.til, 1) });
      const foran = [samlet() ? navne.get(s.zone) : null, dagen() ? null : V.datoLang(lokalDag(s.fra))].filter(Boolean).join(', ');
      ud.push(el('p', { className: 'vg-advarsel vg-gul drift-mellem', textContent: foran ? fmt(D.medForan, { p: foran, t }) : t }));
    }
    if (f.spaend.length > MAKS_SPAEND) ud.push(el('p', { className: 'vg-note drift-mellem-flere', textContent: fmt(D.flereSpaend, { n: f.spaend.length - MAKS_SPAEND }) }));
    /* FIX 1: forsiden svarede med en fejl uden for et spænd (fx et website hos Cloudflare, der fejler: ingen time med fejl, for de tæller kun "kunne
       ikke nås"). Linjen siger, at websitet ikke svarede, uden at påstå, at Cloudflare ikke kunne nå en server. */
    const forsideN = f.raekker.filter(r => r.slags === 'forside').reduce((a, r) => a + r.svar, 0);
    if (forsideN > 0 && !f.spaend.some(s => s.forside_fejlede)) ud.push(el('p', { className: 'vg-advarsel vg-gul drift-forside-linje', textContent: fmt(D.forsideLinje, { n: forsideN }) }));
    const fejl = sum(d.dage, x => x.fejl), fs = d.dage.filter(x => x.forside_fejl !== null);
    if (!f.spaend.length && fejl && fejl.n > 0 && fs.length && !fs.some(x => x.forside_fejl)) ud.push(el('p', { className: 'vg-note drift-nogle', textContent: D.nogle }));
    return ud;
  }
  function fejlAfsnit(d, f) {
    const u = udsnit(), vaert = vaertHost() !== null, dele = [fejlFliser(d), ...linjer(d, f), fejlGraf(d)];
    const hentet = vaert ? Boolean(d.vaert && d.vaert.dage.some(x => x.hentet)) : d.dage.some(x => x.fejl !== null);
    const rk = raekker(f);
    if (rk.length) dele.push(...tabel(rk));
    else dele.push(hentet ? V.tomt(D.tom, 'drift-tom') : el('p', { className: 'vg-status dyk-tom drift-ikke-hentet', textContent: D.ikkeHentetP }));
    const fra = vaert ? d.vaert && d.vaert.fra : d.foerste_dag;
    if (fra && fra > u.fra) dele.push(el('p', { className: 'vg-note drift-fra', textContent: fmt(vaert ? D.subFra : D.fra, { dato: V.datoLang(fra) }) }));
    const afsk = vaert ? (d.vaert ? d.vaert.afskaaret.length : 0) : d.afskaaret.length;
    if (afsk) dele.push(el('p', { className: 'vg-note drift-afskaaret', textContent: fmt(D.afskaaret, { n: tal(afsk) }) }));
    if (d.afvist || (vaert && d.vaert && d.vaert.afvist_nu)) dele.push(el('p', { className: 'vg-advarsel vg-gul drift-afvist', textContent: D.afvist }));
    dele.push(el('p', { className: 'vg-note drift-note', textContent: D.note }), el('p', { className: 'vg-note drift-ser', textContent: D.ser }));
    const s = V.blok(D.fejlH, D.fejlP, ...dele);
    s.id = 'drift-fejl';
    return s;
  }

  /* ── "Subdomæner med fejl": kun for ét domæne uden filter og med fejl på mindst to værter. Navnet sætter filteret; kategorierne er ikke navne. ── */
  const KAT = { '[ip]': 'ip', '[anden]': 'anden', '[ugyldig]': 'ugyldig' };
  function subAfsnit(f) {
    const rigtige = f.vaerter.filter(v => !v.vaert.startsWith('['));
    if (samlet() || vaertHost() !== null || rigtige.length < 2) return null;
    const rk = f.vaerter.map(v => {
      let celle;
      if (v.vaert.startsWith('[')) celle = el('th', { textContent: v.vaert === '[andre]' ? D.subAndre : U[KAT[v.vaert]] || U.ugyldig });
      else { const navn = V.subHost(v.vaert, domaene), label = fmt(D.subAria, { vaert: navn }), b = V.knap(V.domaene(navn), gaa(medVaert(navn)), 'link dyk-celle sub-navn', label); b.title = label; celle = el('th', {}, b); }
      celle.setAttribute('scope', 'row');
      const tr = el('tr', { className: v.vaert.startsWith('[') ? 'sub-kat' : 'sub-raekke' }, celle, el('td', { className: 'tal-kol', textContent: tal(v.svar) }), el('td', { className: 'tal-kol', textContent: tal(v.kunne_ikke) }));
      if (!v.vaert.startsWith('[')) tr.dataset.vaert = V.subHost(v.vaert, domaene);
      return tr;
    });
    const s = V.blok(D.subH, D.subP, V.tabelRul(V.laeseTabel('sub-tabel drift-sub-tabel', [th(U.th), th(D.thFejl, 'tal-kol'), th(D.kunneIkke, 'tal-kol')], rk), D.subH), el('p', { className: 'vg-note sub-dns', textContent: U.dnsOnly }));
    s.id = 'drift-sub';
    return s;
  }

  /* ── cache: altid hele domænet. ── */
  const STATUS = ['hit', 'miss', 'expired', 'stale', 'bypass', 'revalidated', 'updating', 'dynamic', 'none'];
  const statusNoegle = s => (s === 'unknown' ? 'none' : STATUS.includes(s) ? s : null);
  function cacheAfsnit(d) {
    const dele = [];
    if (vaertHost() !== null) dele.push(el('p', { className: 'vg-note drift-hele', textContent: D.heleDomaenet }));
    else if (!d.cache) dele.push(el('p', { className: 'vg-status dyk-tom drift-cache-ikke', textContent: D.ikkeHentetP }));
    else {
      const c = d.cache, g = c.svar, i_alt = g.lager + g.selv + g.server + g.andet, dataIAlt = c.data.lager + c.data.selv + c.data.server + c.data.andet;
      if (!i_alt) dele.push(V.tomt(D.cacheTom, 'drift-cache-tom'));
      else {
        const [pL, pS, pH, pA] = andele([g.lager, g.selv, g.server, g.andet]), valgt = cacheValg();
        const f = (vaerdi, tekst, under, noegle, n) => flise({ vaerdi, tekst, under, klasse: 'drift-c-' + noegle, maal: n ? gaa(medValg('cache', noegle)) : null, aktiv: valgt === noegle });
        dele.push(el('div', { className: 'sfliser vg-fliser drift-fliser drift-cache-fliser' },
          f(pct(pL), D.lager, fmt(D.afForesp, { n: tal(g.lager), i: tal(i_alt) }), 'lager', g.lager),
          f(data(c.data.lager), D.slap, fmt(D.afData, { p: procent(c.data.lager, dataIAlt) }), 'data', c.data.lager),
          f(tal(g.selv), D.selv, fmt(D.andel, { p: pct(pS) }), 'selv', g.selv),
          f(tal(g.server), D.server, fmt(D.andel, { p: pct(pH) }), 'server', g.server)));
        const sumLinje = [[D.lager, pL], [D.selv, pS], [D.server, pH], ...(g.andet ? [[D.andet, pA]] : [])].map(([navn, p]) => fmt(D.sum, { navn, p: pct(p) })).join(' · ');
        dele.push(el('p', { className: 'vg-note drift-cache-sum', textContent: sumLinje }));
        if (!dagen()) {
          const raa = d.dage.map(x => ({ dag: x.dag, lager: x.cache ? x.cache.svar.lager : 0, server: x.cache ? x.cache.svar.server : 0, hentet: x.cache !== null }));
          const pr_uge = raa.length > UGE_FRA, h = pr_uge ? D.cacheGrafUgeH : D.cacheGrafH, ug = pr_uge ? uger(raa, ['lager', 'server']) : null, pr = new Map((ug || []).map(u => [u.dag, u]));
          const tekst = x => fmt(pr_uge ? D.cacheUge : D.cacheDag, { d: pr_uge ? V.interval(pr.get(x.dag).fra, pr.get(x.dag).til) : V.datoLang(x.dag), a: tal(x.lager), b: tal(x.server) });
          const maal = dag => gaa(pr_uge ? medInterval(pr.get(dag).fra, pr.get(dag).til) : medDag(dag));
          const legende = el('ul', { className: 'vg-legende vg-legende-inde' }, el('li', {}, el('i', { className: 'drift-leg-lager' }), D.lager), el('li', {}, el('i', { className: 'drift-leg-server' }), D.server));
          dele.push(el('div', { className: 'drift-graf', id: 'drift-cache-graf' }, el('h3', { className: 'drift-graf-h', textContent: h }), el('p', { className: 'vg-intro', textContent: pr_uge ? D.cacheGrafUgeP : D.cacheGrafP }), legende,
            V.soejlerSerier(ug || raa, [{ felt: 'lager', klasse: 'vg-soejle drift-lager' }, { felt: 'server', klasse: 'vg-soejle drift-server' }], { titel: `${h}. ${T.tastatur}`, tekst, maal })));
        }
        /* Tabellen: de ni dokumenterede værdier i klart sprog; alt andet lægges sammen som "Andet". */
        const i = valgt, med = x => (!i ? true : i === 'data' ? x.gruppe_data === 'lager' : x.gruppe === i), rk = new Map();
        for (const x of c.status.filter(med)) { const k = statusNoegle(x.status) || 'andet', a = rk.get(k) || rk.set(k, { svar: 0, bytes: 0 }).get(k); a.svar += x.svar; a.bytes += x.bytes; }
        const tr = [...rk.entries()].sort((a, b) => b[1].svar - a[1].svar || (a[0] < b[0] ? -1 : 1)).map(([k, a]) => {
          const navn = el('th', { textContent: k === 'andet' ? D.andet : D.status[k] });
          navn.setAttribute('scope', 'row');
          const e = el('tr', { className: 'drift-cache-raekke' }, navn, el('td', { className: 'tal-kol', textContent: tal(a.svar) }), el('td', { className: 'tal-kol', textContent: data(a.bytes) }));
          e.dataset.status = k;
          return e;
        });
        dele.push(V.tabelRul(V.laeseTabel('drift-cache-tabel', [th(D.thStatus), th(D.thForesp, 'tal-kol'), th(D.thData, 'tal-kol')], tr), D.cacheTabelH));
        if (c.stale > 0) dele.push(el('p', { className: 'vg-advarsel vg-gul drift-gammel', textContent: fmt(D.gammel, { n: tal(c.stale) }) }));
      }
      dele.push(el('p', { className: 'vg-note drift-cache-note', textContent: D.cacheNote }), el('p', { className: 'vg-note drift-dynamisk', textContent: D.dynamisk }), el('p', { className: 'vg-note drift-data-note', textContent: D.dataNote }));
    }
    const s = V.blok(D.cacheH, vaertHost() !== null ? '' : D.cacheP, ...dele);
    s.id = 'drift-cache';
    return s;
  }

  function tegn(d, f) {
    $('drift-under').textContent = domaene;
    const pv = params.get('periode'), u = udsnit(), p = periode();
    R.baandValg('drift-periode', [[T.dykP7, medPeriode(), !interval() && !pv], [T.dykP30, medPeriode('30'), !interval() && pv === '30'], [B.aar, medPeriode('aar'), !interval() && pv === 'aar']], 'drift');
    const vaelger = el('div', { className: 'besoeg-valg' }, V.datoVaelger({ visning: 'drift', plads: 'drift-datoer', tom: !interval(), periode: { fra: p.fra, til: p.til, i_dag: iDag(), data_fra: '' }, gaa: (a, b) => R.gaaTil('drift', medInterval(a, b)) }));
    /* Datovælgerens note siger, at tidspunktet på dagen ikke gemmes. Her gemmes timerne, så noten er fanens egen. */
    const note = vaelger.querySelector('.dato-vaelger > .vg-note');
    if (note) note.textContent = D.datoNote;
    const valgt = el('h2', { className: 'besoeg-valgt-h drift-valgt-h', textContent: fmt(B.valgtPeriode, { p: V.interval(u.fra, u.til) }) });
    const vaert = vaertHost() !== null, genaabn = V.husk(rod), dele = [vaelger, valgt];
    /* Hver chip fjerner sit eget filter (×). */
    const chips = [];
    if (interval()) chips.push(V.fjernChip(periodeNavn(), gaa(medPeriode()), 'drift-interval-chip'));
    if (dagen()) chips.push(V.fjernChip(V.datoLang(dagen()), gaa(uden('dag', 'time')), 'drift-dag-chip'));
    if (timen()) chips.push(V.fjernChip(fmt(D.chipTime, { a: kl(`${dagen()}T${timen()}`), b: kl(`${dagen()}T${timen()}`, 1) }), gaa(uden('time')), 'drift-time-chip'));
    if (vaert) chips.push(V.fjernChip(fmt(U.filter, { vaert: vaertHost() }), gaa(uden('vaert')), 'drift-vaert-chip'));
    const visNavn = { naa: D.kunneIkke, server: D.serverFejl, timer: D.timer, forside: D.forside }, cacheNavn = { lager: D.lager, data: D.slap, selv: D.selv, server: D.server };
    if (vis() && !(vaert && vis() !== 'naa')) chips.push(V.fjernChip(fmt(D.chipKun, { navn: visNavn[vis()] }), gaa(uden('vis')), 'drift-vis-chip'));
    if (cacheValg() && !vaert) chips.push(V.fjernChip(fmt(D.chipKun, { navn: cacheNavn[cacheValg()] }), gaa(uden('cache')), 'drift-cache-chip'));
    if (chips.length) dele.push(el('div', { className: 'login-stier besoeg-chips drift-chips' }, ...chips));
    if (vaert) dele.push(V.subUden(D.udenSub, domaene, gaa(uden('vaert')), 'drift-sub-uden'));
    dele.push(fejlAfsnit(d, f), ...[subAfsnit(f)].filter(Boolean), cacheAfsnit(d));
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
    for (const fn of afmeld.splice(0)) fn();
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
    /* igen: køen meldte nye tal, så fokus og rullepositionen bliver, hvor de er. */
    let vist = null;
    async function hent(igen) {
      const u = udsnit(), arg = { zone: samlet() ? null : p.get('zone') || '', fra: u.fra, til: u.til, vaert: vaertN() };
      let d, f;
      try { [d, f] = await Promise.all([invoke('drift', arg), invoke('drift_fejl', arg)]); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
      if (g !== gen) return;
      const gendan = igen ? V.huskFokus(rod) : null;
      vist = [d, f];
      tegn(d, f);
      if (gendan) gendan();
    }
    /* Domænernes navne kommer fra Overblikkets rækker: når de er hentet (eller ændret), tegnes tabellen igen med de samme tal. */
    const navneIgen = () => { if (g !== gen || !vist) return; const gendan = V.huskFokus(rod); tegn(...vist); gendan(); };
    await hent();
    if (g !== gen) return;
    if (!R.genbesoeg()) $('drift-h').focus();
    let sidstHentet = Date.now(), ventende = null;
    const snart = () => {
      if (ventende) return;
      ventende = setTimeout(() => { ventende = null; sidstHentet = Date.now(); hent(true); }, Math.max(0, sidstHentet + 3000 - Date.now()));
    };
    afmeld.push(R.paa('hentekoe', snart), R.paa('ekstra', snart), R.paa('raekker', navneIgen), () => clearTimeout(ventende));
  }
  R.registrer('drift', {
    domaener: true,
    hvad: () => ({ samlet: samlet(), zone: params.get('zone') || '', navn: domaene }),
    /* Et andet domæne: samme periode og dag, men hverken time, filtre eller filteret pr. subdomæne (det hører til domænet). */
    skift(q) {
      const ny = new URLSearchParams(q);
      for (const k of ['periode', 'fra', 'til', 'dag']) if (params.has(k)) ny.set(k, params.get(k));
      R.gaaTil('drift', ny);
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
