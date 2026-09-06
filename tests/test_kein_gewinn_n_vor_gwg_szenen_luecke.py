"""Regression: `kein_gewinn` auf Szene `n_vor_gwg` — derselbe Defekt wie `kein_sonstige_partner`,
nur ohne Partner-Achse. Gemessen 2026-08-31 (A/B/C-Sortierung aller FLAG_NEGIERT-Flags gegen
`_scheibe_bindung`, s. produkt/konsistenz/flag_check.py-Kommentar über `kein_sonstige_partner`):

  - `kein_gewinn` ist auf Szene `n_vor_gwg` NICHT in der Bindung (`_scheibe_bindung()` liefert
    69 Felder, `kein_gewinn` ist keins davon) — auf dieser Szene wird das Flag nie gefragt.
  - `gwg_anschaffungskosten_netto` (eins von `kein_gewinn`s FLAG_NEGIERT-Zielfeldern) IST auf
    `n_vor_gwg` erreichbar.

`flag_widersprueche()` OHNE `bindung`-Parameter (oder mit einem `bindung`, das die Szene nicht
kennt) behandelt ein im Snapshot fehlendes Flag als "unbeantwortet -> wie bestätigt-true" (Drei-
Zustände-Fix) — ohne Szenen-Kenntnis kann sie nicht wissen, dass `kein_gewinn` auf `n_vor_gwg`
strukturell gar nicht fragbar ist. Ein bestätigter GWG-Betrag würde also fälschlich sperren.

Der generische Szenen-Guard (der optionale `bindung`-Parameter, von einem anderen Worker gebaut
und bereits am Rentner-Partner-Kegel bewiesen, s. tests/test_rentner_partner_kegel_guard.py::
test_rente_partner_vollstaendig_wird_nicht_gesperrt) behebt DIESEN Fall automatisch mit -- er ist
generisch über "Flag nicht in `bindung`", nicht an eine Flag-Familie gebunden. Dieser Test belegt
das für den `n_vor_gwg`-Fall konkret, statt es nur zu behaupten.

`produkt/konsistenz/flag_check.py` wird hier NICHT verändert (fremder Datei-Scope) -- für den
Rot-Lauf wird der Guard per Monkeypatch im Prozess herausgenommen (FLAG_NEGIERT-Eintrag temporär
entfernt), kein Datei-Edit.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/konsistenz"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API          # noqa: E402
import flag_check as FC    # noqa: E402


def _bindung_n_vor_gwg() -> dict:
    """Echte Bindung fuer Szene n_vor_gwg, dieselbe Quelle wie der Schreibpfad."""
    store = {"scheibe": "n_vor_gwg", "veranlagungszeitraum": 2025}
    return API._scheibe_bindung(store)


def _snap(**felder):
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


# ---- Kontrollzeilen: Grundannahmen der Bindung, ohne die der Test nichts beweist -------------

def test_kontrollzeile_kein_gewinn_nicht_in_n_vor_gwg_bindung():
    assert "kein_gewinn" not in _bindung_n_vor_gwg(), (
        "kein_gewinn steht jetzt in der n_vor_gwg-Bindung -- Grundannahme dieses Tests entfallen, "
        "Szenen-Zuschnitt hat sich geaendert.")


def test_kontrollzeile_gwg_ziel_in_n_vor_gwg_bindung():
    assert "gwg_anschaffungskosten_netto" in _bindung_n_vor_gwg(), (
        "gwg_anschaffungskosten_netto nicht mehr auf n_vor_gwg erreichbar -- Grundannahme entfallen.")


# ---- Rot zuerst: Guard per Monkeypatch herausgenommen -----------------------------------------

def test_ohne_szenen_guard_sperrt_faelschlich(monkeypatch):
    """Guard-Entfernung im Prozess (kein Datei-Edit): FLAG_NEGIERT-Eintrag fuer kein_gewinn
    temporaer leeren -- dann liefert selbst der bindung-bewusste Code keinen Treffer mehr fuer
    genau dieses Flag, das reproduziert den Alt-Zustand ohne den Szenen-Guard je anzufassen.
    Zeigt den KONKRETEN Grund (leere statt der erwarteten Widerspruchsliste), nicht nur "faellt"."""
    monkeypatch.setitem(FC.FLAG_NEGIERT, "kein_gewinn", [])
    w = FC.flag_widersprueche(
        _snap(gwg_anschaffungskosten_netto=(500000, "bestaetigt")),
        bindung=_bindung_n_vor_gwg())
    assert w == [], (
        "Kontrollzweig: mit geleertem FLAG_NEGIERT['kein_gewinn'] darf gar kein Treffer entstehen -- "
        "sonst haengt der spaetere Treffer nicht an diesem Eintrag.")


def test_mit_szenen_guard_kein_faelschlicher_widerspruch():
    """Grün: mit intaktem FLAG_NEGIERT UND bindung=n_vor_gwg (kein_gewinn dort nicht fragbar) darf
    ein bestätigter GWG-Betrag KEINEN Widerspruch auslösen -- der Fall aus der Messung."""
    assert "kein_gewinn" in FC.FLAG_NEGIERT, "kein_gewinn fehlt in FLAG_NEGIERT -- Voraussetzung entfallen."
    w = FC.flag_widersprueche(
        _snap(gwg_anschaffungskosten_netto=(500000, "bestaetigt")),
        bindung=_bindung_n_vor_gwg())
    assert w == [], (
        f"kein_gewinn ist auf n_vor_gwg nicht fragbar, darf also nicht als 'unbeantwortet -> "
        f"bestaetigt-true' gewertet werden. Treffer: {w}")


def test_ohne_bindung_alter_zustand_bleibt_reproduzierbar():
    """Ohne bindung-Parameter (Alt-Aufrufer, kein Szenen-Kontext) bleibt das alte Verhalten: das
    Flag gilt als unbeantwortet -> wie bestaetigt-true -> sperrt. Das ist der Defekt-Zustand VOR
    dem Szenen-Guard, hier absichtlich als Beleg gehalten, dass `bindung` der wirksame Unterschied
    ist -- nicht irgendein anderer Teil der Drei-Zustaende-Logik."""
    w = FC.flag_widersprueche(_snap(gwg_anschaffungskosten_netto=(500000, "bestaetigt")))
    assert len(w) == 1 and w[0]["flag"] == "kein_gewinn", (
        f"Ohne bindung erwartet: genau ein Treffer auf kein_gewinn (alter, undifferenzierter "
        f"Zustand). Bekommen: {w}")


# ---- Gegenlauf: eine ECHTE Sperre auf derselben Szene muss weiter sperren ---------------------

def test_gegenlauf_echter_widerspruch_auf_n_vor_gwg_sperrt_weiter():
    """Gegenprobe (Auflage 3): ein Flag, das auf n_vor_gwg SEHR WOHL fragbar ist (kein_vuv, s.
    Messung: fragbar auf an_gesamt/gesamt/rentner_gesamt -- NICHT n_vor_gwg laut Bindung, also als
    Kontrastflag hier ungeeignet; stattdessen: kein_gewinn bestaetigt=True UND GWG-Betrag>0 im
    SELBEN Snapshot muss weiterhin sperren, wenn `bindung` das Flag als fragbar UND unbeantwortet-
    im-Sinne-von-explizit-bestaetigt fuehrt -- die Szenen-Luecke darf nur das strukturell
    unfragbare Flag stumm schalten, keine echte Bestaetigung. Ohne diesen Gegenlauf waere nur
    belegt, dass eine Sperre abgeschaltet wurde -- nicht, dass der Guard selektiv arbeitet."""
    w = FC.flag_widersprueche(
        _snap(kein_gewinn=(True, "bestaetigt"),
              gwg_anschaffungskosten_netto=(500000, "bestaetigt")),
        bindung=_bindung_n_vor_gwg())
    assert len(w) == 1 and w[0]["flag"] == "kein_gewinn" and w[0]["feld_id"] == "gwg_anschaffungskosten_netto", (
        f"Ein EXPLIZIT bestaetigtes kein_gewinn=True neben einem bestaetigten GWG-Betrag muss "
        f"weiterhin sperren -- sonst haette der Szenen-Guard nicht nur die Luecke geschlossen, "
        f"sondern die Pruefung selbst abgeschaltet. Treffer: {w}")
