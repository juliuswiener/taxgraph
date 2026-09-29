//! Eigenschaften der Deklaration ueber die ECHTE Bindung (`produkt/bindung/*.yaml`), ohne
//! Python: Round-Trip, fail-closed, Rundung zugunsten der Steuerpflichtigen, Instanz-Meet.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::collections::HashMap;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Cent, Feldtyp, Herkunft, PruefTiefe, Zustand};
use elster::{cent_nach_kz, deklariere, kz_format, zuruecklesen, Felder, KzFormat};
use proptest::prelude::*;
use serde_json::{json, Value};
use store::SnapshotFeld;

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
        bindung::lade_registry(&pfad).expect("Bindung laedt").dateien.into_iter().flat_map(|(_, d)| d.bindungen).collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

fn laie_herkunft() -> Herkunft {
    let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
    Herkunft { herkunft: a("laie"), pruef_tiefe: PruefTiefe::Ungeprueft, haftung: a("nutzer") }
}

fn feld(wert: Value, zustand: Zustand) -> SnapshotFeld {
    SnapshotFeld { wert, zustand, herkunft: laie_herkunft().into() }
}

fn einzeln(feld_id: &str, wert: Value, zustand: Zustand) -> Felder {
    Felder::from([(feld_id.to_owned(), feld(wert, zustand))])
}

fn beispiel(b: &Bindung) -> Value {
    match b.typ {
        Feldtyp::Cent => json!(12_345),
        Feldtyp::Int => json!(3),
        Feldtyp::Bool => json!(true),
        Feldtyp::Enum => json!(b.enum_werte.as_ref().and_then(|w| w.first()).cloned().unwrap_or_default()),
        Feldtyp::Datum => json!("05.05.1990"),
        Feldtyp::Text => b.beispielwert.clone(),
    }
}

/// fail-closed: ein einziges VORLAEUFIGES Feld macht die Deklaration unvollstaendig und traegt
/// nichts ausser der Konstanten E0100001 in die Deklaration ein — fuer JEDE Bindung.
#[test]
fn vorlaeufig_deklariert_nie() {
    for b in bindungen() {
        let d = deklariere(&einzeln(&b.feld_id, beispiel(b), Zustand::Vorlaeufig), index(), None).unwrap();
        assert!(!d.eingaben_konsistent(), "{}", b.feld_id);
        assert_eq!(d.deklaration.keys().collect::<Vec<_>>(), ["E0100001"], "{}", b.feld_id);
        assert!(d.person_b.is_empty() && d.anlage_instanzen.is_empty() && d.dokumentiert.is_empty(), "{}", b.feld_id);
    }
}

/// Round-Trip fuer JEDE 1:1-Bindung (eigener `elster_kz`, keine Instanz): steht die Kz nach der
/// Deklaration mit dem Rohwert in der Deklaration, liest `zuruecklesen` exakt den Rohwert zurueck.
/// Ausnahmen muessen hier namentlich stehen.
#[test]
fn round_trip_eins_zu_eins() {
    let mut geprueft = 0;
    for b in bindungen().iter().filter(|b| b.elster_kz.is_some() && b.instanz_gruppe.is_none()) {
        let kz = b.elster_kz.as_deref().unwrap();
        let wert = beispiel(b);
        let d = deklariere(&einzeln(&b.feld_id, wert.clone(), Zustand::Bestaetigt), index(), None).unwrap();
        let Some(deklariert) = d.deklaration.get(kz) else { continue };
        let zurueck = zuruecklesen(&d, index());
        let erwartet = if b.typ == Feldtyp::Cent { deklariert.clone() } else { wert.clone() };
        // Wertekodierung (Religionsschluessel) ist bewusst nicht umkehrbar: "02" statt "evangelisch".
        if b.feld_id.starts_with("kist_konfession") {
            continue;
        }
        assert_eq!(zurueck.felder.get(&b.feld_id), Some(&erwartet), "{} ({kz})", b.feld_id);
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
                wert: beispiel(b),
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

/// Die handgepflegten Kz-Mengen gegen die aus dem XSD ABGELEITETEN Typen (Ersatz fuer die
/// `ponytail`-Grenzen in `est_mapping.py:149-151,179-180`). Die Teilmengen-Beziehung ist hart;
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
        assert!(typ(kz).starts_with("Dezimalzahl") && typ(kz).contains("MinNK2_MaxNK2"), "{kz}: {}", typ(kz));
    }
    for kz in elster::NULL_UNZULAESSIG_KZ {
        assert!(typ(kz).starts_with("GanzzahlPos"), "{kz}: {}", typ(kz));
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
        .filter(|kz| typ(kz).starts_with("GanzzahlPos") && !elster::NULL_UNZULAESSIG_KZ.contains(kz))
        .collect();
    println!("[xsd-abgeleitet] cent-Kz={} Dezimal-Typ ohne Komma-Format={komma_luecke:?}", cent_kz.len());
    println!("[xsd-abgeleitet] GanzzahlPos-Typ ohne 0-Sperre={} {null_luecke:?}", null_luecke.len());
}
