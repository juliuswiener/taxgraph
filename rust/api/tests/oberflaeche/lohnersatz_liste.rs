//! Lohnersatzleistungen als Liste auf einer Seite (Nr. 97, Entscheidung d-48 von Julius, 2026-10-20).
//!
//! Statt EINER Summenfrage traegt der Nutzer jede Leistung als Zeile ein (Art, Betrag), alle auf der Seite der Frage
//! `p32b_progressionseinkuenfte`. Die Seite bildet die Summe; der Fall kennt weiter nur sie (Kz E0104801), Rechnung und
//! Sperren lesen diesen einen Wert. GELD: bildet die Seite die Summe falsch (eine Zeile verschluckt, Cent abgeschnitten), steht
//! im Fall ein anderer Betrag als der, den der Nutzer sah, und der Progressionsvorbehalt rechnet mit ihm.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Summen sind Handrechnung (1.000,10 + 0,29 + 1,15 = 1.001,54 EUR; 3.000 + 2.000 = 5.000 EUR).
//! Die Steuer ohne Lohnersatz (13.924 EUR, Einzelveranlagung, Lohn 60.000 EUR) steht so in `fluss_i.rs` (dort aus
//! `kist_bei_einzelveranlagung_bleibt_satz_mal_est`). Die Steuer MIT Lohnersatz ist hier GEMESSEN am Dienst, nicht von Hand gerechnet
//! (Grenze 1 im Plan-Eintrag); gerechnet ist nur ihre Gleichheit: dieselbe Summe als eine oder als zwei Zeilen gibt dieselbe Zahl.
//!
//! Grenzen: Chromium allein. Die Zeilen (Art, Einzelbetrag) stehen nur in der Seite, nicht im Fall; der Test misst deshalb den
//! Fall (nur die Summe) und nicht, was nach dem Neuladen in den Zeilen steht. Die Hilfsfunktionen des Durchlaufs sind eine
//! Kopie der Muster aus `fluss_i.rs` (ponytail: ein gemeinsames Modul, wenn ein dritter Test sie braucht).

use serde_json::{json, Value};

use crate::dienst::Start;
use crate::seite::{zum_fragebogen, Umgebung};

const FELD: &str = "p32b_progressionseinkuenfte";

/// Die Seite der Frage nach den Lohnersatzleistungen: Startseite, Kachel „Gesamt“, Fragebogen, das Kreuz „Lohnersatzleistungen“ in der
/// Ankreuzliste (ein Kreuz heisst „ja, das gab es“), dann die Frage selbst auf die Seite gebracht.
fn zur_frage(u: &Umgebung) {
    u.oeffne_start();
    u.fall_beginnen("fragebogen");
    zum_fragebogen(&u.s, Some(&["keine_lohnersatzleistungen"]));
    assert!(u.rendere_frage(FELD), "die Frage {FELD} steht trotz Kreuz nicht in /fragen");
}

/// Zeile `nr` (ab 1) fuellen: Art waehlen, Betrag tippen (wie ein Nutzer, mit Tasten).
fn trage_ein(u: &Umgebung, nr: usize, art: &str, betrag: &str) {
    let zeile = format!(".lersl-zeile:nth-child({nr})");
    u.s.waehle_option(&format!("{zeile} .lersl-art"), art);
    u.s.tippe(&format!("{zeile} .lersl-betrag"), betrag);
}

/// Der Wert des Feldes im Fall (`GET /stand`), `None` solange er fehlt.
fn im_fall(u: &Umgebung, fall: &str) -> Option<Value> {
    let felder = u.stand_felder(fall);
    felder.get(FELD).filter(|f| !f["wert"].is_null()).cloned()
}

#[test]
fn lohnersatz_liste_bildet_die_summe_und_schreibt_ein_ereignis() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    zur_frage(&u);
    let fall = u.fall();

    // Eine Seite: eine Zeile mit Art und Betrag, die Taste fuer weitere Leistungen, die Summe, der Bestaetigen-Knopf. Das
    // Feld, das `leseWert()` liest, ist versteckt und traegt die Summe.
    pp!(
        "N.die_liste_steht_auf_einer_seite",
        u.s.anzahl(".lersl-zeile") == 1
            && u.s.anzahl(".lersl-zeile .lersl-art") == 1
            && u.s.anzahl(".lersl-zeile .lersl-betrag") == 1
            && u.s.sichtbar(".lersl-mehr")
            && u.s.sichtbar("#bestaetigen"),
        "die Seite zeigt {} Zeilen; Taste {}, Knopf {}",
        u.s.anzahl(".lersl-zeile"),
        u.s.sichtbar(".lersl-mehr"),
        u.s.sichtbar("#bestaetigen")
    );
    pp!("N.die_liste_steht_auf_einer_seite", u.s.ausdruck("document.getElementById('feld-input').type") == json!("hidden"), "das Lesefeld ist nicht versteckt");

    // Nichts eingetragen: kein Ereignis, die bekannte Meldung (Stille-Null-Regel, wie beim einzelnen Feld).
    u.s.klick("#bestaetigen");
    u.s.warte_bis("document.getElementById('netz-banner').textContent.includes('gültigen Wert')", 10, "die Meldung zur leeren Eingabe");
    pp!("N.eine_leere_liste_schreibt_nichts", im_fall(&u, &fall).is_none(), "ein leerer Bestaetigen-Klick schrieb {:?}", im_fall(&u, &fall));

    // Zwei weitere Zeilen auf derselben Seite.
    u.s.klick(".lersl-mehr");
    u.s.klick(".lersl-mehr");
    pp!(
        "N.die_liste_waechst_auf_derselben_seite",
        u.s.anzahl(".lersl-zeile") == 3 && u.s.sichtbar(".lersl-zeile:nth-child(3) .lersl-betrag"),
        "Zeilen: {}",
        u.s.anzahl(".lersl-zeile")
    );

    // Drei Zeilen mit Cent-Betraegen: 1.000,10 + 0,29 + 1,15 = 1.001,54 EUR. 0,29 * 100 und 1,15 * 100 liegen in JavaScript knapp UNTER der ganzen Zahl (28,999999999999996 und 114,99999999999999): abgeschnittene Cent (Math.floor statt round) oder eine
    // verschluckte Zeile gaebe einen anderen Betrag.
    trage_ein(&u, 1, "arbeitslosengeld", "1000.10");
    trage_ein(&u, 2, "krankengeld", "0.29");
    trage_ein(&u, 3, "elterngeld", "1.15");
    let anzeige = u.s.text(".lersl-summe").replace('\u{a0}', " ");
    pp!("N.die_seite_zeigt_die_summe", anzeige == "Summe: 1.001,54 €", "die Seite zeigt {anzeige:?}");
    pp!("N.die_seite_zeigt_die_summe", u.s.wert("#feld-input") == "1001.54", "das Lesefeld traegt {:?}", u.s.wert("#feld-input"));

    // Auf dem Handy (360 px) brechen die Zeilen um; die Seite wird nicht breiter als der Schirm (dieselbe Messlatte wie `fluss_f`).
    u.s.sichtflaeche(360, 640);
    let breite = u.s.ausdruck("document.documentElement.scrollWidth").as_i64().unwrap_or(-1);
    pp!("N.die_zeilen_sprengen_den_schirm_nicht", breite <= 360, "scrollWidth {breite} bei 360 px Breite");
    u.s.sichtflaeche(1280, 900);

    // Bestaetigen schreibt GENAU die Summe in Cent als EIN Ereignis auf das Feld des Falls.
    u.s.klick("#bestaetigen");
    let mut stand = None;
    for _ in 0..100 {
        stand = im_fall(&u, &fall);
        if stand.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let stand = stand.expect("nach dem Bestaetigen steht kein Wert im Fall");
    pp!("N.der_fall_traegt_die_summe_in_cent", stand["wert"] == json!(100_154), "im Fall steht {}", stand);
    pp!("N.der_fall_traegt_die_summe_in_cent", stand["zustand"] == json!("bestaetigt"), "Zustand {}", stand["zustand"]);
    let konsole = u.konsolenfehler();
    pp!("N.die_liste_laesst_die_konsole_sauber", konsole.is_empty(), "Konsolenfehler: {konsole:?}");
}

// ---- Der Durchlauf von der Startseite bis zur Steuerzahl ----------------------------------------------------------

/// Was der Nutzer auf eine Frage antwortet. Geld steht in Euro, wie es getippt wird.
#[derive(Clone, Copy, Debug)]
enum Gib {
    Euro(&'static str),
    Wahl(&'static str),
    Text(&'static str),
    /// Die Liste der Lohnersatzleistungen: je Zeile Art und Betrag in Euro.
    Liste(&'static [(&'static str, &'static str)]),
}

type Tabelle = Vec<(&'static str, Gib)>;

fn aktuell(u: &Umgebung) -> Value {
    u.s.ausdruck("AKTUELL")
}

/// `true` bei einer Frage, `false` beim Ergebnis.
fn warte_auf_frage_oder_ergebnis(u: &Umgebung) -> bool {
    let v = u.s.warte_bis(
        "(() => { const f = !document.getElementById('fertig').hidden; \
         const w = !document.getElementById('wegpunkt').hidden && AKTUELL; return f ? 'ergebnis' : (w ? 'frage' : false); })()",
        30,
        "eine Frage oder das Ergebnis erscheint",
    );
    v == json!("frage")
}

/// Tippt oder klickt die Antwort auf die aktuelle Frage `q` und bestaetigt. Liefert (Feld, Antwort).
fn antworte(u: &Umgebung, q: &Value, tabelle: &[(&'static str, Gib)]) -> (String, String) {
    let s = &u.s;
    let feld = q["feld_id"].as_str().unwrap().to_owned();
    let typ = q["typ"].as_str().unwrap().to_owned();
    let bekannt = tabelle.iter().find(|(f, _)| *f == feld).map(|(_, g)| *g);
    let antwort = match (bekannt, typ.as_str()) {
        (Some(Gib::Liste(zeilen)), "cent") => {
            for (i, (art, betrag)) in zeilen.iter().enumerate() {
                if i > 0 {
                    s.klick(".lersl-mehr");
                }
                trage_ein(u, i + 1, art, betrag);
            }
            format!("{zeilen:?}")
        }
        (Some(Gib::Euro(t)), "cent" | "int") | (Some(Gib::Text(t)), "text") => {
            s.tippe("#feld-input", t);
            t.to_owned()
        }
        (None, "cent" | "int") => {
            s.tippe("#feld-input", "0");
            "0".to_owned()
        }
        (None, "bool") => {
            s.klick(".wahl-opt[data-wert='false']");
            "Nein".to_owned()
        }
        (Some(Gib::Text(t)), "datum") => {
            let d: Vec<&str> = t.split('.').collect();
            s.fuelle("#feld-input", &format!("{}-{}-{}", d[2], d[1], d[0]));
            t.to_owned()
        }
        (Some(Gib::Wahl(w)), "enum") => {
            let werte = q["enum_werte"].as_array().unwrap();
            assert!(werte.contains(&json!(w)), "{feld}: Option {w} gibt es nicht");
            if werte.len() <= 6 {
                s.klick(&format!(".wahl-opt[data-wert='{w}']"));
            } else {
                s.waehle_option("#feld-input", w);
            }
            w.to_owned()
        }
        (gib, typ) => panic!("die Frage {feld} (Typ {typ}, „{}“) steht nicht (passend) in der Antworttabelle: {gib:?}", q["fragetext_laie"]),
    };
    s.klick("#bestaetigen");
    (feld, antwort)
}

/// Frage fuer Frage durch die Oberflaeche bis zum Ergebnis. Liefert die Felder in der Reihenfolge, in der sie gefragt wurden.
fn durchlauf(u: &Umgebung, tabelle: &[(&'static str, Gib)]) -> Vec<String> {
    let mut verlauf: Vec<String> = Vec::new();
    loop {
        assert!(verlauf.len() < 600, "ueber 600 Fragen; zuletzt {:?}", verlauf.last());
        if !warte_auf_frage_oder_ergebnis(u) {
            return verlauf;
        }
        let q = aktuell(u);
        let (feld, antwort) = antworte(u, &q, tabelle);
        eprintln!("SCHRITT {:3} {feld} -> {antwort}", verlauf.len());
        let weiter = u.s.try_warte_bis(
            &format!(
                "(AKTUELL && AKTUELL.feld_id !== '{feld}' && !document.getElementById('wegpunkt').hidden) \
                 || !document.getElementById('fertig').hidden"
            ),
            20,
        );
        assert!(weiter.is_some(), "nach der Antwort {antwort:?} auf {feld} zeigt die Oberflaeche weiter dieselbe Frage; Banner: „{}“", u.s.textinhalt("#netz-banner"));
        verlauf.push(feld);
    }
}

/// Geld in Cent so, wie die Oberflaeche es anzeigt: `1392400` -> „13.924,00 €“ (mit gewoehnlichem Leerzeichen).
fn euro_text(cent: i64) -> String {
    let (euro, rest) = (cent / 100, cent % 100);
    let mut gruppen = Vec::new();
    let mut e = euro;
    while e >= 1000 {
        gruppen.push(format!("{:03}", e % 1000));
        e /= 1000;
    }
    gruppen.push(e.to_string());
    gruppen.reverse();
    format!("{},{rest:02} €", gruppen.join("."))
}

/// Person A, Einzelveranlagung, Lohn 60.000 EUR, evangelisch in Nordrhein-Westfalen (wie `fluss_i_einzelnutzer_nur_lohn_mit_kirchensteuer`).
fn einzelnutzer(lersl: Option<&'static [(&'static str, &'static str)]>) -> Tabelle {
    let mut t: Tabelle = vec![
        ("stammdaten_nachname", Gib::Text("Mustermann")),
        ("stammdaten_vorname", Gib::Text("Max")),
        ("stammdaten_geburtsdatum", Gib::Text("15.03.1985")),
        ("stammdaten_plz", Gib::Text("80331")),
        ("stammdaten_wohnort", Gib::Text("München")),
        ("stammdaten_strasse", Gib::Text("Marienplatz")),
        ("stammdaten_hausnummer", Gib::Text("1")),
        ("stammdaten_hausnummerzusatz", Gib::Text("a")),
        ("stammdaten_bic", Gib::Text("PBNKDEFF370")),
        ("stammdaten_iban", Gib::Text("DE02120300000000202051")),
        ("stammdaten_steuernummer", Gib::Text("9181081508155")),
        ("steuerklasse", Gib::Wahl("4")),
        ("bruttoarbeitslohn", Gib::Euro("60000")),
        ("versicherungsart", Gib::Wahl("gesetzlich_an")),
        ("ep_ziel_des_weges", Gib::Wahl("1")),
        ("ep_ziel_adresse", Gib::Text("80331 München, Marienplatz 1")),
        ("veranlagung", Gib::Wahl("einzel")),
        ("kist_konfession", Gib::Wahl("evangelisch")),
        ("kist_bundesland", Gib::Wahl("nordrhein_westfalen")),
    ];
    if let Some(zeilen) = lersl {
        t.push((FELD, Gib::Liste(zeilen)));
    }
    t
}

/// Ein Durchlauf: neuer Fall, das Kreuz „Lohnersatzleistungen“ gesetzt oder nicht, alle Fragen, die Zahl des Dienstes.
/// Liefert (`zahl_cent`, Wert des Feldes im Fall, wie oft die Oberflaeche die Frage stellte).
fn lauf(u: &Umgebung, mit_lohnersatz: Option<&'static [(&'static str, &'static str)]>) -> (i64, Option<Value>, usize) {
    u.oeffne_start();
    u.fall_beginnen("fragebogen");
    zum_fragebogen(&u.s, Some(if mit_lohnersatz.is_some() { &["keine_lohnersatzleistungen"][..] } else { &[][..] }));
    let verlauf = durchlauf(u, &einzelnutzer(mit_lohnersatz));
    let fall = u.fall();
    let (status, api) = u.dienst.get(&format!("/fall/{fall}/ergebnis"));
    assert_eq!(status, 200, "ergebnis: {api}");
    assert_eq!(api["grund"], json!("bestaetigt"), "der Dienst sperrt: {api}");
    u.s.warte_sichtbar("#fertig", 20);
    let gezeigt = u.s.text("#ergebnis .erg-zahl").replace('\u{a0}', " ");
    let zahl = api["zahl_cent"].as_i64().expect("zahl_cent");
    assert_eq!(gezeigt, euro_text(zahl), "die Oberflaeche zeigt {gezeigt:?}, der Dienst meldet {zahl} Cent");
    (zahl, im_fall(u, &fall), verlauf.iter().filter(|f| *f == FELD).count())
}

const ZWEI_ZEILEN: &[(&str, &str)] = &[("arbeitslosengeld", "3000"), ("krankengeld", "2000")];
const EINE_ZEILE: &[(&str, &str)] = &[("arbeitslosengeld", "5000")];

#[test]
fn lohnersatz_liste_vom_start_bis_zur_steuerzahl_gleiche_summe_gleiche_steuer() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);

    // Ohne Lohnersatz: die Frage kommt nicht, die Steuer ist die der Handrechnung aus `fluss_i.rs` (13.924 EUR).
    let (ohne, fall_ohne, fragen_ohne) = lauf(&u, None);
    pp!("N.ohne_lohnersatz_keine_frage_und_die_bekannte_steuer", fragen_ohne == 0 && fall_ohne.is_none(), "Frage {fragen_ohne}-mal, Fall: {fall_ohne:?}");
    pp!("N.ohne_lohnersatz_keine_frage_und_die_bekannte_steuer", ohne == 1_392_400, "die Steuer ohne Lohnersatz ist {ohne} Cent, nicht 1.392.400");

    // Zwei Zeilen (3.000 + 2.000 EUR) und eine Zeile (5.000 EUR): dieselbe Summe im Fall, dieselbe Steuer.
    let (zwei, fall_zwei, fragen_zwei) = lauf(&u, Some(ZWEI_ZEILEN));
    let (eins, fall_eins, _) = lauf(&u, Some(EINE_ZEILE));
    pp!("N.die_frage_kommt_einmal_mit_den_zeilen", fragen_zwei == 1, "die Frage kam {fragen_zwei}-mal");
    pp!(
        "N.zwei_zeilen_tragen_die_summe_in_den_fall",
        fall_zwei.as_ref().is_some_and(|f| f["wert"] == json!(500_000)) && fall_eins.as_ref().is_some_and(|f| f["wert"] == json!(500_000)),
        "im Fall stehen {fall_zwei:?} und {fall_eins:?}, erwartet 500000 Cent"
    );
    pp!("N.dieselbe_summe_gibt_dieselbe_steuer", zwei == eins, "zwei Zeilen: {zwei} Cent, eine Zeile: {eins} Cent");
    // Der Progressionsvorbehalt hebt den Satz: mit 5.000 EUR Lohnersatz ist die Steuer hoeher als ohne.
    pp!("N.lohnersatz_hebt_die_steuer", zwei > ohne, "mit Lohnersatz {zwei} Cent, ohne {ohne} Cent");
    // GEMESSEN am Dienst (2026-10-20, Basis 97453c5d), nicht von Hand gerechnet: 14.648 EUR statt 13.924 EUR. Die Zahl haelt den Stand
    // fest; aendert sich Tarif oder Progressionsvorbehalt, faerbt sich dieser Punkt, und die Gleichheit oben bleibt der eigentliche Beleg.
    pp!("N.lohnersatz_hebt_die_steuer", zwei == 1_464_800, "mit 5.000 EUR Lohnersatz ist die Steuer {zwei} Cent, gemessen waren 1.464.800");
    eprintln!("MESSUNG Steuer ohne {ohne} Cent, mit 5.000 EUR Lohnersatz {zwei} Cent");
    let konsole = u.konsolenfehler();
    pp!("N.die_liste_laesst_die_konsole_sauber", konsole.is_empty(), "Konsolenfehler: {konsole:?}");
}
