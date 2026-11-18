#!/bin/sh
# signer.sh – Huginn and Muninn signeret med Developer ID, notariseret og hæftet, med fast signeringstid (V-SIGN.1).
#
#   sh tools/signer.sh proeve [--toer]               # debug-bundle, Developer ID, hardened runtime, UDEN tidsstempel
#   sh tools/signer.sh udgiv [--toer] [--uden-vagt] [--urent]  # release: bygning uden navne, signering, notarisering, hæftning
#   sh tools/signer.sh tjek <sti til .app> [--toer]  # kun kontrollerne, ingen signering
#   sh tools/signer.sh planlaeg|afplanlaeg|status [--toer]   # launchd kører udgiv ved hvert vindues start, til det er lykkedes
#   --toer: tørkørsel. Skriver kommandoerne (linjer med "+ "), kører dem ikke, skriver intet til disken.
#
# Identitet: $APPLE_SIGNING_IDENTITY, ellers identitet=… i tools/signer.lokal (git-ignoreret), ellers Developer ID nedenfor.
# Notarisering: en Nøglering-profil, standard ham-notary (profil=… i signer.lokal vælger en anden, fx en profil, der
# allerede findes til et andet program). Ingen adgangskode i filer, miljø eller log.
# Udgivelser og notariseringslog: udgivelser=… i signer.lokal, ellers ~/Huginn-Muninn-udgivelser (aldrig i repoet).
# Vinduer: git config fasttid.vinduer (som tools/fast-tid.sh), en kommasepareret liste TT:MM-TT:MM i lokal tid, fx
# 06:30-07:30,19:30-20:30. udgiv signerer og notariserer kun i et af vinduerne, og filtiderne i pakken er starten på
# det vindue, kørslen startede i. Lukker det undervejs, stopper udgiv; et andet vindue tæller ikke for samme kørsel.
# Ugyldig værdi, overlappende vinduer eller et vindue hen over midnat giver en fejl. Apples tidsstempel kan ikke vælges.
# Er fasttid.vinduer ikke sat, gælder det gamle: ét klokkeslæt, git config fasttid.klokken (standard 23:30:00), og ét
# vindue derfra, git config signer.vindue (minutter, standard 30); de to bruges ikke, når fasttid.vinduer er sat.
# Er Mac'en i dvale ved et vindues start, kører launchd jobbet ved næste opvågning; er vinduet lukket da, prøves der
# igen ved det næste vindue. Kun macOS.
# udgiv kræver et rent arbejdstræ og at HEAD findes på origin (den sporede gren, som git sidst kendte den; intet hentes
# fra nettet), før noget bygges. --urent tilsidesætter det med en advarsel (til prøver); --uden-vagt gør ikke. Version og
# commit står i den sidste linje og i Huginn-Muninn-<version>.txt ved zip-filen. Bundlen hedder "Huginn and Muninn.app"
# (med mellemrum); filerne i udgivelsesmappen har ingen mellemrum.
# Jobbet (koer) har en tidsgrænse: når vinduet lukker, stoppes hele udgiv (bygning, codesign, notarytool), og der
# efterlades ingen halv zip i udgivelsesmappen. Jobbet bliver stående til næste vindue.
set -eu

ROD=$(cd "$(dirname "$0")/.." && pwd -P)
ENT="$ROD/src-tauri/Huginn-Muninn.entitlements"
APPNAVN="Huginn and Muninn"          # bundlens navn (Tauris productName) og første linje i tekstfilen
FILNAVN="Huginn-Muninn"              # filnavne i udgivelsesmappen, uden mellemrum
STANDARD_IDENTITET="Developer ID Application: Jan Tingsvad (5DAVNFA355)"
STANDARD_PROFIL="ham-notary"

fejl() { echo "signer: $*" >&2; exit 1; }
trin() { echo "== $*"; }
lokal() { # $1 = nøgle → sidste værdi i tools/signer.lokal (læses som tekst, køres aldrig)
  f="$ROD/tools/signer.lokal"
  [ -f "$f" ] || return 0
  sed -n "s/^$1=//p" "$f" | tail -n 1
}

TOER=0; UDEN_VAGT=0; URENT=0; URENT_BRUGT=0; MAAL=""
TILSTAND=${1:-}
[ $# -gt 0 ] && shift
for a in "$@"; do
  case $a in
    --toer) TOER=1 ;;
    --uden-vagt) UDEN_VAGT=1 ;;
    --urent) URENT=1 ;;
    -*) fejl "ukendt flag: $a" ;;
    *) [ -z "$MAAL" ] || fejl "for mange argumenter: $a"; MAAL=$a ;;
  esac
done

IDENTITET=${APPLE_SIGNING_IDENTITY:-$(lokal identitet)}
IDENTITET=${IDENTITET:-$STANDARD_IDENTITET}
PROFIL=$(lokal profil); PROFIL=${PROFIL:-$STANDARD_PROFIL}
UD=$(lokal udgivelser); UD=${UD:-$HOME/$FILNAVN-udgivelser}
case "$UD/" in "$ROD"/*) fejl "udgivelser=$UD ligger i repoet; vælg en mappe uden for $ROD" ;; esac

# ---- tørkørsel: én funktion kører eller skriver hver kommando ----
vis() { # skriver argumenterne som en kommandolinje, med anførselstegn hvor det trænger
  printf '+'
  for x in "$@"; do
    case $x in
      ''|*[!A-Za-z0-9_./:=,+@%-]*) printf " '%s'" "$x" ;;
      *) printf ' %s' "$x" ;;
    esac
  done
  printf '\n'
}
koer() { if [ "$TOER" = 1 ]; then vis "$@"; else "$@"; fi; }
saet() { # $1 = navn, $2 = værdi, $3 = visning (valgfri) → export
  export "$1=$2"
  echo "+ export $1=${3:-$2}"
}

# ---- klokkeslæt og vinduer ----
heltal() { t=$1; while [ "${t#0}" != "$t" ]; do t=${t#0}; done; echo "${t:-0}"; }
hhmm() { printf '%02d:%02d' $(( ($1 / 60) % 24 )) $(( $1 % 60 )); }
nu_min() { echo $(( $(heltal "$(date +%H)") * 60 + $(heltal "$(date +%M)") )); }

laes_vinduer() { # $1 = værdien af fasttid.vinduer → VINDUER ("fra til fra til …", minutter siden midnat, sorteret) og VINDUER_TEKST
  raa=$1
  hjaelp="skriv fx: git config fasttid.vinduer 06:30-07:30,19:30-20:30"
  case $raa in ''|,*|*,|*,,*) fejl "fasttid.vinduer er ugyldig (fik '$raa'); $hjaelp" ;; esac
  linjer=""
  gl_ifs=$IFS; IFS=,; set -f; set -- $raa; set +f; IFS=$gl_ifs
  for v in "$@"; do
    case $v in
      [0-2][0-9]:[0-5][0-9]-[0-2][0-9]:[0-5][0-9]) ;;
      *) fejl "fasttid.vinduer: '$v' er ikke TT:MM-TT:MM; $hjaelp" ;;
    esac
    fra=${v%-*}; til=${v#*-}
    f_t=$(heltal "${fra%:*}"); t_t=$(heltal "${til%:*}")
    { [ "$f_t" -le 23 ] && [ "$t_t" -le 23 ]; } || fejl "fasttid.vinduer: timen i '$v' skal være 00-23"
    f=$(( f_t * 60 + $(heltal "${fra#*:}") )); t=$(( t_t * 60 + $(heltal "${til#*:}") ))
    [ "$t" -ne "$f" ] || fejl "fasttid.vinduer: vinduet '$v' er tomt (samme start og slut)"
    [ "$t" -gt "$f" ] || fejl "fasttid.vinduer: vinduet '$v' går hen over midnat; det kan et vindue ikke"
    linjer="$linjer$f $t
"
  done
  set -- $(printf '%s' "$linjer" | sort -n)
  VINDUER="$*"; VINDUER_TEKST=""; STARTER=""; forrige_til=-1; forrige=""
  while [ $# -ge 2 ]; do
    denne="$(hhmm "$1")-$(hhmm "$2")"
    [ "$1" -ge "$forrige_til" ] || fejl "fasttid.vinduer: vinduerne $forrige og $denne overlapper"
    VINDUER_TEKST="${VINDUER_TEKST:+$VINDUER_TEKST, }$denne"
    STARTER="${STARTER:+$STARTER, }$(hhmm "$1")"
    forrige_til=$2; forrige=$denne
    shift 2
  done
}
find_vindue() { # $1 = minutter siden midnat → V_AABEN (1/0), V_FRA, V_TIL (det åbne, ellers det næste), V_IMORGEN (1: åbner først i morgen)
  fv_n=$1
  set -- $VINDUER
  V_AABEN=0; V_IMORGEN=1; V_FRA=$1; V_TIL=$2
  while [ $# -ge 2 ]; do
    if [ "$fv_n" -ge "$1" ] && [ "$fv_n" -lt "$2" ]; then V_AABEN=1; V_IMORGEN=0; V_FRA=$1; V_TIL=$2; return 0; fi
    if [ "$fv_n" -lt "$1" ]; then V_IMORGEN=0; V_FRA=$1; V_TIL=$2; return 0; fi
    shift 2
  done
}
naeste_tekst() { # hvad der gælder lige nu: et åbent vindue, eller hvornår det næste åbner
  find_vindue "$(nu_min)"
  if [ "$V_AABEN" = 1 ]; then echo "vinduet $(hhmm "$V_FRA")-$(hhmm "$V_TIL") er åbent nu"
  elif [ "$V_IMORGEN" = 1 ]; then echo "næste vindue åbner kl. $(hhmm "$V_FRA") i morgen"
  else echo "næste vindue åbner kl. $(hhmm "$V_FRA") i dag"; fi
}

if VINDUER_RAA=$(git -C "$ROD" config --get fasttid.vinduer 2>/dev/null); then
  # Flere vinduer (V-VINDUE.1). Det vindue, der er åbent ved start (ellers det næste), gælder for hele kørslen:
  # dets start er den faste tid til filtider, og dets slut er grænsen for vagten og for jobbet.
  MED_VINDUER=1
  laes_vinduer "$VINDUER_RAA"
  NU_DAG=$(date +%Y%m%d)
  find_vindue "$(nu_min)"
  FRA=$V_FRA; TIL=$V_TIL
  K_T=$(( FRA / 60 )); K_M=$(( FRA % 60 )); K_S=00
  KLOKKEN="$(hhmm "$FRA"):00"
  if [ "$V_IMORGEN" = 1 ]; then DAG=$(date -j -v+1d -f %Y%m%d "$NU_DAG" +%Y%m%d); else DAG=$NU_DAG; fi
else
  MED_VINDUER=0
  KLOKKEN=$(git -C "$ROD" config --get fasttid.klokken 2>/dev/null || echo "23:30:00")
  case $KLOKKEN in
    [0-2][0-9]:[0-5][0-9]) KLOKKEN="$KLOKKEN:00" ;;
    [0-2][0-9]:[0-5][0-9]:[0-5][0-9]) ;;
    *) fejl "fasttid.klokken skal være TT:MM eller TT:MM:SS (fik $KLOKKEN)" ;;
  esac
  K_T=$(heltal "$(echo "$KLOKKEN" | cut -d: -f1)")
  K_M=$(heltal "$(echo "$KLOKKEN" | cut -d: -f2)")
  K_S=$(echo "$KLOKKEN" | cut -d: -f3)
  [ "$K_T" -le 23 ] || fejl "fasttid.klokken: timen skal være 00-23 (fik $KLOKKEN)"
  VINDUE=$(git -C "$ROD" config --get signer.vindue 2>/dev/null || echo 30)
  case $VINDUE in ''|*[!0-9]*) fejl "signer.vindue skal være et antal minutter (fik $VINDUE)" ;; esac
  FRA=$(( K_T * 60 + K_M )); TIL=$(( FRA + VINDUE ))
  # Den faste tid til filtider: dagens dato + klokkeslættet (gårsdagens, hvis vinduet er gået hen over midnat).
  if [ "$(nu_min)" -lt "$FRA" ] && [ "$TIL" -gt 1440 ] && [ "$(nu_min)" -lt $(( TIL - 1440 )) ]; then
    DAG=$(date -v-1d +%Y%m%d)
  else
    DAG=$(date +%Y%m%d)
  fi
fi
TS=$(printf '%s%02d%02d.%s' "$DAG" "$K_T" "$K_M" "$K_S")   # touch -t CCYYMMDDhhmm.SS
TILLADTE=$KLOKKEN; KLOKKE_TEKST="klokkeslættet $KLOKKEN"    # hvad tjek_filtider godtager (tjek med vinduer: alle starter)

i_vinduet() { # 0 hvis nu ligger i [FRA, TIL): kørslens vindue; uden fasttid.vinduer også hen over midnat
  n=$(nu_min)
  { [ "$n" -ge "$FRA" ] && [ "$n" -lt "$TIL" ]; } || { [ "$TIL" -gt 1440 ] && [ "$n" -lt $(( TIL - 1440 )) ]; }
}

vagt() { # $1 = hvad der skal ske nu
  if i_vinduet; then return 0; fi
  if [ "$MED_VINDUER" = 1 ]; then
    if [ "$UDEN_VAGT" = 1 ]; then
      echo "signer: ADVARSEL: $1 uden for vinduerne $VINDUER_TEKST (--uden-vagt); $(naeste_tekst). Apples tidsstempel og notariseringen viser det rigtige klokkeslæt." >&2
      return 0
    fi
    nt=$(naeste_tekst)
    case $nt in
      vinduet*) fejl "$1 stoppet: vinduet $(hhmm "$FRA")-$(hhmm "$TIL"), som kørslen hører til, er lukket, og klokken er $(date +%H:%M). Start udgiv igen ($nt). Bevidst nu: --uden-vagt (røber tidspunktet)" ;;
    esac
    fejl "$1 stoppet: udgiv kører kun i vinduerne $VINDUER_TEKST, og klokken er $(date +%H:%M); $nt. Planlæg: sh tools/signer.sh planlaeg · bevidst nu: --uden-vagt (røber tidspunktet)"
  fi
  if [ "$UDEN_VAGT" = 1 ]; then
    echo "signer: ADVARSEL: $1 uden for vinduet $(hhmm "$FRA")-$(hhmm "$TIL") (--uden-vagt). Apples tidsstempel og notariseringen viser det rigtige klokkeslæt." >&2
    return 0
  fi
  fejl "$1 stoppet: udgiv kører kun i vinduet $(hhmm "$FRA")-$(hhmm "$TIL"), og klokken er $(date +%H:%M). Planlæg: sh tools/signer.sh planlaeg · bevidst nu: --uden-vagt (røber tidspunktet)"
}

# ---- byggeklodser ----
version() { plutil -extract version raw -o - "$ROD/src-tauri/tauri.conf.json"; }
kort() { git -C "$ROD" rev-parse --short HEAD 2>/dev/null || echo ingen; }

ren_kode() { # rent arbejdstræ, og HEAD findes på origin (den sporede gren), før noget bygges. Kalder aldrig fetch eller push.
  problemer=""; spor=""
  if ! git -C "$ROD" rev-parse --verify -q HEAD >/dev/null; then
    problemer="der er ingen commit"
  else
    aendret=$(git -C "$ROD" status --porcelain --untracked-files=normal | wc -l | tr -d ' ')
    [ "$aendret" = 0 ] || problemer="arbejdstræet er ikke rent ($aendret ændrede eller usporede filer, se git status)"
    if spor=$(git -C "$ROD" rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null); then
      case $spor in
        origin/*)
          git -C "$ROD" merge-base --is-ancestor HEAD '@{u}' 2>/dev/null ||
            problemer="${problemer:+$problemer; }HEAD $(kort) er ikke pushet til $spor (som git sidst kendte den; intet hentet)" ;;
        *) problemer="${problemer:+$problemer; }grenen sporer $spor, ikke en gren på origin" ;;
      esac
    else
      problemer="${problemer:+$problemer; }grenen sporer ingen gren på origin"
    fi
  fi
  if [ -z "$problemer" ]; then echo "kode: arbejdstræet er rent, og HEAD $(kort) findes på $spor"; return 0; fi
  if [ "$URENT" = 1 ]; then
    URENT_BRUGT=1
    echo "signer: ADVARSEL: $problemer (--urent). Udgivelsen svarer ikke til en pushet commit; kun til prøver." >&2
    return 0
  fi
  fejl "udgiv stoppet før bygning: $problemer. Commit og push først (med fast tid), eller --urent til en prøve"
}

info() { # $1 = sti → version og commit ved siden af zip-filen, uden klokkeslæt; filtiden er den faste tid
  if [ "$TOER" = 1 ]; then
    echo "+ (fil) $1: $APPNAVN $v, commit $h$([ "$URENT_BRUGT" = 0 ] || echo ", urent arbejdstræ")"
    return 0
  fi
  { echo "$APPNAVN $v"; echo "commit $h"; [ "$URENT_BRUGT" = 0 ] || echo "arbejdstræ: urent (--urent), svarer ikke til commit $h"; } > "$1"
  filtider "$1"
}

filtider() { # $1 = sti → alle filtider (også mapper og links) sættes til den faste tid
  koer find "$1" -exec touch -h -t "$TS" '{}' +
}

tjek_filtider() { # $1 = sti → fejler, hvis en filtid ikke har det faste klokkeslæt
  if [ "$TOER" = 1 ]; then echo "+ (kontrol) alle filtider i $1 har $KLOKKE_TEKST"; return 0; fi
  forkerte=$(find "$1" -exec stat -f '%Sm' -t '%H:%M:%S' '{}' + | grep -v -c -x -F -e "$TILLADTE" || true)
  [ "$forkerte" = 0 ] || fejl "$forkerte filtid(er) i $1 har ikke $KLOKKE_TEKST"
  echo "filtider: alle har $KLOKKE_TEKST"
}

tjek_navne() { # $1 = sti → fejler, hvis brugernavn, hjemmemappe eller maskinnavn står i en fil (værdierne skrives ikke)
  if [ "$TOER" = 1 ]; then echo "+ (kontrol) intet brugernavn, ingen hjemmemappe og intet maskinnavn i $1"; return 0; fi
  fundet=0
  for par in "brugernavnet=$(id -un)" "hjemmemappen=$HOME" "maskinnavnet=$(hostname -s 2>/dev/null || true)" \
             "computernavnet=$(scutil --get ComputerName 2>/dev/null || true)" \
             "værtsnavnet=$(scutil --get LocalHostName 2>/dev/null || true)"; do
    navn=${par%%=*}; vaerdi=${par#*=}
    [ -n "$vaerdi" ] || continue
    # Under 4 tegn (fx et generisk "Mac") rammer det dele af andre ord ("macOS") og siger intet om maskinen.
    if [ "${#vaerdi}" -lt 4 ]; then echo "navne: $navn har under 4 tegn og måles ikke"; continue; fi
    filer=$(LC_ALL=C grep -r -a -l -F -- "$vaerdi" "$1" || true)
    if [ -n "$filer" ]; then
      fundet=1
      echo "$filer" | while read -r f; do echo "signer: $navn står i ${f#"$1"/}" >&2; done
    fi
  done
  [ "$fundet" = 0 ] || fejl "navne i bygningen (se ovenfor); intet er signeret"
  echo "navne: intet brugernavn, ingen hjemmemappe og intet maskinnavn i bundlen"
}

kontrol() { # $1 = sti til .app
  koer codesign --verify --deep --strict --verbose=2 "$1"
  koer spctl -a -vv "$1"
  koer xcrun stapler validate "$1"
}

zip_af() { # $1 = .app, $2 = zip-fil → ditto uden udvidede attributter, og zip-filens egen tid sættes
  filtider "$1"
  koer ditto -c -k --norsrc --noextattr --noqtn --noacl --keepParent "$1" "$2"
  filtider "$2"
}

notariser() { # $1 = zip-fil
  if [ "$TOER" = 1 ]; then
    vis xcrun notarytool submit "$1" --keychain-profile "$PROFIL" --wait --output-format json
    echo "+ (status læses fra svaret; ved andet end Accepted hentes loggen, og scriptet stopper med exit 1)"
    vis xcrun notarytool log '<id>' --keychain-profile "$PROFIL" "$UD/notarisering-<id>.json"
    return 0
  fi
  svar="$UD/notarisering-svar.json"
  rc=0
  echo "notarisering: indsender $(basename "$1") (en indsendelse kan ikke trækkes tilbage)"
  xcrun notarytool submit "$1" --keychain-profile "$PROFIL" --wait --output-format json > "$svar" || rc=$?
  status=$(plutil -extract status raw -o - "$svar" 2>/dev/null || echo ukendt)
  id=$(plutil -extract id raw -o - "$svar" 2>/dev/null || true)
  echo "notarisering: status=$status id=${id:-ingen} (notarytool exit $rc)"
  if [ -n "$id" ]; then
    log="$UD/notarisering-$id.json"
    if xcrun notarytool log "$id" --keychain-profile "$PROFIL" "$log"; then echo "notariseringslog: $log"; fi
  fi
  [ "$status" = Accepted ] || fejl "notariseringen er ikke godkendt (status $status). Loggen ligger i $UD. Intet er hæftet."
}

byg_release() {
  us=$(printf '\037')
  cargo_hjem=${CARGO_HOME:-$HOME/.cargo}; rustup_hjem=${RUSTUP_HOME:-$HOME/.rustup}
  # Sidste matchende omskrivning vinder (rustc-bogen, "Remap source paths"), så den mest specifikke står sidst.
  rf="--remap-path-prefix=$HOME=/home${us}--remap-path-prefix=$cargo_hjem=/cargo${us}--remap-path-prefix=$rustup_hjem=/rustup${us}--remap-path-prefix=$ROD=/huginn-muninn"
  saet APPLE_SIGNING_IDENTITY "$IDENTITET"
  saet SOURCE_DATE_EPOCH "$(date -j -f %Y%m%d%H%M.%S "$TS" +%s)"
  saet CARGO_ENCODED_RUSTFLAGS "$rf" "$(printf '%s' "$rf" | tr "$us" ' ')"
  koer npm run tauri build -- --no-sign
}

# ---- tilstande ----
proeve() {
  cd "$ROD"
  app="$ROD/src-tauri/target/debug/bundle/macos/$APPNAVN.app"
  trin "prøvebygning: $IDENTITET, hardened runtime, uden tidsstempel, uden notarisering"
  saet APPLE_SIGNING_IDENTITY "$IDENTITET"
  koer npm run tauri build -- --debug --no-sign
  [ "$TOER" = 1 ] || [ -d "$app" ] || fejl "bundlen findes ikke: $app"
  koer xattr -cr "$app"
  koer codesign --force --sign "$IDENTITET" --options runtime --entitlements "$ENT" --timestamp=none "$app"
  koer codesign --verify --strict --verbose=2 "$app"
  koer codesign -dv "$app"
  koer codesign -d -r- "$app"
}

udgiv() {
  vagt "udgiv"
  ren_kode
  [ "$TOER" = 1 ] || mkdir -p "$UD"
  cd "$ROD"
  v=$(version)
  h=$(kort)
  app="$ROD/src-tauri/target/release/bundle/macos/$APPNAVN.app"
  trin "udgivelse $v (commit $h): fast tid $TS, identitet $IDENTITET, profil $PROFIL, ud: $UD"
  byg_release
  [ "$TOER" = 1 ] || [ -d "$app" ] || fejl "bundlen findes ikke: $app"
  tjek_navne "$app"
  koer xattr -cr "$app"
  filtider "$app"
  vagt "signering"
  koer codesign --force --sign "$IDENTITET" --options runtime --entitlements "$ENT" --timestamp "$app"
  koer codesign --verify --strict --verbose=2 "$app"
  indsend="$UD/$FILNAVN-$v-til-notarisering.zip"
  zip_af "$app" "$indsend"
  vagt "notarisering"
  notariser "$indsend"
  koer xcrun stapler staple "$app"
  filtider "$app"
  trin "kontrol"
  tjek_filtider "$app"
  kontrol "$app"
  # Den endelige zip skrives under et midlertidigt navn og flyttes til sidst: en afbrudt kørsel efterlader aldrig en halv
  # Huginn-Muninn-<version>.zip (og rører ikke en tidligere, hel en).
  slut="$UD/$FILNAVN-$v.zip"
  zip_af "$app" "$slut.ufaerdig"
  koer mv -f "$slut.ufaerdig" "$slut"
  koer rm -f "$indsend"
  info "$UD/$FILNAVN-$v.txt"
  trin "færdig: $slut ($APPNAVN $v, commit $h$([ "$URENT_BRUGT" = 0 ] || echo ", urent"))"
}

tjek() {
  [ -n "$MAAL" ] || fejl "brug: sh tools/signer.sh tjek <sti til .app> [--toer]"
  [ "$TOER" = 1 ] || [ -d "$MAAL" ] || fejl "findes ikke: $MAAL"
  koer codesign -dvv "$MAAL"
  if [ "$MED_VINDUER" = 1 ]; then # en færdig pakke kan være lavet i et hvilket som helst af vinduerne
    TILLADTE=$(echo "$STARTER" | sed 's/, /:00\
/g; s/$/:00/')
    KLOKKE_TEKST="starten på et vindue ($STARTER)"
  fi
  tjek_navne "$MAAL"
  tjek_filtider "$MAAL"
  kontrol "$MAAL"
}

etiket() { echo "com.wi24rd.signer.$(basename "$ROD" | tr 'A-Z' 'a-z' | tr -c 'a-z0-9\n' '-')"; }
plist() { echo "$HOME/Library/LaunchAgents/$(etiket).plist"; }
logfil() { echo "$HOME/Library/Logs/$(etiket).log"; }
plist_tider() { # StartCalendarInterval: ét tidspunkt uden fasttid.vinduer, ellers ét pr. vindue (dets start)
  if [ "$MED_VINDUER" = 0 ]; then
    echo "  <dict><key>Hour</key><integer>$K_T</integer><key>Minute</key><integer>$K_M</integer></dict>"
    return 0
  fi
  echo "  <array>"
  set -- $VINDUER
  while [ $# -ge 2 ]; do
    echo "    <dict><key>Hour</key><integer>$(( $1 / 60 ))</integer><key>Minute</key><integer>$(( $1 % 60 ))</integer></dict>"
    shift 2
  done
  echo "  </array>"
}
plist_indhold() {
  cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$(etiket)</string>
  <key>ProgramArguments</key>
  <array><string>/bin/sh</string><string>$ROD/tools/signer.sh</string><string>koer</string></array>
  <key>WorkingDirectory</key><string>$ROD</string>
  <key>EnvironmentVariables</key>
  <dict><key>PATH</key><string>$PATH</string></dict>
  <key>StartCalendarInterval</key>
$(plist_tider)
  <key>RunAtLoad</key><false/>
  <key>StandardOutPath</key><string>$(logfil)</string>
  <key>StandardErrorPath</key><string>$(logfil)</string>
</dict>
</plist>
PLIST
}

planlaeg() {
  [ "$(uname)" = "Darwin" ] || fejl "kun macOS (launchd)"
  p=$(plist)
  if [ "$TOER" = 1 ]; then
    echo "+ cat > '$p' <<PLIST"; plist_indhold; echo "PLIST"
  else
    mkdir -p "$HOME/Library/LaunchAgents" "$HOME/Library/Logs"
    plist_indhold > "$p"
  fi
  if [ "$TOER" = 1 ]; then vis launchctl bootout "gui/$(id -u)/$(etiket)"; else launchctl bootout "gui/$(id -u)/$(etiket)" 2>/dev/null || true; fi
  koer launchctl bootstrap "gui/$(id -u)" "$p"
  if [ "$MED_VINDUER" = 1 ]; then
    echo "planlagt: udgiv kl. $STARTER (vinduerne $VINDUER_TEKST), indtil det lykkes. Log: $(logfil)"
  else
    echo "planlagt: udgiv kl. $(hhmm "$FRA") (vindue $VINDUE min), indtil det lykkes. Log: $(logfil)"
  fi
}

afplanlaeg() {
  if [ "$TOER" = 1 ]; then vis launchctl bootout "gui/$(id -u)/$(etiket)"; else launchctl bootout "gui/$(id -u)/$(etiket)" 2>/dev/null || true; fi
  koer rm -f "$(plist)"
  echo "afplanlagt: $(etiket)"
}

status() {
  if [ -f "$(plist)" ]; then p=planlagt; else p="ikke planlagt"; fi
  if launchctl print "gui/$(id -u)/$(etiket)" >/dev/null 2>&1; then l=indlæst; else l="ikke indlæst"; fi
  echo "job:       $(etiket) ($p, $l)"
  if [ "$MED_VINDUER" = 1 ]; then
    echo "vinduer:   $VINDUER_TEKST"
    echo "nu:        $(naeste_tekst)"
  else
    echo "vindue:    $(hhmm "$FRA")-$(hhmm "$TIL") ($( i_vinduet && echo åbent nu || echo lukket nu ))"
  fi
  echo "identitet: $IDENTITET"
  echo "profil:    $PROFIL"
  echo "ud:        $UD"
  if [ -f "$(logfil)" ]; then echo "log:       $(logfil), sidste linjer:"; tail -n 5 "$(logfil)"; fi
}

graense_sek() { # sekunder, til vinduet lukker (mindst 60, så vagtens besked kan nå loggen); SIGNER_GRAENSE: kun til prøver
  if [ -n "${SIGNER_GRAENSE:-}" ]; then
    case $SIGNER_GRAENSE in ''|*[!0-9]*) fejl "SIGNER_GRAENSE skal være et antal sekunder (fik $SIGNER_GRAENSE)" ;; esac
    echo "$SIGNER_GRAENSE"; return 0
  fi
  # Med fasttid.vinduer er FRA-TIL det vindue, jobbet startede i. Startede det uden for et vindue (Mac'en sov), stopper
  # udgiv selv ved vagten; grænsen er da den mindste.
  if [ "$MED_VINDUER" = 1 ] && ! i_vinduet; then echo 60; return 0; fi
  n=$(( $(heltal "$(date +%H)") * 3600 + $(heltal "$(date +%M)") * 60 + $(heltal "$(date +%S)") ))
  if [ "$TIL" -gt 1440 ] && [ "$n" -lt $(( (TIL - 1440) * 60 )) ]; then n=$(( n + 86400 )); fi
  r=$(( TIL * 60 - n ))
  [ "$r" -ge 60 ] || r=60
  echo "$r"
}

naar_igen() { if [ "$MED_VINDUER" = 1 ]; then echo "i næste vindue"; else echo "næste aften"; fi; }
kl_igen() { # hvornår launchd starter jobbet igen
  if [ "$MED_VINDUER" = 1 ]; then echo "ved næste vindue (kl. $STARTER)"; else echo "kl. $(hhmm "$FRA")"; fi
}

koer_job() { # kaldes af launchd: udgiv i en ny proces (så set -e gælder fuldt) med tidsgrænse, og afplanlæg efter succes
  echo "== launchd $(date '+%Y-%m-%d %H:%M')"
  g=$(graense_sek)
  v=$(version)
  maerke=$(mktemp "${TMPDIR:-/tmp}/signer-graense.XXXXXX")
  rm -f "$maerke"
  # Uden coreutils' timeout (findes ikke på macOS): med set -m får hvert baggrundsjob sin egen procesgruppe, så udgiv og
  # alt, det starter (npm, cargo, codesign, notarytool), kan stoppes samlet med kill mod gruppen. Vagten er et andet job.
  set -m
  sh "$ROD/tools/signer.sh" udgiv &
  job=$!
  ( sleep "$g"; : > "$maerke"; kill -s TERM -- "-$job" 2>/dev/null ) &
  ur=$!
  set +m
  echo "tidsgrænse: $g s (vinduet lukker $(hhmm "$TIL"))"
  rc=0
  wait "$job" || rc=$?
  kill -s TERM -- "-$ur" 2>/dev/null || true
  if [ -f "$maerke" ]; then
    rm -f "$maerke"
    sleep 2
    kill -s KILL -- "-$job" 2>/dev/null || true   # det, der ikke stoppede ved TERM
    halv=""
    for f in "$UD/$FILNAVN-$v-til-notarisering.zip" "$UD/$FILNAVN-$v.zip.ufaerdig" "$UD/notarisering-svar.json"; do
      if [ -e "$f" ]; then halv="$halv $(basename "$f")"; rm -f "$f"; fi
    done
    echo "tidsgrænse: udgiv afbrudt efter $g s, fordi vinduet lukkede (exit $rc). Fjernet:${halv:- intet}."
    case $halv in
      *til-notarisering*) echo "tidsgrænse: zip-filen til notarisering var lavet. Var den indsendt, gør Apple den færdig, men den bruges ikke; $(naar_igen) bygges, signeres og indsendes en ny." ;;
    esac
    echo "udgiv lykkedes ikke; jobbet prøver igen $(kl_igen)"
    exit 1
  fi
  if [ "$rc" = 0 ]; then
    echo "udgiv lykkedes; jobbet fjernes"
    rm -f "$(plist)"
    launchctl bootout "gui/$(id -u)/$(etiket)" 2>/dev/null || true
  else
    echo "udgiv lykkedes ikke; jobbet prøver igen $(kl_igen)"
    exit 1
  fi
}

case $TILSTAND in
  proeve) proeve ;;
  udgiv) udgiv ;;
  tjek) tjek ;;
  planlaeg) planlaeg ;;
  afplanlaeg) afplanlaeg ;;
  status) status ;;
  koer) koer_job ;;
  *) awk 'NR > 1 && /^#/ { print; next } NR > 1 { exit }' "$0"; exit 1 ;;
esac
