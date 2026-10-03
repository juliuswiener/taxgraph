//! Vier Abweichungen bei Zahlen und Gestalten aus einem Fremddienst, die Python als Absturz zeigt und
//! Rust als Rueckfall (Vault `decisions/fremddienst-zahlfehler-nur-in-python-bleiben-bis-zum-cutover`;
//! Messung `berichte/k9-fremddienst.md`, `fa181f0`/`b4fbe8b`), dazu eine fuenfte (C, einzelnes Surrogat
//! im Text), bei der Python mehr rettet als Rust und die Frage offen ist.
//!
//! Das ist KEIN Paarvergleich: beide Server antworten verschieden. Der Fall haelt fest, WIE beide
//! abweichen:
//! - die Rust-Seite ist die Soll-Seite (Korrektheit vor Paritaet, `REWRITE_PLAN.md` §4, Auflage (1)):
//!   wird sie anders, faellt der Fall. AUSNAHME C: dort ist nicht entschieden, welche Seite korrekt
//!   ist; der Fall haelt beide Seiten fest, bis Julius entscheidet;
//! - die Python-Seite ist der gemessene Ist-Zustand: wird Python repariert, faellt der Fall, und der
//!   Eintrag ist zu streichen (Auflage (2): die Abweichung steht samt Begruendung in der Liste).
//!
//! | Nr. | Ausloeser                                                  | Python                   | Rust                |
//! |-----|------------------------------------------------------------|--------------------------|---------------------|
//! | A1  | `rechenweg` / `vorschlag_wert` mit `NaN`, `Infinity`, `1e400` | 200, kein gueltiges JSON | 200, gueltiges JSON |
//! | A2  | `aussage` mit `Infinity`, `1e400`                          | 500 `OverflowError`      | 200                 |
//! | A3  | ORS `distance` mit `NaN`, `Infinity`, `1e400`              | 500                      | 503 (Rueckfall)     |
//! | B   | `content` / `kategorie` als Liste, Objekt, Zahl            | 500                      | 200                 |
//! | C   | `begruendung` mit einzelnem Surrogat-Escape (`\ud800`)     | 200, nur das Feld abgelehnt | 200, ganze Antwort verworfen |
//!
//! Die Steuerakte bleibt in allen Faellen unberuehrt: `ereignisse` zaehlt die Ereignisse der Akte nach
//! der Anfrage (Python schreibt in A1 den gueltigen Vorschlag, Rust verwirft die ganze Antwort; in C
//! schreibt Python den gesunden Vorschlag, Rust keinen).
//! Dass Rust "zurueckfaellt", sagt der Body: `aussagen[0].status` ist `kein_feld` (Stufe 3 unlesbar) oder
//! `werte_ausgefallen` (Stufe 3 gescheitert), nie ein Vorschlag aus einer halb gelesenen Antwort.
use serde_json::{json, Value};

use super::{
    chat_huelle, geocode_nach, geocode_ok, regeln_aus, route_mit, Schritt, Stub, ASD, GGR, TEXT,
};
use crate::{ereignis, schreibe_seed, sende, starte_mit, Anfrage, Server};

/// Wie eine Seite antwortet.
struct Seite {
    status: u16,
    /// `Ok(())`: der Body ist gueltiges JSON. `Err(t)`: er ist es nicht und enthaelt `t` (das bare
    /// `NaN` / `Infinity`, das `json.dumps` schreibt und kein Browser liest).
    json: Result<(), &'static str>,
    /// Anfang von `fehler` bei 500/503 (Pythons Klasse und Meldung bzw. der Rueckfall-Text).
    fehler_beginnt: Option<&'static str>,
    /// Ereignisse der Akte nach der Anfrage (die `vorher`-Ereignisse mitgezaehlt).
    ereignisse: usize,
    /// Die Anfragen an den Dienst (Schluessel wie im Skript).
    anfragen: &'static [&'static str],
    /// Stellen des Bodys (JSON-Zeiger) mit ihrem Wert; nur bei gueltigem JSON.
    koerper: Vec<(&'static str, Value)>,
}

struct Abweichung {
    nr: &'static str,
    name: &'static str,
    route: &'static str,
    rumpf: Value,
    vorher: Vec<(&'static str, Value)>,
    schritte: Vec<Schritt>,
    python: Seite,
    rust: Seite,
    /// Die Nutzerwirkung: was der Nutzer in Python sieht und was in Rust.
    grund: &'static str,
}

const AUSZUG_ZWECK: &str = "Sonstige Zahlung Meyer GmbH Rechnung 77";
const NIE_GELESEN: &[&str] = &["chat:aussagen", "chat:zuordnung"];
const EIN_ANRUF: &[&str] = &["chat:"];
const UEBERLAUF: &str = "OverflowError: cannot convert float infinity to integer";

/// Stufe 3 des Gespraechs als ROHER Text. Er enthaelt `NaN` oder `1e400`, also nichts, was `json!`
/// schreiben koennte. `wert` ist gueltig (60000 Euro), der Rest laesst sich je Fall verderben.
fn stufe3(rechenweg: &str, aussage: &str, rueckfragen: &str, wert: &str) -> String {
    format!(
        r#"{{"vorschlaege": [{{"feld_id": "bruttoarbeitslohn", "wert": {wert}, "beleg": "60000 Euro brutto",
             "begruendung": "b", "aussage": {aussage}, "rechenweg": {rechenweg}}}],
             "rueckfragen": {rueckfragen}, "antwort": "ok", "unsicher": false}}"#
    )
}

fn stufen(dialog: &str) -> Vec<Schritt> {
    vec![
        Schritt::chat(
            "chat:aussagen",
            &json!({"aussagen": [
                {"text": "Der Nutzer hat 60000 Euro brutto verdient", "beleg": "60000 Euro brutto verdient"}]}),
            "stop",
        ),
        Schritt::regeln(3),
        Schritt::roh("chat:dialog", 200, &chat_huelle(dialog, "stop")),
    ]
}

/// Stufe 3 mit eigener Huelle (Gestalt von `message.content`).
fn stufen_mit_huelle(huelle: &str) -> Vec<Schritt> {
    let mut s = stufen("");
    s[2] = Schritt::roh("chat:dialog", 200, huelle);
    s
}

fn content(inhalt: &str) -> String {
    format!(
        r#"{{"provider": "StubAnbieter", "choices": [{{"finish_reason": "stop", "message": {{"content": {inhalt}}}}}]}}"#
    )
}

fn chat_fall(
    nr: &'static str,
    name: &'static str,
    schritte: Vec<Schritt>,
    python: Seite,
    rust: Seite,
    grund: &'static str,
) -> Abweichung {
    Abweichung {
        nr,
        name,
        route: "chat",
        rumpf: json!({ "text": TEXT }),
        vorher: vec![],
        schritte,
        python,
        rust,
        grund,
    }
}

const A1_GRUND: &str = "Python schreibt den Vorschlag und schickt eine Antwort mit nacktem NaN/Infinity (kein JSON): \
    der Browser meldet einen Fehler, obwohl der Vorschlag gespeichert ist. Rust liest die Modellantwort \
    mit serde_json, verwirft sie als Ganzes und antwortet gueltig, ohne Vorschlag.";
const A2_GRUND: &str = "Python: `int(Infinity)` wirft `OverflowError`, 500, der Nutzer verliert die Antwort. Rust: \
    gueltige Antwort (die Modellantwort mit der unlesbaren Zahl faellt weg, bei Stufe 2 greift der ganze Katalog).";
const A3_GRUND: &str =
    "Python: `round(NaN)` / `round(Infinity)` wirft, 500 statt des Rueckfalls. Rust: 503, die \
    Oberflaeche faellt auf die manuelle Eingabe der Kilometer zurueck.";
const B_GRUND: &str =
    "Python ruft `.strip()` bzw. `in dict` auf einen Nicht-Text, 500. Rust behandelt die Antwort \
    als unbrauchbar: 200 ohne Vorschlag, der Auszug bleibt unklassifiziert.";

const C_GRUND: &str = "Python weist nur das Feld mit dem Surrogat ab (`UnicodeEncodeError` beim Schreiben der Akte, \
    Grund in `abgelehnt_gruende`) und behaelt das zweite Feld der Antwort. Rust liest Stufe 3 mit serde_json, das \
    ein einzelnes Surrogat-Escape als Ganzes ablehnt: kein Vorschlag, auch nicht fuer das gesunde Feld. OFFEN: \
    Hier ist Python fuer den Nutzer besser (ein Vorschlag mehr); ob Rust je Feld abweisen soll, ist nicht \
    entschieden (REWRITE_PLAN §4: wo unklar ist, was korrekt ist, geht die Stelle als Frage an Julius).";

/// Python: 200, aber der Body ist kein JSON und nennt `enthaelt`; der Vorschlag steht in der Akte.
fn python_kein_json(enthaelt: &'static str, ereignisse: usize) -> Seite {
    Seite {
        status: 200,
        json: Err(enthaelt),
        fehler_beginnt: None,
        ereignisse,
        anfragen: ASD,
        koerper: vec![],
    }
}

/// Python: 500 mit Pythons Klasse und Meldung, nichts in der Akte.
fn python_500(fehler: &'static str, anfragen: &'static [&'static str]) -> Seite {
    Seite {
        status: 500,
        json: Ok(()),
        fehler_beginnt: Some(fehler),
        ereignisse: 0,
        anfragen,
        koerper: vec![],
    }
}

/// Rust: 200 mit gueltigem JSON.
fn rust_200(
    ereignisse: usize,
    anfragen: &'static [&'static str],
    koerper: Vec<(&'static str, Value)>,
) -> Seite {
    Seite {
        status: 200,
        json: Ok(()),
        fehler_beginnt: None,
        ereignisse,
        anfragen,
        koerper,
    }
}

/// Rust fiel zurueck: kein Vorschlag, kein Konflikt; `status` sagt, warum.
fn ohne_vorschlag(status: &'static str) -> Vec<(&'static str, Value)> {
    vec![
        ("/vorschlaege", json!([])),
        ("/konflikte", json!([])),
        ("/aussagen/0/status", json!(status)),
    ]
}

#[allow(clippy::too_many_lines)]
fn faelle(regeln: &[String]) -> Vec<Abweichung> {
    let mut v = Vec::new();
    // ---------------------------------------------------------------- A1: Antwort kein gueltiges JSON
    v.push(chat_fall(
        "A1",
        "rechenweg NaN",
        stufen(&stufe3("NaN", "0", "[]", "6000000")),
        python_kein_json("NaN", 1),
        rust_200(0, ASD, ohne_vorschlag("kein_feld")),
        A1_GRUND,
    ));
    v.push(chat_fall(
        "A1",
        "rechenweg 1e400 (Python liest Infinity)",
        stufen(&stufe3("1e400", "0", "[]", "6000000")),
        python_kein_json("Infinity", 1),
        rust_200(0, ASD, ohne_vorschlag("kein_feld")),
        A1_GRUND,
    ));
    for (name, wert, enthaelt) in [
        ("vorschlag_wert NaN im Konflikt", "NaN", "NaN"),
        (
            "vorschlag_wert Infinity im Konflikt",
            "Infinity",
            "Infinity",
        ),
    ] {
        // Das Feld ist schon belegt: der Vorschlag wird zum Konflikt, und `vorschlag_wert` traegt die Zahl.
        v.push(Abweichung {
            vorher: vec![("bruttoarbeitslohn", json!(5_000_000))],
            ..chat_fall(
                "A1",
                name,
                stufen(&stufe3("null", "0", "[]", wert)),
                python_kein_json(enthaelt, 1),
                rust_200(1, ASD, ohne_vorschlag("kein_feld")),
                A1_GRUND,
            )
        });
    }
    // ---------------------------------------------------------------- A2: aussage Infinity / 1e400
    for (name, aussage) in [
        ("Stufe 3 aussage Infinity", "Infinity"),
        ("Stufe 3 aussage 1e400", "1e400"),
    ] {
        v.push(chat_fall(
            "A2",
            name,
            stufen(&stufe3("null", aussage, "[]", "6000000")),
            python_500(UEBERLAUF, ASD),
            rust_200(0, ASD, ohne_vorschlag("kein_feld")),
            A2_GRUND,
        ));
    }
    v.push(chat_fall(
        "A2",
        "Rueckfrage aussage Infinity",
        stufen(&stufe3(
            "null",
            "0",
            r#"[{"frage": "x?", "feld_id": "bruttoarbeitslohn", "aussage": Infinity}]"#,
            "6000000",
        )),
        python_500(UEBERLAUF, ASD),
        rust_200(0, ASD, ohne_vorschlag("kein_feld")),
        A2_GRUND,
    ));
    if let Some(erste) = regeln.first() {
        // Stufe 2 mit einer Regel, die es gibt (sonst fiele die Zeile vor `_index` weg).
        let zuordnung =
            format!(r#"{{"zuordnungen": [{{"aussage": 1e400, "regeln": ["{erste}"]}}]}}"#);
        let mut s = stufen(&stufe3("null", "0", "[]", "6000000"));
        s[1] = Schritt::roh("chat:zuordnung", 200, &chat_huelle(&zuordnung, "stop"));
        v.push(chat_fall(
            "A2",
            "Stufe 2 aussage 1e400",
            s,
            python_500(UEBERLAUF, NIE_GELESEN),
            rust_200(
                1,
                ASD,
                vec![
                    ("/vorschlaege/0/feld_id", json!("bruttoarbeitslohn")),
                    ("/aussagen/0/status", json!("vorschlag")),
                ],
            ),
            A2_GRUND,
        ));
    }
    // ---------------------------------------------------------------- A3: ORS distance
    for (name, distanz, fehler) in [
        (
            "distance NaN",
            "NaN",
            "ValueError: cannot convert float NaN to integer",
        ),
        ("distance Infinity", "Infinity", UEBERLAUF),
        ("distance -Infinity", "-Infinity", UEBERLAUF),
        ("distance 1e400", "1e400", UEBERLAUF),
    ] {
        v.push(Abweichung {
            nr: "A3",
            name,
            route: "entfernung",
            rumpf: json!({"von": "Musterstr. 1, 80331 München", "nach": "Beispielweg 2, 80333 München"}),
            vorher: vec![],
            schritte: vec![geocode_ok(), geocode_nach(), route_mit(distanz)],
            python: python_500(fehler, GGR),
            rust: Seite {
                status: 503,
                json: Ok(()),
                fehler_beginnt: Some("unavailable"),
                ereignisse: 0,
                anfragen: GGR,
                koerper: vec![],
            },
            grund: A3_GRUND,
        });
    }
    // ---------------------------------------------------------------- B: Gestalt von content / kategorie
    for (name, inhalt, fehler) in [
        (
            "content Liste",
            r#"["a"]"#,
            "AttributeError: 'list' object has no attribute 'strip'",
        ),
        (
            "content Zahl",
            "5",
            "AttributeError: 'int' object has no attribute 'strip'",
        ),
        (
            "content true",
            "true",
            "AttributeError: 'bool' object has no attribute 'strip'",
        ),
        (
            "content NaN",
            "NaN",
            "AttributeError: 'float' object has no attribute 'strip'",
        ),
    ] {
        v.push(chat_fall(
            "B",
            name,
            stufen_mit_huelle(&content(inhalt)),
            python_500(fehler, ASD),
            rust_200(0, ASD, ohne_vorschlag("werte_ausgefallen")),
            B_GRUND,
        ));
    }
    let auszug = |name: &'static str, huelle: String, fehler: &'static str| Abweichung {
        nr: "B",
        name,
        route: "kontoauszug",
        rumpf: json!({"format": "json", "inhalt": format!(
            r#"[{{"datum": "2025-03-15", "betrag": -120000, "verwendungszweck": "{AUSZUG_ZWECK}"}}]"#)}),
        vorher: vec![],
        schritte: vec![Schritt::roh("*", 200, &huelle).immer()],
        python: python_500(fehler, EIN_ANRUF),
        rust: rust_200(0, EIN_ANRUF, vec![("/uebernommen", json!(0))]),
        grund: B_GRUND,
    };
    let liste = "TypeError: cannot use 'list' as a dict key (unhashable type: 'list')";
    let objekt = "TypeError: cannot use 'dict' as a dict key (unhashable type: 'dict')";
    for (name, kategorie, fehler) in [
        ("kategorie Liste", "[]", liste),
        ("kategorie Liste mit Treffer", r#"["spende"]"#, liste),
        ("kategorie Objekt", r#"{"a": 1}"#, objekt),
    ] {
        v.push(auszug(
            name,
            chat_huelle(&format!(r#"{{"kategorie": {kategorie}}}"#), "stop"),
            fehler,
        ));
    }
    v.push(auszug(
        "Klassifikator content Zahl",
        content("5"),
        "AttributeError: 'int' object has no attribute 'strip'",
    ));
    // ---------------------------------------------------------------- C: einzelnes Surrogat im Text
    // Zwei Vorschlaege in EINER Antwort: das erste traegt ein einzelnes Surrogat-Escape in `begruendung`,
    // das zweite ist in Ordnung. Das zeigt, was der Nutzer verliert.
    let zwei_vorschlaege = r#"{"vorschlaege": [
        {"feld_id": "bruttoarbeitslohn", "wert": 6000000, "beleg": "60000 Euro brutto",
         "begruendung": "\ud800", "aussage": 0, "rechenweg": null},
        {"feld_id": "fam_anzahl_kinder", "wert": 0, "beleg": "bin ledig",
         "begruendung": "b", "aussage": 0, "rechenweg": null}],
        "rueckfragen": [], "antwort": "ok", "unsicher": false}"#;
    v.push(chat_fall(
        "C",
        "begruendung mit Surrogat, zweites Feld in Ordnung",
        stufen(zwei_vorschlaege),
        rust_200(
            1,
            ASD,
            vec![
                ("/vorschlaege/0/feld_id", json!("fam_anzahl_kinder")),
                ("/abgelehnt", json!(["bruttoarbeitslohn"])),
                (
                    "/abgelehnt_gruende/bruttoarbeitslohn",
                    json!("UnicodeEncodeError: bruttoarbeitslohn"),
                ),
                ("/aussagen/0/status", json!("vorschlag")),
            ],
        ),
        rust_200(
            0,
            ASD,
            vec![
                ("/vorschlaege", json!([])),
                ("/abgelehnt", json!([])),
                ("/aussagen/0/status", json!("kein_feld")),
            ],
        ),
        C_GRUND,
    ));
    v
}

/// Die Regel-Kennungen, die der Server in Stufe 2 an das Modell schickt (wie `Schritt::regeln` sie liest).
fn regel_kennungen(server: &Server, stub: &Stub) -> Vec<String> {
    stub.setze(stufen(&stufe3("null", "0", "[]", "6000000")));
    neuer_fall(server, "regeln");
    let a = Anfrage::neu("regeln", "POST", "/fall/regeln/chat").json(&json!({ "text": TEXT }));
    let _ = sende(server.port, &a);
    let zuordnung = stub
        .nimm()
        .into_iter()
        .find(|g| g.schluessel == "chat:zuordnung")
        .expect("Stufe 2 wurde nicht angefragt");
    regeln_aus(zuordnung.body.as_bytes())
}

fn neuer_fall(server: &Server, id: &str) {
    let a = Anfrage::neu("anlegen", "POST", "/fall")
        .json(&json!({"fall_id": id, "scheibe": "gesamt", "veranlagungszeitraum": 2025}));
    let r = sende(server.port, &a);
    assert_eq!(
        r.status,
        201,
        "Fall {id}: {}",
        String::from_utf8_lossy(&r.body)
    );
}

fn ereignisse_der_akte(server: &Server, id: &str) -> usize {
    let datei = server.faelle().join(format!("{id}.json"));
    let akte: Value = serde_json::from_slice(&std::fs::read(&datei).unwrap()).unwrap();
    akte["events"].as_array().map_or(0, Vec::len)
}

/// Fuehrt einen Fall an EINEM Server aus und meldet, was von `soll` abweicht.
fn pruefe_seite(
    server: &Server,
    stub: &Stub,
    f: &Abweichung,
    id: &str,
    soll: &Seite,
) -> Vec<String> {
    neuer_fall(server, id);
    for (feld, wert) in &f.vorher {
        let a = Anfrage::neu("vorher", "POST", &format!("/fall/{id}/event"))
            .json(&ereignis(feld, wert, None));
        let r = sende(server.port, &a);
        assert_eq!(
            r.status,
            201,
            "{} vorher {feld}: {}",
            f.name,
            String::from_utf8_lossy(&r.body)
        );
    }
    stub.setze(f.schritte.clone());
    let a = Anfrage::neu(f.name, "POST", &format!("/fall/{id}/{}", f.route)).json(&f.rumpf);
    let r = sende(server.port, &a);
    let gesehen = stub.nimm();
    let mut d = Vec::new();
    if r.status != soll.status {
        d.push(format!("Status {}, erwartet {}", r.status, soll.status));
    }
    let text = String::from_utf8_lossy(&r.body).into_owned();
    let gueltig = serde_json::from_slice::<Value>(&r.body).ok();
    match (&soll.json, &gueltig) {
        (Ok(()), None) => d.push(format!(
            "Body kein gueltiges JSON: {}",
            text.chars().take(120).collect::<String>()
        )),
        (Err(t), Some(_)) => d.push(format!(
            "Body ist gueltiges JSON, erwartet ungueltig mit {t:?}"
        )),
        (Err(t), None) if !text.contains(t) => d.push(format!("Body enthaelt {t:?} nicht")),
        _ => {}
    }
    if let Some(erwartet) = soll.fehler_beginnt {
        let ist = gueltig
            .as_ref()
            .and_then(|b| b["fehler"].as_str())
            .unwrap_or_default();
        if !ist.starts_with(erwartet) {
            d.push(format!("fehler {ist:?} beginnt nicht mit {erwartet:?}"));
        }
    }
    for (zeiger, erwartet) in &soll.koerper {
        let ist = gueltig.as_ref().and_then(|b| b.pointer(zeiger));
        if ist != Some(erwartet) {
            d.push(format!("Body {zeiger}: {ist:?}, erwartet {erwartet}"));
        }
    }
    let ereignisse = ereignisse_der_akte(server, id);
    if ereignisse != soll.ereignisse {
        d.push(format!(
            "{ereignisse} Ereignisse in der Akte, erwartet {}",
            soll.ereignisse
        ));
    }
    let namen: Vec<&str> = gesehen.iter().map(|g| g.schluessel.as_str()).collect();
    if namen != soll.anfragen {
        d.push(format!(
            "Anfragen an den Dienst {namen:?}, erwartet {:?}",
            soll.anfragen
        ));
    }
    d
}

/// Die Faelle der Klassen `nrs` gegen je einen Python- und einen Rust-Server mit je eigenem Stub. Die
/// Aufrufer sind die Eintraege 6 (A1-A3, B) und 7 (C) in `dokumentierte_abweichungen`.
pub(crate) fn pruefe(nrs: &[&str]) {
    let wurzel = tempfile::tempdir().unwrap();
    let seed = wurzel.path().join("seed");
    schreibe_seed(&seed);
    let (stub_py, stub_rs) = (Stub::starte(), Stub::starte());
    let (env_py, env_rs) = (stub_py.umgebung(), stub_rs.umgebung());
    let extra_py: Vec<(&str, &str)> = env_py.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let extra_rs: Vec<(&str, &str)> = env_rs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let py = starte_mit("python", wurzel.path(), true, false, &seed, &extra_py);
    let rs = starte_mit("rust", wurzel.path(), true, false, &seed, &extra_rs);
    let regeln = regel_kennungen(&py, &stub_py);
    assert!(
        regeln.len() >= 3,
        "Stufe 2 schickte keine Regel-Kennungen: {regeln:?}"
    );
    let mut abweichungen = Vec::new();
    let faelle: Vec<Abweichung> = faelle(&regeln)
        .into_iter()
        .filter(|f| nrs.contains(&f.nr))
        .collect();
    for (i, f) in faelle.iter().enumerate() {
        let id = format!("fremd_{i}");
        let (dp, dr) = (
            pruefe_seite(&py, &stub_py, f, &id, &f.python),
            pruefe_seite(&rs, &stub_rs, f, &id, &f.rust),
        );
        println!(
            "  {} {}: py {} {} | rs {} {}",
            f.nr,
            f.name,
            f.python.status,
            if f.python.json.is_ok() {
                "JSON"
            } else {
                "KEIN JSON"
            },
            f.rust.status,
            if f.rust.json.is_ok() {
                "JSON"
            } else {
                "KEIN JSON"
            },
        );
        abweichungen.extend(
            dp.into_iter()
                .map(|x| format!("{} {} [python]: {x}", f.nr, f.name)),
        );
        abweichungen.extend(
            dr.into_iter()
                .map(|x| format!("{} {} [rust]: {x}", f.nr, f.name)),
        );
        assert!(
            !f.grund.is_empty(),
            "{} {}: Begruendung fehlt",
            f.nr,
            f.name
        );
    }
    for nr in nrs {
        assert!(faelle.iter().any(|f| f.nr == *nr), "kein Fall fuer {nr}");
    }
    println!(
        "EXTERN Fremddienst-Abweichungen {nrs:?}: {} Faelle, {} Abweichungen vom Soll",
        faelle.len(),
        abweichungen.len()
    );
    assert!(abweichungen.is_empty(), "{abweichungen:#?}");
}
