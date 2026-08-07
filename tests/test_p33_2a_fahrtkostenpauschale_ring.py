"""Ring-Level §33 Abs.2a Fahrtkostenpauschale (Person A): Pauschale IN agb-Slot.

S.3: 900 EUR für GdB >= 80 oder GdB >= 70 + Merkzeichen G (fahrtkosten_pausch_gdb80_oder_70g).
S.4: 4.500 EUR für aG/Bl/TBl/H (fahrtkosten_pausch_ag_bl_tbl_h).
S.5: 4.500 schließt 900 aus (nicht additiv).
S.7: Pauschale geht IN den aussergewoehnliche_belastungen-Slot von catala_p33_agb,
     NICHT in `ausserg` (PB-Pfad).

Zwei kritische Tests:
  - 900er-Pauschale bei GdE 40.000 + 0 agB → 0 EUR Wirkung (Gesetz, kein Bug).
  - 4.500er-Pauschale bei GdE 40.000 + 0 agB → 2.254 EUR Abzug.

NULL LLM.
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

jsonschema = pytest.importorskip("jsonschema")
SCHEMA_DIR = os.path.join(ROOT, "produkt", "haut", "api_schema")


def _val(name: str, obj: dict) -> None:
    with open(os.path.join(SCHEMA_DIR, f"{name}.json"), encoding="utf-8") as f:
        jsonschema.Draft202012Validator(json.load(f)).validate(obj)


def _req(base: str, method: str, path: str, body: dict | None = None,
         erwarte: int | None = None):
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


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


VZ = 2025


def _anlegen(base, scheibe, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201, f"{feld}={wert} abgelehnt: {st}"


def _zahl(base, scheibe, fid, kegel):
    _anlegen(base, scheibe, fid, kegel)
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt", f"grund={erg['grund']} offen={erg.get('offen')}"
    return erg["zahl_cent"]


# ---- Kegel-Bausteine (gesamt) ----
# Einzelveranlagung, 40k€ Brutto, keine agB, keine Vorsorge, keine Gewinne.
# GdE ~40k → zumutbare Belastung ~2.400€ (Einzel, 0 Kinder, 6%).
# 900er-Pauschale: 900 - 2.400 = 0 → 0 EUR Abzug (Gesetz, kein Bug).
# 4.500er-Pauschale: 4.500 - 2.400 = 2.100 → 2.100 EUR agB-Wirkung.
# Gemessene Deltas: 900er=0, 4500er=72300ct, BEIDE=72300ct (deckungsgleich S.5).
GESAMT_KEGEL_BASIS = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 4000000),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
    ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("einkuenfte_gewinn", 0), ("gewinn_betriebsart", "gewerbe"),
    ("fam_anzahl_kinder", 0),
]

# 900er-Pauschale: GdB >= 80 oder GdB >= 70 + Merkzeichen G
FK_900ER = [
    ("fahrtkosten_pausch_gdb80_oder_70g", True),
    ("fahrtkosten_pausch_ag_bl_tbl_h", False),
]

# 4.500er-Pauschale: aG/Bl/TBl/H
FK_4500ER = [
    ("fahrtkosten_pausch_gdb80_oder_70g", False),
    ("fahrtkosten_pausch_ag_bl_tbl_h", True),
]

# Beide True → S.5: 4.500 schließt 900 aus → nur 4.500
FK_BEIDE = [
    ("fahrtkosten_pausch_gdb80_oder_70g", True),
    ("fahrtkosten_pausch_ag_bl_tbl_h", True),
]

# ---- Kegel rentner_gesamt ----
# Einzelveranlagung, 200k€ Rente, keine agB, keine Vorsorge.
# GdE ~200k → zumutbare Belastung 12.000€ (6%) → frisst 4.500 komplett.
RENTNER_KEGEL_BASIS = [
    ("veranlagung", "einzel"),
    ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 4000000),
    ("rentner_renten_beginn_jahr", 2025), ("rentner_alter_bei_rentenbeginn", 65),
    ("rentner_rentenfreibetrag", 0), ("rentner_grad_der_behinderung", 0),
    ("rentner_hilflos_blind_taubblind", False), ("rentner_hinterbliebenenbezuege", False),
    ("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
    ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("fam_anzahl_kinder", 0),
]


# ===== TESTS =============================================================

def test_p33_2a_fahrtkosten_900er_hat_keine_wirkung_bei_gde_40k(base):
    """900er-Pauschale bei GdE 40.000 + 0 agB → 0 EUR Wirkung (Gesetz, kein Bug).

    Die Pauschale wird mit der zumutbaren Belastung verrechnet (§ 33 Abs. 2a S. 7).
    Bei GdE 40.000 (Einzel, 0 Kinder) beträgt die zumutbare Belastung 2.246 EUR.
    Die 900er-Pauschale wird davon vollständig aufgezehrt → 0 EUR Abzug.
    Der Test dokumentiert, dass dies KEIN Bug ist — es ist die gesetzliche Folge von S.7.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "fk900_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "fk900_mit", GESAMT_KEGEL_BASIS + FK_900ER)
    delta = baseline - mit
    assert delta == 0, (
        f"baseline={baseline} mit={mit} Δ={delta} — "
        "900er-Pauschale bei GdE 40.000 + 0 agB muss 0 EUR Wirkung haben "
        "(Gesetz §33 Abs.2a S.7, kein Bug)")


def test_p33_2a_fahrtkosten_4500er_hat_wirkung_bei_gde_40k(base):
    """4.500er-Pauschale bei GdE 40.000 + 0 agB → 72300ct (= 723 EUR) Abzug.

    Exakter Wert, gemessen 2026-08-06. Bei Zugrundeliegendem ändert sich der
    Wert — der Test bricht dann auf, statt im Toleranzfenster zu schlafen.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "fk45_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "fk45_mit", GESAMT_KEGEL_BASIS + FK_4500ER)
    delta = baseline - mit
    assert delta == 72300, (
        f"baseline={baseline} mit={mit} Δ={delta} — "
        "erwartet 72300 (723 EUR), 4.500er-Pauschale bei GdE 40k")


def test_p33_2a_fahrtkosten_beide_schliesst_900_aus(base):
    """S.5: beide Kz True → delta deckungsgleich mit NUR-4500er (nicht additiv).

    Die Delta-Berechnung ist identisch zu test_p33_2a_fahrtkosten_4500er_hat_wirkung_bei_gde_40k.
    Ist S.5 verletzt (4500+900), wäre delta grosser — der Test faengt es.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "gesamt", "fkbe_base", GESAMT_KEGEL_BASIS)
    mit = _zahl(base, "gesamt", "fkbe_mit", GESAMT_KEGEL_BASIS + FK_BEIDE)
    delta = baseline - mit
    assert delta == 72300, (
        f"baseline={baseline} mit={mit} Δ={delta} — "
        "S.5: BEIDE muss exakt 72300 ergeben (wie NUR-4500er), nicht additiv (S.5 verletzt?)")


def test_p33_2a_fahrtkosten_rentner_900er_keine_wirkung(base):
    """Selbe 0-Wirkung für rentner_gesamt-Scheibe."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "rentner_gesamt", "fk900r_base", RENTNER_KEGEL_BASIS)
    mit = _zahl(base, "rentner_gesamt", "fk900r_mit", RENTNER_KEGEL_BASIS + FK_900ER)
    delta = baseline - mit
    assert delta == 0, f"Δ={delta} — 900er-Pauschale bei GdE 40.000 + 0 agB muss 0 EUR Wirkung haben"


def test_p33_2a_fahrtkosten_rentner_4500er_hat_wirkung(base):
    """Selbe 4.500-Wirkung für rentner_gesamt-Scheibe. Exakter Wert 77200ct."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline = _zahl(base, "rentner_gesamt", "fk45r_base", RENTNER_KEGEL_BASIS)
    mit = _zahl(base, "rentner_gesamt", "fk45r_mit", RENTNER_KEGEL_BASIS + FK_4500ER)
    delta = baseline - mit
    assert delta == 77200, (
        f"baseline={baseline} mit={mit} Δ={delta} — "
        "rentner 4.500er-Pauschale: erwartet 77200 (772 EUR)")


def test_p33_2a_fahrtkosten_accessor_s5_nicht_additiv(base):
    """Accessor-Unit: catala_p33_2a_fahrtkostenpauschale(beide) == 4500 (exakt).

    Prüft S.5 auf Accessor-Ebene, OHNE Ring-Integrationstest-Latenz.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    import runner  # noqa: F811
    both = {"veranlagungszeitraum": VZ, "hat_gdb80_oder_70g": True, "hat_ag_bl_tbl_h": True}
    only_4500 = {"veranlagungszeitraum": VZ, "hat_gdb80_oder_70g": False, "hat_ag_bl_tbl_h": True}
    assert runner.catala_p33_2a_fahrtkostenpauschale(both) == 4500, (
        "S.5 verletzt: beide=True muss 4500 liefern (nicht 5400)")
    assert runner.catala_p33_2a_fahrtkostenpauschale(both) == runner.catala_p33_2a_fahrtkostenpauschale(only_4500), (
        "S.5 verletzt: beide=True muss gleich NUR-4500er sein")