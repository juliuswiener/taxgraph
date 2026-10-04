//! Entscheidungs- und Grenzstellen der Crate `auth`, hermetisch und ohne Python: HTTP-Status je
//! Fehlerart, Audit-Spur je Pfad, Nutzerdatei (bcrypt-Kosten, `created_at`), 72-Byte-Grenze,
//! Token-Inhalt (`sub`, `exp`, `jti`) und Token-Pruefung gegen von Hand signierte Tokens
//! (Sperrliste, fehlende/falsche Claims). Gemessen mit einer Mutationsliste von Hand
//! (`berichte/auth-eingang-mutation.md`): diese Stellen hielt vorher kein Test.
#![allow(
    clippy::cast_precision_loss, // Zeitstempel in Sekunden: weit unter 2^52
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used
)]

use auth::{Anmeldung, Auth, AuthFehler, Username, JWT_TTL_S};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde_json::{json, Value};
use store::audit::lies;

const GEHEIM: &str = "geheimnis-fuer-die-tests";

fn jetzt() -> i64 {
    chrono::Utc::now().timestamp()
}

fn an(u: &str, p: &str) -> Anmeldung {
    Anmeldung {
        username: u.into(),
        password: p.into(),
    }
}

fn ohne_datei() -> Auth {
    Auth::neu(GEHEIM.into(), "/nie/gelesen/users.json".into(), None)
}

/// Ein Token mit beliebigem Payload, vom Dienst-Geheimnis signiert: so gelangen Claims in die
/// Pruefung, die `stelle_aus` nie ausstellt.
fn fremd(payload: &Value) -> String {
    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        payload,
        &EncodingKey::from_secret(GEHEIM.as_bytes()),
    )
    .unwrap()
}

/// Der Payload eines ausgestellten Tokens (Signatur geprueft, Claims nicht).
fn payload(token: &str) -> Value {
    let mut v = Validation::new(Algorithm::HS256);
    v.required_spec_claims.clear();
    v.validate_exp = false;
    jsonwebtoken::decode::<Value>(token, &DecodingKey::from_secret(GEHEIM.as_bytes()), &v)
        .unwrap()
        .claims
}

#[test]
fn status_je_fehlerart() {
    let julius = Username::neu("julius").unwrap();
    let io = || std::io::Error::other("x");
    let faelle: Vec<(AuthFehler, u16)> = vec![
        (AuthFehler::PflichtfelderFehlen(vec!["password"]), 400),
        (AuthFehler::UsernameUngueltig, 400),
        (AuthFehler::PasswortUngueltig, 400),
        (AuthFehler::Existiert(julius), 409),
        (AuthFehler::Falsch, 401),
        (AuthFehler::PasswortUeber72Bytes, 500),
        (AuthFehler::FeldKeinText("username"), 500),
        (AuthFehler::Speicher(io()), 500),
        (AuthFehler::NutzerdateiOhneUsers, 500),
        (AuthFehler::Zufall("x".into()), 500),
    ];
    for (fehler, status) in faelle {
        assert_eq!(fehler.status(), status, "{fehler}");
    }
}

#[test]
fn nutzerdatei_traegt_bcrypt_kosten_12_und_zeitstempel() {
    let dir = tempfile::tempdir().unwrap();
    let datei = dir.path().join("users.json");
    let a = Auth::neu(GEHEIM.into(), datei.clone(), None);
    let mut stempel = Vec::new();
    for name in ["julius", "anna", "berta", "carla"] {
        a.registriere(&an(name, "geheim123")).unwrap();
        let bestand: Value =
            serde_json::from_str(&std::fs::read_to_string(&datei).unwrap()).unwrap();
        let nutzer = &bestand["users"][name];
        let hash = nutzer["password_hash"].as_str().unwrap();
        assert!(hash.starts_with("$2b$12$"), "{hash}");
        assert!(bcrypt::verify("geheim123", hash).unwrap());
        stempel.push(nutzer["created_at"].as_str().unwrap().to_owned());
    }
    // `datetime.now(utc).isoformat()`: Mikrosekunden nur, wenn nicht 0 — also entweder keine
    // oder genau sechs Stellen; bei vier Eintraegen ist "alle ohne" (Chance 1e-24) ausgeschlossen.
    for s in &stempel {
        let (kopf, zone) = s.split_at(s.len() - 6);
        assert_eq!(zone, "+00:00", "{s}");
        let (sek, bruch) = kopf.split_once('.').unwrap_or((kopf, ""));
        assert_eq!(sek.len(), 19, "{s}");
        assert_eq!(sek.as_bytes()[10], b'T', "{s}");
        assert!(
            bruch.is_empty() || (bruch.len() == 6 && bruch.bytes().all(|b| b.is_ascii_digit())),
            "{s}"
        );
    }
    assert!(stempel.iter().any(|s| s.contains('.')), "{stempel:?}");
}

#[test]
fn audit_spur_je_pfad() {
    let dir = tempfile::tempdir().unwrap();
    let audit = dir.path().join("audit.jsonl");
    let a = Auth::neu(
        GEHEIM.into(),
        dir.path().join("users.json"),
        Some(audit.clone()),
    );
    let spur = || -> Vec<(String, String)> {
        lies(&audit)
            .unwrap()
            .into_iter()
            .map(|e| (e.user_id, e.action.als_str().to_owned()))
            .collect()
    };
    let eintrag = |u: &str, aktion: &str| (u.to_owned(), aktion.to_owned());

    a.registriere(&an("julius", "geheim123")).unwrap();
    assert_eq!(spur(), [eintrag("julius", "register")]);

    // Falsches Passwort und unbekannter Name: dieselbe 401, je ein `login_fehlgeschlagen`.
    let falsch = a.login(&an("julius", "falsch1234")).unwrap_err();
    let unbekannt = a.login(&an("niemand", "geheim123")).unwrap_err();
    assert_eq!(falsch.status(), 401);
    assert!(matches!(falsch, AuthFehler::Falsch), "{falsch:?}");
    assert!(matches!(unbekannt, AuthFehler::Falsch), "{unbekannt:?}");
    assert_eq!(falsch.to_string(), unbekannt.to_string());
    // Ein Name ausserhalb des Musters: ebenfalls Falsch, protokolliert unter dem Rohnamen.
    let muster = a.login(&an("a b", "geheim123")).unwrap_err();
    assert!(matches!(muster, AuthFehler::Falsch), "{muster:?}");
    assert!(matches!(a.weise_ab("[]"), AuthFehler::Falsch));
    assert_eq!(
        spur()[1..],
        [
            eintrag("julius", "login_fehlgeschlagen"),
            eintrag("niemand", "login_fehlgeschlagen"),
            eintrag("a b", "login_fehlgeschlagen"),
            eintrag("[]", "login_fehlgeschlagen"),
        ]
    );

    let n_vorher = spur().len();
    let token = a.login(&an("julius", "geheim123")).unwrap();
    assert_eq!(spur()[n_vorher..], [eintrag("julius", "login")]);

    assert_eq!(
        a.logout(&format!("Bearer {token}")).as_deref(),
        Some("julius")
    );
    assert_eq!(spur()[n_vorher + 1..], [eintrag("julius", "logout")]);

    // Ein Token ohne lesbaren `sub` meldet nichts ab und schreibt keinen Audit-Eintrag.
    let ohne_sub = fremd(&json!({"jti": "j1", "exp": jetzt() + 3600}));
    assert_eq!(a.logout(&ohne_sub), None);
    let leer_sub = fremd(&json!({"sub": "", "jti": "j2", "exp": jetzt() + 3600}));
    assert_eq!(a.logout(&leer_sub), None);
    assert_eq!(spur().len(), n_vorher + 2);
}

/// Eine Nutzerdatei von Hand mit einem schnellen Hash (Kosten 4) vom abschneidenden `bcrypt::hash`.
fn auth_mit_nutzer(dir: &std::path::Path, name: &str, passwort: &str) -> Auth {
    let hash = bcrypt::hash(passwort, 4).unwrap();
    let datei = dir.join("users.json");
    let bestand = json!({"users": {name: {"password_hash": hash, "created_at": "2026-01-01T00:00:00+00:00"}}});
    std::fs::write(&datei, bestand.to_string()).unwrap();
    Auth::neu(GEHEIM.into(), datei, None)
}

#[test]
fn passwort_grenze_72_byte_gilt_in_bytes_und_erst_fuer_vorhandene_nutzer() {
    let dir = tempfile::tempdir().unwrap();
    // 36 Zeichen zu je 2 Byte = 72 Byte; 37 Zeichen = 74 Byte, obwohl weit unter 128 Zeichen.
    let (pw72, pw73) = ("p".repeat(72), "p".repeat(73));
    let (ae36, ae37) = ("\u{e4}".repeat(36), "\u{e4}".repeat(37));

    // Einloggen mit genau 72 Byte geht (`>` 72, nicht `>=`), mit 73 Byte nicht.
    let a = auth_mit_nutzer(dir.path(), "julius", &pw72);
    assert!(a.login(&an("julius", &pw72)).is_ok());
    for zu_lang in [&pw73, &ae37] {
        let fehler = a.login(&an("julius", zu_lang)).unwrap_err();
        assert!(
            matches!(fehler, AuthFehler::PasswortUeber72Bytes),
            "{fehler:?}"
        );
        assert_eq!(fehler.status(), 500);
    }
    // Python prueft erst den Namen: fuer einen unbekannten Nutzer bleibt es bei 401.
    let fehler = a.login(&an("niemand", &pw73)).unwrap_err();
    assert!(matches!(fehler, AuthFehler::Falsch), "{fehler:?}");
    // Bytes, nicht Zeichen: 36 Zeichen sind 72 Byte.
    let a = auth_mit_nutzer(dir.path(), "anna", &ae36);
    assert!(a.login(&an("anna", &ae36)).is_ok());

    // Registrieren: 71 und 72 Byte gehen, 73 Byte werden abgewiesen (Python-bcrypt 5: `ValueError`).
    let a = Auth::neu(GEHEIM.into(), dir.path().join("neu.json"), None);
    a.registriere(&an("berta", &"p".repeat(71))).unwrap();
    a.registriere(&an("dora", &ae36)).unwrap();
    let fehler = a.registriere(&an("carla", &pw73)).unwrap_err();
    assert!(
        matches!(fehler, AuthFehler::PasswortUeber72Bytes),
        "{fehler:?}"
    );
}

/// Python-bcrypt 5.0.0 hasht ein Passwort von genau 72 Byte (`register` -> 201, `login` -> 200) und
/// meldet erst ab 73 Byte `ValueError` (500). Rust hat dort einmal mit 500 abgewiesen, weil
/// `bcrypt::non_truncating_hash` das NUL-Byte mitzaehlt; jetzt hasht `registriere` bis 72 Byte
/// abschneidend (es geht kein Passwortbyte verloren) und weist erst darueber ab.
#[test]
fn registrieren_mit_72_byte_passwort_wie_python() {
    let dir = tempfile::tempdir().unwrap();
    let datei = dir.path().join("users.json");
    let a = Auth::neu(GEHEIM.into(), datei.clone(), None);
    a.registriere(&an("julius", &"p".repeat(72))).unwrap();
    assert!(a.login(&an("julius", &"p".repeat(72))).is_ok());
    // Ein falsches Byte ganz hinten (Byte 72) wird gesehen: nichts ist abgeschnitten.
    let fast = format!("{}q", "p".repeat(71));
    assert!(matches!(
        a.login(&an("julius", &fast)).unwrap_err(),
        AuthFehler::Falsch
    ));

    // 73 Byte: Fehler, und die Datei bekommt keinen Eintrag.
    let vorher = std::fs::read_to_string(&datei).unwrap();
    let fehler = a.registriere(&an("anna", &"p".repeat(73))).unwrap_err();
    assert!(
        matches!(fehler, AuthFehler::PasswortUeber72Bytes),
        "{fehler:?}"
    );
    assert_eq!(fehler.status(), 500);
    assert_eq!(std::fs::read_to_string(&datei).unwrap(), vorher);
}

/// Hashes von Python-bcrypt 5.0.0 (`hashpw(pw, gensalt(4))`, erzeugt am 2026-10-04): 71 Byte, 72 Byte und
/// 24 Zeichen zu je 3 Byte (`U+20AC`) = 72 Byte. Der Gegenweg, Hashes von Rust durch Python pruefen
/// lassen, steht im Bericht (`berichte/auth-eingang-mutation.md`).
const PYTHON_HASHES: [(&str, &str); 3] = [
    (
        "71",
        "$2b$04$lXxqIQ9uvj7Vf1nIt1TGm.tzOfMmKqzI2OXwuTrmompdfggkAOHfq",
    ),
    (
        "72",
        "$2b$04$YKjIUfswBSM.j/6SxB/7NeQbSn2cz8YcVpBH5kykWjRwW8nyRpOka",
    ),
    (
        "24x3",
        "$2b$04$EH0Bn1raC345ybDLAqwRp.mkN/F.WHaR2wsR3nDeqB6U4scQx0xb.",
    ),
];

fn passwort_zu(fall: &str) -> String {
    match fall {
        "71" => "p".repeat(71),
        "72" => "p".repeat(72),
        _ => "\u{20ac}".repeat(24),
    }
}

#[test]
fn python_hashes_gelten_und_rust_hash_hat_die_72_byte_form() {
    let dir = tempfile::tempdir().unwrap();
    for (fall, py_hash) in PYTHON_HASHES {
        let pw = passwort_zu(fall);
        assert_eq!(pw.len(), if fall == "71" { 71 } else { 72 }, "{fall}");

        // Python -> Rust: ein Nutzer mit dem Python-Hash meldet sich an; ein anderes Passwort nicht.
        let datei = dir.path().join(format!("py-{fall}.json"));
        let bestand = json!({"users": {"julius": {"password_hash": py_hash, "created_at": "2026-01-01T00:00:00+00:00"}}});
        std::fs::write(&datei, bestand.to_string()).unwrap();
        let a = Auth::neu(GEHEIM.into(), datei, None);
        assert!(a.login(&an("julius", &pw)).is_ok(), "{fall}");
        let anders: String = std::iter::once('x').chain(pw.chars().skip(1)).collect();
        assert!(
            matches!(
                a.login(&an("julius", &anders)).unwrap_err(),
                AuthFehler::Falsch
            ),
            "{fall}"
        );

        // Rust -> Python: `registriere` schreibt einen Hash, der denselben Regeln folgt. Bei 72 Byte
        // faellt das NUL weg, ein Anhang aendert nichts (Python schneidet bei 72 Byte ab); bei 71 Byte
        // steht das NUL im Hash, ein Anhang aendert ihn.
        let datei = dir.path().join(format!("rs-{fall}.json"));
        let a = Auth::neu(GEHEIM.into(), datei.clone(), None);
        a.registriere(&an("julius", &pw)).unwrap();
        let bestand: Value =
            serde_json::from_str(&std::fs::read_to_string(&datei).unwrap()).unwrap();
        let hash = bestand["users"]["julius"]["password_hash"]
            .as_str()
            .unwrap();
        assert!(hash.starts_with("$2b$12$"), "{fall}: {hash}");
        assert!(a.login(&an("julius", &pw)).is_ok(), "{fall}");
        assert_eq!(
            bcrypt::verify(format!("{pw}Z"), hash).unwrap(),
            pw.len() == 72,
            "{fall}: Anhang hinter dem Passwort"
        );
    }
}

#[test]
fn ausgestelltes_token_hat_sub_ablauf_und_zufalls_jti() {
    let a = ohne_datei();
    let vor = jetzt();
    let t = a.stelle_aus("Julius").unwrap();
    let nach = jetzt();
    let p = payload(&t);
    assert_eq!(p["sub"], "Julius");
    let iat = p["iat"].as_i64().unwrap();
    assert!((vor..=nach).contains(&iat), "{iat} nicht in {vor}..={nach}");
    assert_eq!(JWT_TTL_S, 86_400);
    assert_eq!(p["exp"].as_i64().unwrap(), iat + 86_400);
    assert_eq!(p.as_object().unwrap().len(), 4);
    assert_eq!(a.pruefe_token(&t).as_deref(), Some("Julius"));

    // jti: 16 Zufallsbytes als 32 Kleinbuchstaben-Hex, auch mit fuehrenden Null-Nibbles; bei 60
    // Tokens trifft ein fehlendes `{:02x}` (Wahrscheinlichkeit je Token ~ 63 %) mit Sicherheit.
    let mut gesehen = std::collections::HashSet::new();
    for _ in 0..60 {
        let jti = payload(&a.stelle_aus("julius").unwrap())["jti"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(jti.len(), 32, "{jti}");
        assert!(
            jti.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "{jti}"
        );
        gesehen.insert(jti);
    }
    assert_eq!(gesehen.len(), 60, "jti wiederholt sich");
}

#[test]
fn abmelden_sperrt_genau_die_jti_dieses_tokens() {
    let a = ohne_datei();
    let t1 = a.stelle_aus("julius").unwrap();
    let t2 = a.stelle_aus("julius").unwrap();
    assert_eq!(a.pruefe_token(&t1).as_deref(), Some("julius"));
    assert_eq!(a.logout(&format!("Bearer {t1}")).as_deref(), Some("julius"));
    assert_eq!(a.pruefe_token(&t1), None);
    assert_eq!(
        a.pruefe_token(&t2).as_deref(),
        Some("julius"),
        "t2 hat eine andere jti"
    );
    // Ohne `Bearer `-Praefix geht es auch; mit einem anderen Praefix nicht.
    assert_eq!(a.logout(&t2).as_deref(), Some("julius"));
    assert_eq!(a.pruefe_token(&t2), None);
    let t3 = a.stelle_aus("julius").unwrap();
    assert_eq!(a.logout(&format!("Basic {t3}")), None);
    assert_eq!(a.pruefe_token(&t3).as_deref(), Some("julius"));
    assert_eq!(a.logout(""), None);
    assert_eq!(a.logout("Bearer "), None);
}

#[test]
fn token_ohne_jti_teilt_sich_einen_sperreintrag() {
    // PARITAET (`payload.get("jti")` ohne jti): das Abmelden eines solchen Tokens sperrt alle
    // anderen ohne jti mit.
    let a = ohne_datei();
    let t = |sub: &str| fremd(&json!({"sub": sub, "exp": jetzt() + 3600}));
    assert_eq!(a.pruefe_token(&t("anna")).as_deref(), Some("anna"));
    assert_eq!(a.logout(&t("julius")).as_deref(), Some("julius"));
    assert_eq!(a.pruefe_token(&t("julius")), None);
    assert_eq!(a.pruefe_token(&t("anna")), None);
    // Mit jti bleibt ein Token unberuehrt.
    let mit = fremd(&json!({"sub": "anna", "jti": "abc", "exp": jetzt() + 3600}));
    assert_eq!(a.pruefe_token(&mit).as_deref(), Some("anna"));
}

#[test]
fn ein_token_ohne_objekt_payload_sperrt_nichts() {
    let a = ohne_datei();
    let ohne_jti = fremd(&json!({"sub": "anna", "exp": jetzt() + 3600}));
    // `jsonwebtoken` liest nur ein Array mit genau fuenf Elementen als Claims-Struktur; erst dann
    // erreicht die Pruefung ihre eigene Objekt-Abfrage. Kuerzere Arrays und Skalare lehnt schon
    // `decode` ab.
    for kein_objekt in [
        json!([0, 0, 0, 0, 0]),
        json!([1, 2]),
        json!("text"),
        json!(5),
        json!(null),
    ] {
        assert_eq!(a.logout(&fremd(&kein_objekt)), None, "{kein_objekt}");
        assert_eq!(a.pruefe_token(&fremd(&kein_objekt)), None, "{kein_objekt}");
    }
    assert_eq!(a.pruefe_token(&ohne_jti).as_deref(), Some("anna"));
}

#[test]
fn fremde_signatur_und_fremder_algorithmus_gelten_nicht() {
    let a = ohne_datei();
    let payload = json!({"sub": "julius", "jti": "x", "exp": jetzt() + 3600});
    let anders = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &payload,
        &EncodingKey::from_secret(b"anderes-geheimnis"),
    )
    .unwrap();
    assert_eq!(a.pruefe_token(&anders), None);
    let hs512 = jsonwebtoken::encode(
        &Header::new(Algorithm::HS512),
        &payload,
        &EncodingKey::from_secret(GEHEIM.as_bytes()),
    )
    .unwrap();
    assert_eq!(a.pruefe_token(&hs512), None);
    assert_eq!(a.pruefe_token(&fremd(&payload)).as_deref(), Some("julius"));
}

/// Zeitgrenzen und Claim-Formen wie PyJWT. `gueltig` = `pruefe_token` liefert den `sub`.
fn gueltig(a: &Auth, claims: &Value) -> bool {
    let mut p = json!({"sub": "julius", "jti": "j"});
    for (k, v) in claims.as_object().unwrap() {
        p[k] = v.clone();
    }
    a.pruefe_token(&fremd(&p)).as_deref() == Some("julius")
}

#[test]
fn ablauf_und_beginn_wie_pyjwt() {
    let a = ohne_datei();
    let j = jetzt();
    // Ohne `exp` gilt ein Token (PyJWT), mit `exp` nur in der Zukunft.
    assert!(gueltig(&a, &json!({})));
    assert!(gueltig(&a, &json!({"exp": j + 3600})));
    assert!(!gueltig(&a, &json!({"exp": j - 3600})));
    assert!(!gueltig(&a, &json!({"exp": 0})));
    // Keine Toleranz: ein Token, das vor 30 s ablief, ist abgelaufen.
    assert!(!gueltig(&a, &json!({"exp": j - 30})));
    // Ein Bruchteil ueber `jetzt` im `nbf` schneidet Python mit `int()` ab: gueltig.
    assert!(gueltig(&a, &json!({"nbf": j as f64 + 0.5})));
    // `iat` und `nbf` duerfen nicht in der Zukunft liegen.
    assert!(gueltig(
        &a,
        &json!({"iat": j - 10, "nbf": j - 10, "exp": j + 3600})
    ));
    assert!(!gueltig(&a, &json!({"iat": j + 3600})));
    assert!(!gueltig(&a, &json!({"nbf": j + 3600})));
    assert!(!gueltig(&a, &json!({"iat": j + 30})));
    assert!(!gueltig(&a, &json!({"nbf": j + 30})));
    // Nicht lesbare Zeitwerte: fail-closed.
    for k in ["iat", "nbf", "exp"] {
        assert!(!gueltig(&a, &json!({ k: "morgen" })), "{k}");
        assert!(!gueltig(&a, &json!({ k: null })), "{k}");
        assert!(!gueltig(&a, &json!({ k: [1] })), "{k}");
    }
}

#[test]
fn zeitwerte_als_text_bool_und_float_wie_python_int() {
    let a = ohne_datei();
    let j = jetzt();
    let zukunft = j + 3600;
    // iat <= jetzt: Python `int(...)` darf das lesen.
    for ok in [
        json!(true),
        json!("5"),
        json!(" 5 "),
        json!("+5"),
        json!("1_0"),
        json!(5.9),
        json!(-3),
    ] {
        assert!(gueltig(&a, &json!({"iat": ok.clone()})), "iat {ok}");
    }
    for schlecht in [
        json!(""),
        json!("_5"),
        json!("5_"),
        json!("5__0"),
        json!("+"),
        json!("5x"),
        json!("1e3"),
        json!(f64::MAX),
    ] {
        assert!(
            !gueltig(&a, &json!({"iat": schlecht.clone()})),
            "iat {schlecht}"
        );
    }
    // exp: `true` ist 1 -> abgelaufen; ein Text mit Zukunftswert gilt.
    assert!(!gueltig(&a, &json!({"exp": true})));
    assert!(gueltig(&a, &json!({"exp": zukunft.to_string()})));
    assert!(gueltig(&a, &json!({"exp": format!(" +{zukunft}\n")})));
    assert!(gueltig(&a, &json!({"exp": zukunft as f64 + 0.5})));
}

#[test]
fn aud_sub_und_jti_wie_pyjwt() {
    let a = ohne_datei();
    // Ein wahres `aud` ist ein Fehler (`Invalid audience`), ein falsches nicht.
    assert!(!gueltig(&a, &json!({"aud": "x"})));
    assert!(!gueltig(&a, &json!({"aud": ["x"]})));
    assert!(!gueltig(&a, &json!({"aud": 1})));
    assert!(!gueltig(&a, &json!({"aud": true})));
    assert!(!gueltig(&a, &json!({"aud": {"a": 1}})));
    for falsch in [
        json!(""),
        json!([]),
        json!({}),
        json!(0),
        json!(0.0),
        json!(false),
        json!(null),
    ] {
        assert!(gueltig(&a, &json!({"aud": falsch.clone()})), "aud {falsch}");
    }
    // `sub` und `jti` muessen Text sein, wenn sie da sind.
    assert!(!gueltig(&a, &json!({"sub": 5})));
    assert!(!gueltig(&a, &json!({"jti": 5})));
    assert!(!gueltig(&a, &json!({"jti": null})));
    assert!(!gueltig(&a, &json!({"sub": 5, "jti": 5})));
    // Ohne `sub` gibt es keinen Nutzer, mit `sub: ""` den Namen "".
    let p = |v: Value| a.pruefe_token(&fremd(&v));
    assert_eq!(p(json!({"jti": "j"})), None);
    assert_eq!(p(json!({"sub": "", "jti": "j"})).as_deref(), Some(""));
}

#[test]
fn namens_und_passwortgrenzen_zaehlen_codepunkte_und_nur_ascii_im_namen() {
    use auth::{ist_gueltiger_username as name, ist_gueltiges_passwort as pw};
    let a = |n: usize| "a".repeat(n);
    assert!(!name(&a(2)) && name(&a(3)) && name(&a(32)) && !name(&a(33)));
    assert!(name("a-b_c9") && !name("1abc") && !name("-abc") && !name("") && !name("a b"));
    // Nur ASCII: `is_alphanumeric` liesse Umlaute und fremde Ziffern durch.
    assert!(!name("aäb") && !name("a\u{663}b") && !name("abé"));

    assert!(!pw(&a(7)) && pw(&a(8)) && pw(&a(128)) && !pw(&a(129)));
    // Codepunkte, nicht Bytes: 5 Zeichen zu 2 Byte sind zu kurz, 128 zu 2 Byte sind erlaubt.
    assert!(!pw("äääää") && pw(&"ä".repeat(128)) && !pw(&"ä".repeat(129)));
    assert!(!pw("zeile\neins1") && !pw("\nxxxxxxx") && pw("mit leerzeichen"));
}

#[test]
fn registrieren_prueft_name_und_passwort_vor_der_datei() {
    let a = ohne_datei(); // die Datei /nie/gelesen/ gibt es nicht: ein Zugriff waere ein 500
    let fehler = a.registriere(&an("julius", "kurz123")).unwrap_err();
    assert!(
        matches!(fehler, AuthFehler::PasswortUngueltig),
        "{fehler:?}"
    );
    assert_eq!(fehler.status(), 400);
    let fehler = a.registriere(&an("1x", "geheim123")).unwrap_err();
    assert!(
        matches!(fehler, AuthFehler::UsernameUngueltig),
        "{fehler:?}"
    );
    assert_eq!(fehler.status(), 400);
}

#[test]
fn aus_body_meldet_fehlende_felder_sortiert_und_prueft_zuerst_den_namen() {
    let meldung = |b: Value| Anmeldung::aus_body(&b).unwrap_err().to_string();
    assert_eq!(
        meldung(json!({})),
        "Pflichtfelder fehlen: ['password', 'username']"
    );
    assert_eq!(
        meldung(json!({"username": "julius"})),
        "Pflichtfelder fehlen: ['password']"
    );
    assert_eq!(
        meldung(json!({"password": "geheim123"})),
        "Pflichtfelder fehlen: ['username']"
    );
    let fehler = Anmeldung::aus_body(&json!({})).unwrap_err();
    assert_eq!(fehler.status(), 400);
    // Beide kein Text: Python scheitert zuerst am Namen.
    let fehler = Anmeldung::aus_body(&json!({"username": 1, "password": 2})).unwrap_err();
    assert!(
        matches!(fehler, AuthFehler::FeldKeinText("username")),
        "{fehler:?}"
    );
    let fehler = Anmeldung::aus_body(&json!({"username": "julius", "password": 2})).unwrap_err();
    assert!(
        matches!(fehler, AuthFehler::FeldKeinText("password")),
        "{fehler:?}"
    );
    assert_eq!(fehler.status(), 500);
    let ok = Anmeldung::aus_body(&json!({"username": "julius", "password": "geheim123"})).unwrap();
    assert_eq!(
        (ok.username.as_str(), ok.password.as_str()),
        ("julius", "geheim123")
    );
}

#[test]
fn nutzerdatei_wird_angelegt_atomar_ersetzt_und_bleibt_privat() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    // Das Verzeichnis gibt es noch nicht.
    let datei = dir.path().join("neu/unter/users.json");
    let tmp = dir.path().join("neu/unter/users.json.tmp");
    let a = Auth::neu(GEHEIM.into(), datei.clone(), None);
    a.registriere(&an("julius", "geheim123")).unwrap();
    assert!(datei.is_file());
    assert!(!tmp.exists(), "die tmp-Datei wird umbenannt, nicht kopiert");
    // Eine liegen gebliebene tmp-Datei (Absturz, fremde Rechte) blockiert nichts.
    std::fs::write(&tmp, "kaputt").unwrap();
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644)).unwrap();
    a.registriere(&an("anna", "geheim123")).unwrap();
    assert!(!tmp.exists());
    assert_eq!(
        std::fs::metadata(&datei).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let bestand: Value = serde_json::from_str(&std::fs::read_to_string(&datei).unwrap()).unwrap();
    let namen: Vec<&String> = bestand["users"].as_object().unwrap().keys().collect();
    assert_eq!(namen, ["anna", "julius"]);
}

#[test]
fn nutzerdatei_ohne_users_objekt_oder_kein_json_ist_ein_500() {
    let dir = tempfile::tempdir().unwrap();
    let datei = dir.path().join("users.json");
    let a = Auth::neu(GEHEIM.into(), datei.clone(), None);
    // Fehlt die Datei, ist der Bestand leer: unbekannter Name, 401.
    assert!(matches!(
        a.login(&an("julius", "geheim123")).unwrap_err(),
        AuthFehler::Falsch
    ));
    for inhalt in [
        r#"{"users": 5}"#,
        r#"{"users": null}"#,
        r#"{"users": []}"#,
        r#"{"users": "x"}"#,
        "{}",
        "[]",
    ] {
        std::fs::write(&datei, inhalt).unwrap();
        for fehler in [
            a.login(&an("julius", "geheim123")).unwrap_err(),
            a.registriere(&an("julius", "geheim123")).unwrap_err(),
        ] {
            assert!(
                matches!(fehler, AuthFehler::NutzerdateiOhneUsers),
                "{inhalt}: {fehler:?}"
            );
            assert_eq!(fehler.status(), 500);
        }
    }
    std::fs::write(&datei, "{").unwrap();
    let fehler = a.login(&an("julius", "geheim123")).unwrap_err();
    assert!(
        matches!(fehler, AuthFehler::NutzerdateiKaputt(_)),
        "{fehler:?}"
    );
    assert_eq!(fehler.status(), 500);
}

/// Nur eine fehlende Nutzerdatei ist ein leerer Bestand; jeder andere Lesefehler ist ein 500. Sonst
/// meldete sich ein Nutzer als "unbekannt" ab (401), obwohl die Datei nur unlesbar ist, und
/// `registriere` legte einen Bestand mit einem einzigen Nutzer an.
#[test]
fn eine_unlesbare_nutzerdatei_ist_ein_speicherfehler_und_kein_leerer_bestand() {
    let dir = tempfile::tempdir().unwrap();
    // Der Pfad der Nutzerdatei ist ein Verzeichnis: `read_to_string` scheitert mit `IsADirectory`.
    let a = Auth::neu(GEHEIM.into(), dir.path().to_path_buf(), None);
    let fehler = a.login(&an("julius", "geheim123")).unwrap_err();
    assert!(matches!(fehler, AuthFehler::Speicher(_)), "{fehler:?}");
    assert_eq!(fehler.status(), 500);
    let fehler = a.registriere(&an("julius", "geheim123")).unwrap_err();
    assert!(matches!(fehler, AuthFehler::Speicher(_)), "{fehler:?}");
    assert_eq!(fehler.status(), 500);
}

/// Das Audit ist ein Nebenkanal: scheitert das Anhaengen, laufen Registrieren, Anmelden, Abweisen und
/// Abmelden trotzdem durch. Python hat hier keinen `try` und bricht ab (bewusste Abweichung, siehe
/// `Auth::protokolliere`).
#[test]
fn ein_audit_fehler_kippt_keine_anmeldung() {
    let dir = tempfile::tempdir().unwrap();
    // Der Audit-Pfad ist ein Verzeichnis: jedes Anhaengen scheitert.
    let audit = dir.path().join("audit");
    std::fs::create_dir(&audit).unwrap();
    let a = Auth::neu(
        GEHEIM.into(),
        dir.path().join("users.json"),
        Some(audit.clone()),
    );
    a.registriere(&an("julius", "geheim123")).unwrap();
    let token = a.login(&an("julius", "geheim123")).unwrap();
    assert_eq!(a.pruefe_token(&token).as_deref(), Some("julius"));
    assert!(matches!(
        a.login(&an("julius", "falsch1234")).unwrap_err(),
        AuthFehler::Falsch
    ));
    assert!(matches!(a.weise_ab("a b"), AuthFehler::Falsch));
    assert_eq!(a.logout(&token).as_deref(), Some("julius"));
    // Gegenprobe: das Verzeichnis ist unberuehrt, es wurde nichts geschrieben.
    assert_eq!(std::fs::read_dir(&audit).unwrap().count(), 0);
}

/// PyJWT weist ein `sub`, das kein Text ist, mit `InvalidSubjectError` ab. Ein solches Token meldet
/// keinen Nutzer und sperrt beim Abmelden nichts: liesse die Pruefung es durch, truege `logout` seine
/// `jti` trotzdem in die Sperrliste ein, und ein echtes Token mit derselben `jti` waere gesperrt.
#[test]
fn ein_token_mit_nicht_text_sub_sperrt_beim_abmelden_nichts() {
    let a = ohne_datei();
    let echt = fremd(&json!({"sub": "anna", "jti": "geteilt", "exp": jetzt() + 3600}));
    for kein_text in [json!(5), json!(null), json!(["anna"]), json!(true)] {
        let t = fremd(&json!({"sub": kein_text, "jti": "geteilt", "exp": jetzt() + 3600}));
        assert_eq!(a.pruefe_token(&t), None, "{kein_text}");
        assert_eq!(a.logout(&t), None, "{kein_text}");
        assert_eq!(
            a.pruefe_token(&echt).as_deref(),
            Some("anna"),
            "{kein_text}"
        );
    }
}
