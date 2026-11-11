// Wi24rd UI Kit 2: kopierne i ui/assets/kit2/ skal være byte for byte som pakken. Rettes aldrig i hånden.
//   node tools/kit2-tjek.mjs            tjekker kopierne mod PAKKE.json (og mod pakken ved siden af, hvis den findes)
//   node tools/kit2-tjek.mjs --skriv    skriver PAKKE.json ud fra kopierne (efter en ny pakke er kopieret ind)
// Pakken ligger i kittets kilde-repo (../Grimnir/design/ui-kit-2/pakke); den læses kun. Ingen afhængigheder ud over Node.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const KIT = path.join(REPO, 'ui', 'assets', 'kit2');
const PAKKE = path.resolve(REPO, '..', 'Grimnir', 'design', 'ui-kit-2', 'pakke');
/* Kopi → hvor filen ligger i pakken. */
const kilde = f => (f.startsWith('fonts/') ? 'web/' + f : f === 'tokens.css' ? 'tokens/tokens.css' : /\.(css|js)$/.test(f) ? 'web/' + f : f);
const sha = buf => crypto.createHash('sha256').update(buf).digest('hex');

function filer(dir = KIT, rod = '') {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap(e => (e.isDirectory() ? filer(path.join(dir, e.name), rod + e.name + '/') : [rod + e.name]))
    .filter(f => f !== 'PAKKE.json').sort();
}

export function tjek() {
  const fejl = [], noter = [];
  const manifestFil = path.join(KIT, 'PAKKE.json');
  if (!fs.existsSync(manifestFil)) return { fejl: ['ui/assets/kit2/PAKKE.json mangler'], noter, version: null };
  const m = JSON.parse(fs.readFileSync(manifestFil, 'utf8'));
  const paa = filer();
  for (const f of Object.keys(m.filer)) if (!paa.includes(f)) fejl.push(`mangler: ${f}`);
  for (const f of paa) {
    if (!(f in m.filer)) { fejl.push(`ikke i PAKKE.json: ${f}`); continue; }
    if (sha(fs.readFileSync(path.join(KIT, f))) !== m.filer[f]) fejl.push(`ændret i forhold til PAKKE.json: ${f}`);
  }
  if (fs.readFileSync(path.join(KIT, 'VERSION'), 'utf8').trim() !== m.version) fejl.push('VERSION passer ikke med PAKKE.json');
  if (fs.existsSync(path.join(PAKKE, 'VERSION'))) {
    const v = fs.readFileSync(path.join(PAKKE, 'VERSION'), 'utf8').trim();
    if (v !== m.version) noter.push(`pakken ved siden af er version ${v}; kopierne er ${m.version}`);
    else for (const f of paa) {
      const p = path.join(PAKKE, kilde(f));
      if (!fs.existsSync(p)) fejl.push(`findes ikke i pakken: ${kilde(f)}`);
      else if (!fs.readFileSync(p).equals(fs.readFileSync(path.join(KIT, f)))) fejl.push(`afviger fra pakken: ${f}`);
    }
  } else noter.push('pakken ved siden af findes ikke; kun PAKKE.json er tjekket');
  return { fejl, noter, version: m.version, antal: paa.length };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.includes('--skriv')) {
    const version = fs.readFileSync(path.join(KIT, 'VERSION'), 'utf8').trim();
    const ud = { kit: 'Wi24rd UI Kit 2', version, filer: Object.fromEntries(filer().map(f => [f, sha(fs.readFileSync(path.join(KIT, f)))])) };
    fs.writeFileSync(path.join(KIT, 'PAKKE.json'), JSON.stringify(ud, null, 2) + '\n');
    console.log(`PAKKE.json skrevet: version ${version}, ${Object.keys(ud.filer).length} filer`);
  } else {
    const r = tjek();
    r.noter.forEach(n => console.log('note ' + n));
    r.fejl.forEach(f => console.log('FEJL ' + f));
    if (!r.fejl.length) console.log(`ok   kit2: ${r.antal} filer som pakke ${r.version}`);
    process.exit(r.fejl.length ? 1 : 0);
  }
}
