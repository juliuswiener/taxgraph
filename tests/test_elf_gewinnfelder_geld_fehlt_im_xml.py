"""Naht Ring<->Deklaration, betragsseitig: elf Betragsfelder (sechs Person-A-Gewinn-Komponenten,
vier _partner-Varianten, kap_gewinn_sonstige) haben `elster_kz: null` und werden von
est_mapping.deklariere() nie in deklaration/person_b/anlage_instanzen geschrieben (live gemessen).
bescheid_einkuenfte.py liest denselben rohen Snapshot direkt und rechnet mit -- der ausgewiesene
Steuerbetrag bewegt sich (2.135,50 EUR mehr in einer Vergleichsmessung), die abgegebene Erklaerung
aber nicht.

Dieser Test prueft die letzte offene Stelle: landen die Betraege wenigstens ueber ein Aggregat
(z.B. einkuenfte_gewinn -> E0800302) im tatsaechlich erzeugten XML? Gesucht werden die BETRAEGE
SELBST im XML-Text -- nicht Feldnamen, nicht Bucket-Mitgliedschaft.

Drei Teile:
  1. test_positivkontrolle_*    -- im SELBEN Lauf: ein nachweislich verdrahtetes Feld muss ankommen.
                                    Ohne das beweist ein roter Befund nur einen kaputten Messaufbau.
  2. test_leerlauf_*            -- ohne die elf Eingaben taucht keiner ihrer Betraege im XML auf.
                                    Schliesst aus, dass einer der gewaehlten EUR-Werte zufaellig im
                                    XML-Skelett liegt (Datum, Formularversion), unabhaengig von der
                                    Eingabe.
  3. test_elf_gewinnfelder_*    -- xfail(strict=True): die elf Einzelbetraege UND vier plausible
                                    Zwischensummen (6 Person-A-Komponenten / 4 Partner / alle elf /
                                    zehn ohne kap) sollten im XML stehen. Tun sie aktuell nicht.
                                    Reparaturrichtung bewusst offen gelassen -- ob die elf ins Mapping
                                    aufgenommen werden oder der Ring anders angebunden wird, ist nicht
                                    entschieden. Wird der Test gruen (XPASS), ist das ein Befund:
                                    entweder ein Kz oder ein Aggregat faengt die Betraege doch auf,
                                    und der xfail muss ueberprueft/entfernt werden.
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
TS = "2026-08-30T20:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

PERSON_A_GEWINN = ["betriebseinnahmen", "afa_jahresbetrag", "gewinnanteil",
                    "verguetung_taetigkeit", "verguetung_darlehen", "verguetung_ueberlassung"]
PARTNER = ["gewinnanteil_partner", "verguetung_taetigkeit_partner",
           "verguetung_darlehen_partner", "verguetung_ueberlassung_partner"]
KAP = ["kap_gewinn_sonstige"]
ELF_FELDER = PERSON_A_GEWINN + PARTNER + KAP
# dieselben unterscheidbaren Betraege wie in den vorangegangenen /tmp-Messlaeufen (n*111100 Cent),
# damit alle Messungen an denselben Zahlen haengen.
BETRAG_CENT = {f: (i + 1) * 111100 for i, f in enumerate(ELF_FELDER)}

KONTROLL_FELD = "bruttoarbeitslohn_partner"
KONTROLL_KZ = "E0200201"
KONTROLL_CENT = 4000000


def _b(s, feld_id, wert):
    ST.append_event(store=s, feld_id=feld_id, wert=wert, zustand="bestaetigt", herkunft=H,
                     schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{feld_id}"}, ts=TS)


def _basis(s, *, mit_elf: bool):
    """Person A/B-Basisfelder + Pflichtkegel, identisch in beiden Laeufen. mit_elf steuert, ob
    die elf Testfelder mitbestaetigt werden."""
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "vor_an_anteil_rv", 200000)
    _b(s, "vor_ag_anteil_rv", 150000)
    _b(s, "vor_rv_ausserhalb_lstb", 100000)
    _b(s, "kap_kapitalertraege", 500000)
    _b(s, "kap_gewinn_aktien", 0)
    _b(s, "kap_verlust_aktien", 0)
    _b(s, "kap_verlust_sonstige", 0)
    _b(s, "vv_einnahmen", 0)

    _b(s, KONTROLL_FELD, KONTROLL_CENT)
    _b(s, "vor_an_anteil_rv_partner", 160000)
    _b(s, "vor_ag_anteil_rv_partner", 120000)
    _b(s, "vor_rv_ausserhalb_lstb_partner", 80000)
    _b(s, "kap_kapitalertraege_partner", 300000)
    _b(s, "kap_gewinn_aktien_partner", 0)
    _b(s, "kap_verlust_aktien_partner", 0)
    _b(s, "kap_verlust_sonstige_partner", 0)

    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", False)   # Gewinnfelder liegen vor -- muss stimmen (sonst FLAG_NEGIERT-artig)
    _b(s, "kein_kap", False)      # kap_gewinn_sonstige liegt vor
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)
    _b(s, "gewinn_betriebsart", "gewerbe")
    _b(s, "gewinn_bezeichnung", "Testfall")
    _b(s, "einkuenfte_gewinn", 0)  # Direktwert bewusst 0

    if mit_elf:
        for f in ELF_FELDER:
            _b(s, f, BETRAG_CENT[f])


def _kz_werte_aus_xml(xml: str) -> dict[str, list[str]]:
    clean = xml.replace("ns0:", "").replace("ns1:", "")
    werte: dict[str, list[str]] = {}
    for m in re.finditer(r"<E(\d{7})>([^<]*)</E\1>", clean):
        werte.setdefault(f"E{m.group(1)}", []).append(m.group(2))
    return werte


def _betrag_im_xml(kz_werte: dict, eur: int) -> list[tuple[str, str]]:
    return [(kz, w) for kz, ws in kz_werte.items() for w in ws
            if w.strip() in (str(eur), f"{eur}.00", f"{eur},00")]


def _summen() -> dict[str, int]:
    return {
        "6 Person-A-Gewinn-Komponenten": sum(BETRAG_CENT[f] for f in PERSON_A_GEWINN) // 100,
        "4 _partner-Felder": sum(BETRAG_CENT[f] for f in PARTNER) // 100,
        "alle elf": sum(BETRAG_CENT[f] for f in ELF_FELDER) // 100,
        "zehn ohne kap_gewinn_sonstige": sum(BETRAG_CENT[f] for f in PERSON_A_GEWINN + PARTNER) // 100,
    }


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


@pytest.fixture(scope="module")
def hauptlauf(bindung):
    """EIN Store, EIN deklariere()/erzeuge_xml()-Aufruf: elf Testfelder + Kontrollfeld zusammen --
    Kontrolle und Befund durchlaufen garantiert dieselbe Messstrecke ('im selben Lauf')."""
    s = ST.leerer_store(2025, fall_id="elf-felder-geld-fehlt-hauptlauf")
    _basis(s, mit_elf=True)
    snap, sid = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung, snapshot_id=sid)
    assert result.get("eingaben_konsistent") is True, (
        "Fixture-Voraussetzung verletzt: eingaben_konsistent muss True sein, sonst wirft "
        "erzeuge_xml() vor jeder eigentlichen Pruefung.")
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, snapshot=snap)
    return _kz_werte_aus_xml(xml)


@pytest.fixture(scope="module")
def leerlauf(bindung):
    """Dieselbe Basis, aber OHNE die elf Testfelder bestaetigt."""
    s = ST.leerer_store(2025, fall_id="elf-felder-geld-fehlt-leerlauf")
    _basis(s, mit_elf=False)
    snap, sid = ST.materialisiere(s)
    result = est_mapping.deklariere(snap, bindung, snapshot_id=sid)
    assert result.get("eingaben_konsistent") is True
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, snapshot=snap)
    return _kz_werte_aus_xml(xml)


def test_positivkontrolle_bruttoarbeitslohn_partner_kommt_an(hauptlauf):
    """Im selben Lauf wie die elf: ein nachweislich verdrahtetes Feld (bruttoarbeitslohn_partner
    -> PARTNER_INSTANZ -> E0200201) muss ankommen. Ohne diese Kontrolle beweist der xfail-Befund
    unten nur, dass irgendetwas nicht durchkommt -- mit ihr beweist er, dass die Messstrecke
    funktioniert und die Luecke feldspezifisch ist."""
    eur = KONTROLL_CENT // 100
    assert KONTROLL_KZ in hauptlauf, (
        f"{KONTROLL_KZ} fehlt komplett im XML -- Messstrecke selbst ist kaputt, der xfail-Befund "
        f"unten waere nicht aussagekraeftig.")
    assert str(eur) in hauptlauf[KONTROLL_KZ], (
        f"{KONTROLL_KZ} steht im XML, aber nicht mit dem erwarteten Wert {eur}: "
        f"{hauptlauf[KONTROLL_KZ]}")


def test_leerlauf_ohne_die_elf_erscheint_kein_betrag(leerlauf):
    """Baseline: ohne dass die elf Testfelder je bestaetigt wurden, taucht keiner ihrer (fuer
    diesen Test frei gewaehlten) Betraege im XML auf. Schliesst aus, dass einer der gewaehlten
    EUR-Werte zufaellig Teil des XML-Skeletts ist -- sonst waere ein spaeterer Treffer im
    Hauptlauf nicht eindeutig der Dateneingabe zuzuschreiben."""
    fehltreffer = []
    for f in ELF_FELDER:
        eur = BETRAG_CENT[f] // 100
        t = _betrag_im_xml(leerlauf, eur)
        if t:
            fehltreffer.append(f"{f}={eur} EUR: {t}")
    for label, eur in _summen().items():
        t = _betrag_im_xml(leerlauf, eur)
        if t:
            fehltreffer.append(f"Summe '{label}'={eur} EUR: {t}")
    assert not fehltreffer, (
        "Betrag/Summe taucht auch OHNE Eingabe im XML auf -- Suche ist nicht aussagekraeftig:\n"
        + "\n".join(f"  – {x}" for x in fehltreffer))


@pytest.mark.xfail(strict=True, reason=(
    "Diese Betraege erreichen die Erklaerung nicht -- weder einzeln noch als Zwischensumme. "
    "Reparaturrichtung bewusst offen (Mapping oder Ring-Anbindung, nicht entschieden). Wird "
    "dieser Test gruen (XPASS), ist das ein Befund, kein Rauschen: dann faengt entweder ein Kz "
    "oder ein Aggregat die Betraege doch auf, und dieser xfail muss geprueft/entfernt werden."))
def test_elf_gewinnfelder_und_ihre_summen_erreichen_die_erklaerung(hauptlauf):
    """Jeder der elf Betraege und die vier plausiblen Zwischensummen sollten im erzeugten XML
    stehen -- Invariante: bestaetigtes Geld landet in der Erklaerung. Aktuell tut es das nicht,
    weder einzeln noch aggregiert."""
    fehlend = []
    for f in ELF_FELDER:
        eur = BETRAG_CENT[f] // 100
        if not _betrag_im_xml(hauptlauf, eur):
            fehlend.append(f"{f}={eur} EUR")
    for label, eur in _summen().items():
        if not _betrag_im_xml(hauptlauf, eur):
            fehlend.append(f"Summe '{label}'={eur} EUR")
    assert not fehlend, (
        f"{len(fehlend)} Betraege/Summen erreichen die Erklaerung nicht:\n"
        + "\n".join(f"  – {x}" for x in fehlend))
