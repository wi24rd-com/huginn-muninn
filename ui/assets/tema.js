/* Huginn and Muninn — udseendet (Indstillinger → Udseende). localStorage['hm-tema'] = system | light | dark; System = ingen
   <html data-theme>, så tokens.css følger styresystemet. Står uden defer i <head>, så temaet er sat, før siden tegnes første
   gang (ingen hvid blink i mørk). Skiftet virker straks: alle farver er CSS-variabler.
   Tekststørrelsen (Indstillinger → Tekststørrelse, ⌘+ ⌘− ⌘0) følger samme mønster: localStorage['hm-tekst'] = compact | normal |
   large → <html data-density>, som vælger tekstskalaen i tokens.css. Et skift melder 'hm-tekst' på document. */
(function () {
  'use strict';
  const NOEGLE = 'hm-tema';
  let valg = 'system';
  function anvend() {
    if (valg === 'system') document.documentElement.removeAttribute('data-theme');
    else document.documentElement.setAttribute('data-theme', valg);
  }
  try { const v = localStorage.getItem(NOEGLE); if (v === 'light' || v === 'dark') valg = v; } catch (e) { }
  anvend();
  window.hmTema = {
    get valg() { return valg; },
    saet(v) {
      valg = v === 'light' || v === 'dark' ? v : 'system';
      try { localStorage.setItem(NOEGLE, valg); } catch (e) { }
      anvend();
    },
  };

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
