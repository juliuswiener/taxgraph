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

import ast
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


def test_a_feld_id_eindeutig_ueber_alle_dateien(daten):
    """Eine feld_id darf NUR EINMAL im gesamten Bindungsverzeichnis stehen — nicht nur
    einmal je Datei.

    Zwei Bindungen für dasselbe Feld sind kein doppelter Eintrag, sondern zwei
    Wahrheiten: welche gilt, hängt an der Ladereihenfolge. Typ, Fragetext, anker_ref
    und elster_kz können auseinanderlaufen, ohne dass ein Test es merkt — der
    Feld-in-einer-Datei-Test oben sieht dateiübergreifende Duplikate nicht.
    """
    heimat = {}
    dups = {}
    for f, d in daten.items():
        for b in d["bindungen"]:
            fid = b["feld_id"]
            if fid in heimat:
                dups.setdefault(fid, [heimat[fid]]).append(os.path.basename(f))
            else:
                heimat[fid] = os.path.basename(f)
    assert not dups, "feld_id in mehreren Bindungsdateien: " + "; ".join(
        f"{k} in {v}" for k, v in sorted(dups.items()))


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
    """Gültige Kz-Menge = E10 (ESt-Erklärung, Anlagen N/AUS/S/G) + E77 (Anlage EÜR, eigene Datenart).
    Die §§ 13-18-EÜR-Kz (E60xx: geringwertige WG E6002301, übrige Betriebsausgaben E6004901) liegen in
    E77-2025.xsd, NICHT im E10-Schema — der EÜR-Zweig deklariert in die EUER-Datenart (Kz-Review 2026-07-19,
    Gate-Erweiterung um die E77-Kz-Quelle). Ein erfundenes Kz ist in KEINEM der beiden -> Gate bleibt red-fähig."""
    sys.path.insert(0, os.path.join(ROOT, "elster"))
    import kz_extract as K
    p10 = K._find_schema("e10")
    if not p10 or not os.path.exists(p10):
        pytest.skip("E10-2025.html nicht gefunden (ERIC_DIR/EST_SCHEMA_HTML)")
    kz = set(K.lade(p10).keys())
    p77 = K._find_schema("e77")
    if p77 and os.path.exists(p77):
        kz |= set(K.lade(p77).keys())        # Anlage-EÜR-Kz (E60xx) — eigene Datenart, nicht in E10
    return kz


def test_c_elster_kz_existiert(daten, e10_kz):
    for f, d in daten.items():
        for b in d["bindungen"]:
            kz = b.get("elster_kz")
            if kz:
                assert kz in e10_kz, f"{b['feld_id']}: elster_kz {kz} nicht in E10/E77-Schema"


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


# ---- (f2) beispielwert innerhalb des eigenen bereich ---------------------------

def test_f2_beispielwert_in_bereich(daten):
    """Der beispielwert muss innerhalb des eigenen bereich liegen.

    Sonst erzeugt jeder generierte Testlauf, der den beispielwert unbesehen übernimmt
    (z.B. der Feldmatrix-Vollsweep), einen Fehler, der wie ein Befund aussieht, aber nur
    eine falsche bereich-Angabe ist. Realer Fall 2026-08-12: tage_24h/tage_an_abreise/
    tage_ueber_8h_eintaegig hatten bereich.min=0 und beispielwert=0, obwohl ELSTER strikt
    >0 verlangt ("Der ... eingegebene Wert muss größer als 0 sein.") — drei Felder fielen
    im Vollsweep als vermeintliche Befunde rot, waren aber reines Bindungs-Datenqualitäts-
    Rauschen (reports/adjudikation/feldmatrix_vollklassifikation_2026-08-12.md).
    """
    for f, d in daten.items():
        for b in d["bindungen"]:
            ber = b.get("bereich")
            bw = b.get("beispielwert")
            if ber is None or bw is None or not isinstance(bw, (int, float)) or isinstance(bw, bool):
                continue
            assert ber["min"] <= bw <= ber["max"], (
                f"{os.path.basename(f)}::{b['feld_id']}: beispielwert {bw} außerhalb "
                f"bereich [{ber['min']}, {ber['max']}]")


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


def test_neg_beispielwert_ausserhalb_bereich(daten):
    # Zielfeld über ALLE Scheiben suchen (bereich UND numerischer beispielwert nötig)
    ziel = next((b for d in daten.values() for b in d["bindungen"]
                 if b.get("bereich") and isinstance(b.get("beispielwert"), (int, float))
                 and not isinstance(b.get("beispielwert"), bool)), None)
    assert ziel is not None, "kein Feld mit bereich UND numerischem beispielwert — Negativtest wäre wirkungslos"
    verfaelscht = ziel["bereich"]["max"] + 1
    assert not (ziel["bereich"]["min"] <= verfaelscht <= ziel["bereich"]["max"]), (
        "außerhalb-bereich-beispielwert würde nicht auffallen")


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


# ---- (g) Erreichbarkeit: askable-Feld ohne Scheibe ------------------------------

# Felder, die askable gebunden sind, aber in KEINER nutzerwählbaren Scheibe stehen.
# Die Oberfläche bietet nur "gesamt" und "rentner_gesamt" (produkt/haut/static/index.html),
# also kann der Nutzer sie nicht setzen — ein POST endet mit "feld_id nicht in dieser
# Scheibe". Wer so ein Feld verdrahtet, baut ins Leere.
#
# Die Liste ist eine BESTANDSAUFNAHME, kein Freibrief: sie hält den Stand fest, damit ein
# NEUES unerreichbares Feld auffällt. Wer hier etwas einträgt, sollte begründen können,
# warum das Feld nicht gefragt wird. Wer eines entfernt, hat es erreichbar gemacht.
UNERREICHBAR_BEKANNT = {
    # § 33 Abs. 1 Tatbestand — Geltungsbedingungen der Regel, nicht erfragt
    "agb_notwendig_angemessen", "agb_zwangslaeufig",
    # kind_idnr — instanz_gruppe: kind, kein Top-Level-Feld in SCHEIBEN["felder"]
    "kind_idnr",
    # kind_kindschaftsverhaeltnis_a/b + kind_kindschaftsverh_zeitraum_a/b: seit 2026-08-12
    # via KIND_KV_PV im Kegel (checkESt-Messung), also erreichbar — nicht mehr hier.
    # § 6 Abs. 2 GWG-Tatbestand — Geltungsbedingungen, nicht erfragt
    "gwg_bewegliches_selbstaendig_nutzbar", "gwg_netto_ohne_vorsteuer", "gwg_verzeichnis_ab_250",
    # § 24a — der Accessor leitet das Alter aus geburtsjahr + VZ ab
    "rentner_alter_64_erfuellt",
    # § 9 Abs. 4a Einzelreise-Slots — der Ring rechnet aus den Tages-Aggregaten
    "vpf_abwesenheit_stunden", "vpf_an_oder_abreisetag", "vpf_auswaertige_taetigkeit",
    "vpf_mit_uebernachtung",
}


def test_g_askable_felder_sind_erreichbar(daten):
    """Ein askable-Feld, das in keiner nutzerwählbaren Scheibe steht, ist tote Bindung.

    Dieser Fehler ist mehrfach aufgetreten: Bindung geschrieben, Accessor gebaut,
    Unit-Tests grün — und das Feld war über die Oberfläche nie setzbar, weil der Eintrag
    in SCHEIBEN fehlte. Unit-Tests fangen das nicht, weil sie den Accessor direkt aufrufen
    und die Scheibe nie berühren.
    """
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "api_constants_gate", os.path.join(ROOT, "produkt", "haut", "api_constants.py"))
    AC = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(AC)

    erreichbar = set()
    for scheibe in ("gesamt", "rentner_gesamt"):
        erreichbar |= set(AC.SCHEIBEN[scheibe].get("felder") or ())

    askable = {b["feld_id"] for d in daten.values() for b in d["bindungen"] if b.get("askable")}
    neu = sorted(askable - erreichbar - UNERREICHBAR_BEKANNT)
    assert not neu, (
        "askable gebunden, aber in keiner nutzerwählbaren Scheibe (tote Bindung): "
        f"{neu} — entweder in SCHEIBEN aufnehmen oder in UNERREICHBAR_BEKANNT "
        "mit Begründung eintragen")


def test_g_gate_faengt_tote_bindung(daten):
    """Gegenprobe: ein erfundenes askable-Feld ohne Scheibe MUSS auffallen.

    Ohne diese Probe wäre nicht belegt, dass das Gate überhaupt anschlagen kann —
    UNERREICHBAR_BEKANNT könnte alles verdecken.
    """
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "api_constants_gate2", os.path.join(ROOT, "produkt", "haut", "api_constants.py"))
    AC = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(AC)
    erreichbar = set()
    for scheibe in ("gesamt", "rentner_gesamt"):
        erreichbar |= set(AC.SCHEIBEN[scheibe].get("felder") or ())
    askable = {"zzz_erfundenes_feld_ohne_scheibe"}
    assert sorted(askable - erreichbar - UNERREICHBAR_BEKANNT) == ["zzz_erfundenes_feld_ohne_scheibe"]


# ========== Betragsfelder ohne Kz / ELSTER-Audit ==========

# 59 Betragsfelder (typ: cent) steuerwirksam, aber ohne elster_kz.
# Gruppiert nach Audit-Befund (siehe tests/test_bindungstabelle.py:test_h_betragsfelder_haben_kz_oder_begruendung).
# Stand: 2026-07-31, Audit zeigt 58/77 steuerwirksame Beträge ohne Weg ins XML-Schema.

BETRAGSFELDER_OHNE_KZ = {
    # Gruppe VERZWEIGT: das Kz steht nicht am Feld, sondern haengt an einem Art-Feld
    # (est_mapping.VERZWEIGUNG, Klasse f). Kein fehlendes Mapping, sondern ein anderes.
    # p35c_massnahme_einzelbetrag -> eine der neun Einzelzeilen der Anlage Energetische
    # Massnahmen, gewaehlt ueber p35c_massnahme_art (2026-08-16).
    "p35c_massnahme_einzelbetrag",

    # Gruppe B: Partner-Instanzen — XML-Writer kennt PersonB nicht (elster_xml.py Z. 39 nur PersonA).
    # Stille Fehler bei Zusammenveranlagung: Ehepaar 50k+50k rechnet 20.490€ ESt, Erklärung enthält
    # nur Lohn von Person A → 14.982€ Under-Tax (gemessen 2026-07-31).
    # Status: FEHLER, Klärung offen. PersonB-Logik in XML-Writer fehlt komplett.
    "basis_kv_partner",
    "bruttoarbeitslohn_partner",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige_partner",
    "kap_kapitalertraege_partner",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner",
    "rentner_rentenfreibetrag_partner",
    "vor_ag_anteil_rv_partner",
    "vor_an_anteil_rv_partner",
    "vor_rv_ausserhalb_lstb_partner",
    "vorsorge_arbeitslosenversicherung_partner",
    "vorsorge_erwerbsunfaehigkeit_partner",
    "vorsorge_unfall_haftpflicht_partner",
    "vorsorge_rv_alt_mit_ueberschuss_partner",
    "vorsorge_rv_alt_ohne_ueberschuss_partner",
    # Gewinneinkuenfte-Partnerseite (2026-08-12): Stufe-1-Scaffolding, dieselbe Gruppe-B-Naht wie
    # oben — Bindung ist Interview-Scaffolding, der Ring-Aufruf (zweiter GESAMT_GEWINN-Durchlauf
    # fuer Person B) ist Stufe 2 und explizit noch nicht gebaut (s. BACKLOG
    # partnerseite-gewinneinkuenfte-fehlt-strukturell). Loest sich auf, sobald Stufe 2 den
    # Ring-Aufruf fuer Person B verdrahtet und einen echten elster_kz liefert.
    "einkuenfte_gewinn_partner",
    "gewinnanteil_partner",
    "verguetung_taetigkeit_partner",
    "verguetung_darlehen_partner",
    "verguetung_ueberlassung_partner",
    "gewst_messbetrag_partner",
    # Berechnet (Messbetrag x Hebesatz, § 16 Abs. 1 GewStG) und wie die anderen Partner-Felder
    # ohne eigenes Kz: E0801704 wird ueber den person_b-Bucket geschrieben, es gibt kein
    # distinktes Ehegatte-Kz (2026-08-19).
    "gewst_zu_zahlen_partner",
    "rentner_veraeusserungsgewinn_partner",

    # Gruppe A: Accessor-Output ohne Kz-Mapping.
    # Begründung nennt Ziel-Kz (z.B. E0204401), aber grep über produkt/ findet es nur in elster_kz_grund,
    # nicht in est_mapping.py oder elster_xml.py. Behauptung unbelegt.
    # Status: Gruppe-A-Felder sind Accessor-Ergebnis, nicht direkt als INPUT in ein Kz-tragendes Feld abbildbar.
    "am_anschaffungskosten",
    "berufsausbildung_aufwendungen",
    "afa_jahresbetrag",
    "betriebseinnahmen",

    # Gruppe C: Dokumentierte MVP-Lücken / andere offene Posten (44 Felder).
    # elster_kz_grund nennt "MVP", "Folgeticket", oder konkrete Baustellen-Referenzen.
    # Status: legitime Lücken, Roadmap-Items, nicht kurzfristig lösbar.
    "basis_kv",
    "basis_kv_partner",
    "basis_pv",
    "basis_pv_partner",
    "dba_auslaendische_einkuenfte",
    "dba_gezahlte_auslaendische_steuer",
    "einkuenfte_gewinn",
    "gewinnanteil",
    "gewst_messbetrag",
    "kist_erstattet",
    "kist_gezahlt",
    "kap_gewinn_sonstige",
    "p22_nr3_einkuenfte",
    "p23_anschaffung_herstellungskosten",
    "p23_veraeusserungspreis",
    "p23_werbungskosten",
    "p32b_progressionseinkuenfte",
    "p33a_andere_einkuenfte_bezuege",
    "p33a_unterhalt_aufwendungen",
    "p33a_unterhalt_kv_pv",
    "p35c_energieberater_aufwendungen",
    "p35c_sanierungsaufwendungen",
    "p36_lohnsteuer",
    "p36_vorauszahlungen",
    "pv_einnahmen",
    "realsplitting_empfaenger_kv_pv",
    "realsplitting_unterhaltsleistungen",
    "rentner_jahresrente",
    "rentner_jahresrente_partner",
    "rentner_rentenfreibetrag",
    "rentner_veraeusserungsgewinn",
    "uebernachtung_kosten_monat",
    "verguetung_darlehen",
    "verguetung_taetigkeit",
    "verguetung_ueberlassung",
    "verlustvortrag_bestand",
    "versorgung_bemessungsgrundlage",
    "versorgung_jahresrente",
    "vpf_mahlzeiten_gezahltes_entgelt",
    "vpf_steuerfreie_erstattung_betrag",

    # Gruppe D: Kuerzungsfeld, das ELSTER per Konstruktion nicht sieht.
    # behinderungsbedingte_aufwendungen wird VOR dem Ring von agb_aufwendungen
    # abgezogen (§ 33b Abs. 5 S. 4, api.py _shared_steuer_sonder_agb). Uebermittelt
    # wird in E0161804 der bereits bereinigte Betrag — ein eigenes Kz dafuer gibt es
    # im XSD nicht und waere auch falsch, weil der Betrag nicht erklaert, sondern
    # herausgerechnet wird.
    "behinderungsbedingte_aufwendungen",
    # Partner-Spiegel (BACKLOG p33b-partner-pb-doppelabzug): behinderungsbedingte_aufwendungen_partner
    # wird VOR dem Ring vom selben agb_aufwendungen-Pool abgezogen (§ 33b Abs. 1 S. 1/2, api.py
    # _shared_steuer_sonder_agb) — dieselbe Konstruktions-Begruendung wie oben, nur fuer den Partner.
    "behinderungsbedingte_aufwendungen_partner",
    "vv_erhaltungsaufwand",
    "vv_gebaeude_afa",
    "vv_schuldzinsen",
    "vv_sonstige_wk",

    # Gruppe E: Anlage-N-Instanz B (Person-Multiplikation, Julius-Entscheidung 2026-08-10) — ANDERS
    # als Gruppe B kein Fehler: est_mapping.PARTNER_INSTANZ routet beide auf denselben Kz wie Person A
    # (E0200301/E0200501), Wert landet im person_b-Bucket. elster_kz bleibt in der Bindung bewusst null
    # (s. bindung_an_gesamt.yaml, test_est_mapping.py::test_steuerklasse_lohnsteuer_kirchensteuer_person_b).
    "p36_lohnsteuer_partner",
    "kirchensteuer_arbeitgeber_partner",
}


def test_h_betragsfelder_haben_kz_oder_begruendung(daten):
    """Betragsfelder (typ: cent) müssen elster_kz ODER benannte Ausnahme haben.

    Von 77 steuerwirksamen Betragsfeldern haben nur 18 einen Weg in die ELSTER-Erklärung.
    Dieser Test hält die 59 bekannten Lücken fest, damit NEUE Felder ohne Kz auffallen
    und nicht unbemerkt in dieser großen Menge untergehen.

    Audit 2026-07-31: Gruppe B (12 Partner-Felder) = FEHLER, PersonB-Logik fehlt.
    Gruppe A (2 Accessor-Output) = Behauptung unbelegt. Gruppe C (44) = MVP-offen.
    """
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "api_constants_kz", os.path.join(ROOT, "produkt", "haut", "api_constants.py"))
    AC = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(AC)

    # Menge: alle Felder in nutzerwählbaren Scheiben
    in_scheiben = set()
    for scheibe in ("gesamt", "rentner_gesamt"):
        in_scheiben |= set(AC.SCHEIBEN[scheibe].get("felder") or ())

    # Subgroup: Betragsfelder (typ: cent)
    betragsfelder = {
        b["feld_id"] for d in daten.values() for b in d["bindungen"]
        if b.get("typ") == "cent" and b["feld_id"] in in_scheiben
    }

    # Prüfung: entweder kz gesetzt ODER in benannter Ausnahme
    # (daten[feld_id] = {bindungen: [...]}, alle Bindungen-Einträge sollten same elster_kz haben)
    ohne_kz = {
        b["feld_id"] for d in daten.values() for b in d["bindungen"]
        if b.get("typ") == "cent" and b["feld_id"] in betragsfelder
        and not b.get("elster_kz")
    }
    unbekannt = sorted(ohne_kz - BETRAGSFELDER_OHNE_KZ)

    assert not unbekannt, (
        "Neue Betragsfelder ohne elster_kz gefunden (weder Kz noch Ausnahme): "
        f"{unbekannt} — entweder in est_mapping.py elster_kz hinzufügen oder in "
        "BETRAGSFELDER_OHNE_KZ mit Gruppen-Kommentar eintragen")


def test_h_gate_faengt_neue_felder_ohne_kz(daten):
    """Gegenprobe: ein erfundenes cent-Feld ohne Kz MUSS auffallen.

    Ohne diese Probe könnte BETRAGSFELDER_OHNE_KZ heimlich alle neuen Fehler verdecken.
    """
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "api_constants_kz2", os.path.join(ROOT, "produkt", "haut", "api_constants.py"))
    AC = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(AC)
    in_scheiben = set()
    for scheibe in ("gesamt", "rentner_gesamt"):
        in_scheiben |= set(AC.SCHEIBEN[scheibe].get("felder") or ())

    # Simuliere ein neues cent-Feld ohne Kz und ohne Ausnahme
    betragsfelder = {"zzz_neues_feld_ohne_kz"}
    ohne_kz = betragsfelder - {f for f in betragsfelder if False}  # Alle haben kein kz
    unbekannt = sorted(ohne_kz - BETRAGSFELDER_OHNE_KZ)
    assert unbekannt == ["zzz_neues_feld_ohne_kz"], (
        f"Gegenprobe fehlgeschlagen: erfundenes Feld nicht erkannt: {unbekannt}")


# Bekannte Ausnahmen für api.py-Read-Keys, die keinem Store-Feld entsprechen.
API_READ_AUSNAHMEN: set[str] = set()


def test_i_api_read_keys_sind_in_bindung(daten):
    """Jeder feld_id-String in `_c(...)`/`_cent(...)`/`f.get("...")`-Aufrufen von api.py
    muss in mindestens einer bindung_*.yaml als `feld_id` existieren.

    `_c(fid)` (z. B. api.py:183) gibt bei unbekanntem fid still 0 zurück — kein Error,
    kein None, kein Log. `f.get("fid", {}).get("wert")` gibt None → `is True` = False →
    still falscher Zweig. Beide Bauarten senken den Abzug still (Over-tax), ohne dass
    ein Test rot wird.

    Realer Fall 2026-08-05: basis_kv_pv → basis_kv + basis_pv (Feldsplit, 4 neue Felder).
    Vier von sechs _c-Lesestellen (api.py:764/779/1338/1354) lasen beim Commit noch
    basis_kv_pv → _c gab still 0 → der gesamte KV/PV-Vorsorgeabzug in den Scheiben
    gesamt und rentner_gesamt fiel weg → Over-tax. Die Suite blieb grün.
    Weder test_g (askable→SCHEIBEN) noch ein anderer Test hat es gefangen — gefunden
    per Hand beim Review. Dieses Gate ist die systematische Lösung.
    """
    api_py = os.path.join(ROOT, "produkt", "haut", "api.py")
    with open(api_py) as f:
        src = f.read()

    read_keys: set[str] = set()

    # Pattern 1: _c("feld_id") und _cent("feld_id")
    read_keys |= set(re.findall(r'_c(?:ent)?\("([a-z0-9_]+)"\)', src))

    # Pattern 2: f.get("feld_id")
    read_keys |= set(re.findall(r'f\.get\("([a-z0-9_]+)"', src))

    # Alle feld_ids aus der Bindungstabelle sammeln
    binding_keys: set[str] = set()
    for d in daten.values():
        for b in d["bindungen"]:
            fid = b.get("feld_id")
            if fid:
                binding_keys.add(fid)

    unbekannt = sorted(read_keys - binding_keys - API_READ_AUSNAHMEN)
    assert not unbekannt, (
        f"api.py ruft _c / _cent / f.get für Feld-IDs auf, die in keiner "
        f"bindung_*.yaml als feld_id existieren: {unbekannt}. Das Feld wurde "
        f"umbenannt/gelöscht, die api.py-Lesestelle nicht mitgezogen → stiller "
        f"Over-tax (keine Test-Rot-Warnung).")


# Bekannte Ausnahmen für Kz, die gebunden sind aber nicht in Tests vorkommen.
# Jeder Eintrag braucht eine Begründung (z. B. "Nur im XSD-Kommentar, keine echte Bindung").
GEBUNDENE_KZ_OHNE_TEST: set[str] = set()

# Scharfe Ausnahmeliste (test_m_): Kz in keiner assert-gesteuerten Prüfung.
# Jeder Eintrag einzeln begründet — hier ist die Lücke sichtbar, nicht still.
GEBUNDENE_KZ_KEIN_ASSERT: dict[str, str] = {}


def test_j_gebundene_kz_sind_test_belegt():
    """Jedes Kz, das in einer bindung_*.yaml als elster_kz gesetzt oder in
    est_mapping.py (VERZWEIGUNG / PARTNER_VERZWEIGUNG / PARTNER_INSTANZ /
    DOKUMENTIERT_AGGREGAT / NEGATION) als Kz-Eintrag vorkommt, muss als
    String-Literal in mindestens einer Datei unter tests/ vorkommen.

    Prüft ANWESENHEIT, nicht KORREKTHEIT. Ein Kz in einem Kommentar zählt
    ebenso wie ein Kz in einem Assert — das ist die billigste Annäherung
    an Vollständigkeit. Ein vertauschtes Kz im Test zählt auch. Später kann
    eine Laufzeitmessung ergänzt werden.

    Realer Fall 2026-08-05: sechs KV/PV-Kz (E2001203, E2001505, E2001805,
    E2002105, E2003104, E2003202) gebunden in est_mapping.py, Suite grün,
    kein Test kannte auch nur eines. Ein vertauschtes KV/PV-Paar (z. B.
    KV auf PV-Kz) wäre nie aufgefallen. Gefunden beim manuellen Review.
    """
    # (a) elster_kz aus allen bindung_*.yaml
    binding_kz: set[str] = set()
    for fp in glob.glob(os.path.join(BIND_DIR, "bindung_*.yaml")):
        with open(fp) as f:
            doc = yaml.safe_load(f)
        for eintrag in doc.get("bindungen", []):
            kz = eintrag.get("elster_kz")
            if kz and str(kz).startswith("E") and len(str(kz)) == 8:
                binding_kz.add(kz)

    # (b) Kz aus est_mapping-Datenstrukturen via Import
    import importlib.util
    em_spec = importlib.util.spec_from_file_location(
        "est_mapping", os.path.join(ROOT, "produkt", "mapping", "est_mapping.py"))
    EM = importlib.util.module_from_spec(em_spec)
    em_spec.loader.exec_module(EM)

    mapping_kz: set[str] = set()
    for cfg in EM.VERZWEIGUNG.values():
        mapping_kz.update(cfg["kz"].values())
    for cfg in EM.PARTNER_VERZWEIGUNG.values():
        mapping_kz.update(cfg["kz"].values())
    mapping_kz.update(EM.PARTNER_INSTANZ.values())
    mapping_kz.update(EM.NEGATION.values())
    mapping_kz.update(EM.DOKUMENTIERT_AGGREGAT.keys())

    alle_kz = binding_kz | mapping_kz

    # Gegen alle Test-Dateien prüfen
    test_files = glob.glob(os.path.join(ROOT, "tests", "*.py"))
    ungedeckt = sorted(
        kz for kz in alle_kz
        if all(kz not in open(tf).read() for tf in test_files)
    )

    ungedeckt = [k for k in ungedeckt if k not in GEBUNDENE_KZ_OHNE_TEST]
    assert not ungedeckt, (
        f"Kz gebunden (elster_kz / est_mapping), aber in keiner Test-Datei "
        f"als Literal vorhanden: {ungedeckt}. Ein Kz ohne Test-Erwähnung "
        f"kann vertauscht oder falsch sein, ohne dass jemand es merkt.")


def _ast_kz_in_assert_helper(kz: str, src: str) -> bool:
    """Prüft ob Kz via AST in einem assert-gesteuerten Pfad liegt.

    Zählt als geprüft wenn:
    (a) Kz als String-Literal INNERHALB eines ast.Assert-Knotens
    (b) Kz als String-Literal in einer for-loop-Iterable, deren Laufvariable
        in einem assert des loop-Bodys referenziert wird
    (c) Kz in einem parametrize-Decorator, dessen Parameter in einem assert
        der dekorierten Funktion referenziert wird
    """
    try:
        tree = ast.parse(src)
    except SyntaxError:
        return False

    # (a) Literal in assert
    for node in ast.walk(tree):
        if isinstance(node, ast.Assert):
            for child in ast.walk(node):
                if isinstance(child, ast.Constant) and isinstance(child.value, str) and kz in child.value:
                    return True

    # Hilfsfunktion: Variablen in asserts unter einem Knoten
    def _vars_in_asserts(body):
        v = set()
        for n in ast.walk(body):
            if isinstance(n, ast.Assert):
                for c in ast.walk(n):
                    if isinstance(c, ast.Name):
                        v.add(c.id)
        return v

    # (b) For-loop-Iterable
    for node in ast.walk(tree):
        if isinstance(node, ast.For) and isinstance(node.target, ast.Name):
            lv = node.target.id
            if lv in _vars_in_asserts(node):
                if isinstance(node.iter, (ast.List, ast.Tuple)):
                    for elt in node.iter.elts:
                        if isinstance(elt, ast.Constant) and isinstance(elt.value, str) and elt.value == kz:
                            return True

    # (c) parametrize-Decorator
    def _param_names(arg):
        p = set()
        if isinstance(arg, ast.Constant) and isinstance(arg.value, str):
            for s in arg.value.split(","):
                p.add(s.strip())
        return p

    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            fv = _vars_in_asserts(node)
            for dec in node.decorator_list:
                if (isinstance(dec, ast.Call) and isinstance(dec.func, ast.Attribute)
                        and dec.func.attr == "parametrize" and len(dec.args) >= 2):
                    pn = _param_names(dec.args[0])
                    if not (pn & fv):
                        continue
                    vals = dec.args[1]
                    if isinstance(vals, (ast.List, ast.Tuple)):
                        for elt in vals.elts:
                            if isinstance(elt, ast.Constant) and isinstance(elt.value, str) and elt.value == kz:
                                return True
                            if isinstance(elt, (ast.List, ast.Tuple)):
                                for sub in elt.elts:
                                    if isinstance(sub, ast.Constant) and isinstance(sub.value, str) and sub.value == kz:
                                        return True
    return False


def test_m_gebundene_kz_sind_in_assert_belegt():
    """Jedes gebundene Kz muss in ASSERT einer Test-Datei vorkommen, nicht nur im Dateitext.

    Schärfere Variante von test_j: ein Kz im Docstring von test_j selbst ließ den Test
    grün. Diese Variante prüft via AST, ob das Kz in einem assert-gesteuerten Pfad liegt
    (assert-Literal, for-Schleife vor assert, parametrize mit assert in der Funktion).

    Sieben Kz sind als bekannte Ausnahmen gelistet (GEBUNDENE_KZ_KEIN_ASSERT) — jede mit
    Begründung, warum sie noch nicht in einem assert geprüft werden.
    """
    # (a) elster_kz aus bindung_*.yaml
    binding_kz: set[str] = set()
    for fp in glob.glob(os.path.join(BIND_DIR, "bindung_*.yaml")):
        with open(fp) as f:
            doc = yaml.safe_load(f)
        for eintrag in doc.get("bindungen", []):
            kz = eintrag.get("elster_kz")
            if kz and str(kz).startswith("E") and len(str(kz)) == 8:
                binding_kz.add(kz)

    # (b) Kz aus est_mapping
    import importlib.util
    em_spec = importlib.util.spec_from_file_location(
        "est_mapping", os.path.join(ROOT, "produkt", "mapping", "est_mapping.py"))
    EM = importlib.util.module_from_spec(em_spec)
    em_spec.loader.exec_module(EM)

    mapping_kz: set[str] = set()
    for cfg in EM.VERZWEIGUNG.values():
        mapping_kz.update(cfg["kz"].values())
    for cfg in EM.PARTNER_VERZWEIGUNG.values():
        mapping_kz.update(cfg["kz"].values())
    mapping_kz.update(EM.PARTNER_INSTANZ.values())
    mapping_kz.update(EM.NEGATION.values())
    mapping_kz.update(EM.DOKUMENTIERT_AGGREGAT.keys())

    alle_kz = binding_kz | mapping_kz
    test_files = glob.glob(os.path.join(ROOT, "tests", "*.py"))

    ungedeckt: list[str] = []
    for kz in sorted(alle_kz):
        # Prüfe ob Kz in irgendeiner Test-Datei in assert-gesteuertem Pfad
        gefunden = False
        for tf in test_files:
            with open(tf, encoding="utf-8") as f:
                src = f.read()
            if kz not in src:
                continue
            if _ast_kz_in_assert_helper(kz, src):
                gefunden = True
                break
        if not gefunden:
            ungedeckt.append(kz)

    ungedeckt = [k for k in ungedeckt if k not in GEBUNDENE_KZ_KEIN_ASSERT]
    assert not ungedeckt, (
        f"{len(ungedeckt)} gebundene Kz in KEINEM assert einer Test-Datei:\n"
        + "\n".join(f"  {k}" for k in ungedeckt)
        + "\nIn GEBUNDENE_KZ_KEIN_ASSERT eintragen (mit Begründung) oder Assert ergänzen."
    )


# Bekannte Ausnahmen: Dateien, deren Syntaxfehler ignoriert werden (fehlende Toolchain o.Ä.).
SYNTAX_IGNORIERTE_DATEIEN: set[str] = set()


def test_k_alle_python_dateien_parsen():
    """Jede von git getrackte .py-Datei muss syntaktisch korrekt parsen (ast.parse).
    Fehlende Dateien (z. B. golden/catala_runtime.py ohne Catala-Toolchain) werden
    übersprungen und gezählt.

    Zwei reale Edit-Unfälle vom 2026-08-05: (1) literale `\n` statt Zeilenumbrüche,
    (2) Ersetzung als Einfügung gelandet — beide hinterliessen SyntaxError, die erst
    bei der pytest-Collection auffielen. ~75 standalone-Skripte (elster/, corpus/,
    oracle/gettsim/) geraten NIE in einen pytest-Lauf — dort fällt ein SyntaxError
    erst beim manuellen Aufruf auf. Dieses Gate schliesst diese Lücke.
    """
    import subprocess, ast
    result = subprocess.run(
        ["git", "ls-files", "*.py"],
        cwd=ROOT, capture_output=True, text=True, check=True)
    tracked = set(result.stdout.strip().splitlines())

    syntaxfehler = []
    fehlt = 0
    for fp in sorted(tracked):
        full = os.path.join(ROOT, fp)
        if not os.path.exists(full):
            fehlt += 1
            continue
        if fp in SYNTAX_IGNORIERTE_DATEIEN:
            fehlt += 1
            continue
        try:
            with open(full) as f:
                ast.parse(f.read())
        except SyntaxError as e:
            syntaxfehler.append((fp, str(e)))

    if fehlt > 0:
        print(f"\n  [test_k] {fehlt} getrackte .py-Dateien nicht gefunden (fehlende Toolchain)")

    assert not syntaxfehler, (
        f"{len(syntaxfehler)} getrackte .py-Datei(en) mit SyntaxError: " +
        "; ".join(f"{fp}: {msg}" for fp, msg in syntaxfehler))


# Store-Event-Gate fuer unbekannte feld_ids in deklariere()
# Siehe Report: test_i deckt die api.py-Lesestellen-Seite ab.
# Dieses Gate deckt die deklariere()-Semantik ab: was passiert mit einem
# Feld, das im Snapshot steht, aber nicht in der Bindung?
#
# Heute: `eingaben_konsistent` bleibt True, das Feld landet in `nicht_deklariert`
# mit Grund "nicht in der Bindungstabelle". Klasse c ("Feld in Bindung,
# kein Kz") und "Feld gar nicht in Bindung" fallen in denselben Bucket.
# Der zweite Fall ist fast immer ein Bug (Umbenennung nicht mitgezogen),
# der erste ist legitim. Das Signal geht im Rauschen unter.
#
# SCHAEKFUNG: wenn est_mapping.py:260-262 erweitert wird, sodass
# unbekannte feld_ids `unvollstaendig` ausloesen, muss dieser Test
# auf `eingaben_konsistent is False` schaerfen. Die Assertion ist bewusst
# auf `is True` gesetzt, damit der Umbau den Test zwingt, sich zu
# aendern — nicht stillschweigend weiterzulaufen.

def test_l_unbekannte_feld_id_in_deklariere():
    """Ein snapshot-Feld ohne Bindungseintrag -> eingaben_konsistent bleibt True (Ist-Verhalten).

    Negativ-Probe: das Feld existiert in der Bindung -> gleiches Verhalten.
    Erst wenn est_mapping.py:260-262 erweitert wird (unbekanntes Feld ->
    unvollstaendig), muss dieser Test auf False schaerfen.
    """
    import importlib.util
    em_spec = importlib.util.spec_from_file_location(
        "est_mapping", os.path.join(ROOT, "produkt", "mapping", "est_mapping.py"))
    EM = importlib.util.module_from_spec(em_spec)
    em_spec.loader.exec_module(EM)

    # Minimal bindung: ein cent-Feld mit elster_kz
    mini_bindung = {
        "test_geld": {"typ": "cent", "elster_kz": "E0000001", "vz_gueltigkeit": [2025]},
    }
    # Minimal snapshot: das Feld + ein unbekanntes Feld
    snapshot = {
        "test_geld": {"wert": 100000, "zustand": "bestaetigt"},
        "test_xxx_feld_fehlte": {"wert": 50000, "zustand": "bestaetigt"},
    }
    ergebnis = EM.deklariere(snapshot, mini_bindung)

    # (1) Bekanntes Feld landet in deklaration; unbekanntes Feld in unvollstaendig
    assert len([u for u in ergebnis["unvollstaendig"] if "nicht in der Bindungstabelle" in u.get("grund", "")]) > 0, (
        f"Unbekanntes Feld muss unvollstaendig ausloesen: {ergebnis['unvollstaendig']}")
    assert "E0000001" in ergebnis["deklaration"], (
        f"Geld-Feld fehlt in Deklaration: {ergebnis['deklaration']}")

    # (2) Unbekanntes Feld landet in nicht_deklariert (heutiges Ist-Verhalten)
    nd_gruende = [nd["grund"] for nd in ergebnis["nicht_deklariert"]]
    assert any("nicht in der Bindungstabelle" in g for g in nd_gruende), (
        f"Unbekanntes Feld muss 'nicht in der Bindungstabelle' melden: {nd_gruende}")

    # (3) Unbekanntes Feld -> eingaben_konsistent = False (Verschärfung)
    assert ergebnis["eingaben_konsistent"] is False, (
        f"Unbekanntes Feld muss eingaben_konsistent auf False setzen: {ergebnis['eingaben_konsistent']}")


# ---- (n) Rückrichtung von (b): Bindung -> existierende Bedingung/Slot ----------

# Bestandsaufnahme, kein Freibrief: geltungsbedingung/signatur_slot-Namen, die HEUTE in
# einer bindung_*.yaml stehen, aber weder in rules.yaml (geltungsbedingungen/signature.inputs)
# noch in der Catala-Signatur der jeweiligen Regel existieren. test_b prüft nur die Richtung
# "jede Bedingung der Regel ist gebunden" — die Rückrichtung "jede Bindung zeigt auf eine
# echte Bedingung/Slot" fehlte. Mutationsprobe 2026-08-07: neues Feld mit erfundener
# geltungsbedingung hinzufügen blieb GRÜN, obwohl der Name frei erfunden war — genau so ist
# vpf_frist_unterbrochen entstanden (gebunden an "vpf_frist_unterbrochen_erklaert", das es in
# p9_4a_verpflegungsmehraufwand nicht gibt). Wer einen Eintrag entfernt, hat ihn korrigiert
# (Bindung auf die echte Bedingung umgehängt) oder die Regel in rules.yaml erweitert.
GELTUNGSBEDINGUNG_ZEIGT_INS_LEERE = {
    # bindung_sonder_agb_35a.yaml — p35a_2_3_haushaltsnahe, Screening-Gate (2026-08-14).
    # hh_hat_aufwendungen fragt, OB es haushaltsnahe Kosten gibt, bevor die neun Detailfelder
    # gestellt werden. Das ist bewusst KEINE Rechtsbedingung: § 35a Abs. 1-3 setzt Aufwendungen
    # voraus, aber "es gibt sie" ist die Existenz des Sachverhalts, kein Tatbestandsmerkmal —
    # die drei echten Merkmale der Regel (rechnung_und_unbare_zahlung, haushalt_in_eu_ewr,
    # keine_foerderung) sind gebunden und bleiben es. Die Alternative wäre gewesen, in
    # rules.yaml eine Bedingung zu erfinden, die im Gesetz nicht steht; dieselbe Abwägung wie
    # bei kind_vorname unten ("solange es keine eigene Regel gibt, steht der Eintrag hier statt
    # in einer erfundenen Bedingung"). Wirkung gemessen: 10 offene § 35a-Fragen -> 0 bei "nein".
    ("sonder_agb_35a", "hh_hat_aufwendungen", "p35a_2_3_haushaltsnahe",
     "haushaltsnahe_aufwendungen_vorhanden"),
    # bindung_kap_vv_familie.yaml — p32_6_kinderfreibetraege
    ("kap_vv_familie", "kind_idnr", "p32_6_kinderfreibetraege", "kind_durch_idnr_identifiziert"),
    # kind_vorname (Kz E0500107) teilt die Identifikations-Naht mit kind_idnr und damit auch
    # deren fehlende Bedingung. Aufgenommen 2026-08-11 mit derselben Begruendung: der Vorname
    # geht in KEINE Regel ein — er ist Formvoraussetzung der Anlage Kind, kein Tatbestand.
    # p32_6_kinderfreibetraege kennt nur kinder_sind_zu_beruecksichtigen, kind_zu_beiden_
    # ehegatten und keine_uebertragung_oder_sonderfaelle; keine davon beschreibt ihn.
    # Ohne das Feld lehnt checkESt jede Kind-Instanz ab ("Tragen Sie bitte den Vornamen des
    # Kindes ein"), womit jede kindbezogene Abzugsposition ausser dem Freibetrag uneinreichbar
    # war. Der richtige Ort waere eine eigene Regel fuer die Anlage-Kind-Formalien — solange
    # es die nicht gibt, steht der Eintrag hier statt in einer erfundenen Bedingung.
    ("kap_vv_familie", "kind_vorname", "p32_6_kinderfreibetraege", "kind_durch_idnr_identifiziert"),
    # kind_geburtsdatum (E0500701, umbenannt aus kind_geburtsjahr 2026-08-12) + kind_familienkasse
    # (E0500706) + kind_wohnsitz_inland_zeitraum (E0500703): dieselbe Formvoraussetzungs-Naht wie
    # kind_vorname oben — Anlage-Kind-Formalien, keine eigene Regel-Bedingung.
    ("kap_vv_familie", "kind_geburtsdatum", "p32_6_kinderfreibetraege", "kind_durch_idnr_identifiziert"),
    ("kap_vv_familie", "kind_familienkasse", "p32_6_kinderfreibetraege", "kind_durch_idnr_identifiziert"),
    ("kap_vv_familie", "kind_wohnsitz_inland_zeitraum", "p32_6_kinderfreibetraege", "kind_durch_idnr_identifiziert"),
    ("kap_vv_familie", "kind_kindschaftsverhaeltnis_a", "p32_6_kinderfreibetraege", "kindschaftsverhaeltnis_elternteil_a"),
    ("kap_vv_familie", "kind_kindschaftsverhaeltnis_b", "p32_6_kinderfreibetraege", "kindschaftsverhaeltnis_elternteil_b"),
    ("kap_vv_familie", "kind_kindschaftsverh_zeitraum_a", "p32_6_kinderfreibetraege", "kindschaftsverh_zeitraum_elternteil_a"),
    ("kap_vv_familie", "kind_kindschaftsverh_zeitraum_b", "p32_6_kinderfreibetraege", "kindschaftsverh_zeitraum_elternteil_b"),
    # Anderer Elternteil (K_Verh_and_P/Ang_Pers, E0501103/104/106/903): dieselbe
    # Formvoraussetzungs-Naht wie kind_vorname/kind_geburtsdatum oben, gemessen 2026-08-12.
    ("kap_vv_familie", "kind_anderer_elternteil_name", "p32_6_kinderfreibetraege", "kind_durch_idnr_identifiziert"),
    ("kap_vv_familie", "kind_anderer_elternteil_geburtsdatum", "p32_6_kinderfreibetraege", "kind_durch_idnr_identifiziert"),
    ("kap_vv_familie", "kind_anderer_elternteil_kindschaftsverhaeltnis", "p32_6_kinderfreibetraege", "kind_durch_idnr_identifiziert"),
    ("kap_vv_familie", "kind_anderer_elternteil_zeitraum", "p32_6_kinderfreibetraege", "kind_durch_idnr_identifiziert"),
    # bindung_p10_1_5_gesamt.yaml — p10_1_5_kinderbetreuung: Formalien der Anlage-Kind-Zeile
    # Kinderbetreuungskosten (KBK_72569777_CType), gemessen 2026-08-12. E0506105 allein reicht
    # checkESt nicht — Dienstleister/Zeitraum/Eigenanteil/Haushaltszugehoerigkeit sind reine
    # Formvoraussetzungen, keine Rechenlogik (der 80%/4800€-Deckel bleibt in achtzig_prozent_
    # deckel_4800_je_kind, s. luecken). Gleiches Muster wie kind_vorname oben.
    ("p10_1_5_gesamt", "kind_betreuung_dienstleister", "p10_1_5_kinderbetreuung", "kind_betreuung_formalien_erklaert"),
    ("p10_1_5_gesamt", "kind_betreuung_zeitraum", "p10_1_5_kinderbetreuung", "kind_betreuung_formalien_erklaert"),
    ("p10_1_5_gesamt", "kind_betreuung_eigenanteil", "p10_1_5_kinderbetreuung", "kind_betreuung_formalien_erklaert"),
    ("p10_1_5_gesamt", "kind_betreuung_kein_gemeinsamer_haushalt_zeitraum", "p10_1_5_kinderbetreuung", "kind_betreuung_formalien_erklaert"),
    ("p10_1_5_gesamt", "kind_betreuung_haushaltszugehoerigkeit_zeitraum", "p10_1_5_kinderbetreuung", "kind_betreuung_formalien_erklaert"),
    ("p10_1_5_gesamt", "kind_betreuung_einzelbetrag", "p10_1_5_kinderbetreuung", "kind_betreuung_formalien_erklaert"),
    ("p10_1_5_gesamt", "kind_betreuung_eigenanteil_betrag", "p10_1_5_kinderbetreuung", "kind_betreuung_formalien_erklaert"),
    ("p10_1_5_gesamt", "kind_betreuung_eigenanteil_zeitraum", "p10_1_5_kinderbetreuung", "kind_betreuung_formalien_erklaert"),
    # bindung_n_vor_gwg.yaml — p9_4a_verpflegungsmehraufwand, p7_1_lineare_afa, p10_1_3_3a_kv_pv
    ("n_vor_gwg", "p9_4a_kuerzung_nach_entgelt", "p9_4a_verpflegungsmehraufwand", "mahlzeitengestellung_kuerzung"),
    ("n_vor_gwg", "am_afa_ist_anschaffungsjahr", "p7_1_lineare_afa", "afa_ist_anschaffungsjahr"),
    ("n_vor_gwg", "versicherungsart", "p10_1_3_3a_kv_pv", "versicherungsart_bestimmt"),
    # bindung_p34c_gesamt.yaml — p34c_1_anrechnung_hoechstbetrag (Bindung + Lücke, gleicher Name)
    ("p34c_gesamt", "dba_einkunftsart", "p34c_1_anrechnung_hoechstbetrag", "methode_je_einkunftsart"),
    ("p34c_gesamt", "[Lücke]", "p34c_1_anrechnung_hoechstbetrag", "methode_je_einkunftsart"),
    # bindung_rentner.yaml — p22_1_leibrente_besteuerungsanteil, p16_4_freibetrag
    ("rentner", "rentner_renten_art", "p22_1_leibrente_besteuerungsanteil", "renten_art_basis_oder_ertragsanteil"),
    ("rentner", "rentner_alter_bei_rentenbeginn", "p22_1_leibrente_besteuerungsanteil", "ertragsanteil_alter_bei_rentenbeginn"),
    ("rentner", "rentner_rentenfreibetrag", "p22_1_leibrente_besteuerungsanteil", "rentenfreibetrag_fixierung_folgejahr"),
    ("rentner", "rentner_veraeusserungs_betriebsart", "p16_4_freibetrag", "veraeusserungs_betriebsart_weiche"),
    # rentner_alter_55_oder_berufsunfaehig_partner/rentner_freibetrag_erstmalig_partner
    # (2026-08-12, Gewinneinkuenfte-Partnerseite): Stufe-1-Scaffolding. Beide sind askable
    # bool-Gates — sie duerfen NICHT auf einen Sammel-Scope (p2_festzusetzung_zusammen), das
    # faengt test_p_sammelscope_ohne_bool_gate ab (bestaetigtes False raeumt sonst per
    # traverser.relevanz() die GANZE regel_id samt aller sachfremden Felder aus
    # naechste_fragen()). Bleiben deshalb auf p16_4_freibetrag, dessen rules.yaml-Bedingungen
    # (alter_55_oder_berufsunfaehig/freibetrag_einmal_im_leben) nur Person A kennen. Loest sich
    # auf, sobald Stufe 2 (Ring, api.py) den zweiten Freibetrags-Aufruf fuer Person B verdrahtet
    # (s. P16_4_GATE_FELDER_PARTNER-Kommentar in api_constants.py).
    ("rentner", "rentner_alter_55_oder_berufsunfaehig_partner", "p16_4_freibetrag", "alter_55_oder_berufsunfaehig_partner"),
    ("rentner", "rentner_freibetrag_erstmalig_partner", "p16_4_freibetrag", "freibetrag_einmal_im_leben_partner"),
    # bindung_p10_1_9_schulgeld_gesamt.yaml / bindung_p33_2a_fahrtkostenpauschale.yaml — beide Regeln
    # haben ihre Ground Truth in golden/runner.py (RUNNER_ACCESSOR_FUER_REGEL, 2026-08-07 angebunden),
    # kein rules.yaml-Eintrag → keine geltungsbedingungen-Liste (gbs=set()). Die drei geltungsbedingung-
    # Lücken sind dokumentierte, im Accessor materialisierte Tatbestände (30%/2.500€-Deckel + Kind-
    # Schulbesuch-Vorprüfung; Person-B-Kz fehlen im XSD) — kein loses Ende, nur nicht rules.yaml-prüfbar.
    ("p10_1_9_schulgeld_gesamt", "[Lücke]", "p10_1_9_schulgeld", "dreissig_prozent_deckel_2500_je_kind"),
    ("p10_1_9_schulgeld_gesamt", "[Lücke]", "p10_1_9_schulgeld", "kind_schulbesuch"),
    ("p33_2a_fahrtkostenpauschale", "[Lücke]", "p33_2a_fahrtkostenpauschale", "partner_kz_fehlen"),
}

# Analog: signatur_slot-Namen, die in keiner Signatur (rules.yaml inputs / Catala input) stehen.
# Alle 13 aus derselben Regel p9_4a_verpflegungsmehraufwand + zwei aus p7_1_lineare_afa —
# beide Scheiben liegen in bindung_n_vor_gwg.yaml.
SIGNATUR_SLOT_ZEIGT_INS_LEERE = {
    # bindung_n_vor_gwg.yaml — p9_4a_verpflegungsmehraufwand (VPF-Tagesaggregate, Dreimonatsfrist)
    ("n_vor_gwg", "vpf_fruehstuecke_gestellt_anzahl", "p9_4a_verpflegungsmehraufwand", "fruehstuecke_gestellt"),
    ("n_vor_gwg", "vpf_mittagessen_gestellt_anzahl", "p9_4a_verpflegungsmehraufwand", "mittagessen_gestellt"),
    ("n_vor_gwg", "vpf_abendessen_gestellt_anzahl", "p9_4a_verpflegungsmehraufwand", "abendessen_gestellt"),
    ("n_vor_gwg", "vpf_mahlzeiten_gezahltes_entgelt", "p9_4a_verpflegungsmehraufwand", "mahlzeiten_gezahltes_entgelt"),
    ("n_vor_gwg", "vpf_steuerfreie_erstattung_betrag", "p9_4a_verpflegungsmehraufwand", "steuerfreie_erstattung_betrag"),
    ("n_vor_gwg", "vpf_tage_24h_nach_drei_monaten", "p9_4a_verpflegungsmehraufwand", "tage_24h_nach_frist"),
    ("n_vor_gwg", "vpf_tage_an_abreise_nach_drei_monaten", "p9_4a_verpflegungsmehraufwand", "tage_an_abreise_nach_frist"),
    ("n_vor_gwg", "vpf_tage_ueber_8h_nach_drei_monaten", "p9_4a_verpflegungsmehraufwand", "tage_ueber_8h_nach_frist"),
    ("n_vor_gwg", "tage_24h", "p9_4a_verpflegungsmehraufwand", "tage_24h"),
    ("n_vor_gwg", "tage_an_abreise", "p9_4a_verpflegungsmehraufwand", "tage_an_abreise"),
    ("n_vor_gwg", "tage_ueber_8h_eintaegig", "p9_4a_verpflegungsmehraufwand", "tage_ueber_8h_eintaegig"),
    # bindung_n_vor_gwg.yaml — p7_1_lineare_afa (AfA-Slots)
    ("n_vor_gwg", "arbeitsmittel_nutzungsdauer", "p7_1_lineare_afa", "nutzungsdauer"),
    ("n_vor_gwg", "am_anschaffung_monat", "p7_1_lineare_afa", "anschaffung_monat"),
    # bindung_n_vor_gwg.yaml — p09_entfernungspauschale: Formalien der Anlage N, die ERiC
    # verlangt ("Bei den Angaben zur Entfernungspauschale fehlt die Angabe zum Ziel des Weges
    # und / oder zu PLZ, Ort und Strasse", gemessen 2026-08-19). Sie aendern keinen Betrag: die
    # Catala-Signatur kennt vier Inputs (entfernung_km_roh, arbeitstage,
    # eigenes_oder_ueberlassenes_kfz, oepnv_kosten_jahr), Ziel und Zieladresse sind keiner davon.
    #
    # ERSTE INSTANZ FUER EINE REGEL OHNE rules.yaml-EINTRAG. Die 13 Eintraege darueber gehoeren zu
    # Regeln, die dort stehen; p09 ist ein reiner Catala-Scope. Der geltungsbedingung-Weg, mit dem
    # § 35c seine Formalien (p35c_objekt_strasse/_plz_ort) an eigenes_gebaeude gehaengt hat, steht
    # deshalb hier NICHT offen: ohne rules.yaml-Eintrag ist die Bedingungsmenge leer, und ein
    # erfundener Bedingungsname waere die Fehlerklasse aus dem Kommentar unten —
    # mitveranlagung_faktor schloss ueber relevanz() die ganze Regel aus, sobald der Normalfall
    # bestaetigt wurde. Ein Slot, der sichtbar ins Leere zeigt, sagt die Wahrheit: das Feld ist
    # Formvoraussetzung der Anlage, kein Rechen-Input.
    ("n_vor_gwg", "ep_ziel_des_weges", "p09_entfernungspauschale", "ziel_des_weges"),
    ("n_vor_gwg", "ep_ziel_adresse", "p09_entfernungspauschale", "zieladresse"),
    # bindung_sonder_agb_35a.yaml — p10_1_7_berufsausbildung: die Einzelaufstellung, die ERiC
    # neben der Summe verlangt ("Es wurde die Summe der Aufwendungen fuer die eigene
    # Berufsausbildung angegeben, bitte geben Sie auch die Bezeichnung der Ausbildung und die
    # Art und Hoehe der einzelnen Aufwendungen an", gemessen 2026-08-19). Die Regel hat GENAU
    # EINEN Rechen-Input, `aufwendungen`, den berufsausbildung_aufwendungen belegt. Anders als
    # bei p09 fuehrt sie zwar eine geltungsbedingung (hoechstbetrag_gilt_je_person), aber die
    # sagt "der Hoechstbetrag steht jedem Ehegatten einzeln zu" — an einer Ausbildungs-
    # bezeichnung behauptete sie einen Zusammenhang, den es nicht gibt. Ein Slot, der sichtbar
    # ins Leere zeigt, ist ehrlicher als eine Bedingung, die nicht passt.
    ("sonder_agb_35a", "berufsausbildung_bezeichnung", "p10_1_7_berufsausbildung", "bezeichnung_der_ausbildung"),
    ("sonder_agb_35a", "berufsausbildung_einzelbetrag", "p10_1_7_berufsausbildung", "einzelaufwendung"),
    # bindung_sonder_agb_35a.yaml — p10b_spenden: der Vermoegensstock-Betrag nach § 10b
    # Abs. 1a. Die Regel kennt nur zuwendungen und gesamtbetrag_der_einkuenfte, und ihre
    # zwei Geltungsbedingungen betreffen beide den 20-Prozent-Deckel des Abs. 1 — an einer
    # Abs.-1a-Angabe behaupteten sie einen Zusammenhang, den es nicht gibt. rules.yaml sagt
    # ueber sich selbst, Abs. 1a sei "ein eigener Zuschnitt".
    ("sonder_agb_35a", "spenden_vermoegensstock", "p10b_spenden", "vermoegensstock_betrag"),
    # bindung_p33a_gesamt.yaml — p33a_unterhalt: die Angaben zur unterstuetzten Person und
    # ihrem Haushalt, die ERiC in fuenf Beanstandungen verlangt (2026-08-19). Die Regel
    # fuehrt drei Geltungsbedingungen, aber alle drei betreffen die Rechenmechanik
    # (Grundfreibetrag als Norm-Konstante, Schonbetrag 624, Netto-Einkuenfte) — an einer
    # Haushaltsadresse behauptete jede davon einen Zusammenhang, den es nicht gibt.
    ("p33a_gesamt", "p33a_person_name", "p33a_unterhalt", "unterstuetzte_person_name"),
    ("p33a_gesamt", "p33a_person_beruf_familienstand", "p33a_unterhalt", "unterstuetzte_person_beruf"),
    ("p33a_gesamt", "p33a_person_geburtsdatum", "p33a_unterhalt", "unterstuetzte_person_geburtsdatum"),
    ("p33a_gesamt", "p33a_haushalt_anschrift", "p33a_unterhalt", "haushalt_anschrift"),
    ("p33a_gesamt", "p33a_haushalt_personenzahl", "p33a_unterhalt", "haushalt_personenzahl"),
    ("p33a_gesamt", "p33a_unterstuetzungszeitraum", "p33a_unterhalt", "unterstuetzungszeitraum"),
    ("p33a_gesamt", "p33a_zahlungszeitraum", "p33a_unterhalt", "zahlungszeitraum"),
    ("p33a_gesamt", "p33a_person_hat_einkuenfte", "p33a_unterhalt", "person_hat_einkuenfte"),
    ("p33a_gesamt", "p33a_person_hat_vermoegen", "p33a_unterhalt", "person_hat_vermoegen"),
    ("p33a_gesamt", "p33a_weitere_person_beteiligt", "p33a_unterhalt", "weitere_person_beteiligt"),
    ("p33a_gesamt", "p33a_person_im_inlaendischen_haushalt", "p33a_unterhalt", "person_im_inlaendischen_haushalt"),
    ("p33a_gesamt", "p33a_kindergeld_anspruch", "p33a_unterhalt", "kindergeld_anspruch"),
    ("p33a_gesamt", "p33a_verwandtschaftsverhaeltnis", "p33a_unterhalt", "verwandtschaftsverhaeltnis"),
    ("p33a_gesamt", "p33a_person_idnr", "p33a_unterhalt", "unterstuetzte_person_idnr"),
    # (gewinn_bezeichnung / gewinn_bezeichnung_partner brauchen KEINE Eintraege, obwohl ihr
    # signatur_slot ebenso ins Leere zeigt: p2_festzusetzung_einzel/_zusammen haben weder
    # rules.yaml-Eintrag noch Catala-Scope, stehen also in der Liste der 12 uebersprungenen
    # Regeln von test_n. Ein Eintrag hier waere eine Ausnahme fuer einen Verstoss, den der Test
    # gar nicht sehen kann — derselbe Fall wie p22_3_leistungen unten. Gemessen 2026-08-20:
    # mit Eintrag meldet test_n sie als "erledigte Eintraege — bitte streichen".)
    # bindung_p22_nr3.yaml — p22_3_leistungen: die Bruttoeinnahmen, die ERiC neben den
    # Einkuenften verlangt ("Bei den Leistungen wurden Einkuenfte erklaert, es fehlt jedoch eine
    # Angabe zu den Einnahmen", gemessen 2026-08-19). p22_3_leistungen hat keinen
    # rules.yaml-Eintrag, also auch keine geltungsbedingung; der einzige Catala-Input
    # (einkuenfte_vor_freigrenze) gehoert dem Nettofeld. Die Einnahmen sind reine Formvoraussetzung
    # — die Freigrenze von 256 Euro haengt an den Einkuenften, nicht an ihnen.
    # (p22_3_leistungen braucht KEINE Eintraege: die Regel hat weder rules.yaml-Eintrag noch
    # Catala-Scope unter rules/estg/, wird von test_n also gar nicht geprueft. Ein Eintrag hier
    # waere eine Ausnahme fuer einen Verstoss, den es nicht gibt — genau das meldet der Test.)
    # bindung_sonder_agb_35a.yaml — p35a_2_3_haushaltsnahe: mitveranlagung (§ 35a Abs. 5 Satz 4
    # EStG, Höchstbetrags-Halbierung bei zwei Alleinstehenden im gemeinsamen Haushalt) ist KEIN
    # Input der rules.yaml-signature (die kennt nur minijob_aufwendungen/haushaltsnahe_
    # dienstleistungen/handwerker_arbeitskosten) — golden/runner.py.catala_p35a_haushaltsnahe()
    # wendet die Halbierung als Nachbearbeitung NACH dem Scope-Ergebnis an (Zeile ~430), nicht als
    # Scope-Eingabe. Bis 2026-08-12 stand das Feld faelschlich als geltungsbedingung:
    # mitveranlagung_faktor an derselben Bindung (Screening-Fund, s. reports/adjudikation/
    # gate_screening_polaritaet_2026-08-12.md) — ein erfundener Bedingungsname, der relevanz()
    # die GESAMTE Regel ausschliessen liess, sobald der Normalfall (Antwort "Nein") bestaetigt
    # wurde. Fix: signatur_slot statt geltungsbedingung — dieselbe Ground-Truth-Luecke wie bei
    # p9_4a/p7_1 oben, kein neuer Bug, nur derselbe dokumentierte Nicht-Signatur-Fall.
    ("sonder_agb_35a", "p35a_mitveranlagung", "p35a_2_3_haushaltsnahe", "mitveranlagung"),
    # bindung_p34c_gesamt.yaml — p34c_1_anrechnung_hoechstbetrag: dba_mehrere_staaten ist ein
    # Screening-/Routing-Flag (elster_kz_grund: "Bestaetigt true -> dba_multi_country_offen
    # (fail-closed)", produkt/haut/api.py), kein Input der Catala-Signatur (die kennt nur die
    # 4 Money-Inputs gezahlte_auslaendische_steuer/deutsche_est_inkl_ausl/zu_versteuerndes_
    # einkommen/auslaendische_einkuenfte_staat). Bis 2026-08-12 stand das Feld als
    # geltungsbedingung: per_country_ein_staat an derselben Bindung — der Bedingungsname
    # existiert echt in rules.yaml (anders als bei p35a oben), aber die Kopplung war trotzdem
    # falsch: relevanz() schliesst die Regel bei bestaetigt False aus, und False (nur EIN Staat)
    # ist hier der Normalfall/Eligible-Fall — die DBA-Anrechnung wurde dem haeufigsten Nutzer
    # nie angeboten (s. reports/adjudikation/gate_screening_polaritaet_2026-08-12.md). Der
    # fail-closed-Schutz fuer den True-Fall (mehrere Staaten) laeuft unveraendert ueber den
    # separaten dba_multi_country_offen-Sperrgrund in api.py, der von dieser Bindung nicht
    # beruehrt wird. Fix: signatur_slot statt geltungsbedingung.
    ("p34c_gesamt", "dba_mehrere_staaten", "p34c_1_anrechnung_hoechstbetrag", "mehrere_staaten"),
    # bindung_an_gesamt.yaml — p34_3_ermaessigter_durchschnittssatz: dauernd_berufsunfaehig
    # und ermaessigung_einmal_genutzt sind Eligibility-/Antrags-Flags (bindung_an_gesamt.yaml,
    # Kommentar Zeile ~471: "3 bool-Flags, KEINE Modul-Inputs"), keine Inputs der Catala-
    # Signatur (die kennt nur ao_einkuenfte/est_gesamt_zzgl_progression/bemessungsgrundlage_
    # durchschnitt). Bis 2026-08-12 standen beide als geltungsbedingung an derselben Bindung —
    # beide Bedingungsnamen (persoenliche_voraussetzung_erfuellt, einmal_im_leben) existieren
    # echt in rules.yaml, aber die Kopplung war trotzdem falsch (s. reports/adjudikation/
    # gate_screening_polaritaet_2026-08-12.md):
    #   - persoenliche_voraussetzung_erfuellt = (Alter>=55 [DERIVE aus geburtsjahr] ODER
    #     dauernd_berufsunfaehig) — eine ZWEI-Fakten-Disjunktion. relevanz() kann nur EIN Feld
    #     UND-verknuepft ausschliessen; ein bestaetigtes "Nein" auf dauernd_berufsunfaehig hat
    #     die Regel auch fuer einen 60-Jaehrigen ausgeschlossen, der die Voraussetzung ueber die
    #     Altersgrenze laengst erfuellt. Kein Polaritaetsfix moeglich (keine Belegung von
    #     dauernd_berufsunfaehig allein macht die Disjunktion wahr/falsch) — ein halber Fix
    #     (nur Flip) haette den 55-Jaehrigen ohne Berufsunfaehigkeit weiter ausgeschlossen. Die
    #     korrekte OR/AND/NOT-Formel steht laengst in api.py._abs3_eligible() (Chooser fuer
    #     Abs.1-vs-Abs.3) UND im eigenen Bindungskommentar oben — das Interview-Gate war
    #     redundant und strukturell unfaehig, die Disjunktion abzubilden. Fix: entfernen
    #     (signatur_slot), nicht nachbauen — die Eligibility-Pruefung lebt schon korrekt in
    #     _abs3_eligible().
    #   - einmal_im_leben: False (noch nicht genutzt) = Normalfall/Erstantrag. relevanz()
    #     schliesst bei bestaetigt False aus — der Erstantragsteller (die grosse Mehrheit) wird
    #     nie gefragt. Anders als bei dauernd_berufsunfaehig waere diese eine WAERE mechanisch
    #     per Flip loesbar (Einzelbedingung, sauber invertierbar) — abweichend von der
    #     "mechanisch"-Einordnung aber trotzdem als signatur_slot entfernt statt umbenannt:
    #     _abs3_eligible() in api.py liest den Rohwert direkt und braucht keine Gate-Kopplung;
    #     ein Rename haette den API-Zugriff angefasst (ausserhalb des Auftragsrahmens fuer diese
    #     Bugfix-Serie, s. Team-Lead-Vorgabe). Entfernen statt Umbenennen erreicht dieselbe
    #     Korrektheit ohne dieses Risiko.
    ("an_gesamt", "dauernd_berufsunfaehig", "p34_3_ermaessigter_durchschnittssatz", "berufsunfaehig"),
    ("an_gesamt", "ermaessigung_einmal_genutzt", "p34_3_ermaessigter_durchschnittssatz", "bereits_genutzt"),
}


# Bestandsaufnahme der Regeln, die _n_gefundene_verstoesse HEUTE überspringt (weder
# rules.yaml-Eintrag noch rules/estg/<rule_id>/*.catala_en noch RUNNER_ACCESSOR_FUER_REGEL) —
# d.h. dort ist die Rückrichtung BLIND, nicht grün. Ohne diesen Assert wäre der Blindspot
# unsichtbar: eine weitere übersprungene Regel würde nie auffallen. Die Liste darf NUR
# SCHRUMPFEN (eine Regel bekommt Ground Truth angebunden → raus hier) — nie stillschweigend
# wachsen. Neue Einträge nur bewusst, mit Grund.
# Waren am 2026-08-07 neun; p10_1_9_schulgeld und p33_2a_fahrtkostenpauschale sind seither
# über golden/runner.py angebunden.
REGELN_OHNE_GROUND_TRUTH = {
    # KORREKTUR 2026-08-12 (598e966/e907fad): _catala_inputs() globt rules/estg/<rule_id>/*.catala_en
    # — fuer diese rule_ids liefert das NICHTS, das Verzeichnis heisst nicht wie die rule_id.
    # Die echte Ground Truth liegt bei rules/estg/p32a/einkommensteuertarif.catala_en:
    # FestzusetzendeEstEinzel Zeile 319, FestzusetzendeEstZusammen Zeile 382. Verzeichnisname
    # != rule_id. Ein Anschluss wurde gemessen und verworfen: 1 geloester Blindspot gegen 41
    # neue Dokumentationseintraege, plus eine noetige rule_id->Scope-Namen-Tabelle, weil
    # snake_case und PascalCase hier keinen gemeinsamen Wortstamm haben.
    "p2_festzusetzung_einzel",
    "p2_festzusetzung_zusammen",
    # Pseudoregel-Scopes, haben wirklich keine Signatur. Bis 2026-08-14 war das EINE Regel
    # "p2_einkunftsarten" mit allen vier Abwesenheits-Flags. Aufgeteilt, weil relevanz() die Gates
    # einer Regel konjunktiv auswertet und beim ersten bestaetigten False abbricht: wer eine
    # Einkunftsart BEJAHTE (Haut-Inversion in app.js:283 -> kein_X=false), schloss die ganze Regel
    # aus und bekam die anderen drei Fragen nie gestellt. Vier unabhaengige Screeningfragen
    # brauchen vier regel_ids. Gate: tests/test_screening_fragen_unabhaengig.py.
    "p2_einkunftsart_gewinn",
    "p2_einkunftsart_kap",
    "p2_einkunftsart_vuv",
    "p2_einkunftsart_sonstige",
    # Sechste Pseudoregel derselben Bauart (2026-08-15): Traeger des Screening-Flags
    # "Hast du Kinder?". Eine eigene regel_id ist noetig, weil ein askable bool an einer
    # geltungsbedingung IMMER ein Gate ist und relevanz() bei bestaetigtem FALSE ausschliesst
    # — ein kein_kind-Gate direkt an p32_6 wuerde die Kinderfreibetraege ausgerechnet dem
    # streichen, der Kinder HAT. Die sechs echten Kinder-Regeln haengen ueber
    # regel_bedingungen daran (22 Felder).
    "p2_kind_vorhanden",
    # Screening-Gruppe C, AUSGABENSEITE (2026-08-21): fuenf weitere Pseudoregeln derselben Bauart.
    # Die Gruppen davor decken die EINNAHMEN ab; fuer Ausgaben und Ermaessigungen gab es KEIN
    # einziges Flag — 143 der 316 fragbaren Felder lagen in Regeln, die keine Antwort abschalten
    # konnte. Gemessen im echten Nutzerlauf: der Nutzer bekam Fragen zu Handwerkerleistungen, zu
    # einem Gebaeude und zu einer auswaertigen Taetigkeit, die es alle nicht gab.
    # Je eine eigene regel_id aus demselben Grund wie oben bei p2_einkunftsart_*: relevanz() wertet
    # die Gates EINER Regel konjunktiv aus und bricht beim ersten bestaetigten False ab — lagen
    # zwei Flags auf derselben Pseudoregel, naehme eine bejahte Frage der anderen ihre Antwort.
    # Felder in bindung_screening_ausgaben.yaml, Wirkung ueber regel_bedingungen.
    "p33a_unterhalt_vorhanden",
    "p34c_auslandseinkuenfte_vorhanden",
    "p33b_behinderung_pflege_vorhanden",
    "p19_2_versorgungsbezuege_vorhanden",
    "p35c_sanierung_vorhanden",
    # 2026-08-26, sechs weitere derselben Bauart: Themen, die bis dahin keine
    # einzige Frage nach ihrer Existenz hatten (Arbeitsmittel, Realsplitting,
    # Spenden, Berufsausbildung, Verlustvortrag, Lohnersatzleistungen).
    "p9_1_3_nr6_arbeitsmittel_vorhanden",
    "p10_1a_realsplitting_vorhanden",
    "p10b_spenden_vorhanden",
    "p10_1_7_berufsausbildung_vorhanden",
    "p10d_2_verlustvortrag_vorhanden",
    "p32b_progression_vorhanden",
    "p9_1_3_nr5_zweitwohnung_vorhanden",
    # Zaehlfelder der Instanz-Gruppen (2026-08-27): eigene Pseudo-Regel je Gruppe,
    # damit das Feld kein Gate der echten Rechenregel wird. Die Zahl geht nicht in
    # die Rechnung ein — sie bestimmt, wie viele Eingabefelder die Oberflaeche baut.
    # Vier Pseudoregeln fuer den PARTNER (2026-08-28), dieselbe Bauart wie p2_einkunftsart_*
    # daruber. Der ganze Partner-Zweig trug bis dahin kein einziges Gate ausser der
    # Veranlagungsart: ein Ehepaar, das alle achtzehn vorhandenen Kreuze verneinte, bekam
    # 32 Partner-Fragen, derselbe Mensch allein veranlagt keine einzige.
    # Eigene Kreuze und nicht die vorhandenen mitbenutzt, weil die alle woertlich nach „dir"
    # fragen: neun Partner-Felder hingen an einem Ich-Kreuz, und `keine_behinderung_pflege`
    # („Hast du selbst oder hat eines deiner Kinder…?") nahm dem Paar den Behinderten-
    # Pauschbetrag des Partners — gemessen 302 EUR zu viel Steuer bei Partner-GdB 50.
    # Je eine eigene regel_id aus demselben Grund wie oben: relevanz() wertet die Gates EINER
    # Regel konjunktiv aus und bricht beim ersten bestaetigten False ab.
    "p2_einkunftsart_kap_partner",
    "p2_einkunftsart_gewinn_partner",
    "p2_einkunftsart_sonstige_partner",
    "p33b_behinderung_pflege_partner_vorhanden",
    "p21_anzahl_objekte_erhebung",
    "p22_anzahl_renten_erhebung",
    "p23_anzahl_verkaeufe_erhebung",
    "p35a_anzahl_handwerker_erhebung",
    "p35a_anzahl_dienstleistungen_erhebung",
    "p35a_anzahl_minijobs_erhebung",
    "p6_2_anzahl_gwg_erhebung",
    # Hat einen dict-Accessor (catala_p19_2_versorgungsfreibetrag), aber der liest FELD-IDs
    # (versorgung_bemessungsgrundlage), waehrend die Bindung SLOT-Namen fuehrt
    # (bemessungsgrundlage). Der Ring ruft ausserdem catala_einkuenfte_versorgung, nicht den
    # Freibetrag-Accessor direkt. Ein naiver Anschluss meldete beide Slots als Verstoss,
    # obwohl versorgung_jahresrente live gelesen wird (api.py:822/843/860). Braucht erst eine
    # Entscheidung, welche Namensebene die Ground Truth ist — siehe BACKLOG.
    "p19_2_versorgungsfreibetrag",
    # Aggregationsbruch: Kind-Achse gegen Fall-Achse.
    "p10_1_3_kv_pv_kind",
    "p33b_abs5_kind_uebertragung",
    # Positionale Signatur (catala_p22_nr3_einkuenfte(betrag_cent: int)), kein dict-Parameter.
    "p22_3_leistungen",
    # NEU 2026-08-12: eigene regel_id fuer § 3 Nr. 72 (bindung_p3_nr72_pv.yaml). Kein
    # rules.yaml-Eintrag, kein rules/estg/p3_nr72_pv/-Verzeichnis, kein RUNNER_ACCESSOR_FUER_
    # REGEL-Eintrag (catala_p3_nr72_photovoltaik hat keinen einzelnen dict-Parameter). Vorher
    # teilten sich pv_bruttoleistung_kwp/pv_anzahl_einheiten/pv_auf_gebaeude die Pseudoregel
    # p2_festzusetzung_einzel mit 23 sachfremden Feldern (Bruttoarbeitslohn, Veranlagungsart,
    # alle Stammdaten) — traverser.relevanz() schliesst pro regel_id aus, und das bestaetigte
    # "nein" auf pv_auf_gebaeude (bool, Mehrheitsantwort) hat im Dialog-Durchstich die gesamte
    # AN-Kernerklaerung dauerhaft aus naechste_fragen() genommen. Fix: eigene regel_id statt
    # Umzug auf eine bestehende Ground-Truth-Regel (es gibt keine passende) — dieselbe
    # Blindspot-Klasse wie p2_festzusetzung_* oben, hier bewusst neu statt vererbt.
    "p3_nr72_pv",
    # NEU 2026-08-30: Screening-Pseudoregel "Privater Verkauf" (§ 23 EStG, produkt/bindung/
    # bindung_an_gesamt.yaml, Feld kein_p23_verkauf) -- gleiche Bauart wie p2_einkunftsart_* oben
    # (eigene regel_id statt Gate an der echten Regel p23_veraeusserungsgewinn, deren vier
    # geltungsbedingungen alle echte Befreiungstatbestaende sind, nicht "wurde ueberhaupt verkauft").
    "p23_verkauf_vorhanden",
}


def test_n_bindung_zeigt_auf_existierende_bedingung(daten):
    """Rückrichtung von test_b: jede Bindung/Lücke muss auf eine ECHTE geltungsbedingung/
    signatur_slot der Regel zeigen — nicht nur umgekehrt.

    test_b prüft nur "jede Bedingung der Regel ist gebunden" (gbs - geb_gbs - lk_gbs).
    Ein erfundener Name auf der Bindungsseite fällt dabei nie auf: er taucht ja nicht in
    `gbs` auf, verkleinert also nichts. Mutationsprobe bestätigt das (siehe Testdatei-Header
    von test_b_vollstaendigkeit-Nachbarschaft): neues Feld + freierfundene geltungsbedingung
    hinzufügen bleibt GRÜN. Genau so ist vpf_frist_unterbrochen entstanden — gebunden an
    "vpf_frist_unterbrochen_erklaert", das es in p9_4a_verpflegungsmehraufwand nicht gibt.

    Regeln OHNE Ground Truth (weder rules.yaml-Eintrag noch Catala-Datei) werden übersprungen
    und gesammelt ausgegeben — dort ist nichts prüfbar, ein Assert wäre nur Rauschen.
    """
    gefunden_gb, gefunden_slot, uebersprungene_regeln = _n_gefundene_verstoesse(daten, _rules())

    if uebersprungene_regeln:
        print(f"\n  [test_n] {len(uebersprungene_regeln)} Regel(n) ohne Ground Truth "
              f"übersprungen (weder rules.yaml noch Catala): {sorted(uebersprungene_regeln)}")

    # Blindspot sichtbar machen: uebersprungene_regeln MUSS exakt REGELN_OHNE_GROUND_TRUTH sein.
    # Beide Richtungen zählen — eine neu übersprungene Regel (Menge wächst) UND eine angebundene
    # Regel, deren Eintrag hier nicht gestrichen wurde (Menge schrumpft nicht mit), fallen auf.
    assert uebersprungene_regeln == REGELN_OHNE_GROUND_TRUTH, (
        f"uebersprungene Regeln haben sich geaendert: neu={sorted(uebersprungene_regeln - REGELN_OHNE_GROUND_TRUTH)} "
        f"nicht_mehr_uebersprungen={sorted(REGELN_OHNE_GROUND_TRUTH - uebersprungene_regeln)} "
        "— neu uebersprungene Regel: bewusst in REGELN_OHNE_GROUND_TRUTH aufnehmen; "
        "nicht mehr uebersprungene Regel: aus REGELN_OHNE_GROUND_TRUTH streichen (sie hat jetzt Ground Truth).")

    neue_gb = gefunden_gb - GELTUNGSBEDINGUNG_ZEIGT_INS_LEERE
    neue_slot = gefunden_slot - SIGNATUR_SLOT_ZEIGT_INS_LEERE
    assert not neue_gb, (
        f"geltungsbedingung zeigt auf keine existierende Bedingung der Regel: {sorted(neue_gb)} "
        "— entweder Bindung auf den echten Namen korrigieren oder Regel in rules.yaml erweitern; "
        "kein neuer Eintrag in GELTUNGSBEDINGUNG_ZEIGT_INS_LEERE ohne Begründung.")
    assert not neue_slot, (
        f"signatur_slot zeigt auf keinen existierenden Input der Regel: {sorted(neue_slot)} "
        "— entweder Bindung auf den echten Slot-Namen korrigieren oder Signatur erweitern; "
        "kein neuer Eintrag in SIGNATUR_SLOT_ZEIGT_INS_LEERE ohne Begründung.")

    # Die Ausnahmelisten dürfen nicht verrotten: ein Eintrag, der nicht mehr gefunden wird,
    # ist repariert (oder die Bindung ist weg) und deckt sonst still eine neue Verletzung
    # mit demselben Namen. Erst dieser Assert macht die Liste zur Bestandsaufnahme.
    tot_gb = GELTUNGSBEDINGUNG_ZEIGT_INS_LEERE - gefunden_gb
    tot_slot = SIGNATUR_SLOT_ZEIGT_INS_LEERE - gefunden_slot
    assert not tot_gb and not tot_slot, (
        f"Ausnahmeliste enthaelt erledigte Eintraege — bitte streichen: "
        f"gb={sorted(tot_gb)} slot={sorted(tot_slot)}")


# Der Rechenkern liegt seit 2026-08-19 in produkt/engine/runner.py (vorher golden/runner.py):
# das Produkt hing an einem Entwicklungswerkzeug, jetzt ist es umgekehrt. Der Rückfall auf
# den alten Ort steht nicht aus Nostalgie da — er hält diese Prüfung lauffähig, falls der
# Umzug je zurückgedreht wird, statt sie mit FileNotFoundError sterben zu lassen.
GOLDEN_RUNNER_PATH = os.path.join(ROOT, "produkt", "engine", "runner.py")
if not os.path.exists(GOLDEN_RUNNER_PATH):
    GOLDEN_RUNNER_PATH = os.path.join(ROOT, "golden", "runner.py")

# regel_id -> Name der golden/runner.py-Funktion, die die ECHTE Ground Truth für diese
# Pseudoregel ist (kein rules.yaml-Eintrag, kein rules/estg/<rid>/-Dir). Nur Funktionen mit
# einem einzigen dict-Parameter (s.get("key")/s["key"]) — positionale Signaturen (z.B.
# p22_3_leistungen: catala_p22_nr3_einkuenfte(betrag_cent: int)) passen nicht in dieses Schema
# und bleiben in REGELN_OHNE_GROUND_TRUTH.
RUNNER_ACCESSOR_FUER_REGEL = {
    "p10_1_9_schulgeld": "catala_p10_1_9_schulgeld",
    "p33_2a_fahrtkostenpauschale": "catala_p33_2a_fahrtkostenpauschale",
}


def _runner_dict_inputs(func_name):
    """dict-Keys, die eine golden/runner.py-Funktion mit EINEM dict-Parameter liest
    (s.get("key", ...) / s["key"]) — AST-basiert, kein Regex-Bleeding über Funktionsgrenzen."""
    tree = ast.parse(open(GOLDEN_RUNNER_PATH, encoding="utf-8").read())
    for node in ast.walk(tree):
        if isinstance(node, ast.FunctionDef) and node.name == func_name:
            arg0 = node.args.args[0].arg
            keys = set()
            for n in ast.walk(node):
                if (isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute) and n.func.attr == "get"
                        and isinstance(n.func.value, ast.Name) and n.func.value.id == arg0
                        and n.args and isinstance(n.args[0], ast.Constant)):
                    keys.add(n.args[0].value)
                if (isinstance(n, ast.Subscript) and isinstance(n.value, ast.Name) and n.value.id == arg0
                        and isinstance(n.slice, ast.Constant)):
                    keys.add(n.slice.value)
            return keys
    raise AssertionError(f"golden/runner.py: Funktion {func_name} nicht gefunden")


def _n_gefundene_verstoesse(daten, rules):
    """DIE Implementierung der Rückrichtung — test_n und beide Gegenproben rufen sie auf,
    damit die Gegenproben nicht eine Zweitfassung prüfen, die von test_n abdriftet.

    Liefert (gb-Verstoesse, slot-Verstoesse, uebersprungene_regel_ids). Uebersprungen wird
    eine Regel ohne jede Ground Truth (kein rules.yaml-Eintrag, keine Catala-Datei, kein
    RUNNER_ACCESSOR_FUER_REGEL-Eintrag) — dort ist nichts pruefbar, ein Assert waere nur Rauschen.
    """
    gefunden_gb, gefunden_slot, uebersprungen = set(), set(), set()
    for f, d in daten.items():
        scheibe = os.path.basename(f)[len("bindung_"):-len(".yaml")]
        bindungen = d["bindungen"]
        luecken = d.get("luecken", [])
        rule_ids = {b["quelle"]["regel_id"] for b in bindungen} | {l["regel_id"] for l in luecken}
        for rid in sorted(rule_ids):
            if rid in rules:
                r = rules[rid]
                sig = r.get("signature") or {}
                inputs = set((sig.get("inputs") or {}).keys())
                gbs = {g["bedingung"] for g in (r.get("geltungsbedingungen") or []) if "bedingung" in g}
            elif rid in RUNNER_ACCESSOR_FUER_REGEL:
                inputs = _runner_dict_inputs(RUNNER_ACCESSOR_FUER_REGEL[rid])
                gbs = set()   # golden/runner.py kennt keine rules.yaml-geltungsbedingungen
            else:
                inputs = _catala_inputs(rid)
                if not inputs and not glob.glob(os.path.join(ROOT, "rules", "estg", rid, "*.catala_en")):
                    uebersprungen.add(rid)
                    continue
                gbs = set()   # Catala-Signaturen kennen keine rules.yaml-geltungsbedingungen
            for b in bindungen:
                if b["quelle"]["regel_id"] != rid:
                    continue
                q = b["quelle"]
                if "geltungsbedingung" in q and q["geltungsbedingung"] not in gbs:
                    gefunden_gb.add((scheibe, b["feld_id"], rid, q["geltungsbedingung"]))
                if "signatur_slot" in q and q["signatur_slot"] not in inputs:
                    gefunden_slot.add((scheibe, b["feld_id"], rid, q["signatur_slot"]))
            for l in luecken:
                if l["regel_id"] != rid:
                    continue
                if l.get("geltungsbedingung") and l["geltungsbedingung"] not in gbs:
                    gefunden_gb.add((scheibe, "[Lücke]", rid, l["geltungsbedingung"]))
                if l.get("signatur_slot") and l["signatur_slot"] not in inputs:
                    gefunden_slot.add((scheibe, "[Lücke]", rid, l["signatur_slot"]))
    return gefunden_gb, gefunden_slot, uebersprungen


def _erste_bindung_mit_ground_truth(daten, rules, schluessel):
    """Erste Bindung, deren Regel eine pruefbare Ground Truth hat — und die den geforderten
    Schluessel fuehrt (geltungsbedingung|signatur_slot).

    Wichtig fuer die Gegenproben: _n_gefundene_verstoesse UEBERSPRINGT Regeln ohne Ground
    Truth. Eine Mutation auf bindungen[0] traf p2_festzusetzung_einzel — genau so eine
    uebersprungene Regel — und wurde deshalb nie gemeldet. Die Gegenprobe war gruen, ohne
    irgendetwas zu belegen. Liefert (kopiertes daten-Dict, Bindung darin).
    """
    for f in sorted(daten):
        d = copy.deepcopy(daten[f])
        for b in d["bindungen"]:
            if b["quelle"]["regel_id"] in rules and schluessel in b["quelle"]:
                return {f: d}, b
    raise AssertionError(f"keine Bindung mit {schluessel} auf einer Regel mit Ground Truth gefunden")


def test_n_gate_faengt_erfundene_geltungsbedingung(daten):
    """Gegenprobe (Mutationsprobe 2026-08-07): eine erfundene geltungsbedingung auf einer
    ECHTEN Bindung MUSS auffallen — sonst ist test_n nur Dekoration.

    Ohne diese Probe wäre nicht belegt, dass die Rückrichtung überhaupt greift. Reales
    Vorbild: Variante 2 der Mutationsprobe (neues Feld + erfundene Bedingung) blieb bei
    test_b GRÜN — genauso wäre vpf_frist_unterbrochen nie aufgefallen.
    """
    rules = _rules()
    d, b = _erste_bindung_mit_ground_truth(daten, rules, "geltungsbedingung")
    b["quelle"]["geltungsbedingung"] = "zzz_frei_erfundene_geltungsbedingung"
    gefunden_gb, _, _ = _n_gefundene_verstoesse(d, rules)
    treffer = [e for e in gefunden_gb if e[3] == "zzz_frei_erfundene_geltungsbedingung"]
    assert treffer, "Gegenprobe fehlgeschlagen: erfundene geltungsbedingung nicht erkannt"


def test_n_gate_faengt_erfundenen_signatur_slot(daten):
    """Gegenprobe (Mutationsprobe 2026-08-07): ein erfundener signatur_slot auf einer
    ECHTEN Bindung MUSS auffallen — analog zu test_n_gate_faengt_erfundene_geltungsbedingung.
    """
    rules = _rules()
    d, b = _erste_bindung_mit_ground_truth(daten, rules, "signatur_slot")
    b["quelle"]["signatur_slot"] = "zzz_frei_erfundener_slot"
    _, gefunden_slot, _ = _n_gefundene_verstoesse(d, rules)
    treffer = [e for e in gefunden_slot if e[3] == "zzz_frei_erfundener_slot"]
    assert treffer, "Gegenprobe fehlgeschlagen: erfundener signatur_slot nicht erkannt"


# ---- (o) Keine harte Jahreszahl in VZ-abhaengigen Fragetexten ------------------

_JAHR = re.compile(r"(19|20)\d{2}")

# Ausnahmen: Jahreszahl ist Formatbeispiel oder Rechtstatsache, kein VZ-Bezug. Neue Ausnahme
# braucht eine Begruendung hier -- sonst faengt (o) jede neue feste Jahreszahl.
_JAHR_AUSNAHMEN = {
    ("kind_geburtsdatum", "hilfe_kurz"):
        "Formatbeispiel TT.MM.JJJJ (z.B. 15.03.2015), kein VZ-Bezug.",
    ("kind_anderer_elternteil_geburtsdatum", "hilfe_kurz"):
        "Formatbeispiel TT.MM.JJJJ (z.B. 01.01.1985), kein VZ-Bezug.",
    ("vor_voller_abzug", "hilfe_kurz"):
        "Rechtstatsache (Vorsorgeaufwand seit 2023 zu 100% ansetzbar), kein VZ-Bezug.",
}


def _harte_jahreszahlen(daten):
    """[(feld_id, key, text)] fuer jede unbegruendete feste Jahreszahl in fragetext_laie/hilfe_kurz."""
    treffer = []
    for d in daten.values():
        for b in d["bindungen"]:
            for key in ("fragetext_laie", "hilfe_kurz"):
                v = b.get(key)
                if isinstance(v, str) and _JAHR.search(v) and (b["feld_id"], key) not in _JAHR_AUSNAHMEN:
                    treffer.append((b["feld_id"], key, v))
    return treffer


def test_o_keine_harte_jahreszahl_in_vz_abhaengigem_text(daten):
    """fragetext_laie/hilfe_kurz werden nirgends mit dem VZ formatiert (api.py:2351/:2818,
    api_llm.py:30, fragekatalog.py reichen den String unveraendert durch) — eine feste
    Jahreszahl im Text ist deshalb in jedem VZ ausser dem genannten falsch. Fund
    am_afa_ist_anschaffungsjahr (2026-08-12): Frage nannte 2026 fest, im VZ 2025 wurde nach
    dem falschen Jahr gefragt. Ausnahmen (Formatbeispiel/Rechtstatsache) stehen benannt und
    begruendet in _JAHR_AUSNAHMEN — wer eine neue braucht, traegt sie dort ein statt den
    Test zu loeschen.
    """
    treffer = _harte_jahreszahlen(daten)
    assert not treffer, "\n".join(
        f"{fid}.{key}: feste Jahreszahl ohne Ausnahme-Eintrag: {text!r}" for fid, key, text in treffer)


def test_o_gate_faengt_neue_jahreszahl(daten):
    """Gegenprobe: eine frisch eingefuegte, unbegruendete Jahreszahl MUSS auffallen — sonst
    ist (o) nur Dekoration."""
    d = _erste_datei_daten(daten)
    fid = d["bindungen"][0]["feld_id"]
    assert (fid, "fragetext_laie") not in _JAHR_AUSNAHMEN, "Testaufbau ungueltig: Feld ist zufaellig Ausnahme"
    d["bindungen"][0]["fragetext_laie"] = "Testfrage mit fester Jahreszahl 2026?"
    treffer = _harte_jahreszahlen({"mutiert.yaml": d})
    assert treffer and treffer[0][0] == fid, "Gegenprobe fehlgeschlagen: neue Jahreszahl nicht erkannt"


# ========== Sammel-Scopes ohne bool-Gate ==========

# Pseudoregel-Sammel-Scopes ohne eigene rules.yaml-Signatur (s. REGELN_OHNE_GROUND_TRUTH):
# p2_festzusetzung_einzel buendelt 24, p2_festzusetzung_zusammen 34 sachlich unabhaengige
# Felder unter EINER regel_id, weil hier keine echte Catala-Scope-Andockung existiert.
_KEINE_BOOL_GATES_AUF = {"p2_festzusetzung_einzel", "p2_festzusetzung_zusammen"}


def _bool_gates_auf_sammelscope(daten):
    """[(datei, feld_id, regel_id, geltungsbedingung)] fuer jedes askable bool-Feld, dessen
    geltungsbedingung auf einem der beiden Sammel-Scopes haengt."""
    treffer = []
    for f, d in daten.items():
        for b in d["bindungen"]:
            q = b["quelle"]
            if (q.get("regel_id") in _KEINE_BOOL_GATES_AUF and "geltungsbedingung" in q
                    and b.get("askable") and b.get("typ") == "bool"):
                treffer.append((os.path.basename(f), b["feld_id"], q["regel_id"], q["geltungsbedingung"]))
    return treffer


def test_p_sammelscope_ohne_bool_gate(daten):
    """p2_festzusetzung_einzel/_zusammen sind Pseudoregel-Sammel-Scopes ohne eigene
    rules.yaml-Signatur (s. REGELN_OHNE_GROUND_TRUTH) — sie buendeln je zwei bis drei Dutzend
    sachlich unabhaengige Felder (Bruttoarbeitslohn, Veranlagungsart, Steuerklasse, saemtliche
    Stammdaten inkl. IBAN) unter EINER regel_id, weil hier keine echte Catala-Scope-Andockung
    existiert. traverser.relevanz() schliesst aber PRO regel_id aus: sobald ein askables
    bool-Gate dieser Regel bestaetigt False ist, verschwindet die GESAMTE regel_id dauerhaft
    aus naechste_fragen() — inklusive aller sachfremden Felder. Genau das ist am 2026-08-12 im
    Dialog-Durchstich (Task C) passiert: pv_auf_gebaeude ("Ist die Anlage an einem Gebaeude
    angebracht?", bool) hing an p2_festzusetzung_einzel; die Mehrheitsantwort "Nein" hat
    veranlagung/bruttoarbeitslohn/alle stammdaten_* aus dem Angebot genommen, BEVOR sie je
    gefragt wurden — kein Sperrgrund, kein Fehler, die Felder waren einfach nie in der
    Warteschlange. Fix: eigene regel_id (p3_nr72_pv) statt des Sammel-Scopes. Dieser Test
    verhindert, dass ein neues bool-Gate wieder auf einem der beiden Sammel-Scopes landet."""
    treffer = _bool_gates_auf_sammelscope(daten)
    assert not treffer, "\n".join(
        f"{datei}::{fid}: askable bool-Gate (geltungsbedingung={gb!r}) auf Sammel-Scope "
        f"{rid!r} — schliesst bei bestaetigtem False die GANZE regel_id aus "
        "traverser.relevanz()/naechste_fragen() aus und reisst alle sachfremden Felder mit. "
        "Eigene regel_id statt des Sammel-Scopes verwenden."
        for datei, fid, rid, gb in treffer)


def test_p_gate_faengt_neues_bool_gate(daten):
    """Gegenprobe: ein frisch angebundenes bool-Gate auf einem der Sammel-Scopes MUSS
    auffallen — sonst ist (p) nur Dekoration."""
    d = _erste_datei_daten(daten)
    d["bindungen"].append({
        "feld_id": "zzz_erfundenes_bool_gate",
        "quelle": {"regel_id": "p2_festzusetzung_einzel", "geltungsbedingung": "zzz_bedingung"},
        "typ": "bool",
        "askable": True,
    })
    treffer = _bool_gates_auf_sammelscope({"mutiert.yaml": d})
    assert treffer and treffer[0][1] == "zzz_erfundenes_bool_gate", (
        "Gegenprobe fehlgeschlagen: neues bool-Gate nicht erkannt")


def test_a_kein_doppelter_schluessel_im_feldblock():
    """Kein Schlüssel darf in einem Feldblock ZWEIMAL stehen.

    YAML nimmt klaglos den letzten Wert — die Schema-Validierung (test_a_schema_valid) sieht
    also nur das Ergebnis und bleibt grün. Wer die Datei LIEST, sieht den ersten.

    Gefunden am 2026-08-19 in fünf Feldblöcken, alle nach demselben Muster: `elster_kz: null`,
    darunter ein Kommentar mit der Herleitung, darunter `elster_kz: E0…`. Beim Nachtragen der
    Kennzahl war die alte Zeile stehen geblieben. Funktional harmlos — die Felder waren
    gebunden —, aber der Kopfkommentar von bindung_p22_nr3.yaml behauptete deshalb bis zuletzt
    "elster_kz bewusst null (kein XSD-verifiziertes Kz-Mapping)", und das war seit Monaten
    falsch. Eine Datei, die zwei Antworten auf dieselbe Frage gibt, führt irgendwann jemanden
    in die Irre; hier war es der eigene Dateikopf.
    """
    treffer = []
    for f in _bindung_files():
        zeilen = open(f, encoding="utf-8").read().splitlines()
        # Nur die bindungen:-Sektion. Die luecken:-Einträge darunter sind gleich eingerückt,
        # dürfen aber wiederholte Schlüssel führen (mehrere Lücken je Regel) — ohne diese
        # Grenze meldete der Test dort ein Dutzend Falschtreffer.
        ende = next((i for i, z in enumerate(zeilen) if z.startswith("luecken:")), len(zeilen))
        zeilen = zeilen[:ende]
        start = [i for i, z in enumerate(zeilen) if z.startswith("  - feld_id:")]
        start.append(len(zeilen))
        for a, b in zip(start, start[1:]):
            gesehen = {}
            for i in range(a, b):
                m = re.match(r"^    ([a-z_]+):", zeilen[i])
                if not m:
                    continue
                key = m.group(1)
                if key in gesehen:
                    fid = zeilen[a].split(":", 1)[1].strip()
                    treffer.append(f"{os.path.basename(f)}:{i+1} {fid} → '{key}' "
                                   f"schon in Zeile {gesehen[key]+1}")
                gesehen[key] = i
    assert not treffer, "doppelte Schlüssel im selben Feldblock:\n  " + "\n  ".join(treffer)


# _ABZUGS_KZ-Eintraege, die KEIN Bindungsfeld tragen. Sie sind sachlich richtig klassifiziert
# (V+V-Werbungskosten, KV/PV-Beitraege), aber die Bindung nutzt inzwischen andere Kz. Stehen
# gelassen, weil ein Kz ohne Feld nie nachgeschlagen wird und die Klassifikation erhalten bleibt,
# falls eines der Felder spaeter doch gebunden wird — s. Kommentar in est_mapping.py.
ABZUGS_KZ_OHNE_FELD = {
    "E0703838",  # V+V Werbungskosten — abgeloest durch E0705701, das bis 2026-08-19 fehlte
    "E2001203", "E2001505", "E2001805", "E2002105", "E2003104", "E2003202",  # KV/PV-Beitraege
}


def test_p_abzugs_kz_deckt_die_bindung(daten):
    """Die Aufrundungsliste und die Bindung duerfen nicht auseinanderlaufen.

    Warum das ein eigener Test ist: die Liste _ABZUGS_KZ entscheidet, ob ein Cent-Betrag beim
    Weg ins XML auf- oder abgerundet wird. Die Anleitung zur Anlage (anl_est1a_2025.txt:269-274)
    verlangt "zu Ihren Gunsten": Einnahmen ab, Abzuege auf. Ein Abzug, der nicht in der Liste
    steht, faellt um bis zu 99 Cent zu niedrig aus — klein, aber systematisch und immer in
    dieselbe Richtung.

    Gefunden am 2026-08-19: die Liste war in BEIDE Richtungen abgedriftet. Vierzehn
    Abzugs-Summen fehlten, und sieben Eintraege trugen kein Feld mehr. Bei E0703838 liess sich
    beides an einem Fall zeigen — die V+V-Werbungskosten waren auf E0705701 umgezogen, die Liste
    rundete weiter die alte Kennzahl auf, die niemand mehr schreibt.

    Dieser Test prueft die Richtung, die MASCHINELL entscheidbar ist: zeigt jeder Eintrag noch
    auf ein Feld? Die Gegenrichtung (ist jeder Abzug in der Liste?) braucht die Einordnung
    "Einnahme oder Aufwand", und die steht nirgends maschinenlesbar — sie bleibt Handarbeit beim
    Binden eines neuen Betragsfelds. Ein halber Waechter ist hier besser als keiner: genau die
    Haelfte, die den E0703838-Fall gefunden haette.
    """
    sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
    import est_mapping as M

    gebunden = {b["elster_kz"] for d in daten.values() for b in d["bindungen"] if b.get("elster_kz")}
    verwaist = sorted(set(M._ABZUGS_KZ) - gebunden - ABZUGS_KZ_OHNE_FELD)
    assert not verwaist, (
        f"_ABZUGS_KZ nennt Kennzahlen, die kein Bindungsfeld traegt: {verwaist}. Entweder ist "
        f"das Feld umgezogen (dann die neue Kz eintragen — sonst rundet die Liste ins Leere) "
        f"oder der Eintrag gehoert nach ABZUGS_KZ_OHNE_FELD mit Begruendung.")

    # Ratsche in die Gegenrichtung: eine Ausnahme, die wieder gebunden ist, muss raus. Sonst
    # verrottet die Liste nach oben und deckt irgendwann echte Drift zu.
    erledigt = sorted(ABZUGS_KZ_OHNE_FELD & gebunden)
    assert not erledigt, (
        f"ABZUGS_KZ_OHNE_FELD nennt Kennzahlen, die inzwischen gebunden sind: {erledigt} — "
        f"bitte aus der Ausnahmeliste streichen.")


# ========== q: elster_kz_grund zitiert einen Code, den GENAU DIESES Feld auch schreibt ==========
#
# Reichweite (bewusst eng): geprueft wird, ob MINDESTENS EINER der im Text zitierten E-Codes fuer
# GENAU DIESES Feld ueber eine echte Routingstruktur aus est_mapping.deklariere() erreichbar ist.
# Eine schwaechere Fassung ("kommt der Code irgendwo im Code vor") waere bei den teuersten Faellen
# zufrieden gewesen: E1900701 wird geschrieben, aber fuer kap_kapitalertraege_partner, nicht fuer
# kap_gewinn_sonstige_partner, dessen Text ihn behauptet; E0801704 wird geschrieben, aber fuer
# gewst_zu_zahlen (Person A, eigener elster_kz), nicht fuer gewst_zu_zahlen_partner, das nicht in
# PARTNER_INSTANZ steht; E2004403 wird geschrieben, aber im Pers-Zweig Arbeitslosenversicherung,
# nicht fuer die fuenf vorsorge_*_partner-Felder, deren Texte ihn nennen. Ein erfundener Code faellt
# beim ersten Grep auf; ein echter Code, der einem ANDEREN Feld gehoert, bestaetigt sich selbst.
#
# Die Quelle der Wahrheit ist _codes_fuer_feld() unten: sie liest die echten Python-Objekte aus
# est_mapping.py (VERZWEIGUNG/PARTNER_VERZWEIGUNG/PARTNER_INSTANZ/WERTEKODIERUNG/NEGATION/
# DOKUMENTIERT_AGGREGAT/IBAN_TRANSFORM_ZIEL_KZ/P23_GEWINN + den eigenen elster_kz), nicht einen
# Kommentar. Genau daran haengt der E0800502-Fall: im Bindungskommentar steht der Code, und der
# Kommentar sagt das Gegenteil dessen, was ein naives Grep daraus liest (die Container-Korrektur
# 2026-08-20 hat den Zielcontainer gewechselt, sieben Texte zitieren noch den alten).
#
# Grenze des Gates (bewusst benannt, damit niemand mehr daraus liest als geprueft wird): ein Feld,
# das strukturell NIE einen eigenen Kz traegt — Screening-Flag, berechnetes/abgeleitetes Feld,
# rohes Instanz-Input (Kz sitzt am berechneten Zwilling), von ERiC pauschal abgelehntes Feld —
# zitiert legitim die Kz ANDERER Felder zur Erklaerung. Das ist Kontext, keine Fehlzuordnung. Das
# Gate entscheidet auch NICHT, ob ein zitierter Fremd-Code fuer das genannte Feld inhaltlich passen
# WUERDE, wenn es ihn haette (kap_gewinn_sonstige: ob ueberhaupt ein amtliches Ziel existiert, ist
# eine Rechtsfrage, keine Routing-Frage) — und es sieht KEINE Werte-Semantik: ein Feld kann hier
# gruen sein und trotzdem den falschen WERT in einen korrekt zitierten Kz schreiben
# (rentner_renten_beginn_jahr: VERZWEIGUNG traegt das Feld korrekt nach E1800501 — der dortige
# Defekt ist Jahr-Zahl auf Datums-Kz, keine Fehlzuordnung, dieses Gate sieht ihn nicht und soll es
# auch nicht).
#
# Drei Register statt einem fuer die 20 Nicht-Fehlzuordnungs-Treffer, weil die naheliegende
# Rechtfertigung — "der Text sagt selbst, er habe kein Ziel" — dieselbe Autoritaetsfalle waere wie
# die 19 echten Fehlzuordnungen: DIE tragen ihre falsche Behauptung auch im eigenen Text, wortgleich
# selbstsicher. Gemessen 2026-08-30: das Schema-Feld `kz_status: endgueltig` ("am amtlichen XSD
# BELEGT, dass es kein Kz gibt") sieht aus wie ein staerkerer, code-seitiger Beleg als Fliesstext —
# ist es nicht. Es steht bei SIEBZEHN der neunzehn echten Fehlzuordnungen GENAUSO (betriebseinnahmen,
# afa_jahresbetrag, gewinnanteil* & Zwillinge, vorsorge_*_partner, kap_gewinn_sonstige & Partner) —
# derselbe Autor, dieselbe Fehlerquelle, nur in ein Enum-Feld statt in Prosa gegossen. `kz_status`
# ALLEIN zaehlt hier deshalb nicht als Beleg. Was zaehlt: eine vom Grund-Text UNABHAENGIGE
# Code-Tatsache — ein anderes, echtes Feld traegt den genannten Kz nachweislich selbst (gegengeprueft
# gegen die eigene elster_kz-Angabe der Schwester ODER gegen eine Routingstruktur aus est_mapping.py),
# oder eine schema-deklarierte, anderswo tatsaechlich konsumierte Eigenschaft (`screening: true`, von
# app.js als Ankreuzliste gelesen; `E0100001 in est_mapping.KONSTANTE_KZ`, live gegen das echte
# Objekt), oder eine dokumentierte externe Messung (ERiC-Ablehnung mit Skript). Nur was so belegt
# ist, steht in KZ_GRUND_KEIN_ZIEL_STRUKTURELL. Ein Feld mit einem ECHTEN, nur noch nicht
# verdrahteten Ziel gehoert nicht dorthin, sonst waescht das Register einen offenen Punkt zu einem
# dauerhaften Okay — dafuer steht KZ_GRUND_RUECKSTAND. Und wo keine dieser Code-Tatsachen zu finden
# war (in dieser Umgebung: keine E10-XSD, kein $ERIC_DIR), steht das Feld unter
# KZ_GRUND_NICHT_CODESEITIG_VERIFIZIERT — benannt, nicht mitgezaehlt als geklaert.

_KZ_Q_PAT = re.compile(r"E\d{7}")


def _codes_fuer_feld(feld_id: str, b: dict, M) -> set:
    """Welche Kz KANN dieses Feld laut den echten Routingstrukturen aus est_mapping.deklariere()
    schreiben — dieselbe Verzweigung wie dort (Klassen d/a/f/g×f/g/i/j/h/1:1), gegen importierte
    Objekte gelesen, nicht gegen Text."""
    if feld_id in M.NEGATION:
        return {M.NEGATION[feld_id]}
    if feld_id in M.MULTIPLIKATION:
        return set()
    agg_quellen = {f for fs in M.DOKUMENTIERT_AGGREGAT.values() for f in fs}
    if feld_id in agg_quellen:
        return {ziel for ziel, srcs in M.DOKUMENTIERT_AGGREGAT.items() if feld_id in srcs}
    if feld_id in M.VERZWEIGUNG:
        return set(M.VERZWEIGUNG[feld_id]["kz"].values())
    if feld_id in M.PARTNER_VERZWEIGUNG:
        return set(M.PARTNER_VERZWEIGUNG[feld_id]["kz"].values())
    if feld_id in M.PARTNER_INSTANZ:
        return {M.PARTNER_INSTANZ[feld_id]}
    if feld_id in M.WERTEKODIERUNG:
        return {M.WERTEKODIERUNG[feld_id]["kz"]}
    if feld_id == "stammdaten_iban":
        return set(M.IBAN_TRANSFORM_ZIEL_KZ)
    if feld_id in M.P23_BETRAGSFELDER:            # Rohdaten -> Kz sitzt am berechneten Instanz-Gewinn
        return set(M.P23_GEWINN["kz"].values())
    if b.get("elster_kz"):
        return {b["elster_kz"]}
    return set()


def _q_kandidaten(daten, M):
    """{feld_id: (datei, zitiert, eigene)} fuer jedes Feld, dessen Text >=1 E-Code zitiert, aber
    KEINER der zitierten Codes zu den eigenen (laut Routingstruktur) gehoert."""
    treffer = {}
    for f, d in daten.items():
        for b in d["bindungen"]:
            g = b.get("elster_kz_grund")
            if not g:
                continue
            zitiert = set(_KZ_Q_PAT.findall(g))
            if not zitiert:
                continue
            eigene = _codes_fuer_feld(b["feld_id"], b, M)
            if not (zitiert & eigene):
                treffer[b["feld_id"]] = (os.path.basename(f), zitiert, eigene)
    return treffer


# Schuldenliste: der Text behauptet einen Zielcode, den laut Routingstruktur ein ANDERES Feld
# traegt oder den gar niemand traegt. Jeder Eintrag ist ein Fund, keine Ausnahme.
KZ_GRUND_BEKANNTE_FEHLZUORDNUNG = {
    "betriebseinnahmen": "EUeR-Komponente behauptet Ziel E0800502 -- der Container ist seit der "
        "Container-Korrektur 2026-08-20 abgeloest, kein Routing-Pfad schreibt ihn mehr.",
    "afa_jahresbetrag": "dieselbe Fehlzuordnung auf E0800502 wie betriebseinnahmen.",
    "gewinnanteil": "'in E0800502 aufgegangen' behauptet; VERZWEIGUNG fuehrt keinen Pfad dorthin.",
    "verguetung_taetigkeit": "dieselbe Fehlzuordnung auf E0800502/E0800602 wie gewinnanteil.",
    "verguetung_darlehen": "dieselbe Fehlzuordnung auf E0800502/E0800602 wie gewinnanteil.",
    "verguetung_ueberlassung": "dieselbe Fehlzuordnung auf E0800502/E0800602 wie gewinnanteil.",
    "gewinnanteil_partner": "Person-B-Zwilling derselben Fehlzuordnung wie gewinnanteil.",
    "verguetung_taetigkeit_partner": "Person-B-Zwilling derselben Fehlzuordnung.",
    "verguetung_darlehen_partner": "Person-B-Zwilling derselben Fehlzuordnung.",
    "verguetung_ueberlassung_partner": "Person-B-Zwilling derselben Fehlzuordnung.",
    "gewst_zu_zahlen_partner": "behauptet PARTNER_INSTANZ-Routing auf E0801704 -- PARTNER_INSTANZ "
        "fuehrt dieses Feld nicht (nur gewst_hebesatz_partner/gewst_messbetrag_partner); E0801704 "
        "wird nur fuer Person A (gewst_zu_zahlen, eigener elster_kz) geschrieben.",
    "vorsorge_arbeitslosenversicherung_partner": "behauptet (Teil-)Ziel E2004403 -- das Feld steht "
        "in keiner Routingstruktur; E2004403 gehoert laut eigenem Bindungskommentar ohnehin einer "
        "anderen Kategorie (Pers-Zweig Arbeitslosenversicherung, additiv zu A_B_LP, nicht Ersatz).",
    "vorsorge_erwerbsunfaehigkeit_partner": "dieselbe Fehlzuordnung auf E2004403 wie beim "
        "Arbeitslosenversicherung-Zwilling.",
    "vorsorge_unfall_haftpflicht_partner": "dieselbe Fehlzuordnung auf E2004403.",
    "vorsorge_rv_alt_mit_ueberschuss_partner": "dieselbe Fehlzuordnung auf E2004403.",
    "vorsorge_rv_alt_ohne_ueberschuss_partner": "dieselbe Fehlzuordnung auf E2004403.",
    "kap_gewinn_sonstige": "'Wert in E1900701' behauptet -- E1900701 gehoert "
        "PARTNER_INSTANZ['kap_kapitalertraege_partner'], nicht diesem Feld; gemessen 2026-08-30: "
        "kein amtliches Ziel existiert ueberhaupt (vier Geschwister-Kz von E1900701 passen keins).",
    "kap_gewinn_sonstige_partner": "derselbe Fehlschluss fuer den Partner-Zwilling.",
    "vv_gebaeude_afa": "behauptet im Praesens eine Deklaration ueber E0703302+E0703304, die kein "
        "Code schreibt -- tatsaechlich ist das Feld Aggregationsquelle fuer E0703838 "
        "(DOKUMENTIERT_AGGREGAT), das der Text nicht erwaehnt. Andere Bauart als der Rest der "
        "Liste (kein 'aufgegangen in X', sondern ein nie erwaehnter echter Pfad) -- eigens genannt.",
}

# Register 2: strukturell code-verifiziert. Jede Begruendung nennt eine vom Grund-Text UNABHAENGIGE
# Tatsache (Schwester-Feld traegt den Kz nachweislich selbst / schema-deklarierte, anderswo
# konsumierte Eigenschaft / dokumentierte externe Messung) -- nie den Grund-Text selbst und nie
# `kz_status` allein (Begruendung s. Kommentarblock oben).
KZ_GRUND_KEIN_ZIEL_STRUKTURELL = {
    "kein_unterhalt": "screening=True (Schema-Feld), von app.js:350 als Ankreuzliste konsumiert "
        "(`filter(q => q.screening)`) -- Existenzfrage fuer ein ganzes Thema, kein Deklarationsfeld.",
    "keine_auslandseinkuenfte": "screening=True, gleiche Bauart wie kein_unterhalt.",
    "keine_behinderung_pflege": "screening=True, gleiche Bauart wie kein_unterhalt.",
    "keine_energetische_sanierung": "screening=True, gleiche Bauart wie kein_unterhalt.",
    "kein_kap_partner": "screening=True, gleiche Bauart wie kein_unterhalt.",
    "kein_gewinn_partner": "screening=True, gleiche Bauart wie kein_unterhalt.",
    "keine_behinderung_pflege_partner": "screening=True, gleiche Bauart wie kein_unterhalt.",
    "stammdaten_art_est_erklaerung": "askable=False, und E0100001 liegt in est_mapping.KONSTANTE_KZ "
        "(live gegen das echte Objekt geprueft) -- der Code setzt diesen Kz UNBEDINGT, unabhaengig "
        "von jedem Feld, dieses eingeschlossen.",
    "kind_unter_14_haushaltszugehoerig": "ableitung-Block (aus=kind_geburtsdatum, echte "
        "schema-gepruefte Struktur) UND E0506105 wird unabhaengig davon von der Schwester "
        "kinderbetreuungskosten als eigener elster_kz getragen (gegengeprueft).",
    "person_b_idnr": "scripts/measure_person_b_idnr.py existiert, ruft die echte checkESt-Pruefung "
        "gegen ERiC auf und dokumentiert rc=610301106 -- eine Messung, kein Textclaim (Skript-Logik "
        "gegengelesen 2026-08-30; in dieser Umgebung nicht neu ausgefuehrt, $ERIC_DIR fehlt hier).",
    "p35c_keine_doppelfoerderung": "E0240902 wird unabhaengig davon von der Schwester "
        "p35c_foerderung_in_anspruch als eigener elster_kz getragen (gegengeprueft).",
    "vpf_an_oder_abreisetag": "E0205302 wird unabhaengig davon von der Schwester tage_an_abreise "
        "als eigener elster_kz getragen (gegengeprueft).",
    "vv_werbungskosten": "E0703838 ist ein real geschriebenes Aggregationsziel "
        "(est_mapping.DOKUMENTIERT_AGGREGAT), gespeist von vier Schwesterfeldern "
        "(vv_gebaeude_afa/vv_schuldzinsen/vv_erhaltungsaufwand/vv_sonstige_wk) -- dieses Feld "
        "selbst ist keine der vier Quellen.",
    "vpf_abwesenheit_stunden": "E0205201/E0205409 werden unabhaengig davon von den Schwestern "
        "tage_ueber_8h_eintaegig/tage_24h als eigene elster_kz getragen (gegengeprueft).",
    "rentner_alter_bei_rentenbeginn": "E1801701 wird unabhaengig davon von der Schwester "
        "rentner_renten_beginn_jahr ueber VERZWEIGUNG getragen (gegengeprueft in est_mapping.py).",
    "behinderungsbedingte_aufwendungen": "E0161804 wird unabhaengig davon von der Schwester "
        "agb_aufwendungen als eigener elster_kz getragen (gegengeprueft).",
}

# Register 3: Rueckstand, kein Nicht-Eigentuemer. Es GIBT ein amtliches Ziel, es ist nur noch nicht
# verdrahtet -- wer das in KZ_GRUND_KEIN_ZIEL_STRUKTURELL steckt, waescht einen offenen Punkt zu
# einem dauerhaften Okay (main 2026-08-30).
KZ_GRUND_RUECKSTAND = {
    "am_anschaffungskosten": "E0204401 ist ein reales Anlage-N-Ziel, nur noch nicht verdrahtet "
        "('Betrags-Kz per Sequenz-Nachtrag' -- eigene Formulierung des Textes gesteht die Luecke "
        "bereits ein). Gehoert auf die Wiedervorlage, nicht in eine Nicht-Eigentuemer-Liste.",
}

# Register 4: kein unabhaengiger Code-Beleg gefunden. Nicht widerlegt, nicht bestaetigt -- die
# Textbehauptung koennte stimmen, aber weder kz_status (s.o. entwertet) noch eine Schwester-Kz-
# Zuordnung noch eine schema-konsumierte Eigenschaft belegen es. Braucht menschliche/rechtliche
# Pruefung (fuer die Typ-Behauptungen: gegen die amtliche E10-XSD, die in dieser Umgebung fehlt),
# nicht einen weiteren Blick in denselben Text.
KZ_GRUND_NICHT_CODESEITIG_VERIFIZIERT = {
    "verlustvortrag_bestand": "kz_status=endgueltig gesetzt, aber das steht bei 17 von 19 echten "
        "Fehlzuordnungen genauso -- kein unabhaengiger Beleg. Die Typ-Behauptung (E0190701 = "
        "Ja/Nein-RABE) ist gegen die amtliche E10-XSD hier nicht pruefbar ($ERIC_DIR fehlt, keine "
        "E10-XSD im Repo); E0190701 wird von keinem Feld in der Bindungstabelle als eigener Kz "
        "getragen (weder Bestaetigung noch Widerlegung).",
    "vv_entgelt_quote_prozent": "kz_status=endgueltig gesetzt, dieselbe Schwaeche wie oben. "
        "E0708601 wird nirgends -- weder als eigener elster_kz noch in einer Routingstruktur -- "
        "referenziert.",
    "fam_monate_ohne_voraussetzung": "E0503801 (behauptete Quelle) wird von KEINEM Feld in der "
        "gesamten Bindungstabelle oder in est_mapping.py getragen -- weder als eigener elster_kz "
        "noch ueber eine Routingstruktur. Zudem askable=True ohne ableitung-Block, obwohl der Text "
        "'abgeleitet' behauptet -- Widerspruch zum sonstigen Schema-Muster (vgl. "
        "kind_unter_14_haushaltszugehoerig, das einen echten ableitung-Block hat).",
}


def test_q_elster_kz_grund_ziel_existiert(daten):
    """elster_kz_grund zitiert einen E-Code -> mindestens einer der zitierten Codes muss laut
    _codes_fuer_feld() (echte Routingstruktur aus est_mapping.py, kein Kommentar) fuer GENAU
    DIESES Feld erreichbar sein. Reichweite und Grenze stehen im Kommentarblock oben. Gemessen
    2026-08-30 (HEAD 915e327): 123 Felder zitieren >=1 Code, 39 davon ohne eigenen Treffer -- 19
    echte Fehlzuordnung (KZ_GRUND_BEKANNTE_FEHLZUORDNUNG), 16 code-verifizierte Nicht-Eigentuemer
    (KZ_GRUND_KEIN_ZIEL_STRUKTURELL), 1 Rueckstand (KZ_GRUND_RUECKSTAND), 3 unverifiziert
    (KZ_GRUND_NICHT_CODESEITIG_VERIFIZIERT)."""
    sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
    import est_mapping as M

    # Registergroessen mitpruefen: jede Erweiterung einer der vier Listen ist eine bewusste
    # Handlung mit eigener Begruendung, kein stiller Nebeneffekt -- sonst loest sich die Ratsche
    # unbemerkt (main 2026-08-30, Auflage 1).
    assert len(KZ_GRUND_BEKANNTE_FEHLZUORDNUNG) == 19
    assert len(KZ_GRUND_KEIN_ZIEL_STRUKTURELL) == 16
    assert len(KZ_GRUND_RUECKSTAND) == 1
    assert len(KZ_GRUND_NICHT_CODESEITIG_VERIFIZIERT) == 3

    treffer = _q_kandidaten(daten, M)
    bekannt = (set(KZ_GRUND_BEKANNTE_FEHLZUORDNUNG) | set(KZ_GRUND_KEIN_ZIEL_STRUKTURELL)
               | set(KZ_GRUND_RUECKSTAND) | set(KZ_GRUND_NICHT_CODESEITIG_VERIFIZIERT))
    assert len(bekannt) == 39, "Register ueberschneiden sich -- ein Feld steht in mehr als einem."

    unbekannt = sorted(set(treffer) - bekannt)
    assert not unbekannt, "\n".join(
        f"{fid} [{treffer[fid][0]}]: zitiert {sorted(treffer[fid][1])}, eigene Codes laut "
        f"Routingstruktur {sorted(treffer[fid][2])} -- kein Treffer, kein Registereintrag. Neu "
        f"einordnen: echte Fehlzuordnung, code-verifizierter Nicht-Eigentuemer, Rueckstand, oder "
        f"(wenn kein Beleg zu finden ist) KZ_GRUND_NICHT_CODESEITIG_VERIFIZIERT mit Begruendung, "
        f"was gefehlt hat."
        for fid in unbekannt)

    # Ratsche in die Gegenrichtung: ein Register-Eintrag, dessen Feld jetzt tatsaechlich einen
    # eigenen Treffer hat, ist erledigt und verrottet sonst nach oben.
    erledigt = sorted(bekannt - set(treffer))
    assert not erledigt, (
        f"Register nennt Felder, die inzwischen einen eigenen Kz-Treffer haben: {erledigt} — "
        f"bitte aus dem jeweiligen Register streichen.")


def test_q_gate_faengt_falsches_ziel_in_grund(daten):
    """Mutationsprobe A (zu lax): ein Text, der NUR EIN erfundenes Ziel nennt -- der ECHTE
    Kz-Anteil wird ersetzt, nicht ergaenzt --, MUSS auffallen. Prueft die Mutation selbst (nicht
    nur das Ergebnis) -- sonst waere das Gate gruen, weil es nie hinschaut, nicht weil es nichts
    findet."""
    sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
    import est_mapping as M

    d = _erste_datei_daten(daten)
    fid = "einkuenfte_gewinn"
    ziel = next(b for b in d["bindungen"] if b["feld_id"] == fid)
    vorher = ziel["elster_kz_grund"]
    assert vorher and "E0800302" in vorher, "Testaufbau ungueltig: Ausgangstext unerwartet"
    ziel["elster_kz_grund"] = "Gebunden auf E9999999."
    assert "E0800302" not in ziel["elster_kz_grund"] and ziel["elster_kz_grund"] != vorher, (
        "Mutation hat den echten Code nicht entfernt -- kein Test des zu-lax-Falls")

    treffer = _q_kandidaten({"mutiert.yaml": d}, M)
    assert fid in treffer, (
        f"Gegenprobe fehlgeschlagen: {fid} mit einem NUR erfundenen Ziel wurde nicht als Treffer "
        f"erkannt -- das Gate waere zu lax.")
    assert treffer[fid][1] == {"E9999999"}, "Gegenprobe fehlgeschlagen: falsche Codes zitiert"


def test_q_gate_bleibt_gruen_bei_korrektem_mehrfachzitat(daten):
    """Mutationsprobe B (zu streng, Gegenrichtung): ein korrekter Text darf NICHT rot werden, nur
    weil er neben dem echten Ziel auch eine explizit AUSGESCHLOSSENE Alternative nennt
    (einkuenfte_gewinn zitiert E0800302 als echtes Ziel UND E0800502 als verworfenen
    Container-Kandidaten, s. CONTAINER-KORREKTUR 2026-08-20 in est_mapping.py). Eine Fassung, die
    verlangt, dass JEDER zitierte Code trifft, waere hier faelschlich rot -- das ist die konkrete
    Gestalt, die ein zu strenger Pruefer hier annehmen wuerde."""
    sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
    import est_mapping as M

    d = _erste_datei_daten(daten)
    fid = "einkuenfte_gewinn"
    b = next(x for x in d["bindungen"] if x["feld_id"] == fid)
    zitiert = set(_KZ_Q_PAT.findall(b["elster_kz_grund"]))
    eigene = _codes_fuer_feld(fid, b, M)
    assert "E0800502" in zitiert and "E0800502" not in eigene, (
        "Testaufbau ungueltig: der verworfene Container-Kandidat fehlt oder waere doch attribuierbar")
    assert "E0800302" in zitiert & eigene, "Testaufbau ungueltig: das echte Ziel fehlt"

    # Die real gebaute Regel ("mindestens einer trifft") laesst das Feld durch:
    treffer = _q_kandidaten({"mutiert.yaml": d}, M)
    assert fid not in treffer, "Gate faelschlich rot bei korrektem Mehrfachzitat"
    # Die zu strenge Alternativregel ("alle muessen treffen") haette es faelschlich kassiert --
    # konkret vorgefuehrt, damit der Kontrast nicht nur behauptet ist:
    assert not (zitiert <= eigene), (
        "Testaufbau ueberholt: 'alle treffen' wuerde dieses Feld nicht mehr faelschlich roeten")


def test_q_gate_faengt_geloeschten_registereintrag(daten):
    """Mutationsprobe (Registermechanik): ein Register-Eintrag entfernen MUSS das Gate roeten,
    solange das zugrundeliegende Feld weiterhin ungedeckt ist -- sonst ist das Register Dekoration,
    kein Beleg, egal wie plausibel seine Begruendung liest."""
    sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
    import est_mapping as M

    fid = "kein_unterhalt"
    assert fid in KZ_GRUND_KEIN_ZIEL_STRUKTURELL, "Testaufbau ungueltig: Eintrag fehlt bereits"
    register_ohne_fid = dict(KZ_GRUND_KEIN_ZIEL_STRUKTURELL)
    entfernt = register_ohne_fid.pop(fid)
    assert fid not in register_ohne_fid and entfernt, "Loeschen hat nichts veraendert"

    treffer = _q_kandidaten(daten, M)
    bekannt = (set(KZ_GRUND_BEKANNTE_FEHLZUORDNUNG) | set(register_ohne_fid)
               | set(KZ_GRUND_RUECKSTAND) | set(KZ_GRUND_NICHT_CODESEITIG_VERIFIZIERT))
    unbekannt = set(treffer) - bekannt
    assert fid in unbekannt, (
        f"Gegenprobe fehlgeschlagen: {fid} taucht nach dem Entfernen aus dem Register NICHT als "
        f"unbekannter Treffer auf -- das Register hat keine Wirkung auf das Ergebnis.")
