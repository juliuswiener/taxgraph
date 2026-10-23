//! Rueckfrage mit Haken, bevor ein zweiter Kontoauszug den eigenen Vorschlag eines frueheren ersetzt (Bankimport S1b,
//! Entscheidung d-63 von Julius, 2026-10-22: „Ja, aber erst nach Rueckfrage mit Haken“).
//!
//! Der Server ersetzt nur mit `ersetzen: true` (Tests in `kontoauszug_naht.rs`); hier steht, was die Seite daraus macht: die
//! Rueckfrage nennt alten und neuen Betrag, der Knopf „Ersetzen“ tut erst mit gesetztem Haken etwas, und der Haken gilt nur fuer
//! den Auszug, den die Rueckfrage nannte. GELD: ersetzt die Seite ohne Haken oder mit einem Haken fuer einen anderen Auszug, ist
//! ein Vorschlag weg, den der Nutzer behalten wollte.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Handrechnung. Erster Auszug 4,00 + 7,00 = 11,00 EUR, zweiter 7,00 (dieselbe Buchung) + 5,00:
//! Vereinigung 4,00 + 7,00 + 5,00 = 16,00 EUR aus 3 Buchungen. Die Saetze der Rueckfrage stehen so in `kontoauszug_naht.rs`.
//!
//! Grenzen: Chromium allein. Die Datei kommt per `DataTransfer` ins Eingabefeld (wie die Dateiwahl sie liefert); der
//! Dateiwahl-Dialog des Betriebssystems laeuft nicht.

use serde_json::{json, Value};

use crate::dienst::Start;
use crate::seite::Umgebung;

const FELD: &str = "spenden_betrag";
const ERSTER: &[&str] = &["01.02.2025;-4,00;Spende Rotes Kreuz", "02.02.2025;-7,00;Spende Tierheim"];
const ZWEITER: &[&str] = &["02.02.2025;-7,00;Spende Tierheim", "03.02.2025;-5,00;Spende Unicef"];

/// Der Nutzer waehlt eine Datei `name` mit diesem Inhalt im Eingabefeld des Kontoauszugs. Das Feld ist waehrend eines Uploads
/// gesperrt (Doppel-Submit-Schutz); der Nutzer wartet, bis es wieder frei ist.
fn waehle_datei(u: &Umgebung, name: &str, inhalt: &str) {
    u.s.warte_bis("!document.getElementById('konto-file').disabled", 15, "das Eingabefeld des Kontoauszugs ist frei");
    u.s.skript(
        "const dt = new DataTransfer(); dt.items.add(new File([arguments[1]], arguments[0])); \
         const e = document.getElementById('konto-file'); e.files = dt.files; e.dispatchEvent(new Event('change', {bubbles: true}));",
        &[json!(name), json!(inhalt)],
    );
}

/// Eine CSV-Datei mit diesen Zeilen.
fn waehle_auszug(u: &Umgebung, zeilen: &[&str]) {
    waehle_datei(u, "auszug.csv", &format!("datum;betrag;verwendungszweck\n{}\n", zeilen.join("\n")));
}

/// Wartet, bis die Statuszeile des Kontoauszugs `text` enthaelt, und liefert sie.
pub(crate) fn warte_status(u: &Umgebung, text: &str) -> String {
    u.s.warte_bis(
        &format!("document.getElementById('konto-status').textContent.includes({})", serde_json::to_string(text).unwrap()),
        15,
        &format!("die Statuszeile nennt {text:?}"),
    );
    u.s.textinhalt("#konto-status")
}

fn rueckfrage_offen(u: &Umgebung) -> bool {
    u.s.ausdruck("!document.getElementById('konto-ersetzen').hidden") == json!(true)
}

fn knopf_gesperrt(u: &Umgebung) -> bool {
    u.s.ausdruck("document.getElementById('konto-ersetzen-go').disabled") == json!(true)
}

fn haken_gesetzt(u: &Umgebung) -> bool {
    u.s.ausdruck("document.getElementById('konto-ersetzen-haken').checked") == json!(true)
}

/// Der Fall im Fragebogen, das Feld fuer den Kontoauszug aufgeklappt.
pub(crate) fn zum_kontoauszug(u: &Umgebung) -> String {
    u.zum_fragebogen_neu();
    u.s.klick("#konto-toggle");
    u.s.warte_sichtbar("#konto-file", 10);
    u.fall()
}

fn werte(ereignisse: &[Value]) -> Vec<Value> {
    ereignisse.iter().map(|e| e["wert"].clone()).collect()
}

#[test]
fn zweiter_auszug_fragt_mit_haken_und_ersetzt_erst_danach() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = zum_kontoauszug(&u);

    // Der erste Auszug schreibt einen Vorschlag; auf ein leeres Feld fragt die Seite nichts.
    waehle_auszug(&u, ERSTER);
    let status = warte_status(&u, "2 von 2 Buchung(en) als Vorschlag erfasst");
    assert!(!rueckfrage_offen(&u), "auf ein leeres Feld fragt die Seite nichts; Status {status:?}");
    let erste = u.dienst.ereignisse(&fall, FELD);
    assert_eq!(werte(&erste), [json!(11_00)], "der erste Auszug schreibt 4,00 + 7,00 EUR");

    // Der zweite Auszug trifft das belegte Feld: die Rueckfrage nennt alt und neu, der Knopf ist gesperrt, der Fall bleibt, wie er war.
    waehle_auszug(&u, ZWEITER);
    let status = warte_status(&u, "Ersetzen möglich");
    assert!(
        status.contains("spenden_betrag: bisher 11,00 € aus 2 Buchung(en), mit diesem Auszug 16,00 € aus 3 Buchung(en)"),
        "die Rueckfrage nennt nicht alten und neuen Betrag: {status:?}"
    );
    assert!(status.starts_with("0 von 2 Buchung(en)"), "ohne Haken wird nichts uebernommen: {status:?}");
    assert!(rueckfrage_offen(&u), "die Rueckfrage steht nicht da");
    assert!(u.s.sichtbar("#konto-ersetzen-haken") && u.s.sichtbar("#konto-ersetzen-go"), "Haken oder Knopf fehlen");
    assert!(!haken_gesetzt(&u), "der Haken steht von sich aus");
    assert!(knopf_gesperrt(&u), "der Knopf ist ohne Haken nicht gesperrt");
    assert_eq!(u.dienst.ereignisse(&fall, FELD), erste, "die Rueckfrage schreibt nichts");

    // Der Aufruf des Knopfes ohne Haken tut nichts (zweite Sperre hinter dem gesperrten Knopf).
    u.s.auswerten("async () => { await kontoauszugErsetzen(); return true; }", &[]);
    assert_eq!(u.dienst.ereignisse(&fall, FELD), erste, "ohne Haken ersetzt die Seite doch");
    assert!(u.s.textinhalt("#konto-status").starts_with("0 von 2 Buchung(en)"), "der Aufruf ohne Haken sendet doch");

    // Der Haken gibt den Knopf frei, nimmt man ihn zurueck, sperrt er wieder; auch das schreibt nichts.
    u.s.klick("#konto-ersetzen-haken");
    assert!(haken_gesetzt(&u) && !knopf_gesperrt(&u), "der Haken gibt den Knopf nicht frei");
    u.s.klick("#konto-ersetzen-haken");
    assert!(!haken_gesetzt(&u) && knopf_gesperrt(&u), "der zurueckgenommene Haken sperrt den Knopf nicht wieder");
    assert_eq!(u.dienst.ereignisse(&fall, FELD), erste, "der Haken allein schreibt nichts");

    // Mit Haken und Klick auf „Ersetzen“: ein neues Ereignis traegt die Vereinigung und verweist auf das alte.
    u.s.klick("#konto-ersetzen-haken");
    u.s.klick("#konto-ersetzen-go");
    let status = warte_status(&u, "Ersetzt:");
    assert!(
        status.contains("spenden_betrag: bisher 11,00 € aus 2 Buchung(en), jetzt 16,00 € aus 3 Buchung(en)"),
        "die Meldung nennt nicht alten und neuen Betrag: {status:?}"
    );
    assert!(status.starts_with("2 von 2 Buchung(en)"), "die Zahl der uebernommenen Buchungen: {status:?}");
    assert!(!rueckfrage_offen(&u) && !haken_gesetzt(&u) && knopf_gesperrt(&u), "nach dem Ersetzen steht die Rueckfrage noch");
    let jetzt = u.dienst.ereignisse(&fall, FELD);
    assert_eq!(werte(&jetzt), [json!(11_00), json!(16_00)], "die Akte traegt nicht 11,00 dann 16,00 EUR");
    assert_eq!(jetzt[1]["ersetzt"], erste[0]["event_id"], "das neue Ereignis verweist nicht auf das alte");
    assert_eq!(u.dienst.letzter_wert(&fall, FELD), json!(16_00));

    // Derselbe Auszug noch einmal: nichts Neues, also keine Rueckfrage und kein drittes Ereignis.
    waehle_auszug(&u, ZWEITER);
    warte_status(&u, "0 von 2 Buchung(en)");
    assert!(!rueckfrage_offen(&u), "ein unveraenderter Auszug fragt noch einmal");
    assert_eq!(u.dienst.ereignisse(&fall, FELD).len(), 2);

    let konsole = u.konsolenfehler();
    assert!(konsole.is_empty(), "Konsolenfehler: {konsole:?}");
}

#[test]
fn der_haken_gilt_nicht_fuer_einen_anderen_auszug_als_den_der_rueckfrage() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = zum_kontoauszug(&u);

    waehle_auszug(&u, ERSTER);
    warte_status(&u, "2 von 2 Buchung(en) als Vorschlag erfasst");
    waehle_auszug(&u, ZWEITER);
    warte_status(&u, "Ersetzen möglich");
    assert!(rueckfrage_offen(&u));
    u.s.klick("#konto-ersetzen-haken");
    assert!(haken_gesetzt(&u) && !knopf_gesperrt(&u));
    let erste = u.dienst.ereignisse(&fall, FELD);

    // Ein dritter Auszug laeuft los, waehrend die Rueckfrage zum zweiten noch offen war. Die Antwort des Dienstes wird festgehalten:
    // solange der Auszug unterwegs ist, darf die Rueckfrage nicht mehr zu klicken sein, sonst nimmt der Haken den dritten mit.
    u.s.skript(
        "window.__echt = jpost; window.__weiter = null; \
         jpost = (...a) => new Promise((ok) => { window.__weiter = () => ok(window.__echt(...a)); });",
        &[],
    );
    waehle_auszug(&u, &["05.02.2025;-480,00;Rentenversicherung Huber"]);
    u.s.warte_bis("window.__weiter !== null", 10, "der dritte Auszug ist unterwegs");
    assert!(!rueckfrage_offen(&u), "waehrend ein neuer Auszug unterwegs ist, steht die alte Rueckfrage noch");
    u.s.skript("jpost = window.__echt; window.__weiter();", &[]);
    warte_status(&u, "1 von 1 Buchung(en) als Vorschlag erfasst");

    // Die Rueckfrage ist zu, der Haken zurueck, der Knopf gesperrt. Auch ein erzwungener Aufruf ersetzt nichts.
    assert!(!rueckfrage_offen(&u) && !haken_gesetzt(&u) && knopf_gesperrt(&u), "die Rueckfrage zum zweiten Auszug lebt");
    let status_vorher = u.s.textinhalt("#konto-status");
    u.s.auswerten(
        "async () => { document.getElementById('konto-ersetzen-go').disabled = false; await kontoauszugErsetzen(); return true; }",
        &[],
    );
    assert_eq!(u.s.textinhalt("#konto-status"), status_vorher, "der erzwungene Aufruf sendet einen Auszug, den keine Rueckfrage nannte");
    assert_eq!(u.dienst.ereignisse(&fall, FELD), erste, "der Haken der alten Rueckfrage hat ersetzt");
    assert_eq!(u.dienst.ereignisse(&fall, "vor_rv_ausserhalb_lstb").len(), 1, "der dritte Auszug schrieb sein Feld nicht");

    // Ein fehlerhafter Auszug (kein lesbares JSON) laesst die Rueckfrage ebenfalls nicht stehen.
    waehle_auszug(&u, ZWEITER);
    warte_status(&u, "Ersetzen möglich");
    u.s.klick("#konto-ersetzen-haken");
    waehle_datei(&u, "kaputt.json", "[{");
    warte_status(&u, "Upload fehlgeschlagen: json-Inhalt nicht parsebar");
    assert!(!rueckfrage_offen(&u) && !haken_gesetzt(&u) && knopf_gesperrt(&u), "nach einem Fehler steht die Rueckfrage noch");
    assert_eq!(u.dienst.ereignisse(&fall, FELD), erste, "nach dem Fehler wurde ersetzt");

    // Die Konsole meldet genau die eine beabsichtigte 400 des kaputten Auszugs, sonst nichts.
    let konsole = u.konsolenfehler();
    assert!(
        konsole.len() == 1 && konsole[0].contains("/kontoauszug - Failed to load resource") && konsole[0].contains("400"),
        "Konsolenfehler: {konsole:?}"
    );
}

/// Von main nachgeschrieben (Mutante U07 „go.disabled = true“ in `kontoauszugErsetzen` ueberlebte alle Tests des Forks): waehrend die
/// Antwort auf „Ersetzen“ aussteht, ist der Knopf gesperrt, und ein zweiter Klick sendet nichts. Ohne die Sperre ginge derselbe
/// Auszug zweimal raus; der Dienst schreibt ihn nur einmal (`schon_erfasst`), aber die Meldung des zweiten Aufrufs ueberschriebe
/// die des Ersetzens.
#[test]
fn der_knopf_ersetzen_sendet_waehrend_der_antwort_nicht_noch_einmal() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let fall = zum_kontoauszug(&u);

    waehle_auszug(&u, ERSTER);
    warte_status(&u, "2 von 2 Buchung(en) als Vorschlag erfasst");
    waehle_auszug(&u, ZWEITER);
    warte_status(&u, "Ersetzen möglich");
    u.s.klick("#konto-ersetzen-haken");
    assert!(haken_gesetzt(&u) && !knopf_gesperrt(&u));

    // Die Antwort des Dienstes wird festgehalten, `window.__aufrufe` zaehlt, wie oft die Seite sendet.
    u.s.skript(
        "window.__echt = jpost; window.__weiter = null; window.__aufrufe = 0; \
         jpost = (...a) => { window.__aufrufe += 1; return new Promise((ok) => { window.__weiter = () => ok(window.__echt(...a)); }); };",
        &[],
    );
    u.s.klick("#konto-ersetzen-go");
    u.s.warte_bis("window.__weiter !== null", 10, "der Auszug zum Ersetzen ist unterwegs");
    assert!(knopf_gesperrt(&u), "waehrend der Antwort ist der Knopf frei");
    assert!(u.s.textinhalt("#konto-status").contains("Ersetze"), "die Seite zeigt nicht, dass sie ersetzt");

    // Ein zweiter Klick auf den Knopf (ein Doppelklick) sendet nichts mehr.
    u.s.skript("document.getElementById('konto-ersetzen-go').click();", &[]);
    assert_eq!(u.s.ausdruck("window.__aufrufe"), json!(1), "der zweite Klick sendet denselben Auszug noch einmal");

    u.s.skript("jpost = window.__echt; window.__weiter();", &[]);
    let status = warte_status(&u, "Ersetzt:");
    assert!(status.starts_with("2 von 2 Buchung(en)"), "die Meldung des Ersetzens fehlt: {status:?}");
    assert_eq!(werte(&u.dienst.ereignisse(&fall, FELD)), [json!(11_00), json!(16_00)], "der Dienst schrieb nicht genau einmal");
    let konsole = u.konsolenfehler();
    assert!(konsole.is_empty(), "Konsolenfehler: {konsole:?}");
}
