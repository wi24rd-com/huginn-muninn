/* Huginn and Muninn — velkomsten: ravnene, navnet, sloganet og "Det her kan du forvente". Ét sæt byggeklodser, tegnet i hvert element med data-velkomst:
   Overblik, når intet domæne er valgt ("intet-valgt", med teksten og knappen til Indstillinger), og Om ("om", uden knap) har det hele; Angreb, Robotter,
   Login og dyk viser den samme (intetValgt()), for det er samme tilstand. Opsætningen er
   delt, så feltet til adgangsnøglen er synligt uden at rulle: "opsaetning-top" (kompakt logo og navn, som ikke er en overskrift, for sidens h1 er "Forbind med
   Cloudflare"), formularen, "opsaetning-punkter" og "filosofi". Filosofien står kun i opsætningen og bærer sloganet dér. Ravnene er masken .tom-ravn, som
   følger temaet; al tekst er HTML, så den kan oversættes og følger Tekststørrelse. "for Cloudflare®" står altid mindre end navnet og med lille f (Cloudflares
   varemærkeregler), også selv om logoet har stort F. Statusordene er programmets egne (T.status), og landet er det gemte (land(), visning-land.js) eller
   "dit land". Ikonerne er klonet fra båndets knapper og Indstillinger. Bygges med createElement. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, R = window.hmRamme, V = window.hmTegn, { el, fmt } = R;
  const nav = v => document.querySelector(`#baand button[data-visning="${v}"] svg`);
  const klokke = () => { const h = document.getElementById('indst-notif'); return h && h.previousElementSibling; }; // kassens ikon står foran overskriften
  /* Skjold, klokke, gitter, ur og lås. */
  const IKONER = [() => nav('vaernet'), klokke, () => nav('overblik'), () => nav('siden-sidst'), () => nav('login')];
  const saetninger = [];
  let land = null;
  /* Ikonerne i visningsskiftet er kittets (uden klasse); kopien får programmets .ik, så den følger teksten her. */
  const ikon = hent => { const s = hent(); if (!s) return []; const k = s.cloneNode(true); k.setAttribute('class', 'ik'); return [k]; };
  const udfyld = s => fmt(s, { groen: T.status.groen, gul: T.status.gul, roed: T.status.roed, land: land || T.velkomstLand });
  function punkt([fed, rest], i) {
    const span = el('span', { textContent: udfyld(rest) });
    saetninger.push([span, rest]);
    return el('li', {}, ...ikon(IKONER[i]), el('p', {}, el('b', { textContent: fed }), span));
  }
  /* Det, der hører til stedet. */
  function situation(sted) {
    if (sted !== 'intet-valgt') return [];
    const knap = V.knap(T.intetValgtKnap, { gaa: 'indstillinger?afsnit=domaener' }, 'btn primary', T.intetValgtKnap);
    return [el('div', { className: 'velkomst-situation' }, el('p', {}, el('b', { textContent: T.intetValgtH }), ' ', T.intetValgtP), knap)];
  }
  function ravne() {
    const r = el('div', { className: 'tom-ravn' });
    r.setAttribute('aria-hidden', 'true');
    return r;
  }
  const navn = () => el('p', { className: 'velkomst-navn' },
    el('span', { className: 'velkomst-wi' }, 'Wi', el('em', { textContent: '24' }), 'rd'),
    el('span', { className: 'velkomst-hm', textContent: 'Huginn and Muninn' }),
    el('span', { className: 'velkomst-cf', textContent: T.under }));
  const forvent = () => [el('h2', { className: 'velkomst-h', textContent: T.forventH }), el('ul', { className: 'velkomst-punkter' }, ...T.forvent.map(punkt))];
  /* Rubrikken står med små bogstaver i teksten; versalerne er CSS. */
  const filosofi = () => el('div', { className: 'filosofi' }, el('h2', { className: 'filosofi-h', textContent: T.filosofi.h }), el('p', { className: 'filosofi-slogan', textContent: T.slogan }), ...T.filosofi.afsnit.map(t => el('p', { textContent: t })));
  function byg(sted) {
    if (sted === 'opsaetning-top') return el('div', { className: 'velkomst-kompakt' }, ravne(), navn());
    if (sted === 'opsaetning-punkter') return el('div', { className: 'velkomst' }, ...forvent());
    if (sted === 'filosofi') return filosofi();
    return el('div', { className: 'velkomst' }, ravne(), navn(), el('p', { className: 'slogan', textContent: T.slogan }), ...forvent(), ...situation(sted));
  }
  document.querySelectorAll('[data-velkomst]').forEach(e => e.replaceChildren(byg(e.dataset.velkomst)));
  window.hmVelkomst = {
    intetValgt: () => byg('intet-valgt'),
    land(navn) {
      land = navn;
      for (const [span, rest] of saetninger) span.textContent = udfyld(rest);
      saetninger.splice(0, saetninger.length, ...saetninger.filter(([span]) => span.isConnected)); // visningerne tegner om
    },
  };
})();
