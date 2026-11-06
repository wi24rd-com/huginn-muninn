// UI-røgtest: jsdom, stub for __TAURI__ (invoke + event.listen),
// ingen netværk. Programmet er én side (ui/index.html) med hash-navigation; hver blok indlæser siden med sin egen adresse.
// Brug: npm run test:ui (= node tests/ui/ui-roeg.mjs). Tidszonen er altid UTC. Notifikationspluginnets init-script læses fra
// Cargos register i den version, Cargo.lock binder (eller HM_NOTIF_IIFE). HM_REPO peger testen på en anden kopi af repoet
// (bruges til mutationstest). HM_SOV ganger alle ventetider (fx HM_SOV=4 på en meget belastet maskine).
process.env.TZ = 'UTC';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import http from 'node:http';
import { spawn, spawnSync } from 'node:child_process';
import { JSDOM } from 'jsdom';
const repo = process.env.HM_REPO || path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
if (!process.env.HM_NOTIF_IIFE) {
  const lock = fs.readFileSync(path.join(repo, 'src-tauri', 'Cargo.lock'), 'utf8');
  const ver = lock.match(/name = "tauri-plugin-notification"\nversion = "([^"]+)"/)[1];
  const reg = path.join(process.env.CARGO_HOME || path.join(os.homedir(), '.cargo'), 'registry', 'src');
  const kilde = fs.readdirSync(reg).map(d => path.join(reg, d, `tauri-plugin-notification-${ver}`, 'src', 'init-iife.js')).find(f => fs.existsSync(f));
  if (!kilde) throw new Error(`tauri-plugin-notification ${ver} findes ikke i ${reg} (kør cargo fetch, eller sæt HM_NOTIF_IIFE)`);
  process.env.HM_NOTIF_IIFE = kilde;
}
const ui = p => fs.readFileSync(path.join(repo, 'ui', p), 'utf8');
let ok = 0, fejl = 0;
const tjek = (navn, betingelse, detalje = '') => { if (betingelse) { ok++; console.log('ok   ' + navn); } else { fejl++; console.log('FEJL ' + navn + (detalje ? ' — ' + detalje : '')); } };
const sov = ms => new Promise(r => setTimeout(r, ms * (Number(process.env.HM_SOV) || 1)));
/* Fast ur: ingen test må afhænge af dagens dato. HM_NU sætter det (samme variabel som Rust-testene), og det går videre derfra, så ventetider stadig virker. Alle datoer i testene og i siderne kommer herfra. */
const NU_TEST = process.env.HM_NU || '2026-11-06T12:00:00Z', NU_MS = Date.parse(NU_TEST), START_MS = Date.now(); /* uret */
/* Med et ur i et andet år flytter testdataene med, så datoerne står i samme forhold til uret: de er skrevet med 2026, og et andet år skal vises med årstal. HM_KILDE: det er kopien, der kører. */
const UR_AAR = new Date(NU_MS).getUTCFullYear();
if (UR_AAR !== 2026 && !process.env.HM_KILDE) {
  const fil = fileURLToPath(import.meta.url);
  const r = spawnSync(process.execPath, ['--input-type=module', '-'], { input: fs.readFileSync(fil, 'utf8').split('\n').map(l => (l.includes('/* fast år */') ? l : l.replace(/\b2026\b/g, String(UR_AAR)))).join('\n'), cwd: path.dirname(fil), stdio: ['pipe', 'inherit', 'inherit'], env: { ...process.env, HM_REPO: repo, HM_KILDE: fil } });
  process.exit(r.status ?? 1);
}
let urSkift = 0;
/* Kl. 01 UTC sidste søndag i måneden: sommertidens skift i EU. Mandagen i en uge. Begge passer i alle år. */
const sidsteSoendag = (y, m) => { const d = new Date(Date.UTC(y, m + 1, 0, 1)); d.setUTCDate(d.getUTCDate() - d.getUTCDay()); return d.getTime(); };
const mandag = dag => { const t = new Date(`${dag}T00:00:00Z`); return new Date(t.getTime() - ((t.getUTCDay() + 6) % 7) * 864e5).toISOString().slice(0, 10); };
const urNu = () => NU_MS + urSkift + (Date.now() - START_MS); /* uret */
const fastUr = w => { const D = w.Date; w.Date = class extends D { constructor(...a) { if (a.length) super(...a); else super(urNu()); } static now() { return urNu(); } }; };
const UR_STUB = `<script>(()=>{const D=Date,n=${NU_MS},s=D.now();Date=class extends D{constructor(...a){if(a.length)super(...a);else super(n+D.now()-s)}static now(){return n+D.now()-s}}})();</script>`;

const URL0 = 'http://tauri.localhost/index.html';
/* Indlæser rammen med adressen (fx '#vaernet?samlet=1'). platform = det, Rust ellers sætter i init-scriptet. listen giver
   en rigtig afmeldingsfunktion, så dobbelte lyttere kan tælles. 'online'-lyttere på window tælles også. indlaes(w, scripts, koer) kan
   styre, hvornår scriptene kører (startrækkefølgen); ellers kører de alle med det samme. */
function side(hash, svar, sprog = 'da', platform = 'windows', foer = null, indlaes = null) {
  const html = ui('index.html');
  const scripts = [...html.matchAll(/<script src="([^"]+)"(?: defer)?><\/script>/g)].map(m => m[1]);
  const dom = new JSDOM(html.replace(/<script[^>]*><\/script>/g, ''), { url: URL0 + (hash || ''), runScripts: 'outside-only', pretendToBeVisual: true });
  const w = dom.window;
  fastUr(w);
  w.localStorage.setItem('hm-sprog', sprog);
  if (platform) w.__HM_PLATFORM__ = platform;
  if (foer) foer(w);
  // jsdom har ikke showModal/close.
  w.HTMLDialogElement.prototype.showModal = function () { this.setAttribute('open', ''); };
  w.HTMLDialogElement.prototype.close = function () { this.removeAttribute('open'); };
  const kald = [], lyttere = {}, online = { n: 0 };
  const add = w.addEventListener.bind(w), fjern = w.removeEventListener.bind(w);
  w.addEventListener = (t, f, o) => { if (t === 'online') online.n++; return add(t, f, o); };
  w.removeEventListener = (t, f, o) => { if (t === 'online') online.n--; return fjern(t, f, o); };
  w.__TAURI__ = {
    core: { invoke: async (cmd, args) => { kald.push([cmd, JSON.parse(JSON.stringify(args || {}))]); const f = svar[cmd]; if (f === undefined) throw { kode: 'intern' }; return typeof f === 'function' ? f(args) : JSON.parse(JSON.stringify(f)); } },
    event: { listen: async (navn, cb) => { (lyttere[navn] ||= []).push(cb); return () => { lyttere[navn] = lyttere[navn].filter(x => x !== cb); }; } },
    app: { getVersion: async () => '9.8.7' },
  };
  const koer = s => w.eval(ui(s));
  if (indlaes) indlaes(w, scripts, koer); else scripts.forEach(koer);
  const emit = (navn, payload) => (lyttere[navn] || []).forEach(cb => cb({ payload }));
  const tast = (key, x = {}) => w.document.dispatchEvent(new w.KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...x }));
  return { w, d: w.document, kald, emit, lyttere, online, tast };
}
const udenKommentarer = k => k.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '');
const synlige = d => [...d.querySelectorAll('section[data-visning]')].filter(s => !s.hidden).map(s => s.getAttribute('data-visning'));

// ── Data ──
const NU_ISO = '2026-11-03T08:01:00.000Z';
const raekke = (id, navn, status, x = {}) => Object.assign({ id, navn, status, gammel: false, i_gaar_dag: '2026-11-02', status_dag: '2026-11-02', uge_fra: '2026-10-27', ugen_foer_fra: '2026-10-20', ugen_foer_til: '2026-10-26', i_gaar: 0, i_dag_hidtil: 0, hele7: 0, hele30: 0, ugen_foer: 0, alle_hele7: 0, fordeling: { d30: {}, i_gaar: {} }, aendring: null, angreb_i_gaar: 0, angreb_snit: 0, sidst_hentet: null, doegn_faerdige: 0, doegn_i_alt: 31, opdateres: false, angreb_i_gaar_top: [], angreb_fordeling: { blokeret: 0, udfordret: 0, labyrint: 0, andet: 0 }, angreb_uge: 0, angreb_ugen_foer: 0 }, x);
const OVERBLIK = [
  raekke('z1', 'wi24rd.com', 'gul', { i_gaar: 1260, hele7: 3408, hele30: 8200, ugen_foer: 2400, aendring: 25, angreb_i_gaar: 30, angreb_snit: 12.4, sidst_hentet: NU_ISO, doegn_faerdige: 30,
    angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 20 }, { navn: 'Wordpress', n: 7 }, { navn: 'min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)', n: 2, egen: true }],
    angreb_fordeling: { blokeret: 30, udfordret: 0, labyrint: 0, andet: 0 }, angreb_uge: 45, angreb_ugen_foer: 36 }),
  raekke('z2', 'xyz.wi24rd.com', 'groen', { i_gaar: 104, hele7: 479, hele30: 2844, ugen_foer: 774, aendring: -33, angreb_i_gaar: 28, angreb_snit: 23.1, sidst_hentet: NU_ISO, doegn_faerdige: 30,
    angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 25 }, { navn: 'Browser Integrity Check', n: 3 }],
    angreb_fordeling: { blokeret: 20, udfordret: 8, labyrint: 0, andet: 0 }, angreb_uge: 140, angreb_ugen_foer: 210 }),
  raekke('z3', 'abe.wi24rd.com', 'graa'),
];
const dage = Array.from({ length: 30 }, (_, i) => ({ dag: new Date(Date.UTC(2026, 9, 5 + i)).toISOString().slice(0, 10), n: i, alle: 100 * i }));
/* Regelkortet med felterne ved siden af W's. */
const kortRegel = (r, x) => Object.assign({ art: 'angreb', h7: r.n7, h30: r.n30, faelder30: 0 }, r, x);
const vaernUd = x => Object.assign({
  genereret: '2026-11-03T06:54:00.000Z', perioder: { d1: 14, d7: 2782, d30: 8200 }, alle: { d7: 21665, d30: 90000 }, dage,
  regler: [{ navn: 'React - RCE - CVE:CVE-2025-55182', produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', n7: 20, n30: 80, foerst: '2026-10-05', sidst: '2026-11-02', stier: [{ sti: '/', n: 80 }], lande: [{ kode: 'US', n: 80 }] }],
  lande: [{ kode: 'US', n: 80 }], netvaerk: [{ navn: 'Google LLC', n: 80 }], stier: [{ sti: '/', n: 80 }], colos: [{ kode: 'EWR', n: 80 }], buer: [{ land: 'US', colo: 'EWR', n: 80 }],
  metoder: { POST: 80 }, kilder: { firewallManaged: 80 }, forsoeg: { i7: 5849, kategorier: [] },
  seneste: [{ t: '2026-11-03T05:00:00Z', regel: 'R', cve: null, land: 'US', sti: '/', colo: 'EWR', metode: 'GET', domaene: 'wi24rd.com' }, { t: '2026-11-03T04:00:00Z', regel: 'R', cve: null, land: 'DK', sti: '/a', colo: 'CPH', metode: 'GET', domaene: 'xyz.wi24rd.com' }],
  i_gaar: 1373, i_dag_hidtil: 58, hele7: 3816, hele30: 11006, ugen_foer: 3117, alle_hele7: 30513,
  fordeling: { d30: { blokeret: 676, udfordret: 4278, labyrint: 6124, andet: 0, faelder: 0 }, i_gaar: { blokeret: 30, udfordret: 134, labyrint: 1200, andet: 0, faelder: 0 } },
  h_dage: dage, h_lande: [{ kode: 'US', n: 80 }], h_netvaerk: [{ navn: 'Google LLC', n: 80 }], h_colos: [{ kode: 'EWR', n: 80 }], h_buer: [{ land: 'US', colo: 'EWR', n: 80 }], h_buer_i_alt: 1, h_idag: '2026-11-03',
  h_forsoeg: (x && x.forsoeg) || { i7: 5849, kategorier: [] },
}, x, { h_regler: (x && x.h_regler) || ((x && x.regler) || [{ navn: 'React - RCE - CVE:CVE-2025-55182', produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', n7: 20, n30: 80, foerst: '2026-10-05', sidst: '2026-11-02', stier: [{ sti: '/', n: 80 }], lande: [{ kode: 'US', n: 80 }] }]).map(r => kortRegel(r)) });
const KORT = { verden: JSON.parse(ui('assets/verden.json')), colo: JSON.parse(ui('assets/colo.json')), tidszoner: JSON.parse(ui('assets/tidszoner.json')) };
/* Angreb-visningen bor i rammen: rammen spørger selv om token og overblik. */
const MED_TOKEN = { token_status: { sat: true }, zones_cache: { zoner: [], hentet: null }, zones: [], overblik: OVERBLIK };
const celler = tr => [...tr.querySelectorAll('td')];
const hv = tr => tr.querySelector('.hv-tekst');

// ── Overblik (tidligere "Forsiden") ──
{
  let valg = ['z1', 'z2'];
  const s = side('', {
    token_status: { sat: true }, zones_cache: { zoner: OVERBLIK.map(r => ({ id: r.id, navn: r.navn })), hentet: NU_ISO },
    zones: OVERBLIK.map(r => ({ id: r.id, navn: r.navn })), overblik: OVERBLIK, valg_hent: () => valg,
    koe_status: null, koe_prioritet: null,
  });
  await sov(50);
  const { d, kald, emit, w } = s;
  const rows = [...d.querySelectorAll('#overblik-raekker tr')];
  tjek('overblik: tre rækker i Rusts rækkefølge', rows.length === 3 && rows[0].textContent.includes('wi24rd.com') && rows[2].textContent.includes('abe.wi24rd.com'));
  const st = rows[0].querySelector('.chip');
  tjek('status som tekst + klasse (Hold øje, chip-gul)', st && st.textContent === 'Hold øje' && st.classList.contains('chip-gul'), st && st.outerHTML);
  const u0 = hv(rows[0]), u1 = hv(rows[1]);
  tjek('P3 årsag: top 3 uden CVE og "alle stoppet"', u0.textContent === '30 angreb i går (snit 12,4) – alle stoppet. Mest et kendt hul i React (20), angreb på WordPress (7) og din egen regel »min-regel« (2). Bed din webmand tjekke, at siden er opdateret.', u0.textContent);
  tjek('P3 årsag: blandet → "20 blokeret, 8 udfordret"', u1.textContent === '28 angreb i går (snit 23,1) – alle stoppet (20 blokeret, 8 udfordret). Mest et kendt hul i React (25) og browsertjekket (3).', u1.textContent);
  tjek('P3 årsag: CVE-nummeret står i title', u0.closest('td').title === '20 React RCE (CVE-2025-55182), 7 Wordpress, 2 min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)', u0.closest('td').title);
  const c0 = celler(rows[0]).map(c => c.textContent);
  tjek('tal: i går, 7 og 30 dage med dansk tusindtalsskille', c0[2] === '1.260' && c0[3] === '3.408' && c0[4] === '8.200', c0.join(' | '));
  const uge0 = celler(rows[0])[5], uge1 = celler(rows[1])[5];
  tjek('P4 "Angreb denne uge": 45 (neutral) mod 36, +25 % i title (angreb, ikke alt afvist)', uge0.textContent === '45' && !uge0.querySelector('.op, .ned') && celler(rows[0])[6].textContent === '36' && uge0.title === '+25 % mod ugen før', uge0.outerHTML);
  tjek('P4 "Angreb denne uge": 140 (neutral) mod 210, −33 % i title', uge1.textContent === '140' && !uge1.querySelector('.op, .ned') && celler(rows[1])[6].textContent === '210' && uge1.title === '−33 % mod ugen før', uge1.outerHTML);
  const th = [...d.querySelectorAll('#overblik-tabel thead th')].map(x => x.textContent);
  tjek('Overblik-tabel: overskrift i to niveauer: Afvist over i går, 7 og 30 dage, Angreb over denne uge og sidste uge', th.slice(3, 5).join() === 'Afvist,Angreb' && th.slice(6).join() === 'i går,7 dage,30 dage,denne uge,sidste uge', th.join(','));
  tjek('grå række: streger og "Ingen tal endnu"', rows[2].querySelector('.chip').textContent === 'Ukendt' && hv(rows[2]).textContent === 'Ingen tal endnu – henter.' && celler(rows[2]).slice(2, 7).every(c => c.textContent === '–'));
  tjek('D4: ingen afkrydsning i tabellen', !d.querySelector('#overblik-tabel input') && !d.getElementById('valg-alle'));
  tjek('D4: titlen siger antal domæner, og knappen "Angreb for alle"', d.getElementById('overblik-antal').textContent === '· 3 domæner' && d.getElementById('samlet').textContent === 'Angreb for alle' && !d.getElementById('samlet').disabled, d.getElementById('overblik-antal').textContent);
  tjek('D4: intet slået fra: ingen linje om det', d.getElementById('fravalgt-note').hidden);
  tjek('koe_status hentes ved sideindlæsning', kald.some(k => k[0] === 'koe_status'));
  tjek('ingen køstatus endnu: pillen er skjult', d.getElementById('koe').hidden);
  emit('hentekoe', { zone: 'z3', navn: 'abe.wi24rd.com', doegn_faerdige: 87, doegn_i_alt: 229, zoner_faerdige: 4, zoner_i_alt: 12, zoner_fejl: 0, tilstand: 'henter' });
  const bar = d.getElementById('koe-fremskridt'), tekst = d.getElementById('koe-tekst');
  tjek('progress fra event: 87 af 229', !d.getElementById('koe').hidden && bar.value === 87 && bar.max === 229, `${bar.value}/${bar.max}`);
  tjek('statustekst fra event', tekst.textContent === 'Henter i baggrunden: 4 af 12 domæner · 87 af 229 dage · nu: abe.wi24rd.com', tekst.textContent);
  await sov(2100);
  tjek('"henter …" i rækken, der hentes nu (overblik hentet igen)', [...d.querySelectorAll('#overblik-raekker tr')][2].textContent.includes('henter …'));
  const antalOverblik = kald.filter(k => k[0] === 'overblik').length;
  for (let i = 0; i < 5; i++) emit('hentekoe', { zone: 'z3', navn: 'abe.wi24rd.com', doegn_faerdige: 88 + i, doegn_i_alt: 229, zoner_faerdige: 4, zoner_i_alt: 12, zoner_fejl: 0, tilstand: 'henter' });
  await sov(50);
  tjek('overblikket hentes højst hvert 2. sekund', kald.filter(k => k[0] === 'overblik').length - antalOverblik <= 1);
  emit('hentekoe', { zone: 'z1', navn: 'wi24rd.com', doegn_faerdige: 93, doegn_i_alt: 229, zoner_faerdige: 5, zoner_i_alt: 12, zoner_fejl: 0, tilstand: 'venter_kvote', fortsaetter_kl: '2026-11-03T09:21:00.000Z' });
  tjek('kvote: gul pille og linje "fortsætter kl. 09.21"', d.getElementById('koe').className === 'pille gul' && tekst.textContent === 'Cloudflare beder os vente (kvoten er brugt) – fortsætter kl. 09.21.' && d.getElementById('koe-besked').textContent === tekst.textContent, d.getElementById('koe').className + ' ' + tekst.textContent);
  emit('hentekoe', { zone: 'z1', navn: 'wi24rd.com', doegn_faerdige: 93, doegn_i_alt: 229, zoner_faerdige: 5, zoner_i_alt: 12, zoner_fejl: 0, tilstand: 'pause', fejl: { kode: 'forbindelse' }, fortsaetter_kl: '2026-11-03T09:00:15.000Z' });
  tjek('ingen forbindelse: gul "prøver igen kl. 09.00"', tekst.textContent === 'Ingen forbindelse til Cloudflare – prøver igen kl. 09.00.', tekst.textContent);
  emit('hentekoe', { zone: 'z1', navn: 'wi24rd.com', doegn_faerdige: 93, doegn_i_alt: 229, zoner_faerdige: 5, zoner_i_alt: 12, zoner_fejl: 0, tilstand: 'fejl', fejl: { kode: 'afvist', status: 403 } });
  tjek('afvist: rød pille med fejltekst', d.getElementById('koe').className === 'pille roed' && tekst.textContent.startsWith('Hentningen i baggrunden er stoppet: Cloudflare afviste API Tokenet'), tekst.textContent);
  emit('hentekoe', { zone: null, navn: null, doegn_faerdige: 229, doegn_i_alt: 229, zoner_faerdige: 13, zoner_i_alt: 12, zoner_fejl: 0, tilstand: 'faerdig' });
  tjek('færdig: grøn, fuld progress', d.getElementById('koe').className === 'pille ok' && bar.value === bar.max && tekst.textContent.startsWith('Alle domæner er hentet'), tekst.textContent);
  w.dispatchEvent(new w.Event('online')); await sov(10);
  tjek('online: koe_prioritet {zone: null}', kald.some(k => k[0] === 'koe_prioritet' && k[1].zone === null));
  tjek('ingen style= i Overblikkets tegnede markup', !d.getElementById('visning-overblik').outerHTML.includes('style='));
  tjek('zones (Cloudflare) kaldes efter zones_cache', kald.findIndex(k => k[0] === 'zones') > kald.findIndex(k => k[0] === 'zones_cache'));
  d.querySelector('#overblik-raekker tr').click(); await sov(30);
  tjek('rækken åbner zonens Angreb (#vaernet?zone=z1&navn=wi24rd.com)', w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com' && synlige(d).join() === 'vaernet', w.location.hash);
}

// ── Overblik på engelsk ──
{
  const s = side('', { token_status: { sat: true }, zones_cache: { zoner: [], hentet: null }, zones: [], overblik: OVERBLIK, valg_hent: ['z1'], koe_status: { zone: null, navn: null, doegn_faerdige: 1, doegn_i_alt: 2, zoner_faerdige: 1, zoner_i_alt: 3, zoner_fejl: 0, tilstand: 'henter' } }, 'en');
  await sov(50);
  const r0 = s.d.querySelector('#overblik-raekker tr');
  tjek('P3/P4 engelsk: årsag og "Attacks this week"', hv(r0).textContent === '30 attacks yesterday (average 12.4) – all stopped. Mostly a known hole in React (20), attacks on WordPress (7) and your own rule “min-regel” (2). Ask your web person to check that the site is up to date.'
    && [...s.d.querySelectorAll('#overblik-tabel thead th')].slice(3, 5).map(x => x.textContent).join() === 'Refused,Attacks' && [...s.d.querySelectorAll('#overblik-tabel thead th')][9].textContent === 'this week' && celler(r0)[5].textContent === '45', hv(r0).textContent);
  tjek('engelsk: "3 domains", status "Watch", koe_status vises straks', s.d.getElementById('overblik-antal').textContent === '· 3 domains' && s.d.querySelector('.chip').textContent === 'Watch' && s.d.getElementById('koe-tekst').textContent.startsWith('Fetching in the background: 1 of 3 domains'));
}

// ── P3/P5: kanttilfælde i "Hvorfor" ──
{
  const rk = [
    raekke('n1', 'nat.wi24rd.com', 'roed', { opdateres: true, angreb_i_gaar: 40, angreb_snit: 2, sidst_hentet: NU_ISO, angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 40 }], angreb_fordeling: { blokeret: 40, udfordret: 0, labyrint: 0, andet: 0 } }),
    raekke('n2', 'nul.wi24rd.com', 'groen', { sidst_hentet: NU_ISO, hele7: 5 }),
    raekke('n3', 'gammel.wi24rd.com', 'gul', { gammel: true, angreb_i_gaar: 99, sidst_hentet: NU_ISO, angreb_i_gaar_top: [{ navn: 'X', n: 99 }] }),
    raekke('n4', 'labyrint.wi24rd.com', 'gul', { angreb_i_gaar: 12, angreb_snit: 1, sidst_hentet: NU_ISO, angreb_i_gaar_top: [{ navn: 'Egen regel', n: 12 }], angreb_fordeling: { blokeret: 2, udfordret: 0, labyrint: 9, andet: 1 } }),
    raekke('n5', '<b>x</b>.wi24rd.com', 'groen', { sidst_hentet: NU_ISO, angreb_i_gaar: 1, angreb_i_gaar_top: [{ navn: '<img src=x>', n: 1 }], angreb_fordeling: { blokeret: 1, udfordret: 0, labyrint: 0, andet: 0 } }),
  ];
  const s = side('', { token_status: { sat: true }, zones_cache: { zoner: [], hentet: null }, zones: [], overblik: rk, valg_hent: [], koe_status: null });
  await sov(50);
  const sm = [...s.d.querySelectorAll('#overblik-raekker .hv-tekst')].map(x => x.textContent);
  tjek('P5 natten: "i forgårs … Opdateres …", Handl', sm[0] === '40 angreb i forgårs (snit 2) – alle stoppet. Mest et kendt hul i React (40). Bed din webmand tjekke, at siden er opdateret. Opdateres …' && s.d.querySelector('.chip').textContent === 'Handl', sm[0]);
  tjek('P3 nul angreb: ingen årsag, ingen "alle stoppet"', sm[1] === 'Ingen angreb i går.', sm[1]);
  tjek('P3 gamle tal: kun "Tallene er gamle" (ingen årsag, ingen title)', sm[2] === 'Tallene er gamle – henter nye.' && !s.d.querySelectorAll('#overblik-raekker td.hv')[2].title, sm[2]);
  tjek('P3 labyrint og andet nævnes, når de findes', sm[3] === '12 angreb i går (snit 1) – 11 af 12 stoppet (2 blokeret, 0 udfordret, 9 i labyrinten, 1 andet). Mest Egen regel (12).', sm[3]);
  tjek('P3 regelnavn og domæne som tekst (ingen HTML)', !s.d.querySelector('#overblik-raekker img, #overblik-raekker b') && sm[4].includes('<img src=x>'), sm[4]);
  tjek('P3/P4 ingen style= i overblikket', !s.d.getElementById('overblik-raekker').outerHTML.includes('style='));
}
{
  const s = side('', { token_status: { sat: true }, zones_cache: { zoner: [], hentet: null }, zones: [], overblik: [raekke('n1', 'nat.wi24rd.com', 'gul', { opdateres: true, angreb_i_gaar: 12, angreb_snit: 1, sidst_hentet: NU_ISO })], valg_hent: [], koe_status: null }, 'en');
  await sov(50);
  tjek('P5 engelsk: "the day before yesterday … Updating …"', s.d.querySelector('#overblik-raekker .hv-tekst').textContent === '12 attacks the day before yesterday (average 1) – 0 of 12 stopped (0 blocked, 0 challenged). Updating …', s.d.querySelector('#overblik-raekker .hv-tekst').textContent);
}

// ── Samlet Angreb (Værnet) ──
{
  const VALGTE = [{ id: 'z3', navn: 'abe.wi24rd.com', med_tal: false }, { id: 'z1', navn: 'wi24rd.com', med_tal: true }, { id: 'z2', navn: 'xyz.wi24rd.com', med_tal: true }];
  let cache = { domaener: 3, med_tal: 2, valgte: VALGTE, vaern: vaernUd() };
  const s = side('#vaernet?samlet=1', {
    ...MED_TOKEN, kortdata: KORT, valg_hent: ['z1', 'z2', 'z3'], vaern_samlet_cache: () => cache, koe_status: { tilstand: 'henter', zoner_i_alt: 12 }, koe_prioritet: null,
  });
  await sov(100);
  const { d, kald, emit, w } = s;
  tjek('samlet: valget hentes fra Rust og sendes til vaern_samlet_cache', JSON.stringify(kald.find(k => k[0] === 'vaern_samlet_cache')[1]) === '{"zoner":["z1","z2","z3"]}');
  tjek('samlet: under titlen "3 domæner"', d.getElementById('vaernet-under').textContent === '3 domæner', d.getElementById('vaernet-under').textContent);
  tjek('samlet: titel "Angreb · 3 domæner · …"', d.title.startsWith('Angreb · 3 domæner ·'), d.title);
  const valgteBoks = d.getElementById('valgte'), valgteA = [...valgteBoks.querySelectorAll('a')];
  tjek('P2 etiketterne står lige under titellinjen', !valgteBoks.hidden && valgteBoks.previousElementSibling === d.querySelector('#visning-vaernet .titelrad'));
  tjek('P2 tre etiketter i Rusts rækkefølge (efter navn)', valgteA.map(a => a.firstChild.textContent).join(',') === 'abe.wi24rd.com,wi24rd.com,xyz.wi24rd.com', valgteA.map(a => a.textContent).join(','));
  tjek('P2 klik åbner domænets eget Angreb', valgteA[1].getAttribute('href') === '#vaernet?zone=z1&navn=wi24rd.com', valgteA[1].getAttribute('href'));
  tjek('P2 domæne uden tal: stiplet klasse + skjult "(ingen tal endnu)"', valgteA[0].parentElement.className === 'hm-valgt-uden' && valgteA[0].querySelector('.sr-only').textContent === ' (ingen tal endnu)' && !valgteA[1].querySelector('.sr-only'));
  tjek('P2 "M af N har tal" som i dag', d.querySelector('.vg-advarsel.vg-gul').textContent.startsWith('2 af 3 domæner har tal endnu'));
  tjek('P2 liste med aria-label, ingen "+ flere" ved 3', valgteBoks.querySelector('ul').getAttribute('aria-label') === 'Dine domæner' && !valgteBoks.querySelector('details'));
  tjek('P2 ingen style= i etiketterne', !valgteBoks.outerHTML.includes('style='));
  tjek('samlet: flertal i lead og m1', d.querySelector('[data-t=lead]').textContent.startsWith('Hver dag banker robotter på dine 3 domæner') && d.querySelector('[data-t=m1]').textContent.includes('dine zoner (3 domæner)'));
  tjek('samlet: forsøg-teksten i flertal', d.getElementById('vg-forsoeg').textContent.includes('Her svarede dine 3 domæner 404'));
  tjek('samlet: "2 af 3 domæner har tal endnu" (gul)', d.querySelector('.vg-advarsel.vg-gul').textContent.startsWith('2 af 3 domæner har tal endnu'));
  tjek('samlet: "hentet" = ældste, og køen henter (gul prik)', d.querySelector('.vg-status').textContent.includes('(det ældste af de valgte) · henter i baggrunden') && d.getElementById('vagt').classList.contains('vg-henter'));
  const fliser = [...d.querySelectorAll('.vg-flise')].map(f => f.textContent);
  tjek('flise "Afvist i går" med "i dag hidtil"', fliser[0] === '1.373Afvist i gåri dag hidtil: 58', fliser[0]);
  tjek('fliser 7 og 30 hele dage', fliser[1].startsWith('3.816Afvist på 7 dage') && fliser[2] === '11.006Afvist på 30 dage', fliser.slice(1, 3).join(' | '));
  const ford = d.querySelector('.vg-fordeling').textContent;
  tjek('fordeling i går med labyrinten synlig', ford.includes('I går: heraf 30 blokeret · 134 udfordret · 1.200 lokket ind i labyrinten'), ford);
  tjek('fordeling 30 dage', ford.includes('30 dage: heraf 676 blokeret · 4.278 udfordret · 6.124 lokket ind i labyrinten'));
  const th = [...d.querySelectorAll('#vg-seneste th')].map(x => x.textContent);
  tjek('seneste: domænekolonne i samlet visning', th[1] === 'Domæne' && d.querySelector('#vg-seneste tbody td:nth-child(2)').textContent === 'wi24rd.com', th.join(','));
  tjek('ord: "Afvist" i tabellen og "De seneste, der blev afvist"', d.getElementById('vg-seneste-h').textContent === 'De seneste, der blev afvist' && d.querySelector('#vg-dage thead').textContent.includes('Afvist'));
  tjek('samlet: intet vaern_oversigt-kald (køens job)', !kald.some(k => k[0] === 'vaern_oversigt'));
  const foer = kald.filter(k => k[0] === 'vaern_samlet_cache').length;
  cache = { domaener: 3, med_tal: 3, valgte: VALGTE.map(v => ({ ...v, med_tal: true })), vaern: vaernUd({ i_gaar: 1400 }) };
  for (let i = 0; i < 4; i++) emit('hentekoe', { zone: 'z3', tilstand: 'henter' });
  emit('hentekoe', { zone: 'zandet', tilstand: 'henter' });
  await sov(100);
  tjek('hentekoe: ingen ny tegning før der er gået 3 s', kald.filter(k => k[0] === 'vaern_samlet_cache').length === foer);
  await sov(3100);
  tjek('hentekoe: tegnet igen én gang efter 3 s', kald.filter(k => k[0] === 'vaern_samlet_cache').length === foer + 1, String(kald.filter(k => k[0] === 'vaern_samlet_cache').length - foer));
  tjek('ny tegning: nye tal og ingen "M af N"', d.querySelector('.vg-flise b').textContent === '1.400' && !d.querySelector('.vg-advarsel'));
  tjek('P2 ny tegning: etiketterne opdateres (ingen stiplet nu, stadig tre)', d.querySelectorAll('#valgte a').length === 3 && !d.querySelector('#valgte .hm-valgt-uden'));
  emit('hentekoe', { zone: 'zandet', tilstand: 'henter' }); await sov(3200);
  tjek('hentekoe for en zone uden for valget: ingen ny tegning', kald.filter(k => k[0] === 'vaern_samlet_cache').length === foer + 1);
  w.dispatchEvent(new w.Event('online')); await sov(10);
  tjek('samlet online: koe_prioritet {zone: null}', kald.some(k => k[0] === 'koe_prioritet' && k[1].zone === null));
  const nyMarkup = d.querySelector('.vg-fliser').outerHTML + d.querySelector('.vg-fordeling').outerHTML + d.querySelector('#vg-seneste').outerHTML + d.querySelector('.vg-bjaelke').outerHTML;
  tjek('ingen style= i det nye markup (fliser, fordeling, seneste, statuslinje)', !nyMarkup.includes('style='));
}

// ── Samlet: ingen tal / intet valgt / engelsk ──
{
  const a = side('#vaernet?samlet=1', { ...MED_TOKEN, kortdata: KORT, valg_hent: ['z3'], vaern_samlet_cache: { domaener: 1, med_tal: 0, vaern: null }, koe_status: null });
  await sov(80);
  tjek('samlet uden tal: gul besked, "1 domæne"', a.d.querySelector('.vg-advarsel.vg-gul').textContent.startsWith('Ingen af dine domæner har tal endnu') && a.d.getElementById('vaernet-under').textContent === '1 domæne');
  const b = side('#vaernet?samlet=1', { ...MED_TOKEN, kortdata: KORT, valg_hent: [] });
  await sov(80);
  tjek('samlet uden domæner: velkomsten', b.d.querySelector('#vagt .velkomst-situation').textContent.startsWith('Du har ikke valgt nogen domæner endnu'));
  const c = side('#vaernet?samlet=1', { ...MED_TOKEN, kortdata: KORT, valg_hent: ['z1', 'z2', 'z3'], vaern_samlet_cache: { domaener: 3, med_tal: 3, vaern: vaernUd() }, koe_status: { tilstand: 'faerdig' } }, 'en');
  await sov(80);
  tjek('engelsk samlet: "3 domains", "Refused yesterday", "lured into the labyrinth"', c.d.getElementById('vaernet-under').textContent === '3 domains' && c.d.querySelector('.vg-flise').textContent.includes('Refused yesterday') && c.d.querySelector('.vg-fordeling').textContent.includes('1,200 lured into the labyrinth') && c.d.getElementById('vg-forsoeg').textContent.includes('Here your 3 domains answered 404'));
  tjek('engelsk samlet: køen færdig → ingen gul prik', !c.d.getElementById('vagt').classList.contains('vg-henter'));
}

// ── P2: mange domæner, "+ N flere", HTML i navne, engelsk ──
{
  const mange = Array.from({ length: 11 }, (_, i) => ({ id: 'm' + i, navn: i === 9 ? '<b>fed</b>.wi24rd.com' : `d${String(i).padStart(2, '0')}.wi24rd.com`, med_tal: true }));
  const s = side('#vaernet?samlet=1', { ...MED_TOKEN, kortdata: KORT, valg_hent: mange.map(v => v.id), vaern_samlet_cache: { domaener: 11, med_tal: 11, valgte: mange, vaern: vaernUd() }, koe_status: null });
  await sov(100);
  const b = s.d.getElementById('valgte'), [forste] = b.querySelectorAll(':scope > ul'), det = b.querySelector('details');
  tjek('P2 11 domæner: de første 8 synlige', forste.querySelectorAll('a').length === 8, String(forste.querySelectorAll('a').length));
  tjek('P2 "+ 3 flere" kan foldes ud med <details>', det && det.querySelector('summary').textContent === '+ 3 flere' && det.querySelectorAll('a').length === 3 && !det.open, det && det.outerHTML.slice(0, 120));
  tjek('P2 navne som tekst (ingen HTML fra Cloudflare)', !b.querySelector('b') && [...b.querySelectorAll('a')].some(a => a.textContent === '<b>fed</b>.wi24rd.com'));
  tjek('P2 titel "Angreb · 11 domæner"', s.d.title.startsWith('Angreb · 11 domæner'), s.d.title);
  const en = side('#vaernet?samlet=1', { ...MED_TOKEN, kortdata: KORT, valg_hent: mange.map(v => v.id), vaern_samlet_cache: { domaener: 11, med_tal: 0, valgte: mange.map(v => ({ ...v, med_tal: false })), vaern: null }, koe_status: null }, 'en');
  await sov(100);
  tjek('P2 engelsk og uden tal: etiketterne står alligevel, "+ 3 more"', en.d.querySelector('#valgte summary').textContent === '+ 3 more' && en.d.querySelectorAll('#valgte .hm-valgt-uden').length === 11 && en.d.querySelector('#valgte .sr-only').textContent === ' (no figures yet)' && en.d.title.startsWith('Attacks · 11 domains'), en.d.title);
}

// ── Én zone ──
{
  const enZone = vaernUd({ seneste: [{ t: '2026-11-03T05:00:00Z', regel: 'R', cve: null, land: 'US', sti: '/', colo: 'EWR', metode: 'GET' }] });
  const s = side('#vaernet?zone=z1&navn=wi24rd.com', { ...MED_TOKEN, kortdata: KORT, vaern_cache: enZone, vaern_oversigt: enZone, koe_prioritet: null });
  await sov(80);
  tjek('én zone: koe_prioritet {zone: z1} (zonen på skærmen først)', s.kald.some(k => k[0] === 'koe_prioritet' && k[1].zone === 'z1'));
  tjek('én zone: vaern_oversigt kaldes stadig', s.kald.some(k => k[0] === 'vaern_oversigt' && k[1].zoneId === 'z1'));
  tjek('én zone: ingen domænekolonne, under titlen står domænet', ![...s.d.querySelectorAll('#vg-seneste th')].some(x => x.textContent === 'Domæne') && s.d.getElementById('vaernet-under').textContent === 'wi24rd.com');
  tjek('P2 én zone: ingen etiketter', s.d.getElementById('valgte').hidden && !s.d.querySelector('#valgte a'));
  tjek('én zone: m1 i ental, m2 nævner to kald', s.d.querySelector('[data-t=m1]').textContent.includes('for zonen wi24rd.com') && s.d.querySelector('[data-t=m2]').textContent.includes('Højst to kald ad gangen'));
  tjek('én zone: domænet escapes (ingen HTML fra adressen)', !side('#vaernet?zone=z1&navn=%3Cimg%20src%3Dx%3E', { ...MED_TOKEN, kortdata: KORT, vaern_cache: enZone, vaern_oversigt: enZone, koe_prioritet: null }).d.querySelector('img[src=x]'));
}
// ── Indstillinger (start ved login, "Glem tokenet") og sproget til Rust ──
{
  let til = true;
  const s = side('#indstillinger', {
    token_status: { sat: true }, zones_cache: { zoner: [], hentet: null }, zones: [], overblik: OVERBLIK, valg_hent: [], koe_status: null,
    autostart_hent: () => til, autostart_saet: a => { til = a.til; return til; }, token_delete: null,
  });
  await sov(50);
  const { d, kald } = s;
  tjek('Opstart, Indstillinger er en visning i rammen; Om står dér, og hjælpeskuffen henviser til den', synlige(d).join() === 'indstillinger' && d.querySelector('#side a[href="#indstillinger"]').getAttribute('aria-current') === 'page' && d.querySelector('main [data-t=om2]') && !d.querySelector('#hjaelp [data-t=om2]') && d.querySelector('#hjaelp [data-t=hjOm]').textContent === 'Version, licenser og varemærke står under Indstillinger → Om.');
  tjek('Opstart, overskrift og kontakt på dansk', d.getElementById('indstillinger-h').textContent === 'Indstillinger' && d.querySelector('label[for=autostart] [data-t=autostart]').textContent === 'Start ved login');
  tjek('Opstart, "Glem nøglen…" er flyttet til Indstillinger', d.querySelector('#visning-indstillinger #glem') && !d.querySelector('#visning-overblik #glem') && !d.getElementById('glem').hidden);
  const boks = d.getElementById('autostart');
  tjek('Opstart, afkrydsningen læses fra OS (autostart_hent → ☑)', kald.some(k => k[0] === 'autostart_hent') && boks.checked);
  boks.click(); await sov(10);
  tjek('Opstart, klik → autostart_saet {til: false} og ☐', JSON.stringify(kald.filter(k => k[0] === 'autostart_saet').pop()) === '["autostart_saet",{"til":false}]' && !boks.checked && !boks.disabled, JSON.stringify(kald.filter(k => k[0] === 'autostart_saet')));
  tjek('Start ved login: fravalg giver det røde banner med knappen, aldrig en grøn besked', d.getElementById('indst-status').textContent === '' && !d.getElementById('autostart-banner').hidden && d.getElementById('autostart-banner').textContent.includes('Programmet starter ikke ved login.'), d.getElementById('indst-status').textContent);
  boks.click(); await sov(10);
  tjek('Opstart, klik igen → {til: true} og ☑', JSON.stringify(kald.filter(k => k[0] === 'autostart_saet').pop()) === '["autostart_saet",{"til":true}]' && boks.checked);
  tjek('Opstart, sprog_saet kaldes ved start med det afgjorte sprog', JSON.stringify(kald.find(k => k[0] === 'sprog_saet')) === '["sprog_saet",{"sprog":"da"}]' && kald[0][0] === 'sprog_saet', JSON.stringify(kald.slice(0, 2)));
  const v = d.getElementById('sprog'); v.value = 'en'; v.dispatchEvent(new s.w.Event('change')); await sov(10);
  tjek('Opstart, sprog_saet kaldes ved skift (en)', JSON.stringify(kald.filter(k => k[0] === 'sprog_saet').pop()) === '["sprog_saet",{"sprog":"en"}]' && kald.filter(k => k[0] === 'sprog_saet').length === 2);
  d.getElementById('glem').click(); d.getElementById('glem-ja').click(); await sov(20);
  tjek('Opstart, "Glem nøglen…" + Glem kalder token_delete', kald.some(k => k[0] === 'token_delete'));
  tjek('Opstart, ingen style= i Indstillinger', !d.getElementById('visning-indstillinger').outerHTML.includes('style='));
}
{
  const s = side('', {
    token_status: { sat: false }, autostart_hent: false, autostart_saet: () => { throw { kode: 'autostart' }; },
  });
  await sov(50);
  const { d } = s, boks = d.getElementById('autostart');
  const indstLink = d.querySelector('#side a[href="#indstillinger"]');
  tjek('Opstart, uden token: opsætningen vises, Indstillinger kan åbnes, "Glem nøglen…" skjult', synlige(d).join() === 'opsaetning' && !indstLink.hasAttribute('aria-disabled') && d.getElementById('glem').hidden);
  tjek('Opstart, OS siger fra: ☐', !boks.checked);
  boks.click(); await sov(10);
  tjek('Opstart, fejl vises som fejl, og afkrydsningen går tilbage', !boks.checked && d.getElementById('indst-status').className === 'r6-status fejl' && d.getElementById('indst-status').textContent === 'Start ved login kunne ikke læses eller ændres.', d.getElementById('indst-status').className + ' ' + d.getElementById('indst-status').textContent);
}
{
  const s = side('#indstillinger', { token_status: { sat: false }, autostart_hent: () => { throw { kode: 'autostart' }; } }, 'en');
  await sov(50);
  tjek('Opstart, engelsk: Settings, Start at login, fejl ved læsning', s.d.getElementById('indstillinger-h').textContent === 'Settings' && s.d.querySelector('label[for=autostart] [data-t=autostart]').textContent === 'Start at login' && s.d.getElementById('indst-status').textContent === 'Start at login could not be read or changed.', s.d.getElementById('indst-status').textContent);
  tjek('Opstart, engelsk: sprog_saet {sprog: en}', JSON.stringify(s.kald.find(k => k[0] === 'sprog_saet')) === '["sprog_saet",{"sprog":"en"}]');
  const a = side('', { token_status: { sat: false }, autostart_hent: true }, 'auto');
  await sov(50);
  tjek('Opstart, "Systemets sprog" sendes afgjort (jsdom: en-US → en), aldrig "auto"', JSON.stringify(a.kald.find(k => k[0] === 'sprog_saet')) === '["sprog_saet",{"sprog":"en"}]', JSON.stringify(a.kald.find(k => k[0] === 'sprog_saet')));
  const vg = side('#vaernet?zone=z1&navn=a.wi24rd.com', { kortdata: KORT, vaern_cache: null, koe_prioritet: null }, 'da');
  await sov(50);
  tjek('Opstart, Angreb-adressen (samme side) sender også sproget', JSON.stringify(vg.kald.find(k => k[0] === 'sprog_saet')) === '["sprog_saet",{"sprog":"da"}]');
}

// ── "Siden sidst" ──
const SIDEN_RK = [
  raekke('z2', 'xyz.wi24rd.com', 'roed', { angreb_i_gaar: 28, angreb_snit: 23.1, sidst_hentet: NU_ISO, angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 25 }, { navn: 'Browser Integrity Check', n: 3 }], angreb_fordeling: { blokeret: 20, udfordret: 8, labyrint: 0, andet: 0 } }),
  raekke('z1', 'wi24rd.com', 'gul', { angreb_i_gaar: 30, angreb_snit: 12.4, sidst_hentet: NU_ISO, angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 20 }], angreb_fordeling: { blokeret: 30, udfordret: 0, labyrint: 0, andet: 0 } }),
  raekke('g1', 'gammel.wi24rd.com', 'gul', { gammel: true, angreb_i_gaar: 99, sidst_hentet: NU_ISO }),
  raekke('n1', 'a.wi24rd.com', 'groen', { angreb_i_gaar: 3, sidst_hentet: NU_ISO, angreb_fordeling: { blokeret: 3, udfordret: 0, labyrint: 0, andet: 0 } }),
  raekke('n2', 'b.wi24rd.com', 'groen', { sidst_hentet: NU_ISO }),
  raekke('x1', 'abe.wi24rd.com', 'graa'),
];
const basis = x => Object.assign({ token_status: { sat: true }, zones_cache: { zoner: [], hentet: null }, zones: [], overblik: SIDEN_RK, valg_hent: [], koe_status: null, autostart_hent: true, notifikation_hent: { niveau: 'standard', roed: true, zoner: {} }, siden_sidst_set: null }, x);
{
  let vis = true;
  const s = side('', basis({ siden_sidst: () => ({ vis, vist: '2026-11-02T07:12:00.000Z', nye: ['z2'], raekker: SIDEN_RK }) }));
  await sov(60);
  const { d, kald, emit, w } = s;
  tjek('N siden: vis=true → rammen skifter til visningen Siden sidst', w.location.hash === '#siden-sidst' && synlige(d).join() === 'siden-sidst', w.location.hash + ' ' + synlige(d).join());
  tjek('N siden: vis=true → synlig, overskrift "Siden sidst" har fokus', !d.getElementById('visning-siden').hidden && d.getElementById('siden-sidst-h').textContent === 'Siden sidst' && d.activeElement === d.getElementById('siden-sidst-h'));
  tjek('N siden: "du kiggede her sidst …" fra vist', d.getElementById('siden-vist').textContent === 'Status for i går. Du kiggede her sidst ' + s.w.hmSprog.kl('2026-11-02T07:12:00.000Z') + '. Døgn i UTC, vist i din tid.', d.getElementById('siden-vist').textContent);
  const li = [...d.querySelectorAll('#siden-liste li')];
  tjek('N siden: Handl og Hold øje hver for sig (Handl først) med statuschip', li[0].className === 'hm-siden-roed' && li[0].querySelector('.chip-roed').textContent === 'Handl' && li[0].querySelector('b').textContent === 'xyz.wi24rd.com' && li[1].className === 'hm-siden-gul', li.map(x => x.className).join(','));
  tjek('N siden: linjen som i overblikket', li[0].querySelector('small').textContent === '28 angreb i går (snit 23,1) – alle stoppet (20 blokeret, 8 udfordret). Mest et kendt hul i React (25) og browsertjekket (3). Bed din webmand tjekke, at siden er opdateret.', li[0].querySelector('small').textContent);
  tjek('N siden: "ny" kun ved nye domæner', li[0].querySelector('.ny') && li[0].querySelector('.ny').textContent === 'ny' && !li[1].querySelector('.ny'));
  tjek('N siden: grønne samlet, gamle og grå hver i én linje', li.slice(2).map(x => x.textContent).join(' | ') === '2 domæner: intet usædvanligt | 1 domæne: tallene er gamle – Huginn henter | 1 domæne: ingen tal endnu', li.slice(2).map(x => x.textContent).join(' | '));
  tjek('N siden: markeres som set (siden_sidst_set) efter visning', kald.findIndex(k => k[0] === 'siden_sidst_set') > kald.findIndex(k => k[0] === 'siden_sidst'));
  d.getElementById('siden-skjul').click(); await sov(20);
  tjek('N siden: [Til overblikket] skifter til Overblik', d.getElementById('visning-siden').hidden && w.location.hash === '#overblik');
  const foer = kald.filter(k => k[0] === 'siden_sidst').length;
  emit('siden_sidst'); await sov(30);
  tjek('N siden: hændelsen siden_sidst (menu/notifikation) henter og viser visningen igen', kald.filter(k => k[0] === 'siden_sidst').length === foer + 1 && !d.getElementById('visning-siden').hidden);
  tjek('N siden: ingen style= og ingen HTML i visningen', !d.getElementById('visning-siden').outerHTML.includes('style='));
}
{
  const s = side('', basis({ siden_sidst: { vis: false, vist: null, nye: [], raekker: SIDEN_RK } }));
  await sov(60);
  tjek('N siden: vis=false → skjult, intet siden_sidst_set', s.d.getElementById('visning-siden').hidden && !s.kald.some(k => k[0] === 'siden_sidst_set'));
  const u = side('', basis({ token_status: { sat: false }, siden_sidst: { vis: true, nye: [], raekker: [] } }));
  await sov(60);
  tjek('N siden: uden token spørges der ikke, og visningen er skjult', u.d.getElementById('visning-siden').hidden && !u.kald.some(k => k[0] === 'siden_sidst'));
}
{
  const groen = [raekke('a', '<b>x</b>.wi24rd.com', 'groen', { angreb_i_gaar: 20, angreb_fordeling: { blokeret: 20 } }), raekke('b', 'b.wi24rd.com', 'groen', { angreb_i_gaar: 10, angreb_fordeling: { blokeret: 10 } })];
  const s = side('', basis({ siden_sidst: { vis: true, vist: null, nye: [], raekker: groen } }));
  await sov(60);
  tjek('N siden: alt grønt → "Alt er roligt: 2 domæner, 30 angreb i går, alle blokeret." og "første gang"', s.d.querySelector('#siden-liste li').textContent === 'Alt er roligt: 2 domæner, 30 angreb i går, alle blokeret.' && s.d.getElementById('siden-vist').textContent === 'Status for i går på alle dine domæner. Døgn i UTC, vist i din tid.', s.d.querySelector('#siden-liste li').textContent);
  const blandet = [raekke('a', 'a.wi24rd.com', 'groen', { angreb_i_gaar: 5, angreb_fordeling: { blokeret: 3, udfordret: 2 } })];
  const b = side('', basis({ siden_sidst: { vis: true, vist: null, nye: [], raekker: blandet } }));
  await sov(60);
  tjek('N siden: "alle blokeret" kun når alle er blokeret', b.d.querySelector('#siden-liste li').textContent === 'Alt er roligt: 1 domæne, 5 angreb i går.', b.d.querySelector('#siden-liste li').textContent);
  const h = side('', basis({ siden_sidst: { vis: true, vist: null, nye: ['a'], raekker: [raekke('a', '<img src=x>.wi24rd.com', 'roed', { angreb_i_gaar: 9, angreb_i_gaar_top: [{ navn: '<b>r</b>', n: 9 }] })] } }));
  await sov(60);
  tjek('N siden: domæne- og regelnavne som tekst (ingen HTML)', !h.d.querySelector('#siden-liste img, #siden-liste small b') && h.d.querySelector('#siden-liste b').textContent === '<img src=x>.wi24rd.com');
  const e = side('', basis({ siden_sidst: { vis: true, vist: null, nye: ['z2'], raekker: SIDEN_RK } }), 'en');
  await sov(60);
  const eli = [...e.d.querySelectorAll('#siden-liste li')];
  tjek('N siden engelsk: overskrift, "new", linjer', e.d.getElementById('siden-sidst-h').textContent === 'Since last time' && eli[0].querySelector('.ny').textContent === 'new' && eli[2].textContent === '2 domains: nothing unusual' && eli[3].textContent === '1 domain: the figures are old – Huginn is fetching', eli.map(x => x.textContent).join(' | '));
}

// ── Notifikationer under Indstillinger og i opsætningen ──
{
  let gemt = null;
  const s = side('#indstillinger', basis({
    overblik: OVERBLIK, siden_sidst: { vis: false, nye: [], raekker: [] },
    notifikation_hent: { niveau: 'alt', roed: false, zoner: { z2: 'fra' } },
    notifikation_saet: a => { gemt = a; return null; }, notifikation_zone_saet: null,
  }));
  await sov(60);
  const { d, kald } = s;
  const valgt = g => [...d.querySelectorAll(`input[name="${g}"]`)].filter(i => i.checked).map(i => i.value).join(',');
  const saetninger = 'Kun alvorligt, højst én gang om dagen|Ved alvorligt (anbefalet)|Også når et domæne skal holdes øje med';
  tjek('N indst: radioknapperne har hele sætninger, og opsætningen bruger de samme tre', [...d.querySelectorAll('#indst-niveau label')].map(b => b.textContent).join('|') === saetninger && [...d.querySelectorAll('#opsaet-niveau label')].map(b => b.textContent).join('|') === saetninger && d.getElementById('indst-niveau-h').textContent.startsWith('Hvornår vil du have besked?'));
  tjek('N indst: notifikation_hent → Alt valgt i begge grupper, "kræver handling" slået fra', valgt('indst-niveau') === 'alt' && valgt('opsaet-niveau') === 'alt' && !d.getElementById('notif-roed').checked, valgt('indst-niveau') + '/' + valgt('opsaet-niveau'));
  const sel = [...d.querySelectorAll('#pr-domaene-liste select')];
  const navne = [...d.querySelectorAll('#pr-domaene-liste tr > th[scope=row]')].map(x => x.textContent);
  tjek('N pr. domæne: én <select> pr. domæne efter navn, z2 = Aldrig', navne.join(',') === 'abe.wi24rd.com,wi24rd.com,xyz.wi24rd.com' && sel[2].value === 'fra' && sel[1].value === 'standard' && [...sel[0].options].map(o => o.textContent).join(',') === 'Følg standard,Alvorligt og hold øje,Kun alvorligt,Aldrig', navne.join(',') + ' ' + sel.map(x => x.value).join(','));
  tjek('N pr. domæne: aria-label med domænet', sel[1].getAttribute('aria-label') === 'Notifikationer for wi24rd.com');
  d.querySelector('#indst-niveau input[value=rolig]').click(); await sov(10);
  tjek('N indst: Rolig → notifikation_saet {niveau: rolig, roed: false}, opsætningen følger med', JSON.stringify(gemt) === '{"niveau":"rolig","roed":false}' && valgt('opsaet-niveau') === 'rolig' && d.getElementById('indst-status').textContent === 'Notifikationerne er gemt.', JSON.stringify(gemt));
  d.getElementById('notif-roed').click(); await sov(10);
  tjek('N indst: "Giv besked, når et domæne kræver handling" → {niveau: rolig, roed: true}', JSON.stringify(gemt) === '{"niveau":"rolig","roed":true}', JSON.stringify(gemt));
  sel[1].value = 'roed'; sel[1].dispatchEvent(new s.w.Event('change')); await sov(10);
  tjek('N pr. domæne: valg → notifikation_zone_saet {zone: z1, valg: roed}', JSON.stringify(kald.filter(k => k[0] === 'notifikation_zone_saet').pop()) === '["notifikation_zone_saet",{"zone":"z1","valg":"roed"}]', JSON.stringify(kald.filter(k => k[0] === 'notifikation_zone_saet')));
  const antal = d.querySelectorAll('#pr-domaene-liste select').length, foersteSel = d.querySelector('#pr-domaene-liste select');
  s.emit('hentekoe', { zone: null, tilstand: 'faerdig', zoner_i_alt: 3, zoner_faerdige: 3, doegn_i_alt: 1, doegn_faerdige: 1, zoner_fejl: 0 }); await sov(60);
  tjek('N pr. domæne: tegnes ikke om, når overblikket tegnes igen med samme zoner', d.querySelector('#pr-domaene-liste select') === foersteSel && d.querySelectorAll('#pr-domaene-liste select').length === antal);
  tjek('N indst: ingen style= i Indstillinger', !d.getElementById('visning-indstillinger').outerHTML.includes('style='));
}
{
  const s = side('#indstillinger', basis({
    overblik: OVERBLIK, siden_sidst: { vis: false, nye: [], raekker: [] },
    notifikation_saet: () => { throw { kode: 'lager' }; }, notifikation_zone_saet: () => { throw { kode: 'lager' }; },
  }));
  await sov(60);
  const { d } = s;
  d.querySelector('#indst-niveau input[value=alt]').click(); await sov(10);
  tjek('N indst: fejl → tilbage til Standard og fejltekst', d.querySelector('#indst-niveau input[value=standard]').checked && d.getElementById('indst-status').className === 'r6-status fejl' && d.getElementById('indst-status').textContent === 'Den lokale database kunne ikke bruges.', d.getElementById('indst-status').textContent);
  const v = d.querySelector('#pr-domaene-liste select'); v.value = 'fra'; v.dispatchEvent(new s.w.Event('change')); await sov(10);
  tjek('N pr. domæne: fejl → valget går tilbage', v.value === 'standard' && d.getElementById('indst-status').className === 'r6-status fejl');
}
{
  let gemt = null;
  const s = side('', { token_status: { sat: false }, autostart_hent: true, notifikation_saet: a => { gemt = a; return null; } });
  await sov(60);
  const { d } = s;
  tjek('N opsætning: forudindstillingen står under token-feltet, Standard forvalgt (også når notifikation_hent fejler)', d.querySelector('#visning-opsaetning #opsaet-niveau') && d.querySelector('#opsaet-niveau input[value=standard]').checked && d.getElementById('notif-roed').checked && d.getElementById('opsaet-niveau-p').textContent.startsWith('Hvornår vil du have besked?'));
  tjek('N opsætning: pr. domæne viser "står her, når zonelisten er hentet"', d.querySelector('#pr-domaene-liste td').textContent === 'Domænerne står her, når zonelisten er hentet.');
  d.querySelector('#opsaet-niveau input[value=alt]').click(); await sov(10);
  tjek('N opsætning: Alt → notifikation_saet {niveau: alt, roed: true}; Indstillinger følger med', JSON.stringify(gemt) === '{"niveau":"alt","roed":true}' && d.querySelector('#indst-niveau input[value=alt]').checked, JSON.stringify(gemt));
  const e = side('', { token_status: { sat: false }, autostart_hent: true }, 'en');
  await sov(60);
  tjek('N engelsk: Notifications · hele sætninger · Notify me when a domain needs action · Per domain', e.d.querySelector('[data-t=niveauH]').textContent === 'Notifications' && [...e.d.querySelectorAll('#indst-niveau label')].map(b => b.textContent).join('|') === 'Only when serious, at most once a day|When serious (recommended)|Also when a domain should be watched' && e.d.querySelector('label[for=notif-roed]').textContent === 'Notify me when a domain needs action' && e.d.querySelector('#pr-domaene-h').textContent === 'Per domain');
  tjek('N note: hvordan de slås fra i Systemindstillinger (da) / System Settings (en)', d.querySelector('[data-t=notifNote]').textContent.includes('Systemindstillinger → Notifikationer → Huginn and Muninn') && e.d.querySelector('[data-t=notifNote]').textContent.includes('System Settings → Notifications → Huginn and Muninn'));
}
{
  const s = side('#vaernet?zone=z1&navn=a.wi24rd.com', { ...MED_TOKEN, kortdata: KORT, vaern_cache: null, koe_prioritet: null, siden_sidst: { vis: false, nye: [], raekker: [] }, siden_sidst_set: null });
  await sov(50);
  s.emit('siden_sidst'); await sov(30);
  tjek('N Angreb: hændelsen siden_sidst skifter til Siden sidst uden at genindlæse (én lytter)', (s.lyttere.siden_sidst || []).length === 1 && s.w.location.hash === '#siden-sidst' && s.w.location.pathname === '/index.html' && synlige(s.d).join() === 'siden-sidst', s.w.location.href);
}

// ── New Notification('x') i webvisningen lykkes ikke ──
// Pluginnets eget init-script (tauri-plugin-notification 2.8.0, src/init-iife.js, som Tauri indsætter i siden) køres i
// jsdom med en model af Tauris ACL: et kald går kun igennem, hvis capabilities/default.json giver rettigheden
// (appens kommandoer: allow-<kommando>; pluginnets: notification:*).
const cap = JSON.parse(fs.readFileSync(path.join(repo, 'src-tauri', 'capabilities', 'default.json'), 'utf8'));
const tilladt = cmd => cmd.startsWith('plugin:notification|') ? cap.permissions.some(p => p.startsWith('notification:')) : cap.permissions.includes('allow-' + cmd.replace(/_/g, '-'));
{
  const iife = fs.readFileSync(process.env.HM_NOTIF_IIFE, 'utf8').replace('__TEMPLATE_windows__', 'false');
  tjek('N sikkerhed: ingen notification:* i capabilities', !JSON.stringify(cap).match(/"notification:/) && cap.permissions.every(p => !p.startsWith('notification')), cap.permissions.join(','));
  const dom = new JSDOM('<!doctype html><p>x</p>', { url: URL0, runScripts: 'outside-only' });
  const w = dom.window, forsoeg = [], lykkedes = [];
  w.__TAURI_INTERNALS__ = { invoke: async (cmd, args) => { forsoeg.push(cmd); if (!tilladt(cmd)) throw `${cmd} not allowed. Permissions associated with this command: notification:allow-notify`; lykkedes.push(cmd); return null; }, transformCallback: () => 0 };
  w.eval(iife);
  // Init-scriptet fanger ikke afvisningen: i webvisningen bliver den en fejl i konsollen, her en uhåndteret rejection.
  const afvist = [], fang = r => afvist.push(String(r));
  process.on('unhandledRejection', fang);
  try { new w.Notification('x'); } catch (e) { afvist.push('kastet: ' + e); }
  await sov(20);
  process.off('unhandledRejection', fang);
  tjek('N sikkerhed: init-scriptet erstatter window.Notification, og new Notification(\'x\') forsøger plugin:notification|notify', typeof w.Notification === 'function' && forsoeg.includes('plugin:notification|notify'), forsoeg.join(','));
  tjek('N sikkerhed: … men kaldet afvises (ingen notification:allow-notify), intet lykkes', !lykkedes.length && !tilladt('plugin:notification|notify') && afvist.some(x => x.startsWith('plugin:notification|notify not allowed')), lykkedes.join(',') + ' / ' + afvist.join(','));
  tjek('N sikkerhed: appens nye kommandoer er tilladt', ['notifikation_hent', 'notifikation_saet', 'notifikation_zone_saet', 'siden_sidst', 'siden_sidst_set'].every(tilladt));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════
// Ramme, navigation, genveje, hændelser, titel, CSP-model, capabilities, tokens og kontrast.
// ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════
const MED_SIDEN = { ...MED_TOKEN, hent_nu: { udfald: 'intet_at_hente', faerdige: 0, i_alt: 0 }, valg_hent: ['z1', 'z2'], koe_status: null, siden_sidst: { vis: false, nye: [], raekker: OVERBLIK }, siden_sidst_set: null, kortdata: KORT, vaern_samlet_cache: { domaener: 2, med_tal: 2, valgte: [], vaern: vaernUd() }, vaern_cache: vaernUd(), vaern_oversigt: vaernUd(), koe_prioritet: null, autostart_hent: true, hent_nu: null };

// ── Navigation: sidebjælken, titlen, hjælpeskuffens del ──
{
  const s = side('', MED_SIDEN);
  await sov(60);
  const { d, w } = s;
  tjek('S nav: start på Overblik (tom adresse), aria-current og titel', synlige(d).join() === 'overblik' && d.querySelector('#side a[href="#overblik"]').getAttribute('aria-current') === 'page' && d.title === 'Overblik · Wi24rd Huginn and Muninn', d.title);
  const forventet = [['siden-sidst', 'Siden sidst · Wi24rd Huginn and Muninn'], ['vaernet', 'Angreb · 2 domæner · Wi24rd Huginn and Muninn'], ['indstillinger', 'Indstillinger · Wi24rd Huginn and Muninn'], ['overblik', 'Overblik · Wi24rd Huginn and Muninn']];
  const ud = [];
  for (const [navn, titel] of forventet) {
    d.querySelector(`#side a[data-visning="${navn}"]`).click(); await sov(40);
    const aktuel = d.querySelector('#side a[aria-current="page"]');
    ud.push([navn, synlige(d).join(), aktuel && aktuel.getAttribute('data-visning'), d.title, [...d.querySelectorAll('#hjaelp .hjaelp-del')].filter(x => !x.hidden).map(x => x.getAttribute('data-hjaelp')).join()]);
    tjek(`S nav: klik på "${navn}" → kun den visning, aria-current, titel og hjælpens del`, synlige(d).join() === navn && aktuel && aktuel.getAttribute('data-visning') === navn && d.title === titel && ud.at(-1)[4] === navn, JSON.stringify(ud.at(-1)));
  }
  tjek('S nav: "Angreb" uden parametre åbner det samlede (#vaernet?samlet=1) første gang', ud[1][1] === 'vaernet' && s.kald.some(k => k[0] === 'vaern_samlet_cache'));
  tjek('S navne: sidebjælken bruger klarsprog (Angreb, ikke Værnet)', d.querySelector('#side a[data-visning="vaernet"]').textContent.startsWith('Angreb') && !d.getElementById('side').textContent.includes('Værn') && !d.querySelector('.band').textContent.includes('Værn'));
}
// ── Genveje (Windows: keydown; macOS: programmenuen) og hændelsen "visning" ──
{
  const s = side('', MED_SIDEN, 'da', 'windows');
  await sov(60);
  const { d, w, tast, kald, emit } = s;
  const spor = [];
  for (const [key, maal] of [['2', 'siden-sidst'], ['3', 'vaernet'], [',', 'indstillinger'], ['1', 'overblik']]) { tast(key, { ctrlKey: true }); await sov(40); spor.push(synlige(d).join()); }
  tjek('S genveje Windows: Ctrl+2/3/,/1 skifter visning', spor.join() === 'siden-sidst,vaernet,indstillinger,overblik', spor.join());
  tast('r', { ctrlKey: true }); await sov(20); tast('F5'); await sov(20);
  tjek('S genveje Windows: Ctrl+R og F5 = Hent nu (hent_nu), ikke genindlæsning', kald.filter(k => k[0] === 'hent_nu').length === 2);
  tjek('S genveje: sidebjælken og hjælpen viser "Ctrl+1" på Windows', d.querySelector('#side kbd[data-genvej="1"]').textContent === 'Ctrl+1' && d.querySelector('#genveje').textContent.includes('Ctrl+R'));
  emit('visning', 'indstillinger'); await sov(30);
  const a = synlige(d).join();
  emit('visning', 'vaernet'); await sov(30);
  emit('visning', 'noget-andet'); await sov(30);
  tjek('S hændelsen "visning" (programmenuen) skifter visning; ukendte navne ignoreres', a === 'indstillinger' && synlige(d).join() === 'vaernet' && w.location.hash.startsWith('#vaernet'), a + ' / ' + synlige(d).join());
  d.getElementById('hent-nu').click(); await sov(10);
  tjek('S "Hent nu" i båndet → hent_nu', kald.filter(k => k[0] === 'hent_nu').length === 3);
  const m = side('', MED_SIDEN, 'da', 'macos');
  await sov(60);
  m.tast('2', { ctrlKey: true }); m.tast('2', { metaKey: true }); m.tast('r', { metaKey: true }); await sov(30);
  tjek('S genveje macOS: keydown gør intet (programmenuen har ⌘1–3, ⌘, og ⌘R), sidebjælken viser ⌘1', synlige(m.d).join() === 'overblik' && !m.kald.some(k => k[0] === 'hent_nu') && m.d.querySelector('#side kbd[data-genvej="1"]').textContent === '⌘1' && m.d.documentElement.getAttribute('data-platform') === 'macos');
}
// ── Lyttere: én pr. hændelse, også efter 10 skift frem og tilbage ──
{
  const s = side('', MED_SIDEN);
  await sov(60);
  const { w, lyttere, online, d } = s, R = w.hmRamme;
  const onlineFoer = online.n, raekkerFoer = R.antalAbonnenter('raekker');
  for (let i = 0; i < 10; i++) {
    R.gaaTil('vaernet', i % 2 ? 'samlet=1' : 'zone=z1&navn=wi24rd.com'); await sov(30);
    R.gaaTil('overblik'); await sov(20);
  }
  tjek('S lyttere: én Tauri-lytter for hentekoe, siden_sidst og visning efter 10 skift', (lyttere.hentekoe || []).length === 1 && (lyttere.siden_sidst || []).length === 1 && (lyttere.visning || []).length === 1, JSON.stringify(Object.fromEntries(Object.entries(lyttere).map(([k, v]) => [k, v.length]))));
  tjek('S lyttere: Angreb afmelder alt ved skift (rammens beskeder og window "online")', R.antalAbonnenter('hentekoe') === 0 && R.antalAbonnenter('raekker') === raekkerFoer && online.n === onlineFoer, `hentekoe ${R.antalAbonnenter('hentekoe')}, raekker ${R.antalAbonnenter('raekker')}/${raekkerFoer}, online ${online.n}/${onlineFoer}`);
  R.gaaTil('vaernet', 'samlet=1'); await sov(60);
  const under = R.antalAbonnenter('hentekoe');
  R.gaaTil('overblik'); await sov(20);
  const n = s.kald.filter(k => k[0] === 'vaern_samlet_cache').length;
  for (let i = 0; i < 3; i++) s.emit('hentekoe', { zone: null, tilstand: 'henter' });
  await sov(3200);
  tjek('S lyttere: et skjult Angreb tegner ikke igen ved hentekoe', under === 1 && s.kald.filter(k => k[0] === 'vaern_samlet_cache').length === n && synlige(d).join() === 'overblik', `under=${under}`);
}
// ── Angreb: domænevælgeren og skift af domæne ──
{
  const s = side('#vaernet?zone=z1&navn=wi24rd.com', MED_SIDEN);
  await sov(80);
  const { d, w, kald } = s, v = d.getElementById('vg-zone');
  tjek('S vælger: "Samlet (3)" + domænerne efter navn, det viste er valgt', [...v.options].map(o => o.textContent).join('|') === 'Samlet (3)|abe.wi24rd.com|wi24rd.com|xyz.wi24rd.com' && v.value === 'z1', [...v.options].map(o => o.textContent).join('|'));
  v.value = 'z2'; v.dispatchEvent(new w.Event('change')); await sov(60);
  tjek('S vælger: skift → #vaernet?zone=z2&navn=xyz.wi24rd.com og nye tal for z2', w.location.hash === '#vaernet?zone=z2&navn=xyz.wi24rd.com' && kald.some(k => k[0] === 'vaern_oversigt' && k[1].zoneId === 'z2') && kald.some(k => k[0] === 'koe_prioritet' && k[1].zone === 'z2') && d.title.startsWith('Angreb · xyz.wi24rd.com'), w.location.hash + ' ' + d.title);
  v.value = 'samlet'; v.dispatchEvent(new w.Event('change')); await sov(60);
  tjek('S vælger: "Samlet" → #vaernet?samlet=1', w.location.hash === '#vaernet?samlet=1' && d.getElementById('vaernet-under').textContent === '2 domæner');
}
// ── Overblik: domslinjen, rækker med tastatur, hjælpeskuffen ──
{
  const s = side('', { ...MED_SIDEN, overblik: SIDEN_RK, siden_sidst: { vis: false, nye: ['z2'], raekker: SIDEN_RK } });
  await sov(60);
  const { d, w, tast } = s;
  tjek('S domslinje: "1 domæne kræver handling" med domæne, ny, Hvorfor, hold øje, gamle, roligt og ukendt', d.getElementById('dom-h').textContent === '1 domæne kræver handling' && d.getElementById('dom').className === 'dom roed'
    && d.getElementById('dom-p').textContent === 'xyz.wi24rd.com ny 28 angreb i går (snit 23,1) – alle stoppet (20 blokeret, 8 udfordret). Mest et kendt hul i React (25) og browsertjekket (3). Bed din webmand tjekke, at siden er opdateret. wi24rd.com: hold øje. 2 domæner: alt roligt. 1 domæne: tallene er gamle. 1 domæne: ingen tal endnu.', d.getElementById('dom-p').textContent);
  const to = side('', { ...MED_SIDEN, overblik: [raekke('a', 'a.wi24rd.com', 'roed'), raekke('b', 'b.wi24rd.com', 'roed'), raekke('c', 'c.wi24rd.com', 'roed', { gammel: true }), raekke('d', 'd.wi24rd.com', 'gul')] });
  await sov(60);
  tjek('S sidebjælken: tallet ved Siden sidst = domæner, der kræver handling (ikke gamle, ikke Hold øje)', d.getElementById('siden-tal').textContent === '1' && d.getElementById('siden-tal').title === '1 domæne kræver handling' && !d.getElementById('siden-tal').hidden
    && to.d.getElementById('siden-tal').textContent === '2' && to.d.getElementById('siden-tal').title === '2 domæner kræver handling', to.d.getElementById('siden-tal').textContent);
  d.getElementById('dom-luk').click();
  tjek('S domslinje: × skjuler den', d.getElementById('dom').hidden);
  const tr = d.querySelector('#overblik-raekker tr');
  tr.focus(); tr.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })); await sov(40);
  const efterEnter = w.location.hash;
  w.hmRamme.gaaTil('overblik'); await sov(30);
  const tr2 = d.querySelectorAll('#overblik-raekker tr')[1];
  tr2.focus(); const mellemrum = new w.KeyboardEvent('keydown', { key: ' ', bubbles: true, cancelable: true }); tr2.dispatchEvent(mellemrum); await sov(40);
  tjek('S rækker: Enter og mellemrum åbner rækkens Angreb (mellemrum ruller ikke)', efterEnter === '#vaernet?zone=z2&navn=xyz.wi24rd.com' && w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com' && mellemrum.defaultPrevented && tr.tabIndex === 0, efterEnter + ' ' + w.location.hash);
  w.hmRamme.gaaTil('overblik'); await sov(30);
  tjek('S rækker: "Hentet" står som tooltip, chevron har en skjult tekst', d.querySelector('#overblik-raekker tr').title === 'Hentet ' + w.hmSprog.kl(NU_ISO) && d.querySelector('#overblik-raekker td.chev .sr-only').textContent === 'Åbn Angreb for xyz.wi24rd.com');
  d.getElementById('hjaelp-knap').click();
  const aaben = !d.getElementById('hjaelp').hidden && d.getElementById('hjaelp-knap').getAttribute('aria-expanded') === 'true' && [...d.querySelectorAll('#hjaelp .hjaelp-del')].filter(x => !x.hidden).map(x => x.getAttribute('data-hjaelp')).join() === 'overblik';
  tast('Escape');
  tjek('S hjælp: "?" åbner skuffen med Overblikkets forklaring, Esc lukker', aaben && d.getElementById('hjaelp').hidden && d.getElementById('hjaelp-knap').getAttribute('aria-expanded') === 'false');
  const alle = side('', { ...MED_SIDEN, overblik: [raekke('a', 'a.wi24rd.com', 'groen', { angreb_i_gaar: 2, angreb_fordeling: { blokeret: 2 } }), raekke('b', 'b.wi24rd.com', 'groen')] });
  await sov(60);
  tjek('S domslinje: alt roligt → "Intet at gøre" og ingen tal ved Siden sidst', alle.d.getElementById('dom-h').textContent === 'Intet at gøre' && alle.d.getElementById('dom-p').textContent === 'Alt er roligt på 2 domæner i går.' && alle.d.getElementById('siden-tal').hidden, alle.d.getElementById('dom-p').textContent);
}
// ── Pillen i båndet ──
{
  const s = side('', MED_SIDEN);
  await sov(60);
  s.emit('hentekoe', { zone: 'z1', navn: 'wi24rd.com', doegn_faerdige: 5, doegn_i_alt: 90, zoner_faerdige: 1, zoner_i_alt: 3, zoner_fejl: 0, tilstand: 'henter' });
  const henter = s.d.getElementById('koe-kort').textContent, klasse = s.d.getElementById('koe').className, besked = s.d.getElementById('koe-besked').textContent;
  s.emit('hentekoe', { zone: null, navn: null, doegn_faerdige: 90, doegn_i_alt: 90, zoner_faerdige: 3, zoner_i_alt: 3, zoner_fejl: 0, tilstand: 'faerdig' });
  tjek('S pille: "Henter 1 af 3" med spinner, "Opdateret 08.01" bagefter; ingen linje i indholdet, mens alt går godt', henter === 'Henter 1 af 3' && klasse === 'pille henter' && besked === '' && s.d.getElementById('koe-kort').textContent === 'Opdateret 08.01' && s.d.getElementById('koe-besked').textContent === '', `${henter} / ${klasse} / ${s.d.getElementById('koe-kort').textContent}`);
}
// ── Uden token: visningerne er slået fra ──
{
  const s = side('#vaernet?samlet=1', { token_status: { sat: false }, autostart_hent: true });
  await sov(60);
  const { d, w, tast } = s;
  const fra = ['overblik', 'siden-sidst', 'vaernet'].every(v => d.querySelector(`#side a[data-visning="${v}"]`).getAttribute('aria-disabled') === 'true');
  d.querySelector('#side a[data-visning="overblik"]').click(); tast('1', { ctrlKey: true }); await sov(30);
  tjek('S uden token: Overblik/Siden sidst/Angreb er slået fra, opsætningen står (titel "Forbind med Cloudflare")', fra && synlige(d).join() === 'opsaetning' && d.title === 'Forbind med Cloudflare · Wi24rd Huginn and Muninn' && !s.kald.some(k => k[0] === 'vaern_samlet_cache'), synlige(d).join() + ' ' + d.title);
  tast(',', { ctrlKey: true }); await sov(30);
  tjek('S uden token: Ctrl+, åbner stadig Indstillinger, og "Hent nu" er skjult', synlige(d).join() === 'indstillinger' && w.location.hash === '#indstillinger' && d.getElementById('hent-nu').hidden);
}
// ── Sprog: "dage" og "Afvist", ordlisten ──
{
  const s = side('#vaernet?samlet=1', { ...MED_SIDEN, valg_hent: ['z1'] });
  await sov(80);
  const strenge = [];
  const saml = o => { for (const v of Object.values(o)) { if (typeof v === 'string') strenge.push(v); else if (v && typeof v === 'object') saml(v); } };
  saml(s.w.hmSprog.T);
  const synligTekst = s.d.body.textContent;
  tjek('S sprog: ingen "døgn" (undtagen datovælgerens "Hele døgn i UTC" og "hele døgn") og ingen "Stoppet" (undtagen "Stoppet ved Edge") i de danske tekster eller i Angreb-visningen', !strenge.some(x => /(?<!Hele |hele )døgn(?! i UTC)|Stoppet(?! ved Edge)/.test(x)) && !/døgn|Stoppet/.test(synligTekst), strenge.filter(x => /døgn|Stoppet(?! ved Edge)/.test(x)).join(' | '));
  const K = s.w.hmSprog.klar;
  tjek('S ordliste: "React RCE" → "et kendt hul i React" (med råd), ukendte navne står som de er', K('React RCE').tekst === 'et kendt hul i React' && K('React RCE').raad.startsWith('Bed din webmand') && K('Egen regel').tekst === 'Egen regel' && K('Egen regel').raad === '' && K('SQLi - Comment').tekst === 'forsøg på at snige databasekommandoer ind');
  const e = side('', MED_SIDEN, 'en');
  await sov(40);
  tjek('S ordliste engelsk og statusord: Act/Watch/Calm/Unknown', e.w.hmSprog.klar('React RCE').tekst === 'a known hole in React' && JSON.stringify(e.w.hmSprog.T.status) === '{"roed":"Act","gul":"Watch","groen":"Calm","graa":"Unknown"}');
}
// ── Statisk HTML, CSS og CSP-model ──
{
  const html = ui('index.html');
  const scripts = [...html.matchAll(/<script\b[^>]*>/g)].map(m => m[0]);
  tjek('S CSP: ingen inline-scripts (hvert <script> har src og defer) og ingen style= i index.html', scripts.length === 15 && scripts.every(t => / src="assets\/[a-z-]+\.js"/.test(t) && (/ defer/.test(t) || t.includes('tema.js'))) && !/<script\b[^>]*>[^<]+<\/script>/.test(html) && !/\sstyle=/.test(html) && !/\son[a-z]+=/.test(html), scripts.join(' '));
  const nye = ['tema.js', 'ramme.js', 'vg-tegn.js', 'velkomst.js', 'visning-dyk.js', 'visning-overblik.js', 'visning-siden.js', 'visning-indstillinger.js', 'sprog.js', 'genforsoeg.js'].map(f => [f, udenKommentarer(ui('assets/' + f))]);
  tjek('S nyt UI: ingen innerHTML og ingen style-attributter i ramme og visninger', nye.every(([, k]) => !/innerHTML|setAttribute\(\s*'style'|\.style\./.test(k)), nye.filter(([, k]) => /innerHTML|\.style\./.test(k)).map(([f]) => f).join());
  const alleUi = ['index.html', ...fs.readdirSync(path.join(repo, 'ui', 'assets')).filter(f => f.endsWith('.js')).map(f => 'assets/' + f)].map(f => [f, udenKommentarer(ui(f))]);
  const inline = /\bstyle\s*=|setAttribute\(\s*['"`]style|\.style\b|cssText|<style/;
  tjek(`S CSP: ingen inline-style i ${alleUi.length} filer i ui/`, alleUi.length >= 15 && alleUi.every(([, k]) => !inline.test(k)), alleUi.filter(([, k]) => inline.test(k)).map(([f]) => f).join());
  const css = ['app.css', 'vagt.css'].map(f => [f, ui('assets/' + f)]);
  tjek('S CSS: kun CSS-variabler, ingen farvekoder uden for tokens.css', css.every(([, k]) => !/#[0-9a-f]{3,8}\b|rgba?\(|hsla?\(/i.test(k.replace(/\/\*[\s\S]*?\*\//g, ''))), css.map(([f, k]) => f + ':' + (k.replace(/\/\*[\s\S]*?\*\//g, '').match(/#[0-9a-f]{3,8}\b|rgba?\([^)]*\)/gi) || []).slice(0, 3).join(',')).join(' '));
  const alleCss = css.map(([, k]) => udenKommentarer(k)).join('\n');
  const rester = ['.side-hoved', '.etiket', '.sek ', '.sek-in', '.hm-top', '.hm-logo', '.hm-sprog', '.hm-tilbage', '.hm-ravne-baggrund', '.hm-om', '.vg-eksempel', '.r12-', '.hero', '.gutter', '.praemis'];
  tjek('S CSS: resterne fra Q1 er væk (site.css, maskinrum.css, .side-hoved*, .etiket, .sek, .hm-top …, .vg-eksempel)', !fs.existsSync(path.join(repo, 'ui', 'assets', 'site.css')) && !fs.existsSync(path.join(repo, 'ui', 'assets', 'maskinrum.css')) && !fs.existsSync(path.join(repo, 'ui', 'vaernet.html')) && rester.every(r => !alleCss.includes(r)), rester.filter(r => alleCss.includes(r)).join());
  const crawlere = ['vg-legende-inde', 'vg-gaest-l', 'vg-kat', 'vg-prik', 'vg-gaester', 'vg-gaest', 'vg-firma', 'vg-kat-chip', 'vg-gaest-n', 'vg-maerker', 'vg-filer', 'vg-maskinfil', 'vg-tom', 'vg-matrix', 'vg-flueben', 'vg-streg', 'vg-to', 'vg-navne', 'vg-udklaedt', 'vg-statusbar', 'vg-s1', 'vg-s2', 'vg-s3', 'vg-s4', 'vg-s5', 'vg-koder'];
  const vagtCss = udenKommentarer(ui('assets/vagt.css'));
  tjek('S CSS: de 26 Crawlerne-klasser er bevaret i vagt.css', crawlere.length === 26 && crawlere.every(k => new RegExp(`\\.${k}(?![\\w-])`).test(vagtCss)), crawlere.filter(k => !new RegExp(`\\.${k}(?![\\w-])`).test(vagtCss)).join());
  tjek('S CSS: index.html indlæser tokens.css før app.css og vagt.css', /tokens\.css[\s\S]*app\.css[\s\S]*vagt\.css/.test(html) && !/site\.css|maskinrum\.css/.test(html));
}
// ── Vinduet og sikkerhedsporten ──
{
  const konf = JSON.parse(fs.readFileSync(path.join(repo, 'src-tauri', 'tauri.conf.json'), 'utf8')), v = konf.app.windows[0];
  const csp = konf.app.security.csp;
  tjek('S vindue: Overlay, skjult titel, trafiklys, minWidth 900; CSP med style-src \'self\'', v.titleBarStyle === 'Overlay' && v.hiddenTitle === true && v.trafficLightPosition && v.minWidth === 900
    && csp === "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'", JSON.stringify(v));
  const html = ui('index.html');
  tjek('S vindue: båndet er trækbart (data-tauri-drag-region="deep"), og dets knapper er <button> (blokerer trækket)', /<header class="band" data-tauri-drag-region="deep">/.test(html) && /<button class="bknap" type="button" id="hent-nu">/.test(html) && /<button class="bknap" type="button" id="hjaelp-knap"/.test(html));
  const p = cap.permissions;
  tjek('S capabilities: + core:window:allow-start-dragging, + allow-hent-nu, − core:menu:default, − core:tray:default', p.includes('core:window:allow-start-dragging') && p.includes('allow-hent-nu') && !p.includes('core:menu:default') && !p.some(x => x.startsWith('core:tray') || x.startsWith('core:menu')) && tilladt('hent_nu'), p.join(','));
  tjek('S capabilities: + allow-data-status', p.includes('allow-data-status'));
  tjek('S build.rs: data_status er med i kommandolisten', /"data_status",/.test(fs.readFileSync(path.join(repo, 'src-tauri', 'build.rs'), 'utf8')));
  tjek('S capabilities: ingen fs, shell, http, opener eller autostart:*', !p.some(x => /^(fs|shell|http|opener|autostart|dialog|clipboard)/.test(x)));
}
// ── Tokens: én kilde, genereret = committet; kontrast fra JSON'en ──
{
  const bt = await import(pathToFileURL(path.join(repo, 'design', 'build-tokens.mjs')).href);
  const tokens = bt.laesTokens(path.join(repo, 'design', 'wi24rd-tokens.json'));
  const ud = bt.byg(tokens);
  const forskel = Object.entries(ud).filter(([fil, indhold]) => !fs.existsSync(path.join(repo, fil)) || fs.readFileSync(path.join(repo, fil), 'utf8') !== indhold).map(([fil]) => fil);
  tjek('S tokens: genereret tokens.css og Wi24rdTokens.xaml = committet', forskel.length === 0 && Object.keys(ud).join() === 'ui/assets/tokens.css,design/out/Wi24rdTokens.xaml', forskel.join());
  const lag = Object.keys(tokens).filter(k => k !== 'meta').join();
  const d = tokens.component.density;
  tjek('S tokens: tre lag (raw, semantic, component), lys + mørk, tætheder (comfortable 38/14 til XAML; compact 32/13, normal 36/15, large 40/17 til CSS)', lag === 'raw,semantic,component' && Object.keys(tokens.semantic).join() === 'light,dark'
    && d.comfortable.control.height === 38 && d.comfortable.font.size === 14 && d.compact.control.height === 32 && d.compact.font.size.lg === 13 && d.normal.control.height === 36 && d.normal.font.size.lg === 15 && d.large.control.height === 40 && d.large.font.size.lg === 17 && tokens.meta.targets.css.density === 'normal', lag);
  const cssUd = ud['ui/assets/tokens.css'];
  tjek('S tokens: CSS har lyst standard, mørk efter prefers-color-scheme og [data-theme] forberedt', /@media \(prefers-color-scheme: dark\) \{\n {2}:root:not\(\[data-theme="light"\]\)/.test(cssUd) && /:root\[data-theme="dark"\]/.test(cssUd) && /--band-height: 60px;/.test(cssUd) && /--sidebar-width: 212px;/.test(cssUd));
  // Kontrast (WCAG 2.x): tekst ≥ 7:1 (AAA), --color-text-disabled ≥ 4,5:1, UI ≥ 3:1. Gennemsigtige farver lægges oven på den flade, de står på.
  const rgba = c => { if (c.startsWith('#')) return [1, 3, 5].map(i => parseInt(c.slice(i, i + 2), 16)).concat(1); const m = c.match(/rgba?\(([^)]+)\)/)[1].split(',').map(Number); return [m[0], m[1], m[2], m[3] ?? 1]; };
  const over = (f, b) => [0, 1, 2].map(i => f[i] * f[3] + b[i] * (1 - f[3])).concat(1);
  const lum = c => { const [r, g, b] = c.slice(0, 3).map(v => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };
  const kontrast = (fg, bg, under) => { let B = rgba(bg); if (B[3] < 1) B = over(B, rgba(under)); let F = rgba(fg); if (F[3] < 1) F = over(F, B); const a = lum(F), b = lum(B); return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05); };
  const hex = c => '#' + rgba(c).slice(0, 3).map(v => Math.round(v).toString(16).padStart(2, '0')).join('');
  const iGradient = (g, t) => { const s = g.stops; for (let i = 1; i < s.length; i++) { const [c0, p0] = s[i - 1], [c1, p1] = s[i]; if (t * 100 <= p1) { const k = (t * 100 - p0) / (p1 - p0), a = rgba(c0), b = rgba(c1); return hex('#' + [0, 1, 2].map(j => Math.round(a[j] + (b[j] - a[j]) * k).toString(16).padStart(2, '0')).join('')); } } return s.at(-1)[0]; };
  const LOGO = [0, 0.1, 0.2, 0.3, 0.42, 0.52], HELE = [0, 0.25, 0.5, 0.75, 1];
  const fejlPar = [], antalPar = { light: 0, dark: 0 };
  for (const tema of ['light', 'dark']) {
    const v = bt.loesTema(tokens, tema), c = k => v[k];
    const par = [
      ...['color.text', 'color.text-muted', 'color.accent-text'].flatMap(f => ['color.bg', 'color.surface', 'color.field', 'color.sidebar', 'color.hover', 'color.accent-soft', 'status.danger.bg', 'status.warn.bg', 'status.ok.bg', 'status.none.bg'].map(b => [f, b, 7])),
      ['color.text-disabled', 'color.surface', 4.5], ['color.text-disabled', 'color.sidebar', 4.5],
      ['color.on-accent', 'color.accent-fill', 7], ['color.on-accent', 'color.danger-strong', 7],
      ...['danger', 'warn', 'ok', 'none'].flatMap(s => [`status.${s}.bg`, 'color.surface', 'color.field', 'color.bg'].map(b => [`status.${s}.fg`, b, 7])), ['nav.selected-text', 'nav.selected-bg', 7, 'color.sidebar'],
      ['color.focus-ring', 'color.bg', 3], ['color.focus-ring', 'color.surface', 3], ['color.switch-off', 'color.surface', 3], ['color.danger-fill', 'color.surface', 3], ['color.control-on', 'color.surface', 3], ['color.accent', 'color.surface', 3],
    ];
    for (const [fg, bg, min, under] of par) {
      const k = kontrast(c(fg), c(bg), c(under || 'color.surface'));
      antalPar[tema]++;
      if (!(k >= min)) fejlPar.push(`${tema} ${fg}/${bg} ${k.toFixed(2)} < ${min}`);
    }
    for (const t of c('primary.fill').stops.map(([x]) => x)) { antalPar[tema]++; const k = kontrast(c('color.on-accent'), t); if (k < 7) fejlPar.push(`${tema} on-accent/primary ${t} ${k.toFixed(2)}`); }
    for (const fg of ['band.text', 'band.text-accent']) for (const t of LOGO) { antalPar[tema]++; const b = iGradient(c('band.fill'), t), k = kontrast(c(fg), b); if (k < 4.5) fejlPar.push(`${tema} ${fg}/bånd@${t} ${b} ${k.toFixed(2)}`); }
    for (const t of HELE) {
      const b = iGradient(c('band.fill'), t);
      antalPar[tema] += 2;
      const k = kontrast(c('band.pill-fg'), c('band.pill-bg'), b); if (k < 7) fejlPar.push(`${tema} pille/bånd@${t} ${k.toFixed(2)}`);
      const f = kontrast(c('band.text'), b); if (f < 3) fejlPar.push(`${tema} fokusring (band.text)/bånd@${t} ${f.toFixed(2)}`);
    }
    for (const t of LOGO) { antalPar[tema]++; const b = iGradient(c('band.fill'), t), k = kontrast(c('band.text'), c('band.subtitle-bg'), b); if (k < 7) fejlPar.push(`${tema} "for Cloudflare®"/bånd@${t} ${k.toFixed(2)}`); }
  }
  tjek(`S kontrast: alle semantiske par i lys og mørk (${antalPar.light}+${antalPar.dark} par; tekst ≥ 7:1, deaktiveret ≥ 4,5:1, UI ≥ 3:1)`, fejlPar.length === 0 && antalPar.light >= 40 && antalPar.dark >= 40, fejlPar.join(' · '));
  // Tavlen (kort, søjler) følger temaet; dens par måles i begge temaer.
  const tavleFejl = [], tavleTal = [];
  for (const tema of ['light', 'dark']) {
    const v = bt.loesTema(tokens, tema), c = k => v['board.' + k];
    for (const [fg, bg, min, navn] of [['text', 'bg', 7], ['text-2', 'bg', 7], ['text-2', 'bg-2', 7, 'legende'], ['label', 'land', 4.5], ['label', 'bg', 7], ['attack', 'bg', 3], ['attack', 'land', 3], ['colo', 'bg', 3], ['colo', 'land', 3], ['net', 'land', 1.5], ['text-2', 'bg', 7, 'akse-tekst'], ['on-colo', 'colo', 7, 'zoom-knap']]) {
      const k = kontrast(c(fg), c(bg), c('bg'));
      tavleTal.push(`${tema} ${navn || fg + '/' + bg} ${k.toFixed(2)}`);
      if (!(k >= min)) tavleFejl.push(`${tema} board.${fg}/board.${bg} ${k.toFixed(2)} < ${min}`);
    }
  }
  console.log('     tavle: ' + tavleTal.join(' · '));
  tjek('S kontrast tavle: tekst, akse, legende, landenavne, angreb, datacentre og net (også mod land) i lys og mørk (24 par)', tavleFejl.length === 0 && tavleTal.length === 24, tavleFejl.join(' · '));
  const lys = bt.loesTema(tokens, 'light'), moerk = bt.loesTema(tokens, 'dark');
  const stop = t => tokens.semantic[t].band.fill.stops.map(([c, p]) => `${c} ${p}`).join();
  tjek('S båndet: gradienten er den rigtige i begge temaer, og der er ingen bølgemaske', stop('light') === '{raw.color.blue.800} 0,{raw.color.blue.720} 55,{raw.color.blue.550} 100' && stop('dark') === '{raw.color.blue.900} 0,{raw.color.blue.820} 55,{raw.color.blue.750} 100' && !/mask-image/.test(udenKommentarer(ui('assets/app.css'))), stop('light'));
  tjek('S tavle følger temaet: board.* ligger pr. tema (lys flade i lys, mørk i mørk), ingen fast color-scheme i vagt.css', !tokens.component.shared.board && lum(rgba(lys['board.bg'])) > 0.8 && lum(rgba(moerk['board.bg'])) < 0.05
    && !/color-scheme/.test(udenKommentarer(ui('assets/vagt.css'))) && /:root \{[\s\S]*--board-bg: var\(--raw-color-board-light-bg\)/.test(cssUd) && /:root\[data-theme="dark"\] \{[\s\S]*--board-bg: var\(--raw-color-board-dark-bg\)/.test(cssUd), `${lys['board.bg']} / ${moerk['board.bg']}`);
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════
// "Hvad gør jeg?" (handlingsguide), Hvorfor-teksten (fletning, egne regler, første gang), domslinjen.
// ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════
{
  const s = side('', { ...MED_SIDEN, overblik: SIDEN_RK, siden_sidst: { vis: false, nye: [], raekker: SIDEN_RK } });
  await sov(60);
  const { d, w } = s;
  d.getElementById('dom-hjaelp').click();
  const guideTitel = d.getElementById('hjaelp-h').textContent, guideTekst = d.getElementById('hjaelp-guide').textContent;
  const guideSynlig = !d.getElementById('hjaelp').hidden && !d.getElementById('hjaelp-guide').hidden && d.getElementById('hjaelp-generelt').hidden;
  const kort = [...d.querySelectorAll('#hjaelp-guide .g-kort')];
  tjek('F2 guide: ét kort pr. Handl/Hold øje-domæne, Handl først (gamle tal ikke med)', kort.map(k => k.dataset.zone).join() === 'z2,z1' && kort[0]?.querySelector('h3 .chip-roed') && kort[1]?.querySelector('h3 .chip-gul'), kort.map(k => k.dataset.zone).join());
  const afsnit = kort.flatMap(k => [...k.querySelectorAll('.g-afsnit')]);
  tjek('F2 guide: hver top-kategori har "Rammer det dig?" og trin; med spørgsmål også "Hvis ja" og "Hvis nej"', afsnit.length === 3 && afsnit.map(a => a.dataset.kategori).join() === 'react,browsertjek,react'
    && afsnit.every(a => a.querySelector('.g-spoerg b')?.textContent === 'Rammer det dig?' && a.querySelectorAll('.g-trin li').length >= 1)
    && afsnit.filter(a => a.dataset.kategori === 'react').every(a => a.querySelector('.g-trin-h')?.textContent === 'Hvis ja, gør dette:' && a.querySelector('.g-nej b')?.textContent === 'Hvis nej:' && a.querySelector('.g-spoerg')?.textContent.includes('React eller Next.js'))
    && afsnit[1]?.querySelector('.g-trin-h')?.textContent === 'Gør dette:' && !afsnit[1]?.querySelector('.g-nej'), afsnit.map(a => a.dataset.kategori + ':' + a.querySelector('.g-trin-h')?.textContent).join(' | '));
  tjek('F2 guide: "Hvad skete der" med Hvorfor-tallene, og "Cloudflare stoppede alle N" til sidst', kort[0]?.querySelector('.g-hvad')?.textContent === 'Hvad skete der: 28 angreb i går (snit 23,1) – alle stoppet (20 blokeret, 8 udfordret).' && kort[0]?.querySelector('.g-slut')?.textContent === 'Cloudflare stoppede alle 28.', kort[0]?.querySelector('.g-hvad')?.textContent);
  tjek('F2 guide: ingen links ud af programmet (ingen href, ingen http-adresser)', !d.querySelector('#hjaelp-guide a, #hjaelp-guide [href]') && !/https?:\/\//.test(d.getElementById('hjaelp-guide').innerHTML) && d.querySelectorAll('#hjaelp-guide button:not([data-dyk])').length === 4 && d.querySelectorAll('#hjaelp-guide button.g-cve').length === 2);
  d.getElementById('hjaelp-knap').click();
  tjek('F2 "Hvad gør jeg?" ≠ "?": guiden og den generelle hjælp har forskellig titel og forskelligt indhold', guideSynlig && guideTitel === 'Hvad gør jeg?' && d.getElementById('hjaelp-h').textContent === 'Hjælp'
    && !d.getElementById('hjaelp').hidden && d.getElementById('hjaelp-guide').hidden && !d.getElementById('hjaelp-generelt').hidden && !d.getElementById('hjaelp-generelt').textContent.includes('Rammer det dig?') && guideTekst.includes('Rammer det dig?'));
  s.tast('Escape');
  const r1 = [...d.querySelectorAll('#overblik-raekker tr')].find(t => t.dataset.zone === 'z1');
  r1.querySelector('.hv-guide').click(); await sov(20);
  const k1 = [...d.querySelectorAll('#hjaelp-guide .g-kort')];
  tjek('F2 det lille link i en Hold øje-række åbner guiden for netop det domæne (rækken åbner ikke Angreb)', k1.length === 1 && k1[0]?.dataset.zone === 'z1' && !d.getElementById('hjaelp').hidden && w.location.hash !== '#vaernet?zone=z1&navn=wi24rd.com' && synlige(d).join() === 'overblik'
    && !d.querySelector('#overblik-raekker tr[data-zone="n1"] .hv-guide'), k1.map(k => k.dataset.zone).join());
  // Domslinjen: kun Hold øje → gul overskrift; alt roligt → "Intet at gøre" + "Hvorfor er alt roligt?".
  const gul = side('', { ...MED_SIDEN, overblik: [raekke('a', 'a.wi24rd.com', 'gul', { angreb_i_gaar: 20, angreb_snit: 5, angreb_fordeling: { blokeret: 20 } }), raekke('b', 'b.wi24rd.com', 'groen')] });
  const ro = side('', { ...MED_SIDEN, overblik: [raekke('a', 'a.wi24rd.com', 'groen', { i_gaar: 900, angreb_i_gaar: 2, angreb_fordeling: { blokeret: 2 } }), raekke('b', 'b.wi24rd.com', 'groen', { i_gaar: 335 })] });
  await sov(60);
  tjek('F2 domslinje: Handl = rød "kræver handling"; kun Hold øje = gul "1 domæne skal holdes øje med"; alt roligt = grøn "Intet at gøre"', d.getElementById('dom').className === 'dom roed' && d.getElementById('dom-h').textContent === '1 domæne kræver handling'
    && gul.d.getElementById('dom').className === 'dom gul' && gul.d.getElementById('dom-h').textContent === '1 domæne skal holdes øje med' && gul.d.getElementById('dom-hjaelp').textContent === 'Hvad gør jeg? →'
    && ro.d.getElementById('dom').className === 'dom groen' && ro.d.getElementById('dom-h').textContent === 'Intet at gøre', gul.d.getElementById('dom-h').textContent + ' / ' + ro.d.getElementById('dom-h').textContent);
  ro.d.getElementById('dom-hjaelp').click();
  tjek('F2 alt roligt: knappen hedder "Hvorfor er alt roligt?" og forklarer, at det er normalt', ro.d.getElementById('dom-hjaelp').textContent === 'Hvorfor er alt roligt? →' && ro.d.getElementById('hjaelp-h').textContent === 'Hvorfor er alt roligt?'
    && ro.d.querySelector('#hjaelp-guide .g-roligt')?.textContent === 'Cloudflare afviste 1.235 forsøg i går på 2 domæner, og ingen af dem var usædvanlige.' && ro.d.getElementById('hjaelp-guide').textContent.includes('Det er normalt'), ro.d.getElementById('hjaelp-guide').textContent);
}
{
  const rk = [
    raekke('w', 'abe.wi24rd.com', 'gul', { angreb_i_gaar: 24, angreb_snit: 5, angreb_i_gaar_top: [{ navn: 'Wordpress', n: 12, egen: false }, { navn: 'React RCE (CVE-2025-55182)', n: 9, egen: false }, { navn: 'Wordpress Remote Code Execution (CVE-2026-63030)', n: 3, egen: false }], angreb_fordeling: { blokeret: 24 } }),
    raekke('e', 'web.wi24rd.com', 'groen', { angreb_i_gaar: 3, angreb_snit: 0, angreb_i_gaar_top: [{ navn: 'min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)', n: 2, egen: true }, { navn: 'SQLi - x', n: 1, egen: false }], angreb_fordeling: { blokeret: 3 } }),
  ];
  const s = side('', { ...MED_SIDEN, overblik: rk });
  await sov(60);
  const hv0 = s.d.querySelectorAll('#overblik-raekker .hv-tekst')[0], hv1 = s.d.querySelectorAll('#overblik-raekker .hv-tekst')[1];
  tjek('F2 Hvorfor: samme klarsprogstekst lægges sammen (WordPress 12 + 3 → 15), sorteret efter summen', hv0.textContent === '24 angreb i går (snit 5) – alle stoppet. Mest angreb på WordPress (15) og et kendt hul i React (9). Bed din webmand tjekke, at WordPress og plugins er opdateret.', hv0.textContent);
  tjek('F2 Hvorfor: egen regel med kort navn, hele beskrivelsen kun i title', hv1.textContent.includes('din egen regel »min-regel« (2)') && !hv1.textContent.includes('max 60') && hv1.closest('td').title.includes('2 min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)')
    && s.w.hmSprog.kortEgen('En meget lang regel uden kolon og tankestreg') === 'En meget lang regel ude…' && s.w.hmSprog.kortEgen('Bloker admin – kun fra kontoret') === 'Bloker admin' && s.w.hmSprog.kortEgen('abcdefghijklmnopqrstuvwxyz') === 'abcdefghijklmnopqrstuvw…' && s.w.hmSprog.kortEgen('abcdefghijklmnopqrstuvwx') === 'abcdefghijklmnopqrstuvwx', hv1.textContent);
  tjek('F2 Hvorfor: "(første gang)" i stedet for "(snit 0)"', hv1.textContent.startsWith('3 angreb i går (første gang) – alle stoppet.') && !s.d.getElementById('overblik-raekker').textContent.includes('snit 0'), hv1.textContent);
  // Ordlisten i sprog.js og i notify.rs (notifikationerne) er den samme, post for post; handlingsbogen dækker hver kategori.
  const rs = fs.readFileSync(path.join(repo, 'src-tauri', 'src', 'notify.rs'), 'utf8');
  const blok = rs.slice(rs.indexOf('pub const ORDLISTE'), rs.indexOf('];', rs.indexOf('pub const ORDLISTE')));
  const ustreng = x => x.replace(/\\(.)/g, '$1');
  const rust = [...blok.matchAll(/\(\s*"([a-z0-9]+)",\s*"((?:[^"\\]|\\.)*)",\s*"([^"]*)",\s*"([^"]*)",\s*"([^"]*)",\s*"([^"]*)",?\s*\)/g)].map(m => [m[1], ustreng(m[2]), m[3], m[4], m[5], m[6]]);
  const js = s.w.hmSprog.ordliste;
  tjek('F2 ordlisten er ens i sprog.js og notify.rs (id, mønster, da, en, råd), og handlingsbogen har en post pr. kategori + egen + andet', rust.length === 9 && JSON.stringify(rust) === JSON.stringify(js)
    && [...new Set(js.map(o => o[0])), 'egen', 'andet'].every(id => s.w.hmSprog.bogIds.includes(id)), rust.length + ' / ' + js.length + ' ' + JSON.stringify(rust.find((r, i) => JSON.stringify(r) !== JSON.stringify(js[i]))));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════
// CVE-numre som knapper (aabn_cve → cve.org i browseren) og "Se de N angreb" (vis=angreb-i-gaar).
// ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════
const I_GAAR = '2026-11-02', I_FORGAARS = '2026-11-01';
const sen = (t, regel, cve) => ({ t, regel, cve, land: 'US', sti: '/', colo: 'EWR', metode: 'POST' });
const SEN_IDAG = [sen('2026-11-03T05:00:00Z', 'React - RCE - CVE:CVE-2025-55182', 'CVE-2025-55182'), sen('2026-11-03T04:00:00Z', 'Egen regel uden CVE', null)];
const dagAngreb = (dag, angreb, seneste, regler = []) => ({ dag, angreb, seneste, regler });
const medDage = dageListe => vaernUd({ seneste: SEN_IDAG, angreb_dage: dageListe,
  regler: [vaernUd().regler[0], { navn: 'Wordpress - Broken Access Control', produkt: 'Wordpress', type: 'Broken Access Control', cve: null, n7: 3, n30: 9, foerst: '2026-10-07', sidst: '2026-11-01', stier: [], lande: [] }] });
const F3_SVAR = (v, overblik = OVERBLIK) => ({ ...MED_TOKEN, overblik, kortdata: KORT, vaern_cache: v, vaern_oversigt: v, koe_prioritet: null, aabn_cve: null, siden_sidst: { vis: false, nye: [], raekker: overblik }, valg_hent: [], koe_status: null });
{
  const v = medDage([dagAngreb(I_GAAR, 2, [sen(`${I_GAAR}T23:00:00Z`, 'React - RCE - CVE:CVE-2025-55182', 'CVE-2025-55182'), sen(`${I_GAAR}T20:00:00Z`, 'PHP - Code Injection', null)]), dagAngreb(I_FORGAARS, 0, [])]);
  const s = side('#vaernet?zone=z1&navn=wi24rd.com', F3_SVAR(v));
  await sov(80);
  const { d, kald } = s;
  const celler = [...d.querySelectorAll('#vg-seneste tbody tr')].map(tr => tr.children[1]);
  const knap = celler[0].querySelector('button.vg-cve-knap');
  knap?.click(); await sov(10);
  tjek('F3 CVE i "De seneste": knap med title/aria-label, kalder aabn_cve med netop id\'et; regel uden CVE er tekst', knap?.textContent === 'CVE-2025-55182' && knap?.getAttribute('aria-label') === 'Åbn CVE-2025-55182 på cve.org (åbner din browser)' && knap?.title === knap?.getAttribute('aria-label')
    && JSON.stringify(kald.filter(k => k[0] === 'aabn_cve')) === '[["aabn_cve",{"id":"CVE-2025-55182"}]]' && !celler[1].querySelector('.vg-cve-knap') && celler[1].querySelector('button[data-dyk]') && celler[1].textContent === 'Egen regel uden CVE', celler.map(c => c.innerHTML).join(' | '));
  const kortKnap = d.querySelector('#vg-regler .vg-cve button.vg-cve-knap');
  kortKnap?.click(); await sov(10);
  tjek('F3 CVE på regelkortet: knap, der kalder aabn_cve; regelkort uden CVE har ingen knap', kortKnap?.textContent === 'CVE-2025-55182' && kald.filter(k => k[0] === 'aabn_cve').length === 2 && [...d.querySelectorAll('#vg-regler .vg-regel')][1].querySelectorAll('.vg-cve-knap').length === 0);
  tjek('F3 ingen <a href="http…"> i appen (statisk HTML, tegnet Angreb, scripts)', !d.querySelector('a[href^="http"], [href^="http"]') && !/href="https?:/.test(ui('index.html')) && ['ramme.js', 'visning-overblik.js', 'visning-siden.js', 'visning-vaernet.js', 'visning-indstillinger.js'].every(f => !/href[^\n]{0,40}https?:\/\//.test(ui('assets/' + f))));
  tjek('F3 hjælpen ("?") siger, at et klik på et CVE-nummer åbner cve.org i browseren', d.querySelector('#hjaelp-generelt [data-t=hjCve]').textContent.startsWith('Et klik på et CVE-nummer (fx CVE-2025-55182) åbner CVE’ens side på cve.org i din browser'));
}
{
  // Guiden: "Læs om CVE-… ↗", linkteksten med tallet, og adressen med vis=angreb-i-gaar.
  const rk = [SIDEN_RK[0], raekke('e', 'en.wi24rd.com', 'gul', { angreb_i_gaar: 1, angreb_snit: 0, angreb_fordeling: { blokeret: 1 }, angreb_i_gaar_top: [{ navn: 'Wordpress', n: 1, egen: false }] }), raekke('n', 'nat.wi24rd.com', 'gul', { opdateres: true, angreb_i_gaar: 5, angreb_snit: 1, angreb_fordeling: { blokeret: 5 } })];
  const s = side('', F3_SVAR(medDage([]), rk));
  await sov(60);
  const { d, w, kald } = s;
  d.getElementById('dom-hjaelp').click();
  const kort = [...d.querySelectorAll('#hjaelp-guide .g-kort')], cve = kort[0].querySelector('.g-afsnit button.g-cve');
  cve.click(); await sov(10);
  tjek('F3 guiden: "Læs om CVE-2025-55182 ↗" under kategorien kalder aabn_cve; kategorier uden CVE har ingen', cve.textContent === 'Læs om CVE-2025-55182 ↗' && cve.getAttribute('aria-label') === 'Åbn CVE-2025-55182 på cve.org (åbner din browser)'
    && JSON.stringify(kald.filter(k => k[0] === 'aabn_cve').pop()) === '["aabn_cve",{"id":"CVE-2025-55182"}]' && !kort[1].querySelector('.g-cve'), cve.textContent);
  tjek('F3 linkteksten: "Se de 28 angreb fra i går →", ental "Se angrebet fra i går →", om natten "fra i forgårs"', kort.map(k => k.querySelector('.g-se').textContent).join(' | ') === 'Se de 28 angreb fra i går → | Se angrebet fra i går → | Se de 5 angreb fra i forgårs →', kort.map(k => k.querySelector('.g-se')?.textContent).join(' | '));
  kort[0].querySelector('.g-se').click(); await sov(40);
  tjek('F3 linket åbner #vaernet?zone=z2&navn=xyz.wi24rd.com&vis=angreb-i-gaar og lukker guiden', w.location.hash === '#vaernet?zone=z2&navn=xyz.wi24rd.com&vis=angreb-i-gaar' && d.getElementById('hjaelp').hidden && synlige(d).join() === 'vaernet', w.location.hash);
}
{
  // vis=angreb-i-gaar: kun dagens egentlige angreb (Rust har sorteret rutine fra), chip og ×.
  const iGaar = [sen(`${I_GAAR}T23:00:00Z`, 'React - RCE - CVE:CVE-2025-55182', 'CVE-2025-55182'), sen(`${I_GAAR}T20:00:00Z`, 'PHP - Code Injection', null)];
  const v = medDage([dagAngreb(I_GAAR, 2, iGaar), dagAngreb(I_FORGAARS, 1, [sen(`${I_FORGAARS}T10:00:00Z`, 'Wordpress - Broken Access Control', null)])]);
  const s = side('#vaernet?zone=z1&navn=wi24rd.com&vis=angreb-i-gaar', F3_SVAR(v));
  await sov(80);
  const { d, w } = s;
  const tider = () => [...d.querySelectorAll('#vg-seneste tbody tr')].map(tr => tr.firstElementChild.textContent);
  const chip = d.getElementById('vg-filter');
  tjek('F3 filter: "De seneste" viser kun i går (ikke dagens hændelser), chip "Viser: angreb i går (2)" lige over tabellen', tider().join() === `${w.hmSprog.dato(I_GAAR)} 23:00,${w.hmSprog.dato(I_GAAR)} 20:00` && chip && chip.firstChild.textContent === 'Viser: angreb i går (2)' && chip.nextElementSibling === d.getElementById('vg-seneste'), tider().join() + ' / ' + (chip && chip.textContent));
  chip.querySelector('button').click(); await sov(80);
  tjek('F3 filter: × fjerner filteret uden genindlæsning (adressen uden vis, almindelig liste, ingen chip)', w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com' && !d.getElementById('vg-filter') && tider().join() === '3. nov. 2026 05:00,3. nov. 2026 04:00' && w.location.pathname === '/index.html', w.location.hash + ' ' + tider().join());
  const nat = side('#vaernet?zone=n&navn=nat.wi24rd.com&vis=angreb-i-gaar', F3_SVAR(v, [raekke('n', 'nat.wi24rd.com', 'gul', { opdateres: true, angreb_i_gaar: 1 })]));
  await sov(80);
  tjek('F3 filter om natten: statusdagen er i forgårs (som Overblikket)', nat.d.getElementById('vg-filter')?.firstChild.textContent === 'Viser: angreb i forgårs (1)' && nat.d.querySelectorAll('#vg-seneste tbody tr').length === 1, nat.d.getElementById('vg-filter')?.textContent);
}
{
  // Ærlig tekst: cachen har kun 12 enkelte angreb for i går (højst 12 hændelser pr. dag gemmes); ingen opfundne rækker.
  const tolv = Array.from({ length: 12 }, (_, i) => sen(`${I_GAAR}T${String(23 - i).padStart(2, '0')}:00:00Z`, 'React - RCE - CVE:CVE-2025-55182', 'CVE-2025-55182'));
  const a = side('#vaernet?zone=z1&navn=wi24rd.com&vis=angreb-i-gaar', F3_SVAR(medDage([dagAngreb(I_GAAR, 30, tolv), dagAngreb(I_FORGAARS, 0, [])])));
  await sov(80);
  tjek('F3 afkortet liste: "Viser de 12 seneste af 30 angreb i går", 12 rækker', a.d.getElementById('vg-filter')?.firstChild.textContent === 'Viser de 12 seneste af 30 angreb i går' && a.d.querySelectorAll('#vg-seneste tbody tr').length === 12, a.d.getElementById('vg-filter')?.textContent);
  const b = side('#vaernet?zone=z1&navn=wi24rd.com&vis=angreb-i-gaar', F3_SVAR(medDage([dagAngreb(I_GAAR, 30, [], [{ navn: 'React - RCE - CVE:CVE-2025-55182', n: 30 }]), dagAngreb(I_FORGAARS, 0, [])])));
  await sov(80);
  const regler = [...b.d.querySelectorAll('#vg-regler .vg-regel h3')].map(h => h.textContent);
  tjek('F3 ingen enkelte angreb gemt: "Reglerne, der slog til" filtreret til dagens regler, og chippen siger det', !b.d.getElementById('vg-seneste') && regler.length === 1 && new URLSearchParams(b.d.querySelector('#vg-regler .vg-regel-aabn').dataset.dyk).get('regel') === 'React - RCE - CVE:CVE-2025-55182'
    && b.d.getElementById('vg-filter')?.firstChild.textContent === 'Viser: reglerne bag de 30 angreb i går (de enkelte angreb er ikke gemt for den dag)' && b.d.getElementById('vg-filter').parentElement === b.d.getElementById('vg-regler').parentElement, b.d.getElementById('vg-filter')?.textContent);
}

// ── Indstillinger-siden ──
const B2 = x => Object.assign({
  token_status: { sat: true }, zones_cache: { zoner: OVERBLIK.map(r => ({ id: r.id, navn: r.navn })), hentet: NU_ISO }, zones: OVERBLIK.map(r => ({ id: r.id, navn: r.navn })), overblik: OVERBLIK,
  valg_hent: ['z1'], koe_status: null, koe_prioritet: null, autostart_hent: true, autostart_saet: true, notifikation_hent: { niveau: 'standard', roed: true, zoner: {} }, notifikation_saet: null,
  siden_sidst: { vis: false, nye: [], raekker: [] }, siden_sidst_set: null, data_status: { bytes: 2412000, foerste: '2026-10-04', sidste: '2026-11-02', domaener: 2, nulstil: { sikkerhed: null, trafik: null } }, token_delete: null, token_set: null, hent_nu: null,
  kortdata: KORT, vaern_cache: vaernUd(), vaern_oversigt: vaernUd(), vaern_samlet_cache: { vaern: vaernUd(), valgte: [], domaener: 1, med_tal: 1 },
}, x);
{
  const s = side('#indstillinger', B2()), { d, w, kald } = s;
  await sov(60);
  const grupper = [...d.querySelectorAll('.indst > h2')].map(h => h.textContent);
  tjek('Indstillinger, grupperne i rækkefølge: Generelt, Domæner, Notifikationer, Forbindelse, Data, Om', grupper.join() === 'Generelt,Domæner,Notifikationer,Forbindelse,Data,Om', grupper.join());
  const generelt = [...d.querySelectorAll('.indst > .kort')[0].querySelectorAll('.raekke')].map(r => r.querySelector('.etik').firstChild.textContent);
  tjek('Indstillinger, Generelt: Sprog er første række, så Datoformat, Land, Udseende, Tekststørrelse og Start ved login', generelt.join('|') === 'Sprog|Datoformat|Land|Udseende|Tekststørrelse|Start ved login', generelt.join('|'));
  tjek('Indstillinger, ingen <details> ud over trinene til API Token, og "token" står kun som "API Token" og Cloudflares egne ord i Indstillinger (da)', !d.querySelector('#visning-indstillinger details:not(.token-trin)') && !/token/i.test(d.getElementById('visning-indstillinger').textContent.replace(/API Token(et)?|Create (Custom )?Token|Token name/g, '')), d.getElementById('visning-indstillinger').textContent.match(/.{20}token.{20}/i)?.[0]);
  tjek('Indstillinger, pr. domæne er en rigtig tabel: domæne · vælger', d.querySelectorAll('#pr-domaene-liste tr').length === 3 && d.querySelector('#pr-domaene-liste tr th').textContent === 'abe.wi24rd.com' && d.querySelector('#pr-domaene-liste tr td select'));
  tjek('Indstillinger, noten om Systemindstillinger står i Notifikationer', d.querySelector('.indst [data-t=notifSystem]').textContent.includes('Systemindstillinger → Notifikationer'));

  // Udseende
  const tema = v => d.querySelector(`input[name=tema][value=${v}]`), html = d.documentElement;
  tjek('Indstillinger, Udseende: System er forvalgt og uden data-theme', tema('system').checked && !html.hasAttribute('data-theme'));
  tema('dark').click();
  tjek('Indstillinger, Udseende: Mørk sætter data-theme straks og huskes', html.getAttribute('data-theme') === 'dark' && w.localStorage.getItem('hm-tema') === 'dark', html.outerHTML.slice(0, 80));
  tema('light').click();
  tjek('Indstillinger, Udseende: Lys sætter data-theme=light', html.getAttribute('data-theme') === 'light' && w.localStorage.getItem('hm-tema') === 'light');
  tema('system').click();
  tjek('Indstillinger, Udseende: System fjerner data-theme', !html.hasAttribute('data-theme') && w.localStorage.getItem('hm-tema') === 'system');
  const husket = side('#indstillinger', B2(), 'da', 'windows', x => x.localStorage.setItem('hm-tema', 'dark'));
  await sov(40);
  tjek('Indstillinger, Udseende: valget gælder ved næste start (før siden tegnes) og står i segmentet', husket.d.documentElement.getAttribute('data-theme') === 'dark' && husket.d.querySelector('input[name=tema][value=dark]').checked);

  // Notifikationsradioerne
  for (const v of ['rolig', 'alt', 'standard']) {
    d.querySelector(`#indst-niveau input[value=${v}]`).click(); await sov(5);
    const g = kald.filter(k => k[0] === 'notifikation_saet').pop();
    tjek(`Indstillinger, radioen ${v} gemmer niveau ${v}`, g && g[1].niveau === v && g[1].roed === true, JSON.stringify(g));
  }

  // Forbindelse
  const chip = d.getElementById('forb-chip');
  tjek('Indstillinger, status: Forbundet · 3 domæner (grøn)', chip.textContent === 'Forbundet · 3 domæner' && chip.classList.contains('chip-groen'), chip.outerHTML);
  s.emit('hentekoe', { zone: null, zoner_i_alt: 3, zoner_faerdige: 0, doegn_i_alt: 1, doegn_faerdige: 0, zoner_fejl: 0, tilstand: 'fejl', fejl: { kode: 'afvist', status: 403 } });
  tjek('Indstillinger, status: køens fejlkode afvist → Afvist (rød)', chip.textContent === 'Afvist' && chip.classList.contains('chip-roed'), chip.outerHTML);
  s.emit('hentekoe', { zone: null, zoner_i_alt: 3, zoner_faerdige: 3, doegn_i_alt: 1, doegn_faerdige: 1, zoner_fejl: 0, tilstand: 'faerdig' });
  const form = d.getElementById('skift-form'), knap = d.getElementById('skift-noegle'), felt = d.getElementById('skift-felt');
  tjek('Indstillinger, Skift nøgle: feltet er foldet sammen', form.hidden && knap.getAttribute('aria-expanded') === 'false');
  knap.click();
  tjek('Indstillinger, Skift nøgle: folder feltet ud i kortet og giver det fokus', !form.hidden && knap.getAttribute('aria-expanded') === 'true' && d.activeElement === felt && form.closest('.kort') && felt.type === 'password');
  felt.value = 'hemmelig';
  d.getElementById('skift-annuller').click();
  tjek('Indstillinger, Skift nøgle: Annullér folder sammen, tømmer feltet og kalder intet', form.hidden && felt.value === '' && !kald.some(k => k[0] === 'token_set') && d.activeElement === knap);
  knap.click(); felt.value = 'ny-noegle';
  form.dispatchEvent(new w.Event('submit', { cancelable: true, bubbles: true })); await sov(40);
  tjek('Indstillinger, Skift nøgle: Gem kalder token_set, tømmer feltet og folder sammen', JSON.stringify(kald.filter(k => k[0] === 'token_set').pop()) === '["token_set",{"token":"ny-noegle"}]' && felt.value === '' && form.hidden && d.getElementById('skift-status').textContent === 'API Tokenet er gyldigt og gemt i nøgleringen.', d.getElementById('skift-status').textContent);

  // Glem nøglen
  const dialog = d.getElementById('glem-dialog');
  d.getElementById('glem').click();
  tjek('Indstillinger, Glem nøglen: bekræftelse i en dialog med den aftalte tekst', dialog.hasAttribute('open') && dialog.textContent.includes('Glem API Tokenet?') && d.getElementById('glem-p').textContent === 'Programmet holder op med at hente, og dine valg pr. domæne slettes. Historikken bliver.' && !kald.some(k => k[0] === 'token_delete'));
  d.getElementById('glem-nej').click(); await sov(10);
  tjek('Indstillinger, Glem nøglen: Annullér lukker og kalder intet', !dialog.hasAttribute('open') && !kald.some(k => k[0] === 'token_delete'));

  // Data
  tjek('Indstillinger, Data: størrelse, periode og "N af M" fra data_status', d.getElementById('data-stoerrelse').textContent === '2,4 MB' && d.getElementById('data-historik').textContent === '4. okt. – 2. nov. 2026' && d.getElementById('data-domaener').textContent === '2 af 3', [d.getElementById('data-stoerrelse').textContent, d.getElementById('data-historik').textContent, d.getElementById('data-domaener').textContent].join(' | '));
  tjek('Indstillinger, Data: noten siger "computer" uden for macOS', d.getElementById('data-hist-note').textContent === 'Programmet gemmer al historik på din computer og sletter intet af sig selv.');
  d.getElementById('data-hent').click(); await sov(5);
  tjek('Indstillinger, Data: "Hent nu" er sekundær og kalder hent_nu', !d.getElementById('data-hent').classList.contains('primaer') && kald.some(k => k[0] === 'hent_nu'));

  // Om
  const om = d.querySelector('.om');
  tjek('Indstillinger, Om: version fra getVersion, ingen eksterne links', d.getElementById('om-version').textContent === 'Version 9.8.7' && !om.querySelector('a, [href]:not(use)') && !/https?:/.test(om.textContent), om.textContent);
  tjek('Indstillinger, Om: navn, "for Cloudflare®" for sig, linjen, skrifter/kortdata og varemærket på ét sprog', d.querySelector('#visning-indstillinger .velkomst-navn').textContent === 'Wi24rdHuginn and Muninnfor Cloudflare®' && d.querySelector('#visning-indstillinger .velkomst-wi em').textContent === '24' && d.querySelector('#visning-indstillinger .velkomst-cf').textContent === 'for Cloudflare®'
    && om.textContent.includes('Apache-2.0-licens · Ingen telemetri · Intet sendes til Wi24rd') && om.textContent.includes('Outfit og JetBrains Mono') && om.textContent.includes('Natural Earth') && om.textContent.includes('Cloudflare® er et registreret varemærke tilhørende Cloudflare, Inc. Wi24rd Huginn and Muninn er ikke tilknyttet Cloudflare, Inc.') && !/registered trademark/.test(om.textContent), om.textContent);
  tjek('Indstillinger, ingen style= i Indstillinger', !d.getElementById('visning-indstillinger').outerHTML.includes('style='));
  d.getElementById('glem').click(); d.getElementById('glem-ja').click(); await sov(30);
  tjek('Indstillinger, Glem nøglen: Glem kalder token_delete, dialogen lukker', !dialog.hasAttribute('open') && kald.filter(k => k[0] === 'token_delete').length === 1);
}
{
  const s = side('#indstillinger', B2({ token_status: { sat: false }, zones_cache: { zoner: [], hentet: null }, data_status: { bytes: 900, foerste: null, sidste: null, domaener: 0 } }), 'en', 'macos');
  await sov(50);
  const { d } = s, c = d.getElementById('forb-chip');
  tjek('Indstillinger, engelsk uden API Token: "No API Token", ingen anden "token", Glem skjult, Hent slået fra', c.textContent === 'No API Token' && c.classList.contains('chip-graa') && !/token/i.test(d.getElementById('visning-indstillinger').textContent.replace(/API Token(et)?|Create (Custom )?Token|Token name/g, '')) && d.getElementById('glem').hidden && d.getElementById('data-hent').disabled, c.outerHTML);
  tjek('Indstillinger, engelsk: grupperne og trademarket på engelsk', [...d.querySelectorAll('.indst > h2')].map(h => h.textContent).join() === 'General,Domains,Notifications,Connection,Data,About' && d.querySelector('.om').textContent.includes('Cloudflare® is a registered trademark of Cloudflare, Inc. Wi24rd Huginn and Muninn is not affiliated with Cloudflare, Inc.'));
  tjek('Indstillinger, tom database: "1 kB", "None yet", macOS-noterne', d.getElementById('data-stoerrelse').textContent === '1 kB' && d.getElementById('data-historik').textContent === 'None yet' && d.getElementById('data-hist-note').textContent === 'The program keeps all history on your Mac and deletes nothing by itself.' && d.getElementById('forb-note').textContent === 'The API Token is in the macOS Keychain.', d.getElementById('data-stoerrelse').textContent);
  const fejl = side('#indstillinger', B2({ data_status: () => { throw { kode: 'lager' }; } }));
  await sov(40);
  tjek('Indstillinger, data_status fejler: streger, ingen falske tal', fejl.d.getElementById('data-stoerrelse').textContent === '–' && fejl.d.getElementById('data-domaener').textContent === '–');
  const flere = side('#indstillinger', B2({ data_status: { bytes: 2000000, foerste: '2025-12-30', sidste: '2026-01-02', domaener: 7 } }));
  await sov(40);
  tjek('Indstillinger, Data: flere domæner i historikken end på zonelisten → kun tallet; periode over årsskifte', flere.d.getElementById('data-domaener').textContent === '7' && flere.d.getElementById('data-historik').textContent === '30. dec. 2025 – 2. jan. 2026', flere.d.getElementById('data-historik').textContent);
  // "API Token" er ordet. "token" står ikke på anden måde (kun Cloudflares "Custom Token" og adressen).
  const strenge = (w, o, sti = '') => Object.entries(o).flatMap(([k, v]) => (typeof v === 'string' ? [[sti + k, v]] : v && typeof v === 'object' ? strenge(w, v, sti + k + '.') : []));
  for (const sp of ['da', 'en']) {
    const x = side('', B2(), sp); await sov(5);
    const alle = strenge(x.w, x.w.hmSprog.T), med = alle.filter(([, v]) => /token/i.test(v.replace(/API Token(?:s|et)?|Create (?:Custom )?Token|Custom Token|Token name|api-tokens/g, ''))).map(([k]) => k);
    tjek(`Indstillinger, ${sp}: "token" står kun som API Token, Custom Token og Cloudflares adresse`, alle.filter(([, v]) => v.includes('API Token')).length > 15 && med.length === 0, med.join());
  }
}
// ── Programmenuens Om ──
{
  const sc = [];
  const s = side('#overblik', B2(), 'da', 'macos');
  await sov(50);
  const { d, w, emit } = s;
  d.getElementById('indst-om').scrollIntoView = o => sc.push(o);
  emit('visning', 'om'); await sov(30);
  tjek('Indstillinger, Om i programmenuen: åbner Indstillinger og ruller til Om', w.location.hash === '#indstillinger?afsnit=om' && synlige(d).join() === 'indstillinger' && sc.length === 1 && sc[0].block === 'start', w.location.hash + ' ' + sc.length);
  d.querySelector('a[data-visning=overblik]').click(); await sov(30);
  emit('historik', 'tilbage'); await sov(40);
  tjek('Indstillinger, tilbage til Om ruller ikke igen (rullepositionen gendannes)', w.location.hash === '#indstillinger?afsnit=om' && sc.length === 1, w.location.hash + ' ' + sc.length);
  const u = side('', B2({ token_status: { sat: false } }), 'da', 'macos');
  await sov(40);
  u.emit('visning', 'om'); await sov(30);
  tjek('Indstillinger, Om i programmenuen virker også uden nøgle', synlige(u.d).join() === 'indstillinger', synlige(u.d).join());
}
// ── Ugetallets farve ──
{
  const s = side('#overblik', B2({ overblik: [raekke('a', 'alfa.wi24rd.com', 'groen', { angreb_uge: 412, angreb_ugen_foer: 28, aendring: 1371, sidst_hentet: NU_ISO }), raekke('b', 'wi24rd.com', 'groen', { angreb_uge: 102, angreb_ugen_foer: 68, aendring: 50, sidst_hentet: NU_ISO }), raekke('c', 'c.wi24rd.com', 'groen', { angreb_uge: 10, angreb_ugen_foer: 60, aendring: -83, sidst_hentet: NU_ISO })] }));
  await sov(50);
  const { d, w } = s, f = w.hmRamme.ugeKlasse;
  const tal = [[412, 28, 'op'], [102, 68, ''], [121, 68, ''], [11, 9, ''], [3, 0, ''], [110, 126, ''], [10, 60, 'ned'], [20, 0, 'op'], [19, 0, ''], [0, 20, 'ned'], [0, 19, ''], [136, 68, 'op'], [34, 68, 'ned'], [0, 0, '']];
  const forkert = tal.filter(([a, b, v]) => f(a, b) !== v).map(([a, b]) => `${a}/${b}=${f(a, b)}`);
  tjek('Ugetal, ren funktion: rød kun ved ≥ 2× og ≥ 20, grøn kun ved ≤ ½ og ≥ 20 (14 tal)', forkert.length === 0, forkert.join());
  const rows = [...d.querySelectorAll('#overblik-raekker tr')], uge = r => celler(r)[5];
  tjek('Ugetal, tabellen: 412/28 rød, 102/68 neutral (procenten i title), 10/60 grøn', uge(rows[0]).querySelector('.op') && !uge(rows[1]).querySelector('.op, .ned') && uge(rows[1]).title === '+50 % mod ugen før' && uge(rows[2]).querySelector('.ned'), uge(rows[1]).outerHTML);
}
// ── Domænevælgeren i båndet ──
{
  const s = side('#overblik', B2()), { d, w, emit } = s;
  await sov(60);
  const vaelger = d.getElementById('vg-zone'), bånd = d.getElementById('band-zone');
  tjek('Domænevælger, vælgeren står i båndet lige efter logoet, ikke i titelrækken', bånd.closest('header.band') && bånd.previousElementSibling.classList.contains('band-logo') && !d.querySelector('.titelrad #vg-zone'));
  tjek('Domænevælger, skjult uden for Angreb (Overblik)', bånd.hidden);
  d.querySelector('#overblik-raekker tr').click(); await sov(60);
  tjek('Domænevælger, synlig i Angreb; titlen viser domænet', !bånd.hidden && d.getElementById('vaernet-under').textContent === 'wi24rd.com' && vaelger.value === 'z1', d.getElementById('vaernet-under').textContent);
  tjek('Domænevælger, samme indhold: "Samlet (N)" + domænerne', [...vaelger.options].map(o => o.textContent).join() === 'Samlet (3),abe.wi24rd.com,wi24rd.com,xyz.wi24rd.com', [...vaelger.options].map(o => o.textContent).join());
  const laengde = w.history.length;
  vaelger.value = 'z2'; vaelger.dispatchEvent(new w.Event('change')); await sov(60);
  tjek('Domænevælger, valg skifter domæne og laver ét trin', w.location.hash === '#vaernet?zone=z2&navn=xyz.wi24rd.com' && d.getElementById('vaernet-under').textContent === 'xyz.wi24rd.com' && w.history.length === laengde + 1, w.location.hash + ' ' + w.history.length);
  emit('hentekoe', { zone: 'z3', navn: 'abe.wi24rd.com', doegn_faerdige: 1, doegn_i_alt: 9, zoner_faerdige: 0, zoner_i_alt: 3, zoner_fejl: 0, tilstand: 'henter' }); await sov(30);
  emit('hentekoe', { zone: null, doegn_faerdige: 9, doegn_i_alt: 9, zoner_faerdige: 3, zoner_i_alt: 3, zoner_fejl: 0, tilstand: 'faerdig' }); await sov(30);
  tjek('Tilbage og frem, hentekøens opdateringer laver aldrig et trin', w.history.length === laengde + 1 && w.location.hash === '#vaernet?zone=z2&navn=xyz.wi24rd.com');
  d.querySelector('a[data-visning=indstillinger]').click(); await sov(30);
  tjek('Domænevælger, skjult i Indstillinger', bånd.hidden);
  const html = ui('index.html'), css = udenKommentarer(ui('assets/app.css'));
  tjek('Domænevælger, flag pr. visning: domaener: true kun på Angreb', /R\.registrer\('vaernet', \{\s*domaener: true/.test(ui('assets/visning-vaernet.js')) && /\.domaener\)/.test(ui('assets/ramme.js')) && !/domaener: true/.test(ui('assets/visning-overblik.js') + ui('assets/visning-siden.js') + ui('assets/visning-indstillinger.js')));
  tjek('Domænevælger, ikke trækbar: <select> uden drag-attribut; hvid pille med båndets pilletokens; navnet forkortes < 1100 px', !/<select id="vg-zone"[^>]*data-tauri-drag-region/.test(html) && /\.band-vaelger select \{[^}]*var\(--band-pill-bg\)[^}]*var\(--band-pill-fg\)/.test(css) && /@media \(max-width: 1099px\) \{\s*\.band-wi \{ display: none; \}/.test(css));
}
// ── Tilbage og frem ──
{
  const s = side('#overblik', B2()), { d, w, emit, tast } = s;
  await sov(60);
  const hoved = d.getElementById('indhold'), tb = d.getElementById('hist-tilbage'), fr = d.getElementById('hist-frem'), rul = t => { hoved.scrollTop = t; };
  tjek('Tilbage og frem, ved start: ‹ og › er deaktiveret, tooltip med genvejen (Windows)', tb.disabled && fr.disabled && tb.title === 'Tilbage (Alt+←)' && fr.title === 'Frem (Alt+→)' && tb.getAttribute('aria-label') === 'Tilbage', tb.title);
  tjek('Tilbage og frem, ‹ › står før logoet, så vælgeren kan stå lige efter det', d.querySelector('.band-nav').nextElementSibling.classList.contains('band-logo'));
  rul(300);
  d.querySelector('#overblik-raekker tr').click(); await sov(60);
  tjek('Tilbage og frem, nyt trin: Angreb for z1, ‹ aktiv, › deaktiveret, rullet til toppen', w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com' && !tb.disabled && fr.disabled && hoved.scrollTop === 0, `${hoved.scrollTop}`);
  rul(500);
  const v = d.getElementById('vg-zone'); v.value = 'z2'; v.dispatchEvent(new w.Event('change')); await sov(60);
  rul(120);
  tb.click(); await sov(60);
  tjek('Tilbage og frem, ‹: tilbage til z1 med domæne, vælger og rulleposition 500', w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com' && v.value === 'z1' && d.getElementById('vaernet-under').textContent === 'wi24rd.com' && hoved.scrollTop === 500 && !fr.disabled, `${w.location.hash} ${v.value} ${hoved.scrollTop}`);
  tb.click(); await sov(60);
  tjek('Tilbage og frem, ‹: tilbage til Overblik og rulleposition 300; ‹ deaktiveret i enden', synlige(d).join() === 'overblik' && hoved.scrollTop === 300 && tb.disabled && !fr.disabled, `${synlige(d)} ${hoved.scrollTop}`);
  tast('ArrowRight', { altKey: true }); await sov(60);
  tjek('Tilbage og frem, Alt+→ (Windows): frem til z1', w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com' && hoved.scrollTop === 500, w.location.hash);
  fr.click(); await sov(60);
  tjek('Tilbage og frem, ›: frem til z2 med rulleposition 120; › deaktiveret i enden', v.value === 'z2' && hoved.scrollTop === 120 && fr.disabled && !tb.disabled, `${v.value} ${hoved.scrollTop}`);
  fr.click(); await sov(30);
  tjek('Tilbage og frem, › i enden gør intet', v.value === 'z2');
  tast('ArrowLeft', { altKey: true }); await sov(60);
  tjek('Tilbage og frem, Alt+←: tilbage', v.value === 'z1', v.value);
  tb.click(); await sov(60); // trin 0 af 2: frem kan nå to trin
  d.querySelector('a[data-visning=siden-sidst]').click(); await sov(60);
  tjek('Tilbage og frem, et nyt skift to trin før enden rydder frem', synlige(d).join() === 'siden-sidst' && w.history.state?.hmTrin === 1 && fr.disabled && !tb.disabled && hoved.scrollTop === 0, JSON.stringify(w.history.state));
  d.querySelector('a[data-visning=siden-sidst]').click(); await sov(30);
  tjek('Tilbage og frem, et klik på den aktuelle visning er intet trin (trinets nummer står)', w.history.state?.hmTrin === 1 && fr.disabled, JSON.stringify(w.history.state));
  // Filter som trin
  d.querySelector('#side a[data-visning=overblik]').click(); await sov(30);
  w.hmRamme.gaaTil('vaernet', new URLSearchParams({ zone: 'z1', navn: 'wi24rd.com', vis: 'angreb-i-gaar' })); await sov(60);
  w.hmRamme.gaaTil('vaernet', new URLSearchParams({ zone: 'z1', navn: 'wi24rd.com' })); await sov(60);
  tb.click(); await sov(60);
  tjek('Tilbage og frem, filteret er et trin: tilbage giver vis=angreb-i-gaar igen', w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com&vis=angreb-i-gaar', w.location.hash);
  // Tekstfelt
  const felt = d.getElementById('skift-felt');
  d.getElementById('skift-form').hidden = false; felt.focus();
  const foer = w.location.hash;
  tast('ArrowLeft', { altKey: true }); await sov(40);
  tjek('Tilbage og frem, Alt+← gør intet, når fokus står i et tekstfelt', w.location.hash === foer && d.activeElement === felt, w.location.hash);
  emit('historik', 'tilbage'); await sov(40);
  tjek('Tilbage og frem, hændelsen historik gør intet, når fokus står i et tekstfelt', w.location.hash === foer);
  felt.blur(); d.getElementById('skift-form').hidden = true;
  d.getElementById('glem-dialog').setAttribute('open', '');
  tast('ArrowLeft', { altKey: true }); await sov(40);
  tjek('Tilbage og frem, ikke mens en dialog er åben', w.location.hash === foer);
  d.getElementById('glem-dialog').removeAttribute('open');
}
{
  const s = side('#overblik', B2(), 'da', 'macos'), { d, w, emit, tast } = s;
  await sov(60);
  d.querySelector('#overblik-raekker tr').click(); await sov(60);
  tjek('Tilbage og frem, macOS: tooltip med ⌘[ og ⌘]', d.getElementById('hist-tilbage').title === 'Tilbage (⌘[)' && d.getElementById('hist-frem').title === 'Frem (⌘])');
  tjek('Tilbage og frem, Genveje i hjælpen nævner tilbage og frem', d.getElementById('genveje').textContent.includes('⌘[Tilbage') && d.getElementById('genveje').textContent.includes('⌘]Frem'), d.getElementById('genveje').textContent);
  emit('historik', 'tilbage'); await sov(50);
  tjek('Tilbage og frem, Vis → Tilbage (hændelsen historik) går et trin tilbage', synlige(d).join() === 'overblik', synlige(d).join());
  emit('historik', 'frem'); await sov(50);
  tjek('Tilbage og frem, Vis → Frem går et trin frem', synlige(d).join() === 'vaernet', synlige(d).join());
  tast('ArrowLeft', { metaKey: true }); await sov(50);
  tjek('Tilbage og frem, ⌘← går tilbage', synlige(d).join() === 'overblik', synlige(d).join());
  tast('ArrowRight', { metaKey: true }); await sov(50);
  tjek('Tilbage og frem, ⌘→ går frem', synlige(d).join() === 'vaernet', synlige(d).join());
  tast('ArrowLeft', { altKey: true }); await sov(50);
  tjek('Tilbage og frem, Alt+← gør intet på macOS (flytter et ord i et tekstfelt)', synlige(d).join() === 'vaernet', synlige(d).join());
  const felt = d.getElementById('token');
  d.getElementById('visning-opsaetning').hidden = false; felt.focus();
  tast('ArrowLeft', { metaKey: true }); await sov(50);
  tjek('Tilbage og frem, ⌘← i et tekstfelt: ingen navigation, standardhandlingen står', synlige(d).includes('vaernet') && d.activeElement === felt, synlige(d).join());
}
{
  // Sprogskift genindlæser siden: history.state og sessionStorage overlever, rammen i hukommelsen gør ikke.
  const s = side('#indstillinger', B2(), 'da', 'windows', w => {
    w.history.replaceState({ hmTrin: 2 }, '');
    w.sessionStorage.setItem('hm-historik', JSON.stringify({ maks: 4, rul: { 2: 210 } }));
  });
  await sov(60);
  const { d, w } = s;
  tjek('Tilbage og frem, genindlæsning: trinet, frem og rullepositionen er bevaret', w.history.state?.hmTrin === 2 && !d.getElementById('hist-tilbage').disabled && !d.getElementById('hist-frem').disabled && d.getElementById('indhold').scrollTop === 210, `${JSON.stringify(w.history.state)} ${d.getElementById('indhold').scrollTop}`);
  const a = side('#overblik', B2()); await sov(50);
  a.d.querySelector('a[data-visning=indstillinger]').click(); await sov(30);
  a.d.getElementById('indhold').scrollTop = 77;
  a.w.dispatchEvent(new a.w.Event('pagehide'));
  const gemt = JSON.parse(a.w.sessionStorage.getItem('hm-historik'));
  tjek('Tilbage og frem, pagehide gemmer maks og rullepositionerne', gemt.maks === 1 && gemt.rul[1] === 77, JSON.stringify(gemt));
}
{
  // Opsætningen: første trin bliver trin 0; nøglen gemmes uden at lave trin.
  let sat = false;
  const s = side('', B2({ token_status: () => ({ sat }), token_set: () => { sat = true; return null; } })), { d, w } = s;
  await sov(50);
  tjek('Tilbage og frem, opsætning: adressen får en visning, trin 0, ‹ › deaktiveret', w.location.hash === '#overblik' && w.history.state?.hmTrin === 0 && d.getElementById('hist-tilbage').disabled && synlige(d).join() === 'opsaetning', w.location.hash);
  const laengde = w.history.length;
  d.getElementById('token').value = 'x';
  d.getElementById('token-form').dispatchEvent(new w.Event('submit', { cancelable: true, bubbles: true })); await sov(60);
  tjek('Tilbage og frem, opsætning → Overblik er intet trin', synlige(d).join() === 'overblik' && w.history.length === laengde && d.getElementById('hist-tilbage').disabled, synlige(d).join());
  d.querySelector('a[data-visning=overblik]').click(); await sov(30);
  tjek('Tilbage og frem, klik på Overblik ved start er intet trin', w.history.length === laengde && d.getElementById('hist-tilbage').disabled);
}
{
  // Filterchippen ruller til listen, men ikke når trinet genbesøges: dér gendannes rullepositionen.
  const v = medDage([dagAngreb(I_GAAR, 1, [sen(`${I_GAAR}T23:00:00Z`, 'PHP - Code Injection', null)]), dagAngreb(I_FORGAARS, 0, [])]);
  const s = side('#overblik', { ...F3_SVAR(v), notifikation_hent: { niveau: 'standard', roed: true, zoner: {} } }), { d, w } = s;
  await sov(60);
  const rulTil = { n: 0 }, filter = new URLSearchParams({ zone: 'z1', navn: 'wi24rd.com', vis: 'angreb-i-gaar' });
  w.Element.prototype.scrollIntoView = () => { rulTil.n++; };
  w.hmRamme.gaaTil('vaernet', filter); await sov(80);
  const foerste = rulTil.n;
  w.hmRamme.gaaTil('overblik'); await sov(40);
  d.getElementById('hist-tilbage').click(); await sov(80);
  tjek('Tilbage og frem, filteret: ruller til listen første gang, ikke ved tilbage (chippen er der begge gange)', foerste > 0 && rulTil.n === foerste && d.getElementById('vg-filter'), `${foerste} ${rulTil.n}`);
}
{
  // Går man videre, mens rullepositionen gendannes (indholdet er endnu for kort), må den gemte position ikke overskrives.
  const s = side('#overblik', B2()), { d, w } = s;
  await sov(60);
  const hoved = d.getElementById('indhold'), tb = d.getElementById('hist-tilbage'), fr = d.getElementById('hist-frem');
  hoved.scrollTop = 300;
  d.querySelector('a[data-visning=indstillinger]').click(); await sov(30);
  tb.click(); await sov(30);
  hoved.scrollTop = 5;
  fr.click(); await sov(30);
  tb.click(); await sov(30);
  tjek('Tilbage og frem, gendannelsen i gang: den gemte rulleposition overskrives ikke', hoved.scrollTop === 300, String(hoved.scrollTop));
}
// ── Kortets farver ──
{
  const bt = await import(pathToFileURL(path.join(repo, 'design', 'build-tokens.mjs')).href);
  const tokens = bt.laesTokens(path.join(repo, 'design', 'wi24rd-tokens.json')), l = tokens.raw.color['board-light'], m = tokens.raw.color['board-dark'];
  tjek('Kortfarver, lys tavle: land #C9D6E6, kant #A9BBD1, angreb #CC333A (bue og fyld samme farve)', l.land === '#C9D6E6' && l['land-edge'] === '#A9BBD1' && l.attack === '#CC333A' && l['attack-arc'].startsWith('rgba(204,51,58,') && l['attack-fill'].startsWith('rgba(204,51,58,'), JSON.stringify(l));
  tjek('Kortfarver, mørk tavle er uændret', m.land === '#1B2731' && m['land-edge'] === '#2D3D49' && m.attack === '#FF7A6B' && m.net === '#4F6270');
  tjek('Kortfarver, søjlerne bruger samme angrebsfarve', /\.vg-soejle \{ fill: var\(--board-attack\); \}/.test(ui('assets/vagt.css')));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════
// Tekststørrelse, etiketter og flertal, nulstil historik, Edge, kortet og README.
// ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════
const fjernCssKommentarer = k => k.replace(/\/\*[\s\S]*?\*\//g, '');
const appCss = fjernCssKommentarer(ui('assets/app.css')), vagtCss = fjernCssKommentarer(ui('assets/vagt.css')), alCss = appCss + '\n' + vagtCss;
/* [selektor, krop] for hver regel (også inde i @media). */
const regler = k => [...k.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map(m => [m[1].trim(), m[2]]);
// ── P1: tekstskalaen ──
{
  const bt = await import(pathToFileURL(path.join(repo, 'design', 'build-tokens.mjs')).href);
  const tokens = bt.laesTokens(path.join(repo, 'design', 'wi24rd-tokens.json')), d = tokens.component.density, css = ui('assets/tokens.css');
  const n = d.normal;
  tjek('P1 Normal: 15 brødtekst, 14 tabeller, 13 noter, 12 chips, 16 overskrifter, 22 sidetitel, 36 kontroller og rækker, linjehøjde 1,5', n.font.size.lg === 15 && n.font.size.md === 14 && n.font.size.sm === 13 && n.font.size.xs === 12 && n.font.size.xl === 16 && n.font.size.title === 22 && n.control.height === 36 && n.row.height === 36 && n.font['line-height'] === 1.5, JSON.stringify(n));
  const trin = ['xs', 'sm', 'md', 'lg', 'xl', 'title', 'display'], tre = tokens.meta.targets.css.densities;
  tjek('P1 tre tætheder i CSS, hver med hele skalaen i stigende orden og aldrig under 12 px', tre.join() === 'compact,normal,large' && tre.every(t => trin.every((k, i) => d[t].font.size[k] >= 12 && (i === 0 || d[t].font.size[k] >= d[t].font.size[trin[i - 1]]))), tre.map(t => JSON.stringify(d[t].font.size)).join(' '));
  tjek('P1 tokens.css: Normal er standard, [data-density] skifter, comfortable kun til XAML', /:root, \[data-density="normal"\] \{[^}]*--font-size-lg: 15px;/.test(css) && /\[data-density="compact"\] \{[^}]*--font-size-lg: 13px;/.test(css) && /\[data-density="large"\] \{[^}]*--font-size-lg: 17px;/.test(css) && !css.includes('comfortable') && tokens.meta.targets.xaml.density === 'comfortable');
  const faste = alCss.match(/font-size:\s*[\d.]+px|\bfont:[^;}]*?[\d.]+px/g) || [];
  tjek('P1 ingen faste px-skriftstørrelser i app.css/vagt.css (font-size og font:)', faste.length === 0, faste.join(' | '));
  const brugte = [...new Set(alCss.match(/--font-size[\w-]*|--title-size/g))], mangler = brugte.filter(v => !css.includes(v + ':'));
  tjek('P1 alle tekstvariabler i CSS findes i tokens.css (ingen rester af --font-size og --title-size)', mangler.length === 0 && brugte.length >= 7, mangler.join());
  tjek('P1 båndets undertitel er også mindst 12 px', tokens.raw.font.size[tokens.component.shared.band['subtitle-size'].match(/(\d+)\}$/)[1]] >= 12);
  const forbudt = /td\.hv|\.hm-siden small|\.dom p|\.dialog p|\.hjaelp |\.g-|\.guide|\.vg-metode|\.vg-flise b|\.vg-graa b|\.vg-tom b|\.vg-regel-n b|\.vg-l-n|\.felt-mono|\.tabel td|\.titelrad \.under/;
  const muted = regler(alCss).filter(([s, k]) => k.includes('--color-text-muted') && forbudt.test(s)).map(([s]) => s);
  tjek('P1 --color-text-muted bruges ikke til tal, "Hvorfor", guiden eller dialogerne', muted.length === 0, muted.join(' | '));
  tjek('P1 "Hvorfor" står i --color-text', /\.tabel td\.hv \{[^}]*color: var\(--color-text\);/.test(appCss) && /\.hm-siden small \{[^}]*color: var\(--color-text\);/.test(appCss) && /\.dom p \{[^}]*color: var\(--color-text\);/.test(appCss));
  tjek('P1 tabeller og "Hvorfor" følger md; brødteksten følger lg', /\.tabel th, \.tabel td \{[^}]*font-size: var\(--font-size-md\)/.test(appCss) && /body \{[^}]*font-size: var\(--font-size-lg\)/.test(appCss));
  tjek('P1 ved smalt vindue: tabellen ruller i sin ramme, og båndets navn kan forkortes', /\.tabel-rul \{[^}]*overflow-x: auto/.test(appCss) && /\.band-navn b \{[^}]*text-overflow: ellipsis/.test(appCss));
  const konf = JSON.parse(fs.readFileSync(path.join(repo, 'src-tauri', 'tauri.conf.json'), 'utf8'));
  tjek('P1 sidebjælkens rækker vokser med teksten (fast højde lod "Siden sidst" flyde ind over naboerne ved Stor)', /\.nav a \{[^}]*min-height: var\(--sidebar-item-height\)/.test(appCss) && !/\.nav a \{[^}]*[^-]height: var/.test(appCss));
  tjek('P1 <code> kommer aldrig under skalaens mindste trin (.9em af 13 px ville give 11,7 px ved Lille)', !/font-size: *\.\d+em/.test(alCss) && (alCss.match(/font-size: max\(\.9em, var\(--font-size-xs\)\)/g) || []).length === 2);
  tjek('P1 ingen WebView-zoom', !JSON.stringify(konf).includes('zoomHotkeysEnabled'));
}
// ── P1: Indstillinger → Tekststørrelse og genvejene ──
{
  const s = side('#indstillinger', B2()), { d, w, tast, emit } = s, html = d.documentElement;
  await sov(50);
  const v = x => d.querySelector(`input[name=tekst][value=${x}]`);
  tjek('P1 standard: data-density=normal, Normal valgt, segmentet hedder Lille/Normal/Stor', html.getAttribute('data-density') === 'normal' && v('normal').checked && [...d.querySelectorAll('#tekst label')].map(l => l.textContent).join() === 'Lille,Normal,Stor' && d.getElementById('tekst').getAttribute('role') === 'radiogroup', html.outerHTML.slice(0, 90));
  v('large').click();
  tjek('P1 Stor sætter data-density=large og gemmes i hm-tekst', html.getAttribute('data-density') === 'large' && w.localStorage.getItem('hm-tekst') === 'large');
  v('compact').click();
  tjek('P1 Lille sætter data-density=compact', html.getAttribute('data-density') === 'compact' && w.localStorage.getItem('hm-tekst') === 'compact');
  const husket = side('#indstillinger', B2(), 'da', 'windows', x => x.localStorage.setItem('hm-tekst', 'large'));
  const straks = husket.d.documentElement.getAttribute('data-density');
  await sov(40);
  tjek('P1 valget gælder ved næste start (før siden tegnes) og står i segmentet', straks === 'large' && husket.d.querySelector('input[name=tekst][value=large]').checked);
  const ugyldig = side('#indstillinger', B2(), 'da', 'windows', x => x.localStorage.setItem('hm-tekst', 'kaempe'));
  tjek('P1 en ugyldig gemt værdi giver Normal', ugyldig.d.documentElement.getAttribute('data-density') === 'normal');
  tjek('P1 tema.js står uden defer før de andre scripts (ingen blink)', /<script src="assets\/tema\.js"><\/script>\s*<script src="assets\/sprog\.js" defer>/.test(ui('index.html')));
  v('normal').click();
  const spor = [], tryk = (key, x) => { const e = new w.KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...x }); d.dispatchEvent(e); spor.push(html.getAttribute('data-density')); return e; };
  const e1 = tryk('+', { ctrlKey: true });
  tryk('+', { ctrlKey: true }); tryk('-', { ctrlKey: true }); tryk('-', { ctrlKey: true }); tryk('-', { ctrlKey: true }); tryk('0', { ctrlKey: true }); tryk('=', { ctrlKey: true });
  tjek('P1 Ctrl++ Ctrl+− Ctrl+0 og Ctrl+= skifter trin, stopper i enderne og forhindrer standardhandlingen', spor.join() === 'large,large,normal,compact,compact,normal,large' && e1.defaultPrevented, spor.join());
  tjek('P1 tastaturet gemmer valget, og segmentet følger med', w.localStorage.getItem('hm-tekst') === 'large' && v('large').checked);
  tast('0', { ctrlKey: true, altKey: true }); tast('0');
  tjek('P1 Ctrl+Alt+0 og et rent 0 gør intet', html.getAttribute('data-density') === 'large');
  tjek('P1 Genveje i hjælpen nævner tekststørrelsen', ['Ctrl++Større tekst', 'Ctrl+−Mindre tekst', 'Ctrl+0Normal tekst'].every(x => d.getElementById('genveje').textContent.includes(x)), d.getElementById('genveje').textContent);
  emit('tekst', 'normal');
  const a = html.getAttribute('data-density');
  emit('tekst', 'mindre');
  const b = html.getAttribute('data-density');
  emit('tekst', 'stoerre');
  tjek('P1 hændelsen tekst (programmenuen) skifter trin', a === 'normal' && b === 'compact' && html.getAttribute('data-density') === 'normal', `${a} ${b}`);
  const m = side('#overblik', B2(), 'da', 'macos');
  await sov(40);
  m.tast('+', { metaKey: true });
  const ma = m.d.documentElement.getAttribute('data-density');
  m.tast('0', { ctrlKey: true });
  tjek('P1 macOS: ⌘+ skifter (⌘ også i Genveje), Ctrl+0 gør intet', ma === 'large' && m.d.documentElement.getAttribute('data-density') === 'large' && m.d.getElementById('genveje').textContent.includes('⌘+Større tekst'), ma);
  m.tast('-', { metaKey: true }); m.tast('-', { metaKey: true });
  const mb = m.d.documentElement.getAttribute('data-density');
  m.tast('0', { metaKey: true });
  tjek('P1 macOS: ⌘− går ned til Lille, ⌘0 tilbage til Normal', mb === 'compact' && m.d.documentElement.getAttribute('data-density') === 'normal' && m.w.localStorage.getItem('hm-tekst') === 'normal', mb);
  const rs = fs.readFileSync(path.join(repo, 'src-tauri', 'src', 'programmenu.rs'), 'utf8');
  tjek('P1 programmenuen har Større, Mindre og Normal tekst', ['"Større tekst"', '"Mindre tekst"', '"Normal tekst"', 'punkt(TEKST_STOERRE', 'punkt(TEKST_MINDRE', 'punkt(TEKST_NORMAL'].every(x => rs.includes(x)));
}
// ── P2: etiketterne og flertallet ──
{
  const fra = sp => { const s = side('#indstillinger', B2({ notifikation_hent: { niveau: 'standard', roed: true, zoner: { z2: 'alt', z3: 'roed' } } }), sp); return s; };
  const da = fra('da'), en = fra('en');
  await sov(50);
  const valg = s => [...s.d.querySelector('#pr-domaene-liste select').options];
  tjek('P2 vælgeren pr. domæne på dansk', valg(da).map(o => o.textContent).join('|') === 'Følg standard|Alvorligt og hold øje|Kun alvorligt|Aldrig', valg(da).map(o => o.textContent).join('|'));
  tjek('P2 vælgeren pr. domæne på engelsk', valg(en).map(o => o.textContent).join('|') === 'Follow default|Serious and watch|Serious only|Never', valg(en).map(o => o.textContent).join('|'));
  tjek('P2 Rusts værdier er uændrede (standard, alt, roed, fra)', valg(da).map(o => o.value).join() === 'standard,alt,roed,fra' && [...da.d.querySelectorAll('#pr-domaene-liste select')].map(x => x.value).join() === 'roed,standard,alt', [...da.d.querySelectorAll('#pr-domaene-liste select')].map(x => x.value).join());
  for (const [navn, s] of [['da', da], ['en', en]]) {
    const { w, d } = s, T = w.hmSprog.T;
    const nogler = Object.keys(T), fl = nogler.filter(k => k.endsWith('__fl')), erklaeret = new Set(fl.map(k => k.slice(0, -4)));
    const billede = () => [...d.querySelectorAll('[data-t]')].map(e => [e.getAttribute('data-t'), e.textContent]);
    w.hmSprog.saetDomaene('3 domæner', false);
    const ental = billede();
    w.hmSprog.saetDomaene('3 domæner', true);
    const flere = billede();
    const forskel = ental.filter(([k, t], i) => t !== flere[i][1]).map(([k]) => k);
    tjek(`P2 ${navn}: flertalsnøglerne er netop lead, m1 og m2, og hver har en entalsnøgle`, fl.join() === 'lead__fl,m1__fl,m2__fl' && [...erklaeret].every(k => k in T), fl.join());
    tjek(`P2 ${navn}: med flertal sand har hver data-t-node samme tekst, medmindre den er erklæret som flertal (${ental.length} noder)`, ental.length > 100 && forskel.every(k => erklaeret.has(k)) && [...erklaeret].every(k => forskel.includes(k)), forskel.join());
    tjek(`P2 ${navn}: "Glem nøglen…" er stadig en knap med sin egen tekst`, d.getElementById('glem').textContent === T.glem && T.glem.length < 20 && d.getElementById('glem-p').textContent === T.glemP, d.getElementById('glem').textContent);
  }
  tjek('P2 sprog.js: flertal ved suffiks, ikke ved "P"', !/k \+ 'P'/.test(ui('assets/sprog.js')));
  tjek('P2 rækken: kontrollerne ombrydes, etiketten fylder mindst 40 %', /\.raekke \{[^}]*flex-wrap: wrap/.test(appCss) && /\.raekke \.etik \{[^}]*flex: 1 1 40%;[^}]*min-width: 40%/.test(appCss) && /\.raekke-knapper \{[^}]*flex-wrap: wrap/.test(appCss) && da.d.getElementById('glem').parentElement.className === 'raekke-knapper' && da.d.getElementById('skift-noegle').parentElement.className === 'raekke-knapper');
}
// ── P3: Nulstil historik ──
const dagFor = n => new Date(urNu() - n * 864e5).toISOString().slice(0, 10);
{
  const tab = (fra, graense, dage) => ({ fra, graense, dage }), TOM = { sikkerhed: null, trafik: null };
  let data = { bytes: 2412000, foerste: '2026-10-04', sidste: '2026-11-05', domaener: 2, nulstil: { sikkerhed: tab('2026-10-04', '2026-10-06', 2), trafik: null } };
  const s = side('#indstillinger', B2({ data_status: () => data, nulstil_historik: null })), { d, kald, w } = s;
  await sov(50);
  const knap = d.getElementById('nulstil'), dialog = d.getElementById('nulstil-dialog'), p = d.getElementById('nulstil-p');
  const dataKort = d.querySelectorAll('.indst > .kort')[4], rk = [...dataKort.querySelectorAll('.raekke')].at(-1);
  tjek('Nulstil: rækken siger, hvad der er væk for altid', rk.contains(knap) && knap.classList.contains('fare') && knap.textContent === 'Nulstil historik…' && rk.textContent.includes('Sletter alle tal på din computer og henter dem forfra. Cloudflare har kun sikkerhedshændelser for den seneste måned og trafiktal for det seneste år — alt ældre er væk for altid.') && knap.getAttribute('aria-describedby') === 'nulstil-note', rk.textContent);
  knap.click(); await sov(10);
  tjek('Nulstil: dialogen siger det med konkrete datoer, og intet slettes endnu', dialog.hasAttribute('open') && d.getElementById('nulstil-h').textContent === 'Nulstil historikken?' && p.textContent === 'Du har sikkerhedstal fra 4. oktober. Efter nulstilling kan kun tal fra 6. oktober hentes igen. 2 dage forsvinder for altid.' && !kald.some(k => k[0] === 'nulstil_historik'), p.textContent);
  tjek('Nulstil: knappen er rød og hedder "Slet og hent forfra"; Annullér har fokus', d.getElementById('nulstil-nej').textContent === 'Annullér' && d.getElementById('nulstil-ja').textContent === 'Slet og hent forfra' && d.getElementById('nulstil-ja').classList.contains('fare') && d.activeElement === d.getElementById('nulstil-nej'), d.activeElement.id);
  d.getElementById('nulstil-nej').click(); await sov(10);
  tjek('P3 Annullér lukker og kalder intet', !dialog.hasAttribute('open') && !kald.some(k => k[0] === 'nulstil_historik'));
  data = { ...data, nulstil: { sikkerhed: tab('2026-10-05', '2026-10-06', 1), trafik: tab('2025-11-01', '2025-11-06', 5) } };
  knap.click(); await sov(10);
  tjek('Nulstil: ental, og trafikken har sin egen sætning med året', p.textContent === 'Du har sikkerhedstal fra 5. oktober. Efter nulstilling kan kun tal fra 6. oktober hentes igen. 1 dag forsvinder for altid. Du har trafiktal fra 1. november 2025. Efter nulstilling kan kun tal fra 6. november 2025 hentes igen. 5 dage forsvinder for altid.', p.textContent);
  d.getElementById('nulstil-nej').click();
  data = { ...data, nulstil: { sikkerhed: tab('2026-10-06', '2026-10-06', 0), trafik: tab('2026-03-05', '2025-11-06', 0) } };
  knap.click(); await sov(10);
  tjek('P3 intet ældre end grænsen: "Alt kan hentes igen fra Cloudflare."', p.textContent === 'Alt kan hentes igen fra Cloudflare.', p.textContent);
  d.getElementById('nulstil-nej').click();
  data = { bytes: 900, foerste: null, sidste: null, domaener: 0, nulstil: TOM };
  knap.click(); await sov(10);
  tjek('P3 ingen historik: intet tab', p.textContent === 'Alt kan hentes igen fra Cloudflare.', p.textContent);
  const foerOverblik = kald.filter(k => k[0] === 'overblik').length;
  d.getElementById('nulstil-ja').click(); await sov(30);
  tjek('P3 Nulstil kalder nulstil_historik én gang, lukker og melder det', !dialog.hasAttribute('open') && kald.filter(k => k[0] === 'nulstil_historik').length === 1 && d.getElementById('indst-status').textContent === 'Historikken er nulstillet. Programmet henter tallene forfra.' && d.getElementById('indst-status').classList.contains('ok'), d.getElementById('indst-status').textContent);
  tjek('P3 overblikket og Data tegnes igen', kald.filter(k => k[0] === 'overblik').length === foerOverblik + 1 && d.getElementById('data-historik').textContent === 'Ingen endnu', d.getElementById('data-historik').textContent);
  knap.click(); await sov(10);
  d.querySelector('a[data-visning=overblik]').click(); await sov(30);
  tjek('P3 et skift af visning lukker dialogen', !dialog.hasAttribute('open'));
}
{
  const fejl = side('#indstillinger', B2({ data_status: () => { throw { kode: 'lager' }; }, nulstil_historik: () => { throw { kode: 'lager' }; } }));
  await sov(50);
  const { d } = fejl;
  d.getElementById('nulstil').click(); await sov(10);
  tjek('P3 kan ældste dag ikke læses: tabet nævnes for en sikkerheds skyld', d.getElementById('nulstil-p').textContent === 'Det, der er ældre end det, Cloudflare stadig har, forsvinder for altid.', d.getElementById('nulstil-p').textContent);
  d.getElementById('nulstil-ja').click(); await sov(20);
  tjek('P3 en fejl fra Rust vises som fejl', d.getElementById('indst-status').className === 'r6-status fejl' && d.getElementById('indst-status').textContent === 'Den lokale database kunne ikke bruges.', d.getElementById('indst-status').textContent);
  const u = side('#indstillinger', B2({ token_status: { sat: false } }));
  await sov(40);
  tjek('P3 uden API Token er knappen slået fra (intet kan hentes igen)', u.d.getElementById('nulstil').disabled);
  const e = side('#indstillinger', B2({ data_status: { bytes: 1, foerste: '2026-10-04', sidste: '2026-11-05', domaener: 1, nulstil: { sikkerhed: { fra: '2026-10-05', graense: '2026-10-06', dage: 1 }, trafik: null } }, nulstil_historik: null }), 'en');
  await sov(40);
  e.d.getElementById('nulstil').click(); await sov(10);
  tjek('Nulstil engelsk: knap, overskrift, ental og tab', e.d.getElementById('nulstil').textContent === 'Reset history…' && e.d.getElementById('nulstil-h').textContent === 'Reset the history?' && e.d.getElementById('nulstil-p').textContent === 'You have security figures from 5 October. After a reset, only figures from 6 October can be fetched again. 1 day is lost for good.' && e.d.getElementById('nulstil-ja').textContent === 'Delete and fetch again', e.d.getElementById('nulstil-p').textContent);
  tjek('P3 sikkerhedsporten: allow-nulstil-historik i capabilities, nulstil_historik i build.rs og i handleren', tilladt('nulstil_historik') && /"nulstil_historik",/.test(fs.readFileSync(path.join(repo, 'src-tauri', 'build.rs'), 'utf8')) && /generate_handler!\[[^\]]*\bnulstil_historik\b/.test(fs.readFileSync(path.join(repo, 'src-tauri', 'src', 'lib.rs'), 'utf8')));
  const lib = fs.readFileSync(path.join(repo, 'src-tauri', 'src', 'lib.rs'), 'utf8'), krop = lib.slice(lib.indexOf('async fn nulstil_historik'), lib.indexOf('/// Start ved login'));
  tjek('P3 Rust: stop, slet, vent på løkken, start igen (i den rækkefølge)', ['koe.stop()', '.nulstil_historik()', 'koe.slut().await', 'start_koe(&app)'].map(x => krop.indexOf(x)).every((v, i, a) => v >= 0 && (i === 0 || v > a[i - 1])), krop);
}
// ── P4: Edge ──
{
  const s = side('#vaernet?zone=z1&navn=wi24rd.com', { ...MED_SIDEN }), e = side('#vaernet?zone=z1&navn=wi24rd.com', { ...MED_SIDEN }, 'en');
  await sov(100);
  const blok = x => x.d.querySelector('#vg-dage');
  tjek('P4 diagrammet: "Edge · 30 dage" og undertitlen', blok(s).querySelector('h2').textContent === 'Edge · 30 dage' && blok(s).querySelector('.vg-intro').textContent === 'Første forsvarslinje: Cloudflares edge, som al trafik møder, før den når din server. Afviste forespørgsler pr. dag (UTC).', blok(s).textContent.slice(0, 160));
  tjek('P4 engelsk: "Edge · 30 days" og undertitlen', blok(e).querySelector('h2').textContent === 'Edge · 30 days' && blok(e).querySelector('.vg-intro').textContent === 'First line of defence: Cloudflare’s edge, which all traffic meets before it reaches your server. Refused requests per day (UTC).', blok(e).textContent.slice(0, 160));
  tjek('P4 kortets beskrivelse nævner datacentret i Cloudflares edge', s.d.getElementById('vagt').textContent.includes('til det datacenter i Cloudflares edge, der stoppede det.') && e.d.getElementById('vagt').textContent.includes('to the data centre in Cloudflare’s edge that stopped it.'));
  const hj = s.d.querySelector('.hjaelp-del[data-hjaelp=vaernet]'), hje = e.d.querySelector('.hjaelp-del[data-hjaelp=vaernet]');
  const edge = hj.querySelector('[data-t=edgeP]').textContent, edgeE = hje.querySelector('[data-t=edgeP]').textContent;
  tjek('P4 "?": afsnittet "Edge – første forsvarslinje" med anycast og forbeholdene', hj.querySelector('[data-t=edgeH]').textContent === 'Edge – første forsvarslinje' && ['netværk af datacentre', 'anycast', 'nærmest angriberen', 'orange sky', 'serverens egen IP-adresse', 'kun tager imod trafik fra Cloudflare'].every(x => edge.includes(x)), edge);
  tjek('P4 "?" på engelsk', hje.querySelector('[data-t=edgeH]').textContent === 'Edge – first line of defence' && ['network of data centres', 'anycast', 'nearest the attacker', 'orange cloud', 'own IP address', 'only accepts traffic from Cloudflare'].every(x => edgeE.includes(x)), edgeE);
  const filer = ['ui/assets/sprog.js', 'ui/index.html', 'README.md', 'README.da.md', ...fs.readdirSync(path.join(repo, 'ui', 'assets')).filter(f => /^visning-.*\.js$/.test(f)).map(f => 'ui/assets/' + f)];
  const fund = filer.filter(f => /\b(ved )?porten\b|\bkanten\b|\bthe gate\b/i.test(fs.readFileSync(path.join(repo, f), 'utf8')));
  tjek(`P4 ingen "porten", "ved porten", "kanten" eller "the gate" (${filer.length} filer)`, fund.length === 0 && filer.length === 13, fund.join());
}
// ── P5: kortet ──
{
  const s = side('#vaernet?zone=z1&navn=wi24rd.com', { ...MED_SIDEN }), e = side('#vaernet?zone=z1&navn=wi24rd.com', { ...MED_SIDEN }, 'en');
  await sov(100);
  const leg = x => [...x.d.querySelectorAll('.vg-legende li')].map(l => l.textContent);
  tjek('P5 legenden forklarer ◆ i en ring', leg(s).at(-1) === '◆ i en ring = stoppet i samme land, som angrebet kom fra' && leg(s).length === 4, leg(s).join(' | '));
  tjek('P5 legenden på engelsk', leg(e).at(-1) === '◆ inside a ring = stopped in the country the attack came from', leg(e).join(' | '));
  const svg = s.d.querySelector('svg.vg-kort');
  tjek('P5 hele kortet: ingen <title>, men role=group (mærkerne kan åbnes) og aria-label med samme tekst', ![...svg.children].some(c => c.tagName.toLowerCase() === 'title') && svg.getAttribute('role') === 'group' && svg.getAttribute('aria-label').startsWith('Verdenskort over afviste angreb de seneste 30 dage') && !svg.hasAttribute('aria-labelledby'), svg.outerHTML.slice(0, 200));
  tjek('P5 beskrivelsen står stadig i <desc>, og aria-describedby peger på den', svg.getAttribute('aria-describedby') === 'vg-kort-d' && svg.querySelector('desc#vg-kort-d').textContent.startsWith('Flest kom fra'));
  tjek('P5 lande og datacentre med tal beholder deres <title>', /: 80 afvist$/.test(svg.querySelector('circle.vg-land title')?.textContent) && /: 80 afvist$/.test(svg.querySelector('path.vg-colo title')?.textContent), svg.querySelector('circle.vg-land')?.outerHTML);
}

// ── ──
const REACT = 'React - RCE - CVE:CVE-2025-55182';
const dykDage = Array.from({ length: 30 }, (_, i) => ({ dag: new Date(Date.UTC(2026, 9, 5 + i)).toISOString().slice(0, 10), n: i % 4 && i !== 2 ? i : 0, hentet: i !== 2 }));
const dykSvar = x => Object.assign({ rettet_mod: [], trafik_serie: [],
  trafik: { i_alt: 0, dage: [] }, kun_i_dag: null, periode: { fra: '2026-10-05', til: '2026-11-03', dage: 30 }, i_gaar: '2026-11-03', i_dag: '2026-11-04', sum: 198, noegletal: { i_gaar: 6, i_dag_hidtil: 2, hele7: 57, hele30: 198, alt: 240 },
  foerst: '2026-10-05', sidst: '2026-11-03', dage: dykDage, fordeling: { blokeret: 190, udfordret: 5, labyrint: 3, andet: 0, faelder: 21 },
  top: { robot: [], status: [], regel: [{ v: REACT, n: 150 }, { v: 'botFight', n: 48 }], land: ['PL', 'US', 'DE', 'NL', 'SG', 'FR', 'T1', 'XX'].map((v, i) => ({ v, n: 80 - i * 9 })), netvaerk: [{ v: 'AT&T Services, Inc.', n: 60 }, { v: '<b>Fed</b> net', n: 2 }], sti: [{ v: '/wp-config%2ephp', n: 40 }, { v: '/', n: 20 }], colo: [{ v: 'WAW', n: 67 }, { v: 'EWR', n: 40 }], metode: [{ v: 'POST', n: 190 }, { v: 'GET', n: 8 }], handling: [{ v: 'blokeret', n: 190 }, { v: 'udfordret', n: 5 }, { v: 'labyrint', n: 3 }] },
  antal: { regel: 2, land: 8, netvaerk: 2, sti: 2, colo: 2, metode: 2, handling: 3 }, buer: [{ land: 'PL', colo: 'EWR', n: 40 }, { land: 'US', colo: 'WAW', n: 8 }],
  regel: null, domaener: [{ zone: 'z2', navn: 'xyz.wi24rd.com', n: 92 }, { zone: 'z3', navn: 'abe.wi24rd.com', n: 8 }],
  enkelte: [{ t: '2026-11-03T21:00:00Z', regel: REACT, cve: 'CVE-2025-55182', land: 'PL', sti: '[hemmeligheder]', colo: 'WAW', metode: 'POST' }], slags: { angreb: 150, rutine: 48 }, loft: false,
  regler: [{ navn: REACT, produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', art: 'angreb', h30: 150, h7: 0, idag: 0, faelder30: 0, foerst: '2026-11-01', sidst: '2026-11-03', stier: [{ sti: '/api', n: 100 }], lande: [{ kode: 'PL', n: 120 }] }, { navn: 'botFight', produkt: '', type: '', cve: null, art: 'rutine', h30: 48, h7: 0, idag: 0, faelder30: 0, foerst: '2026-11-02', sidst: '2026-11-02', stier: [], lande: [] }],
}, x);
const REGEL_HOVED = { navn: REACT, produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', art: 'angreb' };
const DYK = x => ({ ...MED_TOKEN, kortdata: KORT, koe_status: null, valg_hent: ['z1', 'z2'], siden_sidst: { vis: false, nye: [], raekker: [] }, dyk: a => dykSvar({ regel: a.filtre.some(f => f.dim === 'regel') ? REGEL_HOVED : null, regler: a.filtre.some(f => f.dim === 'regel') ? [] : dykSvar().regler }), ...x });
const landDage = Array.from({ length: 30 }, (_, i) => ({ dag: new Date(Date.UTC(2026, 9, 4 + i)).toISOString().slice(0, 10), mit: i === 3 ? null : i % 5 ? 4 : 0, mit_indeks: i === 3 ? null : i % 5 ? 125 : 0, verden_indeks: 90 + i }));
const LAND_UD = {
  kode: 'DK', i_gaar: '2026-11-02', dage: landDage, andel: [{ dag: '2026-11-02', vaerdi: 0.4 }, { dag: '2026-11-01', vaerdi: null }], ord: null,
  sammen7: { land: [{ navn: 'US', vaerdi: 41 }, { navn: 'XX', vaerdi: 3 }], rest: 56, spurgt: true, fra: '2026-10-27', til: '2026-11-02', mine: [{ navn: 'PL', vaerdi: 60, n: 60, saerligt: true }, { navn: 'US', vaerdi: 40, n: 40, saerligt: false }], mine_i_alt: 100 },
  sammen28: { land: [{ navn: 'DE', vaerdi: 30 }, { navn: 'US', vaerdi: 20 }], rest: 50, spurgt: true, fra: '2026-10-06', til: '2026-11-02', mine: [{ navn: 'US', vaerdi: 75, n: 300, saerligt: true }, { navn: 'XX', vaerdi: 25, n: 100, saerligt: false }], mine_i_alt: 400 },
  serie_fejl: [], udfald_verden: [], udfald_verden_hentet: false,
  udfald: [{ start: '2026-10-30T04:00:00Z', slut: '2026-10-30T06:30:00Z', minutter: 150, aarsag: 'CABLE_CUT', slags: 'NETWORK', netvaerk: ['Fjord Net', 'Bakke Net'] }, { start: '2026-10-20T01:00:00Z', slut: null, minutter: null, aarsag: 'NOGET_NYT', slags: '', netvaerk: [] }], udfald_hentet: true,
  maal: [{ navn: 'US', vaerdi: 31 }, { navn: 'DK', vaerdi: 0.4 }], kilde: [{ navn: 'CN', vaerdi: 13 }, { navn: 'T1', vaerdi: 2 }], produkt: [{ navn: 'DDOS', vaerdi: 54 }, { navn: 'WAF', vaerdi: 33 }],
  ai: [{ navn: 'Training', vaerdi: 40 }, { navn: 'Noget nyt', vaerdi: 10 }], ai_navne: [{ navn: 'GPTBot', vaerdi: 30 }], bot: 38.5, kvante: 42.5, http_indeks: 104, verden_dag: '2026-11-02', hentet: NU_ISO, fejl: null,
};
/* land_saet husker landet, som Rust gør; land_visning svarer med det land, der spørges om. */
const LANDV = x => {
  const st = { land: 'DK', forslag: null, ...(x && x.land_hent) };
  return { ...MED_TOKEN, kortdata: KORT, valg_hent: ['z1'], koe_status: null, koe_prioritet: null, land_saet: a => { st.land = a.kode; return null; }, land_visning: a => ({ ...LAND_UD, kode: a.kode }), radar_opdater: null, aabn_radar: null, ...x, land_hent: () => ({ ...st }) };
};
/* Et domæne, Cloudflare ikke kunne nå, et med et trafikspring og et rødt, hvor 52x ikke sænker status. */
const B2B_SERVER = { dag: '2026-11-02', fejl: 40, svar: 120, procent: 33 };
const B2B_TRAFIK = [{ dag: '2026-10-27', n: 350000, median: 2000, stoppet: 67 }, { dag: '2026-10-28', n: 1000, median: 2000, stoppet: 1 }];
const B2B_OVERBLIK = [
  raekke('r1', 'roed.wi24rd.com', 'roed', { angreb_i_gaar: 30, angreb_snit: 2, angreb_i_gaar_top: [{ navn: 'Wordpress', n: 30 }], angreb_fordeling: { blokeret: 30, udfordret: 0, labyrint: 0, andet: 0 }, server: B2B_SERVER, sidst_hentet: NU_ISO }),
  raekke('s1', 'nede.wi24rd.com', 'gul', { server: B2B_SERVER, forstyrrelse: { land: 'DK', netvaerk: 'Fjord Net' }, sidst_hentet: NU_ISO }),
  raekke('t1', 'trafik.wi24rd.com', 'gul', { trafik: { dag: '2026-11-02', n: 350000, median: 2000, stoppet: 67 }, angreb_status: 'groen', sidst_hentet: NU_ISO }),
];
const B2C_TRAFIK = { dag: '2026-11-02', til: '2026-11-02', i_alt: 410000, median: 1500, stoppet: 67, lande: [{ v: 'US', n: 378000, angreb: 60 }, { v: 'DE', n: 32886, angreb: 0 }], lande_i_alt: 410000, status: [{ v: '522', n: 345000 }, { v: '200', n: 65886 }], status_i_alt: 410000, sikkerhed: true, trusler: 0, cachet: 0, server_fejl: 345000, robotter: [{ v: 'Googlebot', n: 300000 }], ekstra: true };
const B2C_TEK = { linjer: [{ tek: 'wordpress', angreb30: 40, forsoeg30: 312, foerst: '2026-10-04', sidst: '2026-11-02', valg: 'ved_ikke', besked: false, forslag: true }, { tek: 'react', angreb30: 9, forsoeg30: 0, foerst: '2026-10-05', sidst: '2026-11-01', valg: 'bruger', besked: true, forslag: false }], generiske: [{ id: 'hemmeligheder', forsoeg30: 4 }] };
const B2C_VERDEN = [{ start: '2026-10-30T04:00:00Z', slut: '2026-10-30T06:00:00Z', minutter: 120, aarsag: 'CABLE_CUT', slags: 'NATIONWIDE', netvaerk: ['Fjord Net'], lande: ['SE'] }];
const B2C_HULLER = { linjer: [{ noegle: '2026-09-02_2026-10-04', fra: '2026-09-02', til: '2026-10-04', tabt: 9 }], dage: ['2026-09-03'], aeldste: '2026-08-01' };
const w0 = x => x.w;
const tidszone = z => w2 => w2.eval(`Intl.DateTimeFormat.prototype.resolvedOptions = function () { return { timeZone: ${JSON.stringify(z)} }; };`);
const dykQ = (x = {}) => new URLSearchParams({ zone: 'z1', navn: 'wi24rd.com', ...x }).toString();
/* Svarene fra robotter, login og login_stier. */
const ROBOT_UD = {
  periode: { fra: '2026-10-28', til: '2026-11-03', dage: 7 },
  crawlere: {
    genereret: NU_ISO, iAlt: { bots7: 3482, bots30: 9000, alle7: 47000, botNavne: 7, falske7: 186 }, kategorier: [], dage: [], filer: {}, status: {},
    bots: [{ navn: 'Googlebot', firma: 'Google', kat: 'Search Engine Crawler', n7: 1411, sider: [], status: {}, filer: {} }, { navn: 'Verificeret (AI Crawler)', firma: '', kat: 'AI Crawler', n7: 231, sider: [], status: {}, filer: {} }, { navn: 'AI-assistent i en browser', firma: '', kat: 'AI Assistant', n7: 118, sider: [], status: {}, filer: {} }],
    falske: [{ navn: 'Googlebot', n: 186 }], uverificerede: [{ navn: 'FooBot', n: 12 }],
  },
  grupper: [{ id: 'ai-traening', n: 412 }, { id: 'ai-soegning', n: 286 }, { id: 'soegemaskiner', n: 1904 }, { id: 'sociale', n: 518 }, { id: 'seo', n: 241 }, { id: 'overvaagning', n: 121 }, { id: 'ukendt', n: 198 }],
  sider: [{ v: '/', n: 620 }, { v: '/priser/', n: 301 }], antal_sider: 2,
  stoppet: { sum: 142, labyrint: 21, robotter: [{ v: 'GPTBot', n: 118 }, { v: 'Ukendt bot', n: 24 }], labyrint_robotter: [{ v: 'ClaudeBot', n: 21 }], mangler_dage: 0, ikke_tilgaengelig: [] },
};
const loginDage = Array.from({ length: 30 }, (_, i) => ({ dag: new Date(Date.UTC(2026, 9, 5 + i)).toISOString().slice(0, 10), edge: i % 3 ? 4 : 0, ikke_fundet: i % 5 ? 0 : 2, server: i === 29 ? 3 : 0, hentet: true }));
const LOGIN_UD = { laek: [], laek_andet: [], laek_dage: 30,
  periode: { fra: '2026-10-05', til: '2026-11-03', dage: 30 }, stier: ['/login', '/wp-login.php'],
  tal: { edge: 161, ikke_fundet: 18, server: 51, afvist: 42, svar: 9, andet: 0 }, dage: loginDage,
  raekker: [{ sti: '/wp-login.php', metoder: ['POST'], tal: { edge: 118, ikke_fundet: 18, server: 30, afvist: 23, svar: 7, andet: 0 } }, { sti: '/login', metoder: ['POST', 'GET'], tal: { edge: 0, ikke_fundet: 0, server: 21, afvist: 19, svar: 2, andet: 0 } }],
  metoder_edge: [{ v: 'POST', n: 161 }], metoder_server: [{ v: 'POST', n: 51 }], lande_edge: [{ v: 'PL', n: 71 }], lande_server: [{ v: 'US', n: 49 }], net_edge: [{ v: 'Eksempel Hosting', n: 64 }], net_server: [{ v: 'AS64500', n: 41 }],
  mangler_dage: 0, ikke_tilgaengelig: [],
};
const LOGIN_STIER = { stier: [{ sti: '/login', til: true, start: true }, { sti: '/admin/', til: false, start: true }, { sti: '/min-side', til: true, start: false }], forslag: ['/api/auth/signin'] };
const VIS = { robotter: ROBOT_UD, login: LOGIN_UD, login_stier: LOGIN_STIER, login_sti_saet: null };
const ROBOT_DYK = dykSvar({ regler: [], top: { regel: [], land: [], netvaerk: [], sti: [{ v: '/', n: 40 }], colo: [], metode: [], handling: [], robot: [{ v: 'GPTBot', n: 30 }, { v: 'Googlebot', n: 10 }], status: [{ v: '200', n: 40 }] }, antal: { regel: 0, land: 0, netvaerk: 0, sti: 1, colo: 0, metode: 0, handling: 0, robot: 2, status: 1 }, fordeling: { blokeret: 0, udfordret: 0, labyrint: 0, andet: 0, faelder: 0 }, buer: [], enkelte: [], domaener: [], slags: { angreb: 0, rutine: 0 } });
const sidsteDyk = s => s.kald.filter(k => k[0] === 'dyk').pop()[1];

// Adresseprøven: værdierne går uændret gennem gaaTil og laesHash.
{
  const s = side('#overblik', DYK());
  await sov(40);
  const R = s.w.hmRamme, vaerdier = ['/wp-config%2ephp', '/sid%20file.php', 'AT&T Services, Inc.', 'a&b=c', 'C++ regel', '/%ff', '100% blokeret', '/æøå', 'min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)', REACT];
  const ud = [];
  for (const v of vaerdier) { R.gaaTil('dyk', new URLSearchParams({ zone: 'z1', navn: 'a.wi24rd.com', sti: v })); await sov(15); ud.push(R.tilstand.params.get('sti')); }
  tjek('D adresseprøven: 10 af 10 værdier uændrede gennem adressen', JSON.stringify(ud) === JSON.stringify(vaerdier), JSON.stringify(ud));
  tjek('D adressen: stien når Rust uændret', sidsteDyk(s).filtre[0].vaerdi === REACT && s.w.location.hash.startsWith('#dyk?'));
  const k = s.kald.filter(x => x[0] === 'dyk').find(x => x[1].filtre[0].vaerdi === '/wp-config%2ephp');
  tjek('D adressen: /wp-config%2ephp bliver ikke til /wp-config.php', Boolean(k));
}

// Regel-dyk
{
  const s = side('#dyk?' + dykQ({ regel: REACT }), DYK());
  await sov(90);
  const { d, w, kald } = s, q = sel => d.querySelector('#visning-dyk ' + sel), alle = sel => [...d.querySelectorAll('#visning-dyk ' + sel)];
  tjek('D regel: dyk kaldes med omfang, filter og 30 dage', JSON.stringify(sidsteDyk(s)) === JSON.stringify({ omfang: { zone: 'z1' }, filtre: [{ dim: 'regel', vaerdi: REACT }], periode: 'd30', slags: null }), JSON.stringify(sidsteDyk(s)));
  tjek('D regel: overskrift i klarsprog med domænet, og titlen følger med', d.getElementById('dyk-h').textContent === 'Angreb på wi24rd.com med reglen Et kendt hul i React' && d.getElementById('dyk-under').textContent === '' && d.title.startsWith('Et kendt hul i React · wi24rd.com'), d.title);
  tjek('D regel: sidebjælken markerer Angreb', d.querySelector('#side a[data-visning="vaernet"]').getAttribute('aria-current') === 'page');
  tjek('D regel: hovedet har mærke, CVE-knap, typetekst, Cloudflares navn og guiden', q('.dyk-hoved .vg-produkt').textContent === 'React' && q('.dyk-hoved .vg-cve-knap').textContent === 'CVE-2025-55182 ↗' && q('.dyk-type').textContent.startsWith('Fjernkørsel af kode') && q('.dyk-navn code').textContent === REACT && q('.dyk-guide .g-spoerg').textContent.includes('React eller Next.js'));
  q('.dyk-hoved .vg-cve-knap').click(); await sov(10);
  tjek('D regel: CVE-knappen kalder aabn_cve og bliver i visningen', JSON.stringify(kald.filter(k => k[0] === 'aabn_cve').pop()) === '["aabn_cve",{"id":"CVE-2025-55182"}]' && w.location.hash.startsWith('#dyk?'));
  tjek('D regel: ingen Alt/Kun angreb-vælger (J4)', !q('.dyk-slags'));
  const fl = alle('.vg-flise').map(f => f.textContent);
  tjek('D fliser: 30 dage (aktiv), 7 dage, i går med i dag hidtil, først og sidst set', fl[0] === '198Afvist på 30 dagehele dage til og med i går' && fl[1] === '57Afvist på 7 dage' && fl[2] === '6Afvist i gåri dag hidtil: 2' && fl[3].includes('5. oktober') && q('.vg-flise[aria-current]').textContent.startsWith('198'), fl.join(' | '));
  tjek('D fordeling: klikbare tal og siderne med fælde-links på egen linje', q('.vg-fordeling').textContent.includes('190 blokeret') && q('.vg-faelder').textContent === '21 sider blev leveret med skjulte fælde-links til robotter' && new URLSearchParams(q('.vg-faelder button').dataset.dyk).get('handling') === 'faelder');
  tjek('D lister: seks lister uden regel-listen (filtrets egen dimension)', alle('.dyk-lister h3').map(h => h.textContent).join() === 'Lande,Netværk,Stier,Datacentre,Metoder,Handling', alle('.dyk-lister h3').map(h => h.textContent).join());
  tjek('D lister: top 5 fremme, resten bag "Vis top 20 (8)"', alle('.dyk-lister > div')[0].querySelectorAll(':scope > ol li').length === 5 && q('.dyk-flere summary').textContent === 'Vis top 20 (8)' && q('.dyk-flere').querySelectorAll('li').length === 3);
  const lande = alle('.dyk-lister > div')[0].textContent;
  tjek('D T1 og XX i klarsprog', lande.includes('Tor-netværket') && lande.includes('Ukendt land') && !lande.includes('T1') && !lande.includes('XX'), lande);
  tjek('D fremmed tekst står som tekst, ingen HTML', !q('.dyk-lister b:not(.x)') && alle('.vg-l-navn').some(e => e.textContent === '<b>Fed</b> net'));
  tjek('D stien står renset som <code>', alle('code.vg-l-navn').some(e => e.textContent === '/wp-config%2ephp'));
  tjek('D andre domæner: overskrift og tal', alle('.vg-blok h2').some(h => h.textContent === 'Samme regel på dine andre domæner') && q('.dyk-andre .vg-l-n').textContent === '92');
  tjek('D de seneste: tabel uden regelkolonne ved regel-filter', [...q('.vg-tabel thead').querySelectorAll('th')].map(t => t.textContent).join() === 'Tidspunkt (UTC, hel time),Land,Sti,Datacenter,Metode');
  tjek('D ingen style= og ingen <text> i dyk', !d.getElementById('visning-dyk').outerHTML.includes('style=') && !q('svg text'));
  const svgs = alle('svg');
  tjek('D tegninger: ét Tab-stop pr. tegning, role=group', svgs.length === 2 && svgs.every(x => x.querySelectorAll('[tabindex="0"]').length === 1 && x.getAttribute('role') === 'group'), svgs.map(x => x.querySelectorAll('[tabindex="0"]').length).join());
  tjek('D søjler: strækkes, og aksens tekst står som HTML', q('svg.dyk-soejler').getAttribute('preserveAspectRatio') === 'none' && q('.dyk-y-top').textContent === '29' && alle('.dyk-x b').length === 5, q('.dyk-y-top').textContent);
  const hvid = alle('.dyk-soejle').length;
  tjek('D søjler: én flade pr. dag; dage med 0 og uhentede kan ikke åbnes', hvid === 30 && alle('.dyk-soejle[data-dyk]').length === dykDage.filter(x => x.n).length && alle('.dyk-uhentet').length === 1 && !q('.dyk-uhentet[data-dyk]') && q('.dyk-uhentet title').textContent === '7. oktober: ingen data — Huginn var slukket', `${alle('.dyk-soejle[data-dyk]').length}`);
  // Tastatur: piletaster flytter Tab-stoppet, Enter åbner.
  const sv = q('svg.dyk-soejler'), start = sv.querySelector('[tabindex="0"]');
  start.focus();
  start.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true, cancelable: true }));
  const ny = d.activeElement;
  tjek('D tastatur: pil flytter fokus og det ene Tab-stop', ny !== start && sv.querySelectorAll('[tabindex="0"]').length === 1 && ny.getAttribute('tabindex') === '0' && start.getAttribute('tabindex') === '-1');
  const adr = w.location.hash;
  ny.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
  await sov(40);
  tjek('D tastatur: Enter åbner dagen', w.location.hash !== adr && new URLSearchParams(w.location.hash.slice(5)).has('dag') && sidsteDyk(s).periode.dag, w.location.hash);
}

// Kæde, chips, tilbage/frem
{
  const s = side('#dyk?' + dykQ({ regel: REACT }), DYK());
  await sov(90);
  const { d, w } = s, q = sel => d.querySelector('#visning-dyk ' + sel), hoved = d.getElementById('indhold');
  hoved.scrollTop = 120;
  [...d.querySelectorAll('.dyk-l')].find(b => b.textContent.startsWith('Polen')).click(); await sov(60);
  tjek('D kæde: land lægges oveni reglen', JSON.stringify(sidsteDyk(s).filtre) === JSON.stringify([{ dim: 'regel', vaerdi: REACT }, { dim: 'land', vaerdi: 'PL' }]), JSON.stringify(sidsteDyk(s).filtre));
  tjek('D kæde: chips for regel og Polen; listen for land er væk', [...q('.dyk-filtre').querySelectorAll('button')].map(l => l.firstChild.firstChild.textContent).join(' › ') === 'Et kendt hul i React › Polen' && ![...d.querySelectorAll('#visning-dyk .dyk-lister h3')].some(h => h.textContent === 'Lande') && d.getElementById('dyk-h').textContent === 'Angreb på wi24rd.com med reglen Et kendt hul i React', q('.dyk-filtre').textContent);
  const fjern = [...d.querySelectorAll('.dyk-filter')];
  tjek('D chip: × fjerner netop det filter', fjern.length === 2 && new URLSearchParams(fjern[0].dataset.dyk).get('land') === 'PL' && !new URLSearchParams(fjern[0].dataset.dyk).has('regel') && fjern[0].getAttribute('aria-label') === 'Fjern filtret Et kendt hul i React');
  w.history.back(); await sov(120);
  tjek('D tilbage: samme filtre og rulleposition', JSON.stringify(sidsteDyk(s).filtre) === JSON.stringify([{ dim: 'regel', vaerdi: REACT }]) && hoved.scrollTop === 120, `${hoved.scrollTop}`);
  w.history.forward(); await sov(120);
  tjek('D frem: kæden igen', sidsteDyk(s).filtre.length === 2);
  q('#dyk-tilbage button').click(); await sov(60);
  tjek('D knappen går til domænets Angreb', w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com', w.location.hash);
}

// Land-dyk, slags, periode, domæneskift, hele historikken
{
  const s = side('#dyk?' + dykQ({ land: 'PL' }), DYK());
  await sov(90);
  const { d, w } = s, q = sel => d.querySelector('#visning-dyk ' + sel), alle = sel => [...d.querySelectorAll('#visning-dyk ' + sel)];
  tjek('D land: overskrift, ingen hovedkort, vælgeren med tal', d.getElementById('dyk-h').textContent === 'Angreb på wi24rd.com fra Polen' && !q('.dyk-hoved') && alle('.dyk-slags label').map(l => l.textContent).join() === 'Alt,Kun angreb (150),Kun rutine (48)', alle('.dyk-slags label').map(l => l.textContent).join());
  tjek('D land: regelkortene erstatter listen, og listen for land er væk', !alle('.dyk-lister h3').some(h => ['Regler', 'Lande'].includes(h.textContent)) && alle('.vg-regel').length === 2);
  const slag = alle('.dyk-slags input')[1];
  slag.focus(); slag.checked = true; slag.dispatchEvent(new w.Event('change', { bubbles: true })); await sov(60);
  tjek('D slags: Kun angreb sendes til Rust, er ét trin i adressen, og fokus bliver på valget', sidsteDyk(s).slags === 'angreb' && w.location.hash.includes('slags=angreb') && d.activeElement === alle('.dyk-slags input')[1], w.location.hash);
  const p7 = d.querySelectorAll('#dyk-periode input')[0];
  p7.checked = true; p7.dispatchEvent(new w.Event('change', { bubbles: true })); await sov(60);
  tjek('D periode: 7 dage med samme filter og slags', sidsteDyk(s).periode === 'd7' && sidsteDyk(s).slags === 'angreb' && sidsteDyk(s).filtre[0].vaerdi === 'PL' && q('.vg-flise[aria-current]').textContent.startsWith('57'), JSON.stringify(sidsteDyk(s)));
  d.querySelector('.dato-vaelger button.link').click(); await sov(60);
  tjek('D periode: hele historikken får sin egen flise', sidsteDyk(s).periode === 'alt' && q('.vg-flise[aria-current]').textContent.includes('Angreb i hele historikken'));
  const v = d.getElementById('vg-zone');
  tjek('D vælgeren: i båndet, viser domænet', !d.getElementById('band-zone').hidden && v.value === 'z1', v.value);
  v.value = 'z2'; v.dispatchEvent(new w.Event('change')); await sov(60);
  tjek('D vælgeren: domæneskift beholder filter, slags og periode', JSON.stringify(sidsteDyk(s).omfang) === '{"zone":"z2"}' && sidsteDyk(s).filtre[0].vaerdi === 'PL' && sidsteDyk(s).slags === 'angreb' && sidsteDyk(s).periode === 'alt' && d.getElementById('dyk-h').textContent.includes('xyz.wi24rd.com'), JSON.stringify(sidsteDyk(s)));
  v.value = 'samlet'; v.dispatchEvent(new w.Event('change')); await sov(60);
  tjek('D vælgeren: Samlet = de valgte zoner', JSON.stringify(sidsteDyk(s).omfang) === '{"zoner":["z1","z2"]}' && w.location.hash.includes('samlet=1') && !w.location.hash.includes('zone=') && d.getElementById('dyk-under').textContent === '2 domæner', JSON.stringify(sidsteDyk(s).omfang));
  s.emit('hentekoe', { zone: 'z1', tilstand: 'henter' });
  const n = s.kald.filter(k => k[0] === 'dyk').length;
  await sov(3100);
  tjek('D hentekoe: tegnes igen, højst hvert 3. sekund', s.kald.filter(k => k[0] === 'dyk').length === n + 1);
  s.w.hmRamme.gaaTil('overblik'); await sov(30);
  tjek('D skjult: afmelder alt', s.w.hmRamme.antalAbonnenter('hentekoe') === 0);
}

// Dag, tomt svar, fejl, sti-kategori, netværk, forsøg og engelsk
{
  const dag = side('#dyk?' + dykQ({ regel: REACT, dag: '2026-11-03' }), DYK());
  await sov(90);
  const q = sel => dag.d.querySelector('#visning-dyk ' + sel);
  tjek('D dag: ingen søjler, de seneste sidst, ingen periode valgt, dagen som chip', !q('.dyk-soejler') && [...dag.d.querySelectorAll('#visning-dyk .vg-blok h2')].at(-1).textContent === 'De seneste, der blev afvist' && JSON.stringify(sidsteDyk(dag).periode) === '{"dag":"2026-11-03"}' && ![...dag.d.querySelectorAll('#dyk-periode input')].some(i => i.checked) && [...dag.d.querySelectorAll('.dyk-filter')].at(-1)?.textContent === '3. november×', q('.dyk-filtre').textContent);
  const tom = side('#dyk?' + dykQ({ land: 'IS' }), DYK({ dyk: () => dykSvar({ sum: 0, noegletal: { i_gaar: 0, i_dag_hidtil: 0, hele7: 0, hele30: 0, alt: 0 }, foerst: null, sidst: null, domaener: [], enkelte: [], top: { regel: [], land: [], netvaerk: [], sti: [], colo: [], metode: [], handling: [] }, buer: [], fordeling: { blokeret: 0, udfordret: 0, labyrint: 0, andet: 0, faelder: 0 } }) }));
  await sov(90);
  tjek('D tomt: besked, ingen tegninger, og 0 er ikke klikbart', tom.d.querySelector('.dyk-tom').textContent === 'Ingen tal i perioden.' && !tom.d.querySelector('#visning-dyk svg') && !tom.d.querySelector('#dyk-rod [data-dyk]'), tom.d.getElementById('dyk-rod').innerHTML.slice(0, 200));
  const fejl = side('#dyk?' + dykQ({ regel: 'x' }), DYK({ dyk: () => { throw { kode: 'ugyldigt_filter' }; } }));
  await sov(90);
  tjek('D fejl: fejlkoden på dansk', fejl.d.querySelector('#dyk-rod .vg-advarsel').textContent === 'Filteret i adressen kan ikke bruges.');
  const kat = side('#dyk?' + dykQ({ sti: '[wordpress]' }), DYK());
  await sov(90);
  tjek('D sti: kategoriens forklaring', kat.d.querySelector('.dyk-note').textContent === 'Login- og adminsider på WordPress.');
  const net = side('#dyk?' + dykQ({ netvaerk: 'AT&T Services, Inc.' }), DYK());
  await sov(90);
  tjek('D netværk: navnet som overskrift og noten om nummeret', net.d.getElementById('dyk-h').textContent === 'Angreb på wi24rd.com fra netværket AT&T Services, Inc.' && net.d.querySelector('.dyk-note').textContent.startsWith('Kun netværkets navn'));
  const fs2 = side('#dyk?' + dykQ({ forsoeg: 'wordpress' }), DYK());
  await sov(90);
  tjek('D forsøg: kategorien, kun stier, ingen kort og ingen vælger', fs2.d.getElementById('dyk-h').textContent === 'Hændelser på wi24rd.com: WordPress' && [...fs2.d.querySelectorAll('.dyk-lister h3')].map(h => h.textContent).join() === 'Stier' && !fs2.d.querySelector('svg.vg-kort') && !fs2.d.querySelector('.dyk-slags') && fs2.d.querySelector('.vg-flise').textContent.includes('Hændelser på 30 dage'), fs2.d.getElementById('dyk-h').textContent);
  const faelder = side('#dyk?' + dykQ({ regel: 'linkMaze', handling: 'faelder' }), DYK({ dyk: () => dykSvar({ regel: { navn: 'linkMaze', produkt: '', type: '', cve: null, art: 'rutine' } }) }));
  await sov(90);
  tjek('D rutine: klarsprog, grå top, ingen CVE, rutinens råd, og fælde-links hedder ikke "Afvist"', faelder.d.getElementById('dyk-h').textContent.startsWith('Hændelser på wi24rd.com med reglen Labyrinten (falske sider til robotter)') && faelder.d.querySelector('.dyk-hoved.dyk-rutine .vg-produkt').textContent === 'Rutine' && !faelder.d.querySelector('.dyk-hoved .vg-cve-knap') && faelder.d.querySelector('.dyk-guide .g-spoerg').textContent.includes('Cloudflares rutine') && faelder.d.querySelector('.vg-flise').textContent.includes('Hændelser på 30 dage'), faelder.d.getElementById('dyk-h').textContent);
  const egen = side('#dyk?' + dykQ({ regel: 'min-regel: max 100 kald' }), DYK({ dyk: () => dykSvar({ regel: { navn: 'min-regel: max 100 kald', produkt: 'min-regel: max 100 kald', type: '', cve: null, art: 'egen' } }) }));
  await sov(90);
  tjek('D egen regel: overskrift og bogens afsnit', egen.d.getElementById('dyk-h').textContent === 'Angreb på wi24rd.com med reglen Din egen regel »min-regel«' && egen.d.querySelector('.dyk-guide .g-spoerg').textContent.includes('Det er din egen regel'), egen.d.getElementById('dyk-h').textContent);
  const uden = side('#dyk?' + dykQ(), DYK());
  await sov(90);
  tjek('D kun periode: "Angreb på domænet", alle lister', uden.d.getElementById('dyk-h').textContent === 'Angreb på wi24rd.com' && uden.d.querySelectorAll('.dyk-lister h3').length === 6 && uden.d.getElementById('dyk-under').textContent === '');
  const en = side('#dyk?' + dykQ({ regel: REACT }), DYK(), 'en');
  await sov(90);
  tjek('D engelsk: tekster og fælde-linjen', en.d.getElementById('dyk-h').textContent === 'Attacks on wi24rd.com with the rule A known hole in React' && en.d.querySelector('.vg-faelder').textContent === '21 pages were served with hidden trap links for robots' && en.d.querySelector('.vg-flise').textContent.startsWith('198Refused in 30 days'), en.d.getElementById('dyk-h').textContent);
  const lands = en.d.querySelector('.dyk-lister').textContent;
  tjek('D engelsk: T1 og XX', lands.includes('The Tor network') && lands.includes('Unknown country'), lands);
  const en1 = side('#dyk?' + dykQ(), DYK({ dyk: () => dykSvar({ fordeling: { blokeret: 1, udfordret: 0, labyrint: 0, andet: 0, faelder: 1 } }) }));
  await sov(90);
  tjek('D fælde-linjen i ental', en1.d.querySelector('.vg-faelder').textContent === '1 side blev leveret med skjulte fælde-links til robotter', en1.d.querySelector('.vg-faelder').textContent);
  const stop = side('#dyk?samlet=1', DYK({ valg_hent: [] }));
  await sov(90);
  tjek('D samlet uden valg: velkomsten', stop.d.querySelector('#dyk-rod .velkomst-situation').textContent.startsWith('Du har ikke valgt nogen domæner endnu'));
}

// Angreb: regelkortene (Q6c, J7, J9) og listerne
{
  const kort = [
    kortRegel({ navn: 'botFight', produkt: '', type: '', cve: null, n7: 1, n30: 1, foerst: '2026-10-05', sidst: '2026-11-02', stier: [], lande: [] }, { art: 'rutine', h7: 40, h30: 500 }),
    kortRegel({ navn: REACT, produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', n7: 20, n30: 161, foerst: '2026-10-05', sidst: '2026-11-02', stier: [{ sti: '/', n: 80 }], lande: [{ kode: 'T1', n: 80 }] }, { h7: 57, h30: 158 }),
    kortRegel({ navn: 'min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)', produkt: 'min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)', type: '', cve: null, n7: 1, n30: 2, foerst: '2026-10-05', sidst: '2026-11-02', stier: [], lande: [] }, { art: 'egen', h7: 1, h30: 2 }),
    kortRegel({ navn: 'linkMaze', produkt: '', type: '', cve: null, n7: 1, n30: 900, foerst: '2026-10-05', sidst: '2026-11-02', stier: [], lande: [] }, { art: 'rutine', h7: 1, h30: 7, faelder30: 900 }),
    kortRegel({ navn: 'Browser Integrity Check', produkt: '', type: 'bic', cve: null, n7: 1, n30: 1, foerst: '2026-10-05', sidst: '2026-11-02', stier: [], lande: [] }, { art: 'rutine' }),
  ];
  const fd = { d30: { blokeret: 10, udfordret: 20, labyrint: 7, andet: 2, faelder: 6021 }, i_gaar: { blokeret: 1, udfordret: 2, labyrint: 0, andet: 0, faelder: 1 } };
  const data = vaernUd({ h_regler: kort, fordeling: fd, dage: [{ dag: '2026-11-04', n: 999999, alle: 1 }], lande: [{ kode: 'DK', n: 999999 }], h_lande: [{ kode: 'PL', n: 67 }, { kode: 'T1', n: 13 }, { kode: 'XX', n: 2 }], h_netvaerk: [{ navn: 'AT&T Services, Inc.', n: 60 }], h_colos: [{ kode: 'WAW', n: 67 }], h_dage: dage.map(x => ({ ...x, n: x.n + 1 })) });
  const s = side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: data, vaern_oversigt: data, koe_prioritet: null }));
  await sov(120);
  const { d, w } = s, k = [...d.querySelectorAll('#vg-regler .vg-regel')], tekst = i => k[i].textContent;
  tjek('Q6c rutine: mærket Rutine, klarsprog, grå top', k[0].classList.contains('vg-rutine') && k[0].querySelector('.vg-produkt').textContent === 'Rutine' && k[0].querySelector('h3').textContent === 'Bot Fight Mode (kendte bot-mønstre)' && !tekst(0).includes('botFight'), tekst(0));
  tjek('Q6c browsertjekket i klarsprog (J7)', k[4].querySelector('h3').textContent === 'Browsertjekket (lignede ikke en rigtig browser)' && k[4].classList.contains('vg-rutine'));
  tjek('Q6c angreb: mærke + CVE-knap, typetekst, ingen <code>-undertitel, ingen rutine-klasse', k[1].querySelector('.vg-produkt').textContent === 'React' && k[1].querySelector('.vg-cve-knap').textContent === 'CVE-2025-55182' && k[1].querySelector('h3').textContent.startsWith('Fjernkørsel af kode') && !tekst(1).includes('CVE:CVE-') && !k[1].classList.contains('vg-rutine'), tekst(1));
  tjek('Q6c egen regel: mærket "Din egen regel »min-regel«" og beskrivelsen én gang', k[2].querySelector('.vg-produkt').textContent === 'Din egen regel »min-regel«' && tekst(2).split('max 100 kald').length === 2, tekst(2));
  tjek('J1 regelkortet viser de hele tal (158, ikke 161)', k[1].querySelector('.vg-regel-n b').textContent === '158' && tekst(1).includes('57 på 7 dage') && !tekst(1).includes('161'), tekst(1));
  tjek('J9 Labyrinten: "lokket ind" og "fælde-links sat ind"', k[3].querySelector('.vg-regel-n').textContent === '7 lokket ind900 fælde-links sat ind', k[3].querySelector('.vg-regel-n').textContent);
  tjek('Q6c noten siger Cloudflare, ikke Værnet', d.querySelector('#vg-regler .vg-note').textContent.includes('Cloudflare stoppede det alligevel') && !d.getElementById('vagt').textContent.includes('Værnet'));
  tjek('J9 fælde-linjen står for sig i "I går" og "30 dage"', [...d.querySelectorAll('.vg-faelder')].map(x => x.textContent).join(' | ') === 'I går: 1 side blev leveret med skjulte fælde-links til robotter | 30 dage: 6.021 sider blev leveret med skjulte fælde-links til robotter', [...d.querySelectorAll('.vg-faelder')].map(x => x.textContent).join(' | '));
  tjek('J9 "heraf" uden fælde-links; andet står som ikke afvist', d.querySelector('.vg-fordeling').textContent.includes('30 dage: heraf 10 blokeret · 20 udfordret · 7 lokket ind i labyrinten · og 2 andet, som ikke tæller som afvist'), d.querySelector('.vg-fordeling').textContent);
  const lister = [...d.querySelectorAll('.vg-tre > div')];
  tjek('J1 lister og søjler bruger de hele felter (h_*), ikke W\'s', lister[0].textContent.includes('Polen') && !lister[0].textContent.includes('Danmark') && d.querySelector('.vg-tavle-top b').textContent === String(dage.reduce((a, x) => a + x.n + 1, 0)) && !d.getElementById('vg-dage').textContent.includes('999'), d.querySelector('.vg-tavle-top').textContent);
  tjek('T1 og XX i Angreb', lister[0].textContent.includes('Tor-netværket') && lister[0].textContent.includes('Ukendt land'), lister[0].textContent);
  tjek('Angreb: <text> er væk fra kort og søjler, og tekst står som HTML', !d.querySelector('#vagt svg text') && d.querySelector('#vagt .dyk-etiket') && d.querySelector('#vagt .dyk-y-top'));
  tjek('Angreb: etiketten sidder via en klasse med en regel, ikke via style=', [...w.hmTegn.ark.cssRules].some(r => r.selectorText === '.' + [...d.querySelector('#vagt .dyk-etiket').classList].find(c => c.startsWith('vg-t'))) && !d.querySelector('.vg-kortramme').innerHTML.includes('style='));
  const b = [...d.querySelectorAll('.vg-tre .dyk-l')];
  tjek('Lande, Netværk, Afvist i er knapper med aria-label', b.length === 5 && b[0].getAttribute('aria-label') === 'Polen: 67. Åbn.' && b.every(x => x.dataset.dyk), b.map(x => x.getAttribute('aria-label')).join());
  b[3].click(); await sov(60);
  tjek('Netværk åbner dyk (AT&T overlever adressen)', sidsteDyk(s).filtre[0].dim === 'netvaerk' && sidsteDyk(s).filtre[0].vaerdi === 'AT&T Services, Inc.' && synlige(d).join() === 'dyk', JSON.stringify(sidsteDyk(s)));
  w.history.back(); await sov(120);
  [...d.querySelectorAll('.vg-regel-aabn')][1].click(); await sov(60);
  tjek('Regelkortet åbner regel-dyk', sidsteDyk(s).filtre[0].vaerdi === REACT && d.getElementById('dyk-h').textContent.endsWith('med reglen Et kendt hul i React'));
  w.history.back(); await sov(120);
  const cve = d.querySelector('#vg-regler .vg-cve-knap'), foer = w.location.hash;
  cve.click(); await sov(20);
  tjek('CVE i kortet åbner cve.org og ikke dyk', w.location.hash === foer && s.kald.some(x => x[0] === 'aabn_cve'));
  const samlet = side('#vaernet?samlet=1', DYK({ vaern_samlet_cache: { domaener: 2, med_tal: 2, valgte: [], vaern: data } }));
  await sov(120);
  tjek('Samlet Angreb: listerne åbner samlet dyk', samlet.d.querySelector('.vg-tre .dyk-l').dataset.dyk === 'samlet=1&land=PL', samlet.d.querySelector('.vg-tre .dyk-l').dataset.dyk);
}

// Klikbarhedseftersynet: Overblik, Siden sidst, Angreb, dyk og Indstillinger, tegnet.
{
  const K_VAERN = vaernUd({ forsoeg: { i7: 12, kategorier: [{ id: 'wordpress', n: 12, stier: [{ sti: '/wp-login.php', n: 12 }] }] }, h_lande: [{ kode: 'US', n: 80 }, { kode: 'T1', n: 3 }], h_colos: [{ kode: 'EWR', n: 80 }, { kode: 'UDI', n: 1 }] });
  const css = ['app.css', 'vagt.css'].map(f => udenKommentarer(ui('assets/' + f))).join('\n').replace(/@media[^{]+\{/g, '');
  const regler = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map(m => [m[1].trim(), m[2]]);
  const delt = v => v.split(/,(?![^(]*\))/).map(x => x.trim()).filter(Boolean);
  const pointer = regler.filter(([, k]) => /cursor:\s*pointer/.test(k)).flatMap(([v]) => delt(v));
  const hover = regler.flatMap(([v]) => delt(v)).filter(v => v.includes(':hover'));
  const ren = v => v.replace(/:(hover|focus-visible|active|focus)\b/g, '').replace(/::[a-z-]+/g, '').replace(/:not\(:disabled\)/g, '').replace(/:not\(\.[a-z]+\)/g, '').replace(/:root/g, 'html');
  const visninger = [
    ['dyk', side('#dyk?' + dykQ({ regel: REACT }), DYK())],
    ['dyk land', side('#dyk?' + dykQ({ land: 'PL' }), DYK())],
    ['Angreb', side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: K_VAERN, vaern_oversigt: K_VAERN, koe_prioritet: null }))],
    ['Overblik', side('#overblik', DYK())],
    ['Siden sidst', side('#siden-sidst', DYK({ siden_sidst: { vis: false, nye: [], raekker: OVERBLIK }, siden_sidst_set: null }))],
    ['Indstillinger', side('#indstillinger', DYK({ data_status: { bytes: 8e6, foerste: '2026-10-04', sidste: '2026-11-03', domaener: 3 }, zones_cache: { zoner: OVERBLIK, hentet: NU_ISO }, domaener_hent: OVERBLIK.map((r, i) => ({ id: r.id, navn: r.navn, til: i > 0 })), ...VIS }))],
    ['Overblik, intet valgt', side('#overblik', DYK({ overblik: [], domaener_hent: OVERBLIK.map(r => ({ id: r.id, navn: r.navn, til: false })) }))],
    ['Overblik med nye domæner', side('#overblik', DYK({ domaener_hent: [{ id: 'x', navn: 'x.wi24rd.com', til: false, ny: true }, { id: 'y', navn: 'y.wi24rd.com', til: false, ny: true }, ...OVERBLIK.map(r => ({ id: r.id, navn: r.navn, til: true, ny: false }))] }))],
    ['Overblik med fravalgte', side('#overblik', DYK({ domaener_hent: [{ id: 'x', navn: 'x.wi24rd.com', til: false }, ...OVERBLIK.map(r => ({ id: r.id, navn: r.navn, til: true }))] }))],
    ['Robotter', side('#robotter?zone=z1&navn=wi24rd.com', DYK(VIS))],
    ['Login', side('#login?zone=z1&navn=wi24rd.com', DYK(VIS))],
    ['dyk robot', side('#dyk?' + dykQ({ robot: 'alle' }), DYK({ dyk: () => ROBOT_DYK }))],
    ['Land', side('#land?kode=DK', LANDV())],
    ['Verden', side('#land?kode=DK&fane=verden', LANDV())],
    ['Overblik med server og trafik', side('#overblik', DYK({ overblik: B2B_OVERBLIK }))],
    ['Angreb med trafik', side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: { ...K_VAERN, trafik: B2B_TRAFIK }, vaern_oversigt: { ...K_VAERN, trafik: B2B_TRAFIK }, koe_prioritet: null }))],
    ['trafik-dyk', side('#dyk?' + dykQ({ dag: '2026-11-02', trafik: '1' }), DYK({ trafik_dag: B2C_TRAFIK }))],
    ['Angreb med teknologi og huller', side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: { ...K_VAERN, uhentet: [K_VAERN.h_dage[0].dag] }, vaern_oversigt: { ...K_VAERN, uhentet: [K_VAERN.h_dage[0].dag] }, koe_prioritet: null, teknologi_hent: B2C_TEK }))],
    ['Verden med forstyrrelser', side('#land?kode=DK&fane=verden', LANDV({ land_visning: a => ({ ...LAND_UD, kode: a.kode, udfald_verden: B2C_VERDEN, udfald_verden_hentet: true }) }))],
    ['Overblik med huller og spørgsmål', side('#overblik', DYK({ overblik: [raekke('s1', 'spoerg.wi24rd.com', 'gul', { angreb_i_gaar: 47, angreb_snit: 3, angreb_i_gaar_top: [{ navn: 'Wordpress', n: 47, tek: 'wordpress' }], angreb_fordeling: { blokeret: 47, udfordret: 0, labyrint: 0, andet: 0 }, tek_spoerg: 'wordpress', angreb_status: 'gul', sidst_hentet: NU_ISO })], huller: B2C_HULLER }))],
  ];
  await sov(130);
  const ER_KONTROL = 'button, a, input, select, summary, label, tr[tabindex], [data-dyk], [data-gaa], [data-rul]';
  const kontrol = e => e.matches(ER_KONTROL) || e.closest(ER_KONTROL) || e.querySelector('[data-dyk], [data-gaa]');
  for (const [navn, s] of visninger) {
    const { d, w } = s, m = [...d.querySelectorAll('section:not([hidden]) [data-dyk], section:not([hidden]) [data-gaa], section:not([hidden]) [data-rul]')];
    // .dyk-hit er kun en større klikflade: fokus og navn har mærket selv, og fladen er skjult for skærmlæsere.
    const flader = m.filter(e => e.matches('.dyk-hit')), maerker = m.filter(e => !e.matches('.dyk-hit'));
    const utilgaengelig = maerker.filter(e => !(e.matches('button, a, input') || e.hasAttribute('tabindex')));
    tjek(`K ${navn}: hvert klikbart element har fokus (${maerker.length})`, maerker.length > 0 && !utilgaengelig.length, utilgaengelig.slice(0, 3).map(e => e.outerHTML.slice(0, 100)).join(' '));
    const udenNavn = m.filter(e => !/\S.*\S/.test(e.getAttribute('aria-label') || ''));
    tjek(`K ${navn}: hvert klikbart element har aria-label med målet`, !udenNavn.length, udenNavn.slice(0, 3).map(e => e.outerHTML.slice(0, 120)).join(' '));
    const forladte = flader.filter(e => e.getAttribute('aria-hidden') !== 'true' || !maerker.some(x => x.getAttribute('data-dyk') === e.getAttribute('data-dyk') && x.getAttribute('aria-label') === e.getAttribute('aria-label')));
    tjek(`K ${navn}: hver klikflade hører til et mærke med samme mål og er skjult for skærmlæsere (${flader.length})`, !forladte.length, forladte.slice(0, 2).map(e => e.outerHTML.slice(0, 120)).join(' '));
    /* Hele-fladen-hover står som `.kort:has(.knap:hover)`: den gælder knappen inde i parentesen. */
    const hoverMaal = v => (/:has\((.+?):hover\)/.exec(v) || [])[1] || ren(v.split(':hover')[0]);
    const udenHover = m.filter(e => !hover.some(v => { try { return e.matches(hoverMaal(v)); } catch (x) { return false; } }));
    /* En stor flade får aldrig understregning. Stor = rummer et blokindhold (tal og tekst i fliser, kort og hele-rækken-lister). */
    const understreg = [...fjernCssKommentarer(ui('assets/app.css') + ui('assets/vagt.css')).matchAll(/([^{}]+)\{([^{}]*)\}/g)].map(x => [x[1].trim(), x[2]]).filter(([sel, krop]) => sel.includes(':hover') && /text-decoration:\s*underline/.test(krop)).flatMap(([sel]) => sel.split(',')).map(x => x.trim());
    const store = m.filter(e => !e.matches('.link') && (e.matches('.dyk-flise, .dyk-flise-aabn, .vg-regel-aabn, .dyk-l, .gruppe-kort, .hm-siden-aabn') || e.querySelector('b, div, ol, ul, p, h3, h4')));
    const streget = store.filter(e => understreg.some(v => { try { return e.matches(v.split(':hover')[0]); } catch (x) { return false; } }));
    tjek(`K ${navn}: ingen understregning ved hover på store mål (${store.length})`, !streget.length, streget.slice(0, 3).map(e => e.outerHTML.slice(0, 100)).join(' '));
    tjek(`K ${navn}: hvert klikbart element har en hover-regel`, !udenHover.length, udenHover.slice(0, 3).map(e => e.outerHTML.slice(0, 100)).join(' '));
    const uden = m.filter(e => !pointer.some(v => { try { return e.matches(ren(v)); } catch (x) { return false; } }));
    tjek(`K ${navn}: hvert klikbart element rammes af cursor: pointer`, !uden.length, uden.slice(0, 3).map(e => e.outerHTML.slice(0, 100)).join(' '));
    const falsk = [], ukendt = [];
    for (const v of [...new Set([...pointer, ...hover])]) {
      let ramt;
      try { ramt = [...d.querySelectorAll(ren(v))]; } catch (x) { ukendt.push(v); continue; }
      for (const e of ramt) if (!kontrol(e)) falsk.push(`${v} → ${e.tagName}.${e.getAttribute('class')}`);
    }
    tjek(`K ${navn}: ingen falsk klikbarhed (pointer eller hover uden handling)`, !falsk.length, [...new Set(falsk)].slice(0, 4).join(' | '));
    tjek(`K ${navn}: alle selektorer kunne prøves`, ukendt.filter(v => !v.includes(':has')).length === 0, ukendt.join(' | '));
    tjek(`K ${navn}: ingen <text> i SVG`, !d.querySelector('section:not([hidden]) svg text'));
    const q = m.filter(e => e.hasAttribute('data-dyk') || e.hasAttribute('data-gaa'));
    let forkert = 0;
    for (const e of q.filter(x => x.hasAttribute('data-dyk'))) { const f = new URLSearchParams(e.getAttribute('data-dyk')); if (!f.has('zone') && !f.has('samlet')) forkert++; }
    tjek(`K ${navn}: hvert mål har omfanget med`, forkert === 0 && m.length > 0, String(forkert));
    // Handling: hvert element klikkes (ikke-knapper også med Enter og mellemrum), og adressen skal blive #dyk?… med præcis det mål.
    const galt = [];
    for (const e of q) {
      const maal = e.hasAttribute('data-dyk') ? '#dyk?' + e.getAttribute('data-dyk') : '#' + e.getAttribute('data-gaa');
      for (const hvordan of e.matches('button') ? ['klik'] : ['klik', 'Enter', ' ']) {
        w.location.hash = '#x-' + Math.random().toString(36).slice(2);
        await sov(0);
        if (hvordan === 'klik') e.dispatchEvent(new w.MouseEvent('click', { bubbles: true, cancelable: true }));
        else e.dispatchEvent(new w.KeyboardEvent('keydown', { key: hvordan, bubbles: true, cancelable: true }));
        if (w.location.hash !== maal) galt.push(`${hvordan}: ${w.location.hash} ≠ ${maal}`);
      }
      if (galt.length > 3) break;
    }
    tjek(`K ${navn}: hvert element åbner sit mål (${q.length})`, !galt.length, galt.slice(0, 2).join(' | '));
  }
}

// Ordene, noterne og resterne (Q6b, Q6c)
{
  const s = side('#overblik', DYK(), 'da', 'macos'), e = side('#overblik', DYK(), 'en', 'macos');
  await sov(60);
  const strenge = o => (typeof o === 'string' ? [o] : o && typeof o === 'object' ? Object.values(o).flatMap(strenge) : []);
  const ord = x => strenge(x.w.hmSprog.T).filter(t => /værn|defences/i.test(t.replace(/first line of defence/gi, '')));
  tjek('Q6c ingen "værn" eller "defences" i teksterne ("first line of defence" er tilladt)', !ord(s).length && !ord(e).length, [...ord(s), ...ord(e)].slice(0, 2).join(' | '));
  tjek('Q6c ingen "Værnet stoppede" eller "The defences" i Angreb', !/Værnet stoppede|The defences/.test(udenKommentarer(ui('assets/visning-vaernet.js'))));
  const { d, w } = s, R = w.hmRamme, Sp = w.hmSprog;
  tjek('Q6b note: macOS-noterne står med data-t', d.getElementById('forb-note').textContent === 'API Tokenet ligger i macOS Nøglering.' && d.getElementById('data-hist-note').textContent === 'Programmet gemmer al historik på din Mac og sletter intet af sig selv.');
  Sp.saetDomaene('a.wi24rd.com', false);
  tjek('Q6b anvend(): saetDomaene skriver ikke noterne tilbage', d.getElementById('forb-note').textContent === 'API Tokenet ligger i macOS Nøglering.' && d.getElementById('data-hist-note').textContent.includes('din Mac'), d.getElementById('forb-note').textContent);
  d.getElementById('dom-hjaelp').click();
  const h = d.getElementById('hjaelp-h').textContent, knap = d.getElementById('dom-hjaelp').textContent;
  Sp.saetDomaene('b.wi24rd.com', true);
  tjek('Q6b anvend(): guidens titel og knappen bliver', d.getElementById('hjaelp-h').textContent === h && h !== 'Hjælp' && d.getElementById('dom-hjaelp').textContent === knap, d.getElementById('hjaelp-h').textContent);
  R.hjaelp(false);
  R.hjaelp(true);
  const ro = side('#overblik', DYK({ overblik: [raekke('a', 'a.wi24rd.com', 'groen', { sidst_hentet: NU_ISO })] }));
  await sov(60);
  ro.w.hmSprog.saetDomaene('a.wi24rd.com', false);
  tjek('Q6b anvend(): "Hvorfor er alt roligt?" bliver stående', ro.d.getElementById('dom-hjaelp').textContent === 'Hvorfor er alt roligt? →', ro.d.getElementById('dom-hjaelp').textContent);
  tjek('Q6b hjælpen får sin egen titel igen', d.getElementById('hjaelp-h').textContent === 'Hjælp');
  tjek('Q6b "af sig selv": m3 og dataHistNote på begge sprog', Sp.T.m3.includes('Intet slettes af sig selv') && e.w.hmSprog.T.m3.includes('Nothing is deleted by itself') && Sp.T.dataHistNote.endsWith('sletter intet af sig selv.') && e.w.hmSprog.T.dataHistNote.endsWith('deletes nothing by itself.'));
  tjek('J7 rutinen står uden for ordlisten, og bogen har en post', !Sp.ordliste.some(o => /bot fight|linkmaze/i.test(o.join())) && Sp.bogIds.includes('rutine') && Sp.rutineNavn('linkMaze') === 'Labyrinten (falske sider til robotter)' && e.w.hmSprog.rutineNavn('botFight') === 'Bot Fight Mode (known bot patterns)');
  tjek('J9 Overblikkets fodnote siger, at fælde-links ikke tæller', Sp.T.fodnote.includes('fælde-links') && e.w.hmSprog.T.fodnote.includes('trap links'));
  tjek('S sikkerhedsporten: dyk er tilladt og står i build.rs', cap.permissions.includes('allow-dyk') && /"dyk",/.test(fs.readFileSync(path.join(repo, 'src-tauri', 'build.rs'), 'utf8')));
}

// ── Alt med data bag er klikbart ──
const qs = e => new URLSearchParams(e.getAttribute('data-dyk'));
const klik = async (s, e, ms = 40) => { e.dispatchEvent(new s.w.MouseEvent('click', { bubbles: true, cancelable: true })); await sov(ms); };

// Overblik: tallene, Hvorfor, domslinjen og rækkens tomme del.
{
  const nat = raekke('z4', 'nat.wi24rd.com', 'gul', { opdateres: true, i_gaar_dag: '2026-11-02', status_dag: '2026-11-01', uge_fra: '2026-10-26', ugen_foer_fra: '2026-10-19', ugen_foer_til: '2026-10-25', i_gaar: 5, hele7: 9, hele30: 0, angreb_i_gaar: 4, angreb_uge: 7, angreb_ugen_foer: 0, sidst_hentet: NU_ISO,
    angreb_i_gaar_top: [{ navn: 'Wordpress', n: 4, regler: ['Wordpress - A', 'Wordpress - B'] }], angreb_fordeling: { blokeret: 4 } });
  const rk = [raekke('z1', 'wi24rd.com', 'roed', { i_gaar: 1260, hele7: 3408, hele30: 8200, angreb_i_gaar: 30, angreb_snit: 12.4, sidst_hentet: NU_ISO, angreb_uge: 45, angreb_ugen_foer: 36,
    angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 30, regler: [REACT] }], angreb_fordeling: { blokeret: 30 } }), nat, OVERBLIK[2]];
  const s = side('#overblik', DYK({ overblik: rk })), { d, w } = s;
  await sov(80);
  const rows = [...d.querySelectorAll('#overblik-raekker tr')], knapper = r => [...r.querySelectorAll('.tal-knap')];
  tjek('Klik, Overblik: fem tal er knapper', knapper(rows[0]).length === 5, String(knapper(rows[0]).length));
  const m = knapper(rows[0]).map(qs);
  tjek('Klik, Overblik: Afvist i går åbner dagen', m[0].get('dag') === '2026-11-02' && m[0].get('zone') === 'z1' && m[0].get('navn') === 'wi24rd.com' && !m[0].has('slags'), m[0].toString());
  tjek('Klik, Overblik: 7 dage og 30 dage åbner perioden', m[1].get('periode') === '7' && !m[2].has('periode') && !m[2].has('dag'), m[1] + ' ' + m[2]);
  tjek('Klik, Overblik: Angreb denne uge = statusugen, kun angreb', m[3].get('fra') === '2026-10-27' && m[3].get('til') === '2026-11-02' && m[3].get('slags') === 'angreb', m[3].toString());
  tjek('Klik, Overblik: Sidste uge = ugen før, kun angreb', m[4].get('fra') === '2026-10-20' && m[4].get('til') === '2026-10-26' && m[4].get('slags') === 'angreb', m[4].toString());
  const n = knapper(rows[1]).map(qs);
  tjek('Klik, Overblik om natten: ugen følger statusdøgnet, Afvist i går gør ikke', n[0].get('dag') === '2026-11-02' && n[2].get('til') === '2026-11-01' && n[2].get('fra') === '2026-10-26', n.join(' | '));
  tjek('Klik, Overblik: et tal med 0 er ikke en knap', knapper(rows[1]).length === 3 && celler(rows[1])[4].textContent === '0' && !celler(rows[1])[4].querySelector('button') && !celler(rows[1])[6].querySelector('button'), celler(rows[1]).map(c => c.textContent).join('|'));
  tjek('Klik, Overblik: grå række har ingen knapper', !rows[2].querySelector('[data-dyk]') && celler(rows[2]).slice(2, 7).every(c => c.textContent === '–'));
  tjek('Ét klikmål: Hvorfor-teksten som helhed er ikke et mål', !rows[0].querySelector('.hv-tekst [data-dyk]') && !rows[0].querySelector('.hv-knap'));
  tjek('Klik, Overblik: Hvorfor uden tal er tekst', !rows[2].querySelector('td.hv button'));
  tjek('Klik, Overblik: aria-label siger domæne, tal og handling', knapper(rows[0])[0].getAttribute('aria-label') === 'wi24rd.com, afvist i går: 1.260. Åbn.', knapper(rows[0])[0].getAttribute('aria-label'));
  tjek('Overblik-tabel: streger: grupperne .sep, hver talkolonne .tal-kol, også i overskriften', [...d.querySelectorAll('#overblik-tabel thead tr:last-child th')].every(t => t.matches('.tal-kol')) && [...d.querySelectorAll('#overblik-tabel thead tr:last-child th.sep')].length === 2 && celler(rows[0]).slice(2, 7).every(c => c.matches('.tal-kol')) && celler(rows[0]).filter(c => c.matches('.sep')).length === 2);
  tjek('Overblik-tabel: grupperne spænder over 3 og 2 kolonner', [...d.querySelectorAll('th.gruppe')].map(t => t.colSpan + t.getAttribute('scope')).join() === '3colgroup,2colgroup' && d.querySelectorAll('#overblik-tabel colgroup').length === 4);
  const css = udenKommentarer(ui('assets/app.css'));
  tjek('Overblik-tabel: CSS: svag streg med --color-border, tydelig med --color-border-strong, fast bredde', /\.tabel \.tal-kol \{[^}]*width: calc[^}]*min-width: calc[^}]*border-left: 1px solid var\(--color-border\)/.test(css) && /\.tabel \.sep, \.tabel \.tal-kol\.sep \{ border-left: 2px solid var\(--color-border-strong\)/.test(css));
  // Fokus bliver på tallet, når tabellen tegnes om.
  knapper(rows[0])[3].focus();
  w.hmRamme.udsend('raekker', rk);
  tjek('Klik, Overblik: fokus bliver på tallet, når køen tegner om', d.activeElement.dataset.fokus === 'uge' && d.activeElement.closest('tr').dataset.zone === 'z1', d.activeElement.outerHTML.slice(0, 90));
  const ny = [...d.querySelectorAll('#overblik-raekker tr')];
  await klik(s, knapper(ny[0])[3]);
  tjek('Klik, Overblik: tallet åbner dyk, ikke Angreb', synlige(d).join() === 'dyk' && sidsteDyk(s).slags === 'angreb' && sidsteDyk(s).periode.spaend.fra === '2026-10-27', synlige(d).join() + ' ' + JSON.stringify(sidsteDyk(s)));
  w.history.back(); await sov(120);
  tjek('Klik, Overblik: tallet er ét trin, så tilbage er Overblik, ikke Angreb', w.location.hash === '#overblik' && synlige(d).join() === 'overblik', w.location.hash);
  const row = d.querySelector('#overblik-raekker tr');
  await klik(s, row.querySelector('th'));
  tjek('Klik, Overblik: rækkens tomme del åbner stadig Angreb', synlige(d).join() === 'vaernet' && w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com', w.location.hash);
  w.history.back(); await sov(120);
  const dom = d.querySelector('#dom-p [data-gaa]');
  tjek('Klik, domslinjen: domænenavnet er en knap med mål', dom && dom.textContent === 'wi24rd.com' && dom.getAttribute('aria-label') === 'Åbn Angreb for wi24rd.com', dom && dom.outerHTML.slice(0, 120));
  await klik(s, dom.querySelector('strong'));
  tjek('Klik, domslinjen: åbner domænets Angreb', w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com', w.location.hash);
  w.history.back(); await sov(120);
  // Guiden: overskriften er et mål, og skuffen lukker.
  d.getElementById('dom-hjaelp').click(); await sov(20);
  const h4 = [...d.querySelectorAll('#hjaelp-guide h4 [data-dyk]')];
  tjek('Klik, guiden: én regel åbner reglen, flere regler dagens angreb', qs(h4[0]).get('regel') === REACT && !qs(h4[0]).has('dag') && qs(h4[1]).get('dag') === '2026-11-01' && qs(h4[1]).get('slags') === 'angreb' && !qs(h4[1]).has('regel'), h4.map(x => x.getAttribute('data-dyk')).join(' | '));
  await klik(s, h4[0]);
  tjek('Klik, guiden: et klik lukker skuffen og åbner dyk', d.getElementById('hjaelp').hidden && synlige(d).join() === 'dyk', synlige(d).join());
  // Køpillen.
  s.emit('hentekoe', { zone: 'z1', navn: 'wi24rd.com', doegn_faerdige: 1, doegn_i_alt: 9, zoner_faerdige: 0, zoner_i_alt: 3, zoner_fejl: 0, tilstand: 'henter' });
  const pille = d.getElementById('koe');
  tjek('Klik, køpillen: en knap til Overblik, og status står uden for knappen', pille.tagName === 'BUTTON' && pille.getAttribute('data-gaa') === 'overblik' && pille.getAttribute('aria-label').endsWith('Åbn Overblik.') && !pille.querySelector('[role=status]') && d.getElementById('koe-tekst').textContent.startsWith('Henter'), pille.outerHTML.slice(0, 140));
  await klik(s, pille);
  tjek('Klik, køpillen: åbner Overblik', w.location.hash === '#overblik' && synlige(d).join() === 'overblik', w.location.hash);
}

// Siden sidst: kortet, chippen og samlelinjerne.
{
  const rk = [{ ...OVERBLIK[0], status: 'roed' }, OVERBLIK[1], OVERBLIK[2]];
  const s = side('#siden-sidst', DYK({ overblik: rk, siden_sidst: { vis: false, nye: [], raekker: rk }, siden_sidst_set: null })), { d, w } = s;
  await sov(100);
  const aabn = d.querySelector('.hm-siden-aabn');
  tjek('Klik, Siden sidst: kortets knap åbner domænets Angreb', aabn && aabn.getAttribute('data-gaa') === 'vaernet?zone=z1&navn=wi24rd.com' && aabn.getAttribute('aria-label') === 'Åbn Angreb for wi24rd.com', aabn && aabn.outerHTML.slice(0, 120));
  tjek('Klik, Siden sidst: tooltippen med CVE-numre flyttes med op på knappen', aabn.title === `wi24rd.com · ${d.querySelector('.hm-siden small').title}` && aabn.title.includes('CVE-2025-55182'), aabn.title);
  const chip = d.querySelector('.hm-siden .chip-knap');
  tjek('Klik, Siden sidst: chippen er en knap med mål', chip && chip.getAttribute('aria-label').includes('Hvad gør jeg?'), chip && chip.outerHTML.slice(0, 120));
  chip.click(); await sov(20);
  tjek('Klik, Siden sidst: chippen åbner guiden for domænet', !d.getElementById('hjaelp').hidden && d.querySelector('#hjaelp-guide .g-kort').dataset.zone === 'z1' && synlige(d).join() === 'siden-sidst');
  w.hmRamme.hjaelp(false);
  const linjer = [...d.querySelectorAll('.hm-siden-linje')];
  tjek('Klik, Siden sidst: samlelinjerne går til Overblik', linjer.length === 2 && linjer.every(l => l.getAttribute('data-gaa') === 'overblik'), linjer.map(l => l.textContent).join(' | '));
  await klik(s, linjer[0]);
  tjek('Klik, Siden sidst: samlelinjen åbner Overblik', w.location.hash === '#overblik', w.location.hash);
}

// Angreb: alt med data bag.
{
  const kat = [{ id: 'wordpress', n: 12, stier: [{ sti: '/wp-login.php', n: 9 }, { sti: '[hemmeligheder]', n: 3 }] }];
  const kort = [
    kortRegel({ navn: REACT, produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', n7: 6, n30: 40, foerst: '2026-10-05', sidst: '2026-11-02', stier: [{ sti: '/wp-config%2ephp', n: 3 }], lande: [{ kode: 'PL', n: 30 }, { kode: 'T1', n: 2 }] }),
    kortRegel({ navn: 'Ny regel', produkt: '', type: '', cve: null, n7: 0, n30: 0, foerst: '2026-11-03', sidst: '2026-11-03', stier: [], lande: [] }, { idag: 3, h7: 0, h30: 0 }),
  ];
  const landene = Array.from({ length: 44 }, (_, i) => ({ kode: ['PL', 'US', 'DE', 'NL', 'SG', 'FR', 'GB', 'SE', 'NO', 'FI', 'DK', 'ES', 'IT', 'BR', 'IN', 'JP', 'CN', 'RU', 'UA', 'RO', 'BG', 'TR', 'ZA', 'NG', 'EG', 'AU', 'NZ', 'CA', 'MX', 'AR', 'CL', 'CO', 'ID', 'TH', 'VN', 'KR', 'PK', 'SA', 'AE', 'IL', 'IE', 'PT', 'AT', 'CH'][i], n: 90 - i }));
  const data = vaernUd({ h_regler: kort, forsoeg: { i7: 999999, kategorier: [] }, h_forsoeg: { i7: 12, kategorier: kat }, fordeling: { d30: { blokeret: 20, udfordret: 0, labyrint: 3, andet: 2, faelder: 7 }, i_gaar: { blokeret: 5, udfordret: 1, labyrint: 0, andet: 0, faelder: 0 } },
    h_lande: [...landene, { kode: 'T1', n: 2 }, { kode: 'XX', n: 1 }], h_colos: [{ kode: 'WAW', n: 60 }, { kode: 'FRA', n: 30 }, { kode: 'UDI', n: 1 }], h_buer: [{ land: 'PL', colo: 'WAW', n: 60 }, { land: 'US', colo: 'FRA', n: 30 }], h_buer_i_alt: 212,
    h_dage: dage.map((x, i) => ({ ...x, n: i === 3 ? 0 : x.n + 1 })) });
  const zone = '#vaernet?zone=z1&navn=wi24rd.com';
  const s = side(zone, DYK({ vaern_cache: data, vaern_oversigt: data, koe_prioritet: null })), { d, w } = s;
  await sov(120);
  const q = sel => d.querySelector('#vagt ' + sel), alle = sel => [...d.querySelectorAll('#vagt ' + sel)];
  const fl = alle('.vg-fliser > *');
  tjek('Klik, Angreb: i går åbner dagen, og "i dag hidtil" har sit eget mål', qs(fl[0].querySelector('.dyk-flise-aabn')).get('dag') === '2026-11-03' && qs(fl[0].querySelector('small [data-dyk]')).get('dag') === '2026-11-03' && fl[0].querySelector('small').textContent === 'i dag hidtil: 58', fl[0].outerHTML.slice(0, 260));
  tjek('Klik, Angreb: 7 og 30 dage åbner perioden', qs(fl[1]).get('periode') === '7' && !qs(fl[2]).has('periode'), fl[1].getAttribute('data-dyk'));
  tjek('Klik, Angreb: Snusere ruller til Hvad de ledte efter', fl[3].getAttribute('data-rul') === 'vg-forsoeg' && fl[3].querySelector('b').textContent === '12' && fl[3].getAttribute('aria-label') === 'Snusere, der intet fandt (7 dage): 12. Gå til listen.', fl[3].outerHTML.slice(0, 160));
  await klik(s, fl[3], 10);
  tjek('Klik, Angreb: rulningen flytter fokus til overskriften og er intet trin', d.activeElement.id === 'vg-forsoeg-h' && w.location.hash === zone, d.activeElement.tagName);
  const nul = side(zone, DYK({ vaern_cache: vaernUd({ i_gaar: 0, i_dag_hidtil: 0, hele7: 0, hele30: 0, forsoeg: { i7: 0, kategorier: [] } }), vaern_oversigt: vaernUd({ i_gaar: 0, i_dag_hidtil: 0, hele7: 0, hele30: 0, forsoeg: { i7: 0, kategorier: [] } }), koe_prioritet: null }));
  await sov(120);
  tjek('Klik, Angreb: fliser med 0 er ikke klikbare', !nul.d.querySelector('.vg-fliser [data-dyk], .vg-fliser [data-rul]'), nul.d.querySelector('.vg-fliser').innerHTML.slice(0, 200));
  const ford = alle('.vg-fordeling [data-dyk]').map(e => [e.textContent, qs(e).get('handling'), qs(e).get('dag')]);
  tjek('Klik, Angreb: "heraf" i går: blokeret og udfordret med dagen; 0 er tekst', JSON.stringify(ford.slice(0, 2)) === '[["5 blokeret","blokeret","2026-11-03"],["1 udfordret","udfordret","2026-11-03"]]' && q('.vg-fordeling li').textContent === 'I går: heraf 5 blokeret · 1 udfordret · 0 lokket ind i labyrinten', JSON.stringify(ford) + q('.vg-fordeling li').textContent);
  tjek('Klik, Angreb: "heraf" 30 dage: uden dag, med andet og fælde-linjen', ford.slice(2).map(x => x[1]).join() === 'blokeret,labyrint,andet,faelder' && ford.slice(2).every(x => !x[2]), JSON.stringify(ford.slice(2)));
  const svg = q('svg.vg-kort'), ringe = [...svg.querySelectorAll('circle.vg-land')], ruder = [...svg.querySelectorAll('path.vg-colo')];
  tjek('Klik, kortet: ét Tab-stop, ringe og ruder åbner land og datacenter', svg.getAttribute('role') === 'group' && svg.querySelectorAll('[tabindex="0"]').length === 1 && qs(ringe[0]).get('land') === 'PL' && qs(ruder[0]).get('colo') === 'WAW' && !d.querySelector('.vg-kortramme').hasAttribute('tabindex'), svg.querySelectorAll('[tabindex="0"]').length + ' ' + d.querySelector('.vg-kortramme').outerHTML.slice(0, 80));
  tjek('Klik, kortet: alle lande med en plads står som mærker, ikke kun top 15', ringe.length === 44, String(ringe.length));
  tjek('Klik, kortet: buen åbner land og datacenter', qs(svg.querySelector('path.vg-bue[data-dyk]')).has('land') && qs(svg.querySelector('path.vg-bue[data-dyk]')).has('colo'));
  const lag = [...svg.children].filter(x => x.tagName === 'g'), iLag = sel => lag.findIndex(g => g.querySelector(sel));
  tjek('Klik, kortet: klikfladerne ligger under alle mærker', iLag('.dyk-hit-bue') < iLag('circle.dyk-hit') && iLag('circle.dyk-hit') < iLag('.vg-bue:not(.dyk-hit)') && iLag('.vg-bue:not(.dyk-hit)') < iLag('.vg-land') && iLag('.vg-land') < iLag('.vg-colo'), JSON.stringify(lag.map(g => g.firstElementChild?.getAttribute('class'))));
  const hit = svg.querySelector('circle.dyk-hit');
  tjek('Klik, kortet: en klikflade er mindst 24 px ved 600 px bredde', 2 * Number(hit.getAttribute('r')) * 600 / 1000 >= 24, hit.getAttribute('r'));
  tjek('Klik, kortet: klikfladen er ingen tastaturstop', !svg.querySelector('.dyk-hit[tabindex]'));
  const leg = alle('.vg-legende li').map(l => l.textContent);
  tjek('Klik, legenden: T1, ukendt og UDI nævnes, og buerne er forklaret', leg.includes('Uden plads på kortet: Tor-netværket, Ukendt land, UDI. De står i listerne.') && leg.includes('Buerne er de 2 største af 212.'), leg.join(' | '));
  const lande = alle('.vg-tre > div')[0];
  tjek('Klik, listen: de første 8 fremme, resten bag "Vis alle (46)", T1 med', lande.querySelector('.vg-liste').children.length === 8 && lande.querySelector('.dyk-flere summary').textContent === 'Vis alle (46)' && lande.querySelector('.dyk-flere').textContent.includes('Tor-netværket'), lande.querySelector('.dyk-flere summary')?.textContent);
  tjek('Klik, listen: UDI står i Afvist i, selv om den ikke kan stå på kortet', alle('.vg-tre > div')[2].textContent.includes('UDI'));
  lande.querySelector('.dyk-flere').open = true;
  s.w.hmRamme.udsend('hentekoe', {});
  const dyk0 = q('svg.dyk-soejler'), soejler = [...dyk0.querySelectorAll('[data-dyk]')];
  tjek('Klik, søjlerne: dage med tal er mærker, dagen med 0 er ikke, ét Tab-stop', soejler.length === 29 && dyk0.getAttribute('role') === 'group' && dyk0.querySelectorAll('[tabindex="0"]').length === 1 && qs(soejler[0]).get('dag') === dage[0].dag && !d.querySelector('.vg-soejler-rul').hasAttribute('tabindex'), soejler.length + ' ' + dyk0.querySelectorAll('[tabindex="0"]').length);
  const bredde = [...w.hmTegn.ark.cssRules].find(r => r.selectorText === '.' + [...q('.dyk-diagram').classList].find(c => c.startsWith('vg-t')));
  tjek('Klik, søjlerne: mindst 24 px pr. dag (bredden står som regel)', bredde && bredde.cssText.includes('min-width') && /30 \* 24px/.test(bredde.cssText), bredde && bredde.cssText);
  const f0 = dyk0.querySelector('[tabindex="0"]');
  f0.focus();
  f0.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true, cancelable: true }));
  tjek('Klik, søjlerne: piletaster flytter Tab-stoppet', d.activeElement !== f0 && d.activeElement.getAttribute('tabindex') === '0' && f0.getAttribute('tabindex') === '-1' && dyk0.querySelectorAll('[tabindex="0"]').length === 1);
  const ring = svg.querySelector('[tabindex="0"]');
  ring.focus();
  const besoegt = new Set();
  for (let i = 0; i < 60; i++) { d.activeElement.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true })); besoegt.add(d.activeElement); }
  tjek('Klik, kortet: piletasterne går gennem mærkerne og aldrig klikfladerne', ![...besoegt].some(e => e.matches('.dyk-hit')) && [...besoegt].some(e => e.matches('.vg-colo')) && [...besoegt].some(e => e.matches('.vg-land')), [...besoegt].filter(e => e.matches('.dyk-hit')).length + ' flader');
  const top = alle('.vg-tavle-top [data-dyk]');
  tjek('Klik, "i alt" og "flest": 30 dage og den travleste dag', top.length === 2 && !qs(top[0]).has('dag') && qs(top[1]).get('dag') === dage[29].dag && q('.vg-tavle-top b').textContent === top[0].textContent, top.map(e => e.getAttribute('data-dyk')).join(' | '));
  const tabel = q('.vg-tal table'), rk = [...tabel.querySelectorAll('tbody tr')];
  tjek('Klik, "Vis tallene": dag og tal åbner dagen, 0 er tekst, trafik er tekst', rk.length === 30 && qs(rk[0].querySelector('th [data-dyk]')).get('dag') === dage[29].dag && qs(rk[0].querySelector('td [data-dyk]')).get('dag') === dage[29].dag && !rk[26].querySelector('[data-dyk]') && !rk[0].children[2].querySelector('button'), rk[26].innerHTML.slice(0, 120));
  tjek('Søjler: "Vis tallene": talkolonnerne har streg og fast bredde, også i overskriften', [...tabel.querySelectorAll('thead th.tal-kol')].length === 2 && rk[0].querySelectorAll('td.tal-kol').length === 2 && /\.vg-tabel \.tal-kol \{[^}]*width: calc[^}]*border-left: 1px solid var\(--color-border\)/.test(udenKommentarer(ui('assets/vagt.css'))));
  const regel = alle('.vg-regel')[0], sti = regel.querySelector('code').closest('[data-dyk]');
  tjek('Klik, regelkortet: stien åbner regel + sti (rå værdi overlever adressen)', qs(sti).get('regel') === REACT && qs(sti).get('sti') === '/wp-config%2ephp', sti.getAttribute('data-dyk'));
  const lk = [...regel.querySelectorAll('.vg-regel-l')][1].querySelectorAll('[data-dyk]');
  tjek('Klik, regelkortet: landene åbner regel + land, T1 i klarsprog', lk.length === 2 && qs(lk[0]).get('land') === 'PL' && qs(lk[0]).get('regel') === REACT && qs(lk[1]).get('land') === 'T1' && lk[1].textContent === 'Tor-netværket', [...lk].map(e => e.textContent).join());
  const nyK = alle('.vg-regel')[1];
  tjek('Klik, regelkort set kun i dag: "i dag hidtil", og kortet åbner dagen i dag', nyK.querySelector('.vg-regel-n span').textContent === 'i dag hidtil: 3' && qs(nyK.querySelector('.vg-regel-aabn')).get('dag') === '2026-11-03' && nyK.querySelector('.vg-regel-aabn').getAttribute('aria-label').endsWith('3 i dag hidtil'), nyK.textContent);
  const fp = alle('.vg-protokol li')[0], fb = fp.querySelector('h3 [data-dyk]');
  tjek('Klik, Hvad de ledte efter: kategorien åbner forsøget på 7 dage', qs(fb).get('forsoeg') === 'wordpress' && qs(fb).get('periode') === '7' && fb.textContent === 'WordPress', fb.getAttribute('data-dyk'));
  const fs = [...fp.querySelectorAll('.vg-p-stier [data-dyk]')];
  tjek('Klik, Hvad de ledte efter: stierne åbner forsøget på 7 dage', fs.length === 2 && qs(fs[0]).get('forsoeg') === '/wp-login.php' && qs(fs[1]).get('forsoeg') === '[hemmeligheder]' && qs(fs[0]).get('periode') === '7', fs.map(e => e.getAttribute('data-dyk')).join(' | '));
  tjek('Klik, Angreb: ingen style= i det tegnede, heller ikke på protokollen', !d.getElementById('vagt').innerHTML.includes('style=') && fp.querySelector('i').className.startsWith('vg-t'), fp.querySelector('i').outerHTML);
  const r0 = q('#vg-seneste tbody tr'), bs = [...r0.querySelectorAll('[data-dyk]')].map(qs);
  tjek('Klik, "De seneste": tid, regel, land, sti, datacenter og metode er mål', bs.length === 6 && bs[0].get('dag') === '2026-11-03' && bs[1].get('regel') === 'R' && bs[2].get('land') === 'US' && bs[3].get('sti') === '/' && bs[4].get('colo') === 'EWR' && bs[5].get('metode') === 'GET', bs.join(' | '));
  const sam = side('#vaernet?samlet=1', DYK({ vaern_samlet_cache: { domaener: 2, med_tal: 2, valgte: [], vaern: data } }));
  await sov(120);
  const dc = sam.d.querySelector('#vg-seneste tbody td [data-gaa]');
  tjek('Klik, "De seneste" samlet: domænet åbner domænets Angreb; ukendt domæne er tekst', dc && dc.getAttribute('data-gaa') === 'vaernet?zone=z1&navn=wi24rd.com' && sam.d.querySelectorAll('#vg-seneste tbody tr')[1].children[1].querySelectorAll('button').length <= 1, (dc && dc.outerHTML.slice(0, 120)) + ' ' + sam.w.location.hash + sam.d.getElementById('vagt').textContent.slice(0, 200));
  tjek('Klik, samlet: målene har samlet=1, og i går-flisen åbner dagen', sam.d.querySelector('.dyk-flise-aabn').getAttribute('data-dyk') === 'samlet=1&dag=2026-11-03', sam.d.querySelector('.dyk-flise-aabn').getAttribute('data-dyk'));
  tjek('Klik, ingen data-dyk uden værdi og ingen med "undefined"', ![...d.querySelectorAll('#vagt [data-dyk]')].some(e => /undefined|=null/.test(e.getAttribute('data-dyk'))));
}

// Dyk: det nye.
{
  const dag = d => ({ ...d });
  const s = side('#dyk?' + dykQ({ land: 'PL' }), DYK({ dyk: a => dykSvar({ top: { ...dykSvar().top, regel: [{ v: REACT, n: 150, art: 'angreb' }, { v: 'min-regel: max 100 kald pr. minut', n: 4, art: 'egen' }, { v: 'botFight', n: 48, art: 'rutine' }] }, antal: { ...dykSvar().antal, land: 30, colo: 25 }, buer_i_alt: 90, regler: [{ navn: 'min-regel: max 100 kald pr. minut', produkt: '', type: '', cve: null, art: 'egen', h30: 4, h7: 0, idag: 0, faelder30: 0, foerst: '', sidst: '', stier: [], lande: [] }] }) })), { d, w } = s;
  await sov(100);
  const regler = d.querySelector('#dyk-rod .vg-regel');
  tjek('Q3.5 egne regler står med kort navn på regelkortet', regler.querySelector('.vg-produkt').textContent === 'Din egen regel »min-regel«', regler.textContent);
  const hdag = d.querySelector('.dyk-flise-to small [data-dyk]');
  tjek('Klik, dyk: "i dag hidtil" åbner dagen i dag', qs(hdag).get('dag') === '2026-11-04' && qs(hdag).get('land') === 'PL', hdag.getAttribute('data-dyk'));
  const legende = [...d.querySelectorAll('.vg-legende li')].map(l => l.textContent);
  tjek('Klik, dyk: legenden siger, hvad kortet ikke viser', legende.some(t => t.startsWith('Uden plads på kortet: Tor-netværket')) && legende.includes('+ 22 lande med få angreb står ikke på kortet.') && legende.includes('+ 23 datacentre med få angreb står ikke på kortet.') && legende.includes('Buerne er de 2 største af 90.'), legende.join(' | '));
  const dato = [...d.querySelectorAll('.dyk-dato')];
  tjek('Q3.7 først og sidst set står hver for sig og kan ombrydes', dato.length === 2 && dato[0].querySelector('span').textContent === 'Først set' && /\.dyk-dato \{[^}]*flex-wrap: wrap/.test(ui('assets/vagt.css')) && /\.dyk-dato \.link \{[^}]*white-space: nowrap/.test(ui('assets/vagt.css')));
  tjek('Klik, dyk: søjlerne ruller i deres egen ramme', d.querySelector('#dyk-rod .vg-soejler-rul .dyk-diagram'));
  const an = side('#dyk?' + dykQ({ fra: '2026-10-27', til: '2026-11-02', slags: 'angreb' }), DYK());
  await sov(80);
  tjek('Q3.4 Kun angreb hedder "Angreb", ikke "Afvist"', an.d.querySelector('.vg-flise[aria-current]').textContent.startsWith('198Angreb 27. oktober – 2. november'), an.d.querySelector('.vg-flise[aria-current]').textContent);
  tjek('Q3.4 adressen fra Overblik giver et spænd og kun angreb', sidsteDyk(an).periode.spaend.til === '2026-11-02' && sidsteDyk(an).slags === 'angreb', JSON.stringify(sidsteDyk(an)));
  const sam = side('#overblik', DYK({ dyk: () => dykSvar({ enkelte: [{ t: '2026-11-03T21:00:00Z', regel: REACT, cve: null, land: 'PL', sti: '/', colo: 'WAW', metode: 'POST', domaene: 'wi24rd.com' }, { t: '2026-11-03T20:00:00Z', regel: REACT, cve: null, land: 'PL', sti: '/', colo: 'WAW', metode: 'POST', domaene: 'ukendt.wi24rd.com' }] }) }));
  await sov(80);
  sam.w.hmRamme.gaaTil('dyk', 'samlet=1&land=PL');
  await sov(100);
  const dk = sam.d.querySelector('#dyk-rod table tbody td:nth-child(2) [data-dyk]');
  tjek('Klik, dyk samlet: domænet åbner samme filter på det domæne; ukendt domæne er tekst', dk && qs(dk).get('zone') === 'z1' && qs(dk).get('land') === 'PL' && !qs(dk).has('samlet') && !sam.d.querySelectorAll('#dyk-rod table tbody tr')[1].children[1].querySelector('button'), dk && dk.getAttribute('data-dyk'));
}

// Indstillinger: chippen, Pr. domæne, Historik og Domæner med tal.
{
  const svar = x => DYK({ data_status: { bytes: 8e6, foerste: '2026-10-04', sidste: '2026-11-03', domaener: 3 }, zones_cache: { zoner: OVERBLIK, hentet: NU_ISO }, ...x });
  const s = side('#indstillinger', svar()), { d, w } = s;
  await sov(100);
  const chip = d.getElementById('forb-chip'), hist = d.getElementById('data-historik'), dom = d.getElementById('data-domaener');
  tjek('Klik, Indstillinger: chippen går til Overblik', chip.getAttribute('data-gaa') === 'overblik' && !chip.disabled && chip.getAttribute('aria-label').endsWith('Åbn.'), chip.outerHTML.slice(0, 140));
  tjek('Klik, Indstillinger: Historik åbner det samlede dyk over hele historikken', hist.getAttribute('data-dyk') === 'samlet=1&periode=alt' && !hist.disabled, hist.outerHTML.slice(0, 140));
  tjek('Klik, Indstillinger: Domæner med tal går til Overblik', dom.getAttribute('data-gaa') === 'overblik' && !dom.disabled, dom.outerHTML.slice(0, 140));
  const pd = d.querySelector('#pr-domaene-liste [data-gaa]');
  tjek('Klik, Indstillinger: domænet under Pr. domæne åbner dets Angreb', pd.getAttribute('data-gaa').startsWith('vaernet?zone=') && pd.getAttribute('aria-label').startsWith('Åbn Angreb for'), pd.outerHTML.slice(0, 140));
  w.hmRamme.tilstand.sat = false;
  w.hmRamme.udsend('token', false);
  tjek('Klik, Indstillinger: glemmes nøglen, mister chippen både mål og aria-label', chip.disabled && !chip.hasAttribute('data-gaa') && !chip.hasAttribute('aria-label'), chip.outerHTML.slice(0, 120));
  const tom = side('#indstillinger', svar({ token_status: { sat: false }, data_status: { bytes: 0, foerste: null, sidste: null, domaener: 0 } }));
  await sov(100);
  const c2 = tom.d.getElementById('forb-chip'), h2 = tom.d.getElementById('data-historik'), d2 = tom.d.getElementById('data-domaener');
  tjek('Klik, Indstillinger: uden data er knapperne slået fra og har intet mål', c2.disabled && h2.disabled && d2.disabled && !c2.hasAttribute('data-gaa') && !h2.hasAttribute('data-dyk') && !d2.hasAttribute('data-gaa') && !c2.hasAttribute('aria-label'), c2.outerHTML.slice(0, 120));
  tjek('Klik, CSS: en slået-fra knap har hverken pointer eller hover', /button\.chip-knap:disabled \{ cursor: default; \}/.test(ui('assets/app.css')) && /button\.chip-knap:hover:not\(:disabled\)/.test(ui('assets/app.css')) && /\.felt-knap:hover:not\(:disabled\)/.test(ui('assets/app.css')));
}

// Menulinjen og CSS'en.
{
  const agent = fs.readFileSync(path.join(repo, 'src-tauri/src/agent.rs'), 'utf8'), lib = fs.readFileSync(path.join(repo, 'src-tauri/src/lib.rs'), 'utf8');
  tjek('Klik, menulinjen: statuslinjen er slået til', /MenuItem::with_id\(app, STATUS, &tekst, true,/.test(agent) && /pub const STATUS/.test(agent));
  tjek('Q3.2 "Først set" over hele historikken for ét domæne og for det samlede', (lib.match(/ud\.lister\.med_historik\(&lager\.fw_datoer\(/g) || []).length === 2);
  tjek('Klik, menulinjen: statuslinjen åbner vinduet på Overblik', /agent::STATUS => \{\s*let _ = aabn_vindue\(app\);\s*let _ = app\.emit\("visning", "overblik"\);/.test(lib));
  const app = udenKommentarer(ui('assets/app.css'));
  tjek('Klik, rækker og sidebjælke har pointer; et slået-fra punkt har ikke', /\.tabel tbody tr \{ cursor: pointer; \}/.test(app) && /\.nav a \{[^}]*cursor: pointer/.test(app) && /\.nav a\[aria-disabled="true"\] \{[^}]*cursor: default/.test(app));
}

// Logoet.
{
  const png = f => { const b = fs.readFileSync(path.join(repo, f)); return { b, w: b.readUInt32BE(16), h: b.readUInt32BE(20), farve: b[25], n: b.length }; };
  const kilde = png('design/brand/hm-ravne-puls.png');
  tjek('L kilden ligger i design/brand uden mellemrum', kilde.w === 1254 && kilde.h === 1254 && !fs.existsSync(path.join(repo, 'White raven duo heartbeat logo.png')));
  const html = ui('index.html'), app = ui('assets/app.css');
  tjek('L de gamle ravne er væk (ravne.svg, inline-SVG, flise-gradient)', !fs.existsSync(path.join(repo, 'ui/assets/ravne.svg')) && !/id="ravn|#ravne|flise-fyld|flise-felt/.test(html) && !/flise-felt|\.ravne|svg \.gren/.test(app));
  for (const s of [38, 56, 76, 114, 168]) { const p = png(`ui/assets/flise-${s}.png`); tjek(`L flisen ${s} px: kvadratisk RGBA og lille`, p.w === s && p.h === s && p.farve === 6 && p.n < 40000, `${p.w}×${p.h} ${p.n} B`); }
  tjek('L båndet: flisen ved 1×, 2× og 3× af 38 px', /<img class="flise" src="assets\/flise-76\.png" srcset="assets\/flise-38\.png 38w, assets\/flise-76\.png 76w, assets\/flise-114\.png 114w" sizes="38px" alt="">/.test(html));
  const m = png('ui/assets/hm-ravne.png');
  tjek('L masken: gråtone + alfa og lille', m.farve === 4 && m.n < 30000, `${m.farve} ${m.n} B`);
  tjek('L opsætningen: masken med token, både lys og mørk', /\.tom-ravn \{[^}]*background: var\(--color-text\);[^}]*-webkit-mask: url\(hm-ravne\.png\)[^}]*mask: url\(hm-ravne\.png\)/.test(app) && (ui('assets/tokens.css').match(/--color-logo: var\(--raw-color-white\)/g) || []).length === 2 && /--color-logo: var\(--raw-color-blue-500\)/.test(ui('assets/tokens.css')));
  const ikon = png('design/brand/hm-app-ikon.png'), m1 = png('src-tauri/icons/menulinje.png'), i512 = png('src-tauri/icons/icon.png');
  tjek('L app-ikonet: 1024-kilde og ikonsæt fra den', ikon.w === 1024 && ikon.h === 1024 && i512.w === 512 && png('src-tauri/icons/32x32.png').w === 32 && png('src-tauri/icons/128x128@2x.png').w === 256);
  tjek('L menulinjen: RGBA (include_image!), 36 px høj, beskåret tæt', m1.h === 36 && m1.w <= 50 && m1.farve === 6, `${m1.w}×${m1.h}`);
  tjek('L intet nyt i CSP eller værter', /img-src 'self' data:/.test(fs.readFileSync(path.join(repo, 'src-tauri/tauri.conf.json'), 'utf8')));
}

// ── ──
// Startrækkefølgen: defer-scripts kører ved readyState 'interactive'. Rammen kører før visningerne, som kommer med en forsinkelse.
{
  const forsinket = async (hash, svar, vent, ms) => {
    let faerdig = null, tilstand = 'interactive', aaben = false;
    const foer = w => {
      Object.defineProperty(w.document, 'readyState', { configurable: true, get: () => tilstand });
      // jsdom fyrer sit eget DOMContentLoaded straks; i en browser kommer det først efter det sidste defer-script.
      w.document.addEventListener('DOMContentLoaded', e => { if (!aaben) e.stopImmediatePropagation(); }, true);
    };
    const s = side(hash, svar, 'da', 'windows', foer, (w, scripts, koer) => {
      faerdig = (async () => {
        const i = scripts.indexOf('assets/ramme.js');
        scripts.slice(0, i + 1).forEach(koer);
        for (const x of scripts.slice(i + 1)) { await sov(vent()); koer(x); }
        aaben = true;
        w.document.dispatchEvent(new w.Event('DOMContentLoaded'));
        tilstand = 'complete';
      })();
    });
    await faerdig;
    await sov(ms);
    return s;
  };
  let hele = 0, vist = '';
  for (let i = 0; i < 20; i++) {
    const s = await forsinket('#overblik', DYK(), () => Math.random() * 6, 40), n = s.d.querySelectorAll('#overblik-raekker tr').length, gange = s.kald.filter(k => k[0] === 'token_status').length;
    if (n === 3 && synlige(s.d).join() === 'overblik' && gange === 1) hele++; else vist = `${n} rækker, ${synlige(s.d).join()}, token_status ${gange}`;
  }
  tjek('F1 start: visningerne kommer forsinket efter rammen, og Overblik tegnes alligevel (20 af 20, startet præcis én gang)', hele === 20, `${hele} af 20: ${vist}`);
  const a = await forsinket('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: vaernUd(), vaern_oversigt: vaernUd(), koe_prioritet: null }), () => 3, 120);
  tjek('F1 start: også Angreb med domænevælgeren', !a.d.getElementById('band-zone').hidden && a.d.querySelector('#vagt .vg-regel'), a.d.getElementById('vagt').innerHTML.slice(0, 80));
  const k = side('#overblik', DYK(), 'da', 'windows', w => {
    Object.defineProperty(w.document, 'readyState', { configurable: true, get: () => 'complete' });
    w.document.addEventListener('DOMContentLoaded', e => e.stopImmediatePropagation(), true); // det er kommet og gået, før scriptet kører
  });
  await sov(60);
  tjek('F1 start: er siden allerede færdig, starter rammen selv', k.d.querySelectorAll('#overblik-raekker tr').length === 3);
}

// Domænenavne: aldrig brud midt i et ord.
{
  const LANGT = 'et-meget-langt-domaenenavn-eksempel.wi24rd.com', rk = [raekke('z1', 'xray.wi24rd.com', 'roed', { sidst_hentet: NU_ISO, angreb_i_gaar: 3, angreb_fordeling: { blokeret: 3 } }), raekke('z2', LANGT, 'gul', { sidst_hentet: NU_ISO, angreb_i_gaar: 2, angreb_fordeling: { blokeret: 2 } })];
  const svar = x => DYK({ overblik: rk, siden_sidst: { vis: false, nye: [], raekker: rk }, siden_sidst_set: null, zones_cache: { zoner: rk, hentet: NU_ISO }, ...x });
  const css = udenKommentarer(ui('assets/app.css'));
  tjek('F2 CSS: .domaene står på én linje og afskæres med …', /\.domaene \{[^}]*overflow: hidden;[^}]*text-overflow: ellipsis;[^}]*white-space: nowrap/.test(css));
  tjek('F2 CSS: grænsen i Overblik er 24 tegn, og kolonnen har kun plads til navnet', /\.tabel \.domaene \{ max-width: 24ch; \}/.test(css) && /\.tabel tbody th \{ width: 1%;/.test(css));
  tjek('F2 CSS: domænenavne må ikke brydes vilkårligt', !/\.tabel tbody th \{[^}]*overflow-wrap/.test(css) && !/\.hm-siden b \{[^}]*overflow-wrap/.test(css) && /\.dom-navn \{[^}]*white-space: nowrap/.test(css));
  const o = side('#overblik', svar());
  await sov(80);
  const th = [...o.d.querySelectorAll('#overblik-raekker th')], sp = th[1].querySelector('.domaene');
  tjek('F2 Overblik: navnet har fuldt navn i title, og cellen i aria-label', sp.textContent === LANGT && sp.title === LANGT && th[1].getAttribute('aria-label') === LANGT, th[1].outerHTML.slice(0, 200));
  o.emit('hentekoe', { zone: 'z2', navn: LANGT, doegn_faerdige: 1, doegn_i_alt: 9, zoner_faerdige: 0, zoner_i_alt: 2, zoner_fejl: 0, tilstand: 'henter' });
  await sov(2100);
  const th2 = [...o.d.querySelectorAll('#overblik-raekker th')][1];
  tjek('F2 Overblik: "henter …" står stadig ved navnet og i aria-label', th2.querySelector('.raekke-henter') && th2.getAttribute('aria-label') === `${LANGT} henter …`, th2.getAttribute('aria-label'));
  const si = side('#siden-sidst', svar());
  await sov(100);
  const kort = [...si.d.querySelectorAll('.hm-siden-aabn')][1];
  tjek('F2 Siden sidst: navnet er .domaene, knappen har fuldt navn i aria-label og title', kort.querySelector('b.domaene').title === LANGT && kort.getAttribute('aria-label').includes(LANGT) && kort.title.startsWith(LANGT), kort.outerHTML.slice(0, 200));
  const ind = side('#indstillinger', svar({ data_status: { bytes: 1, foerste: '2026-10-04', sidste: '2026-11-03', domaener: 2 } }));
  await sov(100);
  const pd = [...ind.d.querySelectorAll('#pr-domaene-liste [data-gaa]')].find(b => b.textContent === LANGT);
  tjek('F2 Pr. domæne: navnet er .domaene, knappen har fuldt navn i aria-label', pd && pd.querySelector('.domaene').title === LANGT && pd.getAttribute('aria-label').includes(LANGT), pd && pd.outerHTML.slice(0, 200));
  const dy = side('#dyk?' + dykQ({ land: 'PL' }), DYK({ dyk: () => dykSvar({ domaener: [{ zone: 'z2', navn: LANGT, n: 92 }] }) }));
  await sov(100);
  const an = dy.d.querySelector('.dyk-andre .domaene');
  tjek('F2 dyk "andre domæner": navnet er .domaene med fuldt navn', an && an.title === LANGT && an.closest('button').getAttribute('aria-label').includes(LANGT), an && an.outerHTML);
  tjek('F2 dyk: andre lister er ikke domæner', !dy.d.querySelector('.dyk-lister .domaene'));
}

// Stier: brud kun efter "/".
{
  const STI = '/wp-content/plugins/a%2eb/x.php', s = side('#overblik', DYK());
  await sov(60);
  const V = s.w.hmTegn, c = V.sti(STI);
  tjek('F3 V.sti: teksten er uændret, med <wbr> efter hver "/"', c.tagName === 'CODE' && c.textContent === STI && c.querySelectorAll('wbr').length === 4 && [...c.querySelectorAll('.sti-led')].map(n => n.textContent).join('|') === '/|wp-content/|plugins/|a%2eb/|x.php', c.innerHTML);
  tjek('F3 V.sti: sti uden "/" og med slutstreg giver intet <wbr> til sidst', V.sti('[hemmeligheder]').querySelectorAll('wbr').length === 0 && V.sti('/a/').lastChild.tagName !== 'WBR' && V.sti('/').textContent === '/', V.sti('/a/').innerHTML);
  const langt = V.sti('/' + 'a'.repeat(41) + '/b');
  tjek('F3 V.sti: et led over 40 tegn må brydes, de andre står samlet', langt.textContent === '/' + 'a'.repeat(41) + '/b' && langt.querySelectorAll('.sti-led').length === 2 && V.sti('/' + 'a'.repeat(39)).querySelectorAll('.sti-led').length === 2, langt.innerHTML);
  const ond = V.sti('/<b>x</b>/"y"');
  tjek('F3 V.sti: fremmede tegn står som tekst', !ond.querySelector('b') && ond.textContent === '/<b>x</b>/"y"', ond.innerHTML);
  const css = udenKommentarer(ui('assets/vagt.css'));
  tjek('F3 CSS: stier brydes ikke midt i ord', /\.vg code \{[^}]*overflow-wrap: break-word;[^}]*word-break: normal/.test(css) && !/\.vg code \{[^}]*anywhere/.test(css) && /\.sti-led \{ white-space: nowrap; \}/.test(css));
  tjek('F3 CSS: regelkortets stil på overskrifter rammer ikke stiens led', /\.vg-regel-l > span/.test(css));
  const kat = [{ id: 'wordpress', n: 12, stier: [{ sti: STI, n: 9 }] }];
  const data = vaernUd({ h_forsoeg: { i7: 12, kategorier: kat }, h_regler: [kortRegel({ navn: REACT, produkt: 'React', type: 'RCE', cve: null, n7: 1, n30: 2, foerst: '2026-10-05', sidst: '2026-11-02', stier: [{ sti: STI, n: 3 }], lande: [] })], seneste: [{ t: '2026-11-03T05:00:00Z', regel: 'R', cve: null, land: 'US', sti: STI, colo: 'EWR', metode: 'GET' }] });
  const a = side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: data, vaern_oversigt: data, koe_prioritet: null }));
  await sov(120);
  const brud = sel => { const e = a.d.querySelector(sel); return e && e.textContent === STI && e.querySelectorAll('wbr').length === 4; };
  tjek('F3 Angreb: "De seneste" bryder efter "/"', brud('#vg-seneste tbody code'));
  tjek('F3 Angreb: regelkortet bryder efter "/"', brud('.vg-regel-l code'));
  tjek('F3 Angreb: "Hvad de ledte efter" bryder efter "/"', brud('.vg-p-stier code'));
  const dy = side('#dyk?' + dykQ({ land: 'PL' }), DYK({ dyk: () => dykSvar({ top: { ...dykSvar().top, sti: [{ v: STI, n: 4 }] }, enkelte: [{ t: '2026-11-03T21:00:00Z', regel: REACT, cve: null, land: 'PL', sti: STI, colo: 'WAW', metode: 'POST' }] }) }));
  await sov(100);
  const liste = dy.d.querySelector('.dyk-lister code.vg-l-navn'), tabel = dy.d.querySelector('#dyk-rod table code');
  tjek('F3 dyk: listen og "De seneste" bryder efter "/", og filteret er den rå sti', liste.querySelectorAll('wbr').length === 4 && tabel.querySelectorAll('wbr').length === 4 && new URLSearchParams(liste.closest('[data-dyk]').getAttribute('data-dyk')).get('sti') === STI, liste.outerHTML);
}

// ── Robotter og Login ──
{
  const s = side('#robotter?zone=z1&navn=wi24rd.com', DYK(VIS)), en = side('#robotter?zone=z1&navn=wi24rd.com', DYK(VIS), 'en');
  await sov(80);
  const { d } = s, q = x => d.querySelector('#visning-robotter ' + x), qa = x => [...d.querySelectorAll('#visning-robotter ' + x)];
  tjek('V Robotter: visningen står, og kaldet har zonen og 7 dage', synlige(d).join() === 'robotter' && s.kald.some(k => k[0] === 'robotter' && k[1].omfang.zone === 'z1' && k[1].periode === 'd7'));
  const maal = t => { const e = qa('.vg-flise').find(x => x.textContent.includes(t)); return new URLSearchParams((e && e.dataset.dyk) || ''); };
  tjek('V Robotter: fliserne åbner dyk med samme periode', maal('Besøg fra kendte').get('robot') === 'alle' && maal('Besøg fra kendte').get('periode') === '7' && maal('Udklædte').get('robot') === 'udklaedt' && maal('Stoppet ved Edge').get('robot') === 'stoppet' && maal('Lokket ind').get('robot') === 'labyrint');
  const andel = qa('.vg-flise').find(x => x.textContent.includes('Andel af al trafik'));
  tjek('V Robotter: andelen er ikke klikbar (ingen rækker bag)', andel.tagName === 'DIV' && !andel.dataset.dyk && andel.querySelector('b').textContent === '7,4 %', andel.outerHTML);
  const kort = qa('.gruppe-kort');
  tjek('V Robotter: syv grupper med note, og hver åbner sin robotgruppe', kort.length === 7 && kort.every(k => k.querySelector('.konsekvens').textContent.length > 10) && new URLSearchParams(kort[0].dataset.dyk).get('robotgruppe') === 'ai-traening', String(kort.length));
  const noter = Object.values(s.w.hmSprog.T.robotGrupper).slice(0, 6).map(g => g[2]);
  tjek('V J1 konsekvensnoterne siger "kan" på dansk og "may" på engelsk', noter.every(t => /\bkan\b/.test(t)) && Object.values(en.w.hmSprog.T.robotGrupper).slice(0, 6).every(g => /\bmay\b/.test(g[2])), noter.join(' | '));
  const lister = qa('.vg-tre .dyk-l').map(b => new URLSearchParams(b.dataset.dyk));
  tjek('V Robotter: lister åbner robot, side og slags', lister.some(p => p.get('robot') === 'Googlebot') && lister.some(p => p.get('robot') === 'alle' && p.get('sti') === '/priser/') && lister.some(p => p.get('robot') === 'stoppet:GPTBot') && lister.some(p => p.get('robot') === 'labyrint:ClaudeBot') && lister.some(p => p.get('robot') === 'udklaedt:Googlebot') && lister.some(p => p.get('robot') === 'uverificeret:FooBot'));
  const tekst = q('.vg-tre').textContent;
  tjek('V Robotter: kategorinavne står i klarsprog', tekst.includes('Verificeret (AI Crawler)') && tekst.includes('Ukendt robot eller browser') && en.d.querySelector('#visning-robotter .vg-tre').textContent.includes('AI assistant in a browser'), tekst.slice(0, 200));
  const T = s.w.hmSprog.T, E = en.w.hmSprog.T, B = id => [s.w.hmSprog.bog(id), en.w.hmSprog.bog(id)];
  tjek('V labyrinten kaldes aldrig blokeret', ![T.robotSlags.labyrint, T.robotLabyrintSub, T.robotLabyrintNote, E.robotSlags.labyrint, E.robotLabyrintSub, E.robotLabyrintNote, ...B('labyrint').flatMap(b => [b.ord, b.alle, ...b.ja])].some(x => /blok|block/i.test(x)));
  tjek('V "blokeret" står heller ikke om det, der blev lokket ind', !/blokeret/i.test(q('.vg-tre').textContent));
  tjek('V Robotter: to perioder ved siden af datovælgeren, 7 dage er valgt', d.querySelectorAll('#robotter-periode input').length === 2 && d.querySelectorAll('#robotter-periode input')[0].checked);
  d.querySelectorAll('#robotter-periode input')[1].click();
  await sov(40);
  tjek('V Robotter: periodevalget er ét trin, og dyk følger med (30 = ingen periode)', s.w.location.hash.includes('periode=30') && s.kald.filter(k => k[0] === 'robotter').pop()[1].periode === 'd30' && !new URLSearchParams(q('.gruppe-kort').dataset.dyk).has('periode'), s.w.location.hash);
  const ingen = side('#robotter?zone=z1&navn=a.wi24rd.com', DYK({ ...VIS, robotter: { ...ROBOT_UD, grupper: [], sider: [], crawlere: { ...ROBOT_UD.crawlere, iAlt: { bots7: 0, bots30: 0, alle7: 0, botNavne: 0, falske7: 0 }, bots: [], falske: [], uverificerede: [] }, stoppet: { ...ROBOT_UD.stoppet, sum: 0, labyrint: 0, robotter: [], labyrint_robotter: [], ikke_tilgaengelig: ['a.wi24rd.com'] } } }));
  await sov(60);
  tjek('V Robotter: 0 er ikke klikbart, og "ikke tilgængelig" står for domænet', !ingen.d.querySelector('#robotter-rod [data-dyk]') && ingen.d.querySelector('#robotter-rod .vg-advarsel:not(.dato-fejl)').textContent.includes('ikke tilgængelig for: a.wi24rd.com') && ingen.d.querySelector('#robotter-rod .dyk-tom').textContent === 'Ingen robotter i perioden.', ingen.d.getElementById('robotter-rod').innerHTML.slice(0, 200));
  const samlet = side('#robotter', DYK(VIS));
  await sov(60);
  tjek('V Robotter uden parametre: de valgte samlet, med domænevælgeren', samlet.kald.some(k => k[0] === 'robotter' && k[1].omfang.zoner) && !samlet.d.getElementById('band-zone').hidden);
  samlet.d.getElementById('vg-zone').value = 'z1';
  samlet.d.getElementById('vg-zone').dispatchEvent(new samlet.w.Event('change', { bubbles: true }));
  await sov(40);
  tjek('V Robotter: domænevælgeren skifter domæne', samlet.w.location.hash.startsWith('#robotter?zone=z1'), samlet.w.location.hash);
}
{
  const s = side('#login?zone=z1&navn=wi24rd.com', DYK(VIS)), { d, w } = s;
  await sov(80);
  const q = x => d.querySelector('#visning-login ' + x), qa = x => [...d.querySelectorAll('#visning-login ' + x)];
  tjek('V Login: visningen står, og kaldet har zonen og 30 dage', synlige(d).join() === 'login' && s.kald.some(k => k[0] === 'login' && k[1].omfang.zone === 'z1' && k[1].periode === 'd30'));
  const fliser = qa('.vg-flise');
  tjek('V Login: de tre tal står hver for sig, og ingen flise lægger dem sammen', ['161', '18', '51'].every(n => fliser.some(f => f.querySelector('b').textContent === n)) && !fliser.some(f => f.querySelector('b').textContent === '230') && !q('.vg-fliser').textContent.includes('Forsøg'), q('.vg-fliser').textContent);
  tjek('V Login: 200/302 i klarsprog', fliser.some(f => f.textContent.includes('Svar 200 eller 302') && f.textContent.includes('kan være lykket, kan være en fejlside')));
  const strenge = o => (typeof o === 'string' ? [o] : o && typeof o === 'object' ? Object.values(o).flatMap(strenge) : []);
  const alle = [...strenge(w.hmSprog.T), ...['loginsti', 'naaede'].flatMap(id => strenge(w.hmSprog.bog(id))), ui('assets/visning-login.js')];
  tjek('V ordet "lykkedes" findes aldrig', !alle.some(t => /lykkedes|succeeded|successful/i.test(t)), alle.filter(t => /lykkedes|succeeded|successful/i.test(t)).join(' | ').slice(0, 200));
  tjek('V Login: note om stikprøver og ingen IP-adresser', [...d.querySelectorAll('#login-rod .vg-note')].some(n => n.textContent.includes('IP-adresser') && n.textContent.includes('stikprøver')));
  const serv = fliser.find(f => f.textContent.includes('Nåede din server'));
  tjek('V Login: fliserne bliver i Login med et filter', new URLSearchParams(serv.dataset.gaa.split('?')[1]).get('udfald') === 'server' && serv.dataset.gaa.startsWith('login?zone=z1'), serv.dataset.gaa);
  const svg = q('svg.dyk-soejler'), dage = [...svg.querySelectorAll('[data-gaa]')];
  tjek('V Login: søjlerne er ét Tab-stop', dage.length > 0 && dage.filter(e => e.getAttribute('tabindex') === '0').length === 1 && dage.filter(e => e.getAttribute('tabindex') === '-1').length === dage.length - 1);
  const sidst = dage[dage.length - 1], rect = [...sidst.querySelectorAll('rect:not(.dyk-flade)')];
  tjek('V Login: søjlerne står side om side, aldrig oven på hinanden', rect.length >= 2 && new Set(rect.map(r => r.getAttribute('x'))).size === rect.length, rect.map(r => r.getAttribute('x')).join());
  tjek('V Login: mærket nævner alle tre tal hver for sig', sidst.getAttribute('aria-label').includes('stoppet ved Edge') && sidst.getAttribute('aria-label').includes('fik 404') && sidst.getAttribute('aria-label').includes('nåede din server'), sidst.getAttribute('aria-label'));
  sidst.focus();
  sidst.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true, cancelable: true }));
  tjek('V Login: piletaster flytter mellem dagene', d.activeElement === dage[dage.length - 2] && dage[dage.length - 2].getAttribute('tabindex') === '0');
  tjek('V Login: en dag åbner #login med dagen', new URLSearchParams(dage[0].dataset.gaa.split('?')[1]).has('dag'), dage[0].dataset.gaa);
  const r = qa('.vg-tabel tbody tr');
  tjek('V Login: en sti med Edge-tal åbner dyk, en uden bliver i Login', new URLSearchParams(r[0].querySelector('.sti-knap').dataset.dyk).get('sti') === '/wp-login.php' && new URLSearchParams(r[0].querySelector('.sti-knap').dataset.dyk).get('metode') === 'POST' && r[1].querySelector('.sti-knap').dataset.gaa.startsWith('login?'), r[1].querySelector('.sti-knap').outerHTML);
  tjek('V Login: et tal på 0 er ikke klikbart', !r[1].children[2].querySelector('button') && r[1].children[2].textContent === '0', r[1].children[2].outerHTML);
  tjek('V Login: tal i tabellen filtrerer på sti og udfald', (p => p.get('sti') === '/wp-login.php' && p.get('udfald') === 'svar')(new URLSearchParams(r[0].children[6].querySelector('button').dataset.gaa.split('?')[1])));
  tjek('V Login: forklaringen lover ikke et login', r[1].querySelector('.forkl').textContent.includes('siger ikke, om nogen kom ind'), r[1].querySelector('.forkl').textContent);
  const lande = qa('.vg-tre .dyk-l').map(b => b.dataset.gaa);
  tjek('V Login: lande og netværk filtrerer i Login, så tallet er det samme', lande.some(x => x.includes('land=PL')) && lande.some(x => x.includes('netvaerk=AS64500')) && lande.some(x => x.includes('metode=POST')), lande.join(' '));
  tjek('V Login: netværk uden navn står som nummer, hver kilde for sig', q('.vg-tre').textContent.includes('Netværk, nåede din server') && q('.vg-tre').textContent.includes('AS64500'));
  const f = side('#login?zone=z1&navn=a.wi24rd.com&udfald=svar&land=PL&dag=2026-11-02', DYK(VIS));
  await sov(70);
  const k = f.kald.filter(x => x[0] === 'login').pop()[1];
  tjek('V Login: filtre og dag sendes til Rust', k.filter.udfald === 'svar' && k.filter.land === 'PL' && k.periode.dag === '2026-11-02', JSON.stringify(k));
  tjek('V Login: 404 står ikke med land, og valgt flise er markeret', ![...f.d.querySelectorAll('#login-rod .vg-flise')].some(x => x.textContent.includes('Fik 404')) && f.d.querySelector('#login-rod .vg-flise[aria-current]').textContent.includes('Svar 200'));
  tjek('V Login: et filter kan fjernes, og uden søjler for én dag', [...f.d.querySelectorAll('#login-rod .sti-chip')].some(b => b.textContent.endsWith('×')) && !f.d.querySelector('#login-rod svg.dyk-soejler'));
  const ingen = side('#login?zone=z1&navn=a.wi24rd.com', DYK({ ...VIS, login: { ...LOGIN_UD, stier: [], ikke_tilgaengelig: ['a.wi24rd.com'] } }));
  await sov(70);
  const ret = ingen.d.querySelector('#login-rod .login-ret');
  tjek('V Login uden stier: listen står åben med startlisten, og ingen tal står', ret.open && ret.querySelectorAll('.login-valg input').length === 3 && !ingen.d.querySelector('#login-rod .vg-fliser'), ingen.d.getElementById('login-rod').innerHTML.slice(0, 300));
  tjek('V Login: "ikke tilgængelig" står for domænet', ingen.d.querySelector('#login-rod .vg-advarsel:not(.dato-fejl)').textContent.includes('ikke tilgængelig for: a.wi24rd.com'));
  const boks = ret.querySelectorAll('.login-valg input')[1];
  boks.checked = true;
  boks.dispatchEvent(new ingen.w.Event('change', { bubbles: true }));
  await sov(30);
  tjek('V Login: en sti slås til fra visningen', ingen.kald.some(x => x[0] === 'login_sti_saet' && x[1].sti === '/admin/' && x[1].til === true && x[1].zone === 'z1'));
  ret.querySelector('.sti-chip.forslag').click();
  await sov(30);
  tjek('V Login: et forslag tilføjes med ét klik', ingen.kald.some(x => x[0] === 'login_sti_saet' && x[1].sti === '/api/auth/signin' && x[1].til === true));
  const samlet = side('#login?samlet=1', DYK(VIS));
  await sov(70);
  tjek('V Login samlet: stierne rettes pr. domæne', !samlet.d.querySelector('#login-rod .login-ret') && samlet.d.getElementById('login-rod').textContent.includes('vælg et domæne'));
}
{
  const s = side('#dyk?' + dykQ({ robot: 'alle' }), DYK({ dyk: () => ROBOT_DYK })), { d } = s;
  await sov(80);
  const a = sidsteDyk(s), rod = d.getElementById('dyk-rod');
  tjek('V dyk: robot sendes som filter', a.filtre.length === 1 && a.filtre[0].dim === 'robot' && a.filtre[0].vaerdi === 'alle', JSON.stringify(a));
  tjek('V dyk: overskriften og sidebjælken er Robotter', d.getElementById('dyk-h').textContent === 'Kendte robotter på wi24rd.com' && d.querySelector('#side a[aria-current]').getAttribute('data-visning') === 'robotter' && d.querySelector('#dyk-tilbage button').textContent === '← Hele oversigten over robotter på wi24rd.com', d.getElementById('dyk-h').textContent);
  tjek('V dyk: ingen vælger for slags, ingen fordeling og ordet er besøg', !rod.querySelector('.dyk-slags') && !rod.querySelector('.vg-fordeling') && rod.querySelector('.vg-fliser').textContent.includes('Besøg'), rod.querySelector('.vg-fliser').textContent.slice(0, 120));
  const robotter = [...rod.querySelectorAll('.dyk-lister .dyk-l')].map(b => new URLSearchParams(b.dataset.dyk));
  tjek('V dyk: listen over robotter erstatter "alle"', robotter.some(p => p.get('robot') === 'GPTBot') && robotter.some(p => p.get('robot') === 'alle' && p.get('sti') === '/'));
  const status = [...rod.querySelectorAll('.dyk-lister h3')].find(h => h.textContent === 'Svar').parentElement;
  tjek('V dyk: svarkoder har intet filter og er ikke klikbare', !status.querySelector('[data-dyk]') && status.textContent.includes('200'));
  const u = side('#dyk?' + dykQ({ robot: 'udklaedt' }), DYK({ dyk: () => ROBOT_DYK }));
  await sov(70);
  tjek('V dyk: blandt de udklædte peger en robot på sin slags', [...u.d.querySelectorAll('.dyk-lister .dyk-l')].some(b => new URLSearchParams(b.dataset.dyk).get('robot') === 'udklaedt:GPTBot'));
  const g = side('#dyk?' + dykQ({ robotgruppe: 'seo' }), DYK({ dyk: () => ROBOT_DYK }));
  await sov(70);
  tjek('V dyk: gruppens navn og note', g.d.getElementById('dyk-h').textContent === 'SEO-værktøjer på wi24rd.com' && g.d.getElementById('dyk-rod').textContent.includes('kan de værktøjer miste data'));
  const l = side('#dyk?' + dykQ({ robot: 'labyrint' }), DYK({ dyk: () => ROBOT_DYK }));
  await sov(70);
  tjek('V dyk: labyrinten har sin note og tæller hændelser, ikke besøg', l.d.getElementById('dyk-rod').textContent.includes('Labyrinten stopper ikke robotten') && l.d.querySelector('#dyk-rod .vg-fliser').textContent.includes('Hændelser'));
}
{
  const nye = [{ zone: 'z1', domaene: 'wi24rd.com', robot: 'NyBot', foerst: '2026-11-02' }];
  const s = side('#siden-sidst', DYK({ siden_sidst: { vis: false, nye: [], raekker: OVERBLIK, nye_robotter: nye }, siden_sidst_set: null }));
  await sov(70);
  const li = s.d.querySelector('.hm-siden-robot button');
  tjek('V Siden sidst: en ny AI-robot har sin egen linje, som åbner robotten', li.textContent === 'Ny AI-robot på wi24rd.com: NyBot' && new URLSearchParams(li.dataset.dyk).get('robot') === 'NyBot' && new URLSearchParams(li.dataset.dyk).get('zone') === 'z1', li.outerHTML);
  const ingen = side('#siden-sidst', DYK({ siden_sidst: { vis: false, nye: [], raekker: OVERBLIK }, siden_sidst_set: null }));
  await sov(70);
  tjek('V Siden sidst: ingen ny robot, ingen linje', !ingen.d.querySelector('.hm-siden-robot'));
}
{
  const z = { zones_cache: { zoner: OVERBLIK, hentet: NU_ISO }, data_status: { bytes: 8e6, foerste: '2026-10-04', sidste: '2026-11-03', domaener: 3 } };
  const s = side('#indstillinger?afsnit=login&zone=z2', DYK({ ...VIS, ...z })), { d } = s;
  await sov(90);
  tjek('V Indstillinger: Pr. domæne har login-stier med domænet fra adressen', d.getElementById('login-domaene').value === 'z2' && s.kald.some(k => k[0] === 'login_stier' && k[1].zone === 'z2') && d.querySelectorAll('#login-panel .login-valg input').length === 3, d.getElementById('login-domaene').value);
  d.getElementById('login-domaene').value = 'z1';
  d.getElementById('login-domaene').dispatchEvent(new s.w.Event('change', { bubbles: true }));
  await sov(40);
  tjek('V Indstillinger: et andet domæne henter sine stier', s.kald.some(k => k[0] === 'login_stier' && k[1].zone === 'z1'));
  const boks = d.querySelector('#login-panel .login-valg input');
  boks.checked = false;
  boks.dispatchEvent(new s.w.Event('change', { bubbles: true }));
  await sov(30);
  tjek('V Indstillinger: en sti slås fra', s.kald.some(k => k[0] === 'login_sti_saet' && k[1].zone === 'z1' && k[1].sti === '/login' && k[1].til === false));
}
{
  const s = side('#overblik', DYK(VIS)), { d, w } = s;
  await sov(70);
  const genvej = async n => { s.tast(n, { ctrlKey: true }); await sov(50); return w.location.hash; };
  tjek('V genveje: Ctrl+4 og Ctrl+5 på Windows', (await genvej('4')).startsWith('#robotter') && (await genvej('5')).startsWith('#login'), w.location.hash);
  tjek('V sidebjælken: Robotter og Login står med genvej', ['robotter', 'login'].every(v => d.querySelector(`#side a[data-visning="${v}"] kbd`).textContent.startsWith('Ctrl+')));
  s.emit('visning', 'robotter');
  await sov(50);
  tjek('V programmenuen: hændelsen visning åbner Robotter', w.location.hash.startsWith('#robotter'), w.location.hash);
  const titel = side('#login?zone=z1&navn=wi24rd.com', DYK(VIS));
  await sov(60);
  tjek('V titel og hjælp følger visningen', titel.d.title.startsWith('Login · wi24rd.com') && !titel.d.querySelector('.hjaelp-del[data-hjaelp=login]').hidden && titel.d.querySelector('#ord-login dt').textContent === 'Login-sti', titel.d.title);
  const k = side('#robotter?zone=z1&navn=a.wi24rd.com', DYK(VIS));
  await sov(60);
  const foer = k.kald.filter(x => x[0] === 'robotter').length;
  k.emit('ekstra', 'z1');
  await sov(3200);
  tjek('V nye ekstra tal tegner visningen igen', k.kald.filter(x => x[0] === 'robotter').length === foer + 1, String(k.kald.filter(x => x[0] === 'robotter').length - foer));
  const bog = ['robot', 'verificeret', 'udklaedt', 'linkpreview', 'labyrint', 'loginsti', 'naaede'];
  tjek('V BOG: de nye ord findes på begge sprog', bog.every(id => w.hmSprog.bogIds.includes(id) && w.hmSprog.bog(id).ord && w.hmSprog.bog(id).alle));
  for (const f of ['visning-robotter.js', 'visning-login.js']) {
    const kode = udenKommentarer(ui('assets/' + f));
    tjek(`V ${f}: ingen innerHTML og ingen style=`, !/innerHTML|\.style\b|style=/.test(kode));
  }
  const bygge = fs.readFileSync(path.join(repo, 'src-tauri', 'build.rs'), 'utf8');
  tjek('V sikkerhedsporten: de nye kommandoer er tilladt og står i build.rs', ['robotter', 'login', 'login_stier', 'login_sti_saet'].every(c => cap.permissions.includes('allow-' + c.replace(/_/g, '-')) && bygge.includes(`"${c}",`)));
  const da = fs.readFileSync(path.join(repo, 'README.da.md'), 'utf8'), enR = fs.readFileSync(path.join(repo, 'README.md'), 'utf8');
  tjek('V begge README: Robotter, Login og Apache-2.0', da.includes('## Robotter og Login') && enR.includes('## Robots and Login') && da.includes('Apache-2.0') && enR.includes('Apache-2.0') && !/^MIT —/m.test(da + enR));
  const lic = fs.readFileSync(path.join(repo, 'LICENSE'), 'utf8'), not = fs.readFileSync(path.join(repo, 'NOTICE'), 'utf8');
  /* fast år */ tjek('V licens: Apache-2.0 med copyright-linjen, NOTICE og varemærket', lic.includes('Apache License') && lic.includes('Version 2.0, January 2004') && lic.includes('Copyright [yyyy] [name of copyright owner]') && !lic.includes('Jan Tingsvad') && not.includes('Copyright (c) 2026 Jan Tingsvad') && !lic.includes('MIT License') && not.includes('Cloudflare is a registered trademark of Cloudflare, Inc.'));
  tjek('V licens: package.json og Cargo.toml', JSON.parse(fs.readFileSync(path.join(repo, 'package.json'), 'utf8')).license === 'Apache-2.0' && /^license = "Apache-2\.0"$/m.test(fs.readFileSync(path.join(repo, 'src-tauri', 'Cargo.toml'), 'utf8')));
}

// ── <Land> lige nu og Verden ──
{
  const s = side('#land?kode=DK', LANDV());
  await sov(60);
  const { d, w } = s, rod = d.getElementById('land-rod');
  tjek('L visningen står med landets navn, og kaldene har landet', synlige(d).join() === 'land' && d.getElementById('land-h').textContent === 'Danmark lige nu' && s.kald.some(k => k[0] === 'land_visning' && k[1].kode === 'DK' && k[1].waf === false), d.getElementById('land-h').textContent);
  tjek('L navnet følger landet i titlen og sidebjælken', d.title.startsWith('Danmark lige nu · ') && d.getElementById('nav-land').textContent === 'Danmark lige nu', d.title);
  tjek('L visningen beder om nye tal, når den åbnes', s.kald.some(k => k[0] === 'radar_opdater' && k[1].kode === 'DK'));
  tjek('L landevælgeren står i båndet, og domænevælgeren gør ikke', !d.getElementById('band-land').hidden && d.getElementById('band-zone').hidden && d.getElementById('vg-land').value === 'DK');
  tjek('L to faner, og sidebjælken har ⌘6', [...d.querySelectorAll('#land-faner input')].length === 2 && d.querySelector('#land-faner').textContent === 'DanmarkVerden' && d.querySelector('#side a[data-visning="land"] kbd').textContent === 'Ctrl+6');
  const kurver = [...rod.querySelectorAll('polyline')].map(p => p.getAttribute('class'));
  tjek('L to kurver på én akse, mine og verden', new Set(kurver).size === 2 && kurver.every(k => /vg-k-(mit|verden)/.test(k)) && !rod.textContent.includes('beregnet'), kurver.join());
  tjek('L et hul bryder min kurve', kurver.filter(k => k.includes('vg-k-mit')).length === 2, String(kurver.length));
  const dage = [...rod.querySelectorAll('svg [data-dyk]')];
  tjek('L kurven er ét Tab-stop', dage.length === 23 && dage.filter(e => e.getAttribute('tabindex') === '0').length === 1 && dage.filter(e => e.getAttribute('tabindex') === '-1').length === 22, String(dage.length));
  const q = new URLSearchParams(dage[0].dataset.dyk);
  tjek('L min kurve åbner dagen i dyk for de valgte domæner', q.get('samlet') === '1' && q.get('dag') === landDage[1].dag && q.get('slags') === 'angreb', dage[0].dataset.dyk);
  tjek('L en dag uden angreb og et hul kan ikke åbnes', ![...rod.querySelectorAll('svg g')].filter(g => g.querySelector('title').textContent.includes('mine domæner 0') || g.querySelector('title').textContent.includes('mine domæner –')).some(g => g.hasAttribute('data-dyk')));
  dage[dage.length - 1].focus();
  dage[dage.length - 1].dispatchEvent(new w.KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true, cancelable: true }));
  tjek('L piletaster flytter mellem dagene', d.activeElement === dage[dage.length - 2] && dage[dage.length - 2].getAttribute('tabindex') === '0');
  tjek('L andelen som tal, med kilde og mål hver for sig', rod.textContent.includes('Danmark var mål for 0,4 % af angrebene i går.') && rod.textContent.includes('Danmark som mål'), rod.textContent.slice(0, 300));
  tjek('L dag for dag: uden for top 100 står som ord, ikke som 0', [...rod.querySelectorAll('.land-dage li')].map(l => l.textContent).join('|') === '2. november0,4 %|1. novemberikke i top 100', [...rod.querySelectorAll('.land-dage li')].map(l => l.textContent).join('|'));
  tjek('L dag for dag er ikke klikbar', !rod.querySelector('.land-dage button, .land-dage [data-gaa]'));
  const mod = d.getElementById('land-kilde'), kol = [...mod.querySelectorAll('.vg-tre > div')];
  tjek('L sammenligningen: to kolonner side om side, fra målsiden', kol.map(k => k.querySelector('h3').textContent).join('|') === 'De største kilder til angreb mod Danmark|Hvor angreb mod dine domæner kom fra', mod.textContent.slice(0, 200));
  tjek('L sammenligningen: 4 uger er valgt, og landets kolonne har Radars procenter', mod.querySelector('input:checked').parentElement.textContent === '4 uger' && kol[0].textContent.includes('Tyskland') && kol[0].textContent.includes('30 %'), kol[0].textContent);
  tjek('L sammenligningen: landets kolonne kan ikke klikkes', !kol[0].querySelector('button, [data-dyk], [data-gaa]'));
  const minKnap = kol[1].querySelector('button'), mq = new URLSearchParams(minKnap.dataset.dyk);
  tjek('L sammenligningen: dit land åbner dyk med land og samme periode', mq.get('samlet') === '1' && mq.get('land') === 'US' && mq.get('slags') === 'angreb' && !mq.has('periode'), minKnap.dataset.dyk);
  tjek('L "Særligt hos dig" står kun ved det, Rust har mærket, og i knappens navn', kol[1].querySelectorAll('.vg-saerligt').length === 1 && minKnap.querySelector('.vg-saerligt').textContent === 'Særligt hos dig' && minKnap.getAttribute('aria-label').includes('Særligt hos dig'), kol[1].innerHTML.slice(0, 200));
  tjek('L note om kilde-lande med landets navn', mod.textContent.includes('Angriberens land bestemmes af IP-adressen og kan skyldes VPN eller lejede servere. Det gælder både for Danmark og for dine domæner, så sammenligningen er fair, men landet siger ikke, hvem angriberen er.'));
  tjek('L "Angreb fra <land>" og top-parrene er væk', !rod.textContent.includes('Angreb fra') && !rod.textContent.includes('→') && !('landFraH' in w.hmSprog.T), rod.textContent.slice(0, 200));
  const andelKurve = d.getElementById('land-maal').querySelector('svg');
  tjek('L andelen er en lille kurve, målt andel og aldrig antal', andelKurve === null, 'to punkter, hvoraf ét mangler, giver ingen kurve');
  const uf = [...rod.querySelectorAll('.land-udfald li')];
  tjek('L forstyrrelser: dato, årsag, varighed og netværk i klarsprog', uf.length === 2 && uf[0].textContent === '30. oktober Et kabel blev kappet · 3 timer · ét netværkBerørte netværk: Fjord Net, Bakke Net', uf[0] && uf[0].textContent);
  tjek('L forstyrrelser: ukendt årsag og tom slutdato er ærlige, og intet kan klikkes', uf[1].textContent === '20. oktober Ukendt årsag · ingen slutdato oplyst' && !rod.querySelector('.land-udfald button'), uf[1] && uf[1].textContent);
  tjek('L intet står i ord, når Rust ikke sender ord (små tal)', !rod.querySelector('.dom'));
  const kilde = rod.querySelector('.radar-kilde button');
  tjek('L kilde og licens under tallene', kilde.textContent === 'Kilde: Cloudflare Radar, CC BY-NC 4.0' && kilde.getAttribute('aria-label').includes('browser'));
  kilde.click(); await sov(10);
  tjek('L kilden åbner Radar gennem Rust', s.kald.some(k => k[0] === 'aabn_radar'));
  const foer = s.kald.filter(k => k[0] === 'land_visning').length;
  s.emit('radar');
  await sov(30);
  tjek('L hændelsen radar tegner igen', s.kald.filter(k => k[0] === 'land_visning').length === foer + 1);
  const v = d.getElementById('vg-land');
  v.value = 'SE';
  v.dispatchEvent(new w.Event('change', { bubbles: true }));
  await sov(60);
  tjek('L båndets vælger gemmer landet, og siden står på det uden kode i adressen', s.kald.some(k => k[0] === 'land_saet' && k[1].kode === 'SE') && !w.location.hash.includes('kode=') && d.getElementById('land-h').textContent === 'Sverige lige nu' && d.getElementById('nav-land').textContent === 'Sverige lige nu' && v.value === 'SE', w.location.hash + ' ' + d.getElementById('land-h').textContent);

  const vd = side('#land?kode=DK&fane=verden', LANDV());
  await sov(60);
  const vr = vd.d.getElementById('land-rod'), lister = [...vr.querySelectorAll('.vg-tre > div')];
  tjek('L Verden: mest ramte, flest fra, produkter, robotter og formål', ['Mest ramte lande', 'Flest angreb fra', 'Hvad der stoppede dem'].every((h, i) => lister[i].querySelector('h3').textContent === h) && vr.textContent.includes('Robottrafik') && vr.textContent.includes('38,5 %') && vr.textContent.includes('AI-robotter efter formål') && vr.textContent.includes('Træning'), vr.textContent.slice(0, 200));
  tjek('L Verden: ingen kurve og ingen andel', !vr.querySelector('polyline') && !vd.d.getElementById('land-maal'));
  const b = i => [...lister[i].querySelectorAll('li')];
  tjek('L Verden: et land går til fanen med landet', b(0)[0].querySelector('button').dataset.gaa === 'land?kode=US&afsnit=maal' && b(1)[0].querySelector('button').dataset.gaa === 'land?kode=CN&afsnit=kilde');
  tjek('L Verden: Tor har ingen fane', !b(1)[1].querySelector('button'));
  tjek('L Verden: kun WAF har data bag, de andre produkter er tekst', b(2)[1].querySelector('button').dataset.gaa === 'land?produkt=WAF' && !b(2)[0].querySelector('button'));
  tjek('L Verden: Radars rene procenter er ikke klikbare', [...vr.querySelectorAll('.vg-flise')].every(f => f.tagName === 'DIV' && !f.querySelector('button')) && ![...vr.querySelectorAll('.vg-blok')][1].querySelector('button[data-gaa], button[data-dyk]'));
  tjek('L Verden: kilde og licens', vr.querySelector('.radar-kilde'));
  tjek('L Verden: ukendte formål står, som de er', vr.textContent.includes('Noget nyt'));

  const wf = side('#land?produkt=WAF', LANDV());
  await sov(60);
  tjek('L WAF: verden er WAF, og filteret kan fjernes', wf.kald.some(k => k[0] === 'land_visning' && k[1].waf === true) && wf.d.getElementById('land-rod').textContent.includes('Verden, WAF') && wf.d.querySelector('#land-rod [data-gaa="land"]'));

  const mn = side('#land?kode=DK', LANDV({ land_visning: { ...LAND_UD, ord: { dag: '2026-11-02', angreb: 40, snit: 10, tendens: 'saerligt_dig' } } }));
  await sov(60);
  tjek('L gul-reglen opfyldt: sætningen står', mn.d.querySelector('#land-rod .dom').textContent.includes('Stigningen er din egen') && mn.d.querySelector('#land-rod .dom').textContent.includes('40 angreb den 2. november (snit 10)'));
  const fg = side('#land?kode=DK', LANDV({ land_visning: { ...LAND_UD, ord: { dag: '2026-11-02', angreb: 40, snit: 10, tendens: 'foelger' } } }));
  await sov(60);
  tjek('L følger: siger kun noget om WAF-angreb, aldrig om en regel', fg.d.querySelector('#land-rod .dom').textContent.includes('følger den globale tendens') && fg.d.querySelector('#land-rod .dom').textContent.includes('WAF-angreb') && !/react|cve|wordpress/i.test(fg.d.querySelector('#land-rod .dom').textContent));
  const ud = side('#land?kode=DK', LANDV({ land_visning: { ...LAND_UD, andel: [{ dag: '2026-11-02', vaerdi: null }] } }));
  await sov(60);
  tjek('L uden for top 100', ud.d.getElementById('land-rod').textContent.includes('Danmark var ikke blandt de 100 mest ramte lande i går.'));
  const ld = side('#land?kode=DK', LANDV({ land_visning: { ...LAND_UD, andel: [{ dag: '2026-10-30', vaerdi: 1.2 }] } }));
  await sov(60);
  tjek('L et ældre døgn står med dato', ld.d.getElementById('land-rod').textContent.includes('Danmark var mål for 1,2 % af angrebene den 30. oktober.'));
  const ft = side('#land?kode=DK', LANDV({ land_visning: { ...LAND_UD, fejl: { kode: 'radar', status: 403 } } }));
  await sov(60);
  const adv = ft.d.querySelector('#land-rod .vg-advarsel');
  tjek('L en Radar-fejl står kun i visningen, og tallene står stadig', adv.textContent.includes('Cloudflare Radar svarer ikke') && adv.textContent.includes('Dine egne tal virker stadig') && ft.d.querySelector('#land-rod polyline') && ft.d.getElementById('koe').hidden, adv.textContent);
  tjek('L ingen "ekstra rettighed": hjælpen siger stadig præcis to rettigheder', !ui('assets/sprog.js').includes('ekstra rettighed') && ft.w.hmSprog.T.tokenAabnNote.includes('de to rettigheder'));
  const tom = side('#land?kode=DK', LANDV({ land_visning: { ...LAND_UD, hentet: null, verden_dag: null, andel: [], par: [], maal: [], kilde: [] } }));
  await sov(60);
  tjek('L første gang: tallene hentes', tom.d.getElementById('land-rod').textContent.includes('hentes') && tom.d.getElementById('land-rod').textContent.includes('Ingen tal for Danmark endnu.'));

  /* Landet første gang */
  const reg = sprog => w2 => Object.defineProperty(w2.navigator, 'language', { value: sprog, configurable: true });
  const f1 = side('#land', LANDV({ land_hent: { land: null, forslag: null } }), 'da', 'windows', reg('da-DK'));
  await sov(70);
  tjek('L første gang: landet kommer fra systemets region og gemmes', f1.kald.some(k => k[0] === 'land_saet' && k[1].kode === 'DK') && f1.kald.some(k => k[0] === 'land_visning' && k[1].kode === 'DK'), JSON.stringify(f1.kald.filter(k => k[0].startsWith('land'))));
  const f2 = side('#land', LANDV({ land_hent: { land: null, forslag: 'SE' } }), 'da', 'windows', reg('da'));
  await sov(70);
  const valgB = f2.d.querySelector('#land-rod .land-valg button');
  tjek('L uden region: brugeren vælger selv, med "Skift til"', !f2.kald.some(k => k[0] === 'land_saet') && !f2.kald.some(k => k[0] === 'land_visning') && valgB && valgB.textContent === 'Skift til Sverige' && f2.d.getElementById('land-rod').textContent.includes('Vælg det land'), f2.d.getElementById('land-rod').textContent);
  valgB.click(); await sov(70);
  tjek('L forslaget gemmer landet og åbner det', f2.kald.some(k => k[0] === 'land_saet' && k[1].kode === 'SE') && f2.d.getElementById('land-h').textContent === 'Sverige lige nu', f2.w.location.hash);
  const f3 = side('#land', LANDV({ land_hent: { land: null, forslag: 'ZZ' } }), 'da', 'windows', reg('es-419'));
  await sov(70);
  tjek('L region som tal og ukendt forslag: vælg selv, aldrig en fejl', !f3.d.querySelector('#land-rod .vg-roed') && !f3.d.querySelector('#land-rod .land-valg') && f3.d.getElementById('land-rod').textContent.includes('Vælg det land'), f3.d.getElementById('land-rod').textContent);
  const f4 = side('#land?kode=ZZ', LANDV({ land_hent: { land: 'XY', forslag: null } }), 'da', 'windows', reg('da'));
  await sov(70);
  tjek('L ukendt kode i adressen og i indstillingen: vælg selv, aldrig en fejl', !f4.d.querySelector('#land-rod .vg-roed') && !f4.kald.some(k => k[0] === 'land_visning') && f4.d.getElementById('land-rod').textContent.includes('Vælg det land'));
  const f5 = side('#land?kode=SE', LANDV());
  await sov(70);
  tjek('L kode i adressen viser landet uden at gemme det', f5.kald.some(k => k[0] === 'land_visning' && k[1].kode === 'SE') && !f5.kald.some(k => k[0] === 'land_saet') && f5.d.getElementById('nav-land').textContent === 'Danmark lige nu');

  /* Indstillinger */
  const ind = side('#indstillinger', LANDV({ land_hent: { land: 'DK', forslag: 'SE' }, autostart_hent: false, notifikation_hent: { niveau: 'standard', roed: true, zoner: {} }, data_status: { bytes: 1, foerste: null, sidste: null, domaener: 0 } }));
  await sov(80);
  const iv = ind.d.getElementById('indst-land'), fk = ind.d.getElementById('indst-land-forslag');
  tjek('L Indstillinger: landet og forslaget', iv.value === 'DK' && !fk.hidden && fk.textContent === 'Skift til Sverige', iv.value);
  iv.value = 'NO';
  iv.dispatchEvent(new ind.w.Event('change', { bubbles: true }));
  await sov(30);
  tjek('L Indstillinger: et skift gemmes', ind.kald.some(k => k[0] === 'land_saet' && k[1].kode === 'NO') && ind.d.getElementById('nav-land').textContent === 'Norge lige nu', ind.d.getElementById('nav-land').textContent);

  /* Ét land overalt */
  const u = side('#land?kode=US', LANDV({ land_hent: { land: 'US', forslag: null }, autostart_hent: false, notifikation_hent: { niveau: 'standard', roed: true, zoner: {} }, data_status: { bytes: 1, foerste: null, sidste: null, domaener: 0 } }));
  await sov(80);
  u.tast(',', { ctrlKey: true });
  await sov(50);
  const ui2 = u.d.getElementById('indst-land');
  ui2.value = 'DK';
  ui2.dispatchEvent(new u.w.Event('change', { bubbles: true }));
  await sov(40);
  u.tast('6', { ctrlKey: true });
  await sov(80);
  const etLand = (x, navn, k) => x.d.getElementById('land-h').textContent === `${navn} lige nu` && x.d.getElementById('nav-land').textContent === `${navn} lige nu` && x.d.getElementById('vg-land').value === k && x.d.querySelector('#land-faner label').textContent === navn && x.d.querySelector('#land-maal h2').textContent === `${navn} som mål` && x.d.querySelector('#land-kilde h3').textContent === `De største kilder til angreb mod ${navn}`;
  tjek('L ét land: skift i Indstillinger → ⌘6: overskrift, vælger, fane, afsnit og sidebjælke siger samme land', etLand(u, 'Danmark', 'DK') && u.w.location.hash === '#land', `${u.w.location.hash} ${u.d.getElementById('land-h').textContent} ${u.d.getElementById('vg-land').value}`);
  u.emit('visning', 'overblik');
  await sov(40);
  u.emit('visning', 'land');
  await sov(80);
  tjek('L ét land: programmenuen åbner også det gemte land', etLand(u, 'Danmark', 'DK') && u.w.location.hash === '#land', u.w.location.hash);
  const aa = side('#land', LANDV());
  await sov(70);
  aa.d.getElementById('indst-land').value = 'NO';
  aa.d.getElementById('indst-land').dispatchEvent(new aa.w.Event('change', { bubbles: true }));
  await sov(80);
  tjek('L ét land: skift, mens #land er åben, slår igennem med det samme', etLand(aa, 'Norge', 'NO'), aa.d.getElementById('land-h').textContent);
  const kig = side('#land?fane=verden', LANDV());
  await sov(70);
  kig.d.querySelector('#land-rod [data-gaa="land?kode=US&afsnit=maal"]').click();
  await sov(80);
  const hjem = kig.d.querySelector('#land-kig button');
  tjek('L kig: et land fra Verden står overalt på siden, men gemmes ikke', kig.d.getElementById('land-h').textContent === 'USA lige nu' && kig.d.getElementById('vg-land').value === 'US' && kig.d.querySelector('#land-faner label').textContent === 'USA' && kig.d.querySelector('#land-maal h2').textContent === 'USA som mål' && !kig.kald.some(k => k[0] === 'land_saet'), kig.d.getElementById('land-h').textContent);
  tjek('L kig: sidebjælken bliver ved det gemte, og overskriften har vejen hjem', kig.d.getElementById('nav-land').textContent === 'Danmark lige nu' && hjem && hjem.textContent === 'Tilbage til Danmark', hjem && hjem.outerHTML);
  kig.tast('6', { ctrlKey: true });
  await sov(80);
  tjek('L kig: ⌘6 går til det gemte land', etLand(kig, 'Danmark', 'DK') && !kig.d.querySelector('#land-kig button'), kig.w.location.hash);
  kig.w.location.hash = '#land?kode=US';
  await sov(80);
  const hjem2 = kig.d.querySelector('#land-kig button');
  if (hjem2) hjem2.click();
  await sov(80);
  tjek('L kig: "Tilbage til" går til det gemte land', hjem2 && etLand(kig, 'Danmark', 'DK'), kig.w.location.hash);
  tjek('L det gemte lands egne links har ingen kode', [...kig.d.querySelectorAll('#land-faner input')].length === 2 && kig.w.location.hash === '#land');

  /* Første valg */
  const sprogEn = reg('en-US');
  const t1 = side('#land', LANDV({ land_hent: { land: null, forslag: null } }), 'da', 'windows', w2 => { sprogEn(w2); tidszone('Europe/Copenhagen')(w2); });
  await sov(70);
  tjek('L første gang: Europe/Copenhagen + en-US → Danmark', t1.kald.some(k => k[0] === 'land_saet' && k[1].kode === 'DK') && !t1.kald.some(k => k[0] === 'land_saet' && k[1].kode === 'US') && t1.d.getElementById('land-h').textContent === 'Danmark lige nu', JSON.stringify(t1.kald.filter(k => k[0] === 'land_saet')));
  const t2 = side('#land', LANDV({ land_hent: { land: null, forslag: null } }), 'da', 'windows', w2 => { reg('sv-SE')(w2); tidszone('Mars/Olympus')(w2); });
  await sov(70);
  tjek('L første gang: ukendt tidszone → sproget', t2.kald.some(k => k[0] === 'land_saet' && k[1].kode === 'SE'));
  const t3 = side('#land', LANDV({ land_hent: { land: null, forslag: null } }), 'da', 'windows', w2 => { reg('da')(w2); tidszone('Etc/UTC')(w2); });
  await sov(70);
  tjek('L første gang: intet → brugeren vælger', !t3.kald.some(k => k[0] === 'land_saet') && t3.d.getElementById('land-rod').textContent.includes('Vælg det land'));
  const t4 = side('#land', LANDV({ land_hent: { land: 'SE', forslag: null } }), 'da', 'windows', tidszone('Europe/Copenhagen'));
  await sov(70);
  tjek('L tidszonen rører ikke et gemt land', !t4.kald.some(k => k[0] === 'land_saet'));
  tjek('L tidszone-tabellen: kilden står i filen, og hver zone har en landekode', KORT.tidszoner._kilde.includes('zone.tab') && KORT.tidszoner._kilde.includes('public domain') && KORT.tidszoner.zoner['Europe/Copenhagen'] === 'DK' && Object.values(KORT.tidszoner.zoner).every(k => /^[A-Z]{2}$/.test(k)) && Object.keys(KORT.tidszoner.zoner).length > 400);
  tjek('L tidszone-tabellen står i tredjepartslisten', fs.readFileSync(path.join(repo, 'THIRD_PARTY_LICENSES.md'), 'utf8').includes('`tidszoner.json`'));

  /* Knappen og fortryd */
  const tb = side('#indstillinger', LANDV({ land_hent: { land: 'DK', forslag: 'US' }, autostart_hent: false, notifikation_hent: { niveau: 'standard', roed: true, zoner: {} }, data_status: { bytes: 1, foerste: null, sidste: null, domaener: 0 } }));
  await sov(80);
  const skiftK = tb.d.getElementById('indst-land-forslag'), tilK = tb.d.getElementById('indst-land-tilbage');
  tjek('L knappen siger, hvad den gør, og "Tilbage til" står ikke endnu', skiftK.textContent === 'Skift til USA' && tilK.hidden);
  skiftK.click();
  await sov(40);
  tjek('L efter et skift står "Tilbage til <tidligere land>"', !tilK.hidden && tilK.textContent === 'Tilbage til Danmark' && skiftK.hidden && tb.d.getElementById('indst-land').value === 'US', tilK.textContent);
  tilK.click();
  await sov(40);
  tjek('L "Tilbage til" gendanner det tidligere land og forsvinder', tb.d.getElementById('indst-land').value === 'DK' && tilK.hidden && !skiftK.hidden && tb.kald.filter(k => k[0] === 'land_saet').pop()[1].kode === 'DK');
  skiftK.click();
  await sov(40);
  tb.tast('1', { ctrlKey: true });
  await sov(50);
  tb.tast(',', { ctrlKey: true });
  await sov(50);
  tjek('L "Tilbage til" står, indtil man forlader Indstillinger', tilK.hidden, tilK.textContent);
  const ingenF = side('#indstillinger', LANDV({ land_hent: { land: 'DK', forslag: null }, autostart_hent: false, notifikation_hent: { niveau: 'standard', roed: true, zoner: {} }, data_status: { bytes: 1, foerste: null, sidste: null, domaener: 0 } }));
  await sov(80);
  tjek('L uden et tydeligt forslag er der ingen knap', ingenF.d.getElementById('indst-land-forslag').hidden && !ingenF.d.getElementById('visning-indstillinger').textContent.includes('Foreslået'));

  /* Ramme, engelsk, Hvorfor */
  const en = side('#land?kode=DK', LANDV(), 'en');
  await sov(70);
  tjek('L engelsk: navnet følger landet', en.d.getElementById('land-h').textContent === 'Denmark now' && en.d.getElementById('land-rod').textContent.includes('Denmark was the target of 0.4% of the attacks yesterday.') && en.d.querySelector('#land-kilde h3').textContent === 'The largest sources of attacks on Denmark', en.d.getElementById('land-rod').textContent.slice(0, 200));
  const gv = side('', LANDV());
  await sov(70);
  gv.tast('6', { ctrlKey: true });
  await sov(70);
  tjek('L Ctrl+6 på Windows', gv.w.location.hash.startsWith('#land'), gv.w.location.hash);
  gv.tast('1', { ctrlKey: true });
  await sov(40);
  gv.emit('visning', 'land');
  await sov(70);
  tjek('L programmenuen: hændelsen visning åbner landet', gv.w.location.hash.startsWith('#land'), gv.w.location.hash);
  const hv1 = side('', LANDV({ overblik: [{ ...OVERBLIK[0], tendens: 'foelger' }, { ...OVERBLIK[1], tendens: 'saerligt_dig', status: 'gul' }, OVERBLIK[2]] }));
  await sov(70);
  const h = [...hv1.d.querySelectorAll('#overblik-raekker tr')].map(tr => hv(tr).textContent);
  tjek('L Hvorfor: sætningen står før reglerne, så den ikke handler om en regel', h[0].startsWith('30 angreb i går (snit 12,4) – alle stoppet. Følger den globale tendens. Mest et kendt hul i React (20)') && h[1].includes('Verden var normal i går, så stigningen er din egen. Mest '), h.join(' / '));
  tjek('L Hvorfor: status er den samme', hv1.d.querySelector('#overblik-raekker tr .chip').textContent === 'Hold øje');
  const hv2 = side('', LANDV());
  await sov(70);
  tjek('L Hvorfor: uden Radar-tal står der ingen sammenligning', !/tendens|verden var normal/i.test(hv(hv2.d.querySelector('#overblik-raekker tr')).textContent));

  /* Hjælp og tekster */
  const hj = side('#land?kode=DK', LANDV());
  await sov(70);
  tjek('L hjælpen følger visningen med ordene', !hj.d.querySelector('.hjaelp-del[data-hjaelp=land]').hidden && [...hj.d.querySelectorAll('#ord-land dt')].map(x => x.textContent).join() === 'Cloudflare Radar,Indeks,Kilde og mål');
  const ids = ['radar', 'indeks', 'kildemaal', 'tendens'];
  tjek('L BOG: de nye ord findes på begge sprog', ids.every(id => hj.w.hmSprog.bogIds.includes(id) && hj.w.hmSprog.bog(id).alle && en.w.hmSprog.bog(id).alle !== hj.w.hmSprog.bog(id).alle));
  tjek('L slogan: ét sted, på Om og opsætningen', [...hj.d.querySelectorAll('.velkomst .slogan')].length === 2 && hj.d.querySelector('#visning-opsaetning .filosofi-slogan').textContent === 'Sikkerhed er ikke noget, man køber. Det er noget, man forstår.' && hj.d.querySelector('#visning-indstillinger .slogan') && en.d.querySelector('#visning-indstillinger .slogan').textContent === "Security isn't something you buy. It's something you understand.");
  const da = fs.readFileSync(path.join(repo, 'README.da.md'), 'utf8'), enR = fs.readFileSync(path.join(repo, 'README.md'), 'utf8');
  tjek('L begge README: slogan under titlen og afsnittet', da.split('\n').slice(0, 5).join().includes('Sikkerhed er ikke noget, man køber.') && enR.split('\n').slice(0, 5).join().includes("Security isn't something you buy.") && da.includes('## Dit land lige nu og Verden') && enR.includes('## Your country now and the World'));
  tjek('L licens: egen kode er Apache-2.0, W\'s filer står som før', (t => t.includes('Programmets egen kode er under Apache-2.0-licensen') && !t.includes('egen kode er under MIT') && t.includes('samme ophavsmand, MIT'))(fs.readFileSync(path.join(repo, 'THIRD_PARTY_LICENSES.md'), 'utf8')));
  const kode = udenKommentarer(ui('assets/visning-land.js'));
  tjek('L visning-land.js: ingen innerHTML og ingen style=', !/innerHTML|\.style\b|style=/.test(kode));
  const bygge = fs.readFileSync(path.join(repo, 'src-tauri', 'build.rs'), 'utf8');
  tjek('L sikkerhedsporten: de nye kommandoer er tilladt og står i build.rs', ['land_hent', 'land_saet', 'land_visning', 'radar_opdater', 'aabn_radar'].every(c => cap.permissions.includes('allow-' + c.replace(/_/g, '-')) && bygge.includes(`"${c}"`)));
  const konf = JSON.parse(fs.readFileSync(path.join(repo, 'src-tauri', 'tauri.conf.json'), 'utf8'));
  tjek('L sikkerhedsporten: CSP har ingen nye værter', !/cloudflare|radar/i.test(JSON.stringify(konf.app.security)));
}

// ── Domænerne i Indstillinger (D1–D4) ──
/* domaener_saet husker listen, som Rust gør; overblik og valg_hent svarer kun med dem, der er slået til. */
const DOMV = (fravalgte = [], x) => {
  const st = { fra: new Set(fravalgte), nye: new Set((x && x.nye) || []) }, til = () => OVERBLIK.filter(r => !st.fra.has(r.id));
  return { ...MED_TOKEN, st, kortdata: KORT, koe_status: null, koe_prioritet: null, siden_sidst: { vis: false, nye: [], raekker: [] }, autostart_hent: false, notifikation_hent: { niveau: 'standard', roed: true, zoner: {} }, data_status: { bytes: 1, foerste: null, sidste: null, domaener: 0 }, land_hent: { land: 'DK', forslag: null }, login_stier: { stier: [], forslag: [] },
    domaener_hent: () => OVERBLIK.map(r => ({ id: r.id, navn: r.navn, til: !st.fra.has(r.id), ny: st.nye.has(r.id) })), domaener_saet: a => { st.fra = new Set(OVERBLIK.map(r => r.id).filter(id => !a.valgte.includes(id))); for (const id of a.valgte) st.nye.delete(id); return null; }, domaene_afvis: a => { for (const id of a.zoner) st.nye.delete(id); return null; }, overblik: () => JSON.parse(JSON.stringify(til())), valg_hent: () => til().map(r => r.id), ...x };
};
{
  const s = side('#overblik', DOMV(['z2', 'z3']));
  await sov(70);
  const { d } = s, rk = [...d.querySelectorAll('#overblik-raekker tr')], note = d.getElementById('fravalgt-note');
  tjek('D2 Overblik: et fravalgt domæne står ikke i tabellen', rk.length === 1 && rk[0].textContent.includes('wi24rd.com') && !d.getElementById('overblik-tabel').textContent.includes('xyz.wi24rd.com'));
  tjek('D4 titlen tæller kun dem, der er til', d.getElementById('overblik-antal').textContent === '· 1 domæne', d.getElementById('overblik-antal').textContent);
  tjek('D4 de fravalgte nævnes ét sted, og linjen er klikbar', !note.hidden && note.textContent === '2 domæner er ikke valgt — Indstillinger' && note.dataset.gaa === 'indstillinger?afsnit=domaener', note.textContent);
  note.click();
  await sov(60);
  tjek('D4 linjen åbner Indstillinger', synlige(d).join() === 'indstillinger' && s.w.location.hash === '#indstillinger?afsnit=domaener', s.w.location.hash);

  const e1 = side('#overblik', DOMV(['z1']), 'en');
  await sov(70);
  tjek('D4 engelsk og ental', e1.d.getElementById('fravalgt-note').textContent === '1 domain is not chosen — Settings' && e1.d.getElementById('samlet').textContent === 'Attacks for all');

  const tom = side('#overblik', DOMV(['z1', 'z2', 'z3']));
  await sov(70);
  const boks = tom.d.getElementById('overblik-tom');
  tjek('Tom: intet valgt giver en venlig tekst med en knap', !boks.hidden && boks.textContent.includes('Du har ikke valgt nogen domæner endnu') && boks.textContent.includes('Vælg de domæner, Huginn skal holde øje med. Kun dem hentes og vises.') && boks.querySelector('button')?.textContent === 'Vælg domæner' && boks.querySelector('button')?.dataset.gaa === 'indstillinger?afsnit=domaener');
  tjek('Tom: tabel, fodnote, domslinje og knap er væk', tom.d.getElementById('overblik-rul').hidden && tom.d.getElementById('overblik-note').hidden && tom.d.getElementById('dom').hidden && tom.d.getElementById('samlet').hidden && tom.d.getElementById('fravalgt-note').hidden);
  const ang = side('#vaernet?samlet=1', DOMV(['z1', 'z2', 'z3']));
  await sov(70);
  tjek('Tom: Angreb viser velkomsten med samme tekst og knap', ang.d.getElementById('vagt').textContent.includes('Du har ikke valgt nogen domæner endnu') && ang.d.querySelector('#vagt .velkomst-situation button').dataset.gaa === 'indstillinger?afsnit=domaener', ang.d.getElementById('vagt').textContent);
  for (const [navn, hash, rod] of [['Robotter', '#robotter?samlet=1', 'robotter-rod'], ['Login', '#login?samlet=1', 'login-rod'], ['dyk', '#dyk?samlet=1', 'dyk-rod']]) {
    const x = side(hash, DOMV(['z1', 'z2', 'z3'], VIS));
    await sov(70);
    tjek(`Tom: ${navn} viser velkomsten`, x.d.querySelector(`#${rod} .velkomst-situation`)?.textContent.includes('Du har ikke valgt nogen domæner endnu') && x.d.querySelector(`#${rod} .velkomst-situation button`)?.textContent === 'Vælg domæner', x.d.getElementById(rod).textContent.slice(0, 120));
    tjek(`Tom: ${navn} følger det gemte land`, x.d.getElementById(rod).textContent.includes('hvordan Danmark og verden'), x.d.getElementById(rod).textContent.slice(300, 520));
  }

  /* Væk overalt */
  const v = side('#vaernet?samlet=1', DOMV(['z2']));
  await sov(70);
  tjek('D2 domænevælgeren har ikke det fravalgte', ![...v.d.getElementById('vg-zone').options].some(o => o.value === 'z2') && [...v.d.getElementById('vg-zone').options][0].textContent === 'Samlet (2)', [...v.d.getElementById('vg-zone').options].map(o => o.textContent).join());
  tjek('D2 summen spørger kun om dem, der er til', v.kald.some(k => k[0] === 'vaern_samlet_cache' && JSON.stringify(k[1].zoner) === '["z1","z3"]'), JSON.stringify(v.kald.filter(k => k[0] === 'vaern_samlet_cache')));
  const gl = side('#robotter?zone=z2&navn=b.wi24rd.com', DOMV(['z2'], { ...VIS }));
  await sov(90);
  tjek('D2 en gammel adresse til et fravalgt domæne viser det ikke', gl.w.location.hash === '#robotter?samlet=1' && !gl.kald.some(k => k[0] === 'robotter' && k[1].omfang.zone === 'z2' && gl.kald.indexOf(k) > gl.kald.findIndex(x => x[0] === 'domaener_hent')), gl.w.location.hash);
  const gl2 = side('#overblik', DOMV(['z2']));
  await sov(70);
  gl2.w.location.hash = '#dyk?zone=z2&navn=b.wi24rd.com&regel=x';
  await sov(70);
  tjek('D2 dyk på et fravalgt domæne går til Overblik', gl2.w.location.hash === '#overblik' && synlige(gl2.d).join() === 'overblik', gl2.w.location.hash);
  tjek('D2 Pr. domæne og Login-stier har ikke det fravalgte', (x => !x.includes('xyz.wi24rd.com'))(gl2.d.getElementById('pr-domaene-liste').textContent + gl2.d.getElementById('login-domaene').textContent));

  /* Indstillinger → Domæner */
  const i = side('#indstillinger', DOMV(['z2']));
  await sov(80);
  const liste = () => [...i.d.querySelectorAll('#domaener-liste tr')], erNavn = (r, n) => r.textContent.includes(n) && !r.textContent.includes('.' + n), kontakt = n => liste().find(r => erNavn(r, n)).querySelector('input');
  tjek('Domæner: alle domæner står, også det fravalgte', liste().length === 3 && kontakt('xyz.wi24rd.com').checked === false && kontakt('wi24rd.com').checked === true);
  const k0 = kontakt('wi24rd.com');
  tjek('Domæner: kontakten er en switch med navn', k0.getAttribute('role') === 'switch' && k0.className === 'kontakt' && k0.getAttribute('aria-label') === 'Vis wi24rd.com i Huginn', k0.outerHTML);
  tjek('Domæner: forklaringen står', i.d.getElementById('visning-indstillinger').textContent.includes('Vælg de domæner, Huginn skal hente og vise. Et domæne, du fravælger, hentes ikke længere; dets tal bliver liggende.') && i.d.getElementById('domaener-alle-til').textContent === 'Vælg alle' && i.d.getElementById('domaener-alle-fra').textContent === 'Fravælg alle');
  tjek('Domæner: API Tokenet ser N domæner', i.d.getElementById('domaener-ser').textContent === 'API Tokenet ser 3 domæner' && !i.d.getElementById('domaener-ser').hidden, i.d.getElementById('domaener-ser').textContent);
  const navn = n => liste().find(r => erNavn(r, n)).querySelector('th');
  tjek('Domæner: navnet åbner Angreb; et fravalgt domæne er væk, så det kan ikke åbnes', navn('wi24rd.com').querySelector('button').dataset.gaa.startsWith('vaernet?zone=z1') && !navn('xyz.wi24rd.com').querySelector('button'));
  kontakt('xyz.wi24rd.com').click();
  await sov(40);
  const sidst = () => i.kald.filter(k => k[0] === 'domaener_saet').pop()[1].valgte.slice().sort().join();
  tjek('Domæner: et domæne vælges: de valgte gemmes, og rammen henter igen', sidst() === 'z1,z2,z3' && i.w.hmRamme.tilstand.raekker.length === 3 && navn('xyz.wi24rd.com').querySelector('button'), JSON.stringify(i.kald.filter(k => k[0] === 'domaener_saet')));
  kontakt('abe.wi24rd.com').click();
  await sov(40);
  tjek('Domæner: et domæne fravælges', sidst() === 'z1,z2' && i.d.getElementById('domaener-status').textContent === 'Domænerne er gemt.', sidst());
  i.d.getElementById('domaener-alle-til').click();
  await sov(40);
  tjek('Domæner: "Vælg alle"', sidst() === 'z1,z2,z3' && i.w.hmRamme.tilstand.uvalgt.size === 0 && i.d.getElementById('domaener-alle-til').disabled);
  i.d.getElementById('domaener-alle-fra').click();
  await sov(40);
  tjek('Domæner: "Fravælg alle" uden dialog', sidst() === '' && i.w.hmRamme.tilstand.uvalgt.size === 3 && i.d.getElementById('domaener-alle-fra').disabled && !i.d.getElementById('domaener-alle-til').disabled && !i.d.getElementById('domaener-dialog'), sidst());
  i.tast('1', { ctrlKey: true });
  await sov(50);
  tjek('Domæner: "Fravælg alle" giver tom-tilstanden', !i.d.getElementById('overblik-tom').hidden && i.d.getElementById('overblik-rul').hidden);
  const eneste = side('#indstillinger', DOMV(['z2', 'z3']));
  await sov(80);
  [...eneste.d.querySelectorAll('#domaener-liste input')].find(x => x.checked).click();
  await sov(30);
  tjek('Domæner: det sidste domæne kan fravælges uden dialog', eneste.kald.filter(k => k[0] === 'domaener_saet').pop()[1].valgte.length === 0);
  const ufejl = side('#indstillinger', DOMV([], { domaener_saet: () => { throw { kode: 'lager' }; } }));
  await sov(80);
  const uk = ufejl.d.querySelector('#domaener-liste input');
  uk.click();
  await sov(30);
  tjek('Domæner: en fejl vises, og kontakten går tilbage', ufejl.d.getElementById('domaener-status').textContent.includes('database') && ufejl.d.querySelector('#domaener-liste input').checked);
  tjek('Sikkerhedsporten: valg_gem er væk, de nye kommandoer er tilladt', !cap.permissions.includes('allow-valg-gem') && ['domaener_hent', 'domaener_saet', 'domaene_afvis'].every(c => cap.permissions.includes('allow-' + c.replace(/_/g, '-')) && fs.readFileSync(path.join(repo, 'src-tauri', 'build.rs'), 'utf8').includes(`"${c}"`)) && !/valg_gem/.test(fs.readFileSync(path.join(repo, 'src-tauri', 'build.rs'), 'utf8')));
  const glemt = side('#indstillinger', DOMV(['z2'], { token_status: { sat: false } }));
  await sov(60);
  tjek('Uden nøgle: ingen domæner og ingen fravalgte', glemt.w.hmRamme.tilstand.uvalgt.size === 0 && glemt.d.getElementById('domaener-liste').textContent.includes('Domænerne står her'));
  const nat = side('', DOMV([], { overblik: [{ ...OVERBLIK[0], tendens: 'saerligt_dig', opdateres: true }] }));
  await sov(70);
  tjek('Natten: om natten står "i forgårs"', hv(nat.d.querySelector('#overblik-raekker tr')).textContent.includes('Verden var normal i forgårs, så stigningen er din egen.'));
  /* T3: nye domæner */
  const n1 = side('#overblik', DOMV(['z3'], { nye: ['z3'] }));
  await sov(70);
  const linje = n1.d.getElementById('nye-domaener'), lk = () => [...linje.querySelectorAll('button')];
  tjek('T3 ét nyt domæne: linjen står over tabellen', !linje.hidden && linje.textContent === '1 nyt domæne på din Cloudflare-konto: abe.wi24rd.com — Vælg · Ikke nu' && Boolean(linje.compareDocumentPosition(n1.d.getElementById('overblik-rul')) & 4), linje.textContent);
  tjek('T3 knapperne siger, hvad de gør', lk()[0].getAttribute('aria-label') === 'Vælg abe.wi24rd.com' && lk()[1].getAttribute('aria-label') === 'Ikke nu: abe.wi24rd.com', lk().map(b => b.getAttribute('aria-label')).join());
  tjek('T3 et nyt domæne er ikke med i tabellen', ![...n1.d.querySelectorAll('#overblik-raekker tr')].some(r => r.textContent.includes('abe.wi24rd.com')));
  lk()[0].click();
  await sov(40);
  tjek('T3 "Vælg" tilføjer domænet til de valgte, og linjen forsvinder', n1.kald.filter(k => k[0] === 'domaener_saet').pop()?.[1].valgte.slice().sort().join() === 'z1,z2,z3' && linje.hidden && n1.w.hmRamme.tilstand.raekker.length === 3, JSON.stringify(n1.kald.filter(k => k[0] === 'domaener_saet')));
  const n2 = side('#overblik', DOMV(['z3'], { nye: ['z3'] }));
  await sov(70);
  const l2 = n2.d.getElementById('nye-domaener');
  l2.querySelectorAll('button')[1].click();
  await sov(40);
  tjek('T3 "Ikke nu" huskes for domænet, og linjen forsvinder', JSON.stringify(n2.kald.filter(k => k[0] === 'domaene_afvis').pop()?.[1].zoner) === '["z3"]' && l2.hidden && !n2.kald.some(k => k[0] === 'domaener_saet'));
  const n3 = side('#overblik', DOMV(['z2', 'z3'], { nye: ['z2', 'z3'] }));
  await sov(70);
  const l3 = n3.d.getElementById('nye-domaener'), b3 = [...l3.querySelectorAll('button')];
  tjek('T3 flere: antal og "Se dem" til Indstillinger', l3.textContent === '2 nye domæner på din Cloudflare-konto — Se dem · Ikke nu' && b3[0].dataset.gaa === 'indstillinger?afsnit=domaener', l3.textContent);
  b3[1].click();
  await sov(40);
  tjek('T3 flere: "Ikke nu" gælder dem alle', n3.kald.filter(k => k[0] === 'domaene_afvis').pop()?.[1].zoner.slice().sort().join() === 'z2,z3' && l3.hidden);
  const n4 = side('#indstillinger', DOMV(['z3'], { nye: ['z3'] }));
  await sov(80);
  tjek('T3 Indstillinger mærker det nye domæne', [...n4.d.querySelectorAll('#domaener-liste tr')].find(r => r.textContent.includes('abe.wi24rd.com')).querySelector('.ny')?.textContent === 'ny');
  const n5 = side('#overblik', DOMV(['z3'], { nye: ['z3'] }), 'en');
  await sov(70);
  tjek('T3 engelsk', n5.d.getElementById('nye-domaener').textContent === '1 new domain on your Cloudflare account: abe.wi24rd.com — Choose · Not now', n5.d.getElementById('nye-domaener').textContent);
  const n6 = side('#overblik', DOMV());
  await sov(70);
  tjek('T3 ingen nye: ingen linje', n6.d.getElementById('nye-domaener').hidden);
  tjek('Natten: de gamle ord er væk', !/verden er flad|world is flat/i.test(ui('assets/sprog.js') + fs.readFileSync(path.join(repo, 'README.md'), 'utf8') + fs.readFileSync(path.join(repo, 'README.da.md'), 'utf8')));
  tjek('README: Domæner på begge sprog', fs.readFileSync(path.join(repo, 'README.da.md'), 'utf8').includes('Indstillinger → Domæner') && fs.readFileSync(path.join(repo, 'README.md'), 'utf8').includes('Settings → Domains'));
}

// ── Velkomsten ──
{
  const fælles = v => [...v.children].filter(e => !e.matches('.velkomst-situation')).map(e => e.textContent).join('|');
  const s = side('#overblik', DOMV(['z1', 'z2', 'z3'], { land_hent: { land: 'DK', forslag: null } }));
  await sov(90);
  const { d, w } = s, steder = ['opsaetning-punkter', 'intet-valgt', 'om'].map(x => d.querySelector(`[data-velkomst="${x}"] .velkomst`));
  tjek('W velkomsten findes de tre steder', steder.every(Boolean) && d.querySelector('#visning-opsaetning .velkomst') === steder[0] && d.querySelector('#visning-overblik .velkomst') === steder[1] && d.querySelector('#visning-indstillinger .velkomst') === steder[2]);
  tjek('W samme indhold i Om og intet valgt', fælles(steder[1]) === fælles(steder[2]), fælles(steder[1]).slice(0, 200));
  tjek('W samme punkter alle tre steder', new Set(steder.map(x => x.querySelector('.velkomst-punkter').textContent)).size === 1);
  const v = steder[2], punkter = [...v.querySelectorAll('.velkomst-punkter li')];
  tjek('W navnet: Wi24rd med 24 for sig, Huginn and Muninn, for Cloudflare®', v.querySelector('.velkomst-wi').textContent === 'Wi24rd' && v.querySelector('.velkomst-wi em').textContent === '24' && v.querySelector('.velkomst-hm').textContent === 'Huginn and Muninn');
  tjek('W "for Cloudflare®" med lille f, på begge sprog', v.querySelector('.velkomst-cf').textContent === 'for Cloudflare®' && w.hmSprog.T.under === 'for Cloudflare®');
  tjek('W ingen tekst i et billede: ravnene er en tom maske uden <img>', !v.querySelector('img') && v.querySelector('.tom-ravn').textContent === '' && v.querySelector('.tom-ravn').getAttribute('aria-hidden') === 'true');
  tjek('W sloganet og overskriften', v.querySelector('.slogan').textContent === w.hmSprog.T.slogan && v.querySelector('.velkomst-h').textContent === 'Det her kan du forvente');
  const form = e => e.querySelector('path, rect, circle').outerHTML;
  const kilder = ['#side a[data-visning="vaernet"] svg', '#visning-indstillinger h2:has(span[data-t="niveauH"]) svg', '#side a[data-visning="overblik"] svg', '#side a[data-visning="siden-sidst"] svg', '#side a[data-visning="login"] svg'];
  const klokke = [...d.querySelectorAll('#visning-indstillinger h2')].find(h => h.textContent === 'Notifikationer').querySelector('svg');
  const ikoner = kilder.map((q, i) => (i === 1 ? klokke : d.querySelector(q)));
  tjek('W fem punkter med skjold, klokke, gitter, ur og lås fra sidebjælken og Indstillinger', punkter.length === 5 && punkter.every((p, i) => p.querySelector('svg.ik') && form(p.querySelector('svg')) === form(ikoner[i])), String(punkter.length));
  tjek('W punkt 1 bruger programmets egne statusnavne', punkter[0].textContent === 'Huginn holder øje med sikkerheden på alle dine Cloudflare-domæner: angreb, robotter og login-forsøg, samlet på én skærm med en dom i klart sprog: Roligt, Hold øje eller Handl.', punkter[0].textContent);
  tjek('W punkt 2 ordret', punkter[1].textContent === 'Du får besked, når det betyder noget: notifikationer fra din egen computer, når et domæne kræver handling — du bestemmer hvornår.', punkter[1].textContent);
  tjek('W punkt 3 har det gemte land', punkter[2].textContent === 'Dyk ned i det, der skete: klik på et tal og se reglerne, landene, netværkene, stierne og robotterne bag — og hvordan Danmark og verden har det.', punkter[2].textContent);
  tjek('W punkt 4 og 5 ordret', punkter[3].textContent === 'Muninn husker: tallene gemmes på din computer, også efter Cloudflares måned.' && punkter[4].textContent === 'Kun læse, ingen telemetri: programmet kan ikke ændre noget i Cloudflare, og intet sendes til Wi24rd.', punkter[3].textContent);
  tjek('W landet står alle tre steder', steder.every(x => x.textContent.includes('hvordan Danmark og verden')));
  w.hmVelkomst.land('Norge');
  tjek('W et nyt land slår igennem alle tre steder', steder.every(x => x.textContent.includes('hvordan Norge og verden')));
  w.hmVelkomst.land(null);
  tjek('W uden land: "dit land"', steder.every(x => x.textContent.includes('hvordan dit land og verden')) && !steder[0].textContent.includes('{'));
  tjek('W Om har ingen knap', !steder[2].querySelector('button') && d.getElementById('om-version') && d.querySelector('.om').textContent.includes('Cloudflare, Inc.'));
  const knap = steder[1].querySelector('button');
  tjek('W intet valgt: teksten og "Vælg domæner"', steder[1].querySelector('.velkomst-situation')?.textContent.includes('Du har ikke valgt nogen domæner endnu') && knap?.textContent === 'Vælg domæner' && knap?.dataset.gaa === 'indstillinger?afsnit=domaener' && knap?.getAttribute('aria-label') === 'Vælg domæner');
  const ops = d.getElementById('visning-opsaetning'), foer = (x, y) => Boolean(x.compareDocumentPosition(y) & 4);
  const top = ops.querySelector('.velkomst-kompakt'), felt = d.getElementById('token-form'), fil = ops.querySelector('.filosofi');
  tjek('W opsætningen: logo og navn, formularen, punkterne og filosofien i den rækkefølge', top && foer(top, d.getElementById('opsaetning-h')) && foer(d.getElementById('opsaetning-h'), felt) && foer(felt, steder[0]) && foer(steder[0], fil) && top?.querySelector('.tom-ravn') && top?.querySelector('.velkomst-cf').textContent === 'for Cloudflare®' && d.querySelector('#visning-opsaetning .hm-hjaelp'));
  tjek('W opsætningen: ingen knap i velkomsten', !steder[0].querySelector('button') && !top?.querySelector('button') && !fil.querySelector('button'));
  tjek('W opsætningen: h1 er første overskrift, og navnet er ikke en overskrift', ops.querySelector('h1, h2, h3') === d.getElementById('opsaetning-h') && !top?.querySelector('h1, h2, h3') && top?.querySelector('.velkomst-navn').tagName === 'P');
  tjek('W opsætningen: sloganet står én gang, i filosofien', ops.textContent.split(w.hmSprog.T.slogan).length === 2 && fil.querySelector('.filosofi-slogan').textContent === w.hmSprog.T.slogan && !ops.querySelector('.slogan'));
  tjek('W filosofien står kun i opsætningen', d.querySelectorAll('.filosofi').length === 1 && !d.getElementById('visning-indstillinger').textContent.includes('filosofi') && !d.getElementById('visning-overblik').textContent.includes('filosofi'));
  tjek('W Om og intet valgt har sloganet under navnet', steder.slice(1).every(x => x.querySelector('.velkomst-navn + .slogan')));
  tjek('W filosofien ordret på dansk', [...fil.children].map(e => e.textContent).join('|') === 'Wi24rd — vores filosofi|Sikkerhed er ikke noget, man køber. Det er noget, man forstår.|Vi mener, at cybersikkerhed skal være tilgængelig for alle – ikke kun for store virksomheder med dyre værktøjer og egne sikkerhedsteams.|Ingen falske løfter. Ingen unødig kompleksitet. Bare praktiske værktøjer, klare indsigter og den viden, der skal til for at træffe bedre sikkerhedsbeslutninger.|Lavet til at styrke dig. Ikke til at gøre dig afhængig.', [...fil.children].map(e => e.textContent).join('|'));
  tjek('W filosofien: versalerne er CSS, ikke tekst', /\.filosofi-h \{[^}]*text-transform: uppercase/.test(ui('assets/app.css')) && fil.querySelector('h2').textContent !== fil.querySelector('h2').textContent.toUpperCase());
  tjek('W opsætningen: fokus i nøglefeltet ruller ikke velkomsten væk', /\$\('token'\)\.focus\(\{ preventScroll: true \}\)/.test(ui('assets/ramme.js')));
  const en = side('#overblik', DOMV(['z1', 'z2', 'z3'], { land_hent: { land: null, forslag: null } }), 'en', 'windows', w2 => Object.defineProperty(w2.navigator, 'language', { value: 'en', configurable: true }));
  await sov(90);
  const ev = en.d.querySelector('[data-velkomst="om"] .velkomst'), ep = [...ev.querySelectorAll('.velkomst-punkter li')];
  tjek('W engelsk: overskrift, statusnavne og "your country"', ev.querySelector('.velkomst-h').textContent === 'What to expect' && ep[0].textContent === 'Huginn watches the security of all your Cloudflare domains: attacks, robots and login attempts, on one screen with a plain-language verdict: Calm, Watch or Act.' && ep[2].textContent === 'Dig into what happened: click a number to see the rules, countries, networks, paths and robots behind it — and how your country and the world are doing.', ep[2].textContent);
  tjek('W engelsk: de andre punkter ordret', ep[1].textContent === 'You are told when it matters: notifications from your own computer when a domain needs action — you decide when.' && ep[3].textContent === "Muninn remembers: the numbers are kept on your computer, beyond Cloudflare's month." && ep[4].textContent === 'Read-only, no telemetry: the program cannot change anything in Cloudflare, and nothing is sent to Wi24rd.', ep[3].textContent);
  tjek('W engelsk: intet valgt', en.d.querySelector('#overblik-tom .velkomst-situation').textContent.includes("You haven't chosen any domains yet") && en.d.querySelector('#overblik-tom .velkomst-situation').textContent.includes('Choose the domains Huginn should watch. Only those are fetched and shown.') && en.d.querySelector('#overblik-tom button').textContent === 'Choose domains');
  tjek('W filosofien ordret på engelsk', [...en.d.querySelector('.filosofi').children].map(e => e.textContent).join('|') === "Wi24rd — our philosophy|Security isn't something you buy. It's something you understand.|We believe cybersecurity should be accessible to everyone — not just enterprises with expensive tools and dedicated security teams.|No false promises. No unnecessary complexity. Just practical tools, clear insights, and the knowledge to make better security decisions.|Built to empower. Not to create dependency.", en.d.querySelector('.filosofi').textContent);
  tjek('W engelsk: "for Cloudflare®" med lille f', ev.querySelector('.velkomst-cf').textContent === 'for Cloudflare®');
  tjek('W tokens: 24 og navnet bruger --color-accent-text, intet nyt farvetoken', /\.velkomst-wi em \{[^}]*color: var\(--color-accent-text\)/.test(ui('assets/app.css')) && /\.velkomst-hm \{[^}]*color: var\(--color-accent-text\)/.test(ui('assets/app.css')) && !/cyan/i.test(ui('assets/tokens.css')));
  tjek('W velkomst.js: ingen innerHTML og ingen style=', !/innerHTML|\.style\b|style=/.test(udenKommentarer(ui('assets/velkomst.js'))));
  tjek('W ravnene i målbilledet er kilden, og ingen ny billedfil', fs.existsSync(path.join(repo, 'design/brand/hm-logo-transparent.png')) && !fs.readdirSync(path.join(repo, 'ui/assets')).some(f => /logo/i.test(f)));
}

// ── Intet i et kort står nærmere end 16 px fra kanten (målt i en rigtig browser; jsdom har ingen layout) ──
{
  const CHROME = [process.env.HM_CHROME, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome', '/usr/bin/chromium'].find(f => f && fs.existsSync(f));
  if (!CHROME) console.log('SPRUNGET OVER M margen: Chrome findes ikke (sæt HM_CHROME)');
  else {
    const mappe = fs.mkdtempSync(path.join(os.tmpdir(), 'hm-margen-'));
    fs.cpSync(path.join(repo, 'ui'), path.join(mappe, 'ui'), { recursive: true });
    const zoner = [{ id: 'z1', navn: 'eksempel.wi24rd.com' }, { id: 'z2', navn: 'et-meget-langt-domaenenavn-som-skal-kunne-staa-her.wi24rd.com' }];
    const svar = {
      token_status: { sat: true }, zones_cache: { zoner, hentet: NU_ISO }, zones: zoner, overblik: OVERBLIK, valg_hent: ['z1'], koe_status: null, domaener_hent: zoner.map((z, i) => ({ ...z, til: i === 0 })), autostart_hent: true, notifikation_hent: { niveau: 'standard', roed: true, zoner: {} },
      data_status: { bytes: 8e6, foerste: '2026-10-04', sidste: '2026-11-03', domaener: 3 }, siden_sidst: { vis: false, nye: [], raekker: [] }, land_hent: { land: 'DK', forslag: 'US' },
      kortdata: { verden: { lande: { DK: [0, 0], US: [0, 0], VC: [0, 0] }, colo: {}, net: {}, land: '' }, colo: {}, tidszoner: { zoner: {} } },
      login_stier: { stier: [{ sti: '/wp-login.php', til: true, start: true }, { sti: '/wp-admin/', til: false, start: true }, { sti: '/administrator/', til: false, start: true }, { sti: '/user/login', til: false, start: true }, { sti: '/min/egen/meget/lange/login/sti/som/skal/kunne/brydes/uden/at/noget/stikker/ud.php', til: true, start: false }], forslag: ['/api/auth/signin', '/en/meget/lang/sti/til/noget/der/ligner/et/login/og/fylder/hele/raekken/og/lidt/til.php', '/signup', '/register', '/konto/log-ind'] },
    };
    const maal = `setTimeout(() => { const ud = [], ind = 16 - .5; for (const k of document.querySelectorAll('section:not([hidden]) .kort, section:not([hidden]) .velkomst')) { const r = k.getBoundingClientRect(); if (!r.width) continue;
      for (const e of k.querySelectorAll('*')) { if (e.closest('table, .sr-only, dialog') || e.hidden) continue; const kontrol = e.matches('input, select, button, textarea, summary'), tekst = [...e.childNodes].filter(n => n.nodeType === 3 && n.textContent.trim());
        if (!kontrol && !tekst.length) continue; /* Teksten selv, ikke boksen: en boks med padding må gerne gå ud til kanten. */
        const felter = kontrol ? [e.getBoundingClientRect()] : tekst.flatMap(n => { const g = document.createRange(); g.selectNodeContents(n); return [...g.getClientRects()]; });
        for (const b of felter) { if (!b.width || !b.height) continue; const v = b.left - r.left, h = r.right - b.right; if (v < ind || h < ind) ud.push([e.tagName, e.className, e.id, Math.round(v), Math.round(h)].join(' ')); } } }
      const sek = document.querySelector('section:not([hidden])'), v = sek.querySelector('.velkomst-navn'), st = q => getComputedStyle(sek.querySelector(q)); let bag = v, bg = 'rgba(0, 0, 0, 0)';
      while (bag && bg === 'rgba(0, 0, 0, 0)') { bg = getComputedStyle(bag).backgroundColor; bag = bag.parentElement; }
      const velkomst = v && { hm: parseFloat(st('.velkomst-hm').fontSize), cf: parseFloat(st('.velkomst-cf').fontSize), wi: parseFloat(st('.velkomst-wi').fontSize), h: parseFloat(st('.velkomst-h').fontSize), f24: st('.velkomst-wi em').color, hmFarve: st('.velkomst-hm').color, bg };
      const felt = document.getElementById('token'), fr = felt.getBoundingClientRect(), hoved = document.getElementById('indhold');
      const segmenter = [...sek.querySelectorAll('.segment')].map(g => { const r = g.getBoundingClientRect(), k = [...g.querySelectorAll('span')]; return { id: g.id, v: r.left, h: r.right, b: r.width, knapper: k.map(x => x.getBoundingClientRect().width), afkortet: k.filter(x => x.scrollWidth > x.clientWidth).length }; });
      const nl = document.getElementById('nav-land'), nf = parseFloat(getComputedStyle(nl).fontSize);
      const nav = { tekst: nl.textContent, title: nl.title, linjer: nl.getBoundingClientRect().height < nf * 2, afskaaret: nl.scrollWidth > nl.clientWidth, ellipse: getComputedStyle(nl).textOverflow };
      const nl2 = document.getElementById('nye-domaener'), nyLinje = nl2 && !nl2.hidden ? { over: [...document.querySelectorAll('body *')].filter(e => !e.closest('.tabel-rul') && e.getBoundingClientRect().right > innerWidth + 1).slice(0, 6).map(e => e.tagName + '.' + e.className + '#' + e.id), bredde: document.documentElement.scrollWidth, inde: innerWidth, h: Math.max(...[...nl2.querySelectorAll('*')].map(x => x.getBoundingClientRect().right)), plads: nl2.getBoundingClientRect().right } : null;
      const felter = { top: fr.top, bund: fr.bottom, hoejde: fr.height, vindue: innerHeight, rul: hoved.scrollTop, synlig: fr.width > 0 };
      const sl = document.getElementById('server-linje'), serverOver = Boolean(sl && !sl.hidden && [...sl.querySelectorAll('*'), sl].some(e => e.getBoundingClientRect().right > innerWidth + 1));
      const hv = [...document.querySelectorAll('#overblik-raekker .hv-tekst')].map(e => ({ linjer: Math.round(e.getBoundingClientRect().height / parseFloat(getComputedStyle(e).lineHeight)), mere: Boolean(e.nextElementSibling && e.nextElementSibling.classList.contains('hv-mere')) }));
      let kal = null; const kb = document.querySelector('.dato-knap'); if (location.hash.includes('kal=1') && kb) { kb.click(); const b = document.querySelector('.kal').getBoundingClientRect(); kal = { v: b.left, h: b.right, bund: b.bottom, vindue: innerWidth, dage: document.querySelectorAll('.kal-dag').length, vandret: document.documentElement.scrollWidth > innerWidth }; }
      const tb = document.getElementById('teknologi-rod'), tr = document.querySelector('.tek-rul'), rTh = document.querySelector('.tek-gitter tbody th'), hTh = document.querySelector('.tek-gitter thead th:nth-child(2)');
      const tabel = tr && rTh ? { boks: tr.clientWidth, indhold: tr.scrollWidth, ude: tr.getBoundingClientRect().right > tb.getBoundingClientRect().right + 1, foerste: getComputedStyle(rTh).position, top: getComputedStyle(hTh).position, radio: Math.min(...[...document.querySelectorAll('.tek-radio label')].map(l => l.getBoundingClientRect().width)) } : null;
      if (tr && tabel) { tr.scrollLeft = 80; tabel.rul = tr.scrollLeft; tabel.fast = Math.abs(rTh.getBoundingClientRect().left - tr.getBoundingClientRect().left) <= 2; }
      const ov = document.getElementById('overblik-rul'), overblik = ov && !ov.hidden && document.querySelector('#overblik-raekker th') ? { boks: ov.clientWidth, indhold: ov.scrollWidth, foerste: getComputedStyle(document.querySelector('#overblik-raekker th')).position } : null;
      if (overblik) { const h = document.querySelector('.hovedomr'); overblik.fladen = h.scrollWidth <= h.clientWidth; ov.scrollLeft = 80; overblik.fast = Math.abs(document.querySelector('#overblik-raekker th').getBoundingClientRect().left - ov.getBoundingClientRect().left) <= 2; }
      document.body.textContent = 'MAALT:' + JSON.stringify({ tabel, overblik, hv, kal, velkomst, felter, segmenter, nav, nyLinje, serverOver, n: document.querySelectorAll('section:not([hidden]) .kort *').length, login: Boolean(document.querySelector('#login-panel .login-valg')), domaener: document.querySelectorAll('#domaener-liste input').length, vandret: document.documentElement.scrollWidth > innerWidth, ud }) + ':SLUT'; }, 1500);`;
    const koer = (bredde, tekst, tema = 'light', hash = '#indstillinger?zone=z2', ekstra = {}, { sprog = 'da', hoejde = 1400 } = {}) => {
      const stub = `<script>localStorage.setItem('hm-sprog',${JSON.stringify(sprog)});localStorage.setItem('hm-tekst',${JSON.stringify(tekst)});localStorage.setItem('hm-tema',${JSON.stringify(tema)});window.__HM_PLATFORM__='macos';const SVAR=${JSON.stringify({ ...svar, ...ekstra })};window.__TAURI__={core:{invoke:async c=>(c in SVAR?JSON.parse(JSON.stringify(SVAR[c])):null)},event:{listen:async()=>()=>{}},app:{getVersion:async()=>'9.8.7'}};</script>`;
      const html = ui('index.html').replace('<script src="assets/tema.js"></script>', UR_STUB + stub + '<script src="assets/tema.js"></script>').replace('</body>', `<script>addEventListener('load',()=>{${maal}});</script></body>`);
      const navn = `${bredde}-${hoejde}-${tekst}-${tema}-${sprog}-${hash.length}-${JSON.stringify(ekstra).length}`, fil = path.join(mappe, 'ui', `maal-${navn}.html`);
      fs.writeFileSync(fil, html);
      const p = spawnSync(CHROME, ['--headless=new', '--disable-gpu', '--hide-scrollbars', `--user-data-dir=${path.join(mappe, 'p-' + navn)}`, `--window-size=${bredde},${hoejde}`, '--virtual-time-budget=6000', '--dump-dom', pathToFileURL(fil).href + hash], { encoding: 'utf8', timeout: 60000 });
      const m = /MAALT:(.*):SLUT/s.exec(p.stdout || '');
      return m ? JSON.parse(m[1].replace(/&amp;/g, '&')) : null;
    };
    for (const [bredde, tekst] of [[1200, 'normal'], [900, 'large']]) {
      const r = koer(bredde, tekst);
      tjek(`M Indstillinger ved ${bredde} px, ${tekst}: siden er målt, og Login-stierne og Domæner er med`, r && r.n > 50 && r.login && r.domaener === 2, JSON.stringify(r).slice(0, 200));
      tjek(`M Indstillinger ved ${bredde} px, ${tekst}: intet i et kort står nærmere end 16 px fra kanten (tabeller undtaget)`, r && !r.ud.length, r && r.ud.slice(0, 5).join(' | '));
      tjek(`M Indstillinger ved ${bredde} px, ${tekst}: ingen vandret rullebjælke`, r && !r.vandret);
    }
    /* Velkomsten de tre steder, i lys og mørk og ved 900 px med Stor tekst. */
    const rgb = c => c.match(/[\d.]+/g).map(Number).slice(0, 3), lys = c => { const [r, g, b] = rgb(c).map(v => { const x = v / 255; return x <= .03928 ? x / 12.92 : ((x + .055) / 1.055) ** 2.4; }); return .2126 * r + .7152 * g + .0722 * b; };
    const kontrastTal = (a, b) => { const [x, y] = [lys(a), lys(b)].sort((m, n) => n - m); return (x + .05) / (y + .05); };
    const alleFra = { domaener_hent: zoner.map(z => ({ ...z, til: false })), overblik: [], valg_hent: [] }, udenNoegle = { token_status: { sat: false } };
    const kontraster = [], hoejder = {};
    for (const [sted, hash, ekstra] of [['Om', '#indstillinger', {}], ['alle slået fra', '#overblik', alleFra], ['opsætningen', '#overblik', udenNoegle]]) {
      for (const [bredde, tekst, tema] of [[1200, 'normal', 'light'], [1200, 'normal', 'dark'], [900, 'large', 'dark']]) {
        const r = koer(bredde, tekst, tema, hash, ekstra), v = r && r.velkomst, nav = `${sted}, ${bredde} px, ${tekst}, ${tema}`;
        tjek(`W ${nav}: navnet er størst, og "for Cloudflare®" er mindre`, v && v.hm > v.cf && v.hm > v.wi && v.cf < v.hm * .75, JSON.stringify(v));
        (hoejder[`${bredde}-${tekst}`] ||= new Set()).add(v && v.h);
        tjek(`W ${nav}: margen og ingen vandret rullebjælke`, r && !r.ud.length && !r.vandret, r && r.ud.slice(0, 4).join(' | '));
        const k = v ? Math.min(kontrastTal(v.f24, v.bg), kontrastTal(v.hmFarve, v.bg)) : 0;
        kontraster.push(`${tema} ${k.toFixed(2)}`);
        tjek(`W ${nav}: "24" og navnet har kontrast mindst 4,5:1`, k >= 4.5, JSON.stringify(v));
      }
    }
    tjek('W overskriften har samme størrelse alle tre steder', Object.values(hoejder).every(x => x.size === 1), JSON.stringify(Object.values(hoejder).map(x => [...x])));
    /* Feltet uden rul, knapgrupperne og sidebjælken. */
    const felter = [];
    for (const [bredde, hoejde, tema, sprog] of [[1200, 860, 'light', 'da'], [1200, 860, 'dark', 'en'], [1400, 900, 'light', 'da'], [1000, 860, 'light', 'en']]) {
      const r = koer(bredde, 'normal', tema, '#overblik', udenNoegle, { sprog, hoejde }), f = r && r.felter;
      felter.push(f && `${bredde}×${hoejde} ${sprog}: ${Math.round(f.top)}–${Math.round(f.bund)}`);
      tjek(`F feltet til API Tokenet er synligt uden rul ved ${bredde} × ${hoejde}, ${tema}, ${sprog}`, f && f.synlig && f.vindue <= hoejde && f.rul === 0 && f.bund < f.vindue && f.top > 0, JSON.stringify(f));
    }
    const grupper = [];
    for (const [sprog, tekst, bredde] of [['da', 'normal', 1200], ['en', 'normal', 1200], ['da', 'large', 1200], ['en', 'large', 1200], ['en', 'large', 900], ['da', 'compact', 900]]) {
      const r = koer(bredde, tekst, 'light', '#indstillinger', {}, { sprog }), g = r ? r.segmenter : [], [a, b] = g, nav2 = `${sprog}, ${tekst}, ${bredde} px`;
      grupper.push(a && `${nav2}: ${Math.round(a.b)} og ${Math.round(b.b)} px`);
      tjek(`F Udseende og Tekststørrelse flugter: ${nav2}`, g.length === 2 && Math.abs(a.v - b.v) <= 1 && Math.abs(a.h - b.h) <= 1 && Math.abs(a.b - b.b) <= 1, JSON.stringify(g));
      tjek(`F knapperne er lige brede, og ingen etiket er afkortet: ${nav2}`, g.length === 2 && g.every(x => Math.max(...x.knapper) - Math.min(...x.knapper) <= 1 && !x.afkortet), JSON.stringify(g));
    }
    const landeSvar = land => ({ land_hent: { land, forslag: null } });
    const kort = koer(1200, 'normal', 'light', '#indstillinger', landeSvar('DK'), { sprog: 'en' }).nav;
    tjek('F sidebjælken: "Denmark now" på én linje ved Normal', kort.tekst === 'Denmark now' && kort.linjer && !kort.afskaaret, JSON.stringify(kort));
    const dk = koer(1200, 'normal', 'light', '#indstillinger', landeSvar('DK'), { sprog: 'da' }).nav;
    tjek('F sidebjælken: "Danmark lige nu" er uændret og på én linje', dk.tekst === 'Danmark lige nu' && dk.linjer && !dk.afskaaret, JSON.stringify(dk));
    const langt = koer(1200, 'normal', 'light', '#indstillinger', landeSvar('VC'), { sprog: 'en' }).nav;
    tjek('F sidebjælken: et langt landenavn afkortes med … og fuldt navn i tooltip', langt.tekst === 'St. Vincent & Grenadines now' && langt.title === langt.tekst && langt.linjer && langt.afskaaret && langt.ellipse === 'ellipsis', JSON.stringify(langt));
    const stor = koer(900, 'large', 'light', '#indstillinger', landeSvar('DK'), { sprog: 'da' }).nav;
    tjek('F sidebjælken: ingen ombrydning ved Stor tekst og 900 px', stor.linjer, JSON.stringify(stor));
    for (const [hash, ekstra, navn] of [['#login?samlet=1', { login: LOGIN_UD, login_stier: LOGIN_STIER, valg_hent: ['z1'] }, 'Login'], ['#robotter?samlet=1', { robotter: ROBOT_UD, valg_hent: ['z1'] }, 'Robotter']]) {
      const r = koer(900, 'large', 'light', hash, ekstra), g = r ? r.segmenter : [];
      tjek(`F ${navn} ved 900 px, Stor: knapperne er lige brede, intet afkortet, ingen vandret rullebjælke`, g.length >= 1 && g.every(x => Math.max(...x.knapper) - Math.min(...x.knapper) <= 1 && !x.afkortet) && !r.vandret, JSON.stringify(g));
    }
    const nyLang = { domaener_hent: [{ id: 'z1', navn: 'eksempel.wi24rd.com', til: true, ny: false }, { id: 'z2', navn: 'et-meget-langt-domaenenavn-som-skal-kunne-staa-her-uden-at-bryde-noget.wi24rd.com', til: false, ny: true }] };
    for (const [bredde, tekst] of [[1200, 'normal'], [900, 'large']]) {
      const r = koer(bredde, tekst, 'light', '#overblik', nyLang);
      tjek(`T3 linjen med et langt domænenavn ved ${bredde} px, ${tekst}: intet stikker ud`, r && r.nyLinje && !r.nyLinje.over.length && r.nyLinje.h <= r.nyLinje.plads + 1, JSON.stringify(r && r.nyLinje));
    }
    const dykLang = koer(900, 'large', 'light', `#dyk?zone=z1&navn=${zoner[1].navn}&regel=${encodeURIComponent(REACT)}&sti=${encodeURIComponent('/en/meget/lang/sti/til/noget/der/ligner/et/login/og/fylder/hele/raekken/og/lidt/til.php')}&dag=2026-11-03`, { dyk: dykSvar({ regel: REGEL_HOVED }) });
    tjek('DK ved 900 px, Stor: lang overskrift, knap og chips giver ingen vandret rullebjælke', dykLang && !dykLang.velkomst && !dykLang.vandret, JSON.stringify(dykLang).slice(0, 200));
    /* Hvorfor foldes til 3 linjer, og kalenderen holder sig inden for vinduet ved 900 px og Stor tekst. */
    const langHv = [{ ...OVERBLIK[0], moenster: { slags: 'maalrettet', dag: '2026-11-02', stier: 0, normalt: 0, scanner: false, ny: true, netvaerk: 'Mal AS', procent: 75, verden: true, tek: 'wordpress', foreloebig: true } }, { ...OVERBLIK[1], angreb_i_gaar_top: [] , angreb_i_gaar: 0 }];
    const hvr = koer(1200, 'normal', 'light', '#overblik', { overblik: langHv });
    tjek('Hvorfor: højst 3 linjer ved Normal, og "mere" står kun ved den lange', hvr && hvr.hv.length === 2 && hvr.hv.every(x => x.linjer <= 3) && hvr.hv[0].mere && !hvr.hv[1].mere, JSON.stringify(hvr && hvr.hv));
    const kal = koer(900, 'large', 'light', '#dyk?zone=z1&navn=eksempel.wi24rd.com&kal=1', { dyk: dykSvar({ periode: { fra: '2026-10-05', til: '2026-11-03', dage: 30, med: 30, data_fra: '2026-10-04', data_til: '2026-11-04', i_dag: '2026-11-05' } }) });
    tjek('Kalender ved 900 px, Stor: åben, inden for vinduet og uden vandret rullebjælke', kal && kal.kal && kal.kal.dage >= 28 && kal.kal.h <= kal.kal.vindue && kal.kal.v >= 0 && !kal.kal.vandret, JSON.stringify(kal && kal.kal));
    /* Betingede noter står kun, når de er slået til: Start ved login skal være fra, før målingen ser dem. */
    for (const [bredde, tekst] of [[1200, 'normal'], [900, 'large']]) {
      const r = koer(bredde, tekst, 'light', '#indstillinger', { autostart_hent: false });
      tjek(`Margen ved ${bredde} px, ${tekst}: Start ved login fra: noten og banneret står med margen`, r && !r.ud.length && !r.vandret, r && r.ud.slice(0, 5).join(' | '));
    }
    const langtUdfald = { ...LAND_UD, udfald: [{ ...LAND_UD.udfald[0], netvaerk: Array.from({ length: 12 }, (_, i) => `ET-MEGET-LANGT-NETVAERKSNAVN-FRA-RADAR-${i}`) }] };
    for (const [bredde, tekst] of [[1200, 'normal'], [900, 'large']]) {
      for (const [navn, hash, ekstra] of [['Land', '#land', { land_visning: langtUdfald }], ['Verden', '#land?fane=verden', { land_visning: LAND_UD }], ['dyk med regelkort', '#dyk?zone=z1&navn=eksempel.wi24rd.com&land=PL', { dyk: dykSvar() }], ['Overblik med server', '#overblik', { overblik: B2B_OVERBLIK }], ['trafik-dyk', '#dyk?zone=z1&navn=eksempel.wi24rd.com&dag=2026-11-02&trafik=1', { trafik_dag: B2C_TRAFIK }], ['Angreb med teknologi', '#vaernet?zone=z1&navn=eksempel.wi24rd.com', { teknologi_hent: B2C_TEK }], ['Robotter med datovælger', '#robotter?zone=z1&navn=eksempel.wi24rd.com', {}], ['Login med datovælger', '#login?zone=z1&navn=eksempel.wi24rd.com', {}]]) {
        const r = koer(bredde, tekst, 'light', hash, ekstra);
        if (navn === 'Overblik med server') tjek(`Land og site, linjen "Din server svarer ikke" ved ${bredde} px, ${tekst}: stikker ikke ud`, r && r.serverOver === false, JSON.stringify(r).slice(0, 160));
        tjek(`Land og site, ${navn} ved ${bredde} px, ${tekst}: ingen vandret rullebjælke`, r && !r.vandret, JSON.stringify(r).slice(0, 160));
      }
    }
    /* Overblik ved 900 px og Stor tekst: tabellen ruller i sin boks, og domænet bliver stående. */
    for (const [bredde, tekst, sprog] of [[900, 'large', 'da'], [900, 'large', 'en'], [900, 'normal', 'da'], [1200, 'normal', 'da']]) {
      const r = koer(bredde, tekst, 'light', '#overblik', { valg_hent: ['z1', 'z2', 'z3'] }, { sprog }), o = r && r.overblik, nav2 = `${bredde} px, ${tekst}, ${sprog}`;
      tjek(`Overblik ${nav2}: siden ruller ikke vandret, heller ikke hovedfladen`, r && o && !r.vandret && o.fladen, JSON.stringify(o));
      if (bredde === 900) tjek(`Overblik ${nav2}: tabellen ruller i boksen med fastlåst domæne`, o && o.indhold > o.boks && o.foerste === 'sticky' && o.fast, JSON.stringify(o));
    }
    /* Knapgruppen står ens i alle rækker, med og uden "Nulstil" og uanset navn og tal. */
    const TEK3 = { linjer: [{ tek: 'php', angreb30: 3, forsoeg30: 40, foerst: '2026-10-05', sidst: '2026-11-02', valg: 'ved_ikke', besked: false, forslag: false }, { tek: 'wordpress', angreb30: 1240, forsoeg30: 3, foerst: '2026-10-05', sidst: '2026-11-02', valg: 'bruger_ikke', besked: false, forslag: false }, { tek: 'react', angreb30: 0, forsoeg30: 0, foerst: null, sidst: null, valg: 'bruger', besked: true, forslag: false }], generiske: [] };
    /* Tabellen ved 1200 og 900 px, Stor tekst, lys og mørk. */
    const V3 = vaernUd(), alleTil = { domaener_hent: zoner.map(z => ({ ...z, til: true })) };
    const SIDE6 = { domaener: [{ zone: 'z1', navn: 'et-meget-langt-domaenenavn-som-skal-kunne-staa-her.wi24rd.com', valg: { php: ['bruger', true] }, antal: { php: 204 } }, { zone: 'z2', navn: 'b.wi24rd.com', valg: {}, antal: {} }], katalog: ['php', 'wordpress', 'react', 'drupal', 'joomla', 'laravel', 'nextjs', 'magento', 'fortinet', 'ivanti'].map(t => ({ tek: t, navn: t.toUpperCase(), gruppe: 'cms', angreb: 4, set: true, vis: true, ny: false })) };
    for (const [bredde, tekst, tema, sprog] of [[1200, 'normal', 'light', 'da'], [900, 'large', 'dark', 'en'], [900, 'large', 'light', 'da']]) {
      const r = koer(bredde, tekst, tema, '#teknologi', { ...alleTil, teknologi_side: SIDE6 }, { sprog }), t = r && r.tabel, nav2 = `${bredde} px, ${tekst}, ${tema}, ${sprog}`;
      tjek(`Teknologi ${nav2}: siden ruller ikke, boksen holder sig inde, og margen`, r && t && !r.vandret && !t.ude && !r.ud.length, JSON.stringify(t) + r?.ud?.slice(0, 3).join(' | '));
      tjek(`Teknologi ${nav2}: fastlåst domæne og overskrift, og klikfladen er mindst 24 px`, t && t.foerste === 'sticky' && t.top === 'sticky' && t.radio >= 24, JSON.stringify(t));
      if (bredde === 900) tjek(`Teknologi ${nav2}: tabellen ruller i boksen med domænet stående`, t && t.indhold > t.boks && t.rul > 0 && t.fast, JSON.stringify(t));
    }
    /* Vurdering, læk og året ved 900 px og Stor tekst. */
    const VURD9 = { dag: '2026-11-02', maal_ja: false, foreloebig: false, rekog_ja: false, maal: [{ id: 'netvaerk', ok: true, n: 100, m: 60, navn: 'ET-MEGET-LANGT-NETVAERKSNAVN-FRA-CLOUDFLARE-SOM-SKAL-KUNNE-BRYDES', domaener: [] }, { id: 'net_andre', ok: false, n: 9, m: 30, navn: 'X', domaener: [{ zone: 'z2', navn: 'et-meget-langt-domaenenavn-som-skal-kunne-staa-her.wi24rd.com', n: 9 }] }], rekog: [] };
    for (const [bredde, tekst] of [[1200, 'normal'], [900, 'large']]) {
      for (const [navn, hash, ekstra] of [['Angreb med Vurdering', '#vaernet?zone=z1&navn=eksempel.wi24rd.com', { vaern_cache: V3, vaern_oversigt: V3, koe_prioritet: null, teknologi_hent: TEK3, vurdering: VURD9 }], ['Login med læk', '#login?zone=z1&navn=eksempel.wi24rd.com', { login: { ...LOGIN_UD, laek: [{ dag: '2026-11-02', sti: '/en/meget/lang/sti/til/noget/der/ligner/et/login/og/fylder/hele/raekken/og/lidt/til.php', n: 3, naaet: 1 }], laek_andet: [], laek_dage: 30 }, login_stier: LOGIN_STIER }]]) {
        const r = koer(bredde, tekst, 'light', hash, ekstra);
        tjek(`Vurdering og læk ${navn} ved ${bredde} px, ${tekst}: ingen vandret rullebjælke`, r && !r.vandret, JSON.stringify(r).slice(0, 160));
      }
    }
    console.log('feltet: ' + felter.join(' · '));
    console.log('grupper: ' + grupper.join(' · '));
    console.log('kontrast 24: ' + [...new Set(kontraster)].join(' · '));
    fs.rmSync(mappe, { recursive: true, force: true });
  }
}

// ── "API Token" er ordet; intet "adgangsnøgle" eller "nøgle" i det, brugeren læser ──
{
  const tekster = k => [...udenKommentarer(k).matchAll(/'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)].map(m => m[1] ?? m[2]);
  const rust = f => { const k = fs.readFileSync(path.join(repo, 'src-tauri', 'src', f), 'utf8'); return tekster(k.split('#[cfg(test)]')[0]); };
  const html = ui('index.html').replace(/<[^>]*>/g, '\n').split('\n');
  const FORBUDT = /adgangsnøgl|access key|\bnøgle(n|r|rne)?\b|\bkey\b|\bkeys?…/i, TILLADT = /Arrow keys|adgangskoder og nøgler/;
  for (const [navn, liste] of [['sprog.js', tekster(ui('assets/sprog.js'))], ['index.html', html], ['notify.rs', rust('notify.rs')], ['programmenu.rs', rust('programmenu.rs')], ['agent.rs', rust('agent.rs')]]) {
    const fund = liste.filter(t => FORBUDT.test(t) && !TILLADT.test(t) && !/^[a-z_.-]+$/.test(t));
    tjek(`A ${navn}: ingen "adgangsnøgle", "nøgle", "access key" eller "key…" (${liste.length} tekster)`, liste.length > 5 && fund.length === 0, fund.join(' | '));
  }
  for (const f of ['README.md', 'README.da.md']) tjek(`A ${f}: "API Token" og ingen "adgangsnøgle" eller "access key"`, !/adgangsnøgl|access key/i.test(fs.readFileSync(path.join(repo, f), 'utf8')) && fs.readFileSync(path.join(repo, f), 'utf8').includes('API Token'));
  const s = side('#indstillinger', B2(), 'da', 'macos'), en = side('#indstillinger', B2(), 'en', 'macos');
  await sov(50);
  const t = (x, id) => x.d.getElementById(id).textContent;
  tjek('A Indstillinger → Forbindelse (da)', t(s, 'skift-noegle') === 'Skift API Token…' && t(s, 'glem') === 'Glem API Token…' && t(s, 'forb-note') === 'API Tokenet ligger i macOS Nøglering.' && s.d.querySelector('[data-t="forbNoegle"]').textContent === 'API Token til Cloudflare', t(s, 'glem'));
  tjek('A Indstillinger → Forbindelse (en)', t(en, 'skift-noegle') === 'Change API Token…' && t(en, 'glem') === 'Forget API Token…' && t(en, 'forb-note') === 'The API Token is in the macOS Keychain.' && en.d.querySelector('[data-t="forbNoegle"]').textContent === 'API Token for Cloudflare', t(en, 'glem'));
  const T = s.w.hmSprog.T, E = en.w.hmSprog.T;
  tjek('A hjælpen bruger Cloudflares navne: Zone › Analytics › Read og Zone › Zone › Read', [T.tokenTrin[4], E.tokenTrin[4]].every(x => x.includes('Zone › Analytics › Read') && x.includes('Zone › Zone › Read')) && T.ingenZoner.includes('Zone > Zone > Read'), T.hjaelp2);
}

// ── Domænevælgeren findes kun, når der er noget at vælge ──
{
  const DATA = x => ({ ...VIS, dyk: dykSvar(), ...x });
  for (const [hash, navn] of [['#vaernet?samlet=1', 'Angreb'], ['#robotter?samlet=1', 'Robotter'], ['#login?samlet=1', 'Login'], ['#dyk?samlet=1', 'dyk']]) {
    const s = side(hash, { ...DOMV(['z1', 'z2', 'z3']), ...DATA() });
    await sov(70);
    const b = s.d.getElementById('band-zone');
    tjek(`VV ${navn}: intet valgt, vælgeren er skjult (hidden, ikke kun deaktiveret), og der står ingen "Samlet (0)"`, b.hidden, b.outerHTML.slice(0, 120));
    await s.w.hmRamme.gemValgte(['z1']);
    await sov(70);
    tjek(`VV ${navn}: ét valgt, vælgeren kommer frem uden genindlæsning`, !b.hidden && s.d.getElementById('vg-zone').options[0].textContent === 'Samlet (1)', s.d.getElementById('vg-zone').options[0]?.textContent);
    await s.w.hmRamme.gemValgte([]);
    await sov(70);
    tjek(`VV ${navn}: alle fravalgt igen, vælgeren er væk igen`, b.hidden);
  }
  const s = side('#overblik', DOMV());
  await sov(60);
  tjek('VV Overblik har stadig ingen vælger', s.d.getElementById('band-zone').hidden);
  s.w.location.hash = '#vaernet?samlet=1';
  await sov(70);
  tjek('VV med valgte domæner er vælgeren der som før', !s.d.getElementById('band-zone').hidden && s.d.getElementById('vg-zone').options[0].textContent === 'Samlet (3)', s.d.getElementById('vg-zone').options[0]?.textContent);
}

// ── Egne login-stier i Indstillinger og under "Ret stier" (samme panel) ──
{
  const stub = () => {
    const st = { egne: ['/min-side'], fuld: false };
    return {
      st, login_stier: () => ({ stier: [{ sti: '/login', til: true, start: true }, { sti: '/admin/', til: false, start: true }, ...st.egne.map(sti => ({ sti, til: true, start: false }))], forslag: [] }),
      login_sti_saet: a => { if (!a.sti.startsWith('/') || /[\s?]/.test(a.sti)) throw { kode: 'ugyldigt_filter' }; if (st.fuld) throw { kode: 'for_mange_stier' }; if (!/^\/(login|admin\/)$/.test(a.sti) && !st.egne.includes(a.sti)) st.egne.push(a.sti); return null; },
      login_sti_fjern: a => { st.egne = st.egne.filter(x => x !== a.sti); return null; },
    };
  };
  const send = (s, felt, v) => { felt.value = v; felt.closest('form').dispatchEvent(new s.w.Event('submit', { bubbles: true, cancelable: true })); };
  const steder = [
    ['Indstillinger', '#indstillinger', async () => { }],
    ['Ret stier', '#login?zone=z1&navn=wi24rd.com', async s => { const r = s.d.querySelector('details.login-ret'); r.open = true; r.dispatchEvent(new s.w.Event('toggle')); await sov(60); }],
  ];
  for (const [sted, hash, aabn] of steder) {
    const x = stub(), s = side(hash, { ...B2(), ...VIS, ...x, ...x.st && {} });
    await sov(70);
    await aabn(s);
    const p = () => s.d.querySelector('.login-panel'), felt = () => p().querySelector('.login-egen input'), status = () => p().querySelector('.r6-status').textContent;
    tjek(`E ${sted}: feltet har overskrift, pladsholder og knap, under afkrydsningsfelterne`, felt()?.placeholder === '/min-login/' && p().querySelector('.login-egen label')?.textContent === 'Tilføj din egen login-sti' && p().querySelector('.login-egen button')?.textContent === 'Tilføj' && p().querySelector('.login-valg')?.nextElementSibling?.classList.contains('login-egen'), p()?.textContent.slice(0, 120));
    const fjernKnapper = () => [...p().querySelectorAll('.login-valg button')];
    tjek(`E ${sted}: kun egne stier har Fjern; startlistens kan kun slås fra`, fjernKnapper().length === 1 && fjernKnapper()[0].textContent === 'Fjern' && fjernKnapper()[0].getAttribute('aria-label') === 'Fjern /min-side' && fjernKnapper()[0].closest('li').textContent.includes('/min-side'));
    send(s, felt(), 'min login');
    await sov(40);
    tjek(`E ${sted}: en ugyldig sti giver klarsprog, og det indtastede bliver stående`, status() === 'Stien skal starte med / og må ikke have mellemrum eller ?' && felt().value === 'min login', status());
    send(s, felt(), '  /min-login/  ');
    await sov(40);
    tjek(`E ${sted}: en gyldig sti gemmes uden mellemrum om sig, står i listen med Fjern, feltet er tomt og har fokus`, s.kald.filter(k => k[0] === 'login_sti_saet').pop()[1].sti === '/min-login/' && s.kald.filter(k => k[0] === 'login_sti_saet').pop()[1].til === true && fjernKnapper().length === 2 && felt().value === '' && s.d.activeElement === felt(), status());
    x.st.fuld = true;
    send(s, felt(), '/nummer-21');
    await sov(40);
    tjek(`E ${sted}: den 21. sti giver klarsprog`, status() === 'Du kan højst have 20 egne login-stier pr. domæne. Fjern en, før du tilføjer en ny.', status());
    fjernKnapper().at(-1).click();
    await sov(40);
    tjek(`E ${sted}: Fjern kalder login_sti_fjern, og stien er væk`, s.kald.filter(k => k[0] === 'login_sti_fjern').pop()?.[1].sti === '/min-login/' && fjernKnapper().length === 1 && !p().querySelector('.login-valg').textContent.includes('/min-login/'), s.kald.filter(k => k[0] === 'login_sti_fjern').pop()?.[1].sti);
  }
  const x = stub(), en = side('#indstillinger', { ...B2(), ...VIS, ...x }, 'en');
  await sov(70);
  const p = en.d.querySelector('.login-panel');
  tjek('E engelsk: Add your own login path, Add, Remove', p.querySelector('.login-egen label').textContent === 'Add your own login path' && p.querySelector('.login-egen button').textContent === 'Add' && p.querySelector('.login-valg button').textContent === 'Remove' && p.querySelector('.login-egen input').placeholder === '/my-login/');
  tjek('E capabilities: login_sti_fjern er med i build.rs og som allow-login-sti-fjern', tilladt('login_sti_fjern') && cap.permissions.includes('allow-login-sti-fjern') && /"login_sti_fjern",/.test(fs.readFileSync(path.join(repo, 'src-tauri', 'build.rs'), 'utf8')));
  const notify = fs.readFileSync(path.join(repo, 'src-tauri', 'src', 'notify.rs'), 'utf8').split('#[cfg(test)]')[0];
  tjek('E notifikationer: notify.rs kender hverken login-stierne eller Login', !/login::|login_sti|STARTLISTE|\.sti\b/.test(notify));
  tjek('E ADGANG_QUERY har samme form: stierne er stadig kun en variabel', /clientRequestPath_in:\$stier/.test(fs.readFileSync(path.join(repo, 'src-tauri', 'src', 'adgang.rs'), 'utf8')));
}

// ── Ental og flertal for 0, 1 og 2 på begge sprog ──
{
  const da = side('', B2(), 'da'), en = side('', B2(), 'en');
  await sov(5);
  const S = { da: da.w.hmSprog, en: en.w.hmSprog }, F = sp => S[sp].fmt, T = sp => S[sp].T;
  tjek('Ø flertal: 1 og "1" giver ental, 0, 2, 11 og "1.000" giver flertal', [1, '1'].every(n => S.da.flertal(n, 'a', 'b') === 'a') && [0, 2, 11, '0', '2', '1.000'].every(n => S.da.flertal(n, 'a', 'b') === 'b'));
  tjek('Ø fmt: {n|a|b} vælger efter værdien, ordene må indeholde {n}, ukendte navne bliver stående', S.da.fmt('{n} {n|side|sider}', { n: '1' }) === '1 side' && S.da.fmt('{n|det|alle {n}}.', { n: '3' }) === 'alle 3.' && S.da.fmt('{x|a|b} {y}', {}) === '{x|a|b} {y}');
  const tabel = [
    ['hvAngreb', { s: 'x' }, 'a', ['0 angreb i går (x)', '1 angreb i går (x)', '2 angreb i går (x)'], ['0 attacks yesterday (x)', '1 attack yesterday (x)', '2 attacks yesterday (x)']],
    ['hvAngrebNat', { s: 'x' }, 'a', ['0 angreb i forgårs (x)', '1 angreb i forgårs (x)', '2 angreb i forgårs (x)'], ['0 attacks the day before yesterday (x)', '1 attack the day before yesterday (x)', '2 attacks the day before yesterday (x)']],
    ['hvAlleStoppet', {}, 'a', ['alle stoppet', 'stoppet', 'alle stoppet'], ['all stopped', 'stopped', 'all stopped']],
    ['gAlleStoppet', {}, 'n', ['Cloudflare stoppede alle 0.', 'Cloudflare stoppede det.', 'Cloudflare stoppede alle 2.'], ['Cloudflare stopped all 0.', 'Cloudflare stopped it.', 'Cloudflare stopped all 2.']],
    ['ekstraMangler', {}, 'n', ['De ekstra tal hentes i baggrunden: 0 dage mangler stadig.', 'De ekstra tal hentes i baggrunden: 1 dag mangler stadig.', 'De ekstra tal hentes i baggrunden: 2 dage mangler stadig.'], ['The extra figures are being fetched in the background: 0 days are still missing.', 'The extra figures are being fetched in the background: 1 day is still missing.', 'The extra figures are being fetched in the background: 2 days are still missing.']],
    ['kortFaaLande', {}, 'n', ['+ 0 lande med få angreb står ikke på kortet.', '+ 1 land med få angreb står ikke på kortet.', '+ 2 lande med få angreb står ikke på kortet.'], ['+ 0 countries with few attacks are not on the map.', '+ 1 country with few attacks is not on the map.', '+ 2 countries with few attacks are not on the map.']],
    ['kortFaaColos', {}, 'n', ['+ 0 datacentre med få angreb står ikke på kortet.', '+ 1 datacenter med få angreb står ikke på kortet.', '+ 2 datacentre med få angreb står ikke på kortet.'], ['+ 0 data centres with few attacks are not on the map.', '+ 1 data centre with few attacks is not on the map.', '+ 2 data centres with few attacks are not on the map.']],
    ['faelderSat', {}, 'n', ['0 fælde-links sat ind', '1 fælde-link sat ind', '2 fælde-links sat ind'], ['0 trap links inserted', '1 trap link inserted', '2 trap links inserted']],
    ['kortBuer', { m: '9' }, 'n', ['Buerne er de 0 største af 9.', 'Buen er den største af 9.', 'Buerne er de 2 største af 9.'], ['The arcs are the 0 largest of 9.', 'The arc is the largest of 9.', 'The arcs are the 2 largest of 9.']],
  ];
  for (const [k, v, navn, d, e] of tabel) for (const [sp, forvent] of [['da', d], ['en', e]]) {
    const ud = ['0', '1', '2'].map(n => F(sp)(T(sp)[k], { ...v, [navn]: n }));
    tjek(`Ø ${k} (${sp}): 0, 1 og 2`, ud.join('|') === forvent.join('|'), ud.join(' | '));
  }
  for (const [sp, a, b, c] of [['da', '0 domæner', '1 domæne', '2 domæner'], ['en', '0 domains', '1 domain', '2 domains']]) tjek(`Ø domæner (${sp}): 0, 1 og 2`, T(sp).domaener(0) === a && T(sp).domaener(1) === b && T(sp).domaener(2) === c, T(sp).domaener(1));
  /* Alle tekster med en værdi: sat til 1 må ingen stå i flertal, sat til 0 eller 2 ingen i ental. */
  const strenge = [...new Set(fs.readdirSync(path.join(repo, 'ui', 'assets')).filter(f => f.endsWith('.js')).flatMap(f => [...udenKommentarer(ui('assets/' + f)).matchAll(/'((?:[^'\\\n]|\\.)*\{\w+(?:\|(?:[^'\\\n]|\\.)*)?\}(?:[^'\\\n]|\\.)*)'/g)].map(m => m[1]).filter(s => !s.includes('`'))))];
  const fyld = (s, n) => S.da.fmt(s, Object.fromEntries([...s.matchAll(/\{(\w+)[|}]/g)].map(m => [m[1], n])));
  const funktioner = sp => Object.values(T(sp)).filter(f => typeof f === 'function' && f.length === 1);
  const ENTAL_EN = /\b1 (?!is\b|was\b|has\b|as\b|findes\b|hentes\b)[a-z-]+s(?![\wæøå])/, ENTAL_DA = /\b1 (domæner|dage|sider|robotter|lande|datacentre|forespørgsler|stier|regler|fælde-links|timer|minutter|nye)\b/, FLERTAL = /\b[02] (attack|domain|day|request|country|page|robot|domæne|dag|side|forespørgsel|datacenter)\b/;
  const med1 = [...strenge.map(s => fyld(s, '1')), ...['da', 'en'].flatMap(sp => funktioner(sp).flatMap(f => [f(1), f('1')]))].map(String), med2 = [...strenge.flatMap(s => [fyld(s, '0'), fyld(s, '2')]), ...['da', 'en'].flatMap(sp => funktioner(sp).flatMap(f => [f(0), f(2)]))].map(String);
  tjek(`Ø alle ${strenge.length} skabeloner og ${funktioner('da').length + funktioner('en').length} funktioner med tal: ingen "1 attacks" og ingen "1 domæner"`, strenge.length > 100 && !med1.some(t => ENTAL_EN.test(t) || ENTAL_DA.test(t)), med1.filter(t => ENTAL_EN.test(t) || ENTAL_DA.test(t)).slice(0, 6).join(' | '));
  tjek('Ø ingen "0 attack" eller "2 domæne"', !med2.some(t => FLERTAL.test(t)), med2.filter(t => FLERTAL.test(t)).slice(0, 6).join(' | '));
  tjek('Ø intet {x|a|b} er tilbage i en udfyldt tekst', ![...med1, ...med2].some(t => /\{\w+\|/.test(t)), [...med1, ...med2].filter(t => /\{\w+\|/.test(t)).slice(0, 4).join(' | '));
  const en1 = raekke('z1', 'web.wi24rd.com', 'gul', { angreb_i_gaar: 1, angreb_snit: 2.9, angreb_fordeling: { blokeret: 1, udfordret: 0, labyrint: 0, andet: 0, faelder: 0 } });
  const s = side('', { ...MED_SIDEN, overblik: [en1], siden_sidst: { vis: false, nye: [], raekker: [en1] } }, 'en');
  await sov(40);
  tjek('Ø Hvorfor siger "1 attack yesterday (average 2.9) – stopped."', hv(s.d.querySelector('#overblik-raekker tr')).textContent.startsWith('1 attack yesterday (average 2.9) – stopped.'), hv(s.d.querySelector('#overblik-raekker tr')).textContent);
}

// ── Gruppen med kommende visninger er væk fra sidebjælken ──
{
  const ORD = new RegExp([['Kommer', 'snart'].join(' '), ['Coming', 'soon'].join(' '), ['Hast', 'ighed'].join(''), `\\b${['Spe', 'ed'].join('')}\\b`].join('|'), 'i');
  const filer = ['ui/index.html', ...fs.readdirSync(path.join(repo, 'ui', 'assets')).filter(f => /\.(js|css)$/.test(f) && f !== 'tokens.css').map(f => 'ui/assets/' + f), 'src-tauri/src/programmenu.rs', 'src-tauri/src/agent.rs', 'src-tauri/src/lib.rs', 'README.md', 'README.da.md', 'tests/ui/ui-roeg.mjs'];
  const fund = filer.filter(f => ORD.test(fs.readFileSync(path.join(repo, f), 'utf8')));
  tjek(`KS ingen rest i UI, menu, README og tests (${filer.length} filer)`, filer.length > 15 && fund.length === 0, fund.join());
  const s = side('', B2());
  await sov(30);
  tjek('KS sidebjælken har de seks visninger og Indstillinger, og intet punkt er slået fra', !s.d.getElementById('kommer') && s.d.querySelectorAll('#side a[data-visning]').length === 7 && !s.d.querySelector('#side details') && !/\.kommer|kommerSnart/.test(ui('assets/app.css') + ui('assets/sprog.js')), s.d.querySelectorAll('#side a[data-visning]').length);
}

// ── Dyk kan ikke forveksles med Angreb ──
{
  const Z = 'zone=z1&navn=wi24rd.com', Q = x => '#dyk?' + new URLSearchParams(x).toString();
  const FIL = { ...DYK(), dyk: () => dykSvar(), ...VIS };
  for (const [navn, x, da, en] of [
    ['land', { land: 'FR' }, 'Angreb på wi24rd.com fra Frankrig', 'Attacks on wi24rd.com from France'],
    ['regel', { regel: REACT }, 'Angreb på wi24rd.com med reglen Et kendt hul i React', 'Attacks on wi24rd.com with the rule A known hole in React'],
    ['sti', { sti: '/wp-login.php' }, 'Angreb på wi24rd.com på stien /wp-login.php', 'Attacks on wi24rd.com on the path /wp-login.php'],
    ['colo', { colo: 'AMS' }, 'Angreb på wi24rd.com stoppet i Amsterdam (AMS)', 'Attacks on wi24rd.com stopped in Amsterdam (AMS)'],
    ['netværk', { netvaerk: 'Google LLC' }, 'Angreb på wi24rd.com fra netværket Google LLC', 'Attacks on wi24rd.com from the network Google LLC'],
    ['metode', { metode: 'POST' }, 'Angreb på wi24rd.com med metoden POST', 'Attacks on wi24rd.com with the method POST'],
    ['robot', { robot: 'GPTBot' }, 'Besøg på wi24rd.com af robotten GPTBot', 'Visits on wi24rd.com by the robot GPTBot'],
    ['dag', { dag: '2026-11-03' }, 'Angreb på wi24rd.com den 3. november', 'Attacks on wi24rd.com on 3 November'],
    ['intet filter', {}, 'Angreb på wi24rd.com', 'Attacks on wi24rd.com'],
  ]) {
    const a = side(Q({ zone: 'z1', navn: 'wi24rd.com', ...x }), FIL, 'da'), b = side(Q({ zone: 'z1', navn: 'wi24rd.com', ...x }), FIL, 'en');
    await sov(70);
    tjek(`DK overskriften siger filtret, ${navn} (da, en)`, a.d.getElementById('dyk-h').textContent === da && b.d.getElementById('dyk-h').textContent === en, `${a.d.getElementById('dyk-h').textContent} | ${b.d.getElementById('dyk-h').textContent}`);
  }
  const s = side('#dyk?' + Z + '&land=FR&sti=%2Flogin&dag=2026-11-03&slags=angreb', { ...FIL, valg_hent: ['z1', 'z2'] }), { d, w } = s;
  await sov(80);
  const knap = d.querySelector('#dyk-tilbage button'), h = d.getElementById('dyk-h'), chips = () => [...d.querySelectorAll('#dyk-filtre button')], foer = (a, b) => Boolean(a.compareDocumentPosition(b) & w.Node.DOCUMENT_POSITION_FOLLOWING);
  tjek('DK knappen: øverst, en rigtig sekundærknap med navn og fast mål', d.querySelector('#visning-dyk').firstElementChild.id === 'dyk-tilbage' && knap.className === 'knap' && knap.tagName === 'BUTTON' && knap.textContent === '← Hele oversigten over angreb på wi24rd.com' && knap.getAttribute('aria-label') === 'Hele oversigten over angreb på wi24rd.com' && knap.dataset.gaa === 'vaernet?zone=z1&navn=wi24rd.com', knap?.outerHTML);
  tjek('DK flere filtre: det første i overskriften, alle som chips, ingen brødkrumme', h.textContent === 'Angreb på wi24rd.com fra Frankrig' && chips().map(c => c.textContent).join() === 'Frankrig×,/login×,3. november×' && !d.getElementById('dyk-krummer') && !d.querySelector('.dyk-fjern'), chips().map(c => c.textContent).join());
  tjek('DK chips: navn til skærmlæsere og en liste med navn', chips()[2].getAttribute('aria-label') === 'Fjern filtret 3. november' && d.getElementById('dyk-filtre').tagName === 'NAV' && d.getElementById('dyk-filtre').getAttribute('aria-label') === 'Filtre' && d.querySelectorAll('#dyk-filtre li').length === 3);
  const q1 = new URLSearchParams(chips()[0].dataset.dyk), q3 = new URLSearchParams(chips()[2].dataset.dyk);
  tjek('DK chips: × fjerner kun sin dimension og bevarer resten, perioden og slags', !q1.has('land') && q1.get('sti') === '/login' && q1.get('dag') === '2026-11-03' && q1.get('slags') === 'angreb' && q1.get('zone') === 'z1', q1.toString());
  tjek('DK chips: × på dagen fjerner dagen og bevarer filtrene', !q3.has('dag') && q3.get('land') === 'FR' && q3.get('sti') === '/login' && q3.get('slags') === 'angreb', q3.toString());
  const maerke = d.getElementById('dyk-udsnit');
  tjek('DK mærket: "Udsnit" står ved overskriften, er ikke klikbart og bruger eksisterende tokens', maerke?.textContent === 'Udsnit' && maerke?.previousElementSibling === h && maerke?.tagName === 'SPAN' && !maerke?.closest('button') && /\.dyk-udsnit \{[^}]*background: var\(--color-accent-soft\); color: var\(--color-accent-text\)/.test(ui('assets/vagt.css')) && !/\.dyk-udsnit:hover/.test(ui('assets/vagt.css')));
  tjek('DK chippen er en knap med egen hover og markør', chips().every(c => c.tagName === 'BUTTON') && /\.dyk-filter:hover \{[^}]*border-color: var\(--color-accent\)/.test(ui('assets/vagt.css')) && /\.dyk-filter \{[^}]*cursor: pointer/.test(ui('assets/vagt.css')));
  tjek('DK Tab-rækkefølge: fokus på knappen, så chips og vælgere', d.activeElement === knap && foer(knap, h) && foer(h, chips()[0]) && foer(chips().at(-1), d.querySelector('#dyk-periode input')) && h.getAttribute('tabindex') === '-1', d.activeElement && d.activeElement.outerHTML.slice(0, 80));
  tjek('DK sidebjælken bliver ved Angreb', d.querySelector('#side a[aria-current]').getAttribute('data-visning') === 'vaernet');
  chips()[1].click(); await sov(60);
  tjek('DK × ét filter: sidste forespørgsel har kun det andet filter', JSON.stringify(sidsteDyk(s).filtre) === JSON.stringify([{ dim: 'land', vaerdi: 'FR' }]) && JSON.stringify(sidsteDyk(s).periode) === '{"dag":"2026-11-03"}' && sidsteDyk(s).slags === 'angreb', JSON.stringify(sidsteDyk(s)));
  const antal = w.history.length;
  d.querySelector('#dyk-tilbage button').click(); await sov(60);
  tjek('DK knappen fører frem til oversigten for samme domæne, ikke tilbage i historikken', w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com' && w.history.length === antal + 1 && synlige(d).join() === 'vaernet', `${w.location.hash} ${w.history.length}/${antal}`);
  const en = side('#dyk?' + Z + '&dag=2026-11-03', FIL, 'en');
  await sov(70);
  const c = en.d.querySelector('#dyk-filtre button');
  tjek('DK engelsk: knap, mærke og chip', en.d.querySelector('#dyk-tilbage button').textContent === '← All attacks on wi24rd.com' && en.d.getElementById('dyk-udsnit')?.textContent === 'Filtered view' && c.getAttribute('aria-label') === 'Remove the filter 3 November', c?.getAttribute('aria-label'));
  tjek('DK den sidste chip fører til oversigten', c.dataset.gaa === 'vaernet?zone=z1&navn=wi24rd.com' && !c.dataset.dyk, c.outerHTML.slice(0, 160));
  c.click(); await sov(60);
  tjek('DK × på den sidste: man står i oversigten', en.w.location.hash === '#vaernet?zone=z1&navn=wi24rd.com' && synlige(en.d).join() === 'vaernet', en.w.location.hash);
  const ingen = side('#dyk?' + Z, FIL);
  await sov(70);
  tjek('DK uden filter: ingen chips, men knap og mærke', ingen.d.getElementById('dyk-filtre').hidden && ingen.d.querySelector('#dyk-tilbage button') && ingen.d.getElementById('dyk-udsnit')?.hidden === false);
  const per = side('#dyk?' + Z + '&land=FR&periode=7', FIL);
  await sov(70);
  tjek('DK perioden og slags står ikke i overskriften', per.d.getElementById('dyk-h').textContent === 'Angreb på wi24rd.com fra Frankrig', per.d.getElementById('dyk-h').textContent);
  for (const [zoner, sp, tekst, h1] of [[['z1', 'z2'], 'da', '← Hele oversigten over angreb på dine domæner', 'Angreb på dine domæner fra Frankrig'], [['z1', 'z2'], 'en', '← All attacks on your domains', 'Attacks on your domains from France'], [['z1'], 'da', '← Hele oversigten over angreb på dit domæne', 'Angreb på dit domæne fra Frankrig'], [['z1'], 'en', '← All attacks on your domain', 'Attacks on your domain from France']]) {
    const x = side('#dyk?samlet=1&land=FR', { ...FIL, valg_hent: zoner }, sp);
    await sov(70);
    const b = x.d.querySelector('#dyk-tilbage button');
    tjek(`DK samlet, ${zoner.length} ${sp}: knap og overskrift, og målet er det samlede`, b.textContent === tekst && b.dataset.gaa === 'vaernet?samlet=1' && x.d.getElementById('dyk-h').textContent === h1, `${b.textContent} | ${x.d.getElementById('dyk-h').textContent}`);
  }
  const rob = side('#dyk?' + dykQ({ robot: 'alle' }), DYK({ dyk: () => ROBOT_DYK }));
  await sov(80);
  const rb = rob.d.querySelector('#dyk-tilbage button');
  tjek('DK robot: knappen går til Robotter, og overskriften er ikke "Angreb"', rb.textContent === '← Hele oversigten over robotter på wi24rd.com' && rb.dataset.gaa === 'robotter?zone=z1&navn=wi24rd.com' && rob.d.getElementById('dyk-h').textContent === 'Kendte robotter på wi24rd.com', rb.outerHTML.slice(0, 160));
}

// ── ──
{
  const MAAL = { slags: 'maalrettet', dag: '2026-11-02', stier: 0, normalt: 0, scanner: false, ny: true, netvaerk: 'Mal AS', procent: 75, verden: true, tek: null, foreloebig: true };
  const rk = (id, navn, x) => ({ ...OVERBLIK[1], id, navn, status: 'groen', ...x });

  /* Del A1: under 3 valgte domæner */
  const lidt = [rk('z1', 'wi24rd.com', { kan_sammenligne: false })], nok = [rk('z1', 'wi24rd.com', { kan_sammenligne: true })];
  const vn = over => side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ overblik: over, vaern_cache: vaernUd(), vaern_oversigt: vaernUd() }));
  const v1 = vn(lidt), v2 = vn(nok), sam = side('#vaernet?samlet=1', DYK({ overblik: lidt, vaern_samlet_cache: { domaener: 1, med_tal: 1, valgte: [], vaern: vaernUd() } }));
  await sov(220);
  tjek('Få domæner: under 3 valgte siger Angreb det én gang', v1.d.querySelectorAll('.moenster-note').length === 1 && v1.d.querySelector('.moenster-note').textContent === 'Huginn kan kun sige »Ligner målrettet«, når du har mindst 3 domæner valgt at sammenligne med.', v1.d.getElementById('vagt').textContent.slice(0, 160));
  tjek('Få domæner: også i det samlede Angreb', sam.d.querySelectorAll('.moenster-note').length === 1);
  tjek('Få domæner: med 3 valgte står forklaringen ikke', !v2.d.querySelector('.moenster-note'));
  const og = side('', DYK({ overblik: lidt }));
  await sov(120);
  og.w.hmRamme.guide(null); await sov(5);
  tjek('Få domæner: guiden siger det én gang', og.d.querySelectorAll('#hjaelp-guide .g-note').length === 1 && og.d.querySelector('#hjaelp-guide .g-note').textContent.includes('mindst 3 domæner'));

  /* Del A2: foreløbig */
  const fo = sp => side('', DYK({ overblik: [rk('z1', 'wi24rd.com', { moenster: MAAL })] }), sp), fd = fo('da'), fe = fo('en');
  await sov(120);
  tjek('Foreløbigt mærke: mærket står med det samme, men foreløbigt', fd.d.querySelector('.moenster-maerke').textContent === 'Ligner målrettet · foreløbig' && fe.d.querySelector('.moenster-maerke').textContent === 'Looks targeted · provisional', fd.d.querySelector('.moenster-maerke').textContent);
  tjek('Foreløbigt mærke: teksten siger, hvornår det afgøres', fd.w.hmSprog.moensterTekst(MAAL).endsWith('Foreløbig — afgøres, når i morgen er hentet.') && fe.w.hmSprog.moensterTekst(MAAL).endsWith('Provisional — decided when tomorrow is fetched.'));
  tjek('Foreløbigt mærke: afgjort står uden "foreløbig"', !fd.w.hmSprog.moensterTekst({ ...MAAL, foreloebig: false }).includes('Foreløbig'));

  /* Den korte tabel er erstattet af en linje og knappen til #teknologi. */
  const OVER = { zoner: { z1: 3, z2: 1, z3: 0 }, valg: true };
  let over = JSON.parse(JSON.stringify(OVER));
  const zoner = [{ id: 'z1', navn: 'wi24rd.com', til: true, ny: false }, { id: 'z2', navn: 'xyz.wi24rd.com', til: true, ny: false }, { id: 'z3', navn: 'tom.wi24rd.com', til: true, ny: false }, { id: 'z4', navn: 'fravalgt.wi24rd.com', til: false, ny: false }];
  const ind = side('#indstillinger', DYK({ domaener_hent: zoner, autostart_hent: true, data_status: { bytes: 1, foerste: null, sidste: null, domaener: 0, nulstil: { sikkerhed: null, trafik: null } }, domaener_saet: null, teknologi_oversigt: () => over, teknologi_nulstil: () => { over = { zoner: { z1: 0, z2: 0, z3: 0 }, valg: false }; return null; } }));
  await sov(140);
  tjek('Teknologi: en kort linje i stedet for tabellen', ind.d.getElementById('tek-linje').textContent === 'wi24rd.com: 3 valg · xyz.wi24rd.com: 1 valg · 1 domæne uden valg' && !ind.d.getElementById('tek-tabel'), ind.d.getElementById('tek-linje').textContent);
  tjek('Teknologi: knappen åbner siden', ind.d.getElementById('tek-aabn').dataset.gaa === 'teknologi' && ind.d.getElementById('tek-aabn').textContent === 'Teknologi pr. domæne →', ind.d.getElementById('tek-aabn').outerHTML);
  tjek('Domænelisten: den lange linje er væk fra domænelisten', !ind.d.querySelector('#domaener-liste .tek-oversigt, #domaener-liste .tek-nulstil'));
  tjek('Domænelisten: linjen står efter "API Tokenet ser N domæner"', Boolean(ind.d.getElementById('domaener-ser').compareDocumentPosition(ind.d.getElementById('tek-blok')) & 4));
  const nulKald = () => ind.kald.filter(k => k[0] === 'teknologi_nulstil');
  ind.d.getElementById('tek-nulstil-alle').click(); await sov(10);
  tjek('Nulstil teknologi: Nulstil for alle spørger først', ind.d.getElementById('bekraeft-dialog').hasAttribute('open') && !nulKald().length);
  ind.d.getElementById('bekraeft-nej').click(); await sov(20);
  tjek('Nulstil teknologi: Annullér nulstiller intet', !nulKald().length && !ind.d.getElementById('bekraeft-dialog').hasAttribute('open'));
  ind.d.getElementById('tek-nulstil-alle').click(); await sov(10); ind.d.getElementById('bekraeft-ja').click(); await sov(60);
  tjek('Nulstil teknologi: alle domæner, og knappen går, når der ikke er flere valg', JSON.stringify(nulKald().at(-1)) === '["teknologi_nulstil",{"zone":null}]' && ind.d.getElementById('tek-raekke').hidden && !ind.d.getElementById('tek-blok').hidden, JSON.stringify(nulKald().at(-1)));
  const kontakt = ind.d.querySelector('#domaener-liste input');
  const oversigter = () => ind.kald.filter(k => k[0] === 'teknologi_oversigt').length, foerKontakt = oversigter();
  over = OVER; kontakt.focus(); kontakt.checked = false; kontakt.dispatchEvent(new ind.w.Event('change', { bubbles: true })); await sov(80);
  tjek('Nulstil teknologi: linjen følger valget af domæner', oversigter() > foerKontakt, oversigter() - foerKontakt);

  /* Boksen "Vurdering" */
  const VURD = { dag: '2026-11-05', maal_ja: false, foreloebig: false, rekog_ja: false, maal: [
    { id: 'stigning', ok: true, n: 1800, m: 1.3, navn: null, domaener: [] },
    { id: 'netvaerk', ok: true, n: 100, m: 60, navn: 'Mal Hosting Ltd', domaener: [] },
    { id: 'net_andre', ok: false, n: 900, m: 1800, navn: 'Mal Hosting Ltd', domaener: [{ zone: 'z2', navn: 'web.wi24rd.com', n: 900 }] },
    { id: 'gule', ok: true, n: 1, m: 1, navn: null, domaener: [{ zone: 'z2', navn: 'web.wi24rd.com', n: 0 }] },
    { id: 'valgte', ok: true, n: 4, m: 3, navn: null, domaener: [] },
    { id: 'radar', ok: null, n: 0, m: 1.3, navn: null, domaener: [] },
    { id: 'efter', ok: false, n: 0, m: 0, navn: null, domaener: [{ zone: 'z1', navn: 'nord.wi24rd.com', n: 0 }] },
  ], rekog: [{ id: 'stier', ok: false, n: 12, m: 100, navn: null, domaener: [] }, { id: 'faktor', ok: true, n: 12, m: 6, navn: null, domaener: [] }] };
  const vk = side('#vaernet?zone=z1&navn=nord.wi24rd.com', DYK({ vaern_cache: vaernUd(), vaern_oversigt: vaernUd(), koe_prioritet: null, teknologi_hent: { linjer: [], generiske: [] }, vurdering: VURD }));
  const vkd = side('#dyk?' + dykQ({ navn: 'nord.wi24rd.com', dag: '2026-11-05' }), DYK({ vurdering: VURD })), vke = side('#dyk?' + dykQ({ dag: '2026-11-05' }), DYK({ vurdering: { ...VURD, maal_ja: true, foreloebig: true } }), 'en');
  const vks = side('#vaernet?samlet=1', DYK({ vaern_samlet_cache: { vaern: vaernUd(), valgte: [], domaener: 1, med_tal: 1 }, vurdering: VURD })), vkp = side('#dyk?' + dykQ({ periode: '7' }), DYK({ vurdering: VURD }));
  await sov(260);
  const vb = vk.d.querySelector('.vurdering'), vl = id => vb?.querySelector(`li[data-id="${id}"]`), tegnet = id => vl(id)?.querySelector('.vurd-tegn').textContent;
  tjek('Vurdering: i Angreb for i går, foldet sammen og med spørgsmålet', vb && !vb.open && vb.querySelector('summary').textContent === 'Hvorfor står der (ikke) »Ligner målrettet« eller »Ligner rekognoscering«?' && vk.kald.some(k => k[0] === 'vurdering' && k[1].zone === 'z1'), vb?.outerHTML.slice(0, 120));
  tjek('Vurdering: ✓ og ✗ følger Rust, og – ved det, der ikke kan vurderes', tegnet('stigning') === '✓' && tegnet('net_andre') === '✗' && tegnet('efter') === '✗' && tegnet('radar') === '–' && tegnet('stier') === '✗' && tegnet('faktor') === '✓', ['stigning', 'net_andre', 'radar'].map(tegnet).join());
  tjek('Vurdering: tegnene siges også i ord', vl('net_andre')?.querySelector('.sr-only').textContent === 'Ikke opfyldt: ' && vl('stigning')?.querySelector('.sr-only').textContent === 'Opfyldt: ' && vl('radar')?.querySelector('.sr-only').textContent === 'Tæller ikke med: ');
  tjek('Vurdering: tallene i klarsprog', vl('stigning')?.textContent.includes('Stigning: 1.800 angreb mod snit 1,3') && vl('netvaerk')?.textContent.includes('Ét netværk står for mindst 60 %: Mal Hosting Ltd 100 %') && vl('net_andre')?.textContent.includes('900 på andre valgte domæner ±1 dag mod 1.800 her') && vl('radar')?.textContent.includes('Radar har ikke tal for dagen'), vb?.textContent);
  tjek('Vurdering: overskrifterne siger udfaldet', [...(vb?.querySelectorAll('h4') || [])].map(h => h.textContent).join() === 'Ligner målrettet: står ikke,Ligner rekognoscering: står ikke', vb?.textContent.slice(0, 200));
  const q1 = new URLSearchParams(vl('netvaerk')?.querySelector('[data-dyk]')?.dataset.dyk), q2 = new URLSearchParams(vl('net_andre')?.querySelector('[data-dyk]')?.dataset.dyk), q3 = new URLSearchParams(vl('stigning')?.querySelector('[data-dyk]')?.dataset.dyk);
  tjek('Vurdering: tal med data bag kan åbnes', q1.get('netvaerk') === 'Mal Hosting Ltd' && q1.get('zone') === 'z1' && q2.get('zone') === 'z2' && q2.get('netvaerk') === 'Mal Hosting Ltd' && q3.get('slags') === 'angreb' && vl('valgte')?.querySelector('[data-gaa]')?.dataset.gaa === 'indstillinger?afsnit=domaener', [...q1, ...q2].join());
  tjek('Vurdering: det uden data bag kan ikke klikkes', !vl('radar')?.querySelector('button') && !vb?.querySelector('li[data-id="faktor"] button'));
  const db = vkd.d.querySelector('.vurdering');
  tjek('Vurdering: i dyk for én dag, med dagen', db && vkd.kald.some(k => k[0] === 'vurdering' && k[1].dag === '2026-11-05') && db.textContent.includes('for 5. november'), db?.textContent.slice(0, 200));
  tjek('Vurdering: engelsk, og foreløbig siges', vke.d.querySelector('.vurdering summary')?.textContent.startsWith('Why does it (not) say') && vke.d.querySelector('.vurdering h4')?.textContent === 'Looks targeted: shown as preliminary', vke.d.querySelector('.vurdering')?.textContent.slice(0, 200));
  tjek('Vurdering: ikke i det samlede Angreb og ikke for en periode', !vks.d.querySelector('.vurdering') && !vkp.d.querySelector('.vurdering'));

  /* Trafikken */
  const tOv = (t, sprog) => side('#overblik', DYK({ overblik: [{ ...OVERBLIK[1], id: 't1', navn: 't.wi24rd.com', status: 'gul', angreb_i_gaar: 0, angreb_i_gaar_top: [], trafik: t }] }), sprog);
  const t1 = tOv({ dag: '2026-11-05', n: 3200, median: 13, stoppet: 1800 }), t2 = tOv({ dag: '2026-11-05', n: 3000, median: 13, stoppet: 1000 }), t3 = tOv({ dag: '2026-11-05', n: 3200, median: 13, stoppet: 1800 }, 'en'), t4 = tOv({ dag: '2026-11-05', n: 2, median: 0, stoppet: 1 });
  await sov(120);
  const hvT3 = x => x.d.querySelector('tr[data-zone="t1"] .hv-tekst')?.textContent || '';
  tjek('Trafiktekst: ≥ 50 %: det meste var angreb, ordret som notify', hvT3(t1) === 'Ingen angreb i går. Usædvanlig meget trafik: 3.200 forespørgsler (normalt 13). Det meste af trafikspringet var angreb, som Cloudflare stoppede. Det var 1.800. De øvrige 1.400 slap igennem og tæller ikke som angreb. Se, hvor trafikken kom fra.', hvT3(t1));
  tjek('Trafiktekst: under 50 %: stoppet som angreb, de øvrige slap igennem', hvT3(t2).includes('Cloudflare stoppede 1.000 som angreb. De øvrige 2.000 slap igennem og tæller ikke som angreb — det kan være et forsøg'), hvT3(t2));
  tjek('Trafiktekst: engelsk ordret som notify', hvT3(t3) === 'No attacks yesterday. Unusually high traffic: 3,200 requests (normally 13). Most of the traffic spike was attacks that Cloudflare stopped. That was 1,800. The other 1,400 got through and do not count as attacks. See where the traffic came from.', hvT3(t3));
  tjek('Trafiktekst: ental', hvT3(t4).includes('Den sidste 1 slap igennem'), hvT3(t4));
  tjek('Trafiktekst: "så de tæller ikke som angreb" er væk', !/så de tæller ikke som angreb|so they do not count/.test(ui('assets/sprog.js') + fs.readFileSync(path.join(repo, 'src-tauri/src/notify.rs'), 'utf8').replace(/assert![^\n]*\n/g, '')));
  const tA = side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: { ...vaernUd(), trafik: [{ dag: '2026-11-05', n: 3200, median: 13, stoppet: 1800 }] }, vaern_oversigt: { ...vaernUd(), trafik: [{ dag: '2026-11-05', n: 3200, median: 13, stoppet: 1800 }] }, koe_prioritet: null }));
  const tD = side('#dyk?' + dykQ({ fra: '2026-11-05', til: '2026-11-05' }), DYK({ dyk: dykSvar({ trafik: { i_alt: 3200, dage: [{ dag: '2026-11-05', n: 3200, median: 13, stoppet: 1800 }] } }) }));
  await sov(220);
  tjek('Trafiktekst: Angreb-linjen har også tallet for det, der slap igennem', tA.d.querySelector('.vg-trafik .trafik-maerke')?.textContent === 'Usædvanlig meget trafik den 5. november: 3.200 forespørgsler. Det meste var angreb, som Cloudflare stoppede: 1.800, og 1.400 slap igennem', tA.d.querySelector('.vg-trafik')?.textContent.slice(0, 160));
  tjek('Trafiktekst: dyk-linjen', tD.d.querySelector('.dyk-trafik-linje')?.textContent.includes('Det meste af trafikspringet var angreb, som Cloudflare stoppede. Det var 1.800. De øvrige 1.400 slap igennem'), tD.d.querySelector('.dyk-trafik-linje')?.textContent);

  /* Året */
  const AAR = Array.from({ length: 40 }, (_, i) => ({ dag: new Date(Date.UTC(2026, 0, 1 + i)).toISOString().slice(0, 10), n: i === 3 ? 0 : 10 + i, hentet: i !== 3 }));
  const aarPeriode = { fra: '2026-01-01', til: '2026-11-02', dage: 306, med: 30, data_fra: '2026-01-01', data_til: '2026-11-02', i_dag: '2026-11-03', sikkerhed_fra: '2026-10-04', trafik_fra: '2026-01-01' };
  const aSvar = dykSvar({ periode: aarPeriode, trafik_serie: AAR, dage: [{ dag: '2026-01-02', n: 0, hentet: false }, { dag: '2026-10-05', n: 4, hentet: true }] });
  const ah = side('#dyk?' + dykQ({ periode: 'alt' }), DYK({ dyk: aSvar })), ahe = side('#dyk?' + dykQ({ periode: 'alt' }), DYK({ dyk: aSvar }), 'en'), a30 = side('#dyk?' + dykQ(), DYK());
  const aOld = side('#dyk?' + dykQ({ dag: '2026-03-01', trafik: '1' }), DYK({ trafik_dag: { dag: '2026-03-01', til: '2026-03-01', i_alt: 900, median: null, stoppet: 0, lande: [{ v: 'DK', n: 800, angreb: 0 }], lande_i_alt: 800, status: [{ v: '200', n: 880 }], status_i_alt: 880, robotter: [], sikkerhed: false, trusler: 12, cachet: 300, server_fejl: 0, ekstra: true } }));
  const aData = side('#indstillinger', DYK({ data_status: { bytes: 1, trafik_fra: '2026-01-01', sikkerhed_fra: '2026-10-04', foerste: '2026-01-01', sidste: '2026-11-03', domaener: 2, nulstil: { sikkerhed: null, trafik: null } }, autostart_hent: true, domaener_hent: [] })), aUden = side('#indstillinger', DYK({ data_status: { bytes: 1, trafik_fra: null, foerste: '2026-10-04', sidste: '2026-11-03', domaener: 2 }, autostart_hent: true, domaener_hent: [] }));
  await sov(260);
  const note = x => x.d.querySelector('.dyk-kun-trafik');
  tjek('Et års trafik: sikkerhedstal over 31 dage står med noten', note(ah)?.textContent === 'Cloudflare gemmer sikkerhedshændelser i en måned — før 4. oktober findes kun trafiktal.' && note(ahe)?.textContent.startsWith('Cloudflare keeps security events for a month'), note(ah)?.textContent);
  tjek('Et års trafik: ingen note inden for sikkerhedstallene', !note(a30));
  const hs = [...ah.d.querySelectorAll('#dyk-rod h2')].map(h => h.textContent), diagrammer = [...ah.d.querySelectorAll('#dyk-rod .vg-soejler-rul svg')];
  tjek('Et års trafik: trafikken har sin egen graf for hele perioden, og angrebene kun, hvor de findes', hs.includes('Forespørgsler dag for dag') && diagrammer.length === 2 && diagrammer[0].querySelectorAll('[data-dyk]').length >= 30, hs.join(' | '));
  const iTrafik = [...diagrammer[0].querySelectorAll('[data-dyk]')].map(x => new URLSearchParams(x.dataset.dyk));
  tjek('Et års trafik: angrebene tegnes kun fra den første dag med sikkerhedstal', diagrammer[1]?.querySelectorAll('.dyk-soejle').length === 1 && new URLSearchParams(diagrammer[1]?.querySelector('[data-dyk]')?.dataset.dyk).get('dag') === '2026-10-05', diagrammer[1]?.querySelectorAll('.dyk-soejle').length);
  tjek('Et års trafik: et hul i trafikken beskylder ikke Huginn', diagrammer[0]?.querySelector('.dyk-uhentet title')?.textContent === '4. januar: ingen tal fra Cloudflare', diagrammer[0]?.querySelector('.dyk-uhentet title')?.textContent);
  tjek('Et års trafik: en dag i trafikgrafen åbner dagens trafik, og et hul er ikke 0', iTrafik.every(q => q.get('trafik') === '1') && !iTrafik.some(q => q.get('dag') === '2026-01-04'), iTrafik.length);
  tjek('Et års trafik: datovælgerens grænse er året', aSvar.periode.data_fra === '2026-01-01' && ah.d.querySelector('.dato-knap'));
  const oldRod = aOld.d.getElementById('dyk-rod');
  tjek('Et års trafik: et gammelt døgn: trusler og cache i stedet for "stoppede", og noten', oldRod.textContent.includes('Trusler (Cloudflares dagstal)') && oldRod.textContent.includes('Fra Cloudflares cache') && !oldRod.querySelector('.vg-fliser').textContent.includes('Cloudflare stoppede') && oldRod.textContent.includes('Dagen er ældre end sikkerhedshændelserne'), oldRod.textContent.slice(0, 200));
  tjek('Et års trafik: Indstillinger → Data', aData.d.getElementById('data-kilder').textContent === 'Trafik fra 1. jan. 2026 · sikkerhedshændelser fra 4. okt. 2026' && !aData.d.getElementById('data-kilder-raekke').hidden && aUden.d.getElementById('data-kilder-raekke').hidden, aData.d.getElementById('data-kilder').textContent);

  /* Lækkede kodeord */
  const LAEK_UD = { ...LOGIN_UD, laek: [{ dag: '2026-11-02', sti: '/login', n: 5, naaet: 2 }, { dag: '2026-11-01', sti: '/wp-login.php', n: 1, naaet: 0 }], laek_andet: [{ v: 'username_leaked', n: 9 }, { v: 'noget_nyt', n: 1 }], laek_dage: 30 };
  const lk = side('#login?zone=z1&navn=wi24rd.com', DYK({ ...VIS, login: LAEK_UD })), lke = side('#login?zone=z1&navn=wi24rd.com', DYK({ ...VIS, login: LAEK_UD }), 'en'), lk0 = side('#login?zone=z1&navn=wi24rd.com', DYK({ ...VIS })), lkv = side('#login?zone=z1&navn=wi24rd.com', DYK({ ...VIS, login: { ...LOGIN_UD, laek_dage: 0 } }));
  await sov(220);
  const lr = lk.d.getElementById('login-rod');
  tjek('Læk i Overblik: "N login-forsøg brugte et kodeord fra et kendt læk"', lr.querySelector('.login-laek-linje')?.textContent === '6 login-forsøg brugte et kodeord fra et kendt læk' && lke.d.querySelector('.login-laek-linje')?.textContent === '6 login attempts used a password from a known leak', lr.querySelector('.login-laek-linje')?.textContent);
  const lt = [...lr.querySelector('.login-laek-linje')?.parentElement.querySelectorAll('tbody tr') || []];
  tjek('Læk i Overblik: pr. dag og sti, med det, der nåede serveren', lt.length === 2 && lt[0].cells[1].textContent === '/login' && lt[0].cells[2].textContent === '5' && lt[0].cells[3].textContent === '2', lt[0]?.textContent);
  tjek('Læk i Overblik: dagen og stien åbner', lt[0]?.cells[0].querySelector('[data-gaa]')?.dataset.gaa.includes('dag=2026-11-02') && lt[0]?.cells[1].querySelector('[data-gaa]')?.dataset.gaa.includes('sti=%2Flogin'), lt[0]?.cells[1].innerHTML.slice(0, 120));
  tjek('Læk i Overblik: forklaringen siger, hvad Cloudflare tjekker, og at intet gemmes', lr.textContent.includes('Et match siger kun, at kodeordet findes i et læk') && lr.textContent.includes('Hverken Cloudflare eller Huginn gemmer kodeordet'));
  tjek('Læk i Overblik: en ukendt værdi står rå', lr.textContent.includes('kun brugernavnet stod i et læk (9), ukendt: noget_nyt (1)'), lr.textContent.slice(0, 80));
  tjek('Læk i Overblik: ingen og ikke hentet', lk0.d.getElementById('login-rod').textContent.includes('Kodeord fra kendte læk: ingen forsøg i perioden.') && lkv.d.getElementById('login-rod').textContent.includes('Tjekket er ikke hentet for perioden endnu.'));

  /* Hvorfor og guiden */
  const med = x => ({ ...OVERBLIK[1], id: 'q1', navn: 'q.wi24rd.com', status: 'roed', angreb_i_gaar: 0, angreb_i_gaar_top: [], ...x });
  const hvOv = (x, sprog) => side('#overblik', DYK({ overblik: [med(x)] }), sprog);
  const h1 = hvOv({ tilstand: [{ slags: 'pause', status: null, kl: null }] }), h2 = hvOv({ tilstand: [{ slags: 'inaktiv', status: 'pending', kl: null }] }), h3 = hvOv({ tilstand: [{ slags: 'phishing', status: null, kl: null }] }), h4 = hvOv({ status: 'gul', tilstand: [{ slags: 'udvikler', status: null, kl: '14.30' }] }), h5 = hvOv({ status: 'gul', laek: { dag: '2026-11-02', n: 1, naaet: 0 } }), h6 = hvOv({ laek: { dag: '2026-11-02', n: 3, naaet: 1 } }), h7 = hvOv({ tilstand: [{ slags: 'pause', status: null, kl: null }] }, 'en');
  await sov(160);
  const hvQ = x => x.d.querySelector('tr[data-zone="q1"] .hv-tekst')?.textContent || '';
  tjek('Domænets tilstand: pause', hvQ(h1).startsWith('Domænet er sat på pause — Cloudflare beskytter det ikke. Ingen angreb i går.'), hvQ(h1));
  tjek('Domænets tilstand: ikke aktiv', hvQ(h2).startsWith('Domænet er ikke aktivt hos Cloudflare (status: pending).'), hvQ(h2));
  tjek('Domænets tilstand: phishing', hvQ(h3).startsWith('Cloudflare mistænker phishing på domænet.'), hvQ(h3));
  tjek('Domænets tilstand: udviklertilstand med tiden', hvQ(h4).startsWith('Udviklertilstand er slået til — cachen er slået fra, til kl. 14.30.'), hvQ(h4));
  tjek('Domænets tilstand: engelsk', hvQ(h7).startsWith('The domain is paused — Cloudflare is not protecting it.'), hvQ(h7));
  tjek('Læk i Overblik: Hvorfor, ordret som notify', hvQ(h5) === 'Ingen angreb i går. 1 login-forsøg brugte et kodeord fra et kendt læk i går.' && hvQ(h6) === 'Ingen angreb i går. 3 login-forsøg brugte et kodeord fra et kendt læk i går, og 1 af dem nåede din server og fik et svar, der kan ligne et login.', hvQ(h6));
  h1.d.querySelector('tr[data-zone="q1"] .hv-guide')?.click(); await sov(20);
  const gk = h1.d.querySelector('.g-kort');
  tjek('Domænets tilstand: guide med Cloudflares ord', gk?.querySelector('[data-kategori="pause"]')?.textContent.includes('Pause Cloudflare'), gk?.textContent.slice(0, 200));
  h6.d.querySelector('tr[data-zone="q1"] .hv-guide')?.click(); await sov(20);
  tjek('Læk i Overblik: guide', h6.d.querySelector('.g-kort [data-kategori="laek"]')?.textContent.includes('rate limiting'));

  const TEK = { linjer: [{ tek: 'wordpress', angreb30: 40, forsoeg30: 0, foerst: '2026-10-05', sidst: '2026-11-02', valg: 'bruger_ikke', besked: false, forslag: false }, { tek: 'php', angreb30: 3, forsoeg30: 0, foerst: '2026-10-05', sidst: '2026-11-02', valg: 'ved_ikke', besked: false, forslag: false }], generiske: [] };
  const tk = side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: vaernUd(), vaern_oversigt: vaernUd(), teknologi_hent: TEK, teknologi_saet: null, teknologi_nulstil: null }));
  await sov(220);
  const tkSek = tk.d.getElementById('vg-teknologi');
  tjek('Dit sites teknologi: chips for valgene, ingen knapgrupper', [...tkSek.querySelectorAll('.tek-chip')].map(x => x.textContent).join() === 'Bruger ikke: WordPress' && !tkSek.querySelector('input[type="radio"]'), tkSek.textContent);
  const retK = tkSek.querySelector('.tek-ret');
  tjek('Dit sites teknologi: "Ret teknologier" åbner #teknologi med domænet', retK.textContent === 'Ret teknologier →' && retK.dataset.gaa === 'teknologi?zone=z1&navn=wi24rd.com', retK.outerHTML);
  const tom = side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: vaernUd(), vaern_oversigt: vaernUd(), teknologi_hent: { linjer: [{ ...TEK.linjer[1] }], generiske: [] } }));
  await sov(220);
  tjek('Dit sites teknologi: uden valg: "Du har ikke sagt, hvad sitet bruger."', tom.d.getElementById('vg-teknologi').textContent.includes('Du har ikke sagt, hvad sitet bruger.') && tom.d.querySelector('#vg-teknologi .tek-ret'), tom.d.getElementById('vg-teknologi').textContent);

  const hvr = sp => side('', DYK({ overblik: [rk('z1', 'wi24rd.com', { ikke_brugt: [{ tek: 'wordpress', n: 47 }], angreb_i_gaar: 47 })] }), sp), hd = hvr('da'), he = hvr('en');
  await sov(120);
  const ln = hd.d.querySelector('.hv-tek');
  tjek('Bruger ikke: "som du ikke bruger" er et link til valget', ln && ln.textContent === 'som du ikke bruger' && ln.dataset.gaa.startsWith('teknologi?') && ln.dataset.gaa.includes('zone=z1') && he.d.querySelector('.hv-tek').textContent === 'which you do not use', ln && ln.outerHTML);
  tjek('Bruger ikke: teksten er den samme med linket', hd.d.querySelector('.hv-tekst').textContent.includes('47 angreb på WordPress, som du ikke bruger — ufarlige for dig.'), hd.d.querySelector('.hv-tekst').textContent);
  tjek('Mere: uden mål i webvisningen står "mere" ikke', !hd.d.querySelector('.hv-mere'));

  /* Del B */
  const sv = x => dykSvar({ enkelte: [{ t: '2026-11-03T21:00:00Z', regel: REACT, cve: null, land: 'PL', sti: '/x', colo: 'WAW', metode: 'GET', n: 3 }], enkelte_info: { vist: 120, i_alt: 300, side: 2, sider: 3, loft: true }, ...x });
  const pg = side('#dyk?' + dykQ({ dag: '2026-11-03', side: '2' }), DYK({ dyk: () => sv() }));
  await sov(120);
  const blokS = [...pg.d.querySelectorAll('#dyk-rod .vg-blok')].at(-1), nav = blokS.querySelector('.dyk-sider');
  tjek('Foreløbigt mærke: "Viser N af M", og det er Huginn, der gemmer 200', blokS.querySelector('.vg-intro')?.textContent === 'Viser 120 af 300 angreb. Huginn gemmer de 200 seneste angreb pr. dag.', blokS.querySelector('.vg-intro')?.textContent);
  tjek('Tidszone: siden sendes til Rust', pg.kald.filter(k => k[0] === 'dyk').at(-1)[1].side === 2);
  tjek('Tidszone: forrige og næste', nav?.textContent.includes('Side 2 af 3') && [...(nav?.querySelectorAll('[data-dyk]') || [])].map(b => new URLSearchParams(b.dataset.dyk).get('side')).join() === ',3', nav?.textContent);
  tjek('Tidszone: hændelser med samme nøgle står med ×', blokS.querySelector('tbody td').textContent === '3. nov. 2026 21:00 ×3', blokS.querySelector('tbody td').textContent);
  const nyt = [...pg.d.querySelectorAll('.vg-fordeling [data-dyk]')][0];
  tjek('Tidszone: et nyt filter starter på første side', nyt && !new URLSearchParams(nyt.dataset.dyk).has('side'));
  const en = side('#dyk?' + dykQ({ dag: '2026-11-03' }), DYK({ dyk: () => sv({ enkelte_info: { vist: 1, i_alt: 1, side: 1, sider: 1, loft: false } }) }), 'en');
  await sov(120);
  tjek('Tidszone: engelsk og ental, og ingen sidevælger ved én side', [...en.d.querySelectorAll('#dyk-rod .vg-blok')].at(-1).querySelector('.vg-intro').textContent === 'Showing 1 of 1 attack.' && !en.d.querySelector('.dyk-sider'));
  const gl = side('#dyk?' + dykQ({ dag: '2026-11-03' }), DYK({ dyk: () => dykSvar({ enkelte_info: null }) }));
  await sov(120);
  tjek('Tidszone: uden gemte angreb er teksten den gamle', [...gl.d.querySelectorAll('#dyk-rod .vg-blok')].at(-1).querySelector('.vg-intro').textContent === gl.w.hmSprog.T.dykSenesteP);
  const rs = fs.readFileSync(path.join(repo, 'src-tauri/src/vaern.rs'), 'utf8'), q = /ANGREB_QUERY: &str = r#"([\s\S]*?)"#/.exec(rs)[1], st = fs.readFileSync(path.join(repo, 'src-tauri/src/store.rs'), 'utf8');
  tjek('Tidszone: ingen IP-adresse eller brugeragent i forespørgsel eller skema', !/clientIP|userAgent/i.test(q) && !/\b(ip|klient_ip|brugeragent)\b/i.test(/SKEMA_11: &str = "([\s\S]*?)";/.exec(st)[1]));

  /* Del C */
  const tz0 = process.env.TZ, ts = side('', DYK({ tidszone_saet: null }));
  await sov(80);
  process.env.TZ = 'Europe/Copenhagen';
  const skift = ts.w.hmRamme.tidsskift(Date.UTC(2026, 9, 1));
  process.env.TZ = 'America/New_York';
  const ny = ts.w.hmRamme.tidsskift(Date.UTC(2026, 9, 1));
  process.env.TZ = tz0;
  tjek('Tidsskift: skiftene til og fra sommertid', JSON.stringify(skift) === JSON.stringify([[Date.UTC(UR_AAR, 9, 1) - 40 * 864e5, 120], [sidsteSoendag(UR_AAR, 9), 60], [sidsteSoendag(UR_AAR + 1, 2), 120], [sidsteSoendag(UR_AAR + 1, 9), 60]].slice(0, skift.length)) && skift.length >= 3, JSON.stringify(skift));
  tjek('Tidsskift: vest for UTC', ny[0][1] === -240 && ny[1][1] === -300, JSON.stringify(ny));
  tjek('Tidsskift: skiftene sendes til Rust ved start', ts.kald.some(k => k[0] === 'tidszone_saet' && Array.isArray(k[1].skift) && k[1].skift.length >= 1));
  const nat = sp => side('#siden-sidst', { ...MED_SIDEN, overblik: [rk('z1', 'a.wi24rd.com', { forgaars: true, angreb_i_gaar: 2, status: 'gul', angreb_fordeling: { blokeret: 2 } })], siden_sidst: { vis: false, nye: [], raekker: [rk('z1', 'a.wi24rd.com', { forgaars: true, angreb_i_gaar: 2, status: 'gul', angreb_fordeling: { blokeret: 2 } })], nye_robotter: [] }, siden_sidst_set: null }, sp), nd = nat('da'), ne = nat('en');
  await sov(120);
  tjek('Tidsskift: Siden sidst siger "i forgårs" efter den lokale dag og UTC én gang', nd.d.getElementById('siden-vist').textContent === 'Status for i forgårs på alle dine domæner. Døgn i UTC, vist i din tid.' && ne.d.getElementById('siden-vist').textContent.endsWith('Days in UTC, shown in your time.'), nd.d.getElementById('siden-vist').textContent);
  tjek('Tidsskift: Hvorfor følger med', nd.d.getElementById('siden-liste').textContent.includes('2 angreb i forgårs') && !nd.d.getElementById('siden-liste').textContent.includes('Opdateres'), nd.d.getElementById('siden-liste').textContent);
  tjek('Rettigheder: nye kommandoer har deres rettighed', ['tidszone-saet', 'teknologi-oversigt', 'teknologi-nulstil'].every(k => ui('../src-tauri/capabilities/default.json').includes(`allow-${k}`)));
}

// ── Dyk, mærkerne og API Token ──
{
  const P30 = { fra: '2026-10-05', til: '2026-11-03', dage: 30, med: 30, data_fra: '2026-10-04', data_til: '2026-11-04', i_dag: '2026-11-04' };
  const titler = x => [...x.d.querySelectorAll('#dyk-rod .vg-blok h2')].map(h => h.textContent);

  /* Rækkefølgen */
  const dy = side('#dyk?' + dykQ({ dag: '2026-11-02' }), DYK({ dyk: () => dykSvar({ periode: { ...P30, fra: '2026-11-02', til: '2026-11-02', dage: 1, med: 1 } }) }));
  const dp = side('#dyk?' + dykQ({ land: 'PL' }), DYK({ dyk: () => dykSvar({ periode: P30 }) }));
  await sov(120);
  const raekke = (x, ...sel) => { const rod = x.d.getElementById('dyk-rod'), alle = [...rod.children]; return sel.map(s => alle.findIndex(e => e.matches(s) || e.querySelector(s))); };
  const stigende = v => v.every(x => x >= 0) && v.every((x, i) => !i || v[i - 1] < x);
  tjek('Dyk-opbygning: dyk er vælgere → fliser → fordeling → kort', stigende(raekke(dp, '.dato-vaelger', '.vg-fliser', '.vg-fordeling', '.dyk-kort')), raekke(dp, '.dato-vaelger', '.vg-fliser', '.vg-fordeling', '.dyk-kort').join());
  tjek('Dyk-opbygning: kortet står før søjler, regelkort og lister, og De seneste sidst', stigende(raekke(dp, '.dyk-kort', '.dyk-soejler', '.vg-regler', '.dyk-lister', '.vg-tabel')), titler(dp).join(' | '));
  tjek('Dyk-opbygning: også for en enkelt dag', stigende(raekke(dy, '.vg-fliser', '.dyk-kort', '.vg-tabel')) && titler(dy).at(-1) === 'De seneste, der blev afvist', titler(dy).join(' | '));
  const tr = side('#dyk?' + dykQ({ dag: '2026-11-02', trafik: '1' }), DYK({ trafik_dag: B2C_TRAFIK }));
  await sov(120);
  tjek('Dyk-opbygning: trafik-dyk har landene på kortets plads, lige efter fliserne', stigende(raekke(tr, '.vg-fliser', '.dyk-trafik-lande')) && !tr.d.querySelector('.dyk-kort') && tr.d.querySelector('.dyk-trafik-lande h3').textContent === tr.w.hmSprog.T.trafikLande);

  /* Kun i dag */
  const NUL = { i_gaar: 0, i_dag_hidtil: 1, hele7: 0, hele30: 0, alt: 0, med7: 7, med30: 30 };
  const kun = sp => side('#dyk?' + dykQ({ robot: 'GoogleOther', periode: 'alt' }), DYK({ dyk: () => dykSvar({ ...ROBOT_DYK, sum: 0, noegletal: NUL, foerst: '2026-11-04', sidst: '2026-11-04', periode: P30, kun_i_dag: { stier: [{ v: '/robots.txt', n: 1 }], lande: [], t: null } }) }), sp);
  const ku = kun('da'), ke = kun('en');
  await sov(120);
  tjek('Kun i dag: boksen siger det ordret og står øverst', ku.d.querySelector('#dyk-rod').firstElementChild?.classList.contains('dyk-kun-i-dag') && ku.d.querySelector('.dyk-kun-i-dag p')?.textContent === 'GoogleOther er kun set i dag (1 besøg indtil nu). Hele døgn tælles fra i morgen.', ku.d.querySelector('#dyk-rod').textContent.slice(0, 200));
  tjek('Kun i dag: boksen har dagens tal og et klik til dagen', ku.d.querySelector('.dyk-kun-i-dag')?.textContent.includes('/robots.txt (1)') && new URLSearchParams(ku.d.querySelector('.dyk-kun-i-dag [data-dyk]')?.dataset.dyk).get('dag') === '2026-11-04');
  tjek('Kun i dag: flisen modsiger ikke "Først set"', ku.d.querySelector('.vg-flise[aria-current]').textContent.includes('0 hele døgn · 1 i dag'), ku.d.querySelector('.vg-flise[aria-current]').textContent);
  tjek('Kun i dag: engelsk', ke.d.querySelector('.dyk-kun-i-dag p')?.textContent === 'GoogleOther has only been seen today (1 visit so far). Whole days are counted from tomorrow.', ke.d.querySelector('.dyk-kun-i-dag p')?.textContent);
  tjek('Kun i dag: uden tal i dag står boksen ikke', !dp.d.querySelector('.dyk-kun-i-dag'));

  /* Fra og Til følger udsnittet */
  const felter = x => [...x.d.querySelectorAll('.dato-knap')].map(b => b.dataset.dag).join();
  tjek('Fra og Til: en dag står i begge felter', felter(dy) === '2026-11-02,2026-11-02', felter(dy));
  tjek('Fra og Til: 30 dage står som perioden', felter(dp) === '2026-10-05,2026-11-03', felter(dp));

  /* Kalenderen: tastatur og skærmlæser */
  const knapFra = dp.d.querySelector('.dato-knap');
  tjek('Kalender: knappen siger felt og dato', knapFra.getAttribute('aria-label') === 'Fra: 5. okt. 2026. Vælg en anden dato' && knapFra.getAttribute('aria-expanded') === 'false', knapFra.getAttribute('aria-label'));
  knapFra.click(); await sov(5);
  const kal = dp.d.querySelector('.kal');
  tjek('Kalender: en dialog med fokus på den valgte dag', kal.getAttribute('role') === 'dialog' && knapFra.getAttribute('aria-expanded') === 'true' && dp.d.activeElement.dataset.dag === '2026-10-05' && dp.d.activeElement.getAttribute('aria-label') === '5. okt. 2026', dp.d.activeElement.outerHTML);
  const tast = (key) => dp.d.activeElement.dispatchEvent(new dp.w.KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  tast('ArrowRight'); tjek('Kalender: pil højre er næste dag', dp.d.activeElement.dataset.dag === '2026-10-06');
  tast('ArrowDown'); tjek('Kalender: pil ned er næste uge', dp.d.activeElement.dataset.dag === '2026-10-13');
  tast('Home'); tjek('Kalender: Home er mandag', dp.d.activeElement.dataset.dag === mandag('2026-10-13'));
  tast('PageDown'); tjek('Kalender: PageDown går ikke forbi i dag', dp.d.activeElement.dataset.dag === mandag('2026-10-13'), dp.d.activeElement.dataset.dag);
  tast('PageUp'); tjek('Kalender: PageUp går ikke før første dag med tal', dp.d.activeElement.dataset.dag === mandag('2026-10-13'), dp.d.activeElement.dataset.dag);
  kal.querySelectorAll('.kal-nav')[1].click(); await sov(5);
  tjek('Kalender: dage efter i dag kan ikke vælges', kal.querySelector('[data-dag="2026-11-05"]').disabled && !kal.querySelector('[data-dag="2026-11-04"]').disabled);
  kal.querySelectorAll('.kal-nav')[0].click(); await sov(5); kal.querySelector(`[data-dag="${mandag('2026-10-13')}"]`).focus();
  tast('Escape');
  tjek('Kalender: Esc lukker og giver fokus tilbage', !dp.d.querySelector('.kal') && dp.d.activeElement === knapFra);
  tjek('Kalender: intet <input type="date"> er tilbage', !ui('assets/vg-tegn.js').includes("type: 'date'"));

  /* Forespørgsler i alt og den gule linje */
  const SPRING = { i_alt: 809000, dage: [{ dag: '2026-10-21', n: 410000, median: 1500, stoppet: 19 }, { dag: '2026-10-22', n: 399000, median: 1500, stoppet: 26 }] };
  const tf = side('#dyk?' + dykQ({ fra: '2026-10-21', til: '2026-10-22' }), DYK({ dyk: () => dykSvar({ periode: { ...P30, fra: '2026-10-21', til: '2026-10-22', dage: 2, med: 2 }, slags: { angreb: 45, rutine: 0 }, trafik: SPRING }) }));
  await sov(120);
  const fl = [...tf.d.querySelectorAll('.vg-flise')].find(x => x.textContent.includes('Forespørgsler i alt'));
  tjek('Forespørgsler i alt: flisen med forholdet', fl && fl.textContent.includes('809.000') && fl.textContent.includes('Angreb: 45 — 1 af hver 17.978'), fl && fl.textContent);
  const gl = tf.d.querySelector('.dyk-trafik-linje');
  tjek('Forespørgsler i alt: den gule linje står øverst', gl && gl.textContent.startsWith('Usædvanlig meget trafik 21.–22. oktober: 809.000 forespørgsler (normalt 1.500 om dagen). Cloudflare stoppede 45 som angreb. De øvrige 808.955 slap igennem og tæller ikke som angreb') && gl.classList.contains('vg-gul') && tf.d.getElementById('dyk-rod').firstElementChild === gl, gl && gl.textContent);
  const q = new URLSearchParams(gl?.querySelector('[data-dyk]')?.dataset.dyk);
  tjek('Forespørgsler i alt: "Se trafikken" er trafik-dyk for perioden', q.get('trafik') === '1' && q.get('fra') === '2026-10-21' && q.get('til') === '2026-10-22' && gl?.querySelector('[data-dyk]')?.textContent === 'Se trafikken →', q.toString());
  gl?.querySelector('[data-dyk]')?.click(); await sov(10);
  const tp = side('#dyk?' + q, DYK({ trafik_dag: { ...B2C_TRAFIK, dag: '2026-10-21', til: '2026-10-22' } }));
  await sov(120);
  tjek('Forespørgsler i alt: trafik-dyk henter hele perioden', JSON.stringify(tp.kald.filter(k => k[0] === 'trafik_dag').at(-1)?.[1] || {}).includes('"dag":"2026-10-21","til":"2026-10-22"') && tp.d.getElementById('dyk-h').textContent.includes('21.–22. oktober'), tp.d.getElementById('dyk-h').textContent);
  const filt = side('#dyk?' + dykQ({ land: 'PL' }), DYK({ dyk: () => dykSvar({ periode: P30, trafik: SPRING }) }));
  const ro = side('#dyk?' + dykQ({ robot: 'GPTBot' }), DYK({ dyk: () => ({ ...ROBOT_DYK, sum: 40, periode: P30, trafik: { i_alt: 4000, dage: [] } }) }));
  await sov(120);
  tjek('Forespørgsler i alt: et filter gælder ikke forespørgslerne', !filt.d.querySelector('.dyk-trafik-linje') && !filt.d.getElementById('dyk-rod').textContent.includes('Forespørgsler i alt'));
  tjek('Forespørgsler i alt: robot-dyk viser besøg i alt', ro.d.getElementById('dyk-rod').textContent.includes('Besøg: 40 — 1 af hver 100'), ro.d.getElementById('dyk-rod').textContent.slice(0, 300));

  /* Ét fælles domænevalg */
  const baand = (x) => x.d.getElementById('vg-zone').value;
  const faelles = () => { let gemt = 'samlet'; const zoner = OVERBLIK.map(r => ({ id: r.id, navn: r.navn })); return side('', { ...DYK({}), robotter: ROBOT_UD, login: LOGIN_UD, login_stier: LOGIN_STIER, vaern_cache: vaernUd(), vaern_oversigt: vaernUd(), vaern_samlet_cache: { domaener: 2, med_tal: 2, valgte: [], vaern: vaernUd() }, domaener_hent: zoner.map(z => ({ ...z, til: true, ny: false })), domaene_valg_hent: () => gemt, domaene_valg_saet: a => { gemt = a.valg; return null; } }, 'da', 'windows'); };
  const f = faelles();
  await sov(150);
  const ctrl = k => { f.tast(k, { ctrlKey: true }); };
  ctrl('3'); await sov(100);
  const vz = f.d.getElementById('vg-zone');
  vz.value = 'z1'; vz.dispatchEvent(new f.w.Event('change')); await sov(100);
  tjek('Fælles domænevalg: wi24rd.com valgt i Angreb', baand(f) === 'z1' && f.kald.some(k => k[0] === 'domaene_valg_saet' && k[1].valg === 'z1'), f.w.location.hash);
  ctrl('4'); await sov(100); tjek('Fælles domænevalg: ⌘4 viser wi24rd.com', baand(f) === 'z1' && f.w.location.hash.startsWith('#robotter') && f.w.location.hash.includes('zone=z1'), f.w.location.hash);
  ctrl('5'); await sov(100); tjek('Fælles domænevalg: ⌘5 viser wi24rd.com', baand(f) === 'z1' && f.w.location.hash.startsWith('#login') && f.w.location.hash.includes('zone=z1'), f.w.location.hash);
  ctrl('3'); await sov(100); tjek('Fælles domænevalg: ⌘3 viser wi24rd.com', baand(f) === 'z1' && f.w.location.hash.startsWith('#vaernet') && f.w.location.hash.includes('zone=z1'), f.w.location.hash);
  ctrl('5'); await sov(100);
  vz.value = 'samlet'; vz.dispatchEvent(new f.w.Event('change')); await sov(100);
  ctrl('3'); await sov(100); tjek('Fælles domænevalg: Samlet i Login → ⌘3 viser Samlet', baand(f) === 'samlet' && f.w.location.hash.includes('samlet=1'), f.w.location.hash);
  f.w.hmRamme.gaaTil('dyk', 'zone=z2&navn=xyz.wi24rd.com&land=PL'); await sov(100);
  ctrl('4'); await sov(100); tjek('Fælles domænevalg: et link med zone skifter det fælles valg', baand(f) === 'z2', f.w.location.hash);
  f.w.history.go(-1); await sov(100);
  tjek('Fælles domænevalg: historikken beholder sin egen adresse', f.w.location.hash.startsWith('#dyk?zone=z2'), f.w.location.hash);
  f.w.hmRamme.tilstand.domaeneValg = 'z-vaek'; ctrl('4'); await sov(100);
  tjek('Fælles domænevalg: et domæne, der er væk, er Samlet', baand(f) === 'samlet' && f.w.location.hash.includes('samlet=1'), f.w.location.hash);

  /* */
  let til = false;
  const au = side('#indstillinger', { ...DYK({ autostart_hent: () => til, autostart_saet: a => { til = a.til; return til; }, data_status: { bytes: 1000, foerste: '2026-08-01', sidste: '2026-11-02', domaener: 2 } }) });
  await sov(100);
  const ban = au.d.getElementById('autostart-banner'), st = au.d.getElementById('indst-status');
  tjek('Start ved login: banneret med knappen står, så snart det er slået fra', !ban.hidden && ban.querySelector('button').textContent === 'Slå Start ved login til');
  const css = ui('assets/app.css');
  tjek('Start ved login: markeringen er rød', /\.autostart-banner \{[^}]*--status-danger-dot[^}]*--status-danger-bg/.test(css) && /\.autostart-banner p::before \{[^}]*--status-danger-dot/.test(css) && !/\.autostart-banner[^{]*\{[^}]*--status-ok/.test(css));
  tjek('Start ved login: noten er kort og står med margen', au.d.getElementById('autostart-hul').textContent === 'Slået fra: der kan komme huller i historikken.' && au.d.getElementById('autostart-hul').classList.contains('kort-note'));
  ban.querySelector('button').click(); await sov(40);
  tjek('Start ved login: knappen slår det til', til && ban.hidden && au.d.getElementById('autostart').checked && au.d.getElementById('autostart-hul').hidden && st.textContent === 'Programmet starter ved login.', st.textContent);

  /* Datoformat: alle tre valg, alle steder */
  const MED_MAERKE = raekke0 => ({ ...raekke0, moenster: { slags: 'rekognoscering', dag: '2026-11-02', stier: 240, normalt: 12, scanner: false, ny: true, netvaerk: null, procent: 0, verden: false, tek: null } });
  const medFormat = (fmtV, hash, sp = 'da', x = {}) => side(hash, DYK({ dyk: () => dykSvar({ periode: P30 }), ...x }), sp, 'windows', w => w.localStorage.setItem('hm-dato', fmtV));
  const fi = medFormat('iso', '#dyk?' + dykQ({ dag: '2026-11-02' })), fs = medFormat('sprog', '#dyk?' + dykQ({ dag: '2026-11-02' })), fe = medFormat('sprog', '#dyk?' + dykQ({ dag: '2026-11-02' }), 'en'), fy = medFormat('system', '#dyk?' + dykQ({ dag: '2026-11-02' }));
  await sov(120);
  const S0 = x => x.w.hmSprog;
  tjek('Datoformat: Følg sproget da og en', S0(fs).dato('2026-11-04') === '4. nov. 2026' && S0(fe).dato('2026-11-04') === '4 Nov 2026' && S0(fs).datoKort('2026-11-04') === '4/11' && S0(fe).datoKort('2026-11-04') === '4 Nov', `${S0(fs).dato('2026-11-04')} ${S0(fe).dato('2026-11-04')}`);
  tjek('Datoformat: ISO alle steder', S0(fi).dato('2026-11-04') === '2026-11-04' && S0(fi).datoLang('2026-11-04') === '2026-11-04' && S0(fi).datoKort('2026-11-04') === '11-04' && S0(fi).periode('2026-10-04', '2026-11-04') === '2026-10-04 – 2026-11-04' && S0(fi).datoTid('2026-11-04T21:00:00Z') === '2026-11-04 21:00' && /^2026-11-03 \d\d:\d\d$/.test(S0(fi).kl('2026-11-03T08:01:00.000Z')), S0(fi).kl('2026-11-03T08:01:00.000Z'));
  const sys = new Intl.DateTimeFormat(undefined, { timeZone: 'UTC', dateStyle: 'medium' }).format(new Date('2026-11-04T00:00:00Z'));
  tjek('Datoformat: Følg systemet bruger webvisningens egen', S0(fy).dato('2026-11-04') === sys, S0(fy).dato('2026-11-04'));
  tjek('Datoformat: De seneste følger formatet', [...fi.d.querySelectorAll('.vg-tabel tbody tr td:first-child')].every(t => /^2026-\d\d-\d\d \d\d:\d\d$/.test(t.textContent)) && [...fs.d.querySelectorAll('.vg-tabel tbody tr td:first-child')].every(t => /^\d+\. [a-zæø]+\.? 2026 \d\d:\d\d$/.test(t.textContent)), fs.d.querySelector('.vg-tabel tbody tr td:first-child')?.textContent);
  tjek('Datoformat: vælgeren og chippen følger formatet', fi.d.querySelector('.dato-knap').textContent === '2026-10-05' && [...fi.d.querySelectorAll('.dyk-filter')].at(-1).textContent.startsWith('2026-11-02') && fs.d.querySelector('.dato-knap').textContent === '5. okt. 2026' && fe.d.querySelector('.dato-knap').textContent === '5 Oct 2026', fi.d.querySelector('.dato-knap').textContent);
  tjek('Datoformat: ingen måned står skrevet i ISO-dyk', !/(oktober|november)/.test(fi.d.getElementById('visning-dyk').textContent), fi.d.getElementById('visning-dyk').textContent.match(/.{20}(oktober|november).{10}/)?.[0]);
  const fo = medFormat('iso', '', 'da', { overblik: [MED_MAERKE(OVERBLIK[0])] });
  await sov(120);
  tjek('Datoformat: valget står i Indstillinger og sendes til Rust', fo.d.getElementById('dato-format').value === 'iso' && JSON.stringify(fo.kald.find(k => k[0] === 'dato_format_saet')[1]).startsWith('{"format":"iso","moenster":"'), JSON.stringify(fo.kald.find(k => k[0] === 'dato_format_saet')));
  tjek('Datoformat: standarden er sproget', side('', DYK({})).d.getElementById('dato-format').value === 'sprog');
  /* fast år */ tjek('Datoformat: ingen kalder toLocaleDateString uden om hjælperen', ['vg-tegn.js', 'visning-dyk.js', 'visning-vaernet.js', 'visning-overblik.js', 'visning-siden.js', 'visning-indstillinger.js', 'visning-land.js', 'visning-login.js', 'visning-robotter.js', 'ramme.js'].every(f => { const k = udenKommentarer(ui('assets/' + f)).replace(/foerste\.toLocaleDateString\(loc2[^)]*\)|new Date\(Date\.UTC\(2026, 10, 2 \+ i\)\)\.toLocaleDateString\(loc2[^)]*\)/g, ''); return !/toLocaleDateString|toLocaleString\(loc, \{ day/.test(k); }));

  /* Mærkerne */
  const REK = { slags: 'rekognoscering', dag: '2026-11-02', stier: 240, normalt: 12, scanner: true, ny: true, netvaerk: null, procent: 0, verden: false, tek: null };
  const MAAL = { slags: 'maalrettet', dag: '2026-11-02', stier: 0, normalt: 0, scanner: false, ny: true, netvaerk: 'Mal AS', procent: 75, verden: true, tek: 'wordpress' };
  const groen = (m, x) => ({ ...OVERBLIK[1], status: 'groen', moenster: m, ...x });
  const ob = sp => side('', { ...MED_SIDEN, overblik: [groen(MAAL, { id: 'z1', navn: 'wi24rd.com' }), groen(REK, { id: 'z2', navn: 'xyz.wi24rd.com' })] }, sp);
  const od = ob('da'), oe = ob('en');
  await sov(120);
  const mk = x => [...x.d.querySelectorAll('.moenster-maerke')];
  tjek('Mærke: i Overblik ved status, også på en grøn række', mk(od).map(m => m.textContent).join() === 'Ligner målrettet,Ligner rekognoscering' && mk(od).every(m => m.closest('td.st') && m.closest('td.st').querySelector('.chip-groen')), mk(od).map(m => m.textContent).join());
  tjek('Mærke: engelsk', mk(oe).map(m => m.textContent).join() === 'Looks targeted,Looks like reconnaissance', mk(oe).map(m => m.textContent).join());
  const knapAf = (m, x) => od.w.hmTegn.moensterKnap(groen(m, x)), k0 = knapAf(MAAL, { id: 'z1', navn: 'wi24rd.com' }), m0 = new URLSearchParams(k0.dataset.dyk), m1 = new URLSearchParams(knapAf(REK, { id: 'z2', navn: 'xyz.wi24rd.com' }).dataset.dyk);
  tjek('Mærke: målrettet åbner dagen med netværket', m0.get('dag') === '2026-11-02' && m0.get('netvaerk') === 'Mal AS' && m0.get('zone') === 'z1' && m0.get('slags') === 'angreb', m0.toString());
  tjek('Mærke: rekognoscering åbner dagen med stierne', m1.get('dag') === '2026-11-02' && m1.get('forsoeg') === 'alle', m1.toString());
  tjek('Mærke: som knap (Angreb, Siden sidst) med label og title', k0.tagName === 'BUTTON' && k0.getAttribute('aria-label') === 'Ligner målrettet på wi24rd.com den 2. november: åbn dagen' && k0.title.startsWith('Ligner målrettet:'), k0.getAttribute('aria-label'));
  tjek('Ét klikmål: i Overblik er mærket tekst med forklaringen som title', mk(od)[0]?.tagName === 'SPAN' && mk(od)[0]?.title.startsWith('Ligner målrettet:'), mk(od)[0]?.outerHTML.slice(0, 100));
  const SP = od.w.hmSprog, SE = oe.w.hmSprog;
  tjek('Mærke: teksterne er ens med Rust', SP.moensterTekst(MAAL) === 'Ligner målrettet: angrebene steg her, men ikke i resten af verden, og 75 % kom fra ét netværk, og angrebene ramte WordPress, som dit site bruger.' && SE.moensterTekst(MAAL) === 'Looks targeted: attacks rose here but not elsewhere, and 75% came from one network, and the attacks hit WordPress, which your site uses.' && SP.moensterTekst({ ...REK, scanner: false }) === 'Ligner rekognoscering: 240 forskellige stier på én dag (normalt 12).' && SP.moensterTekst(REK) === 'Ligner rekognoscering: 240 forskellige stier på én dag (normalt 12), som en almindelig scanner.', SP.moensterTekst(REK));
  tjek('Mærke: altid "ligner" og "looks"', [MAAL, REK, { ...MAAL, verden: false }].every(m => SP.moensterTekst(m).startsWith('Ligner ') && SE.moensterTekst(m).startsWith('Looks ')));
  tjek('Mærke: uden Radar siger teksten kun dine andre domæner', SP.moensterTekst({ ...MAAL, verden: false, tek: null }) === 'Ligner målrettet: angrebene steg her, men ikke på dine andre domæner, og 75 % kom fra ét netværk.');
  tjek('Mærke: status er uændret', od.d.querySelector('#overblik-raekker .chip-groen') && !od.d.querySelector('#overblik-raekker .chip-roed'));
  tjek('Mærke: guide-posterne findes på begge sprog', SP.bog('maalrettet').alle && SP.bog('rekognoscering').alle === 'Nogen har prøvet mange adresser for at se, hvad der findes. De fleste er automatiske scannere.' && SE.bog('maalrettet').alle.includes('cannot say for certain') && SP.bog('maalrettet').ja.join(' ').includes('Under Attack Mode'));
  od.d.querySelector('#overblik-raekker .hv-guide').click(); await sov(10);
  tjek('Mærke: "Hvad betyder Ligner målrettet?" i guiden', od.d.getElementById('hjaelp-guide').textContent.includes('Hvad betyder »Ligner målrettet«?'), od.d.getElementById('hjaelp-guide').textContent.slice(0, 200));

  const SIDE = { ...MED_SIDEN, overblik: [groen(MAAL, { id: 'z1', navn: 'wi24rd.com' })], siden_sidst: { vis: false, nye: ['z1'], raekker: [groen(MAAL, { id: 'z1', navn: 'wi24rd.com' }), groen(null, { id: 'z2', navn: 'xyz.wi24rd.com' })], nye_robotter: [] }, siden_sidst_set: null };
  const sd = side('#siden-sidst', SIDE);
  await sov(120);
  const kortS = sd.d.querySelector('#siden-liste li .moenster-maerke');
  tjek('Mærke: i Siden sidst med dom og klik', kortS && kortS.dataset.dyk.includes('netvaerk=Mal') && kortS.closest('li').textContent.includes('Ligner målrettet: angrebene steg her') && sd.d.getElementById('siden-liste').textContent.includes('1 domæne'), sd.d.getElementById('siden-liste').textContent.slice(0, 240));
  const vn = side('#vaernet?zone=z1&navn=wi24rd.com', { ...DYK({ overblik: [groen(MAAL, { id: 'z1', navn: 'wi24rd.com' })], vaern_cache: vaernUd(), vaern_oversigt: vaernUd() }) });
  await sov(200);
  const ml = vn.d.querySelector('.moenster-linje');
  tjek('Mærke: i Angreb øverst med forklaring', ml && vn.d.getElementById('vagt').firstElementChild === ml && ml.textContent.includes('og 75 % kom fra ét netværk') && ml.querySelector('[data-dyk]') && ml.querySelector('button.link').textContent === 'Hvad betyder det? →', vn.d.getElementById('vagt').textContent.slice(0, 200));
  /* Rækkerne er her allerede, når Angreb åbnes igen: så er det tegningen selv, der skal sætte linjen. */
  vn.w.hmRamme.gaaTil('overblik'); await sov(40);
  vn.w.hmRamme.gaaTil('vaernet', 'zone=z1&navn=wi24rd.com'); await sov(200);
  tjek('Mærke: står også, når Angreb åbnes igen uden nye rækker', vn.d.getElementById('vagt').firstElementChild?.classList.contains('moenster-linje'), vn.d.getElementById('vagt').textContent.slice(0, 120));
  vn.w.hmRamme.tilstand.raekker = [groen(null, { id: 'z1', navn: 'wi24rd.com' })]; vn.w.hmRamme.udsend('raekker'); await sov(20);
  tjek('Mærke: forsvinder, når overblikket ikke længere har det', !vn.d.querySelector('.moenster-linje'));

  const nt = side('#indstillinger', DYK({ notifikation_hent: { niveau: 'standard', roed: true, rekognoscering: false, zoner: {} }, notifikation_rekog_saet: null, data_status: { bytes: 1000, foerste: null, sidste: null, domaener: 0 }, autostart_hent: true }));
  await sov(100);
  const rk = nt.d.getElementById('notif-rekog');
  tjek('Mærke, notifikation: rekognoscering er fra som standard', rk && !rk.checked && rk.getAttribute('role') === 'switch');
  rk.checked = true; rk.dispatchEvent(new nt.w.Event('change')); await sov(30);
  tjek('Mærke, notifikation: kontakten gemmes i Rust', JSON.stringify(nt.kald.filter(k => k[0] === 'notifikation_rekog_saet').at(-1)) === '["notifikation_rekog_saet",{"til":true}]');

  /* Del C */
  const op = side('', { token_status: { sat: false }, autostart_hent: true, aabn_api_token: null, token_set: () => { throw { kode: 'mangler_analytics' }; } }), ope = side('', { token_status: { sat: false }, autostart_hent: true, token_set: () => { throw { kode: 'mangler_zone' }; } }, 'en');
  await sov(80);
  const opS = op.d.getElementById('visning-opsaetning'), ak = [...opS.querySelectorAll('button')].find(b => b.textContent === 'Opret API Token hos Cloudflare');
  tjek('API Token: knappen i opsætningen åbner Cloudflare gennem Rust', ak && (ak.click(), true) && op.kald.some(k => k[0] === 'aabn_api_token'));
  const ind = side('#indstillinger', DYK({ autostart_hent: true, data_status: { bytes: 1000, foerste: null, sidste: null, domaener: 0 } }));
  await sov(100);
  tjek('API Token: knappen og trinene står også under Forbindelse', [...ind.d.querySelectorAll('#visning-indstillinger button')].some(b => b.textContent === 'Opret API Token hos Cloudflare') && ind.d.querySelector('#visning-indstillinger details.token-trin'));
  const trin = [...opS.querySelectorAll('.token-trin li')].map(l => l.textContent);
  tjek('API Token: trinene med Cloudflares ord, foldbart', opS.querySelector('details.token-trin') && !opS.querySelector('details.token-trin').open && trin.length === 9 && ['Create Token', 'Create Custom Token', 'Get started', 'Token name', 'Zone › Analytics › Read', 'Zone › Zone › Read', 'Zone Resources', 'Include › All zones', 'Specific zone', 'Continue to summary'].every(o => trin.join(' ').includes(o)), trin.join(' | '));
  const it = opS.querySelector('.token-it');
  tjek('API Token: IT-firma-linjen står over feltet', it?.textContent === 'Har du fået et API Token af dit IT-firma? Indsæt det her.' && it && (it.compareDocumentPosition(op.d.getElementById('token')) & 4) !== 0);
  op.d.getElementById('token').value = 'x'.repeat(40); op.d.getElementById('token-form').dispatchEvent(new op.w.Event('submit', { cancelable: true, bubbles: true })); await sov(40);
  const ts = op.d.getElementById('token-status');
  tjek('API Token: fejlen siger, hvilken rettighed der mangler', ts.textContent.startsWith('API Tokenet mangler rettigheden Zone › Analytics › Read') && ts.classList.contains('fejl'), ts.textContent);
  ts.querySelector('button')?.click();
  tjek('API Token: fejlen har et link til trinene', ts.querySelector('button')?.textContent === 'Se trinene' && opS.querySelector('details.token-trin').open);
  ope.d.getElementById('token').value = 'x'.repeat(40); ope.d.getElementById('token-form').dispatchEvent(new ope.w.Event('submit', { cancelable: true, bubbles: true })); await sov(40);
  tjek('API Token: engelsk og den anden rettighed', ope.d.getElementById('token-status').textContent.startsWith('The API Token cannot see any domains: it lacks the permission Zone › Zone › Read'), ope.d.getElementById('token-status').textContent);
  tjek('API Token: ny kommando har sin rettighed', ['aabn-api-token', 'domaene-valg-hent', 'domaene-valg-saet', 'dato-format-saet', 'notifikation-rekog-saet'].every(k => ui('../src-tauri/capabilities/default.json').includes(`allow-${k}`)));
}

// ── Al tekst har mindst 7:1 mod det, den faktisk står på (målt i en rigtig browser, lys og mørk) ──
{
  const CHROME = [process.env.HM_CHROME, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome', '/usr/bin/chromium'].find(f => f && fs.existsSync(f));
  /* ── <Land> lige nu fra målsiden, regelkort i dyk og "Er sitet oppe?" ── */
  {
    const s7 = side('#land?kode=DK&periode=7', LANDV()), s30 = side('#land?kode=DK', LANDV({ land_visning: a => ({ ...LAND_UD, kode: a.kode, andel: [{ dag: '2026-11-02', vaerdi: 0.4 }, { dag: '2026-11-01', vaerdi: null }, { dag: '2026-10-31', vaerdi: 0.8 }] }) }));
    const en = side('#land?kode=DK', LANDV(), 'en');
    const ens = side('#land?kode=DK', LANDV({ land_visning: a => ({ ...LAND_UD, kode: a.kode, sammen28: { land: [], rest: null, spurgt: true, mine: [], mine_i_alt: 0, fra: '2026-10-06', til: '2026-11-02' }, udfald: [], udfald_hentet: true }) }));
    const tom = side('#land?kode=DK', LANDV({ land_visning: a => ({ ...LAND_UD, kode: a.kode, sammen28: { land: [], rest: null, spurgt: false, mine: [], mine_i_alt: 0, fra: '2026-10-06', til: '2026-11-02' }, udfald: [], udfald_hentet: false, andel: [] }) }));
    const vd = side('#land?kode=DK&fane=verden', LANDV());
    await sov(90);
    const kol = x => [...x.d.getElementById('land-kilde').querySelectorAll('.vg-tre > div')];
    tjek('Land og site, 7 dage: kolonnerne skifter til 7 dage, og dyk får perioden', kol(s7)[0].textContent.includes('41 %') && new URLSearchParams(kol(s7)[1].querySelector('button').dataset.dyk).get('periode') === '7' && s7.d.querySelector('#land-kilde input:checked').parentElement.textContent === '7 dage', kol(s7)[0].textContent);
    tjek('Land og site, procenterne står pr. kolonne, som Rust sender dem', [...kol(s7)[1].querySelectorAll('.vg-l-n')].map(x => x.textContent).join() === '60 %,40 %', kol(s7)[1].textContent);
    tjek('Land og site, XX i din kolonne er "Ukendt land" og kan stadig åbnes', kol(s30)[1].textContent.includes('Ukendt land') && kol(s30)[1].querySelectorAll('button').length === 2);
    const radio = s30.d.querySelectorAll('#land-kilde input')[0];
    radio.focus(); radio.checked = true; radio.dispatchEvent(new s30.w.Event('change', { bubbles: true }));
    await sov(60);
    tjek('Land og site, vælgeren: ét trin i adressen, afsnittet bevares, og fokus bliver på valget', s30.w.location.hash.includes('periode=7') && s30.w.location.hash.includes('afsnit=kilde') && s30.d.activeElement.dataset.fokus === 'land-mod:7 dage', s30.w.location.hash);
    const svg = s30.d.getElementById('land-maal').querySelector('svg');
    tjek('Land og site, andelen dag for dag er én kurve, som ikke kan klikkes og siger andel, ikke antal', svg && svg.getAttribute('role') === 'img' && !svg.querySelector('[data-dyk], [tabindex]') && s30.d.getElementById('land-maal').textContent.includes('Andel af alle angreb, Cloudflare så') && svg.querySelector('title').textContent.includes('af alle angreb, Cloudflare så'), svg && svg.outerHTML.slice(0, 120));
    tjek('Land og site, kurven for andelen: procent på aksen og ingen 100-linje', s30.d.querySelector('#land-maal .dyk-y-top').textContent === '0,8 %' && s30.d.querySelectorAll('#land-maal .dyk-y').length === 1, s30.d.querySelector('#land-maal .dyk-y-top')?.textContent);
    tjek('Trafik og huller, ikke oplyst: ingen liste, en ærlig note, og din kolonne står for sig', kol(ens)[0].textContent.includes('Cloudflare Radar oplyser ikke, hvor angrebene mod Danmark kom fra (landet er for lille i statistikken)') && !kol(ens)[0].querySelector('li'), kol(ens)[0].textContent);
    tjek('Land og site, tom tilstand for forstyrrelser, kun når der er spurgt', ens.d.getElementById('land-rod').textContent.includes('Ingen kendte forstyrrelser i Danmark de seneste 30 dage') && !tom.d.getElementById('land-rod').textContent.includes('Ingen kendte forstyrrelser') && tom.d.getElementById('land-rod').textContent.includes('ikke hentet endnu'));
    tjek('Land og site, tomme kolonner', kol(tom)[0].textContent.includes('Ingen tal for angreb mod Danmark endnu') && kol(tom)[1].textContent.includes('Ingen angreb på dine domæner i perioden'));
    tjek('Land og site, engelsk', kol(en)[0].querySelector('h3').textContent === 'The largest sources of attacks on Denmark' && kol(en)[1].querySelector('.vg-saerligt').textContent === 'Especially you' && en.d.getElementById('land-rod').textContent.includes('A cable was cut · 3 hours · one network'), kol(en)[0].textContent);
    const vt = vd.d.getElementById('land-rod').textContent;
    tjek('Land og site, Verden: kvante, AI-robotter efter navn og noten; intet af det under landet', vt.includes('Kvantesikre forbindelser') && vt.includes('42,5 %') && vt.includes('AI-robotter efter navn') && vt.includes('GPTBot') && vt.includes('ikke hvor sitet står') && !s7.d.getElementById('land-rod').textContent.includes('Kvantesikre') && !s7.d.getElementById('land-rod').textContent.includes('GPTBot'));
    tjek('Land og site, ingen tekst nævner "Angreb fra {land}"', !/landFra|Angreb fra \{land\}|Attacks from \{land\}/.test(ui('assets/sprog.js') + ui('assets/visning-land.js')));

    /* Dyk */
    const dl = side('#dyk?' + dykQ({ land: 'PL' }), DYK()), dr = side('#dyk?' + dykQ({ regel: REACT }), DYK()), dt = side('#dyk?' + dykQ({ dag: '2026-11-03' }), DYK()), dp = side('#dyk?' + dykQ({ periode: '7' }), DYK());
    const ru = side('#dyk?' + dykQ({ land: 'PL', slags: 'rutine' }), DYK()), ang = side('#dyk?' + dykQ({ land: 'PL', slags: 'angreb' }), DYK()), rue = side('#dyk?' + dykQ({ land: 'PL', slags: 'rutine' }), DYK(), 'en');
    const B_VAERN = vaernUd(), dv = side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: B_VAERN, vaern_oversigt: B_VAERN, koe_prioritet: null }));
    await sov(90);
    const kort = dl.d.querySelectorAll('#dyk-rod .vg-regel'), h2 = x => [...x.d.querySelectorAll('#dyk-rod h2')].map(h => h.textContent);
    tjek('Land og site, dyk: overskriften siger udsnittet i ord', h2(dl).includes('Reglerne, der slog til — Polen') && h2(dt).includes('Reglerne, der slog til — 3. november') && h2(dp).includes('Reglerne, der slog til — 7 dage'), h2(dl).join(' | '));
    tjek('Land og site, dyk: samme kort som Angreb (mærke, CVE, stier, fra, datoer)', kort.length === 2 && kort[0].querySelector('.vg-produkt') && kort[0].querySelector('.vg-cve') && kort[0].querySelector('.vg-regel-d').textContent === 'Først set 1. november, sidst 3. november' && kort[1].classList.contains('vg-rutine'), kort[0] && kort[0].textContent);
    tjek('Land og site, dyk: Angreb og dyk bruger samme byggeklods', dv.d.querySelector('.vg-regel .vg-regel-aabn') && /V\.regelKort\(/.test(ui('assets/visning-vaernet.js')) && /V\.regelKort\(/.test(ui('assets/visning-dyk.js')));
    const aabnQ = new URLSearchParams(kort[0].querySelector('.vg-regel-aabn').dataset.dyk);
    tjek('Land og site, dyk: et klik på kortet kæder reglen på udsnittet', aabnQ.get('regel') === REACT && aabnQ.get('land') === 'PL' && aabnQ.get('navn') === 'wi24rd.com');
    const sq = new URLSearchParams(kort[0].querySelectorAll('.vg-regel-l button')[0].dataset.dyk), lq = new URLSearchParams(kort[0].querySelectorAll('.vg-regel-l button')[1].dataset.dyk);
    tjek('Land og site, dyk: stier og lande på kortet kæder også reglen på', sq.get('regel') === REACT && sq.get('sti') === '/api' && sq.get('land') === 'PL' && lq.get('regel') === REACT && lq.get('land') === 'PL');
    tjek('Land og site, dyk: tallet er udsnittets, uden "7 dage" og "30 dage"', kort[0].querySelector('.vg-regel-n').textContent === '150 i udsnittet' && !kort[0].querySelector('.vg-regel-n span'), kort[0].querySelector('.vg-regel-n').textContent);
    tjek('Land og site, regel-dyk: ingen andre regelkort, og hovedet står', !dr.d.querySelector('#dyk-rod .vg-regel') && dr.d.querySelector('.dyk-hoved') && !h2(dr).some(h => h.startsWith('Reglerne, der slog til')), h2(dr).join(' | '));
    tjek('Land og site, Kun rutine: overskriften hedder Hændelser (da, en)', ru.d.getElementById('dyk-h').textContent === 'Hændelser på wi24rd.com fra Polen' && rue.d.getElementById('dyk-h').textContent === 'Events on wi24rd.com from Poland', ru.d.getElementById('dyk-h').textContent);
    tjek('Land og site, Kun angreb: overskriften hedder stadig Angreb', ang.d.getElementById('dyk-h').textContent === 'Angreb på wi24rd.com fra Polen');
    tjek('Land og site, fokus ved åbning af dyk lander på knappen', dl.d.activeElement === dl.d.querySelector('#dyk-tilbage button'), dl.d.activeElement && dl.d.activeElement.outerHTML.slice(0, 80));

    /* Er sitet oppe? */
    const ov = side('#overblik', DYK({ overblik: B2B_OVERBLIK })), ove = side('#overblik', DYK({ overblik: B2B_OVERBLIK }), 'en');
    const ingen = side('#overblik', DYK());
    const at = side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: { ...B_VAERN, trafik: B2B_TRAFIK }, vaern_oversigt: { ...B_VAERN, trafik: B2B_TRAFIK }, koe_prioritet: null }));
    await sov(90);
    const rk = id => ov.d.querySelector(`tr[data-zone="${id}"]`), linje = ov.d.getElementById('server-linje');
    tjek('Land og site, egen gul linje med domænerne', !linje.hidden && /--status-warn-bg/.test(ui('assets/vagt.css').match(/\.server-linje \{[^}]*\}/)[0]) && linje.textContent === 'Din server svarer ikke: roed.wi24rd.com, nede.wi24rd.com', linje.textContent);
    tjek('Land og site, uden 52x står linjen ikke', ingen.d.getElementById('server-linje').hidden);
    const hv = rk('s1').querySelector('.hv-tekst').textContent;
    tjek('Land og site, Hvorfor er ordret den samme som notify::aarsag, plus forstyrrelsen', hv === 'Ingen angreb i går. Din server svarer ikke: 33 % af 120 svar i går var fejl 520–527, så Cloudflare kunne ikke få svar fra din server. Se »Hvad gør jeg?«. Samme dag var der en forstyrrelse hos Fjord Net i Danmark.', hv);
    tjek('Land og site, Hvorfor på engelsk', ove.d.querySelector('tr[data-zone="s1"] .hv-tekst').textContent === 'No attacks yesterday. Your server is not answering: 33% of 120 responses yesterday were errors 520–527, so Cloudflare could not get an answer from your server. See “What do I do?”. The same day there was a disruption at Fjord Net in Denmark.', ove.d.querySelector('tr[data-zone="s1"] .hv-tekst').textContent);
    tjek('Land og site, rød forbliver rød, og Hvorfor nævner også serveren', rk('r1').querySelector('.chip').className.includes('chip-roed') && rk('r1').querySelector('.hv-tekst').textContent.includes('Din server svarer ikke'));
    const m = rk('t1').querySelector('.trafik-maerke'), mQ = new URLSearchParams(m.dataset.dyk);
    tjek('Trafik og huller, mærket åbner trafik-dyk for dagen og siger, hvad det er', m.textContent === 'Usædvanlig meget trafik' && mQ.get('dag') === '2026-11-02' && mQ.get('trafik') === '1' && mQ.get('zone') === 't1' && m.getAttribute('aria-label').includes('350.000'), m.getAttribute('aria-label'));
    tjek('Land og site, mærket har egen hover og markør', /\.trafik-maerke:hover \{[^}]*border-color/.test(ui('assets/vagt.css')) && /\.trafik-maerke \{[^}]*cursor: pointer/.test(ui('assets/vagt.css')));
    tjek('Trafik og huller, trafikspring står som Hold øje, og mærket kalder det ikke angreb', rk('t1').querySelector('.chip').className.includes('chip-gul') && !/angreb/i.test(m.textContent));
    rk('s1').querySelector('.hv-guide').click();
    await sov(40);
    const g = ov.d.getElementById('guide-indhold');
    tjek('Land og site, Hvorfor uden angreb har et lille link til guiden, som forklarer 52x og DNS-posten', g && g.textContent.includes('Fejl 520–527') && g.textContent.includes('Din DNS peger måske på en IP-adresse, du ikke ejer længere') && g.textContent.includes('DNS → Records') && g.textContent.includes('Content') && g.textContent.includes('den, der har adressen nu') && !g.querySelector('.g-se') && !g.querySelector('.g-slut') && w0(ov).location.hash === '#overblik', g && g.textContent.slice(0, 200));
    const b2 = at.d.querySelectorAll('#vagt .vg-trafik .trafik-maerke'), n2 = at.d.querySelector('#vagt .vg-trafik-note');
    tjek('Land og site, Angreb: hver dag er en knap til dyk, med ental ved 1.000 og flertal ellers', b2.length === 2 && b2[0].textContent === 'Usædvanlig meget trafik den 27. oktober: 350.000 forespørgsler. Cloudflare stoppede 67 som angreb, og 349.933 slap igennem' && new URLSearchParams(b2[0].dataset.dyk).get('dag') === '2026-10-27' && new URLSearchParams(b2[0].dataset.dyk).get('trafik') === '1', b2[0] && b2[0].textContent);
    tjek('Trafik og huller, Angreb: noten påstår ikke, at det er harmløst', n2.textContent === 'Det, Cloudflare ikke stoppede, tæller ikke som angreb. Det kan være et forsøg på at overbelaste siden (DDoS), en robot, der løber løbsk, eller en fejl. En normal dag har 2.000 forespørgsler.', n2.textContent);
    tjek('Land og site, Angreb uden trafikspring har ingen linje', !dv.d.querySelector('#vagt .trafik-maerke'));
  }

  /* ── Ærlig trafik, angreb mod landet, datovælger, domænets teknologi og huller ── */
  {
    const TRAFIK_UD = { dag: '2026-11-02', til: '2026-11-02', i_alt: 410000, median: 1500, stoppet: 67, lande: [{ v: 'US', n: 378000, angreb: 60 }, { v: 'DE', n: 32886, angreb: 0 }], lande_i_alt: 410000, status: [{ v: '522', n: 345000 }, { v: '200', n: 65886 }], status_i_alt: 410000, sikkerhed: true, trusler: 0, cachet: 0, server_fejl: 345000, robotter: [{ v: 'Googlebot', n: 300000 }], ekstra: true };
    const tq = (x = {}) => new URLSearchParams({ zone: 'z1', navn: 'wi24rd.com', dag: '2026-11-02', trafik: '1', ...x }).toString();
    const td = side('#dyk?' + tq(), DYK({ trafik_dag: TRAFIK_UD })), tde = side('#dyk?' + tq(), DYK({ trafik_dag: TRAFIK_UD }), 'en');
    const tu = side('#dyk?' + tq(), DYK({ trafik_dag: { ...TRAFIK_UD, lande: [], status: [], robotter: [], lande_i_alt: 0, status_i_alt: 0, sikkerhed: true, trusler: 0, cachet: 0, server_fejl: 0, ekstra: false, median: null } }));
    await sov(90);
    const rodT = td.d.getElementById('dyk-rod'), tt = rodT.textContent, kald = td.kald.find(k => k[0] === 'trafik_dag');
    tjek('Trafik-dyk: spørger trafik_dag for dagen og ikke dyk', kald && kald[1].dag === '2026-11-02' && JSON.stringify(kald[1].omfang) === '{"zone":"z1"}' && !td.kald.some(k => k[0] === 'dyk'), JSON.stringify(kald));
    tjek('Trafik-dyk: overskrift, tal og normal', td.d.getElementById('dyk-h').textContent === 'Trafik på wi24rd.com den 2. november' && tt.includes('410.000') && tt.includes('normalt 1.500') && tt.includes('Cloudflare stoppede'), td.d.getElementById('dyk-h').textContent);
    tjek('Trafik-dyk: de tre fordelinger', ['Besøgende pr. land', 'Svarkoder', 'Robotter'].every(x => [...rodT.querySelectorAll('h3')].some(h => h.textContent === x)), [...rodT.querySelectorAll('h3')].map(h => h.textContent).join());
    tjek('Trafik-dyk: sætninger i klarsprog', tt.includes('92 % kom fra ét land (USA)') && tt.includes('84 % fik 52x (din server svarede ikke)') && tt.includes('De fleste var robotten Googlebot'), tt.slice(0, 300));
    tjek('Trafik-dyk: engelsk', tde.d.getElementById('dyk-rod').textContent.includes('84% got 52x (your server did not answer)') && tde.d.getElementById('dyk-h').textContent === 'Traffic on wi24rd.com on 2 November', tde.d.getElementById('dyk-h').textContent);
    const raek = h => [...rodT.querySelectorAll('.dyk-lister > div')].find(x => x.querySelector('h3').textContent === h);
    const usa = raek('Besøgende pr. land').querySelectorAll('li');
    tjek('Trafik-dyk: et land med stoppede åbner dyk; et land uden har intet at åbne', usa[0].querySelector('button') && new URLSearchParams(usa[0].querySelector('button').dataset.dyk).get('land') === 'US' && !new URLSearchParams(usa[0].querySelector('button').dataset.dyk).has('trafik') && !usa[1].querySelector('button'));
    tjek('Trafik-dyk: svarkoder er ikke klikbare, robotter er', !raek('Svarkoder').querySelector('button') && new URLSearchParams(raek('Robotter').querySelector('button').dataset.dyk).get('robot') === 'Googlebot');
    tjek('Trafik-dyk: "Cloudflare stoppede" åbner dagens dyk', new URLSearchParams(rodT.querySelector('button.vg-flise').dataset.dyk).get('dag') === '2026-11-02' && !new URLSearchParams(rodT.querySelector('button.vg-flise').dataset.dyk).has('trafik'));
    tjek('Trafik-dyk: guiden med Cloudflares ord', ['HTTP DDoS attack protection', 'Security rules', 'DDoS protection', 'Under Attack Mode', 'Quick Actions', 'Rate limiting rules', 'DNS → Records'].every(x => tt.includes(x)) && rodT.querySelector('details summary').textContent === 'Hvad gør jeg ved usædvanlig meget trafik?', tt.slice(-500));
    tjek('Trafik-dyk: uden ekstra tal siger den det, og uden median ingen "normalt"', tu.d.getElementById('dyk-rod').textContent.includes('Tal for lande og svarkoder er ikke hentet for dagen endnu') && !tu.d.getElementById('dyk-rod').textContent.includes('normalt'));
    tjek('Trafik og huller, ingen tekst kalder et trafikspring "ikke et angreb"', !/ikke et angreb|not an attack/i.test(ui('assets/sprog.js') + ui('assets/visning-overblik.js') + ui('assets/visning-vaernet.js') + ui('assets/visning-dyk.js')) && !/ikke et angreb|not an attack/i.test(fs.readFileSync(path.join(repo, 'src-tauri/src/notify.rs'), 'utf8').split('#[cfg(test)]')[0]));
    const trafikR = raekke('t1', 'trafik.wi24rd.com', 'gul', { trafik: { dag: '2026-11-02', n: 410000, median: 1500, stoppet: 67 }, angreb_status: 'groen', sidst_hentet: NU_ISO });
    const ovT = side('#overblik', DYK({ overblik: [trafikR] })), ovTe = side('#overblik', DYK({ overblik: [trafikR] }), 'en');
    await sov(90);
    const hvT = ovT.d.querySelector('tr[data-zone="t1"] .hv-tekst');
    tjek('Trafik og huller, Hvorfor er ordret den samme som notify::aarsag', hvT.textContent === 'Ingen angreb i går. Usædvanlig meget trafik: 410.000 forespørgsler (normalt 1.500). Cloudflare stoppede 67 som angreb. De øvrige 409.933 slap igennem og tæller ikke som angreb — det kan være et forsøg på at overbelaste siden (DDoS), en robot, der løber løbsk, eller en fejl. Se, hvor trafikken kom fra.', hvT.textContent);
    tjek('Trafik og huller, Hvorfor på engelsk', ovTe.d.querySelector('tr[data-zone="t1"] .hv-tekst').textContent === 'No attacks yesterday. Unusually high traffic: 410,000 requests (normally 1,500). Cloudflare stopped 67 as attacks. The other 409,933 got through and do not count as attacks — it may be an attempt to overload the site (DDoS), a robot running wild, or a fault. See where the traffic came from.');
    ovT.d.querySelector('tr[data-zone="t1"] .hv-guide').click(); await sov(40);
    const gT = ovT.d.getElementById('guide-indhold');
    tjek('Trafik og huller, guiden har trafik og DNS, ikke 52x', gT.textContent.includes('Hvad gør jeg ved usædvanlig meget trafik?') && gT.textContent.includes('Din DNS peger måske') && !gT.textContent.includes('Din server svarer ikke'), gT.textContent.slice(0, 120));

    /* Landet */
    const VERDEN_UD = [{ start: '2026-10-30T04:00:00Z', slut: '2026-10-30T06:00:00Z', minutter: 120, aarsag: 'CABLE_CUT', slags: 'NATIONWIDE', netvaerk: [], lande: ['SE', 'ZZ'] }];
    const ld = side('#land?kode=DK', LANDV({ land_visning: a => ({ ...LAND_UD, kode: a.kode, udfald: [], udfald_hentet: true, udfald_verden: VERDEN_UD, udfald_verden_hentet: true }) }));
    const lv = side('#land?kode=DK&fane=verden', LANDV({ land_visning: a => ({ ...LAND_UD, kode: a.kode, udfald_verden: VERDEN_UD, udfald_verden_hentet: true }) }));
    const lf = side('#land?kode=DK', LANDV({ land_visning: a => ({ ...LAND_UD, kode: a.kode, serie_fejl: ['par28', 'udfald'] }) }));
    await sov(90);
    const kilde = ld.d.getElementById('land-kilde'), kolL = [...kilde.querySelectorAll('.vg-tre > div')];
    tjek('Trafik og huller, kilderne: overskrift, andele og resten', kolL[0].querySelector('h3').textContent === 'De største kilder til angreb mod Danmark' && kolL[0].textContent.includes('30 %') && kolL[0].textContent.includes('Andre lande (ikke oplyst af Radar)') && kolL[0].textContent.includes('50 %'), kolL[0].textContent);
    tjek('Trafik og huller, kilderne: resten og Radars lande kan ikke klikkes', !kolL[0].querySelector('button'));
    tjek('Trafik og huller, 4 uger: vælgeren og teksten siger det', kilde.querySelector('input:checked').parentElement.textContent === '4 uger' && kilde.textContent.includes('de seneste 4 uger'), kilde.textContent.slice(0, 200));
    const mitQ = new URLSearchParams(kolL[1].querySelector('button').dataset.dyk);
    tjek('Trafik og huller, 4 uger: dine lande åbner dyk med samme 28 døgn', mitQ.get('fra') === '2026-10-06' && mitQ.get('til') === '2026-11-02' && !mitQ.has('periode'), mitQ.toString());
    const seV = ld.d.getElementById('land-udfald').querySelector('button');
    tjek('Trafik og huller, forstyrrelser: tom tekst og link til verden', ld.d.getElementById('land-udfald').textContent.includes('Ingen kendte forstyrrelser i Danmark de seneste 30 dage') && seV.textContent === 'Se forstyrrelser i verden' && seV.dataset.gaa.includes('fane=verden') && seV.dataset.gaa.includes('afsnit=udfald'), seV.dataset.gaa);
    const uv = lv.d.getElementById('land-udfald-verden');
    tjek('Trafik og huller, Verden: forstyrrelserne med land i klarsprog; et kendt land åbner landet, en ukendt kode er tekst', uv.textContent.includes('Sverige') && uv.textContent.includes('Et kabel blev kappet') && uv.querySelectorAll('button').length === 1 && uv.querySelector('button').dataset.gaa.includes('kode=SE'), uv.textContent);
    const rodF = lf.d.getElementById('land-rod');
    tjek('Trafik og huller, en fejl i én serie: note ved afsnittet, intet banner', lf.d.getElementById('land-kilde').textContent.includes('gav ikke tal til dette afsnit') && lf.d.getElementById('land-udfald').textContent.includes('gav ikke tal til dette afsnit') && !lf.d.getElementById('land-maal').textContent.includes('gav ikke tal') && !rodF.querySelector('.vg-advarsel') && !rodF.textContent.includes('er ikke opdateret'), rodF.textContent.slice(0, 160));
    tjek('Trafik og huller, vagten mod origin?location er væk', !/landModEns|\.ens\b/.test(ui('assets/visning-land.js') + ui('assets/sprog.js')) && !fs.readFileSync(path.join(repo, 'src-tauri/src/radar.rs'), 'utf8').split('#[cfg(test)]')[0].includes('origin?location'));

    /* Datovælgeren */
    const P = { fra: '2026-11-01', til: '2026-11-04', dage: 4, med: 4, data_fra: '2026-10-04', data_til: '2026-11-04', i_dag: '2026-11-05' };
    const dv2 = side('#dyk?' + dykQ({ fra: '2026-11-01', til: '2026-11-04' }), DYK({ dyk: () => dykSvar({ periode: P }) }));
    const ro = side('#robotter?zone=z1&navn=wi24rd.com', DYK({ robotter: { ...ROBOT_UD, periode: P } })), lo = side('#login?zone=z1&navn=wi24rd.com', DYK({ login: { ...LOGIN_UD, periode: P }, login_stier: LOGIN_STIER }));
    await sov(120);
    const felter = x => [...x.d.querySelectorAll('.dato-knap')];
    const vaelgDag = async (x, i, dag) => { felter(x)[i].click(); await sov(5); const kal = x.d.querySelector('.kal'); for (let n = 0; n < 24 && !kal.querySelector(`[data-dag="${dag}"]`); n++) { kal.querySelectorAll('.kal-nav')[dag < felter(x)[i].dataset.dag ? 0 : 1].click(); await sov(1); } kal.querySelector(`[data-dag="${dag}"]`).click(); await sov(50); };
    tjek('Datovælger: Fra og Til som tekstknapper i datoformatet; intervallet står i felterne', felter(dv2).map(i => i.textContent).join() === '1. nov. 2026,4. nov. 2026' && felter(dv2).map(i => i.dataset.dag).join() === '2026-11-01,2026-11-04' && felter(dv2)[0].getAttribute('aria-haspopup') === 'dialog', felter(dv2).map(i => i.outerHTML).join());
    tjek('Datovælger: Robotter og Login viser også det aktive udsnit', [ro, lo].every(x => felter(x).map(i => i.dataset.dag).join() === '2026-11-01,2026-11-04'), felter(lo).map(i => i.dataset.dag).join());
    tjek('Datovælger, interval: noten og Nulstil', dv2.d.querySelector('.dato-vaelger').textContent.includes('Hele døgn i UTC. Tidspunkt på dagen gemmes ikke.') && dv2.d.querySelector('.dato-vaelger button.link').textContent === 'Nulstil' && dv2.d.querySelector('.dato-vaelger button.link').getAttribute('aria-label') === 'Nulstil til hele historikken');
    tjek('Datovælger, interval: kun 7 og 30 dage i vælgeren, "Hele historikken" er væk', [ro, lo, dv2].every(x => ![...x.d.querySelectorAll('.segment')].some(s => s.textContent.includes('Hele historikken'))));
    const chip = [...dv2.d.querySelectorAll('#dyk-filtre button')].map(b => b.textContent);
    tjek('Datovælger, interval: intervallet får en chip', chip.join() === '1.–4. november×', chip.join());
    const antal = dv2.kald.filter(k => k[0] === 'dyk').length, fejlT = dv2.d.querySelector('.dato-fejl');
    await vaelgDag(dv2, 1, '2026-10-30');
    tjek('Datovælger, interval: fra efter til giver en fejltekst og ingen hentning', !fejlT.hidden && fejlT.textContent === '»Fra« skal ligge før eller på »Til«.' && dv2.kald.filter(k => k[0] === 'dyk').length === antal && fejlT.getAttribute('role') === 'alert');
    await vaelgDag(dv2, 1, '2026-11-03');
    const sidst = dv2.kald.filter(k => k[0] === 'dyk').at(-1)[1].periode;
    tjek('Datovælger, interval: et gyldigt interval er ét trin i adressen og hentes', dv2.w.location.hash.includes('fra=2026-11-01') && dv2.w.location.hash.includes('til=2026-11-03') && JSON.stringify(sidst) === '{"spaend":{"fra":"2026-11-01","til":"2026-11-03"}}', JSON.stringify(sidst));
    dv2.d.querySelector('.dato-vaelger button.link').click(); await sov(80);
    tjek('Datovælger, interval: Nulstil er hele historikken', dv2.kald.filter(k => k[0] === 'dyk').at(-1)[1].periode === 'alt' && dv2.w.location.hash.includes('periode=alt') && !dv2.w.location.hash.includes('fra='), dv2.w.location.hash);
    const ri = side('#robotter?zone=z1&navn=wi24rd.com&fra=2026-11-01&til=2026-11-04', DYK({ robotter: { ...ROBOT_UD, periode: P } })), li2 = side('#login?zone=z1&navn=wi24rd.com&fra=2026-11-01&til=2026-11-04', DYK({ login: { ...LOGIN_UD, periode: P }, login_stier: LOGIN_STIER }));
    await sov(120);
    tjek('Datovælger, interval: Robotter og Login sender intervallet og har en chip', [[ri, 'robotter'], [li2, 'login']].every(([x, c]) => JSON.stringify(x.kald.find(k => k[0] === c)[1].periode) === '{"spaend":{"fra":"2026-11-01","til":"2026-11-04"}}' && [...x.d.querySelectorAll('.sti-chip')].some(b => b.textContent === '1.–4. november ×')));
    const rq = new URLSearchParams(ri.d.querySelector('#robotter-rod [data-dyk]').dataset.dyk);
    tjek('Datovælger, interval: et klik i Robotter tager intervallet med til dyk', rq.get('fra') === '2026-11-01' && rq.get('til') === '2026-11-04', rq.toString());
    tjek('Trafik og huller, sorte ravne: --color-text, som følger temaet; båndet og mærker er uændrede', /\.tom-ravn \{[^}]*background: var\(--color-text\)/.test(ui('assets/app.css')) && !/\.tom-ravn \{[^}]*--color-logo/.test(ui('assets/app.css')) && /--color-logo/.test(ui('assets/tokens.css')));

    /* Dit sites teknologi */
    const TEK = { linjer: [
      { tek: 'wordpress', angreb30: 40, forsoeg30: 312, foerst: '2026-10-04', sidst: '2026-11-02', valg: 'ved_ikke', besked: false, forslag: true },
      { tek: 'react', angreb30: 9, forsoeg30: 0, foerst: '2026-10-05', sidst: '2026-11-01', valg: 'bruger', besked: true, forslag: false },
      { tek: 'php', angreb30: 0, forsoeg30: 0, foerst: null, sidst: null, valg: 'bruger_ikke', besked: false, forslag: false },
    ], generiske: [{ id: 'hemmeligheder', forsoeg30: 4 }] };
    const BV = vaernUd(), TV = (x = {}) => DYK({ vaern_cache: BV, vaern_oversigt: BV, koe_prioritet: null, teknologi_hent: TEK, teknologi_saet: null, teknologi_alle: 3, ...x });
    const tv = side('#vaernet?zone=z1&navn=wi24rd.com', TV()), sa = side('#vaernet?samlet=1', TV({ vaern_samlet_cache: { vaern: BV, valgte: [{ id: 'z1', navn: 'a', med_tal: true }], domaener: 1, med_tal: 1 } }));
    await sov(120);
    const sek = tv.d.getElementById('vg-teknologi');
    tjek('Trafik og huller, teknologi: afsnittet står efter regelkortene, kun for ét domæne', sek && sek.querySelector('h2').textContent === 'Dit sites teknologi' && sek.previousElementSibling.id === 'vg-regler' && !sa.d.getElementById('vg-teknologi'));
    tjek('Trafik og huller, teknologi: generiske slags har intet valg og kan åbnes', sek.textContent.includes('Altid relevant') && !sek.querySelector('.tek-generiske input') && new URLSearchParams(sek.querySelector('.tek-generiske button').dataset.dyk).get('forsoeg') === 'hemmeligheder');
    tjek('Dit sites teknologi: forslaget er én linje, og intet er sat', sek.querySelector('.tek-forslag').textContent.startsWith('Alle 312 WordPress-forsøg fik 404 — sæt til »Bruger ikke«?') && !tv.kald.some(k => k[0] === 'teknologi_saet'), sek.textContent);
    sek.querySelector('.tek-forslag button').click(); await sov(60);
    tjek('Trafik og huller, forslag: først et klik sætter det', JSON.stringify(tv.kald.filter(k => k[0] === 'teknologi_saet').at(-1)[1]) === '{"zone":"z1","tek":"wordpress","valg":"bruger_ikke","besked":false}', JSON.stringify(tv.kald.at(-1)));
    tjek('Teknologinavne: katalogets navne står i sprog.js som i Rust', (r => [...r.slice(r.indexOf('pub const KATALOG'), r.indexOf('pub const GENERISKE_REGLER')).matchAll(/\(\s*"([a-z0-9 ]+)",\s*"([^"]+)",\s*"[a-z]+",\s*&\[/g)].map(m => `${m[1]}=${m[2]}`).sort().join())(fs.readFileSync(path.join(repo, 'src-tauri/src/teknologi.rs'), 'utf8')) === Object.entries(tv.w.hmSprog.T.tek).map(([k, v]) => `${k}=${v}`).sort().join());
    const dt = side('#dyk?' + dykQ({ teknologi: 'wordpress', slags: 'angreb' }), DYK());
    await sov(90);
    tjek('Trafik og huller, dyk på en teknologi: filter, chip og overskrift', dt.kald.find(k => k[0] === 'dyk')[1].filtre[0].dim === 'teknologi' && dt.d.getElementById('dyk-h').textContent === 'Angreb på wi24rd.com mod WordPress' && dt.d.querySelector('#dyk-filtre button').textContent.includes('WordPress'), dt.d.getElementById('dyk-h').textContent);
    const top = [{ navn: 'Wordpress', n: 47, tek: 'wordpress' }], fordel = { blokeret: 47, udfordret: 0, labyrint: 0, andet: 0 };
    const ikke = raekke('i1', 'ikke.wi24rd.com', 'groen', { angreb_i_gaar: 47, angreb_snit: 40, angreb_i_gaar_top: top, angreb_fordeling: fordel, ikke_brugt: [{ tek: 'wordpress', n: 47 }], angreb_status: 'groen', sidst_hentet: NU_ISO });
    const uv2 = raekke('u1', 'uventet.wi24rd.com', 'gul', { uventet: { tek: 'wordpress', sti: '/wp-login.php', n: 2 }, angreb_status: 'groen', sidst_hentet: NU_ISO });
    const na = raekke('n1', 'naaet.wi24rd.com', 'roed', { angreb_i_gaar: 4, angreb_snit: 1, angreb_i_gaar_top: top, angreb_fordeling: { ...fordel, blokeret: 0, andet: 4 }, naaet: { tek: 'wordpress', n: 4 }, sidst_hentet: NU_ISO });
    const sp = raekke('s1', 'spoerg.wi24rd.com', 'gul', { angreb_i_gaar: 47, angreb_snit: 3, angreb_i_gaar_top: top, angreb_fordeling: fordel, tek_spoerg: 'wordpress', angreb_status: 'gul', sidst_hentet: NU_ISO });
    const ovK = side('#overblik', DYK({ overblik: [ikke, uv2, na, sp] }));
    await sov(90);
    const hv2 = id => ovK.d.querySelector(`tr[data-zone="${id}"] .hv-tekst`);
    tjek('Trafik og huller, Hvorfor: "Bruger vi ikke" er ordret som notify::aarsag, uden råd', hv2('i1').textContent === '47 angreb i går (snit 40) – alle stoppet. Mest angreb på WordPress (47). 47 angreb på WordPress, som du ikke bruger — ufarlige for dig.', hv2('i1').textContent);
    tjek('Trafik og huller, Hvorfor: tallene står fuldt ved "Bruger vi ikke"', hv2('i1').closest('tr').querySelector('.chip').className.includes('chip-groen') && hv2('i1').textContent.startsWith('47 angreb'));
    tjek('Trafik og huller, Hvorfor: sikkerhedsnettet', hv2('u1').textContent === 'Ingen angreb i går. Uventet: din server svarede på en WordPress-sti. Bruger du alligevel WordPress?', hv2('u1').textContent);
    tjek('Trafik og huller, Hvorfor: nåede din server', hv2('n1').textContent.endsWith(' 4 angreb på WordPress nåede din server, og du har sagt, at du bruger WordPress.'), hv2('n1').textContent);
    const lk = hv2('s1').closest('td').querySelector('.hv-spoerg');
    tjek('Trafik og huller, Hvorfor: linket "Bruger du ikke …?" fører til #teknologi', lk && lk.textContent === 'Bruger du ikke WordPress? Sig det her' && lk.dataset.gaa.startsWith('teknologi?') && !hv2('i1').closest('td').querySelector('.hv-spoerg'), lk && lk.outerHTML);
    const tvv = side('#vaernet?zone=z1&navn=wi24rd.com&vis=teknologi', TV()); let rullet = false;
    tvv.w.HTMLElement.prototype.scrollIntoView = function () { if (this.id === 'vg-teknologi') rullet = true; };
    await sov(120);
    tjek('Trafik og huller, linket ruller til afsnittet', rullet);

    /* Huller */
    const H = { linjer: [{ noegle: '2026-09-02_2026-10-04', fra: '2026-09-02', til: '2026-10-04', tabt: 9 }, { noegle: '2026-08-01_2026-08-05', fra: '2026-08-01', til: '2026-08-05', tabt: 0 }], dage: ['2026-09-03', '2026-09-04', '2026-09-10'], aeldste: '2026-08-01' };
    const hv = { ...DYK({ overblik: [raekke('h1', 'hul.wi24rd.com', 'groen', { hele7: 12, hele30: 40, med7: 6, med30: 27, sidst_hentet: NU_ISO }), raekke('h2', 'tom.wi24rd.com', 'groen', { hele7: 0, hele30: 3, med7: 0, med30: 30, sidst_hentet: NU_ISO })], huller: H, hul_luk: null }), siden_sidst: { vis: false, nye: [], raekker: [] } };
    const oh = side('#overblik', hv), sh = side('#siden-sidst', hv);
    await sov(100);
    const linjer = x => [...x.querySelectorAll('.hul-linje')];
    tjek('Trafik og huller, hul-linjen i Overblik', linjer(oh.d.getElementById('hul-linjer'))[0].textContent.startsWith('Huginn var ikke tændt fra 2. september til 4. oktober. Cloudflare gemmer kun en måned, så 9 dage mangler for altid i historikken.'), linjer(oh.d.getElementById('hul-linjer'))[0]?.textContent);
    tjek('Trafik og huller, hul-linjen uden tabte dage', linjer(oh.d.getElementById('hul-linjer'))[1].textContent.startsWith('Huginn var ikke tændt fra 1. august til 5. august. Cloudflare havde stadig tallene, så intet mangler.'));
    tjek('Trafik og huller, hul-linjen i Siden sidst', linjer(sh.d.getElementById('siden-huller')).length === 2);
    linjer(oh.d.getElementById('hul-linjer'))[0].querySelector('button').click(); await sov(50);
    const tilbage = linjer(oh.d.getElementById('hul-linjer'));
    tjek('Trafik og huller, hul-linjen kan lukkes og huskes pr. hul', oh.kald.some(k => k[0] === 'hul_luk' && k[1].noegle === '2026-09-02_2026-10-04') && tilbage.length === 1 && tilbage[0].textContent.includes('1. august'));
    const c = id => [...oh.d.querySelectorAll(`tr[data-zone="${id}"] .tal-kol`)];
    tjek('Trafik og huller, Overblik: summer med huller siger "N af M dage"', c('h1')[1].textContent.includes('6 af 7 dage') && c('h1')[2].textContent.includes('27 af 30 dage') && !c('h2')[2].textContent.includes('af 30'), c('h1')[2].textContent);
    tjek('Trafik og huller, Overblik: uden hentede døgn er tallet "–", ikke 0', c('h2')[1].textContent === '–', c('h2')[1].textContent);
    const HD = vaernUd(), huller = [HD.h_dage[HD.h_dage.length - 2].dag, HD.h_dage[0].dag], hvv = { ...HD, uhentet: huller };
    const ah = side('#vaernet?zone=z1&navn=wi24rd.com', DYK({ vaern_cache: hvv, vaern_oversigt: hvv, koe_prioritet: null }));
    await sov(120);
    tjek('Trafik og huller, Angreb: et hul er markeret og kan ikke åbnes, aldrig som 0', ah.d.querySelectorAll('#vagt .dyk-uhentet').length === 2 && !ah.d.querySelector('#vagt .dyk-uhentet[data-dyk]') && ah.d.querySelector('#vagt .dyk-uhentet title').textContent.endsWith('ingen data — Huginn var slukket'), ah.d.querySelector('#vagt .dyk-uhentet title')?.textContent);
    tjek('Trafik og huller, Angreb: fliserne siger "N af M dage"', ah.d.querySelector('#vagt').textContent.includes('6 af 7 dage') && ah.d.querySelector('#vagt').textContent.includes('28 af 30 dage'));
    const hr = [...ah.d.querySelectorAll('#vagt .hul-raekke')];
    tjek('Trafik og huller, Angreb: tabellen skriver "ingen data", ikke 0, og uden knapper', hr.length === 2 && hr[0].querySelectorAll('td')[0].textContent === 'ingen data' && !hr[0].querySelector('button'));
    const dS = dykSvar({ periode: { ...P, dage: 30, med: 27 }, noegletal: { i_gaar: 6, i_dag_hidtil: 2, hele7: 57, hele30: 198, alt: 240, med7: 6, med30: 27 } }), dh = side('#dyk?' + dykQ(), DYK({ dyk: () => dS }));
    await sov(90);
    const dtxt = dh.d.getElementById('dyk-rod').textContent;
    tjek('Trafik og huller, dyk: fliserne og noten siger "N af M dage"', dtxt.includes('27 af 30 dage') && dtxt.includes('6 af 7 dage') && dtxt.includes('Tallene dækker 27 af 30 dage'), dtxt.slice(0, 200));
    const rh = side('#robotter?zone=z1&navn=wi24rd.com', DYK({ robotter: { ...ROBOT_UD, periode: { ...P, dage: 7, med: 6 } } })), lh = side('#login?zone=z1&navn=wi24rd.com', DYK({ login: { ...LOGIN_UD, periode: { ...P, dage: 30, med: 28 } }, login_stier: LOGIN_STIER }));
    const land = side('#land?kode=DK', LANDV({ land_visning: a => ({ ...LAND_UD, kode: a.kode, dage: [{ dag: '2026-11-01', mit: null, mit_indeks: null, verden_indeks: 90 }, { dag: '2026-11-02', mit: 0, mit_indeks: 0, verden_indeks: 110 }] }) }));
    await sov(120);
    tjek('Trafik og huller, Robotter og Login: noten', rh.d.getElementById('robotter-rod').textContent.includes('Tallene dækker 6 af 7 dage') && lh.d.getElementById('login-rod').textContent.includes('Tallene dækker 28 af 30 dage'));
    const titler = [...land.d.querySelectorAll('.dyk-soejle title')].map(t => t.textContent);
    tjek('Trafik og huller, kurven: et døgn uden tal er "ingen data", og 0 er 0', titler[0].includes('ingen data') && titler[1].includes('mine domæner 0'), titler.join(' | '));
    const ih = side('#indstillinger', { ...DYK({ huller: H, data_status: { bytes: 1000, foerste: '2026-08-01', sidste: '2026-11-02', domaener: 2 }, autostart_hent: false }) }), ij = side('#indstillinger', { ...DYK({ huller: { linjer: [], dage: [], aeldste: '2026-08-01' }, data_status: { bytes: 1000, foerste: '2026-08-01', sidste: '2026-11-02', domaener: 2 }, autostart_hent: true, autostart_saet: false }) });
    await sov(120);
    tjek('Trafik og huller, Indstillinger: Start ved login fra giver teksten i banneret', !ih.d.getElementById('autostart-banner').hidden && ih.d.getElementById('autostart-banner').textContent.includes('Så henter Huginn kun, når programmet er åbent. Åbn det mindst hver tredje uge, ellers kommer der huller i historikken.') && ih.d.getElementById('autostart-hul').textContent === 'Slået fra: der kan komme huller i historikken.' && ij.d.getElementById('autostart-hul').hidden && ij.d.getElementById('autostart-banner').hidden);
    const au = ij.d.getElementById('autostart');
    au.checked = false; au.dispatchEvent(new ij.w.Event('change', { bubbles: true })); await sov(60);
    tjek('Trafik og huller, Indstillinger: teksten står, så snart det er slået fra', !ij.d.getElementById('autostart-hul').hidden);
    const bh = ih.d.getElementById('data-huller');
    tjek('Trafik og huller, Data: Ældste dag og Huller', ih.d.getElementById('data-aeldste').textContent === '1. aug. 2026' && bh.textContent === '3 dage' && !bh.disabled, bh.textContent);
    bh.click(); await sov(40);
    const liste = [...ih.d.querySelectorAll('#hjaelp-guide li')].map(l => l.textContent);
    tjek('Trafik og huller, Data: Huller åbner listen med dage i træk samlet', liste.length === 2 && liste[0].includes('3. sep.') && liste[0].includes('4. sep.'), liste.join(' | '));
    tjek('Trafik og huller, Data: uden huller er der intet at klikke på', ij.d.getElementById('data-huller').disabled && ij.d.getElementById('data-huller').textContent === '0');
  }

  if (!CHROME) console.log('SPRUNGET OVER K7 kontrast: Chrome findes ikke (sæt HM_CHROME)');
  else {
    const mappe = fs.mkdtempSync(path.join(os.tmpdir(), 'hm-kontrast-'));
    fs.cpSync(path.join(repo, 'ui'), path.join(mappe, 'ui'), { recursive: true });
    /* Lagene under hvert tekststykke lægges sammen bagfra (farve, gradient, gennemsigtighed), i tre punkter, og det værste tæller.
       Undtaget: --color-text-disabled og deaktiverede kontroller. Tekst står aldrig i SVG (vg-tegn.js), så color er altid tekstens farve. */
    /* WCAG AAA: stor tekst (mindst 24 px, eller 18,66 px fed) skal have 4,5:1; al anden tekst 7:1. */
    const stor = (px, vaegt) => px >= 24 || (px >= 18.66 && vaegt >= 700);
    tjek('K7 stor tekst: 24 px, eller 18,66 px fed; 17 px fed og 23 px normal er ikke stor', stor(24, 400) && stor(18.66, 700) && stor(19, 700) && !stor(17, 700) && !stor(23.9, 400) && !stor(19, 600) && !stor(18.6, 700));
    const maal = String.raw`(async () => {
      const stor = ${stor}, baand = [];
      const sov = ms => new Promise(r => setTimeout(r, ms)), H = window.__HM_MAAL__;
      const farve = c => { const m = /^color\(srgb ([-\d.e]+) ([-\d.e]+) ([-\d.e]+)(?: \/ ([\d.e-]+))?\)/.exec(c); if (m) return [m[1] * 255, m[2] * 255, m[3] * 255, m[4] === undefined ? 1 : +m[4]]; const t = c.match(/[\d.]+/g).map(Number); return [t[0], t[1], t[2], t[3] === undefined ? 1 : t[3]]; };
      const over = (f, b) => [0, 1, 2].map(i => f[i] * f[3] + b[i] * (1 - f[3])).concat(1);
      const lum = c => { const [r, g, b] = c.slice(0, 3).map(v => { v /= 255; return v <= .03928 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4; }); return .2126 * r + .7152 * g + .0722 * b; };
      const kontrast = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((m, n) => n - m); return (x + .05) / (y + .05); };
      const opacitet = e => { let o = 1; for (let x = e; x; x = x.parentElement) o *= parseFloat(getComputedStyle(x).opacity); return o; };
      function gradient(e, billede, x, y) {
        const g = /linear-gradient\((?:([\d.]+)deg, )?(.*)\)$/.exec(billede); if (!g) return null;
        const stop = [...g[2].matchAll(/(rgba?\([^)]*\)|color\([^)]*\)) ([\d.]+)%/g)].map(m => [farve(m[1]), +m[2] / 100]); if (stop.length < 2) return null;
        const r = e.getBoundingClientRect(), a = (g[1] === undefined ? 180 : +g[1]) * Math.PI / 180, dx = Math.sin(a), dy = -Math.cos(a), L = Math.abs(r.width * dx) + Math.abs(r.height * dy);
        const t = ((x - r.left - r.width / 2) * dx + (y - r.top - r.height / 2) * dy) / L + .5;
        for (let i = 1; i < stop.length; i++) if (t <= stop[i][1]) { const [c0, p0] = stop[i - 1], [c1, p1] = stop[i], k = Math.max(0, (t - p0) / (p1 - p0 || 1)); return c0.map((v, j) => v + (c1[j] - v) * k); }
        return stop.at(-1)[0];
      }
      /* Båndets bølger er SVG-stier, ikke baggrunde: de lægges til, hvor punktet ligger inde i dem (øverste først). */
      function boelger(band, x, y) {
        const svg = band.querySelector('.band-boelge'), r = svg.getBoundingClientRect(), pt = svg.createSVGPoint();
        pt.x = (x - r.left) * 1200 / r.width; pt.y = (y - r.top) * 60 / r.height;
        return [...svg.querySelectorAll('path')].reverse().filter(s => s.isPointInFill(pt)).map(s => farve(getComputedStyle(s).fill));
      }
      function bag(p, x, y) {
        let stak = document.elementsFromPoint(x, y); const i = stak.findIndex(e => e === p || e.contains(p) || p.contains(e)); if (i > 0) stak = stak.slice(i);
        const lag = [];
        for (const e of stak) {
          const cs = getComputedStyle(e), o = opacitet(e), g = cs.backgroundImage !== 'none' ? gradient(e, cs.backgroundImage, x, y) : null, c = farve(cs.backgroundColor);
          if (e.matches('.band')) lag.push(...boelger(e, x, y));
          if (g) lag.push([g[0], g[1], g[2], g[3] * o]);
          if (c[3] > 0) lag.push([c[0], c[1], c[2], c[3] * o]);
          if (lag.length && lag.at(-1)[3] >= 1) break;
        }
        return lag.reverse().reduce((b, f) => over(f, b), [255, 255, 255, 1]);
      }
      const prove = document.createElement('span'); prove.style.color = 'var(--color-text-disabled)'; document.body.append(prove); const graa = getComputedStyle(prove).color; prove.remove();
      const hex = c => '#' + c.slice(0, 3).map(v => Math.round(v).toString(16).padStart(2, '0')).join('');
      const navn = e => e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') + (typeof e.className === 'string' && e.className ? '.' + e.className.trim().split(/\s+/).join('.') : '');
      function mal(rod, hvad, ud) {
        const v = { hvad, n: 0, undtaget: 0, klasser: [] }, kl = new Set();
        const et = (p, felter, tekst, cf) => {
          const cs = getComputedStyle(p), f = farve(cf || cs.color);
          if ((cf || cs.color) === graa) { v.undtaget++; return; }
          f[3] *= opacitet(p);
          let min = 99, b0 = null;
          const r = felter[0], y = r.top + r.height / 2;
          for (const x of [r.left + 1, r.left + r.width / 2, r.right - 1]) { const b = bag(p, x, y), k = kontrast(over(f, b), b); if (k < min) { min = k; b0 = b; } }
          v.n++; for (let x = p; x; x = x.parentElement) for (const c of x.classList) kl.add(c);
          const erStor = stor(parseFloat(cs.fontSize), parseInt(cs.fontWeight, 10));
          if (p.closest('.band-navn')) baand.push({ e: p.tagName, k: +min.toFixed(2), stor: erStor, px: cs.fontSize });
          if (min < (erStor ? 4.5 : 7)) ud.push({ hvad, k: +min.toFixed(2), fg: hex(over(f, b0)), bg: hex(b0), e: navn(p), t: tekst.slice(0, 40) });
        };
        const felter = g => [...g.getClientRects()].filter(r => r.width > 1 && r.height > 1);
        const w = document.createTreeWalker(rod, NodeFilter.SHOW_TEXT);
        for (let n = w.nextNode(); n; n = w.nextNode()) {
          const p = n.parentElement, t = n.textContent.trim();
          if (!t || !p || p.closest('script, style, select, .sr-only, noscript, title, [hidden]') || !p.checkVisibility({ visibilityProperty: true })) continue;
          if (p.closest(':disabled, [aria-disabled="true"]')) { v.undtaget++; continue; }
          const g = document.createRange(); g.selectNodeContents(n);
          let r = felter(g); if (!r.length) continue;
          if (r[0].top < 0 || r[0].bottom > innerHeight) { p.scrollIntoView({ block: 'center' }); r = felter(g); if (!r.length) continue; }
          if (r[0].left < 0 || r[0].right > innerWidth || r[0].top < 0 || r[0].bottom > innerHeight) continue;
          et(p, r, t);
        }
        for (const e of rod.querySelectorAll('input, select, textarea')) {
          if (!e.checkVisibility({ visibilityProperty: true }) || ['checkbox', 'radio', 'hidden'].includes(e.type) || e.closest('.sr-only')) continue;
          if (e.disabled) { v.undtaget++; continue; }
          e.scrollIntoView({ block: 'center' });
          const r = e.getBoundingClientRect(); if (!r.width) continue;
          if (e.tagName !== 'SELECT' && !e.value && e.placeholder) et(e, [r], 'pladsholder: ' + e.placeholder, getComputedStyle(e, '::placeholder').color);
          else et(e, [r], String(e.value || e.tagName));
        }
        v.klasser = [...kl]; return v;
      }
      await sov(1200);
      const ud = [], sider = [];
      for (const h of H) {
        if (h.hash !== undefined) { location.hash = h.hash; await sov(900); }
        if (h.aabn) { for (const s of h.aabn) { const e = document.querySelector(s); if (e) { e.click(); await sov(300); } } }
        const rod = h.rod ? document.querySelector(h.rod) : document.body;
        sider.push(mal(rod, h.navn, ud));
        if (h.luk) { const e = document.querySelector(h.luk); if (e) { e.click(); await sov(200); } }
      }
      /* Kontrolprøve: samme farve (5,55:1) skal fejle som lille tekst og bestå som stor, ellers måler testen ikke det, den siger. */
      const proeve = document.createElement('div'), kontrol = [];
      proeve.style.cssText = 'position:fixed;top:0;left:0;z-index:9999;padding:20px;background:#fff;color:#56698A';
      for (const [navn, css] of [['lille', 'font:400 14px sans-serif'], ['fed', 'font:700 19px sans-serif'], ['stor', 'font:400 24px sans-serif']]) { const e = document.createElement('p'); e.textContent = navn; e.style.cssText = css; proeve.append(e); }
      document.body.append(proeve);
      mal(proeve, 'kontrolprøve', kontrol);
      document.body.textContent = 'MAALT:' + JSON.stringify({ sider, ud, baand, kontrol: kontrol.map(k => k.t) }) + ':SLUT';
    })();`;
    const zoner = OVERBLIK.map(r => ({ id: r.id, navn: r.navn })), vaern = vaernUd();
    const svar = {
      ...B2(), valg_hent: zoner.map(z => z.id), domaener_hent: zoner.map((z, i) => ({ ...z, til: true, ny: i === 2 })), vaern_samlet_cache: { vaern, valgte: zoner.map(z => ({ ...z, med_tal: true })), domaener: zoner.length, med_tal: zoner.length },
      siden_sidst: { vis: true, nye: [], raekker: SIDEN_RK }, dyk: dykSvar({ regel: REGEL_HOVED }), robotter: ROBOT_UD, login: LOGIN_UD, login_stier: LOGIN_STIER, land_hent: { land: 'DK', forslag: 'US' }, land_visning: { ...LAND_UD, udfald_verden: B2C_VERDEN, udfald_verden_hentet: true }, trafik_dag: B2C_TRAFIK, teknologi_hent: B2C_TEK, huller: B2C_HULLER,
    };
    Object.assign(svar, { teknologi_oversigt: { zoner: { z1: 2, z2: 1 }, valg: true }, teknologi_side: { domaener: [{ zone: 'z1', navn: 'wi24rd.com', valg: { php: ['bruger', true], wordpress: ['bruger_ikke', false] }, antal: { php: 204, wordpress: 12 } }, { zone: 'z2', navn: 'xyz.wi24rd.com', valg: {}, antal: { php: 3 } }], katalog: [{ tek: 'php', navn: 'PHP', gruppe: 'server', angreb: 207, set: true, vis: true, ny: false }, { tek: 'wordpress', navn: 'WordPress', gruppe: 'cms', angreb: 12, set: true, vis: true, ny: true }, { tek: 'magento', navn: 'Magento', gruppe: 'cms', angreb: 0, set: false, vis: false, ny: false }] }, teknologi_hent: { linjer: [{ tek: 'php', angreb30: 3, forsoeg30: 40, foerst: '2026-10-05', sidst: '2026-11-02', valg: 'bruger', besked: true, forslag: false }, { tek: 'wordpress', angreb30: 12, forsoeg30: 0, foerst: '2026-10-05', sidst: '2026-11-02', valg: 'ved_ikke', besked: false, forslag: true }], generiske: [] } });
    const VURD7 = { dag: '2026-11-02', maal_ja: false, foreloebig: false, rekog_ja: false, maal: [{ id: 'stigning', ok: true, n: 40, m: 2, navn: null, domaener: [] }, { id: 'net_andre', ok: false, n: 9, m: 30, navn: 'Mal AS', domaener: [{ zone: 'z2', navn: 'xyz.wi24rd.com', n: 9 }] }, { id: 'radar', ok: null, n: 0, m: 1.3, navn: null, domaener: [] }], rekog: [{ id: 'stier', ok: false, n: 12, m: 100, navn: null, domaener: [] }] };
    Object.assign(svar, { vurdering: VURD7 });
    const dom = 'zone=z1&navn=wi24rd.com';
    const SIDER = [
      { navn: 'Overblik', hash: 'overblik' }, { navn: 'Siden sidst', hash: 'siden-sidst' }, { navn: 'Angreb samlet', hash: 'vaernet?samlet=1' }, { navn: 'Angreb', hash: 'vaernet?' + dom }, { navn: 'dyk', hash: 'dyk?' + dom }, { navn: 'dyk i en regel', hash: `dyk?${dom}&regel=${encodeURIComponent(REACT)}` }, { navn: 'dyk med to filtre', hash: `dyk?${dom}&land=FR&dag=2026-11-03` },
      { navn: 'Robotter', hash: 'robotter?' + dom }, { navn: 'Login', hash: 'login?' + dom, aabn: ['details.login-ret summary'] }, { navn: 'Danmark lige nu', hash: 'land' }, { navn: 'Verden', hash: 'land?fane=verden' }, { navn: 'dyk i et land', hash: 'dyk?' + dom + '&land=PL' }, { navn: 'Indstillinger', hash: 'indstillinger', aabn: ['#skift-noegle'] },
      { navn: 'trafik-dyk', hash: `dyk?${dom}&dag=2026-11-02&trafik=1` }, { navn: 'hjælpen', hash: 'overblik', aabn: ['#hjaelp-knap'], rod: '#hjaelp', luk: '#hjaelp-luk' }, { navn: 'bekræftelsen', hash: 'indstillinger', aabn: ['#glem'], rod: '#glem-dialog' },
    ];
    const koer = (navn, tema, ekstra, sider, bredde = 1400, tekst = 'normal') => {
      const stub = `<script>localStorage.setItem('hm-sprog','da');localStorage.setItem('hm-tekst',${JSON.stringify(tekst)});localStorage.setItem('hm-tema',${JSON.stringify(tema)});window.__HM_PLATFORM__='macos';window.__HM_MAAL__=${JSON.stringify(sider)};const SVAR=${JSON.stringify({ ...svar, ...ekstra })};window.__TAURI__={core:{invoke:async c=>(c in SVAR?JSON.parse(JSON.stringify(SVAR[c])):null)},event:{listen:async()=>()=>{}},app:{getVersion:async()=>'9.8.7'}};</script>`;
      const html = ui('index.html').replace('<script src="assets/tema.js"></script>', UR_STUB + stub + '<script src="assets/tema.js"></script>').replace('</body>', `<script>addEventListener('load',()=>{${maal}});</script></body>`);
      const fil = path.join(mappe, 'ui', `kontrast-${navn}-${tema}.html`);
      fs.writeFileSync(fil, html);
      const p = spawnSync(CHROME, ['--headless=new', '--disable-gpu', '--hide-scrollbars', `--user-data-dir=${path.join(mappe, `p-${navn}-${tema}`)}`, `--window-size=${bredde},1600`, '--virtual-time-budget=40000', '--dump-dom', pathToFileURL(fil).href], { encoding: 'utf8', timeout: 120000, maxBuffer: 1 << 26 });
      const m = /MAALT:(.*):SLUT/s.exec(p.stdout || '');
      return m ? JSON.parse(m[1].replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&')) : null;
    };
    const alleFra = { domaener_hent: zoner.map(z => ({ ...z, til: false, ny: false })), overblik: [], valg_hent: [] };
    for (const tema of ['light', 'dark']) {
      const REK7 = { slags: 'rekognoscering', dag: '2026-11-02', stier: 240, normalt: 12, scanner: true, ny: true, netvaerk: null, procent: 0, verden: false, tek: null }, MAAL7 = { ...REK7, slags: 'maalrettet', netvaerk: 'Mal AS', procent: 75, verden: true };
      const B3RK = [{ ...OVERBLIK[0], moenster: MAAL7 }, { ...OVERBLIK[1], moenster: REK7 }];
      const B3 = koer('b3', tema, { autostart_hent: false, overblik: B3RK, siden_sidst: { vis: false, nye: [], raekker: B3RK, nye_robotter: [] }, dyk: dykSvar({ trafik: { i_alt: 809000, dage: [{ dag: '2026-10-28', n: 410000, median: 1500, stoppet: 19 }] }, kun_i_dag: { stier: [{ v: '/x', n: 1 }], lande: [], t: null } }) }, [{ navn: 'Overblik med mærker', hash: 'overblik' }, { navn: 'Siden sidst med mærker', hash: 'siden-sidst' }, { navn: 'Angreb med mærke', hash: 'vaernet?' + dom }, { navn: 'Start ved login fra', hash: 'indstillinger' }, { navn: 'dyk med gul linje, boks og kalender', hash: 'dyk?' + dom, aabn: ['.dato-knap'] }]);
      const KAL = koer('kal900', tema, {}, [{ navn: 'kalenderen ved 900 px og Stor tekst', hash: 'dyk?' + dom, aabn: ['.dato-knap'] }], 900, 'large');
      const FIX3 = koer('fix3', tema, { overblik: [{ ...OVERBLIK[0], tilstand: [{ slags: 'pause', status: null, kl: null }], laek: { dag: '2026-11-02', n: 3, naaet: 1 } }, OVERBLIK[1]], vurdering: VURD7, login: { ...LOGIN_UD, laek: [{ dag: '2026-11-02', sti: '/login', n: 3, naaet: 1 }], laek_andet: [{ v: 'username_leaked', n: 2 }], laek_dage: 30 }, dyk: dykSvar({ periode: { fra: '2026-01-01', til: '2026-11-02', dage: 306, med: 30, data_fra: '2026-01-01', data_til: '2026-11-02', i_dag: '2026-11-03', sikkerhed_fra: '2026-10-04', trafik_fra: '2026-01-01' }, trafik_serie: [{ dag: '2026-01-01', n: 40, hentet: true }, { dag: '2026-01-02', n: 0, hentet: false }] }) }, [{ navn: 'Overblik med tilstand og læk', hash: 'overblik' }, { navn: 'Angreb med Vurdering', hash: 'vaernet?' + dom, aabn: ['details.vurdering summary'] }, { navn: 'Login med læk', hash: 'login?' + dom }, { navn: 'dyk med året', hash: 'dyk?' + dom + '&periode=alt' }]);
      /* DMARC-afsnittet og den åbne kolonnemenu. */
      const TEK5 = { domaener: [{ zone: 'z1', navn: 'eksempel.wi24rd.com', valg: { react: ['bruger', false] }, antal: { react: 4 } }], katalog: [{ tek: 'react', navn: 'React', gruppe: 'ramme', angreb: 4, set: true, vis: true, ny: false }] };
      const FIX5 = koer('fix5', tema, { email_oversigt: { mails: 1200, fejlet: 41, afsendere: 2, hentede_dage: 30, fejl: null, liste: [{ afsender: 'Ond Afsender', land: 'RU', mails: 40, fejlet: 40 }, { afsender: '', land: '', mails: 9, fejlet: 1 }] }, teknologi_side: TEK5 }, [{ navn: 'Angreb med DMARC', hash: 'vaernet?' + dom }, { navn: '#teknologi med åben kolonnemenu', hash: 'teknologi', aabn: ['.tek-kol-knap'] }]);
      const r = [koer('alt', tema, {}, SIDER), B3, FIX3, FIX5, KAL, koer('serveren', tema, { overblik: B2B_OVERBLIK, vaern_cache: { ...vaern, trafik: B2B_TRAFIK }, vaern_oversigt: { ...vaern, trafik: B2B_TRAFIK } }, [{ navn: 'Overblik med server og trafik', hash: 'overblik' }, { navn: 'Angreb med trafik', hash: 'vaernet?' + dom }]), koer('opsaetning', tema, { token_status: { sat: false } }, [{ navn: 'opsætningen', hash: 'overblik', aabn: ['.hm-hjaelp summary'] }]), koer('velkomst', tema, alleFra, [{ navn: 'velkomsten', hash: 'overblik' }])];
      /* Gradienten er lysest længst mod højre, så båndet måles også i det smalleste vindue og i standardvinduet. */
      const smalle = [900, 1200].map(b => koer('baand' + b, tema, {}, [{ navn: `båndet ved ${b} px`, hash: 'vaernet?samlet=1' }], b));
      const sider = r.flatMap(x => (x ? x.sider : [])), ud = [...r, ...smalle].flatMap(x => (x ? x.ud : [])), klasser = new Set(sider.flatMap(s => s.klasser));
      console.log(`     kontrast ${tema}: ` + sider.map(s => `${s.hvad} ${s.n}`).join(' · ') + ` · undtaget ${sider.reduce((a, s) => a + s.undtaget, 0)}`);
      tjek(`K7 ${tema}: alle ${SIDER.length + 16} visninger er målt, hver med tekst`, sider.length === SIDER.length + 16 && sider.every(s => s.n >= 3), sider.map(s => `${s.hvad} ${s.n}`).join(' · '));
      tjek(`K7 ${tema}: målingen rammer brødteksten (.vg-fordeling, .vg-note, .vg-intro), og dyks knap, mærke og chips`, ['vg-fordeling', 'vg-note', 'vg-intro', 'dyk-udsnit', 'dyk-filter', 'knap', 'vg-saerligt', 'trafik-maerke', 'server-linje', 'hul-linje', 'tek-chip', 'dato-felt', 'vg-trafik-note', 'land-udfald', 'moenster-maerke', 'moenster-linje', 'autostart-banner', 'autostart-note', 'dato-knap', 'kal-dag', 'dyk-trafik-linje', 'dyk-kun-i-dag'].every(k => klasser.has(k)), ['vg-fordeling', 'vg-note', 'vg-intro', 'dyk-udsnit', 'dyk-filter', 'knap'].filter(k => !klasser.has(k)).join());
      tjek(`K7 ${tema}: kontrolprøven: 5,55:1 fejler som lille tekst og består som stor`, r[0] && r[0].kontrol.join() === 'lille', r[0] && r[0].kontrol.join());
      const baand = [r[0], ...smalle].flatMap(x => (x ? x.baand : [])), navne = baand.filter(b => b.e !== 'SMALL'), lille = baand.filter(b => b.e === 'SMALL'), lav = l => Math.min(...l.map(b => b.k));
      console.log(`     bånd ${tema}: navnet ${lav(navne)}:1 (${navne[0]?.px}), "for Cloudflare®" ${lav(lille)}:1 (${lille[0]?.px}), bølgerne medregnet`);
      tjek(`K7 ${tema}: båndets navn er stor tekst efter getComputedStyle og har mindst 4,5:1 med bølgerne medregnet`, navne.length > 0 && navne.every(b => b.stor) && lav(navne) >= 4.5, JSON.stringify(navne.slice(0, 3)));
      tjek(`K7 ${tema}: "for Cloudflare®" er lille tekst og har mindst 7:1`, lille.length > 0 && lille.every(b => !b.stor) && lav(lille) >= 7, JSON.stringify(lille.slice(0, 3)));
      const set = new Map(); for (const f of ud) { const k = `${f.e} ${f.fg}/${f.bg}`; if (!set.has(k) || set.get(k).k > f.k) set.set(k, f); }
      tjek(`K7 ${tema}: al tekst undtagen --color-text-disabled har mindst 7:1 mod sin faktiske baggrund (stor tekst 4,5:1)`, ud.length === 0, [...set.values()].sort((a, b) => a.k - b.k).slice(0, 60).map(f => `${f.hvad}: ${f.e} ${f.fg}/${f.bg} ${f.k} »${f.t}«`).join('\n  '));
    }
    /* ── Alle visninger med appens egen CSP som rigtig header, og ingen inline-script i testsiden. En kontrolprøve viser, at lytteren virker. ── */
    const CSP = JSON.parse(fs.readFileSync(path.join(repo, 'src-tauri', 'tauri.conf.json'), 'utf8')).app.security.csp;
    const typer = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.png': 'image/png', '.woff2': 'font/woff2' };
    const vaert = http.createServer((q, s) => {
      const f = path.join(mappe, 'ui', decodeURIComponent(new URL(q.url, 'http://x').pathname));
      if (!f.startsWith(path.join(mappe, 'ui')) || !fs.existsSync(f)) { s.writeHead(404); return s.end(); }
      s.writeHead(200, { 'Content-Type': typer[path.extname(f)] || 'application/octet-stream', 'Content-Security-Policy': CSP });
      s.end(fs.readFileSync(f));
    });
    await new Promise(r => vaert.listen(0, '127.0.0.1', r));
    const gang = `addEventListener('load', async () => {
      const sov = ms => new Promise(r => setTimeout(r, ms)), set = [];
      for (const v of window.__HM_SIDER__) {
        location.hash = '#' + v.hash; await sov(350);
        for (const a of v.aabn || []) { const e = document.querySelector(a); if (e) { e.click(); await sov(120); } }
        document.querySelector('#indhold').scrollTop = 400; await sov(30);
        if (v.luk) { const e = document.querySelector(v.luk); if (e) e.click(); }
        const d = document.querySelector('dialog[open]'); if (d) d.close();
        set.push(v.navn);
      }
      location.hash = '#vaernet?zone=z1&navn=wi24rd.com'; await sov(350);
      const ark = document.adoptedStyleSheets.flatMap(a => [...a.cssRules]).length, tegnet = [...document.querySelectorAll('.dyk-diagram')].some(e => parseFloat(getComputedStyle(e).minWidth) > 0);
      location.hash = '#teknologi?zone=z2&navn=xyz.wi24rd.com'; await sov(500);
      const a = document.activeElement, li = a.closest('tr'), r = li ? li.getBoundingClientRect() : null;
      const fokus = { tek: li && li.dataset.zone, valgt: a.checked, kant: li && li.classList.contains('tek-fokus'), synlig: Boolean(r && r.top >= 0 && r.bottom <= innerHeight), hash: location.hash.includes('zone='), side: document.documentElement.scrollTop };
      const brud = window.__HM_BRUD__.slice(), proeve = document.createElement('div'); document.body.append(proeve);
      proeve.style.setProperty('color', 'rgb(1, 2, 3)'); await sov(30);
      const cssom = { ok: getComputedStyle(proeve).color === 'rgb(1, 2, 3)', brud: window.__HM_BRUD__.length - brud.length };
      proeve.setAttribute('style', 'color: rgb(4, 5, 6)'); await sov(60);
      const kontrol = window.__HM_BRUD__.slice(brud.length).map(b => b.dir);
      document.body.textContent = 'MAALT:' + JSON.stringify({ set, brud, ark, tegnet, fokus, cssom, kontrol }) + ':SLUT';
    });`;
    const csp = (tema, sprog) => new Promise(ok => {
      const navn = `csp-${tema}-${sprog}`, stub = `localStorage.setItem('hm-sprog',${JSON.stringify(sprog)});localStorage.setItem('hm-tekst','normal');localStorage.setItem('hm-tema',${JSON.stringify(tema)});window.__HM_PLATFORM__='macos';window.__HM_BRUD__=[];addEventListener('securitypolicyviolation',e=>__HM_BRUD__.push({dir:e.effectiveDirective,uri:e.blockedURI,fil:e.sourceFile,linje:e.lineNumber,prove:e.sample}));window.__HM_SIDER__=${JSON.stringify(SIDER)};const SVAR=${JSON.stringify(svar)};window.__TAURI__={core:{invoke:async c=>(c in SVAR?SVAR[c]:null)},event:{listen:async()=>()=>{}},app:{getVersion:async()=>'9.8.7'}};`;
      fs.writeFileSync(path.join(mappe, 'ui', `${navn}-stub.js`), stub);
      fs.writeFileSync(path.join(mappe, 'ui', 'csp-gang.js'), gang);
      fs.writeFileSync(path.join(mappe, 'ui', `${navn}.html`), ui('index.html').replace('<head>', `<head><script src="${navn}-stub.js"></script>`).replace('</body>', '<script src="csp-gang.js"></script></body>'));
      let ud = '';
      const p = spawn(CHROME, ['--headless=new', '--disable-gpu', '--hide-scrollbars', `--user-data-dir=${path.join(mappe, 'p-' + navn)}`, '--window-size=1400,1000', '--virtual-time-budget=60000', '--dump-dom', `http://127.0.0.1:${vaert.address().port}/${navn}.html`]);
      const ur = setTimeout(() => p.kill(), 120000);
      p.stdout.on('data', x => { ud += x; });
      p.on('close', () => { clearTimeout(ur); const m = /MAALT:(.*):SLUT/s.exec(ud); ok(m ? JSON.parse(m[1].replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&')) : null); });
    });
    for (const [tema, sprog] of [['light', 'da'], ['dark', 'da'], ['light', 'en'], ['dark', 'en']]) {
      const r = await csp(tema, sprog), nav2 = `${tema}, ${sprog}`;
      tjek(`CSP ${nav2}: alle ${SIDER.length} visninger er besøgt med appens CSP som header`, r && r.set.length === SIDER.length, r && r.set.length);
      tjek(`CSP ${nav2}: 0 overtrædelser`, r && r.brud.length === 0, JSON.stringify(r && r.brud.slice(0, 6)));
      tjek(`CSP ${nav2}: kontrolprøve: setAttribute('style') bliver stoppet og set af lytteren`, r && r.kontrol.join() === 'style-src-attr', r && r.kontrol.join());
      tjek(`CSP ${nav2}: CSSOM virker: setProperty og reglerne i eget stylesheet slår igennem uden overtrædelse`, r && r.cssom.ok && !r.cssom.brud && r.ark > 0 && r.tegnet, JSON.stringify(r && [r.cssom, r.ark, r.tegnet]));
      tjek(`Teknologi-linjen ${nav2}: klik i tabellen: linjen er synlig, fokus på det valgte, med kant`, r && r.fokus.tek && r.fokus.valgt && r.fokus.kant && r.fokus.synlig && r.fokus.hash && r.fokus.side === 0, JSON.stringify(r && r.fokus));
    }
    vaert.close();
    fs.rmSync(mappe, { recursive: true, force: true });
  }
}

// ── P6: README ──
{
  const undtagelser = /wi24rd-tokens|build-tokens|tokens\.css|API Token(?:s|et)?|Custom Token|api-tokens/g;
  for (const f of ['README.md', 'README.da.md']) {
    const rest = fs.readFileSync(path.join(repo, f), 'utf8').replace(undtagelser, '').match(/.{0,30}token.{0,20}/gi) || [];
    tjek(`P6 ${f}: ingen "token" uden for undtagelserne`, rest.length === 0, rest.join(' | '));
  }
  const da = fs.readFileSync(path.join(repo, 'README.da.md'), 'utf8'), en = fs.readFileSync(path.join(repo, 'README.md'), 'utf8');
  tjek('P6 begge README har Tekststørrelse og Nulstil historik og de nye valg pr. domæne', da.includes('**Tekststørrelse:**') && da.includes('**Nulstil historik**') && da.includes('Alvorligt og hold øje') && en.includes('**Text size:**') && en.includes('**Reset history**') && en.includes('Serious and watch') && !/Kun ved Handl|Only for Act/.test(da + en));
}

// ── ──
{
  const LANG = '2026-11-05';
  /* Hent nu */
  const hentSvar = u => ({ ...MED_TOKEN, overblik: [], hent_nu: u, valg_hent: [], koe_status: null, siden_sidst: { vis: false, nye: [], raekker: [] }, siden_sidst_set: null });
  const hent = async (u, x = {}) => { const s = side('#overblik', { ...hentSvar(u), ...x }); await sov(60); return s; };
  const knap = s => s.d.getElementById('hent-nu'), tekst = s => s.d.getElementById('hent-status').textContent;
  let slip;
  const a1 = await hent(() => new Promise(r => { slip = r; }));
  knap(a1).click(); await sov(5);
  tjek('Hent nu: straks "Henter …": deaktiveret, spinner og aria-busy', knap(a1).disabled && knap(a1).classList.contains('henter') && knap(a1).getAttribute('aria-busy') === 'true' && knap(a1).textContent === 'Henter …' && tekst(a1) === 'Henter …', knap(a1).outerHTML.slice(0, 120));
  tjek('Hent nu: svaret læses op (aria-live)', a1.d.getElementById('hent-status').getAttribute('aria-live') === 'polite');
  a1.d.getElementById('data-hent').click(); await sov(5);
  tjek('Hent nu: Indstillinger → Data viser det samme', a1.d.getElementById('data-hent').disabled && a1.d.getElementById('data-hent-status').textContent === 'Henter …' && a1.kald.filter(k => k[0] === 'hent_nu').length === 1, a1.d.getElementById('data-hent-status').textContent);
  slip({ udfald: 'startet', faerdige: 0, i_alt: 4 }); await sov(10);
  tjek('Hent nu: en startet runde står, til køen melder', knap(a1).disabled);
  const koe = (t, x = {}) => ({ zone: null, navn: null, doegn_faerdige: 1, doegn_i_alt: 2, zoner_faerdige: 4, zoner_i_alt: 4, zoner_fejl: 0, zoner_opdateret: 0, tilstand: t, ...x });
  a1.emit('hentekoe', koe('faerdig', { zoner_opdateret: 3 })); await sov(5);
  tjek('Hent nu: en gammel "færdig" før runden tæller ikke', knap(a1).disabled);
  a1.emit('hentekoe', koe('henter')); await sov(5);
  a1.emit('hentekoe', koe('faerdig', { zoner_opdateret: 3 })); await sov(5);
  tjek('Hent nu: "Hentet kl. … — N domæner opdateret"', /^Hentet kl\. \d\d\.\d\d — 3 domæner opdateret$/.test(tekst(a1)) && !knap(a1).disabled && knap(a1).textContent === 'Hent nu' && !knap(a1).hasAttribute('aria-busy'), tekst(a1));
  const a2 = await hent({ udfald: 'intet_at_hente', faerdige: 0, i_alt: 0 }, { overblik: [{ ...OVERBLIK[0], sidst_hentet: '2026-11-06T08:52:00Z' }] });
  knap(a2).click(); await sov(10);
  tjek('Hent nu: "Alt er hentet — intet nyt siden kl. …"', /^Alt er hentet — intet nyt siden kl\. \d\d\.52$/.test(tekst(a2)) && !knap(a2).disabled, tekst(a2));
  const a3 = await hent({ udfald: 'koerer_allerede', faerdige: 2, i_alt: 4 });
  knap(a3).click(); await sov(10);
  tjek('Hent nu: "Henter allerede (2 af 4) …"', tekst(a3) === 'Henter allerede (2 af 4) …' && !knap(a3).disabled, tekst(a3));
  const a4 = await hent(() => { throw { kode: 'lager' }; });
  knap(a4).click(); await sov(10);
  tjek('Hent nu: en fejl i klarsprog', tekst(a4) === 'Kunne ikke hente: Den lokale database kunne ikke bruges.' && !knap(a4).disabled && a4.d.getElementById('hent-status').classList.contains('roed'), tekst(a4));
  await sov(4300);
  tjek('Hent nu: fejlen står, og "Alt er hentet" forsvinder efter 4 sekunder', tekst(a4).startsWith('Kunne ikke hente') && tekst(a2) === '', tekst(a2));
  const a5 = await hent({ udfald: 'startet', faerdige: 0, i_alt: 4 });
  knap(a5).click(); await sov(10);
  a5.emit('hentekoe', koe('fejl', { fejl: { kode: 'forbindelse' } })); await sov(5);
  tjek('Hent nu: køen fejler undervejs', tekst(a5).startsWith('Kunne ikke hente: ') && !knap(a5).disabled, tekst(a5));
  const a6 = await hent({ udfald: 'intet_at_hente', faerdige: 0, i_alt: 0 }, {});
  a6.emit('hent_nu'); await sov(10);
  tjek('Hent nu: ⌘R går samme vej (hændelsen hent_nu)', a6.kald.some(k => k[0] === 'hent_nu') && tekst(a6) === 'Alt er hentet.', tekst(a6));
  const a7 = side('#overblik', { ...hentSvar({ udfald: 'intet_at_hente', faerdige: 0, i_alt: 0 }) }, 'en'); await sov(60);
  knap(a7).click(); await sov(10);
  tjek('Hent nu engelsk', tekst(a7) === 'All fetched.', tekst(a7));

  /* Blokken står altid */
  const L0 = { ...LOGIN_UD, stier: [], raekker: [] };
  const l1 = side('#login?zone=z1&navn=wi24rd.com', DYK({ ...VIS, login: L0 })), l2 = side('#login?zone=z1&navn=wi24rd.com', DYK({ ...VIS, login: { ...LOGIN_UD, laek_andet: [{ v: 'lookup_timeout', n: 3 }, { v: 'ukendt', n: 1 }] } })), l3 = side('#login?zone=z1&navn=wi24rd.com', DYK({ ...VIS, login: { ...L0, laek_dage: 0 } }));
  await sov(120);
  const rodL = x => x.d.getElementById('login-rod').textContent;
  tjek('Lækkede kodeord: blokken står også uden slåede-til stier', rodL(l1).includes('Kodeord fra kendte læk: ingen forsøg i perioden.') && rodL(l1).includes('Ingen login-stier er slået til') && rodL(l1).includes('Cloudflare sammenligner kodeordet'), rodL(l1).slice(0, 200));
  tjek('Lækkede kodeord: siger, at det er slået til på Free, og hvor på en betalt plan', rodL(l1).includes('slået til som standard på Free-planen') && rodL(l1).includes('Security → Settings'));
  tjek('Lækkede kodeord: med stier', rodL(l2).includes('ingen forsøg i perioden'));
  tjek('Lækkede kodeord: en ukendt værdi står rå, så den kan slås op', rodL(l2).includes('ukendt: lookup_timeout (3)') && rodL(l2).includes('Uden svar fra tjekket: 1.'), rodL(l2).slice(0, 400));
  tjek('Lækkede kodeord: ikke hentet endnu siger det', rodL(l3).includes('Tjekket er ikke hentet for perioden endnu.'));

  /* Årstal */
  const SP = side('#overblik', DYK()).w.hmSprog;
  await sov(40);
  tjek('Årstal: et andet år står med året', SP.datoLang('2025-11-07') === '7. november 2025' && SP.datoLang(`${new Date(urNu()).getUTCFullYear()}-11-07`) === '7. november', SP.datoLang('2025-11-07'));
  tjek('Årstal: to datoer over et årsskifte har begge året', SP.datoPar('2025-11-12', `${new Date(urNu()).getUTCFullYear()}-11-05`).join(' – ') === `12. november 2025 – 5. november ${new Date(urNu()).getUTCFullYear()}`, SP.datoPar('2025-11-12', '2026-11-05').join());
  tjek('Årstal: i samme år uden', SP.datoPar('2026-10-12', '2026-11-05').join(' – ') === '12. oktober – 5. november');
  const dy = side('#dyk?' + dykQ({ fra: '2025-11-12', til: '2026-11-05' }), DYK({ dyk: dykSvar({ sum: 5, periode: { ...dykSvar().periode, fra: '2025-11-12', til: '2026-11-05', dage: 359, med: 359 } }) }));
  await sov(120);
  const rodD = dy.d.getElementById('dyk-rod').textContent, chipD = dy.d.getElementById('dyk-filtre').textContent;
  tjek('Årstal: flise og chip over årsskiftet', rodD.includes('12. november 2025 – 5. november 2026') && chipD.includes('12. nov. 2025 – 5. nov. 2026'), rodD.slice(0, 200) + chipD);
  const dataS = x => side('#indstillinger', DYK({ data_status: { bytes: 1, nulstil: { sikkerhed: null, trafik: null }, domaener: 2, ...x }, autostart_hent: true, domaener_hent: [] }));
  const i1 = dataS({ trafik_fra: '2025-11-07', sikkerhed_fra: '2026-10-04', foerste: '2025-11-07', sidste: '2026-11-06' }), i2 = dataS({ trafik_fra: null, sikkerhed_fra: '2026-10-04', foerste: '2026-10-04', sidste: '2026-11-06' });
  await sov(120);
  tjek('Årstal: Indstillinger med år, og Historik er hele spændet', i1.d.getElementById('data-kilder').textContent === 'Trafik fra 7. nov. 2025 · sikkerhedshændelser fra 4. okt. 2026' && i1.d.getElementById('data-historik').textContent === '7. nov. 2025 – 6. nov. 2026' && i1.d.getElementById('data-aeldste').textContent === '7. nov. 2025', i1.d.getElementById('data-kilder').textContent);
  tjek('Årstal: mangler årstal: "Trafiktal hentes …"', i2.d.getElementById('data-kilder').textContent === 'Trafiktal hentes … · sikkerhedshændelser fra 4. okt. 2026', i2.d.getElementById('data-kilder').textContent);

  /* Del B: kvoten i Indstillinger */
  const q1 = dataS({ kvote: { kald: 1201, minutter: 5, brugt: 42 } }), q2 = dataS({ kvote: null });
  await sov(120);
  tjek('Del B: kvoten står, når Cloudflare har meldt den', q1.d.getElementById('data-kvote').textContent === 'Cloudflare tillader 1.201 kald pr. 5 minutter; Huginn brugte 42 i sidste runde.' && !q1.d.getElementById('data-kvote-raekke').hidden, q1.d.getElementById('data-kvote').textContent);
  tjek('Del B: uden headere står rækken ikke', q2.d.getElementById('data-kvote-raekke').hidden);

  /* Ét mål */
  const R7 = raekke('z1', 'wi24rd.com', 'gul', { i_gaar: 12, hele7: 30, hele30: 90, angreb_i_gaar: 15, angreb_snit: 2, angreb_fordeling: { blokeret: 15 }, angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 15, egen: false, tek: 'react', regler: [] }], ikke_brugt: [{ tek: 'wordpress', n: 9 }, { tek: 'react', n: 6 }], tek_spoerg: 'php', angreb_status: 'gul', trafik: { dag: LANG, n: 300, median: 3, stoppet: 200 }, moenster: { slags: 'maalrettet', dag: LANG, netvaerk: 'X', procent: 75, verden: true, foreloebig: false, tek: null }, sidst_hentet: NU_ISO });
  const ov = side('#overblik', DYK({ overblik: [R7] }));
  await sov(120);
  const nyTr = () => ov.d.querySelector('tr[data-zone="z1"]'), til = () => ov.w.location.hash;
  const klikPaa = async find => { ov.w.location.hash = '#overblik'; await sov(30); find(nyTr()).dispatchEvent(new ov.w.MouseEvent('click', { bubbles: true, cancelable: true })); await sov(30); return til(); };
  const ANGREB = '#vaernet?zone=z1&navn=wi24rd.com';
  for (const [navn, find] of [['domænet', t => t.querySelector('th')], ['status', t => t.querySelector('.chip')], ['mærket "Ligner"', t => t.querySelector('.moenster-maerke')], ['Hvorfor-teksten', t => t.querySelector('.hv-tekst')], ['tom plads', t => t.querySelector('.chev')]]) {
    tjek(`Ét klikmål: et klik på ${navn} åbner Angreb`, await klikPaa(find) === ANGREB, til());
  }
  tjek('Ét klikmål: mærket og chippen i rækken er ikke knapper', !nyTr().querySelector('td.st button'));
  tjek('Ét klikmål: tal går til dyk', (await klikPaa(x => x.querySelector('.tal-knap'))).startsWith('#dyk?'), til());
  tjek('Ét klikmål: "som du ikke bruger" går til #teknologi', (await klikPaa(x => x.querySelector('.hv-tek'))).startsWith('#teknologi?zone=z1'), til());
  tjek('Ét klikmål: "Bruger du ikke …?" går til #teknologi', (await klikPaa(x => x.querySelector('.hv-spoerg'))).startsWith('#teknologi?zone=z1'), til());
  tjek('Ét klikmål: mærket trafik går til trafik-dyk', (await klikPaa(x => x.querySelector('.trafik-maerke'))).includes('trafik=1'), til());
  await klikPaa(x => x.querySelector('.hv-guide'));
  tjek('Ét klikmål: "Hvad gør jeg?" åbner guiden og ikke Angreb', til() === '#overblik' && !ov.d.getElementById('hjaelp').hidden, til());
  ov.w.hmRamme.hjaelp(false);
  const ht = nyTr().querySelector('.hv-tekst').textContent;
  tjek('Hvorfor-tekst: tegnsætning og flere teknologier i én sætning', ht.includes('15 angreb på WordPress og React, som du ikke bruger — ufarlige for dig.') && !/,som|bruger—/.test(ht), ht);
  const css = ui('assets/vagt.css') + ui('assets/app.css');
  tjek('Ét klikmål: reglen for små links er kun på .link', /\.link\[data-dyk\]:hover/.test(css) && !/(^|[\s,])button\[data-dyk\]:hover/m.test(css));
  tjek('Hvorfor-tekst: linket står inline', /\.hv-tek \{ display: inline;/.test(css));
  const rust = fs.readFileSync(path.join(repo, 'src-tauri/src/notify.rs'), 'utf8');
  tjek('Hvorfor-tekst: Rust samler også sætningen', rust.includes('Flere teknologier står i én sætning'));

  /* #teknologi */
  const SIDE = () => ({ domaener: [{ zone: 'z1', navn: 'wi24rd.com', valg: { react: ['bruger_ikke', false], php: ['bruger', true] }, antal: { react: 204, wordpress: 5 } }, { zone: 'z2', navn: 'b.wi24rd.com', valg: {}, antal: { wordpress: 1 } }], katalog: [
    { tek: 'magento', navn: 'Magento', gruppe: 'cms', angreb: 0, set: false, vis: false, ny: false }, { tek: 'php', navn: 'PHP', gruppe: 'server', angreb: 0, set: false, vis: true, ny: false }, { tek: 'react', navn: 'React', gruppe: 'ramme', angreb: 204, set: true, vis: true, ny: false }, { tek: 'wordpress', navn: 'WordPress', gruppe: 'cms', angreb: 6, set: true, vis: true, ny: true }, { tek: 'yeti platform', navn: 'Yeti Platform', gruppe: 'andet', angreb: 0, set: false, vis: false, ny: false }] });
  const TS = x => DYK({ teknologi_side: SIDE(), teknologi_saet: null, teknologi_vis_saet: null, teknologi_kolonne: null, teknologi_nulstil: null, teknologi_alle: 1, teknologi_tilfoej: 'min butik', ...x });
  const t = side('#teknologi?zone=z2&navn=b.wi24rd.com', TS(), 'da', 'windows', w => { w.HTMLElement.prototype.scrollIntoView = function () { w.__rul = this.dataset.zone; }; });
  await sov(120);
  const { d } = t, rkT = () => [...d.querySelectorAll('.tek-gitter tbody tr')], hd = () => [...d.querySelectorAll('.tek-gitter thead th')].map(x => x.querySelector('.tek-kol')?.textContent || x.textContent);
  tjek('Teknologi: rækker = valgte domæner, kolonner = "Vis i tabellen"', hd().join() === 'Domæne,PHP,React,WordPress' && rkT().map(x => x.dataset.zone).join() === 'z1,z2', hd().join());
  const cel = (r, k) => rkT()[r].cells[k], grp = c => c.querySelector('[role="radiogroup"]');
  tjek('Teknologi: en radiogruppe pr. celle med aria-label "domæne, teknologi"', grp(cel(0, 2)).getAttribute('aria-label') === 'wi24rd.com, React' && grp(cel(0, 2)).querySelectorAll('input[type="radio"]').length === 3 && cel(0, 2).querySelector('input:checked').value === 'bruger_ikke', grp(cel(0, 2)).outerHTML.slice(0, 120));
  tjek('Teknologi: tegnene ✓ ✕ ? med navn til skærmlæsere', [...grp(cel(0, 2)).querySelectorAll('label')].map(l => l.textContent).join('') === '✓✕?' && [...grp(cel(0, 2)).querySelectorAll('input')].map(i => i.getAttribute('aria-label')).join() === 'Bruger,Bruger ikke,Ved ikke');
  tjek('Teknologi: ikke-set er lysere, men kan sættes', cel(1, 2).classList.contains('tek-ikke-set') && !cel(0, 2).classList.contains('tek-ikke-set') && !cel(1, 2).querySelector('input').disabled);
  const nq = new URLSearchParams(cel(0, 2).querySelector('.tek-n')?.dataset.dyk || '');
  tjek('Teknologi: antallet åbner dyk for domænet og teknologien', cel(0, 2).querySelector('.tek-n')?.textContent === '204' && nq.get('zone') === 'z1' && nq.get('teknologi') === 'react' && nq.get('slags') === 'angreb' && cel(0, 2).querySelector('.tek-n')?.getAttribute('aria-label') === '204 angreb på React på wi24rd.com: åbn dem', [...nq].join());
  tjek('Teknologi: Giv mig besked kun ved Bruger', cel(0, 1).querySelector('input[type="checkbox"]').checked && !cel(0, 2).querySelector('input[type="checkbox"]'));
  tjek('Dit sites teknologi: domænet i fokus', d.activeElement.closest('tr')?.dataset.zone === 'z2' && t.w.__rul === 'z2' && rkT()[1].classList.contains('tek-fokus'), d.activeElement.outerHTML.slice(0, 100));
  const tabStop = () => [...d.querySelectorAll('.tek-radio input')].filter(i => i.tabIndex === 0);
  tjek('Teknologi, tastatur: tabellen er ét Tab-stop', tabStop().length === 1, tabStop().length);
  const tryk = (key, x = {}) => d.activeElement.dispatchEvent(new t.w.KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...x }));
  tryk('ArrowRight');
  tjek('Teknologi, tastatur: pil højre flytter mellem celler, uden at ændre valg', d.activeElement.closest('td') === cel(1, 2) && tabStop().length === 1 && !t.kald.some(k => k[0] === 'teknologi_saet'));
  tryk('ArrowUp');
  tjek('Teknologi, tastatur: pil op', d.activeElement.closest('td') === cel(0, 2));
  tryk('ArrowDown'); tryk('ArrowRight'); tryk('ArrowRight');
  tjek('Teknologi, tastatur: ved kanten sker der intet', d.activeElement.closest('td') === cel(1, 3));
  tryk(' '); await sov(40);
  const sidst = () => t.kald.filter(k => k[0] === 'teknologi_saet').at(-1)?.[1];
  tjek('Teknologi: mellemrum skifter valg og gemmer', JSON.stringify(sidst()) === '{"zone":"z2","tek":"wordpress","valg":"bruger","besked":false}' && d.activeElement.closest('td')?.dataset.tek === 'wordpress' && t.kald.some(k => k[0] === 'overblik'), JSON.stringify(sidst()));
  tryk('Enter'); tryk('ArrowRight'); await sov(40);
  tjek('Teknologi: Enter går ind i gruppen, og piletasterne vælger', sidst().valg === 'bruger_ikke' && d.getElementById('tek-side-status').textContent !== '', JSON.stringify(sidst()));
  tryk('ArrowRight'); await sov(40);
  tjek('Teknologi: piletasterne bliver ved med at vælge, selv om cellen tegnes om', sidst().valg === 'ved_ikke' && d.activeElement.closest('td')?.dataset.tek === 'wordpress', JSON.stringify(sidst()));
  tryk('Escape'); tryk('ArrowLeft');
  tjek('Teknologi: Esc går tilbage til tabellen', d.activeElement.closest('td')?.dataset.tek === 'react', d.activeElement.outerHTML.slice(0, 90));
  const kolKnap = (i, n) => d.querySelectorAll('.tek-gitter thead th')[i].querySelectorAll('button')[n];
  kolKnap(2, 0).click(); kolKnap(2, 1).click(); await sov(40);
  tjek('Teknologi: "Bruger ikke på alle"', JSON.stringify(t.kald.filter(k => k[0] === 'teknologi_kolonne').at(-1)[1]) === '{"tek":"react","valg":"bruger_ikke"}');
  kolKnap(2, 0).click(); kolKnap(2, 2).click(); await sov(40);
  tjek('Kolonnemenu: efter handlingen står fokus på kolonnens knap', d.activeElement.classList.contains('tek-kol-knap') && d.activeElement.dataset.tek === 'react', d.activeElement.outerHTML.slice(0, 90));
  tjek('Teknologi: "Nulstil kolonnen"', JSON.stringify(t.kald.filter(k => k[0] === 'teknologi_kolonne').at(-1)[1]) === '{"tek":"react","valg":"ved_ikke"}');
  rkT()[0].querySelector('th button').click(); await sov(20);
  d.getElementById('bekraeft-ja').click(); await sov(40);
  tjek('Teknologi: "Nulstil domænet" spørger og nulstiller kun domænet', JSON.stringify(t.kald.filter(k => k[0] === 'teknologi_nulstil').at(-1)[1]) === '{"zone":"z1"}');
  const soeg = d.querySelector('.tek-soeg input');
  soeg.value = 'word'; soeg.dispatchEvent(new t.w.Event('input', { bubbles: true })); await sov(10);
  tjek('Teknologi: søgefeltet filtrerer kolonnerne', hd().join() === 'Domæne,WordPress', hd().join());
  soeg.value = 'zzz'; soeg.dispatchEvent(new t.w.Event('input', { bubbles: true })); await sov(10);
  tjek('Teknologi: ingen træf siger det', d.getElementById('teknologi-rod').textContent.includes('Ingen teknologi passer til søgningen.'));
  soeg.value = ''; soeg.dispatchEvent(new t.w.Event('input', { bubbles: true })); await sov(10);
  const nyL = d.querySelector('.tek-ny');
  tjek('Alle teknologier: diskret linje om en ny teknologi', nyL.textContent.startsWith('Ny teknologi set: WordPress — vis i tabellen?'), nyL.textContent);
  nyL.querySelectorAll('button')[1].click(); await sov(40);
  tjek('Alle teknologier: "Nej" skjuler den i tabellen', JSON.stringify(t.kald.filter(k => k[0] === 'teknologi_vis_saet').at(-1)[1]) === '{"tek":"wordpress","vis":false}');
  const a = side('#teknologi?fane=alle', TS());
  await sov(120);
  const ad = a.d, grupper = () => [...ad.querySelectorAll('.tek-gruppe h2')].map(x => x.textContent);
  tjek('Alle teknologier: fanen "Alle teknologier" er grupperet', grupper().join() === 'CMS,Webrammeværk,Server og sprog,Andet', grupper().join());
  tjek('Alle teknologier: kun synlige er afkrydset', ad.querySelector('[data-tek="magento"] input').checked === false && ad.querySelector('[data-tek="react"] input').checked === true);
  const rq = new URLSearchParams(ad.querySelector('[data-tek="react"] [data-dyk]').dataset.dyk);
  tjek('Alle teknologier: antal angreb på kontoen åbner det samlede dyk', ad.querySelector('[data-tek="react"] [data-dyk]').textContent === '204 angreb på kontoen' && rq.get('samlet') === '1' && rq.get('teknologi') === 'react' && ad.querySelector('[data-tek="magento"]').textContent.includes('Ikke set'), [...rq].join());
  const vb = ad.querySelector('[data-tek="react"] input');
  vb.checked = false; vb.dispatchEvent(new a.w.Event('change', { bubbles: true })); await sov(40);
  tjek('Alle teknologier: skjul i tabellen er ét kald og intet valg', JSON.stringify(a.kald.filter(k => k[0] === 'teknologi_vis_saet').at(-1)[1]) === '{"tek":"react","vis":false}' && !a.kald.some(k => k[0] === 'teknologi_saet' || k[0] === 'teknologi_kolonne'));
  ad.querySelector('[data-tek="magento"] .tek-lille').click(); await sov(40);
  tjek('Alle teknologier: "Bruger ikke på alle" før første angreb', JSON.stringify(a.kald.filter(k => k[0] === 'teknologi_kolonne').at(-1)[1]) === '{"tek":"magento","valg":"bruger_ikke"}');
  const fm = ad.querySelector('form.login-egen'), felt = fm.querySelector('input');
  tjek('Teknologinavne: "Tilføj teknologi" foreslår kendte navne', ad.querySelector('#tek-navne').querySelectorAll('option').length === 5 && felt.getAttribute('list') === 'tek-navne');
  felt.value = ' Min Butik '; fm.dispatchEvent(new a.w.Event('submit', { bubbles: true, cancelable: true })); await sov(50);
  tjek('Teknologinavne: fri tekst tilføjes', a.kald.filter(k => k[0] === 'teknologi_tilfoej').at(-1)[1].navn === 'Min Butik' && ad.getElementById('tek-side-status').textContent === 'Min Butik er tilføjet.', ad.getElementById('tek-side-status').textContent);
  const af = side('#teknologi?fane=alle', TS({ teknologi_tilfoej: () => { throw { kode: 'ugyldigt_filter' }; } }));
  await sov(120);
  const fm2 = af.d.querySelector('form.login-egen');
  fm2.querySelector('input').value = 'generic rules'; fm2.dispatchEvent(new af.w.Event('submit', { bubbles: true, cancelable: true })); await sov(40);
  tjek('Teknologinavne: et navn, der ikke kan bruges, siger hvorfor', af.d.getElementById('tek-side-status').textContent.startsWith('Navnet kan ikke bruges.'), af.d.getElementById('tek-side-status').textContent);
  tjek('Teknologi: sidebjælken peger på Indstillinger, og vinduet har en titel', a.d.querySelector('#side a[data-visning="indstillinger"]').getAttribute('aria-current') === 'page' && a.d.title.startsWith('Teknologi pr. domæne'), a.d.title);
  tjek('Teknologi: de nye kommandoer er tilladt', ['teknologi_side', 'teknologi_vis_saet', 'teknologi_kolonne', 'teknologi_tilfoej'].every(tilladt));
  const tom = side('#teknologi', TS({ teknologi_side: { domaener: [], katalog: [] } }));
  await sov(120);
  tjek('Teknologi: intet valgt: velkomsten', !tom.d.querySelector('.tek-gitter') && tom.d.getElementById('teknologi-rod').textContent.length > 0);
  const eng = side('#teknologi', TS(), 'en');
  await sov(120);
  tjek('Teknologi engelsk', eng.d.getElementById('teknologi-rod').textContent.includes('✓ Uses · ✕ Does not use · ? Don’t know') && eng.d.querySelector('.tek-gitter thead .tek-lille').textContent === 'Not used anywhere', eng.d.querySelector('.tek-gitter thead').textContent.slice(0, 80));

  /* "Hvad angrebene var rettet mod" */
  const RM = [{ tek: 'react', n: 1800, valg: 'bruger_ikke' }, { tek: 'wordpress', n: 40, valg: 'bruger' }, { tek: 'magento', n: 7, valg: 'ved_ikke' }, { tek: 'andet', n: 3, valg: 'ved_ikke' }];
  const V4 = vaernUd(), RV = x => DYK({ vaern_cache: V4, vaern_oversigt: V4, koe_prioritet: null, teknologi_hent: { linjer: [], generiske: [] }, dyk: dykSvar({ sum: 1879, rettet_mod: RM }), ...x });
  const rv = side('#vaernet?zone=z1&navn=wi24rd.com', RV()), rs = side('#vaernet?samlet=1', RV({ vaern_samlet_cache: { vaern: V4, valgte: [{ id: 'z1', navn: 'a', med_tal: true }], domaener: 1, med_tal: 1 } }));
  await sov(160);
  const lj = x => [...x.d.querySelectorAll('.rettet-linje')];
  tjek('Rettet mod: alle teknologier står i Angreb, sorteret efter antal', lj(rv).map(l => l.querySelector('.rettet-navn').textContent).join() === 'React,WordPress,Magento,Andet' && rv.d.querySelector('[data-plads="rettet"] h2').textContent === 'Hvad angrebene var rettet mod', lj(rv).map(l => l.textContent).join('|'));
  tjek('Rettet mod: Angreb spørger om de 30 dage og kun angreb', (k => k && k[1].periode === 'd30' && k[1].slags === 'angreb' && k[1].omfang.zone === 'z1')(rv.kald.find(x => x[0] === 'dyk')));
  tjek('Rettet mod: "Bruger ikke" står gråt med "ufarligt for dig", men står der', lj(rv)[0].classList.contains('rettet-ikke') && lj(rv)[0].textContent.includes('ufarligt for dig') && lj(rv)[0].textContent.includes('Bruger ikke'), lj(rv)[0].textContent);
  tjek('Rettet mod: valget er et mærke', lj(rv)[1].querySelector('.tek-maerke').textContent === 'Bruger' && lj(rv)[2].querySelector('.tek-maerke').textContent === 'Ved ikke' && !lj(rv)[3].querySelector('.tek-maerke'));
  const rq2 = new URLSearchParams(lj(rv)[0].querySelector('.rettet-n').dataset.dyk);
  tjek('Rettet mod: antallet åbner dyk', lj(rv)[0].querySelector('.rettet-n').textContent === '1.800' && rq2.get('teknologi') === 'react' && rq2.get('slags') === 'angreb' && rq2.get('zone') === 'z1', [...rq2].join());
  tjek('Rettet mod: står også samlet', lj(rs).length === 4 && rs.kald.find(x => x[0] === 'dyk')[1].omfang.zoner.length > 0 && new URLSearchParams(lj(rs)[0].querySelector('.rettet-n').dataset.dyk).get('samlet') === '1');
  const dr = side('#dyk?' + dykQ({}), RV({ teknologi_hent: { linjer: [{ tek: 'php', angreb30: 1, forsoeg30: 0, foerst: '2026-10-05', sidst: '2026-11-02', valg: 'bruger', besked: false, forslag: false }], generiske: [] } })), dq = side('#dyk?' + dykQ({ teknologi: 'react' }), RV());
  await sov(160);
  const rodR = dr.d.getElementById('dyk-rod');
  tjek('Dit sites teknologi: dyk for ét domæne har chips og "Ret teknologier"', rodR.querySelector('.tek-chip')?.textContent === 'Bruger: PHP' && rodR.querySelector('.tek-ret')?.dataset.gaa === 'teknologi?zone=z1&navn=wi24rd.com', rodR.textContent.slice(0, 160));
  tjek('Rettet mod: dyk har listen, og tallet lægger et filter på', lj(dr).length === 4 && new URLSearchParams(lj(dr)[2].querySelector('.rettet-n').dataset.dyk).get('teknologi') === 'magento');
  tjek('Rettet mod: ikke, når udsnittet allerede er én teknologi', !lj(dq).length);

  // ── ──
  {
  /* `none` er ikke et læk, og teksten siger kun det, Cloudflare kan stå inde for. */
  const LN = { ...LOGIN_UD, laek_andet: [{ v: 'none', n: 1234 }, { v: 'ukendt', n: 3 }, { v: 'username_leaked', n: 2 }] };
  const n1 = side('#login?zone=z1&navn=wi24rd.com', DYK({ ...VIS, login: LN })), n2 = side('#login?zone=z1&navn=wi24rd.com', DYK({ ...VIS, login: LN }), 'en');
  await sov(120);
  const rodN = x => x.d.getElementById('login-rod').textContent;
  tjek('none: "none" står med Cloudflares svar og tallet', rodN(n1).includes('Cloudflare svarede »none«: forespørgslen blev ikke tjekket for lækkede kodeord: 1.234.'), rodN(n1).slice(0, 300));
  tjek('none: tom værdi har sin egen linje', rodN(n1).includes('Uden svar fra tjekket: 3.'));
  tjek('none: aldrig "ukendt: none" og aldrig "login-forsøg" om det', !rodN(n1).includes('ukendt: none') && !rodN(n1).includes('(1.234)') && !n1.d.querySelector('.login-laek-none').textContent.includes('login'));
  tjek('none: de andre svar står stadig', rodN(n1).includes('Andre svar fra tjekket: kun brugernavnet stod i et læk (2).'));
  tjek('none: "none" tæller ikke i læk-tallet', rodN(n1).includes('Kodeord fra kendte læk: ingen forsøg i perioden.') && !n1.d.querySelector('.login-laek-linje'));
  tjek('none: engelsk', rodN(n2).includes('Cloudflare answered “none”: the request was not checked for leaked passwords: 1,234.') && rodN(n2).includes('No answer from the check: 3.'));

  /* Ét Tab-stop pr. kolonne. */
  const m1 = side('#teknologi', TS());
  await sov(120);
  const kol = i => m1.d.querySelectorAll('.tek-gitter thead th')[i], stop = th => [...th.querySelectorAll('button')].filter(b => !b.closest('[hidden]'));
  tjek('Kolonnemenu: ét Tab-stop pr. kolonne', [1, 2, 3].every(i => stop(kol(i)).length === 1), [1, 2, 3].map(i => stop(kol(i)).length).join());
  const mk = kol(2).querySelector('.tek-kol-knap');
  tjek('Kolonnemenu: knappen siger hvilken kolonne, og at den er lukket', mk.textContent === '⋯' && mk.getAttribute('aria-label') === 'Handlinger for React' && mk.getAttribute('aria-expanded') === 'false');
  mk.click();
  tjek('Kolonnemenu: åben viser handlingerne, og fokus står på den første', stop(kol(2)).length === 3 && mk.getAttribute('aria-expanded') === 'true' && m1.d.activeElement === stop(kol(2))[1], m1.d.activeElement.outerHTML.slice(0, 80));
  kol(3).querySelector('.tek-kol-knap').click();
  tjek('Kolonnemenu: kun én er åben ad gangen', stop(kol(2)).length === 1 && stop(kol(3)).length === 3 && mk.getAttribute('aria-expanded') === 'false');
  m1.d.activeElement.dispatchEvent(new m1.w.KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  tjek('Kolonnemenu: Esc lukker og giver fokus tilbage', stop(kol(3)).length === 1 && m1.d.activeElement === kol(3).querySelector('.tek-kol-knap'));
  tjek('Kolonnemenu: ingen handling kaldes af at åbne', !m1.kald.some(k => k[0] === 'teknologi_kolonne'));

  /* DNS-mærket og boksen "Vurdering". */
  const DNS_M = { slags: 'dns_rekognoscering', dag: '2026-11-02', stier: 300, normalt: 4.4, dns_navne: true, scanner: false, ny: true, netvaerk: null, procent: 0, verden: false, tek: null, angreb: 0, stoppet: 0, foreloebig: false };
  const OD = [{ ...OVERBLIK[1], moenster: DNS_M }];
  const VD = { dag: '2026-11-02', maal_ja: false, foreloebig: false, rekog_ja: false, maal: [], rekog: [], dns_ja: true, dns: [{ id: 'dns_nx', ok: false, n: 41, m: 100, navn: null, domaener: [] }, { id: 'dns_navne', ok: true, n: 300, m: 100, navn: 'vpn.a.wi24rd.com, mail2.a.wi24rd.com', domaener: [] }] };
  const o1 = side('#overblik', { ...MED_SIDEN, overblik: OD }), o2 = side('#overblik', { ...MED_SIDEN, overblik: OD }, 'en');
  await sov(120);
  const chipD = o1.d.querySelector('#overblik-raekker .moenster-maerke');
  tjek('DNS og DMARC: mærket står som tekst i rækken', chipD?.textContent === 'Ligner DNS-rekognoscering' && chipD.tagName === 'SPAN' && chipD.classList.contains('moenster-dns_rekognoscering'), chipD?.outerHTML.slice(0, 120));
  tjek('DNS og DMARC: mærkets forklaring', chipD?.title === 'Ligner DNS-rekognoscering: 300 forskellige navne, der ikke findes, på én dag (normalt 4).', chipD?.title);
  tjek('DNS-mærke: engelsk', o2.d.querySelector('#overblik-raekker .moenster-maerke')?.textContent === 'Looks like DNS reconnaissance');
  tjek('DNS og DMARC: ental og flertal', o1.w.hmSprog.moensterTekst({ ...DNS_M, stier: 1, normalt: 1 }).includes('1 navn, der ikke findes') && o2.w.hmSprog.moensterTekst({ ...DNS_M, stier: 1, dns_navne: false }).includes('1 lookup of non-existent names'));
  const kn = o1.w.hmTegn.moensterKnap(OD[0]), kq = new URLSearchParams(kn.dataset.dyk);
  tjek('DNS og DMARC: knappen åbner dagen uden et stifilter', kq.get('dag') === '2026-11-02' && kq.get('zone') === OD[0].id && !kq.has('forsoeg') && !kq.has('netvaerk'), kn.dataset.dyk);
  tjek('DNS og DMARC: guiden har sit eget afsnit', o1.w.hmSprog.bog('dns_rekognoscering').alle.includes('mange navne under dit domæne'));
  const v1 = side('#overblik', { ...MED_SIDEN, overblik: OD, vurdering: VD });
  await sov(120);
  const boks = v1.w.hmTegn.vurderingBoks({ zone: 'z1', navn: 'a.wi24rd.com', dag: '2026-11-02' });
  await sov(40);
  const li = id => boks.querySelector(`li[data-id="${id}"]`);
  tjek('DNS og DMARC: Vurdering viser ✗ og ✓ med tallene', li('dns_nx')?.textContent === '✗Ikke opfyldt: Opslag på navne, der ikke findes: 41 (skal være mindst 100)' && li('dns_navne')?.textContent === '✓Opfyldt: Forskellige navne, der ikke findes: 300 (skal være mindst 100), fx vpn.a.wi24rd.com, mail2.a.wi24rd.com', li('dns_nx')?.textContent + ' | ' + li('dns_navne')?.textContent);
  tjek('DNS og DMARC: Vurdering siger, om mærket står', [...boks.querySelectorAll('h4')].some(h => h.textContent === 'Ligner DNS-rekognoscering: står'));
  const vn = side('#overblik', { ...MED_SIDEN, overblik: OD, vurdering: { ...VD, dns_ja: false, dns: VD.dns.map(b => ({ ...b, ok: false, navn: null })) } });
  await sov(120);
  const boks2 = vn.w.hmTegn.vurderingBoks({ zone: 'z1', navn: 'a.wi24rd.com', dag: '2026-11-02' });
  await sov(40);
  tjek('DNS og DMARC: står ikke, når ingen af dem er opfyldt', [...boks2.querySelectorAll('h4')].some(h => h.textContent === 'Ligner DNS-rekognoscering: står ikke') && boks2.querySelectorAll('.vurd-nej').length === 2);
  tjek('DNS og DMARC: notifikationen gælder også DNS', o1.w.hmSprog.T.notifRekogNote.includes('Ligner DNS-rekognoscering'));

  /* DMARC. */
  const EM = { mails: 1200, fejlet: 41, afsendere: 2, hentede_dage: 30, fejl: null, liste: [{ afsender: 'Ond Afsender', land: 'RU', mails: 40, fejlet: 40 }, { afsender: '', land: '', mails: 9, fejlet: 1 }] };
  const em = x => RV({ email_oversigt: x });
  const e1 = side('#vaernet?zone=z1&navn=wi24rd.com', em(EM)), e2 = side('#vaernet?zone=z1&navn=wi24rd.com', em(EM), 'en'), e3 = side('#vaernet?zone=z1&navn=wi24rd.com', em({ ...EM, mails: 0, fejlet: 0, afsendere: 0, liste: [] }));
  const e4 = side('#vaernet?zone=z1&navn=wi24rd.com', em({ ...EM, mails: 0, fejlet: 0, afsendere: 0, liste: [], hentede_dage: 0 })), e5 = side('#vaernet?zone=z1&navn=wi24rd.com', em({ mails: 0, fejlet: 0, afsendere: 0, liste: [], hentede_dage: 0, fejl: 'does not have access to the field' }));
  const e6 = side('#vaernet?zone=z1&navn=wi24rd.com', em({ ...EM, fejlet: 0, afsendere: 0, liste: [] })), e7 = side('#vaernet?zone=z1&navn=wi24rd.com', RV()), e8 = side('#vaernet?samlet=1', RV({ email_oversigt: EM, vaern_samlet_cache: { vaern: V4, valgte: [{ id: 'z1', navn: 'a' }], i_alt: 1 } }));
  const e9 = side('#vaernet?zone=z1&navn=wi24rd.com', em({ ...EM, fejlet: 3, afsendere: 1, liste: EM.liste.slice(0, 1) }));
  await sov(200);
  const pl = x => x.d.querySelector('[data-plads="email"]');
  tjek('DNS og DMARC: linjen med tal', pl(e1)?.querySelector('.email-linje')?.textContent === 'Mails sendt i dit domænes navn: 1.200 · heraf ikke godkendt (DMARC fejlede): 41 fra 2 afsendere', pl(e1)?.textContent.slice(0, 200));
  tjek('DNS og DMARC: ental', pl(e9).querySelector('.email-linje').textContent.endsWith('3 fra 1 afsender'), pl(e9).querySelector('.email-linje').textContent);
  tjek('DNS og DMARC: engelsk', pl(e2).querySelector('.email-linje').textContent === 'Mail sent in your domain’s name: 1,200 · not approved (failed DMARC): 41 from 2 senders');
  const er = [...pl(e1).querySelectorAll('tbody tr')].map(r => [...r.cells].map(c => c.textContent).join('|'));
  tjek('DNS og DMARC: listen over afsendere, der fejlede', er.join(';') === 'Ond Afsender|RU|40|40;Ukendt afsender|–|1|9', er.join(';'));
  tjek('DNS og DMARC: rækkerne har intet bag sig, så de ser ikke klikbare ud', !pl(e1).querySelector('tbody button, tbody a, tbody [data-dyk]'));
  tjek('DNS og DMARC: listen ruller i sin egen boks og har et navn', pl(e1).querySelector('.vg-tabel-rul').getAttribute('aria-label') === 'Mails sendt i dit domænes navn');
  tjek('DNS og DMARC: ingen IP-adresser i teksten', !/\b\d{1,3}(\.\d{1,3}){3}\b/.test(pl(e1).textContent));
  tjek('DNS og DMARC: tom tilstand med Cloudflares ord og guide', pl(e3).querySelector('.email-tom').textContent === 'Cloudflare har ingen DMARC-rapporter for domænet — DMARC Management er måske ikke slået til.' && pl(e3).querySelector('details')?.textContent.includes('Email → DMARC Management') && pl(e3).querySelector('details')?.textContent.includes('Enable DMARC Management'), pl(e3).textContent.slice(0, 200));
  tjek('DNS og DMARC: ikke hentet endnu er noget andet end tomt', pl(e4).querySelector('.email-tom').textContent === 'DMARC-tallene er ikke hentet endnu.' && !pl(e4).querySelector('details'));
  tjek('DNS og DMARC: en fejl står ved afsnittet med Cloudflares ord', pl(e5).textContent.includes('Cloudflare svarede ikke på DMARC-tallene: does not have access to the field') && !pl(e5).querySelector('details'));
  tjek('DNS og DMARC: alle godkendt', pl(e6).querySelector('.email-linje').textContent === 'Mails sendt i dit domænes navn: 1.200 · alle godkendt' && !pl(e6).querySelector('table'));
  tjek('DNS og DMARC: fejler opslaget, står afsnittet ikke, og resten af Angreb gør', pl(e7) && !pl(e7).children.length && e7.d.querySelectorAll('.rettet-linje').length === 4);
  tjek('DNS og DMARC: kun for ét domæne', !pl(e8) && !e8.kald.some(k => k[0] === 'email_oversigt'));
  tjek('DNS og DMARC: spørger om domænet', e1.kald.find(k => k[0] === 'email_oversigt')[1].zone === 'z1');

  const DF = { dag: '2026-11-02', nye: ['Ny A/S'], stiger: true, procent: 40, normalt: 10 };
  const h1 = side('#overblik', { ...MED_SIDEN, overblik: [{ ...OVERBLIK[1], status: 'gul', dmarc: DF }] }), h2 = side('#overblik', { ...MED_SIDEN, overblik: [{ ...OVERBLIK[1], status: 'gul', dmarc: DF }] }, 'en');
  await sov(120);
  const ht5 = x => x.d.querySelector('#overblik-raekker .hv-tekst').textContent;
  tjek('DNS og DMARC: Hvorfor nævner DMARC', ht5(h1).includes('Mails i dit domænes navn fejlede DMARC fra en ny afsender: Ny A/S.') && ht5(h1).includes('40 % af mails i dit domænes navn fejlede DMARC (normalt 10 %).'), ht5(h1));
  tjek('DNS og DMARC: engelsk Hvorfor', ht5(h2).includes('Mail in your domain’s name failed DMARC from a new sender: Ny A/S.'), ht5(h2));
  tjek('DNS og DMARC: "Hvad gør jeg?" står, og guiden har afsnittet', h1.d.querySelector('#overblik-raekker .hv-guide') && h1.w.hmSprog.bog('dmarc').alle.includes('udgiver sig for dig'));
  const lib5 = fs.readFileSync(path.join(repo, 'src-tauri/src/lib.rs'), 'utf8'), bygg = fs.readFileSync(path.join(repo, 'src-tauri/build.rs'), 'utf8');
  tjek('DNS og DMARC: allow-email-oversigt i capabilities, build.rs og handleren', cap.permissions.includes('allow-email-oversigt') && bygg.includes('"email_oversigt",') && lib5.includes('            email_oversigt,'));

  /* Det faste ur. */
  urSkift = 400 * 864e5;
  const u1 = side('#overblik', { ...MED_SIDEN, overblik: OD });
  const skiftet = u1.w.eval('new Date().getTime()'), skiftetNu = u1.w.eval('Date.now()');
  urSkift = 0;
  tjek('Fast ur: siderne følger testens ur, ikke maskinens', Math.abs(skiftet - (NU_MS + 400 * 864e5)) < 3600e3 * 2 && Math.abs(skiftetNu - skiftet) < 60e3, new Date(skiftet).toISOString());
  tjek('Fast ur: uret går stadig', u1.w.eval('new Date(5).getTime()') === 5 && u1.w.eval('Date.UTC(2026, 0, 1)') === Date.UTC(2026, 0, 1));
  const kilde = fs.readFileSync(process.env.HM_KILDE || fileURLToPath(import.meta.url), 'utf8').split('\n').filter(l => !l.includes('/* uret */') && !l.includes('tjek(\'Fast ur')).join('\n');
  tjek('Fast ur: ingen test læser dagens dato uden om uret', !/new Date\(\)|\bDate\.now\(\)/.test(kilde.replace(/`<script>\(\(\)=>\{const D=Date[^`]*`/, '').replace(/static now\(\) \{ return urNu\(\); \}/, '').replace(/u1\.w\.eval\('[^']*'\)/g, '')), (kilde.match(/.*(new Date\(\)|\bDate\.now\(\)).*/) || [''])[0].slice(0, 120));
  }
}

console.log(`\n${ok} bestået, ${fejl} fejlet`);
process.exit(fejl ? 1 : 0);
