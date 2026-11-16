/* Huginn and Muninn — Indstillinger, Domæner, Om og guiden ved første start (V-INDST.1). Markup'en står i index.html (#indst-dele
   og #guide-dele, skjulte holdere); her flyttes den ind i kittets dialoger, så hver kontrol beholder sit id og sin logik:
   · popuppen Indstillinger bag tandhjulet i titellinjen (#indst-knap, ⌘, / Ctrl+,): Udseende og sprog, Dato og land, Start,
     Notifikationer, Forbindelse(r), Data og Første start. Alt gemmes med det samme. Sprog og datoformat genindlæser, og popuppen
     åbner igen bagefter (sessionStorage). "Glem…" og "Nulstil historik…" spørger i en dialog oven på popuppen; "Omdøb…" har sin egen.
     Forbindelse(r) (V-KONTI.1): én forbindelse = ét API Token og et navn; Tilføj, Skift, Omdøb og Glem pr. forbindelse.
   · dialogen Domæner (Overblik › Vælg domæner, og alle gamle veje med afsnit=domaener): valget pr. domæne. "Giv besked" pr. valgt domæne er sin egen
     dialog bag klokken i båndet (Overblik › Notifikationer). Ingen valgte er en normal tilstand, så der er ingen bekræftelse. Samme del står i guidens trin 3.
   · dialogen Om bag #om-knap og programmenuens Om.
   · guiden (kittets Kit.guide i en dialog): Velkommen, Dit API Token, Dine domæner, Besked. Den åbner af sig selv, første gang
     programmet står uden API Token, og fra "Vis guiden igen"; at den er vist, huskes i localStorage (hm-guide). "Næste" på
     trin 2 er spærret, til et API Token er gemt og virker.
   Rammen kalder window.hmIndst (aabn, domaener, om, guide); adressen #indstillinger oversættes i ramme.js. Bygges med createElement/textContent. */
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
  const visTema = () => temaValg().forEach(i => { i.checked = i.value === window.Tema.valg; });
  visTema();
  temaValg().forEach(i => i.addEventListener('change', () => { if (i.checked) window.Tema.saet(i.value); }));
  document.addEventListener('hm-tema', visTema);

  const tekstValg = () => document.querySelectorAll('input[name="tekst"]');
  const visTekst = () => tekstValg().forEach(i => { i.checked = i.value === window.Tekst.valg; });
  tekstValg().forEach(i => i.addEventListener('change', () => { if (i.checked) window.Tekst.saet(i.value); }));
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
          status($('besked-status'), T.notifGemt, 'ok');
        } catch (e) { v.value = foer; status($('besked-status'), S.fejl(e), 'fejl'); }
      });
      tb.append(el('tr', {}, el('th', { scope: 'row' }, window.hmTegn.knap(window.hmTegn.domaene(r.navn), { gaa: 'vaernet?' + new URLSearchParams({ zone: r.id, navn: r.navn }) }, 'link dyk-celle', fmt(T.aabnFor, { navn: r.navn }))), el('td', {}, el('span', { className: 'vliste' }, v, window.Kit.ikon('ned')))));
    }
  }

  /* ── Domæner ── */
  async function gemDomaener(valgte) {
    try { await R.gemValgte(valgte); status($('domaener-status'), T.domaenerGemt, 'ok'); }
    catch (e) { status($('domaener-status'), S.fejl(e), 'fejl'); tegnDomaener(true); }
  }
  let domNoegle = null; // listen, som "Domæner" sidst blev tegnet med: rammen melder ofte, og en omtegning tager fokus
  const valgteNu = () => new Set(R.tilstand.domaener.filter(x => x.til).map(x => x.id));
  function domRaekke(d) {
    const k = el('input', { className: 'tilfra', type: 'checkbox', checked: d.til });
    k.setAttribute('role', 'switch');
    k.setAttribute('aria-label', fmt(T.domaenerFor, { navn: d.navn }));
    k.addEventListener('change', () => {
      const valgte = valgteNu();
      if (k.checked) valgte.add(d.id); else valgte.delete(d.id);
      gemDomaener(valgte);
    });
    const navn = d.til ? window.hmTegn.knap(window.hmTegn.domaene(d.navn), { gaa: 'vaernet?' + new URLSearchParams({ zone: d.id, navn: d.navn }) }, 'link dyk-celle', fmt(T.aabnFor, { navn: d.navn })) : window.hmTegn.domaene(d.navn);
    const th = el('th', { scope: 'row' }, navn);
    if (d.ny) th.append(' ', el('span', { className: 'ny', textContent: T.ny }));
    return el('tr', {}, th, el('td', {}, k));
  }
  const tomRaekke = () => el('tr', {}, el('td', { className: 'note', textContent: T.prDomaeneTom, colSpan: 2 }));
  /* Med mere end én forbindelse (V-KONTI.1, punkt 3): én overskrift med det viste navn og én tabel pr. forbindelse i id-orden, hver med sit eget
     "Vælg alle / Ingen". En delt zone står under sin ejer (domaener_hent.forbindelse); en zone uden ejer står under den første. Med én: som i dag. */
  function domGrupper(liste) {
    const fs = R.tilstand.forbindelser, foerste = fs[0].id;
    return fs.map(f => {
      const egne = liste.filter(d => (fs.some(x => x.id === d.forbindelse) ? d.forbindelse : foerste) === f.id), navn = R.vistNavn(f), hId = `dom-gruppe-${f.id}-h`;
      const antalTil = egne.filter(d => d.til).length;
      const alle = window.Kit.knap({ tekst: T.domGruppeAlle, aria: fmt(T.domGruppeAlleAria, { navn }), deaktiveret: !egne.length || antalTil === egne.length, ved: () => { const v = valgteNu(); egne.forEach(d => v.add(d.id)); gemDomaener(v); } });
      const ingen = window.Kit.knap({ tekst: T.domGruppeIngen, aria: fmt(T.domGruppeIngenAria, { navn }), deaktiveret: !antalTil, ved: () => { const v = valgteNu(); egne.forEach(d => v.delete(d.id)); gemDomaener(v); } });
      const h = el('h3', { className: 'etik dom-gruppe-h', id: hId, textContent: navn, tabIndex: -1 });
      const hoved = el('thead', { className: 'sr-only' }, el('tr', {}, el('th', { scope: 'col', textContent: T.thDomaene }), el('th', { scope: 'col', textContent: T.domaenerVis })));
      const tabel = el('table', { className: 'mini' }, hoved, el('tbody', {}, ...(egne.length ? egne.map(domRaekke) : [tomRaekke()])));
      tabel.setAttribute('aria-labelledby', hId);
      const sek = el('section', { className: 'dom-gruppe' }, el('div', { className: 'raekke' }, h, el('div', { className: 'raekke-knapper' }, alle, ingen)), tabel);
      sek.dataset.forbindelse = String(f.id);
      sek.setAttribute('aria-labelledby', hId);
      return sek;
    });
  }
  function tegnDomaener(tving) {
    const liste = [...R.tilstand.domaener].sort((a, b) => a.navn.localeCompare(b.navn)), fs = R.tilstand.forbindelser, flere = fs.length > 1;
    const noegle = (flere ? fs.map(f => `${f.id}\t${R.vistNavn(f)}`).join('\n') + '\n\n' : '') + liste.map(d => `${d.id}\t${d.navn}\t${d.til}\t${d.ny}\t${d.forbindelse}`).join('\n');
    const antalTil = liste.filter(d => d.til).length;
    $('domaener-alle-til').disabled = antalTil === liste.length;
    $('domaener-alle-fra').disabled = !antalTil;
    $('domaener-alle-til').parentElement.hidden = flere; // med flere forbindelser har hver gruppe sine egne
    $('domaener-ser').hidden = !liste.length;
    $('domaener-ser').textContent = flere ? fmt(T.domaenerSerFlere, { n: liste.length, m: fs.length }) : T.domaenerSer(liste.length);
    if (noegle === domNoegle && !tving) return;
    domNoegle = noegle;
    $('domaener-tabel').hidden = flere;
    $('domaener-grupper').hidden = !flere;
    if (flere) { $('domaener-liste').replaceChildren(); $('domaener-grupper').replaceChildren(...domGrupper(liste)); return; }
    $('domaener-grupper').replaceChildren();
    $('domaener-liste').replaceChildren(...(liste.length ? liste.map(domRaekke) : [tomRaekke()]));
  }
  $('domaener-alle-til').addEventListener('click', () => gemDomaener(R.tilstand.domaener.map(d => d.id)));
  $('domaener-alle-fra').addEventListener('click', () => gemDomaener([]));

  async function visAutostart() {
    try { $('autostart').checked = await invoke('autostart_hent'); visAutostartHul(); } catch (e) { status($('indst-status'), S.fejl(e), 'fejl'); }
  }
  /* Slået fra er rødt: banneret øverst med knappen, og noten i rækken er kort, så det ikke står to gange. */
  const visAutostartHul = () => { const fra = !$('autostart').checked; $('autostart-hul').hidden = !fra; $('autostart-banner').hidden = !fra; $('guide-autostart').checked = !fra; };
  async function saetAutostart(til) {
    const boks = $('autostart'), st = $('indst-status');
    boks.disabled = true;
    try { boks.checked = await invoke('autostart_saet', { til }); status(st, boks.checked ? T.autostartTil : '', 'ok'); }
    catch (e) { boks.checked = !til; status(st, S.fejl(e), 'fejl'); }
    finally { boks.disabled = false; visAutostartHul(); }
  }
  $('autostart').addEventListener('change', () => saetAutostart($('autostart').checked));
  $('autostart-til').addEventListener('click', () => saetAutostart(true));
  $('guide-autostart').addEventListener('change', () => saetAutostart($('guide-autostart').checked));
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

  /* ── Forbindelse(r) (V-KONTI.1) ──
     Én forbindelse ser ud som i dag (D8): Status-chippen og "Skift API Token…"/"Glem API Token…", plus "Omdøb…" og "Tilføj API Token…". Med flere står
     én række pr. forbindelse: navnet (ikke klikbart), "{n} domæner" (åbner Domæner ved forbindelsens gruppe), tilstanden som chip (grøn → Overblik,
     rød → "Skift API Token…" på rækken) og knapperne Omdøb…, Skift API Token… og Glem…. Formularen er den samme for Tilføj og Skift og flyttes hen
     under den række, den hører til. */
  const forbindelser = () => R.tilstand.forbindelser;
  const flere = () => forbindelser().length > 1;
  /* Den eneste forbindelses id. Kendes listen ikke (forbindelser_hent fejlede), er det 1: nøgleringens oprindelige post. */
  const enesteId = () => (forbindelser()[0] ? forbindelser()[0].id : 1);
  const navnPaa = id => R.vistNavn(forbindelser().find(f => f.id === id) || { id });
  const erAfvist = id => R.afvist().ider.includes(id) || (!flere() && Boolean(R.tilstand.koe && R.tilstand.koe.fejl && R.tilstand.koe.fejl.kode === 'afvist'));
  let skiftMaal = null; // null: formularen er lukket; { id }: Skift på den forbindelse; { ny: true }: Tilføj
  let skiftKnap = null;
  function tegnForbindelse() {
    const st = R.tilstand, mange = st.sat && flere();
    $('forb-overskrift').dataset.t = mange ? 'grpForbindelser' : 'grpForbindelse';
    $('forb-overskrift').textContent = mange ? T.grpForbindelser : T.grpForbindelse;
    $('forb-status-raekke').hidden = mange;
    $('forb-noegle-raekke').hidden = mange;
    $('forb-liste').hidden = !mange;
    $('forb-tilfoej-raekke').hidden = !st.sat;
    $('forb-omdoeb').hidden = !st.sat;
    const c = $('forb-chip'), afvist = st.sat && erAfvist(enesteId());
    const n = st.gemte && st.gemte.zoner ? st.gemte.zoner.length : 0;
    const [klasse, tekst] = !st.sat ? ['graa', T.forbIngen] : afvist ? ['roed', T.forbAfvist] : ['groen', fmt(T.forbOk, { n: T.domaener(n) })];
    const ny = window.hmTegn.chip(klasse, tekst, 'button');
    c.className = ny.className;
    c.replaceChildren(...ny.childNodes);
    /* Rød: chippen åbner "Skift API Token…" (som på rækkerne med flere). */
    saetMaal(c, st.sat && !afvist ? { gaa: 'overblik' } : null, tekst);
    if (afvist) { c.disabled = false; c.setAttribute('aria-label', fmt(T.forbSkiftAria, { navn: navnPaa(enesteId()) })); c.setAttribute('aria-controls', 'skift-form'); } else c.removeAttribute('aria-controls');
    $('glem').hidden = !st.sat;
    $('data-hent').disabled = !st.sat || R.henterNu();
    $('nulstil').disabled = !st.sat; // uden nøgle kan intet hentes igen
    if (mange) tegnListe();
    else if (skiftMaal && skiftMaal.id != null && skiftMaal.id !== enesteId()) visSkift(null); // rækken findes ikke mere
  }
  let listeNoegle = null;
  function tegnListe() {
    const fs = forbindelser(), afviste = R.afvist().ider;
    const noegle = fs.map(f => [f.id, R.vistNavn(f), f.domaener, afviste.includes(f.id)].join('\t')).join('\n') + '\n' + S.lang;
    if (noegle === listeNoegle && $('forb-liste').childElementCount) return;
    listeNoegle = noegle;
    const aaben = skiftMaal && skiftMaal.id != null ? skiftMaal.id : null;
    $('forb-liste').replaceChildren(...fs.map(f => {
      const navn = R.vistNavn(f), roed = afviste.includes(f.id), nId = `forb-${f.id}-navn`;
      const dom = el('button', { type: 'button', className: 'link forb-domaener', textContent: T.domaener(f.domaener) });
      dom.setAttribute('aria-label', fmt(T.forbDomaenerAria, { navn }));
      dom.addEventListener('click', () => aabnDomaener(f.id));
      const chipTekst = roed ? T.forbAfvist : T.forbForbundet, chip = window.hmTegn.chip(roed ? 'roed' : 'groen', chipTekst, 'button');
      chip.classList.add('forb-tilstand');
      if (roed) { chip.setAttribute('aria-label', fmt(T.forbSkiftAria, { navn })); chip.setAttribute('aria-controls', 'skift-form'); chip.addEventListener('click', () => visSkift({ id: f.id }, chip)); }
      else saetMaal(chip, { gaa: 'overblik' }, chipTekst);
      const omdoeb = window.Kit.knap({ tekst: T.forbOmdoeb, aria: fmt(T.forbOmdoebAria, { navn }), ved: () => aabnOmdoeb(f.id) });
      const skift = window.Kit.knap({ tekst: T.skift, aria: fmt(T.forbSkiftAria, { navn }) });
      skift.setAttribute('aria-controls', 'skift-form');
      skift.setAttribute('aria-expanded', String(aaben === f.id));
      skift.addEventListener('click', () => visSkift(skiftMaal && skiftMaal.id === f.id ? null : { id: f.id }, skift));
      const glem = window.Kit.knap({ tekst: T.forbGlem, variant: 'danger', aria: fmt(T.forbGlemAria, { navn }), ved: () => aabnGlem(f.id) });
      for (const [b, n] of [[omdoeb, 'omdoeb'], [skift, 'skift'], [glem, 'glem']]) b.id = `forb-${f.id}-${n}`;
      chip.id = `forb-${f.id}-chip`;
      dom.id = `forb-${f.id}-domaener`;
      const r = el('div', { className: 'raekke forb-raekke', role: 'listitem' }, el('span', { className: 'etik' }, el('span', { id: nId, className: 'forb-navn', textContent: navn }), el('small', {}, dom)), el('div', { className: 'raekke-knapper' }, chip, omdoeb, skift, glem));
      r.dataset.forbindelse = String(f.id);
      r.setAttribute('aria-labelledby', nId);
      return r;
    }));
    if (aaben != null) { const r = $('forb-liste').querySelector(`[data-forbindelse="${aaben}"]`); if (r) { r.after($('skift-form'), $('skift-status')); skiftKnap = $(`forb-${aaben}-skift`); } else visSkift(null); }
  }
  /* Formularen: maal { id } (Skift på den forbindelse) eller { ny: true } (Tilføj); null lukker. Den står under den række eller knap, den hører til. */
  function visSkift(maal, knap) {
    const form = $('skift-form'), st = $('skift-status');
    for (const b of document.querySelectorAll('[aria-controls="skift-form"][aria-expanded]')) b.setAttribute('aria-expanded', 'false');
    skiftMaal = maal;
    skiftKnap = maal ? knap || null : null;
    form.hidden = !maal;
    $('skift-felt').value = '';
    status(st, '');
    $('skift-note').hidden = !maal || Boolean(maal.ny);
    if (!maal) { $('forb-noegle-raekke').after(form, st); return; }
    if (maal.ny) $('forb-tilfoej-raekke').after(form, st);
    else if (flere()) { const r = $('forb-liste').querySelector(`[data-forbindelse="${maal.id}"]`); if (r) r.after(form, st); }
    else $('forb-noegle-raekke').after(form, st);
    const b = maal.ny ? $('forb-tilfoej') : flere() ? $(`forb-${maal.id}-skift`) : $('skift-noegle');
    if (b) b.setAttribute('aria-expanded', 'true');
    $('skift-felt').focus();
  }
  const lukSkift = () => { const b = skiftKnap; visSkift(null); if (b && b.isConnected) b.focus(); };
  $('skift-noegle').addEventListener('click', () => visSkift(skiftMaal && !skiftMaal.ny ? null : { id: enesteId() }, $('skift-noegle')));
  $('forb-chip').addEventListener('click', () => { if (!$('forb-chip').dataset.gaa && R.tilstand.sat && erAfvist(enesteId())) visSkift({ id: enesteId() }, $('forb-chip')); });
  $('forb-tilfoej').addEventListener('click', () => visSkift(skiftMaal && skiftMaal.ny ? null : { ny: true }, $('forb-tilfoej')));
  $('skift-annuller').addEventListener('click', lukSkift);
  $('skift-form').addEventListener('submit', async ev => {
    ev.preventDefault();
    const maal = skiftMaal;
    if (!maal) return;
    const id = await R.gemNoegle($('skift-felt'), $('skift-gem'), $('skift-status'), maal.ny ? null : maal.id);
    if (id === false) return;
    const st = $('skift-status'), tekst = st.textContent;
    visSkift(null);
    status(st, tekst, 'ok');
    /* En ny forbindelse, der ser mere end ét domæne: Domæner åbner ved dens gruppe, så man kan vælge (de vælges ikke af sig selv). */
    if (maal.ny && typeof id === 'number') { const f = forbindelser().find(x => x.id === id); if (f && f.domaener > 1) aabnDomaener(id); }
  });

  /* Omdøb: én dialog med ét felt. Tomt felt gemmes som null og viser Cloudflares navn igen. */
  let omdoebId = null;
  const omdoebFelt = window.Kit.felt({ id: 'omdoeb-felt', etiket: T.forbNavnFelt, hjaelp: T.forbNavnNote, foelsom: true });
  const omdoebDialog = R.dialog('omdoeb', document.body, { t: 'forbOmdoebH', indhold: omdoebFelt, fejl: true, udfyld: e => S.anvend(e),
    knapper: [{ id: 'nej', t: 'annuller' }, { id: 'gem', t: 'forbNavnGem', variant: 'primary', ved: async () => {
      const v = $('omdoeb-felt').value.trim();
      try { await invoke('forbindelse_omdoeb', { id: omdoebId, navn: v || null }); }
      catch (e) { throw S.fejl(e); }
      await R.opfrisk();
      status($('indst-status'), T.forbNavnGemt, 'ok');
      return true;
    } }] });
  $('omdoeb-felt').addEventListener('keydown', ev => { if (ev.key === 'Enter') { ev.preventDefault(); omdoebDialog.knap('gem').click(); } });
  function aabnOmdoeb(id) {
    const f = forbindelser().find(x => x.id === id) || { id };
    omdoebId = id;
    $('omdoeb-felt').value = f.eget_navn || '';
    $('omdoeb-felt').placeholder = f.cf_navn || fmt(T.forbStandard, { n: id });
    omdoebDialog.aabn();
  }
  $('forb-omdoeb').addEventListener('click', () => aabnOmdoeb(enesteId()));

  /* Kittets dialog lukker selv ved Annullér, Glem og Esc; her står kun det, Glem gør. Med én forbindelse: "Glem API Tokenet?" som i dag; med flere:
     forbindelsens navn i overskriften. Glem af den sidste er "Glem tokenet" fra 1.0.0 (Rust), og opsætningen vises. */
  const NEJ_JA = ja => [{ id: 'nej', t: 'annuller' }, { id: 'ja', t: ja, variant: 'danger' }];
  const glemDialog = R.dialog('glem', document.body, { t: 'glemH', p: { t: 'glemP' }, knapper: NEJ_JA('glemJa') }), dialog = glemDialog.rod;
  let glemId = null;
  function aabnGlem(id) {
    glemId = id;
    const en = !flere(), h = $('glem-h'), p = $('glem-p');
    h.dataset.t = 'glemH';
    p.dataset.t = 'glemP';
    if (en) { h.textContent = T.glemH; p.textContent = T.glemP; }
    else { delete h.dataset.t; delete p.dataset.t; h.textContent = fmt(T.glemForbH, { navn: navnPaa(id) }); p.textContent = T.glemForbP; }
    glemDialog.aabn();
  }
  $('glem').addEventListener('click', () => aabnGlem(enesteId()));
  $('glem-ja').addEventListener('click', async () => {
    const id = glemId, sidste = !flere();
    try {
      await invoke('forbindelse_glem', { id });
      if (sidste) { popup.luk(); await R.vis(); R.glemt(); }
      else { if (skiftMaal && skiftMaal.id === id) visSkift(null); await R.opfrisk(); tegnForbindelse(); status($('indst-status'), T.glemt, 'ok'); }
    } catch (e) { status($('indst-status'), S.fejl(e), 'fejl'); }
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
  const nulstilKit = R.dialog('nulstil', document.body, { t: 'nulstilH', p: '', knapper: NEJ_JA('nulstilJa') }), nulstilDialog = nulstilKit.rod;
  $('nulstil').addEventListener('click', async () => {
    let d = null;
    try { d = await invoke('data_status'); } catch (e) { /* ukendt */ }
    $('nulstil-p').textContent = nulstilLinjer(d).join(' ');
    nulstilKit.aabn();
  });
  $('nulstil-ja').addEventListener('click', async () => {
    try { await invoke('nulstil_historik'); status($('indst-status'), T.nulstilt, 'ok'); await R.hentOverblik(); }
    catch (e) { status($('indst-status'), S.fejl(e), 'fejl'); }
  });

  /* ── Om ── */
  async function visOm() {
    const v = window.__TAURI__.app ? await window.__TAURI__.app.getVersion().catch(() => null) : null;
    $('om-version').textContent = v ? fmt(T.omVersion, { v }) : '';
  }
  $('om-vejledning').setAttribute('aria-label', T.vejledningLabel);
  $('om-vejledning').addEventListener('click', () => R.aabnVejledning());
  if (KILDEKODE) { $('om-kilde').textContent = fmt(T.omKilde, { url: KILDEKODE }); $('om-kilde').hidden = false; }
  // data-t, så sprog.js' anvend() ikke skriver den almene tekst tilbage.
  if (R.mac) { for (const [id, k] of [['forb-note', 'forbNoteMac'], ['data-hist-note', 'dataHistNoteMac'], ['nulstil-note', 'nulstilNoteMac']]) { $(id).dataset.t = k; $(id).textContent = T[k]; } }

  /* ── popuppen, dialogerne og guiden ── */
  const LUK = [{ id: 'luk', t: 'luk' }];
  const husk = (lager, noegle, v) => { try { if (v === undefined) return lager.getItem(noegle); if (v === null) lager.removeItem(noegle); else lager.setItem(noegle, v); } catch (e) { /* gælder kun denne gang */ } return null; };
  const ryd = () => { visSkift(null); if (dialog.open) dialog.close(); if (omdoebDialog.aaben) omdoebDialog.luk(); if (nulstilDialog.open) nulstilDialog.close(); window.hmLand.glemTilbage(); };
  /* Et mål inde i en dialog (et domæne, chippen, Historik) lukker dialogerne, før rammen skifter visning. */
  const lukVedMaal = d => d.rod.addEventListener('click', ev => { if (ev.target.closest('[data-gaa], [data-dyk]')) d.luk(); }, true);

  const popupIndhold = el('div', { className: 'indst-popup' }, $('indst-status'), $('autostart-banner'), document.querySelector('#indst-dele .indst'));
  const popup = R.dialog('indst', document.body, { t: 'indstH', indhold: popupIndhold, bred: true, knapper: LUK, fokus: 'indst-h',
    vedLuk() { $('indst-knap').setAttribute('aria-expanded', 'false'); husk(sessionStorage, 'hm-indst-aaben', null); ryd(); } });
  lukVedMaal(popup);
  function aabnPopup() {
    if (guideDialog.aaben) return;
    tegnForbindelse();
    tegnData();
    $('indst-knap').setAttribute('aria-expanded', 'true');
    husk(sessionStorage, 'hm-indst-aaben', '1');
    popup.aabn();
    popupIndhold.closest('.modal-indhold').scrollTop = 0;
  }
  $('indst-knap').addEventListener('click', aabnPopup);

  const domDel = $('dom-del');
  const domDialog = R.dialog('domaener', document.body, { t: 'domaenerH', indhold: el('div', { id: 'dom-plads' }), bred: true, knapper: LUK, fokus: 'domaener-h' });
  lukVedMaal(domDialog);
  function tegnDomDel() { tegnDomaener(); }
  /* id: åbn ved den forbindelses gruppe (med flere forbindelser): gruppen rulles frem, og dens overskrift får fokus. */
  function aabnDomaener(id) {
    if (guideDialog.aaben) return;
    if (popup.aaben) popup.luk();
    $('dom-plads').append(domDel);
    tegnDomDel();
    status($('domaener-status'), '');
    const h = typeof id === 'number' && flere() ? $(`dom-gruppe-${id}-h`) : null;
    domDialog.aabn(h || undefined);
    if (h && h.scrollIntoView) h.scrollIntoView({ block: 'start' });
  }
  /* "Skift API Token…" fra banneret i Overblik: Indstillinger åbner med formularen på forbindelsens række. */
  function aabnSkift(id) {
    aabnPopup();
    if (!popup.aaben) return;
    const knap = flere() ? $(`forb-${id}-skift`) : $('skift-noegle');
    visSkift({ id }, knap);
    if ($('skift-form').scrollIntoView) $('skift-form').scrollIntoView({ block: 'center' });
  }

  /* Notifikationer pr. domæne: sin egen knap med klokken i båndet på Overblik (Jan 2026-11-13). */
  const beskedDialog = R.dialog('besked', document.body, { t: 'beskedH', indhold: $('besked-del'), bred: true, knapper: LUK, fokus: 'besked-h' });
  lukVedMaal(beskedDialog);
  function aabnBesked() {
    if (guideDialog.aaben) return;
    if (popup.aaben) popup.luk();
    prDomaeneNoegle = null;
    tegnPrDomaene();
    status($('besked-status'), '');
    beskedDialog.aabn();
  }

  const omDialog = R.dialog('om', document.body, { t: 'omH', indhold: $('om-del'), bred: true, knapper: LUK, fokus: 'om-h' });
  function aabnOm() { if (!guideDialog.aaben) omDialog.aabn(); }
  $('om-knap').addEventListener('click', aabnOm);

  /* Guiden. Trinenes indhold er noderne fra #guide-dele; trin 3 låner domænedelen, mens guiden står på det. */
  const TRIN = ['velkommen', 'token', 'domaener', 'besked'];
  /* KIT-ØNSKE K48: kittet tømmer pladsen ved hvert skift. Et trin, der ikke vises, og domænedelen lægges tilbage i de skjulte holdere, så
     kontrollerne altid findes i dokumentet (rammen melder domæner og token, også mens guiden står på et andet trin). */
  const trinNoder = TRIN.map(n => $('guide-trin-' + n)), venter = $('guide-domaener-venter');
  const guideRod = el('div', { id: 'guide' });
  /* KIT-ØNSKE K46: "Næste" kan ikke spærres af et trin (Kit.guide har ingen betingelse pr. trin); knappen slås fra her. */
  const guideNote = el('p', { className: 'note guide-spaerret', id: 'guide-spaerret', textContent: T.guideTokenFoerst, hidden: true });
  let wiz = null;
  const guideDialog = window.Kit.dialog({ guide: true, ariaLabelledby: 'guide-h', id: 'guide-dialog', indhold: guideRod, vedLuk() { husk(localStorage, 'hm-guide', 'vist'); } });
  document.body.append(guideDialog.rod);
  function guideTilstand() {
    if (!wiz) return;
    const frem = guideRod.querySelector('.fod .primary'), spaerret = wiz.aktuel === 1 && !R.tilstand.sat;
    frem.disabled = spaerret;
    if (spaerret) frem.setAttribute('aria-describedby', 'guide-spaerret'); else frem.removeAttribute('aria-describedby');
    guideNote.hidden = !spaerret;
    venter.hidden = !R.tilstand.sat || R.tilstand.domaener.length > 0;
  }
  wiz = window.Kit.guide(guideRod, {
    hId: 'guide-h',
    trin: TRIN.map((n, i) => ({ t: 'guideTrin' + (i + 1), h: 'guideH' + (i + 1), p: 'guideP' + (i + 1), indhold: trinNoder[i] })),
    naeste: { t: 'guideNaeste' }, tilbage: { t: 'guideTilbage' }, slut: { t: 'guideSlut' }, spring: { t: 'guideSpring' },
    vedSkift(i) {
      trinNoder.forEach((e, n) => { if (n !== i) $('guide-dele').append(e); });
      if (TRIN[i] === 'domaener') { trinNoder[i].append(domDel); tegnDomDel(); } else if (!domDialog.aaben) $('indst-dele').append(domDel);
      S.anvend(guideRod);
      guideRod.dataset.trin = String(i + 1);
      guideTilstand();
      const h = $('guide-h');
      if (h && guideDialog.aaben) h.focus();
    },
    vedSpring() { guideDialog.luk('spring'); },
    vedSlut() { guideDialog.luk('slut'); if (R.tilstand.sat) R.gaaTil('overblik'); },
  });
  trinNoder[1].append(guideNote);
  for (const b of guideRod.querySelectorAll('.fod button')) b.id = 'guide-' + (b.classList.contains('primary') ? 'naeste' : b.classList.contains('ghost') ? 'spring' : 'tilbage');
  function aabnGuide() {
    for (const d of [popup, domDialog, beskedDialog, omDialog]) if (d.aaben) d.luk();
    wiz.gaaTil(0);
    guideDialog.aabn();
  }
  $('guide-igen').addEventListener('click', aabnGuide);

  /* Den tomme tilstand bag guiden, når der ikke er et API Token. */
  $('opsaet-tom').append(window.Kit.tomTilstand({ ikon: 'laas', t: 'opsaetTomH', niveau: 2, p: { t: 'opsaetTomP' }, handling: { t: 'opsaetTomKnap', variant: 'primary', ved: aabnGuide } }));
  $('opsaet-tom').querySelector('button').id = 'opsaet-guide';
  S.anvend($('opsaet-tom'));

  R.paa('raekker', () => { tegnPrDomaene(); tegnForbindelse(); if (popup.aaben) tegnData(); });
  R.paa('forbindelser', () => { tegnForbindelse(); tegnDomaener(); });
  R.paa('domaener', () => { tegnDomaener(); guideTilstand(); });
  let foersteToken = true;
  R.paa('token', sat => {
    tegnForbindelse();
    guideTilstand();
    /* Første gang programmet står uden API Token, og efter "Glem API Token…": guiden åbner af sig selv, hvis den ikke er vist. */
    if (!sat && !guideDialog.aaben && (husk(localStorage, 'hm-guide') !== 'vist' || !foersteToken)) setTimeout(() => { if (!R.tilstand.sat && !window.hmIndst.aaben()) aabnGuide(); }, 0);
    foersteToken = false;
  });
  R.paa('koe', tegnForbindelse);
  const ALLE = [popup, domDialog, beskedDialog, omDialog, guideDialog];
  window.hmIndst = { aabn: aabnPopup, domaener: aabnDomaener, skift: aabnSkift, besked: aabnBesked, om: aabnOm, guide: aabnGuide, aaben: () => ALLE.some(d => d.aaben), lukAlle() { for (const d of ALLE) if (d.aaben) d.luk(); } };
  if (husk(sessionStorage, 'hm-indst-aaben')) setTimeout(aabnPopup, 0); // efter et skift af sprog eller datoformat
  visAutostart();
  hentNotif();
  visOm();
})();
