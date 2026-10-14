#!/usr/bin/env bash
# Fasst aus dem Log von `cargo test` zusammen, was der Lauf NICHT geprueft hat, und macht es sichtbar:
# eine Zeile auf stdout, ein ::warning (erscheint am Lauf, nicht nur im Log) und eine Tabelle in $GITHUB_STEP_SUMMARY.
# Aufruf: scripts/ci-nicht-geprueft.sh <cargo-test.log>      Exit 2: Log fehlt, ist leer oder hat keine 'test result:'-Zeile.
# Gezaehlt werden (Muster = Wortlaut im Quelltext, ein Test haelt beides fest, siehe ci_nicht_geprueft.rs):
#   Schema      Zeilen 'XSD NICHT geprueft'              elster::testhilfe::schemas_da (rust/elster/src/testhilfe.rs)
#               Zeilen 'UEBERSPRUNGEN: E10-2025.xsd'     eigene Wachen (api/tests/oberflaeche/fluss_g.rs, einreichen_attrappe_hermetisch.rs)
#   Browser     Zeilen 'UEBERSPRUNGEN: kein Browser'     rust/api/tests/oberflaeche/browser.rs
#   Verlauf     Zeilen 'UEBERSPRUNGEN: Verlauf fehlt'        rust/bindung/tests/belege_aufloesbar.rs (flacher Klon mit TAXGRAPH_OHNE_VERLAUF=1)
#   ignoriert   Summe der Zahl vor 'ignored' in den 'test result:'-Zeilen; davon ERiC/checkESt: Zeilen 'ignored, braucht die ERiC-Bibliothek'
# Die Zeilen muessen am libtest-Capture vorbei auf stderr stehen (writeln!(std::io::stderr(), ..)); eprintln! sieht die CI nie.
set -euo pipefail
log="${1:?Aufruf: scripts/ci-nicht-geprueft.sh <cargo-test.log>}"
if [ ! -s "$log" ]; then
  echo "ci-nicht-geprueft: Log fehlt oder ist leer: $log" >&2
  exit 2
fi
if [ "$(grep -c '^test result:' "$log" || true)" -eq 0 ]; then
  echo "ci-nicht-geprueft: keine 'test result:'-Zeile im Log; null gelaufene Tests sind kein Gruen" >&2
  exit 2
fi
schema_hilfe=$(grep -c 'XSD NICHT geprueft' "$log" || true)
schema_eigen=$(grep -c 'UEBERSPRUNGEN: .*E10-2025.xsd' "$log" || true)
schema=$((schema_hilfe + schema_eigen))
browser=$(grep -c 'UEBERSPRUNGEN: kein Browser' "$log" || true)
verlauf=$(grep -c 'UEBERSPRUNGEN: Verlauf fehlt' "$log" || true)
ignoriert=$(awk '/^test result:/ { for (i = 1; i <= NF; i++) if ($i == "ignored;") s += $(i - 1) } END { print s + 0 }' "$log")
eric=$(grep -c 'ignored, braucht die ERiC-Bibliothek' "$log" || true)
sonst=$((ignoriert - eric))
zeile="NICHT GEPRUEFT: Schema-Pruefung ${schema}x uebersprungen, Browser-Tests ${browser}x uebersprungen, ERiC/checkESt ${eric}x ignoriert, sonst ${sonst}x ignoriert, Verlauf-Pruefung ${verlauf}x uebersprungen"
echo "$zeile"
echo "::warning title=Nicht geprueft::${zeile}"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "### Was dieser gruene Haken nicht belegt"
    echo
    echo "| Was | Anzahl | Wo es geprueft wird |"
    echo "|---|---|---|"
    echo "| XML gegen das amtliche Schema (XSD) uebersprungen | ${schema} | lokal mit ERiC-Schema (\`ERIC_DIR\`) |"
    echo "| Browser-Tests uebersprungen | ${browser} | lokal \`cargo test -p api --test oberflaeche\` |"
    echo "| Tests gegen ERiC und checkESt (\`#[ignore]\`) | ${eric} | lokal \`make blockmatrix-rust\`, \`make checkest-rust\`, \`make abgabeweg-freigabe-rust\` |"
    echo "| weitere \`#[ignore]\`-Tests | ${sonst} | je Test: \`cargo test -- --ignored\` |"
    echo "| Belege der Testkarte gegen den Git-Verlauf (Hash, Verlaufsform) uebersprungen | ${verlauf} | lokal im vollen Klon, ohne \`TAXGRAPH_OHNE_VERLAUF\` |"
  } >> "$GITHUB_STEP_SUMMARY"
fi
