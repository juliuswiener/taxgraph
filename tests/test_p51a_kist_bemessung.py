"""KiSt §51a Bemessung: Angestellter kirchensteuerpflichtig, kein Kapital.

BUG (2026-08-06): gesamt slot_fn Z.1146 gibt `kap_st_total` als `est_mit_fb`,
catala_kist erwartet aber die volle ESt OHNE §32d-Kapital. Bei kap=0 ist
kap_st_total=0 → KiSt=0 für JEDEN kirchensteuerpflichtigen Angestellten.

Fix: est_mit_fb = ESt ohne Kapitalanteil (seit 2026-09-26 die zusätzlich
kreditfreie Basis solz_info["est_ohne_p35"], Abs. 2 S. 3; davor
est_roh_ohne_kap, die SolZ-Naht).
§32d-Abgeltung-KiSt wird über e/(4+k) in §32d Abs.1 S.3-4 geregelt, nicht
separat.

Test muss auf aktuellem Code ROT sein (KiSt=0 trotz Konfession).
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
from _kegel import kegel_fuer  # noqa: E402 — ein Bauer fuer alle Scheiben (tests/_kegel.py)


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _req(base, method, path, body=None, erwarte=None):
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
        assert status == erwarte, f"erwarte={erwarte}, erhalten={status} {method} {path}"
    elif status >= 400:
        raise AssertionError(f"Fehler {status} {method} {path}: {content}")
    return status, content


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


def _catala_da():
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


# Pflicht-Kegel gesamt (api_constants: VV_GESAMT_FELDER + veranlagung + bruttoarbeitslohn
# + EP_FELDER + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + AN_GESAMT_FLAGS)
GESAMT_KEGEL_BASE = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 2400000),  # 24000 EUR
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
]


def test_kist_bemessung_ohne_kapital(base):
    """Angestellter, roem.-kath., NRW, kein Kapital → KiSt = 9% der ESt.

    MUSS ROT sein auf aktuellem Code (kap_st_total=0 → KiSt=0).
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    kegel += [("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
              ("kist_bundesland", "nordrhein_westfalen")]
    _anlegen(base, "kist0", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kist0/ergebnis")
    assert st == 200
    kist_cent = erg.get("kist_cent")
    assert kist_cent is not None, f"kist_cent fehlt: {erg}"
    assert kist_cent > 0, (
        f"KiSt = 0 obwohl kirchensteuerpflichtig (roem.-kath., NRW). "
        f"BUG: gesamt slot_fn gibt kap_st_total statt ESt. {erg}")
    # Erwartet: 9 % der ESt (§51a, NRW → 9%)
    est_cent = erg["zahl_cent"]
    expected = est_cent // 100 * 9   # 9 % von ESt (EUR → KiSt CENT)
    assert kist_cent == expected, (
        f"KiSt {kist_cent} != {expected} (9% von {est_cent} CENT)")


def test_kist_bemessung_ohne_kapital_24000(base):
    """24.000 EUR Lohn, roem.-kath., NRW → KiSt ~20988 cent (209,88 EUR).

    Explizite Werte aus Mains Messung: ESt 233.200 cent, KiSt 20.988 cent.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    kegel += [("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
              ("kist_bundesland", "nordrhein_westfalen")]
    _anlegen(base, "kist24", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kist24/ergebnis")
    assert st == 200
    kist_cent = erg.get("kist_cent")
    assert kist_cent is not None, f"kist_cent fehlt: {erg}"
    # Rot-Erwartung: aktuell kap_st_total=0 → KiSt=0
    assert kist_cent > 0, f"ROT bestätigt: KiSt=0 bei 24.000 EUR Lohn, kath., NRW"


def test_kist_mit_kapital(base):
    """Angestellter 24.000 Lohn + 5.000 Kapital, roem.-kath. NRW.

    KiSt = §51a auf Nicht-Kapital-ESt + §32d Abs.1 S.3-5 Kapital-KiSt (e/(4+k)).
    5.000 Kapital − 1.000 Sparer-PB = 4.000 steuerpflichtig.
    Abgeltung (25 %) = 1.000,00 EUR → e/(4+k) mit k=9 = 977,99 EUR (CENT-Floor).
    KiSt auf Kapital = 977,99 × 9 % = 88,01 EUR = 8.801 CENT.
    KiSt auf Nicht-Kapital = 209,88 EUR (§51a, 9 % von 2.332).
    Summe = 209,88 + 88,01 = 297,89 EUR = 29.789 CENT.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    for i, (k, v) in enumerate(kegel):
        if k == "kein_kap":
            kegel[i] = (k, False)
        elif k == "kap_kapitalertraege":
            kegel[i] = (k, 500000)  # 5000 EUR in cent
    kegel += [("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
              ("kist_bundesland", "nordrhein_westfalen")]
    _anlegen(base, "kistkap", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kistkap/ergebnis")
    assert st == 200
    kist_cent = erg.get("kist_cent")
    assert kist_cent is not None, f"kist_cent fehlt: {erg}"
    # KiSt = 297,89 EUR (Nicht-Kapital 209,88 + Kapital 88,01)
    assert kist_cent == 29789, (
        f"KiSt {kist_cent} != 29789 (Nicht-Kapital 20988 + Abgeltung-KiSt 8801). erg={erg}")
    # ESt = 3.309,00 EUR (2.332 + 977,99 nach e/(4+k), statt 2.332 + 1.000 = 3.332)
    assert erg["zahl_cent"] == 330900, (
        f"ESt {erg['zahl_cent']} != 330900. erg={erg}")


# ===== KAP Stufe 3 (Zeile 41, Kz E1905101, § 32d Abs. 1 S. 2/4-5): q-Anrechnung =====

def test_kist_mit_kapital_und_q(base):
    """Wie test_kist_mit_kapital, zusaetzlich 100 EUR anrechenbare auslaendische Steuer (q).

    § 32d Abs. 1 S. 4-5: e=4.000, q=100, k=9% -> (4.000-400)/4,09 = 880,19 EUR (CENT-Floor
    88.019). KiSt auf Kapital = 88.019 × 9% = 792,171 -> 7.921 CENT (Floor).
    KiSt gesamt = 20.988 (Nicht-Kapital, unveraendert) + 7.921 = 28.909 CENT.
    ESt = 2.332 + 880 = 3.212 EUR = 321.200 CENT (vs. 330.900 CENT ohne q, s. test_kist_mit_kapital).
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    for i, (k, v) in enumerate(kegel):
        if k == "kein_kap":
            kegel[i] = (k, False)
        elif k == "kap_kapitalertraege":
            kegel[i] = (k, 500000)  # 5000 EUR in cent
    kegel += [("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
              ("kist_bundesland", "nordrhein_westfalen"),
              ("kap_q_auslaendische_steuer", 10000)]  # 100 EUR
    _anlegen(base, "kistkapq", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kistkapq/ergebnis")
    assert st == 200
    assert erg.get("kist_cent") == 28909, (
        f"KiSt {erg.get('kist_cent')} != 28909 (Nicht-Kapital 20988 + Kapital-mit-q 7921). erg={erg}")
    assert erg["zahl_cent"] == 321200, (
        f"ESt {erg['zahl_cent']} != 321200 (2.332 + 880 nach q-Anrechnung, vs. 330900 ohne q). erg={erg}")


def test_kist_mit_kapital_q_deckel(base):
    """q weit ueber kap_st (25%-Abgeltung auf Kapital, VOR der KiSt-Formel) -> Deckel (§ 32d
    Abs. 5 S. 3) greift, q wird NICHT voll angerechnet.

    Kapitalertraege 5.001 EUR (bewusst NICHT durch 4 teilbar nach Sparer-PB-Abzug) statt der
    500.000-Cent-Rundzahl der Nachbartests: kap_st = e*25//100 ist ein EUR-Floor von e/4, bei
    e=4.000 (Nachbartests) trifft der Deckel q_eur=kap_st EXAKT auf 4*kap_st=e -> Zaehler der
    KiSt-Formel (e-4q) wird IMMER 0, ob mit oder ohne Deckel (das aeussere max(0,...) faengt
    ein unbegrenztes q genauso ab) — an dieser Stelle waere der Deckel nicht vom bereits
    vorhandenen max(0,...)-Floor zu unterscheiden. Mit e=4.001 bleibt bei korrektem Deckel ein
    EUR-Rundungsrest (e-4*kap_st=1) uebrig, den ein FEHLENDER Deckel nicht erzeugen kann.

    e=4.001, kap_st=4.001*25//100=1.000 EUR (Abgeltung, CENT-Floor). q_deklariert=5.000 EUR
    wird gedeckelt auf min(5.000, 1.000)=1.000. (4.001-4×1.000)/4,09 = 1/4,09 = 0,2445 EUR
    (CENT-Floor 24). KiSt auf Kapital = 24×9% = 2,16 -> 2 CENT (Floor). KiSt gesamt =
    20.988+2 = 20.990 CENT. kap_st_k = 24 CENT // 100 = 0 EUR -> ESt = 2.332+0 = 233.200 CENT
    (unveraendert ggue. q=0, weil der 24-CENT-Rest EUR-seitig abrundet — ESt allein
    unterscheidet Deckel/kein-Deckel hier NICHT).
    OHNE Deckel waere q_eur=5.000 > e=4.001 -> (4.001-20.000) negativ -> max(0,...) = 0 ->
    KiSt-Anteil Kapital = 0 CENT -> KiSt gesamt 20.988 CENT (NICHT 20.990). Der 2-CENT-
    Unterschied in kist_cent ist der Beweis: der Deckel laesst den gesetzlich noch
    geschuldeten Rest (Abs. 5 S. 3 begrenzt auf die tatsaechlich entfallende deutsche Steuer,
    NICHT auf 0) stehen, ein fehlender Deckel wuerde ihn ueber den max(0,...)-Floor
    verschlucken.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    for i, (k, v) in enumerate(kegel):
        if k == "kein_kap":
            kegel[i] = (k, False)
        elif k == "kap_kapitalertraege":
            kegel[i] = (k, 500100)  # 5001 EUR in cent (bewusst nicht durch 4 teilbar, s.o.)
    kegel += [("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
              ("kist_bundesland", "nordrhein_westfalen"),
              ("kap_q_auslaendische_steuer", 500000)]  # 5000 EUR, weit ueber kap_st=1000
    _anlegen(base, "kistkapdeckel", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kistkapdeckel/ergebnis")
    assert st == 200
    assert erg["zahl_cent"] == 233200, (
        f"ESt {erg['zahl_cent']} != 233200 (kap_st_k rundet EUR-seitig auf 0, s. Docstring). "
        f"erg={erg}")
    assert erg.get("kist_cent") == 20990, (
        f"KiSt {erg.get('kist_cent')} != 20990 (Nicht-Kapital 20988 + 2 CENT Deckel-Rest — "
        f"bei fehlendem Deckel waeren es 20988, der max(0,...)-Floor verschluckt dann auch "
        f"den gesetzlich geschuldeten Rest). erg={erg}")


def test_kap_q_ohne_kirchensteuerpflicht(base):
    """§ 32d Abs. 1 S. 1+2: q-Anrechnung gilt AUCH ohne Kirchensteuerpflicht (S. 2 ist
    grammatisch unabhaengig von S. 3 — Fund neben der team-lead-Vorgabe, die nur den
    KiSt-pflichtigen (e-4q)/(4+k)-Zweig verlangte). Netto-neuer Code-Pfad.

    e=4.000, Abgeltung 25% = 1.000 EUR (kap_st==abgeltung, konfession=keine -> kein KiSt-Zweig).
    q=300 EUR -> kap_st_k = max(0, 1.000-300) = 700 EUR. ESt = 2.332+700 = 3.032 EUR =
    303.200 CENT (vs. 3.332 EUR = 333.200 CENT ohne q).

    Die Zeile ("kist_konfession", "keine") unten ist NICHT Beiwerk und darf nicht wieder
    wegfallen: bis 2026-08-28 setzte dieser Test das Feld gar nicht und nannte das trotzdem
    „keine Konfession" — GESAMT_KEGEL_BASE fuehrt es nicht. Damit hing der ganze Test an der
    Vorgabe, die der eigentliche Defekt war, und er waere beim Fix rot geworden. Ausfuehrlich
    an der Zeile selbst.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    for i, (k, v) in enumerate(kegel):
        if k == "kein_kap":
            kegel[i] = (k, False)
        elif k == "kap_kapitalertraege":
            kegel[i] = (k, 500000)
    kegel += [("kap_q_auslaendische_steuer", 30000)]  # 300 EUR, keine Kirchensteuerpflicht
    # Die Konfession wird hier AUSDRUECKLICH auf "keine" gesetzt (2026-08-28). Vorher fehlte sie
    # ganz — GESAMT_KEGEL_BASE fuehrt das Feld nicht — und der Test nannte das „keine Konfession".
    # Das war dieselbe Verwechslung wie im Produktionscode: unbeantwortet ist nicht dasselbe wie
    # „gehoert keiner Kirche an". Seit die Vorgabe "keine" weg ist (_kist_konfession in
    # bescheid_zweige.py), meldet /ergebnis fuer den unbeantworteten Fall gar keine Kirchensteuer
    # mehr statt einer gerechneten 0. Mit der ausdruecklichen Antwort prueft dieser Test wieder
    # das, was sein Name verspricht — die q-Anrechnung OHNE Kirchensteuerpflicht. Am ESt-Zweig
    # aendert das nichts: "keine" nimmt denselben Weg wie zuvor die Vorgabe (zahl_cent bleibt).
    kegel += [("kist_konfession", "keine")]
    _anlegen(base, "kapqnokist", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kapqnokist/ergebnis")
    assert st == 200
    assert erg["zahl_cent"] == 303200, (
        f"ESt {erg['zahl_cent']} != 303200 (2.332+700 nach q-Anrechnung ohne KiSt-Pflicht, "
        f"vs. 333200 ohne q). erg={erg}")
    assert erg.get("kist_cent") == 0, (
        f"KiSt sollte gerechnete 0 sein (Konfession ausdruecklich 'keine'). erg={erg}")


def test_ohne_konfession_keine_erfundene_null(base):
    """Unbeantwortete Konfession -> kist_cent ist None (nicht rechenbar), nicht 0.

    GEMESSEN 2026-08-28 am Live-Fall serie-verheiratet-1kind-handwerker: Bundesland, gezahlte
    (580 EUR) und erstattete Kirchensteuer bestaetigt, die Konfession nie beantwortet. /ergebnis
    meldete kist_cent = 0 — bei roemisch-katholisch waeren es 105336 Cent. Die Null war nicht
    gerechnet, sondern kam aus der Vorgabe `f.get("kist_konfession", {}).get("wert", "keine")`,
    die aus „unbeantwortet" ein „gehoert keiner Kirche an" machte.

    Der Beweis, dass 0 hier gelogen war, stand im SELBEN Antwortobjekt: mobilitaetspraemie_cent
    war None. Deshalb pruefen wir beide zusammen — sie muessen auf dieselbe Lage dieselbe Antwort
    geben. Die festzusetzende ESt bleibt davon unberuehrt; falsch war allein die Kirchensteuer.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    # Alles zur Kirchensteuer ausser der Mitgliedschaft selbst — wie im gemessenen Fall.
    kegel += [("kist_bundesland", "nordrhein_westfalen"), ("kist_gezahlt", 58000),
              ("kist_erstattet", 0)]
    _anlegen(base, "kistoffen", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kistoffen/ergebnis")
    assert st == 200
    assert erg["kist_cent"] is None, (
        f"kist_cent = {erg['kist_cent']!r} statt None: ohne beantwortete Konfession ist die "
        f"Kirchensteuer nicht rechenbar, und eine 0 behauptet das Gegenteil. erg={erg}")
    assert erg["mobilitaetspraemie_cent"] is None, (
        "Kontrollwert: die Praemie war in diesem Fall schon immer None. Ist sie es nicht mehr, "
        f"misst dieser Test nicht mehr, was er soll. erg={erg}")
    assert erg["zahl_cent"] is not None and erg["zahl_cent"] > 0, (
        f"Die Einkommensteuer selbst muss weiter herauskommen — nur die Kirchensteuer fehlt. "
        f"erg={erg}")


def test_mit_konfession_kommt_die_kirchensteuer(base):
    """Gegenprobe zum Test darueber, an DEMSELBEN Fall: mit beantworteter Mitgliedschaft steht
    eine Kirchensteuer da. Ohne diese Haelfte waere nicht gezeigt, dass None an der fehlenden
    Antwort haengt und nicht daran, dass dieser Fall gar keine Kirchensteuer erzeugt.

    Der gemessene Betrag ist der, den der Live-Fall verloren hatte."""
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE)
    kegel += [("kist_bundesland", "nordrhein_westfalen"), ("kist_gezahlt", 58000),
              ("kist_erstattet", 0), ("kist_konfession", "roemisch-katholisch")]
    _anlegen(base, "kistda", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kistda/ergebnis")
    assert st == 200
    assert erg["kist_cent"] is not None and erg["kist_cent"] > 0, (
        f"Mit roem.-kath. muss eine Kirchensteuer herauskommen. erg={erg}")
    # 9 % der festzusetzenden ESt (NRW), dieselbe Massstabsteuer wie in den Tests oben.
    assert erg["kist_cent"] == erg["zahl_cent"] // 100 * 9, (
        f"KiSt {erg['kist_cent']} != 9 % von {erg['zahl_cent']} CENT. erg={erg}")


# ===== RENTNER-KiSt (Befund 1: Z.1526 gibt est_mit_fb = est_raw + kap_st) =====

# RENTNER_KEGEL (api_constants Z.133): RENTNER_22 + RENTNER_33B + veranlagung
# + AN_GESAMT_FLAGS + VOR_FELDER + KV_PV_FELDER
RENTNER_KEGEL_BASE = [
    ("veranlagung", "einzel"),
    ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 2000000),  # 20.000 EUR
    ("rentner_renten_beginn_jahr", 2023), ("rentner_alter_bei_rentenbeginn", 65),
    ("rentner_grad_der_behinderung", 0), ("rentner_hilflos_blind_taubblind", False),
    ("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False), ("rentner_hinterbliebenenbezuege", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("einkuenfte_gewinn", 0), ("rentner_veraeusserungsgewinn", 0),
    # gewinn_betriebsart absichtlich NICHT gesetzt: "keine" war nie ein gültiger enum-Wert
    # (bindung_an_gesamt.yaml erlaubt nur gewerbe/selbstaendig/land_forst). kein_gewinn=True
    # trägt die Abwesenheit schon; das Feld hat keinen signatur_slot (reine Anlage-G/S/L-Weiche
    # fürs XML, kein Catala-Input), absent verhält sich wie "keine" — kein Rechenwert ändert sich.
    ("geburtsjahr", 1960),
    ("gewst_hebesatz", 0), ("gewst_messbetrag", 0), ("verlustvortrag_bestand", 0),
    ("rentner_rentenfreibetrag", 0),
]


def _rentner_kegel(mit_kapital: bool):
    kegel = list(RENTNER_KEGEL_BASE)
    kegel += [("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
              ("kist_bundesland", "nordrhein_westfalen")]
    if mit_kapital:
        for i, (k, v) in enumerate(kegel):
            if k == "kein_kap":
                kegel[i] = (k, False)
            elif k == "kap_kapitalertraege":
                kegel[i] = (k, 5000000)  # 50.000 EUR in cent
    return kegel


def test_kist_rentner_ohne_kapital(base):
    """Rentner 20.000 Rente, roem.-kath., NRW, kein Kapital → KiSt = 9% von est_raw.

    Baseline: ESt 1.605,00 → KiSt 144,45 (9%).
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    _anlegen(base, "rkist0", "rentner_gesamt", _rentner_kegel(mit_kapital=False))
    st, erg = _req(base, "GET", "/fall/rkist0/ergebnis")
    assert st == 200
    kist_cent = erg.get("kist_cent")
    assert kist_cent is not None, f"kist_cent fehlt: {erg}"
    assert kist_cent > 0, f"KiSt = 0 obwohl kirchensteuerpflichtig. {erg}"
    # Erwartet: 9% von ESt ohne Kapital (= est_raw). est=1605 EUR → KiSt 144,45 EUR
    est_cent = erg["zahl_cent"]
    expected = est_cent // 100 * 9
    assert kist_cent == expected, (
        f"KiSt {kist_cent} != {expected} (9% von {est_cent} CENT)")


def test_kist_rentner_mit_kapital(base):
    """Rentner 20.000 Rente + 50.000 Kapital, roem.-kath. NRW.

    KiSt = §51a auf Rente (est_raw=1.605) + §32d Abs.1 S.3-5 Kapital-KiSt.
    Kapital 50.000 − Sparer-PB 1.000 = 49.000. Abgeltung 25 % = 12.250 EUR.
    e/(4+k) mit k=9: 12.250 × 400 // 409 = 11.980 EUR (CENT-Floor).
    KiSt auf Kapital = 11.980 × 9 % = 1.078,23 EUR (CENT-Floor).
    KiSt auf Rente (§51a) = 1.605 × 9 % = 144,45 EUR.
    Summe KiSt = 144,45 + 1.078,23 = 1.222,68 EUR = 122.268 CENT.
    ESt = 1.605 + 11.980 = 13.585 EUR.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    _anlegen(base, "rkistkap", "rentner_gesamt", _rentner_kegel(mit_kapital=True))
    st, erg = _req(base, "GET", "/fall/rkistkap/ergebnis")
    assert st == 200
    kist_cent = erg.get("kist_cent")
    assert kist_cent is not None, f"kist_cent fehlt: {erg}"
    assert kist_cent == 122268, (
        f"KiSt {kist_cent} != 122268 (Rente 14445 + Kapital 107823). erg={erg}")
    assert erg["zahl_cent"] == 1358500, (
        f"ESt {erg['zahl_cent']} != 1358500 (1.605 + 11.980). erg={erg}")


def test_kist_rentner_mit_kapital_solz(base):
    """Rentner 20.000 + 50.000 Kapital: SolZ-Hauptbasis (§3 Abs.3 S.1) bleibt 0, aber der
    separate Kapital-SolZ (§3 Abs.3 S.2, 5,5% ohne Freigrenze) kommt seit dem
    rentner-solz-kap-st-tracking-Fix (BACKLOG, api.py — Setzstelle solz_info_r["kap_st"]
    analog gesamt-Ring Z. 1282) korrekt oben drauf.

    §32d Abs.1 S.3-5 (e/(4+k)) ändert die Kapitalsteuer (kap_st_k=11.980 EUR), NICHT die
    SolZ-Formel selbst: 5,5% × 11.980 EUR = 658,90 EUR = 65.890 Cent. Hauptbasis-Anteil
    bleibt 0 (est_mit_fb=13.585€ − kap_st_k=11.980€ = 1.605€, weit unter Freigrenze).
    KiSt = 1.222,68 EUR, ESt = 13.585 EUR (nach e/(4+k)-Ermäßigung von 270 EUR).

    Vor dem Fix stand hier `solz_cent == 0` — das war der gefrorene Bug-Zustand (kap_st lief
    nie in catala_solz, s. test_solz_ring_rentner_grenzfall_mit_kapital für die Kernmessung).
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    _anlegen(base, "rkistsolz", "rentner_gesamt", _rentner_kegel(mit_kapital=True))
    st, erg = _req(base, "GET", "/fall/rkistsolz/ergebnis")
    assert st == 200
    # Kapital-SolZ: 5,5% × 11.980€ = 65.890 Cent (§3 Abs.3 S.2, additiv ohne Freigrenze)
    assert erg.get("solz_cent") == 65890, f"SolZ != 65.890 Cent: {erg}"
    # KiSt = 122.268 CENT (Rente 14445 + Kapital 107823)
    assert erg.get("kist_cent") == 122268, (
        f"KiSt {erg.get('kist_cent')} != 122268. erg={erg}")
    # ESt = 13.585 EUR (est_raw 1.605 + kap_st_k 11.980 nach e/(4+k))
    assert erg.get("zahl_cent") == 1358500, (
        f"ESt {erg.get('zahl_cent')} != 1358500. erg={erg}")


def _anlegen(base, fid, scheibe, kegel):
    st, _ = _req(base, "POST", "/fall", {"scheibe": scheibe, "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201
    kegel = kegel_fuer(scheibe, dict(kegel))
    for feld, wert in kegel:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201


# ===== §51a Abs. 2 S. 3: die §35-GewSt-Ermäßigung mindert die KiSt-Basis NICHT =====
#
# § 51a Abs. 2 S. 3 EStG: "§ 35 ist bei der Ermittlung der festzusetzenden Einkommensteuer
# nach Satz 1 nicht anzuwenden." Die SolZ-Basis (§ 3 Abs. 2 SolZG) kennt keinen solchen
# Ausschluss, dort mindert § 35 über § 2 Abs. 6 EStG mit. Geprüft wird gegen Bundesrecht;
# ob ein Landes-KiStG abweicht, ist offen.
# Quelle: sources/gesetze-im-internet/estg_p51a_2026-09-26.txt, solzg_1995_p3_2026-07-11.txt.
#
# Fall wie test_ring_regression_kampagne.test_p34c_mit_p35_kombi_senkt_steuer_kumulativ
# (kbo/kbg): 200.000 EUR Gewerbegewinn; mit GewSt Messbetrag 1.000 EUR, Hebesatz 400 %
# → § 35-Ermäßigung 4.000 EUR. Dazu roem.-kath., NRW (9 %).

P35_GEWST = [("gewst_messbetrag", 100000), ("gewst_hebesatz", 400)]  # 1.000 EUR, 400 %


def _p35_gesamt_kegel(mit_gewst: bool):
    """GESAMT_KEGEL_BASE ohne Lohn + 200.000 EUR Gewerbegewinn (= kampagne _mit_gewinn)."""
    kegel = [(k, {"bruttoarbeitslohn": 0, "kein_gewinn": False}.get(k, v))
             for k, v in GESAMT_KEGEL_BASE]
    kegel += [("einkuenfte_gewinn", 20000000), ("gewinn_betriebsart", "gewerbe"),
              ("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
              ("kist_bundesland", "nordrhein_westfalen")]
    return kegel + (P35_GEWST if mit_gewst else [])


def _p35_rentner_kegel(mit_gewst: bool):
    """Rentner 20.000 EUR (_rentner_kegel: roem.-kath., NRW) + 200.000 EUR Gewerbegewinn."""
    ersetzt = {"kein_gewinn": False, "einkuenfte_gewinn": 20000000}
    if mit_gewst:
        ersetzt.update(P35_GEWST)
    kegel = [(k, ersetzt.get(k, v)) for k, v in _rentner_kegel(mit_kapital=False)]
    return kegel + [("gewinn_betriebsart", "gewerbe")]


def _p35_differenz(base, fid, scheibe, kegel_fn):
    """ohne GewSt minus mit GewSt, je für zahl_cent, solz_cent und kist_cent (CENT)."""
    erg = {}
    for mit_gewst in (False, True):
        f = f"{fid}{int(mit_gewst)}"
        _anlegen(base, f, scheibe, kegel_fn(mit_gewst))
        st, erg[mit_gewst] = _req(base, "GET", f"/fall/{f}/ergebnis")
        assert st == 200
        assert erg[mit_gewst]["grund"] == "bestaetigt", f"{f}: {erg[mit_gewst]}"
        assert erg[mit_gewst].get("kist_cent") is not None, f"kist_cent fehlt: {erg[mit_gewst]}"
    return {k: erg[False][k] - erg[True][k] for k in ("zahl_cent", "solz_cent", "kist_cent")}


def _p35_pruefe(d):
    # Vorbedingung: § 35 wirkt (4.000 EUR). Kontrolle: der SolZ sinkt um 5,5 % davon.
    assert d["zahl_cent"] == 400000, f"§ 35 senkt die ESt nicht um 4.000 EUR: {d}"
    assert d["solz_cent"] == 22000, f"SolZ sinkt nicht um 5,5 % von 4.000 EUR: {d}"
    assert d["kist_cent"] == 0, (
        f"KiSt sinkt um {d['kist_cent']} CENT mit der § 35-Ermäßigung; "
        f"§ 51a Abs. 2 S. 3 EStG nimmt § 35 aus der KiSt-Basis. {d}")


def test_kist_p35_mindert_basis_nicht_gesamt(base):
    """Gesamt: § 35 senkt ESt um 4.000 EUR und SolZ um 220 EUR, die KiSt bleibt gleich.

    MUSS ROT sein auf aktuellem Code: die KiSt sinkt um 9 % von 4.000 EUR = 36.000 CENT mit.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    _p35_pruefe(_p35_differenz(base, "kistp35g", "gesamt", _p35_gesamt_kegel))


def test_kist_p35_mindert_basis_nicht_rentner(base):
    """Rentner-Zweig (rentner_gesamt): dieselbe Prüfung wie im gesamt-Zweig.

    MUSS ROT sein auf aktuellem Code: die KiSt sinkt um 9 % von 4.000 EUR = 36.000 CENT mit.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    _p35_pruefe(_p35_differenz(base, "kistp35r", "rentner_gesamt", _p35_rentner_kegel))


# ===== Deckel-Fall: § 35 ist nur so weit wirksam, wie er unter den Catala-Deckel passt =====
#
# Die Regel rechnet wirksame_ermaessigung = min(steuerermaessigungen, verfuegbare_steuer)
# (rules/estg/p32a/einkommensteuertarif.catala_en). Treffen § 35a/§ 35c und § 35 zusammen
# auf eine kleine Steuer, ist der § 35-Kredit nur TEILWEISE wirksam. Ein Zurückaddieren des
# vollen Kredits auf die reduzierte ESt überzeichnet die KiSt-Basis dann.
#
# Gemessen im Ring (2026-09-26, VZ 2025, Gewinn 20.000 EUR als einziges Einkommen,
# § 35a-Dienstleistungen 5.000 EUR → 1.000 EUR, § 35 GewSt: Deckel 3 kappt 4.000 auf 1.630 EUR):
#   tarifliche ESt 1.630 EUR; Ermäßigungen 1.000 (§ 35a) + 1.630 (§ 35) = 2.630
#   ohne GewSt: ESt 630 EUR, KiSt 5.670 CENT
#   mit  GewSt: ESt 0 EUR,   KiSt 5.670 CENT
#   § 35 wirksam nur 630 EUR statt 1.630 EUR — der Catala-Deckel greift.
# Der Fall trennt die NEU gerechnete Basis von beiden falschen Wegen (Mutation 2026-09-26):
#   alte Basis (est_raw = ESt MIT § 35): KiSt 0 statt 5.670.
#   Zurückaddieren (est_raw + p35_credit = 0 + 1.630 EUR): KiSt 14.670 statt 5.670.
# Bis 2026-09-26 fehlte hh_in_eu_ewr: § 35a wirkte nicht (ESt 1.630 EUR mit und ohne
# § 35a-Felder), der Catala-Deckel griff nie, und das Zurückaddieren blieb gruen.

# 5.000 EUR Dienstleistungen → § 35a 20 % = 1.000 EUR. Das SUM-Feld (hh_dienstleistungen),
# nicht der Einzelposten: _hh_summe faellt ohne Instanz auf das Sum-Feld zurueck, ein
# gesetzter Einzelposten allein ergaebe 0 (gemessen 2026-09-26). hh_in_eu_ewr: ohne das Feld
# gibt catala_p35a_haushaltsnahe 0 (§ 35a Abs. 4).
P35A_DECKEL = [("hh_dienstleistungen", 500000), ("hh_rechnung_unbar", True), ("hh_in_eu_ewr", True)]
# § 35a 1.000 + § 35 1.630 gegen eine tarifliche Steuer von 1.630 EUR: die Ermäßigungen
# uebersteigen die Steuer, der Deckel greift — und § 35a allein deckt sie NICHT.


def _p35_deckel_kegel(mit_gewst: bool):
    """Gewinn 20.000 EUR als einziges Einkommen + § 35a 1.000 EUR + § 35 4.000 EUR.

    Kein Lohn: die tarifliche Steuer muss klein bleiben, sonst greift der Deckel nicht.
    """
    kegel = [(k, {"bruttoarbeitslohn": 0, "kein_gewinn": False}.get(k, v))
             for k, v in GESAMT_KEGEL_BASE]
    kegel += [("einkuenfte_gewinn", 2000000), ("gewinn_betriebsart", "gewerbe"),
              ("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
              ("kist_bundesland", "nordrhein_westfalen")] + P35A_DECKEL
    return kegel + (P35_GEWST if mit_gewst else [])


def test_kist_deckel_35_ist_nur_teilweise_wirksam(base):
    """Unter dem Catala-Deckel: die KiSt-Basis folgt der WIRKLICHEN Wirkung, nicht dem Kredit.

    Die KiSt darf sich mit/ohne GewSt nicht ändern (Abs. 2 S. 3). Sie darf aber auch nicht
    auf die volle Kreditsumme reagieren: wirksam ist nur der Teil unter dem Deckel. Ein
    Zurückaddieren des vollen Kredits wäre hier um 1.000 EUR zu hoch (s. Kopfkommentar).
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    d = _p35_differenz(base, "kistdeckel", "gesamt", _p35_deckel_kegel)
    assert d["kist_cent"] == 0, (
        f"KiSt sinkt um {d['kist_cent']} CENT mit der § 35-Ermäßigung, obwohl § 51a "
        f"Abs. 2 S. 3 EStG sie aus der KiSt-Basis nimmt. {d}")
    # Vorbedingung des Falls: der Catala-Deckel MUSS greifen, sonst prüft dieser Test nichts
    # Neues gegenüber test_kist_p35_mindert_basis_nicht_gesamt. 163.000 CENT = der § 35-Kredit
    # nach seinem eigenen Deckel 3; nur darunter ist er teilweise wirksam.
    assert d["zahl_cent"] < 163000, (
        f"Vorbedingung kaputt: § 35 wirkt hier voll ({d['zahl_cent']} CENT statt < 163.000). "
        f"Ohne greifenden Deckel sagt dieser Test nichts über die Teilwirksamkeit aus. {d}")
    # Und die KiSt-Basis ist genau die ESt OHNE § 35 — nicht die reduzierte (dann KiSt 0) und
    # nicht die zurückaddierte (dann KiSt 14.670).
    assert d["kist_cent"] == 0 and d["zahl_cent"] == 63000, (
        f"Erwartet: ESt-Wirkung 63.000 CENT (Deckel), KiSt unveraendert. {d}")


# ===== § 32b bleibt in der KiSt-Basis, § 35 nicht =====
#
# § 51a Abs. 2 EStG nimmt nur § 3 Nr. 40/§ 3c (S. 2) und § 35 (S. 3) aus der Basis. Der
# Progressionsvorbehalt (§ 32b) bleibt drin: Basis ist die ESt, wie sie festzusetzen wäre
# (S. 1). Der § 32b-Wrapper im Ring läuft NACH catala_est. Eine Basis, die vor ihm abgegriffen
# wird, verliert ihn still — so die erste Fassung des § 35-Fixes (gemessen 2026-09-26:
# KiSt 20.988 statt 31.833 CENT).
# Erwartete Werte: HEAD 82729e8 (vor dem § 35-Fix), dort trug die Basis § 32b.
# Quelle: sources/gesetze-im-internet/estg_p51a_2026-09-26.txt.

P32B_PE = [("p32b_progressionseinkuenfte", 1000000)]  # 10.000 EUR Lohnersatzleistungen


def test_kist_p32b_bleibt_in_basis_gesamt(base):
    """Lohn 24.000 EUR + 10.000 EUR Progressionseinkünfte: die KiSt folgt der ESt MIT § 32b.

    ESt 3.537 EUR → KiSt 9 % = 318,33 EUR = 31.833 CENT. Ohne § 32b in der Basis wären es
    9 % von 2.332 EUR (ESt ohne Progressionseinkünfte) = 20.988 CENT.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = list(GESAMT_KEGEL_BASE) + [("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
                                       ("kist_bundesland", "nordrhein_westfalen")] + P32B_PE
    _anlegen(base, "kistpe", "gesamt", kegel)
    st, erg = _req(base, "GET", "/fall/kistpe/ergebnis")
    assert st == 200 and erg["grund"] == "bestaetigt", erg
    kurz = {k: erg[k] for k in ("zahl_cent", "solz_cent", "kist_cent")}
    assert erg["zahl_cent"] == 353700, f"Vorbedingung: § 32b hebt die ESt auf 3.537 EUR. {kurz}"
    assert erg["kist_cent"] == 31833, (
        f"KiSt {erg['kist_cent']} != 31.833 CENT (9 % von 3.537 EUR). 20.988 heisst: "
        f"die KiSt-Basis hat § 32b verloren. {kurz}")


def _ring_gesamt(kegel):
    """(zahl_cent, solz_cent, kist_cent) direkt aus dem gesamt-Ring, ohne /ergebnis.

    Mit Absicht am Endpunkt vorbei: _an_gesamt_sperrgrund sperrt § 32b zusammen mit § 35
    (p32b_kombi_offen), /ergebnis liefert dort keine Zahl. Gemessen wird der Ring selbst.
    """
    felder = {k: {"wert": v, "zustand": "bestaetigt"} for k, v in kegel}
    solz, extras = [None], {}
    bf = API._bescheid_fn("festzusetzende_est_gesamt", 2025, API.TR.lade_bindung(), felder,
                          store=None, nur_bestaetigt=True, solz_container=solz, extras=extras)
    assert bf is not None, "bescheid_fn gab None (catala?)"
    return bf(dict(kegel)), solz[0], extras.get("kist_cent")


def _ring_rentner(kegel):
    """(zahl_cent, solz_cent, kist_cent) direkt aus dem rentner-Ring — wie _ring_gesamt."""
    felder = {k: {"wert": v, "zustand": "bestaetigt"} for k, v in kegel}
    solz, extras = [None], {}
    bf = API._bescheid_fn("festzusetzende_est_rentner", 2025, API.TR.lade_bindung(), felder,
                          store=None, nur_bestaetigt=True, solz_container=solz, extras=extras)
    assert bf is not None, "bescheid_fn gab None (catala?)"
    return bf(dict(kegel)), solz[0], extras.get("kist_cent")


def test_kist_p35_und_p32b_zugleich_gesamt(base):
    """Beide Ausnahmen in einem Fall: § 35 raus aus der Basis, § 32b drin.

    Gewinn 200.000 EUR, § 35 4.000 EUR, dazu 10.000 EUR Progressionseinkünfte. Erwartet mit
    und ohne GewSt: KiSt 662.319 CENT = 9 % von 73.591 EUR (ESt mit § 32b, ohne § 35). Falsch
    wären 657.648 (§ 32b verloren, 9 % von 73.072 EUR) und 626.319 (§ 35 in der Basis).
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    ohne = _p35_gesamt_kegel(mit_gewst=False) + P32B_PE
    mit = _p35_gesamt_kegel(mit_gewst=True) + P32B_PE
    # Kalibrierung: ohne GewSt ist /ergebnis offen — der direkte Ring muss dort dieselben
    # Zahlen liefern wie der Endpunkt.
    _anlegen(base, "kistpe35", "gesamt", ohne)
    st, erg = _req(base, "GET", "/fall/kistpe35/ergebnis")
    assert st == 200 and erg["grund"] == "bestaetigt", erg
    z0, s0, k0 = _ring_gesamt(ohne)
    assert (z0, s0, k0) == (erg["zahl_cent"], erg["solz_cent"], erg["kist_cent"]), (
        f"Ring {(z0, s0, k0)} weicht von /ergebnis ab: {erg}")
    z1, s1, k1 = _ring_gesamt(mit)
    # Vorbedingung: § 35 wirkt mit 4.000 EUR, der SolZ sinkt um 5,5 % davon.
    assert (z0 - z1, s0 - s1) == (400000, 22000), (
        f"§ 35 wirkt nicht wie erwartet: ESt {z0}->{z1}, SolZ {s0}->{s1}")
    assert (k0, k1) == (662319, 662319), (
        f"KiSt ohne/mit GewSt {k0}/{k1}, erwartet 662.319/662.319 CENT. 657.648 heisst: "
        f"§ 32b fehlt in der Basis; 626.319 mit GewSt heisst: § 35 steckt in der Basis.")


# ===== Rentner-Ring: § 32b-Zuschlag gehoert in die KiSt-Basis =====
#
# § 51a Abs. 2 S. 1 nimmt die ESt, wie sie festzusetzen waere — der § 32b-Zuschlag gehoert
# dazu. Im Rentner-Ring laeuft die Kapital-KiSt getrennt ueber kap_st_k (Abs. 1 S. 3-5), also
# darf ins tarifliche Delta nur tarifliche_32b - tarifliche_pre32b, sonst zaehlte Kapital
# doppelt. Entscheidung kistensteuer-basis-ohne-gewerbesteuer-ermaessigung, Weg (c).

def test_kist_rentner_p32b_delta_mit_kapital(base):
    """Rentner 20.000 EUR + Kapital 50.000 EUR + 10.000 EUR Progressionseinkuenfte.

    Handrechnung (VZ 2025, gemessen im Ring, nicht aus dem Gedaechtnis):
      tarifliche ESt ohne Progressionseinkuenfte ....... 1.605 EUR
      tarifliche ESt mit  10.000 EUR Progressionseink. . 2.836 EUR
      Delta (der § 32b-Zuschlag) ...................... 1.231 EUR
      KiSt-Basis ohne Zuschlag 1.605 EUR → 9 % ......... 14.445 CENT
      KiSt-Basis mit  Zuschlag 2.836 EUR → 9 % ......... 25.524 CENT
      dazu der § 32d-Kapital-Nachtrag (getrennt) ...... 107.823 CENT
    Erwartet ohne Progressionseinkuenfte 122.268 CENT, mit 133.347 CENT — die Differenz ist
    genau der Zuschlag, 9 % von 1.231 EUR = 11.079 CENT.

    Falsch waere 122.268 auch MIT Progressionseinkuenften: dann fehlt der § 32b-Zuschlag in
    der KiSt-Basis (genau der Zustand vor diesem Fix, gemessen 2026-09-26). Der Zuschlag darf
    die Kapital-KiSt nicht doppelt zaehlen — die laeuft ueber kap_st_k, ins Delta geht nur der
    tarifliche Teil.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    ohne_pe, _, k_ohne = _ring_rentner(_rentner_kegel(mit_kapital=True))
    mit_pe, _, k_mit = _ring_rentner(_rentner_kegel(mit_kapital=True) + P32B_PE)
    # Vorbedingung: § 32b hebt die ESt um 1.231 EUR (135.850 → 148.160 CENT).
    assert mit_pe - ohne_pe == 123100, (
        f"§ 32b hebt die ESt nicht um 1.231 EUR: {ohne_pe} → {mit_pe}")
    assert (k_ohne, k_mit - k_ohne) == (122268, 11079), (
        f"KiSt ohne/mit Progressionseinkuenften {k_ohne}/{k_mit}, erwartet 122.268 und "
        f"+11.079 CENT. Ein unveraendertes {k_ohne} heisst: der § 32b-Zuschlag fehlt in der "
        f"KiSt-Basis (Zustand vor diesem Fix, gemessen 2026-09-26).")


def test_kist_rentner_p32b_delta_ohne_kapital(base):
    """Kontrollfall ohne Kapital (main: HEAD-Paritaet 25.524 ct), sonst derselbe Fall.

    Dieselbe Rente, dieselben 10.000 EUR Progressionseinkuenfte, kein Kapital: die KiSt ist
    9 % der ESt MIT § 32b-Zuschlag = 9 % von 2.836 EUR = 25.524 CENT.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    kegel = ([x for x in RENTNER_KEGEL_BASE]
             + [("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
                ("kist_bundesland", "nordrhein_westfalen")] + P32B_PE)
    zahl, _, kist = _ring_rentner(kegel)
    assert zahl == 283600, f"ESt-Vorbedingung: § 32b hebt auf 2.836 EUR. {zahl}"
    assert kist == 25524, (
        f"KiSt {kist} != 25.524 CENT (9 % von 2.836 EUR). 14.445 heisst: die Basis hat "
        f"§ 32b verloren.")


def test_kist_rentner_p32b_delta_guenstigerpruefung(base):
    """§ 32d Abs. 6 gewinnt: das Kapital steckt im Tarif — der Delta zaehlt es nicht doppelt.

    Rente 4.000 EUR + Kapital 50.000 EUR: hier gewinnt die tarifliche Veranlagung
    (kap_guenstiger_gewonnen=True), kist_kap_cent ist 0 — es gibt keinen Abgeltungs-Nachtrag,
    der doppelt anfallen koennte. Der § 32b-Zuschlag darf deshalb NUR ueber den tariflichen
    Teil wirken.

    Gemessen 2026-09-26 (Rente 4.000 EUR, mit/ohne Progressionseinkuenfte):
      ohne PE: kist 0 CENT;  mit PE: kist 684 CENT.
    Vor dem Fix war kist mit PE ebenfalls 0 — der Zuschlag fehlte ganz.
    """
    if not _catala_da():
        pytest.skip("Catala nicht verfügbar")
    def _kegel_guenstiger(mit_pe):
        k = []
        for f, v in RENTNER_KEGEL_BASE:
            if f == "rentner_jahresrente":
                v = 400000          # 4.000 EUR
            elif f == "kein_kap":
                v = False
            elif f == "kap_kapitalertraege":
                v = 5000000          # 50.000 EUR
            k.append((f, v))
        k += [("kist_konfession", "roemisch-katholisch"), ("kist_gezahlt", 0), ("kist_erstattet", 0),
              ("kist_bundesland", "nordrhein_westfalen")]
        return k + (P32B_PE if mit_pe else [])

    ohne_pe, _, k_ohne = _ring_rentner(_kegel_guenstiger(False))
    mit_pe, _, k_mit = _ring_rentner(_kegel_guenstiger(True))
    # GEPINNT, nicht gebaendert: k_mit > k_ohne waere kein Pin — ein zu weit wirkender
    # Delta (Kapital doppelt gezaehlt) bliebe darunter gruen.
    assert (k_ohne, k_mit) == (0, 684), (
        f"KiSt ohne/mit Progressionseinkuenften {k_ohne}/{k_mit}, erwartet 0/684 CENT. "
        f"0/0 heisst: der § 32b-Zuschlag fehlt in der Basis. Deutlich mehr als 684 hiesse: "
        f"der Delta zaehlt das Kapital doppelt.")
