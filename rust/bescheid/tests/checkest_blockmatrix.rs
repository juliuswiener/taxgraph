//! Block-Matrix gegen die amtliche Pruefung (`checkESt`): ist ein BLOCK als Ganzes einreichbar? Rust-Ersatz fuer
//! `tests/test_checkest_blockmatrix.py` (Option b, Entscheid `blockmatrix-laeuft-in-rust-gesperrte-zeilen-ueber-den-test-umweg`).
//!
//! Viele Felder ergeben nur zusammen mit ihrem Block eine Erklaerung, die `checkESt` annimmt (Kind ohne Vorname, Anlage V ohne
//! Objekt, § 35c ohne Gebaeude). Der Test baut je Block eine Akte (Basis plus Blockfelder), laesst sie ueber den echten
//! Abgabeweg laufen (`einreichungs_xml`, mit Guard) und gibt das XML an `elster::validiere` (`ERiC`, nur `ERIC_VALIDIERE`, kein
//! Versand). Ein Block ist gruen bei `rc=0` und 0 Meldungen. checkESt prueft Plausibilitaet, nicht Richtigkeit der Zahlen.
//!
//! KONTROLLEN. Ein gruener Lauf ohne `ERiC` saehe gleich aus. Darum laufen vor den Bloecken die Basis allein (muss `rc=0` sein) und
//! zwei Akten, die `ERiC` beanstanden MUSS (Kind ohne Vorname, Anlage V nur mit Einnahmen): `rc=610001002` mit lesbarer Meldung.
//!
//! ZEILEN "SONSTIGES" (Abweichung Nr. 48 Unfallkosten, Nr. 49 § 34c-Abzug). Beide Sperren fielen auf Julius' Wort am 2026-10-10,
//! nachdem dieser Test die Zeilen gemessen hatte: der ECHTE Abgabeweg (`einreichungs_xml`) baut das XML, kein Umweg. Wo der
//! Bescheid den Abzug nicht rechnet, bleibt die Sperre des Abzugs (`p34c_abzug_weitere_wk_xml.rs`); sie hat hier keinen Fall.
//!
//! ANLAGE AUS (Abweichung Nr. 51, Staat als Listentext in `E0600301`). Jede Akte mit Auslandseinkuenften scheiterte bei `checkESt`
//! an der fehlenden Staat-Angabe; seit Nr. 51 steht der Staat. Der dritte echte Test misst die Anrechnung ueber den echten Abgabeweg
//! (Block "Ausland Anrechnung"), den Abzug nach § 34c Abs. 2 (Nr. 41, `E0600920`) ueber den echten Abgabeweg und drei Kontrollen, die `ERiC`
//! beanstanden MUSS: Staat ohne Einkuenfte, Einkuenfte ohne Staat, Steuer ohne Einkuenfte.
//!
//! ANLAGE SO, PRIVATVERKAUF (Abweichung Nr. 54, Spike AK0 in `checkest_privatverkauf_spike.rs`). Der vierte echte Test misst, was der
//! Schreiber fuer steuerpflichtige Verkaeufe baut (Grundstueck, anderes Wirtschaftsgut, mit Abschreibung, Verlust, Gewinn 0, zwei
//! Verkaeufe) ueber den echten Abgabeweg, und fuenf Kontrollen, die `ERiC` beanstanden MUSS (Pflicht-Kz fehlt, Gewinn passt nicht zu
//! den Einzelbetraegen). Ein steuerfreier Verkauf steht nicht in der Erklaerung, die Abgabe sperrt (`privatverkauf_einreichung_hermetisch.rs`).
//!
//! KEIN SKIP. Die echten Tests sind `#[ignore]` (`cargo test --workspace` und die CI laufen ohne sie: `ERiC` und die Hersteller-ID
//! gehoeren nicht auf einen Runner, Entscheid Julius 2026-09-12). Wer sie mit `--ignored` ruft, will den echten Weg: jede fehlende
//! Voraussetzung ist ein `panic`, nie ein `return`. Lokal: `make blockmatrix-rust` (laedt die ID aus der gitignorierten `.env`).
//! Die CI deckt nur die Verdrahtung (`die_blockmatrix_ist_verdrahtet`) und die ID-Pruefung (`die_id_pruefung_*`).
//!
//! KEIN GEHEIMNIS IN DER AUSGABE. Gedruckt werden `rc`, Klasse und die `<Text>`-Meldungen, die ID darin ersetzt durch `<ID>`; nie das
//! XML, nie der Rumpf der `ERiC`-Antwort, nie die ID.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use bescheid::deklaration::{einreichungs_xml, mit_ring_werten};
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Vz, Zustand};
use elster::testhilfe::schemas_da;
use elster::{
    deklariere, erzeuge_xml, klassifiziere_rc, validiere, Deklaration, EricKlasse, Felder, XmlOptionen,
};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

/// Die Namen der echten Tests: Make-Ziel und Verdrahtungs-Test pruefen sie.
const ECHTE_TESTS: [&str; 5] = [
    "zwoelf_bloecke_gegen_echtes_checkest",
    "zeilen_nr48_nr49_gegen_echtes_checkest",
    "zeilen_nr48_nr49_vz2024_gegen_echtes_checkest",
    "ausland_staat_und_nr41_gegen_echtes_checkest",
    "privatverkauf_gegen_echtes_checkest",
];
const MAKE_ZIEL: &str = "blockmatrix-rust";
/// Die Zahl der Bloecke der Python-Matrix. Faellt eine Zeile aus `bloecke()` weg, wird der Test rot, nicht kleiner.
const ANZAHL_BLOECKE: usize = 12;
/// `ERiC` antwortet auf die Platzhalter-ID mit `rc=610301202` und prueft nichts.
const PLATZHALTER_ID: &str = "74931";
const RC_PLAUSIBILITAET: i32 = 610_001_002;

/// Die ID aus dem Prozess-Env, oder der Grund, warum sie nicht taugt. Der Grund nennt nie den Wert.
fn pruefe_id(roh: Option<&str>) -> Result<String, &'static str> {
    let id = roh.map_or("", str::trim);
    if id.is_empty() {
        return Err("ID fehlt: ELSTER_HERSTELLER_ID ist nicht gesetzt (make blockmatrix-rust laedt sie aus der .env)");
    }
    if id == PLATZHALTER_ID || id.chars().all(|c| c == '0') {
        return Err("ID ist der gesperrte Platzhalter: ERiC gaebe rc=610301202 und prueft nichts");
    }
    Ok(id.to_owned())
}

fn hersteller_id() -> String {
    pruefe_id(std::env::var("ELSTER_HERSTELLER_ID").ok().as_deref()).unwrap_or_else(|g| panic!("{g}"))
}

fn voraussetzungen() -> String {
    assert!(
        elster::find_eric_lib().is_some(),
        "Bibliothek fehlt: libericapi.so liegt weder unter $ERIC_DIR noch unter ~/02_Software/eric"
    );
    assert!(schemas_da(2025), "ERiC-Schema fuer VZ 2025 fehlt: ohne es baut der Writer kein XML");
    hersteller_id()
}

fn setze(s: &mut Store, feld: &str, wert: Value) {
    let leer = HashMap::new();
    s.append_roh(
        &NeuesEventRoh {
            feld_id: feld.to_owned(),
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: HerkunftVektor::Voll(Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: Some(format!("ok@{feld}")),
                signal_2_fehlt: false,
            },
            signal_2_fremd: None,
            ersetzt: None,
            ts: Some("2026-09-01T00:00:00+00:00".to_owned()),
        },
        None,
        BindungNachschlag::neu(&leer),
    )
    .unwrap();
}

/// Kegel einer vollstaendigen `gesamt`-Erklaerung der Angestellten wie `basis` in `api/tests/einreichen_eric_echt.rs`
/// (Gegenstueck zu `_BASIS_A` in `tests/test_checkest_durchstich.py`), Cent. Allein ist sie `rc=0`.
fn basis() -> Vec<(&'static str, Value)> {
    vec![
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("vor_an_anteil_rv", json!(4_200_000)),
        ("vor_ag_anteil_rv", json!(1_200_000)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("stammdaten_nachname", json!("Maier")),
        ("stammdaten_vorname", json!("Hans")),
        ("stammdaten_geburtsdatum", json!("05.05.1955")),
        ("stammdaten_strasse", json!("Musterstr.")),
        ("stammdaten_hausnummer", json!("55")),
        ("stammdaten_plz", json!("55555")),
        ("stammdaten_wohnort", json!("Musterort")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("kist_konfession", json!("keine")),
        // Praefix 9181 muss zur Finanzamtsnummer des Empfaengers passen, sonst baut der Dienst kein XML.
        ("stammdaten_steuernummer", json!("9181081508155")),
        ("steuerklasse", json!("1")),
        ("p36_lohnsteuer", json!(1_200_000)),
        ("veranlagung", json!("einzel")),
    ]
}

fn leere_akte() -> Store {
    leere_akte_vz(2025)
}

fn leere_akte_vz(vz: i64) -> Store {
    Store::aus_datei(
        serde_json::from_value(
            json!({"version": 1, "veranlagungszeitraum": vz, "scheibe": "gesamt", "events": []}),
        )
        .unwrap(),
    )
}

/// Die `<Text>`-Meldungen aus der `ERiC`-Antwort, Leerraum zusammengezogen, die ID ersetzt.
fn meldungen(antwort: &str, id: &str) -> Vec<String> {
    let mut aus = Vec::new();
    let mut rest = antwort;
    while let Some(von) = rest.find("<Text>") {
        let nach = &rest[von + "<Text>".len()..];
        let Some(bis) = nach.find("</Text>") else { break };
        let t: String = nach[..bis].split_whitespace().collect::<Vec<_>>().join(" ");
        aus.push(t.replace(id, "<ID>"));
        rest = &nach[bis + "</Text>".len()..];
    }
    aus
}

struct Urteil {
    rc: i32,
    klasse: EricKlasse,
    meldungen: Vec<String>,
}

impl Urteil {
    fn plausibel(&self) -> bool {
        self.rc == 0 && self.meldungen.is_empty()
    }

    fn zeile(&self) -> String {
        format!(
            "rc={} klasse={:?} meldungen={} | {}",
            self.rc,
            self.klasse,
            self.meldungen.len(),
            self.meldungen.iter().take(3).cloned().collect::<Vec<_>>().join(" / ")
        )
    }
}

fn urteil(xml: &str, id: &str) -> Urteil {
    urteil_vz(xml, id, 2025)
}

/// Wie [`urteil`], fuer die Datenart `ESt_<vz>`.
fn urteil_vz(xml: &str, id: &str, vz: i64) -> Urteil {
    let (rc, antwort) = validiere(xml.as_bytes(), &format!("ESt_{vz}"))
        .unwrap_or_else(|e| panic!("ERiC laedt nicht oder bricht ab: {e:?}"));
    Urteil { rc, klasse: klassifiziere_rc(i64::from(rc)), meldungen: meldungen(&antwort, id) }
}

/// Eine Akte aus Basis plus `zusatz` (ueberschreibt gleichnamige Basisfelder), ueber den echten Abgabeweg zum XML. `Err` nennt den
/// Grund, aus dem der Guard oder der Writer kein XML baut.
fn block_xml(zusatz: &[(&str, Value)], id: &str) -> Result<String, String> {
    let mut s = leere_akte();
    let mut paare = basis();
    for (f, w) in zusatz {
        paare.retain(|(g, _)| g != f);
        paare.push((f, w.clone()));
    }
    for (f, w) in paare {
        setze(&mut s, f, w);
    }
    einreichungs_xml(&s, index(), params(), "BY", Some(id.to_owned()))
        .map(|e| e.xml)
        .map_err(|f| f.to_string().replace(id, "<ID>"))
}

/// Antworten, die der Rust-Guard (`an_gesamt_sperrgrund`) zusaetzlich verlangt. Der Python-Test rief `deklariere` und `erzeuge_xml`
/// OHNE Guard; der Rust-Weg ist strenger und naeher am Nutzerweg (`sperre/gesamt.rs`, `sperre/werbungskosten.rs`).
fn guard_antworten(block: &str) -> Vec<(&'static str, Value)> {
    match block {
        "anlage_v_vermietung" => vec![("kein_vuv", json!(false))],
        "kap_mit_auslandssteuer" | "kap_mit_steuerabzug" => vec![("kein_kap", json!(false))],
        "p35a_handwerker" => vec![
            ("hh_rechnung_unbar", json!(true)),
            ("hh_handwerker_keine_foerderung", json!(true)),
            ("hh_in_eu_ewr", json!(true)),
        ],
        "kinderbetreuung" => vec![
            ("kind_betreuung_reine_betreuung", json!(true)),
            ("kind_betreuung_rechnung_ueberweisung", json!(true)),
        ],
        "verpflegung" => vec![("vpf_monate_am_ort", json!(2))],
        _ => Vec::new(),
    }
}

/// Die Kindfelder, die `kinderbetreuung` und `anlage_kind_instanz` teilen (minimal vollstaendige Anlage Kind).
fn kind_felder() -> Vec<(&'static str, Value)> {
    vec![
        ("fam_anzahl_kinder", json!(1)),
        ("kind_idnr", json!("12345678911")),
        ("kind_vorname", json!("Anna")),
        ("kind_geburtsdatum", json!("15.03.2015")),
        ("kind_familienkasse", json!("Familienkasse Bayern Nord")),
        ("kind_wohnsitz_inland_zeitraum", json!("01.01-31.12")),
        ("kind_kindschaftsverhaeltnis_a", json!("1")),
        ("kind_kindschaftsverh_zeitraum_a", json!("01.01-31.12")),
        ("kind_anderer_elternteil_name", json!("Michael Beispiel")),
        ("kind_anderer_elternteil_geburtsdatum", json!("01.01.1985")),
        ("kind_anderer_elternteil_kindschaftsverhaeltnis", json!("1")),
        ("kind_anderer_elternteil_zeitraum", json!("01.01-31.12")),
    ]
}

/// Die zwoelf Bloecke der Python-Matrix (`BLOECKE` in `tests/test_checkest_blockmatrix.py`), je mit den Feldern, die den Block
/// minimal vollstaendig machen. Betraege in Cent.
fn bloecke() -> Vec<(&'static str, Vec<(&'static str, Value)>)> {
    let mut betreuung = kind_felder();
    betreuung.extend([
        ("kind_unter_14_haushaltszugehoerig", json!(true)),
        ("kinderbetreuungskosten", json!(200_000)),
        ("kind_betreuung_dienstleister", json!("Kindertagesstätte Sonnenschein, Musterstr. 1, 12345 Musterstadt")),
        ("kind_betreuung_zeitraum", json!("01.01-31.12")),
        ("kind_betreuung_eigenanteil", json!(200_000)),
        ("kind_betreuung_kein_gemeinsamer_haushalt_zeitraum", json!("01.01-31.12")),
        ("kind_betreuung_haushaltszugehoerigkeit_zeitraum", json!("01.01-31.12")),
        ("kind_betreuung_einzelbetrag", json!(200_000)),
        ("kind_betreuung_eigenanteil_betrag", json!(200_000)),
        ("kind_betreuung_eigenanteil_zeitraum", json!("01.01-31.12")),
    ]);
    let pflege_gemeinsam = [
        ("rentner_gepflegter_wohnsitz_inland", json!(true)),
        ("rentner_pflege_weitere_personen", json!(0)),
        ("rentner_gepflegter_idnr", json!("12345678911")),
        ("rentner_gepflegter_angaben", json!("Maria Muster, Musterweg 3, 12345 Musterstadt, meine Mutter")),
        ("rentner_pflege_durch", json!("1")),
    ];
    let mut merkzeichen_h = vec![("rentner_gepflegter_hilflos", json!(true))];
    merkzeichen_h.extend(pflege_gemeinsam.clone());
    let mut pauschbetrag = vec![("rentner_pflegegrad", json!(4))];
    pauschbetrag.extend(pflege_gemeinsam);
    vec![
        ("anlage_kind_instanz", kind_felder()),
        ("anlage_v_vermietung", vec![
            ("vv_einnahmen", json!(1_200_000)),
            ("vv_wohnzwecke", json!(true)),
            ("vv_auf_dauer", json!(true)),
            ("vv_objekt_strasse", json!("Mietweg 7")),
            ("vv_objekt_plz", json!("12345")),
            ("vv_objekt_ort", json!("Musterstadt")),
            ("vv_wohneinheit_bezeichnung", json!("1. OG links")),
            ("vv_nebenkosten_nicht_vereinbart", json!(true)),
            ("vv_nutzung_ferienwohnung", json!(false)),
            ("vv_nutzung_an_angehoerige", json!(false)),
            ("vv_nutzung_kurzfristig", json!(false)),
            ("vv_gebaeude_afa", json!(200_000)),
            ("vv_schuldzinsen", json!(0)),
            ("vv_erhaltungsaufwand", json!(0)),
            ("vv_sonstige_wk", json!(0)),
        ]),
        ("kap_mit_auslandssteuer", vec![
            ("kap_kapitalertraege", json!(500_000)),
            ("kap_q_auslaendische_steuer", json!(100_000)),
        ]),
        ("kap_mit_steuerabzug", vec![
            ("kap_kapitalertraege", json!(500_000)),
            ("p36_kapitalertragsteuer", json!(125_000)),
            ("p36_kapitalertragsteuer_solz", json!(6_875)),
        ]),
        ("kinderbetreuung", betreuung),
        ("kinderfreibetrag_ohne_anlage", vec![("fam_anzahl_kinder", json!(1))]),
        ("p35a_handwerker", vec![
            ("hh_handwerker_betrag", json!(300_000)),
            ("hh_handwerker_art", json!("Malerarbeiten")),
        ]),
        ("p35c_anlage_energetische_massnahmen", vec![
            ("p35c_sanierungsaufwendungen", json!(2_000_000)),
            ("p35c_keine_doppelfoerderung", json!(true)),
            ("p35c_objekt_strasse", json!("Musterweg 3")),
            ("p35c_objekt_plz_ort", json!("12345 Musterstadt")),
            ("p35c_gebaeude_herstellungsbeginn", json!("01.06.1995")),
            ("p35c_baubeginn_massnahme", json!("15.03.2025")),
            ("p35c_gesamtflaeche_qm", json!(120)),
            ("p35c_eigene_wohnflaeche_qm", json!(120)),
            ("p35c_bereits_ermaessigung_frueher", json!(false)),
            ("p35c_massnahme_art", json!("heizung")),
        ]),
        ("pflege_merkzeichen_h", merkzeichen_h),
        ("pflege_pauschbetrag", pauschbetrag),
        ("realsplitting", vec![
            ("realsplitting_unterhaltsleistungen", json!(1_200_000)),
            ("realsplitting_empfaenger_kv_pv", json!(180_000)),
            ("realsplitting_empfaenger_kv_krankengeld", json!(150_000)),
        ]),
        ("verpflegung", vec![
            ("tage_24h", json!(10)),
            ("vpf_fruehstuecke_gestellt_anzahl", json!(5)),
        ]),
    ]
}

/// Zwoelf Bloecke gegen das echte `checkESt`, mit Kontrollen davor. Alle roten Bloecke werden genannt, nicht nur der erste.
#[test]
#[ignore = "braucht die ERiC-Bibliothek und die registrierte Hersteller-ID (Umgebungs-Gate, Entscheid Julius 2026-09-12): lokal mit `make blockmatrix-rust`"]
fn zwoelf_bloecke_gegen_echtes_checkest() {
    let id = voraussetzungen();

    // --- Kontrollen: ERiC wurde erreicht und beanstandet, was es beanstanden muss.
    let k0 = urteil(&block_xml(&[], &id).expect("K0: die Basis allein baut kein XML"), &id);
    eprintln!("{:<40} {}", "K0_nur_basis", k0.zeile());
    assert!(k0.plausibel(), "K0: die Basis allein ist nicht rc=0 (ID gesperrt oder ERiC defekt?): {}", k0.zeile());
    let rote_kontrollen = [
        ("K1_kind_ohne_vorname", vec![("fam_anzahl_kinder", json!(1)), ("kind_idnr", json!("12345678911"))]),
        (
            "K2_anlage_v_nur_einnahmen",
            vec![("kein_vuv", json!(false)), ("vv_einnahmen", json!(1_200_000))],
        ),
    ];
    for (name, felder) in rote_kontrollen {
        let u = urteil(&block_xml(&felder, &id).unwrap_or_else(|g| panic!("{name}: kein XML: {g}")), &id);
        eprintln!("{name:<40} {}", u.zeile());
        assert!(
            u.rc == RC_PLAUSIBILITAET && !u.meldungen.is_empty(),
            "{name}: ERiC muss rc={RC_PLAUSIBILITAET} mit Meldung geben, gab: {}",
            u.zeile()
        );
    }

    // --- Die Bloecke.
    let bloecke = bloecke();
    assert_eq!(bloecke.len(), ANZAHL_BLOECKE, "die Matrix hat {ANZAHL_BLOECKE} Bloecke");
    let mut rot = Vec::new();
    for (name, felder) in bloecke {
        let mut alle = guard_antworten(name);
        alle.extend(felder);
        let (gruen, zeile) = match block_xml(&alle, &id) {
            Ok(xml) => {
                let u = urteil(&xml, &id);
                (u.plausibel(), u.zeile())
            }
            Err(grund) => (false, format!("KEIN XML: {grund}")),
        };
        eprintln!("{name:<40} {zeile}");
        if !gruen {
            rot.push(name);
        }
    }
    assert!(rot.is_empty(), "{} von {ANZAHL_BLOECKE} Bloecken nicht rc=0: {rot:?}", rot.len());
}

// ----------------------------------------------------------------------------- Nr. 48 und Nr. 49: die Zeilen "Sonstiges"

const WAHL: &str = "dba_abzug_statt_anrechnung";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const ART: &str = "dba_einkunftsart";
const UNFALL: &str = "ep_unfallkosten";
/// `Bezeichnung`, `Betrag` der Zeile "Sonstiges" und die Summe der weiteren Werbungskosten (Anlage N).
const SONST_KZ: [&str; 3] = ["E0205405", "E0205406", "E0204803"];

/// Die e2e-artige Akte: Basis plus ein Arbeitsweg, dazu auf Wunsch Unfallkosten (Cent) und der Abzug der auslaendischen Steuer
/// (`Some(Steuer in Cent)`, Wahl Abzug).
fn zeilen_akte(unfall: Option<i64>, abzug: Option<i64>) -> Store {
    zeilen_akte_vz(2025, unfall, abzug)
}

fn zeilen_akte_vz(vz: i64, unfall: Option<i64>, abzug: Option<i64>) -> Store {
    zeilen_akte_voll(vz, unfall, abzug, 500_000)
}

/// Wie [`zeilen_akte_vz`], mit den Auslandseinkuenften in Cent (nur mit Abzug gesetzt).
fn zeilen_akte_voll(vz: i64, unfall: Option<i64>, abzug: Option<i64>, einkuenfte: i64) -> Store {
    let mut s = leere_akte_vz(vz);
    for (f, w) in basis() {
        setze(&mut s, f, w);
    }
    for (f, w) in [
        ("ep_entfernung_km", json!(30)),
        ("ep_arbeitstage", json!(220)),
        ("ep_eigenes_kfz", json!(true)),
        ("ep_ziel_des_weges", json!("1")),
        ("ep_ziel_adresse", json!("80331 München, Marienplatz 1")),
    ] {
        setze(&mut s, f, w);
    }
    if let Some(cent) = unfall {
        setze(&mut s, UNFALL, json!(cent));
    }
    if let Some(steuer) = abzug {
        setze(&mut s, EINKUENFTE, json!(einkuenfte));
        setze(&mut s, ART, json!("unselbstaendige_arbeit"));
        setze(&mut s, "dba_staat", json!("Frankreich"));
        setze(&mut s, "dba_mehrere_staaten", json!(false));
        setze(&mut s, STEUER, json!(steuer));
        setze(&mut s, WAHL, json!(true));
    }
    s
}

/// Die Deklaration einer Zeilen-Akte, so wie die Produktion sie baut: Ring (`mit_ring_werten`), dann `deklariere`.
fn deklaration(unfall: Option<i64>, abzug: Option<i64>) -> (Deklaration, Felder) {
    let (mut felder, _) = zeilen_akte(unfall, abzug).materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    (d, felder)
}

/// Abgabe-XML wie `einreichungs_xml` (abgabefaehig, Snapshot), aber aus einer Deklaration, die der Test zusammengesetzt hat.
fn xml_abgabe(d: &Deklaration, felder: &Felder, id: &str) -> String {
    erzeuge_xml(
        d,
        &XmlOptionen {
            hersteller_id: Some(id.to_owned()),
            abgabefaehig: true,
            snapshot: Some(felder),
            ..XmlOptionen::default()
        },
    )
    .unwrap_or_else(|e| panic!("erzeuge_xml scheitert: {}", e.to_string().replace(id, "<ID>")))
}

/// Der echte Abgabeweg einer Zeilen-Akte: Guard, Ring, `deklariere`, `erzeuge_xml` (`einreichungs_xml`), kein Umweg. `Err` nennt
/// den Grund, aus dem der Guard, eine Sperre oder der Writer kein XML baut.
fn echter_abgabeweg(unfall: Option<i64>, abzug: Option<i64>, id: &str) -> Result<String, String> {
    echter_abgabeweg_vz(2025, unfall, abzug, id)
}

fn echter_abgabeweg_vz(vz: i64, unfall: Option<i64>, abzug: Option<i64>, id: &str) -> Result<String, String> {
    echter_abgabeweg_ek(vz, unfall, abzug, 500_000, id)
}

/// Wie [`echter_abgabeweg_vz`], mit den Auslandseinkuenften in Cent.
fn echter_abgabeweg_ek(vz: i64, unfall: Option<i64>, abzug: Option<i64>, einkuenfte: i64, id: &str) -> Result<String, String> {
    einreichungs_xml(&zeilen_akte_voll(vz, unfall, abzug, einkuenfte), index(), params(), "BY", Some(id.to_owned()))
        .map(|e| e.xml)
        .map_err(|f| f.to_string().replace(id, "<ID>"))
}

/// Fall mit Zahl der erwarteten `<Sonst>`-Zeilen im XML, ueber den echten Abgabeweg; `rc=0` und 0 Meldungen.
fn pruefe_zeilen(name: &str, unfall: Option<i64>, abzug: Option<i64>, sonst: usize, id: &str) {
    let xml = echter_abgabeweg(unfall, abzug, id)
        .unwrap_or_else(|g| panic!("{name}: kein XML ueber den echten Abgabeweg: {g}"));
    let n = xml.matches("<Sonst>").count();
    let u = urteil(&xml, id);
    eprintln!("{name:<40} sonst={n} {}", u.zeile());
    assert_eq!(n, sonst, "{name}: erwartet {sonst} Zeilen <Sonst> im XML, gefunden {n}");
    assert!(u.plausibel(), "{name}: nicht rc=0: {}", u.zeile());
}

/// Die Zeilen "Sonstiges" der Anlage N (Unfallkosten Nr. 48, § 34c-Abzug Nr. 49) gegen das echte `checkESt`: einzeln, zusammen,
/// und mit Kontrollen, die `ERiC` auf genau diese Zeile beanstanden muss.
#[test]
#[ignore = "braucht die ERiC-Bibliothek und die registrierte Hersteller-ID (Umgebungs-Gate, Entscheid Julius 2026-09-12): lokal mit `make blockmatrix-rust`"]
fn zeilen_nr48_nr49_gegen_echtes_checkest() {
    let id = voraussetzungen();

    // Kontrolle: die Akte ohne beide Zeilen ist einreichbar, ihr XML hat keine Zeile "Sonstiges".
    pruefe_zeilen("K0_arbeitsweg_ohne_zeilen", None, None, 0, &id);

    // Nr. 48 allein: 150.001 Cent Unfallkosten, aufgerundet 1.501 EUR. Die Sperre ist gefallen (Julius, 2026-10-10): der ECHTE
    // Abgabeweg (Guard, Ring, deklariere, erzeuge_xml) baut das XML, ohne Umweg.
    let echt = echter_abgabeweg(Some(150_001), None, &id)
        .unwrap_or_else(|g| panic!("Nr48_unfallkosten: kein XML ueber den echten Abgabeweg: {g}"));
    assert!(echt.contains("<E0205406>1501</E0205406>"), "Nr48_unfallkosten: die Zeile steht nicht im XML");
    pruefe_zeilen("Nr48_unfallkosten", Some(150_001), None, 1, &id);

    // Kontrollen zu Nr. 48: dieselbe Zeile ohne Bezeichnung und mit falscher Summe beanstandet ERiC, mit dem Text der Zeile.
    // Sie brauchen die Deklaration (Kz entfernen), nicht das fertige XML.
    let (sauber, felder) = deklaration(Some(150_001), None);
    assert!(sauber.unvollstaendig().is_empty(), "Nr. 48 sperrt nicht mehr: {:?}", sauber.unvollstaendig());
    let mut ohne_text = sauber.clone();
    ohne_text.deklaration.remove(SONST_KZ[0]);
    let u = urteil(&xml_abgabe(&ohne_text, &felder, &id), &id);
    eprintln!("{:<40} {}", "K_ohne_bezeichnung", u.zeile());
    assert!(
        u.rc == RC_PLAUSIBILITAET && u.meldungen.iter().any(|m| m.contains("sonstigen Werbungskosten")),
        "Kontrolle ohne Bezeichnung: ERiC muss die Zeile beanstanden, gab: {}",
        u.zeile()
    );
    let mut falsche_summe = sauber;
    falsche_summe.deklaration.insert(SONST_KZ[2].to_owned(), json!(9_999));
    let u = urteil(&xml_abgabe(&falsche_summe, &felder, &id), &id);
    eprintln!("{:<40} {}", "K_falsche_summe", u.zeile());
    assert!(
        u.rc == RC_PLAUSIBILITAET && u.meldungen.iter().any(|m| m.contains("Summe der weiteren Werbungskosten")),
        "Kontrolle falsche Summe: ERiC muss die Summe nachrechnen, gab: {}",
        u.zeile()
    );

    // Nr. 49 allein: Abzug statt Anrechnung, 70.001 Cent Steuer, aufgerundet 701 EUR, ueber den echten Abgabeweg. Zeile 10 der
    // Anlage AUS (`E0600920`) steht mit 701, Zeile 7 (`E0601401`) NACH dem Abzug mit 5.000 - 701 = 4.299, die Anrechnung fehlt.
    pruefe_zeilen("Nr49_abzug", None, Some(70_001), 1, &id);
    let echt = echter_abgabeweg(None, Some(70_001), &id)
        .unwrap_or_else(|g| panic!("Nr49_abzug: kein XML ueber den echten Abgabeweg: {g}"));
    assert!(echt.contains("<E0600920>701</E0600920>"), "Nr49_abzug: Zeile 10 steht nicht mit 701 im XML");
    assert!(echt.contains("<E0601401>4299</E0601401>"), "Nr49_abzug: Zeile 7 steht nicht nach dem Abzug im XML");
    assert!(!echt.contains("<E0601901>"), "Nr49_abzug: Anrechnung neben dem Abzug");

    // Zeile 7 negativ: 500 Euro Einkuenfte, 700,01 Euro Steuer -> Zeile 7 = 500 - 701 = -201. Die Anleitung nennt negative
    // Einkuenfte durch den Abzug ausdruecklich; das Schema erlaubt sie, `ERiC` muss sie annehmen.
    let echt = echter_abgabeweg_ek(2025, None, Some(70_001), 50_000, &id)
        .unwrap_or_else(|g| panic!("Nr49_abzug_negativ: kein XML ueber den echten Abgabeweg: {g}"));
    assert!(echt.contains("<E0601401>-201</E0601401>"), "Nr49_abzug_negativ: Zeile 7 steht nicht mit -201 im XML");
    let u = urteil(&echt, &id);
    eprintln!("{:<40} {}", "Nr49_abzug_zeile7_negativ", u.zeile());
    assert!(u.plausibel(), "Nr49_abzug_negativ: nicht rc=0: {}", u.zeile());

    // Nr. 48 und Nr. 49 zusammen: zwei Zeilen unter einem Weitere_Wk, die Summe ist die der gerundeten Zeilen (1.501 + 701 = 2.202).
    pruefe_zeilen("Nr48_und_Nr49", Some(150_001), Some(70_001), 2, &id);
}

/// Nr. 48 und Nr. 49 fuer den Veranlagungszeitraum 2024 (`ESt_2024`): dieselben Zeilen "Sonstiges" und der Abzug in der Anlage AUS
/// gegen das echte `checkESt`, ueber den echten Abgabeweg. Die Sperren galten fuer beide Jahre; ohne diesen Lauf fielen sie fuer 2024
/// ungemessen. Kontrollen: die Akte ohne die Zeile ist einreichbar (sonst liesse ein Fehler sich nicht der Zeile zuschreiben), und
/// dieselbe Zeile ohne Bezeichnung beanstandet `ERiC`.
#[test]
#[ignore = "braucht die ERiC-Bibliothek und die registrierte Hersteller-ID (Umgebungs-Gate, Entscheid Julius 2026-09-12): lokal mit `make blockmatrix-rust`"]
fn zeilen_nr48_nr49_vz2024_gegen_echtes_checkest() {
    let id = voraussetzungen();
    assert!(schemas_da(2024), "ERiC-Schema fuer VZ 2024 fehlt: ohne es baut der Writer kein XML");

    let ohne = echter_abgabeweg_vz(2024, None, None, &id)
        .unwrap_or_else(|g| panic!("K0_vz2024: kein XML ueber den echten Abgabeweg: {g}"));
    assert!(!ohne.contains("<Sonst>"), "K0_vz2024: Zeile Sonstiges ohne Unfallkosten");
    let u = urteil_vz(&ohne, &id, 2024);
    eprintln!("{:<40} {}", "K0_vz2024_arbeitsweg_ohne_zeile", u.zeile());
    assert!(u.plausibel(), "K0_vz2024: nicht rc=0: {}", u.zeile());

    let xml = echter_abgabeweg_vz(2024, Some(150_001), None, &id)
        .unwrap_or_else(|g| panic!("Nr48_vz2024: kein XML ueber den echten Abgabeweg: {g}"));
    assert!(xml.contains("<E0205406>1501</E0205406>"), "Nr48_vz2024: die Zeile steht nicht im XML");
    assert_eq!(xml.matches("<Sonst>").count(), 1, "Nr48_vz2024: erwartet eine Zeile <Sonst>");
    let u = urteil_vz(&xml, &id, 2024);
    eprintln!("{:<40} {}", "Nr48_vz2024_unfallkosten", u.zeile());
    assert!(u.plausibel(), "Nr48_vz2024: nicht rc=0: {}", u.zeile());

    let bezeichnung = "<E0205405>Unfallkosten Arbeitsweg</E0205405>";
    assert!(xml.contains(bezeichnung), "Nr48_vz2024: die Bezeichnung steht nicht wie erwartet im XML");
    let u = urteil_vz(&xml.replacen(bezeichnung, "", 1), &id, 2024);
    eprintln!("{:<40} {}", "K_vz2024_ohne_bezeichnung", u.zeile());
    assert!(
        u.rc == RC_PLAUSIBILITAET && u.meldungen.iter().any(|m| m.contains("sonstigen Werbungskosten")),
        "Kontrolle VZ 2024 ohne Bezeichnung: ERiC muss die Zeile beanstanden, gab: {}",
        u.zeile()
    );

    // Nr. 49 und Nr. 41 mit den Unfallkosten: zwei Zeilen, Summe 1.501 + 701 = 2.202, Zeile 10 mit 701, Zeile 7 nach dem Abzug.
    let xml = echter_abgabeweg_vz(2024, Some(150_001), Some(70_001), &id)
        .unwrap_or_else(|g| panic!("Nr49_vz2024: kein XML ueber den echten Abgabeweg: {g}"));
    assert_eq!(xml.matches("<Sonst>").count(), 2, "Nr49_vz2024: erwartet zwei Zeilen <Sonst>");
    assert!(xml.contains("<E0204803>2202</E0204803>"), "Nr49_vz2024: Summe nicht 2202");
    assert!(xml.contains("<E0600920>701</E0600920>"), "Nr49_vz2024: Zeile 10 steht nicht mit 701 im XML");
    assert!(xml.contains("<E0601401>4299</E0601401>"), "Nr49_vz2024: Zeile 7 steht nicht nach dem Abzug im XML");
    let u = urteil_vz(&xml, &id, 2024);
    eprintln!("{:<40} {}", "Nr48_Nr49_vz2024", u.zeile());
    assert!(u.plausibel(), "Nr49_vz2024: nicht rc=0: {}", u.zeile());
}

// ----------------------------------------------------------------------------- Anlage AUS: Staat, Anrechnung, Nr. 41

/// Anlage AUS, `Staat_Spez_InvFonds`: Staat (Abweichung Nr. 51), Einkuenfte, anzurechnende Steuer.
const STAAT_KZ: &str = "E0600301";
const EINKUENFTE_KZ: &str = "E0601401";
const ANRECHNUNG_KZ: &str = "E0601901";

/// Die Auslandsangaben der Anrechnung: 5.000 Euro aus Arbeitslohn in Frankreich, `steuer` in Cent.
fn ausland_felder(steuer: i64) -> Vec<(&'static str, Value)> {
    vec![
        (ART, json!("unselbstaendige_arbeit")),
        (EINKUENFTE, json!(500_000)),
        ("dba_staat", json!("Frankreich")),
        ("dba_mehrere_staaten", json!(false)),
        (STEUER, json!(steuer)),
    ]
}

/// Die Deklaration der Anrechnung (Basis plus Auslandsangaben), so wie die Produktion sie baut: Ring, dann `deklariere`. Sie
/// sperrt nicht; die Kontrollen unten nehmen aus ihr Kz heraus und lassen `ERiC` urteilen.
fn anrechnung_deklaration() -> (Deklaration, Felder) {
    let mut s = leere_akte();
    for (f, w) in basis().into_iter().chain(ausland_felder(70_000)) {
        setze(&mut s, f, w);
    }
    let (mut felder, _) = s.materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    assert!(d.unvollstaendig().is_empty(), "die Anrechnung sperrt schon: {:?}", d.unvollstaendig());
    (d, felder)
}

/// Die Anlage AUS gegen das echte `checkESt` (Abweichung Nr. 51 und Nr. 41): der Block "Ausland Anrechnung" ueber den echten
/// Abgabeweg, der Abzug nach § 34c Abs. 2 (`E0600920`) ebenso, und Kontrollen, die `ERiC` beanstanden MUSS. Der Staat
/// steht genau dann, wenn die Einkuenfte stehen (auch mit dem Wert 0); Steuer ohne Einkuenfte lehnt `ERiC` auch mit Staat ab.
#[test]
#[ignore = "braucht die ERiC-Bibliothek und die registrierte Hersteller-ID (Umgebungs-Gate, Entscheid Julius 2026-09-12): lokal mit `make blockmatrix-rust`"]
fn ausland_staat_und_nr41_gegen_echtes_checkest() {
    let id = voraussetzungen();

    // Block "Ausland Anrechnung": der echte Abgabeweg (Guard, Ring, deklariere, erzeuge_xml), nicht gesperrt.
    let xml = block_xml(&ausland_felder(70_000), &id).unwrap_or_else(|g| panic!("Ausland_Anrechnung: kein XML: {g}"));
    assert!(xml.contains("<E0600301>Frankreich</E0600301>"), "Ausland_Anrechnung: der Staat steht nicht im XML");
    assert!(xml.contains("<E0601901>700</E0601901>"), "Ausland_Anrechnung: die Steuer steht nicht unter E0601901");
    let u = urteil(&xml, &id);
    eprintln!("{:<40} {}", "Ausland_Anrechnung", u.zeile());
    assert!(u.plausibel(), "Ausland_Anrechnung: nicht rc=0: {}", u.zeile());

    // Kontrollen: ohne die Paarung beanstandet ERiC die Akte, mit dem Text der Zeile. Aus der Anrechnung wird je Kontrolle ein
    // Kz-Satz entfernt.
    let (d, felder) = anrechnung_deklaration();
    let kontrollen: [(&str, &[&str], &str); 3] = [
        ("K_Staat_ohne_Einkuenfte", &[EINKUENFTE_KZ, ANRECHNUNG_KZ], "nicht erklärt"),
        ("K_Einkuenfte_ohne_Staat", &[STAAT_KZ], "aus welchem Staat"),
        ("K_Steuer_ohne_Einkuenfte", &[EINKUENFTE_KZ], "keine Einkünfte"),
    ];
    for (name, entfernt, soll) in kontrollen {
        let mut m = d.clone();
        for kz in entfernt {
            assert!(m.deklaration.remove(*kz).is_some(), "{name}: {kz} stand nicht in der Anrechnung");
        }
        let u = urteil(&xml_abgabe(&m, &felder, &id), &id);
        eprintln!("{name:<40} {}", u.zeile());
        assert!(
            u.rc == RC_PLAUSIBILITAET && u.meldungen.iter().any(|x| x.contains(soll)),
            "{name}: ERiC muss rc={RC_PLAUSIBILITAET} mit `{soll}` geben, gab: {}",
            u.zeile()
        );
    }

    // Nullwerte: "erklaert" heisst vorhanden, auch mit 0 (gemessen 2026-10-10). Einkuenfte 0 mit Staat sind rc=0, ohne Staat nicht.
    let mut null_mit_staat = d.clone();
    null_mit_staat.deklaration.remove(ANRECHNUNG_KZ);
    null_mit_staat.deklaration.insert(EINKUENFTE_KZ.to_owned(), json!(0));
    let u = urteil(&xml_abgabe(&null_mit_staat, &felder, &id), &id);
    eprintln!("{:<40} {}", "Einkuenfte_null_mit_Staat", u.zeile());
    assert!(u.plausibel(), "Einkuenfte_null_mit_Staat: nicht rc=0: {}", u.zeile());
    let mut null_ohne_staat = null_mit_staat;
    null_ohne_staat.deklaration.remove(STAAT_KZ);
    let u = urteil(&xml_abgabe(&null_ohne_staat, &felder, &id), &id);
    eprintln!("{:<40} {}", "K_Einkuenfte_null_ohne_Staat", u.zeile());
    assert!(
        u.rc == RC_PLAUSIBILITAET && u.meldungen.iter().any(|x| x.contains("aus welchem Staat")),
        "K_Einkuenfte_null_ohne_Staat: ERiC muss den fehlenden Staat beanstanden, gab: {}",
        u.zeile()
    );

    // Nr. 41 und Nr. 49: der Abzug in der Anlage AUS (Zeile 10, `E0600920`, mit Staat und Einkuenften) UND als Zeile "Sonstiges" der
    // Anlage N, ueber den echten Abgabeweg (die Sperre ist gefallen, Julius 2026-10-10). Zeile 7 steht nach dem Abzug.
    let xml = echter_abgabeweg(None, Some(70_001), &id)
        .unwrap_or_else(|g| panic!("Nr41_und_Nr49: kein XML ueber den echten Abgabeweg: {g}"));
    assert!(xml.contains("<E0600920>701</E0600920>"), "Nr41_und_Nr49: die Steuer steht nicht unter E0600920");
    assert!(xml.contains("<E0600301>Frankreich</E0600301>"), "Nr41_und_Nr49: der Staat steht nicht im XML");
    assert_eq!(xml.matches("<Sonst>").count(), 1, "Nr41_und_Nr49: erwartet eine Zeile <Sonst>");
    let u = urteil(&xml, &id);
    eprintln!("{:<40} {}", "Nr41_und_Nr49", u.zeile());
    assert!(u.plausibel(), "Nr41_und_Nr49: nicht rc=0: {}", u.zeile());

    // Kontrolle: derselbe Abzug ohne Staat beanstandet `ERiC`.
    let (mut d, felder) = deklaration(None, Some(70_001));
    assert!(d.unvollstaendig().is_empty(), "der getragene Abzug sperrt: {:?}", d.unvollstaendig());
    assert!(d.deklaration.remove(STAAT_KZ).is_some(), "K_Nr41_ohne_Staat: {STAAT_KZ} stand nicht im Abzug");
    let u = urteil(&xml_abgabe(&d, &felder, &id), &id);
    eprintln!("{:<40} {}", "K_Nr41_ohne_Staat", u.zeile());
    assert!(
        u.rc == RC_PLAUSIBILITAET && u.meldungen.iter().any(|x| x.contains("aus welchem Staat")),
        "K_Nr41_ohne_Staat: ERiC muss den fehlenden Staat beanstanden, gab: {}",
        u.zeile()
    );
}

// ----------------------------------------------------------------------------- Anlage SO: Privatverkauf (Nr. 54)

// Die Kz, die der Schreiber je Art setzt (`P23_ARTFELDER` und der Gewinn in `elster/src/deklaration.rs`): Grundstueck E0306111,
// E0306201, E0306202, E0306801 (alle vier Pflicht); anderes Wirtschaftsgut E0307101, E0307201, E0307202, E0307701 (Pflicht nur
// E0307101 und E0307701, die Daten schreibt `TaxGraph` freiwillig; Spike AK0). Die Faelle unten nennen sie mit ihrem Wert.

fn so_name(basis: &str, n: u8) -> String {
    if n == 1 {
        basis.to_owned()
    } else {
        format!("{basis}__{n}")
    }
}

/// Ein vollstaendiger steuerpflichtiger Verkauf der Instanz `n` als Antworten: Art `typ`, `(Kauf, Verkauf)` als TT.MM.JJJJ, Betraege in
/// Cent als `[Preis, Anschaffungskosten, AfA, Werbungskosten]`. Beim Grundstueck "keine Eigennutzung", beim anderen Wirtschaftsgut "kein
/// taeglicher Gebrauch, keine Einkunftsquelle".
fn so_verkauf(n: u8, typ: &str, daten: (&str, &str), betraege: [i64; 4]) -> Vec<(String, Value)> {
    let [preis, ak, afa, wk] = betraege;
    let mut k = vec![
        (so_name("p23_veraeusserungs_typ", n), json!(typ)),
        (so_name("p23_bezeichnung", n), json!("Testobjekt")),
        (so_name("p23_anschaffungsdatum", n), json!(daten.0)),
        (so_name("p23_veraeusserungsdatum", n), json!(daten.1)),
        (so_name("p23_veraeusserungspreis", n), json!(preis)),
        (so_name("p23_anschaffung_herstellungskosten", n), json!(ak)),
        (so_name("p23_afa", n), json!(afa)),
        (so_name("p23_werbungskosten", n), json!(wk)),
    ];
    if typ == "anderes_wg" {
        k.push((so_name("p23_taeglicher_gebrauch", n), json!(false)));
        k.push((so_name("p23_einkunftsquelle", n), json!(false)));
    } else {
        k.push((so_name("p23_eigennutzung", n), json!("nein")));
    }
    k.push(("kein_p23_verkauf".to_owned(), json!(false)));
    k
}

/// Die Basis-Akte plus Verkaeufe, ueber den echten Abgabeweg (Guard, Ring, `deklariere`, `erzeuge_xml`) zum XML.
fn so_xml(verkaeufe: &[(String, Value)], id: &str) -> Result<String, String> {
    let zusatz: Vec<(&str, Value)> = verkaeufe.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
    block_xml(&zusatz, id)
}

/// Die Deklaration einer Akte mit Verkaeufen, wie die Produktion sie baut (Ring, dann `deklariere`); die Kontrollen nehmen aus ihr ein
/// Kz heraus und lassen `ERiC` urteilen.
fn so_deklaration(verkaeufe: &[(String, Value)]) -> (Deklaration, Felder) {
    let mut s = leere_akte();
    for (f, w) in basis() {
        setze(&mut s, f, w);
    }
    for (f, w) in verkaeufe {
        setze(&mut s, f, w.clone());
    }
    let (mut felder, _) = s.materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    assert!(d.unvollstaendig().is_empty(), "der Verkauf sperrt schon: {:?}", d.unvollstaendig());
    (d, felder)
}

/// Dieselbe Deklaration ohne das Kz `kz` in der Instanz `index` der Gruppe `p23_veraeusserung`, oder mit dem Wert `neu`, wenn er gegeben ist.
fn so_abgeaendert(d: &Deklaration, index: u64, kz: &str, neu: Option<Value>) -> Deklaration {
    let mut m = d.clone();
    let felder = m
        .anlage_instanzen
        .iter_mut()
        .filter(|(g, _)| g == "p23_veraeusserung")
        .flat_map(|(_, ii)| ii.iter_mut())
        .find(|i| i.index == index)
        .map(|i| &mut i.felder)
        .expect("die Instanz fehlt in der Deklaration");
    match neu {
        Some(w) => assert!(felder.insert(kz.to_owned(), w).is_some(), "{kz} stand nicht in der Instanz"),
        None => assert!(felder.remove(kz).is_some(), "{kz} stand nicht in der Instanz"),
    }
    m
}

/// Je Fall: Name, die Antworten, die Kz mit ihrem Wert im XML.
type SoFall<'a> = (&'a str, Vec<(String, Value)>, Vec<(&'a str, &'a str)>);

/// Der Privatverkauf (Anlage SO) gegen das echte `checkESt`: was der Schreiber fuer steuerpflichtige Verkaeufe baut, ist `rc=0` mit
/// 0 Meldungen, und `ERiC` beanstandet, was fehlt oder nicht zu den Einzelbetraegen passt. Jedes Kz im XML wird mit seinem Wert
/// geprueft: ein verschluckter oder vertauschter Wert laesst den Lauf nicht gruen werden.
#[test]
#[ignore = "braucht die ERiC-Bibliothek und die registrierte Hersteller-ID (Umgebungs-Gate, Entscheid Julius 2026-09-12): lokal mit `make blockmatrix-rust`"]
fn privatverkauf_gegen_echtes_checkest() {
    let id = voraussetzungen();
    let grundstueck = |daten: (&str, &str), betraege: [i64; 4]| so_verkauf(1, "grundstueck", daten, betraege);
    let wirtschaftsgut = |n: u8, betraege: [i64; 4]| so_verkauf(n, "anderes_wg", ("15.02.2025", "20.08.2025"), betraege);
    let kauf = ("15.06.2020", "10.03.2025");
    // Betraege in Cent: Preis, Anschaffungskosten, AfA, Werbungskosten.
    let mut zwei = grundstueck(kauf, [20_000_000, 15_000_000, 0, 0]);
    zwei.extend(wirtschaftsgut(2, [10_000_000, 5_000_000, 0, 0]));
    let faelle: Vec<SoFall<'_>> = vec![
        (
            "SO_Grundstueck",
            grundstueck(kauf, [20_000_000, 15_000_000, 0, 0]),
            vec![("E0306111", "Testobjekt"), ("E0306201", "15.06.2020"), ("E0306202", "10.03.2025"), ("E0306801", "50000")],
        ),
        // 200.000 - (150.000 - 10.000) - 5.000 = 55.000 EUR: die Formel von `checkESt` (Spike AK0).
        ("SO_Grundstueck_AfA_WK", grundstueck(kauf, [20_000_000, 15_000_000, 1_000_000, 500_000]), vec![("E0306801", "55000")]),
        ("SO_Grundstueck_Verlust", grundstueck(kauf, [4_000_000, 5_000_000, 0, 0]), vec![("E0306801", "-10000")]),
        ("SO_Grundstueck_Gewinn_0", grundstueck(kauf, [5_000_000, 5_000_000, 0, 0]), vec![("E0306801", "0")]),
        (
            "SO_Wirtschaftsgut",
            wirtschaftsgut(1, [10_000_000, 5_000_000, 0, 0]),
            vec![("E0307101", "Testobjekt"), ("E0307201", "15.02.2025"), ("E0307202", "20.08.2025"), ("E0307701", "50000")],
        ),
        // Beim Wirtschaftsgut gibt es kein AfA-Kz: 100.000 - (50.000 - 10.000) = 60.000 EUR stehen als Gewinn.
        ("SO_Wirtschaftsgut_AfA", wirtschaftsgut(1, [10_000_000, 5_000_000, 1_000_000, 0]), vec![("E0307701", "60000")]),
        ("SO_zwei_Verkaeufe", zwei.clone(), vec![("E0306801", "50000"), ("E0307701", "50000")]),
    ];
    for (name, verkaeufe, kz_im_xml) in &faelle {
        let xml = so_xml(verkaeufe, &id).unwrap_or_else(|g| panic!("{name}: kein XML: {g}"));
        for (kz, wert) in kz_im_xml {
            assert!(xml.contains(&format!("<{kz}>{wert}</{kz}>")), "{name}: <{kz}>{wert}</{kz}> steht nicht im XML");
        }
        let u = urteil(&xml, &id);
        eprintln!("{name:<40} {}", u.zeile());
        assert!(u.plausibel(), "{name}: nicht rc=0: {}", u.zeile());
    }

    // Kontrollen: ERiC beanstandet, was fehlt. Aus der Deklaration des Grundstuecks bzw. des Wirtschaftsguts wird je Kontrolle ein Kz
    // entfernt oder der Gewinn verschoben.
    let (g, g_felder) = so_deklaration(&grundstueck(kauf, [20_000_000, 15_000_000, 0, 0]));
    let (w, w_felder) = so_deklaration(&wirtschaftsgut(1, [10_000_000, 5_000_000, 0, 0]));
    let kontrollen: [(&str, Deklaration, &Felder); 5] = [
        ("K_Grundstueck_ohne_Bezeichnung", so_abgeaendert(&g, 1, "E0306111", None), &g_felder),
        ("K_Grundstueck_ohne_Verkaufsdatum", so_abgeaendert(&g, 1, "E0306202", None), &g_felder),
        ("K_Grundstueck_ohne_Gewinn", so_abgeaendert(&g, 1, "E0306801", None), &g_felder),
        ("K_Wirtschaftsgut_ohne_Art", so_abgeaendert(&w, 1, "E0307101", None), &w_felder),
        ("K_Wirtschaftsgut_ohne_Gewinn", so_abgeaendert(&w, 1, "E0307701", None), &w_felder),
    ];
    for (name, d, felder) in kontrollen {
        let u = urteil(&xml_abgabe(&d, felder, &id), &id);
        eprintln!("{name:<40} {}", u.zeile());
        assert!(
            u.rc == RC_PLAUSIBILITAET && !u.meldungen.is_empty(),
            "{name}: ERiC muss rc={RC_PLAUSIBILITAET} mit Meldung geben, gab: {}",
            u.zeile()
        );
    }
}

// ----------------------------------------------------------------------------- Verdrahtung (laeuft in der CI)

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Die Zeilen des Rezepts eines Make-Ziels (die Tab-Zeilen nach `ziel:` bis zur ersten anderen Zeile), Fortsetzungszeilen mit `\`
/// zusammengezogen. Wie in `api/tests/einreichen_eric_echt.rs`.
fn rezept(makefile: &str, ziel: &str) -> Option<String> {
    let kopf = format!("{ziel}:");
    let mut zeilen = makefile.lines().skip_while(|z| !z.starts_with(&kopf));
    zeilen.next()?;
    let text: Vec<&str> = zeilen.take_while(|z| z.starts_with('\t') || z.trim().is_empty()).collect();
    Some(text.join("\n").replace("\\\n", " "))
}

/// `Ok`, wenn Make-Ziel und Quelltext der echten Tests zusammenpassen, sonst der Grund.
fn verdrahtung(makefile: &str, quelle: &str, test_datei: &str, tests: &[&str]) -> Result<(), String> {
    let Some(r) = rezept(makefile, MAKE_ZIEL) else {
        return Err(format!("Make-Ziel `{MAKE_ZIEL}` fehlt"));
    };
    // Ganze Woerter, nicht Teilketten: `--test checkest_blockmatrix2` ist eine andere Testdatei.
    let woerter: Vec<&str> = r.split(|c: char| c.is_whitespace() || c == '(' || c == ')').collect();
    if !woerter.contains(&"cargo") || !woerter.windows(2).any(|w| w == ["--test", test_datei]) {
        return Err(format!("`{MAKE_ZIEL}` ruft `cargo test --test {test_datei}` nicht"));
    }
    if !woerter.contains(&"--ignored") {
        return Err(format!("`{MAKE_ZIEL}` ruft die Tests ohne `--ignored`: die echten Tests liefen nie"));
    }
    if !woerter.contains(&"--exact") {
        return Err(format!("`{MAKE_ZIEL}` ruft die Tests ohne `--exact`: ein Namensteil traefe fremde Tests"));
    }
    let erwartet = format!("test result: ok. {} passed", tests.len());
    if !r.contains(&erwartet) {
        return Err(format!(
            "`{MAKE_ZIEL}` prueft nicht auf `{erwartet}`: ein falscher Name liefe als 0 Tests gruen"
        ));
    }
    for name in tests {
        if !woerter.contains(name) {
            return Err(format!("`{MAKE_ZIEL}` nennt den Test `{name}` nicht (Name weicht ab)"));
        }
        // Der Test steht im Quelltext und traegt `#[ignore` und `#[test]`: sonst liefe er in `cargo test --workspace`
        // (ohne ERiC) oder gar nicht.
        let kopf = format!("fn {name}()");
        let Some(stelle) = quelle.find(&kopf) else {
            return Err(format!("`{kopf}` steht nicht im Quelltext (Name weicht ab)"));
        };
        let davor: Vec<&str> = quelle[..stelle].lines().rev().take(3).collect();
        if !davor.iter().any(|z| z.trim_start().starts_with("#[ignore")) {
            return Err(format!("`{name}` traegt kein `#[ignore`: ohne es braucht jedes Tor ERiC und die ID"));
        }
        if !davor.iter().any(|z| z.trim() == "#[test]") {
            return Err(format!("`{name}` traegt kein `#[test]`"));
        }
    }
    Ok(())
}

#[test]
fn die_blockmatrix_ist_verdrahtet() {
    let makefile = std::fs::read_to_string(wurzel().join("Makefile")).unwrap();
    // Der Quelltext dieser Datei, zur Uebersetzungszeit; der Name der Testdatei ist der Name des Binaers.
    let quelle = include_str!("checkest_blockmatrix.rs");
    if let Err(grund) = verdrahtung(&makefile, quelle, env!("CARGO_CRATE_NAME"), &ECHTE_TESTS) {
        panic!("{grund}");
    }
}

/// Ein Make-Ziel, das `verdrahtung` annimmt.
fn muster_ziel() -> String {
    format!(
        "{MAKE_ZIEL}:\n\t(cd rust && cargo test -p bescheid --test checkest_blockmatrix -- --ignored --nocapture --exact {}) > $$log 2>&1; rc=$$?; \\\n\tgrep -q 'test result: ok. {} passed' $$log\n",
        ECHTE_TESTS.join(" "),
        ECHTE_TESTS.len()
    )
}

/// Ein Quelltext, den `verdrahtung` annimmt. Die Namen stehen hier nie als `fn <Name>()` im Quelltext dieser Datei, sonst faende
/// `verdrahtung` die Nachbildung statt der echten Tests.
fn muster_quelle() -> String {
    let mut quelle = String::new();
    for n in ECHTE_TESTS {
        writeln!(quelle, "#[test]\n#[ignore = \"x\"]\nfn {n}() {{}}").unwrap();
    }
    quelle
}

#[test]
fn der_verdrahtungs_test_wird_bei_jedem_bruch_rot() {
    const DATEI: &str = "checkest_blockmatrix";
    let (ziel, quelle) = (muster_ziel(), muster_quelle());
    assert_eq!(verdrahtung(&ziel, &quelle, DATEI, &ECHTE_TESTS), Ok(()), "Ausgangslage muss gruen sein");
    let rot = |makefile: &str, quelle: &str, datei: &str, soll: &str| {
        let r = verdrahtung(makefile, quelle, datei, &ECHTE_TESTS);
        assert!(r.as_ref().is_err_and(|g| g.contains(soll)), "{soll:?} erwartet, erhalten {r:?}");
    };
    // 1. das Make-Ziel fehlt (auch: ein anderes Ziel mit dem Namen als Vorsilbe zaehlt nicht)
    rot("anderes:\n\ttrue\n", &quelle, DATEI, "fehlt");
    rot(&ziel.replace("blockmatrix-rust:", "blockmatrix-rustx:"), &quelle, DATEI, "fehlt");
    // 2. `--ignored` oder `--exact` fehlt
    rot(&ziel.replace("--ignored", ""), &quelle, DATEI, "--ignored");
    rot(&ziel.replace("--exact", ""), &quelle, DATEI, "--exact");
    // 3. ein Testname im Rezept weicht ab, oder der Quelltext traegt ihn nicht mehr
    rot(&ziel.replace("gegen_echtes_checkest", "gegen_checkest"), &quelle, DATEI, "Name weicht ab");
    rot(&ziel, &quelle.replace("zwoelf_bloecke", "elf_bloecke"), DATEI, "Name weicht ab");
    // 4. `#[ignore]` oder `#[test]` an einem echten Test fehlt
    rot(&ziel, &quelle.replacen("#[ignore = \"x\"]\n", "", 1), DATEI, "#[ignore");
    rot(&ziel, &quelle.replacen("#[test]\n", "", 1), DATEI, "#[test]");
    // dazu: falsche Testdatei (auch als Vorsilbe), kein Nachweis, dass beide Tests liefen
    rot(&ziel, &quelle, "anderer_test", "--test anderer_test");
    rot(&ziel.replace("--test checkest_blockmatrix", "--test checkest_blockmatrix2"), &quelle, DATEI, "--test checkest_blockmatrix");
    let soll = format!("{} passed", ECHTE_TESTS.len());
    rot(&ziel.replace(&soll, &format!("{} passed", ECHTE_TESTS.len() - 1)), &quelle, DATEI, &soll);
    rot(&ziel.replace(&format!("test result: ok. {soll}"), "ok"), &quelle, DATEI, &soll);
}

#[test]
fn die_id_pruefung_laesst_nur_eine_echte_id_durch() {
    // Der Grund zaehlt: "fehlt" und "Platzhalter" sind zwei Fehler, ein Tausch faellt auf.
    let grund = |roh: Option<&str>| pruefe_id(roh).expect_err("diese ID muss abgelehnt werden");
    assert!(grund(None).contains("fehlt"), "fehlende ID");
    assert!(grund(Some("")).contains("fehlt"), "leere ID");
    assert!(grund(Some("  \n")).contains("fehlt"), "ID nur aus Leerraum");
    assert!(grund(Some(PLATZHALTER_ID)).contains("Platzhalter"), "Platzhalter-ID");
    assert!(grund(Some(" 74931\n")).contains("Platzhalter"), "Platzhalter-ID mit Leerraum");
    assert!(grund(Some("00000000000")).contains("Platzhalter"), "ID aus Nullen");
    assert_eq!(pruefe_id(Some(" abc12\n")), Ok("abc12".to_owned()), "eine ID geht, Leerraum faellt weg");
}
