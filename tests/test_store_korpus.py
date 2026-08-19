"""Store-Korpus-Gate: haelt jede feld_id jeder ECHTEN Falldatei gegen die Bindungstabelle
(Architektur-Audit 2026-08-16).

Befund, der dieses Gate ausloeste: tests/test_store.py::test_e_feld_id_in_bindung prueft nur
feld_ids, die das `sample`-Fixture SELBST im selben Testlauf per ST.append_event erzeugt — mit
hartkodierten, immer aktuellen feld_ids. Gruen per Konstruktion, kann strukturell nie anschlagen
(s. Docstring dort). Gemessen am echten Bestand (2026-08-16, vor der Loeschung der 313 Fall-
dateien): 739 verwaiste Events in 267 von 313 Falldateien — Werte, deren feld_id bei einer
Umbenennung aus der Bindung verschwand, aber im Store liegen blieb (stiller Datenverlust: die
Zahl steckt weiterhin im materialisierten Snapshot, taucht aber in keiner Deklaration mehr auf,
weil est_mapping ihre feld_id nicht mehr kennt).

Dieses Gate liest den ECHTEN Korpus statt eines selbstgebauten Fixtures. Den Speicherort holt
es sich aus api_constants (seit dem XDG-Umzug am 2026-08-19 nicht mehr produkt/haut/faelle) —
und zwar BEIDE Orte, s. Kommentar bei FAELLE_DIRS.

Instanz-Achse (Klasse INSTANZ, produkt/mapping/est_mapping.py): ein wiederholbares Anlage-Feld
traegt fuer Instanz 2..N das Suffix __<n> (z.B. vv_einnahmen__2). Die Bindung kennt nur die
Basis-feld_id (Instanz-Reuse derselben Kz) — ohne parse_instanz() waere JEDE Instanz ab 2 ein
falscher Verwaisungs-Fund. Beide Seiten (Deklaration UND dieses Gate) muessen dieselbe Funktion
rufen, sonst drifted die Enumeration wieder auseinander (genau die Garantie, die parse_instanz's
eigener Docstring verspricht).

Leere-Menge-Falle: der Korpus ist HEUTE leer — an beiden Orten. Die 313 Falldateien liegen in
den Sicherungen unter ../taxgraph-backups/ (Stand 2026-08-17 12:37, zwei byte-gleiche Archive);
ob der leere Live-Store Absicht ist oder ein Umzugsverlust, weiss nur Julius und NICHT dieser
Test. Ein Gate, das ueber 0 Dateien iteriert, ist immer gruen — nicht weil der Bestand sauber
ist, sondern weil niemand hingesehen hat.

GEMESSEN AM SICHERUNGSBESTAND (2026-08-19, read-only, nichts eingespielt), damit die Zahl unten
nicht wieder aus dem Gedaechtnis kommt: 313 Dateien, 8084 Events, davon 695 verwaist in 267
Dateien — verteilt auf nur SECHS feld_ids, und die zerfallen in drei verschiedene Faelle:
  * kap_zusammenveranlagung (158) — bewusst gestrichenes Doppel-Feld (c8eec39, "Veranlagungsart
    hat eine Quelle"); alle Werte leer. Nichts zu retten, nichts verloren.
  * kap_ertraege (1) — hat mit kap_kapitalertraege einen benannten Nachfolger: mechanisch.
  * basis_kv_pv(_partner) (268) — wurde in basis_kv + basis_pv AUFGETEILT. Der Altwert ist eine
    Summe, die Aufteilung steckt nicht in den Daten: eine Migration kann das NICHT mechanisch,
    ohne zu raten.
  * weitere_vorsorgeaufwendungen(_partner) (268) — ersatzlos, kein Nachfolger in der Bindung.
Alle 313 Namen sind Kuerzel nach Paragraph/Merkmal (f31f, p35k, vpf, zbgs_ohne), Median 29
Events — Entwicklungsmaterial, keine Personenfaelle. Das ist der Grund, warum hier KEINE
Reconciliation gebaut wurde: sie wuerde 268 Summen fuer Wegwerf-Faelle raten. Sobald ein echter
Fall entsteht, ist diese Abwaegung neu zu treffen — dann aber mit echtem Anlass. Das ist dieselbe Krankheit wie das Fixture-Gate oben,
nur mit umgekehrtem Vorzeichen: dort wurden nie echte Daten geprueft, weil sie synthetisch waren;
hier wuerden nie echte Daten geprueft, weil es (gerade) keine gibt. Deshalb: 0 Falldateien ->
SKIP mit explizitem Grund (kein stilles PASS, das wie "Bestand sauber" aussieht). Sobald die
erste echte Falldatei entsteht, wird dieses Gate von selbst scharf — der Glob unten kennt keine
Dateizahl und keine Whitelist, die jemand nachfuehren muesste.
"""
from __future__ import annotations

import glob
import json
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/mapping", "produkt/traverser", "produkt/haut"):
    p = os.path.join(ROOT, sub)
    if p not in sys.path:
        sys.path.insert(0, p)

import traverser as TR                  # noqa: E402
from est_mapping import parse_instanz    # noqa: E402
import api_constants as AK               # noqa: E402

# Der Speicherort wird GELESEN, nicht wiederholt: seit dem XDG-Umzug (2026-08-19) liegen die
# Faelle unter $XDG_DATA_HOME/taxgraph/faelle. Bis dieses Gate nachgezogen wurde, zeigte es
# fest auf produkt/haut/faelle und meldete "Korpus leer: 0 Falldateien" — ehrlich benannt und
# trotzdem wertlos, weil es den echten Bestand nicht mehr sah. Vierter Scanner mit festem Pfad
# innerhalb von zwei Tagen; deshalb hier die Konstante der Haut statt einer eigenen Kopie.
#
# BEIDE Orte, nicht nur der neue: ein Rest im alten Verzeichnis (unvollstaendiger Umzug,
# zurueckgedrehter Umzug, eine aeltere Arbeitskopie) wuerde sonst genau so unsichtbar wie der
# neue Bestand es bis eben war. Ein Gate, das nach einem Umzug nur den Zielort kennt, ist beim
# naechsten Umzug wieder blind.
FAELLE_DIRS = (AK.FAELLE, AK.FAELLE_ALT)


def _fall_dateien() -> list[str]:
    """Echte Faelle: <fall_id>.json direkt unter einem der Speicherorte (kein Unterordner,
    kein .jsonl — das ist audit.jsonl vorbehalten und matcht *.json nicht)."""
    treffer: list[str] = []
    for d in FAELLE_DIRS:
        treffer.extend(glob.glob(os.path.join(d, "*.json")))
    return sorted(treffer)


def test_korpus_feld_id_in_bindung():
    dateien = _fall_dateien()
    if not dateien:
        pytest.skip(
            "Korpus leer: 0 Falldateien (*.json) unter " + " und ".join(FAELLE_DIRS) +
            ". Dieses Gate prueft dann NICHTS — SKIP statt stillem PASS, damit ein leerer "
            "Bestand nicht wie ein sauberer aussieht. Wird automatisch scharf, sobald die "
            "erste echte Falldatei entsteht."
        )

    bindung = TR.lade_bindung()
    geprueft = 0
    verwaist: list[str] = []
    for pfad in dateien:
        with open(pfad, encoding="utf-8") as f:
            store = json.load(f)
        for e in store.get("events", []):
            fid = e["feld_id"]
            geprueft += 1
            if fid in bindung:
                continue
            inst = parse_instanz(fid)          # base__n -> (base, n); sonst None
            if inst is not None and inst[0] in bindung:
                continue                        # Instanz-Reuse: Basis-Kz deckt die Instanz ab
            verwaist.append(f"{os.path.basename(pfad)}: {fid}")

    # Ueberraschungs-Waechter: Falldateien ohne ein einziges Event waeren ein Format-Bruch,
    # kein "nichts zu pruefen" — der obige Leere-Menge-Skip greift nur bei 0 DATEIEN, nicht bei
    # Dateien mit 0 Events. Ein solcher Fund soll auffallen statt als stilles 0/0-PASS zu gelten.
    assert geprueft > 0, (
        f"{len(dateien)} Falldatei(en) unter " + " und ".join(FAELLE_DIRS) + ", aber 0 Events "
        "insgesamt — unerwartetes Store-Format? (Schema-Aenderung? Leere Test-Fixtures im "
        "echten Ordner?)"
    )
    assert not verwaist, (
        f"{len(verwaist)} verwaiste feld_id(s) in {len(dateien)} echten Falldateien "
        f"(aktuelle Bindung hat {len(bindung)} Eintraege) — vermutlich eine Feld-Umbenennung, "
        "die den Store nicht mitgezogen hat:\n" + "\n".join(verwaist[:50]) +
        (f"\n... ({len(verwaist) - 50} weitere)" if len(verwaist) > 50 else "")
    )
