//! Leser und Vergleich fuer `orakel.txt`, von zwei Tests per `#[path]` eingebunden:
//! `catala-sys/tests/catala_orakel.rs` (Rust-Huelle `catala_sys::*`) und
//! `engine/tests/catala_orakel_replay.rs` (Module von `engine`). Ein Format, ein Leser: laeuft der
//! Generator (`examples/orakel_erzeugen.rs`) und ein Leser auseinander, faellt das in beiden Tests auf.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use catala_sys::{CatalaFehler, FestzusetzendeEstErgebnis, Vz};

/// Eine Zeile der Datei: `funktion | name=wert ... | name=wert ...` oder `... | FEHLER`.
pub struct Zeile {
    pub nr: usize,
    pub funktion: String,
    pub ein: BTreeMap<String, String>,
    /// `None` = der Interpreter meldete einen Fehler.
    pub aus: Option<Vec<(String, i128)>>,
}

fn paare(text: &str) -> Result<Vec<(String, String)>, String> {
    text.split_whitespace()
        .map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .ok_or_else(|| format!("kein name=wert: {p:?}"))
        })
        .collect()
}

fn parse_zeile(nr: usize, zeile: &str) -> Result<Zeile, String> {
    let teile: Vec<&str> = zeile.split(" | ").collect();
    let &[funktion, ein, aus] = teile.as_slice() else {
        return Err(format!("Zeile {nr}: nicht drei Teile"));
    };
    let aus = if aus.trim() == "FEHLER" {
        None
    } else {
        let mut werte = Vec::new();
        for (k, v) in paare(aus)? {
            werte.push((k, v.parse::<i128>().map_err(|_| format!("Zeile {nr}: kein Wert: {v:?}"))?));
        }
        Some(werte)
    };
    Ok(Zeile { nr, funktion: funktion.trim().to_owned(), ein: paare(ein)?.into_iter().collect(), aus })
}

/// Liest die Datei: Kopf-Hash und alle Zeilen. Eine fehlende Datei ist ein Fehler, kein leerer Lauf.
pub fn lies_aus(pfad: &Path) -> (String, Vec<Zeile>) {
    let text = std::fs::read_to_string(pfad)
        .unwrap_or_else(|e| panic!("{} fehlt oder ist unlesbar ({e}) -- `make catala-orakel`", pfad.display()));
    let mut hash = String::new();
    let mut zeilen = Vec::new();
    for (i, zeile) in text.lines().enumerate() {
        if let Some(h) = zeile.strip_prefix("# regeln-hash: ") {
            h.trim().clone_into(&mut hash);
        } else if !zeile.starts_with('#') && !zeile.trim().is_empty() {
            zeilen.push(parse_zeile(i + 1, zeile).unwrap_or_else(|f| panic!("{f}")));
        }
    }
    (hash, zeilen)
}

/// Die Eingaben einer Zeile, jede genau einmal verbraucht: eine fehlende oder uebrige Eingabe
/// (Generator und Abspieler sind auseinandergelaufen) ist ein Fehler, kein stilles Weglassen.
pub struct Ein(pub BTreeMap<String, String>);

impl Ein {
    pub fn text(&mut self, name: &str) -> Result<String, String> {
        self.0.remove(name).ok_or_else(|| format!("Eingabe {name} fehlt"))
    }

    pub fn zahl(&mut self, name: &str) -> Result<i64, String> {
        self.text(name)?.parse().map_err(|_| format!("Eingabe {name} ist keine Zahl"))
    }

    pub fn wahr(&mut self, name: &str) -> Result<bool, String> {
        self.text(name)?.parse().map_err(|_| format!("Eingabe {name} ist kein Wahrheitswert"))
    }

    pub fn vz(&mut self, name: &str) -> Result<Vz, String> {
        let jahr = u16::try_from(self.zahl(name)?).map_err(|_| format!("Eingabe {name} ist kein Jahr"))?;
        Vz::try_from(jahr).map_err(|e| e.to_string())
    }

    /// `zaehler/nenner`.
    pub fn bruch(&mut self, name: &str) -> Result<(i64, u64), String> {
        let text = self.text(name)?;
        let (z, n) = text.split_once('/').ok_or_else(|| format!("Eingabe {name} ist kein Bruch"))?;
        let fehler = || format!("Eingabe {name} ist kein Bruch");
        Ok((z.parse().map_err(|_| fehler())?, n.parse().map_err(|_| fehler())?))
    }

    pub fn leer(self) -> Result<(), String> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(format!("unbenutzte Eingaben: {:?}", self.0.keys().collect::<Vec<_>>()))
        }
    }
}

/// Aeussere `Err` = der Aufruf scheiterte; innere `Err` = eine gelesene Ausgabe passt nicht in `i64`.
pub type Ergebnis = Result<Vec<(&'static str, Result<i64, CatalaFehler>)>, CatalaFehler>;

pub fn einzel(name: &'static str, r: Result<i64, CatalaFehler>) -> Ergebnis {
    r.map(|w| vec![(name, Ok(w))])
}

pub fn est_ausgaben(r: Result<FestzusetzendeEstErgebnis, CatalaFehler>) -> Ergebnis {
    r.map(|e| {
        vec![
            ("summe_der_einkuenfte", e.summe_der_einkuenfte_cent()),
            ("gesamtbetrag_der_einkuenfte", e.gesamtbetrag_der_einkuenfte_cent()),
            ("einkommen", e.einkommen_cent()),
            ("zu_versteuerndes_einkommen", e.zu_versteuerndes_einkommen_cent()),
            ("tarifliche_est", e.tarifliche_est_cent()),
            ("festzusetzende_est", e.festzusetzende_est_cent()),
        ]
    })
}

/// `None` = Rust und Interpreter stimmen ueberein. Ein Rechenfehler des Interpreters (zum Beispiel
/// Teilung durch null) muss in Rust als `Assertion` ankommen. Passt ein Wert des Interpreters nicht in
/// `i64`, muss Rust genau das melden (`Ueberlauf`), statt eine falsche Zahl zu liefern.
pub fn abweichung(z: &Zeile, rust: &Ergebnis) -> Option<String> {
    match (&z.aus, rust) {
        (None, Err(CatalaFehler::Assertion)) => None,
        (None, Err(f)) => Some(format!("Catala meldet einen Fehler, Rust meldet {f:?} statt Assertion")),
        (None, Ok(_)) => Some("Catala meldet einen Fehler, Rust liefert Werte".to_owned()),
        (Some(_), Err(f)) => Some(format!("Catala liefert Werte, Rust scheitert: {f}")),
        (Some(erwartet), Ok(ist)) => {
            if erwartet.len() != ist.len() {
                return Some(format!("{} Ausgaben erwartet, Rust liefert {}", erwartet.len(), ist.len()));
            }
            erwartet.iter().find_map(|(name, soll)| {
                let Some((_, got)) = ist.iter().find(|(n, _)| n == name) else {
                    return Some(format!("Ausgabe {name} fehlt in Rust"));
                };
                let gleich = match i64::try_from(*soll) {
                    Ok(w) => *got == Ok(w),
                    Err(_) => matches!(got, Err(CatalaFehler::Ueberlauf(_))),
                };
                (!gleich).then(|| format!("{name}: Catala {soll}, Rust {got:?}"))
            })
        }
    }
}
