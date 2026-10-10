//! Regal der Tabellen dieser Crate: welche Konstante oder Regel zu welchem Fixture-Schluessel in
//! `rust/fixtures/kz_tabellen.json` gehoert — und welche eine Ausnahme mit Grund ist.
//!
//! Der Guard `tabellen::tests::jede_tabelle_ist_eingetragen` liest die Quelltexte der Crate
//! (`include_str!`) und verlangt fuer jede Top-Level-Konstante einer als `vollstaendig`
//! markierten Datei einen Eintrag. Eine neue Tabelle, die niemandem zugeordnet ist, ist damit
//! nicht mehr still: sie macht den Standardlauf rot (`cargo test -p elster`, ohne `PARITY=1`).
//! Das ist die Sicherung gegen die Grenze „neue Tabelle, im Generator vergessen“
//! (Vault `backlog/taxgraph/kz-tabellen-rest`).
//!
//! Die Werte vergleicht der Test gegen die Konstanten, nicht gegen deren Text — ein Testmodul
//! sieht das `pub(crate)` und die privaten Konstanten seiner Crate. Regeln, die als Code in einer
//! Funktion stehen (`iban_muster`, `stnr_muster` ueber `elster_xml.py` verglichen, `datenart`,
//! `ist_ja_typ`, `instanz_re`), prueft der Test ueber Proben: dieselben Werte annehmen,
//! dieselben verwerfen. Ein reiner Mustertext-Vergleich waere taub, weil beide Seiten
//! denselben Text laesen.
//!
//! Welche Literale die Fixture wirklich liest, haelt `aufzeichnen` fest: `wortliche` und `einzel`
//! tragen die Lage jedes gelesenen Literals ein, solange eine Aufzeichnung laeuft. Die Wache
//! `tabellen::tests::jedes_kz_literal_ist_von_der_fixture_gelesen` (`kz_wache.rs`) verlangt, dass
//! jedes Kz-Literal im Produktionstext der Crate dazugehoert.
//!
//! ponytail: eine Zeile, die nach `pub`/`const`/`static`/`fn` nichts weiter als das Wort selbst
//! traegt, gilt als Top-Level — auch im Testmodul. Upgrade: Klammertiefe zaehlen, Testmodule
//! ueberspringen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    dead_code
)]

use std::cell::RefCell;

use serde_json::Value;

thread_local! {
    /// Adressbereiche (Anfang, Ende) der Literale, die seit `aufzeichnen` gelesen wurden; `None`: aus.
    /// Je Thread, denn `cargo test` fuehrt jeden Test auf einem eigenen aus.
    static GELESEN: RefCell<Option<Vec<(usize, usize)>>> = const { RefCell::new(None) };
}

/// Fuehrt `f` aus und liefert dazu die Adressbereiche aller Literale, die `wortliche`/`einzel`
/// dabei aus einem Regal-Text gelesen haben (Anfang inklusive, Ende exklusive, samt Anfuehrungszeichen).
pub(crate) fn aufzeichnen<T>(f: impl FnOnce() -> T) -> (T, Vec<(usize, usize)>) {
    GELESEN.with(|g| *g.borrow_mut() = Some(Vec::new()));
    let aus = f();
    let gelesen = GELESEN.with(|g| g.borrow_mut().take()).unwrap_or_default();
    (aus, gelesen)
}

/// Tragen `text[von..bis]` als gelesen ein; ohne laufende Aufzeichnung ein Nichts.
fn gelesen(text: &str, von: usize, bis: usize) {
    let basis = text.as_ptr().addr();
    GELESEN.with(|g| {
        if let Some(liste) = g.borrow_mut().as_mut() {
            liste.push((basis + von, basis + bis));
        }
    });
}

/// Was eine Konstante oder Regel mit der Fixture zu tun hat.
#[derive(Clone, Copy)]
pub(crate) enum Zuordnung {
    /// Der Wert steht unter diesem Pfad in der Fixture und muss gleich sein.
    Schluessel(&'static str),
    /// Dieselbe Regel, die ein anderer Eintrag traegt (Zaehlung, kein eigener Vergleich).
    Regel(&'static str),
    /// Kein Gegenstueck in `est_mapping.py`, `elster_xml.py` oder `checkest_gate.py`: mit Grund.
    Ausnahme(&'static str),
    /// Eine Funktion, die die Regel als Code traegt (Muster, Routing, Klassenliste): geprueft
    /// ueber die Proben unter diesem Pfad in der Fixture, nicht ueber den Wortlaut.
    Funktion(&'static str),
    /// Eine Funktion, deren Kz-Literale kein Fixture-Wert liest, sondern Verhaltenstests: `kz` sind
    /// alle Kz-Literale der Funktion in Lesereihenfolge (jede Abweichung macht `kz_wache` rot, bis
    /// jemand sie hier und im Test nachzieht), `tests` die Tests im selben Quelltext, die bei einem
    /// falschen Literal rot werden (gemessen, mit einem Mutanten je Literal).
    Verhalten { kz: &'static [&'static str], tests: &'static [&'static str] },
}

/// Eine Datei der Crate, ihr Text und ihre Eintraege.
pub(crate) struct Datei {
    pub name: &'static str,
    pub text: &'static str,
    pub eintraege: &'static [(&'static str, Zuordnung)],
    /// `true`: jede Top-Level-Konstante dieser Datei ist im Regal verpflichtet.
    pub vollstaendig: bool,
}

/// Der Kernsatz: je Datei ihre Konstanten, Funktionen und ihre Zuordnung.
pub(crate) const REGAL: &[Datei] = &[
    Datei {
        name: "tabellen.rs",
        text: include_str!("tabellen.rs"),
        eintraege: &[
            ("KONSTANTE_KZ", Zuordnung::Schluessel("konstante_kz")),
            ("IBAN_TRANSFORM_ZIEL_KZ", Zuordnung::Schluessel("iban_transform_ziel_kz")),
            ("NEGATION", Zuordnung::Schluessel("negation")),
            ("MULTIPLIKATION", Zuordnung::Schluessel("multiplikation")),
            ("DOKUMENTIERT_AGGREGAT", Zuordnung::Schluessel("dokumentiert_aggregat")),
            ("P23_BETRAGSFELDER", Zuordnung::Schluessel("p23/betragsfelder")),
            ("P23_ART_FELD", Zuordnung::Schluessel("p23/art_feld")),
            ("P23_GEWINN_KZ", Zuordnung::Schluessel("p23/gewinn_kz")),
            ("VERAEUSSERUNG", Zuordnung::Regel("verzweigung")),
            ("ANTRAG_ABS3", Zuordnung::Regel("verzweigung")),
            ("GEWINN", Zuordnung::Regel("verzweigung")),
            ("GEWINN_BEZEICHNUNG", Zuordnung::Regel("verzweigung")),
            ("BASIS_KV", Zuordnung::Regel("verzweigung")),
            ("BASIS_PV", Zuordnung::Regel("verzweigung")),
            ("VERZWEIGUNG", Zuordnung::Schluessel("verzweigung")),
            ("VERZWEIGUNG", Zuordnung::Schluessel("p35c_massnahme_art_reihenfolge")),
            ("PARTNER_VERZWEIGUNG", Zuordnung::Schluessel("partner_verzweigung")),
            ("PARTNER_INSTANZ", Zuordnung::Schluessel("partner_instanz")),
            ("PFLEGE_KZ", Zuordnung::Schluessel("pflege_kz")),
            ("KAP_FELDER_A", Zuordnung::Schluessel("kap_felder_a")),
            ("KAP_FELDER_B", Zuordnung::Schluessel("kap_felder_b")),
            ("KAP_NULL_GRUND", Zuordnung::Schluessel("kap_null_grund")),
            ("PFLICHTFELDER", Zuordnung::Schluessel("pflichtfelder")),
            ("WERTEKODIERUNG", Zuordnung::Schluessel("wertekodierung")),
            ("WERTEKODIERUNG", Zuordnung::Schluessel("hinweise")),
            ("WERTEKODIERUNG", Zuordnung::Schluessel("wertekodierung_andere_ohne_code")),
            ("rente_wert", Zuordnung::Funktion("verzweigung")),
            ("rente_beginn", Zuordnung::Funktion("verzweigung")),
            ("konfession_code", Zuordnung::Funktion("wertekodierung")),
            ("suche", Zuordnung::Ausnahme("Nachschlag-Helfer, fuehrt keine Kz")),
        ],
        vollstaendig: true,
    },
    Datei {
        name: "kz_format.rs",
        text: include_str!("kz_format.rs"),
        eintraege: &[
            ("ABZUGS_KZ", Zuordnung::Schluessel("abzugs_kz")),
            ("KOMMA_OHNE_E60_KZ", Zuordnung::Schluessel("komma_ohne_e60_kz")),
            ("NULL_UNZULAESSIG_KZ_2024", Zuordnung::Schluessel("null_unzulaessig/je_vz/2024")),
            ("NULL_UNZULAESSIG_KZ_2025", Zuordnung::Schluessel("null_unzulaessig/je_vz/2025")),
            ("NULL_UNZULAESSIG_KZ_VEREINIGUNG", Zuordnung::Schluessel("null_unzulaessig/vereinigung")),
            ("DATUMS_KZ", Zuordnung::Schluessel("datums_kz")),
            ("kz_format", Zuordnung::Funktion("datums_kz")),
            ("cent_nach_kz", Zuordnung::Funktion("abzugs_kz")),
            ("kz_wert", Zuordnung::Funktion("datums_kz")),
            ("schreibe_kz", Zuordnung::Funktion("null_unzulaessig")),
            ("null_unzulaessig", Zuordnung::Funktion("null_unzulaessig/je_vz")),
        ],
        vollstaendig: true,
    },
    Datei {
        name: "deklaration.rs",
        text: include_str!("deklaration.rs"),
        eintraege: &[
            ("P35A_SUMME_AUS_POSTEN", Zuordnung::Schluessel("p35a_summe_aus_posten")),
            // Abweichung Nr. 41: das Kz des Abzugs nach § 34c Abs. 2 gibt es in Python nicht; der Test deckt das Literal.
            (
                "DBA_ABZUG_KZ",
                Zuordnung::Verhalten {
                    kz: &["E0600920"],
                    tests: &["abzug_traegt_die_steuer_unter_dem_abzugs_kz_und_nicht_als_anrechnung"],
                },
            ),
            // Abweichung Nr. 48: die Zeile "Sonstiges" der Anlage N fuer die Unfallkosten gibt es in Python nicht; der Test deckt die Literale.
            // Abweichung Nr. 49: dieselben Kz tragen die zweite Zeile (Abzug nach § 34c Abs. 2), die Summe bildet beide Zeilen.
            (
                "UNFALLKOSTEN_TEXT_KZ",
                Zuordnung::Verhalten {
                    kz: &["E0205405"],
                    tests: &[
                        "unfallkosten_stehen_ueber_null_in_der_zeile_sonstiges",
                        "abzug_steht_als_zweite_zeile_sonstiges_und_die_summe_bildet_beide_zeilen",
                    ],
                },
            ),
            (
                "UNFALLKOSTEN_BETRAG_KZ",
                Zuordnung::Verhalten {
                    kz: &["E0205406"],
                    tests: &[
                        "unfallkosten_stehen_ueber_null_in_der_zeile_sonstiges",
                        "abzug_steht_als_zweite_zeile_sonstiges_und_die_summe_bildet_beide_zeilen",
                    ],
                },
            ),
            (
                "WEITERE_WK_SUMME_KZ",
                Zuordnung::Verhalten {
                    kz: &["E0204803"],
                    tests: &[
                        "unfallkosten_stehen_ueber_null_in_der_zeile_sonstiges",
                        "abzug_steht_als_zweite_zeile_sonstiges_und_die_summe_bildet_beide_zeilen",
                    ],
                },
            ),
            // Abweichung Nr. 51: der Staat der Auslandseinkuenfte (Anlage AUS "1. Staat") gibt es in Python nicht; der Test deckt das Literal.
            (
                "STAAT_KZ",
                Zuordnung::Verhalten {
                    kz: &["E0600301"],
                    tests: &["der_staat_steht_als_listentext_in_e0600301_genau_mit_den_einkuenften"],
                },
            ),
            // Abweichung Nr. 50: die Zeilen 11 bis 13 der Anlage N (Versorgungsbezug) gibt es in Python nicht; der Test deckt die Literale.
            (
                "VERSORGUNG_BETRAG_KZ",
                Zuordnung::Verhalten {
                    kz: &["E0200801"],
                    tests: &["versorgung_steht_nur_mit_dem_ring_wert_in_den_zeilen_11_bis_13"],
                },
            ),
            (
                "VERSORGUNG_BMG_KZ",
                Zuordnung::Verhalten {
                    kz: &["E0200902"],
                    tests: &["versorgung_steht_nur_mit_dem_ring_wert_in_den_zeilen_11_bis_13"],
                },
            ),
            (
                "VERSORGUNG_BEGINN_KZ",
                Zuordnung::Verhalten {
                    kz: &["E0201307"],
                    tests: &["versorgung_steht_nur_mit_dem_ring_wert_in_den_zeilen_11_bis_13"],
                },
            ),
            // Diese Regeln stehen als Literale in Methoden von `Bau`; ihr Top-Level-Name ist `deklariere`.
            ("deklariere", Zuordnung::Funktion("iban_weiche/praefix")),
            ("deklariere", Zuordnung::Funktion("iban_weiche/inland")),
            ("deklariere", Zuordnung::Funktion("iban_weiche/ausland")),
            ("deklariere", Zuordnung::Funktion("bankverbindung/iban")),
            ("deklariere", Zuordnung::Funktion("bankverbindung/keine_bankverbindung")),
            ("deklariere", Zuordnung::Funktion("bankverbindung/kontoinhaber")),
            ("deklariere", Zuordnung::Funktion("pflege/grad_kz")),
            ("deklariere", Zuordnung::Funktion("pflege/h_kz")),
            ("iban_muster", Zuordnung::Funktion("regex/iban")),
            ("zustand_text", Zuordnung::Ausnahme("Helfer ohne Fixture-Wert")),
            ("iban_pruefziffer_gueltig", Zuordnung::Ausnahme("ISO 13616 Modulo 97, keine Python-Tabelle")),
        ],
        vollstaendig: false,
    },
    Datei {
        name: "xml.rs",
        text: include_str!("xml.rs"),
        eintraege: &[
            ("NS_ELSTER", Zuordnung::Schluessel("elster_xml/ns_elster")),
            ("erzeuge_xml", Zuordnung::Funktion("elster_xml/ns_e10_format")),
            ("TESTMERKER_ERIC", Zuordnung::Schluessel("elster_xml/testmerker_eric")),
            ("E10_AUSSCHLUSS_DATENART", Zuordnung::Schluessel("elster_xml/e10_ausschluss_datenart")),
            ("INSTANZ_CONTAINER_TIEFER", Zuordnung::Schluessel("elster_xml/instanz_container_tiefer")),
            ("PFLICHT_DEFAULT", Zuordnung::Schluessel("elster_xml/pflicht_default")),
            ("INSTANZ_NUMMER_FELDER", Zuordnung::Schluessel("elster_xml/instanz_nummer_felder")),
            ("ABSENDER_HERKUNFT", Zuordnung::Schluessel("elster_xml/absender_herkunft")),
            ("ABSENDER_STRASSE_ZUSATZ_KZ", Zuordnung::Schluessel("elster_xml/absender_strasse_zusatz_kz")),
            // Nr 59: die Bankverbindungs-Entscheidung; ihre drei Kz fuehrt keine Fixture, die Tests schon.
            (
                "abgabe_pruefen",
                Zuordnung::Verhalten {
                    kz: &["E0102002", "E0102102", "E0102603"],
                    tests: &[
                        "nur_der_schalter_keine_bankverbindung_genuegt",
                        "nur_die_inlands_iban_genuegt_als_bankverbindung",
                        "nur_die_auslands_iban_genuegt_als_bankverbindung",
                    ],
                },
            ),
        ],
        vollstaendig: false,
    },
    Datei {
        name: "xsd.rs",
        text: include_str!("xsd.rs"),
        eintraege: &[
            ("XS", Zuordnung::Schluessel("xsd_verify/xs_namespace")),
            ("MAX_DEPTH", Zuordnung::Schluessel("xsd_verify/max_depth")),
            ("datenart", Zuordnung::Funktion("xsd_verify/datenart/default")),
            ("datenart", Zuordnung::Funktion("xsd_verify/datenart/routing/60")),
            ("schema_info", Zuordnung::Funktion("elster_xml/eric_pflicht_trotz_optional")),
            ("datenart", Zuordnung::Funktion("xsd_verify/datenart")),
            ("ist_ja_typ", Zuordnung::Funktion("xsd_verify/ja_typ_pattern")),
        ],
        vollstaendig: false,
    },
    Datei {
        name: "instanz.rs",
        text: include_str!("instanz.rs"),
        eintraege: &[
            ("instanz_re", Zuordnung::Funktion("regex/instanz")),
            ("parse_instanz", Zuordnung::Funktion("regex/instanz")),
        ],
        vollstaendig: true,
    },
    Datei {
        name: "xmllint.rs",
        text: include_str!("xmllint.rs"),
        eintraege: &[
            ("finde_xsd_schema", Zuordnung::Funktion("eric/extern_schema_muster")),
        ],
        vollstaendig: false,
    },
    Datei {
        name: "eric/mod.rs",
        text: include_str!("eric/mod.rs"),
        eintraege: &[
            ("ERIC_VALIDIERE", Zuordnung::Schluessel("eric_rc/validiere")),
            ("VALIDIERE_MELDUNGEN_MAX", Zuordnung::Schluessel("eric_rc/meldungen_max")),
            ("RC_OK", Zuordnung::Schluessel("eric_rc/rc/RC_OK")),
            ("RC_PLAUSIBILITAET", Zuordnung::Schluessel("eric_rc/rc/RC_PLAUSIBILITAET")),
            ("RC_IO_SCHEMA_VALIDIERUNGSFEHLER", Zuordnung::Schluessel("eric_rc/rc/RC_IO_SCHEMA_VALIDIERUNGSFEHLER")),
            ("RC_HERSTELLER_GESPERRT", Zuordnung::Schluessel("eric_rc/rc/RC_HERSTELLER_GESPERRT")),
            ("RC_DATENARTVERSION_UNBEKANNT", Zuordnung::Schluessel("eric_rc/rc/RC_DATENARTVERSION_UNBEKANNT")),
            ("RC_IO_UNERWARTETE_ELEMENTE", Zuordnung::Schluessel("eric_rc/rc/RC_IO_UNERWARTETE_ELEMENTE")),
            ("klassifiziere_rc", Zuordnung::Funktion("eric_rc/klassen")),
            ("klasse_name", Zuordnung::Funktion("eric_rc/klassen")),
            ("nicht_geprueft", Zuordnung::Funktion("eric_rc/nicht_geprueft_klassen")),
            ("ARBEITER", Zuordnung::Ausnahme("Thread-Handle des ERiC-Arbeiters")),
            ("LOG_DIR", Zuordnung::Ausnahme("Zielverzeichnis fuer eric.log")),
        ],
        vollstaendig: false,
    },
    Datei {
        name: "py.rs",
        text: include_str!("py.rs"),
        eintraege: &[
        ],
        vollstaendig: false,
    },
    Datei {
        name: "geordnet.rs",
        text: include_str!("geordnet.rs"),
        eintraege: &[

        ],
        vollstaendig: true,
    },
    Datei {
        name: "lib.rs",
        text: include_str!("lib.rs"),
        eintraege: &[
        ],
        vollstaendig: true,
    },
    // `regal.rs` fuehrt sich nicht selbst: seine Hilfen sind Werkzeug, keine Tabelle.
];

/// Die Proben, die ein Rust-Praedikat und die Fixture bestehen muessen. Pfad = Schluessel, unter
/// dem die Fixture `proben` traegt; der Wert ist das Python-Ergebnis (gemessen, kein Rat).
pub(crate) const PROBE_PAARE: &[(&str, &[(&str, bool)])] = &[
    ("regex/iban", &[
        ("DE89370400440532013000", true), ("GB82WEST12345698765432", true), ("DE8937", true),
        ("XX00", false), ("de89370400440532013000", false), ("12345", false),
    ]),
    ("regex/steuernummer", &[
        ("9181012345678", true), ("1234012345678", true), ("91811234567", false),
        ("91810123456789", false), ("918101234567a", false),
    ]),
    ("regex/instanz", &[
        ("vv_einnahmen__2", true), ("vv_einnahmen__10", true), ("vv_einnahmen__1", false),
        ("vv_einnahmen__02", false), ("vv_einnahmen", false), ("a__2", true),
    ]),
    ("xsd_verify/kz_pattern", &[
        ("E0100001", true), ("E0241302", true), ("E123456", false), ("e0100001", false),
        ("X0100001", false), ("E01000011", false),
    ]),
    ("xsd_verify/ja_typ_pattern", &[
        ("Ja1BaseCType", true), ("JaXBaseCType", true), ("JaNein12BaseCType", true),
        ("Ja2BaseCType_RABE", true), ("JaBaseCType", true), ("GanzzahlPosCType_RABE", false),
    ]),
];

/// Name des Eintrags in einer Zeile, die mit `wort` beginnt (nach `pub`, `pub(crate)`). `i` zeigt
/// auf den Anfang des Wortes; die Zeile davor muss leer sein — eine eingerueckte Zeile gehoert
/// einem Funktionskoerper oder einem Testmodul und ist kein Top-Level-Eintrag.
fn oberster_name<'a>(text: &'a str, i: usize, wort: &str) -> Option<&'a str> {
    let anfang = text[..i].rfind('\n').map_or(0, |j| j + 1);
    // Kein `trim_start`: eine eingerueckte Zeile ist nie Top-Level.
    let vor = &text[anfang..i];
    let vor = match vor.strip_prefix("pub") {
        Some(rest) => rest
            .strip_prefix('(')
            .map_or(rest, |k| k.split_once(')').map_or("", |(_, nach)| nach))
            .trim_start_matches(' '),
        None => vor,
    };
    if !vor.is_empty() {
        return None;
    }
    Some(name_am_anfang(text[i..].strip_prefix(wort)?))
}

/// Der Bezeichner am Anfang eines Textstuecks (auch `fn NAME` nach einem Schluesselwort).
fn name_am_anfang(ohne: &str) -> &str {
    let ohne = ohne.strip_prefix("fn ").unwrap_or(ohne);
    let ende = ohne
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(ohne.len());
    &ohne[..ende]
}

/// Die Top-Level-Konstanten und Statik-Variablen eines Textes, sortiert, ohne Doppelung.
pub(crate) fn top_level_namen(text: &str) -> Vec<&str> {
    let mut aus = Vec::new();
    for wort in ["const ", "static "] {
        let mut i = 0;
        while let Some(j) = text[i..].find(wort) {
            let p = i + j;
            // `const fn` ist eine Funktion, keine Konstante.
            let ist_funktion = text[p + wort.len()..].starts_with("fn ");
            if let Some(name) = oberster_name(text, p, wort).filter(|_| !ist_funktion) {
                if !name.is_empty() {
                    aus.push(name);
                }
            }
            i = p + wort.len();
        }
    }
    aus.sort_unstable();
    aus.dedup();
    aus
}

/// Die Top-Level-Funktionen eines Textes, sortiert. Fuer `Zuordnung::Funktion`: Regeln, die als
/// Code stehen und nur ueber ihre Proben messbar sind.
pub(crate) fn top_level_funktionen(text: &str) -> Vec<&str> {
    let mut aus = Vec::new();
    for wort in ["fn ", "const fn "] {
        let mut i = 0;
        while let Some(j) = text[i..].find(wort) {
            let p = i + j;
            if let Some(name) = oberster_name(text, p, wort) {
                if !name.is_empty() {
                    aus.push(name);
                }
            }
            i = p + wort.len();
        }
    }
    aus.sort_unstable();
    aus.dedup();
    aus
}

/// Die Regal-Datei ihres Namens wegen.
pub(crate) fn datei_von(name: &str) -> Option<&'static Datei> {
    REGAL.iter().find(|d| d.name == name)
}

/// Die Textzone einer Regel in der Regal-Datei: von der Zeile mit `anker` bis zum ersten
/// Vorkommen von `schliessen`, das `offnen` ausgleicht (`"[`/`"]"`; `""` = die Ankerzeile allein).
/// `Err` nennt den Grund — fehlender Anker, fehlendes Blockende. Eine Regel, die der Schluessel
/// nicht tragen kann, meldet also, statt still gruen zu werden.
pub(crate) fn zone(name: &str, anker: &str, offnen: &str, schliessen: &str) -> Result<&'static str, String> {
    let text = datei_von(name)
        .ok_or_else(|| format!("Datei {name} fehlt im Regal"))?
        .text;
    anker_zone(text, anker, offnen, schliessen)
}

/// Von der Zeile mit `anker` bis zum ersten `ende` danach, einschliesslich: fuer Text, der mit `\`
/// ueber Zeilen laeuft, und fuer Anweisungen, deren Klammern nichts zaehlen sollen.
pub(crate) fn zone_bis(name: &str, anker: &str, ende: &str) -> Result<&'static str, String> {
    let text = datei_von(name)
        .ok_or_else(|| format!("Datei {name} fehlt im Regal"))?
        .text;
    let i = text
        .find(anker)
        .ok_or_else(|| format!("Anker {anker:?} nicht gefunden"))?;
    let anfang = text[..i].rfind('\n').map_or(0, |j| j + 1);
    let j = text[i..]
        .find(ende)
        .ok_or_else(|| format!("Ende {ende:?} nach {anker:?} fehlt"))?;
    Ok(&text[anfang..i + j + ende.len()])
}

pub(crate) fn anker_zone(
    text: &'static str,
    anker: &str,
    offnen: &str,
    schliessen: &str,
) -> Result<&'static str, String> {
    let i = text
        .find(anker)
        .ok_or_else(|| format!("Anker {anker:?} nicht gefunden"))?;
    let anfang = text[..i].rfind('\n').map_or(0, |j| j + 1);
    if offnen.is_empty() {
        let ende = text[anfang..]
            .find('\n')
            .map_or(text.len(), |j| anfang + j);
        return Ok(&text[anfang..ende]);
    }
    // Bei `const X: &[&str] = &[…]` zaehlt die Klammer des Wertes, nicht die des Typs links vom `=`.
    let zeilenende = text[anfang..].find('\n').map_or(text.len(), |j| anfang + j);
    let ab = text[anfang..zeilenende]
        .find(" = ")
        .map_or(anfang, |j| anfang + j + 3);
    let start = ab
        + text[ab..]
            .find(offnen)
            .ok_or_else(|| format!("{offnen} fehlt in der Ankerzeile {anker:?}"))?;
    let mut tiefe = 0usize;
    let mut i = start;
    while i < text.len() {
        // Byteweise vergleichen: `text[i..]` paniert mitten in einem Mehrbyte-Zeichen („Straße“).
        if text.as_bytes()[i..].starts_with(offnen.as_bytes()) {
            tiefe += 1;
            i += offnen.len();
            continue;
        }
        if text.as_bytes()[i] == b'\\' && i + 1 < text.len() {
            i += 2;
            continue;
        }
        if text.as_bytes()[i..].starts_with(schliessen.as_bytes()) {
            tiefe = tiefe.checked_sub(1).ok_or_else(|| format!("einseitiges {schliessen:?}"))?;
            i += schliessen.len();
            if tiefe == 0 {
                return Ok(&text[anfang..i]);
            }
            continue;
        }
        i += 1;
    }
    Err(format!("Blockende {schliessen:?} nach {anker:?} fehlt"))
}

/// Alle `"..."`-Literale eines Textstuecks mit ihrer Lage (Byte-Bereich samt Anfuehrungszeichen),
/// in Reihenfolge des Lesens. Traegt nichts als gelesen ein.
///
/// Rust-Zeilenfortsetzung: ein `\` vor dem Zeilenende streicht den Umbruch und den fuehrenden
/// Leerraum der Folgezeile, wie der Compiler es tut.
fn literale(text: &str) -> Vec<(String, usize, usize)> {
    let mut aus = Vec::new();
    let mut zeichen = text.char_indices().peekable();
    while let Some((von, c)) = zeichen.next() {
        if c != '"' {
            continue;
        }
        let mut innen = String::new();
        let mut bis = text.len();
        while let Some((i, c)) = zeichen.next() {
            match c {
                '"' => {
                    bis = i + 1;
                    break;
                }
                '\\' => match zeichen.next().map(|(_, x)| x) {
                    Some('n') => innen.push('\n'),
                    Some('t') => innen.push('\t'),
                    Some('r') => innen.push('\r'),
                    Some('\n') => {
                        while zeichen.peek().is_some_and(|(_, x)| x.is_whitespace()) {
                            zeichen.next();
                        }
                    }
                    Some(anderes) => innen.push(anderes),
                    None => break,
                },
                _ => innen.push(c),
            }
        }
        aus.push((innen, von, bis));
    }
    aus
}

/// Alle `"..."`-Literale eines Textstuecks, in Reihenfolge des Lesens. Jedes zaehlt als gelesen.
pub(crate) fn wortliche(text: &str) -> Vec<String> {
    literale(text)
        .into_iter()
        .map(|(s, von, bis)| {
            gelesen(text, von, bis);
            s
        })
        .collect()
}

/// Literale als Menge (sortiert, ohne Doppelung): die Form der Fixture fuer `frozenset`.
pub(crate) fn menge(text: &str) -> Value {
    let mut liste: Vec<String> = wortliche(text);
    liste.sort_unstable();
    liste.dedup();
    Value::Array(liste.into_iter().map(Value::from).collect())
}

/// Literale in Reihenfolge des Textes (Listen, geordnete Mengen).
pub(crate) fn folge(text: &str) -> Value {
    Value::Array(wortliche(text).into_iter().map(Value::from).collect())
}

/// `(Schluessel, Wert)`-Paare eines Textstuecks als Objekt.
pub(crate) fn paar_objekt(text: &str) -> Value {
    let w = wortliche(text);
    assert!(w.len().is_multiple_of(2), "unaarige Literale in {text:?}");
    let mut m = serde_json::Map::new();
    for paar in w.chunks(2) {
        m.insert(paar[0].clone(), Value::String(paar[1].clone()));
    }
    Value::Object(m)
}

/// Das `n`-te Literal eines Textstuecks (fuer Einzelwerte, z. B. einen Namespace). Nur dieses
/// zaehlt als gelesen: ein weiteres Literal in derselben Zone sieht die Fixture nicht.
pub(crate) fn einzel(text: &str, n: usize) -> String {
    let (s, von, bis) = literale(text)
        .into_iter()
        .nth(n)
        .unwrap_or_else(|| panic!("kein Literal {n} in {text:?}"));
    gelesen(text, von, bis);
    s
}

/// Die Form einer Kennzahl `E` + 7 Ziffern: sie trennt Kz-Literale von Feldnamen und Texten.
pub(crate) fn ist_kz_form(s: &str) -> bool {
    s.len() == 8 && s.starts_with('E') && s[1..].bytes().all(|b| b.is_ascii_digit())
}

/// Eine Fixture-Referenz; `None` bei fehlendem Pfad. Pfade getrennt durch `/`.
pub(crate) fn hole<'a>(fix: &'a Value, pfad: &str) -> Option<&'a Value> {
    let mut aktuell = fix;
    for teil in pfad.split('/') {
        aktuell = aktuell.get(teil)?;
    }
    Some(aktuell)
}

/// Proben-Vergleich: Rust-Praedikat gegen die Paare des Regals, Fixture gegen dieselben Paare.
/// Ein einseitiges Muster (nur eine Seite hat Proben) ist ein Fehler.
pub(crate) fn proben(pfad: &str, fix: &Value, rust: fn(&str) -> bool) -> Vec<String> {
    let Some(paare) = PROBE_PAARE.iter().find(|(n, _)| *n == pfad) else {
        return vec![format!("{pfad}: keine Proben im Regal")];
    };
    let mut aus = Vec::new();
    for (wert, ok) in paare.1 {
        if rust(wert) != *ok {
            aus.push(format!("{pfad}: {wert:?} Rust={}, Regal erwartet {ok}", rust(wert)));
        }
    }
    match fix.get("proben").and_then(|p| p.get(pfad)) {
        None => aus.push(format!("{pfad}/proben: fehlt in der Fixture")),
        Some(Value::Array(liste)) => {
            for p in liste {
                let (Some(w), Some(ok)) = (
                    p.get("wert").and_then(Value::as_str),
                    p.get("erwartet").and_then(Value::as_bool),
                ) else {
                    aus.push(format!("{pfad}/proben: defekter Eintrag {p}"));
                    continue;
                };
                match paare.1.iter().find(|(x, _)| *x == w) {
                    None => aus.push(format!("{pfad}/proben: {w:?} fehlt im Regal")),
                    Some((_, r)) if *r != ok => {
                        aus.push(format!("{pfad}/proben: {w:?} Regal {r}, Fixture {ok}"));
                    }
                    Some(_) => {}
                }
            }
            if liste.len() != paare.1.len() {
                aus.push(format!(
                    "{pfad}/proben: Fixture {} Proben, Regal {}",
                    liste.len(),
                    paare.1.len()
                ));
            }
        }
        Some(anders) => aus.push(format!("{pfad}/proben: unerwartete Form {anders}")),
    }
    aus
}

/// Rueckgabe: (Top-Level-Eintraege ohne Regal-Eintrag, Regal-Eintraege ohne solchen Eintrag).
pub(crate) fn regal_fehler() -> (Vec<String>, Vec<String>) {
    let mut fehlt: Vec<String> = Vec::new();
    let mut veraltet: Vec<String> = Vec::new();
    for datei in REGAL {
        let namen = top_level_namen(datei.text);
        if datei.vollstaendig {
            for n in &namen {
                if !datei.eintraege.iter().any(|(e, _)| e == n) {
                    fehlt.push(format!("{}::{n}", datei.name));
                }
            }
        }
        for (e, _) in datei.eintraege {
            // Eine Konstante oder eine Funktion kann tragen: eine Regel, die als Code steht, hat
            // nur ihre Funktion als Namen.
            let traegt = namen.iter().any(|n| n == e)
                || top_level_funktionen(datei.text).contains(e);
            if !traegt {
                veraltet.push(format!("{}::{e}", datei.name));
            }
        }
    }
    fehlt.sort();
    fehlt.dedup();
    veraltet.sort();
    veraltet.dedup();
    (fehlt, veraltet)
}

/// Die Pfade, die das Regal der Fixture zuordnet und die die Fixture haben muss: `Schluessel`
/// und `Funktion` (dessen Text ein Pfad ist, kein Satz). `Regel` und `Ausnahme` zaehlen nicht.
pub(crate) fn gemeldete_schluessel() -> Vec<&'static str> {
    let mut aus: Vec<&'static str> = REGAL
        .iter()
        .flat_map(|d| d.eintraege.iter())
        .filter_map(|(_, z)| match z {
            Zuordnung::Schluessel(s) => Some(*s),
            Zuordnung::Funktion(s) if !s.contains(' ') => Some(*s),
            _ => None,
        })
        .collect();
    aus.sort_unstable();
    aus.dedup();
    aus
}
