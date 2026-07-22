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
