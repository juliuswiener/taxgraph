//! `nicht_negativ` in der Bindung gegen den Schematyp des Kz: ein Geldfeld, dessen Schematyp kein Minus kennt, lehnt ein Minus schon beim
//! Speichern ab (Vault `decisions/geldfeld-ohne-minus-im-schema-lehnt-minus-bei-eingabe-ab`). Gegenstueck zu
//! `tests/test_bindung_nicht_negativ.py`.
//!
//! **Worum es geht.** Ohne die Marke nimmt der Store ein Minus an (HTTP 201), und ELSTER lehnt die Erklaerung erst bei der Abgabe ab
//! ("Unzulaessiges Vorzeichen", gemessen 2026-10-02). `rust/store` prueft die Marke (`negativer_betrag_wird_abgewiesen_...` in
//! `store.rs`) -- aber ob JEDES Feld, dessen Schematyp kein Minus kennt, die Marke TRAEGT, und ob keine Marke an einem Feld steht, das
//! ein Minus braucht (ein Verlust), prueft kein Rust-Test: die Bindung gehoert seit Weg B Rust (`rust/bindung/daten`), die elf neuen
//! Betragsfelder seit Python (Parteispenden, R-AV/bAV, DBA, Privatverkauf, ...) kennt der Python-Test nicht mehr.
//!
//! Zwei Teile:
//!
//! - **Ohne Schema** (laeuft in der CI): die Marke steht nur an Zahlfeldern, die Verlustfelder tragen sie nicht, ein Partner-Spiegel
//!   ohne eigenes Kz folgt seinem Grundfeld, und der Store weist an JEDEM markierten Feld ein Minus ab, die 0 und Positives nicht.
//! - **Mit Schema** (nur lokal): jedes Zahlfeld, dessen Schematyp KEIN Minus kennt, traegt die Marke, und jede Marke steht an einem
//!   Schematyp ohne Minus. Das Urteil geht ueber die ganze Vererbungskette des Typs (Grundtyp `nonNegativeInteger`/`positiveInteger`
//!   oder ein Muster der Kette, auf das "-1" nicht passt), nicht ueber den Typnamen; bei einer Art-Verzweigung (das Kz haengt am
//!   Art-Feld) gilt "kein Minus" nur, wenn es fuer JEDEN Zweig gilt. Ohne Schema rot, ausser `TAXGRAPH_OHNE_XSD=1`
//!   (`elster::testhilfe::schemas_da`).
//!
//! Grenze: die Felder MIT `bereich` fehlen im Schema-Teil, ihr Minimum deckt das Minus. Ein Feld ohne Kz im Schema (Summanden, Partner-
//! Spiegel, Rechenfelder) hat kein Urteil; fuer die Spiegel haelt `partner_spiegel_...` die Gleichheit mit dem Grundfeld.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Feldtyp, Feldzustand, Herkunft, PruefTiefe, PyWert, Schreiber, Signal2};
use elster::testhilfe::schemas_da;
use elster::{ernte_est_mapping_kz, finde_schema, kz_meta, KzMeta};
use serde_json::{json, Value};
use store::{Abweisung, BindungNachschlag, NeuesEvent, Store};

mod gemeinsam;
use gemeinsam::{typen, voll, Typen};

const VZ: i64 = 2025;

/// Diese vier tragen einen Verlust und duerfen nie die Marke tragen (Gewinn/Verlust aus Gewerbe, Selbstaendigkeit, Beteiligung).
const VERLUSTFELDER: [&str; 4] = ["einkuenfte_gewinn", "einkuenfte_gewinn_partner", "gewinnanteil", "gewinnanteil_partner"];

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
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

fn mit_marke() -> Vec<&'static Bindung> {
    let mut v: Vec<&Bindung> = bindungen().iter().filter(|b| b.nicht_negativ).collect();
    v.sort_by(|a, b| a.feld_id.cmp(&b.feld_id));
    v
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Ohne Schema (CI)

#[test]
fn die_marke_steht_nur_an_zahlfeldern_und_die_verlustfelder_tragen_sie_nicht() {
    let marken = mit_marke();
    // Floor: gemessen 63 am 2026-10-13 (der Python-Stand von 2026-10-02 zaehlte 52; elf Rust-eigene Betragsfelder kamen dazu).
    assert!(marken.len() >= 63, "nur {} Felder tragen nicht_negativ: die Bindung wurde nicht gelesen oder eine Marke fiel weg", marken.len());
    let kein_zahlfeld: Vec<&str> = marken
        .iter()
        .filter(|b| !matches!(b.typ, Feldtyp::Cent | Feldtyp::Int))
        .map(|b| b.feld_id.as_str())
        .collect();
    assert!(kein_zahlfeld.is_empty(), "nicht_negativ an Feldern, die keine Zahl sind: {kein_zahlfeld:?}");
    for v in VERLUSTFELDER {
        let b = index().get(v).unwrap_or_else(|| panic!("{v} fehlt in der Bindung"));
        assert!(!b.nicht_negativ, "{v} traegt einen Verlust und darf nicht_negativ nicht tragen");
    }
}

#[test]
fn partner_spiegel_ohne_kz_folgt_dem_grundfeld() {
    // Ein Partner-Feld ohne eigenes Kz tragt die Marke genau dann, wenn sein Grundfeld sie traegt (Klasse g: derselbe Schematyp).
    let mut abweichend = Vec::new();
    let mut geprueft = 0;
    for b in bindungen() {
        let Some(grund) = b.feld_id.strip_suffix("_partner") else {
            continue;
        };
        if b.typ != Feldtyp::Cent || b.elster_kz.is_some() {
            continue;
        }
        let Some(g) = index().get(grund) else {
            continue;
        };
        geprueft += 1;
        if b.nicht_negativ != g.nicht_negativ {
            abweichend.push(format!("{} (Marke {}) <-> {grund} (Marke {})", b.feld_id, b.nicht_negativ, g.nicht_negativ));
        }
    }
    assert!(geprueft >= 8, "nur {geprueft} Partner-Spiegel gefunden (Python-Stand: 8)");
    assert!(abweichend.is_empty(), "Partner-Spiegel und Grundfeld weichen bei nicht_negativ ab: {abweichend:#?}");
}

fn mensch(feld: &str, wert: &Value) -> NeuesEvent {
    NeuesEvent {
        feld_id: feld.to_owned(),
        wert: wert.clone().into(),
        feldzustand: Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() },
        herkunft: Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("laie".to_owned()),
        signal_1: None,
        ersetzt: None,
        ts: Some("2026-01-01T00:00:00+00:00".to_owned()),
    }
}

fn nachschlag() -> BindungNachschlag<'static> {
    BindungNachschlag::neu(index())
}

/// Legt `wert` in `feld` ab, in einem leeren Store.
fn speichere(feld: &str, wert: &Value) -> (Store, Result<store::EventId, Abweisung>) {
    let mut s = Store::leer(2025, None);
    let r = s.append(&mensch(feld, wert), None, nachschlag());
    (s, r)
}

#[test]
fn der_store_weist_an_jedem_markierten_feld_ein_minus_ab_die_null_und_positives_nicht() {
    for b in mit_marke() {
        let f = b.feld_id.as_str();
        let (s, r) = speichere(f, &json!(-1));
        assert_eq!(r, Err(Abweisung::NegativerBetrag { feld_id: f.to_owned(), wert: -1 }), "{f}: ein Minus muss abgewiesen werden");
        assert!(s.aktives(f).is_none(), "{f}: ein abgewiesener Wert darf nichts schreiben");
        assert!(r.unwrap_err().to_string().starts_with(&format!("fail-closed (Vorzeichen): {f}=-1 darf nicht negativ")));
        for wert in [0_i64, 50_000] {
            let (s, r) = speichere(f, &json!(wert));
            // Ein Wert ausserhalb von `bereich` weist eine ANDERE Auflage ab; an der Vorzeichen-Auflage darf es nicht liegen.
            assert!(!matches!(r, Err(Abweisung::NegativerBetrag { .. })), "{f}={wert}: {r:?}");
            if r.is_ok() {
                assert_eq!(s.aktives(f).unwrap().wert, PyWert::Ganz(wert), "{f}");
            }
        }
    }
}

#[test]
fn die_verlustfelder_nehmen_weiter_ein_minus_ein_und_ohne_bindung_prueft_der_store_nichts() {
    for v in VERLUSTFELDER {
        let (s, r) = speichere(v, &json!(-5_000_000));
        assert!(r.is_ok(), "{v}: {r:?}");
        assert_eq!(s.aktives(v).unwrap().wert, PyWert::Ganz(-5_000_000), "{v}");
    }
    // Eine Instanz (`__2`) prueft die Grenze ihres Basisfelds.
    let (_, r) = speichere("hh_handwerker_betrag__2", &json!(-1));
    assert_eq!(r, Err(Abweisung::NegativerBetrag { feld_id: "hh_handwerker_betrag__2".to_owned(), wert: -1 }));
    let (_, r) = speichere("hh_handwerker_betrag__2", &json!(1));
    assert!(r.is_ok(), "{r:?}");
    // Ein Text im Geldfeld bleibt ein Typfehler, kein Vorzeichenfehler.
    let (_, r) = speichere("hh_handwerker_betrag", &json!("-5"));
    assert!(matches!(r, Err(Abweisung::TypInkonform { .. })), "{r:?}");
    // Die Pruefung haengt wie Auflage T an der Bindung: ohne sie bleiben Bestandsaufrufe unberuehrt.
    let leer: HashMap<String, &Bindung> = HashMap::new();
    let mut s = Store::leer(2025, None);
    let r = s.append(&mensch("hh_handwerker_betrag", &json!(-1)), None, BindungNachschlag::neu(&leer));
    assert!(r.is_ok(), "{r:?}");
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Mit Schema (lokal)

struct Schema {
    e10: (HashMap<String, KzMeta>, Typen),
    e77: (HashMap<String, KzMeta>, Typen),
}

fn schema() -> &'static Schema {
    static CELL: OnceLock<Schema> = OnceLock::new();
    CELL.get_or_init(|| {
        let lade = |muster: &str, start: &str| {
            let pfad = finde_schema(VZ, muster).unwrap_or_else(|| panic!("{muster} fehlt"));
            (kz_meta(&pfad, start).unwrap(), typen(VZ, muster))
        };
        Schema { e10: lade("E10-{jahr}.xsd", "E10"), e77: lade("E77-{jahr}.xsd", "E77") }
    })
}

/// `Some(true)`: der Schematyp des Kz kennt kein Minus (Grundtyp `nonNegativeInteger`/`positiveInteger`, oder ein Muster der Kette, auf
/// das "-1" bzw. "-1,00" nicht passt; die Muster EINES Schritts sind Alternativen, die Schritte gelten zugleich). `None`: das Kz steht
/// nicht im Schema.
fn kz_kennt_kein_minus(kz: &str) -> Option<bool> {
    let (meta, typen) = if kz.get(1..3) == Some("60") { &schema().e77 } else { &schema().e10 };
    let m = meta.get(kz)?;
    let probe = if m.type_name.contains("Dezimal") { "-1,00" } else { "-1" };
    let (mut name, mut gruppen) = (m.type_name.clone(), Vec::<&Vec<String>>::new());
    while let Some((basis, muster)) = typen.get(&name) {
        gruppen.push(muster);
        match basis {
            Some(b) => name = b.clone(),
            None => break,
        }
    }
    if name == "xs:nonNegativeInteger" || name == "xs:positiveInteger" {
        return Some(true);
    }
    Some(!gruppen.iter().filter(|g| !g.is_empty()).all(|g| g.iter().any(|p| voll(p, probe))))
}

/// Das eigene Kz, sonst alle Kz der Art-Verzweigung (das Kz haengt am Art-Feld, `elster_kz` der Bindung ist `null`).
fn kz_des_felds(b: &Bindung) -> Vec<String> {
    static ERNTE: OnceLock<Vec<elster::KzPruefling>> = OnceLock::new();
    if let Some(k) = &b.elster_kz {
        return vec![k.as_str().to_owned()];
    }
    let ernte = ERNTE.get_or_init(|| ernte_est_mapping_kz(index()).expect("Tabellen-Kz haben ein Feld in der Bindung"));
    let praefix = format!("verzweigung:{}:", b.feld_id);
    let mut kz: Vec<String> = ernte.iter().filter(|k| k.feld_id.starts_with(&praefix)).map(|k| k.elster_kz.clone()).collect();
    kz.sort();
    kz.dedup();
    kz
}

/// Bei einer Art-Verzweigung gilt "kein Minus" nur, wenn es fuer JEDEN Zweig gilt. `None` ohne Kz oder ohne Schemaeintrag.
fn feld_kennt_kein_minus(b: &Bindung) -> Option<bool> {
    let urteile: Vec<Option<bool>> = kz_des_felds(b).iter().map(|k| kz_kennt_kein_minus(k)).collect();
    if urteile.is_empty() || urteile.contains(&None) {
        return None;
    }
    Some(urteile.iter().all(|u| *u == Some(true)))
}

#[test]
fn jedes_zahlfeld_mit_schematyp_ohne_minus_traegt_die_marke() {
    if !schemas_da(VZ) {
        return;
    }
    let zahl: Vec<&Bindung> = bindungen()
        .iter()
        .filter(|b| matches!(b.typ, Feldtyp::Cent | Feldtyp::Int) && b.bereich.is_none())
        .collect();
    let urteil: BTreeMap<&str, Option<bool>> = zahl.iter().map(|b| (b.feld_id.as_str(), feld_kennt_kein_minus(b))).collect();
    let unmarkiert: Vec<&str> = zahl
        .iter()
        .filter(|b| urteil[b.feld_id.as_str()] == Some(true) && !b.nicht_negativ)
        .map(|b| b.feld_id.as_str())
        .collect();
    assert!(
        unmarkiert.is_empty(),
        "{} Zahlfelder, deren Schematyp kein Minus kennt, tragen nicht_negativ nicht -- ein Minus landet dort ungefragt in der Erklaerung \
         und scheitert erst bei ELSTER: {unmarkiert:?}",
        unmarkiert.len()
    );
    // KONTROLLE (ein Null-Ergebnis zaehlt erst, wenn die Pruefung etwas findet): dieselbe Funktion erkennt die markierten Felder mit
    // Schematyp als "kein Minus" (gemessen 2026-10-13: siehe Ausgabe); ein Feld ohne Schematyp (Partner-Spiegel, Summand) hat kein Urteil.
    let markiert_mit_urteil: Vec<&Bindung> = mit_marke().into_iter().filter(|b| b.bereich.is_none() && !kz_des_felds(b).is_empty()).collect();
    println!(
        "nicht_negativ_gegen_schema: {} Zahlfelder ohne bereich, {} davon mit Marke und Schema-Kz, {} Urteile 'kein Minus'",
        zahl.len(),
        markiert_mit_urteil.len(),
        urteil.values().filter(|u| **u == Some(true)).count()
    );
    // Gemessen 2026-10-13: 149 Zahlfelder ohne bereich, 49 markiert mit Schema-Kz, 46 Urteile "kein Minus" (49 minus die drei bewussten).
    assert!(markiert_mit_urteil.len() >= 45, "nur {} markierte Felder mit Schema-Kz: die Kontrolle sah zu wenig", markiert_mit_urteil.len());
    assert!(
        urteil.values().filter(|u| **u == Some(true)).count() >= 44,
        "nur {} Zahlfelder mit Urteil 'kein Minus' (Python-Stand 2026-10-02: 44)",
        urteil.values().filter(|u| **u == Some(true)).count()
    );
    // Die Zweig-Logik unterscheidet etwas: gemischte Zweige sind NICHT "kein Minus" (gewerbe/land_forst NichtNeg, selbstaendig nicht).
    assert_eq!(feld_kennt_kein_minus(index()["rentner_veraeusserungsgewinn"]), Some(false));
    // Das int-Feld ohne bereich, dessen Schematyp kein Minus kennt: -5 km kam ueber POST /event in die Akte.
    assert_eq!(feld_kennt_kein_minus(index()["ep_entfernung_km"]), Some(true));
    // Der Fund der Haertung: p35c_massnahme_einzelbetrag hat kein eigenes Kz, aber in KEINEM der neun Zweige ein Minus.
    assert_eq!(feld_kennt_kein_minus(index()["p35c_massnahme_einzelbetrag"]), Some(true));
}

#[test]
fn keine_marke_steht_an_einem_schematyp_der_ein_minus_erlaubt() {
    if !schemas_da(VZ) {
        return;
    }
    // Die Gegenrichtung: nicht_negativ ohne Schemagrund wuerde ein erlaubtes Minus abweisen.
    let zuviel: BTreeSet<&str> = mit_marke()
        .into_iter()
        .filter(|b| feld_kennt_kein_minus(b) == Some(false))
        .map(|b| b.feld_id.as_str())
        .collect();
    let bewusst: BTreeSet<&str> = BEWUSSTE_MARKE_OHNE_SCHEMAGRUND.iter().map(|(f, _)| *f).collect();
    // Eine neue Marke ohne Schemagrund ist ein Entscheid mit Grund (Zeile in der Liste), kein Versehen ...
    let neu: Vec<&&str> = zuviel.difference(&bewusst).collect();
    assert!(neu.is_empty(), "nicht_negativ an Feldern, deren Schematyp ein Minus erlaubt (Entscheid mit Grund in BEWUSSTE_MARKE_OHNE_SCHEMAGRUND): {neu:?}");
    // ... und eine Zeile, die nichts mehr deckt (Marke weg, oder das Schema verbietet das Minus jetzt selbst), faellt aus der Liste.
    let veraltet: Vec<&&str> = bewusst.difference(&zuviel).collect();
    assert!(veraltet.is_empty(), "BEWUSSTE_MARKE_OHNE_SCHEMAGRUND nennt Felder, die keine solche Marke mehr tragen: {veraltet:?}");
}

/// Felder, deren Marke STRENGER ist als das Schema: der Schematyp (`GanzzahlOhneFuehrNull_MaxVK12_...`, wie `E0200201` Bruttoarbeitslohn)
/// erlaubt ein Minus, die Bindung nimmt es nicht an. Alle drei sind Rust-eigen (Abweichung Nr. 55, 2026-10-10): der Python-Test von
/// 2026-10-02 hielt "keine Marke ohne Schemagrund" fest und kannte sie nicht. Der Grund steht bei jeder Zeile; eine Marke ohne Zeile ist rot.
const BEWUSSTE_MARKE_OHNE_SCHEMAGRUND: [(&str, &str); 3] = [
    ("rav_bav_nr1_betrag", "Leistungsbetrag Nr. 1 einer Leistungsmitteilung (Anlage R-AV/bAV Zeile 4): nie negativ, das Schema laesst es zu"),
    ("rav_bav_nr4_betrag", "Leistungsbetrag Nr. 4 einer Leistungsmitteilung (Zeile 10 + 12): nie negativ, das Schema laesst es zu"),
    ("rav_bav_nr5_betrag", "Leistungsbetrag Nr. 5 einer Leistungsmitteilung (Zeile 15 + 16): nie negativ, das Schema laesst es zu"),
];
