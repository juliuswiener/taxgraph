"""Aufwands-Einzel-Kz runden wie ihre Summe: zu Gunsten des Nutzers auf volle Euro AUF.

Entstanden als ROT-Entwurf zu tickets/aufwendungs-kz-runden-zu-lasten-des-nutzers (quellen,
2026-10-01). Seit 2026-10-02 steht der Fix in Python UND Rust (Vault:
decisions/aufwand-einzelposten-aufrunden-summe-aus-posten): die vierzehn Einzel-Kz stehen in
_ABZUGS_KZ, die drei Paragraf-35a-Summen entstehen aus den gerundeten Posten
(_P35A_SUMME_AUS_POSTEN).

Anleitung ESt 1 A 2025 (anl_est1a_2025.txt:269-274): "Cent-Beträge runden Sie zu Ihren Gunsten
auf volle Euro-Beträge auf oder ab, es sei denn, die Vordrucke sehen ausdrücklich die Eintragung
von Cent-Beträgen vor." Alle Kz unten sind GanzzahlPosOhneFuehrNull (E10-2025.xsd), die
Vordruckzeilen tragen "EUR ,–". Cent sind also nicht vorgesehen, Aufwand rundet auf.

Bis zum Fix rundete _cent_nach_kz nur die Summen-Kz auf, die Einzel-Kz ab.
Bei Paragraf 35c (Regel 102240010) und eigener Berufsausbildung (Regel 1284) prueft ERiC Summe
gegen Einzelposten EXAKT: ein Betrag mit Cent machte die ganze Erklaerung ungueltig
(rc=610001002, gemessen). Bei Paragraf 35a liegt die Toleranz bei 2 Euro, bei Kinderbetreuung
bei 5 Euro. Mehrere Paragraf-35a-Posten scheiterten bis fc403c4 zusaetzlich am Writer (HA_35a
statt Einz wiederholt) — deshalb prueft der Mehrposten-Test die Deklaration, nicht ERiC.

Gemessen (2026-10-01, e64a8c4, Treiber ~/.cache/taxgraph-tmp/quellen/lauf_entwurf.py mit
--runxfail): ohne Fix 21 failed, mit dem Fix 21 passed.
"""

from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("elster", "produkt/haut", "produkt/eingang", "produkt/mapping",
            "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, HERE)

import api as API                   # noqa: E402
import audit                        # noqa: E402
import checkest_gate as CE          # noqa: E402
import est_mapping                  # noqa: E402
import store as ST                  # noqa: E402

from test_checkest_blockmatrix import BLOECKE, _block_scharf, _setz  # noqa: E402
from test_checkest_durchstich import _fall_einzel, braucht_eric  # noqa: E402

# (Einzel-Kz, Summen-Kz) mit Vordruckzeile
PAARE = [
    ("E0104108", "E0104109"),   # 35a Minijob, Anlage Haushaltsnahe Aufwendungen Zeile 4
    ("E0107207", "E0107208"),   # 35a Dienstleistungen, Zeile 5
    ("E0111214", "E0111215"),   # 35a Handwerker, Zeilen 6-8 und Summe Zeile 9
    ("E0108002", "E0108202"),   # eigene Berufsausbildung, Anlage Sonderausgaben Zeile 13
    ("E0506104", "E0506105"),   # Kinderbetreuung, Anlage Kind Zeile 67
] + [(kz, "E0241901") for kz in (   # 35c, Anlage Energetische Massnahmen (Vordruck fehlt in sources/)
    "E0241001", "E0241101", "E0241201", "E0241301", "E0241302",
    "E0241401", "E0241501", "E0241601", "E0241701")]

P35C = [(f, w) for f, w in BLOECKE["p35c_anlage_energetische_massnahmen"]
        if f != "p35c_sanierungsaufwendungen"]
AUSBILDUNG = [("berufsausbildung_bezeichnung", "Studium Betriebswirtschaft, Semesterbeiträge")]

# Name -> (Felder, Summen-Kz, Einzel-Kz, Euro), je 50 Cent ueber vollen Euro
EIN_POSTEN = {
    "p35c_heizung": (P35C + [("p35c_sanierungsaufwendungen", 2000050)],
                     "E0241901", "E0241501", 20001),
    "berufsausbildung": (AUSBILDUNG + [("berufsausbildung_aufwendungen", 200050)],
                         "E0108202", "E0108002", 2001),
}


@pytest.fixture(autouse=True)
def _isoliert(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


def _dekl(felder):
    store = _fall_einzel()
    for feld, wert in felder:
        _setz(store, feld, wert)
    store = dict(store)
    store["scheibe"] = "gesamt"
    bindung = API._scheibe_bindung(store)
    snap, sid = ST.materialisiere(store)
    snap = API._mit_ring_werten(snap, 2025)
    return est_mapping.deklariere(snap, bindung, snapshot_id=sid, vz=2025)


@pytest.mark.parametrize("einzel,summe", PAARE)
def test_einzel_kz_rundet_wie_seine_summe_auf(einzel, summe):
    assert (est_mapping._cent_nach_kz(200050, einzel),
            est_mapping._cent_nach_kz(200050, summe)) == (2001, 2001)


@pytest.mark.parametrize("fall", sorted(EIN_POSTEN))
def test_ein_posten_mit_cent_steht_in_summe_und_einzel_gleich(fall):
    felder, summe, einzel, euro = EIN_POSTEN[fall]
    d = _dekl(felder)["deklaration"]
    assert (d.get(summe), d.get(einzel)) == (euro, euro)


@pytest.mark.parametrize("praefix,art,einzel,summe", [
    ("hh_minijob", "Haushaltshilfe", "E0104108", "E0104109"),
    ("hh_dienstleistung", "Fensterreinigung", "E0107207", "E0107208"),
    ("hh_handwerker", "Malerarbeiten", "E0111214", "E0111215"),
])
def test_35a_summe_ist_die_summe_der_gerundeten_posten(praefix, art, einzel, summe):
    """4 x 100,01 EUR: jeder Posten 101, die Summe also 404 (Vordruck Zeile 9: "+ ... =").

    Ohne Fix 4 x 100 gegen aufgerundete Rohsumme 401. Nur die 14 Kz nachzutragen ergibt 4 x 101
    gegen 401 — ERiC-Toleranz 2 ueberschritten. Die Summe muss aus den gerundeten Posten kommen.
    """
    felder = []
    for i in range(4):
        s = "" if i == 0 else f"__{i + 1}"
        felder += [(f"{praefix}_betrag{s}", 10001), (f"{praefix}_art{s}", art)]
    dekl = _dekl(felder)
    posten = [dekl["deklaration"].get(einzel)] + [
        i["felder"][einzel] for lst in dekl["anlage_instanzen"].values() for i in lst
        if einzel in i["felder"]]
    assert (posten, dekl["deklaration"].get(summe)) == ([101] * 4, 404)


def test_35a_summe_ohne_posten_bleibt_stehen():
    """Gegenprobe: ohne Posten bleibt eine bestaetigte Summe stehen (Bestandswert vor 2026-08-10).

    Ersetzte die Summenbildung sie durch die leere Postensumme 0, verschwaende der Abzug still.
    tests/test_p35a_bestandsdaten.py prueft dasselbe gegen ERiC, hier ohne ERiC.
    """
    d = _dekl([("hh_handwerker_arbeitskosten", 300050)])["deklaration"]
    assert (d.get("E0111215"), d.get("E0111214")) == (3001, None)


@braucht_eric
@pytest.mark.parametrize("fall", sorted(EIN_POSTEN))
def test_ein_posten_mit_cent_ist_einreichbar(fall):
    rc, texte = _block_scharf(EIN_POSTEN[fall][0])
    assert rc == CE.RC_OK, f"{fall}: rc={rc}\n" + "\n".join(texte[:5])
