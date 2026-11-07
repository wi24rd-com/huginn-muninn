// Sammenligner to fotobog-mapper på filstørrelse og SHA-256 af PNG-filerne og lister det, der er ændret.
// Brug: node tests/ui/fotobog-diff.mjs <gammel mappe> <ny mappe>. Exit 0: ens. Exit 1: forskelle. Exit 2: forkert brug.
// Hashen er på hele filen, og Chrome tegner ikke bit for bit ens: to kørsler af samme kode kan give "ændret" på omkring halvdelen af billederne (tekstmarkøren, kantudjævning).
// Sammenlign derfor størrelsen: støj flytter få bytes, en rigtig ændring normalt mange. Se selv på billedet, før et "ændret" tæller som en ændring.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';

const [gammel, ny] = process.argv.slice(2);
if (!gammel || !ny || ![gammel, ny].every(m => fs.existsSync(m) && fs.statSync(m).isDirectory())) { console.log('Brug: node tests/ui/fotobog-diff.mjs <gammel mappe> <ny mappe>'); process.exit(2); }
const laes = mappe => new Map(fs.readdirSync(mappe).filter(f => f.endsWith('.png')).map(f => { const b = fs.readFileSync(path.join(mappe, f)); return [f, { bytes: b.length, hash: createHash('sha256').update(b).digest('hex') }]; }));
const a = laes(gammel), b = laes(ny), navne = [...new Set([...a.keys(), ...b.keys()])].sort();
let ens = 0, aendret = 0, nye = 0, vaek = 0;
for (const f of navne) {
  const x = a.get(f), y = b.get(f);
  if (!x) { nye++; console.log(`ny      ${f}`); }
  else if (!y) { vaek++; console.log(`væk     ${f}`); }
  else if (x.hash === y.hash) ens++;
  else { aendret++; console.log(`ændret  ${f} (${x.bytes} → ${y.bytes} bytes)`); }
}
console.log(`${ens} ens, ${aendret} ændret, ${nye} nye, ${vaek} væk`);
process.exit(aendret + nye + vaek ? 1 : 0);
