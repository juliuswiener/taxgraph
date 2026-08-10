"""Ring-Level §33b Abs.1 S.1: eigener Behinderten-Pauschbetrag "anstelle" einer
Steuerermäßigung nach §33 für dieselben Aufwendungen — Wahlrecht des Steuerpflichtigen.

  S.1: "Behinderte Menschen ... können [...] anstelle einer Steuerermäßigung nach §33
        einen Pauschbetrag [...] geltend machen"

"Anstelle" = XOR, nicht beides. Wer den eigenen GdB-Pauschbetrag hat UND behinderungs-
bedingte Aufwendungen als agB geltend machen könnte, MUSS wählen — es gibt keinen
over-tax-sicheren Default (kleiner Aufwand: PB günstiger; großer Aufwand: Einzelnachweis
günstiger). Ohne Antwort: Sperrgrund.

Stufe 2a (Abs.5 S.4, Kind-PB-Übertragung) hat Vorrang: "Abs.5 zuerst, dann Abs.1"
(BACKLOG p33b-stufe-2). Ist eine Kind-Übertragung aktiv, kürzt bereits diese die agb —
das eigene-PB-Wahlrecht wird dort gar nicht erst befragt (siehe Präzedenz-Test unten).

Differential gegen denselben Kegel ohne GdB / ohne behinderungsbedingte Aufwendungen.
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


def _zahl(base, scheibe, fid, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": VZ, "fall_id": fid})
    assert st == 201
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201, f"{feld}={wert} abgelehnt: {st}"
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    return erg


def _bestaetigt(base, scheibe, fid, kegel):
    erg = _zahl(base, scheibe, fid, kegel)
    assert erg["grund"] == "bestaetigt", f"grund={erg['grund']} offen={erg.get('offen')}"
    return erg["zahl_cent"]


# Hochverdiener 200k, einzel — 45%-Zone (siehe test_p33b_abs5_s4_ring.py: Begründung dort).
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

# eigener GdB 100 -> eigener Pauschbetrag 2.840 EUR (params/2025/behinderten_pauschbetrag_p33b.yaml)
EIGENER_GDB100 = [("rentner_grad_der_behinderung", 100), ("rentner_hilflos_blind_taubblind", False)]
BEHINDERUNGSBEDINGT_3000 = [("behinderungsbedingte_aufwendungen", 300000)]

UEBERTRAGUNG = [
    ("kind_idnr", "11111111111"),
    ("kind_behinderten_pb_antrag", True), ("kind_pb_nicht_selbst_genutzt", True),
    ("kind_grad_der_behinderung", 100), ("kind_hilflos_blind_taubblind", False),
    ("kind_hinterbliebenen_uebertragung", False),
]

# Gemessen 2026-08-10: Wahlrecht=True kürzt agb um den behinderungsbedingten Anteil,
# derselbe Betrag/dieselbe Zone wie Abs.5 S.4 (strukturell identische Kürzungsstelle).
WAHLRECHT_TRUE_DELTA_GESAMT = 135000
WAHLRECHT_TRUE_DELTA_RENTNER = 126000


def test_p33b_abs1_s1_wahlrecht_true_kuerzt_agb_gesamt(base):
    """Wahlrecht=True (PB gewählt) → agb um den behinderungsbedingten Anteil gekürzt."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne_bb = _bestaetigt(base, "gesamt", "w_true_ohne",
                           KEGEL_BASIS + EIGENER_GDB100)
    mit_bb = _bestaetigt(base, "gesamt", "w_true_mit",
                          KEGEL_BASIS + EIGENER_GDB100 + BEHINDERUNGSBEDINGT_3000 +
                          [("behinderungsbedingte_aufwendungen_wahlrecht_pb", True)])
    delta = mit_bb - ohne_bb
    assert delta == WAHLRECHT_TRUE_DELTA_GESAMT, (
        f"ohne={ohne_bb} mit={mit_bb} Δ={delta} ≠ {WAHLRECHT_TRUE_DELTA_GESAMT} — "
        f"Wahlrecht=True kürzt agb nicht oder falscher Betrag")


def test_p33b_abs1_s1_wahlrecht_true_kuerzt_agb_rentner(base):
    """rentner_gesamt teilt sich _shared_steuer_sonder_agb — dasselbe muss dort gelten."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne_bb = _bestaetigt(base, "rentner_gesamt", "w_true_r_ohne",
                           _rentner_kegel(gdb=100))
    mit_bb = _bestaetigt(base, "rentner_gesamt", "w_true_r_mit",
                          _rentner_kegel(gdb=100) + BEHINDERUNGSBEDINGT_3000 +
                          [("behinderungsbedingte_aufwendungen_wahlrecht_pb", True)])
    delta = mit_bb - ohne_bb
    assert delta == WAHLRECHT_TRUE_DELTA_RENTNER, (
        f"ohne={ohne_bb} mit={mit_bb} Δ={delta} ≠ {WAHLRECHT_TRUE_DELTA_RENTNER}")


def test_p33b_abs1_s1_wahlrecht_false_entfernt_eigenen_pb_gesamt(base):
    """Wahlrecht=False (Einzelnachweis/agb gewählt) → eigener PB entfällt, agb bleibt voll.

    Muss exakt auf die Zahl OHNE jeden GdB zurückfallen: Wahlrecht=False heißt "gar
    keinen PB", also darf am Ergebnis nichts vom GdB übrig bleiben.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline_ohne_gdb = _bestaetigt(base, "gesamt", "w_false_baseline", KEGEL_BASIS)
    mit_false = _bestaetigt(base, "gesamt", "w_false_mit",
                             KEGEL_BASIS + EIGENER_GDB100 + BEHINDERUNGSBEDINGT_3000 +
                             [("behinderungsbedingte_aufwendungen_wahlrecht_pb", False)])
    assert mit_false == baseline_ohne_gdb, (
        f"baseline(kein GdB)={baseline_ohne_gdb} wahlrecht_false={mit_false} — "
        f"eigener PB wurde nicht (oder falsch) aus ausserg entfernt")


def test_p33b_abs1_s1_wahlrecht_false_entfernt_eigenen_pb_rentner(base):
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    baseline_ohne_gdb = _bestaetigt(base, "rentner_gesamt", "w_false_r_baseline",
                                     _rentner_kegel(gdb=0))
    mit_false = _bestaetigt(base, "rentner_gesamt", "w_false_r_mit",
                             _rentner_kegel(gdb=100) + BEHINDERUNGSBEDINGT_3000 +
                             [("behinderungsbedingte_aufwendungen_wahlrecht_pb", False)])
    assert mit_false == baseline_ohne_gdb, (
        f"baseline(kein GdB)={baseline_ohne_gdb} wahlrecht_false={mit_false}")


def test_p33b_abs1_s1_sperrgrund_ohne_antwort(base):
    """Eigener PB + behinderungsbedingte Aufwendungen, Wahlrecht unbeantwortet → Sperrgrund."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    erg = _zahl(base, "gesamt", "w_sperr_offen",
                KEGEL_BASIS + EIGENER_GDB100 + BEHINDERUNGSBEDINGT_3000)
    assert erg["grund"] == "behinderungsbedingte_aufwendungen_wahlrecht_offen", (
        f"grund={erg['grund']} — Sperrgrund fehlt trotz unbeantwortetem Wahlrecht")
    assert erg["zahl_cent"] is None


def test_p33b_abs1_s1_kein_gdb_unberuehrt(base):
    """GEGENTEST (Pflicht): kein eigener GdB → Wahlrecht nie relevant, kein Sperrgrund.

    Muss identisch zur Zahl ganz ohne behinderungsbedingte Aufwendungen sein — sonst
    wirkt das Feld ohne GdB und das ist Over-tax (Gate-Polarität-Falle).
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne_bb = _bestaetigt(base, "gesamt", "w_kein_gdb_ohne_bb", KEGEL_BASIS)
    mit_bb_unbeantwortet = _bestaetigt(base, "gesamt", "w_kein_gdb_mit_bb",
                                        KEGEL_BASIS + BEHINDERUNGSBEDINGT_3000)
    assert mit_bb_unbeantwortet == ohne_bb, (
        f"ohne_bb={ohne_bb} mit_bb={mit_bb_unbeantwortet} — behinderungsbedingte "
        f"Aufwendungen wirken ohne eigenen GdB, Wahlrecht-Sperrgrund müsste aber "
        f"NIE feuern (over-tax bzw. fälschliche Blockade)")


def test_p33b_abs1_s1_gdb_ohne_aufwendungen_kein_sperrgrund(base):
    """Eigener GdB, aber behinderungsbedingte_aufwendungen=0 → nichts zum Wählen, kein Sperrgrund."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    erg = _bestaetigt(base, "gesamt", "w_gdb_kein_bb", KEGEL_BASIS + EIGENER_GDB100)
    assert isinstance(erg, int)  # bestaetigt, PB unconditional gewährt


def test_p33b_abs1_s1_praezedenz_kind_uebertragung_hat_vorrang(base):
    """Eigener GdB + gleichzeitig Kind-PB-Übertragung (Abs.5 S.4) + BB, Wahlrecht offen.

    "Abs.5 zuerst, dann Abs.1": die Kind-Übertragung kürzt die agb bereits automatisch;
    das eigene-PB-Wahlrecht wird in diesem Überschneidungsfall gar nicht erst befragt,
    Sperrgrund darf NICHT feuern.
    """
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ohne_bb = _bestaetigt(base, "gesamt", "w_praez_ohne_bb",
                           KEGEL_BASIS + EIGENER_GDB100 + UEBERTRAGUNG)
    mit_bb = _bestaetigt(base, "gesamt", "w_praez_mit_bb",
                          KEGEL_BASIS + EIGENER_GDB100 + UEBERTRAGUNG + BEHINDERUNGSBEDINGT_3000)
    delta = mit_bb - ohne_bb
    assert delta == WAHLRECHT_TRUE_DELTA_GESAMT, (
        f"ohne={ohne_bb} mit={mit_bb} Δ={delta} ≠ {WAHLRECHT_TRUE_DELTA_GESAMT} — "
        f"Präzedenz falsch: Kind-Übertragung sollte identisch zu Abs.5-S.4-Kürzung "
        f"greifen, unabhängig vom (unbeantworteten) eigenen Wahlrecht")


def _rentner_kegel(gdb: int):
    return [
        ("veranlagung", "einzel"),
        ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 20000000),
        ("rentner_renten_beginn_jahr", 2025), ("rentner_alter_bei_rentenbeginn", 65),
        ("rentner_rentenfreibetrag", 0), ("rentner_grad_der_behinderung", gdb),
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
