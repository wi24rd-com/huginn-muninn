// PNG til webp med Chromes egen webp-koder, uden nye afhængigheder og uden netværk ud af maskinen (V-DOK.1).
// Samme opskrift som billederne i V-README.1: hver PNG tegnes på et lærred i headless Chrome fra en side på 127.0.0.1 og gemmes med
// canvas.toDataURL('image/webp', kvalitet). Chrome lægger et VP8X-hoved og en farveprofil (ICCP) i filen; de fjernes, så filen kun har én
// 'VP8 '-chunk og ingen metadata, som de andre billeder i docs/manual/img/. Billeddata røres ikke af det trin.
// Brug: node tools/png-til-webp.mjs [--kvalitet 0.9] <kilde.png>=<maal.webp> …
//   Kilden er fx et billede fra npm run test:foto (tests/ui/out/<dato>/fotobog/<id>--<tema>-<sprog>-<BxH>.png).
//   HM_CHROME peger på Chrome, som i fotobogen. Exit 1, hvis et billede ikke kan laves, eller hvis det ikke har samme størrelse som PNG-filen. Chrome får 5 sekunder pr. billede, mindst 2 minutter.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import { spawn } from 'node:child_process';

const args = process.argv.slice(2);
let kvalitet = 0.9;
const par = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--kvalitet') { kvalitet = Number(args[++i]); continue; }
  const [kilde, maal] = args[i].split('=');
  if (!kilde || !maal || !kilde.endsWith('.png') || !maal.endsWith('.webp')) { console.log(`Forstår ikke "${args[i]}": skriv <kilde.png>=<maal.webp>`); process.exit(1); }
  par.push({ kilde: path.resolve(kilde), maal: path.resolve(maal) });
}
if (!par.length || !(kvalitet > 0 && kvalitet <= 1)) { console.log('Brug: node tools/png-til-webp.mjs [--kvalitet 0.9] <kilde.png>=<maal.webp> …'); process.exit(1); }
for (const p of par) if (!fs.existsSync(p.kilde)) { console.log(`Findes ikke: ${p.kilde}`); process.exit(1); }
const CHROME = [process.env.HM_CHROME, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome', '/usr/bin/chromium'].find(f => f && fs.existsSync(f));
if (!CHROME) { console.log('Chrome findes ikke (sæt HM_CHROME)'); process.exit(1); }

/* PNG-filens størrelse står i IHDR (byte 16–23). */
const pngStr = b => (b.toString('latin1', 1, 4) === 'PNG' ? [b.readUInt32BE(16), b.readUInt32BE(20)] : null);
/* Kun 'VP8 '-chunken beholdes; RIFF-hovedet skrives forfra med den nye længde. Størrelsen læses i VP8-rammens hoved (startkoden 9d 01 2a). */
function kunVp8(b) {
  if (b.toString('latin1', 0, 4) !== 'RIFF' || b.toString('latin1', 8, 12) !== 'WEBP') throw new Error('ikke en webp-fil');
  const chunks = [];
  for (let o = 12; o + 8 <= b.length;) { const id = b.toString('latin1', o, o + 4), n = b.readUInt32LE(o + 4); chunks.push({ id, data: b.subarray(o + 8, o + 8 + n) }); o += 8 + n + (n & 1); }
  const vp8 = chunks.filter(c => c.id === 'VP8 ');
  if (vp8.length !== 1) throw new Error(`forventede én 'VP8 '-chunk, fik ${chunks.map(c => c.id).join(',')}`);
  const d = vp8[0].data, pad = d.length & 1 ? Buffer.alloc(1) : Buffer.alloc(0), hoved = Buffer.alloc(20);
  hoved.write('RIFF', 0, 'latin1'); hoved.writeUInt32LE(4 + 8 + d.length + pad.length, 4); hoved.write('WEBPVP8 ', 8, 'latin1'); hoved.writeUInt32LE(d.length, 16);
  if (d[3] !== 0x9d || d[4] !== 0x01 || d[5] !== 0x2a) throw new Error('VP8-rammen har ikke en startkode');
  return { fil: Buffer.concat([hoved, d, pad]), str: [d.readUInt16LE(6) & 0x3fff, d.readUInt16LE(8) & 0x3fff], fjernet: chunks.filter(c => c.id !== 'VP8 ').map(c => c.id) };
}

/* Siden tegner én PNG ad gangen og sender webp'en tilbage med POST; serveren svarer kun på 127.0.0.1 og kun på sine egne adresser. */
const SIDE = `<!doctype html><meta charset="utf-8"><script>
(async () => {
  const n = ${par.length}, q = ${kvalitet};
  for (let i = 0; i < n; i++) {
    let svar;
    try {
      const img = new Image(); img.src = '/png/' + i; await img.decode();
      const c = document.createElement('canvas'); c.width = img.naturalWidth; c.height = img.naturalHeight;
      c.getContext('2d').drawImage(img, 0, 0);
      svar = c.toDataURL('image/webp', q);
    } catch (e) { svar = 'FEJL ' + e; }
    await fetch('/webp/' + i, { method: 'POST', body: svar });
  }
})();
</script>`;
const modtaget = new Array(par.length);
let faerdig, antal = 0;
const alle = new Promise(r => { faerdig = r; });
const vaert = http.createServer((q, s) => {
  const m = /^\/(png|webp)\/(\d+)$/.exec(q.url), i = m ? Number(m[2]) : -1;
  if (q.url === '/') { s.writeHead(200, { 'Content-Type': 'text/html' }); return s.end(SIDE); }
  if (m && m[1] === 'png' && par[i]) { s.writeHead(200, { 'Content-Type': 'image/png' }); return s.end(fs.readFileSync(par[i].kilde)); }
  if (m && m[1] === 'webp' && q.method === 'POST' && par[i]) {
    const dele = [];
    q.on('data', d => dele.push(d));
    /* Tælles, ikke every(): every() springer de huller over, et array med længde har, og ville kalde det færdigt efter første billede. */
    q.on('end', () => { if (modtaget[i] === undefined) antal++; modtaget[i] = Buffer.concat(dele).toString('latin1'); s.end(); if (antal === par.length) faerdig(); });
    return;
  }
  s.writeHead(404); s.end();
});
await new Promise(r => vaert.listen(0, '127.0.0.1', r));
const profil = fs.mkdtempSync(path.join(os.tmpdir(), 'hm-webp-'));
const chrome = spawn(CHROME, ['--headless=new', '--disable-gpu', `--user-data-dir=${profil}`, '--no-first-run', '--remote-debugging-port=0', `http://127.0.0.1:${vaert.address().port}/`], { stdio: 'ignore' });
const GRAENSE = Math.max(120000, par.length * 5000), ur = setTimeout(() => faerdig('tid'), GRAENSE);
const udfald = await alle;
clearTimeout(ur);
chrome.kill('SIGKILL');
vaert.close();
fs.rmSync(profil, { recursive: true, force: true });
if (udfald === 'tid') { console.log(`Chrome blev ikke færdig inden for ${GRAENSE / 1000} sekunder`); process.exit(1); }

let fejl = 0;
par.forEach((p, i) => {
  const t = modtaget[i], ind = pngStr(fs.readFileSync(p.kilde));
  if (!t || !t.startsWith('data:image/webp;base64,')) { fejl++; console.log(`FEJL ${path.basename(p.kilde)}: ${t ? t.slice(0, 80) : 'intet svar'} (kan Chrome skrive webp?)`); return; }
  try {
    const r = kunVp8(Buffer.from(t.slice('data:image/webp;base64,'.length), 'base64'));
    if (!ind || r.str[0] !== ind[0] || r.str[1] !== ind[1]) throw new Error(`størrelsen er ${r.str.join('×')}, PNG-filen er ${ind ? ind.join('×') : '?'}`);
    fs.mkdirSync(path.dirname(p.maal), { recursive: true });
    fs.writeFileSync(p.maal, r.fil);
    console.log(`ok   ${path.basename(p.maal)} ${r.str.join('×')} ${r.fil.length} bytes (fjernet: ${r.fjernet.join(', ') || 'intet'})`);
  } catch (e) { fejl++; console.log(`FEJL ${path.basename(p.kilde)}: ${e.message}`); }
});
console.log(`${par.length - fejl} af ${par.length} webp-billeder lavet, kvalitet ${kvalitet}`);
process.exit(fejl ? 1 : 0);
