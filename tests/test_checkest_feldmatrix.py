"""Feld-Matrix gegen die amtliche Pruefung: jedes Kz-tragende Feld einmal EINZELN scharf.

WARUM DIESER TEST EXISTIERT — die Luecke zwischen zwei guten Gates.

Es gab schon zwei Gates, die sich nicht ueberlappen:

  test_ring_deklaration_differential   jedes Betragsfeld hat einen Weg ins XML
                                       (Kz / nicht_deklariert / Aggregat / Ausschluss)
                                       -> kennt ERiC NICHT
  test_checkest_durchstich (Ratsche)   das XML ist amtlich plausibel
                                       -> kennt ERiC, faehrt aber EINEN Fall

Am 2026-08-10 fiel dazwischen ein Abgabe-Blocker: der Differential-Test setzt selbst
`kap_kapitalertraege = 500000` und war gruen, weil E1900701 im XML ankommt. Gegen checkESt
gefahren ergibt derselbe Wert rc=610001002:

    "Auf den Anlagen KAP und / oder KAP-BET wurden Kapitalertraege erklaert, die dem
     inlaendischen Steuerabzug unterlegen haben. Bitte geben Sie auf der Anlage KAP auch
     einen Grund fuer die Angabe der Kapitalertraege an."

Die Ratsche sah es nicht, weil ihre Fixtur `kap_kapitalertraege = 0` fuehrt (Option A:
Nullwerte nicht deklarieren — korrekt, deckt aber genau deshalb den Fall nicht ab).
Zusammenstecken lassen sich die beiden nicht: die Differential-Szenarien haben keine
Stammdaten und sind strukturell nicht abgabefaehig, koennen ERiC also nie erreichen.
Deshalb setzt diese Matrix auf der Ratschen-Fixtur auf, die Stammdaten fuehrt.

WAS DIESE MATRIX FINDET: Felder, die wir erklaeren, ohne das mitzuliefern, was das Finanzamt
dazu verlangt — Antragsgruende, Pflicht-Begleitangaben, Feldkopplungen.

WAS SIE NICHT FINDET: Rechenfehler. ERiC prueft Plausibilitaet, nicht Richtigkeit. Ob 8400
Cent die korrekte Kuerzung sind, sagen Golden Cases und der GETTSIM-Crosscheck.

KOSTEN: ~0,75 s je Lauf, die ganze Datei unter 10 s.
"""

from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("elster", "produkt/import", "produkt/mapping", "produkt/store",
            "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, HERE)

import checkest_gate as CE          # noqa: E402
import elster_xml as EX             # noqa: E402
import est_mapping                  # noqa: E402
import store as ST                  # noqa: E402
import traverser as TR              # noqa: E402

from test_checkest_durchstich import (  # noqa: E402
    _ABSENDER, _HID, _b, _fall_einzel, braucht_eric,
)


def _scharf(store) -> tuple[int, list[str]]:
    """Store -> Deklaration -> Abgabe-XML -> amtliches Plugin. Kein Versand."""
    snap, _ = ST.materialisiere(store)
    xml = EX.erzeuge_xml(est_mapping.deklariere(snap, TR.lade_bindung()),
                         vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split())
             for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    return rc, texte


def _mit(feld_id, wert):
    """Ratschen-Basisfall plus EIN zusaetzliches Feld.

    Der Store ist fail-closed gegen Ueberschreiben eines aktiven Events (store.py:232),
    darum wird ein bereits in der Fixtur gesetztes Feld ersetzt statt doppelt angelegt.
    """
    s = _fall_einzel()
    aktiv = None
    for e in reversed(s.get("events") or []):
        if e.get("feld_id") == feld_id and not e.get("ersetzt_durch"):
            aktiv = e["event_id"]
            break
    if aktiv:
        ST.append_event(s, feld_id=feld_id, wert=wert, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="laie",
                        signal={"signal_1": {"typ": "laie_eingabe"},
                                "signal_2": "laie_bestaetigt"},
                        ersetzt=aktiv)
    else:
        _b(s, feld_id, wert)
    return s


# Felder, die ALLEIN eine sinnvolle Erklaerung ergeben. Kuratiert, nicht generiert — und
# das ist eine gemessene Entscheidung, keine Bequemlichkeit:
#
# Der erste Wurf zog alle 55 Kz-tragenden askable Felder automatisch aus der Bindung. 29
# davon wurden rot, aber nur eine Minderheit waren echte Funde. Die Mehrheit war Rauschen
# aus zwei Quellen:
#   (a) Feld ohne seinen Block  — kind_kv ohne Anlage Kind ("Tragen Sie bitte den Vornamen
#       des Kindes ein"), p35c ohne Gebaeude-Standort, vv_einnahmen ohne Laufende_Nummer_V,
#       gewst_hebesatz ohne gewst_messbetrag. Der Fall ist unvollstaendig, nicht der Bau.
#   (b) untauglicher Probewert  — GdB=5 ("ungueltiger Grad der Behinderung", zulaessig sind
#       20/30/.../100), Pflegegrad=5 ("ungueltiger Schluesselwert").
# `instanz_gruppe` trennt (a) nur teilweise: p35c_* und p33a_* brauchen ebenfalls Kontext,
# fuehren aber keine Instanzgruppe. Ein Gate mit 70 % Rauschen wird weggeklickt statt
# gelesen — deshalb hier nur, was fuer sich steht.
#
# Ein Feld gehoert in diese Liste, wenn es ohne Begleit-Instanz erklaerbar ist. Der Rest
# braucht eine Block-Matrix (minimal vollstaendige Anlage je Block) — eigener Bau, siehe
# BACKLOG checkest-blockmatrix.
MATRIX = [
    # Anlage KAP — Betraege laut Steuerbescheinigung
    ("kap_kapitalertraege", 500000),
    ("kap_gewinn_aktien", 200000),
    ("kap_verlust_aktien", 100000),
    ("kap_verlust_sonstige", 50000),
    # § 35a — die drei Toepfe, jeder fuer sich erklaerbar
    ("hh_handwerker_arbeitskosten", 300000),
    ("hh_dienstleistungen", 200000),
    ("hh_minijob_aufwendungen", 100000),
    # § 33 / Vorsorge / Anlage N — je ein Vertreter, der allein steht
    ("agb_aufwendungen", 500000),
    ("tage_24h", 10),
    ("vor_rv_ausserhalb_lstb", 100000),
    # basis_kv steht NICHT hier: es braucht `versicherungsart`, sonst bricht schon der
    # Writer ab ("Art (versicherungsart) unbestaetigt — Kz-Zweig offen"). Gehoert damit in
    # die Block-Matrix, nicht in die Einzelfeld-Matrix.
]

# Bekannte, GEMESSENE Beanstandungen mit Begruendung. Ein Eintrag hier ist eine Schuld,
# kein Freibrief: er benennt, was fehlt, damit der Test nicht dauerrot ist und trotzdem
# niemand vergisst, dass der Fall heute nicht einreichbar ist.
BEKANNTE_LUECKEN = {
    "kap_kapitalertraege": (
        "Anlage KAP: Antragsgrund fehlt (E1900401 Guenstigerpruefung / E1900501 Ueberpruefung "
        "des Steuereinbehalts). Gemessen 2026-08-10, rc=610001002. BACKLOG kap-antragsgrund-fehlt."),
    "kap_gewinn_aktien": ("dito Anlage KAP Antragsgrund"),
    "kap_verlust_aktien": ("dito Anlage KAP Antragsgrund"),
    "kap_verlust_sonstige": ("dito Anlage KAP Antragsgrund"),
    "hh_handwerker_arbeitskosten": (
        "§ 35a: checkESt verlangt eine EINZELAUFSTELLUNG, wir liefern nur die Summe. Von dieser "
        "Matrix beim ersten Lauf gefunden (2026-08-10, rc=610001002): '...es wurde aber keine "
        "Einzelaufstellung der Handwerkerleistungen vorgenommen.' BACKLOG p35a-einzelaufstellung."),
    "hh_dienstleistungen": (
        "§ 35a, gleiche Ursache wie hh_handwerker_arbeitskosten: Einzelaufstellung fehlt. "
        "Der Befund ist damit systematisch ueber alle drei § 35a-Toepfe, nicht punktuell."),
    "hh_minijob_aufwendungen": (
        "§ 35a Minijobs, gleiche Ursache: 'Gesamtbetrag der Aufwendungen angegeben, es wurde "
        "aber kein(e Einzelaufstellung)...' — dritter von drei Toepfen."),
}


@braucht_eric
@pytest.mark.parametrize("feld_id,wert", MATRIX)
def test_feld_einzeln_bleibt_amtlich_plausibel(feld_id, wert):
    """Ein Feld anschalten darf die Erklaerung nicht uneinreichbar machen.

    Faellt ein Feld hier neu durch, fehlt eine Begleitangabe, die das Finanzamt zu genau
    diesem Feld verlangt — dieselbe Klasse wie der KAP-Antragsgrund. Der Fehlertext von
    ERiC steht in der Assertion, er benennt die fehlende Angabe meist woertlich.
    """
    rc, texte = _scharf(_mit(feld_id, wert))

    klasse = CE.klassifiziere_rc(rc)
    assert klasse != "io_gate_nicht_geprueft", (
        f"{feld_id}: rc={rc} — XML bricht VOR der Pruefung ab. Ein leerer Fehlerpuffer "
        f"heisst hier NICHT fehlerfrei.")

    if feld_id in BEKANNTE_LUECKEN:
        assert rc != CE.RC_OK, (
            f"{feld_id} ist als bekannte Luecke eingetragen, laeuft aber sauber durch "
            f"(rc={rc}). Wenn die Luecke geschlossen wurde: Eintrag aus BEKANNTE_LUECKEN "
            f"entfernen — sonst deckt der Test die Regression nicht mehr ab.")
        return

    assert rc == CE.RC_OK, (
        f"{feld_id}={wert} macht die Erklaerung uneinreichbar (rc={rc}).\n"
        + "\n".join(f"   - {t}" for t in texte[:5])
        + "\nEntweder fehlt eine Begleitangabe (dann bauen) oder es ist eine bewusste "
          "Luecke (dann mit gemessener Begruendung in BEKANNTE_LUECKEN eintragen).")


@braucht_eric
def test_basisfall_ohne_zusatzfeld_ist_sauber():
    """Kontrolle: die Fixtur selbst ist rc=0.

    Ohne diesen Test waere eine Matrix, in der ALLES rot ist, nicht von einer kaputten
    Fixtur zu unterscheiden.
    """
    rc, texte = _scharf(_fall_einzel())
    assert rc == CE.RC_OK, (
        f"Basisfall der Matrix ist selbst nicht plausibel (rc={rc}) — dann sagt kein "
        f"einziger Matrix-Eintrag etwas aus.\n" + "\n".join(f"   - {t}" for t in texte[:5]))


@braucht_eric
def test_bekannte_luecken_sind_begruendet():
    """Jeder Freistellungs-Eintrag nennt Messung und Grund — kein stilles Ausklammern."""
    for feld, grund in BEKANNTE_LUECKEN.items():
        assert feld in dict(MATRIX), (
            f"{feld} steht in BEKANNTE_LUECKEN, wird aber gar nicht gefahren — "
            f"toter Eintrag, der eine Abdeckung vortaeuscht.")
        assert len(grund) > 20, f"{feld}: Begruendung zu duenn, um spaeter noch zu tragen"
