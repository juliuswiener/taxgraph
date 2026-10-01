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
  der Bestand trägt in 23 Fällen Verpflegungs-Felder, aber 0 gestellte Mahlzeiten, die
  Kürzung ist dort gesetzlich richtig 0. Dieser Test misst die Naht, nicht den Schaden.
  Die 23 hielten bis 2026-10-01 nur im Commit-Text; test_bestand_hat_keine_kuerzung misst sie.

DER BESTAND IST EINE LEBENDE MENGE. Die Falldateien unter api_constants.FAELLE
($TAXGRAPH_DATEN/faelle bzw. ~/.local/share/taxgraph/faelle) entstehen im Entwicklungsbetrieb
weiter — 115 im August 2026, 77 im September. test_bestand_hat_keine_kuerzung pinnt den Nenner
deshalb mit einer Zahl, die veralten KANN; sie ist vom 2026-10-01 und wird von Hand
nachgezogen. Der Test sagt das in seiner Fehlermeldung — er ist ein Wecker, kein Gesetz.
"""
from __future__ import annotations

import glob
import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/eingang", "produkt/store", "produkt/mapping", "golden",
             "produkt/engine"):
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


# --------------------------------------------------------------------------- Bestandsmessung
# Die zweite Ebene dieses Befunds: der Ring rechnet richtig — nur rechnet er im Bestand nichts
# zu rechnen. Diese zwei Tests halten das fest, ohne einen Server: sie lesen die echten
# Falldateien und rufen DIESELBE Funktion, die bescheid_deklaration._mit_ring_werten ruft.
#
# Warum ohne Server: die Kürzung ist dort gesetzlich 0, es gibt also kein XML zu vergleichen.
# Was hier geprüft wird, ist der Nenner — nicht "es kam nichts an" (das wäre auch bei einem
# kaputten Ring grün), sondern "es gab nichts zu kürzen, und das lag an der Mahlzeit".

NENNER_STAND = 23        # 2026-10-01 12:38 (Commit f7bb8cb): Fälle mit Verpflegungs-Feldern,
                         # alle mit 0 Mahlzeiten.
                         # ROT SEIT 2026-10-01 14:07 — und das ist der Befund, nicht der Fehler:
                         # der Bestand ist um zan_g.json gewachsen, angelegt 14:07, Teil eines
                         # Wegwerf-Skripts eines anderen Workers, das in die echten Nutzerdaten
                         # geschrieben hat (15 Dateien zwischen 14:06 und 14:07). Der Fall trägt
                         # nur Tages-Kz, alle 0, KEINE Mahlzeiten — sachlich gehört er in den
                         # Nenner, aber er ist keine Nutzerdaten-Probe. Das Aufräumen läuft; bis
                         # dahin misst dieser Test die Verunreinigung mit. NICHT auf 24 ziehen:
                         # das macht sie dauerhaft und der Test fällt nach dem Aufräumen wieder um.
TAGE_KZ = ("tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig")
MAHLZEITEN_KZ = ("vpf_fruehstuecke_gestellt_anzahl", "vpf_mittagessen_gestellt_anzahl",
                 "vpf_abendessen_gestellt_anzahl")
FELDER_VPF = TAGE_KZ + MAHLZEITEN_KZ + ("vpf_mahlzeiten_gezahltes_entgelt",
                                        "vpf_steuerfreie_erstattung_betrag",
                                        "vpf_keine_mahlzeitengestellung")


def _bestand_felder():
    """Je Fall die materialisierten felder — nur für Fälle mit Verpflegungs-Feldern.

    Die Projektion kommt aus store.materialisiere (derselbe Weg wie api.deklaration), nicht
    aus einer eigenen ersetzt-Auflösung. Ein Fall mit fremdem VZ (Testfixture, 2099) fliegt
    raus: der Ring würde dort an fehlenden Params scheitern, und die Aussage gilt dem Bestand
    der drei unterstützten Jahrgänge (runner.VZ_ENUM).
    """
    import store as ST
    import api_constants as AK
    dateien = sorted(glob.glob(os.path.join(AK.FAELLE, "*.json")))
    if not dateien:
        pytest.skip(f"Kein Bestand unter {AK.FAELLE} — dieser Test prüft dann NICHTS. "
                    f"SKIP statt stillem PASS, damit ein leerer Bestand nicht sauber aussieht.")
    raus = []
    for pfad in dateien:
        with open(pfad, encoding="utf-8") as f:
            store = json.load(f)
        if store.get("veranlagungszeitraum") not in (2024, 2025, 2026):
            continue
        felder, _sid = ST.materialisiere(store)
        if any(f in felder for f in FELDER_VPF):
            raus.append((store["veranlagungszeitraum"], felder))
    return dateien, raus


def _kuerzung_cent(felder: dict, vz: int) -> int:
    """Dieselbe Funktion, die der Produktweg ruft — ohne except: 0.

    Mitzählbar bleibt sie nicht: eine Ausnahme ist hier ein Fehler und keine Null. In
    bescheid_deklaration._mit_ring_werten steht das try/except (dort ist eine 0 der sichere
    Ausgang); im Test würde es "grün" und "Ring tot" ununterscheidbar machen.
    """
    import runner as RN
    return RN._verpflegung_kuerzung_cent({fid: e["wert"] for fid, e in felder.items()}, vz)


def test_bestand_hat_keine_kuerzung():
    """Die Zahl aus dem Commit-Text, jetzt gemessen: 0 Kürzung in 23 Fällen mit Mahlzeiten-Feldern.

    Der Befund vom 2026-10-01 war: der Bestand trägt Verpflegungs-Felder, aber keine gestellte
    Mahlzeit — die Kürzung ist dort gesetzlich richtig 0. Das stand nur im Commit; kein Test
    hielt es fest. Damit war "0 Kürzung" von "Ring rechnet nichts" nicht zu unterscheiden,
    denn beide liefern 0.

    Beide Zahlen sind vom 2026-10-01 und können veralten: der Bestand wächst. Wird dieser Test
    rot, ist zuerst zu prüfen, ob ein echter Fall mit gestellter Mahlzeit dazugekommen ist —
    dann ist die rote Zahl der Befund und nicht der Test.
    """
    dateien, bestand = _bestand_felder()

    assert len(bestand) == NENNER_STAND, (
        f"Nenner verschoben: {len(bestand)} Fälle mit Verpflegungs-Feldern unter {len(dateien)} "
        f"Falldateien, festgehalten waren {NENNER_STAND} (2026-10-01). Prüf zuerst, ob ein neuer "
        f"Fall dabei ist (dann NENNER_STAND nachziehen) — oder ob ein Feld aus FELDER_VPF aus "
        f"der Bindung fiel.")

    mit_mahlfeld = [(vz, f) for vz, f in bestand if any(k in f for k in MAHLZEITEN_KZ)]
    werte = sorted({f[k]["wert"] for _vz, f in mit_mahlfeld for k in MAHLZEITEN_KZ if k in f})
    assert werte == [0], (
        f"Ein Bestandsfall trägt eine gestellte Mahlzeit: Werte {werte} in {len(mit_mahlfeld)} "
        f"Fällen mit Mahlzeiten-Anzahl-Feld. Dann ist die Null unten NICHT mehr gesetzlich "
        f"richtig, sondern ein Fund — und zwar einer für den XML-Weg, nicht für diesen Test.")

    summe = sum(_kuerzung_cent(f, vz) for vz, f in bestand)
    assert summe == 0, (
        f"Der Ring kürzt im Bestand {summe} Cent. Bei {len(bestand)} Fällen mit "
        f"Verpflegungs-Feldern und durchweg 0 gestellten Mahlzeiten darf das nicht sein — "
        f"entweder rechnet _verpflegung_kuerzung_cent aus einem anderen Feld, oder ein Fall "
        f"trägt doch eine Mahlzeit.")


def test_kuerzung_wird_positiv_sobald_eine_mahlzeit_gestellt_ist():
    """Die Geschwisterprüfung, über den echten Schreibpfad: ein gestelltes Frühstück kürzt.

    Ohne sie wäre test_bestand_hat_keine_kuerzung auch dann grün, wenn
    _verpflegung_kuerzung_cent konstant 0 zurückgäbe — genau die Verwechslung, die der
    Auftrag beheben will ("sonst ist grün und leer wieder nicht zu unterscheiden").

    Gebaut wird ein Store mit GENAU EINEM Event und dieses durch store.materialisiere
    geschickt — derselbe Weg, den api.deklaration nimmt. Ein Fall mit 28-Tage-Pauschale
    (tage_24h=1, 2800 Cent) und einem Frühstück: der Deckel (S. 8 HS 3) kann hier nicht
    greifen, also ist die Erwartung exakt, nicht nur "größer als 0".

    560 Cent = 20 % von 28 EUR, der Satz aus runner._verpflegung_roh_cent. Die Zahl steht
    hier als Literal: ein Test, der seine Erwartung aus derselben Funktion zieht, die er
    prüft, prüft nichts.
    """
    import store as ST
    store = ST.leerer_store(2025)
    ST.append_event(store, feld_id="tage_24h", wert=1, zustand="bestaetigt",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                              "haftung": "nutzer"},
                    schreiber="ui:laie",
                    signal={"signal_1": None, "signal_2": "ok@tage_24h"})
    ohne = _kuerzung_cent(ST.materialisiere(store)[0], 2025)

    ST.append_event(store, feld_id="vpf_fruehstuecke_gestellt_anzahl", wert=1,
                    zustand="bestaetigt",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                              "haftung": "nutzer"},
                    schreiber="ui:laie",
                    signal={"signal_1": None, "signal_2": "ok@vpf_fruehstuecke"})
    mit = _kuerzung_cent(ST.materialisiere(store)[0], 2025)

    assert ohne == 0, f"Ohne Mahlzeit darf nichts gekürzt werden: {ohne} Cent"
    assert mit - ohne == 560, (
        f"Ein gestelltes Frühstück ändert die Kürzung um {mit - ohne} Cent statt um 560 "
        f"({ohne} -> {mit}). Der Ring zählt das Mahlzeiten-Feld nicht — dann ist die Null im "
        f"Bestand kein Befund über den Bestand, sondern ein blinder Fleck.")


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
