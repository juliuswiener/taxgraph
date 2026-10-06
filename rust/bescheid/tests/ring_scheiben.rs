//! Ring x Scheibe: was der Ring rechnet, muss die Scheibe auch zeigen. Ersatz fuer vier Python-Tests
//! (Loeschplan V4, Stapel 1b, Zeilen Z2, Z21, Z22, Z23):
//!
//! - `tests/test_abgabescheibe_gate.py` (Naht 1: Abzugs-Kz ohne Kuerzungs-Kz; Naht 2: Scheibe ohne
//!   Stammdaten),
//! - `tests/test_ring_gruppen_vollstaendigkeit.py` (jede Ring-Gruppe x Scheibe),
//! - `tests/test_ring_betragsfelder_vollstaendig.py` (`RING_BETRAGSFELDER` gegen den Quelltext),
//! - `tests/test_ring_zweig_erreichbarkeit.py` (jede Ring-Quantitaet laesst sich aufloesen).
//!
//! GELD. Die Naht ist dieselbe wie bei den zwei Funden, die Geld gekostet haben (Verpflegungskuerzung auf
//! `an_gesamt`, Gewerbesteuer auf `rentner_gesamt`): `mit_ring_werten` rechnet ein Feld aus, die Scheibe fuehrt
//! es nicht, `scheibe_bindung` laesst es fallen, und die Erklaerung zeigt eine Zahl ohne ihre Kuerzung.
//! Die Steuer faellt zu hoch oder zu niedrig aus, ohne dass ein Test rot wird.
//!
//! Anders als in Python:
//!
//! - Die Ring-Regeln stehen mit Wert in einer Tabelle ([`regeln`]). Jede Regel laeuft durch das echte
//!   `mit_ring_werten`: sie schreibt genau ihre Ergebnisfelder, und nimmt man einen Ausloeser weg, schreibt sie
//!   sie nicht mehr. Damit ist die Ausloeser-Liste geprueft, nicht nur behauptet (Pythons Gate sah "einen
//!   elften Bau" nicht und vertraute seiner Liste).
//! - Die Tabelle ist gegen den Quelltext abgeschlossen: die nicht fragbaren Felder, die `ring_werte.rs` nennt,
//!   sind genau die Ergebnisfelder der Tabelle. Ein neues Ergebnisfeld im Ring ohne Zeile hier ist rot.
//! - Die Abgabescheiben ergeben sich aus `einreichungs_xml` selbst, nicht aus einer Kopie von
//!   `STAMMDATEN_FELDER`.
//! - `RING_BETRAGSFELDER` wird bei jedem Lauf aus dem Quelltext der Ring-Module abgeleitet (Rust-Fassung der
//!   Python-Ableitung, s. [`abgeleitet`]), und die je Scheibe geschnittenen Listen (`ring_kandidaten`, bisher nur
//!   von Parity gehalten) werden gegen die Definition gemessen.
//!
//! Nicht portiert: die Ausnahmeliste `RING_LUECKEN_AUSNAHMEN` (war leer, "darf nur schrumpfen"). Hier gibt es
//! keine: jede Luecke ist rot.
//!
//! Nicht gesehen (Grenzen): ob ein Ergebnisfeld den richtigen WERT traegt (das tun die Tests je Block in
//! `ring_werte.rs` und `offene_defekte.rs`); ob der Ausloeser einer Regel auf einer Scheibe fragbar ist
//! (`scheiben_tabellen_konsistenz.rs`, Kegel-Tests); ob das Ergebnisfeld ein `elster_kz` traegt (Abdeckung in
//! `elster/src/abdeckung.rs`). Die Ausloeser-Regel ist wie in Python "irgendein Ausloeser in der Scheibe", also
//! streng: sie meldet auch eine Scheibe, die nur einen von zwei noetigen Ausloesern fuehrt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use bescheid::deklaration::{
    einreichungs_xml, mit_ring_werten, scheibe_bindung, vorlaeufige_ring_betraege, Cfg,
    EinreichFehler,
};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Quantitaet;
use bindung::{Bindung, Registry};
use domain::{Feldtyp, PyWert, Scheibe, Vz};
use serde_json::{json, Value};
use store::Store;

/// Alle fuenf Scheiben. Das `match` in [`jede_scheibe_ist_erfasst`] bricht den Bau, wenn eine dazukommt.
const SCHEIBEN: [Scheibe; 5] = [
    Scheibe::Ep,
    Scheibe::NVorGwg,
    Scheibe::AnGesamt,
    Scheibe::Gesamt,
    Scheibe::RentnerGesamt,
];

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel).expect("Registry (Rust-Besitz) laedt")
    })
}

fn bindungen() -> BTreeMap<&'static str, &'static Bindung> {
    registry()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .map(|b| (b.feld_id.as_str(), b))
        .collect()
}

/// Die Feld-Ids aus der YAML, die `n_vor_gwg` als Feldliste nennt (`Cfg::felder_datei`).
fn datei_felder(datei: &str) -> Vec<String> {
    registry()
        .dateien
        .iter()
        .find(|(p, _)| p.file_name().is_some_and(|n| n == datei))
        .unwrap_or_else(|| panic!("{datei} fehlt in der Registry"))
        .1
        .bindungen
        .iter()
        .map(|b| b.feld_id.clone())
        .collect()
}

/// `SCHEIBEN[<name>]["felder"]`, auch fuer `n_vor_gwg` (aus der YAML).
fn scheiben_felder(s: Scheibe) -> BTreeSet<String> {
    Cfg::fuer(s)
        .felder(datei_felder)
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .collect()
}

fn scheiben_kegel(s: Scheibe) -> BTreeSet<String> {
    Cfg::fuer(s)
        .kegel(datei_felder)
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .collect()
}

// ------------------------------------------------------------------ Naht 2: Abgabe-Scheiben

/// `Ok`, wenn `einreichen` die Scheibe annimmt; sonst `(Scheibe, fehlende Stammdatenfelder)` der 409.
/// Die Akte ist leer: der Lauf haelt vor jedem Rechnen an, es zaehlt nur die Scheibenpruefung.
fn abgabe_pruefung(s: Scheibe) -> Result<(), (Scheibe, Vec<&'static str>)> {
    let datei = serde_json::from_value(
        json!({"version": 1, "veranlagungszeitraum": 2025, "scheibe": s.als_str(), "events": []}),
    )
    .unwrap();
    let st = Store::aus_datei(datei);
    match einreichungs_xml(&st, index(), params(), "BY", Some("74931".to_owned())) {
        Err(EinreichFehler::ScheibeNichtAbgabefaehig {
            scheibe,
            fehlende_stammdatenfelder,
        }) => Err((scheibe, fehlende_stammdatenfelder)),
        _ => Ok(()),
    }
}

/// Die Scheiben, auf denen eine Erklaerung entstehen kann: `einreichen` weist sie nicht wegen der Stammdaten ab.
fn abgabescheiben() -> Vec<Scheibe> {
    SCHEIBEN
        .into_iter()
        .filter(|s| abgabe_pruefung(*s).is_ok())
        .collect()
}

/// Naht 2 (`test_einreichen_lehnt_scheibe_ohne_stammdaten_ab`, `test_abgabefaehige_scheibe_wird_nicht_...
/// _gesperrt`): eine Teilrechnung laeuft nicht in die Abgabe und sagt, welche Felder fehlen; die zwei
/// Gesamt-Scheiben sperrt die Pruefung nicht. Die Gegenprobe steht in derselben Zeile: ein Check, der alles
/// ablehnt, liesse die Liste leer und den Vergleich rot.
#[test]
fn nur_die_zwei_gesamtscheiben_tragen_eine_erklaerung() {
    assert_eq!(
        abgabescheiben(),
        [Scheibe::Gesamt, Scheibe::RentnerGesamt],
        "die Abgabescheiben haben sich geaendert: die Paarregel unten gilt nur fuer diese"
    );
    for s in [Scheibe::Ep, Scheibe::NVorGwg, Scheibe::AnGesamt] {
        let Err((scheibe, fehlen)) = abgabe_pruefung(s) else {
            panic!("{s}: die Teilrechnung laeuft in die Abgabe");
        };
        assert_eq!(scheibe, s, "die 409 nennt eine andere Scheibe");
        assert!(
            !fehlen.is_empty(),
            "{s}: 409 ohne Angabe, welche Felder fehlen"
        );
    }
}

// ------------------------------------------------------- Naht 1: Abzugs-Kz ohne Kuerzungs-Kz

/// Abzugs-Kz -> das Kz, das den Abzug wieder mindert. Fehlt der Wert rechts, waehrend links gebunden ist,
/// deklariert die Erklaerung einen ungeminderten Abzug. Auf `gesamt` ging das einmal so aus: der Ring kuerzte
/// 8.400 Cent, die Deklaration zeigte nur die vollen Tage (zu hohe Erklaerung, Haftung beim Nutzer).
const KUERZUNGS_PAARE: [(&[&str], &str); 1] = [(
    // § 9 Abs. 4a S. 8 EStG: Verpflegungspauschale gekuerzt um gestellte Mahlzeiten.
    &["E0205409", "E0205302", "E0205201"],
    "E0205508",
)];

/// Alle `elster_kz` der Felder, die die Scheibe fuehrt (die Kz der Bindung nach `scheibe_bindung`).
fn scheiben_kz(s: Scheibe) -> BTreeSet<String> {
    let alle = bindungen();
    scheiben_felder(s)
        .iter()
        .filter_map(|f| alle.get(f.as_str()))
        .filter_map(|b| b.elster_kz.as_ref().map(|k| k.as_str().to_owned()))
        .collect()
}

/// Die Paare, bei denen mindestens ein Abzugs-Kz gebunden ist, das Kuerzungs-Kz aber nicht:
/// `(gebundene Abzugs-Kz, fehlendes Kuerzungs-Kz)`.
fn paare_ohne_kuerzung(kz: &BTreeSet<String>) -> Vec<(Vec<&'static str>, &'static str)> {
    KUERZUNGS_PAARE
        .iter()
        .filter_map(|(abzuege, kuerzung)| {
            let gebunden: Vec<&'static str> = abzuege
                .iter()
                .copied()
                .filter(|k| kz.contains(*k))
                .collect();
            (!gebunden.is_empty() && !kz.contains(*kuerzung)).then_some((gebunden, *kuerzung))
        })
        .collect()
}

#[test]
fn jede_abgabescheibe_fuehrt_zu_jedem_abzug_seine_kuerzung() {
    let scheiben = abgabescheiben();
    assert!(
        !scheiben.is_empty(),
        "keine Abgabescheibe: Testvoraussetzung kaputt"
    );
    for s in &scheiben {
        let kz = scheiben_kz(*s);
        let verletzt = paare_ohne_kuerzung(&kz);
        assert!(
            verletzt.is_empty(),
            "Scheibe {s} bindet {verletzt:?}: der Ring kuerzt, die Erklaerung verschweigt es -> sie faellt ZU \
             HOCH aus (§ 9 Abs. 4a S. 8 EStG, Haftung beim Nutzer)"
        );
    }
    // Die Regel greift ueberhaupt: auf `gesamt` ist ein Abzugs-Kz gebunden (sonst prueft sie nichts).
    assert!(
        scheiben_kz(Scheibe::Gesamt).contains("E0205409"),
        "gesamt bindet E0205409 nicht mehr: die Paarregel waere leer wahr"
    );
}

/// Die Regel an erfundenen Mengen: ein Abzug ohne Kuerzung ist ein Verstoss, mit ihr nicht, ohne Abzug auch nicht.
#[test]
fn die_paarregel_findet_einen_abzug_ohne_kuerzung() {
    let menge = |k: &[&str]| -> BTreeSet<String> { k.iter().map(|s| (*s).to_owned()).collect() };
    assert_eq!(
        paare_ohne_kuerzung(&menge(&["E0205302", "E0100401"])),
        [(vec!["E0205302"], "E0205508")]
    );
    assert!(paare_ohne_kuerzung(&menge(&["E0205409", "E0205508"])).is_empty());
    assert!(paare_ohne_kuerzung(&menge(&["E0100401"])).is_empty());
    assert!(paare_ohne_kuerzung(&BTreeSet::new()).is_empty());
}

/// Der berechnete Wert kommt an, nicht nur die Bindung: Ring rechnet die Kuerzung (10 Tage, 5 Fruehstuecke,
/// 5 Mittagessen = 84 EUR), und sie steht als `E0205508` in der Deklaration der Scheibe. Eine Bindung ohne
/// Wert waere eine stille Null. Nur Scheiben, die die Tage fragen: `rentner_gesamt` fuehrt keines der Felder
/// (und keine Kuerzung), dort kann der Ring nicht ausloesen, und `deklariere` fand keine Bindung.
#[test]
fn die_kuerzung_kommt_in_der_deklaration_jeder_abgabescheibe_an() {
    let mit_tagen: Vec<Scheibe> = abgabescheiben()
        .into_iter()
        .filter(|s| scheiben_felder(*s).contains("tage_24h"))
        .collect();
    assert!(
        mit_tagen.contains(&Scheibe::Gesamt),
        "gesamt fragt die Verpflegungstage nicht mehr"
    );
    for s in mit_tagen {
        let st = store(&[
            ("tage_24h", json!(10), true),
            ("vpf_fruehstuecke_gestellt_anzahl", json!(5), true),
            ("vpf_mittagessen_gestellt_anzahl", json!(5), true),
        ]);
        let mut f = felder(&st);
        mit_ring_werten(&mut f, Some(Vz::Vz2025), params()).unwrap();
        let ring = f
            .get("p9_4a_kuerzung_nach_entgelt")
            .unwrap_or_else(|| panic!("{s}: der Ring liefert die Kuerzung nicht (Vorbedingung)"));
        assert_eq!(ring.wert, PyWert::Ganz(8_400), "{s}: Kuerzung in Cent");
        let liste: Vec<String> = scheiben_felder(s).into_iter().collect();
        let sb = scheibe_bindung(&liste, index()).unwrap();
        let d = elster::deklariere(&f, &sb, 2025, None).unwrap();
        assert_eq!(
            d.deklaration.get("E0205409"),
            Some(&json!(10)),
            "{s}: die vollen Tage fehlen"
        );
        let kuerzung = d.deklaration.get("E0205508").and_then(Value::as_f64);
        assert!(
            kuerzung.is_some_and(|k| k > 0.0),
            "{s}: der Ring kuerzt 8.400 Cent, E0205508 ist {:?}: genau die stille Differenz, die die Erklaerung \
             zu hoch macht",
            d.deklaration.get("E0205508")
        );
    }
}

// ------------------------------------------------------------------------- Ring-Regeln

/// Eine Ring-Regel: aus den `ausloeser` (alle, bestaetigt) und den `begleit`-Feldern schreibt
/// `mit_ring_werten` genau die `ergebnis`-Felder.
struct Regel {
    gruppe: &'static str,
    ausloeser: Vec<(&'static str, Value)>,
    begleit: Vec<(&'static str, Value)>,
    ergebnis: &'static [&'static str],
}

fn regel(
    gruppe: &'static str,
    ausloeser: &[(&'static str, Value)],
    begleit: &[(&'static str, Value)],
    ergebnis: &'static [&'static str],
) -> Regel {
    Regel {
        gruppe,
        ausloeser: ausloeser.to_vec(),
        begleit: begleit.to_vec(),
        ergebnis,
    }
}

/// Die elf Gruppen aus `mit_ring_werten` (`ring_werte.rs`), je Ausloeser eine Regel. Python fuehrte zehn
/// (ohne `p34`); die Rust-Tabelle fuehrt dazu `kap_*_partner` (Verlust) und die Ausloeser einzeln.
/// Betraege in Cent.
fn regeln() -> Vec<Regel> {
    let mut r = Vec::new();
    // (1) § 9 Abs. 4a: Kuerzung nach Entgelt. Ausloeser sind die Tage, die Mahlzeiten sind Begleiter.
    for tage in ["tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig"] {
        r.push(regel(
            "verpflegung",
            &[(tage, json!(10))],
            &[
                ("vpf_fruehstuecke_gestellt_anzahl", json!(5)),
                ("vpf_mittagessen_gestellt_anzahl", json!(5)),
            ],
            &["p9_4a_kuerzung_nach_entgelt"],
        ));
    }
    // (2)+(3) Anlage KAP: Antrag Guenstigerpruefung + genutzter Sparer-Pauschbetrag, zusammen oder keins.
    for t in [
        "kap_kapitalertraege",
        "kap_gewinn_aktien",
        "kap_verlust_aktien",
        "kap_gewinn_sonstige",
        "kap_verlust_sonstige",
    ] {
        r.push(regel(
            "kap_antrag",
            &[(t, json!(1_000_000))],
            &[],
            &[
                "kap_antrag_guenstigerpruefung",
                "kap_sparer_pauschbetrag_genutzt",
            ],
        ));
    }
    for t in [
        "kap_kapitalertraege_partner",
        "kap_gewinn_aktien_partner",
        "kap_verlust_aktien_partner",
        "kap_gewinn_sonstige_partner",
        "kap_verlust_sonstige_partner",
    ] {
        r.push(regel(
            "kap_antrag_partner",
            &[(t, json!(1_000_000))],
            &[("veranlagung", json!("zusammen"))],
            &[
                "kap_antrag_guenstigerpruefung",
                "kap_sparer_pauschbetrag_genutzt",
            ],
        ));
    }
    // (4) § 35a: Summenzeile aus der Summe der Einzel-Instanzen.
    let summen: [(&str, &'static [&'static str]); 3] = [
        ("hh_minijob_betrag", &["hh_minijob_aufwendungen"]),
        ("hh_dienstleistung_betrag", &["hh_dienstleistungen"]),
        ("hh_handwerker_betrag", &["hh_handwerker_arbeitskosten"]),
    ];
    for (t, summe) in summen {
        r.push(regel(
            "haushalt_35a_summe",
            &[(t, json!(100_000))],
            &[],
            summe,
        ));
    }
    // (5) § 35c: die Umkehr-Antwort E0240902 (Ausloeser ist das Gate-Feld, nicht die Summe).
    r.push(regel(
        "p35c_umkehr",
        &[("p35c_keine_doppelfoerderung", json!(true))],
        &[],
        &["p35c_foerderung_in_anspruch"],
    ));
    // (6) § 35c-Einzelzeile: derselbe Betrag in der Zeile der Massnahmenart.
    r.push(regel(
        "p35c_einzelzeile",
        &[("p35c_sanierungsaufwendungen", json!(100_000))],
        &[],
        &["p35c_massnahme_einzelbetrag"],
    ));
    // (7) Anlage V: Summenzeilen der Mieteinnahmen.
    r.push(regel(
        "vv_summen",
        &[("vv_einnahmen", json!(1_200_000))],
        &[],
        &[
            "vv_mieteinnahmen_summe",
            "vv_einnahmen_summe_gesamt",
            "vv_summe_werbungskosten",
            "vv_ueberschuss",
            "vv_ueberschuss_person_a",
        ],
    ));
    // § 35 / § 16 Abs. 1 GewStG: zu zahlende Gewerbesteuer = Messbetrag x Hebesatz, beide noetig.
    r.push(regel(
        "gewst_zu_zahlen",
        &[
            ("gewst_messbetrag", json!(100_000)),
            ("gewst_hebesatz", json!(400)),
        ],
        &[],
        &["gewst_zu_zahlen"],
    ));
    r.push(regel(
        "gewst_zu_zahlen_partner",
        &[
            ("gewst_messbetrag_partner", json!(100_000)),
            ("gewst_hebesatz_partner", json!(400)),
        ],
        &[],
        &["gewst_zu_zahlen_partner"],
    ));
    // § 22 Nr. 3: Einzelposten + Werbungskosten aus Einnahmen und Einkuenften, beide noetig.
    r.push(regel(
        "p22_nr3",
        &[
            ("p22_nr3_einnahmen", json!(500_000)),
            ("p22_nr3_einkuenfte", json!(300_000)),
        ],
        &[],
        &["p22_nr3_einnahmen_einzelbetrag", "p22_nr3_werbungskosten"],
    ));
    // § 10 Abs. 1 Nr. 7: Einzelzeile zur Berufsausbildungs-Summe.
    r.push(regel(
        "berufsausbildung_einzelzeile",
        &[("berufsausbildung_aufwendungen", json!(100_000))],
        &[],
        &["berufsausbildung_einzelbetrag"],
    ));
    // (8) § 34 Abs. 3: die Antragszeile, wenn der Chooser Abs. 3 rechnet. Antrag und Gewinn sind die zwei
    // Felder, die der Nutzer beantwortet; die uebrigen sind Berechtigung (wie `p34_antrag_tests`).
    r.push(regel(
        "p34_antrag",
        &[
            ("antrag_ermaessigter_satz", json!(true)),
            ("rentner_veraeusserungsgewinn", json!(50_000_000)),
        ],
        &[
            ("rentner_veraeusserungs_betriebsart", json!("gewerbe")),
            ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
            ("rentner_freibetrag_erstmalig", json!(true)),
            ("geburtsjahr", json!(1955)),
            ("dauernd_berufsunfaehig", json!(false)),
            ("ermaessigung_einmal_genutzt", json!(false)),
        ],
        &["p34_abs3_antragsbetrag"],
    ));
    r
}

/// Die Felder, die `mit_ring_werten` neu schreibt, wenn nur `eingabe` bestaetigt ist.
fn geschrieben(eingabe: &[(&'static str, Value)]) -> BTreeSet<String> {
    let events: Vec<(&str, Value, bool)> =
        eingabe.iter().map(|(f, w)| (*f, w.clone(), true)).collect();
    let mut f = felder(&store(&events));
    let vorher: BTreeSet<String> = f.keys().cloned().collect();
    mit_ring_werten(&mut f, Some(Vz::Vz2025), params()).unwrap();
    f.keys().filter(|k| !vorher.contains(*k)).cloned().collect()
}

fn mit(r: &Regel, ohne: Option<&str>) -> Vec<(&'static str, Value)> {
    r.ausloeser
        .iter()
        .filter(|(f, _)| Some(*f) != ohne)
        .chain(r.begleit.iter())
        .cloned()
        .collect()
}

/// Jede Regel schreibt genau ihre Ergebnisfelder, und ohne einen ihrer Ausloeser schreibt sie sie nicht.
/// Das haelt die Ausloeser-Liste ehrlich: ein Ausloeser, den der Ring nicht liest, macht die zweite Haelfte rot;
/// ein Ergebnisfeld, das die Regel nicht nennt, die erste.
#[test]
fn jede_regel_schreibt_genau_ihre_ergebnisfelder_und_braucht_jeden_ausloeser() {
    for r in regeln() {
        let soll: BTreeSet<String> = r.ergebnis.iter().map(|s| (*s).to_owned()).collect();
        let ist = geschrieben(&mit(&r, None));
        assert_eq!(
            ist, soll,
            "{} / {:?}: geschrieben ist nicht das Ergebnis der Regel",
            r.gruppe, r.ausloeser
        );
        for (fehlt, _) in &r.ausloeser {
            let ohne = geschrieben(&mit(&r, Some(fehlt)));
            assert!(
                !soll.is_subset(&ohne),
                "{} / {:?}: auch ohne {fehlt} schreibt der Ring {ohne:?}: der Ausloeser wird nicht gelesen",
                r.gruppe,
                r.ausloeser
            );
        }
    }
}

/// Die Tabelle ist abgeschlossen: die nicht fragbaren Felder, die `ring_werte.rs` im Produktionstext nennt,
/// sind genau die Ergebnisfelder der Regeln. Ein neues Ergebnis im Ring ohne Zeile in [`regeln`] ist rot;
/// ein Ergebnis in der Tabelle, das der Ring nicht mehr nennt, auch.
#[test]
fn die_ergebnisfelder_der_tabelle_sind_die_nicht_fragbaren_literale_des_rings() {
    let quelle = fs::read_to_string(src("deklaration/ring_werte.rs")).unwrap();
    let alle = bindungen();
    let im_ring: BTreeSet<String> = literale(&quelle)
        .into_iter()
        .filter(|l| alle.get(l.as_str()).is_some_and(|b| !b.askable))
        .collect();
    let in_tabelle: BTreeSet<String> = regeln()
        .iter()
        .flat_map(|r| r.ergebnis.iter().map(|s| (*s).to_owned()))
        .collect();
    assert_eq!(
        im_ring, in_tabelle,
        "ring_werte.rs nennt nicht fragbare Felder, die keine Regel hat (links) oder umgekehrt (rechts)"
    );
    assert_eq!(in_tabelle.len(), 19, "Stand 2026-10-06: 19 Ergebnisfelder");
    for r in regeln() {
        for f in r
            .ausloeser
            .iter()
            .chain(&r.begleit)
            .map(|(f, _)| *f)
            .chain(r.ergebnis.iter().copied())
        {
            assert!(
                alle.contains_key(f),
                "{}: {f} steht nicht in der Registry (Tippfehler?)",
                r.gruppe
            );
        }
        for f in r.ergebnis {
            assert!(
                !alle[f].askable,
                "{}: {f} ist fragbar, ein Ring-Ergebnis ist es nie",
                r.gruppe
            );
        }
    }
}

/// Fuehrt die Scheibe einen Ausloeser der Regel, muss sie jedes Ergebnisfeld fuehren. Sonst rechnet der Ring,
/// `scheibe_bindung` laesst das Ergebnis fallen, und das XML zeigt es nicht. Je (Scheibe, Regel) ein Eintrag in
/// der Meldung, damit eine kaputte Zeile keine grune verdeckt.
#[test]
fn jede_scheibe_fuehrt_zu_jedem_ausloeser_seine_ergebnisfelder() {
    let mut luecken = Vec::new();
    let mut geprueft = 0_usize;
    for s in SCHEIBEN {
        let felder = scheiben_felder(s);
        for r in regeln() {
            if !r.ausloeser.iter().any(|(f, _)| felder.contains(*f)) {
                continue;
            }
            geprueft += 1;
            let fehlend: Vec<&str> = r
                .ergebnis
                .iter()
                .copied()
                .filter(|e| !felder.contains(*e))
                .collect();
            if !fehlend.is_empty() {
                luecken.push(format!(
                    "{s} / {} {:?}: fehlt {fehlend:?}",
                    r.gruppe,
                    r.ausloeser.iter().map(|(f, _)| *f).collect::<Vec<_>>()
                ));
            }
        }
    }
    assert!(
        luecken.is_empty(),
        "der Ring rechnet, die Scheibe fuehrt das Ergebnis nicht (Bescheid zeigt es nicht): {luecken:#?}"
    );
    // Boden: ohne ihn waere ein Lauf ohne Paare gruen. Gemessen 2026-10-06 auf 155fbc3a: 50 Paare.
    assert!(
        geprueft >= 45,
        "nur {geprueft} Paare (Scheibe, Regel) geprueft"
    );
}

// ------------------------------------------------------------- RING_BETRAGSFELDER

/// `rust/bescheid/src/<rel>`.
fn src(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)
}

/// Die Zeichenketten im Produktionstext einer Quelldatei: ohne Kommentare, ohne alles ab dem ersten
/// `#[cfg(test)]` (in diesen Dateien steht Testcode am Ende), ohne Zeichenliterale.
///
/// ponytail: keine Raw-Strings (die Ring-Module haben keine; `panic!` statt stiller Blindheit) und
/// `#[cfg(test)]` nur als Schnitt. Upgrade: `syn`, wenn jemand in die Ring-Module Raw-Strings schreibt.
fn literale(quelle: &str) -> BTreeSet<String> {
    let text = quelle.split("#[cfg(test)]").next().unwrap_or("");
    let z: Vec<char> = text.chars().collect();
    let mut aus = BTreeSet::new();
    let mut i = 0;
    while i < z.len() {
        match z[i] {
            '/' if z.get(i + 1) == Some(&'/') => {
                while i < z.len() && z[i] != '\n' {
                    i += 1;
                }
            }
            '/' if z.get(i + 1) == Some(&'*') => {
                let mut tiefe = 0_usize;
                while i < z.len() {
                    if z[i] == '/' && z.get(i + 1) == Some(&'*') {
                        tiefe += 1;
                        i += 2;
                    } else if z[i] == '*' && z.get(i + 1) == Some(&'/') {
                        tiefe -= 1;
                        i += 2;
                        if tiefe == 0 {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
            }
            '"' => {
                assert!(
                    !roh_beginn(&z, i),
                    "Raw-String im Quelltext: der Scanner kann ihn nicht lesen"
                );
                let mut s = String::new();
                i += 1;
                while z[i] != '"' {
                    if z[i] == '\\' {
                        i += 1;
                    }
                    s.push(z[i]);
                    i += 1;
                }
                i += 1;
                aus.insert(s);
            }
            '\'' => {
                // Zeichenliteral ('x', '\n', '\'') oder Lebensdauer ('a).
                if z.get(i + 1) == Some(&'\\') {
                    i += 3;
                    while z[i] != '\'' {
                        i += 1;
                    }
                    i += 1;
                } else if z.get(i + 2) == Some(&'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    aus
}

/// `"` an Stelle `i` beginnt einen Raw-String: davor `r` (nicht Teil eines Namens) oder `#`.
fn roh_beginn(z: &[char], i: usize) -> bool {
    let Some(vor) = i.checked_sub(1).map(|j| z[j]) else {
        return false;
    };
    let eigener_name = |j: usize| z.get(j).is_some_and(|c| c.is_alphanumeric() || *c == '_');
    vor == '#' || (vor == 'r' && !i.checked_sub(2).is_some_and(eigener_name))
}

fn ist_bezeichner(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Die Ring-Module (Pythons `bescheid_zweige`, `bescheid_einkuenfte`, `bescheid_abzuege`): `zweige.rs` samt
/// `zweige/*.rs`, `einkuenfte.rs`, `abzuege.rs`.
fn ring_quellen() -> Vec<PathBuf> {
    let mut v = vec![src("zweige.rs")];
    let mut zweige: Vec<PathBuf> = fs::read_dir(src("zweige"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    zweige.sort();
    v.extend(zweige);
    v.push(src("einkuenfte.rs"));
    v.push(src("abzuege.rs"));
    v
}

/// Die Felder, die der Ring lesen kann, bei jedem Lauf aus dem Quelltext abgeleitet (Pythons
/// `_abgeleitet`): ein Feld einer Guard-Scheibe (`guard`), flach (keine Instanz-Gruppe), Typ Cent oder Int, das
/// als Zeichenkette in einem Ring-Modul steht.
///
/// Person B: Ein Ring-Modul, das den Suffix `"_partner"` als Zeichenkette fuehrt, bildet daraus `<feld>_partner`
/// (`fid("gewinnanteil")`, `s = if partner { "_partner" } ...`). Python fand diese Felder ueber seine
/// `PARTNER`-Namen aus `api_constants`; hier gilt der Suffix je Datei fuer jedes Literal derselben Datei.
///
/// ponytail: je Datei, nicht je Aufruf. `ausgaben.rs` fuehrt `p36_lohnsteuer`, aber keinen Suffix, daher kein
/// `p36_lohnsteuer_partner` (gemessen: mit dem Suffix ueber alle Dateien wuerde dieses eine Feld zu viel).
/// Upgrade: Aufrufstellen auswerten statt Dateien.
fn abgeleitet() -> BTreeSet<String> {
    let mut erreichbar = BTreeSet::new();
    for pfad in ring_quellen() {
        let lit: BTreeSet<String> = literale(&fs::read_to_string(&pfad).unwrap())
            .into_iter()
            .filter(|l| l == "_partner" || ist_bezeichner(l))
            .collect();
        if lit.contains("_partner") {
            erreichbar.extend(lit.iter().map(|l| format!("{l}_partner")));
        }
        erreichbar.extend(lit);
    }
    let alle = bindungen();
    SCHEIBEN
        .into_iter()
        .filter(|s| Cfg::fuer(*s).guard())
        .flat_map(scheiben_felder)
        .filter(|f| erreichbar.contains(f))
        .filter(|f| {
            alle.get(f.as_str()).is_some_and(|b| {
                matches!(b.typ, Feldtyp::Cent | Feldtyp::Int) && b.instanz_gruppe.is_none()
            })
        })
        .collect()
}

/// Beide Richtungen wie in Python: fehlt ein Feld in der Konstante, faellt ein vorlaeufiger Betrag dort still aus
/// der Zahl (Klasse C); steht eines zu viel darin, sperrt die Sperre Faelle, die der Ring nie liest.
#[test]
fn ring_betragsfelder_ist_die_abgeleitete_menge() {
    let konstante: BTreeSet<String> = konsistenz::RING_BETRAGSFELDER
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let abg = abgeleitet();
    let fehlt: Vec<&String> = abg.difference(&konstante).collect();
    let zuviel: Vec<&String> = konstante.difference(&abg).collect();
    assert!(
        fehlt.is_empty(),
        "der Ring liest {fehlt:?}, RING_BETRAGSFELDER (konsistenz/src/preflight.rs) kennt sie nicht: ein vorlaeufiger \
         Wert dort faellt still aus der Zahl"
    );
    assert!(
        zuviel.is_empty(),
        "RING_BETRAGSFELDER nennt {zuviel:?}, der Ring erreicht sie nicht: die Sperre trifft Faelle, die nie betroffen waren"
    );
}

/// Blindheitswaechter: eine kaputte Ableitung waere in beiden Vergleichen gruen (zwei leere Mengen sind gleich).
#[test]
fn die_abgeleitete_menge_ist_nicht_leer_und_traegt_die_anlaesse() {
    let abg = abgeleitet();
    assert!(
        abg.len() > 50,
        "nur {} Felder abgeleitet: die Extraktion ist kaputt",
        abg.len()
    );
    for anlass in [
        "rentner_veraeusserungsgewinn",
        "einkuenfte_gewinn",
        "gewinnanteil_partner",
    ] {
        assert!(
            abg.contains(anlass),
            "{anlass} fehlt in der abgeleiteten Menge"
        );
        assert!(
            konsistenz::RING_BETRAGSFELDER.contains(&anlass),
            "{anlass} fehlt in RING_BETRAGSFELDER"
        );
    }
}

/// Der Scanner an erfundenem Quelltext: Kommentare, Doku, Zeichenliterale, Escapes und Testcode zaehlen nicht,
/// echte Zeichenketten schon.
#[test]
fn der_scanner_liest_wie_der_compiler() {
    let quelle = r#"
//! "doku_kopf"
/// "doku_fn"
fn f() {
    let a = "echt_eins"; // "kommentar"
    /* "block" /* "verschachtelt" */ "block_zwei" */
    let b = ['"', '\'', 'x'];
    let c = "mit \" escape";
    let d: &'static str = "echt_zwei";
}
#[cfg(test)]
mod tests { fn t() { let _ = "nur_test"; } }
"#;
    let l = literale(quelle);
    let soll: BTreeSet<String> = ["echt_eins", "echt_zwei", "mit \" escape"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert_eq!(l, soll);
    assert!(ist_bezeichner("kap_gewinn_aktien") && ist_bezeichner("_partner"));
    assert!(
        !ist_bezeichner("E0205508") && !ist_bezeichner("Mit Leerzeichen") && !ist_bezeichner("")
    );
}

/// `vorlaeufige_ring_betraege` (und damit die je Scheibe geschnittenen Listen `RING_AN_GESAMT`, `RING_GESAMT`,
/// `RING_RENTNER_GESAMT` in `konstanten.rs`) trifft genau: Ring-Betragsfeld, in den Feldern der Scheibe, nicht im
/// Kegel, Cent oder Int. Bisher hielt diese Schnittmenge nur der Parity-Vergleich mit Python.
#[test]
fn die_ring_kandidaten_je_scheibe_sind_der_schnitt_aus_der_definition() {
    let alle = bindungen();
    let vorlaeufig: Vec<(&str, Value, bool)> = konsistenz::RING_BETRAGSFELDER
        .iter()
        .map(|f| (*f, json!(1), false))
        .collect();
    let f = felder(&store(&vorlaeufig));
    let mut gesamt = 0_usize;
    for s in SCHEIBEN {
        let ist: BTreeSet<String> = vorlaeufige_ring_betraege(&f, &Cfg::fuer(s), index())
            .into_iter()
            .map(str::to_owned)
            .collect();
        let (felder, kegel) = (scheiben_felder(s), scheiben_kegel(s));
        let soll: BTreeSet<String> = konsistenz::RING_BETRAGSFELDER
            .iter()
            .filter(|f| felder.contains(**f) && !kegel.contains(**f))
            .filter(|f| {
                alle.get(**f)
                    .is_some_and(|b| matches!(b.typ, Feldtyp::Cent | Feldtyp::Int))
            })
            .map(|f| (*f).to_owned())
            .collect();
        assert_eq!(
            ist, soll,
            "{s}: Ring-Kandidaten weichen von der Definition ab"
        );
        gesamt += soll.len();
    }
    // Boden: eine leere Schnittmenge waere in jeder Scheibe gleich. Gemessen 2026-10-06: 204 (28 + 99 + 77).
    assert!(
        gesamt > 190,
        "nur {gesamt} Ring-Kandidaten ueber alle Scheiben"
    );
}

// --------------------------------------------------------------- Quantitaeten

/// Jede Quantitaet, die eine Scheibe als Gesamt-Ring oder Teil-Ring nennt, loest sich auf (Pythons `_bescheid_fn`
/// gaebe sonst "ehrlich" `None`: keine Zahl). Die Namen stehen als Zeichenkette in `Cfg::fuer`; die Enum kennt
/// vier.
#[test]
fn jede_ring_quantitaet_loest_ueber_aus_name_auf() {
    let mut geprueft = 0_usize;
    let mut teil = 0_usize;
    for s in SCHEIBEN {
        let cfg = Cfg::fuer(s);
        if let Some(q) = cfg.gesamt_ring() {
            assert!(
                Quantitaet::aus_name(q).is_some(),
                "{s}: gesamt_ring {q:?} kennt `Quantitaet` nicht"
            );
            geprueft += 1;
        }
        for (familie, q, felder) in cfg.teil_ringe() {
            assert!(
                Quantitaet::aus_name(q).is_some(),
                "{s}: Teil-Ring {familie} nennt {q:?}, `Quantitaet` kennt ihn nicht"
            );
            assert!(!felder.is_empty(), "{s}: Teil-Ring {familie} ohne Felder");
            geprueft += 1;
            teil += 1;
        }
    }
    // Vier Gesamt-Ringe (ep, an_gesamt, gesamt, rentner_gesamt) und der eine Teil-Ring von `n_vor_gwg`.
    assert_eq!(
        (geprueft, teil),
        (5, 1),
        "Zahl der Ring-Quantitaeten hat sich geaendert"
    );
    // Die Gegenprobe: ein erfundener Name loest nicht auf.
    assert_eq!(Quantitaet::aus_name("festzusetzende_est_haushalt"), None);
}

/// Der Nenner selbst: eine sechste Scheibe bricht diesen Bau (kein `_`-Arm).
#[test]
fn jede_scheibe_ist_erfasst() {
    for s in SCHEIBEN {
        match s {
            Scheibe::Ep
            | Scheibe::NVorGwg
            | Scheibe::AnGesamt
            | Scheibe::Gesamt
            | Scheibe::RentnerGesamt => {}
        }
    }
}
