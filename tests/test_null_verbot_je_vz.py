"""Die Null-Verbots-Liste ist jahresunabhaengig eingefroren — und 2024 ist ein anderer Jahrgang.

`est_mapping._NULL_UNZULAESSIG_KZ` (35 Kz, wortgleich in `rust/elster/src/kz_format.rs`) haelt
fest, in welchen Kennzeichen eine 0 die ganze Erklaerung uneinreichbar macht. Sie wurde aus
`E10-2025.xsd` abgeleitet und dann eingefroren, damit `deklariere()` ohne XSD laeuft — und
`deklariere()` hat keinen `vz`-Parameter, also wirkt dieselbe Menge fuer jedes Jahr.

Das ist die Grenze, die der ponytail-Kommentar an der Stelle selbst nennt. Dieser Test misst sie
statt sie zu behaupten: er leitet die verbietende Menge JE JAHR aus dem amtlichen Schema ab und
stellt sie der eingefrorenen gegenueber.

Gemessen wird nur die eine Richtung, die eine Luecke ist:
  Kz, in die TaxGraph eine ganze Zahl schreibt, die im Jahr DIE 0 VERBIETEN und trotzdem nicht
  in der Liste stehen -> dort geht eine 0 hinaus, die das Schema abweist.

Die Gegenrichtung (Liste verbietet, Jahr erlaubt) waere ein unterdrueckter Wert. Sie ist leer;
der Test prueft sie mit, damit ein spaeterer Jahrgang sie nicht still erzeugt.

Grenze der ersten Richtung: der Schreibweg ist der der Bindung (cent/int-Felder mit ihrer
`vz_gueltigkeit`). Kz, die nur der Mapper schreibt (Art-Verzweigung, Partner-Instanz), sieht
sie nicht; fuer die Jahresdrift wurden sie am 2026-10-01 gesondert gemessen (0 weitere Kz).
E10-2026.xsd lag nicht vor; 2026 ist ungeprueft, obwohl die Bindung das Jahr fuehrt.

Die Ableitungsregel ist wortgleich zum Rust-Zwilling `kz_mengen_aus_xsd`
(`rust/elster/tests/eigenschaften.rs`) und zum Drift-Test `test_kz_bindung_durchgang.py`:
GanzzahlPos*, oder eine enumeration ohne "0", oder ein pattern, auf das "0" nicht passt.

Nicht gemeint sind die Dezimal-Kz (`_KOMMA_OHNE_E60_KZ`): dort schreibt `_kz_wert` den String
"0,00", und "0,00" == 0 ist in Python falsch — die Unterdrueckung greift dort gar nicht und
braucht sie auch nicht, weil "0,00" schema-gueltig ist.
"""

import pathlib
import re
import sys

import pytest

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "produkt" / "mapping"))

import est_mapping as EM  # noqa: E402
import xsd_verify as XV  # noqa: E402


def _verbietet_null(m: dict) -> bool:
    """Dieselbe Regel wie im Drift-Test — wortgleich, damit die Ableitung vergleichbar bleibt."""
    return (m["type_name"].startswith("GanzzahlPos")
            or bool(m["enums"]) and "0" not in m["enums"]
            or bool(m["patterns"]) and not any(re.fullmatch(p, "0") for p in m["patterns"]))


def _felder_mit_vz() -> dict:
    """feld_id -> {typ, kz, vz} aus allen Bindungstabellen.

    `vz_gueltigkeit` fehlt bei Bestandsfeldern; sie gelten dann in allen drei Jahren.
    """
    felder = {}
    for yaml in sorted((ROOT / "produkt" / "bindung").glob("*.yaml")):
        txt = yaml.read_text(encoding="utf-8")
        for block in txt.split("- feld_id:")[1:]:
            m_fid = re.match(r"\s*(\w+)", block)
            m_kz = re.search(r"elster_kz:\s*(E\d{7})", block)
            if not (m_fid and m_kz):
                continue
            m_typ = re.search(r"^\s*typ:\s*(\w+)", block, re.M)
            m_vz = re.search(r"vz_gueltigkeit:\s*\[([^\]]*)\]", block)
            felder[m_fid.group(1)] = {
                "typ": m_typ.group(1) if m_typ else None,
                "kz": m_kz.group(1),
                "vz": set(re.findall(r"\d{4}", m_vz.group(1))) if m_vz
                      else {"2024", "2025", "2026"},
            }
    return felder


def _ganzzahl_schreibweg(jahr: int, meta: dict) -> set:
    """Kz, in die `_schreibe_kz` eine GANZE ZAHL schreibt und die es im Jahr gibt.

    Der typ-Filter (cent/int) ist noetig: Text- und Datums-Kz tragen dieselbe 0-Sperre im XSD,
    aber dort schreibt `_kz_wert` einen String, den `v == 0` nie trifft.
    """
    komma = set(EM._KOMMA_OHNE_E60_KZ)
    return {f["kz"] for f in _felder_mit_vz().values()
            if f["typ"] in ("cent", "int")
            and str(jahr) in f["vz"]
            and f["kz"] in meta
            and not f["kz"].startswith("E60")
            and f["kz"] not in komma}


@pytest.mark.parametrize("jahr", [2024, 2025])
def test_keine_null_in_einem_kz_das_sie_im_jahr_verbietet(jahr):
    """Fuers Jahr 2024 wird dieser Test rot: E0106603 (Anzahl weiterer Pflegepersonen) traegt
    2024 `GanzzahlPosOhneFuehrNull` (E10-2024.xsd:9581, Basis-Muster `[1-9].*`) — die 0 ist dort
    verboten. 2025 traegt dasselbe Kz `GanzzahlNichtNegOhneFuehrNull` (E10-2025.xsd:9746, Muster
    `[1-9].*|0`) und das XSD-Label verlangt die 0 sogar ausdruecklich.

    Die Bindung faehrt das Feld ueber alle drei Jahre (`bindung_rentner.yaml:432`) und der
    Fragetext laesst die 0 als Normalfall zu ("0, wenn du allein pflegst"). Wer 2024 allein
    pflegt, schreibt also eine 0 in ein Kz, das sie verbietet — die ganze Erklaerung wird
    uneinreichbar.
    """
    pfad = XV._find_schema(jahr)
    if not pfad:
        pytest.skip(f"E10-{jahr}.xsd nicht gefunden — $ERIC_DIR setzen")
    meta = XV._resolve_kz_meta(pfad)
    V = {kz for kz, m in meta.items() if _verbietet_null(m)}
    schreibweg = _ganzzahl_schreibweg(jahr, meta)

    luecke = sorted(kz for kz in schreibweg & V
                    if kz not in EM._NULL_UNZULAESSIG_KZ)
    assert not luecke, (
        f"VZ {jahr}: {len(luecke)} Kz verbieten die 0 laut E10-{jahr}.xsd, stehen aber nicht in "
        f"_NULL_UNZULAESSIG_KZ — eine 0 geht dort hinaus und macht die Erklaerung "
        f"uneinreichbar: {luecke}")


@pytest.mark.parametrize("jahr", [2024, 2025])
def test_liste_unterdrueckt_keinen_erlaubten_wert(jahr):
    """Die Gegenrichtung: ein Kz der Liste, in dem das Jahr die 0 ERLAUBT, verschluckt einen
    gehoerenden Wert. Der Test haelt sie leer.

    Er ist die zweite Haelfte des Befunds, nicht seine Wiederholung: er zeigt, dass die Liste in
    ihrer heutigen Form NICHT reparierbar ist. E0106603 muss 2024 verboten und 2025 erlaubt sein
    (dort verlangt das XSD-Label die 0 sogar). Eine jahresunabhaengige Menge kann beides nicht —
    nimmt sie das Kz auf, wird dieser Test fuer 2025 rot; laesst sie es weg, wird der Zwillings-
    test fuer 2024 rot. Der naheliegende Fix (Kz anhaengen) repariert 2024 und zerstoert 2025.
    """
    pfad = XV._find_schema(jahr)
    if not pfad:
        pytest.skip(f"E10-{jahr}.xsd nicht gefunden — $ERIC_DIR setzen")
    meta = XV._resolve_kz_meta(pfad)
    V = {kz for kz, m in meta.items() if _verbietet_null(m)}

    # Alle Listen-Kz, nicht nur die des Bindungs-Schreibwegs: die neun § 35c-Kz der
    # Art-Verzweigung stehen in keiner Bindung, werden aber ebenso unterdrueckt.
    zu_streng = sorted(kz for kz in EM._NULL_UNZULAESSIG_KZ
                       if kz in meta and kz not in V)
    assert not zu_streng, (
        f"VZ {jahr}: {len(zu_streng)} Kz stehen in _NULL_UNZULAESSIG_KZ, obwohl E10-{jahr}.xsd "
        f"die 0 dort erlaubt — ein gehoerender Wert wird unterdrueckt: {zu_streng}")


def test_e0106603_ist_der_gemessene_fall():
    """Die Handprobe zum Befund, in beide Richtungen: dieselbe Kennzahl, zwei Jahrgaenge,
    zwei verschiedene XSD-Typen. Ohne diesen Test waere die Jahresdrift nur eine Zahl aus einem
    Lauf, die beim naechsten Schema-Update still verschwindet.

    Beide Typen stehen im amtlichen Schema der jeweiligen Version; die Basis-Muster sind
    `[1-9].*` (2024) gegen `[1-9].*|0` (2025).
    """
    typen = {}
    for jahr in (2024, 2025):
        pfad = XV._find_schema(jahr)
        if not pfad:
            pytest.skip(f"E10-{jahr}.xsd nicht gefunden — $ERIC_DIR setzen")
        m = XV._resolve_kz_meta(pfad).get("E0106603")
        assert m, f"E0106603 fehlt im E10-{jahr}.xsd"
        typen[jahr] = m

    assert typen[2024]["type_name"].startswith("GanzzahlPosOhneFuehrNull"), typen[2024]
    assert typen[2025]["type_name"].startswith("GanzzahlNichtNegOhneFuehrNull"), typen[2025]
    assert _verbietet_null(typen[2024]), "2024 muss die 0 verbieten"
    assert not _verbietet_null(typen[2025]), "2025 muss die 0 erlauben"

    # Und die Bindung faehrt das Feld ueber BEIDE Jahre — genau daraus entsteht die Luecke.
    feld = _felder_mit_vz().get("rentner_pflege_weitere_personen")
    assert feld, "rentner_pflege_weitere_personen fehlt in der Bindung"
    assert feld["kz"] == "E0106603"
    assert {"2024", "2025"} <= feld["vz"], feld["vz"]
