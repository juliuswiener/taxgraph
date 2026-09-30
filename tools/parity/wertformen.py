#!/usr/bin/env python3
"""K0-Messung (REWRITE_PLAN.md §7, 9b-B): welche JSON-Formen haben die Event-`wert`e wirklich?

Zaehlt die JSON-Form jedes Event-`wert` in allen realen Fall-Stores und jedes Eingabewerts
(`sachverhalt`) der Golden-Faelle, gekreuzt mit dem Bindungstyp (`typ` in
produkt/bindung/bindung_*.yaml) des Feldes. Grundlage fuer `domain::PyWert` und `Lage<T>` (K1).

Geladen wird auf den Wegen des Produkts: Store per `json.load` wie `api.lade_fall`; aktive
Events per `store._aktives` (dieselbe Faltung wie `store.materialisiere`, je Store
gegengeprueft); Bindung per `traverser.lade_bindung`; Instanz-ID `basis__n` per
`est_mapping.parse_instanz` wie Auflage T; Typ-Konformitaet per `store._typ_konform`;
Golden per `yamlstrict.load_str`.

DATENSCHUTZ: die Stores tragen echte Steuerdaten. Ausgegeben werden NUR Zaehlwerte, feld_ids
und Bindungstypen -- nie Werte, Fall-IDs oder Dateinamen. Gelesen wird nur, nie geschrieben.

Run:  python3 tools/parity/wertformen.py [--faelle DIR] [--golden DIR]
      python3 tools/parity/wertformen.py --selbsttest     (synthetischer Store, keine Echtdaten)
Exit: 0 gemessen · 2 null Stores/Events/Golden-Werte gelesen · 3 Store nicht ladbar oder
      Faltung weicht von materialisiere ab
"""
from __future__ import annotations

import argparse
import collections
import glob
import json
import math
import os
import re
import sys
import tempfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
for _d in ("produkt/mapping", "produkt/traverser", "produkt/store", ""):
    sys.path.insert(0, os.path.join(ROOT, _d))
sys.path.append(os.path.join(ROOT, "produkt", "haut"))

import api_constants  # noqa: E402  (nur FAELLE: TAXGRAPH_DATEN / XDG, reine Konstanten)
import est_mapping as EM  # noqa: E402
import store as ST  # noqa: E402
import traverser as TR  # noqa: E402
import yamlstrict  # noqa: E402

I64_MIN, I64_MAX, U64_MAX = -(2**63), 2**63 - 1, 2**64 - 1
FORMEN = {  # Schluessel -> Zeilenbeschriftung, in Tabellenreihenfolge
    "null": "null", "bool": "bool", "int_i64": "int in i64", "int_u64": "int in (i64, u64]",
    "int_gross": "int > u64 oder < i64::MIN", "float": "float endlich", "nan_inf": "NaN/Infinity",
    "text": "string", "liste": "Liste", "objekt": "Objekt", "fehlt": "(kein wert-Schluessel)",
}
SPALTEN = ST._TYP_ORD + ("ohne Bindung",)
_FEHLT = object()
# XML 1.0 Char-Produktion: erlaubt #x9 #xA #xD [#x20-#xD7FF] [#xE000-#xFFFD] [#x10000-#x10FFFF].
XML_UNGUELTIG = re.compile("[\x00-\x08\x0b\x0c\x0e-\x1f\ud800-\udfff\ufffe\uffff]")
ZEICHEN = {"XML-1.0-unzulaessig": XML_UNGUELTIG, "\\t": re.compile("\t"),
           "\\n": re.compile("\n"), "\\r": re.compile("\r")}
# "Zahl-string": was ein nachsichtiger Leser als Zahl nehmen koennte (Vorzeichen, Ziffern mit . oder ,).
ZAHL_TEXT = re.compile(r"^\s*[+-]?\d[\d.,]*\s*(?:€|EUR)?\s*$")
FELD_ID = re.compile(r"^[a-z][a-z0-9_]{0,79}$")   # store/schema.json; alles andere wird maskiert
PFLICHT = ("bool auf cent", "bool auf int", "int in i64 auf bool", "float endlich auf cent",
           "float endlich auf int", "Zahl-string auf cent", "Zahl-string auf int",
           "string ausserhalb enum_werte", "Liste (jeder Typ)", "Objekt (jeder Typ)")


def form(wert) -> str:
    if wert is _FEHLT:
        return "fehlt"
    if wert is None:
        return "null"
    if isinstance(wert, bool):           # vor int: bool ist in Python ein int
        return "bool"
    if isinstance(wert, int):
        if I64_MIN <= wert <= I64_MAX:
            return "int_i64"
        return "int_u64" if I64_MAX < wert <= U64_MAX else "int_gross"
    if isinstance(wert, float):          # json.load: NaN/Infinity-Token und Ueberlauf wie 1e999
        return "float" if math.isfinite(wert) else "nan_inf"
    if isinstance(wert, str):
        return "text"
    return "liste" if isinstance(wert, list) else "objekt"


def basis_id(feld_id) -> str | None:
    """Wie store._pruefe_typ_konformitaet: `basis__n` -> `basis`. None fuer Nicht-Schema-IDs."""
    if not isinstance(feld_id, str) or not FELD_ID.match(feld_id):
        return None
    teile = EM.parse_instanz(feld_id)
    return teile[0] if teile else feld_id


def abweichung(wert, fm: str, eintrag: dict | None) -> str | None:
    """Etikett, wenn der Wert nicht zum Bindungstyp passt, sonst None. Massstab: Auflage T
    (`store._typ_konform`) und Auflage F (`muster`), feiner aufgeteilt. Zusaetzlich markiert:
    Ganzzahl ausserhalb i64 auf cent/int -- Python-konform, fuer `Cent(i64)` nicht."""
    if fm in ("liste", "objekt"):
        return f"{FORMEN[fm]} (jeder Typ)"
    if eintrag is None:
        return None
    typ = eintrag.get("typ")
    if fm == "fehlt":
        return f"kein wert auf {typ}"
    if typ in ("cent", "int") and fm in ("int_u64", "int_gross"):
        return f"{FORMEN[fm]} auf {typ} (Python-konform)"
    if ST._typ_konform(wert, typ, eintrag.get("enum_werte")):
        muster = eintrag.get("muster")
        if muster and isinstance(wert, str) and not re.match(muster, wert):
            return f"string verletzt muster auf {typ}"
        return None
    if fm == "text":
        if typ in ("cent", "int"):
            return f"{'Zahl' if ZAHL_TEXT.match(wert) else 'Nicht-Zahl'}-string auf {typ}"
        if typ == "enum":
            return "string ausserhalb enum_werte"
        if typ == "datum":
            return "string nicht TT.MM.JJJJ auf datum"
    return f"{FORMEN[fm]} auf {typ}"


def strings(wert):
    """Alle str im Wert, auch in Listen/Objekten (Schluessel eingeschlossen)."""
    if isinstance(wert, str):
        yield wert
    elif isinstance(wert, list):
        for w in wert:
            yield from strings(w)
    elif isinstance(wert, dict):
        for k, w in wert.items():
            yield from strings(k)
            yield from strings(w)


class Zaehler:
    """Eine Sicht (alle Events / aktive Events / Golden). Quelle = Store- bzw. Fall-Index."""

    def __init__(self):
        self.zellen = collections.Counter()           # (zustand, form, spalte) -> Werte
        self.n = collections.Counter()                # Etikett -> Werte
        self.quellen = collections.defaultdict(set)   # Etikett -> Quell-Indizes
        self.fids = collections.defaultdict(collections.Counter)  # Etikett -> Basis-feld_id -> Werte
        self.instanz = collections.Counter()

    def _merke(self, schluessel: str, quelle: int, fid: str) -> None:
        self.n[schluessel] += 1
        self.quellen[schluessel].add(quelle)
        self.fids[schluessel][fid] += 1

    def zaehle(self, quelle: int, feld_id, wert, zustand, bindung: dict) -> None:
        basis = basis_id(feld_id)
        eintrag = bindung.get(basis) if basis else None
        fid = basis or "<feld_id ausserhalb Schema>"
        fm = form(wert)
        spalte = eintrag.get("typ", "Bindung ohne typ") if eintrag else "ohne Bindung"
        self.zellen[(str(zustand), fm, spalte)] += 1
        if eintrag is None:
            self._merke("ohne Bindung", quelle, fid)
        if basis and basis != feld_id:
            self.instanz["Instanz-ID basis__n"] += 1
            self.instanz["davon basis__1"] += feld_id.endswith("__1")
        etikett = abweichung(wert, fm, eintrag)
        if etikett:
            self._merke(etikett, quelle, fid)
        for name, muster in ZEICHEN.items():
            if any(muster.search(s) for s in strings(wert)):
                self._merke(name, quelle, fid)


def messe(faelle: str, golden: str, bindung: dict) -> dict:
    m = {"alle": Zaehler(), "aktiv": Zaehler(), "golden": Zaehler(), "stores": 0, "events": 0,
         "aktive_felder": 0, "golden_faelle": 0, "golden_werte": 0,
         "ladefehler": collections.Counter(), "faltung": collections.Counter()}
    for i, pfad in enumerate(sorted(glob.glob(os.path.join(faelle, "*.json")))):
        try:
            with open(pfad, encoding="utf-8") as f:   # wie produkt/haut/api.py:lade_fall
                store = json.load(f)
        except (OSError, ValueError) as e:
            m["ladefehler"][type(e).__name__] += 1
            continue
        events = store.get("events") if isinstance(store, dict) else None
        if not isinstance(events, list):
            m["ladefehler"]["ohne events-Liste"] += 1
            continue
        m["stores"] += 1
        m["events"] += len(events)
        for e in events:
            m["alle"].zaehle(i, e.get("feld_id"), e.get("wert", _FEHLT), e.get("zustand"), bindung)
        aktiv = ST._aktives(store)
        m["aktive_felder"] += len(aktiv)
        for fid, e in aktiv.items():
            m["aktiv"].zaehle(i, fid, e.get("wert", _FEHLT), e.get("zustand"), bindung)
        try:   # Gegenprobe: dieselbe Menge landet im Snapshot (SnapshotFeld.wert)
            felder, _ = ST.materialisiere(store)
            gleich = ({f: (v["wert"], v["zustand"]) for f, v in felder.items()}
                      == {f: (e["wert"], e["zustand"]) for f, e in aktiv.items()})
            m["faltung"]["gleich" if gleich else "abweichend"] += 1
        except Exception as e:  # noqa: BLE001 -- z. B. UnicodeEncodeError im snapshot_id-Hash
            m["faltung"][f"materialisiere wirft {type(e).__name__}"] += 1
    for i, pfad in enumerate(sorted(glob.glob(os.path.join(golden, "*.yaml")))):
        with open(pfad, encoding="utf-8") as fh:
            fall = yamlstrict.load_str(fh.read(), herkunft=pfad)
        m["golden_faelle"] += 1
        for k, v in (fall.get("sachverhalt") or {}).items():
            m["golden_werte"] += 1
            m["golden"].zaehle(i, k, v, "golden", bindung)
    return m


def drucke_tabelle(z: Zaehler, zustaende, titel: str) -> None:
    extra = sorted({s for (_, _, s) in z.zellen} - set(SPALTEN))
    spalten = list(SPALTEN) + extra
    print(f"\n#### {titel}\n")
    print("| Form | " + " | ".join(spalten) + " | Summe |")
    print("|---|" + "---:|" * (len(spalten) + 1))
    summen = [0] * len(spalten)
    for fm, label in FORMEN.items():
        werte = [sum(z.zellen[(zs, fm, s)] for zs in zustaende) for s in spalten]
        summen = [a + b for a, b in zip(summen, werte)]
        print(f"| {label} | " + " | ".join(map(str, werte)) + f" | {sum(werte)} |")
    print("| **Summe** | " + " | ".join(map(str, summen)) + f" | {sum(summen)} |")


def drucke_etiketten(z: Zaehler, etiketten, titel: str, quelle: str) -> None:
    print(f"\n#### {titel}\n")
    print(f"| Etikett | Werte | {quelle} | feld_ids (Basis-ID, max. 5, nach Haeufigkeit) |")
    print("|---|---:|---:|---|")
    for e in etiketten:
        fids = ", ".join(f"`{f}` ({n})" for f, n in z.fids[e].most_common(5)) if e in z.fids else "—"
        print(f"| {e} | {z.n[e]} | {len(z.quellen.get(e, ()))} | {fids} |")


def drucke_sicht(z: Zaehler, name: str, quelle: str) -> None:
    zustaende = sorted({zs for (zs, _, _) in z.zellen})
    drucke_tabelle(z, zustaende, f"{name}: Form × Bindungstyp, alle Zustaende")
    if len(zustaende) > 1:
        for zs in zustaende:
            drucke_tabelle(z, [zs], f"{name}: Form × Bindungstyp, zustand={zs}")
    abw = sorted(set(PFLICHT) | {e for e in z.n if e not in ZEICHEN and e != "ohne Bindung"})
    drucke_etiketten(z, abw, f"{name}: Abweichungen vom Bindungstyp", quelle)
    drucke_etiketten(z, list(ZEICHEN), f"{name}: Steuer- und Sonderzeichen in string-Werten", quelle)
    print(f"\n{name}: ohne Bindung {z.n['ohne Bindung']} Werte, {len(z.fids['ohne Bindung'])} "
          f"Basis-feld_ids in {len(z.quellen.get('ohne Bindung', ()))} {quelle}; "
          + ", ".join(f"`{f}` ({n})" for f, n in z.fids["ohne Bindung"].most_common(20)))
    print(f"\n{name}: {dict(z.instanz) or 'keine Instanz-IDs'}")


def main(argv=None) -> int:
    bindung = TR.lade_bindung()
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--faelle", default=api_constants.FAELLE, help="Verzeichnis mit <fall_id>.json")
    ap.add_argument("--golden", default=os.path.join(ROOT, "golden", "cases"))
    ap.add_argument("--selbsttest", action="store_true")
    args = ap.parse_args(argv)
    if args.selbsttest:
        return selbsttest(bindung)
    print("Module: " + ", ".join(f"{m.__name__}={m.__file__}"
                                 for m in (ST, TR, EM, yamlstrict, api_constants)))
    m = messe(args.faelle, args.golden, bindung)
    print(f"Fall-Verzeichnis: {args.faelle} — {m['stores']} Stores, {m['events']} Events, "
          f"{m['aktive_felder']} aktive Felder; Ladefehler: {dict(m['ladefehler']) or 0}; "
          f"Faltung _aktives vs. materialisiere: {dict(m['faltung'])}")
    print(f"Golden: {args.golden} — {m['golden_faelle']} Faelle, {m['golden_werte']} Werte; "
          f"Bindung: {len(bindung)} feld_ids")
    if not (m["stores"] and m["events"] and m["golden_faelle"] and m["golden_werte"]):
        print("FEHLER: null Stores, Events oder Golden-Werte gelesen — das ist kein Messergebnis.")
        return 2
    drucke_sicht(m["alle"], "ALLE Events", "Stores")
    drucke_sicht(m["aktiv"], "AKTIVE Events", "Stores")
    drucke_sicht(m["golden"], "GOLDEN sachverhalt", "Faelle")
    return 3 if m["ladefehler"] or set(m["faltung"]) - {"gleich"} else 0


def kalibrier_events(bindung: dict) -> list[dict]:
    """Je Zielzelle EIN Event mit eigener feld_id (vorlaeufig, ersetzt nichts, also aktiv).
    Auch fuer die Kalibrierung an einer Store-KOPIE: an deren `events` anhaengen."""
    je_typ = collections.defaultdict(list)
    for fid in sorted(bindung):
        je_typ[bindung[fid].get("typ")].append(fid)
    enum_fid = je_typ["enum"][0]
    faelle = [("text", "a\x01b"), ("text", "a\ud800b"), ("text", "a\ufffeb"), ("text", "a\tb"),
              ("text", "a\nb"), ("text", "a\rb"), (None, 2**63), (None, 2**64), (None, -(2**63) - 1),
              (None, float("nan")), (None, float("inf")), (None, [1, 2]), (None, {"a": 1}),
              ("cent", True), ("int", False), ("bool", 1), ("cent", 12.5), ("int", 2.0),
              ("cent", "50000"), ("int", "12,5"), ("enum", "kalibrierung_kein_enum_wert"),
              ("cent", 2**64)]
    events, zaehler = [], collections.Counter()
    for n, (typ, wert) in enumerate(faelle):
        if typ is None:
            fid = f"kalibrierung_ohne_bindung_{n}"
        elif typ == "enum":
            fid = enum_fid
        else:
            fid = je_typ[typ][zaehler[typ]]
            zaehler[typ] += 1
        events.append({"event_id": f"{n:064x}", "ts": "2026-10-01T00:00:00+00:00", "feld_id": fid,
                       "wert": wert, "zustand": "vorlaeufig", "schreiber": "kalibrierung",
                       "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                    "haftung": "nutzer"}, "ersetzt": None})
    return events


# Erwartung je Sicht (alle = aktiv, denn kein Kalibrier-Event wird ersetzt): Form -> Anzahl,
# Etikett -> Anzahl. Passt zu kalibrier_events; aendert sich das eine, bricht --selbsttest.
ERWARTET_FORMEN = {"text": 9, "int_u64": 1, "int_gross": 3, "nan_inf": 2, "liste": 1, "objekt": 1,
                   "bool": 2, "int_i64": 1, "float": 2}
ERWARTET_ETIKETTEN = {"XML-1.0-unzulaessig": 3, "\\t": 1, "\\n": 1, "\\r": 1, "bool auf cent": 1,
                      "bool auf int": 1, "int in i64 auf bool": 1, "float endlich auf cent": 1,
                      "float endlich auf int": 1, "Zahl-string auf cent": 1, "Zahl-string auf int": 1,
                      "string ausserhalb enum_werte": 1, "Liste (jeder Typ)": 1,
                      "Objekt (jeder Typ)": 1, "int > u64 oder < i64::MIN auf cent (Python-konform)": 1}


def zellen_je_form(z: Zaehler) -> collections.Counter:
    c = collections.Counter()
    for (_, fm, _), n in z.zellen.items():
        c[fm] += n
    return c


def selbsttest(bindung: dict) -> int:
    """Kalibrierung ohne Echtdaten: synthetischer Store, je Zielzelle ein Wert -> Zaehlung == 1.
    Dazu: leeres Verzeichnis und Store ohne Events muessen Exit 2 liefern."""
    golden = os.path.join(ROOT, "golden", "cases")
    with tempfile.TemporaryDirectory() as d:
        with open(os.path.join(d, "kalibrierung.json"), "w", encoding="utf-8") as f:
            # ensure_ascii (Standard): das einzelne Surrogat geht als \ud800 ins JSON, NaN als NaN
            json.dump({"version": 1, "veranlagungszeitraum": 2025,
                       "events": kalibrier_events(bindung)}, f)
        m = messe(d, golden, bindung)
    for sicht in ("alle", "aktiv"):
        z = m[sicht]
        assert +zellen_je_form(z) == ERWARTET_FORMEN, (sicht, zellen_je_form(z))
        ist = {e: z.n[e] for e in ERWARTET_ETIKETTEN}
        assert ist == ERWARTET_ETIKETTEN, (sicht, ist)
    with tempfile.TemporaryDirectory() as leer:
        assert main(["--faelle", leer]) == 2
        with open(os.path.join(leer, "ohne_events.json"), "w", encoding="utf-8") as f:
            json.dump({"version": 1, "veranlagungszeitraum": 2025, "events": []}, f)
        assert main(["--faelle", leer]) == 2
    print(f"selbsttest ok: {len(ERWARTET_FORMEN)} Formen, {len(ERWARTET_ETIKETTEN)} Etiketten "
          f"je Sicht getroffen; Exit 2 bei 0 Stores und 0 Events")
    return 0


if __name__ == "__main__":
    sys.exit(main())
