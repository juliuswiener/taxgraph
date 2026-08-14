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


def test_llm_darf_jedes_askable_feld_vorschlagen():
    """ERSETZT test_freigabe_bleibt_ein_einzelfall_kein_praezedenzfall (2026-08-14).

    Der alte Test nagelte die Zahl der llm-Felder auf 17 fest, damit der Katalog nicht unbemerkt
    wächst. Er hat gehalten und ist an Julius' Entscheid rot geworden — genau wie in seinem
    eigenen Docstring vorgesehen ("prüfen, ob die neue Freigabe beabsichtigt war").

    Sie war es: "ich denke alles darf das llm ausfüllen, aber der mensch muss bei JEDEM feld
    bestätigen." Anlass war der erste echte Chat-Aufruf — aus "Ich bin Arbeitnehmer, verheiratet,
    fahre an 220 Tagen 15 km zur Arbeit und habe 62000 Euro brutto verdient" konnte die KI genau
    EINEN Wert übernehmen (veranlagung), weil Bruttolohn, Arbeitstage und Entfernung nicht im
    Katalog standen.

    Statt einer Zahl prüft dieser Test jetzt die Invariante — sie veraltet nicht, wenn Felder
    hinzukommen, und wird trotzdem rot, sobald jemand die Freigabe wieder einschränkt, ohne es
    in LLM_NICHT_VORSCHLAGBAR zu schreiben."""
    bindung = _bindung()
    llm_felder = ST.lade_katalog(bindung).get("llm", frozenset())
    askable = {f for f, b in bindung.items() if b.get("askable")}
    erwartet = askable - ST.LLM_NICHT_VORSCHLAGBAR
    assert llm_felder == erwartet, (
        f"llm-Katalog weicht ab. Fehlend: {sorted(erwartet - llm_felder)}, "
        f"zu viel: {sorted(llm_felder - erwartet)}")
    # Nicht-askable Felder bleiben draußen: sie werden berechnet oder aus Instanz-Summen gefüllt.
    nicht_askable = {f for f, b in bindung.items() if not b.get("askable")}
    assert not (llm_felder & nicht_askable), (
        f"Die KI darf berechnete Felder vorschlagen: {sorted(llm_felder & nicht_askable)[:5]}")


def test_die_freigabe_hebelt_das_zwei_signal_nicht_aus():
    """Der Preis der Freigabe wäre zu hoch, wenn mit dem Katalog auch die Sicherheit fiele.
    Sie fällt nicht: der Schutz lag nie im Katalog, sondern darin, dass ein Vorschlag vorläufig
    bleibt und der Mensch bestätigt. Hier für ein Feld nachgewiesen, das vorher gesperrt war."""
    ev = ST.append_event(
        ST.leerer_store(2025, fall_id="freigabe"), feld_id="bruttoarbeitslohn", wert=6200000,
        zustand="vorlaeufig",
        herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="llm:chat", signal={"signal_1": {"typ": "chat"}, "signal_2": None},
        katalog=ST.lade_katalog(_bindung()), ts="2026-08-14T10:00:00Z")
    assert ev["zustand"] == "vorlaeufig", "Die KI hat einen bestätigten Wert geschrieben."
    assert ev["signal"]["signal_2"] is None, "Die KI hat das zweite Signal selbst gesetzt."


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
