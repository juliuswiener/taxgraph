"""Paket-B Haut — reine Endpunkt-Logik über die Paket-A-Naht (traverser/API.md). LLM-frei.

Jede Funktion ist eine reine `(fall/eingabe) -> (http_status, obj)`-Hülle über GENAU eine Naht-
Funktion — kein Transport, kein Socket (den setzt server.py drum). So bleibt der Upgrade-Pfad
(stdlib -> uvicorn) ein reiner Austausch der Transport-Schicht.

Naht-Grenze (API.md): LESEN über traverser/intervall/est_mapping/bindung; SCHREIBEN
ausschliesslich `store.append_event`. Keine Steuerlogik, keine zweite Wahrheit hier.

Bescheid-Ehrlichkeit (K2): ein numerischer [min,max]-Ring erscheint NUR, wo ein exponierter
golden-Accessor die Größe wirklich rechnet. Eine Scheibe mit Gesamt-Accessor (EP) trägt einen
`intervall` (Scheiben-Bescheid); eine Multi-Regel-Scheibe ohne ehrlichen Gesamt-Accessor
(N+VOR+GWG) trägt KEINEN Gesamt-Bescheid — nur ring-fähige Teilfamilien (EP-Abzug) als
`teil_ringe`, der Rest ist ehrlich engine=unavailable. Kein erfundener Betrag.
"""
from __future__ import annotations

import json
import os
import re
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)
for _sub in ("produkt/store", "produkt/traverser", "produkt/unsicherheit", "produkt/mapping",
             "produkt/konsistenz", "golden"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import store as ST          # noqa: E402
import traverser as TR      # noqa: E402
import intervall as IV      # noqa: E402
import est_mapping as EM    # noqa: E402
import flag_check as FC     # noqa: E402  (Flag↔Einkunftsart-Widersprüche, dev-2)
import partner_check as PC  # noqa: E402  (Partner-Behinderungsfeld↔Zusammenveranlagung, dev-2)

FAELLE = os.path.join(HERE, "faelle")

EP_FELDER = ("ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz")

# an_gesamt MVP (reiner Arbeitnehmerfall). Die 4 Abwesenheits-Flags sind die Anwendbarkeits-
# Voraussetzung (bestätigte Null je Einkunftsart); Invertierung der positiven Laienfrage macht die Haut.
AN_GESAMT_FLAGS = ("kein_gewinn", "kein_kap", "kein_vuv", "kein_sonstige")
# Sperr-Felder (K2-Guard): ein aktiver Wert > 0 macht den Ring UNMÖGLICH (die Engine kann diese
# Werbungskosten mangels Modell nicht rechnen) → Ring bleibt unavailable, NIE still auf 0.
# Stufe 1b: dHf + Verpflegung sind RING-FÄHIG (raus); nur Arbeitsmittel (kein Modell) bleibt hart im Guard.
GUARD_WERBUNGSKOSTEN = ("am_anschaffungskosten",)
# Verpflegung (§ 9 Abs. 4a): 3 Tage-Ring-Inputs + 2 Reduktions-Guard-Felder. FAIL-CLOSED-ON-UNSET:
# bei Tagen > 0 ist der Ring nur fähig, wenn beide Guard-Felder EXPLIZIT sicher sind (monate ≤ 3
# gesetzt UND keine_mahlzeitengestellung=true gesetzt); sonst (inkl. UNSET) verpflegung_reduktion_offen.
VERPFLEGUNG_TAGE = ("tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig")
VERPFLEGUNG_GUARD = ("vpf_monate_am_ort", "vpf_keine_mahlzeitengestellung")
# Stufe 1a: die VOR-Felder (§ 10 Altersvorsorge) sind RING-FÄHIG — echte Rechnung via
# _vorsorge_abzug über den Store-Einzelfeld-Zugriff (kein Guard mehr, aber Teil des Kegels).
VOR_FELDER = ("vor_an_anteil_rv", "vor_ag_anteil_rv", "vor_rv_ausserhalb_lstb")
# Stufe 1b — doppelte Haushaltsführung (§ 9 Abs. 1 S. 3 Nr. 5): 3 Ring-Inputs + 4 Tatbestands-
# Bedingungen. dHf-Abzug greift NUR wenn alle 4 bestätigt-true UND Inland; Kosten > 0 mit offener
# Bedingung → dhf_tatbestand_offen, mit Ausland → ausland_dhf_nicht_ring_faehig (kein Fake).
DHF_KOSTEN = "dhf_unterkunftskosten_monat"
DHF_RING = ("dhf_unterkunftskosten_monat", "dhf_monate", "dhf_im_inland")
DHF_BEDINGUNGEN = ("dhf_beruflich_veranlasst", "dhf_eigener_hausstand",
                   "dhf_finanzielle_beteiligung", "dhf_keine_pflicht_dienstwohnung")
# Front 2 — Zusammenveranlagung (§ 26b, Splitting). Partner-Pflichtfelder + Partner-VOR. Nur bei
# veranlagung=zusammen relevant: der Ring rechnet dann catala_est_zusammen (§9a je Person + Splitting
# im Scope). MVP: Person B ohne gesonderte WK (wk_b=0), ohne VOR (Partner-VOR sperrt via Guard).
AN_GESAMT_PARTNER = ("bruttoarbeitslohn_partner", "person_b_idnr")
VOR_PARTNER_FELDER = ("vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner",
                      "vor_rv_ausserhalb_lstb_partner")
# Front V+V (§ 21): Überschuss-Rechnung Einnahmen − Werbungskosten (Scheibe 3, referenziert).
VV_GESAMT_FELDER = ("vv_einnahmen", "vv_gebaeude_afa", "vv_schuldzinsen",
                    "vv_erhaltungsaufwand", "vv_sonstige_wk")
# Front Kapital (§ 20 / § 32d): E0121709-Aggregat ODER die zwei Verlust-Töpfe (Aktien/sonstige) — MVP
# SINGLE-SOURCE (Instructor-Entscheid Q1): beide gesetzt → kapital_semantik_offen (Vordruck-Semantik
# additiv-vs-subset = benannter GAP, nicht geraten). kap_zusammenveranlagung verdoppelt den Sparer-PB.
KAP_ERTRAEGE = "kap_kapitalertraege"
KAP_TOEPFE = ("kap_gewinn_aktien", "kap_verlust_aktien", "kap_gewinn_sonstige", "kap_verlust_sonstige")
KAP_FELDER = (KAP_ERTRAEGE,) + KAP_TOEPFE + ("kap_zusammenveranlagung",)
# Front Rentner (§ 22 Leibrente + § 33b Pauschbeträge). § 22 → einkuenfte_sonstige (4-Zweig-Accessor,
# aa Rentenfreibetrag-Fixierung), § 33b → aussergewoehnliche_belastungen (behinderten+pflege+hinterbliebenen
# additiv). alter_bei_rentenbeginn ist DERIVED (immer da); rentenfreibetrag nur aa-Folgejahr (nicht im Kegel,
# Guard erzwingt ihn). Partner-Behinderung nur bei Zusammenveranlagung (partner_check).
RENTNER_AA_ARTEN = ("gesetzliche_rente", "berufsstaendische_versorgung", "private_basisrente")
RENTNER_22 = ("rentner_renten_art", "rentner_jahresrente", "rentner_renten_beginn_jahr",
              "rentner_alter_bei_rentenbeginn")
RENTNER_33B = ("rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind", "rentner_pflegegrad",
               "rentner_gepflegter_hilflos", "rentner_hinterbliebenenbezuege")
RENTNER_PARTNER = ("rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner")
RENTNER_KEGEL = RENTNER_22 + RENTNER_33B + ("veranlagung",) + AN_GESAMT_FLAGS
RENTNER_FELDER = RENTNER_KEGEL + ("rentner_rentenfreibetrag",) + RENTNER_PARTNER

# Scheiben-Konfiguration.
#   felder      : feste feld_id-Menge (None -> aus felder_datei laden).
#   felder_datei: bindung_*.yaml, aus der ALLE feld_ids der Scheibe gezogen werden.
#   gesamt_ring : quantitaet-Key, wenn EIN Accessor die GANZE Scheibe als Bescheid bedient
#                 (-> /stand.intervall + /ergebnis feste Zahl). None = kein ehrlicher Gesamt-Bescheid.
#   teil_ringe  : [(name, quantitaet, felder)] ring-fähige Teilfamilien für Scheiben OHNE Gesamt-Ring
#                 (ehrlicher Teil-Ring, ausdrücklich KEIN Scheiben-Bescheid).
SCHEIBEN = {
    "ep": {
        "felder": EP_FELDER, "felder_datei": None,
        "gesamt_ring": "abziehbarer_betrag",
        "teil_ringe": [],
    },
    "n_vor_gwg": {
        "felder": None, "felder_datei": "bindung_n_vor_gwg.yaml",
        "gesamt_ring": None,      # Gesamtsteuer via catala_gesamt = eigenes Integrations-Paket
        "teil_ringe": [("ep_werbungskosten", "abziehbarer_betrag", EP_FELDER)],
        "guard": False,
    },
    # Gesamtsteuer-Ring MVP: technischer Durchstich — echte festzusetzende ESt (§2) für den REINEN
    # Arbeitnehmerfall (Bruttolohn + Entfernungspauschale, keine gesondert erfassten Sonderausgaben,
    # keine anderen Einkunftsarten). NICHT „fertig für Angestellte": VOR/dHf/… sperren via Guard.
    "an_gesamt": {
        "felder": (("bruttoarbeitslohn", "veranlagung") + EP_FELDER + VOR_FELDER
                   + DHF_RING + DHF_BEDINGUNGEN + VERPFLEGUNG_TAGE + VERPFLEGUNG_GUARD
                   + AN_GESAMT_FLAGS + AN_GESAMT_PARTNER + VOR_PARTNER_FELDER),
        # Pflicht-Kegel = einzel-Basis (inkl. Verpflegungs-TAGE; die Reduktions-Guard-Felder prüft
        # der Guard nur bei Tagen > 0). Partner-Pflichtfelder prüft der Guard nur bei zusammen.
        "kegel": (("bruttoarbeitslohn", "veranlagung") + EP_FELDER + VOR_FELDER
                  + DHF_RING + DHF_BEDINGUNGEN + VERPFLEGUNG_TAGE + AN_GESAMT_FLAGS),
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est",
        "teil_ringe": [],
        "guard": True,
    },
    # DER §-2-GESAMT-RING (EIN catala_gesamt-Ring): dieselbe Scheibe rechnet reinen §21 (Bruttolohn =
    # bestätigte Null), Job+Vermietung UND Kapital (§ 20/§ 32d, Günstigerprüfung über zwei gesamt-Läufe).
    # §19 → einkuenfte_nichtselbststaendig (§9a-bereinigt), §21 → einkuenfte_vermietung, §20 → Kapital-
    # Steuer (est_ohne + min(Abgeltung, Günstiger-delta)). §21-Verlust mindert §19-Lohn (Loss-Offset, K2),
    # §10c einmal. Der frühere Name „vv_gesamt" war nach der §19/§20-Integration ein Fehlname — die
    # Named-Architektur-Schuld ist mit dem Rename auf „gesamt" eingelöst (an_gesamt bleibt der schmale
    # AN-only-MVP über catala_est). NAMED GAPS: § 10d Verlustvortrag; Kapital-Co-Okkurrenz E0121709+Töpfe
    # (kapital_semantik_offen); zusammen+§19 (Person-B); §22-Rente = weitere Summanden.
    "gesamt": {
        "felder": (VV_GESAMT_FELDER + ("veranlagung", "bruttoarbeitslohn")
                   + EP_FELDER + KAP_FELDER + AN_GESAMT_FLAGS),
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est_gesamt",
        "teil_ringe": [],
        "guard": True,
        "gesamt_guard": True,   # aktiviert flag_check- + Kapital-Semantik-Guards (Einkunftsart-Konsistenz)
        # fremd_arten = Einkunftsarten, die DIESE Scheibe NICHT rechnet -> müssen abwesend bestätigt sein
        # (kein_gewinn §§13-18, kein_sonstige §22). §19/§21/§20 rechnet sie -> deren Flags NICHT hier.
        "fremd_arten": ("kein_gewinn", "kein_sonstige"),
    },
    # Rentner-Ring (§ 22 Leibrente + § 33b): eigene Scheibe (Feld-Ergonomie — Renten-Felder blähen den
    # AN/gesamt-Kegel nicht). Rechnet über DENSELBEN catala_gesamt-Kern (einkuenfte_sonstige = § 22-Renten-
    # Einkünfte, aussergewoehnliche_belastungen = § 33b). fremd_arten = {kein_gewinn, kein_kap, kein_vuv} —
    # NICHT kein_sonstige (die Rente IST § 22-sonstige, kein_sonstige=False ist hier korrekt). § 24a=0 (Leib-
    # renten nach § 24a S. 2 ausgeschlossen, Renten-only-MVP). partner_check LIVE (Ehegatte-Behinderung).
    "rentner_gesamt": {
        "felder": RENTNER_FELDER,
        "kegel": RENTNER_KEGEL,     # rentenfreibetrag + Partner-Behinderung nur bedingt (Guard/Accessor)
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est_rentner",
        "teil_ringe": [],
        "guard": True,
        "gesamt_guard": True,
        "rentner": True,           # aktiviert die § 22-Rentenfreibetrag-Fixierungs-Prüfung (K2)
        "fremd_arten": ("kein_gewinn", "kein_kap", "kein_vuv"),
    },
}

_FALL_RE = re.compile(r"[A-Za-z0-9_-]{1,64}")


class ApiError(ValueError):
    """Trägt einen HTTP-Status mit (fail-closed-Antwort statt 500)."""
    def __init__(self, status: int, msg: str):
        super().__init__(msg)
        self.status = status


# ----------------------------------------------------------------- Fall-Persistenz (atomar JSON)

def _fall_pfad(fall_id: str) -> str:
    if not _FALL_RE.fullmatch(fall_id):
        raise ApiError(400, f"ungültige fall_id (nur [A-Za-z0-9_-]{{1,64}}): {fall_id!r}")
    return os.path.join(FAELLE, f"{fall_id}.json")


def lade_fall(fall_id: str) -> dict:
    p = _fall_pfad(fall_id)
    if not os.path.exists(p):
        raise ApiError(404, f"Fall {fall_id!r} existiert nicht")
    with open(p, encoding="utf-8") as f:
        return json.load(f)


def speichere_fall(fall_id: str, store: dict) -> None:
    p = _fall_pfad(fall_id)
    os.makedirs(FAELLE, exist_ok=True)
    tmp = tempfile.NamedTemporaryFile("w", dir=FAELLE, delete=False, encoding="utf-8", suffix=".tmp")
    try:
        json.dump(store, tmp, ensure_ascii=False)
        tmp.flush()
        os.fsync(tmp.fileno())
    finally:
        tmp.close()
    os.replace(tmp.name, p)


# ----------------------------------------------------------------- Scheibe -> Bindung/Engine

def _cfg(store: dict) -> dict:
    sch = store.get("scheibe")
    if sch not in SCHEIBEN:
        raise ApiError(400, f"unbekannte Scheibe {sch!r}")
    return SCHEIBEN[sch]


def _datei_felder(dateiname: str) -> tuple:
    import yaml
    d = yaml.safe_load(open(os.path.join(PRODUKT, "bindung", dateiname), encoding="utf-8"))
    return tuple(b["feld_id"] for b in d.get("bindungen", []))


def _scheibe_felder(store: dict) -> tuple:
    cfg = _cfg(store)
    return cfg["felder"] if cfg["felder"] is not None else _datei_felder(cfg["felder_datei"])


def _scheibe_bindung(store: dict) -> dict:
    felder = _scheibe_felder(store)
    b = TR.lade_bindung()
    fehlend = [f for f in felder if f not in b]
    if fehlend:
        raise ApiError(500, f"Bindungstabelle unvollständig für Scheibe: {fehlend}")
    return {f: b[f] for f in felder}


def _bescheid_fn(quantitaet: str, vz: int, bindung: dict, felder: dict | None = None):
    """bescheid_fn(feld_werte)->cent für eine ring-fähige Familie (Naht-Einheit CENT via
    intervall.bescheid_via_slots). None, wenn die Catala-Toolchain oder ein Accessor fehlt —
    dann bleibt der Ring ehrlich leer, nie ein erfundener Betrag. `felder` (materialisierter
    Store-Snapshot) erlaubt den Zugriff auf Einzelfelder, die die Summen-Slots verdecken (VOR-AG)."""
    if quantitaet == "abziehbarer_betrag":          # § 9 Entfernungspauschale
        try:
            import runner  # noqa: F401
        except Exception:
            return None

        def slot_fn(slots: dict) -> int:
            s = {"veranlagungszeitraum": int(vz),
                 "arbeitstage": int(slots.get("arbeitstage", 0)),
                 "entfernung_km_roh": int(slots.get("entfernung_km_roh", 0)),
                 "oepnv_kosten_jahr": int(slots.get("oepnv_kosten_jahr", 0)),
                 "eigenes_oder_ueberlassenes_kfz": bool(slots.get("eigenes_oder_ueberlassenes_kfz", False))}
            return runner.catala_entfernungspauschale(s)

        return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="abziehbarer_betrag")

    if quantitaet == "festzusetzende_est":          # § 2 Gesamtsteuer MVP (reiner AN-Fall)
        try:
            import runner  # noqa: F401
        except Exception:
            return None

        f = felder or {}

        def _cent(fid):
            v = f.get(fid, {}).get("wert")
            return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

        def slot_fn(slots: dict) -> int:
            wk_input = {"veranlagungszeitraum": vz,
                        **{k: slots[k] for k in
                           ("arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr", "eigenes_oder_ueberlassenes_kfz")
                           if k in slots}}
            # doppelte Haushaltsführung (Stufe 1b): dHf-Abzug NUR bei erfülltem Tatbestand —
            # Kosten > 0, Inland, alle 4 Geltungsbedingungen bestätigt-true. Sonst legitim 0
            # (Bedingung bestätigt-false = kein dHf); offene Bedingung/Ausland sperrt der Guard.
            if (_cent(DHF_KOSTEN) > 0 and f.get("dhf_im_inland", {}).get("wert") is True
                    and all(f.get(b, {}).get("wert") is True for b in DHF_BEDINGUNGEN)):
                wk_input["unterkunftskosten_monat"] = _cent(DHF_KOSTEN) // 100    # cent -> euro
                wk_input["monate"] = _cent("dhf_monate")
                wk_input["im_inland"] = True
            # Verpflegung (Stufe 1b): Tage je Kategorie in den Roh-WK, NUR wenn Reduktion explizit
            # safe (≤ 3 Monate + keine Mahlzeitengestellung); fail-closed bei UNSET (Guard sperrt
            # den Ring dann sowieso, hier doppelt sicher gegen Über-Abzug). Tage sind Anzahl (kein cent).
            _mon = f.get("vpf_monate_am_ort", {}).get("wert")
            if (sum(_cent(t) for t in VERPFLEGUNG_TAGE) > 0
                    and isinstance(_mon, int) and not isinstance(_mon, bool) and _mon <= 3
                    and f.get("vpf_keine_mahlzeitengestellung", {}).get("wert") is True):
                for t in VERPFLEGUNG_TAGE:
                    wk_input[t] = _cent(t)
            wk = runner.catala_werbungskosten_n(wk_input)   # Person A: EP + dHf + Verpflegung, roh
            # Zusammenveranlagung (§ 26b): Roh-Bruttolohn + Roh-WK pro Person -> catala_est_zusammen
            # (Pauschbetrag je Ehegatte + Splitting IM Scope). MVP: Person B ohne gesonderte WK (0),
            # ohne VOR (Partner-VOR sperrt der Guard). Person-B-WK/VOR = benannte Folge-Nachträge.
            if f.get("veranlagung", {}).get("wert") == "zusammen":
                return runner.catala_est_zusammen({
                    "veranlagungszeitraum": vz,
                    "bruttoarbeitslohn_a": int(slots.get("bruttoarbeitslohn", 0)) // 100,
                    "bruttoarbeitslohn_b": _cent("bruttoarbeitslohn_partner") // 100,
                    "werbungskosten_a": wk, "werbungskosten_b": 0,
                    "sonderausgaben_gemeinsam": 0})
            # § 10 Altersvorsorge (Stufe 1a): die VOR-Einzelfelder DIREKT aus dem Store greifen —
            # der Summen-Slot gesamtbeitraege_inkl_ag würde den AG-Anteil verschmelzen und die
            # Kürzung (nach dem Cap) unmöglich machen. gesamtbeitraege = AN + AG + außerhalb; der
            # steuerfreie AG-Anteil getrennt. Naht-CENT -> EURO für _vorsorge_abzug.
            gesamt = (_cent("vor_an_anteil_rv") + _cent("vor_ag_anteil_rv")
                      + _cent("vor_rv_ausserhalb_lstb")) // 100
            ag = _cent("vor_ag_anteil_rv") // 100
            so = runner._vorsorge_abzug({"vorsorge_gesamtbeitraege_inkl_ag": gesamt,
                                         "vorsorge_ag_anteil_steuerfrei": ag}, vz)
            return runner.catala_est({
                "veranlagungszeitraum": vz,
                "veranlagung": slots.get("veranlagung", "einzel"),
                # bruttoarbeitslohn ist Naht-CENT (Bindung typ:cent) -> catala_est erwartet EURO.
                "bruttoarbeitslohn": int(slots.get("bruttoarbeitslohn", 0)) // 100,
                "werbungskosten": wk,
                "sonderausgaben": so})
        return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="festzusetzende_est")

    if quantitaet == "festzusetzende_est_gesamt":   # § 21 V+V via catala_gesamt (reiner §21-MVP)
        try:
            import runner  # noqa: F401
        except Exception:
            return None
        f = felder or {}

        def _c(fid):
            v = f.get(fid, {}).get("wert")
            return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

        def slot_fn(slots: dict) -> int:
            # § 21 Überschuss (Einnahmen − Werbungskosten), Naht-CENT -> EURO. catala_gesamt macht
            # den Verlust-Floor (negativ → est 0, K2), die § 2-Abs.-3-Verrechnung und den Tarif.
            vv = runner.catala_vermietung_einkuenfte({
                "einnahmen": _c("vv_einnahmen") // 100,
                "gebaeude_afa": _c("vv_gebaeude_afa") // 100,
                "schuldzinsen": _c("vv_schuldzinsen") // 100,
                "erhaltungsaufwand": _c("vv_erhaltungsaufwand") // 100,
                "sonstige_werbungskosten": _c("vv_sonstige_wk") // 100})
            g = {"gesamtfall": True, "veranlagungszeitraum": vz,
                 "veranlagung": slots.get("veranlagung", "einzel"),
                 "einkuenfte_vermietung": vv}
            # kombiniert §19+§21: Bruttolohn im Kegel -> §19-Einkünfte (§9a-bereinigt, § 2 Abs. 2 Nr. 2)
            # als einkuenfte_nichtselbststaendig in die §-2-Summe; der §21-Verlust mindert dann den
            # §19-Lohn (§ 2 Abs. 3). Bruttolohn 0 (reiner Vermieter) -> einkuenfte_ns 0, kein Effekt.
            # §19-WK = Entfernungspauschale (roh, § 9a-Günstiger im einzel-Tarif); dHf/Verpflegung/AM
            # sind hier NICHT modelliert (Folge-Nachtrag) — es gibt keine solchen Slots in der Scheibe.
            ns_wk = runner.catala_werbungskosten_n({"veranlagungszeitraum": vz,
                **{k: slots[k] for k in
                   ("arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr", "eigenes_oder_ueberlassenes_kfz")
                   if k in slots}})
            g["einkuenfte_nichtselbststaendig"] = runner.catala_einkuenfte_nichtselbststaendig({
                "veranlagungszeitraum": vz,
                "bruttoarbeitslohn": int(slots.get("bruttoarbeitslohn", 0)) // 100,   # Naht-CENT -> EURO
                "werbungskosten": ns_wk})
            est_ohne = runner.catala_est(g)     # § 19 + § 21, KEIN Kapital (est_regulaer_ohne_kap)
            # Kapital § 20/§ 32d: SINGLE-SOURCE (Instructor-Q1) — E0121709-Aggregat XOR Verlust-Töpfe;
            # Co-Okkurrenz sperrt der Guard (kapital_semantik_offen). Töpfe (§ 20 Abs. 6, per-Topf-Floor)
            # → verrechnete; sonst das Aggregat. Dann Sparer-PB (§ 20 Abs. 9). Kapitaleinkünfte ≤ 0 ->
            # reiner §19/§21-Bescheid (kein Kapital-Effekt).
            if any(_c(t) != 0 for t in KAP_TOEPFE):
                verrechnete = runner.catala_kapital_verrechnung({
                    "gewinn_aktien": _c("kap_gewinn_aktien") // 100,
                    "verlust_aktien": _c("kap_verlust_aktien") // 100,
                    "gewinn_sonstige": _c("kap_gewinn_sonstige") // 100,
                    "verlust_sonstige": _c("kap_verlust_sonstige") // 100})
            else:
                verrechnete = _c(KAP_ERTRAEGE) // 100
            zusammen = (g["veranlagung"] == "zusammen"
                        or f.get("kap_zusammenveranlagung", {}).get("wert") is True)
            kapitaleinkuenfte = runner.catala_sparer_pb({
                "veranlagungszeitraum": vz, "kapitalertraege": verrechnete, "zusammenveranlagung": zusammen})
            if kapitaleinkuenfte <= 0:
                return est_ohne
            # Günstigerprüfung (§ 32d Abs. 6): reguläre est MIT Kapital im zvE (Grundtarif) vs Abgeltung.
            est_mit = runner.catala_est(dict(g, einkuenfte_kapitalvermoegen=kapitaleinkuenfte))
            kapital_steuer = runner.catala_kapital_steuer({
                "veranlagungszeitraum": vz, "kapitaleinkuenfte": kapitaleinkuenfte,
                "est_regulaer_mit_kap": est_mit, "est_regulaer_ohne_kap": est_ohne})
            return est_ohne + kapital_steuer    # Günstiger -> est_mit; Abgeltung -> est_ohne + abgeltung
        return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="festzusetzende_est")

    if quantitaet == "festzusetzende_est_rentner":   # § 22 Renten + § 33b via catala_gesamt
        try:
            import runner  # noqa: F401
        except Exception:
            return None
        f = felder or {}

        def _c(fid):
            v = f.get(fid, {}).get("wert")
            return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

        def _b(fid):
            return f.get(fid, {}).get("wert")

        def slot_fn(slots: dict) -> int:
            # § 22 Renten-Einkünfte → einkuenfte_sonstige. Naht-CENT → EURO (jahresrente/rentenfreibetrag);
            # renten_beginn_jahr/alter sind int (kein cent). Die aa-Folgejahr-ohne-RF-Sperre fängt der Guard
            # VOR diesem Aufruf (rentenfreibetrag_fixierung_offen); hier kommt nur der rechenbare Fall an.
            rf = _b("rentner_rentenfreibetrag")
            renten = runner.catala_renten_einkuenfte({
                "veranlagungszeitraum": vz,
                "renten_art": _b("rentner_renten_art"),
                "jahresrente": _c("rentner_jahresrente") // 100,
                "renten_beginn_jahr": _c("rentner_renten_beginn_jahr"),
                "alter_bei_rentenbeginn": _c("rentner_alter_bei_rentenbeginn"),
                "rentenfreibetrag": (rf // 100 if isinstance(rf, (int, float))
                                     and not isinstance(rf, bool) else None)})
            # § 33b Behinderten- + Pflege- + Hinterbliebenen-Pauschbetrag (additiv) → aussergewoehnliche_belastungen.
            ausserg = (runner.catala_behinderten_pb({
                           "veranlagungszeitraum": vz, "grad_der_behinderung": _c("rentner_grad_der_behinderung"),
                           "ist_hilflos_blind_taubblind": _b("rentner_hilflos_blind_taubblind") is True})
                       + runner.catala_pflege_pb({
                           "veranlagungszeitraum": vz, "pflegegrad": _c("rentner_pflegegrad"),
                           "ist_hilflos": _b("rentner_gepflegter_hilflos") is True})
                       + runner.catala_hinterbliebenen_pb({
                           "veranlagungszeitraum": vz,
                           "hat_hinterbliebenenbezuege": _b("rentner_hinterbliebenenbezuege") is True}))
            return runner.catala_est({
                "gesamtfall": True, "veranlagungszeitraum": vz,
                "veranlagung": _b("veranlagung") or "einzel",
                "einkuenfte_sonstige": renten,
                "aussergewoehnliche_belastungen": ausserg})
        return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="festzusetzende_est")

    return None     # kein exponierter Accessor -> ehrlich None (dHf/Verpflegung/AM/VOR/GWG)


def _feste_zahl(felder: dict, bindung: dict, cfg: dict, vz: int, scheibe_felder: tuple):
    """Fail-closed: die festzusetzende Zahl NUR bei Scheiben-Gesamt-Accessor UND vollständig
    bestätigtem Input-Kegel (Meet). Ohne Gesamt-Accessor gibt es KEINE Scheiben-Zahl (ehrlich)."""
    q = cfg["gesamt_ring"]
    if q is None:
        return None
    zustaende = [felder[f]["zustand"] for f in scheibe_felder if f in felder]
    if len(zustaende) < len(scheibe_felder) or ST.meet_zustand(zustaende) != "bestaetigt":
        return None
    bf = _bescheid_fn(q, vz, bindung, felder)
    if bf is None:
        return None
    return bf({f: felder[f]["wert"] for f in scheibe_felder})


def _an_gesamt_sperrgrund(felder: dict, cfg: dict | None = None, vz: int | None = None):
    """K2-Guard: nicht-ring-fähige Werbungskosten/Einkunftsarten sperren den Ring GANZ (nie Fake-0).
    Ein dHf-/Verpflegung-/AM-Feld mit Wert > 0 (vorläufig ODER bestätigt) sperrt (kein Catala-Modul);
    ein fremd_arten-Flag = false (Nutzer HAT eine NICHT von dieser Scheibe gerechnete Art) macht den
    §2-Ring unzulässig. VOR (§ 10) ist seit Stufe 1a ring-fähig — kein Guard mehr."""
    def _positiv(f):
        v = felder.get(f)
        w = v and v.get("wert")
        return isinstance(w, (int, float)) and not isinstance(w, bool) and w > 0
    # Partner-Behinderungsfeld (§ 33b Person B) ohne Zusammenveranlagung: benannte Inkonsistenz
    # (dev-2s partner_check, Spiegel zu partner_kegel_offen). Universell VOR der Scheiben-Verzweigung —
    # feuert live, sobald eine Scheibe (rentner_gesamt) die rentner_*_partner-Felder führt.
    if PC.partner_ohne_zusammen(felder):
        return "partner_konsistenz_offen"
    if cfg and cfg.get("gesamt_guard"):
        # Gesamt-Ring: Flag↔Einkunftsart-Widerspruch (kein_X=true + echtes Feld > 0 bestätigt) surfacen —
        # K2, keine still übergangene Einkunftsart (dev-2s flag_check).
        if FC.flag_widersprueche(felder):
            return "flag_konsistenz_offen"
        # Kapital-Semantik (Instructor-Q1, fail-closed): E0121709-Aggregat UND Verlust-Töpfe beide gesetzt
        # → additiv-vs-subset ungeklärt (benannter GAP) → kein Rate-Bescheid (die slot_fn nähme sonst still
        # nur die Töpfe und verschluckte das Aggregat).
        if _positiv(KAP_ERTRAEGE) and any(_positiv(t) for t in KAP_TOEPFE):
            return "kapital_semantik_offen"
        # § 22 aa Rentenfreibetrag-Fixierung (K2): ab dem 2. Jahr ist der Freibetrag in EURO fix; fehlt er
        # (aa-Folgejahr, renten_beginn < VZ, kein rentenfreibetrag) → fail-closed, kein %×erhöhte-Rente.
        if cfg.get("rentner"):
            art = felder.get("rentner_renten_art", {}).get("wert")
            beginn = felder.get("rentner_renten_beginn_jahr", {}).get("wert")
            rf = felder.get("rentner_rentenfreibetrag", {}).get("wert")
            if (art in RENTNER_AA_ARTEN and isinstance(beginn, int) and vz is not None and beginn < vz
                    and not (isinstance(rf, (int, float)) and not isinstance(rf, bool))):
                return "rentenfreibetrag_fixierung_offen"
        # fremd_arten = Arten, die DIESE Scheibe NICHT rechnet → bestätigt-false (Nutzer HAT die Art) sperrt
        # (Stufe 2). Die von der Scheibe GERECHNETEN Arten stehen NICHT in fremd_arten (kein Fehl-Sperr).
        if any(felder.get(fl, {}).get("wert") is False for fl in cfg.get("fremd_arten", ())):
            return "einkunftsart_nicht_ring_faehig"
        return None
    if any(_positiv(f) for f in GUARD_WERBUNGSKOSTEN):
        return "werbungskosten_nicht_ring_faehig"
    # dHf-Tatbestand: Kosten > 0, aber Auslandsunterkunft ODER eine der 4 Geltungsbedingungen offen
    # (nicht bestätigt) → kein Ring (K2: kein dHf-Abzug ohne bestätigten Tatbestand). Eine Bedingung
    # bestätigt-FALSE ist NICHT offen — dann greift dHf legitim nicht (Abzug 0), Ring bleibt gültig.
    if _positiv(DHF_KOSTEN):
        if felder.get("dhf_im_inland", {}).get("wert") is False:
            return "ausland_dhf_nicht_ring_faehig"
        if any((felder.get(b) or {}).get("zustand") != "bestaetigt" for b in DHF_BEDINGUNGEN):
            return "dhf_tatbestand_offen"
    # Verpflegung: Tage > 0 → Ring nur fähig, wenn BEIDE Reduktions-Guard-Felder EXPLIZIT sicher
    # sind (§ 9 Abs. 4a S. 6 3-Monats-Frist + S. 8 Mahlzeitenkürzung). FAIL-CLOSED bei UNSET: wer
    # Reisetage einträgt, aber die Reduktions-Fragen nicht beantwortet, bekommt keinen Über-Abzug.
    if sum((felder.get(t, {}).get("wert") or 0) for t in VERPFLEGUNG_TAGE) > 0:
        mon = felder.get("vpf_monate_am_ort", {}).get("wert")
        safe = (isinstance(mon, int) and not isinstance(mon, bool) and mon <= 3
                and felder.get("vpf_keine_mahlzeitengestellung", {}).get("wert") is True)
        if not safe:
            return "verpflegung_reduktion_offen"
    # Zusammenveranlagung: der Splitting-Ring braucht den vollständigen Kegel BEIDER Personen.
    if felder.get("veranlagung", {}).get("wert") == "zusammen":
        if any((felder.get(pf) or {}).get("zustand") != "bestaetigt" for pf in AN_GESAMT_PARTNER):
            return "partner_kegel_offen"        # Person-B-Pflichtfeld offen → kein halber Bescheid
        if any(_positiv(vf) for vf in VOR_FELDER + VOR_PARTNER_FELDER):
            return "partner_vor_offen"           # MVP-zusammen ohne VOR; VOR-Feld (A/B) gesetzt sperrt
    if any(felder.get(f, {}).get("wert") is False for f in AN_GESAMT_FLAGS):
        return "einkunftsart_nicht_ring_faehig"
    return None


# ----------------------------------------------------------------- Endpunkte (reine Logik)

def fall_anlegen(body: dict) -> tuple[int, dict]:
    scheibe = body.get("scheibe", "ep")
    if scheibe not in SCHEIBEN:
        raise ApiError(400, f"unbekannte Scheibe {scheibe!r}")
    vz = int(body.get("veranlagungszeitraum", 2025))
    fall_id = body.get("fall_id")
    if not fall_id or not _FALL_RE.fullmatch(str(fall_id)):
        raise ApiError(400, "fall_id fehlt oder ungültig (nur [A-Za-z0-9_-]{1,64})")
    if os.path.exists(_fall_pfad(fall_id)):
        raise ApiError(409, f"Fall {fall_id!r} existiert bereits")
    store = ST.leerer_store(vz, fall_id=fall_id)
    store["scheibe"] = scheibe
    speichere_fall(fall_id, store)
    return 201, {"fall_id": fall_id, "scheibe": scheibe, "veranlagungszeitraum": vz}


def _badge(herkunft: dict) -> str:
    """Herkunfts-Kategorie je Wert (UI-Lab Dim 1, „Herkunft zum Anfassen"): die 6 Store-Herkünfte
    direkt als anzeigbare Badge-Klasse (statt binär solide/schimmernd). Die Haut stylt jede Kategorie
    (laie=selbst · beleg_import=Beleg · vorjahr/berechnet/orakel=abgeleitet · llm_vorschlag=KI-schimmernd)."""
    return herkunft.get("herkunft", "laie")


def _ring_bindung(cfg: dict, bindung: dict) -> dict:
    """Bindung für die Spannen-/intervall-Rechnung: nur die Pflicht-Kegel-Felder. Sonst zögen die
    (bei einzel ungesetzten) Partner-Felder als unbounded-ohne-Wert das Intervall auf nicht_fixierbar."""
    kegel = cfg.get("kegel")
    return {f: bindung[f] for f in kegel if f in bindung} if kegel else bindung


def _gesamt_beitrag(store: dict, cfg: dict, bindung: dict, felder: dict, sid: str, vz: int):
    """Frage-Reihenfolge-Gewichte aus dem verfügbaren Ring (Gesamt bevorzugt, sonst erster Teil)."""
    if cfg["gesamt_ring"]:
        rb = _ring_bindung(cfg, bindung)
        bf = _bescheid_fn(cfg["gesamt_ring"], vz, rb, felder)
        if bf is not None:
            return {b["feld_id"]: b["spanne_cent"]
                    for b in IV.intervall(felder, rb, bf, snapshot_id=sid)["beitraege"]}
    for _name, q, tfelder in cfg["teil_ringe"]:
        tb = {f: bindung[f] for f in tfelder if f in bindung}
        bf = _bescheid_fn(q, vz, tb)
        if bf is not None:
            return {b["feld_id"]: b["spanne_cent"]
                    for b in IV.intervall(felder, tb, bf, snapshot_id=sid)["beitraege"]}
    return None


def fragen(fall_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    cfg = _cfg(store)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    beitrag = _gesamt_beitrag(store, cfg, bindung, felder, sid, int(store["veranlagungszeitraum"]))
    queue = TR.naechste_fragen(store, bindung, beitrag)
    out = []
    for fid in queue:
        b = bindung[fid]
        out.append({
            "feld_id": fid,
            "fragetext_laie": b.get("fragetext_laie"),
            "hilfe_kurz": b.get("hilfe_kurz"),
            "typ": b["typ"],
            "einheit": b.get("einheit"),
            "bereich": b.get("bereich"),
            "enum_werte": b.get("enum_werte"),
            "beispielwert": b.get("beispielwert"),
            "anker_ref": b.get("anker_ref"),
        })
    return 200, {"fall_id": fall_id, "snapshot_id": sid, "fragen": out}


def stand(fall_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    cfg = _cfg(store)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    rel = TR.relevanz(store, bindung)
    vz = int(store["veranlagungszeitraum"])
    felder_out = {
        fid: {"wert": v["wert"], "zustand": v["zustand"], "herkunft": v["herkunft"],
              "herkunft_badge": _badge(v["herkunft"])}
        for fid, v in felder.items()
    }

    gesamt_iv, engine, teil = None, "unavailable", []
    gesperrt = _an_gesamt_sperrgrund(felder, cfg, vz) if cfg.get("guard") else None
    if gesperrt:
        engine = "gesperrt"          # nicht-ring-fähiger Abzug/Einkunftsart -> kein Ring (K2)
    elif cfg["gesamt_ring"]:
        rb = _ring_bindung(cfg, bindung)
        bf = _bescheid_fn(cfg["gesamt_ring"], vz, rb, felder)
        if bf is not None:
            gesamt_iv = IV.intervall(felder, rb, bf, snapshot_id=sid)["intervall"]
            engine = "catala"
    else:
        for name, q, tfelder in cfg["teil_ringe"]:
            tb = {f: bindung[f] for f in tfelder if f in bindung}
            bf = _bescheid_fn(q, vz, tb, felder)
            if bf is not None:
                tiv = IV.intervall(felder, tb, bf, snapshot_id=sid)["intervall"]
                teil.append({"familie": name, "quantitaet": q, "intervall": tiv})
        engine = "catala_teilweise" if teil else "unavailable"

    return 200, {"fall_id": fall_id, "snapshot_id": sid, "engine": engine,
                 "felder": felder_out, "relevanz": rel, "intervall": gesamt_iv,
                 "teil_ringe": teil, "ring_gesperrt": gesperrt}


_ERLAUBTE_ZUSTAENDE = {"vorlaeufig", "bestaetigt"}


def event(fall_id: str, body: dict) -> tuple[int, dict]:
    """DER einzige Schreib-Endpunkt — dünne Hülle über store.append_event. Die fail-closed-Garantien
    (llm->vorlaeufig, bestaetigt->signal_2, ein aktives Event/feld) erzwingt der Store, nicht die Haut."""
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    fid = body.get("feld_id")
    if fid not in bindung:
        raise ApiError(400, f"feld_id {fid!r} nicht in dieser Scheibe")
    zustand = body.get("zustand")
    if zustand not in _ERLAUBTE_ZUSTAENDE:
        raise ApiError(400, f"zustand muss {_ERLAUBTE_ZUSTAENDE} sein")
    herkunft = body.get("herkunft")
    if not isinstance(herkunft, dict) or "herkunft" not in herkunft:
        raise ApiError(400, "herkunft-Objekt (mit Schlüssel 'herkunft') ist Pflicht")
    schreiber = body.get("schreiber")
    if not isinstance(schreiber, str) or not schreiber:
        raise ApiError(400, "schreiber ist Pflicht")
    try:
        ev = ST.append_event(
            store, feld_id=fid, wert=body.get("wert"), zustand=zustand, herkunft=herkunft,
            schreiber=schreiber, signal=body.get("signal"), ersetzt=body.get("ersetzt"),
            ts=body.get("ts"))
    except ValueError as e:
        # fail-closed-Abweisung des Stores -> 422 (nicht 500): die Haut hat korrekt weitergereicht.
        raise ApiError(422, str(e))
    speichere_fall(fall_id, store)
    return 201, {"event_id": ev["event_id"], "feld_id": fid, "zustand": zustand}


def warum(fall_id: str, feld_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    j = TR.justification(store, feld_id, bindung)
    if j is None:
        raise ApiError(404, f"Feld {feld_id!r} hat (noch) kein Event")
    return 200, {"fall_id": fall_id, "justification": j}


def ergebnis(fall_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    cfg = _cfg(store)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    # Pflicht-Kegel: einzel-Basis (cfg["kegel"]); die Partner-Pflichtfelder gehören nur bei
    # veranlagung=zusammen dazu und werden dort vom Guard geprüft.
    scheibe_felder = cfg.get("kegel") or _scheibe_felder(store)
    vz = int(store["veranlagungszeitraum"])
    if cfg.get("guard"):
        # K2: ein nicht-ring-fähiger Abzug/Einkunftsart sperrt den Ring VOR jeder Zahl — nie Fake-Bescheid.
        sperr = _an_gesamt_sperrgrund(felder, cfg, vz)
        if sperr:
            return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": None,
                         "grund": sperr, "offen": [], "trace": None}
    zahl = _feste_zahl(felder, bindung, cfg, vz, scheibe_felder)
    if zahl is None:
        if cfg["gesamt_ring"] is None:
            # Multi-Regel-Scheibe ohne ehrlichen Gesamt-Accessor: bewusst KEINE Scheiben-Zahl.
            return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": None,
                         "grund": "kein_scheiben_gesamtbescheid", "offen": [], "trace": None}
        offen = [f for f in scheibe_felder
                 if f not in felder or felder[f]["zustand"] != "bestaetigt"]
        bf = _bescheid_fn(cfg["gesamt_ring"], vz, bindung, felder)
        grund = "engine_unavailable" if (bf is None and not offen) else "input_kegel_nicht_bestaetigt"
        return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": None,
                     "grund": grund, "offen": sorted(offen), "trace": None}
    trace = TR.trace_ergebnis(store, bindung, snapshot_id=sid)
    return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": zahl,
                 "grund": "bestaetigt", "offen": [], "trace": trace}


def deklaration(fall_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    result = EM.deklariere(felder, bindung, snapshot_id=sid)
    return 200, {"fall_id": fall_id, **result}


def graph(fall_id: str) -> tuple[int, dict]:
    """Read-only Abhängigkeits-Übersicht (Desktop): Knoten = Regeln der Scheibe mit ihrem
    Relevanz-Status (aus traverser.relevanz), Kanten = Feld→Regel (welches Abfrage-Feld welche Regel
    speist, mit Feld-Zustand). Reine Ableitung, EIN Traverser-Aufruf, kein Bescheid, kein Schreibpfad."""
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    rel = TR.relevanz(store, bindung)
    knoten = [{"regel_id": rid, "status": s["status"],
               "gates_offen": s["gates_offen"], "annahmen_offen": s["annahmen_offen"]}
              for rid, s in sorted(rel.items())]
    kanten = []
    for fid in sorted(bindung):
        q = bindung[fid]["quelle"]
        ev = felder.get(fid)
        kanten.append({
            "feld_id": fid,
            "regel_id": q["regel_id"],
            "rolle": "slot" if "signatur_slot" in q else "gate",
            "zustand": ev["zustand"] if ev else "offen",
            "fragetext_laie": bindung[fid].get("fragetext_laie"),
        })
    return 200, {"fall_id": fall_id, "snapshot_id": sid, "knoten": knoten, "kanten": kanten}


# POST /chat und POST /elster-ampel: bewusst KEINE 200-Antwort in dieser Stufe.
CHAT_501 = {
    "fehler": "not_implemented",
    "vertrag": ("LLM-Chat schreibt qua Store-Auflage A ausschliesslich vorlaeufig-Events "
                "(schreiber='llm:…', herkunft.herkunft='llm_vorschlag', signal_2=null); "
                "Bestätigung bleibt der menschliche Zwei-Signal-Klick."),
    "stufe": "spätere Stufe mit eigenem Julius-Cap — kein LLM-Call in dieser Stufe.",
}
AMPEL_503 = {
    "fehler": "unavailable",
    "grund": ("ELSTER-Ampel (warmer checkESt-Daemon) ist für diese Scheibe noch nicht verdrahtet "
              "— ein gültiger ESt-Fall entsteht erst mit der Gesamtsteuer-Integration. Kein Fake-Grün."),
    "regel": "gekappt_verdacht=true ist nie grün (API.md-Garantie 5).",
}
# Arbeitsweg-Entfernung über Karten-Dienst (Julius-Feature): der eigentliche Geocoding+Routing-Aufruf ist
# eine AUSGEHENDE Integration mit PII (Adressen verlassen das Gerät) → wartet auf Julius' Service-Wahl + Cap.
# Bis dahin STUB (kein Live-Aufruf), analog CHAT_501. Die UI-Affordance (Adress-Eingabe) ist gebaut; der
# Karten-km-Vorschlag kommt erst, wenn der Dienst verbunden ist — die manuelle km-Eingabe bleibt Fallback.
ENTFERNUNG_FALLBACK = {
    "fehler": "unavailable",
    "vertrag": ("Der Karten-Dienst ist nicht verbunden (kein Schlüssel gesetzt oder Netz-/Antwort-Fehler) "
                "— bitte gib die Entfernung manuell ein (kürzeste Straßenverbindung, § 9 Abs. 1 S. 3 Nr. 4 EStG)."),
}


def entfernung(fall_id: str, body: dict) -> tuple[int, dict]:
    """Arbeitsweg-km über den Karten-Dienst (Julius-Feature). AUSGEHENDE PII-Integration: die Adressen
    gehen an OpenRouteService (nur auf Nutzer-Klick). Das Ergebnis ist ein VORSCHLAG — die Haut prefillt
    das km-Feld, der Nutzer bestätigt/überschreibt (Zwei-Signal, § 9 kürzeste Straßenverbindung; eine
    längere ist bei regelmäßiger Nutzung zulässig). Kein Key / Netzfehler → 503-Fallback (manuell), nie
    Crash, nie still gesetzt. Der API-Key kommt nur aus $ORS_API_KEY (nie im Repo)."""
    lade_fall(fall_id)                                   # 404, wenn der Fall nicht existiert
    von = (body.get("von") or "").strip()
    nach = (body.get("nach") or "").strip()
    if not von or not nach:
        raise ApiError(400, "von und nach (Adressen) sind Pflicht")
    try:
        import ors_client
        km = ors_client.entfernung_km(von, nach)
    except Exception:                                    # OrsNichtVerfuegbar / Import — sauberer Fallback
        return 503, ENTFERNUNG_FALLBACK
    return 200, {"km": km, "quelle": "openrouteservice",
                 "hinweis": "Vorschlag aus dem Karten-Dienst — bitte prüfen und bestätigen."}
