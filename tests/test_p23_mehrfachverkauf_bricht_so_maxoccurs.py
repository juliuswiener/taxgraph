"""§23 EStG (private Veraeusserungsgeschaefte), Mehrfachverkauf DERSELBEN Person: die generische
Instanz-Wiederholung (`p23_anzahl_verkaeufe` -> `anlage_instanzen["p23_veraeusserung"]`) haengt
am direkten E10-Kind `SO` (`container = kz_path[:2]`, produkt/import/elster_xml.py). Das E10-2025-
Schema erlaubt `<SO>` aber nur EINMAL:

    E10-2025.xsd:8298  <xs:element name="SO" ... minOccurs="0" maxOccurs="1">

-- anders als die meisten Geschwister-Anlagen (G, Zins, S, N, KAP, R, ...), die maxOccurs="2"
tragen (je einmal pro Person A/B). Zwei Verkaeufe derselben Person erzeugen deshalb ZWEI `<SO>`-
Geschwister unter `<E10>`, was gegen maxOccurs="1" verstoesst, bevor die fuenf `xs:unique`-
Blocks auf `Priv_VA_G` (E10-2025.xsd:21891-21910, je ein Block fuer Grdst/Virt_Waehr/And_WG/
Ant_Ek/Begr_V_Rue, Feld `Person`) ueberhaupt in Reichweite kommen -- jedes der zwei frisch
erzeugten `Priv_VA_G` enthaelt nur ein einziges `Grdst`, die xs:unique-Pruefung sieht darin nie
zwei Geschwister.

Das Schema selbst sieht die richtige Stelle fuer "mehrere Verkaeufe EINER Person" vor:

    E10-2025.xsd:22224  <xs:complexType name="Grdst_972825866_CType">
    E10-2025.xsd:22231    <xs:element name="Einz" ... minOccurs="0" maxOccurs="99"/>

-- mehrere `<Einz>` INNERHALB EINES `<Grdst>`. Ein Grep ueber produkt/import/elster_xml.py und
produkt/mapping/est_mapping.py nach `Einz` findet dort NIRGENDS eine tatsaechliche Erzeugung
dieses Elements (nur unverwandte Treffer wie "Einzel-"/"Einzelposten") -- die vorgesehene Stelle
ist unbenutzt; der bestehende Instanz-Mechanismus vermischt die Personen-Achse (2. Grdst-Instanz)
und die Mehrfachheits-Achse (Einz) zu einer einzigen Wiederholung auf SO-Ebene.

Doppelt bestaetigt, nicht nur per XSD:
  - xmllint (elster/submission/validate_xsd.py, amtliches E10-2025-Schema) lehnt das
    2-Verkaeufe-XML hart ab: "Element '...SO': This element is not expected."
  - ERiC/checkESt (elster/checkest_gate.py, EricBearbeiteVorgang/ERIC_VALIDIERE, KEIN Versand)
    schlaegt zwar nicht am Schema-Kurzschluss (rc=610301200) fehl, sondern erst in der
    Plausibilitaetspruefung (rc=610001002) -- dort aber mit ZWEI Fehlern, die im 1-Verkauf-
    Kontrollfall fehlen: FachlicheFehlerId=zuGrosseLfdNummer auf
    `/SO[2]/Priv_VA_G[1]/Grdst[1]/Einz[1]/E0306801[1]` und `/SO[2]/Priv_VA_G[1]/Grdst[1]/
    Person[1]`. Beide Antwortpuffer sind nicht leer (9502/10719 Zeichen, 17/19
    FehlerRegelpruefung-Eintraege) und `gekappt_verdacht()` meldet False fuer beide -- der
    Falsch-Gruen-Fall "leerer Puffer bei 610301200" liegt hier NICHT vor.

Gemessen wird ausschliesslich die anlage_instanzen["p23_veraeusserung"]-Repraesentation --
`deklaration` traegt fuer p23 kein einziges Kz (est_mapping.py: die p23-Betragsfelder werden nur
ueber `_deklariere_instanz()`/anlage_instanzen geschrieben, auch fuer den ersten, unindizierten
Verkauf).

Gemessen bei HEAD b3d1a1b (2026-08-30), gegen den geteilten Baum, ueber den niedrigen Pfad
Store -> deklariere -> erzeuge_xml (produkt/haut/api_constants.py, produkt/bindung/
bindung_an_gesamt.yaml und darauf haengende WIP einer Nachbarinstanz werden hier nicht
beruehrt/importiert). elster_xml.py/est_mapping.py sind seit a4da29b (Basis der ersten
Messung, /tmp-Worktree) unveraendert (`git diff a4da29b..HEAD -- produkt/import/elster_xml.py
produkt/mapping/est_mapping.py` = leer).

Repariert (HEAD 915e327, derselbe Commit wie diese Testaenderung): `erzeuge_xml()` verankert die
Instanz-Achse fuer die Gruppe `p23_veraeusserung` jetzt an `Einz` statt am E10-Direktkind
(`INSTANZ_CONTAINER_TIEFER` in produkt/import/elster_xml.py) -- gruppen-gebunden, weil "Einz" als
Elementname im E10-Schema generisch fuer ~100 andere Stellen wiederkehrt und ein pfadbasierter
Trigger fremde Instanzgruppen (kind/gwg/vv_objekt/rente/...) mitgerissen haette.

BELEGT nach dem Fix: 1/2/3 Verkaeufe derselben Person ergeben strukturell 1/1/1, 1/1/2, 1/1/3
(<SO>/<Grdst>/<Einz>), alle drei xmllint-valide gegen E10-2025.xsd (Tests unten). Die Kontrolle
(ein Verkauf) bleibt dabei unveraendert 1/1/1 -- ein Fix, der den Normalfall verschoben haette,
waere keiner gewesen.

NICHT BELEGT nach dem Fix: die amtliche ERiC/checkESt-Plausibilitaetsantwort. Die Test-
Hersteller-ID ist seit der urspruenglichen Messung (rc=610001002, 17 vs. 19 Fehler, s.o.)
amtlich gesperrt (rc=610301202 `ERIC_IO_TESTHERSTELLERID_GESPERRT`, leerer Puffer -- die
dokumentierte NICHT-GEPRUEFT-Klasse in checkest_gate.py, RC_HERSTELLER_GESPERRT), vermutlich
Fleet-weite Erschoepfung. xmllint-gruen traegt das ERiC-Bein NICHT von selbst: vor diesem Fix
widersprachen sich beide Pruefer bereits einmal (xmllint harte Schemaablehnung, ERiC dagegen
eine Plausibilitaetsfehlerliste -- der Reader liess den Fall durch). Der xfail(strict)-Gate-Test
unten behauptet nur, DASS ERiC ueberhaupt antwortet (rc != RC_HERSTELLER_GESPERRT), nicht WAS
er sagt -- er kippt auf XPASS, sobald die ID wieder frei ist, und meldet sich damit selbst statt
als `skip` unsichtbar zu bleiben.

Die PERSONENACHSE (Verkauf des Partners -> zweites <Grdst> mit Person=PersonB) ist von dieser
Reparatur bewusst NICHT beruehrt -- sie gehoert der Nachbarinstanz
(tests/test_p23_partner_verkauf_still_unter_person_a_eingereicht.py) und bleibt nachweislich
offen: ihr Test `..._zwei_instanzen_mit_unterschiedlichem_person_kennzeichen` faellt nach diesem
Fix auf einer ANDEREN Zeile (Person-Zuordnung PersonA/PersonA statt PersonA/PersonB), nicht mehr
auf der <SO>-Zaehlung -- die Vielfachheitsachse ist zu, die Personenachse steht.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser",
            "elster/submission", "elster"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import checkest_gate as CE     # noqa: E402
import elster_xml as EX        # noqa: E402
import est_mapping             # noqa: E402
import store as ST             # noqa: E402
import traverser as TR         # noqa: E402
import validate_xsd as VX      # noqa: E402

HID = "74931"
TS = "2026-08-30T20:45:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}

_schema_da = VX.find_schema("2025") is not None
_xmllint_da = bool(__import__("shutil").which("xmllint"))
braucht_xsd = pytest.mark.skipif(
    not (_schema_da and _xmllint_da),
    reason="E10-2025.xsd oder xmllint fehlt — XSD-Gate nicht lauffähig")


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
    not _ERIC_DA, reason="ERiC oder Hersteller-ID fehlt — amtlicher Prüfer nicht lauffähig "
                          "(credential-freies CI)")

# Drei Verkaeufe derselben Person, alle 'grundstueck' -- Werte 1+2 identisch mit der /tmp-
# Erstmessung, damit sich beide Ergebnisse an derselben Zahl verankern lassen. Der dritte ist die
# main-Auflage "Zusatzprobe": zwei koennte zufaellig die Personenachse treffen, drei nicht mehr.
VERKAEUFE = [(20000000, 15000000, 500000, "grundstueck"),
             (13000000, 9000000, 200000, "grundstueck"),
             (8000000, 6000000, 100000, "grundstueck")]


def _b(s, feld_id, wert):
    ST.append_event(store=s, feld_id=feld_id, wert=wert, zustand="bestaetigt", herkunft=H,
                     schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{feld_id}"}, ts=TS)


def _fall(fall_id, n_verkaeufe):
    s = ST.leerer_store(2025, fall_id=fall_id)
    _b(s, "bruttoarbeitslohn", 5000000)
    _b(s, "veranlagung", "einzel")
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", False)
    _b(s, "fam_anzahl_kinder", 0)
    _b(s, "verlustvortrag_bestand", 0)
    _b(s, "p23_anzahl_verkaeufe", n_verkaeufe)
    for i, (preis, ak, wk, typ) in enumerate(VERKAEUFE[:n_verkaeufe], start=1):
        suffix = "" if i == 1 else f"__{i}"
        _b(s, f"p23_veraeusserungspreis{suffix}", preis)
        _b(s, f"p23_anschaffung_herstellungskosten{suffix}", ak)
        _b(s, f"p23_werbungskosten{suffix}", wk)
        _b(s, f"p23_veraeusserungs_typ{suffix}", typ)
    return s


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


@braucht_xsd
def test_kontrolle_ein_verkauf_ist_xsd_valide(bindung, tmp_path):
    """Kontrolle (main-Auflage): ein Fix, der den Normalfall verschiebt, ist keiner. Ein Verkauf
    -> genau ein <SO>, ein <Grdst>, ein <Einz>, XSD-valide -- unveraendert vor UND nach dem Fix."""
    snap, sid = ST.materialisiere(_fall("p23_kontrolle_ein_verkauf", 1))
    result = est_mapping.deklariere(snap, bindung, snapshot_id=sid)
    assert result.get("eingaben_konsistent") is True, result.get("unvollstaendig")
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, snapshot=snap)
    n_so, n_grdst, n_einz = xml.count("<SO"), xml.count("<Grdst"), xml.count("<Einz")
    assert (n_so, n_grdst, n_einz) == (1, 1, 1), (
        f"Kontrolle (1 Verkauf) sollte genau ein <SO>, ein <Grdst>, ein <Einz> ergeben, "
        f"tatsaechlich <SO>={n_so}, <Grdst>={n_grdst}, <Einz>={n_einz} -- Messaufbau kaputt.")

    pfad = str(tmp_path / "p23_ein_verkauf.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, f"Kontrollfall (1 Verkauf) nicht schema-valide -- Messaufbau kaputt: {meldung}"


@braucht_xsd
def test_zwei_verkaeufe_gleiche_person_ist_xsd_valide(bindung, tmp_path):
    """Reparatur (main-Auflage): zwei Verkaeufe derselben Person -> EIN <SO>, EIN <Grdst>, ZWEI
    <Einz> -- schema-valide. Bis HEAD 915e327 war dieser Test xfail(strict): zwei <SO>-
    Geschwister verletzten maxOccurs=1 (E10-2025.xsd:8298), xmllint lehnte hart ab, ERiC
    bestaetigte unabhaengig ueber die Plausibilitaetspruefung (FachlicheFehlerId=
    zuGrosseLfdNummer auf SO[2]/Priv_VA_G[1]/Grdst[1], rc=610001002). Repariert ueber
    INSTANZ_CONTAINER_TIEFER in produkt/import/elster_xml.py (Achse -> Grdst.Einz,
    maxOccurs=99, E10-2025.xsd:22231) -- der xfail-Marker ist im selben Commit gefallen
    (XPASS(strict) beobachtet, nicht angenommen), nicht nur entfernt."""
    snap, sid = ST.materialisiere(_fall("p23_messung_zwei_verkaeufe", 2))
    result = est_mapping.deklariere(snap, bindung, snapshot_id=sid)
    assert result.get("eingaben_konsistent") is True, result.get("unvollstaendig")
    ai = result.get("anlage_instanzen", {}).get("p23_veraeusserung", [])
    assert len(ai) == 2, f"Erwartet 2 anlage_instanzen[p23_veraeusserung]-Eintraege, erhalten: {ai}"

    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, snapshot=snap)
    n_so, n_grdst, n_einz = xml.count("<SO"), xml.count("<Grdst"), xml.count("<Einz")
    assert (n_so, n_grdst, n_einz) == (1, 1, 2), (
        f"2 Verkaeufe derselben Person sollten ein <SO>, ein <Grdst>, zwei <Einz> ergeben, "
        f"tatsaechlich <SO>={n_so}, <Grdst>={n_grdst}, <Einz>={n_einz}.")

    pfad = str(tmp_path / "p23_zwei_verkaeufe.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, (
        f"2 Verkaeufe derselben Person: XML nicht schema-valide (<SO>={n_so}, <Grdst>={n_grdst}, "
        f"<Einz>={n_einz}). xmllint: {meldung}")


@braucht_xsd
def test_drei_verkaeufe_gleiche_person_liefert_drei_einz(bindung, tmp_path):
    """Zusatzprobe (main-Auflage): zwei Verkaeufe koennten zufaellig auf dieselbe Zahl treffen
    wie die Personenachse (max. 2 Personen) -- bei drei Verkaeufen derselben Person kann das
    nicht mehr sein. Drei Verkaeufe -> EIN <SO>, EIN <Grdst>, DREI <Einz>, schema-valide."""
    snap, sid = ST.materialisiere(_fall("p23_zusatzprobe_drei_verkaeufe", 3))
    result = est_mapping.deklariere(snap, bindung, snapshot_id=sid)
    assert result.get("eingaben_konsistent") is True, result.get("unvollstaendig")
    ai = result.get("anlage_instanzen", {}).get("p23_veraeusserung", [])
    assert len(ai) == 3, f"Erwartet 3 anlage_instanzen[p23_veraeusserung]-Eintraege, erhalten: {ai}"

    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, snapshot=snap)
    n_so, n_grdst, n_einz = xml.count("<SO"), xml.count("<Grdst"), xml.count("<Einz")
    assert (n_so, n_grdst, n_einz) == (1, 1, 3), (
        f"3 Verkaeufe derselben Person sollten ein <SO>, ein <Grdst>, drei <Einz> ergeben, "
        f"tatsaechlich <SO>={n_so}, <Grdst>={n_grdst}, <Einz>={n_einz}.")

    pfad = str(tmp_path / "p23_drei_verkaeufe.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, (
        f"3 Verkaeufe derselben Person: XML nicht schema-valide (<SO>={n_so}, <Grdst>={n_grdst}, "
        f"<Einz>={n_einz}). xmllint: {meldung}")


@braucht_eric
@pytest.mark.xfail(strict=True, reason=(
    "Test-Hersteller-ID amtlich gesperrt (rc=610301202 ERIC_IO_TESTHERSTELLERID_GESPERRT, "
    "gemessen 2026-08-30, eric.log: 'Die im XML angegebene Hersteller-ID ist gesperrt.' -- "
    "vermutlich Fleet-weite Erschoepfung, mehrere Instanzen liefen heute ERiC-Checks. "
    "hersteller_id_gesperrt ist eine von VIER NICHT-GEPRUEFT-Klassen in checkest_gate.py "
    "(CE.NICHT_GEPRUEFT_KLASSEN: io_gate_nicht_geprueft, hersteller_id_gesperrt, "
    "datenartversion_unbekannt, io_reader_unerwartete_elemente) -- alle liefern einen leeren "
    "Antwortpuffer, der wie 'keine Beanstandungen' aussieht. Ein Marker auf nur den einen heute "
    "gemessenen rc waere blind fuer die anderen drei; die Bedingung unten prueft deshalb "
    "Klassenzugehoerigkeit, nicht den einzelnen Code. xmllint-gruen traegt das ERiC-Bein NICHT "
    "von selbst: vor diesem Fix widersprachen sich beide Pruefer bereits einmal (xmllint harte "
    "Schemaablehnung, ERiC dagegen eine Plausibilitaetsfehlerliste ueber den Reader hinweg). "
    "Dieser Test behauptet nur, DASS ERiC ueberhaupt geprueft hat, nicht WAS er sagt. Ein `skip` "
    "waere hier falsch (sieht aus wie bestanden) -- kippt er auf XPASS, ist eine NICHT-GEPRUEFT-"
    "Klasse nicht mehr aktiv, und die eigentliche 17-gegen-17-Fehlerzahl-Messung gegen die "
    "Kontrolle ist nachzuholen."))
def test_eric_hat_ueberhaupt_geantwortet_zwei_verkaeufe(bindung):
    """Meldet sich selbst, sobald ERiC wieder antwortet -- misst NUR, ob ueberhaupt eine echte
    Pruefung stattgefunden hat (Klassifikation ueber die rc-Klassenmenge, nicht ueber einen
    einzelnen Code oder die Pufferlaenge: ein `len(fehler) == 0`-Test waere bei gesperrter ID
    falsch-gruen, und ein Marker auf nur RC_HERSTELLER_GESPERRT waere blind fuer die anderen drei
    NICHT-GEPRUEFT-Klassen)."""
    snap, sid = ST.materialisiere(_fall("p23_eric_gate_zwei_verkaeufe", 2))
    result = est_mapping.deklariere(snap, bindung, snapshot_id=sid)
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, snapshot=snap)
    rc, _antwort = CE.validate(xml, "ESt_2025")
    klasse = CE.klassifiziere_rc(rc)
    assert klasse not in CE.NICHT_GEPRUEFT_KLASSEN, (
        f"ERiC antwortet mit rc={rc} ({klasse}) -- das ist eine NICHT-GEPRUEFT-Klasse "
        f"(CE.NICHT_GEPRUEFT_KLASSEN), keine echte Pruefung hat stattgefunden.")
