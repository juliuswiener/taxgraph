//! Fluss G: der Absendeknopf. Er prueft lokal und sendet NICHTS ans Finanzamt; die Anzeige darf „nicht geprueft“,
//! „beanstandet“ und „in Ordnung“ nie verwechseln.
//!
//! Gegenstueck zu `tests/test_absendeknopf_im_browser.py` (14 Funktionen). GELD und DATEN: ein Nutzer, der „nicht
//! geprueft“ fuer „in Ordnung“ haelt, reicht eine ungeprueft gebliebene Erklaerung ein; ein Doppelklick startet zwei
//! Pruefungen; rohe Servertexte (Pfade, Ausnahmen) duerfen nie auf dem Schirm stehen.
//!
//! Zwei Tests. `fluss_g_absendeknopf_ohne_eric` braucht nur Browser und Dienst (die Server-Antworten sind echt oder
//! im Browser gespielt: `window.fetch` statt `page.route`). `fluss_g_absendeknopf_mit_eric_attrappe` laesst den
//! Dienst gegen die ATTRAPPE von `libericapi.so` laufen (`rust/elster/tests/eric_attrappe/eric_attrappe.c`, wie
//! `einreichen_attrappe_hermetisch.rs`): sie antwortet nach einem Skript, sendet nichts, braucht kein Zertifikat.
//! `ERIC_DIR` zeigt in ein Wegwerf-Verzeichnis, die echte Bibliothek wird nie geladen. Der XML-Schreiber verlangt eine
//! Hersteller-Kennung; es ist ein Platzhalter aus Nullen, KEINE echte Kennung. Das Schema `E10-2025.xsd` kommt aus der
//! lokalen ERiC-Auslieferung; fehlt es, ist der Test rot, ausser mit `TAXGRAPH_OHNE_XSD=1` (die CI hat sie nicht).
//!
//! Python ersetzte `checkest_gate.validate`, `erzeuge_xml` und `deklariere` per `monkeypatch`. Hier gibt es diese
//! Naht nicht (kein Quelltext-Eingriff): `rc` kommt aus dem Skript der Attrappe, `xml_nicht_baubar` aus einem Fall ohne
//! Geburtsdatum, `eric_nicht_verfuegbar` aus einem Dienst ohne Bibliothek, `nicht_deklariert` aus der Konfession
//! „andere“.

use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

use crate::browser::Sitzung;
use crate::dienst::{wurzel, Dienst, Start};
use crate::seite::oeffne_startseite;

const STANDARD_SATZ: &str = "Aus der Prüfung liegt kein Ergebnis vor. Der Fall gilt als offen.";
const MARKE: &str = "GEHEIM-PFAD-eric.log";

/// Der Satz je Ursache (Wortlaut von Julius, 2026-10-03): als Literal, damit eine stille Aenderung der Tabelle auffaellt.
fn satz(ursache: &str) -> &'static str {
    match ursache {
        "scheibe_nicht_abgabefaehig" => {
            "Dieser Fall ist eine Teilrechnung und kann keine Erklärung tragen. Lege ihn mit der vollständigen \
             Berechnung an, wenn du eine Erklärung brauchst."
        }
        "xml_nicht_baubar" => "Aus deinen Angaben lässt sich noch keine Erklärung erzeugen. Der Fall gilt als offen.",
        "eric_nicht_verfuegbar" => "Das Prüfprogramm der Finanzverwaltung ist auf diesem Rechner nicht verfügbar.",
        "kein_pruefmodul_fuer_vz" => "Für das Veranlagungsjahr 2025 gibt es im Prüfprogramm kein Prüfmodul.",
        "hersteller_id_gesperrt" => {
            "Die Hersteller-Kennung dieses Programms ist beim Prüfprogramm gesperrt. Das liegt nicht an deinen Angaben."
        }
        "io_reader_unerwartete_elemente" => {
            "Das Prüfprogramm hat die erzeugte Erklärung nicht gelesen: Sie enthält Elemente, die es nicht erwartet."
        }
        andere => panic!("unbekannte Ursache {andere}"),
    }
}

// ---- Wege durch die Oberflaeche ---------------------------------------------------------------------------------

/// Die Seite des Dienstes unter `basis` laden und direkt auf den Schirm `#fertig` springen (der Knopf wird geprueft,
/// nicht der Weg dorthin).
fn fertig_screen(s: &Sitzung, basis: &str, fall: &str) {
    oeffne_startseite(s, basis);
    s.skript(
        &format!(
            "FALL = '{fall}'; document.getElementById('login').hidden = true; document.getElementById('start').hidden = true; \
             document.getElementById('flow').hidden = false; document.getElementById('fertig').hidden = false;"
        ),
        &[],
    );
}

/// Klicken, warten, bis ein Ergebnis steht; dann der Text je Absatz von `#einreichen-status` (None: kommt nicht).
fn absaetze(s: &Sitzung) -> Option<Vec<String>> {
    s.klick("#einreichen-btn");
    s.try_warte_bis("document.getElementById('einreichen-status').textContent.trim().length > 0", 10)?;
    let v = s.ausdruck("Array.from(document.getElementById('einreichen-status').children).map(e => e.textContent)");
    Some(v.as_array()?.iter().map(|t| t.as_str().unwrap_or_default().to_owned()).collect())
}

macro_rules! absaetze {
    ($id:literal, $s:expr) => {{
        let a = absaetze($s);
        pp!($id, a.is_some(), "die Anzeige des Ergebnisses kommt nicht");
        a.unwrap_or_default()
    }};
}

/// Der Server antwortet (gespielt) mit `body`: `window.fetch` liefert fuer `/einreichen` die Antwort selbst, alles
/// andere geht durch. Ersetzt `page.route`.
fn spiele_antworten_ein(s: &Sitzung) {
    s.skript(
        "window.__SPIEL = null; if (!window.__ECHT) { window.__ECHT = window.fetch; } \
         window.fetch = (u, o) => (String(u).includes('/einreichen') && window.__SPIEL) \
           ? Promise.resolve(new Response(JSON.stringify(window.__SPIEL.body), \
               {status: window.__SPIEL.status, headers: {'Content-Type': 'application/json'}})) \
           : window.__ECHT(u, o);",
        &[],
    );
}

fn gespielt(s: &Sitzung, status: u16, body: &Value) -> Option<Vec<String>> {
    s.skript("window.__SPIEL = {status: arguments[0], body: arguments[1]};", &[json!(status), body.clone()]);
    absaetze(s)
}

/// Die Regeln jeder „nicht geprueft“-Anzeige (`_pruefe_nicht_geprueft`): Ueberschrift, Satz, optional Pruefcode; kein
/// Wort nach Urteil oder Freigabe.
fn ist_nicht_geprueft(a: &[String], satz: &str, pruefcode: Option<i64>) -> bool {
    let code: Vec<String> = pruefcode.map(|c| format!("Prüfcode: {c}")).into_iter().collect();
    let kl = a.join(" ").to_lowercase();
    a.len() >= 2 && a[0] == "Nicht geprüft." && a[1] == satz && a[2..] == code[..] && !kl.contains("beanstandet") && !kl.contains("ordnung")
}

// ---- Test 1: Knopf, Anzeige und Sperrgruende, ohne ERiC -------------------------------------------------------------

#[test]
fn fluss_g_absendeknopf_ohne_eric() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 640);
    let (s, d) = (&u.s, &u.dienst);

    // Struktur: der Knopf steht im Schirm `#fertig`, NACH `#preflight`, ohne Formular-Absenden.
    d.fall("gesamt", "knopf-struktur");
    fertig_screen(s, &d.basis, "knopf-struktur");
    pp!("G.knopf_ist_im_fertig_screen_nach_preflight", s.finde("#einreichen-btn").is_some(), "kein #einreichen-btn im DOM: der Absendeknopf fehlt");
    pp!(
        "G.knopf_ist_im_fertig_screen_nach_preflight",
        s.ausdruck("document.getElementById('einreichen-btn').getAttribute('type')") == json!("button"),
        "Knopf ohne type=button riskiert Form-Submit"
    );
    let reihenfolge = s.ausdruck("Array.from(document.getElementById('fertig').querySelectorAll('#preflight, #einreichen-btn')).map(k => k.id)");
    pp!("G.knopf_ist_im_fertig_screen_nach_preflight", reihenfolge == json!(["preflight", "einreichen-btn"]), "Reihenfolge im #fertig-Screen falsch: {reihenfolge}");

    // Vor dem ersten Klick steht sichtbar, dass nur lokal geprueft und nichts ans Finanzamt gesendet wird.
    d.fall("gesamt", "knopf-vorab-satz");
    fertig_screen(s, &d.basis, "knopf-vorab-satz");
    pp!("G.vor_dem_klick_steht_nur_lokal_nichts_ans_finanzamt", s.sichtbar("#einreichen-lokal"), "kein sichtbarer Satz vor dem Absendeknopf");
    let text = s.textinhalt("#einreichen-lokal");
    pp!("G.vor_dem_klick_steht_nur_lokal_nichts_ans_finanzamt", text.contains("lokal") && text.contains("Finanzamt"), "Satz sagt nicht lokal/Finanzamt: {text:?}");
    let davor = s.ausdruck(
        "!!(document.getElementById('einreichen-lokal').compareDocumentPosition(document.getElementById('einreichen-btn')) \
         & Node.DOCUMENT_POSITION_FOLLOWING)",
    );
    pp!("G.vor_dem_klick_steht_nur_lokal_nichts_ans_finanzamt", davor == json!(true), "der Satz muss VOR dem Knopf stehen");
    pp!(
        "G.vor_dem_klick_steht_nur_lokal_nichts_ans_finanzamt",
        s.ausdruck("document.getElementById('einreichen-btn').getAttribute('aria-describedby')") == json!("einreichen-lokal"),
        "aria-describedby fehlt: ein Screenreader liest den Satz am Knopf nicht vor"
    );

    // Doppelklick: der Knopf ist gesperrt, solange die Pruefung laeuft (sonst zwei ERiC-Laeufe zugleich).
    d.fall("gesamt", "zustand-doppelklick");
    fertig_screen(s, &d.basis, "zustand-doppelklick");
    s.skript(
        "window.__EINREICHEN_OFFEN = null; window.__FETCH_ECHT = window.fetch; \
         window.fetch = (url, opt) => { if (String(url).includes('/einreichen')) { \
           return new Promise((res, rej) => { window.__EINREICHEN_OFFEN = { res, rej }; }); } \
           return window.__FETCH_ECHT(url, opt); };",
        &[],
    );
    s.klick("#einreichen-btn");
    let offen = s.try_warte_bis("window.__EINREICHEN_OFFEN !== null", 10);
    pp!("G.doppel_klick_schuetzt_gegen_doppel_submit", offen.is_some(), "der Klick hat keine Anfrage an /einreichen ausgeloest");
    pp!(
        "G.doppel_klick_schuetzt_gegen_doppel_submit",
        s.ausdruck("document.getElementById('einreichen-btn').disabled") == json!(true),
        "Knopf muss waehrend der laufenden Pruefung disabled sein"
    );
    s.skript(
        "const offen = window.__EINREICHEN_OFFEN; setTimeout(() => offen.res(new Response(JSON.stringify(arguments[0]), \
         { status: 200, headers: { 'Content-Type': 'application/json' } })), 0);",
        &[json!({"fall_id": "zustand-doppelklick", "eingereicht": false, "basis_snapshot": "x", "befund_gebunden": false,
                 "vz": 2025, "rc": 0, "klasse": "plausibel", "xml_bytes": 10, "plausibel": true, "hinweis": "checkESt bestanden."})],
    );
    let da = s.try_warte_bis("document.getElementById('einreichen-status').textContent.trim().length > 0", 10);
    pp!("G.doppel_klick_schuetzt_gegen_doppel_submit", da.is_some(), "die gespielte Antwort wird nicht angezeigt");
    pp!(
        "G.doppel_klick_schuetzt_gegen_doppel_submit",
        s.ausdruck("document.getElementById('einreichen-btn').disabled") == json!(false),
        "Knopf muss nach der Antwort wieder klickbar sein"
    );

    // Ein Schluessel, den die Tabelle nicht kennt, zeigt den Rueckfallsatz und bleibt „Nicht geprüft.“ (nie „in Ordnung“);
    // auch Namen, die ein JS-Objekt von Haus aus hat, sind keine Treffer.
    d.fall("gesamt", "unbekannt");
    fertig_screen(s, &d.basis, "unbekannt");
    spiele_antworten_ein(s);
    let rueckfall = json!(["Nicht geprüft.", STANDARD_SATZ]);
    for grund in ["gibt_es_nicht", "constructor", "__proto__", "toString"] {
        let a = gespielt(s, 422, &json!({"fall_id": "unbekannt", "eingereicht": false, "grund": grund}));
        pp!("G.unbekannter_schluessel_zeigt_den_rueckfallsatz", a.as_ref().is_some_and(|a| json!(a) == rueckfall), "grund={grund:?}: {a:?}");
    }
    for klasse in ["gibt_es_nicht", "constructor", "__proto__"] {
        let a = gespielt(
            s,
            422,
            &json!({"fall_id": "unbekannt", "eingereicht": false, "grund": "rc_kein_plausibilitaetsverdikt", "klasse": klasse, "rc": 777_777}),
        );
        pp!(
            "G.unbekannter_schluessel_zeigt_den_rueckfallsatz",
            a.as_ref().is_some_and(|a| json!(a) == json!(["Nicht geprüft.", STANDARD_SATZ, "Prüfcode: 777777"])),
            "klasse={klasse:?}: {a:?}"
        );
    }
    let a = gespielt(s, 500, &json!({"fehler": "irgendwas"}));
    pp!("G.unbekannter_schluessel_zeigt_den_rueckfallsatz", a.as_ref().is_some_and(|a| json!(a) == rueckfall), "kein grund und kein 200 ist nicht „in Ordnung“: {a:?}");

    // Erfolg bei nachweislich leerer `nicht_deklariert`: „in Ordnung“, ohne Beanstandung und ohne „nicht geprueft“
    // (gespielt: der echte Fall hat nie eine leere Liste, s. Test 2).
    let a = gespielt(
        s,
        200,
        &json!({"fall_id": "unbekannt", "eingereicht": false, "rc": 0, "klasse": "plausibel", "plausibel": true,
                "hinweis": "checkESt bestanden.", "nicht_deklariert": []}),
    )
    .unwrap_or_default();
    let tl = a.join(" ").to_lowercase();
    pp!("G.erfolg_zeigt_in_ordnung_ohne_beanstandung_oder_unsicherheit", tl.contains("ordnung"), "kein Erfolgs-Wortlaut in: {a:?}");
    pp!("G.erfolg_zeigt_in_ordnung_ohne_beanstandung_oder_unsicherheit", !tl.contains("beanstandet") && !tl.contains("nicht geprüft"), "{a:?}");

    // Rohe Felder des Servers (`detail`, `ericantwort`, `hinweis`) stehen nie auf dem Schirm; die Antwort wird gezeigt.
    d.fall("gesamt", "roh");
    let formen = [
        ("rc_kein_plausibilitaetsverdikt", json!({"klasse": "sonstig", "rc": 610_301_006})),
        ("kein_pruefmodul_fuer_vz", json!({"klasse": "datenartversion_unbekannt", "rc": 610_001_042, "vz": 2025})),
        ("xml_nicht_baubar", json!({})),
        ("eric_nicht_verfuegbar", json!({})),
        ("scheibe_nicht_abgabefaehig", json!({"scheibe": "an_gesamt"})),
        ("plausibilitaet_verletzt", json!({"klasse": "plausibilitaet_fehler", "rc": 610_001_002})),
    ];
    for feld in ["detail", "ericantwort", "hinweis"] {
        fertig_screen(s, &d.basis, "roh");
        spiele_antworten_ein(s);
        for (grund, rest) in &formen {
            let mut body = json!({"fall_id": "roh", "eingereicht": false, "grund": grund});
            for (k, v) in rest.as_object().unwrap() {
                body[k] = v.clone();
            }
            body[feld] = json!(MARKE);
            let a = gespielt(s, 422, &body).unwrap_or_default();
            let text = a.join(" ");
            pp!(
                "G.roh_felder_des_servers_stehen_nie_auf_dem_schirm",
                a.first().is_some_and(|k| k == "Nicht geprüft." || k == "Geprüft und beanstandet."),
                "{grund}: keine Anzeige des Ergebnisses: {a:?}"
            );
            pp!(
                "G.roh_felder_des_servers_stehen_nie_auf_dem_schirm",
                !text.contains(MARKE) && !text.contains("eric.log"),
                "{grund}: Feld {feld:?} steht roh auf dem Schirm: {a:?}"
            );
        }
    }

    // Eine Teilrechnung traegt keine Erklaerung: 409 `scheibe_nicht_abgabefaehig`, der feste Satz, kein Pruefcode.
    d.fall("an_gesamt", "teilscheibe");
    fertig_screen(s, &d.basis, "teilscheibe");
    let a = absaetze!("G.je_ursache_ein_fester_text_und_alle_verschieden", s);
    pp!("G.je_ursache_ein_fester_text_und_alle_verschieden", ist_nicht_geprueft(&a, satz("scheibe_nicht_abgabefaehig"), None), "scheibe_nicht_abgabefaehig: {a:?}");

    // Der Sperrgrund bringt seinen Klartext mit; die Anzeige zeigt ihn statt des Sammelsatzes. Echter Fall, ERiC wird
    // nie erreicht: ein bestaetigtes `dhf_unterkunftskosten_monat` ergibt `dhf_tatbestand_offen`.
    d.fall("gesamt", "sperre");
    let (st, r) = d.post(
        "/fall/sperre/event",
        &json!({"feld_id": "dhf_unterkunftskosten_monat", "wert": 140_000, "zustand": "bestaetigt",
                "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}, "schreiber": "ui:laie",
                "signal": {"signal_1": {"typ": "laie_eingabe"}, "signal_2": "laie_bestaetigt"}}),
    );
    assert_eq!(st, 201, "{r}");
    let (st, erg) = d.get("/fall/sperre/ergebnis");
    pp!("G.sperrgrund_zeigt_den_klartext_des_servers", st == 200 && erg["grund"] == json!("dhf_tatbestand_offen"), "der Fall sperrt nicht aus dem erwarteten Grund: {erg}");
    let klartext = erg["klartext"].as_str().unwrap_or_default().to_owned();
    pp!("G.sperrgrund_zeigt_den_klartext_des_servers", !klartext.is_empty() && !klartext.contains("erforderliche Angabe fehlt"), "Klartext des Servers: {klartext:?}");
    fertig_screen(s, &d.basis, "sperre");
    let a = absaetze!("G.sperrgrund_zeigt_den_klartext_des_servers", s);
    pp!("G.sperrgrund_zeigt_den_klartext_des_servers", ist_nicht_geprueft(&a, &klartext, None), "{a:?}");
    pp!("G.sperrgrund_zeigt_den_klartext_des_servers", !a.join(" ").contains("erforderliche Angabe fehlt"), "Sammelsatz steht noch da: {a:?}");
}

// ---- Test 2: gegen die Attrappe von libericapi.so ---------------------------------------------------------------------

fn lade_fixture(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(wurzel().join(format!("rust/fixtures/e2e/{name}.json"))).unwrap()).unwrap()
}

/// Der volle `gesamt`-Fall als Akte unter `faelle/<name>.json` des Dienstes.
fn lege_fall_an(d: &Dienst, name: &str, mut akte: Value) {
    akte["fall_id"] = json!(name);
    std::fs::write(d.daten().join("faelle").join(format!("{name}.json")), serde_json::to_vec(&akte).unwrap()).unwrap();
}

fn setze(akte: &mut Value, feld: &str, wert: &Value) {
    for e in akte["events"].as_array_mut().unwrap() {
        if e["feld_id"] == feld {
            e["wert"] = wert.clone();
        }
    }
}

fn baue_attrappe(ziel: &Path) {
    let quelle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../elster/tests/eric_attrappe/eric_attrappe.c");
    let st = Command::new("cc")
        .args(["-shared", "-fPIC", "-O0", "-Wall", "-Wextra", "-o"])
        .arg(ziel)
        .arg(&quelle)
        .status()
        .expect("cc fehlt: die Attrappe von libericapi.so laesst sich nicht bauen");
    assert!(st.success(), "cc scheiterte an {}", quelle.display());
}

/// Die Antwort von `ERiC` fuer den naechsten Aufruf: Zeile 1 der Rueckgabecode, danach der Puffer.
fn skript(steuer: &Path, rc: i64, puffer: &str) {
    for n in ["zaehler", "gesehen"] {
        let p = steuer.join(n);
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_dir_all(&p);
    }
    std::fs::write(steuer.join("skript"), format!("{rc}\n{puffer}")).unwrap();
}

fn aufrufe(steuer: &Path) -> usize {
    let mut n = 0;
    while steuer.join(format!("gesehen/{}.xml", n + 1)).exists() {
        n += 1;
    }
    n
}

fn fehlerliste(n: usize) -> String {
    use std::fmt::Write as _;
    let mut s = String::from("<EricAntwort><Rueckgabe>");
    for i in 0..n {
        let _ = write!(s, "<FehlerRegelpruefung><Nr>{i}</Nr><Text>Regel {i} verletzt</Text></FehlerRegelpruefung>");
    }
    s.push_str("</Rueckgabe></EricAntwort>");
    s
}

#[test]
fn fluss_g_absendeknopf_mit_eric_attrappe() {
    let _sperre = crate::browser::seriell();
    let Some(xsd) = elster::finde_schema(2025, "E10-{jahr}.xsd") else {
        assert!(
            std::env::var("TAXGRAPH_OHNE_XSD").as_deref() == Ok("1"),
            "E10-2025.xsd fehlt in der lokalen ERiC-Auslieferung ($ERIC_DIR, ~/02_Software/eric); nur TAXGRAPH_OHNE_XSD=1 erlaubt das Fehlen"
        );
        eprintln!("[oberflaeche/fluss_g] UEBERSPRUNGEN: E10-2025.xsd fehlt, TAXGRAPH_OHNE_XSD=1");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    // Eine Auslieferung MIT Attrappe und eine OHNE Bibliothek (nur das Schema): ERiC ist dann „nicht verfuegbar“.
    let eric = tmp.path().join("eric");
    let eric_leer = tmp.path().join("eric_leer");
    for ordner in [&eric, &eric_leer] {
        std::fs::create_dir_all(ordner.join("schema/2025")).unwrap();
        std::fs::copy(&xsd, ordner.join("schema/2025/E10-2025.xsd")).unwrap();
    }
    std::fs::create_dir_all(eric.join("lib")).unwrap();
    baue_attrappe(&eric.join("lib/libericapi.so"));
    let steuer = tmp.path().join("steuer");
    std::fs::create_dir_all(&steuer).unwrap();
    let env = |dir: &Path| {
        Start {
            umgebung: vec![
                ("ERIC_DIR".into(), dir.display().to_string()),
                ("ERIC_ATTRAPPE_DIR".into(), steuer.display().to_string()),
                ("ELSTER_HERSTELLER_ID".into(), "00000".into()),
            ],
            ..Start::default()
        }
    };
    let u = umgebung_oder_ende!(env(&eric), 360, 640);
    let (s, d) = (&u.s, &u.dienst);
    let gesamt = lade_fixture("gesamt");
    let antworten_merken = "window.__ANTWORTEN = []; { const echt = window.fetch; window.fetch = async (u, o) => { const r = await echt(u, o); \
                            window.__ANTWORTEN.push({url: String(u), methode: (o && o.method) || 'GET', status: r.status}); return r; }; }";

    // Ein echter Klick loest einen echten POST an /einreichen aus, der bis ERiC kommt (sonst ist der Knopf Dekoration).
    skript(&steuer, 0, "<Ok/>");
    lege_fall_an(d, "knopf-ruft-endpunkt", gesamt.clone());
    fertig_screen(s, &d.basis, "knopf-ruft-endpunkt");
    s.skript(antworten_merken, &[]);
    s.klick("#einreichen-btn");
    let kam = s.try_warte_bis("window.__ANTWORTEN.some(a => a.url.endsWith('/fall/knopf-ruft-endpunkt/einreichen'))", 10);
    pp!("G.klick_ruft_wirklich_den_einreichen_endpunkt", kam.is_some(), "der Klick hat keinen POST an /fall/<id>/einreichen ausgeloest");
    let antwort = s.ausdruck("window.__ANTWORTEN.find(a => a.url.endsWith('/einreichen'))");
    pp!("G.klick_ruft_wirklich_den_einreichen_endpunkt", antwort["methode"] == json!("POST") && antwort["status"] == json!(200), "{antwort}");
    pp!("G.klick_ruft_wirklich_den_einreichen_endpunkt", aufrufe(&steuer) == 1, "ERiC wurde {} Mal gefragt, erwartet 1", aufrufe(&steuer));

    // Der Text je Zustand: Erfolg, beanstandet, nicht geprueft.
    skript(&steuer, 0, "<Ok/>");
    lege_fall_an(d, "zustand-ok", gesamt.clone());
    fertig_screen(s, &d.basis, "zustand-ok");
    let a = absaetze!("G.erfolg_zeigt_in_ordnung_ohne_beanstandung_oder_unsicherheit", s);
    let tl = a.join(" ").to_lowercase();
    pp!("G.erfolg_zeigt_in_ordnung_ohne_beanstandung_oder_unsicherheit", !tl.contains("beanstandet"), "{a:?}");
    pp!("G.erfolg_zeigt_in_ordnung_ohne_beanstandung_oder_unsicherheit", !tl.contains("nicht geprüft"), "{a:?}");
    // ABWEICHUNG vom Python-Test: dort ersetzte `monkeypatch` die Deklaration durch eine mit leerer `nicht_deklariert`
    // und sah „in Ordnung“. Im echten Fall ist die Liste NIE leer (Steuerungs-Flaggen wie `kein_gewinn` stehen immer
    // darin), also zeigt die echte Antwort die Hinweisform. „in Ordnung“ bei leerer Liste misst Test 1 (gespielt).
    pp!("G.erfolg_zeigt_in_ordnung_ohne_beanstandung_oder_unsicherheit", tl.contains("nicht alle werte stehen in der erklärung"), "kein Erfolgs-Wortlaut in: {a:?}");

    skript(&steuer, 610_001_002, &fehlerliste(1));
    lege_fall_an(d, "zustand-beanstandet", gesamt.clone());
    fertig_screen(s, &d.basis, "zustand-beanstandet");
    let beanstandet = absaetze!("G.beanstandet_zeigt_anderen_text_als_erfolg", s);
    let tl = beanstandet.join(" ").to_lowercase();
    pp!("G.beanstandet_zeigt_anderen_text_als_erfolg", tl.contains("beanstandet"), "kein Beanstandungs-Wortlaut in: {beanstandet:?}");
    pp!("G.beanstandet_zeigt_anderen_text_als_erfolg", !tl.contains("nicht geprüft"), "beanstandet darf nicht wie „nicht geprueft“ klingen: {beanstandet:?}");

    // Gleicher HTTP-Status 422, aber „nicht geprueft“ ist nicht „beanstandet“ (und erst recht keine Freigabe).
    skript(&steuer, 610_301_200, "");
    lege_fall_an(d, "zustand-nicht-geprueft", gesamt.clone());
    fertig_screen(s, &d.basis, "zustand-nicht-geprueft");
    let nicht_geprueft = absaetze!("G.nicht_geprueft_unterscheidet_sich_von_beanstandet_trotz_gleichem_http_status", s);
    let tl = nicht_geprueft.join(" ").to_lowercase();
    pp!("G.nicht_geprueft_unterscheidet_sich_von_beanstandet_trotz_gleichem_http_status", !tl.contains("beanstandet"), "{nicht_geprueft:?}");
    pp!("G.nicht_geprueft_unterscheidet_sich_von_beanstandet_trotz_gleichem_http_status", !tl.contains("ordnung"), "{nicht_geprueft:?}");
    skript(&steuer, 610_001_002, &fehlerliste(1));
    lege_fall_an(d, "zustand-beanstandet-vergleich", gesamt.clone());
    fertig_screen(s, &d.basis, "zustand-beanstandet-vergleich");
    let vergleich = absaetze!("G.nicht_geprueft_unterscheidet_sich_von_beanstandet_trotz_gleichem_http_status", s);
    pp!(
        "G.nicht_geprueft_unterscheidet_sich_von_beanstandet_trotz_gleichem_http_status",
        nicht_geprueft != vergleich,
        "beide Faelle liefern denselben Browser-Text: nicht_geprueft={nicht_geprueft:?} beanstandet={vergleich:?}"
    );

    // Nicht uebertragene Werte: der 200-Rumpf traegt `nicht_deklariert`, die Anzeige nennt jeden Eintrag.
    // Die Konfession „andere“ hat keinen XSD-Code (E0100402); wer einer Kirche angehoert, muss beide Betraege nennen.
    let mut nd_akte = gesamt.clone();
    setze(&mut nd_akte, "kist_konfession", &json!("andere"));
    let nd_fall = |name: &str| {
        lege_fall_an(d, name, nd_akte.clone());
        for feld in ["kist_gezahlt", "kist_erstattet"] {
            let (st, r) = d.schreibe(name, feld, &json!(0));
            assert_eq!(st, 201, "{feld}: {r}");
        }
    };
    skript(&steuer, 0, "<Ok/>");
    nd_fall("nd-body");
    let (st, r) = d.post("/fall/nd-body/einreichen", &json!({}));
    pp!("G.einreichen_200_traegt_nicht_deklariert", st == 200, "{st} {r}");
    let liste = r["nicht_deklariert"].as_array().cloned();
    pp!("G.einreichen_200_traegt_nicht_deklariert", liste.is_some(), "ohne `nicht_deklariert` im 200-Body zeigte die Oberflaeche „in Ordnung“: {r}");
    let liste = liste.unwrap_or_default();
    let eintrag = liste.iter().find(|e| e["feld_id"] == json!("kist_konfession")).cloned().unwrap_or(Value::Null);
    pp!(
        "G.einreichen_200_traegt_nicht_deklariert",
        eintrag["grund"].as_str().is_some_and(|g| g.contains("E0100402")) && eintrag["hinweis"].as_str().is_some_and(|h| h.contains("Mein ELSTER")),
        "die Konfession „andere“ fehlt in nicht_deklariert oder ohne Grund/Hinweis: {liste:?}"
    );

    skript(&steuer, 0, "<Ok/>");
    nd_fall("nd-anzeige");
    fertig_screen(s, &d.basis, "nd-anzeige");
    let a = absaetze!("G.nicht_deklariert_zeigt_jeden_eintrag_statt_in_ordnung", s);
    let klickbar = s.ausdruck("!document.getElementById('einreichen-btn').disabled");
    let text = a.join("\n");
    pp!("G.nicht_deklariert_zeigt_jeden_eintrag_statt_in_ordnung", !text.to_lowercase().contains("in ordnung"), "„in Ordnung“ trotz nicht uebertragener Werte: {a:?}");
    for e in &liste {
        let (f, g) = (e["feld_id"].as_str().unwrap_or("?"), e["grund"].as_str().unwrap_or("?"));
        pp!("G.nicht_deklariert_zeigt_jeden_eintrag_statt_in_ordnung", text.contains(f) && text.contains(g), "Eintrag fehlt: {e} in {a:?}");
        if let Some(h) = e["hinweis"].as_str() {
            pp!("G.nicht_deklariert_zeigt_jeden_eintrag_statt_in_ordnung", text.contains(h), "hinweis fehlt: {h} in {a:?}");
        }
    }
    pp!("G.nicht_deklariert_zeigt_jeden_eintrag_statt_in_ordnung", klickbar == json!(true), "Knopf muss nach der Antwort klickbar bleiben: keine neue Sperre");

    // Je Ursache ein fester Text, und kein Fall zeigt dasselbe wie ein anderer.
    let mut ohne_geburtsdatum = gesamt.clone();
    ohne_geburtsdatum["events"].as_array_mut().unwrap().retain(|e| e["feld_id"] != "stammdaten_geburtsdatum");
    let faelle: [(&str, Option<i64>, &str, &str, Option<&Value>); 7] = [
        ("hersteller_id_gesperrt", Some(610_301_202), satz("hersteller_id_gesperrt"), "", None),
        ("kein_pruefmodul_fuer_vz", Some(610_001_042), satz("kein_pruefmodul_fuer_vz"), "<Fehler/>", None),
        ("io_reader_unerwartete_elemente", Some(610_301_106), satz("io_reader_unerwartete_elemente"), "", None),
        ("io_gate_nicht_geprueft", Some(610_301_200), STANDARD_SATZ, "", None),
        ("sonstig", Some(610_301_006), STANDARD_SATZ, "", None),
        ("xml_nicht_baubar", None, satz("xml_nicht_baubar"), "", Some(&ohne_geburtsdatum)),
        ("eric_nicht_verfuegbar", None, satz("eric_nicht_verfuegbar"), "", None),
    ];
    let mut gesehen: Vec<(&str, Vec<String>)> = Vec::new();
    for (i, (name, code, erwarteter_satz, puffer, akte)) in faelle.iter().enumerate() {
        let fall = format!("ursache-{i}");
        let a = if *name == "eric_nicht_verfuegbar" {
            // Ein zweiter Dienst, dessen Auslieferung keine Bibliothek hat.
            let d2 = Dienst::starte(&env(&eric_leer));
            lege_fall_an(&d2, &fall, gesamt.clone());
            fertig_screen(s, &d2.basis, &fall);
            absaetze!("G.je_ursache_ein_fester_text_und_alle_verschieden", s)
        } else {
            skript(&steuer, code.unwrap_or(0), puffer);
            lege_fall_an(d, &fall, akte.cloned().unwrap_or_else(|| gesamt.clone()));
            fertig_screen(s, &d.basis, &fall);
            absaetze!("G.je_ursache_ein_fester_text_und_alle_verschieden", s)
        };
        pp!("G.je_ursache_ein_fester_text_und_alle_verschieden", ist_nicht_geprueft(&a, erwarteter_satz, *code), "{name}: {a:?}");
        gesehen.push((name, a));
    }
    // Die Teilrechnung (aus Test 1) zaehlt zu den Ursachen: ihr Text darf keinem anderen gleichen.
    d.fall("an_gesamt", "teilscheibe");
    fertig_screen(s, &d.basis, "teilscheibe");
    let a = absaetze!("G.je_ursache_ein_fester_text_und_alle_verschieden", s);
    pp!("G.je_ursache_ein_fester_text_und_alle_verschieden", ist_nicht_geprueft(&a, satz("scheibe_nicht_abgabefaehig"), None), "scheibe_nicht_abgabefaehig: {a:?}");
    gesehen.push(("scheibe_nicht_abgabefaehig", a));
    let verschieden = gesehen.iter().enumerate().all(|(i, (_, a))| gesehen.iter().skip(i + 1).all(|(_, b)| a != b));
    pp!("G.je_ursache_ein_fester_text_und_alle_verschieden", verschieden, "verschiedene Ursachen, gleiche Anzeige: {gesehen:?}");

    // Der Pruefcode steht in einer eigenen Zeile; ohne `rc` (503) gibt es keine.
    for (i, (rc, satz_soll)) in [(610_301_200, STANDARD_SATZ), (610_301_106, satz("io_reader_unerwartete_elemente")), (610_301_006, STANDARD_SATZ)]
        .into_iter()
        .enumerate()
    {
        skript(&steuer, rc, "");
        let fall = format!("code-{i}");
        lege_fall_an(d, &fall, gesamt.clone());
        fertig_screen(s, &d.basis, &fall);
        let a = absaetze!("G.pruefcode_steht_in_einer_eigenen_zeile", s);
        pp!("G.pruefcode_steht_in_einer_eigenen_zeile", a == ["Nicht geprüft.".to_owned(), satz_soll.to_owned(), format!("Prüfcode: {rc}")], "rc={rc}: {a:?}");
    }
    let d2 = Dienst::starte(&env(&eric_leer));
    lege_fall_an(&d2, "code-503", gesamt);
    fertig_screen(s, &d2.basis, "code-503");
    let a = absaetze!("G.pruefcode_steht_in_einer_eigenen_zeile", s);
    pp!("G.pruefcode_steht_in_einer_eigenen_zeile", a == ["Nicht geprüft.".to_owned(), satz("eric_nicht_verfuegbar").to_owned()], "ohne rc keine Pruefcode-Zeile: {a:?}");
}
