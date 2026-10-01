//! **Pin, kein Fix.** `meet_zustand([])` ist `Bestaetigt` — "leeres Aggregat = neutral"
//! (`store.py:52-57`). Das ist eine von zwei Bedingungen, die `_feste_zahl` vor jeder Zahl
//! prueft. Hier steht, was sie bei einem LEEREN Kegel wirklich tut: **nichts**.
//!
//! ```text
//! zustaende.len() < kegel.len()  ||  meet_zustand(zustaende) != Bestaetigt
//!        0 < 0 = falsch                     [] = Bestaetigt, also falsch
//! ```
//!
//! **Beide** Haelften der Bedingung sind bei leerem Kegel falsch. Der Meet sperrt dort nicht,
//! der Laengenvergleich auch nicht — die Zeile ist fuer einen leeren Kegel wirkungslos. Was die
//! vier abbrechenden Scheiben rettet, ist **allein die `slot_fn`** (bzw. Lage 1 bei `n_vor_gwg`),
//! nicht diese Bedingung.
//!
//! Das ist der Grund, warum der Laengenvergleich in `feste_zahl.rs` als *der Riegel, nicht
//! `meet_zustand`* kommentiert ist: er ist der staerkere der beiden, und er ist genau
//! um **ein** Feld zu schwach. `meet_zustand` allein waere bei einem leeren *und* bei einem
//! einfeldrigen Kegel blind — der Vergleich faengt wenigstens den einfeldrigen Fall.
//!
//! Der leere Kegel ist im Betrieb **nicht erreichbar** (Minimum 15 von 28 Feldern auf
//! `rentner_gesamt`, s. `znull_kegel.rs`); dieser Test braucht einen kuenstlichen Aufruf.
//!
//! Wird dieser Test rot, weil `meet_zustand` den leeren Kegel nicht mehr neutral laesst, ist
//! das eine **Aenderung am Vertrag** und gehoert gemeldet, nicht still nachgezogen.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bescheid::deklaration::Cfg;
use domain::{meet_zustand, Scheibe, Zustand};

/// Punkt 1: der Pin selber. `meet_zustand([])` ist `Bestaetigt`.
///
/// Derselbe Wert steht schon in `domain::meet::tests::leeres_aggregat_ist_bestaetigt`
/// (seit `3abbed6`). Er wird hier **nicht** verdoppelt, sondern zitiert -- was hier
/// dazukommt, ist die Folge fuer den Kegel, nicht der Wert selber.
#[test]
fn leerer_meet_ist_bestaetigt() {
    assert_eq!(meet_zustand(std::iter::empty()), Zustand::Bestaetigt);
    assert_eq!(meet_zustand([]), Zustand::Bestaetigt);
    assert_eq!(
        meet_zustand([Zustand::Bestaetigt]),
        Zustand::Bestaetigt,
        "das neutrale Element ist die Identitaet"
    );
}

/// Punkt 2: dieselbe Frage fuer alle fuenf Scheiben -- und die Antwort ist die scharfe.
///
/// Der Aufruf ist `feste_zahl(..., scheibe_felder = &[])`: der Kegel ist **künstlich leer**,
/// nicht durch `relevante_kegel_felder` geleert. Dann stehen auf beiden Seiten der Bedingung
/// Nullen:
///
/// ```text
/// zustaende.len() < kegel.len()   ||   meet_zustand(zustaende) != Bestaetigt
///        0        <      0  = falsch          []  =  Bestaetigt, also falsch
/// ```
///
/// **Keine der beiden Haelften sperrt.** Der Meet ist bei einem leeren Aggregat neutral, und
/// der Laengenvergleich vergleicht zwei Nullen. Die Zeile ist fuer diesen Aufruf wirkungslos.
///
/// Der wichtige Punkt: mit dem **echten** Scheibenkegel kann der Fall nicht entstehen --
/// `scheibe_felder = &[]` heisst in `feste_zahl`, dass die Relevanz-Sicht gar nicht erst
/// gefragt wird. Was die vier abbrechenden Scheiben rettet, ist die `slot_fn` (bzw. Lage 1 bei
/// `n_vor_gwg`), **nicht** diese Bedingung.
#[test]
fn bei_leerem_kegel_sperrt_keine_der_beiden_haelfen() {
    let zustaende: [Zustand; 0] = [];
    let kegel_len = 0; // `scheibe_felder = &[]` -- der kuenstlich leere Kegel

    // Die Meet-Haelfte: neutral, sie sperrt nie.
    assert_eq!(meet_zustand(zustaende), Zustand::Bestaetigt);
    assert_ne!(
        meet_zustand(zustaende),
        Zustand::Vorlaeufig,
        "waere der leere Meet Vorlaeufig, spuerte die Meet-Haelfte den leeren Kegel"
    );

    // Die Laengen-Haelfte: zwei Nullen, verglichen -- genau die Quelltext-Bedingung.
    let laengenvergleich_sperrt = zustaende.len() < kegel_len;
    assert!(
        !laengenvergleich_sperrt,
        "0 < 0 ist falsch -- der Laengenvergleich sperrt hier nicht"
    );

    // Und die Gegenprobe zur Gegenprobe: mit dem ECHTEN Scheibenkegel waere `0 < n` wahr --
    // nur wird der echte Kegel bei `scheibe_felder = &[]` gar nicht herangezogen.
    for s in [
        Scheibe::Ep,
        Scheibe::NVorGwg,
        Scheibe::AnGesamt,
        Scheibe::Gesamt,
        Scheibe::RentnerGesamt,
    ] {
        let echt = Cfg::fuer(s).kegel_roh().map_or(1, <[&str]>::len);
        assert!(
            0 < echt,
            "{s}: mit dem echten Kegel ({echt} Felder) wuerde der Vergleich sperren --              der leere Aufruf umgeht ihn, weil `scheibe_felder` leer ist"
        );
    }
}

/// Die Abgrenzung: bei einem **einfeldrigen** Kegel greift der Laengenvergleich sehr wohl.
///
/// Ohne diesen Fall waere die Aussage "die Zeile sperrt nicht" nur eine Behauptung ueber den
/// leeren Kegel. Hier steht, wo sie anfaengt zu greifen -- genau ab dem ersten fehlenden Feld.
#[test]
fn ab_einem_fehlenden_feld_greift_der_laengenvergleich() {
    let ein_feld: [Zustand; 1] = [Zustand::Bestaetigt];
    let kegel_len = 2;

    assert_eq!(
        meet_zustand(ein_feld),
        Zustand::Bestaetigt,
        "ein bestaetigtes Feld -- der Meet sperrt auch hier nicht"
    );
    assert!(
        ein_feld.len() < kegel_len,
        "aber der Laengenvergleich sperrt: {} < {kegel_len}",
        ein_feld.len()
    );
}
