//! Erzeugt `rust/catala-sys/orakel/orakel.txt`: erzeugte Eingaben je Catala-Scope, gerechnet NUR vom
//! Catala-Interpreter (`catala interpret`: weder C-Backend noch Rust), samt Ergebnis. Der Test
//! `tests/catala_orakel.rs` spielt die Datei gegen die Rust-Huelle `catala_sys::*` ab.
//!
//! Aufruf: `make catala-orakel` (braucht den opam-Switch `taxgraph`). Gleiche Regeln, gleicher Seed:
//! gleiche Bytes; die CI erzeugt die Datei neu und prueft mit `git diff --exit-code`.
//!
//! Mechanismus: je Stapel von hoechstens 100 Faellen eine Kopie der Moduldatei aus `rules/estg/` mit
//! angehaengten `#[test]`-Scopes, die den echten Scope mit den Eingaben aufrufen (ein Scope in einem
//! ```catala-Block ist ausserhalb seiner Datei nicht sichtbar, deshalb dieselbe Datei). Der Interpreter
//! druckt die Scopes in Dateireihenfolge. Ein Fehlerblock traegt keinen Namen: die Zuordnung laeuft ueber
//! die Reihenfolge und wird an den Namen der Ergebnisbloecke geprueft.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

const SEED: u64 = 20_261_011;
/// Mehr Scopes je Datei machen den Interpreter ueberlinear langsam (gemessen: 100 in 0,2 s, 1000 in 32 s).
const STAPEL: usize = 100;
const FRIST: Duration = Duration::from_secs(300);

#[derive(Clone, Copy)]
enum Art {
    /// Geld in Cent, nie negativ.
    Geld,
    /// Geld in Cent, negativ zulaessig (Einkuenfte).
    GeldMitMinus,
    /// Geld in Cent von 0 bis zur Obergrenze (Saetze, Pauschalen, Hoechstbetraege).
    GeldBis(i64),
    /// Ganze Zahl von 0 bis zur Obergrenze.
    Ganz(i64),
    Wahr,
    /// Wie `Wahr`, aber aus einem eigenen Strom (Fallnummer, Eingabenummer): der Hauptstrom behaelt seine
    /// Ziehungen, die Zeilen der Nachfolger-Scopes aendern sich nicht. Fuer Eingaben, die einem
    /// bestehenden Scope nachtraeglich zuwachsen (`kranken_pflege_vorsorge`, Abweichung Nr. 115).
    /// ponytail: kein Zusammenhang mit den uebrigen Eingaben des Falls; Upgrade: neue Eingaben ans Ende
    /// von `SPECS` als eigener Scope, wenn die Nachfolger neu gezogen werden duerfen.
    WahrNebenstrom,
    Vz,
    /// Bruch `zaehler / nenner` (Catala-`decimal`), Wert 0 bis 150.
    Bruch,
    /// Wie `Bruch`, aber der Nenner ist 1, 10 oder 100: so fuehrt `engine` den Wert als `Decimal`
    /// (`Satz`, `Km`), und nur so spielt `engine/tests/catala_orakel_replay.rs` die Zeile ab.
    Dezimal,
}

#[derive(Clone, Copy)]
enum Wert {
    Geld(i64),
    Ganz(i64),
    Wahr(bool),
    Vz(u16),
    Bruch(i64, u64),
}

struct Spec {
    /// Name der Rust-Funktion in `catala_sys`.
    name: &'static str,
    /// Moduldatei unter `rules/estg/`.
    datei: &'static str,
    scope: &'static str,
    eingaben: &'static [(&'static str, Art)],
    ausgaben: &'static [&'static str],
    zufall: usize,
    /// Raster fuer Scopes mit einer Geld-Eingabe (und gegebenenfalls dem Jahr): alle `schritt` Cent bis
    /// `bis` Cent.
    raster: Option<(i64, i64)>,
}

const EST_DATEI: &str = "p32a/einkommensteuertarif.catala_en";

const GESAMT_EIN: &[(&str, Art)] = &[
    ("einkuenfte_nichtselbststaendig", Art::GeldMitMinus),
    ("einkuenfte_kapitalvermoegen", Art::GeldMitMinus),
    ("einkuenfte_vermietung", Art::GeldMitMinus),
    ("einkuenfte_sonstige", Art::GeldMitMinus),
    ("einkuenfte_gewinn", Art::GeldMitMinus),
    ("altersentlastungsbetrag", Art::Geld),
    ("entlastungsbetrag_alleinerziehende", Art::Geld),
    ("sonderausgaben", Art::Geld),
    ("aussergewoehnliche_belastungen", Art::Geld),
    ("freibetraege_kinder", Art::Geld),
    ("sonstige_abzuege_vom_einkommen", Art::Geld),
    ("anzurechnende_auslaendische_steuern", Art::Geld),
    ("steuerermaessigungen", Art::Geld),
    ("steuer_kapital_gesondert", Art::Geld),
    ("hinzurechnung_kindergeld", Art::Geld),
    ("hinzurechnung_zulage", Art::Geld),
    ("tarif_modifiziert", Art::Wahr),
    ("tarifliche_est_modifiziert", Art::Geld),
    ("veranlagungszeitraum", Art::Vz),
];

const EST_AUS: &[&str] = &[
    "summe_der_einkuenfte",
    "gesamtbetrag_der_einkuenfte",
    "einkommen",
    "zu_versteuerndes_einkommen",
    "tarifliche_est",
    "festzusetzende_est",
];

const EINZEL_EIN: &[(&str, Art)] = &[
    ("bruttoarbeitslohn", Art::Geld),
    ("werbungskosten", Art::Geld),
    ("sonderausgaben", Art::Geld),
    ("veranlagungszeitraum", Art::Vz),
];

/// Die Reihenfolge ist Teil des Formats: neue Eintraege nur am Ende, sonst aendert sich der Zufallsstrom
/// aller Nachfolger.
const SPECS: &[Spec] = &[
    Spec {
        name: "grundtarif",
        datei: EST_DATEI,
        scope: "Grundtarif",
        eingaben: &[("zu_versteuerndes_einkommen", Art::Geld), ("veranlagungszeitraum", Art::Vz)],
        ausgaben: &["tarifliche_steuer"],
        zufall: 300,
        raster: Some((250_000, 40_000_000)),
    },
    Spec {
        name: "zumutbare_belastung",
        datei: "p33/zumutbarebelastung.catala_en",
        scope: "ZumutbareBelastung",
        eingaben: &[
            ("gesamtbetrag_der_einkuenfte", Art::Geld),
            ("anzahl_kinder", Art::Ganz(8)),
            ("splitting", Art::Wahr),
        ],
        ausgaben: &["zumutbare_belastung"],
        zufall: 400,
        raster: None,
    },
    Spec {
        name: "festzusetzende_est_gesamt",
        datei: EST_DATEI,
        scope: "FestzusetzendeEstGesamt",
        eingaben: GESAMT_EIN,
        ausgaben: EST_AUS,
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "festzusetzende_est_gesamt_zusammen",
        datei: EST_DATEI,
        scope: "FestzusetzendeEstGesamtZusammen",
        eingaben: GESAMT_EIN,
        ausgaben: EST_AUS,
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "splittingtarif",
        datei: EST_DATEI,
        scope: "Splittingtarif",
        eingaben: &[("zu_versteuerndes_einkommen_gemeinsam", Art::Geld), ("veranlagungszeitraum", Art::Vz)],
        ausgaben: &["tarifliche_steuer"],
        zufall: 300,
        raster: Some((500_000, 80_000_000)),
    },
    Spec {
        name: "festzusetzende_est_einzel",
        datei: EST_DATEI,
        scope: "FestzusetzendeEstEinzel",
        eingaben: EINZEL_EIN,
        ausgaben: &["festzusetzende_est"],
        zufall: 400,
        raster: None,
    },
    Spec {
        name: "festzusetzende_est_einzel_voll",
        datei: EST_DATEI,
        scope: "FestzusetzendeEstEinzel",
        eingaben: EINZEL_EIN,
        ausgaben: EST_AUS,
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "festzusetzende_est_zusammen",
        datei: EST_DATEI,
        scope: "FestzusetzendeEstZusammen",
        eingaben: &[
            ("bruttoarbeitslohn_a", Art::Geld),
            ("werbungskosten_a", Art::Geld),
            ("bruttoarbeitslohn_b", Art::Geld),
            ("werbungskosten_b", Art::Geld),
            ("sonderausgaben_gemeinsam", Art::Geld),
            ("veranlagungszeitraum", Art::Vz),
        ],
        ausgaben: &["festzusetzende_est"],
        zufall: 400,
        raster: None,
    },
    Spec {
        name: "spenden_abzug",
        datei: "p10b/spendenabzug.catala_en",
        scope: "SpendenAbzug",
        eingaben: &[("zuwendungen", Art::Geld), ("gesamtbetrag_der_einkuenfte", Art::Geld)],
        ausgaben: &["spenden_abzug"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "agb_abzug",
        datei: "p33/agbabzug.catala_en",
        scope: "AgbAbzug",
        eingaben: &[("aussergewoehnliche_belastungen", Art::Geld), ("zumutbare_belastung", Art::Geld)],
        ausgaben: &["abzug_agb"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "kirchensteuerabzug",
        datei: "p10/kirchensteuerabzug.catala_en",
        scope: "Kirchensteuerabzug",
        eingaben: &[("gezahlte_kirchensteuer", Art::Geld), ("erstattete_kirchensteuer", Art::Geld)],
        ausgaben: &["abziehbare_kirchensteuer"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "altersentlastungsbetrag",
        datei: "p24a/altersentlastungsbetrag.catala_en",
        scope: "Altersentlastungsbetrag",
        eingaben: &[
            ("arbeitslohn", Art::Geld),
            ("positive_andere_einkuenfte", Art::Geld),
            ("prozentsatz", Art::Bruch),
            ("hoechstbetrag", Art::Geld),
        ],
        ausgaben: &["altersentlastungsbetrag"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "entlastungsbetrag",
        datei: "p24b/entlastungsbetrag.catala_en",
        scope: "Entlastungsbetrag",
        eingaben: &[
            ("alleinstehend", Art::Wahr),
            ("anzahl_kinder", Art::Ganz(8)),
            ("monate_ohne_voraussetzung", Art::Ganz(14)),
        ],
        ausgaben: &["entlastungsbetrag"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "familienleistungsausgleich",
        datei: "p31/familienleistungsausgleich.catala_en",
        scope: "Familienleistungsausgleich",
        eingaben: &[
            ("est_ohne_freibetraege", Art::Geld),
            ("est_mit_freibetraegen", Art::Geld),
            ("kindergeld", Art::Geld),
        ],
        ausgaben: &["est_nach_familienausgleich"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "verbilligte_vermietung_wk",
        datei: "p21_2/verbilligtevermietungwk.catala_en",
        scope: "VerbilligteVermietungWk",
        eingaben: &[("werbungskosten", Art::Geld), ("entgelt_quote_prozent", Art::Bruch)],
        ausgaben: &["abziehbare_werbungskosten"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "kranken_pflege_vorsorge",
        datei: "p10_1_3_3a/krankenpflegevorsorge.catala_en",
        scope: "KrankenPflegeVorsorge",
        eingaben: &[
            ("basis_kv_pv", Art::Geld),
            ("weitere_vorsorgeaufwendungen", Art::Geld),
            ("mit_anspruch_auf_zuschuss", Art::Wahr),
            ("zusammenveranlagt", Art::WahrNebenstrom),
            ("mit_anspruch_auf_zuschuss_ehegatte", Art::WahrNebenstrom),
        ],
        ausgaben: &["abziehbare_kv_pv_vorsorge"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "berufsausbildung",
        datei: "p10_1_7/berufsausbildungsaufwendungen.catala_en",
        scope: "Berufsausbildung",
        eingaben: &[("aufwendungen", Art::Geld)],
        ausgaben: &["abziehbare_sonderausgaben"],
        zufall: 200,
        raster: Some((25_000, 1_000_000)),
    },
    Spec {
        name: "betriebs_freibetrag",
        datei: "p16_4_freibetrag/betriebsfreibetrag.catala_en",
        scope: "BetriebsFreibetrag",
        eingaben: &[("veraeusserungsgewinn", Art::Geld)],
        ausgaben: &["freibetrag"],
        zufall: 200,
        raster: Some((500_000, 50_000_000)),
    },
    Spec {
        name: "euer_gewinn",
        datei: "p4_3_gewinn/euergewinn.catala_en",
        scope: "EuerGewinn",
        eingaben: &[("betriebseinnahmen", Art::Geld), ("betriebsausgaben", Art::Geld)],
        ausgaben: &["gewinn"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "mitunternehmer_einkuenfte",
        datei: "p15_1_2_mitunternehmer/mitunternehmereinkuenfte.catala_en",
        scope: "MitunternehmerEinkuenfte",
        eingaben: &[
            ("gewinnanteil", Art::GeldMitMinus),
            ("verguetung_taetigkeit", Art::Geld),
            ("verguetung_darlehen", Art::Geld),
            ("verguetung_ueberlassung", Art::Geld),
        ],
        ausgaben: &["einkuenfte_mitunternehmer"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "gwg_sofortabzug",
        datei: "p6_2_gwg_sofortabzug/gwgsofortabzug.catala_en",
        scope: "GwgSofortabzug",
        eingaben: &[("anschaffungskosten_netto", Art::Geld)],
        ausgaben: &["sofortabzug"],
        zufall: 200,
        raster: Some((5_000, 200_000)),
    },
    Spec {
        name: "haushaltsnahe",
        datei: "p35a/haushaltsnahe.catala_en",
        scope: "Haushaltsnahe",
        eingaben: &[
            ("minijob_aufwendungen", Art::Geld),
            ("haushaltsnahe_dienstleistungen", Art::Geld),
            ("handwerker_arbeitskosten", Art::Geld),
        ],
        ausgaben: &["steuerermaessigung"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "verlustvortrag_abzug",
        datei: "p10d_2_verlustvortrag_abzug/verlustvortrag.catala_en",
        scope: "VerlustvortragAbzug",
        eingaben: &[
            ("gesamtbetrag_einkuenfte", Art::GeldMitMinus),
            ("verlustvortrag_bestand", Art::Geld),
            ("zusammenveranlagung", Art::Wahr),
        ],
        ausgaben: &["verlustabzug"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "ermaessigter_durchschnittssatz",
        datei: "p34_3_ermaessigter_durchschnittssatz/ermaessigterdurchschnittssatz.catala_en",
        scope: "ErmaessigterDurchschnittssatz",
        eingaben: &[
            ("ao_einkuenfte", Art::Geld),
            ("est_gesamt_zzgl_progression", Art::Geld),
            ("bemessungsgrundlage_durchschnitt", Art::Geld),
        ],
        ausgaben: &["est_ao"],
        zufall: 300,
        raster: None,
    },
    Spec {
        name: "entfernungspauschale",
        datei: "p09_entfernungspauschale/entfernungspauschale.catala_en",
        scope: "Berechnung",
        eingaben: &[
            ("entfernung_km_roh", Art::Bruch),
            ("arbeitstage", Art::Ganz(366)),
            ("eigenes_oder_ueberlassenes_kfz", Art::Wahr),
            ("oepnv_kosten_jahr", Art::GeldBis(1_000_000)),
            ("satz_bis_20_km", Art::GeldBis(100)),
            ("satz_ab_21_km", Art::GeldBis(100)),
            ("staffelgrenze_km", Art::Ganz(40)),
            ("hoechstbetrag", Art::GeldBis(1_000_000)),
        ],
        ausgaben: &["entfernungspauschale", "abziehbarer_betrag"],
        zufall: 400,
        raster: None,
    },
    Spec {
        name: "raumkostenabzug",
        datei: "p04_arbeitszimmer_homeoffice/arbeitszimmer_homeoffice.catala_en",
        scope: "Raumkostenabzug",
        eingaben: &[
            ("arbeitszimmer_vorhanden", Art::Wahr),
            ("ist_mittelpunkt", Art::Wahr),
            ("tatsaechliche_aufwendungen", Art::GeldBis(2_000_000)),
            ("jahrespauschale_gewaehlt", Art::Wahr),
            ("monate_ohne_mittelpunkt", Art::Ganz(12)),
            ("homeoffice_tage", Art::Ganz(366)),
            ("jahrespauschale", Art::GeldBis(300_000)),
            ("tagespauschale_pro_tag", Art::GeldBis(2_000)),
            ("tagespauschale_hoechstbetrag", Art::GeldBis(300_000)),
        ],
        ausgaben: &["abzug_arbeitszimmer", "abzug_homeoffice", "abzug_gesamt"],
        zufall: 400,
        raster: None,
    },
    // Dezimalzeilen fuer die drei Scopes mit Bruch-Eingabe, die `engine` als `Decimal` fuehrt: dieselben Scopes
    // noch einmal, die Bruch-Eingabe mit Nenner 1, 10 oder 100. Hinter allen anderen, damit der Zufallsstrom
    // der Vorgaenger und damit ihre Zeilen byte-gleich bleiben.
    Spec {
        name: "altersentlastungsbetrag",
        datei: "p24a/altersentlastungsbetrag.catala_en",
        scope: "Altersentlastungsbetrag",
        eingaben: &[
            ("arbeitslohn", Art::Geld),
            ("positive_andere_einkuenfte", Art::Geld),
            ("prozentsatz", Art::Dezimal),
            ("hoechstbetrag", Art::Geld),
        ],
        ausgaben: &["altersentlastungsbetrag"],
        zufall: 150,
        raster: None,
    },
    Spec {
        name: "verbilligte_vermietung_wk",
        datei: "p21_2/verbilligtevermietungwk.catala_en",
        scope: "VerbilligteVermietungWk",
        eingaben: &[("werbungskosten", Art::Geld), ("entgelt_quote_prozent", Art::Dezimal)],
        ausgaben: &["abziehbare_werbungskosten"],
        zufall: 150,
        raster: None,
    },
    Spec {
        name: "entfernungspauschale",
        datei: "p09_entfernungspauschale/entfernungspauschale.catala_en",
        scope: "Berechnung",
        eingaben: &[
            ("entfernung_km_roh", Art::Dezimal),
            ("arbeitstage", Art::Ganz(366)),
            ("eigenes_oder_ueberlassenes_kfz", Art::Wahr),
            ("oepnv_kosten_jahr", Art::GeldBis(1_000_000)),
            ("satz_bis_20_km", Art::GeldBis(100)),
            ("satz_ab_21_km", Art::GeldBis(100)),
            ("staffelgrenze_km", Art::Ganz(40)),
            ("hoechstbetrag", Art::GeldBis(1_000_000)),
        ],
        ausgaben: &["entfernungspauschale", "abziehbarer_betrag"],
        zufall: 150,
        raster: None,
    },
];

/// `SplitMix64`: kein Fremdpaket, damit der Strom ueber Versionen hinweg dieselben Bytes liefert.
struct Rng(u64);

impl Rng {
    fn naechste(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// In `0..n` (n > 0); die Modulo-Verzerrung ist hier ohne Belang.
    fn unter(&mut self, n: u64) -> u64 {
        self.naechste() % n
    }

    /// In `lo..=hi`.
    fn zwischen(&mut self, lo: i64, hi: i64) -> i64 {
        let spanne = u64::try_from(hi - lo + 1).unwrap_or(1);
        lo + i64::try_from(self.unter(spanne)).unwrap_or(0)
    }
}

/// Ein Geldbetrag in Cent: oft 0, oft rund (ganze 1.000 EUR und +-1 Cent daneben, dort liegen die
/// Stufengrenzen), sonst logarithmisch verteilt von 10 EUR bis 1 Mio EUR, selten darueber, und in einem
/// von hundert Faellen so gross, dass Summen und Produkte nicht mehr in `i64` passen.
fn geld(rng: &mut Rng, mit_minus: bool) -> i64 {
    let betrag = match rng.unter(100) {
        0..=11 => 0,
        12..=19 => rng.zwischen(1, 100),
        20..=63 => {
            let stelle = u32::try_from(rng.zwischen(3, 8)).unwrap_or(3);
            rng.zwischen(10_i64.pow(stelle - 1), 10_i64.pow(stelle))
        }
        64..=83 => rng.zwischen(1, 500) * 100_000,
        84..=93 => rng.zwischen(1, 500) * 100_000 + rng.zwischen(-1, 1),
        94..=98 => rng.zwischen(100_000_000, 10_000_000_000),
        _ => rng.zwischen(i64::MAX / 2, i64::MAX),
    };
    if mit_minus && rng.unter(4) == 0 {
        -betrag
    } else {
        betrag
    }
}

fn geld_bis(rng: &mut Rng, max: i64) -> i64 {
    match rng.unter(10) {
        0 => 0,
        1 => max,
        _ => rng.zwischen(0, max),
    }
}

fn wert(rng: &mut Rng, art: Art, neben: u64) -> Wert {
    match art {
        Art::Geld => Wert::Geld(geld(rng, false)),
        Art::GeldMitMinus => Wert::Geld(geld(rng, true)),
        Art::GeldBis(max) => Wert::Geld(geld_bis(rng, max)),
        Art::Ganz(max) => Wert::Ganz(rng.zwischen(0, max)),
        Art::Wahr => Wert::Wahr(rng.unter(2) == 1),
        Art::WahrNebenstrom => Wert::Wahr(Rng(neben).unter(2) == 1),
        Art::Vz => Wert::Vz(2024 + u16::try_from(rng.unter(3)).unwrap_or(0)),
        Art::Bruch => {
            let nenner = rng.zwischen(1, 100);
            Wert::Bruch(rng.zwischen(0, 150 * nenner), u64::try_from(nenner).unwrap_or(1))
        }
        Art::Dezimal => {
            let nenner = match rng.unter(3) {
                0 => 1,
                1 => 10,
                _ => 100,
            };
            Wert::Bruch(rng.zwischen(0, 150 * nenner), u64::try_from(nenner).unwrap_or(1))
        }
    }
}

fn faelle(spec: &Spec, rng: &mut Rng) -> Vec<Vec<Wert>> {
    let mut alle = Vec::new();
    if let Some((schritt, bis)) = spec.raster {
        let mit_vz = spec.eingaben.last().is_some_and(|&(_, art)| matches!(art, Art::Vz));
        let mut betrag = 0;
        while betrag <= bis {
            if mit_vz {
                for jahr in 2024..=2026 {
                    alle.push(vec![Wert::Geld(betrag), Wert::Vz(jahr)]);
                }
            } else {
                alle.push(vec![Wert::Geld(betrag)]);
            }
            betrag += schritt;
        }
    }
    for fall in 0..spec.zufall {
        let fall = u64::try_from(fall).unwrap_or(0);
        alle.push(
            spec.eingaben
                .iter()
                .zip(0_u64..)
                .map(|(&(_, art), nr)| wert(rng, art, SEED ^ (fall << 8) ^ nr))
                .collect(),
        );
    }
    alle
}

/// `12345` Cent als Catala-Literal `$123.45`, negativ als `-$123.45`.
fn catala_geld(cent: i64) -> String {
    let abs = cent.unsigned_abs();
    let euro = (abs / 100).to_string();
    let mut gruppiert = String::new();
    for (i, ziffer) in euro.chars().enumerate() {
        if i > 0 && (euro.len() - i).is_multiple_of(3) {
            gruppiert.push(',');
        }
        gruppiert.push(ziffer);
    }
    format!("{}${gruppiert}.{:02}", if cent < 0 { "-" } else { "" }, abs % 100)
}

fn catala_wert(w: Wert) -> String {
    match w {
        Wert::Geld(c) => catala_geld(c),
        Wert::Ganz(n) => n.to_string(),
        Wert::Wahr(b) => b.to_string(),
        Wert::Vz(j) => format!("VZ{j}"),
        Wert::Bruch(zaehler, nenner) => format!("{zaehler} / {nenner}"),
    }
}

fn datei_wert(w: Wert) -> String {
    match w {
        Wert::Geld(n) | Wert::Ganz(n) => n.to_string(),
        Wert::Wahr(b) => b.to_string(),
        Wert::Vz(j) => j.to_string(),
        Wert::Bruch(zaehler, nenner) => format!("{zaehler}/{nenner}"),
    }
}

fn scope_name(spec_nr: usize, fall_nr: usize) -> String {
    format!("O{spec_nr}x{fall_nr}")
}

/// Ein `#[test]`-Scope, der den echten Scope aufruft und dessen Ausgaben unter denselben Namen ausgibt.
fn scope_text(spec: &Spec, name: &str, fall: &[Wert]) -> String {
    let mut s = format!("#[test]\ndeclaration scope {name}:\n  internal r content {}\n", spec.scope);
    for aus in spec.ausgaben {
        let _ = writeln!(s, "  output {aus} content money");
    }
    let _ = writeln!(s, "\nscope {name}:\n  definition r equals output of {} with {{", spec.scope);
    let felder: Vec<String> = spec
        .eingaben
        .iter()
        .zip(fall)
        .map(|(&(n, _), &w)| format!("    -- {n}: {}", catala_wert(w)))
        .collect();
    let _ = writeln!(s, "{} }}", felder.join("\n"));
    for aus in spec.ausgaben {
        let _ = writeln!(s, "  definition {aus} equals r.{aus}");
    }
    s.push('\n');
    s
}

type Felder = Vec<(String, String)>;

/// Zerlegt die Standardausgabe von `catala interpret` in Ergebnisbloecke je Scope-Name. Scheitert ein
/// Scope, steht sein Fehlerblock ohne Namen auf der Fehlerausgabe, sein Ergebnisblock fehlt.
fn ergebnisse(ausgabe: &str) -> BTreeMap<String, Felder> {
    let mut alle: BTreeMap<String, Felder> = BTreeMap::new();
    let mut aktuell: Option<String> = None;
    for zeile in ausgabe.lines() {
        if let Some(rest) = zeile.strip_prefix("┌─[RESULT]─ ") {
            let name = rest.trim_end_matches('─').trim().to_owned();
            alle.entry(name.clone()).or_default();
            aktuell = Some(name);
        } else if zeile.starts_with("┌─") {
            aktuell = None;
        } else if let (Some(name), Some(feld)) = (&aktuell, zeile.strip_prefix("│ ")) {
            if let (Some((k, v)), Some(felder)) = (feld.split_once(" = "), alle.get_mut(name)) {
                felder.push((k.trim().to_owned(), v.trim().to_owned()));
            }
        }
    }
    alle
}

fn zaehle_fehlerbloecke(fehlerausgabe: &str) -> usize {
    fehlerausgabe.lines().filter(|z| z.starts_with("┌─[ERROR]─")).count()
}

/// `$1,735.30`, `-$12.00` oder `$-12.00` als Cent.
fn cent_aus_text(t: &str) -> Result<i128, String> {
    let fehler = || format!("kein Geldbetrag: {t:?}");
    let ohne: String = t.chars().filter(|&c| c != ',' && c != '$').collect();
    let (negativ, zahl) = ohne.strip_prefix('-').map_or((false, ohne.as_str()), |r| (true, r));
    let (euro, cent) = zahl.split_once('.').ok_or_else(fehler)?;
    if cent.len() != 2 {
        return Err(fehler());
    }
    let betrag = euro.parse::<i128>().map_err(|_| fehler())? * 100 + cent.parse::<i128>().map_err(|_| fehler())?;
    Ok(if negativ { -betrag } else { betrag })
}

/// Fuehrt einen Prozess mit Frist aus und gibt Standard- und Fehlerausgabe zurueck. Beide gehen in
/// Dateien, damit kein Pipe-Puffer volllaeuft.
fn starte(cmd: &str, args: &[&str], cwd: &Path) -> Result<(String, String), String> {
    let aus_pfad = cwd.join(".lauf.out");
    let fehl_pfad = cwd.join(".lauf.err");
    let aus = File::create(&aus_pfad).map_err(|e| e.to_string())?;
    let fehl = File::create(&fehl_pfad).map_err(|e| e.to_string())?;
    let mut kind = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(aus)
        .stderr(fehl)
        .spawn()
        .map_err(|e| format!("{cmd} startet nicht ({e}); opam-Env laden: eval $(opam env --switch=taxgraph --set-switch)"))?;
    let start = Instant::now();
    loop {
        if kind.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if start.elapsed() > FRIST {
            let _ = kind.kill();
            let _ = kind.wait();
            return Err(format!("{cmd} ueberschreitet {} s", FRIST.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let lies = |p: &Path| std::fs::read_to_string(p).map_err(|e| e.to_string());
    Ok((lies(&aus_pfad)?, lies(&fehl_pfad)?))
}

/// Ein Projektordner fuer eine Spec: die Moduldatei und eine minimale `clerk.toml`.
struct Projekt<'a> {
    ordner: PathBuf,
    datei: String,
    modul: String,
    spec: &'a Spec,
    nr: usize,
}

impl<'a> Projekt<'a> {
    fn neu(arbeit: &Path, wurzel: &Path, nr: usize, spec: &'a Spec) -> Result<Self, String> {
        let ordner = arbeit.join(format!("spec{nr}"));
        std::fs::create_dir_all(&ordner).map_err(|e| e.to_string())?;
        std::fs::write(ordner.join("clerk.toml"), "[project]\ninclude_dirs = [\".\"]\n").map_err(|e| e.to_string())?;
        let quelle = wurzel.join("rules/estg").join(spec.datei);
        let modul = std::fs::read_to_string(&quelle).map_err(|e| format!("{}: {e}", quelle.display()))?;
        let datei = Path::new(spec.datei).file_name().and_then(|n| n.to_str()).ok_or("Dateiname")?.to_owned();
        Ok(Self { ordner, datei, modul, spec, nr })
    }

    /// Rechnet einen Stapel im Interpreter. `None` = der Scope scheiterte (Fehlerblock).
    fn stapel(&self, start: usize, chunk: &[Vec<Wert>]) -> Result<Vec<Option<Vec<i128>>>, String> {
        let mut text = self.modul.clone();
        text.push_str("\n\n```catala\n");
        for (i, fall) in chunk.iter().enumerate() {
            text.push_str(&scope_text(self.spec, &scope_name(self.nr, start + i), fall));
        }
        text.push_str("```\n");
        std::fs::write(self.ordner.join(&self.datei), text).map_err(|e| e.to_string())?;
        if start == 0 {
            // Baut _build/libcatala, die Standardbibliothek, die `interpret` braucht. Das Ergebnis des
            // Testlaufs zaehlt nicht, nur dass die Bibliothek danach da ist.
            starte("clerk", &["test", &self.datei], &self.ordner)?;
        }
        let (aus, fehl) = starte("catala", &["interpret", &self.datei], &self.ordner)?;
        let ergebnis = ergebnisse(&aus);
        let antworten = (0..chunk.len())
            .map(|i| self.antwort(&scope_name(self.nr, start + i), &ergebnis))
            .collect::<Result<Vec<_>, _>>()?;
        // Jeder Scope ohne Ergebnisblock muss einen Fehlerblock auf der Fehlerausgabe haben; sonst ist
        // etwas anderes schiefgegangen (Syntaxfehler im erzeugten Scope, Abbruch), nicht ein Rechenfehler.
        let fehlend = antworten.iter().filter(|a| a.is_none()).count();
        if zaehle_fehlerbloecke(&fehl) != fehlend {
            return Err(format!(
                "{}: {fehlend} Scopes ohne Ergebnis, aber {} Fehlerbloecke:\n{}",
                self.spec.name,
                zaehle_fehlerbloecke(&fehl),
                fehl.lines().take(12).collect::<Vec<_>>().join("\n")
            ));
        }
        Ok(antworten)
    }

    /// `None` = der Scope hat keinen Ergebnisblock (Rechenfehler, etwa Teilung durch null).
    fn antwort(&self, name: &str, ergebnis: &BTreeMap<String, Felder>) -> Result<Option<Vec<i128>>, String> {
        let Some(felder) = ergebnis.get(name) else {
            return Ok(None);
        };
        let spec = self.spec.name;
        self.spec
            .ausgaben
            .iter()
            .map(|aus| {
                let (_, v) = felder
                    .iter()
                    .find(|(k, _)| k == aus)
                    .ok_or_else(|| format!("{spec}: Ausgabe {aus} fehlt in {name}"))?;
                cent_aus_text(v)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    fn rechne(&self, alle: &[Vec<Wert>]) -> Result<Vec<Option<Vec<i128>>>, String> {
        let mut antworten = Vec::new();
        for (i, chunk) in alle.chunks(STAPEL).enumerate() {
            antworten.extend(self.stapel(i * STAPEL, chunk)?);
        }
        Ok(antworten)
    }
}

fn zeilen(spec: &Spec, alle: &[Vec<Wert>], antworten: &[Option<Vec<i128>>]) -> String {
    let mut s = String::new();
    for (fall, antwort) in alle.iter().zip(antworten) {
        let ein: Vec<String> =
            spec.eingaben.iter().zip(fall).map(|(&(n, _), &w)| format!("{n}={}", datei_wert(w))).collect();
        let aus = antwort.as_ref().map_or_else(
            || "FEHLER".to_owned(),
            |werte| spec.ausgaben.iter().zip(werte).map(|(n, w)| format!("{n}={w}")).collect::<Vec<_>>().join(" "),
        );
        let _ = writeln!(s, "{} | {} | {aus}", spec.name, ein.join(" "));
    }
    s
}

fn repo_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn arbeitsordner() -> Result<PathBuf, String> {
    let basis = std::env::var_os("TMPDIR").map_or_else(
        || Path::new(&std::env::var_os("HOME").unwrap_or_default()).join(".cache/taxgraph-tmp"),
        PathBuf::from,
    );
    let ordner = basis.join("catala-orakel-gen");
    if ordner.exists() {
        std::fs::remove_dir_all(&ordner).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&ordner).map_err(|e| e.to_string())?;
    Ok(ordner)
}

fn regeln_hash(wurzel: &Path) -> Result<String, String> {
    let stand =
        std::fs::read_to_string(wurzel.join("rust/catala-sys/generated/SOURCE_HASH")).map_err(|e| e.to_string())?;
    let jetzt = catala_sys::recompute_source_hash(wurzel).map_err(|e| e.to_string())?;
    if stand.trim() != jetzt {
        return Err("generated/ ist veraltet gegen rules/ -- erst `make catala-c`".to_owned());
    }
    Ok(jetzt)
}

fn erzeuge() -> Result<usize, String> {
    let wurzel = repo_wurzel();
    let hash = regeln_hash(&wurzel)?;
    let arbeit = arbeitsordner()?;
    let (version, _) = starte("catala", &["--version"], &arbeit)?;
    let mut text = format!(
        "# catala-orakel v1 -- NICHT von Hand aendern, `make catala-orakel` erzeugt die Datei\n\
         # regeln-hash: {hash}\n# catala: {}\n# seed: {SEED}\n\
         # funktion | eingaben (Cent, ganze Zahl, true/false, Jahr, Bruch) | ausgaben (Cent) oder FEHLER\n",
        version.trim()
    );
    let mut rng = Rng(SEED);
    let mut anzahl = 0;
    for (nr, spec) in SPECS.iter().enumerate() {
        let alle = faelle(spec, &mut rng);
        let antworten = Projekt::neu(&arbeit, &wurzel, nr, spec)?.rechne(&alle)?;
        text.push_str(&zeilen(spec, &alle, &antworten));
        anzahl += alle.len();
        println!("[orakel] {}: {} Faelle", spec.name, alle.len());
    }
    let ziel = wurzel.join("rust/catala-sys/orakel/orakel.txt");
    std::fs::create_dir_all(ziel.parent().ok_or("Ziel")?).map_err(|e| e.to_string())?;
    std::fs::write(&ziel, text).map_err(|e| e.to_string())?;
    Ok(anzahl)
}

fn main() -> ExitCode {
    match erzeuge() {
        Ok(n) => {
            println!("[orakel] {n} Faelle geschrieben");
            ExitCode::SUCCESS
        }
        Err(f) => {
            eprintln!("[orakel] {f}");
            ExitCode::FAILURE
        }
    }
}
