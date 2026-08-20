"""Gate: der Antrag Guenstigerpruefung muss bei Zusammenveranlagung in BEIDEN Person-Containern
stehen (Anlage KAP Zeile 4, Kz 201/401 -> E1900401).

ANLASS 2026-08-20 (Abgabe-Blocker, gemessen gegen checkESt). Der Bau vom 2026-08-10 setzte den
Antrag nur fuer Person A. `einzel` war damit rc=0 — `zusammen` mit Kapitalertraegen aber nicht:

    rc=610001002, zwei Beanstandungen, beide zu Person B:
      "Beim Ehemann / Person A wurde ein Antrag auf Guenstigerpruefung fuer saemtliche
       Kapitalertraege gestellt. Da es sich um eine Zusammenveranlagung handelt ist dieser
       Antrag auch bei der Ehefrau / bei Person B zu stellen."
      "Auf den Anlagen KAP und / oder KAP-BET wurden Kapitalertraege erklaert, die dem
       inlaendischen Steuerabzug unterlegen haben. Bitte geben Sie auf der Anlage KAP auch einen
       Grund fuer die Angabe der Kapitalertraege an (Antrag auf Guenstigerpruefung, Antrag auf
       Ueberpruefung des Steuereinbehalts, Erklaerung zur Kirchensteuerpflicht)
       (Ehefrau / Person B)."

Und zwar unabhaengig davon, WER die Ertraege hat (A allein, B allein oder beide) — deshalb die
Parametrisierung unten.

WARUM ES NIEMAND SAH: die vorhandenen Gates fahren am Fall vorbei.
  test_checkest_durchstich    fuehrt kap_* = 0 -> Option A unterdrueckt die Kz, kein Antrag noetig
  test_checkest_feldmatrix    setzt Kapitalertraege scharf, aber nur im EINZEL-Fall
  test_kap_antrag_kz          prueft die BINDUNG (Kz auf der Scheibe), nicht das XML
Dieser Test schliesst genau die Kreuzung: Zusammenveranlagung UND echte Kapitalertraege.

WAS GEPRUEFT WIRD: der deklarierte Output bzw. das XML — nicht die Existenz eines Feldes in der
Bindung. Ein Gate auf der Bindung waere gruen geblieben, denn die Bindung war nie das Problem.

E1901401 (Sparer-Pauschbetrag) gehoert bewusst NICHT dazu: er ist nach § 20 Abs. 9 S. 2 ein
GEMEINSAMER Pauschbetrag, die Angabe bei einer Person genuegt. Gemessen 2026-08-20 — E1901401
allein bei Person B laesst beide Beanstandungen stehen, E1900401 allein bei B schliesst auf rc=0.
"""

from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("elster", "produkt/haut", "produkt/import", "produkt/mapping",
            "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, HERE)

import api as API                   # noqa: E402
import checkest_gate as CE          # noqa: E402
import elster_xml as EX             # noqa: E402
import est_mapping                  # noqa: E402
import store as ST                  # noqa: E402

from test_checkest_durchstich import (  # noqa: E402
    _ABSENDER, _BASIS_A, _BASIS_B, _HID, _b, braucht_eric,
)

ANTRAG_KZ = "E1900401"          # Anlage KAP Zeile 4, Antrag Guenstigerpruefung (§ 32d Abs. 6)
SPARER_PB_KZ = "E1901401"       # Anlage KAP Zeile 16, gemeinsamer Sparer-PB (§ 20 Abs. 9)

# Wer die Kapitalertraege hat, aendert nichts an der Pflicht: der Antrag gilt nach
# § 32d Abs. 6 S. 4 stets fuer saemtliche Kapitalertraege BEIDER Ehegatten.
WER_HAT_ERTRAEGE = [
    pytest.param(500000, 0, id="nur_A"),
    pytest.param(0, 500000, id="nur_B"),
    pytest.param(500000, 500000, id="beide"),
]


def _fall(veranlagung, kap_a=0, kap_b=0, fall_id="kap_antrag_b"):
    """Ratschen-Fixtur (fuehrt Stammdaten, also abgabefaehig) mit frei setzbaren KAP-Betraegen."""
    s = ST.leerer_store(2025, fall_id=fall_id)
    basis = _BASIS_A if veranlagung == "einzel" else (_BASIS_A + _BASIS_B)
    for f, w in basis:
        if f == "kap_kapitalertraege":
            w = kap_a
        elif f == "kap_kapitalertraege_partner":
            w = kap_b
        elif f == "kein_kap":
            w = not (kap_a or kap_b)
        _b(s, f, w)
    _b(s, "veranlagung", veranlagung)
    s["scheibe"] = "gesamt"
    return s


def _deklariere(store):
    """Der scharfe Pfad: Scheiben-Bindung + Ring-Injektion, wie api.einreichen() ihn faehrt.

    _mit_ring_werten gehoert dazu — kap_antrag_guenstigerpruefung ist askable=false und entsteht
    erst hier. tests/test_checkest_durchstich.py::_pruefe laesst den Schritt aus und kann diesen
    Fehler deshalb strukturell nicht sehen."""
    bindung = API._scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    felder = API._mit_ring_werten(felder, 2025)
    return est_mapping.deklariere(felder, bindung, snapshot_id=sid)


# ---- 1) die Naht: person_b traegt den Antrag ---------------------------------------------

@pytest.mark.parametrize("kap_a,kap_b", WER_HAT_ERTRAEGE)
def test_antrag_steht_in_beiden_person_containern(kap_a, kap_b):
    r = _deklariere(_fall("zusammen", kap_a, kap_b))
    assert r["deklaration"].get(ANTRAG_KZ) is not None, (
        f"{ANTRAG_KZ} fehlt schon bei Person A (kap_a={kap_a}, kap_b={kap_b}) — dann misst der "
        f"Test unten nicht die Person-B-Spiegelung, sondern einen kaputten Antrag ueberhaupt.")
    assert r["person_b"].get(ANTRAG_KZ) is not None, (
        f"{ANTRAG_KZ} fehlt in der Person-B-Instanz (kap_a={kap_a}, kap_b={kap_b}). checkESt: "
        f"'Da es sich um eine Zusammenveranlagung handelt ist dieser Antrag auch bei der "
        f"Ehefrau / bei Person B zu stellen' -> rc=610001002, die Erklaerung ist uneinreichbar. "
        f"§ 32d Abs. 6 S. 4: der Antrag gilt fuer saemtliche Kapitalertraege BEIDER Ehegatten. "
        f"person_b={sorted(r['person_b'])}")


@pytest.mark.parametrize("kap_a,kap_b", WER_HAT_ERTRAEGE)
def test_sparer_pauschbetrag_wird_nicht_doppelt_gezaehlt(kap_a, kap_b):
    """Gegenrichtung: der Antrags-Fix darf E1901401 nicht mitspiegeln.

    § 20 Abs. 9 S. 2 gewaehrt Ehegatten EINEN gemeinsamen Sparer-Pauschbetrag von 2.000 EUR.
    Derselbe Betrag zweimal deklariert waere die Behauptung, er sei zweimal gewaehrt worden —
    die naheliegende Copy-Paste-Falle beim Spiegeln der KAP-Antragsfelder.

    Verboten ist deshalb der GLEICHE Betrag bei beiden, nicht ein Person-B-Eintrag ueberhaupt:
    § 20 Abs. 9 S. 3 verlangt den gemeinsamen Pauschbetrag "bei jedem Ehegatten je zur Haelfte".
    Heute deklariert das Produkt den vollen Betrag bei Person A (gemessen 2026-08-20:
    E1901401=2000 bei A, nichts bei B; checkESt akzeptiert das, rc=0). Wird die Halbteilung
    spaeter gebaut, muss dieser Test NICHT angefasst werden — 1000/1000 passiert ihn."""
    r = _deklariere(_fall("zusammen", kap_a, kap_b))
    pb_a, pb_b = r["deklaration"].get(SPARER_PB_KZ), r["person_b"].get(SPARER_PB_KZ)
    assert not (pb_b is not None and pb_b == pb_a), (
        f"{SPARER_PB_KZ} steht bei beiden Personen mit demselben Betrag ({pb_a!r}) — der "
        f"gemeinsame Sparer-Pauschbetrag (§ 20 Abs. 9 S. 2) waere damit doppelt erklaert. "
        f"Beim Spiegeln des Antrags (E1900401) darf dieses Kz nicht mitlaufen.")


# ---- 2) Gegenproben: die Spiegelung darf nicht bedingungslos feuern ------------------------

def test_einzelveranlagung_erzeugt_keine_person_b_instanz():
    """Ohne die Gegenprobe waere ein Bau gruen, der bedingungslos spiegelt — und jede
    Einzelveranlagung bekaeme einen Ehegatten-Container, den es nicht gibt."""
    r = _deklariere(_fall("einzel", 500000, 0, fall_id="kap_antrag_b_einzel"))
    assert r["deklaration"].get(ANTRAG_KZ) is not None, (
        "Vorbedingung verletzt: schon Person A hat keinen Antrag, der Fall misst nichts.")
    assert ANTRAG_KZ not in r["person_b"], (
        f"{ANTRAG_KZ} in person_b bei EINZELveranlagung — es gibt keinen Ehegatten, dem der "
        f"Antrag zuzurechnen waere.")


def test_ohne_kapitalertraege_kein_antrag_bei_person_b():
    """Das Schweigen testen, nicht nur das Feuern: ohne Kapitalertraege greift Option A (die
    KAP-Kz werden unterdrueckt), dann darf auch bei Person B kein Antrag stehen — sonst
    beantragen wir die Guenstigerpruefung fuer Ehepaare ohne einen einzigen Kapitalertrag."""
    r = _deklariere(_fall("zusammen", 0, 0, fall_id="kap_antrag_b_null"))
    assert ANTRAG_KZ not in r["deklaration"] and ANTRAG_KZ not in r["person_b"], (
        f"{ANTRAG_KZ} deklariert, obwohl beide Personen 0 Kapitalertraege erklaert haben "
        f"(deklaration={ANTRAG_KZ in r['deklaration']}, person_b={ANTRAG_KZ in r['person_b']}).")


# ---- 3) die zweite Naht: person_b -> XML ---------------------------------------------------

@pytest.mark.parametrize("kap_a,kap_b", WER_HAT_ERTRAEGE)
def test_xml_traegt_den_antrag_in_beiden_kap_bloecken(kap_a, kap_b):
    """person_b im deklariere()-Output genuegt nicht — der Writer muss den Wert auch in die
    zweite KAP-Instanz einhaengen UND den bool als Ja-Enum serialisieren (Ja1BaseCType_RABE,
    Wert "1"). Braucht kein ERiC, deckt aber genau die Naht, an der ein person_b-Eintrag ohne
    Schema-Pfad still verschwinden wuerde."""
    r = _deklariere(_fall("zusammen", kap_a, kap_b))
    xml = EX.erzeuge_xml(r, vz=2025, hersteller_id=_HID or "00000",
                         abgabefaehig=False, **_ABSENDER)
    kap_bloecke = re.findall(r"<KAP\b.*?</KAP>", xml, re.S)
    assert len(kap_bloecke) == 2, (
        f"{len(kap_bloecke)} KAP-Bloecke statt 2 (Person A + Person B) — die Instanz-Achse "
        f"traegt nicht, der Antrag kann gar nicht bei beiden ankommen.")
    mit_antrag = [i for i, blk in enumerate(kap_bloecke) if f"<{ANTRAG_KZ}>" in blk]
    assert mit_antrag == [0, 1], (
        f"{ANTRAG_KZ} steht nur in KAP-Block {mit_antrag} statt in beiden. Ohne den Antrag im "
        f"Person-B-Block ist die Erklaerung uneinreichbar (rc=610001002).")


# ---- 4) scharf gegen die amtliche Pruefung --------------------------------------------------

@braucht_eric
@pytest.mark.parametrize("kap_a,kap_b", WER_HAT_ERTRAEGE)
def test_zusammen_mit_kapitalertraegen_ist_amtlich_plausibel(kap_a, kap_b):
    """Der eigentliche Beweis: rc=0 statt 610001002.

    Der Test oben prueft die Naht, dieser die Wirkung — und zwar mit derselben Bibliothek, die
    die Abgabe spaeter ablehnt. Faellt er aus (kein ERiC/keine Hersteller-ID), bleiben die
    credential-freien Tests oben als Gate."""
    r = _deklariere(_fall("zusammen", kap_a, kap_b))
    xml = EX.erzeuge_xml(r, vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split())
             for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    assert rc == CE.RC_OK, (
        f"zusammen + Kapitalertraege (A={kap_a}, B={kap_b}) ist uneinreichbar (rc={rc}):\n"
        + "\n".join(f"   - {t}" for t in texte))
    assert not texte, f"rc=0, aber Beanstandungen:\n" + "\n".join(f"   - {t}" for t in texte)
