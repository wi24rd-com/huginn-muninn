/* Wi24rd UI Kit 2 — tema (window.Tema). localStorage['<præfiks>-tema'] = lys | moerk | system (standard: system) →
   <html data-tema>, som vælger farverne i tokens.css. Står uden defer i <head>, så temaet er sat, før siden tegnes første
   gang (intet hvidt blink i mørkt tema). Et skift melder '<præfiks>-tema' på document.
   Præfikset er programmets eget og står på <html data-kit-praefiks="…"> (standard: wi24rd), så to programmer i samme
   webvisning aldrig deler valg. Mønsteret er Huginn and Muninns (ui/assets/tema.js). */
(function () {
  'use strict';
  const NOEGLE = (document.documentElement.getAttribute('data-kit-praefiks') || 'wi24rd') + '-tema';
  const VALG = ['lys', 'moerk', 'system'];
  let valg = 'system';
  try {
    const v = localStorage.getItem(NOEGLE);
    if (VALG.includes(v)) valg = v;
  } catch (e) { /* ingen lagring (fx privat tilstand): temaet følger systemet */ }
  document.documentElement.setAttribute('data-tema', valg);
  window.Tema = {
    VALG,
    get valg() { return valg; },
    saet(v) {
      valg = VALG.includes(v) ? v : 'system';
      try { localStorage.setItem(NOEGLE, valg); } catch (e) { /* valget gælder kun denne gang */ }
      document.documentElement.setAttribute('data-tema', valg);
      document.dispatchEvent(new CustomEvent(NOEGLE));
    },
  };
})();
