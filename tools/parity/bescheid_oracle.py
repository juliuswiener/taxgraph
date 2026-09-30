"""Paritaets-Orakel fuer `rust/bescheid` (Schritt 7): beantwortet `{"fn": "bescheid.<name>", ...}`
ueber die Python-Referenz `produkt/bescheid/*.py`. Die Funktionen werden DIREKT aufgerufen — kein
Umweg ueber `_bescheid_fn`.

Aufruf:
- ueber `tools/parity/oracle.py` (Praefix `bescheid.`), ein Prozess fuer alle Tests, oder
- eigenstaendig: `python3 tools/parity/bescheid_oracle.py` (JSON-Zeilen stdin → stdout).

Anfragen:
- `{"fn": "bescheid.fall", "funktionen": [<name>, ...] | null, "store": {..} | null, "felder": {..} | null,
   "vz": 2025, "nur_bestaetigt": true, "store_uebergeben": true, "bindung_uebergeben": true, "args": {..}}`
  Ein Kontext, mehrere Funktionen: Antwort `{"ok": {<name>: {"ok": <ergebnis>} | {"err": "<Klasse>"}}}`.
  `store` wird wie in `_bescheid_fn` materialisiert und bei `nur_bestaetigt` auf bestaetigt gefiltert
  (`bescheid_zweige.py:1487`); ohne `store` gilt `felder` (Golden-Faelle). `store_uebergeben=false`
  ruft die Funktionen mit `store=None`, `bindung_uebergeben=false` mit `bindung=None`.
- `{"fn": "bescheid.<name>", ...}`: dieselben Felder, nur diese eine Funktion; Antwort wie oben je Name.
- `{"fn": "bescheid.konstanten"}`: die Tabellen aus `api_constants.py`, die Rust nachbildet.

Erweitern (spaetere Schritte): einen Handler `h(ctx) -> JSON-faehig` schreiben und in `DISPATCH`
eintragen. `ctx` traegt `f` (Snapshot), `store`, `bindung`, `vz`, `nur` (nur_bestaetigt), `args`.

Antwort je Funktion: `{"ok": <ergebnis>}` oder `{"err": "<Ausnahmeklasse>", "msg": "<str(exc)>"}`.
"""
from __future__ import annotations

import json
import os
import sys
from types import SimpleNamespace

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_M: dict = {}


def _module() -> dict:
    if not _M:
        for sub in ("produkt/bescheid", "produkt/store", "produkt/traverser", "produkt/mapping", "produkt/haut"):
            p = os.path.join(ROOT, sub)
            if p not in sys.path:
                sys.path.insert(0, p)
        import api_constants as AC  # noqa: E402
        import bescheid_abzuege as BA  # noqa: E402
        import bescheid_einkuenfte as BE  # noqa: E402
        import store as ST  # noqa: E402
        import traverser as TR  # noqa: E402
        _M.update(AC=AC, BA=BA, BE=BE, ST=ST, bindung=TR.lade_bindung())
    return _M


def _ctx(req: dict) -> SimpleNamespace:
    m = _module()
    store = req.get("store")
    if store is not None:
        felder, _ = m["ST"].materialisiere(store)
    else:
        felder = req.get("felder") or {}
    nur = bool(req.get("nur_bestaetigt", True))
    if nur and felder:  # bescheid_zweige.py:1487
        felder = {fid: ev for fid, ev in felder.items() if ev.get("zustand") == "bestaetigt"}
    return SimpleNamespace(
        f=felder,
        store=store if req.get("store_uebergeben", True) else None,
        bindung=m["bindung"] if req.get("bindung_uebergeben", True) else None,
        vz=req.get("vz"), nur=nur, args=req.get("args") or {})


def _c_aus(f: dict):
    """Der Closure `_c`, den die Zweige an `_p20_kapitaleinkuenfte` reichen (`bescheid_zweige.py:418`)."""
    def _c(fid):
        v = f.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
    return _c


# ------------------------------------------------------------------ Handler (bescheid_abzuege.py)

def _abs3(c):
    return _module()["BA"]._abs3_eligible(c.f, c.vz)


def _oepnv(c):
    return _module()["BA"]._oepnv_eur(c.args["slots"])


def _kind_kv_pv(c):
    return _module()["BA"]._kind_kv_pv_summe(c.store, c.bindung, c.nur)


def _kinderbetreuung(c):
    return _module()["BA"]._kinderbetreuung_summe(c.store, c.bindung, c.nur, c.vz)


def _gate_fehlend(c):
    return sorted(_module()["BA"]._p10_1_5_gate_fehlend(c.store, c.bindung))


def _schulgeld(c):
    return _module()["BA"]._schulgeld_summe(c.store, c.bindung, c.nur, c.vz, c.f)


def _kind_pb_daten(c):
    return _module()["BA"]._kind_behinderten_pb_daten(c.store, c.bindung, c.nur)


def _p33b_kind_pb(c):
    return _module()["BA"]._p33b_kind_pauschbetraege(c.store, c.bindung, c.nur, c.vz)


def _steuer_sonder_agb(c):
    g: dict = {}
    a = c.args
    _module()["BA"]._shared_steuer_sonder_agb(g, a["gde"], a["ausserg"], a["veranlagung"], c.f, c.vz,
                                              c.store, c.bindung, c.nur)
    return {k: g[k] for k in ("steuerermaessigungen", "sonderausgaben", "aussergewoehnliche_belastungen")}


# ------------------------------------------------------------------ Handler (bescheid_einkuenfte.py)

def _gwg(c):
    return _module()["BE"]._gwg_sofortabzug_summe(c.f, c.store, c.bindung, c.nur)


def _laufend_partner(c):
    return list(_module()["BE"]._laufender_gewinn_partner(c.f))


def _laufend(c):
    return list(_module()["BE"]._laufender_gewinn(c.f, c.store, c.bindung, c.nur))


def _gewinn_partner_anteil(c):
    return list(_module()["BE"]._gewinn_partner_anteil(c.f))


def _p20(c):
    return _module()["BE"]._p20_kapitaleinkuenfte(_c_aus(c.f), c.args["zusammen"], c.vz)


def _p23(c):
    return _module()["BE"]._p23_ansonsten_einkuenfte(c.f, c.store, c.bindung, c.nur)


def _p35_partner(c):
    return list(_module()["BE"]._p35_partner_anteile(c.f))


def _p35_gezahlt(c):
    a = c.args
    return _module()["BE"]._p35_gezahlte_gewst(a["messbetrag_a"], a["hebesatz_a"], a["messbetrag_b"], a["hebesatz_b"])


def _p35_summen(c):
    a = c.args
    return list(_module()["BE"]._p35_summen(c.f, a["messbetrag_a"], a["hebesatz_a"], a["zaehler_a"]))


def _dba_sonstige(c):
    g = dict(c.args["g"])
    ret = _module()["BE"]._shared_dba_sonstige(g, c.args["gde_p10d"], c.args["veranlagung"], c.f, c.vz)
    return {"ret": ret, "sonstige_abzuege_vom_einkommen": g["sonstige_abzuege_vom_einkommen"],
            "anzurechnende_auslaendische_steuern": g["anzurechnende_auslaendische_steuern"],
            "p32b_progressionseinkuenfte": g.get("p32b_progressionseinkuenfte")}


def _dba_methode(c):
    return _module()["AC"].dba_methode_fuer(c.args.get("staat"), c.args.get("einkunftsart"))


def _konstanten(_c):
    AC = _module()["AC"]
    return {"kap_ertraege": AC.KAP_ERTRAEGE, "kap_toepfe": list(AC.KAP_TOEPFE),
            "kap_ertraege_partner": AC.KAP_ERTRAEGE_PARTNER, "kap_toepfe_partner": list(AC.KAP_TOEPFE_PARTNER),
            "euer_komponenten": list(AC.EUER_KOMPONENTEN), "gewinn_quellen_mengen": list(AC.GEWINN_QUELLEN_MENGEN),
            "mitu_felder": list(AC.MITU_FELDER),
            "dba_staat_iso": sorted(AC.DBA_STAAT_ISO.items()),
            "dba_methode": sorted(AC.DBA_METHOD_MAP.items()),
            "dba_methode_art": sorted([list(k), v] for k, v in AC.DBA_METHOD_MAP_ART.items())}


DISPATCH = {
    "bescheid.abs3_eligible": _abs3,
    "bescheid.oepnv_eur": _oepnv,
    "bescheid.kind_kv_pv_summe": _kind_kv_pv,
    "bescheid.kinderbetreuung_summe": _kinderbetreuung,
    "bescheid.p10_1_5_gate_fehlend": _gate_fehlend,
    "bescheid.schulgeld_summe": _schulgeld,
    "bescheid.kind_behinderten_pb_daten": _kind_pb_daten,
    "bescheid.p33b_kind_pauschbetraege": _p33b_kind_pb,
    "bescheid.shared_steuer_sonder_agb": _steuer_sonder_agb,
    "bescheid.gwg_sofortabzug_summe": _gwg,
    "bescheid.laufender_gewinn_partner": _laufend_partner,
    "bescheid.laufender_gewinn": _laufend,
    "bescheid.gewinn_partner_anteil": _gewinn_partner_anteil,
    "bescheid.p20_kapitaleinkuenfte": _p20,
    "bescheid.p23_ansonsten_einkuenfte": _p23,
    "bescheid.p35_partner_anteile": _p35_partner,
    "bescheid.p35_gezahlte_gewst": _p35_gezahlt,
    "bescheid.p35_summen": _p35_summen,
    "bescheid.shared_dba_sonstige": _dba_sonstige,
    "bescheid.dba_methode_fuer": _dba_methode,
}


def _fang(h, c):
    try:
        return {"ok": h(c)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)}


def handle(req: dict) -> dict:
    """Antwort auf eine Anfrage; Harness-Fehler (unbekannte Funktion, kaputtes JSON) als `{"err": ..}`."""
    try:
        fn = req["fn"]
        if fn == "bescheid.konstanten":
            return {"ok": _konstanten(None)}
        c = _ctx(req)
        if fn == "bescheid.fall":
            namen = req.get("funktionen") or list(DISPATCH)
            return {"ok": {n: _fang(DISPATCH[n], c) for n in namen}}
        return {"ok": {fn: _fang(DISPATCH[fn], c)}}
    except Exception as exc:  # noqa: BLE001
        return {"err": type(exc).__name__, "msg": str(exc)}


def main() -> None:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        sys.stdout.write(json.dumps(handle(json.loads(line)), ensure_ascii=False) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
