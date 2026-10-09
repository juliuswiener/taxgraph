//! Anlage AUS, Zeile "1. Staat" (`E0600301`, Abweichung Nr. 51 in `rust/fixtures/README.md`): wer Auslandseinkuenfte erklaert,
//! muss den Staat nennen. Die amtliche Pruefung (`checkESt`) lehnt jede Akte mit Auslandseinkuenften ohne Staat ab
//! (rc=610001002), auch die Anrechnung, die nicht gesperrt ist. TaxGraph fragt den Staat (`dba_staat`), schrieb ihn aber nicht.
//!
//! Der Staat steht als KLARTEXT aus der amtlichen Laenderliste im Schema, nicht als Schluessel. Drei Bindungswerte weichen vom
//! Listentext ab (`Oesterreich`, `Tschechien`, `Grossbritannien`), `sonstiger_staat` hat keinen. `checkESt` prueft den Text
//! NICHT (26 Texte, alle rc=0, auch "Atlantis"): nur die Gegenprobe gegen die Liste in diesem Test faengt einen falschen Text.
//!
//! Die Regeln (Vault `staat-der-auslandseinkuenfte-steht-als-listentext-in-der-erklaerung-sonstiger-staat-sperrt`):
//! 1. Die 15 benannten Staaten schreiben ihren Listentext.
//! 2. `sonstiger_staat`, ein fehlender und ein unbekannter Wert schreiben nichts und sperren die Abgabe mit Klartext VOR dem XML.
//! 3. Der Staat steht genau dann, wenn die Einkuenfte `E0601401` stehen. Steuer (Anrechnung oder Abzug) ohne Einkuenfte sperrt.
//!
//! Der Test geht ueber `einreichungs_xml` (Ring, Guard, `deklariere`), nicht ueber `POST /einreichen`: die Sperre liegt VOR dem
//! Writer und braucht kein ERiC. Die Akte ist `rust/fixtures/e2e/gesamt.json` (vollstaendig, ohne Auslandsangaben) plus die
//! Auslandsangaben des jeweiligen Falls. Nur die Gegenprobe gegen die Laenderliste liest das ERiC-Schema und entfaellt ohne.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::deklaration::{einreichungs_xml, EinreichFehler};
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Zustand};
use elster::testhilfe::schemas_da;
use elster::{deklariere, finde_schema, kz_meta};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const STAAT: &str = "dba_staat";
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const ART: &str = "dba_einkunftsart";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const WAHL: &str = "dba_abzug_statt_anrechnung";

/// Anlage AUS, `Staat_Spez_InvFonds`: "aus dem Staat / Spezial-Investmentfonds".
const STAAT_KZ: &str = "E0600301";
const EINKUENFTE_KZ: &str = "E0601401";
const ANRECHNUNG_KZ: &str = "E0601901";
const ABZUG_KZ: &str = "E0600920";

/// Die 15 benannten Bindungswerte von `dba_staat` und ihr Text in der Laenderliste des Schemas (E10-2025.xsd, Typ
/// `NAEnum_LAND_OF_WORLD_2020_1_BaseCType`). 12 sind gleich, 3 weichen ab. Die Erwartung steht hier OHNE Bezug auf die Tabelle
/// im Produkt, sonst prueften beide Seiten dasselbe.
const STAATEN: [(&str, &str); 15] = [
    ("Deutschland", "Deutschland"),
    ("Frankreich", "Frankreich"),
    ("Italien", "Italien"),
    ("Oesterreich", "Österreich"),
    ("Schweiz", "Schweiz"),
    ("Niederlande", "Niederlande"),
    ("Polen", "Polen"),
    ("Tschechien", "Tschechische Republik"),
    ("Dänemark", "Dänemark"),
    ("Luxemburg", "Luxemburg"),
    ("Türkei", "Türkei"),
    ("Grossbritannien", "Vereinigtes Königreich"),
    ("Spanien", "Spanien"),
    ("USA", "USA"),
    ("Kanada", "Kanada"),
];

/// Ein Wert mit dem Zustand `bestaetigt` (zwei Signale).
fn setze(s: &mut Store, feld: &str, wert: Value) {
    let leer = HashMap::new();
    s.append_roh(
        &NeuesEventRoh {
            feld_id: feld.to_owned(),
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: HerkunftVektor::Voll(Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: Some(format!("ok@{feld}")),
                signal_2_fehlt: false,
            },
            signal_2_fremd: None,
            ersetzt: None,
            ts: Some("2026-09-01T00:00:00+00:00".to_owned()),
        },
        None,
        BindungNachschlag::neu(&leer),
    )
    .unwrap();
}

/// Die Auslandsangaben eines Falls. `staat`: der Wert von `dba_staat`, `None` = nie beantwortet. `einkuenfte`: 5.000 Euro aus
/// Arbeitslohn, sonst nie beantwortet. `steuer` in CENT. `abzug`: die Wahl "Abzug statt Anrechnung" mit `true`.
struct Ausland {
    staat: Option<Value>,
    einkuenfte: bool,
    steuer: Option<i64>,
    abzug: bool,
}

impl Ausland {
    fn mit_staat(staat: Value) -> Self {
        Self { staat: Some(staat), einkuenfte: true, steuer: None, abzug: false }
    }
}

/// Die vollstaendige Akte `gesamt.json` (Arbeitslohn, einzeln veranlagt, ohne Auslandsangaben) plus `a`.
fn akte(a: &Ausland) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    if a.einkuenfte {
        setze(&mut s, EINKUENFTE, json!(500_000));
        setze(&mut s, ART, json!("unselbstaendige_arbeit"));
    }
    if let Some(staat) = &a.staat {
        setze(&mut s, STAAT, staat.clone());
    }
    if let Some(cent) = a.steuer {
        setze(&mut s, STEUER, json!(cent));
    }
    if a.abzug {
        setze(&mut s, WAHL, json!(true));
    }
    s
}

fn lauf(s: &Store) -> Result<String, EinreichFehler> {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned())).map(|e| e.xml)
}

/// Die Eintraege der Sperre als `(feld_id, grund)`; leer, wenn `lauf` nicht mit `DeklarationUnvollstaendig` endet.
fn sperre(r: &Result<String, EinreichFehler>) -> Vec<(String, String)> {
    match r {
        Err(EinreichFehler::DeklarationUnvollstaendig(e)) => {
            e.iter().map(|x| (x.feld_id.clone(), x.grund.clone())).collect()
        }
        _ => Vec::new(),
    }
}

/// Der Kz-Wert aus `deklariere` als Text (ohne Anfuehrungszeichen); `None`, wenn der Kz fehlt.
fn kz_wert(s: &Store, kz: &str) -> Option<String> {
    let (felder, _) = s.materialisiere(None).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    d.deklaration.get(kz).map(|v| v.to_string().trim_matches('"').to_owned())
}

/// Die Sperr-Eintraege zu einem Feld.
fn sperre_zu<'a>(eintraege: &'a [(String, String)], feld: &str) -> Vec<&'a str> {
    eintraege.iter().filter(|(f, _)| f == feld).map(|(_, g)| g.as_str()).collect()
}

/// KONTROLLE zuerst: ohne Auslandsangaben steht kein Staat, nichts sperrt, die Akte ist einreichbar (XML mit Schema, sonst
/// endet der Lauf am Writer). Sonst belegten die Sperren unten nichts: ein Fall, der aus anderem Grund sperrt, laesst sie gruen.
#[test]
fn kontrolle_ohne_auslandsangaben_steht_kein_staat_und_nichts_sperrt() {
    let s = akte(&Ausland { staat: None, einkuenfte: false, steuer: None, abzug: false });
    let r = lauf(&s);
    assert_eq!(r.is_ok(), schemas_da(2025), "{r:?}");
    assert!(sperre(&r).is_empty(), "ohne Auslandsangaben sperrt schon etwas: {r:?}");
    assert_eq!(kz_wert(&s, STAAT_KZ), None);
    assert_eq!(kz_wert(&s, EINKUENFTE_KZ), None);
}

/// AK1: die 15 benannten Staaten schreiben ihren Listentext in `E0600301`, und die Akte sperrt nicht. Die Tabelle dieses Tests
/// deckt genau die Werte der Bindung (ausser `sonstiger_staat`): kommt ein Wert dazu, fehlt er hier und der Test wird rot.
#[test]
fn die_fuenfzehn_staaten_stehen_als_listentext_in_e0600301() {
    let mut bindungswerte = index()[STAAT].enum_werte.clone().unwrap();
    bindungswerte.retain(|w| w != "sonstiger_staat");
    bindungswerte.sort();
    let mut erwartet: Vec<String> = STAATEN.iter().map(|(b, _)| (*b).to_owned()).collect();
    erwartet.sort();
    assert_eq!(bindungswerte, erwartet, "die Bindung nennt andere Staaten als die Tabelle dieses Tests");

    for (bindungswert, text) in STAATEN {
        let s = akte(&Ausland::mit_staat(json!(bindungswert)));
        assert_eq!(kz_wert(&s, STAAT_KZ).as_deref(), Some(text), "Staat {bindungswert}");
        assert_eq!(kz_wert(&s, EINKUENFTE_KZ).as_deref(), Some("5000"), "Staat {bindungswert}: Einkuenfte");
        // Der Staat sperrt nicht. Andere Sperren bleiben moeglich und gehoeren nicht hierher: der Ring leitet die DBA-Methode aus
        // Staat und Einkunftsart ab, und fuer Oesterreich mit Arbeitslohn sperrt die Freistellung (Abweichung Nr. 40).
        let r = lauf(&s);
        let alle = sperre(&r);
        assert!(sperre_zu(&alle, STAAT).is_empty(), "Staat {bindungswert}: die Sperre zum Staat greift: {r:?}");
        assert!(sperre_zu(&alle, STEUER).is_empty(), "Staat {bindungswert}: die Sperre zur Steuer greift: {r:?}");
    }
}

/// Die Laenderliste des Schemas: der Dokumentationstext des Typs, den `E0600301` fuehrt (`NAEnum_LAND_OF_WORLD_2020_1_BaseCType`,
/// ohne Aufzaehlung im Schema). Die Eintraege stehen durch Komma getrennt; wenige tragen selbst ein Komma ("Kongo, Demokratische
/// Republik"), sie zerfallen in zwei Stuecke. ponytail: Eintraege mit Komma zerfallen, weil kein benannter Staat eines hat;
/// kommt einer mit Komma dazu, braucht der Test die echte Trennung (Eintrag = Stueck, das mit Grossbuchstaben beginnt).
fn laenderliste() -> Vec<String> {
    let pfad = finde_schema(2025, "E10-{jahr}.xsd").expect("E10-2025.xsd");
    let meta = kz_meta(&pfad, "E10").unwrap();
    let typ = meta[STAAT_KZ].type_name.clone();
    let basis = typ.strip_suffix("_RABE").unwrap_or(&typ);
    assert_eq!(
        basis, "NAEnum_LAND_OF_WORLD_2020_1_BaseCType",
        "E0600301 fuehrt einen anderen Typ als bei der Messung (2026-10-10): die Liste muss neu gelesen werden"
    );
    let xsd = std::fs::read_to_string(&pfad).unwrap();
    let ab = xsd.find(&format!("<xs:complexType name=\"{basis}\">")).expect("Typ im Schema");
    let doku = &xsd[ab..];
    let von = doku.find("<xs:documentation>").unwrap() + "<xs:documentation>".len();
    let bis = doku.find("</xs:documentation>").unwrap();
    doku[von..bis].replace("&apos;", "'").split(',').map(|e| e.trim().to_owned()).collect()
}

/// AK1, Gegenprobe: jeder Text, den das Produkt schreibt, steht in der Laenderliste des Schemas. `checkESt` prueft das nicht.
/// Die Kontrollen zeigen, dass die Liste gelesen wird und etwas ablehnt: "Frankreich" steht drin, "Atlantis" und die
/// Bindungswerte `Tschechien` und `Grossbritannien` nicht (der Rohwert waere ein falscher Text).
#[test]
fn jeder_geschriebene_text_steht_in_der_laenderliste_des_schemas() {
    if !schemas_da(2025) {
        return;
    }
    let liste = laenderliste();
    assert!(liste.iter().any(|e| e == "Frankreich"), "die Liste wurde nicht gelesen: {} Stuecke", liste.len());
    for falsch in ["Atlantis", "Tschechien", "Grossbritannien", "Oesterreich", "sonstiger_staat"] {
        assert!(!liste.iter().any(|e| e == falsch), "{falsch} steht in der Liste: die Gegenprobe taugt nicht");
    }
    for (bindungswert, _) in STAATEN {
        let s = akte(&Ausland::mit_staat(json!(bindungswert)));
        let text = kz_wert(&s, STAAT_KZ).unwrap_or_else(|| panic!("{bindungswert}: kein E0600301"));
        assert!(liste.iter().any(|e| *e == text), "{bindungswert}: '{text}' steht nicht in der Laenderliste");
    }
}

/// AK2: `sonstiger_staat`, ein fehlender und ein unbekannter Wert schreiben kein `E0600301` und sperren mit Klartext VOR dem
/// XML. Unbekannt ist alles, was kein Bindungswert ist: andere Schreibweise, der Listentext selbst (`Österreich`), ein Leerzeichen
/// zu viel. Eine Zahl als Staat erreicht `deklariere` nicht: der Ring lehnt sie vorher ab (`strip() auf Nicht-Text`, Parität).
/// KONTROLLE im selben Lauf: derselbe Fall mit `Frankreich` sperrt nicht.
#[test]
fn sonstiger_staat_fehlender_und_unbekannter_wert_schreiben_nichts_und_sperren_mit_klartext() {
    let kontrolle = akte(&Ausland::mit_staat(json!("Frankreich")));
    assert!(sperre(&lauf(&kontrolle)).is_empty(), "die Kontrolle sperrt schon");
    assert_eq!(kz_wert(&kontrolle, STAAT_KZ).as_deref(), Some("Frankreich"));

    let faelle: [(&str, Option<Value>); 7] = [
        ("sonstiger_staat", Some(json!("sonstiger_staat"))),
        ("Staat nie beantwortet", None),
        ("Atlantis", Some(json!("Atlantis"))),
        ("frankreich", Some(json!("frankreich"))),
        ("Frankreich mit Leerzeichen", Some(json!("Frankreich "))),
        ("Listentext Österreich", Some(json!("Österreich"))),
        ("Listentext Tschechische Republik", Some(json!("Tschechische Republik"))),
    ];
    for (name, staat) in faelle {
        let s = akte(&Ausland { staat, einkuenfte: true, steuer: None, abzug: false });
        assert_eq!(kz_wert(&s, STAAT_KZ), None, "{name}: der Staat darf nicht im XML stehen");
        let r = lauf(&s);
        let alle = sperre(&r);
        let gruende = sperre_zu(&alle, STAAT);
        assert_eq!(gruende.len(), 1, "{name}: genau ein Sperr-Eintrag zu {STAAT} erwartet, Lauf: {r:?}");
        assert!(gruende[0].contains("Staat"), "{name}: der Grund nennt den Staat nicht: {}", gruende[0]);
        assert!(r.is_err(), "{name}: die Abgabe ist nicht gesperrt");
    }
}

/// AK3: der Staat steht genau dann, wenn die Einkuenfte `E0601401` stehen. Steuer (Anrechnung `E0601901` oder Abzug `E0600920`)
/// ohne Einkuenfte sperrt mit Klartext zur Steuer; `checkESt` lehnt sie sonst mit "Einkuenfte mit dem Wert 0 erklaeren" ab
/// (15 Kombinationen gemessen, Scope 2026-10-10). Ein Staat ohne Einkuenfte und ohne Steuer steht nicht (`checkESt`: "Einkuenfte
/// nicht erklaert"), und nichts sperrt: es gibt nichts zu erklaeren.
#[test]
fn der_staat_steht_genau_dann_wenn_die_einkuenfte_stehen() {
    let frankreich = || Some(json!("Frankreich"));
    // (Name, Fall, Staat im XML, Einkuenfte im XML, Sperr-Eintrag zur Steuer)
    let faelle = [
        (
            "Staat und Einkuenfte",
            Ausland { staat: frankreich(), einkuenfte: true, steuer: None, abzug: false },
            true, true, false,
        ),
        (
            "Staat, Einkuenfte und Anrechnung",
            Ausland { staat: frankreich(), einkuenfte: true, steuer: Some(70_000), abzug: false },
            true, true, false,
        ),
        (
            "Staat, Einkuenfte und Abzug",
            Ausland { staat: frankreich(), einkuenfte: true, steuer: Some(70_001), abzug: true },
            true, true, false,
        ),
        (
            "nur der Staat",
            Ausland { staat: frankreich(), einkuenfte: false, steuer: None, abzug: false },
            false, false, false,
        ),
        (
            "Staat und Anrechnung ohne Einkuenfte",
            Ausland { staat: frankreich(), einkuenfte: false, steuer: Some(70_000), abzug: false },
            false, false, true,
        ),
        (
            "Staat und Abzug ohne Einkuenfte",
            Ausland { staat: frankreich(), einkuenfte: false, steuer: Some(70_001), abzug: true },
            false, false, true,
        ),
        (
            "Anrechnung ohne Staat und ohne Einkuenfte",
            Ausland { staat: None, einkuenfte: false, steuer: Some(70_000), abzug: false },
            false, false, true,
        ),
    ];
    for (name, fall, staat_steht, einkuenfte_stehen, steuer_sperrt) in faelle {
        let s = akte(&fall);
        assert_eq!(kz_wert(&s, STAAT_KZ).is_some(), staat_steht, "{name}: Staat im XML");
        assert_eq!(kz_wert(&s, EINKUENFTE_KZ).is_some(), einkuenfte_stehen, "{name}: Einkuenfte im XML");
        let r = lauf(&s);
        let alle = sperre(&r);
        let gruende = sperre_zu(&alle, STEUER);
        assert_eq!(!gruende.is_empty(), steuer_sperrt, "{name}: Sperre zur Steuer, Lauf: {r:?}");
        for grund in &gruende {
            assert!(grund.contains("Einkünfte"), "{name}: der Grund nennt die Einkuenfte nicht: {grund}");
        }
        assert!(
            sperre_zu(&alle, STAAT).is_empty(),
            "{name}: die Sperre zum Staat gilt nur mit Einkuenften"
        );
    }
    // Die Steuer steht je nach Wahl unter ihrem eigenen Kz; der Staat aendert daran nichts.
    let anrechnung = akte(&Ausland { staat: frankreich(), einkuenfte: true, steuer: Some(70_000), abzug: false });
    assert_eq!(kz_wert(&anrechnung, ANRECHNUNG_KZ).as_deref(), Some("700"));
    let abzug = akte(&Ausland { staat: frankreich(), einkuenfte: true, steuer: Some(70_001), abzug: true });
    assert_eq!(kz_wert(&abzug, ABZUG_KZ).as_deref(), Some("701"));
    assert_eq!(kz_wert(&abzug, ANRECHNUNG_KZ), None);
}

/// Die Oberflaeche laesst "Steuer ohne Einkuenfte" zu: weder die Steuer noch die Einkuenfte haengen in der Bindung an einer
/// `feld_bedingung`, jedes der beiden Felder wird also fuer sich gefragt (abgeleitet aus der Bindung, nicht an der Oberflaeche
/// gemessen). Der Fall ist erreichbar; die Sperre in `der_staat_steht_genau_dann_wenn_die_einkuenfte_stehen` faengt ihn.
#[test]
fn steuer_ohne_einkuenfte_ist_ueber_die_bindung_erreichbar() {
    for feld in [STEUER, EINKUENFTE] {
        assert!(index()[feld].feld_bedingung.is_none(), "{feld} haengt an einer feld_bedingung");
    }
}
