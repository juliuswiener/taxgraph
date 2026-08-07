"""Ring-Level §33b Abs.5 S.4: bei Kind-PB-Übertragung sind die behinderungsbedingten
Aufwendungen nicht zusätzlich als agB nach §33 abziehbar.

  S.4: "In diesen Fällen besteht für Aufwendungen, für die der Behinderten-Pauschbetrag
        gilt, kein Anspruch auf eine Steuerermäßigung nach § 33"

"In diesen Fällen" = die Übertragung nach S.1. Ohne Übertragung bleibt agb_aufwendungen
unberührt — der eigene PB des Steuerpflichtigen fällt unter Abs.1 S.1 (Wahlrecht, offen).

Differential gegen denselben Kegel ohne behinderungsbedingte_aufwendungen. NULL LLM.
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


def _zahl(base, scheibe, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201, f"{feld}={wert} abgelehnt: {st}"
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt", f"grund={erg['grund']} offen={erg.get('offen')}"
    return erg["zahl_cent"]


# Hochverdiener 200k, einzel — 45%-Zone, damit die Kürzung eine sichtbare Steuerwirkung hat.
# agb_aufwendungen 3.000.000ct = 30.000 EUR, davon 300.000ct = 3.000 EUR behinderungsbedingt.
# Die 30.000 sind kein Zufall: bei 200k GdE liegt die zumutbare Belastung nach § 33 Abs. 3
# im fuenfstelligen Bereich. Mit kleinerem Betrag ist der agB-Abzug ohnehin 0, dann kuerzt
# S.4 an einer Null und der Test misst nichts (erste Messung 2026-08-07: Δ war exakt 0).
KEGEL_BASIS = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 20000000),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
    ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("einkuenfte_gewinn", 20000000), ("gewinn_betriebsart", "gewerbe"),
    ("fam_anzahl_kinder", 1),
    ("agb_aufwendungen", 3000000),
]

# Kind mit GdB 100, PB wird uebertragen (Antrag + Kind-nimmt-nicht + idnr kumulativ).
UEBERTRAGUNG = [
    ("kind_idnr", "11111111111"),
    ("kind_behinderten_pb_antrag", True), ("kind_pb_nicht_selbst_genutzt", True),
    ("kind_grad_der_behinderung", 100), ("kind_hilflos_blind_taubblind", False),
    ("kind_hinterbliebenen_uebertragung", False),
]

# Dasselbe Kind, aber Antrag=False -> keine Uebertragung, S.4 greift nicht.
KEINE_UEBERTRAGUNG = [
    ("kind_idnr", "11111111111"),
    ("kind_behinderten_pb_antrag", False), ("kind_pb_nicht_selbst_genutzt", True),
    ("kind_grad_der_behinderung", 100), ("kind_hilflos_blind_taubblind", False),
    ("kind_hinterbliebenen_uebertragung", False),
]

BEHINDERUNGSBEDINGT_3000 = [("behinderungsbedingte_aufwendungen", 300000)]

# Gemessen 2026-08-07: 3000 EUR weniger agB in der 45%-Zone.
S4_DELTA_EXACT = 135000

# rentner: 20k Rente statt 200k Lohn -> flachere Tarifzone (42 % statt 45 %).
# Gemessen 2026-08-07: 4790200 -> 4916200.
S4_DELTA_RENTNER = 126000


def test_p33b_abs5_s4_kuerzt_bei_uebertragung(base):
    """Kind-PB übertragen + behinderungsbedingte Aufwendungen → agB wird gekürzt.

    Vor dem Fix waren beide Abzüge nebeneinander wirksam (Doppelabzug, under-tax).
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne = _zahl(base, "gesamt", "s4_ohne", KEGEL_BASIS + UEBERTRAGUNG)
    mit = _zahl(base, "gesamt", "s4_mit", KEGEL_BASIS + UEBERTRAGUNG + BEHINDERUNGSBEDINGT_3000)
    delta = mit - ohne
    assert delta == S4_DELTA_EXACT, (
        f"ohne={ohne} mit={mit} Δ={delta} ≠ {S4_DELTA_EXACT} — "
        f"S.4-Kürzung fehlt oder trifft den falschen Betrag")


def test_p33b_abs5_s4_ohne_uebertragung_unberuehrt(base):
    """Keine Übertragung → agB bleibt voll abziehbar.

    "In diesen Fällen" (S.4) knüpft an die Übertragung nach S.1 an. Ohne sie darf das
    Feld nichts bewirken, sonst wäre es Over-tax.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne = _zahl(base, "gesamt", "s4_kein_ohne", KEGEL_BASIS + KEINE_UEBERTRAGUNG)
    mit = _zahl(base, "gesamt", "s4_kein_mit",
                KEGEL_BASIS + KEINE_UEBERTRAGUNG + BEHINDERUNGSBEDINGT_3000)
    assert mit == ohne, (
        f"ohne={ohne} mit={mit} — behinderungsbedingte_aufwendungen wirkt ohne "
        f"Übertragung, das ist Over-tax")


def test_p33b_abs5_s4_kuerzung_nicht_negativ(base):
    """behinderungsbedingt > agb_aufwendungen → Ergebnis wie bei agB 0, keine Steuererhöhung.

    Achtung, was dieser Test NICHT leistet: das `max(0, ...)` in api.py ist Redundanz.
    catala_p33_agb klemmt negative Eingaben selbst auf 0 (gemessen 2026-08-07:
    agB=-200 → 0). Entfernt man `max(0, ...)`, bleibt dieser Test grün. Er sichert
    also das Verhalten der Naht, nicht die eine Zeile — falls die Catala-Regel ihre
    Klemmung je verliert, schlägt er hier an.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    # agb 30.000 EUR, behinderungsbedingt 50.000 EUR
    voll = _zahl(base, "gesamt", "s4_ueber",
                 KEGEL_BASIS + UEBERTRAGUNG + [("behinderungsbedingte_aufwendungen", 5000000)])
    # Referenz: agb komplett weg (0), sonst identisch
    kein_agb = [(f, w) for f, w in KEGEL_BASIS if f != "agb_aufwendungen"] + [("agb_aufwendungen", 0)]
    null = _zahl(base, "gesamt", "s4_null_agb", kein_agb + UEBERTRAGUNG)
    assert voll == null, (
        f"ueberkuerzt={voll} agb_null={null} — Kürzung unter 0 gelaufen")


# rentner_gesamt geht durch DENSELBEN Helfer (_shared_steuer_sonder_agb, api.py:387).
# Dieser Test belegt das an der Zahl statt es dem Schnittplan zu glauben.
RENTNER_KEGEL = [
    ("veranlagung", "einzel"),
    ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 20000000),
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
    ("fam_anzahl_kinder", 1),
    ("agb_aufwendungen", 3000000),
]


def test_p33b_abs5_s4_kuerzt_auch_im_rentner_zweig(base):
    """rentner_gesamt kürzt ebenfalls — beide Zweige teilen sich _shared_steuer_sonder_agb."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne = _zahl(base, "rentner_gesamt", "s4_r_ohne", RENTNER_KEGEL + UEBERTRAGUNG)
    mit = _zahl(base, "rentner_gesamt", "s4_r_mit",
                RENTNER_KEGEL + UEBERTRAGUNG + BEHINDERUNGSBEDINGT_3000)
    delta = mit - ohne
    assert delta > 0, (
        f"ohne={ohne} mit={mit} Δ={delta} — im rentner-Zweig kürzt S.4 nicht")
    assert delta == S4_DELTA_RENTNER, (
        f"ohne={ohne} mit={mit} Δ={delta} ≠ {S4_DELTA_RENTNER}")
