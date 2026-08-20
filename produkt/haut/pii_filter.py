"""PII-Filter vor ausgehendem LLM-Call.

Filtert personenbezogene Daten aus Freitext, bevor er an einen externen
LLM-Provider geht. Gibt (gefilterter_text, getroffene_kategorien) zurück.

HINWEIS: FREIE PERSONENNAMEN werden bewusst NICHT erkannt. Ein Regex auf
Eigennamen trifft Steuerbegriffe ("Riester", "Rürup", "Ehegatte") und
zerstört den Freitext, aus dem das LLM Werte extrahieren soll. Die enge
Regel "Herr/Frau + folgendes Wort" ist der max. vertretbare Eingriff.
Geldbeträge und Paragraphen-Nennungen bleiben unangetastet — sonst kann
das LLM keine Cent-Werte mehr vorschlagen.

Reihenfolge: IBAN vor steuer_id prüfen (sonst frisst die 11-Ziffern-Regel
Teile der IBAN), steuer_id vor kontonummer (sonst meldet die unspezifische
Ziffernregel die Kategorie, und das Audit verliert die Aussage).
"""

from __future__ import annotations

import re

# ------------------------------------------------------------ Kennungen (Audit pii-filter-chat-pfad)
# GEMESSEN 2026-08-20 gegen die Vorfassung (`[A-Z]{2}\d{2}(?:[ ]?\d){10,30}` / `\b\d(?:[ /]?\d){10}\b`
# und GAR KEINE Ziffernlauf-Regel): von 17 Proben gingen ZEHN unmaskiert an den LLM-Provider.
# Dieselben drei Bauart-Fehler, die am selben Tag im Kontoauszug-Pfad geschlossen wurden — plus
# einer, den dieser Pfad allein hat:
#
#   [A-Z]{2}          jede IBAN in KLEINSCHRIFT ging durch, ebenso "De89" (nur Kürzel gross).
#                     Bei mehreren Instituten ist Kleinschrift die normale Export-Schreibweise,
#                     und wer tippt, tippt sowieso klein.
#   \d im Rumpf       ausländische IBAN mit Buchstaben im Rumpf (NL91 ABNA …) ging durch.
#   [ ]? als Trenner  die Bindestrich-Gruppierung (DE89-3704-…) ging durch.
#   {10} fest         die 13-stellige bundeseinheitliche Steuernummer ging durch — in BEIDEN
#                     Schreibweisen. Die kompakte entkam nicht trotz, sondern WEGEN ihrer Länge.
#   (fehlte ganz)     Kontonummern und 16-stellige Kartennummern: dieser Pfad hatte nie eine
#                     Regel dafür. Der Kontoauszug-Pfad hat sie seit jeher (_KTO).
#
# Deshalb trägt _KONTONUMMER `{8,}` OHNE Obergrenze: eine Obergrenze in einem Sicherheitsmuster
# ist eine Einladung, sie zu überschreiten.
#
# PARALLELE FASSUNG: produkt/import/kontoauszug_writer.py führt dieselben Klassen für den
# Kontoauszug-Pfad. NICHT zusammengelegt und bewusst nicht: dort bleibt ein Präfix stehen
# (`DE89****`), damit die Klassifikation noch etwas zu greifen hat; hier wird vollständig durch
# `[PII]` ersetzt. Gleiche Bauart, verschiedene Ersetzungsregel.

# IBAN: Länderkürzel + Prüfziffern, dann Rumpf zusammengeschrieben ODER in Vierergruppen.
# Die Vierergruppen-Alternative endet an der ersten Gruppe, die keine vier Zeichen hat — der
# umgebende Satz bleibt lesbar, was hier zählt: das Beleg-Gate in api_llm prüft die Zitate des
# Modells gegen genau diesen gefilterten Text.
_IBAN = re.compile(
    r"\b[A-Za-z]{2}\d{2}"
    r"(?:[A-Za-z0-9]{10,30}"                                   # DE89370400440532013000
    r"|(?:[ -][A-Za-z0-9]{4}){2,7}(?:[ -][A-Za-z0-9]{1,4})?)"  # DE89 3704 0044 0532 0130 00
    r"\b")                                                     # Bindestrich mit, Handeingaben führen ihn

# Steuernummer (Landesform 11-stellig, bundeseinheitlich 13-stellig) und IdNr (11-stellig,
# § 139b AO) in ihren getrennten Schreibweisen. `{10,12}` = 11 bis 13 Ziffern insgesamt.
# IBAN läuft VORHER (steht zuerst in _KATEGORIEN) — kein Konflikt mit DE...-IBAN.
_STEUER_ID = re.compile(r"\b\d(?:[ /]?\d){10,12}\b")

# Kontonummern, Kartennummern, sonstige lange Ziffernläufe. Ohne Obergrenze, s.o.
# Läuft NACH _STEUER_ID, fängt also nur, was die spezifischere Regel übrig lässt.
# 8 als Untergrenze ist die Trennlinie zum Geldbetrag: Beträge im Freitext tragen Punkt oder
# Komma (`1.234,56`), und ein trennzeichenloser 8-stelliger Betrag wäre zweistellig in Millionen.
_KONTONUMMER = re.compile(r"\b\d{8,}\b")

# Datum TT.MM.JJJJ
_DATUM = re.compile(r"\b(0[1-9]|[12]\d|3[01])\.(0[1-9]|1[0-2])\.\d{4}\b")

# PLZ + großgeschriebenes Wort: 5 Ziffern + Leerzeichen + Großbuchstabe.
# Negative Lookahead für Wörter, die nach 5 Ziffern KEIN Ort sind:
# Währungen (Euro, EUR, €), Einheiten (Euro, Kilometer, Meter, Stück, …),
# und häufige Steuerkontext-Wörter.
# Der Set ist klein und endlich — im Steuer-Freitext folgt auf 5 Ziffern
# fast immer "Euro" oder eine Einheit, kein echter Ortsname.
_PLZ_ORT = re.compile(
    r"\b\d{5}\s+"
    r"(?!(?:Euro|EUR|€|Kilometer|km|Meter|m|Stück|Tonnen|kg|g|Liter|"
    r"Jahre|Tage|Stunden|Mitglieder|Mitarbeiter|Einwohner|Jahr|Tag|Stunde)\b)"
    r"[A-ZÄÖÜ][a-zäöüß]+\b"
)

# Straße + Hausnummer: Wort auf -straße/-str./-weg/-allee/-platz + HAUSNUMMER PFLICHT.
# Ohne Hausnummer erzeugen "Arbeitsplatz", "Ehering", "Studienplatz" Fehlalarme.
# -straße/-strasse/-str. bleiben ohne Hausnummer erlaubt (sehr spezifisch, kaum
# Fehlalarme im Steuerkontext — "Arbeitsstraße" gibt es praktisch nicht).
_STRASSE = re.compile(
    r"\b[A-ZÄÖÜ][a-zäöüß]+(?:straße|strasse|str\.)"
    r"(?:\s+\d{1,4}[a-z]?)?"
    r"|"
    r"\b[A-ZÄÖÜ][a-zäöüß]+(?:weg|allee|platz|gasse|damm|ring|chaussee)"
    r"(?:\s+\d{1,4}[a-z]?)"
)

# Anrede (enge Regel): "Herr" oder "Frau" + folgendes Wort
_ANREDE = re.compile(r"\b(?:Herr|Frau)\s+[A-ZÄÖÜ][a-zäöüß]+\b")

_PLATZHALTER = "[PII]"

# --------------------------------------------------- Art. 9 DSGVO: besondere Kategorien
#
# Der Filter oben maskiert KENNUNGEN (IBAN, Steuer-ID, Datum, Anschrift, Anrede+Name). Er
# maskiert keine MERKMALE — und kann es nicht: „80" ist als Zeichenfolge nicht von einem
# Betrag zu unterscheiden. Ob eine Angabe eine Behinderung offenbart, weiss allein das FELD,
# aus dem sie stammt. Deshalb steht die Sperre hier auf der Feld-Ebene, nicht als Textregel.
#
# GEMESSEN 2026-08-18 (Audit gdpr-art9-und-drittdaten-an-llm, nachgerechnet): der Chat-Kontext
# trug „Grad der Behinderung → 80", „hilflos, blind oder taubblind → ja", „Pflegegrad → 3"
# unverändert nach draussen; filtere() gab die Probe BYTE-IDENTISCH zurück und meldete keine
# Kategorie. Bei der gepflegten Person wurden Anschrift und Geburtsdatum zu [PII] — ihr NAME
# blieb stehen (die Anrede-Regel greift nur bei „Herr/Frau“). Name plus Pflegegrad plus
# Verwandtschaftsverhältnis einer Person, die nie mit dem System zu tun hatte.
#
# 21 von 307 Bindungsfeldern fallen darunter, 13 davon betreffen DRITTE (Partner, Kind,
# gepflegte Person). Art. 9 Abs. 1 DSGVO nennt religiöse Überzeugung und Gesundheitsdaten
# ausdrücklich; ein Auftragsverarbeitungsvertrag mit dem LLM-Anbieter existiert nicht.
#
# WAS HIER NICHT DRINSTEHT und bewusst nicht: reine Beträge, die aus einem solchen Sachverhalt
# FOLGEN, ihn aber nicht nennen (`kist_gezahlt`, `basis_kv`). Sie zu sperren würde den Chat
# ohne Gewinn verstümmeln — aus „Kirchensteuer gezahlt: 412 €" folgt keine Konfession, und die
# Zahl steht ohnehin auf jeder Lohnsteuerbescheinigung.
#
# Die Liste ist ein REGEX über feld_id UND Fragetext, nicht eine Aufzählung: eine Aufzählung
# vergisst das nächste Feld. tests/test_art9_nicht_an_llm.py hält beides zusammen und wird rot,
# sobald ein neues Bindungsfeld ein solches Merkmal trägt, ohne dass jemand hier hingesehen hat.
_ART9 = re.compile(
    r"konfession|kirche"                                    # religiöse Überzeugung
    r"|grad_der_behinderung|schwerbehind|gehbehind|behinderungsbedingte"
    r"|behinderten_pb|behinderten.?pausch"                  # „Behinderten-Pauschbetrag" im Text
    r"|pflegegrad|pflegebeduerftig|gepflegter"              # Gesundheit
    r"|pflegst|pflege_durch|pflegt die person"              # wer pflegt WEN
    r"|hilflos|blind|taubblind|merkzeichen"
    r"|berufsunfaehig|erwerbsunfaehig|krankheitskosten|heilbehandlung",
    re.I)


def ist_besondere_kategorie(feld_id: str, fragetext: str = "") -> bool:
    """Offenbart dieses FELD eine besondere Kategorie nach Art. 9 DSGVO?

    Geprüft wird die feld_id UND der Fragetext: ein Feld kann unverdächtig heissen und in der
    Frage nach dem Merkmal fragen (`rentner_gepflegter_angaben` → „Wer ist die Person, die du
    pflegst?“), und umgekehrt.

    Der Aufrufer entscheidet, was er damit tut — hier wird nichts maskiert. Für den
    LLM-Kontext heisst es: der WERT bleibt daheim. Die blosse Frage darf hinaus, sie steht in
    jedem amtlichen Vordruck und ist ohne Antwort kein personenbezogenes Datum."""
    return bool(_ART9.search(feld_id or "") or _ART9.search(fragetext or ""))

_KATEGORIEN: list[tuple[str, re.Pattern]] = [
    ("iban", _IBAN),
    ("steuer_id", _STEUER_ID),
    ("kontonummer", _KONTONUMMER),
    ("datum", _DATUM),
    ("plz_ort", _PLZ_ORT),
    ("strasse", _STRASSE),
    ("anrede_name", _ANREDE),
]


def filtere(text: str) -> tuple[str, list[str]]:
    """Ersetzt PII im Text durch Platzhalter.

    Args:
        text: Roher Freitext (Nutzereingabe).

    Returns:
        (gefilterter Text, sortierte Liste der getroffenen Kategorien).
    """
    if not text:
        return text, []

    getroffen: list[str] = []
    for kategorie, pattern in _KATEGORIEN:
        neuer_text, n = pattern.subn(_PLATZHALTER, text)
        if n > 0:
            getroffen.append(kategorie)
            text = neuer_text

    return text, sorted(getroffen)