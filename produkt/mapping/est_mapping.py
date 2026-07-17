"""est_mapping-Schicht — Store-Snapshot -> ELSTER-Deklaration (Task #11). Deterministisch, NULL LLM.

`deklariere(snapshot, bindung)` übersetzt die BESTÄTIGTEN Store-Felder in eine ELSTER-Deklaration
(E-Nr -> Wert). Fünf Fall-Klassen (Instructor-abgenommen, produkt/mapping/KONZEPT.md):

  1  1:1            Feld -> bindung.elster_kz (direkt)
  a  Aggregation    §21-WK Detail-Slots -> DOKUMENTIERTES Ziel-Kz (E0703838), Summe im dokumentiert-
                    Bucket, NICHT deklariert (Anlage-V-Ruling: kein sauberes Einzel-Kz); verlustbehaftet
  b  Split          VOR-Summanden sind je 1:1 (eigene Kz); die Regel-Summe wird NICHT deklariert
  c  Berechnet      berechnete/steuernde Felder werden NICHT deklariert (maschinenlesbar gemeldet)
  d  Negation       fam_alleinstehend -> EfA-Feld invertiert
  e  Multiplikation anzahl_kinder -> N Anlage-Kind-Instanzen

Fail-closed (K2-Invariante auf Deklarations-Ebene, Auflage 3): ein vorlaeufiges Pflicht-Feld macht die
Deklaration UNVOLLSTÄNDIG (kein Versand). Auflage A: die dokumentierte Aggregation wird EXPLIZIT
ausgewiesen (dokumentiert-Bucket: Summe + Quell-Felder, aggregat-genau ≠ detail-genau) und NICHT in die
deklaration geschrieben. Auflage C: das NICHT-Deklarierte + die Unvollständigkeits-Gründe sind
maschinenlesbar (fehlend ≠ leer).
"""
from __future__ import annotations

import glob
import os

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)
FELDMAPPING = os.path.join(ROOT, "elster", "feldmapping.stub.yaml")   # Andock-Referenz (Auflage B)

# --- Transform-Konfiguration (source-verankert via 2026-07-17-enr-nachtraege-kandidaten.md) ---
# Klasse a — DOKUMENTIERTE Aggregation (dokumentiert, NICHT deklariert): die §21-WK-Detail-Slots
# summieren auf ein Ziel-Kz, das die E10-Submission NICHT als sauberes Einzel-Kz führt (Anlage-V-Ruling
# 2026-07-17: E0703838 braucht Zuordnungsart Direkt/Verhaelt + Mehrzeilen je Objekt). Die Summe wird
# DOKUMENTIERT (Audit/Round-Trip), aber NICHT in die submittable deklaration geschrieben — konsistent
# mit der Bindungstabelle (dort elster_kz=null+Grund für dieselben Felder). Multi-Objekt/Zuordnungsart
# = benannte Lücke (braucht anzahl_vermietungsobjekte + Zuordnungs-Modell).
DOKUMENTIERT_AGGREGAT = {
    "E0703838": ["vv_gebaeude_afa", "vv_schuldzinsen", "vv_erhaltungsaufwand", "vv_sonstige_wk"],
}
# Klasse d — Negation: Store-Feld -> EfA-Kz (invertiert; Vordruck kodiert die schädliche Haushaltsgem.).
NEGATION = {"fam_alleinstehend": "E0503701"}
# Klasse e — Multiplikation: Zähl-Feld -> N Anlage-Kind-Instanzen (MVP: nur Anzahl; Per-Kind-Kz Nachtrag).
MULTIPLIKATION = ("fam_anzahl_kinder",)


def _aggregation_quellen() -> set:
    return {f for fs in DOKUMENTIERT_AGGREGAT.values() for f in fs}


def deklariere(snapshot: dict, bindung: dict, *, snapshot_id: str | None = None) -> dict:
    """snapshot: {feld_id -> {wert, zustand, herkunft}} (materialisiert). bindung: {feld_id -> Eintrag}."""
    # Nachauflage D (Eingabe-Guard gegen Falsch-Grün): die falsche Eingabe-Ebene würde sonst STILL
    # ein leeres, vollstaendig=True-Ergebnis liefern. Erst die Snapshot-Objekt-Form abfangen ...
    if isinstance(snapshot, dict) and ("felder" in snapshot or "snapshot_id" in snapshot):
        raise ValueError("deklariere() erwartet die felder-Ebene (feld_id -> {wert, zustand, ...}), "
                         "NICHT das Snapshot-Objekt — übergib materialisiere()[0] bzw. snapshot['felder'].")

    agg_quellen = _aggregation_quellen()
    deklaration: dict = {}
    dokumentiert: dict = {}
    kind_anlagen: list = []
    nicht_deklariert: list = []
    unvollstaendig: list = []
    agg_akku = {ziel: [] for ziel in DOKUMENTIERT_AGGREGAT}
    getroffen = 0                       # wie viele Eingabe-Felder überhaupt in der Bindungstabelle sind

    for feld_id in sorted(snapshot):
        sfeld = snapshot[feld_id]
        b = bindung.get(feld_id)
        if b is None:
            nicht_deklariert.append({"feld_id": feld_id, "grund": "nicht in der Bindungstabelle"})
            continue
        getroffen += 1
        # fail-closed (Auflage 3/C): nur bestätigte Werte deklarieren; vorlaeufig/offen -> unvollständig
        if sfeld.get("zustand") != "bestaetigt":
            unvollstaendig.append({"feld_id": feld_id,
                                   "grund": f"Wert {sfeld.get('zustand')} — Pflicht-Bestätigung (Zwei-Signal) fehlt"})
            continue
        wert = sfeld["wert"]
        if feld_id in NEGATION:                                   # Klasse d
            deklaration[NEGATION[feld_id]] = not bool(wert)
        elif feld_id in MULTIPLIKATION:                           # Klasse e
            kind_anlagen = [{"index": i + 1} for i in range(int(wert))]
        elif feld_id in agg_quellen:                             # Klasse a (dokumentierte Aggregation sammeln)
            for ziel, srcs in DOKUMENTIERT_AGGREGAT.items():
                if feld_id in srcs:
                    agg_akku[ziel].append((feld_id, int(wert)))
        elif b.get("elster_kz"):                                  # Klasse 1 / b (1:1)
            deklaration[b["elster_kz"]] = wert
        else:                                                     # Klasse c (nicht deklariert)
            nicht_deklariert.append({"feld_id": feld_id,
                                     "grund": b.get("elster_kz_grund", "kein elster_kz")})

    # ... dann: nicht-leere Eingabe, aber KEIN Feld in der Bindungstabelle -> falsche Ebene/Struktur,
    # kein stilles Leer-Grün (Nachauflage D).
    if snapshot and getroffen == 0:
        raise ValueError("kein Eingabe-Feld in der Bindungstabelle gefunden — vermutlich falsche "
                         "Eingabe-Ebene/-Struktur; deklariere() liefert kein stilles Leer-Ergebnis.")

    # Dokumentierte Aggregation ausrechnen (Auflage A: Summe + Quell-Felder explizit; NICHT deklariert)
    for ziel, akku in agg_akku.items():
        if akku:
            dokumentiert[ziel] = {"summe": sum(w for _, w in akku),
                                  "quell_felder": sorted(f for f, _ in akku)}

    return {
        "basis_snapshot": snapshot_id,
        "deklaration": deklaration,
        "kind_anlagen": kind_anlagen,
        "dokumentiert": dokumentiert,            # dokumentiert, NICHT deklariert: E-Nr -> {summe, quell_felder}
        "nicht_deklariert": nicht_deklariert,    # Auflage C: bewusst nicht deklariert (Grund)
        "unvollstaendig": unvollstaendig,        # Auflage C: welches Pflicht-Feld vorläufig
        "vollstaendig": not unvollstaendig,      # fail-closed
    }


def zuruecklesen(result: dict, bindung: dict) -> dict:
    """Round-Trip (Lab N3). 1:1/Negation invertierbar -> {felder: feld_id->wert}. Die dokumentierte
    Aggregation ist VERLUSTBEHAFTET: nur die Summe je Ziel-Kz ist rekonstruierbar (-> {aggregat:
    E-Nr->Summe}) und stammt aus dem dokumentiert-Bucket (NICHT aus der deklaration), NIE die
    Detail-Felder — der Store bleibt ihre Wahrheit (Auflage A, kein stiller Detail-Verlust)."""
    e_nach_feld = {b["elster_kz"]: fid for fid, b in bindung.items() if b.get("elster_kz")}
    e_nach_negation = {ziel: fid for fid, ziel in NEGATION.items()}
    felder: dict = {}
    aggregat: dict = {}
    # dokumentierte Aggregate (dokumentiert, nicht deklariert): nur die Summe, KEINE Details
    for e_nr, info in result.get("dokumentiert", {}).items():
        aggregat[e_nr] = info["summe"]
    for e_nr, wert in result["deklaration"].items():
        if e_nr in e_nach_negation:
            felder[e_nach_negation[e_nr]] = not bool(wert)
        elif e_nr in e_nach_feld:
            felder[e_nach_feld[e_nr]] = wert
    return {"felder": felder, "aggregat": aggregat}


def konsistenz_feldmapping(bindung: dict, feldmapping_path: str = FELDMAPPING) -> list:
    """Auflage B: kein Kz-Konflikt zwischen Bindungstabelle (+ est_mapping-Zielen) und
    elster/feldmapping.stub.yaml. Ein E-Nr, das in beiden Tabellen für UNTERSCHIEDLICHE Regeln steht,
    ist ein Konflikt (benannter Nachtrag). Die Tabellen mappen bewusst verschiedene Seiten
    (Bindung=Inputs, feldmapping=Regel-Outputs) — Überschneidung daher selten, der Check ist der Wächter."""
    import yaml
    if not os.path.exists(feldmapping_path):
        return [{"grund": f"feldmapping fehlt: {feldmapping_path}"}]
    fm = yaml.safe_load(open(feldmapping_path)) or {}
    fm_kz_regel = {}   # elster_feld_id -> regel_id-Präfix
    for row in fm.get("mapping", []):
        kz = row.get("elster_feld_id")
        rid = str(row.get("regel_output", "")).split(".")[0]
        if kz and str(kz).startswith("E"):
            fm_kz_regel.setdefault(kz, set()).add(rid)
    # unsere Kz: aus Bindung (1:1) + est_mapping-Zielen (Aggregation/Negation)
    unsere = {}   # E-Nr -> regel_id
    for fid, b in bindung.items():
        if b.get("elster_kz"):
            unsere.setdefault(b["elster_kz"], set()).add(b["quelle"]["regel_id"])
    konflikte = []
    for kz, regeln in unsere.items():
        if kz in fm_kz_regel and not (regeln & fm_kz_regel[kz]):
            konflikte.append({"elster_kz": kz, "bindung_regeln": sorted(regeln),
                              "feldmapping_regeln": sorted(fm_kz_regel[kz]),
                              "grund": "gleiches Kz für unterschiedliche Regeln in beiden Tabellen"})
    return konflikte
