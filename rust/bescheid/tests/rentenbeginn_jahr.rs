//! § 22 Anlage R: ein Rentenbeginn-Jahr <= 0 sperrt mit `rentenbeginn_jahr_ungueltig` (das Kz-Datum
//! wuerde "01.01.0000", `ERiC` lehnt es ab). Der Test braucht weder Python noch `PARITY=1`: die
//! Parity-Suite `bescheid_deklaration_paritaet` haelt dieselben Faelle gegen Python fest, laeuft aber nur
//! mit `PARITY=1` und damit nicht in der CI. Ohne diesen Test bliebe `if beginn <= 0` in
//! `deklaration/sperre/gesamt.rs` (`beginn_grund`) ohne Gegenprobe im Standardlauf.
//!
//! Gemessen am 2026-10-03: die Mutation `if beginn <= 0` → `if false` laesst `cargo test -p bescheid`
//! ohne diese Tests gruen (152 passed, 0 failed); mit ihnen wird `jahr_null_und_minus_eins_sperren` rot
//! (14 von 14 Faellen), `jahr_eins_sperrt_nicht_wegen_des_jahres` bleibt gruen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::deklaration::{an_gesamt_sperrgrund, Cfg};
use bescheid::testhilfe::{felder, index, store};
use bescheid::{BindungIndex, Instanzquelle};
use domain::{Scheibe, Sperrgrund, Vz};
use serde_json::{json, Value};
use std::sync::OnceLock;

type Ereignisse<'a> = Vec<(&'a str, Value, bool)>;

/// Die Bindung der Scheibe `rentner_gesamt`: nur ihre Feld-Ids, wie `api._scheibe_bindung`. Der volle Index
/// kennt mehr Flags und liesse `flag_widersprueche` auf Person B ansprechen (`FlagKonsistenzOffen`).
fn scheiben_index() -> &'static BindungIndex<'static> {
    static I: OnceLock<BindungIndex<'static>> = OnceLock::new();
    I.get_or_init(|| {
        let ids = Cfg::fuer(Scheibe::RentnerGesamt)
            .felder(|_| Vec::new())
            .unwrap();
        index()
            .iter()
            .filter(|(k, _)| ids.contains(k))
            .map(|(k, b)| (k.clone(), *b))
            .collect()
    })
}

/// Der Sperrgrund der Scheibe `rentner_gesamt` fuer VZ 2025 auf einem Store aus bestaetigten Feldern.
fn grund(events: &[(&str, Value, bool)]) -> Option<Sperrgrund> {
    let st = store(events);
    let f = felder(&st);
    let q = Instanzquelle {
        store: Some(&st),
        bindung: Some(scheiben_index()),
        nur_bestaetigt: false,
    };
    let cfg = Cfg::fuer(Scheibe::RentnerGesamt);
    an_gesamt_sperrgrund(&f, Some(&cfg), Some(Vz::Vz2025), &q).unwrap()
}

/// Rente der Person A (Basisinstanz): Art, Beginn-Jahr, optional der Freibetrag.
fn person_a(art: &str, beginn: i64, freibetrag: Option<i64>) -> Ereignisse<'_> {
    let mut e = vec![
        ("rentner_renten_art", json!(art), true),
        ("rentner_renten_beginn_jahr", json!(beginn), true),
    ];
    if let Some(fb) = freibetrag {
        e.push(("rentner_rentenfreibetrag", json!(fb), true));
    }
    e
}

/// Zweite Rente der Person A (Instanz `__2`, Mehrfach-Rente): die vier Kernfelder bestaetigt.
fn person_a_instanz_2(art: &str, beginn: i64) -> Ereignisse<'_> {
    vec![
        ("kein_sonstige", json!(false), true),
        ("rentner_renten_art__2", json!(art), true),
        ("rentner_jahresrente__2", json!(900_000), true),
        ("rentner_renten_beginn_jahr__2", json!(beginn), true),
        ("rentner_alter_bei_rentenbeginn__2", json!(65), true),
    ]
}

/// Rente der Person B (Zusammenveranlagung): die vier Kernfelder bestaetigt.
fn person_b(art: &str, beginn: i64) -> Ereignisse<'_> {
    vec![
        ("veranlagung", json!("zusammen"), true),
        ("rentner_renten_art_partner", json!(art), true),
        ("rentner_jahresrente_partner", json!(1_200_000), true),
        ("rentner_renten_beginn_jahr_partner", json!(beginn), true),
        ("rentner_alter_bei_rentenbeginn_partner", json!(65), true),
    ]
}

/// Jahr 0 und -1, Person A (Basis und Instanz `__2`) und Person B, aa mit und ohne Freibetrag, beide
/// Leibrenten: jedes Mal `rentenbeginn_jahr_ungueltig`. Die aa-Zeile ohne Freibetrag zeigt die
/// Reihenfolge: der Grund steht VOR der Freibetrag-Bedingung (sonst kaeme `rentenfreibetrag_fixierung_offen`).
#[test]
fn jahr_null_und_minus_eins_sperren() {
    let faelle: Vec<(&str, Ereignisse<'_>)> = vec![
        (
            "A aa ohne Freibetrag, Jahr 0",
            person_a("gesetzliche_rente", 0, None),
        ),
        (
            "A aa mit Freibetrag, Jahr 0",
            person_a("gesetzliche_rente", 0, Some(600_000)),
        ),
        (
            "A aa ohne Freibetrag, Jahr -1",
            person_a("gesetzliche_rente", -1, None),
        ),
        (
            "A aa mit Freibetrag, Jahr -1",
            person_a("gesetzliche_rente", -1, Some(600_000)),
        ),
        (
            "A private Leibrente, Jahr 0",
            person_a("private_leibrente", 0, None),
        ),
        (
            "A private Leibrente, Jahr -1",
            person_a("private_leibrente", -1, None),
        ),
        (
            "A sonstige Leibrente, Jahr 0",
            person_a("sonstige_leibrente", 0, None),
        ),
        (
            "A sonstige Leibrente, Jahr -1",
            person_a("sonstige_leibrente", -1, None),
        ),
        (
            "A Instanz 2 private Leibrente, Jahr 0",
            person_a_instanz_2("private_leibrente", 0),
        ),
        (
            "A Instanz 2 private Leibrente, Jahr -1",
            person_a_instanz_2("private_leibrente", -1),
        ),
        ("B aa, Jahr 0", person_b("gesetzliche_rente", 0)),
        ("B aa, Jahr -1", person_b("gesetzliche_rente", -1)),
        (
            "B private Leibrente, Jahr 0",
            person_b("private_leibrente", 0),
        ),
        (
            "B sonstige Leibrente, Jahr -1",
            person_b("sonstige_leibrente", -1),
        ),
    ];
    // Alle Faelle durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|(name, events)| {
            let g = grund(events);
            (g != Some(Sperrgrund::RentenbeginnJahrUngueltig)).then(|| format!("{name}: {g:?}"))
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "kein rentenbeginn_jahr_ungueltig bei {} von {} Faellen: {abweichend:#?}",
        abweichend.len(),
        faelle.len()
    );
}

/// Kontrolle: Jahr 1 ist ein gueltiges Jahr. Ohne sie bewiese jede Sperre oben nichts (ein Guard, der
/// alles sperrt, bestuende den Test). aa mit Freibetrag und die Leibrenten sperren gar nicht; aa
/// ohne Freibetrag sperrt, aber mit dem anderen Grund (Fixierung), nicht mit dem Jahr.
#[test]
fn jahr_eins_sperrt_nicht_wegen_des_jahres() {
    assert_eq!(
        grund(&person_a("gesetzliche_rente", 1, Some(600_000))),
        None,
        "A aa mit Freibetrag"
    );
    assert_eq!(
        grund(&person_a("private_leibrente", 1, None)),
        None,
        "A private Leibrente"
    );
    assert_eq!(
        grund(&person_a("sonstige_leibrente", 1, None)),
        None,
        "A sonstige Leibrente"
    );
    assert_eq!(
        grund(&person_b("private_leibrente", 1)),
        None,
        "B private Leibrente"
    );
    assert_eq!(
        grund(&person_a("gesetzliche_rente", 1, None)),
        Some(Sperrgrund::RentenfreibetragFixierungOffen),
        "A aa ohne Freibetrag: die Fixierung, nicht das Jahr"
    );
}
