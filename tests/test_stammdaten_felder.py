"""Abnahme: Stammdaten-Felder (Team-Lead-Briefing 2026-08-09) — Name/Geburtsdatum/Adresse/
Konfession/Bankverbindung/Art_Erkl fuer Person A + B, Store -> deklariere() -> XML.

Ziel: checkESt-Pflichtmeldungen "Kein Hauptvordruck", "Art_Erkl fehlt", "Religion fehlt",
"Name/Vorname fehlt", "Adresse fehlt", "Bankverbindung fehlt" (siehe reports/adjudikation/
stammdaten_felder_bau_2026-08-09.md) schliessen.

Kernrisiko WERTEKODIERUNG (Klasse i, est_mapping.py): Laien-Enum -> amtlicher Religionsschluessel
ist reine Werte-Uebersetzung, KEIN 1:1-Passthrough — ein Tippfehler im Code-Mapping wuerde still
den falschen amtlichen Code deklarieren (XSD-strukturell weiterhin gueltig, inhaltlich falsch).
test_wertekodierung_code_typo_waechter demonstriert, dass ein mutierter Code den Test rot macht.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX       # noqa: E402
import est_mapping as EM      # noqa: E402
import store as ST            # noqa: E402
import traverser as TR        # noqa: E402

HID = "74931"
TS = "2026-08-10T09:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _b(s, feld_id, wert, zustand="bestaetigt"):
    sig = ({"signal_1": None, "signal_2": f"ok@{feld_id}"} if zustand == "bestaetigt"
           else {"signal_1": None, "signal_2": None})
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand=zustand,
                    herkunft=H, schreiber="ui:laie", signal=sig, ts=TS)


def _flags_einzel(s):
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)


def _flags_zusammen(s):
    _b(s, "veranlagung", "zusammen")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", True)


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


# ---------------------------------------------------------------- Person A: 1:1-Felder

def test_person_a_stammdaten_1_zu_1(bindung):
    """Name/Geburtsdatum/Adresse/Bankverbindung/Art_Erkl landen unveraendert unter ihrem Kz
    (Klasse 1, kein Wertekodierung/Instanz-Umweg)."""
    s = ST.leerer_store(2025, fall_id="stammdaten_a")
    _b(s, "stammdaten_nachname", "Maier")
    _b(s, "stammdaten_vorname", "Hans")
    _b(s, "stammdaten_geburtsdatum", "05.05.1955")
    _b(s, "stammdaten_strasse", "Musterstr.")
    _b(s, "stammdaten_hausnummer", "55")
    _b(s, "stammdaten_plz", "55555")
    _b(s, "stammdaten_wohnort", "Musterort")
    _b(s, "stammdaten_keine_bankverbindung", True)
    _b(s, "stammdaten_art_est_erklaerung", True)
    _flags_einzel(s)

    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    dekl = result["deklaration"]

    assert dekl["E0100201"] == "Maier"
    assert dekl["E0100301"] == "Hans"
    assert dekl["E0100401"] == "05.05.1955"
    assert dekl["E0101104"] == "Musterstr."
    assert dekl["E0101206"] == "55"
    assert dekl["E0100601"] == "55555"
    assert dekl["E0100602"] == "Musterort"
    assert dekl["E0102002"] is True
    assert dekl["E0100001"] is True
    assert result["eingaben_konsistent"] is True


def test_stammdaten_vorlaeufig_macht_unvollstaendig(bindung):
    """Fail-closed (Auflage 3): ein Pflicht-Stammdatum im Zustand vorlaeufig -> vollstaendig=False,
    kein stiller Versand mit unbestaetigtem Namen."""
    s = ST.leerer_store(2025, fall_id="stammdaten_vorlaeufig")
    _b(s, "stammdaten_nachname", "Maier", zustand="vorlaeufig")
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["eingaben_konsistent"] is False
    gruende = [u["feld_id"] for u in result["unvollstaendig"]]
    assert "stammdaten_nachname" in gruende


# ---------------------------------------------------------------- Person B: eigene Kz, kein Bucket

def test_person_b_stammdaten_direkt_in_deklaration(bindung):
    """Person-B-Stammdaten tragen EIGENE Kz (E0100901/E0100801/E0101001) -> 1:1 in die
    Haupt-Deklaration, NICHT in den person_b-Bucket (analog person_b_idnr; kein Person-
    Multiplikations-Reuse wie bei den Einkommens-Kz in PARTNER_INSTANZ)."""
    s = ST.leerer_store(2025, fall_id="stammdaten_b")
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "bruttoarbeitslohn_partner", 4000000)
    _b(s, "stammdaten_nachname_partner", "Maier")
    _b(s, "stammdaten_vorname_partner", "Carolina")
    _b(s, "stammdaten_geburtsdatum_partner", "09.07.1988")
    _flags_zusammen(s)

    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)

    assert result["deklaration"]["E0100901"] == "Maier"
    assert result["deklaration"]["E0100801"] == "Carolina"
    assert result["deklaration"]["E0101001"] == "09.07.1988"
    assert "E0100901" not in result["person_b"], (
        "Person-B-Stammdaten gehoeren NICHT in den person_b-Bucket (eigenes Kz, kein Reuse)")
    assert "E0100801" not in result["person_b"]
    assert "E0101001" not in result["person_b"]


# ---------------------------------------------------------------- Wertekodierung (Klasse i)

@pytest.mark.parametrize("laie_wert, xsd_code", [
    ("keine", "11"),
    ("evangelisch", "02"),
    ("roemisch-katholisch", "03"),
])
def test_kist_konfession_wertekodierung(bindung, laie_wert, xsd_code):
    s = ST.leerer_store(2025, fall_id=f"konfession_{laie_wert}")
    _b(s, "kist_konfession", laie_wert)
    _b(s, "kist_bundesland", "bayern")
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["deklaration"]["E0100402"] == xsd_code


def test_kist_konfession_andere_fail_closed(bindung):
    """'andere' hat keinen verifizierten Religionsschluessel (~35 moegliche XSD-Werte) ->
    nicht_deklariert, NICHT geraten (kein Rate-Kz)."""
    s = ST.leerer_store(2025, fall_id="konfession_andere")
    _b(s, "kist_konfession", "andere")
    _b(s, "kist_bundesland", "bayern")
    _flags_einzel(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert "E0100402" not in result["deklaration"]
    gruende = [n["grund"] for n in result["nicht_deklariert"] if n["feld_id"] == "kist_konfession"]
    assert gruende and "andere" in gruende[0]


def test_kist_konfession_partner_wertekodierung(bindung):
    s = ST.leerer_store(2025, fall_id="konfession_partner")
    _b(s, "kist_konfession_partner", "roemisch-katholisch")
    _flags_zusammen(s)
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["deklaration"]["E0101002"] == "03"


def test_wertekodierung_code_typo_waechter():
    """Mutations-Beweis: WERTEKODIERUNG['kist_konfession']['code']['roemisch-katholisch'] MUSS
    '03' sein (Enum_Religionsschluessel_ab_VZ_2014_3, XSD-verifiziert gegen die amtliche
    Referenz-XML). Ein Tippfehler hier (z.B. '04') waere sonst rein strukturell weiter XSD-valide
    und wuerde erst bei checkESt/amtlicher Pruefung aggregat aufschlagen — deshalb hart pinnen."""
    assert EM.WERTEKODIERUNG["kist_konfession"]["code"]["roemisch-katholisch"] == "03"
    assert EM.WERTEKODIERUNG["kist_konfession"]["code"]["evangelisch"] == "02"
    assert EM.WERTEKODIERUNG["kist_konfession"]["code"]["keine"] == "11"
    assert EM.WERTEKODIERUNG["kist_konfession"]["kz"] == "E0100402"
    assert EM.WERTEKODIERUNG["kist_konfession_partner"]["kz"] == "E0101002"


# ---------------------------------------------------------------- XML-Durchgang (Erreichbarkeit)

def test_stammdaten_im_xml(bindung):
    """Durchstich Store -> deklariere -> erzeuge_xml: alle Stammdaten-Werte tauchen im XML auf."""
    s = ST.leerer_store(2025, fall_id="stammdaten_xml")
    _b(s, "bruttoarbeitslohn", 6000000)
    _b(s, "stammdaten_nachname", "Maier")
    _b(s, "stammdaten_vorname", "Hans")
    _b(s, "stammdaten_geburtsdatum", "05.05.1955")
    _b(s, "stammdaten_strasse", "Musterstr.")
    _b(s, "stammdaten_hausnummer", "55")
    _b(s, "stammdaten_plz", "55555")
    _b(s, "stammdaten_wohnort", "Musterort")
    _b(s, "stammdaten_keine_bankverbindung", True)
    _b(s, "stammdaten_art_est_erklaerung", True)
    _b(s, "kist_konfession", "roemisch-katholisch")
    _b(s, "kist_bundesland", "bayern")
    _flags_einzel(s)

    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID)
    clean = xml.replace("ns0:", "").replace("ns1:", "")

    assert "<E0100201>Maier</E0100201>" in clean
    assert "<E0100301>Hans</E0100301>" in clean
    assert "<E0100401>05.05.1955</E0100401>" in clean
    assert "<E0101104>Musterstr.</E0101104>" in clean
    assert "<E0101206>55</E0101206>" in clean
    assert "<E0100601>55555</E0100601>" in clean
    assert "<E0100602>Musterort</E0100602>" in clean
    assert "<E0100402>03</E0100402>" in clean
    assert "<E0102002>X</E0102002>" in clean
    assert "<E0100001>X</E0100001>" in clean


def test_konfession_andere_nennt_dem_nutzer_den_ausweg(bindung):
    """"andere" ist bewusst fail-closed — aber der Nutzer muss erfahren, was er tun kann.

    Der amtliche Enum fuehrt rund zwanzig Religionsschluessel, fast durchweg regionale
    Koerperschaften (juedische Gemeinden getrennt nach Bundesland: 19 Hessen, 26 Bayern,
    18 Frankfurt, 25 Baden). Eine Laien-Option "juedisch" waere nicht eindeutig abbildbar,
    ein geratener Schluessel ein Kirchensteuerfehler, der in keiner Summe auffaellt.
    Deshalb kein Code — aber auch keine Sackgasse: "Wert 'andere' ohne XSD-Code-Zuordnung"
    allein sagt dem Nutzer weder warum noch was jetzt zu tun ist.
    """
    s = ST.leerer_store(2025, fall_id="konf_andere")
    _b(s, "kist_konfession", "andere")
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)

    assert "E0100402" not in result["deklaration"], "geratener Religionsschluessel — nie tun"
    eintrag = next(x for x in result["nicht_deklariert"] if x["feld_id"] == "kist_konfession")
    hinweis = eintrag.get("hinweis", "")
    assert hinweis, "kein Hinweis — der Nutzer steht vor einer Sackgasse"
    assert "ELSTER" in hinweis, "Hinweis nennt keinen Weg, wie der Nutzer weiterkommt"
    assert "unberuehrt" in hinweis, (
        "Hinweis sagt nicht, dass der Rest der Erklaerung gueltig bleibt — sonst glaubt "
        "der Nutzer, seine ganze Erklaerung sei blockiert")


def test_konfession_bekannter_wert_hat_keinen_hinweis(bindung):
    """Gegenprobe: ein zuordenbarer Wert erzeugt gar keinen nicht_deklariert-Eintrag."""
    s = ST.leerer_store(2025, fall_id="konf_ev")
    _b(s, "kist_konfession", "evangelisch")
    snap, _ = ST.materialisiere(s)
    result = EM.deklariere(snap, bindung)
    assert result["deklaration"]["E0100402"] == "02"
    assert not [x for x in result["nicht_deklariert"] if x["feld_id"] == "kist_konfession"]
