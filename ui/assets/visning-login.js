/* Huginn and Muninn — Login: hvad der skete på domænets login-stier. #login?zone=<id>&navn=… eller #login?samlet=1, plus periode=7|alt
   (ingen = 30 hele dage) eller dag=…, og filtrene udfald, sti, metode, land og netvaerk. Tre tal fra hver sin kilde, som aldrig lægges
   sammen, heller ikke i tegningen: stoppet ved Edge, fik 404 og nåede din server. Svar 200 og 302 siger ikke, at nogen kom ind. Tallene
   regnes i Rust (login, kun fra cachen). Et klik bliver her med et filter (data-gaa), for dyk kender kun det, Edge stoppede; kun stien
   åbner dyk, når Edge har stoppet noget på den. Listen over login-stier (panel) deles med Indstillinger. Bygges med createElement. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, R = window.hmRamme, V = window.hmTegn;
  const $ = id => document.getElementById(id);
  const { el, fmt, invoke } = R, tal = S.tal;
  const FILTRE = ['udfald', 'sti', 'metode', 'land', 'netvaerk'];
  const TOP_SYNLIGE = 5;
  const rod = $('login-rod');

  let params = new URLSearchParams(), domaene = '…', sidste = 'samlet=1';
  const samlet = () => params.get('samlet') === '1';
  const har = k => params.has(k);
  function periodeArg() {
    if (har('dag')) return { dag: params.get('dag') };
    if (har('fra') && har('til')) return { spaend: { fra: params.get('fra'), til: params.get('til') } };
    return { 7: 'd7', alt: 'alt' }[params.get('periode')] || 'd30';
  }
  const filter = () => Object.fromEntries(FILTRE.filter(har).map(k => [k, params.get(k)]));
  const kopi = fn => { const p = new URLSearchParams(params); fn(p); return p.toString(); };
  const gaa = fn => ({ gaa: 'login?' + kopi(fn) });
  const med = (k, v) => gaa(p => p.set(k, v));
  const uden = k => gaa(p => p.delete(k));
  /* Et valg af udfald kan vælges fra igen. */
  const udfald = v => (params.get('udfald') === v ? uden('udfald') : med('udfald', v));
  const medPeriode = (k, v) => kopi(p => { ['periode', 'dag', 'fra', 'til'].forEach(x => p.delete(x)); if (k) p.set(k, v); });
  const medInterval = (a, b) => kopi(p => { ['periode', 'dag'].forEach(x => p.delete(x)); p.set('fra', a); p.set('til', b); });
  const dykQ = x => new URLSearchParams({ ...(samlet() ? { samlet: '1' } : { zone: params.get('zone'), navn: domaene }), ...x, ...(har('fra') && har('til') ? { fra: params.get('fra'), til: params.get('til') } : { d7: { periode: '7' }, alt: { periode: 'alt' } }[periodeArg()] || {}) }).toString();
  const netLabel = v => v;

  /* ── listen over login-stier: ét domæne ad gangen; efter() kaldes, når noget er ændret ── */
  async function panel(zone, efter) {
    const boks = el('div', { className: 'login-panel' });
    const st = el('p', { className: 'r6-status' });
    st.setAttribute('role', 'status');
    const fejltekst = e => (e && e.kode === 'ugyldigt_filter' ? T.loginStiUgyldig : S.fejl(e));
    async function saet(sti, til) {
      try { await invoke('login_sti_saet', { zone, sti, til }); R.status(st, T.notifGemt, 'ok'); if (efter) efter(); await tegn(); return true; } catch (e) { R.status(st, fejltekst(e), 'fejl'); return false; }
    }
    async function fjern(sti) {
      try { await invoke('login_sti_fjern', { zone, sti }); R.status(st, T.notifGemt, 'ok'); if (efter) efter(); await tegn(); } catch (e) { R.status(st, fejltekst(e), 'fejl'); }
    }
    /* Reglerne for en sti står kun i Rust (tjek_sti); her bliver det indtastede, til det er godkendt. */
    function egenSti() {
      const felt = el('input', { type: 'text', placeholder: T.loginEgenEksempel, autocomplete: 'off', spellcheck: false, autocapitalize: 'off' });
      const form = el('form', { className: 'hm-form login-egen' }, el('label', {}, el('span', { textContent: T.loginEgenH }), felt), el('button', { className: 'knap', type: 'submit', textContent: T.loginEgenTilfoej }));
      form.addEventListener('submit', async ev => {
        ev.preventDefault();
        if (await saet(felt.value.trim(), true)) { const nyt = boks.querySelector('.login-egen input'); if (nyt) nyt.focus(); }
      });
      return form;
    }
    async function tegn() {
      let d;
      try { d = await invoke('login_stier', { zone }); } catch (e) { R.status(st, S.fejl(e), 'fejl'); boks.replaceChildren(st); return; }
      const ul = el('ul', { className: 'login-valg' });
      for (const s of d.stier) {
        const i = el('input', { className: 'boks', type: 'checkbox', checked: s.til });
        i.addEventListener('change', () => saet(s.sti, i.checked));
        const li = el('li', {}, el('label', { className: 'hm-valgmulighed' }, i, V.sti(s.sti)));
        if (!s.start) {
          const b = el('button', { className: 'link', type: 'button', textContent: T.loginEgenFjern });
          b.setAttribute('aria-label', fmt(T.loginEgenFjernNavn, { sti: s.sti }));
          b.addEventListener('click', () => fjern(s.sti));
          li.append(b);
        }
        ul.append(li);
      }
      const dele = [ul, egenSti()];
      if (d.forslag.length) {
        const f = el('div', { className: 'login-stier' }, el('span', { className: 'note', textContent: T.loginForslagH }));
        for (const sti of d.forslag) {
          const b = el('button', { type: 'button', className: 'sti-chip forslag', textContent: `+ ${sti}` });
          b.setAttribute('aria-label', fmt(T.loginForslagTilfoej, { sti }));
          b.addEventListener('click', () => saet(sti, true));
          f.append(b);
        }
        dele.push(f, el('p', { className: 'note', textContent: T.loginForslagNote }));
      }
      boks.replaceChildren(...dele, st);
    }
    await tegn();
    return boks;
  }

  /* ── dele af visningen ── */
  function stierBar(d) {
    const bar = el('div', { className: 'login-stier' }, el('span', { className: 'note', textContent: T.loginDineStier }));
    for (const sti of d.stier) {
      const valgt = params.get('sti') === sti, b = V.knap(sti, valgt ? uden('sti') : med('sti', sti), 'sti-chip' + (valgt ? ' valgt' : ''));
      if (valgt) b.setAttribute('aria-pressed', 'true');
      bar.append(b);
    }
    return bar;
  }
  function filtreBar(d) {
    const navn = { udfald: v => T.loginUdfald[v] || v, sti: v => v, metode: v => v, land: V.land, netvaerk: netLabel };
    const aktive = FILTRE.filter(har);
    if (!aktive.length && !har('dag') && !har('fra')) return [];
    const bar = el('div', { className: 'login-stier' }, el('span', { className: 'note', textContent: T.loginFiltre }));
    for (const k of aktive) bar.append(V.knap(`${navn[k](params.get(k))} ×`, uden(k), 'sti-chip', fmt(T.dykFjern, { navn: navn[k](params.get(k)) })));
    if (har('dag')) bar.append(V.knap(`${V.datoLang(params.get('dag'))} ×`, { gaa: 'login?' + medPeriode() }, 'sti-chip', fmt(T.dykFjern, { navn: V.datoLang(params.get('dag')) })));
    if (har('fra') && har('til')) { const n = V.interval(params.get('fra'), params.get('til')); bar.append(V.knap(`${n} ×`, { gaa: 'login?' + medPeriode() }, 'sti-chip', fmt(T.dykFjern, { navn: n }))); }
    return [bar];
  }
  function fliser(d) {
    const t = d.tal, boks = el('div', { className: 'vg-fliser' }), u = params.get('udfald');
    const flise = (n, id, sub) => boks.append(V.flise(n, T.loginUdfald[id], sub, udfald(id), { aktiv: u === id }));
    flise(t.edge, 'edge', T.loginEdgeSub);
    if (!har('land') && !har('netvaerk') && !har('metode')) flise(t.ikke_fundet, 'ikke_fundet', T.loginIkkeFundetSub);
    flise(t.server, 'server', T.loginServerSub);
    flise(t.afvist, 'afvist', T.loginAfvistSub);
    flise(t.svar, 'svar', T.loginSvarSub);
    return boks;
  }
  const SERIER = [{ felt: 'edge', klasse: 'vg-s-edge', id: 'edge' }, { felt: 'ikke_fundet', klasse: 'vg-s-404', id: 'ikke_fundet' }, { felt: 'server', klasse: 'vg-s-server', id: 'server' }];
  function dageBlok(d) {
    if (har('dag')) return [];
    const tekst = x => fmt(T.loginDag, { d: V.datoLang(x.dag), e: tal(x.edge), f: tal(x.ikke_fundet), s: tal(x.server) });
    const diagram = V.soejlerSerier(d.dage, SERIER, { titel: T.loginSoejleTitel, tekst, maal: dag => ({ gaa: 'login?' + medPeriode('dag', dag) }) });
    const legende = el('ul', { className: 'vg-legende vg-legende-inde' }, ...SERIER.map(s => el('li', {}, el('i', { className: s.klasse }), T.loginUdfald[s.id])));
    return [V.blok(T.loginDageH, T.loginDageP, el('div', { className: 'vg-tavle vg-tavle-pad' }, legende, el('div', { className: 'vg-soejler-rul' }, diagram)))];
  }
  function talCelle(n, sti, id) {
    const td = el('td', { className: 'tal-kol' });
    if (!n) { td.textContent = tal(n); return td; }
    const b = V.knap(tal(n), gaa(p => { p.set('sti', sti); p.set('udfald', id); }), 'tal-knap', fmt(T.loginCelle, { sti, navn: T.loginUdfald[id], n: tal(n) }));
    td.append(b);
    return td;
  }
  /* Kodeord fra kendte læk, pr. dag og sti. Dagen og stien filtrerer Login. Tallene selv har intet bag sig. */
  function laekBlok(d) {
    const dele = [];
    if (d.laek.length) {
      const n = d.laek.reduce((s, x) => s + x.n, 0), kol = t => el('th', { scope: 'col', textContent: t });
      const rk = d.laek.map(x => el('tr', {}, el('td', {}, V.knap(V.dato(x.dag), { gaa: 'login?' + medPeriode('dag', x.dag) }, 'link dyk-celle', fmt(T.loginLaekDag, { d: V.datoLang(x.dag) }))), el('td', {}, V.knap(x.sti, gaa(p => p.set('sti', x.sti)), 'link dyk-celle', fmt(T.loginLaekAabn, { sti: x.sti }))), el('td', { className: 'tal-kol', textContent: tal(x.n) }), el('td', { className: 'tal-kol', textContent: tal(x.naaet) })));
      dele.push(el('p', { className: 'vg-advarsel vg-gul login-laek-linje', textContent: fmt(T.loginLaekLinje, { n: tal(n) }) }), el('div', { className: 'vg-tabel-rul' }, el('table', { className: 'vg-tabel' }, el('thead', {}, el('tr', {}, kol(T.loginLaekThDag), kol(T.loginLaekThSti), kol(T.loginLaekThN), kol(T.loginLaekThNaaet))), el('tbody', {}, ...rk))));
    } else dele.push(el('p', { className: 'vg-status login-laek-ingen', textContent: d.laek_dage ? T.loginLaekIngen : T.loginLaekVenter }));
    /* `none` og tomme svar er ikke læk. En værdi, Cloudflare ikke har dokumenteret, står rå, så den kan slås op. */
    const svar = v => d.laek_andet.find(x => x.v === v), andre = d.laek_andet.filter(x => x.v !== 'none' && x.v !== 'ukendt');
    if (svar('none')) dele.push(el('p', { className: 'vg-note login-laek-none', textContent: fmt(T.loginLaekNone, { n: tal(svar('none').n) }) }));
    if (svar('ukendt')) dele.push(el('p', { className: 'vg-note login-laek-uden', textContent: fmt(T.loginLaekUdenSvar, { n: tal(svar('ukendt').n) }) }));
    if (andre.length) dele.push(el('p', { className: 'vg-note', textContent: fmt(T.loginLaekAndet, { l: andre.map(x => `${T.laekVaerdi[x.v] || `${T.laekVaerdi.ukendt}: ${x.v}`} (${tal(x.n)})`).join(', ') }) }));
    dele.push(el('p', { className: 'vg-note', textContent: T.loginLaekSlaaetTil }));
    dele.push(el('details', { className: 'dyk-guide' }, el('summary', { textContent: T.gLaekH }), ...V.bogAfsnit('laek')));
    return [V.blok(T.loginLaekH, T.loginLaekP, ...dele)];
  }
  function forklaring(r) {
    if (T.loginForkl[r.sti]) return T.loginForkl[r.sti];
    return r.tal.svar ? T.loginSvarForkl : '';
  }
  function tabel(d) {
    if (!d.raekker.length) return [];
    const th = (t, k) => { const e = el('th', { textContent: t, className: k || 'v' }); e.setAttribute('scope', 'col'); return e; };
    const hoved = el('tr', {}, th(T.dykSti), th(T.dykMetode), th(T.loginKortEdge, 'tal-kol'), th(T.loginKort404, 'tal-kol'), th(T.loginKortServer, 'tal-kol'), th(T.loginKortAfvist, 'tal-kol'), th(T.loginKortSvar, 'tal-kol'), th(T.loginBetyder));
    const rk = d.raekker.map(r => {
      /* Dyk kender kun det, Edge stoppede: uden den slags tal bliver klikket her. */
      const maal = r.tal.edge ? dykQ({ sti: r.sti, ...(r.metoder.length === 1 ? { metode: r.metoder[0] } : {}) }) : gaa(p => p.set('sti', r.sti));
      const sti = el('th', {}, V.knap(V.sti(r.sti), maal, 'sti-knap', fmt(T.dykAabnTekst, { navn: r.sti })));
      sti.setAttribute('scope', 'row');
      return el('tr', {}, sti, el('td', { className: 'v', textContent: r.metoder.join(', ') || '–' }), talCelle(r.tal.edge, r.sti, 'edge'), talCelle(r.tal.ikke_fundet, r.sti, 'ikke_fundet'), talCelle(r.tal.server, r.sti, 'server'), talCelle(r.tal.afvist, r.sti, 'afvist'), talCelle(r.tal.svar, r.sti, 'svar'), el('td', { className: 'forkl', textContent: forklaring(r) }));
    });
    const rul = el('div', { className: 'vg-tabel-rul', tabIndex: 0 }, el('table', { className: 'vg-tabel' }, el('thead', {}, hoved), el('tbody', {}, ...rk)));
    rul.setAttribute('role', 'region');
    rul.setAttribute('aria-label', T.loginTabelH);
    return [V.blok(T.loginTabelH, '', rul)];
  }
  function lister(d) {
    const boks = el('div', { className: 'vg-tre' });
    const liste = (h, noegle, rk, dim, navn) => {
      if (!rk.length || har(dim)) return;
      const rows = rk.map(x => ({ navn: navn(x.v), n: x.n, gaa: med(dim, x.v).gaa }));
      boks.append(el('div', {}, el('h3', { textContent: h }), ...V.listeFlere(rows, TOP_SYNLIGE, noegle, fmt(T.dykFlere, { n: tal(rows.length) }))));
    };
    liste(T.loginLandeEdge, 'l-edge', d.lande_edge, 'land', V.land);
    liste(T.loginLandeServer, 'l-server', d.lande_server, 'land', V.land);
    liste(T.loginNetEdge, 'n-edge', d.net_edge, 'netvaerk', netLabel);
    liste(T.loginNetServer, 'n-server', d.net_server, 'netvaerk', netLabel);
    liste(T.loginMetodeEdge, 'm-edge', d.metoder_edge, 'metode', netLabel);
    liste(T.loginMetodeServer, 'm-server', d.metoder_server, 'metode', netLabel);
    return boks.childElementCount ? [V.blok(T.loginHvorH, T.loginHvorP, boks)] : [];
  }
  async function retStier() {
    if (samlet()) return [el('p', { className: 'vg-note', textContent: T.loginVaelgDomaene })];
    const zone = params.get('zone') || '', ret = el('details', { className: 'login-ret' }, el('summary', { textContent: T.loginRet }));
    ret.dataset.noegle = 'login-ret';
    ret.addEventListener('toggle', async () => {
      if (ret.open && !ret.querySelector('.login-panel')) ret.append(await panel(zone, () => snartUdefra()));
    });
    return [ret];
  }
  let snartUdefra = () => { };
  async function tegn(d, g) {
    $('login-under').textContent = domaene;
    const p = periodeArg(), fast = !har('dag') && !har('fra');
    $('login-periode').replaceChildren(V.segment('login-periode', T.dykPeriode, [[T.dykP7, medPeriode('periode', '7'), fast && p === 'd7'], [T.dykP30, medPeriode(), fast && p === 'd30']], 'login'));
    const genaabn = V.husk(rod), ret = await retStier();
    if (g !== gen) return;
    const dele = [V.datoVaelger({ visning: 'login', periode: d.periode, nulstil: medPeriode('periode', 'alt'), gaa: (a, b) => R.gaaTil('login', medInterval(a, b)) })];
    if (d.stier.length) dele.push(stierBar(d));
    dele.push(...ret, ...filtreBar(d));
    if (d.ikke_tilgaengelig.length) dele.push(el('p', { className: 'vg-advarsel vg-gul', textContent: fmt(T.loginIkkeTilg, { d: d.ikke_tilgaengelig.map(V.dato).join(', ') }) }));
    else if (d.mangler_dage) dele.push(el('p', { className: 'vg-note', textContent: fmt(T.ekstraMangler, { n: tal(d.mangler_dage) }) }));
    if (!d.stier.length) {
      dele.push(el('p', { className: 'vg-advarsel vg-gul', textContent: T.loginIngenStier }), ...laekBlok(d));
      if (ret[0].tagName === 'DETAILS') {
        ret[0].append(await panel(params.get('zone') || '', () => snartUdefra()));
        if (g !== gen) return;
        ret[0].open = true;
      }
    } else {
      dele.push(fliser(d), ...V.daekning(d.periode), ...laekBlok(d), ...dageBlok(d), ...tabel(d), ...lister(d));
    }
    dele.push(el('p', { className: 'vg-note', textContent: T.loginNote }));
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(...dele);
    genaabn();
    rod.querySelectorAll('.vg-soejler-rul').forEach(r => { r.scrollLeft = r.scrollWidth; });
  }
  function besked(tekst, farve = 'roed') {
    rod.removeAttribute('aria-busy');
    rod.replaceChildren(el('p', { className: `vg-advarsel vg-${farve}`, textContent: tekst }));
    $('login-periode').replaceChildren();
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
      try { d = await invoke('login', { omfang, periode: periodeArg(), filter: filter() }); } catch (e) { if (g === gen) besked(S.fejl(e)); return; }
      if (g === gen) await tegn(d, g);
    }
    await hent();
    if (g !== gen) return;
    if (!R.genbesoeg()) $('login-h').focus();
    let sidstHentet = Date.now(), ventende = null;
    const snart = () => {
      if (ventende) return;
      ventende = setTimeout(() => { ventende = null; sidstHentet = Date.now(); hent(); }, Math.max(0, sidstHentet + 3000 - Date.now()));
    };
    snartUdefra = snart;
    afmeld.push(R.paa('hentekoe', snart), R.paa('ekstra', snart), () => clearTimeout(ventende));
  }
  const ord = $('ord-login');
  for (const id of ['loginsti', 'naaede']) ord.append(el('dt', { textContent: S.bog(id).ord }), el('dd', { textContent: S.bog(id).alle }));
  window.hmLoginStier = { panel };
  R.registrer('login', {
    domaener: true,
    hvad: () => ({ samlet: samlet(), zone: params.get('zone') || '', navn: domaene }),
    /* Et andet domæne: samme periode. Stierne er domænets egne, så filtrene følger ikke med. */
    skift(q) {
      const ny = new URLSearchParams(q);
      for (const k of ['periode', 'dag', 'fra', 'til']) if (params.has(k)) ny.set(k, params.get(k));
      R.gaaTil('login', ny);
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
