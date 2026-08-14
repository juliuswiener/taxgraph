"""Die vier Einkunftsart-Screeningfragen dürfen sich nicht gegenseitig abschneiden.

Befund 2026-08-14: `kein_gewinn`, `kein_kap`, `kein_vuv` und `kein_sonstige` hingen alle an EINER
Regel (`p2_einkunftsarten`). traverser.relevanz() behandelt die Gates einer Regel konjunktiv —
ein bestätigtes False schließt die ganze Regel aus und bricht die Gate-Schleife ab
(traverser.py:118-124). Gemessen:

    kein_gewinn=False  ->  p2_einkunftsarten ausgeschlossen
                       ->  kein_kap / kein_vuv / kein_sonstige werden NIE mehr gestellt

Wen es trifft: die Felder sind negativ benannt, die Frage in der UI ist positiv gestellt, und
app.js:283 invertiert beim Speichern (`feld_id.startsWith("kein_") ? !ja : ja`). Wer also
"Ja, ich hatte Gewerbeeinkünfte" anklickt, legt kein_gewinn=False ab — und verliert damit die
Fragen nach Kapitalerträgen, Vermietung und sonstigen Einkünften. Betroffen ist der KOMPLEXERE
Fall, nicht der reine Arbeitnehmer: wer alle vier verneint, speichert überall True und löst
keinen Ausschluss aus. Genau deshalb fiel es bisher niemandem auf — der Standardfall läuft
daran vorbei.

Dieselbe Klasse wie der Dialog-Killer vom 2026-08-12 (`pv_auf_gebaeude` am Sammel-Scope
p2_festzusetzung_einzel), aber andere Ursache: dort hing EIN Feld an einer fremden Regel, hier
hängen VIER unabhängige Fragen an derselben. Konjunktive Gates sind für Tatbestandsmerkmale
richtig (§ 35a: bar bezahlt -> kein Abzug, alle Detailfragen entfallen zu Recht) und für
unabhängige Screeningfragen falsch.

Fix: jede Screeningfrage bekommt ihre eigene regel_id und ist dort das einzige Gate.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API          # noqa: E402
import store as ST         # noqa: E402
import traverser as TR     # noqa: E402

SCREENING = ("kein_gewinn", "kein_kap", "kein_vuv", "kein_sonstige")


def _fall(**setz):
    store = ST.leerer_store(2025, fall_id="screening")
    store["scheibe"] = "gesamt"
    for k, v in setz.items():
        ST.append_event(store=store, feld_id=k, wert=v, zustand="bestaetigt",
                        herkunft={"quelle": "test"}, schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"ok@{k}"},
                        ts="2026-08-14T10:00:00Z")
    return store, API._scheibe_bindung(store)


@pytest.mark.parametrize("beantwortet", SCREENING)
def test_eine_verneinte_screeningfrage_schneidet_die_anderen_nicht_ab(beantwortet):
    """Kern: eine mit Nein beantwortete Screeningfrage darf die übrigen drei nicht aus der
    Dialog-Queue nehmen. Sie sind fachlich unabhängig — wer kein Gewerbe hat, kann trotzdem
    Kapitalerträge haben."""
    store, bindung = _fall(**{beantwortet: False})
    fragen = TR.naechste_fragen(store, bindung)
    fehlend = [f for f in SCREENING if f != beantwortet and f not in fragen]
    assert not fehlend, (
        f"{beantwortet}=False nimmt {fehlend} aus dem Dialog — der Nutzer kann diese "
        f"Einkunftsarten nicht mehr angeben.")


@pytest.mark.parametrize("beantwortet", SCREENING)
def test_jede_screeningfrage_hat_ihre_eigene_regel(beantwortet):
    """Strukturgate gegen den Rückbau: sobald zwei Screeningfragen wieder auf dieselbe regel_id
    zeigen, greift die konjunktive Gate-Auswertung erneut und der Test oben wird rot — aber erst
    zur Laufzeit. Dieser hier zeigt die Ursache direkt."""
    yaml = pytest.importorskip("yaml")
    import glob

    regel_je_feld = {}
    for fp in sorted(glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml"))):
        for b in yaml.safe_load(open(fp)).get("bindungen", []):
            if b["feld_id"] in SCREENING:
                regel_je_feld[b["feld_id"]] = b["quelle"]["regel_id"]

    eigene = regel_je_feld[beantwortet]
    kollisionen = [f for f, r in regel_je_feld.items() if r == eigene and f != beantwortet]
    assert not kollisionen, (
        f"{beantwortet} teilt sich die Regel {eigene!r} mit {kollisionen} — ein Nein bei einer "
        f"der Fragen schließt die Regel aus und nimmt die anderen mit.")


def test_p35a_screening_nimmt_alle_neun_detailfragen_aus_dem_dialog():
    """§ 35a-Top-Level-Gate (Julius-Entscheid 2026-08-14).

    Die drei vorhandenen bool-Gates der Regel (rechnung_unbar, in_eu_ewr, keine_foerderung) sind
    TATBESTANDSMERKMALE — sie prüfen die Bedingungen des Abzugs. Es fehlte die vorgelagerte
    Ob-Frage: wer nie einen Handwerker im Haus hatte, wurde trotzdem durch alle neun Felder
    geführt. Anders als bei den vier Einkunftsart-Fragen oben ist der konjunktive Ausschluss hier
    RICHTIG: fällt die Ob-Frage weg, ist auch jedes Tatbestandsmerkmal gegenstandslos.

    hh_hat_aufwendungen trägt bewusst KEIN "kein_"-Präfix — app.js:283 invertiert bool-Antworten
    genau bei diesem Präfix, und eine stille Inversion sieht man der Bindung nicht an."""
    import api_constants as AC

    hh = list(AC.HAUSHALT_35A) + ["hh_rechnung_unbar"]
    store, bindung = _fall()
    offen_vorher = [f for f in TR.naechste_fragen(store, bindung) if f in hh]
    assert len(offen_vorher) >= 9, (
        f"Erwartet mindestens 9 offene § 35a-Fragen, gefunden {len(offen_vorher)} — der Test "
        f"misst sonst nichts.")

    store, bindung = _fall(hh_hat_aufwendungen=False)
    offen_nachher = [f for f in TR.naechste_fragen(store, bindung) if f in hh]
    assert not offen_nachher, (
        f"Nach 'keine haushaltsnahen Aufwendungen' stehen weiter {offen_nachher} im Dialog.")

    # Gegenrichtung: "ja" darf nichts abschneiden, sonst wäre das Gate ein Sackgassen-Filter
    store, bindung = _fall(hh_hat_aufwendungen=True)
    offen_ja = [f for f in TR.naechste_fragen(store, bindung) if f in hh]
    assert len(offen_ja) >= 9, (
        f"Nach 'ja' sind nur noch {len(offen_ja)} § 35a-Fragen offen — das Gate schneidet die "
        f"Detailfragen ab, die es freigeben soll.")


def test_verneinte_frage_schaltet_ihre_eigene_einkunftsart_nicht_still_ab():
    """Gegenrichtung, damit der Fix nicht zu viel tut: das Screening ist heute eine reine Frage,
    es hängt noch KEINE Zielregel daran (das ist Gruppe B der Vorsortierung, separat). Wenn
    jemand später regel_bedingungen ergänzt, muss er das bewusst tun — dieser Test hält fest,
    was heute gilt, statt es stillschweigend mitzuändern."""
    store, bindung = _fall(kein_gewinn=False)
    rel = TR.relevanz(store, bindung)
    for ziel in ("p4_3_gewinn", "p15_1_2_mitunternehmer"):
        if ziel in rel:
            assert rel[ziel]["status"] != "ausgeschlossen", (
                f"{ziel} wird durch das Screening ausgeschlossen — das ist Gruppe B und "
                f"gehört bewusst konfiguriert, nicht als Nebenwirkung dieses Fixes.")
