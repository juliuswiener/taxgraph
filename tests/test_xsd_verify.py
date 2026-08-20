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
import est_mapping as EM  # noqa: E402

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
    # einkuenfte_gewinn hat Kz — der EIGENE Betrieb (Container-Korrektur 2026-08-20):
    # gewerbe=E0800302 (G/Gew/Einz_U/Betr_1_2), selbstaendig=E0803202 (S/Gewinn/Freiber_T).
    # NICHT E0800502/E0803402 (Ges_Fest/Sum) — das ist der gesondert festgestellte
    # Beteiligungsanteil, dessen Einz-Block Finanzamt und Steuernummer der Gesellschaft fuehrt.
    assert synth.get("verzweigung:einkuenfte_gewinn:gewerbe", {}).get("elster_kz") == "E0800302"
    assert synth.get("verzweigung:einkuenfte_gewinn:selbstaendig", {}).get("elster_kz") == "E0803202"
    # Die Bezeichnung steht im selben Block wie der Betrag (checkESt verlangt sie gemeinsam).
    assert synth.get("verzweigung:gewinn_bezeichnung:gewerbe", {}).get("elster_kz") == "E0800301"
    assert synth.get("verzweigung:gewinn_bezeichnung:selbstaendig", {}).get("elster_kz") == "E0803101"
    # land_forst BEWUSST ohne Kz: Anlage L trennt § 4 Abs. 1/3 (E0901007) und
    # § 13a (E0901103) — unser gewinn_betriebsart unterscheidet das nicht.
    # fail-closed bis zweites Art-Feld existiert.
    assert "verzweigung:einkuenfte_gewinn:land_forst" not in synth


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
def test_real_cluster_a_35a_agb_domaenen_swap_korrekt():
    """XSD-Kz-Section-Sweep Cluster A (reports/review/2026-07-20-xsd-kz-section-sweep-findings.md):
    §35a-Felder MÜSSEN in HA_35a/St_Erm/... landen (nicht AgB/And_Aufw/...), agb_aufwendungen MUSS
    im §33-AgB-Sonst-Bucket landen (nicht HA_35a)."""
    bindung = T.lade_bindung()
    erwartet = {
        "hh_minijob_aufwendungen": ("E0104109", "E10/HA_35a/St_Erm/Minijobs/Sum/E0104109"),
        "hh_dienstleistungen": ("E0107208", "E10/HA_35a/St_Erm/Hhn_BV_DL/Sum/E0107208"),
        "hh_handwerker_arbeitskosten": ("E0111215", "E10/HA_35a/St_Erm/Handw_L/Sum/E0111215"),
        "agb_aufwendungen": ("E0161804", "E10/AgB/And_Aufw/Sonst/Sum/E0161804"),
    }
    for feld_id, (kz, pfad) in erwartet.items():
        assert bindung[feld_id]["elster_kz"] == kz

        ergebnis = X.pruefe_bindung({feld_id: bindung[feld_id]}, schema_pfade={2025: _SCHEMA_2025},
                                     start_element="E10")

        assert ergebnis["felder"][feld_id]["status"] == X.STATUS_OK
        assert ergebnis["felder"][feld_id]["jahre"][2025]["pfade"] == [pfad]
        assert ergebnis["exit_code"] == 0


@requires_real_schema
def test_real_cluster_b_kap_kapitalertraege_kap_elternzeile():
    """XSD-Kz-Section-Sweep Cluster B: kap_kapitalertraege (§20 Abs.9) MUSS in der Anlage-KAP-Elternzeile
    (KAP/KapErt_inl_StAbz/...) landen, nicht in der Unterhaltsleistungs-Zusatzsektion des Hauptvordrucks
    (ESt1A_U/.../KapV/E0121709). Person A UND Person-B-PARTNER_INSTANZ-Reuse (est_mapping.py) müssen
    denselben korrigierten Kz tragen."""
    bindung = T.lade_bindung()
    feld_id = "kap_kapitalertraege"
    assert bindung[feld_id]["elster_kz"] == "E1900701"

    ergebnis = X.pruefe_bindung({feld_id: bindung[feld_id]}, schema_pfade={2025: _SCHEMA_2025},
                                 start_element="E10")

    assert ergebnis["felder"][feld_id]["status"] == X.STATUS_OK
    assert ergebnis["felder"][feld_id]["jahre"][2025]["pfade"] == [
        "E10/KAP/KapErt_inl_StAbz/Betr_lt_StBesch/E1900701"]
    assert ergebnis["exit_code"] == 0
    assert EM.PARTNER_INSTANZ["kap_kapitalertraege_partner"] == "E1900701"
    assert bindung["kap_kapitalertraege_partner"]["elster_kz"] is None  # Klasse g: kein eigenes Kz


@requires_real_schema
def test_real_cluster_c_partner_behinderung_instanz_reuse():
    """XSD-Kz-Section-Sweep Cluster C: rentner_grad_der_behinderung_partner/
    rentner_hilflos_blind_taubblind_partner tragen KEIN eigenes Kz mehr (E0505809/E0505807 wären der
    §33b Abs.5-Kind-Übertragungsmechanismus, strukturell fremd) — stattdessen Klasse-g-Instanz-Reuse
    von Person As E0109708/E0109706 (AgB/Beh-Block, walk-verifiziert), via est_mapping.PARTNER_INSTANZ."""
    bindung = T.lade_bindung()
    assert bindung["rentner_grad_der_behinderung_partner"]["elster_kz"] is None
    assert bindung["rentner_hilflos_blind_taubblind_partner"]["elster_kz"] is None
    assert EM.PARTNER_INSTANZ["rentner_grad_der_behinderung_partner"] == "E0109708"
    assert EM.PARTNER_INSTANZ["rentner_hilflos_blind_taubblind_partner"] == "E0109706"

    erwartet = {
        "rentner_grad_der_behinderung": ("E0109708", "E10/AgB/Beh/Ausw_Rentb_Besch/E0109708"),
        "rentner_hilflos_blind_taubblind": ("E0109706", "E10/AgB/Beh/Geh_Steh_Blind_Hilfl/E0109706"),
    }
    for feld_id, (kz, pfad) in erwartet.items():
        assert bindung[feld_id]["elster_kz"] == kz

        ergebnis = X.pruefe_bindung({feld_id: bindung[feld_id]}, schema_pfade={2025: _SCHEMA_2025},
                                     start_element="E10")

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


@requires_real_schema
def test_voll_katalog_ist_gate_gruen():
    """CI-Gate (H1): der komplette Produktions-Katalog — lade_bindung() + ernte_est_mapping_kz-Harvest —
    muss gegen das lokale ERiC-Schema restlos OK sein (main() exit_code 0). Fängt jede künftige
    Kz-Section-Fehlbindung automatisch, ohne dass jemand xsd_verify manuell fahren muss (VZ2026 ohne
    lokales Schema bleibt SCHEMA_UNVERFUEGBAR und rollt für Felder mit gültigem 2024/2025 zu OK)."""
    assert X.main([]) == 0
