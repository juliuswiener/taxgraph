#!/usr/bin/env bash
# Startet den Rust-Dienst `taxgraph-api` in einem EIGENEN Datenverzeichnis.
#
# Warum dieses Skript im Repo liegt (und nicht in /tmp): bis heute startete `taxgraph-api`
# nur ein Parity-Test (`rust/parity/tests/api_http_paritaet.rs`). Es gab kein Ziel, mit dem
# ein Mensch den Dienst hochzieht und eine echte Anfrage dagegen stellt. Ein Startvorgang,
# den nur die Testsuite kennt, ist kein Nutzerpfad.
#
# WARUM EIN EIGENES DATENVERZEICHNIS: `TAXGRAPH_DATEN` bestimmt das Fall-Verzeichnis. Ohne
# die Variable zeigt der Dienst auf `~/.local/share/taxgraph` — den Bestand mit den echten
# Falldateien. Dieses Skript setzt sie IMMER auf ein Verzeichnis unter `.start/` im Repo-Wurzel
# (gitignored), damit der Start nichts anfasst, was ihm nicht gehört.
#
# Die Umgebung ist dieselbe, die der Parity-Test setzt (`starte()`, Zeile 120-145), damit
# beide Wege denselben Dienst beschreiben.
set -euo pipefail

WURZEL="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZIEL="${TAXGRAPH_TARGET_DIR:-$HOME/.cache/taxgraph-tmp/taxgraph-api-target}"

# --- Datenverzeichnis: NIEMALS der Bestand -----------------------------------
DATEN="${START_DATEN:-$WURZEL/.start/daten}"
case "$DATEN" in
    */.local/share/taxgraph|*/.local/share/taxgraph/) 
        echo "ABBRUCH: START_DATEN zeigt auf den Bestand ($DATEN)." >&2; exit 2 ;;
esac
FAELLE="$DATEN/faelle"
mkdir -p "$FAELLE" "$DATEN/audit"

# --- Bauen -------------------------------------------------------------------
BIN="$ZIEL/debug/taxgraph-api"
if [ ! -x "$BIN" ] || [ "${START_BAUEN:-0}" = "1" ]; then
    echo "baue taxgraph-api ..."
    ( cd "$WURZEL/rust" && CARGO_TARGET_DIR="$ZIEL" cargo build -p api --bin taxgraph-api )
fi

# --- Starten -----------------------------------------------------------------
PORT="${PORT:-0}"   # 0 = freier Port; der Dienst meldet ihn auf stdout
echo "Dienst startet (Daten=$DATEN)"
cd "$WURZEL"
exec env \
    CARGO_TARGET_DIR="$ZIEL" \
    TAXGRAPH_ROOT="$WURZEL" \
    TAXGRAPH_DATEN="$DATEN" \
    TAXGRAPH_AUDIT_DIR="$FAELLE" \
    TAXGRAPH_USER_STORE="$DATEN/users.json" \
    TAXGRAPH_JWT_SECRET="${TAXGRAPH_JWT_SECRET:-start-skript-geheimnis}" \
    TAXGRAPH_NO_AUTH="${TAXGRAPH_NO_AUTH:-1}" \
    TAXGRAPH_FLOW=0 \
    TAXGRAPH_KI_DEBUG=0 \
    LLM_API_KEY= LLM_API_BASE= LLM_MODEL= ORS_API_KEY= \
    XDG_DATA_HOME= \
    "$BIN" "$PORT"
