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

Reparaturrichtung bewusst offen: ob der Instanz-Mechanismus auf Grdst/Einz-Ebene absteigt oder
anders angebunden wird, ist nicht entschieden -- dieser Test bindet nur das amtliche Ergebnis
(XSD-valide), nicht den Weg dorthin. Wird der Test gruen (XPASS), ist das ein Befund: der
maxOccurs-Verstoss ist behoben, und dieser xfail muss geprueft/entfernt werden.

Gemessen bei HEAD b3d1a1b (2026-08-30), gegen den geteilten Baum, ueber den niedrigen Pfad
Store -> deklariere -> erzeuge_xml (produkt/haut/api_constants.py, produkt/bindung/
bindung_an_gesamt.yaml und darauf haengende WIP einer Nachbarinstanz werden hier nicht
beruehrt/importiert). elster_xml.py/est_mapping.py sind seit a4da29b (Basis der ersten
Messung, /tmp-Worktree) unveraendert (`git diff a4da29b..HEAD -- produkt/import/elster_xml.py
produkt/mapping/est_mapping.py` = leer).

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser", "elster/submission"):
    sys.path.insert(0, os.path.join(ROOT, sub))

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

# Zwei Verkaeufe derselben Person, beide 'grundstueck' -- identische Werte wie in der /tmp-
# Erstmessung, damit sich beide Ergebnisse an derselben Zahl verankern lassen.
VERKAEUFE = [(20000000, 15000000, 500000, "grundstueck"), (13000000, 9000000, 200000, "grundstueck")]


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
    """Kontrolle (main-Auflage): OHNE Kontrollfall, der sauber durchlaeuft, beweist ein roter
    Befund unten nur einen kaputten Messaufbau. Ein Verkauf -> genau ein <SO>, ein <Grdst>,
    XSD-valide."""
    snap, sid = ST.materialisiere(_fall("p23_kontrolle_ein_verkauf", 1))
    result = est_mapping.deklariere(snap, bindung, snapshot_id=sid)
    assert result.get("eingaben_konsistent") is True, result.get("unvollstaendig")
    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, snapshot=snap)
    n_so, n_grdst = xml.count("<SO"), xml.count("<Grdst")
    assert (n_so, n_grdst) == (1, 1), (
        f"Kontrolle (1 Verkauf) sollte genau ein <SO> und ein <Grdst> ergeben, "
        f"tatsaechlich <SO>={n_so}, <Grdst>={n_grdst} -- Messaufbau selbst waere kaputt.")

    pfad = str(tmp_path / "p23_ein_verkauf.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, f"Kontrollfall (1 Verkauf) nicht schema-valide -- Messaufbau kaputt: {meldung}"


@braucht_xsd
@pytest.mark.xfail(strict=True, reason=(
    "p23_anzahl_verkaeufe>1 fuer dieselbe Person haengt am generischen Instanz-Container "
    "('E10','SO'), aber <SO> traegt maxOccurs=1 (E10-2025.xsd:8298). Zwei Verkaeufe erzeugen "
    "zwei <SO>-Geschwister -- xmllint lehnt hart ab ('Element SO: This element is not "
    "expected'), ERiC/checkESt bestaetigt unabhaengig ueber die Plausibilitaetspruefung "
    "(FachlicheFehlerId=zuGrosseLfdNummer auf SO[2]/Priv_VA_G[1]/Grdst[1], rc=610001002, "
    "Puffer nicht leer/nicht gekappt). Das Schema sieht die richtige Stelle fuer mehrere "
    "Verkaeufe EINER Person bereits vor (Grdst.Einz, maxOccurs=99, E10-2025.xsd:22231) -- "
    "unbenutzt (kein <Einz> in elster_xml.py/est_mapping.py). Reparaturrichtung offen, dieser "
    "Test bindet nur das Ergebnis (XSD-valide). Wird er gruen (XPASS): Befund pruefen/xfail "
    "entfernen."))
def test_zwei_verkaeufe_gleiche_person_ist_xsd_valide(bindung, tmp_path):
    """Messung: zwei Verkaeufe derselben Person sollten -- wie der Kontrollfall -- schema-
    valide bleiben. Tun sie aktuell nicht (zwei <SO> statt eines)."""
    snap, sid = ST.materialisiere(_fall("p23_messung_zwei_verkaeufe", 2))
    result = est_mapping.deklariere(snap, bindung, snapshot_id=sid)
    assert result.get("eingaben_konsistent") is True, result.get("unvollstaendig")
    ai = result.get("anlage_instanzen", {}).get("p23_veraeusserung", [])
    assert len(ai) == 2, f"Erwartet 2 anlage_instanzen[p23_veraeusserung]-Eintraege, erhalten: {ai}"

    xml = EX.erzeuge_xml(result, vz=2025, hersteller_id=HID, snapshot=snap)
    n_so, n_grdst = xml.count("<SO"), xml.count("<Grdst")

    pfad = str(tmp_path / "p23_zwei_verkaeufe.xml")
    with open(pfad, "w", encoding="utf-8") as f:
        f.write(xml)
    ok, meldung = VX.validate(pfad, "2025")
    assert ok, (
        f"2 Verkaeufe derselben Person: XML nicht schema-valide (<SO>={n_so}, <Grdst>={n_grdst}, "
        f"erwartet je 1). xmllint: {meldung}")
