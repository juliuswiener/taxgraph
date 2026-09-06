"""§23 EStG Private Veräußerungsgeschäfte — Accessor-Fidelity (3 Snapshots). NULL LLM.

3 promoted+inert Snapshots (p23_veraeusserungsgewinn, p23_freigrenze, p23_3_verlusttopf):
Jede Regel muss JEDEN test_seed EXAKT reproduzieren. EURO (integer).
"""

import os, sys
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))
import runner as R  # noqa: E402

sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))
import api as API  # noqa: E402
import audit        # noqa: E402


# -- p23_veraeusserungsgewinn (3 seeds: 200000-150000-5000=45000 / 100000-120000-3000=-23000 / 50000-50000-0=0)

def test_gewinn_seed_0():
    r = R.catala_p23_veraeusserungsgewinn({"veraeusserungspreis": 200000, "anschaffungs_herstellungskosten": 150000,
                                            "werbungskosten": 5000})
    assert r == 45000


def test_gewinn_seed_1_verlust():
    r = R.catala_p23_veraeusserungsgewinn({"veraeusserungspreis": 100000, "anschaffungs_herstellungskosten": 120000,
                                            "werbungskosten": 3000})
    assert r == -23000


def test_gewinn_seed_2_null():
    r = R.catala_p23_veraeusserungsgewinn({"veraeusserungspreis": 50000, "anschaffungs_herstellungskosten": 50000,
                                            "werbungskosten": 0})
    assert r == 0


# -- p23_freigrenze (4 seeds: 5000→5000 / 999→0 / 1000→1000 / 0→0)

def test_freigrenze_seed_0():
    assert R.catala_p23_freigrenze({"gesamtgewinn": 5000}) == 5000


def test_freigrenze_seed_1_unter():
    assert R.catala_p23_freigrenze({"gesamtgewinn": 999}) == 0


def test_freigrenze_seed_2_schwelle():
    """Wächter: 1000 ist die Grenze — AB 1000 fällt der VOLLE Gewinn an."""
    assert R.catala_p23_freigrenze({"gesamtgewinn": 1000}) == 1000


def test_freigrenze_seed_3_null():
    assert R.catala_p23_freigrenze({"gesamtgewinn": 0}) == 0


# -- p23_3_verlusttopf (4 seeds: 2000-500=1500 / 500-800=0 / 1000-1000=0 / 0-0=0)

def test_verlusttopf_seed_0():
    assert R.catala_p23_verlusttopf({"gewinn_pvg": 2000, "verlust_pvg": 500}) == 1500


def test_verlusttopf_seed_1_verlust_uebersteigt():
    assert R.catala_p23_verlusttopf({"gewinn_pvg": 500, "verlust_pvg": 800}) == 0


def test_verlusttopf_seed_2_gleich():
    assert R.catala_p23_verlusttopf({"gewinn_pvg": 1000, "verlust_pvg": 1000}) == 0


def test_verlusttopf_seed_3_beide_null():
    assert R.catala_p23_verlusttopf({"gewinn_pvg": 0, "verlust_pvg": 0}) == 0


# -- Accessor-Glue (_p23_ansonsten_einkuenfte via /ergebnis): die obigen Tests rufen die
# Catala-Funktionen DIREKT mit Hand-Dicts — nie über EM.instanzen()/den Store/api.py. Sie
# hätten den Bug in _p23_ansonsten_einkuenfte (fehlendes .get("wert") beim Store-Zugriff)
# nie gefangen. Dieser Test geht über den echten Ring-Pfad (/fall -> /event -> /ergebnis).

def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": {"typ": "laie_eingabe"}, "signal_2": "laie_bestaetigt"}}


def test_p23_ueber_ring_accessor(tmp_path, monkeypatch):
    """ZUSTANDSFESTSCHREIBUNG, kein Sollverhalten: kein_p23_verkauf=False (Nutzer HAT den Verkauf
    und sagt es) liegt in fremd_arten fuer die Scheibe "gesamt" (api_constants.py) und sperrt dort
    JEDEN §23-Verkauf, bevor der unten beschriebene Ring-Pfad ueberhaupt erreicht wird
    (bescheid_deklaration.py::_an_gesamt_sperrgrund, Zweig "fremd_arten" -> grund=
    "einkunftsart_nicht_ring_faehig"). Ob ein erklaerter §23-Verkauf auf dieser Scheibe rechnen
    soll, ist eine Produktfrage und entscheidet Julius (2026-08-31) -- dieser Test haelt nur den
    HEUTE gemessenen Zustand fest, kein Wunschverhalten. Wird der Zweig irgendwann geoeffnet, muss
    dieser Test bewusst umgeschrieben werden, nicht nur nachgezogen werden.

    Ehemals: „§23-Instanz mit Preis/AK/WK über den echten Store/Ring-Pfad" — Kegel unten baut
    genau diesen Pfad, ABER der fremd_arten-Zweig sperrt VOR dem Ring-Zugriff
    (_p23_ansonsten_einkuenfte via EM.instanzen()); der Zugriffsweg selbst wird durch DIESEN Test
    nicht mehr erreicht/geprueft. Ob ein anderer Test ihn noch durchlaeuft: nein, gemessen 2026-08-31
    -- test_p23_vorlaeufige_instanz_nicht_in_bestaetigter_rechnung unten setzt kein_p23_verkauf gar
    nicht (Betraege bleiben "vorlaeufig", _bestaetigt_wert liefert dafuer None, keine Sperre und kein
    Ring-Zugriff mit BESTAETIGTEN Werten). Die Ring-Zugriffsfidelity dieses Pfads hat damit aktuell
    KEINEN gruenen Test mehr -- offene Luecke, nicht in diesem Auftrag repariert.
    """
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    fid = "p23ring"
    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201, r
    kegel = [
        ("veranlagung", "einzel"), ("bruttoarbeitslohn", 6000000),
        ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
        ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0),
        ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0),
        ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
        ("mit_anspruch_auf_zuschuss", False),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
        ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
        ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
        # kein_p23_verkauf=False: der Nutzer HAT den unten deklarierten Verkauf und sagt es. Das
        # sperrt NICHT (nur) ueber flag_check.flag_widersprueche() -- fremd_arten
        # (bescheid_deklaration.py, gespeist aus api_constants.py "gesamt") sperrt bereits vorher
        # bei wert is False, s. Docstring oben.
        ("kein_p23_verkauf", False),
        # §23-Instanz: 200.000€ − 150.000€ − 5.000€ = 45.000€ Gewinn (Cent) -- wird durch die
        # fremd_arten-Sperre unten NICHT mehr erreicht, bleibt zur Dokumentation des Kegels stehen.
        ("p23_veraeusserungspreis", 20000000),
        ("p23_anschaffung_herstellungskosten", 15000000),
        ("p23_werbungskosten", 500000),
        ("p23_veraeusserungs_typ", "grundstueck"),
    ]
    for feld, wert in kegel:
        st, r = API.event(fid, _laie(feld, wert))
        assert st == 201, f"{feld}={wert}: {st} {r}"
    st, erg = API.ergebnis(fid)
    assert st == 200, erg
    # Zustandsfestschreibung (s. Docstring): fremd_arten sperrt JEDEN erklaerten §23-Verkauf auf
    # dieser Scheibe, bevor der Ring-Pfad ueberhaupt gerechnet wird. Konkreter Wert statt "scheitert
    # irgendwie" -- haelt auch dann noch, wenn ein anderer Sperrgrund zufaellig zuerst greift.
    assert erg["grund"] == "einkunftsart_nicht_ring_faehig", erg
    assert erg["zahl_cent"] is None, erg


def test_p23_vorlaeufige_instanz_nicht_in_bestaetigter_rechnung(tmp_path, monkeypatch):
    """Zwei-Signal-Regel (K2): eine p23-Instanz im Zustand 'vorlaeufig' darf NICHT in die
    bestätigte Rechnung (/ergebnis, nur_bestaetigt=True) einfließen — sonst bewegt eine
    unbestätigte Angabe die festgesetzte Steuer OHNE Confirm. _p23_ansonsten_einkuenfte
    nimmt den Parameter nur_bestaetigt entgegen, wertet ihn aber nirgends aus (kein
    `if not nur_bestaetigt or inst["zustand"] == "bestaetigt"` wie an den 9 anderen
    EM.instanzen()-Stellen in dieser Datei) — die vorläufige Instanz fließt daher genauso
    ein wie eine bestätigte. Referenz: 1392400 ct (derselbe Kegel OHNE die p23-Instanz)."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    fid = "p23vorlaeufig"
    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201, r
    basis = [
        ("veranlagung", "einzel"), ("bruttoarbeitslohn", 6000000),
        ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
        ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
        ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
        ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
        ("basis_kv", 0), ("basis_pv", 0),
        ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0),
        ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0),
        ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
        ("mit_anspruch_auf_zuschuss", False),
        ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
        ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
        ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ]
    for feld, wert in basis:
        st, r = API.event(fid, _laie(feld, wert))
        assert st == 201, f"{feld}={wert}: {st} {r}"
    vorlaeufig = dict(_laie("p23_veraeusserungspreis", 20000000), zustand="vorlaeufig",
                      signal={"signal_1": {"typ": "laie_eingabe"}, "signal_2": None})
    st, r = API.event(fid, vorlaeufig)
    assert st == 201, r
    for feld, wert in (("p23_anschaffung_herstellungskosten", 15000000),
                       ("p23_werbungskosten", 500000)):
        ev = dict(_laie(feld, wert), zustand="vorlaeufig",
                  signal={"signal_1": {"typ": "laie_eingabe"}, "signal_2": None})
        st, r = API.event(fid, ev)
        assert st == 201, r
    ev = dict(_laie("p23_veraeusserungs_typ", "grundstueck"), zustand="vorlaeufig",
              signal={"signal_1": {"typ": "laie_eingabe"}, "signal_2": None})
    st, r = API.event(fid, ev)
    assert st == 201, r

    st, erg = API.ergebnis(fid)
    assert st == 200, erg
    assert erg["grund"] == "bestaetigt", erg
    assert erg["zahl_cent"] == 1392400, (
        f"vorlaeufige p23-Instanz fliesst in die bestaetigte Rechnung ein: {erg['zahl_cent']} ct "
        "statt 1392400 ct (Basis ohne p23). K2-Zwei-Signal-Verletzung.")
