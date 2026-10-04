#!/usr/bin/env python3
"""Python-Erwartung fuer die festen Faelle in `rust/api/tests/kette_endstand_hermetisch.rs`.

WARUM ES DIESE DATEI GIBT. Die Tests dort laufen ohne `PARITY=1` und ohne Python; ihre Erwartungswerte sind eingefroren
(ponytail im Kopf der Testdatei). Aendert sich der Tarif oder der Kegel, rechnet man sie hier nach. Das Skript liest die
Feldlisten (`kegel_*()` und `*_aenderungen()`) aus dem Rust-Quelltext, schickt sie ueber die echten Endpunkte an den
Python-Server (`tools/parity/korpus_faelle.lege_an` = `api.fall_anlegen` + `api.event`, dann `api.ergebnis`) und druckt die
Felder, die die Tests pruefen. Es meldet ausserdem, wenn `tests/_kegel.py::kegel_fuer` zur Feldliste etwas hinzufuegt oder
einen Wert aendert -- dann pruefte der Rust-Test einen anderen Fall als Python.

Aufruf (aus dem Repo-Wurzelverzeichnis):

    python3 tools/parity/kette_erwartung.py [--datei=PFAD] [--vorlaeufig=FELD,FELD] SCHEIBE BASIS_FN AENDERUNGS_FN [AENDERUNGS_FN ...]
    python3 tools/parity/kette_erwartung.py gesamt kegel_gesamt g5_vg_einzel_p34_aenderungen
    python3 tools/parity/kette_erwartung.py an_gesamt kegel_an_gesamt a4_partner_kv_pv_aenderungen

`--datei` waehlt eine andere Rust-Testdatei mit denselben Funktionsformen (Vorgabe: die API-Testdatei oben).
`--vorlaeufig` schreibt die genannten Felder als VORLAEUFIG (kein `signal_2`) statt bestaetigt, wie `ergebnis_vorlaeufig` im Rust-Test.

Ausgabe je Fall (eine JSON-Zeile): `grund`, `zahl` (Cent), `solz`, `kist`, `mobil` (Mobilitaetspraemie), `abschluss`
(Abschlusszahlung), `kette` (GdE/zvE/tarifliche/festzusetzende in EURO), `p31` (Sieger des § 31) und `kap_guenstiger`
(§ 32d Abs. 6: hat der tarifliche Zweig gewonnen? steht nur in den `extras` von `_feste_zahl`, nicht in der Antwort).

TEMP-WURZEL: das Skript setzt `$TAXGRAPH_DATEN` selbst, vor dem Import, auf ein frisches Verzeichnis und loescht es am Ende.
Die echte Fallliste (~/.local/share/taxgraph/faelle) bleibt unberuehrt.

SICHERHEIT: alle Werte sind ERFUNDEN (keine echten Steuerdaten).
"""
from __future__ import annotations

import atexit
import json
import os
import re
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
TESTDATEI = os.path.join(ROOT, "rust", "api", "tests", "kette_endstand_hermetisch.rs")

WURZEL = tempfile.mkdtemp(prefix="kette-erwartung-")
atexit.register(shutil.rmtree, WURZEL, ignore_errors=True)
os.environ["TAXGRAPH_DATEN"] = WURZEL      # VOR dem Import: `api_constants.FAELLE`

sys.path.insert(0, HERE)
from korpus_faelle import lege_an          # noqa: E402  (setzt sys.path und TAXGRAPH_NO_AUTH)
from _kegel import kegel_fuer              # noqa: E402
import api as API                          # noqa: E402
import korpus_faelle as KF                 # noqa: E402

VORLAEUFIG: set = set()
_LAIE = KF._laie


def _laie_vorlaeufig(feld_id: str, wert):
    """Wie `korpus_faelle._laie`, aber die Felder in `VORLAEUFIG` ohne `signal_2` und als `vorlaeufig`."""
    e = _LAIE(feld_id, wert)
    if feld_id in VORLAEUFIG:
        e["zustand"], e["signal"] = "vorlaeufig", {"signal_1": None, "signal_2": None}
    return e


KF._laie = _laie_vorlaeufig


# `extras["kap_guenstiger_gewonnen"]` erreicht die HTTP-Antwort nie; hier wird `_feste_zahl` abgehoert.
_EXTRAS: list = []
_FESTE_ZAHL = API._feste_zahl


def _feste_zahl_abgehoert(*args, **kwargs):
    r = _FESTE_ZAHL(*args, **kwargs)
    if r:
        _EXTRAS.append(r[2])
    return r


API._feste_zahl = _feste_zahl_abgehoert


def rust_paare(fn: str, quelle: str) -> list[tuple[str, object]]:
    """Die `(Feld, Wert)`-Paare der Rust-Funktion `fn name() -> Paare { vec![ ... ] }`."""
    m = re.search(r"fn %s\(\) -> Paare \{\s*vec!\[(.*?)\]\s*\n\}" % re.escape(fn), quelle, re.S)
    if not m:
        sys.exit(f"Funktion {fn}() -> Paare fehlt in der Testdatei")
    return [(k, json.loads(re.sub(r"(?<=\d)_(?=\d)", "", v)))
            for k, v in re.findall(r'\("([a-z0-9_]+)",\s*json!\((.*?)\)\),?', m.group(1), re.S)]


def zusammen(basis: list, aenderungen: list) -> dict:
    """Wie `mit()` im Rust-Test: ein vorhandenes Feld wird ersetzt, ein neues angehaengt."""
    d = dict(basis)
    d.update(dict(aenderungen))
    return d


def kompakt(a: dict, extras: dict | None = None) -> dict:
    k = a.get("kette") or {}
    stufen = [k.get(s) for s in ("gesamtbetrag_der_einkuenfte", "zu_versteuerndes_einkommen", "tarifliche_est",
                                 "festzusetzende_est")]
    p31 = k.get("p31")
    return {"grund": a.get("grund"), "zahl": a.get("zahl_cent"), "solz": a.get("solz_cent"), "kist": a.get("kist_cent"),
            "mobil": a.get("mobilitaetspraemie_cent"), "abschluss": a.get("abschlusszahlung_cent"),
            "kette": stufen if any(x is not None for x in stufen) else None,
            "p31": p31 and {"guenstiger": p31.get("guenstiger"), "kindergeld": p31.get("kindergeld")},
            "kap_guenstiger": (extras or {}).get("kap_guenstiger_gewonnen")}


def main(argv: list[str]) -> int:
    datei = TESTDATEI
    while argv and argv[0].startswith("--"):
        if argv[0].startswith("--datei="):
            datei = os.path.abspath(argv[0].split("=", 1)[1])
        elif argv[0].startswith("--vorlaeufig="):
            VORLAEUFIG.update(f for f in argv[0].split("=", 1)[1].split(",") if f)
        else:
            sys.exit(__doc__)
        argv = argv[1:]
    if len(argv) < 3:
        sys.exit(__doc__)
    scheibe, basis_fn, *faelle = argv
    quelle = open(datei, encoding="utf-8").read()
    for nr, fn in enumerate(faelle, 1):
        gesetzt = zusammen(rust_paare(basis_fn, quelle), rust_paare(fn, quelle))
        ergaenzt = dict(kegel_fuer(scheibe, dict(gesetzt)))
        diff = sorted(set(gesetzt) ^ set(ergaenzt)) + sorted(k for k in gesetzt if k in ergaenzt and gesetzt[k] != ergaenzt[k])
        fall_id = f"kette{nr}"
        status = lege_an(fall_id, scheibe, gesetzt)
        if status != "neu":
            print(json.dumps({"fall": fn, "abweisung": status}, ensure_ascii=False))
            continue
        _EXTRAS.clear()
        _, antwort = API.ergebnis(fall_id)
        print(json.dumps({"fall": fn, "kegel_abweichung": diff or None, **kompakt(antwort, _EXTRAS[-1] if _EXTRAS else None)},
                         ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
