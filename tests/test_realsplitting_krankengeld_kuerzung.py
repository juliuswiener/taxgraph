"""Die 4-Prozent-Kürzung beim Realsplitting — Regel, Spiegelung und Naht.

§ 10 Abs. 1a Nr. 1 S. 2 EStG erhöht den 13.805-Euro-Deckel um die Beiträge "nach Absatz 1
Nummer 3". Und § 10 Abs. 1 Nr. 3 Buchst. a S. 4 sagt zu genau diesen Beiträgen: "Wenn sich aus
den Krankenversicherungsbeiträgen nach Satz 2 ein Anspruch auf Krankengeld … ergeben kann, ist
der jeweilige Beitrag um 4 Prozent zu vermindern". Die Verweisung zieht die Kürzung mit — der
Deckel erhöht sich also um die GEKÜRZTEN Beiträge.

Bis zum 2026-08-16 rechnete die Regel ungekürzt: Deckel zu hoch, Abzug ggf. zu groß, Steuer zu
niedrig. Das ist die gefährliche Richtung — eine abgelehnte Erklärung ist laut, eine falsche Zahl
ist still.

Drei Ebenen, die hier zusammengehalten werden, weil der Fehler an jeder einzelnen wieder
entstehen könnte:

  1. Die REGEL (pipeline/produktion/rules.yaml + Snapshot): trägt die Kürzung, die Seeds und die
     Provenance des Hand-Fixes.
  2. Die SPIEGELUNG (golden/runner.py): muss dieselben Zahlen liefern wie das kompilierte Catala.
     Sie ist von Hand geschrieben — genau die Stelle, an der eine Regeländerung sonst hängen
     bleibt.
  3. Die NAHT (produkt/haut/api.py): muss den neuen Slot auch übergeben. Ein Slot, den niemand
     füllt, ist stumm — dieselbe Bauart wie das Kegel-Feld, das 2026-08-07 351 Euro kostete.

NULL LLM: die Seed-Erwartungen stammen aus rules.yaml und wurden dort vom clerk-Gate gegen das
kompilierte Catala gefahren; hier wird die Python-Seite dagegen gehalten.
"""
from __future__ import annotations

import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("golden", "produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import runner  # noqa: E402
import traverser as TR  # noqa: E402

RULES = os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")
SNAPSHOT = os.path.join(ROOT, "pipeline", "snapshots", "p10_1a_realsplitting.json")


def _regel() -> dict:
    with open(RULES, encoding="utf-8") as f:
        doc = yaml.safe_load(f)
    return next(r for r in doc["regeln"] if r["rule_id"] == "p10_1a_realsplitting")


# ---------------------------------------------------------------- 1. die Regel
def test_regel_kennt_die_kuerzung():
    """Signatur-Input, Geltungsbedingung und Provenance — die drei Spuren des Hand-Fixes."""
    r = _regel()
    assert "kv_krankengeld" in r["signature"]["inputs"], (
        "Der Eingang für den Krankengeld-Anteil fehlt in der Signatur.")
    assert "kuerzung_krankengeld_4_prozent" in {b["bedingung"] for b in r["geltungsbedingungen"]}
    assert r.get("hand_fix"), (
        "Die Provenance fehlt. Das judge_verdict im Snapshot stammt vom ursprünglichen Lauf und "
        "gilt der Fassung OHNE Kürzung — ohne diesen Vermerk sähe die Regel modellverifiziert aus.")


def test_snapshot_traegt_die_gekuerzte_fassung():
    """Der Snapshot ist der Vertrauensanker. Trägt er die alte Fassung, fällt der Fix beim
    nächsten Regate still zurück — genau der Grund, warum ein Fix allein in runner.py nichts
    wert gewesen wäre."""
    import json
    d = json.load(open(SNAPSHOT, encoding="utf-8"))
    assert "kv_krankengeld" in d["catala_a"], "catala_a kennt die Kürzung nicht"
    assert "kv_krankengeld" in d["catala_b"], "catala_b kennt die Kürzung nicht"
    import hashlib
    assert hashlib.sha256(d["catala_a"].encode()).hexdigest() == d["catala_a_sha256"], (
        "sha256 passt nicht zu catala_a — der Snapshot würde beim Laden hart failen.")


# ---------------------------------------------------------------- 2. die Spiegelung
def test_python_spiegelung_trifft_jeden_seed():
    """Die Seeds stehen in rules.yaml und wurden dort vom clerk-Gate gegen das KOMPILIERTE
    Catala gefahren (6 seed tests passed). Hier wird die handgeschriebene Python-Spiegelung
    gegen dieselben Erwartungen gehalten.

    Der Test liest die Seeds aus der Regel statt sie zu kopieren: eine kopierte Erwartung
    veraltet still, sobald jemand die Regel ändert."""
    seeds = _regel()["test_seed"]
    assert len(seeds) >= 6, f"Nur {len(seeds)} Seeds — die differenzierenden fehlen."
    for s in seeds:
        got = runner.catala_p10_1a_realsplitting(dict(s["inputs"]))
        assert got == int(s["expected"]), (
            f"Spiegelung weicht ab bei {s['inputs']}: {got} statt {int(s['expected'])}\n"
            f"  {s['rechenweg']}")


def test_kuerzung_wirkt_nur_ueber_dem_deckel():
    """Die Kürzung senkt den DECKEL, nicht den Abzug. Wer unter dem Deckel bleibt, merkt sie
    nicht — sonst würde die Änderung Fälle bewegen, die das Gesetz gar nicht betrifft."""
    f = runner.catala_p10_1a_realsplitting
    unter = {"unterhaltsleistungen": 5000, "kv_pv_beitraege": 2000, "kv_krankengeld": 2000}
    assert f(unter) == 5000
    assert f(dict(unter, kv_krankengeld=0)) == 5000


def test_ohne_krankengeld_anteil_unveraendert():
    """Rückwärtskompatibilität: fehlt der Wert ganz (Altfall, Vorjahr, unbeantwortet), rechnet
    die Regel wie vorher. Ein KeyError hier wäre ein Absturz statt einer Steuer."""
    f = runner.catala_p10_1a_realsplitting
    assert f({"unterhaltsleistungen": 15000, "kv_pv_beitraege": 2000}) == 15000
    assert f({"unterhaltsleistungen": 20000, "kv_pv_beitraege": 2000}) == 15805


def test_kuerzung_wird_aufgerundet():
    """Der Accessor rechnet in ganzen Euro (api.py übergibt Cent//100), Catala in Cent. Bei
    krummen Beträgen weichen beide um bis zu einen Euro ab. Aufrunden der Kürzung senkt den
    Deckel — die vorsichtige Richtung. Abrunden hieße: Deckel höher, Abzug größer, Steuer
    niedriger."""
    f = runner.catala_p10_1a_realsplitting
    # 999 * 4 % = 39,96 -> aufgerundet 40
    assert f({"unterhaltsleistungen": 99999, "kv_pv_beitraege": 999, "kv_krankengeld": 999}) \
        == 13805 + 999 - 40


# ---------------------------------------------------------------- 3. die Naht
def test_feld_ist_an_den_slot_gebunden():
    """Ohne signatur_slot erreicht der erfasste Wert die Regel nie — das Feld wäre reine
    Deklaration und die Kürzung im Betrieb immer 0. Genau diese Naht (Feld erfasst, Slot nicht
    verdrahtet) hat hier schon mehrfach Geld gekostet."""
    b = TR.lade_bindung()["realsplitting_empfaenger_kv_krankengeld"]
    assert b["quelle"].get("signatur_slot") == "kv_krankengeld", (
        f"Feld nicht an den Rechen-Slot gebunden: {b['quelle']}")
    assert b.get("slot_beitrag") != "summand", (
        "Der Krankengeld-Anteil ist eine TEILMENGE der KV/PV-Beiträge, kein zweiter Summand — "
        "als summand würde er den Deckel erhöhen statt ihn zu senken.")


def test_api_uebergibt_den_slot():
    """Die Naht selbst: api.py muss den Wert auch mitgeben. Ein Slot, den niemand füllt, ist
    stumm — und das fällt in keiner Ring-Rechnung auf, weil 0 ein gültiger Wert ist."""
    # Das VERZEICHNIS lesen, nicht einzelne Dateien aufzählen.
    #
    # Am 2026-08-18 zog der Rechenkern von api.py nach produkt/bescheid/bescheid.py, und dieser
    # Scanner bekam eine feste Zweierliste — mit dem Kommentar, ein fester Pfad wäre nach dem
    # Umzug stumm gewesen. Am 2026-08-19 wurde bescheid.py in vier Module geteilt, und genau
    # diese Zweierliste ging ins Leere. Dritter Scanner, dieselbe Lehre, drittes Mal.
    #
    # Die Menge wächst jetzt mit dem Verzeichnis: ein weiterer Schnitt darin ändert nichts.
    quelle = ""
    _pfade = [os.path.join(ROOT, "produkt", "haut", "api.py")]
    _kern = os.path.join(ROOT, "produkt", "bescheid")
    if os.path.isdir(_kern):
        _pfade += sorted(os.path.join(_kern, n) for n in os.listdir(_kern) if n.endswith(".py"))
    for _p in _pfade:
        if os.path.exists(_p):
            quelle += open(_p, encoding="utf-8").read()
    i = quelle.find("catala_p10_1a_realsplitting")
    assert i > 0, ("catala_p10_1a_realsplitting in keiner Quelldatei gefunden — die Regel wird "
                   "nirgends mehr gerufen, oder der Rechenkern ist erneut umgezogen.")
    aufruf = quelle[i:i + 900]
    assert '"kv_krankengeld"' in aufruf, (
        "api.py ruft die Regel ohne den Krankengeld-Slot auf — die Kürzung bliebe im Betrieb "
        "immer 0, obwohl die Regel sie kennt.")
    assert "realsplitting_empfaenger_kv_krankengeld" in aufruf, (
        "Der Slot wird nicht aus dem erfassten Feld gefüllt.")
