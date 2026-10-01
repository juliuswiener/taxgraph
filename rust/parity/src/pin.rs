//! Bekannt leere Vergleichszeilen -- gepinnt, in BEIDE Richtungen streng.
//!
//! Ticket `parity-lauf-gruen-ohne-dass-die-zeile-rechnet`. Der Waechter aus
//! `9ad984f` macht jeden Block rot, in dem eine Vergleichszeile nie einen Wert
//! sah. Auf `main` sind damit sieben Bloecke dauerhaft rot, und "rot" verliert
//! die Bedeutung "etwas ist kaputtgegangen".
//!
//! Dieses Modul ist das Rust-Gegenstueck zu `pytest.mark.xfail(strict=True)`
//! (Muster: `tests/test_luf_gewinn_kz_fehlt.py`): eine Zeile, die heute
//! nachweislich leer ist, wird GELISTET -- mit Grund. Die Pruefung bleibt
//! dadurch streng, statt entschaerft zu werden:
//!
//! 1. Eine **nicht gelistete** Zeile, die null sieht, macht den Block rot.
//! 2. Eine **gelistete** Zeile, die einen Wert sieht, macht den Block rot.
//!    Dann ist der Grund weggefallen und die Liste veraltet.
//! 3. Eine gelistete Zeile, die es gar nicht mehr gibt, macht den Block rot.
//!
//! Eine Ausnahmeliste, die nur in eine Richtung prueft, waere genau der Fehler,
//! den das Ticket beschreibt: sie deckt die Luecke zu, statt sie zu benennen.
//!
//! # Der Korpus gehoert in die Liste
//!
//! Ob eine Zeile leer bleibt, haengt am Korpus. Gemessen am 2026-10-01:
//! `bescheid_blatt`/`reale_faelle` hat gegen [`KORPUS`] **8** leere Zeilen und
//! gegen die Sicht von `gdb-bau` (192 Verweise + 6 neue Faelle = 198 Dateien)
//! **6** -- `p23_ansonsten_einkuenfte` und `shared_dba_sonstige` rechnen dort.
//! Die Liste gilt darum fuer genau einen Korpus; ein anderer Lauf braucht eine
//! andere Liste, nicht eine laxere Pruefung.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Der Korpus, fuer den die Listen in den Suiten gelten: 192 Fall-Dateien,
/// aufgeloest `~/.cache/taxgraph-tmp/korpus-rt/faelle -> k2-bau/tmp/rt`.
///
/// Das ist **nicht** der Live-Korpus `~/.local/share/taxgraph/faelle`, der
/// sich waehrend einer Messung aendern kann (am 2026-10-01 von 192 auf 198).
pub const KORPUS: &str = "~/.cache/taxgraph-tmp/korpus-rt (192 Fall-Dateien)";

/// Zeilenname -> `nicht_leer`-Zaehler. 0 heisst "hat nie einen Wert gesehen".
pub type Gesehen = BTreeMap<&'static str, u64>;

/// Prueft einen Block in beide Richtungen. Panikt mit Klartext, wenn die Liste
/// nicht mehr zum Lauf passt.
///
/// `gesehen` enthaelt **nur** die Zeilen, die beurteilt werden sollen -- eine
/// reine Abdeckungszeile ohne echten Vergleich gehoert nicht hinein (s.
/// `konsistenz_paritaet.rs`, `vergleiche > 0`).
///
/// # Panics
///
/// Panikt, sobald die Liste nicht mehr zum Lauf passt: eine ungelistete Zeile
/// steht auf 0, eine gelistete rechnet wieder, oder eine gelistete Zeile gibt
/// es nicht mehr.
pub fn pruefe(block: &str, korpus: &str, liste: &[(&str, &str)], gesehen: &Gesehen) {
    let mut neu_leer: Vec<&str> = Vec::new();
    let mut wieder_da: Vec<(&str, &str)> = Vec::new();

    for (name, n) in gesehen {
        // Reihenfolge ist wesentlich: `(_, Some(..))` wuerde sonst auch `n == 0` schlucken und
        // JEDE gelistete Zeile als "rechnet wieder" melden. Genau das ist am 2026-10-01 passiert
        // -- der eigene Waechter war rot, wo er gruen sein musste.
        match (liste.iter().find(|(l, _)| l == name), *n) {
            (Some(_), 0) => {}
            (Some((l, grund)), _) => wieder_da.push((l, grund)),
            (None, 0) => neu_leer.push(name),
            (None, _) => {}
        }
    }
    let verschwunden: Vec<&str> = liste
        .iter()
        .map(|(l, _)| *l)
        .filter(|l| !gesehen.contains_key(l))
        .collect();

    let mut meldung = String::new();
    if !neu_leer.is_empty() {
        let _ = write!(
            meldung,
            "\n{block}: {} Zeile(n) stehen auf 0 und sind NICHT gelistet: [{}] -- \
             entweder rechnet der Block sie nicht mehr, oder die Liste ist unvollstaendig.",
            neu_leer.len(),
            neu_leer.join(", ")
        );
    }
    if !wieder_da.is_empty() {
        let _ = write!(
            meldung,
            "\n{block}: {} gelistete Zeile(n) rechnen WIEDER: [{}] -- Liste aktualisieren \
             (Eintrag streichen und den Grund pruefen).",
            wieder_da.len(),
            wieder_da
                .iter()
                .map(|(l, g)| format!("{l} ({g})"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !verschwunden.is_empty() {
        let _ = write!(
            meldung,
            "\n{block}: {} gelistete Zeile(n) gibt es nicht mehr: [{}] -- Liste aktualisieren.",
            verschwunden.len(),
            verschwunden.join(", ")
        );
    }
    assert!(
        meldung.is_empty(),
        "{meldung}\nKorpus dieser Liste: {korpus}\n\
         Grund je Eintrag und Korpus stehen an der Liste im Test."
    );
}
