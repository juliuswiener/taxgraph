"""Eine übersprungene Instanz geht nicht mehr lautlos verloren — sie wird GEMELDET.

GEMESSEN am 2026-08-27, drei Kinder angegeben, zwei Namen eingetragen:

    vor      : kind_vorname offen? True  | __2 einzeln? False | instanz_anzahl (3, 'Kind')
    nach 1+2 : base offen? False | __2? False | __3? False    | instanz_anzahl (3, 'Kind')

Ist die erste Instanz beantwortet, fällt das Basisfeld GANZ aus `naechste_fragen`. Der Traverser
führt nur das Basisfeld und legt die Zahl als `instanz_anzahl` daneben; `kind_vorname__3` steht
dort nie als eigene Frage — die Instanzfelder stehen nicht einmal in der Bindung. Zurück führt
auch kein Weg: die Korrektur sucht das Feld in `/fragen`, und `__n` steht dort nicht.

Zwei Stellen versprachen wörtlich das Gegenteil („die dritte Frage bleibt offen und kommt im
Fragebogen wieder") — in `app.js` und im Docstring von test_ui_instanzen.py. Kein einziger Assert
prüfte die Zusage, deshalb konnte sie jahrelang danebenstehen.

WARUM GEMELDET UND NICHT DIE FRAGE OFFENGEHALTEN: das Zählfeld ist nach dem Beantworten selbst
nicht mehr im Fragebogen (gemessen: `'fam_anzahl_kinder' in naechste_fragen(...)` ist False). Der
Nutzer könnte „es sind doch nur zwei" also gar nicht mehr sagen, die Frage würde nie schliessen —
und seit `_themen_folge` ein angefangenes Thema vorn hält, bliebe der Fragebogen dauerhaft darauf
stehen. Eine Sackgasse ist schlimmer als eine Lücke, die man sieht.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "produkt/konsistenz", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API        # noqa: E402
import api_auth          # noqa: E402
import audit             # noqa: E402
import preflight as PF   # noqa: E402
import store as ST       # noqa: E402
import traverser as TR   # noqa: E402

LAIE = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
KLICK = {"signal_1": None, "signal_2": "klick"}
BINDUNG = TR.lade_bindung()


def _fall(kinder=3, namen=("Anna", "Ben"), zustand="bestaetigt"):
    """Ein Fall mit `kinder` angekündigten und `namen` eingetragenen Kindern."""
    s = ST.leerer_store(veranlagungszeitraum=2025, fall_id="instanz-luecke")
    for fid, e in BINDUNG.items():
        if e.get("screening"):
            ST.append_event(s, feld_id=fid,
                            wert=(False if fid == "kein_kind"
                                  else fid.startswith(("kein_", "keine_"))),
                            zustand="bestaetigt", herkunft=LAIE, schreiber="ui:laie",
                            signal=KLICK, bindung=BINDUNG)
    ST.append_event(s, feld_id="fam_anzahl_kinder", wert=kinder, zustand="bestaetigt",
                    herkunft=LAIE, schreiber="ui:laie", signal=KLICK, bindung=BINDUNG)
    for i, name in enumerate(namen, start=1):
        ST.append_event(s, feld_id=TR.instanz_feld_id("kind_vorname", i), wert=name,
                        zustand=zustand, herkunft=LAIE, schreiber="ui:laie", signal=KLICK,
                        bindung=BINDUNG)
    return s


# ------------------------------------------------------- die Tatsache, nicht die Prosa

def test_die_uebersprungene_instanz_kommt_wirklich_nicht_wieder():
    """PINNT DEN IST-ZUSTAND, damit die falsche Zusage nicht unbemerkt zurückkehrt.

    Ein Test über einen Kommentar wäre spröde und würde nichts beweisen. Dieser hier misst, was
    wirklich passiert — und wird rot, sobald jemand das Basisfeld doch offenhält. Dann ist nicht
    dieser Test falsch, sondern die Entscheidung von 2026-08-27 neu zu treffen: das Zählfeld muss
    dann ebenfalls korrigierbar sein, sonst entsteht die Sackgasse."""
    s = _fall()
    offen = TR.naechste_fragen(s, BINDUNG)
    assert "kind_vorname" not in offen, (
        "Das Basisfeld steht wieder im Fragebogen — dann kommt die übersprungene Instanz doch "
        "wieder, und die Meldung in preflight ist überflüssig geworden. Bitte auch prüfen, ob das "
        "Zählfeld korrigierbar ist; ohne das ist die Frage eine Sackgasse.")
    assert "kind_vorname__3" not in offen, (
        "Ein Instanzfeld steht einzeln in der Queue — der Traverser führt nur Basisfelder.")
    assert TR.instanz_anzahl(s, BINDUNG, "kind_vorname") == (3, "Kind"), (
        "Vorbedingung: die Oberfläche muss drei Eingabefelder bauen, sonst misst der Test nichts.")


def test_die_luecke_wird_erkannt():
    """Der gemessene Fall: 3 angekündigt, 2 eingetragen, Nummer 3 fehlt."""
    felder, _ = ST.materialisiere(_fall())
    luecken = TR.fehlende_instanzen(felder, BINDUNG)
    assert len(luecken) == 1, f"Erwartet genau eine Lücke, gemeldet: {luecken}"
    assert luecken[0]["feld_id"] == "kind_vorname"
    assert luecken[0]["vorhanden"] == [1, 2] and luecken[0]["fehlend"] == [3]
    assert luecken[0]["anzahl"] == 3 and luecken[0]["etikett"] == "Kind"


def test_ein_unangetastetes_feld_ist_keine_luecke():
    """DIE VERENGUNG, ohne die die Meldung im Rauschen unterginge.

    Bei drei Kindern trägt fast jedes kinderbezogene Feld eine Instanz-Achse — 69 Felder haben
    eine. Würde jedes unbeantwortete davon dreifach gemeldet, stünden Dutzende Meldungen neben der
    einen, die zählt. Gemeldet wird nur die Lücke IN einer begonnenen Reihe: wer nach dem zweiten
    Namen aufhört, hat etwas vergessen; wer nie eine Steuer-ID eingetragen hat, hat die Frage
    schlicht nicht beantwortet."""
    felder, _ = ST.materialisiere(_fall())
    gemeldet = {luecke["feld_id"] for luecke in TR.fehlende_instanzen(felder, BINDUNG)}
    mit_achse = {f for f, e in BINDUNG.items() if e.get("instanz_gruppe")}
    assert len(mit_achse) >= 50, f"Nur {len(mit_achse)} Felder mit Achse — Messung prüfen."
    assert gemeldet == {"kind_vorname"}, (
        f"Gemeldet werden {len(gemeldet)} Felder, obwohl nur eines eine angefangene Reihe hat: "
        f"{sorted(gemeldet)}")


def test_ein_vorlaeufiger_wert_fuellt_keine_instanz():
    """Nur BESTÄTIGT zählt, wie überall im Traverser. Ein vorläufiger Vorschlag geht in keine
    Summe und in keine Erklärung ein — für die Abgabe ist er so gut wie nicht da, und eine
    Meldung, die ihn mitzählt, spräche den Nutzer von einer Lücke frei, die er noch hat."""
    felder, _ = ST.materialisiere(_fall(kinder=3, namen=("Anna", "Ben"), zustand="vorlaeufig"))
    luecken = TR.fehlende_instanzen(felder, BINDUNG)
    assert luecken == [], (
        f"Ohne eine einzige bestätigte Instanz gibt es keine angefangene Reihe: {luecken}")

    gemischt = _fall(kinder=3, namen=("Anna",))
    ST.append_event(gemischt, feld_id="kind_vorname__2", wert="Ben", zustand="vorlaeufig",
                    herkunft=LAIE, schreiber="ki:vorschlag", signal=KLICK, bindung=BINDUNG)
    felder, _ = ST.materialisiere(gemischt)
    luecke = TR.fehlende_instanzen(felder, BINDUNG)
    assert luecke and luecke[0]["fehlend"] == [2, 3], (
        f"Der vorläufige zweite Name wurde als ausgefüllt gezählt: {luecke}")


def test_ohne_angekuendigte_mehrzahl_schweigt_die_pruefung():
    """Ein Kind, ein Name — es gibt nichts zu melden. Und ohne Zählfeld-Antwort erst recht nicht:
    dann ist die Zahl 1, genau wie `instanz_anzahl` sie liest."""
    felder, _ = ST.materialisiere(_fall(kinder=1, namen=("Anna",)))
    assert TR.fehlende_instanzen(felder, BINDUNG) == []

    ohne_zahl = ST.leerer_store(veranlagungszeitraum=2025, fall_id="ohne-zahl")
    ST.append_event(ohne_zahl, feld_id="kind_vorname", wert="Anna", zustand="bestaetigt",
                    herkunft=LAIE, schreiber="ui:laie", signal=KLICK, bindung=BINDUNG)
    felder, _ = ST.materialisiere(ohne_zahl)
    assert TR.fehlende_instanzen(felder, BINDUNG) == []


def test_die_zahl_der_instanzen_wird_nur_an_einer_stelle_entschieden():
    """`instanz_anzahl` (Store) und `fehlende_instanzen` (Snapshot) MÜSSEN dieselbe Zahl lesen.

    Zwei Repräsentationen mit je eigener Regel sind die Fehlerbauart, an der im Repo schon
    mehrfach etwas auseinandergelaufen ist. Sie teilen sich deshalb `_anzahl_aus_eintrag` — und
    dieser Test hält fest, dass die Naht hält, nicht bloss dass beide heute zufällig gleich sind."""
    for kinder in (1, 2, 3, 99):
        s = _fall(kinder=kinder, namen=("Anna",))
        felder, _ = ST.materialisiere(s)
        aus_store, _etikett = TR.instanz_anzahl(s, BINDUNG, "kind_vorname")
        gruppe = TR.lade_instanz_gruppen()["kind"]
        aus_snapshot = TR._anzahl_aus_eintrag(felder.get(gruppe["anzahl_feld"]), gruppe)
        assert aus_store == aus_snapshot, (
            f"{kinder} Kinder: Store liest {aus_store}, Snapshot liest {aus_snapshot} — die "
            f"beiden Leserichtungen sind auseinandergelaufen.")
        assert aus_store <= int(gruppe["max"]), "Die Obergrenze der Gruppe wurde nicht angewandt."


# ------------------------------------------------------- der Weg bis zum Nutzer

def test_die_meldung_erreicht_den_endpunkt(tmp_path, monkeypatch):
    """Vom Ende her: durch den ECHTEN Endpunkt, nicht nur durch die Prüffunktion.

    Absichtlich im vorhandenen Schlüssel `widersprueche_plausibilitaet` — `api.preflight_check`
    führt eine fest verdrahtete Liste, und ein neuer Schlüssel ohne Eintrag dort wäre totes
    Wiring. Genau das ist an dieser Datei am 2026-08-27 schon einmal passiert."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", "pruefer")
    fid = "instanz-luecke-http"
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    for feld, wert in (("kein_kind", False), ("fam_anzahl_kinder", 3),
                       ("kind_vorname", "Anna"), ("kind_vorname__2", "Ben")):
        API.event(fid, {"feld_id": feld, "wert": wert, "zustand": "bestaetigt",
                        "herkunft": LAIE, "schreiber": "ui:laie",
                        "signal": {"signal_1": None, "signal_2": f"klick@{feld}"}})

    st, body = API.preflight_check(fid)
    assert st == 200
    treffer = [i for i in body["items"]
               if i.get("bereich") == "plausibilitaet" and "Kind 3" in i.get("text", "")]
    assert treffer, (
        f"Drei Kinder angekündigt, zwei Namen eingetragen — und in der Antwort steht kein Wort "
        f"davon: status={body['status']!r}, items={body['items']}")


def test_der_meldetext_nennt_beide_zahlen_und_keine_kennung():
    """Die Form, die dieses Modul für alle Meldungen gesetzt hat: beide Zahlen nennen, keine
    Feldkennung, sagen was zu tun ist. Geprüft wird die KENNUNG, nicht das Wort — „Kind" gehört
    in den Satz, `kind_vorname` nicht."""
    felder, _ = ST.materialisiere(_fall())
    meldungen = PF.unvollstaendige_instanzen(felder)
    assert len(meldungen) == 1, f"Erwartet eine Meldung: {meldungen}"
    text = meldungen[0]["grund"]
    assert "kind_vorname" not in text and "fam_anzahl" not in text, (
        f"Der Text nennt eine Feldkennung, die der Nutzer nie gesehen hat: {text!r}")
    assert "3" in text and "2" in text, f"Der Text nennt nicht beide Zahlen: {text!r}"
    assert "Kind 3" in text, f"Der Text sagt nicht, WELCHE Angabe fehlt: {text!r}"
    assert text.rstrip().endswith("."), "Meldetexte sind ganze Sätze."


def test_die_meldung_haengt_im_plausibilitaets_schluessel():
    """Der Schlüssel selbst, namentlich — damit ein späterer Umbau auf einen eigenen Schlüssel
    nicht unbemerkt an `api.preflight_check` vorbeiläuft (das Erreichbarkeits-Gate in
    tests/test_preflight_erreichbarkeit.py würde ihn fangen, dieser Test sagt zusätzlich, WARUM
    er hier liegt)."""
    felder, _ = ST.materialisiere(_fall())
    ergebnis = PF.preflight(felder)
    texte = [w["grund"] for w in ergebnis["widersprueche_plausibilitaet"]]
    assert any("Kind 3" in t for t in texte), (
        f"Die Meldung steht nicht unter `widersprueche_plausibilitaet`: {sorted(ergebnis)}")
    assert ergebnis["status"] == "RED", (
        f"Eine fehlende Angabe lässt den Status auf {ergebnis['status']!r} — sie ist ein "
        f"Widerspruch zwischen zwei Angaben und gehört auf RED.")


def test_ein_sauberer_fall_bleibt_still():
    """Kein Rauschen: drei angekündigt, drei eingetragen — nichts zu melden."""
    felder, _ = ST.materialisiere(_fall(kinder=3, namen=("Anna", "Ben", "Cem")))
    assert TR.fehlende_instanzen(felder, BINDUNG) == []
    assert PF.unvollstaendige_instanzen(felder) == []
