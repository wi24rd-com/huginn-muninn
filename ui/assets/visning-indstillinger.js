/* Huginn and Muninn — Indstillinger (mockup 3, skærm 2): Generelt (sprog i sprog.js, udseende og tekststørrelse i tema.js, "Start ved login", hvor
   OS'et er sandheden: autostart_hent/autostart_saet), Notifikationer, Forbindelse, Data og Om. Notifikationerne:
   forudindstillingen vælges i opsætningen og her (to grupper, samme værdi); "Giv besked, når et domæne kræver handling" og
   valget pr. domæne gemmes i Rust (notifikation_hent/_saet/_zone_saet). "Pr. domæne" tegnes kun igen, når zonelisten ændrer
   sig (rammen melder 'raekker' hvert 2. sekund, mens køen henter). Forbindelse: chippen læses af rammens tilstand (token_status,
   zones_cache, køens fejlkode); "Skift nøgle…" folder feltet ud i kortet, og "Glem nøglen…" kræver bekræftelse i en <dialog>.
   Data: data_status, når siden åbnes og når overblikket er nyt. "Nulstil historik…" kræver bekræftelse i en <dialog>, som
   nævner det konkrete tab. #indstillinger?afsnit=om (programmenuens Om) ruller til Om. Klikbart: chippen og "Domæner med tal" åbner
   Overblik, et domæne under Pr. domæne dets Angreb, og Historik det samlede dyk (kun når der er data bag; ellers er knappen slået fra).
   Login-stier pr. domæne står under Notifikationer (visning-login.js tegner listen); #indstillinger?afsnit=login&zone=… ruller dertil.
   Domæner er sin egen gruppe, for Notifikationer og Login-stier viser kun det, der er valgt her. Domæner er et tilvalg: kontakten gemmer de valgte i Rust, og
   ingen valgte er en normal tilstand, så der er ingen bekræftelse. Kun et valgt domæne kan åbnes: det andet er væk overalt. Nye domæner står med mærket "ny".
   Under domænelisten står en linje om teknologi-valgene og knappen til siden #teknologi. Land står under Generelt: samme valg som vælgeren i <Land> lige nu (visning-land.js fylder og gemmer).
   Bygges med createElement/textContent. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, R = window.hmRamme;
  const $ = id => document.getElementById(id);
  const { el, fmt, status } = R;
  const invoke = R.invoke;
  if (!invoke) return;

  /* Kildekodens adresse (J6: tekst, der kan markeres). Tom: rækken vises ikke. */
  const KILDEKODE = '';

  /* ── Generelt ── */
  const temaValg = () => document.querySelectorAll('input[name="tema"]');
  temaValg().forEach(i => {
    i.checked = i.value === window.hmTema.valg;
    i.addEventListener('change', () => { if (i.checked) window.hmTema.saet(i.value); });
  });

  const tekstValg = () => document.querySelectorAll('input[name="tekst"]');
  const visTekst = () => tekstValg().forEach(i => { i.checked = i.value === window.hmTekst.valg; });
  tekstValg().forEach(i => i.addEventListener('change', () => { if (i.checked) window.hmTekst.saet(i.value); }));
  document.addEventListener('hm-tekst', visTekst); // ⌘+ ⌘− ⌘0 skifter også
  visTekst();

  let notif = { niveau: 'standard', roed: true, rekognoscering: false, zoner: {} };
  let prDomaeneNoegle = null; // zonelisten, som "Pr. domæne" sidst blev tegnet med

  const NIVEAU_GRUPPER = ['opsaet-niveau', 'indst-niveau'];
  function visNotif() {
    for (const g of NIVEAU_GRUPPER) document.querySelectorAll(`input[name="${g}"]`).forEach(i => { i.checked = i.value === notif.niveau; });
    $('notif-roed').checked = notif.roed;
    $('notif-rekog').checked = Boolean(notif.rekognoscering);
    prDomaeneNoegle = null;
    tegnPrDomaene();
  }
  async function hentNotif() {
    try { notif = await invoke('notifikation_hent'); } catch (e) { /* standarden står */ }
    visNotif();
  }
  async function gemNotif(niveau, roed) {
    const foer = { ...notif };
    notif = { ...notif, niveau, roed };
    visNotif();
    try { await invoke('notifikation_saet', { niveau, roed }); status($('indst-status'), T.notifGemt, 'ok'); }
    catch (e) { notif = foer; visNotif(); status($('indst-status'), S.fejl(e), 'fejl'); }
  }
  function tegnPrDomaene() {
    const zoner = [...R.tilstand.raekker].sort((a, b) => a.navn.localeCompare(b.navn));
    const noegle = zoner.map(r => r.id + '\t' + r.navn).join('\n');
    if (noegle === prDomaeneNoegle) return;
    prDomaeneNoegle = noegle;
    const tb = $('pr-domaene-liste');
    tb.replaceChildren();
    if (!zoner.length) {
      const td = el('td', { className: 'note', textContent: T.prDomaeneTom, colSpan: 2 });
      tb.append(el('tr', {}, td));
      return;
    }
    for (const r of zoner) {
      const v = el('select');
      for (const k of ['standard', 'alt', 'roed', 'fra']) v.append(el('option', { value: k, textContent: T.zv[k] }));
      v.value = (notif.zoner || {})[r.id] || 'standard';
      v.setAttribute('aria-label', fmt(T.prDomaeneFor, { navn: r.navn }));
      v.addEventListener('change', async () => {
        const foer = (notif.zoner || {})[r.id] || 'standard';
        try {
          await invoke('notifikation_zone_saet', { zone: r.id, valg: v.value });
          notif.zoner = { ...(notif.zoner || {}), [r.id]: v.value };
          if (v.value === 'standard') delete notif.zoner[r.id];
          status($('indst-status'), T.notifGemt, 'ok');
        } catch (e) { v.value = foer; status($('indst-status'), S.fejl(e), 'fejl'); }
      });
      tb.append(el('tr', {}, el('th', { scope: 'row' }, window.hmTegn.knap(window.hmTegn.domaene(r.navn), { gaa: 'vaernet?' + new URLSearchParams({ zone: r.id, navn: r.navn }) }, 'link dyk-celle', fmt(T.aabnFor, { navn: r.navn }))), el('td', {}, v)));
    }
  }

  /* ── Domæner ── */
  /* "Teknologi pr. domæne": én linje og knappen til siden #teknologi (visning-teknologi.js), hvor valgene sættes. */
  let tekValg = { zoner: {}, valg: false };
  $('tek-aabn').setAttribute('aria-label', T.domTekH);
  async function hentTekValg() {
    try { tekValg = await invoke('teknologi_oversigt') || tekValg; } catch (e) { tekValg = { zoner: {}, valg: false }; }
    tegnTek();
  }
  function tegnTek() {
    const domaener = R.tilstand.domaener.filter(d => d.til).sort((a, b) => a.navn.localeCompare(b.navn)), n = d => tekValg.zoner[d.id] || 0;
    const med = domaener.filter(n), uden = domaener.length - med.length;
    $('tek-blok').hidden = !domaener.length;
    $('tek-raekke').hidden = !tekValg.valg;
    $('tek-linje').textContent = [...med.slice(0, 3).map(d => fmt(T.domTekValg, { navn: d.navn, n: n(d) })), ...(uden ? [fmt(T.domTekUden, { n: T.domaener(uden) })] : [])].join(' · ');
  }
  $('tek-nulstil-alle').addEventListener('click', async () => {
    if (!await R.bekraeft(T.bekraeftTekAlleP)) return;
    try { await invoke('teknologi_nulstil', { zone: null }); status($('domaener-status'), T.tekNulstillet, 'ok'); await R.hentOverblik(); await hentTekValg(); } catch (e) { status($('domaener-status'), S.fejl(e), 'fejl'); }
  });
  async function gemDomaener(valgte) {
    try { await R.gemValgte(valgte); status($('domaener-status'), T.domaenerGemt, 'ok'); hentTekValg(); }
    catch (e) { status($('domaener-status'), S.fejl(e), 'fejl'); tegnDomaener(true); }
  }
  let domNoegle = null; // listen, som "Domæner" sidst blev tegnet med: rammen melder ofte, og en omtegning tager fokus
  function tegnDomaener(tving) {
    const liste = [...R.tilstand.domaener].sort((a, b) => a.navn.localeCompare(b.navn)), noegle = liste.map(d => `${d.id}\t${d.navn}\t${d.til}\t${d.ny}`).join('\n');
    const antalTil = liste.filter(d => d.til).length;
    $('domaener-alle-til').disabled = antalTil === liste.length;
    $('domaener-alle-fra').disabled = !antalTil;
    $('domaener-ser').hidden = !liste.length;
    $('domaener-ser').textContent = T.domaenerSer(liste.length);
    if (noegle === domNoegle && !tving) return;
    domNoegle = noegle;
    const tb = $('domaener-liste');
    if (!liste.length) { tb.replaceChildren(el('tr', {}, el('td', { className: 'note', textContent: T.prDomaeneTom, colSpan: 2 }))); return; }
    tb.replaceChildren(...liste.map(d => {
      const k = el('input', { className: 'kontakt', type: 'checkbox', checked: d.til });
      k.setAttribute('role', 'switch');
      k.setAttribute('aria-label', fmt(T.domaenerFor, { navn: d.navn }));
      k.addEventListener('change', () => {
        const valgte = new Set(liste.filter(x => x.til).map(x => x.id));
        if (k.checked) valgte.add(d.id); else valgte.delete(d.id);
        gemDomaener(valgte);
      });
      const navn = d.til ? window.hmTegn.knap(window.hmTegn.domaene(d.navn), { gaa: 'vaernet?' + new URLSearchParams({ zone: d.id, navn: d.navn }) }, 'link dyk-celle', fmt(T.aabnFor, { navn: d.navn })) : window.hmTegn.domaene(d.navn);
      const th = el('th', { scope: 'row' }, navn);
      if (d.ny) th.append(' ', el('span', { className: 'ny', textContent: T.ny }));
      return el('tr', {}, th, el('td', {}, k));
    }));
  }
  $('domaener-alle-til').addEventListener('click', () => gemDomaener(R.tilstand.domaener.map(d => d.id)));
  $('domaener-alle-fra').addEventListener('click', () => gemDomaener([]));

  /* ── Login-stier pr. domæne ── */
  const loginValg = $('login-domaene');
  let loginOensket = null; // domænet fra adressen: zonelisten kan komme, efter at siden er åbnet
  function tegnLoginValg(zone) {
    loginOensket = zone || loginOensket;
    const zoner = [...R.tilstand.raekker].sort((a, b) => a.navn.localeCompare(b.navn)), foer = loginOensket || loginValg.value;
    const noegle = zoner.map(r => r.id + '\t' + r.navn).join('\n');
    $('indst-login').hidden = !zoner.length;
    if (noegle !== loginValg.dataset.noegle) {
      loginValg.dataset.noegle = noegle;
      loginValg.replaceChildren(...zoner.map(r => el('option', { value: r.id, textContent: r.navn })));
    }
    if (zoner.some(r => r.id === foer)) { loginValg.value = foer; loginOensket = null; }
    if (loginValg.value && loginValg.value !== loginValg.dataset.vist) tegnLoginPanel();
  }
  function tegnLoginPanel() {
    loginValg.dataset.vist = loginValg.value;
    window.hmLoginStier.panel(loginValg.value).then(p => $('login-panel').replaceChildren(p));
  }
  loginValg.addEventListener('change', tegnLoginPanel);

  async function visAutostart() {
    try { $('autostart').checked = await invoke('autostart_hent'); visAutostartHul(); } catch (e) { status($('indst-status'), S.fejl(e), 'fejl'); }
  }
  /* Slået fra er rødt: banneret øverst med knappen, og noten i rækken er kort, så det ikke står to gange. */
  const visAutostartHul = () => { const fra = !$('autostart').checked; $('autostart-hul').hidden = !fra; $('autostart-banner').hidden = !fra; };
  async function saetAutostart(til) {
    const boks = $('autostart'), st = $('indst-status');
    boks.disabled = true;
    try { boks.checked = await invoke('autostart_saet', { til }); status(st, boks.checked ? T.autostartTil : '', 'ok'); }
    catch (e) { boks.checked = !til; status(st, S.fejl(e), 'fejl'); }
    finally { boks.disabled = false; visAutostartHul(); }
  }
  $('autostart').addEventListener('change', () => saetAutostart($('autostart').checked));
  $('autostart-til').addEventListener('click', () => saetAutostart(true));
  for (const g of NIVEAU_GRUPPER) document.querySelectorAll(`input[name="${g}"]`).forEach(i => i.addEventListener('change', () => { if (i.checked) gemNotif(i.value, notif.roed); }));
  $('notif-roed').addEventListener('change', () => gemNotif(notif.niveau, $('notif-roed').checked));
  $('notif-rekog').addEventListener('change', async () => {
    const til = $('notif-rekog').checked;
    try { await invoke('notifikation_rekog_saet', { til }); notif.rekognoscering = til; status($('indst-status'), T.notifGemt, 'ok'); }
    catch (e) { $('notif-rekog').checked = !til; status($('indst-status'), S.fejl(e), 'fejl'); }
  });

  /* En knap i et kort: et mål, når der er data bag; ellers slået fra og uden handling. */
  function saetMaal(b, maal, tekst) {
    b.disabled = !maal;
    delete b.dataset.gaa;
    delete b.dataset.dyk;
    b.removeAttribute('aria-label');
    if (!maal) return;
    if (maal.gaa) b.dataset.gaa = maal.gaa; else b.dataset.dyk = maal.dyk;
    b.setAttribute('aria-label', fmt(T.dykAabnTekst, { navn: tekst }));
  }

  /* ── Forbindelse ── */
  function tegnForbindelse() {
    const st = R.tilstand, c = $('forb-chip'), afvist = Boolean(st.koe && st.koe.fejl && st.koe.fejl.kode === 'afvist');
    const n = st.gemte && st.gemte.zoner ? st.gemte.zoner.length : 0;
    const [klasse, tekst] = !st.sat ? ['graa', T.forbIngen] : afvist ? ['roed', T.forbAfvist] : ['groen', fmt(T.forbOk, { n: T.domaener(n) })];
    c.className = 'chip chip-knap bred chip-' + klasse;
    c.textContent = tekst;
    saetMaal(c, st.sat ? { gaa: 'overblik' } : null, tekst);
    $('glem').hidden = !st.sat;
    $('data-hent').disabled = !st.sat || R.henterNu();
    $('nulstil').disabled = !st.sat; // uden nøgle kan intet hentes igen
  }
  function visSkift(aaben) {
    $('skift-form').hidden = !aaben;
    $('skift-noegle').setAttribute('aria-expanded', String(aaben));
    $('skift-felt').value = '';
    status($('skift-status'), '');
    if (aaben) $('skift-felt').focus();
  }
  $('skift-noegle').addEventListener('click', () => visSkift($('skift-form').hidden));
  $('skift-annuller').addEventListener('click', () => { visSkift(false); $('skift-noegle').focus(); });
  $('skift-form').addEventListener('submit', async ev => {
    ev.preventDefault();
    if (!await R.gemNoegle($('skift-felt'), $('skift-gem'), $('skift-status'))) return;
    $('skift-form').hidden = true;
    $('skift-noegle').setAttribute('aria-expanded', 'false');
  });
  const dialog = $('glem-dialog');
  $('glem').addEventListener('click', () => dialog.showModal());
  $('glem-nej').addEventListener('click', () => dialog.close());
  $('glem-ja').addEventListener('click', async () => {
    dialog.close();
    try { await invoke('token_delete'); await R.vis(); status($('indst-status'), T.glemt, 'ok'); R.glemt(); }
    catch (e) { status($('indst-status'), S.fejl(e), 'fejl'); }
  });

  /* ── Data ── */
  /* Hullerne som perioder: dage i træk står samlet. */
  let hulDage = [];
  function hulPerioder(dage) {
    const ud = [], ms = d => Date.parse(`${d}T00:00:00Z`);
    for (const d of dage) { const s = ud[ud.length - 1]; if (s && ms(d) - ms(s[1]) === 86400000) s[1] = d; else ud.push([d, d]); }
    return ud;
  }
  $('data-huller').addEventListener('click', () => {
    const liste = el('ul', {}, ...hulPerioder(hulDage).map(([a, b]) => el('li', { textContent: S.periode(a, b) })));
    R.hjaelp(true, { titel: 'hulListeH', indhold: el('div', {}, el('p', { textContent: T.hulListeP }), liste) });
  });
  async function tegnData() {
    let d = null;
    try { d = await invoke('data_status'); } catch (e) { /* streger står */ }
    const m = R.tilstand.gemte && R.tilstand.gemte.zoner ? R.tilstand.gemte.zoner.length : 0;
    $('data-stoerrelse').textContent = d ? S.stoerrelse(d.bytes) : '–';
    /* Alle datoer her har året: historikken går over årsskiftet. Tallene kommer fra samme beregning som dyk (Store::aeldste). */
    $('data-historik').textContent = !d ? '–' : d.foerste ? S.periode(d.foerste, d.sidste) : T.dataIngen;
    saetMaal($('data-historik'), d && d.foerste ? { dyk: 'samlet=1&periode=alt' } : null, $('data-historik').textContent);
    // Domæner fra en tidligere nøgle tælles med i historikken, så N kan være større end zonelisten.
    $('data-domaener').textContent = !d ? '–' : m && d.domaener <= m ? fmt(T.dataNafM, { n: S.tal(d.domaener), m: S.tal(m) }) : S.tal(d.domaener);
    saetMaal($('data-domaener'), d && d.domaener ? { gaa: 'overblik' } : null, $('data-domaener').textContent);
    let h = null;
    try { h = await invoke('huller'); } catch (e) { /* streger står */ }
    $('data-aeldste').textContent = d && d.foerste ? S.dato(d.foerste) : '–';
    $('data-kilder-raekke').hidden = !(d && d.sikkerhed_fra);
    $('data-kilder').textContent = !d || !d.sikkerhed_fra ? '' : d.trafik_fra ? fmt(T.dataKilderTekst, { a: S.dato(d.trafik_fra), b: S.dato(d.sikkerhed_fra) }) : fmt(T.dataTrafikHentes, { b: S.dato(d.sikkerhed_fra) });
    $('data-kvote-raekke').hidden = !(d && d.kvote);
    $('data-kvote').textContent = d && d.kvote ? fmt(T.dataKvote, { n: S.tal(d.kvote.kald), m: S.tal(d.kvote.minutter), k: S.tal(d.kvote.brugt) }) : '';
    const b = $('data-huller'), n = h ? h.dage.length : 0;
    b.textContent = h ? (n ? fmt(T.dataHullerN, { n: S.tal(n) }) : S.tal(0)) : '–';
    b.disabled = !n;
    hulDage = h ? h.dage : [];
  }
  $('data-hent').addEventListener('click', R.hentNu);
  /* Nulstil sletter Muninns hukommelse: det, Cloudflare ikke længere har, findes kun her. Dialogen siger det med datoer, pr. slags tal. Kan databasen ikke læses, nævnes tabet for en sikkerheds skyld. */
  function nulstilLinjer(d) {
    if (!d) return [T.nulstilUkendt];
    const linje = (t, tekst) => { const [fra, g] = S.datoPar(t.fra, t.graense); return fmt(tekst, { fra, g, n: S.tal(t.dage) }); };
    const { sikkerhed: s, trafik: f } = d.nulstil, ud = [...(s && s.dage ? [linje(s, T.nulstilSikkerhed)] : []), ...(f && f.dage ? [linje(f, T.nulstilTrafik)] : [])];
    return ud.length ? ud : [T.nulstilIntetTab];
  }
  const nulstilDialog = $('nulstil-dialog');
  $('nulstil').addEventListener('click', async () => {
    let d = null;
    try { d = await invoke('data_status'); } catch (e) { /* ukendt */ }
    $('nulstil-p').textContent = nulstilLinjer(d).join(' ');
    nulstilDialog.showModal();
    $('nulstil-nej').focus();
  });
  $('nulstil-nej').addEventListener('click', () => nulstilDialog.close());
  $('nulstil-ja').addEventListener('click', async () => {
    nulstilDialog.close();
    try { await invoke('nulstil_historik'); status($('indst-status'), T.nulstilt, 'ok'); await R.hentOverblik(); }
    catch (e) { status($('indst-status'), S.fejl(e), 'fejl'); }
  });

  /* ── Om ── */
  async function visOm() {
    const v = window.__TAURI__.app ? await window.__TAURI__.app.getVersion().catch(() => null) : null;
    $('om-version').textContent = v ? fmt(T.omVersion, { v }) : '';
  }
  if (KILDEKODE) { $('om-kilde').textContent = fmt(T.omKilde, { url: KILDEKODE }); $('om-kilde').hidden = false; }
  // data-t, så sprog.js' anvend() ikke skriver den almene tekst tilbage.
  if (R.mac) { for (const [id, k] of [['forb-note', 'forbNoteMac'], ['data-hist-note', 'dataHistNoteMac'], ['nulstil-note', 'nulstilNoteMac']]) { $(id).dataset.t = k; $(id).textContent = T[k]; } }

  R.paa('raekker', () => { tegnPrDomaene(); tegnLoginValg(); tegnForbindelse(); if (R.tilstand.visning === 'indstillinger') tegnData(); });
  R.paa('domaener', () => { tegnDomaener(); tegnTek(); });
  R.paa('token', tegnForbindelse);
  R.paa('koe', tegnForbindelse);
  R.registrer('indstillinger', {
    vis(params) {
      tegnForbindelse();
      tegnData();
      tegnLoginValg(params.get('zone'));
      tegnDomaener();
      hentTekValg();
      const afsnit = { om: $('indst-om'), login: $('indst-login'), domaener: $('indst-domaener') }[params.get('afsnit')];
      if (afsnit && !R.genbesoeg() && afsnit.scrollIntoView) afsnit.scrollIntoView({ block: 'start' });
    },
    skjul() { visSkift(false); if (dialog.open) dialog.close(); if (nulstilDialog.open) nulstilDialog.close(); window.hmLand.glemTilbage(); },
  });
  visAutostart();
  hentNotif();
  visOm();
})();
