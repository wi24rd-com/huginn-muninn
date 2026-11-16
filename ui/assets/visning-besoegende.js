/* Huginn and Muninn — Besøgende: hvor mange der besøger sitet, hvorfra, hvilke robotter og hvilke sider. #besoegende?zone=<id>&navn=… eller #besoegende?samlet=1,
   perioden periode=30|aar (ingen = 7 hele dage) eller fra=…&til=… (højst et år), og højst ét filter: land=XX (besøg fra landet pr. dag), sti=/… (visninger af
   siden pr. dag) eller robot=<gruppe> (besøg fra robotgruppen pr. dag). Tallene regnes i Rust (besoegende, kun fra cachen): unikke besøgende er årstallene
   (unikke IP-adresser pr. dag), besøg, kendte robotter og sider er Cloudflares estimat ud fra en stikprøve og findes en måned tilbage plus det, programmet har
   gemt. Perioder over 90 dage vises pr. uge (mandag–søndag, UTC); et klik på en uge viser ugen. Et klik på en dag åbner "Trafik den …" på Angreb, et klik på et
   land, en robotgruppe eller en sti lægger det som filter her (chip med ×), og fliserne sætter perioden. Fliserne er faste og står øverst, over vælgeren;
   under vælgeren står "Valgt periode: …" over alt, der følger perioden. Stier, der svarede med sidens almindelige side, står i en egen liste. Dage uden tal er huller, ikke 0. Intet uden data bag er
   klikbart. Søjlerne er blå (--info-mark), så de ikke læses som angreb. Tegnes igen, når køen eller de ekstra tal melder nyt, højst hvert 3. sekund; svar,
   der kommer for sent, kasseres (gen). Bygges med createElement.
   V-SUBDOM.1 B2: for ét domæne står afsnittet "Subdomæner" (kommandoen subdomaener, kun fra cachen), når domænet har tal på mindst to værter i perioden, og
   kun uden filter. vaert=<navn> er filteret pr. subdomæne: fliser og grafer for besøg og forespørgsler pr. dag, lande, kendte robotter og Mest besøgte sider
   for værten (V-SUBDOM.1 FIX 1: lande og robotter pr. vært fra Rust). Unikke besøgende og IP-klasserne findes ikke pr. subdomæne: de står ikke, og én linje
   under chippen siger det og fører til hele domænet. Et afsnit uden tal for værten i perioden står ikke. Det kan stå sammen med sti, land eller robot
   (et klik på et land eller en robotgruppe beholder værten) og nulstilles, når et andet domæne vælges. Værtsnavnene sættes kun som tekst.
   V-SUBDOM.1 FIX 2: et tal pr. subdomæne, der ikke er hentet endnu, er "–", aldrig 0, og en linje (role=status) siger, hvad køen stadig henter. Når køen
   melder nye tal, tegnes siden igen med fokus og rulleposition, hvor de var. */
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
  const U = T.sub;
  /* Filteret pr. subdomæne: navnet fra adressen (domænet selv eller et navn under det), og labelen, Rust bruger (V.subNoegle). Kun for ét domæne. */
  const vaertHost = () => (samlet() || V.subNoegle(params.get('vaert'), domaene) === null ? null : params.get('vaert').toLowerCase());
  const vaertN = () => (vaertHost() === null ? null : V.subNoegle(vaertHost(), domaene));
  const periodeArg = () => (interval() ? { spaend: { fra: params.get('fra'), til: params.get('til') } } : { 30: 'd30', aar: 'alt' }[params.get('periode')] || 'd7');
  const omfangQ = () => (samlet() ? { samlet: '1' } : { zone: params.get('zone') || '', navn: domaene });
  /* Adresserne er forespørgsler; gaa() gør dem til et mål i visningen. */
  const kopi = fn => { const p = new URLSearchParams(params); fn(p); return p.toString(); };
  const gaa = q => ({ gaa: 'besoegende?' + q });
  const medPeriode = v => kopi(p => { ['periode', 'fra', 'til'].forEach(k => p.delete(k)); if (v) p.set('periode', v); });
  const medInterval = (a, b) => kopi(p => { ['periode', 'fra', 'til'].forEach(k => p.delete(k)); p.set('fra', a); p.set('til', b); });
  /* Højst ét filter: tallene kommer fra hvert sit kald hos Cloudflare og kan ikke deles på hinanden. */
  /* Et land eller en robotgruppe beholder filteret pr. subdomæne (FIX 1: tallene er værtens). */
  const medFilter = (navn, v) => kopi(p => { FILTRE.forEach(k => p.delete(k)); if (vaertHost() === null) p.delete('vaert'); if (v) p.set(navn, v); });
  const udenFilter = k => kopi(p => p.delete(k));
  const medVaert = host => kopi(p => { FILTRE.forEach(k => p.delete(k)); p.set('vaert', host); });
  /* En sti åbner visningerne pr. dag. Står siderne pr. vært (V-SUBDOM.1), åbner rækken stien på sin vært, så tallene passer til rækken. */
  const stiMaal = (x, medVaerter) => kopi(p => { FILTRE.forEach(k => p.delete(k)); p.delete('vaert'); p.set('sti', x.sti); const h = vaertHost() ?? (medVaerter ? V.subHost(x.vaert || '', domaene) : null); if (h) p.set('vaert', h); });
  const dagFoer = (dag, n) => new Date(Date.parse(`${dag}T00:00:00Z`) - n * 864e5).toISOString().slice(0, 10);
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
  /* Filteret pr. subdomæne: unikke besøgende findes ikke pr. vært og står ikke (aldrig domænets tal under et subdomænes navn; linjen under chippen siger det).
     Besøg og forespørgsler på værten de seneste 30 dage er kommandoen subdomaener for de dage (sub30); med en sti eller et land er det besøgene på stien eller
     fra landet på værten (Rust). */
  function fliserVaert(d, sub30) {
    const boks = el('div', { className: 'sfliser vg-fliser besoeg-fliser' }), host = vaertHost();
    if (sti()) boks.append(flise(d.besoeg30, fmt(B.fBesoegSti, { sti: sti() }), medPeriode('30'), d.side_ikke.length ? B.ikkeTilgKort : B.ingenTal));
    else if (land()) boks.append(flise(d.besoeg30, fmt(B.fBesoegLand, { land: V.land(land()) }), medPeriode('30')));
    else {
      const v = sub30 && sub30.vaerter.find(x => x.vaert === vaertN());
      const sum = k => (v ? { n: v[k], med: v.dage.filter(x => x[k] !== null).length, dage: v.dage.length } : null);
      boks.append(flise(sum('besoeg'), fmt(U.fBesoeg, { vaert: host }), medPeriode('30')), flise(sum('foresp'), fmt(U.fForesp, { vaert: host }), medPeriode('30')));
    }
    const s = el('section', { className: 'besoeg-faste' }, el('h2', { className: 'besoeg-faste-h', textContent: B.fasteH }), boks);
    s.setAttribute('aria-label', B.fasteH);
    return s;
  }
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
  /* Med et filter pr. subdomæne og et land eller en robotgruppe viser et klik på en dag dagen her med samme filtre (som værtens egne grafer). */
  function graf(d) {
    const [hDag, hUge] = filterH(), her = vaertHost() !== null && (land() || robot());
    return grafBlok(d.dage, d.uger, d.periode, hDag, hUge, her ? U.grafP : B.grafP, her ? (dag => gaa(medInterval(dag, dag))) : dagQ, 'besoeg-graf');
  }
  /* Grafen pr. dag, eller pr. uge for perioder over 90 dage (uger fra Rust, eller regnet her for et subdomæne ud fra de samme uger). */
  function grafBlok(dageL, ugerL, periode, hDag, hUge, pDag, dykDag, id) {
    let h, soejler;
    const d = { dage: dageL, uger: ugerL };
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
      soejler = V.soejler(d.dage.map(x => ({ dag: x.dag, n: x.n || 0, hentet: x.n !== null })), { titel: h, dyk: dykDag, klasse: 'besoeg-soejle' });
    }
    const dele = [soejler];
    if (periode.med < periode.dage) dele.push(el('p', { className: 'vg-note hul-note', textContent: fmt(B.hulNote, { m: tal(periode.med), n: tal(periode.dage) }) }));
    const s = V.blok(h, d.uger ? B.grafUgeP : pDag, ...dele);
    s.id = id;
    return s;
  }
  /* Filteret pr. subdomæne uden sti: besøg og forespørgsler pr. dag for værten (subdomaener). En dag uden tal er et hul, ikke 0; et klik på en dag viser dagen
     her, med samme filter. */
  function grafVaert(d, sub) {
    const host = vaertHost(), v = sub && sub.vaerter.find(x => x.vaert === vaertN()), pr = new Map(v ? v.dage.map(x => [x.dag, x]) : []);
    const serie = k => d.dage.map(x => ({ dag: x.dag, n: pr.has(x.dag) ? pr.get(x.dag)[k] : null }));
    const uger = s => (d.uger ? d.uger.map(u => { const i = s.filter(x => x.dag >= u.fra && x.dag <= u.til), med = i.filter(x => x.n !== null).length; return { dag: u.dag, fra: u.fra, til: u.til, n: med ? i.reduce((a, x) => a + (x.n || 0), 0) : null, med }; }) : null);
    const dyk = dag => gaa(medInterval(dag, dag));
    return [['besoeg', U.grafBesoegH, U.grafBesoegUgeH, 'besoeg-graf'], ['foresp', U.grafForespH, U.grafForespUgeH, 'besoeg-graf-foresp']].map(([k, hd, hu, id]) => {
      const s = serie(k), med = s.filter(x => x.n !== null).length;
      return grafBlok(s, uger(s), { med, dage: s.length }, fmt(hd, { vaert: host }), fmt(hu, { vaert: host }), U.grafP, dyk, id);
    });
  }
  const th = (t, k) => { const e = el('th', { textContent: t, className: k || '' }); e.setAttribute('scope', 'col'); return e; };
  /* En tabel: første kolonne er et link (filter), resten er tal uden data bag. */
  function tabel(klasse, hoved, rk) {
    const raekker = rk.map(x => {
      const c = el('th', {}, V.knap(x.navn, x.maal, 'link dyk-celle' + (x.kode ? ' besoeg-sti' : ''), x.label));
      c.setAttribute('scope', 'row');
      return el('tr', {}, c, ...x.tal.map(t => el('td', { className: 'tal-kol', textContent: t })));
    });
    return V.laeseTabel(klasse, [th(hoved[0]), ...hoved.slice(1).map(t => th(t, 'tal-kol'))], raekker);
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
  /* Med et filter pr. subdomæne er tallene værtens (Rust), og afsnittet står kun, når værten har tal i perioden. */
  function lande(d) {
    if (vaertHost() !== null && !d.lande.length) return null;
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
    if (vaertHost() !== null && !d.robotter.length) return null;
    const dele = [...ikkeLinje(d.robot_ikke, fmt(B.robotIkkeTilg, { d: domaene }), B.robotIkkeTilg, 'besoeg-robot-ikke')];
    if (d.ipklasse && d.ipklasse.i_alt && vaertHost() === null) {
      const tekst = fmt(B.klasseLinje, { p: procent(d.ipklasse.n, d.ipklasse.i_alt) }) + (d.ipklasse.med < 365 ? ' ' + fmt(B.klasseDage, { m: tal(d.ipklasse.med) }) : '');
      dele.push(el('p', { className: 'besoeg-klasse', textContent: tekst }));
    }
    if (d.robotter.length) {
      const i_alt = d.robotter.reduce((a, x) => a + x.n, 0);
      const rk = d.robotter.map(x => { const navn = gruppeNavn(x.id); return { navn, maal: gaa(medFilter('robot', x.id)), label: fmt(B.gruppeAabn, { navn, n: tal(x.n) }), tal: [tal(x.n), procent(x.n, i_alt)] }; });
      dele.push(el('div', { className: 'vg-tabel-rul' }, tabel('besoeg-robotter', [B.thGruppe, B.thBesoeg, B.thAndel], rk)));
    } else if (!d.robot_ikke.length || d.robot_fra) dele.push(el('p', { className: 'vg-status dyk-tom', textContent: d.robot_fra ? B.robotIngen : B.robotIkkeHentet }));
    /* Med et filter pr. subdomæne står IP-klasserne ikke, så noten nævner dem ikke (FIX 2). */
    dele.push(el('p', { className: 'vg-note besoeg-robot-note', textContent: vaertHost() !== null ? B.robotNoteVaert : B.robotNote }));
    const s = V.blok(B.robotH, B.robotP, ...dele);
    s.id = 'besoeg-robotter';
    return s;
  }
  function sider(d, sub) {
    const dele = [...ikkeLinje(d.side_ikke, fmt(B.sideIkkeTilg, { d: domaene }), B.sideIkkeTilg, 'besoeg-side-ikke')];
    const forside = d.sider_forside || [];
    /* V-SUBDOM.1: samme sti på to subdomæner er to rækker; rækken uden for domænet selv har subdomænet ved stien. Med et filter pr. subdomæne er alle
       rækker værtens, så navnet står kun i chippen. */
    const medVaerter = !samlet() && [...d.sider, ...d.sider_404, ...forside].some(x => x.vaert);
    const ved = x => (medVaerter && vaertHost() === null && x.vaert ? V.subHost(x.vaert, domaene) : null);
    const navn = x => { const h = ved(x); return h ? el('span', {}, x.sti, el('small', { className: 'besoeg-vaert', textContent: h })) : x.sti; };
    const label = x => { const h = ved(x); return h ? fmt(U.stiVaert, { sti: x.sti, vaert: h, n: tal(x.n) }) : fmt(B.stiAabn, { sti: x.sti, n: tal(x.n) }); };
    const raekke = x => ({ navn: navn(x), kode: true, maal: gaa(stiMaal(x, medVaerter)), label: label(x), tal: [tal(x.n), tal(x.besoeg), procent(x.n, d.sider_i_alt)] });
    const stiRk = x => ({ navn: navn(x), kode: true, maal: gaa(stiMaal(x, medVaerter)), label: label(x), tal: [tal(x.n)] });
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
    if (sub && sub.sider_afskaaret) dele.push(el('p', { className: 'vg-note besoeg-side-afskaaret', textContent: U.afskaaret }));
    dele.push(el('p', { className: 'vg-note besoeg-side-note', textContent: B.sideNote }));
    const s = V.blok(B.sideH, B.sideP, ...dele);
    s.id = 'besoeg-sider';
    return s;
  }
  function tegn(d, sub, sub30) {
    $('besoegende-under').textContent = domaene;
    const p = periodeArg();
    /* Øverst de faste fliser, så vælgeren (7/30/Et år og Fra/Til), og under den en overskrift for alt, der følger perioden (V-BESOEG.2 FIX 1). */
    R.baandValg('besoegende-periode', [[T.dykP7, medPeriode(), !interval() && p === 'd7'], [T.dykP30, medPeriode('30'), !interval() && p === 'd30'], [B.aar, medPeriode('aar'), !interval() && p === 'alt']], 'besoegende');
    const vaelger = el('div', { className: 'besoeg-valg' }, V.datoVaelger({ visning: 'besoegende', plads: 'besoegende-datoer', tom: !interval(), periode: d.periode, gaa: (a, b) => R.gaaTil('besoegende', medInterval(a, b)) }));
    const valgt = el('h2', { className: 'besoeg-valgt-h', textContent: fmt(B.valgtPeriode, { p: V.interval(d.periode.fra, d.periode.til) }) });
    const vaert = vaertHost() !== null, genaabn = V.husk(rod), dele = [vaert ? fliserVaert(d, sub30) : fliser(d), vaelger, valgt];
    /* Hver chip fjerner sit eget filter (×). */
    const chips = [], chip = (n, klasse, k) => chips.push(V.fjernChip(n, gaa(udenFilter(k)), klasse));
    if (interval()) { const n = periodeNavn(); chips.push(V.fjernChip(n, gaa(medPeriode()))); }
    if (vaert) chip(fmt(U.filter, { vaert: vaertHost() }), 'besoeg-vaert-chip', 'vaert');
    if (land()) chip(V.land(land()), 'besoeg-land-chip', 'land');
    if (sti()) chip(sti(), 'besoeg-sti-chip', 'sti');
    if (robot()) chip(gruppeNavn(robot()), 'besoeg-robot-chip', 'robot');
    if (chips.length) dele.push(el('div', { className: 'login-stier besoeg-chips' }, ...chips));
    /* Én linje under chippen: det, der ikke findes pr. subdomæne, og vejen til hele domænet (samme mål som chippens ×). */
    if (vaert) dele.push(V.subUden(U.udenBes, domaene, gaa(udenFilter('vaert')), 'besoeg-sub-uden'));
    /* FIX 2 (F3): trafikken pr. subdomæne, køen stadig henter for perioden. Fliserne og graferne viser "–" og huller, aldrig 0. */
    const henter = vaert ? V.subHenter(sub, ['trafik']) : null;
    if (henter) dele.push(henter);
    dele.push(...(vaert && !sti() && !land() && !robot() ? grafVaert(d, sub) : [graf(d)]));
    /* Subdomæner: kun for ét domæne, uden filter, og kun med tal på mindst to værter (eller når Cloudflare afviste forespørgslen). Navnet åbner Besøgende
       for subdomænet, Besøg og Forespørgsler også, Angreb Angreb (samme periode). */
    if (!samlet() && !vaert && !land() && !sti() && !robot() && V.subVis(sub, sub && sub.trafik_afvist)) {
      const zq = { zone: params.get('zone') || '', navn: domaene };
      const angQ = n => new URLSearchParams({ ...zq, ...V.subPeriodeAng(params), vaert: n }).toString();
      dele.push(V.subAfsnit(sub, { domaene, afvist: sub.trafik_afvist, intro: U.pBes, aria: U.ariaBes, maal: n => gaa(medVaert(n)), maalBes: n => gaa(medVaert(n)), maalAng: angQ, fra: d.periode.fra, id: 'besoeg-sub' }));
    }
    dele.push(...[lande(d), robotter(d)].filter(Boolean), sider(d, sub));
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
    /* igen: køen meldte nye tal, så fokus og rullepositionen bliver, hvor de er (FIX 2). */
    async function hent(igen) {
      let d;
      try { d = await invoke('besoegende', { omfang, periode: periodeArg(), land: land(), sti: sti(), robot: robot(), vaert: vaertN() }); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
      if (g !== gen) return;
      /* V-SUBDOM.1: tallene pr. subdomæne for perioden (og de seneste 30 dage til fliserne med et filter pr. subdomæne). Et afslag her rører intet andet. */
      let sub = null, sub30 = null;
      if (!samlet()) {
        const spm = (fra, til) => invoke('subdomaener', { zone: omfang.zone, ...V.subSpaend(fra, til) }).catch(() => null);
        [sub, sub30] = await Promise.all([spm(d.periode.fra, d.periode.til), vaertHost() !== null && !sti() && !land() ? spm(dagFoer(d.i_gaar_dag, 29), d.i_gaar_dag) : null]);
      }
      if (g !== gen) return;
      const gendan = igen ? V.huskFokus(rod) : null;
      tegn(d, sub, sub30);
      if (gendan) gendan();
    }
    await hent();
    if (g !== gen) return;
    if (!R.genbesoeg()) $('besoegende-h').focus();
    let sidstHentet = Date.now(), ventende = null;
    const snart = () => {
      if (ventende) return;
      ventende = setTimeout(() => { ventende = null; sidstHentet = Date.now(); hent(true); }, Math.max(0, sidstHentet + 3000 - Date.now()));
    };
    afmeld.push(R.paa('hentekoe', snart), R.paa('ekstra', snart), () => clearTimeout(ventende));
  }
  R.registrer('besoegende', {
    domaener: true,
    hvad: () => ({ samlet: samlet(), zone: params.get('zone') || '', navn: domaene }),
    /* Et andet domæne: samme periode og samme filter, men ikke filteret pr. subdomæne (det hører til domænet). */
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
