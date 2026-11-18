// V-SIGN.1: tools/signer.sh prøvet uden at signere, notarisere eller røre Nøglering.
//
// Scriptet kopieres til et midlertidigt git-repo med HOME i en midlertidig mappe. De værktøjer, der signerer, sender
// til Apple eller ændrer launchd (codesign, xcrun, spctl, launchctl), og bygningen (npm) er erstattet af små stubbe
// forrest i PATH, som kun skriver deres argumenter i en log. Ægte er kun de lokale værktøjer (git, touch, find, stat,
// ditto, xattr, plutil, date). Alle identiteter og profiler er opdigtede.
//
// V-PUDS.2: testrepoet har en commit, en .gitignore (target/, signer.lokal) og en "origin", der er en lokal, tom
// git-mappe i den midlertidige mappe (intet net). Det er kun testens egne, midlertidige repoer; programmets eget repo røres
// ikke. Bundlen hedder "Huginn and Muninn.app": mellemrummet følges gennem hver kommando, zip-filen og launchd-plisten.
//
//   node tests/signer/signer.mjs
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
// SIGNER_SCRIPT: kun til at prøve testen selv mod en ændret kopi (mutationer); standard er repoets script.
const SCRIPT = process.env.SIGNER_SCRIPT ?? path.join(REPO, "tools/signer.sh");
const IDENTITET = "Developer ID Application: Prøve Person (TEST000000)";
const PROFIL = "proeve-profil";
// Versionen læses fra programmets egen tauri.conf.json (testrepoet får en kopi), så testen følger et versionsskift.
const VER = JSON.parse(fs.readFileSync(path.join(REPO, "src-tauri/tauri.conf.json"), "utf8")).version;

let ok = 0;
const fejl = [];
function tjek(navn, betingelse, detalje = "") {
  if (betingelse) ok++;
  else fejl.push(`${navn}${detalje ? `: ${detalje}` : ""}`);
}

// ---- klokkeslæt omkring nu (minutter siden midnat) ----
const pad = (n) => String(n).padStart(2, "0");
const nuDato = new Date();
const nuMin = nuDato.getHours() * 60 + nuDato.getMinutes();
const klokkenFra = (min) => {
  const m = ((min % 1440) + 1440) % 1440;
  return `${pad(Math.floor(m / 60))}:${pad(m % 60)}:00`;
};
const IND = klokkenFra(nuMin - 1); // vinduet (30 min) er åbent nu
const UDE = klokkenFra(nuMin + 180); // vinduet åbner om tre timer

// ---- midlertidigt repo med stubbe ----
const STUBBE = {
  // npm run tauri build -- [--debug] --no-sign → laver en lille bundle som Tauri, og skriver miljøet i loggen
  npm: `#!/bin/sh
echo "npm $*" >> "$STUB_LOG"
echo "npm-miljoe APPLE_SIGNING_IDENTITY=\${APPLE_SIGNING_IDENTITY:-} SOURCE_DATE_EPOCH=\${SOURCE_DATE_EPOCH:-} CARGO_ENCODED_RUSTFLAGS=$(printf '%s' "\${CARGO_ENCODED_RUSTFLAGS:-}" | tr '\\037' ' ')" >> "$STUB_LOG"
case " $* " in *" --debug "*) p=debug ;; *) p=release ;; esac
a="src-tauri/target/$p/bundle/macos/Huginn and Muninn.app"
rm -rf "$a"; mkdir -p "$a/Contents/MacOS" "$a/Contents/Resources"
printf '%s' "\${STUB_BINAER:-opdigtet binaer}" > "$a/Contents/MacOS/huginn-muninn"
[ -z "\${STUB_BYG_TIL:-}" ] || printf '%s' "$STUB_BYG_TIL" > "$STUB_NU_FIL"
printf '<plist/>' > "$a/Contents/Info.plist"
printf 'ikon' > "$a/Contents/Resources/icon.icns"
ln -s ../Info.plist "$a/Contents/Resources/link"
`,
  // codesign: skriver loggen; ved --sign lægges en signaturfil med ny filtid i bundlen, som ægte codesign gør
  codesign: `#!/bin/sh
echo "codesign $*" >> "$STUB_LOG"
case " $* " in *" --sign "*) for x; do a=$x; done; mkdir -p "$a/Contents/_CodeSignature"; printf 'sig' > "$a/Contents/_CodeSignature/CodeResources" ;; esac
exit 0
`,
  xcrun: `#!/bin/sh
echo "xcrun $*" >> "$STUB_LOG"
case "$1 $2" in
  "notarytool submit") if [ -n "\${STUB_HAENG:-}" ]; then echo $$ > "$STUB_HAENG"; sleep 6007; fi; printf '{"id":"00000000-0000-0000-0000-000000000001","status":"%s","message":"opdigtet"}\\n' "\${STUB_STATUS:-Accepted}" ;;
  "notarytool log") for x; do f=$x; done; printf '{"logFormatVersion":1,"status":"opdigtet"}\\n' > "$f" ;;
  "stapler staple") for x; do a=$x; done; printf 'billet' > "$a/Contents/CodeResources" ;;
esac
exit 0
`,
  spctl: `#!/bin/sh
echo "spctl $*" >> "$STUB_LOG"
exit 0
`,
  launchctl: `#!/bin/sh
echo "launchctl $*" >> "$STUB_LOG"
[ "$1" = print ] && exit 113
exit 0
`,
  // date (V-VINDUE.1): uden STUB_NU er det det ægte ur. Med STUB_NU ("ÅÅÅÅ-MM-DD TT:MM:SS") svarer date, som om klokken
  // var dét; findes filen i STUB_NU_FIL, gælder dens indhold i stedet (npm-stubben kan stille uret frem med STUB_BYG_TIL,
  // som om bygningen tog tid). Kald med -f (scriptets egne omregninger af en given dato) går uændret til det ægte date.
  // Stubben kalder altid det ægte date med -j, så uret aldrig kan blive stillet.
  date: `#!/bin/sh
if [ -n "\${STUB_NU_FIL:-}" ] && [ -f "$STUB_NU_FIL" ]; then STUB_NU=$(cat "$STUB_NU_FIL"); fi
if [ -z "\${STUB_NU:-}" ]; then exec /bin/date "$@"; fi
for a; do if [ "$a" = -f ]; then exec /bin/date "$@"; fi; done
fmt=""
for a; do
  shift
  case $a in +*) fmt=$a ;; *) set -- "$@" "$a" ;; esac
done
exec /bin/date -j -f '%Y-%m-%d %H:%M:%S' "$@" "$STUB_NU" \${fmt:+"$fmt"}
`,
};

function nytRepo() {
  const rod = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "signer-test-")));
  const repo = path.join(rod, "repo");
  const hjem = path.join(rod, "hjem");
  const stub = path.join(rod, "stub");
  for (const d of [repo, hjem, stub, path.join(repo, "tools"), path.join(repo, "src-tauri")]) fs.mkdirSync(d, { recursive: true });
  fs.copyFileSync(SCRIPT, path.join(repo, "tools/signer.sh"));
  for (const f of ["tauri.conf.json", "Huginn-Muninn.entitlements"]) fs.copyFileSync(path.join(REPO, "src-tauri", f), path.join(repo, "src-tauri", f));
  for (const [navn, indhold] of Object.entries(STUBBE)) fs.writeFileSync(path.join(stub, navn), indhold, { mode: 0o755 });
  fs.writeFileSync(path.join(repo, ".gitignore"), "/target/\n/src-tauri/target/\n/tools/signer.lokal\n");
  const r = { rod, repo, hjem, stub, log: path.join(rod, "stub.log"), origin: path.join(rod, "origin.git") };
  git(r, "init", "-q", "-b", "main");
  git(r, "add", "-A");
  git(r, "commit", "-q", "-m", "opdigtet");
  execFileSync("git", ["init", "-q", "--bare", r.origin], { env: gitMiljoe(r) });
  git(r, "remote", "add", "origin", r.origin);
  git(r, "push", "-q", "-u", "origin", "main");
  return r;
}

// git i testrepoet med en midlertidig HOME (ingen global opsætning, hooks eller signering fra maskinen).
function gitMiljoe(r) {
  return { PATH: "/usr/bin:/bin", HOME: r.hjem, GIT_CONFIG_NOSYSTEM: "1", GIT_AUTHOR_NAME: "Prøve Person", GIT_AUTHOR_EMAIL: "proeve@eksempel.dk", GIT_COMMITTER_NAME: "Prøve Person", GIT_COMMITTER_EMAIL: "proeve@eksempel.dk" };
}
function git(r, ...args) {
  return execFileSync("git", args, { cwd: r.repo, env: gitMiljoe(r), encoding: "utf8" });
}

// vinduer: værdien af git config fasttid.vinduer (null: ikke sat, som i alle tjek fra før V-VINDUE.1).
// nu: "ÅÅÅÅ-MM-DD TT:MM:SS" til date-stubben (null: det ægte ur).
function koer(r, args, { klokken = IND, miljoe = {}, lokal = null, vinduer = null, nu = null } = {}) {
  git(r, "config", "fasttid.klokken", klokken);
  if (vinduer === null) spawnSync("git", ["config", "--unset-all", "fasttid.vinduer"], { cwd: r.repo, env: gitMiljoe(r) });
  else git(r, "config", "fasttid.vinduer", vinduer);
  fs.rmSync(path.join(r.rod, "nu.txt"), { force: true });
  const lokalSti = path.join(r.repo, "tools/signer.lokal");
  if (lokal === null) fs.rmSync(lokalSti, { force: true });
  else fs.writeFileSync(lokalSti, lokal);
  fs.writeFileSync(r.log, "");
  const env = {
    PATH: `${r.stub}:/usr/bin:/bin:/usr/sbin:/sbin`,
    HOME: r.hjem,
    LANG: "en_US.UTF-8",
    STUB_LOG: r.log,
    APPLE_SIGNING_IDENTITY: IDENTITET,
    STUB_NU: nu ?? undefined,
    STUB_NU_FIL: path.join(r.rod, "nu.txt"),
    ...miljoe,
  };
  for (const [k, v] of Object.entries(env)) if (v === undefined) delete env[k];
  const res = spawnSync("sh", [path.join(r.repo, "tools/signer.sh"), ...args], { cwd: r.repo, env, encoding: "utf8" });
  const linjer = res.stdout.split("\n").filter((l) => l.startsWith("+ "));
  return { status: res.status, ud: res.stdout, fejl: res.stderr, linjer, stub: fs.readFileSync(r.log, "utf8").split("\n").filter(Boolean) };
}

const indeks = (linjer, del) => linjer.findIndex((l) => l.includes(del));
// Kører et lokalt værktøj; fejler det (fx fordi filen mangler), bliver det en fejl i tjek, ikke et nedbrud.
const lokalt = (cmd, args) => { try { return execFileSync(cmd, args, { encoding: "utf8" }); } catch { return ""; } };
const lokalProfil = `profil=${PROFIL}\n`;
const tidMinut = (s) => s.slice(0, 5);

// App-specifik adgangskode (fire grupper á fire små bogstaver) og et adgangskodeflag; bygget, så testen ikke selv matcher.
const APP_KODE = new RegExp(["[a-z]{4}"].concat(Array(3).fill("[a-z]{4}")).join("-"));
const KODE_FLAG = "--" + "pass" + "word";

// ---- 1. syntaks ----
{
  const r = spawnSync("sh", ["-n", SCRIPT], { encoding: "utf8" });
  tjek("sh -n tools/signer.sh", r.status === 0, r.stderr);
  const kilde = fs.readFileSync(SCRIPT, "utf8");
  // Stubbene virker kun, hvis scriptet kalder værktøjerne ved navn; en fast sti ville ramme det ægte codesign.
  tjek("ingen faste stier til codesign, xcrun, spctl, launchctl eller npm", !/\/usr\/bin\/(codesign|xcrun|spctl|launchctl)|\/npm\b/.test(kilde));
  tjek("scriptet nævner ingen adgangskode", !APP_KODE.test(kilde) && !kilde.includes(KODE_FLAG));
}

const r = nytRepo();
try {
  // ---- 2. proeve --toer ----
  {
    const k = koer(r, ["proeve", "--toer"], { lokal: lokalProfil });
    const L = k.linjer;
    tjek("proeve --toer exit 0", k.status === 0, k.fejl);
    tjek("proeve --toer kører ingen værktøjer", k.stub.length === 0, k.stub.join(" | "));
    tjek("proeve --toer skriver intet til disken", !fs.existsSync(path.join(r.repo, "src-tauri/target")));
    const byg = indeks(L, "+ npm run tauri build -- --debug --no-sign");
    const sign = indeks(L, "+ codesign --force --sign");
    tjek("proeve: debug-bygning uden Tauris signering", byg >= 0);
    tjek("proeve: bygning før signering", byg >= 0 && sign > byg);
    tjek("proeve: --timestamp=none", sign >= 0 && L[sign].includes(" --timestamp=none "));
    tjek("proeve: intet sikkert tidsstempel", !L.some((l) => / --timestamp( |$)/.test(l)));
    tjek("proeve: hardened runtime og entitlements", sign >= 0 && L[sign].includes("--options runtime") && L[sign].includes("src-tauri/Huginn-Muninn.entitlements"));
    tjek("proeve: identiteten fra miljøet", sign >= 0 && L[sign].includes(`'${IDENTITET}'`));
    tjek("proeve: codesign -dv og -d -r- bagefter", indeks(L, "+ codesign -dv ") > sign && indeks(L, "+ codesign -d -r- ") > sign);
    tjek("proeve: ingen notarisering", !L.some((l) => l.includes("notarytool") || l.includes("stapler")));
    tjek("proeve: APPLE_SIGNING_IDENTITY sættes af scriptet", indeks(L, `+ export APPLE_SIGNING_IDENTITY=${IDENTITET}`) >= 0 && indeks(L, "+ export APPLE_SIGNING_IDENTITY") < byg);
  }

  // ---- 3. identitet: miljø > signer.lokal > standard ----
  {
    const lokal = "identitet=Developer ID Application: Lokal Person (LOKAL00000)\n";
    const a = koer(r, ["proeve", "--toer"], { lokal, miljoe: { APPLE_SIGNING_IDENTITY: undefined } });
    tjek("identitet fra signer.lokal", a.linjer.some((l) => l.includes("--sign 'Developer ID Application: Lokal Person (LOKAL00000)'")));
    const b = koer(r, ["proeve", "--toer"], { lokal, miljoe: { APPLE_SIGNING_IDENTITY: IDENTITET } });
    tjek("miljøet vinder over signer.lokal", b.linjer.some((l) => l.includes(`--sign '${IDENTITET}'`)));
    const c = koer(r, ["proeve", "--toer"], { miljoe: { APPLE_SIGNING_IDENTITY: undefined } });
    tjek("standardidentiteten uden miljø og fil", c.linjer.some((l) => l.includes("--sign 'Developer ID Application: Jan Tingsvad (5DAVNFA355)'")));
  }

  // ---- 4. udgiv --toer i vinduet ----
  {
    const k = koer(r, ["udgiv", "--toer"], { lokal: lokalProfil });
    const L = k.linjer;
    tjek("udgiv --toer exit 0 i vinduet", k.status === 0, k.fejl);
    tjek("udgiv --toer kører ingen værktøjer", k.stub.length === 0, k.stub.join(" | "));
    tjek("udgiv --toer skriver intet til disken", !fs.existsSync(path.join(r.hjem, "Huginn-Muninn-udgivelser")) && !fs.existsSync(path.join(r.repo, "src-tauri/target")));
    const i = {
      flag: indeks(L, "+ export CARGO_ENCODED_RUSTFLAGS="),
      epoch: indeks(L, "+ export SOURCE_DATE_EPOCH="),
      byg: indeks(L, "+ npm run tauri build -- --no-sign"),
      navne: indeks(L, "+ (kontrol) intet brugernavn"),
      xattr: indeks(L, "+ xattr -cr "),
      tid1: indeks(L, "+ find "),
      sign: indeks(L, "+ codesign --force --sign"),
      zip1: indeks(L, "+ ditto -c -k"),
      submit: indeks(L, "+ xcrun notarytool submit"),
      log: indeks(L, "+ xcrun notarytool log"),
      staple: indeks(L, "+ xcrun stapler staple"),
      verify: indeks(L, "+ codesign --verify --deep --strict"),
      spctl: indeks(L, "+ spctl -a -vv"),
      validate: indeks(L, "+ xcrun stapler validate"),
    };
    const at = (n) => L[i[n]] ?? ""; // et manglende trin giver en fejl i tjek, ikke et nedbrud
    const zip2 = L.findLastIndex((l) => l.startsWith("+ ditto -c -k"));
    const raekke = ["flag", "byg", "navne", "xattr", "tid1", "sign", "zip1", "submit", "staple", "verify", "spctl", "validate"];
    tjek("udgiv: alle trin findes", raekke.every((n) => i[n] >= 0), JSON.stringify(i));
    tjek("udgiv: rækkefølgen bygning → navne → filtider → signering → zip → notarisering → hæftning → kontrol", raekke.every((n, j) => j === 0 || i[raekke[j - 1]] < i[n]), JSON.stringify(i));
    tjek("udgiv: den endelige zip efter hæftning og kontrol", zip2 > i.validate);
    tjek("udgiv: release (ikke --debug)", !at("byg").includes("--debug"));
    tjek("udgiv: sikkert tidsstempel", / --timestamp '?\//.test(at("sign")) && !at("sign").includes("--timestamp=none"));
    tjek("udgiv: hardened runtime (--options runtime)", at("sign").includes("--options runtime"));
    tjek("udgiv: entitlements", at("sign").includes("src-tauri/Huginn-Muninn.entitlements"));
    tjek("udgiv: notarytool med profilnavnet og --wait", at("submit").includes(`--keychain-profile ${PROFIL}`) && at("submit").includes("--wait"));
    tjek("udgiv: notarytool log med samme profil", i.log > i.submit && at("log").includes(`--keychain-profile ${PROFIL}`));
    tjek("udgiv: zip og log uden for repoet", at("zip1").includes(r.hjem) && !at("zip1").includes(`${r.repo}/Huginn-Muninn`) && at("log").includes(r.hjem));
    tjek("udgiv: ditto uden udvidede attributter", at("zip1").includes("--noextattr") && at("zip1").includes("--norsrc") && at("zip1").includes("--keepParent"));
    const ts = `${new Date().getFullYear()}`;
    tjek("udgiv: filtider sættes til dagens dato og klokkeslættet", at("tid1").includes(`touch -h -t ${ts}`) && at("tid1").includes(`${IND.slice(0, 2)}${IND.slice(3, 5)}.00`));
    tjek("udgiv: stier omskrives (hjem, cargo, rustup, repo)", ["=/home", "=/cargo", "=/rustup", `${r.repo}=/huginn-muninn`].every((s) => at("flag").includes(s)) && at("flag").includes(`--remap-path-prefix=${r.hjem}=/home`));
    tjek("udgiv: SOURCE_DATE_EPOCH sat", i.epoch >= 0 && /=\d{10}$/.test(at("epoch")));
    tjek("udgiv: ingen adgangskode i output", !APP_KODE.test(k.ud + k.fejl) && !(k.ud + k.fejl).includes(KODE_FLAG));
  }

  // ---- 5. vagten ----
  {
    const k = koer(r, ["udgiv", "--toer"], { klokken: UDE, lokal: lokalProfil });
    tjek("vagt: udgiv uden for vinduet stopper (exit ≠ 0)", k.status !== 0);
    tjek("vagt: siger hvorfor", k.fejl.includes("stoppet") && k.fejl.includes(tidMinut(UDE)));
    tjek("vagt: intet bygget eller signeret", k.linjer.length === 0, k.linjer.join(" | "));
    const t = koer(r, ["udgiv", "--toer", "--uden-vagt"], { klokken: UDE, lokal: lokalProfil });
    tjek("vagt: --uden-vagt tilsidesætter", t.status === 0 && t.linjer.some((l) => l.startsWith("+ codesign --force --sign")));
    tjek("vagt: --uden-vagt advarer", t.fejl.includes("ADVARSEL"));
    const u = koer(r, ["udgiv"], { klokken: UDE, lokal: lokalProfil });
    tjek("vagt: også uden --toer, og ingen værktøjer kørt", u.status !== 0 && u.stub.length === 0, u.stub.join(" | "));
    const d = koer(r, ["udgiv", "--toer"], { klokken: klokkenFra(nuMin - 40), lokal: lokalProfil });
    tjek("vagt: vinduet lukker efter 30 minutter", d.status !== 0);
    git(r, "config", "signer.vindue", "60");
    const e = koer(r, ["udgiv", "--toer"], { klokken: klokkenFra(nuMin - 40), lokal: lokalProfil });
    tjek("vagt: git config signer.vindue 60 forlænger", e.status === 0, e.fejl);
    git(r, "config", "--unset", "signer.vindue");
    const f = koer(r, ["udgiv", "--toer"], { klokken: "25:99", lokal: lokalProfil });
    tjek("vagt: ugyldigt klokkeslæt afvises", f.status !== 0);
  }

  // ---- 6. tjek --toer ----
  {
    const k = koer(r, ["tjek", "/eksempel/Huginn and Muninn.app", "--toer"]);
    const L = k.linjer;
    tjek("tjek --toer exit 0", k.status === 0, k.fejl);
    tjek("tjek: kun kontroller", !L.some((l) => l.includes("--sign") || l.includes("notarytool") || l.includes("stapler staple")));
    tjek("tjek: codesign --verify --deep --strict, spctl -a -vv, stapler validate", ["+ codesign --verify --deep --strict", "+ spctl -a -vv", "+ xcrun stapler validate"].every((s) => indeks(L, s) >= 0));
    tjek("tjek uden sti fejler", koer(r, ["tjek"]).status !== 0);
  }

  // ---- 7. planlaeg / afplanlaeg / status ----
  {
    const k = koer(r, ["planlaeg", "--toer"], { klokken: "23:30:00" });
    tjek("planlaeg --toer exit 0", k.status === 0, k.fejl);
    tjek("planlaeg: kl. 23:30", k.ud.includes("<key>Hour</key><integer>23</integer><key>Minute</key><integer>30</integer>"));
    tjek("planlaeg: kører signer.sh koer", k.ud.includes(`<string>${r.repo}/tools/signer.sh</string><string>koer</string>`));
    tjek("planlaeg: launchctl bootstrap", indeks(k.linjer, "+ launchctl bootstrap gui/") >= 0);
    tjek("planlaeg --toer skriver ingen plist", !fs.existsSync(path.join(r.hjem, "Library")) && k.stub.length === 0);
    const k2 = koer(r, ["planlaeg", "--toer"], { klokken: "08:05:00" });
    tjek("planlaeg: følger fasttid.klokken (08:05)", k2.ud.includes("<integer>8</integer><key>Minute</key><integer>5</integer>"));
    const a = koer(r, ["afplanlaeg", "--toer"]);
    tjek("afplanlaeg --toer", a.status === 0 && indeks(a.linjer, "+ launchctl bootout") >= 0 && indeks(a.linjer, "+ rm -f ") >= 0);
    const s = koer(r, ["status"], { lokal: lokalProfil });
    tjek("status", s.status === 0 && s.ud.includes("ikke planlagt") && s.ud.includes(`profil:    ${PROFIL}`));
  }

  // ---- 8. udgiv med stubbe: filtider, zip, rækkefølge ----
  {
    const k = koer(r, ["udgiv"], { lokal: lokalProfil });
    tjek("udgiv (stubbe) exit 0", k.status === 0, k.fejl + k.ud);
    const S = k.stub;
    const si = (del) => S.findIndex((l) => l.startsWith(del));
    tjek("udgiv (stubbe): bygning, signering, notarisering, hæftning i den rækkefølge",
      si("npm run tauri build -- --no-sign") >= 0 && si("npm run") < si("codesign --force") && si("codesign --force") < si("xcrun notarytool submit") && si("xcrun notarytool submit") < si("xcrun stapler staple"));
    const miljoe = S.find((l) => l.startsWith("npm-miljoe")) ?? "";
    tjek("udgiv (stubbe): bygningen får identitet, SOURCE_DATE_EPOCH og omskrivning", miljoe.includes(`APPLE_SIGNING_IDENTITY=${IDENTITET}`) && /SOURCE_DATE_EPOCH=\d{10}/.test(miljoe) && miljoe.includes(`--remap-path-prefix=${r.hjem}=/home`));
    const app = path.join(r.repo, "src-tauri/target/release/bundle/macos/Huginn and Muninn.app");
    const tider = lokalt("find", [app, "-exec", "stat", "-f", "%Sm", "-t", "%Y-%m-%d %H:%M:%S", "{}", "+"]).trim().split("\n");
    const forventet = `${nuDato.getFullYear()}-${pad(nuDato.getMonth() + 1)}-${pad(nuDato.getDate())}`;
    const dag = nuMin < 1 ? null : forventet; // lige efter midnat er klokkeslættet gårsdagens
    tjek("filtider: alle filer, mapper og links i bundlen har klokkeslættet", tider.length >= 8 && tider.every((t) => t.endsWith(IND)), tider.join(", "));
    tjek("filtider: dagens dato", dag === null || tider.every((t) => t.startsWith(dag)), tider[0]);
    tjek("filtider: også signaturen og billetten, der kom til efter første touch", fs.existsSync(path.join(app, "Contents/_CodeSignature/CodeResources")) && fs.existsSync(path.join(app, "Contents/CodeResources")));
    const ud = path.join(r.hjem, "Huginn-Muninn-udgivelser");
    const zip = path.join(ud, `Huginn-Muninn-${VER}.zip`);
    tjek("zip: den endelige zip ligger uden for repoet", fs.existsSync(zip));
    tjek("zip: zip-filen til notarisering er fjernet", !fs.existsSync(path.join(ud, `Huginn-Muninn-${VER}-til-notarisering.zip`)));
    tjek("zip: zip-filens egen filtid", lokalt("stat", ["-f", "%Sm", "-t", "%H:%M:%S", zip]).trim() === IND);
    const poster = lokalt("unzip", ["-Z", "-T", zip]).split("\n").filter((l) => l.includes("Huginn and Muninn.app"));
    const hhmmss = IND.replaceAll(":", "");
    tjek("zip: alle poster har klokkeslættet", poster.length >= 8 && poster.every((l) => l.includes(`.${hhmmss} `)), poster.join(" | "));
    tjek("zip: ingen __MACOSX/udvidede attributter", fs.existsSync(zip) && !lokalt("unzip", ["-Z", "-1", zip]).includes("__MACOSX"));
    tjek("notarisering: loggen hentet til en fil uden for repoet", fs.existsSync(path.join(ud, "notarisering-00000000-0000-0000-0000-000000000001.json")));
    // V-PUDS.2 N2: version og commit i den sidste linje og i en tekstfil ved zip-filen, uden klokkeslæt.
    const h = git(r, "rev-parse", "--short", "HEAD").trim();
    const txt = path.join(ud, `Huginn-Muninn-${VER}.txt`);
    const indhold = fs.existsSync(txt) ? fs.readFileSync(txt, "utf8") : "";
    tjek("N2: tekstfilen ved zip-filen: version og commit", indhold === `Huginn and Muninn ${VER}\ncommit ${h}\n`, JSON.stringify(indhold));
    tjek("N2: tekstfilen har intet klokkeslæt, og dens filtid er den faste", !/\d\d:\d\d/.test(indhold) && lokalt("stat", ["-f", "%Sm", "-t", "%H:%M:%S", txt]).trim() === IND);
    const sidste = k.ud.trim().split("\n").pop();
    tjek("N2: den sidste linje har version og commit", sidste.startsWith("== færdig: ") && sidste.includes(`Huginn and Muninn ${VER}, commit ${h}`), sidste);
    tjek("N2: rent og pushet står i loggen", k.ud.includes(`kode: arbejdstræet er rent, og HEAD ${h} findes på origin/main`));
    tjek("N2: intet midlertidigt zip-navn tilbage", !fs.readdirSync(ud).some((f) => f.endsWith(".ufaerdig")), fs.readdirSync(ud).join(", "));
  }

  // ---- 9. udgiv med stubbe: notariseringen afvist ----
  {
    fs.rmSync(path.join(r.hjem, "Huginn-Muninn-udgivelser"), { recursive: true, force: true });
    const k = koer(r, ["udgiv"], { lokal: lokalProfil, miljoe: { STUB_STATUS: "Invalid" } });
    tjek("afvist: exit ≠ 0", k.status !== 0);
    tjek("afvist: loggen hentet uden for repoet", fs.existsSync(path.join(r.hjem, "Huginn-Muninn-udgivelser/notarisering-00000000-0000-0000-0000-000000000001.json")));
    tjek("afvist: notarytool log med profilen", k.stub.some((l) => l.startsWith("xcrun notarytool log 00000000-0000-0000-0000-000000000001 --keychain-profile " + PROFIL)));
    tjek("afvist: intet hæftet", !k.stub.some((l) => l.startsWith("xcrun stapler")));
    tjek("afvist: ingen endelig zip", !fs.existsSync(path.join(r.hjem, `Huginn-Muninn-udgivelser/Huginn-Muninn-${VER}.zip`)));
    tjek("afvist: siger hvorfor", k.fejl.includes("ikke godkendt") && k.fejl.includes("Invalid"));
  }

  // ---- 10. udgiv med stubbe: hjemmemappen i binæren stopper før signering ----
  {
    const k = koer(r, ["udgiv"], { lokal: lokalProfil, miljoe: { STUB_BINAER: `panik i ${r.hjem}/kilde.rs` } });
    tjek("navne: exit ≠ 0", k.status !== 0);
    tjek("navne: intet signeret", !k.stub.some((l) => l.startsWith("codesign")), k.stub.join(" | "));
    tjek("navne: siger hvilken fil, men ikke værdien", k.fejl.includes("hjemmemappen står i Contents/MacOS/huginn-muninn") && !k.fejl.includes(`${r.hjem}/kilde`));
  }

  // ---- 11. proeve med stubbe ----
  {
    const k = koer(r, ["proeve"], { lokal: lokalProfil });
    tjek("proeve (stubbe) exit 0", k.status === 0, k.fejl);
    const sign = k.stub.find((l) => l.startsWith("codesign --force")) ?? "";
    tjek("proeve (stubbe): --timestamp=none og --options runtime", sign.includes("--timestamp=none") && sign.includes("--options runtime"));
    tjek("proeve (stubbe): ingen kontakt til Apple", !k.stub.some((l) => l.startsWith("xcrun")));
  }

  // ---- 17. mellemrummet i bundlens navn ----
  {
    const APP = "Huginn and Muninn.app";
    const rel = `'${r.repo}/src-tauri/target/release/bundle/macos/${APP}'`;
    const t = koer(r, ["udgiv", "--toer"], { lokal: lokalProfil });
    const med = t.linjer.filter((l) => l.includes(APP));
    tjek("mellemrum: tørkørslen nævner bundlen i hver kommando, der rører den", med.length >= 10, String(med.length));
    tjek("mellemrum: i hver kommandolinje står bundlens sti som ét argument i anførselstegn", med.filter((l) => !l.startsWith("+ (")).every((l) => l.includes(rel)), med.find((l) => !l.startsWith("+ (") && !l.includes(rel)) ?? "");
    tjek("mellemrum: ingen kommandolinje har stien delt ved et mellemrum", !t.linjer.some((l) => !l.startsWith("+ (") && l.replaceAll(rel, "").includes("Muninn.app")), t.linjer.find((l) => !l.startsWith("+ (") && l.replaceAll(rel, "").includes("Muninn.app")) ?? "");
    const p = koer(r, ["proeve", "--toer"], { lokal: lokalProfil });
    const deb = `'${r.repo}/src-tauri/target/debug/bundle/macos/${APP}'`;
    tjek("mellemrum: proeve signerer debug-bundlen under src-tauri/target som ét argument", p.linjer.some((l) => l.startsWith("+ codesign --force --sign") && l.endsWith(` ${deb}`)), p.linjer.find((l) => l.startsWith("+ codesign --force")) ?? "");
    const j = koer(r, ["tjek", `/eksempel/mappe med rum/${APP}`, "--toer"]);
    tjek("mellemrum: tjek tager en sti med mellemrum som ét mål", j.status === 0 && j.linjer.some((l) => l === `+ codesign -dvv '/eksempel/mappe med rum/${APP}'`), j.linjer[0] ?? j.fejl);
    // Med stubbe: zip-filen har bundlen med sit rigtige navn, filtiderne er sat på alt under den, og filnavnene udenfor har ingen mellemrum.
    fs.rmSync(path.join(r.hjem, "Huginn-Muninn-udgivelser"), { recursive: true, force: true });
    const k = koer(r, ["udgiv"], { lokal: lokalProfil });
    const ud = path.join(r.hjem, "Huginn-Muninn-udgivelser");
    const zip = path.join(ud, `Huginn-Muninn-${VER}.zip`);
    const navne = lokalt("unzip", ["-Z", "-1", zip]).split("\n").filter(Boolean);
    tjek("mellemrum: ditto-zippen indeholder Huginn and Muninn.app/Contents/…", k.status === 0 && navne.includes(`${APP}/Contents/MacOS/huginn-muninn`) && navne.includes(`${APP}/Contents/Info.plist`) && navne.every((n) => n.startsWith(`${APP}/`)), navne.slice(0, 4).join(" | ") + k.fejl);
    tjek("mellemrum: filerne i udgivelsesmappen har ingen mellemrum i navnet", fs.existsSync(ud) && fs.readdirSync(ud).length >= 3 && fs.readdirSync(ud).every((f) => !f.includes(" ")), fs.existsSync(ud) ? fs.readdirSync(ud).join(", ") : "");
    const app = path.join(r.repo, "src-tauri/target/release/bundle/macos", APP);
    const tider = lokalt("find", [app, "-exec", "stat", "-f", "%Sm", "-t", "%H:%M:%S", "{}", "+"]).trim().split("\n");
    tjek("mellemrum: filtiderne er sat på alt under bundlen", tider.length >= 8 && tider.every((x) => x === IND), tider.join(","));
    const sign = k.stub.find((l) => l.startsWith("codesign --force")) ?? "";
    tjek("mellemrum: codesign fik hele stien (stubben lagde signaturen i den rigtige bundle)", sign.endsWith(`/${APP}`) && fs.existsSync(path.join(app, "Contents/_CodeSignature/CodeResources")) && !fs.existsSync(path.join(r.repo, "src-tauri/target/release/bundle/macos/Huginn")), sign);
    // Et plantet navn i en fil under en mappe med mellemrum findes, og linjen nævner filen, ikke værdien.
    const n = koer(r, ["udgiv"], { lokal: lokalProfil, miljoe: { STUB_BINAER: `sti ${r.hjem}/x` } });
    tjek("mellemrum: navnekontrollen finder et plantet navn under mappen med mellemrum", n.status !== 0 && n.fejl.includes("hjemmemappen står i Contents/MacOS/huginn-muninn") && !n.stub.some((l) => l.startsWith("codesign")), n.fejl);
    const pl = koer(r, ["planlaeg", "--toer"], { klokken: "23:30:00" });
    const arr = /<key>ProgramArguments<\/key>\s*<array>(.*?)<\/array>/s.exec(pl.ud)?.[1] ?? "";
    const dele = [...arr.matchAll(/<string>(.*?)<\/string>/g)].map((x) => x[1]);
    tjek("mellemrum: launchd-plistens ProgramArguments har stien som ét element", dele.length === 3 && dele[0] === "/bin/sh" && dele[1] === `${r.repo}/tools/signer.sh` && dele[2] === "koer", dele.join(" | "));
    tjek("jobbets etiket dannes af repoets mappenavn", pl.ud.includes("<key>Label</key><string>com.wi24rd.signer.repo</string>"));
    const st = koer(r, ["status"], {});
    tjek("standardprofilen er programmets egen, og udgivelsesmappen ligger i hjemmemappen", st.ud.includes("profil:    ham-notary") && st.ud.includes(`ud:        ${r.hjem}/Huginn-Muninn-udgivelser`), st.ud);
    const kilde = fs.readFileSync(SCRIPT, "utf8");
    tjek("scriptet kalder aldrig security og læser dermed ikke Nøglering selv", !/(^|[\s;|&(])security\s/m.test(kilde.replace(/^\s*#.*$/gm, "")));
  }

  // ---- 12. udgivelsesmappen må ikke ligge i repoet ----
  {
    const k = koer(r, ["udgiv", "--toer"], { lokal: `udgivelser=${r.repo}/ud\n` });
    tjek("udgivelser i repoet afvises", k.status !== 0 && k.fejl.includes("ligger i repoet"));
    const u = koer(r, ["proeve", "--toer", "--ukendt"]);
    tjek("ukendt flag afvises", u.status !== 0);
  }
  // ---- 15. N2: urent træ og upushet HEAD stopper udgiv før bygning ----
  {
    const stopper = (navn, k) => {
      tjek(`${navn}: exit ≠ 0`, k.status !== 0);
      tjek(`${navn}: intet bygget, signeret eller indsendt`, !k.linjer.some((l) => l.startsWith("+ npm") || l.startsWith("+ codesign") || l.startsWith("+ xcrun")) && k.stub.length === 0, k.linjer.concat(k.stub).join(" | "));
    };
    // En usporet fil.
    fs.writeFileSync(path.join(r.repo, "ny-fil.txt"), "opdigtet\n");
    let k = koer(r, ["udgiv", "--toer"], { lokal: lokalProfil });
    stopper("N2 usporet fil (--toer)", k);
    tjek("N2 usporet fil: siger hvorfor", k.fejl.includes("stoppet før bygning") && k.fejl.includes("arbejdstræet er ikke rent (1 ændrede"), k.fejl);
    stopper("N2 usporet fil, også med --uden-vagt uden for vinduet", koer(r, ["udgiv", "--toer", "--uden-vagt"], { klokken: UDE, lokal: lokalProfil }));
    stopper("N2 usporet fil, også uden --toer", koer(r, ["udgiv"], { lokal: lokalProfil }));
    // --urent tilsidesætter, med advarsel, og tekstfilen siger det.
    k = koer(r, ["udgiv", "--toer", "--urent"], { lokal: lokalProfil });
    tjek("N2 --urent: tilsidesætter (bygger og signerer)", k.status === 0 && k.linjer.some((l) => l.startsWith("+ codesign --force --sign")), k.fejl);
    tjek("N2 --urent: advarer", k.fejl.includes("ADVARSEL") && k.fejl.includes("--urent") && k.fejl.includes("ikke rent"), k.fejl);
    tjek("N2 --urent --toer: tekstfilen nævnt med urent", k.linjer.some((l) => l.includes(`Huginn-Muninn-${VER}.txt: Huginn and Muninn ${VER}, commit `) && l.endsWith(", urent arbejdstræ")));
    fs.rmSync(path.join(r.hjem, "Huginn-Muninn-udgivelser"), { recursive: true, force: true });
    k = koer(r, ["udgiv", "--urent"], { lokal: lokalProfil });
    const txt = path.join(r.hjem, `Huginn-Muninn-udgivelser/Huginn-Muninn-${VER}.txt`);
    const indhold = fs.existsSync(txt) ? fs.readFileSync(txt, "utf8") : "";
    tjek("N2 --urent (stubbe): tekstfilen siger urent", k.status === 0 && indhold.includes("arbejdstræ: urent (--urent)"), k.fejl + indhold);
    tjek("N2 --urent (stubbe): den sidste linje siger urent", k.ud.trim().split("\n").pop().endsWith(", urent)"));
    fs.rmSync(path.join(r.repo, "ny-fil.txt"));
    // En ændret, sporet fil.
    fs.appendFileSync(path.join(r.repo, ".gitignore"), "# ændret\n");
    k = koer(r, ["udgiv", "--toer"], { lokal: lokalProfil });
    stopper("N2 ændret fil", k);
    git(r, "checkout", "-q", "--", ".gitignore");
    // En commit, der ikke er pushet.
    fs.writeFileSync(path.join(r.repo, "ny-fil.txt"), "opdigtet\n");
    git(r, "add", "ny-fil.txt");
    git(r, "commit", "-q", "-m", "ikke pushet");
    k = koer(r, ["udgiv", "--toer"], { lokal: lokalProfil });
    stopper("N2 upushet HEAD", k);
    tjek("N2 upushet HEAD: siger hvorfor", k.fejl.includes("er ikke pushet til origin/main"), k.fejl);
    stopper("N2 upushet HEAD, også med --uden-vagt", koer(r, ["udgiv", "--toer", "--uden-vagt"], { klokken: UDE, lokal: lokalProfil }));
    const kilde = fs.readFileSync(path.join(r.repo, "tools/signer.sh"), "utf8");
    tjek("N2: scriptet henter og pusher aldrig selv", !/git[^\n]*\b(fetch|push|pull)\b/.test(kilde));
    // Ingen sporet gren.
    git(r, "push", "-q", "origin", "main");
    git(r, "branch", "-q", "--unset-upstream");
    k = koer(r, ["udgiv", "--toer"], { lokal: lokalProfil });
    stopper("N2 ingen sporet gren", k);
    tjek("N2 ingen sporet gren: siger hvorfor", k.fejl.includes("sporer ingen gren på origin"), k.fejl);
    git(r, "branch", "-q", "--set-upstream-to=origin/main");
    k = koer(r, ["udgiv", "--toer"], { lokal: lokalProfil });
    tjek("N2: rent og pushet igen: udgiv kører", k.status === 0, k.fejl);
  }

  // ---- 16. N3: koer med tidsgrænse ----
  {
    const ud = path.join(r.hjem, "Huginn-Muninn-udgivelser");
    // Uden hængende kald: udgiv lykkes, og jobbet fjerner sig selv.
    fs.rmSync(ud, { recursive: true, force: true });
    let k = koer(r, ["koer"], { lokal: lokalProfil, miljoe: { SIGNER_GRAENSE: "60" } });
    tjek("N3 koer: exit 0 og jobbet fjernes", k.status === 0 && k.ud.includes("udgiv lykkedes; jobbet fjernes") && k.stub.some((l) => l.startsWith("launchctl bootout")), k.ud + k.fejl);
    tjek("N3 koer: tidsgrænsen står i loggen", k.ud.includes("tidsgrænse: 60 s"));
    // notarytool hænger (som en dialog, ingen svarer på): jobbet stoppes ved grænsen.
    fs.rmSync(ud, { recursive: true, force: true });
    const pidfil = path.join(r.rod, "haeng.pid");
    const start = Date.now();
    k = koer(r, ["koer"], { lokal: lokalProfil, miljoe: { SIGNER_GRAENSE: "2", STUB_HAENG: pidfil } });
    const sek = (Date.now() - start) / 1000;
    if (process.env.SIGNER_VIS) console.log(`--- koer med hængende notarytool (${sek} s), stdout:\n${k.ud}--- stderr:\n${k.fejl}---`);
    tjek("N3 hænger: afbrudt (exit ≠ 0) inden for få sekunder", k.status !== 0 && sek < 20, `${sek} s, exit ${k.status}`);
    tjek("N3 hænger: notarytool var i gang", fs.existsSync(pidfil) && k.stub.some((l) => l.startsWith("xcrun notarytool submit")));
    tjek("N3 hænger: loglinjen siger hvad der skete", k.ud.includes("tidsgrænse: udgiv afbrudt efter 2 s") && k.ud.includes(`Fjernet: Huginn-Muninn-${VER}-til-notarisering.zip`), k.ud);
    tjek("N3 hænger: loglinjen om den indsendte notarisering", k.ud.includes("næste aften bygges, signeres og indsendes en ny"));
    tjek("N3 hænger: jobbet bliver stående (ingen bootout) og prøver igen", !k.stub.some((l) => l.startsWith("launchctl")) && k.ud.includes("jobbet prøver igen kl."), k.stub.join(" | "));
    const tilbage = fs.existsSync(ud) ? fs.readdirSync(ud) : [];
    tjek("N3 hænger: ingen zip (hel eller halv) i udgivelsesmappen", !tilbage.some((f) => f.includes(".zip")), tilbage.join(", "));
    const pid = fs.existsSync(pidfil) ? Number(fs.readFileSync(pidfil, "utf8")) : 0;
    let lever = false;
    try { process.kill(pid, 0); lever = true; } catch { /* stoppet */ }
    tjek("N3 hænger: det hængende kald er stoppet", pid > 0 && !lever, String(pid));
    tjek("N3 hænger: også barnebarnet (sleep) er stoppet", lokalt("pgrep", ["-f", "sleep 6007"]).trim() === "");
    tjek("N3: ugyldig grænse afvises", koer(r, ["koer"], { lokal: lokalProfil, miljoe: { SIGNER_GRAENSE: "x" } }).status !== 0);
    const kilde = fs.readFileSync(SCRIPT, "utf8");
    tjek("N3: intet timeout fra coreutils", !/\b(g?timeout)\s/.test(kilde.replace(/^\s*#.*$/gm, "")));
  }
  // ---- 18. V-VINDUE.1: to faste vinduer (git config fasttid.vinduer) ----
  // Uret er date-stubben med et fast, opdigtet tidspunkt; intet tjek her afhænger af det ægte ur.
  {
    const V = "06:30-07:30,19:30-20:30";
    const D = "2026-03-10";
    const med = (args, tid, ekstra = {}) => koer(r, args, { lokal: lokalProfil, vinduer: V, nu: `${D} ${tid}`, ...ekstra });
    const app = path.join(r.repo, "src-tauri/target/release/bundle/macos/Huginn and Muninn.app");
    const ud = path.join(r.hjem, "Huginn-Muninn-udgivelser");
    const touch = (k) => k.linjer.find((l) => l.startsWith("+ find ")) ?? "";

    // Åbent i det første og i det andet vindue; filtiden er det åbne vindues start.
    let k = med(["udgiv", "--toer"], "06:45:00");
    tjek("vinduer: åbent i det første (06:45)", k.status === 0 && k.linjer.some((l) => l.startsWith("+ codesign --force --sign")), k.fejl);
    tjek("vinduer: filtiden er det første vindues start", touch(k).includes("touch -h -t 202603100630.00 "), touch(k));
    tjek("vinduer: tørkørslen i et vindue kører ingen værktøjer", k.stub.length === 0, k.stub.join(" | "));
    k = med(["udgiv", "--toer"], "19:45:00");
    tjek("vinduer: åbent i det andet (19:45)", k.status === 0 && k.linjer.some((l) => l.startsWith("+ xcrun notarytool submit")), k.fejl);
    tjek("vinduer: filtiden er det andet vindues start", touch(k).includes("touch -h -t 202603101930.00 "), touch(k));
    tjek("vinduer: SOURCE_DATE_EPOCH er vinduets start", k.linjer.some((l) => l === `+ export SOURCE_DATE_EPOCH=${Math.floor(new Date(2026, 2, 10, 19, 30, 0).getTime() / 1000)}`), k.linjer.find((l) => l.includes("SOURCE_DATE_EPOCH")) ?? "");

    // Lukket imellem og om natten; fejlteksten nævner vinduerne og det næste.
    const lukket = (navn, tid, naeste) => {
      const x = med(["udgiv", "--toer"], tid);
      tjek(`vinduer: lukket ${navn} (exit ≠ 0, intet bygget eller signeret)`, x.status !== 0 && x.linjer.length === 0 && x.stub.length === 0, x.linjer.join(" | "));
      tjek(`vinduer: lukket ${navn}: fejlteksten nævner vinduerne, klokken og det næste`, x.fejl.includes("udgiv stoppet") && x.fejl.includes("06:30-07:30, 19:30-20:30") && x.fejl.includes(`klokken er ${tid.slice(0, 5)}`) && x.fejl.includes(naeste), x.fejl);
    };
    lukket("imellem (12:00)", "12:00:00", "næste vindue åbner kl. 19:30 i dag");
    lukket("om aftenen (23:00)", "23:00:00", "næste vindue åbner kl. 06:30 i morgen");
    lukket("om natten (03:00)", "03:00:00", "næste vindue åbner kl. 06:30 i dag");
    k = med(["udgiv"], "12:00:00");
    tjek("vinduer: lukket også uden --toer, og ingen værktøjer kørt", k.status !== 0 && k.stub.length === 0, k.stub.join(" | "));

    // Grænserne: første og sidste minut i hvert vindue, og minuttet før og efter.
    for (const [tid, aaben] of [["06:29:59", false], ["06:30:00", true], ["07:29:59", true], ["07:30:00", false], ["19:29:59", false], ["19:30:00", true], ["20:29:59", true], ["20:30:00", false]]) {
      const x = med(["udgiv", "--toer"], tid);
      tjek(`vinduer: grænse ${tid} er ${aaben ? "åben" : "lukket"}`, (x.status === 0) === aaben, x.fejl);
    }

    // --uden-vagt uden for vinduerne: advarsel, og filtiden er det næste vindues start (næste dag efter det sidste).
    k = med(["udgiv", "--toer", "--uden-vagt"], "12:00:00");
    tjek("vinduer: --uden-vagt tilsidesætter og advarer", k.status === 0 && k.fejl.includes("ADVARSEL") && k.fejl.includes("uden for vinduerne 06:30-07:30, 19:30-20:30") && k.linjer.some((l) => l.startsWith("+ codesign --force --sign")), k.fejl);
    tjek("vinduer: --uden-vagt kl. 12:00: filtiden er 19:30 samme dag", touch(k).includes("touch -h -t 202603101930.00 "), touch(k));
    k = med(["udgiv", "--toer", "--uden-vagt"], "23:00:00");
    tjek("vinduer: --uden-vagt kl. 23:00: filtiden er 06:30 næste dag", touch(k).includes("touch -h -t 202603110630.00 "), touch(k));

    // fasttid.klokken og signer.vindue bruges ikke, når fasttid.vinduer er sat.
    git(r, "config", "signer.vindue", "600");
    k = med(["udgiv", "--toer"], "12:00:00", { klokken: "11:59:00" });
    tjek("vinduer: fasttid.klokken og signer.vindue åbner ikke et vindue", k.status !== 0 && k.fejl.includes("19:30"), k.fejl);
    git(r, "config", "--unset", "signer.vindue");
    k = med(["udgiv", "--toer"], "19:45:00", { vinduer: "19:30-20:30,06:30-07:30" });
    tjek("vinduer: rækkefølgen i indstillingen er ligegyldig", k.status === 0 && touch(k).includes("202603101930.00"), k.fejl);
    k = med(["udgiv", "--toer"], "07:45:00", { vinduer: "06:30-07:30,07:30-08:30" });
    tjek("vinduer: to vinduer, der støder op til hinanden, overlapper ikke", k.status === 0 && touch(k).includes("202603100730.00"), k.fejl);

    // Med stubbe: filtiderne i bundlen og zip-filen er det åbne vindues start, med dagens (opdigtede) dato.
    for (const [tid, start] of [["19:45:00", "19:30:00"], ["06:45:00", "06:30:00"]]) {
      fs.rmSync(ud, { recursive: true, force: true });
      k = med(["udgiv"], tid);
      tjek(`vinduer (stubbe) ${tid}: udgiv exit 0`, k.status === 0, k.fejl + k.ud);
      const tider = lokalt("find", [app, "-exec", "stat", "-f", "%Sm", "-t", "%Y-%m-%d %H:%M:%S", "{}", "+"]).trim().split("\n");
      tjek(`vinduer (stubbe) ${tid}: alle filtider i bundlen er ${D} ${start}`, tider.length >= 8 && tider.every((t) => t === `${D} ${start}`), tider.join(", "));
      const zip = path.join(ud, `Huginn-Muninn-${VER}.zip`);
      const poster = lokalt("unzip", ["-Z", "-T", zip]).split("\n").filter((l) => l.includes("Huginn and Muninn.app"));
      tjek(`vinduer (stubbe) ${tid}: zip-filen og alle dens poster har ${start}`, lokalt("stat", ["-f", "%Sm", "-t", "%Y-%m-%d %H:%M:%S", zip]).trim() === `${D} ${start}` && poster.length >= 8 && poster.every((l) => l.includes(` 20260310.${start.replaceAll(":", "")} `)), poster.join(" | "));
      tjek(`vinduer (stubbe) ${tid}: kontrollen af filtider nævner vinduets start`, k.ud.includes(`filtider: alle har klokkeslættet ${start}`), k.ud);
      tjek(`vinduer (stubbe) ${tid}: tekstfilens filtid`, lokalt("stat", ["-f", "%Sm", "-t", "%H:%M:%S", path.join(ud, `Huginn-Muninn-${VER}.txt`)]).trim() === start);
    }

    // tjek af en færdig bundle: lavet i morgenvinduet, kontrolleret om aftenen; en filtid uden for et vindues start afvises.
    k = med(["tjek", app], "19:45:00");
    tjek("vinduer: tjek godtager en bundle fra et andet vindue", k.status === 0 && k.ud.includes("filtider: alle har starten på et vindue (06:30, 19:30)"), k.fejl + k.ud);
    execFileSync("touch", ["-t", "202603101200.00", path.join(app, "Contents/Info.plist")]);
    k = med(["tjek", app], "19:45:00");
    tjek("vinduer: tjek afviser en filtid, der ikke er starten på et vindue", k.status !== 0 && k.fejl.includes("1 filtid(er)") && k.fejl.includes("starten på et vindue"), k.fejl);

    // Vinduet lukker, mens der bygges: signeringen stoppes, også når et andet vindue er åbnet i mellemtiden.
    fs.rmSync(ud, { recursive: true, force: true });
    k = med(["udgiv"], "07:25:00", { miljoe: { STUB_BYG_TIL: `${D} 07:31:00` } });
    tjek("vinduer: lukker under bygningen: intet signeret eller indsendt", k.status !== 0 && k.stub.some((l) => l.startsWith("npm run")) && !k.stub.some((l) => l.startsWith("codesign --force") || l.startsWith("xcrun")), k.stub.join(" | "));
    tjek("vinduer: lukker under bygningen: fejlteksten nævner det næste vindue", k.fejl.includes("signering stoppet") && k.fejl.includes("næste vindue åbner kl. 19:30 i dag"), k.fejl);
    k = med(["udgiv"], "07:25:00", { miljoe: { STUB_BYG_TIL: `${D} 19:35:00` } });
    tjek("vinduer: et andet vindue tæller ikke for en kørsel, der startede i det første", k.status !== 0 && !k.stub.some((l) => l.startsWith("codesign --force")) && k.fejl.includes("vinduet 06:30-07:30, som kørslen hører til, er lukket") && k.fejl.includes("vinduet 19:30-20:30 er åbent nu"), k.fejl);

    // status
    k = med(["status"], "06:45:00");
    tjek("vinduer: status viser alle vinduer og det åbne", k.status === 0 && k.ud.includes("vinduer:   06:30-07:30, 19:30-20:30") && k.ud.includes("nu:        vinduet 06:30-07:30 er åbent nu") && k.ud.includes(`profil:    ${PROFIL}`), k.ud + k.fejl);
    k = med(["status"], "12:00:00");
    tjek("vinduer: status viser, hvornår det næste åbner (i dag)", k.ud.includes("nu:        næste vindue åbner kl. 19:30 i dag"), k.ud);
    k = med(["status"], "21:00:00");
    tjek("vinduer: status viser, hvornår det næste åbner (i morgen)", k.ud.includes("nu:        næste vindue åbner kl. 06:30 i morgen"), k.ud);
    tjek("vinduer: status har ingen linje om ét vindue", !/^vindue: /m.test(k.ud), k.ud);
    k = med(["status"], "07:30:00");
    tjek("vinduer: status i minuttet efter et vindue: lukket, det næste åbner 19:30", k.ud.includes("nu:        næste vindue åbner kl. 19:30 i dag"), k.ud);
    k = med(["status"], "20:29:59");
    tjek("vinduer: status i et vindues sidste minut: åbent", k.ud.includes("nu:        vinduet 19:30-20:30 er åbent nu"), k.ud);

    // planlaeg: ét tidspunkt pr. vindue, og plisten er gyldig.
    const plistAf = (x) => /<<PLIST\n(.*?)\nPLIST\n/s.exec(x.ud)?.[1] ?? "";
    const lint = (indhold) => {
      const f = path.join(r.rod, "proeve.plist");
      fs.writeFileSync(f, indhold);
      return spawnSync("plutil", ["-lint", f], { encoding: "utf8" }).status === 0;
    };
    k = med(["planlaeg", "--toer"], "12:00:00");
    const pl = plistAf(k);
    const tidspunkter = [...pl.matchAll(/<dict><key>Hour<\/key><integer>(\d+)<\/integer><key>Minute<\/key><integer>(\d+)<\/integer><\/dict>/g)].map((m) => `${m[1]}:${m[2]}`);
    tjek("vinduer: planlaeg --toer exit 0, og intet skrives eller køres", k.status === 0 && k.stub.length === 0 && !fs.existsSync(path.join(r.hjem, "Library")), k.fejl);
    tjek("vinduer: planlaeg har to tidspunkter, ét pr. vindue", tidspunkter.join(" ") === "6:30 19:30", tidspunkter.join(" "));
    tjek("vinduer: tidspunkterne står i en liste under StartCalendarInterval", /<key>StartCalendarInterval<\/key>\s*<array>\s*<dict>.*<\/dict>\s*<dict>.*<\/dict>\s*<\/array>\s*<key>RunAtLoad<\/key>/s.test(pl), pl);
    tjek("vinduer: plisten med to tidspunkter er gyldig (plutil -lint)", pl.length > 0 && lint(pl));
    tjek("vinduer: planlaeg siger begge tidspunkter", k.ud.includes("planlagt: udgiv kl. 06:30, 19:30 (vinduerne 06:30-07:30, 19:30-20:30)"), k.ud);
    k = koer(r, ["planlaeg", "--toer"], { klokken: "23:30:00" });
    tjek("uden vinduer: plisten har ét tidspunkt uden liste og er gyldig", !plistAf(k).includes("<array>\n") && (plistAf(k).match(/<key>Hour<\/key>/g) ?? []).length === 1 && lint(plistAf(k)), plistAf(k));

    // koer (launchd-jobbet): tidsgrænsen følger det vindue, jobbet startede i.
    fs.rmSync(ud, { recursive: true, force: true });
    k = med(["koer"], "19:45:00");
    tjek("vinduer: koer i aftenvinduet: grænsen er til 20:30", k.status === 0 && k.ud.includes("tidsgrænse: 2700 s (vinduet lukker 20:30)") && k.ud.includes("udgiv lykkedes; jobbet fjernes"), k.ud + k.fejl);
    fs.rmSync(ud, { recursive: true, force: true });
    k = med(["koer"], "07:00:00");
    tjek("vinduer: koer i morgenvinduet: grænsen er til 07:30", k.status === 0 && k.ud.includes("tidsgrænse: 1800 s (vinduet lukker 07:30)"), k.ud + k.fejl);
    fs.rmSync(ud, { recursive: true, force: true });
    k = med(["koer"], "12:00:00");
    tjek("vinduer: koer uden for vinduerne: intet signeret, og jobbet bliver stående til næste vindue", k.status !== 0 && !k.stub.some((l) => l.startsWith("codesign") || l.startsWith("launchctl")) && k.ud.includes("jobbet prøver igen ved næste vindue (kl. 06:30, 19:30)") && k.fejl.includes("udgiv stoppet"), k.ud + k.fejl);
    tjek("vinduer: koer uden for vinduerne: grænsen er den mindste (60 s), ikke tiden til det næste vindue lukker", k.ud.includes("tidsgrænse: 60 s"), k.ud);
    tjek("vinduer: ingen vagt (sleep) bliver hængende efter koer", lokalt("pgrep", ["-f", "sleep 2700"]).trim() === "" && lokalt("pgrep", ["-f", "sleep 1800"]).trim() === "");

    // Ugyldige værdier: klar fejl, intet gæt, og intet kørt.
    for (const [vaerdi, ord] of [["abc", "ikke TT:MM-TT:MM"], ["06:30-07:30,07:00-08:00", "overlapper"], ["06:30-07:30,06:30-07:30", "overlapper"], ["22:00-01:00", "midnat"], ["24:00-25:00", "00-23"],
      ["06:30-06:30", "tomt"], ["06:30-07:30,", "ugyldig"], ["", "ugyldig"], ["06:30-07:30, 19:30-20:30", "ikke TT:MM-TT:MM"], ["6:30-7:30", "ikke TT:MM-TT:MM"], ["06:30-07:30;19:30-20:30", "ikke TT:MM-TT:MM"]]) {
      const x = med(["udgiv", "--toer"], "06:45:00", { vinduer: vaerdi });
      tjek(`vinduer: ugyldig værdi "${vaerdi}" afvises med en klar fejl`, x.status !== 0 && x.linjer.length === 0 && x.fejl.includes("fasttid.vinduer") && x.fejl.includes(ord), x.fejl);
    }
    k = med(["status"], "06:45:00", { vinduer: "06:30-07:30,07:00-08:00" });
    tjek("vinduer: også status og planlaeg afviser overlappende vinduer", k.status !== 0 && med(["planlaeg", "--toer"], "06:45:00", { vinduer: "06:30-07:30,07:00-08:00" }).status !== 0, k.fejl);

    // Indstillingen fjernet igen: scriptet er tilbage ved ét klokkeslæt og ét vindue.
    k = koer(r, ["status"], { lokal: lokalProfil, klokken: "23:30:00" });
    tjek("uden vinduer igen: status viser ét vindue fra fasttid.klokken", k.status === 0 && /^vindue:    23:30-00:00 \((åbent|lukket) nu\)$/m.test(k.ud) && !k.ud.includes("vinduer:"), k.ud);
  }
} finally {
  fs.rmSync(r.rod, { recursive: true, force: true });
}

// ---- 13. repoet indeholder ingen app-specifik adgangskode og intet adgangskodeflag ----
{
  const filer = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "-z"], { cwd: REPO, encoding: "utf8" }).split("\0").filter(Boolean);
  const fund = [];
  let laest = 0;
  for (const f of filer) {
    const sti = path.join(REPO, f);
    if (!fs.existsSync(sti) || !fs.statSync(sti).isFile()) continue;
    const buf = fs.readFileSync(sti);
    if (buf.includes(0)) continue; // binær
    laest++;
    const tekst = buf.toString("utf8");
    const m = tekst.match(new RegExp(`(^|[^A-Za-z0-9-])${APP_KODE.source}([^A-Za-z0-9-]|$)`));
    if (m) fund.push(`${f}: app-specifik adgangskode`);
    if (tekst.includes(KODE_FLAG)) fund.push(`${f}: adgangskodeflag`);
  }
  tjek(`repoet (${laest} tekstfiler): ingen app-specifik adgangskode og intet adgangskodeflag`, laest > 50 && fund.length === 0, fund.join(", "));
}

// ---- 14. tauri.conf.json: identiteten står ikke fast, entitlements-filen findes ----
{
  const conf = JSON.parse(fs.readFileSync(path.join(REPO, "src-tauri/tauri.conf.json"), "utf8"));
  const mac = conf.bundle?.macOS ?? {};
  tjek("tauri.conf.json: ingen signingIdentity", !("signingIdentity" in mac));
  tjek("tauri.conf.json: hardenedRuntime true", mac.hardenedRuntime === true);
  tjek("tauri.conf.json: entitlements peger på filen, scriptet bruger", mac.entitlements === "Huginn-Muninn.entitlements" && fs.existsSync(path.join(REPO, "src-tauri", mac.entitlements)));
  tjek("tauri.conf.json: minimumSystemVersion", typeof mac.minimumSystemVersion === "string" && mac.minimumSystemVersion.length > 0);
  const lint = spawnSync("plutil", ["-lint", path.join(REPO, "src-tauri/Huginn-Muninn.entitlements")], { encoding: "utf8" });
  tjek("Huginn-Muninn.entitlements er en gyldig plist", lint.status === 0, lint.stdout + lint.stderr);
  const gi = fs.readFileSync(path.join(REPO, ".gitignore"), "utf8");
  tjek(".gitignore: tools/signer.lokal", gi.split("\n").includes("/tools/signer.lokal"));
}

const i_alt = ok + fejl.length;
for (const f of fejl) console.log(`FEJL: ${f}`);
console.log(`${i_alt} tjek: ${ok} ok, ${fejl.length} fejl`);
process.exit(fejl.length === 0 ? 0 : 1);
