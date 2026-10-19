//! Das Programm `taxgraph-abruf`: STUMPF fuer den roten Test.

use std::io::{BufRead, Write};
use std::path::PathBuf;

/// Woertliche Freigabe fuer den echten Abruf.
pub const ABRUF_FREIGABE: &str = "JA ICH RUFE DEN BELEG BEI ELSTER AB";

/// Was das Programm aus der Prozess-Umgebung braucht.
pub struct AbrufUmgebung {
    pub zertifikat_pfad: String,
    pub pin: String,
    pub hersteller_id: String,
    pub eric_lib: Option<PathBuf>,
    pub heute_jahr: i32,
    pub temp: PathBuf,
}

/// STUMPF.
pub fn lauf_abruf(
    _args: &[String],
    _env: &AbrufUmgebung,
    _stdin: &mut dyn BufRead,
    _ist_terminal: bool,
    _aus: &mut dyn Write,
) -> i32 {
    99
}
