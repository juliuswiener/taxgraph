"""Paritaets-Orakel fuer `rust/elster` (Schritt 6): beantwortet `{"fn": "elster.<name>", ...}`
ueber die Python-Referenz `est_mapping.py`, `elster_xml.py`, `xsd_verify.py`, `checkest_gate.py`.

Module werden wie in `produkt/haut/api.py` geladen (flache Modulnamen ueber sys.path), damit
dieselben Modul-Identitaeten gelten wie im Produkt. Der Import passiert erst beim ersten Aufruf.

Antwort: `{"ok": <ergebnis>}` oder `{"err": "<Ausnahmeklasse>", "msg": "<str(exc)>"}`.
Die Hersteller-ID kommt nur im Request (Pipe), nie aus einer Datei dieses Moduls.
"""
from __future__ import annotations

import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_M: dict = {}


def _module() -> dict:
    if not _M:
        for sub in ("produkt/store", "produkt/traverser", "produkt/mapping", "produkt/eingang", "elster"):
            p = os.path.join(ROOT, sub)
            if p not in sys.path:
                sys.path.insert(0, p)
        import est_mapping as EM  # noqa: E402
        import elster_xml as EX  # noqa: E402
        import xsd_verify as XV  # noqa: E402
        import store as ST  # noqa: E402
        import traverser as TR  # noqa: E402
        _M.update(EM=EM, EX=EX, XV=XV, ST=ST, bindung=TR.lade_bindung())
    return _M


def _fang(f, *a, **kw):
    try:
        return {"ok": f(*a, **kw)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)}


def _fall(req: dict) -> dict:
    """Ein ganzer Fall: materialisieren, deklarieren, zuruecklesen, Instanzen, XML-Varianten."""
    m = _module()
    EM, EX, ST, bindung = m["EM"], m["EX"], m["ST"], m["bindung"]
    store = req["store"]
    felder, sid = ST.materialisiere(store)
    if req.get("nur_bestaetigt"):
        felder = {k: v for k, v in felder.items() if v.get("zustand") == "bestaetigt"}
    out = {"snapshot_id": sid, "n_felder": len(felder)}
    # vz wie haut/api.py aus dem Fall; fehlt es, wird es 0 und deklariere wirft (null_unzulaessig).
    vz = int(store.get("veranlagungszeitraum") or 0)
    dekl = _fang(EM.deklariere, felder, bindung, vz=vz, snapshot_id=sid)
    out["deklariere"] = dekl
    if "ok" in dekl:
        out["zuruecklesen"] = _fang(EM.zuruecklesen, dekl["ok"], bindung)
        xml = {}
        for name, kw in req.get("xml", {}).items():
            kw = dict(kw)
            if kw.pop("mit_snapshot", False):
                kw["snapshot"] = felder
            xml[name] = _fang(EX.erzeuge_xml, dekl["ok"], **kw)
        out["xml"] = xml
    out["instanzen"] = {g: _fang(EM.instanzen, store, bindung, g) for g in req.get("gruppen", [])}
    return out


def _cent_sweep(req: dict) -> list:
    EM = _module()["EM"]
    return [EM._cent_nach_kz(w, req["kz"]) for w in range(req["von"], req["bis"] + 1, req["schritt"])]


def _cent_liste(req: dict) -> list:
    EM = _module()["EM"]
    return [EM._cent_nach_kz(w, kz) for kz, w in req["paare"]]


def _abzugs_kz(_req: dict) -> list:
    EM = _module()["EM"]
    return sorted(EM._ABZUGS_KZ)


def _kz_wert(req: dict) -> list:
    EM = _module()["EM"]
    return [_fang(EM._kz_wert, w, kz, typ) for w, kz, typ in req["faelle"]]


def _jahr_aus(req: dict) -> list:
    EM = _module()["EM"]
    return [EM._jahr_aus_kz_wert(w, kz) for w, kz in req["faelle"]]


def _parse_instanz(req: dict) -> list:
    EM = _module()["EM"]
    return [EM.parse_instanz(f) for f in req["ids"]]


def _schema(req: dict) -> dict:
    """kz_pfade, pflicht_kinder, kz_meta fuer einen VZ (Schema-Reihenfolge der Pfade erhalten)."""
    m = _module()
    EX, XV = m["EX"], m["XV"]
    vz = req["vz"]
    pfade = EX.kz_pfade(vz)
    pflicht = EX.pflicht_kinder(vz)
    meta = XV._resolve_kz_meta(XV._find_schema(vz))
    return {"pfade": [[k, list(p)] for k, p in pfade.items()],
            "pflicht": sorted([list(p), k] for p, k in pflicht.items()),
            "kz_meta": meta}


def _klassifiziere(req: dict) -> list:
    sys.path.insert(0, os.path.join(ROOT, "elster"))
    import checkest_gate as CE  # noqa: E402  (laedt ERiC erst bei validate)
    return [[CE.klassifiziere_rc(rc), CE.klassifiziere_rc(rc) in CE.NICHT_GEPRUEFT_KLASSEN] for rc in req["rcs"]]


def _pruefe_bindung(_req: dict) -> dict:
    m = _module()
    XV, bindung = m["XV"], m["bindung"]
    return XV.pruefe_bindung({**bindung, **XV.ernte_est_mapping_kz(bindung)})


def _checkest(req: dict) -> list:
    """NUR ERIC_VALIDIERE (checkest_gate.validate) — nie senden."""
    sys.path.insert(0, os.path.join(ROOT, "elster"))
    import checkest_gate as CE  # noqa: E402
    rc, antwort = CE.validate(req["xml"].encode("utf-8"), req["datenart"])
    return [rc, antwort.count("<FehlerRegelpruefung>")]


HANDLER = {
    "fall": _fall,
    "cent_sweep": _cent_sweep,
    "cent_liste": _cent_liste,
    "abzugs_kz": _abzugs_kz,
    "kz_wert": _kz_wert,
    "jahr_aus_kz_wert": _jahr_aus,
    "parse_instanz": _parse_instanz,
    "schema": _schema,
    "klassifiziere_rc": _klassifiziere,
    "pruefe_bindung": _pruefe_bindung,
    "checkest": _checkest,
}


def handle(req: dict) -> dict:
    return _fang(HANDLER[req["fn"][len("elster."):]], req)
