"""veranlagung darf vom LLM vorgeschlagen werden — aber nur als Vorschlag, und nur als
"großer" Konflikt, wenn der Nutzer schon geantwortet hat.

Hintergrund: der Feld-Katalog (store.lade_katalog) ist fail-closed — ein Feld ohne
`vorschlagbar_von` darf von KEINEM Vorschlags-Schreiber gesetzt werden, und Wahlrechte waren
davon bewusst ausgenommen. Julius hat am 2026-08-12 entschieden, die Veranlagungsart zu
öffnen: das LLM liest aus dem Freitext einen FAKT ("verheiratet, geben zusammen ab") und übt
kein Wahlrecht aus — die Ausübung bleibt beim Menschen.

Diese Datei sichert die drei Eigenschaften, die diese Öffnung erst vertretbar machen. Sie
prüft ABSICHTLICH nur die Bindungs-/Katalog-Ebene und ruft chat() nicht auf: der Endpunkt
wird gerade umgebaut, und die Invarianten hier gelten unabhängig davon, wie chat() intern
aussieht.
"""

from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST          # noqa: E402
import traverser as TR      # noqa: E402


def _bindung():
    return TR.lade_bindung()


def test_llm_darf_veranlagung_vorschlagen():
    """Die Freigabe existiert und wirkt bis in den Katalog, den append_event prüft."""
    katalog = ST.lade_katalog(_bindung())
    assert "veranlagung" in katalog.get("llm", frozenset()), (
        "veranlagung fehlt im llm-Katalog — die Julius-Freigabe vom 2026-08-12 ist nicht "
        "wirksam. Ursache ist fast immer ein fehlendes `vorschlagbar_von: [llm]` in "
        "bindung_an_gesamt.yaml.")


def test_freigabe_bleibt_ein_einzelfall_kein_praezedenzfall():
    """Kein anderes Feld ohne elster_kz und mit signatur_slot auf die Festsetzung ist
    nebenbei mitgeöffnet worden.

    Der Katalog ist die einzige Stelle, die entscheidet, was eine KI anfassen darf. Wächst
    er unbemerkt, ist die fail-closed-Zusage wertlos — deshalb hier eine harte Zahl statt
    einer Stichprobe. Wird sie rot, ist das kein Fehler: prüfen, ob die neue Freigabe
    beabsichtigt war, und die Zahl bewusst nachziehen.
    """
    katalog = ST.lade_katalog(_bindung())
    llm_felder = katalog.get("llm", frozenset())
    assert len(llm_felder) == 17, (
        f"Zahl der llm-freigegebenen Felder ist {len(llm_felder)}, erwartet 17 "
        f"(16 Bestand + veranlagung). Neu: {sorted(llm_felder)}")


def test_veranlagung_ist_ein_grosser_konflikt():
    """Ein Vorschlag auf ein schon beantwortetes veranlagung darf nicht still verschwinden.

    Die Unterscheidung "großer Konflikt" hängt daran, ob das Feld ANDERE Regeln steuert —
    dann ändert eine Übernahme nicht einen Wert, sondern welche Fragen überhaupt gelten.
    Genau das tut veranlagung: es ist die Ob-Bedingung von p2_festzusetzung_zusammen. Fällt
    dieser Eintrag weg, wäre die Freigabe oben nicht mehr vertretbar — dann könnte ein
    LLM-Vorschlag als beiläufige Feld-Änderung durchgehen.
    """
    bedingungen = TR.lade_regel_bedingungen()
    gesteuerte = {rb["feld"] for eintraege in bedingungen.values() for rb in eintraege}
    assert "veranlagung" in gesteuerte, (
        "veranlagung steuert keine Regel mehr über bindung_regel_bedingungen.yaml — damit "
        "wäre ein LLM-Vorschlag darauf kein 'großer' Konflikt mehr und würde nicht "
        "zurückfragen. Entweder den Eintrag wiederherstellen oder die Freigabe "
        "(vorschlagbar_von: [llm]) zurücknehmen.")


def test_veranlagung_bleibt_askable_und_ohne_kz():
    """Der Mensch muss weiterhin gefragt werden können, und die Wahl wird nicht deklariert.

    Wäre veranlagung nicht mehr askable, gäbe es keinen menschlichen Pfad mehr, über den ein
    LLM-Vorschlag bestätigt werden könnte — das Wahlrecht läge faktisch bei der KI.
    """
    b = _bindung()["veranlagung"]
    assert b.get("askable") is True, "veranlagung muss erfragbar bleiben"
    assert b.get("elster_kz") is None, (
        "veranlagung hat ein elster_kz bekommen — die Veranlagungsart steuert den Tarif und "
        "wird nicht als Betrag deklariert; ein Kz hier wäre eine zweite Repräsentation.")
