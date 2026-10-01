#!/usr/bin/env python3
"""Ticket textfeld-ohne-xsd-muster-in-der-bindung: welche XSD-Vorgabe eines Textfelds prueft das
Speichern nicht, und wie viele echte Werte verletzen sie heute?

Je Bindungsfeld `typ: text` mit Kz -- `elster_kz` und die Verzweigungs-Kz aus
`xsd_verify.ernte_est_mapping_kz` -- liest das Skript die Facetten des XSD-Typs aus ALLEN
Ableitungsschritten (`xsd_verify._resolve_type_facets` liefert nur die Patterns des ersten
Schritts ab dem Kz-Typ, der welche traegt, und keine Laengen). Jeder Wert geht durch zwei Pruefungen:
  Speichern = store._pruefe_typ_konformitaet (Auflage T + F), dieselbe Funktion wie append_event
  XSD       = Facetten des Kz-Typs, so angewandt wie ein Schema-Validator
Quellen: echte Stores (aktive und alle Events), Golden-`sachverhalt`, dazu die Deklaration
(`est_mapping.deklariere`: nur bestaetigte Werte, also das, was ins XML ginge) -- dort JEDES
deklarierte string-Kz, nicht nur die Auswahl. Schema je Store nach Veranlagungszeitraum.

XSD-Regeln: das Pattern gilt fuer den ganzen Wert; `.` trifft weder \\n noch \\r; Patterns EINES
Schritts sind ODER, die Schritte UND; Laengen zaehlen Zeichen. Klassen einer Verletzung: Format
(Pattern mit Struktur), Enum, Laenge (min/maxLength, `.{m,n}`), Zeilenumbruch (StringBaseCType,
`.`), Zeichensatz (StringZUBaseCType, Standard_E_V2), leer (leerer Wert, den das XSD abweist).

DATENSCHUTZ: ausgegeben werden NUR feld_ids der Bindung, Kz, Typnamen und Patterns des Schemas
und Zaehlwerte -- nie Werte, Fall-IDs oder Dateinamen. Gelesen wird nur, nie geschrieben.

Run:  python3 tools/parity/xsd_muster.py [--faelle DIR] [--golden DIR] [--typ text datum]
      python3 tools/parity/xsd_muster.py --selbsttest   (synthetischer Store, keine Echtdaten)
Exit: 0 gemessen · 2 null Stores, Events oder Golden-Werte gelesen
"""
from __future__ import annotations

import argparse
import collections
import contextlib
import functools
import glob
import io
import json
import os
import re
import sys
import tempfile

import wertformen as WF  # Pfade, Bindung, Store-Laden, basis_id wie in K0 (gleiches Verzeichnis)
import xsd_verify as X   # liegt nach dem WF-Import auf sys.path

EM, ST, TR = WF.EM, WF.ST, WF.TR
BASIS = {"StringZUBaseCType": "Zeichensatz", "StringBaseCType": "Zeilenumbruch"}  # laut XSD-Annotation
LAENGE = re.compile(r"\.\{(\d+),(\d+)\}")
KLASSEN = ("Format", "Enum", "Laenge", "Zeilenumbruch", "Zeichensatz", "leer")
RETTUNG = (("strip", str.strip), ("ohne Leerraum", lambda t: re.sub(r"\s+", "", t)),
           ("ohne Leerraum, gross", lambda t: re.sub(r"\s+", "", t).upper()))
PLATZHALTER = {"null", "none", "undefined", "nan", "-", "--", "/", "?", "x", "xx", "xxx", "n/a", "na", "k.a.",
               "kein", "keine", "test"}


def zeichenart(t: str) -> str:
    """Grobe Art eines Werts -- ein Etikett aus dieser Funktion, nie der Wert selbst."""
    if not t:
        return "leer"
    if t.strip().lower() in PLATZHALTER:
        return "Platzhalter"
    if re.fullmatch("[0-9]+", t):
        return "nur Ziffern"
    if re.fullmatch(r"[^\W_]+", t):
        return "Buchstaben/Ziffern"
    return "mit Leerraum" if re.search(r"\s", t) else "mit Sonderzeichen"


def py_regex(xsd: str) -> str:
    """XSD-Pattern -> Python-re fuer fullmatch. `.` ausserhalb einer Klasse trifft in XSD weder \\n
    noch \\r. Klassen-Subtraktion `[a-[b]]` kennt Python nicht: abbrechen statt still falsch messen."""
    out, klasse, i = [], False, 0
    while i < len(xsd):
        c = xsd[i]
        if c == "\\":
            out.append(xsd[i:i + 2])
            i += 2
            continue
        if klasse and c == "-" and xsd[i + 1:i + 2] == "[":
            raise ValueError(f"XSD-Klassensubtraktion: {xsd}")
        if c == "[":
            klasse = True
        elif c == "]":
            klasse = False
        out.append("[^\n\r]" if c == "." and not klasse else c)
        i += 1
    return "".join(out)


def schritte(typ: str, types: dict) -> list[dict]:
    """XSD-Typ -> Facetten je Ableitungsschritt, vom Typ bis zum eingebauten Basistyp."""
    out = []
    while typ in types and typ not in (s["typ"] for s in out):
        r = next((ch for sc in types[typ] if sc.tag == X.XS + "simpleContent"
                  for ch in sc if ch.tag in (X.XS + "restriction", X.XS + "extension")), None)
        if r is None:
            break
        f = collections.defaultdict(list)
        for e in r:
            f[e.tag.removeprefix(X.XS)].append(e.get("value"))
        out.append({"typ": typ, "xsd": f["pattern"], "re": [re.compile(py_regex(p)) for p in f["pattern"]],
                    "enum": f["enumeration"],
                    "min": max(map(int, f["minLength"] + f["length"]), default=None),
                    "max": min(map(int, f["maxLength"] + f["length"]), default=None)})
        typ = r.get("base")
    return out


def verletzt(text: str, sch: list[dict]) -> set[str]:
    """Klassen der Facetten, die `text` verletzt; leer = schema-gueltig. Ein leerer Wert, den das
    XSD abweist, zaehlt nur als "leer", gleich welche Facette ihn abweist."""
    k = set()
    for s in sch:
        if s["enum"] and text not in s["enum"]:
            k.add("Enum")
        if (s["min"] is not None and len(text) < s["min"]) or (s["max"] is not None and len(text) > s["max"]):
            k.add("Laenge")
        if s["re"] and not any(r.fullmatch(text) for r in s["re"]):
            if s["typ"] in BASIS:
                k.add(BASIS[s["typ"]])
            elif all(LAENGE.fullmatch(p) for p in s["xsd"]):
                k.add("Zeilenumbruch" if re.search("[\n\r]", text) else "Laenge")
            else:
                k.add("Format")
    return {"leer"} if k and not text else k


def speichern_ok(fid: str, wert, bindung: dict) -> bool:
    """Nimmt append_event den Wert heute an? Auflage T + F, dieselbe Funktion."""
    try:
        ST._pruefe_typ_konformitaet(fid, wert, bindung)
    except ValueError:
        return False
    return True


@functools.lru_cache(maxsize=None)
def schema_jahr(vz) -> int:
    """VZ des Stores -> Jahr des lokalen Schemas; ohne lokales Schema 2025 (aktueller Produkt-VZ)."""
    return vz if isinstance(vz, int) and not isinstance(vz, bool) and X._find_schema(vz) else 2025


@functools.lru_cache(maxsize=None)
def schema(jahr: int) -> dict[str, dict]:
    """Kz -> {typ, schritte, verify}; E10 und E77 geroutet wie xsd_verify._datenart_fuer_kz.
    `verify` = Patterns, die xsd_verify._resolve_kz_meta fuer das Kz meldet (Vergleich)."""
    kz = {}
    for name, start in {X._DATENART_DEFAULT, *X._DATENART_ROUTING.values()}:
        pfad = X._find_schema(jahr, name)
        if pfad is None:
            continue
        types = X._load_indices(X._parse_top_level_children(pfad))[0]
        for k, meta in X._resolve_kz_meta(pfad, start).items():
            if X._datenart_fuer_kz(k) == (name, start):
                kz[k] = {"typ": meta["type_name"], "schritte": schritte(meta["type_name"], types),
                         "verify": meta["patterns"]}
    return kz


def auswahl(bindung: dict, typen) -> list[tuple[str, str]]:
    """(feld_id, Kz): `elster_kz` der Felder dieser Typen, dazu ihre Verzweigungs-Kz (Klasse f)."""
    paare = {(f, b["elster_kz"]) for f, b in bindung.items() if b.get("typ") in typen and b.get("elster_kz")}
    for name, e in X.ernte_est_mapping_kz(bindung).items():
        art, _, rest = name.partition(":")
        feld = rest.split(":")[0]
        if art == "verzweigung" and bindung[feld].get("typ") in typen:
            paare.add((feld, e["elster_kz"]))
    return sorted(paare)


class Zaehlung:
    """(feld_id, Kz) -> Etikett -> Anzahl. Fuer verletzende Werte dazu die Quellen (Store-Index) je
    Zeile und je Klasse sowie Merkmale je Zeile: Laenge, Zeichenart, Herkunft, rettbar -- Etiketten
    und Zahlen, nie Werte."""

    def __init__(self):
        self.n = collections.defaultdict(collections.Counter)
        self.quellen = collections.defaultdict(set)
        self.merkmale = collections.defaultdict(collections.Counter)

    def pruefe(self, quelle, herkunft: str, fid: str, kz: str, sch, wert, bindung) -> None:
        c = self.n[(fid, kz)]
        if sch is None:
            c["Kz nicht im Schema des Jahres"] += 1
            return
        if not isinstance(wert, str):
            c["kein string"] += 1
            return
        c["Werte"] += 1
        k = verletzt(wert, sch)
        if not k:
            return
        c["verletzt"] += 1
        c.update(k)
        if bindung is not None:   # Deklaration: Wert kann umgeformt sein, Speichern sah ihn so nie
            c["Speichern nimmt an" if speichern_ok(fid, wert, bindung) else "Speichern weist ab"] += 1
        for schluessel in ((fid, kz), *k):
            self.quellen[schluessel].add(quelle)
        rett = next((n for n, f in RETTUNG if not verletzt(f(wert), sch)), "nein")
        self.merkmale[(fid, kz)].update([("Laengen", len(wert)), ("Zeichenart", zeichenart(wert)),
                                         ("Herkunft", herkunft), ("rettbar", rett)])


def kz_werte(d: dict):
    """(Kz, Wert) aus deklariere(): Hauptteil, Person B, Instanzen."""
    yield from d["deklaration"].items()
    yield from d["person_b"].items()
    for instanzen in d["anlage_instanzen"].values():
        for inst in instanzen:
            yield from inst["felder"].items()


def messe(faelle: str, golden: str, bindung: dict, paare: list) -> dict:
    je_feld = collections.defaultdict(list)
    for fid, kz in paare:
        je_feld[fid].append(kz)
    m: dict = {s: Zaehlung() for s in ("aktiv", "alle", "golden", "deklaration")}
    m.update(stores=0, events=0, golden_faelle=0, golden_werte=0, fehler=collections.Counter(),
             jahre=collections.Counter())

    def zaehle(z, quelle, herkunft, jahr, feld_id, wert):
        fid = WF.basis_id(feld_id)
        for kz in je_feld.get(fid, ()):
            info = schema(jahr).get(kz)
            z.pruefe(quelle, herkunft, fid, kz, info and info["schritte"], wert, bindung)

    for i, pfad in enumerate(sorted(glob.glob(os.path.join(faelle, "*.json")))):
        try:
            with open(pfad, encoding="utf-8") as f:   # wie produkt/haut/api.py:lade_fall
                store = json.load(f)
        except (OSError, ValueError) as e:
            m["fehler"][f"laden: {type(e).__name__}"] += 1
            continue
        events = store.get("events") if isinstance(store, dict) else None
        if not isinstance(events, list):
            m["fehler"]["ohne events-Liste"] += 1
            continue
        vz = store.get("veranlagungszeitraum")
        jahr = schema_jahr(vz)
        vz_zusatz = "" if jahr == vz else ", VZ ohne Schema"   # dann gilt Schema 2025
        m["stores"] += 1
        m["events"] += len(events)
        m["jahre"][jahr] += 1
        for e in events:
            zaehle(m["alle"], i, f"{e.get('schreiber')}{vz_zusatz}", jahr, e.get("feld_id"), e.get("wert"))
        for fid, e in ST._aktives(store).items():
            zaehle(m["aktiv"], i, f"{e.get('schreiber')}{vz_zusatz}", jahr, fid, e.get("wert"))
        try:
            d = EM.deklariere(ST.materialisiere(store)[0], bindung)
        except Exception as e:  # noqa: BLE001 -- Messung, kein Produktpfad: zaehlen statt abbrechen
            m["fehler"][f"deklariere wirft {type(e).__name__}"] += 1
            continue
        # api.einreichen baut das XML nur bei eingaben_konsistent. Davor und im Bau sperren weitere
        # Pruefungen (Scheibe, Ring, Absender, Bankverbindung): konsistent heisst nicht "erreicht ERiC".
        weg = ("konsistent" if d["eingaben_konsistent"] else "nicht konsistent") + vz_zusatz
        for kz, w in kz_werte(d):
            if isinstance(w, str):
                info = schema(jahr).get(kz)
                m["deklaration"].pruefe(i, weg, "(deklariert)", kz if X._KZ_RE.match(kz) else "(kein Kz)",
                                        info and info["schritte"], w, None)
    for i, pfad in enumerate(sorted(glob.glob(os.path.join(golden, "*.yaml")))):
        with open(pfad, encoding="utf-8") as fh:
            fall = WF.yamlstrict.load_str(fh.read(), herkunft=pfad)
        m["golden_faelle"] += 1
        for k, v in (fall.get("sachverhalt") or {}).items():
            m["golden_werte"] += 1
            zaehle(m["golden"], i, "golden", 2025, k, v)
    return m


def proben(bw: str) -> dict[str, str]:
    """Synthetische Abwandlungen des beispielwerts: nimmt Speichern an, was das XSD abweist?
    `ł` (U+0142) ist ein gueltiges XML-Zeichen, liegt aber nicht in Standard_E_V2."""
    return {"Ziffern->0": re.sub(r"\d", "0", bw), "Ziffern->9": re.sub(r"\d", "9", bw), "+Ziffer": bw + "0",
            "Leerzeichen vorn": " " + bw, "+\\n": bw + "\n", "+ł": bw + "ł", "leer": "",
            "1000 Zeichen": "x" * 1000}


def urteil(fid: str, kz: str, bindung: dict) -> str:
    info = schema(2025).get(kz)
    if info is None:
        return "Kz nicht im Schema"
    bw = bindung[fid].get("beispielwert")
    if not isinstance(bw, str):
        return "nicht pruefbar: kein string-beispielwert"
    if verletzt(bw, info["schritte"]) or not speichern_ok(fid, bw, bindung):
        return "beispielwert verletzt XSD oder Speichern"
    locker = [n for n, p in proben(bw).items() if speichern_ok(fid, p, bindung) and verletzt(p, info["schritte"])]
    return "Speichern lockerer: " + ", ".join(locker) if locker else "Speichern = XSD auf allen Proben"


def vorgabe(sch: list[dict]) -> str:
    """Format-Patterns, Enum-Umfang, Laengengrenzen und Basis-Schritte eines Typs, tabellenfest."""
    teile, lo, hi = [], 0, None
    basis = ["Standard_E_V2" if BASIS[s["typ"]] == "Zeichensatz" else "ohne \\n \\r" for s in sch if s["typ"] in BASIS]
    for s in sch:
        grenzen = [(s["min"], s["max"])] + [(int(m[1]), int(m[2])) for m in map(LAENGE.fullmatch, s["xsd"]) if m]
        for a, b in grenzen:
            lo = max(lo, a or 0)
            hi = b if hi is None else (hi if b is None else min(hi, b))
        teile += ["`" + (p if len(p) <= 48 else p[:45] + "...").replace("|", "\\|") + "`"
                  for p in s["xsd"] if s["typ"] not in BASIS and not LAENGE.fullmatch(p)]
        if s["enum"]:
            teile.append(f"Enum ({len(s['enum'])})")
    if lo or hi is not None:
        teile.append(f"Laenge {lo}-{'' if hi is None else hi}")
    return ", ".join(teile + basis) or "—"


def drucke_struktur(paare: list, bindung: dict) -> collections.Counter:
    print("\n#### Struktur: Bindung gegen XSD 2025\n")
    print("| feld_id | Kz | typ | XSD-Typ | Vorgabe im XSD | muster | Urteil |")
    print("|---|---|---|---|---|---|---|")
    zahl, blind, je_probe = collections.Counter(), collections.Counter(), collections.Counter()
    for fid, kz in paare:
        info = schema(2025).get(kz)
        u = urteil(fid, kz, bindung)
        art, _, locker = u.partition(": ")
        zahl[art] += 1
        je_probe.update(locker.split(", ") if art == "Speichern lockerer" else ())
        if info:
            alle = {p for s in info["schritte"] for p in s["xsd"]}
            blind.update(BASIS.get(s["typ"], s["typ"]) for s in info["schritte"]
                         for p in s["xsd"] if p not in info["verify"])
            zahl["Kz, bei denen xsd_verify Patterns fehlen"] += bool(alle - set(info["verify"]))
        print(f"| `{fid}` | {kz} | {bindung[fid].get('typ')} | {info['typ'] if info else '—'} | "
              f"{vorgabe(info['schritte']) if info else '—'} | {'ja' if bindung[fid].get('muster') else 'nein'} | {u} |")
    print(f"\nUrteile: {dict(zahl)}")
    print(f"Speichern lockerer je Probe ((feld_id, Kz)-Paare): {dict(je_probe)}")
    print(f"Patterns, die xsd_verify._resolve_kz_meta nicht meldet (je Typ-Schritt): {dict(blind)}")
    return zahl


def drucke_werte(z: Zaehlung, titel: str) -> None:
    spalten = ["Werte", "kein string", "verletzt", *KLASSEN, "Speichern nimmt an", "Speichern weist ab"]
    merkmale = ("Laengen", "Zeichenart", "Herkunft", "rettbar")
    print(f"\n#### {titel}\n")
    print("| feld_id | Kz | " + " | ".join(spalten) + " | Quellen | " + " | ".join(merkmale) + " |")
    print("|---|---|" + "---:|" * (len(spalten) + 1) + "---|" * len(merkmale))
    summe, gesamt = collections.Counter(), collections.Counter()
    for (fid, kz), c in sorted(z.n.items(), key=lambda kv: (-kv[1]["verletzt"], kv[0])):
        summe.update(c)
        gesamt.update(z.merkmale[(fid, kz)])
        if not c["Werte"] and not c["kein string"]:
            continue
        zellen = [", ".join(f"{w}×{n}" for (sp, w), n in sorted(z.merkmale[(fid, kz)].items()) if sp == m) or "—"
                  for m in merkmale]
        print(f"| `{fid}` | {kz} | " + " | ".join(str(c[s]) for s in spalten)
              + f" | {len(z.quellen[(fid, kz)])} | " + " | ".join(zellen) + " |")
    quellen = set().union(*z.quellen.values()) if z.quellen else set()
    print(f"\n{titel} — Summe: {dict((s, summe[s]) for s in spalten if summe[s])}; "
          f"Zeilen mit Verletzung: {sum(1 for c in z.n.values() if c['verletzt'])}; "
          f"Quellen mit Verletzung: {len(quellen)}, je Klasse: { {k: len(z.quellen[k]) for k in KLASSEN if k in z.quellen} }; "
          f"sonst: {dict((e, n) for e, n in summe.items() if e not in spalten)}")
    for m in merkmale[1:]:
        print(f"  {m}: {dict((w, n) for (sp, w), n in gesamt.most_common() if sp == m)}")


def main(argv=None) -> int:
    bindung = TR.lade_bindung()
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--faelle", default=WF.api_constants.FAELLE, help="Verzeichnis mit <fall_id>.json")
    ap.add_argument("--golden", default=os.path.join(WF.ROOT, "golden", "cases"))
    ap.add_argument("--typ", nargs="+", default=["text"], help="Bindungstypen der Auswahl")
    ap.add_argument("--selbsttest", action="store_true")
    a = ap.parse_args(argv)
    if a.selbsttest:
        return selbsttest(bindung)
    paare = auswahl(bindung, a.typ)
    print("Module: " + ", ".join(f"{m.__name__}={m.__file__}" for m in (ST, TR, EM, X, WF)))
    print(f"Schema 2025: {X._find_schema(2025)}; Auswahl typ={a.typ}: {len(paare)} (feld_id, Kz)-Paare, "
          f"{len({f for f, _ in paare})} feld_ids")
    drucke_struktur(paare, bindung)
    m = messe(a.faelle, a.golden, bindung, paare)
    print(f"\nFall-Verzeichnis: {m['stores']} Stores, {m['events']} Events; Schema-Jahr je Store: "
          f"{dict(m['jahre'])}; Fehler: {dict(m['fehler']) or 0}")
    print(f"Golden: {m['golden_faelle']} Faelle, {m['golden_werte']} Werte (Schema 2025)")
    if not (m["stores"] and m["events"] and m["golden_faelle"] and m["golden_werte"]):
        print("FEHLER: null Stores, Events oder Golden-Werte gelesen — das ist kein Messergebnis.")
        return 2
    drucke_werte(m["aktiv"], "AKTIVE Events (Stand je Feld)")
    drucke_werte(m["alle"], "ALLE Events (mit ersetzten)")
    drucke_werte(m["golden"], "GOLDEN sachverhalt")
    drucke_werte(m["deklaration"], "DEKLARATION (bestaetigt, jedes string-Kz)")
    return 0


# Kalibrierung: (feld_id, Wert, erwartete Klassen; leer = schema-gueltig). Speichern weist seit der Decision
# textfeld-format-aus-xsd-beim-speichern Format-Verletzungen und leeren Text ab, Freitext-Laenge, \n und
# Zeichensatz nicht.
KALIBRIERUNG = [("stammdaten_nachname", "x" * 26, {"Laenge"}), ("stammdaten_vorname", "a\nb", {"Zeilenumbruch"}),
                ("stammdaten_wohnort", "Łódź", {"Zeichensatz"}), ("stammdaten_strasse", "", {"leer"}),
                ("stammdaten_plz", "1234", {"Format", "Laenge"}), ("stammdaten_hausnummer", "12a", {"Format"}),
                ("stammdaten_bic", "null", {"Format"}), ("kind_idnr", "1234567890", {"Format"}),
                ("kind_kindschaftsverh_zeitraum_a", "01.01-31.12\n", {"Format"}),
                ("kind_wohnsitz_inland_zeitraum", "1.1.-31.12.", {"Format"}), ("kind_vorname", "Müller", set())]
SPEICHERN_WEIST_AB = {"stammdaten_strasse", "stammdaten_plz", "stammdaten_hausnummer", "stammdaten_bic", "kind_idnr",
                      "kind_kindschaftsverh_zeitraum_a", "kind_wohnsitz_inland_zeitraum"}


def selbsttest(bindung: dict) -> int:
    """Kalibrierung ohne Echtdaten: synthetischer Store, je Feld ein Wert mit bekannter Verletzung, in
    allen drei Store-Sichten. Dazu py_regex, die Luecke in xsd_verify, zwei Urteile, Exit 2 ohne Daten."""
    assert py_regex(".{1,3}") == "[^\n\r]{1,3}" and py_regex("[.]\\.") == "[.]\\."
    try:
        py_regex("[a-[b]]")
        raise AssertionError("Klassensubtraktion nicht erkannt")
    except ValueError:
        pass
    name = schema(2025)["E0100201"]
    zeichensatz = [p for s in name["schritte"] if s["typ"] == "StringZUBaseCType" for p in s["xsd"]]
    assert zeichensatz and not set(zeichensatz) & set(name["verify"]), "xsd_verify meldet Standard_E_V2 doch"
    assert urteil("kind_kindschaftsverh_zeitraum_a", "E0500601", bindung) == "Speichern = XSD auf allen Proben"
    assert set(urteil("stammdaten_nachname", "E0100201", bindung).partition(": ")[2].split(", ")) == {
        "+\\n", "+ł", "1000 Zeichen"}
    events = [{"event_id": f"{n:064x}", "ts": "2026-10-01T00:00:00+00:00", "feld_id": fid, "wert": wert,
               "zustand": "bestaetigt", "schreiber": "kalibrierung",
               "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}, "ersetzt": None}
              for n, (fid, wert, _) in enumerate(KALIBRIERUNG)]
    with tempfile.TemporaryDirectory() as d, tempfile.TemporaryDirectory() as golden:
        with open(os.path.join(d, "kalibrierung.json"), "w", encoding="utf-8") as f:
            json.dump({"version": 1, "veranlagungszeitraum": 2025, "events": events}, f)
        m = messe(d, golden, bindung, auswahl(bindung, ["text"]))
    for sicht in ("alle", "aktiv", "deklaration"):
        for fid, _, erwartet in KALIBRIERUNG:
            kz = bindung[fid]["elster_kz"]
            c = m[sicht].n[("(deklariert)" if sicht == "deklaration" else fid, kz)]
            assert {k for k in KLASSEN if c[k]} == erwartet and c["Werte"] == 1, (sicht, fid, dict(c))
            if sicht != "deklaration" and erwartet:
                ab = fid in SPEICHERN_WEIST_AB
                assert (c["Speichern weist ab"], c["Speichern nimmt an"]) == (ab, not ab), (sicht, fid, dict(c))
    with tempfile.TemporaryDirectory() as leer, contextlib.redirect_stdout(io.StringIO()):
        assert main(["--faelle", leer]) == 2
        with open(os.path.join(leer, "ohne_events.json"), "w", encoding="utf-8") as f:
            json.dump({"version": 1, "veranlagungszeitraum": 2025, "events": []}, f)
        assert main(["--faelle", leer]) == 2
    print(f"selbsttest ok: {len(KALIBRIERUNG)} Kalibrierwerte in alle/aktiv/deklaration klassiert, "
          f"Speichern-Urteil getroffen; Standard_E_V2 fehlt in xsd_verify; Exit 2 bei 0 Stores und 0 Events")
    return 0


if __name__ == "__main__":
    sys.exit(main())
