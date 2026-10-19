//! Kommandozeile von `taxgraph-quellen`: `pruefe`, `einfrieren`, `pdf-einfrieren`.
//!
//! Die Optionen heißen wie die der Python-Skripte (`--url`, `--name`, …), `--opt wert` und `--opt=wert`. Exit-Codes wie
//! Python: 0 gut, 1 abgelehnt (`SystemExit` mit Text), 2 falsche Aufrufform (`argparse`).
//!
//! ponytail: kein `argparse`-Nachbau mit Kurzformen (`--min` für `--min-laenge`); die Skripte wurden immer mit den vollen
//! Namen gerufen. Ausbau: ein Argument-Crate, wenn das Werkzeug mehr Unterbefehle bekommt.

use std::path::{Path, PathBuf};

use crate::pruefen;

const HILFE: &str = "Aufruf: taxgraph-quellen <befehl> [optionen]

  pruefe           [--wurzel <ordner>]
                   jede sources/**/*.meta.yaml pruefen (make sources-check)
";

/// Die Wurzel des Baums, in dem das Programm gebaut wurde (zwei Ordner über `rust/quellen`).
fn vorgabe_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Optionen einer Kommandozeile: Paare `(name, wert)` in der Reihenfolge des Aufrufs.
struct Optionen(Vec<(String, String)>);

impl Optionen {
    /// Liest `--name wert` und `--name=wert`. Jeder Name in `erlaubt` verlangt einen Wert.
    fn lies(args: &[String], erlaubt: &[&str]) -> Result<Self, String> {
        let mut paare = Vec::new();
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            let Some(rest) = arg.strip_prefix("--") else {
                return Err(format!("unerwartetes Argument: {arg}"));
            };
            let (name, wert) = match rest.split_once('=') {
                Some((n, w)) => (n, w.to_owned()),
                None => (rest, iter.next().cloned().ok_or_else(|| format!("--{rest} braucht einen Wert"))?),
            };
            if !erlaubt.contains(&name) {
                return Err(format!("unbekannte Option: --{name}"));
            }
            paare.push((name.to_owned(), wert));
        }
        Ok(Self(paare))
    }

    /// Der letzte Wert (wie `argparse` bei `store`).
    fn eins(&self, name: &str) -> Option<&str> {
        self.0.iter().rev().find(|(n, _)| n == name).map(|(_, w)| w.as_str())
    }
}

/// Führt die Kommandozeile aus (ohne den Programmnamen) und gibt den Exit-Code zurück.
#[must_use]
pub fn lauf(args: &[String]) -> i32 {
    match args.split_first() {
        Some((befehl, rest)) if befehl == "pruefe" => pruefe(rest),
        Some((befehl, _)) if matches!(befehl.as_str(), "-h" | "--help" | "help") => {
            print!("{HILFE}");
            0
        }
        _ => aufruf_falsch("befehl fehlt oder ist unbekannt"),
    }
}

fn aufruf_falsch(grund: &str) -> i32 {
    eprint!("taxgraph-quellen: {grund}\n{HILFE}");
    2
}

fn pruefe(args: &[String]) -> i32 {
    let opt = match Optionen::lies(args, &["wurzel"]) {
        Ok(o) => o,
        Err(grund) => return aufruf_falsch(&grund),
    };
    let wurzel = opt.eins("wurzel").map_or_else(vorgabe_wurzel, PathBuf::from);
    let wurzel = std::path::absolute(&wurzel).unwrap_or(wurzel);
    match pruefen::pruefe_zeilenweise(&wurzel, &mut |z| println!("{z}")) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(fehler) => {
            eprintln!("{fehler}");
            1
        }
    }
}
