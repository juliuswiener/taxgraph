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
import re

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)
FELDMAPPING = os.path.join(ROOT, "elster", "feldmapping.stub.yaml")   # Andock-Referenz (Auflage B)


# ABZUGS_KZ — E10-Kz die Abzüge/Aufwendungen/Verluste deklarieren → ceiling (aufrunden).
# Quelle: anl_est1a_2025.txt:269-274 "zu Ihren Gunsten": Einnahmen/Einkünfte→abrunden (floor),
# Abzüge/Aufwendungen/Verluste→aufrunden (ceiling). Jedes Kz quell-verankert klassifiziert:
# signatur_slot/Slot-Beitrag+Regel-Kontext (Einnahme=pos Einkunft-Summand/Abzug=neg Summand/Ausgabe).
_ABZUGS_KZ = frozenset({
    "E0703838",  # V+V Werbungskosten (Aggregat §21)
    "E2000401",  # AN-Anteil RV (Vorsorgeaufwand §10 Abs.1 Nr.2)
    "E2000801",  # AG-Anteil RV (Vorsorgeaufwand §10 Abs.1 Nr.2)
    "E2000601",  # RV außerhalb LStB (Vorsorgeaufwand §10 Abs.1 Nr.2)
    "E1901301",  # Verlust aus Aktien (§20 Abs.6)
    "E1901201",  # Sonstige Verluste (§20 Abs.6)
    "E0161804",  # agB Aufwendungen (§33 Abs.1-2)
    "E0104109",  # §35a Minijob-Aufwendungen (haushaltsnahe)
    "E0107208",  # §35a haushaltsnahe Dienstleistungen
    "E0111215",  # §35a Handwerker-Arbeitskosten
    "E2001203",  # KV AN (Vorsorgeaufwand §10 Abs.1 Nr.3) (ceiling)
    "E2001505",  # PV AN (Vorsorgeaufwand §10 Abs.1 Nr.3) (ceiling)
    "E2001805",  # KV And_Pers (Vorsorgeaufwand §10 Abs.1 Nr.3) (ceiling)
    "E2002105",  # PV And_Pers (Vorsorgeaufwand §10 Abs.1 Nr.3) (ceiling)
    "E2003104",  # KV privat (Vorsorgeaufwand §10 Abs.1 Nr.3) (ceiling)
    "E2003202",  # PV privat (Vorsorgeaufwand §10 Abs.1 Nr.3) (ceiling)
    # § 10 Abs. 1 Nr. 3a — weitere Vorsorgeaufwendungen, 5 Kategorien (2026-08-06 Feldsplit)
    "E2001403",  # AL_Vers/Sum (Arbeitslosenversicherung) (ceiling)
    "E2001503",  # ErwU_BU_Vers/Sum (Erwerbsunfähigkeit) (ceiling)
    "E2001803",  # U_HP_Ris_Vers/Sum (Unfall/Haftpflicht) (ceiling)
    "E2001903",  # RV_m_WR_KapLV/Sum (Rente mit Überschuss) (ceiling)
    "E2002003",  # RV_o_WR_o_AV/Sum (Rente ohne Überschuss) (ceiling)
    "E0505607",  # Schulgeld-Summe (§10 Abs.1 Nr.9, Abzug) (ceiling)
})


def _cent_nach_kz(wert: int, kz: str) -> int | str:
    """Store-CENT → Kz-Wert: E60-Präfix → "N,NN"-Dezimalstring, E10 integer asymmetrisch gerundet.
    anl_est1a_2025.txt:269-274: "zu Ihren Gunsten" — Einnahmen/Einkünfte→abrunden (floor),
    Abzüge/Aufwendungen/Verluste→aufrunden (ceiling)."""
    if kz.startswith("E60"):
        return f"{wert // 100},{wert % 100:02d}"
    if kz in _ABZUGS_KZ:
        return -(-wert // 100)   # ceiling: 1..99 Cent → 1 EUR (aufrunden=günstiger)
    return wert // 100            # floor: 1..99 Cent → 0 EUR (abrunden=günstiger)


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
# Klasse e — Multiplikation: Zähl-Feld -> N Anlage-Kind-Instanzen (MVP: nur die ANZAHL).
# Der count-MVP ist TARIF-KORREKT — Kinderfreibetrag (p32_6) + Kindergeld (p31) rechnen count × Betrag,
# KEINE Per-Kind-Tarif-Inputs. Die Per-Kind-FORM-Kz für die ELSTER-Submission SIND (Stand 2026-07-20)
# 5/6 gebunden — instanz_gruppe:kind, Klasse INSTANZ (s.u.): Identifikationsnummer E0500406, Art des
# Kindschaftsverhältnisses E0500807/E0500808 je Elternteil, Zeitraum E0500601/E0500805 (bindung_kap_vv_
# familie.yaml, getestet test_est_mapping.py inkl. Instanz-2-Roundtrip). NUR Kindergeld-Anspruch E0500702
# bleibt offen — NICHT weil ein Repeated-Instance-Store-Modell fehlt (das existiert längst), sondern (a)
# Werte-Kodierung nicht amtlich verifizierbar (ERiC-Beispiel-XMLs 2021-2025 zeigen konstant Platzhalter
# "2448", keine erkennbare Semantik) und (b) bereits 2026-07-18 explizit abgelehnt (reports/review/
# 2026-07-18-anlage-kind-kz-kandidaten.md: "≠Haushaltszugehörigkeit"). Bewusste Lücke, kein Vergessen.
MULTIPLIKATION = ("fam_anzahl_kinder",)
# §23 private Veraeusserungsgeschaefte: Rohdaten-Felder je Instanz (Ring-Input,
# kein eigenes Kz). Der per-Instanz-Gewinn wird in deklariere() berechnet
# und an die Art-Kz (Grundstueck/anderes WG) geroutet.
P23_BETRAGSFELDER = frozenset({
    "p23_veraeusserungspreis", "p23_anschaffung_herstellungskosten", "p23_werbungskosten",
})
P23_GEWINN = {"art_feld": "p23_veraeusserungs_typ", "kz": {
    "grundstueck": "E0306801", "anderes_wg": "E0307701"}}
# Klasse f — Verzweigung: EIN Wert-Slot -> N-Kz je Enum-Wert eines Art-Felds. § 22-Renten: die Anlage-R-
# Zeile (und damit das Kz) hängt an rentner_renten_art. aa-Basisversorgung (gesetzl/berufsst/basisrente)
# -> Leibr_gesetzl; bb-Ertragsanteil private/sonstige -> Leibr_priv/Leibr_sonst. Nur DEKLARATION (Kz-Wahl);
# die bb-STEUERBERECHNUNG (Ertragsanteil) ist der Registry-Nachtrag p22_1_ertragsanteil (dev-1).
VERZWEIGUNG = {
    "rentner_jahresrente": {"art_feld": "rentner_renten_art", "kz": {
        "gesetzliche_rente": "E1800301", "berufsstaendische_versorgung": "E1800301",
        "private_basisrente": "E1800301", "private_leibrente": "E1801601",
        "sonstige_leibrente": "E1803102"}},
    "rentner_renten_beginn_jahr": {"art_feld": "rentner_renten_art", "kz": {
        "gesetzliche_rente": "E1800501", "berufsstaendische_versorgung": "E1800501",
        "private_basisrente": "E1800501", "private_leibrente": "E1801701",
        "sonstige_leibrente": "E1803202"}},
    # § 16 Abs. 4 Betriebsveräußerungsgewinn: der Deklarations-Kz hängt an der Betriebsart (Anlage). p16_4-
    # Freibetrag ist anlage-agnostisch (rechnet), nur der Kz verzweigt zwischen allen drei Anlagen
    # (per XSD-Walk verifiziert): gewerbe/Mitunternehmeranteil -> Anlage G E0801301; selbstaendig § 18 ->
    # Anlage S E0804501 [Vor_FB]; land_forst § 14 -> Anlage L E0901201 [VAe_G_FB_Antr].
    "rentner_veraeusserungsgewinn": {"art_feld": "rentner_veraeusserungs_betriebsart", "kz": {
        "gewerbe": "E0801301", "selbstaendig": "E0804501", "land_forst": "E0901201"}},
    # §§ 13-18 Gewinneinkünfte (Stufe 1, p2_festzusetzung_einzel): der vorberechnete Gewinn verzweigt nach
    # gewinn_betriebsart in Anlage G/S. land_forst NICHT gebunden — Anlage L hat zwei alternative Kz
    # (P4_Abs_1_3=E0901007, P13a=E0901103) je Gewinnermittlungsart; unser Modell differenziert nicht.
    # land_forst -> fail-closed in nicht_deklariert ("ohne Kz-Zweig", kein Over-Tax — Gewinn rechnet).
    "einkuenfte_gewinn": {"art_feld": "gewinn_betriebsart", "kz": {
        "gewerbe": "E0800502", "selbstaendig": "E0803402"}},

# KV/PV-Versicherungsart-Weiche: ein Betragsfeld -> 3 Kz je Art.
# Art-Feld = versicherungsart. XML-Writer routet ueber Schema-Pfad.
# TEILMENGEN (nicht binden): E2001405 (KV ohne Krankengeld, AN-Subset von E2001203),
# E2002005 (KV mit Krankengeld, And_Pers-Subset von E2001805) — keine Doppelzählung. (AN/And_Pers/Beitr_p).
"basis_kv": {"art_feld": "versicherungsart", "kz": {
    "gesetzlich_an": "E2001203", "gesetzlich_freiwillig": "E2001805", "privat": "E2003104"}},
"basis_pv": {"art_feld": "versicherungsart", "kz": {
    "gesetzlich_an": "E2001505", "gesetzlich_freiwillig": "E2002105", "privat": "E2003202"}}}
# Klasse g — Person-Multiplikation (Zusammenveranlagung, Store-Modell A): die person-individuellen
# _partner-Einkommensfelder gehen in die Anlage-N-INSTANZ B (person_b-Bucket) — DIESELBEN Kz wie Person A
# (KEINE neuen Kz; E0220201/E2200401 existieren nicht, Person-B ist ein zweites Sub-Dokument). Globale
# Felder bleiben in der Haupt-Deklaration (Person A). person_b_idnr trägt ein DISTINKTES Mantelbogen-Kz
# (E0100082) und läuft daher als 1:1 in die Haupt-Deklaration, nicht in den person_b-Bucket.
PARTNER_INSTANZ = {
    "bruttoarbeitslohn_partner": "E0200201",
    "vor_an_anteil_rv_partner": "E2000401",
    "vor_ag_anteil_rv_partner": "E2000801",
    "vor_rv_ausserhalb_lstb_partner": "E2000601",
    # § 20 Kapital Person-B (Anlage-KAP-Instanz B): dieselben Person-A-Kz, kein distinktes Ehegatte-Kz
    # (Schema-Recon 2026-07-18). Anlage KAP wird per Person gefüllt (eigener Sparer-Pauschbetrag).
    "kap_kapitalertraege_partner": "E1900701",
    "kap_gewinn_aktien_partner": "E1900901",
    "kap_verlust_aktien_partner": "E1901301",
    "kap_verlust_sonstige_partner": "E1901201",
    # § 33b Abs. 3/5 Behinderten-Pauschbetrag Person-B: AgB_67907_CType.Beh hat maxOccurs=2,
    # indiziert über Person A/B (E10-2025.xsd Z.9622-9639) — Person B nutzt bei zweiter
    # Block-Instanz DIESELBEN Kz wie Person A, kein eigenes Ehegatte-Kz.
    "rentner_grad_der_behinderung_partner": "E0109708",
    "rentner_hilflos_blind_taubblind_partner": "E0109706",
}
# Klasse g×f — Renten-Verzweigung Person-B (§ 22, Anlage-R-Instanz B): wie VERZWEIGUNG (aa/bb-Kz je
# renten_art), aber der Wert läuft in den person_b-Bucket (dieselben Person-A-Kz, kein Ehegatte-Kz).
# Art-Weiche = rentner_renten_art_partner. fail-closed ohne bestätigte Partner-Art (wie Person A).
PARTNER_VERZWEIGUNG = {
    "rentner_jahresrente_partner": {"art_feld": "rentner_renten_art_partner", "kz": {
        "gesetzliche_rente": "E1800301", "berufsstaendische_versorgung": "E1800301",
        "private_basisrente": "E1800301", "private_leibrente": "E1801601",
        "sonstige_leibrente": "E1803102"}},
    "rentner_renten_beginn_jahr_partner": {"art_feld": "rentner_renten_art_partner", "kz": {
        "gesetzliche_rente": "E1800501", "berufsstaendische_versorgung": "E1800501",
        "private_basisrente": "E1800501", "private_leibrente": "E1801701",
        "sonstige_leibrente": "E1803202"}},

# KV/PV Versicherungsart-Weiche Person B (Klasse g×f): Art-Feld = versicherungsart_partner.
"basis_kv_partner": {"art_feld": "versicherungsart_partner", "kz": {
    "gesetzlich_an": "E2001203", "gesetzlich_freiwillig": "E2001805", "privat": "E2003104"}},
"basis_pv_partner": {"art_feld": "versicherungsart_partner", "kz": {
    "gesetzlich_an": "E2001505", "gesetzlich_freiwillig": "E2002105", "privat": "E2003202"}}}
# Klasse INSTANZ — Repeated-Instance (Store-Modell A, Multi-Objekt/Multi-Rente/Per-Kind): ein wiederholbares
# Anlage-Feld trägt für Instanz 2..N das Suffix __<n> am feld_id (Instanz 1 = die Basis-feld_id ohne Suffix,
# unverändert deklariert). Das Suffix liegt vollständig in [a-z0-9_] → der Store-feld_id-Pattern
# ^[a-z][a-z0-9_]*$ bleibt UNVERÄNDERT (bewusst kein '#' — '#' bräche das Store-Schema-Gate). Der Store lernt
# die Instanz GAR NICHT; sie ist reine Konvention der Bindung (instanz_gruppe) + est_mapping. Je Instanz wird
# DIESELBE Basis-Kz wiederverwendet (Instanz-Reuse, kein neuer Kz — analog Person-B/Klasse g auf der Instanz-
# Achse; Kz-Instanz-Recon 2026-07-18: alle drei Anlagen V/R/Kind = Reuse, kein distinkter Instanz-Kz).
_INSTANZ_RE = re.compile(r"^(?P<base>[a-z][a-z0-9_]*)__(?P<idx>[1-9][0-9]*)$")


def parse_instanz(feld_id: str):
    """Die EINE Enumerations-Wahrheit für Repeated-Instance: `base__<n>` (n>=2) -> (base, n); eine Basis-
    feld_id OHNE Suffix -> None (= Instanz 1). BEIDE Seiten — die Deklaration (deklariere) UND der Ring-
    Instanz-Reader (instanzen) — rufen NUR diese Funktion (kein zweites Regex, keine Enumerations-Drift
    zwischen Deklaration und Ring)."""
    m = _INSTANZ_RE.match(feld_id)
    return (m.group("base"), int(m.group("idx"))) if m else None


def _aggregation_quellen() -> set:
    return {f for fs in DOKUMENTIERT_AGGREGAT.values() for f in fs}


def _deklariere_instanz(basis: str, idx: int, feld_id: str, sfeld: dict, snapshot: dict, bindung: dict,
                        anlage_instanzen: dict, unvollstaendig: list, nicht_deklariert: list) -> None:
    """Routet EIN Instanz-Feld (base__n) in den anlage_instanzen-Bucket seiner Gruppe (Instanz-Reuse der
    Basis-Kz). fail-closed je Instanz: ein vorlaeufiges Instanz-Feld -> unvollständig (nicht deklariert).
    Reuse der Basis-Mapping-Logik: 1:1 (elster_kz) -> instanz.felder[kz]; Aggregat-Quelle -> instanz.
    dokumentiert[ziel] (Summe je Instanz); Art-Verzweigung (Klasse f) -> Kz hängt an der INSTANZ-Art
    art_feld__idx (Multi-Rente: jede Rente-Instanz hat eigene renten_art -> eigener aa/bb-Kz)."""
    b = bindung[basis]                                    # Aufrufer garantiert: basis instanz-fähig
    if sfeld.get("zustand") != "bestaetigt":
        unvollstaendig.append({"feld_id": feld_id,
                               "grund": f"Instanz-Wert {sfeld.get('zustand')} — Pflicht-Bestätigung (Zwei-Signal) fehlt"})
        return
    gruppe = b["instanz_gruppe"]
    wert = sfeld["wert"]
    inst = anlage_instanzen.setdefault(gruppe, {}).setdefault(
        idx, {"index": idx, "felder": {}, "dokumentiert": {}})
    if basis in _aggregation_quellen():                   # Klasse a je Instanz (dokumentiert, nicht deklariert)
        for ziel, srcs in DOKUMENTIERT_AGGREGAT.items():
            if basis in srcs:
                agg = inst["dokumentiert"].setdefault(ziel, {"summe": 0, "quell_felder": []})
                agg["summe"] += (_cent_nach_kz(int(wert), ziel) if b.get("typ") == "cent" else int(wert))
                agg["quell_felder"].append(feld_id)
    elif basis in VERZWEIGUNG:                            # Klasse f je Instanz (Kz je INSTANZ-Art, Multi-Rente)
        cfg = VERZWEIGUNG[basis]
        art_feld_inst = f"{cfg['art_feld']}__{idx}"       # die Art DIESER Instanz (z.B. rentner_renten_art__2)
        art = snapshot.get(art_feld_inst)
        if art is None or art.get("zustand") != "bestaetigt":
            unvollstaendig.append({"feld_id": feld_id,
                                   "grund": f"Instanz-Art ({art_feld_inst}) unbestätigt — Kz-Zweig offen"})
        else:
            kz = cfg["kz"].get(art["wert"])
            if kz:
                inst["felder"][kz] = _cent_nach_kz(wert, kz) if b.get("typ") == "cent" else wert
            else:
                nicht_deklariert.append({"feld_id": feld_id,
                                         "grund": f"Instanz-Art '{art['wert']}' ohne Kz-Zweig"})
    elif basis in P23_BETRAGSFELDER:                     # Klasse h — §23 Rohdaten (still speichern, kein Kz)
        inst.setdefault("rohdaten", {})[basis] = int(wert)
    elif b.get("elster_kz"):                              # 1:1 je Instanz (Kz-Reuse der Basis)
        inst["felder"][b["elster_kz"]] = _cent_nach_kz(wert, b["elster_kz"]) if b.get("typ") == "cent" else wert
    else:
        nicht_deklariert.append({"feld_id": feld_id,
                                 "grund": f"Instanz-Basis '{basis}' ohne elster_kz/Aggregat-Ziel"})


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
    person_b: dict = {}                 # Anlage-N-Instanz B (Zusammenveranlagung, Klasse g)
    anlage_instanzen: dict = {}         # gruppe -> {idx -> {index, felder, dokumentiert}} (Klasse INSTANZ)
    nicht_deklariert: list = []
    unvollstaendig: list = []
    agg_akku = {ziel: [] for ziel in DOKUMENTIERT_AGGREGAT}
    getroffen = 0                       # wie viele Eingabe-Felder überhaupt in der Bindungstabelle sind

    for feld_id in sorted(snapshot):
        sfeld = snapshot[feld_id]
        # Klasse INSTANZ: base__n (n>=2) einer instanz-fähigen Basis -> anlage_instanzen (Reuse Basis-Kz).
        parsed = parse_instanz(feld_id)                   # dieselbe Enumerations-Wahrheit wie der Ring-Reader
        basis = parsed[0] if parsed else None
        if basis is not None and bindung.get(basis, {}).get("instanz_gruppe"):
            getroffen += 1
            _deklariere_instanz(basis, parsed[1], feld_id, sfeld, snapshot, bindung,
                                anlage_instanzen, unvollstaendig, nicht_deklariert)
            continue
        b = bindung.get(feld_id)
        if b is None:
            # Instanz-1-Felder (ohne __1-Suffix) werden über _deklariere_instanz geroutet, siehe oben.
            nicht_deklariert.append({"feld_id": feld_id, "grund": "nicht in der Bindungstabelle"})
            unvollstaendig.append({"feld_id": feld_id, "grund": "Feld nicht in der Bindungstabelle"})
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
            nicht_deklariert.append({"feld_id": feld_id,
                                     "grund": b.get("elster_kz_grund") or
                                     "Multiplikation -> kind_anlagen (Kinderzahl implizit)"})
        elif feld_id in agg_quellen:                             # Klasse a (dokumentierte Aggregation sammeln)
            for ziel, srcs in DOKUMENTIERT_AGGREGAT.items():
                if feld_id in srcs:
                    agg_akku[ziel].append((feld_id, _cent_nach_kz(int(wert), ziel) if b.get("typ") == "cent" else int(wert)))
        elif feld_id in VERZWEIGUNG:                             # Klasse f (Art-Verzweigung: 1 Slot -> N-Kz)
            cfg = VERZWEIGUNG[feld_id]
            art = snapshot.get(cfg["art_feld"])
            if art is None or art.get("zustand") != "bestaetigt":
                # fail-closed: ohne bestätigte Art ist die Kz-Zuordnung offen -> nicht deklarieren
                unvollstaendig.append({"feld_id": feld_id,
                                       "grund": f"Art ({cfg['art_feld']}) unbestätigt — Kz-Zweig offen"})
            else:
                kz = cfg["kz"].get(art["wert"])
                if kz:
                    deklaration[kz] = _cent_nach_kz(wert, kz) if b.get("typ") == "cent" else wert
                else:
                    nicht_deklariert.append({"feld_id": feld_id,
                                             "grund": f"Art '{art['wert']}' ({cfg['art_feld']}) ohne Kz-Zweig"})
        elif feld_id in PARTNER_VERZWEIGUNG:                     # Klasse g×f (Renten-Verzweigung Person B -> person_b)
            cfg = PARTNER_VERZWEIGUNG[feld_id]
            art = snapshot.get(cfg["art_feld"])
            if art is None or art.get("zustand") != "bestaetigt":
                unvollstaendig.append({"feld_id": feld_id,
                                       "grund": f"Partner-Renten-Art ({cfg['art_feld']}) unbestätigt — Kz-Zweig offen"})
            else:
                kz = cfg["kz"].get(art["wert"])
                if kz:
                    person_b[kz] = _cent_nach_kz(wert, kz) if b.get("typ") == "cent" else wert
                else:
                    nicht_deklariert.append({"feld_id": feld_id,
                                             "grund": f"Partner-Renten-Art '{art['wert']}' ohne Kz-Zweig"})
        elif feld_id in PARTNER_INSTANZ:                         # Klasse g (Person-Multiplikation, Instanz B)
            person_b[PARTNER_INSTANZ[feld_id]] = _cent_nach_kz(wert, PARTNER_INSTANZ[feld_id]) if b.get("typ") == "cent" else wert
        elif b.get("elster_kz"):                                  # Klasse 1 / b (1:1)
            deklaration[b["elster_kz"]] = _cent_nach_kz(wert, b["elster_kz"]) if b.get("typ") == "cent" else wert
        elif feld_id in P23_BETRAGSFELDER:                  # Klasse h — §23 Instanz-1-Rohdaten: in p23_veraeusserung sammeln
            anlage_instanzen.setdefault("p23_veraeusserung", {}).setdefault(
                1, {"index": 1, "felder": {}, "dokumentiert": {}}).setdefault("rohdaten", {})[feld_id] = int(wert)
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

    # §23 per-Instanz-Gewinn (ALLE p23-Rohdaten sind jetzt gesammelt):
    # Gewinn = preis - AK/HK - WK (Cent), dann via P23_GEWINN an Kz des Typs.
    for idx, inst in anlage_instanzen.get("p23_veraeusserung", {}).items():
        raw = inst.get("rohdaten", {})
        preis = int(raw.get("p23_veraeusserungspreis", 0))
        ak = int(raw.get("p23_anschaffung_herstellungskosten", 0))
        wk = int(raw.get("p23_werbungskosten", 0))
        gewinn_cent = preis - ak - wk
        if gewinn_cent == 0:
            continue
        art_feld_inst = "p23_veraeusserungs_typ" if idx == 1 else f"p23_veraeusserungs_typ__{idx}"
        art = snapshot.get(art_feld_inst)
        if art is None or art.get("zustand") != "bestaetigt":
            unvollstaendig.append({"feld_id": f"p23_veraeusserung__{idx}",
                                   "grund": f"{art_feld_inst} unbestätigt — Kz-Zweig offen"})
            continue
        kz = P23_GEWINN["kz"].get(art["wert"])
        if kz:
            inst["felder"][kz] = _cent_nach_kz(gewinn_cent, kz)
        else:
            nicht_deklariert.append({"feld_id": f"p23_veraeusserung__{idx}",
                                     "grund": f"p23-Typ '{art['wert']}' ohne Kz-Zweig"})

    # Klasse INSTANZ: dict-of-dicts -> stabile, index-sortierte Liste je Gruppe (leere dokumentiert-Buckets weg)
    anlage_instanzen_out: dict = {}
    for gruppe, instanzen in anlage_instanzen.items():
        eintraege = []
        for i in sorted(instanzen):
            inst = instanzen[i]
            if not inst["felder"] and not inst["dokumentiert"]:
                continue                                  # leere Instanz (alle Instanz-Felder fail-closed) -> nicht ausgeben
            eintrag = {"index": i, "felder": inst["felder"]}
            if inst["dokumentiert"]:
                eintrag["dokumentiert"] = {ziel: {"summe": agg["summe"],
                                                  "quell_felder": sorted(agg["quell_felder"])}
                                           for ziel, agg in inst["dokumentiert"].items()}
            eintraege.append(eintrag)
        if eintraege:                                     # leere Gruppe (alle Instanzen fail-closed) weglassen
            anlage_instanzen_out[gruppe] = eintraege

    return {
        "basis_snapshot": snapshot_id,
        "deklaration": deklaration,
        "kind_anlagen": kind_anlagen,
        "person_b": person_b,                    # Anlage-N-Instanz B (Zusammenveranlagung): E-Nr -> Wert (Kz wie Person A)
        "anlage_instanzen": anlage_instanzen_out,  # Klasse INSTANZ: gruppe -> [{index, felder{kz->wert}, dokumentiert?}] (Instanz 2..N, Kz-Reuse)
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
    # Klasse f: alle Art-Zweig-Kz eines Wert-Felds zeigen zurück auf dasselbe Wert-Feld (der VALUE ist
    # invertierbar; die exakte Renten-Art ist nur gruppen-genau [aa/bb], nicht rekonstruierbar).
    e_nach_verzweigung = {kz: feld for feld, cfg in VERZWEIGUNG.items() for kz in cfg["kz"].values()}
    b_nach_feld = {kz: feld for feld, kz in PARTNER_INSTANZ.items()}
    # Klasse INSTANZ: Reuse-Kz -> Basis-feld_id (nur instanz-fähige Felder); Instanz-Suffix __idx angehängt.
    inst_kz_nach_feld = {b["elster_kz"]: fid for fid, b in bindung.items()
                         if b.get("elster_kz") and b.get("instanz_gruppe")}
    felder: dict = {}
    aggregat: dict = {}
    # dokumentierte Aggregate (dokumentiert, nicht deklariert): nur die Summe, KEINE Details
    for e_nr, info in result.get("dokumentiert", {}).items():
        aggregat[e_nr] = info["summe"]
    # Person-B-Instanz (Zusammenveranlagung): Kz -> _partner-Feld (invertierbar, eigener Bucket)
    for e_nr, wert in result.get("person_b", {}).items():
        if e_nr in b_nach_feld:
            felder[b_nach_feld[e_nr]] = wert
    # Klasse INSTANZ (Instanz 2..N): base__idx -> Wert (1:1 invertierbar); Aggregat je Instanz nur Summe.
    # 1:1-Instanz über inst_kz_nach_feld; VERZWEIGUNG-Instanz (Multi-Rente, elster_kz=null) über die
    # Art-Zweig-Kz (e_nach_verzweigung, Value invertierbar; die exakte Art ist gruppen-genau wie bei Basis).
    for instanzen in result.get("anlage_instanzen", {}).values():
        for inst in instanzen:
            idx = inst["index"]
            for kz, wert in inst.get("felder", {}).items():
                if kz in inst_kz_nach_feld:
                    felder[f"{inst_kz_nach_feld[kz]}__{idx}"] = wert
                elif kz in e_nach_verzweigung:
                    felder[f"{e_nach_verzweigung[kz]}__{idx}"] = wert
            for ziel, agg in inst.get("dokumentiert", {}).items():
                aggregat[f"{ziel}__{idx}"] = agg["summe"]
    for e_nr, wert in result["deklaration"].items():
        if e_nr in e_nach_negation:
            felder[e_nach_negation[e_nr]] = not bool(wert)
        elif e_nr in e_nach_verzweigung:
            felder[e_nach_verzweigung[e_nr]] = wert
        elif e_nr in e_nach_feld:
            felder[e_nach_feld[e_nr]] = wert
    return {"felder": felder, "aggregat": aggregat}


def instanzen(store: dict, bindung: dict, gruppe: str) -> list:
    """Ring-Naht (dev-2 -> dev-1, #5): enumeriert ALLE Instanzen einer instanz_gruppe aus dem Store und
    liefert je Instanz das materialisierte Feld-Dict der Gruppe INKL. zustand + herkunft — für dev-1s
    per-Instanz-Σ (Multi-Objekt einkuenfte_vermietung, Multi-Rente per-Rente-Ertragsanteil) + K2-Guard.

    KONVENTION (explizit, damit dev-1 STUMPF summiert — jede Instanz genau einmal):
    - liefert ALLE Instanzen: Instanz 1 = die Basis-feld_ids (OHNE Suffix, index==1) UND Instanz 2..N =
      die __n-Felder. Kein Sonderfall für die Basis — sie ist die erste Listen-Position.
    - Enumeration über `parse_instanz` (DIESELBE Wahrheit wie die Deklaration, kein zweites Regex).
    - felder je Instanz sind auf die BASIS-feld_id normiert (`vv_einnahmen__2` -> Schlüssel "vv_einnahmen"),
      damit dev-1 je Instanz denselben Schlüssel liest.
    - der per-Instanz `zustand` ist der meet (bestaetigt NUR, wenn ALLE Instanz-Felder bestaetigt — sonst
      vorlaeufig): fail-closed für dev-1s K2-Guard, damit eine unvollständige Instanz kein still-falsches Σ
      erzeugt. Die per-Feld-zustand/herkunft bleiben in `felder` sichtbar.
    Rückgabe: [{"index": i, "felder": {basis_feld_id: {wert, zustand, herkunft}}, "zustand": <meet>}], index-sortiert.
    """
    import store as _ST
    felder, _ = _ST.materialisiere(store)
    basis_felder = {fid for fid, b in bindung.items() if b.get("instanz_gruppe") == gruppe}
    vorkommen: dict = {}
    for fid, sfeld in felder.items():
        parsed = parse_instanz(fid)
        basis, idx = (parsed[0], parsed[1]) if parsed else (fid, 1)
        if basis in basis_felder:
            vorkommen.setdefault(idx, {})[basis] = sfeld
    ausgabe = []
    for idx in sorted(vorkommen):
        inst_felder = vorkommen[idx]
        zustand = _ST.meet_zustand([sf["zustand"] for sf in inst_felder.values()])
        ausgabe.append({"index": idx, "felder": inst_felder, "zustand": zustand})
    return ausgabe


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
