"""AK3: Erreicht die Mahlzeitenkürzung (§ 9 Abs. 4a S. 8-10 EStG) die ELSTER-Erklärung?

AUFTRAG (main, 2026-10-01): "Die Verpflegungspauschale wird bei den Mahlzeiten gekürzt [...],
aber die Kürzung erreicht die ELSTER-Erklärung nicht — sie rechnet sie, und das XML zeigt nur
die Tage. Ein Ring kürzt 84 EUR, das XML weiß nichts davon."

DIESER TEST PRÜFT GENAU DIESE BEHAUPTUNG. Er geht über den echten Weg (HTTP-Server → Store →
_mit_ring_werten → deklariere → einreichen → erzeuge_xml) und vergleicht an EINEM Fall die drei
Ebenen: Ring-Wert (Cent), deklariertes Kz E0205508 (Euro), echtes abgesendetes XML.

Drei Fälle, gleiche Pauschale (100 × 28 EUR = 2.800 EUR, klar über dem AN-Pauschbetrag von
1.230 EUR für 2025 — sonst maskiert der Pauschbetrag jede Steuerwirkung):

  baseline          keine Mahlzeiten (Antwort: vpf_keine_mahlzeitengestellung=True)
  achtundzwanzig    5 × Frühstück           -> 5 × 5,60 = 28,00 EUR Kürzung
  vierundachtzig    5 × Frühstück + 5 × Mittag -> 5×5,60 + 5×11,20 = 84,00 EUR Kürzung

Der Vergleich baseline ↔ achtundzwanzig ↔ vierundachtzig ist die Gegenprobe: die Kürzung muss
die Steuer erhöhen, und zwar umso mehr, je größer sie ist. Wäre E0205508 eine Zierzeile ohne
Wirkung, fiele dieser Test — der XML-Vergleich allein könnte das nicht zeigen.

Die Mutationsprobe (AK3, "zeig beide Zustände") steht NICHT in dieser Datei, sondern im
Bericht: mit entfernter Injektion in _mit_ring_werten wird test_kuerzung_erreicht_das_xml rot.
Ein grüner Test, der nie rot war, beweist nichts.

WAS DIESER TEST AUSDRÜCKLICH NICHT BEHAUPTET:
  Nicht, dass die Kürzungsrechnung in jeder Reihenfolge richtig ist (Deckel S. 8 HS 3 vor
  Entgelt S. 10 — separat gemessen, Bericht AK2). Nicht, dass ein Bestandsfall betroffen ist:
  der Bestand trägt in 17 Fällen Verpflegungs-Felder, aber 0 gestellte Mahlzeiten, die
  Kürzung ist dort gesetzlich richtig 0. Dieser Test misst die Naht, nicht den Schaden.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/eingang", "produkt/store", "produkt/mapping", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

os.environ["TAXGRAPH_NO_AUTH"] = "1"   # wie tests/conftest.py — sonst 401 auf /fall

import api as API              # noqa: E402
import server as SRV           # noqa: E402
import audit                   # noqa: E402
import elster_xml as EX        # noqa: E402

from _kegel import kegel_fuer  # noqa: E402


@pytest.fixture(scope="module", autouse=True)
def _fake_hersteller_id():
    """Hersteller-ID nur für die Dauer dieser Datei — nur lokale XML-Erzeugung, kein Versand.

    Auf Modulebene gesetzt leckt die ID in fremde Testdateien (dort gemessen 2026-09-26:
    ERiC-Tests hielten sich für lauffähig und scheiterten mit rc=610301200 statt zu skippen).
    """
    alt = os.environ.get("ELSTER_HERSTELLER_ID")
    if alt is None:
        os.environ["ELSTER_HERSTELLER_ID"] = "00000000000"
    try:
        yield
    finally:
        if alt is None:
            os.environ.pop("ELSTER_HERSTELLER_ID", None)
        else:
            os.environ["ELSTER_HERSTELLER_ID"] = alt


def _req(base, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                  headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, wert):
    return {"feld_id": fld, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


# Der volle Pflicht-Kegel der Scheibe 'gesamt' (tests/_kegel.py, gebaut — nicht handgepflegt).
# Nur die Overrides sind echte Werte, der Rest trägt den Abwesenheitswert.
_GRUND = kegel_fuer("gesamt", {
    "bruttoarbeitslohn": 6000000, "vor_an_anteil_rv": 4200000, "vor_ag_anteil_rv": 1200000,
    "veranlagung": "einzel",
    "agb_zwangslaeufig": True, "agb_notwendig_angemessen": True,
}) + [
    ("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
    ("stammdaten_geburtsdatum", "05.05.1955"),
    ("stammdaten_strasse", "Musterstr."), ("stammdaten_hausnummer", "55"),
    ("stammdaten_plz", "55555"), ("stammdaten_wohnort", "Musterort"),
    ("stammdaten_keine_bankverbindung", True),
    ("stammdaten_art_est_erklaerung", True),
    ("kist_konfession", "keine"),
    ("stammdaten_steuernummer", "9181081508155"),
    ("steuerklasse", "1"), ("p36_lohnsteuer", 1200000),
]

TAGE_24H = 100          # 100 × 28 EUR = 2.800 EUR Pauschale
KUERZUNG_28 = 28        # 5 × 5,60 EUR
KUERZUNG_84 = 84        # 5 × 5,60 + 5 × 11,20 EUR


@pytest.fixture(scope="module")
def gemessen(tmp_path_factory):
    """Baut die drei Fälle über den echten HTTP-Server und fängt das abgesendete XML ab.

    Der Spion sitzt auf EX.erzeuge_xml — demselben Aufruf, den api.einreichen() benutzt. Er
    verändert nichts, er hält das Ergebnis fest. Nur so ist die dritte Ebene (das echte XML)
    messbar, ohne den Abgabeweg nachzubauen.
    """
    faelle_dir = tmp_path_factory.mktemp("faelle")
    API.FAELLE = str(faelle_dir)
    audit.AUDIT_DIR = str(faelle_dir)
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    base = f"http://{srv.server_address[0]}:{srv.server_address[1]}"

    xml_erfasst = {}
    orig = EX.erzeuge_xml

    def _spion(*a, **kw):
        text = orig(*a, **kw)
        xml_erfasst["letztes"] = text
        return text

    EX.erzeuge_xml = _spion
    hid = pytest.MonkeyPatch()
    if "ELSTER_HERSTELLER_ID" not in os.environ:
        hid.setenv("ELSTER_HERSTELLER_ID", "00000000000")

    def _messe(fid, verpflegung):
        st, r = _req(base, "POST", "/fall",
                     {"fall_id": fid, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
        assert st == 201, (fid, "fall_anlegen", st, r)
        for fld, w in _GRUND:
            st, r = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
            assert st == 201, (fid, fld, st, r)
        for fld, w in verpflegung:
            st, r = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
            assert st == 201, (fid, fld, st, r)
        st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
        assert st == 200, (fid, "ergebnis", st, erg)
        st, dek = _req(base, "GET", f"/fall/{fid}/deklaration")
        assert st == 200, (fid, "deklaration", st, dek)
        xml_erfasst.pop("letztes", None)
        st_e, ein = API.einreichen(fid, {})
        return {"ergebnis": erg, "deklaration": dek, "einreichen": (st_e, ein),
                "xml": xml_erfasst.get("letztes")}

    # Gemeinsamer Rumpf: gleiche Tage, gleiche Frist-Antwort. Nur die Mahlzeiten unterscheiden
    # die Fälle — sonst vergliche der Steuer-Delta unten zwei verschiedene Pauschalen.
    rumpf = [("tage_24h", TAGE_24H), ("vpf_monate_am_ort", 2)]

    ergebnisse = {}
    try:
        # Baseline: der Nutzer bestätigt, dass ihm KEINE Mahlzeit gestellt wurde. Ohne diese
        # Antwort blockt der Wächter `verpflegung_reduktion_offen` (bescheid_deklaration.py:918)
        # und es entstünde gar kein XML — die Baseline braucht sie also selbst.
        ergebnisse["baseline"] = _messe("vpfxml_base", rumpf + [
            ("vpf_keine_mahlzeitengestellung", True)])
        ergebnisse["achtundzwanzig"] = _messe("vpfxml_28", rumpf + [
            ("vpf_fruehstuecke_gestellt_anzahl", 5)])
        ergebnisse["vierundachtzig"] = _messe("vpfxml_84", rumpf + [
            ("vpf_fruehstuecke_gestellt_anzahl", 5),
            ("vpf_mittagessen_gestellt_anzahl", 5)])
    finally:
        EX.erzeuge_xml = orig
        hid.undo()
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()
    return ergebnisse


def _kz(gemessen, fall):
    return gemessen[fall]["deklaration"]["deklaration"].get("E0205508")


def test_baseline_ohne_mahlzeiten_hat_keine_kuerzung(gemessen):
    """Der Nullpunkt. Ohne ihn ist jede Zahl unten mit nichts verglichen.

    Ein bestätigtes "keine Mahlzeit gestellt" darf keine Kürzung erzeugen — kein E0205508 im
    Dict und keins im XML. Der Typ GanzzahlNichtNegOhneFuehrNull_MaxVK12_CType_RABE
    (E10-2025.xsd:18383) verbietet die 0 im XML ohnehin; das Feld gehört ganz weg.
    """
    b = gemessen["baseline"]
    assert b["xml"] is not None, f"Baseline blockiert, kein XML: {b['einreichen']}"
    assert _kz(gemessen, "baseline") is None, (
        f"Ohne Mahlzeiten darf keine Kürzung entstehen: {_kz(gemessen, 'baseline')!r}")
    assert "<E0205508>" not in b["xml"], "E0205508 im XML, obwohl keine Mahlzeit gestellt wurde"


@pytest.mark.parametrize("fall,erwartet", [("achtundzwanzig", KUERZUNG_28),
                                           ("vierundachtzig", KUERZUNG_84)])
def test_kuerzung_erreicht_das_xml(gemessen, fall, erwartet):
    """Der Kern: E0205508 (Kürzung) steht im abgesendeten XML, und der Betrag stimmt.

    Das Kz führt EURO (Typ GanzzahlNichtNegOhneFuehrNull_MaxVK12_CType_RABE,
    E10-2025.xsd:18383); der Ring liefert CENT. _cent_nach_kz (est_mapping.py:197-205) teilt
    durch 100 — E0205508 steht NICHT in _ABZUGS_KZ, also floor.
    """
    g = gemessen[fall]
    dek = g["deklaration"]["deklaration"]

    assert dek.get("E0205409") == TAGE_24H, (
        f"Die Tage-Position fehlt oder weicht ab — dann kürzt der Bescheid eine Position, "
        f"die nicht in der Erklärung steht: {dek}")

    kz = _kz(gemessen, fall)
    assert isinstance(kz, int) and kz == erwartet, (
        f"E0205508 (Kürzung) fehlt in der Deklaration oder weicht ab: {kz!r} — "
        f"erwartet {erwartet} EUR")

    assert g["xml"] is not None, (
        f"Kein XML erfasst — einreichen() wurde von einem Wächter geblockt: {g['einreichen']}")
    assert f"<E0205508>{kz}</E0205508>" in g["xml"], (
        f"DIE KÜRZUNG ERREICHT DAS XML NICHT: E0205508={kz} steht in der Deklaration, aber "
        f"nicht im abgesendeten XML. Dann zeigt das XML nur die vollen Tage-Kz (E0205409) und "
        f"die Erklärung fällt um die Kürzung zu hoch aus.")


def test_kuerzung_erhoeht_die_steuer_und_skaliert_mit_ihrem_betrag(gemessen):
    """Die Gegenprobe: E0205508 ist keine Zierzeile.

    Die Kürzung mindert den Werbungskostenabzug und muss die festgesetzte Steuer deshalb
    ERHÖHEN. Wäre das Kz nur dekorativ — oder würde der Ring es zwar rechnen, der Bescheid es
    aber nicht verbrauchen —, fiele dieser Test, während der reine XML-Vergleich oben grün
    bliebe.

    Zusätzlich die Skalierung: die größere Kürzung (84 EUR) muss stärker wirken als die
    kleinere (28 EUR). Das schließt aus, dass beide Fälle zufällig denselben Betrag tragen.
    """
    zahl = {f: gemessen[f]["ergebnis"]["zahl_cent"]
            for f in ("baseline", "achtundzwanzig", "vierundachtzig")}
    for f, z in zahl.items():
        assert isinstance(z, int), f"keine Zahl für {f}: {gemessen[f]['ergebnis']}"

    d28 = zahl["achtundzwanzig"] - zahl["baseline"]
    d84 = zahl["vierundachtzig"] - zahl["baseline"]
    assert d28 > 0, (
        f"28 EUR Kürzung erhöhen die Steuer nicht (delta={d28} cent) — das Kz wirkt nicht")
    assert d84 > d28, (
        f"84 EUR Kürzung wirken nicht stärker als 28 EUR (d28={d28}, d84={d84} cent)")

    print(f"\n[AK3] Steuer-Delta gegen Baseline: 28 EUR Kürzung -> {d28/100:.2f} EUR, "
          f"84 EUR -> {d84/100:.2f} EUR")
