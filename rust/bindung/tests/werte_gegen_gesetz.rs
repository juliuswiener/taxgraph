//! Zwei Jahreswerte gegen das GESETZ, nicht gegen die YAML (Loeschplan V4, Stapel 1d, Zeile Z19). Ersatz fuer
//! `tests/test_renten_ertragsanteil_p22.py`.
//!
//! `params_zuordnung_hermetisch.rs` vergleicht den Zugriff mit der YAML-Datei: ein falscher Wert in `params/`
//! bleibt dort gruen, weil Zugriff und Datei dasselbe falsche Zahl meinen. Hier steht der Erwartungswert nicht im
//! Test. Er wird bei jedem Lauf aus dem eingefrorenen Gesetzestext unter `sources/gesetze-im-internet/` gelesen
//! und mit dem verglichen, was `bindung::Params` liefert (der Lesepfad, den `engine` nimmt). Ein Wert steht nur
//! in einer Fehlermeldung, wenn er mit Fundstelle aus der Quelle kommt; keine Zahl dieser Datei stammt aus dem
//! Gedaechtnis oder aus einem Python-Test.
//!
//! - Ertragsanteil: `estg_p22_2026-07-13.txt`, § 22 Nr. 1 Satz 3 Buchstabe a Doppelbuchstabe bb, Tabelle "Bei
//!   Beginn der Rente vollendetes Lebensjahr des Rentenberechtigten / Ertragsanteil in %". Die Tabelle steht in
//!   der Quelle als eine Zeile mit Bereichen ("17 bis 18 51") und einem offenen Ende ("ab 97 1"); der Parser
//!   [`ertragsanteil_im_gesetz`] expandiert sie je Alter. Verglichen wird jedes Alter von 0 bis 97 mit
//!   `Params::rente_ertragsanteil`.
//! - Renten-Werbungskosten-Pauschbetrag: `estg_p9a_2026-07-09.txt`, § 9a Satz 1 Nr. 3 ("von den Einnahmen im Sinne
//!   des § 22 Nummer 1, 1a und 5: ein Pauschbetrag von insgesamt N Euro").
//!
//! WAS DIE QUELLE BELEGT UND WAS NICHT. Beide Dateien sind "geltende Fassung", abgerufen am 2026-07-13 und
//! 2026-07-09. Sie belegen den Wert fuer VZ 2026. Fuer 2024 und 2025 liegt in `sources/` keine Fassung von § 9a und
//! keine Aenderungsvorschrift, die Nr. 3 beruehrt (die drei Aenderungsgesetze unter `sources/bgbl` nennen § 9a nicht);
//! ob Nr. 3 in diesen Jahren denselben Betrag trug, ist NICHT belegt. Der Test [`renten_wk_pauschbetrag_2024_und_
//! 2025_gleichen_dem_2026er`] haelt darum nur fest, dass die Jahresdateien einander gleichen. Er ist ein
//! Gleichheitswaechter, kein Rechtsbeleg. Der Ertragsanteil ist nicht vz-versioniert (fixiert am Rentenbeginn).
//!
//! Nicht gesehen: ob der Ertragsanteil im Rechenweg am richtigen Alter greift (Rentenbeginn nach vollendetem
//! Lebensjahr; die Rente-Tests in `engine`), und die Tabelle fuer Alter ueber 97 (die Quelle sagt "ab 97 1"; die
//! YAML hat keine Zeile darueber, `engine/src/zugriff/teil2/rente.rs` klemmt auf 97).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bindung::Params;
use domain::{Euro, Satz, Vz};
use regex::Regex;
use rust_decimal::Decimal;

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn quelle(datei: &str) -> String {
    let pfad = wurzel()
        .join("sources")
        .join("gesetze-im-internet")
        .join(datei);
    std::fs::read_to_string(&pfad)
        .unwrap_or_else(|e| panic!("{} nicht lesbar: {e}", pfad.display()))
}

// --------------------------------------------------------------------------------- Ertragsanteil

/// Die Ertragsanteil-Tabelle des Gesetzestexts, je Alter von 0 bis zum Alter des "ab"-Eintrags:
/// `Alter -> Prozent`. Ein Fehler heisst: der Text ist nicht mehr die Tabelle, die der Parser kennt.
///
/// Grammatik nach dem Anker `Ertragsanteil in %`: `A bis B P` | `A P` | `ab A P`; die Eintraege schliessen
/// luckenlos aneinander, `ab` steht zuletzt.
fn ertragsanteil_im_gesetz(text: &str) -> Result<BTreeMap<i64, i64>, String> {
    let woerter: Vec<&str> = text.split_whitespace().collect();
    let anker = woerter
        .windows(4)
        .position(|w| w == ["berechtigten", "Ertragsanteil", "in", "%"])
        .ok_or("Anker 'berechtigten Ertragsanteil in %' fehlt")?;
    let zahl = |i: usize| -> Result<i64, String> {
        woerter
            .get(i)
            .ok_or_else(|| format!("Text endet bei Wort {i}"))?
            .parse::<i64>()
            .map_err(|_| format!("keine Zahl: {:?}", woerter[i]))
    };
    let mut tabelle = BTreeMap::new();
    let mut i = anker + 4;
    let mut naechstes = 0_i64;
    loop {
        if woerter.get(i) == Some(&"ab") {
            let (von, prozent) = (zahl(i + 1)?, zahl(i + 2)?);
            if von != naechstes {
                return Err(format!("Luecke vor 'ab {von}': erwartet Alter {naechstes}"));
            }
            tabelle.insert(von, prozent);
            return Ok(tabelle);
        }
        let von = zahl(i)?;
        let (bis, prozent, laenge) = if woerter.get(i + 1) == Some(&"bis") {
            (zahl(i + 2)?, zahl(i + 3)?, 4)
        } else {
            (von, zahl(i + 1)?, 2)
        };
        if von != naechstes || bis < von {
            return Err(format!(
                "Eintrag {von}..{bis}: erwartet Beginn bei Alter {naechstes}"
            ));
        }
        for alter in von..=bis {
            tabelle.insert(alter, prozent);
        }
        naechstes = bis + 1;
        i += laenge;
    }
}

fn params() -> Params {
    Params::lade(&wurzel()).unwrap()
}

/// Jedes Alter der Tabelle im Gesetz gleich dem, was `Params` fuer den Rechenweg liefert.
#[test]
fn der_ertragsanteil_jedes_alters_ist_der_wert_im_gesetz() {
    let text = quelle("estg_p22_2026-07-13.txt");
    let gesetz = ertragsanteil_im_gesetz(&text).unwrap();
    let p = params();
    let mut abweichungen = Vec::new();
    for (alter, prozent) in &gesetz {
        let ist = p.rente_ertragsanteil(*alter).unwrap().map(Satz::get);
        let soll = Decimal::new(*prozent, 0);
        if ist != Some(soll) {
            abweichungen.push(format!("Alter {alter}: Gesetz {prozent} %, params/kohorten/rente_ertragsanteil_p22.yaml {ist:?}"));
        }
    }
    assert!(
        abweichungen.is_empty(),
        "Ertragsanteil weicht von § 22 Nr. 1 S. 3 a bb EStG (sources/gesetze-im-internet/estg_p22_2026-07-13.txt) ab:\n  {}",
        abweichungen.join("\n  ")
    );
    // Boden: die Tabelle ist gelesen, nicht leer. 0 bis 96 in Bereichen und "ab 97" = 98 Alter.
    assert_eq!(
        gesetz.len(),
        98,
        "der Parser liest nicht von Alter 0 bis 97"
    );
    assert_eq!(gesetz.keys().next(), Some(&0));
    assert_eq!(gesetz.keys().next_back(), Some(&97));
}

/// Der Parser an dem, was er nicht durchlaesst, und an einem erfundenen Auszug: ohne diese Probe waere "alle Alter
/// gleich" auch dann gruen, wenn der Parser eine falsche Tabelle liest.
#[test]
fn der_tabellenleser_expandiert_bereiche_und_lehnt_luecken_ab() {
    let t = "x berechtigten Ertragsanteil in % 0 bis 1 60 2 55 3 bis 4 50 ab 5 40 Satz 5 ...";
    let tab = ertragsanteil_im_gesetz(t).unwrap();
    assert_eq!(
        tab.into_iter().collect::<Vec<_>>(),
        [(0, 60), (1, 60), (2, 55), (3, 50), (4, 50), (5, 40)]
    );
    assert!(
        ertragsanteil_im_gesetz("berechtigten Ertragsanteil in % 0 bis 1 60 3 55 ab 4 40")
            .unwrap_err()
            .contains("erwartet Beginn bei Alter 2")
    );
    assert!(
        ertragsanteil_im_gesetz("berechtigten Ertragsanteil in % 0 bis 1 60 ab 3 40")
            .unwrap_err()
            .contains("Luecke vor 'ab 3'")
    );
    assert!(ertragsanteil_im_gesetz("kein Anker hier")
        .unwrap_err()
        .contains("Anker"));
    assert!(
        ertragsanteil_im_gesetz("berechtigten Ertragsanteil in % 0 bis 1 60").is_err(),
        "ohne 'ab' endet der Text zu frueh"
    );
}

/// Die Tabelle im Gesetz faellt nie: ein hoeheres Alter ist ein kleinerer Ertragsanteil. Das ist eine Eigenschaft
/// des Gesetzes, die der Parser nicht kennt; er faellt auf, wenn er zwei Zahlen vertauscht.
#[test]
fn die_gelesene_tabelle_faellt_nie() {
    let gesetz = ertragsanteil_im_gesetz(&quelle("estg_p22_2026-07-13.txt")).unwrap();
    let werte: Vec<i64> = gesetz.values().copied().collect();
    assert!(
        werte.windows(2).all(|w| w[0] >= w[1]),
        "die Tabelle faellt nicht durchgehend: {werte:?}"
    );
    assert!(werte[0] > werte[werte.len() - 1], "die Tabelle ist flach");
}

// ------------------------------------------------------------------ Renten-Werbungskosten-Pauschbetrag

/// Der Betrag aus § 9a Satz 1 Nr. 3 `EStG` in Euro, wie der Text ihn nennt.
fn renten_pauschbetrag_im_gesetz(text: &str) -> Result<i64, String> {
    let einzeilig = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let muster = Regex::new(
        r"3\. von den Einnahmen im Sinne des § 22 Nummer 1, 1a und 5: ein Pauschbetrag von insgesamt (\d+) Euro",
    )
    .unwrap();
    let treffer: Vec<_> = muster.captures_iter(&einzeilig).collect();
    match treffer.as_slice() {
        [t] => t[1].parse().map_err(|e| format!("{e}")),
        anders => Err(format!(
            "{} Treffer fuer § 9a Satz 1 Nr. 3 statt einem",
            anders.len()
        )),
    }
}

/// VZ 2026: der Wert im eingefrorenen Gesetz.
#[test]
fn der_renten_wk_pauschbetrag_2026_ist_der_wert_im_gesetz() {
    let betrag = renten_pauschbetrag_im_gesetz(&quelle("estg_p9a_2026-07-09.txt")).unwrap();
    assert_eq!(
        params().renten_wk_pauschbetrag(Vz::Vz2026).unwrap(),
        Euro::new(betrag),
        "§ 9a Satz 1 Nr. 3 EStG (sources/gesetze-im-internet/estg_p9a_2026-07-09.txt) nennt {betrag} Euro; \
         params/2026/renten_werbungskostenpauschbetrag_p9a.yaml weicht ab"
    );
}

/// NICHT belegt, nur festgehalten: 2024 und 2025 tragen denselben Wert wie 2026. Die Quelle ist die geltende
/// Fassung von 2026; ob Nr. 3 in den beiden Jahren so lautete, steht in `sources/` nirgends.
#[test]
fn renten_wk_pauschbetrag_2024_und_2025_gleichen_dem_2026er() {
    let p = params();
    let wert_2026 = p.renten_wk_pauschbetrag(Vz::Vz2026).unwrap();
    for vz in [Vz::Vz2024, Vz::Vz2025] {
        assert_eq!(
            p.renten_wk_pauschbetrag(vz).unwrap(),
            wert_2026,
            "{vz:?}: weicht vom Wert 2026 ab. Die Quelle belegt nur 2026; wer hier einen anderen Betrag setzt, \
             braucht die Fassung dieses Jahres in sources/"
        );
    }
}

#[test]
fn der_pauschbetrag_leser_findet_genau_eine_stelle() {
    let satz = "1. a) ... ein Pauschbetrag von 102 Euro;\n3. von den Einnahmen im Sinne des § 22 Nummer 1, 1a und 5: ein Pauschbetrag\n   von insgesamt 77 Euro.\n";
    assert_eq!(
        renten_pauschbetrag_im_gesetz(satz),
        Ok(77),
        "Zeilenumbruch im Satz, nicht die Nr. 1 b"
    );
    assert!(renten_pauschbetrag_im_gesetz("nichts")
        .unwrap_err()
        .contains("0 Treffer"));
    assert!(renten_pauschbetrag_im_gesetz(&format!("{satz}{satz}"))
        .unwrap_err()
        .contains("2 Treffer"));
}
