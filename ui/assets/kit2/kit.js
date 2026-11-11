/* Wi24rd UI Kit 2 — kittets opførsel som kode (window.Kit). Fælles for alle Wi24rd-skrivebordsprogrammer; intet her
   kender det enkelte program. Forbrugeren leverer indholdet (faner, grupper, knapper, træets punkter, visninger, rækker) og
   hvad der sker ved valg. Tekster gives som nøgler (t: 'noegle'); elementerne får data-t / data-t-aria / data-t-title, og
   programmets sprogmodul udfylder dem (Sprog.anvend(rod), se sprogmotor.js). Gives tekst i stedet for t, står teksten, som
   den er. Ingen innerHTML med data: alt bygges med createElement og textContent. Hver funktion er beskrevet i pakkens
   KOMPONENTER.md. */
(function () {
  'use strict';
  const SVG = 'http://www.w3.org/2000/svg';

  /* Stregikoner, 24×24, 2 px streg, runde ender, currentColor (kittets README: egne stregikoner). Kittet har endnu intet
     fuldt ikonsæt; disse er de almene. Hver post er en liste af [element, attributter]. Et program lægger sine egne
     ikoner til med registrerIkoner (samme format), så de kan bruges overalt, hvor kittet tager et ikonnavn. */
  const IKONER = {
    indsaet: [['path', { d: 'M9 4h6v3H9z' }], ['path', { d: 'M9 5H6v15h12V5h-3' }]],
    fil: [['path', { d: 'M6 3h8l4 4v14H6z' }], ['path', { d: 'M14 3v4h4' }]],
    soeg: [['circle', { cx: 11, cy: 11, r: 6.5 }], ['path', { d: 'M20 20l-4.2-4.2' }]],
    skjold: [['path', { d: 'M12 3l8 3v6c0 4.5-3.4 8.3-8 9-4.6-.7-8-4.5-8-9V6z' }]],
    kopier: [['rect', { x: 8, y: 8, width: 12, height: 12, rx: 2 }], ['path', { d: 'M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2' }]],
    oeje: [['path', { d: 'M3 12s3.5-6 9-6 9 6 9 6-3.5 6-9 6-9-6-9-6z' }], ['circle', { cx: 12, cy: 12, r: 2.5 }]],
    mappe: [['path', { d: 'M3 7h6l2 2h10v10H3z' }]],
    bog: [['path', { d: 'M5 4h12a2 2 0 0 1 2 2v14H7a2 2 0 0 1-2-2z' }], ['path', { d: 'M9 8h6M9 12h6' }]],
    gem: [['path', { d: 'M12 4v11' }], ['path', { d: 'M7 10l5 5 5-5' }], ['path', { d: 'M5 20h14' }]],
    plus: [['path', { d: 'M12 5v14M5 12h14' }]],
    aabn: [['path', { d: 'M14 4h6v6' }], ['path', { d: 'M20 4l-9 9' }], ['path', { d: 'M18 14v6H4V6h6' }]],
    eksport: [['path', { d: 'M12 15V4' }], ['path', { d: 'M7 9l5-5 5 5' }], ['path', { d: 'M5 14v6h14v-6' }]],
    import: [['path', { d: 'M12 4v11' }], ['path', { d: 'M7 10l5 5 5-5' }], ['path', { d: 'M5 14v6h14v-6' }]],
    slet: [['path', { d: 'M4 7h16' }], ['path', { d: 'M9 7V4h6v3' }], ['path', { d: 'M6 7l1 13h10l1-13' }]],
    afspil: [['path', { d: 'M7 5l12 7-12 7z' }]],
    sol: [['circle', { cx: 12, cy: 12, r: 4 }], ['path', { d: 'M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4' }]],
    maane: [['path', { d: 'M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z' }]],
    skaerm: [['rect', { x: 3, y: 4, width: 18, height: 12, rx: 1.5 }], ['path', { d: 'M8 20h8M12 16v4' }]],
    sprog: [['circle', { cx: 12, cy: 12, r: 9 }], ['path', { d: 'M3 12h18' }], ['path', { d: 'M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z' }]],
    laas: [['rect', { x: 5, y: 11, width: 14, height: 9, rx: 2 }], ['path', { d: 'M8 11V8a4 4 0 0 1 8 0v3' }]],
    /* Tandhjul: seks tænder (yderradius 10, inderradius 7,3) om et hul (r 3). Den gamle tegning (stråler om en cirkel)
       lignede solen og dermed et tema-ikon. */
    tandhjul: [['path', { d: 'M9.4 5.2L10.1 2.2L13.9 2.2L14.6 5.2A7.3 7.3 0 0 1 16.6 6.3L19.6 5.4L21.5 8.7L19.2 10.9A7.3 7.3 0 0 1 19.2 13.1L21.5 15.3L19.6 18.6L16.6 17.7A7.3 7.3 0 0 1 14.6 18.8L13.9 21.8L10.1 21.8L9.4 18.8A7.3 7.3 0 0 1 7.4 17.7L4.5 18.6L2.5 15.3L4.8 13.1A7.3 7.3 0 0 1 4.8 10.9L2.5 8.7L4.5 5.4L7.4 6.3A7.3 7.3 0 0 1 9.4 5.2z' }], ['circle', { cx: 12, cy: 12, r: 3 }]],
    ned: [['path', { d: 'M6 9l6 6 6-6' }]],
    hoejre: [['path', { d: 'M9 6l6 6-6 6' }]],
    filter: [['path', { d: 'M4 5h16l-6 7v6l-4 2v-8z' }]],
    luk: [['path', { d: 'M6 6l12 12M18 6L6 18' }]],
    rediger: [['path', { d: 'M4 20l1-4L16 5l3 3L8 19z' }], ['path', { d: 'M14 7l3 3' }]],
    kalender: [['rect', { x: 4, y: 5, width: 16, height: 15, rx: 1.5 }], ['path', { d: 'M4 10h16M8 3v4M16 3v4' }]],
  };

  /* Programmets egne ikoner: { navn: [[element, attributter], …] }. Et navn, kittet allerede har, kan ikke overtages. */
  function registrerIkoner(nye) {
    for (const [navn, dele] of Object.entries(nye)) {
      if (navn in IKONER) throw new Error('ikonet findes allerede: ' + navn);
      IKONER[navn] = dele;
    }
  }

  function ikon(navn, klasse) {
    const dele = IKONER[navn];
    if (!dele) throw new Error('ukendt ikon: ' + navn);
    const svg = document.createElementNS(SVG, 'svg');
    svg.setAttribute('viewBox', '0 0 24 24');
    svg.setAttribute('aria-hidden', 'true');
    svg.setAttribute('focusable', 'false');
    if (klasse) svg.setAttribute('class', klasse);
    for (const [el, att] of dele) {
      const d = document.createElementNS(SVG, el);
      for (const [k, v] of Object.entries(att)) d.setAttribute(k, String(v));
      svg.appendChild(d);
    }
    return svg;
  }

  /* Et element med tekst fra en nøgle (t) eller en færdig tekst. */
  function el(tag, klasse, tekst) {
    const e = document.createElement(tag);
    if (klasse) e.className = klasse;
    if (tekst && typeof tekst === 'object' && tekst.t) e.setAttribute('data-t', tekst.t);
    else if (tekst != null) e.textContent = String(tekst);
    return e;
  }
  const tekstAf = o => (o.t ? { t: o.t } : o.tekst);
  function aria(e, o) {
    if (o.ariaT) e.setAttribute('data-t-aria', o.ariaT);
    else if (o.aria) e.setAttribute('aria-label', o.aria);
  }

  /* ── StatusChip: symbolet følger varianten, så status aldrig hviler på farven alene. ── */
  const SYMBOL = { ok: '✓', warn: '!', danger: '?', info: 'i', none: '–' };
  function chip(variant, ord) {
    if (!(variant in SYMBOL)) throw new Error('ukendt chip-variant: ' + variant);
    const c = el('span', 'chip ' + variant);
    const s = el('span', null, SYMBOL[variant]);
    s.setAttribute('aria-hidden', 'true');
    c.append(s, el('span', 'ord', ord));
    return c;
  }

  /* ── Button ── variant: primary | danger | ghost | (sekundær). */
  function knap(o) {
    const b = el('button', 'btn' + (o.variant ? ' ' + o.variant : ''));
    b.type = 'button';
    if (o.ikon) b.appendChild(ikon(o.ikon, 'ik'));
    b.appendChild(el('span', null, tekstAf(o)));
    if (o.genvej) b.appendChild(el('span', 'gv', o.genvej));
    aria(b, o);
    if (o.deaktiveret) b.disabled = true;
    if (o.ved) b.addEventListener('click', o.ved);
    return b;
  }

  /* ── TextField ── Følsom tekst: ingen stavekontrol, autoudfyld eller autokorrektur. */
  let feltNr = 0;
  function felt(o) {
    const id = o.id || 'kit-felt-' + ++feltNr;
    const rod = el('div', 'felt');
    const etiket = el('label', 'label', o.etiket);
    etiket.htmlFor = id;
    const inp = document.createElement('input');
    inp.id = id;
    inp.type = 'text';
    if (o.vaerdi != null) inp.value = String(o.vaerdi);
    if (o.mono) inp.classList.add('mono');
    if (o.foelsom) {
      inp.spellcheck = false;
      inp.setAttribute('autocomplete', 'off');
      inp.setAttribute('autocorrect', 'off');
      inp.setAttribute('autocapitalize', 'off');
    }
    const hj = el('span', 'hj', o.fejl || o.hjaelp || null);
    hj.id = id + '-hj';
    if (o.fejl) {
      inp.setAttribute('aria-invalid', 'true');
      hj.classList.add('fejltekst');
    }
    if (o.fejl || o.hjaelp) inp.setAttribute('aria-describedby', hj.id);
    rod.append(etiket, inp, hj);
    return rod;
  }

  /* ── Banner ── Fire varianter som StatusChip; én sætning og højst én handling. */
  const BAN_SYMBOL = { info: 'i', ok: '✓', warn: '!', danger: '!' };
  function banner(o) {
    if (!(o.variant in BAN_SYMBOL)) throw new Error('ukendt banner-variant: ' + o.variant);
    const b = el('div', 'ban ' + o.variant);
    b.setAttribute('role', o.variant === 'danger' || o.variant === 'warn' ? 'alert' : 'status');
    const s = el('span', 's', BAN_SYMBOL[o.variant]);
    s.setAttribute('aria-hidden', 'true');
    b.append(s, el('span', 'ord', tekstAf(o)));
    if (o.handling) b.appendChild(knap({ ...o.handling, variant: 'ghost' }));
    return b;
  }

  /* ── DataTable ── kolonner: [{t|tekst, tal}]; raekker: [{gruppe: t|tekst, note} | {celler: [...], mono: [i…], data}].
     Hele rækken vælges med klik og piletaster (op/ned, Home/End); vedValg(række, data).
     Fra 2.3.0, kun med de nye indstillinger (uden dem er tabellen præcis som i 2.0.0):
     - Sortering: sort: true på en kolonne gør dens hoved til en knap. sortering: { kolonne, retning: 'stigende' |
       'faldende' } er den aktuelle; et klik vender retningen eller sorterer efter en ny kolonne (sortStart: 'faldende'
       på kolonnen begynder dér), sætter aria-sort og pilen og kalder vedSortering(kolonne, retning). Programmet sorterer
       selv og giver de nye rækker med saetRaekker: kun programmet har alle rækkerne (sider, grupper) og kender typerne.
     - To hovedrækker: kolonnegrupper: [{ id, t|tekst }] og gruppe: id på kolonnerne. Hver gruppe får en overskrift over
       sine kolonner (scope="colgroup", colspan, og en <colgroup>); en kolonne uden gruppe står i begge rækker (rowspan).
     - t.saetRaekker(raekker) bygger rækkerne om uden at røre hovedet (fokus på en sorteringsknap bliver), og
       t.saetSortering(kolonne, retning) sætter sorteringen udefra uden at kalde vedSortering. */
  function tabel(o) {
    const t = el('table', 'tabel');
    const thead = document.createElement('thead');
    const sortHoveder = [];
    const retningAf = r => (r === 'faldende' ? 'faldende' : 'stigende');
    let sortering = o.sortering && o.sortering.kolonne != null ? { kolonne: o.sortering.kolonne, retning: retningAf(o.sortering.retning) } : null;
    function hoved(k, i) {
      if (!k.sort) {
        const th = el('th', k.tal ? 'r' : null, tekstAf(k));
        th.scope = 'col';
        return th;
      }
      const th = el('th', k.tal ? 'r' : null);
      th.scope = 'col';
      const b = el('button', 'sorter');
      b.type = 'button';
      b.dataset.kolonne = String(i);
      const pil = el('span', 'sorter-pil');
      pil.setAttribute('aria-hidden', 'true');
      b.append(el('span', null, tekstAf(k)), pil);
      th.appendChild(b);
      sortHoveder.push({ th, pil, i });
      return th;
    }
    function tegnSortering() {
      for (const s of sortHoveder) {
        const on = Boolean(sortering) && sortering.kolonne === s.i;
        if (on) s.th.setAttribute('aria-sort', sortering.retning === 'faldende' ? 'descending' : 'ascending');
        else s.th.removeAttribute('aria-sort');
        s.pil.textContent = on ? (sortering.retning === 'faldende' ? '▼' : '▲') : '↕';
      }
    }
    if (o.kolonnegrupper && o.kolonner.some(k => k.gruppe != null)) {
      const navne = new Map(o.kolonnegrupper.map(g => [g.id, g]));
      const loeb = [];
      o.kolonner.forEach((k, i) => {
        const sidste = loeb[loeb.length - 1];
        if (k.gruppe != null && sidste && sidste.gruppe === k.gruppe) sidste.kol.push(i);
        else loeb.push({ gruppe: k.gruppe, kol: [i] });
      });
      const r1 = document.createElement('tr'), r2 = document.createElement('tr');
      for (const l of loeb) {
        const cg = document.createElement('colgroup');
        if (l.kol.length > 1) cg.span = l.kol.length;
        t.appendChild(cg);
        if (l.gruppe == null) {
          const th = hoved(o.kolonner[l.kol[0]], l.kol[0]);
          th.rowSpan = 2;
          r1.appendChild(th);
        } else {
          const th = el('th', 'kolgruppe', tekstAf(navne.get(l.gruppe) || { tekst: String(l.gruppe) }));
          th.scope = 'colgroup';
          th.colSpan = l.kol.length;
          r1.appendChild(th);
          for (const i of l.kol) r2.appendChild(hoved(o.kolonner[i], i));
        }
      }
      thead.append(r1, r2);
    } else {
      const hr = document.createElement('tr');
      o.kolonner.forEach((k, i) => hr.appendChild(hoved(k, i)));
      thead.appendChild(hr);
    }
    const tbody = document.createElement('tbody');
    const valgbare = [];
    function bygRaekker(raekker) {
      tbody.textContent = '';
      valgbare.length = 0;
      for (const r of raekker || []) {
        const tr = document.createElement('tr');
        if (r.gruppe) {
          tr.className = 'gruppe';
          const td = document.createElement('td');
          td.colSpan = o.kolonner.length;
          td.appendChild(el('b', null, typeof r.gruppe === 'object' ? r.gruppe : String(r.gruppe)));
          if (r.note) {
            td.append(' ');
            td.appendChild(el('span', 'muted', r.note));
          }
          tr.appendChild(td);
        } else {
          r.celler.forEach((c, i) => {
            const td = document.createElement('td');
            const kl = [o.kolonner[i] && o.kolonner[i].tal ? 'r' : '', (r.mono || []).includes(i) ? 'mono' : ''].filter(Boolean).join(' ');
            if (kl) td.className = kl;
            if (c instanceof Node) td.appendChild(c);
            else td.textContent = c == null ? '' : String(c);
            tr.appendChild(td);
          });
          tr.tabIndex = valgbare.length ? -1 : 0;
          tr.setAttribute('aria-selected', 'false');
          tr.__data = r.data;
          valgbare.push(tr);
        }
        tbody.appendChild(tr);
      }
    }
    bygRaekker(o.raekker);
    t.append(thead, tbody);
    if (sortHoveder.length) {
      tegnSortering();
      thead.addEventListener('click', e => {
        const b = e.target.closest('.sorter');
        if (!b) return;
        const i = Number(b.dataset.kolonne);
        const retning = sortering && sortering.kolonne === i ? (sortering.retning === 'stigende' ? 'faldende' : 'stigende') : retningAf(o.kolonner[i].sortStart);
        sortering = { kolonne: i, retning };
        tegnSortering();
        if (o.vedSortering) o.vedSortering(i, retning);
      });
    }
    function vaelg(tr, fokus) {
      for (const x of valgbare) {
        const on = x === tr;
        x.classList.toggle('sel', on);
        x.setAttribute('aria-selected', String(on));
        x.tabIndex = on ? 0 : -1;
      }
      if (fokus) tr.focus();
      if (o.vedValg) o.vedValg(tr, tr.__data);
    }
    tbody.addEventListener('click', e => {
      const tr = e.target.closest('tr');
      if (tr && valgbare.includes(tr) && !e.target.closest('input,button,select,textarea')) vaelg(tr, true);
    });
    tbody.addEventListener('keydown', e => {
      const i = valgbare.indexOf(e.target);
      if (i < 0) return;
      const ny = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: valgbare.length - 1 }[e.key];
      if (ny == null) return;
      e.preventDefault();
      vaelg(valgbare[Math.max(0, Math.min(valgbare.length - 1, ny))], true);
    });
    t.setAttribute('role', 'grid');
    if (o.ariaT) t.setAttribute('data-t-aria', o.ariaT);
    t.vaelg = i => vaelg(valgbare[i], false);
    t.saetRaekker = raekker => bygRaekker(raekker);
    t.saetSortering = (kolonne, retning) => {
      sortering = kolonne == null ? null : { kolonne, retning: retningAf(retning) };
      tegnSortering();
    };
    return t;
  }

  /* ── Sideskift (fra 2.3.0, hører til DataTable) ── Under en tabel, når programmet deler rækkerne i sider: forrige og
     næste, "side X af Y" og antal pr. side. Programmet leverer siderne: et klik flytter tallet og kalder vedSide(n), og
     programmet giver de nye rækker (t.saetRaekker) og kalder saet(side, sider), hvis antallet af sider ændres. En <nav>
     med navn. Forrige og næste har aria-disabled ved første og sidste side (de kan stadig nås, så fokus ikke forsvinder).
     o: { ariaT|aria, side, sider, forrige, naeste, sideAf ({v} og {w}), prSide, valg: [25, 50, 100], prSideEtiket,
     vedSide(n), vedPrSide(n) }. Returnerer { rod, saet(side, sider), side, sider, prSide }. */
  function sideskift(o) {
    const rod = el('nav', 'sideskift');
    aria(rod, o);
    const sideKnap = (tekst, hvilken) => {
      const b = el('button', 'btn');
      b.type = 'button';
      b.dataset.side = hvilken;
      b.appendChild(el('span', null, tk(tekst)));
      return b;
    };
    const forrige = sideKnap(o.forrige, 'forrige'), naeste = sideKnap(o.naeste, 'naeste');
    const nr = el('span', 'sideskift-nr');
    nr.setAttribute('role', 'status');
    rod.append(forrige, nr, naeste);
    let prSel = null;
    if (o.valg && o.valg.length) {
      const lab = el('label', 'sideskift-pr');
      lab.appendChild(el('span', null, tk(o.prSideEtiket)));
      const ramme = el('span', 'vliste');
      prSel = document.createElement('select');
      for (const n of o.valg) {
        const op = document.createElement('option');
        op.value = String(n);
        op.textContent = String(n);
        prSel.appendChild(op);
      }
      prSel.value = String(o.prSide != null ? o.prSide : o.valg[0]);
      ramme.append(prSel, ikon('ned'));
      lab.appendChild(ramme);
      rod.appendChild(lab);
      prSel.addEventListener('change', () => { if (o.vedPrSide) o.vedPrSide(Number(prSel.value)); });
    }
    let side = 1, sider = 1;
    function saet(s, n) {
      if (n != null) sider = Math.max(1, Number(n));
      if (s != null) side = Math.max(1, Math.min(sider, Number(s)));
      forrige.setAttribute('aria-disabled', String(side <= 1));
      naeste.setAttribute('aria-disabled', String(side >= sider));
      const t = tk(o.sideAf);
      for (const a of ['data-t', 'data-v', 'data-w']) nr.removeAttribute(a);
      if (t && typeof t === 'object') {
        nr.setAttribute('data-t', t.t);
        nr.setAttribute('data-v', String(side));
        nr.setAttribute('data-w', String(sider));
        nr.textContent = '';
      } else nr.textContent = String(t == null ? '{v} / {w}' : t).replace(/\{v\}/g, String(side)).replace(/\{w\}/g, String(sider));
    }
    function gaa(n) {
      if (n < 1 || n > sider || n === side) return;
      saet(n);
      if (o.vedSide) o.vedSide(side);
    }
    forrige.addEventListener('click', () => gaa(side - 1));
    naeste.addEventListener('click', () => gaa(side + 1));
    saet(o.side || 1, o.sider || 1);
    return {
      rod, saet,
      get side() { return side; },
      get sider() { return sider; },
      get prSide() { return prSel ? Number(prSel.value) : null; },
    };
  }

  /* ── CommandBar ── faner: [{id, t, grupper: [{t, knapper: [{id, t, ikon, pri, deaktiveret, hvorfor, ved, trykket}]}]}].
     Fanerne skifter båndets indhold, aldrig visningen. En deaktiveret kommando står nedtonet med aria-disabled (så den
     stadig kan nås med Tab og forklarer hvorfor i title); den forsvinder ikke. Returnerer { rod, vaelgFane, knap(id) }.
     Fra 2.2.0: udenFaner: true giver et bånd med præcis én fane og ingen fanerække (et program, man ser i). En post i
     knapper kan være { id, element } i stedet for en kommando; elementet (fx en vælger eller en segmentknap) står så i
     gruppen som det er. Uden de to er båndet præcis som i 2.0.0. */
  let cbNr = 0;
  function commandBar(rod, o) {
    const enkel = Boolean(o.udenFaner);
    if (enkel && o.faner.length !== 1) throw new Error('commandBar: udenFaner kræver præcis én fane (fik ' + o.faner.length + ')');
    rod.classList.add('cmd');
    rod.textContent = '';
    const nr = ++cbNr;
    const tabs = el('div', 'tabs');
    tabs.setAttribute('role', 'tablist');
    if (o.ariaT) tabs.setAttribute('data-t-aria', o.ariaT);
    if (enkel) {
      rod.classList.add('enkel');
      rod.setAttribute('role', 'group');
      if (o.ariaT) rod.setAttribute('data-t-aria', o.ariaT);
    }
    const knapper = {}, faner = [], paneler = [];
    let grpNr = 0;
    for (const f of o.faner) {
      const tab = el('button', 'tab', tekstAf(f));
      tab.type = 'button';
      tab.id = `kit-cb${nr}-fane-${f.id}`;
      tab.setAttribute('role', 'tab');
      tab.setAttribute('aria-controls', `kit-cb${nr}-panel-${f.id}`);
      tab.dataset.fane = f.id;
      tabs.appendChild(tab);
      faner.push(tab);
      const panel = el('div', 'grupper');
      panel.id = `kit-cb${nr}-panel-${f.id}`;
      if (!enkel) {
        panel.setAttribute('role', 'tabpanel');
        panel.setAttribute('aria-labelledby', tab.id);
      }
      panel.dataset.fane = f.id;
      for (const g of f.grupper) {
        const grp = el('div', 'grp');
        grp.setAttribute('role', 'group');
        const kn = el('div', 'kn');
        for (const k of g.knapper) {
          if (k.element) {
            const plads = el('div', 'baand-el');
            plads.appendChild(k.element);
            if (k.id) knapper[k.id] = k.element;
            kn.appendChild(plads);
            continue;
          }
          const b = el('button', 'cb' + (k.pri ? ' pri' : ''));
          b.type = 'button';
          b.dataset.kommando = k.id;
          if (k.ikon) b.appendChild(ikon(k.ikon));
          b.appendChild(el('span', null, tekstAf(k)));
          if (k.deaktiveret) {
            b.setAttribute('aria-disabled', 'true');
            if (k.hvorfor) b.setAttribute('data-t-title', k.hvorfor);
          }
          if (k.trykket != null) b.setAttribute('aria-pressed', String(Boolean(k.trykket)));
          b.addEventListener('click', e => {
            if (b.getAttribute('aria-disabled') === 'true') { e.preventDefault(); return; }
            if (k.ved) k.ved(e);
          });
          knapper[k.id] = b;
          kn.appendChild(b);
        }
        const cap = el('span', 'cap', tekstAf(g));
        cap.id = `kit-cb${nr}-grp-${++grpNr}`;
        grp.setAttribute('aria-labelledby', cap.id);
        grp.append(kn, cap);
        panel.appendChild(grp);
      }
      paneler.push(panel);
    }
    if (enkel) rod.append(...paneler);
    else rod.append(tabs, ...paneler);
    function vaelgFane(id, fokus) {
      for (const t of faner) {
        const on = t.dataset.fane === id;
        t.classList.toggle('on', on);
        t.setAttribute('aria-selected', String(on));
        t.tabIndex = on ? 0 : -1;
        if (on && fokus) t.focus();
      }
      for (const p of paneler) p.hidden = p.dataset.fane !== id;
      if (o.vedFane) o.vedFane(id);
    }
    tabs.addEventListener('click', e => {
      const t = e.target.closest('.tab');
      if (t) vaelgFane(t.dataset.fane, true);
    });
    tabs.addEventListener('keydown', e => {
      const i = faner.indexOf(e.target);
      if (i < 0) return;
      const ny = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: faner.length - 1 }[e.key];
      if (ny == null) return;
      e.preventDefault();
      vaelgFane(faner[(ny + faner.length) % faner.length].dataset.fane, true);
    });
    vaelgFane(o.valgt || o.faner[0].id);
    return { rod, vaelgFane, knap: id => knapper[id], paneler };
  }

  /* ── NavPane: træet ── punkter: [{id, t, antal, boern: [...]}]. Et klik filtrerer arbejdsfladen; det skifter aldrig
     visning. Piletaster flytter, højre/venstre folder ud og ind, Enter og mellemrum vælger. vedValg(id). */
  function navTrae(ul, punkter, o = {}) {
    ul.classList.add('trae');
    ul.textContent = '';
    ul.setAttribute('role', 'tree');
    if (o.ariaT) ul.setAttribute('data-t-aria', o.ariaT);
    const alle = [];
    function byg(liste, foraelder, niveau) {
      for (const p of liste) {
        const li = document.createElement('li');
        li.setAttribute('role', 'none');
        const b = el('button', 'tr');
        b.type = 'button';
        b.dataset.punkt = p.id;
        b.setAttribute('role', 'treeitem');
        b.setAttribute('aria-level', String(niveau));
        b.tabIndex = -1;
        const harBoern = p.boern && p.boern.length;
        if (harBoern) {
          b.appendChild(ikon('ned'));
          b.setAttribute('aria-expanded', 'true');
        }
        b.appendChild(el('span', null, tekstAf(p)));
        if (p.antal != null) b.appendChild(el('span', 'ant', String(p.antal)));
        li.appendChild(b);
        alle.push(b);
        if (harBoern) {
          const sub = document.createElement('ul');
          sub.setAttribute('role', 'group');
          byg(p.boern, sub, niveau + 1);
          li.appendChild(sub);
        }
        foraelder.appendChild(li);
      }
    }
    byg(punkter, ul, 1);
    const synlige = () => alle.filter(b => !b.closest('ul[hidden]'));
    function fold(b, ud) {
      if (!b.hasAttribute('aria-expanded')) return;
      b.setAttribute('aria-expanded', String(ud));
      b.parentElement.querySelector(':scope > ul').hidden = !ud;
      const gammel = b.querySelector('svg');
      b.replaceChild(ikon(ud ? 'ned' : 'hoejre'), gammel);
    }
    function vaelg(b, fokus) {
      for (const x of alle) {
        const on = x === b;
        x.classList.toggle('on', on);
        if (on) x.setAttribute('aria-current', 'true');
        else x.removeAttribute('aria-current');
        x.setAttribute('aria-selected', String(on));
        x.tabIndex = on ? 0 : -1;
      }
      if (fokus) b.focus();
      if (o.vedValg) o.vedValg(b.dataset.punkt);
    }
    ul.addEventListener('click', e => {
      const b = e.target.closest('.tr');
      if (!b) return;
      if (b.hasAttribute('aria-expanded') && e.target.closest('svg')) fold(b, b.getAttribute('aria-expanded') !== 'true');
      else vaelg(b, true);
    });
    ul.addEventListener('keydown', e => {
      const s = synlige(), i = s.indexOf(e.target);
      if (i < 0) return;
      const b = s[i];
      let ny = null;
      if (e.key === 'ArrowDown') ny = s[i + 1];
      else if (e.key === 'ArrowUp') ny = s[i - 1];
      else if (e.key === 'Home') ny = s[0];
      else if (e.key === 'End') ny = s[s.length - 1];
      else if (e.key === 'ArrowRight' && b.getAttribute('aria-expanded') === 'false') fold(b, true);
      else if (e.key === 'ArrowLeft' && b.getAttribute('aria-expanded') === 'true') fold(b, false);
      else if (e.key === 'ArrowLeft') ny = b.parentElement.parentElement.closest('li') && b.parentElement.parentElement.closest('li').querySelector(':scope > .tr');
      else if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); vaelg(b, true); return; }
      else return;
      e.preventDefault();
      if (ny) {
        for (const x of alle) x.tabIndex = x === ny ? 0 : -1;
        ny.focus();
      }
    });
    const start = alle.find(b => b.dataset.punkt === o.valgt) || alle[0];
    if (start) vaelg(start, false);
    return { vaelg: id => vaelg(alle.find(b => b.dataset.punkt === id), false), punkter: alle };
  }

  /* ── NavPane: visningsskift ── visninger: [{id, t, ikon, genvej}] (højst otte, altid samme rækkefølge). Hver er et link
     til #id; den aktive har aria-current="page". markér(id) sætter den aktive (programmet ejer skiftet). Fra 2.2.0: ved
     syv og otte får skiftet klassen taet (punkter på 30 px, så otte kan stå i det mindste vindue); seks eller færre er
     præcis som i 2.0.0. Ni eller flere afvises med en fejl, som et ukendt ikon. */
  const MAKS_VISNINGER = 8;
  function visningsskift(rod, visninger, o = {}) {
    if (visninger.length > MAKS_VISNINGER) throw new Error('visningsskift: højst otte visninger (fik ' + visninger.length + ')');
    rod.classList.add('skift');
    rod.classList.toggle('taet', visninger.length > 6);
    rod.textContent = '';
    rod.setAttribute('role', 'group');
    if (o.ariaT) rod.setAttribute('data-t-aria', o.ariaT);
    const links = visninger.map(v => {
      const a = el('a', 'vs');
      a.href = '#' + v.id;
      a.dataset.visning = v.id;
      if (v.ikon) a.appendChild(ikon(v.ikon));
      a.appendChild(el('span', null, tekstAf(v)));
      if (v.genvej) {
        const k = el('kbd', null, v.genvej);
        k.setAttribute('aria-hidden', 'true');
        a.appendChild(k);
        if (v.genvejAria) a.setAttribute('aria-keyshortcuts', v.genvejAria);
      }
      rod.appendChild(a);
      return a;
    });
    function markér(id) {
      for (const a of links) {
        const on = a.dataset.visning === id;
        a.classList.toggle('on', on);
        if (on) a.setAttribute('aria-current', 'page');
        else a.removeAttribute('aria-current');
      }
    }
    return { markér, links };
  }

  /* ── Wizard ── En guide ét trin ad gangen (design/ui-kit-2/components/Wizard). trin: [{t, h, p, indhold}], hvor t er
     trinnets navn i trinlisten, h overskriften (h2), p den ene sætning og indhold en valgfri Node under sætningen.
     Trin, der er nået, har flueben og kan klikkes; det aktuelle har aria-current="step"; kommende kan ikke klikkes.
     Fodlinjen har altid samme knapper: spring (valgfri, til venstre), Tilbage og den primære, der hedder naeste og i sidste
     trin slut. o: { trin, naeste, tilbage, slut, spring, hId, vedSlut(), vedSpring(), vedSkift(i) }. Returnerer
     { rod, gaaTil(i), get aktuel }. */
  function guide(rod, o) {
    rod.classList.add('wiz');
    rod.textContent = '';
    const liste = el('ol', 'trin');
    const indhold = el('div', 'indhold');
    const h = el('h2', null);
    if (o.hId) h.id = o.hId;
    h.tabIndex = -1;
    const p = el('p', 'muted');
    const plads = el('div', 'trin-indhold');
    indhold.append(h, p, plads);
    const fod = el('div', 'fod');
    const spring = o.spring ? knap({ ...o.spring, variant: 'ghost', ved: () => o.vedSpring && o.vedSpring() }) : null;
    const luft = el('span', 'luft');
    const tilbage = knap({ ...o.tilbage, ved: () => gaaTil(aktuel - 1) });
    const frem = knap({ ...o.naeste, variant: 'primary', ved: () => (aktuel === o.trin.length - 1 ? o.vedSlut && o.vedSlut() : gaaTil(aktuel + 1)) });
    if (spring) fod.appendChild(spring);
    fod.append(luft, tilbage, frem);
    rod.append(liste, indhold, fod);
    let aktuel = 0;
    function tegnListe() {
      liste.textContent = '';
      o.trin.forEach((t, i) => {
        const li = document.createElement('li');
        const naaet = i < aktuel;
        const e = el(naaet ? 'button' : 'span', 't' + (naaet ? ' done' : i === aktuel ? ' now' : ''));
        const n = el('span', 'n', naaet ? '✓' : String(i + 1));
        if (naaet) {
          e.type = 'button';
          n.setAttribute('aria-hidden', 'true');
          e.addEventListener('click', () => gaaTil(i));
        }
        if (i === aktuel) e.setAttribute('aria-current', 'step');
        e.append(n, el('span', null, tekstAf(t)));
        li.appendChild(e);
        liste.appendChild(li);
      });
    }
    function gaaTil(i) {
      aktuel = Math.max(0, Math.min(o.trin.length - 1, i));
      const t = o.trin[aktuel];
      h.textContent = '';
      h.removeAttribute('data-t');
      p.removeAttribute('data-t');
      if (t.h) h.setAttribute('data-t', t.h);
      if (t.p) p.setAttribute('data-t', t.p);
      plads.textContent = '';
      if (t.indhold) plads.appendChild(t.indhold);
      tegnListe();
      tilbage.disabled = aktuel === 0;
      const sidst = aktuel === o.trin.length - 1;
      frem.querySelector('span').setAttribute('data-t', (sidst ? o.slut : o.naeste).t);
      if (o.vedSkift) o.vedSkift(aktuel);
    }
    gaaTil(0);
    return { rod, gaaTil, get aktuel() { return aktuel; }, overskrift: h };
  }

  /* ── Fletbaand ── Kittets ene pynteelement: en tynd flettet linje i accent (design/ui-kit-2/components/Fletbaand).
     Ét sted pr. skærm, ingen betydning: aria-hidden. Højde 10 px, mønsterbredde 28 px, streg 1 px. */
  let fletNr = 0;
  function fletbaand() {
    const id = 'kit-flet-' + ++fletNr;
    const svg = document.createElementNS(SVG, 'svg');
    svg.setAttribute('class', 'flet');
    svg.setAttribute('width', '100%');
    svg.setAttribute('height', '10');
    svg.setAttribute('aria-hidden', 'true');
    svg.setAttribute('focusable', 'false');
    const defs = document.createElementNS(SVG, 'defs');
    const m = document.createElementNS(SVG, 'pattern');
    for (const [k, v] of Object.entries({ id, width: 28, height: 10, patternUnits: 'userSpaceOnUse' })) m.setAttribute(k, String(v));
    const sti = document.createElementNS(SVG, 'path');
    sti.setAttribute('d', 'M0 5 L7 1.5 L14 5 L21 8.5 L28 5 M0 5 L7 8.5 L14 5 L21 1.5 L28 5');
    m.appendChild(sti);
    defs.appendChild(m);
    const r = document.createElementNS(SVG, 'rect');
    r.setAttribute('width', '100%');
    r.setAttribute('height', '10');
    r.setAttribute('fill', `url(#${id})`);
    svg.append(defs, r);
    return svg;
  }

  /* En tekst som nøgle eller færdig tekst: { t: 'noegle' }, { tekst: 'Domæne' } eller 'Domæne'. */
  const tk = v => (v && typeof v === 'object' ? (v.t ? { t: v.t } : v.tekst) : v);

  /* ── Vaelger (fra 2.2.0) ── En knap, der viser den aktuelle værdi, og en liste, der åbner under den. Må stå i
     .titel .hoejre, i værktøjslinjen og i båndet; udseendet følger fladen (kit.css). Listen ligger over siden under
     knappen (eller over den) og holder sig inden for vinduet, så hverken titellinjen eller vinduets kant klipper den.
     o: { etiket, punkter: [{id, t|tekst, antal}], valgt, flere, alle, nValgt, soeg: {ph, ariaT|aria}, ingen, vedValg }.
     Ét valg (standard): listen lukker ved valg, og vedValg(id) kaldes, når valget ændres. Flere valg (flere: true):
     afkrydsning pr. punkt, alle (valgfri) øverst, listen bliver åben, vedValg([id, …]) ved hver ændring. Knappen viser
     det valgte navn, alle-teksten, når alt er valgt, eller nValgt med data-v = antallet. saetPunkter og saetValgt sætter
     indhold og valg udefra uden at kalde vedValg. */
  let vlgNr = 0, vlgPNr = 0;
  const VLG_KANT = 8, VLG_AFSTAND = 4;
  function vaelger(o) {
    const nr = ++vlgNr, flere = Boolean(o.flere);
    const rod = el('div', 'vlg');
    const knap = el('button', 'vlg-knap');
    knap.type = 'button';
    knap.setAttribute('aria-haspopup', 'listbox');
    knap.setAttribute('aria-expanded', 'false');
    const etiket = el('span', 'skjult-tekst', tk(o.etiket));
    etiket.id = `kit-vlg${nr}-etiket`;
    const vaerdi = el('span', 'vlg-vaerdi');
    knap.append(etiket, vaerdi, ikon('ned'));
    const pop = el('div', 'vlg-pop');
    pop.hidden = true;
    const liste = el('ul', 'vlg-liste');
    liste.id = `kit-vlg${nr}-liste`;
    liste.setAttribute('role', 'listbox');
    liste.setAttribute('aria-labelledby', etiket.id);
    liste.tabIndex = -1;
    if (flere) liste.setAttribute('aria-multiselectable', 'true');
    knap.setAttribute('aria-controls', liste.id);
    let soeg = null;
    if (o.soeg) {
      const felt = el('label', 'soeg vlg-soeg');
      felt.appendChild(ikon('soeg'));
      soeg = document.createElement('input');
      soeg.type = 'search';
      soeg.spellcheck = false;
      soeg.setAttribute('autocomplete', 'off');
      soeg.setAttribute('aria-controls', liste.id);
      if (o.soeg.ph) soeg.setAttribute('data-t-ph', o.soeg.ph);
      aria(soeg, o.soeg);
      felt.appendChild(soeg);
      pop.appendChild(felt);
    }
    pop.appendChild(liste);
    const ingen = o.ingen ? el('p', 'vlg-ingen', tk(o.ingen)) : null;
    if (ingen) {
      ingen.hidden = true;
      pop.appendChild(ingen);
    }
    rod.append(knap, pop);

    let punkter = [], opts = [], alleLi = null, aktiv = null;
    let valgt = flere ? new Set() : null;
    const navnAf = li => li.querySelector('.vlg-navn');
    const synlige = () => [alleLi, ...opts].filter(li => li && !li.hidden);
    function punkt(id, tekst, antal) {
      const li = el('li', 'vlg-p');
      li.id = `kit-vlg${nr}-p${++vlgPNr}`;
      li.setAttribute('role', 'option');
      li.dataset.punkt = id;
      const m = flere ? el('span', 'vlg-boks', '✓') : el('span', 'vlg-mark', '✓');
      m.setAttribute('aria-hidden', 'true');
      li.append(m, el('span', 'vlg-navn', tekst));
      if (antal != null) li.appendChild(el('span', 'ant', String(antal)));
      liste.appendChild(li);
      return li;
    }
    function bygListe() {
      liste.textContent = '';
      alleLi = flere && o.alle ? punkt('', tk(o.alle), null) : null;
      if (alleLi) alleLi.classList.add('vlg-alle');
      opts = punkter.map(p => punkt(p.id, tekstAf(p), p.antal));
      aktiv = null;
    }
    const valgteIds = () => opts.filter(li => valgt.has(li.dataset.punkt)).map(li => li.dataset.punkt);
    function tegnValg() {
      for (const li of opts) li.setAttribute('aria-selected', String(flere ? valgt.has(li.dataset.punkt) : li.dataset.punkt === valgt));
      const alt = flere && opts.length > 0 && valgt.size === opts.length;
      if (alleLi) alleLi.setAttribute('aria-selected', String(alt));
      vaerdi.removeAttribute('data-t');
      vaerdi.removeAttribute('data-v');
      vaerdi.textContent = '';
      let kilde = null;
      if (!flere) kilde = opts.find(li => li.dataset.punkt === valgt) || null;
      else if (alt && alleLi) kilde = alleLi;
      else if (valgt.size === 1) kilde = opts.find(li => valgt.has(li.dataset.punkt));
      if (kilde) {
        const n = navnAf(kilde);
        if (n.hasAttribute('data-t')) vaerdi.setAttribute('data-t', n.getAttribute('data-t'));
        vaerdi.textContent = n.textContent;
      } else if (flere && o.nValgt) {
        const nv = tk(o.nValgt);
        if (nv && typeof nv === 'object') {
          vaerdi.setAttribute('data-t', nv.t);
          vaerdi.setAttribute('data-v', String(valgt.size));
        } else vaerdi.textContent = String(nv).replace(/\{v\}/g, String(valgt.size));
      }
    }
    function saetValgt(v) {
      const findes = id => punkter.some(p => p.id === id);
      if (flere) valgt = new Set((Array.isArray(v) ? v : v == null ? [] : [v]).filter(findes));
      else valgt = findes(v) ? v : punkter.length ? punkter[0].id : null;
      tegnValg();
    }
    function saetPunkter(nye, nytValg) {
      const foer = flere ? [...valgt] : valgt;
      punkter = nye.slice();
      bygListe();
      saetValgt(nytValg === undefined ? foer : nytValg);
      if (!pop.hidden) {
        if (soeg) filtrer();
        saetAktiv(synlige()[0] || null);
        placer();
      }
    }
    function saetAktiv(li) {
      for (const x of [alleLi, ...opts]) if (x) x.classList.toggle('aktiv', x === li);
      aktiv = li;
      for (const f of soeg ? [liste, soeg] : [liste]) {
        if (li) f.setAttribute('aria-activedescendant', li.id);
        else f.removeAttribute('aria-activedescendant');
      }
      if (li && li.scrollIntoView) li.scrollIntoView({ block: 'nearest' });
    }
    function filtrer() {
      const q = soeg.value.trim().toLocaleLowerCase();
      let n = 0;
      for (const li of opts) {
        const vis = !q || navnAf(li).textContent.toLocaleLowerCase().includes(q);
        li.hidden = !vis;
        if (vis) n++;
      }
      if (alleLi) alleLi.hidden = Boolean(q);
      if (ingen) ingen.hidden = n > 0;
      const s = synlige();
      if (!s.includes(aktiv)) saetAktiv(s[0] || null);
    }
    /* Under knappen, eller over den, hvis der er mere plads dér; venstre kant ved knappens, eller højre kant ved knappens,
       hvis listen ellers ville gå ud over vinduet. Kun klasser (op, hoejre-kant): ingen inline-stil, så kittet også virker
       i et program, der forbyder den. Største højde står i kit.css (højst 360 px og højst et halvt vindue minus 48 px),
       så listen altid er der, hvor der er mest plads. */
    function placer() {
      if (pop.hidden) return;
      pop.classList.remove('op', 'hoejre-kant');
      const k = knap.getBoundingClientRect(), p = pop.getBoundingClientRect();
      const vb = window.innerWidth || document.documentElement.clientWidth;
      const vh = window.innerHeight || document.documentElement.clientHeight;
      const under = vh - k.bottom - VLG_AFSTAND - VLG_KANT, over = k.top - VLG_AFSTAND - VLG_KANT;
      pop.classList.toggle('op', p.height > under && over > under);
      pop.classList.toggle('hoejre-kant', k.left + p.width > vb - VLG_KANT && k.right - p.width >= 0);
    }
    function udenfor(e) {
      if (!rod.contains(e.target)) luk(false);
    }
    function aabn() {
      if (!pop.hidden) return;
      pop.hidden = false;
      knap.setAttribute('aria-expanded', 'true');
      if (soeg) {
        soeg.value = '';
        filtrer();
      }
      const s = synlige();
      saetAktiv(s.find(li => li.getAttribute('aria-selected') === 'true' && li !== alleLi) || s[0] || null);
      placer();
      window.addEventListener('resize', placer);
      document.addEventListener('scroll', placer, true);
      document.addEventListener('mousedown', udenfor, true);
      (soeg || liste).focus();
    }
    function luk(fokus) {
      if (pop.hidden) return;
      pop.hidden = true;
      knap.setAttribute('aria-expanded', 'false');
      window.removeEventListener('resize', placer);
      document.removeEventListener('scroll', placer, true);
      document.removeEventListener('mousedown', udenfor, true);
      if (fokus) knap.focus();
    }
    function vaelg(li) {
      if (!li) return;
      if (!flere) {
        const aendret = valgt !== li.dataset.punkt;
        valgt = li.dataset.punkt;
        tegnValg();
        luk(true);
        if (aendret && o.vedValg) o.vedValg(valgt);
        return;
      }
      if (li === alleLi) {
        if (valgt.size === opts.length) valgt.clear();
        else for (const x of opts) valgt.add(x.dataset.punkt);
      } else if (valgt.has(li.dataset.punkt)) valgt.delete(li.dataset.punkt);
      else valgt.add(li.dataset.punkt);
      tegnValg();
      saetAktiv(li);
      if (o.vedValg) o.vedValg(valgteIds());
    }
    /* Et museklik på knappen skal lukke en åben liste, også i WebKit, hvor knappen ikke får fokus ved klik. */
    let aabenVedNed = null;
    knap.addEventListener('mousedown', () => { aabenVedNed = !pop.hidden; });
    knap.addEventListener('click', () => {
      const varAaben = aabenVedNed === null ? !pop.hidden : aabenVedNed;
      aabenVedNed = null;
      if (varAaben) luk(true);
      else aabn();
    });
    knap.addEventListener('keydown', e => {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        aabn();
      }
    });
    /* Klik i listen tager ikke fokus fra listen eller søgefeltet. */
    pop.addEventListener('mousedown', e => { if (e.target !== soeg) e.preventDefault(); });
    liste.addEventListener('click', e => {
      const li = e.target.closest('.vlg-p');
      if (li && liste.contains(li)) vaelg(li);
    });
    if (soeg) soeg.addEventListener('input', filtrer);
    pop.addEventListener('keydown', e => {
      const s = synlige(), i = s.indexOf(aktiv), iFelt = e.target === soeg;
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        luk(true);
      } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const ny = s[Math.max(0, Math.min(s.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))];
        if (ny) saetAktiv(ny);
      } else if ((e.key === 'Home' || e.key === 'End') && !iFelt) {
        e.preventDefault();
        if (s.length) saetAktiv(e.key === 'Home' ? s[0] : s[s.length - 1]);
      } else if (e.key === 'Enter' || (e.key === ' ' && !iFelt)) {
        e.preventDefault();
        vaelg(aktiv);
      } else if (!iFelt && !soeg && e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
        const b = e.key.toLocaleLowerCase();
        const ny = [...s.slice(i + 1), ...s.slice(0, i + 1)].find(li => navnAf(li).textContent.trim().toLocaleLowerCase().startsWith(b));
        if (ny) {
          e.preventDefault();
          saetAktiv(ny);
        }
      }
    });
    rod.addEventListener('focusout', e => {
      if (!pop.hidden && e.relatedTarget && !rod.contains(e.relatedTarget)) luk(false);
    });

    punkter = (o.punkter || []).slice();
    bygListe();
    saetValgt(o.valgt);
    return {
      rod, knap, liste, aabn, luk: () => luk(false), saetPunkter, saetValgt,
      get valgt() { return flere ? valgteIds() : valgt; },
      get aaben() { return !pop.hidden; },
    };
  }

  /* ── Sidepanel (fra 2.2.0) ── Et panel i højre side af .midte (programmet lægger <aside> efter .arb) med overskrift,
     lukkeknap og rulbart indhold. Fra GRAENSE px vindue og op skubber det arbejdsfladen; under lægger det sig over med
     skygge (klassen over). overlaeg: true lægger det altid over. Ikke modalt: fokus fanges ikke. Fokus går til
     overskriften ved åbning og tilbage til det, der havde fokus, ved lukning (hvis fokus var i panelet). Esc i panelet
     lukker; ligger det over, lukker Esc også fra resten af siden. Knapper med aria-controls = panelets id får
     aria-expanded. o: { t|tekst, lukAriaT|lukAria, indhold, overlaeg, graense, vedAabn(), vedLuk(aarsag) }, hvor
     aarsag er 'knap', 'esc' eller 'kode'. Returnerer { rod, aabn, luk, skift, aaben, overlagt, indhold, overskrift }. */
  const SP_GRAENSE = 1200;
  let spNr = 0;
  function sidepanel(rod, o) {
    const nr = ++spNr, graense = o.graense || SP_GRAENSE;
    rod.classList.add('sidepanel');
    rod.textContent = '';
    if (!rod.id) rod.id = 'kit-sp' + nr;
    rod.setAttribute('role', 'complementary');
    rod.hidden = true;
    const top = el('div', 'sp-top');
    const h = el('h2', null, tekstAf(o));
    h.id = rod.id + '-h';
    h.tabIndex = -1;
    rod.setAttribute('aria-labelledby', h.id);
    const lukKnap = el('button', 'sp-luk');
    lukKnap.type = 'button';
    lukKnap.appendChild(ikon('luk'));
    aria(lukKnap, { ariaT: o.lukAriaT, aria: o.lukAria });
    top.append(h, lukKnap);
    const indhold = el('div', 'sp-indhold');
    if (o.indhold) indhold.appendChild(o.indhold);
    rod.append(top, indhold);
    let fra = null;
    function lag() {
      rod.classList.toggle('over', Boolean(o.overlaeg) || window.innerWidth < graense);
    }
    function styrere(aaben) {
      for (const b of document.querySelectorAll('[aria-controls]')) {
        if (b.getAttribute('aria-controls') === rod.id) b.setAttribute('aria-expanded', String(aaben));
      }
    }
    function escOver(e) {
      if (e.key === 'Escape' && !e.defaultPrevented && rod.classList.contains('over') && !rod.contains(e.target)) luk('esc');
    }
    function aabn() {
      if (!rod.hidden) {
        h.focus();
        return;
      }
      const a = document.activeElement;
      fra = a && a !== document.body ? a : null;
      lag();
      rod.hidden = false;
      styrere(true);
      window.addEventListener('resize', lag);
      document.addEventListener('keydown', escOver);
      h.focus();
      if (o.vedAabn) o.vedAabn();
    }
    function luk(aarsag) {
      if (rod.hidden) return;
      const varInde = rod.contains(document.activeElement);
      rod.hidden = true;
      styrere(false);
      window.removeEventListener('resize', lag);
      document.removeEventListener('keydown', escOver);
      const tilbage = fra;
      fra = null;
      if (varInde && tilbage && tilbage.isConnected) tilbage.focus();
      if (o.vedLuk) o.vedLuk(aarsag);
    }
    lukKnap.addEventListener('click', () => luk('knap'));
    rod.addEventListener('keydown', e => {
      if (e.key === 'Escape' && !e.defaultPrevented) {
        e.preventDefault();
        luk('esc');
      }
    });
    return {
      rod, aabn, luk: () => luk('kode'), skift: () => (rod.hidden ? aabn() : luk('kode')),
      get aaben() { return !rod.hidden; },
      get overlagt() { return !rod.hidden && rod.classList.contains('over'); },
      indhold, overskrift: h,
    };
  }

  /* ── Segmentknap (fra 2.2.0) ── To til fem valg, præcis ét valgt (fx 7 dage / 30 dage / År). role="radiogroup" med
     role="radio" og aria-checked. Tab går ind og ud som én kontrol (kun det valgte kan nås med Tab); pilene flytter og
     vælger, Home og End går til første og sidste. o: { ariaT|aria, valg: [{id, t|tekst}], valgt, vedValg(id) }.
     vedValg kaldes, når valget ændres; vaelg(id) sætter valget udefra uden at kalde den. */
  function segment(o) {
    const n = o.valg.length;
    if (n < 2 || n > 5) throw new Error('segment: to til fem valg (fik ' + n + ')');
    const rod = el('div', 'seg');
    rod.setAttribute('role', 'radiogroup');
    aria(rod, o);
    const knapper = o.valg.map(v => {
      const b = el('button', 'seg-v', tekstAf(v));
      b.type = 'button';
      b.setAttribute('role', 'radio');
      b.dataset.valg = v.id;
      rod.appendChild(b);
      return b;
    });
    let valgt = null;
    function saet(id, fokus) {
      const b = knapper.find(x => x.dataset.valg === id) || knapper[0];
      valgt = b.dataset.valg;
      for (const x of knapper) {
        const on = x === b;
        x.setAttribute('aria-checked', String(on));
        x.tabIndex = on ? 0 : -1;
      }
      if (fokus) b.focus();
    }
    function vaelg(b, fokus) {
      const foer = valgt;
      saet(b.dataset.valg, fokus);
      if (valgt !== foer && o.vedValg) o.vedValg(valgt);
    }
    rod.addEventListener('click', e => {
      const b = e.target.closest('.seg-v');
      if (b && knapper.includes(b)) vaelg(b, true);
    });
    rod.addEventListener('keydown', e => {
      const i = knapper.indexOf(e.target);
      if (i < 0) return;
      const ny = { ArrowRight: i + 1, ArrowDown: i + 1, ArrowLeft: i - 1, ArrowUp: i - 1, Home: 0, End: n - 1 }[e.key];
      if (ny == null) return;
      e.preventDefault();
      vaelg(knapper[(ny + n) % n], true);
    });
    saet(o.valgt, false);
    return { rod, knapper, vaelg: id => saet(id, false), get valgt() { return valgt; } };
  }

  /* ── Filterchip (fra 2.3.0) ── En chip, der kan klikkes: en knap med aria-pressed, der slår et filter til og fra. Den
     almindelige chip (Kit.chip, .chip) er uændret og kan ikke klikkes. Valgt ses på flade, kant, fed skrift og et
     flueben, ikke kun på farven. o: { id, t|tekst, antal, valgt, deaktiveret, vedSkift(til, id) }. vedSkift kaldes, når
     brugeren slår chippen til eller fra. */
  function filterchip(o) {
    const b = el('button', 'fchip');
    b.type = 'button';
    if (o.id != null) b.dataset.chip = o.id;
    const m = el('span', 'fchip-mark', '✓');
    m.setAttribute('aria-hidden', 'true');
    b.append(m, el('span', null, tekstAf(o)));
    if (o.antal != null) b.appendChild(el('span', 'ant', String(o.antal)));
    b.setAttribute('aria-pressed', String(Boolean(o.valgt)));
    if (o.deaktiveret) b.disabled = true;
    b.addEventListener('click', () => {
      const til = b.getAttribute('aria-pressed') !== 'true';
      b.setAttribute('aria-pressed', String(til));
      if (o.vedSkift) o.vedSkift(til, o.id);
    });
    return b;
  }

  /* En gruppe af filterchips med et navn (role="group"). Hver chip slås til og fra for sig; nul, en eller flere kan være
     valgt. o: { ariaT|aria, chips: [{id, t|tekst, antal, deaktiveret}], valgt: [id, …], vedValg([id, …]) }. Returnerer
     { rod, chips, valgt, saetValgt }. saetValgt sætter valget udefra uden at kalde vedValg. */
  function filterchips(o) {
    const rod = el('div', 'fchips');
    rod.setAttribute('role', 'group');
    aria(rod, o);
    const foer = new Set(o.valgt || []);
    const valgte = () => chips.filter(b => b.getAttribute('aria-pressed') === 'true').map(b => b.dataset.chip);
    const chips = o.chips.map(c => {
      const b = filterchip({ id: c.id, t: c.t, tekst: c.tekst, antal: c.antal, deaktiveret: c.deaktiveret, valgt: foer.has(c.id), vedSkift: () => { if (o.vedValg) o.vedValg(valgte()); } });
      rod.appendChild(b);
      return b;
    });
    function saetValgt(ids) {
      const s = new Set(ids || []);
      for (const b of chips) b.setAttribute('aria-pressed', String(s.has(b.dataset.chip)));
    }
    return { rod, chips, saetValgt, get valgt() { return valgte(); } };
  }

  /* ── Kasse (fra 2.3.0) ── Kittets kort (card): en flade med overskrift, indhold og valgfri handling i hovedet og valgfri
     fodlinje. Klassen hedder .kasse, fordi .kort betyder noget andet i et af programmerne. En <section> med navn fra
     overskriften (aria-labelledby). o: { id, t|tekst, niveau (2–4; standard 2), handling: Node, indhold: Node, fod: Node,
     fyld }. fyld: true giver indholdet uden luft (fx en tabel helt ud til kanten). Returnerer { rod, overskrift, indhold,
     fod }; fod er null uden en fodlinje. */
  let kasseNr = 0;
  function kasse(o) {
    const nr = ++kasseNr;
    const rod = el('section', 'kasse');
    if (o.id) rod.id = o.id;
    const top = el('div', 'kasse-top');
    const niveau = [2, 3, 4].includes(o.niveau) ? o.niveau : 2;
    const h = el('h' + niveau, null, tekstAf(o));
    h.id = (o.id || 'kit-kasse' + nr) + '-h';
    rod.setAttribute('aria-labelledby', h.id);
    top.appendChild(h);
    if (o.handling) {
      const plads = el('div', 'kasse-handling');
      plads.appendChild(o.handling);
      top.appendChild(plads);
    }
    const indhold = el('div', 'kasse-indhold' + (o.fyld ? ' fyld' : ''));
    if (o.indhold) indhold.appendChild(o.indhold);
    rod.append(top, indhold);
    let fod = null;
    if (o.fod) {
      fod = el('div', 'kasse-fod');
      fod.appendChild(o.fod);
      rod.appendChild(fod);
    }
    return { rod, overskrift: h, indhold, fod };
  }

  /* ── StatusTile (fra 2.3.0) ── En flise svarer på "skal jeg gøre noget?" med ét tal, én etiket og én status
     (design/ui-kit-2/components/StatusTile). Hele flisen er en knap (ved) eller et link (href), der åbner det, tallet
     tæller; uden ved og href er den tekst og kan ikke klikkes. Status som StatusChip (symbol og ord) og en streg i
     statussens mærkefarve i venstre side, så status aldrig hviler på farven alene. o: { tal, t|tekst (etiketten),
     status: { variant, t|tekst }, under: t|tekst, udvikling: { retning: 'op' | 'ned' | 'flad', t|tekst }, ved, href,
     ariaT|aria }. Udviklingen er en pil og en tekst; pilen alene siger ikke noget. */
  const UDVIKLING = { op: '↑', ned: '↓', flad: '→' };
  function statusflise(o) {
    if (o.status && !(o.status.variant in SYMBOL)) throw new Error('ukendt status: ' + o.status.variant);
    if (o.udvikling && !(o.udvikling.retning in UDVIKLING)) throw new Error('ukendt udvikling: ' + o.udvikling.retning);
    const tag = o.href ? 'a' : o.ved ? 'button' : 'div';
    const f = el(tag, 'sflise' + (o.status ? ' ' + o.status.variant : ''));
    if (tag === 'button') {
      f.type = 'button';
      f.addEventListener('click', o.ved);
    } else if (tag === 'a') f.href = o.href;
    f.append(el('span', 'sflise-navn', tekstAf(o)), el('span', 'sflise-tal', o.tal == null ? '–' : String(o.tal)));
    if (o.status) f.appendChild(chip(o.status.variant, tekstAf(o.status)));
    if (o.udvikling) {
      const u = el('span', 'sflise-under');
      const pil = el('span', 'sflise-pil', UDVIKLING[o.udvikling.retning]);
      pil.setAttribute('aria-hidden', 'true');
      u.append(pil, el('span', null, tekstAf(o.udvikling)));
      f.appendChild(u);
    } else if (o.under) f.appendChild(el('span', 'sflise-under', tk(o.under)));
    aria(f, o);
    return f;
  }

  /* En tekst som nøgle (med data-v og data-w) eller færdig tekst sættes ind i et element, der havde en anden. */
  function saetTekst(e, v) {
    for (const a of ['data-t', 'data-v', 'data-w']) e.removeAttribute(a);
    e.textContent = '';
    const t = tk(v);
    if (t && typeof t === 'object') {
      e.setAttribute('data-t', t.t);
      if (v.v != null) e.setAttribute('data-v', String(v.v));
      if (v.w != null) e.setAttribute('data-w', String(v.w));
    } else if (t != null) e.textContent = String(t);
  }

  /* ── Tom tilstand (fra 2.3.0) ── Det, der står, når der intet er at vise: et ikon, en overskrift, én sætning og højst én
     handling. Til en tom visning, en tom tabel og "ingen træffere". o: { ikon, t|tekst (overskriften), p (sætningen),
     handling: { t|tekst, ved, variant, ikon }, lille, niveau (2–4; standard 2) }. lille: true er den lave udgave under en
     tabel eller en søgning; dens overskrift er et afsnit (ingen overskrift i sidens opbygning), og den har role="status",
     så "ingen træffere" læses op, når teksten skifter. */
  function tomTilstand(o) {
    const lille = Boolean(o.lille);
    const rod = el('div', 'tomt' + (lille ? ' lille' : ''));
    if (lille) rod.setAttribute('role', 'status');
    if (o.ikon) rod.appendChild(ikon(o.ikon, 'tomt-ikon'));
    const niveau = [2, 3, 4].includes(o.niveau) ? o.niveau : 2;
    rod.appendChild(el(lille ? 'p' : 'h' + niveau, 'tomt-h', tekstAf(o)));
    if (o.p != null) rod.appendChild(el('p', 'tomt-p', tk(o.p)));
    if (o.handling) rod.appendChild(knap(o.handling));
    return rod;
  }

  /* ── Fremdrift (fra 2.3.0) ── Viser, at programmet arbejder, med en tekst, der siger hvad. Med maks: en bestemt bjælke
     (<progress>, navngivet af teksten). Uden maks: en ubestemt ring, der snurrer, ved siden af teksten; ringen er aldrig
     den eneste besked. Teksten har role="status", så den læses op, når den skifter. omraade (det område, der arbejder)
     får aria-busy="true", indtil faerdig() kaldes. Ringen står stille ved prefers-reduced-motion. o: { t|tekst, maks,
     vaerdi, omraade }. Returnerer { rod, bjaelke, saet(vaerdi, tekst), faerdig() }; tekst er en nøgle ({ t, v, w }) eller
     færdig tekst. */
  let fremNr = 0;
  function fremdrift(o) {
    const nr = ++fremNr, bestemt = o.maks != null;
    const rod = el('div', 'frem' + (bestemt ? '' : ' ubestemt'));
    const tekst = el('span', 'frem-tekst');
    saetTekst(tekst, o.t ? o : o.tekst);
    tekst.id = 'kit-frem' + nr;
    tekst.setAttribute('role', 'status');
    let bjaelke = null;
    if (bestemt) {
      bjaelke = document.createElement('progress');
      bjaelke.className = 'frem-bjaelke';
      bjaelke.max = o.maks;
      bjaelke.value = o.vaerdi || 0;
      bjaelke.setAttribute('aria-labelledby', tekst.id);
      rod.append(tekst, bjaelke);
    } else {
      const ring = el('span', 'snurr');
      ring.setAttribute('aria-hidden', 'true');
      rod.append(ring, tekst);
    }
    if (o.omraade) o.omraade.setAttribute('aria-busy', 'true');
    return {
      rod, bjaelke,
      saet(v, ny) {
        if (bjaelke && v != null) bjaelke.value = v;
        if (ny !== undefined) saetTekst(tekst, ny);
      },
      faerdig() { if (o.omraade) o.omraade.removeAttribute('aria-busy'); },
    };
  }

  /* ── Værktøjstip (fra 2.3.0) ── En kort forklaring, der vises ved hover og ved tastaturfokus på maal, kan holdes åben
     med musen (man kan flytte musen op på tippet), og forsvinder ved Esc, når musen går væk, og når fokus går (WCAG
     1.4.13). role="tooltip"; maal får aria-describedby. Tippet står midt over maal, under det, hvis der ikke er plads
     over, og ved venstre eller højre kant, hvis det ellers går ud over vinduet: kun klasser (under, venstre-kant,
     hoejre-kant), ingen inline-stil. Kittet lægger maal i en <span class="tip-plads">, som tippet placeres ud fra (står
     maal allerede i en, bruges den). o: { t|tekst }. Returnerer { tip, plads, vis(), skjul(), synlig }. */
  let tipNr = 0;
  const TIP_VENT = 300, TIP_SLIP = 150, TIP_AFSTAND = 6, TIP_KANT = 8;
  function vaerktoejstip(maal, o) {
    const nr = ++tipNr;
    let plads = maal.parentElement;
    if (!plads || !plads.classList.contains('tip-plads')) {
      plads = el('span', 'tip-plads');
      if (maal.parentNode) maal.parentNode.insertBefore(plads, maal);
      plads.appendChild(maal);
    }
    const tip = el('span', 'tip', tekstAf(o));
    tip.id = 'kit-tip' + nr;
    tip.setAttribute('role', 'tooltip');
    tip.hidden = true;
    plads.appendChild(tip);
    const ids = (maal.getAttribute('aria-describedby') || '').split(/\s+/).filter(Boolean);
    maal.setAttribute('aria-describedby', [...ids, tip.id].join(' '));
    let visUr = null, slipUr = null, mus = false, fokus = false;
    function placer() {
      tip.classList.remove('under', 'venstre-kant', 'hoejre-kant');
      const m = maal.getBoundingClientRect(), t = tip.getBoundingClientRect();
      const vb = window.innerWidth || document.documentElement.clientWidth;
      if (m.top - TIP_AFSTAND - t.height < TIP_KANT) tip.classList.add('under');
      const midt = (m.left + m.right) / 2;
      if (midt - t.width / 2 < TIP_KANT) tip.classList.add('venstre-kant');
      else if (midt + t.width / 2 > vb - TIP_KANT) tip.classList.add('hoejre-kant');
    }
    function esc(e) {
      if (e.key !== 'Escape' || tip.hidden) return;
      e.preventDefault();
      e.stopPropagation();
      skjul();
    }
    function vis() {
      clearTimeout(visUr);
      clearTimeout(slipUr);
      if (!tip.hidden) return;
      tip.hidden = false;
      placer();
      document.addEventListener('keydown', esc, true);
    }
    function skjul() {
      clearTimeout(visUr);
      clearTimeout(slipUr);
      if (tip.hidden) return;
      tip.hidden = true;
      document.removeEventListener('keydown', esc, true);
    }
    const tastatur = () => { try { return maal.matches(':focus-visible'); } catch (e) { return true; } };
    plads.addEventListener('mouseenter', () => {
      mus = true;
      clearTimeout(slipUr);
      if (tip.hidden) visUr = setTimeout(vis, TIP_VENT);
    });
    plads.addEventListener('mouseleave', () => {
      mus = false;
      clearTimeout(visUr);
      if (!fokus) slipUr = setTimeout(skjul, TIP_SLIP);
    });
    maal.addEventListener('focus', () => {
      fokus = true;
      if (tastatur()) vis();
    });
    maal.addEventListener('blur', () => {
      fokus = false;
      if (!mus) skjul();
    });
    return { tip, plads, vis, skjul, get synlig() { return !tip.hidden; } };
  }

  /* ── Afkrydsning, kontakt, radiogruppe og valgliste (fra 2.3.0) ── Formularens valg på browserens egne <input> og
     <select> med kittets udseende (appearance: none og kittets tegning): ingen hjemmebyggede roller, så tastatur,
     etiket, formular og skærmlæser virker som i browseren. Hjælp eller fejl står under valget med aria-describedby; en
     fejl giver aria-invalid. */
  let valgNr = 0;
  function hjaelpUnder(rod, inp, id, o, klasse) {
    if (o.fejl == null && o.hjaelp == null) return null;
    const hj = el('span', klasse + (o.fejl != null ? ' fejltekst' : ''), tk(o.fejl != null ? o.fejl : o.hjaelp));
    hj.id = id + '-hj';
    inp.setAttribute('aria-describedby', hj.id);
    if (o.fejl != null) inp.setAttribute('aria-invalid', 'true');
    rod.appendChild(hj);
    return hj;
  }
  /* En række med en boks og en tekst: <label class="afk"><input class="afk-boks">…</label> i en .afk-felt. */
  function boksRaekke(type, o, id) {
    const rod = el('div', 'afk-felt');
    const lab = el('label', 'afk');
    const inp = document.createElement('input');
    inp.type = type;
    inp.id = id;
    inp.className = 'afk-boks';
    if (o.navn) inp.name = o.navn;
    if (o.vaerdi != null) inp.value = String(o.vaerdi);
    if (o.deaktiveret) inp.disabled = true;
    lab.append(inp, el('span', 'afk-tekst', tekstAf(o)));
    rod.appendChild(lab);
    hjaelpUnder(rod, inp, id, o, 'afk-hj');
    return { rod, inp };
  }

  /* Afkrydsning: <input type="checkbox">. o: { id, t|tekst, valgt, delvis, deaktiveret, navn, vaerdi, hjaelp, fejl,
     vedSkift(valgt) }. delvis: true er "delvis valgt" (indeterminate), fx en "Vælg alle" over en liste, hvor nogle er
     valgt; et klik gør den hel. Returnerer { rod, input, valgt, saetValgt(v), saetDelvis() }. */
  function afkrydsning(o) {
    const { rod, inp } = boksRaekke('checkbox', o, o.id || 'kit-afk-' + ++valgNr);
    inp.checked = Boolean(o.valgt);
    if (o.delvis) inp.indeterminate = true;
    if (o.vedSkift) inp.addEventListener('change', () => o.vedSkift(inp.checked));
    return {
      rod, input: inp,
      get valgt() { return inp.checked; },
      saetValgt(v) { inp.checked = Boolean(v); inp.indeterminate = false; },
      saetDelvis() { inp.indeterminate = true; },
    };
  }

  /* Kontakt (til/fra): <input type="checkbox" role="switch">. Valgt frem for <button role="switch" aria-checked>, fordi
     input'en har tilstanden (checked), mellemrum, etiketten, formularen og change fra browseren, og skærmlæsere siger
     "kontakt, til/fra". Til og fra ses på knoppens plads (venstre/højre) og fladen, og med ord: { til, fra } også på et
     ord ved siden af (aria-hidden; kontakten siger selv sin tilstand). Begge ord står i markup, og kit.css viser det, der
     passer (:checked), så et sprogskift aldrig skal bygge kontakten om. En kontakt virker straks; noget, der først gælder
     efter Gem, er en afkrydsning. o: { id, t|tekst, til, deaktiveret, navn, hjaelp, fejl, ord: { til, fra },
     vedSkift(til) }. Returnerer { rod, input, til, saet(v) }. */
  function kontakt(o) {
    const id = o.id || 'kit-tf-' + ++valgNr;
    const rod = el('div', 'afk-felt');
    const lab = el('label', 'afk');
    const inp = document.createElement('input');
    inp.type = 'checkbox';
    inp.id = id;
    inp.className = 'tilfra';
    inp.setAttribute('role', 'switch');
    if (o.navn) inp.name = o.navn;
    if (o.deaktiveret) inp.disabled = true;
    inp.checked = Boolean(o.til);
    lab.append(inp, el('span', 'afk-tekst', tekstAf(o)));
    if (o.ord) {
      const ord = el('span', 'tilfra-ord');
      ord.setAttribute('aria-hidden', 'true');
      ord.append(el('span', 'tilfra-til', tk(o.ord.til)), el('span', 'tilfra-fra', tk(o.ord.fra)));
      lab.appendChild(ord);
    }
    rod.appendChild(lab);
    hjaelpUnder(rod, inp, id, o, 'afk-hj');
    if (o.vedSkift) inp.addEventListener('change', () => o.vedSkift(inp.checked));
    return {
      rod, input: inp,
      get til() { return inp.checked; },
      saet(v) { inp.checked = Boolean(v); },
    };
  }

  /* Radiogruppe: <fieldset> med <legend> og én <input type="radio"> pr. valg (samme name). Pilene flytter og vælger, og
     Tab går ind og ud som ét stop: browserens egen opførsel. o: { etiket, navn, valg: [{ id, t|tekst, hjaelp,
     deaktiveret }], valgt, vandret, hjaelp, fejl, vedValg(id) }. Returnerer { rod, inputs, valgt, vaelg(id) }; vaelg kalder
     ikke vedValg. */
  function radiogruppe(o) {
    const nr = ++valgNr, navn = o.navn || 'kit-radio-' + nr;
    const rod = el('fieldset', 'valgsaet' + (o.vandret ? ' vandret' : ''));
    rod.appendChild(el('legend', 'label', tk(o.etiket)));
    const inputs = o.valg.map((v, i) => {
      const { rod: r, inp } = boksRaekke('radio', { ...v, navn, vaerdi: v.id }, `kit-radio-${nr}-${i + 1}`);
      inp.checked = v.id === o.valgt;
      rod.appendChild(r);
      return inp;
    });
    /* Hjælp eller fejl under gruppen, med aria-describedby på <fieldset>. aria-invalid hører ikke til en gruppe eller en
       radioknap; en fejl ses på teksten og (kit.css) på boksenes kant. */
    hjaelpUnder(rod, rod, 'kit-radio-' + nr, o, 'afk-hj');
    rod.removeAttribute('aria-invalid');
    rod.addEventListener('change', e => { if (inputs.includes(e.target) && o.vedValg) o.vedValg(e.target.value); });
    return {
      rod, inputs,
      get valgt() { const v = inputs.find(x => x.checked); return v ? v.value : null; },
      vaelg(id) { for (const x of inputs) x.checked = x.value === id; },
    };
  }

  /* Valgliste: browserens egen <select> i en .felt med etiket over og hjælp eller fejl under, med kittets kant, flade og
     pil. Til ét valg blandt mange i en formular; listen åbner som styresystemets. o: { id, etiket, valg: [{ id, t|tekst,
     deaktiveret }], valgt, deaktiveret, hjaelp, fejl, vedValg(id) }. Returnerer { rod, select, valgt, vaelg(id) }; vaelg
     kalder ikke vedValg. */
  function valgliste(o) {
    const id = o.id || 'kit-vl-' + ++valgNr;
    const rod = el('div', 'felt');
    const lab = el('label', 'label', tk(o.etiket));
    lab.htmlFor = id;
    const ramme = el('span', 'vliste');
    const sel = document.createElement('select');
    sel.id = id;
    for (const v of o.valg) {
      const op = document.createElement('option');
      op.value = v.id;
      if (v.t) op.setAttribute('data-t', v.t);
      else op.textContent = String(v.tekst);
      if (v.deaktiveret) op.disabled = true;
      sel.appendChild(op);
    }
    if (o.valgt != null) sel.value = o.valgt;
    if (o.deaktiveret) sel.disabled = true;
    ramme.append(sel, ikon('ned'));
    rod.append(lab, ramme);
    hjaelpUnder(rod, sel, id, o, 'hj');
    if (o.vedValg) sel.addEventListener('change', () => o.vedValg(sel.value));
    return { rod, select: sel, get valgt() { return sel.value; }, vaelg(v) { sel.value = v; } };
  }

  /* ── Dialog (fra 2.3.0) ── En modal dialog på browserens <dialog> og showModal(): fokusfælden (resten af siden er inert),
     Esc og den mørke baggrund er browserens. Overskrift (h2), en sætning og/eller frit indhold, fodlinje med knapper (den
     primære står til højre; en farlig handling er variant 'danger') og en valgfri lukkeknap. aria-labelledby fra
     overskriften og aria-describedby fra sætningen. Ved åbning får det element fokus, programmet peger på (fokus), ellers
     det første felt i indholdet, ellers knappen, der ikke er farlig, når der er en farlig, ellers den primære. Ved lukning
     går fokus tilbage til det, der havde det. o: { t|tekst, p, indhold: Node, knapper: [{ id, t|tekst, variant, ikon, svar,
     ved(dlg) }], luk: { ariaT|aria }, fokus: id | Node, bred, vedLuk(svar) }. Lukkeknappen skal have et navn (kittet har
     ingen tekster); luk uden navn giver en fejl. Et klik på en knap kalder ved; giver
     den false, bliver dialogen åben (fx ved en fejl i en formular), ellers lukker den med knappens svar (standard: id).
     Esc giver svaret 'esc', lukkeknappen 'luk' og luk() fra koden 'kode'. Returnerer { rod, aabn(fokus), luk(svar),
     aaben, indhold, overskrift, knap(id) }. */
  let dlgNr = 0;
  function dialog(o) {
    const nr = ++dlgNr;
    const rod = el('dialog', 'modal' + (o.bred ? ' bred' : ''));
    const top = el('div', 'modal-top');
    const h = el('h2', null, tekstAf(o));
    h.id = 'kit-dlg' + nr + '-h';
    h.tabIndex = -1;
    rod.setAttribute('aria-labelledby', h.id);
    top.appendChild(h);
    let lukKnap = null;
    if (o.luk) {
      if (typeof o.luk !== 'object' || !(o.luk.ariaT || o.luk.aria)) throw new Error('dialog: lukkeknappen skal have et navn (luk: { ariaT } eller { aria })');
      lukKnap = el('button', 'modal-luk');
      lukKnap.type = 'button';
      lukKnap.appendChild(ikon('luk'));
      aria(lukKnap, o.luk);
      lukKnap.addEventListener('click', () => luk('luk'));
      top.appendChild(lukKnap);
    }
    rod.appendChild(top);
    if (o.p != null) {
      const p = el('p', 'modal-tekst', tk(o.p));
      p.id = 'kit-dlg' + nr + '-p';
      rod.setAttribute('aria-describedby', p.id);
      rod.appendChild(p);
    }
    const indhold = el('div', 'modal-indhold');
    if (o.indhold) indhold.appendChild(o.indhold);
    rod.appendChild(indhold);
    const knapper = {};
    const fod = el('div', 'modal-fod');
    const liste = (o.knapper || []).slice();
    const pri = liste.findIndex(k => k.variant === 'primary');
    if (pri >= 0) liste.push(liste.splice(pri, 1)[0]);
    let api = null;
    for (const k of liste) {
      const b = knap({ t: k.t, tekst: k.tekst, variant: k.variant, ikon: k.ikon, ved: () => {
        if (k.ved && k.ved(api) === false) return;
        luk(k.svar != null ? k.svar : k.id);
      } });
      if (k.id != null) {
        b.dataset.svar = k.id;
        knapper[k.id] = b;
      }
      fod.appendChild(b);
    }
    if (liste.length) rod.appendChild(fod);
    let fra = null, svar = 'kode';
    function foersteFokus(f) {
      if (f && typeof f === 'object') return f;
      if (f != null) return knapper[f] || document.getElementById(f);
      const felt = indhold.querySelector('input:not([type="hidden"]):not([disabled]), select:not([disabled]), textarea:not([disabled])');
      if (felt) return felt;
      const alle = [...fod.querySelectorAll('button:not([disabled])')];
      if (alle.some(b => b.classList.contains('danger'))) return alle.find(b => !b.classList.contains('danger')) || lukKnap || h;
      return alle.find(b => b.classList.contains('primary')) || alle[alle.length - 1] || lukKnap || h;
    }
    function aabn(fokus) {
      if (rod.open) return;
      if (!rod.isConnected) document.body.appendChild(rod);
      const a = document.activeElement;
      fra = a && a !== document.body ? a : null;
      svar = 'kode';
      rod.showModal();
      const maal = foersteFokus(fokus !== undefined ? fokus : o.fokus);
      if (maal) maal.focus();
    }
    function luk(s) {
      if (!rod.open) return;
      svar = s == null ? 'kode' : s;
      rod.close(String(svar));
    }
    rod.addEventListener('cancel', () => { svar = 'esc'; });
    rod.addEventListener('close', () => {
      const s = svar, tilbage = fra;
      svar = 'kode';
      fra = null;
      if (tilbage && tilbage.isConnected && typeof tilbage.focus === 'function') tilbage.focus();
      if (o.vedLuk) o.vedLuk(s);
    });
    api = { rod, aabn, luk: s => luk(s == null ? 'kode' : s), indhold, overskrift: h, knap: id => knapper[id], get aaben() { return rod.open; } };
    return api;
  }

  /* Bekræftelse som kort vej ("Er du sikker?"): en dialog med en sætning og to knapper, nej til venstre og ja til højre.
     o: { t|tekst, p, ja, nej, farlig, udfyld(rod) }, hvor ja og nej er { t } eller { tekst }. Returnerer et løfte: true
     ved ja, false ved nej, Esc og luk. farlig: true gør ja til en farlig knap og giver nej fokus; ellers er ja primær og
     har fokus. udfyld kaldes med dialogen, lige før den åbner (fx rod => Sprog.anvend(rod), når teksterne er nøgler).
     Dialogen fjernes fra siden igen, når den er lukket. */
  function bekraeft(o) {
    const spec = v => (v && typeof v === 'object' ? { t: v.t, tekst: v.tekst } : { tekst: v });
    return new Promise(svar => {
      const d = dialog({
        t: o.t, tekst: o.tekst, p: o.p, fokus: o.farlig ? 'nej' : 'ja',
        knapper: [{ id: 'nej', ...spec(o.nej) }, { id: 'ja', ...spec(o.ja), variant: o.farlig ? 'danger' : 'primary' }],
        vedLuk: s => {
          d.rod.remove();
          svar(s === 'ja');
        },
      });
      if (o.udfyld) o.udfyld(d.rod);
      d.aabn();
    });
  }

  /* En række fliser, der ombrydes: <ul role="list"> med én <li> pr. flise. o: { ariaT|aria, fliser: [o | Node, …] }. */
  function statusfliser(o) {
    const ul = el('ul', 'sfliser');
    ul.setAttribute('role', 'list');
    aria(ul, o);
    for (const f of o.fliser) {
      const li = document.createElement('li');
      li.appendChild(f instanceof Node ? f : statusflise(f));
      ul.appendChild(li);
    }
    return ul;
  }

  window.Kit = Object.freeze({ IKONER, ikon, registrerIkoner, chip, knap, felt, banner, tabel, commandBar, navTrae, visningsskift, guide, fletbaand, SYMBOL, vaelger, sidepanel, segment, filterchip, filterchips, kasse, statusflise, statusfliser, tomTilstand, fremdrift, vaerktoejstip, afkrydsning, kontakt, radiogruppe, valgliste, dialog, bekraeft, sideskift });
})();
