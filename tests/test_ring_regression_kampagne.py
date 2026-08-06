"""Ring-Level-Regression-Tests für Kampagnen-Fronten: §34c, §35-Mitu, Gap-A/B, §23, §33a.
K1-Lehre: Accessor-grün ≠ Ring emittiert korrekt. Jeder Test fährt den ECHTEN /ergebnis-Pfad
und assertet den WERT je Scheibe. Deterministisch, NULL LLM.
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


def _req(base: str, method: str, path: str, body: dict | None = None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


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
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
]

RENTNER_KEGEL_HOCH = [
    ("veranlagung", "einzel"),
    ("rentner_renten_art", "gesetzliche_rente"),
    ("rentner_jahresrente", 20000000),
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

AN_KEGEL_HOCH = [
    ("bruttoarbeitslohn", 20000000),
    ("veranlagung", "einzel"),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("dhf_unterkunftskosten_monat", 0), ("dhf_monate", 0), ("dhf_im_inland", True),
    ("dhf_beruflich_veranlasst", True), ("dhf_eigener_hausstand", True),
    ("dhf_finanzielle_beteiligung", True), ("dhf_keine_pflicht_dienstwohnung", True),
    ("tage_24h", 0), ("tage_an_abreise", 0), ("tage_ueber_8h_eintaegig", 0),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("fam_anzahl_kinder", 0), ("verlustvortrag_bestand", 0),
]


def _mit_gewinn(kegel, gewinn_cent=20000000):
    """Baut Kegel mit Gewinn-Einkünften (kein_gewinn=False + einkuenfte_gewinn + gewerbe)."""
    k = [(f, (False if f == "kein_gewinn" else w)) for f, w in kegel]
    k.append(("einkuenfte_gewinn", gewinn_cent))
    k.append(("gewinn_betriebsart", "gewerbe"))
    return k


def _mit_mitu(kegel, gewinnanteil_cent=20000000):
    """Baut Kegel mit Mitunternehmer (kein_gewinn=False + gewinnanteil + selbstaendig)."""
    k = [(f, (False if f == "kein_gewinn" else w)) for f, w in kegel]
    k.append(("gewinnanteil", gewinnanteil_cent))
    k.append(("gewinn_betriebsart", "selbstaendig"))
    return k


# ===== §34c DBA-ANRECHNUNG ==============================================

def test_p34c_dba_senkt_steuer_gesamt(base):
    """200k Gewinn + DBA 3000€ credit → tax um 3000€ niedriger (gezahlt < Höchstbetrag).
    Verifiziert: anzurechnende_auslaendische_steuern fließt in festzusetzende_est_gesamt."""
    catala = _catala_da()
    kegel_ohne = _mit_gewinn(GESAMT_KEGEL_BASIS)
    _ges_anlegen(base, "dba1", kegel_ohne)
    st, ohne = _req(base, "GET", "/fall/dba1/ergebnis")
    _val("ergebnis", ohne)

    kegel_mit = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_mit.append(("dba_gezahlte_auslaendische_steuer", 300000))    # 3000€
    kegel_mit.append(("dba_auslaendische_einkuenfte", 4000000))        # 40000€
    _ges_anlegen(base, "dba2", kegel_mit)
    st, mit = _req(base, "GET", "/fall/dba2/ergebnis")
    _val("ergebnis", mit)

    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        # HB = tarifliche * 40000 / zve ≈ 73072*40000/200000=14614 > 3000 → credit=3000
        assert delta == 300000, f"DBA delta={delta} ≠ 300000 cent (3000€)"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


def test_p34c_dba_senkt_steuer_rentner(base):
    """200k Rente + DBA 3000€ credit → rentner-tax um 3000€ niedriger.
    Verifiziert: DBA-Anrechnung im rentner_gesamt-Ring (1:1 gesamt-Präzedenz)."""
    catala = _catala_da()
    _rent_anlegen(base, "dbr1", RENTNER_KEGEL_HOCH)
    st, ohne = _req(base, "GET", "/fall/dbr1/ergebnis")
    _val("ergebnis", ohne)

    kegel_mit = list(RENTNER_KEGEL_HOCH) + [
        ("dba_gezahlte_auslaendische_steuer", 300000),
        ("dba_auslaendische_einkuenfte", 4000000)]
    _rent_anlegen(base, "dbr2", kegel_mit)
    st, mit = _req(base, "GET", "/fall/dbr2/ergebnis")
    _val("ergebnis", mit)

    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        assert delta == 300000, f"DBA rentner delta={delta} ≠ 300000 cent"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


# ===== §34c Abs.2 ABZUG STATT ANRECHNUNG (Wahlrecht, Mutual-Exclusion) ==================
# Abs.2: „Statt der Anrechnung (Absatz 1) ist die ausländische Steuer auf Antrag bei der Ermittlung
# der Einkünfte abzuziehen …" — statt Anrechnung (Steuergutschrift, flach ×100) wird die ausländische
# Steuer von der Bemessungsgrundlage abgezogen (aggregiert-GdE → PROGRESSIV). MUTUAL-EXCLUSION:
# Antrag=True ⟹ KEINE Anrechnung mehr (sonst Doppel-Relief = Under-tax, K2). Fail-closed: Flag absent
# → Abs.1-Anrechnung (test_p34c_dba_senkt_steuer_gesamt oben ist die Flag-absent-Regression).

DBA_ABS2 = [("dba_gezahlte_auslaendische_steuer", 300000),   # 3000€ ausländische Steuer
            ("dba_auslaendische_einkuenfte", 4000000),       # 40000€ ausländische Einkünfte
            ("dba_abzug_statt_anrechnung", True)]             # Antrag §34c Abs.2
# Progressiver Abzug 3000€ × 42% (~200k zvE, § 32a-Zone 68.481–277.825) ≈ 1260€. Anrechnung (Abs.1)
# wäre flach 3000€. Band großzügig gegen zvE-/§22-Besteuerungsanteil-Rundung.
ABS2_MIN = 100000   # 1000€
ABS2_MAX = 150000   # 1500€


def test_p34c_2_abzug_statt_anrechnung_gesamt(base):
    """Abs.2-Antrag: die ausländische Steuer (3000€) mindert die Bemessungsgrundlage progressiv
    (~1260€ bei 42%) statt als Anrechnung flach (3000€). Beweist: Feld erreichbar (POST 201, nicht
    400) + Base-Reduction-Durchgriff am echten /ergebnis."""
    catala = _catala_da()
    _ges_anlegen(base, "abz1", _mit_gewinn(GESAMT_KEGEL_BASIS))
    st, ohne = _req(base, "GET", "/fall/abz1/ergebnis")
    _val("ergebnis", ohne)

    _ges_anlegen(base, "abz2", _mit_gewinn(GESAMT_KEGEL_BASIS) + DBA_ABS2)
    st, mit = _req(base, "GET", "/fall/abz2/ergebnis")
    _val("ergebnis", mit)

    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        assert ABS2_MIN <= delta <= ABS2_MAX, f"Abs.2 Abzug delta={delta} nicht in [{ABS2_MIN},{ABS2_MAX}]"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


def test_p34c_2_mutual_exclusion_kein_doppel_relief_gesamt(base):
    """MUTUAL-EXCLUSION: bei Abs.2-Antrag darf die Abs.1-Anrechnung NICHT zusätzlich greifen.
    Abs.1 (nur Anrechnung) senkt flach um 3000€; Abs.2 (Antrag) senkt nur progressiv (~1260€).
    Wäre die Anrechnung fälschlich ADDITIV, läge Abs.2-delta > Abs.1-delta (Doppel-Relief = Under-tax).
    Assert: Abs.2-delta im progressiven Band UND strikt < Abs.1-delta."""
    catala = _catala_da()
    kegel_abs1 = _mit_gewinn(GESAMT_KEGEL_BASIS) + [
        ("dba_gezahlte_auslaendische_steuer", 300000),
        ("dba_auslaendische_einkuenfte", 4000000)]           # OHNE Antrag → Abs.1-Anrechnung
    _ges_anlegen(base, "me_abs1", kegel_abs1)
    st, r_abs1 = _req(base, "GET", "/fall/me_abs1/ergebnis")
    _val("ergebnis", r_abs1)

    _ges_anlegen(base, "me_abs2", _mit_gewinn(GESAMT_KEGEL_BASIS) + DBA_ABS2)
    st, r_abs2 = _req(base, "GET", "/fall/me_abs2/ergebnis")
    _val("ergebnis", r_abs2)

    _ges_anlegen(base, "me_base", _mit_gewinn(GESAMT_KEGEL_BASIS))
    st, r_base = _req(base, "GET", "/fall/me_base/ergebnis")
    _val("ergebnis", r_base)

    if catala:
        for r in (r_abs1, r_abs2, r_base):
            assert r["grund"] == "bestaetigt"
        d_abs1 = r_base["zahl_cent"] - r_abs1["zahl_cent"]   # 3000€ Anrechnung (flach)
        d_abs2 = r_base["zahl_cent"] - r_abs2["zahl_cent"]   # ~1260€ Abzug (progressiv)
        assert d_abs1 == 300000, f"Abs.1 Anrechnung delta={d_abs1} ≠ 300000 (Regression Flag-absent)"
        assert ABS2_MIN <= d_abs2 <= ABS2_MAX, f"Abs.2 delta={d_abs2} nicht im progressiven Band"
        assert d_abs2 < d_abs1, f"Doppel-Relief? Abs.2-delta {d_abs2} ≥ Abs.1-delta {d_abs1}"
    else:
        assert r_abs1["zahl_cent"] is None or r_abs2["zahl_cent"] is None


def test_p34c_2_abzug_statt_anrechnung_rentner(base):
    """Abs.2-Antrag im rentner_gesamt-Ring: ausländische Steuer mindert die Bemessungsgrundlage
    progressiv (Point C). Spiegelt gesamt (1:1 Präzedenz)."""
    catala = _catala_da()
    _rent_anlegen(base, "abzr1", RENTNER_KEGEL_HOCH)
    st, ohne = _req(base, "GET", "/fall/abzr1/ergebnis")
    _val("ergebnis", ohne)

    _rent_anlegen(base, "abzr2", list(RENTNER_KEGEL_HOCH) + DBA_ABS2)
    st, mit = _req(base, "GET", "/fall/abzr2/ergebnis")
    _val("ergebnis", mit)

    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        assert ABS2_MIN <= delta <= ABS2_MAX, f"Abs.2 rentner delta={delta} nicht in [{ABS2_MIN},{ABS2_MAX}]"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


def test_p34c_mit_p35_kombi_senkt_steuer_kumulativ(base):
    """200k Gewinn + DBA (3000€) + §35 GewSt (4000€) → tax um 7000€ niedriger.
    Verifiziert: §34c VOR §35 (§35 Abs.1 S.4: geminderte tarifliche nach §34c).
    Deckel-3 nutzt die POST-§34c-tarifliche → korrekte Reihenfolge."""
    catala = _catala_da()

    # Ohne beides
    kegel_base = _mit_gewinn(GESAMT_KEGEL_BASIS)
    _ges_anlegen(base, "kbo", kegel_base)
    st, ohne = _req(base, "GET", "/fall/kbo/ergebnis")
    _val("ergebnis", ohne)

    # Nur DBA
    kegel_dba = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_dba.append(("dba_gezahlte_auslaendische_steuer", 300000))
    kegel_dba.append(("dba_auslaendische_einkuenfte", 4000000))
    _ges_anlegen(base, "kbd", kegel_dba)
    st, nur_dba = _req(base, "GET", "/fall/kbd/ergebnis")
    _val("ergebnis", nur_dba)

    # Nur §35
    kegel_p35 = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_p35.append(("gewst_messbetrag", 100000))  # 1000€
    kegel_p35.append(("gewst_hebesatz", 400))
    _ges_anlegen(base, "kbg", kegel_p35)
    st, nur_p35 = _req(base, "GET", "/fall/kbg/ergebnis")
    _val("ergebnis", nur_p35)

    # Beide
    kegel_beide = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_beide.append(("dba_gezahlte_auslaendische_steuer", 300000))
    kegel_beide.append(("dba_auslaendische_einkuenfte", 4000000))
    kegel_beide.append(("gewst_messbetrag", 100000))
    kegel_beide.append(("gewst_hebesatz", 400))
    _ges_anlegen(base, "kbx", kegel_beide)
    st, beide = _req(base, "GET", "/fall/kbx/ergebnis")
    _val("ergebnis", beide)

    if catala:
        for r in [ohne, nur_dba, nur_p35, beide]:
            assert r["grund"] == "bestaetigt"
        delta_dba = ohne["zahl_cent"] - nur_dba["zahl_cent"]
        delta_p35 = ohne["zahl_cent"] - nur_p35["zahl_cent"]
        delta_beide = ohne["zahl_cent"] - beide["zahl_cent"]
        assert delta_dba == 300000, f"DBA allein delta={delta_dba}"
        # §35 credit = min(4*1000=4000, 1000*400/100=4000, zähler*tarifliche/nenner)
        # 200000 * post-DBA-tarifliche / 200000 ≈ 73072→4000 deckelt → credit 4000€
        assert delta_p35 == 400000, f"§35 allein delta={delta_p35}"
        assert delta_beide == 700000, f"DBA+§35 kumulativ delta={delta_beide} ≠ 700000"
    else:
        for r in [ohne, nur_dba, nur_p35, beide]:
            assert r["zahl_cent"] is None


# ===== §35 MITUNTERNEHMER =================================================

def test_p35_mitu_senkt_steuer_gesamt(base):
    """Mitunternehmer (betriebsart=selbstaendig, mitu=200k) + GewSt (1000€ MB, 400% Hebesatz).
    Verifiziert: §35-Mitu-Zähler greift korrekt (betriebsart≠gewerbe → Zähler=mitu).
    Vor Fix: Zähler=0 (Over-tax). Jetzt: §35-Kredit=4000€ → tax um 4000€ niedriger."""
    catala = _catala_da()

    kegel_ohne = _mit_mitu(GESAMT_KEGEL_BASIS)
    _ges_anlegen(base, "mtu", kegel_ohne)
    st, ohne = _req(base, "GET", "/fall/mtu/ergebnis")
    _val("ergebnis", ohne)

    kegel_mit = _mit_mitu(GESAMT_KEGEL_BASIS)
    kegel_mit.append(("gewst_messbetrag", 100000))  # 1000€
    kegel_mit.append(("gewst_hebesatz", 400))
    _ges_anlegen(base, "mtg", kegel_mit)
    st, mit = _req(base, "GET", "/fall/mtg/ergebnis")
    _val("ergebnis", mit)

    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        # mitu=200000 → zähler=200000, nenner=200000, tarifliche≈73072
        # Deckel: min(4000, 4000, 200000*73072/200000=73072) = 4000
        assert delta == 400000, f"§35-Mitu delta={delta} ≠ 400000 cent (4000€)"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


# ===== GAP-A / GAP-B (an_gesamt-Guards) ==================================

def test_gap_a_kinder_gehoeren_in_gesamt(base):
    """an_gesamt mit fam_anzahl_kinder=2 → grund='kinder_gehoeren_in_gesamt', zahl_cent=null.
    Verifiziert: Gap-A sperrt AN-Scheibe mit Kindern (kein §31 ohne gesamt_guard)."""
    _an_anlegen(base, "gapA", [(f, (2 if f == "fam_anzahl_kinder" else w)) for f, w in AN_KEGEL_HOCH])
    st, erg = _req(base, "GET", "/fall/gapA/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "kinder_gehoeren_in_gesamt"
    assert erg["zahl_cent"] is None


def test_gap_b_verlustvortrag_gehoert_in_gesamt(base):
    """an_gesamt mit verlustvortrag_bestand=500000 cent → grund='verlustvortrag_gehoert_in_gesamt'.
    Verifiziert: Gap-B sperrt AN-Scheibe mit Verlustvortrag (kein sonstige_abzuege-Slot)."""
    k = [(f, (500000 if f == "verlustvortrag_bestand" else w)) for f, w in AN_KEGEL_HOCH]
    _an_anlegen(base, "gapB", k)
    st, erg = _req(base, "GET", "/fall/gapB/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "verlustvortrag_gehoert_in_gesamt"
    assert erg["zahl_cent"] is None




# ===== §33a UNTERHALT + AUSBILDUNGSFREIBETRAG ==============================

def test_p33a_unterhalt_und_ausbildung_senkt_steuer_gesamt(base):
    """200k Gewinn + §33a Unterhalt (15000€→GFB-gedeckelt 12096€) + 2×Ausbildungsfreibetrag (2400€)
    → sonstige_abzuege=14496€ → zve sinkt → tax niedriger als ohne §33a.
    Verifiziert: GdE-Abzug (Unterhalt + Ausbildungsfreibetrag) im Ring wirksam."""
    catala = _catala_da()

    kegel_ohne = _mit_gewinn(GESAMT_KEGEL_BASIS)
    _ges_anlegen(base, "p33a1", kegel_ohne)
    st, ohne = _req(base, "GET", "/fall/p33a1/ergebnis")
    _val("ergebnis", ohne)

    kegel_mit = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_mit.append(("p33a_unterhalt_aufwendungen", 1500000))   # 15000€ → gedeckelt auf GFB 12096€
    kegel_mit.append(("p33a_ausbildung_anzahl_kinder", 2))        # 2×1200€ = 2400€
    _ges_anlegen(base, "p33a2", kegel_mit)
    st, mit = _req(base, "GET", "/fall/p33a2/ergebnis")
    _val("ergebnis", mit)

    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        # sonstige_abzuege = 12096 (Unterhalt GFB-gedeckelt) + 2400 (2×Ausbildung) = 14496€
        # zve sinkt um 14496€ → tax sinkt um ≈ 14496 × 42% ≈ 6088€ (Grenzsteuersatz)
        assert delta > 0, f"§33a sollte tax senken: ohne={ohne['zahl_cent']}, mit={mit['zahl_cent']}"
        assert delta >= 500000, f"§33a delta={delta} < 500000 cent (Mindestwirkung 5000€)"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


# ===== B1 §9 dHf/Verpflegung im GESAMT-Ring (gemischt-§19, Over-tax-Fix) ==================
# Der gesamt-Ring rechnete §19-WK bisher NUR als Entfernungspauschale; ein Angestellter MIT
# weiterer Einkunftsart (→ gesamt statt an_gesamt) verlor doppelte Haushaltsführung + Verpflegung
# = Over-tax. B1 verdrahtet catala_werbungskosten_n mit dHf/Verpflegung (Parität an_gesamt) UND
# registriert die Felder in gesamt.felder (Erreichbarkeit, POST 201). Der SHARED _an_gesamt_sperrgrund
# hält Ausland-dHf / offene Verpflegungs-Reduktion fail-closed offen (auch für gesamt).

# Reiner Angestellter (200k Lohn) im GESAMT-Ring (nicht an_gesamt): kein_gewinn=True, aber die Scheibe
# ist gesamt → nur hier greifen die weiteren Einkunftsarten/Abzüge. dHf/Verpflegung müssen trotzdem wirken.
GESAMT_AN_KEGEL = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 20000000),   # 200.000 € §19-Lohn
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
]

# dHf Inland gültig: 1000 €/Monat × 12 = 12.000 € Roh-WK (Kappung §9 Abs.1 Nr.5 = 1000 €/Monat).
DHF_GESAMT_VALID = [
    ("dhf_unterkunftskosten_monat", 100000), ("dhf_monate", 12), ("dhf_im_inland", True),
    ("dhf_beruflich_veranlasst", True), ("dhf_eigener_hausstand", True),
    ("dhf_finanzielle_beteiligung", True), ("dhf_keine_pflicht_dienstwohnung", True),
]
# Verpflegung: 100 volle Tage × 28 € = 2.800 € Roh-WK; Reduktion explizit safe (≤3 Monate + keine Mahlzeit).
VPF_GESAMT_VALID = [
    ("tage_24h", 100), ("tage_an_abreise", 0), ("tage_ueber_8h_eintaegig", 0),
    ("vpf_monate_am_ort", 2), ("vpf_keine_mahlzeitengestellung", True),
]


def test_b1_dhf_senkt_steuer_gesamt(base):
    """gesamt-Ring, 200k Lohn + gültige dHf (12.000 € Roh-WK statt AN-Pauschbetrag 1230 €): einkuenfte_
    nichtselbststaendig sinkt um 10.770 € → tax progressiv (~4.523 € bei 42 %). Beweist Erreichbarkeit
    (POST 201, nicht 400) + dHf-Durchgriff im gesamt-WK. Ohne B1 wäre Δ=0 (Over-tax)."""
    catala = _catala_da()
    _ges_anlegen(base, "b1dhf_o", GESAMT_AN_KEGEL)
    st, ohne = _req(base, "GET", "/fall/b1dhf_o/ergebnis")
    _val("ergebnis", ohne)
    _ges_anlegen(base, "b1dhf_m", GESAMT_AN_KEGEL + DHF_GESAMT_VALID)
    st, mit = _req(base, "GET", "/fall/b1dhf_m/ergebnis")
    _val("ergebnis", mit)
    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt", \
            f"ohne={ohne.get('grund')} mit={mit.get('grund')}"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        # Δeinkünfte = 12000 − 1230 = 10770 €; × 42 % ≈ 4523 € = 452340 ct.
        assert 430000 <= delta <= 475000, f"dHf gesamt delta={delta} nicht in [430000,475000]"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


def test_b1_dhf_ausland_haelt_offen_gesamt(base):
    """gesamt-Ring + dHf mit dhf_im_inland=False → der SHARED Guard sperrt fail-closed
    (ausland_dhf_nicht_ring_faehig, zahl_cent=null). K2: kein stiller Über-Abzug im gesamt-Ring."""
    kegel = GESAMT_AN_KEGEL + [
        ("dhf_unterkunftskosten_monat", 100000), ("dhf_monate", 12), ("dhf_im_inland", False),
        ("dhf_beruflich_veranlasst", True), ("dhf_eigener_hausstand", True),
        ("dhf_finanzielle_beteiligung", True), ("dhf_keine_pflicht_dienstwohnung", True)]
    _ges_anlegen(base, "b1ausl", kegel)
    st, erg = _req(base, "GET", "/fall/b1ausl/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "ausland_dhf_nicht_ring_faehig", f"grund={erg.get('grund')}"
    assert erg["zahl_cent"] is None


def test_b1_verpflegung_senkt_steuer_gesamt(base):
    """gesamt-Ring, 200k Lohn + Verpflegung (100 volle Tage = 2.800 € Roh-WK, Reduktion safe): einkuenfte_
    nichtselbststaendig sinkt (2.800 − 1.230 = 1.570 € über Pauschbetrag) → tax progressiv (~660 €).
    Beweist die zweite §9-WK-Art (Verpflegung) im gesamt-Ring erreichbar + verdrahtet."""
    catala = _catala_da()
    _ges_anlegen(base, "b1vpf_o", GESAMT_AN_KEGEL)
    st, ohne = _req(base, "GET", "/fall/b1vpf_o/ergebnis")
    _val("ergebnis", ohne)
    _ges_anlegen(base, "b1vpf_m", GESAMT_AN_KEGEL + VPF_GESAMT_VALID)
    st, mit = _req(base, "GET", "/fall/b1vpf_m/ergebnis")
    _val("ergebnis", mit)
    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt", \
            f"ohne={ohne.get('grund')} mit={mit.get('grund')}"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        # 100×28=2800 € Roh; Δeinkünfte ≈ 1570 €; × 42 % ≈ 659 € = 65940 ct. Band robust ggü. Satz-Rundung.
        assert 40000 <= delta <= 85000, f"Verpflegung gesamt delta={delta} nicht in [40000,85000]"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


# ===================== A5: § 9 Abs. 1 S. 3 Nr. 5a Übernachtung im gesamt-Ring =====================
# Dritte §9-WK-Art (nach dHf/Verpflegung) im gesamt-Ring: tatsächliche Übernachtungskosten bei
# Auswärtstätigkeit. Beweist Erreichbarkeit (POST 201) + Durchgriff (catala_werbungskosten_n), die
# Satz-4-Kappung nach 48 Monaten (Under-tax-Wächter: monate_bisher muss durch den Ring fließen),
# und fail-closed bei Ausland / überspannendem 48-Monats-Zeitraum.

# Übernachtung Inland gültig, vor 48 Monaten: 1.000 €/Monat × 12 = 12.000 € Roh-WK (ungekappt).
UEBERNACHTUNG_GESAMT_VALID = [
    ("uebernachtung_kosten_monat", 100000), ("uebernachtung_monate", 12),
    ("uebernachtung_monate_bisher", 10), ("uebernachtung_im_inland", True),
    ("uebernachtung_auswaerts", True), ("uebernachtung_alleinnutzung", True),
    ("uebernachtung_keine_lange_unterbrechung", True),
]


def _ueb(bisher, kosten=200000, monate=12):
    """Übernachtungs-Kegelteil; bisher steuert vor/nach 48 (Kappung), kosten in Cent (2.000 €/Monat)."""
    return [("uebernachtung_kosten_monat", kosten), ("uebernachtung_monate", monate),
            ("uebernachtung_monate_bisher", bisher), ("uebernachtung_im_inland", True),
            ("uebernachtung_auswaerts", True), ("uebernachtung_alleinnutzung", True),
            ("uebernachtung_keine_lange_unterbrechung", True)]


def test_a5_uebernachtung_senkt_steuer_gesamt(base):
    """gesamt-Ring, 200k Lohn + gültige Übernachtung (12.000 € Roh-WK statt Pauschbetrag 1.230 €):
    einkuenfte_nichtselbststaendig sinkt um 10.770 € → tax progressiv (~4.523 € bei 42 %). Beweist
    Erreichbarkeit (POST 201, nicht 400) + Übernachtungs-Durchgriff im gesamt-WK. Ohne A5 wäre Δ=0."""
    catala = _catala_da()
    _ges_anlegen(base, "a5ueb_o", GESAMT_AN_KEGEL)
    st, ohne = _req(base, "GET", "/fall/a5ueb_o/ergebnis")
    _val("ergebnis", ohne)
    _ges_anlegen(base, "a5ueb_m", GESAMT_AN_KEGEL + UEBERNACHTUNG_GESAMT_VALID)
    st, mit = _req(base, "GET", "/fall/a5ueb_m/ergebnis")
    _val("ergebnis", mit)
    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt", \
            f"ohne={ohne.get('grund')} mit={mit.get('grund')}"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        # Δeinkünfte = 12000 − 1230 = 10770 €; × 42 % ≈ 4523 € = 452340 ct.
        assert 430000 <= delta <= 475000, f"Übernachtung gesamt delta={delta} nicht in [430000,475000]"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


def test_a5_uebernachtung_ausland_haelt_offen_gesamt(base):
    """gesamt-Ring + uebernachtung_im_inland=False → der SHARED Guard sperrt fail-closed
    (ausland_uebernachtung_nicht_ring_faehig, zahl_cent=null). K2: kein stiller Über-Abzug (2.000er-
    Auslandsgrenze ist außerhalb dieser Scheibe)."""
    kegel = GESAMT_AN_KEGEL + [
        ("uebernachtung_kosten_monat", 100000), ("uebernachtung_monate", 12),
        ("uebernachtung_monate_bisher", 10), ("uebernachtung_im_inland", False),
        ("uebernachtung_auswaerts", True), ("uebernachtung_alleinnutzung", True),
        ("uebernachtung_keine_lange_unterbrechung", True)]
    _ges_anlegen(base, "a5ausl", kegel)
    st, erg = _req(base, "GET", "/fall/a5ausl/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "ausland_uebernachtung_nicht_ring_faehig", f"grund={erg.get('grund')}"
    assert erg["zahl_cent"] is None


def test_a5_uebernachtung_zeitraum_offen_gesamt(base):
    """gesamt-Ring + Übernachtungs-Zeitraum überspannt die 48-Monats-Schwelle (bisher=40, monate=12 →
    40<48<52). Die Einzel-Regel (_vor_48 / _nach_48) kann den gemischten Zeitraum nicht kappen → der
    Guard sperrt fail-closed (uebernachtung_zeitraum_offen). K2: kein still-ungekappter Über-Abzug."""
    kegel = GESAMT_AN_KEGEL + _ueb(40)
    _ges_anlegen(base, "a5span", kegel)
    st, erg = _req(base, "GET", "/fall/a5span/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "uebernachtung_zeitraum_offen", f"grund={erg.get('grund')}"
    assert erg["zahl_cent"] is None


def test_a5_uebernachtung_nach_48_kappung_gesamt(base):
    """gesamt-Ring: 2.000 €/Monat Übernachtung. VOR 48 (bisher=10) → 24.000 € Roh-WK ungekappt; NACH 48
    (bisher=48) → Satz-4-Kappung min(2.000,1.000)×12 = 12.000 €. Der nach-48-Fall MUSS mehr Steuer haben
    (12.000 € weniger Abzug ≈ 5.040 € Δtax bei 42 %). K2-Wächter: ein vergessenes monate_bisher-Wiring
    rechnete beide ungekappt = stiller Under-tax nach 48 Monaten."""
    catala = _catala_da()
    _ges_anlegen(base, "a5vor48", GESAMT_AN_KEGEL + _ueb(10))
    st, vor = _req(base, "GET", "/fall/a5vor48/ergebnis")
    _val("ergebnis", vor)
    _ges_anlegen(base, "a5nach48", GESAMT_AN_KEGEL + _ueb(48))
    st, nach = _req(base, "GET", "/fall/a5nach48/ergebnis")
    _val("ergebnis", nach)
    if catala:
        assert vor["grund"] == "bestaetigt" and nach["grund"] == "bestaetigt", \
            f"vor={vor.get('grund')} nach={nach.get('grund')}"
        delta = nach["zahl_cent"] - vor["zahl_cent"]
        # (24000−12000)=12000 € weniger Abzug nach 48; × 42 % ≈ 5040 € = 504000 ct.
        assert 470000 <= delta <= 540000, f"nach-48-Kappung delta={delta} nicht in [470000,540000]"
    else:
        assert vor["zahl_cent"] is None or nach["zahl_cent"] is None


# ===================== A6: § 9 Abs. 1 S. 3 Nr. 6/7 i.V.m. § 6 Abs. 2 Arbeitsmittel-GWG =====================
# Vierte §9-WK-Art (nach dHf/Verpflegung/Übernachtung): Arbeitsmittel als GWG-Sofortabzug ≤ 800 € (§ 6
# Abs. 2). Level-1 = nur der Sofortabzug ist ring-fähig; der mehrjährige § 7-AfA-Zweig (> 800 €) sowie
# ein abgelehntes Wahlrecht sperren fail-closed. Da der GWG-Betrag (max 800 €) UNTER dem AN-Pauschbetrag
# (1.230 €) liegt, kann er allein den Pauschbetrag nie überschreiten (Δ=0 maskiert vom Günstigerprinzip)
# → der Senk-Test stapelt das GWG auf eine gültige Übernachtungs-Basis (12.000 € Roh-WK > 1.230 €), sodass
# der 800-€-Zuwachs als reiner Inkrement-Effekt messbar wird. CENT-Schwelle (80000) statt Euro-Floor: 800,01 €
# = 80001 ct darf NICHT als GWG durchrutschen (Under-tax-Wächter).

# Arbeitsmittel GWG gültig: 800 € (= Grenze, ≤ 800) mit ausgeübtem Wahlrecht. In CENT (80000).
ARBEITSMITTEL_GWG_VALID = [
    ("am_anschaffungskosten", 80000), ("am_gwg_sofortabzug_gewaehlt", True),
]


def test_a6_arbeitsmittel_gwg_senkt_steuer_gesamt(base):
    """gesamt-Ring: gültige Übernachtungs-Basis (12.000 € Roh-WK) OHNE vs. MIT Arbeitsmittel-GWG (800 €,
    Wahlrecht ausgeübt). Der GWG-Sofortabzug erhöht die Roh-WK um 800 € → einkuenfte_nichtselbststaendig
    sinkt um 800 € → Δtax ≈ 336 € bei 42 %. Beweist Erreichbarkeit (POST 201) + GWG-Durchgriff
    (catala_p6_2_gwg) im gesamt-WK. Ohne A6 wäre Δ=0 (stiller Over-tax = kein Arbeitsmittel-Abzug)."""
    catala = _catala_da()
    _ges_anlegen(base, "a6gwg_o", GESAMT_AN_KEGEL + UEBERNACHTUNG_GESAMT_VALID)
    st, ohne = _req(base, "GET", "/fall/a6gwg_o/ergebnis")
    _val("ergebnis", ohne)
    _ges_anlegen(base, "a6gwg_m", GESAMT_AN_KEGEL + UEBERNACHTUNG_GESAMT_VALID + ARBEITSMITTEL_GWG_VALID)
    st, mit = _req(base, "GET", "/fall/a6gwg_m/ergebnis")
    _val("ergebnis", mit)
    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt", \
            f"ohne={ohne.get('grund')} mit={mit.get('grund')}"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        # Δeinkünfte = 800 €; × 42 % ≈ 336 € = 33600 ct. Band robust ggü. Satz-Rundung.
        assert 25000 <= delta <= 42000, f"Arbeitsmittel-GWG gesamt delta={delta} nicht in [25000,42000]"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


def test_a6_arbeitsmittel_afa_ueber_800_haelt_offen_gesamt(base):
    """gesamt-Ring + Arbeitsmittel-AK 1.000 € (> 800 € = kein GWG, sondern mehrjährige § 7-AfA), Wahlrecht
    ausgeübt. Der § 7-AfA-Zweig ist ungebunden (Nutzungsdauer/Anschaffungsmonat nicht im Ring) → der
    SHARED Guard sperrt fail-closed (arbeitsmittel_afa_ueber_gwg_offen, zahl_cent=null). K2: kein stiller,
    ungezwölftelter Voll-Abzug der AfA-Basis."""
    kegel = GESAMT_AN_KEGEL + [("am_anschaffungskosten", 100000), ("am_gwg_sofortabzug_gewaehlt", True)]
    _ges_anlegen(base, "a6afa", kegel)
    st, erg = _req(base, "GET", "/fall/a6afa/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "arbeitsmittel_afa_ueber_gwg_offen", f"grund={erg.get('grund')}"
    assert erg["zahl_cent"] is None


def test_a6_arbeitsmittel_gwg_cent_floor_haelt_offen_gesamt(base):
    """gesamt-Ring + Arbeitsmittel-AK 800,01 € = 80001 ct (> 800-€-Grenze). Der Under-tax-Wächter: würde
    die Schwelle in EURO (80001 // 100 = 800) statt CENT (80000) geprüft, rutschte 800,01 € fälschlich als
    GWG durch (voller Sofortabzug statt AfA-Verteilung = Under-tax). Guard muss fail-closed sperren
    (arbeitsmittel_afa_ueber_gwg_offen)."""
    kegel = GESAMT_AN_KEGEL + [("am_anschaffungskosten", 80001), ("am_gwg_sofortabzug_gewaehlt", True)]
    _ges_anlegen(base, "a6floor", kegel)
    st, erg = _req(base, "GET", "/fall/a6floor/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "arbeitsmittel_afa_ueber_gwg_offen", f"grund={erg.get('grund')}"
    assert erg["zahl_cent"] is None


def test_a6_arbeitsmittel_wahlrecht_abgelehnt_haelt_offen_gesamt(base):
    """gesamt-Ring + Arbeitsmittel-AK 800 € (≤ 800, GWG-fähig), aber Wahlrecht NICHT ausgeübt
    (am_gwg_sofortabzug_gewaehlt=False → § 6 Abs. 2 „koennen"). Ohne ausgeübtes Wahlrecht ist der
    Sofortabzug nicht anwendbar; der § 7-AfA-Pfad wäre nötig, ist aber ungebunden → Guard sperrt
    fail-closed (arbeitsmittel_afa_ueber_gwg_offen). K2: kein Sofortabzug ohne ausgeübtes Wahlrecht."""
    kegel = GESAMT_AN_KEGEL + [("am_anschaffungskosten", 80000), ("am_gwg_sofortabzug_gewaehlt", False)]
    _ges_anlegen(base, "a6nowahl", kegel)
    st, erg = _req(base, "GET", "/fall/a6nowahl/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "arbeitsmittel_afa_ueber_gwg_offen", f"grund={erg.get('grund')}"
    assert erg["zahl_cent"] is None


def test_a6_l2_catala_afa_linear_unit(base):
    """Unit-Test für runner.catala_p7_linear_afa: bestätigt korrekte lineare AfA-Berechnung
    (anschaffungskosten_cent // nutzungsdauer_euro). Nutzungsdauer=0 oder negativ → 0."""
    if not _catala_da():
        pytest.skip("catala runner not available")
    assert R.catala_p7_linear_afa({"anschaffungskosten_cent": 150000, "nutzungsdauer": 5}) == 300   # 1500/5=300
    assert R.catala_p7_linear_afa({"anschaffungskosten_cent": 80001, "nutzungsdauer": 3}) == 266    # 800.01/3=266.67 floor=266
    assert R.catala_p7_linear_afa({"anschaffungskosten_cent": 100000, "nutzungsdauer": 10}) == 100  # 1000/10=100
    assert R.catala_p7_linear_afa({"anschaffungskosten_cent": 100000, "nutzungsdauer": 0}) == 0    # fail-safe
    assert R.catala_p7_linear_afa({"anschaffungskosten_cent": 100000, "nutzungsdauer": -1}) == 0   # fail-safe


def test_a6_l2_afa_mit_nutzungsdauer_senkt_steuer_gesamt(base):
    """§ 7 lineare AfA im gesamt-Ring (A6-L2): AK 1500€ = 150000 ct (> 800€ → kein GWG) + Nutzungsdauer
    5 Jahre → AfA = 150000 // 500 = 300 €/Jahr → WK +300 → zvE −300 → Steuer sinkt messbar. Beweist
    ring-fähiger A6-L2-Durchgriff (nicht mehr nur Guard). Baucht auf Übernachtungs-Basis 12000€ (wie GWG-
    Test), damit Δ über AN-Pauschbetrag (1230€) sichtbar ist."""
    catala = _catala_da()
    a6l2 = [("am_anschaffungskosten", 150000), ("arbeitsmittel_nutzungsdauer", 5)]
    _ges_anlegen(base, "a6l2o", GESAMT_AN_KEGEL + UEBERNACHTUNG_GESAMT_VALID)
    st, ohne = _req(base, "GET", "/fall/a6l2o/ergebnis")
    _val("ergebnis", ohne)
    _ges_anlegen(base, "a6l2m", GESAMT_AN_KEGEL + UEBERNACHTUNG_GESAMT_VALID + a6l2)
    st, mit = _req(base, "GET", "/fall/a6l2m/ergebnis")
    _val("ergebnis", mit)
    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt", \
            f"ohne={ohne.get('grund')} mit={mit.get('grund')}"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        # AfA 300€ → Δzve 300€ → Δtax ≈ 126€ = 12600 ct bei ~42%. Band ±5000 ct.
        assert 8000 <= delta <= 17000, f"A6-L2 AfA delta={delta} nicht in [8000,17000]"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


DBA_KEGEL_ANRECHNUNG = [
    ("dba_staat", "at"),
    ("dba_methode", "anrechnung"),
    ("dba_gezahlte_auslaendische_steuer", 500000),     # 5000€ gezahlt
    ("dba_auslaendische_einkuenfte", 3000000),          # 30000€ ausländisch
]

DBA_KEGEL_FREISTELLUNG = [
    ("dba_staat", "at"),
    ("dba_methode", "freistellung"),
    ("dba_gezahlte_auslaendische_steuer", 0),
    ("dba_auslaendische_einkuenfte", 3000000),          # 30000€ ausländisch
]

# DBA-Konstante für Unit-Tests — Accessor nur. Keine HTTP-E2E wegen gesamt-g2-Bug.

def test_dba_unit_catala_p34c_1_anrechnung_seed():
    """Unit-Test für catala_p34c_1 (Anrechnung): min(gezahlt, HB)."""
    if not _catala_da():
        pytest.skip("catala runner not available")
    s = {"gezahlte_auslaendische_steuer": 5000, "deutsche_est_inkl_ausl": 30000,
         "zu_versteuerndes_einkommen": 60000, "auslaendische_einkuenfte_staat": 30000}
    result = R.catala_p34c_1(s)
    hb = 30000 * 30000 // 60000  # = 15000
    assert result == 5000, f"min(5000, 15000) = {result}, expected 5000"


def test_dba_unit_catala_p34c_1_hoechstbetrag_limit():
    """Unit-Test catala_p34c_1: HB limit funktioniert."""
    if not _catala_da():
        pytest.skip("catala runner not available")
    s = {"gezahlte_auslaendische_steuer": 20000, "deutsche_est_inkl_ausl": 30000,
         "zu_versteuerndes_einkommen": 60000, "auslaendische_einkuenfte_staat": 30000}
    result = R.catala_p34c_1(s)
    hb = 15000  # 30000*30000//60000
    assert result == 15000, f"Höchstbetrag: {result}, expected {hb}"


def test_dba_method_map_coverage():
    """Verify all 11 DBA countries are mapped to a method."""
    if not _catala_da():
        pytest.skip("catala runner not available")
    for country in ["at", "ch", "dk", "es", "fr", "gb", "lu", "nl", "pl", "tr", "us"]:
        assert country in API.DBA_METHOD_MAP, f"{country} missing from DBA_METHOD_MAP"
        method = API.DBA_METHOD_MAP[country]
        assert method in ("anrechnung", "freistellung"), \
            f"{country} has invalid method '{method}'"


# ---- A4 § 36 Abs. 2+4: Anrechnung / Abschlusszahlung (Post-Festsetzung, ändert NIE die ESt) ----

def _a4_zahl_baseline(base, fid="a4base"):
    """Baseline an_gesamt OHNE §36-Felder → festgesetzte ESt (zahl_cent); prüft abschlusszahlung None."""
    _an_anlegen(base, fid, AN_KEGEL_HOCH)
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt", f"grund={erg.get('grund')}"
    assert erg["abschlusszahlung_cent"] is None
    return erg["zahl_cent"]


def test_a4_abschlusszahlung_none_ohne_felder(base):
    """Ohne §36-Anrechnungsfelder bleibt abschlusszahlung_cent None (keine irreführende Voll-Nachzahlung)."""
    z = _a4_zahl_baseline(base)
    assert isinstance(z, int) and z > 0


def test_a4_lohnsteuer_only_erstattung_und_kein_est_impact(base):
    """LSt hoch einbehalten → Abschlusszahlung = ESt − aufgerundete LSt (hier Erstattung, negativ).
    KERN: zahl_cent IDENTISCH zur Baseline → §36-Anrechnung bewegt die festgesetzte ESt NIE."""
    z0 = _a4_zahl_baseline(base, "a4b0")
    lst = z0 + 5000000
    _an_anlegen(base, "a4lst", AN_KEGEL_HOCH + [("p36_lohnsteuer", lst)])
    st, erg = _req(base, "GET", "/fall/a4lst/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt"
    assert erg["zahl_cent"] == z0, "§36-Anrechnung darf die festgesetzte ESt nicht verändern"
    exp = R.catala_p36_abschlusszahlung(
        {"festzusetzende_est_cent": z0, "lohnsteuer_cent": lst, "vorauszahlungen_cent": 0})
    assert erg["abschlusszahlung_cent"] == exp
    assert exp < 0


def test_a4_lohnsteuer_und_vorauszahlung_nachzahlung(base):
    """LSt + Vorauszahlung beide bestätigt → Abschlusszahlung = ESt − LSt(aufgerundet) − VZ."""
    z0 = _a4_zahl_baseline(base, "a4c0")
    lst, vor = 3000000, 1000000
    _an_anlegen(base, "a4lv", AN_KEGEL_HOCH + [("p36_lohnsteuer", lst), ("p36_vorauszahlungen", vor)])
    st, erg = _req(base, "GET", "/fall/a4lv/ergebnis")
    _val("ergebnis", erg)
    assert erg["zahl_cent"] == z0
    exp = R.catala_p36_abschlusszahlung(
        {"festzusetzende_est_cent": z0, "lohnsteuer_cent": lst, "vorauszahlungen_cent": vor})
    assert erg["abschlusszahlung_cent"] == exp
    assert exp == z0 - 3000000 - 1000000


def test_a4_lohnsteuer_cent_aufrundung_auf_volle_euro(base):
    """§36 Abs.3 S.1: Steuerabzugsbetrag auf volle Euro AUFrunden. 7.500,30 € → 7.501 € abgezogen."""
    z0 = _a4_zahl_baseline(base, "a4d0")
    _an_anlegen(base, "a4ceil", AN_KEGEL_HOCH + [("p36_lohnsteuer", 750030)])
    st, erg = _req(base, "GET", "/fall/a4ceil/ergebnis")
    _val("ergebnis", erg)
    assert erg["zahl_cent"] == z0
    assert erg["abschlusszahlung_cent"] == z0 - 750100


def test_a4_vorlaeufige_lohnsteuer_bewegt_anrechnung_nicht(base):
    """[[ring-liest-vorlaeufig-parallel-pfad-luecke]]: vorläufige (nicht bestätigte) LSt darf die
    Abschlusszahlung NICHT bewegen → bleibt None wie ohne Feld. Nur bestätigte Anrechnung zählt."""
    z0 = _a4_zahl_baseline(base, "a4s0")
    _an_anlegen(base, "a4vorl", AN_KEGEL_HOCH)
    body = {"feld_id": "p36_lohnsteuer", "wert": z0 + 5000000, "zustand": "vorlaeufig",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": None}}
    st, _ = _req(base, "POST", "/fall/a4vorl/event", body)
    assert st == 201, f"vorläufiger POST erwartet 201, war {st}"
    st, erg = _req(base, "GET", "/fall/a4vorl/ergebnis")
    _val("ergebnis", erg)
    assert erg["zahl_cent"] == z0
    assert erg["abschlusszahlung_cent"] is None, "vorläufige LSt darf Anrechnung nicht auslösen"


def test_a4_abschlusszahlung_gesamt_scheibe(base):
    """§36-Anrechnung ist scheibe-agnostisch: auch im gesamt-Ring registriert + emittiert (Reachability)."""
    _ges_anlegen(base, "a4g0", GESAMT_AN_KEGEL)
    st, e0 = _req(base, "GET", "/fall/a4g0/ergebnis")
    _val("ergebnis", e0)
    z0 = e0["zahl_cent"]
    assert e0["abschlusszahlung_cent"] is None
    _ges_anlegen(base, "a4g1", GESAMT_AN_KEGEL + [("p36_lohnsteuer", 500000)])
    st, e1 = _req(base, "GET", "/fall/a4g1/ergebnis")
    _val("ergebnis", e1)
    assert e1["zahl_cent"] == z0
    assert e1["abschlusszahlung_cent"] == z0 - 500000


def test_a4_accessor_snapshot_seeds():
    """catala_p36_abschlusszahlung gegen die 4 verified_bedingt-Snapshot-Seeds (EURO→CENT ×100)."""
    seeds = [
        (10000, 8000, 0, 2000),
        (5000, 8000, 0, -3000),
        (12000, 7500.30, 0, 4499),
        (10000, 6000, 3000, 1000),
    ]
    for est_eur, lst_eur, vor_eur, exp_eur in seeds:
        got = R.catala_p36_abschlusszahlung({
            "festzusetzende_est_cent": round(est_eur * 100),
            "lohnsteuer_cent": round(lst_eur * 100),
            "vorauszahlungen_cent": round(vor_eur * 100)})
        assert got == exp_eur * 100, f"seed {est_eur}/{lst_eur}/{vor_eur}: {got} != {exp_eur * 100}"


# ---- A8 § 22 Nr. 3: Sonstige Einkünfte aus Leistungen (Freigrenze 256 €, < 256 → 0 / ≥ 256 → voll) ----
# Nutzt gesamt-Ring (fremd_arten:kein_sonstige = schützt Rente §22 Nr.1, erlaubt §22 Nr.3 via p23-Präzedenz).

def test_a8_accessor_freigrenze_seeds():
    """Accessor-Unit: Freigrenze exakt bei 25600 Cent. < 25600 → 0, ≥ 25600 → Betrag."""
    for betrag, erwartet in [(0, 0), (25599, 0), (25600, 25600), (100000, 100000)]:
        assert R.catala_p22_nr3_einkuenfte(betrag) == erwartet, f"seed {betrag} → {erwartet}"


def _a8_baseline_gesamt(base, fid="a8base"):
    """Baseline gesamt-Ring OHNE §22 Nr.3-Feld bei ansonsten leeres GESAMT_AN_KEGEL."""
    _ges_anlegen(base, fid, GESAMT_AN_KEGEL)
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt", f"grund={erg.get('grund')}"
    return erg["zahl_cent"]


def _a8_kegel_gesamt(betrag_cent):
    """GESAMT_AN_KEGEL + p22_nr3_einkuenfte."""
    kegel = list(GESAMT_AN_KEGEL) + [("p22_nr3_einkuenfte", betrag_cent)]
    return kegel


def test_a8_freigrenze_25599_kein_est_impact_gesamt(base):
    """§22 Nr.3 = 25599 Cent (< 25600) → Δ zahl_cent == 0. Freigrenze absorbiert, Wiring lebt."""
    z0 = _a8_baseline_gesamt(base, "a8b0")
    _ges_anlegen(base, "a8unter", _a8_kegel_gesamt(25599))
    st, erg = _req(base, "GET", "/fall/a8unter/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt", f"grund={erg.get('grund')}"
    assert erg["zahl_cent"] == z0, f"25599 Freigrenze: {erg['zahl_cent']} != {z0}"


def test_a8_freigrenze_25600_steuer_steigt_gesamt(base):
    """§22 Nr.3 = 25600 Cent (≥ 25600) → Δ zahl_cent > 0. Freigrenze ≠ Freibetrag."""
    z0 = _a8_baseline_gesamt(base, "a8c0")
    _ges_anlegen(base, "a8grenze", _a8_kegel_gesamt(25600))
    st, erg = _req(base, "GET", "/fall/a8grenze/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt", f"grund={erg.get('grund')}"
    assert erg["zahl_cent"] > z0, f"25600 muss strikt steigen: {erg['zahl_cent']} <= {z0}"
    diff = erg["zahl_cent"] - z0
    assert diff > 0, f"Delta muss positiv sein, war {diff}"


def test_a8_ring_lebt_exakt_an_der_freigrenze(base):
    """25599 vs 25600 im selben Kegel (nur p22_nr3_einkuenfte differiert). 25599=Δ0, 25600=Δ>0."""
    z0 = _a8_baseline_gesamt(base, "a8d0")
    _ges_anlegen(base, "a8free", _a8_kegel_gesamt(25599))
    st, e_free = _req(base, "GET", "/fall/a8free/ergebnis")
    _val("ergebnis", e_free)
    assert e_free["grund"] == "bestaetigt", f"grund={e_free.get('grund')}"
    assert e_free["zahl_cent"] == z0, f"25599: {e_free['zahl_cent']} != {z0}"
    _ges_anlegen(base, "a8tax", _a8_kegel_gesamt(25600))
    st, e_tax = _req(base, "GET", "/fall/a8tax/ergebnis")
    _val("ergebnis", e_tax)
    assert e_tax["grund"] == "bestaetigt", f"grund={e_tax.get('grund')}"
    assert e_tax["zahl_cent"] > z0, f"25600: {e_tax['zahl_cent']} <= {z0}"


# ---- § 51a KiSt: kist_konfession/kist_bundesland Erreichbarkeit (totes Wiring-Fix) ----

def test_kist_accessor_konfession_keine_oder_andere(base):
    """Accessor-Unit: konfession keine/andere → kist_cent = 0 (auch bei beliebigem bundesland)."""
    assert R.catala_kist({"est_mit_fb": 10000, "konfession": "keine", "bundesland": "nordrhein_westfalen"}) == 0
    assert R.catala_kist({"est_mit_fb": 10000, "konfession": "andere", "bundesland": "bayern"}) == 0


def test_kist_accessor_satz_9_nrw_und_8_bayern(base):
    """Accessor-Unit: 9 % NRW (steuererhebend) vs 8 % BY (Kirchensteuer-Cent exakt)."""
    assert R.catala_kist({"est_mit_fb": 10000, "konfession": "roemisch-katholisch", "bundesland": "nordrhein_westfalen"}) == 90000
    assert R.catala_kist({"est_mit_fb": 10000, "konfession": "evangelisch", "bundesland": "bayern"}) == 80000


def _kist_kegel(betrag_cent=20000000, konfession="roemisch-katholisch", bundesland="nordrhein_westfalen"):
    """AN_KEGEL_HOCH + KiSt-Felder."""
    k = list(AN_KEGEL_HOCH)
    k.append(("kist_konfession", konfession))
    k.append(("kist_bundesland", bundesland))
    return k


def test_kist_erreichbarkeit_an_gesamt(base):
    """§51a KiSt: kist_konfession + kist_bundesland POSTbar → kist_cent > 0."""
    _an_anlegen(base, "kist1", _kist_kegel())
    st, erg = _req(base, "GET", "/fall/kist1/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt", f"grund={erg.get('grund')}"
    assert isinstance(erg["kist_cent"], int) and erg["kist_cent"] > 0, f"kist_cent={erg['kist_cent']}"


def test_kist_ratio_9zu8_exakt(base):
    """NRW(9%) vs Bayern(8%) bei gleichem est_mit_fb: kist_nrw × 8 == kist_by × 9 (Ganzzahl-Ratio)."""
    _an_anlegen(base, "kist_nrw", _kist_kegel(konfession="roemisch-katholisch", bundesland="nordrhein_westfalen"))
    st, e_nrw = _req(base, "GET", "/fall/kist_nrw/ergebnis")
    _val("ergebnis", e_nrw)
    assert e_nrw["grund"] == "bestaetigt"
    _an_anlegen(base, "kist_by", _kist_kegel(konfession="evangelisch", bundesland="bayern"))
    st, e_by = _req(base, "GET", "/fall/kist_by/ergebnis")
    _val("ergebnis", e_by)
    assert e_by["grund"] == "bestaetigt"
    assert e_nrw["kist_cent"] * 8 == e_by["kist_cent"] * 9, f"9:8-Ratio verletzt: {e_nrw['kist_cent']}*8 vs {e_by['kist_cent']}*9"


def test_kist_ohne_konfession_null(base):
    """Keine Konfession gesetzt (default→keine) → kist_cent = 0 (Accessor liefert 0)."""
    _an_anlegen(base, "kist_null", AN_KEGEL_HOCH)  # kein kist_konfession/kist_bundesland
    st, erg = _req(base, "GET", "/fall/kist_null/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt"
    assert erg["kist_cent"] == 0, f"kist_cent={erg['kist_cent']}"


# ---- § 16 Abs. 4: Veräußerungsfreibetrag fail-closed gaten (Under-tax-Fix) ----

_RENTNER_KEGEL_VG_BASIS = [
    (f, (False if f == "kein_gewinn" else w)) for f, w in RENTNER_KEGEL_HOCH
] + [
    ("rentner_veraeusserungsgewinn", 8000000),           # 80.000 €, FB = 45.000 € (≤ 136k), netto = 35.000 €
    ("rentner_veraeusserungs_betriebsart", "gewerbe"),
]


def test_p16_4_gate_sperrt_ohne_bedingungen(base):
    """vg > 0 ohne gate-Felder → p16_4_gate_offen (fail-closed, kein stiller FB)."""
    _rent_anlegen(base, "p16a", _RENTNER_KEGEL_VG_BASIS)
    st, erg = _req(base, "GET", "/fall/p16a/ergebnis")
    assert st == 200
    assert erg["grund"] == "p16_4_gate_offen", f"grund={erg.get('grund')}"
    assert erg["zahl_cent"] is None


def test_p16_4_gate_sperrt_bei_alter_false(base):
    """vg > 0 + alter_55=False + erstmalig=True → sperr (S.1 nicht erfüllt)."""
    k = list(_RENTNER_KEGEL_VG_BASIS) + [("rentner_alter_55_oder_berufsunfaehig", False),
                                          ("rentner_freibetrag_erstmalig", True)]
    _rent_anlegen(base, "p16b", k)
    st, erg = _req(base, "GET", "/fall/p16b/ergebnis")
    assert erg["grund"] == "p16_4_gate_offen", f"grund={erg.get('grund')}"


def test_p16_4_gate_durchlaesst_mit_bedingungen(base):
    """vg > 0 + alter_55=True + erstmalig=True → bestaetigt, FB greift (zahl_cent > baseline)."""
    # Baseline ohne vg
    _rent_anlegen(base, "p16c0", RENTNER_KEGEL_HOCH)
    st, e0 = _req(base, "GET", "/fall/p16c0/ergebnis")
    _val("ergebnis", e0)
    assert e0["grund"] == "bestaetigt"
    z0 = e0["zahl_cent"]
    # vg + beide Bedingungen → bestaetigt
    k = list(_RENTNER_KEGEL_VG_BASIS) + [("rentner_alter_55_oder_berufsunfaehig", True),
                                          ("rentner_freibetrag_erstmalig", True)]
    _rent_anlegen(base, "p16c1", k)
    st, e1 = _req(base, "GET", "/fall/p16c1/ergebnis")
    _val("ergebnis", e1)
    assert e1["grund"] == "bestaetigt", f"grund={e1.get('grund')}"
    assert e1["zahl_cent"] > z0, "vg+FB muss ESt erhöhen (netto 35k€ zusätzlich)"
    diff = e1["zahl_cent"] - z0
    # Plausibilität: max Grenzsteuer 45% × 35.000 € = 15.750 €; 35k bei 0% = 0
    assert 0 < diff < 1575000, f"Delta {diff} außerhalb plausibler Band (0–15.750 €)"


def test_p16_4_gate_ignoriert_ohne_vg(base):
    """vg = 0 → Gate nicht aktiv (grund=bestaetigt)."""
    _rent_anlegen(base, "p16d", RENTNER_KEGEL_HOCH)
    st, erg = _req(base, "GET", "/fall/p16d/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt"
    assert isinstance(erg["zahl_cent"], int)


# ---- P2-#2: Rentner Person-B KV/PV bei Zusammenveranlagung (fehlte → Over-tax) ----

# Für rentner zusammen brauchen wir Partner-Rentenfelder, damit keine Guard-Sperre.
_RENTNER_ZUSAMMEN_BASIS = [
    (f, ("zusammen" if f == "veranlagung" else w)) for f, w in RENTNER_KEGEL_HOCH
] + [
    ("rentner_renten_art_partner", "gesetzliche_rente"),
    ("rentner_jahresrente_partner", 0),
    ("rentner_renten_beginn_jahr_partner", 2025),
    ("rentner_alter_bei_rentenbeginn_partner", 65),
]


def test_p2_nr2_erreichbarkeit_partner_kv_pv_post(base):
    """Person-B-KV/PV-Felder in rentner_gesamt POSTbar (201, nicht 400/422 totes Wiring)."""
    _rent_anlegen(base, "p2kv_e", _RENTNER_ZUSAMMEN_BASIS)
    # Dann POSTen wir die 3 Partner-Felder einzeln via _laie (signal_2 ok@feld)
    for feld, wert in [("basis_kv_partner", 200000), ("vorsorge_arbeitslosenversicherung_partner", 0), ("vorsorge_erwerbsunfaehigkeit_partner", 0), ("vorsorge_unfall_haftpflicht_partner", 0), ("vorsorge_rv_alt_mit_ueberschuss_partner", 0), ("vorsorge_rv_alt_ohne_ueberschuss_partner", 0),
                       ("mit_anspruch_auf_zuschuss_partner", True)]:
        st, resp = _req(base, "POST", "/fall/p2kv_e/event", _laie(feld, wert))
        assert st == 201, f"POST {feld}: {st} {resp.get('fehler', resp)}"


def test_p2_nr2_ring_differential_kv_pv_senkt_steuer(base):
    """Rentner zusammen MIT B-KV/PV → zahl_cent strikt niedriger als OHNE (Over-tax entfernt)."""
    _rent_anlegen(base, "p2k0", _RENTNER_ZUSAMMEN_BASIS)
    st, e0 = _req(base, "GET", "/fall/p2k0/ergebnis")
    _val("ergebnis", e0)
    assert e0["grund"] == "bestaetigt", f"grund={e0.get('grund')}"
    z0 = e0["zahl_cent"]

    k = list(_RENTNER_ZUSAMMEN_BASIS) + [
        ("basis_kv_partner", 200000),     # 2000 € × ~14-40% HB
        ("vorsorge_arbeitslosenversicherung_partner", 0), ("vorsorge_erwerbsunfaehigkeit_partner", 0), ("vorsorge_unfall_haftpflicht_partner", 0), ("vorsorge_rv_alt_mit_ueberschuss_partner", 0), ("vorsorge_rv_alt_ohne_ueberschuss_partner", 0),
        ("mit_anspruch_auf_zuschuss_partner", True)]
    _rent_anlegen(base, "p2k1", k)
    st, e1 = _req(base, "GET", "/fall/p2k1/ergebnis")
    _val("ergebnis", e1)
    assert e1["grund"] == "bestaetigt", f"grund={e1.get('grund')}"
    assert e1["zahl_cent"] < z0, f"B-KV/PV: {e1['zahl_cent']} >= {z0} (Over-tax noch da)"
    delta = z0 - e1["zahl_cent"]
    # HB max 2800 €, min 0 → Delta im plausiblen Band
    assert delta > 0 and delta < 280000, f"Delta {delta} außerhalb plausiblen Band"


def test_p35a_mitveranlagung_senkt_steuer_gesamt(base):
    """200k Gewinn + haushaltsnahe Aufwendungen 4000€ → bei Zusammenveranlagung nur halb so viel Abzug.
    Verifiziert: Bei zusammen veranlagt wird der Betrag halbiert (je Ehegatte nur die Hälfte).
    """
    catala = _catala_da()
    if not catala:
        pytest.skip("Catala nicht verfügbar")

    # Ohne haushaltsnahe Aufwendungen
    kegel_ohne = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_ohne.append(("hh_minijob_aufwendungen", 0))
    kegel_ohne.append(("hh_dienstleistungen", 0))
    kegel_ohne.append(("hh_handwerker_arbeitskosten", 0))
    kegel_ohne.append(("hh_in_eu_ewr", True))
    kegel_ohne.append(("hh_rechnung_unbar", True))
    _ges_anlegen(base, "p35a_no", kegel_ohne)
    st, ohne = _req(base, "GET", "/fall/p35a_no/ergebnis")
    _val("ergebnis", ohne)

    # Mit haushaltsnahe Aufwendungen 4000€ (400000 Cent) und zusammen veranlagt
    # (Mitveranlagung aktivieren)
    kegel_mit = list(kegel_ohne)
    # Replace the zero values with actual numbers
    new_kegel_mit = []
    for f, v in kegel_mit:
        if f == "hh_dienstleistungen":
            new_kegel_mit.append((f, 400000))
        elif f == "hh_minijob_aufwendungen":
            new_kegel_mit.append((f, 0))
        elif f == "hh_handwerker_arbeitskosten":
            new_kegel_mit.append((f, 0))
        else:
            new_kegel_mit.append((f, v))
    new_kegel_mit.append(("p35a_mitveranlagung", True))
    _ges_anlegen(base, "p35a_yes", new_kegel_mit)
    st, mit = _req(base, "GET", "/fall/p35a_yes/ergebnis")
    _val("ergebnis", mit)

    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        # Erwarteter Abzug: 400000 Cent Aufwand → 20% = 80000 Cent Erstattung bei Einzelveranlagung
        # Bei zusammen veranlagt wird der Höchstbetrag nur einmal gewährt → nur die Hälfte des Betrags (40000 Cent) wird insgesamt abgezogen
        assert delta == 40000, f"Erwarteter Steuervorteil bei Zusammenveranlagung: 40000 Cent, bekommen: {delta}"
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


# ===== §32b PROGRESSIONSVORBEHALT ======================================

def test_p32b_progressionsvorbehalt_erhoeht_steuer_gesamt(base):
    """P7.2 — Ring-Beweis: Progressionseinkünfte erhöhen die festzusetzende ESt.

    § 32b Abs. 1 EStG: steuerfreie Lohnersatzleistungen und (bei Freistellungs-DBA)
    ausländische Einkünfte bleiben selbst steuerfrei, heben aber den Steuersatz auf
    das übrige Einkommen. Der Accessor war getestet, der Ring nicht — ohne diesen
    Test bliebe unbemerkt, wenn p32b_progressionseinkuenfte im Bescheid versandet.
    """
    catala = _catala_da()
    kegel_ohne = _mit_gewinn(GESAMT_KEGEL_BASIS)
    _ges_anlegen(base, "p32b_ohne", kegel_ohne)
    st, ohne = _req(base, "GET", "/fall/p32b_ohne/ergebnis")
    _val("ergebnis", ohne)

    kegel_mit = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_mit.append(("p32b_progressionseinkuenfte", 3000000))     # 30.000 €
    _ges_anlegen(base, "p32b_mit", kegel_mit)
    st, mit = _req(base, "GET", "/fall/p32b_mit/ergebnis")
    _val("ergebnis", mit)

    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt"
        delta = mit["zahl_cent"] - ohne["zahl_cent"]
        assert delta > 0, (
            f"§32b bewegt die Steuer nicht (delta={delta}) — Progressionseinkünfte "
            f"erreichen den Bescheid nicht")
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


def test_p32b_null_progression_aendert_nichts_gesamt(base):
    """Negativtest: pe=0 darf die Steuer NICHT bewegen (sonst feuert der Zweig ungewollt)."""
    catala = _catala_da()
    _ges_anlegen(base, "p32b_n0", _mit_gewinn(GESAMT_KEGEL_BASIS))
    st, ohne = _req(base, "GET", "/fall/p32b_n0/ergebnis")

    kegel_null = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_null.append(("p32b_progressionseinkuenfte", 0))
    _ges_anlegen(base, "p32b_n1", kegel_null)
    st, null = _req(base, "GET", "/fall/p32b_n1/ergebnis")

    if catala:
        assert ohne["zahl_cent"] == null["zahl_cent"], (
            "pe=0 verändert die Steuer — der §32b-Zweig feuert, obwohl es nichts zu "
            "progressionieren gibt")


def test_p32b_hoehere_progression_hoehere_steuer_gesamt(base):
    """Monotonie: mehr Progressionseinkünfte → höherer Steuersatz → mehr Steuer.

    Fängt ein Vorzeichen- oder Einheiten-Vertauschen ab, das ein einzelner
    Ja/Nein-Vergleich durchgehen ließe.
    """
    catala = _catala_da()
    kegel_klein = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_klein.append(("p32b_progressionseinkuenfte", 1000000))    # 10.000 €
    _ges_anlegen(base, "p32b_k", kegel_klein)
    st, klein = _req(base, "GET", "/fall/p32b_k/ergebnis")

    kegel_gross = _mit_gewinn(GESAMT_KEGEL_BASIS)
    kegel_gross.append(("p32b_progressionseinkuenfte", 5000000))    # 50.000 €
    _ges_anlegen(base, "p32b_g", kegel_gross)
    st, gross = _req(base, "GET", "/fall/p32b_g/ergebnis")

    if catala:
        assert gross["zahl_cent"] > klein["zahl_cent"], (
            f"50k PE ({gross['zahl_cent']}) muss mehr Steuer erzeugen als 10k PE "
            f"({klein['zahl_cent']})")


# ===== §3 Nr. 72 PHOTOVOLTAIK =========================================

PV_GATE = [("pv_auf_gebaeude", True), ("pv_anzahl_einheiten", 1)]


def test_p3_nr72_pv_senkt_steuer_gesamt(base):
    """P7.3 — Ring-Beweis: steuerfreie PV-Einnahmen mindern die festzusetzende ESt.

    § 3 Nr. 72 EStG: Einnahmen aus Gebäude-PV bis 30 kWp/Einheit (max. 100 kWp) sind
    steuerfrei. Sie mindern den Gewinn vor der § 2-Summe.
    """
    catala = _catala_da()
    _ges_anlegen(base, "pv_ohne", _mit_gewinn(GESAMT_KEGEL_BASIS))
    st, ohne = _req(base, "GET", "/fall/pv_ohne/ergebnis")
    _val("ergebnis", ohne)

    kegel_mit = _mit_gewinn(GESAMT_KEGEL_BASIS) + PV_GATE + [
        ("pv_bruttoleistung_kwp", 12), ("pv_einnahmen", 500000)]      # 12 kWp, 5.000 €
    _ges_anlegen(base, "pv_mit", kegel_mit)
    st, mit = _req(base, "GET", "/fall/pv_mit/ergebnis")
    _val("ergebnis", mit)

    if catala:
        assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt"
        delta = ohne["zahl_cent"] - mit["zahl_cent"]
        assert delta > 0, (
            f"§3 Nr.72 bewegt die Steuer nicht (delta={delta}) — die Befreiung "
            f"erreicht den Bescheid nicht")
    else:
        assert ohne["zahl_cent"] is None or mit["zahl_cent"] is None


def test_p3_nr72_ueber_grenze_keine_befreiung_gesamt(base):
    """Freigrenze im Ring: 35 kWp bei 1 Einheit reißt die 30-kWp-Grenze → volle Steuer.

    Fängt ab, dass der Ring die Befreiung anteilig oder ungeprüft gewährt.
    """
    catala = _catala_da()
    _ges_anlegen(base, "pv_g0", _mit_gewinn(GESAMT_KEGEL_BASIS))
    st, ohne = _req(base, "GET", "/fall/pv_g0/ergebnis")

    kegel_ueber = _mit_gewinn(GESAMT_KEGEL_BASIS) + PV_GATE + [
        ("pv_bruttoleistung_kwp", 35), ("pv_einnahmen", 500000)]      # 35 kWp > 30
    _ges_anlegen(base, "pv_g1", kegel_ueber)
    st, ueber = _req(base, "GET", "/fall/pv_g1/ergebnis")

    if catala:
        assert ohne["zahl_cent"] == ueber["zahl_cent"], (
            "35 kWp bei 1 Einheit überschreitet die Freigrenze — es darf KEINE "
            "Steuerminderung geben")


def test_p3_nr72_freiflaeche_keine_befreiung_gesamt(base):
    """Ohne Gebäude-Merkmal keine Befreiung — § 3 Nr. 72 verlangt 'auf, an oder in Gebäuden'."""
    catala = _catala_da()
    _ges_anlegen(base, "pv_f0", _mit_gewinn(GESAMT_KEGEL_BASIS))
    st, ohne = _req(base, "GET", "/fall/pv_f0/ergebnis")

    kegel_frei = _mit_gewinn(GESAMT_KEGEL_BASIS) + [
        ("pv_auf_gebaeude", False), ("pv_anzahl_einheiten", 1),
        ("pv_bruttoleistung_kwp", 12), ("pv_einnahmen", 500000)]
    _ges_anlegen(base, "pv_f1", kegel_frei)
    st, frei = _req(base, "GET", "/fall/pv_f1/ergebnis")

    if catala:
        assert ohne["zahl_cent"] == frei["zahl_cent"], (
            "Freiflächenanlage ist nicht begünstigt — keine Steuerminderung erwartet")


# ===== §34c DBA PER-EINKUNFTSART (P7.1) ================================

def test_dba_einkunftsart_freistellung_statt_anrechnung_gesamt(base):
    """P7.1 — Ring-Beweis: die Einkunftsart entscheidet über die Methode.

    Polen steht pauschal auf Anrechnung. Für Arbeitslohn stellt Art. 24 Abs. 1 a
    DBA-PL aber frei — derselbe Sachverhalt muss je nach Einkunftsart anders rechnen.
    Freistellung heisst: keine Anrechnung der ausländischen Steuer, dafür
    Progressionsvorbehalt.
    """
    catala = _catala_da()
    dba_basis = [("dba_staat", "Polen"),
                 ("dba_gezahlte_auslaendische_steuer", 300000),     # 3.000 €
                 ("dba_auslaendische_einkuenfte", 4000000)]         # 40.000 €

    kegel_zinsen = _mit_gewinn(GESAMT_KEGEL_BASIS) + dba_basis + [("dba_einkunftsart", "zinsen")]
    _ges_anlegen(base, "dbaart_z", kegel_zinsen)
    st, zinsen = _req(base, "GET", "/fall/dbaart_z/ergebnis")
    _val("ergebnis", zinsen)

    kegel_lohn = _mit_gewinn(GESAMT_KEGEL_BASIS) + dba_basis + [
        ("dba_einkunftsart", "unselbstaendige_arbeit")]
    _ges_anlegen(base, "dbaart_l", kegel_lohn)
    st, lohn = _req(base, "GET", "/fall/dbaart_l/ergebnis")
    _val("ergebnis", lohn)

    if catala:
        assert zinsen["grund"] == "bestaetigt" and lohn["grund"] == "bestaetigt"
        assert zinsen["zahl_cent"] != lohn["zahl_cent"], (
            "Zinsen (Anrechnung) und Arbeitslohn (Freistellung) ergeben dieselbe Steuer — "
            "die Einkunftsart erreicht das Methoden-Routing nicht")
    else:
        assert zinsen["zahl_cent"] is None or lohn["zahl_cent"] is None


def test_dba_ohne_einkunftsart_bleibt_pauschal_gesamt(base):
    """Rückwärtskompatibilität: ohne dba_einkunftsart rechnet der Ring wie bisher."""
    catala = _catala_da()
    dba_basis = [("dba_staat", "Polen"),
                 ("dba_gezahlte_auslaendische_steuer", 300000),
                 ("dba_auslaendische_einkuenfte", 4000000)]

    _ges_anlegen(base, "dbaart_o", _mit_gewinn(GESAMT_KEGEL_BASIS) + dba_basis)
    st, ohne_art = _req(base, "GET", "/fall/dbaart_o/ergebnis")

    kegel_zinsen = _mit_gewinn(GESAMT_KEGEL_BASIS) + dba_basis + [("dba_einkunftsart", "zinsen")]
    _ges_anlegen(base, "dbaart_z2", kegel_zinsen)
    st, zinsen = _req(base, "GET", "/fall/dbaart_z2/ergebnis")

    if catala:
        assert ohne_art["zahl_cent"] == zinsen["zahl_cent"], (
            "Polen steht pauschal auf Anrechnung und Zinsen ebenfalls — beide Fälle "
            "müssen identisch rechnen")


def test_dba_nicht_ausgearbeitetes_land_ignoriert_einkunftsart_gesamt(base):
    """Für die zehn noch nicht adjudizierten Länder darf die Einkunftsart nichts ändern."""
    catala = _catala_da()
    dba_basis = [("dba_staat", "Frankreich"),
                 ("dba_gezahlte_auslaendische_steuer", 300000),
                 ("dba_auslaendische_einkuenfte", 4000000)]

    _ges_anlegen(base, "dbaart_f0", _mit_gewinn(GESAMT_KEGEL_BASIS) + dba_basis)
    st, ohne_art = _req(base, "GET", "/fall/dbaart_f0/ergebnis")

    kegel_lohn = _mit_gewinn(GESAMT_KEGEL_BASIS) + dba_basis + [
        ("dba_einkunftsart", "unselbstaendige_arbeit")]
    _ges_anlegen(base, "dbaart_f1", kegel_lohn)
    st, mit_art = _req(base, "GET", "/fall/dbaart_f1/ergebnis")

    if catala:
        assert ohne_art["zahl_cent"] == mit_art["zahl_cent"], (
            "Frankreich ist nicht per-Einkunftsart adjudiziert — die Angabe darf die "
            "Berechnung nicht verändern")


def test_dba_oesterreich_freistellung_wirkt_im_ring_gesamt(base):
    """REGRESSION: Österreich ist ein Freistellungs-DBA — der Ring muss freistellen.

    Vorher traf der Enum-Wert "Oesterreich" die ISO-basierte DBA_METHOD_MAP nicht und
    fiel auf den Anrechnungs-Default. Freistellung heisst: keine Anrechnung der
    ausländischen Steuer, dafür Progressionsvorbehalt — also eine ANDERE Steuer als
    bei einem Anrechnungs-Land mit sonst identischen Werten.
    """
    catala = _catala_da()
    werte = [("dba_gezahlte_auslaendische_steuer", 300000),      # 3.000 €
             ("dba_auslaendische_einkuenfte", 4000000)]          # 40.000 €

    _ges_anlegen(base, "dba_at", _mit_gewinn(GESAMT_KEGEL_BASIS)
                 + [("dba_staat", "Oesterreich")] + werte)
    st, at = _req(base, "GET", "/fall/dba_at/ergebnis")
    _val("ergebnis", at)

    _ges_anlegen(base, "dba_fr", _mit_gewinn(GESAMT_KEGEL_BASIS)
                 + [("dba_staat", "Frankreich")] + werte)
    st, fr = _req(base, "GET", "/fall/dba_fr/ergebnis")
    _val("ergebnis", fr)

    if catala:
        assert at["grund"] == "bestaetigt" and fr["grund"] == "bestaetigt"
        assert at["zahl_cent"] != fr["zahl_cent"], (
            "Österreich (Freistellung) und Frankreich (Anrechnung) ergeben dieselbe "
            "Steuer — die Länder-Methode erreicht den Ring nicht")
    else:
        assert at["zahl_cent"] is None or fr["zahl_cent"] is None


# ===== § 20 Abs. 9 Sparer-Pauschbetrag hängt an `veranlagung` ==========

def test_sparer_pauschbetrag_folgt_der_veranlagungsart(base):
    """REGRESSION: der Sparer-Pauschbetrag richtet sich nach der Veranlagungsart — und NUR danach.

    Bis 2026-07-30 gab es ein zweites Feld kap_zusammenveranlagung, das dieselbe Frage stellte.
    Bei veranlagung=einzel + Flag=true verdoppelte es den Pauschbetrag (2.000 statt 1.000 €),
    ohne das Partner-Kapital zu addieren: 250 € zu wenig Steuer bei 4.000 € Kapital. Das Feld
    ist entfernt; dieser Test pinnt, dass der Einzelveranlagungs-Fall den einfachen
    Pauschbetrag bekommt und niemand ihn von aussen verdoppeln kann.
    """
    catala = _catala_da()

    def _kegel(kapital):
        ersatz = {"kap_kapitalertraege": kapital, "kein_kap": False}
        return [(f, ersatz.get(f, w)) for f, w in _mit_gewinn(GESAMT_KEGEL_BASIS)]

    # 1.000 € Kapital = genau der einfache Sparer-Pauschbetrag → nichts zu versteuern
    _ges_anlegen(base, "spb_1000", _kegel(100000))
    st, bei_1000 = _req(base, "GET", "/fall/spb_1000/ergebnis")
    _val("ergebnis", bei_1000)

    # 2.000 € Kapital: bei Einzelveranlagung sind 1.000 € steuerpflichtig
    _ges_anlegen(base, "spb_2000", _kegel(200000))
    st, bei_2000 = _req(base, "GET", "/fall/spb_2000/ergebnis")
    _val("ergebnis", bei_2000)

    if catala:
        assert bei_1000["grund"] == "bestaetigt" and bei_2000["grund"] == "bestaetigt"
        delta = bei_2000["zahl_cent"] - bei_1000["zahl_cent"]
        # 1.000 € über dem Pauschbetrag × 25 % Abgeltungsteuer = 250 €
        assert delta == 25000, (
            f"Bei Einzelveranlagung muss der zweite Tausender voll besteuert werden "
            f"(1.000 € × 25 % = 250 €), gemessen: {delta} ct")
    else:
        assert bei_1000["zahl_cent"] is None or bei_2000["zahl_cent"] is None
