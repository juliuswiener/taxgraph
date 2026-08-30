"""§ 22 Anlage R: `rentner_renten_beginn_jahr` traegt ein Jahr (`typ: int`, bindung_rentner.yaml:32),
die Kz-Verzweigung (est_mapping.VERZWEIGUNG) schickt denselben Wert aber nach E1800501 (o.ae.,
je `rentner_renten_art`) -- und E1800501 hat laut XSD den Typ `DatumTTpMMpJJJJBekanntBaseCType_RABE`
(E10-2025.xsd:20918, Dokumentation "Beginn der Rente"), erwartet also TT.MM.JJJJ, kein Jahr allein.

`elster_xml._wert_text` (Zeile 179-183) kennt genau zwei Faelle: bool -> "X", sonst `str(wert)`.
Keine Datumskonvertierung, kein Sonderfall fuer `typ: datum` -- ein Grep nach einer Umwandlungs-
funktion im Writer war leer. Ein `int`-Jahr wie 2015 wird also woertlich als `"2015"` geschrieben.

Real gemessen (HEAD a4da29b, echtes ERiC, `stammdaten_geburtsdatum="12.03.1950"`,
`rentner_renten_art="gesetzliche_rente"`, `rentner_renten_beginn_jahr=2015`):

  E1800501 (Beginn der Rente) im XML: '2015'          -> ERiC:
    '$/R[1]/Leibr_gesetzl[1]/Einz[1]/E1800501[1]$':
    Bitte geben Sie ein gültiges Datum TT.MM.JJJJ ein.  (rc=610001002)
  E0100401 (Geburtsdatum)     im XML: '12.03.1950'     -> ERiC akzeptiert das Feld.

Reichweite: JEDER Rentner, der ueberhaupt eine Rentenzeile ausfuellt (rentner_renten_art
bestaetigt), traegt zwangslaeufig `rentner_renten_beginn_jahr` -- das Feld ist Teil derselben
Instanzgruppe (`instanz_gruppe: rente`) und Voraussetzung fuer die Besteuerungsanteil-Kohorte.
Ohne diese Zeile kommt keine Anlage-R-Deklaration durch checkESt.

Kein Reparaturweg wird hier vorweggenommen. Zwei sind erkennbar (Tag/Monat zusaetzlich erfragen,
oder einen festen Tag ergaenzen) und BEIDE waeren Tatsachenbehauptungen ueber etwas, das der
Nutzer nie explizit angegeben hat -- die Entscheidung dazwischen liegt bei Julius, nicht hier.
Dieser Test prueft nur das beobachtbare Format-Symptom: der geschriebene Text fuer E1800501 muss
ueberhaupt ein Datum TT.MM.JJJJ sein, unabhaengig davon, welcher Tag/Monat am Ende steht.

Die Kontrolle daneben (`test_geburtsdatum_schreibt_datumsformat`) beweist Unterscheidungskraft:
derselbe Writer, derselbe XSD-Typ, ein ANDERES Feld -- und dort geht es durch, weil
`stammdaten_geburtsdatum` (typ: datum, bindung_an_gesamt.yaml:1107) den Wert bereits als
TT.MM.JJJJ-String im Store haelt (`beispielwert: "05.05.1955"`) und `_wert_text` ihn nur
durchreicht. Der Writer selbst formatiert nichts -- die Luecke ist feldspezifisch (int statt
vorformatierter String), kein genereller Datumsdefekt.

NULL LLM.
"""
from __future__ import annotations

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import pytest

import elster_xml as EX      # noqa: E402
import est_mapping           # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402

from test_checkest_durchstich import _ABSENDER, _b  # noqa: E402

_DATUM_TTMMJJJJ = re.compile(r"^\d{2}\.\d{2}\.\d{4}$")

_STAMM_RENTNER = (
    ("stammdaten_nachname", "Schulz"), ("stammdaten_vorname", "Erika"),
    ("stammdaten_geburtsdatum", "12.03.1950"),
    ("stammdaten_strasse", "Rentnerweg"), ("stammdaten_hausnummer", "3"),
    ("stammdaten_plz", "12345"), ("stammdaten_wohnort", "Musterstadt"),
    ("stammdaten_keine_bankverbindung", True),
    ("stammdaten_art_est_erklaerung", True),
    ("kist_konfession", "keine"),
    ("stammdaten_steuernummer", "9181081508155"),
)

# Rentner MIT Rentenzeile (anders als _fall_rentner in test_pflichtfelder_eric_sweep.py, die die
# Zeile bewusst ausspart) -- braucht rentner_renten_art bestaetigt, um die Instanzgruppe "rente"
# und damit E1800501 ueberhaupt zu erreichen.
_BASIS_RENTNER_MIT_RENTE = (
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_verlust_aktien", 0),
    ("kap_verlust_sonstige", 0),
    ("vor_rv_ausserhalb_lstb", 0),
    ("versicherungsart", "gesetzlich_an"), ("basis_kv", 0), ("basis_pv", 0),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("rentner_renten_art", "gesetzliche_rente"),
    ("rentner_jahresrente", 1_800_000),
    ("rentner_renten_beginn_jahr", 2015),
) + _STAMM_RENTNER


def _xml_text():
    """Baut den Rentner-mit-Rente-Fall und liefert den erzeugten XML-Text -- kein ERiC-Aufruf,
    das Symptom (Formatfrage) ist am Text selbst pruefbar."""
    s = ST.leerer_store(2025, fall_id="rentenbeginn_datumsformat")
    for f, w in _BASIS_RENTNER_MIT_RENTE:
        _b(s, f, w)
    _b(s, "veranlagung", "einzel")
    bindung = TR.lade_bindung()
    felder, sid = ST.materialisiere(s)
    dekl = est_mapping.deklariere(felder, bindung, snapshot_id=sid)
    # Dummy-Hersteller-ID: nur Textfeld im XML, kein ERiC-Aufruf in diesem Test (das
    # Format-Symptom ist am Text selbst pruefbar, s. Modul-Docstring).
    xml = EX.erzeuge_xml(dekl, vz=2025, hersteller_id="74931", abgabefaehig=True, **_ABSENDER)
    return xml if isinstance(xml, str) else xml.decode("utf-8")


def _kz_text(xml_text: str, kz: str) -> str | None:
    m = re.search(rf"<[^>]*:?{kz}[^>]*>([^<]*)</", xml_text)
    return m.group(1) if m else None


# ---------------------------------------------------------------- Kontrolle (Unterscheidungskraft)

def test_geburtsdatum_schreibt_datumsformat():
    """Positivkontrolle: E0100401 (Geburtsdatum) hat denselben XSD-Typ wie E1800501
    (DatumTTpMMpJJJJBekanntBaseCType_RABE), aber der Writer schreibt hier korrekt -- weil
    stammdaten_geburtsdatum den Wert bereits als TT.MM.JJJJ-String im Store haelt. Ohne diese
    Kontrolle koennte ein kaputter Testaufbau (z.B. eine grundsaetzlich fehlerhafte XML-Erzeugung)
    denselben xfail unten erzeugen, ohne dass die Rentenbeginn-Luecke ueberhaupt beteiligt ist."""
    text = _kz_text(_xml_text(), "E0100401")
    assert text is not None, "E0100401 nicht im XML gefunden -- Testaufbau selbst kaputt."
    assert _DATUM_TTMMJJJJ.match(text), (
        f"E0100401 (Geburtsdatum) = {text!r} -- kein TT.MM.JJJJ, obwohl dieses Feld als "
        "Kontrolle gelten soll (Writer schreibt Daten korrekt, wenn der Store sie so haelt).")


# ---------------------------------------------------------------- der Defekt

@pytest.mark.xfail(
    strict=True,
    reason="elster_xml._wert_text (Zeile 179-183) kennt keine Datumskonvertierung -- "
           "rentner_renten_beginn_jahr ist typ:int (bindung_rentner.yaml:32), die "
           "Kz-Verzweigung schickt den rohen Jahreswert nach E1800501, XSD-Typ "
           "DatumTTpMMpJJJJBekanntBaseCType_RABE erwartet TT.MM.JJJJ. Marker faellt am Tag des "
           "Fixes (XPASS) und zwingt dazu, ihn zu entfernen.")
def test_rentenbeginn_schreibt_kein_datumsformat():
    """Erwartung nach Fix: der geschriebene Text fuer E1800501 ist irgendein gueltiges TT.MM.JJJJ
    -- WELCHER Tag/Monat das ist (nachgefragt oder ergaenzt), entscheidet Julius, nicht dieser
    Test. Aktuell (gemessen) steht dort der blanke Jahreswert '2015', kein Datum."""
    text = _kz_text(_xml_text(), "E1800501")
    assert text is not None, "E1800501 nicht im XML gefunden."
    assert _DATUM_TTMMJJJJ.match(text), (
        f"E1800501 (Beginn der Rente) = {text!r} -- kein TT.MM.JJJJ. Real gemessenes ERiC-Echo "
        "auf genau diesen Text (rc=610001002): 'Bitte geben Sie ein gültiges Datum TT.MM.JJJJ "
        "ein.'")
