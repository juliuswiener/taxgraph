"""Den `bereich:` der Bindung prüft nur der Browser (Vault tickets/bindungsbereich-prueft-nur-der-browser.md,
gemessen 2026-10-02, Bericht ~/.cache/taxgraph-tmp/berichte/messung-35a-bereich.md).

42 Bindungen tragen `bereich: {min, max}`. Durchgesetzt wird er nur im HTML-Feld (`app.js`); POST /event
nimmt min-1 und max+1 mit 201 an, außer bei den drei Feldern mit `enum_werte` (422 über
store._wert_erlaubt). Danach rechnet der Ring still mit dem Wert, z. B. -1 auszubildende Kinder (§ 33a)
oder 367 Arbeitstage (Entfernungspauschale).

Seit 2026-10-02 (decisions/zahl-ausserhalb-des-bereichs-wird-beim-speichern-abgewiesen-die-null-nicht)
weist der Store beim Schreiben jede Zahl AUSSERHALB von min..max ab (422), außer der 0: sie heißt bei diesen
Feldern „nichts anzugeben" und bleibt auch unter einem Minimum > 0 zulässig
(decisions/speichern-lehnt-nullwerte-nicht-ab). Darum prüft der erste Test min-1 und max+1 nur dort, wo
der Wert nicht 0 ist (11 Felder mit min 1 haben min-1 == 0), und der zweite Test hält die 0 fest.
Die drei enum-Felder sind die Kontrolle: sie wurden schon vorher über `enum_werte` abgewiesen.

Daneben: zwei Werte INNERHALB des Bereichs enden in HTTP 500 auf /ergebnis und /stand —
rentner_renten_beginn_jahr nach dem VZ (runner.catala_renten_einkuenfte wirft
RentenfreibetragFixierungOffen am Guard vorbei) und rentner_alter_bei_rentenbeginn 98..100 (KeyError in
params/kohorten/rente_ertragsanteil_p22.yaml, Schlüssel 0..97).

Der letzte Abschnitt gilt dem Ableitungsweg (decisions/ableitung-schreibt-keinen-wert-ausserhalb-des-bereichs):
`_leite_ab` und `_rechne_ab` hängen ihr Ergebnis direkt an die Ereignisliste und gingen an der Prüfung oben
vorbei. Eine Ableitung schreibt jetzt keinen Wert, der ausserhalb von `bereich` liegt; die Frage bleibt stehen."""
from __future__ import annotations

import glob
import json
import os
import urllib.error
import urllib.request

import pytest
import yaml

import store as ST  # conftest legt produkt/store auf sys.path
import traverser as TR
from test_p33b_abs5_s4_ring import KEGEL_BASIS
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
    for fid, lo, hi, _enum in FELDER:
        for name, w in (("min-1", lo - 1), ("max+1", hi + 1)):
            if w != 0:   # die 0 unter min bleibt zulässig, s. test_null_unter_minimum_wird_angenommen
                yield pytest.param(fid, w, lo if w < lo else hi, id=f"{fid}-{name}")


def test_es_sind_42_bereich_felder_davon_3_enum():
    assert (len(FELDER), sum(e for *_, e in FELDER)) == (42, 3)


@pytest.mark.parametrize("fid,wert,nachbar", list(_faelle()))
def test_wert_ausserhalb_bereich_wird_abgewiesen(base, fid, wert, nachbar):
    """422 beim Schreiben (AK1). Die alte Zweitlösung „oder der Ring sperrt" gilt nicht mehr: nur die
    Abweisung am Schreibweg schließt auch die KI und den direkten Aufruf. `nachbar` (der Randwert im
    Bereich) wird angenommen und rechnet ohne 500 — die Prüfung weist nicht zu viel ab."""
    st_feld, (st_erg, erg), _ = _fall(base, f"b-{fid}-{wert}".replace("_", "-"), fid, wert)
    assert st_feld == 422, f"{fid}={wert}: Event {st_feld}"
    assert st_erg < 500, erg
    st_nachbar, (st_erg_n, erg_n), _ = _fall(base, f"n-{fid}-{nachbar}".replace("_", "-"), fid, nachbar)
    assert st_nachbar == 201, f"{fid}={nachbar} (Rand im Bereich): Event {st_nachbar}"
    assert st_erg_n < 500, erg_n


@pytest.mark.parametrize("fid,lo", [(f, lo) for f, lo, _hi, enum in FELDER if lo > 0 and not enum])
def test_null_unter_minimum_wird_angenommen(base, fid, lo):
    """AK2: die 0 heißt bei diesen Feldern „nichts anzugeben" (129 Werte in 51 echten Akten,
    decisions/speichern-lehnt-nullwerte-nicht-ab) und bleibt auch unter einem Minimum > 0 zulässig.
    Was die Rechnung mit ihr tut, regeln die Sperren, nicht der Speicher. Hier nur der Schreibweg."""
    st_feld, (st_erg, erg), _ = _fall(base, f"null-{fid}".replace("_", "-"), fid, 0)
    assert st_feld == 201, f"{fid}=0 (min {lo}): Event {st_feld}"
    assert st_erg < 500, erg


def test_abweisung_nennt_feld_wert_und_bereich(base):
    """Die 422-Meldung trägt Feldname, Wert und Bereich (AK1) und sagt „Bereich" statt „Typ"."""
    assert _http(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                         "fall_id": "meldung-bereich"})[0] == 201
    st, body = _http(base, "POST", "/fall/meldung-bereich/event", _laie("fam_anzahl_kinder", -1))
    assert st == 422, body
    assert "fail-closed (Bereich)" in body["fehler"] and "fam_anzahl_kinder=-1" in body["fehler"], body
    assert "0 bis 20" in body["fehler"], body


def _bindung_mit_bereich():
    return {"zahl": {"typ": "int", "bereich": {"min": 1, "max": 5}},
            "betrag": {"typ": "cent", "bereich": {"min": -100, "max": 100}},
            "ohne": {"typ": "int"}}


def _schreibe(feld_id, wert, bindung=None):
    st = ST.leerer_store(2025, fall_id="bereich-store")
    return ST.append_event(
        st, feld_id=feld_id, wert=wert, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"},
        bindung=bindung or _bindung_mit_bereich())


@pytest.mark.parametrize("feld_id,wert", [("zahl", 1), ("zahl", 5), ("zahl", 0), ("betrag", -100),
                                           ("betrag", 100), ("betrag", 0), ("ohne", 999), ("zahl__2", 3),
                                           ("unbekannt", 12345)])
def test_store_nimmt_wert_im_bereich_die_null_und_felder_ohne_bereich(feld_id, wert):
    assert _schreibe(feld_id, wert)["wert"] == wert


@pytest.mark.parametrize("feld_id,wert", [("zahl", 6), ("zahl", -1), ("betrag", 101), ("betrag", -101),
                                           ("zahl__2", 6)])
def test_store_weist_wert_ausserhalb_ab(feld_id, wert):
    with pytest.raises(ValueError, match=r"^fail-closed \(Bereich\): "):
        _schreibe(feld_id, wert)


def test_store_ohne_bindung_prueft_nichts():
    """Die Prüfung hängt wie Auflage T an `bindung=`: Bestandsaufrufe ohne sie bleiben, Laden prüft nie."""
    st = ST.leerer_store(2025, fall_id="bereich-ohne")
    ev = ST.append_event(st, feld_id="zahl", wert=99, zustand="bestaetigt",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"})
    assert ev["wert"] == 99


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


@pytest.mark.parametrize("art", ["private_leibrente", "sonstige_leibrente"])
@pytest.mark.parametrize("beginn", [2026, 2025, 2024])
def test_leibrente_sperrt_nur_mit_beginn_nach_vz(base, art, beginn):
    """bb (Ertragsanteil) wie aa: nach dem VZ gibt es noch keine Rente in diesem Jahr (Vault-Entscheid
    partner-hebesatz-und-leibrente-nach-dem-steuerjahr-sperren-wie-ihr-gegenstueck). Im und vor dem VZ
    rechnet bb wie bisher; bb kennt keinen Rentenfreibetrag, also auch keine Fixierungssperre."""
    kegel = _rentner_kegel(renten_art=art, alter=65)
    _, (_, erg), _ = _fall(base, f"bb-{art}-{beginn}".replace("_", "-"), "rentner_renten_beginn_jahr", beginn, kegel)
    if beginn > 2025:
        assert (erg.get("zahl_cent"), erg.get("grund")) == (None, "rentenbeginn_nach_vz"), erg
    else:
        assert erg.get("zahl_cent") is not None, erg


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


# ---- Der Ableitungsweg: kein Wert ausserhalb von `bereich` ---------------------------------------------------
# `append_event` prüft Typ, Muster und Bereich; `_leite_ab`/`_rechne_ab` hängen ihr Ergebnis dagegen direkt an
# die Ereignisliste. Gemessen 2026-10-03: aus dem Geburtsdatum 15.07.1850 wurde `geburtsjahr` 1850 (Bereich
# 1900..2010) geschrieben, die Frage verschwand aus der Queue, und die Rechnung las 1850 als gesicherte Angabe.
# Das Datum selbst bekommt keine Grenze (eigene Entscheidung); nur die Ableitung daraus unterbleibt.

BINDUNG = TR.lade_bindung()
KATALOG = ST.lade_katalog(BINDUNG)
LAIE = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
QUELLEN = [("stammdaten_geburtsdatum", "geburtsjahr"), ("stammdaten_geburtsdatum_partner", "geburtsjahr_partner")]


def _store_mit(*paare, bindung=None):
    """Bestätigte Antworten in dieser Reihenfolge; ohne `bindung` gegen die echte Bindung samt Katalog."""
    s = ST.leerer_store(2025, fall_id="ableitung-bereich")
    for feld, wert in paare:
        ST.append_event(s, feld_id=feld, wert=wert, zustand="bestaetigt", herkunft=LAIE, schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"klick@{feld}"}, ts="2026-10-03T12:00:00+00:00",
                        bindung=bindung or BINDUNG, katalog=None if bindung else KATALOG)
    return s


def _abgeleitet(s):
    return {e["feld_id"]: e["wert"] for e in s["events"] if e["schreiber"].startswith("abgeleitet:")}


@pytest.mark.parametrize("quelle,ziel", QUELLEN)
@pytest.mark.parametrize("datum", ["15.07.1850", "31.12.1899", "01.01.2011", "15.07.2025", "15.07.2999"])
def test_ableitung_schreibt_kein_geburtsjahr_ausserhalb_des_bereichs(quelle, ziel, datum):
    """AK1: Jahr ausserhalb 1900..2010 (je eines unter min, über max, im Jahr 2999 wie im Auftrag): kein Event,
    die Frage bleibt offen. Das Datum selbst bleibt in der Akte."""
    s = _store_mit((quelle, datum))
    assert ziel not in _abgeleitet(s), f"{quelle}={datum}: Ableitung schrieb {ziel}={_abgeleitet(s)[ziel]}"
    assert ziel not in ST.materialisiere(s)[0]
    assert [e["wert"] for e in s["events"] if e["feld_id"] == quelle] == [datum]


@pytest.mark.parametrize("quelle,ziel", QUELLEN)
@pytest.mark.parametrize("datum,jahr", [("01.01.1900", 1900), ("15.07.1960", 1960), ("31.12.2010", 2010),
                                        ("01.01.0000", 0)])
def test_ableitung_schreibt_das_jahr_im_bereich_und_die_null(quelle, ziel, datum, jahr):
    """Die Ränder 1900 und 2010 und eine Mitte werden abgeleitet wie bisher. Die 0 (Datum 01.01.0000) geht wie
    beim Speichern durch (decisions/zahl-ausserhalb-des-bereichs-…-die-null-nicht): eine Regel, eine Stelle.
    Ob eine GERECHNETE 0 dasselbe verdient wie eine getippte, ist offen (Bericht, „Entscheidung für Julius")."""
    assert _abgeleitet(_store_mit((quelle, datum))).get(ziel) == jahr


def test_ableitung_ohne_bereich_am_ziel_laeuft_weiter():
    """Kontrolle: `rentner_alter_64_erfuellt` (bool, ohne `bereich`) wird auch für das Datum 1850 abgeleitet.
    Nur der Wert ausserhalb fällt weg, nicht die ganze Quelle."""
    abgeleitet = _abgeleitet(_store_mit(("stammdaten_geburtsdatum", "15.07.1850")))
    assert abgeleitet.get("rentner_alter_64_erfuellt") is True, abgeleitet


def _bindung_ableitung():
    return {"datum": {"typ": "datum"},
            "jahr": {"typ": "int", "bereich": {"min": 1900, "max": 2010},
                     "ableitung": {"aus": "datum", "art": "jahr_aus_datum", "grund": "t"}}}


def _bindung_beweist(wert):
    return {"anzahl": {"typ": "int", "beweist": {"feld_id": "ziel", "wert": wert, "ab": 1}},
            "ziel": {"typ": "int", "bereich": {"min": 1, "max": 5}}}


@pytest.mark.parametrize("datum,erwartet", [("15.07.1850", None), ("15.07.1960", 1960)])
def test_rechne_ab_prueft_den_bereich_des_ziels(datum, erwartet):
    """Dieselbe Regel ohne die echte Bindung: der Ableitungsschritt, nicht das Geburtsjahr, trägt die Prüfung."""
    s = _store_mit(("datum", datum), bindung=_bindung_ableitung())
    assert _abgeleitet(s).get("jahr") == erwartet


@pytest.mark.parametrize("wert,erwartet", [(99, None), (-1, None), (3, 3), (0, 0)])
def test_leite_ab_prueft_den_bereich_des_ziels(wert, erwartet):
    """`beweist` schreibt eine feste Zahl; liegt sie ausserhalb von `bereich` des Ziels, bleibt die Frage stehen.
    In der echten Bindung trägt kein `beweist`-Ziel einen Bereich: dieser Weg ist heute nur synthetisch erreichbar."""
    s = _store_mit(("anzahl", 3), bindung=_bindung_beweist(wert))
    assert _abgeleitet(s).get("ziel") == erwartet


# AK5: Auflage B bleibt. Die Prüfung hängt VOR dem Schreiben, `ziel in aktiv` sperrt wie bisher.

def test_bestaetigte_null_am_ziel_sperrt_die_ableitung_weiter():
    s = _store_mit(("geburtsjahr", 0), ("stammdaten_geburtsdatum", "15.07.1960"))
    assert [(e["wert"], e["schreiber"]) for e in s["events"] if e["feld_id"] == "geburtsjahr"] == [(0, "ui:laie")]


def test_vorjahreswert_am_ziel_sperrt_die_ableitung_weiter():
    """Ein vorläufiger Vorjahreswert bleibt stehen und gewinnt gegen die spätere Ableitung (Entscheidung Punkt 4)."""
    s = _store_mit()
    ST.append_event(s, feld_id="geburtsjahr", wert=1950, zustand="vorlaeufig",
                    herkunft={"herkunft": "vorjahr", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="import:vorjahr", signal={"signal_1": None, "signal_2": None},
                    ts="2026-10-03T12:00:00+00:00", bindung=BINDUNG, katalog=KATALOG)
    ST.append_event(s, feld_id="stammdaten_geburtsdatum", wert="15.07.1960", zustand="bestaetigt", herkunft=LAIE,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "klick"},
                    ts="2026-10-03T12:00:01+00:00", bindung=BINDUNG, katalog=KATALOG)
    assert [(e["wert"], e["zustand"]) for e in s["events"] if e["feld_id"] == "geburtsjahr"] == [(1950, "vorlaeufig")]


def _lauf_geburtsdatum(base, fall_id, datum):
    """Gesamt-Fall mit der Basis des p24a-Rückfalltests und (optional) einem Geburtsdatum.
    -> (zahl_cent, Fragen-IDs). Die Basis bestätigt den Kegel; `zahl_cent` ist die Steuerzahl des Bescheids."""
    assert _http(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                         "fall_id": fall_id})[0] == 201
    felder = [*KEGEL_BASIS, ("agb_zwangslaeufig", True), ("agb_notwendig_angemessen", True)]
    if datum:
        felder.append(("stammdaten_geburtsdatum", datum))
    for f, w in felder:
        st, antwort = _http(base, "POST", f"/fall/{fall_id}/event", _laie(f, w))
        assert st == 201, (f, w, antwort)
    _, erg = _http(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert erg["grund"] == "bestaetigt", erg
    _, fr = _http(base, "GET", f"/fall/{fall_id}/fragen")
    return erg["zahl_cent"], [f.get("feld_id") or f.get("feld") for f in fr["fragen"]]


@pytest.mark.parametrize("datum", ["15.07.1850", "15.07.2999"])
def test_jahr_ausserhalb_bleibt_frage_und_bewegt_die_zahl_nicht(base, datum):
    """Geld und Frage über HTTP (gemessen 2026-10-03, Python und Rust gleich): vor dem Fix machte 15.07.1850
    aus `geburtsjahr` 1850 den höchsten Altersentlastungsbetrag — 889 EUR weniger Steuer als ohne Angabe
    (15.267.300 gegen 15.356.200 ct); bei 2999 fiel nur die Frage weg. Jetzt rechnet der Fall wie einer ohne
    Geburtsdatum, und `geburtsjahr` steht weiter in der Queue, bis der Nutzer es selbst beantwortet."""
    ohne_zahl, ohne_fragen = _lauf_geburtsdatum(base, "ab-ohne", None)
    zahl, fragen = _lauf_geburtsdatum(base, f"ab-{datum}".replace(".", "-"), datum)
    assert "geburtsjahr" in ohne_fragen
    assert "geburtsjahr" in fragen, "die Ableitung hat die Frage still beantwortet"
    assert zahl == ohne_zahl, f"{datum}: Steuer {zahl} ct statt {ohne_zahl} ct"


def test_jahr_im_bereich_beantwortet_die_frage_und_bewegt_die_zahl(base):
    """Kontrolle zum Test oben: ohne sie wäre „zahl == ohne_zahl" auch gültig, wenn das Datum nie etwas bewegte."""
    ohne_zahl, _ = _lauf_geburtsdatum(base, "ab-ohne-k", None)
    zahl, fragen = _lauf_geburtsdatum(base, "ab-1958", "15.07.1958")
    assert "geburtsjahr" not in fragen
    assert zahl < ohne_zahl, (zahl, ohne_zahl)
