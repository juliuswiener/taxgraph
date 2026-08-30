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

# Flag (behauptet Abwesenheit) -> die echten Einkunfts-Betragsfelder derselben Art (§ 2 Abs. 1).
FLAG_NEGIERT = {
    "kein_kap":      ["kap_kapitalertraege", "kap_gewinn_aktien"],          # § 2 Abs. 1 Nr. 5
    "kein_vuv":      ["vv_einnahmen"],                                       # § 2 Abs. 1 Nr. 6
    "kein_sonstige": ["rentner_jahresrente"],                               # § 2 Abs. 1 Nr. 7 (Renten)
    # Partner-Spiegel (§ 26b: Einkuenfte je Ehegatten getrennt ermittelt) derselben zwei Flags oben.
    # Person As eigene Liste deckt nur zwei von fuenf Kap-Feldern (nicht kap_gewinn_sonstige/
    # kap_verlust_aktien/kap_verlust_sonstige) -- gespiegelt, nicht erweitert: der Partner soll
    # denselben Schutz wie Person A bekommen, keinen groesseren. 2026-08-30, kap_partner-Defekt:
    # Korrektur kein_kap_partner=True nach bereits bestaetigtem kap_kapitalertraege_partner>0 blieb
    # unentdeckt, 750 EUR Steuer auf zurueckgezogene Einkuenfte liefen ohne Sperre weiter.
    "kein_kap_partner":      ["kap_kapitalertraege_partner", "kap_gewinn_aktien_partner"],
    # kein_sonstige_partner: die Aussage ist richtig, ABER gemessen 2026-08-31 (HEAD 5af945c) derzeit
    # unfeuerbar -- kein Bug in dieser Zeile, sondern ein disjunkter Scheiben-Zuschnitt eine Ebene
    # hoeher. Das Flag lebt nur auf Scheibe "gesamt" (PARTNER_SCREENING in api_constants.py), das
    # Zielfeld rentner_jahresrente_partner nur auf Scheibe "rentner_gesamt" (RENTNER_22_PARTNER via
    # RENTNER_FELDER) -- api.py::event() laesst pro Fall nur Felder der EIGENEN Scheibe zu (HTTP 400
    # sonst), Flag und Ziel koennen also nie im selben Fall-Snapshot koexistieren. flag_widersprueche()
    # sieht die beiden deshalb nie zusammen; der Eintrag bleibt, bis der Scheiben-Zuschnitt es aendert
    # (s. tests/test_kein_sonstige_partner_korrektur_ueberlebt_widerspruch.py, xfail(strict=True) als
    # Melder fuer genau diesen Tag).
    "kein_sonstige_partner": ["rentner_jahresrente_partner"],               # § 22 Nr. 1, Partner-Rente
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


from _helpers import _bestaetigt_wert


def flag_widersprueche(snapshot: dict) -> list:
    """{feld_id -> {wert, zustand, ...}} → Liste der Flag↔Einkunftsart-Widersprüche.
    Ein Widerspruch: das Flag ist bestätigt=true (Abwesenheit behauptet) UND ein negiertes Einkunfts-
    Feld ist bestätigt mit Wert > 0. Rein deterministisch, kein Rate-Wert."""
    # Lesbare Namen für die Flags (aus fragetext_laie der Bindung)
    FLAG_NAME = {
        "kein_gewinn": "Gewinneinkünfte (Gewerbe, Landwirtschaft, selbständige Arbeit)",
        "kein_kap": "Kapitalerträge (Zinsen, Dividenden, Kursgewinne)",
        "kein_vuv": "Einnahmen aus Vermietung oder Verpachtung",
        "kein_sonstige": "sonstige Einkünfte (z. B. Renten, private Verkäufe)",
        "kein_kap_partner": "Kapitalerträge deines Partners (Zinsen, Dividenden, Kursgewinne)",
        "kein_sonstige_partner": "sonstige Einkünfte deines Partners (z. B. Renten)",
    }
    # Lesbare Namen für die negierten Felder
    FELD_NAME = {
        "kap_kapitalertraege": "Kapitaleinkünfte",
        "kap_gewinn_aktien": "Aktiengewinne",
        "vv_einnahmen": "Einnahmen aus Vermietung",
        "rentner_jahresrente": "Renteneinkünfte",
        "kap_kapitalertraege_partner": "Kapitaleinkünfte des Partners",
        "kap_gewinn_aktien_partner": "Aktiengewinne des Partners",
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
    }

    widersprueche = []
    for flag, felder in FLAG_NEGIERT.items():
        if _bestaetigt_wert(snapshot, flag) is not True:
            continue                                    # Flag nicht bestätigt-true -> keine Behauptung
        flag_titel = FLAG_NAME.get(flag, flag)
        for feld_id in felder:
            wert = _bestaetigt_wert(snapshot, feld_id)
            if isinstance(wert, (int, float)) and wert > 0:
                feld_titel = FELD_NAME.get(feld_id, feld_id)
                # Cent → Euro für monetäre Felder (Wert > 1000 gilt als Cent)
                betrag = f"{wert // 100} €" if wert > 1000 else str(wert)
                widersprueche.append({
                    "flag": flag, "feld_id": feld_id, "wert": wert,
                    "grund": f"Du hast angegeben, keine {flag_titel} zu haben — "
                             f"bei den {feld_titel} wurden aber {betrag} erfasst. "
                             f"Bitte prüfe, welche der beiden Angaben stimmt."})
    return widersprueche
