"""§ 35c Abs. 3 S. 2 — die Ermäßigung entfällt GANZ bei Doppelförderung.

Wortlaut: "Die Steuerermäßigung nach Absatz 1 ist ebenfalls nicht zu gewähren, wenn für die
energetischen Maßnahmen eine Steuerbegünstigung nach § 10f oder eine Steuerermäßigung nach § 35a
in Anspruch genommen wird oder es sich um eine öffentlich geförderte Maßnahme handelt, für die
zinsverbilligte Darlehen oder steuerfreie Zuschüsse in Anspruch genommen werden."

Nicht anteilig — ganz.

GEMESSEN 2026-08-16, vor dem Guard: dieselben 20.000 EUR einmal als § 35a-Handwerkerkosten und
einmal als § 35c-Sanierung ergaben 1.200 + 1.400 = 2.600 EUR Entlastung. Zulässig sind höchstens
1.400 (oder 1.200, wenn der Nutzer § 35a wählt). 1.200 EUR zu wenig Steuer — die gefährliche
Richtung, denn eine zu niedrige Steuer fällt niemandem auf, bis das Finanzamt sie korrigiert.

Die Bedingung `keine_10f_35a_oeffentliche_foerderung` stand seit jeher in der adjudizierten Regel.
Sie war nur an keine Frage gebunden, sondern als Lücke geführt ("Komplementär-Guard im Ring,
Folge-Gate") — die Absicht war da, der Bau fehlte. Genau diese Sorte Eintrag ist gefährlich: sie
sieht aus wie eine Entscheidung und ist eine Schuld.

DREI WEGE, alle drei geprüft — denn ein Guard, der nur den Sperrfall kennt, macht das Produkt
unbenutzbar, und einer, der nur den Normalfall kennt, ist wirkungslos:

  unbeantwortet          -> Sperrgrund (fail-closed, keine stille Ermäßigung)
  Doppelförderung ja     -> § 35c entfällt ganz, § 35a bleibt
  getrennte Maßnahmen    -> beides zulässig, beide wirken

NULL LLM.
"""
from __future__ import annotations

import os
import sys
import tempfile

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API          # noqa: E402
import audit               # noqa: E402
import traverser as TR     # noqa: E402

BINDUNG = TR.lade_bindung()

# Vollständiger Arbeitnehmer-Fall, damit der Ring überhaupt bis zu einer Zahl kommt. Die
# § 35a-Voraussetzungen sind gesetzt (Haushalt in EU/EWR, unbare Zahlung, keine öffentliche
# Förderung der Handwerkerleistung) — sonst nullt der § 35a-Zweig und der Vergleich misst nichts.
BASIS = {
    "veranlagung": "einzel", "bruttoarbeitslohn": 6000000,
    "kein_gewinn": True, "kein_kap": True, "kein_vuv": True, "kein_sonstige": True,
    "kein_kind": True,
    "hh_hat_aufwendungen": True, "hh_rechnung_unbar": True,
    "hh_handwerker_keine_foerderung": True, "hh_in_eu_ewr": True,
    "p35c_ist_uebernaechstes_foerderjahr": False,
}
BEIDE_TOEPFE = {"hh_handwerker_arbeitskosten": 2000000,   # 20.000 EUR § 35a
                "p35c_sanierungsaufwendungen": 2000000}   # 20.000 EUR § 35c


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _lauf(felder: dict, name: str, monkeypatch, tmp_path):
    """Fall anlegen, Felder setzen, Kegel auffüllen bis eine Zahl kommt oder ein Grund steht.

    Der Kegel wird iterativ aus `offen` gefüllt: die Scheibe `gesamt` verlangt Dutzende Felder,
    und sie einzeln zu pflegen würde diesen Test bei jeder Kegel-Änderung brechen. Die EXPLIZIT
    gesetzten Felder werden dabei nie überschrieben."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / ("f_" + name)))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / ("f_" + name)))
    fid = "p35c" + name
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    gesetzt = dict(felder)
    fix = set(gesetzt)
    for _ in range(8):
        for f, w in gesetzt.items():
            try:
                API.event(fid, {"feld_id": f, "wert": w, "zustand": "bestaetigt",
                                "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                             "haftung": "nutzer"},
                                "schreiber": "ui:laie",
                                "signal": {"signal_1": None, "signal_2": f"k@{f}"}})
            except Exception:            # noqa: BLE001 — schon gesetzt (Auflage B), egal
                pass
        _, body = API.ergebnis(fid)
        if body.get("zahl_cent") is not None:
            return body["zahl_cent"], None
        offen = [f for f in (body.get("offen") or []) if f not in fix]
        if not offen:
            return None, body.get("grund")
        for f in offen:
            e = BINDUNG.get(f, {})
            t = e.get("typ")
            if t == "bool":
                gesetzt[f] = f.startswith("kein_")
            elif t == "enum":
                gesetzt[f] = (e.get("enum_werte") or ["x"])[0]
            else:
                gesetzt[f] = 0
    return None, body.get("grund")


@pytest.fixture(autouse=True)
def _catala(monkeypatch):
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")


def test_unbeantwortet_sperrt(monkeypatch, tmp_path):
    """Ohne Antwort keine Zahl. Der stille Abzug wäre die Under-tax: wir wüssten nicht, ob die
    Ermäßigung überhaupt zusteht, und würden sie trotzdem gewähren."""
    zahl, grund = _lauf(dict(BASIS, **BEIDE_TOEPFE), "offen", monkeypatch, tmp_path)
    assert zahl is None
    assert grund == "p35c_doppelfoerderung_offen", f"Sperrgrund ist {grund!r}"


def test_doppelfoerderung_streicht_die_ganze_ermaessigung(monkeypatch, tmp_path):
    """Kern des Gesetzes: "nicht zu gewähren" heißt GANZ, nicht anteilig. § 35a bleibt unberührt —
    der Nutzer verliert also nicht beides, sondern behält die Variante, die er gewählt hat."""
    mit_doppel, _ = _lauf(dict(BASIS, p35c_keine_doppelfoerderung=False, **BEIDE_TOEPFE),
                          "doppelt", monkeypatch, tmp_path)
    getrennt, _ = _lauf(dict(BASIS, p35c_keine_doppelfoerderung=True, **BEIDE_TOEPFE),
                        "getrennt", monkeypatch, tmp_path)
    assert mit_doppel is not None and getrennt is not None
    assert mit_doppel > getrennt, (
        "Bei Doppelförderung muss die Steuer HÖHER sein — die § 35c-Ermäßigung entfällt.")
    differenz = (mit_doppel - getrennt) / 100
    assert 1300 <= differenz <= 1500, (
        f"Erwartet ~1.400 EUR (7 % von 20.000, Förderjahr 1), gemessen {differenz:.2f} EUR")


def test_getrennte_massnahmen_bleiben_zulaessig(monkeypatch, tmp_path):
    """Die Gegenrichtung, und sie ist wichtiger als sie klingt: § 35a und § 35c schließen sich
    NICHT generell aus, sondern nur für DIESELBE Maßnahme. Wer das Bad renovieren lässt (§ 35a)
    und eine Wärmepumpe einbaut (§ 35c), bekommt beides. Ein Guard, der hier sperrt, nähme dem
    Nutzer Geld, das ihm zusteht."""
    nur_35a, _ = _lauf(dict(BASIS, p35c_keine_doppelfoerderung=True,
                            hh_handwerker_arbeitskosten=2000000), "n35a", monkeypatch, tmp_path)
    beide, _ = _lauf(dict(BASIS, p35c_keine_doppelfoerderung=True, **BEIDE_TOEPFE),
                     "nbeide", monkeypatch, tmp_path)
    assert nur_35a is not None and beide is not None
    assert beide < nur_35a, (
        "Bei getrennten Maßnahmen muss § 35c zusätzlich entlasten.")


def test_ohne_p35c_aufwand_kein_sperrgrund(monkeypatch, tmp_path):
    """Conditional-mandatory wie beim § 35a-Zwilling: wer gar keine energetische Sanierung
    erklärt, darf die Frage nie zu sehen bekommen — sonst hätte das Screening eine Frage mehr
    für alle, die sie nichts angeht."""
    zahl, grund = _lauf(dict(BASIS, hh_handwerker_arbeitskosten=2000000),
                        "ohne35c", monkeypatch, tmp_path)
    assert zahl is not None, f"Unerwartet gesperrt: {grund}"


def test_feld_haengt_an_der_norm_bedingung():
    """Die Bindung selbst: das Feld muss an `keine_10f_35a_oeffentliche_foerderung` hängen, die
    Bedingung der adjudizierten Regel. Hinge es woanders, wäre der Guard eine Erfindung neben der
    Norm statt ihre Umsetzung."""
    b = BINDUNG["p35c_keine_doppelfoerderung"]
    assert b["quelle"]["geltungsbedingung"] == "keine_10f_35a_oeffentliche_foerderung"
    assert b["typ"] == "bool" and b["askable"] is True
    assert "35a" in b["anker_ref"]["zitatanker"], (
        "Der Zitatanker nennt § 35a nicht — dann belegt er die Doppelförderungs-Regel nicht.")
