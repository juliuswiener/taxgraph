"""Gate für den XSD-Kz-Verifikationspass (produkt/mapping/xsd_verify.py, Task #11).

Design: reports/review/2026-07-20-xsd-kz-verifikationspass-design.md. Prüft H1 (Exit-Code
gate-tauglich), H2 (§1-Geteilt-Typ-Fixture MUSS den VOLLEN Report-Treiber durchlaufen, nicht nur
walk() — non-vacuous), H3 (MAX_DEPTH-Abbruch gezählt + sichtbar + exit 1).
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))
sys.path.insert(0, ROOT)

import xsd_verify as X  # noqa: E402
import traverser as T  # noqa: E402

pytest.importorskip("yaml")

_SCHEMA_2025 = X._find_schema(2025)
requires_real_schema = pytest.mark.skipif(
    _SCHEMA_2025 is None, reason="lokales ERiC-E10-2025.xsd nicht gefunden ($ERIC_DIR/~/02_Software/eric)")

_E77_SCHEMA_2025 = X._find_schema(2025, "E77-{jahr}.xsd")
requires_real_e77_schema = pytest.mark.skipif(
    _E77_SCHEMA_2025 is None, reason="lokales ERiC-E77-2025.xsd nicht gefunden ($ERIC_DIR/~/02_Software/eric)")


def _schreibe_schema(tmp_path, name: str, xml: str) -> str:
    pfad = tmp_path / name
    pfad.write_text(xml)
    return str(pfad)


# ---------------------------------------------------------------- §1-Fixture: geteilter complexType

_GETEILT_XSD = """<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="Envelope" type="EnvelopeType"/>
  <xs:complexType name="EnvelopeType">
    <xs:sequence>
      <xs:element name="Absender" type="AdresseType"/>
      <xs:element name="Empfaenger" type="AdresseType"/>
    </xs:sequence>
  </xs:complexType>
  <xs:complexType name="AdresseType">
    <xs:sequence>
      <xs:element name="E9999001" type="xs:string"/>
    </xs:sequence>
  </xs:complexType>
</xs:schema>
"""


def test_geteilter_typ_walker_liefert_zwei_pfade(tmp_path):
    """Walker-Primitive (§1-Gegenbeweis): 2 unterscheidbare Fundstellen, keine Kollision."""
    pfad = _schreibe_schema(tmp_path, "geteilt.xsd", _GETEILT_XSD)
    kz_index, hits = X.walk(pfad, start_element="Envelope")
    assert hits == 0
    assert kz_index["E9999001"] == [
        ("Envelope", "Absender", "E9999001"),
        ("Envelope", "Empfaenger", "E9999001"),
    ]


def test_h2_geteilter_typ_voller_report_treiber_ambiguous_und_exit_1(tmp_path):
    """H2: NICHT nur len(walk())==2 prüfen (vacuous) — der Report-KONSUMENT muss AMBIGUOUS klassifizieren
    und der Gate wirklich auf exit_code==1 anschlagen (conf_map-Lehre: Primitive != Konsument)."""
    pfad = _schreibe_schema(tmp_path, "geteilt.xsd", _GETEILT_XSD)
    bindung = {
        "feld_x": {"elster_kz": "E9999001", "vz_gueltigkeit": [2099]},
    }
    ergebnis = X.pruefe_bindung(bindung, schema_pfade={2099: pfad}, start_element="Envelope")

    assert ergebnis["felder"]["feld_x"]["status"] == X.STATUS_AMBIGUOUS
    assert ergebnis["felder"]["feld_x"]["jahre"][2099]["status"] == X.STATUS_AMBIGUOUS
    assert len(ergebnis["felder"]["feld_x"]["jahre"][2099]["pfade"]) == 2
    assert ergebnis["exit_code"] == 1


# ---------------------------------------------------------------- NOT_FOUND / OK

_EINFACH_XSD = """<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="Root" type="RootType"/>
  <xs:complexType name="RootType">
    <xs:sequence>
      <xs:element name="Block" type="BlockType"/>
    </xs:sequence>
  </xs:complexType>
  <xs:complexType name="BlockType">
    <xs:sequence>
      <xs:element name="E1111111" type="xs:string"/>
    </xs:sequence>
  </xs:complexType>
</xs:schema>
"""


def test_ok_eindeutiger_pfad(tmp_path):
    pfad = _schreibe_schema(tmp_path, "einfach.xsd", _EINFACH_XSD)
    bindung = {"feld_ok": {"elster_kz": "E1111111", "vz_gueltigkeit": [2099]}}
    ergebnis = X.pruefe_bindung(bindung, schema_pfade={2099: pfad}, start_element="Root")

    assert ergebnis["felder"]["feld_ok"]["status"] == X.STATUS_OK
    assert ergebnis["exit_code"] == 0
    assert ergebnis["max_depth_hits_gesamt"] == 0


def test_not_found_kz_nicht_im_schema(tmp_path):
    pfad = _schreibe_schema(tmp_path, "einfach.xsd", _EINFACH_XSD)
    bindung = {"feld_fehlt": {"elster_kz": "E9999999", "vz_gueltigkeit": [2099]}}
    ergebnis = X.pruefe_bindung(bindung, schema_pfade={2099: pfad}, start_element="Root")

    assert ergebnis["felder"]["feld_fehlt"]["status"] == X.STATUS_NOT_FOUND
    assert ergebnis["exit_code"] == 1


def test_feld_ohne_kz_wird_nie_nachgeschlagen(tmp_path):
    """§3(5): elster_kz: null (mit dokumentiertem Grund) ist kein Sonderfall — einfach nie geprüft."""
    pfad = _schreibe_schema(tmp_path, "einfach.xsd", _EINFACH_XSD)
    bindung = {
        "feld_ohne_kz": {"elster_kz": None, "elster_kz_grund": "keine Kz", "vz_gueltigkeit": [2099]},
    }
    ergebnis = X.pruefe_bindung(bindung, schema_pfade={2099: pfad}, start_element="Root")

    assert ergebnis["felder"] == {}
    assert ergebnis["exit_code"] == 0


# ---------------------------------------------------------------- H3: MAX_DEPTH-Abbruch sichtbar

def _tiefe_kette_xsd(tiefe: int, fruehe_kz: str) -> str:
    """Root -> W1 -> W2 (mit Geschwister-Kz `fruehe_kz`, flach) -> W3 -> ... -> W<tiefe> -> Ende.
    `fruehe_kz` liegt VOR dem MAX_DEPTH-Cutoff (OK), die Kette dahinter überschreitet ihn (Abbruch)."""
    teile = ['<?xml version="1.0" encoding="UTF-8"?>',
             '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">',
             '<xs:element name="Root" type="W1Type"/>']
    for i in range(1, tiefe + 1):
        naechster = f"W{i + 1}Type" if i < tiefe else None
        kinder = ""
        if i == 1:
            kinder += f'<xs:element name="{fruehe_kz}" type="xs:string"/>'
        if naechster:
            kinder += f'<xs:element name="W{i + 1}" type="{naechster}"/>'
        else:
            kinder += '<xs:element name="Blatt" type="xs:string"/>'
        teile.append(f'<xs:complexType name="W{i}Type"><xs:sequence>{kinder}</xs:sequence></xs:complexType>')
    teile.append("</xs:schema>")
    return "".join(teile)


def test_h3_max_depth_hits_gezaehlt_und_sichtbar_auch_bei_sonst_ok_feld(tmp_path):
    xml = _tiefe_kette_xsd(tiefe=X.MAX_DEPTH + 20, fruehe_kz="E2222222")
    pfad = _schreibe_schema(tmp_path, "tief.xsd", xml)
    bindung = {"feld_frueh": {"elster_kz": "E2222222", "vz_gueltigkeit": [2099]}}
    ergebnis = X.pruefe_bindung(bindung, schema_pfade={2099: pfad}, start_element="Root")

    # Das geprüfte Feld selbst liegt VOR dem Cutoff und ist eindeutig OK...
    assert ergebnis["felder"]["feld_frueh"]["status"] == X.STATUS_OK
    # ...trotzdem schlägt der Gate an, weil die Kette dahinter abgebrochen wurde (verpasster Teilbaum).
    assert ergebnis["max_depth_hits_gesamt"] > 0
    assert ergebnis["exit_code"] == 1


def test_max_depth_hits_immer_im_ergebnis_auch_bei_null(tmp_path):
    pfad = _schreibe_schema(tmp_path, "einfach.xsd", _EINFACH_XSD)
    bindung = {"feld_ok": {"elster_kz": "E1111111", "vz_gueltigkeit": [2099]}}
    ergebnis = X.pruefe_bindung(bindung, schema_pfade={2099: pfad}, start_element="Root")
    assert "max_depth_hits_gesamt" in ergebnis
    assert ergebnis["max_depth_hits_gesamt"] == 0


# ---------------------------------------------------------------- VZ2026 "unverifizierbar", nie still

def test_vz_ohne_lokales_schema_wird_gemeldet_nicht_uebersprungen(tmp_path):
    pfad = _schreibe_schema(tmp_path, "einfach.xsd", _EINFACH_XSD)
    bindung = {"feld_mixed": {"elster_kz": "E1111111", "vz_gueltigkeit": [2099, 2026]}}
    ergebnis = X.pruefe_bindung(bindung, schema_pfade={2099: pfad, 2026: None}, start_element="Root")

    assert ergebnis["felder"]["feld_mixed"]["jahre"][2026]["status"] == X.STATUS_UNVERFUEGBAR
    assert 2026 in ergebnis["unverfuegbare_jahre"]
    # 2099 verifiziert OK -> Feld-Rollup bleibt OK, exit 0 (2026 blockt nicht, wird aber sichtbar gemeldet)
    assert ergebnis["felder"]["feld_mixed"]["status"] == X.STATUS_OK
    assert ergebnis["exit_code"] == 0


def test_vz_ausschliesslich_unverfuegbar_ist_kein_ok(tmp_path):
    bindung = {"feld_nur_2026": {"elster_kz": "E1111111", "vz_gueltigkeit": [2026]}}
    ergebnis = X.pruefe_bindung(bindung, schema_pfade={2026: None}, start_element="Root")

    assert ergebnis["felder"]["feld_nur_2026"]["status"] == X.STATUS_UNVERFUEGBAR
    assert ergebnis["exit_code"] == 1


# ---------------------------------------------------------------- xs:include-Auflösung (§3 Punkt 4, defensiv)

def test_include_wird_aufgeloest(tmp_path):
    haupt = """<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:include schemaLocation="typen.xsd"/>
  <xs:element name="Root" type="RootType"/>
  <xs:complexType name="RootType">
    <xs:sequence><xs:element name="E3333333" type="xs:string"/></xs:sequence>
  </xs:complexType>
</xs:schema>
"""
    typen = """<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:complexType name="UngenutztType"><xs:sequence/></xs:complexType>
</xs:schema>
"""
    (tmp_path / "typen.xsd").write_text(typen)
    pfad = _schreibe_schema(tmp_path, "haupt.xsd", haupt)
    kz_index, hits = X.walk(pfad, start_element="Root")
    assert kz_index["E3333333"] == [("Root", "E3333333")]
    assert hits == 0


# ---------------------------------------------------------------- Regressionsfall gegen echtes Schema

@requires_real_schema
def test_real_schema_k_verh_a_b_disambiguierung():
    """Design §4 golden probe: K_Verh_A/E0500807 vs. K_Verh_B/E0500808 — dev-3s ursprüngliche Motivation."""
    kz_index, hits = X.walk(_SCHEMA_2025, start_element="E10")
    assert hits == 0
    assert kz_index["E0500807"] == [("E10", "Kind", "K_Verh", "K_Verh_A", "E0500807")]
    assert kz_index["E0500808"] == [("E10", "Kind", "K_Verh", "K_Verh_B", "E0500808")]


@requires_real_e77_schema
def test_real_bindung_gwg_e77_routing_gegen_echtes_schema_2025():
    """Task #8: E60xx-Kz routen auf E77 statt E10 (E77/EÜR eigene Datenart, dev-2 Root-Cause 2026-07-20)."""
    bindung = T.lade_bindung()
    e77_felder = {fid: b for fid, b in bindung.items()
                  if fid in ("gwg_anschaffungskosten_netto", "sonstige_betriebsausgaben")}
    assert len(e77_felder) == 2, "bindung lieferte nicht beide erwarteten E77-Felder (Fixture-Bruch?)"

    ergebnis = X.pruefe_bindung(e77_felder, schema_pfade={2025: _E77_SCHEMA_2025})

    for fid, f in ergebnis["felder"].items():
        assert f["jahre"][2025]["status"] == X.STATUS_OK, f"{fid}: {f['jahre'][2025]}"


# ---------------------------------------------------------------- Task #13: Verzweigungs-Kz-Ernte
# (Klasse d/f/g×f — NEGATION/VERZWEIGUNG/PARTNER_VERZWEIGUNG in est_mapping.py, Coverage-Audit Task #11:
# diese Ziel-Kz waren zuvor NIE Teil der Iteration, weil sie nur als Python-Literal existieren, nie als
# `elster_kz:` in bindung yaml.)

def test_ernte_est_mapping_kz_negation_und_verzweigung_mit_echtem_vz():
    """Schema-unabhängig: Ernte liest reale bindung yaml (vz_gueltigkeit) + reale est_mapping-Tabellen."""
    bindung = T.lade_bindung()
    synth = X.ernte_est_mapping_kz(bindung)

    assert synth["negation:fam_alleinstehend"] == {
        "elster_kz": "E0503701", "vz_gueltigkeit": bindung["fam_alleinstehend"]["vz_gueltigkeit"]}
    assert synth["verzweigung:rentner_jahresrente:sonstige_leibrente"] == {
        "elster_kz": "E1803102", "vz_gueltigkeit": bindung["rentner_jahresrente"]["vz_gueltigkeit"]}
    # einkuenfte_gewinn hat ein LEERES kz-dict (Kz-Review noch offen) -> bewusst KEINE Synth-Einträge
    assert not any(fid.startswith("verzweigung:einkuenfte_gewinn:") for fid in synth)


@requires_real_schema
def test_real_verzweigung_kz_geerntet_und_gegen_echtes_schema_ok():
    """Task #13: ein geernteter Klasse-f-Ziel-Kz (rentner_jahresrente, unstrittiger Zweig) MUSS durch den
    vollen Report-Treiber laufen und OK sein — nicht nur walk() (non-vacuous, wie H2)."""
    bindung = T.lade_bindung()
    synth = X.ernte_est_mapping_kz(bindung)
    feld_id = "verzweigung:rentner_jahresrente:gesetzliche_rente"
    assert feld_id in synth

    ergebnis = X.pruefe_bindung({feld_id: synth[feld_id]}, schema_pfade={2025: _SCHEMA_2025})

    assert ergebnis["felder"][feld_id]["status"] == X.STATUS_OK
    assert ergebnis["felder"][feld_id]["jahre"][2025]["pfade"] == [
        "E10/R/Leibr_gesetzl/Einz/E1800301"]
    assert ergebnis["exit_code"] == 0


@requires_real_schema
def test_real_verzweigung_veraeusserungsgewinn_alle_drei_anlagen_kz_korrekt():
    """Task #15-Regression: alle drei Betriebsart-Zweige zeigen auf die korrekte Anlage — 'selbstaendig'
    MUSS auf E0804501 (Anlage S) zeigen (nicht E0901201/Anlage L); 'land_forst' MUSS auf E0901201
    (Anlage L) zeigen. Läuft gegen den vollen Report-Treiber (wie H2), nicht nur walk()."""
    bindung = T.lade_bindung()
    synth = X.ernte_est_mapping_kz(bindung)
    erwartet = {
        "verzweigung:rentner_veraeusserungsgewinn:gewerbe": (
            "E0801301", "E10/G/VAe_G_v_FB/Betr_TBetr_MUAnt/VAe_G_FB_Antr/E0801301"),
        "verzweigung:rentner_veraeusserungsgewinn:selbstaendig": ("E0804501", "E10/S/VAe_Gew/Vor_FB/E0804501"),
        "verzweigung:rentner_veraeusserungsgewinn:land_forst": (
            "E0901201", "E10/L/VAe_G_v_FB/VAe_G_FB_Antr/E0901201"),
    }
    for feld_id, (kz, pfad) in erwartet.items():
        assert feld_id in synth
        assert synth[feld_id]["elster_kz"] == kz

        ergebnis = X.pruefe_bindung({feld_id: synth[feld_id]}, schema_pfade={2025: _SCHEMA_2025})

        assert ergebnis["felder"][feld_id]["status"] == X.STATUS_OK
        assert ergebnis["felder"][feld_id]["jahre"][2025]["pfade"] == [pfad]
        assert ergebnis["exit_code"] == 0


@requires_real_schema
def test_real_bindung_rentner_gegen_echtes_schema_2025():
    """Die gefreezten rentner-Kz aus bindung_rentner.yaml müssen alle eindeutig OK sein (VZ2025)."""
    bindung = T.lade_bindung()
    rentner_felder = {fid: b for fid, b in bindung.items() if fid.startswith("rentner_") and b.get("elster_kz")}
    assert rentner_felder, "bindung_rentner.yaml lieferte keine Kz-Felder (Fixture-Bruch?)"

    ergebnis = X.pruefe_bindung(rentner_felder, schema_pfade={2025: _SCHEMA_2025}, start_element="E10")

    nicht_ok = {fid: f["jahre"][2025] for fid, f in ergebnis["felder"].items()
                if f["jahre"][2025]["status"] != X.STATUS_OK}
    assert nicht_ok == {}
