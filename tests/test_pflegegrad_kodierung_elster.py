"""Pflegegrad -> E0161606: das Schema kennt nur die Schluessel "2", "3" und "4".

Bau-Beleg zu backlog/taxgraph/pflegegrad-kodierung-elster (Worker quellen, 2026-10-01), Entscheidung
decisions/pflegegrad-ausserhalb-des-schemas-abbilden-oder-weglassen. Ort der Regeln: die Deklaration
(est_mapping.deklariere / elster::deklaration::deklariere), nicht der Writer.

E10-2025.xsd, Enum_AgB_Pflege_PB_Einz_Ang_pflegebeduerft_Pers_E0161606_CType: "2" = "Pflegegrad
2", "3" = "Pflegegrad 3", "4" = "Pflegegrad 4 oder 5". Vordruck AgB 2025 Zeile 15 ebenso.
Paragraf 33b Abs. 6 S. 3 EStG gewaehrt den Pauschbetrag erst ab Pflegegrad 2; nach S. 4 genuegt
auch das Merkzeichen H.

Drei Regeln (Entscheidung, Punkt 1-3):
  1. Grad 5 -> "4": Schema und Gesetz fassen 4 und 5 zusammen.
  2. Grad nicht in {2,3,4} und kein Merkzeichen H -> der ganze Pflege-Block entfaellt.
  3. Grad nicht in {2,3,4} mit Merkzeichen H -> nur E0161606 entfaellt, E0161808 traegt.

Regel 3 prueft E0161808 auf `is True`, nicht auf Abwesenheit: ein bestaetigtes "Nein" steht als
False in der Deklaration (die Naht, an der E0106603=0 im XML sichtbar ist). Wer nur die
Abwesenheit prueft, haelt ein "Nein" fuer ein "Ja" und laesst bei Grad 1 den Rest-Block stehen —
genau der Fall, den ERiC mit Regel 101100086 abweist.

Gemessen (2026-10-01, 3c7bb01, Rust-P9 davor): P9 laesst die 0 in E0161606 weg und laesst den
Rest des Blocks stehen. "Grad 0 ohne H-Feld" ist damit halb gruen — E0161606 fehlt schon, die
fuenf anderen Kz stehen aber weiter im XML und machen die Erklaerung ungueltig.
"""

from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("elster", "produkt/haut", "produkt/eingang", "produkt/mapping",
            "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, HERE)

import api as API                   # noqa: E402
import audit                        # noqa: E402
import checkest_gate as CE          # noqa: E402
import elster_xml as EX             # noqa: E402
import est_mapping                  # noqa: E402
import store as ST                  # noqa: E402

from test_checkest_blockmatrix import BLOECKE, _block_scharf, _setz  # noqa: E402
from test_checkest_durchstich import (  # noqa: E402
    _ABSENDER, _HID, _fall_einzel, braucht_eric, hid_attrappe,
)

# Alle Kz des Pflege-Blocks (bindung_rentner.yaml, AgB/Pflege_PB/Einz)
PFLEGE_KZ = ("E0161606", "E0161808", "E0161607", "E0161506", "E0110601", "E0106507", "E0106603")


@pytest.fixture(autouse=True)
def _isoliert(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


def _mit_grad(grad, hilflos="fehlt"):
    """Der Pflege-Block aus der Blockmatrix, aber mit dem Grad dieser Probe. `hilflos="fehlt"`
    laesst das Merkzeichen-Feld ganz weg (so wie ein Fall, der es nie beantwortet hat)."""
    felder = [(f, w) for f, w in BLOECKE["pflege_pauschbetrag"] if f != "rentner_pflegegrad"]
    felder.append(("rentner_pflegegrad", grad))
    if hilflos != "fehlt":
        felder.append(("rentner_gepflegter_hilflos", hilflos))
    return felder


def _xml(felder):
    store = _fall_einzel()
    for feld, wert in felder:
        _setz(store, feld, wert)
    store = dict(store)
    store["scheibe"] = "gesamt"
    bindung = API._scheibe_bindung(store)
    snap, sid = ST.materialisiere(store)
    snap = API._mit_ring_werten(snap, 2025)
    return EX.erzeuge_xml(est_mapping.deklariere(snap, bindung, snapshot_id=sid),
                          vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)


def _werte(xml, kz):
    return re.findall(f"<{kz}>([^<]*)</{kz}>", xml)


@pytest.mark.parametrize("grad,schluessel", [(2, "2"), (3, "3"), (4, "4"), (5, "4")])
def test_pflegegrad_steht_als_xsd_schluessel_im_xml(grad, schluessel, hid_attrappe):
    """Grad 2, 3 und 4 laufen unveraendert durch; Grad 5 wird auf "4" abgebildet."""
    assert _werte(_xml(_mit_grad(grad)), "E0161606") == [schluessel]


@pytest.mark.parametrize("felder", [
    _mit_grad(1),
    _mit_grad(0),
    # Die Falle: der VOLLE Block mit einem bestaetigten "Nein" beim Merkzeichen. Ein bestaetigtes
    # Nein steht als False in der Deklaration; wer dort auf Abwesenheit prueft statt auf `is True`,
    # haelt es fuer ein "Ja" und laesst die fuenf Begleit-Kz stehen — ERiC-Regel 101100086.
    _mit_grad(1, hilflos=False),
    _mit_grad(0, hilflos=False),
    # die haeufigste Form in echten Faellen: 0 als "kein Pflegegrad", sonst nichts zur Pflege
    [("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False)],
], ids=["grad1", "grad0", "grad1_h_nein", "grad0_h_nein", "grad0_ohne_block"])
def test_ohne_grad_2_bis_5_und_ohne_h_faellt_der_pflegeblock_weg(felder, hid_attrappe):
    """Ohne Grad 2..5 und ohne Merkzeichen H gibt es keinen Pauschbetrag. Bleibt ein Kz des
    Blocks stehen, weist ERiC die ganze Erklaerung ab (Regel 101100086, gemessen)."""
    xml = _xml(felder)
    assert [kz for kz in PFLEGE_KZ if _werte(xml, kz)] == []


@pytest.mark.parametrize("grad", [1, 0])
def test_ohne_grad_2_bis_5_mit_h_meldet_nur_das_merkzeichen(grad, hid_attrappe):
    """Mit Merkzeichen H traegt E0161808 den Anspruch (§33b Abs. 6 S. 4 EStG): nur E0161606
    entfaellt, der Rest des Blocks bleibt (er beschreibt dieselbe gepflegte Person)."""
    xml = _xml(_mit_grad(grad, hilflos=True))
    assert _werte(xml, "E0161606") == []
    assert _werte(xml, "E0161808") == ["1"]


@braucht_eric
@pytest.mark.parametrize("grad,hilflos", [(5, False), (1, False), (1, True), (0, False)])
def test_pflegegrad_ausserhalb_des_enums_ist_einreichbar(grad, hilflos):
    rc, texte = _block_scharf(_mit_grad(grad, hilflos))
    assert rc == CE.RC_OK, f"Pflegegrad {grad}, H={hilflos}: rc={rc}\n" + "\n".join(texte[:5])
