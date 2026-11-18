// V-VINDUE.1: tools/fast-tid.sh prøvet i et midlertidigt repo uden for dette, med et styret ur.
//
// tools/fast-tid.sh er med vilje ikke i git (.gitignore), så CI har den ikke: mangler filen, springer testen over med en
// tydelig linje og exit 0. Findes den, kopieres den til et midlertidigt git-repo med HOME i en midlertidig mappe, og
// hooks installeres dér (aldrig i dette repo). "origin" er en lokal, tom git-mappe ved siden af; intet går til nettet.
// Uret er en date-stub forrest i PATH, som svarer med et fast, opdigtet tidspunkt (STUB_NU); intet tjek afhænger af det
// ægte ur. Alle navne og adresser er opdigtede.
//
//   node tests/signer/fast-tid.mjs
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
// FAST_TID_SCRIPT: kun til at prøve testen selv mod en ændret kopi (mutationer); standard er repoets script.
const SCRIPT = process.env.FAST_TID_SCRIPT ?? path.join(REPO, "tools/fast-tid.sh");

if (!fs.existsSync(SCRIPT)) {
  console.log("SPRUNGET OVER: tools/fast-tid.sh findes ikke (filen er ikke i git), så fast-tid er ikke prøvet her. 0 tjek.");
  process.exit(0);
}

let ok = 0;
const fejl = [];
function tjek(navn, betingelse, detalje = "") {
  if (betingelse) ok++;
  else fejl.push(`${navn}${detalje ? `: ${detalje}` : ""}`);
}

// Samme date-stub som i signer.mjs: uden STUB_NU det ægte ur; med STUB_NU ("ÅÅÅÅ-MM-DD TT:MM:SS") svarer date, som om
// klokken var dét. Kald med -f går uændret til det ægte date. Altid med -j, så uret aldrig kan blive stillet.
const DATE_STUB = `#!/bin/sh
if [ -z "\${STUB_NU:-}" ]; then exec /bin/date "$@"; fi
for a; do if [ "$a" = -f ]; then exec /bin/date "$@"; fi; done
fmt=""
for a; do
  shift
  case $a in +*) fmt=$a ;; *) set -- "$@" "$a" ;; esac
done
exec /bin/date -j -f '%Y-%m-%d %H:%M:%S' "$@" "$STUB_NU" \${fmt:+"$fmt"}
`;

const V = "06:30-07:30,19:30-20:30";
const D = "2026-03-10";
const D1 = "2026-03-11";

function nytRepo() {
  const rod = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "fast-tid-test-")));
  const r = { rod, repo: path.join(rod, "repo"), hjem: path.join(rod, "hjem"), stub: path.join(rod, "stub"), origin: path.join(rod, "origin.git"), nr: 0 };
  for (const d of [r.repo, r.hjem, r.stub, path.join(r.repo, "tools")]) fs.mkdirSync(d, { recursive: true });
  fs.writeFileSync(path.join(r.stub, "date"), DATE_STUB, { mode: 0o755 });
  fs.copyFileSync(SCRIPT, path.join(r.repo, "tools/fast-tid.sh"));
  fs.writeFileSync(path.join(r.repo, ".gitignore"), "/tools/fast-tid.sh\n");
  git(r, null, "init", "-q", "-b", "main");
  git(r, null, "add", "-A");
  git(r, null, "commit", "-q", "-m", "opdigtet");
  execFileSync("git", ["init", "-q", "--bare", r.origin], { env: miljoe(r, null) });
  git(r, null, "remote", "add", "origin", r.origin);
  git(r, null, "push", "-q", "-u", "origin", "main");
  return r;
}
// git og scriptet med en midlertidig HOME (ingen global opsætning, hooks eller signering fra maskinen) og det styrede ur.
function miljoe(r, nu, ekstra = {}) {
  const env = { PATH: `${r.stub}:/usr/bin:/bin`, HOME: r.hjem, GIT_CONFIG_NOSYSTEM: "1", LANG: "en_US.UTF-8", GIT_AUTHOR_NAME: "Test Person", GIT_AUTHOR_EMAIL: "test@wi24rd.com", GIT_COMMITTER_NAME: "Test Person", GIT_COMMITTER_EMAIL: "test@wi24rd.com", ...ekstra };
  if (nu) env.STUB_NU = nu;
  return env;
}
// Kører git; fejler aldrig med et nedbrud, men giver status, stdout og stderr.
function git(r, nu, ...args) {
  const sidste = args.at(-1);
  const ekstra = sidste && typeof sidste === "object" ? args.pop() : {};
  const res = spawnSync("git", args, { cwd: r.repo, env: miljoe(r, nu, ekstra), encoding: "utf8" });
  return { status: res.status, ud: res.stdout, fejl: res.stderr };
}
function script(r, nu, ...args) {
  const res = spawnSync("sh", ["tools/fast-tid.sh", ...args], { cwd: r.repo, env: miljoe(r, nu), encoding: "utf8" });
  return { status: res.status, ud: res.stdout, fejl: res.stderr };
}
// En ny, tom commit gennem hooken (post-commit), som om klokken var nu.
function commit(r, nu, besked) {
  return git(r, nu, "commit", "-q", "--allow-empty", "-m", besked ?? `opdigtet ${++r.nr}`);
}
// En commit uden om hooken med en bestemt tid (som en commit fra før vinduerne, eller en med rigtigt klokkeslæt).
function commitMedTid(r, tid, besked) {
  return git(r, null, "commit", "-q", "--allow-empty", "-m", besked, { FAST_TID_AKTIV: "1", GIT_AUTHOR_DATE: tid, GIT_COMMITTER_DATE: tid });
}
const tider = (r, rev = "HEAD") => git(r, null, "log", "-1", "--format=%ad|%cd", "--date=format:%Y-%m-%d %H:%M:%S", rev).ud.trim();
const begge = (t) => `${t}|${t}`;
const hoved = (r) => git(r, null, "rev-parse", "HEAD").ud.trim();
const paaOrigin = (r, ref = "refs/heads/main") => spawnSync("git", ["--git-dir", r.origin, "rev-parse", "-q", "--verify", ref], { encoding: "utf8", env: miljoe(r, null) }).stdout.trim();
const upushet = (r) => git(r, null, "rev-list", "--count", "origin/main..HEAD").ud.trim();

// ---- 1. syntaks ----
{
  const s = spawnSync("sh", ["-n", SCRIPT], { encoding: "utf8" });
  tjek("sh -n tools/fast-tid.sh", s.status === 0, s.stderr);
  // Kommentarer og de linjer, der kun skriver en besked (echo/fejl), tæller ikke: beskederne nævner "git push".
  const kilde = fs.readFileSync(SCRIPT, "utf8").split("\n").filter((l) => !/^\s*#/.test(l) && !/\b(echo|fejl) "/.test(l)).join("\n");
  tjek("scriptet henter aldrig fra nettet og pusher aldrig selv", !/\bgit\s+(-C\s+\S+\s+)?(fetch|pull|push|clone|ls-remote)\b/.test(kilde) && !/\b(curl|wget|ssh|nc)\s/.test(kilde));
}

// ---- 2. uden fasttid.vinduer: som før (ét klokkeslæt, intet vindue for push) ----
{
  const r = nytRepo();
  try {
    const i = script(r, null, "install");
    tjek("uden vinduer: install lægger begge hooks i det midlertidige repo", i.status === 0 && i.ud.includes("commits får klokken 23:30:00") && ["post-commit", "pre-push"].every((h) => fs.existsSync(path.join(r.repo, ".git/hooks", h))), i.ud + i.fejl);
    commit(r, `${D} 12:00:00`);
    const [a, c] = tider(r).split("|");
    tjek("uden vinduer: en commit får klokken 23:30:00 (author og committer ens)", a === c && a.endsWith(" 23:30:00"), tider(r));
    let p = git(r, `${D} 03:00:00`, "push", "-q", "origin", "main");
    tjek("uden vinduer: push har intet vindue (kl. 03:00 går igennem)", p.status === 0 && paaOrigin(r) === hoved(r), p.fejl);
    git(r, null, "config", "fasttid.klokken", "12:00:00");
    commit(r, `${D} 08:00:00`);
    tjek("uden vinduer: git config fasttid.klokken følges", tider(r).split("|").every((t) => t.endsWith(" 12:00:00")), tider(r));
    git(r, null, "config", "--unset", "fasttid.klokken");
    p = git(r, `${D} 12:00:00`, "push", "-q", "origin", "main");
    tjek("uden vinduer: en commit med et andet klokkeslæt end det faste stopper push", p.status !== 0 && p.fejl.includes("har rigtigt klokkeslæt") && paaOrigin(r) !== hoved(r), p.fejl);
    commitMedTid(r, `${D} 12:34:56`, "rigtigt klokkeslæt");
    let k = script(r, null, "check");
    tjek("uden vinduer: check mærker begge med RET og nævner intet vindue", k.status === 0 && k.ud.trim().split("\n").length === 2 && k.ud.trim().split("\n").every((l) => l.startsWith("RET  ")) && !/vindue|Push:/.test(k.ud), k.ud);
    const x = script(r, null, "ret");
    tjek("uden vinduer: ret giver hver commit sin egen dag og 23:30:00", x.status === 0 && tider(r) === begge(`${D} 23:30:00`) && tider(r, "HEAD~1").split("|").every((t) => t.endsWith(" 23:30:00")), x.fejl + tider(r));
    k = script(r, null, "check");
    tjek("uden vinduer: check siger ok efter ret", k.ud.trim().split("\n").every((l) => l.startsWith("ok  ")), k.ud);
    p = git(r, `${D} 15:00:00`, "push", "-q", "origin", "main");
    tjek("uden vinduer: push går igennem efter ret", p.status === 0 && paaOrigin(r) === hoved(r), p.fejl);
    p = git(r, `${D} 15:00:00`, "push", "-q", "origin", "main", { FAST_TID_UDEN_VINDUE: "1" });
    tjek("uden vinduer: FAST_TID_UDEN_VINDUE=1 giver ingen advarsel", p.status === 0 && !p.fejl.includes("ADVARSEL"), p.fejl);
  } finally {
    fs.rmSync(r.rod, { recursive: true, force: true });
  }
}

// ---- 3. med to vinduer ----
{
  const r = nytRepo();
  try {
    git(r, null, "config", "fasttid.vinduer", V);
    const i = script(r, null, "install");
    tjek("vinduer: install nævner vinduerne", i.status === 0 && i.ud.includes("06:30-07:30, 19:30-20:30"), i.ud + i.fejl);

    // V2: commit-tid.
    commit(r, `${D} 06:45:00`);
    tjek("V2: commit i det første vindue får vinduets start", tider(r) === begge(`${D} 06:30:00`), tider(r));
    commit(r, `${D} 19:45:00`);
    tjek("V2: commit i det andet vindue får vinduets start", tider(r) === begge(`${D} 19:30:00`), tider(r));
    commit(r, `${D} 12:00:00`);
    tjek("V2: commit mellem to vinduer får det næste vindues start samme dag", tider(r) === begge(`${D} 19:30:00`), tider(r));
    commit(r, `${D} 03:00:00`);
    tjek("V2: commit om natten får morgenvinduets start samme dag", tider(r) === begge(`${D} 06:30:00`), tider(r));
    commit(r, `${D} 21:00:00`);
    tjek("V2: commit efter det sidste vindue får 06:30 med næste dags dato", tider(r) === begge(`${D1} 06:30:00`), tider(r));
    commit(r, "2026-12-31 22:00:00");
    tjek("V2: efter det sidste vindue nytårsaften: 1. januar", tider(r) === begge("2027-01-01 06:30:00"), tider(r));
    for (const [tid, forventet] of [["06:29:59", `${D} 06:30:00`], ["06:30:00", `${D} 06:30:00`], ["07:29:59", `${D} 06:30:00`], ["07:30:00", `${D} 19:30:00`], ["20:29:59", `${D} 19:30:00`], ["20:30:00", `${D1} 06:30:00`]]) {
      commit(r, `${D} ${tid}`);
      tjek(`V2: grænse ${tid} → ${forventet}`, tider(r) === begge(forventet), tider(r));
    }
    tjek("V2: 12 commits er ikke pushet, og ingen har et andet klokkeslæt end et vindues start", upushet(r) === "12" && git(r, null, "log", "--format=%ad %cd", "--date=format:%H:%M:%S", "origin/main..HEAD").ud.trim().split("\n").every((l) => /^(06:30:00|19:30:00) (06:30:00|19:30:00)$/.test(l)));

    // V3: push uden for vinduerne nægtes, og beskeden siger, hvornår det næste åbner.
    const foer = paaOrigin(r);
    let p = git(r, `${D} 12:00:00`, "push", "-q", "origin", "main");
    tjek("V3: push mellem vinduerne nægtes", p.status !== 0 && paaOrigin(r) === foer, p.fejl);
    tjek("V3: beskeden nævner klokken, vinduerne og det næste (i dag)", p.fejl.includes("push stoppet") && p.fejl.includes("klokken er 12:00") && p.fejl.includes("06:30-07:30, 19:30-20:30") && p.fejl.includes("Næste vindue åbner kl. 19:30 i dag"), p.fejl);
    p = git(r, `${D} 22:15:00`, "push", "-q", "origin", "main");
    tjek("V3: push om aftenen nægtes, og det næste åbner i morgen", p.status !== 0 && p.fejl.includes("Næste vindue åbner kl. 06:30 i morgen") && paaOrigin(r) === foer, p.fejl);
    for (const [tid, aaben] of [["06:29:59", false], ["07:30:00", false], ["19:29:59", false], ["20:30:00", false]]) {
      p = git(r, `${D} ${tid}`, "push", "-q", "origin", "main");
      tjek(`V3: grænse ${tid}: push nægtes, fordi vinduet er lukket`, p.status !== 0 && p.fejl.includes("der pushes kun i vinduerne") === !aaben, p.fejl);
    }

    // V3: i et åbent vindue nægtes en commit, der hører til et senere vindue.
    p = git(r, `${D} 06:45:00`, "push", "-q", "origin", "main");
    tjek("V3: push i vinduet nægtes med commits fra et senere vindue", p.status !== 0 && paaOrigin(r) === foer && p.fejl.includes("VENT") && p.fejl.includes("hører til et senere vindue"), p.fejl);
    tjek("V3: kun de senere commits nævnes (19:30 samme dag, 06:30 næste dag og 1. januar), ikke dem fra 06:30", (p.fejl.match(/fast-tid: VENT /g) ?? []).length === 7 && !p.fejl.includes(`${D} 06:30`), p.fejl);

    // V4: check og ret.
    let k = script(r, `${D} 06:45:00`, "check");
    const linjer = k.ud.trim().split("\n");
    tjek("V4: check viser hver commit med sit vindue", k.status === 0 && linjer.length === 13 && linjer.filter((l) => l.startsWith("ok  ") && l.endsWith("[vindue 06:30-07:30]")).length === 5 && linjer.filter((l) => l.startsWith("VENT") && l.endsWith("[vindue 19:30-20:30, ikke åbnet endnu]")).length === 4 && linjer.filter((l) => l.startsWith("VENT") && l.endsWith("[vindue 06:30-07:30, ikke åbnet endnu]")).length === 3, k.ud);
    tjek("V4: check siger, at der ikke kan pushes nu, og hvorfor", linjer.at(-1).startsWith("Push: ikke nu – 7 commit(s) hører til et senere vindue") && linjer.at(-1).includes("vinduet 06:30-07:30 er åbent"), linjer.at(-1));
    k = script(r, `${D} 12:00:00`, "check");
    tjek("V4: check uden for vinduerne siger, hvornår det næste åbner", k.ud.trim().split("\n").at(-1) === "Push: ikke nu – klokken er 12:00; næste vindue (19:30-20:30) åbner kl. 19:30 i dag.", k.ud.trim().split("\n").at(-1));
    let x = script(r, `${D} 06:45:00`, "ret");
    tjek("V4: ret i et åbent vindue sætter alle ikke-pushede commits til vinduets start", x.status === 0 && x.ud.includes(`Rettet til ${D} 06:30:00 (vinduet 06:30-07:30 er åbent)`) && upushet(r) === "12" && git(r, null, "log", "--format=%ad|%cd", "--date=format:%Y-%m-%d %H:%M:%S", "origin/main..HEAD").ud.trim().split("\n").every((l) => l === begge(`${D} 06:30:00`)), x.ud + x.fejl);
    k = script(r, `${D} 06:45:00`, "check");
    tjek("V4: check efter ret: alle ok, og der kan pushes nu", k.ud.trim().split("\n").filter((l) => l.startsWith("ok  ")).length === 12 && k.ud.trim().split("\n").at(-1) === "Push: muligt nu (vinduet 06:30-07:30 er åbent).", k.ud);

    // V3: push tilladt i vinduet.
    p = git(r, `${D} 06:45:00`, "push", "-q", "origin", "main");
    tjek("V3: push i vinduet går igennem", p.status === 0 && paaOrigin(r) === hoved(r) && !p.fejl.includes("ADVARSEL"), p.fejl);
    k = script(r, `${D} 06:50:00`, "check");
    tjek("V4: check uden ikke-pushede commits", k.status === 0 && k.ud.includes("Ingen commits, der ikke er pushet.") && k.ud.includes("Push: muligt nu"), k.ud);
    for (const tid of ["06:30:00", "07:29:59", "19:30:00", "20:29:59"]) {
      commit(r, `${D} ${tid}`);
      p = git(r, `${D} ${tid}`, "push", "-q", "origin", "main");
      tjek(`V3: grænse ${tid}: commit og push i vinduets første eller sidste minut går igennem`, p.status === 0 && paaOrigin(r) === hoved(r), p.fejl);
    }

    // V3: en commit med det gamle 23:30 (fra før vinduerne) godkendes ikke stiltiende; ret flytter den.
    commitMedTid(r, "2026-03-09 23:30:00", "fra før vinduerne");
    p = git(r, `${D} 19:45:00`, "push", "-q", "origin", "main");
    tjek("V3: en ikke-pushet commit med 23:30 stopper push i et åbent vindue", p.status !== 0 && paaOrigin(r) !== hoved(r) && p.fejl.includes("RET") && p.fejl.includes("2026-03-09 23:30") && p.fejl.includes("Kør: sh tools/fast-tid.sh ret"), p.fejl);
    k = script(r, `${D} 19:45:00`, "check");
    tjek("V4: check mærker den med RET og siger nej", k.ud.includes("[intet vindue]") && k.ud.trim().split("\n").at(-1).startsWith("Push: nej – 1 commit(s) har et klokkeslæt, der ikke er starten på et vindue"), k.ud);
    p = git(r, `${D} 12:00:00`, "push", "-q", "origin", "main", { FAST_TID_UDEN_VINDUE: "1" });
    tjek("V3: FAST_TID_UDEN_VINDUE=1 slipper ikke en commit med et forkert klokkeslæt igennem", p.status !== 0 && paaOrigin(r) !== hoved(r) && p.fejl.includes("ADVARSEL") && p.fejl.includes("RET"), p.fejl);
    x = script(r, `${D} 12:00:00`, "ret");
    tjek("V4: ret uden for vinduerne sætter commits til det næste vindues start", x.status === 0 && tider(r) === begge(`${D} 19:30:00`) && x.ud.includes("næste vindue, 19:30-20:30, åbner i dag"), x.ud + x.fejl + tider(r));
    x = script(r, `${D} 21:00:00`, "ret");
    tjek("V4: ret efter det sidste vindue: 06:30 næste dag", x.status === 0 && tider(r) === begge(`${D1} 06:30:00`) && x.ud.includes("åbner i morgen"), x.ud + x.fejl + tider(r));

    // V3: den bevidste undtagelse.
    p = git(r, `${D} 21:05:00`, "push", "-q", "origin", "main", { FAST_TID_UDEN_VINDUE: "0" });
    tjek("V3: kun værdien 1 er en undtagelse", p.status !== 0 && paaOrigin(r) !== hoved(r), p.fejl);
    p = git(r, `${D} 21:05:00`, "push", "-q", "origin", "main", { FAST_TID_UDEN_VINDUE: "1" });
    tjek("V3: FAST_TID_UDEN_VINDUE=1 slipper et push igennem uden for vinduerne", p.status === 0 && paaOrigin(r) === hoved(r), p.fejl);
    tjek("V3: undtagelsen advarer på stderr om, at tidspunktet bliver offentligt", p.fejl.includes("ADVARSEL") && p.fejl.includes("FAST_TID_UDEN_VINDUE=1") && p.fejl.includes("bliver offentligt"), p.fejl);
    commit(r, `${D1} 06:45:00`);
    p = git(r, `${D1} 06:45:00`, "push", "-q", "origin", "main", { FAST_TID_UDEN_VINDUE: "1" });
    tjek("V3: i et åbent vindue gør undtagelsen ingen forskel og advarer ikke", p.status === 0 && !p.fejl.includes("ADVARSEL"), p.fejl);

    // Tags følger samme regel.
    git(r, null, "tag", "v-let");
    p = git(r, `${D1} 12:00:00`, "push", "-q", "origin", "v-let");
    tjek("tags: et tag pushes ikke uden for vinduerne", p.status !== 0 && paaOrigin(r, "refs/tags/v-let") === "" && p.fejl.includes("Næste vindue åbner kl. 19:30 i dag"), p.fejl);
    p = git(r, `${D1} 19:45:00`, "push", "-q", "origin", "v-let");
    tjek("tags: et tag pushes i vinduet", p.status === 0 && paaOrigin(r, "refs/tags/v-let") !== "", p.fejl);
    git(r, null, "tag", "-a", "v-ann", "-m", "opdigtet", { GIT_COMMITTER_DATE: `${D1} 12:34:56` });
    p = git(r, `${D1} 19:45:00`, "push", "-q", "origin", "v-ann");
    tjek("tags: et annoteret tag med rigtigt klokkeslæt stoppes", p.status !== 0 && paaOrigin(r, "refs/tags/v-ann") === "" && p.fejl.includes("tagget v-ann har klokkeslættet 12:34:56") && p.fejl.includes("git tag -a -f"), p.fejl);
    git(r, null, "tag", "-a", "-f", "v-ann", "-m", "opdigtet", { GIT_COMMITTER_DATE: `${D1} 19:30:00` });
    p = git(r, `${D1} 19:45:00`, "push", "-q", "origin", "v-ann");
    tjek("tags: et annoteret tag med vinduets start som tid pushes", p.status === 0 && paaOrigin(r, "refs/tags/v-ann") !== "", p.fejl);
    p = git(r, `${D1} 12:00:00`, "push", "-q", "origin", ":refs/tags/v-let");
    tjek("tags: heller ikke en sletning pushes uden for vinduerne", p.status !== 0 && paaOrigin(r, "refs/tags/v-let") !== "", p.fejl);

    // V1: ugyldige værdier giver en klar fejl og intet push.
    commit(r, `${D1} 19:50:00`);
    for (const [vaerdi, ord] of [["abc", "ikke TT:MM-TT:MM"], ["06:30-07:30,07:00-08:00", "overlapper"], ["22:00-01:00", "midnat"], ["24:00-25:00", "00-23"], ["06:30-06:30", "tomt"], ["06:30-07:30,", "ugyldig"], ["", "ugyldig"]]) {
      git(r, null, "config", "fasttid.vinduer", vaerdi);
      p = git(r, `${D1} 19:50:00`, "push", "-q", "origin", "main");
      const c = script(r, `${D1} 19:50:00`, "check");
      tjek(`V1: ugyldig værdi "${vaerdi}": push og check stopper med en klar fejl`, p.status !== 0 && paaOrigin(r) !== hoved(r) && p.fejl.includes("fasttid.vinduer") && p.fejl.includes(ord) && c.status !== 0 && c.fejl.includes(ord), p.fejl + c.fejl);
    }
    git(r, null, "config", "fasttid.vinduer", "abc");
    const foerRet = hoved(r);
    x = script(r, `${D1} 19:50:00`, "ret");
    tjek("V1: ugyldig værdi: ret ændrer intet", x.status !== 0 && hoved(r) === foerRet && x.fejl.includes("fasttid.vinduer"), x.fejl);

    // V1: indstillingen fjernet igen: som før (commits får 23:30, push har intet vindue, den gamle regel gælder).
    git(r, null, "config", "--unset", "fasttid.vinduer");
    p = git(r, `${D1} 12:00:00`, "push", "-q", "origin", "main");
    tjek("uden indstillingen igen: en commit med 19:30 stoppes af den gamle regel (23:30)", p.status !== 0 && p.fejl.includes("har rigtigt klokkeslæt"), p.fejl);
    x = script(r, null, "ret");
    p = git(r, `${D1} 12:00:00`, "push", "-q", "origin", "main");
    tjek("uden indstillingen igen: ret giver 23:30, og push går igennem uden for ethvert vindue", x.status === 0 && tider(r) === begge(`${D1} 23:30:00`) && p.status === 0 && paaOrigin(r) === hoved(r), x.fejl + p.fejl + tider(r));
  } finally {
    fs.rmSync(r.rod, { recursive: true, force: true });
  }
}

const i_alt = ok + fejl.length;
for (const f of fejl) console.log(`FEJL: ${f}`);
console.log(`fast-tid: ${i_alt} tjek: ${ok} ok, ${fejl.length} fejl`);
process.exit(fejl.length === 0 ? 0 : 1);
