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

/// „Verwerfen“ gilt auch fuer eine Anfrage, die noch laeuft: ihre Antwort bringt die Vorschau nicht zurueck.
#[test]
fn bank_vorschlaege_pruefen_verwerfen_waehrend_eine_anfrage_laeuft_zeigt_die_antwort_nicht_mehr() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, AUSZUG_A);
    beobachte(&u, true);
    u.s.klick(&box_selektor(SPENDEN, 0));
    u.s.warte_bis("window.__wartend.length === 1", 10, "die Anfrage ist unterwegs");
    u.s.klick("#konto-verwerfen");
    u.s.warte_unsichtbar("#konto-vorschau", 10);
    u.s.skript("window.__halten = false; window.__wartend[0]();", &[]);
    // Die Antwort braucht einen Augenblick; danach darf die Vorschau nicht wieder da sein.
    u.s.auswerten("async () => { await new Promise((ok) => setTimeout(ok, 700)); return true; }", &[]);
    assert!(!u.s.sichtbar("#konto-vorschau"), "die Antwort brachte die verworfene Vorschau zurueck");
    assert_eq!(u.s.ausdruck("KONTO_LETZTER"), Value::Null);
    assert!(uebernehmen_gesperrt(&u), "ohne Vorschau steht „Uebernehmen“ frei");
}

/// Laeuft „Uebernehmen“, sind Knopf, Verwerfen, Dateiwahl und Kaestchen gesperrt, und ein erzwungener zweiter Aufruf sendet nichts
/// (Doppel-Submit: derselbe Auszug ginge sonst zweimal an den Dienst).
#[test]
fn bank_vorschlaege_pruefen_uebernehmen_laeuft_sperrt_alles_und_ein_zweiter_aufruf_sendet_nichts() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    u.s.skript(
        "window.__echt = jpost; window.__weiter = null; window.__n = 0; \
         jpost = (url, b) => { if (url.endsWith('/kontoauszug')) { window.__n += 1; \
           return new Promise((ok) => { window.__weiter = () => ok(window.__echt(url, b)); }); } return window.__echt(url, b); };",
        &[],
    );
    u.s.klick("#konto-uebernehmen");
    u.s.warte_bis("window.__weiter !== null", 10, "Uebernehmen ist unterwegs");
    for (id, was) in [("konto-uebernehmen", "Uebernehmen"), ("konto-verwerfen", "Verwerfen"), ("konto-file", "die Dateiwahl")] {
        assert!(u.s.ausdruck(&format!("document.getElementById('{id}').disabled")) == json!(true), "{was} ist waehrend des Uebernehmens frei");
    }
    assert!(
        u.s.ausdruck("document.querySelector('#konto-vorschau .kv-box').disabled") == json!(true),
        "ein Kaestchen ist waehrend des Uebernehmens frei"
    );
    u.s.skript("document.getElementById('konto-uebernehmen').disabled = false; kontoauszugUebernehmen(); kontoauszugVerwerfen();", &[]);
    assert_eq!(u.s.ausdruck("window.__n"), json!(1), "der zweite Aufruf sendet den Auszug noch einmal");
    assert_ne!(u.s.ausdruck("KONTO_LETZTER"), Value::Null, "Verwerfen griff in das laufende Uebernehmen ein");
    u.s.skript("jpost = window.__echt; window.__weiter();", &[]);
    warte_status(&u, "als Vorschlag erfasst");
    u.s.warte_bis("!document.getElementById('konto-file').disabled", 15, "die Dateiwahl ist wieder frei");
    assert_eq!(ereignisse(&u, &fall, SPENDEN).len(), 1, "der Dienst schrieb nicht genau einmal");
}

/// Die Abwahl gilt auch fuer die Rueckfrage des Ersetzens (Bankimport S1b, d-63): „Ersetzen“ sendet den Auszug mit denselben Haken und
/// Abwahlen noch einmal. Ginge die Abwahl verloren, stuende 19,00 statt 16,00 EUR in der Akte.
#[test]
fn bank_vorschlaege_pruefen_abwahl_gilt_auch_fuer_die_rueckfrage_des_ersetzens() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = zum_kontoauszug(&u);
    waehle(&u, "auszug.csv", &["01.03.2025;-4,00;Spende Rotes Kreuz", "02.03.2025;-7,00;Spende Tierheim"]);
    crate::konto_ersetzen::uebernimm_vorschau(&u);
    warte_status(&u, "2 von 2 Buchung(en) als Vorschlag erfasst");
    assert_eq!(ereignisse(&u, &fall, SPENDEN).len(), 1);

    // Zweiter Auszug: 7,00 EUR gibt es schon, dazu 5,00 und 3,00 EUR; die 3,00 EUR (Stelle 2) wählt die Nutzerin ab.
    waehle(&u, "auszug.csv", &["02.03.2025;-7,00;Spende Tierheim", "03.03.2025;-5,00;Spende Unicef", "04.03.2025;-3,00;Spende Caritas"]);
    u.s.warte_sichtbar("#konto-vorschau", 15);
    warte_summe(&u, SPENDEN, "19,00 €");
    u.s.klick(&box_selektor(SPENDEN, 2));
    warte_summe(&u, SPENDEN, "16,00 €");
    crate::konto_ersetzen::uebernimm_vorschau(&u);
    let status = warte_status(&u, "Ersetzen möglich");
    assert!(status.contains("16,00 €") && !status.contains("19,00 €"), "{status:?}");
    u.s.klick("#konto-ersetzen-haken");
    u.s.klick("#konto-ersetzen-go");
    warte_status(&u, "Ersetzt:");
    let ev = ereignisse(&u, &fall, SPENDEN);
    assert_eq!(ev.iter().map(|e| e["wert"].clone()).collect::<Vec<_>>(), [json!(11_00), json!(16_00)], "die Abwahl ging beim Ersetzen verloren");
    assert_eq!(ev[1]["signal"]["signal_1"]["posten"].as_array().unwrap().len(), 3);
}

/// Das Kreuz aendert sich zwischen Vorschau und „Uebernehmen“ (die Nutzerin beantwortet es in einem anderen Fenster mit „ja, gespendet“):
/// die Antwort des Dienstes nennt kein Kreuz mehr, also schreibt die Seite keins, auch wenn ihr Kaestchen noch angehakt ist. Sonst
/// kaeme eine zweite Antwort ohne Bezug auf die erste ins Protokoll der Nutzerin oder die Seite meldete einen Fehler, den es nicht gibt.
#[test]
fn bank_vorschlaege_pruefen_kreuz_aenderte_sich_nach_der_vorschau_die_seite_schreibt_keins() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    assert!(haekchen(&u, "konto-kreuz-box"), "Kontrolle: das Kaestchen steht an");
    let (s, a) = u.dienst.schreibe(&fall, KREUZ, &json!(false));
    assert_eq!(s, 201, "{a}");
    u.s.klick("#konto-uebernehmen");
    let status = warte_status(&u, "als Vorschlag erfasst");
    u.s.warte_bis("!document.getElementById('konto-file').disabled", 15, "das Uebernehmen ist zu Ende");
    let status = format!("{status} || {}", u.s.textinhalt("#konto-status"));
    assert!(!status.contains("Kreuz"), "die Seite nennt ein Kreuz, das der Dienst nicht nannte: {status:?}");
    let kreuz = ereignisse(&u, &fall, KREUZ);
    assert_eq!(kreuz.len(), 1, "nur die Antwort aus dem anderen Fenster; die Seite schrieb ein zweites: {kreuz:?}");
    assert_eq!(kreuz[0]["schreiber"], json!("ui:laie"));
    assert_eq!(ereignisse(&u, &fall, SPENDEN).len(), 1, "Kontrolle: der Vorschlag steht");
    // Der Lauf endet sauber (Mutant J36: `kontoKreuzAbwaehlen(undefined)` wirft einen TypeError, den niemand faengt): kein Skriptfehler in der
    // Konsole, und die Statuszeile nennt den Vorschlag ohne Satz zum Kreuz.
    u.s.auswerten("async () => { await new Promise((ok) => setTimeout(ok, 400)); return true; }", &[]);
    let konsole = u.konsolenfehler();
    assert!(konsole.is_empty(), "Konsolenfehler: {konsole:?}");
}

/// Eine Vorschau, die der Dienst abweist (hier eine JSON-Datei, die kein JSON ist), laesst weder Vorschau noch gemerkten Auszug noch
/// die Dateiwahl stehen: dieselbe Datei noch einmal zu waehlen loest wieder eine Anfrage aus, und „Uebernehmen“ bleibt gesperrt.
#[test]
fn bank_vorschlaege_pruefen_abgewiesene_vorschau_leert_auszug_und_dateiwahl() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = zum_kontoauszug(&u);
    beobachte(&u, false);
    let kaputt = |u: &Umgebung| {
        u.s.warte_bis("!document.getElementById('konto-file').disabled", 15, "das Eingabefeld ist frei");
        u.s.skript(
            "const dt = new DataTransfer(); dt.items.add(new File(['[{'], 'kaputt.json')); \
             const e = document.getElementById('konto-file'); e.files = dt.files; e.dispatchEvent(new Event('change', {bubbles: true}));",
            &[],
        );
    };
    kaputt(&u);
    warte_status(&u, "Upload fehlgeschlagen");
    u.s.warte_bis("!document.getElementById('konto-file').disabled", 15, "das Eingabefeld ist wieder frei");
    assert_eq!(u.s.ausdruck("KONTO_LETZTER"), Value::Null, "der abgewiesene Auszug ist noch gemerkt");
    assert_eq!(u.s.ausdruck("document.getElementById('konto-file').value"), json!(""), "die Dateiwahl ist nicht geleert");
    assert!(!u.s.sichtbar("#konto-vorschau"));
    assert!(uebernehmen_gesperrt(&u));
    assert_eq!(aufrufe(&u, "/bank/vorschau").len(), 1);
    // Dieselbe Datei noch einmal: eine zweite Anfrage geht raus (bliebe die Dateiwahl gefuellt, loeste sie kein `change` aus).
    kaputt(&u);
    u.s.warte_bis("window.__log.length === 2", 10, "die zweite Anfrage ist raus");
    assert_eq!(ereignisse(&u, &fall, SPENDEN), Vec::<Value>::new());
}

/// Eine Rueckbuchung (Betrag mit Plus, gleiche Kategorie) steht in der Vorschau als Rueckbuchung, nicht als Ausgabe, und die Ausgabe ohne
/// Vorzeichen; die Summe ist der Saldo (50,00 − 30,00 = 20,00 EUR).
#[test]
fn bank_vorschlaege_pruefen_rueckbuchung_steht_als_rueckbuchung_da() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, &["01.03.2025;-50,00;Rentenversicherung Ruerup", "02.03.2025;30,00;Rueckbuchung Rentenversicherung Ruerup"]);
    let feld = "vor_rv_ausserhalb_lstb";
    assert!(summe(&u, feld).contains("20,00 €"), "der Saldo: {:?}", summe(&u, feld));
    let ausgabe = u.s.text(&format!("{} .kv-posten-zeile:nth-child(1)", feld_selektor(feld)));
    let rueck = u.s.text(&format!("{} .kv-posten-zeile:nth-child(2)", feld_selektor(feld)));
    assert!(ausgabe.contains("50,00 €") && !ausgabe.contains("Rückbuchung") && !ausgabe.contains('+') && !ausgabe.contains('-'), "{ausgabe:?}");
    assert!(rueck.contains("+30,00 €") && rueck.contains("Rückbuchung"), "{rueck:?}");
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
    // Die Seite schickt keinen Betrag und keine Summe, nur den Auszug, die Haken und die Stellen: die Summe rechnet der Dienst.
    let mut schluessel: Vec<&str> = senden[0]["body"].as_object().unwrap().keys().map(String::as_str).collect();
    schluessel.sort_unstable();
    assert_eq!(schluessel, ["abgewaehlt", "art9", "format", "inhalt", "ki_buchungstext"], "{}", senden[0]);
    let vorschau_aufrufe = aufrufe(&u, "/bank/vorschau");
    let mut schluessel: Vec<&str> = vorschau_aufrufe[0]["body"].as_object().unwrap().keys().map(String::as_str).collect();
    schluessel.sort_unstable();
    assert_eq!(schluessel, ["abgewaehlt", "art9", "format", "inhalt"], "die Vorschau schickt den KI-Haken nie: {}", vorschau_aufrufe[0]);
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

// ---------------------------------------------------------------------------------------------------------------------------
// Nachtrag zu den Mutanten von main (P13, J5, J16 bis J23): Zustand der Seite zwischen den Auszuegen, der Satz zur leeren Vorschau, das
// Ereignis des Kreuzes Feld fuer Feld.
// ---------------------------------------------------------------------------------------------------------------------------

fn kaestchen_an(u: &Umgebung, feld: &str, index: usize) -> bool {
    u.s.ausdruck(&format!("document.querySelector({}).checked", serde_json::to_string(&box_selektor(feld, index)).unwrap())) == json!(true)
}

/// Warten, bis die Vorschau eines frisch gewaehlten Auszugs steht (die alte ist beim Waehlen sofort weg).
fn warte_auf_neue_vorschau(u: &Umgebung) {
    u.s.warte_sichtbar("#konto-vorschau", 15);
    u.s.warte_bis("document.querySelectorAll('#konto-vorschau .kv-feld').length > 0", 10, "die neue Vorschau nennt ein Feld");
}

/// Der Satz „Es gibt nichts zu uebernehmen.“ steht genau dann da, wenn nichts zu uebernehmen ist: mit Inhalt unsichtbar, ohne Inhalt
/// sichtbar, und er folgt der Abwahl in beide Richtungen (Mutant J5: `hidden = false`).
#[test]
fn bank_vorschlaege_pruefen_satz_nichts_zu_uebernehmen_steht_nur_ohne_inhalt_da() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, AUSZUG_A);
    assert!(!u.s.sichtbar("#konto-leer"), "der Satz steht da, obwohl die Spenden zu uebernehmen sind");
    assert!(!u.s.text("#konto-vorschau").contains("nichts zu übernehmen"), "{:?}", u.s.text("#konto-vorschau"));
    assert!(!uebernehmen_gesperrt(&u));
    // Beide Spenden abgewaehlt: nur die Nachfrage bleibt, es gibt nichts zu uebernehmen.
    u.s.klick(&box_selektor(SPENDEN, 0));
    warte_summe(&u, SPENDEN, "7,00 €");
    u.s.klick(&box_selektor(SPENDEN, 1));
    warte_summe(&u, SPENDEN, "0,00 €");
    u.s.warte_sichtbar("#konto-leer", 10);
    assert!(u.s.text("#konto-vorschau").contains("nichts zu übernehmen"), "{:?}", u.s.text("#konto-vorschau"));
    assert!(uebernehmen_gesperrt(&u));
    // Eine Spende wieder angewaehlt: der Satz geht, der Knopf ist frei.
    u.s.klick(&box_selektor(SPENDEN, 1));
    warte_summe(&u, SPENDEN, "7,00 €");
    u.s.warte_unsichtbar("#konto-leer", 10);
    assert!(!uebernehmen_gesperrt(&u));
}

/// Das Haekchen der Nutzerin am Kreuz bleibt, wenn sie danach einen Posten abwaehlt und die Vorschau neu entsteht (Mutant J16:
/// `KONTO_KREUZ_WAHL = null`). Beide Richtungen: Vorgabe an (Kreuz unbeantwortet) und von ihr weggenommen; Vorgabe aus (Kreuz `true`) und
/// von ihr angesetzt.
#[test]
fn bank_vorschlaege_pruefen_kreuz_wahl_der_nutzerin_bleibt_bei_neuer_vorschau() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, AUSZUG_A);
    assert!(haekchen(&u, "konto-kreuz-box"), "Kontrolle: bei einem unbeantworteten Kreuz steht das Kaestchen an");
    u.s.klick("#konto-kreuz-box");
    assert!(!haekchen(&u, "konto-kreuz-box"));
    u.s.klick(&box_selektor(SPENDEN, 0));
    warte_summe(&u, SPENDEN, "7,00 €");
    assert!(u.s.sichtbar("#konto-kreuz-box"), "das Kaestchen fehlt in der neuen Vorschau");
    assert!(!haekchen(&u, "konto-kreuz-box"), "die neue Vorschau setzt das Haekchen zurueck, das die Nutzerin weggenommen hat");

    let fall2 = zum_kontoauszug(&u);
    let (s, a) = u.dienst.schreibe(&fall2, KREUZ, &json!(true));
    assert_eq!(s, 201, "{a}");
    waehle(&u, "auszug.csv", AUSZUG_A);
    u.s.warte_sichtbar("#konto-kreuz-box", 15);
    assert!(!haekchen(&u, "konto-kreuz-box"), "Kontrolle: bei `true` steht das Kaestchen aus");
    u.s.klick("#konto-kreuz-box");
    assert!(haekchen(&u, "konto-kreuz-box"));
    u.s.klick(&box_selektor(SPENDEN, 0));
    warte_summe(&u, SPENDEN, "7,00 €");
    assert!(haekchen(&u, "konto-kreuz-box"), "die neue Vorschau nimmt das Haekchen weg, das die Nutzerin gesetzt hat");
}

/// Die Wahl am Kreuz gilt nur fuer diesen Auszug: nach einer neuen Dateiwahl und nach „Verwerfen“ gilt wieder die Vorgabe nach dem
/// Stand des Kreuzes (Mutant J22b: `KONTO_KREUZ_WAHL = null` fehlt in `kontoVorschauWeg`).
#[test]
fn bank_vorschlaege_pruefen_kreuz_wahl_gilt_nicht_fuer_den_naechsten_auszug() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, AUSZUG_A);
    u.s.klick("#konto-kreuz-box");
    assert!(!haekchen(&u, "konto-kreuz-box"));
    // Andere Datei: die Vorgabe (an) gilt wieder.
    waehle(&u, "auszug.csv", AUSZUG_B);
    warte_auf_neue_vorschau(&u);
    warte_summe(&u, SPENDEN, "11,00 €");
    assert!(haekchen(&u, "konto-kreuz-box"), "die Wahl zum alten Auszug gilt fuer den neuen");
    // „Verwerfen“, dann dieselbe Datei noch einmal: ebenso.
    u.s.klick("#konto-kreuz-box");
    assert!(!haekchen(&u, "konto-kreuz-box"));
    u.s.klick("#konto-verwerfen");
    u.s.warte_unsichtbar("#konto-vorschau", 10);
    waehle(&u, "auszug.csv", AUSZUG_A);
    warte_auf_neue_vorschau(&u);
    warte_summe(&u, SPENDEN, "11,00 €");
    assert!(haekchen(&u, "konto-kreuz-box"), "die Wahl vor „Verwerfen“ gilt fuer den naechsten Auszug");
}

/// Die Abwahl gilt nur fuer diesen Auszug (Mutant J22: `KONTO_ABGEWAEHLT = new Set()` fehlt in `kontoVorschauWeg`): nach einer neuen
/// Dateiwahl sind alle Kaestchen an, die Summe stimmt, und „Uebernehmen“ sendet `abgewaehlt: []`.
#[test]
fn bank_vorschlaege_pruefen_abwahl_gilt_nicht_fuer_die_naechste_datei() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    u.s.klick(&box_selektor(SPENDEN, 0));
    warte_summe(&u, SPENDEN, "7,00 €");
    beobachte(&u, false);
    waehle(&u, "auszug.csv", AUSZUG_B);
    warte_auf_neue_vorschau(&u);
    warte_summe(&u, SPENDEN, "11,00 €");
    for i in [0, 1] {
        assert!(kaestchen_an(&u, SPENDEN, i), "Kaestchen {i} steht nach der neuen Dateiwahl aus");
    }
    let vorschauen = aufrufe(&u, "/bank/vorschau");
    assert_eq!(vorschauen.last().unwrap()["body"]["abgewaehlt"], json!([]), "die neue Vorschau traegt die Abwahl des alten Auszugs: {}", vorschauen.last().unwrap());
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "als Vorschlag erfasst");
    let senden = aufrufe(&u, "/kontoauszug");
    assert_eq!(senden.len(), 1);
    assert_eq!(senden[0]["body"]["abgewaehlt"], json!([]), "{}", senden[0]);
    assert_eq!(ereignisse(&u, &fall, SPENDEN)[0]["wert"], json!(11_00), "beide Spenden gehen ein");
}

/// Wie oben, mit „Verwerfen“ statt der neuen Datei: danach derselbe Auszug noch einmal, alles angehakt.
#[test]
fn bank_vorschlaege_pruefen_abwahl_gilt_nicht_nach_verwerfen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    u.s.klick(&box_selektor(SPENDEN, 1));
    warte_summe(&u, SPENDEN, "4,00 €");
    u.s.klick("#konto-verwerfen");
    u.s.warte_unsichtbar("#konto-vorschau", 10);
    beobachte(&u, false);
    waehle(&u, "auszug.csv", AUSZUG_A);
    warte_auf_neue_vorschau(&u);
    warte_summe(&u, SPENDEN, "11,00 €");
    for i in [0, 1] {
        assert!(kaestchen_an(&u, SPENDEN, i), "Kaestchen {i} steht nach „Verwerfen“ aus");
    }
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "als Vorschlag erfasst");
    assert_eq!(aufrufe(&u, "/kontoauszug")[0]["body"]["abgewaehlt"], json!([]));
    assert_eq!(ereignisse(&u, &fall, SPENDEN)[0]["wert"], json!(11_00));
}

/// Die Haken gelten je Import (d-59, d-60: KI und Art. 9 nur mit Haken je Import): nach einer neuen Dateiwahl und nach „Verwerfen“ sind
/// beide aus, die neue Vorschau sendet `art9: false`, und „Uebernehmen“ sendet beide als `false`. Mutanten J23 (`art9`) und J23b (KI).
#[test]
fn bank_vorschlaege_pruefen_haken_gelten_je_import_und_bleiben_nicht_stehen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    let art9_feld = serde_json::to_string(&feld_selektor("basis_kv")).unwrap();
    u.s.klick("#konto-haken-art9");
    u.s.warte_bis(&format!("document.querySelectorAll({art9_feld}).length === 1"), 10, "die Krankenversicherung steht als Feld da");
    u.s.klick("#konto-haken-ki");
    assert!(haekchen(&u, "konto-haken-art9") && haekchen(&u, "konto-haken-ki"), "Kontrolle: beide Haken stehen");

    // Neue Dateiwahl: beide aus, die Vorschau ohne die Krankenversicherung.
    beobachte(&u, false);
    waehle(&u, "auszug.csv", AUSZUG_A);
    warte_auf_neue_vorschau(&u);
    assert!(!haekchen(&u, "konto-haken-art9"), "der Art.-9-Haken gilt fuer den naechsten Import");
    assert!(!haekchen(&u, "konto-haken-ki"), "der KI-Haken gilt fuer den naechsten Import");
    assert!(u.s.finde(&feld_selektor("basis_kv")).is_none(), "die neue Vorschau wertet Art. 9 aus");
    assert_eq!(aufrufe(&u, "/bank/vorschau").last().unwrap()["body"]["art9"], json!(false));

    // „Verwerfen“: beide aus, sofort.
    u.s.klick("#konto-haken-art9");
    u.s.warte_bis(&format!("document.querySelectorAll({art9_feld}).length === 1"), 10, "die Krankenversicherung steht wieder da");
    u.s.klick("#konto-haken-ki");
    u.s.klick("#konto-verwerfen");
    assert!(!haekchen(&u, "konto-haken-art9"), "der Art.-9-Haken bleibt nach „Verwerfen“ stehen");
    assert!(!haekchen(&u, "konto-haken-ki"), "der KI-Haken bleibt nach „Verwerfen“ stehen");
    waehle(&u, "auszug.csv", AUSZUG_A);
    warte_auf_neue_vorschau(&u);
    assert!(u.s.finde(&feld_selektor("basis_kv")).is_none());
    assert_eq!(aufrufe(&u, "/bank/vorschau").last().unwrap()["body"]["art9"], json!(false));
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "als Vorschlag erfasst");
    let senden = aufrufe(&u, "/kontoauszug");
    assert_eq!(senden.len(), 1);
    assert_eq!((senden[0]["body"]["art9"].clone(), senden[0]["body"]["ki_buchungstext"].clone()), (json!(false), json!(false)), "{}", senden[0]);
    assert_eq!(ereignisse(&u, &fall, "basis_kv"), Vec::<Value>::new(), "die Krankenversicherung wurde ohne Haken geschrieben");
}

/// Jedes Feld des Ereignisses, das die Seite fuer das Kreuz schreibt, einzeln (Mutanten J17, J18b, J18c und die Verwandten): Wert `false`
/// („ja, gespendet“), bestaetigt, Schreiber `ui:laie`, Herkunft `laie`, Pruefung `ungeprueft`, Haftung `nutzer`, Beleg `screening@…`.
fn pruefe_kreuz_ereignis(e: &Value) {
    assert_eq!(e["feld_id"], json!(KREUZ), "{e}");
    assert_eq!(e["wert"], json!(false), "{e}");
    assert_eq!(e["zustand"], json!("bestaetigt"), "{e}");
    assert_eq!(e["schreiber"], json!("ui:laie"), "{e}");
    assert_eq!(e["herkunft"]["herkunft"], json!("laie"), "{e}");
    assert_eq!(e["herkunft"]["pruef_tiefe"], json!("ungeprueft"), "{e}");
    assert_eq!(e["herkunft"]["haftung"], json!("nutzer"), "{e}");
    assert_eq!(e["signal"]["signal_2"], json!("screening@keine_spenden"), "{e}");
}

/// Das Kreuz-Ereignis im Fall: ohne Vorgaenger (`aktuell: null`) ohne `ersetzt` und mit `signal_1: null`; hat die Nutzerin das Kreuz selbst
/// gesetzt (`true`), tragen `ersetzt` UND `signal_1` die Kennung des alten Ereignisses.
#[test]
fn bank_vorschlaege_pruefen_kreuz_ereignis_traegt_jedes_feld_der_antwort_der_nutzerin() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(&u, AUSZUG_A);
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "abgewählt");
    let kreuz = ereignisse(&u, &fall, KREUZ);
    assert_eq!(kreuz.len(), 1, "{kreuz:?}");
    pruefe_kreuz_ereignis(&kreuz[0]);
    assert_eq!(kreuz[0]["signal"]["signal_1"], Value::Null, "{}", kreuz[0]);
    assert!(kreuz[0].get("ersetzt").is_none_or(Value::is_null), "ein erstes Kreuz ersetzt nichts: {}", kreuz[0]);

    let fall2 = zum_kontoauszug(&u);
    let (s, a) = u.dienst.schreibe(&fall2, KREUZ, &json!(true));
    assert_eq!(s, 201, "{a}");
    let alt = ereignisse(&u, &fall2, KREUZ)[0]["event_id"].clone();
    assert!(alt.is_string(), "Kontrolle: das alte Ereignis hat eine Kennung");
    waehle(&u, "auszug.csv", AUSZUG_A);
    u.s.warte_sichtbar("#konto-kreuz-box", 15);
    u.s.klick("#konto-kreuz-box");
    assert!(haekchen(&u, "konto-kreuz-box"));
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "abgewählt");
    let kreuz = ereignisse(&u, &fall2, KREUZ);
    assert_eq!(kreuz.len(), 2, "{kreuz:?}");
    pruefe_kreuz_ereignis(&kreuz[1]);
    assert_eq!(kreuz[1]["ersetzt"], alt, "{}", kreuz[1]);
    assert_eq!(kreuz[1]["signal"]["signal_1"], alt, "signal_1 traegt nicht die Kennung des alten Ereignisses: {}", kreuz[1]);
}

/// Nach der zweiten Dateiwahl zeigt die Seite nie die Posten und Summen des ersten Auszugs, auch nicht, solange die neue Vorschau noch
/// aussteht (Mutant J24: `kontoVorschauWeg()` fehlt in `kontoauszugHochladen`). Die gehaltene Antwort ist wie im Test der veralteten Antwort.
#[test]
fn bank_vorschlaege_pruefen_zweite_dateiwahl_zeigt_nie_die_posten_des_ersten_auszugs() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, AUSZUG_A);
    beobachte(&u, true);
    waehle(&u, "auszug.csv", AUSZUG_B);
    u.s.warte_bis("window.__wartend.length === 1", 15, "die Vorschau des zweiten Auszugs ist unterwegs");
    assert!(!u.s.sichtbar("#konto-vorschau"), "die Vorschau des ersten Auszugs steht, waehrend die zweite aussteht");
    assert_eq!(u.s.anzahl("#konto-vorschau .kv-feld"), 0, "Felder des ersten Auszugs stehen noch da");
    assert_eq!(u.s.ausdruck("KONTO_VORSCHAU"), Value::Null);
    assert!(uebernehmen_gesperrt(&u), "ohne Vorschau steht „Uebernehmen“ frei");
    u.s.skript("window.__halten = false; window.__wartend[0]();", &[]);
    warte_auf_neue_vorschau(&u);
    warte_summe(&u, SPENDEN, "11,00 €");
    assert!(u.s.finde(&feld_selektor("vor_rv_ausserhalb_lstb")).is_some(), "die Vorschau des zweiten Auszugs fehlt (Stelle 5: Rentenversicherung)");
}

/// Der Hinweis des Dienstes steht im Seitentext (Mutant J44: `#konto-hinweis` bleibt leer). Der Wortlaut kommt aus dem Fall: die
/// Krankenversicherung ohne Haken ist gesperrt (S2, `hinweis_art9_gesperrt`), mit dem Haken nennt der Regelhinweis den Basisanteil; ohne
/// Besonderheit bleibt das Feld leer.
#[test]
fn bank_vorschlaege_pruefen_hinweis_des_dienstes_steht_im_seitentext() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, AUSZUG_A);
    let hinweis = u.s.text("#konto-hinweis");
    assert!(hinweis.contains("1 Buchung(en) mit Bezug zu Gesundheit, Gewerkschaft oder Kirche wurden nicht ausgewertet"), "{hinweis:?}");
    assert!(hinweis.contains("Arbeitskosten laut Rechnung"), "der Satz zu § 35a fehlt: {hinweis:?}");
    u.s.klick("#konto-haken-art9");
    u.s.warte_bis("document.getElementById('konto-hinweis').innerText.includes('Basisanteil laut Beitragsbescheinigung')", 10, "der Regelhinweis steht da");
    let hinweis = u.s.text("#konto-hinweis");
    assert!(!hinweis.contains("wurden nicht ausgewertet"), "der Satz zur Sperre steht noch da: {hinweis:?}");

    vorschau(&u, &["01.03.2025;-4,00;Spende Rotes Kreuz"]);
    assert_eq!(u.s.text("#konto-hinweis"), "", "ohne Besonderheit bleibt der Hinweis leer");
}

/// Die Zeile einer § 35a-Zahlung im Wortlaut: Feld, Datum, Betrag OHNE Vorzeichen, Zweck (Mutant J48: `kontoEuro(n.betrag_cent)`).
#[test]
fn bank_vorschlaege_pruefen_nachfrage_zeile_nennt_den_betrag_ohne_vorzeichen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, AUSZUG_A);
    let zeile = u.s.text("#konto-vorschau .kv-nachfrage");
    assert!(
        zeile.contains("Handwerker (§ 35a): 03.03.2025, 73,00 €, Maler Huber. Eingetragen wird nur der Anteil der Arbeitskosten laut Rechnung"),
        "{zeile:?}"
    );
    assert!(!zeile.contains("-73"), "{zeile:?}");
}

/// Der Abgleich im Wortlaut mit Summe UND Anzahl (Mutant J49: die Anzahl nennt die Summe in Cent): zwei Spenden, 4,00 und 7,00 EUR, auf einem
/// Feld mit Wert.
#[test]
fn bank_vorschlaege_pruefen_abgleich_nennt_summe_und_anzahl_der_buchungen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = zum_kontoauszug(&u);
    let (s, a) = u.dienst.schreibe(&fall, SPENDEN, &json!(50_00));
    assert_eq!(s, 201, "{a}");
    waehle(&u, "auszug.csv", &["01.03.2025;-4,00;Spende Rotes Kreuz", "02.03.2025;-7,00;Spende Tierheim"]);
    u.s.warte_sichtbar("#konto-vorschau", 15);
    u.s.warte_bis("document.querySelectorAll('#konto-abgleich .kv-abgleich').length > 0", 10, "der Abgleich steht da");
    let abgleich = u.s.text("#konto-abgleich");
    assert!(
        abgleich.contains("Spenden: Das Feld hat schon einen Wert (Beleg oder deine Eingabe), der Kontoauszug ändert ihn nicht. Das Konto ergäbe 11,00 € aus 2 Buchung(en)."),
        "{abgleich:?}"
    );
}

/// Die Folge eines Felds steht als Satz da, auch die der Abwahl aller Buchungen (Mutant J52: ihr Satz fehlt), und eine negative Summe
/// traegt ihr Minus (Mutant J45: `kontoEuro` ohne Vorzeichen): Rentenversicherung 30,00 EUR Ausgabe, 50,00 EUR Rueckbuchung, Saldo -20,00 EUR.
#[test]
fn bank_vorschlaege_pruefen_folge_je_feld_steht_als_satz_da_und_die_negative_summe_hat_ihr_minus() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = vorschau(
        &u,
        &[
            "01.03.2025;-4,00;Spende Rotes Kreuz",
            "02.03.2025;-7,00;Spende Tierheim",
            "03.03.2025;-30,00;Rentenversicherung Ruerup",
            "04.03.2025;50,00;Rueckbuchung Rentenversicherung Ruerup",
        ],
    );
    let vorsorge = feld_selektor("vor_rv_ausserhalb_lstb");
    assert!(summe(&u, "vor_rv_ausserhalb_lstb").contains("-20,00 €"), "die negative Summe: {:?}", summe(&u, "vor_rv_ausserhalb_lstb"));
    assert!(u.s.text(&vorsorge).contains("Kein Vorschlag: Die Rückbuchungen gleichen die Ausgaben aus."), "{:?}", u.s.text(&vorsorge));
    // Geschrieben: der Satz der Spenden.
    assert!(u.s.text(&feld_selektor(SPENDEN)).contains("Wird beim Übernehmen als Vorschlag erfasst."), "{:?}", u.s.text(&feld_selektor(SPENDEN)));
    // Alle Posten abgewaehlt.
    u.s.klick(&box_selektor(SPENDEN, 0));
    warte_summe(&u, SPENDEN, "7,00 €");
    u.s.klick(&box_selektor(SPENDEN, 1));
    warte_summe(&u, SPENDEN, "0,00 €");
    assert!(u.s.text(&feld_selektor(SPENDEN)).contains("Kein Vorschlag: Du hast alle Buchungen abgewählt."), "{:?}", u.s.text(&feld_selektor(SPENDEN)));
    // Beide wieder angewaehlt, uebernommen; ein zweiter Auszug trifft das Feld mit Vorschlag: der Satz zum Ersetzen.
    u.s.klick(&box_selektor(SPENDEN, 0));
    u.s.klick(&box_selektor(SPENDEN, 1));
    warte_summe(&u, SPENDEN, "11,00 €");
    u.s.klick("#konto-uebernehmen");
    warte_status(&u, "2 von 4 Buchung(en) als Vorschlag erfasst");
    assert_eq!(ereignisse(&u, &fall, SPENDEN).len(), 1);
    waehle(&u, "auszug.csv", &["02.03.2025;-7,00;Spende Tierheim", "05.03.2025;-5,00;Spende Unicef"]);
    warte_auf_neue_vorschau(&u);
    warte_summe(&u, SPENDEN, "16,00 €");
    assert!(
        u.s.text(&feld_selektor(SPENDEN)).contains("Das Feld trägt schon einen Vorschlag aus einem früheren Auszug. Ersetzt wird erst nach deiner Rückfrage."),
        "{:?}",
        u.s.text(&feld_selektor(SPENDEN))
    );
}

/// Eine Buchung ohne Zweck steht mit dem Ersatztext da, als Posten und als Nachfrage-Zeile (Mutanten J51 und J51b). Der Posten der
/// Krankenversicherung traegt mit dem Haken `art9` nie einen Zweck. Eine Nachfrage ohne Zweck liefert der Dienst heute nie (das
/// Stichwort steht im Zweck); die Antwort des Dienstes ist hier nachgestellt, damit die Seite weder „undefined“ noch eine Luecke zeigt.
#[test]
fn bank_vorschlaege_pruefen_buchung_ohne_zweck_steht_mit_ersatztext_da() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    vorschau(&u, AUSZUG_A);
    u.s.klick("#konto-haken-art9");
    u.s.warte_bis(&format!("document.querySelectorAll({}).length === 1", serde_json::to_string(&feld_selektor("basis_kv")).unwrap()), 10, "die Krankenversicherung steht als Feld da");
    let kv = u.s.text(&format!("{} .kv-posten-zeile", feld_selektor("basis_kv")));
    assert!(kv.contains("37,00 €") && kv.contains("(Text wird nicht angezeigt)"), "{kv:?}");

    u.s.auswerten(
        "async () => { kontoRendern({felder: [], nachfragen: [{feld_id: 'hh_handwerker_betrag', datum: '2025-03-03', betrag_cent: -7300, zweck_maskiert: ''}], \
         abgleich: [], transaktionen: 1, ohne_stichwort: 0}); return true; }",
        &[],
    );
    let zeile = u.s.text("#konto-vorschau .kv-nachfrage");
    assert!(zeile.contains("73,00 €, (Text wird nicht angezeigt). Eingetragen wird nur"), "{zeile:?}");
}

/// Hinter dem gesperrten Knopf steht die Sperre „keine Vorschau“ (Mutant J26c): In der Rueckfrage des Ersetzens ist der Auszug noch
/// gemerkt (`KONTO_LETZTER`), die Vorschau aber weg und keine Anfrage unterwegs. Per Klick ist der Knopf dort gesperrt; ein erzwungener
/// Aufruf sendet trotzdem nichts, sonst gingen Haken und Abwahl des gemerkten Auszugs verloren.
#[test]
fn bank_vorschlaege_pruefen_uebernehmen_ohne_vorschau_sendet_nichts_auch_in_der_rueckfrage() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    zum_kontoauszug(&u);
    waehle(&u, "auszug.csv", &["01.03.2025;-4,00;Spende Rotes Kreuz", "02.03.2025;-7,00;Spende Tierheim"]);
    crate::konto_ersetzen::uebernimm_vorschau(&u);
    warte_status(&u, "2 von 2 Buchung(en) als Vorschlag erfasst");
    waehle(&u, "auszug.csv", &["02.03.2025;-7,00;Spende Tierheim", "03.03.2025;-5,00;Spende Unicef"]);
    crate::konto_ersetzen::uebernimm_vorschau(&u);
    warte_status(&u, "Ersetzen möglich");
    assert_ne!(u.s.ausdruck("KONTO_LETZTER"), Value::Null, "Kontrolle: die Rueckfrage haelt den Auszug");
    assert_eq!((u.s.ausdruck("KONTO_VORSCHAU"), u.s.ausdruck("KONTO_OFFEN")), (Value::Null, json!(0)), "Kontrolle: keine Vorschau, keine Anfrage");
    assert!(uebernehmen_gesperrt(&u), "per Klick ist der Knopf dort gesperrt");
    beobachte(&u, false);
    u.s.auswerten("async () => { document.getElementById('konto-uebernehmen').disabled = false; await kontoauszugUebernehmen(); return true; }", &[]);
    assert_eq!(aufrufe(&u, "/kontoauszug"), Vec::<Value>::new(), "ein erzwungener Aufruf sendet ohne Vorschau");
}
