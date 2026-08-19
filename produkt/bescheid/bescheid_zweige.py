"""Die Rechenzweige selbst: der Dispatcher und die vier Rechenkerne (abziehbarer Betrag, festzusetzende ESt in drei Auspraegungen) plus die Abschlusszahlung.

Aufgeteilt am 2026-08-19 aus bescheid.py (2610 Zeilen). Kein Logik-Edit — die
Rümpfe sind byte-identisch übernommen. bescheid.py ist die Fassade geblieben und
re-exportiert alles; die Aufrufer (api.py, Tests) merken den Schnitt nicht.
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "produkt/engine", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import intervall as IV      # noqa: E402
import est_mapping as EM    # noqa: E402
import flag_check as FC     # noqa: E402  (Flag↔Einkunftsart-Widersprüche)
import partner_check as PC  # noqa: E402  (Partner-Behinderungsfeld↔Zusammenveranlagung)
from api_constants import (  # noqa: E402
    AN_GESAMT_FLAGS,
    AN_GESAMT_PARTNER,
    ARBEITSMITTEL_KOSTEN,
    DHF_BEDINGUNGEN,
    DHF_KOSTEN,
    EUER_KOMPONENTEN,
    GESAMT_PARTNER_19,
    GESAMT_PARTNER_KAP,
    KAP_ERTRAEGE,
    KAP_ERTRAEGE_PARTNER,
    KAP_TOEPFE,
    KAP_TOEPFE_PARTNER,
    MITU_FELDER,
    RENTNER_22,
    RENTNER_22_PARTNER,
    RENTNER_AA_ARTEN,
    UEBERNACHTUNG_BEDINGUNGEN,
    UEBERNACHTUNG_KOSTEN,
    VERPFLEGUNG_TAGE,
    VERPFLEGUNG_TAGE_NACH_FRIST,
    VOR_FELDER,
    VOR_PARTNER_FELDER,
    VV_GESAMT_FELDER,
    dba_methode_fuer,
)


# Aus den Blatt-Modulen. Namentlich, nicht per Star-Import (tests/test_split_naht_gate.py).
from bescheid_abzuege import (  # noqa: E402
    _abs3_eligible,
    _kind_kv_pv_summe,
    _oepnv_eur,
    _p33b_kind_pauschbetraege,
    _shared_steuer_sonder_agb,
)
from bescheid_einkuenfte import (  # noqa: E402
    _gewinn_partner_anteil,
    _gwg_sofortabzug_summe,
    _laufender_gewinn,
    _p20_kapitaleinkuenfte,
    _p23_ansonsten_einkuenfte,
    _p35_summen,
    _shared_dba_sonstige,
)

def _abschlusszahlung_cent(felder: dict, zahl_cent: int):
    """§ 36 Abs. 2+4 EStG — Abschlusszahlung (+) / Erstattung (−) in CENT auf der bereits festgesetzten
    ESt (zahl_cent), scheibe-agnostisch (jede Rate-Scheibe erzeugt genau eine festzusetzende ESt).
    None, wenn KEIN einziges Anrechnungsfeld (LSt/VZ/KapESt/SolZ-KapESt/KiSt-KapESt) bestätigt
    vorliegt — dann keine irreführende Voll-Steuer-Nachzahlung ausweisen. Nur BESTÄTIGTE Felder
    (vorläufige Werte bewegen die Anrechnung nie — [[ring-liest-vorlaeufig-parallel-pfad-luecke]]).
    Stufe 2 (2026-08-10, BAU-GO team-lead): KapESt/SolZ/KiSt (Zeilen 37-39 Anlage KAP) spiegeln
    p36_lohnsteuer — dieselbe Quelle speist Deklaration (bindung_p36_abschlusszahlung.yaml) UND
    Ring-Anrechnung hier, sonst zeigt /ergebnis eine Zahl, die der Bescheid unterschreitet."""
    def _best(fid):
        e = felder.get(fid)
        if e is None or e.get("zustand") != "bestaetigt":
            return None
        w = e.get("wert")
        return w if isinstance(w, (int, float)) and not isinstance(w, bool) else None
    lst, vor = _best("p36_lohnsteuer"), _best("p36_vorauszahlungen")
    kapest = _best("p36_kapitalertragsteuer")
    solz = _best("p36_kapitalertragsteuer_solz")
    kist = _best("p36_kapitalertragsteuer_kist")
    if lst is None and vor is None and kapest is None and solz is None and kist is None:
        return None
    import runner
    return runner.catala_p36_abschlusszahlung({
        "festzusetzende_est_cent": int(zahl_cent),
        "lohnsteuer_cent": int(lst or 0),
        "kapitalertragsteuer_cent": int(kapest or 0),
        "kapitalertragsteuer_solz_cent": int(solz or 0),
        "kapitalertragsteuer_kist_cent": int(kist or 0),
        "vorauszahlungen_cent": int(vor or 0)})


def _zweig_abziehbarer_betrag(vz: int, bindung: dict):
    """§ 9 Entfernungspauschale — quantitaet='abziehbarer_betrag'.

    Aus _bescheid_fn herausgezogen (Refactor 2026-08-13, Schritt 1). Kontextfrei: hängt an
    KEINER Closure des _bescheid_fn-Kopfes, nur an vz/bindung/_oepnv_eur. Deshalb der erste
    Schnitt — Verhaltensgleichheit ist hier durch Augenschein nachweisbar, nicht nur durch
    den Golden-Vergleich."""
    try:
        import runner  # noqa: F401
    except Exception:
        return None

    def slot_fn(slots: dict) -> int:
        s = {"veranlagungszeitraum": int(vz),
             "arbeitstage": int(slots["arbeitstage"]),
             "entfernung_km_roh": int(slots["entfernung_km_roh"]),
             "oepnv_kosten_jahr": _oepnv_eur(slots),
             "eigenes_oder_ueberlassenes_kfz": bool(slots["eigenes_oder_ueberlassenes_kfz"])}
        return runner.catala_entfernungspauschale(s)

    return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="abziehbarer_betrag")


def _zweig_festzusetzende_est(vz: int, bindung: dict, felder, store, nur_bestaetigt: bool,
                             solz_container, extras):
    """§ 2 Gesamtsteuer MVP (reiner AN-Fall) — Scheibe an_gesamt.

    Aus _bescheid_fn herausgezogen (Refactor 2026-08-13). `felder` ist der bei
    nur_bestaetigt=True bereits auf zustand=bestaetigt gefilterte Snapshot — die Filterung
    bleibt im Kopf von _bescheid_fn, damit die Zwei-Signal-Invariante EINE Stelle behält
    statt je Zweig wiederholt zu werden."""
    try:
        import runner  # noqa: F401
    except Exception:
        return None

    f = felder or {}

    def _cent(fid):
        v = f.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

    def slot_fn(slots: dict) -> int:
        # Klasse-3 fail-open geschlossen (2026-08-09): alle 4 Keys stehen im Pflicht-Kegel
        # von an_gesamt (EP_FELDER, api_constants.py) — "if k in slots" liess einen fehlenden
        # Key (defekte Bindung) lautlos aus wk_input verschwinden statt zu werfen; gemessen
        # 323 EUR stille Steuermehrbelastung ohne KeyError (reports/adjudikation/klasse3_fail_open_2026-08-09.md).
        wk_input = {"veranlagungszeitraum": vz,
                    **{k: slots[k] for k in
                       ("arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr", "eigenes_oder_ueberlassenes_kfz")}}
        if "oepnv_kosten_jahr" in wk_input:
            wk_input["oepnv_kosten_jahr"] = _oepnv_eur(wk_input)   # Naht-CENT -> EURO
        # doppelte Haushaltsführung (Stufe 1b): dHf-Abzug NUR bei erfülltem Tatbestand —
        # Kosten > 0, Inland, alle 4 Geltungsbedingungen bestätigt-true. Sonst legitim 0
        # (Bedingung bestätigt-false = kein dHf); offene Bedingung/Ausland sperrt der Guard.
        if (_cent(DHF_KOSTEN) > 0 and f.get("dhf_im_inland", {}).get("wert") is True
                and all(f.get(b, {}).get("wert") is True for b in DHF_BEDINGUNGEN)):
            wk_input["unterkunftskosten_monat"] = _cent(DHF_KOSTEN) // 100    # cent -> euro
            wk_input["monate"] = _cent("dhf_monate")
            wk_input["im_inland"] = True
        # Verpflegung (Stufe 1b, § 9 Abs. 4a): Tage-Kategorien + Mahlzeitenkürzung (S. 8-11).
        # NUR wenn Verpflegungstage > 0 UND (≤ 3 Monate ODER Monate offen, aber nicht > 3).
        # Der Guard _dhf_vpf_grund sperrt bei monate > 3 (S. 6 bleibt offen); hier verdrahten wir
        # alles Rechenbare (Tage, Mahlzeitenzahl, Entgelt, Erstattung). Der Accessor _verpflegung_abzug
        # komposiert die Logik.
        _mon = f.get("vpf_monate_am_ort", {}).get("wert")
        if sum(_cent(t) for t in VERPFLEGUNG_TAGE) > 0:
            # Tage verdrahten, aber Logik bei > 3 Monaten neu: nicht alles sperren, sondern
            # NACH_FRIST-Reduktion versuchen. § 9 Abs. 4a S. 6: bei > 3 Monaten am selben Ort
            # nur die Tage VOR Fristablauf abziehbar.
            if isinstance(_mon, int) and not isinstance(_mon, bool) and _mon > 3:
                # > 3 Monate: nur Tage innerhalb der Frist (Tage_gesamt - Tage_nach_Frist)
                # NACH_FRIST-Felder müssen angegeben sein (Guard prüft fail-closed).
                for t in VERPFLEGUNG_TAGE:
                    wk_input[t] = _cent(t)
                for t_nach in VERPFLEGUNG_TAGE_NACH_FRIST:
                    wk_input[t_nach] = _cent(t_nach)
            else:
                # ≤ 3 Monate oder offen: alle Tage normal
                for t in VERPFLEGUNG_TAGE:
                    wk_input[t] = _cent(t)
            # Mahlzeitenkürzung (S. 8-11): unabhängig von Monaten, wenn Mahlzeitenfrage beantwortet
            for k in ("vpf_fruehstuecke_gestellt_anzahl", "vpf_mittagessen_gestellt_anzahl",
                      "vpf_abendessen_gestellt_anzahl"):
                v = _cent(k)
                if v > 0:
                    wk_input[k] = v
            # Entgelt des Arbeitnehmer (S. 10): Kürzungsminderung
            if _cent("vpf_mahlzeiten_gezahltes_entgelt") > 0:
                wk_input["vpf_mahlzeiten_gezahltes_entgelt"] = _cent("vpf_mahlzeiten_gezahltes_entgelt")
            # Steuerfreie Erstattung (S. 11): Abzugsausschluss
            if _cent("vpf_steuerfreie_erstattung_betrag") > 0:
                wk_input["vpf_steuerfreie_erstattung_betrag"] = _cent("vpf_steuerfreie_erstattung_betrag")
        # Übernachtung Auswärtstätigkeit (Stufe 1b, § 9 Abs. 1 Nr. 5a): NUR bei Inland, allen 3
        # Tatbestands-Bedingungen bestätigt-true UND ohne 48-Monats-Schwellenübertritt (der Guard
        # sperrt sonst); der Accessor kappt nach-48 auf 1.000/Monat. Kosten = cent, Monate = Anzahl.
        _ub_bisher = _cent("uebernachtung_monate_bisher")
        _ub_monate = _cent("uebernachtung_monate")
        if (_cent(UEBERNACHTUNG_KOSTEN) > 0 and f.get("uebernachtung_im_inland", {}).get("wert") is True
                and all(f.get(b, {}).get("wert") is True for b in UEBERNACHTUNG_BEDINGUNGEN)
                and not (_ub_bisher < 48 < _ub_bisher + _ub_monate)):
            wk_input["uebernachtung_kosten_monat"] = _cent(UEBERNACHTUNG_KOSTEN) // 100   # cent -> euro
            wk_input["uebernachtung_monate"] = _ub_monate
            wk_input["uebernachtung_monate_bisher"] = _ub_bisher
        # Arbeitsmittel (A6, § 9 Abs. 1 Nr. 7 i.V.m. § 6 Abs. 2 GWG): NUR GWG-Sofortabzug — AK ≤ 800 EUR
        # (Schwelle in CENT, 80000) UND Wahlrecht ausgeübt. > 800 = mehrjährige § 7-AfA sperrt der Guard.
        if 0 < _cent(ARBEITSMITTEL_KOSTEN) <= 80000 and f.get("am_gwg_sofortabzug_gewaehlt", {}).get("wert") is True:
            wk_input["am_anschaffungskosten"] = _cent(ARBEITSMITTEL_KOSTEN) // 100   # cent -> euro
        # A6-L2 (§ 7 Abs. 1 lineare AfA): AK > 80000 CENT → kein GWG mehr → mehrjährige AfA
        # über die Nutzungsdauer (Jahre). Nutzungsdauer absent → Guard sperrt (fail-closed).
        elif _cent(ARBEITSMITTEL_KOSTEN) > 80000:
            # `_cent`, nicht `_c`: dieser Block wurde aus dem Zweig festzusetzende_est_gesamt
            # kopiert, wo der Feldleser `_c` heißt. Hier hieß er immer `_cent` — der falsche
            # Name warf einen NameError, sobald ein Arbeitsmittel über 800 EUR im an_gesamt-
            # Ring lag (Fix 2026-08-13, Test test_arbeitsmittel_ueber_gwg_schwelle_stuerzt_nicht_ab).
            nd = _cent("arbeitsmittel_nutzungsdauer")
            if nd > 0:
                wk_input["am_anschaffungskosten"] = runner.catala_p7_linear_afa({
                    "anschaffungskosten_cent": _cent(ARBEITSMITTEL_KOSTEN),
                    "nutzungsdauer": nd})  # europe, already euro
        wk = runner.catala_werbungskosten_n(wk_input)   # Person A: EP + dHf + Verpflegung + Übernachtung + AM-GWG, roh
        # § 10 Abs. 1 Nr. 3/3a KV/PV-Vorsorge (Pflicht-Kegel Person A, Gesamt-Parität, Over-tax-Fix):
        # eigener Abs.4-Höchstbetrag (1900/2800), additiv, GETRENNT von der VOR-Basisvorsorge unten.
        kv_pv_a = runner.catala_p10_kv_pv({
            # §10 Abs.1 Nr.3 S.2: Kind-Beiträge in DENSELBEN Abs.4-Deckel (kind_summe CENT, direkt addiert).
            "basis_kv_pv": (_cent("basis_kv") + _cent("basis_pv")
                            + _kind_kv_pv_summe(store, bindung, nur_bestaetigt)) // 100,
            "weitere_vorsorgeaufwendungen": (_cent("vorsorge_arbeitslosenversicherung") + _cent("vorsorge_erwerbsunfaehigkeit") + _cent("vorsorge_unfall_haftpflicht") + _cent("vorsorge_rv_alt_mit_ueberschuss") + _cent("vorsorge_rv_alt_ohne_ueberschuss")) // 100,
            "mit_anspruch_auf_zuschuss": f.get("mit_anspruch_auf_zuschuss", {}).get("wert") is True})
        # Zusammenveranlagung (§ 26b): Roh-Bruttolohn + Roh-WK pro Person -> catala_est_zusammen
        # (Pauschbetrag je Ehegatte + Splitting IM Scope). MVP: Person B ohne gesonderte WK (0),
        # ohne VOR (Partner-VOR sperrt der Guard). Person-B-KV/PV optional (absent -> 0, eigener
        # Höchstbetrag je Person, additiv wie in gesamt) — Person-B-WK/VOR bleiben Folge-Nachträge.
        zusammen = f.get("veranlagung", {}).get("wert") == "zusammen"
        if zusammen:
            kv_pv_b = runner.catala_p10_kv_pv({
                "basis_kv_pv": (_cent("basis_kv_partner") + _cent("basis_pv_partner")) // 100,
                "weitere_vorsorgeaufwendungen": (_cent("vorsorge_arbeitslosenversicherung_partner") + _cent("vorsorge_erwerbsunfaehigkeit_partner") + _cent("vorsorge_unfall_haftpflicht_partner") + _cent("vorsorge_rv_alt_mit_ueberschuss_partner") + _cent("vorsorge_rv_alt_ohne_ueberschuss_partner")) // 100,
                "mit_anspruch_auf_zuschuss": f.get("mit_anspruch_auf_zuschuss_partner", {}).get("wert") is True})
            est = runner.catala_est_zusammen({
                "veranlagungszeitraum": vz,
                "bruttoarbeitslohn_a": int(slots["bruttoarbeitslohn"]) // 100,
                "bruttoarbeitslohn_b": _cent("bruttoarbeitslohn_partner") // 100,
                "werbungskosten_a": wk, "werbungskosten_b": 0,
                "sonderausgaben_gemeinsam": kv_pv_a + kv_pv_b})
        else:
            # § 10 Altersvorsorge (Stufe 1a): die VOR-Einzelfelder DIREKT aus dem Store greifen —
            # der Summen-Slot gesamtbeitraege_inkl_ag würde den AG-Anteil verschmelzen und die
            # Kürzung (nach dem Cap) unmöglich machen. gesamtbeitraege = AN + AG + außerhalb; der
            # steuerfreie AG-Anteil getrennt. Naht-CENT -> EURO für _vorsorge_abzug.
            gesamt = (_cent("vor_an_anteil_rv") + _cent("vor_ag_anteil_rv")
                      + _cent("vor_rv_ausserhalb_lstb")) // 100
            ag = _cent("vor_ag_anteil_rv") // 100
            so = runner._vorsorge_abzug({"vorsorge_gesamtbeitraege_inkl_ag": gesamt,
                                         "vorsorge_ag_anteil_steuerfrei": ag}, vz) + kv_pv_a
            est = runner.catala_est({
                "veranlagungszeitraum": vz,
                "veranlagung": slots["veranlagung"],
                # bruttoarbeitslohn ist Naht-CENT (Bindung typ:cent) -> catala_est erwartet EURO.
                "bruttoarbeitslohn": int(slots["bruttoarbeitslohn"]) // 100,
                "werbungskosten": wk,
                "sonderausgaben": so})
        # SolZ §3, §4 SolzG: Basis = festzusetzende ESt (kein KiFB/§32d-Kapital im AN-Ring)
        if solz_container is not None:
            solz_container[0] = runner.catala_solz({
                "veranlagungszeitraum": vz,
                "bemessungsgrundlage": est,
                "splitting": zusammen})
        # KiSt § 51a: dieselbe Maßstabsteuer wie SolZ (reiner AN-Fall: kein KiFB/§32d)
        if extras is not None:
            extras["kist_cent"] = runner.catala_kist({
                "est_mit_fb": est,
                "konfession": f.get("kist_konfession", {}).get("wert", "keine"),
                "bundesland": f.get("kist_bundesland", {}).get("wert", "")})
        # § 101 Mobilitätsprämie (Post-Engine-Prämie, KEIN ESt-Impact; extras-Naht wie KiSt).
        # Stufe-1: reiner-AN einzel mit Pendlerstrecke (ab-21km-EP). zusammen = Stufe-2 (per-
        # Ehegatte-S.3 + doppelter GFB, S.2 Hs.2). Ohne Entfernung → kein § 101 (Feld absent →
        # mobilitaetspraemie_cent bleibt None). Prämie zahlt nur bei zvE < GFB (Accessor: 0 sonst).
        if (extras is not None and not zusammen
                and int(slots["entfernung_km_roh"]) > 0):
            ep_ab_21 = runner.catala_ep_ab_21km({
                "veranlagungszeitraum": vz,
                "arbeitstage": int(slots["arbeitstage"]),
                "entfernung_km_roh": int(slots["entfernung_km_roh"]),
                "eigenes_oder_ueberlassenes_kfz": bool(slots["eigenes_oder_ueberlassenes_kfz"]),
                "oepnv_kosten_jahr": _oepnv_eur(slots)})
            extras["mobilitaetspraemie_cent"] = runner.catala_p101_mobilitaetspraemie_cent({
                "entfernungspauschale_ab_21km": ep_ab_21,
                "zu_versteuerndes_einkommen": runner.catala_est_einzel_zve({
                    "veranlagungszeitraum": vz,
                    "bruttoarbeitslohn": int(slots["bruttoarbeitslohn"]) // 100,
                    "werbungskosten": wk, "sonderausgaben": so}),
                "grundfreibetrag": runner.catala_grundfreibetrag(vz),
                "ist_arbeitnehmer": True,                 # § 101 S. 3 (AN-Pauschbetrag-soweit)
                "werbungskosten_gesamt": wk,              # roh AN-WK inkl. voller EP
                "arbeitnehmer_pauschbetrag": runner.catala_arbeitnehmer_pauschbetrag(vz)})
        return est
    return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="festzusetzende_est")


def _zweig_festzusetzende_est_gesamt(vz: int, bindung: dict, felder, store, nur_bestaetigt: bool,
                                    solz_container, extras):
    """§ 21 V+V via catala_gesamt — Scheibe gesamt, der breiteste Rechenweg.

    Aus _bescheid_fn herausgezogen (Refactor 2026-08-13). `felder` ist der bei
    nur_bestaetigt=True bereits auf zustand=bestaetigt gefilterte Snapshot — die Filterung
    bleibt im Kopf von _bescheid_fn, damit die Zwei-Signal-Invariante EINE Stelle behält
    statt je Zweig wiederholt zu werden."""
    try:
        import runner  # noqa: F401
    except Exception:
        return None
    f = felder or {}

    def _c(fid):
        v = f.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

    def _vv_objekt(fi: dict) -> int:
        # § 21 Überschuss EINES Objekts (Einnahmen − Werbungskosten), Naht-CENT -> EURO. KEIN per-
        # Objekt-Floor (catala_vermietung_einkuenfte gibt Verluste durch → horizontaler Verlustausgleich
        # innerhalb § 21; der Floor kommt erst im gesamt-Scope, § 2 Abs. 3). fi = das je-Instanz auf die
        # Basis-feld_id normierte Feld-Dict (instanzen-Naht) ODER die felder-closure f (Basis-Fallback).
        def _ci(k):
            v = fi.get(k, {}).get("wert")
            return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
        def _bi(k):
            return fi.get(k, {}).get("wert")
        vv_voll = runner.catala_vermietung_einkuenfte({
            "einnahmen": _ci("vv_einnahmen") // 100,
            "gebaeude_afa": _ci("vv_gebaeude_afa") // 100,
            "schuldzinsen": _ci("vv_schuldzinsen") // 100,
            "erhaltungsaufwand": _ci("vv_erhaltungsaufwand") // 100,
            "sonstige_werbungskosten": _ci("vv_sonstige_wk") // 100})
        # § 21 Abs. 2: verbilligte Wohnraumvermietung (Entgelt < 66 % ortsüblich) → WK nur anteilig (× quote/
        # 100), das erhöht die § 21-Einkünfte (mehr Steuer, K2-sichere Richtung). NUR bei Tatbestand Wohnzwecke
        # + auf Dauer (§ 21 Abs. 2 S. 1); gewerblich/nicht-dauer (Feld = False) → volle WK. quote 100 (default)
        # ≥ 66 → keine Kürzung. PER OBJEKT (jede Instanz eigene Quote/Tatbestand). Umgesetzt als Add-back der
        # nicht abziehbaren WK auf den Voll-Überschuss (behält catala_vermietung_einkuenfte + dessen Konsistenz-
        # Gate im Pfad). quote im Pflicht-Kegel → immer beantwortet (kein stiller Unter-Abzug).
        # C-Fix (K2, Under-tax): quote=0 = UNENTGELTLICHE Überlassung (keine Einkünfteerzielungsabsicht, § 21
        # greift nicht) → dieses Objekt trägt seine Einnahmen OHNE WK-Abzug bei = kein WK-Verlust der die § 19
        # senkt. Vorher kollabierte `_ci or 100` die 0 auf 100 → voller WK-Verlust = Under-tax. Robust ggü.
        # Extrem-verbilligt (Mini-Miete, quote ganzzahlig auf 0 gerundet, einnahmen > 0): Einnahmen STATT hart 0
        # (kein neuer Under-tax; bei einnahmen 0 = die reine unentgeltliche Überlassung → 0). absent (nur Alt-
        # Aufrufer/Teil-Ring, im echten Ring nie: Pflicht-Kegel) → 100 (nicht verbilligt).
        quote_raw = _bi("vv_entgelt_quote_prozent")
        quote_present = isinstance(quote_raw, (int, float)) and not isinstance(quote_raw, bool)
        if quote_present and quote_raw == 0:
            return _ci("vv_einnahmen") // 100   # unentgeltlich → Einnahmen ohne WK, kein Verlust
        quote = int(quote_raw) if quote_present else 100
        tatbestand = (_bi("vv_wohnzwecke") is not False) and (_bi("vv_auf_dauer") is not False)
        if tatbestand and 0 < quote < 66:
            wk_voll = (_ci("vv_gebaeude_afa") + _ci("vv_schuldzinsen")
                       + _ci("vv_erhaltungsaufwand") + _ci("vv_sonstige_wk")) // 100
            wk_abziehbar = runner.catala_p21_2_verbilligt({
                "werbungskosten": wk_voll, "entgelt_quote_prozent": quote})
            return vv_voll + (wk_voll - wk_abziehbar)   # nicht abziehbare WK zurück → höhere Einkünfte
        return vv_voll

    def slot_fn(slots: dict) -> int:
        # § 21 Überschuss je Objekt, dann STUMPFE Σ über ALLE vv_objekt-Instanzen (Multi-Objekt, #5):
        # est_mapping.instanzen (dev-2s Ring-Naht, EINE Enumerations-Wahrheit — index==1 = Basis-vv-Felder,
        # __n = weitere Objekte, je Instanz auf die Basis-feld_id normiert). Ohne store (Teil-Ring/Alt-
        # Aufrufer) nur die Basis aus f. Unvollständige Instanzen fängt der Guard VOR diesem Aufruf ab.
        if store is not None:
            # Zwei-Signal-Filter (Instanz-Pfad, nur_bestaetigt=True): vorläufige Objekt-Instanz nie in die Σ.
            # Defense-in-depth — der vv_instanz_offen-Guard (_an_gesamt_sperrgrund) sperrt index≥2-vorläufig schon
            # VOR diesem Aufruf, die Basis liegt im Kegel-Meet — aber konsistent zum gwg-Σ (refactor-sicher).
            # nur_bestaetigt=False (/stand) zeigt die vorläufige Wirkung im Range.
            vv = sum(_vv_objekt(inst["felder"])
                     for inst in EM.instanzen(store, bindung, "vv_objekt")
                     if not nur_bestaetigt or inst["zustand"] == "bestaetigt")
        else:
            vv = _vv_objekt(f)
        g = {"gesamtfall": True, "veranlagungszeitraum": vz,
             "veranlagung": slots["veranlagung"],
             "einkuenfte_vermietung": vv}
        # kombiniert §19+§21: Bruttolohn im Kegel -> §19-Einkünfte (§9a-bereinigt, § 2 Abs. 2 Nr. 2)
        # als einkuenfte_nichtselbststaendig in die §-2-Summe; der §21-Verlust mindert dann den
        # §19-Lohn (§ 2 Abs. 3). Bruttolohn 0 (reiner Vermieter) -> einkuenfte_ns 0, kein Effekt.
        # §19-WK = Entfernungspauschale (roh, § 9a-Günstiger im einzel-Tarif) + dHf + Verpflegung + Über-
        # nachtung + Arbeitsmittel-GWG (B1/A5/A6: gemischt-§19-Fall bekommt dieselben §9-WK wie der reine
        # an_gesamt-Ring, sonst Over-tax). Der mehrjährige AM-AfA-Zweig (> 800) sperrt hier wie an_gesamt.
        # Klasse-3 fail-open geschlossen (2026-08-09): dieselben 4 Keys sind auch im
        # Pflicht-Kegel von gesamt (EP_FELDER) — "if k in slots" liess einen fehlenden Key
        # lautlos aus gesamt_wk_input verschwinden statt zu werfen; gemessen 351 EUR stille
        # Steuermehrbelastung ohne KeyError (reports/adjudikation/klasse3_fail_open_2026-08-09.md).
        gesamt_wk_input = {"veranlagungszeitraum": vz,
            **{k: slots[k] for k in
               ("arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr", "eigenes_oder_ueberlassenes_kfz")}}
        if "oepnv_kosten_jahr" in gesamt_wk_input:
            gesamt_wk_input["oepnv_kosten_jahr"] = _oepnv_eur(gesamt_wk_input)   # Naht-CENT -> EURO
        # doppelte Haushaltsführung (B1, Parität an_gesamt): dHf-Roh-WK NUR bei erfülltem Tatbestand
        # (Kosten>0, Inland, alle 4 Bedingungen bestätigt-true). Offener/Ausland-Tatbestand sperrt der
        # SHARED _an_gesamt_sperrgrund (gesamt guard=True) → hier doppelt sicher gegen Über-Abzug.
        if (_c(DHF_KOSTEN) > 0 and f.get("dhf_im_inland", {}).get("wert") is True
                and all(f.get(b, {}).get("wert") is True for b in DHF_BEDINGUNGEN)):
            gesamt_wk_input["unterkunftskosten_monat"] = _c(DHF_KOSTEN) // 100    # cent -> euro
            gesamt_wk_input["monate"] = _c("dhf_monate")
            gesamt_wk_input["im_inland"] = True
        # Verpflegung (B1, Parität an_gesamt, § 9 Abs. 4a): Tage-Kategorien + Mahlzeitenkürzung (S. 8-11).
        # NUR wenn Verpflegungstage > 0 UND (≤ 3 Monate ODER Monate offen, aber nicht > 3).
        # Der SHARED _an_gesamt_sperrgrund sperrt bei monate > 3 (S. 6 bleibt offen); hier verdrahten wir
        # alles Rechenbare (Tage, Mahlzeitenzahl, Entgelt, Erstattung). Der Accessor _verpflegung_abzug
        # komposiert die Logik.
        _mon = f.get("vpf_monate_am_ort", {}).get("wert")
        if sum(_c(t) for t in VERPFLEGUNG_TAGE) > 0:
            # Tage verdrahten, aber Logik bei > 3 Monaten neu: nicht alles sperren, sondern
            # NACH_FRIST-Reduktion versuchen. § 9 Abs. 4a S. 6: bei > 3 Monaten am selben Ort
            # nur die Tage VOR Fristablauf abziehbar.
            if isinstance(_mon, int) and not isinstance(_mon, bool) and _mon > 3:
                # > 3 Monate: nur Tage innerhalb der Frist (Tage_gesamt - Tage_nach_Frist)
                # NACH_FRIST-Felder müssen angegeben sein (Guard prüft fail-closed).
                for t in VERPFLEGUNG_TAGE:
                    gesamt_wk_input[t] = _c(t)
                for t_nach in VERPFLEGUNG_TAGE_NACH_FRIST:
                    gesamt_wk_input[t_nach] = _c(t_nach)
            else:
                # ≤ 3 Monate oder offen: alle Tage normal
                for t in VERPFLEGUNG_TAGE:
                    gesamt_wk_input[t] = _c(t)
            # Mahlzeitenkürzung (S. 8-11): unabhängig von Monaten, wenn Mahlzeitenfrage beantwortet
            for k in ("vpf_fruehstuecke_gestellt_anzahl", "vpf_mittagessen_gestellt_anzahl",
                      "vpf_abendessen_gestellt_anzahl"):
                v = _c(k)
                if v > 0:
                    gesamt_wk_input[k] = v
            # Entgelt des Arbeitnehmer (S. 10): Kürzungsminderung
            if _c("vpf_mahlzeiten_gezahltes_entgelt") > 0:
                gesamt_wk_input["vpf_mahlzeiten_gezahltes_entgelt"] = _c("vpf_mahlzeiten_gezahltes_entgelt")
            # Steuerfreie Erstattung (S. 11): Abzugsausschluss
            if _c("vpf_steuerfreie_erstattung_betrag") > 0:
                gesamt_wk_input["vpf_steuerfreie_erstattung_betrag"] = _c("vpf_steuerfreie_erstattung_betrag")
        # Übernachtung Auswärtstätigkeit (B1/A5, § 9 Abs. 1 Nr. 5a): Parität an_gesamt — NUR bei
        # Inland, allen 3 Bedingungen bestätigt-true UND ohne 48-Monats-Schwellenübertritt (Guard
        # sperrt sonst); Accessor kappt nach-48 auf 1.000/Monat. Kosten = cent, Monate = Anzahl.
        _ub_bisher = _c("uebernachtung_monate_bisher")
        _ub_monate = _c("uebernachtung_monate")
        if (_c(UEBERNACHTUNG_KOSTEN) > 0 and f.get("uebernachtung_im_inland", {}).get("wert") is True
                and all(f.get(b, {}).get("wert") is True for b in UEBERNACHTUNG_BEDINGUNGEN)
                and not (_ub_bisher < 48 < _ub_bisher + _ub_monate)):
            gesamt_wk_input["uebernachtung_kosten_monat"] = _c(UEBERNACHTUNG_KOSTEN) // 100    # cent -> euro
            gesamt_wk_input["uebernachtung_monate"] = _ub_monate
            gesamt_wk_input["uebernachtung_monate_bisher"] = _ub_bisher
        # Arbeitsmittel (A6, Parität an_gesamt): NUR GWG-Sofortabzug — AK ≤ 800 EUR (CENT-Schwelle 80000)
        # UND Wahlrecht ausgeübt. > 800 = mehrjährige § 7-AfA sperrt der SHARED _an_gesamt_sperrgrund.
        if 0 < _c(ARBEITSMITTEL_KOSTEN) <= 80000 and f.get("am_gwg_sofortabzug_gewaehlt", {}).get("wert") is True:
            gesamt_wk_input["am_anschaffungskosten"] = _c(ARBEITSMITTEL_KOSTEN) // 100    # cent -> euro
        # § 7 Abs. 1 Lineare AfA (>800€): Nutzungsdauer MUSS beantwortet sein.
        # Monat + Zustand-Flag: nur wenn Anschaffungsjahr=true (S. 4 Zwölftelung).
        # Flag unbeantwortet → Folgejahr angenommen (voller Jahresbetrag).
        am_afa_betrag = 0
        if _c(ARBEITSMITTEL_KOSTEN) > 80000:
            nd = _c("arbeitsmittel_nutzungsdauer")
            ist_aj = f.get("am_afa_ist_anschaffungsjahr", {}).get("wert")
            if nd > 0:
                if ist_aj is True:
                    # Anschaffungsjahr: Monat MUSS beantwortet sein für S. 4 Zwölftelung
                    am = _c("am_anschaffung_monat")
                    if am > 0:
                        am_afa_betrag = runner.catala_p7_linear_afa({
                            "anschaffungskosten": _c(ARBEITSMITTEL_KOSTEN) // 100,
                            "nutzungsdauer": nd,
                            "anschaffung_monat": am,
                            "ist_anschaffungsjahr": True})
                else:
                    # Folgejahr oder Flag unbeantwortet: voller Jahresbetrag, Monat egal
                    am_afa_betrag = runner.catala_p7_linear_afa({
                        "anschaffungskosten": _c(ARBEITSMITTEL_KOSTEN) // 100,
                        "nutzungsdauer": nd,
                        "ist_anschaffungsjahr": False})
        ns_wk = runner.catala_werbungskosten_n(gesamt_wk_input)
        ns_wk += am_afa_betrag  # § 7 AfA addieren (Accessor-Betrag in EUR)
        # § 19 Abs. 2 Versorgungsfreibetrag (K2): Gate entscheidet über Behandlung.
        # cent-Felder (jahresrente, bemessungsgrundlage) sind CENT, Accessor rechnet EURO.
        versorgung_jahresrente_cent = _c("versorgung_jahresrente")
        versorgung_bemessungsgrundlage_cent = _c("versorgung_bemessungsgrundlage")
        versorgung_beginn_jahr = _c("versorgung_beginn_jahr")
        versorgung_art = f.get("versorgung_art", {}).get("wert")
        versorgung_alter = _c("versorgung_alter_bei_beginn")
        # § 19 Abs. 2 S. 2 Nr. 2 Alters-Gate: nur bei altersgrenze_sonstige prüfen.
        # Nicht-beamtenrechtliche Bezüge zählen nur ab 63. Lj (oder 60. Lj bei Schwerbehinderung, GdB ≥ 50).
        alters_gate_erfuellt = True
        if versorgung_art == "altersgrenze_sonstige" and versorgung_alter > 0:
            gdb = _c("rentner_grad_der_behinderung")
            alters_grenze = 60 if gdb >= 50 else 63
            if versorgung_alter < alters_grenze:
                alters_gate_erfuellt = False
        # Gate entscheidet über Behandlung: erfüllt → VFB-Weg (102€ Pauschbetrag Nr. 1b),
        # nicht erfüllt → Arbeitslohn-Weg (1.230€ Pauschbetrag Nr. 1a).
        bruttoarbeitslohn_basis = int(slots["bruttoarbeitslohn"]) // 100   # Naht-CENT -> EURO
        if versorgung_jahresrente_cent > 0 and versorgung_bemessungsgrundlage_cent > 0 and versorgung_beginn_jahr > 0:
            if not alters_gate_erfuellt:
                # Gate nicht erfüllt: Bezüge als normale Arbeitseinkünfte (§ 19 Abs. 1) addieren.
                # Der Pauschbetrag (Nr. 1a, 1.230€ insgesamt) wirkt über catala_einkuenfte_nichtselbststaendig
                # auf die SUMME bruttolohn + versorgungsbezüge.
                bruttoarbeitslohn_basis += versorgung_jahresrente_cent // 100  # CENT → EURO
        ns = runner.catala_einkuenfte_nichtselbststaendig({
            "veranlagungszeitraum": vz,
            "bruttoarbeitslohn": bruttoarbeitslohn_basis,
            "werbungskosten": ns_wk})
        # Person B (§ 26b Zusammenveranlagung, #4): die § 19-Einkünfte des Ehegatten (§9a-bereinigt JE
        # PERSON) in DIESELBE einkuenfte_nichtselbststaendig-Summe — kein _a/_b-Split, der Gesamt-Scope
        # rechnet Splitting + doppelten § 10c aus veranlagung=zusammen (handverifiziert: gesamt(zusammen,
        # ns_A+ns_B) == catala_est_zusammen(brutto_A, brutto_B)). Person-B-WK MVP 0 (Folge-Nachtrag).
        if g["veranlagung"] == "zusammen":
            ns += runner.catala_einkuenfte_nichtselbststaendig({
                "veranlagungszeitraum": vz,
                "bruttoarbeitslohn": _c("bruttoarbeitslohn_partner") // 100, "werbungskosten": 0})
        # § 19 Abs. 2 Gate erfüllt: VFB + Zuschlag + 102€ Pauschbetrag (Nr. 1b)
        if versorgung_jahresrente_cent > 0 and versorgung_bemessungsgrundlage_cent > 0 and versorgung_beginn_jahr > 0:
            if alters_gate_erfuellt:
                vers_einkuenfte = runner.catala_einkuenfte_versorgung({
                    "versorgung_jahresrente": versorgung_jahresrente_cent // 100,       # CENT → EURO
                    "versorgung_bemessungsgrundlage": versorgung_bemessungsgrundlage_cent // 100,  # CENT → EURO
                    "versorgung_beginn_jahr": versorgung_beginn_jahr})
                ns += vers_einkuenfte
        g["einkuenfte_nichtselbststaendig"] = ns
        # §§ 13-18 Gewinneinkünfte (Stufe 1 + 2a): der laufende Gewinn als einkuenfte_gewinn-Summand in die
        # § 2-Summe (Engine-Slot einkuenfte_gewinn_in LIVE, runner.py _gesamt_out). _laufender_gewinn wählt
        # Stufe 2a (EÜR § 4 Abs. 3, betriebseinnahmen − BA komponentenweise) wenn eine EÜR-Komponente vorliegt,
        # sonst den direkten Stufe-1-Betrag. OPTIONAL (absent → 0, over-tax-safe). Doppelquelle/land_forst +
        # Einkunftsart-Konsistenz (kein_gewinn=True) fangen gewinn_quelle_offen/luf_euer_offen/flag_check vorher.
        # § 16 Veräußerungsgewinn (Non-Rentner-§16-vg, REUSE des generellen §16-vg-Felds): netto nach § 16 Abs. 4-
        # Freibetrag (catala_p16_4_freibetrag + rentner-2-I-Fold gespiegelt; der „rentner_"-Feldname ist NICHT
        # rentner-spezifisch, Kz Anlage G/S). GEFLOORT bei 0 (FB > vg → kein Phantom-Verlust) + ADDITIV in
        # einkuenfte_gewinn (§ 16 Abs. 1: Veräußerungs- + laufender Gewinn = dieselbe § 2-Einkunftsart). Absent → 0.
        vg_euro = _c("rentner_veraeusserungsgewinn") // 100
        netto_vg = max(0, vg_euro - runner.catala_p16_4_freibetrag({"rentner_veraeusserungsgewinn": vg_euro}))
        laufender_gewinn, mitu = _laufender_gewinn(f, store, bindung, nur_bestaetigt)   # § 15/§ 18 laufend (für § 35-Zähler, OHNE § 16-vg)
        # § 26b: bei Zusammenveranlagung kommen die Gewinneinkünfte des Ehegatten hinzu (Stufe 2
        # der Partnerachse, 2026-08-13). Bis dahin wurde der Partner-Gewinn zwar deklariert und
        # übermittelt, aber nicht besteuert — die angezeigte Steuer war zu niedrig.
        gewinn_partner, _mitu_partner = _gewinn_partner_anteil(f)
        g["einkuenfte_gewinn"] = laufender_gewinn + netto_vg + gewinn_partner
        # § 24a/§ 24b Freibeträge (Weg ii Stage 2, § 2 Abs. 3 — MINDERN den GdE VOR den Abzügen): § 24a
        # Altersentlastungsbetrag (§24a S.1: Arbeitslohn BRUTTO + max(0, positive Summe der Nicht-§19-Einkünfte =
        # V+V + §§13-18-Gewinn; Leibrenten/Versorgungsbez. raus S.2; Kohorten-Satz/-Deckel aus geburtsjahr + 65;
        # §20-tarifl.-Kapital-Günstiger = seltener Nachtrag) + § 24b Entlastungsbetrag Alleinerziehende (fam_alleinstehend IST das §24b-Abs.3-Flag — quelle
        # p24b/alleinstehend, fragetext „ohne anderen Erwachsenen im Haushalt"; anzahl_kinder + monate). Absent
        # → 0 (fail-safe). Beide fließen in den GdE-Zwilling (echte GdE post § 24a/§24b für die §10b/§33-
        # Deckelung) UND in g (est).
        alt24a = runner.catala_p24a_altersentlastung({
            "veranlagungszeitraum": vz,   # § 24a S. 3 64+-Gate (geburtsjahr+65 ≤ VZ)
            "geburtsjahr": _c("geburtsjahr"),
            "arbeitslohn": _c("bruttoarbeitslohn") // 100,
            # §23 (§22Nr.2) gehört IN die §24a-Bemessung (S.2-Ausschluss nennt nur §22Nr.1/4/5)
            "positive_andere_einkuenfte": max(0, vv + g["einkuenfte_gewinn"]
                                               + g.get("einkuenfte_sonstige", 0))})
        # § 24a PER PERSON (§ 24a S. 1 „der Steuerpflichtige", A.2): bei Zusammenveranlagung hat der Ehegatte
        # eine EIGENE Kohorte (geburtsjahr_partner + 65) + eigene Bemessung (bruttoarbeitslohn_partner; positive
        # andere Einkünfte-B = 0, da vv/Kapital im Ring nicht owner-getrennt = konservativ/over-tax-safe, mit
        # dev-2 abgestimmt). Absent (geburtsjahr_partner ≤ 0) → 0 (fail-safe). Additiv zu Person A.
        alt24a_b = runner.catala_p24a_altersentlastung({
            "veranlagungszeitraum": vz,   # § 24a S. 3 64+-Gate (Person B: geburtsjahr_partner+65 ≤ VZ)
            "geburtsjahr": _c("geburtsjahr_partner"),
            "arbeitslohn": _c("bruttoarbeitslohn_partner") // 100,
            "positive_andere_einkuenfte": 0}) if g["veranlagung"] == "zusammen" else 0
        ent24b = runner.catala_p24b_entlastung({
            "alleinstehend": f.get("fam_alleinstehend", {}).get("wert") is True,
            "anzahl_kinder": _c("fam_anzahl_kinder"),
            "monate_ohne_voraussetzung": _c("fam_monate_ohne_voraussetzung")})
        g["altersentlastungsbetrag"] = alt24a + alt24a_b
        g["entlastungsbetrag_alleinerziehende"] = ent24b
        # § 10 Abs. 1 Nr. 2/Abs. 3 Altersvorsorge (Basisvorsorge RV): die 3 VOR-Felder DIREKT aus dem Store —
        # gesamtbeitraege = AN + AG + außerhalb LStB, der steuerfreie AG-Anteil getrennt (Kürzung NACH dem
        # knappschaft-Höchstbetrag-Cap). catala_gesamt ruft _vorsorge_abzug(s) SCHON intern (runner.py Z.759,
        # addiert auf sonderausgaben nach _sonderausgaben_final) → hier nur die Slots setzen, kein Doppelzählen.
        # Naht-CENT → EURO. Absent → 0 (im Pflicht-Kegel, also immer gefragt: kein stiller Über-tax). Person-B-
        # VOR (vor_*_partner) + zusammen-VOR = Nachtrag wie an_gesamt (MVP Person-A-Altersvorsorge).
        g["vorsorge_gesamtbeitraege_inkl_ag"] = (_c("vor_an_anteil_rv") + _c("vor_ag_anteil_rv")
                                                 + _c("vor_rv_ausserhalb_lstb")) // 100
        g["vorsorge_ag_anteil_steuerfrei"] = _c("vor_ag_anteil_rv") // 100
        # Person-B-Basisvorsorge (§ 10 Abs. 3, A.2): bei Zusammenveranlagung die vor_*_rv_partner ADDITIV in
        # dieselben Summen-Slots (catala_gesamt/_vorsorge_abzug deckelt EINMAL, kürzt den AG-Anteil). ⚠ RESIDUAL
        # (gemeldet): der Höchstbetrag (_vorsorge_hb) wird bei zusammen NICHT verdoppelt (§ 10 Abs. 3 S. 3) →
        # nur Hoch-RV-Paare mit kombinierten Beiträgen > ~27566 € leicht unter-abgezogen (over-tax-safe, selten).
        if g["veranlagung"] == "zusammen":
            g["vorsorge_gesamtbeitraege_inkl_ag"] += (_c("vor_an_anteil_rv_partner")
                + _c("vor_ag_anteil_rv_partner") + _c("vor_rv_ausserhalb_lstb_partner")) // 100
            g["vorsorge_ag_anteil_steuerfrei"] += _c("vor_ag_anteil_rv_partner") // 100
        # Sonder-Abzüge (Weg ii, Faltung): §35a → steuerermaessigungen, §10b + §10-KiSt → sonderausgaben,
        # §33-agB → aussergewoehnliche_belastungen — ADDITIV auf JEDE Einkunfts-Kombi (§19+§21+§20 zusammen
        # MIT §35a/§10b/§33 in EINEM Bescheid). GdE (§2 Abs.3 = ns+vv+gewinn+sonstige − §24a − §24b = ALLE tarifl. Arten, VOR den Abzügen fest
        # §2 Abs.3-vor-Abs.4 → kein Zirkel) = Basis der §10b-20%-Deckelung + §33-zumutbar-Staffel (Korrektheit
        # vs. §19-only der Sonder-Scheiben). Absente Abzugs-Felder → 0 (fail-SAFE: über-, nie unterbesteuert).
        # rechnung_unbar=false nullt §35a Abs.2/3 (Minijob unberührt); fam_anzahl_kinder/splitting → zumutbar.
        # einkuenfte_gewinn (§§13-18) + einkuenfte_sonstige (§22) SIND jetzt in der §10b/§33-GdE (§33-K2-Fix:
        # §2 Abs.3 SdE = alle tarifl. Arten). NUR §32d-Kapital bewusst RAUS (§2 Abs.5b Abgeltung). Guard fängt K2.
        gde = runner.catala_gesamt_gde({"veranlagungszeitraum": vz, "veranlagung": g["veranlagung"],
                                        "einkuenfte_nichtselbststaendig": ns, "einkuenfte_vermietung": vv,
                                        "einkuenfte_gewinn": g["einkuenfte_gewinn"],
                                        "einkuenfte_sonstige": g.get("einkuenfte_sonstige", 0),
                                        "altersentlastungsbetrag": alt24a + alt24a_b,
                                        "entlastungsbetrag_alleinerziehende": ent24b})
        # § 10 Abs. 4b S. 3: übersteigt die erstattete die gezahlte Kirchensteuer, wird der
        # Überhang dem GdE hinzugerechnet. Hier auf die eben berechnete gde, damit der
        # § 10b-20%-Deckel und die § 33-zumutbare Belastung sich am erhöhten Betrag bemessen.
        # Die Hinzurechnung in den ENGINE-Slot passiert bei einkuenfte_sonstige unten — hier
        # wäre sie tot, weil die Zeile dort das Feld neu setzt (§23/§22 Nr.3).
        kist_ueberhang = runner.catala_p10_4b_erstattungsueberhang({
            "gezahlte_kirchensteuer": _c("kist_gezahlt") // 100,
            "erstattete_kirchensteuer": _c("kist_erstattet") // 100})
        gde += kist_ueberhang
        # §33b Behinderten-/Pflege-/Hinterbliebenen-Pauschbetrag Person A (additiv zu §33-agB).
        # Steht VOR _shared_steuer_sonder_agb, weil ausserg dort benötigt wird.
        # 1:1 gespiegelt vom rentner-Ring (api.py:1002-1010). Felder = rentner_-globale IDs
        # (Kz E0109708/etc. über rentner-Bindung, selbe Felder wie rentner-Scheibe). Absent→0 (safe).
        ausserg = (runner.catala_behinderten_pb({
                       "veranlagungszeitraum": vz, "grad_der_behinderung": _c("rentner_grad_der_behinderung"),
                       "ist_hilflos_blind_taubblind": f.get("rentner_hilflos_blind_taubblind", {}).get("wert") is True})
                   + runner.catala_pflege_pb({
                       "veranlagungszeitraum": vz, "pflegegrad": _c("rentner_pflegegrad"),
                       "ist_hilflos": f.get("rentner_gepflegter_hilflos", {}).get("wert") is True})
                   + runner.catala_hinterbliebenen_pb({
                       "veranlagungszeitraum": vz,
                       "hat_hinterbliebenenbezuege": f.get("rentner_hinterbliebenenbezuege", {}).get("wert") is True}))
        # §33b Abs.5 Kind-PB-Übertragung: per Kind, nur wenn Antrag +
        # Kind-nimmt-nicht + kind_idnr (kumulativ). S.4-Ausschluss greift
        # in _shared_steuer_sonder_agb (agb_cent-Kürzung).
        # Kind-PB additiv zu Person-A/B-PB (eigener Abzugstatbestand).
        ausserg += _p33b_kind_pauschbetraege(store, bindung, nur_bestaetigt, vz)
        # Person-B-§33b: eigener Behinderten-Pauschbetrag des Ehegatten additiv (1:1 Rentner-Präzedenz
        # api.py:1015-1018). Nur Zusammenveranlagung. Pflege-/Hinterbliebenen-PB für Person B nicht
        # modelliert (wie rentner). Felder = rentner_*-globale IDs.
        if g["veranlagung"] == "zusammen":
            ausserg += runner.catala_behinderten_pb({
                "veranlagungszeitraum": vz, "grad_der_behinderung": _c("rentner_grad_der_behinderung_partner"),
                "ist_hilflos_blind_taubblind": f.get("rentner_hilflos_blind_taubblind_partner", {}).get("wert") is True})
        # Abzüge §35a/§35c + Sonderausgaben + agB (via _shared)
        _shared_steuer_sonder_agb(g, gde, ausserg, g["veranlagung"],
                                  f, vz, store, bindung, nur_bestaetigt)
        # Kapital § 20/§ 32d: SINGLE-SOURCE (Instructor-Q1) — E1900701-Aggregat XOR Verlust-Töpfe;
        # Co-Okkurrenz sperrt der Guard (kapital_semantik_offen). Töpfe (§ 20 Abs. 6, per-Topf-Floor)
        # → verrechnete; sonst das Aggregat. Dann Sparer-PB (§ 20 Abs. 9). kapitaleinkuenfte ist UNABHÄNGIG
        # vom § 31-Kinderfreibetrag (§ 2 Abs. 5b/Abs. 6) → EINMAL vorab, vor der § 31-Verzweigung.
        #
        # Zusammenveranlagung für § 20 kommt AUSSCHLIESSLICH aus der Veranlagungsart (§ 26).
        # Bis 2026-07-30 gab es ein zweites Feld kap_zusammenveranlagung, das dieselbe Frage
        # stellte und ihr widersprechen konnte: bei veranlagung=einzel + Flag=true wurde der
        # Sparer-Pauschbetrag verdoppelt, ohne das Partner-Kapital zu addieren — 250 € zu wenig
        # Steuer bei 4.000 € Kapital. Das Feld ist entfernt; eine Kapital-Veranlagung getrennt
        # von der allgemeinen Veranlagungsart kennt § 26 EStG nicht.
        zusammen = g["veranlagung"] == "zusammen"
        kapitaleinkuenfte = _p20_kapitaleinkuenfte(_c, zusammen, vz)

        # §23 Private Veräußerungsgeschäfte (Stufe-1): Σ über Instanzen → ADDITIV in einkuenfte_sonstige
        g["einkuenfte_sonstige"] = _p23_ansonsten_einkuenfte(f, store, bindung, nur_bestaetigt)
        # §22 Nr.3 Sonstige Leistungen (A8): per Accessor mit Freigrenze 256€ → ADDITIV in einkuenfte_sonstige
        # Accessor gibt CENT, einkuenfte_sonstige erwartet EURO → //100.
        nr3 = _c("p22_nr3_einkuenfte")
        if nr3:
            g["einkuenfte_sonstige"] += runner.catala_p22_nr3_einkuenfte(nr3) // 100
        # § 10 Abs. 4b S. 3 KiSt-Erstattungsüberhang: „ist dem Gesamtbetrag der Einkünfte
        # hinzuzurechnen". Der Engine-GdE entsteht aus den Einkunfts-Slots, es gibt keinen
        # eigenen Hinzurechnungs-Slot — die Aufstockung läuft deshalb über einkuenfte_sonstige
        # (§ 2 Abs. 3-Summand). Erst HIER, nach den §23-/§22-Nr.3-Zuweisungen oben, sonst
        # überschreibt Z. 768 den Zuschlag wieder. Der lokale `gde` (§10b/§33-Deckel) wurde
        # schon oben erhöht; gde_p10d unten liest einkuenfte_sonstige und bekommt ihn hier.
        g["einkuenfte_sonstige"] += kist_ueberhang

        # § 10d Abs. 2 Verlustvortrag (opt-in via verlustvortrag_bestand): der festgestellte verbleibende Verlust-
        # vortrag mindert den GdE „VORRANGIG vor Sonderausgaben, agB, sonstigen Abzugsbeträgen" (§ 10d Abs. 2 S. 1)
        # → Fold in sonstige_abzuege_vom_einkommen (§ 2 Abs. 5-Rest-Slot). Die zvE-Kette ist rein linear OHNE Floor
        # (einkommensteuertarif:494-514) → die Reihenfolge ist wertgleich zu „vorrangig vor". Der Höchstbetrag
        # (catala_p10d_2, min(GdE, Sockel+70%-Überstieg), gefixt sha 294cdd6a) braucht die VOLLE GdE (alle
        # tariflichen Einkunftsarten INKL. §§ 13-18-Gewinn; § 32d-Kapital § 2 Abs. 5b-exkl.) — jetzt IDENTISCH zur
        # §10b/§33-gde oben (§33-K2-Fix: beide voll ns+vv+gewinn+sonstige; der frühere Gap ist geschlossen). absent → 0
        # (over-tax-safe). Steht VOR dem § 35 (der § 35-Deckel-3 nutzt die post-§10d geminderte tarifliche ESt).
        gde_p10d = runner.catala_gesamt_gde({
            "veranlagungszeitraum": vz, "veranlagung": g["veranlagung"],
            "einkuenfte_nichtselbststaendig": ns, "einkuenfte_vermietung": vv,
            "einkuenfte_gewinn": g["einkuenfte_gewinn"],
            "einkuenfte_sonstige": g.get("einkuenfte_sonstige", 0),   # K2-Sweep-Konsistenz: §22-Loch-Vorsorge (heute ≡0 im gesamt, kein künftiger §10d-Under-tax)
            "altersentlastungsbetrag": alt24a + alt24a_b,
            "entlastungsbetrag_alleinerziehende": ent24b})
        # Abzüge §33a + §10d + DBA (§34c) via _shared
        dba_anrechnung = _shared_dba_sonstige(g, gde_p10d, g["veranlagung"], f, vz)
        # § 35 GewSt-Anrechnung (S1, opt-in via gewst_messbetrag): der GewSt-Steuermessbetrag (INPUT aus dem
        # GewSt-Messbescheid, enthält § 8-Hinzurechnung/§ 9-Kürzung schon) + Hebesatz → Anrechnung auf die
        # tarifliche ESt. Zähler des Ermäßigungshöchstbetrags (§ 35 Abs. 1 S. 2) = gewerbliche Einkünfte (S. 3
        # „der Gewerbesteuer unterliegenden Gewinne") = laufender § 15-Gewerbe-Gewinn — NUR betriebsart=gewerbe
        # (§ 18-selbständig/§ 13-LuF nicht gewerbesteuerpflichtig), § 16-vg-netto RAUS (§ 7 S. 2 GewStG: Ver-
        # äußerungsgewinn natürl. Person nicht im Gewerbeertrag). Nenner = Σ positive tarifliche Einkünfte
        # (Kapital = § 32d-Abgeltung, § 2 Abs. 5b-separat, NICHT im tariflichen zvE → exkl.). Kein gewst_messbetrag
        # → kein § 35 (opt-out, over-tax-safe). Der gewst_hebesatz_offen-Guard sperrt Messbetrag-ohne-Hebesatz.
        p35_messbetrag = _c("gewst_messbetrag") // 100
        p35_hebesatz = _c("gewst_hebesatz")
        p35_zaehler = max(0, laufender_gewinn) if f.get("gewinn_betriebsart", {}).get("wert") == "gewerbe" else max(0, mitu)
        # § 26b + § 35 Abs. 1 S. 2 ("Summe der positiven gewerblichen Einkünfte"): bei
        # Zusammenveranlagung zählt auch der Gewerbebetrieb des Ehegatten. Sein Gewinn steht seit
        # der Partner-Stufe-2 ohnehin im Nenner — ohne diese Zeilen fehlte nur der Zähler, und ein
        # Paar, bei dem NUR der Ehegatte gewerblich tätig ist, bekam gar keine Anrechnung.
        # § 16-vg bleibt draußen (§ 7 S. 2 GewStG), deshalb _laufender_gewinn_partner statt
        # _gewinn_partner_anteil. Der Hebesatz-Deckel wird je Betrieb gerechnet (s. _p35_gezahlt).
        p35_messbetrag_ges, p35_zaehler_ges, p35_gezahlt = _p35_summen(
            f, p35_messbetrag, p35_hebesatz, p35_zaehler)
        # Nenner (§ 35 Abs. 1 S. 2 „Summe aller positiven Einkünfte") = Σ positive TARIFLICHE Einkunftsarten:
        # § 19 (ns) + § 21 (vv) + § 22 (sonstige) + §§ 13-18 (gewinn, inkl. § 16-vg = § 2-Einkunft). Das
        # § 32d-Abgeltung-Kapital ist NICHT einzubeziehen (§ 2 Abs. 5b EStG: „Kapitalerträge nach § 32d Absatz 1
        # und § 43 Absatz 5 nicht einzubeziehen" — es ist nicht im tariflichen zvE, das tarifliche_est skaliert).
        # einkuenfte_sonstige im gesamt-Ring = §23 (§22Nr.2) + künftig eventuell §22Nr.5 — §22Nr.1
        # (Leibrente) lebt in der rentner-Scheibe. §23-Nenner-Integration: §23-Einkünfte sind
        # §2-tariflich, gehören also in den §35-Nenner (alle positiven tariflichen Einkünfte) —
        # der Term dokumentiert die korrekte Formel + ist robust, falls § 22 je in den gesamt-Ring kommt.
        # Der § 10 Abs. 4b-Überhang sitzt technisch in einkuenfte_sonstige (es gibt keinen
        # Hinzurechnungs-Slot), ist aber KEINE Einkunft — § 35 Abs. 1 S. 2 nennt „Einkünfte".
        # Hier wieder raus, sonst verwässert er den Anrechnungsbruch.
        p35_nenner = (max(0, ns) + max(0, vv)
                      + max(0, g.get("einkuenfte_sonstige", 0) - kist_ueberhang)
                      + max(0, g["einkuenfte_gewinn"]))

        # §3 Abs.2 SolzG: SolZ-Basis = KiFB-fiktive ESt (immer mit §32 Abs.6-Freibetraegen,
        # unabhaengig vom §31-Ergebnis) minus §32d-Kapitalsteuer. solz_info wird von
        # _festzusetzende je Lauf befuellt; der letzte Lauf (KiFB>0) ueberschreibt.
        solz_info = {}

        # §32b Progressionsvorbehalt (Stufe-1, Lohnersatz, Post-Engine-Wrapper)
        pe_raw = _c("p32b_progressionseinkuenfte") // 100
        pe_active = pe_raw > 0
        p35_active = p35_messbetrag_ges > 0 and p35_zaehler_ges > 0 and p35_nenner > 0
        # §32b×§34-Koinzidenz-Guard: Post-Engine §32b NACH §34 (tarif_modifiziert).
        # Bewegt: Guard in _an_gesamt_sperrgrund sperrt p32b_kombi_offen bei Co-Präsenz.

        def _festzusetzende(freibetrag: int) -> int:
            # Der volle festzusetzende ESt-Bescheid (§ 19+§21+alle Abzüge, PLUS § 20-Kapital-Günstiger § 32d
            # Abs. 6) bei GEGEBENEM § 32-Abs.6-Kinderfreibetrag. Kapital-Günstiger: est_ohne_kap vs est_mit_kap
            # (Grundtarif) → min(Abgeltung, Delta). freibetrag=0 → kein Kinderfreibetrag.
            g2 = g if freibetrag == 0 else dict(g, freibetraege_kinder=freibetrag)
            # § 34 CHOOSER (XOR — nie beide auf denselben vg): Abs. 1 Fünftel (Default, von Amts wegen) vs Abs. 3
            # ermäßigter Durchschnittssatz (AUF ANTRAG, 55+/dauernd-berufsunfähig, einmal im Leben, ao ≤ 5 Mio). Der
            # § 16-vg (netto_vg, außerordentlich § 34 Abs. 2 Nr. 1) wird geglättet statt voll progressiv. Engine-vor-
            # verdrahtet: tarif_modifiziert setzt tarifliche_est = tarifliche_est_modifiziert (einkommensteuertarif
            # Z.483/518). PER §31-Zweig (zve2 je Zweig — Kinderfreibetrag senkt zvE → eigener Tarif). Guard zve2>0.
            # ao = netto_vg NUR (laufender §15/§18-Gewinn progressiv). §35-Deckel-3 liest die post-§34-tarifliche unten.
            if netto_vg > 0:
                zve2 = runner.catala_gesamt_zve(g2)
                if zve2 > 0:
                    if f.get("antrag_ermaessigter_satz", {}).get("wert") is True \
                            and _abs3_eligible(f, vz) and netto_vg <= 5_000_000:
                        # § 34 Abs. 3: est = plain grundtarif(verbleibendes zvE = zvE−ao, S.3 „allgemeine Tarif-
                        # vorschriften") + ermäßigter_satz × min(ao,5Mio). est_gesamt = grundtarif(VOLLES zvE) OHNE
                        # §32b-Progressionszuschlag (nicht im Ring). catala_est nur-zvE → plain §32a, KEIN Fünftel.
                        est_rest = runner.catala_est({"veranlagungszeitraum": vz, "veranlagung": g2["veranlagung"],
                                                      "zu_versteuerndes_einkommen": max(0, zve2 - netto_vg)})
                        est_ao = runner.catala_ermaessigter_durchschnittssatz({
                            "ao_einkuenfte": netto_vg,
                            "est_gesamt_zzgl_progression": runner.catala_gesamt_tarifliche(g2),
                            "bemessungsgrundlage_durchschnitt": zve2})
                        g2 = dict(g2, tarif_modifiziert=True, tarifliche_est_modifiziert=est_rest + est_ao)
                    else:
                        # § 34 Abs. 1 Fünftel (Default ODER Abs.3-Eligibility-fail-closed — nie Abs.3 erzwingen):
                        # 5×[Tarif(zvE_rest+ao/5)−Tarif(zvE_rest)], S.3-Negativ via catala_fuenftel.
                        g2 = dict(g2, tarif_modifiziert=True, tarifliche_est_modifiziert=runner.catala_fuenftel({
                            "veranlagungszeitraum": vz, "veranlagung": g2["veranlagung"],
                            "zu_versteuerndes_einkommen": zve2, "ausserordentliche_einkuenfte": netto_vg}))
            # § 35 Abs. 1: min(4×Messbetrag [S. 1 „das Vierfache"], Messbetrag×Hebesatz [S. 5 „tatsächlich zu
            # zahlende Gewerbesteuer"], Ermäßigungshöchstbetrag [S. 2: Zähler/Nenner × geminderte tarifliche
            # Steuer]). ADDITIV in steuerermaessigungen DIESES Freibetrag-Zweigs — tarifliche_est ist freibetrag-
            # abhängig (Kinderfreibetrag senkt zvE), daher Deckel-3 JE ZWEIG (global-einmal würde den Kinder-
            # freibetrag-Zweig über-crediten = stille Under-tax). §35 Abs.1 S.4: geminderte tarifliche Steuer
            # = tarifliche_est NACH §34c-Anrechnung (PRE-§35, NEVER umgekehrt — sonst §35-Über-Kredit).
            # §32b Progressionsvorbehalt: §35-Deckel-3 braucht POST-§32b tarifliche wenn pe_active.
            # Stufe-1: §35 NORMAL in g2 wenn ¬pe_active; POST-WRAPPER-APPLY wenn pe_active.
            p35_credit = 0
            if p35_messbetrag_ges > 0 and p35_zaehler_ges > 0 and p35_nenner > 0:
                tarifliche_raw = runner.catala_gesamt_tarifliche(g2)
                tarifliche_gemindert = max(0, tarifliche_raw - dba_anrechnung)
                p35_credit = min(4 * p35_messbetrag_ges, p35_gezahlt,
                                 p35_zaehler_ges * tarifliche_gemindert // p35_nenner)
                if not pe_active:
                    g2 = dict(g2, steuerermaessigungen=g2.get("steuerermaessigungen", 0) + p35_credit)
            est_raw = runner.catala_est(g2)     # KEIN Kapital (est_regulaer_ohne_kap)
            # §32b Post-Engine-Wrapper (NACH catala_est, VOR §35-Apply-if-pe_active)
            if pe_active:
                tarifliche_pre32b = runner.catala_gesamt_tarifliche(g2)
                est_without_tarifliche = est_raw - tarifliche_pre32b
                zve32b = runner.catala_gesamt_zve(g2)
                t_32b = 0
                if zve32b > 0:
                    est_erhoeht = runner.catala_est({"veranlagungszeitraum": vz,
                                                      "veranlagung": g2.get("veranlagung", "einzel"),
                                                      "zu_versteuerndes_einkommen": zve32b + pe_raw})
                    t_32b = runner.catala_p32b_1({
                        "zu_versteuerndes_einkommen": zve32b,
                        "progressionseinkuenfte": pe_raw,
                        "est_auf_erhoehte_bemessung": est_erhoeht})
                    est_raw = t_32b + est_without_tarifliche
                # §35-Deckel-3 apply post-wrapper mit t_32b (§35 Abs.1 S.4 geminderte tarifliche = Post-§32b)
                if p35_messbetrag_ges > 0 and p35_zaehler_ges > 0 and p35_nenner > 0:
                    deckel3_32b = p35_zaehler_ges * max(0, t_32b - dba_anrechnung) // p35_nenner
                    p35_credit_pe = min(4 * p35_messbetrag_ges, p35_gezahlt, deckel3_32b)
                    est_raw = max(0, est_raw - p35_credit_pe)
            # End §32b wrapper
            if kapitaleinkuenfte <= 0:
                if freibetrag > 0 or kinder == 0:
                    solz_info["est_mit_fb"] = est_raw
                    solz_info["kap_st"] = 0
                    solz_info["est_roh_ohne_kap"] = est_raw
                    solz_info["est_roh_mit_kap"] = est_raw
                return est_raw
            est_mit = runner.catala_est(dict(g2, einkuenfte_kapitalvermoegen=kapitaleinkuenfte))
            kap_st = runner.catala_kapital_steuer({
                "veranlagungszeitraum": vz, "kapitaleinkuenfte": kapitaleinkuenfte,
                "est_regulaer_mit_kap": est_mit, "est_regulaer_ohne_kap": est_raw})
            # § 32d Abs. 6 S. 1 „wenn dies zu einer niedrigeren Einkommensteuer ... führt":
            # kap_st = min(abgeltung, delta) aus catala_kapital_steuer; guenstiger=True heißt,
            # der tarifliche Zweig (delta) hat GEWONNEN (strikt <, nicht <=, laut Gesetzeswortlaut
            # „niedrigeren"). Speist E1900401 (Antrag Günstigerprüfung), siehe _mit_ring_werten.
            abgeltung = kapitaleinkuenfte * runner._abgeltungssatz(vz) // 100
            guenstiger = kap_st < abgeltung
            # § 32d Abs. 1 S. 3-5: bei KiSt-Pflicht ermäßigt sich die Abgeltungsteuer um
            # 25 % der Kapital-KiSt (e/(4+k)-Formel), die Kapital-KiSt kommt als eigener
            # Nachtrag zu extras["kist_cent"] hinzu. NUR im Abs. 1-Fall (kap_st == abgeltung,
            # S. 6 verweist die Günstigerprüfung raus — dort greift die Formel nicht, die
            # KiSt läuft dann über §51a auf den tariflichen Kapitalanteil). Rechnung in CENT
            # (kap_st ist bereits EURO-gerundet aus catala_kapital_steuer, S. 32d-p32d-
            # abgeltung-kist.md Abschnitt 5.1) für den KiSt-Nachtrag; kap_st_k fließt EURO-
            # geglättet (Ganzzahl-Addition, wie der Rest dieser Kette) in `result`.
            kap_st_k = kap_st
            kist_kap_cent = 0
            konfession = f.get("kist_konfession", {}).get("wert", "keine")
            if kap_st == abgeltung:
                # § 32d Abs. 1 S. 2 „Die Steuer nach Satz 1 vermindert sich um die nach
                # Maßgabe des Absatzes 5 anrechenbaren ausländischen Steuern" — gilt
                # UNABHÄNGIG von KiSt-Pflicht (S. 2 ist grammatisch nicht an S. 3 gekoppelt).
                # Deckel Abs. 5 S. 3 „nur bis zur Höhe der ... entfallenden deutschen Steuer":
                # nur Jahres-Summe geprüft (q <= kap_st), der engere Pro-Kapitalertrag-25%-
                # Deckel (Abs. 5 S. 1) ist mangels Einzelposten-Datenmodell NICHT geprüft.
                q_roh_cent = f.get("kap_q_auslaendische_steuer", {}).get("wert") or 0
                q_eur = min(int(q_roh_cent) // 100, kap_st)
                if konfession in runner._KIST_KONFESSION_STEUERERHEBEND:
                    bundesland = f.get("kist_bundesland", {}).get("wert", "")
                    ksatz = 8 if bundesland in runner._KIST_BY_BW else 9
                    # § 32d Abs. 1 S. 4-5: e = kapitaleinkuenfte, q = q_eur, k = ksatz.
                    kap_st_k_cent = max(0, (kapitaleinkuenfte - 4 * q_eur) * 10000 // (400 + ksatz))
                    kist_kap_cent = kap_st_k_cent * ksatz // 100
                    kap_st_k = kap_st_k_cent // 100
                else:
                    kap_st_k = max(0, kap_st - q_eur)
            result = est_raw + kap_st_k
            if freibetrag > 0 or kinder == 0:
                solz_info["est_mit_fb"] = result
                solz_info["kap_st"] = kap_st_k
                solz_info["est_roh_ohne_kap"] = est_raw
                solz_info["est_roh_mit_kap"] = est_mit
                if extras is not None:
                    extras["kist_kap_cent"] = kist_kap_cent
                    extras["kap_guenstiger_gewonnen"] = guenstiger
            return result

        # § 31 Familienleistungsausgleich (Günstigerprüfung Kindergeld vs Kinderfreibetrag § 32 Abs. 6): bei
        # Kindern den vollen Bescheid EINMAL OHNE + einmal MIT Kinderfreibetrag rechnen; FL wählt das für den
        # Steuerpflichtigen Günstigere (Kindergeld-besser → est_ohne_fb, Kindergeld bleibt; Freibetrag-besser
        # → est_mit_fb + Kindergeld-Hinzurechnung § 31 S. 4). Der Kinderfreibetrag mindert das zvE (§ 2 Abs. 5),
        # NICHT die GdE (§ 2 Abs. 3) → die §10b/§33-Deckel (auf gde) bleiben unberührt. Ohne Kinder kein § 31.
        kinder = _c("fam_anzahl_kinder")
        if kinder > 0:
            est = runner.catala_p31_familienleistung({
                "est_ohne_freibetraege": _festzusetzende(0),
                "est_mit_freibetraegen": _festzusetzende(
                    kinder * runner._kinderfreibetrag(vz, g["veranlagung"])),
                "kindergeld": kinder * runner._kindergeld(vz) * 12})
        else:
            est = _festzusetzende(0)
        # P5.4 Rechenweg-Kette: nur im kinderlosen Fall (§ 31-Zweig-Ambiguität vermeiden).
        # Kette in extras = dict von runner.catala_gesamt_kette(g) für /ergebnis-Erklär-UI.
        if extras is not None and kinder == 0:
            extras["kette"] = runner.catala_gesamt_kette(g)
        # SolZ §3, §4 SolzG: Basis = KiFB-fiktive ESt (§3 Abs.2) minus §32d-Kapitalsteuer (§3 Abs.3 S.1);
        # §32d-Kapital-SolZ 5,5% ohne Freigrenze (§3 Abs.3 S.2) wird von catala_solz separat addiert.
        if solz_container is not None and "est_mit_fb" in solz_info:
            solz_container[0] = runner.catala_solz({
                "veranlagungszeitraum": vz,
                "bemessungsgrundlage": solz_info["est_mit_fb"],
                "kapital_steuer": solz_info.get("kap_st", 0),
                "splitting": g["veranlagung"] == "zusammen"})
        # KiSt § 51a: Bemessungsgrundlage = veranlagte ESt mit Kinderfreibetrag, OHNE
        # §32d-Kapitalanteil (= SolZ-basis_main). est_roh_ohne_kap = ESt ohne Kapitalsteuer.
        # BUG-FIX 2026-08-06: c09bd7d hatte hier versehentlich kap_st_total (nur die
        # Kapitalsteuer) als est_mit_fb übergeben → KiSt=0 für jeden kirchensteuerpflichtigen
        # Angestellten ohne Kapital (Under-tax im Normalfall).
        # § 32d Abs. 1 S. 3-5: KiSt auf die Abgeltungsteuer ist ein SEPARATER Summand zur
        # §51a-KiSt (die nur die Nicht-Kapital-ESt erfasst, est_roh_ohne_kap). Berechnet
        # oben in _festzusetzende (e/(4+k)-Korrektur, benötigt kap_st/abgeltung, die dort
        # bereits vorliegen), seit KAP Stufe 3 inkl. q-Anrechnung (kap_q_auslaendische_steuer,
        # Zeile 41/E1905101) mit Jahres-Summen-Deckel (Abs. 5 S. 3). REST-LÜCKE: der engere
        # Pro-Kapitalertrag-25%-Deckel (Abs. 5 S. 1) bleibt ungeprüft, over-tax-safe (der
        # Jahres-Deckel lässt höchstens so viel q durch wie der Jahres-Deckel erlaubt) — s.
        # reports/adjudikation/p32d-abgeltung-kist.md Abschnitt 3.
        if extras is not None:
            extras["kist_cent"] = runner.catala_kist({
                "est_mit_fb": solz_info.get("est_roh_ohne_kap", 0),
                "konfession": f.get("kist_konfession", {}).get("wert", "keine"),
                "bundesland": f.get("kist_bundesland", {}).get("wert", "")}) + extras.get("kist_kap_cent", 0)
        return est
    return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="festzusetzende_est")


def _zweig_festzusetzende_est_rentner(vz: int, bindung: dict, felder, store, nur_bestaetigt: bool,
                                     solz_container, extras):
    """§ 22 Renten + § 33b via catala_gesamt — Scheibe rentner_gesamt.

    Aus _bescheid_fn herausgezogen (Refactor 2026-08-13). `felder` ist der bei
    nur_bestaetigt=True bereits auf zustand=bestaetigt gefilterte Snapshot — die Filterung
    bleibt im Kopf von _bescheid_fn, damit die Zwei-Signal-Invariante EINE Stelle behält
    statt je Zweig wiederholt zu werden."""
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

    def _rente_instanz(fi: dict) -> int:
        # § 22 Renten-Einkünfte EINER Rente (aa/bb je Instanz-Art) → einkuenfte_sonstige-Summand. Naht-CENT
        # → EURO (jahresrente/rentenfreibetrag); renten_beginn_jahr/alter sind int (kein cent). fi = das je-
        # Instanz auf die Basis-feld_id normierte Feld-Dict (instanzen-Naht) ODER die felder-closure f (Basis).
        def _ci(k):
            v = fi.get(k, {}).get("wert")
            return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
        rf = fi.get("rentner_rentenfreibetrag", {}).get("wert")
        return runner.catala_renten_einkuenfte({
            "veranlagungszeitraum": vz,
            "renten_art": fi.get("rentner_renten_art", {}).get("wert"),
            "jahresrente": _ci("rentner_jahresrente") // 100,
            "renten_beginn_jahr": _ci("rentner_renten_beginn_jahr"),
            "alter_bei_rentenbeginn": _ci("rentner_alter_bei_rentenbeginn"),
            "rentenfreibetrag": (rf // 100 if isinstance(rf, (int, float))
                                 and not isinstance(rf, bool) else None)})

    # Keine rentner-eigenen shared-Funktionen nötig — _shared_steuer_sonder_agb
    # und _shared_dba_sonstige im Hauptscope werden mit den rentner-Closures aufgerufen.

    def slot_fn(slots: dict) -> int:
        # § 22 Renten-Einkünfte → einkuenfte_sonstige, als STUMPFE Σ über ALLE rente-Instanzen der Person A
        # (Multi-Rente, #6: gesetzl. + Betriebs- + Leibrente je eigene aa/bb-Behandlung, § 22-Anteil JE
        # RENTE, dann summiert — DIESELBE instanzen-Naht wie Multi-Objekt-§21). index==1 = Basis-rentner_*-
        # Felder, __n = weitere Renten. Ohne store (Alt-Aufrufer) nur die Basis. Die aa-Folgejahr-ohne-RF-
        # Sperre fängt der Guard je Instanz VORHER (rentenfreibetrag_fixierung_offen); hier kommt nur der
        # rechenbare Fall an. Unvollständige Renten-Instanz sperrt der Guard (rente_instanz_offen).
        if store is not None:
            # Zwei-Signal-Filter (Instanz-Pfad, nur_bestaetigt=True): vorläufige Renten-Instanz nie in die Σ.
            # Defense-in-depth — rente_instanz_offen-Guard sperrt index≥2-vorläufig schon vorher, Basis im Kegel-
            # Meet — konsistent zum gwg/vv-Σ. nur_bestaetigt=False (/stand) zeigt die vorläufige Wirkung im Range.
            renten = sum(_rente_instanz(inst["felder"])
                         for inst in EM.instanzen(store, bindung, "rente")
                         if not nur_bestaetigt or inst["zustand"] == "bestaetigt")
        else:
            renten = _rente_instanz(f)
        # Person B (§ 26b, #4b): die § 22-Rente des Ehegatten in DIESELBE einkuenfte_sonstige-Summe
        # (§9a-/Ertragsanteil JE PERSON, dann summiert). Die aa-Folgejahr-ohne-RF-Sperre für B fängt der
        # Guard vorab. Nur bei Zusammenveranlagung; Person-B ohne Rente → jahresrente 0 → renten 0.
        if (_b("veranlagung") == "zusammen") and _b("rentner_renten_art_partner"):
            rfp = _b("rentner_rentenfreibetrag_partner")
            renten += runner.catala_renten_einkuenfte({
                "veranlagungszeitraum": vz,
                "renten_art": _b("rentner_renten_art_partner"),
                "jahresrente": _c("rentner_jahresrente_partner") // 100,
                "renten_beginn_jahr": _c("rentner_renten_beginn_jahr_partner"),
                "alter_bei_rentenbeginn": _c("rentner_alter_bei_rentenbeginn_partner"),
                "rentenfreibetrag": (rfp // 100 if isinstance(rfp, (int, float))
                                     and not isinstance(rfp, bool) else None)})
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
        # §33b Abs.5 Kind-PB-Übertragung: per Kind, nur wenn Antrag +
        # Kind-nimmt-nicht + kind_idnr (kumulativ). S.4-Ausschluss greift
        # in _shared_steuer_sonder_agb (agb_cent-Kürzung).
        ausserg += _p33b_kind_pauschbetraege(store, bindung, nur_bestaetigt, vz)
        # Partner-§33b (§ 26b, #4b, Wiring-Fix): eigener Behinderten-Pauschbetrag des Ehegatten additiv zur
        # gemeinsamen ausserg-Summe — nur Zusammenveranlagung (RENTNER_PARTNER hat nur GdB/hilflos, kein
        # Pflegegrad/Hinterbliebenenbezüge für Person B). Vorher: Felder standen nur im Gate-Tuple, nie
        # tatsächlich addiert → stiller Über-tax (250 € je betroffenem Fall).
        if _b("veranlagung") == "zusammen":
            ausserg += runner.catala_behinderten_pb({
                "veranlagungszeitraum": vz, "grad_der_behinderung": _c("rentner_grad_der_behinderung_partner"),
                "ist_hilflos_blind_taubblind": _b("rentner_hilflos_blind_taubblind_partner") is True})
        # §23 Private Veräußerungsgeschäfte (Stufe-1): Σ über Instanzen → ADDITIV zu renten
        p23_eink = _p23_ansonsten_einkuenfte(f, store, bindung, nur_bestaetigt)
        renten += p23_eink
        # §22 Nr.3 Sonstige Leistungen (A8): per Accessor mit Freigrenze 256€ → ADDITIV zu renten
        # Accessor gibt CENT, einkuenfte_sonstige erwartet EURO → //100.
        nr3_r = _c("p22_nr3_einkuenfte")
        if nr3_r:
            renten += runner.catala_p22_nr3_einkuenfte(nr3_r) // 100
        # §§ 13-18 Gewinn (2-I + 2a): laufender § 15/§ 18-Gewinn (aus _laufender_gewinn — Stufe 2a EÜR ODER
        # Stufe-1-Direktwert, Scope A geteilt mit dem gesamt-Ring) + § 16-Ver-
        # äußerungsgewinn NACH § 16 Abs. 4-Freibetrag. FB (roh) via catala_p16_4_freibetrag; der steuerbare Rest
        # ist bei 0 GEFLOORT (§ 16 Abs. 4 „soweit nicht übersteigt" — FB > vg erzeugt KEINEN Verlust, sonst
        # stiller Under-tax gegen die Rente). ADDITIV (§ 16 Abs. 1: Veräußerungs- + laufender Gewinn = dieselbe
        # § 2-Einkunftsart Gewerbebetrieb → EIN einkuenfte_gewinn-Summand). Absent → 0 (over-tax-safe). CENT→EURO.
        # Naht-CENT → EURO: der Accessor nimmt EUROS (wie catala_p10_1_7_berufsausbildung — die //100-
        # Umrechnung liegt im slot_fn, nicht im Accessor). vg_euro EINMAL, an Freibetrag + Subtraktion.
        vg_euro = _c("rentner_veraeusserungsgewinn") // 100
        netto_vg = max(0, vg_euro - runner.catala_p16_4_freibetrag(
            {"rentner_veraeusserungsgewinn": vg_euro}))
        laufender_gewinn, mitu = _laufender_gewinn(f, store, bindung, nur_bestaetigt)   # § 15/§ 18 laufend (§ 35-Zähler, OHNE § 16-vg)
        # § 24a Altersentlastungsbetrag im Rentner-Ring (b): Bemessung = positive Nicht-§19-Einkünfte = §§13-18-Gewinn
        # (laufender + § 16-vg-netto); LEIBRENTE § 22 Nr. 1 (renten) + Versorgungsbezüge § 19 Abs. 2 sind KEINE Bemessung
        # (§ 24a S. 2-Ausschlüsse). §23 private Veräußerungsgeschäfte (§22 Nr.2) gehören IN die
        # §24a-Bemessung (S.2 erwähnt nur §19Abs.2/§22Nr.1/§22Nr.4/5 — kein §22Nr.2-Ausschluss).
        # p23_eink additiv zu laufender_gewinn + netto_vg (K2-konservativ: floor auf kombinierte Summe).
        # Kein § 19-Mini-Job-Arbeitslohn im rentner-Ring (MVP-Lücke, over-tax-safe → 0). MIT
        # dem 64+-Gate (§ 24a S. 3, geerbt vom Accessor via veranlagungszeitraum). Pure Leibrente (kein Gewinn) → 0.
        alt24a_r = runner.catala_p24a_altersentlastung({
            "veranlagungszeitraum": vz, "geburtsjahr": _c("geburtsjahr"), "arbeitslohn": 0,
            "positive_andere_einkuenfte": max(0, laufender_gewinn + netto_vg + p23_eink)})
        # § 24b Entlastungsbetrag Alleinerziehende (Weg-ii-Parität-Fix, K2, Over-tax): fehlte im Rentner-Ring
        # komplett (GESAMT_FREIBETRAEGE nie an RENTNER_FELDER, s.o.) — Rentner-Witwe/-Witwer mit Kindern kriegt
        # sonst den 4260€+240€/Kind-Freibetrag nicht. 1:1 gesamt-Präzedenz Z. 671-674 (ungegatet, fam_anzahl_
        # kinder-Kinder-Gate ist Fund D/separater Scope). alleinerziehend_mit_zusammen-Guard (Z. 1110) fängt
        # den Widerspruch automatisch, sobald fam_alleinstehend gebunden ist — kein Zusatzcode nötig.
        ent24b_r = runner.catala_p24b_entlastung({
            "alleinstehend": f.get("fam_alleinstehend", {}).get("wert") is True,
            "anzahl_kinder": _c("fam_anzahl_kinder"),
            "monate_ohne_voraussetzung": _c("fam_monate_ohne_voraussetzung")})
        # § 26b: Gewinneinkünfte des Ehegatten (Stufe 2 der Partnerachse, 2026-08-13). BEWUSST erst
        # hier und nicht in alt24a_r oben: § 24a S. 1 knüpft an "den Steuerpflichtigen" an, der
        # Altersentlastungsbetrag von Person A darf sich am Gewinn von Person B nicht erhöhen.
        gewinn_partner, _mitu_partner = _gewinn_partner_anteil(f)
        rentner_g = {
            "gesamtfall": True, "veranlagungszeitraum": vz,
            "veranlagung": _b("veranlagung") or "einzel",
            "einkuenfte_sonstige": renten,
            "einkuenfte_gewinn": laufender_gewinn + netto_vg + gewinn_partner,
            "altersentlastungsbetrag": alt24a_r,
            "entlastungsbetrag_alleinerziehende": ent24b_r,
            "aussergewoehnliche_belastungen": ausserg}
        # § 10d Abs. 2 Verlustvortrag (Rentner-Ring): mindert die volle rentner-GdE (renten § 22 + einkuenfte_
        # gewinn) „vorrangig vor …" (§ 10d Abs. 2 S. 1) → sonstige_abzuege_vom_einkommen, VOR dem § 35 (dessen
        # Deckel-3 die post-§10d geminderte tarifliche ESt nutzt). catala_p10d_2-Höchstbetrag = min(GdE, …), absent
        # → 0. Der GdE-Zwilling nimmt die rentner-Einkunftsarten (kein § 19/§ 21 hier), § 32d-Kapital gibt's nicht.
        # DIESELBE gde speist auch die Weg-ii-Abzüge unten (§10b-20%-Deckel + §33-zumutbar-Staffel) — EIN Aufruf.
        gde = runner.catala_gesamt_gde({
            "veranlagungszeitraum": vz, "veranlagung": rentner_g["veranlagung"],
            "einkuenfte_sonstige": renten, "einkuenfte_gewinn": rentner_g["einkuenfte_gewinn"],
            "altersentlastungsbetrag": alt24a_r, "entlastungsbetrag_alleinerziehende": ent24b_r})
        # Kapital § 20/§ 32d (Rentner): SINGLE-SOURCE (Instructor-Q1) — Töpfe XOR Aggregat.
        # Töpfe (§ 20 Abs. 6) → verrechnet; sonst Aggregat. Sparer-PB (§ 20 Abs. 9) — UNABHÄNGIG von GdE.
        # § 2 Abs. 5b: § 32d-Kapitalerträge gehören NICHT in gde (oben berechnet) — nur Renten + Gewinn.
        # Zusammenveranlagung für den Sparer-PB: nur aus der Veranlagungsart (§ 26),
        # § 20 Abs. 9 S. 3 verdoppelt ihn dann — s. _p20_kapitaleinkuenfte.
        zusammen_r = rentner_g["veranlagung"] == "zusammen"
        kapitaleinkuenfte_r = _p20_kapitaleinkuenfte(_c, zusammen_r, vz)
        # § 10 Abs. 4b S. 3 — wie im gesamt-Pfad: der Erstattungsüberhang erhöht den GdE
        # (lokale gde für §10b/§33) und geht mangels Hinzurechnungs-Slot über
        # einkuenfte_sonstige in die Engine. NICHT in den § 35-Nenner unten — der liest
        # `renten`, nicht rentner_g, und bleibt damit auf echten Einkünften.
        kist_ueberhang_r = runner.catala_p10_4b_erstattungsueberhang({
            "gezahlte_kirchensteuer": _c("kist_gezahlt") // 100,
            "erstattete_kirchensteuer": _c("kist_erstattet") // 100})
        gde += kist_ueberhang_r
        rentner_g["einkuenfte_sonstige"] += kist_ueberhang_r
        # Abzüge §33a + §10d + DBA (§34c) via _shared (1:1 gesamt-Präzedenz).
        # Steht NACH kist_ueberhang (gde vollständig), VOR vorsorge/§35a (kein Overlap).
        dba_anrechnung = _shared_dba_sonstige(rentner_g, gde, rentner_g["veranlagung"], f, vz)
        # Weg-ii-Fix (K2, Over-tax): § 10 Abs. 1 Nr. 2 Basisvorsorge (VOR_FELDER jetzt Pflicht-Kegel) —
        # catala_est ruft _vorsorge_abzug intern (runner.py), kein Doppelzählen, nur die Slots setzen.
        rentner_g["vorsorge_gesamtbeitraege_inkl_ag"] = (_c("vor_an_anteil_rv") + _c("vor_ag_anteil_rv")
                                                          + _c("vor_rv_ausserhalb_lstb")) // 100
        rentner_g["vorsorge_ag_anteil_steuerfrei"] = _c("vor_ag_anteil_rv") // 100
        # Person-B-Altersvorsorge (§ 10 Abs. 1 Nr. 2, A.2): bei zusammen die vor_*_rv_partner ADDITIV
        # in dieselben Summen-Slots (catala_gesamt/_vorsorge_abzug deckelt EINMAL, 1:1 gesamt-Z.921-924).
        if _b("veranlagung") == "zusammen":
            rentner_g["vorsorge_gesamtbeitraege_inkl_ag"] += (_c("vor_an_anteil_rv_partner")
                + _c("vor_ag_anteil_rv_partner") + _c("vor_rv_ausserhalb_lstb_partner")) // 100
            rentner_g["vorsorge_ag_anteil_steuerfrei"] += _c("vor_ag_anteil_rv_partner") // 100
        # Abzüge §35a/§35c + Sonderausgaben + agB (via _shared, 1:1 gesamt-Präzedenz)
        _shared_steuer_sonder_agb(rentner_g, gde, ausserg, rentner_g["veranlagung"],
                                  f, vz, store, bindung, nur_bestaetigt)
        # § 35 GewSt-Anrechnung Basiswerte (freibetrag-unabhängig — Zähler/Nenner hängen nicht vom § 31-Zweig
        # ab, nur die tarifliche_est im Deckel-3 unten). Zähler = laufender Gewerbe-Gewinn (NUR betriebsart=
        # gewerbe, § 16-vg-netto RAUS § 7 S. 2 GewStG). Nenner = renten (§ 22 IM Nenner — echt hier, anders als
        # gesamt wo sonstige=0) + einkuenfte_gewinn (VOLLSTÄNDIG: die rentner-Scheibe hat kein § 19/§ 21). Opt-
        # in via gewst_messbetrag; gewst_hebesatz_offen-Guard (shared) fängt Messbetrag-ohne-Hebesatz.
        p35_messbetrag = _c("gewst_messbetrag") // 100
        p35_hebesatz = _c("gewst_hebesatz")
        p35_zaehler = max(0, laufender_gewinn) if _b("gewinn_betriebsart") == "gewerbe" else max(0, mitu)
        # § 26b: Gewerbebetrieb des Ehegatten, s. gesamt-Zweig (§ 35 Abs. 1 S. 2).
        p35_messbetrag_ges, p35_zaehler_ges, p35_gezahlt = _p35_summen(
            f, p35_messbetrag, p35_hebesatz, p35_zaehler)
        p35_nenner = max(0, renten) + max(0, rentner_g["einkuenfte_gewinn"])

        # §3 Abs.2 SolzG: SolZ-Basis = KiFB-fiktive ESt (immer mit §32 Abs.6-Freibetraegen;
        # solz_info_r wird im KiFB-Lauf und im § 32d-Kapital-Lauf gefüllt.
        solz_info_r = {}

        # §32b Progressionsvorbehalt (Rentner-Ring, 1:1 gesamt-Präzedenz)
        pe_raw = _c("p32b_progressionseinkuenfte") // 100
        pe_active = pe_raw > 0

        def _festzusetzende_r(freibetrag: int) -> int:
            # § 31 Familienleistungsausgleich (Fund D, Rentner-Ring-Fix): PER §31-Zweig neu gerechnet (g2 statt
            # rentner_g direkt) — Kinderfreibetrag senkt zvE (§ 2 Abs. 5) → eigener Tarif + eigene § 35-Deckel-3-
            # Anrechnung je Zweig. 1:1 gesamt-Naht-Präzedenz (_festzusetzende Z. 842-892).
            g2 = dict(rentner_g, freibetraege_kinder=freibetrag) if freibetrag else dict(rentner_g)
            # § 34 CHOOSER im Rentner-Ring (Abs. 1 Fünftel Default vs Abs. 3 ermäßigter Satz auf Antrag):
            # identisch zur gesamt-Naht. Guard zve2>0. ao = netto_vg (laufender Gewinn progressiv).
            if netto_vg > 0:
                zve2 = runner.catala_gesamt_zve(g2)
                if zve2 > 0:
                    if f.get("antrag_ermaessigter_satz", {}).get("wert") is True \
                            and _abs3_eligible(f, vz) and netto_vg <= 5_000_000:
                        # § 34 Abs. 3: plain grundtarif(verbleibendes zvE, S.3) + ermäßigter_satz × min(ao,5Mio).
                        est_rest = runner.catala_est({"veranlagungszeitraum": vz, "veranlagung": g2["veranlagung"],
                                                      "zu_versteuerndes_einkommen": max(0, zve2 - netto_vg)})
                        est_ao = runner.catala_ermaessigter_durchschnittssatz({
                            "ao_einkuenfte": netto_vg,
                            "est_gesamt_zzgl_progression": runner.catala_gesamt_tarifliche(g2),
                            "bemessungsgrundlage_durchschnitt": zve2})
                        g2 = dict(g2, tarif_modifiziert=True, tarifliche_est_modifiziert=est_rest + est_ao)
                    else:
                        g2 = dict(g2, tarif_modifiziert=True, tarifliche_est_modifiziert=runner.catala_fuenftel({
                            "veranlagungszeitraum": vz, "veranlagung": g2["veranlagung"],
                            "zu_versteuerndes_einkommen": zve2, "ausserordentliche_einkuenfte": netto_vg}))
            # § 35 GewSt-Anrechnung Deckel-3 (JE §31-Zweig — tarifliche_est ist freibetrag-abhängig, global-
            # einmal würde den Kinderfreibetrag-Zweig über-crediten = stille Under-tax, 1:1 gesamt-Präzedenz).
            p35_credit_r = 0
            if p35_messbetrag_ges > 0 and p35_zaehler_ges > 0 and p35_nenner > 0:
                tarifliche_raw = runner.catala_gesamt_tarifliche(g2)
                tarifliche_gemindert = max(0, tarifliche_raw - dba_anrechnung)
                # Weg-ii-Fix (K2, PFLICHT): ADDITIV statt hart überschreiben — sonst löscht § 35 GewSt-
                # Anrechnung das § 35a-Ergebnis (steuerermaessigungen) still.
                p35_credit_r = min(4 * p35_messbetrag_ges, p35_gezahlt,
                                   p35_zaehler_ges * tarifliche_gemindert // p35_nenner)
                if not pe_active:
                    g2 = dict(g2, steuerermaessigungen=g2.get("steuerermaessigungen", 0) + p35_credit_r)
            # § 32d Abs. 6 Günstigerprüfung: Kapitalerträge tariflich oder Abgeltungsteuer?
            if kapitaleinkuenfte_r <= 0:
                result = runner.catala_est(g2)
            else:
                # EST ohne Kapitalerträge (Baseline)
                est_raw = runner.catala_est(g2)
                # EST mit Kapitalerträgen (tariflich)
                est_mit = runner.catala_est(dict(g2, einkuenfte_kapitalvermoegen=kapitaleinkuenfte_r))
                # Günstigerprüfung: min(25 % Abgeltungsteuer, tariflicher Mehrbetrag)
                kap_st = runner.catala_kapital_steuer({
                    "veranlagungszeitraum": vz,
                    "kapitaleinkuenfte": kapitaleinkuenfte_r,
                    "est_regulaer_mit_kap": est_mit,
                    "est_regulaer_ohne_kap": est_raw})
                # § 32d Abs. 6 S. 1 (1:1 gesamt-Präzedenz Z. 1201-1204): guenstiger=True heißt,
                # der tarifliche Zweig hat GEWONNEN (strikt <). Speist E1900401.
                abgeltung_r = kapitaleinkuenfte_r * runner._abgeltungssatz(vz) // 100
                guenstiger_r = kap_st < abgeltung_r
                # § 32d Abs. 1 S. 3-5 (1:1 gesamt-Präzedenz Z. 1138-1160): Abgeltungsteuer-
                # Ermäßigung + Kapital-KiSt-Nachtrag, NUR im Abs. 1-Fall (kap_st == abgeltung).
                kap_st_k = kap_st
                kist_kap_cent = 0
                konfession_r = f.get("kist_konfession", {}).get("wert", "keine")
                if kap_st == abgeltung_r:
                    # § 32d Abs. 1 S. 2 q-Anrechnung, unabhängig von KiSt-Pflicht (1:1
                    # gesamt-Präzedenz, s. dort). Deckel Abs. 5 S. 3: Jahres-Summe only.
                    q_roh_cent_r = f.get("kap_q_auslaendische_steuer", {}).get("wert") or 0
                    q_eur_r = min(int(q_roh_cent_r) // 100, kap_st)
                    if konfession_r in runner._KIST_KONFESSION_STEUERERHEBEND:
                        bundesland_r = f.get("kist_bundesland", {}).get("wert", "")
                        ksatz_r = 8 if bundesland_r in runner._KIST_BY_BW else 9
                        kap_st_k_cent = max(0, (kapitaleinkuenfte_r - 4 * q_eur_r) * 10000 // (400 + ksatz_r))
                        kist_kap_cent = kap_st_k_cent * ksatz_r // 100
                        kap_st_k = kap_st_k_cent // 100
                    else:
                        kap_st_k = max(0, kap_st - q_eur_r)
                result = est_raw + kap_st_k
                # SolZ-Tracking: est_mit_fb = KiFB-fiktive ESt (SolzG §3 Abs.3 S.1: cap-st ist
                # abgezogen VOR Freigrenze). kap_st_k = §32d-Abgeltung-SolZ (5,5% ohne Freigrenze).
                # est_roh_ohne_kap = ESt vor §32d-Kapital — für §51a-KiSt-Basis (OHNE §32d).
                if freibetrag > 0 or kinder == 0:
                    solz_info_r["est_mit_fb"] = result
                    solz_info_r["est_roh_ohne_kap"] = est_raw
                    # BACKLOG rentner-solz-kap-st-tracking (bada2a0-Fund): fehlte, dadurch lief
                    # kapital_steuer=0 in catala_solz -> §3 Abs.3 S.1 minderte die Basis nie und
                    # S.2 (5,5% ohne Freigrenze) fehlte ganz. 1:1 gesamt-Präzedenz (Z. 1282).
                    solz_info_r["kap_st"] = kap_st_k
                    if extras is not None:
                        extras["kist_kap_cent"] = kist_kap_cent
                        extras["kap_guenstiger_gewonnen"] = guenstiger_r
            # §32b Post-Engine-Wrapper (Rentner-Ring, 1:1 gesamt-Präzedenz)
            if pe_active:
                tarifliche_pre32b = runner.catala_gesamt_tarifliche(g2)
                est_without_tarifliche = result - tarifliche_pre32b
                zve32b = runner.catala_gesamt_zve(g2)
                if zve32b > 0:
                    est_erhoeht = runner.catala_est({"veranlagungszeitraum": vz,
                                                      "veranlagung": g2.get("veranlagung", "einzel"),
                                                      "zu_versteuerndes_einkommen": zve32b + pe_raw})
                    tarifliche_32b = runner.catala_p32b_1({
                        "zu_versteuerndes_einkommen": zve32b,
                        "progressionseinkuenfte": pe_raw,
                        "est_auf_erhoehte_bemessung": est_erhoeht})
                    result = tarifliche_32b + est_without_tarifliche
                # §35-Deckel-3 post-wrapper mit tarifliche_32b
                if p35_credit_r > 0:
                    result = max(0, result - p35_credit_r)
            # SolZ-Tracking: est_mit_fb wird HIER final gesetzt (nach §32b/§35-Wrappern), da
            # `result` sich seit dem kap>0-Zweig (falls durchlaufen) noch ändern kann.
            # est_roh_ohne_kap/kap_st bleiben unverändert, wenn bereits vom kap>0-Zweig gesetzt
            # (dort = est_raw bzw. kap_st_k). Bei kap<=0 werden beide hier erstmals gesetzt
            # (est_roh_ohne_kap=result, kap_st=0 — kein §32d-Kapitalanteil vorhanden).
            if freibetrag > 0 or kinder == 0:
                solz_info_r["est_mit_fb"] = result
                solz_info_r["est_roh_ohne_kap"] = solz_info_r.get("est_roh_ohne_kap", result)
                solz_info_r["kap_st"] = solz_info_r.get("kap_st", 0)
            return result

        # § 31 Familienleistungsausgleich (Günstigerprüfung Kindergeld vs Kinderfreibetrag § 32 Abs. 6, Fund D):
        # fehlte komplett im Rentner-Ring — Rentner mit Kindern verlor die Günstiger-Freibetrag-Anrechnung
        # (Over-tax). 1:1 gesamt-Naht-Präzedenz Z. 894-906. Ohne Kinder kein § 31.
        kinder = _c("fam_anzahl_kinder")
        if kinder > 0:
            est = runner.catala_p31_familienleistung({
                "est_ohne_freibetraege": _festzusetzende_r(0),
                "est_mit_freibetraegen": _festzusetzende_r(
                    kinder * runner._kinderfreibetrag(vz, rentner_g["veranlagung"])),
                "kindergeld": kinder * runner._kindergeld(vz) * 12})
        else:
            est = _festzusetzende_r(0)
        # SolZ §3, §4 SolzG: Basis = KiFB-fiktive ESt (§3 Abs.2) minus §32d-Kapitalsteuer (§3 Abs.3 S.1);
        # §32d-Kapital-SolZ 5,5% ohne Freigrenze (§3 Abs.3 S.2) wird von catala_solz separat addiert.
        if solz_container is not None and "est_mit_fb" in solz_info_r:
            solz_container[0] = runner.catala_solz({
                "veranlagungszeitraum": vz,
                "bemessungsgrundlage": solz_info_r["est_mit_fb"],
                # kap_st_k (1:1 gesamt-Präzedenz Z. 1282) — BACKLOG rentner-solz-kap-st-tracking
                # geschlossen (bada2a0-Fund): Setzstelle oben ergänzt (kap>0-Zweig), .get()
                # bleibt als fail-closed-Default (0) für den kap<=0-Zweig stehen.
                "kapital_steuer": solz_info_r.get("kap_st", 0),
                "splitting": rentner_g["veranlagung"] == "zusammen"})
        # KiSt § 51a: Basis = KiFB-fiktive ESt OHNE §32d-Kapital (= est_roh_ohne_kap).
        # §32d-Abgeltung-KiSt ist separater Nachtrag (§32d Abs.1 S.3-5, benannte Lücke).
        # BUG-FIX 2026-08-06: selbe Bugklasse wie gesamt-Ring (Z.1144). Z.1575/1578 setzte
        # est_mit_fb = est_raw + kap_st → KiSt auf 13.855 statt 1.605 (Rentner 20k + Kapital 50k).
        # est_roh_ohne_kap = est_raw (ESt ohne §32d-Kapital), separat von est_mit_fb (SolZ-Basis).
        if extras is not None and "est_roh_ohne_kap" in solz_info_r:
            extras["kist_cent"] = runner.catala_kist({
                "est_mit_fb": solz_info_r["est_roh_ohne_kap"],
                "konfession": f.get("kist_konfession", {}).get("wert", "keine"),
                "bundesland": f.get("kist_bundesland", {}).get("wert", "")}) + extras.get("kist_kap_cent", 0)
        return est
    return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="festzusetzende_est")


def _bescheid_fn(quantitaet: str, vz: int, bindung: dict, felder: dict | None = None,
                 store: dict | None = None, nur_bestaetigt: bool = True, solz_container=None,
                 extras: dict | None = None):
    """bescheid_fn(feld_werte)->cent für eine ring-fähige Familie (Naht-Einheit CENT via
    intervall.bescheid_via_slots). None, wenn die Catala-Toolchain oder ein Accessor fehlt —
    dann bleibt der Ring ehrlich leer, nie ein erfundener Betrag. `felder` (materialisierter
    Store-Snapshot) erlaubt den Zugriff auf Einzelfelder, die die Summen-Slots verdecken (VOR-AG).
    `store` (optional) erlaubt die Multi-Objekt-Instanz-Enumeration (est_mapping.instanzen, #5) — ohne
    store rechnet der §21-Ring nur die Basis-Instanz (Alt-Aufrufer/Teil-Ringe).

    `nur_bestaetigt` (DEFAULT True = fail-safe): filtert den felder-Snapshot (+ die Instanz-Σ) auf bestätigt-
    only → für die FESTGESETZTE Steuer (/ergebnis via _feste_zahl). False NUR in den Estimate-Pfaden (/stand +
    fragen), die die [min,max]-Spanne bauen und NIE die festgesetzte Steuer emittieren — dort SOLL die vorläufige
    Wirkung sichtbar sein (Steuer-at-Risk-Range, Instructor-Vertrag). Default True hält jeden neuen Caller sicher."""
    # ⭐ SECURITY — Zwei-Signal-Invariant AM RING: bei nur_bestaetigt=True liest der Bescheid AUSSCHLIESSLICH
    # bestätigte Werte. Ein VORLÄUFIGER Vorschlag (llm:chat / import:beleg / kontoauszug / vorjahr / berechnet:maps)
    # für ein OPTIONALES Feld (agB §33, Spenden §10b, Berufsausbildung §10, Mitunternehmer §15, §16-vg … — NICHT im
    # Pflicht-Kegel, also NICHT vom Meet-Gate in _feste_zahl erfasst) darf die festgesetzte Steuer NIE bewegen,
    # bevor der Mensch signal_2 gesetzt hat ("ein Vorschlag bewegt die Summe nie ohne Confirm"). Ohne den Filter
    # las _c/_cent/_b den Roh-Aktiv-Wert (zustand-blind) → ein vorläufiger llm:chat-agB senkte /ergebnis (604k
    # statt 691k) OHNE Confirm. vorläufig → hier absent → 0 → kein Abzug bis zum Confirm (over-tax-safe). Kegel-
    # Felder sind ohnehin alle bestätigt (Gate in _feste_zahl). Der Sperr-Guard (_an_gesamt_sperrgrund) sieht die
    # Roh-felder SEPARAT weiter (ein vorläufiges nicht-ring-fähiges Feld muss den Ring weiter sperren). Die store-
    # basierte Instanz-Σ (est_mapping.instanzen) liest den store separat → nur_bestaetigt wird DURCHgefädelt
    # (_gwg_sofortabzug_summe/_laufender_gewinn + vv/rente-Σ inline), sonst zeigte /stand die gwg-Instanz-Wirkung nicht.
    if nur_bestaetigt and felder:
        felder = {fid: ev for fid, ev in felder.items() if ev.get("zustand") == "bestaetigt"}
    # Ab hier nur noch Verteilung. Die Rechenwege stehen seit dem Refactor (2026-08-13) als
    # _zweig_*-Modulfunktionen daneben, die gemeinsamen Bausteine als _kind_kv_pv_summe,
    # _kind_behinderten_pb_daten, _shared_steuer_sonder_agb (§ 35a/§ 35c + Sonderausgaben + agB)
    # und _shared_dba_sonstige (§ 33a/§ 10d + DBA). Alle vier Zweige bekommen dieselbe Signatur —
    # gemessen, nicht vereinheitlicht: sie hatten schon vorher identische freie Variablen.
    if quantitaet == "abziehbarer_betrag":          # § 9 Entfernungspauschale
        return _zweig_abziehbarer_betrag(vz, bindung)

    if quantitaet == "festzusetzende_est":          # § 2 Gesamtsteuer MVP (reiner AN-Fall)
        return _zweig_festzusetzende_est(vz, bindung, felder, store, nur_bestaetigt,
                                        solz_container, extras)

    if quantitaet == "festzusetzende_est_gesamt":          # § 21 V+V via catala_gesamt (reiner §21-MVP)
        return _zweig_festzusetzende_est_gesamt(vz, bindung, felder, store, nur_bestaetigt,
                                               solz_container, extras)

    if quantitaet == "festzusetzende_est_rentner":          # § 22 Renten + § 33b via catala_gesamt
        return _zweig_festzusetzende_est_rentner(vz, bindung, felder, store, nur_bestaetigt,
                                                solz_container, extras)

    # festzusetzende_est_haushalt (§35a+§10b) + festzusetzende_est_agb (§33+§10-KiSt) ENTFERNT (Weg ii, Stage 1b):
    # ihre Abzüge sind in den gesamt-Ring gefaltet (siehe festzusetzende_est_gesamt slot_fn — GESAMT_ABZUEGE).
    return None     # kein exponierter Accessor -> ehrlich None (dHf/Verpflegung/AM/VOR/GWG)
