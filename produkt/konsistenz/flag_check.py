"""K2-Flag-Konsistenz — Abwesenheits-Flag ↔ echte Einkunftsart-Felder (Task #11, Front V+V). NULL LLM.

Die an_gesamt-Abwesenheits-Flags (kein_gewinn/kein_kap/kein_vuv/kein_sonstige) behaupten die ABWESENHEIT
einer Einkunftsart (kein_X=true ⇒ „diese Einkunftsart liegt nicht vor"). Sind sie GESETZT und trotzdem
ein echtes Einkunfts-Feld dieser Art mit einem Wert > 0 belegt, ist das ein WIDERSPRUCH — der reine-AN-
Ring würde eine Einkunftsart still übergehen (falscher Bescheid). Dieser Guard surft den Widerspruch als
benannte Inkonsistenz (K2: nie still eine Einkunftsart schlucken).

Beispiel V+V (die Ring-relevante Inversion): `kein_vuv=true` + `vv_einnahmen>0` bestätigt → Widerspruch;
für einen echten V+V-Fall muss `kein_vuv=false` sein (dann greift der V+V-Ring statt der Sperre).
"""
from __future__ import annotations

# Falle: ein Flag fehlt hier oben aus GUTEM Grund, nicht aus Versehen -- gemessen 2026-08-31 sind
# kein_gewinn_partner und keine_behinderung_pflege_partner heute nur deshalb harmlos, WEIL sie nicht in
# dieser Liste stehen (Flag nur auf Scheibe "gesamt" fragbar, Zielfelder aber auch auf "rentner_gesamt"
# erreichbar). Vor dem Eintragen erst Szenen-Fragbarkeit von Flag und Zielfeldern gegen _scheibe_bindung()
# pruefen, sonst reproduziert das Eintragen denselben Defekt wie kein_sonstige_partner unten.
# Flag (behauptet Abwesenheit) -> die echten Einkunfts-Betragsfelder derselben Art (§ 2 Abs. 1).
FLAG_NEGIERT = {
    "kein_kap":      ["kap_kapitalertraege", "kap_gewinn_aktien", "kap_verlust_aktien",
                      "kap_gewinn_sonstige", "kap_verlust_sonstige"],        # § 2 Abs. 1 Nr. 5
    "kein_vuv":      ["vv_einnahmen"],                                       # § 2 Abs. 1 Nr. 6
    "kein_sonstige": ["rentner_jahresrente"],                               # § 2 Abs. 1 Nr. 7 (Renten)
    # Partner-Spiegel (§ 26b: Einkuenfte je Ehegatten getrennt ermittelt) derselben fuenf Kap-Felder oben.
    # 2026-08-31: Person As Liste wurde von zwei auf fuenf Felder erweitert; diese Zeile zieht mit, sonst
    # waere die Spiegel-Begruendung unwahr geworden. kap_gewinn_sonstige_partner/kap_verlust_aktien_partner/
    # kap_verlust_sonstige_partner haben denselben feld_bedingung-Mechanismus (bindung_kap_vv_familie.yaml)
    # wie die zwei urspruenglichen Felder: die Bedingung verhindert nur NEUE Eingabe, nicht das Fortbestehen
    # eines bereits bestaetigten Vorjahres-/Import-Werts nach einer spaeteren Korrektur auf true. 2026-08-30,
    # kap_partner-Defekt: Korrektur kein_kap_partner=True nach bereits bestaetigtem kap_kapitalertraege_partner>0
    # blieb unentdeckt, 750 EUR Steuer auf zurueckgezogene Einkuenfte liefen ohne Sperre weiter.
    "kein_kap_partner":      ["kap_kapitalertraege_partner", "kap_gewinn_aktien_partner",
                              "kap_verlust_aktien_partner", "kap_gewinn_sonstige_partner",
                              "kap_verlust_sonstige_partner"],
    # kein_sonstige_partner: Flag lebt nur auf Scheibe "gesamt" (PARTNER_SCREENING in api_constants.py),
    # das Zielfeld rentner_jahresrente_partner nur auf Scheibe "rentner_gesamt" (RENTNER_22_PARTNER via
    # RENTNER_FELDER) -- Flag und Zielfeld koennen NIE im selben Snapshot koexistieren, weil die Scheiben
    # disjunkt sind. Genau das ist die Ursache, nicht der Schutz: die Pruefung unten braucht Koexistenz
    # nie, ihr genuegt die Abwesenheit des Flags. Ein bestaetigter rentner_jahresrente_partner-Betrag auf
    # rentner_gesamt trifft auf ein Flag, das dort nie gefragt wurde und darum als "unbeantwortet -> wie
    # bestaetigt-true" gilt (s. Drei-Zustaende-Logik unten). Reproduziert 2026-08-31: 1.500.000 Cent
    # rentner_jahresrente_partner auf rentner_gesamt, Zusammenveranlagung, loeste faelschlich
    # flag_konsistenz_offen aus. Der bindung-Parameter unten schliesst genau diese Luecke.
    "kein_sonstige_partner": ["rentner_jahresrente_partner"],               # § 22 Nr. 1, Partner-Rente
    # kein_p23_verkauf: § 23 Abs. 1/3 EStG, private Veraeusserungsgeschaefte. Alle drei Betragsfelder
    # sind ausschliesslich instanz_gruppe:p23_veraeusserung (bindung_p23_gesamt.yaml), gespeist einzig
    # in regel_id:p23_veraeusserungsgewinn -- kein anderer Verwendungszweck, anders als geprueft bei den
    # Kap-Feldern. 2026-08-31: Luecke zwischen zwei Waechtern gemessen -- _an_gesamt_sperrgrund()s
    # fremd_arten feuert nur bei bestaetigt-FALSE, dieser Guard beobachtete das Flag bis hierher gar
    # nicht; ein unbeantwortetes Kreuz neben einem bestaetigten §23-Gewinn rechnete durch (gemessen:
    # 3.265.600 ct zahl_cent). Der Betrag verschwindet dabei NICHT: er wird korrekt besteuert, aber
    # nicht erklaert -- est_mapping.py (P23_GEWINN) vergibt die Kennzahl (E0306801) nur, wenn
    # p23_veraeusserungs_typ bestaetigt ist; bleibt das Kreuz unbeantwortet, bleibt auch die Art
    # unbestaetigt, der Kz-Zweig schreibt sich selbst einen Vermerk ("unbestaetigt -- Kz-Zweig offen")
    # und bleibt leer, waehrend dieselbe Zahl in der Endsumme steckt. Dieser Eintrag deckt
    # "unbeantwortet" UND "bestaetigt-true trotz Betrag" ab; "bestaetigt-false" bleibt allein
    # fremd_arten (Julius-Wort: Befreiungstatbestaende Eigennutzung/Frist/taeglicher Gebrauch fehlen
    # dort als Feld).
    "kein_p23_verkauf": ["p23_veraeusserungspreis", "p23_anschaffung_herstellungskosten",
                         "p23_werbungskosten"],
    # § 2 Abs. 1 Nr. 1-3 (§§ 13-18): Stufe-1-Direktgewinn + § 16-vg (2-I) + EÜR-Komponenten (2-II) + GWG (2-III).
    # ALLE Betriebs-Indikatoren negieren kein_gewinn — auch ein Verlustjahr (einnahmen=0, aber afa/ausgaben>0)
    # oder nur GWG-Anschaffungen belegen einen existierenden Betrieb, dürfen nicht als "kein Gewinn" durchrutschen.
    # gwg_anschaffungskosten_netto ist die Instanz-1-Basis (instanz_gruppe:gwg) — präsent, sobald irgendein GWG
    # vorliegt (Instanz 2..N tragen das Suffix __n, Instanz 1 = Basis; jede GWG-Präsenz setzt die Basis).
    # Mitunternehmer (§ 15 Abs. 1 Nr. 2): Gewinnanteil + 3 Sondervergütungen sind ebenfalls gewerbliche
    # Einkünfte — jede Präsenz belegt einen Betrieb, negiert kein_gewinn (auch ein negativer Anteil = Verlustjahr).
    "kein_gewinn":   ["einkuenfte_gewinn", "rentner_veraeusserungsgewinn",
                      "betriebseinnahmen", "sonstige_betriebsausgaben", "afa_jahresbetrag",
                      "gwg_anschaffungskosten_netto",
                      "gewinnanteil", "verguetung_taetigkeit", "verguetung_darlehen",
                      "verguetung_ueberlassung"],
}


import re

from _helpers import _bestaetigt_wert

# Instanzfähige Zielfelder (instanz_gruppe im YAML, z. B. p23_veraeusserung/vv_objekt/rente/gwg)
# können als feld_id__<n> (n>=2) im Snapshot liegen, Instanz 1 bleibt die Basis-feld_id ohne Suffix
# (dieselbe Konvention wie est_mapping.py::parse_instanz, hier bewusst NICHT importiert -- dieser
# Guard braucht keine Kz-Semantik, nur das Namensmuster). 2026-08-31, main: die Instanzenzahl darf
# NICHT aus einem Zaehlfeld (z. B. p23_anzahl_verkaeufe) kommen -- eine dort gemessene Funktion hat
# einen Boden bei 1 (unbeantwortet/0/negativ liefern alle 1), eine bestaetigte "0 Verkaeufe" wuerde
# also trotzdem einmal nachsehen und koennte einen Widerspruch zu einer Instanz melden, die es nicht
# gibt. Stattdessen: nur ueber tatsaechlich im Snapshot vorhandene Schluessel iterieren, die auf
# base oder base__<n> passen. Bewusst generisch (reines Namensmuster, keine Feldliste im Code) --
# main 2026-08-31: eine feldweise Reparatur haette so viele Zweige gebraucht wie betroffene Felder
# und beim naechsten neu instanzfaehig gewordenen Feld erneut gefehlt.
#
# Bekannt geteilte Wurzel (main 2026-08-31, verbindlich ausgezaehlt, 2026-08-31 nachgemessen --
# len(set(f for lst in FLAG_NEGIERT.values() for f in lst)) == 26, nicht 21): von 26 FLAG_NEGIERT-
# Zielfeldern sind 6 instanzfaehig -- p23_veraeusserungspreis/anschaffung_herstellungskosten/
# werbungskosten (kein_p23_verkauf), rentner_jahresrente (kein_sonstige), vv_einnahmen (kein_vuv),
# gwg_anschaffungskosten_netto (kein_gewinn); die uebrigen 20 (kein_kap, kein_kap_partner,
# kein_sonstige_partner -- inkl. rentner_jahresrente_partner, das TROTZ Namensnaehe zu
# rentner_jahresrente NICHT instanzfaehig ist) nicht. Alle sechs haengen an derselben
# Einzel-Schluessel-Fehlform, hier UND an drei weiteren, nachgemessenen Stellen (2026-08-31,
# nicht fuenf wie zuvor unbelegt behauptet -- der ganze restliche Baum ausser diesen beiden Dateien
# und flag_check.py selbst hat NULL Treffer): produkt/haut/api.py:227 (_feste_zahl, Kegel-Meet
# `f in felder` -- der Kegel fuehrt rentner_jahresrente als Basisfeld), produkt/haut/api.py:595
# (_ergebnis_roh, `offen`-Liste `f not in felder`, derselbe Kegel), produkt/mapping/est_mapping.py:415
# (_pflichtfelder_luecken, `f not in snapshot`) -- alle drei pruefen nur die Basis-feld_id, ohne
# __n-Suffix-Kenntnis. Diese Aenderung repariert NUR flag_widersprueche() -- die anderen drei fallen
# fail-closed (ein vorhandenes Instanz-Feld wird als fehlend gemeldet und sperrt/meldet offen) und
# sind NICHT Teil dieser Aenderung.
_INSTANZ_SUFFIX_RE = re.compile(r"^(?P<idx>[1-9][0-9]*)$")


def _instanz_feld_ids(snapshot: dict, basis: str) -> list:
    """Alle im Snapshot tatsächlich vorhandenen Schlüssel für `basis`: die Basis-feld_id selbst
    (Instanz 1) und jedes `basis__<n>` (n>=2), das als Key existiert. Keine Zählung, kein Zaehlfeld,
    keine Obergrenze -- was da ist, ist da."""
    treffer = [basis] if basis in snapshot else []
    prefix = basis + "__"
    for key in snapshot:
        if key.startswith(prefix) and _INSTANZ_SUFFIX_RE.match(key[len(prefix):]):
            treffer.append(key)
    return treffer


def flag_widersprueche(snapshot: dict, bindung: dict | None = None) -> list:
    """{feld_id -> {wert, zustand, ...}} → Liste der Flag↔Einkunftsart-Widersprüche.
    Ein Widerspruch: das Flag ist bestätigt=true (Abwesenheit behauptet) UND ein negiertes Einkunfts-
    Feld ist bestätigt mit Wert > 0. Rein deterministisch, kein Rate-Wert.

    `bindung` (optional, dieselbe Quelle wie der Schreibpfad: api.py::_scheibe_bindung(store),
    Feld-Id -> Bindungseintrag NUR für die Felder der aktuellen Scheibe) trennt "auf dieser Scheibe
    nie gefragt" (Flag fehlt in `bindung` -> strukturell unbeantwortbar hier, kein Widerspruch) von
    "auf dieser Scheibe fragbar, aber unbeantwortet" (Flag fehlt im Snapshot, ABER steht in `bindung`
    -> wie bestätigt-true behandeln, s. u.). Ohne `bindung` (Alt-Aufrufer, Unit-Tests ohne Scheiben-
    Kontext) bleibt das alte Verhalten: jedes im Snapshot fehlende Flag gilt als unbeantwortet."""
    # Lesbare Namen für die Flags (aus fragetext_laie der Bindung)
    FLAG_NAME = {
        "kein_gewinn": "Gewinneinkünfte (Gewerbe, Landwirtschaft, selbständige Arbeit)",
        "kein_kap": "Kapitalerträge (Zinsen, Dividenden, Kursgewinne)",
        "kein_vuv": "Einnahmen aus Vermietung oder Verpachtung",
        "kein_sonstige": "sonstige Einkünfte (z. B. Renten, private Verkäufe)",
        "kein_kap_partner": "Kapitalerträge deines Partners (Zinsen, Dividenden, Kursgewinne)",
        "kein_sonstige_partner": "sonstige Einkünfte deines Partners (z. B. Renten)",
        "kein_p23_verkauf": "private Verkäufe (z. B. Grundstück, Wertpapiere außerhalb §20)",
    }
    # Lesbare Namen für die negierten Felder
    FELD_NAME = {
        "kap_kapitalertraege": "Kapitaleinkünfte",
        "kap_gewinn_aktien": "Aktiengewinne",
        "kap_verlust_aktien": "Aktienverluste",
        "kap_gewinn_sonstige": "sonstige Kapitalgewinne",
        "kap_verlust_sonstige": "sonstige Kapitalverluste",
        "vv_einnahmen": "Einnahmen aus Vermietung",
        "rentner_jahresrente": "Renteneinkünfte",
        "kap_kapitalertraege_partner": "Kapitaleinkünfte des Partners",
        "kap_gewinn_aktien_partner": "Aktiengewinne des Partners",
        "kap_verlust_aktien_partner": "Aktienverluste des Partners",
        "kap_gewinn_sonstige_partner": "sonstige Kapitalgewinne des Partners",
        "kap_verlust_sonstige_partner": "sonstige Kapitalverluste des Partners",
        "rentner_jahresrente_partner": "Renteneinkünfte des Partners",
        "einkuenfte_gewinn": "Gewinneinkünfte",
        "rentner_veraeusserungsgewinn": "Veräußerungsgewinne",
        "betriebseinnahmen": "Betriebseinnahmen",
        "sonstige_betriebsausgaben": "sonstige Betriebsausgaben",
        "afa_jahresbetrag": "Abschreibungen (AfA)",
        "gwg_anschaffungskosten_netto": "GWG-Anschaffungen",
        "gewinnanteil": "Mitunternehmer-Gewinnanteil",
        "verguetung_taetigkeit": "Mitunternehmer-Vergütung (Tätigkeit)",
        "verguetung_darlehen": "Mitunternehmer-Vergütung (Darlehen)",
        "verguetung_ueberlassung": "Mitunternehmer-Vergütung (Überlassung)",
        "p23_veraeusserungspreis": "Veräußerungspreis (privater Verkauf)",
        "p23_anschaffung_herstellungskosten": "Anschaffungs-/Herstellungskosten (privater Verkauf)",
        "p23_werbungskosten": "Werbungskosten (privater Verkauf)",
    }

    widersprueche = []
    for flag, felder in FLAG_NEGIERT.items():
        # Drei Zustände: bestätigt-true (Abwesenheit behauptet -> prüfen), bestätigt-false (Nutzer
        # HAT die Einkunftsart, kein Widerspruch für dieses Flag), unbeantwortet -- Flag gar nicht
        # im Snapshot (nie gefragt). "Nie gefragt" ist NICHT "verneint": ein Betrag, der neben einem
        # nie gestellten Screening-Kreuz steht, ist derselbe stille Widerspruch wie bei explizit
        # bestätigtem Flag -- nur bestätigt-false darf den Zweig überspringen.
        if flag not in snapshot:
            if bindung is not None and flag not in bindung:
                continue                                # auf dieser Scheibe strukturell unfragbar -> kein Widerspruch
            pass                                        # nie gefragt -> wie bestätigt-true behandeln
        elif _bestaetigt_wert(snapshot, flag) is not True:
            continue                                    # bestätigt-false oder nur vorläufig -> keine Behauptung
        flag_titel = FLAG_NAME.get(flag, flag)
        for basis in felder:
            for feld_id in _instanz_feld_ids(snapshot, basis):
                wert = _bestaetigt_wert(snapshot, feld_id)
                if isinstance(wert, (int, float)) and wert > 0:
                    feld_titel = FELD_NAME.get(basis, basis)
                    # Cent → Euro für monetäre Felder (Wert > 1000 gilt als Cent)
                    betrag = f"{wert // 100} €" if wert > 1000 else str(wert)
                    widersprueche.append({
                        "flag": flag, "feld_id": feld_id, "wert": wert,
                        "grund": f"Du hast angegeben, keine {flag_titel} zu haben — "
                                 f"bei den {feld_titel} wurden aber {betrag} erfasst. "
                                 f"Bitte prüfe, welche der beiden Angaben stimmt."})
    return widersprueche
