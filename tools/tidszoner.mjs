#!/usr/bin/env node
/* Bygger ui/assets/tidszoner.json (IANA-tidszone → landekode) fra tz-databasens zone.tab, som er public domain. Programmet bruger tabellen til
   at vælge land første gang (visning-land.js): ingen ny afhængighed og intet netværk. Kør igen med en nyere zone.tab, når tabellen skal
   opdateres: node tools/tidszoner.mjs [sti til zone.tab]. Med --check ændres intet, og forskelle giver exit 1. */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2), check = args.includes('--check');
const kilde = args.find(a => a !== '--check') || '/usr/share/zoneinfo/zone.tab';
const mappe = path.dirname(kilde), ud = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'ui', 'assets', 'tidszoner.json');
const version = fs.existsSync(path.join(mappe, '+VERSION')) ? fs.readFileSync(path.join(mappe, '+VERSION'), 'utf8').trim() : 'ukendt';
const zoner = {};
for (const linje of fs.readFileSync(kilde, 'utf8').split('\n')) {
  if (!linje || linje.startsWith('#')) continue;
  const [land, , zone] = linje.split('\t');
  zoner[zone] = land;
}
const sorteret = Object.fromEntries(Object.entries(zoner).sort(([a], [b]) => (a < b ? -1 : 1)));
const gammel = fs.existsSync(ud) ? JSON.parse(fs.readFileSync(ud, 'utf8')) : null;
if (check) {
  const ens = gammel && JSON.stringify(gammel.zoner) === JSON.stringify(sorteret);
  console.log(ens ? `ok ${path.basename(ud)}` : `FORSKEL ${path.basename(ud)}`);
  process.exit(ens ? 0 : 1);
}
const _kilde = `tz-databasens zone.tab (public domain), version ${version}, genereret ${new Date().toISOString().slice(0, 10)} med tools/tidszoner.mjs`;
fs.writeFileSync(ud, JSON.stringify({ _kilde, zoner: sorteret }, null, 0) + '\n');
console.log(`${path.basename(ud)}: ${Object.keys(sorteret).length} zoner, ${_kilde}`);
