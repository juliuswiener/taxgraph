"""Den `bereich:` der Bindung prüft nur der Browser (Vault tickets/bindungsbereich-prueft-nur-der-browser.md,
gemessen 2026-10-02, Bericht ~/.cache/taxgraph-tmp/berichte/messung-35a-bereich.md).

42 Bindungen tragen `bereich: {min, max}`. Durchgesetzt wird er nur im HTML-Feld (`app.js`); POST /event
nimmt min-1 und max+1 mit 201 an, außer bei den drei Feldern mit `enum_werte` (422 über
store._wert_erlaubt). Danach rechnet der Ring still mit dem Wert, z. B. -1 auszubildende Kinder (§ 33a)
oder 367 Arbeitstage (Entfernungspauschale).

Verlangt ist nur, was in jeder Lösung gilt, die den Bereich als Zusage nimmt: ein Wert außerhalb wird
beim Schreiben abgewiesen (4xx), oder der Ring liefert keine Zahl. Die drei enum-Felder sind die
Kontrolle und MÜSSEN grün bleiben: sie belegen, dass der Test die Abweisung misst.

Daneben: zwei Werte INNERHALB des Bereichs enden in HTTP 500 auf /ergebnis und /stand —
rentner_renten_beginn_jahr nach dem VZ (runner.catala_renten_einkuenfte wirft
RentenfreibetragFixierungOffen am Guard vorbei) und rentner_alter_bei_rentenbeginn 98..100 (KeyError in
params/kohorten/rente_ertragsanteil_p22.yaml, Schlüssel 0..97)."""
from __future__ import annotations

import glob
import json
import os
import urllib.error
import urllib.request

import pytest
import yaml

from test_paket_b_e2e_http import _gesamt_kegel, _laie, _rentner_kegel, base  # noqa: F401 — Fixture

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NUR_RENTNER = {"rentner_renten_beginn_jahr", "rentner_alter_bei_rentenbeginn",
               "rentner_renten_beginn_jahr_partner", "rentner_alter_bei_rentenbeginn_partner"}


def _bereich_felder():
    out = []
    for p in sorted(glob.glob(os.path.join(ROOT, "produkt", "bindung", "*.yaml"))):
        with open(p, encoding="utf-8") as f:
            for b in yaml.safe_load(f).get("bindungen") or []:
                if "bereich" in b:
                    out.append((b["feld_id"], b["bereich"]["min"], b["bereich"]["max"], bool(b.get("enum_werte"))))
    return out


FELDER = _bereich_felder()


def _http(base, method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(base + path, data=data, method=method, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(r, timeout=60) as a:
            return a.status, json.loads(a.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"{}")


def _fall(base, fall_id, fid, wert, kegel=None):
    """Voller Kegel (gesamt bzw. rentner_gesamt), das Feld überschrieben. -> (event-Status, /ergebnis, /stand)."""
    sch = "rentner_gesamt" if fid in NUR_RENTNER else "gesamt"
    k = dict(kegel or (_rentner_kegel() if sch == "rentner_gesamt"
                       else [*_gesamt_kegel(0, bruttolohn=6000000, kein_vuv=True), ("veranlagung", "einzel")]))
    k[fid] = wert
    assert _http(base, "POST", "/fall", {"scheibe": sch, "veranlagungszeitraum": 2025, "fall_id": fall_id})[0] == 201
    st_feld = None
    for f, w in k.items():
        st, _ = _http(base, "POST", f"/fall/{fall_id}/event", _laie(f, w))
        if f == fid:
            st_feld = st
    return st_feld, _http(base, "GET", f"/fall/{fall_id}/ergebnis"), _http(base, "GET", f"/fall/{fall_id}/stand")


def _faelle():
    for fid, lo, hi, enum in FELDER:
        for name, w in (("min-1", lo - 1), ("max+1", hi + 1)):
            marks = () if enum else pytest.mark.xfail(
                strict=True, raises=AssertionError, reason="bereich nur im Browser geprüft (gemessen 2026-10-02)")
            yield pytest.param(fid, w, lo if w < lo else hi, id=f"{fid}-{name}", marks=marks)


def test_es_sind_42_bereich_felder_davon_3_enum():
    assert (len(FELDER), sum(e for *_, e in FELDER)) == (42, 3)


@pytest.mark.parametrize("fid,wert,nachbar", list(_faelle()))
def test_wert_ausserhalb_bereich_wird_abgewiesen_oder_sperrt(base, fid, wert, nachbar):
    st_feld, (st_erg, erg), _ = _fall(base, f"b-{fid}-{wert}".replace("_", "-"), fid, wert)
    assert st_erg < 500, erg
    if 400 <= st_feld < 500:
        return
    # Eine Sperre zählt nur, wenn der Nachbar im Bereich nicht genauso sperrt: tage_24h u. a. sperren im
    # Grundfall auch mit 366 (verpflegung_dreimonatsfrist_aufteilung_offen), das wäre kein Beleg.
    _, (_, im_bereich), _ = _fall(base, f"n-{fid}-{nachbar}".replace("_", "-"), fid, nachbar)
    assert erg.get("zahl_cent") is None and erg.get("grund") != im_bereich.get("grund"), (
        f"{fid}={wert}: Event {st_feld}, Ring {erg.get('zahl_cent')} Cent ({erg.get('grund')}); "
        f"{fid}={nachbar}: {im_bereich.get('zahl_cent')} Cent ({im_bereich.get('grund')})")


LEIBRENTE = _rentner_kegel(renten_art="private_leibrente")


@pytest.mark.parametrize("fid,wert,kegel", [
    ("rentner_renten_beginn_jahr", 2026, None),
    ("rentner_alter_bei_rentenbeginn", 98, LEIBRENTE),
])
def test_wert_im_bereich_endet_nicht_in_500(base, fid, wert, kegel):
    _, (st_erg, erg), (st_stand, stand) = _fall(base, f"i-{fid}".replace("_", "-"), fid, wert, kegel)
    assert (st_erg, st_stand) == (200, 200), (erg.get("fehler"), stand.get("fehler"))


def test_rentenbeginn_nach_vz_sperrt_benannt(base):
    _, (_, erg), _ = _fall(base, "nach-vz", "rentner_renten_beginn_jahr", 2026)
    assert (erg.get("zahl_cent"), erg.get("grund")) == (None, "rentenbeginn_nach_vz"), erg


# 2 Mio. EUR Leibrente: erst dann trägt 1 % gegen 2 % Ertragsanteil eine Steuer, die sich unterscheidet.
GROSSE_LEIBRENTE = _rentner_kegel(renten_art="private_leibrente", jahresrente=200000000)


@pytest.mark.parametrize("alter,gleich_wie", [(98, 97), (100, 97), (96, 97)])
def test_alter_ueber_97_rechnet_wie_97(base, alter, gleich_wie):
    """„… 94 bis 96 2 ab 97 1“ (sources/gesetze-im-internet/estg_p22_2026-07-13.txt:23).
    (96, 97) ist die Kontrolle: 2 % statt 1 % MUSS eine andere Zahl geben, sonst misst der Fall nichts."""
    fid = "rentner_alter_bei_rentenbeginn"
    _, (_, erg), _ = _fall(base, f"alter-{alter}", fid, alter, GROSSE_LEIBRENTE)
    _, (_, ref), _ = _fall(base, f"alter-{gleich_wie}-{alter}", fid, gleich_wie, GROSSE_LEIBRENTE)
    assert erg.get("zahl_cent") and ref.get("zahl_cent"), (erg, ref)
    assert (erg["zahl_cent"] == ref["zahl_cent"]) is (alter > 97), (erg["zahl_cent"], ref["zahl_cent"])
