"""stammdaten_steuernummer: eigenes Feld ohne Kz-Spiegel (Vorsatz/<StNr> ist kein E10-Kz-
Element). Auftrag (team-lead, 2026-08-10): Steuernummer erreicht erzeuge_xml() nicht ueber
`deklaration`, sondern ueber den neuen `snapshot`-Parameter (`_leite_steuernummer_ab()`).

Vier Anforderungen aus dem Auftrag, je eigener Testblock:
  1. Kein Kz — Ableitung laeuft ueber snapshot['stammdaten_steuernummer'], nicht ueber ein
     Kz in `deklaration` (s. `_leite_absender_ab()` fuer die anderen vier Absender-Felder).
  2. Format ist geprueft, hart — `_STNR_PATTERN` (amtliches Muster, E10-2025.xsd:1779-1786),
     PLUS die bereits bestehende Finanzamts-Praefix-Kopplung (tests/test_elster_xml.py).
  3. PII — die Steuernummer darf bei einem Formatfehler nicht in der Fehlermeldung stehen.
  4. Typ text mit Formatpruefung, nicht Zahl — bindung.yaml-Eintrag typ=text, kein Kz, daher
     von tests/test_bindungs_typ_vs_xsd_typ.py strukturell uebersprungen (`if not kz: continue`);
     dieselbe Pruefidee (beispielwert gegen amtliches Muster) wird hier nachgeholt.

Die Messung, auf die es ankommt (Aequivalenz Ableitung vs. expliziter Parameter, ueber den
ECHTEN Ring-Pfad und das amtliche checkESt-Plugin), steht in tests/test_checkest_durchstich.py
(gleiche Fixtur-Stelle, `_STAMM_A`) — dort liegen bereits _fall_einzel()/_HID/braucht_eric.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "import"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))
sys.path.insert(0, os.path.join(ROOT, "elster"))

import elster_xml as EX      # noqa: E402
import traverser as TR       # noqa: E402
import api_constants as AC   # noqa: E402

HID = "74931"                # ERiC-Test-Hersteller-ID aus dem amtlichen Beispiel-XML

_STNR = "9181081508155"      # amtliches Beispiel, Praefix = Finanzamtsnummer 9181
_STAMM_KZ = dict(E0100201="Maier", E0100301="Hans", E0101104="Musterstr.",
                 E0101206="55", E0100601="55555", E0100602="Musterort")


def _dekl(**kz) -> dict:
    return {"vollstaendig": True, "deklaration": dict(kz)}


def _snap(wert, zustand="bestaetigt") -> dict:
    return {"stammdaten_steuernummer": {"wert": wert, "zustand": zustand}}


# ------------------------------------------------------------ 1. Kein Kz — Snapshot-Ableitung

def test_ableitung_liefert_wert_bei_bestaetigtem_snapshot():
    assert EX._leite_steuernummer_ab(_snap(_STNR)) == _STNR


def test_ableitung_verlangt_bestaetigten_zustand():
    """vorlaeufig/offen zaehlt nicht — Zwei-Signal, wie bei jedem anderen Feld auch.

    Mutationsprobe: `!= "bestaetigt"` zu `== "bestaetigt"` gedreht wuerde das Gegenteil pruefen
    und *diesen* Test rot machen (er verlangt None bei "vorlaeufig")."""
    assert EX._leite_steuernummer_ab(_snap(_STNR, zustand="vorlaeufig")) is None
    assert EX._leite_steuernummer_ab(_snap(_STNR, zustand="offen")) is None


def test_ableitung_ohne_feld_im_snapshot_liefert_none():
    assert EX._leite_steuernummer_ab({}) is None


def test_ableitung_mit_leerem_wert_liefert_none():
    assert EX._leite_steuernummer_ab(_snap("")) is None


def test_explizite_steuernummer_hat_vorrang_vor_snapshot_ableitung():
    """Ein gesetzter Parameter gewinnt — Ableitung ist Fallback, keine Ueberschreibung
    (dasselbe Prinzip wie bei absender_name/_strasse/_plz/_ort, s. test_elster_xml.py).

    Mutationsprobe: `if not absender_steuernummer and snapshot is not None` zu
    `if snapshot is not None` (Vorrangpruefung entfernt) macht diesen Test rot, weil dann
    der abgeleitete Snapshot-Wert den expliziten Parameter ueberschreiben wuerde."""
    xml = EX.erzeuge_xml(_dekl(**_STAMM_KZ), vz=2025, hersteller_id=HID, abgabefaehig=True,
                         absender_name="Maier Hans", absender_strasse="Musterstr. 55",
                         absender_plz="55555", absender_ort="Musterort",
                         absender_steuernummer="9181000000009",
                         snapshot=_snap("9181099999999"))
    assert "<StNr>9181000000009</StNr>" in xml
    assert "9181099999999" not in xml


def test_snapshot_ohne_explizite_steuernummer_baut_das_xml():
    """Der geforderte Weg: abgabefaehig=True OHNE absender_steuernummer-Parameter, nur ueber
    snapshot — das ist die im Auftrag verlangte Bauweise fuer den Live-Endpunkt."""
    xml = EX.erzeuge_xml(_dekl(**_STAMM_KZ), vz=2025, hersteller_id=HID, abgabefaehig=True,
                         snapshot=_snap(_STNR))
    assert f"<StNr>{_STNR}</StNr>" in xml


def test_snapshot_ohne_bestaetigung_bleibt_fail_closed():
    """Ein nur vorlaeufiger Wert im Snapshot darf NICHT als Absender-StNr einsickern."""
    with pytest.raises(EX.XmlFehler, match="absender_steuernummer.*kein Kz-Spiegel"):
        EX.erzeuge_xml(_dekl(**_STAMM_KZ), vz=2025, hersteller_id=HID, abgabefaehig=True,
                       snapshot=_snap(_STNR, zustand="vorlaeufig"))


# ------------------------------------------------------------ 2. Format ist geprueft, hart

@pytest.mark.parametrize("wert", [
    "9181081508155",   # amtliches Beispiel
    "0000000000001",   # 5. Ziffer '0' — auch bei fuehrenden Nullen im Praefix gueltige Form
])
def test_stnr_pattern_akzeptiert_amtliche_form(wert):
    assert EX._STNR_PATTERN.match(wert)


@pytest.mark.parametrize("wert,grund", [
    ("918108150815", "12 Ziffern statt 13"),
    ("91810815081555", "14 Ziffern statt 13"),
    ("9181181508155", "5. Ziffer ist '1' statt '0'"),
    ("9181A81508155", "Buchstabe statt Ziffer"),
    ("", "leer"),
])
def test_stnr_pattern_lehnt_falsche_form_ab(wert, grund):
    assert not EX._STNR_PATTERN.match(wert), grund


def test_falsches_format_bricht_vor_dem_finanzamts_praefix_check_ab():
    """Steuernummer mit FALSCHEM Format UND falschem Praefix muss den Muster-Fehler zeigen,
    nicht den Praefix-Fehler — Formpruefung sitzt VOR der Kopplungspruefung (elster_xml.py).
    Beide Bedingungen muessen gleichzeitig verletzt sein, sonst entscheidet die Reihenfolge
    gar nichts (ein einzeln falscher Praefix bei sonst korrektem Format wuerde ohnehin den
    Praefix-Fehler zeigen, unabhaengig von der Reihenfolge — kein Mutationssignal).

    Mutationsprobe: die beiden `if`-Bloecke in erzeuge_xml() vertauscht zeigt hier
    'Finanzamtsnummer' statt 'amtlichen Muster' -> match schlaegt fehl (gemessen: mit
    vertauschter Reihenfolge grün, wenn nur der Praefix bricht — deshalb bewusst ein Wert
    gewählt, bei dem AUCH der Praefix nicht passt)."""
    with pytest.raises(EX.XmlFehler, match="amtlichen Muster") as exc:
        EX.erzeuge_xml(_dekl(**_STAMM_KZ), vz=2025, hersteller_id=HID,
                       empfaenger_finanzamt="9181", abgabefaehig=True,
                       absender_name="Maier Hans", absender_strasse="Musterstr. 55",
                       absender_plz="55555", absender_ort="Musterort",
                       absender_steuernummer="123456789012")  # 12 Ziffern UND falscher Praefix
    assert "Finanzamtsnummer" not in str(exc.value)


def test_formfehler_gilt_auch_fuer_abgeleitete_steuernummer():
    """Die Formpruefung gilt fuer Parameter UND Ableitung gleich — ein kaputter Snapshot-Wert
    darf nicht durchrutschen, nur weil er nicht explizit uebergeben wurde."""
    with pytest.raises(EX.XmlFehler, match="amtlichen Muster"):
        EX.erzeuge_xml(_dekl(**_STAMM_KZ), vz=2025, hersteller_id=HID, abgabefaehig=True,
                       snapshot=_snap("918108150815"))  # 12 Ziffern statt 13


# ------------------------------------------------------------ 3. PII — Wert nie in der Meldung

def test_formatfehler_meldung_enthaelt_die_steuernummer_nicht():
    """Regel: Steuernummer nie im Klartext ausgeben — auch nicht in einer Fehlermeldung."""
    kaputt = "9181A81508155"
    with pytest.raises(EX.XmlFehler) as exc:
        EX.erzeuge_xml(_dekl(**_STAMM_KZ), vz=2025, hersteller_id=HID, abgabefaehig=True,
                       absender_name="Maier Hans", absender_strasse="Musterstr. 55",
                       absender_plz="55555", absender_ort="Musterort",
                       absender_steuernummer=kaputt)
    assert kaputt not in str(exc.value)


# ------------------------------------------------------------ 4. Typ text mit Formatpruefung

def test_bindung_stammdaten_steuernummer_ist_typ_text_ohne_kz():
    """Fuehrende Nullen sind bedeutungstragend -> typ=text, nicht int/cent (Auftrag Punkt 4)."""
    b = TR.lade_bindung()["stammdaten_steuernummer"]
    assert b["typ"] == "text"
    assert b["elster_kz"] is None
    assert b["kz_status"] == "endgueltig"
    assert b.get("elster_kz_grund"), "kz_status=endgueltig ohne Begruendung"


def test_bindung_beispielwert_erfuellt_das_amtliche_muster():
    """tests/test_bindungs_typ_vs_xsd_typ.py ueberspringt Felder ohne Kz (`if not kz: continue`)
    strukturell — dieses Feld hat keins. Dieselbe Pruefidee (beispielwert gegen XSD-Muster)
    wird hier nachgeholt, gegen `_STNR_PATTERN` statt gegen die XSD-Metadaten direkt."""
    beispiel = TR.lade_bindung()["stammdaten_steuernummer"]["beispielwert"]
    assert EX._STNR_PATTERN.match(str(beispiel)), (
        f"beispielwert {beispiel!r} erfuellt nicht das amtliche StNr-Muster")


def test_stammdaten_steuernummer_ist_askable_feld():
    assert "stammdaten_steuernummer" in AC.STAMMDATEN_FELDER
