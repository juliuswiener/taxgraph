//! Das Programm `taxgraph-abruf`: die EINZIGE Aufrufstelle von [`hole_beleg`].
//!
//! Der Abruf geht ans Netz (`ERIC_SENDE`) und fragt Steuerdaten ab. Darum gelten dieselben Huerden
//! wie bei `taxgraph-versand`: kein Standard ruft ab; ohne `--dry-run` oder `--abruf` bricht das
//! Programm ab, `--abruf` verlangt die woertliche Freigabe [`ABRUF_FREIGABE`] UND ihre zweite Eingabe
//! an einem Terminal. Alles von aussen (Umgebung, stdin, Ausgabe) kommt als Parameter in
//! [`lauf_abruf`]; so laeuft jeder Pfad im Test, ohne Prozess-Umgebung und ohne Terminal.
//!
//! Die Steuer-ID kommt nur als Aufrufparameter, wird nie gespeichert und nie ausgegeben. Der Beleg
//! geht in eine Datei (0600 in einem Verzeichnis 0700); das Programm druckt ihren Pfad, nie den
//! Inhalt.

use std::fmt;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use chrono::Datelike as _;

use crate::cli::{bestaetigt_phrase, zeile};
use crate::{hole_beleg, lege_beleg_ab, AbrufAuftrag, AbrufFehler, Zertifikat};

/// Woertliche Freigabe fuer den echten Abruf. Bewusst lang, bewusst keine Kurzform.
pub const ABRUF_FREIGABE: &str = "JA ICH RUFE DEN BELEG BEI ELSTER AB";

const NUTZUNG: &str = "Nutzung: taxgraph-abruf --idnr STEUER-ID --jahr JAHR [--zertifikat PFAD] \
[--ziel VERZEICHNIS] (--dry-run | --abruf --freigabe PHRASE)\n\
Zertifikat und PIN: ELSTER_ZERTIFIKAT_PFAD und ELSTER_ZERTIFIKAT_PIN. Hersteller-ID: ELSTER_HERSTELLER_ID.";

/// Was das Programm aus der Prozess-Umgebung braucht. `Debug` zeigt PIN und Hersteller-ID nicht.
pub struct AbrufUmgebung {
    /// `$ELSTER_ZERTIFIKAT_PFAD` (leer, wenn nicht gesetzt); `--zertifikat` hat Vorrang.
    pub zertifikat_pfad: String,
    /// `$ELSTER_ZERTIFIKAT_PIN` (leer, wenn nicht gesetzt). Nie eine Kommandozeilenoption.
    pub pin: String,
    /// `$ELSTER_HERSTELLER_ID` (leer, wenn nicht gesetzt): die 5-stellige Hersteller-ID.
    pub hersteller_id: String,
    /// `libericapi.so` (`elster::find_eric_lib()`).
    pub eric_lib: Option<PathBuf>,
    /// Das laufende Jahr; das Veranlagungsjahr darf hoechstens vier Jahre zurueckliegen.
    pub heute_jahr: i32,
    /// Hier liegen die Log-Verzeichnisse von ERiC (und, ohne `--ziel`, der Beleg).
    pub temp: PathBuf,
}

impl AbrufUmgebung {
    /// Liest die Prozess-Umgebung; ruft ERiC nicht.
    #[must_use]
    pub fn aus_prozess() -> Self {
        Self {
            zertifikat_pfad: std::env::var("ELSTER_ZERTIFIKAT_PFAD").unwrap_or_default(),
            pin: std::env::var("ELSTER_ZERTIFIKAT_PIN").unwrap_or_default(),
            hersteller_id: std::env::var("ELSTER_HERSTELLER_ID").unwrap_or_default(),
            eric_lib: elster::find_eric_lib(),
            heute_jahr: chrono::Utc::now().year(),
            temp: std::env::temp_dir(),
        }
    }
}

impl fmt::Debug for AbrufUmgebung {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AbrufUmgebung")
            .field("eric_lib", &self.eric_lib)
            .field("heute_jahr", &self.heute_jahr)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Aufruf {
    DryRun,
    Abruf,
}

/// Die Steuer-ID steht hier als Text; die Struktur hat kein `Debug`.
struct Args {
    idnr: String,
    jahr: String,
    zertifikat: Option<String>,
    ziel: Option<PathBuf>,
    aufruf: Aufruf,
    freigabe: Option<String>,
}

/// `--name WERT` und `--name=WERT`; ein Modus darf nur einmal vorkommen. Die Fehlertexte nennen nie
/// ein Argument, denn ein Argument kann die Steuer-ID sein.
fn lies_args(args: &[String]) -> Result<Args, String> {
    let (mut idnr, mut jahr, mut zertifikat, mut ziel, mut freigabe) =
        (None, None, None, None, None);
    let mut aufruf: Option<Aufruf> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let (name, inline) = match a.split_once('=') {
            Some((n, w)) if n.starts_with("--") => (n, Some(w.to_owned())),
            _ => (a.as_str(), None),
        };
        let mut wert = |was: &str| {
            inline
                .clone()
                .or_else(|| it.next().cloned())
                .ok_or_else(|| format!("{was} verlangt einen Wert"))
        };
        let modus = match name {
            "--idnr" => {
                idnr = Some(wert("--idnr")?);
                continue;
            }
            "--jahr" => {
                jahr = Some(wert("--jahr")?);
                continue;
            }
            "--zertifikat" => {
                zertifikat = Some(wert("--zertifikat")?);
                continue;
            }
            "--ziel" => {
                ziel = Some(PathBuf::from(wert("--ziel")?));
                continue;
            }
            "--freigabe" => {
                freigabe = Some(wert("--freigabe")?);
                continue;
            }
            "--dry-run" => Aufruf::DryRun,
            "--abruf" => Aufruf::Abruf,
            _ => return Err("unbekannte Option oder unbekanntes Argument".to_owned()),
        };
        if aufruf.replace(modus).is_some() {
            return Err("--dry-run und --abruf schliessen sich aus".to_owned());
        }
    }
    Ok(Args {
        idnr: idnr.ok_or("--idnr ist Pflicht")?,
        jahr: jahr.ok_or("--jahr ist Pflicht")?,
        zertifikat,
        ziel,
        aufruf: aufruf.ok_or("einer von --dry-run, --abruf ist Pflicht")?,
        freigabe,
    })
}

/// Was geschaehe; ERiC bleibt unberuehrt, die Steuer-ID wird nicht gezeigt.
fn dry_run(auftrag: &AbrufAuftrag, zertifikat: &Zertifikat, aus: &mut dyn Write) -> i32 {
    zeile(aus, "[abruf] DRY-RUN — es wird NICHTS abgerufen.");
    zeile(aus, &format!("[abruf]   datenart_version: {}", auftrag.datenart_version()));
    zeile(aus, &format!("[abruf]   veranlagungsjahr: {}", auftrag.veranlagungsjahr()));
    zeile(aus, "[abruf]   steuer_id: 11 Ziffern (nicht gezeigt)");
    zeile(aus, &format!("[abruf]   anfrage_xml_bytes: {}", auftrag.anfrage_xml().len()));
    zeile(aus, &format!("[abruf]   zertifikat_vorhanden: {}", zertifikat.vorhanden()));
    0
}

/// Beide Huerden des echten Abrufs: die Freigabe als Option, dann dieselbe Phrase am Terminal.
fn huerden(
    freigabe: Option<&str>,
    stdin: &mut dyn BufRead,
    ist_terminal: bool,
    aus: &mut dyn Write,
) -> bool {
    if freigabe != Some(ABRUF_FREIGABE) {
        zeile(aus, "[abruf] ABRUF VERWEIGERT — --freigabe muss woertlich sein:");
        zeile(aus, &format!("[abruf]   {ABRUF_FREIGABE}"));
        return false;
    }
    bestaetigt_phrase(
        stdin,
        ist_terminal,
        aus,
        "[abruf] ABRUF",
        "Abruf der Lohnsteuerbescheinigung bei ELSTER — tippe zur Bestaetigung erneut ein (nicht kopieren):",
        ABRUF_FREIGABE,
    )
}

/// 1: ELSTER oder ERiC lief, aber es gibt keinen Beleg; 2: abgebrochen, bevor eine Antwort zaehlt.
fn exit_code(fehler: &AbrufFehler) -> i32 {
    match fehler {
        AbrufFehler::Abgelehnt { .. }
        | AbrufFehler::KeinBeleg { .. }
        | AbrufFehler::MehrereBelege(_)
        | AbrufFehler::AntwortUnlesbar(_)
        | AbrufFehler::Dekodieren { .. } => 1,
        AbrufFehler::DatenpaketLeer
        | AbrufFehler::DatenpaketNulByte
        | AbrufFehler::Zertifikat(_)
        | AbrufFehler::Eric(_)
        | AbrufFehler::Eingabe { .. }
        | AbrufFehler::Ablage(_) => 2,
    }
}

/// Nach einem Fehler bleibt `eric.log` liegen, wenn es entstanden ist (sein Inhalt nennt Auszuege
/// der Anfrage); sonst raeumt das Fallen von `log` das Verzeichnis auf.
fn behalte_log(log: tempfile::TempDir, aus: &mut dyn Write) {
    if log.path().join("eric.log").is_file() {
        let dir = log.keep();
        zeile(aus, &format!("[abruf] eric.log: {}", dir.join("eric.log").display()));
    }
}

/// Der echte Abruf: ERiC-Log in ein eigenes Verzeichnis (0700, nach Erfolg weg), Beleg unter `ziel`.
fn abruf(
    auftrag: &AbrufAuftrag,
    zertifikat: &Zertifikat,
    env: &AbrufUmgebung,
    ziel: &Path,
    aus: &mut dyn Write,
) -> i32 {
    use std::os::unix::fs::PermissionsExt as _;
    let Ok(log) = tempfile::Builder::new()
        .prefix("eric_abruf_")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(&env.temp)
    else {
        zeile(aus, "[abruf] FEHLER: Das Log-Verzeichnis fuer ERiC laesst sich nicht anlegen.");
        return 2;
    };
    let klartext = match hole_beleg(env.eric_lib.as_deref(), zertifikat, auftrag, log.path()) {
        Ok(k) => k,
        Err(e) => {
            zeile(aus, &format!("[abruf] FEHLER: {e}"));
            behalte_log(log, aus);
            return exit_code(&e);
        }
    };
    match lege_beleg_ab(ziel, &klartext) {
        Ok(pfad) => {
            zeile(aus, &format!("[abruf] Beleg abgelegt: {}", pfad.display()));
            zeile(aus, &format!("[abruf]   bytes: {}", klartext.len()));
            let xml = klartext.first() == Some(&b'<');
            zeile(aus, &format!("[abruf]   beginnt_mit_xml_kopf: {}", if xml { "ja" } else { "nein" }));
            if !xml {
                zeile(
                    aus,
                    "[abruf]   Der Beleg beginnt nicht mit '<': er ist vielleicht noch komprimiert (GZIP).",
                );
            }
            0
        }
        Err(e) => {
            zeile(aus, &format!("[abruf] FEHLER: {e}"));
            exit_code(&e)
        }
    }
}

/// Fuehrt das Programm aus; der Rueckgabewert ist der Exit-Code: 0 Erfolg (oder Dry-run), 1 ELSTER
/// oder ERiC lief, aber es gibt keinen Beleg, 2 abgebrochen (Nutzung, Eingabe, Huerde, Sperre).
///
/// `ist_terminal`: ist `stdin` ein Terminal? Der echte Abruf verweigert sich sonst.
pub fn lauf_abruf(
    args: &[String],
    env: &AbrufUmgebung,
    stdin: &mut dyn BufRead,
    ist_terminal: bool,
    aus: &mut dyn Write,
) -> i32 {
    let args = match lies_args(args) {
        Ok(a) => a,
        Err(e) => {
            zeile(aus, &format!("[abruf] {e}"));
            zeile(aus, NUTZUNG);
            return 2;
        }
    };
    let auftrag = match AbrufAuftrag::neu(&args.idnr, &args.jahr, &env.hersteller_id, env.heute_jahr) {
        Ok(a) => a,
        Err(e) => {
            zeile(aus, &format!("[abruf] {e}"));
            if matches!(e, AbrufFehler::Eingabe { feld: "hersteller_id", .. }) {
                zeile(aus, "[abruf] Die Hersteller-ID kommt aus der Umgebungsvariable ELSTER_HERSTELLER_ID.");
            }
            return 2;
        }
    };
    let zertifikat = Zertifikat::neu(
        args.zertifikat.clone().unwrap_or_else(|| env.zertifikat_pfad.clone()),
        env.pin.clone(),
    );
    if args.aufruf == Aufruf::DryRun {
        return dry_run(&auftrag, &zertifikat, aus);
    }
    // ponytail: ohne --ziel liegt der Beleg unter dem Temp-Verzeichnis (Verzeichnis 0700); ein
    // festes Ziel im Home, wenn der Abruf zur Gewohnheit wird.
    let ziel = args.ziel.clone().unwrap_or_else(|| env.temp.clone());
    if !ziel.is_dir() {
        zeile(aus, "[abruf] Das Ziel ist kein Verzeichnis; abgebrochen VOR jedem ERiC-Aufruf.");
        return 2;
    }
    if !huerden(args.freigabe.as_deref(), stdin, ist_terminal, aus) {
        return 2;
    }
    abruf(&auftrag, &zertifikat, env, &ziel, aus)
}
