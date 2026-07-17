#!/usr/bin/env python3
"""P9 Runde 3 — checkESt-Fuzzing (AUFTRAG 2).

Golden-Sachverhalt -> est_mapping (Deklarations-Kz) -> ELSTER-XML -> EricBearbeiteVorgang
(ERIC_VALIDIERE, offline, KEIN Versand). Jede FehlerRegelpruefung = Befund; katalogisiert mit
Feldidentifikator + RegelName + FachlicheFehlerId + Text und einem Triage-Vorschlag:

  mapping-fix        die Meldung entsteht durch UNSERE XML-/Mapping-Konstruktion (fehlendes
                     Pflicht-Begleitfeld, unscharfe Kz-Zuordnung) — bei uns zu beheben.
  erwartbar          bekannte, inhaerente Grenze (checkESt validiert Deklaration, nicht
                     berechnete Groessen; Golden-Fall hat keine ESt-Deklarationsprojektion;
                     Feld liegt in einer Anlage ausserhalb des Minimalfixtures).
  echter-engine-fund checkESt deckt eine echte Inkonsistenz auf -> manueller Review-Kandidat.

Vier Phasen:
  P1  Golden-Injektion   ausgewaehlte Goldens der 5 Gruppen (EP/GWG/§10d/KAP/VOR) durch die
                         Bruecke schicken; rc==0 = CLEAN (Mapping plausibel), rc!=0 = Befunde.
  P2  Struktur-Regel-Enumeration  Mutations-Operatoren (Pflicht-Begleitfeld droppen, Cross-Feld
                         brechen) katalogisieren die STRUKTUR-Regeln, die checkESt auf unseren
                         deklarierten Feldern erzwingt. (checkESt prueft Format/Struktur, NICHT
                         Magnitude — 999 Arbeitstage/9999 km bleiben rc==0.)
  P3  Trunkierungs-Sonde  viele unabhaengige Fehler stapeln, zurueckgegebene Zahl zaehlen;
                         Falsch-Gruen-Sperre gegen eine stille Fehler-Kappung.
  P4  Groß-XML + Nebenläufigkeit  Latenz-Messpunkt auf groesserem Payload + Serialisierungs-
                         hinweis (Nachlauf zur A1-Latenzmessung).

Hersteller-ID NUR aus $ELSTER_HERSTELLER_ID (nie im Code, nie im Katalog). Report untracked;
Commit ueber Instructor.

    ERIC_DIR=~/02_Software/eric ELSTER_HERSTELLER_ID=... \
        python3 elster/fuzz/checkest_fuzz.py [--json OUT.json]
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
ELSTER = os.path.dirname(HERE)
ROOT = os.path.dirname(ELSTER)
sys.path.insert(0, ELSTER)
sys.path.insert(0, os.path.join(ELSTER, "submission"))

import checkest_gate as CE  # noqa: E402

BASE_XML = os.path.join(ELSTER, "submission", "testfall_est2025_minimal.xml")
GOLDEN = os.path.join(ROOT, "golden", "cases")
DATENART = "ESt_2025"
NS = "{http://www.elster.de/EricXML/1.2/EricBearbeiteVorgang}"

try:
    import yaml
except ImportError:
    yaml = None


def _base() -> bytes:
    with open(BASE_XML, "rb") as f:
        xml, hid = CE._mit_hersteller_id(f.read())
    if not hid:
        sys.exit("ABBRUCH: $ELSTER_HERSTELLER_ID leer — ohne ID nur GESPERRT-Kurzschluss.")
    return xml


def _set(xml: bytes, tag: str, val) -> bytes:
    """<TAG>..</TAG> ersetzen (Feld muss im Fixture existieren)."""
    pat = re.compile(rf"<{tag}>.*?</{tag}>".encode(), re.S)
    new = f"<{tag}>{val}</{tag}>".encode()
    out, n = pat.subn(new, xml)
    if n == 0:
        raise KeyError(f"Feld {tag} nicht im Fixture — Insertion noetig")
    return out


def _drop(xml: bytes, tag: str) -> bytes:
    return re.sub(rf"<{tag}>.*?</{tag}>".encode(), b"", xml, flags=re.S)


def parse_fehler(antwort: str) -> list[dict]:
    """FehlerRegelpruefung-Liste aus der Ericantwort."""
    out = []
    try:
        root = ET.fromstring(antwort)
    except ET.ParseError:
        return out
    for f in root.findall(f"{NS}FehlerRegelpruefung"):
        def g(t):
            e = f.find(f"{NS}{t}")
            return e.text if e is not None else None
        out.append({"feld": g("Feldidentifikator"), "regel": g("RegelName"),
                    "fehler_id": g("FachlicheFehlerId"), "zeile": g("VordruckZeilennummer"),
                    "text": (g("Text") or "")[:200]})
    return out


def validate(xml: bytes) -> tuple[int, list[dict]]:
    rc, ant = CE.validate(xml, DATENART)
    return rc, parse_fehler(ant)


# ---------------------------------------------------------------- est_mapping-Bruecke
# Deklarations-Groessen (Golden-Sachverhalt) -> amtliche ELSTER-Kz. Nur GEERDETE Felder
# (Kz-Label via kz_extract aus E10-2025.html bestaetigt). Berechnete Groessen (tarifliche
# Steuer, zvE) sind NICHT deklariert und stehen hier bewusst nicht.

def map_ep(sv: dict) -> tuple[dict, list[str]]:
    m, notes = {}, []
    if "arbeitstage" in sv:
        m["E0203503"] = int(sv["arbeitstage"])                    # aufgesucht an Tagen
    if "entfernung_km_roh" in sv:
        m["E0203504"] = int(sv["entfernung_km_roh"])              # einfache Entfernung km (abger.)
    if sv.get("oepnv_kosten_jahr", 0):
        notes.append("oepnv_kosten_jahr -> E0203611 (Aufwand ÖPNV) NICHT im Minimalfixture — "
                     "Insertion in Erste_Taetig noetig; hier nicht injiziert (erwartbar)")
    return m, notes


def map_vor(sv: dict) -> tuple[dict, list[str]]:
    m, notes = {}, []
    gesamt = sv.get("vorsorge_gesamtbeitraege_inkl_ag")
    ag = sv.get("vorsorge_ag_anteil_steuerfrei")
    if gesamt is not None and ag is not None:
        m["E2000801"] = int(ag)                                   # AG-Anteil (LStB Nr. 22 a/b)
        m["E2000401"] = int(gesamt - ag)                          # AN-Anteil (LStB Nr. 23 a/b)
        notes.append("MAPPING-UNSCHÄRFE: Golden 'vorsorge_gesamtbeitraege_inkl_ag' ist EINE Zahl; "
                     "ELSTER splittet LStB-zeilenscharf (E2000401 AN Nr.23 / E2000801 AG Nr.22 / "
                     "E2000601 ges.RV ausserhalb LStB). Split hier best-effort AN=gesamt-AG.")
    return m, notes


def map_arbeitnehmer(sv: dict) -> tuple[dict, list[str]]:
    m, notes = {}, []
    lohn = sv.get("bruttoarbeitslohn") or sv.get("einkuenfte_nichtselbststaendig")
    if lohn is not None:
        m["E0200201"] = int(lohn)                                 # Bruttoarbeitslohn laut LStB
    return m, notes


# Auswahl: 5–10 Goldens quer durch die vom Instructor genannten Gruppen.
SELECTION = [
    ("EP", "ep_2024_staffel_30km", map_ep),
    ("EP", "ep_2024_beispiel1_oepnv", map_ep),
    ("EP", "ep_2026_flach_30km", map_ep),
    ("VOR", "gesamt_2024_vorsorge_capped", map_vor),
    ("VOR", "gesamt_2026_vorsorge_capped", map_vor),
    ("N", "arbeitnehmer_2026_einzel_60000", map_arbeitnehmer),
]

# Gruppen ohne ESt1A-Deklarationsprojektion im Minimalfixture — dokumentierter Struktur-Befund.
GAPS = {
    "GWG": ("p6_2 GwG-Sofortabzug ist Anlage-EÜR-Feld E6002301 (feldmapping status=mapped), NICHT "
            "im ESt1A-Kern; kein ESt-seitiger Golden-Input-Fall. Deklarierbar nur via Anlage EÜR "
            "(eigene Datenart E77). -> erwartbar."),
    "§10d": ("§10d-Verlustvortrag ist gesondert FESTGESTELLTE Bescheid-Größe (analog tarifliche "
             "Steuer, vgl. feldmapping-Doktrin); Golden verlustvortrag_bestand liegt im KStG-"
             "Nenner-B-Kontext, nicht in der ESt-Deklaration. Kein Kz im Minimalfixture. -> erwartbar."),
    "KAP": ("KAP-Kapitalertragsteuer-Abzugsbeträge (E1904701 etc.) sind im Fixture bereits populiert "
            "und rc==0-plausibel; es existiert KEIN ESt-seitiger Golden-KAP-Fall (zinsertrag/-aufwand "
            "liegen im KStG-Nenner-B-Kontext). -> erwartbar, kein Golden zu injizieren."),
}


def load_golden(case_id: str) -> dict:
    p = os.path.join(GOLDEN, case_id + ".yaml")
    with open(p) as f:
        return yaml.safe_load(f)


def triage(feld: str, injected: set, gap: bool) -> str:
    if gap:
        return "erwartbar"
    if feld and any(k in (feld or "") for k in injected):
        return "mapping-fix"
    # Pflicht-Begleitfelder der injizierten Sektionen (Instructor-Härtung)
    companions = ["E0203003", "E0203501", "BV", "E0102002", "Zeitraum", "Nutzdatenticket"]
    if feld and any(c in feld for c in companions):
        return "mapping-fix"
    return "echter-engine-fund"


def phase1() -> list[dict]:
    print("\n=== P1  Golden-Injektion ===")
    base = _base()
    results = []
    for gruppe, case_id, mapper in SELECTION:
        sv = load_golden(case_id)["sachverhalt"]
        kz, notes = mapper(sv)
        xml = base
        applied = {}
        for tag, val in kz.items():
            try:
                xml = _set(xml, tag, val)
                applied[tag] = val
            except KeyError as e:
                notes.append(str(e))
        rc, fehler = validate(xml)
        injected = set(applied)
        for f in fehler:
            f["triage"] = triage(f["feld"], injected, gap=False)
        status = "CLEAN" if rc == 0 else f"BEFUNDE({len(fehler)})"
        print(f"  [{gruppe:4s}] {case_id:32s} inj={list(applied)} rc={rc} -> {status}")
        for n in notes:
            print(f"          note: {n}")
        for f in fehler:
            print(f"          FUND [{f['triage']}] {f['fehler_id']} @ {f['feld']}")
        results.append({"gruppe": gruppe, "case": case_id, "injiziert": applied,
                        "rc": rc, "notes": notes, "fehler": fehler})
    # Gruppen-Luecken dokumentieren (keine Injektion)
    for g, reason in GAPS.items():
        print(f"  [{g:4s}] (kein ESt1A-Slot) -> erwartbar: {reason[:80]}...")
        results.append({"gruppe": g, "case": None, "rc": None, "triage": "erwartbar",
                        "grund": reason})
    return results


def phase2() -> list[dict]:
    print("\n=== P2  Struktur-Regel-Enumeration (Mutations-Operatoren) ===")
    base = _base()
    ops = [
        ("drop_EP_Ziel_E0203003", lambda x: _drop(x, "E0203003")),
        ("drop_EP_PLZ_E0203501", lambda x: _drop(x, "E0203501")),
        ("drop_EP_Tage_E0203503", lambda x: _drop(x, "E0203503")),
        ("drop_EP_km_E0203504", lambda x: _drop(x, "E0203504")),
        ("empty_BV_E0102002", lambda x: _drop(x, "E0102002")),
        ("Zeitraum_ungleich_VZ", lambda x: x.replace(b"<Zeitraum>2025</Zeitraum>",
                                                      b"<Zeitraum>2024</Zeitraum>")),
        ("drop_Nutzdatenticket", lambda x: _drop(x, "NutzdatenTicket")),
        ("drop_Bruttolohn_E0200201", lambda x: _drop(x, "E0200201")),
        ("drop_KiSt_KAP_E1900601", lambda x: _drop(x, "E1900601")),
        ("magnitude_999AT_9999km", lambda x: _set(_set(x, "E0203503", 999), "E0203504", 9999)),
    ]
    catalog = []
    for name, op in ops:
        try:
            xml = op(base)
        except KeyError as e:
            print(f"  {name:30s} SKIP ({e})")
            continue
        rc, fehler = validate(xml)
        # P2 mutiert ABSICHTLICH -> jede gefeuerte Regel ist die dokumentierte Folge, KEIN
        # Engine-Fund. rc-Klasse mitfuehren: 610001002 = Plausibilitaet, 610301200 = I/O-Gate
        # (z.B. fehlendes Nutzdatenticket) kurzschliesst VOR der Plausibilitaet.
        for f in fehler:
            f["klasse"] = "struktur-regel (Mutation absichtlich)"
        if rc == 0:
            verdikt = "rc=0 (KEINE Regel — Format/Magnitude ungeprueft)"
        elif rc == 610301200:
            verdikt = f"rc={rc} (I/O-Gate, short-circuit VOR Plausibilitaet)"
        else:
            verdikt = f"rc={rc} -> {len(fehler)} FehlerRegelpruefung"
        print(f"  {name:30s} {verdikt}")
        for f in fehler:
            print(f"       {f['fehler_id']:14s} {f['regel']}")
        catalog.append({"operator": name, "rc": rc, "fehler": fehler})
    return catalog


def phase3() -> dict:
    print("\n=== P3  Trunkierungs-Sonde (Falsch-Grün-Sperre gegen Fehler-Kappung) ===")
    base = _base()
    # Viele unabhaengige PLAUSIBILITAETS-Fehler gleichzeitig stapeln. WICHTIG: NutzdatenTicket
    # NICHT droppen — das ist ein I/O-Gate (rc=610301200), das VOR der Plausibilitaet
    # kurzschliesst und 0 FehlerRegelpruefung liefert (verfaelscht die Zaehlung).
    x = base
    for tag in ["E0203003", "E0203501", "E0102002", "E0200201", "E1900601", "E0203503",
                "E0203504"]:
        x = _drop(x, tag)
    x = x.replace(b"<Zeitraum>2025</Zeitraum>", b"<Zeitraum>2024</Zeitraum>")
    rc, fehler = validate(x)
    n_struktur = len(fehler)
    print(f"  (a) 8 Struktur-Drops: rc={rc}, {n_struktur} FehlerRegelpruefung zurueck "
          f"(alle -> keine Kappung bei {n_struktur}).")

    # (b) Cap hart ausreizen: JEDEN numerischen E-Feld-Inhalt mit unzulaessigem Wert ueberschreiben
    # -> ein Format-Fehler je Feld. Zeigt, ob ERiC bei vielen Fehlern kappt.
    corrupt = re.sub(rb"<(E\d{7})>([^<]*)</\1>", rb"<\1>ZZ99XX</\1>", base)
    n_felder = len(re.findall(rb"<E\d{7}>ZZ99XX</E\d{7}>", corrupt))
    rc2, fehler2 = validate(corrupt)
    n_cap = len(fehler2)
    gekappt = n_felder > n_cap
    print(f"  (b) {n_felder} Felder korrumpiert: rc={rc2}, {n_cap} FehlerRegelpruefung zurueck "
          f"-> {'GEKAPPT' if gekappt else 'vollstaendig'}.")
    if gekappt:
        print(f"      *** TRUNKIERUNG BESTAETIGT: {n_felder} Fehler eingebaut, nur {n_cap} gemeldet. "
              f"checkESt kappt die FehlerRegelpruefung-Liste bei {n_cap}. FALSCH-GRUEN-RISIKO: eine "
              f"Erklaerung mit >{n_cap} Fehlern verliert Fehler {n_cap+1}+ STILL. ***")
        print(f"      Mitigation (setting-unabhaengig): Fixpunkt-Revalidierung — gemeldete Fehler "
              f"beheben, RE-validieren, wiederholen bis rc==0; NIE eine nicht-leere Fehlerliste als "
              f"vollstaendig behandeln. Cap-Anhebung via EricEinstellungSetzen ist im Entwickler-"
              f"handbuch ('Bedeutung der ERiC-Einstellungen') zu suchen (PDF nicht im Extract) — offen.")
    return {"struktur_drops": n_struktur, "korrumpierte_felder": n_felder,
            "gemeldete_fehler_cap": n_cap, "trunkierung_bestaetigt": gekappt}


def phase4(reps: int = 8) -> dict:
    print("\n=== P4  Groß-XML + Nebenläufigkeit (A1-Nachlauf) ===")
    base = _base()
    # "Groß": EP-Sektion-Werte gesetzt + KAP/VOR belegt (Fixture ist klein ~3.8KB; groessere
    # reale Erklaerung nicht verfuegbar — daher Messpunkt auf dem vollen validen Fixture).
    xml = _set(_set(base, "E0203503", 220), "E0203504", 20)
    CE.validate(xml, DATENART)               # warm-up (Plugin laden)
    ts = []
    for _ in range(reps):
        t0 = time.perf_counter()
        rc, _a = CE.validate(xml, DATENART)
        ts.append(time.perf_counter() - t0)
        if rc != 0:
            sys.exit(f"ABBRUCH P4: rc={rc} != 0")
    ts.sort()
    med = ts[len(ts) // 2] * 1e3
    print(f"  warm Fixture ({len(xml)} B): median={med:.1f}ms, min={ts[0]*1e3:.1f} max={ts[-1]*1e3:.1f} (n={reps})")
    print(f"  Nebenläufigkeit: EIN ERiC-Prozess = EIN ctypes-Aufruf zur Zeit (serialisiert). Bei M "
          f"gleichzeitigen UI-Validierungen ~ M×{med:.0f}ms seriell -> Worker-Pool noetig; die "
          f"~70ms-Einzelbudget aus A1 gilt pro Slot, nicht global.")
    return {"bytes": len(xml), "warm_median_ms": med, "n": reps}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", metavar="OUT")
    args = ap.parse_args()
    if yaml is None:
        sys.exit("ABBRUCH: PyYAML fehlt.")
    if not os.environ.get("ELSTER_HERSTELLER_ID", "").strip():
        sys.exit("ABBRUCH: $ELSTER_HERSTELLER_ID nicht gesetzt.")

    print(f"[fuzz] P9-R3 checkESt-Fuzzing — {DATENART}, ERIC_VALIDIERE offline, HID gesetzt.")
    p1 = phase1()
    p2 = phase2()
    p3 = phase3()
    p4 = phase4()
    CE.beende()

    # Engine-Fund-Kandidaten kommen NUR aus P1 (valides Golden -> unerwarteter Fehler). P2 mutiert
    # absichtlich, jede Regel dort ist erwartbar (Regel-Katalog).
    echte = [f for r in p1 for f in r.get("fehler", []) if f.get("triage") == "echter-engine-fund"]
    print("\n=== ZUSAMMENFASSUNG ===")
    print(f"  P1 Golden: {sum(1 for r in p1 if r.get('rc') == 0)} CLEAN, "
          f"{sum(1 for r in p1 if r.get('rc') not in (0, None))} mit Befunden, "
          f"{sum(1 for r in p1 if r.get('rc') is None)} Gruppen-Luecken (erwartbar).")
    print(f"  P2 Struktur-Regeln katalogisiert: "
          f"{sum(len(r['fehler']) for r in p2)} FehlerRegelpruefung ueber {len(p2)} Operatoren.")
    print(f"  echter-engine-fund-Kandidaten aus P1 (manueller Review): {len(echte)}")

    if args.json:
        with open(args.json, "w") as f:
            json.dump({"p1": p1, "p2": p2, "p3": p3, "p4": p4}, f, indent=2, ensure_ascii=False)
        print(f"[fuzz] JSON -> {args.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
