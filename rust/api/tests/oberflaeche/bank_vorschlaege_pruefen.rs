//! Bankimport S3 (Vault `backlog/taxgraph/bankimport-s3-oberflaeche-vorschlaege-pruefen-mit-haken-und-kreuz-abwahl`): die Seite
//! zeigt einen Kontoauszug erst als Vorschau (nichts gespeichert), die Nutzerin waehlt Posten ab, setzt die zwei Haken und
//! uebernimmt; ein Treffer darf das Eingangskreuz „keine Spenden“ abwaehlen, wenn sie es will.
//!
//! GELD UND DATEN: Was die Seite schickt (`abgewaehlt`, Haken) und was sie anzeigt, muss derselbe Stand sein. Zeigt sie eine Summe, die
//! eine aeltere Antwort traegt, oder uebernimmt sie eine andere Abwahl als die gezeigte, stimmt die Zahl der Nutzerin nicht mit der
//! Akte. Ein Zweck ist fremder Text: er steht nur als Text auf der Seite (`textContent`), nie als HTML. Das Kreuz schreibt die Seite
//! als Antwort der Nutzerin (`ui:laie`) erst nach einem erfolgreichen Uebernehmen und nur mit ihrem Haekchen; der Dienst schreibt es nie.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Handrechnung. Auszug A: Spenden 4,00 und 7,00 EUR (Summe 11,00 EUR), ein Handwerker (Nachfrage, 73,00 EUR),
//! eine Krankenversicherung (37,00 EUR, nur mit dem Haken `art9`), eine Ausgabe ohne Stichwort (9,00 EUR). Alle Texte sind erfunden.
//!
//! Grenzen: Chromium allein. Die Datei kommt per `DataTransfer` ins Eingabefeld (wie die Dateiwahl sie liefert); der Dateiwahl-Dialog
//! des Betriebssystems laeuft nicht.

use serde_json::{json, Value};

use crate::dienst::Start;
use crate::seite::Umgebung;

const SPENDEN: &str = "spenden_betrag";
const KREUZ: &str = "keine_spenden";
/// Stellen 0 und 1: Spenden; 2: Handwerker (Nachfrage); 3: Krankenversicherung (art9); 4: ohne Stichwort.
const AUSZUG_A: &[&str] = &[
    "01.03.2025;-4,00;Spende Rotes Kreuz",
    "02.03.2025;-7,00;Spende Tierheim",
    "03.03.2025;-73,00;Maler Huber",
    "04.03.2025;-37,00;Krankenversicherung Muster Kasse",
    "05.03.2025;-9,00;Miete Muster",
];

/// Wie A, dazu eine Rentenversicherung (Stelle 5): auch ohne die Spenden bleibt etwas zu uebernehmen.
const AUSZUG_B: &[&str] = &[
    "01.03.2025;-4,00;Spende Rotes Kreuz",
    "02.03.2025;-7,00;Spende Tierheim",
    "03.03.2025;-73,00;Maler Huber",
    "04.03.2025;-37,00;Krankenversicherung Muster Kasse",
    "05.03.2025;-9,00;Miete Muster",
    "06.03.2025;-24,00;Rentenversicherung Ruerup",
];

/// Der Fall im Fragebogen, das Feld fuer den Kontoauszug aufgeklappt; die Ankreuzliste ist weggeraeumt, OHNE zu antworten
/// (`keine_spenden` bleibt offen).
fn zum_kontoauszug(u: &Umgebung) -> String {
    u.zum_fragebogen_neu();
    u.s.klick("#konto-toggle");
    u.s.warte_sichtbar("#konto-file", 10);
    u.fall()
}

/// Der Nutzer waehlt eine Datei im Eingabefeld des Kontoauszugs; das Feld ist waehrend einer Anfrage gesperrt.
fn waehle(u: &Umgebung, name: &str, zeilen: &[&str]) {
    u.s.warte_bis("!document.getElementById('konto-file').disabled", 15, "das Eingabefeld des Kontoauszugs ist frei");
    let inhalt = format!("datum;betrag;verwendungszweck\n{}\n", zeilen.join("\n"));
    u.s.skript(
        "const dt = new DataTransfer(); dt.items.add(new File([arguments[1]], arguments[0])); \
         const e = document.getElementById('konto-file'); e.files = dt.files; e.dispatchEvent(new Event('change', {bubbles: true}));",
        &[json!(name), json!(inhalt)],
    );
}

/// Fall, Datei gewaehlt, Vorschau steht da.
fn vorschau(u: &Umgebung, zeilen: &[&str]) -> String {
    let fall = zum_kontoauszug(u);
    waehle(u, "auszug.csv", zeilen);
    u.s.warte_sichtbar("#konto-vorschau", 15);
    u.s.warte_bis("document.querySelectorAll('#konto-vorschau .kv-feld, #konto-vorschau .kv-nachfrage, #konto-abgleich .kv-abgleich').length > 0", 10, "die Vorschau nennt etwas");
    fall
}

fn feld_selektor(feld: &str) -> String {
    format!("#konto-vorschau .kv-feld[data-feld=\"{feld}\"]")
}

fn box_selektor(feld: &str, index: usize) -> String {
    format!("{} .kv-box[data-index=\"{index}\"]", feld_selektor(feld))
}

fn summe(u: &Umgebung, feld: &str) -> String {
    u.s.text(&format!("{} .kv-summe", feld_selektor(feld)))
}

fn warte_summe(u: &Umgebung, feld: &str, text: &str) {
    u.s.warte_bis(
        &format!(
            "(() => {{ const e = document.querySelector({}); return !!e && e.innerText.includes({}); }})()",
            serde_json::to_string(&format!("{} .kv-summe", feld_selektor(feld))).unwrap(),
            serde_json::to_string(text).unwrap()
        ),
        10,
        &format!("die Summe von {feld} nennt {text:?}"),
    );
}

fn haekchen(u: &Umgebung, id: &str) -> bool {
    u.s.ausdruck(&format!("document.getElementById('{id}').checked")) == json!(true)
}

fn uebernehmen_gesperrt(u: &Umgebung) -> bool {
    u.s.ausdruck("document.getElementById('konto-uebernehmen').disabled") == json!(true)
}

fn warte_status(u: &Umgebung, text: &str) -> String {
    u.s.warte_bis(
        &format!("document.getElementById('konto-status').textContent.includes({})", serde_json::to_string(text).unwrap()),
        15,
        &format!("die Statuszeile nennt {text:?}"),
    );
    u.s.textinhalt("#konto-status")
}

/// Die Aufrufe der Seite an den Dienst festhalten (`window.__log`: URL und Rumpf). Mit `halten` bleiben die Antworten der Vorschau
/// stehen (`window.__wartend`), bis der Test sie einzeln freigibt.
fn beobachte(u: &Umgebung, halten: bool) {
    u.s.skript(
        "window.__echt = window.__echt || jpost; window.__log = []; window.__wartend = []; window.__halten = arguments[0]; \
         jpost = (url, b) => { window.__log.push({url, body: b}); \
           if (window.__halten && url.endsWith('/bank/vorschau')) { return new Promise((ok) => { window.__wartend.push(() => ok(window.__echt(url, b))); }); } \
           return window.__echt(url, b); };",
        &[json!(halten)],
    );
}

fn aufrufe(u: &Umgebung, ende: &str) -> Vec<Value> {
    u.s.ausdruck(&format!("window.__log.filter((a) => a.url.endsWith({}))", serde_json::to_string(ende).unwrap()))
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn ereignisse(u: &Umgebung, fall: &str, feld: &str) -> Vec<Value> {
    u.dienst.ereignisse(fall, feld)
}

/// Die Vorschau zeigt alles, was der Dienst nennt, im Seitentext: Felder mit Summe, Posten mit Datum, Betrag und Zweck, die
/// Nachfrage (§ 35a), die Haken (beide aus), den Satz zur Zahl. Sie schreibt nichts: kein Ereignis, kein Kreuz, keine Meldung
/// „als Vorschlag erfasst“.
#[test]
fn bank_vorschlaege_pruefen_vorschau_zeigt_alles_und_schreibt_nichts() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);

    let text = u.s.text("#konto-vorschau");
    assert!(text.contains("noch nichts gespeichert"), "der Kopf der Vorschau fehlt: {text:?}");
    assert!(
        text.contains("Die Zahl im Ergebnis erscheint erst, wenn du die Vorschläge im Fragebogen bestätigst."),
        "der Satz zur Zahl fehlt: {text:?}"
    );
    // Die Spenden: Summe, zwei Posten mit Datum, Betrag und Zweck, beide angehakt.
    assert!(summe(&u, SPENDEN).contains("11,00 €"), "die Summe der Spenden: {:?}", summe(&u, SPENDEN));
    assert_eq!(u.s.anzahl(&format!("{} .kv-box", feld_selektor(SPENDEN))), 2);
    for (i, (datum, betrag, zweck)) in [("01.03.2025", "4,00 €", "Spende Rotes Kreuz"), ("02.03.2025", "7,00 €", "Spende Tierheim")].iter().enumerate() {
        let zeile = u.s.text(&format!("{} .kv-posten-zeile:nth-child({})", feld_selektor(SPENDEN), i + 1));
        assert!(zeile.contains(datum) && zeile.contains(betrag) && zeile.contains(zweck), "Posten {i}: {zeile:?}");
        assert!(u.s.ausdruck(&format!("document.querySelector({}).checked", serde_json::to_string(&box_selektor(SPENDEN, i)).unwrap())) == json!(true), "Posten {i} ist nicht angehakt");
    }
    // Der Handwerker ist eine Nachfrage, kein Feld und kein Kaestchen (d-61); der Text steht auf der Seite, nicht in `title`.
    let nachfrage = u.s.text("#konto-vorschau .kv-nachfrage");
    assert!(nachfrage.contains("Maler Huber") && nachfrage.contains("73,00 €") && nachfrage.contains("Arbeitskosten laut Rechnung"), "{nachfrage:?}");
    assert_eq!(u.s.anzahl("#konto-vorschau .kv-nachfrage .kv-box"), 0, "eine Nachfrage hat kein Kaestchen");
    assert!(u.s.finde(&feld_selektor("hh_handwerker_betrag")).is_none(), "der Handwerker ist ein Feld der Vorschau");
    // Die Krankenversicherung steht ohne den Haken nicht als Feld da; die Zahl der Buchungen ohne Stichwort steht beim KI-Haken.
    assert!(u.s.finde(&feld_selektor("basis_kv")).is_none());
    let ki = u.s.text("#konto-haken-ki-satz");
    assert!(ki.contains("1 Buchung(en) ohne Stichwort") && ki.contains("beim Übernehmen"), "{ki:?}");
    assert!(!haekchen(&u, "konto-haken-ki") && !haekchen(&u, "konto-haken-art9"), "ein Haken steht von sich aus");
    assert!(u.s.sichtbar("#konto-haken-ki") && u.s.sichtbar("#konto-haken-art9"));

    // Nichts geschrieben: weder ein Vorschlag noch ein Kreuz, und die Statuszeile meldet keinen Vorschlag.
    for feld in [SPENDEN, "vor_rv_ausserhalb_lstb", "hh_handwerker_betrag", "basis_kv", KREUZ] {
        assert!(ereignisse(&u, &fall, feld).is_empty(), "die Vorschau hat {feld} geschrieben");
    }
    assert!(!u.s.textinhalt("#konto-status").contains("als Vorschlag erfasst"), "{:?}", u.s.textinhalt("#konto-status"));
    let konsole = u.konsolenfehler();
    assert!(konsole.is_empty(), "Konsolenfehler: {konsole:?}");
}

/// Ein Klick auf ein Kaestchen waehlt den Posten ab: der Dienst rechnet die Summe neu, und „Uebernehmen“ schreibt genau diese
/// Summe ohne den Posten.
#[test]
fn bank_vorschlaege_pruefen_abwaehlen_rechnet_der_dienst_und_uebernehmen_schreibt_sie() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);

    u.s.klick(&box_selektor(SPENDEN, 0));
    warte_summe(&u, SPENDEN, "7,00 €");
    assert!(
        u.s.ausdruck(&format!("document.querySelector({}).checked", serde_json::to_string(&box_selektor(SPENDEN, 0)).unwrap())) == json!(false),
        "der abgewaehlte Posten ist noch angehakt"
    );
    assert!(ereignisse(&u, &fall, SPENDEN).is_empty(), "das Abwaehlen schreibt nichts");

    u.s.klick("#konto-uebernehmen");
    let status = warte_status(&u, "als Vorschlag erfasst");
    assert!(status.starts_with("1 von 5 Buchung(en)"), "nur der Tierheim-Posten geht ein: {status:?}");
    let ev = ereignisse(&u, &fall, SPENDEN);
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0]["wert"], json!(7_00), "die Summe ohne den abgewaehlten Posten");
    assert_eq!(ev[0]["signal"]["signal_1"]["posten"].as_array().unwrap().len(), 1);
    // Nach dem Uebernehmen ist die Vorschau weg.
    u.s.warte_unsichtbar("#konto-vorschau", 10);
    let konsole = u.konsolenfehler();
    assert!(konsole.is_empty(), "Konsolenfehler: {konsole:?}");
}

/// Das Kreuz ist unbeantwortet: das Kaestchen „Kreuz abwaehlen“ steht an. Nach einem erfolgreichen Uebernehmen schreibt die Seite die
/// Antwort der Nutzerin (`false` = „ja, gespendet“, `ui:laie`, bestaetigt) NACH dem Vorschlag, und die Kreuzfrage faellt aus der
/// Liste. Mit abgeschaltetem Kaestchen schreibt sie nichts.
#[test]
fn bank_vorschlaege_pruefen_kreuz_kaestchen_an_schreibt_ui_laie_nach_dem_uebernehmen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);

    assert!(u.s.sichtbar("#konto-kreuz-box"), "das Kaestchen fehlt");
    assert!(haekchen(&u, "konto-kreuz-box"), "bei einem unbeantworteten Kreuz steht das Kaestchen an");
    let satz = u.s.text("#konto-kreuz");
    assert!(satz.contains("keine Spenden") && satz.contains("Spende gefunden"), "{satz:?}");
    assert!(u.offene(&fall).contains(&KREUZ.to_owned()), "Kontrolle: die Kreuzfrage steht in der Liste");

    u.s.klick("#konto-uebernehmen");
    let status = warte_status(&u, "abgewählt");
    let kreuz = ereignisse(&u, &fall, KREUZ);
    assert_eq!(kreuz.len(), 1, "genau ein Ereignis fuer das Kreuz; Status {status:?}");
    assert_eq!(kreuz[0]["wert"], json!(false));
    assert_eq!(kreuz[0]["schreiber"], json!("ui:laie"));
    assert_eq!(kreuz[0]["zustand"], json!("bestaetigt"));
    assert_eq!(kreuz[0]["signal"]["signal_2"], json!("screening@keine_spenden"));
    // Reihenfolge: erst der Vorschlag, dann das Kreuz.
    let akte = u.dienst.akte(&fall);
    let stelle = |feld: &str| akte["events"].as_array().unwrap().iter().position(|e| e["feld_id"] == feld).unwrap();
    assert!(stelle(SPENDEN) < stelle(KREUZ), "das Kreuz steht vor dem Vorschlag");
    assert!(!u.offene(&fall).contains(&KREUZ.to_owned()), "die Kreuzfrage steht noch in der Liste");

    // Ohne Haekchen kein Kreuz (ein zweiter Fall im selben Browser).
    let fall2 = vorschau(&u, AUSZUG_A);
    u.s.klick("#konto-kreuz-box");
    assert!(!haekchen(&u, "konto-kreuz-box"));
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "als Vorschlag erfasst");
    assert_eq!(ereignisse(&u, &fall2, SPENDEN).len(), 1, "Kontrolle: der Vorschlag steht");
    assert!(ereignisse(&u, &fall2, KREUZ).is_empty(), "ohne Haekchen schreibt die Seite das Kreuz nicht");
}

/// Hat die Nutzerin selbst „keine Spenden“ gesagt (`true`), steht das Kaestchen AUS, mit dem Satz zum Widerspruch; die Seite ueberschreibt
/// ihre Antwort nur, wenn sie es anschaltet (dann mit `ersetzt` auf das alte Ereignis). `false` bekommt keine Zeile.
#[test]
fn bank_vorschlaege_pruefen_kreuz_true_kaestchen_aus_und_nur_mit_haekchen_ueberschrieben() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = zum_kontoauszug(&u);
    let (s, a) = u.dienst.schreibe(&fall, KREUZ, &json!(true));
    assert_eq!(s, 201, "{a}");
    waehle(&u, "auszug.csv", AUSZUG_A);
    u.s.warte_sichtbar("#konto-vorschau", 15);
    u.s.warte_sichtbar("#konto-kreuz-box", 10);
    assert!(!haekchen(&u, "konto-kreuz-box"), "bei `true` steht das Kaestchen an: die Seite wuerde die Antwort der Nutzerin ueberschreiben");
    let satz = u.s.text("#konto-kreuz");
    assert!(satz.contains("Du hattest") && satz.contains("keine Spenden") && satz.contains("angekreuzt"), "{satz:?}");

    // Ohne Haekchen bleibt das Kreuz, wie es war.
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "als Vorschlag erfasst");
    assert_eq!(ereignisse(&u, &fall, KREUZ).len(), 1, "die Seite hat das Kreuz ohne Haekchen geaendert");
    assert_eq!(u.dienst.letzter_wert(&fall, KREUZ), json!(true));

    // Mit Haekchen: ein neues Ereignis ersetzt das alte.
    let fall2 = zum_kontoauszug(&u);
    let (s, a) = u.dienst.schreibe(&fall2, KREUZ, &json!(true));
    assert_eq!(s, 201, "{a}");
    waehle(&u, "auszug.csv", AUSZUG_A);
    u.s.warte_sichtbar("#konto-kreuz-box", 15);
    u.s.klick("#konto-kreuz-box");
    assert!(haekchen(&u, "konto-kreuz-box"));
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "als Vorschlag erfasst");
    warte_status(&u, "abgewählt");
    let kreuz = ereignisse(&u, &fall2, KREUZ);
    assert_eq!(kreuz.iter().map(|e| e["wert"].clone()).collect::<Vec<_>>(), [json!(true), json!(false)]);
    assert_eq!(kreuz[1]["ersetzt"], kreuz[0]["event_id"], "das neue Ereignis verweist nicht auf das alte");
    assert_eq!(kreuz[1]["schreiber"], json!("ui:laie"));

    // `false`: keine Zeile.
    let fall3 = zum_kontoauszug(&u);
    let (s, _) = u.dienst.schreibe(&fall3, KREUZ, &json!(false));
    assert_eq!(s, 201);
    waehle(&u, "auszug.csv", AUSZUG_A);
    u.s.warte_sichtbar("#konto-vorschau", 15);
    u.s.warte_bis("document.querySelectorAll('#konto-vorschau .kv-feld').length > 0", 10, "die Vorschau steht");
    assert!(!u.s.sichtbar("#konto-kreuz-box"), "bei `false` gibt es nichts abzuwaehlen");
}

/// Faellt das Schreiben des Kreuzes aus, zeigt die Statuszeile den Fehler, und das Kreuz bleibt unberuehrt: kein stiller Erfolg.
#[test]
fn bank_vorschlaege_pruefen_kreuz_schreiben_scheitert_die_seite_nennt_den_fehler() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    u.s.skript(
        "window.__echt = jpost; \
         jpost = (url, b) => url.endsWith('/event') ? Promise.resolve({status: 422, body: {fehler: 'Testabweisung des Kreuzes'}}) : window.__echt(url, b);",
        &[],
    );
    u.s.klick("#konto-uebernehmen");
    let status = warte_status(&u, "Testabweisung des Kreuzes");
    assert!(status.contains("Kreuz") && status.contains("nicht abgewählt"), "{status:?}");
    assert!(!status.contains("wurde abgewählt"), "ein stiller Erfolg: {status:?}");
    assert_eq!(ereignisse(&u, &fall, SPENDEN).len(), 1, "der Vorschlag steht (das Uebernehmen ging durch)");
    assert!(ereignisse(&u, &fall, KREUZ).is_empty(), "das Kreuz wurde trotz des Fehlers geschrieben");
}

/// Ein Zweck ist fremder Text: `<b>x</b>` steht als Text auf der Seite, nie als Element (Mutant: `innerHTML`).
#[test]
fn bank_vorschlaege_pruefen_zweck_mit_html_steht_als_text_da() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, &["01.03.2025;-4,00;Spende <b>x</b> Tierheim", "03.03.2025;-73,00;Maler <i>y</i> Huber"]);
    let zeile = u.s.text(&format!("{} .kv-posten-zeile", feld_selektor(SPENDEN)));
    assert!(zeile.contains("<b>x</b>"), "der Zweck steht nicht als Text da: {zeile:?}");
    let nachfrage = u.s.text("#konto-vorschau .kv-nachfrage");
    assert!(nachfrage.contains("<i>y</i>"), "{nachfrage:?}");
    assert_eq!(u.s.anzahl("#konto-vorschau b, #konto-vorschau i"), 0, "der Zweck ist als Element in die Seite gekommen");
}

/// Eine veraltete Antwort wird verworfen: zwei schnelle Klicks auf zwei Kaestchen, die Antwort auf den zweiten kommt zuerst, die auf
/// den ersten danach. Die Seite zeigt den Stand des zweiten und uebernimmt genau diese Abwahl.
#[test]
fn bank_vorschlaege_pruefen_eine_veraltete_antwort_wird_verworfen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_B);
    beobachte(&u, true);
    u.s.klick(&box_selektor(SPENDEN, 0));
    u.s.klick(&box_selektor(SPENDEN, 1));
    u.s.warte_bis("window.__wartend.length === 2", 10, "beide Anfragen sind unterwegs");
    // Zuerst die Antwort auf die zweite Anfrage (abgewaehlt 0 und 1), dann die auf die erste (nur 0).
    u.s.skript("window.__wartend[1]();", &[]);
    warte_summe(&u, SPENDEN, "0,00 €");
    u.s.skript("window.__wartend[0]();", &[]);
    // Die alte Antwort braucht einen Augenblick; danach darf sich nichts geaendert haben.
    u.s.auswerten("async () => { await new Promise((ok) => setTimeout(ok, 700)); return true; }", &[]);
    assert!(summe(&u, SPENDEN).contains("0,00 €"), "die veraltete Antwort ueberschrieb die neuere: {:?}", summe(&u, SPENDEN));
    for i in [0, 1] {
        assert!(
            u.s.ausdruck(&format!("document.querySelector({}).checked", serde_json::to_string(&box_selektor(SPENDEN, i)).unwrap())) == json!(false),
            "Posten {i} ist wieder angehakt"
        );
    }
    // Uebernehmen sendet genau die gezeigte Abwahl.
    u.s.skript("window.__halten = false;", &[]);
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "als Vorschlag erfasst");
    let senden = aufrufe(&u, "/kontoauszug");
    assert_eq!(senden.len(), 1);
    assert_eq!(senden[0]["body"]["abgewaehlt"], json!([0, 1]), "{}", senden[0]);
    assert!(ereignisse(&u, &fall, SPENDEN).is_empty(), "beide Spenden waren abgewaehlt");
    assert_eq!(ereignisse(&u, &fall, "vor_rv_ausserhalb_lstb").len(), 1, "Kontrolle: die Vorsorge ging ein");
}

/// „Uebernehmen“ ist gesperrt, solange eine Anfrage der Vorschau laeuft; ein erzwungener Aufruf sendet nichts.
#[test]
fn bank_vorschlaege_pruefen_uebernehmen_ist_gesperrt_solange_eine_anfrage_laeuft() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    assert!(!uebernehmen_gesperrt(&u), "Kontrolle: mit einer Vorschau steht der Knopf frei");
    beobachte(&u, true);
    u.s.klick(&box_selektor(SPENDEN, 0));
    u.s.warte_bis("window.__wartend.length === 1", 10, "die Anfrage ist unterwegs");
    assert!(uebernehmen_gesperrt(&u), "waehrend die Anfrage laeuft, ist der Knopf frei");
    u.s.auswerten("async () => { document.getElementById('konto-uebernehmen').disabled = false; await kontoauszugUebernehmen(); return true; }", &[]);
    assert!(aufrufe(&u, "/kontoauszug").is_empty(), "ein erzwungener Aufruf sendet, obwohl eine Anfrage laeuft");
    assert_eq!(ereignisse(&u, &fall, SPENDEN), Vec::<Value>::new());
    u.s.skript("window.__halten = false; window.__wartend[0]();", &[]);
    warte_summe(&u, SPENDEN, "7,00 €");
    u.s.warte_bis("!document.getElementById('konto-uebernehmen').disabled", 10, "der Knopf ist wieder frei");
}

/// Der Haken `art9` sendet die Vorschau neu, mit denselben Abwahlen (die Kennung haengt nicht vom Haken ab); die Krankenversicherung
/// steht dann als Feld da, ohne Zweck. „Uebernehmen“ schickt den Haken mit. Der KI-Haken sendet keine neue Vorschau (sie ruft die KI nie),
/// geht aber beim Uebernehmen mit.
#[test]
fn bank_vorschlaege_pruefen_art9_haken_sendet_neu_und_ki_haken_gilt_beim_uebernehmen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    beobachte(&u, false);
    u.s.klick(&box_selektor(SPENDEN, 0));
    warte_summe(&u, SPENDEN, "7,00 €");
    let vor = aufrufe(&u, "/bank/vorschau").len();

    u.s.klick("#konto-haken-art9");
    u.s.warte_bis(&format!("document.querySelectorAll({}).length === 1", serde_json::to_string(&feld_selektor("basis_kv")).unwrap()), 10, "die Krankenversicherung steht als Feld da");
    let alle = aufrufe(&u, "/bank/vorschau");
    assert_eq!(alle.len(), vor + 1, "der Haken sendet genau eine neue Vorschau");
    let letzte = alle.last().unwrap();
    assert_eq!(letzte["body"]["art9"], json!(true), "{letzte}");
    assert_eq!(letzte["body"]["abgewaehlt"], json!([0]), "der Haken darf die Abwahl nicht verlieren: {letzte}");
    let kv = u.s.text(&feld_selektor("basis_kv"));
    assert!(kv.contains("37,00 €") && !kv.contains("Krankenversicherung Muster"), "kein Buchungstext in der Zeile: {kv:?}");
    assert!(summe(&u, SPENDEN).contains("7,00 €"), "die Abwahl der Spende gilt weiter");

    // Der KI-Haken sendet nichts neu.
    u.s.klick("#konto-haken-ki");
    assert!(haekchen(&u, "konto-haken-ki"));
    assert_eq!(aufrufe(&u, "/bank/vorschau").len(), vor + 1, "der KI-Haken sendet eine Vorschau");

    u.s.klick("#konto-uebernehmen");
    let status = warte_status(&u, "als Vorschlag erfasst");
    let senden = aufrufe(&u, "/kontoauszug");
    assert_eq!(senden.len(), 1);
    assert_eq!(
        (senden[0]["body"]["art9"].clone(), senden[0]["body"]["ki_buchungstext"].clone(), senden[0]["body"]["abgewaehlt"].clone()),
        (json!(true), json!(true), json!([0])),
        "{}",
        senden[0]
    );
    // Ohne Anbieter sagt der Dienst, dass die KI nicht eingerichtet ist; die Seite nennt es.
    assert!(status.contains("nicht eingerichtet"), "{status:?}");
    let kv = ereignisse(&u, &fall, "basis_kv");
    assert_eq!(kv.len(), 1);
    assert_eq!(kv[0]["signal"]["signal_1"]["posten"][0]["zweck_maskiert"], json!(""), "der Buchungstext steht in der Akte");
}

/// „Verwerfen“ leert die Vorschau und den gemerkten Auszug; dieselbe Datei noch einmal zu waehlen zeigt sie wieder.
#[test]
fn bank_vorschlaege_pruefen_verwerfen_leert_vorschau_und_auszug() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    u.s.klick("#konto-verwerfen");
    u.s.warte_unsichtbar("#konto-vorschau", 10);
    assert_eq!(u.s.ausdruck("KONTO_LETZTER"), Value::Null, "der Auszug ist noch gemerkt");
    assert_eq!(u.s.ausdruck("document.getElementById('konto-file').value"), json!(""), "die Dateiwahl ist nicht geleert");
    assert_eq!(ereignisse(&u, &fall, SPENDEN), Vec::<Value>::new());
    // Ein erzwungenes Uebernehmen ohne Auszug sendet nichts.
    beobachte(&u, false);
    u.s.auswerten("async () => { document.getElementById('konto-uebernehmen').disabled = false; await kontoauszugUebernehmen(); return true; }", &[]);
    assert_eq!(aufrufe(&u, "/kontoauszug"), Vec::<Value>::new());
    // Dieselbe Datei noch einmal: die Vorschau steht wieder da.
    waehle(&u, "auszug.csv", AUSZUG_A);
    u.s.warte_sichtbar("#konto-vorschau", 15);
    warte_summe(&u, SPENDEN, "11,00 €");
}

/// Nichts zu uebernehmen (nur eine Nachfrage, oder ein Feld mit Beleg): der Knopf ist gesperrt, die Vorschau nennt den Abgleich
/// mit Feld und Kontosumme im Seitentext.
#[test]
fn bank_vorschlaege_pruefen_nichts_zu_uebernehmen_und_abgleich_stehen_im_seitentext() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = zum_kontoauszug(&u);
    // Das Feld traegt schon einen Wert aus einer Eingabe (Beleg schlaegt Konto).
    let (s, a) = u.dienst.schreibe(&fall, SPENDEN, &json!(50_00));
    assert_eq!(s, 201, "{a}");
    waehle(&u, "auszug.csv", &["01.03.2025;-4,00;Spende Rotes Kreuz", "03.03.2025;-73,00;Maler Huber"]);
    u.s.warte_sichtbar("#konto-vorschau", 15);
    u.s.warte_bis("document.querySelectorAll('#konto-abgleich .kv-abgleich').length > 0", 10, "der Abgleich steht da");
    let abgleich = u.s.text("#konto-abgleich");
    assert!(abgleich.contains("Spenden") && abgleich.contains("4,00 €") && abgleich.contains("Beleg"), "{abgleich:?}");
    assert!(u.s.finde(&feld_selektor(SPENDEN)).is_none(), "ein belegtes Feld ist kein Feld der Vorschau");
    assert!(uebernehmen_gesperrt(&u), "es gibt nichts zu uebernehmen");
    assert!(u.s.text("#konto-vorschau").contains("nichts zu übernehmen"), "{:?}", u.s.text("#konto-vorschau"));
    assert_eq!(ereignisse(&u, &fall, SPENDEN).len(), 1, "der Wert des Belegs steht allein");
}
