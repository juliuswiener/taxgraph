"""Stille-Null-Klasse (Variante Typ-Konformität) — team-lead-Auftrag 2026-08-16.

Zwei Befunde, ein Muster: ein Wert, der nie richtig ankam, wurde trotzdem als `bestaetigt`
gespeichert — der Ring liest ihn dann still als 0 (kein Fehler, kein Absturz, falsche Steuer).

Befund A (Backend): `store.append_event` prüfte `wert` NIE gegen den Bindungstyp (typ=cent/int/
bool/enum/datum/text aus produkt/bindung/*.yaml) — für KEINEN Schreiber, auch nicht für den
menschlichen (ui:laie). Live reproduziert (Scratchpad, siehe Bericht): ein Gehalt als String
'50000' auf bruttoarbeitslohn (typ=cent) wurde mit zustand=bestaetigt akzeptiert; die _c/_cent/
_best-Lesehelfer in api.py (>15 Stellen) machen aus jedem Nicht-Zahl-Wert kommentarlos 0.

Fix: neue Auflage T in produkt/store/store.py (append_event bekommt ein optionales `bindung`-
Argument; wird es übergeben, muss `wert` zum Bindungstyp passen, sonst ValueError). Bewusst
OPTIONAL statt scharf für ALLE Aufrufer — die >2100 Bestandstests von ST.append_event(...) rufen
ohne bindung= und bleiben unberührt (sie testen andere Mechanismen mit absichtlich minimalem
Setup). Real verdrahtet an api.event() (HTTP, hier getestet), api.entfernung(), dem KI-Chat-
Vorschlags-Schreiber, beleg_writer und kontoauszug_writer (s. Bericht an team-lead für die
vollständige Naht-(a)-vs-(b)-Abwägung — insbesondere: import:elster/eDaten schreibt DIREKT
bestaetigt und hat noch KEINEN Produktions-Aufrufer, darum dort nur vorbereitet, nicht verdrahtet).

Befund B (Frontend, produkt/haut/static/app.js:leseWert) ist NICHT hier, sondern in
tests/test_ui_leerwert_stille_null.py (Playwright, wie test_ui_wahl_buttons.py).

NULL LLM.
"""
from __future__ import annotations

import glob
import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API     # noqa: E402
import audit          # noqa: E402
import store as ST    # noqa: E402

TS = "2026-08-16T12:00:00+00:00"


def _bindung() -> dict:
    b = {}
    for f in glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")):
        for e in (yaml.safe_load(open(f, encoding="utf-8")).get("bindungen") or []):
            b[e["feld_id"]] = e
    return b


BINDUNG = _bindung()


def _laie(fld, w, zustand="bestaetigt"):
    signal2 = f"ok@{fld}" if zustand == "bestaetigt" else None
    return {"feld_id": fld, "wert": w, "zustand": zustand,
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": None, "signal_2": signal2}}


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "sn-typ"})
    assert st == 201, r
    return "sn-typ"


# ---------------------------------------------------------------- HTTP-Ebene (api.event, Befund A)

def test_string_auf_cent_feld_wird_abgelehnt(fall):
    """Der genaue Live-Repro aus dem Auftrag: Gehalt als String statt Cent-Integer.
    api.event() fängt die ValueError des Stores und wirft ApiError(422, ...) — kein Tupel-Return
    (anders als z.B. einreichen()), darum pytest.raises statt st==422 (Muster aus
    test_fall_loeschen.py/test_kontoauszug_pdf_endpoint.py)."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("bruttoarbeitslohn", "50000"))
    assert exc.value.status == 422
    assert "Typ" in str(exc.value)


def test_bool_auf_cent_feld_wird_abgelehnt(fall):
    """Python-Falle: bool ist ein int-Subtyp (isinstance(True, int) is True). Ohne den expliziten
    Ausschluss ginge ein Häkchen als 1 Cent durch — genau der in produkt/store/store.py
    dokumentierte Grund für `not isinstance(wert, bool)`."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("bruttoarbeitslohn", True))
    assert exc.value.status == 422


def test_float_auf_cent_feld_wird_abgelehnt(fall):
    """cent ist ein Integer-Betrag — ein Bruchteil-Cent (z.B. aus falscher Euro→Cent-Umrechnung
    ohne round()) ist kein gültiger Speicherwert."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("bruttoarbeitslohn", 50000.5))
    assert exc.value.status == 422


def test_valider_cent_wert_wird_akzeptiert(fall):
    st, r = API.event(fall, _laie("bruttoarbeitslohn", 6000000))
    assert st == 201, r


def test_explizite_null_bleibt_gueltig(fall):
    """Der Kern der Team-Lead-Vorgabe: eine ECHTE, absichtlich eingegebene 0 darf NICHT durch die
    neue Typ-Prüfung verworfen werden — sie ist ein gültiger Cent-Betrag, keine Nicht-Antwort."""
    st, r = API.event(fall, _laie("bruttoarbeitslohn", 0))
    assert st == 201, r


def test_falscher_enum_wert_wird_abgelehnt(fall):
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("veranlagung", "gemeinsam"))   # gültig wären einzel/zusammen
    assert exc.value.status == 422


def test_gueltiger_enum_wert_wird_akzeptiert(fall):
    st, r = API.event(fall, _laie("veranlagung", "einzel"))
    assert st == 201, r


def test_string_auf_datum_feld_wird_abgelehnt(fall):
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("stammdaten_geburtsdatum", "kein Datum"))
    assert exc.value.status == 422


def test_iso_datum_wird_abgelehnt(fall):
    """Die Gegenrichtung, und sie ist der eigentliche Punkt: ISO ist hier FALSCH. Der erste
    Anlauf dieser Prüfung verlangte ISO und hätte damit jede echte Geburtsdatums-Eingabe mit
    422 abgewiesen — fail-closed gegen den eigenen dokumentierten Standard. Wer den Regex
    künftig „modernisiert", macht diesen Test rot und liest den Grund."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("stammdaten_geburtsdatum", "1985-04-12"))
    assert exc.value.status == 422


def test_valides_datum_wird_akzeptiert(fall):
    """TT.MM.JJJJ — amtliches ELSTER-Format (XSD DatumTTpMMpJJJJBekanntBaseCType_RABE zu
    E0100401), so auch der beispielwert der Bindung. Nichts in der Pipeline konvertiert."""
    st, r = API.event(fall, _laie("stammdaten_geburtsdatum", "12.04.1985"))
    assert st == 201, r


# ---------------------------------------------------------------- Store-Ebene (append_event, direkt)

def test_ohne_bindung_bleibt_rueckwaertskompatibel():
    """Die >2100 Bestandsaufrufe von ST.append_event(...) übergeben KEIN bindung= — sie dürfen
    durch die neue Auflage T nicht brechen. Ohne bindung greift die Prüfung nicht, exakt wie vor
    diesem Fix (das ist der Grund, warum die Suite ohne Massen-Retrofit grün bleiben kann)."""
    s = ST.leerer_store(2025, fall_id="sn-typ-kompat")
    ev = ST.append_event(s, feld_id="bruttoarbeitslohn", wert="50000", zustand="bestaetigt",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS)
    assert ev["wert"] == "50000"   # unverändert akzeptiert -- kein bindung= übergeben


def test_mit_bindung_greift_direkt_am_store():
    s = ST.leerer_store(2025, fall_id="sn-typ-direkt")
    with pytest.raises(ValueError, match="fail-closed \\(Typ\\)"):
        ST.append_event(s, feld_id="bruttoarbeitslohn", wert="50000", zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS,
                        bindung=BINDUNG)


def test_steuerzeichen_im_text_wird_abgelehnt():
    """Ticket elster-xml-steuerzeichen-im-textwert (Entscheidung Julius 2026-09-30): ein Zeichen
    ausserhalb der XML-1.0-Char-Produktion (hier NUL aus kopiertem Text) passt nicht zu typ=text.
    Gespeichert, landete es roh im ELSTER-XML, und ELSTER weist die ganze Abgabe ab."""
    s = ST.leerer_store(2025, fall_id="sn-typ-steuerzeichen")
    with pytest.raises(ValueError, match="fail-closed \\(Typ\\)"):
        ST.append_event(s, feld_id="stammdaten_nachname", wert="Maier\x00", zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS,
                        bindung=BINDUNG)


def test_steuerzeichen_im_text_ist_422_an_event(fall):
    """Dieselbe Abweisung am Schreib-Endpunkt: das bestehende 422-Format, kein neuer 500. Die
    Meldung nennt das Feld, aber weder den Wert (PII) noch das Zeichen selbst."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("stammdaten_nachname", "Maier\x00"))
    assert exc.value.status == 422
    meldung = str(exc.value)
    assert "stammdaten_nachname" in meldung and "Steuerzeichen" in meldung, meldung
    assert "Maier" not in meldung and "\x00" not in meldung, meldung


def test_abgewiesener_altwert_im_vorjahr_wird_uebersprungen(fall):
    """Ein Alt-Vorjahresfall (vor diesen Prüfungen gespeichert) trägt NUL in einem Textfeld und einen
    Zeitraum, der nicht aufs Muster passt. /vorjahr übernimmt die übrigen Felder, speichert den Fall
    und nennt die zwei übersprungenen feld_ids, nie ihren Wert (decisions/vorjahr-unpassenden-
    altwert-ueberspringen). Bis 2026-10-01 brach die ganze Übernahme mit 422 ab, und die Meldung
    eines Formatfehlers nannte den Wert."""
    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2024, "fall_id": "sn-typ-vj"})
    assert st == 201, r
    vj = API.lade_fall("sn-typ-vj")
    for fid, wert in (("vv_einnahmen", 120000), ("bruttoarbeitslohn", 5000000),
                      ("ep_ziel_adresse", "Werkstr. 1\x00"),
                      ("kind_wohnsitz_inland_zeitraum", "01.01-31.122")):
        ST.append_event(vj, feld_id=fid, wert=wert, zustand="bestaetigt",   # ohne bindung= wie ein Alt-Store
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS)
    API.speichere_fall("sn-typ-vj", vj)
    st, r = API.vorjahr(fall, {"vorjahr_fall_id": "sn-typ-vj"})
    assert st == 200, r
    assert r["uebernommen"] == 2, r
    assert r["uebersprungen"] == ["ep_ziel_adresse", "kind_wohnsitz_inland_zeitraum"], r
    assert "Werkstr" not in repr(r) and "31.122" not in repr(r), r
    aktiv = set(ST._aktives(API.lade_fall(fall)))
    assert {"vv_einnahmen", "bruttoarbeitslohn"} <= aktiv
    assert not aktiv & {"ep_ziel_adresse", "kind_wohnsitz_inland_zeitraum"}


# ------------------------------- ganzer Wert (Decision textfeld-format-aus-xsd-beim-speichern)

def _speichere(fid, wert):
    s = ST.leerer_store(2025, fall_id="sn-typ-ganz")
    return ST.append_event(s, feld_id=fid, wert=wert, zustand="bestaetigt",
                           herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                           schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS,
                           bindung=BINDUNG)


def test_idnr_mit_zehn_ziffern_wird_abgelehnt():
    """AK1: die IdNr hat im XSD genau 11 Ziffern (E0500406). Ohne `muster` nahm der Store jede
    Zeichenfolge an; eine fehlende Ziffer fiel erst beim Finanzamt auf. Alle 17 Felder mit festem
    Format prüft tests/test_bindungs_typ_vs_xsd_typ.py gegen das XSD selbst."""
    with pytest.raises(ValueError, match="fail-closed \\(Format\\)"):
        _speichere("kind_idnr", "1234567890")


def test_muster_prueft_den_ganzen_wert():
    """AK2: `re.match` mit `$` liess ein abschliessendes \\n durch, denn `$` trifft in Python auch
    VOR einem letzten Zeilenumbruch. XSD und Rust (`$` = Textende) weisen den Wert ab. Jedes Feld
    mit `muster` nimmt seinen beispielwert an und weist beispielwert + \\n ab."""
    felder = sorted(f for f, e in BINDUNG.items() if e.get("muster"))
    assert len(felder) >= 12, felder   # 8 Zeiträume und 4 Datumsfelder trugen schon vorher eins
    durchgelassen = []
    for fid in felder:
        _speichere(fid, BINDUNG[fid]["beispielwert"])
        try:
            _speichere(fid, BINDUNG[fid]["beispielwert"] + "\n")
            durchgelassen.append(fid)
        except ValueError:
            pass
    assert not durchgelassen, f"beispielwert + \\n gespeichert: {durchgelassen}"


def test_datum_prueft_den_ganzen_wert_mit_ziffern_0_bis_9():
    """AK2: die Datumsprüfung urteilt über den ganzen Wert und kennt nur 0-9, wie Rust
    (domain::Wert, ist_tt_mm_jjjj). `\\d` trifft in Python auch arabisch-indische Ziffern."""
    for wert in ("12.04.1985\n", "١٢.٠٤.١٩٨٥"):
        with pytest.raises(ValueError, match="fail-closed \\(Typ\\)"):
            _speichere("stammdaten_geburtsdatum", wert)


def test_leerer_text_wird_abgelehnt():
    """AK3: jeder Text-Kz-Typ im Schema verlangt mindestens ein Zeichen. Ein leerer Wert sagt
    nichts, was „nicht beantwortet“ nicht sagt; kein Produktpfad leert ein Feld mit ""."""
    with pytest.raises(ValueError, match="fail-closed \\(Typ\\)"):
        _speichere("stammdaten_nachname", "")


def test_unbekanntes_feld_id_durchlaesst():
    """Team-Lead-Vorgabe Schritt 3: unbekanntes feld_id -> durchlassen, nicht raten."""
    s = ST.leerer_store(2025, fall_id="sn-typ-unbekannt")
    ev = ST.append_event(s, feld_id="dieses_feld_gibt_es_nicht", wert="irgendwas", zustand="bestaetigt",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS,
                         bindung=BINDUNG)
    assert ev["wert"] == "irgendwas"


# ------------------------------------------------- Grad der Behinderung (Werteliste auf typ=int)
#
# Das ELSTER-Schema laesst an den beiden GdB-Kz nur 17 Werte zu (pattern 20|25|30|...|100,
# E10-2025.xsd). Die Bindung fuehrte bis 2026-10-01 nur einen Bereich 20..100 — jeder
# Zwischenwert wie 33 ging durch, ERiC wies dann die GANZE Erklaerung ab (rc=610001002,
# „The value '33' is not accepted by the pattern"), und der Nutzer erfuhr es erst beim Absenden.
#
# Bis hierher war die Werteliste ueberhaupt nur fuer `typ: enum` wirksam: `_typ_konform` las sie in
# keinem anderen Zweig. Ein `int`-Feld mit `enum_werte` sagte eine Grenze ZU, die der Schreibpfad
# nicht kannte — eine Zusage ohne Durchsetzung (Vault:
# decisions/grad-der-behinderung-folgt-dem-amtlichen-muster).

_GDB_FELDER = ("rentner_grad_der_behinderung", "rentner_grad_der_behinderung_partner",
               "kind_grad_der_behinderung")

# Die 17 Werte des amtlichen Musters, und die 0 als „nichts anzugeben". Die Null ist NICHT im
# Muster — sie steht in der Liste, weil sie in 39 echten Faellen als bestaetigter Wert liegt;
# `_schreibe_kz` laesst sie aus der Deklaration weg (Vault:
# decisions/speichern-lehnt-nullwerte-nicht-ab.md).
_GDB_ERLAUBT = (0, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 95, 100)
_GDB_VERBOTEN = (1, 19, 21, 33, 47, 61, 99, 101)


def test_grad_der_behinderung_nimmt_die_amtlichen_werte():
    """AK1: die drei GdB-Felder nehmen genau die Werte des amtlichen Musters an (plus die 0). Die
    Gegenprobe gehoert dazu: 25 und 45 muessen durchgehen, nicht nur 33 abgewiesen werden."""
    for fid in _GDB_FELDER:
        for w in _GDB_ERLAUBT:
            _speichere(fid, w)
        for w in _GDB_VERBOTEN:
            with pytest.raises(ValueError, match="fail-closed \\(Typ\\)"):
                _speichere(fid, w)


def test_grad_der_behinderung_abweisung_nennt_das_feld():
    """AK2: ein Zwischenwert wird abgewiesen, BEVOR die Erklaerung entsteht, und die Meldung nennt
    das Feld. ERiC nennt nur das Muster („The value '33' is not accepted by the pattern") — der
    Nutzer sieht dann nicht, welches Feld gemeint ist."""
    with pytest.raises(ValueError) as exc:
        _speichere("rentner_grad_der_behinderung", 33)
    assert "rentner_grad_der_behinderung" in str(exc.value)
    assert "33" in str(exc.value)


def test_grad_der_behinderung_in_der_deklaration_unveraendert():
    """Die Sperre sitzt am Schreibpfad, nicht am Mapper: was durchkommt, geht unveraendert hinaus.
    45 bleibt 45 in E0109708 — nicht gerundet auf 40. Der Ring stuft selbst (`min(100, gdb//10*10)`,
    catala_behinderten_pb), die Deklaration nicht."""
    import est_mapping as EM   # produkt/mapping liegt oben schon auf sys.path
    for w in (25, 45, 100):
        d = EM.deklariere({"rentner_grad_der_behinderung": {"wert": w, "zustand": "bestaetigt"}},
                          BINDUNG, vz=2025)
        assert str(d["deklaration"].get("E0109708")) == str(w), (w, d["deklaration"].get("E0109708"))


def test_grad_der_behinderung_null_bleibt_gueltig():
    """Die Null ist der Kern der Entscheidung speichern-kehnt-nullwerte-nicht-ab: sie heisst
    „nichts anzugeben" und darf NICHT abgewiesen werden. Ohne die 0 in der Werteliste faellt fast
    ein Drittel des echten Bestands durch (39 von 127 Eintragungen, gemessen 2026-10-01)."""
    for fid in _GDB_FELDER:
        _speichere(fid, 0)



def test_typ_konform_spiegelt_test_store_typ_ok():
    """produkt/store/store.py:_typ_konform ist ABSICHTLICH eine gespiegelte Kopie von
    tests/test_store.py:_typ_ok (Produktionscode importiert keine Testdatei) — dieser Test hält
    beide synchron: driftet die eine Semantik von der anderen, schlägt er an, statt dass es erst
    an einem falsch akzeptierten/abgelehnten Produktionswert auffällt."""
    from test_store import _typ_ok
    faelle = [
        (50000, "cent", None, True), ("50000", "cent", None, False), (True, "cent", None, False),
        (50000.5, "cent", None, False), (0, "cent", None, True),
        (True, "bool", None, True), (1, "bool", None, False), ("true", "bool", None, False),
        ("einzel", "enum", ["einzel", "zusammen"], True), ("x", "enum", ["einzel", "zusammen"], False),
        ("12.04.1985", "datum", None, True), ("1985-04-12", "datum", None, False),
        ("kein Datum", "datum", None, False), ("12.04.1985\n", "datum", None, False),
        ("١٢.٠٤.١٩٨٥", "datum", None, False),
        ("Text", "text", None, True), (5, "text", None, False), ("Maier\x00", "text", None, False),
        ("", "text", None, False),
    ]
    for wert, typ, enum_werte, erwartet in faelle:
        assert ST._typ_konform(wert, typ, enum_werte) == erwartet == _typ_ok(wert, typ, enum_werte), (
            f"Drift zwischen store._typ_konform und test_store._typ_ok bei wert={wert!r} typ={typ!r}")
