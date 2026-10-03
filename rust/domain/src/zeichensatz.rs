//! ELSTER-Zeichensatz fuer Textfelder (Ticket `elster-zeichensatz-strenger-als-xml`, Vault
//! `decisions/elster-zeichensatz-beim-speichern-abweisen`).
//!
//! ELSTER erlaubt in Textfeldern nur den Zeichensatz „`Standard_E_V2`" — viel enger als XML. Was
//! beim Kopieren aus Webseiten und Word entsteht (geschuetztes Leerzeichen, Gedankenstrich,
//! typografische Anfuehrungszeichen, Tabulator, „ł" in polnischen Namen), lehnt die Schemapruefung
//! ab, erst beim Absenden. Die Regel steht hier EINMAL: `store` weist den Wert beim Speichern ab
//! (`Abweisung::ZeichensatzVerletzt`), `elster::erzeuge_xml` sperrt Altbestaende und Importe ein
//! zweites Mal. Die Meldung nennt das Zeichen und einen Vorschlag — nie den Wert (PII).
//! Das Python-Gegenstueck ist `produkt/store/zeichensatz.py`; Zeichenmenge, Tabelle und Texte sind
//! wortgleich (Paritaetstest `store_zeichensatz_paritaet`).

/// Ist `c` im Zeichensatz eines ELSTER-Textfelds?
///
/// `E10-2025.xsd:1810` `StringZUBaseCType`, Muster „Zeichensatz `Standard_E_V2`", WOERTLICH:
/// `[&#xa;&#xd;&#x20;-&#x7e;&#xa1;-&#xa3;&#xa5;&#xa7;&#xaa;-&#xac;&#xae;-&#xb3;&#xb5;&#xb9;-&#xbb;
/// &#xbf;-&#xff;&#x152;-&#x153;&#x160;-&#x161;&#x178;&#x17d;-&#x17e;&#x20ac;]+`, dazu `:1792`
/// `StringBaseCType` `[^&#xa;&#xd;]+` — also ohne `&#xa;` und `&#xd;`. Alle Textfelder der Bindung
/// enden auf `StringBaseCType` oder auf engeren Mustern (Datumsbereich, `IdNr`, BIC).
///
/// ```
/// assert!(domain::zeichensatz::elster_zeichen('ü') && domain::zeichensatz::elster_zeichen('ß'));
/// assert!(domain::zeichensatz::elster_zeichen('€'));
/// assert!(!domain::zeichensatz::elster_zeichen('ł') && !domain::zeichensatz::elster_zeichen('–'));
/// assert!(!domain::zeichensatz::elster_zeichen('\t') && !domain::zeichensatz::elster_zeichen('\u{a0}'));
/// ```
#[must_use]
pub fn elster_zeichen(c: char) -> bool {
    matches!(
        c,
        '\u{20}'..='\u{7e}'
            | '\u{a1}'..='\u{a3}'
            | '\u{a5}'
            | '\u{a7}'
            | '\u{aa}'..='\u{ac}'
            | '\u{ae}'..='\u{b3}'
            | '\u{b5}'
            | '\u{b9}'..='\u{bb}'
            | '\u{bf}'..='\u{ff}'
            | '\u{152}'..='\u{153}'
            | '\u{160}'..='\u{161}'
            | '\u{178}'
            | '\u{17d}'..='\u{17e}'
            | '\u{20ac}'
    )
}

/// Das erste Zeichen von `text` ausserhalb des ELSTER-Zeichensatzes. Ein leerer Text hat keins
/// (die Leer-Pruefung steht an anderer Stelle).
#[must_use]
pub fn erstes_unerlaubtes_zeichen(text: &str) -> Option<char> {
    text.chars().find(|c| !elster_zeichen(*c))
}

/// Vorschlaege: (Ersatz, Zeichen…), das erste Treffer-Paar gilt. Leerer Ersatz heisst „loeschen",
/// ein einzelnes Leerzeichen „normales Leerzeichen". Was hier fehlt, bekommt den allgemeinen Rat.
/// Wortgleich mit `zeichensatz.py::_VORSCHLAEGE`.
// ponytail: Latin-1/Latin Extended-A und die Satzzeichen, die beim Kopieren vorkommen; Kyrillisch,
// Griechisch, Vietnamesisch u. a. bekommen den allgemeinen Rat. Wer mehr will, haengt Zeilen an (und
// in produkt/store/zeichensatz.py dieselben).
const VORSCHLAEGE: &[(&str, &str)] = &[
    (
        " ",
        "\t\n\r\u{b}\u{c}\u{85}\u{a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}",
    ),
    ("", "\u{ad}\u{200b}\u{200c}\u{200d}\u{2060}\u{feff}"),
    ("-", "\u{2010}\u{2011}\u{2012}\u{2013}\u{2014}\u{2015}\u{2212}"),
    ("'", "\u{b4}\u{2018}\u{2019}\u{201a}\u{201b}\u{2032}"),
    ("\"", "\u{201c}\u{201d}\u{201e}\u{201f}\u{2033}"),
    ("...", "\u{2026}"),
    ("(c)", "\u{a9}"),
    ("1/4", "\u{bc}"),
    ("1/2", "\u{bd}"),
    ("3/4", "\u{be}"),
    ("(TM)", "\u{2122}"),
    ("A", "\u{100}\u{102}\u{104}"),
    ("a", "\u{101}\u{103}\u{105}"),
    ("C", "\u{106}\u{108}\u{10a}\u{10c}"),
    ("c", "\u{107}\u{109}\u{10b}\u{10d}"),
    ("D", "\u{10e}\u{110}"),
    ("d", "\u{10f}\u{111}"),
    ("E", "\u{112}\u{114}\u{116}\u{118}\u{11a}"),
    ("e", "\u{113}\u{115}\u{117}\u{119}\u{11b}"),
    ("G", "\u{11c}\u{11e}\u{120}\u{122}"),
    ("g", "\u{11d}\u{11f}\u{121}\u{123}"),
    ("H", "\u{124}\u{126}"),
    ("h", "\u{125}\u{127}"),
    ("I", "\u{128}\u{12a}\u{12c}\u{12e}\u{130}"),
    ("i", "\u{129}\u{12b}\u{12d}\u{12f}\u{131}"),
    ("IJ", "\u{132}"),
    ("ij", "\u{133}"),
    ("J", "\u{134}"),
    ("j", "\u{135}"),
    ("K", "\u{136}"),
    ("k", "\u{137}\u{138}"),
    ("L", "\u{139}\u{13b}\u{13d}\u{13f}\u{141}"),
    ("l", "\u{13a}\u{13c}\u{13e}\u{140}\u{142}"),
    ("N", "\u{143}\u{145}\u{147}\u{14a}"),
    ("n", "\u{144}\u{146}\u{148}\u{149}\u{14b}"),
    ("O", "\u{14c}\u{14e}\u{150}"),
    ("o", "\u{14d}\u{14f}\u{151}"),
    ("R", "\u{154}\u{156}\u{158}"),
    ("r", "\u{155}\u{157}\u{159}"),
    ("S", "\u{15a}\u{15c}\u{15e}\u{218}"),
    ("s", "\u{15b}\u{15d}\u{15f}\u{17f}\u{219}"),
    ("T", "\u{162}\u{164}\u{166}\u{21a}"),
    ("t", "\u{163}\u{165}\u{167}\u{21b}"),
    ("U", "\u{168}\u{16a}\u{16c}\u{16e}\u{170}\u{172}"),
    ("u", "\u{169}\u{16b}\u{16d}\u{16f}\u{171}\u{173}"),
    ("W", "\u{174}"),
    ("w", "\u{175}"),
    ("Y", "\u{176}"),
    ("y", "\u{177}"),
    ("Z", "\u{179}\u{17b}"),
    ("z", "\u{17a}\u{17c}"),
];

/// Zeichen, die man nicht lesen kann: sie bekommen einen Namen (Meldung: Name, dann Codepunkt).
const NAMEN: &[(char, &str)] = &[
    ('\t', "Tabulator"),
    ('\n', "Zeilenumbruch"),
    ('\r', "Zeilenumbruch"),
    ('\u{a0}', "geschütztes Leerzeichen"),
    ('\u{ad}', "bedingter Trennstrich"),
    ('\u{200b}', "Leerzeichen der Breite null"),
    ('\u{feff}', "Byte-Order-Mark"),
];

/// Dasselbe fuer ganze Bereiche (Codepunkt von, bis, Name): ein Steuerzeichen oder ein Leerzeichen
/// anderer Breite stuende sonst als unsichtbares Zeichen zwischen den Anfuehrungszeichen der
/// Meldung. `NAMEN` geht vor.
const NAMEN_BEREICHE: &[(u32, u32, &str)] = &[
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
];

const ALLGEMEINER_RAT: &str = "ersetze es durch ein Zeichen, das ELSTER annimmt (lateinische Buchstaben mit Umlauten und ß, Ziffern, übliche Satzzeichen).";

/// Kombinierende Akzente (U+0300..U+036F): macOS und manche PDFs liefern „ü" als „u" plus Akzent.
/// Loeschen machte aus „Müller" ein „Muller", darum ein eigener Rat.
const KOMBINIEREND: std::ops::RangeInclusive<char> = '\u{300}'..='\u{36f}';
const KOMBINIEREND_RAT: &str = "tippe den Buchstaben mit Akzent neu, als ein Zeichen (zum Beispiel „ü\" statt „u\" und einem losen Akzent).";

/// Wie die Meldung das Zeichen nennt: `„ł" (U+0142)`, ein unlesbares mit Namen: `Tabulator (U+0009)`.
#[must_use]
pub fn zeichen_anzeige(c: char) -> String {
    let punkt = format!("U+{:04X}", u32::from(c));
    if KOMBINIEREND.contains(&c) {
        return format!("loser Akzent ({punkt})");
    }
    let cp = u32::from(c);
    let name = NAMEN
        .iter()
        .find(|(z, _)| *z == c)
        .map(|(_, name)| *name)
        .or_else(|| {
            NAMEN_BEREICHE
                .iter()
                .find(|(von, bis, _)| (*von..=*bis).contains(&cp))
                .map(|(_, _, name)| *name)
        });
    match name {
        Some(name) => format!("{name} ({punkt})"),
        None => format!("„{c}\" ({punkt})"),
    }
}

/// Der Vorschlag zu einem unerlaubten Zeichen, als Satzende der Meldung.
#[must_use]
pub fn zeichen_rat(c: char) -> String {
    if KOMBINIEREND.contains(&c) {
        return KOMBINIEREND_RAT.to_owned();
    }
    for (ersatz, zeichen) in VORSCHLAEGE {
        if zeichen.contains(c) {
            return match *ersatz {
                "" => "lösche es.".to_owned(),
                " " => "schreibe stattdessen ein normales Leerzeichen.".to_owned(),
                _ => format!("schreibe stattdessen „{ersatz}\"."),
            };
        }
    }
    ALLGEMEINER_RAT.to_owned()
}

/// Die Abweisung beim Speichern. Nennt Feld, Zeichen und Vorschlag, nie den Wert (PII).
/// Python: `zeichensatz.py::feld_meldung`, wortgleich.
///
/// ```
/// assert_eq!(
///     domain::zeichensatz::feld_meldung("stammdaten_nachname", 'ł'),
///     "fail-closed (Zeichensatz): stammdaten_nachname enthält das Zeichen „ł\" (U+0142), das ELSTER in Textfeldern nicht annimmt — schreibe stattdessen „l\"."
/// );
/// ```
#[must_use]
pub fn feld_meldung(feld_id: &str, c: char) -> String {
    format!(
        "fail-closed (Zeichensatz): {feld_id} enthält das Zeichen {}, das ELSTER in Textfeldern nicht annimmt — {}",
        zeichen_anzeige(c),
        zeichen_rat(c)
    )
}

/// Die Sperre an der XML-Erzeugung (Altbestaende, Importe): die Meldung zu einem Element, dessen
/// Text ein unerlaubtes Zeichen traegt. Nennt das Element, nie den Wert.
/// Python: `zeichensatz.py::element_meldung`, wortgleich.
#[must_use]
pub fn element_meldung(element: &str, text: &str) -> Option<String> {
    let c = erstes_unerlaubtes_zeichen(text)?;
    Some(format!(
        "Element {element} enthält das Zeichen {}, das ELSTER in Textfeldern nicht annimmt — ELSTER wiese die ganze Abgabe ab. Wert nicht geloggt. Rat: {}",
        zeichen_anzeige(c),
        zeichen_rat(c)
    ))
}
