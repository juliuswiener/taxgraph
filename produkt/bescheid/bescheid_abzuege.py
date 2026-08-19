"""Abzuege: Kinder (§§ 10 Abs. 1 Nr. 5/9, 33b), Sonderausgaben und aussergewoehnliche Belastungen, dazu zwei kleine geteilte Helfer (OePNV-Kosten, § 34 Abs. 3-Eignung).

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
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "golden", "elster"):
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

def _abs3_eligible(f: dict, vz: int) -> bool:
    """§ 34 Abs. 3 S. 1: (Alter ≥ 55 [DERIVE aus geburtsjahr] ODER dauernd berufsunfähig) UND § 34 Abs. 3 S. 4 nicht
    schon einmal genutzt. SHARED zwischen Chooser (_festzusetzende) und Guard (_an_gesamt_sperrgrund) — bit-identisch,
    sonst Guard/Chooser-Drift. antrag_ermaessigter_satz (S. 1 „auf Antrag") wird vom Aufrufer separat geprüft."""
    gj = f.get("geburtsjahr", {}).get("wert")
    gj = int(gj) if isinstance(gj, (int, float)) and not isinstance(gj, bool) else 0
    alter_ge_55 = (vz - gj) >= 55 if gj > 0 else False
    berufsunfaehig = f.get("dauernd_berufsunfaehig", {}).get("wert") is True
    einmal_genutzt = f.get("ermaessigung_einmal_genutzt", {}).get("wert") is True
    return (alter_ge_55 or berufsunfaehig) and not einmal_genutzt


def _oepnv_eur(slots: dict) -> int:
    """oepnv_kosten_jahr Naht-CENT -> EURO (Store liefert Cent, EP_FELDER-Runner-Accessor erwartet Euro)."""
    return int(slots["oepnv_kosten_jahr"]) // 100


def _kind_kv_pv_summe(store, bindung: dict, nur_bestaetigt: bool) -> int:
    """§10 Abs.1 Nr.3 S.2 KV/PV-Beiträge des Kindes — addiert kind_kv + kind_pv pro Kind-Instanz
    (CENT, direkt zu basis_kv/basis_pv addierbar). Voraussetzung kind_idnr (S.2): bei fehlender
    kind_idnr wird der Kind-Beitrag NICHT eingerechnet (over-tax-safe, kein Abzug ohne IdNr).

    Aus _bescheid_fn herausgezogen (Refactor 2026-08-13, Schritt 2). Dort stand sie zweimal —
    einmal echt (store vorhanden) und einmal als 0-Stub im else-Zweig; die Fallunterscheidung
    ist jetzt die erste Zeile statt zweier konkurrierender Definitionen desselben Namens."""
    if store is None:
        return 0
    total = 0
    for inst in EM.instanzen(store, bindung, "kind"):
        if not nur_bestaetigt or inst["zustand"] == "bestaetigt":
            idnr = inst["felder"].get("kind_idnr", {}).get("wert")
            if not idnr or not isinstance(idnr, str) or len(idnr) < 11:
                continue
            kv = inst["felder"].get("kind_kv", {}).get("wert")
            if isinstance(kv, (int, float)) and not isinstance(kv, bool) and kv > 0:
                total += int(kv)
            pv = inst["felder"].get("kind_pv", {}).get("wert")
            if isinstance(pv, (int, float)) and not isinstance(pv, bool) and pv > 0:
                total += int(pv)
    return total


def _kinderbetreuung_summe(store, bindung: dict, nur_bestaetigt: bool, vz: int) -> int:
    """Per-Kind-Summe §10 Abs.1 Nr.5 via EM.instanzen. Keine Gleichverteilung mehr.
    (2026-08-06 Fix: 2 Kinder/10000€ → 6400€ statt 8000€.)
    Voraussetzung kind_unter_14_haushaltszugehoerig (S.1, Geltungsbedingung in
    rules.yaml/p10_1_5_kinderbetreuung, bislang unbewacht): bei fehlender/verneinter
    Bestaetigung wird das Kind NICHT eingerechnet (over-tax-safe, kein Abzug ohne
    Altersnachweis). 2026-08-11 Fix: vorher summierte diese Funktion JEDES Kind ohne
    Alters-/Behinderungspruefung.

    Zusammengeführt 2026-08-13: gesamt- und rentner-Zweig hielten je eine eigene Kopie mit
    identischem Rumpf. Das Qualifikationsgate oben musste deshalb am 2026-08-11 zweimal
    eingebaut werden — genau die Doppel-Bug-Klasse, die diese Zusammenführung beendet."""
    import runner
    if store is None:
        return 0
    total = 0
    for inst in EM.instanzen(store, bindung, "kind"):
        if not nur_bestaetigt or inst["zustand"] == "bestaetigt":
            qualifiziert = inst["felder"].get("kind_unter_14_haushaltszugehoerig", {}).get("wert")
            if qualifiziert is not True:
                continue
            aufw = inst["felder"].get("kinderbetreuungskosten", {}).get("wert")
            if isinstance(aufw, (int, float)) and not isinstance(aufw, bool) and aufw > 0:
                total += runner.catala_p10_1_5_kinderbetreuung({
                    "aufwendungen": int(aufw) // 100,
                    "veranlagungszeitraum": vz})
    return total


def _schulgeld_summe(store, bindung: dict, nur_bestaetigt: bool, vz: int, f: dict) -> int:
    """Per-Kind-Summe §10 Abs.1 Nr.9 via EM.instanzen (Ring-Lese-Naht wie Nr.5).
    ANNAHME: bei Einzelveranlagung 2.500€ je Kind (hb aus params), bei Zusammenveranlagung
    5.000€ je Kind (Accessor-splitting). Aufteilung bei getrennter Veranlagung (Kz E0504603)
    wird NICHT ausgewertet — Begründung im Accessor-Docstring.

    Zusammengeführt 2026-08-13 (s. _kinderbetreuung_summe). `f` nur für die Veranlagungsart."""
    import runner
    if store is None:
        return 0
    total = 0
    splitting = f.get("veranlagung", {}).get("wert") == "zusammen"
    for inst in EM.instanzen(store, bindung, "kind"):
        if not nur_bestaetigt or inst["zustand"] == "bestaetigt":
            aufw = inst["felder"].get("schulgeld", {}).get("wert")
            if isinstance(aufw, (int, float)) and not isinstance(aufw, bool) and aufw > 0:
                total += runner.catala_p10_1_9_schulgeld({
                    "aufwendungen": int(aufw) // 100,
                    "veranlagungszeitraum": vz, "splitting": splitting})
    return total


def _kind_behinderten_pb_daten(store, bindung: dict, nur_bestaetigt: bool) -> list:
    """§33b Abs.5 Kind-PB-Übertragung — list[dict] je Kind, passend für catala_behinderten_pb /
    catala_hinterbliebenen_pb.

    S.1: "auf Antrag ... wenn ihn das Kind nicht in Anspruch nimmt" (kumulativ).
    S.5: kind_idnr als Voraussetzung (fail-closed wie B2).
    S.4-Ausschluss: in _shared_steuer_sonder_agb — sobald diese Funktion eine nichtleere Liste
    liefert, wird behinderungsbedingte_aufwendungen von agb_aufwendungen abgezogen. Getrennt
    statt pauschal, weil eine pauschale Kürzung von agb_aufwendungen Over-tax wäre.

    Aus _bescheid_fn herausgezogen (Refactor 2026-08-13, Schritt 3)."""
    if store is None:
        return []
    daten = []
    for inst in EM.instanzen(store, bindung, "kind"):
        if not nur_bestaetigt or inst["zustand"] == "bestaetigt":
            idnr = inst["felder"].get("kind_idnr", {}).get("wert")
            if not idnr or not isinstance(idnr, str) or len(idnr) < 11:
                continue
            antrag = inst["felder"].get("kind_behinderten_pb_antrag", {}).get("wert") is True
            nicht_selbst = inst["felder"].get("kind_pb_nicht_selbst_genutzt", {}).get("wert") is True
            if not (antrag and nicht_selbst):
                continue
            gdb_raw = inst["felder"].get("kind_grad_der_behinderung", {}).get("wert")
            gdb = int(gdb_raw) if isinstance(gdb_raw, (int, float)) and not isinstance(gdb_raw, bool) else 0
            daten.append({
                "grad_der_behinderung": gdb,
                "ist_hilflos_blind_taubblind": inst["felder"].get("kind_hilflos_blind_taubblind", {}).get("wert") is True,
                "hat_hinterbliebenenbezuege": inst["felder"].get("kind_hinterbliebenen_uebertragung", {}).get("wert") is True,
            })
    return daten


def _shared_steuer_sonder_agb(g_dict, gde, ausserg, veranlagung,
                              f_dict, vz: int, store, bindung: dict, nur_bestaetigt: bool):
    """Füllt g_dict: steuerermaessigungen, sonderausgaben, aussergewoehnliche_belastungen.

    Aus _bescheid_fn herausgezogen (Refactor 2026-08-13, Schritt 5). vz/store/bindung/
    nur_bestaetigt sind jetzt Parameter statt Closure-Variablen — sie steuern die Instanz-
    Enumeration in _hh_summe (§ 35a) und _kind_behinderten_pb_daten (§ 33b Abs. 5). `runner`
    wird lokal importiert wie in jedem quantitaet-Zweig. Der frühere Parameter `c_fn` ist
    entfallen — er wurde nie gelesen, die Funktion baut ihr `_c` selbst aus f_dict."""
    import runner  # noqa: F401
    def _c(fid):
        v = f_dict.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
    # § 35a Einzelaufstellung (Anlass 2026-08-10, checkESt rc=610001002 ohne Einz-Kz): die
    # Sum-Kz sind seit dem Fix askable:false, ihr Ring-Input kommt aus der STUMPFEN Σ über
    # alle <gruppe>-Instanzen (Instanz-Reuse, Naht wie GWG/Vermietung/Rente — Instanz 1 =
    # bare feld_id). store/bindung/nur_bestaetigt hier via Closure aus _bescheid_fn (nested
    # def, siehe _kind_behinderten_pb_daten-Präzedenz). Ohne store (Alt-Aufrufer) nur die
    # Instanz-1-Basis aus f_dict — genau wie _gwg_sofortabzug_summe.
    def _hh_summe(betrag_fid, gruppe, sum_fid):
        if store is None or bindung is None:
            return _c(betrag_fid)
        total = 0
        for inst in EM.instanzen(store, bindung, gruppe):
            if nur_bestaetigt and inst["zustand"] != "bestaetigt":
                continue
            v = inst["felder"].get(betrag_fid, {}).get("wert")
            total += int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
        if total > 0:
            return total
        # Bestandsdaten-Fallback (test_p35a_bestandsdaten.py, Nebenbefund aus 03aa10b): ohne
        # Instanz auf den alten Flat-Wert zurückfallen. Sonst rechnete der Ring 0 für einen
        # Fall, der die Summe schon bestätigt hat, während _mit_ring_werten (Deklaration)
        # denselben Flat-Wert weiter zeigt — Ring und XML liefen auseinander. `_c` liest aus
        # f_dict, das bei nur_bestaetigt=True schon auf zustand=bestaetigt gefiltert ist
        # (Zeile 291f., Zwei-Signal-Invariant) — der Fallback erbt den Schutz, kein eigener
        # Zustandscheck nötig.
        return _c(sum_fid)
    base = runner.catala_p35a_haushaltsnahe({
        "hh_minijob_aufwendungen": _hh_summe("hh_minijob_betrag", "hh_minijob", "hh_minijob_aufwendungen") // 100,
        "hh_dienstleistungen": _hh_summe("hh_dienstleistung_betrag", "hh_dienstleistung", "hh_dienstleistungen") // 100,
        "hh_handwerker_arbeitskosten": _hh_summe("hh_handwerker_betrag", "hh_handwerker", "hh_handwerker_arbeitskosten") // 100,
        "hh_in_eu_ewr": f_dict.get("hh_in_eu_ewr", {}),
        "hh_rechnung_unbar": f_dict.get("hh_rechnung_unbar", {}),
        "p35a_mitveranlagung": f_dict.get("p35a_mitveranlagung", {})})
    g_dict["steuerermaessigungen"] = base
    # § 35c Abs. 3 S. 2: "Die Steuerermäßigung nach Absatz 1 ist ebenfalls nicht zu gewähren, wenn
    # für die energetischen Maßnahmen eine Steuerbegünstigung nach § 10f oder eine Steuerermäßigung
    # nach § 35a in Anspruch genommen wird oder es sich um eine öffentlich geförderte Maßnahme
    # handelt, für die zinsverbilligte Darlehen oder steuerfreie Zuschüsse in Anspruch genommen
    # werden." NICHT anteilig — die Ermäßigung entfällt ganz.
    #
    # Gemessen 2026-08-16, VOR diesem Guard: dieselben 20.000 EUR einmal als § 35a-Handwerker und
    # einmal als § 35c-Sanierung ergaben 1.200 + 1.400 = 2.600 EUR Entlastung. Zulässig sind
    # höchstens 1.400 (oder 1.200, wenn der Nutzer § 35a wählt) — 1.200 EUR zu wenig Steuer.
    #
    # Der Guard sitzt hier im Ring und nicht in der Regel: die Bedingung
    # keine_10f_35a_oeffentliche_foerderung war von Anfang an als "Komplementär-Guard im Ring"
    # vorgesehen (Lücken-Eintrag der Bindung). Bestätigt-False heißt "es gibt eine solche
    # Doppelförderung" -> beide § 35c-Töpfe auf 0. Unbeantwortet sperrt der Guard in
    # _an_gesamt_sperrgrund (p35c_doppelfoerderung_offen), hier zählt also nur der klare Fall.
    _p35c_doppelt = f_dict.get("p35c_keine_doppelfoerderung", {}).get("wert") is False
    p35c_sanierung = 0 if _p35c_doppelt else runner.catala_p35c_sanierung({
        "sanierungsaufwendungen": _c("p35c_sanierungsaufwendungen") // 100,
        "ist_uebernaechstes_foerderjahr": f_dict.get("p35c_ist_uebernaechstes_foerderjahr", {}).get("wert") is True})
    p35c_energieberater = 0 if _p35c_doppelt else runner.catala_p35c_energieberater({
        "energieberater_aufwendungen": _c("p35c_energieberater_aufwendungen") // 100})
    p35c_deckel = runner.catala_p35c_jahresdeckel({
        "sanierung_ermaessigung": p35c_sanierung,
        "energieberater_ermaessigung": p35c_energieberater,
        "ist_uebernaechstes_foerderjahr": f_dict.get("p35c_ist_uebernaechstes_foerderjahr", {}).get("wert") is True})
    g_dict["steuerermaessigungen"] += p35c_deckel
    g_dict["sonderausgaben"] = (runner.catala_p10b_spenden({
            "zuwendungen": _c("spenden_betrag") // 100, "gesamtbetrag_der_einkuenfte": gde})
        + runner.catala_p10_kist({
            "gezahlte_kirchensteuer": _c("kist_gezahlt") // 100,
            "erstattete_kirchensteuer": _c("kist_erstattet") // 100})
        + runner.catala_p10_kv_pv({
            "basis_kv_pv": (_c("basis_kv") + _c("basis_pv")
                            + _kind_kv_pv_summe(store, bindung, nur_bestaetigt)) // 100,
            "weitere_vorsorgeaufwendungen": (_c("vorsorge_arbeitslosenversicherung") + _c("vorsorge_erwerbsunfaehigkeit") + _c("vorsorge_unfall_haftpflicht") + _c("vorsorge_rv_alt_mit_ueberschuss") + _c("vorsorge_rv_alt_ohne_ueberschuss")) // 100,
            "mit_anspruch_auf_zuschuss": f_dict.get("mit_anspruch_auf_zuschuss", {}).get("wert") is True})
        + _kinderbetreuung_summe(store, bindung, nur_bestaetigt, vz)
        + _schulgeld_summe(store, bindung, nur_bestaetigt, vz, f_dict)
        + (runner.catala_p10_1a_realsplitting({
            "unterhaltsleistungen": _c("realsplitting_unterhaltsleistungen") // 100,
            "kv_pv_beitraege": _c("realsplitting_empfaenger_kv_pv") // 100,
            # TEILMENGE von kv_pv_beitraege, kein zusätzlicher Summand: der Anteil, aus dem ein
            # Anspruch auf Krankengeld folgen kann, geht nur zu 96 % in die Deckel-Erhöhung ein
            # (§ 10 Abs. 1 Nr. 3 Buchst. a S. 4, über die Verweisung in Abs. 1a Nr. 1 S. 2).
            # Fehlt der Wert, ist die Kürzung 0 — dann rechnet die Regel wie vor dem Hand-Fix.
            "kv_krankengeld": _c("realsplitting_empfaenger_kv_krankengeld") // 100})
           if f_dict.get("realsplitting_zustimmung", {}).get("wert") is True else 0)
        + (runner.catala_p10_kv_pv({
            "basis_kv_pv": (_c("basis_kv_partner") + _c("basis_pv_partner")) // 100,
            "weitere_vorsorgeaufwendungen": (_c("vorsorge_arbeitslosenversicherung_partner") + _c("vorsorge_erwerbsunfaehigkeit_partner") + _c("vorsorge_unfall_haftpflicht_partner") + _c("vorsorge_rv_alt_mit_ueberschuss_partner") + _c("vorsorge_rv_alt_ohne_ueberschuss_partner")) // 100,
            "mit_anspruch_auf_zuschuss": f_dict.get("mit_anspruch_auf_zuschuss_partner", {}).get("wert") is True})
           if veranlagung == "zusammen" else 0)
        + runner.catala_p10_1_7_berufsausbildung({
            "berufsausbildung_aufwendungen": _c("berufsausbildung_aufwendungen") // 100}))
    # § 33b Abs. 5 S. 4: "In diesen Fällen besteht für Aufwendungen, für die der
    # Behinderten-Pauschbetrag gilt, kein Anspruch auf eine Steuerermäßigung nach § 33".
    # "In diesen Fällen" = Übertragung des Kind-PB (Abs. 5 S. 1) — Automatismus, Ring
    # kürzt selbst. § 33b Abs. 1 S. 1 (Stufe 2b): eigener GdB-PB gilt ANSTELLE der agB-
    # Ermäßigung für dieselben Aufwendungen — WAHLRECHT, S.2 nur einheitlich ausübbar,
    # nie beides. wahlrecht_pb=True (PB gewählt) kürzt agb wie Abs.5 S.4. wahlrecht_pb=
    # False (Einzelnachweis gewählt): agb bleibt voll, aber dann muss der eigene PB aus
    # ausserg raus — sonst wäre JEDE Antwort ein Doppelabzug (die Bauanleitungs-Skizze
    # ließ das offen: 40k/GdB100/3000EUR agB ergäbe bei false weiter 1.131 statt der
    # korrekten 259 EUR agB-only-Wirkung, siehe reports/adjudikation/
    # p33b_stufe2_bauanleitung_2026-08-07.md Z.168-176 vs. Referenzzahlen Z.101-107).
    # eigener_pb_eur exakt wie an den ausserg-Aufbaustellen (api.py ~966), damit beide
    # Stellen bit-identisch bleiben. Beide Summanden in CENT, genau eine Division am Ende.
    agb_cent = _c("agb_aufwendungen")
    eigener_pb_eur = runner.catala_behinderten_pb({
        "veranlagungszeitraum": vz,
        "grad_der_behinderung": _c("rentner_grad_der_behinderung"),
        "ist_hilflos_blind_taubblind": f_dict.get("rentner_hilflos_blind_taubblind", {}).get("wert") is True})
    wahlrecht_pb = f_dict.get("behinderungsbedingte_aufwendungen_wahlrecht_pb", {}).get("wert")
    if _kind_behinderten_pb_daten(store, bindung, nur_bestaetigt):
        agb_cent = max(0, agb_cent - _c("behinderungsbedingte_aufwendungen"))
    elif eigener_pb_eur > 0 and wahlrecht_pb is True:
        agb_cent = max(0, agb_cent - _c("behinderungsbedingte_aufwendungen"))
    elif eigener_pb_eur > 0 and wahlrecht_pb is False:
        ausserg = max(0, ausserg - eigener_pb_eur)
    # Partner-Spiegel (BACKLOG p33b-partner-pb-doppelabzug): der Partner-PB lief bisher
    # unconditional additiv (Aufbaustelle api.py ~1035/~1425), OHNE dass ein Wahlrecht/
    # Sperrgrund geprüft wurde — 1.168-1.234 EUR stiller Doppelabzug. § 33b Abs. 1 S. 1
    # gilt PRO PERSON (Subjekt individuell), Abs. 1 S. 2 "einheitlich" bindet nur EINE
    # Person über ihre eigenen Aufwandsarten hinweg, nicht zwei Ehegatten aneinander —
    # deshalb EIGENSTÄNDIGE if/elif-Kette (nicht an Person A elif-gekettet: beide Wahlrechte
    # sind unabhängig, jede Kombination ist möglich). partner_pb_eur bit-identisch zu den
    # beiden ausserg-Aufbaustellen. Nur zusammen — Partnerfelder sind sonst nicht gesetzt.
    if veranlagung == "zusammen":
        partner_pb_eur = runner.catala_behinderten_pb({
            "veranlagungszeitraum": vz,
            "grad_der_behinderung": _c("rentner_grad_der_behinderung_partner"),
            "ist_hilflos_blind_taubblind": f_dict.get("rentner_hilflos_blind_taubblind_partner", {}).get("wert") is True})
        wahlrecht_pb_partner = f_dict.get("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner", {}).get("wert")
        if partner_pb_eur > 0 and wahlrecht_pb_partner is True:
            agb_cent = max(0, agb_cent - _c("behinderungsbedingte_aufwendungen_partner"))
        elif partner_pb_eur > 0 and wahlrecht_pb_partner is False:
            ausserg = max(0, ausserg - partner_pb_eur)
    g_dict["aussergewoehnliche_belastungen"] = ausserg + runner.catala_p33_agb({
        "aussergewoehnliche_belastungen": (agb_cent // 100
            + runner.catala_p33_2a_fahrtkostenpauschale({
                "veranlagungszeitraum": vz,
                "hat_gdb80_oder_70g": f_dict.get("fahrtkosten_pausch_gdb80_oder_70g", {}).get("wert") is True,
                "hat_ag_bl_tbl_h": f_dict.get("fahrtkosten_pausch_ag_bl_tbl_h", {}).get("wert") is True})),
        "gesamtbetrag_der_einkuenfte": gde, "anzahl_kinder": _c("fam_anzahl_kinder"),
        "splitting": veranlagung == "zusammen"})


def _p33b_kind_pauschbetraege(store, bindung: dict, nur_bestaetigt: bool, vz: int) -> int:
    """Summe der auf die Eltern übertragenen Kind-Pauschbeträge (§ 33b Abs. 5) — additiv zum
    eigenen PB, deshalb ein eigener Betrag statt einer Kürzung.

    Extrahiert aus BEIDEN Zweig-Funktionen (Phase 2b, 2026-08-17), wo dieselben neun Zeilen
    byte-identisch standen. Die Duplikation ist keine Kosmetik: derselbe Paragraf zweimal
    gepflegt ist genau die Bauart des Kirchensteuer-Doppelbugs, bei dem ein Fix nur in EINER
    Kopie ankam und die beiden Zweige in entgegengesetzte Geldrichtungen liefen. Ein
    Funktionsaufruf mit gleicher Eingabe kann nicht auseinanderlaufen — deshalb ist die
    Extraktion der Fix und nicht nur die Aufräumarbeit.

    Abgesichert durch tests/test_zweig_duplikation_differential.py: catala_behinderten_pb und
    catala_hinterbliebenen_pb gehören zu den neun Rechenstellen, die dort in beiden Zweigen auf
    Cent-Gleichheit geprüft werden."""
    import runner
    summe = 0
    for kd in _kind_behinderten_pb_daten(store, bindung, nur_bestaetigt):
        summe += runner.catala_behinderten_pb({
            "veranlagungszeitraum": vz,
            "grad_der_behinderung": kd["grad_der_behinderung"],
            "ist_hilflos_blind_taubblind": kd["ist_hilflos_blind_taubblind"]})
        if kd["hat_hinterbliebenenbezuege"]:
            summe += runner.catala_hinterbliebenen_pb({
                "veranlagungszeitraum": vz,
                "hat_hinterbliebenenbezuege": True})
    return summe
