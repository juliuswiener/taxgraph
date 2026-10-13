//! Eigenschaften der Deklaration ueber die ECHTE Bindung (`rust/bindung/daten/*.yaml`), ohne
//! Python: Round-Trip, fail-closed, Rundung zugunsten der Steuerpflichtigen, Instanz-Meet.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Cent, Feldtyp, Herkunft, Kz, PruefTiefe, Vz, Zustand};
use elster::testhilfe::schemas_da;
use elster::{
    cent_nach_kz, deklariere, erzeuge_xml, kz_format, zuruecklesen, Felder, KzFormat, XmlOptionen,
};
use proptest::prelude::*;
use serde_json::{json, Value};
use store::SnapshotFeld;

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        // Die Bindung des Dienstes (`rust/bindung/daten`): ein Kz eines Rust-Felds gehoert in dieselbe
        // Pruefung gegen das XSD wie jedes andere (Weg B voll, 2026-10-05).
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Bindung laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

fn laie_herkunft() -> Herkunft {
    let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
    Herkunft {
        herkunft: a("laie"),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: a("nutzer"),
    }
}

fn feld(wert: Value, zustand: Zustand) -> SnapshotFeld {
    SnapshotFeld {
        wert: wert.into(),
        zustand,
        herkunft: laie_herkunft().into(),
    }
}

fn einzeln(feld_id: &str, wert: Value, zustand: Zustand) -> Felder {
    Felder::from([(feld_id.to_owned(), feld(wert, zustand))])
}

fn beispiel(b: &Bindung) -> Value {
    match b.typ {
        Feldtyp::Cent => json!(12_345),
        Feldtyp::Int => json!(3),
        Feldtyp::Bool => json!(true),
        Feldtyp::Enum => json!(b
            .enum_werte
            .as_ref()
            .and_then(|w| w.first())
            .cloned()
            .unwrap_or_default()),
        Feldtyp::Datum => json!("05.05.1990"),
        Feldtyp::Text => b.beispielwert.clone(),
    }
}

/// fail-closed: ein einziges VORLAEUFIGES Feld macht die Deklaration unvollstaendig und traegt
/// nichts ausser der Konstanten E0100001 in die Deklaration ein — fuer JEDE Bindung.
#[test]
fn vorlaeufig_deklariert_nie() {
    for b in bindungen() {
        let d = deklariere(
            &einzeln(&b.feld_id, beispiel(b), Zustand::Vorlaeufig),
            index(),
            2025,
            None,
        )
        .unwrap();
        assert!(!d.eingaben_konsistent(), "{}", b.feld_id);
        assert_eq!(
            d.deklaration.keys().collect::<Vec<_>>(),
            ["E0100001"],
            "{}",
            b.feld_id
        );
        assert!(
            d.person_b.is_empty() && d.anlage_instanzen.is_empty() && d.dokumentiert.is_empty(),
            "{}",
            b.feld_id
        );
    }
}

/// fail-closed: ein Zeichen ausserhalb der XML-1.0-Char-Produktion (NUL aus kopiertem Text)
/// landet nicht roh im XML, ELSTER wiese sonst die ganze Abgabe ab (Ticket
/// elster-xml-steuerzeichen-im-textwert, `fuzz/regressions/elster/xml-nul-im-textwert.bin`).
#[test]
fn steuerzeichen_im_textwert_ist_harter_fehler() {
    if !schemas_da(2025) {
        return;
    }
    let d = deklariere(
        &einzeln(
            "stammdaten_nachname",
            json!("Maier\u{0}"),
            Zustand::Bestaetigt,
        ),
        index(),
        2025,
        None,
    )
    .unwrap();
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        ..XmlOptionen::default()
    };
    let fehler = erzeuge_xml(&d, &opt).unwrap_err();
    assert!(fehler.0.contains("Steuerzeichen"), "{fehler}");
    // nennt die Kz, nie den Wert (PII) und nie das Zeichen selbst
    assert!(fehler.0.contains("E0100201"), "{fehler}");
    assert!(
        !fehler.0.contains("Maier") && !fehler.0.contains('\u{0}'),
        "{fehler}"
    );
    // jede Kz, nicht nur typ=text: der Fuzz-Fund war das bool-Feld hinter E0161806 mit NUL
    let d = deklariere(
        &einzeln(
            "fahrtkosten_pausch_ag_bl_tbl_h",
            json!("\u{0}"),
            Zustand::Bestaetigt,
        ),
        index(),
        2025,
        None,
    )
    .unwrap();
    let fehler = erzeuge_xml(&d, &opt).unwrap_err();
    assert!(
        fehler
            .0
            .contains("Element E0161806 enthält ein Steuerzeichen"),
        "{fehler}"
    );
}

/// Der Wert eines `xs:pattern` im benannten `complexType` des Schemas (roxmltree loest
/// `&#xa;` und `&#x20;` zu den Zeichen auf).
fn xsd_muster(schema: &roxmltree::Document<'_>, typ: &str) -> String {
    schema
        .descendants()
        .find(|n| n.tag_name().name() == "complexType" && n.attribute("name") == Some(typ))
        .and_then(|ct| ct.descendants().find(|n| n.tag_name().name() == "pattern"))
        .and_then(|p| p.attribute("value"))
        .unwrap_or_else(|| panic!("{typ} fehlt im Schema"))
        .to_owned()
}

/// Ticket elster-zeichensatz-strenger-als-xml: die Menge im Code ist die des Schemas.
/// `StringZUBaseCType` (E10-2025.xsd:1810) ohne die Zeilenumbrueche, die `StringBaseCType`
/// (:1792) verbietet. Geprueft wird JEDES Zeichen der Ebenen 0 bis 2 (U+0000..U+2FFFF) und
/// U+E000..U+FFFF; eine Abschrift mit einem Tippfehler wird rot. Python: `test_menge_gleicht_dem_xsd`.
#[test]
fn zeichensatz_gleicht_dem_xsd() {
    if !schemas_da(2025) {
        return;
    }
    let pfad = elster::finde_schema(2025, "E10-{jahr}.xsd").unwrap();
    let text = std::fs::read_to_string(pfad).unwrap();
    let schema = roxmltree::Document::parse(&text).unwrap();
    let zu = xsd_muster(&schema, "StringZUBaseCType");
    let base = xsd_muster(&schema, "StringBaseCType");
    assert_eq!(
        base, "[^\n\r]+",
        "StringBaseCType (:1792) verbietet nur CR/LF"
    );
    let zu = regex::Regex::new(&format!("^(?:{zu})$")).unwrap();
    let base = regex::Regex::new(&format!("^(?:{base})$")).unwrap();
    let mut falsch = Vec::new();
    let mut erlaubt = 0;
    for cp in (0..0x30000u32).chain(0xE000..0x10000) {
        let Some(c) = char::from_u32(cp) else {
            continue;
        };
        let s = c.to_string();
        let im_schema = zu.is_match(&s) && base.is_match(&s);
        erlaubt += usize::from(im_schema);
        if domain::zeichensatz::elster_zeichen(c) != im_schema {
            falsch.push(format!("U+{cp:04X}"));
        }
    }
    assert!(falsch.is_empty(), "Menge weicht vom Schema ab: {falsch:?}");
    // Gegenprobe, dass der Vergleich etwas gesehen hat: 186 Zeichen, nicht 0 von 0.
    assert_eq!(erlaubt, 186);
}

/// AK5: die zweite Sperre an der XML-Erzeugung. Ein Wert, den ein Alt-Store oder ein Import ohne
/// Auflage Z annahm, wird deklariert (Laden prueft nie) und scheitert erst am Writer — nie erst bei
/// ELSTER. Die Meldung nennt Element, Zeichen und Vorschlag, nie den Wert (PII).
#[test]
fn zeichen_ausserhalb_des_zeichensatzes_ist_harter_fehler() {
    if !schemas_da(2025) {
        return;
    }
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        ..XmlOptionen::default()
    };
    for (wert, zeichen, vorschlag) in [
        (
            "Kowalski\u{a0}Anna",
            "geschütztes Leerzeichen (U+00A0)",
            "ein normales Leerzeichen",
        ),
        ("Müller\u{2013}Straße", "„\u{2013}\" (U+2013)", "„-\""),
        ("Wa\u{142}esa", "„\u{142}\" (U+0142)", "„l\""),
        (
            "Maier\tMüller",
            "Tabulator (U+0009)",
            "ein normales Leerzeichen",
        ),
        (
            "Maier\nMüller",
            "Zeilenumbruch (U+000A)",
            "ein normales Leerzeichen",
        ),
    ] {
        let d = deklariere(
            &einzeln("stammdaten_nachname", json!(wert), Zustand::Bestaetigt),
            index(),
            2025,
            None,
        )
        .unwrap();
        assert_eq!(
            d.deklaration["E0100201"],
            json!(wert),
            "{wert:?}: Laden prueft nie"
        );
        let meldung = erzeuge_xml(&d, &opt).unwrap_err().0;
        assert!(
            meldung.contains("ELSTER in Textfeldern nicht annimmt")
                && meldung.contains("Element E0100201")
                && meldung.contains(zeichen)
                && meldung.contains(vorschlag),
            "{wert:?}: {meldung}"
        );
        for teil in ["Müller", "Maier", "Kowalski", "esa"] {
            assert!(
                !meldung.contains(teil),
                "{wert:?}: die Meldung nennt den Wert: {meldung}"
            );
        }
    }
}

/// Gegenprobe zu AK5: Umlaute, ß, € und Œ gehen durch die zweite Sperre und stehen im XML.
#[test]
fn erlaubte_zeichen_kommen_ins_xml() {
    if !schemas_da(2025) {
        return;
    }
    let mut paare = seitengate();
    paare[0] = ("stammdaten_nachname", json!("Müller-Größe ß € Œuvre"));
    paare.push(("stammdaten_geburtsdatum", json!("05.05.1955")));
    paare.push(("kist_konfession", json!("keine")));
    let xml = abgabe_xml(&bestaetigt(&paare)).unwrap();
    assert!(
        xml.contains("<E0100201>Müller-Größe ß € Œuvre</E0100201>"),
        "{xml}"
    );
}

/// Round-Trip fuer JEDE 1:1-Bindung (eigener `elster_kz`, keine Instanz): steht die Kz nach der
/// Deklaration mit dem Rohwert in der Deklaration, liest `zuruecklesen` exakt den Rohwert zurueck.
/// Ausnahmen muessen hier namentlich stehen.
#[test]
fn round_trip_eins_zu_eins() {
    let mut geprueft = 0;
    for b in bindungen()
        .iter()
        .filter(|b| b.elster_kz.is_some() && b.instanz_gruppe.is_none())
    {
        let kz = b.elster_kz.as_ref().unwrap().as_str();
        let wert = beispiel(b);
        let d = deklariere(
            &einzeln(&b.feld_id, wert.clone(), Zustand::Bestaetigt),
            index(),
            2025,
            None,
        )
        .unwrap();
        let Some(deklariert) = d.deklaration.get(kz) else {
            continue;
        };
        let zurueck = zuruecklesen(&d, index());
        let erwartet = if b.typ == Feldtyp::Cent {
            deklariert.clone()
        } else {
            wert.clone()
        };
        // Wertekodierung (Religionsschluessel) ist bewusst nicht umkehrbar: "02" statt "evangelisch".
        if b.feld_id.starts_with("kist_konfession") {
            continue;
        }
        assert_eq!(
            zurueck.felder.get(&b.feld_id),
            Some(&erwartet),
            "{} ({kz})",
            b.feld_id
        );
        geprueft += 1;
    }
    assert!(geprueft > 100, "nur {geprueft} 1:1-Bindungen geprueft");
}

proptest! {
    /// Rundung zugunsten der Steuerpflichtigen ueber JEDE cent-Bindung mit Kz: Einnahme nie
    /// hoeher, Abzug nie niedriger als der Cent-Betrag, Dezimal-Kz exakt.
    #[test]
    fn rundung_zugunsten_je_bindung(c in 0_i64..10_000_000, i in 0_usize..1000) {
        let cent: Vec<&Bindung> = bindungen().iter()
            .filter(|b| b.typ == Feldtyp::Cent && b.elster_kz.is_some() && b.instanz_gruppe.is_none())
            .collect();
        let b = cent[i % cent.len()];
        let kz_typ = b.elster_kz.as_ref().unwrap();
        let kz = kz_typ.as_str();
        let d = deklariere(&einzeln(&b.feld_id, json!(c), Zustand::Bestaetigt), index(), 2025, None).unwrap();
        // Fehlt die Kz ganz, ist das ein Fehler: kein stilles Ueberspringen der Pruefung.
        let Some(v) = d.deklaration.get(kz).or_else(|| d.person_b.get(kz)) else {
            return Err(TestCaseError::fail(format!("{kz} ({}): fehlt in der Deklaration, c = {c}", b.feld_id)));
        };
        prop_assert_eq!(v, &cent_nach_kz(Cent::new(c), kz_typ).als_json());
        match (kz_format(kz_typ), v.as_i64()) {
            (KzFormat::EuroAbgerundet, Some(e)) => prop_assert!(e * 100 <= c),
            (KzFormat::EuroAufgerundet, Some(e)) => prop_assert!(e * 100 >= c),
            (KzFormat::KommaCent, None) => prop_assert_eq!(v, &json!(format!("{},{:02}", c / 100, c % 100))),
            (f, _) => prop_assert!(false, "{kz}: Format {f:?} passt nicht zu {v}"),
        }
    }

    /// Instanz-Meet: eine Instanz ist genau dann bestaetigt, wenn ALLE ihre Felder es sind.
    #[test]
    fn instanz_meet(zustaende in prop::collection::vec(any::<bool>(), 1..6), idx in 2_u64..5) {
        let gruppe: Vec<&Bindung> = bindungen().iter().filter(|b| b.instanz_gruppe.as_deref() == Some("vv_objekt")).collect();
        let mut store = store::Store::leer(2025, None);
        let nachschlag = store::BindungNachschlag::neu(index());
        let signal = domain::Signal2::new("ui").unwrap();
        let mut erwartet = Zustand::Bestaetigt;
        for (b, bestaetigt) in gruppe.iter().zip(&zustaende) {
            let neu = store::NeuesEvent {
                feld_id: format!("{}__{idx}", b.feld_id),
                wert: beispiel(b).into(),
                feldzustand: if *bestaetigt { domain::Feldzustand::Bestaetigt { signal_2: signal.clone() } } else { domain::Feldzustand::Vorlaeufig },
                herkunft: laie_herkunft(),
                schreiber: domain::Schreiber::Mensch("t".to_owned()),
                signal_1: None,
                ersetzt: None,
                ts: Some("2026-01-01T00:00:00+00:00".to_owned()),
            };
            if store.append(&neu, None, nachschlag).is_ok() && !*bestaetigt {
                erwartet = Zustand::Vorlaeufig;
            }
        }
        let inst = elster::instanzen(&store, index(), "vv_objekt").unwrap();
        // Erscheint die Instanz nie, ist das ein Fehler: kein stilles Ueberspringen der Pruefung.
        let Some(i) = inst.iter().find(|i| i.index == idx) else {
            return Err(TestCaseError::fail(format!("Instanz vv_objekt__{idx} fehlt, Zustaende {zustaende:?}")));
        };
        prop_assert_eq!(i.zustand, erwartet);
    }
}

/// Verbietet der XSD-Typ der Kz die 0? Typ `GanzzahlPos*` (Basis `xs:positiveInteger`) oder eine
/// enumeration-/pattern-Facette ohne "0" (`GdB`, Pflegegrad).
fn verbietet_null(meta: &HashMap<String, elster::KzMeta>, kz: &str) -> bool {
    meta.get(kz).is_some_and(|m| {
        m.type_name.starts_with("GanzzahlPos")
            || (!m.enums.is_empty() && !m.enums.iter().any(|e| e == "0"))
            || (!m.patterns.is_empty()
                && !m.patterns.iter().any(|p| {
                    regex::Regex::new(&format!("^(?:{p})$"))
                        .unwrap()
                        .is_match("0")
                }))
    })
}

/// Die handgepflegten Kz-Mengen gegen die aus dem XSD ABGELEITETEN Typen (Ersatz fuer die
/// `ponytail`-Grenzen in `est_mapping.py:154-158,197-198`). Die Teilmengen-Beziehung ist hart;
/// was das XSD zusaetzlich nahelegt, wird gezaehlt und ausgegeben (offene Punkte, s. Bericht).
#[test]
fn kz_mengen_aus_xsd() {
    if !schemas_da(2025) {
        return;
    }
    let pfad = elster::finde_schema(2025, "E10-{jahr}.xsd").unwrap();
    let meta = elster::kz_meta(&pfad, "E10").unwrap();
    let typ = |kz: &str| meta.get(kz).map_or("", |m| m.type_name.as_str());
    for kz in elster::DATUMS_KZ {
        assert!(typ(kz).starts_with("DatumTTpMMpJJJJ"), "{kz}: {}", typ(kz));
    }
    for kz in elster::KOMMA_OHNE_E60_KZ {
        assert!(
            typ(kz).starts_with("Dezimalzahl") && typ(kz).contains("MinNK2_MaxNK2"),
            "{kz}: {}",
            typ(kz)
        );
    }
    // Die Jahresmengen sind aus dem XSD ABGELEITET und eingefroren. Beide Jahrgaenge werden
    // geprueft: 2025 gegen DIESES Schema, 2024 gegen `E10-2024.xsd` (die Kz, die nur 2024
    // verbietet — E0106603 —, stehen dort und nicht hier).
    for kz in elster::null_unzulaessig(2025).unwrap() {
        assert!(verbietet_null(&meta, kz), "{kz}: {}", typ(kz));
    }
    if schemas_da(2024) {
        let pfad24 = elster::finde_schema(2024, "E10-{jahr}.xsd").unwrap();
        let meta24 = elster::kz_meta(&pfad24, "E10").unwrap();
        for kz in elster::null_unzulaessig(2024).unwrap() {
            assert!(verbietet_null(&meta24, kz), "2024 {kz}: {}", typ(kz));
        }
    }
    let cent_kz: Vec<&str> = bindungen()
        .iter()
        .filter(|b| b.typ == Feldtyp::Cent)
        .filter_map(|b| b.elster_kz.as_ref().map(Kz::as_str))
        .collect();
    let komma_luecke: Vec<&str> = cent_kz
        .iter()
        .copied()
        .filter(|kz| {
            typ(kz).starts_with("Dezimalzahl")
                && kz_format(&Kz::new(*kz).unwrap()) != KzFormat::KommaCent
        })
        .collect();
    let null_luecke: Vec<&str> = cent_kz
        .iter()
        .copied()
        .filter(|kz| {
            typ(kz).starts_with("GanzzahlPos")
                && !elster::null_unzulaessig(2025).unwrap().contains(kz)
        })
        .collect();
    println!(
        "[xsd-abgeleitet] cent-Kz={} Dezimal-Typ ohne Komma-Format={komma_luecke:?}",
        cent_kz.len()
    );
    println!(
        "[xsd-abgeleitet] GanzzahlPos-Typ ohne 0-Sperre={} {null_luecke:?}",
        null_luecke.len()
    );
}

/// P9 (Vault-Ticket `elster-xml-null-in-ganzzahlpos-kz`): eine 0 in einer Kz, deren XSD-Typ sie
/// verbietet, macht das ganze XML schema-ungueltig (2026-10-01: 214 von 378 XML aus echten
/// Faellen). Die Kz-Menge kommt aus dem XSD, nicht aus `null_unzulaessig`: Typ `GanzzahlPos*`
/// (Basis `xs:positiveInteger`) oder eine enumeration-/pattern-Facette ohne "0" (`GdB`, Pflegegrad).
/// Geprueft wird jede Kz in allen drei Buckets, nicht nur die Kz des Feldes.
///
/// Je Veranlagungsjahr (2024 und 2025): die Jahresmenge `null_unzulaessig(jahr)` ist je Jahr eingefroren (`E0106603` verbietet die 0
/// nur 2024), und eine fehlende Kz in der EINEN Jahresmenge blieb ohne den 2024er Lauf unbemerkt (Gegenversuch Nr. 69 `N1`;
/// Python: `test_keine_null_in_einem_kz_das_sie_im_jahr_verbietet[2024]`).
fn null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet_im(jahr: i64) {
    if !schemas_da(jahr) {
        return;
    }
    let pfad = elster::finde_schema(jahr, "E10-{jahr}.xsd").unwrap();
    let meta = elster::kz_meta(&pfad, "E10").unwrap();
    // Jeder Schreibweg ueber die Bindung: 1:1, Instanz (`__2`), Person B (Klasse g); 50 Cent werden
    // auf 0 Euro abgerundet. Die Art-Verzweigung (Klasse f) haengt an `pub(crate)`-Tabellen, sie
    // prueft `art_verzweigung_schreibt_keine_verbotene_null` (`src/deklaration.rs`).
    // Der dritte Wert ist ein echter Betrag: er zeigt, welche verbietenden Kz der Lauf ueberhaupt mit einem Wert gesehen hat.
    let ergebnis: Vec<(Vec<String>, Vec<String>)> = bindungen()
        .iter()
        .filter(|b| matches!(b.typ, Feldtyp::Cent | Feldtyp::Int))
        .flat_map(|b| {
            let werte: &'static [i64] = if b.typ == Feldtyp::Cent {
                &[0, 50, 100_000]
            } else {
                &[0, 3]
            };
            let instanz = b
                .instanz_gruppe
                .as_ref()
                .map(|_| format!("{}__2", b.feld_id));
            std::iter::once(b.feld_id.clone())
                .chain(instanz)
                .flat_map(move |s| werte.iter().map(move |&w| (s.clone(), w)))
        })
        .map(|(schluessel, wert)| {
            let d = deklariere(
                &einzeln(&schluessel, json!(wert), Zustand::Bestaetigt),
                index(),
                jahr,
                None,
            )
            .unwrap();
            let instanzen = d.anlage_instanzen.iter().flat_map(|(_, ii)| ii);
            let alle: Vec<(&String, &Value)> = [&d.deklaration, &d.person_b]
                .into_iter()
                .chain(instanzen.map(|i| &i.felder))
                .flatten()
                .filter(|(kz, _)| verbietet_null(&meta, kz))
                .collect();
            let durch = alle
                .iter()
                .filter(|(_, v)| **v == json!(0))
                .map(|(kz, _)| format!("{kz} {schluessel}={wert}"))
                .collect();
            let gesehen = alle
                .iter()
                .filter(|(_, v)| **v != json!(0))
                .map(|(kz, _)| (*kz).clone())
                .collect();
            (durch, gesehen)
        })
        .collect();
    let durch: Vec<&String> = ergebnis.iter().flat_map(|(d, _)| d).collect();
    let gesehen: std::collections::BTreeSet<&String> = ergebnis.iter().flat_map(|(_, g)| g).collect();
    println!("null_verbot {jahr}: {} verbietende Kz mit einem Wert gesehen", gesehen.len());
    // KONTROLLE: der Lauf hat verbietende Kz mit echten Werten gesehen (gemessen 2026-10-13: 2025: 35, 2024: 35). Ohne sie
    // waere `durch.is_empty()` gruen, weil der Lauf nichts las.
    assert!(
        gesehen.len() >= 30,
        "{jahr}: nur {} verbietende Kz der Bindung kamen mit einem Wert in der Deklaration an",
        gesehen.len()
    );
    assert!(
        durch.is_empty(),
        "{jahr}: 0 in {} Faellen durchgelassen, obwohl der XSD-Typ sie verbietet: {durch:?}",
        durch.len()
    );
}

#[test]
fn null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet() {
    null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet_im(2025);
}

#[test]
fn null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet_auch_2024() {
    null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet_im(2024);
}

/// Der Belegfall der Jahresspanne: `E0106603` (Anzahl weiterer Pflegepersonen) traegt 2024 `GanzzahlPosOhneFuehrNull` (die 0 ist
/// verboten) und 2025 `GanzzahlNichtNegOhneFuehrNull` (die 0 ist erlaubt, das XSD-Label verlangt sie). Die Bindung fuehrt das Feld
/// ueber beide Jahre. Ohne diesen Test waere die Jahresdrift nur eine Zahl aus einem Lauf, die beim naechsten Schema-Update still
/// verschwindet. Python: `test_e0106603_ist_der_gemessene_fall`.
#[test]
fn e0106603_verbietet_die_null_2024_und_erlaubt_sie_2025() {
    if !(schemas_da(2024) && schemas_da(2025)) {
        return;
    }
    let meta = |jahr| {
        let pfad = elster::finde_schema(jahr, "E10-{jahr}.xsd").unwrap();
        elster::kz_meta(&pfad, "E10").unwrap()
    };
    let (m24, m25) = (meta(2024), meta(2025));
    assert!(
        m24["E0106603"].type_name.starts_with("GanzzahlPosOhneFuehrNull"),
        "{}",
        m24["E0106603"].type_name
    );
    assert!(
        m25["E0106603"].type_name.starts_with("GanzzahlNichtNegOhneFuehrNull"),
        "{}",
        m25["E0106603"].type_name
    );
    assert!(verbietet_null(&m24, "E0106603"), "2024 muss die 0 verbieten");
    assert!(!verbietet_null(&m25, "E0106603"), "2025 muss die 0 erlauben");
    // Und die Bindung faehrt das Feld ueber BEIDE Jahre -- genau daraus entsteht die Luecke.
    let b = index()["rentner_pflege_weitere_personen"];
    assert_eq!(b.elster_kz.as_ref().map(Kz::as_str), Some("E0106603"));
    assert!(b.vz_gueltigkeit.contains(&2024) && b.vz_gueltigkeit.contains(&2025), "{:?}", b.vz_gueltigkeit);
}

/// Dieselbe Spanne an den Listen des Produkts, ohne Schema (laeuft auch in der CI): `E0106603` steht NUR in der 2024er Liste.
/// Der Test oben prueft Schematyp und Bindung; kein Eingabeweg schreibt die 0 dorthin (Gate), also faengt nur dieser Test eine Liste,
/// die das Jahr vertauscht. Python: `test_e0106603_ist_der_gemessene_fall`.
#[test]
fn e0106603_steht_nur_in_der_null_verbots_liste_2024() {
    assert!(elster::null_unzulaessig(2024).unwrap().contains(&"E0106603"));
    assert!(!elster::null_unzulaessig(2025).unwrap().contains(&"E0106603"));
}

/// P9 (Vault: `decisions/elster-null-in-kz-ohne-null-weglassen`, Punkt 3): eine weggelassene 0
/// ist kein verlorener Wert, sondern „nichts anzugeben" — kein `nicht_deklariert`-Eintrag. Der
/// Spenden-Zweig schrieb bis P9 einen; mit Eintrag meldete die Pruefanzeige „nicht alle Werte".
#[test]
fn spende_null_ohne_nicht_deklariert() {
    let d = deklariere(
        &einzeln("spenden_betrag", json!(0), Zustand::Bestaetigt),
        index(),
        2025,
        None,
    )
    .unwrap();
    assert!(!d.deklaration.contains_key("E0108105"));
    assert!(d.nicht_deklariert.is_empty(), "{:?}", d.nicht_deklariert);
}
// ------------------------------------------------- leere Huelle (Ankreuzfeld "Nein")

/// Elemente im E10-Teil, die keinen Inhalt tragen ausser dem Person-Diskriminator — genau die
/// Klasse, die checkESt mit "Der Kontext ... ist leer" beanstandet (gemessen 2026-10-01,
/// ERiC 44.2.4.0). Strukturell, nicht ueber ERiC: ohne echte Hersteller-ID liefert checkESt
/// rc=610301200 mit LEEREM Fehlerpuffer und saehe damit "fehlerfrei" aus.
fn leere_huellen(xml: &str) -> Vec<String> {
    let mut raus = Vec::new();
    let mut tiefe = 0usize;
    let mut im_e10 = false;
    let mut stapel: Vec<(String, bool, bool)> = Vec::new(); // (Name, hat Kind, hat Text)
    for zeile in xml.lines() {
        let z = zeile.trim();
        if z.starts_with("<E10 ") || z.starts_with("<E10>") {
            im_e10 = true;
            tiefe = 1;
            continue;
        }
        if !im_e10 {
            continue;
        }
        if z.starts_with("</E10>") {
            break;
        }
        // Selbstschliessend: kein Kind, kein Text.
        if let Some(name) = z
            .strip_prefix('<')
            .and_then(|r| r.split([' ', '/', '>']).next())
        {
            if z.ends_with("/>") {
                if let Some((pname, _, _)) = stapel.last_mut() {
                    let _ = pname;
                }
                raus.push(name.to_owned());
                continue;
            }
            if let Some(rest) = z.strip_prefix(&format!("<{name}>")) {
                // Eroeffnendes Tag mit Text auf derselben Zeile: kein leeres Element.
                let text = rest.strip_suffix(&format!("</{name}>")).unwrap_or(rest);
                if z.contains(&format!("</{name}>")) && !text.trim().is_empty() {
                    continue;
                }
                stapel.push((name.to_owned(), false, false));
                tiefe += 1;
                continue;
            }
            // Eroeffnendes Tag mit Kindern (Rest der Zeile ist leer).
            if z.ends_with('>') {
                stapel.push((name.to_owned(), false, false));
                tiefe += 1;
                continue;
            }
        }
        if let Some(name) = z.strip_prefix("</").and_then(|r| r.strip_suffix('>')) {
            tiefe = tiefe.saturating_sub(1);
            if let Some((kname, _, hat_text)) = stapel.pop() {
                if kname == name && !hat_text {
                    // Nur <Person>-Kinder? Dann Huelle, sonst nur ein leerer Blattname.
                    raus.push(kname);
                }
            }
        }
    }
    let _ = tiefe;
    raus
}

/// Ankreuzfeld "Nein" darf keinen leeren Container hinterlassen.
///
/// Ohne den Fix steht `<Geh_Steh_Blind_Hilfl />` (bzw. mit nur `<Person>`) im XML, und
/// checkESt weist die ganze Abgabe ab. Ticket: elster-leerer-container-neben-ankreuzfeld-nein.
#[test]
fn ankreuzfeld_nein_hinterlaesst_keine_leere_huelle() {
    if !schemas_da(2025) {
        return;
    }
    let f = einzeln(
        "rentner_hilflos_blind_taubblind",
        json!(false),
        Zustand::Bestaetigt,
    );
    let d = deklariere(&f, index(), 2025, None).unwrap();
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        ..XmlOptionen::default()
    };
    let xml = erzeuge_xml(&d, &opt).unwrap();
    let h = leere_huellen(&xml);
    assert!(
        h.is_empty(),
        "leere Huelle(n) im XML: {h:?} — checkESt beanstandet sie mit 'Der Kontext ... ist leer'"
    );
}

/// Gegenprobe: "Ja" fuellt den Container. Das Gate darf hier nicht anschlagen.
#[test]
fn gegenprobe_ja_fuellt_die_huelle() {
    if !schemas_da(2025) {
        return;
    }
    let f = einzeln(
        "rentner_hilflos_blind_taubblind",
        json!(true),
        Zustand::Bestaetigt,
    );
    let d = deklariere(&f, index(), 2025, None).unwrap();
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        ..XmlOptionen::default()
    };
    let xml = erzeuge_xml(&d, &opt).unwrap();
    let h = leere_huellen(&xml);
    assert!(h.is_empty(), "leere Huelle(n) im XML: {h:?}");
    assert!(
        xml.contains("<Geh_Steh_Blind_Hilfl>"),
        "Ja muss den Container fuellen"
    );
}

/// Gegenprobe: wird das Feld gar nicht gestellt, entsteht ueberhaupt kein Container.
#[test]
fn gegenprobe_feld_nicht_gestellt_erzeugt_keine_huelle() {
    if !schemas_da(2025) {
        return;
    }
    let f = einzeln("stammdaten_nachname", json!("Muster"), Zustand::Bestaetigt);
    let d = deklariere(&f, index(), 2025, None).unwrap();
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        ..XmlOptionen::default()
    };
    let xml = erzeuge_xml(&d, &opt).unwrap();
    let h = leere_huellen(&xml);
    assert!(h.is_empty(), "leere Huelle(n) im XML: {h:?}");
}

// ---------------------------------------------------------------- §35a: mehrere Posten je Topf
//
// Haushaltsnahe Aufwendungen (§ 35a EStG) haben drei Toepfe: Minijob, Dienstleistung, Handwerker.
// Zwei Posten in einem Topf ergaben bis 2026-10-01 ein schema-ungueltiges XML: `<HA_35a>` traegt
// `maxOccurs="1"` (E10-2025.xsd:8236), die Posten wiederholen sich ueber `<Einz>` darunter
// (`maxOccurs="99"`, :10048). Ohne Eintrag in `INSTANZ_CONTAINER_TIEFER` fiel der Writer auf
// `kz_pfad[..2]` zurueck — also auf `<HA_35a>` selbst — und wiederholte den ganzen Abschnitt.
// xmllint: „Element HA_35a: This element is not expected". Von mehreren Posten erreichte nur
// einer die Datei. Die Python-Seite prueft dasselbe in tests/test_elster_xml.py; hier steht die
// Rust-Messung direkt, damit die Paritaet nicht zwei gleich falsche Seiten gruen nennt.

fn zaehle_tag(xml: &str, tag: &str) -> usize {
    xml.match_indices(&format!("<{tag}>")).count() + xml.match_indices(&format!("<{tag} ")).count()
}

fn posten_xml(gruppe: &str, art: &str, betrag: &str, werte: &[(i64, i64)]) -> String {
    let felder: Felder = werte
        .iter()
        .enumerate()
        .flat_map(|(i, (a, b))| {
            let suffix = if i == 0 { String::new() } else { format!("__{}", i + 1) };
            [
                (
                    format!("{art}{suffix}"),
                    feld(json!(a.to_string()), Zustand::Bestaetigt),
                ),
                (
                    format!("{betrag}{suffix}"),
                    feld(json!(b), Zustand::Bestaetigt),
                ),
            ]
        })
        .collect();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    // Instanz 1 ist die BASIS-feld_id ohne Suffix und steht in `deklaration`; erst `__2` und
    // hoeher landen in `anlage_instanzen` (`instanz.rs`). Der Ring muss also n-1 sehen.
    assert_eq!(
        d.instanzen_der_gruppe(gruppe).len(),
        werte.len() - 1,
        "{gruppe}: Instanzen jenseits der Basis",
    );
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        ..XmlOptionen::default()
    };
    erzeuge_xml(&d, &opt).unwrap()
}

/// AK1 (Rust-Seite): ein Topf mit ZWEI Posten -> EIN `<HA_35a>` mit ZWEI `<Einz>`.
#[test]
fn hh_top_zwei_posten_ein_ha35a() {
    if !schemas_da(2025) {
        return;
    }
    for (gruppe, art, betrag) in [
        ("hh_minijob", "hh_minijob_art", "hh_minijob_betrag"),
        (
            "hh_dienstleistung",
            "hh_dienstleistung_art",
            "hh_dienstleistung_betrag",
        ),
        (
            "hh_handwerker",
            "hh_handwerker_art",
            "hh_handwerker_betrag",
        ),
    ] {
        let xml = posten_xml(gruppe, art, betrag, &[(1, 120_000), (2, 80_000)]);
        assert_eq!(
            zaehle_tag(&xml, "HA_35a"),
            1,
            "{gruppe}: genau ein <HA_35a> erwartet"
        );
        assert_eq!(
            zaehle_tag(&xml, "Einz"),
            2,
            "{gruppe}: zwei <Einz> erwartet"
        );
    }
}

/// AK2 (Rust-Seite): Gegenprobe — ein Posten bleibt ein `<HA_35a>` mit einem `<Einz>`.
#[test]
fn hh_top_ein_posten_bleibt_unveraendert() {
    if !schemas_da(2025) {
        return;
    }
    for (gruppe, art, betrag) in [
        ("hh_minijob", "hh_minijob_art", "hh_minijob_betrag"),
        (
            "hh_dienstleistung",
            "hh_dienstleistung_art",
            "hh_dienstleistung_betrag",
        ),
        (
            "hh_handwerker",
            "hh_handwerker_art",
            "hh_handwerker_betrag",
        ),
    ] {
        let xml = posten_xml(gruppe, art, betrag, &[(1, 120_000)]);
        assert_eq!(zaehle_tag(&xml, "HA_35a"), 1, "{gruppe}");
        assert_eq!(zaehle_tag(&xml, "Einz"), 1, "{gruppe}");
    }
}

/// AK5 (Rust-Seite): das erzeugte XML haelt das amtliche Schema. Ohne den Fix scheiterte genau
/// das an `<HA_35a>` — der Test war damals rot (Gegenprobe im Bericht).
#[test]
fn hh_top_mehrere_posten_ist_xsd_valide() {
    for vz in [Vz::Vz2024, Vz::Vz2025] {
        let jahr = i64::from(vz.jahr());
        if !schemas_da(jahr) {
            continue;
        }
        let felder: Felder = [
            (
                "hh_minijob_art".to_owned(),
                feld(json!("1"), Zustand::Bestaetigt),
            ),
            (
                "hh_minijob_betrag".to_owned(),
                feld(json!(120_000), Zustand::Bestaetigt),
            ),
            (
                "hh_minijob_art__2".to_owned(),
                feld(json!("2"), Zustand::Bestaetigt),
            ),
            (
                "hh_minijob_betrag__2".to_owned(),
                feld(json!(80_000), Zustand::Bestaetigt),
            ),
        ]
        .into();
        let d = deklariere(&felder, index(), 2025, None).unwrap();
        let opt = XmlOptionen {
            vz: jahr,
            hersteller_id: Some("74931".to_owned()),
            ..XmlOptionen::default()
        };
        let xml = erzeuge_xml(&d, &opt).unwrap();
        let (ok, meldung) = elster::validiere_xsd_text(xml.as_bytes(), vz);
        assert!(ok, "VZ {vz}: {meldung}");
    }
}

/// R2 (Rust-Seite): die § 35a-Summe ist die Summe der GERUNDETEN Posten. 4 × 100,01 € ergeben
/// vier Posten zu 101 und die Summe 404, nicht die aufgerundete Rohsumme 401. Das Summenfeld
/// traegt die Rohsumme, wie der Ring sie einhaengt. Python: `tests/test_aufwand_einzel_kz_rundung.py`.
#[test]
fn hh_summe_ist_die_summe_der_gerundeten_posten() {
    for (gruppe, summe, betrag, posten_kz, summe_kz) in [
        (
            "hh_minijob",
            "hh_minijob_aufwendungen",
            "hh_minijob_betrag",
            "E0104108",
            "E0104109",
        ),
        (
            "hh_dienstleistung",
            "hh_dienstleistungen",
            "hh_dienstleistung_betrag",
            "E0107207",
            "E0107208",
        ),
        (
            "hh_handwerker",
            "hh_handwerker_arbeitskosten",
            "hh_handwerker_betrag",
            "E0111214",
            "E0111215",
        ),
    ] {
        let mut felder: Felder = (1..=4)
            .map(|i| {
                let suffix = if i == 1 {
                    String::new()
                } else {
                    format!("__{i}")
                };
                (
                    format!("{betrag}{suffix}"),
                    feld(json!(10_001), Zustand::Bestaetigt),
                )
            })
            .collect();
        felder.insert(summe.to_owned(), feld(json!(40_004), Zustand::Bestaetigt));
        let d = deklariere(&felder, index(), 2025, None).unwrap();
        let posten: Vec<&Value> = d
            .deklaration
            .get(posten_kz)
            .into_iter()
            .chain(
                d.instanzen_der_gruppe(gruppe)
                    .iter()
                    .filter_map(|i| i.felder.get(posten_kz)),
            )
            .collect();
        assert_eq!(posten, vec![&json!(101); 4], "{gruppe}: Posten");
        assert_eq!(d.deklaration.get(summe_kz), Some(&json!(404)), "{gruppe}");
    }
    // Ohne Posten bleibt die Summe stehen (Bestandswert, `tests/test_p35a_bestandsdaten.py`).
    let felder = einzeln("hh_handwerker_arbeitskosten", json!(300_050), Zustand::Bestaetigt);
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    assert_eq!(d.deklaration.get("E0111215"), Some(&json!(3001)));
}

/// Python: `test_einzel_kz_rundet_wie_seine_summe_auf` (Gegenversuch Nr. 69 `A1`/`A2`). Die vierzehn Einzel-Kz der Aufwandsposten runden
/// wie ihre Summe auf volle Euro AUF: 2000,50 EUR ergeben 2001, nicht 2000. Bei § 35c und Berufsausbildung prueft ERiC Summe gegen
/// Einzelposten EXAKT; ein Betrag, der in der Summe 2001 und im Posten 2000 hiesse, machte die ganze Erklaerung ungueltig.
/// Die Paare sind die Vordruckzeilen (Posten, Summe), von Hand abgeschrieben und nicht aus `ABZUGS_KZ` abgeleitet.
#[test]
fn jedes_einzel_kz_rundet_wie_seine_summe_auf() {
    let paare = [
        ("E0104108", "E0104109"), // 35a Minijob
        ("E0107207", "E0107208"), // 35a Dienstleistungen
        ("E0111214", "E0111215"), // 35a Handwerker
        ("E0108002", "E0108202"), // eigene Berufsausbildung
        ("E0506104", "E0506105"), // Kinderbetreuung
    ]
    .into_iter()
    .chain(
        [
            "E0241001", "E0241101", "E0241201", "E0241301", "E0241302", "E0241401", "E0241501", "E0241601", "E0241701",
        ]
        .map(|k| (k, "E0241901")), // 35c, je Massnahme
    )
    .collect::<Vec<_>>();
    assert_eq!(paare.len(), 14);
    for (einzel, summe) in paare {
        let kz = |s: &str| Kz::new(s).unwrap();
        assert_eq!(
            (cent_nach_kz(Cent::new(200_050), &kz(einzel)).als_json(), cent_nach_kz(Cent::new(200_050), &kz(summe)).als_json()),
            (json!(2001), json!(2001)),
            "{einzel} / {summe}"
        );
    }
}

/// Python: `test_ein_posten_mit_cent_steht_in_summe_und_einzel_gleich`. Ein Posten mit 50 Cent steht in Summe UND Einzelzeile gleich
/// (die Einzelzeile entsteht im Ring aus der Summe, `einzelzeile`; hier tragen beide Felder den Rohbetrag, wie der Ring ihn einhaengt).
#[test]
fn ein_posten_mit_cent_steht_in_summe_und_einzelzeile_gleich() {
    for (name, paare, summe, einzel, euro) in [
        (
            "p35c_heizung",
            vec![
                ("p35c_sanierungsaufwendungen", json!(2_000_050)),
                ("p35c_massnahme_einzelbetrag", json!(2_000_050)),
                ("p35c_massnahme_art", json!("heizung")),
            ],
            "E0241901",
            "E0241501",
            20_001,
        ),
        (
            "berufsausbildung",
            vec![("berufsausbildung_aufwendungen", json!(200_050)), ("berufsausbildung_einzelbetrag", json!(200_050))],
            "E0108202",
            "E0108002",
            2001,
        ),
    ] {
        let d = deklariere(&bestaetigt(&paare), index(), 2025, None).unwrap();
        assert_eq!((d.deklaration.get(summe), d.deklaration.get(einzel)), (Some(&json!(euro)), Some(&json!(euro))), "{name}");
    }
}

// ------------------------------------------- Anlage R: mehrere Renten einer Person
//
// `<R>` traegt `maxOccurs="2"` (ein `<R>` je PERSON, Index 1 = PersonB); die Renten einer Person
// stehen als `<Einz>` (`maxOccurs="99"`) in `<Leibr_gesetzl>`/`<Leibr_priv>`/`<Leibr_sonst>` (je
// `maxOccurs="1"`). Ohne Eintrag `("rente", "Einz")` in `INSTANZ_CONTAINER_TIEFER` legte die zweite
// Rente einer Person ein zweites `<R>` an (PersonB) — ERiC lehnt die Einzelveranlagung ab
// (rc=610001002, gemessen 2026-10-03). Die Python-Seite prueft dasselbe in
// tests/test_elster_xml.py; hier steht die Rust-Messung direkt.

/// Der `<R>`-Baum als Zeilen, in Dokumentreihenfolge: je `<R>` eine Zeile `R <Person>`, je `<Einz>`
/// darunter `<Container>: <Kz>=<Text> …`. Ein leeres `<Einz>` ist eine Zeile ohne Kz und faellt im
/// Vergleich auf. Eine Kz, die in EINEM `<Einz>` zweimal steht, bricht ab.
fn anlagen_r(xml: &str) -> Vec<String> {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let name = |n: &roxmltree::Node<'_, '_>| n.tag_name().name().to_owned();
    let mut zeilen = Vec::new();
    for r in doc
        .descendants()
        .filter(|n| n.is_element() && name(n) == "R")
    {
        let person = r
            .children()
            .find(|c| c.is_element() && name(c) == "Person")
            .and_then(|c| c.text())
            .unwrap_or("?");
        zeilen.push(format!("R {person}"));
        for container in r
            .children()
            .filter(|c| c.is_element() && name(c).starts_with("Leibr"))
        {
            for einz in container.children().filter(roxmltree::Node::is_element) {
                let kz: Vec<(String, &str)> = einz
                    .children()
                    .filter(roxmltree::Node::is_element)
                    .map(|k| (name(&k), k.text().unwrap_or("")))
                    .collect();
                let mut namen: Vec<&String> = kz.iter().map(|(n, _)| n).collect();
                namen.sort();
                namen.dedup();
                assert_eq!(namen.len(), kz.len(), "Kz doppelt im <Einz>: {kz:?}");
                let inhalt: Vec<String> = kz.iter().map(|(n, t)| format!("{n}={t}")).collect();
                zeilen.push(format!("{}: {}", name(&container), inhalt.join(" ")));
            }
        }
    }
    zeilen
}

fn rente_1() -> Vec<(&'static str, Value)> {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(1_800_000)),
        ("rentner_renten_beginn_jahr", json!(2015)),
    ]
}

fn rente_2(art: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("rentner_renten_art__2", json!(art)),
        ("rentner_jahresrente__2", json!(900_000)),
        ("rentner_renten_beginn_jahr__2", json!(2012)),
        ("rentner_alter_bei_rentenbeginn__2", json!(65)),
    ]
}

fn rente_partner() -> Vec<(&'static str, Value)> {
    vec![
        ("kein_sonstige_partner", json!(false)),
        ("rentner_renten_art_partner", json!("gesetzliche_rente")),
        ("rentner_jahresrente_partner", json!(700_000)),
        ("rentner_renten_beginn_jahr_partner", json!(2016)),
    ]
}

fn rente_xml(veranlagung: &str, teile: &[Vec<(&'static str, Value)>]) -> String {
    let mut paare = vec![("veranlagung", json!(veranlagung))];
    paare.extend(teile.iter().flatten().cloned());
    let d = deklariere(&bestaetigt(&paare), index(), 2025, None).unwrap();
    assert!(d.eingaben_konsistent(), "{:?}", d.unvollstaendig());
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        ..XmlOptionen::default()
    };
    erzeuge_xml(&d, &opt).unwrap()
}

const GESETZL_1: &str = "Leibr_gesetzl: E1800301=18000 E1800501=01.01.2015";
const GESETZL_2: &str = "Leibr_gesetzl: E1800301=9000 E1800501=01.01.2012";
const PRIV_2: &str = "Leibr_priv: E1801601=9000 E1801701=01.01.2012";
const GESETZL_B: &str = "Leibr_gesetzl: E1800301=7000 E1800501=01.01.2016";

/// AK1 (Rust-Seite): Einzelveranlagung, zwei gesetzliche Renten -> EIN `<R>`, EIN
/// `<Leibr_gesetzl>`, ZWEI `<Einz>`.
#[test]
fn rente_zwei_gesetzliche_einer_person_ein_r_zwei_einz() {
    if !schemas_da(2025) {
        return;
    }
    let xml = rente_xml("einzel", &[rente_1(), rente_2("gesetzliche_rente")]);
    assert_eq!(anlagen_r(&xml), ["R PersonA", GESETZL_1, GESETZL_2]);
}

/// AK2 (Rust-Seite): gesetzlich + privat -> je EIN `<Einz>` in `<Leibr_gesetzl>` und
/// `<Leibr_priv>`. Zaehlte die Gruppen-Nummer 2 statt des Rangs, stuende vor dem privaten Posten
/// ein leeres `<Einz>` (ERiC: `Kontext '/R[1]/Leibr_priv[1]/Einz[1]' ist leer`).
#[test]
fn rente_gesetzlich_plus_privat_je_ein_einz_ohne_leeres() {
    if !schemas_da(2025) {
        return;
    }
    let xml = rente_xml("einzel", &[rente_1(), rente_2("private_leibrente")]);
    assert_eq!(anlagen_r(&xml), ["R PersonA", GESETZL_1, PRIV_2]);
}

/// AK3 (Rust-Seite): Zusammenveranlagung, zweite Rente von A + Rente von B -> `<R>[1]` (A) mit
/// zwei `<Einz>`, `<R>[2]` (B) mit einem. Person B behaelt ihre eigene Anlage R.
#[test]
fn rente_zusammen_zweite_rente_a_und_rente_b_zwei_r() {
    if !schemas_da(2025) {
        return;
    }
    let xml = rente_xml(
        "zusammen",
        &[rente_1(), rente_2("gesetzliche_rente"), rente_partner()],
    );
    assert_eq!(
        anlagen_r(&xml),
        ["R PersonA", GESETZL_1, GESETZL_2, "R PersonB", GESETZL_B]
    );
}

/// AK8 (Rust-Seite): eine Rente bleibt wie bisher EIN `<R>` mit EINEM `<Einz>`; mit Partner-Rente
/// zwei `<R>`. Gegenprobe, dass der Rang die Einzelfaelle nicht anruehrt.
#[test]
fn rente_einzelne_rente_bleibt_unveraendert() {
    if !schemas_da(2025) {
        return;
    }
    assert_eq!(
        anlagen_r(&rente_xml("einzel", &[rente_1()])),
        ["R PersonA", GESETZL_1]
    );
    assert_eq!(
        anlagen_r(&rente_xml("zusammen", &[rente_1(), rente_partner()])),
        ["R PersonA", GESETZL_1, "R PersonB", GESETZL_B]
    );
}

/// AK3 (Rust-Seite, Schema): das XML mit mehreren Renten haelt das amtliche XSD.
#[test]
fn rente_mehrere_renten_sind_xsd_valide() {
    if !schemas_da(2025) {
        return;
    }
    for (name, veranlagung, teile) in [
        (
            "einzel gesetzlich+gesetzlich",
            "einzel",
            vec![rente_1(), rente_2("gesetzliche_rente")],
        ),
        (
            "einzel gesetzlich+privat",
            "einzel",
            vec![rente_1(), rente_2("private_leibrente")],
        ),
        (
            "zusammen zweite Rente A + Rente B",
            "zusammen",
            vec![rente_1(), rente_2("gesetzliche_rente"), rente_partner()],
        ),
    ] {
        let xml = rente_xml(veranlagung, &teile);
        let (ok, meldung) = elster::validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
        assert!(ok, "{name}: {meldung}");
    }
}

/// Luecke im Instanzindex: Instanz 1 und 3, keine 2. Der Store erlaubt das (`eingaben_konsistent`
/// bleibt wahr, es fehlt nur `__2`). Die Gruppen-Nummer 3 legte VOR dem Posten ein leeres `<Einz>`
/// an (ERiC: "Kontext ... ist leer"); der Rang zaehlt dicht. Das ist die EINZIGE Abweichung vom
/// alten XML der vier bestehenden Gruppen und sie ist gewollt. Python-Gegenstueck:
/// `test_luecke_im_instanzindex_zaehlt_dicht_ohne_leeres_einz`.
#[test]
fn luecke_im_instanzindex_zaehlt_dicht_ohne_leeres_einz() {
    if !schemas_da(2025) {
        return;
    }
    for (gruppe, art, betrag) in [
        ("hh_minijob", "hh_minijob_art", "hh_minijob_betrag"),
        (
            "hh_dienstleistung",
            "hh_dienstleistung_art",
            "hh_dienstleistung_betrag",
        ),
        ("hh_handwerker", "hh_handwerker_art", "hh_handwerker_betrag"),
    ] {
        let paare = [
            (art, json!("1")),
            (betrag, json!(120_000)),
            (&format!("{art}__3")[..], json!("2")),
            (&format!("{betrag}__3")[..], json!(80_000)),
        ];
        let d = deklariere(&bestaetigt(&paare), index(), 2025, None).unwrap();
        let indizes: Vec<u64> = d
            .instanzen_der_gruppe(gruppe)
            .iter()
            .map(|i| i.index)
            .collect();
        assert!(d.eingaben_konsistent(), "{gruppe}: Vorbedingung");
        assert_eq!(indizes, [3], "{gruppe}: Vorbedingung, Instanz 2 fehlt");
        let opt = XmlOptionen {
            hersteller_id: Some("74931".to_owned()),
            ..XmlOptionen::default()
        };
        let xml = erzeuge_xml(&d, &opt).unwrap();
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let einz: Vec<Vec<&str>> = doc
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "Einz")
            .map(|e| {
                e.children()
                    .filter(roxmltree::Node::is_element)
                    .map(|k| k.text().unwrap_or(""))
                    .collect()
            })
            .collect();
        // beide Posten stehen drin, in Instanz-Reihenfolge, kein leeres <Einz> (120000 Cent = 1200)
        assert_eq!(einz, [["1", "1200"], ["2", "800"]], "{gruppe}");
    }
}

/// Was das Vorsatz-Seitengate verlangt: Name, Anschrift, Bankentscheidung, Steuernummer.
fn seitengate() -> Vec<(&'static str, Value)> {
    vec![
        ("stammdaten_nachname", json!("Maier")),
        ("stammdaten_vorname", json!("Hans")),
        ("stammdaten_strasse", json!("Musterstr.")),
        ("stammdaten_hausnummer", json!("5")),
        ("stammdaten_plz", json!("55555")),
        ("stammdaten_wohnort", json!("Musterort")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_steuernummer", json!("9181081508155")),
    ]
}

fn bestaetigt(paare: &[(&str, Value)]) -> Felder {
    paare
        .iter()
        .map(|(id, w)| ((*id).to_owned(), feld(w.clone(), Zustand::Bestaetigt)))
        .collect()
}

fn abgabe_xml(f: &Felder) -> Result<String, elster::XmlFehler> {
    let d = deklariere(f, index(), 2025, None).unwrap();
    let opt = XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        abgabefaehig: true,
        snapshot: Some(f),
        ..XmlOptionen::default()
    };
    erzeuge_xml(&d, &opt)
}

/// Umfangs-Pruefung am Writer (Original: `tests/test_pflichtfelder_am_writer.py`). Ein leerer
/// Store widerspricht sich nicht, also ist er konsistent — abgabefaehig ist er nicht.
#[test]
fn leerer_store_ergibt_kein_abgabefaehiges_xml() {
    let fehler = abgabe_xml(&Felder::new()).unwrap_err();
    assert!(fehler.0.contains("pflichtfelder_vollstaendig"), "{fehler}");
    for id in [
        "stammdaten_nachname",
        "stammdaten_vorname",
        "stammdaten_geburtsdatum",
        "stammdaten_strasse",
        "stammdaten_plz",
        "stammdaten_wohnort",
        "kist_konfession",
    ] {
        assert!(fehler.0.contains(id), "{id} fehlt in der Meldung: {fehler}");
    }
}

/// Der Fall, den vorher erst ERiC fing (gemessen 2026-10-01, rc=610001002): alles fuer den
/// Vorsatz da, Geburtsdatum und Konfession nicht. Das Seitengate sieht diese zwei nicht.
#[test]
fn seitengate_voll_aber_geburtsdatum_und_konfession_fehlen() {
    let fehler = abgabe_xml(&bestaetigt(&seitengate())).unwrap_err();
    assert_eq!(
        fehler.0,
        "abgabefaehig=True verlangt pflichtfelder_vollstaendig=True — fehlend: \
         ['stammdaten_geburtsdatum', 'kist_konfession']. checkESt lehnt das XML sonst ab \
         (rc=610001002)."
    );
}

/// Gegenprobe: ohne sie waere eine Pruefung, die alles ablehnt, ebenfalls gruen.
#[test]
fn gegenprobe_vollstaendiger_store_ergibt_das_xml() {
    if !schemas_da(2025) {
        return;
    }
    let mut paare = seitengate();
    paare.push(("stammdaten_geburtsdatum", json!("05.05.1955")));
    paare.push(("kist_konfession", json!("keine")));
    let xml = abgabe_xml(&bestaetigt(&paare)).unwrap();
    assert!(xml.contains("<Vorsatz>"));
}
