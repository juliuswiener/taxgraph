//! C2 (§ 32 Abs. 6 Satz 3 Nr. 1 `EStG`, gebaut 2026-10-06): der Weg der beiden neuen Antworten ins
//! amtliche XML und durch den Schreibpfad des Dienstes, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Ein Elternteil, der nicht mit dem anderen zusammen veranlagt wird, bekommt den
//! Kinderfreibetrag des anderen dazu, wenn dieser verstorben ist oder im Ausland lebte. Dafuer fragt die
//! Software je Kind zwei Dinge: den Todestag und den Zeitraum im Ausland. Beides gehoert in die
//! Steuererklaerung, in den Abschnitt `Weit_Ang` der Anlage Kind.
//!
//! **Warum es zaehlt.** Landen die Antworten nicht im XML, rechnet die Software den doppelten Freibetrag,
//! die Erklaerung traegt ihn aber nicht: das Finanzamt rechnet anders als die Software. Landet ein Wert
//! im XML, den das Schema nicht annimmt, weist die ERiC-Pruefung die ganze Einreichung ab.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_kap_vv_familie.yaml` bindet die Kennzeichen `E0501102`
//! (Todestag) und `E0503903` (Ausland-Zeitraum); der XML-Schreiber baut die Elemente aus dem amtlichen
//! Schema, eine Pfadtabelle gibt es nicht. Die Rechnung liest die Antworten in
//! `rust/bescheid/src/zweige/kinderfreibetrag.rs`.
//!
//! Gemessen am 2026-10-06 (Schema `E10-2025`, `ERiC` 44.2.4.0, `xmllint`): ein XML mit beiden Antworten ist
//! schemagueltig; `Weit_Ang` darf ohne den Geschwister-Abschnitt `Ang_Pers` stehen (beide sind im Schema
//! optional). Ob die ERiC-PRUEFUNG (nicht das Schema) diese Auslassung annimmt, kann der Test nicht
//! messen: `checkESt` braucht eine Hersteller-ID, die das Repo nicht hat.
//!
//! Das Schema liegt nur in der lokalen ERiC-Auslieferung (`~/02_Software/eric`), nie im Repo. Ohne
//! Schema baut der Writer kein XML (`XmlNichtBaubar`), in der CI gilt `TAXGRAPH_OHNE_XSD=1`: die Tests
//! melden den Verzicht auf stderr, statt ihn zu verschweigen. Der Schreibpfad-Test und die Sperren
//! laufen ohne Schema.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::path::{Path, PathBuf};

use bescheid::deklaration::{einreichungs_xml, EinreichFehler};
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, PruefTiefe, Sperrgrund, Zustand};
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const TOD: &str = "kind_anderer_elternteil_tod_am";
const AUSLAND: &str = "kind_anderer_elternteil_ausland_zeitraum";
const GANZ: &str = "01.01-31.12";

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json")
}

/// Die Akte des E2E-Falls `gesamt` (Einzelveranlagung, Vermietung, Handwerker) mit EINEM Kind: ohne
/// `fam_anzahl_kinder` liest die Rechnung keine Kind-Instanz, und die Sperre wuerde nie greifen.
fn akte() -> Store {
    let mut s = Store::aus_datei(store::lade(&fixture()).unwrap());
    s.append_roh(&antwort("fam_anzahl_kinder", json!(1)), None, BindungNachschlag::neu(index()))
        .unwrap();
    s
}

fn antwort(feld_id: &str, wert: Value) -> NeuesEventRoh {
    NeuesEventRoh {
        feld_id: feld_id.to_owned(),
        wert: wert.into(),
        zustand: Zustand::Bestaetigt,
        herkunft: domain::HerkunftVektor::Voll(Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        }),
        schreiber: "ui:laie".to_owned(),
        signal: Signal {
            signal_1: Some(None),
            signal_2: Some(format!("ok@{feld_id}")),
            signal_2_fehlt: false,
        },
        signal_2_fremd: None,
        ersetzt: None,
        ts: None,
    }
}

/// Schreibweg WIE DER DIENST: die Bindung liegt an, Auflage T (Wertepalette) und M (Muster) wirken.
fn setze(s: &mut Store, feld_id: &str, wert: Value) -> Result<(), String> {
    s.append_roh(
        &antwort(feld_id, wert),
        None,
        BindungNachschlag::neu(index()),
    )
    .map(|_| ())
    .map_err(|a| format!("{a:?}"))
}

fn bau_xml(s: &Store) -> Result<String, EinreichFehler> {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned())).map(|e| e.xml)
}

/// Der Abschnitt `Weit_Ang` der Akte, oder `None`, wenn es ihn nicht gibt.
fn weit_ang(xml: &str) -> Option<String> {
    xml.find("<Weit_Ang>")
        .map(|i| xml[i..].chars().take(400).collect::<String>())
}

/// Trifft in einem XML-Block ein Element mit diesem Namen, und welches? `None` = kein Element.
fn element(block: &str, name: &str) -> Option<String> {
    let offen = format!("<{name}>");
    let selbst = format!("<{name} />");
    if let Some(i) = block.find(&offen) {
        let rest = &block[i + offen.len()..];
        return Some(rest.split('<').next().unwrap_or_default().to_owned());
    }
    if block.contains(&selbst) {
        return Some(String::new());
    }
    None
}

#[test]
fn die_beiden_angaben_gehen_ins_xml_und_bleiben_gueltig() {
    let ohne = bau_xml(&akte());
    let mut s = akte();
    setze(&mut s, TOD, json!("31.12.2024")).unwrap();
    setze(&mut s, AUSLAND, json!("01.01-30.06")).unwrap();
    let mit = bau_xml(&s);
    if !schemas_da(2025) {
        // Kein Schema: der Writer baut nicht (bestehendes Verhalten aller E2E-Faelle), und die
        // Aussage "gueltig" waere blind. Der Verzicht wird gemeldet, nicht uebersprungen.
        for e in [&ohne, &mit] {
            assert!(
                matches!(e, Err(EinreichFehler::XmlNichtBaubar(_))),
                "ohne Schema: {e:?}"
            );
        }
        eprintln!("C2-Weit_Ang: kein lokales Schema, beide Akten enden am Writer (verzoegert geprueft)");
        return;
    }
    let xml = mit.unwrap_or_else(|f| panic!("mit den beiden Antworten: {f}"));
    let block = weit_ang(&xml).expect("<Weit_Ang> fehlt im XML");
    assert_eq!(element(&block, "E0501102").as_deref(), Some("31.12.2024"));
    assert_eq!(element(&block, "E0503903").as_deref(), Some("01.01-30.06"));
    let (ok, meldung) = elster::validiere_xsd_text(xml.as_bytes(), domain::Vz::Vz2025);
    assert!(ok, "das XML mit Weit_Ang ist nicht schemagueltig: {meldung}");
    assert!(
        xml.find("<Weit_Ang>").is_some_and(|i| xml.find("</Kind>").unwrap_or(0) > i),
        "Weit_Ang liegt nicht im Kind-Abschnitt"
    );
}

/// Ohne Antwort fehlt der Abschnitt: die Akte von heute (die beiden Felder ungefragt) schreibt
/// weder `E0501102` noch `E0503903`, und ihr XML ist unveraendet gueltig. Ein `null` oder ein leerer
/// Text erreicht den Store auf dem echten Schreibweg gar nicht (dritter Test): die Wertepalette
/// eines `datum`-Feldes nimmt beides nicht an, Auflage M weist den leeren Text des Zeitraums ab.
#[test]
fn ohne_antwort_fehlt_der_abschnitt() {
    let a = bau_xml(&akte());
    if !schemas_da(2025) {
        eprintln!("C2-Weit_Ang: ohne Schema kein XML, die Akte bleibt ohne die beiden Antworten");
        return;
    }
    let xml = a.unwrap_or_else(|f| panic!("Akte ohne die beiden Antworten: {f}"));
    assert!(weit_ang(&xml).is_none(), "Weit_Ang ohne jede Antwort: {}", weit_ang(&xml).unwrap_or_default());
}

/// Der Schreibpfad des Dienstes (mit Bindung, wie `POST /event`): leeren Text und `null` weist er
/// fuer beide neuen Felder ab -- wie beim bestehenden Geburtsdatum desselben Zweigs. Einen
/// kalenderrisch unmoeglichen Tag laesst Auflage M durch (das Muster prueft nur Ziffern): die
/// Sperre sitzt in der Rechnung, nicht im Store. Das Ausland-Muster prueft auch die Monate,
/// `01.13-31.12` ist dort aussen vor.
#[test]
fn der_schreibpfad_weist_leeren_text_ab_und_laesst_einen_unmoglichen_tag_zu() {
    for feld in [TOD, AUSLAND, "kind_anderer_elternteil_geburtsdatum"] {
        for wert in [json!(""), Value::Null] {
            let mut s = akte();
            assert!(
                setze(&mut s, feld, wert.clone()).is_err(),
                "{feld}: {wert} wurde angenommen"
            );
        }
    }
    for (feld, wert) in [(TOD, "31.02.2025"), (TOD, "29.02.2025"), (TOD, "15.07.2025"), (AUSLAND, GANZ)] {
        let mut s = akte();
        assert!(
            setze(&mut s, feld, json!(wert)).is_ok(),
            "{feld} {wert}: der Schreibpfad sollte durchlassen (die Sperre rechnet)"
        );
    }
    let mut s = akte();
    assert!(setze(&mut s, AUSLAND, json!("01.13-31.12")).is_err(), "Monat 13 gegen das Muster");
    // Durchgelassen heisst: die Einreichung sperrt mit dem eigenen Grund, statt ein XML mit einem
    // Datum zu bauen, das es nicht gibt (der Guard laeuft vor dem Writer).
    let mut s = akte();
    setze(&mut s, TOD, json!("31.02.2025")).unwrap();
    assert!(
        matches!(bau_xml(&s), Err(EinreichFehler::Gesperrt(Sperrgrund::KindZeitraumUnlesbar))),
        "ein unmoeglicher Todestag: {:?}",
        bau_xml(&s).err()
    );
    let mut s = akte();
    setze(&mut s, TOD, json!("29.02.2025")).unwrap();
    assert!(
        matches!(bau_xml(&s), Err(EinreichFehler::Gesperrt(Sperrgrund::KindZeitraumUnlesbar))),
        "der 29.02. im Nicht-Schaltjahr des Steuerjahres: {:?}",
        bau_xml(&s).err()
    );
    // Der 29.02.2024 dagegen: lesbar (2024 ist Schaltjahr) und vor dem Steuerjahr, also volles Jahr.
    let mut s = akte();
    setze(&mut s, TOD, json!("29.02.2024")).unwrap();
    assert!(bau_xml(&s).is_ok() || !schemas_da(2025), "ein Todestag im Schaltjahr 2024");
}

/// Der andere Elternteil wird nur bei Einzelveranlagung gefragt (`feld_bedingung`), und sein
/// Kennzeichen-Zweig gilt im Schema nur fuer nicht zusammen veranlagte Eltern. Traegt eine Akte die
/// beiden Antworten trotzdem (Vorjahr uebernommen, oder die Veranlagungsart erst spaeter beantwortet),
/// aendern sie nichts: die Einreichung endet auf denselben Fehler wie ohne sie, und kein Weg
/// beschwert sich ueber ein unbekanntes Feld.
#[test]
fn zusammenveranlagung_liest_die_beiden_angaben_nicht() {
    let mit = {
        let mut s = akte();
        setze(&mut s, TOD, json!("31.12.2024")).unwrap();
        setze(&mut s, AUSLAND, json!(GANZ)).unwrap();
        auf_zusammen(&mut s);
        bau_xml(&s)
    };
    let ohne = {
        let mut s = akte();
        auf_zusammen(&mut s);
        bau_xml(&s)
    };
    if schemas_da(2025) {
        // Ohne die Partnerangaben des Zusammen-Kegels bleibt die Einreichung gesperrt -- an der
        // Veranlagung, nicht an den beiden Antworten.
        for e in [&mit, &ohne] {
            let text = format!("{e:?}");
            assert!(
                matches!(e, Err(EinreichFehler::Gesperrt(_))) && !text.contains("nicht in der Bindungstabelle"),
                "die beiden Antworten sind in der Zusammenveranlagung ein unbekanntes Feld: {text}"
            );
        }
    }
}

/// `veranlagung` auf derselben Akte auf `zusammen` umbiegen (ersetzt das aktive Event, wie der
/// Nutzer es tue).
fn auf_zusammen(s: &mut Store) {
    let vid = s
        .aktive()
        .find(|(fid, _)| *fid == "veranlagung")
        .map(|(_, e)| e.event_id.to_string())
        .expect("KONTROLLE: die E2E-Akte trägt veranlagung");
    let mut um = antwort("veranlagung", json!("zusammen"));
    um.ersetzt = Some(vid);
    s.append_roh(&um, None, BindungNachschlag::neu(index())).unwrap();
}
