"""Ring<->Deklaration-Differential: jedes Betragsfeld hat einen Weg ins XML.

INVARIANTE (Vorbild test_g_askable_felder_sind_erreichbar):
  Jedes bestaetigte Betragsfeld (typ cent/int) hat entweder
  (a) ein Kz, das im erzeugten XML ankommt, ODER
  (b) einen Eintrag in `nicht_deklariert` mit Begruendung, ODER
  (c) einen Eintrag in `dokumentiert` (Aggregat), ODER
  (d) einen benannten Ausschluss (E10_AUSSCHLUSS_DATENART).
  Ein fuenftes gibt es nicht.

Getestet: Einzelveranlagung + Zusammenveranlagung, damit person_b abgedeckt.
"""
from __future__ import annotations

import os
import re
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX        # noqa: E402
import est_mapping             # noqa: E402
import store as ST             # noqa: E402
import traverser as TR         # noqa: E402

HID = "74931"
TS = "2026-08-04T14:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _b(s, feld_id, wert, zustand="bestaetigt"):
    sig = {"signal_1": None, "signal_2": f"ok@{feld_id}"} if zustand == "bestaetigt" else {
        "signal_1": None, "signal_2": None}
    ST.append_event(store=s, feld_id=feld_id, wert=wert, zustand=zustand,
                    herkunft=H, schreiber="ui:laie", signal=sig, ts=TS)


def _alle_kz_im_xml(xml_str: str) -> set[str]:
    """Extrahiert ALLE vorkommenden E-Nummern aus dem XML.
    Namespace-Präfixe (ns1:, ns0:) werden vorher entfernt."""
    clean = xml_str.replace("ns0:", "").replace("ns1:", "")
    return set(f"E{m}" for m in re.findall(r"<E(\d{7})>", clean))


def _pruefe_differential(felder_snapshot: dict, bindung: dict,
                         result: dict, xml: str,
                         label: str) -> list[str]:
    """Prueft Invariante fuer EINEN Snapshot. Gibt Liste der Funde (leer = OK)."""
    # Töpfe aufbauen
    kz_im_xml = _alle_kz_im_xml(xml)
    dekl = result.get("deklaration", {})
    nicht_dekl = {e["feld_id"] for e in result.get("nicht_deklariert", [])}
    dokumentiert_kz = set(result.get("dokumentiert", {}).keys())
    person_b = result.get("person_b", {})

    # Alle Kz, die im XML ankommen (aus deklaration + person_b)
    kz_ankommend = set(dekl.keys()) | set(person_b.keys())

    funde: list[str] = []
    geprueft = 0
    betraege_ohne_kz = 0

    for feld_id, sfeld in felder_snapshot.items():
        b = bindung.get(feld_id)
        if b is None:
            continue
        if sfeld.get("zustand") != "bestaetigt":
            continue
        # Nur Betragsfelder (cent/int, monetary)
        typ = b.get("typ", "")
        if typ not in ("cent", "int", "euro"):
            continue
        wert = sfeld.get("wert", 0)
        if wert == 0 or wert is None:
            continue  # Nullwerte ohne Steuerwirkung ignorieren

        geprueft += 1

        # (d) benannter Ausschluss?
        kz = str(b.get("elster_kz") or "")
        if kz in EX.E10_AUSSCHLUSS_DATENART:
            betraege_ohne_kz += 1
            continue

        # (b) nicht_deklariert?
        if feld_id in nicht_dekl:
            betraege_ohne_kz += 1
            continue

        # PARTNER_INSTANZ (Klasse g): mapped zu person_b
        if feld_id in est_mapping.PARTNER_INSTANZ:
            kz = est_mapping.PARTNER_INSTANZ[feld_id]
            if kz in kz_ankommend and kz in kz_im_xml:
                continue

        # anlage_instanzen-Kz: Instanz 1 via deklaration, Instanz 2..N via anlage_instanzen
        # Fuer nicht-__-Felder mit instanz_gruppe: Kz in deklaration
        if b.get("instanz_gruppe") and "__" not in feld_id:
            if kz in kz_ankommend and kz in kz_im_xml:
                continue

        # (a) Kz in der XML (direkt via deklaration)
        if kz and kz in kz_im_xml:
            continue

        # (c) dokumentiert (Aggregat-Ziel-Kz, z.B. E0703838)
        if kz and kz in dokumentiert_kz:
            betraege_ohne_kz += 1
            continue

        # (c) alternativ: feld_id ist QUELL-feld eines Aggregats -> in dokumentiert
        is_agg_quelle = False
        for ziel_kz, quellen in getattr(est_mapping, "DOKUMENTIERT_AGGREGAT", {}).items():
            if feld_id in quellen:
                is_agg_quelle = True
                break
        if is_agg_quelle:
            betraege_ohne_kz += 1
            continue

        # Instanz-Felder (base__n): Kz-Reuse, Kz in anlage_instanzen
        if "__" in feld_id and b.get("instanz_gruppe"):
            # Kz ist das Basis-Kz
            if kz and kz in kz_im_xml:
                continue

        funde.append(
            f"{feld_id} (typ={typ}, wert={wert}): Kz={kz!r} nicht in XML "
            f"und nicht in nicht_deklariert/dokumentiert/Ausschluss")

    if funde:
        print(f"\n  [{label}] {geprueft} Betragsfelder geprueft, "
              f"{betraege_ohne_kz} ohne Kz (dokumentiert), "
              f"{len(funde)} Funde:")
        for f in funde:
            print(f"    FUND: {f}")

    return funde


# ---------------------------------------------------------------- Einzelveranlagung

def test_differential_einzel_keine_luecken(bindung):
    """Einzelveranlagung: alle Betragsfelder haben Weg ins XML oder sind dokumentiert."""
    s = ST.leerer_store(2025, fall_id="diff_einzel")
    # === Pflichtkegel ===
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", False)
    _b(s, "kein_vuv", False)
    _b(s, "kein_sonstige", True)
    # === Kz-tragende Felder ===
    _b(s, "bruttoarbeitslohn", 5000000)         # → E0200201 (1:1)
    _b(s, "vor_an_anteil_rv", 200000)            # → E2000401 (1:1)
    _b(s, "vor_ag_anteil_rv", 150000)            # → E2000801 (1:1)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)      # → E2000601 (1:1)
    _b(s, "kap_kapitalertraege", 500000)          # → E1900701 (1:1)
    _b(s, "kap_gewinn_aktien", 0)                # Null
    _b(s, "kap_verlust_aktien", 0)
    _b(s, "kap_verlust_sonstige", 0)
    _b(s, "vv_einnahmen", 3000000)               # → E0700201 (1:1)
    _b(s, "vv_gebaeude_afa", 500000)             # Klasse a (Aggregat)
    _b(s, "vv_schuldzinsen", 200000)
    _b(s, "vv_erhaltungsaufwand", 100000)
    _b(s, "vv_sonstige_wk", 50000)
    _b(s, "vv_entgelt_quote_prozent", 100)       # Klasse c (nicht deklariert)
    _b(s, "hh_handwerker_arbeitskosten", 300000)  # → E0111215 (1:1)
    _b(s, "agb_aufwendungen", 500000)             # → E0161804 (1:1)
    _b(s, "fam_alleinstehend", True)              # Klasse d (Negation)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe_differential(snap, bindung, result, xml, "Einzel")
    assert not funde, (
        f"Einzelveranlagung: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ---------------------------------------------------------------- Zusammenveranlagung

def test_differential_zusammen_keine_luecken(bindung):
    """Zusammenveranlagung + person_b: alle Betragsfelder haben Weg."""
    s = ST.leerer_store(2025, fall_id="diff_zusammen")

    # Person A
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)
    _b(s, "kap_kapitalertraege", 500000)
    _b(s, "kap_gewinn_aktien", 0)
    _b(s, "kap_verlust_aktien", 0)
    _b(s, "kap_verlust_sonstige", 0)
    _b(s, "vv_einnahmen", 3000000)
    _b(s, "vv_gebaeude_afa", 500000)
    _b(s, "vv_schuldzinsen", 200000)
    _b(s, "vv_erhaltungsaufwand", 100000)
    _b(s, "vv_sonstige_wk", 50000)
    _b(s, "vv_entgelt_quote_prozent", 100)
    _b(s, "hh_handwerker_arbeitskosten", 300000)
    _b(s, "agb_aufwendungen", 500000)
    _b(s, "fam_alleinstehend", True)

    # Person B (Partner)
    _b(s, "bruttoarbeitslohn_partner", 4000000)
    _b(s, "vor_an_anteil_rv_partner", 160000)
    _b(s, "vor_ag_anteil_rv_partner", 120000)
    _b(s, "vor_rv_ausserhalb_lstb_partner", 80000)
    _b(s, "kap_kapitalertraege_partner", 300000)
    _b(s, "kap_gewinn_aktien_partner", 0)
    _b(s, "kap_verlust_aktien_partner", 0)
    _b(s, "kap_verlust_sonstige_partner", 0)

    # Pflichtkegel
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", False)
    _b(s, "kein_vuv", False)
    _b(s, "kein_sonstige", True)

    snap, _ = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)

    funde = _pruefe_differential(snap, bindung, result, xml, "Zusammen")
    assert not funde, (
        f"Zusammenveranlagung: {len(funde)} Betragsfelder ohne Weg ins XML:\n"
        + "\n".join(f"  – {f}" for f in funde))


# ---------------------------------------------------------------- Mutations-Probe

def test_differential_mutation_kz_entfernt(bindung):
    """Gegenprobe: E0200201 aus der Deklaration entfernt (als fiele es still weg) -> Fund."""
    snap, _ = ST.materialisiere(_store_basic(bindung))
    result = est_mapping.deklariere(snap, bindung)
    # Mutation: E0200201 aus der deklaration entfernen
    if "E0200201" in result["deklaration"]:
        del result["deklaration"]["E0200201"]
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    funde = _pruefe_differential(snap, bindung, result, xml, "Mutation")
    assert funde, "Mutation: Kz entfernt -> MUSS Funde liefern"
    # Der Fund MUSS bruttoarbeitslohn nennen
    fund_text = " ".join(funde)
    assert "bruttoarbeitslohn" in fund_text, (
        f"Mutation: erwarte Fund 'bruttoarbeitslohn', habe: {funde}")


def _store_basic(bindung):
    s = ST.leerer_store(2025, fall_id="diff_mutation")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)
    _b(s, "kap_kapitalertraege", 500000)
    _b(s, "kap_gewinn_aktien", 0)
    _b(s, "kap_verlust_aktien", 0)
    _b(s, "kap_verlust_sonstige", 0)
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", False)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)
    return s