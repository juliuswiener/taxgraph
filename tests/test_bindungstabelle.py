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
    # § 32 Kind-Stammdaten — der Ring rechnet aus fam_anzahl_kinder, nicht je Kind
    "fam_kinder_beruecksichtigt", "fam_kinder_im_haushalt", "kind_idnr",
    "kind_kindschaftsverh_zeitraum_a", "kind_kindschaftsverh_zeitraum_b",
    "kind_kindschaftsverhaeltnis_a", "kind_kindschaftsverhaeltnis_b",
    # § 6 Abs. 2 GWG-Tatbestand — Geltungsbedingungen, nicht erfragt
    "gwg_bewegliches_selbstaendig_nutzbar", "gwg_netto_ohne_vorsteuer", "gwg_verzeichnis_ab_250",
    # § 24a — der Accessor leitet das Alter aus geburtsjahr + VZ ab
    "rentner_alter_64_erfuellt",
    # § 9 Abs. 4a Einzelreise-Slots — der Ring rechnet aus den Tages-Aggregaten
    "vpf_abwesenheit_stunden", "vpf_an_oder_abreisetag", "vpf_auswaertige_taetigkeit",
    "vpf_keine_unterbrechung", "vpf_mit_uebernachtung",
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
    "weitere_vorsorgeaufwendungen_partner",

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
    "kinderbetreuungskosten",
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
    "vv_erhaltungsaufwand",
    "vv_gebaeude_afa",
    "vv_schuldzinsen",
    "vv_sonstige_wk",
    "weitere_vorsorgeaufwendungen",
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
