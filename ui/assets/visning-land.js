/* Huginn and Muninn — <Land> lige nu og Verden: dine egne angreb ved siden af Cloudflare Radar. #land viser det gemte land, og det er altid dér,
   sidebjælken, ⌘6 og programmenuen fører hen. #land?kode=XX er et kig: en anden visning har peget på et land. Det gemmes ikke, og overskriften får
   "Tilbage til <gemt land>". Desuden fane=verden, afsnit=maal|kilde|udfald, periode=7 (ellers 4 uger i sammenligningen) og produkt=WAF (verden er kun WAF). Landet vælges første gang fra tidszonen
   (tidszoner.json), derefter fra Intl.Locale(navigator.language).region, for sproget siger ikke, hvor man er; ellers vælger brugeren selv, og en
   ukendt kode er aldrig en fejl. Vælgeren i båndet og Indstillinger gemmer, og efter et skift i Indstillinger står "Tilbage til", indtil man forlader
   siden. Forslaget kommer fra Rust og kun, når det er tydeligt (radar::forslag). Tallene regnes i Rust (land_visning, kun fra cachen); radar_opdater
   henter det, der er over 1 time gammelt, og hændelsen 'radar' tegner igen. Angreb vises fra målsiden (radar.rs): de største kilder til angreb mod landet, som andel af angrebene mod det,
   ved siden af hvor angreb mod dine domæner kom fra i samme periode (7 dage eller 4 uger: Radar kender ikke 30 dage her), og landets andel som mål, tegnet som en målt andel og aldrig som et antal. Landene i landets
   kolonne kan ikke klikkes: der er ingen data bag dem hos os. Små tal: kurven tegnes, men der står kun noget i ord, når Rust sender ord (gul-reglen).
   Bygges med createElement. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, R = window.hmRamme, V = window.hmTegn;
  const $ = id => document.getElementById(id);
  const { el, fmt, invoke } = R, tal = S.tal;
  const loc = S.lang === 'da' ? 'da-DK' : 'en-GB';
  const rod = $('land-rod'), vaelger = R.landVaelger, indst = $('indst-land'), forslagKnap = $('indst-land-forslag'), tilbageKnap = $('indst-land-tilbage');
  const pct = v => Number(v).toLocaleString(loc, { maximumFractionDigits: v < 1 ? 2 : 1 }) + (S.lang === 'da' ? ' %' : '%');
  const indeks = v => (v === null || v === undefined ? '–' : tal(Math.round(v)));

  /* koder: alle lande, kortdata kender (til navne og links). medTal: kun dem, Radar har haft tal for som mål de seneste 30 dage (Rust: land_hent.med_tal);
     kun dem står i vælgerne og forslaget, for for et land uden tal er siden den samme som for alle andre (V-BESOEG.2). */
  let koder = [], medTal = [], gemt = null, forslag = null, params = new URLSearchParams(), kode = null, tilbage = null;
  const kendt = k => koder.includes(k);
  const verdenFane = () => params.get('fane') === 'verden';
  const waf = () => params.get('produkt') === 'WAF';
  const ligeNu = k => fmt(T.landLigeNu, { land: V.land(k) });
  /* Første gang: tidszonen siger, hvor brugeren er. En Mac på engelsk (US) i Danmark har sproget en-US og tidszonen Europe/Copenhagen. */
  function fraTidszone() {
    try { const k = V.tidszoneLand(new Intl.DateTimeFormat().resolvedOptions().timeZone); return kendt(k) ? k : null; } catch (e) { return null; }
  }
  /* es-419 giver et tal, og "da" giver ingen region. */
  function fraSprog() {
    try { const r = new Intl.Locale(navigator.language).region; return kendt(r) ? r : null; } catch (e) { return null; }
  }
  async function klar() {
    await V.klar();
    koder = V.landeKoder().sort((a, b) => V.land(a).localeCompare(V.land(b), loc));
    const d = await invoke('land_hent');
    medTal = koder.filter(k => (d.med_tal || []).includes(k));
    gemt = kendt(d.land) ? d.land : null;
    forslag = medTal.includes(d.forslag) ? d.forslag : null;
    const foerste = gemt ? null : fraTidszone() || fraSprog();
    if (foerste) await gem(foerste);
    tegnValg();
  }
  async function gem(k) {
    await invoke('land_saet', { kode: k });
    gemt = k;
    tegnValg();
  }
  /* Vælgerne og sidebjælkens navn. */
  function fyld(s, valgt) {
    if (s.options.length !== medTal.length + 1 || [...s.options].some((o, i) => i && o.value !== medTal[i - 1])) s.replaceChildren(el('option', { value: '', textContent: T.landUkendt }), ...medTal.map(k => el('option', { value: k, textContent: V.land(k) })));
    s.value = valgt || '';
  }
  function tegnValg() {
    /* Landevælgeren i båndet (Kit.vaelger): de samme lande som i Indstillinger. Uden valgt land står "ukendt" øverst. */
    const valgt = kode || gemt;
    vaelger.saetPunkter([{ id: 'ukendt', tekst: T.landUkendt }, ...medTal.map(k => ({ id: k, tekst: V.land(k) }))], valgt || 'ukendt');
    fyld(indst, gemt);
    $('nav-land').dataset.t = gemt ? '' : 'navLand';
    $('nav-land').textContent = gemt ? ligeNu(gemt) : T.navLand;
    // et langt landenavn afkorter kittet med … og viser det fulde navn i knappens tip (fra 2.7.0)
    window.hmVelkomst.land(gemt ? V.land(gemt) : null);
    $('indst-land-note').textContent = gemt ? fmt(T.landIndstNote, { land: V.land(gemt) }) : T.landVaelgP;
    const vis = forslag && forslag !== gemt;
    forslagKnap.hidden = !vis;
    if (vis) forslagKnap.textContent = fmt(T.landForslag, { land: V.land(forslag) });
    tilbageKnap.hidden = !tilbage;
    if (tilbage) tilbageKnap.textContent = fmt(T.landTilbage, { land: V.land(tilbage) });
  }
  /* kode står kun i adressen, når den er en anden end det gemte land. */
  const adresse = (k, x) => { const p = new URLSearchParams(k === gemt ? {} : { kode: k }); for (const [n, v] of Object.entries(x || {})) p.set(n, v); return p.toString(); };
  const gaaLand = q => (q ? 'land?' + q : 'land');
  /* Efter et skift står siden på det gemte land, hvis den er åben. */
  async function skift(k, husk) {
    const foer = gemt;
    await gem(k);
    if (husk) { tilbage = foer !== k ? foer : null; tegnValg(); }
    if (R.tilstand.visning === 'land') { const q = new URLSearchParams(params); ['kode', 'afsnit'].forEach(n => q.delete(n)); R.gaaTil('land', q); }
  }
  const iIndst = k => skift(k, true).catch(e => R.status($('indst-status'), S.fejl(e), 'fejl'));
  forslagKnap.addEventListener('click', () => iIndst(forslag));
  tilbageKnap.addEventListener('click', () => { const k = tilbage; tilbage = null; skift(k, false).catch(e => R.status($('indst-status'), S.fejl(e), 'fejl')); });
  indst.addEventListener('change', () => { if (indst.value) iIndst(indst.value); });
  R.paa('landValg', k => { if (k && k !== 'ukendt') skift(k, false).catch(e => besked(S.fejl(e))); });

  /* ── Land ── */
  const dagTekst = (d, iGaar) => (d === iGaar ? T.dagIGaar : fmt(T.dagDen, { d: V.datoLang(d) }));
  /* Kilde og licens, og hvornår tallene er fra. */
  function kilde(d) {
    const k = V.radarKilde();
    if (d.hentet) k.append(' · ', fmt(T.landHentet, { t: S.kl(d.hentet) }));
    return k;
  }
  function dom(d) {
    if (!d.ord || !d.ord.tendens) return [];
    const foelger = d.ord.tendens === 'foelger', v = { n: tal(d.ord.angreb), d: V.datoLang(d.ord.dag), s: tal(d.ord.snit) };
    const hvad = el('button', { type: 'button', className: 'link', textContent: T.landTendensHvad });
    hvad.addEventListener('click', () => R.hjaelp(true, { titel: 'hjaelpH', indhold: el('div', {}, el('p', { textContent: S.bog('tendens').alle }), el('p', { textContent: S.bog('tendens').ja[0] })) }));
    return [el('div', { className: 'dom ' + (foelger ? 'groen' : 'gul') }, el('div', { className: 'dom-tekst' }, el('b', { className: 'dom-overskrift', textContent: foelger ? T.landFoelgerH : T.landDigH }), el('p', { textContent: fmt(foelger ? T.landFoelgerP : T.landDigP, v) })), el('div', { className: 'dom-handling' }, hvad))];
  }
  function kurveBlok(d) {
    const verden = waf() ? T.landVerdenWaf : T.landVerden;
    const tekst = x => {
      const v = { d: V.datoLang(x.dag), m: x.mit === null ? T.hulIngen : tal(x.mit), i: indeks(x.mit_indeks), v: indeks(x.verden_indeks) };
      return fmt(x.mit ? T.landDag : T.landDagUden, v);
    };
    /* Min kurve åbner dagen i dyk; en dag uden angreb har intet at åbne. */
    const maal = x => (x.mit ? new URLSearchParams({ samlet: '1', dag: x.dag, slags: 'angreb' }).toString() : null);
    const tegning = V.kurver(d.dage, [{ felt: 'verden_indeks', klasse: 'vg-k-verden' }, { felt: 'mit_indeks', klasse: 'vg-k-mit' }], { titel: T.landKurveTitel, tekst, maal });
    const legende = el('ul', { className: 'vg-legende vg-legende-inde' }, el('li', {}, el('i', { className: 'vg-kl vg-k-mit' }), T.landMit), el('li', {}, el('i', { className: 'vg-kl vg-k-verden' }), verden), el('li', { textContent: T.landAkse }));
    const dele = [...serieNote(d, 'l7', 'l7_waf'), el('div', { className: 'vg-tavle vg-tavle-pad' }, legende, el('div', { className: 'vg-soejler-rul' }, tegning))];
    if (waf()) dele.push(el('p', { className: 'vg-note' }, V.knap(T.landWafFjern, { gaa: gaaLand(uden('produkt')) })));
    return V.blok(T.landKurveH, T.landKurveP, ...dele);
  }
  /* En serie, der ikke kunne bruges, koster kun sit eget afsnit: en note her, ikke banneret øverst. */
  const serieNote = (d, ...serier) => (serier.some(s => d.serie_fejl.includes(s)) ? [el('p', { className: 'vg-note', textContent: T.landSerieFejl })] : []);
  const uden = k => { const p = new URLSearchParams(params); p.delete(k); return p.toString(); };
  function andelBlok(d) {
    const navn = V.land(d.kode), s = d.andel[0];
    const saetning = !s ? fmt(T.landAndelIngen, { land: navn }) : s.vaerdi === null ? fmt(T.landUdenfor, { land: navn, d: dagTekst(s.dag, d.i_gaar) }) : fmt(T.landAndel, { land: navn, p: pct(s.vaerdi), d: dagTekst(s.dag, d.i_gaar) });
    const blok = V.blok(fmt(T.landAndelH, { land: navn }), T.landAndelP, ...serieNote(d, 'maal'), el('p', { className: 'land-saetning', textContent: saetning }));
    blok.id = 'land-maal';
    if (d.andel.length > 1) {
      const liste = el('ol', { className: 'land-dage' }, ...d.andel.map(a => el('li', {}, el('span', { textContent: V.datoLang(a.dag) }), el('span', { textContent: a.vaerdi === null ? T.landUdenforKort : pct(a.vaerdi) }))));
      const mere = el('details', { className: 'dyk-flere' }, el('summary', { textContent: T.landAndelDage }), liste);
      mere.dataset.noegle = 'andel';
      blok.append(mere);
    }
    const dage = d.andel.slice().reverse();
    if (dage.filter(a => a.vaerdi !== null).length > 1) {
      const maks = Math.max(...dage.map(a => a.vaerdi || 0));
      const tekst = a => fmt(a.vaerdi === null ? T.landAndelDagUden : T.landAndelDag, { d: V.datoLang(a.dag), p: a.vaerdi === null ? '' : pct(a.vaerdi) });
      const kurve = V.kurver(dage, [{ felt: 'vaerdi', klasse: 'vg-k-mit' }], { titel: T.landAndelKurve, tekst, maal: () => null, akse: { maks, tekst: pct } });
      blok.append(el('div', { className: 'vg-tavle vg-tavle-pad' }, el('p', { className: 'vg-tavle-top', textContent: T.landAndelKurve }), el('div', { className: 'vg-soejler-rul' }, kurve)));
    }
    return blok;
  }
  const syv = () => params.get('periode') === '7';
  const medPeriode = v => { const p = new URLSearchParams(params); p.delete('periode'); if (v) p.set('periode', v); p.set('afsnit', 'kilde'); return p.toString(); };
  /* Dine lande åbner dyk med samme tal og periode; landets kolonne er ren tekst, for Radar giver kun procenterne. Resten er det, Radar ikke oplyser. */
  function modBlok(d) {
    const navn = V.land(d.kode), s = syv() ? d.sammen7 : d.sammen28, tom = t => el('p', { className: 'vg-note', textContent: t });
    const dyk = k => new URLSearchParams({ samlet: '1', land: k, slags: 'angreb', ...(syv() ? { periode: '7' } : { fra: s.fra, til: s.til }) }).toString();
    const mine = s.mine.slice(0, 10).map(x => ({ navn: V.land(x.navn), vaerdi: x.vaerdi, dyk: dyk(x.navn), maerke: x.saerligt ? T.landSaerligt : null }));
    const kilder = s.land.map(x => ({ navn: V.land(x.navn), vaerdi: x.vaerdi })).concat(s.rest === null ? [] : [{ navn: T.landModRest, vaerdi: s.rest }]);
    const kol = (h, indhold) => el('div', {}, el('h3', { textContent: h }), indhold);
    const blok = V.blok(T.landModBlokH, fmt(T.landModP, { land: navn, p: syv() ? T.landMod7Tekst : T.landMod28Tekst }),
      ...serieNote(d, syv() ? 'par7' : 'par28'),
      el('div', { className: 'vg-tre' },
        kol(fmt(T.landModH, { land: navn }), kilder.length ? procentListe(kilder) : tom(fmt(s.spurgt ? T.landModIkkeOplyst : T.landModIngen, { land: navn }))),
        kol(T.landModMineH, mine.length ? procentListe(mine) : tom(T.landMineIngen))),
      el('p', { className: 'vg-note', textContent: fmt(T.landModNote, { land: navn }) }));
    blok.id = 'land-kilde';
    return blok;
  }
  const varighed = m => (m < 60 ? fmt(T.landUdfaldMin, { n: Math.max(1, m) }) : m < 2880 ? fmt(T.landUdfaldTimer, { n: Math.round(m / 60) }) : fmt(T.landUdfaldDage, { n: Math.round(m / 1440) }));
  /* Radars koder i klarsprog. */
  function udfaldPunkt(u, lande) {
    const dele = [T.landUdfaldAarsag[u.aarsag] || T.landUdfaldUkendt, u.minutter === null ? T.landUdfaldUdenSlut : varighed(u.minutter), ...(T.landUdfaldSlags[u.slags] ? [T.landUdfaldSlags[u.slags]] : [])];
    return el('li', {}, el('b', { textContent: V.datoLang(u.start.slice(0, 10)) }), ...lande, ' ', dele.join(' · '), ...(u.netvaerk.length ? [el('small', { textContent: fmt(T.landUdfaldNet, { liste: u.netvaerk.join(', ') }) })] : []));
  }
  function udfaldBlok(d) {
    const navn = V.land(d.kode), blok = V.blok(fmt(T.landUdfaldH, { land: navn }), T.landUdfaldP, ...serieNote(d, 'udfald'));
    blok.id = 'land-udfald';
    if (d.udfald.length) blok.append(el('ul', { className: 'land-udfald' }, ...d.udfald.map(u => udfaldPunkt(u, []))));
    else blok.append(el('p', { className: 'vg-note', textContent: d.udfald_hentet ? fmt(T.landUdfaldIngen, { land: navn }) : T.landUdfaldVenter }));
    if (d.udfald_verden.length) blok.append(el('p', { className: 'vg-note' }, V.knap(T.landUdfaldSeVerden, { gaa: gaaLand(adresse(d.kode, { fane: 'verden', afsnit: 'udfald' })) })));
    return blok;
  }
  /* Verden: landet står i klarsprog, og et land, vi kender, åbner dets side. */
  function udfaldVerden(d) {
    const blok = V.blok(T.landUdfaldVerdenH, T.landUdfaldVerdenP, ...serieNote(d, 'udfald'));
    blok.id = 'land-udfald-verden';
    const land = k => (kendt(k) ? V.knap(V.land(k), { gaa: landMaal(k, 'udfald') }) : V.land(k));
    const lande = u => u.lande.flatMap((k, i) => [i ? ', ' : ' ', land(k)]);
    if (d.udfald_verden.length) blok.append(el('ul', { className: 'land-udfald' }, ...d.udfald_verden.map(u => udfaldPunkt(u, [...lande(u), ':']))));
    else blok.append(el('p', { className: 'vg-note', textContent: d.udfald_verden_hentet ? T.landUdfaldVerdenIngen : T.landUdfaldVenter }));
    return blok;
  }
  /* Radars tal er procenter: rækker med et mål er knapper, og resten er ren tekst. */
  function procentListe(rk) {
    const m = Math.max(1, ...rk.map(r => r.vaerdi));
    return el('ol', { className: 'vg-liste' }, ...rk.map(r => {
      const navn = el('span', { className: 'vg-l-navn', textContent: r.navn }), n = el('span', { className: 'vg-l-n', textContent: pct(r.vaerdi) }), li = el('li');
      if (r.maerke) navn.append(el('span', { className: 'vg-saerligt', textContent: r.maerke }));
      if (r.gaa || r.dyk) {
        const b = el('button', { type: 'button', className: 'dyk-l' }, navn, n);
        if (r.dyk) b.dataset.dyk = r.dyk; else b.dataset.gaa = r.gaa;
        b.setAttribute('aria-label', fmt(T.dykAabn, { navn: r.maerke ? `${r.navn}, ${r.maerke}` : r.navn, n: pct(r.vaerdi) }));
        li.append(b);
      } else li.append(navn, n);
      li.append(el('i', { className: V.bredde(100 * r.vaerdi / m) }));
      return li;
    }));
  }
  const landMaal = (k, afsnit) => gaaLand(adresse(k, { afsnit }));

  /* ── Verden ── */
  const produktNavn = n => T.landProdukt[n] || n;
  function verdenDele(d) {
    if (!d.verden_dag) return [el('p', { className: 'vg-note', textContent: T.landTom }), udfaldVerden(d)];
    const lande = (rk, afsnit) => rk.map(x => ({ navn: V.land(x.navn), vaerdi: x.vaerdi, gaa: kendt(x.navn) ? landMaal(x.navn, afsnit) : null }));
    /* Kun WAF har en serie bag sig (Hvorfor og kurven); de andre produkter er rene procenter. */
    const produkter = d.produkt.map(x => ({ navn: produktNavn(x.navn), vaerdi: x.vaerdi, gaa: x.navn === 'WAF' ? gaaLand(adresse(d.kode, { produkt: 'WAF' })) : null }));
    const tre = el('div', { className: 'vg-tre' });
    for (const [h, rk] of [[T.landMaalH, lande(d.maal, 'maal')], [T.landKildeH, lande(d.kilde, 'kilde')], [T.landProduktH, produkter]]) if (rk.length) tre.append(el('div', {}, el('h3', { textContent: h }), procentListe(rk)));
    const fliser = el('div', { className: 'sfliser vg-fliser' });
    if (d.bot !== null) fliser.append(V.fliseTekst(pct(d.bot), T.landBotH, T.landBotSub));
    if (d.http_indeks !== null) fliser.append(V.fliseTekst(indeks(d.http_indeks), T.landHttpH, T.landHttpSub));
    if (d.kvante !== null) fliser.append(V.fliseTekst(pct(d.kvante), T.landKvanteH, T.landKvanteSub));
    const robot = [fliser], ai = el('div', { className: 'vg-tre' });
    if (d.ai.length) ai.append(el('div', {}, el('h3', { textContent: T.landAiH }), procentListe(d.ai.map(x => ({ navn: T.landFormaal[x.navn] || x.navn, vaerdi: x.vaerdi })))));
    if (d.ai_navne.length) ai.append(el('div', {}, el('h3', { textContent: T.landAiNavnH }), procentListe(d.ai_navne.map(x => ({ navn: V.robotNavn(x.navn), vaerdi: x.vaerdi })))));
    if (ai.childElementCount) robot.push(ai);
    robot.push(el('p', { className: 'vg-note', textContent: T.landRobotNote }));
    return [V.blok(T.landVerdenH, fmt(T.landVerdenP, { d: V.datoLang(d.verden_dag) }), ...serieNote(d, 'kilde', 'produkt'), tre), V.blok(T.landVerdenRobotH, '', ...serieNote(d, 'bot', 'ai', 'ai_navn', 'kvante', 'http'), ...robot), udfaldVerden(d)];
  }

  function faner() {
    R.baandValg('land-mod', verdenFane() ? null : [[T.landMod7, medPeriode('7'), syv()], [T.landMod28, medPeriode(''), !syv()]], 'land');
    R.baandValg('land-faner', [[V.land(kode), adresse(kode), !verdenFane()], [T.landVerdenFane, adresse(kode, { fane: 'verden' }), verdenFane()]], 'land');
    /* Et kig på et andet land end det gemte: vejen hjem. */
    $('land-kig').replaceChildren(...(gemt && kode !== gemt ? [V.knap(fmt(T.landTilbage, { land: V.land(gemt) }), { gaa: 'land' })] : []));
  }
  function tegn(d) {
    const dele = [];
    if (d.fejl) dele.push(el('p', { className: 'vg-advarsel vg-gul', textContent: fmt(T.landFejl, { f: S.fejl(d.fejl) }) }));
    else if (!d.hentet) dele.push(el('p', { className: 'vg-status', textContent: T.landHenter }));
    if (forslag && !gemt) dele.push(forslagValg());
    if (verdenFane()) dele.push(...verdenDele(d));
    else dele.push(...dom(d), kurveBlok(d), andelBlok(d), modBlok(d), udfaldBlok(d));
    dele.push(kilde(d));
    const genaabn = V.husk(rod);
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(...dele);
    genaabn();
    rod.querySelectorAll('.vg-soejler-rul').forEach(r => { r.scrollLeft = r.scrollWidth; });
  }
  function forslagValg() {
    const b = el('button', { type: 'button', className: 'btn', textContent: fmt(T.landForslag, { land: V.land(forslag) }) });
    b.addEventListener('click', () => skift(forslag, false).catch(e => besked(S.fejl(e))));
    return el('p', { className: 'land-valg' }, b, el('span', { className: 'note', textContent: T.landForslagNote }));
  }
  /* Intet land: vælgeren i båndet og forslaget. */
  function vaelgSelv() {
    rod.removeAttribute('aria-busy');
    R.baandValg('land-faner', null); R.baandValg('land-mod', null);
    $('land-kig').replaceChildren();
    $('land-h').textContent = T.navLand;
    S.saetLand(T.navLand);
    R.titel();
    rod.replaceChildren(el('p', { className: 'vg-advarsel vg-gul', textContent: T.landVaelgP }), ...(forslag ? [forslagValg()] : []));
  }
  function besked(tekst) {
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(el('p', { className: 'vg-advarsel vg-roed', textContent: tekst }));
  }

  let gen = 0, afmeld = [];
  function stop() {
    gen++;
    for (const f of afmeld.splice(0)) f();
  }
  async function start(p) {
    stop();
    const g = gen;
    params = p;
    /* Periodevælgeren har fokus: "Henter …" fjerner den, og den skal have fokus igen bagefter. */
    const a = document.activeElement, valgt = a && a.dataset && /^land-mod:/.test(a.dataset.fokus || '') ? a.dataset.fokus : null;
    rod.setAttribute('aria-busy', 'true');
    rod.replaceChildren(el('p', { className: 'vg-status', textContent: T.dykHenter }));
    if (!invoke) return besked(T.mangler);
    try { await klar(); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
    if (g !== gen) return;
    const k = p.get('kode');
    kode = kendt(k) ? k : gemt;
    tegnValg();
    if (!kode) return vaelgSelv();
    $('land-h').textContent = ligeNu(kode);
    S.saetLand(ligeNu(kode));
    R.titel();
    faner();
    const hent = async () => {
      let d;
      try { d = await invoke('land_visning', { kode, waf: waf() }); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
      if (g === gen) tegn(d);
    };
    await hent();
    if (g !== gen) return;
    const afsnit = { maal: 'land-maal', kilde: 'land-kilde', udfald: verdenFane() ? 'land-udfald-verden' : 'land-udfald' }[p.get('afsnit')], maal = afsnit && $(afsnit);
    const igen = valgt && [...$('land-mod').querySelectorAll('[data-fokus]'), ...rod.querySelectorAll('[data-fokus]')].find(x => x.dataset.fokus === valgt); // periodevælgeren står i båndet
    if (maal && !R.genbesoeg() && maal.scrollIntoView) maal.scrollIntoView({ block: 'start' });
    else if (!R.genbesoeg() && !igen) $('land-h').focus();
    if (igen) igen.focus();
    afmeld.push(R.paa('radar', hent));
    invoke('radar_opdater', { kode }).catch(() => { });
  }
  const ord = $('ord-land');
  for (const id of ['radar', 'indeks', 'kildemaal']) ord.append(el('dt', { textContent: S.bog(id).ord }), el('dd', { textContent: S.bog(id).alle }));
  R.paa('token', () => { klar().catch(() => { }); });
  R.registrer('land', {
    land: true,
    /* Samme adresse kan være et andet land: det gemte er skiftet. */
    vis(p, { samme } = {}) {
      if (samme && (p.get('kode') || gemt) === kode) return;
      start(p);
    },
    skjul: stop,
  });
  /* Indstillinger: "Tilbage til" står, indtil man forlader siden. */
  window.hmLand = { glemTilbage() { tilbage = null; tegnValg(); } };
})();
