//! `POST /fall/{id}/bank/vorschau` (Bankimport S3, Rust-eigen: kein Gegenstück in Python): derselbe Auszug, dieselbe Rechnung wie
//! `kontoauszug`, aber auf einer KOPIE der Akte. Die Antwort nennt je Feld die Summe, die der Import schriebe, und die Posten mit
//! ihrer Kennung (die Stelle in der gelesenen Liste), damit die Nutzerin einzelne abwählt, bevor etwas in die Akte geht.
//!
//! Die Vorschau schreibt nicht, weil sie es nicht kann: der Handler bekommt den Fall nur lesend, rechnet auf einer Kopie des Stores und
//! ruft das Sprachmodell nie. Ein Wächter am Quelltext (`tests/kontoauszug_vorschau.rs`) hält das fest: dieses Modul nennt weder das
//! Speichern noch den schreibenden Zugriff noch das Protokoll noch eine Datei.
use eingang::kontoauszug::{uebernehme_auswahl, Ergebnis, Folge, Zeile};
use serde_json::{json, Value};
use store::BindungNachschlag;

use crate::antwort::Antwort;
use crate::eigener_fall::EigenerFall;
use crate::fehler::ApiFehler;
use crate::kontoauszug::{bericht, kreuz_abwahl, lese_fehler, lies_anfrage};
use crate::zustand::Zustand as Dienst;

/// Die Folge eines Felds als Text der Antwort.
const fn folge_text(folge: Folge) -> &'static str {
    match folge {
        Folge::Geschrieben => "geschrieben",
        Folge::Ersetzbar => "ersetzbar",
        Folge::SaldoNichtPositiv => "saldo_nicht_positiv",
        Folge::SchonErfasst => "schon_erfasst",
    }
}

/// Ein Posten der Vorschau; das Feld steht beim Elternteil. `quelle`: `ki`, wenn der Klassifikator die Buchung einordnete.
fn posten_json(z: &Zeile) -> Value {
    json!({"index": z.index, "datum": z.datum, "betrag_cent": z.betrag_cent, "zweck_maskiert": z.zweck_maskiert,
        "regel_id": z.regel_id, "quelle": if z.ueber_ki { "ki" } else { "stichwort" }, "abgewaehlt": z.abgewaehlt})
}

/// Die Felder in der Reihenfolge, in der ihre erste Buchung im Auszug steht. Ein Feld, dessen Posten alle abgewählt sind, hat keine
/// Gruppe und keine Folge im Ergebnis: es steht mit Summe 0 und der Folge `abgewaehlt` da, damit die Seite die Posten wieder anbietet.
fn felder_json(erg: &Ergebnis) -> Value {
    let mut ids: Vec<&str> = Vec::new();
    for zeile in &erg.zeilen {
        if !ids.contains(&zeile.feld_id) {
            ids.push(zeile.feld_id);
        }
    }
    let felder = ids.into_iter().map(|id| {
        let (summe, anzahl, folge) = erg
            .felder
            .iter()
            .find(|f| f.feld_id == id)
            .map_or((0, 0, "abgewaehlt"), |f| (f.summe_cent, f.anzahl, folge_text(f.folge)));
        let posten: Vec<Value> = erg.zeilen.iter().filter(|z| z.feld_id == id).map(posten_json).collect();
        json!({"feld_id": id, "summe_cent": summe, "anzahl": anzahl, "folge": folge, "posten": posten})
    });
    Value::Array(felder.collect())
}

/// Der Rumpf nach dem Owner-Check: derselbe Rumpf wie bei `kontoauszug` (auch `abgewaehlt` und die Haken), gerechnet auf einer Kopie.
///
/// # Errors
/// Dieselben wie `kontoauszug`: 400 für Format, Inhalt und `abgewaehlt`, 422 für ein PDF, das nicht zu lesen ist.
pub fn vorschau(z: &Dienst, fall: &EigenerFall, body: &Value) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(fall.store())?;
    let a = lies_anfrage(body)?;
    let katalog = z.katalog()?;
    let mut kopie = fall.store().clone();
    // Ohne Klassifikator: das Sprachmodell sieht beim Vorschauen nie einen Buchungstext, auch nicht mit dem Haken `ki_buchungstext`.
    let erg = uebernehme_auswahl(
        &mut kopie,
        &a.tx,
        a.stellen(),
        BindungNachschlag::neu(&sb.index),
        None,
        (None, Some(&katalog)),
        a.haken(),
    )
    .map_err(|e| lese_fehler(&e))?;
    let (mut aus, hinweise) = bericht(&a, &erg);
    aus.insert("vorschau".into(), json!(true));
    aus.insert("felder".into(), felder_json(&erg));
    aus.insert("ohne_stichwort".into(), json!(erg.uebernahme.ohne_stichwort));
    aus.insert(
        "haken".into(),
        json!({"ersetzen": a.ersetzen, "art9": a.art9, "ki_buchungstext": a.ki_buchungstext}),
    );
    // Die Vorschau nennt das Kreuz auch ohne das Feld `abgewaehlt`: die Seite fragt es nicht eigens an.
    if let Some(kreuz) = kreuz_abwahl(fall.store(), &erg.felder) {
        aus.insert("kreuz_abwahl".into(), kreuz);
    }
    if !hinweise.is_empty() {
        aus.insert("hinweis".into(), json!(hinweise.join(" ")));
    }
    Ok(Antwort::neu(200, Value::Object(aus)))
}
