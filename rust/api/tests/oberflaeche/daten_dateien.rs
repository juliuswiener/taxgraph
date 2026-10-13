//! Was an der Oberflaeche haengt und keinen Browser braucht: die Sicherheitskoepfe der ausgelieferten Seite, die
//! HTML-Dateien unter dieser Richtlinie, und die Bindungsdaten, aus denen die Oberflaeche Ankreuzliste, Typen und
//! Instanz-Achse liest. Dienst auf Wegwerf-Daten wie in den Fluessen, nie `~/.local/share/taxgraph`.
//!
//! Jede Pruefung hat eine Untergrenze („es gibt ueberhaupt …“): ein Test ueber eine leere Menge waere von selbst gruen.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::browser::{http_roh, seriell};
use crate::dienst::{wurzel, Dienst, Start};

/// Der Wert des Kopfs `Content-Security-Policy` einer GET-Antwort (leer, wenn er fehlt).
fn csp(d: &Dienst, pfad: &str) -> String {
    let (status, kopf, _) = http_roh(d.port, "GET", pfad, None, &[]);
    assert_eq!(status, 200, "GET {pfad}");
    kopf.lines()
        .filter_map(|z| z.split_once(':'))
        .find(|(n, _)| n.eq_ignore_ascii_case("content-security-policy"))
        .map(|(_, v)| v.trim().to_owned())
        .unwrap_or_default()
}

/// Die Richtlinie steht auf dem JSON-Weg UND auf der ausgelieferten Seite, ohne Inline-Skript und ohne `eval`.
#[test]
fn csp_header_der_ausgelieferten_seite() {
    let _sperre = seriell();
    let d = Dienst::starte(&Start::default());
    let health = csp(&d, "/health");
    let seite = csp(&d, "/");
    pp!("A.csp_auf_der_ausgelieferten_seite", health.contains("default-src 'none'"), "/health ohne default-src 'none': {health:?}");
    pp!("A.csp_auf_der_ausgelieferten_seite", seite.contains("default-src 'none'"), "die Seite ohne default-src 'none': {seite:?}");
    pp!("A.csp_auf_der_ausgelieferten_seite", seite.contains("script-src 'self'"), "die Seite ohne script-src 'self': {seite:?}");
    // Erst Existenz, dann Inhalt: „unsafe-inline“ steht auch in einem leeren String nicht drin.
    pp!("A.csp_erlaubt_kein_inline_skript", !seite.is_empty(), "gar kein CSP-Kopf: die Pruefungen darunter waeren trivial erfuellt");
    pp!("A.csp_erlaubt_kein_inline_skript", !seite.contains("unsafe-inline"), "die Richtlinie erlaubt Inline-Skript oder -Style: {seite:?}");
    pp!("A.csp_erlaubt_kein_inline_skript", !seite.contains("unsafe-eval"), "die Richtlinie erlaubt eval(): {seite:?}");

    // Ohne Python-Vorbild (Mutant S4 aus der Nachmessung in main; auch die Python-Tests pruefen nur default-src, script-src
    // und unsafe-*): keine Einbettung in einen fremden Rahmen (Clickjacking), kein umgebogenes `<base>`, kein Formular-Ziel.
    // Jede Anweisung einzeln und GANZ (`'self'` statt `'none'` ist ein anderer Satz), auf dem JSON-Weg und auf der Seite.
    for (ort, kopf) in [("/health", &health), ("/", &seite)] {
        let anweisungen: Vec<&str> = kopf.split(';').map(str::trim).collect();
        for soll in ["frame-ancestors 'none'", "base-uri 'none'", "form-action 'none'"] {
            pp!("A.csp_sperrt_einbettung_basis_und_formulare", anweisungen.contains(&soll), "{ort}: die Anweisung {soll:?} fehlt oder ist aufgeweicht: {kopf:?}");
        }
    }
}

/// Eine Richtlinie, die die eigene Oberflaeche abschaltet, waere schlimmer als keine. Gemessen gegen die Dateien:
/// kein Inline-`<script>`, kein `style=`, keine `javascript:`-URL, kein `on…=`-Handler, keine externe Ressource.
#[test]
fn csp_html_dateien_passen_zur_richtlinie() {
    let verzeichnis: PathBuf = wurzel().join("rust/api/static");
    let mut dateien: Vec<PathBuf> = std::fs::read_dir(&verzeichnis)
        .unwrap_or_else(|e| panic!("{}: {e}", verzeichnis.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "html"))
        .collect();
    dateien.sort();
    pp!("A.es_gibt_ueberhaupt_html_dateien_fuer_die_richtlinienpruefung", !dateien.is_empty(), "keine HTML-Datei unter {}: die Richtlinienpruefung prueft nichts", verzeichnis.display());
    let handler = regex::Regex::new(r"\son(click|change|input|submit|load|error)\s*=").unwrap();
    for pfad in &dateien {
        let html = std::fs::read_to_string(pfad).unwrap();
        let name = pfad.file_name().unwrap().to_string_lossy();
        pp!("A.richtlinie_passt_zur_ausgelieferten_seite", !html.contains("<script>"), "{name}: Inline-<script>: die Richtlinie schaltet es ab");
        pp!("A.richtlinie_passt_zur_ausgelieferten_seite", !html.contains("style=\""), "{name}: style=-Attribut: die Richtlinie schaltet es ab");
        pp!("A.richtlinie_passt_zur_ausgelieferten_seite", !html.contains("javascript:"), "{name}: javascript:-URL: die Richtlinie schaltet sie ab");
        pp!("A.richtlinie_passt_zur_ausgelieferten_seite", !handler.is_match(&html), "{name}: Inline-Ereignis-Handler: die Richtlinie schaltet ihn ab");
        pp!("A.richtlinie_passt_zur_ausgelieferten_seite", !html.contains("src=\"http") && !html.contains("href=\"http"), "{name}: externe Ressource: default-src 'none' blockiert sie");
    }
}

/// Die Bindungsdaten, an denen die Oberflaeche haengt: Gruppe `kind`, Ankreuzliste (`screening`), Feldtypen.
#[test]
fn bindung_der_oberflaeche() {
    let registry = bindung::lade_registry_der_wurzel(&wurzel()).expect("Registry laedt");
    let bindungen: Vec<&bindung::Bindung> = registry.dateien.iter().flat_map(|(_, d)| d.bindungen.iter()).collect();
    let gruppen: Vec<&bindung::InstanzGruppe> = registry.dateien.iter().flat_map(|(_, d)| d.instanz_gruppen.iter()).collect();

    // Untergrenze: ohne den Eintrag waere alles darunter gruen und die Achse im Betrieb tot.
    let kind = gruppen.iter().find(|g| g.gruppe == "kind");
    pp!("D.die_bindung_erklaert_die_gruppe_kind", kind.is_some(), "Gruppe `kind` fehlt: {:?}", gruppen.iter().map(|g| &g.gruppe).collect::<Vec<_>>());
    let kind = kind.unwrap();
    pp!("D.die_bindung_erklaert_die_gruppe_kind", kind.anzahl_feld == "fam_anzahl_kinder", "das Zaehlfeld heisst {}", kind.anzahl_feld);
    pp!("D.die_bindung_erklaert_die_gruppe_kind", !kind.etikett.is_empty() && kind.max >= 2, "Etikett {:?}, max {}", kind.etikett, kind.max);
    let zaehlfeld = bindungen.iter().find(|b| b.feld_id == kind.anzahl_feld);
    pp!("D.die_bindung_erklaert_die_gruppe_kind", zaehlfeld.is_some_and(|b| b.askable), "das Zaehlfeld wird gar nicht gefragt");
    pp!("D.die_bindung_erklaert_die_gruppe_kind", zaehlfeld.is_some_and(|b| b.typ == domain::Feldtyp::Int), "das Zaehlfeld ist keine Ganzzahl");
    let kind_felder = bindungen.iter().filter(|b| b.instanz_gruppe.as_deref() == Some("kind")).count();
    pp!("D.die_bindung_erklaert_die_gruppe_kind", kind_felder >= 20, "nur {kind_felder} Kind-Felder: Messung pruefen");

    // Die Ankreuzliste: welche Fragen dazugehoeren, sagt die Bindung, nicht der Feldname.
    let screening: Vec<&&bindung::Bindung> = bindungen.iter().filter(|b| b.screening == Some(true)).collect();
    pp!("B.die_bindung_erklaert_welche_fragen_dazugehoeren", screening.len() >= 10, "nur {} screening-Felder", screening.len());
    for f in ["keine_behinderung_pflege", "kein_vuv", "kein_kap"] {
        pp!("B.die_bindung_erklaert_welche_fragen_dazugehoeren", screening.iter().any(|b| b.feld_id == f), "{f} fehlt in der Ankreuzliste");
    }
    for b in &screening {
        pp!("B.die_bindung_erklaert_welche_fragen_dazugehoeren", b.typ == domain::Feldtyp::Bool, "{} ist kein bool", b.feld_id);
        // Die Umkehr NUR dort, wo der Feldname die Abwesenheit benennt: ein Kreuz muss „ja, trifft zu“ heissen.
        let benennt_abwesenheit = b.feld_id.starts_with("kein_") || b.feld_id.starts_with("keine_");
        pp!("B.die_bindung_erklaert_welche_fragen_dazugehoeren", b.frage_invertiert == benennt_abwesenheit, "{}: frage_invertiert={}, aber der Feldname {} die Abwesenheit", b.feld_id, b.frage_invertiert, if benennt_abwesenheit { "benennt" } else { "benennt nicht" });
    }

    // Gegen den stillen Leerlauf der Feldtyp-Pruefungen: `text` und `datum` kommen wirklich vor.
    let zahl = |t: domain::Feldtyp| bindungen.iter().filter(|b| b.typ == t).count();
    pp!("C.die_bindung_enthaelt_diese_typen_wirklich", zahl(domain::Feldtyp::Text) >= 50, "nur noch {} text-Felder", zahl(domain::Feldtyp::Text));
    pp!("C.die_bindung_enthaelt_diese_typen_wirklich", zahl(domain::Feldtyp::Datum) >= 5, "nur noch {} datum-Felder", zahl(domain::Feldtyp::Datum));
}

/// Die Zaehlung der Instanz-Achse am Dienst: nur ein BESTAETIGTES Zaehlfeld spannt Eingabefelder auf, ein Feld ohne
/// Achse bleibt bei einem, ein Kind-Feld ohne Zaehlfeld auch.
#[test]
fn instanz_zaehlung_am_dienst() {
    let _sperre = seriell();
    let d = Dienst::starte(&Start::default());
    let frage = |fall: &str, feld: &str| -> Option<Value> {
        let (status, antwort) = d.get(&format!("/fall/{fall}/fragen"));
        assert_eq!(status, 200, "fragen: {antwort}");
        antwort["fragen"].as_array()?.iter().find(|q| q["feld_id"] == feld).cloned()
    };
    let zahl = |q: &Option<Value>| q.as_ref().map(|q| (q["instanz_anzahl"].clone(), q["instanz_etikett"].clone()));

    // Ohne Zaehlfeld: ein Eingabefeld, das Etikett der Gruppe steht trotzdem da.
    let leer = d.fall("gesamt", "iz0");
    let q = frage(&leer, "kind_vorname");
    pp!("D.ohne_zaehlfeld_ein_feld_mit_etikett", q.is_some(), "kind_vorname steht nicht in den Fragen des frischen Falls: der Test misst sonst nichts");
    pp!("D.ohne_zaehlfeld_ein_feld_mit_etikett", zahl(&q) == Some((json!(1), json!("Kind"))), "ohne Zaehlfeld: {:?}", zahl(&q));

    // Kontrolle: ein BESTAETIGTES Zaehlfeld spannt drei Felder auf.
    let drei = d.fall("gesamt", "iz3");
    let (st, r) = d.schreibe(&drei, "fam_anzahl_kinder", &json!(3));
    assert_eq!(st, 201, "{r}");
    let q = frage(&drei, "kind_vorname");
    pp!("D.ein_vorlaeufiger_wert_zaehlt_nicht", zahl(&q) == Some((json!(3), json!("Kind"))), "KONTROLLE: das bestaetigte Zaehlfeld spannt nicht drei Felder auf: {:?}", zahl(&q));

    // Ein KI-Vorschlag („3 Kinder“) ist noch nicht bestaetigt und entscheidet nicht, wie viele Felder der Nutzer fuellt.
    let vorschlag = d.fall("gesamt", "izv");
    let (st, r) = d.post(
        &format!("/fall/{vorschlag}/event"),
        &json!({"feld_id": "fam_anzahl_kinder", "wert": 3, "zustand": "vorlaeufig", "schreiber": "llm:naht",
                "herkunft": {"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                "signal": {"signal_1": null, "signal_2": null}}),
    );
    pp!("D.ein_vorlaeufiger_wert_zaehlt_nicht", st == 201, "der vorlaeufige Vorschlag wird nicht geschrieben ({st}): der Test misst sonst nichts: {r}");
    let q = frage(&vorschlag, "kind_vorname");
    pp!("D.ein_vorlaeufiger_wert_zaehlt_nicht", q.is_some(), "kind_vorname steht nicht in den Fragen");
    pp!("D.ein_vorlaeufiger_wert_zaehlt_nicht", zahl(&q).is_some_and(|(n, _)| n == json!(1)), "ein vorlaeufiger Vorschlag hat {:?} Eingabefelder aufgespannt", zahl(&q));

    // Die Gegenrichtung: ein Feld ohne Achse bleibt bei einem, auch wenn drei Kinder bestaetigt sind.
    for feld in ["bruttoarbeitslohn", "veranlagung", "ep_arbeitstage"] {
        let q = frage(&drei, feld);
        pp!("D.ein_feld_ohne_achse_bleibt_bei_einem", q.is_some(), "{feld} steht nicht in den Fragen: der Test misst sonst nichts");
        pp!("D.ein_feld_ohne_achse_bleibt_bei_einem", zahl(&q) == Some((json!(1), json!(""))), "{feld} hat ploetzlich {:?} Instanzen", zahl(&q));
    }
}
