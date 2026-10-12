//! Waechter der `.gitignore`: auch ein VERWEIS (Symlink) auf ein Build-Verzeichnis wird ignoriert, nicht nur das Verzeichnis.
//! Ersatz von `test_gitignore_nimmt_verweise_auf_build_verzeichnisse_aus` aus `tests/test_ci_konfiguration.py`
//! (Vault: Entscheidung `ignorier-regeln-der-build-verzeichnisse-gelten-auch-fuer-verweise`).
//!
//! Warum es zaehlt: Ein Arbeitsbaum bekommt statt eines Build-Verzeichnisses einen Verweis auf den Hauptbaum. Eine Regel mit `/` am
//! Ende (`_build/`) gilt nur fuer echte Verzeichnisse; der Verweis waere fuer Git eine neue Datei, und `git add -A` (die
//! Auto-Speicherung von orch) nahme ihn mit. Ein solcher Merge ersetzte still die Rechen-Engine des Hauptbaums durch einen Verweis
//! (gemessen 2026-10-03: drei Zweige trugen einen).
//!
//! Wie: je Name aus `BUILD_NAMEN` zwei Faelle in einem Wegwerf-Repo im Temp-Verzeichnis mit der echten `.gitignore` des Baums: ein
//! Verweis (der Fall, der fehlte) und ein echtes Verzeichnis (muss ignoriert bleiben). `git check-ignore -q`: Exit 0 ignoriert,
//! 1 nicht ignoriert, alles andere ist ein Git-Fehler und bricht laut ab. Die Git-Umgebung des Aufrufers (`GIT_DIR`, globale und
//! XDG-Konfiguration, `core.excludesFile`) ist ausgeschaltet. `die_pruefung_schlaegt_an` zeigt an Fassungen der `.gitignore`, dass
//! sie rot werden kann (Regeln mit `/` am Ende, eine fehlende Regel, eine leere Datei).
//!
//! Grenzen: Die Liste der Namen steht hier; ein neues Build-Verzeichnis in der `.gitignore` ohne Eintrag hier ist ungeschuetzt.
//! Der Test braucht `git` und Symlinks (Linux, macOS); auf dem CI-Runner `ubuntu-latest` ist beides da.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BUILD_NAMEN: [&str; 6] = [
    "_build",
    "_target",
    "_targets",
    "oracle/.venv",
    "oracle/.venv312",
    "oracle/gettsim/_catala",
];

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

static ZAEHLER: AtomicUsize = AtomicUsize::new(0);

/// Ein frisches Verzeichnis im Temp-Ordner (je Prozess und Aufruf eigener Name).
fn frisches_verzeichnis() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "taxgraph-gitignore-verweise-{}-{}",
        std::process::id(),
        ZAEHLER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn git(basis: &Path, repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", basis)
        .env("XDG_CONFIG_HOME", basis)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"))
}

/// Die Pfade, die Git bei dieser `.gitignore` NICHT ignoriert: je Name ein Verweis auf ein Verzeichnis (`verweis`) oder ein
/// Verzeichnis mit einer Datei darin.
fn nicht_ignoriert(gitignore: &str, verweis: bool) -> Vec<String> {
    let basis = frisches_verzeichnis();
    let (ziel, repo) = (basis.join("ziel"), basis.join("repo"));
    std::fs::create_dir_all(&ziel).unwrap();
    std::fs::create_dir_all(&repo).unwrap();
    let init = git(&basis, &repo, &["init", "-q", "."]);
    assert!(init.status.success(), "git init: {}", String::from_utf8_lossy(&init.stderr));
    std::fs::write(repo.join(".gitignore"), gitignore).unwrap();
    let mut offen = Vec::new();
    for name in BUILD_NAMEN {
        std::fs::create_dir_all(repo.join(name).parent().unwrap()).unwrap();
        let pfad = if verweis {
            std::os::unix::fs::symlink(&ziel, repo.join(name)).unwrap();
            name.to_string()
        } else {
            std::fs::create_dir_all(repo.join(name)).unwrap();
            std::fs::write(repo.join(name).join("datei.txt"), "x").unwrap();
            format!("{name}/datei.txt")
        };
        let aus = git(&basis, &repo, &["check-ignore", "-q", &pfad]);
        match aus.status.code() {
            Some(0) => {}
            Some(1) => offen.push(pfad),
            c => panic!("git check-ignore {pfad}: Exit {c:?}: {}", String::from_utf8_lossy(&aus.stderr)),
        }
    }
    let _ = std::fs::remove_dir_all(&basis);
    offen
}

#[test]
fn die_gitignore_nimmt_verweise_auf_build_verzeichnisse_aus() {
    let gitignore = std::fs::read_to_string(wurzel().join(".gitignore")).unwrap();
    let verweis = nicht_ignoriert(&gitignore, true);
    let verzeichnis = nicht_ignoriert(&gitignore, false);
    assert!(
        verweis.is_empty(),
        "Git ignoriert diese Build-Namen nicht, wenn sie ein VERWEIS sind (eine Regel mit `/` am Ende gilt nur fuer Verzeichnisse): {}",
        verweis.join(", ")
    );
    assert!(
        verzeichnis.is_empty(),
        "Git ignoriert ein echtes Verzeichnis unter diesen Namen nicht mehr: {}",
        verzeichnis.join(", ")
    );
}

/// Ein Waechter, der nichts findet, ist gruen: die Pruefung muss an schlechten Fassungen rot werden koennen.
#[test]
fn die_pruefung_schlaegt_an() {
    let alle: Vec<String> = BUILD_NAMEN.iter().map(|n| (*n).to_string()).collect();
    let alle_dateien: Vec<String> = BUILD_NAMEN.iter().map(|n| format!("{n}/datei.txt")).collect();

    // Der Rueckfall, den die Entscheidung beschreibt: jede Regel mit `/` am Ende. Das Verzeichnis bleibt ignoriert, der Verweis nicht.
    let keine = Vec::<String>::new();
    let mit_schraegstrich = BUILD_NAMEN.map(|n| format!("{n}/\n")).concat();
    assert_eq!(nicht_ignoriert(&mit_schraegstrich, true), alle, "Regeln mit `/`: der Verweis muss durchfallen");
    assert_eq!(nicht_ignoriert(&mit_schraegstrich, false), keine, "Regeln mit `/`: das Verzeichnis bleibt ignoriert");

    // Die Regeln ohne `/` (der heutige Stand) sind in beiden Faellen gut.
    let ohne_schraegstrich = BUILD_NAMEN.map(|n| format!("{n}\n")).concat();
    assert_eq!(nicht_ignoriert(&ohne_schraegstrich, true), keine);
    assert_eq!(nicht_ignoriert(&ohne_schraegstrich, false), keine);

    // Eine Regel fehlt: beide Faelle melden genau diesen Namen.
    let ohne_catala = BUILD_NAMEN
        .iter()
        .filter(|n| **n != "oracle/gettsim/_catala")
        .map(|n| format!("{n}\n"))
        .collect::<Vec<String>>()
        .concat();
    assert_eq!(nicht_ignoriert(&ohne_catala, true), vec!["oracle/gettsim/_catala".to_string()]);
    assert_eq!(nicht_ignoriert(&ohne_catala, false), vec!["oracle/gettsim/_catala/datei.txt".to_string()]);

    // Leere Datei: alles faellt durch.
    assert_eq!(nicht_ignoriert("", true), alle);
    assert_eq!(nicht_ignoriert("", false), alle_dateien);
}
