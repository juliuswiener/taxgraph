//! Eigenschaften der Bindungsdaten (`rust/bindung/daten`), die weder ein Typ noch der Lader ausdruecken und
//! die bis zum 2026-10-05 nur Python-Tests hielten (Stufe 3 von B voll, Plan `bvoll-waechter` Abschnitt 4, W3).
//! Jeder Test liest die Registry, die der Dienst benutzt (`lade_registry_der_wurzel`), nie einen festen Pfad.
//!
//! Was durchrutscht, wenn ein Test fehlt:
//! - Verweis auf ein Feld, das es nicht gibt: die Bedingung greift nie, die Frage bleibt fuer immer stehen.
//! - Gemischte Summanden: der Slot-Wert stimmt nicht, ohne Fehlermeldung nur im Betrag.
//! - Zwei Felder auf einem Kz: eine Angabe ueberschreibt die andere in der Erklaerung.
//! - Bool-Gate auf einem Sammel-Scope: eine Antwort "Nein" nimmt der ganzen Regel alle Fragen weg.
//! - Partner-Feld hinter einem Ich-Kreuz: wer nur einen behinderten Partner hat, antwortet "nein" und verliert
//!   dessen Pauschbetrag (gemessen im Echtfall: 302 EUR zu viel Steuer).
//!
//! Python-Gegenstuecke: `tests/test_bindungstabelle.py` (e, o, p, Laengengrenze),
//! `test_detailfrage_braucht_existenzfrage.py`, `test_screening_partner.py`, `test_screening_ausgabenseite.py`.
//! Die Python-Tests laufen ueber den Traverser; hier steht die Eigenschaft der Daten, die dahinter wirkt.
//!
//! Jede Untergrenze (`mindestens`) verhindert, dass ein Test gruen wird, weil seine Menge leer ist.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

use bindung::{Bindung, BindungDatei, Bindungspunkt, RegelBedingung, Registry, SlotBeitrag};
use domain::{Feldtyp, MAX_FELD_ID_LAENGE};

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel).expect("Registry laedt")
    })
}

/// `(Dateiname, Datei)` je `bindung_*.yaml`.
fn dateien() -> Vec<(String, &'static BindungDatei)> {
    registry()
        .dateien
        .iter()
        .map(|(pfad, datei)| {
            let name = pfad
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            (name, datei)
        })
        .collect()
}

fn bindungen() -> Vec<&'static Bindung> {
    registry()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .collect()
}

fn felder() -> BTreeMap<&'static str, &'static Bindung> {
    bindungen()
        .into_iter()
        .map(|b| (b.feld_id.as_str(), b))
        .collect()
}

fn regel_bedingungen() -> Vec<&'static RegelBedingung> {
    registry()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.regel_bedingungen.iter())
        .collect()
}

/// Prueft, dass jedes Ziel ein Feld der Registry ist, und dass es genug Verweise gab, damit der Test etwas sah.
fn pruefe_verweise(art: &str, verweise: &[(String, &str)], mindestens: usize) {
    let alle = felder();
    let fehlt: Vec<String> = verweise
        .iter()
        .filter(|(_, ziel)| !alle.contains_key(*ziel))
        .map(|(wer, ziel)| format!("{wer} -> {ziel}"))
        .collect();
    assert!(
        fehlt.is_empty(),
        "{art} zeigt auf Felder, die es nicht gibt: {fehlt:?}"
    );
    assert!(
        verweise.len() >= mindestens,
        "nur {} Verweise der Art {art} gefunden, erwartet mindestens {mindestens}: der Test saehe nichts",
        verweise.len()
    );
}

#[test]
fn feld_bedingung_zeigt_auf_ein_feld_das_es_gibt() {
    let verweise: Vec<(String, &str)> = bindungen()
        .into_iter()
        .filter_map(|b| {
            b.feld_bedingung
                .as_ref()
                .map(|fb| (b.feld_id.clone(), fb.feld.as_str()))
        })
        .collect();
    pruefe_verweise("feld_bedingung", &verweise, 50);
}

/// Eine Ableitung mit einem Tippfehler im Quellfeld laeuft nie; das Ziel bleibt fuer immer leer.
#[test]
fn ableitung_zeigt_auf_felder_die_es_gibt() {
    let mut verweise: Vec<(String, &str)> = Vec::new();
    for b in bindungen() {
        if let Some(a) = &b.ableitung {
            verweise.push((format!("{}.ableitung.aus", b.feld_id), a.aus.as_str()));
            if let Some(und) = &a.und_feld {
                verweise.push((format!("{}.ableitung.und_feld", b.feld_id), und.as_str()));
            }
        }
    }
    pruefe_verweise("ableitung", &verweise, 3);
}

#[test]
fn beweist_zeigt_auf_ein_feld_das_es_gibt() {
    let verweise: Vec<(String, &str)> = bindungen()
        .into_iter()
        .filter_map(|b| {
            b.beweist
                .as_ref()
                .map(|w| (b.feld_id.clone(), w.feld_id.as_str()))
        })
        .collect();
    pruefe_verweise("beweist", &verweise, 1);
}

/// Ein Tippfehler im `feld` einer Regelbedingung ist lautlos: die Bedingung findet kein Ereignis, greift nie,
/// und die Regel bleibt fuer immer relevant.
#[test]
fn regel_bedingung_zeigt_auf_ein_feld_das_es_gibt() {
    let verweise: Vec<(String, &str)> = regel_bedingungen()
        .into_iter()
        .map(|r| (r.regel_id.clone(), r.feld.as_str()))
        .collect();
    pruefe_verweise("regel_bedingung", &verweise, 30);
}

#[test]
fn instanz_gruppe_hat_ihr_zaehlfeld() {
    let verweise: Vec<(String, &str)> = dateien()
        .into_iter()
        .flat_map(|(name, d)| {
            d.instanz_gruppen
                .iter()
                .map(move |g| (format!("{name}: {}", g.gruppe), g.anzahl_feld.as_str()))
        })
        .collect();
    pruefe_verweise("instanz_gruppe", &verweise, 5);
}

type SlotFeld = (&'static str, SlotBeitrag, Feldtyp);

/// Ein Signatur-Slot einer Regel, auf den mehr als ein Feld bindet. Gezaehlt wird je Datei
/// (Scheibe), wie in Python: dieselbe Slot-Bezeichnung in zwei Dateien ist kein gemeinsamer Slot.
struct GeteilterSlot {
    name: String,
    felder: Vec<SlotFeld>,
}

fn geteilte_slots() -> Vec<GeteilterSlot> {
    let mut ergebnis = Vec::new();
    for (datei, d) in dateien() {
        let mut je_slot: BTreeMap<(&str, &str), Vec<SlotFeld>> = BTreeMap::new();
        for b in &d.bindungen {
            if let Bindungspunkt::SignaturSlot(slot) = &b.quelle.bindungspunkt {
                je_slot
                    .entry((b.quelle.regel_id.as_str(), slot.as_str()))
                    .or_default()
                    .push((
                        b.feld_id.as_str(),
                        b.slot_beitrag.unwrap_or(SlotBeitrag::Exakt),
                        b.typ,
                    ));
            }
        }
        for ((regel, slot), felder) in je_slot {
            if felder.len() > 1 {
                ergebnis.push(GeteilterSlot {
                    name: format!("{datei}: {regel}/{slot}"),
                    felder,
                });
            }
        }
    }
    ergebnis
}

/// Summen-Konvention: Bindet mehr als ein Feld auf einen Slot, sind alle `summand`. Ein `exakt` neben einem
/// zweiten Feld waere "das Feld IST der Slot-Wert" und "mehrere Felder addieren sich" zugleich.
#[test]
fn mehrere_felder_auf_einem_slot_sind_alle_summanden() {
    let slots = geteilte_slots();
    assert!(slots.len() >= 3, "nur {} geteilte Slots", slots.len());
    let falsch: Vec<String> = slots
        .iter()
        .filter(|s| {
            s.felder
                .iter()
                .any(|(_, beitrag, _)| *beitrag != SlotBeitrag::Summand)
        })
        .map(|s| format!("{}: {:?}", s.name, s.felder))
        .collect();
    assert!(
        falsch.is_empty(),
        "Slot mit mehreren Feldern, aber nicht alle `summand`: {falsch:?}"
    );
}

#[test]
fn summanden_eines_slots_haben_denselben_zahltyp() {
    let slots = geteilte_slots();
    assert!(slots.len() >= 3, "nur {} geteilte Slots", slots.len());
    let falsch: Vec<String> = slots
        .iter()
        .filter(|s| {
            let erster = s.felder.first().map(|(_, _, t)| *t);
            let gleich = s.felder.iter().all(|(_, _, t)| Some(*t) == erster);
            let zahl = s
                .felder
                .iter()
                .all(|(_, _, t)| matches!(t, Feldtyp::Cent | Feldtyp::Int));
            !(zahl && gleich)
        })
        .map(|s| format!("{}: {:?}", s.name, s.felder))
        .collect();
    assert!(
        falsch.is_empty(),
        "Summanden nicht typ-homogen cent/int: {falsch:?}"
    );
}

/// Zwei Felder auf einem Kz schreiben in dieselbe Stelle der Erklaerung; die zweite Angabe ueberschreibt die erste.
/// Heute teilt kein Feld sein Kz (Person B und Instanzen laufen ueber die Routing-Tabelle, nicht ueber die Bindung).
/// Wer ein Kz bewusst teilt, entscheidet das hier mit einem benannten Eintrag und einem Grund.
#[test]
fn kein_kz_gehoert_zwei_feldern() {
    let mut je_kz: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for b in bindungen() {
        if let Some(kz) = &b.elster_kz {
            je_kz
                .entry(kz.as_str())
                .or_default()
                .push(b.feld_id.as_str());
        }
    }
    assert!(je_kz.len() >= 100, "nur {} Kz gebunden", je_kz.len());
    let doppelt: Vec<String> = je_kz
        .iter()
        .filter(|(_, felder)| felder.len() > 1)
        .map(|(kz, felder)| format!("{kz}: {felder:?}"))
        .collect();
    assert!(doppelt.is_empty(), "ein Kz hat mehrere Felder: {doppelt:?}");
}

/// Die Route nimmt eine Kennung nur bis `MAX_FELD_ID_LAENGE` Zeichen an (`rust/api/src/routen.rs` baut ihr Muster
/// aus derselben Zahl). Eine laengere Kennung ist als Bindung gueltig und ueber die Route nicht erreichbar.
#[test]
fn keine_feld_id_ist_laenger_als_die_route_erlaubt() {
    let alle = bindungen();
    assert!(alle.len() > 300, "nur {} Felder geladen", alle.len());
    let zu_lang: Vec<&str> = alle
        .iter()
        .map(|b| b.feld_id.as_str())
        .filter(|id| id.len() > MAX_FELD_ID_LAENGE)
        .collect();
    assert!(
        zu_lang.is_empty(),
        "feld_id laenger als {MAX_FELD_ID_LAENGE} Zeichen, ueber die Route nicht erreichbar: {zu_lang:?}"
    );
}

/// `p2_festzusetzung_einzel` und `_zusammen` buendeln dutzende sachlich unabhaengige Felder (Bruttolohn,
/// Veranlagungsart, alle Stammdaten) unter einer Regel-Id, weil dort keine Catala-Regel andockt. Ein bestaetigtes
/// `false` an einem fragbaren bool-Gate dieser Regel schliesst die GANZE Regel aus. So verschwanden am 2026-08-12
/// Veranlagung, Lohn und alle Stammdaten aus der Fragenliste, bevor sie je gefragt wurden.
#[test]
fn kein_fragbares_bool_gate_auf_einem_sammel_scope() {
    const SAMMEL_SCOPES: [&str; 2] = ["p2_festzusetzung_einzel", "p2_festzusetzung_zusammen"];
    let alle = bindungen();
    let auf_scope = alle
        .iter()
        .filter(|b| SAMMEL_SCOPES.contains(&b.quelle.regel_id.as_str()))
        .count();
    assert!(
        auf_scope >= 20,
        "nur {auf_scope} Felder auf den Sammel-Scopes"
    );
    let gates: Vec<String> = alle
        .iter()
        .filter(|b| {
            SAMMEL_SCOPES.contains(&b.quelle.regel_id.as_str())
                && matches!(b.quelle.bindungspunkt, Bindungspunkt::Geltungsbedingung(_))
                && b.askable
                && matches!(b.typ, Feldtyp::Bool)
        })
        .map(|b| format!("{} auf {}", b.feld_id, b.quelle.regel_id))
        .collect();
    assert!(
        gates.is_empty(),
        "fragbares bool-Gate auf einem Sammel-Scope (eigene Regel-Id verwenden): {gates:?}"
    );
}

/// `(Ort, Text)` einer festen Jahreszahl, die kein VZ-Bezug ist; der Grund steht daneben. Wer eine neue
/// Ausnahme braucht, traegt sie hier mit Grund ein, statt den Test zu lockern.
const JAHR_AUSNAHMEN: [(&str, &str, &str); 3] = [
    (
        "kind_geburtsdatum",
        "hilfe_kurz",
        "Formatbeispiel TT.MM.JJJJ (z.B. 15.03.2015), kein VZ-Bezug.",
    ),
    (
        "kind_anderer_elternteil_geburtsdatum",
        "hilfe_kurz",
        "Formatbeispiel TT.MM.JJJJ (z.B. 01.01.1985), kein VZ-Bezug.",
    ),
    (
        "vor_voller_abzug",
        "hilfe_kurz",
        "Rechtstatsache (Vorsorgeaufwand seit 2023 zu 100% ansetzbar), kein VZ-Bezug.",
    ),
];

/// Vier Ziffern, die mit `19` oder `20` beginnen (Pythons `(19|20)\d{2}`, ohne Wortgrenze).
fn enthaelt_jahreszahl(text: &str) -> bool {
    let zeichen: Vec<char> = text.chars().collect();
    zeichen.windows(4).any(|w| {
        matches!(w, ['1', '9', a, b] | ['2', '0', a, b] if a.is_ascii_digit() && b.is_ascii_digit())
    })
}

/// Frage- und Hilfetext werden nirgends mit dem Veranlagungsjahr formatiert. Eine feste Jahreszahl ist deshalb in
/// jedem Jahr ausser dem genannten falsch (Fund 2026-08-12: `am_afa_ist_anschaffungsjahr` fragte im VZ 2025 nach 2026).
#[test]
fn keine_feste_jahreszahl_in_frage_und_hilfetext() {
    let alle = bindungen();
    let mut treffer = Vec::new();
    let mut ausnahmen_gesehen = BTreeSet::new();
    for b in &alle {
        let texte = [
            ("fragetext_laie", b.fragetext_laie.as_deref()),
            ("hilfe_kurz", Some(b.hilfe_kurz.as_str())),
        ];
        for (schluessel, text) in texte {
            let Some(text) = text else { continue };
            if !enthaelt_jahreszahl(text) {
                continue;
            }
            if JAHR_AUSNAHMEN
                .iter()
                .any(|(f, k, _)| *f == b.feld_id && *k == schluessel)
            {
                ausnahmen_gesehen.insert((b.feld_id.as_str(), schluessel));
            } else {
                treffer.push(format!("{}.{schluessel}: {text:?}", b.feld_id));
            }
        }
    }
    assert!(
        treffer.is_empty(),
        "feste Jahreszahl ohne Eintrag in JAHR_AUSNAHMEN: {treffer:?}"
    );
    // Jede Ausnahme muss noch treffen; sonst deckt eine tote Ausnahme spaeter einen echten Fund mit zu.
    assert_eq!(
        ausnahmen_gesehen.len(),
        JAHR_AUSNAHMEN.len(),
        "eine Ausnahme in JAHR_AUSNAHMEN trifft nicht mehr (Text geaendert?): gesehen {ausnahmen_gesehen:?}"
    );
}

/// Eine Regelbedingung nimmt dem Nutzer Fragen weg. Das ist eine steuerliche Aussage und braucht dieselbe
/// Fundstelle wie jede andere: ein Paragraf und eine Begruendung von mindestens 80 Zeichen.
#[test]
fn jede_regel_bedingung_nennt_ihre_norm() {
    let alle = regel_bedingungen();
    assert!(alle.len() >= 30, "nur {} Regelbedingungen", alle.len());
    let ohne: Vec<String> = alle
        .iter()
        .filter(|r| !r.grund.contains('§') || r.grund.chars().count() < 80)
        .map(|r| {
            format!(
                "{}: {:?}",
                r.regel_id,
                r.grund.chars().take(60).collect::<String>()
            )
        })
        .collect();
    assert!(ohne.is_empty(), "Regelbedingungen ohne Normbeleg: {ohne:?}");
}

fn screening_flags() -> Vec<&'static Bindung> {
    bindungen()
        .into_iter()
        .filter(|b| b.screening == Some(true))
        .collect()
}

fn ist_partner_feld(b: &Bindung) -> bool {
    b.feld_id.ends_with("_partner")
}

/// Anwendbarkeits-Flags steuern nur, welche Fragen kommen. Mit einem Kz stuende die Antwort im XML.
#[test]
fn screening_flag_traegt_kein_kz() {
    let flags = screening_flags();
    assert!(flags.len() >= 15, "nur {} Screening-Flags", flags.len());
    let mit_kz: Vec<&str> = flags
        .iter()
        .filter(|b| b.elster_kz.is_some())
        .map(|b| b.feld_id.as_str())
        .collect();
    assert!(mit_kz.is_empty(), "Screening-Flags mit Kz: {mit_kz:?}");
}

/// Das Feld benennt die Abwesenheit (`kein_...`), die Frage fragt nach der Anwesenheit; `frage_invertiert` dreht
/// sie um. Passt die Umkehr nicht zum Namen, speichert die Oberflaeche das Gegenteil der Antwort.
#[test]
fn frage_invertiert_passt_zum_namen_des_screening_flags() {
    let flags = screening_flags();
    assert!(flags.len() >= 15, "nur {} Screening-Flags", flags.len());
    let falsch: Vec<&str> = flags
        .iter()
        .filter(|b| {
            b.frage_invertiert
                != (b.feld_id.starts_with("kein_") || b.feld_id.starts_with("keine_"))
        })
        .map(|b| b.feld_id.as_str())
        .collect();
    assert!(
        falsch.is_empty(),
        "frage_invertiert passt nicht zum Feldnamen: {falsch:?}"
    );
}

/// Jedes Partner-Kreuz ist das EINZIGE fragbare Feld seiner Regel. Die Gates einer Regel gelten zusammen und
/// brechen beim ersten bestaetigten `false` ab; lagen zwei Kreuze auf einer Regel, naehme eine bejahte Frage
/// der anderen die Antwort.
#[test]
fn partner_kreuz_ist_das_einzige_fragbare_feld_seiner_regel() {
    let alle = bindungen();
    let kreuze: Vec<&&Bindung> = alle
        .iter()
        .filter(|b| b.screening == Some(true) && ist_partner_feld(b))
        .collect();
    assert!(kreuze.len() >= 3, "nur {} Partner-Kreuze", kreuze.len());
    let geteilt: Vec<String> = kreuze
        .iter()
        .filter_map(|k| {
            let geschwister: Vec<&str> = alle
                .iter()
                .filter(|b| {
                    b.askable && b.quelle.regel_id == k.quelle.regel_id && b.feld_id != k.feld_id
                })
                .map(|b| b.feld_id.as_str())
                .collect();
            (!geschwister.is_empty()).then(|| {
                format!(
                    "{} teilt {} mit {geschwister:?}",
                    k.feld_id, k.quelle.regel_id
                )
            })
        })
        .collect();
    assert!(geteilt.is_empty(), "{geteilt:?}");
}

/// Die Felder, von deren Antwort die Frage nach `b` abhaengt: die eigene `feld_bedingung` und die
/// `regel_bedingungen` der Regel von `b` (der Traverser wertet beide aus).
fn gate_felder(b: &Bindung) -> BTreeSet<String> {
    let mut gates: BTreeSet<String> = regel_bedingungen()
        .into_iter()
        .filter(|r| r.regel_id == b.quelle.regel_id)
        .map(|r| r.feld.clone())
        .collect();
    if let Some(fb) = &b.feld_bedingung {
        gates.insert(fb.feld.clone());
    }
    gates
}

/// Alle Ich-Kreuze (Screening-Flags ohne `_partner`) fragen woertlich in der ersten Person. Keines darf ein
/// Partner-Feld abschalten. Bis 2026-08-28 hingen neun Partner-Felder an einem Ich-Kreuz; gemessen kostete das
/// einem Paar mit behindertem Partner 302 EUR (5.532,00 gegen 5.834,00 EUR Steuer).
#[test]
fn kein_partner_feld_haengt_an_einem_ich_kreuz() {
    let ich: BTreeSet<String> = screening_flags()
        .into_iter()
        .filter(|b| !ist_partner_feld(b))
        .map(|b| b.feld_id.clone())
        .collect();
    assert!(ich.len() >= 10, "nur {} Ich-Kreuze", ich.len());
    let partner: Vec<&Bindung> = bindungen()
        .into_iter()
        .filter(|b| ist_partner_feld(b))
        .collect();
    assert!(partner.len() >= 30, "nur {} Partner-Felder", partner.len());
    let treffer: Vec<String> = partner
        .iter()
        .flat_map(|b| {
            gate_felder(b)
                .into_iter()
                .filter(|g| ich.contains(g))
                .map(|g| format!("{} haengt an {g}", b.feld_id))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        treffer.is_empty(),
        "Partner-Felder hinter einem Kreuz, das nur nach dem Nutzer fragt: {treffer:?}"
    );
}

/// Vorname, Nachname, Geburtsdatum und Konfession des Partners tragen ein Kz und gehen in jede
/// Zusammenveranlagung. Kein Screening-Kreuz darf sie abschalten.
#[test]
fn partner_stammdaten_haengen_an_keinem_screening_kreuz() {
    const STAMMDATEN: [&str; 4] = [
        "stammdaten_vorname_partner",
        "stammdaten_nachname_partner",
        "stammdaten_geburtsdatum_partner",
        "kist_konfession_partner",
    ];
    let alle = felder();
    let flags: BTreeSet<String> = screening_flags()
        .into_iter()
        .map(|b| b.feld_id.clone())
        .collect();
    assert!(flags.len() >= 15, "nur {} Screening-Flags", flags.len());
    for name in STAMMDATEN {
        let b = alle
            .get(name)
            .unwrap_or_else(|| panic!("{name} fehlt in der Registry"));
        let gates: Vec<String> = gate_felder(b)
            .into_iter()
            .filter(|g| flags.contains(g))
            .collect();
        assert!(
            gates.is_empty(),
            "{name} haengt an Screening-Kreuz {gates:?}"
        );
    }
}
