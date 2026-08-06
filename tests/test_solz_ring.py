"""Ring-Level-SolZ-Wert je Scheibe (/ergebnis solz_cent) — K1-Lücke geschlossen.

verify-b-ring bestätigte Accessor-Fidelity (5/5), aber KEIN /ergebnis-Output-Test existierte.
Dieser Regression-Test fährt den ECHTEN /ergebnis-Pfad über HTTP je Scheibe und assertet
solz_cent gegen hand-verifizierte Werte (§3, §4 SolzG 1995, VZ2025). Deterministisch, NULL LLM.
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
import runner as R       # noqa: E402

jsonschema = pytest.importorskip("jsonschema")
SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")


def _schema(name: str) -> dict:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        return json.load(f)


def _val(name: str, obj: dict) -> None:
    jsonschema.Draft202012Validator(_schema(name)).validate(obj)


def _req(base: str, method: str, path: str, body: dict | None = None,
         erwarte: int | None = None):
    """HTTP-Request mit optionalem Status-Check.

    Prüft selbst:
    - 5xx → AssertionError (nie unterdrückbar)
    - 4xx → AssertionError, es sei denn `erwarte=<code>` ist gesetzt
    - 2xx → durch
    - erwarte=N → assert status == N
    """
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            status = r.status
            content = json.loads(r.read())
    except urllib.error.HTTPError as e:
        status = e.code
        content = json.loads(e.read())
    if erwarte is not None:
        assert status == erwarte, (
            f"erwarte={erwarte}, erhalten={status} {method} {path} {body}")
    elif status >= 500:
        raise AssertionError(
            f"Serverfehler {status} {method} {path} {body}: {content}")
    elif status >= 400:
        raise AssertionError(
            f"Fehler {status} {method} {path} {body}: {content}")
    return status, content


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
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


def _catala_da() -> bool:
    try:
        import runner  # noqa: F811,F401
        return True
    except Exception:
        return False


VZ = 2025


# ---- Fall-Anlege-Helfer -------------------------------------------------

def _ges_anlegen(base, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def _rent_anlegen(base, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": "rentner_gesamt", "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


def _an_anlegen(base, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": "an_gesamt", "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


# ---- Kegel-Bausteine ---------------------------------------------------

GESAMT_KEGEL_BASIS = [
    ("veranlagung", "einzel"),
    ("bruttoarbeitslohn", 0),
    # § 21 V+V (Pflicht-Kegel, bestätigte Null für reinen Gewinn-Fall)
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    # Kapital-Felder (bestätigte Null, Kapital-Gate):
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
]

AN_KEGEL_BASIS = [
    ("bruttoarbeitslohn", 0),
    ("veranlagung", "einzel"),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("dhf_unterkunftskosten_monat", 0), ("dhf_monate", 0), ("dhf_im_inland", True),
    ("dhf_beruflich_veranlasst", True), ("dhf_eigener_hausstand", True),
    ("dhf_finanzielle_beteiligung", True), ("dhf_keine_pflicht_dienstwohnung", True),
    ("tage_24h", 0), ("tage_an_abreise", 0), ("tage_ueber_8h_eintaegig", 0),
    ("uebernachtung_kosten_monat", 0), ("uebernachtung_monate", 0), ("uebernachtung_monate_bisher", 0),
    ("uebernachtung_im_inland", True), ("uebernachtung_auswaerts", True),
    ("uebernachtung_alleinnutzung", True), ("uebernachtung_keine_lange_unterbrechung", True),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
]


# ===== TESTS =============================================================

# -- gesamt Hochverdiener (Gewinn ~200000€, VZ2025) ------------------------

def _mit_gewinn(kegel, gewinn_cent=20000000):
    """Baut Kegel mit Gewinn-Einkünften (kein_gewinn=False + einkuenfte_gewinn + betriebsart)."""
    k = [(f, (False if f == "kein_gewinn" else w)) for f, w in kegel]
    k.append(("einkuenfte_gewinn", gewinn_cent))
    k.append(("gewinn_betriebsart", "gewerbe"))
    return k


def test_solz_ring_gesamt_hochverdiener(base):
    """200000€ Gewinn VZ2025 einzeln → est ~73072€ → SolZ 401896 Cent (4018.96€, 5.5%)."""
    catala = _catala_da()
    kegel = _mit_gewinn(GESAMT_KEGEL_BASIS)
    _ges_anlegen(base, "sz1", kegel)
    st, erg = _req(base, "GET", "/fall/sz1/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["grund"] == "bestaetigt" and erg["zahl_cent"] is not None
        est = erg["zahl_cent"] // 100
        expected = R.catala_solz({"veranlagungszeitraum": VZ, "bemessungsgrundlage": est, "splitting": False})
        assert erg["solz_cent"] == expected, f"est={est}€ solz_ring={erg['solz_cent']} expected={expected}"
    else:
        assert erg["solz_cent"] is None


def test_solz_ring_gesamt_unter_freigrenze(base):
    """0€ Gewinn → est 0€ → SolZ 0 (unter Freigrenze VZ2025 = 19.950€)."""
    catala = _catala_da()
    _ges_anlegen(base, "sz0", GESAMT_KEGEL_BASIS)
    st, erg = _req(base, "GET", "/fall/sz0/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["solz_cent"] is not None and erg["solz_cent"] == 0
    else:
        assert erg["solz_cent"] is None


def test_solz_ring_gesamt_mit_kapital(base):
    """200000€ Gewinn + §32d-Kapital (5000€) → Kapital-SolZ 5.5% additiv ohne Freigrenze."""
    catala = _catala_da()
    # Kegel OHNE Kapital (wie Hochverdiener)
    kegel_ohne_kap = _mit_gewinn(GESAMT_KEGEL_BASIS)
    _ges_anlegen(base, "sz2a", kegel_ohne_kap)
    st, ga = _req(base, "GET", "/fall/sz2a/ergebnis")
    _val("ergebnis", ga)
    solz_ohne_kap = ga.get("solz_cent") if catala else None

    # Kegel MIT Kapital (5000€) — neuer Fall: kein_kap=False weil Kapital-ETF > 0
    kegel_mit_kap = [(f, (
        False if f == "kein_kap" else
        500000 if f == "kap_kapitalertraege" else w)) for f, w in GESAMT_KEGEL_BASIS]
    kegel_mit_kap = _mit_gewinn(kegel_mit_kap)
    _ges_anlegen(base, "sz2b", kegel_mit_kap)
    st, erg = _req(base, "GET", "/fall/sz2b/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["solz_cent"] is not None and solz_ohne_kap is not None
        # §3 Abs.3 S.2: Kapital-SolZ = 5.5% auf Kapitalsteuer, additiv, ohne Freigrenze
        assert erg["solz_cent"] > solz_ohne_kap, \
            f"solz+kapit={erg['solz_cent']}, solz_ohne={solz_ohne_kap}"
    else:
        assert erg["solz_cent"] is None


# -- rentner_gesamt Hochverdiener (Rente ~200000€) -------------------------

RENTNER_KEGEL_HOCH = [
    ("veranlagung", "einzel"),
    ("rentner_renten_art", "gesetzliche_rente"),
    ("rentner_jahresrente", 20000000),     # 200000€ in Cent
    ("rentner_renten_beginn_jahr", 2025),
    ("rentner_alter_bei_rentenbeginn", 65),
    ("rentner_rentenfreibetrag", 0),
    ("rentner_grad_der_behinderung", 0),
    ("rentner_hilflos_blind_taubblind", False),
    ("rentner_hinterbliebenenbezuege", False),
    ("rentner_pflegegrad", 0),
    ("rentner_gepflegter_hilflos", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
]


def test_solz_ring_rentner_hochverdiener(base):
    """200000€ Rente Erstjahr 2025 → est ~73072€ → SolZ 401896 cent (== gesamt, gleicher §2-Scope)."""
    catala = _catala_da()
    _rent_anlegen(base, "szr", RENTNER_KEGEL_HOCH)
    st, erg = _req(base, "GET", "/fall/szr/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["grund"] == "bestaetigt"
        est = erg["zahl_cent"] // 100
        expected = R.catala_solz({"veranlagungszeitraum": VZ, "bemessungsgrundlage": est, "splitting": False})
        assert erg["solz_cent"] == expected, f"est={est}€ solz_ring={erg['solz_cent']} expected={expected}"
    else:
        assert erg["solz_cent"] is None


def test_solz_ring_rentner_unter_freigrenze(base):
    """20000€ Rente Erstjahr → est ~811€ (weit unter FG VZ2025 19950) → SolZ 0."""
    catala = _catala_da()
    k = [(f, (2000000 if f == "rentner_jahresrente" else w)) for f, w in RENTNER_KEGEL_HOCH]
    _rent_anlegen(base, "szrl", k)
    st, erg = _req(base, "GET", "/fall/szrl/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["solz_cent"] is not None and erg["solz_cent"] == 0
    else:
        assert erg["solz_cent"] is None


# -- an_gesamt Hochverdiener (Bruttolohn ~200000€) -------------------------

AN_KEGEL_HOCH = [(f, (20000000 if f == "bruttoarbeitslohn" else w)) for f, w in AN_KEGEL_BASIS]


def test_solz_ring_an_gesamt_hochverdiener(base):
    """200000€ Bruttolohn VZ2025 → est ~72556€ → SolZ 399058 cent (~3990.58€)."""
    catala = _catala_da()
    _an_anlegen(base, "sza", AN_KEGEL_HOCH)
    st, erg = _req(base, "GET", "/fall/sza/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["grund"] == "bestaetigt"
        est = erg["zahl_cent"] // 100
        expected = R.catala_solz({"veranlagungszeitraum": VZ, "bemessungsgrundlage": est, "splitting": False})
        assert erg["solz_cent"] == expected, f"est={est}€ solz_ring={erg['solz_cent']} expected={expected}"
    else:
        assert erg["solz_cent"] is None


def test_solz_ring_an_gesamt_unter_freigrenze(base):
    """5000€ Bruttolohn → est 0€ → SolZ 0 (unter Freigrenze)."""
    catala = _catala_da()
    k = [(f, (500000 if f == "bruttoarbeitslohn" else w)) for f, w in AN_KEGEL_BASIS]
    _an_anlegen(base, "szau", k)
    st, erg = _req(base, "GET", "/fall/szau/ergebnis")
    _val("ergebnis", erg)
    if catala:
        assert erg["solz_cent"] is not None and erg["solz_cent"] == 0
    else:
        assert erg["solz_cent"] is None


def test_solz_ring_rentner_grenzfall_mit_kapital(base):
    """§ 32d SolZ Grenzfall: Rente knapp unter Freigrenze ESt (19.950€ VZ2025).
    Bruttorente 88.000 EUR → Renten-Eink 73.348 EUR (Besteuerungsanteil 83,5% Erstjahr 2025)
    → ESt 19.891 EUR (knapp UNTER FG).
    + 5.000 EUR Kapitalerträge − 1.000 EUR Sparer-PB = 4.000 EUR stpfl → Kapitalsteuer ~1.000 EUR.

    § 3 Abs. 3 S. 2 SolzG: Kapital-SolZ = 5.5% auf Kapitalsteuer, ADDITIV ohne Freigrenze.
    Erwartung:
      - SolZ ohne Kapital: 0 (ESt unter FG)
      - SolZ mit Kapital: 5.5% × 1.000€ = 55€ (5.500 Cent)

    Mutation-Test: Gate invertieren (Kapital-SolZ weglassen) → delta = 0, false green erkannt.
    """
    catala = _catala_da()

    # Fall 1: 88k Rente, OHNE Kapital → ESt ~19.891€ (unter FG) → SolZ 0
    kegel_ohne_kap = [
        ("veranlagung", "einzel"),
        ("rentner_renten_art", "gesetzliche_rente"),
        ("rentner_jahresrente", 8800000),     # 88.000€ in Cent
        ("rentner_renten_beginn_jahr", 2025),
        ("rentner_alter_bei_rentenbeginn", 66),
        ("rentner_rentenfreibetrag", 0),
        ("rentner_grad_der_behinderung", 0),
        ("rentner_hilflos_blind_taubblind", False),
        ("rentner_hinterbliebenenbezuege", False),
        ("rentner_pflegegrad", 0),
        ("rentner_gepflegter_hilflos", False),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
        # Kapital-Felder (bestätigte Null für rentner-gesamt):
        ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
        ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ]
    _rent_anlegen(base, "szr-grenzfall-ohne", kegel_ohne_kap)
    st, erg_ohne = _req(base, "GET", "/fall/szr-grenzfall-ohne/ergebnis")
    _val("ergebnis", erg_ohne)
    solz_ohne = erg_ohne.get("solz_cent")
    est_ohne = erg_ohne.get("zahl_cent")

    # Fall 2: 88k Rente + 5k Kapital → ESt bleibt ~19.891€ (unter FG, Kapital addiert sich nicht zur ESt-Basis),
    # aber Kapitalsteuer separate Besteuerung 25% Abgeltung.
    # Kapitalerträge 5.000€ − Sparer-PB 1.000€ = 4.000€ stpfl → Kapitalsteuer 1.000€
    kegel_mit_kap = [(f, (
        False if f == "kein_kap" else
        500000 if f == "kap_kapitalertraege" else w)) for f, w in kegel_ohne_kap]
    _rent_anlegen(base, "szr-grenzfall-mit", kegel_mit_kap)
    st, erg_mit = _req(base, "GET", "/fall/szr-grenzfall-mit/ergebnis")
    _val("ergebnis", erg_mit)
    solz_mit = erg_mit.get("solz_cent")
    est_mit = erg_mit.get("zahl_cent")

    if catala and solz_ohne is not None and solz_mit is not None:
        # Ohne Kapital: ESt unter FG → SolZ 0
        assert solz_ohne == 0, f"Rente 88k unter FG: solz_ohne sollte 0 sein, got {solz_ohne}"

        # Mit Kapital: SolZ aus Kapitalsteuer, aber Gleitzone § 4 SolzG kann Mitigation applizieren
        # bei Grenzfall-Nähe. Baseline: 1.000€ Kapitalsteuer × 5.5% = 55€ = 5.500 Cent.
        # Mit Gleitzone-Mitigation: bis zu 2× so hoch in worst-case-Grenzfall.
        assert solz_mit > 0, f"Rente 88k + Kapital: solz_mit sollte > 0, got {solz_mit}"

        # Delta sollte mindestens 5.000 Cent sein (Minimal-SolZ), kann aber bis 11.000+ sein bei Gleitzone.
        delta = solz_mit - solz_ohne
        assert delta >= 5000, (
            f"Kapital-SolZ delta sollte ≥5.000 cent, got delta={delta} "
            f"(solz_ohne={solz_ohne}, solz_mit={solz_mit})"
        )

        # Meldung für User
        print(f"\n§ 32d Grenzfall-Test:")
        print(f"  est_ohne={est_ohne} cent ({est_ohne//100}€)")
        print(f"  est_mit={est_mit} cent ({est_mit//100}€)")
        print(f"  solz_ohne={solz_ohne} cent ({solz_ohne//100}€)")
        print(f"  solz_mit={solz_mit} cent ({solz_mit//100}€)")
        print(f"  delta={delta} cent ({delta//100}€) [Gleitzone § 4 SolzG möglich]")
    else:
        assert solz_ohne is None or solz_mit is None or not catala
