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
- Schritt 7b (`bescheid_deklaration.py`): dieselbe Anfrage mit `"scheibe": "<name>"|null`; `cfg` ist dann
  `api_constants.SCHEIBEN[<name>]` und `bindung` der Scheiben-Ausschnitt wie `api._scheibe_bindung`.
  `nur_bestaetigt` muss `false` sein (der Guard sieht Roh-Felder). `bescheid.scheibe_felder`,
  `bescheid.sperrgrund_literale`, `bescheid.dateien` (welche Python-Dateien geladen sind).

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
        import bescheid_zweige as BZ  # noqa: E402
        import store as ST  # noqa: E402
        import traverser as TR  # noqa: E402
        _M.update(AC=AC, BA=BA, BE=BE, BZ=BZ, ST=ST, TR=TR, bindung=TR.lade_bindung())
    return _M


def _deklaration():
    """`bescheid_deklaration` (zieht runner/est_mapping/...; erst bei Bedarf)."""
    m = _module()
    if "BD" not in m:
        import bescheid_deklaration as BD  # noqa: E402
        m["BD"] = BD
    return m["BD"]


def _scheibe(name):
    """`(cfg, bindung)` einer Scheibe wie `api._scheibe_bindung`; `(None, None)` ohne Name."""
    if not name:
        return None, None
    m = _module()
    cfg = m["AC"].SCHEIBEN[name]
    if cfg["felder"] is not None:
        felder = tuple(cfg["felder"])
    else:
        import yaml
        d = yaml.safe_load(open(os.path.join(ROOT, "produkt", "bindung", cfg["felder_datei"]), encoding="utf-8"))
        felder = tuple(b["feld_id"] for b in d.get("bindungen", []))
    return cfg, {f: m["bindung"][f] for f in felder if f in m["bindung"]}


def _ctx(req: dict) -> SimpleNamespace:
    m = _module()
    cfg, sbindung = _scheibe(req.get("scheibe"))
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
        cfg=cfg, sbindung=sbindung if req.get("bindung_uebergeben", True) else None,
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


# ------------------------------------------------------------------ Handler (bescheid_deklaration.py)

def _projiziere(ev):
    """Snapshot-Sicht eines Events: `wert`, `zustand`, `herkunft` (`schreiber`/`signal` kennt Rust nicht)."""
    return {k: ev.get(k) for k in ("wert", "zustand", "herkunft")}


def _mit_ring_werten(c):
    import copy
    felder = copy.deepcopy(c.f)
    vorher = {k: _projiziere(v) for k, v in felder.items()}
    _deklaration()._mit_ring_werten(felder, c.vz)
    return {k: _projiziere(v) for k, v in felder.items() if vorher.get(k) != _projiziere(v)}


def _sperrgrund_klartext(c):
    return _deklaration().sperrgrund_klartext(c.args.get("grund"))


def _sperrgrund_felder(c):
    return _deklaration().sperrgrund_felder(c.args.get("grund"), c.f)


def _rentenbeginn(c):
    return _deklaration()._rentenbeginn_offen_stand(c.f, c.cfg)


def _vorlaeufig(c):
    return _deklaration()._vorlaeufige_ring_betraege(c.f, c.cfg, c.sbindung)


def _an_gesamt(c):
    vz = None if c.args.get("vz_none") else c.vz
    return _deklaration()._an_gesamt_sperrgrund(c.f, c.cfg, vz, c.store, c.sbindung)
# ------------------------------------------------------------------ Handler (bescheid_zweige.py)

def _kern_werte(x):
    """`extras`/Kette JSON-faehig: dict-Schluessel bleiben, Werte sind int/bool/str/dict."""
    return json.loads(json.dumps(x, ensure_ascii=False))


def _scheibe_zu(m, q: str) -> dict:
    """Die Scheibe, deren `gesamt_ring` die Quantitaet ist (eindeutig, api_constants.SCHEIBEN)."""
    treffer = [c for c in m["AC"].SCHEIBEN.values() if c["gesamt_ring"] == q]
    assert len(treffer) == 1, (q, len(treffer))
    return treffer[0]


def _zweig(req: dict) -> dict:
    """`_bescheid_fn(...)` wie `_feste_zahl` (api.py:220-226) sie aufruft, danach `bf(werte)`.

    Anfrage: `quantitaet`, `store` | `felder`, `vz`, `nur_bestaetigt`, `bindung` ("scheibe" wie /ergebnis,
    "voll" = ganze Tabelle), `werte` ("kegel" wie /ergebnis, "alle" = alle Scheiben-Felder im Snapshot),
    `solz`/`extras` (Out-Parameter uebergeben ja/nein), `store_uebergeben`.
    Antwort: `{"none": true}` (kein Accessor) oder `{"zahl_cent", "solz", "extras", "bindung_ids", "werte_ids"}`
    bzw. `{"err": Klasse, ..}` mit denselben `*_ids` — Rust baut damit GENAU dieselben Eingaben."""
    m = _module()
    q, vz = req["quantitaet"], req["vz"]
    store = req.get("store")
    felder = m["ST"].materialisiere(store)[0] if store is not None else (req.get("felder") or {})
    cfg = _scheibe_zu(m, q) if q in {c["gesamt_ring"] for c in m["AC"].SCHEIBEN.values()} else None
    if cfg is None:
        bf = m["BZ"]._bescheid_fn(q, vz, m["bindung"], felder, store, nur_bestaetigt=bool(req.get("nur_bestaetigt", True)))
        return {"none": bf is None}
    alle = cfg["felder"] if cfg["felder"] is not None else tuple(
        b["feld_id"] for b in __import__("yaml").safe_load(open(os.path.join(ROOT, "produkt", "bindung", cfg["felder_datei"]), encoding="utf-8")).get("bindungen", []))
    kegel = cfg.get("kegel") or alle
    voll = m["bindung"]
    bindung = voll if req.get("bindung") == "voll" else {f: voll[f] for f in alle}
    werte_ids = [f for f in (alle if req.get("werte") == "alle" else kegel) if f in felder]
    solz_out = [None] if req.get("solz", True) else None
    extras = {} if req.get("extras", True) else None
    st = store if req.get("store_uebergeben", True) else None
    bf = m["BZ"]._bescheid_fn(q, vz, bindung, felder, st, nur_bestaetigt=bool(req.get("nur_bestaetigt", True)),
                              solz_container=solz_out, extras=extras)
    kopf = {"bindung_ids": list(bindung), "werte_ids": werte_ids, "quelle": m["BZ"].__file__}
    if bf is None:
        return {"none": True, **kopf}
    try:
        zahl = bf({f: felder[f]["wert"] for f in werte_ids})
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet
        return {"err": type(exc).__name__, "msg": str(exc), **kopf}
    return {"zahl_cent": zahl, "solz": None if solz_out is None else solz_out[0],
            "extras": None if extras is None else _kern_werte(extras), **kopf}


def _abschlusszahlung(req: dict) -> dict:
    """`_abschlusszahlung_cent(felder, zahl_cent)` (bescheid_zweige.py:103): Snapshot UNGEFILTERT, wie api.py:638."""
    m = _module()
    store = req.get("store")
    felder = m["ST"].materialisiere(store)[0] if store is not None else (req.get("felder") or {})
    try:
        return {"ok": m["BZ"]._abschlusszahlung_cent(felder, req["zahl_cent"]), "quelle": m["BZ"].__file__}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet
        return {"err": type(exc).__name__, "msg": str(exc), "quelle": m["BZ"].__file__}


def _scheiben(_req: dict) -> dict:
    """`{quantitaet: {"felder": [...], "kegel": [...]}}` fuer die vier Zweig-Quantitaeten (api_constants.SCHEIBEN)."""
    m = _module()
    out = {}
    for cfg in m["AC"].SCHEIBEN.values():
        q = cfg["gesamt_ring"]
        if q is None:
            continue
        alle = cfg["felder"] if cfg["felder"] is not None else tuple(
            b["feld_id"] for b in __import__("yaml").safe_load(open(os.path.join(ROOT, "produkt", "bindung", cfg["felder_datei"]), encoding="utf-8")).get("bindungen", []))
        out[q] = {"felder": list(alle), "kegel": list(cfg.get("kegel") or alle)}
    return out


def _konstanten(_c):
    AC = _module()["AC"]
    return {"kap_ertraege": AC.KAP_ERTRAEGE, "kap_toepfe": list(AC.KAP_TOEPFE),
            "kap_ertraege_partner": AC.KAP_ERTRAEGE_PARTNER, "kap_toepfe_partner": list(AC.KAP_TOEPFE_PARTNER),
            "euer_komponenten": list(AC.EUER_KOMPONENTEN), "gewinn_quellen_mengen": list(AC.GEWINN_QUELLEN_MENGEN),
            "mitu_felder": list(AC.MITU_FELDER),
            "dba_staat_iso": sorted(AC.DBA_STAAT_ISO.items()),
            "dba_methode": sorted(AC.DBA_METHOD_MAP.items()),
            "dba_methode_art": sorted([list(k), v] for k, v in AC.DBA_METHOD_MAP_ART.items()),
            "deklaration": _deklaration_konstanten()}


def _deklaration_konstanten():
    """Tabellen, die `bescheid_deklaration.py` liest, und die je Scheibe geschnittenen Kandidaten."""
    AC = _module()["AC"]
    namen = ("AGB_KIST", "AN_GESAMT_FLAGS", "AN_GESAMT_PARTNER", "DHF_BEDINGUNGEN", "GESAMT_PARTNER_19",
             "GESAMT_PARTNER_KAP", "RENTNER_22", "RENTNER_22_PARTNER", "RENTNER_AA_ARTEN",
             "UEBERNACHTUNG_BEDINGUNGEN", "VERPFLEGUNG_TAGE", "VERPFLEGUNG_TAGE_NACH_FRIST", "VOR_FELDER",
             "VOR_PARTNER_FELDER", "VV_GESAMT_FELDER")
    tab = {n: list(getattr(AC, n)) for n in namen}
    tab.update(ARBEITSMITTEL_KOSTEN=[AC.ARBEITSMITTEL_KOSTEN], DHF_KOSTEN=[AC.DHF_KOSTEN],
               UEBERNACHTUNG_KOSTEN=[AC.UEBERNACHTUNG_KOSTEN])
    ring, cfgs = {}, {}
    for name, cfg in AC.SCHEIBEN.items():
        kegel, felder = set(cfg.get("kegel") or ()), set(cfg.get("felder") or ())
        ring[name] = [f for f in AC.RING_BETRAGSFELDER if f not in kegel and f in felder]
        cfgs[name] = {"gesamt_guard": bool(cfg.get("gesamt_guard")), "rentner": bool(cfg.get("rentner")),
                      "partner_19": bool(cfg.get("partner_19")), "multi_objekt": cfg.get("multi_objekt"),
                      "multi_rente": cfg.get("multi_rente"), "fremd_arten": list(cfg.get("fremd_arten", ()))}
    return {"tabellen": tab, "ring_kandidaten": ring, "cfg": cfgs}


def _scheibe_felder(req):
    """Die Feld-Ids des Scheiben-Ausschnitts (Rust baut daraus dieselbe Bindung)."""
    _, sb = _scheibe(req.get("scheibe"))
    return sorted(sb or ())


def _sperrgrund_literale(_req):
    """Jedes Sperrgrund-Literal, das Python kennt: Klartext-Schluessel + jede Rueckgabe der Guards (AST)."""
    import ast
    BD = _deklaration()
    baum = ast.parse(open(BD.__file__, encoding="utf-8").read())
    rueck = set()
    for fn in ast.walk(baum):
        if isinstance(fn, ast.FunctionDef) and fn.name in ("_an_gesamt_sperrgrund", "_rentenbeginn_offen_stand"):
            for node in ast.walk(fn):
                if isinstance(node, ast.Return) and isinstance(node.value, ast.Constant) \
                        and isinstance(node.value.value, str):
                    rueck.add(node.value.value)
    return {"klartext_schluessel": sorted(BD.SPERRGRUND_KLARTEXT), "rueckgaben": sorted(rueck)}


def _dateien(_req):
    m = _module()
    BD = _deklaration()
    return {"bescheid_deklaration": BD.__file__, "bescheid_abzuege": m["BA"].__file__,
            "api_constants": m["AC"].__file__, "runner": sys.modules["runner"].__file__ if "runner" in sys.modules else None}


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
    "bescheid.mit_ring_werten": _mit_ring_werten,
    "bescheid.sperrgrund_klartext": _sperrgrund_klartext,
    "bescheid.sperrgrund_felder": _sperrgrund_felder,
    "bescheid.rentenbeginn_offen_stand": _rentenbeginn,
    "bescheid.vorlaeufige_ring_betraege": _vorlaeufig,
    "bescheid.an_gesamt_sperrgrund": _an_gesamt,
}

# Anfragen ohne Fall-Kontext: `{"fn": ..., ...}` -> JSON-faehiges Ergebnis.
OHNE_KONTEXT = {
    "bescheid.scheibe_felder": _scheibe_felder,
    "bescheid.sperrgrund_literale": _sperrgrund_literale,
    "bescheid.dateien": _dateien,
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
        if fn in OHNE_KONTEXT:
            return {"ok": OHNE_KONTEXT[fn](req)}
        c = _ctx(req)
        if fn == "bescheid.abschlusszahlung":
            return {"ok": _abschlusszahlung(req)}
        if fn == "bescheid.scheiben":
            return {"ok": _scheiben(req)}
        if fn == "bescheid.zweig":
            return {"ok": _zweig(req)}
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
