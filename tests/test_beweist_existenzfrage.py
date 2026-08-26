"""Wer die Anzahl seiner Kinder nennt, muss nicht noch gefragt werden, OB er Kinder hat.

ANLASS, Julius im Durchgang am 2026-08-25, mit Bildschirmfoto:

    „frage gerade beantwortet wird jetzt wieder gestellt"

Darauf sichtbar: unter „SCHON BEANTWORTET" steht *„Für wie viele Kinder stehen dir in diesem Jahr
Kindergeld oder ein Kinderfreibetrag zu? — 2 Kinder"*, und die Karte darüber fragt *„Hast du
Kinder, für die du dieses Jahr Kindergeld oder einen Kinderfreibetrag bekommst?"*

Zwei Felder, eines beweist das andere. GEMESSEN vor dem Bau: mit `fam_anzahl_kinder=2` bestätigt
stand `kein_kind` auf **Position 2 von 320**.

DIE ANDERE HÄLFTE DES BEFUNDS ist die, die man leicht übersieht: die Frage bloss aus der Queue zu
nehmen hätte `kein_kind` LEER gelassen. Der Wert fehlte dann in der Rechnung und in der
ELSTER-Deklaration, und zwar still. Deshalb schreibt die Ableitung ein echtes Event — und deshalb
prüfen die Tests hier den STORE, nicht die Queue.

KEIN LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST       # noqa: E402
import traverser as TR   # noqa: E402

LAIE = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _mit_kinderzahl(n, zustand="bestaetigt", bindung=None):
    b = bindung if bindung is not None else TR.lade_bindung()
    s = ST.leerer_store(veranlagungszeitraum=2025, fall_id="beweis")
    ST.append_event(s, feld_id="fam_anzahl_kinder", wert=n, zustand=zustand, herkunft=LAIE,
                    schreiber="ui:laie",
                    signal={"signal_1": None,
                            "signal_2": "klick@fam_anzahl_kinder" if zustand == "bestaetigt" else None},
                    bindung=b)
    return s, b


# ---------------------------------------------------------------- die Deklaration

def test_die_bindung_erklaert_den_beweis():
    """Untergrenze: ohne den Eintrag ist alles darunter grün und im Betrieb wirkungslos."""
    b = TR.lade_bindung()
    regel = b["fam_anzahl_kinder"].get("beweist")
    assert regel, "fam_anzahl_kinder trägt kein `beweist` — dann fragt der Fragebogen wieder nach."
    assert regel["feld_id"] == "kein_kind"
    assert regel["wert"] is False, (
        "`kein_kind` ist frage_invertiert: „hat Kinder“ heisst im Store False. Stünde hier True, "
        "nähme die Ableitung dem Nutzer seine Kinder aus der Erklärung.")
    assert b["kein_kind"].get("frage_invertiert"), (
        "Die Polarität von kein_kind hat sich geändert — dann stimmt der Wert oben nicht mehr.")


# ---------------------------------------------------------------- der Nutzerpfad

@pytest.mark.parametrize("n", [1, 2, 7])
def test_die_anzahl_beantwortet_die_existenzfrage(n):
    """Julius' Fall. Geprüft wird beides: die Frage ist weg UND der Wert steht."""
    s, b = _mit_kinderzahl(n)
    felder, _ = ST.materialisiere(s)
    assert "kein_kind" in felder, (
        f"{n} Kinder bestätigt, aber kein_kind steht nicht im Store — der Wert fehlt damit still "
        f"in Rechnung und ELSTER-Deklaration.")
    assert felder["kein_kind"]["wert"] is False
    assert felder["kein_kind"]["zustand"] == "bestaetigt"
    assert "kein_kind" not in TR.naechste_fragen(s, b, None), (
        "Die Frage steht trotz beantworteter Anzahl noch in der Queue.")


def test_der_abgeleitete_wert_gibt_sich_nicht_als_eingabe_des_nutzers_aus():
    """Die Oberfläche zeigt je Wert, woher er kommt („tippe auf das Symbol links"). `laie` würde
    dort „✓ selbst" anzeigen — für einen Satz, den der Nutzer nie gesagt hat. Er hat die ZAHL
    gesagt; dass daraus die Existenz folgt, ist unsere Schlussfolgerung, nicht seine Aussage."""
    s, _ = _mit_kinderzahl(2)
    ev = [e for e in s["events"] if e["feld_id"] == "kein_kind"]
    assert len(ev) == 1
    assert ev[0]["herkunft"]["herkunft"] == "berechnet", (
        f"herkunft={ev[0]['herkunft']['herkunft']!r} — die Oberfläche zeigt das als Eingabe des "
        f"Nutzers aus.")
    assert ev[0]["herkunft"]["haftung"] == "nutzer", (
        "Die Haftung bleibt beim Nutzer: er hat die Zahl angegeben.")
    assert ev[0]["schreiber"] == "abgeleitet:beweist"
    assert "fam_anzahl_kinder" in (ev[0]["signal"]["signal_2"] or ""), (
        "Das Signal muss sagen, WORAUS abgeleitet wurde — sonst ist der Wert im Nachhinein nicht "
        "erklärbar (/warum).")


# ---------------------------------------------------------------- die vier Schranken

def test_null_kinder_leitet_nichts_ab():
    """`bereich.min` lässt 0 zu. Aus einer 0 „also keine Kinder" zu schreiben wäre bequem und
    würde dem Nutzer eine Aussage zuschreiben, die er nicht gemacht hat — er könnte die 0
    versehentlich getippt haben. Fail-closed: Frage bleibt stehen."""
    s, b = _mit_kinderzahl(0)
    felder, _ = ST.materialisiere(s)
    assert "kein_kind" not in felder, "Aus 0 Kindern wurde still ein Wert geschrieben."
    assert "kein_kind" in TR.naechste_fragen(s, b, None)


def test_ein_vorlaeufiger_vorschlag_beweist_nichts():
    """Sagt die KI „2 Kinder", steht das vorläufig im Store — der Nutzer hat es noch nicht gesehen.
    Daraus eine BESTÄTIGTE Existenz abzuleiten hiesse, die KI bestätigen zu lassen. Dieselbe Regel
    wie bei `instanz_anzahl`, und dieselbe wie Auflage A für llm:-Schreiber."""
    s, b = _mit_kinderzahl(2, zustand="vorlaeufig")
    felder, _ = ST.materialisiere(s)
    assert "kein_kind" not in felder
    assert "kein_kind" in TR.naechste_fragen(s, b, None)


def test_eine_vorhandene_antwort_wird_nicht_ueberschrieben():
    """Auflage B. Hat der Nutzer die Existenzfrage schon selbst beantwortet, bleibt SEINE Antwort
    stehen — auch wenn sie der Anzahl widerspricht. Ein Widerspruch gehört ihm gezeigt, nicht von
    uns weggeschrieben."""
    b = TR.lade_bindung()
    s = ST.leerer_store(veranlagungszeitraum=2025, fall_id="beweis")
    ST.append_event(s, feld_id="kein_kind", wert=True, zustand="bestaetigt", herkunft=LAIE,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "klick@kein_kind"},
                    bindung=b)
    ST.append_event(s, feld_id="fam_anzahl_kinder", wert=2, zustand="bestaetigt", herkunft=LAIE,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "klick"}, bindung=b)
    ev = [e for e in s["events"] if e["feld_id"] == "kein_kind"]
    assert len(ev) == 1, f"{len(ev)} Events auf kein_kind — die Ableitung hat überschrieben."
    assert ev[0]["herkunft"]["herkunft"] == "laie"


def test_ohne_bindung_wird_nichts_abgeleitet():
    """`bindung` ist in `append_event` optional (Rückwärtskompatibilität der Bestandsaufrufe).
    Ohne sie kennt der Schreibpfad die Regel nicht — dann darf er auch nichts erfinden."""
    s = ST.leerer_store(veranlagungszeitraum=2025, fall_id="beweis")
    ST.append_event(s, feld_id="fam_anzahl_kinder", wert=2, zustand="bestaetigt", herkunft=LAIE,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "k"})
    assert [e for e in s["events"] if e["feld_id"] == "kein_kind"] == []


def test_die_ableitung_loest_keine_kette_aus():
    """Eine Ebene. Trüge das Zielfeld selbst ein `beweist`, liefe die Ableitung sonst weiter — und
    ein Tippfehler in einer YAML-Zeile würde zur Lawine. Geprüft am Ereignisstand: genau zwei
    Events, nicht mehr."""
    s, _ = _mit_kinderzahl(2)
    assert len(s["events"]) == 2, (
        f"{len(s['events'])} Events statt 2: {[e['feld_id'] for e in s['events']]}")


def test_die_instanz_achse_sieht_die_kinder_weiterhin():
    """Naht zur Instanz-Achse: die Ableitung darf die Zahl nicht verstellen, aus der die
    Eingabefelder je Kind entstehen."""
    s, b = _mit_kinderzahl(3)
    n, etikett = TR.instanz_anzahl(s, b, "kind_vorname")
    assert (n, etikett) == (3, "Kind")


# ---------------------------------------------------------------- die Naht zum KI-Weg

def test_das_zaehlfeld_erreicht_die_ki_auch_ueber_das_falsche_thema():
    """WARUM DIE ABLEITUNG OBEN IM ECHTEN LAUF TROTZDEM NICHT GRIFF (Julius, 2026-08-25).

    Der KI-Weg verengt in zwei Stufen: erst wählt das Modell THEMEN (Regeln), dann sieht es nur
    die Felder dieser Themen. `fam_anzahl_kinder` gehört aber zu `p24b_entlastungsbetrag` — der
    Regel für ALLEINERZIEHENDE. Julius schrieb „verheiratet, 2 kinder"; das Modell nahm
    `p32_6_kinderfreibetraege`, dessen erste Frage nach dem Vornamen fragt — und genau die
    Vornamensfrage bekam er. Die Zahl 2 hatte kein Feld, in das sie gehen konnte.

    Folge war doppelt sichtbar: EIN Vornamensfeld statt zwei, und die Existenzfrage nach Kindern
    stand danach immer noch in der Ankreuzliste — beides, weil dasselbe Feld fehlte.

    Geprüft wird die Katalog-Erweiterung, nicht das Modell: KEIN LLM-Aufruf."""
    import api_llm as L  # noqa: E402 — nur hier gebraucht

    b = TR.lade_bindung()
    voll = [{"feld_id": f, "regel_id": (e.get("quelle") or {}).get("regel_id"),
             "instanz_gruppe": e.get("instanz_gruppe")} for f, e in b.items()]

    kat3 = [f for f in voll if f["regel_id"] == "p32_6_kinderfreibetraege"]
    assert kat3, "Die Regel p32_6_kinderfreibetraege gibt es nicht mehr — Test anpassen."
    assert not any(f["feld_id"] == "fam_anzahl_kinder" for f in kat3), (
        "Das Zählfeld liegt inzwischen in derselben Regel wie die Kind-Felder — dann ist dieser "
        "Test gegenstandslos (und die Erweiterung vermutlich auch).")

    erweitert = L._mit_zaehlfeldern(kat3, voll)
    assert any(f["feld_id"] == "fam_anzahl_kinder" for f in erweitert), (
        "Die KI bekommt die Kinderzahl nicht angeboten, wenn sie das Thema Kinderfreibeträge "
        "wählt — dann bleibt es bei einem Eingabefeld je Kind-Angabe.")


def test_ein_katalog_ohne_instanzfelder_waechst_nicht():
    """Die Erweiterung darf nicht jedem Thema fremde Felder anhängen — sonst verwässert sie genau
    die Verengung, für die Stufe 2 gebaut wurde."""
    import api_llm as L  # noqa: E402

    b = TR.lade_bindung()
    voll = [{"feld_id": f, "regel_id": (e.get("quelle") or {}).get("regel_id"),
             "instanz_gruppe": e.get("instanz_gruppe")} for f, e in b.items()]
    ohne = [f for f in voll if f["regel_id"] == "p9_1_3_nr5_doppelte_haushaltsfuehrung"]
    assert ohne and not any(f["instanz_gruppe"] for f in ohne)
    assert len(L._mit_zaehlfeldern(ohne, voll)) == len(ohne)
