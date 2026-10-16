#!/bin/bash
# Pre-Commit-Pruefung der Bindungstabelle (E7 des Python-Loeschplans). Ersetzt den Hook, den `orch init-hooks` schrieb und
# der `pytest tests/test_bindungstabelle.py` rief; der faellt mit L2/L3 weg.
# Installation (einmal, main): `.git/hooks/pre-commit` ruft dieses Skript. Das Git-Verzeichnis gehoert allen Worktrees; ein Baum
# aelter als E7 hat das Skript nicht, dort uebergeht der Hook die Pruefung laut statt jeden Commit zu sperren:
#   s='"$(git rev-parse --show-toplevel)/scripts/orch-pre-commit.sh"'
#   printf '#!/bin/bash\ns=%s\n[ -x "$s" ] || { echo "[orch-gate] $s fehlt (Baum aelter als E7): Pre-Commit-Pruefung uebersprungen"; exit 0; }\nexec "$s"\n' "$s" > .git/hooks/pre-commit
# `orch init-hooks` schreibt den alten Hook zurueck, wenn man es erneut aufruft.
# Prueft alle Testbinaries des Crates `bindung` (Schema, Registry, Anker, Bereich, Beleg-Aufloesung, TESTMAP). Warm etwa 1 s,
# kalt (leeres Target) etwa 12 s. Eigenes Target je Baum, wenn CARGO_TARGET_DIR nicht gesetzt ist (rust/target).
# ponytail: nur `bindung`; die Kz-Pruefung gegen das amtliche Schema (rust/elster/tests) braucht das lokale ERiC-Schema und
# haengt nicht am Hook. Upgrade-Pfad: weitere Crates in die Zeile unten.
echo "🔍 [orch-gate] Pre-Commit: Bindungstabelle, Anker, Beleg-Aufloesung, TESTMAP (cargo test -p bindung)..."
top=$(git rev-parse --show-toplevel) || exit 1
# Git gibt dem Hook GIT_DIR, GIT_INDEX_FILE und `-c`-Einstellungen mit. Die Tests legen eigene Git-Verzeichnisse an und rufen
# `git commit`; ohne diese Zeile arbeiten sie im Repo des Hooks und rufen den Hook selbst (gemessen: belege_aufloesbar rot).
unset $(git rev-parse --local-env-vars)
cd "$top/rust" || { echo "❌ [orch-gate] rust/ fehlt in $top"; exit 1; }
if ! out=$(cargo test -p bindung --tests --quiet 2>&1); then
    printf '%s\n' "$out" | tail -80
    echo "❌ [orch-gate] Pre-Commit-Pruefung gescheitert. Behebe den Fehler vor dem Commit."
    exit 1
fi
echo "✅ [orch-gate] Pruefung bestanden."
exit 0
