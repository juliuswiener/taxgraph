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

use serde_json::Value;

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
            ("PARTNER_VERZWEIGUNG", Zuordnung::Schluessel("partner_verzweigung")),
            ("PARTNER_INSTANZ", Zuordnung::Schluessel("partner_instanz")),
            ("PFLEGE_KZ", Zuordnung::Schluessel("pflege_kz")),
            ("KAP_FELDER_A", Zuordnung::Schluessel("kap_felder_a")),
            ("KAP_FELDER_B", Zuordnung::Schluessel("kap_felder_b")),
            ("KAP_NULL_GRUND", Zuordnung::Schluessel("kap_null_grund")),
            ("PFLICHTFELDER", Zuordnung::Schluessel("pflichtfelder")),
            ("WERTEKODIERUNG", Zuordnung::Schluessel("wertekodierung")),
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
            ("ist_leerraum_zeile", Zuordnung::Ausnahme("Helfer ohne Fixture-Wert")),
        ],
        vollstaendig: true,
    },
    Datei {
        name: "deklaration.rs",
        text: include_str!("deklaration.rs"),
        eintraege: &[
            ("P35A_SUMME_AUS_POSTEN", Zuordnung::Schluessel("p35a_summe_aus_posten")),
            ("IBAN_PRAEFIX", Zuordnung::Schluessel("iban_weiche/praefix")),
            ("IBAN_INLAND_KZ", Zuordnung::Schluessel("iban_weiche/inland")),
            ("IBAN_AUSLAND_KZ", Zuordnung::Schluessel("iban_weiche/ausland")),
            ("BV_KEINE_KZ", Zuordnung::Schluessel("bankverbindung/keine_bankverbindung")),
            ("BV_KONTOINHABER_KZ", Zuordnung::Schluessel("bankverbindung/kontoinhaber")),
            ("PFLEGE_GRAD_KZ", Zuordnung::Schluessel("pflege/grad_kz")),
            ("PFLEGE_H_KZ", Zuordnung::Schluessel("pflege/h_kz")),
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
            ("NS_E10_FORMAT", Zuordnung::Schluessel("elster_xml/ns_e10_format")),
            ("TESTMERKER_ERIC", Zuordnung::Schluessel("elster_xml/testmerker_eric")),
            ("E10_AUSSCHLUSS_DATENART", Zuordnung::Schluessel("elster_xml/e10_ausschluss_datenart")),
            ("INSTANZ_CONTAINER_TIEFER", Zuordnung::Schluessel("elster_xml/instanz_container_tiefer")),
            ("PFLICHT_DEFAULT", Zuordnung::Schluessel("elster_xml/pflicht_default")),
            ("INSTANZ_NUMMER_FELDER", Zuordnung::Schluessel("elster_xml/instanz_nummer_felder")),
            ("ABSENDER_HERKUNFT", Zuordnung::Schluessel("elster_xml/absender_herkunft")),
            ("ABSENDER_STRASSE_ZUSATZ_KZ", Zuordnung::Schluessel("elster_xml/absender_strasse_zusatz_kz")),
            ("CACHE", Zuordnung::Ausnahme("Schema-Puffer nach VZ, kein Python-Wert")),
        ],
        vollstaendig: false,
    },
    Datei {
        name: "xsd.rs",
        text: include_str!("xsd.rs"),
        eintraege: &[
            ("XS", Zuordnung::Schluessel("xsd_verify/xs_namespace")),
            ("MAX_DEPTH", Zuordnung::Schluessel("xsd_verify/max_depth")),
            ("DATENART_DEFAULT", Zuordnung::Schluessel("xsd_verify/datenart/default")),
            ("DATENART_PRAEFIX_E60", Zuordnung::Schluessel("xsd_verify/datenart/routing/60")),
            ("DATENART_E77", Zuordnung::Schluessel("xsd_verify/datenart/routing/60")),
            ("ERIC_PFLICHT_KIND", Zuordnung::Schluessel("elster_xml/eric_pflicht_trotz_optional")),
            ("datenart", Zuordnung::Funktion("xsd_verify/datenart")),
            ("ist_ja_typ", Zuordnung::Funktion("xsd_verify/ja_typ_pattern")),
            ("MINI_XSD", Zuordnung::Ausnahme("selbst gebautes Schema, nur im Testmodul (Lizenz)")),
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
            ("EXTERN_SCHEMA_MUSTER", Zuordnung::Schluessel("eric/extern_schema_muster")),
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
            ("INT", Zuordnung::Ausnahme("D-Nummern des int-Helfers, nur im Testmodul")),
            ("REPR", Zuordnung::Ausnahme("D-Nummern des repr-Helfers, nur im Testmodul")),
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
            ("testhilfe", Zuordnung::Ausnahme("versteckte Testhilfe, kein Wert")),
        ],
        vollstaendig: true,
    },
    Datei {
        name: "regal.rs",
        text: include_str!("regal.rs"),
        eintraege: &[
            ("REGAL", Zuordnung::Regel("das Regal selbst")),
            ("PROBE_PAARE", Zuordnung::Regel("das Regal selbst")),
            ("Datei", Zuordnung::Ausnahme("Datensatz des Regals")),
            ("Zuordnung", Zuordnung::Ausnahme("Datensatz des Regals")),
            ("KÜRZEL", Zuordnung::Ausnahme("Baustein der Suchwoerter, damit sie sich nicht selbst finden")),
            ("WORTER", Zuordnung::Ausnahme("Baustein der Suchwoerter")),
            ("wort_const", Zuordnung::Funktion("Bildung des Suchwortes")),
            ("wort_static", Zuordnung::Funktion("Bildung des Suchwortes")),
            ("oberster_name", Zuordnung::Funktion("Bildung des Namens")),
            ("name_am_anfang", Zuordnung::Funktion("Bildung des Namens")),
            ("top_level_namen", Zuordnung::Funktion("Bildung des Regals")),
            ("einzel", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("menge", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("folge", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("paar_objekt", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("erste_menge", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("wortliche", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("zone", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("anker_zone", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("datei_von", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("ort_vergleich", Zuordnung::Funktion("Ortsvergleich")),
            ("zeile_der", Zuordnung::Funktion("Ortsvergleich")),
            ("text_der", Zuordnung::Funktion("Lesen aus dem Quelltext")),
            ("konfession_codes", Zuordnung::Funktion("Regel: wertekodierung")),
            ("rentenart_wert", Zuordnung::Funktion("Regel: verzweigung")),
            ("rentenart_beginn", Zuordnung::Funktion("Regel: verzweigung")),
            ("ibans", Zuordnung::Funktion("Regel: regex/iban")),
            ("pruefziffer_bei", Zuordnung::Funktion("Regel: regex/iban")),
            ("klassen", Zuordnung::Funktion("Regel: eric_rc/klassen")),
            ("sonstige_klasse", Zuordnung::Funktion("Regel: eric_rc/klassen")),
            ("keine_bank", Zuordnung::Funktion("Regel: bankverbindung")),
            ("kontoinhaber_bei", Zuordnung::Funktion("Regel: bankverbindung")),
            ("datenarten", Zuordnung::Funktion("Regel: xsd_verify/datenart")),
            ("ja_typen", Zuordnung::Funktion("Regel: xsd_verify/ja_typ_pattern")),
            ("laengen_grenze", Zuordnung::Funktion("Regel: regex/iban")),
            ("hole", Zuordnung::Ausnahme("Vergleich gegen die Fixture")),
            ("vergleiche", Zuordnung::Ausnahme("Vergleich gegen die Fixture")),
            ("proben", Zuordnung::Ausnahme("Vergleich gegen die Fixture")),
            ("wert", Zuordnung::Ausnahme("Vergleich gegen die Fixture")),
            ("regal_fehler", Zuordnung::Ausnahme("Bildung des Guards")),
            ("gemeldete_schluessel", Zuordnung::Ausnahme("Bildung des Guards")),
            ("top_level_funktionen", Zuordnung::Funktion("Bildung des Regals")),
            ("traeger", Zuordnung::Funktion("Bildung des Regals")),
        ],
        vollstaendig: true,
    },];

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

/// Die Suchwoerter werden zusammengeschrieben: sonst faende `regal.rs` sein eigenes Suchmuster
/// und zaehlte sich selbst als Tabelle.
const KÜRZEL: [&str; 4] = ["co", "n", "fu", "ti"];
const WORTER: [&str; 3] = ["st", "n", "ti"];

fn wort_const() -> String {
    format!("{}{}t ", KÜRZEL[0], WORTER[1])
}

fn wort_static() -> String {
    format!("{}{}c ", WORTER[0], KÜRZEL[3])
}

/// Name des Eintrags in einer Zeile, die mit `wort` beginnt (nach `pub`, `pub(crate)`). `i` zeigt
/// auf den Anfang des Wortes; die Zeile davor muss leer sein — eine eingerueckte Zeile gehoert
/// einem Funktionskoerper oder einem Testmodul und ist kein Top-Level-Eintrag.
fn oberster_name<'a>(text: &'a str, i: usize, wort: &str) -> Option<&'a str> {
    let anfang = text[..i].rfind('\n').map_or(0, |j| j + 1);
    let vor = text[anfang..i].trim_start();
    let vor = vor.strip_prefix("pub(").unwrap_or(vor);
    let vor = if vor.contains(')') {
        vor.splitn(2, ')').nth(1).unwrap_or("").trim_start()
    } else {
        vor
    };
    let vor = vor.strip_prefix("pub ").unwrap_or(vor);
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
    for wort in [wort_const(), wort_static()] {
        let mut i = 0;
        while let Some(j) = text[i..].find(wort.as_str()) {
            let p = i + j;
            if let Some(name) = oberster_name(text, p, wort.as_str()) {
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
    let wort = format!("{}{} ", KÜRZEL[2], WORTER[2]);
    let mut i = 0;
    while let Some(j) = text[i..].find(wort.as_str()) {
        let p = i + j;
        if let Some(name) = oberster_name(text, p, wort.as_str()) {
            if !name.is_empty() {
                aus.push(name);
            }
        }
        i = p + wort.len();
    }
    aus.sort_unstable();
    aus.dedup();
    aus
}

/// Traegt `name` in der Datei etwas: Konstante, Statik-Variable oder Funktion?
pub(crate) fn traeger(name: &str, text: &str) -> bool {
    top_level_namen(text).iter().any(|n| *n == name)
        || top_level_funktionen(text).iter().any(|n| *n == name)
        || text.contains(&format!("fn {name}("))
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
    let start = text[anfang..]
        .find(offnen)
        .ok_or_else(|| format!("{offnen} fehlt in der Ankerzeile {anker:?}"))?;
    let mut tiefe = 0usize;
    let mut i = anfang + start;
    while i < text.len() {
        if text[i..].starts_with(offnen) {
            tiefe += 1;
            i += offnen.len();
            continue;
        }
        if text.as_bytes()[i] == b'\\' && i + 1 < text.len() {
            i += 2;
            continue;
        }
        if text[i..].starts_with(schliessen) {
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

/// Alle `"..."`-Literale eines Textstuecks, in Reihenfolge des Lesens.
pub(crate) fn wortliche(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut aus = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'"' {
            i += 1;
            continue;
        }
        i += 1;
        let mut innen = String::new();
        while i < b.len() && b[i] != b'"' {
            if b[i] == b'\\' && i + 1 < b.len() {
                i += 1;
                innen.push(match b[i] {
                    b'n' => '\n',
                    b't' => '\t',
                    b'r' => '\r',
                    other => other as char,
                });
            } else {
                innen.push(b[i] as char);
            }
            i += 1;
        }
        i += 1;
        aus.push(innen);
    }
    aus
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
    assert!(w.len() % 2 == 0, "unaarige Literale in {text:?}");
    let mut m = serde_json::Map::new();
    for paar in w.chunks(2) {
        m.insert(paar[0].clone(), Value::String(paar[1].clone()));
    }
    Value::Object(m)
}

/// Die ersten `n` Literale eines Textstuecks, als Menge.
pub(crate) fn erste_menge(text: &str, n: usize) -> Value {
    let mut liste: Vec<String> = wortliche(text).into_iter().take(n).collect();
    liste.sort_unstable();
    Value::Array(liste.into_iter().map(Value::from).collect())
}

/// Der Textwert einer Regel, die ueber mehrere Zeilen laufen kann (Rust haelt `--\\` am
/// Zeilenende, Python setzt seine Stuecke in Klammern zusammen): die konkatenzierten Literale
/// ab der `n`-ten Zeile mit `anker`, bis eine Zeile das Literal mit `",` schliesst.
pub(crate) fn text_der(datei: &str, anker: &str, n: usize) -> String {
    let text = datei_von(datei)
        .unwrap_or_else(|| panic!("Datei {datei} fehlt im Regal"))
        .text;
    let mut fund = 0;
    let mut start = None;
    for (i, zeile) in text.lines().enumerate() {
        if zeile.contains(anker) {
            if fund == n {
                start = Some(i);
                break;
            }
            fund += 1;
        }
    }
    let start = start.unwrap_or_else(|| panic!("Anker {anker:?} ({n}.) nicht in {datei}"));
    let mut wert = String::new();
    let mut offen;
    for zeile in text.lines().skip(start) {
        let rest = zeile.find(anker).map_or(zeile, |i| &zeile[i + anker.len()..]);
        let mut i = 0;
        let bytes = rest.as_bytes();
        while i < bytes.len() {
            if bytes[i] != b'"' {
                i += 1;
                continue;
            }
            let ende = rest[i + 1..]
                .find('"')
                .unwrap_or_else(|| panic!("offenes Literal in {zeile:?}"));
            let innen = &rest[i + 1..i + 1 + ende];
            if !innen.contains('\\') {
                wert.push_str(innen);
            }
            let nach = &rest[i + 2 + ende..];
            offen = !nach.trim_start().starts_with(',') && !nach.trim_start().is_empty();
            i = i + 2 + ende;
            if !offen {
                return wert;
            }
        }
    }
    panic!("kein Literal-Ende nach {anker:?} in {datei}");
}

/// Zeilennummer des Ankers in der Regal-Datei (1-basiert, wie der Compiler).
pub(crate) fn zeile_der(datei: &str, anker: &str) -> Option<usize> {
    let text = datei_von(datei)?.text;
    let i = text.find(anker)?;
    Some(text[..i].lines().count() + 1)
}

/// Das `n`-te Literal eines Textstuecks (fuer Einzelwerte, z. B. einen Namespace).
pub(crate) fn einzel(text: &str, n: usize) -> String {
    wortliche(text)
        .into_iter()
        .nth(n)
        .unwrap_or_else(|| panic!("kein Literal {n} in {text:?}"))
}

/// Der `zeile`-Knoten der Fixture gegen den Rust-Quelltext: dieselbe Datei, derselbe Anker,
/// dieselbe Zahl. Ein Anker, den der Generator nicht mehr findet, ist ein Fehler, kein Skip.
pub(crate) fn ort_vergleich(pfad: &str, fix: &Value) -> Vec<String> {
    let Some(ort) = hole(fix, pfad) else {
        return vec![format!("{pfad}: fehlt in der Fixture")];
    };
    let (Some(datei), Some(anker), Some(zeile)) = (
        ort.get("datei").and_then(Value::as_str),
        ort.get("anker").and_then(Value::as_str),
        ort.get("zeile").and_then(Value::as_i64),
    ) else {
        return vec![format!("{pfad}: unvollstaendiger Orts-Knoten {ort}")];
    };
    match datei_von(datei) {
        None => vec![format!("{pfad}: Datei {datei} fehlt im Regal")],
        Some(_) => match zeile_der(datei, anker) {
            None => vec![format!("{pfad}: Anker {anker:?} nicht in {datei}")],
            Some(rust) if rust as i64 != zeile => {
                vec![format!("{pfad}: Zeile Rust {rust}, Fixture {zeile}")]
            }
            Some(_) => Vec::new(),
        },
    }
}

/// Ein String-Wert fuer den Vergleich.
#[must_use]
pub fn wert(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// Eine Fixture-Referenz; `None` bei fehlendem Pfad. Pfade getrennt durch `/`.
pub(crate) fn hole<'a>(fix: &'a Value, pfad: &str) -> Option<&'a Value> {
    let mut aktuell = fix;
    for teil in pfad.split('/') {
        aktuell = aktuell.get(&*teil)?;
    }
    Some(aktuell)
}

/// Eintrag-fuer-Eintrag-Vergleich eines Rust-Wertes gegen die Fixture.
pub(crate) fn vergleiche(pfad: &str, fix: &Value, rust: Value) -> Vec<String> {
    match hole(fix, pfad) {
        None => vec![format!("{pfad}: fehlt in der Fixture")],
        Some(f) if *f == rust => Vec::new(),
        Some(f) => vec![format!("{pfad}: Rust {rust}, Fixture {f}")],
    }
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
        for (e, zuordnung) in datei.eintraege {
            let traegt = match zuordnung {
                Zuordnung::Funktion(_) => top_level_funktionen(datei.text)
                    .iter()
                    .any(|n| *n == *e),
                _ => namen.iter().any(|n| n == e),
            };
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

/// Die Schluessel, die das Regal der Fixture zuordnet (ohne `Regel` und `Ausnahme`).
/// Die Schluessel des Regals, die die Fixture haben muss.
pub(crate) fn gemeldete_schluessel() -> Vec<&'static str> {
    let mut aus: Vec<&'static str> = REGAL
        .iter()
        .flat_map(|d| d.eintraege.iter())
        .filter_map(|(_, z)| match z {
            Zuordnung::Schluessel(s) => Some(*s),
            _ => None,
        })
        .collect();
    aus.sort_unstable();
    aus.dedup();
    aus
}
