"""ELSTER-Zeichensatz fuer Textfelder (Ticket elster-zeichensatz-strenger-als-xml, Decision
elster-zeichensatz-beim-speichern-abweisen).

ELSTER erlaubt in Textfeldern nur den Zeichensatz „Standard_E_V2" — viel enger als XML. Was beim Kopieren
aus Webseiten und Word entsteht (geschuetztes Leerzeichen, Gedankenstrich, typografische Anfuehrungszeichen,
Tabulator, „ł" in polnischen Namen) lehnt die Schemapruefung ab, erst beim Absenden. Hier steht die Regel
EINMAL: `store._pruefe_typ_konformitaet` weist den Wert beim Speichern ab, `elster_xml.erzeuge_xml` sperrt
Altbestaende und Importe ein zweites Mal. Die Meldung nennt das Zeichen und einen Vorschlag — nie den Wert
(PII). Das Rust-Gegenstueck ist `domain::zeichensatz`; Zeichenmenge, Tabelle und Texte sind wortgleich.
"""
from __future__ import annotations

import re

# E10-2025.xsd:1810 `StringZUBaseCType`, Muster „Zeichensatz Standard_E_V2", WOERTLICH:
#   [&#xa;&#xd;&#x20;-&#x7e;&#xa1;-&#xa3;&#xa5;&#xa7;&#xaa;-&#xac;&#xae;-&#xb3;&#xb5;&#xb9;-&#xbb;&#xbf;-&#xff;
#    &#x152;-&#x153;&#x160;-&#x161;&#x178;&#x17d;-&#x17e;&#x20ac;]+
# E10-2025.xsd:1792 `StringBaseCType` (setzt darauf auf, verbietet Zeilenumbrueche): [^&#xa;&#xd;]+ -- also
# ohne &#xa; und &#xd;. Alle Textfelder der Bindung enden auf StringBaseCType oder auf engeren Mustern
# (Datumsbereich, IdNr, BIC): der Store prueft deshalb jedes `typ: text` gegen diese Menge.
# Beleg gegen das Schema: tests/test_store_zeichensatz.py (jedes Zeichen 0..0x2FFFF).
_NICHT_ERLAUBT = re.compile(
    r"[^\x20-\x7e\xa1-\xa3\xa5\xa7\xaa-\xac\xae-\xb3\xb5\xb9-\xbb\xbf-\xff"
    r"Œ-œŠ-šŸŽ-ž€]")

# Vorschlaege: (Ersatz, Zeichen...) -- das erste Treffer-Paar gilt. Leerer Ersatz heisst „loeschen",
# ein einzelnes Leerzeichen „normales Leerzeichen". Was hier fehlt, bekommt den allgemeinen Rat.
# ponytail: Latin-1/Latin Extended-A und die Satzzeichen, die beim Kopieren vorkommen; Kyrillisch, Griechisch,
# Vietnamesisch u. a. bekommen den allgemeinen Rat. Wer mehr Vorschlaege will, haengt Zeilen an (und in
# rust/domain/src/zeichensatz.rs dieselben).
_VORSCHLAEGE = (
    (" ", "\t\n\r\x0b\x0c\x85\xa0            "
          "    　"),
    ("", "\xad​‌‍⁠﻿"),
    ("-", "‐‑‒–—―−"),
    ("'", "\xb4‘’‚‛′"),
    ('"', "“”„‟″"),
    ("...", "…"),
    ("(c)", "\xa9"),
    ("1/4", "\xbc"),
    ("1/2", "\xbd"),
    ("3/4", "\xbe"),
    ("(TM)", "™"),
    ("A", "ĀĂĄ"), ("a", "āăą"),
    ("C", "ĆĈĊČ"), ("c", "ćĉċč"),
    ("D", "ĎĐ"), ("d", "ďđ"),
    ("E", "ĒĔĖĘĚ"), ("e", "ēĕėęě"),
    ("G", "ĜĞĠĢ"), ("g", "ĝğġģ"),
    ("H", "ĤĦ"), ("h", "ĥħ"),
    ("I", "ĨĪĬĮİ"), ("i", "ĩīĭįı"),
    ("IJ", "Ĳ"), ("ij", "ĳ"),
    ("J", "Ĵ"), ("j", "ĵ"),
    ("K", "Ķ"), ("k", "ķĸ"),
    ("L", "ĹĻĽĿŁ"), ("l", "ĺļľŀł"),
    ("N", "ŃŅŇŊ"), ("n", "ńņňŉŋ"),
    ("O", "ŌŎŐ"), ("o", "ōŏő"),
    ("R", "ŔŖŘ"), ("r", "ŕŗř"),
    ("S", "ŚŜŞȘ"), ("s", "śŝşſș"),
    ("T", "ŢŤŦȚ"), ("t", "ţťŧț"),
    ("U", "ŨŪŬŮŰŲ"), ("u", "ũūŭůűų"),
    ("W", "Ŵ"), ("w", "ŵ"),
    ("Y", "Ŷ"), ("y", "ŷ"),
    ("Z", "ŹŻ"), ("z", "źż"),
)

# Zeichen, die man nicht lesen kann: sie bekommen einen Namen (Reihenfolge der Meldung: Name, dann Codepunkt).
_NAMEN = {
    "\t": "Tabulator",
    "\n": "Zeilenumbruch",
    "\r": "Zeilenumbruch",
    "\xa0": "geschütztes Leerzeichen",
    "\xad": "bedingter Trennstrich",
    "​": "Leerzeichen der Breite null",
    "﻿": "Byte-Order-Mark",
}

# Dasselbe fuer ganze Bereiche (Codepunkt von, bis, Name): ein Steuerzeichen oder ein Leerzeichen anderer Breite
# stuende sonst als unsichtbares Zeichen zwischen den Anfuehrungszeichen der Meldung. _NAMEN geht vor.
_NAMEN_BEREICHE = (
    (0x00, 0x1F, "Steuerzeichen"),
    (0x7F, 0x9F, "Steuerzeichen"),
    (0x1680, 0x1680, "Leerzeichen besonderer Breite"),
    (0x2000, 0x200A, "Leerzeichen besonderer Breite"),
    (0x200C, 0x200F, "unsichtbares Zeichen"),
    (0x2028, 0x202E, "unsichtbares Zeichen"),
    (0x202F, 0x202F, "Leerzeichen besonderer Breite"),
    (0x205F, 0x205F, "Leerzeichen besonderer Breite"),
    (0x2060, 0x206F, "unsichtbares Zeichen"),
    (0x3000, 0x3000, "Leerzeichen besonderer Breite"),
)

_ALLGEMEINER_RAT = ("ersetze es durch ein Zeichen, das ELSTER annimmt (lateinische Buchstaben mit Umlauten "
                    "und ß, Ziffern, übliche Satzzeichen).")

# Kombinierende Akzente (U+0300..U+036F): macOS und manche PDFs liefern „ü" als „u" plus Akzent. Loeschen
# machte aus „Müller" ein „Muller", darum ein eigener Rat.
_KOMBINIEREND = ("̀", "ͯ")
_KOMBINIEREND_RAT = ("tippe den Buchstaben mit Akzent neu, als ein Zeichen (zum Beispiel „ü\" statt „u\" "
                     "und einem losen Akzent).")


def erstes_unerlaubtes_zeichen(text: str) -> str | None:
    """Das erste Zeichen von `text` ausserhalb des ELSTER-Zeichensatzes, sonst None. Ein leerer Text hat
    keins (die Leer-Pruefung steht an anderer Stelle: `store._typ_konform`)."""
    treffer = _NICHT_ERLAUBT.search(text)
    return treffer.group(0) if treffer else None


def zeichen_anzeige(zeichen: str) -> str:
    """Wie die Meldung das Zeichen nennt: `„ł" (U+0142)`, ein unlesbares mit Namen: `Tabulator (U+0009)`."""
    punkt = f"U+{ord(zeichen):04X}"
    if _KOMBINIEREND[0] <= zeichen <= _KOMBINIEREND[1]:
        return f"loser Akzent ({punkt})"
    name = _NAMEN.get(zeichen) or next((n for von, bis, n in _NAMEN_BEREICHE if von <= ord(zeichen) <= bis), None)
    return f"{name} ({punkt})" if name else f"„{zeichen}\" ({punkt})"


def zeichen_rat(zeichen: str) -> str:
    """Der Vorschlag zu einem unerlaubten Zeichen, als Satzende der Meldung."""
    if _KOMBINIEREND[0] <= zeichen <= _KOMBINIEREND[1]:
        return _KOMBINIEREND_RAT
    for ersatz, zeichen_liste in _VORSCHLAEGE:
        if zeichen in zeichen_liste:
            if ersatz == "":
                return "lösche es."
            if ersatz == " ":
                return "schreibe stattdessen ein normales Leerzeichen."
            return f"schreibe stattdessen „{ersatz}\"."
    return _ALLGEMEINER_RAT


def feld_meldung(feld_id: str, zeichen: str) -> str:
    """Die Abweisung beim Speichern. Nennt Feld, Zeichen und Vorschlag, nie den Wert (PII). Rust:
    `Abweisung::ZeichensatzVerletzt`, wortgleich."""
    return (f"fail-closed (Zeichensatz): {feld_id} enthält das Zeichen {zeichen_anzeige(zeichen)}, "
            f"das ELSTER in Textfeldern nicht annimmt — {zeichen_rat(zeichen)}")


def element_meldung(element: str, text: str) -> str | None:
    """Die Sperre an der XML-Erzeugung (Altbestaende, Importe): die Meldung zu einem Element, dessen Text ein
    unerlaubtes Zeichen traegt, sonst None. Nennt das Element, nie den Wert. Rust:
    `domain::zeichensatz::element_meldung`, wortgleich."""
    zeichen = erstes_unerlaubtes_zeichen(text)
    if zeichen is None:
        return None
    return (f"Element {element} enthält das Zeichen {zeichen_anzeige(zeichen)}, das ELSTER in Textfeldern "
            f"nicht annimmt — ELSTER wiese die ganze Abgabe ab. Wert nicht geloggt. "
            f"Rat: {zeichen_rat(zeichen)}")
