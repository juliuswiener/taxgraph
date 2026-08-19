"""Durchstich-Gate ueber den HTTP-Endpunkt POST /fall/{id}/einreichen: ECHTES erzeuge_xml,
ECHTES amtliches checkESt — eine Ebene ueber tests/test_checkest_durchstich.py.

Warum es das gibt (Mock-Naht in test_einreichen.py)
-----------------------------------------------------
`tests/test_einreichen.py` deckt den Endpunkt mit 13 Tests ab. JEDER von ihnen mockt
entweder `erzeuge_xml`, `checkest_gate.validate`, oder beide weg — Inventur in
`reports/adjudikation/einreichen_mock_naht_2026-08-09.md`. Das ist fuer das, was diese
Tests je einzeln behaupten (Fehlerbehandlung, Response-Mapping, Kz-Dict-Aufbau in
`_mit_ring_werten`/`EM.deklariere`), korrekt und bewusst gewaehlt. Aber es bedeutet: kein
einziger bestehender Test in dieser Datei prueft, ob das, was der Endpunkt WIRKLICH aus
einem echten Fall baut, das amtliche checkESt-Plugin ueberhaupt erreicht. Genau diese
Luecke liess `rc=610301200` (Namespace-Praefix-Ablehnung) fuer das gesamte Projekt
unentdeckt, bis `tests/test_checkest_durchstich.py` sie auf FUNKTIONS-Ebene schloss
(969edd5, Fix cebb228). Dieser Test schliesst dieselbe Luecke auf ENDPUNKT-Ebene: er geht
durch den echten HTTP-Server, den echten `einreichen()`-Code (Sperrgrund-Pruefung,
`_mit_ring_werten`, `EM.deklariere`, `EX.erzeuge_xml`, `CE.validate`, HTTP-Status/grund/
klasse-Mapping) — nichts davon ist hier gemockt.

Bauart: QUALITATIV, keine zweite Ratsche
------------------------------------------
`tests/test_checkest_durchstich.py` haelt bereits GENAU EINE Ratsche auf die Zahl amtlicher
Beanstandungen (RESTFEHLER_EINZEL/RESTFEHLER_ZUSAMMEN). Eine zweite Ratsche hier wuerde
denselben Fortschritt doppelt zaehlen. Dieser Test prueft nur die Falsch-Gruen-Frage:
kommt das XML bis zur Plausibilitaetspruefung durch, und meldet der Endpunkt das ehrlich
(rc, klasse, HTTP-Status, plausibel-Flag)? `rc=610301200` liefert einen LEEREN Fehlerpuffer
und sieht damit aus wie "keine Beanstandungen" — ist aber ein Abbruch VOR der Pruefung.
`CE.klassifiziere_rc()` haelt das auseinander; der Endpunkt darf das NIE als Erfolg (200,
plausibel=True) melden.

Fixtur-Hinweis: KAP-Detailfelder bewusst weggelassen (nur `kein_kap=True`, keine explizite
kap_*=0-Deklaration) — ein Feld explizit auf 0 zu deklarieren loest bei checkESt einen
eigenen "Angabegrund fehlt"-Fehler aus (Anlage-KAP-Adjudikation, noch offen), das blosse
Weglassen unter `kein_kap=True` nicht. Das reicht, um die Deklaration vollstaendig zu
machen und echtes XML zu erzeugen.

Ueberspringt sauber ohne ERiC/Hersteller-ID (credential-freies CI). Die ID wird nie
geloggt — Skip-Mechanik uebernommen aus `tests/test_checkest_durchstich.py::_hid`.

Endpunkt-Ratsche + Delta-Wache (2026-08-10)
--------------------------------------------
Team-lead-Befund: der Live-Endpunkt uebergibt `abgabefaehig`/`absender_*` NICHT an
`erzeuge_xml` (api.py:2398) — die Funktions-Ratsche in `test_checkest_durchstich.py` misst
also einen saubereren Pfad, als ein echter Nutzer ihn heute faehrt. Bis das nachgezogen ist
(BACKLOG `endpunkt-naht-abgabepfad`, nicht mein Bereich — produkt/haut/api.py gehoert
team-lead), braucht dieser Pfad eine EIGENE Ratsche, sonst kann er unbemerkt weiter
auseinanderlaufen. Zwei Zahlen ohne Beziehung zueinander waren genau der Zustand, der den
Vorsatz-Fortschritt auf Funktionsebene am Endpunkt unsichtbar liess. Deshalb zusaetzlich zur
Endpunkt-Ratsche ein dritter Test, der die Differenz zur Funktions-Ratsche explizit benennt
und rot wird, wenn sie sich aendert — er braucht kein ERiC (reine Konstanten-Arithmetik),
bleibt also auch credential-frei gruen und faengt Drift selbst dort, wo die beiden Ratschen
unabhaengig voneinander editiert werden.

Scheiben-Korrektur (2026-08-10, team-lead-Entscheidung): der erste Anlauf hing die
Stammdaten an scheibe="an_gesamt" — die lehnt sie am Endpunkt mit 400 ab, weil
`api_constants.py` STAMMDATEN_FELDER nur in "gesamt"/"rentner_gesamt" fuehrt (an_gesamt hat
auch kein `gesamt_guard`, ist also strukturell keine abgabefaehige Scheibe, sondern eine
Arbeitnehmer-Teilrechnung). Fixtur + beide Tests laufen deshalb auf scheibe="gesamt" — misst
den Pfad, den ein Nutzer tatsaechlich einreicht, auf Kosten einer groesseren Fixtur.
"""
from __future__ import annotations

import json
import os
import re
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/import", "golden", "produkt/store", "elster"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API                # noqa: E402
import server as SRV             # noqa: E402
import audit                      # noqa: E402
import checkest_gate as CE       # noqa: E402


def _hid() -> str | None:
    """Hersteller-ID aus der Umgebung, sonst aus der gitignoreten .env. Nie loggen."""
    hid = os.environ.get("ELSTER_HERSTELLER_ID")
    if hid:
        return hid
    pfad = os.path.join(ROOT, ".env")
    if not os.path.exists(pfad):
        return None
    for zeile in open(pfad, encoding="utf-8"):
        if zeile.startswith(("ELSTER_HERSTELLER_ID=", "HERSTELLER_ID=")):
            return zeile.split("=", 1)[1].strip().strip('"').strip("'") or None
    return None


_HID = _hid()
_ERIC_DA = bool(_HID) and os.path.isdir(
    os.environ.get("ERIC_DIR", os.path.expanduser("~/02_Software/eric")))

braucht_eric = pytest.mark.skipif(
    not _ERIC_DA,
    reason="ERiC oder Hersteller-ID fehlt — amtliche Pruefung nicht lauffaehig "
           "(credential-freies CI)")


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _req(base, method, path, body=None, erwarte=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            status = r.status
            content = json.loads(r.read())
    except urllib.error.HTTPError as e:
        status = e.code
        content = json.loads(e.read())
    if erwarte is not None:
        assert status == erwarte, f"erwarte={erwarte}, erhalten={status} {method} {path} {body}"
    elif status >= 500:
        raise AssertionError(f"Serverfehler {status} {method} {path} {body}: {content}")
    return status, content


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": {"typ": "laie_eingabe"}, "signal_2": "laie_bestaetigt"}}


def _texte(ericantwort: str) -> list[str]:
    """Fehlertexte aus der rohen ericantwort — gleiche Extraktion wie
    tests/test_checkest_durchstich.py::_pruefe."""
    return [" ".join(t.split())
            for t in re.findall(r"<Text>(.*?)</Text>", ericantwort or "", re.S)]


# Minimale Feldmenge fuer eine vollstaendige gesamt-Deklaration, gemessen 2026-08-10 per
# Probe-Skript gegen den echten HTTP-Endpunkt (s. Docstring, Scheiben-Korrektur). scheibe=
# "gesamt" statt "an_gesamt": nur "gesamt"/"rentner_gesamt" fuehren STAMMDATEN_FELDER in
# api_constants.py, "an_gesamt" hat kein gesamt_guard und ist strukturell keine abgabefaehige
# Scheibe (team-lead-Messung). KAP-Detailfelder bewusst weggelassen (nur kein_kap=True) —
# s. Docstring.
_STAMM_A = (("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
            ("stammdaten_geburtsdatum", "05.05.1955"),
            ("stammdaten_strasse", "Musterstr."), ("stammdaten_hausnummer", "55"),
            ("stammdaten_plz", "55555"), ("stammdaten_wohnort", "Musterort"),
            ("stammdaten_keine_bankverbindung", True),
            ("stammdaten_art_est_erklaerung", True),
            ("kist_konfession", "keine"),
            # 3b5ccfc: kein Kz-Spiegel, erreicht erzeuge_xml() ueber den snapshot-Parameter.
            # Praefix 9181 muss zur Finanzamtsnummer des Empfaengers passen, sonst wirft der
            # Writer. Ohne dieses Feld liefert /einreichen seit der abgabefaehig=True-Umstellung
            # 'xml_nicht_baubar' — und dann zaehlt die Ratsche unten 0 Fehlertexte, weil es gar
            # keine ERiC-Antwort gibt. Genau dieser Fall hat hier einmal Falsch-Gruen erzeugt.
            ("stammdaten_steuernummer", "9181081508155"),
            # Anlage N (Julius-Entscheidung 2026-08-10, Option A): E0200002 + E0200301 real deklariert.
            ("steuerklasse", "1"), ("p36_lohnsteuer", 1200000))

_BASIS_A = (("bruttoarbeitslohn", 6000000), ("vor_an_anteil_rv", 4200000),
            ("vor_ag_anteil_rv", 1200000), ("vor_rv_ausserhalb_lstb", 0),
            ("kein_gewinn", True), ("kein_kap", True),
            ("kein_vuv", True), ("kein_sonstige", True)) + _STAMM_A


def _fall_einzel(base):
    st, r = _req(base, "POST", "/fall", {"fall_id": "durchstich_http", "scheibe": "gesamt",
                                         "veranlagungszeitraum": 2025}, erwarte=201)
    fid = r["fall_id"]
    for fld, w in _BASIS_A:
        _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w), erwarte=201)
    _req(base, "POST", f"/fall/{fid}/event", _laie("veranlagung", "einzel"), erwarte=201)
    return fid


@braucht_eric
def test_einreichen_endpunkt_erreicht_die_amtliche_pruefung(base, monkeypatch):
    """Echter HTTP-Durchstich: /fall -> /event (bestaetigt) -> /einreichen, ECHTES XML,
    ECHTES checkESt — nichts gemockt. Die Deklaration ist absichtlich NICHT vollstaendig
    im Sinne einer abgabefertigen Erklaerung (Vorsatz-Block, Anlage N sind noch offene
    Baustellen, s. Backlog im Vault) — das ist erlaubt, das XML muss nur bis zur
    Plausibilitaetspruefung durchkommen, nicht sie bestehen (rc=0 ist NICHT erwartet)."""
    monkeypatch.setenv("ELSTER_HERSTELLER_ID", _HID)
    fid = _fall_einzel(base)

    st, resp = _req(base, "POST", f"/fall/{fid}/einreichen", {})

    assert resp.get("grund") != "deklaration_unvollstaendig", (
        f"Fixtur unvollstaendig, Test misst den falschen Pfad: {resp}")
    assert resp.get("grund") != "xml_nicht_baubar", (
        f"XML-Bau selbst gescheitert (z.B. fehlende Hersteller-ID im Serverprozess): {resp}")
    assert resp.get("grund") != "eric_nicht_verfuegbar", (
        f"ERiC im Serverprozess nicht ladbar, obwohl braucht_eric nicht geskippt hat: {resp}")

    klasse = resp.get("klasse")
    rc = resp.get("rc")
    assert klasse != "io_gate_nicht_geprueft", (
        f"rc={rc}: das ueber den Endpunkt erzeugte XML bricht VOR der Plausibilitaets-"
        f"pruefung ab (leerer Fehlerpuffer sieht aus wie Erfolg, ist aber keiner). "
        f"Antwort: {resp}")
    assert klasse != "hersteller_id_gesperrt", f"Hersteller-ID abgelehnt: {resp}"
    assert klasse in ("plausibel", "plausibilitaet_fehler"), (
        f"unerwartete Klasse {klasse!r} — weder plausibel noch Plausibilitaetsfehler: {resp}")

    # Falsch-Gruen-Kern auf Endpunkt-Ebene: 200/plausibel=True darf NUR bei rc==0 vorkommen.
    if rc == 0:
        assert st == 200 and resp.get("plausibel") is True, resp
    else:
        assert st == 422 and resp.get("plausibel") is not True, (
            f"rc={rc} != 0, aber der Endpunkt meldet Erfolg: {resp}")
        assert isinstance(resp.get("ericantwort"), str) and resp["ericantwort"], (
            f"Plausibilitaetsfehler ohne ericantwort — der Nutzer saehe keine Regel: {resp}")


# Gemessen 2026-08-10 gegen ERiC 44.2.4.0, ESt_2025, ueber den echten HTTP-Endpunkt, scheibe=
# "gesamt" (dieselbe Fixtur wie _fall_einzel oben).
#
#   11 -> 2   nachdem der Endpunkt auf abgabefaehig=True + snapshot umgestellt wurde
#             (api.py:2398). Die 9 Vorsatz-Block-Fehler sind damit weg; Absender und
#             Steuernummer werden aus den deklarierten Stammdaten abgeleitet.
#
#   2 -> 0    (2026-08-10) Anlage N Steuerklasse + Lohnsteuer (Julius-Entscheidung, Option A =
#             echte Werte statt 0,00; _STAMM_A jetzt mit steuerklasse + p36_lohnsteuer).
#             Gemessen rc=CE.RC_OK ueber den echten HTTP-Endpunkt.
RESTFEHLER_ENDPUNKT_EINZEL = 0


@braucht_eric
def test_restfehler_ratsche_endpunkt(base, monkeypatch):
    """Ratsche auf Endpunkt-Ebene, analog zu test_checkest_durchstich.py::test_restfehler_ratsche.
    Rot in BEIDE Richtungen: Anstieg ist Regression, Rueckgang ohne Nachziehen der Konstante
    ist unverbuchter Fortschritt — beides soll auffallen, nicht verpuffen."""
    monkeypatch.setenv("ELSTER_HERSTELLER_ID", _HID)
    fid = _fall_einzel(base)
    st, resp = _req(base, "POST", f"/fall/{fid}/einreichen", {})

    # Zuerst: wurde ueberhaupt geprueft? Ohne diese Schranke zaehlt die Ratsche bei einem
    # Abbruch VOR checkESt null Fehlertexte und meldet triumphierend Fortschritt.
    # Real passiert am 2026-08-10: nach der Umstellung des Endpunkts auf abgabefaehig=True
    # fehlte in dieser Fixtur stammdaten_steuernummer, der Writer brach mit
    # 'xml_nicht_baubar' ab, es gab keine ericantwort — und die Ratsche las daraus
    # "0 statt 11 Fehler". Dieselbe Falsch-Gruen-Klasse wie rc=610301200 mit leerem Puffer.
    grund = resp.get("grund")
    assert grund not in ("xml_nicht_baubar", "deklaration_unvollstaendig",
                         "eric_nicht_verfuegbar", "kein_pruefmodul_fuer_vz"), (
        f"Abbruch VOR der Pruefung (grund={grund!r}) — keine Fehlertexte heisst hier NICHT "
        f"fehlerfrei. Die Ratsche darf das nie als Fortschritt lesen. Antwort: {resp}")

    rc = resp.get("rc")
    if rc == CE.RC_OK:
        assert RESTFEHLER_ENDPUNKT_EINZEL == 0, (
            f"Endpunkt meldet rc=0 (abgabefaehig!), Ratsche steht noch auf "
            f"{RESTFEHLER_ENDPUNKT_EINZEL}. Setze RESTFEHLER_ENDPUNKT_EINZEL = 0.")
        return

    assert CE.klassifiziere_rc(rc) == "plausibilitaet_fehler", (
        f"rc={rc} [{CE.klassifiziere_rc(rc)}]: kein Pruefergebnis, die Fehlerzahl unten "
        f"waere bedeutungslos. Antwort: {resp}")

    texte = _texte(resp.get("ericantwort", ""))
    assert not CE.gekappt_verdacht(resp.get("ericantwort", "")), (
        "Puffer gekappt — Zahl waere nicht belastbar")
    assert len(texte) <= RESTFEHLER_ENDPUNKT_EINZEL, (
        f"REGRESSION: {len(texte)} amtliche Fehler am Endpunkt, erlaubt sind "
        f"{RESTFEHLER_ENDPUNKT_EINZEL}.\n" + "\n".join(f"  - {t[:160]}" for t in texte))
    assert len(texte) == RESTFEHLER_ENDPUNKT_EINZEL, (
        f"FORTSCHRITT: nur noch {len(texte)} statt {RESTFEHLER_ENDPUNKT_EINZEL} Fehler am "
        f"Endpunkt. Trag das ein: RESTFEHLER_ENDPUNKT_EINZEL = {len(texte)}.\n"
        f"Verbleibend:\n" + "\n".join(f"  - {t[:160]}" for t in texte))


# Differenz Endpunkt minus Funktion, Stand 2026-08-10 (0 - 0 = 0), beide auf scheibe="gesamt"-
# foermigen Fixturen gemessen. ALLE bisherigen Ursachen sind geschlossen:
#   +0  Vorsatz-Block: GESCHLOSSEN. Der Endpunkt faehrt jetzt abgabefaehig=True und reicht
#       den snapshot durch (api.py:2398); Absender und Steuernummer werden aus den
#       deklarierten Stammdaten abgeleitet. Vorher +9.
#   -0  Anlage-KAP: GESCHLOSSEN. Die Funktions-Fixtur deklarierte kap_*=0 explizit und loeste
#       damit einen Angabegrund-Fehler aus, den die Endpunkt-Fixtur durch Weglassen vermied.
#       Seit Julius' Entscheidung (Option A) filtert est_mapping.deklariere() die Nullwerte
#       selbst heraus — beide Fixturen erzeugen jetzt dasselbe Bild. Vorher -1.
#   +0  Anlage N Steuerklasse+Lohnsteuer: GESCHLOSSEN. Beide Fixturen tragen jetzt
#       steuerklasse + p36_lohnsteuer (Julius-Entscheidung 2026-08-10, Option A). Beide
#       Ratschen erreichen rc=CE.RC_OK. Vorher 2 auf beiden Seiten (Delta blieb 0).
#
# Delta 0 heisst NICHT, dass die Wache ueberfluessig ist: sie faengt weiterhin jedes
# kuenftige Auseinanderlaufen. Ein Delta von 0 ist der Zustand, den man will, nicht der,
# den man aufgibt. Historischer Rest des alten Kommentars zur Nachvollziehbarkeit:
#       kap_kapitalertraege/kap_gewinn_aktien/kap_verlust_aktien/kap_verlust_sonstige explizit
#       als 0 und loeste damit einen KAP-Angabegrund-Fehler aus; die Endpunkt-Fixtur laesst
#       diese Felder weg (nur kein_kap=True) und baut Anlage KAP gar nicht erst — KEINE echte
#       Naht, nur ein Fixtur-Unterschied zwischen den beiden Ratschen.
# Wer eine der beiden Ratschen aendert, MUSS diese Zahl nachziehen und den Grund hier
# eintragen — sonst laufen die zwei Messungen wieder unbemerkt auseinander.
DELTA_ENDPUNKT_MINUS_FUNKTION_EINZEL = 0


def test_delta_endpunkt_funktion_ratsche_bekannt():
    """Braucht kein ERiC (reine Konstanten-Pruefung) — bleibt auch credential-frei gruen und
    faengt trotzdem, wenn RESTFEHLER_ENDPUNKT_EINZEL oder die Funktions-Ratsche
    RESTFEHLER_EINZEL (test_checkest_durchstich.py) einzeln editiert werden, ohne die
    Beziehung zwischen beiden nachzuziehen."""
    from test_checkest_durchstich import RESTFEHLER_EINZEL
    ist = RESTFEHLER_ENDPUNKT_EINZEL - RESTFEHLER_EINZEL
    assert ist == DELTA_ENDPUNKT_MINUS_FUNKTION_EINZEL, (
        f"Differenz Endpunkt- vs. Funktions-Ratsche hat sich geaendert: {ist} statt "
        f"{DELTA_ENDPUNKT_MINUS_FUNKTION_EINZEL} (RESTFEHLER_ENDPUNKT_EINZEL="
        f"{RESTFEHLER_ENDPUNKT_EINZEL}, RESTFEHLER_EINZEL={RESTFEHLER_EINZEL}). "
        f"Miss neu, trag die neue Differenz UND den Grund im Kommentar ueber "
        f"DELTA_ENDPUNKT_MINUS_FUNKTION_EINZEL ein — nicht nur die Zahl.")
