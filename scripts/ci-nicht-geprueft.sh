#!/usr/bin/env bash
# Fasst aus dem Log von `cargo nextest run` UND `cargo test --doc` zusammen, was der Lauf NICHT geprueft hat, und macht es sichtbar:
# eine Zeile auf stdout, ein ::warning (erscheint am Lauf, nicht nur im Log) und eine Tabelle in $GITHUB_STEP_SUMMARY.
# Aufruf: scripts/ci-nicht-geprueft.sh <cargo-test.log>   Exit 2: Log fehlt, ist leer, hat weder 'Summary'- noch 'test result:'-Zeile,
#   oder nennt weniger uebersprungene Tests, als die Quelltexte allein als ERiC-Tests ausweisen.
# Das Log darf beides enthalten: die nextest-Form (Zeile 'Summary [..] N tests run: .., M skipped') und die libtest-Form
# (Zeilen 'test result: ..'; so liefern `cargo test --doc` und ein altes `cargo test`-Log). Gezaehlt wird (Muster = Wortlaut im
# Quelltext, ein Test haelt beides fest, siehe ci_nicht_geprueft.rs):
#   Schema      Zeilen 'XSD NICHT geprueft'              elster::testhilfe::schemas_da (rust/elster/src/testhilfe.rs)
#               Zeilen 'UEBERSPRUNGEN: E10-2025.xsd'     eigene Wachen (api/tests/oberflaeche/fluss_g.rs, einreichen_attrappe_hermetisch.rs)
#   Browser     Zeilen 'UEBERSPRUNGEN: kein Browser'     rust/api/tests/oberflaeche/browser.rs
#   Verlauf     Zeilen 'UEBERSPRUNGEN: Verlauf fehlt'        rust/bindung/tests/belege_aufloesbar.rs (flacher Klon mit TAXGRAPH_OHNE_VERLAUF=1)
#   ignoriert   Summe aus: Zahl vor 'skipped' in den nextest-'Summary'-Zeilen (nextest zaehlt #[ignore]-Tests als uebersprungen)
#               plus Zahl vor 'ignored;' in den 'test result:'-Zeilen (Doctests, oder ein altes cargo-test-Log)
#               davon ERiC/checkESt: libtest-Form: Zeilen 'ignored, braucht die ERiC-Bibliothek' im Log;
#               nextest-Form: NENNT KEINEN GRUND, darum zaehlt das Skript die Attribute '#[ignore = "braucht die ERiC-Bibliothek'
#               in den `.rs`-Dateien unter rust/ ohne Ordner `target` (Wurzel = Ordner ueber scripts/, ueberschreibbar mit
#               CI_NICHT_GEPRUEFT_QUELLEN). Grenze: ein ERiC-Test, der per cfg nicht gebaut wird, zaehlte hier mit, ohne im
#               Lauf zu fehlen; und ein Log, das nicht zu diesem Quelltext gehoert, ergibt eine falsche Aufteilung (nicht die Summe).
# Die Zeilen muessen am libtest-Capture vorbei auf stderr stehen (writeln!(std::io::stderr(), ..)); eprintln! sieht die CI nie.
# nextest zeigt die Ausgabe BESTANDENER Tests nur mit success-output = "final" (rust/.config/nextest.toml, Profil ci), und nur
# einmal: ein Profil mit "immediate-final" zaehlte jede Zeile doppelt. Ein Doctest zeigt stderr nie, auch nicht bestanden.
set -euo pipefail
log="${1:?Aufruf: scripts/ci-nicht-geprueft.sh <cargo-test.log>}"
wurzel="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
quellen="${CI_NICHT_GEPRUEFT_QUELLEN:-$wurzel/rust}"
if [ ! -s "$log" ]; then
  echo "ci-nicht-geprueft: Log fehlt oder ist leer: $log" >&2
  exit 2
fi
nextest=$(grep -c '^ *Summary \[' "$log" || true)
libtest=$(grep -c '^test result:' "$log" || true)
if [ "$((nextest + libtest))" -eq 0 ]; then
  echo "ci-nicht-geprueft: weder 'Summary'- noch 'test result:'-Zeile im Log; null gelaufene Tests sind kein Gruen" >&2
  exit 2
fi
schema_hilfe=$(grep -c 'XSD NICHT geprueft' "$log" || true)
schema_eigen=$(grep -c 'UEBERSPRUNGEN: .*E10-2025.xsd' "$log" || true)
schema=$((schema_hilfe + schema_eigen))
browser=$(grep -c 'UEBERSPRUNGEN: kein Browser' "$log" || true)
verlauf=$(grep -c 'UEBERSPRUNGEN: Verlauf fehlt' "$log" || true)
# Eine Zeile je Quelle: 'N skipped' am Ende der nextest-Zusammenfassung, 'N ignored;' in der libtest-Zeile.
uebersprungen=$(awk '/^ *Summary \[/ { for (i = 2; i <= NF; i++) if ($i == "skipped" || $i == "skipped,") s += $(i - 1) } END { print s + 0 }' "$log")
ignoriert_libtest=$(awk '/^test result:/ { for (i = 2; i <= NF; i++) if ($i == "ignored;") s += $(i - 1) } END { print s + 0 }' "$log")
ignoriert=$((uebersprungen + ignoriert_libtest))
if [ "$nextest" -gt 0 ]; then
  eric=$(grep -rh --include='*.rs' --exclude-dir=target '#\[ignore = "braucht die ERiC-Bibliothek' "$quellen" | wc -l)
else
  eric=$(grep -c 'ignored, braucht die ERiC-Bibliothek' "$log" || true)
fi
if [ "$eric" -gt "$ignoriert" ]; then
  echo "ci-nicht-geprueft: ${eric} ERiC-Tests in den Quelltexten, aber nur ${ignoriert} uebersprungene im Log; Log und Quelltext gehoeren zu verschiedenen Staenden" >&2
  exit 2
fi
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
    echo "| Browser-Tests uebersprungen | ${browser} | lokal \`cargo nextest run -p api -E 'binary(oberflaeche)'\` |"
    echo "| Tests gegen ERiC und checkESt (\`#[ignore]\`) | ${eric} | lokal \`make blockmatrix-rust\`, \`make checkest-rust\`, \`make abgabeweg-freigabe-rust\` |"
    echo "| weitere \`#[ignore]\`-Tests | ${sonst} | je Test: \`cargo test -- --ignored\` |"
    echo "| Belege der Testkarte gegen den Git-Verlauf (Hash, Verlaufsform) uebersprungen | ${verlauf} | lokal im vollen Klon, ohne \`TAXGRAPH_OHNE_VERLAUF\` |"
  } >> "$GITHUB_STEP_SUMMARY"
fi
