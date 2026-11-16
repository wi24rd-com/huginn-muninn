// Fotobog: et skærmbillede af hver visning i sider.mjs, i headless Chrome, uden at starte programmet og uden netværk ud af maskinen.
// Brug: npm run test:foto. Matrix: tema × sprog × størrelse; HM_FOTO_MATRIX=lille giver kun light/da/1200×860.
// Output: tests/ui/out/<dato>/fotobog/<id>--<tema>-<sprog>-<BxH>.png + manifest.json. HM_FOTO_UD overstyrer mappen.
// HM_FOTO_SIDER vælger sider efter id, kommasepareret; * til sidst er et præfiks (fx HM_FOTO_SIDER='konti-*').
// En side kan have fokus: en vælger, der skal have fokus og stå helt inden for sin dialogs rulleflade, når billedet tages.
// HM_CHROME peger på Chrome, HM_REPO på en anden kopi af repoet, HM_NU sætter uret (som i ui-roeg.mjs).
// Hver visning hentes to gange: første gang tager billedet, anden gang (--dump-dom) henter fejlene ud og prøver luk. Exit 1 ved fejl, eller hvis et billede mangler eller er under 5 KB.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import { fileURLToPath } from 'node:url';
import { spawn, spawnSync } from 'node:child_process';

const repo = process.env.HM_REPO || path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
process.env.HM_REPO = repo;
let data = await import('./sider.mjs');
const kopi = data.UR_AAR !== 2026 ? data.siderIAar() : null; /* fast år */
if (kopi) data = await import(kopi);
const { svar, UR_STUB, BES_NOEGLE } = data;
const valg = (process.env.HM_FOTO_SIDER || '').split(',').map(x => x.trim()).filter(Boolean);
const SIDER = valg.length ? data.SIDER.filter(s => valg.some(v => (v.endsWith('*') ? s.id.startsWith(v.slice(0, -1)) : s.id === v))) : data.SIDER;
if (!SIDER.length) { console.log('HM_FOTO_SIDER rammer ingen side'); process.exit(1); }

const CHROME = [process.env.HM_CHROME, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome', '/usr/bin/chromium'].find(f => f && fs.existsSync(f));
if (!CHROME) { console.log('Chrome findes ikke (sæt HM_CHROME)'); process.exit(1); }
const MINDST = 5 * 1024, VENT = 4500, TID = 120000;
const lille = process.env.HM_FOTO_MATRIX === 'lille';
const TEMAER = lille ? ['light'] : ['light', 'dark'], SPROG = lille ? ['da'] : ['da', 'en'], STOERRELSER = lille ? [[1200, 860]] : [[1200, 860], [1100, 560]];

/* Mappen hedder efter kørslens rigtige dato, ikke testens ur: billederne skal kunne findes igen. */
const nu = new Date(), to = n => String(n).padStart(2, '0'), dato = `${nu.getFullYear()}-${to(nu.getMonth() + 1)}-${to(nu.getDate())}`;
const ud = path.resolve(process.env.HM_FOTO_UD || path.join(repo, 'tests', 'ui', 'out', dato, 'fotobog'));
fs.mkdirSync(ud, { recursive: true });
const profiler = fs.mkdtempSync(path.join(os.tmpdir(), 'hm-foto-'));
const linje = (cmd, ...a) => { const r = spawnSync(cmd, a, { encoding: 'utf8' }); return r.status === 0 ? r.stdout.trim() : null; };

/* Fanger fejl fra første linje, før noget af programmet er kørt. CSP-brud og filer, der ikke kan hentes, tæller også: så ser billedet forkert ud uden at kaste. */
const FANG = `(()=>{window.__HM_FEJL__=[];const f=(k,a)=>__HM_FEJL__.push((k+': '+a.map(x=>(x&&x.stack)||(x&&typeof x==='object'?JSON.stringify(x):String(x))).join(' ')).slice(0,500));
window.onerror=(m,fil,l,c)=>{f('onerror',[m,fil,l,c])};addEventListener('unhandledrejection',e=>f('unhandledrejection',[e.reason]));
addEventListener('error',e=>{if(e.target!==window)f('ressource',[e.target.src||e.target.href||e.target.tagName])},true);
addEventListener('securitypolicyviolation',e=>f('csp',[e.effectiveDirective,e.blockedURI]));
for(const m of['error','warn']){const o=console[m];console[m]=(...a)=>{f('console.'+m,a);o.apply(console,a)}}})();`;

/* Samme stub som margen-testen, men som fil: siden får appens CSP som header, og den stopper inline-script. siden_sidst.vis er falsk som dér: ellers springer rammen til Siden sidst ved start, og alle billeder bliver det samme. */
/* Versionen i fodlinjen og under Om er programmets egen (tauri.conf.json), så billederne i vejledningen viser den udgave, de beskriver. */
const VERSION = JSON.parse(fs.readFileSync(path.join(repo, 'src-tauri', 'tauri.conf.json'), 'utf8')).version;
const stub = (post, tema, sprog, tjek) => `${UR_STUB.replace(/<\/?script>/g, '')}${FANG}localStorage.setItem('hm-sprog',${JSON.stringify(sprog)});localStorage.setItem('hm-tekst','normal');localStorage.setItem('hm-tema',${JSON.stringify(tema)});window.__HM_PLATFORM__='macos';window.__HM_POST__=${JSON.stringify(post)};window.__HM_TJEK__=${tjek};const SVAR=${JSON.stringify({ ...svar, siden_sidst: { ...svar.siden_sidst, vis: false }, ...post.ekstra })};window.__TAURI__={core:{invoke:async(c,a)=>{const k=c==='dyk'&&a.filtre.some(f=>f.dim==='regel')?'dyk_regel':${BES_NOEGLE};return k in SVAR?JSON.parse(JSON.stringify(SVAR[k])):null}},event:{listen:async()=>()=>{}},app:{getVersion:async()=>${JSON.stringify(VERSION)}}};`;

/* aabn klikkes før billedet, som i CSP-blokken. luk klikkes kun i anden kørsel: ellers viser billedet af hjælpen ikke hjælpen.
   Anden kørsel venter næsten lige så længe som billedet og tjekker så, at det er den rigtige visning, og at vælgerne stadig rammer: ellers viser billedet noget andet end sit navn, uden at noget fejler. */
const GANG = `addEventListener('load', async () => {
  const sov = ms => new Promise(r => setTimeout(r, ms)), v = window.__HM_POST__, F = window.__HM_FEJL__;
  await sov(350);
  /* Et trin i aabn er en vælger, der klikkes, eller [vælger, tekst], der skrives i et felt (V-KONTI.1 FIX 1: Tilføj API Token…). */
  for (const a of v.aabn || []) { const [s, tekst] = Array.isArray(a) ? a : [a, null], e = document.querySelector(s); if (!e) { F.push('gennemløb: aabn rammer ikke ' + s); continue; } if (tekst == null) e.click(); else e.value = tekst; await sov(tekst == null ? 120 : 0); }
  if (v.rul) setTimeout(() => { const e = document.querySelector(v.rul); if (e) e.scrollIntoView({ block: 'start' }); }, 1500);
  if (!window.__HM_TJEK__) return;
  await sov(${VENT});
  /* rulTekst: rul må ramme en linje tekst (fx en note) i stedet for et afsnit med overskrift; den skal bare findes. */
  if (v.rul && !document.querySelector(v.rul + ' h2') && !(v.rulTekst && document.querySelector(v.rul))) F.push('gennemløb: rul rammer ikke et afsnit: ' + v.rul);
  const set = [...document.querySelectorAll('section.visning:not([hidden])')].map(e => e.dataset.visning).join(), vil = v.visning || v.hash.split('?')[0];
  if (set !== vil) F.push('gennemløb: viser ' + (set || 'ingenting') + ', ikke ' + vil);
  if (v.rod) { const e = document.querySelector(v.rod); if (!e || !e.checkVisibility()) F.push('gennemløb: rod er ikke synlig: ' + v.rod); }
  /* fokus: elementet har fokus og står helt inden for sin dialogs rulleflade (V3: feltet under kanten ved 1100×560). */
  if (v.fokus) {
    const e = document.querySelector(v.fokus);
    if (!e || document.activeElement !== e) F.push('gennemløb: fokus er ikke på ' + v.fokus + ' men på ' + (document.activeElement && (document.activeElement.id || document.activeElement.tagName)));
    else { const k = (e.closest('.modal-indhold') || document.documentElement).getBoundingClientRect(), r = e.getBoundingClientRect(); if (r.top < k.top || r.bottom > k.bottom) F.push('gennemløb: ' + v.fokus + ' står uden for kanten (' + Math.round(r.top) + '–' + Math.round(r.bottom) + ' i ' + Math.round(k.top) + '–' + Math.round(k.bottom) + ')'); }
  }
  if (v.luk) { const e = document.querySelector(v.luk); if (e) { e.click(); await sov(120); } else F.push('gennemløb: luk rammer ikke ' + v.luk); }
  document.body.textContent = 'MAALT:' + JSON.stringify(F) + ':SLUT';
});`;

const CSP = JSON.parse(fs.readFileSync(path.join(repo, 'src-tauri', 'tauri.conf.json'), 'utf8')).app.security.csp;
const typer = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.png': 'image/png', '.woff2': 'font/woff2' };
const uiMappe = path.join(repo, 'ui'), index = fs.readFileSync(path.join(uiMappe, 'index.html'), 'utf8'), genereret = new Map([['foto-gang.js', GANG]]);
const vaert = http.createServer((q, s) => {
  const sti = decodeURIComponent(new URL(q.url, 'http://x').pathname).slice(1), f = path.join(uiMappe, sti), g = genereret.get(sti);
  if (g === undefined && (!f.startsWith(uiMappe + path.sep) || !fs.existsSync(f))) { s.writeHead(404); return s.end(); }
  s.writeHead(200, { 'Content-Type': typer[path.extname(sti)] || 'application/octet-stream', 'Content-Security-Policy': CSP });
  s.end(g === undefined ? fs.readFileSync(f) : g);
});
await new Promise(r => vaert.listen(0, '127.0.0.1', r));

/* TZ=UTC som i ui-roeg, så datoerne på siderne står ens på alle maskiner. */
const chrome = (profil, args) => new Promise(ok => {
  let tekst = '';
  const p = spawn(CHROME, ['--headless=new', '--disable-gpu', '--hide-scrollbars', `--user-data-dir=${profil}`, '--virtual-time-budget=6000', ...args], { env: { ...process.env, TZ: 'UTC' }, stdio: ['ignore', 'pipe', 'ignore'] });
  const ur = setTimeout(() => p.kill('SIGKILL'), TID);
  p.stdout.on('data', x => { tekst += x; });
  p.on('close', () => { clearTimeout(ur); ok(tekst); });
});

const side = (n, post, tema, sprog, tjek) => {
  const navn = `foto-${n}-${tjek ? 'tjek' : 'billede'}`;
  genereret.set(`${navn}-stub.js`, stub(post, tema, sprog, tjek));
  genereret.set(`${navn}.html`, index.replace('<script src="assets/gamle-valg.js"></script>', `<script src="${navn}-stub.js"></script><script src="assets/gamle-valg.js"></script>`).replace('</body>', '<script src="foto-gang.js"></script></body>'));
  return `http://127.0.0.1:${vaert.address().port}/${navn}.html#${post.hash}`;
};

const opgaver = SIDER.flatMap(post => TEMAER.flatMap(tema => SPROG.flatMap(sprog => STOERRELSER.map(([bredde, hoejde]) => ({ post, tema, sprog, bredde, hoejde })))));
const billeder = new Array(opgaver.length);
const tag = async ({ post, tema, sprog, bredde, hoejde }, n) => {
  const fil = `${post.id}--${tema}-${sprog}-${bredde}x${hoejde}.png`, sti = path.join(ud, fil), profil = path.join(profiler, String(n)), vindue = `--window-size=${bredde},${hoejde}`, fejl = [];
  fs.rmSync(sti, { force: true }); /* et billede fra en tidligere kørsel må ikke tælle som dette */
  await chrome(profil, [`--screenshot=${sti}`, vindue, side(n, post, tema, sprog, false)]);
  const m = /MAALT:(.*):SLUT/s.exec(await chrome(profil, [vindue, '--dump-dom', side(n, post, tema, sprog, true)]));
  for (const k of [`foto-${n}-billede`, `foto-${n}-tjek`]) { genereret.delete(`${k}.html`); genereret.delete(`${k}-stub.js`); }
  if (m) fejl.push(...JSON.parse(m[1].replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&'))); else fejl.push('gennemløb: siden svarede ikke');
  const png = fs.existsSync(sti) ? fs.readFileSync(sti) : null;
  if (!png) fejl.push('billedet mangler'); else if (png.length < MINDST) fejl.push(`billedet er kun ${png.length} bytes`);
  else if (png.readUInt32BE(16) !== bredde || png.readUInt32BE(20) !== hoejde) fejl.push(`billedet er ${png.readUInt32BE(16)}×${png.readUInt32BE(20)}, ikke ${bredde}×${hoejde}`);
  billeder[n] = { id: post.id, navn: post.navn, hash: post.hash, tema, sprog, bredde, hoejde, fil, fejl };
};

let naeste = 0;
const arbejder = async () => { while (naeste < opgaver.length) { const n = naeste++; await tag(opgaver[n], n); } };
await Promise.all(Array.from({ length: Math.max(2, Math.min(6, os.availableParallelism() >> 1)) }, arbejder));
vaert.close();
fs.rmSync(profiler, { recursive: true, force: true });
if (kopi) fs.rmSync(path.dirname(fileURLToPath(kopi)), { recursive: true, force: true });

fs.writeFileSync(path.join(ud, 'manifest.json'), JSON.stringify({ dato, commit: linje('git', '-C', repo, 'log', '-1', '--format=%h'), chrome: linje(CHROME, '--version'), billeder }, null, 2) + '\n');
const daarlige = billeder.filter(b => b.fejl.length);
for (const b of daarlige) console.log(`FEJL ${b.fil}: ${b.fejl.slice(0, 3).join(' | ')}`);
console.log(`${billeder.length} billeder, ${daarlige.length} med fejl, mappe: ${ud}`);
process.exit(daarlige.length ? 1 : 0);
