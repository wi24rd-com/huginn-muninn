// Wi24rd UI kit: design/wi24rd-tokens.json → ui/assets/tokens.css (Huginn and Muninn) og design/out/Wi24rdTokens.xaml
// (FTP, kun genereret). Ingen afhængigheder ud over Node selv.
//   node design/build-tokens.mjs          skriver begge filer
//   node design/build-tokens.mjs --check  sammenligner med filerne på disken (exit 1 ved forskel)
// Lagene: raw (rå skala) → semantic (light/dark) → component (fælles, pr. tema og pr. tæthed). En reference skrives
// {lag.sti}. I CSS bliver en reference til var(--…), så temaet kun skal skifte det semantiske lag; i XAML slås alt op.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TEMAER = ['light', 'dark'];
const REF = /^\{([^}]+)\}$/;

export function laesTokens(fil = path.join(REPO, 'design', 'wi24rd-tokens.json')) {
  return JSON.parse(fs.readFileSync(fil, 'utf8'));
}

const erBlad = v => typeof v !== 'object' || v === null || Array.isArray(v) || v.type === 'linear';
function blade(obj, sti = [], ud = []) {
  for (const [k, v] of Object.entries(obj)) {
    if (erBlad(v)) ud.push([[...sti, k], v]);
    else blade(v, [...sti, k], ud);
  }
  return ud;
}
const hent = (tokens, sti) => sti.split('.').reduce((o, k) => (o == null ? undefined : o[k]), tokens);

/* Navnet på CSS-variablen. raw beholder lagnavnet (--raw-color-blue-500); semantic.<tema>, component.shared,
   component.<tema> og component.density.<tæthed> taber deres præfiks (--color-bg, --band-height, --control-height). */
function cssNavn(sti) {
  const [lag, ...rest] = sti;
  if (lag === 'raw') return '--raw-' + rest.join('-');
  if (lag === 'semantic') return '--' + rest.slice(1).join('-');
  if (lag === 'component') return '--' + rest.slice(rest[0] === 'density' ? 2 : 1).join('-');
  throw new Error('ukendt lag: ' + sti.join('.'));
}

function enhed(tokens, sti, v) {
  if (typeof v !== 'number') return v;
  return sti.some(s => tokens.meta.unitless.includes(s)) ? String(v) : `${v}px`;
}

/* Én værdi til CSS: referencer bliver var(), gradienter linear-gradient(). */
function cssVaerdi(tokens, sti, v) {
  if (typeof v === 'string') {
    const m = v.match(REF);
    if (m) {
      if (hent(tokens, m[1]) === undefined) throw new Error(`${sti.join('.')}: reference findes ikke: ${v}`);
      return `var(${cssNavn(m[1].split('.'))})`;
    }
    return v;
  }
  if (v && v.type === 'linear') return `linear-gradient(${v.angle}deg, ${v.stops.map(([c, p]) => `${cssVaerdi(tokens, sti, c)} ${p}%`).join(', ')})`;
  return enhed(tokens, sti, v);
}

/* Slår en værdi op til bunds (til XAML og kontrasttesten). */
export function loes(tokens, v, sti = '') {
  for (let i = 0; i < 20 && typeof v === 'string' && REF.test(v); i++) {
    const ref = v.match(REF)[1], ny = hent(tokens, ref);
    if (ny === undefined) throw new Error(`${sti}: reference findes ikke: ${v}`);
    v = ny;
  }
  if (v && v.type === 'linear') return { ...v, stops: v.stops.map(([c, p]) => [loes(tokens, c, sti), p]) };
  return v;
}

/* Det semantiske lag + temaets komponentlag for ét tema, slået helt op: { 'color.bg': '#EEF4FB', … }. */
export function loesTema(tokens, tema) {
  const ud = {};
  for (const [sti, v] of blade(tokens.semantic[tema])) ud[sti.join('.')] = loes(tokens, v, sti.join('.'));
  for (const [sti, v] of blade(tokens.component[tema] || {})) ud[sti.join('.')] = loes(tokens, v, sti.join('.'));
  return ud;
}

function blok(tokens, vaelger, poster, indryk = '  ') {
  const set = new Set();
  const linjer = poster.map(([sti, v]) => {
    const navn = cssNavn(sti);
    if (set.has(navn)) throw new Error('dobbelt CSS-navn: ' + navn);
    set.add(navn);
    return `${indryk}  ${navn}: ${cssVaerdi(tokens, sti, v)};`;
  });
  return `${indryk}${vaelger} {\n${linjer.join('\n')}\n${indryk}}`;
}

export function bygCss(tokens) {
  const raw = blade(tokens.raw, ['raw']);
  const faelles = blade(tokens.component.shared, ['component', 'shared']);
  const tema = t => [...blade(tokens.semantic[t], ['semantic', t]), ...blade(tokens.component[t] || {}, ['component', t])];
  const taethed = d => blade(tokens.component.density[d], ['component', 'density', d]);
  const standard = tokens.meta.targets.css.density;
  // Samme navn må ikke komme fra to lag (fx semantic band.* og component band.*).
  const alle = new Map();
  for (const [sti] of [...raw, ...faelles, ...tema('light'), ...taethed(standard)]) {
    const n = cssNavn(sti);
    if (alle.has(n)) throw new Error(`${n} kommer fra både ${alle.get(n)} og ${sti.join('.')}`);
    alle.set(n, sti.join('.'));
  }
  const ud = [
    `/* GENERERET af design/build-tokens.mjs fra design/wi24rd-tokens.json (${tokens.meta.name} ${tokens.meta.version}). Ret ikke i hånden:`,
    '   ret JSON-filen og kør node design/build-tokens.mjs. Lys er standard; mørk følger macOS/Windows (prefers-color-scheme),',
    `   når <html data-tema="system">; <html data-tema="moerk"> vælger mørk selv (Wi24rd UI Kit 2's tema.js sætter data-tema). Målene er faste (${standard}):
   tekststørrelsen i tre trin ejes af Wi24rd UI Kit 2 (<html data-tekst>, zoom på roden i kit.css). */`,
    blok(tokens, ':root', [...raw, ...faelles], ''),
    blok(tokens, ':root', taethed(standard), ''),
    blok(tokens, ':root', [[['semantic', 'light', 'color-scheme'], 'light'], ...tema('light')], '').replace('--color-scheme: light;', 'color-scheme: light;'),
    `@media (prefers-color-scheme: dark) {\n${blok(tokens, ':root[data-tema="system"]', [[['semantic', 'dark', 'color-scheme'], 'dark'], ...tema('dark')]).replace('--color-scheme: dark;', 'color-scheme: dark;')}\n}`,
    blok(tokens, ':root[data-tema="moerk"]', [[['semantic', 'dark', 'color-scheme'], 'dark'], ...tema('dark')], '').replace('--color-scheme: dark;', 'color-scheme: dark;'),
  ];
  return ud.join('\n') + '\n';
}

/* ── XAML (WPF ResourceDictionary) ── */
const pascal = s => s.split('-').map(d => d.charAt(0).toUpperCase() + d.slice(1)).join('');
const xamlNoegle = sti => sti.map(pascal).join('.');
function argb(c) {
  if (/^#[0-9a-f]{6}$/i.test(c)) return '#FF' + c.slice(1).toUpperCase();
  const m = c.match(/^rgba\(\s*(\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\s*\)$/);
  if (!m) throw new Error('ukendt farve: ' + c);
  const h = n => Number(n).toString(16).padStart(2, '0').toUpperCase();
  return '#' + h(Math.round(Number(m[4]) * 255)) + h(m[1]) + h(m[2]) + h(m[3]);
}
const erFarve = v => typeof v === 'string' && (/^#[0-9a-f]{6}$/i.test(v) || /^rgba\(/.test(v));
const CSS_ALENE = new Set(['-apple-system', 'BlinkMacSystemFont', 'system-ui', 'sans-serif', 'monospace', 'SFMono-Regular', 'Menlo', 'Avenir Next']);
function xamlPost(noegle, v) {
  if (v && v.type === 'linear') {
    const r = v.angle * Math.PI / 180, dx = Math.sin(r) / 2, dy = -Math.cos(r) / 2, f = n => String(Math.round(n * 1000) / 1000);
    const stops = v.stops.map(([c, p]) => `    <GradientStop Color="${argb(c)}" Offset="${p / 100}"/>`).join('\n');
    return `  <LinearGradientBrush x:Key="${noegle}" StartPoint="${f(0.5 - dx)},${f(0.5 - dy)}" EndPoint="${f(0.5 + dx)},${f(0.5 + dy)}">\n${stops}\n  </LinearGradientBrush>`;
  }
  if (erFarve(v)) return `  <SolidColorBrush x:Key="${noegle}" Color="${argb(v)}"/>`;
  if (typeof v === 'number') return `  <sys:Double x:Key="${noegle}">${v}</sys:Double>`;
  if (typeof v === 'string' && v.includes(',')) {
    const navne = v.split(',').map(s => s.trim().replace(/"/g, '')).filter(s => !CSS_ALENE.has(s));
    return `  <FontFamily x:Key="${noegle}">${navne.join(', ')}</FontFamily>`;
  }
  return `  <!-- ${noegle}: kun CSS (${String(v).replace(/--/g, '- -')}) -->`;
}

export function bygXaml(tokens) {
  const d = tokens.meta.targets.xaml.density;
  const poster = [];
  for (const [sti, v] of blade(tokens.raw)) if (erFarve(v)) poster.push(`  <Color x:Key="${xamlNoegle(['raw', ...sti])}">${argb(v)}</Color>`);
  for (const t of TEMAER) {
    for (const [sti, v] of blade(tokens.semantic[t])) poster.push(xamlPost(xamlNoegle([t, ...sti]), loes(tokens, v)));
    for (const [sti, v] of blade(tokens.component[t] || {})) poster.push(xamlPost(xamlNoegle([t, ...sti]), loes(tokens, v)));
  }
  for (const [sti, v] of blade(tokens.component.shared)) poster.push(xamlPost(xamlNoegle(sti), loes(tokens, v)));
  for (const [sti, v] of blade(tokens.component.density[d])) poster.push(xamlPost(xamlNoegle(sti), loes(tokens, v)));
  return [
    `<!-- GENERERET af design/build-tokens.mjs fra design/wi24rd-tokens.json (${tokens.meta.name} ${tokens.meta.version}). Ret ikke i hånden. -->`,
    `<!-- Tæthed: ${d}. Temaer som præfiks (Light./Dark.). Endnu ikke brugt af FTP. -->`,
    '<ResourceDictionary xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"',
    '                    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"',
    '                    xmlns:sys="clr-namespace:System;assembly=mscorlib">',
    ...poster,
    '</ResourceDictionary>',
  ].join('\n') + '\n';
}

export function byg(tokens = laesTokens()) {
  return {
    [tokens.meta.targets.css.file]: bygCss(tokens),
    [tokens.meta.targets.xaml.file]: bygXaml(tokens),
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const check = process.argv.includes('--check');
  let forskel = 0;
  for (const [fil, indhold] of Object.entries(byg())) {
    const fuld = path.join(REPO, fil);
    if (check) {
      const paa = fs.existsSync(fuld) ? fs.readFileSync(fuld, 'utf8') : null;
      if (paa !== indhold) { forskel++; console.log(`FORSKEL ${fil}`); } else console.log(`ok ${fil}`);
    } else {
      fs.mkdirSync(path.dirname(fuld), { recursive: true });
      fs.writeFileSync(fuld, indhold);
      console.log(`skrev ${fil} (${indhold.length} tegn)`);
    }
  }
  process.exit(forskel ? 1 : 0);
}
