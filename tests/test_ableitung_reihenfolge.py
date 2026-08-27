"""Eine Ableitung darf nicht davon abhaengen, in welcher Reihenfolge der Nutzer tippt. NULL LLM.

GEMESSEN am 2026-08-27, ein Kind von fuenf Jahren mit 6.000 EUR Betreuungskosten:

    Haushaltszeitraum zuerst, dann Geburtsdatum  ->  4.800 EUR Abzug
    Geburtsdatum zuerst, dann Haushaltszeitraum  ->      0 EUR Abzug

Dieselben Tatsachen, dieselbe Erklaerung, 4.800 EUR Unterschied — je nach Einkommen 1.460 bis
1.965 EUR zu viel Steuer. Ursache: `store._rechne_ab` lief nur beim Schreiben des QUELLFELDS und
prueste das `und_feld` in genau diesem Moment; wurde es spaeter bestaetigt, loeste nichts die
Ableitung erneut aus.

Der zweite Teil desselben Befunds sitzt im Fragetext. Das Feld traegt zwei Qualifikationswege
(§ 10 Abs. 1 Nr. 5 S. 1: unter 14 und im Haushalt ODER Behinderung vor 25), die Frage fragte nur
nach dem zweiten. Wer ein fuenfjaehriges Kind hat und wahrheitsgemaess „nein" antwortete, bekam
`false` gespeichert — und der Rechenkern laesst bei allem ausser `True` das Kind aus der Summe
fallen.

Die schaerfste Zusage steht hier als eigener Test: „spaeter auch feuern" darf NICHT heissen
„eine Antwort des Nutzers ueberschreiben".
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/bescheid", "produkt/traverser", "produkt/store", "produkt/mapping",
             "produkt/engine", "produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import store as ST             # noqa: E402
import traverser as TR         # noqa: E402
import bescheid_abzuege as BA  # noqa: E402

BINDUNG = TR.lade_bindung()
KATALOG = ST.lade_katalog(BINDUNG)
VZ = 2025
FELD = "kind_unter_14_haushaltszugehoerig"
GEB = ("kind_geburtsdatum", "01.03.2020")            # im VZ 2025 fuenf Jahre alt
HAUS = ("kind_betreuung_haushaltszugehoerigkeit_zeitraum", "01.01-31.12")
KOSTEN = ("kinderbetreuungskosten", 600_000)         # 6.000 EUR


def _setze(s, feld, wert, zustand="bestaetigt"):
    if zustand == "bestaetigt":
        ST.append_event(s, feld_id=feld, wert=wert, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"klick@{feld}"},
                        ts="2026-08-27T12:00:00+00:00", bindung=BINDUNG, katalog=KATALOG)
    else:
        # Ein KI-Vorschlag ist der realistische vorlaeufige Wert. Auflage A erzwingt dabei
        # herkunft=llm_vorschlag, zustand=vorlaeufig und signal_2=None — die KI bestaetigt nie.
        ST.append_event(s, feld_id=feld, wert=wert, zustand="vorlaeufig",
                        herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="llm:test",
                        signal={"signal_1": None, "signal_2": None},
                        ts="2026-08-27T12:00:00+00:00", bindung=BINDUNG, katalog=KATALOG)


def _store(*paare):
    s = ST.leerer_store(VZ, fall_id="ableitung-reihenfolge")
    for feld, wert, *rest in paare:
        _setze(s, feld, wert, rest[0] if rest else "bestaetigt")
    return s


def _wert(s):
    felder, _ = ST.materialisiere(s)
    return felder.get(FELD)


def _abzug(s):
    """Der Abzug nach § 10 Abs. 1 Nr. 5 ueber den echten Rechenweg (kein Nachbau)."""
    return BA._kinderbetreuung_summe(s, BINDUNG, nur_bestaetigt=True, vz=VZ)


# ---- Die Reihenfolge darf nichts entscheiden ---------------------------------

def test_ableitung_feuert_auch_wenn_das_und_feld_zuletzt_kommt():
    """Der gemessene Fall. Vor dem Fix blieb das Feld hier leer."""
    e = _wert(_store(GEB, HAUS))
    assert e is not None, "Ableitung ist ausgefallen, weil das und_feld nach der Quelle kam"
    assert e["wert"] is True


def test_beide_reihenfolgen_ergeben_dasselbe():
    a = _wert(_store(HAUS, GEB))
    b = _wert(_store(GEB, HAUS))
    assert a["wert"] == b["wert"] is True


def test_der_abzug_haengt_nicht_mehr_an_der_reihenfolge():
    """Der Geldtest: derselbe Fall, beide Reihenfolgen, derselbe Betrag.
    Gemessen vor dem Fix: 4.800 EUR gegen 0 EUR."""
    a = _abzug(_store(HAUS, GEB, KOSTEN))
    b = _abzug(_store(GEB, KOSTEN, HAUS))
    assert a == b == 4800, f"Reihenfolge entscheidet ueber den Abzug: {a} EUR gegen {b} EUR"


# ---- Die Schranke: die Antwort des Nutzers gewinnt ---------------------------

def test_nutzerantwort_wird_von_der_spaeteren_ableitung_nicht_ueberschrieben():
    """Die wichtigste Zusage des Fixes. Der Nutzer hat NEIN gesagt; danach kommen Geburtsdatum
    und Haushaltszeitraum, aus denen die Ableitung TRUE rechnen wuerde. Sie darf nicht.
    Spaeter zu feuern heisst nicht, den Nutzer zu korrigieren (Auflage B, nie ueberschreiben)."""
    e = _wert(_store((FELD, False), GEB, HAUS))
    assert e["wert"] is False
    assert (e.get("herkunft") or {}).get("herkunft") == "laie", \
        "der Wert stammt nicht mehr vom Nutzer — die Ableitung hat ihn ueberschrieben"


def test_nutzerantwort_gewinnt_auch_in_die_andere_richtung():
    """Symmetrisch, damit der Test oben nicht bloss „False bleibt False" misst."""
    e = _wert(_store((FELD, True), GEB, HAUS))
    assert e["wert"] is True
    assert (e.get("herkunft") or {}).get("herkunft") == "laie"


def test_ein_nein_des_nutzers_kostet_weiterhin_den_abzug():
    """Kein verstecktes Geschenk: wer wahrheitsgemaess verneint, bekommt keinen Abzug.
    Ohne diesen Test koennte der Fix zu „immer True" entgleisen und still Steuern senken."""
    assert _abzug(_store((FELD, False), GEB, KOSTEN, HAUS)) == 0


# ---- Die vier fail-closed-Schranken bleiben stehen ---------------------------

def test_vorlaeufige_quelle_leitet_nichts_ab():
    """Nur bestaetigte Quellen. Ein KI-Vorschlag fuer das Geburtsdatum darf keinen Abzug
    ausloesen — sonst rechnete ein Vorschlag eine Voraussetzung herbei."""
    s = _store((GEB[0], GEB[1], "vorlaeufig"), HAUS)
    assert _wert(s) is None


def test_vorlaeufiges_und_feld_leitet_nichts_ab():
    """Dieselbe Schranke fuer das und_feld — und die REIHENFOLGE hier ist der ganze Test.

    Erst das vorlaeufige und_feld, DANN die bestaetigte Quelle: nur so laeuft `_rechne_ab`
    ueberhaupt bis zur und_feld-Pruefung. Andersherum (Quelle zuerst, und_feld vorlaeufig
    hinterher) faengt schon die aeussere Schranke `zustand != "bestaetigt"` den Aufruf ab, und
    der Test waere gruen, ohne die Zusage zu messen — die Mutationsprobe am 2026-08-27 hat genau
    das aufgedeckt: Schranke entfernt, Test blieb gruen."""
    s = _store((HAUS[0], HAUS[1], "vorlaeufig"), GEB)
    assert _wert(s) is None, "ein vorlaeufiger Haushaltszeitraum hat einen Abzug ausgeloest"


def test_ohne_und_feld_bleibt_die_frage_offen():
    """Ohne Haushaltszeitraum ist die Haushaltszugehoerigkeit nicht belegt. Sie zu unterstellen
    gaebe einen Abzug, den niemand erklaert hat — also wird gefragt."""
    s = _store(GEB, KOSTEN)
    assert _wert(s) is None
    assert FELD in TR.naechste_fragen(s, BINDUNG)
    assert _abzug(s) == 0


def test_kind_ueber_14_wird_nicht_abgeleitet():
    """Oberhalb der Schwelle rechnet die Ableitung NICHTS — dann kann die Behinderungs-Ausnahme
    greifen, und ein abgeleitetes Nein naehme dem Nutzer den Abzug ungefragt."""
    s = _store(("kind_geburtsdatum", "01.03.2009"), HAUS)
    assert _wert(s) is None
    assert FELD in TR.naechste_fragen(s, BINDUNG)


def test_die_spur_nennt_das_quellfeld_nicht_den_ausloeser():
    """Seit es zwei Ausloeser gibt, fallen „woher kommt der Wert" und „was hat ihn ausgeloest"
    auseinander. `warum` soll die Herkunft des WERTES zeigen."""
    s = _store(GEB, HAUS)
    abgeleitet = [e for e in s["events"] if e["schreiber"] == "abgeleitet:ableitung"]
    assert len(abgeleitet) == 1
    assert abgeleitet[0]["signal"]["signal_2"] == "ableitung@kind_geburtsdatum"


def test_keine_ableitung_speist_eine_andere():
    """Die Zusage „keine Kette", strukturell: kein Quell- oder und_feld einer Ableitung darf
    selbst ein abgeleitetes Feld sein. Sonst haenge das Ergebnis wieder an der Reihenfolge —
    diesmal an der Reihenfolge, in der diese Schleife die Bindung durchlaeuft."""
    abgeleitete = {fid for fid, b in BINDUNG.items() if b.get("ableitung")}
    ketten = []
    for fid in abgeleitete:
        regel = BINDUNG[fid]["ableitung"]
        for rolle in ("aus", "und_feld"):
            quelle = regel.get(rolle)
            if quelle in abgeleitete:
                ketten.append(f"{fid}.{rolle} -> {quelle}")
    assert not ketten, f"Ableitung speist Ableitung: {ketten}"


# ---- Der Fragetext deckt beide Qualifikationswege ab -------------------------

def test_fragetext_nennt_beide_wege():
    """§ 10 Abs. 1 Nr. 5 S. 1 kennt zwei Wege ins Recht: Alter unter 14 ODER Behinderung. Fragt
    der Text nur nach der Behinderung, antwortet der Elternteil eines fuenfjaehrigen Kindes
    wahrheitsgemaess „nein" — und verliert 4.800 EUR Abzug."""
    text = BINDUNG[FELD]["fragetext_laie"]
    assert "14" in text, f"der Altersweg fehlt im Fragetext: {text}"
    assert "Behinderung" in text, f"der Behinderungsweg fehlt im Fragetext: {text}"
    assert "Haushalt" in text, f"die gemeinsame Voraussetzung fehlt im Fragetext: {text}"


def test_hilfe_verspricht_nicht_mehr_dass_die_frage_nur_ab_14_kommt():
    """Nachweislich falsch gewesen: die Frage kommt auch fuer ein fuenfjaehriges Kind, naemlich
    immer dann, wenn Geburtsdatum oder Haushaltszeitraum fehlen."""
    hilfe = BINDUNG[FELD]["hilfe_kurz"]
    assert "kommt nur" not in hilfe, f"die widerlegte Zusage steht wieder da: {hilfe}"


def test_ein_ja_auf_die_neue_frage_rettet_den_abzug():
    """Der Fall, den der neue Fragetext ueberhaupt erst beantwortbar macht: Haushaltszeitraum
    nie angegeben, Kind fuenf Jahre alt. Auf die alte Frage („wegen einer Behinderung?") war die
    wahre Antwort „nein" und der Abzug weg; auf die neue ist sie „ja"."""
    assert _abzug(_store(GEB, KOSTEN, (FELD, True))) == 4800
