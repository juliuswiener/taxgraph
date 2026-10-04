//! Ueberlauf-Naht der Parameter-Caps (`engine::zugriff::teil1`, Stellen Pd3 und Pf2–Pf4, Pf6):
//! die Euro->Cent-Umrechnung der Cap-Werte aus `params/` — im Standardlauf, ohne Python.
//!
//! Die Mutanten (Bericht h8-hermetisch5): Pf2/Pf3/Pf4 ersetzen in `werbungskosten.rs` je ein
//! `in_cent(r.jahrespauschale | r.tagespauschale_pro_tag | r.tagespauschale_hoechstbetrag)?`
//! durch `Ok(Cent::new((…).get().wrapping_mul(100)))`, Pf6 dasselbe für
//! `in_cent(r.hoechstbetrag_ohne_kfz)`, Pd3 für `in_cent(k.hoechstbetrag)` in
//! `ermaessigungen.rs`; N14 in `teil2/rente.rs` (`zehntel`: `i64::try_from(v).ok()` -> `Some(unwrap_or(0))`,
//! ein Satz ausserhalb `i64` wird still zu 0 statt zur Sperre). Alle sechs überleben den Bestand und jeden Fall mit echten Parametern:
//! die echten Caps (1.260, 6, 1.900, 4.500 EUR) bleiben mit × 100 innerhalb `i64`. Erreichbar
//! ist die Stelle nur mit gepatchter Parameterdatei — dieselbe Bauweise wie h4 für die
//! EP-Deckel, dort wurde der Cap gesenkt, hier wird er gehoben.
//!
//! SCHWÄCHE dieses Harness, im Bericht benannt: die gepatchte Datei ist ein Datensatz, kein
//! Quelltext-Mutant. Der Test weist nach, dass die Prüfung AN DIESER STELLE wirkt und dass ohne
//! sie ein gewickelter Wert durchläuft; er weist nicht nach, dass der echte params-Betrieb je
//! dorthin kommt.
//!
//! Je Stelle zwei Zustände: unmutiert meldet die Stelle `Ueberlauf("Euro->Cent")`; der Wert,
//! den die Mutante liefern würde, steht als Python-Folie im Kommentar (`orakel_pz5.py`,
//! runner.py mit demselben Patch im `lru_cache`-Wrapper; die Caps fliessen in Deckel, daher
//! rechnet Python durch). Art: „nur ein Zwischenprodukt ausserhalb i64“ — fail-closed-
//! Konvention, keine Python-Stütze; die Sperre selbst ist der Unterschied.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::path::Path;

use bindung::Params;
use domain::{Euro, Km, Vz};
use engine::zugriff::teil1::ermaessigungen::{p24a_altersentlastung, P24aAltersentlastungEingabe};
use engine::zugriff::teil1::fehler::EngineFehler;
use engine::zugriff::teil1::werbungskosten::{
    entfernungspauschale, raumkosten, EntfernungspauschaleEingabe, RaumkostenEingabe,
};
use engine::zugriff::teil2::rente::{renten_einkuenfte, RentenEingabe, Rentenart};
use engine::zugriff::teil2::EngineFehler as Teil2Fehler;
use rust_decimal::Decimal;

// Die Obergrenze 92233720368547759 in den Patches unten ist `i64::MAX / 100 + 1` EUR: × 100 schon ausserhalb `i64`, als Euro
// noch darstellbar.

fn kopiere_params(from: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for eintrag in std::fs::read_dir(from).unwrap() {
        let eintrag = eintrag.unwrap();
        let ziel = nach.join(eintrag.file_name());
        if eintrag.file_type().unwrap().is_dir() {
            kopiere_params(&eintrag.path(), &ziel);
        } else {
            std::fs::copy(eintrag.path(), &ziel).unwrap();
        }
    }
}

/// `Params` aus einer Temp-Kopie von `params/`, in der EINE Datei an genau den gegebenen
/// Stellen ersetzt ist. Jeder Anker muss in der Originaldatei genau einmal treffen — sonst ist
/// der Test gegen eine geänderte Dateiformatierung taub, und das soll er laut sein.
fn params_gepatcht(zelle: &str, datei: &str, stellen: &[(&str, &str)]) -> Params {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wurzel =
        std::env::temp_dir().join(format!("taxgraph-h8-pz-{zelle}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wurzel);
    kopiere_params(&repo.join("params"), &wurzel.join("params"));
    let pfad = wurzel.join("params").join(datei);
    let mut text = std::fs::read_to_string(&pfad).unwrap();
    for (alt, neu) in stellen {
        assert_eq!(
            text.matches(*alt).count(),
            1,
            "Anker nicht einwandfrei in {datei}: {alt:?}"
        );
        text = text.replacen(alt, neu, 1);
    }
    std::fs::write(&pfad, text).unwrap();
    let p = Params::lade(&wurzel).unwrap();
    std::fs::remove_dir_all(&wurzel).unwrap();
    p
}

fn gepatcht(zelle: &str, datei: &str, stellen: &[(&str, &str)]) -> (Params, String) {
    (
        params_gepatcht(zelle, datei, stellen),
        format!("{zelle}: {datei}"),
    )
}

fn meldet_ueberlauf(r: Result<Euro, EngineFehler>, stelle: &str) {
    let marke = match r {
        Ok(c) => format!("Zahl {}", c.get()),
        Err(EngineFehler::Ueberlauf(m)) => format!("Ueberlauf({m})"),
        Err(other) => format!("{other:?}"),
    };
    assert_eq!(marke, "Ueberlauf(Euro->Cent)", "{stelle}");
}

fn az_eingabe(gewaehlt: bool, tatsaechlich: i64, tage: i64) -> RaumkostenEingabe {
    RaumkostenEingabe {
        veranlagungszeitraum: Vz::Vz2025,
        arbeitszimmer_vorhanden: gewaehlt,
        ist_mittelpunkt: gewaehlt,
        tatsaechliche_aufwendungen: Euro::new(tatsaechlich),
        jahrespauschale_gewaehlt: gewaehlt,
        monate_ohne_mittelpunkt: 0,
        homeoffice_tage: tage,
    }
}

/// Pf2–Pf4: die drei Caps des Arbeitszimmers, je Stelle einzeln gepatcht.
#[test]
fn arbeitszimmer_caps_aus_params_melden_ueberlauf() {
    let datei = "2025/arbeitszimmer_homeoffice.yaml";
    // Pf2: die Jahrespauschale wird zum Riesen (Cap gewaehlt, tatsaechliche Aufwendungen
    // 200 Mio EUR blieben darunter). Folie: 92.233.720.368.547.759 EUR — der Cap geht hier
    // direkt ins Ergebnis, deshalb ist die Zahl so gross.
    let (p, stelle) = gepatcht(
        "pf2",
        datei,
        &[(
            "jahrespauschale:\n  wert: 1260\n",
            "jahrespauschale:\n  wert: 92233720368547759\n",
        )],
    );
    let r = raumkosten(&az_eingabe(true, 200_000_000, 0), &p);
    meldet_ueberlauf(r, &format!("{stelle} (Pf2)"));
    // Pf3: der Tageswert selbst, bei 1 Tag. Der Hoechstbetrag wird auf 10^15 EUR gehoben, damit
    // er die Summe nicht klemmt — sonst pruefte der Fall STILL die Stelle Pf4 statt Pf3 (die
    // Klemme `min(summe, hoechstbetrag)` wertet den HOECHEREN der beiden gepatchten Werte).
    // Folie: 10^15 EUR.
    let (p, stelle) = gepatcht(
        "pf3",
        datei,
        &[
            (
                "tagespauschale_pro_tag:\n  wert: 6\n",
                "tagespauschale_pro_tag:\n  wert: 92233720368547759\n",
            ),
            (
                "tagespauschale_hoechstbetrag:\n  wert: 1260\n",
                "tagespauschale_hoechstbetrag:\n  wert: 1000000000000000\n",
            ),
        ],
    );
    let r = raumkosten(&az_eingabe(false, 0, 1), &p);
    meldet_ueberlauf(r, &format!("{stelle} (Pf3)"));
    // Pf4: der Hoechstbetrag der Tagespauschale, 2000 Tage, pro_tag bleibt 6 (klein).
    // Folie: 12.000 EUR — bei Python begrenzt der GEHOBENE Cap nicht, die Pauschalen liefern
    // 6 EUR x 2000 Tage; der Endwert bleibt gueltig, die Umrechnung des Caps ist die Stelle.
    let (p, stelle) = gepatcht(
        "pf4",
        datei,
        &[(
            "tagespauschale_hoechstbetrag:\n  wert: 1260\n",
            "tagespauschale_hoechstbetrag:\n  wert: 92233720368547759\n",
        )],
    );
    let r = raumkosten(&az_eingabe(false, 0, 2000), &p);
    meldet_ueberlauf(r, &format!("{stelle} (Pf4)"));
}

/// Pd3 (Kohorten-Höchstbetrag) und Pf6 (EP-Höchstbetrag ohne Kfz): je eine Datei, je ein Anker.
#[test]
fn kohorten_und_ep_hoechstbetraege_melden_ueberlauf() {
    // Pd3: geburtsjahr 1940 -> Folgejahr 2005, Kohorte 40 %. Folie: 4.000 EUR
    // (40 % von 10.000 EUR, unter dem echten Hoechstbetrag 1.900 nicht gedeckelt — der gehohte
    // Cap wird nur in seiner eigenen Umrechnung gefaehrt).
    let (p, stelle) = gepatcht(
        "pd3",
        "kohorten/altersentlastungsbetrag_p24a.yaml",
        &[(
            "  2005: {prozentsatz: 40.0, hoechstbetrag: 1900}\n",
            "  2005: {prozentsatz: 40.0, hoechstbetrag: 92233720368547759}\n",
        )],
    );
    let e = P24aAltersentlastungEingabe {
        veranlagungszeitraum: 2025,
        geburtsjahr: 1940,
        arbeitslohn: Euro::new(10_000),
        positive_andere_einkuenfte: Euro::new(0),
    };
    let r = p24a_altersentlastung(&e, &p);
    meldet_ueberlauf(r, &format!("{stelle} (Pd3)"));
    // Pf6: ohne Kfz kaappt `hoechstbetrag_ohne_kfz` die Pauschale. Folie: 600 EUR (Staffel
    // 10 km x 200 Tage x 0,30 EUR = 600 EUR, der echte Cap 4.500 EUR greift nicht).
    let (p, stelle) = gepatcht(
        "pf6",
        "2025/entfernungspauschale.yaml",
        &[(
            "hoechstbetrag_ohne_kfz:\n  wert: 4500\n",
            "hoechstbetrag_ohne_kfz:\n  wert: 92233720368547759\n",
        )],
    );
    let e = EntfernungspauschaleEingabe {
        veranlagungszeitraum: Vz::Vz2025,
        entfernung_km_roh: Km::new(Decimal::from(10)),
        arbeitstage: 200,
        eigenes_oder_ueberlassenes_kfz: false,
        oepnv_kosten_jahr: Euro::new(0),
    };
    let r = entfernungspauschale(&e, &p);
    meldet_ueberlauf(r, &format!("{stelle} (Pf6)"));
}

/// N14: der Ertragsanteil-Prozentsatz der Kohorte 0 aus `params/kohorten/rente_ertragsanteil_p22.yaml`,
/// auf einen Wert gehoben, dessen Zehntel (`prozent * 10`) kein `i64` mehr ist. Die echten Tabellen
/// tragen 1 bis 100 (gemessen: Ertragsanteil 1..59, Besteuerungsanteil 50..100, Versorgungsfreibetrag
/// 0..40 Prozent), die Stelle ist nur mit gepatchter Datei erreichbar — dasselbe Harness wie oben,
/// gleiche Schwaeche (Datensatz, kein Quelltext-Mutant).
///
/// Python-Folien (`orakel_n14.py`, Jahresrente 1.000 EUR, VZ 2025, bb, Alter 0): 1e22 Prozent ->
/// 99.999.999.999.999.991.611.290 EUR, ausserhalb `i64` (Art: klar, das Orakel stuetzt die Sperre);
/// 1e18 Prozent -> 9.999.999.999.999.999.898 EUR, ebenfalls ausserhalb `i64` (Art: klar; das Zehntel
/// selbst ist 1e19 und damit groesser als `i64::MAX`: die Sperre sitzt an der `zehntel`-Stelle);
/// 9,2e17 Prozent -> 9.199.999.999.999.999.898 EUR, innerhalb (Zehntel 9,2e18 passt: Gegenprobe).
#[test]
fn rente_prozent_aus_params_ausserhalb_i64_meldet_ueberlauf() {
    let datei = "kohorten/rente_ertragsanteil_p22.yaml";
    let anker = "  0: {ertragsanteil_prozent: 59.0}\n";
    let rente = |prozent: &str, zelle: &str| {
        let p = params_gepatcht(
            zelle,
            datei,
            &[(
                anker,
                &format!("  0: {{ertragsanteil_prozent: {prozent}}}\n"),
            )],
        );
        let e = RentenEingabe {
            vz: Vz::Vz2025,
            art: Rentenart::Bb {
                alter_bei_rentenbeginn: 0,
            },
            jahresrente: Euro::new(1000),
        };
        renten_einkuenfte(&e, &p)
    };
    let marke = |r: Result<Euro, Teil2Fehler>| match r {
        Ok(e) => format!("Zahl {}", e.get()),
        Err(Teil2Fehler::Basis(EngineFehler::Ueberlauf(m))) => format!("Ueberlauf({m})"),
        Err(other) => format!("{other:?}"),
    };
    // Sperre: Zehntel ausserhalb i64 (1e23 bzw. 1e19).
    assert_eq!(
        marke(rente("10000000000000000000000.0", "n14a")),
        "Ueberlauf(teil2 i64)",
        "1e22 Prozent"
    );
    assert_eq!(
        marke(rente("1000000000000000000.0", "n14b")),
        "Ueberlauf(teil2 i64)",
        "1e18 Prozent"
    );
    // Gegenprobe: 9,2e17 Prozent -> Zehntel 9,2e18 passt; Rust liefert die Python-Zahl.
    assert_eq!(
        marke(rente("920000000000000000.0", "n14c")),
        "Zahl 9199999999999999898",
        "9,2e17 Prozent"
    );
}
