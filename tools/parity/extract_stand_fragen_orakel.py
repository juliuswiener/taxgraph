#!/usr/bin/env python3
"""Erzeugt rust/fixtures/api_stand_fragen_orakel.json: Faelle samt den Antworten des PYTHON-Orakels
(`api.stand`, `api.fragen`, `api.frage_einzeln`). Der Rust-Test `rust/api/tests/stand_fragen_orakel_hermetisch.rs`
spielt dieselben Ereignisse ueber `POST /event` in den Rust-Server und vergleicht die Antworten von
`GET /stand`, `GET /fragen` und `GET /feld/{fid}/frage` -- hermetisch, ohne Python zur Laufzeit.

Je Fall: `events` (die Rumpfe von `POST /fall/<id>/event`, Reihenfolge ist Semantik), `stand` (die ganze Antwort),
`fragen` (die ganze Antwort, nur bei `voll`; sonst `fragen_ids` samt Sperrgrund), `kopf` (was `flow.schreibe`
fuer `fragen` mitschreibt: Anzahl und Kopf der Queue) und `einzeln` (Antwort von `frage_einzeln` je Probe-Feld).

`event_id` jedes Felds in `stand` steht als `<event_id>` da: der Server haengt die Uhrzeit an das Ereignis, der Hash aendert
sich bei jedem Lauf. Die Datei ist sonst deterministisch (zwei Laeufe, dieselben Bytes).

Die Ereignislisten der Basisfaelle stehen in `rust/api/tests/kette_endstand_hermetisch.rs`
(`kegel_gesamt`, `kegel_an_gesamt`, `kegel_rentner`); das Skript liest sie von dort.

TEMP-WURZEL: das Skript setzt `$TAXGRAPH_DATEN` selbst, vor dem Import, auf ein frisches Verzeichnis und
loescht es am Ende. SICHERHEIT: alle Werte sind ERFUNDEN.

Neu erzeugen:   python3 tools/parity/extract_stand_fragen_orakel.py
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
QUELLE = os.path.join(ROOT, "rust", "api", "tests", "kette_endstand_hermetisch.rs")
OUT = os.environ.get("SF_OUT_JSON", os.path.join(ROOT, "rust", "fixtures", "api_stand_fragen_orakel.json"))

WURZEL = tempfile.mkdtemp(prefix="stand-fragen-orakel-")
atexit.register(shutil.rmtree, WURZEL, ignore_errors=True)
os.environ["TAXGRAPH_DATEN"] = WURZEL      # VOR dem Import: `api_constants.FAELLE`

sys.path.insert(0, HERE)
import korpus_faelle as KF                 # noqa: E402  (setzt sys.path und TAXGRAPH_NO_AUTH)
import api as API                          # noqa: E402
import flow as FLOW                      # noqa: E402  (Pythons `produkt/haut/flow.py`, ueber den Pfad von `api`)

VZ = 2025
KOPF = 6

# Probe-Felder fuer `frage_einzeln`: Typen (cent, int mit Bereich, bool invertiert, enum), Instanz, Basis fehlt.
PROBEN = [
    "bruttoarbeitslohn", "kein_gewinn", "veranlagung", "steuerklasse", "fam_anzahl_kinder", "kind_grad_der_behinderung",
    "kind_grad_der_behinderung__2", "kind_kv__1", "kinderbetreuungskosten__3", "ep_entfernung_km", "versicherungsart",
    "stammdaten_iban", "kist_konfession", "vv_entgelt_quote_prozent", "gibt_es_nicht", "gibt_es_nicht__2", "bruttoarbeitslohn__x1",
]

# Zwei Vermietungsobjekte (Instanz `__2`) und eine zweite Rente: Wirkung auf Ring und Guard, je bestaetigt
# und vorlaeufig. Dazu ein vorlaeufiges Einzelfeld, das der Ring liest. Ohne diese Faelle sah kein Fall den
# Unterschied zwischen "nur bestaetigt" und "auch vorlaeufig" und zwischen Ring mit und ohne Store
# (Auftrag 8, Mutanten H177, H184, H190, H211, H212, H215, H238).
VV_ZWEI = {"kein_vuv": False, "vv_einnahmen": 600000, "vv_anzahl_objekte": 2, "vv_einnahmen__2": 1200000,
           "vv_gebaeude_afa__2": 100000, "vv_schuldzinsen__2": 50000, "vv_erhaltungsaufwand__2": 30000,
           "vv_sonstige_wk__2": 20000, "vv_entgelt_quote_prozent__2": 100}
RENTE_ZWEI = {"rentner_renten_art__2": "gesetzliche_rente", "rentner_jahresrente__2": 1200000,
              "rentner_renten_beginn_jahr__2": 2020, "rentner_alter_bei_rentenbeginn__2": 65}

FAELLE = [
    dict(name="gesamt_kegel", scheibe="gesamt", basis="kegel_gesamt", voll=True, proben=PROBEN),
    dict(name="gesamt_vorlaeufiger_lohn", scheibe="gesamt", basis="kegel_gesamt", vorlaeufig=["bruttoarbeitslohn"]),
    dict(name="gesamt_ohne_lohn", scheibe="gesamt", basis="kegel_gesamt", ohne=["bruttoarbeitslohn"]),
    dict(name="gesamt_partner_offen", scheibe="gesamt", basis="kegel_gesamt", setze={"veranlagung": "zusammen"}),
    dict(name="gesamt_zwei_kinder", scheibe="gesamt", basis="kegel_gesamt", setze={"fam_anzahl_kinder": 2}, proben=PROBEN),
    dict(name="an_gesamt_kegel", scheibe="an_gesamt", basis="kegel_an_gesamt", voll=True),
    dict(name="an_gesamt_offene_achse", scheibe="an_gesamt", basis="kegel_an_gesamt", ohne=["fam_anzahl_kinder"]),
    dict(name="rentner_kegel", scheibe="rentner_gesamt", basis="kegel_rentner", voll=True),
    dict(name="rentner_rentenbeginn_offen", scheibe="rentner_gesamt", basis="kegel_rentner", ohne=["rentner_renten_beginn_jahr"]),
    dict(name="ep_leer", scheibe="ep", voll=True),
    dict(name="n_vor_gwg_leer", scheibe="n_vor_gwg", voll=True),
    dict(name="gesamt_vorlaeufige_vorsorge", scheibe="gesamt", basis="kegel_gesamt",
         setze={"vor_an_anteil_rv": 300000}, vorlaeufig=["vor_an_anteil_rv"]),
    dict(name="gesamt_zwei_objekte_bestaetigt", scheibe="gesamt", basis="kegel_gesamt", setze=VV_ZWEI, voll=True),
    dict(name="gesamt_zwei_objekte_vorlaeufig", scheibe="gesamt", basis="kegel_gesamt", setze=VV_ZWEI,
         vorlaeufig=[k for k in VV_ZWEI if k.endswith("__2")]),
    dict(name="rentner_zweite_rente_bestaetigt", scheibe="rentner_gesamt", basis="kegel_rentner", setze=RENTE_ZWEI),
    dict(name="rentner_zweite_rente_vorlaeufig", scheibe="rentner_gesamt", basis="kegel_rentner", setze=RENTE_ZWEI,
         vorlaeufig=list(RENTE_ZWEI)),
    dict(name="rentner_folgejahr_ohne_freibetrag", scheibe="rentner_gesamt", basis="kegel_rentner",
         setze={"rentner_renten_beginn_jahr": 2020}, ohne=["rentner_rentenfreibetrag"]),
    dict(name="rentner_folgejahr_mit_freibetrag", scheibe="rentner_gesamt", basis="kegel_rentner",
         setze={"rentner_renten_beginn_jahr": 2020}),
]


def rust_paare(fn: str, quelle: str) -> list[tuple[str, object]]:
    """Die `(Feld, Wert)`-Paare der Rust-Funktion `fn name() -> Paare { vec![ ... ] }`."""
    m = re.search(r"fn %s\(\) -> Paare \{\s*vec!\[(.*?)\]\s*\n\}" % re.escape(fn), quelle, re.S)
    if not m:
        sys.exit(f"Funktion {fn}() -> Paare fehlt in der Testdatei")
    return [(k, json.loads(re.sub(r"(?<=\d)_(?=\d)", "", v)))
            for k, v in re.findall(r'\("([a-z0-9_]+)",\s*json!\((.*?)\)\),?', m.group(1), re.S)]


def ereignis(feld_id: str, wert, vorlaeufig: bool) -> dict:
    e = KF._laie(feld_id, wert)
    if vorlaeufig:
        e["zustand"], e["signal"] = "vorlaeufig", {"signal_1": None, "signal_2": None}
    return e


def antwort(f, *a):
    """(status, body) einer Python-Funktion, ApiError als (status, {"fehler": detail})."""
    try:
        status, body = f(*a)
        return {"status": status, "body": body}
    except API.ApiError as e:
        return {"status": e.status, "body": {"fehler": str(e)}}


def lauf(nr: int, f: dict, quelle: str) -> dict:
    fall_id = f"sf{nr}"
    paare = rust_paare(f["basis"], quelle) if f.get("basis") else []
    d = dict(paare)
    d.update(f.get("setze", {}))
    paare = [(k, d[k]) for k, _ in paare if k not in f.get("ohne", [])] + [(k, v) for k, v in f.get("setze", {}).items() if k not in dict(paare)]
    st, _ = API.fall_anlegen({"scheibe": f["scheibe"], "veranlagungszeitraum": VZ, "fall_id": fall_id})
    assert st == 201, (f["name"], st)
    events = []
    for feld, wert in paare:
        e = ereignis(feld, wert, feld in f.get("vorlaeufig", []))
        st, _ = API.event(fall_id, e)
        assert st == 201, (f["name"], feld, st)
        events.append(e)
    _, stand = API.stand(fall_id)
    for fid, feld in stand["felder"].items():       # die event_id hat die Uhrzeit des Servers im Hash
        assert re.fullmatch(r"[0-9a-f]{64}", feld["event_id"]), (f["name"], fid)
        feld["event_id"] = "<event_id>"
    _, fragen = API.fragen(fall_id)
    aus = {"name": f["name"], "scheibe": f["scheibe"], "events": events, "stand": stand,
           "kopf": {"offen": len(fragen["fragen"]), "kopf": FLOW.kopf_der_queue(fragen["fragen"], KOPF)}}
    if f.get("voll"):
        aus["fragen"] = fragen
    else:
        aus["fragen_ids"] = [q["feld_id"] for q in fragen["fragen"]]
        aus["fragen_ring_gesperrt"] = [fragen["ring_gesperrt"], fragen["ring_gesperrt_klartext"], fragen["snapshot_id"]]
    if f.get("proben"):
        aus["einzeln"] = {p: antwort(API.frage_einzeln, fall_id, p) for p in f["proben"]}
    return aus


def main() -> int:
    quelle = open(QUELLE, encoding="utf-8").read()
    faelle = [lauf(i, f, quelle) for i, f in enumerate(FAELLE)]
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as fh:
        json.dump({"quelle": "produkt/haut/api.py (stand, fragen, frage_einzeln)", "faelle": faelle}, fh, ensure_ascii=False,
                  separators=(",", ":"), sort_keys=True)
        fh.write("\n")
    print(f"{len(faelle)} Faelle -> {OUT} ({os.path.getsize(OUT)} Bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
