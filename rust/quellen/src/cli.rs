//! Kommandozeile von `taxgraph-quellen`: `pruefe`, `einfrieren`, `pdf-einfrieren`.
//!
//! Die Optionen heißen wie die der Python-Skripte (`--url`, `--name`, …), `--opt wert` und `--opt=wert`. Exit-Codes wie
//! Python: 0 gut, 1 abgelehnt (`SystemExit` mit Text), 2 falsche Aufrufform (`argparse`).
//!
//! ponytail: kein `argparse`-Nachbau mit Kurzformen (`--min` für `--min-laenge`); die Skripte wurden immer mit den vollen
//! Namen gerufen. Ausbau: ein Argument-Crate, wenn das Werkzeug mehr Unterbefehle bekommt.

use std::path::{Path, PathBuf};

use crate::einfrieren::{self, Auftrag, Ebene, Werkzeuge};
use crate::pruefen;

const HILFE: &str = "Aufruf: taxgraph-quellen <befehl> [optionen]

  pruefe           [--wurzel <ordner>]
                   jede sources/**/*.meta.yaml pruefen (make sources-check)

  einfrieren       --url <url> --name <name> --norm <text> --start <passage> --abrufdatum <datum>
                   [--ende <passage>] [--erwarte <passage>]... [--min-laenge <n>] [--fassung <text>]
                   [--verwendet-in <pfad>] [--out <ordner>] [--authority gesetz|verwaltung]
                   [--ebene paragraph|schreiben] [--norm-uri <text>] [--wurzel <ordner>]
                   eine Rechtsquelle abrufen (http(s) ueber curl, file:// aus einer Datei), entstrippen und einfrieren
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

    /// Alle Werte in Aufrufreihenfolge (wie `argparse` bei `append`).
    fn alle(&self, name: &str) -> Vec<String> {
        self.0.iter().filter(|(n, _)| n == name).map(|(_, w)| w.clone()).collect()
    }

    /// Ein Pflichtwert; fehlt er, steht sein Name in der Meldung.
    fn muss(&self, name: &str) -> Result<String, String> {
        self.eins(name).map(str::to_owned).ok_or_else(|| format!("die Option --{name} ist erforderlich"))
    }

    /// Ein Wert aus einer festen Auswahl (`choices=` bei `argparse`).
    fn wahl<'a>(&'a self, name: &str, erlaubt: &[&str], vorgabe: &'a str) -> Result<&'a str, String> {
        let wert = self.eins(name).unwrap_or(vorgabe);
        if erlaubt.contains(&wert) {
            Ok(wert)
        } else {
            Err(format!("ungueltiger Wert fuer --{name}: {wert:?} (erlaubt: {})", erlaubt.join(", ")))
        }
    }
}

/// Liest die Optionen von `einfrieren` wie `argparse` in `freeze_source.py`. `standard_out` ist die Vorgabe von `--out`
/// (Python: `<wurzel>/sources/gesetze-im-internet`).
///
/// ```
/// let args: Vec<String> = ["--url", "u", "--name", "n", "--norm", "N", "--start", "s", "--abrufdatum", "2026-01-01"]
///     .iter().map(ToString::to_string).collect();
/// let a = quellen::cli::einfrier_auftrag(&args, std::path::Path::new("/x")).unwrap();
/// assert_eq!(a.min_laenge, 500);
/// assert_eq!(a.norm_uri, "n");
/// assert_eq!(a.out, std::path::PathBuf::from("/x"));
/// ```
///
/// # Errors
/// Der Text für die Aufrufform: Pflichtoption fehlt, unbekannte Option, Wert nicht erlaubt, Zahl nicht lesbar.
pub fn einfrier_auftrag(args: &[String], standard_out: &Path) -> Result<Auftrag, String> {
    auftrag_aus(&Optionen::lies(args, EINFRIEREN_OPTIONEN)?, standard_out)
}

const EINFRIEREN_OPTIONEN: &[&str] = &[
    "url", "name", "norm", "start", "ende", "erwarte", "min-laenge", "fassung", "abrufdatum", "verwendet-in", "out", "authority",
    "ebene", "norm-uri", "wurzel",
];

fn auftrag_aus(opt: &Optionen, standard_out: &Path) -> Result<Auftrag, String> {
    let name = opt.muss("name")?;
    let min_laenge = match opt.eins("min-laenge") {
        None => 500,
        Some(z) => z.trim().parse::<i64>().map_err(|_| format!("--min-laenge: keine ganze Zahl: {z:?}"))?,
    };
    let ebene = match opt.wahl("ebene", &["paragraph", "schreiben"], "paragraph")? {
        "schreiben" => Ebene::Schreiben,
        _ => Ebene::Paragraph,
    };
    Ok(Auftrag {
        url: opt.muss("url")?,
        norm: opt.muss("norm")?,
        start: opt.muss("start")?,
        abrufdatum: opt.muss("abrufdatum")?,
        ende: opt.eins("ende").map(str::to_owned),
        erwarte: opt.alle("erwarte"),
        min_laenge,
        fassung: opt.eins("fassung").unwrap_or("geltende Fassung 2026").to_owned(),
        verwendet_in: opt.eins("verwendet-in").unwrap_or("").to_owned(),
        out: opt.eins("out").map_or_else(|| standard_out.to_path_buf(), PathBuf::from),
        authority: opt.wahl("authority", &["gesetz", "verwaltung"], "gesetz")?.to_owned(),
        ebene,
        // Python: `args.norm_uri or args.name.replace('_', '/')`
        norm_uri: opt.eins("norm-uri").filter(|s| !s.is_empty()).map_or_else(|| name.replace('_', "/"), str::to_owned),
        name,
    })
}

/// Führt die Kommandozeile aus (ohne den Programmnamen) und gibt den Exit-Code zurück.
#[must_use]
pub fn lauf(args: &[String]) -> i32 {
    match args.split_first() {
        Some((befehl, rest)) if befehl == "pruefe" => pruefe(rest),
        Some((befehl, rest)) if befehl == "einfrieren" => einfrieren(rest),
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

/// Die Wurzel aus `--wurzel` oder dem Bauort, absolut.
fn wurzel_aus(opt: &Optionen) -> PathBuf {
    let wurzel = opt.eins("wurzel").map_or_else(vorgabe_wurzel, PathBuf::from);
    std::path::absolute(&wurzel).unwrap_or(wurzel)
}

fn pruefe(args: &[String]) -> i32 {
    let opt = match Optionen::lies(args, &["wurzel"]) {
        Ok(o) => o,
        Err(grund) => return aufruf_falsch(&grund),
    };
    match pruefen::pruefe_zeilenweise(&wurzel_aus(&opt), &mut |z| println!("{z}")) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(fehler) => {
            eprintln!("{fehler}");
            1
        }
    }
}

fn einfrieren(args: &[String]) -> i32 {
    let opt = match Optionen::lies(args, EINFRIEREN_OPTIONEN) {
        Ok(o) => o,
        Err(grund) => return aufruf_falsch(&grund),
    };
    // `--wurzel` gehört nicht zu `freeze_source.py` (dort ist die Wurzel der Ort des Skripts); es legt nur die Vorgabe von
    // `--out` fest: `<wurzel>/sources/gesetze-im-internet`.
    let auftrag = match auftrag_aus(&opt, &wurzel_aus(&opt).join("sources/gesetze-im-internet")) {
        Ok(a) => a,
        Err(grund) => return aufruf_falsch(&grund),
    };
    let werkzeuge = Werkzeuge::default();
    let ergebnis = einfrieren::hole(&auftrag.url, &werkzeuge)
        .and_then(|roh| einfrieren::text_aus_antwort(roh, &werkzeuge))
        .and_then(|text| einfrieren::friere_ein(&auftrag, &text));
    match ergebnis {
        Ok(gefroren) => {
            println!("{}", gefroren.meldung);
            0
        }
        Err(fehler) => {
            eprintln!("{fehler}");
            1
        }
    }
}
