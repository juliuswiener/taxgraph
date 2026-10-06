//! Das Programm `taxgraph-versand` (`versand.py::_cli`): die EINZIGE Aufrufstelle von [`sende`].
//!
//! Jeder Aufruf ist eine bewusste Handlung am Terminal. Kein Standard sendet: ohne `--dry-run`,
//! `--testversand` oder `--echtversand` bricht das Programm ab. Alles, was von aussen kommt
//! (Umgebung, stdin, Ausgabe), kommt als Parameter in [`lauf`]; so laeuft jeder Pfad im Test, ohne
//! Prozess-Umgebung und ohne Terminal.
//!
//! Abweichung zu Python, nur strenger: die zweite Eingabe der Freigabe verlangt ein TERMINAL als
//! stdin. Python nimmt auch `echo PHRASE | versand.py`; das hebt „nicht aus der Shell-History
//! kopieren" auf.

use std::fmt;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use crate::{sende, zusammenfassung, Modus, Zertifikat, ECHTVERSAND_FREIGABE};

const NUTZUNG: &str = "Nutzung: taxgraph-versand --xml DATEI --datenart ESt_2025 \
[--zertifikat PFAD] (--dry-run | --testversand | --echtversand --freigabe PHRASE)\n\
Siehe elster/VERSAND.md VOR der ersten Nutzung.";

/// Was das Programm aus der Prozess-Umgebung braucht. `Debug` zeigt die PIN nicht.
pub struct Umgebung {
    /// `$ELSTER_ZERTIFIKAT_PFAD` (leer, wenn nicht gesetzt); `--zertifikat` hat Vorrang.
    pub zertifikat_pfad: String,
    /// `$ELSTER_ZERTIFIKAT_PIN` (leer, wenn nicht gesetzt). Nie eine Kommandozeilenoption.
    pub pin: String,
    /// `libericapi.so` (`elster::find_eric_lib()`).
    pub eric_lib: Option<PathBuf>,
}

impl Umgebung {
    /// Liest die Prozess-Umgebung; ruft ERiC nicht.
    #[must_use]
    pub fn aus_prozess() -> Self {
        Self {
            zertifikat_pfad: std::env::var("ELSTER_ZERTIFIKAT_PFAD").unwrap_or_default(),
            pin: std::env::var("ELSTER_ZERTIFIKAT_PIN").unwrap_or_default(),
            eric_lib: elster::find_eric_lib(),
        }
    }
}

impl fmt::Debug for Umgebung {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Umgebung")
            .field("eric_lib", &self.eric_lib)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Aufruf {
    DryRun,
    Testversand,
    Echtversand,
}

struct Args {
    xml: PathBuf,
    datenart: String,
    zertifikat: Option<String>,
    aufruf: Aufruf,
    freigabe: Option<String>,
}

/// `--name WERT` und `--name=WERT`; ein Modus darf nur einmal vorkommen (wie die
/// `argparse`-Gruppe des Originals).
fn lies_args(args: &[String]) -> Result<Args, String> {
    let (mut xml, mut datenart, mut zertifikat, mut freigabe) = (None, None, None, None);
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
            "--xml" => {
                xml = Some(PathBuf::from(wert("--xml")?));
                continue;
            }
            "--datenart" => {
                datenart = Some(wert("--datenart")?);
                continue;
            }
            "--zertifikat" => {
                zertifikat = Some(wert("--zertifikat")?);
                continue;
            }
            "--freigabe" => {
                freigabe = Some(wert("--freigabe")?);
                continue;
            }
            "--dry-run" => Aufruf::DryRun,
            "--testversand" => Aufruf::Testversand,
            "--echtversand" => Aufruf::Echtversand,
            _ => return Err(format!("unbekannte Option {name:?}")),
        };
        if aufruf.replace(modus).is_some() {
            return Err("--dry-run, --testversand und --echtversand schliessen sich aus".to_owned());
        }
    }
    Ok(Args {
        xml: xml.ok_or("--xml ist Pflicht")?,
        datenart: datenart.ok_or("--datenart ist Pflicht")?,
        zertifikat,
        aufruf: aufruf.ok_or("einer von --dry-run, --testversand, --echtversand ist Pflicht")?,
        freigabe,
    })
}

fn zeile(aus: &mut dyn Write, text: &str) {
    // Eine Ausgabe, die nicht ankommt, aendert den Exit-Code nicht: der Versand ist dann schon
    // entschieden oder abgebrochen.
    let _ = writeln!(aus, "{text}");
}

fn dry_run(args: &Args, xml: &[u8], zertifikat: &Zertifikat, aus: &mut dyn Write) -> i32 {
    let info = zusammenfassung(xml, &args.datenart, &Modus::Testversand, zertifikat);
    zeile(aus, "[versand] DRY-RUN — es wird NICHTS gesendet.");
    zeile(aus, &format!("[versand]   datenart_version: {}", info.datenart_version));
    zeile(aus, &format!("[versand]   modus: {}", info.modus));
    zeile(
        aus,
        &format!("[versand]   merker_im_xml: {}", info.merker_im_xml.as_deref().unwrap_or("(keiner)")),
    );
    zeile(aus, &format!("[versand]   merker_konsistent: {}", info.merker_konsistent));
    zeile(aus, &format!("[versand]   xml_bytes: {}", info.xml_bytes));
    zeile(aus, &format!("[versand]   zertifikat_vorhanden: {}", info.zertifikat_vorhanden));
    if !info.merker_konsistent {
        zeile(
            aus,
            "[versand]   WARNUNG: XML und Modus passen nicht zusammen — ein echter Lauf wuerde HIER \
             mit XmlMerkerMismatch abbrechen, bevor irgendetwas gesendet wird.",
        );
    }
    0
}

/// Die zweite Huerde des Echtversands: dieselbe Phrase noch einmal, am Terminal getippt.
fn bestaetigt(stdin: &mut dyn BufRead, ist_terminal: bool, aus: &mut dyn Write) -> bool {
    if !ist_terminal {
        zeile(
            aus,
            "[versand] ECHTVERSAND VERWEIGERT — keine interaktive Bestaetigung moeglich (stdin ist \
             kein Terminal).",
        );
        return false;
    }
    let _ = write!(
        aus,
        "Echtversand ans Finanzamt — tippe zur Bestaetigung erneut ein (nicht kopieren):\n{ECHTVERSAND_FREIGABE}\n> "
    );
    let _ = aus.flush();
    let mut eingabe = String::new();
    match stdin.read_line(&mut eingabe) {
        Ok(0) | Err(_) => {
            zeile(aus, "[versand] ECHTVERSAND VERWEIGERT — keine interaktive Bestaetigung moeglich.");
            false
        }
        Ok(_) => {
            if eingabe.strip_suffix('\n').unwrap_or(&eingabe) == ECHTVERSAND_FREIGABE {
                true
            } else {
                zeile(aus, "[versand] ECHTVERSAND VERWEIGERT — Bestaetigung stimmte nicht ueberein.");
                false
            }
        }
    }
}

/// Der Modus fuer `sende`; `None`, wenn eine Huerde des Echtversands nicht genommen wurde.
fn modus_nach_huerden(
    args: &Args,
    stdin: &mut dyn BufRead,
    ist_terminal: bool,
    aus: &mut dyn Write,
) -> Option<Modus> {
    if args.aufruf == Aufruf::Testversand {
        return Some(Modus::Testversand);
    }
    if args.freigabe.as_deref() != Some(ECHTVERSAND_FREIGABE) {
        zeile(aus, "[versand] ECHTVERSAND VERWEIGERT — --freigabe muss woertlich sein:");
        zeile(aus, &format!("[versand]   {ECHTVERSAND_FREIGABE}"));
        return None;
    }
    bestaetigt(stdin, ist_terminal, aus).then(|| Modus::Echtversand {
        freigabe: ECHTVERSAND_FREIGABE.to_owned(),
    })
}

fn versand(
    args: &Args,
    xml: &[u8],
    zertifikat: &Zertifikat,
    env: &Umgebung,
    modus: &Modus,
    aus: &mut dyn Write,
) -> i32 {
    let antwort = match sende(env.eric_lib.as_deref(), xml, &args.datenart, zertifikat, modus) {
        Ok(a) => a,
        Err(e) => {
            zeile(aus, &format!("[versand] ABGEBROCHEN: {e}"));
            return 2;
        }
    };
    zeile(aus, &format!("[versand] rc={}", antwort.rc));
    let tn = antwort.telenummer();
    zeile(
        aus,
        &format!("[versand] Telenummer: {}", tn.as_deref().unwrap_or("(keine — kein Erfolg)")),
    );
    if antwort.erfolg() {
        return 0;
    }
    zeile(
        aus,
        "[versand] KEIN ERFOLG. Siehe elster/VERSAND.md, Abschnitt 'Wenn etwas schiefgeht' — NICHT \
         blind wiederholen.",
    );
    if antwort.rc != 0 {
        zeile(aus, "[versand] Rueckgabe von ERiC:");
        zeile(aus, &antwort.rueckgabe_xml);
    }
    zeile(aus, &format!("[versand] eric.log: {}", antwort.log_pfad.display()));
    1
}

/// Fuehrt das Programm aus; der Rueckgabewert ist der Exit-Code: 0 Erfolg (oder Dry-run), 1 ERiC
/// lief, aber kein Erfolg, 2 abgebrochen VOR einem Sendeaufruf (Nutzung, Huerde, Sperre).
///
/// `ist_terminal`: ist `stdin` ein Terminal? Der Echtversand verweigert sich sonst.
pub fn lauf(
    args: &[String],
    env: &Umgebung,
    stdin: &mut dyn BufRead,
    ist_terminal: bool,
    aus: &mut dyn Write,
) -> i32 {
    let args = match lies_args(args) {
        Ok(a) => a,
        Err(e) => {
            zeile(aus, &format!("[versand] {e}"));
            zeile(aus, NUTZUNG);
            return 2;
        }
    };
    let Ok(xml) = std::fs::read(&args.xml) else {
        zeile(aus, "[versand] Die XML-Datei laesst sich nicht lesen.");
        return 2;
    };
    let zertifikat = Zertifikat::neu(
        args.zertifikat.clone().unwrap_or_else(|| env.zertifikat_pfad.clone()),
        env.pin.clone(),
    );
    if args.aufruf == Aufruf::DryRun {
        return dry_run(&args, &xml, &zertifikat, aus);
    }
    let Some(modus) = modus_nach_huerden(&args, stdin, ist_terminal, aus) else {
        return 2;
    };
    versand(&args, &xml, &zertifikat, env, &modus, aus)
}
