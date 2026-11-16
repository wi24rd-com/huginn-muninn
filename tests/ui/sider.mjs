// Testdata og listen over visninger, fælles for ui-roeg.mjs og fotobog.mjs. Filen må ikke selv afhænge af noget uden for Node:
// siderIAar() kopierer den til en midlertidig mappe. HM_REPO peger på en anden kopi af repoet, HM_NU sætter uret (se ui-roeg.mjs).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const repo = process.env.HM_REPO || path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const ui = p => fs.readFileSync(path.join(repo, 'ui', p), 'utf8');

/* Fast ur: ingen test må afhænge af dagens dato. HM_NU sætter det (samme variabel som Rust-testene), og det går videre derfra, så ventetider stadig virker. Alle datoer i testene og i siderne kommer herfra. */
export const NU_MS = Date.parse(process.env.HM_NU || '2026-11-06T12:00:00Z');
export const UR_AAR = new Date(NU_MS).getUTCFullYear();
export const UR_STUB = `<script>(()=>{const D=Date,n=${NU_MS},s=D.now();Date=class extends D{constructor(...a){if(a.length)super(...a);else super(n+D.now()-s)}static now(){return n+D.now()-s}}})();</script>`;
/* Med et ur i et andet år flytter testdataene med, så datoerne står i samme forhold til uret: de er skrevet med 2026, og et andet år skal vises med årstal. */
export const flytAar = kilde => kilde.split('\n').map(l => (l.includes('/* fast år */') ? l : l.replace(/\b2026\b/g, String(UR_AAR)))).join('\n'); /* fast år */
/* Denne fil med årstallet flyttet. Svarer med adressen til kopien; den, der kalder, sletter mappen og skal have sat HM_REPO. */
export const siderIAar = () => { const fil = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'hm-sider-')), 'sider.mjs'); fs.writeFileSync(fil, flytAar(fs.readFileSync(fileURLToPath(import.meta.url), 'utf8'))); return pathToFileURL(fil).href; };

// ── Data ──
export const NU_ISO = '2026-11-03T08:01:00.000Z';
export const raekke = (id, navn, status, x = {}) => Object.assign({ id, navn, status, gammel: false, i_gaar_dag: '2026-11-02', status_dag: '2026-11-02', uge_fra: '2026-10-27', ugen_foer_fra: '2026-10-20', ugen_foer_til: '2026-10-26', i_gaar: 0, i_dag_hidtil: 0, hele7: 0, hele30: 0, ugen_foer: 0, alle_hele7: 0, fordeling: { d30: {}, i_gaar: {} }, aendring: null, angreb_i_gaar: 0, angreb_snit: 0, sidst_hentet: null, doegn_faerdige: 0, doegn_i_alt: 31, opdateres: false, angreb_i_gaar_top: [], angreb_fordeling: { blokeret: 0, udfordret: 0, labyrint: 0, andet: 0 }, angreb_uge: 0, angreb_ugen_foer: 0 }, x);
export const OVERBLIK = [
  raekke('z1', 'wi24rd.com', 'gul', { i_gaar: 1260, hele7: 3408, hele30: 8200, ugen_foer: 2400, aendring: 25, angreb_i_gaar: 30, angreb_snit: 12.4, sidst_hentet: NU_ISO, doegn_faerdige: 30,
    angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 20 }, { navn: 'Wordpress', n: 7 }, { navn: 'min-regel: max 100 kald pr. minut pr. IP (20 pr. 10 s)', n: 2, egen: true }],
    angreb_fordeling: { blokeret: 30, udfordret: 0, labyrint: 0, andet: 0 }, angreb_uge: 45, angreb_ugen_foer: 36 }),
  raekke('z2', 'xyz.wi24rd.com', 'groen', { i_gaar: 104, hele7: 479, hele30: 2844, ugen_foer: 774, aendring: -33, angreb_i_gaar: 28, angreb_snit: 23.1, sidst_hentet: NU_ISO, doegn_faerdige: 30,
    angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 25 }, { navn: 'Browser Integrity Check', n: 3 }],
    angreb_fordeling: { blokeret: 20, udfordret: 8, labyrint: 0, andet: 0 }, angreb_uge: 140, angreb_ugen_foer: 210 }),
  raekke('z3', 'abe.wi24rd.com', 'graa'),
];
export const dage = Array.from({ length: 30 }, (_, i) => ({ dag: new Date(Date.UTC(2026, 9, 5 + i)).toISOString().slice(0, 10), n: i, alle: 100 * i }));
/* Regelkortet med felterne ved siden af W's. */
export const kortRegel = (r, x) => Object.assign({ art: 'angreb', h7: r.n7, h30: r.n30, faelder30: 0 }, r, x);
export const vaernUd = x => Object.assign({
  genereret: '2026-11-03T06:54:00.000Z', perioder: { d1: 14, d7: 2782, d30: 8200 }, alle: { d7: 21665, d30: 90000 }, dage,
  regler: [{ navn: 'React - RCE - CVE:CVE-2025-55182', produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', n7: 20, n30: 80, foerst: '2026-10-05', sidst: '2026-11-02', stier: [{ sti: '/', n: 80 }], lande: [{ kode: 'US', n: 80 }] }],
  lande: [{ kode: 'US', n: 80 }], netvaerk: [{ navn: 'Google LLC', n: 80 }], stier: [{ sti: '/', n: 80 }], colos: [{ kode: 'EWR', n: 80 }], buer: [{ land: 'US', colo: 'EWR', n: 80 }],
  metoder: { POST: 80 }, kilder: { firewallManaged: 80 }, forsoeg: { i7: 5849, kategorier: [] },
  seneste: [{ t: '2026-11-03T05:00:00Z', regel: 'R', cve: null, land: 'US', sti: '/', colo: 'EWR', metode: 'GET', domaene: 'wi24rd.com' }, { t: '2026-11-03T04:00:00Z', regel: 'R', cve: null, land: 'DK', sti: '/a', colo: 'CPH', metode: 'GET', domaene: 'xyz.wi24rd.com' }],
  i_gaar: 1373, i_dag_hidtil: 58, hele7: 3816, hele30: 11006, ugen_foer: 3117, alle_hele7: 30513,
  fordeling: { d30: { blokeret: 676, udfordret: 4278, labyrint: 6124, andet: 0, faelder: 0 }, i_gaar: { blokeret: 30, udfordret: 134, labyrint: 1200, andet: 0, faelder: 0 } },
  h_dage: dage, h_lande: [{ kode: 'US', n: 80 }], h_netvaerk: [{ navn: 'Google LLC', n: 80 }], h_colos: [{ kode: 'EWR', n: 80 }], h_buer: [{ land: 'US', colo: 'EWR', n: 80 }], h_buer_i_alt: 1, h_idag: '2026-11-03',
  h_forsoeg: (x && x.forsoeg) || { i7: 5849, kategorier: [{ id: 'hemmeligheder', n: 5849, stier: [{ sti: '/.env', n: 5849 }] }] },
}, x, { h_regler: (x && x.h_regler) || ((x && x.regler) || [{ navn: 'React - RCE - CVE:CVE-2025-55182', produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', n7: 20, n30: 80, foerst: '2026-10-05', sidst: '2026-11-02', stier: [{ sti: '/', n: 80 }], lande: [{ kode: 'US', n: 80 }] }]).map(r => kortRegel(r)) });
export const KORT = { verden: JSON.parse(ui('assets/verden.json')), colo: JSON.parse(ui('assets/colo.json')), tidszoner: JSON.parse(ui('assets/tidszoner.json')) };
/* Angreb-visningen bor i rammen: rammen spørger selv om token og overblik. */
export const MED_TOKEN = { token_status: { sat: true }, zones_cache: { zoner: [], hentet: null }, zones: [], overblik: OVERBLIK };
export const SIDEN_RK = [
  raekke('z2', 'xyz.wi24rd.com', 'roed', { angreb_i_gaar: 28, angreb_snit: 23.1, sidst_hentet: NU_ISO, angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 25 }, { navn: 'Browser Integrity Check', n: 3 }], angreb_fordeling: { blokeret: 20, udfordret: 8, labyrint: 0, andet: 0 } }),
  raekke('z1', 'wi24rd.com', 'gul', { angreb_i_gaar: 30, angreb_snit: 12.4, sidst_hentet: NU_ISO, angreb_i_gaar_top: [{ navn: 'React RCE (CVE-2025-55182)', n: 20 }], angreb_fordeling: { blokeret: 30, udfordret: 0, labyrint: 0, andet: 0 } }),
  raekke('g1', 'gammel.wi24rd.com', 'gul', { gammel: true, angreb_i_gaar: 99, sidst_hentet: NU_ISO }),
  raekke('n1', 'a.wi24rd.com', 'groen', { angreb_i_gaar: 3, sidst_hentet: NU_ISO, angreb_fordeling: { blokeret: 3, udfordret: 0, labyrint: 0, andet: 0 } }),
  raekke('n2', 'b.wi24rd.com', 'groen', { sidst_hentet: NU_ISO }),
  raekke('x1', 'abe.wi24rd.com', 'graa'),
];
export const B2 = x => Object.assign({
  token_status: { sat: true }, zones_cache: { zoner: OVERBLIK.map(r => ({ id: r.id, navn: r.navn })), hentet: NU_ISO }, zones: OVERBLIK.map(r => ({ id: r.id, navn: r.navn })), overblik: OVERBLIK,
  valg_hent: ['z1'], koe_status: null, koe_prioritet: null, autostart_hent: true, autostart_saet: true, notifikation_hent: { niveau: 'standard', roed: true, zoner: {} }, notifikation_saet: null,
  siden_sidst: { vis: false, nye: [], raekker: [] }, siden_sidst_set: null, data_status: { bytes: 2412000, foerste: '2026-10-04', sidste: '2026-11-02', domaener: 2, nulstil: { sikkerhed: null, trafik: null } }, forbindelse_glem: null, forbindelse_tilfoej: 1, forbindelse_skift: null, forbindelse_omdoeb: null, hent_nu: null,
  forbindelser_hent: [{ id: 1, cf_navn: 'Eksempel ApS', eget_navn: null, konti: 1, domaener: 3, afvist: false }],
  kortdata: KORT, vaern_cache: vaernUd(), vaern_oversigt: vaernUd(), vaern_samlet_cache: { vaern: vaernUd(), valgte: [], domaener: 1, med_tal: 1 },
}, x);
export const REACT = 'React - RCE - CVE:CVE-2025-55182';
export const dykDage = Array.from({ length: 30 }, (_, i) => ({ dag: new Date(Date.UTC(2026, 9, 5 + i)).toISOString().slice(0, 10), n: i % 4 && i !== 2 ? i : 0, hentet: i !== 2, alle: 100 * i }));
export const dykSvar = x => Object.assign({ rettet_mod: [], trafik_serie: [], forsoeg: { n: 120, kategorier: [{ id: 'hemmeligheder', n: 80, stier: [{ sti: '/.env', n: 80 }] }, { id: 'wordpress', n: 40, stier: [{ sti: '/wp-login.php', n: 40 }] }] },
  trafik: { i_alt: 0, dage: [] }, kun_i_dag: null, periode: { fra: '2026-10-05', til: '2026-11-03', dage: 30 }, i_gaar: '2026-11-03', i_dag: '2026-11-04', sum: 198, noegletal: { i_gaar: 6, i_dag_hidtil: 2, hele7: 57, hele30: 198, alt: 240 },
  foerst: '2026-10-05', sidst: '2026-11-03', dage: dykDage, fordeling: { blokeret: 190, udfordret: 5, labyrint: 3, andet: 0, faelder: 21 },
  top: { robot: [], status: [], regel: [{ v: REACT, n: 150 }, { v: 'botFight', n: 48 }], land: ['PL', 'US', 'DE', 'NL', 'SG', 'FR', 'T1', 'XX'].map((v, i) => ({ v, n: 80 - i * 9 })), netvaerk: [{ v: 'AT&T Services, Inc.', n: 60 }, { v: '<b>Fed</b> net', n: 2 }], sti: [{ v: '/wp-config%2ephp', n: 40 }, { v: '/', n: 20 }], colo: [{ v: 'WAW', n: 67 }, { v: 'EWR', n: 40 }], metode: [{ v: 'POST', n: 190 }, { v: 'GET', n: 8 }], handling: [{ v: 'blokeret', n: 190 }, { v: 'udfordret', n: 5 }, { v: 'labyrint', n: 3 }] },
  antal: { regel: 2, land: 8, netvaerk: 2, sti: 2, colo: 2, metode: 2, handling: 3 }, buer: [{ land: 'PL', colo: 'EWR', n: 40 }, { land: 'US', colo: 'WAW', n: 8 }],
  regel: null, domaener: [{ zone: 'z2', navn: 'xyz.wi24rd.com', n: 92 }, { zone: 'z3', navn: 'abe.wi24rd.com', n: 8 }],
  enkelte: [{ t: '2026-11-03T21:00:00Z', regel: REACT, cve: 'CVE-2025-55182', land: 'PL', sti: '[hemmeligheder]', colo: 'WAW', metode: 'POST' }], slags: { angreb: 150, rutine: 48 }, loft: false,
  regler: [{ navn: REACT, produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', art: 'angreb', h30: 150, h7: 0, idag: 0, faelder30: 0, foerst: '2026-11-01', sidst: '2026-11-03', stier: [{ sti: '/api', n: 100 }], lande: [{ kode: 'PL', n: 120 }] }, { navn: 'botFight', produkt: '', type: '', cve: null, art: 'rutine', h30: 48, h7: 0, idag: 0, faelder30: 0, foerst: '2026-11-02', sidst: '2026-11-02', stier: [], lande: [] }],
}, x);
/* Svaret fra dyk med de samme tal som et vaernUd: siden tegner fra dyk, og tests af Angreb skriver deres tal i vaern_cache. */
export const dykFraVaern = v => {
  const hul = new Set(v.uhentet || []), dg = v.h_dage.map(d => ({ dag: d.dag, n: d.n, hentet: !hul.has(d.dag), alle: d.alle || 0 })), sidst = dg.at(-1), med = n => dg.slice(-n).filter(d => d.hentet).length;
  const f = v.h_forsoeg || {}, vn = (l, k) => l.map(x => ({ v: x[k], n: x.n }));
  return dykSvar({
    forsoeg: { n: f.i7 || 0, kategorier: f.kategorier || [] }, periode: { fra: dg[0]?.dag || '2026-10-05', til: sidst?.dag || '2026-11-03', dage: dg.length, med: dg.filter(d => d.hentet).length },
    i_gaar: sidst?.dag || '2026-11-03', i_dag: v.h_idag, sum: v.hele30, noegletal: { i_gaar: v.i_gaar, i_dag_hidtil: v.i_dag_hidtil, hele7: v.hele7, hele30: v.hele30, alt: v.hele30, med7: Math.min(7, med(7)), med30: Math.min(30, med(30)) },
    foerst: null, sidst: null, dage: dg, fordeling: (v.fordeling && v.fordeling.d30) || {}, buer: v.h_buer, buer_i_alt: v.h_buer_i_alt,
    top: { robot: [], status: [], regel: v.h_regler.map(r => ({ v: r.navn, n: r.h30, art: r.art })), land: vn(v.h_lande, 'kode'), netvaerk: vn(v.h_netvaerk, 'navn'), sti: [], colo: vn(v.h_colos, 'kode'), metode: [], handling: [] },
    antal: { regel: v.h_regler.length, land: v.h_lande.length, netvaerk: v.h_netvaerk.length, sti: 0, colo: v.h_colos.length, metode: 0, handling: 0 },
    domaener: [], enkelte: v.seneste || [], slags: { angreb: v.hele30, rutine: 0 }, regler: v.h_regler.filter(r => r.h30 || !r.idag), trafik: { i_alt: 0, dage: v.trafik || [] },
  });
};
export const REGEL_HOVED = { navn: REACT, produkt: 'React', type: 'RCE', cve: 'CVE-2025-55182', art: 'angreb' };
export const landDage = Array.from({ length: 30 }, (_, i) => ({ dag: new Date(Date.UTC(2026, 9, 4 + i)).toISOString().slice(0, 10), mit: i === 3 ? null : i % 5 ? 4 : 0, mit_indeks: i === 3 ? null : i % 5 ? 125 : 0, verden_indeks: 90 + i }));
export const LAND_UD = {
  kode: 'DK', i_gaar: '2026-11-02', dage: landDage, andel: [{ dag: '2026-11-02', vaerdi: 0.4 }, { dag: '2026-11-01', vaerdi: null }], ord: null,
  sammen7: { land: [{ navn: 'US', vaerdi: 41 }, { navn: 'XX', vaerdi: 3 }], rest: 56, spurgt: true, fra: '2026-10-27', til: '2026-11-02', mine: [{ navn: 'PL', vaerdi: 60, n: 60, saerligt: true }, { navn: 'US', vaerdi: 40, n: 40, saerligt: false }], mine_i_alt: 100 },
  sammen28: { land: [{ navn: 'DE', vaerdi: 30 }, { navn: 'US', vaerdi: 20 }], rest: 50, spurgt: true, fra: '2026-10-06', til: '2026-11-02', mine: [{ navn: 'US', vaerdi: 75, n: 300, saerligt: true }, { navn: 'XX', vaerdi: 25, n: 100, saerligt: false }], mine_i_alt: 400 },
  serie_fejl: [], udfald_verden: [], udfald_verden_hentet: false,
  udfald: [{ start: '2026-10-30T04:00:00Z', slut: '2026-10-30T06:30:00Z', minutter: 150, aarsag: 'CABLE_CUT', slags: 'NETWORK', netvaerk: ['Fjord Net', 'Bakke Net'] }, { start: '2026-10-20T01:00:00Z', slut: null, minutter: null, aarsag: 'NOGET_NYT', slags: '', netvaerk: [] }], udfald_hentet: true,
  maal: [{ navn: 'US', vaerdi: 31 }, { navn: 'DK', vaerdi: 0.4 }], kilde: [{ navn: 'CN', vaerdi: 13 }, { navn: 'T1', vaerdi: 2 }], produkt: [{ navn: 'DDOS', vaerdi: 54 }, { navn: 'WAF', vaerdi: 33 }],
  ai: [{ navn: 'Training', vaerdi: 40 }, { navn: 'Noget nyt', vaerdi: 10 }], ai_navne: [{ navn: 'GPTBot', vaerdi: 30 }], bot: 38.5, kvante: 42.5, http_indeks: 104, verden_dag: '2026-11-02', hentet: NU_ISO, fejl: null,
};
export const B2C_TRAFIK = { dag: '2026-11-02', til: '2026-11-02', i_alt: 410000, median: 1500, stoppet: 67, lande: [{ v: 'US', n: 378000, angreb: 60 }, { v: 'DE', n: 32886, angreb: 0 }], lande_i_alt: 410000, status: [{ v: '522', n: 345000 }, { v: '200', n: 65886 }], status_i_alt: 410000, sikkerhed: true, trusler: 0, cachet: 0, server_fejl: 345000, robotter: [{ v: 'Googlebot', n: 300000 }], ekstra: true };
export const B2C_TEK = { linjer: [{ tek: 'wordpress', angreb30: 40, forsoeg30: 312, foerst: '2026-10-04', sidst: '2026-11-02', valg: 'ved_ikke', besked: false, forslag: true }, { tek: 'react', angreb30: 9, forsoeg30: 0, foerst: '2026-10-05', sidst: '2026-11-01', valg: 'bruger', besked: true, forslag: false }], generiske: [{ id: 'hemmeligheder', forsoeg30: 4 }] };
export const B2C_VERDEN = [{ start: '2026-10-30T04:00:00Z', slut: '2026-10-30T06:00:00Z', minutter: 120, aarsag: 'CABLE_CUT', slags: 'NATIONWIDE', netvaerk: ['Fjord Net'], lande: ['SE'] }];
export const B2C_HULLER = { linjer: [{ noegle: '2026-09-02_2026-10-04', fra: '2026-09-02', til: '2026-10-04', tabt: 9 }], dage: ['2026-09-03'], aeldste: '2026-08-01' };
/* Svarene fra robotter, login og login_stier. */
export const ROBOT_UD = {
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
export const loginDage = Array.from({ length: 30 }, (_, i) => ({ dag: new Date(Date.UTC(2026, 9, 5 + i)).toISOString().slice(0, 10), edge: i % 3 ? 4 : 0, ikke_fundet: i % 5 ? 0 : 2, server: i === 29 ? 3 : 0, hentet: true }));
export const LOGIN_UD = { laek: [], laek_andet: [], laek_dage: 30,
  periode: { fra: '2026-10-05', til: '2026-11-03', dage: 30 }, stier: ['/login', '/wp-login.php'],
  tal: { edge: 161, ikke_fundet: 18, server: 51, afvist: 42, svar: 9, andet: 0 }, dage: loginDage,
  raekker: [{ sti: '/wp-login.php', metoder: ['POST'], tal: { edge: 118, ikke_fundet: 18, server: 30, afvist: 23, svar: 7, andet: 0 } }, { sti: '/login', metoder: ['POST', 'GET'], tal: { edge: 0, ikke_fundet: 0, server: 21, afvist: 19, svar: 2, andet: 0 } }],
  metoder_edge: [{ v: 'POST', n: 161 }], metoder_server: [{ v: 'POST', n: 51 }], lande_edge: [{ v: 'PL', n: 71 }], lande_server: [{ v: 'US', n: 49 }], net_edge: [{ v: 'Eksempel Hosting', n: 64 }], net_server: [{ v: 'AS64500', n: 41 }],
  mangler_dage: 0, ikke_tilgaengelig: [],
};
export const LOGIN_STIER = { stier: [{ sti: '/login', til: true, start: true }, { sti: '/admin/', til: false, start: true }, { sti: '/min-side', til: true, start: false }], forslag: ['/api/auth/signin'] };

/* Svaret fra besoegende: 7 dage med et hul (2. november) og en dag med 0, og 14 lande, så "Vis alle" står. Med et land som filter: besøg fra landet pr. dag i 30 dage. */
const besDag = (til, i) => new Date(Date.parse(`${til}T00:00:00Z`) - i * 864e5).toISOString().slice(0, 10);
export const BES_LANDE = [['DK', 2200], ['US', 900], ['DE', 300], ['SE', 210], ['NO', 150], ['GB', 120], ['NL', 90], ['FR', 70], ['PL', 40], ['T1', 12], ['XX', 8], ['ES', 5], ['IT', 3], ['CN', 2]].map(([kode, n]) => ({ kode, n }));
/* V-BESOEG.2: kendte robotter pr. gruppe, IP-klasserne for året og 14 sider (så "Vis alle" står) plus 404'erne. */
export const BES_SIDER = [['/', 1840, 1210], ['/produkter', 620, 410], ['/om', 300, 120], ['/kontakt', 210, 90], ['/blog', 160, 70], ['/blog/vinter', 90, 60], ['/priser', 80, 40], ['/sitemap', 40, 0], ['/job', 30, 12], ['/presse', 22, 10], ['/faq', 15, 9], ['/cookies', 9, 2], ['/en', 6, 4], ['/gammel', 2, 0]].map(([sti, n, besoeg]) => ({ sti, n, besoeg }));
export const BES_404 = [['/admin', 310], ['/wp-login.php', 120], ['/.env', 44]].map(([sti, n]) => ({ sti, n, besoeg: 0 }));
/* V-BESOEG.2 FIX 1: stier, der svarede med sidens almindelige side (catch-all): regnet i Rust (besoeg::forside_deling). */
export const BES_FORSIDE = [['//crm/.env', 6], ['/blog/wp-config.php/', 4], ['/.hg/store/requires', 2]].map(([sti, n]) => ({ sti, n, besoeg: 0 }));
const besEkstra = {
  robot30: { n: 980, i_alt: 4210, med: 30 }, uger: null, sti: null, robot: null,
  robotter: [{ id: 'soegemaskiner', n: 610 }, { id: 'ai-traening', n: 240 }, { id: 'overvaagning', n: 90 }, { id: 'sociale', n: 40 }], robot_med: 7, robot_fra: '2026-10-07', robot_ikke: [],
  ipklasse: { n: 41200, i_alt: 158000, med: 340 },
  sider: BES_SIDER, sider_i_alt: BES_SIDER.reduce((a, x) => a + x.n, 0), sider_404: BES_404, sider_forside: BES_FORSIDE, sider_uden_forside: false, side_med: 7, side_fra: '2026-10-07', side_ikke: [],
};
export const BESOEG_UD = {
  periode: { fra: '2026-10-30', til: '2026-11-05', dage: 7, med: 6, data_fra: '2025-11-06', data_til: '2026-11-05', i_dag: '2026-11-06', sikkerhed_fra: null, trafik_fra: null },
  i_gaar_dag: '2026-11-05', i_gaar: 412, d7: { n: 2380, med: 6, dage: 7 }, d30: { n: 11240, med: 30, dage: 30 }, aar: { n: 131900, med: 340, dage: 365 }, besoeg30: { n: 4210, med: 30, dage: 30 },
  dage: [388, 402, 0, null, 455, 723, 412].map((n, i) => ({ dag: besDag('2026-11-05', 6 - i), n })), land: null,
  lande: BES_LANDE, lande_i_alt: BES_LANDE.reduce((a, x) => a + x.n, 0), lande_med: 7, besoeg_fra: '2026-10-07', ikke_tilgaengelig: [], ...besEkstra,
};
export const BESOEG_LAND = { ...BESOEG_UD, land: 'DK', robot30: null, besoeg30: { n: 2810, med: 30, dage: 30 }, periode: { ...BESOEG_UD.periode, fra: '2026-10-07', dage: 30, med: 30 }, dage: Array.from({ length: 30 }, (_, i) => ({ dag: besDag('2026-11-05', 29 - i), n: 40 + ((i * 7) % 23) })), lande_med: 30 };
/* En sti som filter: visninger af /produkter pr. dag, og flisen Besøg følger stien. */
export const BESOEG_SIDE = { ...BESOEG_LAND, land: null, sti: '/produkter', besoeg30: { n: 410, med: 30, dage: 30 }, dage: Array.from({ length: 30 }, (_, i) => ({ dag: besDag('2026-11-05', 29 - i), n: 12 + ((i * 5) % 17) })) };
/* Et år pr. uge: 365 dage, de første 25 uden tal, en uge med 3 af 7 dage; ugerne er mandag–søndag som i Rust (besoeg::uger). */
const aarDage = Array.from({ length: 365 }, (_, i) => { const dag = besDag('2026-11-05', 364 - i); return { dag, n: i < 25 || (dag >= '2026-06-11' && dag <= '2026-06-14') ? null : 300 + ((i * 37) % 260) }; });
const ugerAf = dage => dage.reduce((u, d) => { const ms = Date.parse(d.dag + 'T00:00:00Z'), man = new Date(ms - ((new Date(ms).getUTCDay() + 6) % 7) * 864e5).toISOString().slice(0, 10); const sidst = u[u.length - 1]; const x = sidst && sidst.dag === man ? sidst : (u.push({ dag: man, fra: d.dag, til: d.dag, n: null, med: 0, dage: 0 }), u[u.length - 1]); x.til = d.dag; x.dage++; if (d.n !== null) { x.n = (x.n || 0) + d.n; x.med++; } return u; }, []);
export const BESOEG_AAR = { ...BESOEG_UD, periode: { ...BESOEG_UD.periode, fra: '2025-11-06', dage: 365, med: 336 }, dage: aarDage, uger: ugerAf(aarDage), lande_med: 30 };
/* Stubbene vælger svaret efter kaldet (som dyk_regel): land, sti, et år, ellers 7 dage. */
export const BES_NOEGLE = "c==='subdomaener'&&a.zone!=='z1'?'subdomaener_en':c==='besoegende'&&a.vaert==='api'?'besoegende_vaert_tom':c==='besoegende'&&a.vaert!=null&&!a.sti?'besoegende_vaert':c==='besoegende'&&a.land?'besoegende_land':c==='besoegende'&&a.sti?'besoegende_side':c==='besoegende'&&a.periode==='alt'?'besoegende_aar':c";

/* V-SUBDOM.1 B2: svaret fra subdomaener. Værterne er labels under domænet ("" = domænet selv) med tal pr. dag: [vært, forespørgsler, besøg, angreb, afviste]
   pr. dag, så summerne passer til dagene. Kun opdigtede navne under wi24rd.com. huller: dage uden tal (null, ikke 0). trafikHuller (V-SUBDOM.1 FIX 2): dage
   uden trafik pr. vært (besøg og forespørgsler null), men med angreb. trafik_dage og angreb_dage tælles som i Rust; trafik_mangler/angreb_mangler er tomme,
   medmindre posten sætter dem. */
export const dageFraTil = (fra, til) => { const ud = []; for (let t = Date.parse(`${fra}T00:00:00Z`); t <= Date.parse(`${til}T00:00:00Z`); t += 864e5) ud.push(new Date(t).toISOString().slice(0, 10)); return ud; };
export const subSvar = (fra, til, spec, x = {}) => {
  const dage = dageFraTil(fra, til), hul = new Set(x.huller || []), tHul = new Set(x.trafikHuller || []);
  const vaerter = spec.map(([vaert, f, b, a, af]) => {
    const dg = dage.map(dag => (hul.has(dag) ? { dag, foresp: null, besoeg: null, angreb: null, afvist: null } : tHul.has(dag) ? { dag, foresp: null, besoeg: null, angreb: a, afvist: af } : { dag, foresp: f, besoeg: b, angreb: a, afvist: af }));
    const sum = k => dg.reduce((n, d) => n + (d[k] || 0), 0);
    return { vaert, foresp: sum('foresp'), besoeg: sum('besoeg'), angreb: sum('angreb'), afvist: sum('afvist'), dage: dg };
  });
  const { huller, trafikHuller, ...resten } = x;
  const tDage = dage.filter(d => !hul.has(d) && !tHul.has(d)).length, aDage = dage.filter(d => !hul.has(d)).length;
  return { fra, til, vaerter, trafik_fra: '2026-10-07', angreb_fra: '2026-10-07', trafik_afskaaret: [], angreb_afskaaret: [], sider_afskaaret: false, trafik_afvist: false, angreb_afvist: false, trafik_mangler: [], angreb_mangler: [], trafik_dage: tDage, angreb_dage: aDage, ...resten };
};
/* wi24rd.com med fire subdomæner (www, shop, api, blog) og et opslag med IP-adressen; sorteret som i Rust (navne efter forespørgsler, kategorierne sidst). */
export const SUB_FIRE = [['', 5559, 492, 539, 600], ['api', 900, 0, 25, 30], ['www', 441, 0, 0, 1], ['shop', 143, 3, 77, 80], ['blog', 60, 12, 2, 2], ['[ip]', 30, 0, 12, 14]];
/* Et domæne uden subdomæner: kun domænet selv. */
export const SUB_EN = [['', 2100, 300, 40, 41]];
/* Mest besøgte sider for wi24rd.com med subdomæner: samme sti (/) på domænet selv, shop og blog er tre rækker. */
export const BES_SIDER_VAERT = [['/', 1840, 1210, ''], ['/', 320, 40, 'shop'], ['/produkter', 620, 410, ''], ['/kurv', 210, 30, 'shop'], ['/', 90, 12, 'blog'], ['/v1/status', 60, 0, 'api'], ['/om', 300, 120, '']].map(([sti, n, besoeg, vaert]) => ({ sti, n, besoeg, vaert })).sort((a, b) => b.n - a.n);

// ── Stub-svaret, alle visninger bruger ──
const zoner = OVERBLIK.map(r => ({ id: r.id, navn: r.navn })), vaern = vaernUd();
export const svar = {
  ...B2(), valg_hent: zoner.map(z => z.id), domaener_hent: zoner.map((z, i) => ({ ...z, til: true, ny: i === 2 })), vaern_samlet_cache: { vaern, valgte: zoner.map(z => ({ ...z, med_tal: true })), domaener: zoner.length, med_tal: zoner.length },
  siden_sidst: { vis: true, nye: [], raekker: SIDEN_RK }, dyk: dykSvar(), dyk_regel: dykSvar({ regel: REGEL_HOVED, regler: [] }), robotter: ROBOT_UD, besoegende: BESOEG_UD, besoegende_land: BESOEG_LAND, besoegende_side: BESOEG_SIDE, besoegende_aar: BESOEG_AAR, login: LOGIN_UD, login_stier: LOGIN_STIER, land_hent: { land: 'DK', forslag: 'US', med_tal: ['DE', 'DK', 'FR', 'GB', 'NL', 'PL', 'SE', 'US'] }, land_visning: { ...LAND_UD, udfald_verden: B2C_VERDEN, udfald_verden_hentet: true }, trafik_dag: B2C_TRAFIK, teknologi_hent: B2C_TEK, huller: B2C_HULLER,
};
Object.assign(svar, { teknologi_oversigt: { zoner: { z1: 2, z2: 1 }, valg: true }, teknologi_side: { domaener: [{ zone: 'z1', navn: 'wi24rd.com', valg: { php: ['bruger', true], wordpress: ['bruger_ikke', false] }, antal: { php: 204, wordpress: 12 } }, { zone: 'z2', navn: 'xyz.wi24rd.com', valg: {}, antal: { php: 3 } }], katalog: [{ tek: 'php', navn: 'PHP', gruppe: 'server', angreb: 207, set: true, vis: true, ny: false }, { tek: 'wordpress', navn: 'WordPress', gruppe: 'cms', angreb: 12, set: true, vis: true, ny: true }, { tek: 'magento', navn: 'Magento', gruppe: 'cms', angreb: 0, set: false, vis: false, ny: false }] }, teknologi_hent: { linjer: [{ tek: 'php', angreb30: 3, forsoeg30: 40, foerst: '2026-10-05', sidst: '2026-11-02', valg: 'bruger', besked: true, forslag: false }, { tek: 'wordpress', angreb30: 12, forsoeg30: 0, foerst: '2026-10-05', sidst: '2026-11-02', valg: 'ved_ikke', besked: false, forslag: true }], generiske: [] } });
export const VURD7 = { dag: '2026-11-02', maal_ja: false, foreloebig: false, rekog_ja: false, maal: [{ id: 'stigning', ok: true, n: 40, m: 2, navn: null, domaener: [] }, { id: 'net_andre', ok: false, n: 9, m: 30, navn: 'Mal AS', domaener: [{ zone: 'z2', navn: 'xyz.wi24rd.com', n: 9 }] }, { id: 'radar', ok: null, n: 0, m: 1.3, navn: null, domaener: [] }], rekog: [{ id: 'stier', ok: false, n: 12, m: 100, navn: null, domaener: [] }] };
Object.assign(svar, { vurdering: VURD7 });
/* V-TOKEN.1: følsomme filer med svar 200 og Cloudflares egne fund på Angreb for ét domæne. */
export const STI200_UD = { stier: [{ sti: '/.env', n: 2, snit: 900, troevaerdig: true }, { sti: '/.git/config', n: 14, snit: 38396, troevaerdig: false }, { sti: '/.aws/credentials', n: 11, snit: 42672, troevaerdig: false }], forside: 28949, hentede_dage: 7, liste: 39 };
export const FUND_UD = { tilstand: 'ok', fejl: null, hentet: NU_ISO, fund: [{ zone: 'z1', alvor: 'Critical', klasse: 'always_use_https_not_enabled', type: 'insecure_configuration', siden: '2026-04-24' }, { zone: 'z1', alvor: 'Low', klasse: 'security_txt_not_enabled', type: 'configuration_suggestion', siden: '2026-04-24' }] };
Object.assign(svar, { sti200_oversigt: STI200_UD, sikkerhedsfund_oversigt: FUND_UD, aabn_sikkerhedsfund: null });
export const dom = 'zone=z1&navn=wi24rd.com';
/* Hver post er én visning. id giver filnavnet i fotobogen og må ikke skifte. hash står uden #. rul ruller til et afsnit før billedet. aabn klikkes, rod er det, der skal være synligt bagefter, og luk lukker igen. visning står kun, når en anden visning end den i hash skal vises.
   ekstra lægges oven på svar: de poster kræver deres egen indlæsning, for rammen spørger kun om token og domæner, når den starter. */
export const SIDER = [
  { id: 'overblik', navn: 'Overblik', hash: 'overblik' }, { id: 'siden-sidst', navn: 'Siden sidst', hash: 'siden-sidst' }, { id: 'angreb-samlet', navn: 'Angreb samlet', hash: 'vaernet?samlet=1' }, { id: 'angreb', navn: 'Angreb', hash: 'vaernet?' + dom }, { id: 'dyk', navn: 'dyk', hash: 'vaernet?' + dom + '&periode=30' }, { id: 'angreb-datoer', navn: 'Angreb med egne datoer', hash: 'vaernet?' + dom + '&fra=2026-10-20&til=2026-11-02' }, { id: 'dyk-regel', navn: 'dyk i en regel', hash: `vaernet?${dom}&periode=30&regel=${encodeURIComponent(REACT)}` }, { id: 'dyk-to-filtre', navn: 'dyk med to filtre', hash: `vaernet?${dom}&land=FR&dag=2026-11-03` },
  { id: 'robotter', navn: 'Robotter', hash: 'robotter?' + dom }, { id: 'besoegende', navn: 'Besøgende, et år pr. uge', hash: 'besoegende?' + dom + '&periode=aar' }, { id: 'besoegende-land', navn: 'Besøgende med et land som filter', hash: 'besoegende?' + dom + '&periode=30&land=DK' }, { id: 'besoegende-side', navn: 'Besøgende med en sti som filter', hash: 'besoegende?' + dom + '&periode=30&sti=%2Fprodukter', rul: '#besoeg-sider' }, { id: 'login', navn: 'Login', hash: 'login?' + dom, aabn: ['details.login-ret summary'] }, { id: 'land-dk', navn: 'Danmark lige nu', hash: 'land' }, { id: 'land-verden', navn: 'Verden', hash: 'land?fane=verden' }, { id: 'dyk-land', navn: 'dyk i et land', hash: 'vaernet?' + dom + '&periode=30&land=PL' }, { id: 'indstillinger', navn: 'Indstillinger (popuppen) fra toppen', hash: 'overblik', aabn: ['#indst-knap'], rod: '#indst-dialog', luk: '#indst-luk' }, { id: 'indstillinger-forbindelse', navn: 'Indstillinger: Forbindelse og Data', hash: 'overblik', aabn: ['#indst-knap', '#skift-noegle'], rul: 'section[aria-labelledby="indst-forbindelse"]', rod: '#indst-dialog', luk: '#indst-luk' },
  { id: 'domaener-dialog', navn: 'dialogen Domæner', hash: 'overblik', aabn: ['#vaelg-dom'], rod: '#domaener-dialog', luk: '#domaener-luk' }, { id: 'besked-dialog', navn: 'dialogen Notifikationer pr. domæne', hash: 'overblik', aabn: ['#giv-besked'], rod: '#besked-dialog', luk: '#besked-luk' }, { id: 'om-dialog', navn: 'dialogen Om', hash: 'overblik', aabn: ['#om-knap'], rod: '#om-dialog', luk: '#om-luk' },
  { id: 'guide-3', navn: 'guiden, trin 3: Dine domæner', hash: 'overblik', aabn: ['#indst-knap', '#guide-igen', '#guide-naeste', '#guide-naeste'], rod: '#guide-dialog', luk: '#guide-spring' }, { id: 'guide-4', navn: 'guiden, trin 4: Besked', hash: 'overblik', aabn: ['#indst-knap', '#guide-igen', '#guide-naeste', '#guide-naeste', '#guide-naeste'], rod: '#guide-dialog', luk: '#guide-spring' },
  { id: 'trafik-dyk', navn: 'trafik-dyk', hash: `vaernet?${dom}&dag=2026-11-02&trafik=1` }, { id: 'hjaelp', navn: 'hjælpen', hash: 'overblik', aabn: ['#hjaelp-knap'], rod: '#hjaelp', luk: '#hjaelp-luk' }, { id: 'glem-dialog', navn: 'bekræftelsen', hash: 'overblik', aabn: ['#indst-knap', '#glem'], rod: '#glem-dialog', luk: '#glem-nej' },
  { id: 'teknologi', navn: 'Teknologi', hash: 'teknologi' },
  { id: 'angreb-fund', navn: 'Angreb: følsomme filer og Cloudflares fund', hash: 'vaernet?' + dom, rul: '[data-plads="sti200"]' },
  { id: 'angreb-estimat', navn: 'Angreb: loftet og grænsen mellem talte rækker og estimater', hash: 'vaernet?' + dom + '&periode=30', rul: '.vg-loft', rulTekst: true, ekstra: { dyk: dykSvar({ vaegt_fra: '2026-10-10', loft: true }) } },
  /* Uden API Token åbner guiden af sig selv: den tomme tilstand bag den ses, når guiden er sprunget over. */
  { id: 'opsaetning', navn: 'opsætningen (den tomme tilstand)', hash: 'overblik', visning: 'opsaetning', aabn: ['#guide-spring'], ekstra: { token_status: { sat: false } } },
  { id: 'guide-1', navn: 'guiden, trin 1: Velkommen', hash: 'overblik', visning: 'opsaetning', rod: '#guide-dialog', ekstra: { token_status: { sat: false } } },
  { id: 'guide-2', navn: 'guiden, trin 2: Dit API Token', hash: 'overblik', visning: 'opsaetning', aabn: ['#guide-naeste', '#guide-trin-token .hm-hjaelp summary'], rod: '#guide-dialog', ekstra: { token_status: { sat: false } } },
  { id: 'alle-fra', navn: 'velkomsten', hash: 'overblik', ekstra: { domaener_hent: zoner.map(z => ({ ...z, til: false, ny: false })), overblik: [], valg_hent: [] } },
];
/* V-KONTI.1 B2: to forbindelser (opdigtede navne; den ene er Cloudflares standardnavn med e-mailadresse) og en tilstand, hvor den ene er afvist. Siderne
   hedder konti-…; HM_FOTO_SIDER='konti-*' tager kun dem (fotobogen til cyklussen). */
export const KONTI_TO = [{ id: 1, cf_navn: 'Eksempel ApS', eget_navn: null, konti: 1, domaener: 2, afvist: false }, { id: 2, cf_navn: "test@wi24rd.com's Account", eget_navn: null, konti: 1, domaener: 1, afvist: false }];
const kontiDomaener = zoner.map((z, i) => ({ ...z, til: i !== 2, ny: false, forbindelse: i < 2 ? 1 : 2 }));
const KONTI = { forbindelser_hent: KONTI_TO, domaener_hent: kontiDomaener };
const KONTI_AFVIST = { ...KONTI, forbindelser_hent: KONTI_TO.map(f => ({ ...f, afvist: f.id === 2 })), koe_status: { zone: null, navn: null, zoner_i_alt: 3, zoner_faerdige: 3, doegn_i_alt: 93, doegn_faerdige: 93, zoner_fejl: 0, tilstand: 'faerdig', afviste: [2] } };
const indstForb = 'section[aria-labelledby="indst-forbindelse"]';
SIDER.push(
  { id: 'konti-forbindelser', navn: 'Indstillinger: to forbindelser', hash: 'overblik', aabn: ['#indst-knap'], rul: indstForb, rod: '#forb-liste', luk: '#indst-luk', ekstra: KONTI },
  { id: 'konti-omdoeb', navn: 'Omdøb en forbindelse', hash: 'overblik', aabn: ['#indst-knap', '#forb-2-omdoeb'], rod: '#omdoeb-dialog', luk: '#omdoeb-nej', ekstra: KONTI },
  { id: 'konti-glem', navn: 'Glem en forbindelse (navnet i overskriften)', hash: 'overblik', aabn: ['#indst-knap', '#forb-2-glem'], rod: '#glem-dialog', luk: '#glem-nej', ekstra: KONTI },
  { id: 'konti-domaener', navn: 'Domæner i to grupper', hash: 'overblik', aabn: ['#vaelg-dom'], rod: '#domaener-grupper', luk: '#domaener-luk', ekstra: KONTI },
  { id: 'konti-afvist', navn: 'Overblik med banneret om et afvist API Token', hash: 'overblik', rod: '#afvist-linjer .ban', ekstra: KONTI_AFVIST },
  /* FIX 1 (V3): ingen rul her; billedet viser, hvor programmet selv lægger formularen, og feltet skal have fokus og stå inden for kanten. */
  { id: 'konti-afvist-skift', navn: 'Skift API Token… fra banneret (rækken med den røde chip)', hash: 'overblik', aabn: ['#afvist-linjer .ban button'], rod: '#skift-form', fokus: '#skift-felt', luk: '#indst-luk', ekstra: KONTI_AFVIST },
);

/* V-SUBDOM.1 B2: Besøgende og Angreb for wi24rd.com med fire subdomæner, ufiltreret og filtreret pr. subdomæne, og xyz.wi24rd.com uden subdomæner (uændret).
   Siderne hedder subdom-…; HM_FOTO_SIDER='subdom-*' tager kun dem. Tallene gælder 5. oktober–5. november, så de dækker både Besøgendes og Angrebs perioder. */
const SUB_PERIODE = ['2026-10-05', '2026-11-05'];
export const BES_VAERT = { ...BESOEG_UD, sider: BES_SIDER_VAERT, sider_i_alt: BES_SIDER_VAERT.reduce((a, x) => a + x.n, 0), sider_404: [{ sti: '/admin', n: 310, besoeg: 0, vaert: '' }, { sti: '/.env', n: 44, besoeg: 0, vaert: 'api' }], sider_forside: [] };
/* FIX 1: med et filter pr. subdomæne er lande og robotter værtens (Rust, v_land og v_robot): shop har 3 besøg pr. dag i SUB_FIRE, 21 på 7 dage. IP-klasserne
   og domænets noter findes ikke pr. vært. api har ingen besøg udefra og derfor hverken lande eller robotter (afsnittene står ikke). */
export const BES_SHOP_LANDE = [['DK', 14], ['US', 5], ['DE', 2]].map(([kode, n]) => ({ kode, n }));
export const BES_SHOP = { ...BES_VAERT, sider: BES_SIDER_VAERT.filter(x => x.vaert === 'shop'), sider_i_alt: BES_SIDER_VAERT.filter(x => x.vaert === 'shop').reduce((a, x) => a + x.n, 0), sider_404: [], lande: BES_SHOP_LANDE, lande_i_alt: 21, lande_med: 7, besoeg_fra: '2026-10-07', ikke_tilgaengelig: [], robotter: [{ id: 'soegemaskiner', n: 6 }, { id: 'ai-traening', n: 2 }], robot_med: 7, robot_fra: '2026-10-07', robot_ikke: [], ipklasse: null };
export const BES_API = { ...BES_SHOP, sider: BES_SIDER_VAERT.filter(x => x.vaert === 'api'), sider_i_alt: 60, lande: [], lande_i_alt: 0, robotter: [] };
/* To dage i perioden stod på Cloudflares loft (V2): noten under tabellen. */
const SUB = { subdomaener: subSvar(...SUB_PERIODE, SUB_FIRE, { sider_afskaaret: true, trafik_afskaaret: ['2026-11-01'], angreb_afskaaret: ['2026-11-01', '2026-11-02'] }), subdomaener_en: subSvar(...SUB_PERIODE, SUB_EN), besoegende: BES_VAERT, besoegende_vaert: BES_SHOP, besoegende_vaert_tom: BES_API };
/* FIX 2: trafikken pr. vært mangler for hele perioden (opgraderingen eller genhentningen); køen har de seneste 30 dage til gode. */
const SUB_HENTER_DAGE = dageFraTil(...SUB_PERIODE);
export const SUB_HENTER = { ...SUB, subdomaener: subSvar(...SUB_PERIODE, SUB_FIRE, { trafikHuller: SUB_HENTER_DAGE, trafik_mangler: SUB_HENTER_DAGE.slice(-30) }) };
/* Uden subdomæner: kun domænet selv i subdomaener, og Besøgendes almindelige svar (siderne har ingen vært). */
const SUB_UDEN = { subdomaener_en: subSvar(...SUB_PERIODE, SUB_EN) }, domUden = 'zone=z2&navn=xyz.wi24rd.com';
SIDER.push(
  { id: 'subdom-besoegende', navn: 'Besøgende: afsnittet Subdomæner', hash: 'besoegende?' + dom, rul: '#besoeg-sub', ekstra: SUB },
  { id: 'subdom-besoegende-sider', navn: 'Besøgende: Mest besøgte sider med subdomænet ved stien', hash: 'besoegende?' + dom, rul: '#besoeg-sider', ekstra: SUB },
  { id: 'subdom-besoegende-filter', navn: 'Besøgende filtreret til shop.wi24rd.com', hash: 'besoegende?' + dom + '&vaert=shop.wi24rd.com', ekstra: SUB },
  { id: 'subdom-besoegende-filter-lande', navn: 'Besøgende filtreret til shop.wi24rd.com: lande og kendte robotter pr. subdomæne', hash: 'besoegende?' + dom + '&vaert=shop.wi24rd.com', rul: '#besoeg-lande', ekstra: SUB },
  { id: 'subdom-besoegende-filter-tom', navn: 'Besøgende filtreret til api.wi24rd.com (ingen lande eller robotter: afsnittene står ikke)', hash: 'besoegende?' + dom + '&vaert=api.wi24rd.com', ekstra: SUB },
  { id: 'subdom-angreb', navn: 'Angreb: afsnittet Subdomæner', hash: 'vaernet?' + dom, rul: '#vg-sub', ekstra: SUB },
  { id: 'subdom-angreb-filter', navn: 'Angreb filtreret til shop.wi24rd.com', hash: 'vaernet?' + dom + '&periode=30&vaert=shop.wi24rd.com', ekstra: SUB },
  { id: 'subdom-besoegende-uden', navn: 'Besøgende for et domæne uden subdomæner (uændret)', hash: 'besoegende?' + domUden, ekstra: SUB_UDEN },
  { id: 'subdom-angreb-uden', navn: 'Angreb for et domæne uden subdomæner (uændret)', hash: 'vaernet?' + domUden, ekstra: SUB_UDEN },
  /* FIX 2 (F3): lige efter en opdatering har angrebene pr. subdomæne tal, men trafikken er ikke hentet endnu: "–" i Besøg, Forespørgsler og Andel, og linjen
     "Henter besøg og forespørgsler pr. subdomæne …" under tabellen. */
  { id: 'subdom-besoegende-henter', navn: 'Besøgende: Subdomæner, mens trafikken pr. subdomæne hentes (angreb med tal, trafik "–")', hash: 'besoegende?' + dom, rul: '#besoeg-sub', ekstra: SUB_HENTER },
);

/* V-KONTI.1 FIX 1: Jans forsøg. Forbindelse 1 ser hele kontoen (tre domæner), forbindelse 2 er et API Token til ét af dem på samme konto. Begge hedder
   kontoens navn hos Cloudflare (D5). Det delte domæne står i begge grupper, med noten i forbindelse 2's (F2). Tilføj af en tredje med samme navn åbner
   Omdøb med noten (F1). Omdøb og Gem: beskeden står i rækken, og "Omdøb…" har fokus (F3). */
const ENS = "test@wi24rd.com's Account";
export const KONTI_DELT = [{ id: 1, cf_navn: ENS, eget_navn: null, konti: 1, domaener: 3, afvist: false, hentet: true }, { id: 2, cf_navn: ENS, eget_navn: null, konti: 1, domaener: 1, afvist: false, hentet: true }];
const deltDomaener = zoner.map((z, i) => ({ ...z, til: i !== 2, ny: false, forbindelse: 1, forbindelser: i === 1 ? [1, 2] : [1] }));
/* siden_sidst.vis er falsk som i fotobogens stub: kontrasttesten (K7) bruger svar uden det, og så springer rammen til Siden sidst efter Tilføj. */
const DELT = { forbindelser_hent: KONTI_DELT, domaener_hent: deltDomaener, siden_sidst: { vis: false, nye: [], raekker: [] } };
const DELT_TILFOEJ = { ...DELT, forbindelser_hent: [...KONTI_DELT, { id: 3, cf_navn: ENS, eget_navn: null, konti: 1, domaener: 1, afvist: false, hentet: true }], forbindelse_tilfoej: 3 };
SIDER.push(
  { id: 'konti-delt-forbindelser', navn: 'Indstillinger: to forbindelser med samme navn (samme konto)', hash: 'overblik', aabn: ['#indst-knap'], rul: indstForb, rod: '#forb-liste', luk: '#indst-luk', ekstra: DELT },
  { id: 'konti-delt-domaener', navn: 'Domæner: et delt domæne i begge grupper med noten', hash: 'overblik', aabn: ['#vaelg-dom'], rod: '#domaener-grupper [data-forbindelse="2"]', luk: '#domaener-luk', ekstra: DELT },
  { id: 'konti-ens-omdoeb', navn: 'Tilføj API Token… med samme navn: Omdøb med noten', hash: 'overblik', aabn: ['#indst-knap', '#forb-tilfoej', ['#skift-felt', 'x'.repeat(40)], '#skift-gem'], rod: '#omdoeb-dialog', fokus: '#omdoeb-felt', luk: '#omdoeb-nej', ekstra: DELT_TILFOEJ },
  { id: 'konti-omdoeb-gemt', navn: 'Omdøb og Gem: beskeden i rækken og fokus på Omdøb…', hash: 'overblik', aabn: ['#indst-knap', '#forb-2-omdoeb', '#omdoeb-gem'], rod: '#forb-liste [data-forbindelse="2"]', fokus: '#forb-2-omdoeb', luk: '#indst-luk', ekstra: DELT },
);
