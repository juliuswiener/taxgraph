//! Dateioperationen des Stores am Aufrufort (N4, Mutationsmessung `rust/store`, Teil D):
//! `lade`/`speichere` (Persistenz), `audit::anhaengen` und `fehler_log`. Jeder Test steht fuer
//! Mutanten, die die Tests der Crate und `-p api` ueberlebt haben.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

use store::audit::{AuditAktion, AuditFehler};
use store::fehler_log::{FallId, FehlerLogFehler, Meta, Stufe};
use store::{PersistenzFehler, Sperrform, Store};

fn verzeichnis(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("taxgraph-n4-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Schreibt `text` als Akte und laedt sie; nur die Sperrform interessiert.
fn sperrform(name: &str, text: &str) -> Option<Sperrform> {
    let dir = verzeichnis(name);
    let pfad = dir.join("fall.json");
    std::fs::write(&pfad, text).unwrap();
    let ergebnis = store::lade(&pfad);
    std::fs::remove_dir_all(&dir).ok();
    match ergebnis {
        Err(PersistenzFehler::Sperrform { form, .. }) => Some(form),
        _ => None,
    }
}

/// `json.load` liest jede dieser Zahlformen still als Zahl (`NaN`, `inf`); `lade` sperrt sie unter
/// `events` mit ihrem Namen. Die Tabelle fuehrt jede Schreibweise einzeln: Leerraum vor dem
/// Doppelpunkt des Schluessels, Vorzeichen im Exponenten, grosses `E`, Dezimalpunkt ohne
/// Exponent, Exponent mit Minus ueber einer sehr langen Mantisse. Ein einzelnes `-` ist keine
/// Zahl und kein Ueberlauf.
#[test]
fn sperrformen_unter_events_werden_in_jeder_schreibweise_erkannt() {
    let null400 = "0".repeat(400);
    let null600 = "0".repeat(600);
    let faelle: Vec<(String, Option<Sperrform>)> = vec![
        (r#"{"events" : [NaN]}"#.to_owned(), Some(Sperrform::NaN)),
        (
            r#"{"events":[Infinity]}"#.to_owned(),
            Some(Sperrform::Infinity),
        ),
        (
            r#"{"events":[-Infinity]}"#.to_owned(),
            Some(Sperrform::Infinity),
        ),
        (
            r#"{"events":[1e+400]}"#.to_owned(),
            Some(Sperrform::KommazahlUeberlauf),
        ),
        (
            r#"{"events":[1E400]}"#.to_owned(),
            Some(Sperrform::KommazahlUeberlauf),
        ),
        (
            format!(r#"{{"events":[1{null400}.0]}}"#),
            Some(Sperrform::KommazahlUeberlauf),
        ),
        (
            format!(r#"{{"events":[1{null600}e-100]}}"#),
            Some(Sperrform::KommazahlUeberlauf),
        ),
        (
            format!(r#"{{"events":[1{null400}]}}"#),
            Some(Sperrform::GanzzahlUeberlauf),
        ),
        (r#"{"events":[-]}"#.to_owned(), None),
        (r#"{"events":[1e5, 1.5, 1E-5, 1e+5]}"#.to_owned(), None),
    ];
    for (i, (text, soll)) in faelle.iter().enumerate() {
        let kurz: String = text.chars().take(40).collect();
        assert_eq!(sperrform(&format!("sperrform-{i}"), text), *soll, "{kurz}");
    }
}

/// Das Tempfile von `speichere` heisst `.<name>.<pid>.tmp` im Verzeichnis der Akte.
fn temp_name(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!(".{name}.{}.tmp", std::process::id()))
}

/// Ein Tempfile aus einem frueheren, abgebrochenen Lauf (gleiche Prozesskennung) ist laenger als
/// die neue Akte. `speichere` kuerzt es beim Oeffnen, schreibt, benennt es um und laesst keines
/// zurueck: danach steht genau die neue Akte da, ohne Rest dahinter, und sonst nichts im
/// Verzeichnis.
#[test]
fn speichere_kuerzt_ein_uebrig_gebliebenes_tempfile_und_benennt_es_um() {
    let dir = verzeichnis("speichere-rest");
    let pfad = dir.join("fall.json");
    let tmp = temp_name(&dir, "fall.json");
    std::fs::write(&tmp, "x".repeat(100_000)).unwrap();
    let datei = Store::leer(2025, Some("demo".to_owned())).into_datei();
    store::speichere(&pfad, &datei).unwrap();
    assert_eq!(
        std::fs::read_to_string(&pfad).unwrap(),
        serde_json::to_string(&datei).unwrap()
    );
    assert_eq!(store::lade(&pfad).unwrap().fall_id.as_deref(), Some("demo"));
    assert!(!tmp.exists(), "das Tempfile blieb liegen");
    let namen: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(namen, ["fall.json"]);
    std::fs::remove_dir_all(&dir).ok();
}

/// Eine fehlende Akte meldet den Pfad und den Grund des Betriebssystems, nicht irgendeinen Pfad.
#[test]
fn lade_nennt_den_pfad_und_den_grund_wenn_die_datei_fehlt() {
    let dir = verzeichnis("lade-fehlt");
    let pfad = dir.join("fehlt.json");
    let fehler = store::lade(&pfad).unwrap_err();
    assert!(
        matches!(&fehler, PersistenzFehler::Lesen(p, _) if *p == pfad),
        "{fehler:?}"
    );
    assert!(
        fehler.to_string().starts_with(&format!(
            "store-datei {} konnte nicht gelesen werden: ",
            pfad.display()
        )),
        "{fehler}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Jede Audit-Aktion steht im Log als blanker String, wie ihn `audit.append` in Python schreibt
/// (`"action": action`): ein Name je Variante, rund durch `from`, `als_str` und JSON; ein
/// unbekannter Name bleibt `Andere` mit seinem Text.
#[test]
fn audit_aktionen_tragen_ihre_wire_form() {
    let alle = [
        (AuditAktion::Login, "login"),
        (AuditAktion::Logout, "logout"),
        (AuditAktion::LoginFehlgeschlagen, "login_fehlgeschlagen"),
        (AuditAktion::Register, "register"),
        (AuditAktion::FallAngelegt, "fall_angelegt"),
        (AuditAktion::FallCreate, "fall_create"),
        (AuditAktion::FallGeloescht, "fall_geloescht"),
        (AuditAktion::FallValidiert, "fall_validiert"),
        (AuditAktion::ZugriffVerweigert, "zugriff_verweigert"),
        (AuditAktion::LlmCall, "llm_call"),
        (AuditAktion::Andere("fall_export".to_owned()), "fall_export"),
    ];
    for (aktion, name) in alle {
        assert_eq!(aktion.als_str(), name);
        assert_eq!(AuditAktion::from(name), aktion, "{name}");
        let json = serde_json::to_string(&aktion).unwrap();
        assert_eq!(json, format!("\"{name}\""));
        assert_eq!(serde_json::from_str::<AuditAktion>(&json).unwrap(), aktion);
    }
}

/// Das Audit-Log liest zeilenweise: leere und nur aus Leerraum bestehende Zeilen zaehlen nicht
/// (Python `if line.strip()`), eine Zeile, die kein JSON ist, ist ein Fehler und wird nicht
/// uebergangen.
#[test]
fn audit_lies_ueberspringt_leere_zeilen_und_meldet_kaputte() {
    let dir = verzeichnis("audit-lies");
    let pfad = dir.join("audit.jsonl");
    store::audit::anhaengen(&pfad, Some("julius"), AuditAktion::Login, None, None).unwrap();
    store::audit::anhaengen(&pfad, Some("julius"), AuditAktion::Logout, None, None).unwrap();
    let text = std::fs::read_to_string(&pfad).unwrap();
    let (erste, zweite) = text.trim_end().split_once('\n').unwrap();
    std::fs::write(&pfad, format!("{erste}\n\n   \n\t\n{zweite}\n")).unwrap();
    let eintraege = store::audit::lies(&pfad).unwrap();
    let aktionen: Vec<_> = eintraege.iter().map(|e| e.action.clone()).collect();
    assert_eq!(aktionen, [AuditAktion::Login, AuditAktion::Logout]);
    std::fs::write(&pfad, format!("{erste}\nkein json\n{zweite}\n")).unwrap();
    let fehler = store::audit::lies(&pfad).unwrap_err();
    assert!(matches!(fehler, AuditFehler::Format(_)), "{fehler:?}");
    std::fs::remove_dir_all(&dir).ok();
}

/// Dasselbe fuer das Fehler-Log.
#[test]
fn fehler_log_lies_ueberspringt_leere_zeilen_und_meldet_kaputte() {
    let dir = verzeichnis("fehler-lies");
    let pfad = dir.join("fehler.jsonl");
    for stufe in [Stufe::Fehler, Stufe::Warnung] {
        store::fehler_log::protokolliere(
            &pfad,
            "test.ort",
            &std::fmt::Error,
            stufe,
            None,
            Meta::default(),
        )
        .unwrap();
    }
    let text = std::fs::read_to_string(&pfad).unwrap();
    let (erste, zweite) = text.trim_end().split_once('\n').unwrap();
    std::fs::write(&pfad, format!("{erste}\n\n   \n\t\n{zweite}\n")).unwrap();
    let stufen: Vec<_> = store::fehler_log::lies(&pfad)
        .unwrap()
        .iter()
        .map(|e| e.stufe)
        .collect();
    assert_eq!(stufen, [Stufe::Fehler, Stufe::Warnung]);
    std::fs::write(&pfad, format!("{erste}\nkein json\n{zweite}\n")).unwrap();
    let fehler = store::fehler_log::lies(&pfad).unwrap_err();
    assert!(matches!(fehler, FehlerLogFehler::Format(_)), "{fehler:?}");
    std::fs::remove_dir_all(&dir).ok();
}

/// `fehler_log._sicherer_fall_id` (Python): die Form `^[A-Za-z0-9_-]{1,64}$`, dann die PII-Muster
/// VERANKERT (`fullmatch`): nur eine Kennung, die als GANZES ein Muster ist, wird gesperrt; ein
/// Treffer am Anfang oder am Ende genuegt nicht. Der Text der Sperre ist hier `<gesperrt:pii>`
/// (Python nennt die Kategorien, `PARITAET` Punkt 3 der Moduldoku).
#[test]
fn fall_id_form_und_pii_wie_python() {
    let steuer_id = regex::Regex::new(r"\d{11}").unwrap();
    let iban = regex::Regex::new(r"[A-Z]{2}\d{20}").unwrap();
    let muster = [iban, steuer_id];
    let lang = "a".repeat(64);
    let zu_lang = "a".repeat(65);
    for (kennung, soll) in [
        (lang.as_str(), lang.as_str()),
        (zu_lang.as_str(), "<gesperrt:form>"),
        ("fall_1", "fall_1"),
        ("fall-1", "fall-1"),
        ("fall 1", "<gesperrt:form>"),
        ("", "<gesperrt:form>"),
        ("demo-1", "demo-1"),
        ("12345678901", "<gesperrt:pii>"),
        ("DE12345678901234567890", "<gesperrt:pii>"),
        ("12345678901x", "12345678901x"),
        ("x12345678901", "x12345678901"),
        ("x12345678901y", "x12345678901y"),
    ] {
        assert_eq!(
            FallId::pruefe(kennung, &muster).als_str(),
            soll,
            "{kennung}"
        );
    }
}

/// Ein Fehler-Eintrag traegt Stufe, Ort, Typname des Fehlers, die geprueft Fall-Kennung, die
/// vier erlaubten Zahlen und die Aufrufstelle als `datei:zeile` (nicht Spalte). Der Inhalt des
/// Fehlers steht nie darin.
#[test]
fn fehler_eintrag_traegt_alle_felder_und_die_zeile_des_aufrufs() {
    let dir = verzeichnis("fehler-felder");
    let pfad = dir.join("fehler.jsonl");
    let meta = Meta {
        anzahl: Some(3),
        laenge: Some(4),
        versuche: Some(2),
        geglueckt: Some(true),
    };
    let fall_id = FallId::pruefe("demo-1", &[regex::Regex::new(r"\d{11}").unwrap()]);
    let fehler = std::io::Error::other("IBAN DE89370400440532013000");
    let aufruf = line!() + 1;
    store::fehler_log::protokolliere(
        &pfad,
        "test.ort",
        &fehler,
        Stufe::Warnung,
        Some(fall_id),
        meta,
    )
    .unwrap();
    let e = &store::fehler_log::lies(&pfad).unwrap()[0];
    assert_eq!(e.stufe, Stufe::Warnung);
    assert_eq!(e.ort, "test.ort");
    assert_eq!(e.typ, std::any::type_name::<std::io::Error>());
    assert_eq!(e.fall_id.as_ref().map(FallId::als_str), Some("demo-1"));
    assert_eq!(e.meta, meta);
    assert_eq!(e.quelle, format!("{}:{aufruf}", file!()));
    assert!(!std::fs::read_to_string(&pfad).unwrap().contains("DE89"));
    std::fs::remove_dir_all(&dir).ok();
}
