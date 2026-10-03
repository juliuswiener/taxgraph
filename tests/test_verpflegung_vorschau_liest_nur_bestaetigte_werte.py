"""Die Verpflegungs-Vorschau (E0205508, `_mit_ring_werten` Block (1)) liest nur BESTAETIGTE Werte.

Entscheid verpflegung-vorschau-liest-nur-bestaetigte-werte (2026-10-03), Messung `neunc` auf 69119b9
(~/.cache/taxgraph-tmp/berichte/ring-werte-blocks.md): Der Block las Rohwerte ALLER Felder, auch vorlaeufiger
(Vorjahres-Vorschlaege, nie bestaetigt). Vorlaeufige `tage_24h` aenderten E0205508 von 56 auf 140 EUR, ein einziger
vorlaeufiger Tage-Topf machte aus "keine Zeile" 84 -- in einer Antwort, die `eingaben_konsistent=false` meldete.

Zwei Ebenen, je Feld drei Faelle (bestaetigt / fehlt / vorlaeufig), alles andere bestaetigt:
  - Bibliothek (`_mit_ring_werten` direkt): Kontrolle bestaetigt != fehlt (das Feld wirkt), vorlaeufig == fehlt.
  - HTTP `/deklaration` (echter Server, Scheibe gesamt, Waechter erfuellt): vorlaeufig zeigt dieselbe Zeile wie
    "Feld fehlt", `eingaben_konsistent=false` und das Feld steht unter `unvollstaendig`.
Der Waechter `_an_gesamt_sperrgrund` bleibt: vorlaeufige Nach-Frist-Tage bei `vpf_monate_am_ort > 3` sperren weiter
mit 409 `verpflegung_dreimonatsfrist_aufteilung_offen`, wie "Feld fehlt".

Das Gegenstueck in Rust: rust/bescheid/tests/offene_defekte.rs (`verpflegung_vorlaeufiges_feld_zaehlt_wie_fehlendes`).
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/bescheid", "produkt/haut", "produkt/store", "produkt/traverser",
             "produkt/unsicherheit", "produkt/mapping", "produkt/konsistenz",
             "produkt/eingang", "produkt/engine", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

os.environ["TAXGRAPH_NO_AUTH"] = "1"   # wie tests/conftest.py -- sonst 401 auf /fall

import api as API              # noqa: E402
import audit                   # noqa: E402
import bescheid_deklaration as BD   # noqa: E402
import server as SRV           # noqa: E402

from _kegel import kegel_fuer  # noqa: E402

KUERZUNG = "p9_4a_kuerzung_nach_entgelt"
GRUND_WAECHTER = "verpflegung_dreimonatsfrist_aufteilung_offen"

MAHLZEITEN_M = {"vpf_fruehstuecke_gestellt_anzahl": 5, "vpf_mittagessen_gestellt_anzahl": 2,
                "vpf_abendessen_gestellt_anzahl": 2}
MAHLZEITEN_P = {"vpf_fruehstuecke_gestellt_anzahl": 10, "vpf_mittagessen_gestellt_anzahl": 10,
                "vpf_abendessen_gestellt_anzahl": 10}
TAGE_P = {"tage_24h": 3, "tage_an_abreise": 2, "tage_ueber_8h_eintaegig": 2}
NACH = {"vpf_tage_24h_nach_drei_monaten": 2, "vpf_tage_an_abreise_nach_drei_monaten": 1,
        "vpf_tage_ueber_8h_nach_drei_monaten": 1}


def _ohne(d, feld):
    return {k: v for k, v in d.items() if k != feld}


def _faelle():
    """(name, feld, wert, basis): `basis` ist bestaetigt und enthaelt `feld` nicht.

    Basis M: 100 Tage a 28 EUR, die Mahlzeiten binden die Kuerzung. Basis P: je 10 Mahlzeiten, die Tage binden
    sie. Die Nach-Frist-Felder laufen auf P; die zwei uebrigen Nach-Frist-Felder stehen bestaetigt auf 0.
    """
    m = {"tage_24h": 100, **MAHLZEITEN_M}
    faelle = [(f, f, w, _ohne(m, f)) for f, w in MAHLZEITEN_M.items()]
    faelle.append(("entgelt", "vpf_mahlzeiten_gezahltes_entgelt", 2000, m))
    p = {**TAGE_P, **MAHLZEITEN_P}
    faelle += [(f, f, w, _ohne(p, f)) for f, w in TAGE_P.items()]
    # Ein einziger vorlaeufiger Tage-Topf: ohne ihn keine Zeile, mit ihm (vorher) 84 EUR.
    faelle.append(("einziger_tage_topf", "tage_24h", 3, MAHLZEITEN_P))
    for f, w in NACH.items():
        faelle.append((f, f, w, {**p, **{g: 0 for g in NACH if g != f}}))
    return faelle


FAELLE = _faelle()


# ---------------------------------------------------------------- Bibliothek

def _ev(wert, zustand):
    return {"wert": wert, "zustand": zustand,
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}}


def _kuerzung_cent(basis, feld=None, wert=None, zustand="bestaetigt"):
    felder = {f: _ev(w, "bestaetigt") for f, w in basis.items()}
    if feld is not None:
        felder[feld] = _ev(wert, zustand)
    BD._mit_ring_werten(felder, 2025)
    return (felder.get(KUERZUNG) or {}).get("wert")


@pytest.mark.parametrize("name,feld,wert,basis", FAELLE, ids=[f[0] for f in FAELLE])
def test_vorlaeufiges_feld_zaehlt_wie_fehlendes_in_der_bibliothek(name, feld, wert, basis):
    fehlt = _kuerzung_cent(basis)
    bestaetigt = _kuerzung_cent(basis, feld, wert)
    vorlaeufig = _kuerzung_cent(basis, feld, wert, "vorlaeufig")
    assert bestaetigt != fehlt, (
        f"KONTROLLE: {feld} bestaetigt aendert E0205508 nicht ({fehlt} Cent) -- der Fall misst nichts")
    assert vorlaeufig == fehlt, (
        f"DEFEKT: {feld} nur vorlaeufig gibt E0205508 = {vorlaeufig} Cent, ohne das Feld {fehlt} Cent "
        f"(bestaetigt {bestaetigt})")


# ---------------------------------------------------------------- HTTP /deklaration

def _req(base, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                  headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, wert):
    return {"feld_id": fld, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _vorjahr_vorschlag(fld, wert):
    """Ein VORLAEUFIGER Vorjahres-Vorschlag: store.append_event erzwingt zustand=vorlaeufig, herkunft=vorjahr,
    signal_2=None fuer schreiber import:vorjahr, und der Schreiber ist Katalog-exempt."""
    return {"feld_id": fld, "wert": wert, "zustand": "vorlaeufig",
            "herkunft": {"herkunft": "vorjahr", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "import:vorjahr", "signal": {"signal_1": None, "signal_2": None}}


_STAMM = (("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
          ("stammdaten_geburtsdatum", "05.05.1955"),
          ("stammdaten_strasse", "Musterstr."), ("stammdaten_hausnummer", "55"),
          ("stammdaten_plz", "55555"), ("stammdaten_wohnort", "Musterort"),
          ("stammdaten_keine_bankverbindung", True),
          ("stammdaten_art_est_erklaerung", True),
          ("kist_konfession", "keine"),
          ("stammdaten_steuernummer", "9181081508155"),
          ("steuerklasse", "1"), ("p36_lohnsteuer", 1200000))

_GRUND = kegel_fuer("gesamt", {
    "bruttoarbeitslohn": 6000000, "vor_an_anteil_rv": 4200000, "vor_ag_anteil_rv": 1200000,
    "veranlagung": "einzel",
    # § 33 Abs. 2 S. 1: der Bauer setzt nach Namenspolaritaet False, das waere eine Verneinung des Tatbestands.
    "agb_zwangslaeufig": True, "agb_notwendig_angemessen": True,
}) + list(_STAMM)

# Die Faelle der HTTP-Ebene: die Basis ist so gewaehlt, dass der Waechter erfuellt ist (vpf_monate_am_ort und eine
# bestaetigte Mahlzeiten-Antwort sind da), sonst sperrt /deklaration mit 409 und die Zeile stuende nie da.
HTTP_FAELLE = [f for f in FAELLE if f[0] in (
    "vpf_fruehstuecke_gestellt_anzahl", "entgelt", "tage_24h", "einziger_tage_topf",
    "vpf_tage_24h_nach_drei_monaten")]


@pytest.fixture(scope="module")
def gemessen(tmp_path_factory):
    faelle_dir = tmp_path_factory.mktemp("faelle")
    API.FAELLE = str(faelle_dir)
    audit.AUDIT_DIR = str(faelle_dir)
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    base = f"http://{srv.server_address[0]}:{srv.server_address[1]}"

    def _messe(fid, basis, monate, feld=None, wert=None, zustand=None):
        st, r = _req(base, "POST", "/fall",
                     {"fall_id": fid, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
        assert st == 201, (fid, st, r)
        events = [_laie(f, w) for f, w in _GRUND]
        events += [_laie(f, w) for f, w in {**basis, "vpf_monate_am_ort": monate}.items()]
        if feld is not None:
            events.append(_laie(feld, wert) if zustand == "bestaetigt" else _vorjahr_vorschlag(feld, wert))
        for ev in events:
            st, r = _req(base, "POST", f"/fall/{fid}/event", ev)
            assert st == 201, (fid, ev["feld_id"], st, r)
        return _req(base, "GET", f"/fall/{fid}/deklaration")

    erg = {}
    try:
        for name, feld, wert, basis in HTTP_FAELLE:
            for variante, zustand in (("fehlt", None), ("bestaetigt", "bestaetigt"), ("vorlaeufig", "vorlaeufig")):
                fid = f"vpfb_{name[:12]}_{variante}"
                erg[(name, variante)] = _messe(
                    fid, basis, 2, *(() if zustand is None else (feld, wert, zustand)))
        # Waechter-Fall: mehr als 3 Monate am Ort, Tage da, Nach-Frist-Tage des 24-h-Topfs fehlen/vorlaeufig.
        p = {**TAGE_P, **MAHLZEITEN_P, "vpf_tage_an_abreise_nach_drei_monaten": 0,
             "vpf_tage_ueber_8h_nach_drei_monaten": 0}
        feld, wert = "vpf_tage_24h_nach_drei_monaten", 2
        for variante, zustand in (("fehlt", None), ("bestaetigt", "bestaetigt"), ("vorlaeufig", "vorlaeufig")):
            erg[("waechter", variante)] = _messe(
                f"vpfb_waechter_{variante}", p, 4, *(() if zustand is None else (feld, wert, zustand)))
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()
    return erg


@pytest.mark.parametrize("name,feld,wert,basis", HTTP_FAELLE, ids=[f[0] for f in HTTP_FAELLE])
def test_deklaration_zeigt_fuer_ein_vorlaeufiges_feld_die_zeile_ohne_das_feld(gemessen, name, feld, wert, basis):
    (st_f, fehlt), (st_b, best), (st_v, vorl) = (
        gemessen[(name, "fehlt")], gemessen[(name, "bestaetigt")], gemessen[(name, "vorlaeufig")])
    assert (st_f, st_b, st_v) == (200, 200, 200), (name, st_f, st_b, st_v, fehlt, best, vorl)
    ohne = fehlt["deklaration"].get("E0205508")
    assert best["eingaben_konsistent"] is True and best["deklaration"].get("E0205508") != ohne, (
        f"KONTROLLE: {feld} bestaetigt aendert E0205508 nicht ({ohne}): {best['deklaration']}")
    assert feld in [u["feld_id"] for u in vorl["unvollstaendig"]] and vorl["eingaben_konsistent"] is False, (
        f"KONTROLLE: das vorlaeufige {feld} ist nicht als unvollstaendig gemeldet: {vorl['unvollstaendig']}")
    assert vorl["deklaration"].get("E0205508") == ohne, (
        f"DEFEKT: {feld} nur vorlaeufig, E0205508 = {vorl['deklaration'].get('E0205508')}, ohne das Feld {ohne}")


def test_waechter_sperrt_vorlaeufige_nach_frist_tage_weiter_wie_fehlende(gemessen):
    (st_f, fehlt), (st_b, best), (st_v, vorl) = (
        gemessen[("waechter", "fehlt")], gemessen[("waechter", "bestaetigt")], gemessen[("waechter", "vorlaeufig")])
    assert st_b == 200, f"KONTROLLE: bestaetigte Nach-Frist-Tage sperren: {best}"
    for st, body in ((st_f, fehlt), (st_v, vorl)):
        assert st == 409 and body["grund"] == GRUND_WAECHTER, (st, body)
    assert vorl["klartext"] == fehlt["klartext"]
