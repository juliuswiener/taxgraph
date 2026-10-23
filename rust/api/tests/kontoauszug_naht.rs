//! `POST /fall/{id}/kontoauszug` ohne Python: Format, Beträge, Kategorien, Deckel der LLM-Aufrufe und die
//! Wortlaute, an einem Fall der Scheibe `gesamt`. Die Zahlen und Texte stammen aus einem Lauf von
//! `api.kontoauszug` (`CPython` 3.14); dass beide Server in 190 weiteren Formen gleich antworten
//! (CSV, JSON, PDF), prüft der Differenz-Harness (`rust/parity/tests/api_http_paritaet.rs`,
//! `generatoren`). Dieser Test hält die Gestalt fest und fällt auch dort, wo kein Python und kein
//! `pdftotext` läuft. Kein LLM: ohne Schlüssel liefert der Klassifikator `None`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

struct Dienst {
    zustand: Zustand,
    token: String,
    _tmp: tempfile::TempDir,
}

fn dienst() -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let konfig = Konfig {
        wurzel: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        faelle: tmp.path().join("faelle"),
        audit_dir: tmp.path().join("faelle"),
    };
    let auth = Auth::neu(
        "testgeheimnis".into(),
        tmp.path().join("users.json"),
        Some(konfig.audit_pfad()),
    );
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    Dienst {
        zustand,
        token,
        _tmp: tmp,
    }
}

async fn sende(d: &Dienst, pfad: &str, body: &Value) -> (u16, Value) {
    let text = body.to_string();
    let req = Request::builder()
        .method("POST")
        .uri(pfad)
        .header("authorization", format!("Bearer {}", d.token))
        .header("content-type", "application/json")
        .header("content-length", text.len().to_string())
        .body(Body::from(text))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (
        teile.status.as_u16(),
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Legt Fall `id` der Scheibe `scheibe` an.
async fn fall_anlegen(d: &Dienst, id: &str, scheibe: &str) {
    let rumpf = json!({"fall_id": id, "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "/fall", &rumpf).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
}

/// Ein Auszug an einen frischen Fall `id`.
async fn auszug(d: &Dienst, id: &str, body: Value) -> (u16, Value) {
    fall_anlegen(d, id, "gesamt").await;
    sende(d, &format!("/fall/{id}/kontoauszug"), &body).await
}

fn events(d: &Dienst, id: &str) -> Vec<Value> {
    let pfad = d.zustand.konfig.faelle.join(format!("{id}.json"));
    let akte: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    akte["events"].as_array().unwrap().clone()
}

fn csv(zeilen: &[&str]) -> Value {
    json!({"format": "csv", "inhalt": format!("datum;betrag;verwendungszweck\n{}\n", zeilen.join("\n"))})
}

#[tokio::test]
async fn csv_kategorien_werden_vorlaeufige_vorschlaege() {
    let d = dienst();
    let (status, antwort) = auszug(
        &d,
        "ka1",
        csv(&[
            "01.03.2025;-480,00;Spende Unicef Huber",
            "04.03.2025;-1.200,00;Rentenversicherung Ruerup",
            "05.03.2025;1.000,00;Gehalt",
            "07.03.2025;-9,99;Einkauf",
        ]),
    )
    .await;
    assert_eq!(status, 200, "{antwort}");
    // Seit Bankimport S2 schreiben nur Spende und Vorsorge; Handwerker, Reinigung und Minijob sind Nachfragen (eigene Tests unten).
    assert_eq!(
        antwort,
        json!({"uebernommen": 2, "transaktionen": 4, "verworfen": 0})
    );
    let ev = events(&d, "ka1");
    assert_eq!(ev.len(), 2);
    for e in &ev {
        assert_eq!(e["schreiber"], "import:kontoauszug");
        assert_eq!(e["zustand"], "vorlaeufig");
        assert_eq!(e["herkunft"]["herkunft"], "kontoauszug");
        assert!(e["signal"]["signal_2"].is_null());
    }
    let felder: Vec<_> = ev.iter().map(|e| e["feld_id"].as_str().unwrap()).collect();
    assert_eq!(felder, ["spenden_betrag", "vor_rv_ausserhalb_lstb"]);
    // Der Saldo steht ohne Vorzeichen im Feld; die Posten stehen mit Vorzeichen und ISO-Datum im
    // Beleg (`signal_1`, Bankimport S1). Die alten Schluessel `betrag`/`datum` gibt es nicht mehr.
    assert_eq!(ev[0]["wert"], 48_000);
    let s1 = &ev[0]["signal"]["signal_1"];
    assert_eq!(s1["typ"], "kontoauszug");
    assert_eq!(s1["jahr"], 2025);
    assert_eq!(s1["summe_cent"], 48_000);
    assert_eq!(s1["quelle"], "heuristik");
    assert_eq!(s1["kategorie"], "spende");
    assert_eq!(s1["regel_id"], "spende");
    assert_eq!(s1["posten"].as_array().unwrap().len(), 1);
    assert_eq!(s1["posten"][0]["betrag_cent"], -48_000);
    assert_eq!(s1["posten"][0]["datum"], "2025-03-01");
    assert_eq!(s1["posten"][0]["zweck_maskiert"], "Spende Unicef Huber");
    assert!(s1.get("betrag").is_none() && s1.get("datum").is_none(), "{s1}");
    assert_eq!(ev[1]["wert"], 120_000);
}

#[tokio::test]
async fn unlesbarer_betrag_faellt_einzeln_raus() {
    let d = dienst();
    let (status, antwort) = auszug(
        &d,
        "ka2",
        csv(&[
            "01.03.2025;-100000000,00;Rentenversicherung",
            "01.03.2025;-99999999,99;Rentenversicherung",
            "01.03.2025;100000000,00;Gehalt",
            "01.03.2025;-92233720368547758,07;Rentenversicherung",
            "01.03.2025;-92233720368547758,08;Rentenversicherung",
            "01.03.2025;-123456789012345678901,00;Rentenversicherung",
            "01.03.2025;-abc;Rentenversicherung",
            "01.03.2025;99999999,99;Gehalt",
        ]),
    )
    .await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(antwort["uebernommen"], 1);
    assert_eq!(antwort["transaktionen"], 2);
    assert_eq!(antwort["verworfen"], 6);
    assert_eq!(
        antwort["hinweis"],
        "6 Zeile(n) mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen."
    );
    // Die lesbare Buchung ist gebucht: 99.999.999,99 € = 9_999_999_999 Cent.
    assert_eq!(events(&d, "ka2")[0]["wert"], 9_999_999_999_i64);
}

#[tokio::test]
async fn json_betraege_wie_int_in_python() {
    let d = dienst();
    // (Betrag, bleibt im Auszug): `int()` liest Zahl, Text mit Leerraum und Unterstrich, `true`.
    let faelle = [
        (json!(-480.9), true),
        (json!(" -4_80 "), true),
        (json!(true), true),
        (json!(-9_999_999_999_i64), true),
        (json!(-10_000_000_000_i64), false),
        (json!(i64::MIN), false),
        (json!("1e3"), false),
        (json!("−480"), false),
        (json!(null), false),
        (json!([1]), false),
        (json!(1e10), false),
    ];
    for (i, (betrag, bleibt)) in faelle.into_iter().enumerate() {
        let (status, antwort) = auszug(
            &d,
            &format!("kb{i}"),
            json!({"format": "json", "inhalt": [{"datum": "d", "betrag": betrag, "verwendungszweck": "Maler"}]}),
        )
        .await;
        assert_eq!(status, 200, "{betrag}: {antwort}");
        assert_eq!(
            (
                antwort["transaktionen"].clone(),
                antwort["verworfen"].clone()
            ),
            (json!(i64::from(bleibt)), json!(i64::from(!bleibt))),
            "{betrag}: {antwort}"
        );
    }
}

#[tokio::test]
async fn json_text_wie_json_loads() {
    let d = dienst();
    let text = |t: &str| json!({"format": "json", "inhalt": t});
    // NaN und Infinity liest `json.loads`; der Betrag ist dann keine tragbare Zahl.
    for (i, betrag) in [
        "NaN",
        "Infinity",
        "-Infinity",
        "1e400",
        "-12345678901234567890",
    ]
    .into_iter()
    .enumerate()
    {
        let (status, antwort) = auszug(
            &d,
            &format!("kt{i}"),
            text(&format!(
                r#"[{{"betrag": {betrag}, "verwendungszweck": "Maler"}}]"#
            )),
        )
        .await;
        assert_eq!(status, 200, "{betrag}: {antwort}");
        assert_eq!(
            (
                antwort["transaktionen"].clone(),
                antwort["verworfen"].clone()
            ),
            (json!(0), json!(1)),
            "{betrag}"
        );
    }
    // 4300 Ziffern liest `json.loads` noch, 4301 nicht.
    let ziffern = |n: usize| {
        format!(
            r#"[{{"betrag": -{}, "verwendungszweck": "Maler"}}]"#,
            "9".repeat(n)
        )
    };
    let (status, antwort) = auszug(&d, "kt20", text(&ziffern(4300))).await;
    assert_eq!(
        (status, antwort["verworfen"].clone()),
        (200, json!(1)),
        "{antwort}"
    );
    let (status, antwort) = auszug(&d, "kt21", text(&ziffern(4301))).await;
    assert_eq!(
        (status, antwort),
        (400, json!({"fehler": "json-Inhalt nicht parsebar"}))
    );
    for (i, (inhalt, status, fehler)) in [
        (json!("[{"), 400, "json-Inhalt nicht parsebar"),
        (json!("  "), 400, "json-Inhalt nicht parsebar"),
        (
            json!("{}"),
            400,
            "json muss eine Liste von Transaktionen sein",
        ),
        (
            json!("null"),
            400,
            "json muss eine Liste von Transaktionen sein",
        ),
        (json!(5), 400, "json-Inhalt nicht parsebar"),
        (json!({"a": 1}), 400, "json-Inhalt nicht parsebar"),
    ]
    .into_iter()
    .enumerate()
    {
        let (s, a) = auszug(
            &d,
            &format!("kj{i}"),
            json!({"format": "json", "inhalt": inhalt}),
        )
        .await;
        assert_eq!((s, a), (status, json!({"fehler": fehler})), "{inhalt}");
    }
    // Falsche Inhalte sind die leere Liste.
    for (i, inhalt) in [
        json!(""),
        json!(null),
        json!(false),
        json!(0),
        json!({}),
        json!([]),
    ]
    .into_iter()
    .enumerate()
    {
        let (s, a) = auszug(
            &d,
            &format!("kl{i}"),
            json!({"format": "json", "inhalt": inhalt}),
        )
        .await;
        assert_eq!(
            (s, a),
            (
                200,
                json!({"uebernommen": 0, "transaktionen": 0, "verworfen": 0})
            ),
            "{inhalt}"
        );
    }
}

/// Bankimport, echte Bankdatei (AK2): Ohne KI-Konfiguration (`$LLM_*` fehlt) fragt der Dienst kein Modell. Dann gibt es keinen
/// Deckel der Aufrufe und keinen Satz darueber: eine Ausgabe ohne Stichwort ist „ohne erkanntes Stichwort“, nie „NICHT
/// automatisch eingeordnet“. ABWEICHUNG von Python (`api.kontoauszug` zaehlt den Deckel auch ohne Anbieter, Abweichung Nr. 107):
/// dort meldete die 51. Buchung ohne Stichwort „die Grenze von 50 Klassifikationen … war erreicht“, obwohl nichts klassifiziert
/// werden konnte. Mit KI gilt der Deckel wie zuvor (`kontoauszug_deckel_hermetisch.rs`).
#[tokio::test]
async fn echtdatei_ohne_ki_kein_deckelhinweis() {
    let d = dienst();
    // 56 Buchungen ohne Stichwort, eine Spende, ein unlesbarer Betrag: nur der unlesbare Betrag ist einen Satz wert.
    let mut tx: Vec<Value> = (0..56)
        .map(|i| json!({"datum": "01.03.2025", "betrag": -100 - i, "verwendungszweck": format!("Einkauf {i}")}))
        .collect();
    tx.push(json!({"datum": "01.03.2025", "betrag": -5000, "verwendungszweck": "Spende"}));
    tx.push(json!({"datum": "01.03.2025", "betrag": "x", "verwendungszweck": "Maler"}));
    let (status, antwort) = auszug(&d, "kd", json!({"format": "json", "inhalt": tx})).await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(
        antwort,
        json!({"uebernommen": 1, "transaktionen": 57, "verworfen": 1,
            "hinweis": "1 Zeile(n) mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen."})
    );
    // 51 Buchungen ohne Stichwort (JSON): kein Schluessel `llm_uebersprungen`, kein Hinweis.
    let eine: Vec<Value> = (0..51)
        .map(|i| json!({"datum": "01.03.2025", "betrag": -100 - i, "verwendungszweck": format!("Einkauf {i}")}))
        .collect();
    let (status, antwort) = auszug(&d, "kd3", json!({"format": "json", "inhalt": eine})).await;
    assert_eq!(
        (status, antwort),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 51, "verworfen": 0})
        )
    );
    // Der Fall der echten Datei: 62 Ausgaben im CSV, kein Stichwort. Der Hinweis nennt sie ALLE als „ohne erkanntes Stichwort“.
    let zeilen: Vec<String> = (0..62).map(|i| format!("01.03.2025;-{},00;Einkauf {i}", 3 + i)).collect();
    let zeilen: Vec<&str> = zeilen.iter().map(String::as_str).collect();
    let (status, antwort) = auszug(&d, "kd4", csv(&zeilen)).await;
    assert_eq!(
        (status, antwort),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 62, "verworfen": 0,
                "hinweis": "62 Buchung(en) gelesen, aber kein Vorschlag: 62 Buchung(en) ohne erkanntes Stichwort."})
        )
    );
}

/// Bankimport, echte Bankdatei (AK1): Die Depot-Tabelle hinter den Buchungen ist keine Buchung, auch ueber die Route. Gestalt der
/// echten Datei (erfunden): Vorspann, Kopfzeile mit sechs Feldern, drei Buchungen, Fusszeile, zweite Kopfzeile mit neun Feldern, drei
/// Depot-Zeilen, davon eine mit rein numerischer Kennnummer in der Betragsspalte.
#[tokio::test]
async fn echtdatei_depot_tabelle_ist_ueber_die_route_keine_buchung() {
    let d = dienst();
    let inhalt = "\"Umsätze Girokonto\";\"Zeitraum: 180 Tage\";\r\n\r\n\
        \"Buchungstag\";\"Wertstellung (Valuta)\";\"Vorgang\";\"Buchungstext\";\"Umsatz in EUR\";\r\n\
        \"15.03.2025\";\"15.03.2025\";\"Lastschrift\";\"Spende Tierheim Muster\";\"-480,00\";\r\n\
        \"16.03.2025\";\"16.03.2025\";\"Lastschrift\";\"Einkauf\";\"-12,50\";\r\n\
        \"17.03.2025\";\"17.03.2025\";\"Gutschrift\";\"Gehalt\";\"2.000,00\";\r\n\r\n\
        \"Summe Ausgaben\";\"-492,50 EUR\";\r\n\r\n\
        \"Kaufdatum\";\"Valuta\";\"Stück\";\"Bezeichnung\";\"WKN\";\"Währung\";\"Kurs\";\"Wert\";\r\n\
        \"01.02.2025\";\"01.02.2025\";\"1,000\";\"Muster Fonds\";\"A1B2C3\";\"EUR\";\"12,34\";\"12,34\";\r\n\
        \"02.02.2025\";\"02.02.2025\";\"2,000\";\"Muster Aktie\";\"654321\";\"EUR\";\"1,00\";\"2,00\";\r\n\
        \"03.02.2025\";\"03.02.2025\";\"3,000\";\"Muster Anleihe\";\"D4E5F6\";\"EUR\";\"3,00\";\"9,00\";\r\n";
    let (status, antwort) = auszug(&d, "kz2", json!({"format": "csv", "inhalt": inhalt})).await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(antwort["transaktionen"], 3, "nur die drei Buchungen der ersten Tabelle: {antwort}");
    assert_eq!(antwort["verworfen"], 0, "die Depot-Zeilen sind keine Zeilen mit unlesbarem Betrag: {antwort}");
    assert_eq!(antwort["uebernommen"], 1, "{antwort}");
    // Die vier Zeilen der zweiten Tabelle (Kopfzeile und drei Depot-Zeilen) werden gezaehlt und genannt, sonst nichts.
    assert_eq!(
        antwort["hinweis"],
        "4 Zeile(n) einer anderen Tabelle in der Datei ignoriert (mehr Spalten als die Kopfzeile der Buchungen)."
    );
    // EINE Zeile einer anderen Tabelle (nur ihre Kopfzeile) ist schon einen Satz wert.
    let nur_kopf = "\"Buchungstag\";\"Wertstellung (Valuta)\";\"Vorgang\";\"Buchungstext\";\"Umsatz in EUR\";\r\n\
        \"15.03.2025\";\"15.03.2025\";\"Lastschrift\";\"Einkauf\";\"-12,50\";\r\n\r\n\
        \"Kaufdatum\";\"Valuta\";\"Stück\";\"Bezeichnung\";\"WKN\";\"Währung\";\"Kurs\";\"Wert\";\r\n";
    let (status, antwort) = auszug(&d, "kz3", json!({"format": "csv", "inhalt": nur_kopf})).await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(antwort["transaktionen"], 1, "{antwort}");
    assert!(
        antwort["hinweis"].as_str().unwrap().contains("1 Zeile(n) einer anderen Tabelle in der Datei ignoriert"),
        "{antwort}"
    );
}

/// Auftrag k9-2 (Mutant K34): ein CSV, das Python mit `csv.Error` abbricht (ein `\r` mitten im ungequoteten Feld),
/// ist ein 500 mit der Klasse `Error` -- nicht `ValueError`. Text und Klasse stammen aus `api.kontoauszug`
/// (`CPython` 3.14: `csv.Error: new-line character seen in unquoted field - do you need to open the file with
/// newline=''?`); die Klasse heißt dort `Error`, weil `_dispatch` `type(e).__name__` nimmt.
#[tokio::test]
async fn ein_csv_mit_zeilenumbruch_im_feld_ist_ein_500_error() {
    let d = dienst();
    let (status, antwort) = auszug(&d, "ke", json!({"format": "csv", "inhalt": "a\rb\n"})).await;
    assert_eq!(
        (status, antwort),
        (
            500,
            json!({"fehler": "Error: new-line character seen in unquoted field - do you need to open the file with newline=''?"})
        )
    );
    // Die Akte bleibt leer: nichts wurde übernommen.
    assert_eq!(events(&d, "ke").len(), 0);
}

#[tokio::test]
async fn zielfeld_gilt_je_scheibe_und_wird_nicht_ueberschrieben() {
    let d = dienst();
    // `ep` führt `spenden_betrag` nicht: keine Buchung, aber die Zeile zählt.
    fall_anlegen(&d, "kz1", "ep").await;
    let (status, antwort) = sende(
        &d,
        "/fall/kz1/kontoauszug",
        &json!({"format": "json", "inhalt": [{"datum": "2025-03-01", "betrag": -5000, "verwendungszweck": "Spende"}]}),
    )
    .await;
    assert_eq!(
        (status, antwort),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 1, "verworfen": 0})
        )
    );
    // Zwei Buchungen derselben Kategorie ergeben EINEN Vorschlag mit der Summe (Bankimport S1; Python nahm die
    // erste). Derselbe Auszug noch einmal aendert nichts: die Buchungen stehen schon im Vorschlag (S1b, d-63).
    let rumpf = json!({"format": "json", "inhalt": [
        {"datum": "2025-03-01", "betrag": -100, "verwendungszweck": "Spende Tierheim"},
        {"datum": "2025-03-02", "betrag": -200, "verwendungszweck": "Spende Unicef"}]});
    let (_, antwort) = auszug(&d, "kz2", rumpf.clone()).await;
    assert_eq!(antwort["uebernommen"], 2);
    assert_eq!(events(&d, "kz2")[0]["wert"], 300);
    let (_, antwort) = sende(&d, "/fall/kz2/kontoauszug", &rumpf).await;
    assert_eq!(antwort["uebernommen"], 0);
    assert_eq!(events(&d, "kz2").len(), 1);
    assert_eq!(events(&d, "kz2")[0]["wert"], 300, "der alte Wert bleibt");
}

#[tokio::test]
async fn format_und_rumpf() {
    let d = dienst();
    let leer = "datum;betrag;verwendungszweck\n01.03.2025;-480,00;Maler\n";
    for (i, (format, status)) in [
        (json!("CSV"), 200),
        (json!(" csv\n"), 200),
        (json!("\u{a0}csv\u{a0}"), 200),
        (json!("\u{1c}csv\u{1f}"), 200),
        (json!(null), 400),
        (json!(""), 400),
        (json!("  "), 400),
        (json!([]), 400),
        (json!(0), 400),
        (json!("xml"), 400),
        (json!(5), 500),
        (json!(true), 500),
        (json!(["csv"]), 500),
        (json!({"a": 1}), 500),
    ]
    .into_iter()
    .enumerate()
    {
        let (s, a) = auszug(
            &d,
            &format!("kf{i}"),
            json!({"format": format, "inhalt": leer}),
        )
        .await;
        assert_eq!(s, status, "{format}: {a}");
        match status {
            400 => assert_eq!(a, json!({"fehler": "format muss csv, json oder pdf sein"})),
            500 => assert!(
                a["fehler"]
                    .as_str()
                    .unwrap()
                    .starts_with("AttributeError: '"),
                "{a}"
            ),
            _ => {}
        }
    }
    let (_, a) = auszug(&d, "kf20", json!({"format": 5, "inhalt": leer})).await;
    assert_eq!(
        a,
        json!({"fehler": "AttributeError: 'int' object has no attribute 'strip'"})
    );
    // Der Rumpf ist kein Objekt: `body.get` scheitert.
    for (i, (body, klasse)) in [
        (json!([1]), "list"),
        (json!("csv"), "str"),
        (json!(null), "NoneType"),
        (json!(5), "int"),
        (json!(true), "bool"),
        (json!(1.5), "float"),
    ]
    .into_iter()
    .enumerate()
    {
        let (s, a) = auszug(&d, &format!("kr{i}"), body.clone()).await;
        assert_eq!(
            (s, a),
            (
                500,
                json!({"fehler": format!("AttributeError: '{klasse}' object has no attribute 'get'")})
            ),
            "{body}"
        );
    }
}

#[tokio::test]
async fn pdf_eingabe_wird_vor_dem_lesen_geprueft() {
    let d = dienst();
    let fehlt = "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)";
    let ungueltig = "pdf-Inhalt nicht gültig base64-kodiert";
    for (i, (inhalt, fehler)) in [
        (json!(null), fehlt),
        (json!(""), fehlt),
        (json!(" \n\t"), fehlt),
        (json!(5), fehlt),
        (json!(["QQ=="]), fehlt),
        (json!("QQ"), ungueltig),
        (json!("QQ="), ungueltig),
        (json!("QUJD\nREVG"), ungueltig),
        (json!("QUJD REVG"), ungueltig),
        (json!("=QUJD"), ungueltig),
        (json!("QQ==QQ=="), ungueltig),
        (json!("QQ-_"), ungueltig),
        (json!("QUJ-"), ungueltig),
        (json!("QUJ_"), ungueltig),
        (json!("QUJDä"), ungueltig),
        (json!("QUJDR"), ungueltig),
    ]
    .into_iter()
    .enumerate()
    {
        let (s, a) = auszug(
            &d,
            &format!("kp{i}"),
            json!({"format": "pdf", "inhalt": inhalt}),
        )
        .await;
        assert_eq!((s, a), (400, json!({"fehler": fehler})), "{inhalt}");
    }
}

#[tokio::test]
async fn zweck_datum_und_csv_randfaelle() {
    let d = dienst();
    let tx = |betrag: Value, zweck: Value| json!({"format": "json", "inhalt": [{"datum": "2025-03-01", "betrag": betrag, "verwendungszweck": zweck}]});
    // Ein Zweck, der kein Text ist: bei einer Ausgabe 422 (vorher `AttributeError` aus `.lower()`, 500),
    // bei einer Einnahme nie gelesen; falsch heisst leer.
    for (i, (zweck, klasse)) in [
        (json!(5), "int"),
        (json!(["maler"]), "list"),
        (json!({"a": 1}), "dict"),
        (json!(true), "bool"),
        (json!(1.5), "float"),
    ]
    .into_iter()
    .enumerate()
    {
        let (s, a) = auszug(&d, &format!("kw{i}"), tx(json!(-100), zweck.clone())).await;
        assert_eq!(
            (s, a),
            (
                422,
                json!({"fehler": format!("Kontoauszug nicht lesbar: verwendungszweck einer Ausgabe muss Text sein, nicht {klasse}.")})
            ),
            "{zweck}"
        );
    }
    for (i, zweck) in [json!(null), json!(0), json!([]), json!(false), json!("")]
        .into_iter()
        .enumerate()
    {
        let (s, a) = auszug(&d, &format!("kw1{i}"), tx(json!(-100), zweck.clone())).await;
        assert_eq!(
            (s, a),
            (
                200,
                json!({"uebernommen": 0, "transaktionen": 1, "verworfen": 0})
            ),
            "{zweck}"
        );
    }
    let (s, a) = auszug(&d, "kw20", tx(json!(100), json!(5))).await;
    assert_eq!(
        (s, a),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 1, "verworfen": 0})
        )
    );
    // Ein fehlendes oder falsches Datum wird nicht geraten (Bankimport S1; vorher stand es unverändert im Beleg):
    // die Buchung zählt nicht, und die Antwort sagt es, auch ohne CSV.
    let hinweis_datum = "Nicht gezählt: 1 Buchung(en) mit einem Datum, das der Leser nicht erkennt (erwartet: TT.MM.JJJJ, TT.MM.JJ, JJJJ-MM-TT).";
    let (_, a) = auszug(
        &d,
        "kw30",
        json!({"format": "json", "inhalt": [{"betrag": -5000, "verwendungszweck": "Spende"}]}),
    )
    .await;
    assert_eq!(a["uebernommen"], 0);
    assert_eq!(a["hinweis"], hinweis_datum);
    assert_eq!(events(&d, "kw30").len(), 0);
    let (_, a) = auszug(&d, "kw31", json!({"format": "json", "inhalt": [{"datum": 5.5, "betrag": -5000, "verwendungszweck": "Spende"}]})).await;
    assert_eq!(a["uebernommen"], 0);
    assert_eq!(a["hinweis"], hinweis_datum);
    assert_eq!(events(&d, "kw31").len(), 0);
    let (_, a) = auszug(&d, "kw32", json!({"format": "json", "inhalt": [{"datum": "2025-03-15", "betrag": -5000, "verwendungszweck": "Spende"}]})).await;
    assert_eq!(a["uebernommen"], 1);
    assert!(a.get("hinweis").is_none(), "{a}");
    assert_eq!(events(&d, "kw32")[0]["signal"]["signal_1"]["posten"][0]["datum"], "2025-03-15");
    // CSV: der Inhalt, der kein Text ist, ist leer; Vorzeichen wie `lstrip("+-")`.
    // Seit Bankimport S0 sagt die Antwort, dass nichts gelesen wurde (vorher: kein Hinweis, „0 von 0“).
    let (s, a) = auszug(&d, "kc0", json!({"format": "csv", "inhalt": 5})).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        (&a["uebernommen"], &a["transaktionen"], &a["verworfen"]),
        (&json!(0), &json!(0), &json!(0)),
        "{a}"
    );
    assert!(
        a["hinweis"].as_str().is_some_and(
            |h| h.starts_with("Keine Buchung gelesen") && h.contains("keine Kopfzeile erkannt")
        ),
        "{a}"
    );
    for (i, (betrag, gebucht)) in [
        ("--5", 1),
        ("+-5", 0),
        ("-+5", 1),
        (" - 5,5 € ", 1),
        ("-480.5", 1),
    ]
    .into_iter()
    .enumerate()
    {
        let (s, a) = auszug(
            &d,
            &format!("kc1{i}"),
            csv(&[&format!("01.03.2025;{betrag};Spende Tierheim")]),
        )
        .await;
        assert_eq!(s, 200, "{betrag}: {a}");
        assert_eq!(
            (&a["uebernommen"], &a["transaktionen"], &a["verworfen"]),
            (&json!(gebucht), &json!(1), &json!(0)),
            "{betrag}: {a}"
        );
        // Ohne Vorschlag sagt die Antwort warum (S0); mit Vorschlag bleibt sie ohne Hinweis.
        assert_eq!(a["hinweis"].is_null(), gebucht == 1, "{betrag}: {a}");
    }
}

/// Backlog `falldatei-mit-nan-liest-rust-als-text`, AK4 (Python: `tests/test_kontoauszug_buchungsfelder.py`).
/// `inhalt` ist ein JSON-TEXT, den die Rumpf-Tür nicht sieht. Ein `datum` mit einer Zahl, die die Akte
/// nicht halten kann (nicht endlich oder ausserhalb von `i64`, beliebig tief), und der Zweck einer Ausgabe,
/// der kein Text ist, sind 422; der Text nennt Feld und Typ, nie den Wert, und die Akte bleibt leer.
#[tokio::test]
async fn datum_und_zweck_die_die_akte_nicht_haelt_sind_422() {
    let d = dienst();
    let auszug_text = |datum: &str, zweck: &str, betrag: &str| {
        let tx =
            format!(r#"[{{"datum": {datum}, "betrag": {betrag}, "verwendungszweck": {zweck}}}]"#);
        json!({"format": "json", "inhalt": tx})
    };
    let ruerup = r#""Ruerup-Rente Jahresbeitrag Basisrente""#;
    let datum_meldung = |typ: &str| json!({"fehler": format!("Kontoauszug nicht lesbar: datum einer Buchung enthält eine Zahl ({typ}), die die Akte nicht halten kann.")});
    let zweck_meldung = |typ: &str| json!({"fehler": format!("Kontoauszug nicht lesbar: verwendungszweck einer Ausgabe muss Text sein, nicht {typ}.")});
    let (zwei_63, zwei_64, unter_i64) = (
        "9223372036854775808",
        "18446744073709551616",
        "-9223372036854775809",
    );
    let datum_faelle: Vec<(String, &str)> = vec![
        ("NaN".into(), "float"),
        ("Infinity".into(), "float"),
        ("-Infinity".into(), "float"),
        ("1e400".into(), "float"),
        ("-1e400".into(), "float"),
        ("[1, NaN]".into(), "float"),
        (r#"{"a": {"b": [Infinity]}}"#.into(), "float"),
        (zwei_63.into(), "int"),
        (zwei_64.into(), "int"),
        (unter_i64.into(), "int"),
        (format!("[{zwei_64}]"), "int"),
        (format!(r#"{{"a": {unter_i64}}}"#), "int"),
        (format!("[{zwei_64}, NaN]"), "float"),
    ];
    for (i, (datum, typ)) in datum_faelle.iter().enumerate() {
        let id = format!("bd{i}");
        let (s, a) = auszug(&d, &id, auszug_text(datum, ruerup, "-120000")).await;
        assert_eq!((s, a), (422, datum_meldung(typ)), "{datum}");
        assert!(events(&d, &id).is_empty(), "{datum}: die Akte bleibt leer");
    }
    // Kontrolle: die Tür nimmt tragbare Daten an, auch die Grenzen von i64 und Text "NaN" (200). Der Leser liest
    // keines davon als Datum (Bankimport S1): die Buchung zählt nicht und die Antwort sagt es.
    for (i, datum) in [
        "9223372036854775807",
        "-9223372036854775808",
        "1.5",
        "1e22",
        "-0",
        "null",
        r#""NaN""#,
        "[1, 2.5]",
        "true",
    ]
    .iter()
    .enumerate()
    {
        let (s, a) = auszug(&d, &format!("bg{i}"), auszug_text(datum, ruerup, "-120000")).await;
        assert_eq!(
            (s, a),
            (
                200,
                json!({"uebernommen": 0, "transaktionen": 1, "verworfen": 0,
                    "hinweis": "Nicht gezählt: 1 Buchung(en) mit einem Datum, das der Leser nicht erkennt (erwartet: TT.MM.JJJJ, TT.MM.JJ, JJJJ-MM-TT)."})
            ),
            "{datum}"
        );
    }
    // Gegenprobe: ein lesbares Datum mit demselben Zweck und Betrag wird übernommen.
    let (s, a) = auszug(&d, "bg_ok", auszug_text(r#""2025-03-15""#, ruerup, "-120000")).await;
    assert_eq!((s, a), (200, json!({"uebernommen": 1, "transaktionen": 1, "verworfen": 0})));
    // Kontrolle: nur gültiges JSON kann eine Zahl ausserhalb von `i64` sein; führende Null, `-` und `+5` sind es nicht.
    for (i, zahl) in [
        "0123456789012345678901234",
        "-0123456789012345678901",
        "-",
        "--5",
        "+5",
    ]
    .iter()
    .enumerate()
    {
        let (s, a) = auszug(&d, &format!("bk{i}"), auszug_text(zahl, ruerup, "-120000")).await;
        assert_eq!(
            (s, a),
            (400, json!({"fehler": "json-Inhalt nicht parsebar"})),
            "{zahl}"
        );
    }
    // Das Datum jeder Buchung zählt, auch einer Einnahme.
    let (s, a) = auszug(&d, "be0", auszug_text("NaN", ruerup, "5000")).await;
    assert_eq!((s, a), (422, datum_meldung("float")));
    // `verworfen` kommt zuerst: ein unlesbarer Betrag nimmt die Buchung allein heraus, auch mit NaN im Datum.
    let inhalt = format!(
        r#"[{{"datum": NaN, "betrag": NaN, "verwendungszweck": {ruerup}}}, {{"datum": "2025-03-15", "betrag": -120000, "verwendungszweck": {ruerup}}}]"#
    );
    let (s, a) = auszug(&d, "bv0", json!({"format": "json", "inhalt": inhalt})).await;
    assert_eq!(
        (s, a),
        (
            200,
            json!({"uebernommen": 1, "transaktionen": 1, "verworfen": 1, "hinweis": "1 Zeile(n) mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen."})
        )
    );
    // Der Zweck einer Ausgabe, der kein Text ist; als JSON-Text auch NaN und eine grosse Ganzzahl.
    for (i, (zweck, typ)) in [
        ("5", "int"),
        ("1.5", "float"),
        (r#"["maler"]"#, "list"),
        (r#"{"a": 1}"#, "dict"),
        ("true", "bool"),
        ("NaN", "float"),
        ("-Infinity", "float"),
        (zwei_64, "int"),
    ]
    .iter()
    .enumerate()
    {
        let id = format!("bz{i}");
        let (s, a) = auszug(&d, &id, auszug_text(r#""2025-03-15""#, zweck, "-120000")).await;
        assert_eq!((s, a), (422, zweck_meldung(typ)), "{zweck}");
        assert!(events(&d, &id).is_empty(), "{zweck}: die Akte bleibt leer");
    }
    // Falsch heisst leer, und der Zweck einer Einnahme wird nie gelesen.
    for (i, (zweck, betrag)) in [
        ("null", "-120000"),
        ("0", "-120000"),
        ("[]", "-120000"),
        ("false", "-120000"),
        (r#""""#, "-120000"),
        ("5", "5000"),
    ]
    .iter()
    .enumerate()
    {
        let (s, a) = auszug(&d, &format!("bl{i}"), auszug_text(r#""2025-03-15""#, zweck, betrag)).await;
        assert_eq!(
            (s, a),
            (
                200,
                json!({"uebernommen": 0, "transaktionen": 1, "verworfen": 0})
            ),
            "{zweck}"
        );
    }
    // Das Datum wird vor dem Zweck gemeldet, beides vor dem Schreiben.
    let inhalt = r#"[{"datum": "ok", "betrag": -100, "verwendungszweck": 5}, {"datum": NaN, "betrag": -100, "verwendungszweck": "Maler"}]"#;
    let (s, a) = auszug(&d, "br0", json!({"format": "json", "inhalt": inhalt})).await;
    assert_eq!((s, a), (422, datum_meldung("float")));
    assert_eq!(events(&d, "br0").len(), 0);
}

/// Bankimport S0, erfunden, Aufbau nach Fremdparsern (kein Bankdokument): eine DKB-Datei mit vier Zeilen
/// Vorspann und `Betrag (€)` lieferte „0 von 0“ ohne Hinweis. Jetzt zwei Vorschläge, ohne Hinweis.
#[tokio::test]
async fn eine_csv_datei_mit_vorspann_ergibt_vorschlaege() {
    let d = dienst();
    let dkb = "\"Girokonto\";\"DE89370400440532013000\"\n\n\"Kontostand vom 31.12.2025:\";\"5.000,01 EUR\"\n\n\
\"Buchungsdatum\";\"Wertstellung\";\"Status\";\"Zahlungsempfänger*in\";\"Verwendungszweck\";\"Betrag (€)\"\n\
\"15.03.25\";\"15.03.25\";\"Gebucht\";\"Rente Müller GmbH\";\"Beitrag 123 Basisrente\";\"-480,00\"\n\
\"20.03.25\";\"20.03.25\";\"Gebucht\";\"Rotes Kreuz\";\"Spende Rotes Kreuz\";\"-50,00\"\n";
    let (status, antwort) = auszug(&d, "kv1", json!({"format": "csv", "inhalt": dkb})).await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(
        antwort,
        json!({"uebernommen": 2, "transaktionen": 2, "verworfen": 0})
    );
    let ev = events(&d, "kv1");
    let felder: Vec<_> = ev.iter().map(|e| e["feld_id"].as_str().unwrap()).collect();
    assert_eq!(felder, ["vor_rv_ausserhalb_lstb", "spenden_betrag"]);
    assert_eq!(ev[0]["wert"], 48_000);
    assert_eq!(ev[0]["signal"]["signal_1"]["posten"][0]["datum"], "2025-03-15", "15.03.25 wird 2025");
}

/// Bankimport S0: Liefert eine CSV-Datei keine Buchung, nennt die Antwort den Grund, nie eine Datenzeile.
#[tokio::test]
async fn csv_ohne_buchung_nennt_den_grund() {
    async fn lauf(d: &Dienst, id: &str, inhalt: &str) -> String {
        let (status, antwort) = auszug(d, id, json!({"format": "csv", "inhalt": inhalt})).await;
        assert_eq!(status, 200, "{id}: {antwort}");
        assert_eq!(
            (
                &antwort["uebernommen"],
                &antwort["transaktionen"],
                &antwort["verworfen"]
            ),
            (&json!(0), &json!(0), &json!(0)),
            "{id}: {antwort}"
        );
        antwort["hinweis"].as_str().unwrap_or_default().to_owned()
    }
    let d = dienst();
    // Keine Betragsspalte: die erwarteten Namen und die gefundenen Spalten, aber nicht der Inhalt der Zeile.
    let h = lauf(
        &d,
        "kg1",
        "Buchungstag;Wert;Verwendungszweck\n01.03.2025;5;Geheimer Zweck\n",
    )
    .await;
    assert!(h.starts_with("Keine Buchung gelesen: In den ersten 30 Zeilen steht keine Kopfzeile mit Betragsspalte"), "{h}");
    assert!(
        h.contains("erwartet: betrag, umsatz, amount, betrag (eur), betrag (€), umsatz in eur"),
        "{h}"
    );
    assert!(
        h.ends_with("gefundene Spalten: Buchungstag, Wert, Verwendungszweck."),
        "{h}"
    );
    assert!(
        !h.contains("Geheimer Zweck") && !h.contains("01.03.2025"),
        "{h}"
    );
    // Nichts, das nach einer Kopfzeile aussieht: keine Namen.
    let h = lauf(&d, "kg2", "x;y\n1;2\n").await;
    assert!(h.ends_with("keine Kopfzeile erkannt."), "{h}");
    // Kopfzeile, aber keine Datenzeile.
    let h = lauf(&d, "kg3", "datum;betrag;verwendungszweck\n").await;
    assert_eq!(
        h,
        "Keine Buchung gelesen: Unter der Kopfzeile folgt keine Datenzeile."
    );
    // Kopfzeile, Datenzeilen ohne Betrag.
    let h = lauf(
        &d,
        "kg4",
        "datum;betrag;verwendungszweck\n01.03.2025;;Maler\n02.03.2025; ;Maler\n",
    )
    .await;
    assert_eq!(
        h,
        "Keine Buchung gelesen: 2 Zeile(n) unter der Kopfzeile haben keinen Betrag."
    );
}

/// Bankimport S0: Gibt es eine Kopfzeile, aber jeder Betrag ist unlesbar, zaehlt die Zeile in `verworfen`, und der
/// Hinweis nennt nur das. „Keine Buchung gelesen“ stuende daneben als Widerspruch („Betrag unlesbar“ und zugleich
/// „keine Kopfzeile“ oder „keine Datenzeile“).
#[tokio::test]
async fn csv_mit_nur_unlesbaren_betraegen_nennt_nur_das_verwerfen() {
    let d = dienst();
    let (status, antwort) = auszug(
        &d,
        "kv2",
        json!({"format": "csv", "inhalt": "datum;betrag;verwendungszweck\n01.03.2025;abc;Maler\n"}),
    )
    .await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(
        antwort,
        json!({"uebernommen": 0, "transaktionen": 0, "verworfen": 1,
            "hinweis": "1 Zeile(n) mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen."})
    );
}

/// Bankimport S0: Wurde gelesen, aber nichts vorgeschlagen, nennt die Antwort die Zahl je Grund.
/// Mit mindestens einem Vorschlag bleibt sie ohne Hinweis (auch wenn eine zweite Buchung dasselbe Feld traf).
#[tokio::test]
async fn csv_ohne_vorschlag_nennt_die_zahl_je_grund() {
    let d = dienst();
    // Einnahme, Ausgabe ohne Stichwort, Spende an eine Partei.
    let (s, a) = auszug(
        &d,
        "kh1",
        csv(&[
            "01.03.2025;1.000,00;Gehalt",
            "02.03.2025;-9,99;Einkauf",
            "03.03.2025;-50,00;Spende SPD Ortsverband",
        ]),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["uebernommen"], 0, "{a}");
    assert_eq!(
        a["hinweis"],
        "3 Buchung(en) gelesen, aber kein Vorschlag: 1 Buchung(en) ohne Minuszeichen (nur Beträge mit Minus gelten als Ausgabe); 1 Buchung(en) ohne erkanntes Stichwort; 1 Buchung(en) als Spende mit Parteinamen (dafür gibt es ein eigenes Feld)."
    );
    // Die Scheibe `ep` führt `spenden_betrag` nicht.
    fall_anlegen(&d, "kh2", "ep").await;
    let (s, a) = sende(
        &d,
        "/fall/kh2/kontoauszug",
        &csv(&["01.03.2025;-50,00;Spende Rotes Kreuz"]),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        a["hinweis"],
        "1 Buchung(en) gelesen, aber kein Vorschlag: 1 Buchung(en) für ein Feld, das dieser Fall nicht führt."
    );
    // Zwei Spenden in einem Auszug: EIN Vorschlag mit der Summe, beide Buchungen zählen (Bankimport S1) -- kein Hinweis.
    let zwei = csv(&[
        "01.03.2025;-100,00;Spende Tierheim Huber",
        "02.03.2025;-200,00;Spende Unicef Meier",
    ]);
    let (_, a) = auszug(&d, "kh3", zwei.clone()).await;
    assert_eq!(
        a,
        json!({"uebernommen": 2, "transaktionen": 2, "verworfen": 0})
    );
    assert_eq!(events(&d, "kh3")[0]["wert"], 30_000);
    // Derselbe Auszug noch einmal: der eigene Vorschlag des Felds enthaelt die Buchungen schon (S1b).
    let (_, a) = sende(&d, "/fall/kh3/kontoauszug", &zwei).await;
    assert_eq!(
        a["hinweis"],
        "2 Buchung(en) gelesen, aber kein Vorschlag: 2 Buchung(en) im bisherigen Vorschlag des Felds schon enthalten."
    );
    assert_eq!(events(&d, "kh3").len(), 1);
    // Ein BESTAETIGTER Wert auf dem Feld bleibt „schon einen Wert“, auch mit Haken.
    fall_anlegen(&d, "kh4", "gesamt").await;
    let (s, _) = sende(
        &d,
        "/fall/kh4/event",
        &json!({"feld_id": "spenden_betrag", "wert": 1_000, "zustand": "bestaetigt",
            "herkunft": {"haftung": "nutzer", "herkunft": "laie", "pruef_tiefe": "ungeprueft"},
            "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": "ok@spenden_betrag"}}),
    )
    .await;
    assert_eq!(s, 201);
    let mut mit_haken = zwei.clone();
    mit_haken["ersetzen"] = json!(true);
    let (_, a) = sende(&d, "/fall/kh4/kontoauszug", &mit_haken).await;
    // Seit S2 nennt die Antwort daneben den Abgleich (Beleg schlaegt Konto): was das Konto ergaebe, schreibt es nicht.
    assert_eq!(
        a["hinweis"],
        "2 Buchung(en) gelesen, aber kein Vorschlag: 2 Buchung(en) für ein Feld, das schon einen Wert hat. \
         Abgleich: Für spenden_betrag gilt schon ein Wert aus einem Beleg, einer Eingabe oder einem früheren Import. \
         Das Konto zeigt 300,00 € aus 2 Buchung(en); der Kontoauszug schreibt dort nichts."
    );
    assert_eq!(a["abgleich"], json!([{"feld_id": "spenden_betrag", "summe_cent": 30_000, "anzahl": 2}]));
    assert_eq!(events(&d, "kh4").len(), 1);
}

/// Bankimport S1b (d-63): Ein zweiter Auszug auf ein Feld mit eigenem vorläufigem Vorschlag zeigt ohne
/// `ersetzen` die Vorschau und schreibt nichts; mit `ersetzen: true` ersetzt er durch die Vereinigung.
#[tokio::test]
async fn zweiter_auszug_zeigt_ohne_haken_die_vorschau_und_ersetzt_mit_haken() {
    let d = dienst();
    let (s, a) = auszug(
        &d,
        "ke1",
        csv(&[
            "01.02.2025;-4,00;Spende Rotes Kreuz",
            "02.02.2025;-7,00;Spende Tierheim",
        ]),
    )
    .await;
    assert_eq!((s, &a["uebernommen"]), (200, &json!(2)), "{a}");
    let erstes = events(&d, "ke1");
    assert_eq!((erstes.len(), &erstes[0]["wert"]), (1, &json!(11_00)));
    let zweiter = csv(&[
        "02.02.2025;-7,00;Spende Tierheim",
        "03.02.2025;-5,00;Spende Unicef",
    ]);
    let vorschau = json!([{"feld_id": "spenden_betrag", "bisher_cent": 11_00, "bisher_posten": 2,
        "neu_cent": 16_00, "neu_posten": 3}]);
    let satz = "Ersetzen möglich: spenden_betrag: bisher 11,00 € aus 2 Buchung(en), mit diesem Auszug 16,00 € aus 3 Buchung(en). Setze den Haken, um den bisherigen Vorschlag zu ersetzen.";
    // Ohne `ersetzen` und mit `ersetzen: false`: dieselbe Vorschau, die Akte bleibt, wie sie war.
    for rumpf in [zweiter.clone(), {
        let mut b = zweiter.clone();
        b["ersetzen"] = json!(false);
        b
    }] {
        let (s, a) = sende(&d, "/fall/ke1/kontoauszug", &rumpf).await;
        assert_eq!(s, 200, "{a}");
        assert_eq!(
            a,
            json!({"uebernommen": 0, "transaktionen": 2, "verworfen": 0, "ersetzbar": vorschau,
                "hinweis": format!("2 Buchung(en) gelesen, aber kein Vorschlag: 2 Buchung(en) für ein Feld, dessen bisheriger Vorschlag erst mit dem Haken ersetzt wird. {satz}")})
        );
        assert_eq!(events(&d, "ke1"), erstes, "die Vorschau schreibt nichts");
    }
    // Mit Haken: ein neues Ereignis ersetzt das alte, die Akte auf der Platte traegt es.
    let mut mit_haken = zweiter.clone();
    mit_haken["ersetzen"] = json!(true);
    let (s, a) = sende(&d, "/fall/ke1/kontoauszug", &mit_haken).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        a,
        json!({"uebernommen": 2, "transaktionen": 2, "verworfen": 0, "ersetzt": vorschau,
            "hinweis": "Ersetzt: spenden_betrag: bisher 11,00 € aus 2 Buchung(en), jetzt 16,00 € aus 3 Buchung(en)."})
    );
    let jetzt = events(&d, "ke1");
    assert_eq!(jetzt.len(), 2);
    assert_eq!(
        (&jetzt[1]["wert"], &jetzt[1]["ersetzt"]),
        (&json!(16_00), &erstes[0]["event_id"])
    );
    assert_eq!(
        jetzt[1]["signal"]["signal_1"]["posten"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    // Noch einmal mit Haken: nichts Neues, also auch keine zweite Ersetzung.
    let (s, a) = sende(&d, "/fall/ke1/kontoauszug", &mit_haken).await;
    assert_eq!(
        (s, a.get("ersetzt"), a.get("ersetzbar")),
        (200, None, None),
        "{a}"
    );
    assert_eq!(events(&d, "ke1").len(), 2);
}

/// `ersetzen` ist ein Wahrheitswert oder fehlt; alles andere ist 400, bevor der Auszug gelesen wird.
#[tokio::test]
async fn ersetzen_muss_ein_wahrheitswert_sein() {
    let d = dienst();
    fall_anlegen(&d, "ke2", "gesamt").await;
    for wert in [
        json!("ja"),
        json!(1),
        json!(null),
        json!([]),
        json!({"a": true}),
    ] {
        let mut b = csv(&["01.03.2025;-480,00;Maler Huber"]);
        b["ersetzen"] = wert.clone();
        let (s, a) = sende(&d, "/fall/ke2/kontoauszug", &b).await;
        assert_eq!(
            (s, a),
            (400, json!({"fehler": "ersetzen muss true oder false sein"})),
            "{wert}"
        );
    }
    assert_eq!(
        events(&d, "ke2").len(),
        0,
        "ein ungueltiger Haken schreibt nichts"
    );
}

/// Bankimport S1: Buchungen, die wegen Jahr, Datum oder Saldo nicht zählten, nennt die Antwort mit Zahlen,
/// auch neben einem Vorschlag, in jedem Format. Nie ein Zweck, nie eine Gegenpartei.
#[tokio::test]
async fn die_antwort_nennt_jahr_datum_und_saldo_auch_neben_einem_vorschlag() {
    let d = dienst();
    // Ein Vorschlag (Vorsorge), dazu eine Buchung aus 2024, eine ohne Jahr, und eine Spende, die zurückgebucht wurde.
    let (s, a) = auszug(
        &d,
        "kj1",
        csv(&[
            "01.03.2025;-100,00;Rentenversicherung Huber",
            "30.12.2024;-50,00;Spende Rotes Kreuz",
            "15.03;-20,00;Rentenversicherung ohne Jahr",
            "01.02.2025;-80,00;Spende Tierheim",
            "10.02.2025;80,00;Rueckbuchung Spende Tierheim",
        ]),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(
        a,
        json!({"uebernommen": 1, "transaktionen": 5, "verworfen": 0,
            "hinweis": "Nicht gezählt: 1 Buchung(en) außerhalb des Veranlagungsjahrs; 1 Buchung(en) mit einem Datum, das der Leser nicht erkennt (erwartet: TT.MM.JJJJ, TT.MM.JJ, JJJJ-MM-TT); 2 Buchung(en) in einem Feld, dessen Saldo nach Rückbuchungen null oder negativ ist."})
    );
    let ev = events(&d, "kj1");
    assert_eq!(ev.len(), 1, "nur die Vorsorge; die ausgeglichene Spende schreibt nichts");
    assert_eq!(ev[0]["feld_id"], "vor_rv_ausserhalb_lstb");
    // Weder Zweck noch Gegenpartei der nicht gezählten Buchungen steht in der Antwort.
    let text = a.to_string();
    assert!(!text.contains("Tierheim") && !text.contains("Rotes Kreuz") && !text.contains("ohne Jahr"), "{text}");
    // Ohne einen Vorschlag nennt die CSV-Antwort dieselben Gründe im Satz „gelesen, aber kein Vorschlag“.
    let (_, a) = auszug(&d, "kj2", csv(&["30.12.2024;-50,00;Spende Rotes Kreuz"])).await;
    assert_eq!(
        a["hinweis"],
        "1 Buchung(en) gelesen, aber kein Vorschlag: 1 Buchung(en) außerhalb des Veranlagungsjahrs."
    );
    // JSON ohne Vorschlag: derselbe Zusatz wie neben einem Vorschlag.
    let (_, a) = auszug(
        &d,
        "kj3",
        json!({"format": "json", "inhalt": [{"datum": "2024-12-30", "betrag": -5000, "verwendungszweck": "Spende"}]}),
    )
    .await;
    assert_eq!(
        a,
        json!({"uebernommen": 0, "transaktionen": 1, "verworfen": 0,
            "hinweis": "Nicht gezählt: 1 Buchung(en) außerhalb des Veranlagungsjahrs."})
    );
}

// ---------------------------------------------------------------------------
// Bankimport S2 (d-61, Vault `backlog/taxgraph/bankimport-s2-regelkatalog-mit-stufen-35a-ohne-betrag-und-art9-waechter`):
// § 35a schreibt keinen Betrag ins Feld, die Antwort nennt die Zahlungen als Nachfrage; ein Beleg schlaegt den Kontoauszug.
// ---------------------------------------------------------------------------

/// Die drei § 35a-Kategorien schreiben kein Ereignis; die Antwort nennt je Buchung Feld, Art, Datum, Betrag (Ausgabe negativ)
/// und den maskierten Zweck, in der Reihenfolge der Datei, und einen Satz im Hinweis. Die Spende im selben Auszug wird weiter
/// vorgeschlagen (Kontrolle gegen einen Test, der nie etwas schreibt).
#[tokio::test]
async fn bank_regeln_35a_route_schreibt_kein_ereignis_und_nennt_die_nachfragen() {
    let d = dienst();
    let (s, a) = auszug(
        &d,
        "b351",
        csv(&[
            "01.03.2025;-73,00;Maler Huber",
            "02.03.2025;-41,00;Reinigung Treppenhaus",
            "03.03.2025;-29,00;Minijob-Zentrale",
            "04.03.2025;-11,00;Spende Tierheim",
        ]),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    let felder: Vec<String> = events(&d, "b351").iter().map(|e| e["feld_id"].as_str().unwrap().to_owned()).collect();
    assert_eq!(felder, ["spenden_betrag"], "nur die Spende schreibt");
    assert_eq!(a["uebernommen"], 1, "{a}");
    assert_eq!(a["transaktionen"], 4, "{a}");
    let nachfragen = a["nachfragen"].as_array().unwrap_or_else(|| panic!("Antwortfeld nachfragen fehlt: {a}"));
    let erwartet = [
        ("hh_handwerker_betrag", "2025-03-01", -73_00, "Maler Huber"),
        ("hh_dienstleistung_betrag", "2025-03-02", -41_00, "Reinigung Treppenhaus"),
        ("hh_minijob_betrag", "2025-03-03", -29_00, "Minijob-Zentrale"),
    ];
    assert_eq!(nachfragen.len(), erwartet.len(), "{nachfragen:?}");
    for (zeile, (feld, datum, betrag, zweck)) in nachfragen.iter().zip(erwartet) {
        assert_eq!(
            zeile,
            &json!({"feld_id": feld, "art": "arbeitskosten_nachfragen", "datum": datum, "betrag_cent": betrag,
                "zweck_maskiert": zweck})
        );
    }
    let hinweis = a["hinweis"].as_str().unwrap_or_else(|| panic!("kein Hinweis: {a}"));
    assert!(hinweis.contains("Arbeitskosten laut Rechnung"), "{hinweis}");
}

/// Die Nachfrage ist ein Teil der Antwort an die Nutzerin, kein Teil der Akte: weder das Ereignis noch das Audit noch das
/// Fehlerlog tragen den Zweck einer § 35a-Buchung. Kontrollen: die Spende steht mit ihrem Zweck in der Akte, und das Audit
/// hat den Eintrag des Aufrufs.
#[tokio::test]
async fn bank_regeln_35a_nachfragen_stehen_nicht_in_ereignis_log_audit() {
    let d = dienst();
    let (s, a) = auszug(
        &d,
        "b352",
        csv(&["05.03.2025;-83,00;Maler Zebrastreifen 4711", "06.03.2025;-17,00;Spende Wellensittich"]),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    let akte = std::fs::read_to_string(d.zustand.konfig.faelle.join("b352.json")).unwrap();
    assert!(akte.contains("Wellensittich"), "Kontrolle: der Zweck der Spende steht in der Akte");
    let audit = std::fs::read_to_string(d.zustand.konfig.audit_pfad()).unwrap_or_default();
    assert!(audit.contains("fall_kontoauszug"), "Kontrolle: das Audit nennt den Aufruf: {audit}");
    let fehler = std::fs::read_to_string(d.zustand.konfig.fehler_pfad()).unwrap_or_default();
    for (wo, inhalt) in [("Akte", &akte), ("Audit", &audit), ("Fehlerlog", &fehler)] {
        assert!(!inhalt.contains("Zebrastreifen"), "der Zweck der § 35a-Buchung steht in {wo}");
    }
    assert!(a["nachfragen"].to_string().contains("Zebrastreifen"), "die Antwort nennt den Zweck: {a}");
}

/// Beleg schlaegt Konto, Seite der Antwort: steht auf dem Feld ein Ereignis des Belegs, schreibt der Kontoauszug nichts und
/// nennt seine Summe als Abgleich (Einheit und Vorzeichen wie die Summe, die der Import ins Feld geschrieben haette: Ausgabe
/// positiv, in Cent).
#[tokio::test]
async fn bank_beleg_schlaegt_konto_antwort_nennt_den_abgleich() {
    let d = dienst();
    fall_anlegen(&d, "bb1", "gesamt").await;
    let (s, _) = sende(
        &d,
        "/fall/bb1/event",
        &json!({"feld_id": "spenden_betrag", "wert": 25_00, "zustand": "vorlaeufig",
            "herkunft": {"haftung": "nutzer", "herkunft": "beleg_import", "pruef_tiefe": "ungeprueft"},
            "schreiber": "import:beleg", "signal": {"signal_1": null, "signal_2": null},
            "ts": "2026-01-01T00:00:00+00:00"}),
    )
    .await;
    assert_eq!(s, 201, "der Beleg-Vorschlag wird angenommen");
    let (s, a) = sende(
        &d,
        "/fall/bb1/kontoauszug",
        &csv(&["02.03.2025;-15,00;Spende Tierheim", "03.03.2025;-20,00;Spende Unicef"]),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["uebernommen"], 0, "{a}");
    let ev = events(&d, "bb1");
    assert_eq!((ev.len(), &ev[0]["wert"], &ev[0]["schreiber"]), (1, &json!(25_00), &json!("import:beleg")), "{ev:?}");
    assert_eq!(a["abgleich"], json!([{"feld_id": "spenden_betrag", "summe_cent": 35_00, "anzahl": 2}]), "{a}");
    let hinweis = a["hinweis"].as_str().unwrap_or_else(|| panic!("kein Hinweis: {a}"));
    assert!(hinweis.contains("Abgleich") && hinweis.contains("35,00 €"), "{hinweis}");
}

/// `ki_buchungstext` und `art9` sind Wahrheitswerte oder fehlen; alles andere ist 400 (wie `ersetzen`), bevor der Auszug gelesen
/// wird. Der Rumpf mit beiden Haken auf `false` oder ohne sie liest den Auszug.
#[tokio::test]
async fn bank_haken_muessen_wahrheitswerte_sein() {
    let d = dienst();
    fall_anlegen(&d, "bh1", "gesamt").await;
    for haken in ["ki_buchungstext", "art9"] {
        for wert in [json!("ja"), json!(1), json!(null), json!([]), json!({"a": true})] {
            let mut b = csv(&["01.03.2025;-31,00;Spende Tierheim"]);
            b[haken] = wert.clone();
            let (s, a) = sende(&d, "/fall/bh1/kontoauszug", &b).await;
            assert_eq!((s, a), (400, json!({"fehler": format!("{haken} muss true oder false sein")})), "{haken}={wert}");
        }
    }
    assert_eq!(events(&d, "bh1").len(), 0, "ein ungueltiger Haken schreibt nichts");
    let mut b = csv(&["01.03.2025;-31,00;Spende Tierheim"]);
    b["ki_buchungstext"] = json!(false);
    b["art9"] = json!(false);
    let (s, a) = sende(&d, "/fall/bh1/kontoauszug", &b).await;
    assert_eq!((s, &a["uebernommen"]), (200, &json!(1)), "{a}");
}

/// Art. 9 (d-60) an der Route: ohne den Haken schlaegt keine Gesundheits-Regel an, die Buchungen zaehlen als nicht ausgewertet und
/// die Antwort sagt es; mit dem Haken schreiben `basis_kv` und `basis_pv` je eine Summe, deren Posten KEINEN Zweck tragen.
/// Weder Akte noch Audit noch Fehlerlog noch Antwort tragen den Buchungstext. Die Texte sind erfunden.
#[tokio::test]
async fn bank_art9_route_ohne_haken_gesperrt_und_mit_haken_ohne_zweck() {
    let d = dienst();
    let zeilen = [
        "01.03.2025;-262,00;Muster Krankenkasse Beitrag Zebra",
        "02.03.2025;-57,00;Muster Pflegeversicherung Beitrag Zebra",
        "03.03.2025;-13,00;Zahnarzt Dr. Beispiel Zebra",
    ];
    // Ohne den Haken.
    let (s, a) = auszug(&d, "ba1", csv(&zeilen)).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!((&a["uebernommen"], &a["transaktionen"]), (&json!(0), &json!(3)), "{a}");
    assert_eq!(events(&d, "ba1").len(), 0, "ohne Haken schreibt keine Gesundheits-Regel");
    let hinweis = a["hinweis"].as_str().unwrap_or_else(|| panic!("kein Hinweis: {a}"));
    assert!(hinweis.contains("3 Buchung(en) mit Bezug zu Gesundheit, Gewerkschaft oder Kirche wurden nicht ausgewertet"), "{hinweis}");
    // Mit dem Haken.
    let mut b = csv(&zeilen);
    b["art9"] = json!(true);
    fall_anlegen(&d, "ba2", "gesamt").await;
    let (s, a) = sende(&d, "/fall/ba2/kontoauszug", &b).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!((&a["uebernommen"], &a["transaktionen"]), (&json!(2), &json!(3)), "{a}");
    let ev = events(&d, "ba2");
    let felder: Vec<_> = ev.iter().map(|e| (e["feld_id"].as_str().unwrap(), e["wert"].clone())).collect();
    assert_eq!(felder, [("basis_kv", json!(262_00)), ("basis_pv", json!(57_00))]);
    for e in &ev {
        assert_eq!(e["zustand"], "vorlaeufig");
        let s1 = &e["signal"]["signal_1"];
        assert_eq!(s1["posten"].as_array().unwrap().len(), 1, "{s1}");
        assert_eq!(s1["posten"][0]["zweck_maskiert"], "", "der Posten einer Art.-9-Regel traegt keinen Zweck: {s1}");
    }
    assert_eq!(ev[0]["signal"]["signal_1"]["regel_id"], "krankenversicherung");
    assert_eq!(ev[1]["signal"]["signal_1"]["regel_id"], "pflegeversicherung");
    let hinweis = a["hinweis"].as_str().unwrap_or_else(|| panic!("kein Hinweis: {a}"));
    assert!(hinweis.contains("Beitragsbescheinigung"), "der Satz der Regel fehlt: {hinweis}");
    assert!(a["hinweis"].as_str().is_some_and(|h| !h.contains("wurden nicht ausgewertet")), "kein Gesperrt-Satz mit Haken: {hinweis}");
    // Der Text steht nirgends: weder in der Antwort noch in einer Akte, im Audit oder im Fehlerlog.
    let audit = std::fs::read_to_string(d.zustand.konfig.audit_pfad()).unwrap_or_default();
    let fehler = std::fs::read_to_string(d.zustand.konfig.fehler_pfad()).unwrap_or_default();
    let akten: String = ["ba1", "ba2"]
        .iter()
        .map(|id| std::fs::read_to_string(d.zustand.konfig.faelle.join(format!("{id}.json"))).unwrap())
        .collect();
    for (wo, inhalt) in [("Akte", &akten), ("Audit", &audit), ("Fehlerlog", &fehler), ("Antwort", &a.to_string())] {
        assert!(!inhalt.contains("Zebra"), "der Buchungstext steht in {wo}");
    }
    assert!(audit.contains("haken_art9=true") && audit.contains("haken_art9=false"), "Kontrolle: das Audit nennt beide Haken-Zustaende: {audit}");
}

/// Das Protokoll haelt den Import mit Zaehlern und Haken fest, nie mit Text, Betrag oder Datum der Buchungen. Neben der Zeile des
/// Dispatchers (`fall_kontoauszug`, Status) steht eine Zeile `bank_import`; ohne KI keine Zeile `llm_call`.
#[tokio::test]
async fn bank_import_audit_nennt_zaehler_und_haken_ohne_text_und_betrag() {
    let d = dienst();
    let (s, a) = auszug(
        &d,
        "bz1",
        csv(&["06.03.2025;-77,00;Maler Quokka", "07.03.2025;-19,00;Spende Tierheim", "08.03.2025;1.000,00;Gehalt"]),
    )
    .await;
    assert_eq!(s, 200, "{a}");
    let eintraege = store::audit::lies(&d.zustand.konfig.audit_pfad()).unwrap();
    let dieser: Vec<_> = eintraege.iter().filter(|e| e.fall_id.as_deref() == Some("bz1")).collect();
    let aktionen: Vec<&str> = dieser.iter().map(|e| e.action.als_str()).collect();
    assert!(aktionen.contains(&"fall_kontoauszug"), "Zeile des Dispatchers: {aktionen:?}");
    assert!(!aktionen.contains(&"llm_call"), "ohne Haken kein Aufruf des Modells: {aktionen:?}");
    let import: Vec<_> = dieser.iter().filter(|e| e.action.als_str() == "bank_import").collect();
    assert_eq!(import.len(), 1, "{aktionen:?}");
    let detail = import[0].detail.as_deref().unwrap();
    for teil in [
        "transaktionen=3 ",
        "uebernommen=1 ",
        "nachgefragt=1 ",
        "art9_gesperrt=0 ",
        "einnahmen=1 ",
        "haken_ersetzen=false haken_art9=false haken_ki_buchungstext=false ki_aufrufe=0",
    ] {
        assert!(detail.contains(teil), "{teil:?} fehlt: {detail}");
    }
    for geheim in ["Maler", "Quokka", "Tierheim", "7700", "77,00", "1900", "2025-03"] {
        assert!(!detail.contains(geheim), "{geheim:?} steht im Protokoll: {detail}");
    }
}
