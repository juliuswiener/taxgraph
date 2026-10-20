//! Fluss I: ein Nutzer klickt einen Fall von der Startseite bis zur Steuerzahl durch.
//!
//! Die Fluesse A bis H nehmen Abkuerzungen: sie rendern eine Frage per Skript (`rendere_frage`) oder setzen Werte ueber
//! die Schnittstelle. Hier gibt es keine: die Tests tippen und klicken, was ein Nutzer tippt und klickt (WebDriver-Tasten
//! und -Klicks), und nehmen jede Frage, die die Oberflaeche zeigt, in der Reihenfolge, in der sie sie zeigt. Die
//! Schnittstelle dient nur zum LESEN (`/stand`, `/ergebnis`), zum Vergleich mit dem, was die Oberflaeche zeigt.
//!
//! GELD: die Kirchensteuer des Ehegatten haengt an der Frage nach dem Bundesland (Entscheidung d-43). Fehlt die Frage, rechnet
//! die Software mit 9 statt 8 Prozent in Bayern und Baden-Wuerttemberg. Die Reihenfolge der Fragen in einer echten Sitzung
//! hatte bis hierher kein Test gesehen (Grenze 5 im Plan-Eintrag „Bundesland-Frage“).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Handrechnung nach Gesetz, wie in `kette_endstand_hermetisch.rs` (Tarif und Pauschbetraege aus
//! `params/2025/`, nicht aus der App): Lohn A 60.000 EUR, Lohn B 20.000 EUR, gemeinsame ESt 13.838 EUR, Kirchensteuer von B in
//! Bayern 9.683 Cent (Zeile „Bayern 8 %, A keine, B ev“ in `KIST_FAELLE`); Einzelveranlagung Lohn 60.000 EUR, evangelisch, NRW:
//! ESt 13.924 EUR, Kirchensteuer 125.316 Cent (`kist_bei_einzelveranlagung_bleibt_satz_mal_est`). Die Zahlen stehen hier als
//! Kopien; ein Test, der sie anders liest, faerbt sich, ohne dass jemand die Handrechnung wiederholt.
//!
//! Grenzen: Chromium allein. Das Kalenderfeld (`<input type=date>`) nimmt der Test ueber den Wert, nicht ueber Tasten (die
//! Tasten haengen vom Gebietsschema des Browsers ab). Antworten ausser den in der Tabelle genannten sind „neutral“
//! (Geld und Zahl 0, Ja/Nein „Nein“); Text, Datum und Auswahl MUESSEN in der Tabelle stehen, sonst bricht der Test mit dem
//! Namen der Frage ab.

use serde_json::{json, Value};

use crate::chat::ModellErsatz;
use crate::dienst::Start;
use crate::seite::Umgebung;

/// Was der Nutzer auf eine Frage antwortet. Geld steht in Euro, wie es getippt wird.
#[derive(Clone, Copy, Debug)]
enum Gib {
    /// Euro, wie getippt („60000“ oder „19.99“).
    Euro(&'static str),
    /// Eine Auswahl (Wert der Option).
    Wahl(&'static str),
    /// Text oder Datum, wie getippt (Datum als TT.MM.JJJJ).
    Text(&'static str),
}

type Tabelle = Vec<(&'static str, Gib)>;

/// Ein Schritt des Nutzers: Feld, Typ, Frage, Antwort.
#[derive(Clone, Debug)]
struct Schritt {
    feld: String,
    typ: String,
    frage: String,
    antwort: String,
    /// Die Optionen einer Auswahl, getrennt durch Komma.
    optionen: String,
    /// `true`, wenn die Tabelle die Frage nicht kannte und der neutrale Standard antwortete (0 oder „Nein“).
    standard: bool,
}

impl Schritt {
    fn zeile(&self, i: usize) -> String {
        format!(
            "SCHRITT {i:3} {} [{}] {:?} -> {}{}{}",
            self.feld,
            self.typ,
            self.frage,
            self.antwort,
            if self.standard { " (neutral)" } else { "" },
            if self.optionen.is_empty() { String::new() } else { format!(" aus {}", self.optionen) }
        )
    }
}

/// Nennt bei einer Panik die Frage, an der der Durchlauf stand (die Meldungen des Browser-Clients nennen nur den Selektor).
struct Schrittwache(String);

impl Drop for Schrittwache {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("DER DURCHLAUF STAND BEI DER FRAGE {}", self.0);
        }
    }
}

/// Die aktuelle Frage der Oberflaeche (`AKTUELL`).
fn aktuell(u: &Umgebung) -> Value {
    u.s.ausdruck("AKTUELL")
}

/// Wartet, bis die Oberflaeche eine Frage oder das Ergebnis zeigt. `true` bei einer Frage, `false` beim Ergebnis.
fn warte_auf_frage_oder_ergebnis(u: &Umgebung) -> bool {
    let v = u.s.warte_bis(
        "(() => { const f = !document.getElementById('fertig').hidden; \
         const w = !document.getElementById('wegpunkt').hidden && AKTUELL; return f ? 'ergebnis' : (w ? 'frage' : false); })()",
        30,
        "eine Frage oder das Ergebnis erscheint",
    );
    v == json!("frage")
}

/// Tippt oder klickt die Antwort auf die aktuelle Frage `q` und bestaetigt. Liefert den Schritt.
fn antworte(u: &Umgebung, q: &Value, tabelle: &[(&'static str, Gib)]) -> Schritt {
    let s = &u.s;
    let feld = q["feld_id"].as_str().unwrap().to_owned();
    let typ = q["typ"].as_str().unwrap().to_owned();
    let invertiert = q["frage_invertiert"] == json!(true);
    let optionen = q["enum_werte"].as_array().map_or_else(String::new, |w| w.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(","));
    let bekannt = tabelle.iter().find(|(f, _)| *f == feld).map(|(_, g)| *g);
    let standard = bekannt.is_none();
    let antwort = match (bekannt, typ.as_str()) {
        (Some(Gib::Euro(t)), "cent" | "int") | (Some(Gib::Text(t)), "text") => {
            s.tippe("#feld-input", t);
            t.to_owned()
        }
        (None, "cent" | "int") => {
            s.tippe("#feld-input", "0");
            "0".to_owned()
        }
        // Ja/Nein: der neutrale Standard ist der Knopf „Nein“ (die Antwort, nicht der gespeicherte Wert; `frage_invertiert`
        // kehrt ihn erst beim Speichern um). Der Test nennt `invertiert` nur, damit der Schritt es im Verlauf zeigt.
        (None, "bool") => {
            s.klick(".wahl-opt[data-wert='false']");
            if invertiert { "Nein (gespeichert wird das Gegenteil)".to_owned() } else { "Nein".to_owned() }
        }
        (Some(Gib::Text(t)), "datum") => {
            // Ein Kalenderfeld nimmt ISO; die Tasten haengen vom Gebietsschema des Browsers ab, deshalb setzt der Test den Wert.
            let d: Vec<&str> = t.split('.').collect();
            s.fuelle("#feld-input", &format!("{}-{}-{}", d[2], d[1], d[0]));
            t.to_owned()
        }
        (Some(Gib::Wahl(w)), "enum") => {
            let werte = q["enum_werte"].as_array().unwrap();
            assert!(werte.contains(&json!(w)), "{feld}: Option {w} gibt es nicht, es gibt {optionen}");
            if werte.len() <= 6 {
                s.klick(&format!(".wahl-opt[data-wert='{w}']"));
            } else {
                s.waehle_option("#feld-input", w);
            }
            w.to_owned()
        }
        (gib, typ) => panic!(
            "die Frage {feld} (Typ {typ}, Optionen [{optionen}], „{}“) steht nicht (passend) in der Antworttabelle: {gib:?}",
            q["fragetext_laie"]
        ),
    };
    s.klick("#bestaetigen");
    Schritt { feld, typ, frage: q["fragetext_laie"].as_str().unwrap_or("").to_owned(), antwort, optionen, standard }
}

/// Geht Frage fuer Frage durch die Oberflaeche. Haelt vor der ersten Frage aus `halte_vor` an (ohne sie zu beantworten) oder
/// erst beim Ergebnis. Liefert die Schritte in der Reihenfolge, in der die Oberflaeche sie gestellt hat.
fn durchlauf(u: &Umgebung, tabelle: &[(&'static str, Gib)], halte_vor: &[&str]) -> Vec<Schritt> {
    let mut verlauf: Vec<Schritt> = Vec::new();
    loop {
        assert!(verlauf.len() < 600, "ueber 600 Fragen: die Oberflaeche hoert nicht auf zu fragen; zuletzt {:?}", verlauf.last());
        if !warte_auf_frage_oder_ergebnis(u) {
            return verlauf;
        }
        let q = aktuell(u);
        let feld = q["feld_id"].as_str().unwrap().to_owned();
        if halte_vor.contains(&feld.as_str()) {
            return verlauf;
        }
        let _wache = Schrittwache(format!("{} (Frage {} dieses Durchlaufs)", feld, verlauf.len()));
        let schritt = antworte(u, &q, tabelle);
        eprintln!("{}", schritt.zeile(verlauf.len()));
        // Weiter geht es erst, wenn die Oberflaeche eine ANDERE Frage zeigt oder das Ergebnis; sonst hat sie die Antwort abgewiesen.
        let weiter = u.s.try_warte_bis(
            &format!(
                "(AKTUELL && AKTUELL.feld_id !== '{feld}' && !document.getElementById('wegpunkt').hidden) \
                 || !document.getElementById('fertig').hidden"
            ),
            20,
        );
        assert!(
            weiter.is_some(),
            "nach der Antwort {:?} auf {feld} zeigt die Oberflaeche weiter dieselbe Frage; Banner: „{}“",
            schritt.antwort,
            u.s.textinhalt("#netz-banner")
        );
        verlauf.push(schritt);
    }
}

/// Der Weg bis zur ersten Frage: Startseite, kachel `kachel`, Weg „Fragebogen“, die Ankreuzliste mit den Kreuzen `kreuze`
/// (Feld-IDs der Liste; ein Kreuz heisst „ja, das gab es“), dann „Weiter“. Liefert die Zeilen der Liste (Feld-ID und Text).
fn bis_zur_ersten_frage(u: &Umgebung, kachel: &str, kreuze: &[&str]) -> Value {
    u.oeffne_start();
    u.s.klick(&format!(".kachel[data-scheibe='{kachel}']"));
    u.s.warte_sichtbar("#wegwahl", 10);
    u.s.klick("#weg-fragebogen");
    u.s.warte_sichtbar("#screening", 10);
    let liste = u.s.ausdruck(
        "[...document.querySelectorAll('#screening-liste .sc-zeile')].map(z => \
         [z.querySelector('.sc-box').dataset.feld, z.querySelector('.sc-text').textContent])",
    );
    for k in kreuze {
        let css = format!("#screening-liste .sc-box[data-feld=\"{k}\"]");
        assert!(u.s.finde(&css).is_some(), "{k} steht nicht in der Ankreuzliste: {liste}");
        u.s.klick(&css);
    }
    u.s.klick("#screening-weiter");
    u.s.warte_sichtbar("#wegpunkt", 10);
    let _ = u.konsolenfehler(); // Ausgangslage: Meldungen der Startseite zaehlen nicht
    liste
}

/// Geld in Cent so, wie die Oberflaeche es anzeigt: `1383800` -> „13.838,00 €“ (mit gewoehnlichem Leerzeichen).
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

/// Das Ergebnis, wie die Oberflaeche es zeigt, und wie der Dienst es liefert: `(gezeigte Zahl, Rechenweg-Ende, Antwort)`.
/// Die Oberflaeche muss das Ergebnis zeigen (kein Sperrtext).
fn ergebnis(u: &Umgebung) -> (String, String, Value) {
    let s = &u.s;
    s.warte_sichtbar("#fertig", 20);
    let gezeigt = s.text("#ergebnis .erg-zahl").replace('\u{a0}', " ");
    let rechenweg = s.text("#rechenweg-body .rw-end .rw-wert").replace('\u{a0}', " ");
    let (status, api) = u.dienst.get(&format!("/fall/{}/ergebnis", u.fall()));
    assert_eq!(status, 200, "ergebnis: {api}");
    (gezeigt, rechenweg, api)
}

/// Die gemeinsamen Antworten von Person A: Stammdaten und die Fragen, die keine Zahl aendern. Lohn 60.000 EUR.
fn person_a() -> Tabelle {
    vec![
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
    ]
}

/// Person B bei Zusammenveranlagung. Lohn 20.000 EUR.
fn person_b() -> Tabelle {
    vec![
        ("veranlagung", Gib::Wahl("zusammen")),
        ("bruttoarbeitslohn_partner", Gib::Euro("20000")),
        ("stammdaten_vorname_partner", Gib::Text("Erika")),
        ("stammdaten_nachname_partner", Gib::Text("Mustermann")),
        ("stammdaten_geburtsdatum_partner", Gib::Text("09.07.1988")),
        ("steuerklasse_partner", Gib::Wahl("4")),
        ("versicherungsart_partner", Gib::Wahl("gesetzlich_an")),
        ("versorgung_art_partner", Gib::Wahl("beamtenrechtlich")),
    ]
}

fn kirche(konf_a: &'static str, konf_b: Option<&'static str>, land: &'static str) -> Tabelle {
    let mut t = vec![("kist_konfession", Gib::Wahl(konf_a)), ("kist_bundesland", Gib::Wahl(land))];
    if let Some(b) = konf_b {
        t.push(("kist_konfession_partner", Gib::Wahl(b)));
    }
    t
}

fn ehepaar(konf_a: &'static str, konf_b: &'static str, land: &'static str) -> Tabelle {
    let mut t = person_a();
    t.extend(person_b());
    t.extend(kirche(konf_a, Some(konf_b), land));
    t
}

/// Die Position einer Frage im Verlauf.
fn pos(verlauf: &[Schritt], feld: &str) -> Option<usize> {
    verlauf.iter().position(|s| s.feld == feld)
}

/// Wie oft die Oberflaeche die Frage `feld` gestellt hat.
fn wie_oft(verlauf: &[Schritt], feld: &str) -> usize {
    verlauf.iter().filter(|s| s.feld == feld).count()
}

/// Das Ergebnis pruefen: die Oberflaeche zeigt die Zahl des Dienstes, der Rechenweg endet bei ihr, der Dienst meldet
/// `zahl_cent` und `kist_cent` wie die Handrechnung. `kennung` ist der Name des Falls in den Meldungen.
fn pruefe_ergebnis(u: &Umgebung, kennung: &str, zahl_cent: i64, kist_cent: i64) {
    let (gezeigt, rechenweg, api) = ergebnis(u);
    pp!("I.die_oberflaeche_zeigt_die_zahl_des_dienstes", api["grund"] == json!("bestaetigt"), "{kennung}: der Dienst sperrt: {api}");
    pp!(
        "I.die_oberflaeche_zeigt_die_zahl_des_dienstes",
        api["zahl_cent"].as_i64().is_some_and(|z| gezeigt == euro_text(z)),
        "{kennung}: die Oberflaeche zeigt {gezeigt:?}, der Dienst meldet {}",
        api["zahl_cent"]
    );
    pp!(
        "I.die_oberflaeche_zeigt_die_zahl_des_dienstes",
        api["zahl_cent"].as_i64().is_some_and(|z| rechenweg == format!("{} €", euro_text(z).trim_end_matches(",00 €"))),
        "{kennung}: der Rechenweg endet bei {rechenweg:?}, der Dienst meldet {}",
        api["zahl_cent"]
    );
    pp!(
        "I.die_zahl_stimmt_mit_der_handrechnung",
        api["zahl_cent"] == json!(zahl_cent),
        "{kennung}: die Steuer ist {}, die Handrechnung sagt {zahl_cent} Cent",
        api["zahl_cent"]
    );
    pp!(
        "I.die_kirchensteuer_stimmt_mit_der_handrechnung",
        api["kist_cent"] == json!(kist_cent),
        "{kennung}: die Kirchensteuer ist {}, die Handrechnung sagt {kist_cent} Cent",
        api["kist_cent"]
    );
}

/// Keine Fehler in der Browser-Konsole seit dem Beginn des Durchlaufs.
fn pruefe_konsole(u: &Umgebung, kennung: &str) {
    let fehler = u.konsolenfehler();
    pp!("I.der_durchlauf_laesst_die_konsole_sauber", fehler.is_empty(), "{kennung}: Konsolenfehler: {fehler:?}");
}

/// Die Antwort auf ein bestaetigtes Feld aendern, wie ein Nutzer: die Zeile in „Schon beantwortet“ anklicken, die neue
/// Antwort geben, bestaetigen. Wartet, bis die Oberflaeche wieder das Ergebnis zeigt.
fn korrigiere(u: &Umgebung, feld: &'static str, neu: Gib) {
    let zeile = format!("#belegt-liste li:has(.z-name[title='{feld}'])");
    u.s.klick(&zeile);
    u.s.warte_bis(&format!("!document.getElementById('wegpunkt').hidden && AKTUELL && AKTUELL.feld_id === '{feld}'"), 10, "die Frage der Korrektur erscheint");
    let q = aktuell(u);
    let schritt = antworte(u, &q, &[(feld, neu)]);
    eprintln!("{}", schritt.zeile(0));
    u.s.warte_bis("!document.getElementById('fertig').hidden", 20, "das Ergebnis erscheint nach der Korrektur");
}

#[test]
fn fluss_i_ehepaar_vom_start_bis_zur_steuerzahl_mit_korrektur() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    // Das Kreuz „Kapitalertraege des Partners“ ist noetig: ohne es endet der Durchlauf in der Sperre `partner_kegel_offen`
    // ohne offene Frage (siehe Bericht). Der Partner gibt dann 0 EUR an, wie im Kegel der Handrechnung.
    bis_zur_ersten_frage(&u, "gesamt", &["kein_kap_partner"]);
    let verlauf = durchlauf(&u, &ehepaar("keine", "evangelisch", "bayern"), &[]);
    let fall = u.fall();

    // Die Reihenfolge, wie die Oberflaeche sie stellt: die Konfession des Ehegatten, spaeter das Bundesland, danach die eigene
    // Konfession. Das Bundesland steht VOR der eigenen Konfession (gemessen); die ODER-Bedingung entscheidet in diesem Weg
    // nicht, weil die eigene Antwort beim Bundesland noch offen ist (eine offene Antwort schliesst nicht aus).
    let (b, land, a) = (pos(&verlauf, "kist_konfession_partner"), pos(&verlauf, "kist_bundesland"), pos(&verlauf, "kist_konfession"));
    pp!("I.das_bundesland_wird_einmal_gefragt_nach_der_konfession_des_ehegatten", wie_oft(&verlauf, "kist_bundesland") == 1, "das Bundesland wurde {}-mal gefragt", wie_oft(&verlauf, "kist_bundesland"));
    pp!("I.das_bundesland_wird_einmal_gefragt_nach_der_konfession_des_ehegatten", b.is_some() && land.is_some() && b < land, "Reihenfolge: Ehegatte {b:?}, Bundesland {land:?}, eigene Konfession {a:?}");
    eprintln!("REIHENFOLGE ehepaar: Konfession B {b:?}, Bundesland {land:?}, Konfession A {a:?}, Schritte {}", verlauf.len());

    pruefe_ergebnis(&u, "Ehepaar, nur B evangelisch, Bayern", 1_383_800, 9_683);

    // Die Antwort des Ehegatten zuruecknehmen: ohne Mitglied keine Kirchensteuer, die Steuer bleibt. Das Bundesland bleibt als
    // alte Antwort im Fall stehen (gemessen).
    korrigiere(&u, "kist_konfession_partner", Gib::Wahl("keine"));
    pruefe_ergebnis(&u, "Ehepaar, B nach Korrektur keine", 1_383_800, 0);
    let felder = u.stand_felder(&fall);
    pp!("I.eine_zurueckgenommene_antwort_laesst_das_bundesland_stehen", felder["kist_bundesland"]["wert"] == json!("bayern"), "das Bundesland steht nach der Rücknahme nicht mehr im Fall: {}", felder["kist_bundesland"]);
    // Und wieder hin: B ist wieder Mitglied, das Bundesland steht noch, die Kirchensteuer ist wieder die von B in Bayern.
    korrigiere(&u, "kist_konfession_partner", Gib::Wahl("evangelisch"));
    pruefe_ergebnis(&u, "Ehepaar, B nach zweiter Korrektur wieder evangelisch", 1_383_800, 9_683);
    pruefe_konsole(&u, "Ehepaar");
}

/// Die KI-Sitzung bis zur Bestaetigung: Fragebogen-Weg, die Ankreuzliste mit dem Kreuz des Partners, der Satz im Chat, jede
/// Zeile der Seite „Das habe ich verstanden“ mit „Stimmt“, dann „Weiter“. Das Sprachmodell ersetzt `ModellErsatz`.
fn ki_sitzung(u: &Umgebung, modell: &ModellErsatz, satz: &str, vorschlaege: &[Value]) {
    modell.setze_vorschlaege(vorschlaege);
    bis_zur_ersten_frage(u, "gesamt", &["kein_kap_partner"]);
    u.s.tippe("#chat-text", satz);
    u.s.klick("#chat-send");
    u.s.warte_bis("(() => { const e = document.getElementById('verstanden'); return !!e && !e.hidden; })()", 15, "die Seite „Das habe ich verstanden“");
    for v in vorschlaege {
        let feld = v["feld_id"].as_str().unwrap();
        u.s.klick(&format!(".v-zeile[data-feld='{feld}'] .v-ok"));
        u.s.warte_bis(&format!("!!document.querySelector(\".v-zeile[data-feld='{feld}'].v-fertig\")"), 10, "die Zeile gilt als bestaetigt");
    }
    u.s.klick("#verstanden-weiter");
    u.s.warte_sichtbar("#wegpunkt", 10);
}

#[test]
fn fluss_i_ki_weg_beantwortet_die_konfession_zuerst_dann_steht_die_bundesland_frage() {
    let _sperre = crate::browser::seriell();
    let modell = ModellErsatz::starte(json!({}));
    let u = umgebung_oder_ende!(Start { llm_basis: Some(modell.basis.clone()), ..Start::default() }, 1280, 900);
    let satz = "Wir sind verheiratet, ich gehöre keiner Kirche an, mein Ehepartner ist evangelisch.";

    // Hier ist die Konfession von A schon beantwortet, wenn die Warteschlange zum Bundesland kommt: nur die ODER-Bedingung
    // (d-43) haelt die Frage dann noch offen. Der Weg durch den Fragebogen allein erreicht diesen Fall nie (siehe oben).
    ki_sitzung(
        &u,
        &modell,
        satz,
        &[
            json!({"feld_id": "veranlagung", "wert": "zusammen", "beleg": "verheiratet"}),
            json!({"feld_id": "kist_konfession", "wert": "keine", "beleg": "keiner Kirche"}),
            json!({"feld_id": "kist_konfession_partner", "wert": "evangelisch", "beleg": "mein Ehepartner ist evangelisch"}),
        ],
    );
    let verlauf = durchlauf(&u, &ehepaar("keine", "evangelisch", "bayern"), &[]);
    pp!("I.nach_der_ki_antwort_steht_die_bundesland_frage_fuer_den_ehegatten", wie_oft(&verlauf, "kist_bundesland") == 1, "die Frage nach dem Bundesland kam {}-mal", wie_oft(&verlauf, "kist_bundesland"));
    pp!(
        "I.nach_der_ki_antwort_steht_die_bundesland_frage_fuer_den_ehegatten",
        wie_oft(&verlauf, "kist_konfession") == 0 && wie_oft(&verlauf, "kist_konfession_partner") == 0,
        "beide Konfessionen sind beantwortet und duerfen nicht noch einmal kommen: {:?}",
        verlauf.iter().map(|s| &s.feld).filter(|f| f.starts_with("kist_")).collect::<Vec<_>>()
    );
    pruefe_ergebnis(&u, "KI: A keine, B evangelisch, Bayern", 1_383_800, 9_683);

    // Gegenprobe: ist auch der Ehegatte kein Mitglied, steht die Frage nicht.
    let satz = "Wir sind verheiratet, ich gehöre keiner Kirche an, mein Ehepartner auch nicht.";
    ki_sitzung(
        &u,
        &modell,
        satz,
        &[
            json!({"feld_id": "veranlagung", "wert": "zusammen", "beleg": "verheiratet"}),
            json!({"feld_id": "kist_konfession", "wert": "keine", "beleg": "keiner Kirche"}),
            json!({"feld_id": "kist_konfession_partner", "wert": "keine", "beleg": "mein Ehepartner auch nicht"}),
        ],
    );
    let verlauf = durchlauf(&u, &ehepaar("keine", "keine", "bayern"), &[]);
    pp!("I.ohne_mitglied_steht_die_bundesland_frage_nicht", wie_oft(&verlauf, "kist_bundesland") == 0, "die Frage nach dem Bundesland kam {}-mal, obwohl niemand Mitglied ist", wie_oft(&verlauf, "kist_bundesland"));
    pruefe_ergebnis(&u, "KI: A keine, B keine", 1_383_800, 0);
    pruefe_konsole(&u, "KI-Weg");
}

#[test]
fn fluss_i_einzelnutzer_nur_lohn_mit_kirchensteuer() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    let mut tabelle = person_a();
    tabelle.push(("veranlagung", Gib::Wahl("einzel")));
    tabelle.extend(kirche("evangelisch", None, "nordrhein_westfalen"));
    let verlauf = durchlauf(&u, &tabelle, &[]);
    // Einzelveranlagung: kein Feld des Ehegatten, die Frage nach dem Bundesland genau einmal.
    pp!(
        "I.der_einzelnutzer_wird_nichts_zum_ehegatten_gefragt",
        verlauf.iter().all(|s| !s.feld.ends_with("_partner")),
        "Fragen zum Ehegatten bei Einzelveranlagung: {:?}",
        verlauf.iter().filter(|s| s.feld.ends_with("_partner")).map(|s| &s.feld).collect::<Vec<_>>()
    );
    pp!("I.der_einzelnutzer_wird_nichts_zum_ehegatten_gefragt", wie_oft(&verlauf, "kist_bundesland") == 1, "Bundesland {}-mal gefragt", wie_oft(&verlauf, "kist_bundesland"));
    eprintln!(
        "REIHENFOLGE einzel: Bundesland {:?}, Konfession {:?}, Schritte {}",
        pos(&verlauf, "kist_bundesland"),
        pos(&verlauf, "kist_konfession"),
        verlauf.len()
    );
    pruefe_ergebnis(&u, "Einzelveranlagung, evangelisch, NRW", 1_392_400, 125_316);
    pruefe_konsole(&u, "Einzelnutzer");
}
