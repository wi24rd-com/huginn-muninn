/* Huginn and Muninn — #teknologi: hvad sitet bruger, for alle domæner på én side. #teknologi?zone=<id>&navn=… sætter fokus på domænets række, og fane=alle åbner kataloget. Alt ligger i Rust (teknologi_side, kun fra cachen).
   "Pr. domæne": rækker = valgte domæner, kolonner = det, der er krydset af under "Vis i tabellen". Domænet og overskriften er fastlåst, og tabellen ruller i sin egen boks. Hver celle er én radiogruppe med tre små radioknapper.
   Tastatur: tabellen er ét Tab-stop (J5). Piletasterne flytter mellem cellerne, mellemrum skifter valg, og Enter går ind i gruppen, hvor piletasterne vælger, til Esc. Tal, "Giv mig besked" og handlingerne er almindelige Tab-stop. Hver kolonne har ét: knappen "⋯", der viser kolonnens handlinger, til Esc.
   "Alle teknologier": kataloget i grupper, med søgning og antal angreb på kontoen. At skjule en kolonne ændrer intet valg: de gælder stadig i status, og angrebene står stadig i tal, lister og på kortet. Bygges med createElement. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, R = window.hmRamme, V = window.hmTegn;
  const $ = id => document.getElementById(id);
  const { el, fmt, invoke } = R, tal = S.tal;
  const rod = $('teknologi-rod');
  const VALG = [['bruger', '✓', 'tekKortBruger'], ['bruger_ikke', '✕', 'tekKortIkke'], ['ved_ikke', '?', 'tekVedIkke']];
  const GRUPPER = ['cms', 'ramme', 'server', 'samarbejde', 'netvaerk', 'andet'];
  let d = null, params = new URLSearchParams(), soeg = '', gen = 0, inde = null, sidste = '';

  const alleFane = () => params.get('fane') === 'alle';
  const passer = k => !soeg || k.navn.toLowerCase().includes(soeg.toLowerCase());
  const valgAf = (dom, tek) => dom.valg[tek] || ['ved_ikke', false];
  const fane = v => kopi(p => { if (v) p.set('fane', v); else p.delete('fane'); });
  const kopi = fn => { const p = new URLSearchParams(params); fn(p); return p.toString(); };
  const dykZone = (dom, tek) => new URLSearchParams({ zone: dom.zone, navn: dom.navn, teknologi: tek, slags: 'angreb' }).toString();
  const statusLinje = el('p', { className: 'r6-status', id: 'tek-side-status' });
  statusLinje.setAttribute('role', 'status');
  const besked = (tekst, klasse) => R.status(statusLinje, tekst, klasse);

  /* Status følger valget: det, der er gemt, tegnes igen på Overblik. */
  async function gem(dom, tek, valg, bedt) {
    try {
      await invoke('teknologi_saet', { zone: dom.zone, tek, valg, besked: Boolean(bedt) });
      if (valg === 'ved_ikke') delete dom.valg[tek]; else dom.valg[tek] = [valg, Boolean(bedt)];
      besked(T.tekGemt, 'ok');
      R.hentOverblik();
      return true;
    } catch (e) { besked(S.fejl(e), 'fejl'); return false; }
  }
  async function handling(fn) {
    try { await fn(); await hent(); R.hentOverblik(); } catch (e) { besked(S.fejl(e), 'fejl'); }
  }
  const smaaKnap = (tekst, label, fn) => { const b = el('button', { type: 'button', className: 'link tek-lille', textContent: tekst }); b.setAttribute('aria-label', label); b.addEventListener('click', fn); return b; };

  /* Kolonnens handlinger bag én knap, så en kolonne er ét Tab-stop. De står i overskriften og ikke i en svævende menu, for tabellens rulleboks ville klippe den. Kun én er åben ad gangen. */
  function kolMenu(k) {
    const knap = el('button', { type: 'button', className: 'tek-kol-knap', textContent: '⋯' });
    const aabn = ja => {
      rod.querySelectorAll('.tek-kol-knap').forEach(b => { b.setAttribute('aria-expanded', 'false'); b.nextElementSibling.hidden = true; });
      knap.setAttribute('aria-expanded', String(ja));
      menu.hidden = !ja;
    };
    /* Tabellen tegnes om efter handlingen: fokus tilbage på kolonnens knap. */
    const gem = valg => async () => {
      await handling(() => invoke('teknologi_kolonne', { tek: k.tek, valg }));
      [...rod.querySelectorAll('.tek-kol-knap')].find(b => b.dataset.tek === k.tek)?.focus();
    };
    const menu = el('div', { className: 'tek-kol-menu', hidden: true }, smaaKnap(T.tekIkkeAlle, fmt(T.tekIkkeAlleFor, { t: k.navn }), gem('bruger_ikke')), smaaKnap(T.tekNulKolonne, fmt(T.tekNulKolonneFor, { t: k.navn }), gem('ved_ikke')));
    knap.dataset.tek = k.tek;
    knap.setAttribute('aria-label', fmt(T.tekKolMenu, { t: k.navn }));
    knap.setAttribute('aria-expanded', 'false');
    knap.addEventListener('click', () => {
      const ja = menu.hidden;
      aabn(ja);
      if (ja) menu.querySelector('button').focus();
    });
    const luk = ev => { if (ev.key !== 'Escape' || menu.hidden) return; ev.preventDefault(); ev.stopPropagation(); aabn(false); knap.focus(); };
    knap.addEventListener('keydown', luk);
    menu.addEventListener('keydown', luk);
    return [knap, menu];
  }

  /* ── "Pr. domæne" ── */
  function celle(dom, k) {
    const [v, bedt] = valgAf(dom, k.tek), set = k.tek in dom.antal, n = dom.antal[k.tek] || 0;
    const gruppe = el('div', { className: 'tek-radio' });
    gruppe.setAttribute('role', 'radiogroup');
    gruppe.setAttribute('aria-label', `${dom.navn}, ${k.navn}`);
    const td = el('td', { className: 'tek-c' + (set ? '' : ' tek-ikke-set') }, gruppe);
    td.dataset.tek = k.tek;
    for (const [val, tegn, tekst] of VALG) {
      const i = el('input', { type: 'radio', name: `tek-${dom.zone}-${k.tek}`, value: val, checked: v === val, tabIndex: -1 });
      i.setAttribute('aria-label', T[tekst]);
      i.addEventListener('change', async () => {
        if (!i.checked || !await gem(dom, k.tek, val, false)) return;
        const ny = celle(dom, k), fokus = document.activeElement === i;
        td.replaceWith(ny);
        if (fokus) fokusCelle(ny);
      });
      gruppe.append(el('label', { title: T[tekst] }, i, el('span', { textContent: tegn })));
    }
    if (n) td.append(V.knap(tal(n), dykZone(dom, k.tek), 'link tek-n', fmt(T.tekCelleAntal, { n: tal(n), t: k.navn, navn: dom.navn })));
    if (v === 'bruger') {
      const boks = el('input', { className: 'boks', type: 'checkbox', checked: bedt });
      boks.setAttribute('aria-label', `${T.tekBesked}: ${dom.navn}, ${k.navn}`);
      boks.title = fmt(T.tekBeskedNote, { t: k.navn });
      boks.addEventListener('change', () => gem(dom, k.tek, 'bruger', boks.checked));
      td.append(boks);
    }
    return td;
  }
  function fokusCelle(td) {
    rod.querySelectorAll('.tek-radio input').forEach(i => { i.tabIndex = -1; });
    const r = td.querySelector('input:checked');
    r.tabIndex = 0;
    r.focus();
  }
  function tastatur(ev) {
    const i = ev.target, td = i.closest && i.closest('td.tek-c');
    if (!td || i.type !== 'radio' || ev.metaKey || ev.ctrlKey || ev.altKey) return;
    const noegle = `${td.parentElement.dataset.zone}\t${td.dataset.tek}`, radioer = [...td.querySelectorAll('.tek-radio input')], skift = n => { const r = radioer[(radioer.indexOf(i) + n + radioer.length) % radioer.length]; r.focus(); r.click(); };
    const pil = { ArrowRight: [0, 1], ArrowLeft: [0, -1], ArrowDown: [1, 0], ArrowUp: [-1, 0] }[ev.key];
    /* Nøglen og ikke elementet: cellen tegnes om efter hvert valg. */
    if (inde === noegle) {
      if (ev.key === 'Escape') { ev.preventDefault(); ev.stopPropagation(); inde = null; besked(''); return; }
      if (ev.key === 'Tab') { inde = null; return; }
      if (pil) { ev.preventDefault(); skift(pil[0] + pil[1]); }
      return;
    }
    if (ev.key === ' ') { ev.preventDefault(); skift(1); return; }
    if (ev.key === 'Enter') { ev.preventDefault(); inde = noegle; besked(T.tekIndeGruppe); return; }
    if (!pil) return;
    ev.preventDefault();
    const tr = td.parentElement, rk = [...tr.parentElement.rows], ny = rk[rk.indexOf(tr) + pil[0]]?.cells[td.cellIndex + pil[1]];
    if (ny && ny.classList.contains('tek-c')) fokusCelle(ny);
  }
  function tabel() {
    const kolonner = d.katalog.filter(k => k.vis && passer(k));
    if (!d.katalog.some(k => k.vis)) return el('p', { className: 'vg-status', textContent: T.tekIngenKolonner });
    if (!kolonner.length) return el('p', { className: 'vg-status', textContent: T.tekIngenTraef });
    const hoved = el('tr', {}, el('th', { scope: 'col', textContent: T.thDomaene }), ...kolonner.map(k => el('th', { scope: 'col' }, el('span', { className: 'tek-kol', textContent: k.navn }), ...kolMenu(k))));
    const raekker = d.domaener.map(dom => {
      const th = el('th', { scope: 'row' }, V.domaene(dom.navn),
        smaaKnap(T.tekNulDomaene, fmt(T.tekNulDomaeneFor, { navn: dom.navn }), async () => { if (await R.bekraeft(fmt(T.bekraeftTekP, { d: dom.navn }))) handling(() => invoke('teknologi_nulstil', { zone: dom.zone })); }),
        smaaKnap(T.tekKopier, fmt(T.tekKopierFor, { navn: dom.navn }), () => handling(() => invoke('teknologi_alle', { zone: dom.zone }))));
      const tr = el('tr', { className: dom.zone === params.get('zone') ? 'tek-fokus' : '' }, th, ...kolonner.map(k => celle(dom, k)));
      tr.dataset.zone = dom.zone;
      return tr;
    });
    const tb = el('table', { className: 'tek-gitter' }, el('thead', {}, hoved), el('tbody', {}, ...raekker));
    tb.addEventListener('keydown', tastatur);
    const rul = el('div', { className: 'tek-rul' }, tb);
    rul.setAttribute('role', 'region');
    rul.setAttribute('aria-label', T.tekSideH);
    const start = (raekker.find(x => x.dataset.zone === params.get('zone')) || raekker[0]).querySelector('td.tek-c input:checked');
    if (start) start.tabIndex = 0;
    return rul;
  }

  /* ── "Alle teknologier" ── */
  function linje(k) {
    const boks = el('input', { className: 'boks', type: 'checkbox', checked: k.vis });
    boks.setAttribute('aria-label', fmt(T.tekVisFor, { t: k.navn }));
    boks.addEventListener('change', async () => {
      try { await invoke('teknologi_vis_saet', { tek: k.tek, vis: boks.checked }); k.vis = boks.checked; k.ny = false; besked(T.tekGemt, 'ok'); } catch (e) { boks.checked = !boks.checked; besked(S.fejl(e), 'fejl'); }
    });
    const antal = k.angreb ? V.knap(fmt(T.tekKontoAntal, { n: tal(k.angreb) }), new URLSearchParams({ samlet: '1', teknologi: k.tek, slags: 'angreb' }).toString(), 'link tek-n') : el('span', { className: 'note', textContent: k.set ? fmt(T.tekKontoAntal, { n: tal(0) }) : T.tekIkkeSetKonto });
    const li = el('li', { className: 'tek-katalog-linje' }, el('label', { className: 'hm-valgmulighed' }, boks, el('span', { textContent: k.navn })), antal, smaaKnap(T.tekIkkeAlle, fmt(T.tekIkkeAlleFor, { t: k.navn }), () => handling(() => invoke('teknologi_kolonne', { tek: k.tek, valg: 'bruger_ikke' }))));
    li.dataset.tek = k.tek;
    return li;
  }
  function tilfoej() {
    const felt = el('input', { type: 'text', autocomplete: 'off', spellcheck: false, autocapitalize: 'off' }), liste = el('datalist', { id: 'tek-navne' }, ...d.katalog.map(k => el('option', { value: k.navn })));
    felt.setAttribute('list', 'tek-navne');
    const form = el('form', { className: 'hm-form login-egen' }, el('label', {}, el('span', { textContent: T.tekTilfoej }), felt), liste, el('button', { className: 'knap', type: 'submit', textContent: T.tekTilfoejKnap }));
    form.addEventListener('submit', async ev => {
      ev.preventDefault();
      try {
        const tek = await invoke('teknologi_tilfoej', { navn: felt.value.trim() });
        await hent();
        besked(fmt(T.tekTilfoejet, { t: S.tekNavn(tek) }), 'ok');
      } catch (e) { besked(e && e.kode === 'ugyldigt_filter' ? T.tekTilfoejFejl : S.fejl(e), 'fejl'); }
    });
    return [form, el('p', { className: 'note', textContent: T.tekTilfoejNote })];
  }
  function katalog() {
    const grupper = GRUPPER.map(g => [g, d.katalog.filter(k => k.gruppe === g && passer(k))]).filter(x => x[1].length);
    if (!grupper.length) return [el('p', { className: 'vg-status', textContent: T.tekIngenTraef })];
    return grupper.map(([g, rk]) => el('section', { className: 'tek-gruppe' }, el('h2', { textContent: T.tekGruppe[g] }), el('ul', { className: 'tek-katalog' }, ...rk.map(linje))));
  }

  /* En ny teknologi er allerede krydset af i Rust. Linjen er diskret og forsvinder, når den er besvaret. */
  function nyeLinjer() {
    return d.katalog.filter(k => k.ny).map(k => {
      const svar = vis => handling(() => invoke('teknologi_vis_saet', { tek: k.tek, vis }));
      return el('p', { className: 'tek-ny' }, fmt(T.tekNy, { t: k.navn }), ' ', smaaKnap(T.tekNyJa, fmt(T.tekNyJaFor, { t: k.navn }), () => svar(true)), ' · ', smaaKnap(T.tekNyNej, fmt(T.tekNyNejFor, { t: k.navn }), () => svar(false)));
    });
  }
  function indhold() { return alleFane() ? katalog() : [tabel()]; }
  /* Rammen melder domæner ved start, og en handling henter siden igen: fokus bliver i samme celle. */
  function tegn() {
    const aktiv = document.activeElement, td = aktiv && aktiv.closest ? aktiv.closest('td.tek-c') : null, husk = td ? [td.parentElement.dataset.zone, td.dataset.tek] : null;
    const felt = el('input', { type: 'search', value: soeg, autocomplete: 'off', spellcheck: false });
    felt.setAttribute('aria-label', T.tekSoeg);
    felt.placeholder = T.tekSoeg;
    const plads = el('div', { className: 'tek-indhold' });
    felt.addEventListener('input', () => { soeg = felt.value.trim(); plads.replaceChildren(...indhold()); });
    $('teknologi-faner').replaceChildren(V.segment('tek-fane', T.tekFaner, [[T.tekFanePr, fane(''), !alleFane()], [T.tekFaneAlle, fane('alle'), alleFane()]], 'teknologi'));
    rod.replaceChildren(...nyeLinjer(), el('p', { className: 'vg-intro', textContent: alleFane() ? T.tekGruppeNote : T.tekForkl }), el('p', { className: 'tek-soeg' }, felt), ...(alleFane() ? tilfoej() : []), statusLinje, plads);
    plads.replaceChildren(...indhold());
    const igen = husk && [...rod.querySelectorAll('td.tek-c')].find(c => c.parentElement.dataset.zone === husk[0] && c.dataset.tek === husk[1]);
    if (igen) fokusCelle(igen);
  }
  /* Fra "Ret teknologier": domænets række i fokus. */
  function fokusDomaene() {
    const tr = [...rod.querySelectorAll('tr[data-zone]')].find(x => x.dataset.zone === params.get('zone')), td = tr && tr.querySelector('td.tek-c');
    if (!tr) return;
    if (tr.scrollIntoView) tr.scrollIntoView({ block: 'center' });
    if (td) fokusCelle(td);
  }
  async function hent() {
    const g = gen;
    let ny;
    try { ny = await invoke('teknologi_side'); } catch (e) { if (g === gen) R.status(statusLinje, S.fejl(e), 'fejl'), rod.replaceChildren(statusLinje); return; }
    if (g !== gen) return;
    d = ny;
    rod.removeAttribute('aria-busy');
    if (!d.domaener.length) { $('teknologi-faner').replaceChildren(); rod.replaceChildren(window.hmVelkomst.intetValgt()); return; }
    tegn();
  }
  R.paa('domaener', () => { if (R.tilstand.visning === 'teknologi') hent(); });
  R.registrer('teknologi', {
    async vis(p, { samme } = {}) {
      if (samme && sidste === p.toString()) return;
      gen++;
      params = p;
      sidste = p.toString();
      inde = null;
      besked('');
      rod.setAttribute('aria-busy', 'true');
      await hent();
      if (p.get('zone') && !alleFane() && !R.genbesoeg()) fokusDomaene();
    },
    skjul() { gen++; sidste = ''; },
  });
})();
