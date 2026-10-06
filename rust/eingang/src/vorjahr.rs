//! Vorjahres-Uebernahme (`produkt/eingang/vorjahr_writer.py`): nur Felder mit Flag
//! `vorjahr ∈ {uebernehmbar, vorschlag}`, nur BESTAETIGTE Vorjahreswerte, nie ueberschreiben.
use std::collections::{BTreeMap, HashSet};

use bindung::Vorjahr;
use serde_json::{json, Value};
use store::{Abweisung, BindungNachschlag, Store};

use crate::vorschlag::{Quelle, SchreibFehler, VorschlagEvent};

/// Ein materialisiertes Vorjahresfeld (`{wert, zustand, herkunft}`); nur `wert`/`zustand`
/// zaehlen.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct VorjahrFeld {
    pub wert: Value,
    #[serde(default)]
    pub zustand: Option<String>,
}

/// Ergebnis: Zahl der Uebernahmen, die uebersprungenen Felder und die Vergleichsgroesse
/// `verlustvortrag_bestand`, die der Aufrufer im Fall ablegt (Python schreibt sie direkt als
/// `store["vorjahr_referenz"]`; die Rust-`StoreDatei` kennt das Feld noch nicht — offener Punkt
/// fuer `api`/`store`).
#[derive(Debug, Clone, PartialEq)]
pub struct VorjahrErgebnis {
    pub uebertragen: usize,
    /// `feld_id`s (sortiert), deren Vorjahreswert die heutige Wertpruefung abweist: ein Altwert,
    /// gespeichert vor dieser Pruefung. Nur die `feld_id` — die Abweisung nennt den Wert (PII).
    pub uebersprungen: Vec<String>,
    pub referenz: Option<Value>,
}

/// `uebertragbare_felder(bindung)`: `feld_id → Kategorie`, sortiert.
///
/// ```
/// use eingang::vorjahr::uebertragbare_felder;
/// use store::BindungNachschlag;
/// let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
/// let felder = uebertragbare_felder(nachschlag);
/// assert!(felder.values().all(|k| *k == "uebernehmbar" || *k == "vorschlag"));
/// ```
#[must_use]
pub fn uebertragbare_felder(bindung: BindungNachschlag<'_>) -> BTreeMap<String, &'static str> {
    bindung
        .alle()
        .filter_map(|(fid, b)| {
            b.vorjahr.map(|v| {
                (
                    fid.to_owned(),
                    match v {
                        Vorjahr::Uebernehmbar => "uebernehmbar",
                        Vorjahr::Vorschlag => "vorschlag",
                    },
                )
            })
        })
        .collect()
}

/// `uebernehme_vorjahr(neuer_store, vorjahr_felder, bindung, vorjahr_vz=, ts=)`.
///
/// Reihenfolge: sortiert nach `feld_id`. Python folgt der Reihenfolge der Bindung (bei `api.vorjahr`
/// die der Scheibe), und die steht in der Akte: wer sie nachbilden muss, ruft
/// [`uebernehme_in_reihenfolge`]. Das Ergebnis hängt davon nicht ab, weil vorlaeufige Events keine
/// Ableitung ausloesen (`store::Store::append`, `leite_ab`/`rechne_ab` nur fuer `bestaetigt`).
///
/// Eine Abweisung der Wertpruefung (Typ/Format) ueberspringt das Feld
/// ([`VorjahrErgebnis::uebersprungen`]); Entscheidung `vorjahr-unpassenden-altwert-ueberspringen`.
///
/// # Errors
/// [`SchreibFehler`] bei der ersten anderen Abweisung.
///
/// ```
/// use std::collections::BTreeMap;
/// use eingang::vorjahr::{uebernehme, uebertragbare_felder, VorjahrFeld};
/// use store::Store;
/// use store::BindungNachschlag;
/// let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
/// let mut store = Store::leer(2026, None);
/// // Ein Vorjahrsfeld ohne bestaetigten Zustand wird nie uebertragen.
/// let offen: BTreeMap<String, VorjahrFeld> = uebertragbare_felder(nachschlag)
///     .into_keys()
///     .map(|f| (f, VorjahrFeld { wert: serde_json::json!(1), zustand: Some("vorlaeufig".into()) }))
///     .collect();
/// let erg = uebernehme(&mut store, &offen, nachschlag, 2025, None).unwrap();
/// assert_eq!(erg.uebertragen, 0);
/// ```
pub fn uebernehme(
    store: &mut Store,
    vorjahr_felder: &BTreeMap<String, VorjahrFeld>,
    bindung: BindungNachschlag<'_>,
    vorjahr_vz: i64,
    ts: Option<&str>,
) -> Result<VorjahrErgebnis, SchreibFehler> {
    let reihenfolge: Vec<String> = uebertragbare_felder(bindung).into_keys().collect();
    uebernehme_in_reihenfolge(store, vorjahr_felder, bindung, &reihenfolge, vorjahr_vz, ts)
}

/// [`uebernehme`] in der Reihenfolge `reihenfolge` (`feld_id`s; eine ohne Flag in `bindung` wird
/// uebergangen): Python geht `bindung.items()` durch, und die Events stehen in dieser Reihenfolge
/// in der Akte. Bei der ersten anderen Abweisung bricht die Schleife dort ab, wo Python abbricht.
/// `uebersprungen` ist sortiert (`sorted(uebersprungen)`).
///
/// # Errors
/// [`SchreibFehler`] bei der ersten anderen Abweisung.
pub fn uebernehme_in_reihenfolge(
    store: &mut Store,
    vorjahr_felder: &BTreeMap<String, VorjahrFeld>,
    bindung: BindungNachschlag<'_>,
    reihenfolge: &[String],
    vorjahr_vz: i64,
    ts: Option<&str>,
) -> Result<VorjahrErgebnis, SchreibFehler> {
    let aktiv: HashSet<String> = store.aktive().map(|(f, _)| f.to_owned()).collect();
    let flags = uebertragbare_felder(bindung);
    let mut n = 0;
    let mut uebersprungen = Vec::new();
    for fid in reihenfolge {
        let Some(kat) = flags.get(fid) else {
            continue;
        };
        let Some(vf) = vorjahr_felder
            .get(fid)
            .filter(|v| v.zustand.as_deref() == Some("bestaetigt"))
        else {
            continue;
        };
        if aktiv.contains(fid) {
            continue;
        }
        let signal_1 = json!({"typ": "vorjahr", "vz": vorjahr_vz, "quell_feld_id": fid, "quell_wert": vf.wert, "kategorie": kat});
        let geschrieben = VorschlagEvent {
            quelle: Quelle::Vorjahr,
            feld_id: fid.clone(),
            wert: vf.wert.clone(),
            signal_1,
        }
        .schreibe(store, None, bindung, ts);
        match geschrieben {
            Ok(_) => n += 1,
            Err(e) if ist_pruef_abweisung(&e) => uebersprungen.push(fid.clone()),
            Err(e) => return Err(e),
        }
    }
    uebersprungen.sort();
    Ok(VorjahrErgebnis {
        uebertragen: n,
        uebersprungen,
        referenz: referenzwert_verlustvortrag(vorjahr_felder),
    })
}

/// Die fuenf Abweisungen der Wertpruefung (Auflage T, V, W, F und Z; Python: Praefix `fail-closed
/// (Typ)`/`(Vorzeichen)`/`(Bereich)`/`(Format)`/`(Zeichensatz)`). Nur sie ueberspringt [`uebernehme`],
/// jede andere bricht ab.
fn ist_pruef_abweisung(e: &SchreibFehler) -> bool {
    matches!(
        e,
        SchreibFehler::Abweisung(
            Abweisung::TypInkonform { .. }
                | Abweisung::NegativerBetrag { .. }
                | Abweisung::WertAusserhalbBereich { .. }
                | Abweisung::FormatInkonform { .. }
                | Abweisung::ZeichensatzVerletzt { .. }
        )
    )
}

/// `referenzwert_verlustvortrag`: `{"verlustvortrag_bestand": {"wert": …}}`, nur bei
/// bestaetigtem Vorjahreswert.
///
/// ```
/// let mut f = std::collections::BTreeMap::new();
/// f.insert("verlustvortrag_bestand".to_string(), eingang::vorjahr::VorjahrFeld { wert: 5.into(), zustand: Some("bestaetigt".into()) });
/// assert_eq!(eingang::vorjahr::referenzwert_verlustvortrag(&f), Some(serde_json::json!({"verlustvortrag_bestand": {"wert": 5}})));
/// ```
#[must_use]
pub fn referenzwert_verlustvortrag(
    vorjahr_felder: &BTreeMap<String, VorjahrFeld>,
) -> Option<Value> {
    let vf = vorjahr_felder
        .get("verlustvortrag_bestand")
        .filter(|v| v.zustand.as_deref() == Some("bestaetigt"))?;
    Some(json!({"verlustvortrag_bestand": {"wert": vf.wert}}))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use serde_json::json;
    use store::{Abweisung, BindungNachschlag, EventId, Store};

    use super::{ist_pruef_abweisung, uebernehme, VorjahrFeld};
    use crate::vorschlag::SchreibFehler;

    /// Entscheidung `vorjahr-unpassenden-altwert-ueberspringen`: ein Altwert, den die Wertpruefung
    /// abweist (Steuerzeichen, Muster, Bereich, Vorzeichen, Zeichensatz), reisst die uebrigen Vorschlaege
    /// nicht mit.
    #[test]
    fn abgewiesener_altwert_wird_uebersprungen() {
        let nachschlag = BindungNachschlag::neu(crate::doctest_bindung().unwrap());
        let vorjahr: BTreeMap<String, VorjahrFeld> = [
            ("veranlagung", json!("zusammen")),
            ("bruttoarbeitslohn", json!(4_000_000)),
            ("stammdaten_nachname", json!("Maier\u{0}")), // Auflage T: Steuerzeichen
            ("kind_wohnsitz_inland_zeitraum", json!("01.01-31.122")), // Auflage F: Muster
            ("geburtsjahr", json!(1899)),                 // Auflage W: Bereich 1900..2010
            ("hh_handwerker_betrag", json!(-5000)),       // Auflage V: Betrag ohne Minus
            // Auflage Z: Gedankenstrich
            (
                "rentner_gepflegter_angaben",
                json!("Mutter\u{2013}Pflegegrad"),
            ),
        ]
        .into_iter()
        .map(|(f, wert)| {
            let zustand = Some("bestaetigt".to_owned());
            (f.to_owned(), VorjahrFeld { wert, zustand })
        })
        .collect();
        let mut store = Store::leer(2025, None);
        let erg = uebernehme(&mut store, &vorjahr, nachschlag, 2024, None)
            .expect("ein abgewiesener Altwert bricht die Uebernahme nicht ab");
        assert_eq!(erg.uebertragen, 2);
        assert_eq!(
            erg.uebersprungen,
            [
                "geburtsjahr",
                "hh_handwerker_betrag",
                "kind_wohnsitz_inland_zeitraum",
                "rentner_gepflegter_angaben",
                "stammdaten_nachname"
            ]
        );
        let aktiv: BTreeSet<&str> = store.aktive().map(|(f, _)| f).collect();
        assert_eq!(aktiv, BTreeSet::from(["bruttoarbeitslohn", "veranlagung"]));
    }

    /// Nur die Wertpruefung (Typ/Format/Bereich) wird uebersprungen. Eine andere Abweisung erreicht
    /// `uebernehme` heute nicht (der Writer setzt Schreiber, Herkunft und Zustand selbst und prueft
    /// vorher auf ein aktives Event), darum hier an der Einordnung geprueft: kaeme eine hinzu,
    /// bricht sie ab statt still zu fehlen.
    #[test]
    fn andere_abweisung_bricht_weiter_ab() {
        let ueberspringt = |a: Abweisung| ist_pruef_abweisung(&a.into());
        assert!(ueberspringt(Abweisung::TypInkonform {
            feld_id: "f".into(),
            wert: "w".into(),
            typ: "int",
        }));
        assert!(ueberspringt(Abweisung::FormatInkonform {
            feld_id: "f".into(),
            muster: "m".into(),
        }));
        assert!(ueberspringt(Abweisung::NegativerBetrag {
            feld_id: "f".into(),
            wert: -1,
        }));
        assert!(ueberspringt(Abweisung::WertAusserhalbBereich {
            feld_id: "f".into(),
            wert: 1899,
            min: 1900,
            max: 2010,
        }));
        assert!(ueberspringt(Abweisung::ZeichensatzVerletzt {
            feld_id: "f".into(),
            zeichen: '\u{2013}',
        }));
        assert!(!ueberspringt(Abweisung::AktivesEventVorhanden {
            feld_id: "f".into(),
            aktives_event: EventId::aus_bytes([0; 32]),
        }));
        assert!(!ueberspringt(Abweisung::AuflageA {
            schreiber: "import:vorjahr".into(),
            erwartete_herkunft: "vorjahr",
            folge: "f",
        }));
        assert!(!ueberspringt(Abweisung::Magnitude {
            feld_id: "f".into(),
            schreiber: "import:vorjahr".into(),
            wert: "w".into(),
        }));
        assert!(!ueberspringt(Abweisung::ErsetztFeldMismatch));
        assert!(!ist_pruef_abweisung(&SchreibFehler::Konstante));
    }
}
