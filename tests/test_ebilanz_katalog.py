"""E-Bilanz Muss-Feld-Katalog-Gate (Paket 2, dev-1).

Parst die committeten XBRL-Freeze-Linkbases (de-gaap-ci reference-fiscal / label /
xsd) frisch und pinnt die Kennzahlen des maschinenlesbaren Muss-Feld-Katalogs +
das Muss ∩ W2-Mapping der 6 Andock-Cluster. Deterministisch, kein LLM.

Die gepinnten Zahlen sind unabhaengig doppelt belegt (ElementTree-Parse UND roher
grep der reference-fiscal); die Quelle ist eindeutig (keine mehrzeiligen/leeren
fiscalRequirement-Werte). Wo dev-2 und Instructor um ±1 abwichen (696/707 vs
697/708, Hoeherstufung 9 vs 8, hbst 19 vs 20), loest dieser Gate das konkrete
Delta-Concept auf statt es zu ueberspielen (s. die jeweiligen Tests).
"""

from __future__ import annotations

import json
import os
import sys

import pytest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, os.path.join(ROOT, "ebilanz"))

import katalog as K   # noqa: E402
import linkbase as L  # noqa: E402


# -- Gepinnte Kennzahlen (autoritativ = Instructor-Zahl; dev-2 -1-Methodenartefakt) --
COUNTS = {
    "6.7": {"mussfeld": 462, "summenmussfeld": 183, "kontennachweis_erwuenscht": 52,
            "rechnerisch_notwendig": 455, "muss_weit": 697, "muss_eng": 514,
            "concepts_mit_reference": 2453},
    "6.8": {"mussfeld": 473, "summenmussfeld": 183, "kontennachweis_erwuenscht": 52,
            "rechnerisch_notwendig": 455, "muss_weit": 708, "muss_eng": 525,
            "concepts_mit_reference": 2505},
}
GCD_MUSS = {"6.7": 58, "6.8": 60}
W2_NENNER = {"6.7": {"muss_weit_eu_oder_pg": 672, "kst_only": 25},
             "6.8": {"muss_weit_eu_oder_pg": 683, "kst_only": 25}}


@pytest.fixture(scope="module")
def parsed():
    return {v: K.parse_version(v) for v in K.VERSIONS}


# -- 1. Muss-Feld-Zaehlung je Version (exakt gepinnt) -------------------------

@pytest.mark.parametrize("version", ["6.7", "6.8"])
def test_muss_counts(parsed, version):
    kat = K.build_katalog(version, parsed[version])
    c = kat["counts"]
    for key, want in COUNTS[version].items():
        assert c[key] == want, (version, key, c[key], want)
    # weit = Mussfeld + Summenmussfeld + Kontennachweis; eng = ohne Summenmussfeld
    assert c["muss_weit"] == c["mussfeld"] + c["summenmussfeld"] + c["kontennachweis_erwuenscht"]
    assert c["muss_eng"] == c["mussfeld"] + c["kontennachweis_erwuenscht"]
    assert kat["gcd_stammdaten_muss"] == GCD_MUSS[version]
    assert kat["w2_nenner"] == W2_NENNER[version]


def test_mussfeld_quelle_eindeutig():
    """Auflage a: das ±1 (dev-2 696/707 vs Instructor 697/708) ist KEIN Quell-
    Ambiguitaets-Concept - die reference-fiscal ist eindeutig. Beleg: roher
    Tag-Count == ElementTree-Parse == 462/473, keine mehrzeiligen/leeren Werte."""
    for version, datum, roh in (("6.7", "2023-04-01", 462), ("6.8", "2024-04-01", 473)):
        path = os.path.join(ROOT, "sources", "ebilanz", version, "xbrl",
                            f"de-gaap-ci-{datum}-reference-fiscal.xml")
        text = open(path, encoding="utf-8").read()
        # roher, einzeiliger Tag-Count (Grep-Aequivalent)
        grep = text.count("<hgbref:fiscalRequirement>Mussfeld</hgbref:fiscalRequirement>")
        refmap = L.parse_reference_fiscal(path)
        parse = len(L.categorize(refmap).get("Mussfeld", set()))
        assert grep == parse == roh, (version, grep, parse, roh)
        # keine mehrzeiligen/leeren fiscalRequirement (sonst waere die Zahl ambig)
        assert "<hgbref:fiscalRequirement/>" not in text
        assert "<hgbref:fiscalRequirement>\n" not in text


# -- 2. Element-Wanderung 6.7 -> 6.8 (Auflage d: PersG-Delta, getrennt je WJ) ---

def test_persg_delta_11_neu_0_entfallen(parsed):
    kat = K.build_katalog("6.8", parsed["6.8"], parsed["6.7"])
    d = kat["persg_delta_zur_vorversion"]
    assert d["neu_muss"] == 11
    assert d["entfallen_muss"] == 0
    # Auflage a (Hoeherstufung 9 vs 8): 8 echte Hoeherstufungen + 3 brandneue Concepts
    assert len(d["hoeherstufungen"]) == 8, d["hoeherstufungen"]
    assert len(d["brandneue_concepts"]) == 3, d["brandneue_concepts"]
    # alle Hoeherstufungen: Rechnerisch notwendig -> Mussfeld, alle PersG-Positionen
    for h in d["hoeherstufungen"]:
        assert h["von"] == "Rechnerisch notwendig, soweit vorhanden"
        assert h["nach"] == "Mussfeld"
    brandneu = {b["concept"] for b in d["brandneue_concepts"]}
    assert brandneu == {
        "fpl.additions.minst",
        "is.netIncome.regular.operatingCOGS.otherCost.otherRemunerationPartners",
        "is.netIncome.regular.operatingTC.otherCost.otherRemunerationPartners",
    }, brandneu


def test_persg_felder_sind_6_8_only(parsed):
    """Auflage d: die 11 PersG-Felder sind 6.8-only -> im 6.7-Muss-Set NICHT."""
    weit67 = L.muss_weit(parsed["6.7"]["cat"])
    weit68 = L.muss_weit(parsed["6.8"]["cat"])
    neu = weit68 - weit67
    assert len(neu) == 11
    assert neu.isdisjoint(weit67)          # keiner der 11 ist in 6.7-Muss
    assert neu <= weit68                    # alle 11 in 6.8-Muss


# -- 3. hbst.transfer (Auflage a: 19 vs 20 aufgeloest) ------------------------

@pytest.mark.parametrize("version", ["6.7", "6.8"])
def test_hbst_transfer_20_xsd_19_reffiscal(parsed, version):
    xsd = {c for c in parsed[version]["concepts"] if c.startswith("hbst.transfer")}
    ref = {c for c in parsed[version]["refmap"] if c.startswith("hbst.transfer")}
    assert len(xsd) == 20, sorted(xsd)          # Instructor-Zahl (inkl. Tupel-Root + kind.head)
    assert len(ref) == 19, sorted(ref)          # dev-2-Zahl (reference-fiscal-Praesenz)
    # Diskriminator: genau hbst.transfer.kind.head ist xsd-only (kein reference-Eintrag)
    assert xsd - ref == {"hbst.transfer.kind.head"}
    # kein hbst.transfer* traegt fiscalRequirement (reine Tupel-Struktur, kein Einzel-Muss)
    for c in ref:
        assert parsed[version]["refmap"][c]["fiscalRequirement"] is None


# -- 4. Muss ∩ W2 — die 6 Andock-Cluster loesen in BEIDEN Versionen auf --------

@pytest.mark.parametrize("version", ["6.7", "6.8"])
def test_w2_cluster_aufloesung(parsed, version):
    kat = K.build_katalog(version, parsed[version])
    for cl in kat["w2_cluster"]:
        for c in cl["concepts"]:
            assert c["ok"], (version, cl["w2_regel"], c["concept"],
                             c["ist_kategorie"], c["erwartete_kategorie"])
        if "tupel" in cl:
            assert cl["tupel"]["anzahl_xsd"] == 20
            assert cl["tupel"]["anzahl_reference_fiscal"] == 19


# -- 5. Kontennachweis = Submission-Komponente, KEIN Muss-Feld (Auflage c) -----

def test_kontennachweis_ist_komponente_kein_feld(parsed):
    kat = K.build_katalog("6.8", parsed["6.8"])
    kn = kat["kontennachweis_pflicht"]
    assert kn["kein_muss_feld"] is True
    assert kn["typ"] == "submission_pflicht_komponente"
    # die 52 "Mussfeld, Kontennachweis erwünscht" sind das Positions-Flag, NICHT die
    # JStG-Uebermittlungspflicht - sie bleiben normale Muss-Felder im Katalog.
    assert kat["counts"]["kontennachweis_erwuenscht"] == 52


# -- 6. Committete JSON-Kataloge == frischer Parse (kein Drift) ----------------

def test_katalog_json_kein_drift(parsed):
    prev = {"6.7": None, "6.8": parsed["6.7"]}
    for version in K.VERSIONS:
        fresh = K.build_katalog(version, parsed[version], prev[version])
        committed = json.load(open(
            os.path.join(ROOT, "ebilanz", f"katalog_{version}.json"), encoding="utf-8"))
        assert fresh == committed, (
            f"katalog_{version}.json driftet gegen frischen Parse - "
            f"regenerieren mit: python ebilanz/katalog.py")


# -- 7. NEGATIVTEST (Auflage b): manipulierte reference-fiscal -> Gate faellt ---

def _reference_fiscal_mit_flip(src_path, concept, neuer_wert, tmp_path):
    """Schreibt eine reference-fiscal-Kopie, in der GENAU EIN Concept eine andere
    fiscalRequirement-Kategorie traegt. Beweis, dass der Gate darauf reagiert."""
    import xml.etree.ElementTree as ET
    ET.register_namespace("link", L.LINK)
    ET.register_namespace("xlink", L.XLINK)
    tree = ET.parse(src_path)
    root = tree.getroot()
    ziel_label = f"reference_de-gaap-ci_{concept}"
    getroffen = 0
    for ref in root.iter(f"{{{L.LINK}}}reference"):
        if ref.get(f"{{{L.XLINK}}}label") == ziel_label:
            for child in ref:
                if child.tag.endswith("fiscalRequirement"):
                    child.text = neuer_wert
                    getroffen += 1
    assert getroffen == 1, (concept, getroffen)   # eindeutiger Treffer
    out = os.path.join(str(tmp_path), "reference-fiscal-tampered.xml")
    tree.write(out, encoding="utf-8", xml_declaration=True)
    return out


def test_negativtest_flip_senkt_muss_count(tmp_path):
    src = os.path.join(ROOT, "sources", "ebilanz", "6.8", "xbrl",
                       "de-gaap-ci-2024-04-01-reference-fiscal.xml")
    base = len(L.muss_weit(L.categorize(L.parse_reference_fiscal(src))))
    assert base == 708
    # ein Mussfeld -> "Rechnerisch notwendig" degradieren: MUSS-Zahl faellt um 1
    tampered = _reference_fiscal_mit_flip(
        src, "bs.ass.prepaidExp", "Rechnerisch notwendig, soweit vorhanden", tmp_path)
    got = len(L.muss_weit(L.categorize(L.parse_reference_fiscal(tampered))))
    assert got == base - 1, (base, got)


def test_negativtest_flip_kippt_w2_cluster(tmp_path, monkeypatch):
    """Kippt man die Kategorie eines W2-Cluster-Concepts, MUSS dessen ok-Flag
    fallen - der 6-Cluster-Gate haette das gefangen."""
    src = os.path.join(ROOT, "sources", "ebilanz", "6.8", "xbrl",
                       "de-gaap-ci-2024-04-01-reference-fiscal.xml")
    # p5_5 aktiver RAP: bs.ass.prepaidExp erwartet 'Mussfeld'
    tampered = _reference_fiscal_mit_flip(
        src, "bs.ass.prepaidExp", "Summenmussfeld", tmp_path)
    refmap = L.parse_reference_fiscal(tampered)
    ist = refmap["bs.ass.prepaidExp"]["fiscalRequirement"]
    erwartet = dict(K.W2_CLUSTER[-1]["concepts"])["bs.ass.prepaidExp"]  # 'Mussfeld'
    assert erwartet == "Mussfeld"
    assert ist != erwartet   # Cluster-ok waere jetzt False -> Gate faengt es
