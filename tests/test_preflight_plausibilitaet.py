"""Gate für die Kreuz-Plausibilisierung (produkt/konsistenz/preflight.py). Deterministisch, NULL LLM.

Die Werte in den Positivtests sind nicht ausgedacht: sie stammen aus einem echten Durchgang vom
2026-08-27, bei dem 40.000 € Bruttoarbeitslohn zusammen mit 12.123.213 € Lohnsteuer, 243.234 €
eigenem Rentenversicherungsanteil, 22.222 € Kirchensteuer und 234.234 € Schulgeld für ein Kind
abgegeben werden konnten, ohne dass irgendetwas anschlug.

Zu jeder Prüfung gehört hier ein Negativtest mit einem plausiblen Wert. Der ist der wichtigere von
beiden: eine Prüfung, die auch bei richtigen Angaben meldet, macht die Hinweisliste wertlos, und der
Nutzer gewöhnt sich an, sie wegzuklicken. Dazu je Prüfung der Fall „Bezugsgröße fehlt" — dann muss
geschwiegen werden, denn dann ist die Unplausibilität eine Aussage über unser Nichtwissen.
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                "produkt", "konsistenz"))
import preflight as PF   # noqa: E402


def _snap(**felder):
    """{feld_id: (wert, zustand)} -> Snapshot-felder-Ebene."""
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


def _b(wert):
    """Kurzform für einen bestätigten Wert."""
    return (wert, "bestaetigt")


BRUTTO_40K = 4_000_000        # 40.000 € in Cent — die Bezugsgröße des gemessenen Falls


def _felder(w):
    return {x["feld_id"] for x in w}


# ---- Lohnsteuer ↔ Bruttoarbeitslohn ------------------------------------------

def test_lohnsteuer_ueber_dem_lohn_meldet():
    """Der gemessene Fall: 12.123.213 € Lohnsteuer auf 40.000 € Lohn (Faktor 300)."""
    w = PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), p36_lohnsteuer=_b(1_212_321_300)))
    assert _felder(w) == {"p36_lohnsteuer"}
    assert "12.123.213 €" in w[0]["grund"] and "40.000 €" in w[0]["grund"]


def test_lohnsteuer_im_ueblichen_rahmen_meldet_nicht():
    """5.000 € Lohnsteuer auf 40.000 € Lohn ist ein ganz normaler Arbeitnehmerfall."""
    assert PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), p36_lohnsteuer=_b(500_000))) == []


def test_lohnsteuer_ohne_bruttolohn_schweigt():
    """Ohne Bezugsgröße keine Meldung — sonst meldeten wir unser eigenes Nichtwissen."""
    assert PF.plausibilitaets_widersprueche(_snap(p36_lohnsteuer=_b(1_212_321_300))) == []


def test_lohnsteuer_bei_unbestaetigtem_bruttolohn_schweigt():
    """Ein vorläufiger Bruttolohn ist kein Beleg — dieselbe Regel wie in den Nachbarprüfungen."""
    assert PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=(BRUTTO_40K, "vorlaeufig"),
        p36_lohnsteuer=_b(1_212_321_300))) == []


# ---- Rentenversicherungsbeiträge ↔ Bruttoarbeitslohn -------------------------

def test_rv_eigenanteil_ueber_dem_lohn_meldet():
    """243.234 € eigener Anteil auf 40.000 € Lohn; real sind es rund 3.700 €."""
    w = PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), vor_an_anteil_rv=_b(24_323_400)))
    assert _felder(w) == {"vor_an_anteil_rv"}


def test_rv_arbeitgeberanteil_ueber_dem_lohn_meldet():
    """Der zweite gemessene Betrag, 4.234.234 €, kam über den Arbeitgeberanteil herein."""
    w = PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), vor_ag_anteil_rv=_b(423_423_400)))
    assert _felder(w) == {"vor_ag_anteil_rv"}


def test_rv_realistischer_beitrag_meldet_nicht():
    """3.720 € eigener und Arbeitgeberanteil auf 40.000 € Lohn — der Normalfall."""
    assert PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K),
        vor_an_anteil_rv=_b(372_000), vor_ag_anteil_rv=_b(372_000))) == []


def test_rv_ohne_bruttolohn_schweigt():
    assert PF.plausibilitaets_widersprueche(_snap(vor_an_anteil_rv=_b(24_323_400))) == []


# ---- Kirchensteuer ↔ Bruttoarbeitslohn ---------------------------------------

# Wer Kirchensteuer zahlt, gehört einer Kirche an. Die Betrags-Prüfungen unten brauchen die
# Mitgliedschaft nicht — ein Snapshot ohne sie wäre aber in sich widersprüchlich, und seit
# 2026-08-28 meldet genau das eine eigene Prüfung (s. „Kirchensteuer erklärt, Konfession offen").
# Ohne diese Angabe schlüge in jedem Test hier zusätzlich jene Prüfung an, und die Negativtests
# könnten nicht mehr zeigen, dass über den BETRAG geschwiegen wird. Die beiden Form-Tests am
# Dateiende lassen sie bewusst weg, damit der neue Text dort mitgeprüft wird.
KIRCHENMITGLIED = _b("roemisch-katholisch")


def test_kirchensteuer_ueber_dem_lohnanteil_meldet():
    """22.222 € Kirchensteuer auf 40.000 € Lohn — mehr als die Hälfte des Lohns."""
    w = PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), kist_gezahlt=_b(2_222_200),
        kist_konfession=KIRCHENMITGLIED))
    assert _felder(w) == {"kist_gezahlt"}
    assert "22.222 €" in w[0]["grund"]


def test_kirchensteuer_in_richtiger_groessenordnung_meldet_nicht():
    """Rund 500 € Kirchensteuer auf 40.000 € Lohn ist die reale Größenordnung."""
    assert PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), kist_gezahlt=_b(50_000),
        kist_konfession=KIRCHENMITGLIED)) == []


def test_kirchensteuer_nachzahlung_bleibt_unbeanstandet():
    """Grenzfall, der NICHT melden darf: 3.000 € durch eine Nachzahlung für Vorjahre — die
    Schwelle hat dafür ausdrücklich Luft (Faktor 3 über dem gesetzlichen Höchstsatz)."""
    assert PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), kist_gezahlt=_b(300_000),
        kist_konfession=KIRCHENMITGLIED)) == []


def test_kirchensteuer_ohne_bruttolohn_schweigt():
    assert PF.plausibilitaets_widersprueche(_snap(
        kist_gezahlt=_b(2_222_200), kist_konfession=KIRCHENMITGLIED)) == []


# ---- Schulgeld je Kind --------------------------------------------------------

def test_schulgeld_absurd_hoch_meldet():
    """234.234 € Schulgeld für ein Kind; der Abzug ist ab rund 16.700 € ausgeschöpft."""
    w = PF.plausibilitaets_widersprueche(_snap(schulgeld=_b(23_423_400)))
    assert _felder(w) == {"schulgeld"}
    assert "234.234 €" in w[0]["grund"]


def test_schulgeld_zweites_kind_wird_mitgeprueft():
    """Schulgeld ist ein Instanzfeld (`instanz_gruppe: kind`): Kind 1 steht unter `schulgeld`,
    Kind 2 unter `schulgeld__2`. Wer nur die Basis liest, sieht das zweite Kind nie."""
    w = PF.plausibilitaets_widersprueche(_snap(schulgeld__2=_b(23_423_400)))
    assert _felder(w) == {"schulgeld__2"}


def test_schulgeld_teure_privatschule_meldet_nicht():
    """Der Grenzfall, der durchlaufen muss: 20.000 € an einer internationalen Privatschule."""
    assert PF.plausibilitaets_widersprueche(_snap(schulgeld=_b(2_000_000))) == []


def test_schulgeld_zweiter_betrag_des_gemessenen_falls_meldet_nicht():
    """Im gemessenen Fall stand für das andere Kind 2.332 € — ein völlig normaler Betrag."""
    assert PF.plausibilitaets_widersprueche(_snap(schulgeld=_b(233_200))) == []


# ---- Einbehaltene ↔ gezahlte Kirchensteuer -----------------------------------

def test_kirchensteuer_beide_angaben_weit_auseinander_meldet():
    """Der gemessene Fall: 2.000 € einbehalten, 22.222 € gezahlt — nie abgeglichen.
    Der Bruttolohn fehlt hier bewusst, damit nur dieser Abgleich anschlagen kann."""
    w = PF.plausibilitaets_widersprueche(_snap(
        kirchensteuer_arbeitgeber=_b(200_000), kist_gezahlt=_b(2_222_200),
        kist_konfession=KIRCHENMITGLIED))
    assert _felder(w) == {"kist_gezahlt"}
    assert "2.000 €" in w[0]["grund"] and "22.222 €" in w[0]["grund"]


def test_kirchensteuer_beide_angaben_stimmig_meldet_nicht():
    """450 € einbehalten, 500 € gezahlt — die übliche kleine Abweichung."""
    assert PF.plausibilitaets_widersprueche(_snap(
        kirchensteuer_arbeitgeber=_b(45_000), kist_gezahlt=_b(50_000),
        kist_konfession=KIRCHENMITGLIED)) == []


def test_kirchensteuer_abgleich_braucht_beide_angaben():
    """Nur eine der beiden Zahlen erlaubt keinen Abgleich — dann ist zu schweigen."""
    assert PF.plausibilitaets_widersprueche(_snap(
        kirchensteuer_arbeitgeber=_b(200_000), kist_konfession=KIRCHENMITGLIED)) == []


# ---- Kirchensteuer erklärt ↔ Konfession offen --------------------------------
#
# GEMESSEN 2026-08-28 am Fall serie-verheiratet-1kind-handwerker: Bundesland, gezahlte (580 €)
# und erstattete Kirchensteuer bestätigt, die Konfession nie beantwortet. /ergebnis meldete
# `grund: "bestaetigt"` und 0 € Kirchensteuer — bei römisch-katholisch wären es 1.053,36 €.
# Die Einkommensteuer war dabei richtig (Delta 0 Cent), deshalb MELDET diese Prüfung und sperrt
# nicht: ein Sperrgrund nähme dem Nutzer eine korrekte Zahl weg.
# Der Preflight sagte zu diesem Fall GREEN — er prüfte Kirchensteuer gegen Lohn und gegen den
# Arbeitgeber-Einbehalt, nie gegen die Mitgliedschaft.

def _kirchen_widersprueche(**felder):
    """Nur die Einträge dieser Prüfung — die Nachbarprüfungen schlagen bei denselben Feldern an."""
    return [w for w in PF.plausibilitaets_widersprueche(_snap(**felder))
            if "Kirche" in w["grund"]]


def test_gezahlte_kirchensteuer_ohne_konfession_meldet():
    """Der gemessene Fall: 580 € gezahlt, Bundesland gesetzt, Mitgliedschaft offen."""
    w = _kirchen_widersprueche(kist_gezahlt=_b(58_000),
                               kist_bundesland=_b("nordrhein_westfalen"),
                               kist_erstattet=_b(0))
    assert len(w) == 1, w
    assert "580 €" in w[0]["grund"]


def test_einbehaltene_kirchensteuer_ohne_konfession_meldet():
    """Der Arbeitgeber-Einbehalt ist derselbe Beleg wie die gezahlte Kirchensteuer: er entsteht
    nur bei einem Kirchenmitglied."""
    assert len(_kirchen_widersprueche(kirchensteuer_arbeitgeber=_b(12_300))) == 1


def test_bundesland_allein_ohne_konfession_meldet():
    """Das Bundesland wird ausschließlich gebraucht, um den Hebesatz einer Kirche anzuwenden —
    es steht also nie ohne Grund da, auch wenn gar kein Betrag erfasst ist."""
    assert len(_kirchen_widersprueche(kist_bundesland=_b("bayern"))) == 1


def test_beantwortete_konfession_schweigt():
    """Die wichtigste Gegenprobe: mit beantworteter Mitgliedschaft ist nichts offen. Ohne sie
    meldete die Prüfung bei JEDEM Kirchensteuerzahler und wäre wertlos."""
    assert _kirchen_widersprueche(kist_gezahlt=_b(58_000),
                                  kist_bundesland=_b("nordrhein_westfalen"),
                                  kist_konfession=KIRCHENMITGLIED) == []


def test_bewusstes_nein_zur_kirche_schweigt():
    """„keine" ist eine Antwort, kein fehlender Wert — dann ist die Kirchensteuer mit 0 korrekt
    gerechnet und es gibt nichts zu melden."""
    assert _kirchen_widersprueche(kist_gezahlt=_b(58_000),
                                  kist_konfession=_b("keine")) == []


def test_ohne_jede_kirchensteuerangabe_schweigt():
    """Wer zur Kirchensteuer gar nichts gesagt hat, wird nicht danach gefragt — sonst bekäme
    jeder Konfessionslose diesen Hinweis, obwohl bei ihm nichts fehlt."""
    assert _kirchen_widersprueche(bruttoarbeitslohn=_b(BRUTTO_40K)) == []


def test_erstattung_von_null_ist_keine_angabe():
    """Eine 0 bei der erstatteten Kirchensteuer sagt nichts über eine Mitgliedschaft — sie ist
    der Normalfall und darf den Hinweis nicht auslösen."""
    assert _kirchen_widersprueche(kist_erstattet=_b(0)) == []


def test_vorlaeufige_kirchensteuer_schweigt():
    """Nur bestätigte Werte sind ein Beleg — ein KI-Vorschlag ist keine Angabe des Nutzers
    (Hausregel dieser Datei: vorläufig ist kein Beleg)."""
    assert PF.plausibilitaets_widersprueche(
        {"kist_gezahlt": {"wert": 58_000, "zustand": "vorlaeufig"}}) == []


# ---- Keine Bankverbindung ↔ erfasste IBAN ------------------------------------

def test_keine_bankverbindung_trotz_iban_meldet():
    w = PF.plausibilitaets_widersprueche(_snap(
        stammdaten_keine_bankverbindung=_b(True), stammdaten_iban=_b("DE02120300000000202051")))
    assert _felder(w) == {"stammdaten_iban"}


def test_iban_ohne_das_flag_meldet_nicht():
    """Der Normalfall: eine IBAN ist erfasst und der Nutzer will eine Erstattung."""
    assert PF.plausibilitaets_widersprueche(_snap(
        stammdaten_keine_bankverbindung=_b(False),
        stammdaten_iban=_b("DE02120300000000202051"))) == []


def test_flag_ohne_iban_meldet_nicht():
    """Der andere Normalfall: kein Konto gewünscht, also auch keins erfasst."""
    assert PF.plausibilitaets_widersprueche(_snap(
        stammdaten_keine_bankverbindung=_b(True))) == []


def test_leere_iban_ist_keine_iban():
    """Ein leergeräumtes Textfeld darf den Widerspruch nicht auslösen."""
    assert PF.plausibilitaets_widersprueche(_snap(
        stammdaten_keine_bankverbindung=_b(True), stammdaten_iban=_b("   "))) == []


# ---- Querschnitt --------------------------------------------------------------

def test_bool_gilt_nicht_als_betrag():
    """True ist in Python eine 1. Ohne den bool-Ausschluss ginge ein bestätigtes Ja-Feld als
    Betrag von einem Cent durch und verfälschte die Verhältnisrechnung lautlos."""
    assert PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(True), p36_lohnsteuer=_b(1_212_321_300))) == []


def test_leerer_snapshot_meldet_nichts():
    assert PF.plausibilitaets_widersprueche({}) == []


def test_kein_feldname_im_text():
    """Die Texte gehen an einen Laien. Feld-Kennungen wie `p36_lohnsteuer` sagen ihm nichts —
    dieselbe Regel, die tests/test_bindung_texte_lesbar.py für die Fragetexte durchsetzt."""
    w = PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), p36_lohnsteuer=_b(1_212_321_300),
        vor_an_anteil_rv=_b(24_323_400), kist_gezahlt=_b(2_222_200),
        kirchensteuer_arbeitgeber=_b(200_000), schulgeld=_b(23_423_400),
        stammdaten_keine_bankverbindung=_b(True), stammdaten_iban=_b("DE02120300000000202051")))
    assert len(w) >= 6
    for eintrag in w:
        for kennung in ("p36_lohnsteuer", "vor_an_anteil_rv", "vor_ag_anteil_rv", "kist_gezahlt",
                        "kirchensteuer_arbeitgeber", "schulgeld", "bruttoarbeitslohn",
                        "stammdaten_iban", "stammdaten_keine_bankverbindung", "_"):
            assert kennung not in eintrag["grund"], f"Feld-Kennung im Laientext: {eintrag['grund']}"


def test_jeder_eintrag_bittet_um_pruefung():
    """Ein Widerspruch, der nur behauptet, hilft nicht. Jeder Text muss sagen, was zu tun ist."""
    w = PF.plausibilitaets_widersprueche(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), p36_lohnsteuer=_b(1_212_321_300),
        vor_an_anteil_rv=_b(24_323_400), kist_gezahlt=_b(2_222_200),
        kirchensteuer_arbeitgeber=_b(200_000), schulgeld=_b(23_423_400),
        stammdaten_keine_bankverbindung=_b(True), stammdaten_iban=_b("DE02120300000000202051")))
    for eintrag in w:
        assert "rüfe" in eintrag["grund"], eintrag["grund"]


# ---- Naht zum Orchestrator ----------------------------------------------------

def test_preflight_reicht_die_plausibilitaet_durch_und_setzt_rot():
    """Der Orchestrator muss den neuen Schlüssel führen und den Status heben — sonst ist die
    Prüfung gebaut, aber niemand sieht sie."""
    ergebnis = PF.preflight(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), p36_lohnsteuer=_b(1_212_321_300)))
    assert len(ergebnis["widersprueche_plausibilitaet"]) == 1
    assert ergebnis["status"] == "RED"


def test_preflight_wird_nicht_rot_bei_stimmigen_zahlen():
    """Gegenprobe zum Test darüber: derselbe Aufbau mit plausibler Lohnsteuer wird nicht rot.
    Ohne ihn wäre nicht gezeigt, dass ROT von DIESER Prüfung kommt und nicht von einer Nachbarin.

    Grün wird es hier nicht, und das ist richtig: ein Bruttoarbeitslohn ohne Arbeitstage lässt die
    (ältere) Pauschalen-Prüfung auf die Entfernungspauschale hinweisen — ein Hinweis, kein
    Widerspruch, also AMBER."""
    ergebnis = PF.preflight(_snap(
        bruttoarbeitslohn=_b(BRUTTO_40K), p36_lohnsteuer=_b(500_000)))
    assert ergebnis["widersprueche_plausibilitaet"] == []
    assert ergebnis["status"] != "RED"
