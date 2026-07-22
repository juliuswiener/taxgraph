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
    ("basis_kv_pv", 0), ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0), ("kap_zusammenveranlagung", False),
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
    ("basis_kv_pv", 0), ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
]

AN_KEGEL_HOCH = [
    ("bruttoarbeitslohn", 20000000),
    ("veranlagung", "einzel"),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv_pv", 0), ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
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
    ("basis_kv_pv", 0), ("weitere_vorsorgeaufwendungen", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0), ("kap_zusammenveranlagung", False),
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
