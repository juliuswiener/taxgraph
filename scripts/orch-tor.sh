#!/bin/bash
# Tor, das orch nach jedem `handoff done` ruft (`.orch-gate.json`). Setzt `make rust-test` unter die gemeinsame lokale
# Schwerlast-Sperre (`~/.cache/taxgraph-tmp/lokal-schwer.lock`, dieselbe wie bei den Forks), mit nice/ionice und
# hoechstens 4 Jobs. Zwei Tore verschiedener Forks laufen so nacheinander, nicht gleichzeitig.
# Die Warteschleife schreibt jede Minute eine Zeile: orch meldet sonst "Tor still" und die Schlange sieht nach Haenger aus.
# Aufruf mit Argumenten (`bash scripts/orch-tor.sh echo ok`) ersetzt `make rust-test`; so prueft man das Skript.
# ponytail: lokal und eine Sperre fuer alles Schwere. Wenn das Tor auf arch-think laufen soll, braucht es Baum-Versand und
# Baum-Hash auf beiden Seiten (git archive + ssh + arch-lock); dann diese Datei ersetzen.
cd "$(dirname "$0")/.." || exit 1
exec 9>"${ORCH_TOR_SPERRE:-$HOME/.cache/taxgraph-tmp/lokal-schwer.lock}" || exit 1
until flock -w 60 9; do echo "[orch-tor] wartet auf lokal-schwer.lock ($(date +%T))"; done
echo "[orch-tor] Sperre genommen ($(date +%T)), nice 15, ionice idle, 4 Jobs"
export CARGO_BUILD_JOBS=4 NEXTEST_TEST_THREADS=4
[ $# -gt 0 ] || set -- make rust-test
exec nice -n 15 ionice -c3 "$@"
