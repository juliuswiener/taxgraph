"""Durchstich-Gate: Ring -> deklariere -> erzeuge_xml -> AMTLICHES checkESt.

Warum es das gibt
-----------------
Bis 2026-08-09 wurde der Abgabepfad nirgends gegen die amtliche Plausibilitaetspruefung
gehalten. Es gab zwei Ersatzpruefungen, und beide sind blind fuer das, was ELSTER
tatsaechlich ablehnt:

  * `tests/test_elster_xml.py` und die Zone-Differentialtests validieren gegen das E10-XSD.
    Das XSD winkt Faelle durch, die ERiC abweist — belegt: die von ElementTree erzeugte
    Praefix-Deklaration am <Elster>-Root ist XSD-valide und wurde von ERiC mit
    610301200 abgewiesen (Fix cebb228).
  * `tests/test_einreichen.py` mockt `erzeuge_xml` weg
    (`monkeypatch.setattr(EX, "erzeuge_xml", lambda *a, **k: '<?xml?><Elster/>')`) und
    prueft nur die Verdrahtung des Endpunkts, nie den Inhalt des erzeugten XML.
    Der Docstring von `tests/test_checkest_gate.py` benennt das offen als "Mock-Naht".

Dieser Test schliesst die Luecke: er baut einen echten Fall ueber den echten Pfad und
laesst das amtliche Plugin urteilen.

Bauart: RATSCHE, kein Festwert
------------------------------
Solange die Erklaerung unvollstaendig ist, meldet checkESt eine Zahl von Plausibilitaets-
fehlern. Dieser Test nagelt eine OBERGRENZE fest. Er wird rot, wenn die Zahl STEIGT
(Regression) — und ebenso, wenn sie SINKT, ohne dass jemand die Konstante nachzieht.
Die zweite Richtung ist Absicht: sie zwingt dazu, jeden Fortschritt hier einzutragen,
statt ihn unbemerkt verpuffen zu lassen. Gleiche Bauart wie
`REGELN_OHNE_GROUND_TRUTH` in `tests/test_n_*`.

Ziel ist RESTFEHLER_* == 0. Dann ist der Fall abgabefaehig und der Test wird zum
harten rc==0-Gate.

Ueberspringt sauber, wenn ERiC oder die Hersteller-ID fehlen (credential-freies CI).
Die ID wird nie geloggt.
"""
from __future__ import annotations

import os
import re
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/import", "produkt/mapping", "produkt/store",
             "produkt/traverser", "elster"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import checkest_gate as CE   # noqa: E402
import elster_xml as EX      # noqa: E402
import est_mapping           # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402

# Gemessen gegen ERiC 44.2.4.0, ESt_2025, auf dem ABGABE-Pfad (abgabefaehig=True).
#
# Ausgangsstand nach cebb228 (Namespace-Fix), als die amtliche Pruefung erstmals
# erreichbar wurde:
#   einzel   18: Vorsatz-Block (9), Hauptvordruck ESt 1 A (2), Stammdaten Person A (4),
#                Anlage N Steuerklasse + Lohnsteuer (2), Anlage KAP (1).
#   zusammen 24: dieselben 18, plus 6 fuer Person B — Person B traegt eigene Stammdaten
#                und eine eigene Steuerklasse. Die Zusammenveranlagung ist NICHT nur
#                "einzel mit zweitem Lohn"; ein Stammdaten-Bau, der nur A bedient,
#                laesst 6 Beanstandungen stehen.
#
# Verlauf:
#   e365a37 Vorsatz-Block      einzel 18 -> 9    zusammen 24 -> 15
#   6063dda Stammdaten         einzel  9 -> 3    zusammen 15 -> 6
#   (2026-08-10) KAP-Nulldeklaration (Julius-Entscheidung, Option A, est_mapping.py
#                KAP_FELDER_A/_B + _kap_alle_null): einzel 3 -> 2    zusammen 6 -> 4.
#                Bestaetigte KAP-Nullen werden nicht mehr deklariert, atomar ueber
#                Person A/B (s. dortiger Kommentar zur Writer-Luecken-Falle).
#   (2026-08-10) Anlage N Steuerklasse + Lohnsteuer (Julius-Entscheidung, Option A =
#                echte Werte statt 0,00; bindung_an_gesamt.yaml/bindung_p36_abschlusszahlung.yaml,
#                _STAMM_A/_STAMM_B jetzt mit steuerklasse[_partner] + p36_lohnsteuer[_partner]):
#                einzel 2 -> 0    zusammen 4 -> 0. Gemessen rc=CE.RC_OK (amtlich abgabefaehig)
#                fuer BEIDE Faelle -- kein Restfehler mehr, harter Gate-Zweig unten aktiv.
#
# Der Stammdaten-Schritt zaehlt nur, wenn die Felder im Fall auch BEANTWORTET sind —
# ihr blosses Vorhandensein aendert nichts. Deshalb stehen sie jetzt in _STAMM_A/_STAMM_B.
RESTFEHLER_EINZEL = 0
RESTFEHLER_ZUSAMMEN = 0

TS = "2026-08-09T22:00:00+00:00"
_H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _hid() -> str | None:
    """Hersteller-ID aus der Umgebung, sonst aus der gitignoreten .env. Nie loggen."""
    hid = os.environ.get("ELSTER_HERSTELLER_ID")
    if hid:
        return hid
    pfad = os.path.join(ROOT, ".env")
    if not os.path.exists(pfad):
        return None
    for zeile in open(pfad, encoding="utf-8"):
        if zeile.startswith(("ELSTER_HERSTELLER_ID=", "HERSTELLER_ID=")):
            return zeile.split("=", 1)[1].strip().strip('"').strip("'") or None
    return None


_HID = _hid()
_ERIC_DA = bool(_HID) and os.path.isdir(
    os.environ.get("ERIC_DIR", os.path.expanduser("~/02_Software/eric")))

braucht_eric = pytest.mark.skipif(
    not _ERIC_DA,
    reason="ERiC oder Hersteller-ID fehlt — amtliche Pruefung nicht lauffaehig "
           "(credential-freies CI)")


def _b(s, feld_id, wert):
    ST.append_event(store=s, feld_id=feld_id, wert=wert, zustand="bestaetigt",
                    herkunft=_H, schreiber="ui:laie",
                    signal={"signal_1": None, "signal_2": f"ok@{feld_id}"}, ts=TS)


# Stammdaten Person A (6063dda). Sie MUESSEN im Fall stehen, nicht nur als Feld
# existieren: die Beanstandungen verschwinden erst, wenn der Nutzer sie beantwortet hat.
# Ein Testfall ohne sie misst den Bau der Felder nicht.
# kist_konfession = "keine" -> Religionsschluessel 11 ("nicht kirchensteuerpflichtig").
# Bewusst so: ein kirchensteuerpflichtiger Schluessel verlangt laut Messung ZUSAETZLICH
# Lohnsteuer und Kirchensteuer auf Anlage N (reports/adjudikation/checkest_feldkopplungen_
# 2026-08-09.md), und der Lohnsteuer-Ausschluss ist eine offene Julius-Entscheidung.
# stammdaten_steuernummer (2026-08-10, keine Kz -> erreicht erzeuge_xml() ueber den
# snapshot-Parameter, s. produkt/import/elster_xml.py:_leite_steuernummer_ab()): Wert aus
# demselben amtlichen Beispiel wie _ABSENDER["absender_steuernummer"] unten, Praefix 9181
# passt zum Default-empfaenger_finanzamt. Steht hier in _STAMM_A, nicht als eigene Fixtur --
# _ABSENDER ueberschreibt sie ohnehin explizit (Vorrang der Parameter), aendert also nichts
# an RESTFEHLER_EINZEL/ZUSAMMEN. Erst test_steuernummer_ableitung_liefert_dieselbe_amtliche_
# fehlerzahl() unten nutzt sie ohne expliziten Parameter.
_STAMM_A = (("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
            ("stammdaten_geburtsdatum", "05.05.1955"),
            ("stammdaten_strasse", "Musterstr."), ("stammdaten_hausnummer", "55"),
            ("stammdaten_plz", "55555"), ("stammdaten_wohnort", "Musterort"),
            ("stammdaten_keine_bankverbindung", True),
            ("stammdaten_art_est_erklaerung", True),
            ("kist_konfession", "keine"),
            ("stammdaten_steuernummer", "9181081508155"),
            # Anlage N (Julius-Entscheidung 2026-08-10, Option A): E0200002 + E0200301 real deklariert.
            ("steuerklasse", "1"), ("p36_lohnsteuer", 1200000))

_STAMM_B = (("stammdaten_nachname_partner", "Maier"),
            ("stammdaten_vorname_partner", "Carolina"),
            ("stammdaten_geburtsdatum_partner", "09.07.1988"),
            ("kist_konfession_partner", "keine"),
            ("steuerklasse_partner", "5"), ("p36_lohnsteuer_partner", 1000000))

_BASIS_A = (("bruttoarbeitslohn", 6000000), ("vor_an_anteil_rv", 4200000),
            ("vor_ag_anteil_rv", 1200000), ("vor_rv_ausserhalb_lstb", 0),
            ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0),
            ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
            ("kein_gewinn", True), ("kein_kap", True),
            ("kein_vuv", True), ("kein_sonstige", True)) + _STAMM_A

_BASIS_B = (("bruttoarbeitslohn_partner", 5000000),
            ("vor_an_anteil_rv_partner", 3500000),
            ("vor_ag_anteil_rv_partner", 1000000),
            ("vor_rv_ausserhalb_lstb_partner", 0),
            ("kap_kapitalertraege_partner", 0), ("kap_gewinn_aktien_partner", 0),
            ("kap_verlust_aktien_partner", 0),
            ("kap_verlust_sonstige_partner", 0)) + _STAMM_B


def _fall_einzel():
    s = ST.leerer_store(2025, fall_id="durchstich_einzel")
    for f, w in _BASIS_A:
        _b(s, f, w)
    _b(s, "veranlagung", "einzel")
    return s


def _fall_zusammen():
    # Eigener Store: der Store ist fail-closed gegen Ueberschreiben eines aktiven
    # Events (store.py:232), `veranlagung` darf also nur EINMAL gesetzt werden.
    s = ST.leerer_store(2025, fall_id="durchstich_zusammen")
    for f, w in _BASIS_A + _BASIS_B:
        _b(s, f, w)
    _b(s, "veranlagung", "zusammen")
    return s


def _mit_kirchensteuer(basis, konfession_feld, konfession_wert, kirchensteuer_feld, kirchensteuer_cent):
    """Ersetzt kist_konfession[_partner]='keine' durch einen kirchensteuerpflichtigen Wert und
    ergaenzt kirchensteuer_arbeitgeber[_partner] -- Paar-Zwang (E0200301+E0200501 zusammen,
    reports/adjudikation/entscheidungsvorlage_restfehler_2026-08-10.md:279-290): jeder Code
    ausser 'keine' verlangt checkESt zufolge BEIDE Kz auf Anlage N."""
    ersetzt = tuple((f, konfession_wert) if f == konfession_feld else (f, w) for f, w in basis)
    return ersetzt + ((kirchensteuer_feld, kirchensteuer_cent),)


def _fall_einzel_kirchensteuerpflichtig():
    s = ST.leerer_store(2025, fall_id="durchstich_einzel_kist")
    for f, w in _mit_kirchensteuer(_BASIS_A, "kist_konfession", "evangelisch",
                                   "kirchensteuer_arbeitgeber", 100000):
        _b(s, f, w)
    _b(s, "veranlagung", "einzel")
    return s


def _fall_zusammen_kirchensteuerpflichtig():
    s = ST.leerer_store(2025, fall_id="durchstich_zusammen_kist")
    a = _mit_kirchensteuer(_BASIS_A, "kist_konfession", "evangelisch",
                           "kirchensteuer_arbeitgeber", 100000)
    b = _mit_kirchensteuer(_BASIS_B, "kist_konfession_partner", "roemisch-katholisch",
                           "kirchensteuer_arbeitgeber_partner", 90000)
    for f, w in a + b:
        _b(s, f, w)
    _b(s, "veranlagung", "zusammen")
    return s


# Absender-Stammdaten fuer den Vorsatz-Block. Sie liegen noch nicht als Fall-Felder vor
# (Bau laeuft), muessen fuer den ABGABE-Pfad aber gesetzt sein — erzeuge_xml() verlangt sie
# fail-closed bei abgabefaehig=True. Werte aus dem amtlichen Beispiel-XML
# (elster/testdaten/est_2020_amtliches_beispiel.xml): die Steuernummer MUSS mit der
# Finanzamtsnummer des Empfaengers beginnen (9181), sonst weist der Writer sie zurueck.
_ABSENDER = dict(absender_name="Maier Hans", absender_strasse="Musterstr. 55",
                 absender_plz="55555", absender_ort="Musterort",
                 absender_steuernummer="9181081508155")


def _pruefe(store) -> tuple[int, list[str], str]:
    """Echter Pfad bis zum amtlichen Plugin, auf der ABGABE-Variante.

    `abgabefaehig=True` ist hier nicht optional: ohne das Flag haengt erzeuge_xml() den
    <Vorsatz>-Block nicht an, und die Ratsche wuerde einen Pfad messen, der nie eingereicht
    wird. Gemessen 2026-08-09 nach e365a37: ohne Flag bleiben es 18 Fehler (Vorsatz fehlt
    im XML), mit Flag faellt der Vorsatz-Block weg. Eine Ratsche auf dem Nicht-Abgabe-Pfad
    haette den Fortschritt nie gesehen.

    Rueckgabe (rc, texte, antwort): `antwort` ist die rohe Ericantwort -- CE.gekappt_verdacht()
    braucht den Puffer selbst, nicht die schon geparste texte-Liste.
    """
    snap, _ = ST.materialisiere(store)
    xml = EX.erzeuge_xml(est_mapping.deklariere(snap, TR.lade_bindung()),
                         vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split())
             for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    return rc, texte, antwort


@braucht_eric
@pytest.mark.parametrize("name,bauer,grenze", [
    ("einzel", _fall_einzel, RESTFEHLER_EINZEL),
    ("zusammen", _fall_zusammen, RESTFEHLER_ZUSAMMEN),
])
def test_produkt_xml_erreicht_die_amtliche_pruefung(name, bauer, grenze):
    """Das erzeugte XML muss bis zur Plausibilitaet durchkommen — nicht davor abbrechen.

    Der Unterschied ist entscheidend: rc=610301200 liefert einen LEEREN Fehlerpuffer
    und sieht damit aus wie "keine Beanstandungen", ist aber ein Abbruch VOR der
    Pruefung. Genau diese Verwechslung haelt `klassifiziere_rc` auseinander.
    """
    rc, texte, _ = _pruefe(bauer())
    klasse = CE.klassifiziere_rc(rc)
    assert klasse != "io_gate_nicht_geprueft", (
        f"[{name}] rc={rc}: das XML bricht VOR der Plausibilitaetspruefung ab. "
        f"Ein leerer Fehlerpuffer heisst hier NICHT fehlerfrei. Details in eric.log.")
    assert klasse != "hersteller_id_gesperrt", (
        f"[{name}] rc={rc}: Hersteller-ID abgelehnt — Testaufbau falsch, nicht das XML.")
    assert rc in (CE.RC_OK, CE.RC_PLAUSIBILITAET), (
        f"[{name}] unerwarteter rc={rc} [{klasse}] — weder plausibel noch "
        f"Plausibilitaetsfehler. Das ist ein neuer Fehlermodus.")


@braucht_eric
@pytest.mark.parametrize("name,bauer,grenze", [
    ("einzel", _fall_einzel, RESTFEHLER_EINZEL),
    ("zusammen", _fall_zusammen, RESTFEHLER_ZUSAMMEN),
])
def test_restfehler_ratsche(name, bauer, grenze):
    """Die Zahl amtlicher Beanstandungen darf nur sinken — und muss dann eingetragen werden."""
    rc, texte, antwort = _pruefe(bauer())
    klasse = CE.klassifiziere_rc(rc)
    if klasse in CE.NICHT_GEPRUEFT_KLASSEN:
        pytest.skip(f"[{name}] rc={rc} [{klasse}]: nicht geprueft, kein Restfehler-Urteil "
                    f"moeglich. Leerer Puffer heisst hier NICHT fehlerfrei.")
    if rc == CE.RC_OK:
        assert grenze == 0, (
            f"[{name}] checkESt meldet rc=0 (abgabefaehig!), die Ratsche steht aber "
            f"noch auf {grenze}. Setze RESTFEHLER_{name.upper()} = 0.")
        return
    assert not CE.gekappt_verdacht(antwort), "Puffer gekappt — Zahl waere nicht belastbar"
    assert len(texte) <= grenze, (
        f"[{name}] REGRESSION: {len(texte)} amtliche Fehler, erlaubt sind {grenze}.\n"
        + "\n".join(f"  - {t[:160]}" for t in texte))
    assert len(texte) == grenze, (
        f"[{name}] FORTSCHRITT: nur noch {len(texte)} statt {grenze} Fehler. "
        f"Trag das ein: RESTFEHLER_{name.upper()} = {len(texte)}.\n"
        f"Verbleibend:\n" + "\n".join(f"  - {t[:160]}" for t in texte))


@braucht_eric
@pytest.mark.parametrize("name,bauer", [
    ("einzel", _fall_einzel_kirchensteuerpflichtig),
    ("zusammen", _fall_zusammen_kirchensteuerpflichtig),
])
def test_restfehler_kirchensteuerpflichtig(name, bauer):
    """Explizite Messung des kirchensteuerpflichtigen Pfads (Paar-Zwang E0200301+E0200501):
    kist_konfession != 'keine' + kirchensteuer_arbeitgeber[_partner] mitdeklariert -> amtlich
    weiterhin rc=RC_OK, keine neuen Beanstandungen gegenueber dem konfessionslosen Basisfall."""
    rc, texte, _ = _pruefe(bauer())
    assert rc == CE.RC_OK, (
        f"[{name}] kirchensteuerpflichtig: rc={rc} (erwartet RC_OK/abgabefaehig). "
        f"Beanstandungen: {texte}")


# Gemessen 2026-08-12 (2 Fehler), nachgemessen 2026-08-20 (1 Fehler).
#
# WEG war die "Einzelangaben zu Gewinnen laut gesonderter Feststellung"-Beanstandung. Sie kam
# nicht daher, dass uns Einzelangaben fehlten, sondern daher, dass wir im FALSCHEN Container
# standen: einkuenfte_gewinn deklarierte nach G/Gew/Ges_Fest/Sum (E0800502) -- der gesondert
# festgestellte Anteil an einer Personengesellschaft, dessen Einz-Block Finanzamt und
# Steuernummer DER GESELLSCHAFT fuehrt. Unser Feld fragt aber den selbst ermittelten Gewinn ab.
# Seit der Container-Korrektur (E0800302, G/Gew/Einz_U/Betr_1_2, plus die Bezeichnung E0800301
# im selben Block) faellt die Klasse ganz weg. Details in produkt/mapping/est_mapping.py.
#
# DARUNTER lag eine andere Klasse, die dann sichtbar wurde: "Sie haben angegeben, dass Sie
# Angaben fuer 'PersonA' machen moechten, haben aber ausser der Angabe im Feld
# '$/G[1]/Person[1]$' keine weiteren Angaben getaetigt." In diesem Fall hat NUR der Partner
# einen Betrieb; der Writer legte fuer Person B die zweite Anlage-G-Instanz an und brauchte dafuer
# die erste -- die dann leer blieb bis auf ihr Person-Indexfeld. Eine Writer-Naht (leere
# Anlagen-Instanz), keine fehlende Angabe.
#
# WEG seit dem Instanz-Fix (2026-08-20, produkt/import/elster_xml.py:_person_b_index): erklaert
# Person A in einem Person-Container nichts, rueckt Person B auf Instanz 0 -- mit explizitem
# person_override, damit der Diskriminator nicht ueber den Index zu "PersonA" wird. Die Klasse
# war nicht auf Anlage G beschraenkt: dieselbe Beanstandung mit '$/N[1]/Person[1]$', wenn nur
# der Partner Arbeitslohn hat. Beide gemessen 1 -> 0 (ERiC 44.2.4.0), Gegenprobe "beide haben
# Lohn" war und bleibt rc=0. Der N-Fall haengt als eigener Test unten
# (test_nur_partner_hat_anlage_n_keine_leere_person_a_huelle).
RESTFEHLER_ZUSAMMEN_GEWINN_PARTNER = 0


def _fall_zusammen_mit_gewinn_partner():
    """Zusammen-Basisfall (rc=0, RESTFEHLER_ZUSAMMEN) PLUS Person-B-Gewinneinkuenfte
    (Task Gewinneinkuenfte-Partnerseite Stufe 1, 2026-08-12): einkuenfte_gewinn_partner
    (gewerbe) -> E0800502, dieselbe Kz-Instanz wie Person A, zweite Anlage-G-Instanz."""
    s = ST.leerer_store(2025, fall_id="durchstich_zusammen_gewinn_partner")
    a = tuple((f, False) if f == "kein_gewinn" else (f, w) for f, w in _BASIS_A)
    for f, w in a + _BASIS_B:
        _b(s, f, w)
    _b(s, "einkuenfte_gewinn_partner", 500000)   # 5.000 EUR
    _b(s, "gewinn_betriebsart_partner", "gewerbe")
    _b(s, "gewinn_bezeichnung_partner", "Grafikdesign")   # Formalie des Betriebs-Blocks B
    _b(s, "veranlagung", "zusammen")
    return s


@braucht_eric
def test_restfehler_zusammen_mit_gewinneinkuenfte_partner():
    """Ratsche (wie test_restfehler_ratsche oben): misst ehrlich, statt rc=0 zu behaupten.
    Steht seit dem Instanz-Fix auf 0 (s. Kommentar bei RESTFEHLER_ZUSAMMEN_GEWINN_PARTNER)."""
    rc, texte, antwort = _pruefe(_fall_zusammen_mit_gewinn_partner())
    klasse = CE.klassifiziere_rc(rc)
    if klasse in CE.NICHT_GEPRUEFT_KLASSEN:
        pytest.skip(f"rc={rc} [{klasse}]: nicht geprueft, kein Restfehler-Urteil moeglich. "
                    f"Leerer Puffer heisst hier NICHT fehlerfrei.")
    if rc == CE.RC_OK:
        assert RESTFEHLER_ZUSAMMEN_GEWINN_PARTNER == 0, (
            f"checkESt meldet rc=0, die Ratsche steht aber noch auf "
            f"{RESTFEHLER_ZUSAMMEN_GEWINN_PARTNER}. Setze die Konstante auf 0.")
        return
    assert not CE.gekappt_verdacht(antwort), "Puffer gekappt — Zahl waere nicht belastbar"
    assert len(texte) <= RESTFEHLER_ZUSAMMEN_GEWINN_PARTNER, (
        f"REGRESSION: {len(texte)} amtliche Fehler, erlaubt sind "
        f"{RESTFEHLER_ZUSAMMEN_GEWINN_PARTNER}.\n" + "\n".join(f"  - {t[:200]}" for t in texte))
    assert len(texte) == RESTFEHLER_ZUSAMMEN_GEWINN_PARTNER, (
        f"FORTSCHRITT: nur noch {len(texte)} statt {RESTFEHLER_ZUSAMMEN_GEWINN_PARTNER} Fehler. "
        f"Trag das ein.\nVerbleibend:\n" + "\n".join(f"  - {t[:200]}" for t in texte))


# Anlage-N-Felder von Person A. Sie muessen fuer den Fall unten RAUS, sonst belegt Person A
# die Anlage N und die zu messende Klasse entsteht gar nicht.
_N_FELDER_A = frozenset({"bruttoarbeitslohn", "steuerklasse", "p36_lohnsteuer"})


def _fall_nur_partner_hat_lohn():
    """Person A hat KEINE Anlage-N-Angabe (dafuer einen Betrieb), Person B hat Arbeitslohn.

    Der zweite gemessene Fall der Klasse 'leere Person-A-Huelle' — andere Anlage als der
    Gewinn-Fall oben, damit die Ratsche nicht eine Einzelfall-Korrektur fuer eine Klasse haelt.
    Vor dem Instanz-Fix: 1 Beanstandung, '$/N[1]/Person[1]$' statt '$/G[1]/Person[1]$'.
    """
    s = ST.leerer_store(2025, fall_id="durchstich_nur_partner_lohn")
    a = tuple((f, w) for f, w in _BASIS_A if f not in _N_FELDER_A)
    a = tuple((f, False) if f == "kein_gewinn" else (f, w) for f, w in a)
    for f, w in a + _BASIS_B:
        _b(s, f, w)
    _b(s, "einkuenfte_gewinn", 4000000)            # 40.000 EUR, Person A -> Anlage G
    _b(s, "gewinn_betriebsart", "gewerbe")
    _b(s, "gewinn_bezeichnung", "Softwareentwicklung")
    _b(s, "veranlagung", "zusammen")
    return s


@braucht_eric
def test_nur_partner_hat_anlage_n_keine_leere_person_a_huelle():
    """Zweiter Fall derselben Klasse, andere Anlage (N statt G): amtlich rc=0.

    Kein Ratschen-Kommentar noetig — die Klasse ist zu, und ein Rueckfall waere hier eine
    echte Regression, keine Restfehler-Zahl."""
    rc, texte, _ = _pruefe(_fall_nur_partner_hat_lohn())
    assert rc == CE.RC_OK, (
        f"Nur-Partner-hat-Lohn: rc={rc} [{CE.klassifiziere_rc(rc)}], erwartet RC_OK. "
        f"Beanstandungen:\n" + "\n".join(f"  - {t[:200]}" for t in texte))


@braucht_eric
def test_steuernummer_ableitung_liefert_dieselbe_amtliche_fehlerzahl():
    """Die Messung, auf die es ankommt (Auftrag 2026-08-10): stammdaten_steuernummer im Fall
    setzen (s. _STAMM_A), erzeuge_xml(abgabefaehig=True) OHNE absender_steuernummer aufrufen,
    dafuer mit `snapshot=snap` -- checkESt muss dieselbe Fehlerzahl liefern wie mit dem
    expliziten Parameter aus _ABSENDER. Sonst waere die Ableitung nur syntaktisch aequivalent,
    nicht amtlich."""
    snap, _ = ST.materialisiere(_fall_einzel())
    dekl = est_mapping.deklariere(snap, TR.lade_bindung())
    absender_ohne_stnr = {k: v for k, v in _ABSENDER.items() if k != "absender_steuernummer"}

    xml_explizit = EX.erzeuge_xml(dekl, vz=2025, hersteller_id=_HID, abgabefaehig=True,
                                  **_ABSENDER)
    xml_abgeleitet = EX.erzeuge_xml(dekl, vz=2025, hersteller_id=_HID, abgabefaehig=True,
                                    snapshot=snap, **absender_ohne_stnr)

    assert "9181081508155" not in str(absender_ohne_stnr)  # Vorbedingung: Parameter fehlt wirklich

    rc1, antwort1 = CE.validate(xml_explizit, "ESt_2025")
    rc2, antwort2 = CE.validate(xml_abgeleitet, "ESt_2025")
    for rc, seite in ((rc1, "explizit"), (rc2, "abgeleitet")):
        klasse = CE.klassifiziere_rc(rc)
        if klasse in CE.NICHT_GEPRUEFT_KLASSEN:
            pytest.skip(f"{seite}: rc={rc} [{klasse}]: nicht geprueft, kein Vergleich "
                        f"moeglich. Leerer Puffer heisst hier NICHT fehlerfrei.")
    n1 = len(re.findall(r"<Text>", antwort1 or ""))
    n2 = len(re.findall(r"<Text>", antwort2 or ""))
    assert n1 == n2, (
        f"Ableitung ueber snapshot liefert eine ANDERE amtliche Fehlerzahl als der explizite "
        f"absender_steuernummer-Parameter ({n2} vs {n1}) -- keine echte Aequivalenz.")
