"""Die Kopfzeile (/stand) und /ergebnis müssen für JEDEN Abzugstopf dieselbe Zahl nennen.

Anlass: der Nutzer sah zwei Zahlen für einen Fall. Oben 10.678 €, unten 10.438 € — 240 €
Differenz, beide aus derselben Rechnung. Ursache war eine Naht mit DREI Rollen, die auf einer
Variable lagen (api.py, `_ring_bindung`, vor dem 2026-09-26):

  1. ENUMERIEREN — `EM.instanzen(store, bindung, gruppe)` liest `b.get("instanz_gruppe")`.
     Mit dem Pflicht-Kegel sieht der § 35a-Topf nur dessen Felder, findet keine Instanz und
     summiert 0. Er rechnet, als hätte der Nutzer nichts angegeben.
  2. ACHSEN — `intervall.py` bildet `askable` aus `b.get("askable")`. Mit der vollen Bindung
     werden 341 statt 21 Felder zu Achsen und die Spanne kippt auf `nicht_fixierbar`.
  3. SLOT-ÜBERSETZUNG — `bescheid_via_slots` liest `bindung[fid]["quelle"]["signatur_slot"]`
     und braucht eine Obermenge der Achsen-Bindung (Kegel ⊆ voll, konstruktiv).

Dieser Test prüft die EIGENSCHAFT, nicht den § 35a-Fall: für jeden Topf, der Instanzen hat,
muss `Spanne == Ergebnis` gelten. Gemessen divergierten sechs von zehn Töpfen (240 €, 510 €,
800 €, 1.035 €, 305 €, 402 €) — alle sechs stehen unten als Parameter.

Zwei Zusicherungen, die den Test NICHT vakuum-grün werden lassen:

  * `test_die_instanzen_gehen_verloren` — der Kegel verliert die Instanzen wirklich. Ohne sie
    wäre „Spanne == Ergebnis" auch dann grün, wenn der Topf gar nicht rechnet.
  * `test_volle_bindung_an_beiden_achsen_kippt` — die Gegenprobe, die den Fix vom naiven
    abgrenzt: volle Bindung AUCH auf der Achse ⇒ `min is None` und `nf > 0`. Wer die zwei
    Bindungen später wieder zu einer zusammenzieht, macht genau das und dieser Test wird rot.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden", "produkt/unsicherheit", "produkt/store",
            "produkt/traverser", "produkt/mapping"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, HERE)

import api as API            # noqa: E402
import audit                 # noqa: E402
import est_mapping as EM     # noqa: E402
import intervall as IV       # noqa: E402
import store as ST           # noqa: E402

from test_checkest_durchstich import _H, TS   # noqa: E402


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


@pytest.fixture(autouse=True)
def _isoliert(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


# Der gesamt-Kegel 2025, vollständig bestätigt. Ohne ihn liefert _feste_zahl keine Zahl, und
# der Test maße die Sperre statt der Spanne. Die vv_*-Felder stehen hier auf 0 — kein Topf
# unten schreibt sie, sonst wiese der Store das zweite Event auf dasselbe Feld mit 422 ab
# (fail-closed) und der Fall rechnete still ohne die Topfdaten.
SOCKEL = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 0), ("vv_einnahmen", 0),
    ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0), ("vv_erhaltungsaufwand", 0),
    ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100), ("ep_arbeitstage", 0),
    ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("einkuenfte_gewinn", 5000000), ("gewinn_betriebsart", "gewerbe"),
    # 2026-09-26, beim Merge von `186aa59` („fuenf unerreichbare Gates in die Feldlisten,
    # zwei davon in den Kegel"): AGB_TATBESTAND steht seitdem in `SCHEIBEN["gesamt"]["kegel"]`
    # (api_constants.py:836). Ohne diese zwei Zeilen sperrt jeder Topf mit
    # `input_kegel_nicht_bestaetigt` — der Test maesse die Sperre statt der Spanne. Sieben
    # Dateien tragen denselben Nachtrag, aus derselben Ursache.
    ("agb_zwangslaeufig", True), ("agb_notwendig_angemessen", True),
]

# Die sechs Töpfe, die gemessen auseinanderliefen. Gruppe = `instanz_gruppe` der Bindung.
# Alle sechs liegen auf `gesamt` und teilen denselben Sockel — deshalb EINE Schleife, kein
# Sonderfall je Topf. (Die restlichen vier Töpfe meiner Messung — Betreuung, GWG, § 23,
# § 21 vv_objekt — sind hier NICHT dabei: die ersten drei waren in meinem Aufbau gesperrt,
# ihr Auseinanderlaufen ist damit unbelegt und nicht widerlegt, s. Docstring unten.)
TOEPFE = [
    ("§ 35a Handwerker", "hh_handwerker", [
        ("hh_handwerker_betrag", 120000), ("hh_rechnung_unbar", True),
        ("hh_handwerker_keine_foerderung", True), ("hh_in_eu_ewr", True)]),
    ("§ 35a Minijob", "hh_minijob", [
        ("hh_minijob_betrag", 300000), ("hh_minijob_art", "haushalt"),
        ("hh_rechnung_unbar", True), ("hh_in_eu_ewr", True)]),
    ("§ 35a Dienstleistung", "hh_dienstleistung", [
        ("hh_dienstleistung_betrag", 400000), ("hh_dienstleistung_art", "haushalt"),
        ("hh_rechnung_unbar", True), ("hh_in_eu_ewr", True)]),
    ("§ 10.1.3 KV/PV Kind", "kind", [
        ("fam_anzahl_kinder", 1), ("kind_idnr", "12345678901"), ("kind_kv", 240000),
        ("kind_pv", 60000)]),
    ("§ 10.1.9 Schulgeld", "kind", [
        ("fam_anzahl_kinder", 1), ("schulgeld", 300000)]),
    ("§ 33b.5 Kind-PB", "kind", [
        ("fam_anzahl_kinder", 1), ("kind_idnr", "12345678901"),
        ("kind_grad_der_behinderung", 50), ("kind_behinderten_pb_antrag", True),
        ("kind_pb_nicht_selbst_genutzt", True)]),
]


def _fid(prefix: str, name: str) -> str:
    """fall_id aus dem Topfnamen, DETERMINISTISCH.

    Nicht `hash(name)`: der ist je Prozess gesalzen (PYTHONHASHSEED), und zwei Töpfe können
    auf dieselbe Zahl fallen — dann legt der zweite `fall_anlegen` mit 409 ab und der Test
    wird flaky, ohne dass sich am Code etwas geändert hätte (6 Namen auf 10.000 Eimer,
    gemessen ~0,15 % je Lauf). Die Kennung darf nur [A-Za-z0-9_-] tragen.
    """
    sauber = "".join(c if (c.isalnum() or c in "_-") else "_" for c in name)
    while "__" in sauber:
        sauber = sauber.replace("__", "_")
    return f"{prefix}_{sauber.strip('_')}"[:64]


# Die Kennungen müssen PAARWEISE VERSCHIEDEN sein — sonst teilen sich zwei Parametrisierungen
# denselben Fall, und der zweite Anlauf scheitert an „Fall existiert bereits" (409), was wie
# ein Testfehler aussieht und keiner ist.
_IDS = [_fid("kz", t[0]) for t in TOEPFE]
assert len(set(_IDS)) == len(_IDS), f"fall_ids nicht eindeutig: {_IDS}"


def _fall(fid: str, extra: list) -> str:
    """Sockel + Topfdaten, alles bestätigt. Der Store ist fail-closed gegen ein zweites Event
    auf dasselbe Feld — deshalb darf `extra` kein Sockelfeld wiederholen."""
    API.fall_anlegen({"fall_id": fid, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
    store = API.lade_fall(fid)
    for feld, wert in SOCKEL + list(extra):
        ST.append_event(store=store, feld_id=feld, wert=wert, zustand="bestaetigt",
                        herkunft=_H, schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"ok@{feld}"}, ts=TS)
    API.speichere_fall(fid, store)
    return fid


@pytest.mark.parametrize("name,gruppe,extra", TOEPFE, ids=[t[0] for t in TOEPFE])
def test_kopfzeile_nennt_die_ergebniszahl(name, gruppe, extra):
    """Die Eigenschaft: für diesen Topf deckt sich die Spanne der Kopfzeile mit der
    festgesetzten Zahl. Rot vor dem 2026-09-26-Fix für alle sechs Parameter."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    fid = _fall(_fid("kz", name), extra)

    _, erg = API.ergebnis(fid)
    assert erg["grund"] == "bestaetigt" and erg["zahl_cent"], (
        f"Vorbedingung: /ergebnis muss für {name} eine bestätigte Zahl liefern, sonst misst "
        f"der Test die Sperre statt der Spanne: {erg}")
    zahl = erg["zahl_cent"]

    iv = API.stand(fid)[1]["intervall"]
    assert iv["min_cent"] == iv["max_cent"] == zahl, (
        f"{name}: Kopfzeile {iv['min_cent']}–{iv['max_cent']} neben der festgesetzten Zahl "
        f"{zahl} aus /ergebnis. Beide kommen aus derselben Rechnung — der Nutzer sieht zwei "
        f"Zahlen und kann nicht entscheiden, welche gilt.")


@pytest.mark.parametrize("name,gruppe,extra", TOEPFE, ids=[t[0] for t in TOEPFE])
def test_die_instanzen_gehen_verloren(name, gruppe, extra):
    """Nicht-Vakuum-Zusicherung: der Pflicht-Kegel verliert die Instanzen dieses Topfes
    wirklich. Ohne diese Zusicherung wäre „Spanne == Ergebnis" auch dann grün, wenn der Topf
    überhaupt nicht rechnet — dann wären beide Zahlen gleich falsch."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    fid = _fall(_fid("ki", name), extra)
    store = API.lade_fall(fid)
    bindung = API._scheibe_bindung(store)
    cfg = API._cfg(store)
    aufbau, achsen = API._ring_bindungen(cfg, bindung, store)

    assert len(EM.instanzen(store, aufbau, gruppe)) > 0, (
        f"{name}: die Aufbau-Bindung findet keine Instanz der Gruppe {gruppe} — dann rechnet "
        f"der Topf mit 0, und jede Übereinstimmung oben wäre bedeutungslos.")
    assert len(EM.instanzen(store, achsen, gruppe)) == 0, (
        f"{name}: die Achsen-Bindung findet Instanzen der Gruppe {gruppe}. Der Kegel enthält "
        f"kein einziges Feld dieser Gruppe — wenn sich das ändert, muss der Test neu bewertet "
        f"werden, statt weiter zu gelten.")


def test_volle_bindung_an_beiden_achsen_kippt():
    """Die Gegenprobe, die den Fix vom naiven abgrenzt.

    Derselbe Fall, dieselbe Aufbau-Bindung (voll), aber die ACHSEN bekommen ebenfalls die
    volle Bindung — genau das, was ein „Vereinfacher" tut, der die zwei Zeilen wieder
    zusammenzieht. Ergebnis: `min is None`, `nicht_fixierbar` nicht leer. Der Docstring von
    `_ring_bindung` behauptet genau das, und diese Zusicherung hält die Behauptung am Leben:
    ohne sie kann jemand vereinfachen und den Defekt zurückholen, ohne dass etwas rot wird."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    name, gruppe, extra = TOEPFE[0]
    fid = _fall("gegen_min_none", extra)
    store = API.lade_fall(fid)
    bindung = API._scheibe_bindung(store)
    cfg = API._cfg(store)
    felder, sid = ST.materialisiere(store)
    aufbau, achsen = API._ring_bindungen(cfg, bindung, store)

    bf = API._bescheid_fn(cfg["gesamt_ring"], 2025, aufbau, felder, store, nur_bestaetigt=False)
    assert bf is not None, "Vorbedingung: der Bescheid muss baubar sein, sonst misst der Test nichts"

    iv_voll = IV.intervall(felder, bindung, bf, snapshot_id=sid)["intervall"]
    assert iv_voll["min_cent"] is None, (
        f"Mit der vollen Bindung auf der Achse muss die Spanne kippen — sie lieferte "
        f"{iv_voll['min_cent']}. Dann ist die Kegel-Bindung auf der Achse überflüssig und der "
        f"Docstring von _ring_bindung beschreibt einen Defekt, den es nicht gibt.")
    assert len(iv_voll["nicht_fixierbar"]) > 0, iv_voll
    assert len(iv_voll["nicht_fixierbar"]) > len(achsen), (
        f"die volle Bindung ({len(bindung)} Felder) hat {len(iv_voll['nicht_fixierbar'])} "
        f"nicht_fixierbare Felder, der Kegel {len(achsen)} — der Unterschied ist der Grund "
        f"für die Trennung")

    # Und der Kegel auf der Achse liefert die Zahl: derselbe Fall, eine Zeile anders.
    iv_kegel = IV.intervall(felder, achsen, bf, snapshot_id=sid)["intervall"]
    assert iv_kegel["min_cent"] is not None and iv_kegel["min_cent"] == iv_kegel["max_cent"], (
        f"der Kegel auf der Achse muss eine feste Zahl liefern: {iv_kegel}")
