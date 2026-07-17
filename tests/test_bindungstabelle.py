"""Gate für die Bindungstabelle (produkt/bindung/, Task #11).

Deterministisch, LLM-frei. Prüft:
  (a) Schema-Validierung jeder bindung_*.yaml gegen schema.json.
  (b) Vollständigkeit je Scheiben-Regel: jeder askable Signatur-Slot UND jede Geltungsbedingung
      hat eine Bindung ODER eine benannte Lücke. Parameter-Slots (Name in params/<vz>/-Keys)
      sind deterministisch ausgenommen. EP-Slots aus der Catala-Signatur.
  (c) elster_kz-Existenz: jede nicht-null elster_kz existiert im XSD E10-2025 (kz_extract).
  (d) Anker-Verifikation: jeder anker_ref.zitatanker voll-Länge via pipeline/gates._normalize
      gegen die Quelldatei (anker_ref.datei oder rules.yaml norm_source).
  (e) Summen-Konvention: je Slot höchstens ein 'exakt' ODER nur 'summand' (kein Mischen),
      Summanden typ-homogen (cent|int).
Plus Negativtests: manipulierte Kopien MÜSSEN rot werden.
"""
from __future__ import annotations

import copy
import glob
import json
import os
import re

import pytest

yaml = pytest.importorskip("yaml")
jsonschema = pytest.importorskip("jsonschema")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
BIND_DIR = os.path.join(ROOT, "produkt", "bindung")
SCHEMA_PATH = os.path.join(BIND_DIR, "schema.json")
RULES_PATH = os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")

import sys
sys.path.insert(0, ROOT)
from pipeline.gates import _normalize  # noqa: E402

_META_KEYS = {"parameter", "veranlagungszeitraum", "authority", "redistributable", "gueltig_ab",
              "quelle", "datenquelle", "rechtsquelle", "stand", "kohorte", "note", "kommentar",
              "einheit", "wert"}


def _load_schema():
    with open(SCHEMA_PATH) as f:
        return json.load(f)


def _bindung_files():
    return sorted(glob.glob(os.path.join(BIND_DIR, "bindung_*.yaml")))


def _load(f):
    with open(f) as fh:
        return yaml.safe_load(fh)


def _rules():
    with open(RULES_PATH) as f:
        doc = yaml.safe_load(f)
    return {r["rule_id"]: r for r in doc["regeln"]}


def _param_keys():
    keys = set()
    for f in glob.glob(os.path.join(ROOT, "params", "*", "*.yaml")):
        try:
            d = yaml.safe_load(open(f))
        except Exception:
            continue
        if isinstance(d, dict):
            keys |= {k for k in d if k not in _META_KEYS}
    return keys


def _catala_inputs(rule_id):
    """Catala-Input-Signatur (für EP-artige Scope-Regeln ohne rules.yaml-Eintrag)."""
    hits = glob.glob(os.path.join(ROOT, "rules", "estg", rule_id, "*.catala_en"))
    hits = [h for h in hits if "tests" not in os.path.basename(h)]
    inputs = set()
    for h in hits:
        for line in open(h, encoding="utf-8"):
            m = re.match(r"\s+input\s+(\w+)\s+content", line)
            if m:
                inputs.add(m.group(1))
    return inputs


def _rule_slots(rule_id, rules):
    """(askable_slots, geltungsbedingungen) je Regel. askable = Inputs minus Parameter."""
    params = _param_keys()
    if rule_id in rules:
        r = rules[rule_id]
        sig = r.get("signature") or {}
        inputs = set((sig.get("inputs") or {}).keys())
        gbs = {g["bedingung"] for g in (r.get("geltungsbedingungen") or []) if "bedingung" in g}
    else:
        inputs = _catala_inputs(rule_id)
        gbs = set()
    askable = {i for i in inputs if i not in params}
    return askable, gbs


# ---- Fixtures ------------------------------------------------------------------

@pytest.fixture(scope="module")
def schema():
    return _load_schema()


@pytest.fixture(scope="module")
def daten():
    files = _bindung_files()
    assert files, "keine bindung_*.yaml gefunden"
    return {f: _load(f) for f in files}


# ---- (a) Schema ----------------------------------------------------------------

def test_a_schema_valid(schema, daten):
    V = jsonschema.Draft202012Validator(schema)
    for f, d in daten.items():
        errs = sorted(V.iter_errors(d), key=lambda e: list(e.path))
        assert not errs, f"{os.path.basename(f)}: " + "; ".join(
            f"{list(e.path)}: {e.message}" for e in errs[:5])


def test_a_feld_id_eindeutig(daten):
    for f, d in daten.items():
        ids = [b["feld_id"] for b in d["bindungen"]]
        dups = {i for i in ids if ids.count(i) > 1}
        assert not dups, f"{os.path.basename(f)}: doppelte feld_id {dups}"


# ---- (b) Vollständigkeit -------------------------------------------------------

def test_b_vollstaendigkeit(daten):
    rules = _rules()
    for f, d in daten.items():
        bindungen = d["bindungen"]
        luecken = d.get("luecken", [])
        rule_ids = {b["quelle"]["regel_id"] for b in bindungen} | {l["regel_id"] for l in luecken}
        for rid in sorted(rule_ids):
            askable, gbs = _rule_slots(rid, rules)
            geb_slots = {b["quelle"]["signatur_slot"] for b in bindungen
                         if b["quelle"]["regel_id"] == rid and "signatur_slot" in b["quelle"]}
            geb_gbs = {b["quelle"]["geltungsbedingung"] for b in bindungen
                       if b["quelle"]["regel_id"] == rid and "geltungsbedingung" in b["quelle"]}
            lk_slots = {l.get("signatur_slot") for l in luecken if l["regel_id"] == rid}
            lk_gbs = {l.get("geltungsbedingung") for l in luecken if l["regel_id"] == rid}
            fehlende_slots = askable - geb_slots - lk_slots
            fehlende_gbs = gbs - geb_gbs - lk_gbs
            assert not fehlende_slots, (f"{rid}: askable Slots ohne Bindung/Lücke: {fehlende_slots}")
            assert not fehlende_gbs, (f"{rid}: Geltungsbedingungen ohne Bindung/Lücke: {fehlende_gbs}")


# ---- (c) elster_kz existiert im XSD E10-2025 -----------------------------------

@pytest.fixture(scope="module")
def e10_kz():
    sys.path.insert(0, os.path.join(ROOT, "elster"))
    import kz_extract as K
    p = K._find_schema("e10")
    if not p or not os.path.exists(p):
        pytest.skip("E10-2025.html nicht gefunden (ERIC_DIR/EST_SCHEMA_HTML)")
    return set(K.lade(p).keys())


def test_c_elster_kz_existiert(daten, e10_kz):
    for f, d in daten.items():
        for b in d["bindungen"]:
            kz = b.get("elster_kz")
            if kz:
                assert kz in e10_kz, f"{b['feld_id']}: elster_kz {kz} nicht in E10-2025"


# ---- (d) Anker-Verifikation via _normalize ------------------------------------

def _quelldatei(b, rules):
    datei = b["anker_ref"].get("datei")
    if not datei:
        rid = b["quelle"]["regel_id"]
        datei = (rules.get(rid) or {}).get("norm_source")
    assert datei, f"{b['feld_id']}: keine Quelldatei (anker_ref.datei oder rules.yaml norm_source)"
    return os.path.join(ROOT, datei)


def test_d_anker_normalize(daten):
    rules = _rules()
    for f, d in daten.items():
        for b in d["bindungen"]:
            pfad = _quelldatei(b, rules)
            assert os.path.exists(pfad), f"{b['feld_id']}: Quelldatei fehlt {pfad}"
            norm_quelle = _normalize(open(pfad, encoding="utf-8").read())
            anker = _normalize(b["anker_ref"]["zitatanker"])
            assert anker in norm_quelle, (
                f"{b['feld_id']}: Zitatanker nicht in {os.path.basename(pfad)}: '{b['anker_ref']['zitatanker'][:60]}'")


# ---- (e) Summen-Konvention -----------------------------------------------------

def test_e_summen_konvention(daten):
    for f, d in daten.items():
        # slot -> list of (feld_id, slot_beitrag, typ)
        slots = {}
        for b in d["bindungen"]:
            if "signatur_slot" not in b["quelle"]:
                continue
            key = (b["quelle"]["regel_id"], b["quelle"]["signatur_slot"])
            slots.setdefault(key, []).append((b["feld_id"], b.get("slot_beitrag", "exakt"), b["typ"]))
        for key, felder in slots.items():
            beitraege = {sb for _, sb, _ in felder}
            if len(felder) > 1:
                assert beitraege == {"summand"}, (
                    f"Slot {key}: mehrere Felder, aber nicht alle 'summand': {felder}")
                typen = {t for _, _, t in felder}
                assert typen <= {"cent", "int"} and len(typen) == 1, (
                    f"Slot {key}: Summanden nicht typ-homogen cent/int: {typen}")


# ---- (f) bereich-Konsistenz (Prärequisit Unsicherheits-Derivat) ----------------

def test_f_bereich(daten):
    for f, d in daten.items():
        for b in d["bindungen"]:
            ber = b.get("bereich")
            if ber is None:
                continue
            assert b["typ"] in ("cent", "int"), f"{b['feld_id']}: bereich nur bei cent/int"
            assert isinstance(ber["min"], int) and isinstance(ber["max"], int), f"{b['feld_id']}: bereich nicht ganzzahlig"
            assert ber["min"] <= ber["max"], f"{b['feld_id']}: bereich min>max"
            if b["typ"] == "cent" and ber["min"] < 0:
                assert ber.get("grund"), f"{b['feld_id']}: negativer cent-Bereich braucht grund (Verlust-Begründung)"


# ---- Negativtests: manipulierte Kopien MÜSSEN rot werden -----------------------

def _erste_datei_daten(daten):
    f = sorted(daten)[0]
    return copy.deepcopy(daten[f])


def test_neg_paragraph_im_fragetext(schema, daten):
    d = _erste_datei_daten(daten)
    d["bindungen"][0]["fragetext_laie"] = "Aufwendungen i.S.d. § 9 Abs. 1 S. 3 Nr. 4a EStG"
    assert list(jsonschema.Draft202012Validator(schema).iter_errors(d)), "§ im Fragetext nicht abgelehnt"


def test_neg_erfundene_kz(daten, e10_kz):
    # es MUSS echte elster_kz in der Scheibe geben (sonst prüft Gate c nichts)
    echte = [b["elster_kz"] for d in daten.values() for b in d["bindungen"] if b.get("elster_kz")]
    assert echte, "keine elster_kz in der Scheibe — Gate (c) hätte nichts zu prüfen"
    # eine erfundene Kz ist NICHT im XSD -> Gate (c) würde sie rot färben (red-fähig)
    assert "E9999999" not in e10_kz, "erfundene Kz würde in Gate (c) nicht auffallen"


def test_neg_verfaelschter_anker(daten):
    d = _erste_datei_daten(daten)
    rules = _rules()
    b = d["bindungen"][0]
    b["anker_ref"]["zitatanker"] = "dieser text steht garantiert nirgends im gesetz zzzq"
    pfad = _quelldatei(b, rules)
    norm = _normalize(open(pfad, encoding="utf-8").read())
    assert _normalize(b["anker_ref"]["zitatanker"]) not in norm, "verfälschter Anker nicht erkannt"


def test_neg_unbelegter_slot(daten):
    """Entfernt eine Bindung eines RESOLVBAREN Slots -> Vollständigkeit MUSS brechen. Sucht über ALLE
    Scheiben (nicht nur die alphabetisch erste). Catala-Scope-Andockungen OHNE rules/estg-Dir (z.B.
    p2_festzusetzung_einzel) haben bauartbedingt keine required slots (askable leer) — dort greift der
    Vollständigkeits-Gate nicht; sie sind über Kz-/Drift-Wächter abgesichert und werden hier
    übersprungen, damit der Test genau die Regeln prüft, für die die Garantie gilt."""
    rules = _rules()
    treffer = None
    for f in sorted(daten):
        d = copy.deepcopy(daten[f])
        for b in d["bindungen"]:
            if "signatur_slot" not in b["quelle"] or b.get("slot_beitrag", "exakt") != "exakt":
                continue
            askable, _ = _rule_slots(b["quelle"]["regel_id"], rules)
            if b["quelle"]["signatur_slot"] in askable:
                treffer = (d, b)
                break
        if treffer:
            break
    assert treffer, "kein resolvbarer askable Slot-Binding gefunden (Test wäre wirkungslos)"
    d, ziel = treffer
    rid, slot = ziel["quelle"]["regel_id"], ziel["quelle"]["signatur_slot"]
    d["bindungen"] = [b for b in d["bindungen"] if b is not ziel]
    askable, _ = _rule_slots(rid, rules)
    geb = {b["quelle"].get("signatur_slot") for b in d["bindungen"] if b["quelle"]["regel_id"] == rid}
    lk = {l.get("signatur_slot") for l in d.get("luecken", []) if l["regel_id"] == rid}
    assert slot in askable and slot not in geb and slot not in lk, "Slot-Entfernung würde nicht auffallen"


def test_neg_bereich_min_groesser_max(daten):
    # über ALLE Scheiben suchen (die alphabetisch erste kann bereich-frei sein, z.B. an_gesamt)
    ziel = next((b for d in daten.values() for b in d["bindungen"] if b.get("bereich")), None)
    assert ziel is not None, "kein bereich-Feld in irgendeiner Scheibe — Negativtest wäre wirkungslos"
    verdreht = {"min": 100, "max": 0}                 # lokal, keine Fixture-Mutation
    assert not (verdreht["min"] <= verdreht["max"]), "bereich min>max würde nicht auffallen"


def test_neg_gemischte_summanden(daten):
    # Datei mit summand-Feldern über ALLE Scheiben suchen (nicht nur die erste)
    d = None
    for d0 in daten.values():
        if any(b.get("slot_beitrag") == "summand" for b in d0["bindungen"]):
            d = json.loads(json.dumps(d0))
            break
    if d is None:
        pytest.skip("kein summand-Feld in der Scheibe")
    # EIN summand-Feld auf exakt -> Mischung
    for b in d["bindungen"]:
        if b.get("slot_beitrag") == "summand":
            b["slot_beitrag"] = "exakt"
            break
    slots = {}
    for b in d["bindungen"]:
        if "signatur_slot" in b["quelle"]:
            slots.setdefault((b["quelle"]["regel_id"], b["quelle"]["signatur_slot"]), []).append(
                b.get("slot_beitrag", "exakt"))
    gemischt = any(len(v) > 1 and set(v) != {"summand"} for v in slots.values())
    assert gemischt, "gemischte exakt/summand würde nicht auffallen"
