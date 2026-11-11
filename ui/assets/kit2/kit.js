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
     Hele rækken vælges med klik og piletaster (op/ned, Home/End); vedValg(række, data). */
  function tabel(o) {
    const t = el('table', 'tabel');
    const thead = document.createElement('thead');
    const hr = document.createElement('tr');
    for (const k of o.kolonner) {
      const th = el('th', k.tal ? 'r' : null, tekstAf(k));
      th.scope = 'col';
      hr.appendChild(th);
    }
    thead.appendChild(hr);
    const tbody = document.createElement('tbody');
    const valgbare = [];
    for (const r of o.raekker || []) {
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
    t.append(thead, tbody);
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
    return t;
  }

  /* ── CommandBar ── faner: [{id, t, grupper: [{t, knapper: [{id, t, ikon, pri, deaktiveret, hvorfor, ved, trykket}]}]}].
     Fanerne skifter båndets indhold, aldrig visningen. En deaktiveret kommando står nedtonet med aria-disabled (så den
     stadig kan nås med Tab og forklarer hvorfor i title); den forsvinder ikke. Returnerer { rod, vaelgFane, knap(id) }. */
  let cbNr = 0;
  function commandBar(rod, o) {
    rod.classList.add('cmd');
    rod.textContent = '';
    const nr = ++cbNr;
    const tabs = el('div', 'tabs');
    tabs.setAttribute('role', 'tablist');
    if (o.ariaT) tabs.setAttribute('data-t-aria', o.ariaT);
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
      panel.setAttribute('role', 'tabpanel');
      panel.setAttribute('aria-labelledby', tab.id);
      panel.dataset.fane = f.id;
      for (const g of f.grupper) {
        const grp = el('div', 'grp');
        grp.setAttribute('role', 'group');
        const kn = el('div', 'kn');
        for (const k of g.knapper) {
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
    rod.append(tabs, ...paneler);
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

  /* ── NavPane: visningsskift ── visninger: [{id, t, ikon, genvej}] (højst seks, altid samme rækkefølge). Hver er et link
     til #id; den aktive har aria-current="page". markér(id) sætter den aktive (programmet ejer skiftet). */
  function visningsskift(rod, visninger, o = {}) {
    rod.classList.add('skift');
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

  window.Kit = Object.freeze({ IKONER, ikon, registrerIkoner, chip, knap, felt, banner, tabel, commandBar, navTrae, visningsskift, guide, fletbaand, SYMBOL });
})();
