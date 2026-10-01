//! Eigenschaften der Deklaration ueber die ECHTE Bindung (`produkt/bindung/*.yaml`), ohne
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
use domain::{Achsenwert, Cent, Feldtyp, Herkunft, PruefTiefe, Zustand};
use elster::{
    cent_nach_kz, deklariere, erzeuge_xml, kz_format, zuruecklesen, Felder, KzFormat, XmlOptionen,
};
use proptest::prelude::*;
use serde_json::{json, Value};
use store::SnapshotFeld;

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
        bindung::lade_registry(&pfad)
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
    if elster::finde_schema(2025, "E10-{jahr}.xsd").is_none() {
        println!("E10-2025.xsd fehlt — source_unavailable");
        return;
    }
    let d = deklariere(
        &einzeln(
            "stammdaten_nachname",
            json!("Maier\u{0}"),
            Zustand::Bestaetigt,
        ),
        index(),
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
        let kz = b.elster_kz.as_deref().unwrap();
        let wert = beispiel(b);
        let d = deklariere(
            &einzeln(&b.feld_id, wert.clone(), Zustand::Bestaetigt),
            index(),
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
        let kz = b.elster_kz.as_deref().unwrap();
        let d = deklariere(&einzeln(&b.feld_id, json!(c), Zustand::Bestaetigt), index(), None).unwrap();
        if let Some(v) = d.deklaration.get(kz).or_else(|| d.person_b.get(kz)) {
            prop_assert_eq!(v, &cent_nach_kz(Cent::new(c), kz).als_json());
            match (kz_format(kz), v.as_i64()) {
                (KzFormat::EuroAbgerundet, Some(e)) => prop_assert!(e * 100 <= c),
                (KzFormat::EuroAufgerundet, Some(e)) => prop_assert!(e * 100 >= c),
                (KzFormat::KommaCent, None) => prop_assert_eq!(v, &json!(format!("{},{:02}", c / 100, c % 100))),
                (f, _) => prop_assert!(false, "{kz}: Format {f:?} passt nicht zu {v}"),
            }
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
        if let Some(i) = inst.iter().find(|i| i.index == idx) {
            prop_assert_eq!(i.zustand, erwartet);
        }
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
    let Some(pfad) = elster::finde_schema(2025, "E10-{jahr}.xsd") else {
        println!("E10-2025.xsd fehlt — source_unavailable");
        return;
    };
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
    for kz in elster::NULL_UNZULAESSIG_KZ {
        assert!(verbietet_null(&meta, kz), "{kz}: {}", typ(kz));
    }
    let cent_kz: Vec<&str> = bindungen()
        .iter()
        .filter(|b| b.typ == Feldtyp::Cent)
        .filter_map(|b| b.elster_kz.as_deref())
        .collect();
    let komma_luecke: Vec<&str> = cent_kz
        .iter()
        .copied()
        .filter(|kz| typ(kz).starts_with("Dezimalzahl") && kz_format(kz) != KzFormat::KommaCent)
        .collect();
    let null_luecke: Vec<&str> = cent_kz
        .iter()
        .copied()
        .filter(|kz| {
            typ(kz).starts_with("GanzzahlPos") && !elster::NULL_UNZULAESSIG_KZ.contains(kz)
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
/// Faellen). Die Kz-Menge kommt aus dem XSD, nicht aus `NULL_UNZULAESSIG_KZ`: Typ `GanzzahlPos*`
/// (Basis `xs:positiveInteger`) oder eine enumeration-/pattern-Facette ohne "0" (`GdB`, Pflegegrad).
/// Geprueft wird jede Kz in allen drei Buckets, nicht nur die Kz des Feldes.
#[test]
fn null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet() {
    let Some(pfad) = elster::finde_schema(2025, "E10-{jahr}.xsd") else {
        println!("E10-2025.xsd fehlt — source_unavailable");
        return;
    };
    let meta = elster::kz_meta(&pfad, "E10").unwrap();
    // Jeder Schreibweg ueber die Bindung: 1:1, Instanz (`__2`), Person B (Klasse g); 50 Cent werden
    // auf 0 Euro abgerundet. Die Art-Verzweigung (Klasse f) haengt an `pub(crate)`-Tabellen, sie
    // prueft `art_verzweigung_schreibt_keine_verbotene_null` (`src/deklaration.rs`).
    let durch: Vec<String> = bindungen()
        .iter()
        .filter(|b| matches!(b.typ, Feldtyp::Cent | Feldtyp::Int))
        .flat_map(|b| {
            let werte: &'static [i64] = if b.typ == Feldtyp::Cent {
                &[0, 50]
            } else {
                &[0]
            };
            let instanz = b
                .instanz_gruppe
                .as_ref()
                .map(|_| format!("{}__2", b.feld_id));
            std::iter::once(b.feld_id.clone())
                .chain(instanz)
                .flat_map(move |s| werte.iter().map(move |&w| (s.clone(), w)))
        })
        .flat_map(|(schluessel, wert)| {
            let d = deklariere(
                &einzeln(&schluessel, json!(wert), Zustand::Bestaetigt),
                index(),
                None,
            )
            .unwrap();
            let instanzen = d.anlage_instanzen.iter().flat_map(|(_, ii)| ii);
            let kz: std::collections::BTreeSet<String> = [&d.deklaration, &d.person_b]
                .into_iter()
                .chain(instanzen.map(|i| &i.felder))
                .flatten()
                .filter(|(kz, v)| **v == json!(0) && verbietet_null(&meta, kz))
                .map(|(kz, _)| kz.clone())
                .collect();
            kz.into_iter()
                .map(move |kz| format!("{kz} {schluessel}={wert}"))
        })
        .collect();
    assert!(
        durch.is_empty(),
        "0 in {} Faellen durchgelassen, obwohl der XSD-Typ sie verbietet: {durch:?}",
        durch.len()
    );
}

/// P9 (Vault: `decisions/elster-null-in-kz-ohne-null-weglassen`, Punkt 3): eine weggelassene 0
/// ist kein verlorener Wert, sondern „nichts anzugeben" — kein `nicht_deklariert`-Eintrag. Der
/// Spenden-Zweig schrieb bis P9 einen; mit Eintrag meldete die Pruefanzeige „nicht alle Werte".
#[test]
fn spende_null_ohne_nicht_deklariert() {
    let d = deklariere(
        &einzeln("spenden_betrag", json!(0), Zustand::Bestaetigt),
        index(),
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
    let f = einzeln(
        "rentner_hilflos_blind_taubblind",
        json!(false),
        Zustand::Bestaetigt,
    );
    let d = deklariere(&f, index(), None).unwrap();
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
    let f = einzeln(
        "rentner_hilflos_blind_taubblind",
        json!(true),
        Zustand::Bestaetigt,
    );
    let d = deklariere(&f, index(), None).unwrap();
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
    let f = einzeln("stammdaten_nachname", json!("Muster"), Zustand::Bestaetigt);
    let d = deklariere(&f, index(), None).unwrap();
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
    let d = deklariere(&felder, index(), None).unwrap();
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
    for vz in ["2024", "2025"] {
        let jahr: i64 = vz.parse().unwrap();
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
        let d = deklariere(&felder, index(), None).unwrap();
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
    let d = deklariere(f, index(), None).unwrap();
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
    let mut paare = seitengate();
    paare.push(("stammdaten_geburtsdatum", json!("05.05.1955")));
    paare.push(("kist_konfession", json!("keine")));
    let xml = abgabe_xml(&bestaetigt(&paare)).unwrap();
    assert!(xml.contains("<Vorsatz>"));
}
