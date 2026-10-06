//! Round-Trip von `make backup` und `make restore` gegen den Rust-Dienst, hermetisch (ohne Python, ohne
//! `import server`, ohne Netz).
//!
//! Ersatz von `tests/test_backup_restore_roundtrip.py` (Review 2026-08-17, fuenf Maengel). Der Python-Test sicherte
//! und stellte einen nachgebauten Mini-Bestand wieder her. Hier schreibt der Rust-Dienst den Bestand selbst (Konten
//! ueber `/auth/register`, Akten und Events ueber `/fall`, das Protokoll `audit.jsonl` im selben Ordner), `make backup`
//! sichert ihn, ein zweiter Dienst, der alles frisch von der Platte liest (Neustart), muss nach `make restore` denselben
//! Stand antworten. Gepruft wird, was ein Umzug braucht: byte-gleiche Akten, die Nutzerdatei (ohne sie sperrt die
//! Besitzerpruefung jede Akte), und dass die Anmeldung mit dem alten Passwort wieder klappt.
//!
//! - Round-Trip: Drift nach der Sicherung (neue Akte, geloeschte Akte, beschaedigte Akte, Fremddatei, neues Konto)
//!   darf den Restore nicht ueberleben (ersetzen statt mergen); die Ablehnung der Rueckfrage schreibt nichts; ein
//!   echter Restore legt vorher selbst eine Sicherung an.
//! - Totalverlust: Bestand und Nutzerdatei sind weg (frische Maschine), `restore` muss trotzdem laufen.
//! - Scheitert die Vorher-Sicherung, bricht `restore` ab, BEVOR etwas geloescht wird.
//! - Totalverlust auch in der Variante "Datenverzeichnis existiert gar nicht" (B8): `restore` muss es
//!   anlegen, sonst scheitert `tar -C` (rc 2).
//!
//! Sicherheit: jede `make`-Zeile bekommt `FAELLE_ROOT`, `AUTH_USERS` und `BACKUP_DIR` aus derselben Stelle ([`Umgebung`])
//! und ein `HOME` im Testordner, `TAXGRAPH_DATEN` und `XDG_DATA_HOME` fehlen: auch ein vergessener Operand
//! landet nie im echten Bestand.
//!
//! Grenzen: Der Test fuehrt `make` und `tar` des Hosts aus. Dass der Standardpfad des Makefiles (`TAXGRAPH_DATEN`, dann
//! `XDG_DATA_HOME`, dann `~/.local/share`) mit `Konfig::aus_env` uebereinstimmt, prueft er nicht; er gibt den Pfad
//! ausdruecklich vor. Die Vorher-Sicherung bei Fehler prueft er nicht als root (dort gelingt jede Sicherung).
#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const PASSWORT: &str = "password1";

/// Ein Testordner mit den drei Orten, die `make backup` und `make restore` kennen.
struct Umgebung {
    tmp: tempfile::TempDir,
}

impl Umgebung {
    fn neu() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("auth")).unwrap();
        Self { tmp }
    }

    /// `FAELLE_ROOT` des Makefiles: darunter liegt `faelle/`.
    fn daten(&self) -> PathBuf {
        self.tmp.path().join("daten")
    }

    fn faelle(&self) -> PathBuf {
        self.daten().join("faelle")
    }

    fn users(&self) -> PathBuf {
        self.tmp.path().join("auth").join("users.json")
    }

    fn backups(&self) -> PathBuf {
        self.tmp.path().join("backups")
    }

    /// Ein frischer Dienst, der alles von der Platte liest (wie nach einem Neustart).
    fn dienst(&self) -> Zustand {
        let konfig = Konfig {
            wurzel: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            faelle: self.faelle(),
            audit_dir: self.faelle(),
        };
        let auth = Auth::neu(
            "testgeheimnis".into(),
            self.users(),
            Some(konfig.audit_pfad()),
        );
        Zustand::neu(konfig, auth)
    }

    /// `make <ziel>` im Wurzelordner mit allen drei Orten, `extra` zusaetzlich; `eingabe` geht an stdin.
    fn make(&self, ziel: &str, backup_dir: &Path, extra: &[String], eingabe: &str) -> Output {
        let mut kind = Command::new("make")
            .arg(ziel)
            .arg(format!("FAELLE_ROOT={}", self.daten().display()))
            .arg(format!("AUTH_USERS={}", self.users().display()))
            .arg(format!("BACKUP_DIR={}", backup_dir.display()))
            .args(extra)
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .env("HOME", self.tmp.path())
            .env_remove("TAXGRAPH_DATEN")
            .env_remove("XDG_DATA_HOME")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        kind.stdin
            .take()
            .unwrap()
            .write_all(eingabe.as_bytes())
            .unwrap();
        kind.wait_with_output().unwrap()
    }

    fn sichern(&self) -> Output {
        self.make("backup", &self.backups(), &[], "")
    }

    fn wiederherstellen(&self, archiv: &Path, bestaetigt: bool, eingabe: &str) -> Output {
        let mut extra = vec![format!("ARCHIV={}", archiv.display())];
        if bestaetigt {
            extra.push("CONFIRM=yes".to_string());
        }
        self.make("restore", &self.backups(), &extra, eingabe)
    }

    /// Alle Archive im Sicherungsordner, sortiert.
    fn archive(&self) -> Vec<PathBuf> {
        let mut aus: Vec<PathBuf> = std::fs::read_dir(self.backups())
            .map(|d| {
                d.map(|e| e.unwrap().path())
                    .filter(|p| p.to_string_lossy().ends_with(".tar.gz"))
                    .collect()
            })
            .unwrap_or_default();
        aus.sort();
        aus
    }
}

fn ausgabe(o: &Output) -> String {
    format!(
        "rc={:?}\nstdout:\n{}\nstderr:\n{}",
        o.status.code(),
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// Pfad ab `wurzel` (mit `/`) -> Inhalt, rekursiv.
fn inhalte(wurzel: &Path) -> BTreeMap<String, Vec<u8>> {
    fn gehe(wurzel: &Path, dir: &Path, aus: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                gehe(wurzel, &p, aus);
            } else {
                let rel = p
                    .strip_prefix(wurzel)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                aus.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    let mut aus = BTreeMap::new();
    if wurzel.is_dir() {
        gehe(wurzel, wurzel, &mut aus);
    }
    aus
}

/// Unterschied zweier Staende als Text: nur vorher, nur nachher, inhaltlich anders.
fn unterschied(vorher: &BTreeMap<String, Vec<u8>>, nachher: &BTreeMap<String, Vec<u8>>) -> String {
    let nur = |a: &BTreeMap<String, Vec<u8>>, b: &BTreeMap<String, Vec<u8>>| -> Vec<String> {
        a.keys().filter(|k| !b.contains_key(*k)).cloned().collect()
    };
    let anders: Vec<&String> = vorher
        .iter()
        .filter(|(k, v)| nachher.get(*k).is_some_and(|n| n != *v))
        .map(|(k, _)| k)
        .collect();
    format!(
        "nur vorher: {:?}; nur nachher: {:?}; inhaltlich anders: {anders:?}",
        nur(vorher, nachher),
        nur(nachher, vorher)
    )
}

async fn sende(
    z: &Zustand,
    methode: &str,
    pfad: &str,
    token: Option<&str>,
    body: Option<&Value>,
) -> (u16, Value) {
    let mut b = Request::builder().method(methode).uri(pfad);
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    let rumpf = body.map(ToString::to_string);
    if let Some(r) = &rumpf {
        b = b
            .header("content-type", "application/json")
            .header("content-length", r.len().to_string());
    }
    let req = b.body(rumpf.map_or_else(Body::empty, Body::from)).unwrap();
    let antwort = app(z.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = antwort.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (
        teile.status.as_u16(),
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn registrieren(z: &Zustand, name: &str) {
    let (status, antwort) = sende(
        z,
        "POST",
        "/auth/register",
        None,
        Some(&json!({"username": name, "password": PASSWORT})),
    )
    .await;
    assert_eq!(status, 201, "register {name}: {antwort}");
}

/// Meldet an und liefert den Status; bei 200 steht das Token in der Antwort.
async fn anmelden(z: &Zustand, name: &str, passwort: &str) -> (u16, Value) {
    sende(
        z,
        "POST",
        "/auth/login",
        None,
        Some(&json!({"username": name, "password": passwort})),
    )
    .await
}

async fn token(z: &Zustand, name: &str) -> String {
    let (status, antwort) = anmelden(z, name, PASSWORT).await;
    assert_eq!(status, 200, "login {name}: {antwort}");
    antwort["token"].as_str().unwrap().to_owned()
}

async fn fall_anlegen(z: &Zustand, token: &str, id: &str) {
    let rumpf = json!({"fall_id": id, "scheibe": "ep", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(z, "POST", "/fall", Some(token), Some(&rumpf)).await;
    assert_eq!(status, 201, "POST /fall {id}: {antwort}");
}

/// Ein Event auf `ep_arbeitstage` (so, wie `event_naht.rs` es schreibt).
async fn event_schreiben(z: &Zustand, token: &str, id: &str) {
    let rumpf = json!({
        "feld_id": "ep_arbeitstage", "wert": 220, "zustand": "bestaetigt", "schreiber": "ui:rundlauf",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": "klick"}, "ts": "2026-01-01T00:00:00+00:00"
    });
    let (status, antwort) = sende(
        z,
        "POST",
        &format!("/fall/{id}/event"),
        Some(token),
        Some(&rumpf),
    )
    .await;
    assert_eq!(status, 201, "POST /fall/{id}/event: {antwort}");
}

/// Der Bestand, den der Dienst schreibt: Konto alice, Akten `fall-a` (mit einem Event) und `fall-b`.
/// Liefert die Antwort von `GET /fall/fall-a/stand`, wie der laufende Dienst sie gibt.
async fn bestand_aufbauen(u: &Umgebung) -> Value {
    let z = u.dienst();
    registrieren(&z, "alice").await;
    let t = token(&z, "alice").await;
    fall_anlegen(&z, &t, "fall-a").await;
    event_schreiben(&z, &t, "fall-a").await;
    fall_anlegen(&z, &t, "fall-b").await;
    let (status, stand) = sende(&z, "GET", "/fall/fall-a/stand", Some(&t), None).await;
    assert_eq!(status, 200, "{stand}");
    stand
}

/// Was nach einer Wiederherstellung wieder stimmen muss: die Akten mit ihren Events, `fall-b` ist
/// zurueck, `fall-c` gibt es nicht, die Anmeldung mit dem alten Passwort klappt, das neue Konto fehlt.
async fn pruefe_wiederhergestellt(u: &Umgebung, stand_a: &Value) {
    let neu = u.dienst();
    let t = token(&neu, "alice").await;
    let (status, stand) = sende(&neu, "GET", "/fall/fall-a/stand", Some(&t), None).await;
    assert_eq!(status, 200, "{stand}");
    assert_eq!(&stand, stand_a, "fall-a antwortet nach dem Restore anders");
    let (status, antwort) = sende(&neu, "GET", "/fall/fall-b/stand", Some(&t), None).await;
    assert_eq!(status, 200, "fall-b fehlt nach dem Restore: {antwort}");
    let (status, _) = sende(&neu, "GET", "/fall/fall-c/stand", Some(&t), None).await;
    assert_eq!(status, 404, "fall-c entstand erst nach der Sicherung");
    let (status, _) = anmelden(&neu, "bob", PASSWORT).await;
    assert_eq!(status, 401, "bob entstand erst nach der Sicherung");
}

/// Drift nach der Sicherung: ein Konto, eine neue Akte, eine geloeschte, eine beschaedigte, eine Fremddatei.
async fn drift_erzeugen(u: &Umgebung) {
    let z = u.dienst();
    registrieren(&z, "bob").await;
    let t = token(&z, "alice").await;
    fall_anlegen(&z, &t, "fall-c").await;
    let (status, antwort) = sende(&z, "DELETE", "/fall/fall-b", Some(&t), None).await;
    assert_eq!(status, 200, "DELETE fall-b: {antwort}");
    let mut beschaedigt = std::fs::read(u.faelle().join("fall-a.json")).unwrap();
    beschaedigt.extend_from_slice(b"MANIPULIERT");
    std::fs::write(u.faelle().join("fall-a.json"), beschaedigt).unwrap();
    std::fs::write(
        u.faelle().join("orphan-nach-backup.json"),
        r#"{"nicht": "im archiv"}"#,
    )
    .unwrap();
}

#[tokio::test]
async fn der_rundlauf_stellt_den_stand_des_dienstes_exakt_wieder_her() {
    let u = Umgebung::neu();
    let stand_a = bestand_aufbauen(&u).await;
    let akten = inhalte(&u.faelle());
    let konten = std::fs::read(u.users()).unwrap();
    assert!(
        akten.keys().any(|k| k == "fall-a.json") && akten.keys().any(|k| k == "audit.jsonl"),
        "der Dienst hat Akte und Protokoll geschrieben: {:?}",
        akten.keys()
    );

    let r = u.sichern();
    assert!(r.status.success(), "make backup:\n{}", ausgabe(&r));
    let archive = u.archive();
    assert_eq!(archive.len(), 1, "genau ein Archiv erwartet: {archive:?}");

    drift_erzeugen(&u).await;
    assert_ne!(
        inhalte(&u.faelle()),
        akten,
        "der Drift hat nichts veraendert"
    );
    assert_ne!(std::fs::read(u.users()).unwrap(), konten);

    // Die Ablehnung der Rueckfrage bricht ab und schreibt nichts.
    let r = u.wiederherstellen(&archive[0], false, "n\n");
    assert!(
        !r.status.success(),
        "Ablehnung muss abbrechen:\n{}",
        ausgabe(&r)
    );
    assert!(
        u.faelle().join("orphan-nach-backup.json").exists(),
        "die Ablehnung hat trotzdem geschrieben:\n{}",
        ausgabe(&r)
    );
    assert_eq!(u.archive().len(), 1, "die Ablehnung hat gesichert");

    // Der echte Restore (ohne Rueckfrage): ersetzen statt mergen, die Konten kommen mit.
    let r = u.wiederherstellen(&archive[0], true, "");
    assert!(r.status.success(), "make restore:\n{}", ausgabe(&r));
    let nachher = inhalte(&u.faelle());
    assert_eq!(
        nachher,
        akten,
        "Restore weicht vom Sicherungsstand ab: {}",
        unterschied(&akten, &nachher)
    );
    assert_eq!(
        std::fs::read(u.users()).unwrap(),
        konten,
        "Nutzerdatei weicht ab"
    );
    assert_eq!(
        u.archive().len(),
        2,
        "restore haette vor dem Ueberschreiben selbst sichern muessen"
    );

    pruefe_wiederhergestellt(&u, &stand_a).await;
}

#[tokio::test]
async fn restore_stellt_nach_totalverlust_wieder_her() {
    let u = Umgebung::neu();
    let stand_a = bestand_aufbauen(&u).await;
    let akten = inhalte(&u.faelle());
    let konten = std::fs::read(u.users()).unwrap();
    let r = u.sichern();
    assert!(r.status.success(), "make backup:\n{}", ausgabe(&r));
    let archiv = u.archive().remove(0);

    // Frische Maschine: weder Bestand noch Konten.
    std::fs::remove_dir_all(u.faelle()).unwrap();
    std::fs::remove_file(u.users()).unwrap();

    let r = u.wiederherstellen(&archiv, true, "");
    assert!(
        r.status.success(),
        "restore auf leeres Ziel:\n{}",
        ausgabe(&r)
    );
    assert_eq!(inhalte(&u.faelle()), akten, "Bestand weicht ab");
    assert_eq!(
        std::fs::read(u.users()).unwrap(),
        konten,
        "Nutzerdatei weicht ab"
    );
    assert_eq!(
        u.archive().len(),
        1,
        "ohne Bestand gibt es nichts zu sichern, die Vorher-Sicherung entfaellt"
    );
    pruefe_wiederhergestellt(&u, &stand_a).await;
}

/// Totalverlust des Datenverzeichnisses selbst (B8, V4 Stapel 1e): nicht nur `faelle/` ist weg, das
/// Verzeichnis existiert nicht — der Zustand auf einer frisch installierten Maschine.
/// `restore` muss es anlegen; ohne das `mkdir -p` vor dem Entpacken scheitert `tar -C`
/// (`Cannot chdir`, rc 2) und die Sicherung ist auf genau dem Weg unbenutzbar, fuer den sie da ist.
#[tokio::test]
async fn restore_legt_das_datenverzeichnis_bei_totalverlust_an() {
    let u = Umgebung::neu();
    let stand_a = bestand_aufbauen(&u).await;
    let akten = inhalte(&u.faelle());
    let konten = std::fs::read(u.users()).unwrap();
    let r = u.sichern();
    assert!(r.status.success(), "make backup:\n{}", ausgabe(&r));
    let archiv = u.archive().remove(0);

    // Das Datenverzeichnis ist weg, nicht nur sein Inhalt: `tar -C` faende kein Ziel mehr.
    std::fs::remove_dir_all(u.daten()).unwrap();
    std::fs::remove_file(u.users()).unwrap();
    assert!(
        !u.daten().exists(),
        "KONTROLLE: das Datenverzeichnis existiert noch"
    );

    let r = u.wiederherstellen(&archiv, true, "");
    assert!(
        r.status.success(),
        "restore ohne Datenverzeichnis:\n{}",
        ausgabe(&r)
    );
    assert!(
        u.daten().is_dir(),
        "restore hat das Datenverzeichnis nicht angelegt"
    );
    assert_eq!(inhalte(&u.faelle()), akten, "Bestand weicht ab");
    assert_eq!(
        std::fs::read(u.users()).unwrap(),
        konten,
        "Nutzerdatei weicht ab"
    );
    assert_eq!(
        u.archive().len(),
        1,
        "ohne Bestand gibt es nichts zu sichern, die Vorher-Sicherung entfaellt"
    );
    pruefe_wiederhergestellt(&u, &stand_a).await;
}

/// Stellt die Rechte eines Ordners beim Verlassen wieder her, damit der Testordner sich loeschen laesst.
struct Entsperrt(PathBuf);

impl Drop for Entsperrt {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
    }
}

#[tokio::test]
async fn eine_gescheiterte_vorher_sicherung_verhindert_das_loeschen() {
    let u = Umgebung::neu();
    bestand_aufbauen(&u).await;
    let r = u.sichern();
    assert!(r.status.success(), "make backup:\n{}", ausgabe(&r));
    let archiv = u.archive().remove(0);
    let akten = inhalte(&u.faelle());
    let konten = std::fs::read(u.users()).unwrap();

    // Die Vorher-Sicherung scheitert: ihr Ordner liegt unter einem Elternteil ohne Schreibrecht.
    let gesperrt = u.tmp.path().join("gesperrt");
    std::fs::create_dir(&gesperrt).unwrap();
    let _freigabe = Entsperrt(gesperrt.clone());
    std::fs::set_permissions(&gesperrt, std::fs::Permissions::from_mode(0o500)).unwrap();
    if std::fs::create_dir(gesperrt.join("probe")).is_ok() {
        eprintln!(
            "uebersprungen: als root gelingt jede Sicherung, Schreibrechte sind nicht durchsetzbar"
        );
        return;
    }
    let extra = [
        format!("ARCHIV={}", archiv.display()),
        "CONFIRM=yes".to_string(),
    ];
    let r = u.make("restore", &gesperrt.join("darunter"), &extra, "");
    assert!(
        !r.status.success(),
        "restore lief trotz gescheiterter Vorher-Sicherung weiter:\n{}",
        ausgabe(&r)
    );
    let nachher = inhalte(&u.faelle());
    assert_eq!(
        nachher,
        akten,
        "restore hat vor dem Abbruch schon geschrieben: {}",
        unterschied(&akten, &nachher)
    );
    assert_eq!(std::fs::read(u.users()).unwrap(), konten);
}
