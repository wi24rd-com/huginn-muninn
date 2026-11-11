/* Huginn and Muninn — valg gemt før Wi24rd UI Kit 2, oversat én gang ved start. Udseendet og tekststørrelsen ejes af kittet
   (assets/kit2/tema.js: window.Tema → <html data-tema>, window.Tekst → <html data-tekst>; genvejene ⌘+ ⌘− ⌘0 står i ramme.js).
   Denne fil står lige før kittets, uden defer, og retter de gamle værdier i localStorage, så kittet læser sine egne:
   hm-tema light → lys, dark → moerk; hm-tekst compact → lille, large → stor. */
(function () {
  'use strict';
  const GAMLE = { 'hm-tema': { light: 'lys', dark: 'moerk' }, 'hm-tekst': { compact: 'lille', large: 'stor' } };
  try {
    for (const [noegle, nye] of Object.entries(GAMLE)) {
      const v = localStorage.getItem(noegle);
      if (v !== null && Object.prototype.hasOwnProperty.call(nye, v)) localStorage.setItem(noegle, nye[v]);
    }
  } catch (e) { }
})();
