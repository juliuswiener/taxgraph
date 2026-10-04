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
            "01.03.2025;-480,00;Maler Huber",
            "02.03.2025;-50,00;Spende Rotes Kreuz",
            "03.03.2025;-300,00;Minijob-Zentrale",
            "04.03.2025;-1.200,00;Rentenversicherung Ruerup",
            "05.03.2025;1.000,00;Gehalt",
            "06.03.2025;-20,00;Reinigung Treppenhaus",
            "07.03.2025;-9,99;Einkauf",
        ]),
    )
    .await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(
        antwort,
        json!({"uebernommen": 5, "transaktionen": 7, "verworfen": 0})
    );
    let ev = events(&d, "ka1");
    assert_eq!(ev.len(), 5);
    for e in &ev {
        assert_eq!(e["schreiber"], "import:kontoauszug");
        assert_eq!(e["zustand"], "vorlaeufig");
        assert_eq!(e["herkunft"]["herkunft"], "kontoauszug");
        assert!(e["signal"]["signal_2"].is_null());
    }
    let felder: Vec<_> = ev.iter().map(|e| e["feld_id"].as_str().unwrap()).collect();
    assert_eq!(
        felder,
        [
            "hh_handwerker_betrag",
            "spenden_betrag",
            "hh_minijob_betrag",
            "vor_rv_ausserhalb_lstb",
            "hh_dienstleistung_betrag"
        ]
    );
    // Der Betrag steht ohne Vorzeichen im Feld und mit Vorzeichen im Beleg (`signal_1`).
    assert_eq!(ev[0]["wert"], 48_000);
    assert_eq!(ev[0]["signal"]["signal_1"]["betrag"], -48_000);
    assert_eq!(ev[0]["signal"]["signal_1"]["datum"], "01.03.2025");
    assert_eq!(ev[0]["signal"]["signal_1"]["quelle"], "heuristik");
}

#[tokio::test]
async fn unlesbarer_betrag_faellt_einzeln_raus() {
    let d = dienst();
    let (status, antwort) = auszug(
        &d,
        "ka2",
        csv(&[
            "01.03.2025;-100000000,00;Maler",
            "01.03.2025;-99999999,99;Maler",
            "01.03.2025;100000000,00;Gehalt",
            "01.03.2025;-92233720368547758,07;Maler",
            "01.03.2025;-92233720368547758,08;Maler",
            "01.03.2025;-123456789012345678901,00;Maler",
            "01.03.2025;-abc;Maler",
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

#[tokio::test]
async fn deckel_der_llm_aufrufe_ist_kein_stilles_kuerzen() {
    let d = dienst();
    let mut tx: Vec<Value> = (0..56)
        .map(|i| json!({"datum": "d", "betrag": -100 - i, "verwendungszweck": format!("Einkauf {i}")}))
        .collect();
    tx.push(json!({"datum": "d", "betrag": -5000, "verwendungszweck": "Spende"}));
    tx.push(json!({"datum": "d", "betrag": "x", "verwendungszweck": "Maler"}));
    let (status, antwort) = auszug(&d, "kd", json!({"format": "json", "inhalt": tx})).await;
    assert_eq!(status, 200, "{antwort}");
    assert_eq!(
        antwort,
        json!({"uebernommen": 1, "transaktionen": 57, "verworfen": 1, "llm_uebersprungen": 6,
            "hinweis": "1 Zeile(n) mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen. 6 Buchung(en) wurden NICHT automatisch eingeordnet — die Grenze von 50 Klassifikationen je Auszug war erreicht. Bitte diese Buchungen selbst zuordnen oder den Auszug in kleineren Zeiträumen hochladen."})
    );
    // Genau am Deckel: nichts wird übersprungen, und der Schlüssel fehlt.
    let genau: Vec<Value> = (0..50)
        .map(|i| json!({"datum": "d", "betrag": -100 - i, "verwendungszweck": format!("Einkauf {i}")}))
        .collect();
    let (status, antwort) = auszug(&d, "kd2", json!({"format": "json", "inhalt": genau})).await;
    assert_eq!(
        (status, antwort),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 50, "verworfen": 0})
        )
    );
    // Auftrag k9-2 (Mutant K40): EINE übersprungene Buchung ist schon ein Hinweis wert (Python: `> 0`). Die
    // Antwort stammt aus `api.kontoauszug` (CPython 3.14), 51 Buchungen ohne Stichwort, kein Schlüssel.
    let eine: Vec<Value> = (0..51)
        .map(|i| json!({"datum": "d", "betrag": -100 - i, "verwendungszweck": format!("Einkauf {i}")}))
        .collect();
    let (status, antwort) = auszug(&d, "kd3", json!({"format": "json", "inhalt": eine})).await;
    assert_eq!(
        (status, antwort),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 51, "verworfen": 0, "llm_uebersprungen": 1,
                "hinweis": "1 Buchung(en) wurden NICHT automatisch eingeordnet — die Grenze von 50 Klassifikationen je Auszug war erreicht. Bitte diese Buchungen selbst zuordnen oder den Auszug in kleineren Zeiträumen hochladen."})
        )
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
    assert!(events(&d, "ke").is_empty());
}

#[tokio::test]
async fn zielfeld_gilt_je_scheibe_und_wird_nicht_ueberschrieben() {
    let d = dienst();
    // `ep` führt `spenden_betrag` nicht: keine Buchung, aber die Zeile zählt.
    fall_anlegen(&d, "kz1", "ep").await;
    let (status, antwort) = sende(
        &d,
        "/fall/kz1/kontoauszug",
        &json!({"format": "json", "inhalt": [{"betrag": -5000, "verwendungszweck": "Spende"}]}),
    )
    .await;
    assert_eq!(
        (status, antwort),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 1, "verworfen": 0})
        )
    );
    // Zwei Buchungen derselben Kategorie: nur die erste; ein zweiter Auszug ändert nichts mehr.
    let rumpf = json!({"format": "json", "inhalt": [
        {"betrag": -100, "verwendungszweck": "Maler"}, {"betrag": -200, "verwendungszweck": "Klempner"}]});
    let (_, antwort) = auszug(&d, "kz2", rumpf.clone()).await;
    assert_eq!(antwort["uebernommen"], 1);
    let (_, antwort) = sende(&d, "/fall/kz2/kontoauszug", &rumpf).await;
    assert_eq!(antwort["uebernommen"], 0);
    assert_eq!(events(&d, "kz2").len(), 1);
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
    let tx = |betrag: Value, zweck: Value| json!({"format": "json", "inhalt": [{"datum": "d", "betrag": betrag, "verwendungszweck": zweck}]});
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
    // Ein fehlendes Datum steht als leerer Text im Beleg, ein vorhandenes unverändert.
    let (_, a) = auszug(
        &d,
        "kw30",
        json!({"format": "json", "inhalt": [{"betrag": -5000, "verwendungszweck": "Spende"}]}),
    )
    .await;
    assert_eq!(a["uebernommen"], 1);
    assert_eq!(events(&d, "kw30")[0]["signal"]["signal_1"]["datum"], "");
    let (_, a) = auszug(&d, "kw31", json!({"format": "json", "inhalt": [{"datum": 5.5, "betrag": -5000, "verwendungszweck": "Spende"}]})).await;
    assert_eq!(a["uebernommen"], 1);
    assert_eq!(events(&d, "kw31")[0]["signal"]["signal_1"]["datum"], 5.5);
    // CSV: der Inhalt, der kein Text ist, ist leer; Vorzeichen wie `lstrip("+-")`.
    let (s, a) = auszug(&d, "kc0", json!({"format": "csv", "inhalt": 5})).await;
    assert_eq!(
        (s, a),
        (
            200,
            json!({"uebernommen": 0, "transaktionen": 0, "verworfen": 0})
        )
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
            csv(&[&format!("01.03.2025;{betrag};Maler")]),
        )
        .await;
        assert_eq!(
            (s, a),
            (
                200,
                json!({"uebernommen": gebucht, "transaktionen": 1, "verworfen": 0})
            ),
            "{betrag}"
        );
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
    // Kontrolle: tragbare Daten werden wie vorher übernommen, auch die Grenzen von i64 und Text "NaN".
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
                json!({"uebernommen": 1, "transaktionen": 1, "verworfen": 0})
            ),
            "{datum}"
        );
    }
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
        r#"[{{"datum": NaN, "betrag": NaN, "verwendungszweck": {ruerup}}}, {{"datum": "d", "betrag": -120000, "verwendungszweck": {ruerup}}}]"#
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
        let (s, a) = auszug(&d, &format!("bl{i}"), auszug_text(r#""d""#, zweck, betrag)).await;
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
    assert!(events(&d, "br0").is_empty());
}
