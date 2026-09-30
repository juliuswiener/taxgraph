#!/usr/bin/env python3
"""Parity-Orakel: langlaufender Prozess, JSON-Zeilen auf stdin/stdout, ruft
`produkt/engine/runner.py`s Catala-Aufrufe fuer VZ 2024..2026 (REWRITE_PLAN.md §5).

Ein Request pro Zeile: {"fn": "grundtarif", "zve_cent": <int>, "vz": <2024|2025|2026>}
Eine Antwort pro Zeile: {"ok": true, "cent": <int>} oder {"ok": false, "error": "<str>"}

`cent` ist in beiden Richtungen CENT (nicht Euro) -- direkt vergleichbar mit der
Rust-Seite (`catala-sys::grundtarif` liefert Cent).

Start: python3 tools/parity/oracle.py   (von der Repo-Wurzel; setzt sys.path selbst)
"""

from __future__ import annotations

import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, ROOT)

_CAT = os.path.join(ROOT, "oracle", "gettsim", "_catala")
sys.path.insert(0, os.path.join(_CAT, "rt"))
sys.path.insert(0, _CAT)

from pkg import Einkommensteuertarif as E  # noqa: E402  (Catala-generated)
from pkg import SpendenAbzug as SA  # noqa: E402
from pkg import ZumutbareBelastung as ZB  # noqa: E402
from pkg import AgbAbzug as AG  # noqa: E402
from pkg import Kirchensteuerabzug as KI  # noqa: E402
from pkg import Altersentlastungsbetrag as AE  # noqa: E402
from pkg import Entlastungsbetrag as EB  # noqa: E402
from pkg import Familienleistungsausgleich as FL  # noqa: E402
from pkg import VerbilligteVermietungWk as VV  # noqa: E402
from pkg import KrankenPflegeVorsorge as KP  # noqa: E402
from pkg import Berufsausbildungsaufwendungen as BA  # noqa: E402
from pkg import BetriebsFreibetrag as BF  # noqa: E402
from pkg import EuerGewinn as EG  # noqa: E402
from pkg import Verlustvortrag as VL  # noqa: E402
from pkg import MitunternehmerEinkuenfte as ME  # noqa: E402
from pkg import ErmaessigterDurchschnittssatz as ED  # noqa: E402
from pkg import GwgSofortabzug as GW  # noqa: E402
from catala_runtime import Money, Decimal, Integer, Bool  # noqa: E402
from produkt.store.store import event_id as _store_event_id  # noqa: E402
from produkt.traverser import traverser as _TR  # noqa: E402
from produkt.store import store as _ST  # noqa: E402

VZ_ENUM = {
    2024: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2024, None),
    2025: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2025, None),
    2026: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2026, None),
}


def _m(cent: int) -> Money:
    """Cent (auch negativ) als `Money`, direkt vergleichbar mit `catala-sys` (das nimmt Cent)."""
    cent = int(cent)
    sign = "-" if cent < 0 else ""
    cent = abs(cent)
    return Money(f"{sign}{cent // 100}.{cent % 100:02d}")


def _grundtarif(req: dict) -> int:
    m = Money(f"{int(req['zve_cent']) // 100}.{int(req['zve_cent']) % 100:02d}")
    out = E.grundtarif(E.GrundtarifIn(
        zu_versteuerndes_einkommen_in=m, veranlagungszeitraum_in=VZ_ENUM[int(req["vz"])]))
    return int(out.tarifliche_steuer)


def _splittingtarif(req: dict) -> int:
    m = Money(f"{int(req['zve_cent']) // 100}.{int(req['zve_cent']) % 100:02d}")
    out = E.splittingtarif(E.SplittingtarifIn(
        zu_versteuerndes_einkommen_gemeinsam_in=m, veranlagungszeitraum_in=VZ_ENUM[int(req["vz"])]))
    return int(out.tarifliche_steuer)


def _agb_abzug(req: dict) -> int:
    a = req["args"]
    out = AG.agb_abzug(AG.AgbAbzugIn(
        aussergewoehnliche_belastungen_in=_m(a["aussergewoehnliche_belastungen_cent"]),
        zumutbare_belastung_in=_m(a["zumutbare_belastung_cent"])))
    return int(out.abzug_agb)


def _zumutbare_belastung(req: dict) -> int:
    a = req["args"]
    out = ZB.zumutbare_belastung(ZB.ZumutbareBelastungIn(
        gesamtbetrag_der_einkuenfte_in=_m(a["gesamtbetrag_der_einkuenfte_cent"]),
        anzahl_kinder_in=Integer(int(a["anzahl_kinder"])),
        splitting_in=bool(a["splitting"])))
    return int(out.zumutbare_belastung)


def _kirchensteuerabzug(req: dict) -> int:
    a = req["args"]
    out = KI.kirchensteuerabzug(KI.KirchensteuerabzugIn(
        gezahlte_kirchensteuer_in=_m(a["gezahlte_kirchensteuer_cent"]),
        erstattete_kirchensteuer_in=_m(a["erstattete_kirchensteuer_cent"])))
    return int(out.abziehbare_kirchensteuer)


def _altersentlastungsbetrag(req: dict) -> int:
    a = req["args"]
    out = AE.altersentlastungsbetrag(AE.AltersentlastungsbetragIn(
        arbeitslohn_in=_m(a["arbeitslohn_cent"]),
        positive_andere_einkuenfte_in=_m(a["positive_andere_einkuenfte_cent"]),
        prozentsatz_in=Decimal(str(a["prozentsatz_num"] / a["prozentsatz_den"])),
        hoechstbetrag_in=_m(a["hoechstbetrag_cent"])))
    return int(out.altersentlastungsbetrag)


def _entlastungsbetrag(req: dict) -> int:
    a = req["args"]
    out = EB.entlastungsbetrag(EB.EntlastungsbetragIn(
        alleinstehend_in=Bool(bool(a["alleinstehend"])),
        anzahl_kinder_in=Integer(int(a["anzahl_kinder"])),
        monate_ohne_voraussetzung_in=Integer(int(a["monate_ohne_voraussetzung"]))))
    return int(out.entlastungsbetrag)


def _familienleistungsausgleich(req: dict) -> int:
    a = req["args"]
    out = FL.familienleistungsausgleich(FL.FamilienleistungsausgleichIn(
        est_ohne_freibetraege_in=_m(a["est_ohne_freibetraege_cent"]),
        est_mit_freibetraegen_in=_m(a["est_mit_freibetraegen_cent"]),
        kindergeld_in=_m(a["kindergeld_cent"])))
    return int(out.est_nach_familienausgleich)


def _verbilligte_vermietung_wk(req: dict) -> int:
    a = req["args"]
    out = VV.verbilligte_vermietung_wk(VV.VerbilligteVermietungWkIn(
        werbungskosten_in=_m(a["werbungskosten_cent"]),
        entgelt_quote_prozent_in=Decimal(str(a["entgelt_quote_prozent_num"] / a["entgelt_quote_prozent_den"]))))
    return int(out.abziehbare_werbungskosten)


def _kranken_pflege_vorsorge(req: dict) -> int:
    a = req["args"]
    out = KP.kranken_pflege_vorsorge(KP.KrankenPflegeVorsorgeIn(
        basis_kv_pv_in=_m(a["basis_cent"]),
        weitere_vorsorgeaufwendungen_in=_m(a["weitere_cent"]),
        mit_anspruch_auf_zuschuss_in=Bool(bool(a["mit_zuschuss"]))))
    return int(out.abziehbare_kv_pv_vorsorge)


def _berufsausbildung(req: dict) -> int:
    a = req["args"]
    out = BA.berufsausbildung(BA.BerufsausbildungIn(aufwendungen_in=_m(a["aufwendungen_cent"])))
    return int(out.abziehbare_sonderausgaben)


def _betriebs_freibetrag(req: dict) -> int:
    a = req["args"]
    out = BF.betriebs_freibetrag(BF.BetriebsFreibetragIn(
        veraeusserungsgewinn_in=_m(a["veraeusserungsgewinn_cent"])))
    return int(out.freibetrag)


def _euer_gewinn(req: dict) -> int:
    a = req["args"]
    out = EG.euer_gewinn(EG.EuerGewinnIn(
        betriebseinnahmen_in=_m(a["betriebseinnahmen_cent"]),
        betriebsausgaben_in=_m(a["betriebsausgaben_cent"])))
    return int(out.gewinn)


def _verlustvortrag_abzug(req: dict) -> int:
    a = req["args"]
    out = VL.verlustvortrag_abzug(VL.VerlustvortragAbzugIn(
        gesamtbetrag_einkuenfte_in=_m(a["gesamtbetrag_einkuenfte_cent"]),
        verlustvortrag_bestand_in=_m(a["verlustvortrag_bestand_cent"]),
        zusammenveranlagung_in=Bool(bool(a["zusammenveranlagung"]))))
    return int(out.verlustabzug)


def _mitunternehmer_einkuenfte(req: dict) -> int:
    a = req["args"]
    out = ME.mitunternehmer_einkuenfte(ME.MitunternehmerEinkuenfteIn(
        gewinnanteil_in=_m(a["gewinnanteil_cent"]),
        verguetung_taetigkeit_in=_m(a["verguetung_taetigkeit_cent"]),
        verguetung_darlehen_in=_m(a["verguetung_darlehen_cent"]),
        verguetung_ueberlassung_in=_m(a["verguetung_ueberlassung_cent"])))
    return int(out.einkuenfte_mitunternehmer)


def _gwg_sofortabzug(req: dict) -> int:
    a = req["args"]
    out = GW.gwg_sofortabzug(GW.GwgSofortabzugIn(
        anschaffungskosten_netto_in=_m(a["anschaffungskosten_netto_cent"])))
    return int(out.sofortabzug)


def _ermaessigter_durchschnittssatz(req: dict) -> int:
    a = req["args"]
    out = ED.ermaessigter_durchschnittssatz(ED.ErmaessigterDurchschnittssatzIn(
        ao_einkuenfte_in=_m(a["ao_einkuenfte_cent"]),
        est_gesamt_zzgl_progression_in=_m(a["est_gesamt_zzgl_progression_cent"]),
        bemessungsgrundlage_durchschnitt_in=_m(a["bemessungsgrundlage_durchschnitt_cent"])))
    return int(out.est_ao)


def _spenden_abzug(req: dict) -> int:
    a = req["args"]
    out = SA.spenden_abzug(SA.SpendenAbzugIn(
        zuwendungen_in=_m(a["zuwendungen_cent"]),
        gesamtbetrag_der_einkuenfte_in=_m(a["gesamtbetrag_der_einkuenfte_cent"])))
    return int(out.spenden_abzug)


def _event_id(req: dict) -> str:
    """`produkt/store/store.py::event_id` fuer `rust/store::EventId::von_json`-Paritaet
    (Deliverable #7, `store_paritaet.rs`): `event` ist ein beliebiges JSON-Objekt."""
    return _store_event_id(req["event"])


def _fehlerklasse(msg: str) -> str:
    """Bildet eine `append_event`-`ValueError`-Nachricht auf den Namen der zugehoerigen
    `rust::store::Abweisung`-Variante ab (`store.py:262-390`) -- der Wortlaut ist bewusst KEIN
    Paritaet-Kriterium (s. `abweisung.rs`-Moduldoku: die deutschen Fehlertexte unterscheiden sich),
    die ausgeloeste REGEL schon. Reihenfolge ist wichtig: die spezifischeren Teilstrings (Ersetzt-
    Guard, Katalog-Fehlt) muessen vor ihrem jeweiligen Praefix-Fall (`fail-closed (A):`/`(Katalog):`)
    geprueft werden."""
    if "darf kein ersetzt tragen" in msg:
        return "AuflageAErsetztGuard"
    if msg.startswith("fail-closed (A):"):
        return "AuflageA"
    if "braucht katalog=" in msg:
        return "KatalogFehlt"
    if msg.startswith("fail-closed (Katalog):"):
        return "KatalogNichtFreigegeben"
    if msg.startswith("fail-closed (F2/Magnitude):"):
        return "Magnitude"
    if msg.startswith("fail-closed (Typ):"):
        return "TypInkonform"
    if msg.startswith("fail-closed (Format):"):
        return "FormatInkonform"
    if "braucht ein signal_2" in msg:
        return "ZweiSignalFehlend"
    if "hat schon ein aktives Event" in msg:
        return "AktivesEventVorhanden"
    if "existiert nicht." in msg:
        return "ErsetztZielUnbekannt"
    if "gehört zu anderem feld_id." in msg:
        return "ErsetztFeldMismatch"
    if "ist bereits ersetzt." in msg:
        return "ErsetztBereitsErsetzt"
    return "Sonstig"


def _append_sequence(req: dict) -> dict:
    """`produkt/store/store.py::append_event` als Aufruf-Sequenz gegen EINEN Store (Nachtrag zu
    Deliverable #2, Verhaltens-Paritaet `Store::append`): `store` ist eine Store-Datei (leer oder
    eine reale Fall-Datei), `calls` eine Liste roher `append_event`-kwargs. Bindung/Katalog werden
    GENAU EINMAL geladen -- derselbe dev-2-Kontrakt wie `api.py` (Z. 516-533 fuer `/event`, Z.
    1170-1180 fuer `/chat`): globale `TR.lade_bindung()`, `katalog` nur fuer Vorschlags-Schreiber
    (`llm:`/`berechnet:`/`import:beleg`/`import:kontoauszug`). Ein abgewiesener Aufruf laesst den
    Store unveraendert (store.py haengt nur bei Erfolg an) -- Rueckgabe zeigt PRO Aufruf entweder
    den neuen `event_id` oder eine Fehlerklasse (s. `_fehlerklasse`)."""
    store = req["store"]
    bindung = _TR.lade_bindung()
    katalog = _ST.lade_katalog(bindung)
    ergebnisse = []
    for call in req["calls"]:
        schreiber = call.get("schreiber") or ""
        vorschlag = schreiber.startswith(("llm:", "berechnet:", "import:beleg", "import:kontoauszug"))
        try:
            ev = _ST.append_event(
                store, feld_id=call["feld_id"], wert=call.get("wert"), zustand=call["zustand"],
                herkunft=call.get("herkunft") or {}, schreiber=schreiber, signal=call.get("signal"),
                ersetzt=call.get("ersetzt"), ts=call.get("ts"),
                katalog=(katalog if vorschlag else None), bindung=bindung)
            ergebnisse.append({"event_id": ev["event_id"]})
        except ValueError as exc:
            ergebnisse.append({"err": _fehlerklasse(str(exc))})
        except Exception as exc:  # noqa: BLE001 -- Paritaet zeigt jeden Fehlertyp, nicht nur ValueError
            ergebnisse.append({"err": type(exc).__name__})
    aktiv = _ST._aktives(store)
    return {"results": ergebnisse, "aktive_event_ids": sorted(e["event_id"] for e in aktiv.values())}


def _runner(req: dict) -> dict:
    """Generischer Accessor-Aufruf `runner.<name>` (Schritt 4b): ruft
    `getattr(produkt.engine.runner, name)(*args, **kwargs)` mit den rohen Dict-Argumenten, wie das
    Produkt sie uebergibt, und antwortet `{"ok": ergebnis}` oder `{"err": "<Ausnahmeklasse>"}` --
    dieselbe Form wie `rust/fixtures/corpus/runner/*.jsonl`. `catala` markiert Catala-
    Laufzeitausnahmen (`CatalaError`-Unterklassen), die der Rust-C-Shim nicht einzeln unterscheidet."""
    from produkt.engine import runner as _RUNNER  # lazy: nur wer runner.* ruft, zahlt den Import
    from catala_runtime import CatalaError
    name = req["fn"][len("runner."):]
    try:
        return {"ok": getattr(_RUNNER, name)(*req.get("args", []), **req.get("kwargs", {}))}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "catala": isinstance(exc, CatalaError)}


_VOLL_SORTIERT: dict | None = None


def _voll_sortiert() -> dict:
    """`lade_bindung()` in der Reihenfolge der Rust-Registry (Dateien alphabetisch, in der Datei
    wie geschrieben). Python selbst liest in `glob`-Reihenfolge (Dateisystem); eine Produktions-
    Lesestelle bekommt ohnehin eine Scheiben-Bindung in Scheiben-Reihenfolge (`api._scheibe_bindung`)."""
    global _VOLL_SORTIERT
    if _VOLL_SORTIERT is None:
        import glob
        import yaml
        voll = _TR.lade_bindung()
        reihe = []
        for f in sorted(glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml"))):
            d = yaml.safe_load(open(f, encoding="utf-8")) or {}
            reihe += [b["feld_id"] for b in d.get("bindungen", [])]
        _VOLL_SORTIERT = {f: voll[f] for f in reihe}
    return _VOLL_SORTIERT


def _interview(req: dict) -> dict:
    """Schritt 5a, Crate `interview`: `traverser.<fn>` / `bindung_rollen.<fn>`. `felder` ist die
    Teil-Bindung als geordnete feld_id-Liste (`None` = alle, s. `_voll_sortiert`) -- dieselbe Form
    wie `api._scheibe_bindung`. Antwort `{"ok": <JSON>}` oder `{"err": "<Klasse>: <Text>"}`."""
    name = req["fn"]
    store = req.get("store")
    a = req.get("args") or {}
    voll = _voll_sortiert()
    felder = req.get("felder")
    sicht = voll if felder is None else {f: _TR.lade_bindung()[f] for f in felder}
    try:
        if name == "traverser.relevanz":
            return {"ok": _TR.relevanz(store, sicht)}
        if name == "traverser.naechste_fragen":
            return {"ok": _TR.naechste_fragen(store, sicht, a.get("beitrag"))}
        if name == "traverser.praefix_fragen":
            evs = store.get("events", [])
            return {"ok": [_TR.naechste_fragen(dict(store, events=evs[:k]), sicht)
                           for k in range(len(evs) + 1)]}
        if name == "traverser.gate_gewicht":
            return {"ok": _TR.gate_gewicht(sicht)}
        if name == "traverser.justification":
            return {"ok": [_TR.justification(store, f, sicht) for f in a["feld_ids"]]}
        if name == "traverser.trace_ergebnis":
            return {"ok": _TR.trace_ergebnis(store, sicht, snapshot_id=a.get("snapshot_id"))}
        if name == "traverser.instanz_anzahl":
            return {"ok": [list(_TR.instanz_anzahl(store, sicht, f)) for f in a["feld_ids"]]}
        if name == "traverser.instanz_feld_id":
            return {"ok": [_TR.instanz_feld_id(b, i) for b, i in a["paare"]]}
        if name == "traverser.fehlende_instanzen":
            return {"ok": _TR.fehlende_instanzen(_ST.materialisiere(store)[0], sicht)}
        if name == "traverser.lade_bindung":
            return {"ok": list(_TR.lade_bindung())}
        if name == "traverser.lade_regel_bedingungen":
            return {"ok": _TR.lade_regel_bedingungen()}
        if name == "traverser.lade_themen_zuerst":
            return {"ok": _TR.lade_themen_zuerst()}
        if name == "traverser.lade_instanz_gruppen":
            return {"ok": _TR.lade_instanz_gruppen()}
        if name == "traverser.scheiben":
            # Eingabe-Beschaffung, kein Vergleich: Scheiben-Felder/Kegel wie `api._scheibe_felder`.
            import yaml
            sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))
            import api_constants as _AC
            out = {}
            for sch, cfg in _AC.SCHEIBEN.items():
                fs = cfg["felder"]
                if fs is None:
                    d = yaml.safe_load(open(os.path.join(ROOT, "produkt", "bindung", cfg["felder_datei"]),
                                            encoding="utf-8"))
                    fs = tuple(b["feld_id"] for b in d.get("bindungen", []))
                out[sch] = {"felder": list(fs), "kegel": None if cfg.get("kegel") is None else list(cfg["kegel"])}
            return {"ok": out}
        if name.startswith("bindung_rollen."):
            sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))
            sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))
            import bindung_rollen as _BR
            cfg = {"kegel": None if a.get("kegel") is None else tuple(a["kegel"])}
            if name == "bindung_rollen.relevante_kegel_felder":
                return {"ok": list(_BR.relevante_kegel_felder(cfg["kegel"] or (), sicht, store))}
            if name == "bindung_rollen.ring_bindung":
                return {"ok": list(_BR.ring_bindung(cfg, sicht, store))}
            if name == "bindung_rollen.rollen":
                aufbau, achsen = _BR.rollen(cfg, sicht, store)
                return {"ok": {"aufbau": list(aufbau), "achsen": list(achsen)}}
        return {"err": f"KeyError: unbekannte Funktion {name}"}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": f"{type(exc).__name__}: {exc}"}


DISPATCH = {
    "grundtarif": _grundtarif,
    "splittingtarif": _splittingtarif,
    "store.event_id": _event_id,
    "store.append_sequence": _append_sequence,
    "agb_abzug": _agb_abzug,
    "zumutbare_belastung": _zumutbare_belastung,
    "kirchensteuerabzug": _kirchensteuerabzug,
    "altersentlastungsbetrag": _altersentlastungsbetrag,
    "entlastungsbetrag": _entlastungsbetrag,
    "familienleistungsausgleich": _familienleistungsausgleich,
    "verbilligte_vermietung_wk": _verbilligte_vermietung_wk,
    "kranken_pflege_vorsorge": _kranken_pflege_vorsorge,
    "berufsausbildung": _berufsausbildung,
    "betriebs_freibetrag": _betriebs_freibetrag,
    "euer_gewinn": _euer_gewinn,
    "verlustvortrag_abzug": _verlustvortrag_abzug,
    "mitunternehmer_einkuenfte": _mitunternehmer_einkuenfte,
    "gwg_sofortabzug": _gwg_sofortabzug,
    "ermaessigter_durchschnittssatz": _ermaessigter_durchschnittssatz,
    "spenden_abzug": _spenden_abzug,
}


def main() -> None:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        req = json.loads(line)
        if str(req.get("fn", "")).startswith("runner."):
            sys.stdout.write(json.dumps(_runner(req)) + "\n")
            sys.stdout.flush()
            continue
        if str(req.get("fn", "")).startswith(("traverser.", "bindung_rollen.")):
            sys.stdout.write(json.dumps(_interview(req), ensure_ascii=False) + "\n")
            sys.stdout.flush()
            continue
        if str(req.get("fn", "")).startswith(("konsistenz.", "intervall.")):
            # lazy: die bare-Modulnamen aus produkt/konsistenz|unsicherheit nur laden, wer sie ruft
            sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
            import oracle_konsistenz  # noqa: E402
            sys.stdout.write(json.dumps(oracle_konsistenz.antwort(req), ensure_ascii=False) + "\n")
            sys.stdout.flush()
            continue
        if str(req.get("fn", "")).startswith("elster."):
            # lazy wie oben: tools/parity/elster_oracle.py (rust/elster, Schritt 6)
            sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
            import elster_oracle  # noqa: E402
            sys.stdout.write(json.dumps(elster_oracle.handle(req), ensure_ascii=False) + "\n")
            sys.stdout.flush()
            continue
        if str(req.get("fn", "")).startswith("bescheid."):
            # lazy wie oben: tools/parity/bescheid_oracle.py (rust/bescheid, Schritt 7)
            sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
            import bescheid_oracle  # noqa: E402
            sys.stdout.write(json.dumps(bescheid_oracle.handle(req), ensure_ascii=False) + "\n")
            sys.stdout.flush()
            continue
        if str(req.get("fn", "")).startswith("schritt8."):
            # lazy wie oben: tools/parity/schritt8_oracle.py (rust/auth, rust/llm, rust/eingang)
            sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
            import schritt8_oracle  # noqa: E402
            sys.stdout.write(json.dumps(schritt8_oracle.handle(req), ensure_ascii=False) + "\n")
            sys.stdout.flush()
            continue
        try:
            fn = DISPATCH[req["fn"]]
            ergebnis = fn(req)
            # `cent`-Funktionen liefern `int`, `store.event_id` liefert `str`, `store.append_sequence`
            # liefert ein `dict` -- Antwortform je Rueckgabetyp, damit bestehende `cent`-Aufrufer
            # unveraendert bleiben.
            resp = {"ok": True, "result": ergebnis} if isinstance(ergebnis, (str, dict)) else {
                "ok": True, "cent": ergebnis}
        except Exception as exc:  # noqa: BLE001 -- an Fehler weiterreichen, nicht sterben
            resp = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
