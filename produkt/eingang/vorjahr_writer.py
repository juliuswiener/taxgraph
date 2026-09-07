"""Vorjahr-Übernahme-Writer (import:vorjahr — dritter Store-Writer nach laie + import:beleg). NULL LLM.

Liest die materialisierten Felder eines Vorjahres-Store (VZ n−1) + überträgt die vorjahr-flagged Felder
als VORSCHLAG in den neuen Store (VZ n): herkunft=vorjahr, zustand=vorlaeufig, signal_2=null. Der
Store-Guard (store.append_event, ^import:vorjahr) erzwingt das strukturell — eine Vorjahres-Übernahme
bestätigt NIE direkt (K2/fail-closed: bewegt keine Steuer-Zahl, bis der Mensch sie im neuen VZ neben dem
Vorjahres-Wert bestätigt/aktualisiert; die Ringe rechnen nur über den bestaetigt-Kegel via meet_zustand).

FAIL-CLOSED-DEFAULT: nur Felder mit Bindungs-Flag vorjahr ∈ {uebernehmbar, vorschlag} werden übertragen —
ein Feld OHNE Flag wird NICHT übertragen (kein stiller Fehl-Transfer). uebernehmbar = Stammdaten/kohorten-
fix (direkter Vorschlag), vorschlag = jahres-spezifischer Betrag (Nutzer aktualisiert). Nur BESTÄTIGTE
Vorjahres-Werte werden übernommen (ein vorlaeufiger Vorjahres-Wert ist kein belastbarer Vorschlag).

signal_1 = {typ: "vorjahr", vz: n−1, quell_feld_id, quell_wert, kategorie} — die Justification-Kante zeigt
„aus VZ 2024: 40.000 €".
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "store"))
import store as ST   # noqa: E402


def uebertragbare_felder(bindung: dict) -> dict:
    """{feld_id -> kategorie} für alle Felder mit gesetztem vorjahr-Flag (uebernehmbar|vorschlag)."""
    return {fid: b["vorjahr"] for fid, b in bindung.items() if b.get("vorjahr")}


def uebernehme_vorjahr(neuer_store: dict, vorjahr_felder: dict, bindung: dict, *,
                       vorjahr_vz: int, ts: str | None = None) -> int:
    """Überträgt je vorjahr-flagged Feld mit BESTÄTIGTEM Vorjahres-Wert ein vorlaeufiges Event in
    neuer_store. vorjahr_felder = materialisierte felder-Ebene (feld_id -> {wert, zustand, herkunft}) des
    Vorjahres-Store. Gibt die Anzahl übertragener Felder zurück. Überträgt NICHT, wenn im neuen Store schon
    ein aktives Event für das Feld liegt (One-Active-Event bleibt gewahrt — der Nutzer/Beleg hat Vorrang)."""
    flags = uebertragbare_felder(bindung)
    aktiv = set(ST._aktives(neuer_store))            # feld_ids mit schon aktivem Event (nicht überschreiben)
    n = 0
    for fid, kat in flags.items():
        vf = vorjahr_felder.get(fid)
        if vf is None or vf.get("zustand") != "bestaetigt":
            continue                                  # nur bestätigte Vorjahres-Werte sind belastbar
        if fid in aktiv:
            continue                                  # neuer Store hat schon einen Wert -> nicht überschreiben
        wert = vf["wert"]
        sig1 = {"typ": "vorjahr", "vz": vorjahr_vz, "quell_feld_id": fid,
                "quell_wert": wert, "kategorie": kat}
        ST.append_event(neuer_store, feld_id=fid, wert=wert, zustand="vorlaeufig",
                        herkunft={"herkunft": "vorjahr", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="import:vorjahr",
                        signal={"signal_1": sig1, "signal_2": None}, ts=ts,
                        # Auflage T (Stille-Null-Klasse): gerade Vorjahres-Altbestände können vor
                        # dieser Pruefung geschrieben worden sein — bindung liegt hier schon vor
                        # (uebertragbare_felder() braucht sie ohnehin), also mitgeben statt Luecke lassen.
                        bindung=bindung)
        n += 1
    # Verlustvortrag traegt bewusst kein vorjahr-Flag (s. Docstring oben), deshalb separat: eine
    # reine Vergleichsgroesse fuer preflight.plausibilitaets_widersprueche(), kein Formular-
    # Vorschlag (BACKLOG verlustvortrag-ungeprueft-uebernommen.md).
    ref = referenzwert_verlustvortrag(vorjahr_felder)
    if ref is not None:
        neuer_store["vorjahr_referenz"] = {"verlustvortrag_bestand": ref}
    return n


def referenzwert_verlustvortrag(vorjahr_felder: dict) -> dict | None:
    """Der im Vorjahres-Fall BESTÄTIGTE `verlustvortrag_bestand` als reine Vergleichsgröße für
    preflight.plausibilitaets_widersprueche() — KEIN Vorschlag: das Feld trägt bewusst kein
    vorjahr-Flag (bindung_an_gesamt.yaml, tests/test_vorjahr_writer.py::
    test_bestand_felder_tragen_kein_vorjahr_flag), weil ein wörtlicher Übertrag falsch wäre —
    der Bestand ändert sich durch Verrechnung im Vorjahr. Der Nutzer trägt den neuen Wert
    also selbst ein; diese Funktion liefert nur den alten Wert zum Gegenprüfen.

    None, wenn im Vorjahres-Fall kein bestätigter Wert vorliegt (dann bleibt die Prüfung still,
    wie bei jeder anderen fehlenden Bezugsgröße in preflight.py)."""
    vf = vorjahr_felder.get("verlustvortrag_bestand")
    if vf is None or vf.get("zustand") != "bestaetigt":
        return None
    return {"wert": vf["wert"]}
