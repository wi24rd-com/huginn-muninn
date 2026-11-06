/* Huginn and Muninn — kobler selv på igen. Én hentning ad gangen (et nyt forsøg starter aldrig, mens et kører; Rust-delen
   har desuden en lås pr. zone). Ved "ingen forbindelse" prøves igen efter 15 s, 30 s, 60 s og derefter hvert 2. minut,
   og straks når webvisningen får 'online'-hændelsen eller brugeren trykker "Prøv igen nu". Stopper, når et kald lykkes,
   og ved fejl, der ikke er netværk (fx et afvist token).
   hmGenforsoeg(boks, hent): boks = .hm-genforsoeg med .hm-gf-tekst og en knap; hent = async () => 'ok' | 'net' | 'stop'.
   fjern() afmelder lytterne og ure (Angreb-visningen laver en ny for hvert domæne); et forsøg, der stadig er i gang, rører
   derefter ikke boksen. */
(function () {
  'use strict';
  const TRIN = [15, 30, 60], DEREFTER = 120;
  window.hmGenforsoeg = function (boks, hent) {
    const T = window.hmSprog.T;
    const tekst = boks.querySelector('.hm-gf-tekst'), knap = boks.querySelector('button');
    let koerer = false, fjernet = false, forsoeg = 0, timer = null, ur = null, naeste = 0;
    const stopUr = () => { clearTimeout(timer); clearInterval(ur); timer = ur = null; };
    const nedtael = () => { tekst.textContent = T.proeverOm.replace('{s}', Math.max(0, Math.ceil((naeste - Date.now()) / 1000))); };
    async function koer() {
      if (koerer) return;
      koerer = true;
      stopUr();
      if (forsoeg) { tekst.textContent = T.proeverNu; knap.disabled = true; }
      let r;
      try { r = await hent(); } catch (e) { r = 'stop'; }
      koerer = false;
      if (fjernet) return;
      knap.disabled = false;
      if (r === 'net') {
        const s = forsoeg < TRIN.length ? TRIN[forsoeg] : DEREFTER;
        forsoeg++;
        naeste = Date.now() + s * 1000;
        boks.hidden = false;
        nedtael();
        ur = setInterval(nedtael, 1000);
        timer = setTimeout(koer, s * 1000);
      } else afbryd();
    }
    function afbryd() { stopUr(); forsoeg = 0; boks.hidden = true; }
    const online = () => { if (timer) koer(); };
    knap.addEventListener('click', koer);
    window.addEventListener('online', online);
    function fjern() { fjernet = true; afbryd(); knap.disabled = false; knap.removeEventListener('click', koer); window.removeEventListener('online', online); }
    return { start: koer, afbryd, fjern };
  };
})();
