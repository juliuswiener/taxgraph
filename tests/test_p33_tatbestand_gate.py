"""§ 33 EStG: die zwei Tatbestandsmerkmale muessen den Abzug bewegen.

Gemessener Defekt bis hierher (2026-09-26): `bescheid_abzuege.py:325` uebergibt an
`catala_p33_agb` nur vier Schluessel (aussergewoehnliche_belastungen,
gesamtbetrag_der_einkuenfte, anzahl_kinder, splitting). Die zwei gesetzlichen
Voraussetzungen — `agb_zwangslaeufig` (§ 33 Abs. 1/2 S. 1 "zwangslaeufig") und
`agb_notwendig_angemessen` (§ 33 Abs. 2 S. 1 "notwendig ... angemessen") — erreichen
die Regel nie. Gemessen: bei 10.000 EUR Aufwand und 20.000 EUR Lohn lieferten
"ja", "nein" und ein fehlender Wert alle dieselbe Zahl (0 Cent Steuer).

Die Bindung kennt beide als `geltungsbedingung` an p33_1_2_agb_abzug (askable, typ bool);
keines ist eine Betragsgrenze. Das Gate gehoert daher in den Ring, die Catala-Regel bleibt
rein — Bauform wie `p35c_keine_doppelfoerderung` in derselben Datei, zwanzig Zeilen darueber.

Polaritaet nach der Entscheidung [[fehlende-antwort-sperrt-nicht-die-antwort-nein]]
(2026-09-26): ein FEHLENDER Wert sperrt (input_kegel_nicht_bestaetigt, tut der Kegel schon);
ein bestaetigtes NEIN sperrt NICHT, sondern ist ein rechenbares Ergebnis Null.
"""
import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import audit                # noqa: E402


from test_ui_rechenweg import _laie, _LOHN_60K_EINZEL  # noqa: E402

# 20.000 EUR Lohn, einzel, kinderlos — dieselbe Bauart wie _LOHN_60K_EINZEL, nur der Lohn.
_LOHN_20K_EINZEL = [(f, 2_000_000 if f == "bruttoarbeitslohn" else w)
                    for f, w in _LOHN_60K_EINZEL] + [("kein_kap", True)]

# Gemessen 2026-09-26, 20.000 EUR Lohn, scheibe gesamt, einzel, kinderlos.
# Die Steuer OHNE wirksamen agB-Abzug ist 1.327,00 EUR — unabhaengig davon, wie hoch der
# bestrittene Aufwand war. Mit gewaehrtem Abzug sinkt sie: bei 3.000 EUR auf 843,00 EUR,
# bei 10.000 EUR auf 0,00 EUR (der Abzug frisst dann die ganze Steuer).
# Zwei Aufwandsstufen, weil eine allein den Abzug nicht belegt: bei 10.000 EUR ist die
# erwartete Zahl 0, und 0 sieht wie "nichts gerechnet" aus.
_STEUER_OHNE_ABZUG = 132_700
_FAELLE = [(300_000, 84_300), (1_000_000, 0)]   # (agb_aufwendungen in Cent, Steuer mit Abzug)

_MERKMALE = ("agb_zwangslaeufig", "agb_notwendig_angemessen")


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    assert srv.server_address[0] == "127.0.0.1"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _antwort(base: str, fid: str, felder: list, weglassen: tuple = ()) -> dict:
    """Legt den Fall an, bestaetigt alles ausser `weglassen`, liefert die /ergebnis-Antwort."""
    req = urllib.request.Request(
        base + "/fall", data=json.dumps(
            {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid}).encode(),
        method="POST", headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req) as r:
        assert r.status == 201, r.read()
    for feld, wert in felder:
        if feld in weglassen:
            continue
        req = urllib.request.Request(
            base + f"/fall/{fid}/event", data=json.dumps(_laie(feld, wert)).encode(),
            method="POST", headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req) as r:
            assert r.status == 201, (feld, wert, r.read())
    with urllib.request.urlopen(base + f"/fall/{fid}/ergebnis") as r:
        return json.loads(r.read())


def _kegel(aufwand_cent: int, **merkmale) -> list:
    """Der bestaetigte Fall plus die zwei Tatbestandsmerkmale."""
    return _LOHN_20K_EINZEL + [("kein_gewinn", True), ("agb_aufwendungen", aufwand_cent),
                               *merkmale.items()]


# ---------------------------------------------------------------- (1) ROT ZUERST

@pytest.mark.parametrize("aufwand,mit_abzug", _FAELLE)
def test_zwangslaeufig_verneint_gewaehrt_keinen_abzug(base, aufwand, mit_abzug):
    """Ein bestaetigtes NEIN zur Zwangslaeufigkeit -> Abzug 0 EUR, die Steuer bleibt.

    § 33 Abs. 1 EStG: der Abzug setzt voraus, dass die Aufwendungen zwangslaeufig
    erwachsen. Ist das verneint, gibt es keinen Abzug — 1.327,00 EUR, nicht 843,00.
    """
    e = _antwort(base, f"p33-nein-{aufwand}", _kegel(
        aufwand, agb_zwangslaeufig=False, agb_notwendig_angemessen=True))
    assert e["grund"] == "bestaetigt", e
    assert e["zahl_cent"] == _STEUER_OHNE_ABZUG, (
        f"Zwangslaeufigkeit bestaetigt verneint, trotzdem Abzug gewaehrt: "
        f"{e['zahl_cent']} statt {_STEUER_OHNE_ABZUG} ct")
    # Kalibrierung: derselbe Aufwand MIT bejahtem Merkmal ergibt eine ANDERE Zahl.
    # Ohne sie waere nicht gezeigt, dass der Aufwand ueberhaupt wirkt.
    g = _antwort(base, f"p33-ja-kal-{aufwand}", _kegel(
        aufwand, agb_zwangslaeufig=True, agb_notwendig_angemessen=True))
    assert g["zahl_cent"] == mit_abzug, g


@pytest.mark.parametrize("aufwand,mit_abzug", _FAELLE)
def test_notwendig_angemessen_verneint_gewaehrt_keinen_abzug(base, aufwand, mit_abzug):
    """Dasselbe fuer das zweite Merkmal (§ 33 Abs. 2 S. 1: "notwendig ... angemessen")."""
    e = _antwort(base, f"p33-nicht-angemessen-{aufwand}", _kegel(
        aufwand, agb_zwangslaeufig=True, agb_notwendig_angemessen=False))
    assert e["grund"] == "bestaetigt", e
    assert e["zahl_cent"] == _STEUER_OHNE_ABZUG, e
    g = _antwort(base, f"p33-ja-kal2-{aufwand}", _kegel(
        aufwand, agb_zwangslaeufig=True, agb_notwendig_angemessen=True))
    assert g["zahl_cent"] == mit_abzug, g


# ---------------------------------------------------------------- (3) die Gegenproben

def test_beide_bejaht_gewaehrt_den_abzug(base):
    """Gegenprobe 1: bestaetigt bejaht -> der Abzug wird gewaehrt (0,00 EUR Steuer).

    Ohne diesen Fall prueft der Test nur, dass irgendetwas passiert: ein Ring, der den
    Abzug IMMER verweigert, waere mit (1) allein ebenfalls gruen.
    """
    e = _antwort(base, "p33-ja", _kegel(
        1_000_000, agb_zwangslaeufig=True, agb_notwendig_angemessen=True))
    assert e["grund"] == "bestaetigt", e
    assert e["zahl_cent"] == _FAELLE[1][1], (
        f"Beide Merkmale bestaetigt bejaht, trotzdem kein Abzug: {e['zahl_cent']}")


@pytest.mark.parametrize("fehlt", _MERKMALE)
def test_fehlende_antwort_sperrt(base, fehlt):
    """Gegenprobe 2: ein FEHLENDER Wert sperrt — kein stiller Abzug ueber einer Zahl.

    Polaritaet aus decisions/fehlende-antwort-sperrt-nicht-die-antwort-nein.md: fehlend
    sperrt, ein ausdrueckliches Nein nicht. Faellt dieser Test, ist das Gate zu weit
    gegangen (es sperrt ueber einer Antwort, die es gibt).
    """
    e = _antwort(base, f"p33-offen-{fehlt}", _kegel(
        1_000_000, agb_zwangslaeufig=True, agb_notwendig_angemessen=True), weglassen=(fehlt,))
    assert e["grund"] == "input_kegel_nicht_bestaetigt", e
    assert e["zahl_cent"] is None, e


def test_drei_faelle_drei_verschiedene_ergebnisse(base):
    """Der Kern in einer Zusicherung: verneint, bejaht und fehlend sind drei Zustaende.

    Zwei verschiedene Zahlen und eine Sperre — nicht dreimal dasselbe. Genau das war der
    Defekt: bis 2026-09-26 lieferten alle drei dieselbe Zahl.
    """
    ja = _antwort(base, "p33-k-ja", _kegel(
        1_000_000, agb_zwangslaeufig=True, agb_notwendig_angemessen=True))
    nein = _antwort(base, "p33-k-nein", _kegel(
        1_000_000, agb_zwangslaeufig=False, agb_notwendig_angemessen=True))
    offen = _antwort(base, "p33-k-offen", _kegel(
        1_000_000, agb_zwangslaeufig=True, agb_notwendig_angemessen=True),
        weglassen=("agb_zwangslaeufig",))
    assert (ja["zahl_cent"], nein["zahl_cent"], offen["zahl_cent"]) == (
        _FAELLE[1][1], _STEUER_OHNE_ABZUG, None), (ja, nein, offen)
    assert ja["grund"] == nein["grund"] == "bestaetigt", (ja, nein)
    assert offen["grund"] == "input_kegel_nicht_bestaetigt", offen
