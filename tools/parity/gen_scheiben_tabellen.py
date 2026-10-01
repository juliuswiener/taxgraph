#!/usr/bin/env python3
"""Erzeugt `rust/bescheid/src/deklaration/scheiben_tabellen.rs` aus `produkt/haut/api_constants.py`.

Warum ein Generator statt Handarbeit: die Scheiben-Tabelle ist **nicht ableitbar**. Kein
`bindung_*.yaml` traegt einen Scheiben-Schluessel (gemessen: 0 Treffer in 26 Dateien), die
Zuordnung "Feld -> Scheibe" steht nur in `SCHEIBEN`. Wer sie durch einen Namenspraefix zu
erraten versucht, liegt bei `an_gesamt` und `gesamt` falsch.

Erzeugt werden NUR die drei Dinge, die `Cfg` traegt: `felder`, `kegel`, `teil_ringe` je Scheibe.
Die 94 Einzelkonstanten werden NICHT mitgeschrieben -- `Cfg` braucht nur ihre Verkettung, und
unbenutzte Konstanten waeren toter Code. Die Python-Ausdruecke stehen als Kommentar ueber der
jeweiligen Tabelle, damit nachvollziehbar bleibt, woher die Verkettung kommt.

`n_vor_gwg` traegt `felder = None` und liest `bindung_n_vor_gwg.yaml`; hier wird nur der
Dateiname erzeugt, die 69 Feldnamen kommen zur Laufzeit aus derselben YAML wie in Python.

Nachpruefbar gegen Drift: `konstanten_gleich` in `rust/parity/tests/bescheid_deklaration_paritaet.rs`
vergleicht jede erzeugte Tabelle gegen das Orakel. Aendert sich `api_constants.py`, wird der Test
rot, bis der Generator neu laeuft.

Run: python3 tools/parity/gen_scheiben_tabellen.py
"""
from __future__ import annotations

import ast
import importlib.util
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
QUELLE = ROOT / "produkt" / "haut" / "api_constants.py"
ZIEL = ROOT / "rust" / "bescheid" / "src" / "deklaration" / "scheiben_tabellen.rs"

REIN = re.compile(r"^[a-z0-9_]+$")
PRO_ZEILE = 4


def _lade() -> object:
    spec = importlib.util.spec_from_file_location("ac", QUELLE)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _zeilenliste(name: str, felder: list[str], doc: str) -> list[str]:
    """Eine `const`-Deklaration, `PRO_ZEILE` Feldnamen je Zeile."""
    for f in felder:
        if not REIN.match(f):
            sys.exit(f"{name}: Feldname {f!r} hat unerwartete Zeichen -- Escaping noetig?")
    out = [f"/// {doc}", "#[rustfmt::skip]",
           f"pub(super) const {name}: [&str; {len(felder)}] = ["]
    for i in range(0, len(felder), PRO_ZEILE):
        stueck = ", ".join(f'"{f}"' for f in felder[i:i + PRO_ZEILE])
        out.append(f"    {stueck},")
    out.append("];")
    return out


def main() -> None:
    AC = _lade()
    src = QUELLE.read_text(encoding="utf-8")
    baum = ast.parse(src)
    zu = {n.targets[0].id: n
          for n in baum.body
          if isinstance(n, ast.Assign) and len(n.targets) == 1 and isinstance(n.targets[0], ast.Name)}

    knoten = next(n for n in ast.walk(baum)
                  if isinstance(n, ast.Assign) and isinstance(n.targets[0], ast.Name)
                  and n.targets[0].id == "SCHEIBEN")
    scheiben = list(knoten.value.keys)

    def ausdruck(schluessel: str, wert: ast.expr) -> str:
        for k, v in zip(knoten.value.keys, knoten.value.values):
            if k.value == schluessel:
                for kk, vv in zip(v.keys, v.values):
                    if kk.value == wert:
                        return ast.unparse(vv)
        return "?"

    kopf = [
        "//! GENERIERT von `tools/parity/gen_scheiben_tabellen.py` aus `produkt/haut/api_constants.py`.",
        "//! NICHT von Hand pflegen. Neu erzeugen:",
        "//!",
        "//! ```text",
        "//! python3 tools/parity/gen_scheiben_tabellen.py",
        "//! ```",
        "//!",
        "//! Die Feld-Tupel sind **nicht ableitbar**: kein `bindung_*.yaml` traegt einen",
        "//! Scheiben-Schluessel, die Zuordnung Feld -> Scheibe steht nur in `SCHEIBEN`.",
        "//! `#[rustfmt::skip]` haelt das Generator-Layout, damit ein erneuter Lauf byte-stabil bleibt.",
        "//!",
        "//! Drift faengt `konstanten_gleich` (`rust/parity/tests/bescheid_deklaration_paritaet.rs`):",
        "//! es vergleicht jede Tabelle hier gegen das Orakel.",
        "",
    ]
    zeilen = list(kopf)
    gesehen: set[str] = set()

    for name in scheiben:
        s = name.value
        cfg = AC.SCHEIBEN[s]
        up = s.upper()
        felder = cfg["felder"]
        if felder is None:
            zeilen.append(f"// `SCHEIBEN[{s!r}][\"felder\"]` ist `None` -- die Feldliste kommt zur")
            zeilen.append(f"// Laufzeit aus `produkt/bindung/{cfg['felder_datei']}` (`Cfg::felder_datei`).")
            zeilen.append("")
        else:
            doc = f"`SCHEIBEN[{s!r}][\"felder\"]` = `{ausdruck(s, 'felder')}`"
            zeilen += _zeilenliste(f"SCHEIBEN_{up}_FELDER", list(felder), doc)
            zeilen.append("")
            gesehen.add(f"SCHEIBEN_{up}_FELDER")

        kegel = cfg.get("kegel")
        if kegel is None:
            zeilen.append(f"// `SCHEIBEN[{s!r}][\"kegel\"]` ist `None` -- der Kegel ist der volle `felder`-Satz.")
            zeilen.append("")
        else:
            doc = f"`SCHEIBEN[{s!r}][\"kegel\"]` = `{ausdruck(s, 'kegel')}`"
            zeilen += _zeilenliste(f"SCHEIBEN_{up}_KEGEL", list(kegel), doc)
            zeilen.append("")
            gesehen.add(f"SCHEIBEN_{up}_KEGEL")

        teil = cfg.get("teil_ringe") or []
        for i, (tname, quant, tf) in enumerate(teil):
            cname = f"SCHEIBEN_{up}_TEIL_{i}_FELDER"
            zeilen += _zeilenliste(cname, list(tf),
                                   f"`SCHEIBEN[{s!r}][\"teil_ringe\"][{i}]` = `{tname}`: die Feldliste des Teil-Rings.")
            zeilen.append("")
            gesehen.add(cname)
        if teil:
            paare = ", ".join(f'("{t}", "{q}", &{f"SCHEIBEN_{up}_TEIL_{i}_FELDER"})'
                              for i, (t, q, _) in enumerate(teil))
            zeilen.append(f"/// `SCHEIBEN[{s!r}][\"teil_ringe\"]` -- `(familie, quantitaet, felder)`.")
            zeilen.append("#[rustfmt::skip]")
            zeilen.append("pub(super) const SCHEIBEN_" + up
                          + "_TEIL_RINGE: [(&str, &str, &[&str]); " + str(len(teil)) + "] = [")
            zeilen.append(f"    {paare},")
            zeilen.append("];")
            zeilen.append("")
            gesehen.add(f"SCHEIBEN_{up}_TEIL_RINGE")
        else:
            pass

    ZIEL.write_text("\n".join(zeilen), encoding="utf-8")
    n_felder = sum(len(AC.SCHEIBEN[k.value]["felder"] or ()) for k in scheiben)
    print(f"{len(gesehen)} Tabellen -> {ZIEL.relative_to(ROOT)}")
    print(f"felder-Eintraege: {n_felder} (n_vor_gwg aus YAML, nicht gezaehlt)")


if __name__ == "__main__":
    main()
