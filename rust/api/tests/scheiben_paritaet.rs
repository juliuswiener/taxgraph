//! Bindet [`SCHEIBEN`] (`rust/api/src/routen/fall.rs:18`) an [`domain::Scheibe`]
//! (`rust/domain/src/veranlagung.rs:29`).
//!
//! Zwei unabhaengige Listen derselben fuenf Namen -- bisher von keinem Test gebunden. Wer eine
//! sechste Scheibe anlegt, muss an beide denken. Dieser Test ist der Ort, an dem sie
//! aufeinandertreffen.
//!
//! Die Bruecke gibt es schon: `Scheibe::als_str` und `FromStr for Scheibe`
//! (`veranlagung.rs:47-84`). Sie luegt nicht -- beide Richtungen stimmen mit `SCHEIBEN` ueberein.
//! Der Test baut keine zweite Bruecke, er haelt die vorhandene fest.
//!
//! Kein Produktfix: `SCHEIBEN` und das Enum bleiben getrennt. Der Test ist die Naht.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use api::routen::fall::SCHEIBEN;
use domain::Scheibe;

/// Erschoepfend ueber ALLE Varianten, ohne `_`-Zweig. Eine sechste Variante ohne Eintrag in
/// `SCHEIBEN` macht **diese Datei zum Compile-Fehler** -- die schaerfere Richtung: sie faellt
/// beim Bauen auf, nicht erst im Testlauf.
const fn jede_variante_abgedeckt(s: Scheibe) {
    match s {
        Scheibe::Ep
        | Scheibe::NVorGwg
        | Scheibe::AnGesamt
        | Scheibe::Gesamt
        | Scheibe::RentnerGesamt => {}
    }
}

#[test]
fn jede_scheibe_hat_genau_einen_namen_und_jeder_name_genau_eine_scheibe() {
    // Richtung 1: jeder Name links findet genau eine Variante rechts.
    let varianten: Vec<Scheibe> = SCHEIBEN
        .iter()
        .map(|n| n.parse::<Scheibe>().unwrap_or_else(|e| panic!("{n:?} aus SCHEIBEN: {e}")))
        .collect();
    for v in &varianten {
        jede_variante_abgedeckt(*v);
    }

    // Injektiv: zwei Namen duerfen nicht auf dieselbe Variante zeigen.
    let mut namen: Vec<&str> = varianten.iter().map(|v| v.als_str()).collect();
    namen.sort_unstable();
    let roh = namen.len();
    namen.dedup();
    assert_eq!(roh, namen.len(), "zwei SCHEIBEN-Namen teilen eine Variante: {namen:?}");

    // Richtung 2: jede Variante traegt genau einen Namen, und der steht wortgleich in SCHEIBEN.
    for n in SCHEIBEN {
        let v: Scheibe = n.parse().unwrap_or_else(|e| panic!("{n:?}: {e}"));
        assert!(SCHEIBEN.contains(&v.als_str()), "{n} -> {:?} fehlt in SCHEIBEN", v.als_str());
        assert_eq!(v.to_string(), n, "Display weicht von SCHEIBEN ab");
    }

    // Zahl links = Zahl rechts. Die rechte Seite erzwingt der Compiler oben (fuenf Arme); diese
    // Zahl ist die einzige, die eine sechste Scheibe von Hand mitziehen muss.
    assert_eq!(SCHEIBEN.len(), 5, "SCHEIBEN hat {} Namen", SCHEIBEN.len());
}
