"""VaSt-Belege (ELSTER-Datenabholung) → TaxGraph-Feld-IDs.

Die vorausgefüllte Steuererklärung liefert je Belegart ein XML nach amtlichem Schema
(ERiC-Auslieferung, Dokumentation/Datenarten/ElsterDatenabholung/ElsterVaStDaten).
Dieses Modul übersetzt daraus die Liste, die `elster_writer.uebernehme_edaten()` erwartet:
[{"feld_id": …, "wert": …, "kategorie": …}].

EINHEITEN — die wichtigste Naht:
    VaSt liefert EURO mit zwei Nachkommastellen ("45000.00"), der Typ heißt im Schema
    Dezimalzahl…MinNK2_MaxNK2. Unser Store führt Beträge als CENT-Integer. Die Umrechnung
    passiert an genau einer Stelle (`_cent`), damit sie nicht mehrfach oder gar nicht
    geschieht. Ein 100-facher Fehler an dieser Stelle wäre eine stille Fehlbesteuerung.

NICHT GEMAPPTE FELDER sind kein Versehen: was hier fehlt, hat entweder kein Gegenstück in
unserer Bindungstabelle oder ist eine Größe, die der Ring selbst berechnet. Die Liste
`NICHT_GEMAPPT` benennt sie mit Grund — schweigendes Weglassen wäre ein Datenverlust, den
niemand bemerkt.

Belegarten (Schema-Jahrgänge 2022–2025):
    LStB   Lohnsteuerbescheinigung
    LErsL  Lohnersatzleistungen (§ 32b)
    RBM    Rentenbezugsmitteilung
    KRV    Beiträge zur Kranken-/Rentenversicherung
"""
from __future__ import annotations

from decimal import Decimal, InvalidOperation

# --------------------------------------------------------------- Einheiten-Naht

def _cent(euro_text: str | None) -> int | None:
    """VaSt-Betrag ("45000.00", Euro mit 2 Nachkommastellen) → Cent-Integer.

    None/leer → None (Feld war im Beleg nicht besetzt; minOccurs=0 ist im Schema
    durchgängig). Kein Wert ist etwas anderes als der Wert 0 — wer 0 schreibt, behauptet
    eine bescheinigte Null.
    """
    if euro_text is None:
        return None
    s = str(euro_text).strip().replace(",", ".")
    if not s:
        return None
    try:
        return int((Decimal(s) * 100).to_integral_value())
    except (InvalidOperation, ValueError):
        raise ValueError(f"VaSt-Betrag nicht lesbar: {euro_text!r}")


# --------------------------------------------------------------- Belegart LStB
# Lohnsteuerbescheinigung. Schema: VaSt_LStB-<jahr>01.xsd
# Die Dokumentationstexte stammen wörtlich aus der xs:documentation des Schemas.

LSTB = {
    # VaSt-Element            feld_id                        Doku aus dem Schema
    "BruttoArbLohn":          ("bruttoarbeitslohn",          "Bruttoarbeitslohn"),
    "LSteuer":                ("p36_lohnsteuer",             "einbehaltene Lohnsteuer"),
    "ArbnKiSteuer":           ("kist_gezahlt",               "einbehaltene Kirchensteuer des Arbeitnehmers"),
    "LeistungenProgVorbeh":   ("p32b_progressionseinkuenfte", "Leistungen, die dem Progressionsvorbehalt unterliegen"),
    "ArbnAnteilRenVers":      ("vor_an_anteil_rv",           "Arbeitnehmeranteil zur gesetzlichen Rentenversicherung"),
    "ArbgAnteilRenVers":      ("vor_ag_anteil_rv",           "Arbeitgeberanteil zur gesetzlichen Rentenversicherung"),
    "ArbnAnteilKrankVers":    ("basis_kv",                   "Arbeitnehmerbeiträge zur gesetzlichen Krankenversicherung (LStB Nr. 25)"),
    "ArbnAnteilPflegVers":    ("basis_pv",                   "Arbeitnehmerbeiträge zur sozialen Pflegeversicherung (LStB Nr. 26)"),
}

# Ehemals Summen-Feld KV+PV (basis_kv_pv); seit Feldsplit 2026-08 direkt als zwei 1:1-Einträge
# in LSTB. Beleg für die Zuordnung ist der Gesetzestext:
#
#   § 10 Abs. 1 Nr. 3 a)  Krankenversicherung, soweit zur Erlangung eines
#                         sozialhilfegleichen Versorgungsniveaus erforderlich
#   § 10 Abs. 1 Nr. 3 b)  gesetzliche Pflegeversicherung
#   § 10 Abs. 1 Nr. 3a    "Beiträge zu Kranken- und Pflegeversicherungen, soweit diese
#                         nicht nach Nummer 3 zu berücksichtigen sind; Beiträge zu
#                         Versicherungen gegen Arbeitslosigkeit …"
#   (sources/gesetze-im-internet/estg_p10_2026-07-11.txt)
#
# Die Arbeitslosenversicherung steht damit ausdrücklich in Nr. 3a, nicht in Nr. 3 —
# sie gehört zu weitere_vorsorgeaufwendungen, nicht zur Basisabsicherung.
LSTB_SUMMEN = {
    "weitere_vorsorgeaufwendungen": (
        ("ArbnAnteilArblVers", "Arbeitnehmerbeiträge zur Arbeitslosenversicherung"),
    ),
}

# --------------------------------------------------------------- Belegart LErsL
# Lohnersatzleistungen. Schema: VaSt_LErsL-<jahr>01.xsd
# Struktur: mehrere <Leistung> mit <Betrag> und <Art>. Die Summe geht in § 32b.

LERSL_BETRAG_FELD = "p32b_progressionseinkuenfte"

# --------------------------------------------------------------- Bewusste Lücken

NICHT_GEMAPPT = {
    "LStB/Soli": "Solidaritätszuschlag wird vom Ring aus der festgesetzten ESt berechnet "
                 "(catala_solz), nicht aus der Bescheinigung übernommen.",
    "LStB/EhegKiSteuer": "Kirchensteuer des Ehegatten gehört zur Person-B-Instanz; die "
                         "Partner-Felder werden erst mit dem Zusammenveranlagungs-Abruf "
                         "gefüllt (eigener Vorgang, eigene Berechtigung).",
    "LStB/Steuerklasse": "Die Lohnsteuerklasse steuert den Lohnsteuerabzug, nicht die "
                         "Veranlagung. Der Ring rechnet aus `veranlagung` (§ 26).",
    "LStB/BeitrPrKrankVers": "Private Krankenversicherung: § 10 Abs. 1 Nr. 3 S. 3 zählt "
                             "nur die Beitragsanteile, die den Leistungen des SGB V "
                             "entsprechen — abzüglich des Krankengeld-Anteils. Der Beleg "
                             "liefert den Gesamtbeitrag ohne diese Aufteilung; ihn voll "
                             "als Basisabsicherung zu übernehmen wäre zu hoch. Privat "
                             "Versicherte tragen den Wert weiter selbst ein.",
    "LStB/StFreiGeKrankVers": "Steuerfreie Arbeitgeberzuschüsse mindern die abziehbaren "
                              "Beiträge (§ 10 Abs. 2 S. 1 Nr. 1), sind aber kein eigener "
                              "Abzugsposten. Ob unser basis_kv_pv brutto oder netto "
                              "gemeint ist, klärt die Bindung nicht eindeutig — offen, "
                              "statt auf Verdacht zu saldieren.",
    "LStB/StFreiArbLohnDBA": "Steuerfreier Arbeitslohn nach DBA berührt § 32b und die "
                             "DBA-Methodenwahl; welches unserer Felder das trifft, hängt "
                             "am Staat und ist offen (siehe DBA_METHOD_MAP_ART).",
    "LStB/Versorgungsbezuege": "§ 19 Abs. 2 Versorgungsfreibetrag ist im Ring nicht "
                               "modelliert.",
    "KRV": "Belegart vollständig offen — die Beitragsdaten kommen als Liste mit "
           "BetragArt-Schlüsseln, deren Zuordnung zu unseren Vorsorge-Feldern eine "
           "eigene Runde braucht.",
    "RBM": "Rentenbezugsmitteilung: die Renten-Felder existieren (rentner_jahresrente "
           "u.a.), aber der Beleg unterscheidet Rentenarten feiner als unsere Bindung. "
           "Offen.",
}


# --------------------------------------------------------------- Übersetzung

def aus_lstb(werte: dict) -> list[dict]:
    """Lohnsteuerbescheinigung → Event-Liste für uebernehme_edaten().

    `werte` = {VaSt-Elementname: Text} aus dem entschlüsselten Beleg-XML.
    Nicht besetzte oder unbekannte Elemente werden übersprungen; unbekannte sind
    kein Fehler, weil das Schema mehr Felder kennt als wir mappen (siehe NICHT_GEMAPPT).
    """
    raus = []
    # LSTB_SUMMEN: mehrere Beleg-Felder → ein Zielfeld (Arbeitslosenversicherung).
    # KV/PV sind seit Feldsplit 2026-08 1:1 in LSTB (ArbnAnteilKrankVers -> basis_kv,
    # ArbnAnteilPflegVers -> basis_pv).
    for feld_id, quellen in LSTB_SUMMEN.items():
        teile = [(n, _cent(werte.get(n)), d) for n, d in quellen]
        besetzt = [(n, c, d) for n, c, d in teile if c is not None]
        if not besetzt:
            continue
        kat = "LStB: " + " + ".join(f"{d} ({n})" for n, _c, d in besetzt)
        raus.append({"feld_id": feld_id, "wert": sum(c for _n, c, _d in besetzt),
                     "kategorie": kat})
    for vast_name, (feld_id, doku) in LSTB.items():
        cent = _cent(werte.get(vast_name))
        if cent is None:
            continue
        raus.append({"feld_id": feld_id, "wert": cent,
                     "kategorie": f"LStB/{vast_name}: {doku}"})
    return raus


def aus_lersl(leistungen: list[dict]) -> list[dict]:
    """Lohnersatzleistungen → EIN Event mit der Summe (§ 32b Abs. 1 Nr. 1).

    `leistungen` = [{"Betrag": "1234.00", "Art": "…"}, …]. Der Ring führt ein
    Aggregat-Feld für alle Lohnersatzleistungen, kein Feld je Art — die Summe ist
    also die richtige Größe, nicht eine Vereinfachung.
    """
    summe = 0
    arten = []
    for l in leistungen or []:
        cent = _cent(l.get("Betrag"))
        if cent is None:
            continue
        summe += cent
        art = (l.get("Art") or "").strip()
        if art:
            arten.append(art)
    if summe == 0:
        return []
    kat = "LErsL: Lohnersatzleistungen (§ 32b Abs. 1 Nr. 1)"
    if arten:
        kat += " — " + ", ".join(sorted(set(arten)))
    return [{"feld_id": LERSL_BETRAG_FELD, "wert": summe, "kategorie": kat}]
