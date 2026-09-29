"""Pytest-Plugin: zeichnet echte Aufrufe der `runner.catala_*`-Funktionen aus Schritt 4a
waehrend eines Testlaufs in JSONL auf, CENT-genau im selben `fn`/`args`-Schema wie
`tools/parity/oracle.py`s `DISPATCH` (`REWRITE_PLAN.md` §5). Aktivierung:

    python3 -m pytest tests/ -p tools.parity.record -q

Bewusst OHNE `-n`/`--dist` -- mehrere xdist-Worker sind separate Prozesse und teilen sich
die `_faelle`-Liste nicht; sie wuerden dieselbe Datei gegenseitig ueberschreiben. Laeuft ein
Worker trotzdem parallel (`PYTEST_XDIST_WORKER` gesetzt), bekommt seine Datei einen
Worker-Suffix, und `engine_catala_paritaet.rs`s Corpus-Replay liest alle passenden Dateien.

`catala_p24a_altersentlastung` (Altersentlastungsbetrag) faellt NICHT unter dieses Schema:
der Wrapper leitet `prozentsatz`/`hoechstbetrag` selbst aus einer Kohorten-YAML ab (Teil B,
`rust/engine/src/lib.rs`s Teil-A/Teil-B-Grenze) -- kein 1:1-Pass-through der rohen
Catala-Scope-Eingaben. Die Paritaet fuer diese Funktion deckt allein der Proptest gegen den
Oracle ab (`engine_catala_paritaet.rs`), ohne Corpus-Eintrag.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
_WORKER = os.environ.get("PYTEST_XDIST_WORKER", "")
CORPUS = ROOT / "rust" / "fixtures" / "corpus" / f"engine_calls{f'.{_WORKER}' if _WORKER else ''}.jsonl"

_faelle: list[dict] = []
_originale: list[tuple[object, str, object]] = []
_runner_mod = None  # wird in pytest_configure gesetzt (spaetes Binden, siehe _EXTRAKTOREN)


def _cent(s: dict, key: str) -> int:
    """`s[key]` ist EURO-int (runner.py-Konvention); verlustfrei nach Cent (× 100)."""
    return int(s.get(key, 0)) * 100


def _zumutbar_cent(s: dict) -> int:
    """Dieselbe interne Ableitung wie `catala_p33_agb` selbst (`_zumutbar_money`) --
    kein neuer Teil-B-Schritt, nur die bereits vorhandene Berechnung erneut gelesen."""
    assert _runner_mod is not None, "pytest_configure() setzt _runner_mod vor jedem Aufruf"
    return int(_runner_mod._zumutbar_money(s))


_EXTRAKTOREN: dict[str, tuple[str, object]] = {
    "catala_p10b_spenden": ("spenden_abzug", lambda s: {
        "zuwendungen_cent": _cent(s, "zuwendungen"),
        "gesamtbetrag_der_einkuenfte_cent": _cent(s, "gesamtbetrag_der_einkuenfte"),
    }),
    "catala_p33_zumutbar": ("zumutbare_belastung", lambda s: {
        "gesamtbetrag_der_einkuenfte_cent": _cent(s, "gesamtbetrag_der_einkuenfte"),
        "anzahl_kinder": int(s.get("anzahl_kinder", 0)),
        "splitting": bool(s.get("splitting", False)),
    }),
    "catala_p33_agb": ("agb_abzug", lambda s: {
        "aussergewoehnliche_belastungen_cent": _cent(s, "aussergewoehnliche_belastungen"),
        "zumutbare_belastung_cent": _zumutbar_cent(s),
    }),
    "catala_p10_kist": ("kirchensteuerabzug", lambda s: {
        "gezahlte_kirchensteuer_cent": _cent(s, "gezahlte_kirchensteuer"),
        "erstattete_kirchensteuer_cent": _cent(s, "erstattete_kirchensteuer"),
    }),
    "catala_p24b_entlastung": ("entlastungsbetrag", lambda s: {
        "alleinstehend": bool(s.get("alleinstehend", False)),
        "anzahl_kinder": int(s.get("anzahl_kinder", 0)),
        "monate_ohne_voraussetzung": int(s.get("monate_ohne_voraussetzung", 0)),
    }),
    "catala_p31_familienleistung": ("familienleistungsausgleich", lambda s: {
        "est_ohne_freibetraege_cent": _cent(s, "est_ohne_freibetraege"),
        "est_mit_freibetraegen_cent": _cent(s, "est_mit_freibetraegen"),
        "kindergeld_cent": _cent(s, "kindergeld"),
    }),
    "catala_p21_2_verbilligt": ("verbilligte_vermietung_wk", lambda s: {
        "werbungskosten_cent": _cent(s, "werbungskosten"),
        "entgelt_quote_prozent_num": int(s.get("entgelt_quote_prozent", 100)),
        "entgelt_quote_prozent_den": 1,
    }),
    "catala_p10_kv_pv": ("kranken_pflege_vorsorge", lambda s: {
        "basis_cent": _cent(s, "basis_kv_pv"),
        "weitere_cent": _cent(s, "weitere_vorsorgeaufwendungen"),
        "mit_zuschuss": bool(s.get("mit_anspruch_auf_zuschuss", False)),
    }),
    "catala_p10_1_7_berufsausbildung": ("berufsausbildung", lambda s: {
        "aufwendungen_cent": _cent(s, "berufsausbildung_aufwendungen"),
    }),
    "catala_p16_4_freibetrag": ("betriebs_freibetrag", lambda s: {
        "veraeusserungsgewinn_cent": _cent(s, "rentner_veraeusserungsgewinn"),
    }),
    "catala_euer_gewinn": ("euer_gewinn", lambda s: {
        "betriebseinnahmen_cent": _cent(s, "betriebseinnahmen"),
        "betriebsausgaben_cent": _cent(s, "betriebsausgaben"),
    }),
    "catala_mitunternehmer_einkuenfte": ("mitunternehmer_einkuenfte", lambda s: {
        "gewinnanteil_cent": _cent(s, "gewinnanteil"),
        "verguetung_taetigkeit_cent": _cent(s, "verguetung_taetigkeit"),
        "verguetung_darlehen_cent": _cent(s, "verguetung_darlehen"),
        "verguetung_ueberlassung_cent": _cent(s, "verguetung_ueberlassung"),
    }),
    "catala_p6_2_gwg": ("gwg_sofortabzug", lambda s: {
        "anschaffungskosten_netto_cent": _cent(s, "gwg_anschaffungskosten_netto"),
    }),
    "catala_p10d_2": ("verlustvortrag_abzug", lambda s: {
        "gesamtbetrag_einkuenfte_cent": _cent(s, "gesamtbetrag_einkuenfte"),
        "verlustvortrag_bestand_cent": _cent(s, "verlustvortrag_bestand"),
        "zusammenveranlagung": bool(s.get("zusammenveranlagung", False)),
    }),
    "catala_ermaessigter_durchschnittssatz": ("ermaessigter_durchschnittssatz", lambda s: {
        "ao_einkuenfte_cent": _cent(s, "ao_einkuenfte"),
        "est_gesamt_zzgl_progression_cent": _cent(s, "est_gesamt_zzgl_progression"),
        "bemessungsgrundlage_durchschnitt_cent": _cent(s, "bemessungsgrundlage_durchschnitt"),
    }),
}


def _make_wrapper(original, dispatch_fn: str, extractor):
    def wrapper(s: dict) -> int:
        ergebnis = original(s)
        try:
            _faelle.append({"fn": dispatch_fn, "args": extractor(s)})
        except Exception:  # noqa: BLE001 -- Aufzeichnung darf den echten Testlauf nie stoeren
            pass
        return ergebnis

    return wrapper


def _runner_module_varianten() -> list:
    """Alle Modul-Objekte, unter denen `produkt/engine/runner.py` im Testlauf auftaucht.

    `tests/conftest.py` haengt `produkt/engine` zusaetzlich BAR-benannt in `sys.path`; dieselbe
    Datei landet dann als ZWEI getrennte Modulobjekte mit je eigenem `sys.modules`-Eintrag --
    `runner` (bar, von den Testfixtures via `import runner` genutzt) und `produkt.engine.runner`
    (gepunktet). Patchen nur des einen faengt Aufrufe ueber den anderen Namen nicht ab.
    """
    engine_dir = str(ROOT / "produkt" / "engine")
    if engine_dir not in sys.path:
        sys.path.insert(0, engine_dir)
    varianten = []
    try:
        import runner as bare_runner

        varianten.append(bare_runner)
    except ImportError:
        pass
    import produkt.engine.runner as dotted_runner

    if not varianten or dotted_runner is not varianten[0]:
        varianten.append(dotted_runner)
    return varianten


def pytest_configure(config) -> None:  # noqa: ARG001 (pytest-Hook-Signatur)
    global _runner_mod
    module_varianten = _runner_module_varianten()
    _runner_mod = module_varianten[0]
    for modul in module_varianten:
        for attr, (dispatch_fn, extractor) in _EXTRAKTOREN.items():
            original = getattr(modul, attr)
            _originale.append((modul, attr, original))
            setattr(modul, attr, _make_wrapper(original, dispatch_fn, extractor))


def pytest_unconfigure(config) -> None:  # noqa: ARG001 (pytest-Hook-Signatur)
    for modul, attr, original in _originale:
        setattr(modul, attr, original)
    if not _faelle:
        return
    CORPUS.parent.mkdir(parents=True, exist_ok=True)
    with CORPUS.open("w", encoding="utf-8") as f:
        for fall in _faelle:
            f.write(json.dumps(fall, sort_keys=True) + "\n")
    print(f"\ntools/parity/record.py: {len(_faelle)} Faelle -> {CORPUS}")
