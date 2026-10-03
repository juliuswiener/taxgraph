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


# ---- Das zweite Kind: die Ableitung feuert je Instanz --------------------------
# Gemessen 2026-10-03 auf 431ca41c, derselbe Fall mit dem echten Rechenweg:
#   Kind 1 allein                                -> 4.800 EUR, Ableitung auf `FELD`
#   Kind 1 + Kind 2, je 6.000 EUR                -> 4.800 EUR (soll 9.600), Ableitung NUR auf `FELD`
#   Kind 2 allein (Kind 1 fehlt, `FELD` NICHT aktiv) ->    0 EUR (soll 4.800), keine Ableitung
#   Kind 1 + 2, Nutzer setzt beide Gates selbst  -> 9.600 EUR
# Der dritte Fall trennt den Suffix von der Zyklus-Sperre `ziel in aktiv`: dort gilt die Sperre
# nicht, und es feuert trotzdem nichts. `_rechne_ab` verglich `feld_id` ohne `__2` mit `aus`/`und_feld`.

FELD2 = FELD + "__2"
GEB2 = ("kind_geburtsdatum__2", "01.03.2021")                        # im VZ 2025 vier Jahre alt
HAUS2 = ("kind_betreuung_haushaltszugehoerigkeit_zeitraum__2", "01.01-31.12")
KOSTEN2 = ("kinderbetreuungskosten__2", 600_000)


def _abgeleitet(s):
    return {e["feld_id"]: e for e in s["events"] if e["schreiber"] == "abgeleitet:ableitung"}


def _wert2(s):
    felder, _ = ST.materialisiere(s)
    return felder.get(FELD2)


def test_zweites_kind_leitet_seine_qualifikation_ab():
    """Kind 2 allein: `FELD` ist nicht aktiv, die Zyklus-Sperre kann nicht schuld sein. Vor dem
    Fix blieb `FELD__2` leer — allein wegen des Suffix."""
    s = _store(GEB2, HAUS2)
    e = _wert2(s)
    assert e is not None, "die Ableitung erreicht Instanz 2 nicht (Vergleich ohne Suffix)"
    assert e["wert"] is True
    assert _wert(s) is None, "Instanz 2 hat ein Feld der Instanz 1 geschrieben"
    # Die Spur nennt das Quellfeld der Instanz, aus dem gerechnet wurde.
    assert _abgeleitet(s)[FELD2]["signal"]["signal_2"] == "ableitung@kind_geburtsdatum__2"


def test_zweites_kind_beide_reihenfolgen_ergeben_dasselbe():
    """Quelle zuerst und und_feld zuerst: beide Auslöser müssen die Instanz treffen."""
    assert _wert2(_store(GEB2, HAUS2))["wert"] is True
    assert _wert2(_store(HAUS2, GEB2))["wert"] is True


def test_beide_kinder_leiten_je_instanz_ab():
    s = _store(GEB, HAUS, GEB2, HAUS2)
    assert {k: e["wert"] for k, e in _abgeleitet(s).items()} == {FELD: True, FELD2: True}


def test_zwei_kinder_je_6000_ergeben_9600_abzug():
    """Der Geldtest, über den echten Rechenweg. Vor dem Fix 4.800 EUR."""
    a = _abzug(_store(GEB, KOSTEN, HAUS, GEB2, KOSTEN2, HAUS2))
    b = _abzug(_store(HAUS2, GEB2, KOSTEN2, HAUS, GEB, KOSTEN))
    assert a == b == 9600, f"{a} EUR / {b} EUR statt 9.600 EUR"


def test_kind_2_allein_bekommt_seinen_abzug():
    assert _abzug(_store(GEB2, KOSTEN2, HAUS2)) == 4800


def test_nutzerantwort_auf_instanz_2_bleibt_stehen():
    """Die Sperre gilt je Instanz: ein Nein auf `FELD__2` wird nicht überschrieben."""
    e = _wert2(_store((FELD2, False), GEB2, HAUS2))
    assert e["wert"] is False
    assert (e.get("herkunft") or {}).get("herkunft") == "laie"


def test_antwort_auf_instanz_1_sperrt_instanz_2_nicht():
    """Die Sperre gilt je Instanz, nicht je Basisfeld. Hätte der Fix nur den Suffix im Vergleich
    abgeschnitten, läge `FELD` in `aktiv` und blockte Kind 2."""
    s = _store((FELD, False), GEB2, HAUS2)
    assert _wert(s)["wert"] is False, "die Antwort auf Instanz 1 wurde angefasst"
    assert _wert2(s)["wert"] is True


def test_ein_nein_auf_instanz_2_kostet_weiterhin_den_abzug():
    """Kein verstecktes Geschenk, wie bei Instanz 1."""
    assert _abzug(_store((FELD2, False), GEB2, KOSTEN2, HAUS2)) == 0


def test_instanz_2_ohne_und_feld_oder_ueber_14_leitet_nichts_ab():
    assert _wert2(_store(GEB2, KOSTEN2)) is None
    assert _wert2(_store(("kind_geburtsdatum__2", "01.03.2009"), HAUS2)) is None
    # Quelle der Instanz 2 und und_feld der Instanz 1 sind keine Einheit.
    assert _wert2(_store(GEB2, HAUS)) is None
    assert _wert2(_store(GEB, HAUS2)) is None


def test_feld_id_mit_zeilenumbruch_ist_keine_instanz_2():
    """Pythons `$` in `parse_instanz` passt vor einem abschliessenden `\\n`: `…__2\\n` ist ein
    Instanz-Feld des Typpruefers, aber kein `aus` der Instanz 2. Rust liest es genauso."""
    s = _store(("kind_geburtsdatum__2\n", "01.03.2021"), HAUS2)
    assert not _abgeleitet(s)


def test_ableitung_ohne_instanz_gruppe_ignoriert_den_suffix():
    """Der Suffix gilt nur für Felder mit `instanz_gruppe`. `geburtsjahr` hat keine: ein
    `stammdaten_geburtsdatum__2` darf kein `geburtsjahr__2` erzeugen."""
    s = _store(("stammdaten_geburtsdatum__2", "05.05.1955"))
    assert not [f for f in _abgeleitet(s) if f.endswith("__2")]


# ---- Die Frage-Reihenfolge: das Ziel nach seinem und_feld ------------------------
# Gemessen 2026-10-03 auf 431ca41c, Geburtsdatum bekannt (fuenf Jahre), `api.fragen`:
#   Ziel `kind_unter_14_haushaltszugehoerig` Platz 209, `…zeitraum` Platz 211 von 218.
# Wer der Reihenfolge folgt, beantwortet das Ziel zuerst — ein „Nein" kostet 4.800 EUR. Beantwortet
# er den Zeitraum zuerst, feuert die Ableitung und das Ziel verschwindet aus der Queue.
# Das Ziel ist ein Gate (`geltungsbedingung`, Gewicht > 0), der Zeitraum ein Slot: Gates stehen
# vor Slots, und `_nach_vordruck` sortiert nur innerhalb einer Klasse.

ZEITRAUM = HAUS[0]


def _pos(queue, feld):
    return queue.index(feld) if feld in queue else None


def test_ziel_wird_nach_seinem_und_feld_gefragt():
    """Geburtsdatum + Kosten bekannt, Zeitraum offen: der Zeitraum steht vor dem Ziel."""
    q = TR.naechste_fragen(_store(GEB, KOSTEN), BINDUNG)
    assert _pos(q, ZEITRAUM) is not None and _pos(q, FELD) is not None, "Fragen fehlen in der Queue"
    assert _pos(q, ZEITRAUM) < _pos(q, FELD), (
        f"das Ableitungsziel steht auf {_pos(q, FELD)}, sein und_feld auf {_pos(q, ZEITRAUM)}: "
        "wer der Reihenfolge folgt, beantwortet das Ziel, bevor die Ableitung feuern kann")


def test_leere_queue_hat_das_ziel_hinter_dem_und_feld():
    """Auch ohne jede Antwort (Platz 331 gegen 333 auf 431ca41c)."""
    q = TR.naechste_fragen(ST.leerer_store(VZ, fall_id="leer"), BINDUNG)
    assert _pos(q, ZEITRAUM) < _pos(q, FELD)


def test_wer_der_queue_folgt_wird_nie_nach_dem_ziel_gefragt():
    """Der Ablauf, nicht die Momentaufnahme: die Queue wird beantwortet, Kopf fuer Kopf. Sobald
    eines der beiden Felder am Kopf steht, ist es der Zeitraum — und danach ist das Ziel weg."""
    s = _store(GEB, KOSTEN)
    gefragt = []
    for _ in range(len(TR.naechste_fragen(s, BINDUNG))):
        q = TR.naechste_fragen(s, BINDUNG)
        kopf = next((f for f in q if f in (ZEITRAUM, FELD)), None)
        if kopf is None:
            break
        gefragt.append(kopf)
        if kopf == FELD:
            break
        _setze(s, ZEITRAUM, HAUS[1])          # der Nutzer folgt der Queue und beantwortet den Zeitraum
    assert gefragt == [ZEITRAUM], f"gefragt in dieser Reihenfolge: {gefragt}"
    assert FELD not in TR.naechste_fragen(s, BINDUNG)
    assert _wert(s)["wert"] is True
    assert _abzug(s) == 4800


def test_rueckfall_ueber_14_haelt_das_ziel_fragbar_und_hinter_dem_und_feld():
    """Die Behinderungs-Ausnahme: das Ziel bleibt eine Frage (Entscheidung hergeleitete-felder-
    bleiben-als-rueckfall-fragbar), nur die Reihenfolge aendert sich."""
    q = TR.naechste_fragen(_store(("kind_geburtsdatum", "01.03.2009"), KOSTEN), BINDUNG)
    assert FELD in q
    assert _pos(q, ZEITRAUM) < _pos(q, FELD)


def test_die_umordnung_verliert_und_verdoppelt_keine_frage():
    """Gegenprobe zur Queue: dieselbe Menge, nur anders geordnet."""
    q = TR.naechste_fragen(ST.leerer_store(VZ, fall_id="leer"), BINDUNG)
    assert len(q) == len(set(q))
    kandidaten = {f for f, b in BINDUNG.items() if b.get("askable")}
    assert set(q) <= kandidaten


def test_api_fragen_stellt_das_ziel_hinter_den_zeitraum(tmp_path, monkeypatch):
    """Der echte Weg (`api.fragen`), wie ihn die Oberflaeche liest: Geburtsdatum bekannt, Ziel
    und Zeitraum offen. Danach beantwortet der Nutzer den Zeitraum, und das Ziel ist weg."""
    import api as API
    import audit
    from _kegel import kegel_fuer
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))

    def laie(fld, w):
        return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
                "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}

    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": VZ, "fall_id": "k-reihenfolge"})
    assert st == 201, r
    for feld, wert in kegel_fuer("gesamt", {"bruttoarbeitslohn": 6000000}) + [KOSTEN, GEB]:
        st, r = API.event("k-reihenfolge", laie(feld, wert))
        assert st == 201, (feld, st, r)

    def queue():
        st, obj = API.fragen("k-reihenfolge")
        assert st == 200, obj
        return [f["feld_id"] for f in obj["fragen"]]

    q = queue()
    assert _pos(q, ZEITRAUM) is not None and _pos(q, FELD) is not None
    assert _pos(q, ZEITRAUM) < _pos(q, FELD), (
        f"api.fragen: Ziel Platz {_pos(q, FELD)}, Zeitraum Platz {_pos(q, ZEITRAUM)} von {len(q)}")
    st, r = API.event("k-reihenfolge", laie(*HAUS))
    assert st == 201, r
    assert FELD not in queue()


def _nach_ausloesern_probe():
    """Synthetische Bindung: zwei Ableitungen mit dem GLEICHEN und_feld und eine mit `aus` im
    selben Thema — die Nachzuegler behalten ihre Reihenfolge, nichts geht verloren."""
    thema = {"regel_id": "t"}
    b = {
        "z1": {"quelle": thema, "ableitung": {"aus": "q", "und_feld": "u"}},
        "z2": {"quelle": thema, "ableitung": {"aus": "q", "und_feld": "u"}},
        "v": {"quelle": thema},
        "q": {"quelle": thema},
        "u": {"quelle": thema},
        "w": {"quelle": thema},
    }
    return b


def test_nachzuegler_behalten_ihre_reihenfolge_und_gehen_nicht_verloren():
    b = _nach_ausloesern_probe()
    gruppe = ["z1", "z2", "v", "q", "u", "w"]
    assert TR._nach_ausloesern(gruppe, b) == ["v", "q", "u", "z1", "z2", "w"]
    # Steht schon alles richtig, aendert sich nichts.
    richtig = ["q", "u", "z1", "z2", "v", "w"]
    assert TR._nach_ausloesern(richtig, b) == richtig
    # Fehlt ein Ausloeser in der Gruppe (anderes Thema / schon beantwortet), bleibt das Ziel stehen.
    assert TR._nach_ausloesern(["z1", "v", "q"], b) == ["v", "q", "z1"]
    assert TR._nach_ausloesern(["z1", "v", "w"], b) == ["z1", "v", "w"]
    # Ein Ausloeser mit eigener ableitung zaehlt nicht (keine Kette): nichts geht verloren.
    ring = {"a": {"quelle": {"regel_id": "t"}, "ableitung": {"aus": "b"}},
            "b": {"quelle": {"regel_id": "t"}, "ableitung": {"aus": "a"}}}
    assert TR._nach_ausloesern(["a", "b"], ring) == ["a", "b"]


# ---- Die Monatsfrage: zwei Regeln, zwei Zeitbezuege, zwei Antworten --------------

def _monatsfall(s, feld, monate):
    """Ein bestaetigter Monatswert auf dem uebergebenen Feld."""
    _setze(s, feld, monate)


def test_die_monatsfrage_speist_nicht_mehr_beide_regeln():
    """Die Kopplung war eine Frage zu viel — und eine zu wenig.

    Vorher: `uebernachtung_monate_bisher` wurde per `ableitung: {art: uebernahme}` aus
    `vpf_monate_am_ort` gefuellt. EINE Antwort speiste damit zwei Regeln, die den Wert
    ENTGEGENGESETZT lesen: die Uebernachtung als Stand zu BEGINN ihres Zeitraums (die Restlaufzeit
    ist `48 - bisher`), die Verpflegung als kumulativen Stand (ihre Schwelle ist `> 3`, ein
    Zeitraum-Monatsfeld hat sie nicht).

    Gemessen mit Antwort 47 und monate=12: 12.400 EUR unter der Beginn-Lesart, 16.800 EUR unter der
    kumulativen — dieselbe Zahl, zwei Ergebnisse. Deshalb fragt jetzt jede Regel selbst.
    """
    s = ST.leerer_store(VZ, fall_id="monatsfrage-getrennt")
    _setze(s, "vpf_monate_am_ort", 47)
    felder, _ = ST.materialisiere(s)
    assert "uebernachtung_monate_bisher" not in felder, (
        "die Verpflegungsantwort leitet weiterhin die Uebernachtungs-Monatszahl ab — "
        "eine Antwort speist zwei einander widersprechende Lesarten")


def test_uebernachtung_monate_bisher_ist_eine_eigene_frage():
    """Nach der Trennung wird sie gestellt — und die Antwort gilt nur fuer ihre Regel."""
    s = ST.leerer_store(VZ, fall_id="monatsfrage-eigen")
    _setze(s, "vpf_monate_am_ort", 47)
    q = TR.naechste_fragen(s, BINDUNG, KATALOG)
    assert "uebernachtung_monate_bisher" in q, (
        "die Uebernachtungs-Monatszahl wird nicht gefragt — ohne die Ableitung bekommt sie "
        "sonst nie einen Wert")
    _setze(s, "uebernachtung_monate_bisher", 47)
    felder, _ = ST.materialisiere(s)
    assert felder["uebernachtung_monate_bisher"]["wert"] == 47
    assert felder["vpf_monate_am_ort"]["wert"] == 47, (
        "die Uebernachtungsantwort hat die Verpflegungszahl veraendert — die Felder sind nicht "
        "getrennt")
