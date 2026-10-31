//! § 35a `EStG`, `p35a_haushaltsnahe`: die Rechnung im Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! Die Funktion ruft die Catala-Regel `Haushaltsnahe` (Cent) und rundet erst die Summe ab (Vault
//! `haushaltsnahe-catala-regel-wird-nie-gerechnet`, Entscheidung
//! `haushaltsnahe-ermaessigung-rechnet-die-catala-regel-und-rundet-erst-die-summe`). Die Gegenprobe
//! lief bisher nur in `rust/parity/tests/zugriff_teil1_paritaet.rs` und damit nur mit `PARITY=1`; die CI
//! faehrt Parity nicht. Gemessen am 2026-10-03 auf 583d70f1: die Mutation `x.max(0)` -> `x` im
//! Negativ-Topf (`topf` in `zugriff/teil1/ermaessigungen.rs`) laesst `cargo test -p engine` gruen
//! (142 passed, 0 failed). Mit diesen Tests wird sie rot: `negativer_topf_zaehlt_als_null` meldet 8 von 10 Faellen
//! (zum Beispiel `Handwerker -500 allein`: Rust -100, Orakel 0), 145 passed, 1 failed.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl ist die Ausgabe des Python-Orakels
//! `runner.catala_p35a_haushaltsnahe` auf denselben Eingaben (Orakel-Skript und Lauf: Anlagen zum
//! Bericht) UND stimmt mit der Gesetzes-Arithmetik ueberein (20 %, Deckel 510 / 4.000 / 1.200 EUR,
//! `sources/gesetze-im-internet/estg_p35a_2026-07-09.txt` und `estg_p35a_abs2_3_2026-07-09.txt`; Abs. 4
//! EU/EWR, Abs. 5 S. 3 unbare Zahlung, Abs. 3 S. 2 Foerderung). Das Orakel hat beides gegeneinander
//! geprueft und bricht bei einer Abweichung ab. Kein Wert ist aus dem Rust-Code abgelesen.
//!
//! Negative Toepfe: die Regel ergaebe bei -500 EUR -100 EUR Ermaessigung (die Steuer stiege); die
//! Nachbildung setzt den Topf vorher auf 0 (`ponytail` in `p35a_haushaltsnahe`, bis das Backlog
//! `negativer-aufwand-umgeht-pflichtfrage` negative Aufwaende bei der Eingabe abweist).
//!
//! HERKUNFT DER ZEILEN "halbierung" UND DER DREI ZEILEN "Alleinstehende ..." (Plan Nr. 116, Julius d-67 am
//! 2026-10-24, Vault `julius-entscheidet-p35a-ehepaar-wird-nicht-halbiert-zwei-alleinstehende-teilen-die-hoechstbetraege-
//! 2026-10-24`): NICHT das Python-Orakel, das die Summe halbierte, sondern von Hand aus § 35a Abs. 5 S. 4 `EStG` (zwei
//! Alleinstehende im selben Haushalt nehmen die Hoechstbetraege "insgesamt jeweils nur einmal" in Anspruch) und BMF-Schreiben
//! vom 2016-11-09 Rz. 53 (jeder rechnet seine tatsaechlichen Aufwendungen "bis zur Hoehe des haelftigen Abzugshoechstbetrages",
//! `sources/bmf/bmf_p35a_2016-11-09.txt:809-817`): 20 % der eigenen Kosten, gedeckelt bei 255 / 2.000 / 600 EUR. Die Summe
//! halbieren ist nie mehr als den Deckel halbieren (`min(a, c)/2 <= min(a, c/2)`) und gab bei 5.000 EUR Dienstleistungen 500 statt
//! 1.000 EUR. Ein zusammenveranlagtes Ehepaar teilt nichts (BMF Beispiel 9, `:819-831`): sein Fall steht in
//! `bescheid/tests/steuerermaessigungen_abbildung_hermetisch.rs` (Gruppe "ehepaar").
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use domain::Euro;
use engine::zugriff::teil1::ermaessigungen::{p35a_haushaltsnahe, P35aHaushaltsnaheEingabe};

/// Ein handgebauter Fall; `erwartet` in EUR ist die Ausgabe des Python-Orakels.
// Acht unabhaengige Eingaben, wie `P35aHaushaltsnaheEingabe`; kein Zustandsautomat.
#[allow(clippy::struct_excessive_bools)]
struct Zeile {
    gruppe: &'static str,
    name: &'static str,
    minijob: i64,
    dienstleistungen: i64,
    handwerker: i64,
    eu_ewr: bool,
    unbar: bool,
    gefoerdert: bool,
    mitveranlagt: bool,
    /// Zusammenveranlagt: die Antwort auf `mitveranlagt` zaehlt dann nicht (Ehepaar teilt nie).
    zusammen: bool,
    erwartet: i64,
}

const FAELLE: &[Zeile] = &[
    Zeile {
        gruppe: "gates",
        name: "Doku-Beispiel Rust: 500 / 5000 / 3000",
        minijob: 500,
        dienstleistungen: 5000,
        handwerker: 3000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 1700,
    },
    Zeile {
        gruppe: "negativ",
        name: "Handwerker -500 allein (AK3 des Eintrags)",
        minijob: 0,
        dienstleistungen: 0,
        handwerker: -500,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 0,
    },
    Zeile {
        gruppe: "negativ",
        name: "Minijob -500 allein",
        minijob: -500,
        dienstleistungen: 0,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 0,
    },
    Zeile {
        gruppe: "negativ",
        name: "Dienstleistung -500 allein",
        minijob: 0,
        dienstleistungen: -500,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 0,
    },
    Zeile {
        gruppe: "negativ",
        name: "Handwerker -500 neben positiven Toepfen",
        minijob: 500,
        dienstleistungen: 5000,
        handwerker: -500,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 1100,
    },
    Zeile {
        gruppe: "negativ",
        name: "Minijob -500 neben positiven Toepfen",
        minijob: -500,
        dienstleistungen: 5000,
        handwerker: 3000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 1600,
    },
    Zeile {
        gruppe: "negativ",
        name: "Dienstleistung -500 neben positiven Toepfen",
        minijob: 500,
        dienstleistungen: -500,
        handwerker: 3000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 700,
    },
    Zeile {
        gruppe: "negativ",
        name: "alle drei negativ",
        minijob: -100,
        dienstleistungen: -200,
        handwerker: -300,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 0,
    },
    Zeile {
        gruppe: "negativ",
        name: "Handwerker -500, gefoerdert",
        minijob: 0,
        dienstleistungen: 0,
        handwerker: -500,
        eu_ewr: true,
        unbar: true,
        gefoerdert: true,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 0,
    },
    Zeile {
        gruppe: "gates",
        name: "nicht EU/EWR gatet alles",
        minijob: 500,
        dienstleistungen: 5000,
        handwerker: 3000,
        eu_ewr: false,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 0,
    },
    Zeile {
        gruppe: "negativ",
        name: "nicht EU/EWR, negativ",
        minijob: 500,
        dienstleistungen: 5000,
        handwerker: -500,
        eu_ewr: false,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 0,
    },
    Zeile {
        gruppe: "gates",
        name: "ohne unbare Rechnung: nur der Minijob",
        minijob: 500,
        dienstleistungen: 5000,
        handwerker: 3000,
        eu_ewr: true,
        unbar: false,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 100,
    },
    Zeile {
        gruppe: "negativ",
        name: "ohne unbare Rechnung, Handwerker negativ",
        minijob: 500,
        dienstleistungen: 5000,
        handwerker: -500,
        eu_ewr: true,
        unbar: false,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 100,
    },
    Zeile {
        gruppe: "gates",
        name: "Handwerker gefoerdert: Abs. 3 faellt weg",
        minijob: 500,
        dienstleistungen: 5000,
        handwerker: 3000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: true,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 1100,
    },
    Zeile {
        gruppe: "gates",
        name: "Alleinstehende: 500 / 5000 / 3000 bleiben unter den halben Deckeln (255 / 2000 / 600), nichts wird halbiert",
        minijob: 500,
        dienstleistungen: 5000,
        handwerker: 3000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 1700,
    },
    Zeile {
        gruppe: "gates",
        name: "Alleinstehende, kleine Betraege (4 / 4 / 4): Summe 2,40 EUR unter den halben Deckeln, abgerundet 2",
        minijob: 4,
        dienstleistungen: 4,
        handwerker: 4,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 2,
    },
    Zeile {
        gruppe: "halbierung",
        name: "BMF-Beispiel 9 fuer zwei Alleinstehende: Minijob 2.800 -> 255, Handwerker 4.500 -> 600",
        minijob: 2800,
        dienstleistungen: 0,
        handwerker: 4500,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 855,
    },
    Zeile {
        gruppe: "ehepaar",
        name: "Ehepaar, Antwort ja (gespeichert), BMF-Beispiel 9: Minijob 2.800 -> 510, Handwerker 4.500 -> 900 = 1.410",
        minijob: 2800,
        dienstleistungen: 0,
        handwerker: 4500,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: true,
        erwartet: 1410,
    },
    Zeile {
        gruppe: "ehepaar",
        name: "Ehepaar, Antwort nein: 1.410",
        minijob: 2800,
        dienstleistungen: 0,
        handwerker: 4500,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: true,
        erwartet: 1410,
    },
    Zeile {
        gruppe: "ehepaar",
        name: "Ehepaar, Antwort ja, nur Dienstleistungen 30.000: voller Deckel 4.000, nicht 2.000",
        minijob: 0,
        dienstleistungen: 30_000,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: true,
        erwartet: 4000,
    },
    Zeile {
        gruppe: "ehepaar",
        name: "Ehepaar, Antwort ja, alle drei am vollen Deckel: 5.710, nicht 2.855",
        minijob: 3000,
        dienstleistungen: 30_000,
        handwerker: 10_000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: true,
        erwartet: 5710,
    },
    Zeile {
        gruppe: "halbierung",
        name: "dieselben Kosten ohne Teilung: 510 + 900",
        minijob: 2800,
        dienstleistungen: 0,
        handwerker: 4500,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 1410,
    },
    Zeile {
        gruppe: "halbierung",
        name: "nur Dienstleistungen 5.000: 20 % = 1.000 unter dem halben Deckel (Summe halbieren gaebe 500)",
        minijob: 0,
        dienstleistungen: 5000,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 1000,
    },
    Zeile {
        gruppe: "halbierung",
        name: "nur Dienstleistungen 30.000: halber Deckel 2.000",
        minijob: 0,
        dienstleistungen: 30_000,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 2000,
    },
    Zeile {
        gruppe: "halbierung",
        name: "nur Handwerker 10.000: halber Deckel 600",
        minijob: 0,
        dienstleistungen: 0,
        handwerker: 10_000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 600,
    },
    Zeile {
        gruppe: "halbierung",
        name: "Minijob 1.274 (20 % = 254,80) unter dem halben Deckel: 254",
        minijob: 1274,
        dienstleistungen: 0,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 254,
    },
    Zeile {
        gruppe: "halbierung",
        name: "Minijob 1.275 genau am halben Deckel: 255",
        minijob: 1275,
        dienstleistungen: 0,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 255,
    },
    Zeile {
        gruppe: "halbierung",
        name: "Minijob 1.276 ueber dem halben Deckel: 255",
        minijob: 1276,
        dienstleistungen: 0,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 255,
    },
    Zeile {
        gruppe: "halbierung",
        name: "Dienstleistungen 9.999 (20 % = 1.999,80) unter dem halben Deckel: 1.999",
        minijob: 0,
        dienstleistungen: 9999,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 1999,
    },
    Zeile {
        gruppe: "halbierung",
        name: "Dienstleistungen 10.001 ueber dem halben Deckel: 2.000",
        minijob: 0,
        dienstleistungen: 10_001,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 2000,
    },
    Zeile {
        gruppe: "halbierung",
        name: "Handwerker 2.999 (20 % = 599,80) unter dem halben Deckel: 599",
        minijob: 0,
        dienstleistungen: 0,
        handwerker: 2999,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 599,
    },
    Zeile {
        gruppe: "halbierung",
        name: "Handwerker 3.001 ueber dem halben Deckel: 600",
        minijob: 0,
        dienstleistungen: 0,
        handwerker: 3001,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 600,
    },
    // Weit zwischen dem halben und dem vollen Deckel, je Topf allein: die "knapp darueber"-Faelle liegen unter 1 EUR
    // ueber dem halben Deckel, und die Abrundung der Summe verdeckt dort, ob der Vergleich gegen den halben oder den
    // vollen Deckel laeuft (Mutation: Vergleich im erzeugten C gegen den vollen Deckel, Ergebnis bleibt halbiert).
    Zeile {
        gruppe: "halbierung",
        name: "Minijob 2.000 (20 % = 400) weit ueber dem halben Deckel, unter dem vollen: 255",
        minijob: 2000,
        dienstleistungen: 0,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 255,
    },
    Zeile {
        gruppe: "halbierung",
        name: "Dienstleistungen 15.000 (20 % = 3.000) weit ueber dem halben Deckel, unter dem vollen: 2.000",
        minijob: 0,
        dienstleistungen: 15_000,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 2000,
    },
    Zeile {
        gruppe: "halbierung",
        name: "Handwerker 5.000 (20 % = 1.000) weit ueber dem halben Deckel, unter dem vollen: 600",
        minijob: 0,
        dienstleistungen: 0,
        handwerker: 5000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 600,
    },
    Zeile {
        gruppe: "halbierung",
        name: "alle drei am halben Deckel: 255 + 2.000 + 600 = 2.855 (Grenzfall, gleich der halbierten Summe)",
        minijob: 3000,
        dienstleistungen: 30_000,
        handwerker: 10_000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: true,
        zusammen: false,
        erwartet: 2855,
    },
    Zeile {
        gruppe: "summe",
        name: "Toepfe (4,4,4): Summe 2,40 EUR, nicht je Topf abgerundet",
        minijob: 4,
        dienstleistungen: 4,
        handwerker: 4,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 2,
    },
    Zeile {
        gruppe: "summe",
        name: "Toepfe (0,1,5999): Summe 1.200 EUR, nicht 1.199",
        minijob: 0,
        dienstleistungen: 1,
        handwerker: 5999,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 1200,
    },
    Zeile {
        gruppe: "deckel",
        name: "Deckel Minijob: 2551 -> 510",
        minijob: 2551,
        dienstleistungen: 0,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 510,
    },
    Zeile {
        gruppe: "deckel",
        name: "Minijob genau am Deckel: 2550 -> 510",
        minijob: 2550,
        dienstleistungen: 0,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 510,
    },
    Zeile {
        gruppe: "deckel",
        name: "Minijob unter dem Deckel: 2549 -> 509",
        minijob: 2549,
        dienstleistungen: 0,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 509,
    },
    Zeile {
        gruppe: "deckel",
        name: "Deckel Dienstleistung: 20001 -> 4000",
        minijob: 0,
        dienstleistungen: 20_001,
        handwerker: 0,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 4000,
    },
    Zeile {
        gruppe: "deckel",
        name: "Deckel Handwerker: 6001 -> 1200",
        minijob: 0,
        dienstleistungen: 0,
        handwerker: 6001,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 1200,
    },
    Zeile {
        gruppe: "deckel",
        name: "alle drei am Deckel",
        minijob: 3000,
        dienstleistungen: 30_000,
        handwerker: 10_000,
        eu_ewr: true,
        unbar: true,
        gefoerdert: false,
        mitveranlagt: false,
        zusammen: false,
        erwartet: 5710,
    },
];

fn rechne(z: &Zeile) -> i64 {
    let e = P35aHaushaltsnaheEingabe {
        hh_minijob_aufwendungen: Euro::new(z.minijob),
        hh_dienstleistungen: Euro::new(z.dienstleistungen),
        hh_handwerker_arbeitskosten: Euro::new(z.handwerker),
        hh_in_eu_ewr: z.eu_ewr,
        hh_rechnung_unbar: z.unbar,
        hh_handwerker_gefoerdert: z.gefoerdert,
        p35a_mitveranlagung: z.mitveranlagt,
        zusammenveranlagt: z.zusammen,
    };
    p35a_haushaltsnahe(&e).unwrap().get()
}

/// Alle Faelle der Gruppe durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn pruefe(gruppe: &str) {
    let faelle: Vec<&Zeile> = FAELLE.iter().filter(|z| z.gruppe == gruppe).collect();
    assert!(!faelle.is_empty(), "Gruppe {gruppe} ist leer");
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|z| {
            let got = rechne(z);
            (got != z.erwartet).then(|| format!("{}: Rust {got}, Orakel {}", z.name, z.erwartet))
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len(),
        faelle.len()
    );
}

/// Ein negativer Topf rechnet nicht schlechter als 0 (`x.max(0)`): allein, neben positiven Toepfen, mit
/// Foerderung, ohne Rechnung und ausserhalb EU/EWR. Die Mutation `x.max(0)` -> `x` macht die Faelle rot, in
/// denen ein Topf negativ UND die Rechnung nicht schon vorher auf 0 steht.
#[test]
fn negativer_topf_zaehlt_als_null() {
    pruefe("negativ");
}

/// Die Deckel 510 / 4.000 / 1.200 EUR greifen je Topf, knapp darunter und genau darauf.
#[test]
fn deckel_je_topf() {
    pruefe("deckel");
}

/// Die Summe wird einmal abgerundet, nicht je Topf: (4,4,4) -> 2 EUR (je Topf waeren es 0),
/// (0,1,5999) -> 1.200 EUR (je Topf 1.199).
#[test]
fn summe_wird_abgerundet_nicht_je_topf() {
    pruefe("summe");
}

/// Die Gates: EU/EWR (Abs. 4), unbare Rechnung (Abs. 5 S. 3, nur Abs. 2/3), Foerderung (Abs. 3 S. 2) und die
/// Antwort "ja" auf `p35a_mitveranlagung` (zwei Alleinstehende; die Deckel halbieren, nicht die Summe) sowie das Doku-Beispiel.
#[test]
fn gates_und_halbierung() {
    pruefe("gates");
}

/// Zwei Alleinstehende im selben Haushalt (§ 35a Abs. 5 S. 4, BMF Rz. 53): 20 % der eigenen Kosten laufen gegen die HALBEN
/// Deckel 255 / 2.000 / 600 EUR, je Topf, knapp darunter, genau darauf, knapp darueber und weit darueber (unter dem vollen Deckel). Die Summe wird nicht halbiert: bei
/// 5.000 EUR Dienstleistungen sind es 1.000 EUR, nicht 500. Plan Nr. 116.
#[test]
fn alleinstehende_teilen_die_hoechstbetraege_nicht_die_summe() {
    pruefe("halbierung");
}

/// Ein zusammenveranlagtes Ehepaar teilt nie, auch mit gespeichertem "ja" (BMF Rz. 53 Beispiel 9: 510 + 900 = 1.410). Plan Nr. 116.
#[test]
fn ehepaar_teilt_die_hoechstbetraege_nie() {
    pruefe("ehepaar");
}
