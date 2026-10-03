"""ELSTER-Zeichensatz beim Speichern (Ticket elster-zeichensatz-strenger-als-xml, Decision
elster-zeichensatz-beim-speichern-abweisen): ein Textwert mit einem Zeichen ausserhalb des Zeichensatzes
„Standard_E_V2" wird abgewiesen, mit dem Zeichen UND einem Vorschlag in der Meldung. Umlaute, ß und € gehen
durch. Dieselbe Stelle und derselbe 422-Weg wie beim Steuerzeichen-Fix (2c17f70); Rust-Gegenstueck:
rust/domain/src/zeichensatz.rs, rust/store (Abweisung::ZeichensatzVerletzt).

Die Zeichenmenge steht WOERTLICH im Schema (E10-2025.xsd:1810 `StringZUBaseCType`, :1792 `StringBaseCType`);
`test_menge_gleicht_dem_xsd` vergleicht sie mit dem Schema, Zeichen fuer Zeichen.
"""
from __future__ import annotations

import glob
import os
import re
import sys
import xml.etree.ElementTree as ET

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "produkt/eingang", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API                    # noqa: E402
import audit                         # noqa: E402
import store as ST                   # noqa: E402
import vorjahr_writer as VJ          # noqa: E402
import xsd_verify as XV              # noqa: E402
import zeichensatz as ZS             # noqa: E402

TS = "2026-10-03T12:00:00+00:00"
_SCHEMA = XV._find_schema(2025)
braucht_schema = pytest.mark.skipif(not _SCHEMA, reason="E10-2025.xsd fehlt (ERiC-Schemas nicht installiert)")


def _bindung() -> dict:
    b = {}
    for f in glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")):
        for e in (yaml.safe_load(open(f, encoding="utf-8")).get("bindungen") or []):
            b[e["feld_id"]] = e
    return b


BINDUNG = _bindung()
HERKUNFT = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt", "herkunft": dict(HERKUNFT),
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "zs"})
    assert st == 201, r
    return "zs"


# ------------------------------------------------------------------ die Menge gleicht dem Schema

def _xsd_muster(typ: str) -> str:
    wurzel = ET.parse(_SCHEMA).getroot()
    xs = "{http://www.w3.org/2001/XMLSchema}"
    for ct in wurzel.iter(xs + "complexType"):
        if ct.get("name") == typ:
            return next(p.get("value") for p in ct.iter(xs + "pattern"))
    raise AssertionError(f"{typ} nicht im Schema")


@braucht_schema
def test_menge_gleicht_dem_xsd():
    """Die Menge des Stores ist die des Schemas: StringZUBaseCType (E10-2025.xsd:1810) ohne die
    Zeilenumbrueche, die StringBaseCType (:1792) verbietet. Geprueft wird jedes Zeichen der Ebenen 0 bis
    2 (U+0000..U+2FFFF) sowie U+E000..U+FFFF; eine Abschrift mit einem Tippfehler wird rot."""
    zu = re.compile(_xsd_muster("StringZUBaseCType"))
    base = re.compile(_xsd_muster("StringBaseCType"))
    assert base.pattern == "[^\n\r]+", base.pattern        # :1792 — nur CR/LF kommen dazu
    falsch = []
    for cp in [*range(0x30000), *range(0xE000, 0x10000)]:
        c = chr(cp)
        im_schema = zu.fullmatch(c) is not None and base.fullmatch(c) is not None
        if (ZS.erstes_unerlaubtes_zeichen(c) is None) != im_schema:
            falsch.append(f"U+{cp:04X}")
    assert not falsch, f"Menge weicht vom Schema ab: {falsch[:20]} (insgesamt {len(falsch)})"


@braucht_schema
def test_jedes_textfeld_der_bindung_endet_im_zeichensatz():
    """Warum die Regel am Feldtyp `text` haengt und nicht an der Kz: jedes `typ: text`-Feld endet im Schema
    auf StringBaseCType (479 Kz tun das) oder hat ein engeres Bindungsmuster (Datumsbereiche, IdNr, BIC).
    Kommt ein Textfeld dazu, das beides nicht hat, wird dieser Test rot — dann entscheidet jemand, ob die
    Regel fuer dieses Feld gilt."""
    top = XV._parse_top_level_children(_SCHEMA)
    typen, _, _ = XV._load_indices(top)
    xs = XV.XS
    meta = XV._resolve_kz_meta(_SCHEMA)

    def auf_string_base(kz: str) -> bool:
        name, gesehen = meta[kz]["type_name"], set()
        while name in typen and name not in gesehen:
            if name == "StringBaseCType":
                return True
            gesehen.add(name)
            name = next((ch.get("base") for c in typen[name] if c.tag == xs + "simpleContent"
                         for ch in c if ch.tag in (xs + "restriction", xs + "extension")), None)
        return name == "StringBaseCType"

    # Textfelder ohne eigenes `elster_kz`: ihr Ziel steht in elster_kz_grund (Verzweigung/IBAN), geprueft hier
    ziele_ohne_kz = {"gewinn_bezeichnung": ("E0800301", "E0803101"),
                     "gewinn_bezeichnung_partner": ("E0800301", "E0803101"),
                     "stammdaten_iban": ("E0102102", "E0102603")}
    nur_ziffern = {"stammdaten_steuernummer", "person_b_idnr"}   # Vorsatz-StNr bzw. IdNr: Ziffernmuster im Schema
    schlecht = []
    for fid, e in sorted(BINDUNG.items()):
        if e.get("typ") != "text":
            continue
        kz = e.get("elster_kz")
        if kz and kz in meta:
            ok = auf_string_base(kz) or bool(e.get("muster"))
        elif fid in ziele_ohne_kz:
            ok = all(auf_string_base(k) for k in ziele_ohne_kz[fid])
        else:
            ok = fid in nur_ziffern
        if not ok:
            schlecht.append(fid)
    assert not schlecht, f"Textfelder ausserhalb des ELSTER-Zeichensatzes: {schlecht}"


# ------------------------------------------------------------------ AK1/AK2: abweisen, Zeichen + Vorschlag

# (Name, Wert, Zeichen laut Meldung, Vorschlag laut Meldung) -- die Faelle aus dem Ticket
_VERBOTEN = [
    ("geschuetztes Leerzeichen", "Kowalski Anna", "geschütztes Leerzeichen (U+00A0)", "ein normales Leerzeichen"),
    ("Gedankenstrich", "Müller–Straße", "„–\" (U+2013)", "„-\""),
    ("l mit Strich", "Wałesa", "„ł\" (U+0142)", "„l\""),
    ("grosses L mit Strich", "Łukasz", "„Ł\" (U+0141)", "„L\""),
    ("Tabulator", "Maier\tMüller", "Tabulator (U+0009)", "ein normales Leerzeichen"),
    ("Zeilenumbruch", "Maier\nMüller", "Zeilenumbruch (U+000A)", "ein normales Leerzeichen"),
    ("Wagenruecklauf", "Maier\rMüller", "Zeilenumbruch (U+000D)", "ein normales Leerzeichen"),
    ("typografisches Anfuehrungszeichen unten", "„Müller", "„„\" (U+201E)", "„\"\""),
    ("typografischer Apostroph", "O’Brien", "„’\" (U+2019)", "„'\""),
    ("Auslassungspunkte", "Fortsetzung…", "„…\" (U+2026)", "„...\""),
    ("Breite-null-Leerzeichen", "Mül​ler", "Leerzeichen der Breite null (U+200B)", "lösche es"),
    ("loser Akzent (macOS)", "Müller", "loser Akzent (U+0308)", "als ein Zeichen"),
    ("kyrillisch (allgemeiner Rat)", "Петр", "„П\" (U+041F)", "ersetze es durch ein Zeichen"),
    ("DEL (Steuerzeichen)", "Maier\x7fMüller", "Steuerzeichen (U+007F)", "ersetze es durch ein Zeichen"),
    ("NEL (C1-Steuerzeichen)", "Maier\x85Müller", "Steuerzeichen (U+0085)", "ein normales Leerzeichen"),
    ("Zeilentrenner", "Maier Müller", "unsichtbares Zeichen (U+2028)", "ein normales Leerzeichen"),
    ("Leerzeichen anderer Breite", "Maier Müller", "Leerzeichen besonderer Breite (U+2003)",
     "ein normales Leerzeichen"),
]


@pytest.mark.parametrize("name,wert,zeichen,vorschlag", _VERBOTEN, ids=[f[0] for f in _VERBOTEN])
def test_unerlaubtes_zeichen_wird_abgewiesen_mit_zeichen_und_vorschlag(fall, name, wert, zeichen, vorschlag):
    """AK1 + AK2 durch den echten Speicherweg (api.event, 422): die Meldung nennt das Feld, das Zeichen
    (mit Codepunkt) und einen Vorschlag — nie den ganzen Wert (PII). Der Store bleibt unveraendert."""
    vorher = API.lade_fall(fall)
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("stammdaten_nachname", wert))
    assert exc.value.status == 422
    meldung = str(exc.value)
    assert "fail-closed (Zeichensatz)" in meldung and "stammdaten_nachname" in meldung, meldung
    assert zeichen in meldung, f"das Zeichen fehlt: {meldung}"
    assert vorschlag in meldung, f"der Vorschlag fehlt: {meldung}"
    assert "Müller" not in meldung and "Maier" not in meldung and "Kowalski" not in meldung, meldung
    assert API.lade_fall(fall) == vorher, "ein abgewiesener Wert darf nichts schreiben"


def test_ohne_vorschlag_waere_ein_polnischer_name_nicht_abgebbar(fall):
    """Gegenprobe zu AK2 (Decision: der Vorschlag loest den Einwand auf): folgt der Nutzer dem Vorschlag,
    geht der Name durch. Die Abweisung ist eine Hilfe, keine Sackgasse."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("stammdaten_nachname", "Wałęsa"))
    assert "„l\"" in str(exc.value)
    st, r = API.event(fall, _laie("stammdaten_nachname", "Walesa"))
    assert st == 201, r


def test_store_weist_direkt_ab(fall):
    """Dieselbe Regel am Store selbst (jeder Schreiber geht hier durch: HTTP, Beleg, Kontoauszug, eDaten)."""
    s = ST.leerer_store(2025, fall_id="zs-direkt")
    with pytest.raises(ValueError, match=r"fail-closed \(Zeichensatz\): stammdaten_nachname enthält"):
        ST.append_event(s, feld_id="stammdaten_nachname", wert="Maier–Müller", zustand="bestaetigt",
                        herkunft=dict(HERKUNFT), schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": "ok"}, ts=TS, bindung=BINDUNG)
    assert s["events"] == []


def test_steuerzeichen_behaelt_seine_eigene_meldung(fall):
    """Reihenfolge der Auflagen: ein NUL ist ein Steuerzeichen (Auflage T) und nicht erst ein Zeichensatz-Fehler;
    die bestehende Meldung bleibt, wie sie ist."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("stammdaten_nachname", "Maier\x00"))
    assert "fail-closed (Typ)" in str(exc.value) and "Steuerzeichen" in str(exc.value)


# ------------------------------------------------------------------ AK3: Erlaubtes bleibt erlaubt

# Umlaute und ß liegen in U+00BF..U+00FF, nicht im ASCII-Block; € in U+20AC; Œ Š Ž Ÿ in den Einzelbereichen.
_ERLAUBT = [
    "Müller", "Größe-Öl Ärger Übel", "ÄÖÜäöüß", "Straße 5", "5 €", "Žilina Šmíd", "Œuvre Ÿ œ š ž",
    "O'Brien \"Zitat\"", "§ 35a Abs. 2", "¡Hola! ¿Qué?", "ç à é è ñ ø å æ ð þ", "Müller-Lüdenscheidt",
    "a" * 300, "~ und ` und ^",
]


@pytest.mark.parametrize("wert", _ERLAUBT)
def test_erlaubte_zeichen_gehen_durch(fall, wert):
    """AK3, die Gegenrichtung zu AK1: ohne sie belegte AK1 nur, dass irgendetwas abgewiesen wird."""
    st, r = API.event(fall, _laie("stammdaten_nachname", wert))
    assert st == 201, r
    assert API.lade_fall(fall)["events"][-1]["wert"] == wert


def test_jedes_erlaubte_zeichen_geht_einzeln_durch():
    """Alle Zeichen der Menge, jedes allein durch den Store (nicht nur Beispiele): U+0020..U+007E, die
    Latin-1-Bereiche, Œœ Šš Ÿ Žž und €. Zeilenumbruch und Wagenruecklauf sind nicht dabei (StringBaseCType)."""
    erlaubt = [chr(c) for c in [*range(0x20, 0x7F), 0xA1, 0xA2, 0xA3, 0xA5, 0xA7, *range(0xAA, 0xAD),
                                *range(0xAE, 0xB4), 0xB5, *range(0xB9, 0xBC), *range(0xBF, 0x100),
                                0x152, 0x153, 0x160, 0x161, 0x178, 0x17D, 0x17E, 0x20AC]]
    assert len(erlaubt) == 95 + 3 + 2 + 3 + 6 + 1 + 3 + 65 + 2 + 2 + 1 + 2 + 1 == 186
    for c in erlaubt:
        s = ST.leerer_store(2025, fall_id="zs-erlaubt")
        ST.append_event(s, feld_id="stammdaten_nachname", wert=f"a{c}b", zustand="bestaetigt",
                        herkunft=dict(HERKUNFT), schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": "ok"}, ts=TS, bindung=BINDUNG)


def test_andere_feldtypen_sind_unberuehrt(fall):
    """Die Regel gilt fuer `typ: text`. Eine Zahl, ein Wahrheitswert, eine Auswahl haben ihre eigenen Pruefungen."""
    assert API.event(fall, _laie("bruttoarbeitslohn", 5000000))[0] == 201
    assert API.event(fall, _laie("veranlagung", "einzel"))[0] == 201


# ------------------------------------------------------------------ Vorjahr: Altwerte werden uebersprungen

def test_vorjahr_ueberspringt_einen_altwert_mit_unerlaubtem_zeichen():
    """Ein Altwert, gespeichert vor der Regel, bricht die Uebernahme nicht ab: das Feld bleibt leer und
    steht in `uebersprungen`, die uebrigen Felder kommen an (wie bei Typ, Vorzeichen, Bereich, Format)."""
    neu = ST.leerer_store(2025, fall_id="zs-vj-neu")
    vorjahr = {"rentner_gepflegter_angaben": {"wert": "Mutter–Pflegegrad 3", "zustand": "bestaetigt"},
               "kind_betreuung_dienstleister": {"wert": "Kita Sonnenschein", "zustand": "bestaetigt"}}
    n, uebersprungen = VJ.uebernehme_vorjahr(neu, vorjahr, BINDUNG, vorjahr_vz=2024, ts=TS)
    assert uebersprungen == ["rentner_gepflegter_angaben"], uebersprungen
    assert n == 1


# ------------------------------------------------------------------ Meldungen

def test_meldung_nennt_nie_den_ganzen_wert():
    wert = "Kowalski–Nowak, Hauptstraße 5"
    zeichen = ZS.erstes_unerlaubtes_zeichen(wert)
    assert zeichen == "–"
    meldung = ZS.feld_meldung("stammdaten_nachname", zeichen)
    assert "Kowalski" not in meldung and "Hauptstraße" not in meldung
    el = ZS.element_meldung("E0100201", wert)
    assert el and "Kowalski" not in el and "E0100201" in el and "„-\"" in el


def test_meldung_traegt_nie_ein_unsichtbares_zeichen():
    """Die Meldung geht als 422-detail an den Nutzer und ins Log: ein Steuerzeichen, ein Zeilentrenner oder ein
    Leerzeichen anderer Breite zwischen den Anfuehrungszeichen waere dort unsichtbar (oder risse die Zeile
    auf). Geprueft wird jedes Steuerzeichen (Cc), jedes Leerzeichen und jeder Trenner (Zs, Zl, Zp) bis U+3000
    sowie die unsichtbaren Formatzeichen zwischen U+200B und U+206F, U+00AD und U+FEFF. Grenze: andere
    Formatzeichen (etwa U+0600) zeigt die Meldung mit Glyphe; ihr Codepunkt steht dabei und nennt sie."""
    import unicodedata
    zu_lesen = [chr(cp) for cp in range(0x3001)
                if unicodedata.category(chr(cp)) in ("Cc", "Zs", "Zl", "Zp")
                or cp in (0xAD, 0xFEFF) or (0x200B <= cp <= 0x206F and unicodedata.category(chr(cp)) == "Cf")]
    assert len(zu_lesen) > 100        # die Schleife hat etwas zu pruefen
    for c in zu_lesen:
        if ZS.erstes_unerlaubtes_zeichen(c) is None:
            continue
        for meldung in (ZS.feld_meldung("stammdaten_nachname", c), ZS.element_meldung("E0100201", c)):
            assert meldung.isprintable(), f"U+{ord(c):04X}: {meldung!r}"


def test_vorschlag_ist_selbst_im_zeichensatz():
    """Ein Vorschlag, den ELSTER wieder ablehnt, waere schlimmer als keiner: jeder Ersatz der Tabelle liegt
    im erlaubten Zeichensatz."""
    for ersatz, zeichen_liste in ZS._VORSCHLAEGE:
        assert ZS.erstes_unerlaubtes_zeichen(ersatz) is None, (ersatz, zeichen_liste)
        for z in zeichen_liste:
            assert ZS.erstes_unerlaubtes_zeichen(z) is not None, (
                f"U+{ord(z):04X} steht in der Vorschlagstabelle, ist aber erlaubt")
