/* Huginn and Muninn — det, Angreb (dyk er en del af Angreb siden V-ANGREB.1) tegner: kortet, søjlerne, listerne, navnene på lande, datacentre og regler. Alt bygges
   med createElement/createElementNS: ingen innerHTML og ingen style-attributter. Tekst står aldrig i SVG'en, men som HTML oven på
   (etiketter og akser), så den har --font-size-xs og følger Tekststørrelse uanset, hvordan tegningen skaleres; søjlerne strækkes
   med preserveAspectRatio="none". Etiketternes plads og listernes bredde er tal, der først kendes her: de står som regler i et eget
   stylesheet (plads, bredde), ikke på elementet. Et mærke med data bag får data-dyk, og tegningen er ét Tab-stop: rammen flytter med
   piletasterne. Et mærke uden data bag får ingen handling. Klikflader er mindst 24 px: en søjle er hele dagens flade, og søjlerne
   ruller vandret, hvis der ikke er plads til det; en ring eller rude har en usynlig flade (.dyk-hit) under alle mærker, så et lille
   mærke kan rammes, uden at den tager klik fra naboen. */
(function () {
  'use strict';
  const R = window.hmRamme, S = window.hmSprog, T = S.T, { el, fmt } = R;
  const NS = 'http://www.w3.org/2000/svg';
  const tal = S.tal, loc = S.lang === 'da' ? 'da-DK' : 'en-GB';
  function svg(navn, attr, ...boern) {
    const e = document.createElementNS(NS, navn);
    for (const [k, v] of Object.entries(attr || {})) if (v !== undefined && v !== null) e.setAttribute(k, v);
    e.append(...boern);
    return e;
  }
  const titel = tekst => svg('title', {}, tekst);

  /* ── tal, der kun kendes ved tegning: regler i et eget stylesheet ── */
  const ark = new CSSStyleSheet();
  document.adoptedStyleSheets = [...(document.adoptedStyleSheets || []), ark];
  const klasser = new Map();
  function klasse(regel) {
    if (!klasser.has(regel)) {
      const navn = `vg-t${klasser.size}`;
      ark.insertRule(`.${navn} { ${regel} }`);
      klasser.set(regel, navn);
    }
    return klasser.get(regel);
  }
  const bredde = p => klasse(`width: ${Math.round(p)}%;`);
  const plads = (x, y) => klasse(`left: ${x.toFixed(1)}%; top: ${y.toFixed(1)}%;`);

  /* ── navne ── */
  let landNavn = k => k;
  try { const dn = new Intl.DisplayNames([S.lang], { type: 'region' }); landNavn = k => { try { return dn.of(k) || k; } catch (e) { return k; } }; } catch (e) { }
  const SAERLIGE = { T1: T.dykTor, XX: T.dykUkendtLand }; // Cloudflares koder for Tor og ukendt; Intl kender dem ikke
  const land = k => SAERLIGE[k] || landNavn(k);
  let verden = null, colonavne = {}, tidszoner = {};
  const xyColo = k => (verden && (verden.colo[k] || (verden.net[k] && verden.net[k].slice(0, 2)))) || null;
  const colo = k => { const n = colonavne[k]; if (Array.isArray(n)) return `${n[S.lang === 'da' ? 0 : 1]} (${k})`; if (verden && verden.net[k]) return `${verden.net[k][2]} (${k})`; return k; };
  /* Kortdata hentes én gang, første gang en visning skal bruge dem. */
  async function klar() {
    if (verden) return;
    const k = await R.invoke('kortdata');
    verden = k.verden;
    colonavne = k.colo || {};
    tidszoner = (k.tidszoner && k.tidszoner.zoner) || {};
  }
  const { dato, datoKort, datoLang, datoTid } = S;
  function typeTekst(navn, type) { for (const [re, t] of T.type) if (re.test(type || '') || re.test(navn || '')) return t; return type || ''; }
  const stort = s => String(s).charAt(0).toUpperCase() + String(s).slice(1);
  /* Hvad en regel hedder: mærket og overskriften på regelkortet, klarsprog i dyk (titel) og bogens afsnit (id). */
  function regelVis(rg) {
    const navn = rg.navn;
    if (rg.art === 'rutine') return { id: 'rutine', maerke: T.rutine, overskrift: S.rutineNavn(navn), titel: S.rutineNavn(navn) };
    if (rg.art === 'egen') { const t = stort(S.klar(navn, true).tekst); return { id: 'egen', maerke: t, overskrift: navn, titel: t }; }
    const k = S.klar(navn), type = typeTekst(navn, rg.type);
    return { id: k.id, maerke: rg.produkt || String(navn).split(' - ')[0], overskrift: type || navn, titel: k.id === 'andet' ? (type || navn) : stort(k.tekst) };
  }

  /* Klarsprog for en regel i lister og tabeller. art fra Rust: "egen" står med kort navn, og rutine har sit eget navn. */
  function regelTitel(v, art) {
    if (art === 'egen') return stort(S.klar(v, true).tekst);
    if (S.rutineNavn(v) !== v) return S.rutineNavn(v);
    const k = S.klar(v);
    return k.id === 'andet' ? v : stort(k.tekst);
  }

  /* Hvad en robot hedder. Rust gemmer Cloudflares kategori som navn, når brugeragenten ikke siger mere. */
  function robotNavn(navn) {
    const k = /^Verificeret \((.+)\)$/.exec(navn);
    return k ? fmt(T.robotVerificeret, { k: k[1] }) : navn === 'AI-assistent i en browser' ? T.robotAssistent : navn === 'Ukendt bot' ? T.robotUkendt : navn;
  }
  /* Filteret robot: alle, en slags (udklaedt, uverificeret, stoppet, labyrint), en slags med ":navn" eller et navn. */
  const ROBOT_SLAGS = ['alle', 'udklaedt', 'uverificeret', 'stoppet', 'labyrint'];
  function robotValg(v) {
    const i = String(v).indexOf(':'), slags = i < 0 ? v : String(v).slice(0, i);
    if (!ROBOT_SLAGS.includes(slags)) return robotNavn(v);
    return i < 0 ? T.robotSlags[slags] : `${T.robotSlags[slags]}: ${robotNavn(String(v).slice(i + 1))}`;
  }

  /* ── byggeklodser ── */
  /* Et afsnit med overskrift og måske en indledning. */
  function blok(h, p, ...indhold) {
    const h2 = el('h2', { textContent: h }), s = el('section', { className: 'vg-blok' }, h2);
    if (p) s.append(el('p', { className: 'vg-intro', textContent: p }));
    s.append(...indhold);
    return s;
  }
  /* Kittets segmentknap (Kit.segment); et valg er ét trin i tilbage/frem. Fokus bliver på valget, når siden tegnes om. Intet er
     valgt (valgt: null), når et eget interval gælder. */
  function segment(navn, etiket, valg, visning) {
    const valgt = valg.findIndex(v => v[2]);
    const s = window.Kit.segment({ aria: etiket, valg: valg.map(([tekst], i) => ({ id: String(i), tekst })), valgt: valgt < 0 ? null : String(valgt), vedValg: id => R.gaaTil(visning, valg[Number(id)][1]) });
    s.knapper.forEach((b, i) => { b.dataset.fokus = `${navn}:${valg[i][0]}`; });
    s.rod.dataset.navn = navn;
    return s.rod;
  }
  /* Status som kittets StatusChip (Kit.chip): roed, gul, groen og graa er programmets ord for kittets danger, warn, ok og
     none. tag 'button' giver en chip, der kan klikkes (.chip-knap). */
  const CHIP = { roed: 'danger', gul: 'warn', groen: 'ok', graa: 'none' };
  function chip(status, ord, tag) {
    const k = window.Kit.chip(CHIP[status] || 'none', ord);
    if (tag !== 'button') return k;
    const b = el('button', { type: 'button', className: k.className + ' chip-knap' });
    b.append(...k.childNodes);
    return b;
  }
  /* To datoer som chip: "1.–4. november" i samme måned, ellers den lange form. */
  function interval(a, b) {
    if (a === b) return datoLang(a);
    if (a.slice(0, 7) !== b.slice(0, 7) || S.datoValg !== 'sprog' || datoLang(b) !== datoLang(b).replace(/ \d{4}$/, '')) return S.periode(a, b);
    return `${datoLang(a).replace(/[^\d.]*$/, '')}–${datoLang(b)}`;
  }
  /* En lille kalender med tokens: WebKits <input type="date"> følger systemets region og ikke programmets sprog og Datoformat.
     Tastatur: piletasterne flytter en dag eller en uge, PageUp/PageDown en måned, Home/End ugens start og slut, Enter eller mellemrum vælger, og Esc lukker. o: dag, min, max, etiket, vaelg(dag) og luk(). */
  const flytDag = (d, n) => new Date(Date.parse(`${d}T00:00:00Z`) + n * 864e5).toISOString().slice(0, 10);
  const ugedag = d => (new Date(`${d}T00:00:00Z`).getUTCDay() + 6) % 7;
  function kalender(o) {
    const boks = el('div', { className: 'kal' });
    boks.setAttribute('role', 'dialog');
    boks.setAttribute('aria-label', o.etiket);
    let fokus = o.dag, vist = o.dag.slice(0, 7);
    const iOrden = d => (!o.min || d >= o.min) && (!o.max || d <= o.max);
    const maaned = delta => { const [y, m] = vist.split('-').map(Number); return new Date(Date.UTC(y, m - 1 + delta, 1)).toISOString().slice(0, 7); };
    const navKnap = (tekst, label, delta) => { const b = el('button', { type: 'button', className: 'kal-nav', textContent: tekst }); b.setAttribute('aria-label', label); b.addEventListener('click', ev => { ev.stopPropagation(); vist = maaned(delta); tegnKal(null); }); return b; };
    const flyt = n => {
      const ny = flytDag(fokus, n);
      if (iOrden(ny)) { fokus = ny; vist = ny.slice(0, 7); tegnKal(ny); }
    };
    const taster = { ArrowLeft: () => flyt(-1), ArrowRight: () => flyt(1), ArrowUp: () => flyt(-7), ArrowDown: () => flyt(7), PageUp: () => flyt(-30), PageDown: () => flyt(30), Home: () => flyt(-ugedag(fokus)), End: () => flyt(6 - ugedag(fokus)) };
    function tegnKal(fokuser) {
      const [y, m] = vist.split('-').map(Number), foerste = new Date(Date.UTC(y, m - 1, 1)), dage = new Date(Date.UTC(y, m, 0)).getUTCDate(), start = ugedag(`${vist}-01`), loc2 = S.lang === 'da' ? 'da-DK' : 'en-GB';
      const hoved = el('div', { className: 'kal-hoved' }, navKnap('‹', T.kalForrige, -1), el('b', { textContent: foerste.toLocaleDateString(loc2, { month: 'long', year: 'numeric', timeZone: 'UTC' }) }), navKnap('›', T.kalNaeste, 1));
      const uge = [...Array(7)].map((_, i) => { const th = el('th', { textContent: new Date(Date.UTC(2026, 10, 2 + i)).toLocaleDateString(loc2, { weekday: 'short', timeZone: 'UTC' }) }); th.setAttribute('scope', 'col'); return th; });
      const raekker = [];
      let tr = el('tr', {}, ...[...Array(start)].map(() => el('td')));
      for (let i = 1; i <= dage; i++) {
        const dag = `${vist}-${String(i).padStart(2, '0')}`, b = el('button', { type: 'button', className: 'kal-dag' + (dag === o.dag ? ' valgt' : ''), textContent: String(i), disabled: !iOrden(dag), tabIndex: dag === fokus ? 0 : -1 });
        b.dataset.dag = dag;
        b.setAttribute('aria-label', dato(dag));
        if (dag === o.dag) b.setAttribute('aria-current', 'date');
        b.addEventListener('click', () => o.vaelg(dag));
        tr.append(el('td', {}, b));
        if ((start + i) % 7 === 0) { raekker.push(tr); tr = el('tr'); }
      }
      if (tr.childElementCount) raekker.push(tr);
      boks.replaceChildren(hoved, el('table', { className: 'kal-tabel' }, el('thead', {}, el('tr', {}, ...uge)), el('tbody', {}, ...raekker)));
      if (fokuser) boks.querySelector(`[data-dag="${fokuser}"]`)?.focus();
    }
    boks.addEventListener('keydown', ev => {
      if (ev.key === 'Escape') { ev.preventDefault(); ev.stopPropagation(); o.luk(); return; }
      if (taster[ev.key]) { ev.preventDefault(); taster[ev.key](); }
    });
    tegnKal(null);
    return boks;
  }
  /* Fra og Til som tekstknapper i det valgte datoformat, der åbner kalenderen. Tallene gemmes pr. døgn, så der er intet klokkeslæt. Felterne viser altid det aktive udsnit: p.fra og p.til fra Rust (dagen, de 7 eller 30 dage, intervallet eller hele historikken).
     o: periode (Rusts PeriodeUd), gaa(fra, til), nulstil (query til hele historikken) og visning. Fra efter til giver en fejltekst og ingen hentning. */
  function datoVaelger(o) {
    const p = o.periode, valgt = { fra: p.fra, til: p.til }, fejl = el('p', { className: 'vg-advarsel vg-roed dato-fejl', textContent: T.datoFejl, hidden: true });
    fejl.setAttribute('role', 'alert');
    let aaben = null;
    const luk = fokus => {
      if (!aaben) return;
      const { boks, knap: b, udenfor } = aaben;
      aaben = null;
      boks.remove();
      b.setAttribute('aria-expanded', 'false');
      document.removeEventListener('click', udenfor, true);
      if (fokus) b.focus();
    };
    const felt = (navn, tekst) => {
      const b = el('button', { type: 'button', className: 'dato-knap' }), boksen = el('div', { className: 'dato-felt' }, el('span', { textContent: tekst }), b);
      /* o.tom (en fast periode er valgt): feltet står tomt, til brugeren vælger en dato; kalenderen åbner stadig på periodens dag. */
      const vis = tom => { b.textContent = tom ? '–' : dato(valgt[navn]); b.dataset.dag = tom ? '' : valgt[navn]; b.setAttribute('aria-label', tom ? fmt(T.kalAabnTom, { felt: tekst }) : fmt(T.kalAabn, { felt: tekst, d: dato(valgt[navn]) })); };
      vis(Boolean(o.tom));
      b.dataset.fokus = `dato:${navn}`;
      b.setAttribute('aria-haspopup', 'dialog');
      b.setAttribute('aria-expanded', 'false');
      b.addEventListener('click', () => {
        if (aaben && aaben.knap === b) { luk(true); return; }
        luk(false);
        const vaelg = dag => {
          valgt[navn] = dag;
          vis();
          luk(true);
          fejl.hidden = valgt.fra <= valgt.til;
          if (fejl.hidden) o.gaa(valgt.fra, valgt.til);
        };
        const boks = kalender({ dag: valgt[navn], min: p.data_fra || '', max: p.i_dag, etiket: tekst, luk: () => luk(true), vaelg });
        const udenfor = ev => { if (!boks.contains(ev.target) && ev.target !== b) luk(false); };
        aaben = { boks, knap: b, udenfor };
        boksen.append(boks);
        b.setAttribute('aria-expanded', 'true');
        document.addEventListener('click', udenfor, true);
        boks.querySelector('[tabindex="0"]:not([disabled])')?.focus();
      });
      return boksen;
    };
    /* Fra og Til står i båndets gruppe "Egne datoer" (o.plads, V-FANER.1); noten og en fejl står i visningen. Hele historikken er et valg i Periode. */
    R.iBaand(o.plads, el('div', { className: 'dato-raekke' }, felt('fra', T.datoFra), felt('til', T.datoTil)));
    return el('div', { className: 'dato-vaelger' }, el('p', { className: 'vg-note', textContent: T.datoNote }), fejl);
  }
  /* En sum med huller siger det. p: Rusts PeriodeUd. */
  const daekning = p => (p.med < p.dage ? [el('p', { className: 'vg-note hul-note', textContent: fmt(T.hulNote, { m: tal(p.med), n: tal(p.dage) }) })] : []);
  /* Et domænenavn på én linje: det brydes aldrig midt i et ord, og mangler der plads, afskæres det med … (.domaene). Det fulde navn står i
     title; aria-label sætter den, der ejer navnet (knappen eller cellen), for en <span> kan ikke selv bære det. */
  const domaene = (navn, tag) => el(tag || 'span', { className: 'domaene', textContent: navn, title: navn });
  /* En sti i <code> med et muligt linjeskift efter hver "/". Hvert led står samlet (.sti-led), for ellers brydes der også efter en
     bindestreg; et led over STI_LED tegn kan være bredere end pladsen og må derfor brydes (.vg code). Teksten sættes som tekst. */
  const STI_LED = 40;
  function sti(tekst, klasse) {
    const c = el('code', { className: klasse || '' }), dele = String(tekst).split('/');
    dele.forEach((del, i) => {
      const sidste = i === dele.length - 1, led = del + (sidste ? '' : '/');
      if (led) c.append(led.length <= STI_LED ? el('span', { className: 'sti-led', textContent: led }) : led);
      if (!sidste && dele[i + 1]) c.append(el('wbr'));
    });
    return c;
  }
  /* En knap med data bag: query til dyk, ellers mål { gaa } eller { rul }. Altid med aria-label: hvor den fører hen. */
  /* Et aktivt filter, der kan fjernes, er kittets filterchip med vedFjern (× til højre). Målet er rammens (data-dyk, data-gaa). */
  function fjernChip(navn, maal, klasse) {
    const b = maalPaa(window.Kit.filterchip({ tekst: navn, vedFjern: () => { }, aria: fmt(T.dykFjern, { navn }) }), maal);
    if (klasse) b.classList.add(...klasse.split(' ').filter(Boolean));
    return b;
  }
  /* En tom tilstand i et afsnit er kittets lave udgave (Kit.tomTilstand med lille): én sætning, som læses op, når den skifter. */
  function tomt(tekst, klasse) {
    const t = window.Kit.tomTilstand({ tekst, lille: true });
    if (klasse) t.classList.add(klasse);
    return t;
  }
  /* En læsetabel er kittets tabel uden rækkevalg (Kit.tabel med raekkeHandling 'ingen' og layout 'auto'). Visningerne bygger
     stadig hoveder (<th> eller tekst) og rækker (<tr>), og indholdet flyttes over i kittets celler: .tal-kol er en talkolonne, en
     <th> i rækken er et rækkehoved, cellens øvrige klasser følger kolonnen, og rækkens klasse bliver data-raekke (kittet tager
     kun data-* på en række). */
  function laeseTabel(klasse, hoveder, rk) {
    const hs = (hoveder.tagName ? [...hoveder.children] : hoveder).map(h => (typeof h === 'string' ? { tekst: h, kl: [] } : { tekst: h.textContent, kl: [...h.classList] }));
    const model = rk.find(tr => tr.children.length === hs.length);
    const kolonner = hs.map((h, i) => {
      const c = model ? model.children[i] : null, kl = new Set([...h.kl, ...(c ? c.classList : [])]);
      kl.delete('v');
      return { tekst: h.tekst, tal: kl.has('tal-kol'), hoved: Boolean(c && c.tagName === 'TH'), klasse: [...kl].join(' ') || undefined };
    });
    const raekker = rk.map(tr => {
      const attr = {};
      if (tr.className) attr['data-raekke'] = tr.className;
      for (const [k, v] of Object.entries(tr.dataset)) attr['data-' + k.replace(/[A-Z]/g, m => '-' + m.toLowerCase())] = v;
      return { attr, titel: tr.title || undefined, celler: [...tr.children].map(c => { const f = document.createDocumentFragment(); f.append(...c.childNodes); return f; }) };
    });
    const t = window.Kit.tabel({ raekkeHandling: 'ingen', layout: 'auto', kolonner, raekker });
    t.classList.add('vg-tabel', ...(klasse ? klasse.split(' ').filter(Boolean) : []));
    return t;
  }
  /* Kittets rulleramme (Kit.tabelRul) med navn: et område med Tab-stop, når tabellen er bredere end sin plads. */
  function tabelRul(tabel, navn) {
    const r = window.Kit.tabelRul(tabel, { aria: navn });
    r.rod.classList.add('vg-tabel-rul');
    return r.rod;
  }
  /* Målet på et element, rammens lytter følger: et dyk (tekst), { gaa } eller { rul }. */
  function maalPaa(b, maal) {
    if (typeof maal === 'string') b.dataset.dyk = maal;
    else if (maal.gaa) b.dataset.gaa = maal.gaa;
    else b.dataset.rul = maal.rul;
    return b;
  }
  function knap(indhold, maal, klasse, label) {
    const b = maalPaa(el('button', { type: 'button', className: klasse || 'link dyk-celle' }, indhold), maal);
    b.setAttribute('aria-label', label || fmt(T.dykAabnTekst, { navn: b.textContent }));
    return b;
  }
  /* En flise er kittets StatusTile (Kit.statusflise): navn, tal og undertekst. Et tal med data bag åbner sit mål; 0 fører
     ingen steder hen. opt: aktiv (flisen for det viste: kittets valgt, aria-current), klasse, label og ekstra = { n, dyk } for
     en undertekst med sit eget mål ("i dag hidtil": kittets flise med to mål). Målene er rammens (data-dyk, data-gaa, data-rul),
     så kittets ved er tom. */
  const intet = () => { };
  function fliseTekst(n, tekst, sub) { return window.Kit.statusflise({ tal: n, tekst, under: sub || undefined }); }
  function flise(n, tekst, sub, maal, opt = {}) {
    const klik = Boolean(n && maal), label = opt.label || fmt(T.dykAabn, { navn: tekst, n: tal(n) });
    const klasser = (opt.klasse || '').split(' ').filter(Boolean), aktiv = Boolean(opt.aktiv);
    const toMaal = Boolean(opt.ekstra && opt.ekstra.n);
    const under = toMaal ? { tekst: sub, ved: intet, aria: fmt(T.dykAabnTekst, { navn: sub }) } : sub || undefined;
    const f = window.Kit.statusflise({ tal: tal(n), tekst, under, ved: klik ? intet : undefined, aria: klik ? label : undefined, valgt: aktiv || undefined });
    f.classList.add(...klasser);
    if (klik) maalPaa(f.matches('button') ? f : f.querySelector('.sflise-maal'), maal).classList.add('dyk-flise');
    if (toMaal) maalPaa(f.querySelector('.sflise-under-maal'), opt.ekstra.dyk);
    return f;
  }
  function cveKnap(id, tekst) {
    if (!R.erCve(id)) return document.createTextNode(id);
    const t = fmt(T.cveAabn, { id }), b = el('button', { type: 'button', className: 'link vg-cve-knap', textContent: tekst || id, title: t });
    b.dataset.cve = id;
    b.setAttribute('aria-label', t);
    return b;
  }
  /* Boksen "Vurdering": hvorfor dagen fik eller ikke fik hvert mærke, foldet sammen. Hver linje er en betingelse fra Rust med det tal, mærket afgøres af. uden_normal: der er intet at sammenligne med, så linjen får hverken ✓ eller ✗. o: zone, navn og dag. */
  function vurderingBoks(o) {
    const boks = el('details', { className: 'vurdering' }, el('summary', { textContent: T.vurdSpoerg }));
    boks.dataset.noegle = 'vurdering';
    const q = x => new URLSearchParams({ zone: o.zone, navn: o.navn, dag: o.dag, ...x }).toString();
    const procent = v => String(tal(Math.round(v * 10) / 10));
    const tekst = b => {
      /* En grænse er rundet op: tallene er hele, så "mindst 1.671,4" og "mindst 1.672" er det samme. Et snit er ikke en grænse og beholder decimalen. */
      const v = { n: tal(Math.round(b.n * 10) / 10), m: tal(b.id === 'stigning' ? Math.round(b.m * 10) / 10 : Math.ceil(b.m)), navn: b.navn || '' };
      if (b.uden_normal) return b.id === 'faktor' ? T.vurd.normalNul : fmt(T.vurd[`${b.id}Uden`], v);
      if (b.id === 'netvaerk' && !b.navn) return fmt(T.vurd.netvaerkIngen, v);
      if (b.id === 'radar') return b.ok === null ? T.vurd.radarIngen : fmt(T.vurd.radar, { n: procent(b.n), m: procent(b.m) });
      if (b.id === 'efter') return b.ok ? T.vurd.efter : T.vurd.efterNej;
      if (b.id === 'dns_navne' && b.navn) return fmt(T.vurd.dns_navneEks, v);
      return fmt(T.vurd[b.id], v);
    };
    /* Tal med data bag: dagens angreb, netværket, domænerne, de valgte og Verden. */
    const maal = b => {
      if (b.id === 'stigning' && b.n) return [q({ slags: 'angreb' }), T.vurdAabnDag];
      if (b.id === 'netvaerk' && b.navn) return [q({ netvaerk: b.navn }), fmt(T.vurdAabnNet, { navn: b.navn })];
      if (b.id === 'stier' && b.n) return [q({ forsoeg: 'alle' }), T.vurdAabnStier];
      if (b.id === 'valgte') return [{ gaa: 'indstillinger?afsnit=domaener' }, T.vurdAabnValgte];
      if (b.id === 'radar' && b.ok !== null) return [{ gaa: 'land?fane=verden' }, T.vurdAabnVerden];
      return null;
    };
    const domaene = (b, d) => knap(b.id === 'net_andre' ? `${d.navn} (${tal(d.n)})` : d.navn, new URLSearchParams({ zone: d.zone, navn: d.navn, dag: o.dag, ...(b.id === 'net_andre' && b.navn ? { netvaerk: b.navn } : {}) }).toString(), 'link dyk-celle', fmt(T.vurdAabnDomaene, { navn: d.navn }));
    const linje = b => {
      const [tegn, klasse, ord] = b.ok === null || b.uden_normal ? ['–', 'vurd-ikke', T.vurdIkke] : b.ok ? ['✓', 'vurd-ja', T.vurdJa] : ['✗', 'vurd-nej', T.vurdNej], m = maal(b);
      const li = el('li', { className: klasse }, el('span', { className: 'vurd-tegn', textContent: tegn }), el('span', { className: 'sr-only', textContent: `${ord}: ` }), m ? knap(tekst(b), m[0], 'link dyk-celle', m[1]) : tekst(b));
      li.dataset.id = b.id;
      if (b.domaener.length) li.append(': ', ...b.domaener.flatMap((d, i) => [...(i ? [', '] : []), domaene(b, d)]));
      return li;
    };
    const gruppe = (h, b) => [el('h4', { textContent: h }), el('ul', { className: 'vurd-liste' }, ...b.map(linje))];
    const status = (m, ja, foreloebig) => fmt(foreloebig ? T.vurdStatusForeloebig : ja ? T.vurdStatusJa : T.vurdStatusNej, { m });
    R.invoke('vurdering', { zone: o.zone, dag: o.dag }).then(v => {
      boks.append(el('p', { className: 'vg-note', textContent: fmt(T.vurdIntro, { d: datoLang(v.dag) }) }),
        ...gruppe(status(T.maerkeMaal, v.maal_ja, v.foreloebig), v.maal), ...gruppe(status(T.maerkeRekog, v.rekog_ja), v.rekog), ...(v.dns && v.dns.length ? gruppe(status(T.maerkeDns, v.dns_ja), v.dns) : []));
    }).catch(() => { boks.hidden = true; });
    return boks;
  }
  /* "Rammer det dig?" og trinene fra handlingsbogen (Overblikkets guide og dyk). */
  function bogAfsnit(id) {
    const b = S.bog(id);
    const ud = [el('p', { className: 'g-spoerg' }, el('b', { textContent: T.gRammer }), ' ', b.spoerg || b.alle), el('p', { className: 'g-trin-h', textContent: b.spoerg ? T.gHvisJa : T.gGoer }), el('ol', { className: 'g-trin' }, ...b.ja.map(t => el('li', { textContent: t })))];
    if (b.nej) ud.push(el('p', { className: 'g-nej' }, el('b', { textContent: T.gHvisNej }), ' ', b.nej));
    return ud;
  }
  /* Et regelkort, i Angreb og i dyk: hele kortet åbner reglen (knappen i overskriften strækkes over kortet); CVE, stier og lande ligger ovenpå.
     o: dyk (kortets mål), label, n (tallet), enhed, under (teksten til højre), sti(s) og land(k) → mål. */
  function regelKort(rg, o) {
    const vis = regelVis(rg), aabn = el('button', { type: 'button', className: 'vg-regel-aabn', textContent: vis.overskrift });
    aabn.dataset.dyk = o.dyk;
    aabn.setAttribute('aria-label', o.label);
    const top = el('header', {}, el('span', { className: 'vg-produkt', textContent: vis.maerke }));
    if (rg.cve) top.append(el('span', { className: 'vg-cve' }, cveKnap(rg.cve)));
    const kort = el('article', { className: 'vg-regel' + (rg.art === 'rutine' ? ' vg-rutine' : '') }, top, el('h3', {}, aabn),
      el('p', { className: 'vg-regel-n' }, el('b', { textContent: tal(o.n) }), ' ', o.enhed, ...(o.under ? [el('span', { textContent: o.under })] : [])));
    if (rg.stier && rg.stier.length) kort.append(el('p', { className: 'vg-regel-l' }, el('span', { textContent: T.regelStier }), ' ', ...rg.stier.slice(0, 3).flatMap(x => [knap(sti(x.sti), o.sti(x.sti)), ' '])));
    if (rg.lande && rg.lande.length) kort.append(el('p', { className: 'vg-regel-l' }, el('span', { textContent: T.regelFra }), ' ', ...rg.lande.slice(0, 3).flatMap((l, i) => [...(i ? [', '] : []), knap(land(l.kode), o.land(l.kode))])));
    if (rg.foerst) kort.append(el('p', { className: 'vg-regel-d', textContent: fmt(T.regelFoerst, { d: datoLang(rg.foerst), s: datoLang(rg.sidst) }) }));
    return kort;
  }
  /* rk: [{ navn, n, dyk?, gaa?, label?, kode?, domaene? }]. Med dyk eller gaa er rækken en knap; kode = navnet er en sti; domaene = navnet er et domæne. maks: den fulde liste, når den er delt. */
  function liste(rk, maks) {
    const m = maks || Math.max(1, ...rk.map(r => r.n));
    return el('ol', { className: 'vg-liste' }, ...rk.map(r => {
      const navn = r.kode ? sti(r.navn, 'vg-l-navn') : r.domaene ? domaene(r.navn) : el('span', { className: 'vg-l-navn', textContent: r.navn }), n = el('span', { className: 'vg-l-n', textContent: tal(r.n) });
      const li = el('li');
      if (r.dyk || r.gaa) {
        const b = el('button', { type: 'button', className: 'dyk-l' }, navn, n);
        if (r.dyk) b.dataset.dyk = r.dyk; else b.dataset.gaa = r.gaa;
        b.setAttribute('aria-label', r.label || fmt(T.dykAabn, { navn: r.navn, n: tal(r.n) }));
        li.append(b);
      } else li.append(navn, n);
      li.append(el('i', { className: bredde(100 * r.n / m) }));
      return li;
    }));
  }
  /* De første rækker fremme og resten bag en <details>. noegle: så den er åben igen, når visningen tegnes om. */
  function listeFlere(rk, synlige, noegle, tekst) {
    const m = Math.max(1, ...rk.map(r => r.n)), ud = [liste(rk.slice(0, synlige), m)];
    if (rk.length > synlige) {
      const flere = el('details', { className: 'dyk-flere' }, el('summary', { textContent: tekst }), liste(rk.slice(synlige), m));
      flere.dataset.noegle = noegle;
      ud.push(flere);
    }
    return ud;
  }
  /* <details> er åbne igen, efter at boks er tegnet om: husk() før, og kald svaret bagefter. */
  function husk(boks) {
    const aabne = new Set([...boks.querySelectorAll('details[open]')].map(x => x.dataset.noegle || x.className));
    return () => boks.querySelectorAll('details').forEach(x => { if (aabne.has(x.dataset.noegle || x.className)) x.open = true; });
  }
  /* "De seneste": rk = [{ t, regel, cve, land, sti, colo, metode, domaene? }]. opt.til(dim, v) → forespørgsel eller intet (værdien er
     filtreret i forvejen), opt.domaene(navn) → cellen, opt.uden = kolonner, der ikke står, opt.art(regel), opt.cve: CVE-knappen med. */
  function senesteTabel(rk, opt) {
    const med = rk.some(s => s.domaene), uden = opt.uden || [];
    const celle = (dim, v, tekst, klasse) => {
      const td = el('td', { className: klasse || '' }), q = v ? opt.til(dim, v) : null;
      if (!q) td.append(tekst); else td.append(knap(tekst, q));
      return td;
    };
    const hoved = [T.dykTid, ...(med ? [T.dykDomaene] : []), ...(uden.includes('regel') ? [] : [T.dykRegel]), T.dykLand, T.dykSti, T.dykColo, T.dykMetode];
    const raekker = rk.map(s => {
      const t = String(s.t || ''), tr = el('tr');
      tr.append(celle('dag', t.slice(0, 10), datoTid(t) + (s.n > 1 ? ` ×${tal(s.n)}` : ''), 'vg-mono'));
      if (med) tr.append(opt.domaene(s.domaene || ''));
      if (!uden.includes('regel')) {
        const td = celle('regel', s.regel, regelTitel(s.regel, opt.art && opt.art(s.regel)));
        if (opt.cve && s.cve) td.append(' ', el('span', { className: 'vg-cve' }, cveKnap(s.cve)));
        tr.append(td);
      }
      tr.append(celle('land', s.land, land(s.land)), celle('sti', s.sti, sti(s.sti)), celle('colo', s.colo, s.colo, 'vg-mono'), celle('metode', s.metode, s.metode, 'vg-mono'));
      return tr;
    });
    const tabel = laeseTabel('', [...hoved.map(h => { const th = el('th', { textContent: h }); th.setAttribute('scope', 'col'); return th; })], raekker);
    const rul = tabelRul(tabel, T.dykSenesteH);
    return rul;
  }
  /* Mærket "Ligner …": åbner dagen med netværket eller stierne som filter. Står i Overblik, Siden sidst og Angreb. */
  function moensterKnap(r) {
    const m = r.moenster;
    if (!m) return null;
    /* DNS har ingen stier at filtrere på: dagen åbner, og boksen "Vurdering" har tallene. */
    const navn = S.maerkeNavn(m), filter = { maalrettet: { netvaerk: m.netvaerk, slags: 'angreb' }, dns_rekognoscering: {} }[m.slags] || { forsoeg: 'alle' };
    const b = knap(navn, new URLSearchParams({ zone: r.id, navn: r.navn, dag: m.dag, ...filter }).toString(), `moenster-maerke moenster-${m.slags}`, fmt(T.maerkeAabn, { m: navn, navn: r.navn, d: datoLang(m.dag) }));
    b.title = S.moensterTekst(m);
    b.dataset.fokus = 'moenster';
    return b;
  }
  /* Mærket i en Overblik-række: en del af rækkens ene mål, så det er tekst. Som knap står det i Angreb og Siden sidst. */
  function moensterChip(r) {
    const m = r.moenster;
    return el('span', { className: `moenster-maerke moenster-${m.slags}`, textContent: S.maerkeNavn(m, true), title: S.moensterTekst(m) });
  }
  /* "Dit sites teknologi" i Angreb og dyk: kun valgene som chips, forslag fra data på én linje og knappen til #teknologi. o: zone, navn, linjer (teknologi_hent) og saet(tek, valg). */
  function tekKort(o) {
    const navne = v => o.linjer.filter(l => l.valg === v).map(l => S.tekNavn(l.tek)).join(', '), bruger = navne('bruger'), ikke = navne('bruger_ikke');
    const chips = [...(bruger ? [fmt(T.tekChipBruger, { l: bruger })] : []), ...(ikke ? [fmt(T.tekChipIkke, { l: ikke })] : [])];
    const ret = knap(`${T.tekRet} →`, { gaa: 'teknologi?' + new URLSearchParams({ zone: o.zone, navn: o.navn }) }, 'btn tek-ret', fmt(T.tekRetAabn, { navn: o.navn }));
    const ud = [chips.length ? el('ul', { className: 'tek-chips' }, ...chips.map(t => el('li', { className: 'tek-chip', textContent: t }))) : tomt(T.tekIngenValg, 'tek-tomt')];
    const f = o.linjer.find(l => l.forslag);
    if (f) {
      const ja = el('button', { type: 'button', className: 'link', textContent: T.tekForslagJa });
      ja.addEventListener('click', () => o.saet(f.tek, 'bruger_ikke'));
      ud.push(el('p', { className: 'tek-forslag' }, fmt(T.tekForslagKort, { n: tal(f.forsoeg30), t: S.tekNavn(f.tek) }), ' ', ja));
    }
    ud.push(el('p', { className: 'tek-ret-linje' }, ret));
    return ud;
  }
  /* "Hvad angrebene var rettet mod": alle i udsnittet, de mest angrebne først. Valget er kun et mærke: hverken "Bruger ikke" eller en skjult kolonne fjerner en linje. til(tek): forespørgslen til dyk. */
  function rettetMod(rk, til) {
    const maerke = { bruger: T.tekKortBruger, bruger_ikke: T.tekKortIkke, blandet: T.rettetBlandet };
    return el('ul', { className: 'rettet-liste' }, ...rk.map(x => {
      const navn = S.tekNavn(x.tek), li = el('li', { className: 'rettet-linje' + (x.valg === 'bruger_ikke' ? ' rettet-ikke' : '') }, el('span', { className: 'rettet-navn', textContent: navn }));
      if (x.tek !== 'andet') li.append(el('span', { className: `tek-maerke tek-${x.valg}`, textContent: maerke[x.valg] || T.tekVedIkke }));
      if (x.valg === 'bruger_ikke') li.append(el('small', { textContent: T.rettetUfarlig }));
      li.append(knap(tal(x.n), til(x.tek), 'link rettet-n', fmt(T.rettetAabn, { n: tal(x.n), t: navn })));
      return li;
    }));
  }
  /* Kilden under alt med Radar-tal (J9, CC BY-NC 4.0): åbner Radar i browseren, som de andre eksterne links. */
  function radarKilde() {
    const b = el('button', { type: 'button', className: 'link vg-cve-knap', textContent: T.landKilde, title: T.landKildeAabn });
    b.dataset.radar = '1';
    b.setAttribute('aria-label', T.landKildeAabn);
    return el('p', { className: 'vg-note radar-kilde' }, b);
  }
  /* Landene, kortdata kender: Radars koder for lande. */
  const tidszoneLand = z => tidszoner[z] || null;
  const landeKoder = () => Object.keys((verden && verden.lande) || {}).filter(k => /^[A-Z]{2}$/.test(k));
  /* Et mærke i en tegning, der kan åbnes: query til dyk eller { gaa }. Første mærke er tegningens ene Tab-stop. */
  function klikbar(e, query, label, foerste) {
    if (typeof query === 'string') e.setAttribute('data-dyk', query); else e.setAttribute('data-gaa', query.gaa);
    e.setAttribute('role', 'link');
    e.setAttribute('tabindex', foerste ? '0' : '-1');
    e.setAttribute('aria-label', label);
    return e;
  }

  const HIT = 24; // mindste klikflade i px
  /* Uger (Besøgende, et år): 53 uger skal kunne ses på én gang ved et vindue på 1200 px. Klikfladen er højere end bred (V-BESOEG.2 FIX 1). */
  const HIT_UGE = 14;
  /* ── søjlerne: dage = [{ dag, n, hentet? }]. opt: titel, dyk(dag) → forespørgsel, { gaa } eller intet, klasse (ekstra klasse på søjlen, fx Besøgendes blå),
     tekst(d) → hvad mærket siger (ellers "dato: tal"; bruges til uger), akse(d, i) → tekst under søjlen (ellers datoer; bruges til Drifts timer), hit (mindste bredde pr. søjle i px, ellers HIT) og maaneder (aksen viser
     måneden ved første søjle i en ny måned i stedet for datoer). Kun dage med tal kan åbnes. ── */
  /* FIX 1 (V-DRIFT.1): hjælpelinjerne og deres mærker. Den øverste linje står ved det største tal og den midterste ved det halve. Er det halve afrundet
     det samme som det største (største tal 1) eller 0 (ingen tal: alle dage er 0 eller ikke hentet), står den midterste hverken som linje eller mærke,
     så samme tal aldrig står to gange. Uden tal overhovedet står kun nullinjen: en skala uden tal siger intet. */
  function yAkse(maks, harTal, linje) {
    const midt = Math.round(maks / 2), medTop = harTal, medMidt = harTal && midt > 0 && midt < maks;
    return {
      linjer: [...(medTop ? [linje(maks, 'vg-akse')] : []), ...(medMidt ? [linje(maks / 2, 'vg-akse')] : []), linje(0, 'vg-akse vg-nul')],
      maerker: [...(medTop ? [el('span', { className: 'dyk-y dyk-y-top', textContent: tal(maks) })] : []), ...(medMidt ? [el('span', { className: 'dyk-y dyk-y-midt', textContent: tal(midt) })] : [])],
    };
  }
  function soejler(dage, opt) {
    const W = 1000, H = 170, top = 6, n = dage.length, bw = W / Math.max(1, n), maks = Math.max(1, ...dage.map(d => d.n || 0));
    const y = v => top + (H - top) * (1 - v / maks);
    const linje = (v, k) => svg('line', { class: k, 'vector-effect': 'non-scaling-stroke', x1: 0, x2: W, y1: y(v).toFixed(1), y2: y(v).toFixed(1) });
    const klik = dage.filter(d => d.n && d.hentet !== false && opt.dyk).length, sidste = [...dage].reverse().find(d => d.n && d.hentet !== false);
    const ya = yAkse(maks, dage.some(d => d.hentet !== false && d.n), linje);
    const s = svg('svg', { class: 'dyk-soejler', viewBox: `0 0 ${W} ${H}`, preserveAspectRatio: 'none', role: klik ? 'group' : 'img', 'aria-label': opt.titel }, ...ya.linjer);
    dage.forEach((d, i) => {
      const uhentet = d.hentet === false, v = uhentet ? 0 : d.n || 0, x = i * bw;
      const tekst = opt.tekst ? opt.tekst(d) : uhentet ? fmt(opt.hul || T.dykIkkeHentet, { d: datoLang(d.dag) }) : `${datoLang(d.dag)}: ${tal(v)}`;
      const g = svg('g', { class: 'dyk-soejle' + (uhentet ? ' dyk-uhentet' : '') }, titel(tekst), svg('rect', { class: 'dyk-flade', x: x.toFixed(1), y: 0, width: bw.toFixed(1), height: H }));
      if (v) g.append(svg('rect', { class: 'vg-soejle' + (opt.klasse ? ' ' + opt.klasse : ''), x: (x + bw * 0.14).toFixed(1), y: y(v).toFixed(1), width: (bw * 0.72).toFixed(1), height: Math.max(1, y(0) - y(v)).toFixed(1) }));
      if (v && opt.dyk) klikbar(g, opt.dyk(d.dag), opt.tekst ? tekst : fmt(T.dykDag, { d: datoLang(d.dag), n: tal(v) }), d === sidste);
      s.append(g);
    });
    const hver = n <= 8 ? 1 : n <= 31 ? 7 : Math.ceil(n / 5);
    const maerke = opt.akse ? opt.akse : opt.maaneder ? (d, i) => (i > 0 && d.dag.slice(0, 7) !== dage[i - 1].dag.slice(0, 7) ? S.maaned(d.dag) : '') : (d, i) => ((n - 1 - i) % hver === 0 ? datoKort(d.dag) : '');
    const x = el('div', { className: 'dyk-x' }, ...dage.map((d, i) => { const t = maerke(d, i); return el('span', {}, ...(t ? [el('b', { textContent: t })] : [])); }));
    x.setAttribute('aria-hidden', 'true');
    return el('div', { className: 'dyk-diagram ' + klasse(`min-width: calc(${n} * ${opt.hit || HIT}px + var(--font-size-xs) * 3.6);`) }, ...ya.maerker, s, x);
  }

  /* ── tre søjler pr. dag, side om side: serier = [{ felt, klasse }]. De står aldrig oven på hinanden, for tallene må ikke lægges
     sammen. opt: titel, tekst(dag) → hvad mærket siger, maal(dag) → { gaa } eller query. Kun dage med tal kan åbnes. ── */
  function soejlerSerier(dage, serier, opt) {
    const W = 1000, H = 170, top = 6, n = dage.length, bw = W / Math.max(1, n), s0 = bw * 0.1, sb = bw * 0.8 / serier.length;
    const sum = d => serier.some(x => d[x.felt]), maks = Math.max(1, ...dage.flatMap(d => serier.map(x => d[x.felt] || 0)));
    const y = v => top + (H - top) * (1 - v / maks);
    const linje = (v, k) => svg('line', { class: k, 'vector-effect': 'non-scaling-stroke', x1: 0, x2: W, y1: y(v).toFixed(1), y2: y(v).toFixed(1) });
    const sidste = [...dage].reverse().find(sum), ya = yAkse(maks, Boolean(sidste), linje);
    const s = svg('svg', { class: 'dyk-soejler', viewBox: `0 0 ${W} ${H}`, preserveAspectRatio: 'none', role: sidste ? 'group' : 'img', 'aria-label': opt.titel }, ...ya.linjer);
    dage.forEach((d, i) => {
      const x = i * bw, uhentet = d.hentet === false, tekst = uhentet ? fmt(T.dykIkkeHentet, { d: datoLang(d.dag) }) : opt.tekst(d);
      const g = svg('g', { class: 'dyk-soejle' + (uhentet ? ' dyk-uhentet' : '') }, titel(tekst), svg('rect', { class: 'dyk-flade', x: x.toFixed(1), y: 0, width: bw.toFixed(1), height: H }));
      serier.forEach((r, j) => {
        const v = d[r.felt] || 0;
        if (v) g.append(svg('rect', { class: r.klasse, x: (x + s0 + j * sb).toFixed(1), y: y(v).toFixed(1), width: sb.toFixed(1), height: Math.max(1, y(0) - y(v)).toFixed(1) }));
      });
      if (sum(d)) klikbar(g, opt.maal(d.dag), tekst, d === sidste);
      s.append(g);
    });
    const hver = n <= 8 ? 1 : n <= 31 ? 7 : Math.ceil(n / 5);
    const akse = el('div', { className: 'dyk-x' }, ...dage.map((d, i) => el('span', {}, ...((n - 1 - i) % hver === 0 ? [el('b', { textContent: datoKort(d.dag) })] : []))));
    akse.setAttribute('aria-hidden', 'true');
    return el('div', { className: 'dyk-diagram ' + klasse(`min-width: calc(${n} * ${HIT}px + var(--font-size-xs) * 3.6);`) }, ...ya.maerker, s, akse);
  }

  /* ── kurver på én akse (indeks): dage = [{ dag, ... }], serier = [{ felt, klasse }]. opt: titel, tekst(dag), maal(dag) → query eller intet,
     og akse: { maks, tekst } for en anden skala end indekset (så står 100-linjen ikke). Et døgn uden tal bryder kurven. Kun døgn med et mål kan
     åbnes; resten har ingen handling. Etiketterne står som HTML. ── */
  function kurver(dage, serier, opt) {
    const W = 1000, H = 170, top = 6, n = dage.length, bw = W / Math.max(1, n);
    const akse0 = opt.akse, maks = akse0 ? akse0.maks : Math.max(200, ...dage.flatMap(d => serier.map(s => d[s.felt] || 0))), y = v => top + (H - top) * (1 - v / maks);
    const linje = (v, k) => svg('line', { class: k, 'vector-effect': 'non-scaling-stroke', x1: 0, x2: W, y1: y(v).toFixed(1), y2: y(v).toFixed(1) });
    const klik = dage.filter(d => opt.maal(d)), sidste = klik[klik.length - 1];
    const s = svg('svg', { class: 'dyk-soejler', viewBox: `0 0 ${W} ${H}`, preserveAspectRatio: 'none', role: sidste ? 'group' : 'img', 'aria-label': opt.titel }, ...(akse0 ? [] : [linje(100, 'vg-akse')]), linje(0, 'vg-akse vg-nul'));
    dage.forEach((d, i) => {
      const tekst = opt.tekst(d), g = svg('g', { class: 'dyk-soejle' }, titel(tekst), svg('rect', { class: 'dyk-flade', x: (i * bw).toFixed(1), y: 0, width: bw.toFixed(1), height: H }));
      const q = opt.maal(d);
      if (q) klikbar(g, q, tekst, d === sidste);
      s.append(g);
    });
    for (const r of serier) {
      let stykke = [];
      const tegn = () => { if (stykke.length > 1) s.append(svg('polyline', { class: 'vg-kurve ' + r.klasse, 'vector-effect': 'non-scaling-stroke', points: stykke.join(' ') })); stykke = []; };
      dage.forEach((d, i) => { const v = d[r.felt]; if (v === null || v === undefined) tegn(); else stykke.push(`${((i + .5) * bw).toFixed(1)},${y(v).toFixed(1)}`); });
      tegn();
    }
    const hver = n <= 8 ? 1 : n <= 31 ? 7 : Math.ceil(n / 5);
    const akse = el('div', { className: 'dyk-x' }, ...dage.map((d, i) => el('span', {}, ...((n - 1 - i) % hver === 0 ? [el('b', { textContent: datoKort(d.dag) })] : []))));
    akse.setAttribute('aria-hidden', 'true');
    return el('div', { className: 'dyk-diagram ' + klasse(`min-width: calc(${n} * ${HIT}px + var(--font-size-xs) * 3.6);`) }, el('span', { className: 'dyk-y dyk-y-top', textContent: akse0 ? akse0.tekst(maks) : tal(Math.round(maks)) }), ...(akse0 ? [] : [el('span', { className: 'dyk-y ' + klasse(`top: ${y(100).toFixed(0)}px; transform: translateY(-50%);`), textContent: '100' })]), s, akse);
  }

  /* ── kortet: v = { lande: [{ kode, n }], colos: [{ kode, n }], buer: [{ land, colo, n }] } ──
     opt: titel, id + beskrivelse (<desc>), tal(n) → teksten på et mærke, og dyk: { land(k), colo(k), bue(l, c) } for mærker, der kan åbnes. */
  const proj = (lon, lat) => [500 + lon * 1000 / 360, 250 - lat * 1000 / 360];
  const VIEW = { verden: [0, 28, 1000, 385], europa: [430, 50, 250, 96] };
  const iBilledet = (p, [x, y, b, h]) => p[0] >= x && p[0] <= x + b && p[1] >= y && p[1] <= y + h;
  /* Ses en del af fladen (en sti af M, L og Z) i billedet? */
  const fladeIBilledet = (d, view) => d.split(/[MLZ]/).some(p => p && iBilledet(p.split(',').map(Number), view));
  const TRIN = 5;
  /* Landenes trin (1–5) som kvantiler af de forskellige tal: ét land med alt får det øverste trin og kan ikke skubbe de andre ned i det nederste. Samme tal giver samme trin. Lande uden angreb er ikke med. */
  function angrebsTrin(lande) {
    const tal = [...new Set(lande.map(l => l.n).filter(n => n > 0))].sort((a, b) => a - b);
    return new Map(lande.filter(l => l.n > 0).map(l => [l.kode, Math.ceil(TRIN * (tal.indexOf(l.n) + 1) / tal.length)]));
  }
  /* Forklaringen under kortet: skalaen med det mindste og største tal. Intet, når intet land har angreb. */
  function kortSkala(lande) {
    const tal = lande.map(l => l.n).filter(n => n > 0);
    if (!tal.length) return null;
    const trin = Array.from({ length: TRIN }, (_, i) => el('i', { className: `vg-trin-${i + 1}` }));
    return el('li', { className: 'vg-skala' }, el('span', { textContent: S.tal(Math.min(...tal)) }), el('span', { className: 'vg-skala-trin' }, ...trin), el('span', { textContent: S.tal(Math.max(...tal)) }), el('span', { textContent: T.kortSkala }));
  }
  function kort(v, visning, opt) {
    const [vx, vy, vw, vh] = VIEW[visning], k = 1000 / vw, r = n => (n / k).toFixed(2);
    const L = verden.lande || {}, F = verden.flader || {}, dyk = opt.dyk || null, tekst = opt.tal || (n => fmt(T.nAfvist, { n: tal(n) }));
    /* Kortet er mindst 600 px bredt (vinduets mindste bredde er 900 px), så en flade med denne radius er mindst 24 px. */
    const hitR = (HIT / 2) * vw / 600, set = p => iBilledet(p, VIEW[visning]);
    const fladeLag = svg('g', {}), bueLag = svg('g', {});
    /* Klikfladen er usynlig, også under musen: det er mærket, der lyser op. */
    const flade = (lag, e, q, label, tt, maerke) => {
      e.setAttribute('data-dyk', q); e.setAttribute('aria-label', label); e.setAttribute('aria-hidden', 'true'); e.append(titel(tt)); lag.append(e);
      if (!maerke) return;
      e.addEventListener('pointerenter', () => maerke.classList.add('vg-over'));
      e.addEventListener('pointerleave', () => maerke.classList.remove('vg-over'));
    };
    const s = svg('svg', { class: 'vg-kort', viewBox: `${vx} ${vy} ${vw} ${vh}`, role: dyk ? 'group' : 'img', 'aria-label': opt.titel, 'aria-describedby': opt.beskrivelse ? opt.id : null, preserveAspectRatio: 'xMidYMid meet' });
    if (opt.beskrivelse) s.append(svg('desc', { id: opt.id }, opt.beskrivelse));
    const gitter = svg('g', { class: 'vg-gitter', 'stroke-width': r(.4) });
    for (let lon = -150; lon <= 180; lon += 30) { const x = proj(lon, 0)[0]; gitter.append(svg('line', { x1: x, y1: 0, x2: x, y2: 500 })); }
    for (let lat = -60; lat <= 60; lat += 30) { const y = proj(0, lat)[1]; gitter.append(svg('line', { x1: 0, y1: y, x2: 1000, y2: y })); }
    s.append(svg('rect', { class: 'vg-hav', x: 0, y: 0, width: 1000, height: 500 }), gitter, svg('path', { class: 'vg-landflade', d: verden.land }));
    /* Lande med angreb: fladen åbner det samme som ringen. Tastaturet går til ringen; er den uden for billedet, er fladen selv et mærke, ellers kunne landet kun åbnes med musen. */
    const maerker = [], trin = angrebsTrin(v.lande), fyld = svg('g', {});
    for (const l of v.lande) {
      const d = F[l.kode], t = trin.get(l.kode);
      if (!d || !t) continue;
      const tt = `${land(l.kode)}: ${tekst(l.n)}`, label = fmt(T.dykAabn, { navn: land(l.kode), n: tal(l.n) }), ringSes = Boolean(L[l.kode]) && set(L[l.kode]);
      if (dyk && ringSes) { flade(fyld, svg('path', { class: `vg-fyld vg-trin-${t} dyk-hit`, d, 'fill-rule': 'evenodd' }), dyk.land(l.kode), label, tt); continue; }
      const e = svg('path', { class: `vg-fyld vg-trin-${t}`, d, 'fill-rule': 'evenodd' }, titel(tt));
      if (dyk && fladeIBilledet(d, VIEW[visning])) { e.classList.add('dyk-maerke'); maerker.push([e, dyk.land(l.kode), label]); }
      fyld.append(e);
    }
    s.append(fyld, svg('path', { class: 'vg-graense', d: Object.values(F).join('') + (verden.flader_uden_kode || ''), 'stroke-width': r(.35) }));
    s.append(svg('g', {}, ...[...Object.values(verden.colo), ...Object.values(verden.net).map(a => a.slice(0, 2))].map(([x, y]) => svg('circle', { class: 'vg-net', cx: x, cy: y, r: r(1.3) }))));
    const mb = Math.max(1, ...v.buer.map(b => b.n)), buer = svg('g', {});
    for (const b of v.buer.slice().sort((a, c) => a.n - c.n)) {
      const a = L[b.land], c = xyColo(b.colo);
      if (!a || !c || (!set(a) && !set(c))) continue; // Europa: buer helt uden for billedet er ikke mærker
      const d = Math.hypot(c[0] - a[0], c[1] - a[1]);
      if (d < 0.5) continue;
      const mx = (a[0] + c[0]) / 2, my = (a[1] + c[1]) / 2 - Math.max(6, d * 0.32), sti = `M${a[0]} ${a[1]} Q${mx.toFixed(1)} ${my.toFixed(1)} ${c[0]} ${c[1]}`;
      const tt = `${land(b.land)} → ${colo(b.colo)}: ${tekst(b.n)}`, label = fmt(T.dykBue, { a: land(b.land), b: colo(b.colo), n: tal(b.n) });
      const e = svg('path', { class: 'vg-bue' + (dyk ? ' dyk-maerke' : ''), d: sti, 'stroke-width': r(0.7 + 2.2 * Math.sqrt(b.n / mb)), 'stroke-dasharray': `${r(3)} ${r(3)}` }, titel(tt));
      if (dyk) {
        maerker.push([e, dyk.bue(b.land, b.colo), label]);
        flade(bueLag, svg('path', { class: 'dyk-hit dyk-hit-bue', d: sti, 'stroke-width': (hitR).toFixed(2) }), dyk.bue(b.land, b.colo), label, tt, e); // halv bredde: 60 buer à 24 px ville dække kortet
      }
      buer.append(e);
    }
    const ml = Math.max(1, ...v.lande.map(l => l.n)), ringe = svg('g', { 'stroke-width': r(.6) }), radius = n => 2.4 + 9 * Math.sqrt(n / ml);
    for (const l of v.lande) {
      const p = L[l.kode];
      if (!p || !set(p)) continue;
      const tt = `${land(l.kode)}: ${tekst(l.n)}`, label = fmt(T.dykAabn, { navn: land(l.kode), n: tal(l.n) });
      /* Glorien er en smal streg, der holder ringen læselig oven på et rødt land. */
      ringe.append(svg('circle', { class: 'vg-land-halo', cx: p[0], cy: p[1], r: r(radius(l.n)) }));
      const e = svg('circle', { class: 'vg-land' + (dyk ? ' dyk-maerke' : ''), cx: p[0], cy: p[1], r: r(radius(l.n)) }, titel(tt));
      if (dyk) {
        maerker.push([e, dyk.land(l.kode), label]);
        flade(fladeLag, svg('circle', { class: 'dyk-hit', cx: p[0], cy: p[1], r: hitR.toFixed(2) }), dyk.land(l.kode), label, tt, e);
      }
      ringe.append(e);
    }
    const mc = Math.max(1, ...v.colos.map(c => c.n)), ruder = svg('g', { 'stroke-width': r(.5) });
    for (const c of v.colos) {
      const p = xyColo(c.kode);
      if (!p || !set(p)) continue;
      const m = (1.8 + 3.2 * Math.sqrt(c.n / mc)) / k, tt = `${colo(c.kode)}: ${tekst(c.n)}`, label = fmt(T.dykAabn, { navn: colo(c.kode), n: tal(c.n) });
      const e = svg('path', { class: 'vg-colo' + (dyk ? ' dyk-maerke' : ''), d: `M${p[0]} ${(p[1] - m).toFixed(2)} L${(p[0] + m).toFixed(2)} ${p[1]} L${p[0]} ${(p[1] + m).toFixed(2)} L${(p[0] - m).toFixed(2)} ${p[1]} Z` }, titel(tt));
      if (dyk) {
        maerker.push([e, dyk.colo(c.kode), label]);
        flade(fladeLag, svg('circle', { class: 'dyk-hit', cx: p[0], cy: p[1], r: hitR.toFixed(2) }), dyk.colo(c.kode), label, tt, e);
      }
      ruder.append(e);
    }
    /* Fladerne ligger under alle mærker: et synligt mærke vinder altid over naboens flade. */
    s.append(bueLag, fladeLag, buer, ringe, ruder);
    /* Ringen først i Tab-rækkefølgen: de fleste leder efter et land. */
    maerker.sort((a, b) => Number(b[0].tagName === 'circle') - Number(a[0].tagName === 'circle'));
    maerker.forEach(([e, q, label], i) => klikbar(e, q, label, i === 0));
    /* Etiketterne: HTML oven på tegningen; i en tæt klynge (Europa i verdensvisningen) står kun den største. */
    const boks = el('div', { className: 'dyk-kort' }, s), sat = [];
    for (const l of v.lande.slice(0, visning === 'europa' ? 8 : 5)) {
      const p = L[l.kode];
      if (!p) continue;
      const x = 100 * (p[0] - vx) / vw, y = 100 * (p[1] + radius(l.n) / k - vy) / vh;
      if (x < 0 || x > 100 || y < 0 || y > 100 || sat.some(([a, b]) => Math.abs(a - x) < 9 && Math.abs(b - y) < 6)) continue;
      sat.push([x, y]);
      boks.append(el('span', { className: 'dyk-etiket ' + plads(x, y), textContent: land(l.kode) }));
    }
    return boks;
  }

  /* Det, kortet ikke kan vise: lande og datacentre uden en plads (Tor, ukendt land, datacentre uden for kortdata). */
  const paaKortet = p => Boolean(p) && iBilledet(p, VIEW.verden);
  const udenPlads = v => [...v.lande.filter(l => !paaKortet((verden.lande || {})[l.kode])).map(l => land(l.kode)), ...v.colos.filter(c => !paaKortet(xyColo(c.kode))).map(c => colo(c.kode))];

  /* ── Subdomæner (V-SUBDOM.1 B2): afsnittet i Besøgende og Angreb ud fra kommandoen subdomaener. Værtsnavnene er delvis angriberstyrede (klienten
     skriver Host); Rust har renset dem, og her sættes de kun som tekst. vaert er labelen under domænet ("" = domænet selv, "www", "shop"); kategorierne
     ([ip], [anden], [ugyldig] og [andre]) er ikke navne og kan ikke åbnes. ── */
  const SUB_TOP = 20, SUB_KAT = { '[ip]': 'ip', '[anden]': 'anden', '[ugyldig]': 'ugyldig' };
  const erKat = v => String(v).startsWith('[');
  /* Det fulde navn: domænet selv står som sit eget navn, www som www.<domæne>. */
  const subHost = (vaert, dom) => (vaert === '' ? dom : `${vaert}.${dom}`);
  /* Værten i adressen (vaert=<navn>): domænet selv eller et navn under det med de tegn, Rust gemmer (vaert::ren_vaert). Svarer med labelen, ellers null. */
  function subNoegle(host, dom) {
    const h = String(host || '').toLowerCase();
    if (!h || !dom || h.length > 253) return null;
    if (h === dom) return '';
    if (!h.endsWith('.' + dom)) return null;
    const l = h.slice(0, -dom.length - 1);
    return /^[a-z0-9_-]{1,63}(\.[a-z0-9_-]{1,63})*$/.test(l) ? l : null;
  }
  /* Perioden til kommandoen: højst et år (Rust afviser mere), regnet bagud fra til. */
  const subSpaend = (fra, til) => { const f = flytDag(til, -364); return { fra: fra < f ? f : fra, til }; };
  /* Afsnittet står, når domænet har tal på mindst to rigtige værter i perioden (kategorierne tæller ikke), eller når Cloudflare afviste forespørgslen. */
  const subRigtige = s => (s && Array.isArray(s.vaerter) ? s.vaerter.filter(v => !erKat(v.vaert)) : []);
  const subVis = (s, afvist) => Boolean(s) && Boolean(afvist || subRigtige(s).length >= 2);
  /* Den første dag med tal pr. subdomæne: den seneste af trafik og angreb, for tabellen har begge. */
  const subFra = s => [s.trafik_fra, s.angreb_fra].filter(Boolean).sort().pop() || null;
  /* V-SUBDOM.1 FIX 2 (F3): et 0 betyder "hentet, og tallet er 0". k er 'trafik' (besøg og forespørgsler) eller 'angreb'. subMangler: dagene i perioden, køen
     stadig henter (Rust: færdige døgn i hentevinduet uden tal). subIntet: ingen dag i perioden har tal for kolonnen, så den viser "–", ikke 0. */
  const subMangler = (s, k) => (s && Array.isArray(s[`${k}_mangler`]) ? s[`${k}_mangler`] : []);
  const subIntet = (s, k) => Boolean(s) && s[`${k}_dage`] === 0;
  /* Linjen om det, der hentes: hele perioden for en kolonne ("Henter besøg og forespørgsler pr. subdomæne …"), eller kun nogle dage ("Henter {n} dage
     endnu."). kol er kolonnerne, visningen har. En status (role=status), ikke klikbar; null, når intet mangler. */
  function subHenter(s, kol) {
    const U = T.sub, mangler = kol.filter(k => subMangler(s, k).length), hele = mangler.filter(k => subIntet(s, k)), dele = mangler.filter(k => !subIntet(s, k));
    const tekst = hele.map(k => (k === 'trafik' ? U.henterBes : U.henterAng));
    if (dele.length) {
      const n = tal(new Set(dele.flatMap(k => subMangler(s, k))).size);
      /* Står den ene kolonne helt uden tal, siger linjen, hvilken kolonne de resterende dage hører til. */
      tekst.push(fmt(!hele.length ? U.henterDage : dele[0] === 'trafik' ? U.henterBesDage : U.henterAngDage, { n }));
    }
    if (!tekst.length) return null;
    const p = el('p', { className: 'vg-note sub-henter', textContent: tekst.join(' ') });
    p.setAttribute('role', 'status');
    return p;
  }
  /* Fokus og rullepositionen bliver, når en visning tegnes om, fordi køen meldte nye tal (FIX 2): elementet med fokus findes igen ud fra afsnittet (nærmeste
     id), typen, klassen og teksten (aria-label eller tekst), ellers samme plads blandt de ens elementer. Kaldes før tegningen; svaret kaldes efter. */
  function huskFokus(boks) {
    const a = document.activeElement, rul = document.getElementById('indhold'), top = rul ? rul.scrollTop : null;
    const sted = x => [x.closest('[id]') ? x.closest('[id]').id : '', x.tagName, x.className].join('\u0001');
    const navn = x => x.getAttribute('aria-label') || x.textContent;
    const med = a && a !== boks && boks.contains(a) && a.tagName ? { sted: sted(a), navn: navn(a), i: [...boks.querySelectorAll(a.tagName)].filter(x => sted(x) === sted(a)).indexOf(a), tag: a.tagName } : null;
    return () => {
      if (med && !boks.contains(document.activeElement)) {
        const ens = [...boks.querySelectorAll(med.tag)].filter(x => sted(x) === med.sted);
        const n = ens.find(x => navn(x) === med.navn) || ens[med.i];
        if (n) n.focus({ preventScroll: true });
      }
      if (rul && top !== null && rul.scrollTop !== top) rul.scrollTop = top;
    };
  }
  /* o: domaene, afvist, intro, aria (skabelon med {vaert}), maal(navn) → mål, maalBes(navn) og maalAng(navn) → mål i Besøgende og Angreb med samme periode,
     fra (periodens første dag), id. Tabellen er kittets (laeseTabel). Navnet åbner visningen, tabellen står i (o.maal); et tal åbner det, tallet tæller
     (V-SUBDOM.1 FIX 1, F2): Besøg og Forespørgsler Besøgende, Angreb Angreb, filtreret til subdomænet. Navnet er Tab-stoppet; tallene er der til musen. Et tal,
     der er 0, fører ingen steder hen. Andel og noterne har intet bag sig. */
  function subAfsnit(s, o) {
    const U = T.sub, dele = [];
    if (o.afvist) dele.push(el('p', { className: 'vg-advarsel vg-gul sub-ikke-tilg', textContent: U.ikkeTilg }));
    else {
      const alle = s.vaerter.filter(v => v.vaert !== '[andre]'), synlige = alle.slice(0, SUB_TOP), rest = [...alle.slice(SUB_TOP), ...s.vaerter.filter(v => v.vaert === '[andre]')];
      const iAlt = s.vaerter.reduce((a, v) => a + v.foresp, 0), andel = n => (iAlt && !subIntet(s, 'trafik') ? `${(100 * n / iAlt).toLocaleString(loc, { maximumFractionDigits: 1 })} %` : '–');
      const celle = t => el('td', { className: 'tal-kol', textContent: t });
      /* FIX 2: en kolonne uden tal i hele perioden står med "–" (ikke 0, ikke klikbar); skærmlæseren hører "ikke hentet endnu". */
      const intet = { besoeg: subIntet(s, 'trafik'), foresp: subIntet(s, 'trafik'), angreb: subIntet(s, 'angreb') };
      const streg = () => el('td', { className: 'tal-kol' }, el('span', { className: 'sub-streg', textContent: '–' }), el('span', { className: 'sr-only', textContent: ` (${U.ikkeHentet})` }));
      const tre = (v, k) => (intet[k] ? streg() : celle(tal(v)));
      const th = (t, k) => { const e = el('th', { textContent: t, className: k || '' }); e.setAttribute('scope', 'col'); return e; };
      const hoved = () => [th(U.th), th(U.thBesoeg, 'tal-kol'), th(U.thForesp, 'tal-kol'), th(U.thAngreb, 'tal-kol'), th(U.thAndel, 'tal-kol')];
      /* aria-label og title siger, hvor knappen fører hen. */
      const medTitel = (b, label) => { b.title = label; return b; };
      const raekke = v => {
        if (erKat(v.vaert)) return el('tr', { className: 'sub-kat' }, el('th', { scope: 'row', textContent: U[SUB_KAT[v.vaert]] || U.ugyldig }), ...['besoeg', 'foresp', 'angreb'].map(k => tre(v[k], k)), celle(andel(v.foresp)));
        const navn = subHost(v.vaert, o.domaene), label = fmt(o.aria, { vaert: navn });
        const bes = [o.maalBes(navn), fmt(U.ariaBes, { vaert: navn })], ang = [o.maalAng(navn), fmt(U.ariaAng, { vaert: navn })];
        const tk = (n, [maal, lbl], k) => {
          if (intet[k]) return streg();
          if (!n) return el('td', { className: 'tal-kol' }, String(tal(n)));
          const b = medTitel(knap(tal(n), maal, `link dyk-celle sub-tal sub-tal-${k}`, lbl), lbl);
          b.tabIndex = -1;
          return el('td', { className: 'tal-kol' }, b);
        };
        const tr = el('tr', { className: 'sub-raekke' }, el('th', { scope: 'row' }, medTitel(knap(domaene(navn), o.maal(navn), 'link dyk-celle sub-navn', label), label)), tk(v.besoeg, bes, 'besoeg'), tk(v.foresp, bes, 'foresp'), tk(v.angreb, ang, 'angreb'), celle(andel(v.foresp)));
        tr.dataset.vaert = navn;
        return tr;
      };
      const rk = synlige.map(raekke), sum = k => rest.reduce((a, v) => a + v[k], 0);
      if (rest.length) rk.push(el('tr', { className: 'sub-andre' }, el('th', { scope: 'row', textContent: fmt(U.andre, { n: tal(rest.length) }) }), ...['besoeg', 'foresp', 'angreb'].map(k => tre(sum(k), k)), celle(andel(sum('foresp')))));
      dele.push(tabelRul(laeseTabel('sub-tabel', hoved(), rk), U.h));
      /* Navnene i "Andre" kan stadig åbnes: de står i en liste under tabellen. */
      const flere = rest.filter(v => !erKat(v.vaert));
      if (flere.length) {
        const d = el('details', { className: 'dyk-flere sub-flere' }, el('summary', { textContent: fmt(U.visAndre, { n: tal(flere.length) }) }), tabelRul(laeseTabel('sub-tabel', hoved(), flere.map(raekke)), U.h));
        d.dataset.noegle = 'sub-flere';
        dele.push(d);
      }
      /* FIX 2: det, køen stadig henter (begge kolonner står i tabellen). */
      const henter = subHenter(s, ['trafik', 'angreb']);
      if (henter) dele.push(henter);
      /* V-SUBDOM.1 FIX 1 (V2): døgn, hvor Cloudflare sendte flere rækker, end programmet kan hente på én gang, har ingen tal pr. subdomæne. */
      const afsk = new Set([...(s.trafik_afskaaret || []), ...(s.angreb_afskaaret || [])]).size;
      if (afsk) dele.push(el('p', { className: 'vg-note sub-afskaaret', textContent: fmt(U.afskDage, { n: tal(afsk) }) }));
      const fra = subFra(s);
      if (fra && fra > o.fra) dele.push(el('p', { className: 'vg-note sub-fra', textContent: fmt(U.fra, { dato: datoLang(fra) }) }));
    }
    dele.push(el('p', { className: 'vg-note sub-dns', textContent: U.dnsOnly }));
    const b = blok(U.h, o.afvist ? '' : o.intro, ...dele);
    b.id = o.id;
    return b;
  }
  /* Én linje under chippen i det filtrerede (V-SUBDOM.1 FIX 1, F1): hvad der ikke er med pr. subdomæne. {link} er "hele {domæne}", der fjerner filteret
     pr. subdomæne (samme mål som chippens ×). Afsnit uden tal pr. subdomæne står slet ikke. */
  function subUden(tekst, dom, maal, klasse) {
    const [foer, efter] = tekst.split('{link}'), hele = fmt(T.sub.hele, { domaene: dom });
    const b = knap(hele, maal, 'link sub-hele', fmt(T.sub.heleAria, { domaene: dom }));
    b.title = b.getAttribute('aria-label');
    return el('p', { className: `vg-note sub-uden ${klasse || ''}`.trim() }, foer, b, efter || '');
  }
  /* Perioden oversat mellem Besøgende (ingen = 7 dage, periode=30|aar, fra/til) og Angreb (periode=7|30|alt, dag, fra/til). Besøgendes "Et år" er Angrebs
     "Hele historikken" og omvendt (det nærmeste: Besøgende går højst et år tilbage); én dag i Angreb er Fra = Til i Besøgende. */
  function subPeriodeAng(p) {
    if (p.has('fra') && p.has('til')) return p.get('fra') === p.get('til') ? { dag: p.get('fra') } : { fra: p.get('fra'), til: p.get('til') };
    return { periode: { 30: '30', aar: 'alt' }[p.get('periode')] || '7' };
  }
  function subPeriodeBes(p) {
    if (p.has('dag')) return { fra: p.get('dag'), til: p.get('dag') };
    if (p.has('fra') && p.has('til')) return { fra: p.get('fra'), til: p.get('til') };
    const v = { 30: '30', alt: 'aar' }[p.get('periode')];
    return v ? { periode: v } : {};
  }

  window.hmTegn = { subAfsnit, subUden, subHenter, subIntet, huskFokus, subPeriodeAng, subPeriodeBes, subNoegle, subSpaend, subVis, subHost, subFra, angrebsTrin, kortSkala, chip, moensterChip, tekKort, rettetMod, vurderingBoks, interval, datoVaelger, moensterKnap, dato, daekning, svg, regelKort, klar, land, landeKoder, tidszoneLand, radarKilde, kurver, colo, robotNavn, robotValg, ROBOT_SLAGS, blok, segment, soejlerSerier, datoKort, datoLang, typeTekst, regelVis, regelTitel, stort, domaene, sti, knap, maal: maalPaa, fjernChip, tomt, laeseTabel, tabelRul, flise, fliseTekst, bredde, cveKnap, bogAfsnit, liste, listeFlere, husk, senesteTabel, soejler, HIT_UGE, kort, udenPlads, ark };
})();
