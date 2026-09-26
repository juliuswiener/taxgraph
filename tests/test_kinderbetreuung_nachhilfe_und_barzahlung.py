"""§ 10 Abs. 1 Nr. 5 S. 2 und S. 4 EStG — zwei Ausschluesse, die TaxGraph nie abfragte.

S. 2: "Dies gilt nicht fuer Aufwendungen fuer Unterricht, die Vermittlung besonderer
Faehigkeiten sowie fuer sportliche und andere Freizeitbetaetigungen."
S. 4: "Voraussetzung fuer den Abzug der Aufwendungen nach Satz 1 ist, dass der
Steuerpflichtige fuer die Aufwendungen eine Rechnung erhalten hat und die Zahlung auf
das Konto des Erbringers der Leistung erfolgt ist"

Gemessen im Defer-Sweep (Fund 4, 2026-09-26): 6.000 EUR Nachhilfe und 6.000 EUR bar
bezahlte Betreuung lieferten BEIDE den vollen Abzug von 4.800 EUR. Der Fragetext warnte
zwar ("Bitte nur Betreuungskosten, keine Verpflegung oder Unterricht"), gefragt wurde
nie — eine Warnung im Fliesstext ist keine Voraussetzung.

Die Sperre greift NUR bei Betrag > 0 (conditional-mandatory, wie § 6 Abs. 2 GWG): ohne
Betrag gibt es nichts abzuziehen, dann sind beide Fragen gegenstandslos. Ein bestaetigtes
"nein" sperrt dagegen SEHR WOHL — anders als bei hh_rechnung_unbar, wo der Ring die
betroffene Quote nullt. Hier gibt es keine Teilquote: ein gemischter Betrag wird nicht
aufgeteilt (s. `ponytail:` in der Bindungstabelle), also muss der Nutzer ihn selbst
korrigieren. Ein stiller Abzug waere Under-tax, eine stille Null ein Geldverlust ohne
Hinweis.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API        # noqa: E402
import audit             # noqa: E402
import store as ST       # noqa: E402
import traverser as TR   # noqa: E402

from _kegel import kegel_fuer  # noqa: E402

VZ = 2025

REINE_BETREUUNG = "kind_betreuung_reine_betreuung"
ZAHLUNG = "kind_betreuung_rechnung_ueberweisung"
UNTER_14 = "kind_unter_14_haushaltszugehoerig"
BETRAG = "kinderbetreuungskosten"

GRUND_REINE_BETREUUNG = "kinderbetreuung_reine_betreuung_offen"
GRUND_ZAHLUNG = "kinderbetreuung_zahlung_offen"

# 6.000 EUR: 80 % = 4.800 = genau der Deckel aus S. 1. Damit ist der Abzug 4.800 EUR —
# unabhaengig davon, ob der Betrag 6.000 oder 8.000 ist.
SECHTAUSEND = 600000


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


# Kegel aus tests/test_p10_1_5_ring.py (GESAMT_KEGEL_BASIS / RENTNER_KEGEL_BASIS) —
# 200.000 EUR Einkuenfte, damit der Abzug voll in der 42-%-Zone greift und die Deltas
# dort schon gemessen sind.
# GEBAUT, nicht kopiert (tests/_kegel.py). Von Hand standen hier 34 bzw. 26 Felder; nur die
# drei bzw. vier unten sind echte Werte (gemessen 2026-09-26). Der Rest trug genau den
# Abwesenheitswert und war reine Kopie. Beide Handlisten kannten
# `agb_zwangslaeufig`/`agb_notwendig_angemessen` nicht — seit die im Kegel stehen, sperrten
# die Faelle, statt zu messen.
GESAMT_KEGEL = kegel_fuer("gesamt", {
    "vv_entgelt_quote_prozent": 100,
    "kein_gewinn": False,            # Gewinneinkuenfte sind der Fall
    "einkuenfte_gewinn": 20000000,   # 200.000 EUR, damit der Abzug voll in der 42-%-Zone greift
    "gewinn_betriebsart": "gewerbe",
})

RENTNER_KEGEL = kegel_fuer("rentner_gesamt", {
    "rentner_jahresrente": 20000000,
    "rentner_renten_beginn_jahr": 2025,
    "rentner_alter_bei_rentenbeginn": 65,
    "kein_sonstige": False,
})

KEGEL = {"gesamt": GESAMT_KEGEL, "rentner_gesamt": RENTNER_KEGEL}

# Gemessen in tests/test_p10_1_5_ring.py (2026-08-11) fuer 8.000 EUR bei EINEM Kind:
# 4.800 EUR Abzug x Grenzsatz. 6.000 EUR ergeben denselben Abzug (Deckel), also dasselbe Δ.
DELTA_EIN_KIND = {"gesamt": 200000, "rentner_gesamt": 200100}


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _fall(tmp_path, monkeypatch, scheibe, fid, antworten=()):
    """Legt einen Fall der Scheibe an, setzt den Kegel und `antworten` obendrauf."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, r = API.fall_anlegen({"scheibe": scheibe, "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201, r
    for feld, wert in KEGEL[scheibe] + list(antworten):
        st, r = API.event(fid, _laie(feld, wert))
        assert st == 201, f"{feld}={wert}: {st} {r}"
    st, erg = API.ergebnis(fid)
    assert st == 200, erg
    return erg


def _betreuung(betrag=SECHTAUSEND, unter14=True, reine=None, zahlung=None):
    """Der Fall-Aufbau der Kinderbetreuung. `reine`/`zahlung` = None laesst die Frage offen."""
    a = [(UNTER_14, unter14)]
    if betrag is not None:
        a.append((BETRAG, betrag))
    if reine is not None:
        a.append((REINE_BETREUUNG, reine))
    if zahlung is not None:
        a.append((ZAHLUNG, zahlung))
    return a


# ===== S. 2: keine Aufwendungen fuer Unterricht ===========================

@pytest.mark.parametrize("scheibe", ["gesamt", "rentner_gesamt"])
def test_nachhilfe_sperrt(tmp_path, monkeypatch, scheibe):
    """6.000 EUR Nachhilfe, beide Voraussetzungen sonst erfuellt: S. 2 schliesst den Abzug
    aus. Ohne Sperre flossen die vollen 4.800 EUR fuer eine Leistung, die das Gesetz
    ausdruecklich ausnimmt (Defer-Sweep Fund 4, gemessen 2026-09-26)."""
    erg = _fall(tmp_path, monkeypatch, scheibe, f"kbk-nach-{scheibe}",
                _betreuung(reine=False, zahlung=True))
    assert erg["grund"] == GRUND_REINE_BETREUUNG, (
        f"Nachhilfe ist keine Betreuung (S. 2) und muss sperren, grund={erg['grund']!r}")
    assert erg["zahl_cent"] is None, (
        f"gesperrt heisst keine Zahl, sonst ist die Sperre Dekoration: {erg['zahl_cent']}")


# ===== S. 4: Rechnung und Zahlung auf das Konto ===========================

@pytest.mark.parametrize("scheibe", ["gesamt", "rentner_gesamt"])
def test_barzahlung_sperrt(tmp_path, monkeypatch, scheibe):
    """6.000 EUR Betreuung, aber bar bezahlt: S. 4 verlangt die Zahlung auf das Konto des
    Erbringers. Dieselbe Sperre wie beim Nachhilfe-Fall — der Grund ist ein anderer, damit
    der Nutzer weiss, WAS er korrigieren muss."""
    erg = _fall(tmp_path, monkeypatch, scheibe, f"kbk-bar-{scheibe}",
                _betreuung(reine=True, zahlung=False))
    assert erg["grund"] == GRUND_ZAHLUNG, (
        f"Barzahlung erfuellt S. 4 nicht und muss sperren, grund={erg['grund']!r}")
    assert erg["zahl_cent"] is None, f"gesperrt heisst keine Zahl: {erg['zahl_cent']}"


@pytest.mark.parametrize("scheibe", ["gesamt", "rentner_gesamt"])
def test_unbeantwortet_sperrt(tmp_path, monkeypatch, scheibe):
    """Betrag steht, keine der beiden Fragen beantwortet: fail-closed bei WERTEN. Ein Abzug
    ohne die gesetzlichen Voraussetzungen ist ein erfundener Wert."""
    erg = _fall(tmp_path, monkeypatch, scheibe, f"kbk-offen-{scheibe}", _betreuung())
    assert erg["grund"] in (GRUND_REINE_BETREUUNG, GRUND_ZAHLUNG), (
        f"unbeantwortete Voraussetzungen muessen sperren, grund={erg['grund']!r}")
    assert erg["zahl_cent"] is None, f"gesperrt heisst keine Zahl: {erg['zahl_cent']}"


def test_eine_von_zwei_offen_reicht(tmp_path, monkeypatch):
    """Nur die Betreuungsfrage beantwortet, die Zahlungsfrage nicht: eine UND-Kette darf
    nicht an der ersten Frage haengenbleiben, sonst waere die zweite wirkungslos."""
    erg = _fall(tmp_path, monkeypatch, "gesamt", "kbk-halb",
                _betreuung(reine=True, zahlung=None))
    assert erg["grund"] == GRUND_ZAHLUNG, (
        f"die offene Zahlungsfrage allein muss sperren, grund={erg['grund']!r}")


# ===== Gegenproben: die Sperre darf nicht ueberfeuern =====================

@pytest.mark.parametrize("scheibe", ["gesamt", "rentner_gesamt"])
def test_kontrollfall_beide_ja_zieht_ab(tmp_path, monkeypatch, scheibe):
    """Der Normalfall: reine Betreuung, per Rechnung und Ueberweisung bezahlt. Der Abzug
    muss GENAU so gross sein wie vor dieser Aenderung — gemessen am Delta gegen denselben
    Fall ohne Kinderbetreuung. Eine Sperre, die den ehrlichen Nutzer trifft, waere
    fail-closed am falschen Ort."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    basis = _fall(tmp_path, monkeypatch, scheibe, f"kbk-base-{scheibe}", [])
    mit = _fall(tmp_path, monkeypatch, scheibe, f"kbk-ja-{scheibe}",
                _betreuung(reine=True, zahlung=True))
    assert mit["grund"] == "bestaetigt", f"beide ja darf nicht sperren: {mit['grund']!r}"
    delta = basis["zahl_cent"] - mit["zahl_cent"]
    assert delta == DELTA_EIN_KIND[scheibe], (
        f"4.800 EUR Abzug erwartet (Δ={DELTA_EIN_KIND[scheibe]}), war Δ={delta} "
        f"(basis={basis['zahl_cent']} mit={mit['zahl_cent']})")


def test_ohne_betrag_sperrt_nicht(tmp_path, monkeypatch):
    """Kein Betrag = nichts abzuziehen. Dann sind beide Fragen gegenstandslos und duerfen
    die Abgabe nicht sperren — sonst verlangt das Programm Angaben zu einem Abzug, den es
    selbst nicht gewaehrt (dieselbe Regel wie beim GWG ueber 800 EUR)."""
    erg = _fall(tmp_path, monkeypatch, "gesamt", "kbk-null", _betreuung(betrag=0))
    assert erg["grund"] == "bestaetigt", (
        f"0 EUR Betreuungskosten: nichts zu fragen, grund={erg['grund']!r}")


def test_kind_ueber_14_sperrt_nicht(tmp_path, monkeypatch):
    """Ein Kind ueber 14 zaehlt ohnehin nicht mit (S. 1, _kinderbetreuung_summe laesst es
    fallen). Seine Betreuungskosten duerfen die Abgabe nicht sperren — sonst haengt der
    ganze Ring an einer Instanz, die das Programm selbst schon aussortiert hat."""
    erg = _fall(tmp_path, monkeypatch, "gesamt", "kbk-ueber14",
                _betreuung(unter14=False, reine=False, zahlung=False))
    assert erg["grund"] == "bestaetigt", (
        f"Kind ueber 14 zaehlt nicht mit, also nichts zu sperren, grund={erg['grund']!r}")


def test_zweites_kind_sperrt_auch(tmp_path, monkeypatch):
    """Die Sperre laeuft je Kind-Instanz. Kind 1 vollstaendig und einwandfrei, Kind 2 mit
    Nachhilfe: die Sperre muss auch dann feuern. Eine Pruefung nur an Instanz 1 waere genau
    die Naht, an der die Betreuung schon einmal Geld gekostet hat."""
    erg = _fall(tmp_path, monkeypatch, "gesamt", "kbk-kind2", [
        (UNTER_14, True), (BETRAG, SECHTAUSEND),
        (REINE_BETREUUNG, True), (ZAHLUNG, True),
        (UNTER_14 + "__2", True), (BETRAG + "__2", SECHTAUSEND),
        (REINE_BETREUUNG + "__2", False), (ZAHLUNG + "__2", True),
    ])
    assert erg["grund"] == GRUND_REINE_BETREUUNG, (
        f"Kind 2 mit Nachhilfe muss sperren, grund={erg['grund']!r}")


# ===== Erreichbarkeit: beide Fragen muessen im Dialog stehen ==============

@pytest.mark.parametrize("scheibe", ["gesamt", "rentner_gesamt"])
def test_beide_fragen_stehen_im_dialog(scheibe):
    """Beide Fragen muessen gestellt werden, sonst schaltet die Sperre nie ab. Der Dialog
    wird hier NICHT durchgeklickt: der Betrag und die Qualifikation stehen im Store, und
    die beiden Fragen muessen danach in der Queue auftauchen.

    Mutation: die Felder aus api_constants.KINDERBETREUUNG entfernen (dann kennt die
    Scheibe sie nicht mehr) oder `askable` in der Bindungstabelle streichen — beide machen
    diesen Test rot."""
    store = ST.leerer_store(VZ, fall_id=f"kbk-dialog_{scheibe}")
    store["scheibe"] = scheibe
    bindung = API._scheibe_bindung(store)
    for feld, wert in [(UNTER_14, True), (BETRAG, SECHTAUSEND)]:
        ST.append_event(store=store, feld_id=feld, wert=wert, zustand="bestaetigt",
                        herkunft={"quelle": "kbk_dialog"}, schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"ok@{feld}"},
                        ts="2026-09-26T12:00:00Z")
    queue = TR.naechste_fragen(store, bindung)
    for fid in (REINE_BETREUUNG, ZAHLUNG):
        assert fid in queue, (
            f"{fid} fehlt in der Fragen-Queue der Scheibe {scheibe} — die Sperre kann nie "
            f"abschalten ({{konditionsfeld selbst versteckt}}). Queue: {queue}")
