//! `taxgraph-versand`: das Programm zu [`versand::lauf`]. Alles Wesentliche steht dort; hier nur
//! die Prozess-Umgebung, stdin und stdout. Siehe `elster/VERSAND.md`.

use std::io::IsTerminal;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let env = versand::Umgebung::aus_prozess();
    let stdin = std::io::stdin();
    let ist_terminal = stdin.is_terminal();
    let rc = versand::lauf(
        &args,
        &env,
        &mut stdin.lock(),
        ist_terminal,
        &mut std::io::stdout(),
    );
    ExitCode::from(u8::try_from(rc).unwrap_or(2))
}
