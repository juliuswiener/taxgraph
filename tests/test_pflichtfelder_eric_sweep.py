"""ERiC-Sweep gegen est_mapping.PFLICHTFELDER selbst: mehrere Fixturen, Gruppen-Gegenprobe in
beide Richtungen, Abweichung in beide Richtungen als Befund.

Warum es das gibt
------------------
Ergaenzt test_vollstaendig_pflichtfelder_voll.py. Dort: EIN Fall (_fall_einzel), jedes Kz-Feld
einzeln weg -- prueft die PRUEF-LOGIK (_pflichtfelder_luecken/pflichtfelder_vollstaendig).
Hier: die gepflegte LISTE selbst (est_mapping.PFLICHTFELDER) gegen ECHTES ERiC absichern, ueber
mehrere Fixturen hinweg -- ein Feld zaehlt nur dann "immer pflichtig", wenn sein Fehlen in JEDER
gepruefter Fixtur ablehnt, nicht nur in _fall_einzel (Auftrag @main, Round 4c, 2026-08-30).

Rentner-Fixtur (_fall_rentner): bewusst OHNE Anlage-R-Rentenzeile (rentner_renten_art bleibt
unangetastet) -- Rentner OHNE Rentenzeile, NICHT Rentner MIT Rente. Grund, gemessen 2026-08-30
und an @main gemeldet: E1800501 (Beginn der Rente, XSD DatumTTpMMpJJJJBekanntBaseCType) wird vom
Writer (elster_xml._wert_text) als reiner str(Jahr-int) geschrieben, kein TT.MM.JJJJ -- ERiC
lehnt deshalb JEDE echte Anlage-R-Deklaration ab, unabhaengig vom Pflichtfelder-Thema hier
(Reichweitenmessung laeuft separat). Diese Fixtur deckt trotzdem den Kern, um den es bei den
alle_oder_keins-Gruppen geht: Anlage-N-Trio und RV-Paar duerfen bei fehlendem Lohn KOMPLETT
fehlen, die sieben "immer"-Stammdatenfelder bleiben trotzdem Pflicht.

Gruppen-Gegenprobe -- gemessen, nicht angenommen
-------------------------------------------------
Erwartung vor der Messung war "ein Feld der Gruppe weg -> ERiC akzeptiert, die ganze Gruppe weg
-> ERiC lehnt ab" (klassische "mindestens ein Feld genuegt"-Lesart). GEMESSEN (2026-08-30, gegen
_fall_einzel UND _fall_zusammen) ist es fuer BEIDE Gruppen genau UMGEKEHRT: ein einzelnes Feld
fehlt, der Rest der Gruppe bleibt -> ERiC LEHNT AB. Die GANZE Gruppe fehlt -> ERiC AKZEPTIERT.
Die Gruppen sind also eher "alles-oder-nichts, sobald beruehrt" als "mindestens eines genuegt".

Das CORRECTS die urspruengliche Erwartung, nicht die Liste: est_mapping._pflichtfelder_luecken
implementiert bereits "Gruppe unberuehrt -> kein Pflichtfall, Gruppe beruehrt -> alle Felder
Pflicht" -- das trifft das gemessene ERiC-Verhalten fuer beide Gruppen richtig. Der Bedingungsname
hiess bis zu diesem Fund "mindestens_eins" und beschrieb es damit falsch (klang nach "irgendeins
reicht", war aber "alle oder keins") -- umbenannt auf "alle_oder_keins" (est_mapping.py,
Julius-Regel 2026-08-30: nicht dem alten Namen einen zweiten Sinn geben, den Namen richtigstellen).

Eine Ausnahme bleibt beim Anlage-N-Trio, und sie ist ein Befund, keine Fussnote: die Teilmenge
"nur steuerklasse gesetzt" (bruttoarbeitslohn UND p36_lohnsteuer fehlen) akzeptiert ERiC trotzdem
(rc=0) -- die Code-Logik flaggt das als unvollstaendig (pflichtfelder_vollstaendig=False). Fuer
die ABGABE ist das sicher (fail-closed, kein Sicherheitsloch). Fuer den NUTZER ist es eine falsche
Sperre in der leisen Richtung: er sieht "unvollstaendig", obwohl das Finanzamt seine Erklaerung
genommen haette. Nicht repariert -- s. test_gruppen_gegenprobe_beide_richtungen unten.

Ueberspringt sauber, wenn ERiC oder die Hersteller-ID fehlen (gleiches Muster wie
test_checkest_durchstich.braucht_eric) -- meldet den Grund, statt falsch gruen zu laufen.
"""
from __future__ import annotations

import itertools
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/import", "produkt/mapping", "produkt/store",
             "produkt/traverser", "elster"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import checkest_gate as CE   # noqa: E402
import elster_xml as EX      # noqa: E402
import est_mapping           # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402

from test_checkest_durchstich import (  # noqa: E402
    _ABSENDER, _HID, _b, braucht_eric, _fall_einzel, _fall_zusammen,
)

TS = "2026-08-30T16:00:00+00:00"

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

_BASIS_RENTNER = (
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_verlust_aktien", 0),
    ("kap_verlust_sonstige", 0),
    ("vor_rv_ausserhalb_lstb", 0),
    ("versicherungsart", "gesetzlich_an"), ("basis_kv", 0), ("basis_pv", 0),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
) + _STAMM_RENTNER


def _fall_rentner():
    """Rentner OHNE Rentenzeile, nicht Rentner MIT Rente -- s. Modul-Docstring fuer den Grund
    (E1800501-Datumsformat-Luecke im Writer, 2026-08-30 an @main gemeldet, Reichweite bei
    knopf-bau in Arbeit)."""
    s = ST.leerer_store(2025, fall_id="pflichtfelder_sweep_rentner")
    for f, w in _BASIS_RENTNER:
        _b(s, f, w)
    _b(s, "veranlagung", "einzel")
    return s


FIXTUREN = (
    ("einzel", _fall_einzel),
    ("zusammen", _fall_zusammen),
    ("rentner", _fall_rentner),
)

IMMER_FELDER = next(
    g for g in est_mapping.PFLICHTFELDER if g["bedingung"] == "immer")["felder"]
TRIO = next(
    g for g in est_mapping.PFLICHTFELDER
    if g["bedingung"] == "alle_oder_keins" and "bruttoarbeitslohn" in g["felder"])["felder"]
PAAR = next(
    g for g in est_mapping.PFLICHTFELDER
    if g["bedingung"] == "alle_oder_keins" and "vor_an_anteil_rv" in g["felder"])["felder"]


def _dekl_ohne(entfernen, felder_basis: dict, bindung: dict):
    """Wie _ohne_feld in test_vollstaendig_pflichtfelder_voll, aber ueber eine Feld-MENGE statt
    genau ein Feld, und mit vorab materialisierter Basis (kein erneuter Store-Aufbau pro Ruf)."""
    felder = dict(felder_basis)
    for f in entfernen:
        felder.pop(f, None)
    dekl = est_mapping.deklariere(felder, bindung)
    try:
        xml = EX.erzeuge_xml(dekl, vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    except EX.XmlFehler as exc:
        return dekl["pflichtfelder_vollstaendig"], "WRITER_ABBRUCH", str(exc)
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split()) for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    return dekl["pflichtfelder_vollstaendig"], rc, (texte[0] if texte else "")


def _teilmengen_ohne_volle_und_leere(gruppe):
    """Alle nichtleeren ECHTEN Teilmengen von `gruppe` (weder alles noch nichts weg)."""
    for r in range(1, len(gruppe)):
        yield from itertools.combinations(gruppe, r)


@braucht_eric
def test_fixturen_selbst_sind_sauber():
    """Sanity vor allen Sweeps: jede Fixtur muss VOR jeder Feld-Entfernung selbst rc=0 liefern --
    sonst sagen die nachfolgenden Feld-Sweeps nichts (Fixtur schon kaputt, nicht das entfernte
    Feld)."""
    bindung = TR.lade_bindung()
    unsauber = []
    for name, fixtur_fn in FIXTUREN:
        felder, sid = ST.materialisiere(fixtur_fn())
        dekl = est_mapping.deklariere(felder, bindung, snapshot_id=sid)
        xml = EX.erzeuge_xml(dekl, vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
        rc, antwort = CE.validate(xml, "ESt_2025")
        if rc != CE.RC_OK:
            texte = [" ".join(t.split())
                     for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
            unsauber.append(f"{name}: rc={rc} -- {texte[:2]}")
    assert not unsauber, (
        "Fixtur(en) selbst nicht sauber (rc!=0) -- alle Feld-Sweeps darauf sind nicht "
        "aussagekraeftig:\n" + "\n".join(unsauber))


@braucht_eric
def test_immer_felder_in_jeder_fixtur_pflicht():
    """Requirement 1: ein 'immer'-Feld zaehlt nur dann fixturuebergreifend pflichtig, wenn sein
    Fehlen in JEDER Fixtur (in der es ueberhaupt gesetzt ist) ablehnt. Fail-closed wie
    test_vollstaendig_pflichtfelder_voll: rot, sobald pflichtfelder_vollstaendig=True bleibt,
    obwohl ERiC in irgendeiner Fixtur ablehnt (kann fuer 'immer'-Felder per Konstruktion nicht
    eintreten, bleibt aber als Regression-Netz stehen, s. Kommentar unten)."""
    bindung = TR.lade_bindung()
    luecken = []
    nicht_ueberall = []
    for feld_id in IMMER_FELDER:
        akzeptiert_in = []
        for name, fixtur_fn in FIXTUREN:
            felder_basis, _sid = ST.materialisiere(fixtur_fn())
            if feld_id not in felder_basis:
                continue
            pflicht_vollstaendig, rc, text = _dekl_ohne((feld_id,), felder_basis, bindung)
            if rc == "WRITER_ABBRUCH":
                continue                    # Writer selbst faengt es fail-closed ab -- kein Blindspot
            if rc == CE.RC_OK:
                akzeptiert_in.append(name)
                continue
            # rc lehnt ab: pflichtfelder_vollstaendig darf hier NIE True sein (immer-Gruppe hat
            # keine "unberuehrt"-Ausnahme wie alle_oder_keins -- Fehlen wird immer geflaggt).
            if pflicht_vollstaendig:
                luecken.append(
                    f"{feld_id} in {name}: pflichtfelder_vollstaendig=True, ERiC rc={rc} "
                    f"-> {text[:120]}")
        if akzeptiert_in:
            nicht_ueberall.append(
                f"{feld_id}: ERiC akzeptiert Fehlen in {akzeptiert_in} -- nicht fixturuebergreifend Pflicht")

    assert not luecken, (
        f"{len(luecken)} 'immer'-Pflichtfelder: pflichtfelder_vollstaendig=True, obwohl ERiC "
        f"in mindestens einer Fixtur ablehnt.\n" + "\n".join(luecken))
    if nicht_ueberall:
        print("\nBEFUND (Requirement 4b -- Liste evtl. zu eng gefasst, nicht sicherheitsrelevant "
              "da fail-closed-Richtung unberuehrt):\n" + "\n".join(nicht_ueberall))


@braucht_eric
def test_gruppen_gegenprobe_beide_richtungen():
    """Requirement 2: fuer TRIO und PAAR, in _fall_einzel UND _fall_zusammen (beide tragen beide
    Gruppen vollstaendig), beide Richtungen der Gegenprobe:
      (a) die GANZE Gruppe weg -> muss ERiC akzeptieren (hartes Gate -- das ist exakt die
          Bedingung, unter der _pflichtfelder_luecken das Fehlen ueberhaupt toleriert).
      (b) jede echte, nichtleere Teilmenge weg -> nur festgehalten (informativ), s. Modul-
          Docstring fuer die gemessene, von der urspruenglichen Annahme abweichende Realitaet.
    _fall_rentner testet die Gruppen NICHT hier erneut -- dort sind beide Gruppen von Anfang an
    komplett unberuehrt (kein Lohn/RV-Feld gesetzt), das ist bereits die volle Abwesenheit, die
    (a) unten fuer einzel/zusammen explizit herstellt; test_fixturen_selbst_sind_sauber bestaetigt
    bereits, dass dieser Zustand fuer sich genommen rc=0 liefert.
    """
    bindung = TR.lade_bindung()
    unerwartet_abgelehnt = []
    teilmengen_befund = []

    for name, fixtur_fn in (("einzel", _fall_einzel), ("zusammen", _fall_zusammen)):
        felder_basis, _sid = ST.materialisiere(fixtur_fn())
        if not (all(f in felder_basis for f in TRIO) and all(f in felder_basis for f in PAAR)):
            continue  # Fixtur traegt die Gruppe(n) gar nicht -- ueberspringen (hier: keine)

        for gruppe, label in ((TRIO, "trio"), (PAAR, "paar")):
            for teilmenge in _teilmengen_ohne_volle_und_leere(gruppe):
                _, rc, _text = _dekl_ohne(teilmenge, felder_basis, bindung)
                zustand = "akzeptiert" if rc == CE.RC_OK else f"abgelehnt(rc={rc})"
                teilmengen_befund.append(f"{name}/{label} ohne {teilmenge} -> {zustand}")

            _, rc, text = _dekl_ohne(gruppe, felder_basis, bindung)
            if rc != CE.RC_OK:
                unerwartet_abgelehnt.append(
                    f"{name}/{label} komplett weg: rc={rc} -- {text[:120]}")

    assert not unerwartet_abgelehnt, (
        "Gruppe(n) komplett entfernt, aber ERiC lehnt trotzdem ab -- _pflichtfelder_luecken "
        "toleriert das Fehlen faelschlich (Sicherheitsloch, nicht nur Befund):\n"
        + "\n".join(unerwartet_abgelehnt))

    print("\nTeilmengen-Befund (Requirement 2b, Dokumentation -- keine Assertion, s. Modul-"
          "Docstring fuer Einordnung):\n" + "\n".join(teilmengen_befund))
