//! `taxgraph-xml --fall <id> --ausgabe <pfad> [--land BY]` — schreibt die Erklärung eines Falls als
//! XML-Datei (mit Testmerker), damit `taxgraph-versand` sie prüft und sendet. Dieses Programm
//! sendet nichts, kennt kein Zertifikat und keine PIN und ändert die Akte nicht.
//!
//! Exit 0: die Datei liegt am Ziel. Exit 2: der Bau bricht ab (Sperre, fehlende Angabe, Writer),
//! der Grund steht auf stderr, es entsteht keine Datei. Exit 1: Aufruf- oder Dateifehler.
use std::path::PathBuf;
use std::process::ExitCode;

use api::einreichen::xml_exportieren;
use api::konfig::{lade_env_dateien, Konfig};
use api::{ApiFehler, Zustand};

const HILFE: &str = "taxgraph-xml --fall <id> --ausgabe <pfad> [--land BY]";

struct Args {
    fall: String,
    ausgabe: PathBuf,
    land: String,
}

fn lies_args(mut roh: impl Iterator<Item = String>) -> Result<Args, String> {
    let (mut fall, mut ausgabe, mut land) = (None, None, None);
    while let Some(name) = roh.next() {
        let ziel = match name.as_str() {
            "--fall" => &mut fall,
            "--ausgabe" => &mut ausgabe,
            "--land" => &mut land,
            _ => return Err(format!("unbekannte Option {name:?}")),
        };
        let wert = roh.next().ok_or_else(|| format!("{name} braucht einen Wert"))?;
        if ziel.replace(wert).is_some() {
            return Err(format!("{name} steht zweimal"));
        }
    }
    Ok(Args {
        fall: fall.ok_or("--fall ist Pflicht")?,
        ausgabe: PathBuf::from(ausgabe.ok_or("--ausgabe ist Pflicht")?),
        land: land.unwrap_or_else(|| "BY".to_owned()),
    })
}

fn fehlertext(e: &ApiFehler) -> String {
    match e {
        ApiFehler::Status(s, m) => format!("{s}: {m}"),
        ApiFehler::Unerwartet { typ, meldung } => format!("{typ}: {meldung}"),
    }
}

fn main() -> ExitCode {
    let args = match lies_args(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(f) => {
            eprintln!("[xml] {f}\n[xml] {HILFE}");
            return ExitCode::from(1);
        }
    };
    // Wie `taxgraph-api`: gitignorierte Env-Dateien zuerst (dort steht `ELSTER_HERSTELLER_ID`), das
    // Prozess-Env gewinnt.
    lade_env_dateien(&Konfig::aus_env().wurzel);
    let z = match Zustand::aus_env() {
        Ok(z) => z,
        Err(e) => {
            eprintln!("[xml] Start: {e}");
            return ExitCode::from(1);
        }
    };
    match xml_exportieren(&z, &args.fall, &args.land, &args.ausgabe) {
        Ok(Ok(n)) => {
            println!(
                "[xml] geschrieben: {} ({n} Bytes, Testmerker 700000004)",
                args.ausgabe.display()
            );
            ExitCode::SUCCESS
        }
        Ok(Err(antwort)) => {
            eprintln!("[xml] keine Datei geschrieben, der Bau bricht ab.");
            for schluessel in ["grund", "klartext", "hinweis", "detail"] {
                if let Some(text) = antwort.body.get(schluessel).and_then(|v| v.as_str()) {
                    eprintln!("[xml]   {schluessel}: {text}");
                }
            }
            if let Some(liste) = antwort.body.get("unvollstaendig") {
                eprintln!("[xml]   unvollstaendig: {liste}");
            }
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("[xml] {}", fehlertext(&e));
            ExitCode::from(1)
        }
    }
}
