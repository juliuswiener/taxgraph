"""Der Doppelquellen-Wächter und der EÜR-Umschalter müssen DIESELBE Menge prüfen.

DER DEFEKT (gemessen 2026-09-26). `_laufender_gewinn` schaltet auf den EÜR-Zweig um,
sobald eine EÜR-Komponente **oder ein GWG** vorliegt:

    bescheid_einkuenfte.py:303
        if any(_c(k) for k in EUER_KOMPONENTEN) or gwg_summe > 0:
    bescheid_einkuenfte.py:308   (nur im else-Zweig)
        gewinn = _c("einkuenfte_gewinn") // 100 + mitu

Der Wächter, der genau diese Doppelquelle abfangen soll, kennt GWG aber nicht:

    bescheid_deklaration.py:1000
        if _positiv("einkuenfte_gewinn") and any(_positiv(k) for k in EUER_KOMPONENTEN):
            return "gewinn_quelle_offen"

`EUER_KOMPONENTEN` = `("betriebseinnahmen", "sonstige_betriebsausgaben", "afa_jahresbetrag")`
— `gwg_anschaffungskosten_netto` steht NICHT darin (`api_constants.py:353`).

Die Umschaltbedingung ist damit WEITER als die Wächterbedingung. Eine einzige GWG-Zeile
schaltet den Pfad um, ohne dass der Wächter zuständig wird: der Direktwert fällt lautlos
aus der Bemessung, übrig bleibt der EÜR-Zweig mit Einnahmen 0 und der GWG-Ausgabe als
einziger Betriebsausgabe. Am Fall gemessen: Direktwert 50.000 EUR + GWG 600 EUR ergab
**1.369.300 ct** festzusetzende Steuer statt der Sperre — der Direktwert war auf −600 EUR
gefallen, **21.063,00 EUR Steuer auf 50.000 EUR Gewinn verschwanden still**.

WARUM DREI ZELLEN UND NICHT EINE. Eine Zelle allein zeigt nur, dass sich etwas ändert.
Erst die zwei Gegenproben trennen „Wächter erweitert" von „Pfad verschoben": nur GWG
(ohne Direktwert) und nur Direktwert dürfen sich NICHT ändern. Ohne sie wäre ein Fix,
der schlicht jeden GWG-Fall sperrt, von der richtigen Reparatur nicht zu unterscheiden
([[pruefer-misst-stellvertretermerkmal]]).

Der Kegel kommt aus `tests/_kegel.py`, nicht als Handkopie — eine Handliste sperrte auf
`input_kegel_nicht_bestaetigt`, sobald `agb_zwangslaeufig`/`agb_notwendig_angemessen` in
den Kegel aufgenommen wurden (genau der Anlass, für den der Bauer gebaut ist).

NULL LLM.
"""
from __future__ import annotations

import os
import sys
import tempfile

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("haut", "store", "traverser", "eingang", "mapping", "bescheid", "engine"):
    _p = os.path.join(ROOT, "produkt", _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)
if HERE not in sys.path:
    sys.path.insert(0, HERE)

os.environ.setdefault("TAXGRAPH_NO_AUTH", "1")

import _kegel          # noqa: E402
import api as API      # noqa: E402
import audit           # noqa: E402


def _laie(feld_id, wert):
    return {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": None, "signal_2": f"ok@{feld_id}"}}


# Der Betrieb: Bruttolohn 60.000 (Grenzsatz-Zone), Direktwert 50.000 aus Gewerbe.
# `kein_gewinn=False` ist Pflicht — mit True widerspricht der Direktwert dem Flag
# (FLAG_NEGIERT["kein_gewinn"] führt `einkuenfte_gewinn`) und der Fall sperrt vorher
# mit `flag_konsistenz_offen`, sodass der Messgegenstand nie erreicht wird.
DIREKTWERT = {"bruttoarbeitslohn": 6000000, "kein_gewinn": False,
              "einkuenfte_gewinn": 5000000, "gewinn_betriebsart": "gewerbe"}

# 600 EUR netto: über der 250-EUR-Grenze, also greift die Verzeichnis-Frage,
# unter 800 EUR, also Sofortabzug statt AfA.
GWG = {"gwg_anschaffungskosten_netto": 60000,
       "gwg_bewegliches_selbstaendig_nutzbar": True,
       "gwg_netto_ohne_vorsteuer": True,
       "gwg_verzeichnis_ab_250": True}

# Gemessen auf 206117b, VOR dem Fix (Sonde station1.py):
#   Direktwert + GWG -> 1369300 ct, grund='bestaetigt'   <- der Defekt
#   nur Direktwert   -> 3475600 ct, grund='bestaetigt'   <- muss so bleiben
ZELLE_VOR_DEM_FIX = 1369300
ZELLE_NUR_DIREKTWERT = 3475600


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    n = [0]

    def _bauen(gesetzt):
        n[0] += 1
        fid = f"gwq_{n[0]}"
        st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
        assert st == 201, r
        # Der Kegel-Bauer füllt, was `gesetzt` nicht nennt; was er nennt, gewinnt.
        for feld_id, wert in _kegel.kegel_fuer("gesamt", gesetzt):
            st, r = API.event(fid, _laie(feld_id, wert))
            assert st == 201, f"{feld_id}={wert!r}: HTTP {st} {r}"
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        return erg

    return _bauen


def test_direktwert_und_gwg_sperren_statt_still_zu_rechnen(fall):
    """DER ROTE FALL. Beide Quellen gesetzt → der Fall muss sperren, nicht rechnen.

    Nicht `!= None` als Erwartung, sondern der Grund selbst: `gewinn_quelle_offen`.
    Eine andere Zahl als gesperrt zu liefern hieße, der Direktwert fällt weiter heraus;
    ein ANDERER Sperrgrund hieße, der Fall erreicht den Doppelquellen-Wächter nicht.
    """
    erg = fall({**DIREKTWERT, **GWG})
    assert erg["zahl_cent"] is None, (
        f"Direktwert 50.000 + GWG 600 rechnet weiter: zahl_cent={erg['zahl_cent']} "
        f"statt None. Gemessen vor dem Fix: {ZELLE_VOR_DEM_FIX} ct — der Direktwert ist "
        f"aus der Bemessung gefallen, 21.063,00 EUR Steuer auf 50.000 EUR Gewinn "
        f"verschwinden still.")
    assert erg["grund"] == "gewinn_quelle_offen", (
        f"erwartet 'gewinn_quelle_offen' (Doppelquelle Direktwert + GWG), "
        f"bekommen {erg['grund']!r}")


def test_nur_gwg_rechnet_weiter(fall):
    """GEGENPROBE 1: GWG ohne Direktwert bleibt rechenbar.

    Ohne diese Zelle wäre ein Fix, der jeden GWG-Fall sperrt, vom richtigen nicht zu
    unterscheiden — und der Nutzer, der nur eine EÜR-Aufstellung eingibt, bekäme keinen
    Bescheid mehr.
    """
    erg = fall({**GWG, "kein_gewinn": False, "betriebseinnahmen": 5000000,
                "sonstige_betriebsausgaben": 0, "afa_jahresbetrag": 0})
    assert erg["grund"] == "bestaetigt", (
        f"nur GWG (EÜR-Weg, kein Direktwert) muss rechnen — grund={erg['grund']!r}")
    assert erg["zahl_cent"] is not None, "nur GWG darf keine Sperre auslösen"


def test_nur_direktwert_rechnet_unveraendert(fall):
    """GEGENPROBE 2: der reine Direktwert-Fall rechnet wie vor dem Fix.

    Mit der Zahl, nicht nur mit `is not None`: eine Sperre wäre grün durch `is not None`
    hindurch, ein verschobener Betrag ebenso ([[test-prueft-nicht-was-er-behauptet]]).
    """
    erg = fall(dict(DIREKTWERT))
    assert erg["grund"] == "bestaetigt", f"grund={erg['grund']!r}"
    assert erg["zahl_cent"] == ZELLE_NUR_DIREKTWERT, (
        f"der reine Direktwert-Fall hat sich verschoben: {erg['zahl_cent']} statt "
        f"{ZELLE_NUR_DIREKTWERT}")


def test_der_umschalter_zieht_aus_derselben_menge():
    """DIE WURZEL, als Zusicherung: beide Stellen prüfen dieselbe Menge.

    Ohne diese Zelle kann die Reparatur an EINER der zwei Stellen stehen und beim nächsten
    aufgenommenen Feld wieder auseinanderlaufen — genau der Defekt, nur später
    ([[geltungsbereich-ungleich-verwendung]]: zwei Listen für eine Frage).

    Geprüft wird DREIFACH, weil eine Zusicherung auf die Konstante allein die Lücke nicht
    schließt:
      1. GWG ist in der gemeinsamen Menge (sonst feuert der Wächter nicht),
      2. die gemeinsame Menge UMFASST EUER_KOMPONENTEN (sonst hätte der Fix etwas entfernt),
      3. BEIDE Aufrufstellen nennen sie — keine führt noch eine eigene Liste. Das ist die
         Zelle gegen die Rückkehr: wer die zweite Stelle zurückdreht, wird hier rot.
    """
    import api_constants as C
    assert "gwg_anschaffungskosten_netto" in C.GEWINN_QUELLEN_MENGEN, (
        "gwg_anschaffungskosten_netto fehlt in GEWINN_QUELLEN_MENGEN — dann ist die "
        "Umschaltbedingung wieder weiter als die Wächterbedingung")
    for k in C.EUER_KOMPONENTEN:
        assert k in C.GEWINN_QUELLEN_MENGEN, (
            f"{k} ist aus der gemeinsamen Menge verschwunden — der Fix hat etwas entfernt "
            f"statt nur zu erweitern")
    import bescheid_einkuenfte as BE
    import bescheid_deklaration as BD
    for modul, name in ((BE, "bescheid_einkuenfte.py"), (BD, "bescheid_deklaration.py")):
        quelle = open(modul.__file__, encoding="utf-8").read()
        assert "GEWINN_QUELLEN_MENGEN" in quelle, (
            f"{name} nennt die gemeinsame Menge nicht mehr — dann prüft diese Stelle "
            f"wieder eine eigene Liste")
