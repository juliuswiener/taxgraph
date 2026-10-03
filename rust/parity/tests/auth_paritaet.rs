//! Paritaet `rust/auth` gegen `produkt/auth/auth.py` (`tools/parity/schritt8_oracle.py`).
//!
//! - `validierung`: Name/Passwort-Muster auf 1000 generierten Strings (+ Randfaelle).
//! - `jwt`: Rust-Token → Python `verify_token` und umgekehrt, gleiches Geheimnis; dazu
//!   PyJWT-signierte Negativfaelle (abgelaufen, ohne `exp`, `aud`, `sub` als Zahl, fremdes
//!   Geheimnis, `none`, HS384, `iat` in der Zukunft) → beide Seiten dieselbe Entscheidung.
//! - `bcrypt_und_handler`: Hash-Kreuzpruefung; register/login/logout auf beiden Seiten mit
//!   denselben Eingaben (Status + Meldung), Login kreuzweise gegen die Nutzerdatei der anderen
//!   Seite, Datei-Format byte-gleich, Modus 0600.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `auth_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex, OnceLock};

use auth::{Anmeldung, Auth};
use parity::Oracle;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use serde_json::{json, Value};

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn oracle() -> &'static Mutex<Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
}

fn frage(anfrage: &Value) -> Value {
    let a = oracle()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .call_json(anfrage)
        .expect("orakel antwortet");
    a.get("ok")
        .cloned()
        .unwrap_or_else(|| panic!("Orakel-Fehler: {a}"))
}

#[derive(Default)]
struct Zaehler {
    faelle: usize,
    abw: usize,
}
impl Zaehler {
    fn pruefe(&mut self, was: &str, rust: &Value, py: &Value) {
        self.faelle += 1;
        if rust != py {
            self.abw += 1;
            if self.abw <= 5 {
                eprintln!("ABWEICHUNG {was}\n  rust: {rust}\n  py:   {py}");
            }
        }
    }
}

fn strings(n: usize) -> Vec<String> {
    let zeichen = prop::sample::select(vec![
        'a', 'Z', 'q', '0', '9', '_', '-', ' ', '\n', 'ä', 'ß', '€', '.', '@', '\t', '\u{301}', '1',
    ]);
    let strat =
        prop::collection::vec(zeichen, 0..40).prop_map(|v| v.into_iter().collect::<String>());
    let mut runner = TestRunner::deterministic();
    (0..n)
        .map(|_| strat.new_tree(&mut runner).unwrap().current())
        .collect()
}

#[test]
fn validierung() {
    if skip() {
        return;
    }
    let mut s = strings(1000);
    // Namensnahe Strings (erste Stelle meist Buchstabe, Laenge um die Grenzen 3/32), damit die
    // gueltige Seite nicht nur aus Randfaellen besteht.
    let namensnah = (
        prop::sample::select(vec!['a', 'Z', '1', '_', 'ä']),
        prop::collection::vec(
            prop::sample::select(vec!['a', 'Z', '0', '_', '-', 'b', 'Q', '9']),
            0..36,
        ),
    )
        .prop_map(|(k, r)| std::iter::once(k).chain(r).collect::<String>());
    let mut runner = TestRunner::new_with_rng(
        proptest::test_runner::Config::default(),
        proptest::test_runner::TestRng::deterministic_rng(
            proptest::test_runner::RngAlgorithm::ChaCha,
        ),
    );
    s.extend((0..1000).map(|_| namensnah.new_tree(&mut runner).unwrap().current()));
    s.extend(
        [
            "abc",
            "ab",
            "a".repeat(32).as_str(),
            "a".repeat(33).as_str(),
            "1abc",
            "abc\n",
            "x".repeat(8).as_str(),
            "x".repeat(7).as_str(),
            "x".repeat(128).as_str(),
            "x".repeat(129).as_str(),
            "ä".repeat(8).as_str(),
            "abcdefgh\n",
            "",
        ]
        .map(String::from),
    );
    let py = frage(&json!({"fn": "schritt8.auth.validiere", "namen": s, "pw": s}));
    let mut z = Zaehler::default();
    for (i, t) in s.iter().enumerate() {
        z.pruefe(
            &format!("name {t:?}"),
            &json!(auth::ist_gueltiger_username(t)),
            &py["namen"][i],
        );
        z.pruefe(
            &format!("pw {t:?}"),
            &json!(auth::ist_gueltiges_passwort(t)),
            &py["pw"][i],
        );
    }
    let mut neg = Zaehler::default();
    neg.pruefe(
        "negativ",
        &json!(!auth::ist_gueltiger_username(&s[0])),
        &py["namen"][0],
    );
    let gueltig = py["namen"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| **b == json!(true))
        .count();
    println!("validierung: {} Strings (1000 zufaellig + 1000 namensnah + 13 Rand), {} Vergleiche ({gueltig} gueltige Namen); Abweichungen {}; Negativkontrolle {}", s.len(), z.faelle, z.abw, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}

#[test]
fn jwt() {
    if skip() {
        return;
    }
    let geheim = "paritaets-geheimnis-0123456789abcdef";
    let a = Auth::neu(geheim.into(), "/nie/geschrieben".into(), None);
    let mut z = Zaehler::default();
    for sub in ["julius", "a_b-c", "ÄÖÜ", ""] {
        let t = a.stelle_aus(sub).unwrap();
        let py = frage(&json!({"fn": "schritt8.auth.token", "secret": geheim, "token": t}));
        z.pruefe(&format!("rust→py {sub:?}"), &json!(a.pruefe_token(&t)), &py);
        let t_py: String = serde_json::from_value(frage(
            &json!({"fn": "schritt8.auth.token", "secret": geheim, "sub": sub}),
        ))
        .unwrap();
        let py_selbst =
            frage(&json!({"fn": "schritt8.auth.token", "secret": geheim, "token": t_py}));
        z.pruefe(
            &format!("py→rust {sub:?}"),
            &json!(a.pruefe_token(&t_py)),
            &py_selbst,
        );
    }
    let jetzt = chrono::Utc::now().timestamp();
    let faelle = [
        (
            "gueltig",
            json!({"sub": "j", "iat": jetzt, "exp": jetzt + 60, "jti": "x"}),
            geheim,
            "HS256",
        ),
        (
            "abgelaufen",
            json!({"sub": "j", "exp": jetzt - 1}),
            geheim,
            "HS256",
        ),
        (
            "exp == jetzt",
            json!({"sub": "j", "exp": jetzt}),
            geheim,
            "HS256",
        ),
        ("ohne exp", json!({"sub": "j"}), geheim, "HS256"),
        (
            "exp als Text",
            json!({"sub": "j", "exp": format!("{}", jetzt + 60)}),
            geheim,
            "HS256",
        ),
        (
            "exp kein int",
            json!({"sub": "j", "exp": "morgen"}),
            geheim,
            "HS256",
        ),
        (
            "iat Zukunft",
            json!({"sub": "j", "iat": jetzt + 3600}),
            geheim,
            "HS256",
        ),
        (
            "nbf Zukunft",
            json!({"sub": "j", "nbf": jetzt + 3600}),
            geheim,
            "HS256",
        ),
        (
            "aud gesetzt",
            json!({"sub": "j", "aud": "x"}),
            geheim,
            "HS256",
        ),
        ("aud leer", json!({"sub": "j", "aud": ""}), geheim, "HS256"),
        ("sub Zahl", json!({"sub": 5}), geheim, "HS256"),
        ("ohne sub", json!({"x": 1}), geheim, "HS256"),
        ("jti Zahl", json!({"sub": "j", "jti": 1}), geheim, "HS256"),
        (
            "fremdes Geheimnis",
            json!({"sub": "j"}),
            "anderes-geheimnis-0123456789abcdef",
            "HS256",
        ),
        ("HS384", json!({"sub": "j"}), geheim, "HS384"),
        ("none", json!({"sub": "j"}), geheim, "none"),
    ];
    for (name, payload, schluessel, alg) in faelle {
        let t: String = serde_json::from_value(frage(&json!({"fn": "schritt8.auth.encode", "payload": payload, "secret": schluessel, "alg": alg}))).unwrap();
        let py = frage(&json!({"fn": "schritt8.auth.token", "secret": geheim, "token": t}));
        z.pruefe(&format!("jwt {name}"), &json!(a.pruefe_token(&t)), &py);
    }
    for kaputt in ["", "a.b.c", "x", "Bearer "] {
        let py = frage(&json!({"fn": "schritt8.auth.token", "secret": geheim, "token": kaputt}));
        z.pruefe(
            &format!("kaputt {kaputt:?}"),
            &json!(a.pruefe_token(kaputt)),
            &py,
        );
    }
    let mut neg = Zaehler::default();
    let t = a.stelle_aus("julius").unwrap();
    neg.pruefe(
        "negativ",
        &json!(Auth::neu("falsch".into(), "/x".into(), None).pruefe_token(&t)),
        &frage(&json!({"fn": "schritt8.auth.token", "secret": geheim, "token": t})),
    );
    println!("jwt: {} Vergleiche (4 Namen × 2 Richtungen, 16 PyJWT-Faelle, 4 kaputte Tokens); Abweichungen {}; Negativkontrolle {}", z.faelle, z.abw, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}

fn rust_handler(a: &Auth, aktion: &str, body: &Value) -> Value {
    let ergebnis = match aktion {
        "register" => Anmeldung::aus_body(body)
            .and_then(|an| a.registriere(&an))
            .map(|u| {
                (
                    201,
                    json!({"username": u.as_str(), "message": "registriert"}),
                )
            }),
        "login" => Anmeldung::aus_body(body)
            .and_then(|an| a.login(&an))
            .map(|t| (200, json!({"token": t, "username": body["username"]}))),
        _ => {
            a.logout(body.get("token").and_then(Value::as_str).unwrap_or(""));
            Ok((200, json!({"message": "abgemeldet"})))
        }
    };
    match ergebnis {
        Ok((status, mut b)) => {
            if b.get("token").is_some() {
                b["token"] = json!("<token>");
            }
            json!({"status": status, "body": b})
        }
        Err(e) => json!({"status": e.status(), "msg": e.to_string()}),
    }
}

#[test]
fn bcrypt_und_handler() {
    if skip() {
        return;
    }
    let mut z = Zaehler::default();
    // bcrypt kreuzweise (Kosten 12 — wenige Faelle, jeder ~0,25 s).
    for pw in ["geheim123", "ÄÖÜ-lang-passwort", "x".repeat(72).as_str()] {
        let h_py: String =
            serde_json::from_value(frage(&json!({"fn": "schritt8.auth.bcrypt", "pw": pw})))
                .unwrap();
        let h_rust = bcrypt::hash(pw, 12).unwrap();
        z.pruefe(
            &format!("rust prueft py-hash {pw:?}"),
            &json!(bcrypt::verify(pw, &h_py).unwrap()),
            &json!(true),
        );
        z.pruefe(
            &format!("py prueft rust-hash {pw:?}"),
            &frage(&json!({"fn": "schritt8.auth.bcrypt", "pw": pw, "hash": h_rust})),
            &json!(true),
        );
        z.pruefe(
            "falsches pw",
            &frage(&json!({"fn": "schritt8.auth.bcrypt", "pw": "anders", "hash": h_rust})),
            &json!(false),
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let (datei_py, datei_rust) = (
        dir.path().join("py/users.json"),
        dir.path().join("rust/users.json"),
    );
    let geheim = "handler-geheimnis";
    let a = Auth::neu(geheim.into(), datei_rust.clone(), None);
    let lang = "p".repeat(73);
    // Handarbeit an der Nutzerdatei: ein Name, den `register` nie vergibt, steht mit dem Hash von
    // `geheim123` in BEIDEN Dateien (Vault decisions/login-prueft-das-namensmuster-vor-dem-nachschlagen).
    // Die Registrierungsschritte unten schreiben die Dateien danach in ihrem eigenen Format neu.
    let handarbeit = json!({"users": {"a b": {
        "password_hash": bcrypt::hash("geheim123", 4).unwrap(),
        "created_at": "2026-10-03T00:00:00+00:00",
    }}});
    for d in [&datei_py, &datei_rust] {
        std::fs::create_dir_all(d.parent().unwrap()).unwrap();
        std::fs::write(d, handarbeit.to_string()).unwrap();
    }
    let schritte = [
        (
            "register",
            json!({"username": "julius", "password": "geheim123"}),
        ),
        (
            "register",
            json!({"username": "julius", "password": "geheim123"}),
        ),
        (
            "register",
            json!({"username": "1x", "password": "geheim123"}),
        ),
        ("register", json!({"username": "anna"})),
        ("register", json!({})),
        ("register", json!({"username": "anna", "password": "kurz"})),
        ("register", json!({"username": "bert", "password": lang})),
        (
            "register",
            json!({"username": "Ämil", "password": "geheim123"}),
        ),
        (
            "login",
            json!({"username": "julius", "password": "geheim123"}),
        ),
        (
            "login",
            json!({"username": "julius", "password": "falsch123"}),
        ),
        (
            "login",
            json!({"username": "niemand", "password": "geheim123"}),
        ),
        ("login", json!({"username": "julius", "password": lang})),
        ("login", json!({"password": "x"})),
        // Handarbeit-Name ausserhalb des Musters, richtiges Passwort: beide Seiten 401, gleicher Text.
        ("login", json!({"username": "a b", "password": "geheim123"})),
        ("logout", json!({"token": "Bearer kaputt"})),
        ("logout", json!({})),
    ];
    for (aktion, body) in &schritte {
        let py = frage(
            &json!({"fn": "schritt8.auth.handler", "aktion": aktion, "body": body, "datei": datei_py, "secret": geheim}),
        );
        let mut py = py;
        if py["body"].get("token").is_some() {
            py["body"]["token"] = json!("<token>");
        }
        z.pruefe(
            &format!("{aktion} {body}"),
            &rust_handler(&a, aktion, body),
            &py,
        );
    }
    // Absolut, nicht nur gleich: vor dem Bau antworteten beide 200 (gleich, aber falsch). Beide muessen
    // den Namen ABWEISEN, mit Pythons Text.
    let a_b = json!({"username": "a b", "password": "geheim123"});
    let mut py = frage(
        &json!({"fn": "schritt8.auth.handler", "aktion": "login", "body": a_b, "datei": datei_py, "secret": geheim}),
    );
    // Ein Token steht nie in der Fehlermeldung, auch nicht, wenn der Name doch eines bekam.
    if py["body"].get("token").is_some() {
        py["body"]["token"] = json!("<token>");
    }
    let erwartet = json!({"status": 401, "msg": "username oder password falsch"});
    z.pruefe("login 'a b': Python weist ab", &py, &erwartet);
    z.pruefe(
        "login 'a b': Rust weist ab",
        &rust_handler(&a, "login", &a_b),
        &erwartet,
    );
    // Kreuzweise: Rust loggt gegen Pythons Datei ein und umgekehrt.
    let quer = Auth::neu(geheim.into(), datei_py.clone(), None);
    z.pruefe(
        "rust-login auf py-datei",
        &json!(quer
            .login(&Anmeldung {
                username: "julius".into(),
                password: "geheim123".into()
            })
            .is_ok()),
        &json!(true),
    );
    let py = frage(
        &json!({"fn": "schritt8.auth.handler", "aktion": "login", "body": {"username": "julius", "password": "geheim123"}, "datei": datei_rust, "secret": geheim}),
    );
    z.pruefe("py-login auf rust-datei", &json!(py["status"]), &json!(200));
    // Logout-Sperre: Rust-Token, von Python abgemeldet? (Sperrliste ist je Prozess — beide Seiten
    // pruefen nur ihre eigene.) Hier: Rust meldet ab, Rust lehnt ab.
    let t = a
        .login(&Anmeldung {
            username: "julius".into(),
            password: "geheim123".into(),
        })
        .unwrap();
    a.logout(&format!("Bearer {t}"));
    z.pruefe(
        "rust-token nach logout",
        &json!(a.pruefe_token(&t)),
        &Value::Null,
    );
    // Dateiformat: Rust schreibt Pythons Bytes nach, Modus 0600 auf beiden.

    let text_py = std::fs::read_to_string(&datei_py).unwrap();
    let wert: Value = serde_json::from_str(&text_py).unwrap();
    z.pruefe(
        "format py_json",
        &json!(auth::py_json(&wert).unwrap()),
        &json!(text_py),
    );
    for d in [&datei_py, &datei_rust] {
        z.pruefe(
            &format!("modus {}", d.display()),
            &json!(std::fs::metadata(d).unwrap().permissions().mode() & 0o777),
            &json!(0o600),
        );
    }
    let mut neg = Zaehler::default();
    neg.pruefe("negativ", &json!({"status": 409}), &json!({"status": 201}));
    println!("bcrypt+handler: {} Vergleiche (9 bcrypt, {} Handler-Schritte, 2 Kreuz-Logins, Logout, Format, 2 Modi); Abweichungen {}; Negativkontrolle {}",
        z.faelle, schritte.len(), z.abw, neg.abw);
    assert_eq!(neg.abw, 1);
    assert_eq!(z.abw, 0);
}
