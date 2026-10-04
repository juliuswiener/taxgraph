#!/usr/bin/env python3
"""Read-only Parity-Dump: die Kz-, Feld- und Regel-Tabellen von `est_mapping.py`, `elster_xml.py`,
`xsd_verify.py` und `checkest_gate.py` als JSON, damit Rust ohne `PARITY=1` und ohne Python-Orakel
dagegen geprueft werden kann (`rust/fixtures/kz_tabellen.json`; Rust-Test `tabellen::tests`,
Python-Test `tests/test_kz_tabellen_fixture.py`; Regal: `rust/elster/src/regal.rs`).

Wertquelle ist ausschliesslich Python: Tabellen als Objekt, einzelne Regeln als Zeilenmuster
(`_wert`). Kein Wert wird aus `rust/` gelesen — die Rust-Seite wird an denselben Werten gemessen.
Mengen sortiert. Run: python3 tools/parity/dump_kz_tabellen.py

Herkunft (Fassung): ERiC 44.2.4.0 — so steht es in `pflichtfelder[].eric_version`, aus
`est_mapping.py:454-462` (checkESt-Laeufe). Die amtliche XSD (`elster11_E10_2025_extern.xsd`, unter
`~/02_Software/eric/`) hat diese Fixture NICHT gelesen: sie ist ein Abbild der Python-Tabellen, kein
Abbild des Schemas. Ein Kz, das in Python, Rust und Bindung gleich falsch steht, faengt sie nicht.
"""
from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
ZIEL = ROOT / "rust" / "fixtures" / "kz_tabellen.json"
for _ordner in ("produkt/mapping", "produkt/eingang", "elster"):
    sys.path.insert(0, str(ROOT / _ordner))

import checkest_gate as CG  # noqa: E402
import elster_xml as EX  # noqa: E402
import est_mapping as M  # noqa: E402
import xsd_verify as XV  # noqa: E402

EST = ROOT / "produkt" / "mapping" / "est_mapping.py"
VALIDATE_XSD = ROOT / "elster" / "submission" / "validate_xsd.py"

# Reihenfolge wie in der Bindung (`produkt/bindung/bindung_p35c_sanierung.yaml:243`): Art -> Kz.
P35C_REIHENFOLGE = [
    "waende", "dach", "geschossdecken", "fenster_tueren", "sommerlicher_waermeschutz",
    "lueftung", "heizung", "digital", "heizung_optimierung",
]

PROBEN: dict[str, list[str]] = {
    "IBAN_PATTERN": ["DE89370400440532013000", "GB82WEST12345698765432", "DE8937", "XX00",
                     "de89370400440532013000", "12345"],
    "STNR_PATTERN": ["9181012345678", "1234012345678", "91811234567", "91810123456789",
                     "918101234567a"],
    "INSTANZ_RE": ["vv_einnahmen__2", "vv_einnahmen__10", "vv_einnahmen__1", "vv_einnahmen__02",
                   "vv_einnahmen", "a__2"],
    "KZ_RE": ["E0100001", "E0241302", "E123456", "e0100001", "X0100001", "E01000011"],
    "JA_TYP_RE": ["Ja1BaseCType", "JaXBaseCType", "JaNein12BaseCType", "Ja2BaseCType_RABE",
                  "JaBaseCType", "GanzzahlPosCType_RABE"],
}


def _verzweigung(tabelle: dict) -> dict:
    return {feld: {"art_feld": v["art_feld"], "kz": dict(v["kz"])} for feld, v in tabelle.items()}


def _wert(pfad: pathlib.Path, muster: str, n: int = 1) -> list[str]:
    """Die `n` Fangruppen aus der einen Zeile in `pfad`, die `muster` enthaelt. Genau ein
    Zeilentreffer ist Pflicht: zwei Treffer waere eine geaenderte Quelle, nicht zwei Regeln."""
    found = [m for m in (re.search(muster, z) for z in pfad.read_text(encoding="utf-8").splitlines())
             if m]
    if len(found) != 1:
        raise SystemExit(f"{pfad.name}: Muster {muster!r} {len(found)}x (genau 1 erwartet)")
    return list(found[0].groups()[:n])


def _proben(name: str, regex: re.Pattern) -> list[dict]:
    """Die Proben sind die Messgroesse: Rust muss dieselben Werte annehmen und dieselben
    verwerfen. Der Mustertext steht nicht in der Fixture — Rust und Python benutzen verschiedene
    Engines, ein Textvergleich waere eine false dependency."""
    return [{"wert": w, "erwartet": bool(regex.match(w))} for w in PROBEN[name]]


def tabellen() -> dict:
    """Die Tabellen in der Form, die der Rust-Test aus Konstanten und Regal-Zonen nachbaut."""
    # ponytail: nicht aufgenommen: Konfessions-Namen (Python schluesselt nach „roemisch-katholisch",
    # Rust nach „römisch-katholisch"), Zeichensatz-Bereiche (`zeichensatz.py` / `zeichensatz.rs`),
    # `KAP_*`-Untergruppen, Hinweistexte der WERTEKODIERUNG als eigene Regel. Upgrade: Schluessel
    # ergaenzen und in `tabellen::tests::aus_tabellen` denselben.
    return {
        "konstante_kz": sorted(M.KONSTANTE_KZ),
        "iban_transform_ziel_kz": sorted(M.IBAN_TRANSFORM_ZIEL_KZ),
        "negation": dict(M.NEGATION),
        "dokumentiert_aggregat": {kz: sorted(felder) for kz, felder in M.DOKUMENTIERT_AGGREGAT.items()},
        "p23": {"betragsfelder": sorted(M.P23_BETRAGSFELDER), "art_feld": M.P23_GEWINN["art_feld"],
                "gewinn_kz": dict(M.P23_GEWINN["kz"])},
        "verzweigung": _verzweigung(M.VERZWEIGUNG),
        "partner_verzweigung": _verzweigung(M.PARTNER_VERZWEIGUNG),
        "partner_instanz": dict(M.PARTNER_INSTANZ),
        "pflege_kz": sorted(M.PFLEGE_KZ),
        "wertekodierung": {feld: {"kz": v["kz"], "code": dict(v["code"])}
                           for feld, v in M.WERTEKODIERUNG.items()},
        # ---- ab hier: die Ueberlebenden der Inventur (Vault `kz-tabellen-rest`) ----
        "abzugs_kz": sorted(M._ABZUGS_KZ),
        "komma_ohne_e60_kz": sorted(M._KOMMA_OHNE_E60_KZ),
        "datums_kz": sorted(M._DATUMS_KZ),
        "null_unzulaessig": {"je_vz": {str(vz): sorted(kz) for vz, kz in M._NULL_UNZULAESSIG_KZ.items()},
                             "vereinigung": sorted(M._NULL_UNZULAESSIG_KZ_VEREINIGUNG)},
        "p35a_summe_aus_posten": [[s, p] for s, p in M._P35A_SUMME_AUS_POSTEN],
        "iban_weiche": dict(zip(("inland", "praefix", "ausland"), _wert(
            EST, r'deklaration\["(E\d{7})" if iban_norm\[:2\] == "([A-Z]{2})" else "(E\d{7})"\]', 3))),
        "bankverbindung": {"iban": _wert(EST, r'_bv_iban_kz = "(E\d{7})" in deklaration or "(E\d{7})"', 2),
                           "keine_bankverbindung": _wert(EST, r'_bv_keine = deklaration\.get\("(E\d{7})"')[0],
                           "kontoinhaber": _wert(EST, r'deklaration\["(E\d{7})"\] = True')[0]},
        "p35c_massnahme_art_reihenfolge": P35C_REIHENFOLGE,
        "multiplikation": list(M.MULTIPLIKATION),
        "kap_felder_a": list(M.KAP_FELDER_A),
        "kap_felder_b": list(M.KAP_FELDER_B),
        "kap_null_grund": M._KAP_NULL_GRUND,
        "pflichtfelder": [{"bedingung": g["bedingung"], "eric_version": g["eric_version"],
                           "felder": sorted(g["felder"])} for g in M.PFLICHTFELDER],
        "hinweise": {feld: v["hinweis_unbekannt"] for feld, v in M.WERTEKODIERUNG.items()},
        "wertekodierung_andere_ohne_code": {"felder": sorted(M.WERTEKODIERUNG), "ohne_code": ["andere"]},
        "pflege": {"grad_kz": _wert(EST, r'^_PFLEGE_GRAD_KZ = "(E\d{7})"')[0],
                   "h_kz": _wert(EST, r'^_PFLEGE_H_KZ = "(E\d{7})"')[0],
                   "block": list(M.PFLEGE_KZ)},
        "proben": {"regex/iban": _proben("IBAN_PATTERN", M._IBAN_PATTERN),
                   "regex/instanz": _proben("INSTANZ_RE", M._INSTANZ_RE),
                   "regex/steuernummer": _proben("STNR_PATTERN", EX._STNR_PATTERN),
                   "xsd_verify/kz_pattern": _proben("KZ_RE", XV._KZ_RE),
                   "xsd_verify/ja_typ_pattern": _proben("JA_TYP_RE", XV._JA_TYP_RE)},
        "elster_xml": {"ns_elster": EX.NS_ELSTER, "ns_e10_format": EX.NS_E10,
                       "testmerker_eric": EX.TESTMERKER_ERIC,
                       "pflicht_default": dict(EX.PFLICHT_DEFAULT),
                       "instanz_nummer_felder": sorted(EX.INSTANZ_NUMMER_FELDER),
                       "e10_ausschluss_datenart": sorted(EX.E10_AUSSCHLUSS_DATENART),
                       "instanz_container_tiefer": dict(EX.INSTANZ_CONTAINER_TIEFER),
                       "absender_strasse_zusatz_kz": EX._ABSENDER_STRASSE_ZUSATZ_KZ,
                       "absender_herkunft": {feld: [[kz, t] for kz, t in paare]
                                             for feld, paare in EX._ABSENDER_HERKUNFT.items()},
                       # Rust hat dafuer keine Tabelle, sondern eine feste Zone in `xsd::schema_info`.
                       "eric_pflicht_trotz_optional": {
                           "E10/V": sorted(EX.ERIC_PFLICHT_TROTZ_OPTIONAL[("E10", "V")])}},
        "xsd_verify": {"max_depth": XV.MAX_DEPTH, "xs_namespace": XV.XS,
                       "datenart": {"default": list(XV._DATENART_DEFAULT),
                                    "routing": {k: list(v) for k, v in XV._DATENART_ROUTING.items()}}},
        "eric": {"extern_schema_muster": _wert(VALIDATE_XSD, r'f"(elster11_E10_\{vz\}_extern\.xsd)"')[0]},
        "eric_rc": {
            "rc": {"RC_OK": CG.RC_OK, "RC_PLAUSIBILITAET": CG.RC_PLAUSIBILITAET,
                   "RC_IO_SCHEMA_VALIDIERUNGSFEHLER": CG.RC_IO_SCHEMA_VALIDIERUNGSFEHLER,
                   "RC_HERSTELLER_GESPERRT": CG.RC_HERSTELLER_GESPERRT,
                   "RC_DATENARTVERSION_UNBEKANNT": CG.RC_DATENARTVERSION_UNBEKANNT,
                   "RC_IO_UNERWARTETE_ELEMENTE": CG.RC_IO_UNERWARTETE_ELEMENTE},
            "klassen": {CG.klassifiziere_rc(rc): rc for rc in (
                CG.RC_OK, CG.RC_PLAUSIBILITAET, CG.RC_IO_SCHEMA_VALIDIERUNGSFEHLER,
                CG.RC_HERSTELLER_GESPERRT, CG.RC_DATENARTVERSION_UNBEKANNT,
                CG.RC_IO_UNERWARTETE_ELEMENTE)},
            "sonstig": {"rc": 7, "klasse": CG.klassifiziere_rc(7)},
            "validiere": CG.ERIC_VALIDIERE,
            "meldungen_max": CG.VALIDIERE_MELDUNGEN_MAX,
            "nicht_geprueft_klassen": sorted(CG.NICHT_GEPRUEFT_KLASSEN)},
    }


def text() -> str:
    return json.dumps(tabellen(), ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def main() -> None:
    ZIEL.parent.mkdir(parents=True, exist_ok=True)
    ZIEL.write_text(text(), encoding="utf-8")
    print(f"{len(tabellen())} Tabellen -> {ZIEL}")


if __name__ == "__main__":
    main()
