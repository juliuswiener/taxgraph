"""Einkuenfte: Gewinn (§§ 13-18), Kapital (§ 20), private Veraeusserungen (§ 23), auslaendische Einkuenfte (DBA) und die Gewerbesteuer-Anrechnung (§ 35).

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

def _gwg_sofortabzug_summe(f: dict, store: dict | None, bindung: dict | None,
                           nur_bestaetigt: bool = True) -> int:
    """§ 6 Abs. 2 GWG-Sofortabzug-Σ (EURO, Stufe 2b) — STUMPFE Σ über ALLE gwg-Instanzen: je Asset
    catala_p6_2_gwg (≤ 800 netto → Sofortabzug, sonst 0 = kein GWG, gehört in die AfA). instanzen-Naht wie
    Multi-Objekt-§21/Multi-Rente-§22 (EM.instanzen(gwg); Basis-Feld = Instanz 1). Ohne store (Alt-Aufrufer) nur
    das Basis-gwg_anschaffungskosten_netto aus f. Naht-CENT → EURO (Accessor nimmt EURO). Ein > 800-„GWG" gibt 0
    (over-tax-safe); der User-Hinweis (hilfe_kurz) lenkt auf die AfA."""
    import runner
    def _abzug(fi: dict) -> int:
        v = fi.get("gwg_anschaffungskosten_netto", {}).get("wert")
        netto = int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
        # ⭐ CENT-GUARD (§ 6 Abs. 2, 800€-Schwelle): netto//100 FLOORT Cent vor dem catala-≤800-Vergleich
        # → 800,01-800,99€ (80001-80099 Cent) würde auf 800 abgerundet fälschlich als GWG durchgehen
        # (under-tax, Sofortabzug statt AfA). Schwelle VOR der Euro-Rundung in Cent prüfen.
        if netto > 80000:
            return 0
        return runner.catala_p6_2_gwg({"gwg_anschaffungskosten_netto": netto // 100})
    if store is not None and bindung is not None:
        # ⭐ SECURITY (Zwei-Signal am Ring, INSTANZ-Pfad): EM.instanzen liest den STORE separat vom bestätigt-
        # gefilterten _bescheid_fn-Snapshot → eine VORLÄUFIGE gwg-Instanz bewegte sonst den Sofortabzug OHNE
        # Confirm (dev-2-Repro: 600€ am Ring). gwg ist OPTIONAL → KEIN Kegel-/Sperr-Gate (anders als vv/rente) →
        # bei nur_bestaetigt=True (festgesetzt) ist der Filter PFLICHT. nur_bestaetigt=False (Estimate /stand) zeigt
        # die vorläufige Wirkung im Range (Parität zum agB-Skalar). inst["zustand"] = per-Instanz-meet.
        return sum(_abzug(inst["felder"]) for inst in EM.instanzen(store, bindung, "gwg")
                   if not nur_bestaetigt or inst["zustand"] == "bestaetigt")
    return _abzug(f)


def _laufender_gewinn_partner(f: dict):
    """§§ 13-18 laufender Gewinn des EHEGATTEN (§ 26b Zusammenveranlagung), EURO.

    Bewusst schmaler als _laufender_gewinn (Person A): Stufe 1 der Partnerachse hat für den
    Ehegatten KEINE EÜR-Felder gebaut — Anlage EÜR (E77) hat anders als Anlage G/S keine
    Person-A/B-Achse, ein betriebseinnahmen_partner wäre totes Wiring (BACKLOG
    partnerseite-gewinneinkuenfte-fehlt-strukturell/eueur_keine_partnerachse). Ebenso gibt es
    keine PV-Partnerfelder, also auch keine § 3 Nr. 72-Kürzung hier. Es bleiben der Direktwert
    und die § 15 Abs. 1 S. 1 Nr. 2-Mitunternehmer-Komponente.

    Returns (laufender_gewinn, mitu) wie die Person-A-Variante. `mitu` ist der
    gewerbesteuerpflichtige Anteil; die § 35-Anrechnung für den Partner (gewst_messbetrag_partner)
    ist NICHT Teil dieser Stufe und bleibt offen — sie wirkt zugunsten des Steuerpflichtigen,
    ihr Fehlen ist also over-tax-safe."""
    import runner

    def _c(fid):
        v = f.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

    mitu = runner.catala_mitunternehmer_einkuenfte({
        "gewinnanteil": _c("gewinnanteil_partner") // 100,
        "verguetung_taetigkeit": _c("verguetung_taetigkeit_partner") // 100,
        "verguetung_darlehen": _c("verguetung_darlehen_partner") // 100,
        "verguetung_ueberlassung": _c("verguetung_ueberlassung_partner") // 100,
    }) if any(_c(k + "_partner") for k in MITU_FELDER) else 0
    return _c("einkuenfte_gewinn_partner") // 100 + mitu, mitu


def _p35_gezahlte_gewst(messbetrag_a: int, hebesatz_a: int,
                        messbetrag_b: int, hebesatz_b: int) -> int:
    """§ 35 Abs. 1 S. 5: der Abzug ist auf die tatsächlich zu zahlende Gewerbesteuer beschränkt.

    JE BETRIEB gerechnet und dann summiert — die Hebesätze zweier Gemeinden sind verschieden,
    ein gemeinsamer Hebesatz auf die Messbetragssumme wäre schlicht eine andere Zahl."""
    return messbetrag_a * hebesatz_a // 100 + messbetrag_b * hebesatz_b // 100


def _p23_ansonsten_einkuenfte(f: dict, store: dict | None, bindung: dict | None,
                                nur_bestaetigt: bool = True) -> int:
    """§23 Private Veräußerungsgeschäfte (Stufe-1), EURO — Σ über ALLE p23_veraeusserung-Instanzen:
    je Veräußerung: gewinn = veraeusserungspreis − AK/HK − WK (cent→euro); Σ pos → gewinn_pvg,
    Σ neg-Beträge → verlust_pvg; Gesamtgewinn → Freigrenze 1000€ (§23 Abs.3 S.5, 999→0/1000→1000);
    Verlusttopf max(0, gewinn_pvg−verlust_pvg) (§23 Abs.3 S.7) = anzusetzende_einkuenfte → ADDITIV
    in einkuenfte_sonstige. Mehrjahr-Verlustvor-/rücktrag (S.8) = Stufe-2-Backlog. Absent→0 (safe)."""
    import runner
    if store is None or bindung is None:
        return 0
    # EM = Modul-Level-Import (Z.38). Der frühere lokale `from produkt.mapping import est_mapping`
    # erzeugte eine ZWEITE Modul-Identität (sys.modules['est_mapping'] + ['produkt.mapping.est_mapping'])
    # und hing am sys.path-Seiteneffekt von golden/runner.py (Audit 2026-08-16, arch-dual-module-identity).
    instanzen = EM.instanzen(store, bindung, "p23_veraeusserung")
    gewinn_pvg = 0
    verlust_pvg = 0
    for inst in instanzen:
        # Zwei-Signal-Filter (Instanz-Pfad, wie gwg/kind/vv_objekt/rente): eine vorläufige
        # p23-Instanz darf bei nur_bestaetigt=True nicht in die festgesetzte Summe.
        if nur_bestaetigt and inst["zustand"] != "bestaetigt":
            continue
        # norm: inst["felder"] nutzt Basis-feld_ids (OHNE __n-Suffix); inst["felder"][fid] ist
        # {wert, zustand, herkunft} (EM.instanzen()) — .get("wert") + isinstance-Guard wie an
        # den anderen inst["felder"]-Stellen in dieser Datei (_kind_kv_pv_summe u.a.).
        _preis_v = inst["felder"].get("p23_veraeusserungspreis", {}).get("wert")
        _ak_v = inst["felder"].get("p23_anschaffung_herstellungskosten", {}).get("wert")
        _wk_v = inst["felder"].get("p23_werbungskosten", {}).get("wert")
        preis = int(_preis_v) // 100 if isinstance(_preis_v, (int, float)) and not isinstance(_preis_v, bool) else 0
        ak = int(_ak_v) // 100 if isinstance(_ak_v, (int, float)) and not isinstance(_ak_v, bool) else 0
        wk = int(_wk_v) // 100 if isinstance(_wk_v, (int, float)) and not isinstance(_wk_v, bool) else 0
        gewinn = runner.catala_p23_veraeusserungsgewinn({
            "veraeusserungspreis": preis, "anschaffungs_herstellungskosten": ak, "werbungskosten": wk})
        if gewinn > 0:
            gewinn_pvg += gewinn
        else:
            verlust_pvg += abs(gewinn)
    # Freigrenze auf Gesamtsumme positive+negative = gesamtgewinn
    gesamtgewinn = gewinn_pvg - verlust_pvg
    steuerpflichtig = runner.catala_p23_freigrenze({"gesamtgewinn": gesamtgewinn})
    if steuerpflichtig <= 0:
        return 0
    # Verlusttopf (same-year) auf die FREIGRENZEN-gerechneten Einzelkomponenten
    return runner.catala_p23_verlusttopf({"gewinn_pvg": gewinn_pvg, "verlust_pvg": verlust_pvg})


def _shared_dba_sonstige(g_dict, gde_p10d, veranlagung, f_dict, vz: int):
    """Füllt g_dict: sonstige_abzuege_vom_einkommen (§33a + §10d_2),
    anzurechnende_auslaendische_steuern (§34c). Gibt dba_anrechnung (EURO) zurück.
    f_dict: der Feld-Snapshot der aufrufenden Quantitaet (wertgleich gesamt/rentner).

    Aus _bescheid_fn herausgezogen (Refactor 2026-08-13, Schritt 4). `vz` ist jetzt ein
    Parameter statt einer Closure-Variablen; `runner` wird lokal importiert wie in jedem
    quantitaet-Zweig. Der frühere Parameter `c_fn` ist entfallen — er wurde nie gelesen,
    die Funktion baut ihr `_c` selbst aus f_dict."""
    import runner  # noqa: F401
    def _c(fid):
        v = f_dict.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
    p33a_unt = runner.catala_p33a_unterhalt({
        "veranlagungszeitraum": vz,
        "aufwendungen": _c("p33a_unterhalt_aufwendungen") // 100,
        "kv_pv_beitraege": _c("p33a_unterhalt_kv_pv") // 100,
        "andere_einkuenfte_bezuege": _c("p33a_andere_einkuenfte_bezuege") // 100})
    p33a_ausb = runner.catala_p33a_ausbildungsfreibetrag({
        "anzahl_kinder": _c("p33a_ausbildung_anzahl_kinder")})
    g_dict["sonstige_abzuege_vom_einkommen"] = (runner.catala_p10d_2({
            "gesamtbetrag_einkuenfte": gde_p10d,
            "verlustvortrag_bestand": _c("verlustvortrag_bestand") // 100,
            "zusammenveranlagung": veranlagung == "zusammen"})
        + p33a_unt + p33a_ausb)
    dba_anrechnung = 0
    dba_gezahlt = _c("dba_gezahlte_auslaendische_steuer") // 100
    dba_ausl = _c("dba_auslaendische_einkuenfte") // 100
    dba_staat_raw = f_dict.get("dba_staat", {}).get("wert")
    dba_method_from_user = f_dict.get("dba_methode", {}).get("wert")
    dba_effective_method = "freistellung" if dba_method_from_user == "dba_freistellung" \
        else dba_methode_fuer(dba_staat_raw, f_dict.get("dba_einkunftsart", {}).get("wert"))
    if f_dict.get("dba_abzug_statt_anrechnung", {}).get("wert") is True and dba_gezahlt > 0 and dba_ausl > 0:
        g_dict["sonstige_abzuege_vom_einkommen"] += dba_gezahlt
    elif dba_gezahlt > 0 or dba_ausl > 0:
        if dba_effective_method == "freistellung":
            dba_anrechnung = 0
            g_dict["p32b_progressionseinkuenfte"] = dba_ausl
        else:
            dba_anrechnung = runner.catala_p34c_1({
                "gezahlte_auslaendische_steuer": dba_gezahlt,
                "deutsche_est_inkl_ausl": runner.catala_gesamt_tarifliche(g_dict),
                "zu_versteuerndes_einkommen": runner.catala_gesamt_zve(g_dict),
                "auslaendische_einkuenfte_staat": dba_ausl})
    g_dict["anzurechnende_auslaendische_steuern"] = dba_anrechnung
    return dba_anrechnung


def _p20_kapitaleinkuenfte(_c, zusammen: bool, vz: int) -> int:
    """§ 20 Abs. 6 Verlustverrechnung + § 20 Abs. 9 Sparer-Pauschbetrag, für beide Zweige.

    Töpfe XOR Aggregat: sind einzelne Töpfe belegt, entscheidet die Verrechnung; sonst zählt
    der Aggregat-Betrag. Bei Zusammenveranlagung kommt das Kapital des Ehegatten ROH dazu,
    VOR dem gemeinsamen Sparer-PB — der wird über `zusammenveranlagung` verdoppelt
    (§ 20 Abs. 9 S. 3), nicht zweimal einzeln gewährt.

    `zusammen` wird bewusst übergeben statt hier aus einem Zweig-Dict gelesen: der gesamt-Zweig
    braucht dieselbe Größe an einem Dutzend weiterer Stellen, und zwei Herleitungen desselben
    Merkmals wären wieder der Anfang einer Divergenz. Die Veranlagungsart kommt ohnehin
    AUSSCHLIESSLICH aus § 26 — ein zweites Feld dafür gab es schon einmal, es widersprach der
    Veranlagungsart und kostete 250 EUR Steuer (entfernt 2026-07-30).

    Extrahiert aus beiden Zweigen (Phase 2b, 2026-08-17), wo die zwölf Zeilen bis auf die
    Herkunft von `zusammen` identisch standen. Abgesichert durch
    tests/test_zweig_duplikation_differential.py: catala_kapital_verrechnung und
    catala_sparer_pb gehören zu den Rechenstellen, die dort auf Cent-Gleichheit geprüft werden.
    """
    import runner
    if any(_c(t) != 0 for t in KAP_TOEPFE):
        verrechnete = runner.catala_kapital_verrechnung({
            "gewinn_aktien": _c("kap_gewinn_aktien") // 100,
            "verlust_aktien": _c("kap_verlust_aktien") // 100,
            "gewinn_sonstige": _c("kap_gewinn_sonstige") // 100,
            "verlust_sonstige": _c("kap_verlust_sonstige") // 100})
    else:
        verrechnete = _c(KAP_ERTRAEGE) // 100
    if zusammen:
        if any(_c(t) != 0 for t in KAP_TOEPFE_PARTNER):
            verrechnete += runner.catala_kapital_verrechnung({
                "gewinn_aktien": _c("kap_gewinn_aktien_partner") // 100,
                "verlust_aktien": _c("kap_verlust_aktien_partner") // 100,
                "gewinn_sonstige": _c("kap_gewinn_sonstige_partner") // 100,
                "verlust_sonstige": _c("kap_verlust_sonstige_partner") // 100})
        else:
            verrechnete += _c(KAP_ERTRAEGE_PARTNER) // 100
    return runner.catala_sparer_pb({
        "veranlagungszeitraum": vz, "kapitalertraege": verrechnete,
        "zusammenveranlagung": zusammen})


def _laufender_gewinn(f: dict, store: dict | None = None, bindung: dict | None = None,
                      nur_bestaetigt: bool = True):
    """§§ 13-18 laufender Gewinn (§ 15 Gewerbe / § 18 selbständig), EURO — die EINE Quelle für den laufenden
    (Nicht-Veräußerungs-)Gewinn, geteilt von gesamt- und rentner-Ring (Scope A). § 4 Abs. 3 EÜR (Stufe 2a/2b) wenn
    IRGENDEINE EÜR-Komponente (betriebseinnahmen/sonstige_betriebsausgaben/afa_jahresbetrag) ODER ein GWG vorliegt:
    gewinn = betriebseinnahmen − (sonstige_BA + AfA + GWG-Σ) via catala_euer_gewinn — KANN NEGATIV sein (Verlustjahr
    → § 2 Abs. 3-Ausgleich mindert andere Einkünfte); sonst der direkte einkuenfte_gewinn-Wert (Stufe 1). Der
    GWG-Sofortabzug (§ 6 Abs. 2, Stufe 2b) mindert als Betriebsausgabe den Gewinn (Σ über alle Assets). Naht-CENT
    → EURO. BEIDE Quellen (Direktwert + EÜR) sperrt gewinn_quelle_offen VORHER; land_forst + EÜR sperrt luf_euer_offen.

    Returns (laufender_gewinn, mitu): laufender_gewinn = kompletter §2-Gewinn (inkl. mitu-Summand);
    mitu = §15-Mitunternehmer-Komponente (immer gewerbesteuerpflichtig, §35-Zähler)."""
    import runner
    def _c(fid):
        v = f.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
    gwg_summe = _gwg_sofortabzug_summe(f, store, bindung, nur_bestaetigt)
    # § 15 Abs. 1 S. 1 Nr. 2 Mitunternehmer (#2): SEPARATER §15-gewerblicher Summand (Beteiligung an PersG, additiv
    # zum eigenen Gewerbe/EÜR — KEIN gewinn_quelle_offen-Konflikt, eigene Felder). gewinnanteil = §15a-ausgleichs-
    # fähiger Anteil (kann NEGATIV, roh summiert). Hier IN _laufender_gewinn → symmetrisch in §35-Zähler+Nenner.
    # BOUNDED ASSUMPTION: mitu = ausschl. §15 gewerblich (Anlage G). §18-freiberufl-Mitunternehmerschaft wäre
    # NICHT gewerbesteuerpflichtig → bräuchte eigenes Flag (out-of-scope, Backlog). Instructor-Ruling §35-Mitu.
    mitu = runner.catala_mitunternehmer_einkuenfte({
        "gewinnanteil": _c("gewinnanteil") // 100,
        "verguetung_taetigkeit": _c("verguetung_taetigkeit") // 100,
        "verguetung_darlehen": _c("verguetung_darlehen") // 100,
        "verguetung_ueberlassung": _c("verguetung_ueberlassung") // 100,
    }) if any(_c(k) for k in MITU_FELDER) else 0
    if any(_c(k) for k in EUER_KOMPONENTEN) or gwg_summe > 0:
        gewinn = runner.catala_euer_gewinn({
            "betriebseinnahmen": _c("betriebseinnahmen") // 100,
            "betriebsausgaben": (_c("sonstige_betriebsausgaben") + _c("afa_jahresbetrag")) // 100 + gwg_summe}) + mitu
    else:
        gewinn = _c("einkuenfte_gewinn") // 100 + mitu
    # § 3 Nr. 72: Einnahmen aus Gebäude-PV bis 30 kWp/Einheit und 100 kWp gesamt sind steuerfrei —
    # sie mindern den Gewinn, bevor er in die § 2-Summe geht. Freigrenze: über der Grenze bleibt
    # alles steuerpflichtig (Accessor gibt dann 0 zurück). Nie unter 0 abziehen.
    pv_frei = runner.catala_p3_nr72_photovoltaik({
        "pv_einnahmen": _c("pv_einnahmen") // 100,
        "pv_bruttoleistung_kwp": _c("pv_bruttoleistung_kwp"),
        "pv_anzahl_einheiten": _c("pv_anzahl_einheiten"),
        "pv_auf_gebaeude": f.get("pv_auf_gebaeude", {})})
    if pv_frei > 0:
        gewinn -= min(pv_frei, max(0, gewinn))
    return gewinn, mitu


def _gewinn_partner_anteil(f: dict):
    """Der Beitrag des Ehegatten zu g["einkuenfte_gewinn"] (EURO): laufender Gewinn + § 16-vg
    netto nach EIGENEM § 16 Abs. 4-Freibetrag.

    NUR bei Zusammenveranlagung — bei Einzelveranlagung gibt es in dieser Erklärung keinen
    Ehegatten, dessen Einkünfte mitzuveranlagen wären; ein dort gesetztes Partner-Feld darf die
    eigene Steuer nicht bewegen.

    Der Freibetrag wird EIGENSTÄNDIG gerechnet, nicht auf die Summe beider Gewinne: § 16 Abs. 4
    S. 1 knüpft an den Steuerpflichtigen an, bei Zusammenveranlagung also an jeden Ehegatten
    einzeln. Ein gemeinsamer Freibetrag auf die Summe wäre over-tax
    (test_gewinn_partner_ring::test_p16_4_freibetrag_gilt_je_person misst genau diesen
    Unterschied)."""
    if f.get("veranlagung", {}).get("wert") != "zusammen":
        return 0, 0
    import runner

    def _c(fid):
        v = f.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

    laufend, mitu = _laufender_gewinn_partner(f)
    vg_euro = _c("rentner_veraeusserungsgewinn_partner") // 100
    # GEFLOORT bei 0 wie bei Person A: FB > vg darf keinen Phantom-Verlust erzeugen.
    netto_vg = max(0, vg_euro - runner.catala_p16_4_freibetrag(
        {"rentner_veraeusserungsgewinn": vg_euro}))
    return laufend + netto_vg, mitu


def _p35_partner_anteile(f: dict):
    """(messbetrag_euro, hebesatz, gewerbliche_einkuenfte_euro) des Ehegatten für § 35.

    Nur bei Zusammenveranlagung — sonst (0, 0, 0), dann rechnet unten alles wie vorher.

    Der Zähler nimmt den LAUFENDEN Gewinn ohne § 16-Veräußerungsgewinn (§ 7 S. 2 GewStG: der
    Veräußerungsgewinn einer natürlichen Person gehört nicht zum Gewerbeertrag), deshalb
    _laufender_gewinn_partner und nicht _gewinn_partner_anteil. Betriebsart-Weiche wie bei
    Person A: nur ein Gewerbebetrieb liefert den vollen laufenden Gewinn, sonst zählt allein der
    § 15-Mitunternehmeranteil (§ 18-selbständig und § 13-LuF unterliegen keiner Gewerbesteuer)."""
    if f.get("veranlagung", {}).get("wert") != "zusammen":
        return 0, 0, 0

    def _c(fid):
        v = f.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

    laufend, mitu = _laufender_gewinn_partner(f)
    zaehler = (max(0, laufend) if f.get("gewinn_betriebsart_partner", {}).get("wert") == "gewerbe"
               else max(0, mitu))
    return _c("gewst_messbetrag_partner") // 100, _c("gewst_hebesatz_partner"), zaehler


def _p35_summen(f: dict, messbetrag_a: int, hebesatz_a: int, zaehler_a: int):
    """(messbetrag_ges, zaehler_ges, gezahlt) für § 35 — Person A plus Ehegatte.

    EINE Stelle für die Summen, damit die Deckel weiter unten nicht je Verwendung neu addiert
    werden müssen; `gezahlt` ist der Deckel aus Abs. 1 S. 5, JE BETRIEB gerechnet (zwei
    Gemeinden, zwei Hebesätze — ein gemeinsamer Hebesatz auf die Messbetragssumme wäre eine
    andere Zahl, s. _p35_gezahlte_gewst).

    Extrahiert aus beiden Zweigen (Phase 2b, 2026-08-17), wo diese sieben Zeilen byte-identisch
    standen. Die Anteile des Ehegatten kamen schon vorher aus einer gemeinsamen Quelle
    (_p35_partner_anteile) — nur ihre Verrechnung war doppelt gepflegt, und genau dort ist
    schon einmal Geld verfallen: § 35-Partneranteile wurden im Rentner-Ring gar nicht
    angerechnet (7.000 EUR, Fund vom 2026-08-13)."""
    messbetrag_b, hebesatz_b, zaehler_b = _p35_partner_anteile(f)
    return (messbetrag_a + messbetrag_b,
            zaehler_a + zaehler_b,
            _p35_gezahlte_gewst(messbetrag_a, hebesatz_a, messbetrag_b, hebesatz_b))
