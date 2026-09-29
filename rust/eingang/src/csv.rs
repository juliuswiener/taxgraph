//! Pythons `csv.DictReader(io.StringIO(text), delimiter=…)` mit dem Dialekt `excel` — als
//! Zustandsautomat nach `CPython` `Modules/_csv.c` (`parse_process_char`, `Reader_iternext`).
//!
//! Tragend fuer die Paritaet: `StringIO` liefert Zeilen nur an `\n`; ein einzelnes `\r` in einem
//! ungequoteten Feld beendet den Datensatz, und folgt danach noch ein Zeichen, bricht Python mit
//! `csv.Error` ab. Leere Zeilen werden uebersprungen; Anfuehrungszeichen duerfen Zeilen umspannen.
use std::collections::HashMap;

/// `csv.field_size_limit()` in der Voreinstellung.
const FELD_GRENZE: usize = 131_072;

/// `csv.Error` — Python bricht die ganze Uebernahme damit ab (HTTP 500).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CsvFehler {
    #[error("new-line character seen in unquoted field - do you need to open the file with newline=''?")]
    ZeilenumbruchImFeld,
    #[error("field larger than field limit ({FELD_GRENZE})")]
    FeldZuGross,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Zustand {
    StartDatensatz,
    StartFeld,
    ImFeld,
    ImQuotedFeld,
    QuoteImQuotedFeld,
    CrNlFressen,
}

struct Leser {
    trenner: char,
    zustand: Zustand,
    feld: String,
    feld_laenge: usize,
    felder: Vec<String>,
}

/// Ein Zeichen oder das Zeilenende-Signal (`EOL` in `_csv.c`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Z {
    C(char),
    Eol,
}

impl Leser {
    fn speichere(&mut self) {
        self.felder.push(std::mem::take(&mut self.feld));
        self.feld_laenge = 0;
    }

    fn fuege(&mut self, c: char) -> Result<(), CsvFehler> {
        if self.feld_laenge >= FELD_GRENZE {
            return Err(CsvFehler::FeldZuGross);
        }
        self.feld.push(c);
        self.feld_laenge += 1;
        Ok(())
    }

    fn ende_feld(&mut self, z: Z) {
        self.speichere();
        self.zustand = if z == Z::Eol { Zustand::StartDatensatz } else { Zustand::CrNlFressen };
    }

    fn zeichen(&mut self, z: Z) -> Result<(), CsvFehler> {
        let umbruch = matches!(z, Z::C('\n' | '\r') | Z::Eol);
        match self.zustand {
            Zustand::StartDatensatz => {
                if z == Z::Eol {
                    return Ok(());
                }
                if matches!(z, Z::C('\n' | '\r')) {
                    self.zustand = Zustand::CrNlFressen;
                    return Ok(());
                }
                self.zustand = Zustand::StartFeld;
                self.start_feld(z, umbruch)
            }
            Zustand::StartFeld => self.start_feld(z, umbruch),
            Zustand::ImFeld => match z {
                _ if umbruch => {
                    self.ende_feld(z);
                    Ok(())
                }
                Z::C(c) if c == self.trenner => {
                    self.speichere();
                    self.zustand = Zustand::StartFeld;
                    Ok(())
                }
                Z::C(c) => self.fuege(c),
                Z::Eol => Ok(()),
            },
            Zustand::ImQuotedFeld => match z {
                Z::Eol => Ok(()),
                Z::C('"') => {
                    self.zustand = Zustand::QuoteImQuotedFeld;
                    Ok(())
                }
                Z::C(c) => self.fuege(c),
            },
            Zustand::QuoteImQuotedFeld => match z {
                Z::C('"') => {
                    self.zustand = Zustand::ImQuotedFeld;
                    self.fuege('"')
                }
                Z::C(c) if c == self.trenner => {
                    self.speichere();
                    self.zustand = Zustand::StartFeld;
                    Ok(())
                }
                _ if umbruch => {
                    self.ende_feld(z);
                    Ok(())
                }
                Z::C(c) => {
                    // `strict=False`: das Zeichen gehoert zum Feld, weiter ungequotet.
                    self.zustand = Zustand::ImFeld;
                    self.fuege(c)
                }
                Z::Eol => Ok(()),
            },
            Zustand::CrNlFressen => match z {
                Z::C('\n' | '\r') => Ok(()),
                Z::Eol => {
                    self.zustand = Zustand::StartDatensatz;
                    Ok(())
                }
                Z::C(_) => Err(CsvFehler::ZeilenumbruchImFeld),
            },
        }
    }

    fn start_feld(&mut self, z: Z, umbruch: bool) -> Result<(), CsvFehler> {
        match z {
            _ if umbruch => {
                self.ende_feld(z);
                Ok(())
            }
            Z::C('"') => {
                self.zustand = Zustand::ImQuotedFeld;
                Ok(())
            }
            Z::C(c) if c == self.trenner => {
                self.speichere();
                Ok(())
            }
            Z::C(c) => {
                self.zustand = Zustand::ImFeld;
                self.fuege(c)
            }
            Z::Eol => Ok(()),
        }
    }
}

/// `csv.reader(io.StringIO(text), delimiter=trenner)` bis zum ersten Fehler: die Datensaetze
/// davor (inklusive `[]` fuer Leerzeilen) und der Fehler selbst. Python liest zeilenweise — ein
/// Aufrufer, der Zeile fuer Zeile weiterverarbeitet, sieht einen eigenen Fehler in einer FRUEHEREN
/// Zeile vor dem `csv.Error` einer spaeteren; deshalb beides zurueck.
///
/// ```
/// let (r, f) = eingang::csv::lies_datensaetze("a;\"b;c\"\n\n1;2\nx\ry\n", ';');
/// assert_eq!(r, vec![vec!["a", "b;c"], vec![], vec!["1", "2"]]);
/// assert!(f.is_some());
/// ```
#[must_use]
pub fn lies_datensaetze(text: &str, trenner: char) -> (Vec<Vec<String>>, Option<CsvFehler>) {
    let mut l = Leser { trenner, zustand: Zustand::StartDatensatz, feld: String::new(), feld_laenge: 0, felder: Vec::new() };
    let mut out = Vec::new();
    for zeile in text.split_inclusive('\n') {
        let schritt = zeile.chars().try_for_each(|c| l.zeichen(Z::C(c))).and_then(|()| l.zeichen(Z::Eol));
        if let Err(e) = schritt {
            return (out, Some(e));
        }
        if l.zustand == Zustand::StartDatensatz {
            out.push(std::mem::take(&mut l.felder));
        }
    }
    if l.feld_laenge != 0 || l.zustand == Zustand::ImQuotedFeld {
        l.speichere();
        out.push(std::mem::take(&mut l.felder));
    }
    (out, None)
}

/// Eine `DictReader`-Zeile: `feld -> Wert`; `None` = Pythons `restval` (Zeile zu kurz).
pub type Zeile = HashMap<String, Option<String>>;

/// `csv.DictReader`: Kopfzeile = erster Datensatz (auch ein leerer), danach leere Datensaetze
/// uebersprungen, doppelte Kopfnamen → letzter gewinnt. Ueberzaehlige Werte (Pythons
/// `restkey=None`) fallen weg — kein Aufrufer liest sie.
///
/// Wie [`lies_datensaetze`]: die Zeilen bis zum ersten Fehler, dann der Fehler. Scheitert schon
/// die Kopfzeile, ist der Kopf leer.
///
/// ```
/// let (kopf, zeilen, fehler) = eingang::csv::dict_reader("a;b\n1\n", ';');
/// assert_eq!(kopf, vec!["a", "b"]);
/// assert_eq!(zeilen[0]["b"], None);
/// assert!(fehler.is_none());
/// ```
#[must_use]
pub fn dict_reader(text: &str, trenner: char) -> (Vec<String>, Vec<Zeile>, Option<CsvFehler>) {
    let (saetze, fehler) = lies_datensaetze(text, trenner);
    let mut saetze = saetze.into_iter();
    let Some(kopf) = saetze.next() else { return (Vec::new(), Vec::new(), fehler) };
    let zeilen = saetze
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut z: Zeile = HashMap::new();
            for (k, v) in kopf.iter().zip(s.iter()) {
                z.insert(k.clone(), Some(v.clone()));
            }
            for k in kopf.iter().skip(s.len()) {
                z.insert(k.clone(), None);
            }
            z
        })
        .collect();
    (kopf, zeilen, fehler)
}

#[cfg(test)]
mod tests {
    use super::{lies_datensaetze, CsvFehler};

    #[test]
    fn cr_mitten_in_zeile_ist_fehler() {
        assert_eq!(lies_datensaetze("a\rb\n", ';'), (vec![], Some(CsvFehler::ZeilenumbruchImFeld)));
        assert_eq!(lies_datensaetze("a\r\nb\n", ';').0, vec![vec!["a"], vec!["b"]]);
    }

    #[test]
    fn offenes_quote_am_ende_liefert_teil() {
        assert_eq!(lies_datensaetze("a;\"b", ';').0, vec![vec!["a", "b"]]);
        assert_eq!(lies_datensaetze("\"a\nb\"\n", ';').0, vec![vec!["a\nb"]]);
    }

    #[test]
    fn doppeltes_quote_und_nachlauf() {
        assert_eq!(lies_datensaetze("\"a\"\"b\"x;c\n", ';').0, vec![vec!["a\"bx", "c"]]);
    }
}
