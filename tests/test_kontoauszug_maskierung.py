"""Gate für die PII-Maskierung im Kontoauszug-Pfad (kontoauszug_writer.maskiere).

WARUM DIESES GATE EIGEN STEHT: der maskierte Verwendungszweck verlässt das Haus. Er geht in
`uebernehme_kontoauszug` an einen EXTERNEN LLM-Provider (Zeile 452, `llm_klassifikator(maskiere(zweck), …)`)
und in die dauerhafte Speicherung (Zeile 457, `signal_1["verwendungszweck"]`). Ein Loch hier ist kein
Rechenfehler, den eine spätere Korrektur heilt — es ist ein Datenabfluss an einen Dritten, der sich nicht
zurückholen lässt (DSGVO Art. 6; ein Auftragsverarbeitungsvertrag mit dem Anbieter existiert nicht).

BEFUND, der dieses Gate ausgelöst hat (Abnahme-Audit 2026-08-19, gegen echte Proben gemessen und hier
am 2026-08-20 gegen die echte `maskiere()` nachgemessen): von den Klassen unten gingen SIEBEN unmaskiert
durch — darunter jede IBAN in Kleinschrift, was bei mehreren Instituten die normale Export-Schreibweise
ist, und die Steuernummer in JEDER ihrer üblichen Schreibweisen.

DIE TABELLE IST DER PUNKT: `MASKIERUNGS_PROBEN` wächst beim nächsten Muster-Zusatz mit, statt dass ein
neues Muster einen neuen Einzeltest bekommt und die alten Klassen ungeprüft danebenliegen. Zweite Tabelle
`DURCHLASS_PROBEN` hält die Gegenrichtung: eine Maskierung, die den ganzen Text frisst, nimmt der
Klassifikation die Grundlage und ist keine Lösung, sondern ein anderer Fehler.

KEINE ECHTEN DATEN. Alle Werte sind erfunden; die IBAN ist die öffentlich bekannte Testnummer.
Der Test ruft die ECHTE `maskiere()` — keine nachgebaute Fassung, kein LLM-Aufruf (maskiere ist reine
Textverarbeitung).
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/store", "produkt/traverser", "produkt/import"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST               # noqa: E402
import traverser as TR           # noqa: E402
import kontoauszug_writer as KW   # noqa: E402

TS = "2026-08-20T10:00:00+00:00"


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


# --------------------------------------------------------------------------------------------
# (bezeichnung, klartext, geheimnisse, kontext)
#   geheimnisse = Teilstrings, die den Text NICHT verlassen dürfen
#   kontext     = Teilstrings, die überleben MÜSSEN (sonst klassifiziert niemand mehr etwas)
#
# Schreibweisen sind nicht dekorativ: die Kleinschrift-Zeilen sind der Regelfall mehrerer
# Bank-Exporte, und die Steuernummer steht in Deutschland in drei gängigen Formen nebeneinander.
# --------------------------------------------------------------------------------------------
MASKIERUNGS_PROBEN = [
    # ---- IBAN (öffentliche Testnummer DE89 3704 0044 0532 0130 00) ----
    ("iban_gross_gruppiert",
     "Miete Januar DE89 3704 0044 0532 0130 00 Wohnung",
     ["0044", "0532", "0130"], ["Miete", "Wohnung"]),
    ("iban_klein_gruppiert",                    # Audit-Zeile 2: ging durch (_IBAN verlangte [A-Z]{2})
     "Miete Januar de89 3704 0044 0532 0130 00 Wohnung",
     ["0044", "0532", "0130"], ["Miete", "Wohnung"]),
    ("iban_gross_kompakt",
     "Zahlung an DE89370400440532013000 Malermeister",
     ["370400440532013000"], ["Malermeister"]),
    ("iban_klein_kompakt",                      # dieselbe Lücke, kompakte Schreibweise
     "Zahlung an de89370400440532013000 Malermeister",
     ["370400440532013000"], ["Malermeister"]),
    ("iban_gemischt",                           # Exporte, die nur das Länderkürzel gross setzen
     "Zahlung an De89 3704 0044 0532 0130 00 Malermeister",
     ["0044", "0532", "0130"], ["Malermeister"]),
    ("iban_ausland_mit_buchstaben",             # NL/GB führen Buchstaben IM Rumpf
     "Rechnung NL91 ABNA 0417 1643 00 Beratung",
     ["ABNA", "0417", "1643"], ["Rechnung", "Beratung"]),
    ("iban_ausland_klein",
     "Rechnung nl91 abna 0417 1643 00 Beratung",
     ["abna", "0417", "1643"], ["Rechnung", "Beratung"]),
    ("iban_bindestrich",                        # Handeingaben/Altbestände gruppieren mit "-"
     "Zweck DE89-3704-0044-0532-0130-00 Miete",
     ["0044", "0532", "0130"], ["Zweck", "Miete"]),

    # ---- Steuernummer (Landesform 11-stellig, drei Schreibweisen) ----
    ("steuernummer_slash",                      # Audit-Zeile 3: ging durch (\b\d{8,12}\b bricht am /)
     "Nachzahlung Steuernummer 151/815/08154 Finanzamt",
     ["151/815/08154", "08154"], ["Nachzahlung", "Finanzamt"]),
    ("steuernummer_leerzeichen",
     "Nachzahlung Steuernummer 151 815 08154 Finanzamt",
     ["08154"], ["Nachzahlung", "Finanzamt"]),
    ("steuernummer_kompakt",
     "Nachzahlung Steuernummer 15181508154 Finanzamt",
     ["15181508154"], ["Nachzahlung", "Finanzamt"]),

    # ---- Steuernummer (bundeseinheitlich 13-stellig) ----
    # Vom Audit NICHT benannt, hier nachgemessen: BEIDE Schreibweisen gingen durch. Die kompakte
    # Form entkam der alten Regel nicht trotz, sondern WEGEN ihrer Länge — `\b\d{8,12}\b` findet
    # in einem 13-stelligen Lauf keine Wortgrenze und lässt ihn deshalb ganz in Ruhe.
    ("steuernummer_bund_gruppiert",
     "Nachzahlung Steuernummer 3012 0815 4321 1 Finanzamt",
     ["0815", "4321"], ["Nachzahlung", "Finanzamt"]),
    ("steuernummer_bund_kompakt",
     "Nachzahlung Steuernummer 3012081543211 Finanzamt",
     ["3012081543211"], ["Nachzahlung", "Finanzamt"]),

    # ---- IdNr (11-stellig, § 139b AO) ----
    ("idnr_kompakt",
     "Kindergeld IdNr 12345678901 Erstattung",
     ["12345678901"], ["Kindergeld", "Erstattung"]),
    ("idnr_gruppiert",                          # ebenfalls vom Audit nicht benannt, ging durch
     "Kindergeld IdNr 12 345 678 901 Erstattung",
     ["345", "678", "901"], ["Kindergeld", "Erstattung"]),

    # ---- Kontonummer (die einzige Klasse, die vorher schon hielt — Regressions-Anker) ----
    ("kontonummer",
     "Lastschrift Konto 1234567890 Beitrag",
     ["1234567890"], ["Lastschrift", "Beitrag"]),
]


# --------------------------------------------------------------------------------------------
# Gegenrichtung: was die Maskierung in Ruhe lassen MUSS. Ohne diese Tabelle ist das Gate trivial
# erfüllbar — `return "****"` bestünde jede Zeile oben und wäre trotzdem Unsinn, weil danach weder
# die Keyword-Heuristik noch das LLM eine Kategorie findet und JEDE Buchung als unklar durchfällt.
# --------------------------------------------------------------------------------------------
DURCHLASS_PROBEN = [
    ("klassifikations_keywords", "Malermeister Schmidt Renovierung Bad",
     ["Malermeister", "Renovierung"]),
    ("spende", "Spende Rotes Kreuz Jahresbeitrag", ["Spende", "Rotes Kreuz"]),
    ("betrag_im_zweck", "Handwerkerleistung Rechnung ueber 1.234,56 EUR", ["1.234,56", "EUR"]),
    ("kurze_zahlen", "Rechnung 4711 Position 12 Heizung", ["4711", "Heizung"]),
    ("datum_punktform", "Reinigung 19.08.2026 Treppenhaus", ["19.08.2026", "Reinigung"]),
]


@pytest.mark.parametrize("bezeichnung,klartext,geheimnisse,kontext", MASKIERUNGS_PROBEN,
                         ids=[p[0] for p in MASKIERUNGS_PROBEN])
def test_maskierung_haelt_geheimnis_zurueck(bezeichnung, klartext, geheimnisse, kontext):
    """Kein Geheimnis überlebt — und genug Kontext überlebt, damit die Zeile klassifizierbar bleibt."""
    maskiert = KW.maskiere(klartext)
    for geheim in geheimnisse:
        assert geheim not in maskiert, (
            f"[{bezeichnung}] '{geheim}' steht unmaskiert im Text, der an den externen "
            f"LLM-Provider und in die Speicherung geht: {maskiert!r}")
    for k in kontext:
        assert k in maskiert, (
            f"[{bezeichnung}] '{k}' wurde mitmaskiert — die Zeile ist danach nicht mehr "
            f"klassifizierbar: {maskiert!r}")


@pytest.mark.parametrize("bezeichnung,klartext,kontext", DURCHLASS_PROBEN,
                         ids=[p[0] for p in DURCHLASS_PROBEN])
def test_maskierung_frisst_den_zweck_nicht(bezeichnung, klartext, kontext):
    """Übermaskierung ist kein sicherer Zustand, sondern ein anderer Fehler."""
    maskiert = KW.maskiere(klartext)
    for k in kontext:
        assert k in maskiert, f"[{bezeichnung}] '{k}' verloren: {maskiert!r}"


def test_leerer_und_fehlender_text():
    """Randwerte: kein Absturz, keine Ausgabe aus dem Nichts."""
    assert KW.maskiere("") == ""
    assert KW.maskiere(None) is None


def test_mehrere_geheimnisse_in_einer_zeile():
    """Eine echte Zeile trägt oft beides. Die Muster dürfen sich nicht gegenseitig aufheben —
    die IBAN muss VOR der Steuernummer greifen, sonst frisst die Ziffernregel den IBAN-Rumpf
    und hinterlässt einen Rest, der wie ein unverdächtiges Fragment aussieht."""
    maskiert = KW.maskiere("Ueberweisung DE89 3704 0044 0532 0130 00 Steuernummer 151/815/08154 Rest")
    assert "0532" not in maskiert and "0130" not in maskiert
    assert "08154" not in maskiert
    assert "Ueberweisung" in maskiert and "Rest" in maskiert


# --------------------------------------------------------------------------------------------
# DIE NAHT. Alles oben prüft `maskiere()` für sich. Genau das ist die bekannte Blindstelle dieses
# Repos ([[naht-blindstelle-zwei-repraesentationen]]): zwei Repräsentationen, ungetestete Übergabe.
# Hörte eine der beiden Aufrufstellen in uebernehme_kontoauszug auf, `maskiere()` zu rufen, bliebe
# jede Zeile oben grün — und der ROHE Zweck ginge hinaus. Deshalb wird hier gemessen, was die
# Aufrufstelle WIRKLICH weitergibt, nicht was die Funktion könnte.
# --------------------------------------------------------------------------------------------

# Unklassifizierbar per Keyword (erzwingt den LLM-Zweig) und trägt beide Geheimnis-Klassen.
_SCHMUTZIGER_ZWECK = "KRYPTISCH-REF DE89 3704 0044 0532 0130 00 Steuernummer 151/815/08154"


def test_naht_llm_bekommt_nur_den_maskierten_zweck(bindung):
    """Was den Rechner verlässt, ist maskiert — geprüft am Argument, das der Klassifikator sieht.

    Der Stub ist reines Python und simuliert KEINEN LLM-Call; er hält nur fest, was an der
    Injektionsstelle ankommt (Präzedenz test_kontoauszug_writer.test_llm_fallback_injiziert)."""
    gesehen = []

    def stub(zweck, betrag):
        gesehen.append(zweck)
        return "spende"

    s = ST.leerer_store(2025, fall_id="ka-maske-llm")
    tx = [{"datum": "12.03.2026", "betrag": -7500, "verwendungszweck": _SCHMUTZIGER_ZWECK}]
    KW.uebernehme_kontoauszug(s, tx, bindung, llm_klassifikator=stub, ts=TS)

    assert len(gesehen) == 1, "der LLM-Zweig wurde gar nicht betreten — Probe nicht mehr mehrdeutig"
    assert "0532" not in gesehen[0] and "0130" not in gesehen[0], (
        f"die IBAN geht unmaskiert an den externen Provider: {gesehen[0]!r}")
    assert "08154" not in gesehen[0], (
        f"die Steuernummer geht unmaskiert an den externen Provider: {gesehen[0]!r}")
    assert "KRYPTISCH-REF" in gesehen[0], "der Zweck ist unklassifizierbar zerstört"


def test_naht_speicherung_haelt_nur_den_maskierten_zweck(bindung):
    """Was liegen bleibt, ist maskiert — geprüft am Event im Store, nicht am Rückgabewert.

    Zweite, eigene Naht: die Speicherung ruft `maskiere()` an einer ANDEREN Stelle als der
    LLM-Versand. Eine der beiden zu heilen und die andere zu vergessen ist der Normalfall des
    Fehlers, nicht die Ausnahme."""
    s = ST.leerer_store(2025, fall_id="ka-maske-store")
    tx = [{"datum": "12.03.2026", "betrag": -7500, "verwendungszweck": _SCHMUTZIGER_ZWECK}]
    n, _ = KW.uebernehme_kontoauszug(s, tx, bindung,
                                     llm_klassifikator=lambda z, b: "spende", ts=TS)
    assert n == 1

    gespeichert = ST._aktives(s)["spenden_betrag"]["signal"]["signal_1"]["verwendungszweck"]
    assert "0532" not in gespeichert and "0130" not in gespeichert, (
        f"die IBAN liegt unmaskiert im Store: {gespeichert!r}")
    assert "08154" not in gespeichert, f"die Steuernummer liegt unmaskiert im Store: {gespeichert!r}"


# --------------------------------------------------------------------------------------------
# DOKUMENTIERTE GRENZE — kein Versehen, eine Entscheidung.
# --------------------------------------------------------------------------------------------

def test_grenze_personennamen_werden_nicht_erkannt():
    """PERSONENNAMEN IM VERWENDUNGSZWECK WERDEN NICHT MASKIERT. Das ist die bewusst offene Flanke.

    Ein Namensmuster gibt es nicht. Jede Heuristik, die "Dr. Anna Musterfrau" fängt, lässt
    "Musterfrau, Anna", "ANNA MUSTERFRAU" und "A. Musterfrau" durch — und trifft zugleich
    Steuerbegriffe ("Rürup", "Riester") und Firmennamen, die die Klassifikation braucht. Sie
    würde also gleichzeitig zu wenig schützen und zu viel zerstören, und dabei aussehen, als
    wäre das Problem gelöst. Genau diese Vortäuschung ist teurer als die offene Flanke, weil
    sie die Frage von der Tagesordnung nimmt.

    Der Verwendungszweck einer Bankbuchung TRÄGT den Namen des Zahlungsempfängers — das ist
    kein Randfall, das ist sein Inhalt. Diese Funktion reduziert den Abfluss, sie beendet ihn
    nicht. Die belastbare Abhilfe liegt nicht in einem besseren Regex, sondern eine Ebene
    höher: Auftragsverarbeitungsvertrag mit dem Anbieter, oder den Verwendungszweck gar nicht
    erst hinausgeben (lokales Modell / rein deterministische Klassifikation).

    WIRD DIESER TEST ROT, weil jemand doch eine Namenserkennung gebaut hat: das ist erlaubt.
    Dann diesen Test durch echte Proben ersetzen (Vor- und Nachname, umgestellt, Grossschrift,
    abgekürzt) und die Gegenprobe mitliefern, dass Firmennamen und Steuerbegriffe stehen
    bleiben. Nicht einfach die Zeile anpassen."""
    maskiert = KW.maskiere("Ueberweisung Dr. Anna Musterfrau Honorar")
    assert "Musterfrau" in maskiert, (
        "Es gibt jetzt offenbar eine Namenserkennung — siehe Docstring, dieser Test will "
        "durch echte Proben ersetzt werden, nicht angepasst.")
