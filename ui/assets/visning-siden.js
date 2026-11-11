/* Huginn and Muninn — Siden sidst: domæner, der kræver handling eller skal holdes øje med, hver for sig (med "ny", hvis de
   er nye siden sidst), resten samlet i én linje pr. slags. Et domænekort åbner domænets Angreb, chippen guiden, og en samlelinje
   Overblik (rammens data-gaa). En ny AI-robot får sin egen linje, som åbner robotten i dyk. Visningen åbner af sig selv, når Rust siger det (rammen spørger
   siden_sidst ved start og ved hændelsen 'siden_sidst' fra menulinjen eller en notifikation); den kan også åbnes fra
   sidebjælken (⌘2). Når den er vist, markeres den som set (siden_sidst_set). Bygges med createElement/textContent. */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, R = window.hmRamme;
  const $ = id => document.getElementById(id);
  const { el, fmt } = R;
  const invoke = R.invoke;

  const usaedvanlig = r => !r.gammel && (r.status === 'roed' || r.status === 'gul' || Boolean(r.moenster));
  /* Linjen under domænet er Overblikkets "Hvorfor" (samme tekst og samme title med CVE-numre). */
  function underLinje(r) {
    const small = el('small', { textContent: R.hvorfor(r) });
    const top = r.angreb_i_gaar_top || [];
    if (top.length) small.title = top.map(x => `${S.tal(x.n)} ${x.navn}`).join(', ');
    return small;
  }
  const tegnHuller = () => $('siden-huller').replaceChildren(...R.hulLinjer());
  R.paa('huller', tegnHuller);
  function tegn(d) {
    tegnHuller();
    const liste = $('siden-liste'), rk = d.raekker || [], nye = new Set(d.nye || []), nat = rk.some(r => r.forgaars || r.opdateres);
    $('siden-vist').textContent = `${d.vist ? fmt(nat ? T.sidenVistNat : T.sidenVist, { t: S.kl(d.vist) }) : nat ? T.sidenFoersteNat : T.sidenFoerste} ${T.sidenUtc}`;
    liste.replaceChildren();
    const linje = (tekst, klasse) => liste.append(el('li', { className: klasse || '' }, window.hmTegn.knap(tekst, { gaa: 'overblik' }, 'link hm-siden-linje')));
    const orden = { roed: 0, gul: 1, groen: 2 }, usaed = rk.filter(usaedvanlig).sort((a, b) => orden[a.status] - orden[b.status]);
    const gamle = rk.filter(r => r.gammel), groenne = rk.filter(r => r.status === 'groen' && !r.moenster), graa = rk.filter(r => r.status === 'graa');
    for (const r of usaed) {
      const chip = window.hmTegn.chip(r.status, T.status[r.status], 'button');
      chip.title = T.gTitel;
      chip.setAttribute('aria-label', `${T.status[r.status]}: ${T.gTitel} (${r.navn})`);
      chip.addEventListener('click', () => R.guide(r.id));
      const aabn = window.hmTegn.knap(window.hmTegn.domaene(r.navn, 'b'), { gaa: 'vaernet?' + new URLSearchParams({ zone: r.id, navn: r.navn }) }, 'hm-siden-aabn', fmt(T.aabnFor, { navn: r.navn }));
      const li = el('li', { className: 'hm-siden-' + r.status }, chip, aabn);
      if (r.moenster) li.append(' ', window.hmTegn.moensterKnap(r));
      if (nye.has(r.id)) li.append(el('span', { className: 'ny', textContent: T.ny }));
      const under = underLinje(r);
      aabn.title = [r.navn, under.title].filter(Boolean).join(' · '); // knappen ligger hen over kortet, så tooltippen skal sidde på den
      li.append(under);
      liste.append(li);
    }
    if (!usaed.length && groenne.length) {
      const a = groenne.reduce((x, r) => x + (r.angreb_i_gaar || 0), 0);
      const blok = groenne.reduce((x, r) => x + ((r.angreb_fordeling || {}).blokeret || 0), 0);
      linje(fmt(T.sidenRoligt, { n: T.domaener(groenne.length), a: S.tal(a), b: a && blok === a ? T.sidenAlleBlok : '' }), 'hm-siden-roligt');
    } else if (groenne.length) linje(fmt(T.sidenGroenne, { n: T.domaener(groenne.length) }));
    if (gamle.length) linje(fmt(T.sidenGamle, { n: T.domaener(gamle.length) }));
    if (graa.length) linje(fmt(T.sidenGraa, { n: T.domaener(graa.length) }));
    for (const r of d.nye_robotter || []) {
      const q = new URLSearchParams({ zone: r.zone, navn: r.domaene, robot: r.robot, periode: 'alt' }); // hele døgn (robotter::nye_robotter): klikket fører aldrig til 0
      liste.append(el('li', { className: 'hm-siden-robot' }, window.hmTegn.knap(fmt(T.sidenNyRobot, { robot: r.robot, domaene: r.domaene }), q.toString(), 'link hm-siden-linje')));
    }
  }
  /* Rammen har lige hentet siden_sidst (åbnet af Rust): brug det. Ellers (sidebjælken, ⌘2) hent nu. */
  async function vis() {
    let d = R.tilstand.siden;
    R.tilstand.siden = d ? { ...d, brugt: true } : d;
    if (!d || d.brugt) {
      try { d = await invoke('siden_sidst'); } catch (e) { return; }
      if (!d) return;
      R.tilstand.siden = { ...d, brugt: true };
    }
    tegn(d);
    $('siden-sidst-h').focus();
    try { await invoke('siden_sidst_set'); } catch (e) { }
  }
  $('siden-skjul').addEventListener('click', () => R.gaaTil('overblik'));
  R.registrer('siden-sidst', { vis });
})();
