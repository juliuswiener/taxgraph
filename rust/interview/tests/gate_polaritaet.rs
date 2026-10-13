//! Gate-Polaritaet der Bindungsdaten (`rust/bindung/daten`), ueber die Relevanz-Funktion des Dienstes
//! (`interview::relevanz`), ohne Python. Gegenstueck zu `tests/test_gate_polaritaet.py`.
//!
//! **Worum es geht.** Ein Ja/Nein-Feld an der Geltungsbedingung einer Regel gilt als Gate: ein bestaetigtes "nein" schliesst die
//! ganze Regel aus und nimmt dem Nutzer alle weiteren Fragen dieser Regel. Das ist fuer echte Voraussetzungen richtig und fuer
//! Angaben, die der Vordruck nur verlangt, falsch. Am 2026-08-16 verlor so der normale Vermieter die ganze Anlage V (155 statt
//! 174 Fragen), ohne dass eine Zahl rot wurde. Die Bindung nimmt solche Felder mit `gate: false` aus dem Gate-Dienst.
//!
//! Vier Behauptungen, jede gegen die echten Daten:
//! 1. `gate: false` und die Liste [`KEIN_GATE`] decken sich in BEIDE Richtungen: ein neues `gate: false` braucht einen
//!    Eintrag mit Begruendung, und ein Eintrag ohne `gate: false` ist veraltet.
//! 2. Beide Antworten auf so ein Feld lassen die Regel stehen, und das Feld wird weiter gefragt (sonst waere der bequemste Weg
//!    durch Test 1, das Feld stillzulegen).
//! 3. Sweep ueber ALLE Ja/Nein-Gates: die Antwort des Normalfalls (`beispielwert`) schliesst keine Regel aus, die noch weitere
//!    fragbare Felder hat. Das haette den Anlage-V-Fund am Entstehungstag gefunden.
//! 4. Die Zusammenfassungen `DHF_BEDINGUNGEN` und `UEBERNACHTUNG_BEDINGUNGEN` im Rechenpfad tragen kein Feld mit `gate: false`:
//!    dieser Test steht in `rust/bescheid/src/deklaration/konstanten.rs`, weil die Tabellen dort nicht sichtbar sind.
//!
//! Jede Untergrenze verhindert, dass ein Test gruen wird, weil seine Menge leer ist.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use bindung::{Bindung, Bindungspunkt, Registry};
use domain::{Achsenwert, Feldtyp, Herkunft, PruefTiefe, Schreiber, Zustand};
use interview::{Graph, Regelstatus};
use serde_json::{json, Value};
use store::{Event, EventId, Store, StoreDatei, Veranlagungsjahr};

/// Jedes Feld mit `gate: false`: `(feld, regel, Grund)`. Der Grund sagt, warum die Antwort die Regel NICHT abschalten darf.
const KEIN_GATE: &[(&str, &str, &str)] = &[
    ("vv_nutzung_ferienwohnung", "p21_vermietung_einkuenfte", "Angabe der Anlage V, keine Voraussetzung der Einkunftsart"),
    ("vv_nutzung_an_angehoerige", "p21_vermietung_einkuenfte", "wie vv_nutzung_ferienwohnung"),
    ("vv_nutzung_kurzfristig", "p21_vermietung_einkuenfte", "wie vv_nutzung_ferienwohnung"),
    ("vv_nebenkosten_nicht_vereinbart", "p21_vermietung_einkuenfte", "wie vv_nutzung_ferienwohnung"),
    ("p35c_bereits_ermaessigung_frueher", "p35c_sanierung_ermaessigung", "Fehler in BEIDE Richtungen: Nein (Normalfall) schloss die Ermaessigung aus, Ja liess den 40.000-EUR-Objektdeckel unangewendet"),
    ("vpf_keine_mahlzeitengestellung", "p9_4a_verpflegungsmehraufwand", "§ 9 Abs. 4a S. 8-9 EStG: Mahlzeiten KUERZEN die Pauschale, sie streichen sie nie"),
    ("dhf_keine_pflicht_dienstwohnung", "p9_1_3_nr5_doppelte_haushaltsfuehrung", "§ 9 Abs. 1 S. 3 Nr. 5 S. 4 Hs. 2: hebt nur die Auslandsgrenze auf, schliesst nichts aus"),
    ("gwg_netto_ohne_vorsteuer", "p6_2_gwg_sofortabzug", "§ 9b EStG: wer die Vorsteuer nicht abziehen darf, zieht brutto ab; Angabe ueber die Eingabe, keine Voraussetzung"),
    ("gwg_ohne_vorsteuerabzug", "p6_2_gwg_sofortabzug", "wie gwg_netto_ohne_vorsteuer"),
    ("uebernachtung_im_inland", "p9_1_3_nr5a_uebernachtung_nach_48", "§ 9 Abs. 1 S. 3 Nr. 5a EStG: der Ort entscheidet nur ueber die HOEHE, nie ueber den Abzug dem Grunde nach"),
    ("kind_betreuung_reine_betreuung", "p10_1_5_kinderbetreuung", "§ 10 Abs. 1 Nr. 5 EStG: Voraussetzung des ABZUGS einer Aufwendung, nicht der Einkunftsart"),
    ("kind_betreuung_rechnung_ueberweisung", "p10_1_5_kinderbetreuung", "wie kind_betreuung_reine_betreuung (S. 4: Rechnung und unbare Zahlung)"),
    ("kind_schulgeld_aufteilung_prozent", "p10_1_9_schulgeld", "Zahlenfeld (Anteil), kein Ja/Nein: ein Wert schliesst die Regel nie aus"),
    ("rentner_alter_55_oder_berufsunfaehig", "p16_4_freibetrag", "§ 16 Abs. 4 EStG: Voraussetzung des FREIBETRAGS, nicht der Einkunftsart"),
    ("rentner_alter_55_oder_berufsunfaehig_partner", "p16_4_freibetrag", "Partner-Spiegel zu rentner_alter_55_oder_berufsunfaehig"),
    ("rentner_freibetrag_erstmalig", "p16_4_freibetrag", "§ 16 Abs. 4 S. 2 EStG: nur einmal zu gewaehren, auch das ist Voraussetzung des Freibetrags"),
    ("rentner_freibetrag_erstmalig_partner", "p16_4_freibetrag", "Partner-Spiegel zu rentner_freibetrag_erstmalig"),
];

/// Felder mit `gate: false`, die der Nutzer bewusst nicht mehr gefragt wird; das Feld bleibt gebunden, damit alte Akten
/// einreichbar bleiben.
const VERBORGEN: &[(&str, &str)] = &[(
    "dhf_keine_pflicht_dienstwohnung",
    "Julius 2026-10-06: die Frage zur Dienstwohnung entfaellt (Vault die-frage-zur-dienstwohnung-wird-verborgen-nicht-geloescht)",
)];

/// Gates, deren "nein" die Regel zu Recht ausschliesst: ein WAHLRECHT, das nicht ausgeuebt wurde.
const ECHTES_OPT_IN: &[(&str, &str)] = &[
    (
        "antrag_ermaessigter_satz",
        "§ 34 Abs. 3 EStG wirkt nur auf Antrag; ohne Antrag gibt es die Regel nicht",
    ),
    (
        "dba_abzug_statt_anrechnung",
        "§ 34c Abs. 2 EStG: Abzug statt Anrechnung der Auslandssteuer nur auf Antrag; ohne Antrag gilt die Anrechnung (Abs. 1)",
    ),
];

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| interview::doctest_registry().expect("Registry laedt"))
}

fn graph() -> &'static Graph<'static> {
    static CELL: OnceLock<Graph<'static>> = OnceLock::new();
    CELL.get_or_init(|| Graph::aus_registry(registry()))
}

fn bindungen() -> BTreeMap<&'static str, &'static Bindung> {
    registry()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .map(|b| (b.feld_id.as_str(), b))
        .collect()
}

/// Ein Store mit genau einer bestaetigten Antwort.
fn store_mit(feld_id: &str, wert: &Value) -> Store {
    let mut e = Event {
        event_id: EventId::aus_bytes([0; 32]),
        ts: "2026-08-16T12:00:00Z".to_owned(),
        feld_id: feld_id.to_owned(),
        wert: wert.clone().into(),
        zustand: Zustand::Bestaetigt,
        herkunft: Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        }
        .into(),
        schreiber: Schreiber::Mensch("laie".to_owned()),
        signal: None,
        ersetzt: None,
    };
    e.event_id = e.berechne_event_id().expect("Testwert ist darstellbar");
    Store::aus_datei(StoreDatei {
        version: 1,
        veranlagungszeitraum: Veranlagungsjahr(2025),
        fall_id: None,
        scheibe: None,
        user_id: None,
        events: vec![e],
        snapshots: Vec::new(),
        vorjahr_referenz: None,
    })
}

/// Status der Regel, nachdem genau diese Antwort bestaetigt ist.
fn status_nach(feld: &str, wert: &Value, regel: &str) -> Regelstatus {
    let rel = interview::relevanz(&store_mit(feld, wert), graph().alle(), graph());
    rel.get(regel)
        .unwrap_or_else(|| panic!("{feld}: die Regel {regel} gibt es in der Relevanz nicht"))
        .status
}

#[test]
fn gate_false_und_benannte_liste_decken_sich() {
    let b = bindungen();
    let echt: BTreeSet<&str> = b.values().filter(|x| x.gate == Some(false)).map(|x| x.feld_id.as_str()).collect();
    let benannt: BTreeSet<&str> = KEIN_GATE.iter().map(|(f, _, _)| *f).collect();
    assert!(echt.len() >= 15, "nur {} Felder mit gate: false: der Test saehe nichts", echt.len());
    let neu: Vec<&&str> = echt.difference(&benannt).collect();
    assert!(
        neu.is_empty(),
        "gate: false ohne Eintrag in KEIN_GATE: {neu:?}. Ein gate: false behauptet, dass die Antwort die Regel NICHT abschaltet; \
         ohne Eintrag mit Grund ist die Behauptung unbezeugt"
    );
    let veraltet: Vec<&&str> = benannt.difference(&echt).collect();
    assert!(
        veraltet.is_empty(),
        "in KEIN_GATE, aber ohne gate: false in den Daten: {veraltet:?}. Eintrag entfernen oder die Ursache klaeren"
    );
    for (feld, regel, grund) in KEIN_GATE {
        assert!(!grund.is_empty(), "{feld}: Grund fehlt");
        assert_eq!(b[feld].quelle.regel_id, *regel, "{feld}: Regel in KEIN_GATE stimmt nicht mit den Daten");
    }
}

#[test]
fn beide_antworten_auf_ein_deklarationsfeld_lassen_die_regel_stehen() {
    let b = bindungen();
    for (feld, regel, _) in KEIN_GATE {
        let antworten: Vec<Value> = if matches!(b[feld].typ, Feldtyp::Bool) {
            vec![json!(true), json!(false)]
        } else {
            vec![b[feld].beispielwert.clone(), json!(0)]
        };
        for wert in &antworten {
            assert_ne!(
                status_nach(feld, wert, regel),
                Regelstatus::Ausgeschlossen,
                "{feld}={wert} schliesst {regel} aus. Das Feld ist Deklaration, keine Voraussetzung: eine ganze Anlage verschwindet \
                 aus dem Dialog, ohne dass eine Zahl rot wird"
            );
        }
    }
}

#[test]
fn ein_deklarationsfeld_wird_weiter_gefragt() {
    let b = bindungen();
    for (feld, _, _) in KEIN_GATE {
        let verborgen = VERBORGEN.iter().find(|(f, _)| f == feld);
        match verborgen {
            Some((_, grund)) => {
                assert!(!b[feld].askable, "{feld} steht in VERBORGEN ({grund}), wird aber gefragt: Eintrag entfernen");
            }
            None => assert!(
                b[feld].askable,
                "{feld} wird nicht mehr gefragt. gate: false darf nicht die bequeme Art werden, ein Feld verschwinden zu lassen: \
                 der Vordruck verlangt die Angabe, und ohne Frage faellt sie erst bei checkESt auf"
            ),
        }
    }
}

/// `(feld, regel, beispielwert, Zahl der UEBRIGEN fragbaren Felder der Regel)` je Ja/Nein-Gate.
fn bool_gates() -> Vec<(&'static str, &'static str, bool, usize)> {
    let alle = bindungen();
    let mut fragbar_je_regel: BTreeMap<&str, usize> = BTreeMap::new();
    for b in alle.values().filter(|b| b.askable) {
        *fragbar_je_regel.entry(b.quelle.regel_id.as_str()).or_default() += 1;
    }
    alle.values()
        .filter(|b| {
            b.askable
                && b.gate != Some(false)
                && matches!(b.quelle.bindungspunkt, Bindungspunkt::Geltungsbedingung(_))
                && matches!(b.typ, Feldtyp::Bool)
        })
        .filter_map(|b| {
            let bw = b.beispielwert.as_bool()?;
            let regel = b.quelle.regel_id.as_str();
            Some((b.feld_id.as_str(), regel, bw, fragbar_je_regel[regel] - 1))
        })
        .collect()
}

#[test]
fn die_antwort_des_normalfalls_schaltet_ihre_regel_nicht_ab() {
    let gates = bool_gates();
    assert!(gates.len() >= 40, "nur {} Ja/Nein-Gates gefunden: der Sweep saehe nichts", gates.len());
    let mut schaden = Vec::new();
    let mut geprueft = 0;
    for (feld, regel, bw, rest) in &gates {
        if *rest == 0 || ECHTES_OPT_IN.iter().any(|(f, _)| f == feld) {
            continue;
        }
        geprueft += 1;
        if status_nach(feld, &json!(bw), regel) == Regelstatus::Ausgeschlossen {
            schaden.push(format!("{feld}={bw} schliesst {regel} aus und nimmt {rest} weitere Fragen mit"));
        }
    }
    assert!(geprueft >= 20, "nur {geprueft} von {} Gates ueberhaupt geprueft: der Sweep saehe nichts", gates.len());
    assert!(
        schaden.is_empty(),
        "Gate-Polaritaet verdreht, der Normalfall schaltet seine eigene Regel ab:\n   - {}\nEntweder ist es Deklaration (dann \
         gate: false in der Bindung und ein Eintrag in KEIN_GATE), ein echtes Wahlrecht (dann ECHTES_OPT_IN mit Grund) oder ein \
         Fehler (dann Fragetext und Polaritaet richtigstellen)",
        schaden.join("\n   - ")
    );
}

#[test]
fn die_ausnahmen_des_sweeps_sind_echte_gates() {
    let gates = bool_gates();
    for (feld, grund) in ECHTES_OPT_IN {
        let g = gates.iter().find(|(f, _, _, _)| f == feld);
        assert!(g.is_some(), "{feld} ist kein Ja/Nein-Gate mehr: Ausnahme entfernen ({grund})");
        let (_, regel, bw, rest) = g.unwrap();
        assert!(*rest > 0, "{feld}: {regel} hat keine weiteren fragbaren Felder mehr: Ausnahme entfernen");
        assert_eq!(
            status_nach(feld, &json!(bw), regel),
            Regelstatus::Ausgeschlossen,
            "{feld} ist als Wahlrecht eingetragen ({grund}), schliesst seine Regel aber mit dem Normalfall nicht aus: Ausnahme entfernen"
        );
    }
}
