"""VaSt-Belege → TaxGraph-Feld-IDs.

Der Abruf selbst braucht Hersteller-ID und Zertifikat und ist nicht verdrahtet. Das
Mapping lässt sich trotzdem prüfen: gegen das amtliche Schema (existieren die
VaSt-Elemente?) und gegen die Bindungstabelle (existieren die Ziel-Felder?).

Die Einheiten-Umrechnung Euro→Cent ist der kritische Teil — VaSt liefert "45000.00",
der Store erwartet 4500000.
"""
import glob
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "import"))

import vast_mapping as VM   # noqa: E402


# ----------------------------------------------------------------- Einheiten

@pytest.mark.parametrize("euro,cent", [
    ("45000.00", 4500000),
    ("0.00", 0),
    ("1.23", 123),
    ("999999.99", 99999999),
    ("45000,00", 4500000),      # Komma als Dezimaltrenner
    ("  1234.50  ", 123450),    # Leerzeichen
    ("-500.00", -50000),        # negativ (Bruttoarbeitslohn darf negativ sein)
])
def test_euro_nach_cent(euro, cent):
    assert VM._cent(euro) == cent


def test_rundung_ohne_gleitkomma_fehler():
    """Decimal statt float: 0.29 * 100 wäre in float 28.999999999999996."""
    assert VM._cent("0.29") == 29
    assert VM._cent("1.10") == 110
    assert VM._cent("8.70") == 870


def test_fehlender_wert_ist_none_nicht_null():
    """Kein Wert ≠ Wert 0. Eine bescheinigte Null ist eine Aussage, ein fehlendes
    Feld nicht — minOccurs=0 ist im VaSt-Schema durchgängig."""
    assert VM._cent(None) is None
    assert VM._cent("") is None
    assert VM._cent("   ") is None


def test_unlesbarer_betrag_faellt_auf():
    """Lieber ein Fehler als eine stille 0 — ein verschluckter Betrag wäre
    Unterbesteuerung, die niemand bemerkt."""
    with pytest.raises(ValueError, match="nicht lesbar"):
        VM._cent("keine Zahl")


# ----------------------------------------------------------------- LStB

def test_lstb_uebersetzt_bekannte_felder():
    ev = VM.aus_lstb({"BruttoArbLohn": "45000.00", "LSteuer": "8200.50"})
    per_feld = {e["feld_id"]: e["wert"] for e in ev}
    assert per_feld["bruttoarbeitslohn"] == 4500000
    assert per_feld["p36_lohnsteuer"] == 820050


def test_lstb_ueberspringt_leere_felder():
    ev = VM.aus_lstb({"BruttoArbLohn": "45000.00", "LSteuer": ""})
    assert [e["feld_id"] for e in ev] == ["bruttoarbeitslohn"]


def test_lstb_ignoriert_unbekannte_elemente():
    """Das Schema kennt mehr Felder als wir mappen — unbekannte sind kein Fehler."""
    ev = VM.aus_lstb({"BruttoArbLohn": "1000.00", "GibtsNichtInUnseremMapping": "5"})
    assert len(ev) == 1


def test_lstb_kategorie_nennt_herkunft():
    """Die Kategorie landet in signal_1 und begründet später den Wert im Bescheid."""
    ev = VM.aus_lstb({"BruttoArbLohn": "1000.00"})
    assert "LStB/BruttoArbLohn" in ev[0]["kategorie"]
    assert "Bruttoarbeitslohn" in ev[0]["kategorie"]


def test_lstb_leerer_beleg_ergibt_nichts():
    assert VM.aus_lstb({}) == []


# ----------------------------------------------------------------- LErsL

def test_lersl_summiert_leistungen():
    """§ 32b Abs. 1 Nr. 1: der Ring führt EIN Aggregat, kein Feld je Leistungsart."""
    ev = VM.aus_lersl([{"Betrag": "1200.00", "Art": "Arbeitslosengeld"},
                       {"Betrag": "800.00", "Art": "Krankengeld"}])
    assert len(ev) == 1
    assert ev[0]["feld_id"] == "p32b_progressionseinkuenfte"
    assert ev[0]["wert"] == 200000


def test_lersl_nennt_die_arten():
    ev = VM.aus_lersl([{"Betrag": "1200.00", "Art": "Arbeitslosengeld"}])
    assert "Arbeitslosengeld" in ev[0]["kategorie"]


def test_lersl_ohne_leistungen_ergibt_nichts():
    assert VM.aus_lersl([]) == []
    assert VM.aus_lersl(None) == []


def test_lersl_nullsumme_ergibt_nichts():
    """Nur Null-Beträge → kein Event. Ein 0-€-Progressionseinkommen zu schreiben
    würde eine Aussage treffen, die der Beleg nicht macht."""
    assert VM.aus_lersl([{"Betrag": "0.00", "Art": "x"}]) == []


# ----------------------------------------------------------------- Naht zur Bindung

def _alle_feld_ids() -> set:
    ids = set()
    for f in glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")):
        import yaml
        d = yaml.safe_load(open(f, encoding="utf-8")) or {}
        for b in (d.get("bindungen") or []):
            ids.add(b["feld_id"])
    return ids


def test_alle_ziel_felder_existieren_in_der_bindung():
    """Ein Mapping auf ein Feld, das es nicht gibt, schreibt ins Leere: der Store
    lehnt ab, der Nutzer sieht nichts, und niemand merkt es. Genau die Nahtstelle,
    die in diesem Projekt schon mehrfach Fehler getragen hat."""
    vorhanden = _alle_feld_ids()
    ziele = {feld for feld, _doku in VM.LSTB.values()} | {VM.LERSL_BETRAG_FELD}
    fehlend = sorted(z for z in ziele if z not in vorhanden)
    assert not fehlend, f"Mapping zeigt auf nicht existierende Felder: {fehlend}"


def test_alle_ziel_felder_sind_cent():
    """VaSt liefert Euro, wir rechnen in Cent — das Ziel MUSS ein cent-Feld sein.
    Ein Mapping auf ein int-Feld (z.B. eine Anzahl) wäre ein Einheiten-Fehler."""
    import yaml
    typen = {}
    for f in glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(f, encoding="utf-8")) or {}
        for b in (d.get("bindungen") or []):
            typen[b["feld_id"]] = b.get("typ")
    ziele = {feld for feld, _doku in VM.LSTB.values()} | {VM.LERSL_BETRAG_FELD}
    falsch = {z: typen.get(z) for z in ziele if typen.get(z) != "cent"}
    assert not falsch, f"Ziel-Felder ohne typ=cent: {falsch}"


# ----------------------------------------------------------------- Schema-Abgleich

_SCHEMA_DIR = os.path.join(
    os.path.expanduser("~"), "02_Software", "eric", "doc_extract", "ERiC-44.2.4.0",
    "Dokumentation", "Datenarten", "ElsterDatenabholung", "ElsterVaStDaten",
    "VaSt-Belege", "Schema", "2025")

braucht_schema = pytest.mark.skipif(
    not os.path.isdir(_SCHEMA_DIR),
    reason="ERiC-Schemaverzeichnis nicht vorhanden ($ERIC_DIR / ~/02_Software/eric)")


@braucht_schema
def test_gemappte_vast_elemente_existieren_im_schema():
    """Jeder VaSt-Elementname im Mapping muss im amtlichen Schema vorkommen.

    Fängt Tippfehler und Umbenennungen zwischen Jahrgängen — ein Mapping auf ein
    Element, das der Beleg nicht liefert, bleibt sonst stumm leer.
    """
    import re
    treffer = glob.glob(os.path.join(_SCHEMA_DIR, "VaSt_LStB-*.xsd"))
    haupt = [p for p in treffer if "Nutzdaten" not in p]
    if not haupt:
        pytest.skip("LStB-Schema nicht gefunden")
    xsd = open(haupt[0], encoding="utf-8").read()
    im_schema = set(re.findall(r'name="([A-Za-z_0-9]+)"', xsd))
    fehlend = sorted(n for n in VM.LSTB if n not in im_schema)
    assert not fehlend, f"Nicht im LStB-Schema 2025: {fehlend}"


def test_luecken_sind_benannt():
    """Was nicht gemappt ist, muss mit Grund dastehen — schweigendes Weglassen
    wäre ein Datenverlust, den niemand bemerkt."""
    assert VM.NICHT_GEMAPPT, "Lückenliste ist leer — das wäre unglaubwürdig"
    for schluessel, grund in VM.NICHT_GEMAPPT.items():
        assert len(grund) > 40, f"{schluessel}: Begründung zu dünn"


# ----------------------------------------------------------------- Naht zum Writer

def test_mapping_ausgabe_passt_in_den_writer():
    """Ende-zu-Ende: VaSt-Beleg → Mapping → uebernehme_edaten → Store.

    Die Übergabe zwischen Mapping und Writer ist die Naht, an der in diesem Projekt
    schon mehrfach Fehler saßen: beide Seiten für sich korrekt, das Format dazwischen
    ungeprüft. uebernehme_edaten erwartet [{"feld_id", "wert", "kategorie"}].
    """
    sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))
    import store as ST
    import elster_writer as EW

    s = ST.leerer_store(2025, fall_id="vast-e2e")
    events = VM.aus_lstb({"BruttoArbLohn": "45000.00", "LSteuer": "8200.00"})
    n = EW.uebernehme_edaten(s, events, ts="2026-01-01T00:00:00Z")

    assert n == 2, f"2 Felder erwartet, {n} geschrieben"
    felder, _ = ST.materialisiere(s)
    assert felder["bruttoarbeitslohn"]["wert"] == 4500000
    assert felder["bruttoarbeitslohn"]["zustand"] == "bestaetigt"   # § 150 Abs. 7 S. 2 AO
    assert felder["bruttoarbeitslohn"]["herkunft"]["herkunft"] == "edaten"


def test_eigene_angabe_schlaegt_edaten():
    """§ 150 Abs. 7 S. 2 AO: eine abweichende Angabe des Steuerpflichtigen hat Vorrang.
    Der Writer überschreibt kein aktives Event — hier gegen das Mapping geprüft, nicht
    nur gegen eine handgebaute Liste."""
    sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))
    import store as ST
    import elster_writer as EW

    s = ST.leerer_store(2025, fall_id="vast-vorrang")
    ST.append_event(s, feld_id="bruttoarbeitslohn", wert=5000000, zustand="bestaetigt",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="laie", signal={"signal_1": {"typ": "laie_eingabe"},
                                              "signal_2": "laie_bestaetigt"})
    EW.uebernehme_edaten(s, VM.aus_lstb({"BruttoArbLohn": "45000.00"}))

    felder, _ = ST.materialisiere(s)
    assert felder["bruttoarbeitslohn"]["wert"] == 5000000, \
        "eDaten haben die eigene Angabe überschrieben — § 150 Abs. 7 S. 2 AO verletzt"


# ----------------------------------------------------------------- Summen-Felder

def test_kv_und_pv_getrennt_statt_summiert():
    """§ 10 Abs. 1 Nr. 3: KV und PV sind jetzt getrennte Felder (Feldsplit 2026-08)."""
    ev = VM.aus_lstb({"ArbnAnteilKrankVers": "3200.00", "ArbnAnteilPflegVers": "800.00"})
    per_feld = {e["feld_id"]: e["wert"] for e in ev}
    assert per_feld["basis_kv"] == 320000
    assert per_feld["basis_pv"] == 80000
    assert "basis_kv_pv" not in per_feld, "Summenfeld existiert nicht mehr"


def test_nur_kv_ohne_pv_wird_uebernommen():
    """KV allein, ohne PV — wird als basis_kv geschrieben, basis_pv bleibt leer."""
    ev = VM.aus_lstb({"ArbnAnteilKrankVers": "3200.00"})
    per_feld = {e["feld_id"]: e["wert"] for e in ev}
    assert per_feld["basis_kv"] == 320000
    assert "basis_pv" not in per_feld


def test_arbeitslosenversicherung_nicht_in_der_basis():
    """§ 10 Abs. 1 Nr. 3a nennt die Arbeitslosenversicherung ausdrücklich neben den
    Kranken-/Pflegebeiträgen. KV und PV erscheinen getrennt, die AV nicht in der Basis."""
    ev = VM.aus_lstb({"ArbnAnteilKrankVers": "3200.00",
                      "ArbnAnteilPflegVers": "800.00",
                      "ArbnAnteilArblVers": "1200.00"})
    per_feld = {e["feld_id"]: e["wert"] for e in ev}
    assert per_feld["basis_kv"] == 320000
    assert per_feld["basis_pv"] == 80000
    assert per_feld["vorsorge_arbeitslosenversicherung"] == 120000
    # Probe: keine Vermischung
    assert "basis_kv_pv" not in per_feld


def test_kv_und_pv_kategorie_nennt_herkunft():
    """KV und PV haben eigene Kategorien (nicht mehr summiert)."""
    ev = VM.aus_lstb({"ArbnAnteilKrankVers": "3200.00", "ArbnAnteilPflegVers": "800.00"})
    per_feld = {e["feld_id"]: e for e in ev}
    assert "ArbnAnteilKrankVers" in per_feld["basis_kv"]["kategorie"]
    assert "ArbnAnteilPflegVers" in per_feld["basis_pv"]["kategorie"]


def test_kein_summenfeld_ohne_quelldaten():
    ev = VM.aus_lstb({"BruttoArbLohn": "45000.00"})
    assert "basis_kv" not in {e["feld_id"] for e in ev}
    assert "basis_pv" not in {e["feld_id"] for e in ev}


def test_summen_ziele_existieren_und_sind_cent():
    """Dieselbe Naht-Prüfung wie für die 1:1-Felder — ein Summenfeld, das auf eine
    nicht existierende feld_id zeigt, schriebe still ins Leere."""
    import yaml
    typen = {}
    for f in glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(f, encoding="utf-8")) or {}
        for b in (d.get("bindungen") or []):
            typen[b["feld_id"]] = b.get("typ")
    falsch = {z: typen.get(z) for z in VM.LSTB_SUMMEN if typen.get(z) != "cent"}
    assert not falsch, f"Summen-Ziele ohne typ=cent: {falsch}"


@braucht_schema
def test_summen_quellen_existieren_im_schema():
    import re
    haupt = [p for p in glob.glob(os.path.join(_SCHEMA_DIR, "VaSt_LStB-*.xsd"))
             if "Nutzdaten" not in p]
    if not haupt:
        pytest.skip("LStB-Schema nicht gefunden")
    im_schema = set(re.findall(r'name="([A-Za-z_0-9]+)"',
                               open(haupt[0], encoding="utf-8").read()))
    fehlend = sorted(n for quellen in VM.LSTB_SUMMEN.values()
                     for n, _d in quellen if n not in im_schema)
    assert not fehlend, f"Nicht im LStB-Schema 2025: {fehlend}"
