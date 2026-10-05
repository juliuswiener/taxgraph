//! Die Ueberlaufpruefung (`overflow-checks`) bleibt in dem Bau an, den der Dienst benutzt.
//!
//! Warum: Ohne die Pruefung bricht jedes ungeprueft geschriebene `+`/`-` der Geldrechnung still
//! um und rechnet falsch weiter, statt zu paniken oder einen Fehler zu melden. `make serve` baut
//! mit `cargo build` das dev-Profil (das Makefile verbietet `--release`), und `rust/parity` und
//! `make ui-rust` messen nur diesen Bau. Cargo schaltet die Pruefung fuer dev von selbst ein;
//! eine Zeile `= false` oder ein anderes Profil in `make serve` schaltet sie aus, ohne dass ein
//! anderer Test es merkt. Ersetzt `tests/test_ueberlauf_waechter_text.py` (Waechter 2,
//! Python-Test, entfaellt mit Python); der Shim-Waechter steht in
//! `catala-sys/tests/shim_ueberlauf_waechter.rs`.
//!
//! Zwei Arten von Pruefung, absichtlich beide:
//! - ein Laufzeit-Beleg (`die_ueberlaufpruefung_ist_im_testbau_wirklich_an`): das Testprofil erbt
//!   von dev; ein `overflow-checks = false` dort faellt hier auf, wo immer es steht;
//! - Text-Waechter ueber `rust/Cargo.toml` und das Makefile: der Beleg sieht nicht, dass die Zeile
//!   fehlt (der Cargo-Standard greift dann) und nicht, welches Profil `make serve` baut.
//!
//! Die Text-Waechter sind reine Funktionen; `die_text_waechter_schlagen_an` fuettert sie mit
//! Fassungen, in denen genau der Rueckfall steckt.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `rust/Cargo.toml`: `overflow-checks = true` steht ausdruecklich in `[profile.dev]`, und keine
/// Stelle der Datei setzt die Pruefung auf etwas anderes als `true`. Zeilenweise gelesen, ohne
/// TOML-Bibliothek: Abschnittskopf `[...]`, Schluessel `overflow-checks`, auch in Punktform
/// (`profile.dev.overflow-checks`) oder als Inline-Tabelle erkannt.
fn cargo_probleme(text: &str) -> Vec<String> {
    let mut p = Vec::new();
    let mut abschnitt = String::new();
    let mut dev_an = false;
    for roh in text.lines() {
        let zeile: String = roh
            .split('#')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .collect();
        if let Some(kopf) = zeile.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            kopf.clone_into(&mut abschnitt);
            continue;
        }
        let Some(i) = zeile.find("overflow-checks") else {
            continue;
        };
        let rest = zeile.get(i + "overflow-checks".len()..).unwrap_or("");
        let wert = rest
            .strip_prefix('=')
            .map(|w| w.trim_end_matches(['}', ',']));
        if wert == Some("true") {
            // Wirksamer Pfad: Abschnitt plus der Punktpraefix des Schluessels
            // (`dev.`, `profile.dev.`, `dev={`), z. B. `profile.dev`.
            let praefix = zeile
                .get(..i)
                .unwrap_or("")
                .trim_end_matches(['.', '=', '{']);
            let pfad = [abschnitt.as_str(), praefix]
                .iter()
                .filter(|s| !s.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(".");
            if pfad == "profile.dev" {
                dev_an = true;
            }
        } else {
            p.push(format!(
                "[{abschnitt}] setzt overflow-checks auf etwas anderes als true: {zeile}"
            ));
        }
    }
    if !dev_an {
        p.push("[profile.dev] enthaelt kein ausdrueckliches `overflow-checks = true`".into());
    }
    p
}

/// Rezept des Makefile-Ziels `serve`: die Zeilen mit Tabulator darunter, nach Zeilenverkettung.
fn serve_rezept(makefile: &str) -> Vec<String> {
    let text = makefile.replace("\\\n", " ");
    let mut zeilen = text.lines().skip_while(|z| {
        let Some(rest) = z.strip_prefix("serve") else {
            return true;
        };
        let rest = rest.trim_start();
        !rest.starts_with(':') || rest.starts_with(":=")
    });
    zeilen.next();
    let mut rezept = Vec::new();
    for z in zeilen {
        if z.starts_with('\t') {
            rezept.push(z.trim().to_owned());
        } else if !(z.trim().is_empty() || z.trim_start().starts_with('#')) {
            break;
        }
    }
    rezept
}

/// Makefile: `serve` baut mit `cargo build` in `rust/` das dev-Profil und startet dessen Binary;
/// keine Zeile greift in die Ueberlaufpruefung oder das Profil ein.
fn make_probleme(makefile: &str) -> Vec<String> {
    let mut p = Vec::new();
    let rezept = serve_rezept(makefile);
    if rezept.is_empty() {
        return vec![
            "im Makefile fehlt das Ziel `serve`: das gebaute Profil ist nicht lesbar".into(),
        ];
    }
    let cargo: Vec<&String> = rezept.iter().filter(|z| z.contains("cargo")).collect();
    if !cargo.iter().any(|z| z.contains("cargo build")) {
        p.push("`serve` baut nicht mit `cargo build`: das Profil ist nicht lesbar".into());
    }
    for z in &cargo {
        for (muster, name) in [
            ("--release", "--release"),
            ("--profile", "--profile"),
            (" -r ", "-r"),
            ("CARGO_PROFILE", "CARGO_PROFILE_*"),
        ] {
            if format!("{z} ").contains(muster) {
                p.push(format!(
                    "`serve` waehlt mit {name} ein anderes Profil als dev: {z}"
                ));
            }
        }
    }
    if !cargo.iter().any(|z| z.contains("cd rust")) {
        p.push("`serve` baut nicht in `rust/`: dort liegt das Cargo.toml dieses Waechters".into());
    }
    if !rezept
        .iter()
        .any(|z| z.contains("$(SERVE_TARGET)/debug/taxgraph-api"))
    {
        p.push(
            "`serve` startet nicht `$(SERVE_TARGET)/debug/taxgraph-api` (das dev-Profil)".into(),
        );
    }
    for z in makefile
        .lines()
        .filter(|z| !z.trim_start().starts_with('#'))
    {
        let klein = z.to_lowercase();
        if klein.contains("overflow")
            || klein.contains("rustflags")
            || klein.contains("cargo_profile")
        {
            p.push(format!(
                "das Makefile greift in die Ueberlaufpruefung oder das Profil ein: {}",
                z.trim()
            ));
        }
    }
    p
}

/// Eine Cargo-Konfiguration (`.cargo/config`), die `overflow` nennt, kann die Pruefung ausschalten.
fn konfig_probleme() -> Vec<String> {
    [
        ".cargo/config.toml",
        ".cargo/config",
        "rust/.cargo/config.toml",
        "rust/.cargo/config",
    ]
    .iter()
    .filter_map(|rel| {
        let text = std::fs::read_to_string(wurzel().join(rel)).ok()?;
        text.to_lowercase().contains("overflow").then(|| {
            format!("{rel} nennt overflow: eine Cargo-Konfiguration kann die Pruefung ausschalten")
        })
    })
    .collect()
}

#[test]
fn cargo_toml_haelt_overflow_checks_im_dev_profil_ausdruecklich_an() {
    let text = std::fs::read_to_string(wurzel().join("rust/Cargo.toml")).unwrap();
    assert_eq!(cargo_probleme(&text), Vec::<String>::new());
}

#[test]
fn make_serve_baut_das_dev_profil_und_keine_konfiguration_schaltet_die_pruefung_aus() {
    let text = std::fs::read_to_string(wurzel().join("Makefile")).unwrap();
    assert_eq!(make_probleme(&text), Vec::<String>::new());
    assert_eq!(konfig_probleme(), Vec::<String>::new());
}

/// Laufzeit-Beleg: In diesem Testbau panikt `i64::MAX + 1`. Das Testprofil erbt von dev; steht
/// dort (oder in `[profile.test]`, per Umgebung, per `RUSTFLAGS`) `overflow-checks` aus, rechnet
/// die Addition still um und der Test wird rot. `black_box` haelt den Compiler davon ab, die
/// Rechnung schon beim Uebersetzen zu sehen.
#[test]
fn die_ueberlaufpruefung_ist_im_testbau_wirklich_an() {
    let r = std::panic::catch_unwind(|| std::hint::black_box(i64::MAX) + std::hint::black_box(1));
    assert!(
        r.is_err(),
        "i64::MAX + 1 rechnet still um: overflow-checks ist in diesem Bau aus (rust/Cargo.toml, RUSTFLAGS, \
         CARGO_PROFILE_*). Die Geldrechnung wuerde falsch weiterrechnen statt zu melden."
    );
}

const CARGO_GUT: &str = "[workspace]\nmembers = []\n\n[profile.dev]\nopt-level = 1\n# overflow-checks = false\noverflow-checks = true\n";
const MAKE_GUT: &str = "serve: $(SICHERN_ZIEL)\n\tcd rust && CARGO_TARGET_DIR=$(SERVE_TARGET) cargo build -p api --bin taxgraph-api\n\t@echo \"Dienst\"\n\texec env $(SERVE_TARGET)/debug/taxgraph-api $(SERVE_PORT)\n\nserve-python: x\n\tcargo build --release\n";

#[test]
fn die_text_waechter_schlagen_an() {
    // sauber, auch mit dem Namen im Kommentar und anderer Schreibweise
    assert!(cargo_probleme(CARGO_GUT).is_empty());
    assert!(cargo_probleme("[profile.dev]\noverflow-checks=true # an\n").is_empty());
    assert!(cargo_probleme("[profile]\ndev.overflow-checks = true\n").is_empty());
    assert!(make_probleme(MAKE_GUT).is_empty());
    // das Ziel `serve-python` darf `--release` tragen: nur `serve` zaehlt
    assert!(
        make_probleme(&MAKE_GUT.replace("serve-python: x\n\tcargo build --release\n", ""))
            .is_empty()
    );

    let cargo_schlecht = [
        (
            "Zeile fehlt",
            CARGO_GUT.replace("overflow-checks = true\n", ""),
        ),
        (
            "false",
            CARGO_GUT.replace("overflow-checks = true", "overflow-checks = false"),
        ),
        (
            "falsches Profil",
            CARGO_GUT.replace("[profile.dev]", "[profile.release]"),
        ),
        (
            "Paket-Override",
            format!("{CARGO_GUT}\n[profile.dev.package.\"*\"]\noverflow-checks = false\n"),
        ),
        (
            "Build-Override",
            format!("{CARGO_GUT}\n[profile.dev.build-override]\noverflow-checks = false\n"),
        ),
    ];
    for (name, text) in cargo_schlecht {
        assert!(!cargo_probleme(&text).is_empty(), "Cargo-Waechter: {name}");
    }

    let make_schlecht = [
        (
            "--release",
            MAKE_GUT.replace("cargo build -p api", "cargo build --release -p api"),
        ),
        (
            "--profile",
            MAKE_GUT.replace("cargo build -p api", "cargo build --profile fast -p api"),
        ),
        (
            "-r",
            MAKE_GUT.replace("cargo build -p api", "cargo build -r -p api"),
        ),
        (
            "CARGO_PROFILE",
            MAKE_GUT.replace(
                "cd rust &&",
                "cd rust && CARGO_PROFILE_DEV_OVERFLOW_CHECKS=false",
            ),
        ),
        (
            "Startet release",
            MAKE_GUT.replace("/debug/taxgraph-api", "/release/taxgraph-api"),
        ),
        ("baut nicht in rust/", MAKE_GUT.replace("cd rust && ", "")),
        (
            "RUSTFLAGS",
            format!("RUSTFLAGS=-Coverflow-checks=off\n{MAKE_GUT}"),
        ),
        ("Ziel fehlt", "all:\n\techo\n".to_owned()),
    ];
    for (name, text) in make_schlecht {
        assert!(
            !make_probleme(&text).is_empty(),
            "Makefile-Waechter: {name}"
        );
    }
}
