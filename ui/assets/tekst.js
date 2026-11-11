/* Huginn and Muninn — tekststørrelsen (Indstillinger → Tekststørrelse, ⌘+ ⌘− ⌘0): localStorage['hm-tekst'] = compact | normal |
   large → <html data-density>, som vælger tekstskalaen i tokens.css. Et skift melder 'hm-tekst' på document.
   Udseendet (lys, mørk, system) ejes af Wi24rd UI Kit 2 (assets/kit2/tema.js, window.Tema, <html data-tema>). Denne fil står
   lige før den, uden defer, og oversætter én gang et valg gemt før Kit 2: localStorage['hm-tema'] light → lys, dark → moerk. */
(function () {
  'use strict';
  try {
    const gammelt = { light: 'lys', dark: 'moerk' }[localStorage.getItem('hm-tema')];
    if (gammelt) localStorage.setItem('hm-tema', gammelt);
  } catch (e) { }

  const TEKST = 'hm-tekst', TRIN = ['compact', 'normal', 'large'];
  let tekst = 'normal';
  try { const v = localStorage.getItem(TEKST); if (TRIN.includes(v)) tekst = v; } catch (e) { }
  document.documentElement.setAttribute('data-density', tekst);
  window.hmTekst = {
    get valg() { return tekst; },
    saet(v) {
      tekst = TRIN.includes(v) ? v : 'normal';
      try { localStorage.setItem(TEKST, tekst); } catch (e) { }
      document.documentElement.setAttribute('data-density', tekst);
      document.dispatchEvent(new CustomEvent('hm-tekst'));
    },
    /* 1 = større, -1 = mindre (stopper ved Stor og Lille), 0 = Normal. */
    trin(retning) { this.saet(retning ? TRIN[Math.min(TRIN.length - 1, Math.max(0, TRIN.indexOf(tekst) + retning))] : 'normal'); },
  };
})();
