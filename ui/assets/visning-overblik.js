/* Huginn and Muninn — Overblik (mockup 3): domslinjen øverst og én tabelrække pr. zone (sorteret i Rust). Kolonner:
   Domæne · Status · Hvorfor · Afvist (i går · 7 dage · 30 dage) · Angreb (denne uge · sidste uge) · chevron. Hele rækken er ét mål: et klik hvor som helst, Enter eller mellemrum åbner domænets
   Angreb. Kun hvert tal over 0 (dyk på netop det tal, rammens lytter) og de små links i Hvorfor går andre steder hen. Hvilke domæner der er med, vælges under Indstillinger → Domæner; her står antallet og en diskret linje om dem, der ikke er valgt. Er intet valgt, står velkomsten (velkomst.js) i stedet for tabellen. Nye domæner på kontoen står i en linje over tabellen, til de er valgt eller afvist. "Hentet" står som tooltip på rækken. Status regnes i Rust (statusreglen er uændret); her vises den som chip med
   tekst og symbol (Handl/Hold øje/Roligt/Ukendt). "Hvorfor" bygges af årsagen (de største angrebstyper og hvad Cloudflare
   gjorde ved dem) og siger altid, om angrebene blev stoppet; regelnavne oversættes med ordlisten i sprog.js (hmSprog.klar),
   og hele navnet med CVE-nummer står i title. Typer med samme klarsprogstekst lægges sammen; egne regler står med kort navn;
   uden historik står "(første gang)" i stedet for "snit 0". Samme tekst bygges i Rust til notifikationerne (notify::aarsag).
   "Hvad gør jeg?" (domslinjen, eller det lille link i en Handl/Hold øje-række) åbner handlingsguiden i hjælpeskuffen:
   ét kort pr. domæne med råd fra handlingsbogen i sprog.js. Data kommer fra rammen (hmRamme.tilstand, besked
   'raekker'/'domaener'/'koe'/'siden'). */
(function () {
  'use strict';
  const S = window.hmSprog, T = S.T, R = window.hmRamme;
  const $ = id => document.getElementById(id);
  const { el, fmt } = R;
  const st = R.tilstand;
  let domSkjult = false;

  /* ── Hvorfor ── */
  const udenCve = navn => String(navn).replace(/ \(CVE-[^)]*\)$/, '');
  /* Teksten siger "i forgårs": statusdøgnet er rykket om natten, eller det står to lokale dage tilbage (notify::forgaars). */
  const forgaars = r => Boolean(r.opdateres || r.forgaars);
  /* Stoppet = blokeret, udfordret eller sendt i labyrinten. "Alle stoppet" kun, når det dækker hvert angreb. */
  function stoppet(r) {
    const f = r.angreb_fordeling || {}, a = r.angreb_i_gaar || 0, b = f.blokeret || 0;
    const alle = fmt(T.hvAlleStoppet, { a: S.tal(a) });
    if (b === a) return alle;
    const dele = [fmt(T.nBlokeret, { n: S.tal(b) }), fmt(T.nUdfordret, { n: S.tal(f.udfordret || 0) })];
    if (f.labyrint) dele.push(fmt(T.nLabyrint, { n: S.tal(f.labyrint) }));
    const andet = (f.andet || 0) + (f.faelder || 0);
    if (andet) dele.push(fmt(T.nAndet, { n: S.tal(andet) }));
    const n = b + (f.udfordret || 0) + (f.labyrint || 0);
    return `${n === a ? alle : fmt(T.hvNafM, { n: S.tal(n), a: S.tal(a) })} (${dele.join(', ')})`;
  }
  /* De største angrebstyper i klarsprog; samme tekst lægges sammen (fx to WordPress-regler), sorteret efter summen. */
  function flet(r) {
    const ud = [];
    for (const x of r.angreb_i_gaar_top || []) {
      const k = S.klar(udenCve(x.navn), x.egen), f = ud.find(y => y.k.tekst === k.tekst);
      const cve = (String(x.navn).match(/\((CVE-[0-9]{4}-[0-9]{4,7})\)$/) || [])[1];
      const m = f || { k, n: 0, cver: [], regler: [], tek: x.tek };
      if (f) f.n += x.n; else { m.n = x.n; ud.push(m); }
      if (cve && !m.cver.includes(cve)) m.cver.push(cve);
      for (const fuldt of x.regler || []) if (!m.regler.includes(fuldt)) m.regler.push(fuldt);
    }
    return ud.sort((a, b) => b.n - a.n);
  }
  /* Første sætning: "30 angreb i går (snit 12,4) – alle stoppet." (bruges også af guiden). */
  function hvorforStart(r) {
    const s = r.angreb_snit ? fmt(T.hvSnit, { s: S.tal(r.angreb_snit) }) : T.hvFoerste;
    return `${fmt(forgaars(r) ? T.hvAngrebNat : T.hvAngreb, { a: S.tal(r.angreb_i_gaar), s })} – ${stoppet(r)}.`;
  }
  /* "Din server svarer ikke" (notify::server_tekst) og en forstyrrelse samme dag, med mellemrum foran. */
  function server(r) {
    if (!r.server) return '';
    const f = r.forstyrrelse, land = f && window.hmTegn.land(f.land);
    const t = fmt(forgaars(r) ? T.hvServerNat : T.hvServer, { p: `${S.tal(r.server.procent)}${S.lang === 'da' ? ' %' : '%'}`, n: S.tal(r.server.svar) });
    return ` ${t}${f ? ' ' + (f.netvaerk ? fmt(T.hvForstyrrelseNet, { net: f.netvaerk, land }) : fmt(T.hvForstyrrelse, { land })) : ''}`;
  }
  /* Det, valgene under "Dit sites teknologi" gør (notify::tek_tekst), med mellemrum foran. */
  const tekNavn = S.tekNavn;
  function teknologi(r) {
    const ib = r.ikke_brugt || [], navne = ib.map(x => tekNavn(x.tek));
    /* Flere teknologier står i én sætning. */
    const ud = ib.length ? [fmt(T.hvIkkeBrugt, { n: S.tal(ib.reduce((s, x) => s + x.n, 0)), t: navne.length > 1 ? navne.slice(0, -1).join(', ') + T.hvOg + navne[navne.length - 1] : navne[0] })] : [];
    if (r.uventet) ud.push(fmt(T.hvUventet, { t: tekNavn(r.uventet.tek) }));
    if (r.naaet) ud.push(fmt(T.hvNaaet, { n: S.tal(r.naaet.n), t: tekNavn(r.naaet.tek) }));
    for (const b of r.tek_besked || []) ud.push(fmt(T.hvBesked, { t: tekNavn(b.tek), n: S.tal(b.n), s: S.tal(b.snit) }));
    return ud.map(t => ' ' + t).join('');
  }
  /* "Usædvanlig meget trafik" (notify::trafik_tekst): siger, hvad tallene kan bære. Et angreb er ikke udelukket. */
  const trafik = r => (r.trafik ? ' ' + fmt(T.hvTrafik, { n: S.tal(r.trafik.n), m: S.tal(Math.round(r.trafik.median)), stop: S.trafikStop(r.trafik.n, r.trafik.stoppet) }) : '');
  /* Lækkede kodeord (notify::laek_tekst): aldrig stien. */
  const laek = r => (r.laek ? ' ' + fmt(r.laek.naaet ? T.hvLaekNaaet : T.hvLaek, { n: S.tal(r.laek.n), k: S.tal(r.laek.naaet), d: forgaars(r) ? T.hvForgaarsOrd : T.hvIGaarOrd }) : '');
  /* En følsom fil, der svarede 200 og ligner filen (notify::sti200_tekst): højst 3 stier. */
  const STI200_NAEVN = 3;
  function sti200(r) {
    if (!r.sti200) return '';
    const st = r.sti200.stier, s = st.slice(0, STI200_NAEVN).join(', ') + (st.length > STI200_NAEVN ? ', …' : '');
    return ' ' + fmt(st.length > 1 ? T.hvSti200Flere : T.hvSti200, { s, d: forgaars(r) ? T.hvForgaarsOrd : T.hvIGaarOrd });
  }
  /* Cloudflares egne kritiske fund (notify::cf_fund_tekst). */
  const cfFund = r => (r.cf_fund ? ' ' + (r.cf_fund.n > 1 ? fmt(T.hvCfFundFlere, { n: S.tal(r.cf_fund.n) }) : T.hvCfFund) : '');
  /* Domænets tilstand hos Cloudflare (notify::tilstand_tekst) står først. */
  const tilstand = r => (r.tilstand || []).map(f => fmt(T.hvTilstand[f.slags] || T.hvTilstand.inaktiv, { s: f.status || '', k: f.kl || '' })).join(' ');
  const maerke = r => (r.moenster ? ' ' + S.moensterTekst(r.moenster) : '');
  /* DMARC (notify::dmarc_tekst). */
  const dmarc = r => (r.dmarc ? (r.dmarc.nye.length ? ' ' + fmt(T.hvDmarcNy, { l: r.dmarc.nye.join(', ') }) : '') + (r.dmarc.stiger ? ' ' + fmt(T.hvDmarcAndel, { p: S.tal(r.dmarc.procent), m: S.tal(r.dmarc.normalt) }) : '') : '');
  const tillaeg = r => teknologi(r) + laek(r) + sti200(r) + dmarc(r) + cfFund(r) + trafik(r) + server(r) + maerke(r);
  function hvorfor(r) {
    const foran = tilstand(r), resten = hvorforTal(r);
    return foran ? `${foran} ${resten}` : resten;
  }
  function hvorforTal(r) {
    if (r.status === 'graa') return T.hvIngenTal;
    if (r.gammel) return T.hvGammel;
    const a = r.angreb_i_gaar || 0;
    if (!a) return (forgaars(r) ? T.hvIngenNat : T.hvIngen) + tillaeg(r) + (r.opdateres ? ' ' + T.hvOpdateres : '');
    const typer = flet(r);
    let t = hvorforStart(r);
    /* Står før reglerne, så den ikke læses som en påstand om en bestemt regel (som notify::aarsag). */
    if (r.tendens) t += ' ' + (r.tendens === 'foelger' ? T.hvFoelger : forgaars(r) ? T.hvSaerligtDigNat : T.hvSaerligtDig);
    if (typer.length) {
      const dele = typer.map(x => `${x.k.tekst} (${S.tal(x.n)})`);
      const liste = dele.length > 1 ? dele.slice(0, -1).join(', ') + T.hvOg + dele[dele.length - 1] : dele[0];
      t += ' ' + fmt(T.hvMest, { liste });
      /* Intet råd om noget, sitet ikke kører. */
      const bruges = !(r.ikke_brugt || []).some(x => x.tek === typer[0].tek);
      if ((r.status === 'roed' || r.status === 'gul') && typer[0].k.raad && bruges) t += ' ' + typer[0].k.raad;
    }
    t += tillaeg(r);
    if (r.opdateres) t += ' ' + T.hvOpdateres;
    return t;
  }
  /* Forespørgslen til dyk for ét domæne. */
  const dykQ = (r, x) => new URLSearchParams({ zone: r.id, navn: r.navn, ...x }).toString();
  const angrebQ = r => 'vaernet?' + new URLSearchParams({ zone: r.id, navn: r.navn });
  const statusdag = r => ({ dag: r.status_dag, slags: 'angreb' });
  /* Teksten står i .hv-tekst, foldet til 3 linjer (foldHvorfor), og er ren tekst: et klik på den er et klik på rækken. Kun de små links går andre steder hen: "som du ikke bruger" og spørgsmålet åbner #teknologi, mærket trafik-dyk og "Hvad gør jeg?" guiden. */
  const tekMaal = r => ({ gaa: 'teknologi?' + new URLSearchParams({ zone: r.id, navn: r.navn }) });
  function hvorforCelle(r) {
    const td = document.createDocumentFragment(), tekst = hvorfor(r), boks = el('div', { className: 'hv-tekst' });
    const dele = (r.ikke_brugt || []).length ? tekst.split(T.hvIkkeBrugtOrd) : [tekst];
    dele.forEach((del, i) => {
      if (i) boks.append(window.hmTegn.knap(T.hvIkkeBrugtOrd, tekMaal(r), 'link hv-tek', fmt(T.hvIkkeBrugtAabn, { navn: r.navn })));
      boks.append(del);
    });
    boks.dataset.zone = r.id;
    td.append(boks);
    /* Mærket åbner dagens forespørgsler (trafik-dyk), ikke det, Cloudflare stoppede. */
    if (r.trafik) { const m = window.hmTegn.knap(T.trafikMaerke, dykQ(r, { dag: r.trafik.dag, trafik: '1' }), 'trafik-maerke', fmt(T.trafikAabn, { navn: r.navn, d: window.hmTegn.datoLang(r.trafik.dag), n: S.tal(r.trafik.n) })); m.dataset.fokus = 'trafik'; td.append(' ', m); }
    if (r.tek_spoerg && kraever(r) && r.angreb_status !== 'groen') { const m = window.hmTegn.knap(fmt(T.hvSpoerg, { t: tekNavn(r.tek_spoerg) }), tekMaal(r), 'link hv-spoerg'); m.dataset.fokus = 'tek'; td.append(' ', m); }
    if (kraever(r) || r.moenster || r.server || r.trafik || r.uventet || r.laek || r.sti200 || r.dmarc || r.cf_fund || (r.tilstand || []).length) {
      const m = el('button', { type: 'button', className: 'link hv-guide', textContent: T.gTitel });
      m.setAttribute('aria-label', `${T.gTitel} (${r.navn})`);
      m.dataset.fokus = 'guide';
      m.addEventListener('click', ev => { ev.stopPropagation(); guide(r.id); });
      td.append(' ', m);
    }
    const top = r.status === 'graa' || r.gammel ? [] : r.angreb_i_gaar_top || [];
    if (top.length) boks.title = top.map(x => `${S.tal(x.n)} ${x.navn}`).join(', ');
    return td;
  }
  /* Hvorfor står på højst 3 linjer, og resten foldes ind bag "mere". Først efter tegningen ved vi, om teksten fylder mere. Et åbent felt bliver åbent, når tabellen tegnes om. */
  const hvAabne = new Set();
  function foldHvorfor() {
    document.querySelectorAll('#overblik-raekker .hv-tekst').forEach(boks => {
      const zone = boks.dataset.zone, aaben = hvAabne.has(zone);
      boks.classList.toggle('aaben', aaben);
      if (!aaben && boks.scrollHeight <= boks.clientHeight + 1) return;
      const b = el('button', { type: 'button', className: 'link hv-mere', textContent: aaben ? T.hvMindre : T.hvMere });
      b.setAttribute('aria-expanded', String(aaben));
      b.dataset.fokus = 'hv-mere';
      b.addEventListener('click', ev => { ev.stopPropagation(); if (aaben) hvAabne.delete(zone); else hvAabne.add(zone); tegnTabel(); });
      boks.after(b);
    });
  }
  const chip = r => window.hmTegn.chip(r.status, T.status[r.status] || r.status);
  const kraever = r => !r.gammel && (r.status === 'roed' || r.status === 'gul');
  /* ── "Hvad gør jeg?": handlingsguiden ── */
  function guideKort(r) {
    const kort = el('section', { className: 'g-kort' });
    kort.dataset.zone = r.id;
    kort.append(el('h3', {}, el('span', { textContent: r.navn }), ' ', chip(r)));
    const a = r.angreb_i_gaar || 0;
    kort.append(el('p', { className: 'g-hvad' }, el('b', { textContent: T.gHvad }), ' ', a ? hvorforStart(r) + tillaeg(r) : hvorfor(r)));
    const afsnit = [...(r.tilstand || []).map(f => [T.gTilstandH[f.slags] || T.gTilstandH.inaktiv, T.gTilstandH[f.slags] ? f.slags : 'inaktiv']), ...(r.laek ? [[T.gLaekH, 'laek']] : []), ...(r.sti200 ? [[T.gSti200H, 'sti200']] : []), ...(r.dmarc ? [[T.gDmarcH, 'dmarc']] : []), ...(r.cf_fund ? [[T.gCfFundH, 'cf_fund']] : []), ...(r.moenster ? [{ maalrettet: [T.gMaalH, 'maalrettet'], dns_rekognoscering: [T.gDnsRekogH, 'dns_rekognoscering'] }[r.moenster.slags] || [T.gRekogH, 'rekognoscering']] : []), ...(r.trafik ? [[T.gTrafikH, 'trafik']] : []), ...(r.server ? [[T.gServerH, 'server']] : []), ...(r.trafik || r.server ? [[T.gDnsH, 'dns']] : [])];
    for (const [h, id] of afsnit) {
      const afsnit = el('div', { className: 'g-afsnit' }, el('h4', { textContent: h }), ...window.hmTegn.bogAfsnit(id));
      afsnit.dataset.kategori = id;
      kort.append(afsnit);
    }
    for (const x of flet(r)) {
      const afsnit = el('div', { className: 'g-afsnit' });
      afsnit.dataset.kategori = x.k.id;
      /* Ét fuldt navn: reglens dyk. Flere regler bag samme tekst kan ikke stå i ét filter: så dagens angreb. */
      const h = `${x.k.tekst} (${S.tal(x.n)})`;
      afsnit.append(el('h4', {}, window.hmTegn.knap(h, dykQ(r, x.regler.length === 1 ? { regel: x.regler[0] } : statusdag(r)), 'link dyk-celle')));
      for (const id of x.cver) {
        const b = el('button', { type: 'button', className: 'link g-cve', textContent: fmt(T.cveLaes, { id }), title: fmt(T.cveAabn, { id }) });
        b.setAttribute('aria-label', fmt(T.cveAabn, { id }));
        b.addEventListener('click', () => R.aabnCve(id));
        afsnit.append(b);
      }
      afsnit.append(...window.hmTegn.bogAfsnit(x.k.id));
      kort.append(afsnit);
    }
    /* Uden angreb er der intet at stoppe eller se: det er serveren, guiden handler om. */
    if (!a) return kort;
    const f = r.angreb_fordeling || {}, k = a - ((f.blokeret || 0) + (f.udfordret || 0) + (f.labyrint || 0));
    kort.append(el('p', { className: 'g-slut' + (k > 0 ? ' g-igennem' : '') }, k > 0 ? `${fmt(T.gKomIgennem, { k: S.tal(k) })} ${T.gKomIgennemTrin}` : fmt(T.gAlleStoppet, { n: S.tal(a) })));
    const se = el('button', { type: 'button', className: 'link g-se', textContent: (forgaars(r) ? T.gSeNat : T.gSe)(a) });
    se.addEventListener('click', () => { R.hjaelp(false); R.gaaTil('vaernet', new URLSearchParams({ zone: r.id, navn: r.navn, dag: r.status_dag, slags: 'angreb', vis: 'seneste' })); });
    kort.append(se);
    return kort;
  }
  /* zone: ét domæne (chip-klik); ellers dem, domslinjen nævner (Handl først, så Hold øje). Uden nogen: "Hvorfor er alt roligt?". */
  function guide(zone) {
    const rk = st.raekker, liste = zone ? rk.filter(r => r.id === zone) : [...rk.filter(r => kraever(r) && r.status === 'roed'), ...rk.filter(r => kraever(r) && r.status === 'gul')];
    const indhold = el('div', { className: 'guide', id: 'guide-indhold' });
    if (rk.some(r => r.kan_sammenligne === false)) indhold.append(el('p', { className: 'g-note', textContent: T.maalMin }));
    if (!liste.length) {
      const n = rk.reduce((x, r) => x + (r.i_gaar || 0), 0);
      indhold.append(el('p', { className: 'g-roligt', textContent: fmt(T.gRoligt1, { n: S.tal(n), d: T.domaener(rk.filter(r => r.status === 'groen').length) }) }), el('p', { textContent: T.gRoligt2 }));
      return R.hjaelp(true, { titel: 'gRoligtTitel', indhold });
    }
    indhold.append(el('p', { className: 'g-intro', textContent: T.gIntro }), ...liste.map(guideKort));
    R.hjaelp(true, { titel: 'gTitel', indhold });
  }

  /* ── domslinjen ── */
  function tegnDom() {
    const boks = $('dom'), rk = st.raekker;
    if (!rk.length || domSkjult) { boks.hidden = true; return; }
    const roede = rk.filter(r => r.status === 'roed' && !r.gammel), gule = rk.filter(r => r.status === 'gul' && !r.gammel);
    const groenne = rk.filter(r => r.status === 'groen'), graa = rk.filter(r => r.status === 'graa'), gamle = rk.filter(r => r.gammel);
    const nye = new Set((st.siden && st.siden.nye) || []);
    const p = $('dom-p');
    p.replaceChildren();
    const navn = r => { const s = window.hmTegn.knap(el('strong', { textContent: r.navn }), { gaa: angrebQ(r) }, 'link dom-navn', fmt(T.aabnFor, { navn: r.navn })); return nye.has(r.id) ? [s, ' ', el('span', { className: 'ny', textContent: T.ny })] : [s]; };
    let overskrift, klasse;
    if (roede.length) {
      overskrift = T.domRoed(roede.length); klasse = 'roed';
      roede.forEach((r, i) => p.append(...(i ? [' '] : []), ...navn(r), ' ', hvorfor(r)));
    } else if (gule.length) { overskrift = T.domGul(gule.length); klasse = 'gul'; }
    else if (groenne.length && !gamle.length) { overskrift = T.domIntet; klasse = 'groen'; }
    else { overskrift = T.domUkendt; klasse = ''; }
    if (gule.length) {
      if (p.childNodes.length) p.append(' ');
      gule.forEach((r, i) => p.append(...(i ? [', '] : []), ...navn(r)));
      p.append(T.domHoldOeje);
    }
    if (!roede.length && !gule.length && !gamle.length && groenne.length) p.append(fmt(T.domAltRoligt, { n: T.domaener(groenne.length) }));
    else if (groenne.length) p.append(' ' + fmt(T.domRoligt, { n: T.domaener(groenne.length) }));
    if (gamle.length) p.append((p.childNodes.length ? ' ' : '') + fmt(T.domGamle, { n: T.domaener(gamle.length) }));
    if (graa.length) p.append((p.childNodes.length ? ' ' : '') + (graa.length === rk.length ? T.domHentes : fmt(T.domGraa, { n: T.domaener(graa.length) })));
    $('dom-h').textContent = overskrift;
    const hj = $('dom-hjaelp');
    hj.dataset.t = roede.length || gule.length ? 'domHjaelp' : 'domHjaelpRoligt'; // data-t, så sprog.js' anvend() ikke skriver en anden tekst tilbage
    hj.textContent = T[hj.dataset.t];
    hj.hidden = !(roede.length || gule.length || groenne.length);
    boks.className = 'dom' + (klasse ? ' ' + klasse : '');
    boks.hidden = false;
  }

  /* "Din server svarer ikke" står på sin egen gule linje: hvert domæne åbner guiden for netop det. */
  function tegnServer() {
    const linje = $('server-linje'), rk = st.raekker.filter(r => r.server && !r.gammel);
    linje.hidden = !rk.length;
    linje.replaceChildren(...(rk.length ? [T.serverLinje, ' '] : []), ...rk.flatMap((r, i) => {
      const b = el('button', { type: 'button', className: 'link', textContent: r.navn });
      b.setAttribute('aria-label', fmt(T.serverAabn, { navn: r.navn }));
      b.addEventListener('click', () => guide(r.id));
      return [...(i ? [', '] : []), b];
    }));
  }

  /* ── huller i historikken: samme linje i Overblik og Siden sidst. Lukkes den, kommer den ikke igen for det hul. ── */
  function hulLinjer() {
    const V = window.hmTegn;
    return ((st.huller && st.huller.linjer) || []).map(h => {
      const v = { a: V.datoLang(h.fra), b: V.datoLang(h.til) }, luk = el('button', { type: 'button', className: 'link', textContent: T.hulLuk });
      luk.setAttribute('aria-label', fmt(T.hulLukLabel, v));
      luk.addEventListener('click', async () => {
        try { await R.invoke('hul_luk', { noegle: h.noegle }); } catch (e) { return; }
        st.huller.linjer = st.huller.linjer.filter(x => x !== h);
        R.udsend('huller');
      });
      return el('p', { className: 'hul-linje' }, h.tabt ? fmt(T.hulLinje, { ...v, n: S.tal(h.tabt) }) : fmt(T.hulLinjeIntet, v), ' ', luk);
    });
  }
  const tegnHuller = () => $('hul-linjer').replaceChildren(...hulLinjer());

  /* ── afviste API Tokens (V-KONTI.1) ── Er nogle af flere forbindelser afvist, står ét banner pr. afvist forbindelse (kittets banner) med navnet og
     "Skift API Token…", som åbner Indstillinger med formularen på forbindelsens række. Er alle afvist, står det som i dag (køens linje og fodlinjen).
     Tegnes kun, når de afviste eller navnene ændrer sig: banneret er role="alert" og må ikke læses op ved hver melding fra køen. */
  let afvistNoegle = null;
  function tegnAfvist() {
    const a = R.afvist(), ider = a.delvis ? a.ider : [];
    const noegle = ider.map((id, i) => id + '\t' + a.navne[i]).join('\n');
    if (noegle === afvistNoegle) return;
    afvistNoegle = noegle;
    $('afvist-linjer').replaceChildren(...ider.map((id, i) => {
      const b = window.Kit.banner({ variant: 'danger', tekst: fmt(T.afvistForb, { navn: a.navne[i] }), handling: { tekst: T.skift, aria: fmt(T.forbSkiftAria, { navn: a.navne[i] }), ved: () => window.hmIndst.skift(id) } });
      b.dataset.forbindelse = String(id);
      return b;
    }));
  }

  /* ── tabellen ── */
  /* "Angreb denne uge" farves kun ved en stor ændring: rød ved mindst det dobbelte, grøn ved højst det halve, og i begge
     tilfælde en forskel på mindst 20 angreb. Ellers neutral; procenten står i title. */
  function ugeKlasse(denne, sidste) {
    if (denne >= 2 * sidste && denne - sidste >= 20) return 'op';
    if (2 * denne <= sidste && sidste - denne >= 20) return 'ned';
    return '';
  }
  const aabn = r => R.gaaTil('vaernet', new URLSearchParams({ zone: r.id, navn: r.navn }));
  /* En talcelle: grå række "–", 0 som tekst (intet at åbne), ellers en knap til dyk. fokus: så fokus bliver på tallet, når tabellen tegnes om. daekning: [hentede døgn, døgn i perioden];
     uden hentede døgn er der intet tal, og en sum med huller siger det. */
  function talCelle(r, n, label, x, fokus, indhold, daekning) {
    const td = document.createDocumentFragment();
    if (r.status === 'graa' || (daekning && daekning[0] === 0)) td.append('–');
    else if (!n) td.append(S.tal(n));
    else {
      const b = window.hmTegn.knap(indhold || S.tal(n), dykQ(r, x), 'tal-knap', fmt(T.dykAabn, { navn: `${r.navn}, ${label}`, n: S.tal(n) }));
      b.dataset.fokus = fokus;
      td.append(b);
    }
    if (daekning && r.status !== 'graa' && daekning[0] && daekning[0] < daekning[1]) td.append(el('small', { className: 'dag-med', textContent: fmt(T.dagMed, { m: S.tal(daekning[0]), n: S.tal(daekning[1]) }) }));
    return td;
  }
  /* Tabellen er kittets (Kit.tabel): en række åbner domænets Angreb (raekkeHandling 'aabn': klik, Enter og mellemrum; et klik
     på et tal eller et link i Hvorfor åbner ikke rækken), to hovedrækker, domænet som rækkehoved, og rammen ruller vandret med
     domænet stående (Kit.tabelRul). Kittet bygger; her sættes kun de kendte id'er. */
  const talKol = (t, gruppe) => ({ t, gruppe, tal: true, klasse: 'tal-kol', bredde: 's' });
  const ovTabel = window.Kit.tabel({
    raekkeHandling: 'aabn', layout: 'auto', fastFoerste: true, titelT: 'tabelTitel', raekker: [],
    kolonnegrupper: [{ id: 'afvist', t: 'thAfvist', skille: true }, { id: 'angreb', t: 'thAngreb', skille: true }],
    kolonner: [{ t: 'thDomaene', hoved: true }, { t: 'thStatus', klasse: 'st', bredde: 'm' }, { t: 'thHvorfor', klasse: 'hv' }, talKol('thIGaar', 'afvist'), talKol('th7', 'afvist'), talKol('th30', 'afvist'), talKol('thUgen', 'angreb'), talKol('thSidsteUge', 'angreb'), { tekst: '', klasse: 'chev', bredde: 'xs' }],
    vedAabn: (tr, id) => { const r = st.raekker.find(x => x.id === id); if (r) aabn(r); },
  });
  {
    ovTabel.id = 'overblik-tabel';
    ovTabel.tBodies[0].id = 'overblik-raekker';
    const aabnHoved = el('span', { className: 'sr-only' });
    aabnHoved.dataset.t = 'thAabn';
    ovTabel.tHead.rows[0].lastElementChild.append(aabnHoved);
    const rul = window.Kit.tabelRul(ovTabel, { ariaT: 'tabelTitel' });
    rul.rod.id = 'overblik-rul';
    $('overblik-rul').replaceWith(rul.rod);
    S.anvend(rul.rod);
  }
  const dele = (...n) => { const f = document.createDocumentFragment(); f.append(...n); return f; };
  function tegnTabel() {
    const tb = $('overblik-raekker'), koe = st.koe;
    // Tegnes igen hvert 2. sekund, mens køen henter: kittet holder fokus på samme række; stod fokus på en knap i rækken, sættes det tilbage på den her.
    const aktiv = document.activeElement, fokusZone = aktiv && tb.contains(aktiv) ? aktiv.closest('tr').dataset.zone : null;
    const fokusKnap = fokusZone ? aktiv.dataset.fokus : null;
    ovTabel.saetRaekker(st.raekker.map(r => {
      const graa = r.status === 'graa';
      const henterNu = koe && koe.zone === r.id && koe.tilstand === 'henter';
      const navn = dele(window.hmTegn.domaene(r.navn), ...(henterNu ? [el('small', { className: 'raekke-henter', textContent: T.henterKort })] : []));
      let uge = talCelle(r, r.angreb_uge, T.ovUge, { fra: r.uge_fra, til: r.status_dag, slags: 'angreb' }, 'uge', el('span', { className: ugeKlasse(r.angreb_uge, r.angreb_ugen_foer), textContent: S.tal(r.angreb_uge) }));
      if (!graa && r.aendring != null) { const s = el('span', { className: 'uge-celle', title: fmt(T.ugeTitel, { p: `${r.aendring > 0 ? '+' : r.aendring < 0 ? '−' : '±'}${S.tal(Math.abs(r.aendring))} %` }) }); s.append(uge); uge = s; }
      return {
        data: r.id, attr: { 'data-zone': r.id }, titel: henterNu ? T.henterKort : r.sidst_hentet ? fmt(T.hentetTitel, { t: S.kl(r.sidst_hentet) }) : '',
        celler: [navn, dele(chip(r), ...(r.moenster ? [' ', window.hmTegn.moensterChip(r)] : [])), hvorforCelle(r),
          talCelle(r, r.i_gaar, T.ovIGaar, { dag: r.i_gaar_dag }, 'i-gaar'), talCelle(r, r.hele7, T.ov7, { periode: '7' }, 'h7', null, [r.med7, 7]), talCelle(r, r.hele30, T.ov30, {}, 'h30', null, [r.med30, 30]),
          uge, talCelle(r, r.angreb_ugen_foer, T.ovSidste, { fra: r.ugen_foer_fra, til: r.ugen_foer_til, slags: 'angreb' }, 'foer'),
          dele(el('span', { className: 'sr-only', textContent: fmt(T.aabnFor, { navn: r.navn }) }), pil())],
      };
    }));
    foldHvorfor();
    if (fokusZone) { const tr = [...tb.children].find(x => x.dataset.zone === fokusZone); if (tr) ((fokusKnap && tr.querySelector(`[data-fokus="${fokusKnap}"]`)) || tr).focus(); }
    visAntal();
    tegnDom();
    tegnServer();
    tegnHuller();
  }
  function pil() {
    const ns = 'http://www.w3.org/2000/svg', svg = document.createElementNS(ns, 'svg'), p = document.createElementNS(ns, 'path');
    svg.setAttribute('class', 'ik'); svg.setAttribute('viewBox', '0 0 24 24'); svg.setAttribute('aria-hidden', 'true');
    p.setAttribute('d', 'M9 6l6 6-6 6');
    svg.append(p);
    return svg;
  }
  function visAntal() {
    const n = st.raekker.length, fra = st.uvalgt.size, alleFra = !n && fra > 0;
    $('overblik-antal').textContent = n ? `· ${T.domaener(n)}` : '';
    R.baand.saetDeaktiveret('samlet', n === 0);
    $('samlet').hidden = alleFra;
    $('overblik-tom').hidden = !alleFra;
    $('overblik-rul').hidden = alleFra;
    $('overblik-note').hidden = alleFra;
    const note = $('fravalgt-note');
    note.hidden = !fra || alleFra;
    note.textContent = T.fravalgtNote(fra);
    note.setAttribute('aria-label', note.textContent);
    visNye();
  }
  /* Nye domæner på kontoen (T3): ét domæne kan vælges eller afvises her; flere ses under Indstillinger. */
  function visNye() {
    const linje = $('nye-domaener'), nye = st.domaener.filter(d => d.ny), V = window.hmTegn;
    linje.hidden = !nye.length;
    if (!nye.length) { linje.replaceChildren(); return; }
    const knap = (tekst, label, fn) => { const b = el('button', { type: 'button', className: 'link', textContent: tekst }); b.setAttribute('aria-label', label); b.addEventListener('click', () => fn().catch(e => R.status($('zone-status'), S.fejl(e), 'fejl'))); return b; };
    const ids = nye.map(d => d.id), ikkeNu = navn => knap(T.nyIkkeNu, fmt(T.nyIkkeNuFor, { navn }), () => R.afvisNye(ids));
    if (nye.length === 1) {
      const d = nye[0], valgte = st.domaener.filter(x => x.til).map(x => x.id);
      linje.replaceChildren(T.nyeLinje(1), ' ', V.domaene(d.navn, 'b'), ' — ', knap(T.nyVaelg, fmt(T.nyVaelgFor, { navn: d.navn }), () => R.gemValgte([...valgte, d.id])), ' · ', ikkeNu(d.navn));
    } else linje.replaceChildren(T.nyeLinje(nye.length), ' — ', V.knap(T.nyeSe, { gaa: 'indstillinger?afsnit=domaener' }, 'link', T.nyeSeLabel), ' · ', ikkeNu(T.domaener(nye.length)));
  }

  $('samlet').addEventListener('click', () => R.gaaTil('vaernet', 'samlet=1'));
  $('dom-luk').addEventListener('click', () => { domSkjult = true; tegnDom(); });
  $('dom-hjaelp').addEventListener('click', () => guide(null));

  R.paa('raekker', tegnTabel);
  R.paa('domaener', tegnTabel);
  R.paa('siden', tegnDom); // "ny" ved domæner, der er nye siden sidst
  R.paa('huller', tegnHuller);
  R.paa('koe', tegnAfvist);
  R.paa('forbindelser', tegnAfvist);
  R.hulLinjer = hulLinjer;
  R.hvorfor = hvorfor;
  R.guide = guide;
  R.ugeKlasse = ugeKlasse;
  R.registrer('overblik', { vis: tegnTabel });
})();
