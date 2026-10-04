"""Zwei Waechter gegen einen stillen Rueckfall beim Ueberlauf. Beide pruefen Text, keinen Lauf:
kein Catala, kein Rust-Bau, kein Python-Orakel; `python3 -m pytest tests/test_ueberlauf_waechter_text.py`
genuegt.

1. Shim (`rust/catala-sys/csrc/`): `mpz_get_si` steht nur im Makro `TG_AUS`. GMP rechnet exakt;
   `mpz_get_si` gibt bei einem Wert ausserhalb von `long` still die unteren Bits zurueck. Das Makro
   haelt daneben fest, ob der Wert passte (`mpz_fits_slong_p`), und Rust liest erst dann
   (`Ausgabe::cent`). Ein `mpz_get_si` im Klartext waere ein Rueckfall, den kein anderer Test
   findet (Vault: ueberlauf-guard-am-uebersetzer-2026-10-04).
2. Profil: `make serve` baut mit `cargo build -p api --bin taxgraph-api` das dev-Profil
   (`$(SERVE_TARGET)/debug/`; `--release` verbietet das Makefile). Dort bleibt
   `overflow-checks = true` AUSDRUECKLICH stehen. Ohne die Pruefung bricht jedes ungeprueft
   geschriebene `+`/`-` der Geldrechnung still um (Bericht h8-casts, Punkt 7). Der aeltere Test
   `test_ci_konfiguration.py::test_rust_tests_laufen_mit_opt_level_1_und_pruefungen_bleiben_an`
   lehnt nur ein ausdrueckliches `= false` ab; fehlte die Zeile, liefe er gruen, weil Cargo sie
   vorgibt. Dieser Waechter verlangt sie und liest zusaetzlich, WELCHES Profil `make serve` baut.

Jeder Waechter ist eine reine Funktion auf Text (`shim_probleme`, `profil_probleme`). Die Tests
fuettern sie mit dem echten Text und mit Textfassungen, in denen genau der Rueckfall steckt, und
mit Umformatierungen, die NICHT anschlagen duerfen. Ein Waechter, der nur den echten Text sieht,
bewiese nichts: er kaeme auch gruen, wenn er nie anschluege.
"""
from __future__ import annotations

import re
import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
SHIM_VERZEICHNIS = ROOT / "rust" / "catala-sys" / "csrc"
CARGO_WURZEL = ROOT / "rust" / "Cargo.toml"
MAKEFILE = ROOT / "Makefile"
CARGO_KONFIG = (
    ROOT / ".cargo" / "config.toml", ROOT / ".cargo" / "config",
    ROOT / "rust" / ".cargo" / "config.toml", ROOT / "rust" / ".cargo" / "config",
)

# ---------------------------------------------------------------- Waechter 1: Shim

VERBOTEN = re.compile(r"\bmpz_get_si\b")
MAKRO_KOPF = re.compile(r"^\s*#\s*define\s+TG_AUS\b")
MAKRO_AUFRUF = re.compile(r"\bTG_AUS\s*\(")


def _c_ohne_kommentare(text: str) -> str:
    """C nach den Uebersetzungsphasen 2 und 3: Zeilenverkettung (`\\` am Zeilenende), dann
    Kommentare weg und Inhalt von Zeichenketten und -literalen leer. Zeilenenden bleiben, damit
    eine Direktive (`#define` mit Fortsetzungszeilen) EINE Zeile ist, egal wie sie umbrochen ist."""
    # Ersatz "" und kein Leerzeichen: der Praeprozessor klebt die Zeilen, `mpz_get_\<NL>si` ist `mpz_get_si`.
    text = re.sub(r"\\[ \t]*\r?\n", "", text)
    aus: list[str] = []
    i, n = 0, len(text)
    while i < n:
        c, d = text[i], text[i + 1 : i + 2]
        if c == "/" and d == "*":
            j = text.find("*/", i + 2)
            if j < 0:
                raise ValueError("Blockkommentar ohne Ende")
            aus.append(" ")
            i = j + 2
        elif c == "/" and d == "/":
            j = text.find("\n", i)
            aus.append(" ")
            i = n if j < 0 else j
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c and text[j] != "\n":
                j += 2 if text[j] == "\\" else 1
            aus.append(c + c)
            i = j + 1
        else:
            aus.append(c)
            i += 1
    return "".join(aus)


def shim_probleme(quellen: dict[str, str]) -> list[str]:
    """Leer = in Ordnung. `quellen`: Dateiname -> C-Text."""
    probleme: list[str] = []
    if not quellen:
        return ["keine C-Datei im Shim-Verzeichnis gefunden: der Waechter prueft nichts"]
    koepfe: list[str] = []
    aufrufe = 0
    for name, roh in quellen.items():
        for zeile in _c_ohne_kommentare(roh).split("\n"):
            if MAKRO_KOPF.match(zeile):
                koepfe.append(zeile)
                continue
            aufrufe += len(MAKRO_AUFRUF.findall(zeile))
            if VERBOTEN.search(zeile):
                probleme.append(f"{name}: mpz_get_si ausserhalb von TG_AUS: {' '.join(zeile.split())[:120]}")
    if len(koepfe) != 1:
        probleme.append(f"das Makro TG_AUS ist {len(koepfe)}-mal definiert, erwartet genau einmal")
    elif not (VERBOTEN.search(koepfe[0]) and "mpz_fits_slong_p" in koepfe[0]):
        probleme.append(
            "TG_AUS wandelt nicht mit mpz_get_si UND prueft mit mpz_fits_slong_p: "
            f"{' '.join(koepfe[0].split())[:120]}")
    if aufrufe == 0:
        probleme.append("TG_AUS wird nirgends aufgerufen: der Waechter prueft nichts")
    return probleme


def _shim_quellen() -> dict[str, str]:
    dateien = sorted([*SHIM_VERZEICHNIS.glob("*.c"), *SHIM_VERZEICHNIS.glob("*.h")])
    return {d.name: d.read_text(encoding="utf-8") for d in dateien}


# ---------------------------------------------------------------- Waechter 2: Profil

SERVE_VERBOTEN = (
    (re.compile(r"--release\b"), "--release"),
    (re.compile(r"--profile\b"), "--profile"),
    (re.compile(r"(?<![\w-])-r(?![\w-])"), "-r"),
    (re.compile(r"\bCARGO_PROFILE\w*"), "CARGO_PROFILE_*"),
)


def _make_zeilen(makefile: str) -> list[str]:
    """make-Zeilen nach der Fortsetzung mit `\\`."""
    return re.sub(r"\\[ \t]*\r?\n", " ", makefile).split("\n")


def _serve_rezept(makefile: str) -> list[str]:
    zeilen = _make_zeilen(makefile)
    kopf = next((i for i, z in enumerate(zeilen) if re.match(r"serve\s*:(?!=)", z)), None)
    if kopf is None:
        return []
    rezept: list[str] = []
    for z in zeilen[kopf + 1 :]:
        if z.startswith("\t"):
            rezept.append(z.strip())
        elif z.strip() == "" or z.lstrip().startswith("#"):
            continue
        else:
            break
    return rezept


def _verschachtelt_aus(tabelle: dict, pfad: str = "profile.dev") -> list[str]:
    """Jeder Eintrag `overflow-checks = false` unterhalb von `[profile.dev]` (Paket-, Build-Overrides)."""
    gefunden: list[str] = []
    for schluessel, wert in tabelle.items():
        if schluessel == "overflow-checks" and wert is not True:
            gefunden.append(f"{pfad}.overflow-checks = {wert!r}")
        elif isinstance(wert, dict):
            gefunden += _verschachtelt_aus(wert, f"{pfad}.{schluessel}")
    return gefunden


def profil_probleme(cargo_text: str, makefile: str, konfig: dict[str, str] | None = None) -> list[str]:
    """Leer = in Ordnung. `konfig`: Pfad -> Text vorhandener `.cargo/config`-Dateien."""
    probleme: list[str] = []

    # a) Welches Profil baut `make serve`?
    rezept = _serve_rezept(makefile)
    if not rezept:
        probleme.append("im Makefile fehlt das Ziel `serve`: das gebaute Profil ist nicht lesbar")
    else:
        cargo = [z for z in rezept if re.search(r"\bcargo\b", z)]
        bau = [z for z in cargo if re.search(r"\bcargo\s+build\b", z)]
        if not bau:
            probleme.append("`serve` baut nicht mit `cargo build`: das Profil ist nicht lesbar")
        for z in cargo:
            for muster, name in SERVE_VERBOTEN:
                if muster.search(z):
                    probleme.append(
                        f"`serve` waehlt mit {name} ein anderes Profil als dev; dessen overflow-checks "
                        f"prueft dieser Waechter nicht: {z[:120]}")
        if bau and not any(re.search(r"\bcd\s+rust\s*(&&|;)", z) for z in bau):
            probleme.append("`serve` baut nicht in `rust/`: dort liegt das Cargo.toml, das dieser Waechter liest")
        if not any(re.search(r"\$\(SERVE_TARGET\)/debug/taxgraph-api", z) for z in rezept):
            probleme.append("`serve` startet nicht `$(SERVE_TARGET)/debug/taxgraph-api` (das dev-Profil)")
    for z in _make_zeilen(makefile):
        if z.lstrip().startswith("#"):
            continue
        if re.search(r"overflow|\bRUSTFLAGS\b|\bCARGO_PROFILE\w*", z, re.IGNORECASE):
            probleme.append(f"das Makefile greift in die Ueberlaufpruefung oder das Profil ein: {z.strip()[:120]}")

    # b) Steht `overflow-checks = true` im dev-Profil, ausdruecklich?
    try:
        dev = tomllib.loads(cargo_text).get("profile", {}).get("dev", {})
    except tomllib.TOMLDecodeError as e:
        return [*probleme, f"rust/Cargo.toml ist kein gueltiges TOML: {e}"]
    if dev.get("overflow-checks") is not True:
        probleme.append(
            f"[profile.dev] overflow-checks ist {dev.get('overflow-checks', 'nicht gesetzt')!r}, erwartet ausdruecklich "
            "true: ohne die Pruefung bricht jedes ungeprueft geschriebene +/- still um")
    probleme += [f"{s}: schaltet die Pruefung fuer einen Teil des Baus aus" for s in _verschachtelt_aus(dev)
                 if not s.startswith("profile.dev.overflow-checks")]

    # c) Schaltet eine `.cargo/config` die Pruefung per rustflags aus?
    for pfad, text in (konfig or {}).items():
        if re.search(r"overflow", text, re.IGNORECASE):
            probleme.append(f"{pfad} nennt overflow: eine Cargo-Konfiguration kann die Pruefung ausschalten")
    return probleme


def _echte_konfig() -> dict[str, str]:
    return {str(p.relative_to(ROOT)): p.read_text(encoding="utf-8") for p in CARGO_KONFIG if p.is_file()}


# ---------------------------------------------------------------- die echten Dateien

def test_mpz_get_si_steht_nur_im_makro_tg_aus():
    quellen = _shim_quellen()
    assert "shim.c" in quellen, f"{SHIM_VERZEICHNIS}/shim.c fehlt: {sorted(quellen)}"
    assert shim_probleme(quellen) == []


def test_make_serve_baut_das_dev_profil_und_es_behaelt_overflow_checks():
    probleme = profil_probleme(
        CARGO_WURZEL.read_text(encoding="utf-8"), MAKEFILE.read_text(encoding="utf-8"), _echte_konfig())
    assert probleme == []


# ---------------------------------------------------------------- Waechter 1 gegen Text

MAKRO = (
    "#define TG_AUS(dst, r, feld)                          \\\n"
    "  do {                                                \\\n"
    "    (dst).wert = mpz_get_si((r)->feld);               \\\n"
    "    (dst).passt = mpz_fits_slong_p((r)->feld) != 0;   \\\n"
    "    (dst).name = #feld;                               \\\n"
    "  } while (0)\n"
)
NUTZER = "void f(Out *out, R *r) {\n  TG_AUS(*out, r, A__b);\n}\n"


def test_shim_waechter_laesst_den_sauberen_text_durch():
    assert shim_probleme({"shim.c": MAKRO + NUTZER}) == []


@pytest.mark.parametrize("schreibweise", [
    "mpz_get_si",
    "mpz_get_\\\nsi",  # der Praeprozessor klebt die Zeilen OHNE Leerzeichen zu `mpz_get_si`
    "mpz_\\ \nget_\\\r\nsi",
])
def test_shim_waechter_schlaegt_am_echten_text_an_wenn_ein_aufruf_zu_klartext_wird(schreibweise):
    """Der Rueckfall am echten Ort: ein `TG_AUS(...)` des echten shim.c wird in-memory durch das
    Klartext-Aequivalent ersetzt. Ohne diesen Test pruefte der Waechter nur, dass heute alles gut ist."""
    echt = _shim_quellen()["shim.c"]
    m = re.search(r"^(\s*)TG_AUS\(([^,]+),\s*r,\s*(\w+)\);", echt, re.MULTILINE)
    assert m, "kein TG_AUS-Aufruf in shim.c gefunden"
    klartext = f"{m.group(1)}({m.group(2).strip('*')}).wert = {schreibweise}(r->{m.group(3)});"
    mutiert = echt.replace(m.group(0), klartext, 1)
    assert mutiert != echt
    probleme = shim_probleme({"shim.c": mutiert})
    assert len(probleme) == 1 and "ausserhalb von TG_AUS" in probleme[0], probleme


@pytest.mark.parametrize("name, text", [
    ("Klartext-Aufruf", MAKRO + "long x(R *r) { return mpz_get_si(r->a); }\n" + NUTZER),
    ("Klartext in einer Zeile mit Makro-Aufruf", MAKRO + "void f(R *r) { TG_AUS(o, r, a); long y = mpz_get_si(r->b); }\n"),
    ("zweites Makro mit mpz_get_si", MAKRO + "#define NOCH_EINS(r) mpz_get_si(r)\n" + NUTZER),
    ("Alias-Makro", MAKRO + "#define GET mpz_get_si\n" + NUTZER),
    ("Aufruf ohne Leerraum vor der Klammer in einer Funktion", MAKRO + "static long g(mpz_srcptr z){return mpz_get_si(z);}\n" + NUTZER),
    ("Kommentarende vor dem Aufruf", MAKRO + "/* ok */ long x(R *r) { return mpz_get_si(r->a); }\n" + NUTZER),
    ("ein Schraegstrich-Stern-Muster im String schuetzt nicht", MAKRO + 'const char *s = "/*"; long x(R *r) { return mpz_get_si(r->a); }\n' + NUTZER),
    ("Name durch Zeilenverkettung gespalten", MAKRO + "long x(R *r) { return mpz_get_\\\nsi(r->a); }\n" + NUTZER),
    ("Name an zwei Stellen gespalten, Leerraum nach dem Strich", MAKRO + "long x(R *r) { return mpz_\\ \nget\\\n_si(r->a); }\n" + NUTZER),
    ("Name gespalten, CRLF", MAKRO + "long x(R *r) { return mpz_get_\\\r\nsi(r->a); }\r\n" + NUTZER),
])
def test_shim_waechter_schlaegt_an(name, text):
    probleme = shim_probleme({"shim.c": text})
    assert any("ausserhalb von TG_AUS" in p for p in probleme), (name, probleme)


@pytest.mark.parametrize("name, text", [
    ("Makro auf zwei Zeilen", "#define TG_AUS(dst, r, feld) \\\n  do { (dst).wert = mpz_get_si((r)->feld); (dst).passt = mpz_fits_slong_p((r)->feld) != 0; } while (0)\n" + NUTZER),
    ("Makro auf einer Zeile", "#define TG_AUS(dst, r, feld) do { (dst).wert = mpz_get_si((r)->feld); (dst).passt = mpz_fits_slong_p((r)->feld) != 0; } while (0)\n" + NUTZER),
    ("Makro, jedes Token auf eigener Zeile", "#define TG_AUS(dst, r, feld) \\\n do \\\n { \\\n (dst).wert = \\\n mpz_get_si \\\n ( \\\n (r)->feld \\\n ); \\\n (dst).passt = \\\n mpz_fits_slong_p((r)->feld) != 0; \\\n } \\\n while (0)\n" + NUTZER),
    ("Leerraum nach # und vor der Klammer", "#  define   TG_AUS (dst, r, feld) \\\n do { (dst).wert = mpz_get_si((r)->feld); (dst).passt = mpz_fits_slong_p((r)->feld); } while (0)\n" + NUTZER),
    ("Fortsetzungsstrich mit Leerraum dahinter und CRLF", MAKRO.replace("\\\n", "\\ \r\n") + NUTZER),
    ("Tabs statt Leerzeichen", MAKRO.replace("  ", "\t") + NUTZER),
    ("mpz_get_si im Blockkommentar", MAKRO + "/* frueher: mpz_get_si(r->a) */\n" + NUTZER),
    ("mpz_get_si im Zeilenkommentar", MAKRO + "// mpz_get_si(r->a)\n" + NUTZER),
    ("mpz_get_si im Kommentar ueber mehrere Zeilen", MAKRO + "/* Zeile eins\n * mpz_get_si(x)\n * Zeile drei */\n" + NUTZER),
    ("mpz_get_si in einer Zeichenkette", MAKRO + 'const char *s = "mpz_get_si";\n' + NUTZER),
    ("aehnlicher Name", MAKRO + "long mpz_get_si_alt(void); long my_mpz_get_si2;\n" + NUTZER),
    ("Kommentar mit Apostroph", MAKRO + "/* don't use mpz_get_si */\n" + NUTZER),
    ("Zeilenkommentar mit Fortsetzung", MAKRO + "// Hinweis \\\n mpz_get_si(x)\n" + NUTZER),
    ("Kommentaranfang und -ende durch Zeilenverkettung gespalten", MAKRO + "/\\\n* frueher: mpz_get_si(x) *\\\n/\n" + NUTZER),
    ("Name im Makrokopf gespalten", MAKRO.replace("mpz_get_si", "mpz_get_\\\nsi") + NUTZER),
    ("Makro-Aufruf gespalten", MAKRO + "void g(Out *o, R *r) { TG_\\\nAUS(*o, r, a); }\n"),
])
def test_shim_waechter_laesst_umformatierung_und_kommentare_durch(name, text):
    assert shim_probleme({"shim.c": text}) == [], name


@pytest.mark.parametrize("name, quellen, stichwort", [
    ("Makro fehlt", {"shim.c": NUTZER}, "0-mal definiert"),
    ("Makro doppelt", {"shim.c": MAKRO + MAKRO + NUTZER}, "2-mal definiert"),
    ("Makro ohne Passt-Pruefung", {"shim.c": "#define TG_AUS(dst, r, feld) do { (dst).wert = mpz_get_si((r)->feld); } while (0)\n" + NUTZER}, "mpz_fits_slong_p"),
    ("Makro ohne mpz_get_si", {"shim.c": "#define TG_AUS(dst, r, feld) do { (dst).passt = mpz_fits_slong_p((r)->feld); } while (0)\n" + NUTZER}, "mpz_get_si UND"),
    ("Makro wird nie benutzt", {"shim.c": MAKRO}, "nirgends aufgerufen"),
    ("keine Datei", {}, "keine C-Datei"),
])
def test_shim_waechter_meldet_ein_totes_oder_zerlegtes_makro(name, quellen, stichwort):
    assert any(stichwort in p for p in shim_probleme(quellen)), (name, shim_probleme(quellen))


# ---------------------------------------------------------------- Waechter 2 gegen Text

CARGO_OK = '[profile.dev]\ndebug = "line-tables-only"\nopt-level = 1\noverflow-checks = true\n'
MAKE_OK = (
    "serve: $(SICHERN_ZIEL)\n"
    "\tcd rust && CARGO_TARGET_DIR=$(SERVE_TARGET) cargo build -p api --bin taxgraph-api\n"
    '\t@echo "Rust-Dienst"\n'
    "\texec env -u TAXGRAPH_NO_AUTH $(SERVE_TARGET)/debug/taxgraph-api $(SERVE_PORT)\n"
    "\n"
    "serve-python: $(SICHERN_ZIEL)\n"
    "\texec env python3 produkt/haut/server.py $(SERVE_PORT)\n"
)


def test_profil_waechter_laesst_den_sauberen_text_durch():
    assert profil_probleme(CARGO_OK, MAKE_OK, {}) == []


# Die Zeile in jeder Schreibweise des Schluessels, die TOML kennt: nackt, "gequotet", 'literal'; Leerraum
# und Kommentar dahinter egal. ponytail: erste passende Zeile der Datei, Punkt-Schluessel
# (`profile.dev.overflow-checks = true`) und Inline-Tabellen erkennt die Regex nicht. Dann wird
# `_ohne_und_mit_false` LAUT rot (assert), nie still gruen; der Waechter selbst liest per tomllib
# und kennt beide Formen (Faelle unten). Ausbau, falls das Cargo.toml so umgebaut wird: den
# Rueckfall am geparsten dict erzeugen statt am Text.
UEBERLAUF_ZEILE = re.compile(r"^[ \t]*([\"']?)overflow-checks\1[ \t]*=[ \t]*true\b[^\n]*$", re.MULTILINE)


def _ohne_und_mit_false(cargo_text: str) -> tuple[str, str]:
    """Der Rueckfall am Text: dieselbe Datei ohne die Zeile / mit `= false` (Schreibweise bleibt)."""
    zeile = UEBERLAUF_ZEILE.search(cargo_text)
    assert zeile, "rust/Cargo.toml hat keine ausdrueckliche Zeile `overflow-checks = true` im Profil dev"
    ohne = cargo_text.replace(zeile.group(0), "", 1)
    aus = cargo_text.replace(zeile.group(0), re.sub(r"\btrue\b", "false", zeile.group(0), count=1), 1)
    assert ohne != cargo_text and aus != cargo_text
    return ohne, aus


def test_profil_waechter_schlaegt_am_echten_text_an_wenn_die_zeile_fehlt_oder_false_ist():
    """Der Rueckfall am echten Ort: das echte Cargo.toml in-memory ohne die Zeile / mit `= false`."""
    echt = CARGO_WURZEL.read_text(encoding="utf-8")
    make = MAKEFILE.read_text(encoding="utf-8")
    ohne, aus = _ohne_und_mit_false(echt)
    assert any("nicht gesetzt" in p for p in profil_probleme(ohne, make))
    assert any("False" in p for p in profil_probleme(aus, make))


@pytest.mark.parametrize("schreibweise", [
    "overflow-checks = true",
    "overflow-checks=true",
    '"overflow-checks" = true',
    "'overflow-checks' = true",
    '"overflow-checks"=true # nie ausschalten',
    "  overflow-checks\t=\ttrue",
])
def test_profil_waechter_faengt_den_rueckfall_in_jeder_schluesselschreibweise(schreibweise):
    """Das echte Cargo.toml, nur die Zeile umgeschrieben: unveraendert in Ordnung, ohne die Zeile und mit
    `= false` rot. Haette der Selbsttest nur die nackte Schreibweise gefunden, fiele er bei der ersten
    Umformatierung der Zeile um, ohne dass der Waechter etwas falsch gemacht haette."""
    echt = CARGO_WURZEL.read_text(encoding="utf-8")
    make = MAKEFILE.read_text(encoding="utf-8")
    zeile = UEBERLAUF_ZEILE.search(echt)
    assert zeile, "rust/Cargo.toml hat keine ausdrueckliche Zeile `overflow-checks = true` im Profil dev"
    text = echt.replace(zeile.group(0), schreibweise, 1)
    assert profil_probleme(text, make) == []
    ohne, aus = _ohne_und_mit_false(text)
    assert any("nicht gesetzt" in p for p in profil_probleme(ohne, make))
    assert any("False" in p for p in profil_probleme(aus, make))


@pytest.mark.parametrize("name, cargo", [
    ("Zeile fehlt (Cargo gaebe sie vor, der Waechter verlangt sie)", '[profile.dev]\nopt-level = 1\n'),
    ("false", CARGO_OK.replace("true", "false")),
    ("Text statt Wahrheitswert", CARGO_OK.replace("true", '"true"')),
    ("Profil dev fehlt", '[profile.release]\noverflow-checks = true\n'),
    ("nur im Profil release gesetzt", '[profile.dev]\nopt-level = 1\n[profile.release]\noverflow-checks = true\n'),
    ("Paket-Override schaltet aus", CARGO_OK + '[profile.dev.package."*"]\noverflow-checks = false\n'),
    ("Build-Override schaltet aus", CARGO_OK + '[profile.dev.build-override]\noverflow-checks = false\n'),
    ("verschachtelt zwei Ebenen", CARGO_OK + '[profile.dev.package.domain]\noverflow-checks = false\n'),
    ("gequoteter Schluessel mit false", CARGO_OK.replace("overflow-checks = true", '"overflow-checks" = false')),
    ("gequoteter Schluessel in Punktschreibweise mit false", 'profile.dev.opt-level = 1\nprofile.dev."overflow-checks" = false\n'),
    ("gequoteter Schluessel verschwunden, nur Kommentar bleibt", CARGO_OK.replace("overflow-checks = true", '# "overflow-checks" = true')),
])
def test_profil_waechter_schlaegt_am_cargo_toml_an(name, cargo):
    assert profil_probleme(cargo, MAKE_OK, {}) != [], name


@pytest.mark.parametrize("name, cargo", [
    ("Punktschreibweise", 'profile.dev.opt-level = 1\nprofile.dev.overflow-checks = true\n'),
    ("Inline-Tabelle", '[profile]\ndev = { opt-level = 1, overflow-checks = true }\n'),
    ("ohne Leerzeichen um das Gleichheitszeichen", '[profile.dev]\noverflow-checks=true\n'),
    ("Kommentar dahinter und davor", '[profile.dev]\n# bleibt an\noverflow-checks = true # nie ausschalten\n'),
    ("Reihenfolge der Schluessel", '[profile.dev]\noverflow-checks = true\nopt-level = 1\ndebug = "line-tables-only"\n'),
    ("Kopf mit Leerraum", '[ profile.dev ]\noverflow-checks = true\n'),
    ("gequoteter Schluessel", CARGO_OK.replace("overflow-checks = true", '"overflow-checks" = true')),
    ("Schluessel in Apostrophen", CARGO_OK.replace("overflow-checks = true", "'overflow-checks' = true")),
    ("gequoteter Schluessel in Punktschreibweise", 'profile.dev.opt-level = 1\nprofile.dev."overflow-checks" = true\n'),
    ("Paket-Override mit true", CARGO_OK + '[profile.dev.package."*"]\noverflow-checks = true\nopt-level = 3\n'),
    ("fremde Profile daneben", CARGO_OK + '[profile.release]\noverflow-checks = false\n[profile.test]\nopt-level = 1\n'),
])
def test_profil_waechter_laesst_umformatierung_durch(name, cargo):
    assert profil_probleme(cargo, MAKE_OK, {}) == [], name


@pytest.mark.parametrize("name, make", [
    ("--release", MAKE_OK.replace("--bin taxgraph-api", "--bin taxgraph-api --release")),
    ("--profile", MAKE_OK.replace("-p api", "-p api --profile serve")),
    ("-r", MAKE_OK.replace("-p api", "-p api -r")),
    ("CARGO_PROFILE_ in der Rezeptzeile", MAKE_OK.replace("cargo build", "CARGO_PROFILE_DEV_OPT_LEVEL=3 cargo build")),
    ("release-Verzeichnis", MAKE_OK.replace("/debug/", "/release/")),
    ("falsches Verzeichnis", MAKE_OK.replace("cd rust &&", "cd rust/fuzz &&")),
    ("anderer Befehl", MAKE_OK.replace("cargo build", "cargo install --path .")),
    ("kein serve-Ziel", MAKE_OK.replace("serve:", "dienen:", 1)),
    ("RUSTFLAGS im Makefile", "RUSTFLAGS = -C overflow-checks=off\n" + MAKE_OK),
    ("Ueberlaufpruefung im Makefile", "export CARGO_PROFILE_DEV_OVERFLOW_CHECKS = false\n" + MAKE_OK),
])
def test_profil_waechter_schlaegt_am_makefile_an(name, make):
    assert profil_probleme(CARGO_OK, make, {}) != [], name


@pytest.mark.parametrize("name, make", [
    ("Rezeptzeile mit make-Fortsetzung", MAKE_OK.replace("cargo build -p api", "cargo build \\\n\t  -p api")),
    ("Kommentare und Leerzeilen im Rezept", MAKE_OK.replace('\t@echo', "\n# Kommentar am Zeilenanfang\n\t@echo")),
    ("Kommentar nennt --release und overflow", "## Kein --release: overflow-checks gehen sonst verloren.\n" + MAKE_OK),
    ("serve-python daneben darf anders bauen", MAKE_OK + "\tcargo build --release\n"),
    ("Leerraum vor dem Doppelpunkt", MAKE_OK.replace("serve:", "serve :", 1)),
])
def test_profil_waechter_laesst_umformatierung_im_makefile_durch(name, make):
    assert profil_probleme(CARGO_OK, make, {}) == [], name


def test_profil_waechter_schlaegt_an_wenn_eine_cargo_konfiguration_overflow_nennt():
    konfig = {".cargo/config.toml": '[build]\nrustflags = ["-C", "overflow-checks=off"]\n'}
    assert any("overflow" in p for p in profil_probleme(CARGO_OK, MAKE_OK, konfig))
    assert profil_probleme(CARGO_OK, MAKE_OK, {".cargo/config.toml": "[net]\nretry = 3\n"}) == []
