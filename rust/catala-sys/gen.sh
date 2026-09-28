#!/usr/bin/env bash
# rust/catala-sys/gen.sh — generates the Catala C backend for the modules the Rust port
# needs today (aliases in produkt/engine/runner.py:66-84 minus Haushaltsnahe, plus
# GwgSofortabzug at runner.py:780) and drops the result into generated/.
#
# Mechanism (REWRITE_PLAN.md §3, Audit C Teil 2.3/2.6): scopes declared in ```catala fences
# compile to `static` C functions (module-private). The fix is a mechanical rewrite of every
# ```catala fence to ```catala-metadata on a COPY of rules/estg, built under
# $HOME/.cache/taxgraph-tmp — rules/ itself is never touched. Verified against the working
# probe (~/.cache/taxgraph-tmp/catala-c-probe/full/): a whole-tree diff of the converted
# modules is exactly this substitution, nothing else.
#
# generated/ is committed so `cargo build` needs no opam (Entscheidung REWRITE_PLAN.md §3).
# Re-run this script (`make catala-c`) after any change under rules/estg/.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/rust/catala-sys/generated"
BUILD_ROOT="${TMPDIR:-$HOME/.cache/taxgraph-tmp}/catala-sys-gen"

if ! command -v clerk >/dev/null 2>&1; then
  # shellcheck disable=SC2046
  eval $(opam env --switch=taxgraph --set-switch)
fi
command -v clerk >/dev/null 2>&1 || {
  echo "gen.sh: clerk not on PATH even after 'opam env --switch=taxgraph'. Install the" >&2
  echo "  taxgraph opam switch first (docs/setup.md)." >&2
  exit 1
}

rm -rf "$BUILD_ROOT"
mkdir -p "$BUILD_ROOT/rules/estg"
cp -r "$REPO_ROOT/rules/estg/." "$BUILD_ROOT/rules/estg/"

find "$BUILD_ROOT/rules/estg" -name '*.catala_en' -exec \
  sed -i 's/^```catala$/```catala-metadata/' {} +

cat > "$BUILD_ROOT/clerk.toml" <<'TOML'
[project]
# One entry per directory that declares Catala modules (module resolution is not recursive).
# All 27 rule directories under rules/estg are listed so cross-module `> Using` resolves, even
# though the "rust-c" target below only asks for the 19 modules the Rust port needs today.
include_dirs = [
  "rules/estg/p07_afa_ueberhangsjahr",
  "rules/estg/p11",
  "rules/estg/p35c",
  "rules/estg/p32a",
  "rules/estg/p04_arbeitszimmer_homeoffice",
  "rules/estg/p09_entfernungspauschale",
  "rules/estg/p32_kinderfreibetrag",
  "rules/estg/p33a_unterhalt",
  "rules/estg/p33",
  "rules/estg/p35a",
  "rules/estg/p10b",
  "rules/estg/p10",
  "rules/estg/p31",
  "rules/estg/p24a",
  "rules/estg/p24b",
  "rules/estg/p21_2",
  "rules/estg/p10_1_3_3a",
  "rules/estg/p10_1_7",
  "rules/estg/p16_4_freibetrag",
  "rules/estg/p4_3_gewinn",
  "rules/estg/p6_2_gwg_sofortabzug",
  "rules/estg/p10d_2_verlustvortrag_abzug",
  "rules/estg/p15_1_2_mitunternehmer",
  "rules/estg/p34_3_ermaessigter_durchschnittssatz",
  "rules/estg/solzg",
  "rules/estg/arbeitnehmerfall",
  "rules/estg/integration",
]
build_dir = "_build"
target_dir = "_target"

[[target]]
name = "rust-c"
modules = ["Einkommensteuertarif", "Entfernungspauschale", "Arbeitszimmer_homeoffice", "SpendenAbzug", "ZumutbareBelastung", "AgbAbzug", "Kirchensteuerabzug", "Altersentlastungsbetrag", "Entlastungsbetrag", "Familienleistungsausgleich", "VerbilligteVermietungWk", "KrankenPflegeVorsorge", "Berufsausbildungsaufwendungen", "BetriebsFreibetrag", "EuerGewinn", "Verlustvortrag", "MitunternehmerEinkuenfte", "ErmaessigterDurchschnittssatz", "GwgSofortabzug"]
backends = ["c"]
include_sources = false
TOML

(
  cd "$BUILD_ROOT"
  eval $(opam env --switch=taxgraph --set-switch)
  clerk build rust-c
)

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
cp "$BUILD_ROOT"/_target/rust-c/c/*.c "$BUILD_ROOT"/_target/rust-c/c/*.h "$OUT_DIR/"
cp "$BUILD_ROOT"/_build/libcatala/c/*.c "$BUILD_ROOT"/_build/libcatala/c/*.h "$OUT_DIR/"

find "$REPO_ROOT/rules" -name '*.catala_en' -print0 | sort -z \
  | xargs -0 cat | sha256sum | cut -d' ' -f1 > "$OUT_DIR/SOURCE_HASH"

echo "generated/ refreshed: $(find "$OUT_DIR" -type f | wc -l) files, SOURCE_HASH=$(cat "$OUT_DIR/SOURCE_HASH")"
