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
                    if kz not in EM.null_unzulaessig(jahr))
    assert not luecke, (
        f"VZ {jahr}: {len(luecke)} Kz verbieten die 0 laut E10-{jahr}.xsd, stehen aber nicht in "
        f"der Jahresmenge — eine 0 geht dort hinaus und macht die Erklaerung "
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
    zu_streng = sorted(kz for kz in EM.null_unzulaessig(jahr)
                       if kz in meta and kz not in V)
    assert not zu_streng, (
        f"VZ {jahr}: {len(zu_streng)} Kz stehen in der Jahresmenge, obwohl E10-{jahr}.xsd "
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


# ---- Der Zugriff selbst: drei Faelle, kein stiller Rueckfall -----------------
# Die Jahresdrift ist nur die Haelfte. Die andere Haelfte ist, dass ein Jahr, das die
# Zuordnung nicht kennt, NICHT stillschweigend eine fremde Menge bekommt.


def test_unbekanntes_jahr_bekommt_die_vereinigung_nicht_den_fehler():
    """VZ 2026 und spaeter: die VEREINIGUNG aller bekannten Jahre, kein Fehler.

    Das Produkt rechnet VZ 2026 (params/2026 ist vollstaendig); eine 2026er Erklaerung laeuft
    heute bis ERiC und bekommt dort 610001042. Ein harter Fehler hier waere ein Rueckschritt auf
    einem Pfad, der funktioniert. Der Preis ist eine erlaubte 0, die weggelassen wird — und weil
    0 in diesen Kz "nichts anzugeben" heisst (minOccurs 0), ist der Verlust null.
    """
    for jahr in (2026, 2027, 2030, 2100):
        menge = EM.null_unzulaessig(jahr)
        assert menge, f"VZ {jahr} lieferte eine leere Menge — das waere der stille Rueckfall"
        assert "E0106603" in menge, (
            f"VZ {jahr} traegt E0106603 nicht — die Vereinigung muss die 2024er Sicht enthalten, "
            "sonst geht dort eine 0 hinaus, die 2024 verbietet")


def test_vereinigung_ist_obermenge_jeder_einzelmenge():
    """Maschinell, nicht abgeschrieben: die Vereinigung wird gegen die Einzelmengen gehalten.

    Eine handgepflegte Vereinigungsmenge driftet beim naechsten Jahres-XSD still auseinander.
    Dieser Test leitet sie aus `_NULL_UNZULAESSIG_KZ` selbst ab und prueft die Teilmengen-
    beziehung fuer JEDES gefuehrte Jahr.
    """
    vereinigung = EM.null_unzulaessig(2026)
    assert EM._NULL_UNZULAESSIG_KZ, "keine Jahresmengen vorhanden"
    for jahr, menge in EM._NULL_UNZULAESSIG_KZ.items():
        fehlend = sorted(menge - vereinigung)
        assert not fehlend, (
            f"die Vereinigung (VZ 2026) ist keine Obermenge der Menge fuer VZ {jahr} — "
            f"{len(fehlend)} Kz fehlen: {fehlend}")
    # Und die Vereinigung ist echt groesser als jede Einzelmenge, sonst prueft der Test nichts.
    assert any(len(vereinigung) > len(m) for m in EM._NULL_UNZULAESSIG_KZ.values()), (
        "die Vereinigung ist so gross wie jede Einzelmenge — dann traegt sie keinen Jahresbezug")


def test_fehlendes_oder_nulljahr_ist_ein_fehler():
    """Fail-closed in der anderen Richtung: kein Jahr, Jahr 0, kein Steuerjahr -> Fehler.

    `store.get("veranlagungszeitraum") or 0` in haut/api.py liefert die 0, wenn das Feld fehlt.
    Sie darf nicht in eine Jahresmenge greifen. Und die 10^38 eines Korpusfalls ist kein
    "spaeter als 2025" — sie darf die Vereinigung nicht durch die Hintertuer bekommen.
    """
    for schlecht in (None, 0, -5, 1900, 2101, 10**38, True, "2025", 2025.0):
        with pytest.raises(ValueError):
            EM.null_unzulaessig(schlecht)
    # Gegenprobe: die gueltigen Jahre werfen NICHT.
    for gut in (2024, 2025, 2026, 2100):
        assert EM.null_unzulaessig(gut)


def test_deklariere_ohne_vz_ist_ein_fehler():
    """vz ist Pflicht-Schluesselwort an deklariere() — kein Vorgabewert, kein None, kein
    Rueckfall auf 2025. Ohne diesen Test waere ein spaeterer Vorgabewert unbemerkt."""
    with pytest.raises(TypeError):
        EM.deklariere({}, {})                      # vz fehlt ganz
    with pytest.raises(ValueError):
        EM.deklariere({}, {}, vz=0)                # Jahr 0 = "Feld fehlt im Store"
    with pytest.raises(ValueError):
        EM.deklariere({}, {}, vz=10**38)           # der Korpusfall, kein Steuerjahr


def test_der_belegfall_am_echten_schreibweg():
    """E0106603 durch das ECHTE deklariere(), nicht nur durch den Zugriff.

    Das ist die Probe, die zaehlt: 2024 wird die 0 unterdrueckt, 2025 geht sie hinaus.
    Ein Zugriff, der die richtigen Mengen liefert, aber am Schreibweg nicht ankommt, waere
    die haelfte des Fixes — und saehe im Zugriffstest trotzdem gruen aus.
    """
    bindung = _bindung_oder_skip()
    probe = {
        "rentner_pflege_weitere_personen": {"wert": 0, "zustand": "bestaetigt", "herkunft": "t"},
        "rentner_pflegegrad": {"wert": 3, "zustand": "bestaetigt", "herkunft": "t"},
    }
    d24 = EM.deklariere(probe, bindung, vz=2024)["deklaration"]
    d25 = EM.deklariere(probe, bindung, vz=2025)["deklaration"]
    assert "E0106603" not in d24, (
        "VZ 2024: die 0 ging in E0106603 hinaus, obwohl E10-2024.xsd sie verbietet — "
        "die Erklaerung waere uneinreichbar")
    assert d25.get("E0106603") == 0, (
        f"VZ 2025: die 0 fehlt in E0106603, obwohl E10-2025.xsd sie verlangt "
        f"(geschrieben: {d25.get('E0106603')!r})")
    # Gegenprobe mit einer NICHT-null: 2024 muss sehr wohl schreiben, sonst unterdrueckt
    # der Fix den ganzen Schreibweg statt nur die Null.
    probe["rentner_pflege_weitere_personen"]["wert"] = 2
    assert EM.deklariere(probe, bindung, vz=2024)["deklaration"]["E0106603"] == 2


def _bindung_oder_skip():
    sys.path.insert(0, str(ROOT / "produkt" / "traverser"))
    import traverser as TR   # noqa: PLC0415
    return TR.lade_bindung()
