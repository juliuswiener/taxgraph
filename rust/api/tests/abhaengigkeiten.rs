//! Belegt Schritt 1 der `stand`-Naht: `api` erreicht die vier Bausteine, die `GET /stand`
//! braucht. Ohne diesen Test waere eine Zeile in `Cargo.toml` ein Vorsatz, kein Nachweis —
//! eine Abhaengigkeit, die niemand aufruft, kompiliert auch dann, wenn ihr Name falsch ist.
//!
//! Der Test ruft aus JEDEM der vier Bausteine genau die Funktion, die `/stand` braucht
//! (`api.py:447-491`), und prueft ihre Antwort auf Gestalt.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

/// `produkt/bindung/` liegt zwei Ebenen ueber dem Crate.
fn wurzel() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `bindung::lade_registry` (`api._scheibe_bindung` laedt hierueber die Feldtabelle).
#[test]
fn bindung_ist_erreichbar() {
    let reg = bindung::lade_registry(&wurzel().join("produkt/bindung")).expect("Registry");
    let n: usize = reg.dateien.iter().map(|(_, d)| d.bindungen.len()).sum();
    assert!(n > 300, "nur {n} Bindungen geladen — falscher Pfad?");
}

/// `interview::{Graph, Sicht}` (`api.py` uebergibt die Scheiben-Bindung als `Sicht`).
#[test]
fn interview_ist_erreichbar() {
    let reg = bindung::lade_registry(&wurzel().join("produkt/bindung")).expect("Registry");
    let graph = interview::Graph::aus_registry(&reg);
    let sicht = graph.sicht(["ep_arbeitstage", "ep_entfernung_km"]).expect("Sicht");
    assert_eq!(sicht.feld_ids().count(), 2);
}

/// `interview::relevanz` — das ist die Funktion, deren Ergebnis `/stand` als `relevanz` liefert.
#[test]
fn interview_rechnet_relevanz() {
    let reg = bindung::lade_registry(&wurzel().join("produkt/bindung")).expect("Registry");
    let graph = interview::Graph::aus_registry(&reg);
    let store = store::Store::leer(2025, None);
    let rel = interview::relevanz(&store, graph.alle(), &graph);
    assert!(!rel.is_empty(), "relevanz lieferte nichts");
    // Jede Regel traegt einen Status; `ausgeschlossen` ist im leeren Fall keiner.
    assert!(rel.values().all(|r| r.status != interview::Regelstatus::Ausgeschlossen));
}

/// `bescheid::deklaration::{an_gesamt_sperrgrund, Cfg, rentenbeginn_offen_stand}` und
/// `bescheid::zweige::bescheid_fn` — die drei Aufrufe aus `api.stand`.
#[test]
fn bescheid_ist_erreichbar() {
    use bescheid::deklaration::{an_gesamt_sperrgrund, rentenbeginn_offen_stand, Cfg};
    use domain::Scheibe;
    let leer = bescheid::testhilfe::felder(&store::Store::leer(2025, None));
    // Der ep-Guard: `SCHEIBEN['ep']` hat kein `guard`, also laeuft `an_gesamt_sperrgrund`
    // gar nicht erst. Der Aufruf belegt trotzdem, dass die Funktion erreichbar ist.
    let q = bescheid::Instanzquelle { store: None, bindung: None, nur_bestaetigt: true };
    let g = an_gesamt_sperrgrund(&leer, Some(&Cfg::fuer(Scheibe::Ep)), Some(domain::Vz::Vz2025), &q)
        .expect("Guard");
    assert_eq!(g, None);
    assert_eq!(rentenbeginn_offen_stand(&leer, Some(&Cfg::fuer(Scheibe::Ep))), None);

    let umg = bescheid::zweige::Umgebung {
        achsen: &[],
        index: bescheid::testhilfe::index(),
        params: bescheid::testhilfe::params(),
    };
    let f = bescheid::zweige::bescheid_fn(
        "abziehbarer_betrag",
        domain::Vz::Vz2025,
        &umg,
        Some(&leer),
        None,
        false,
        None,
        None,
    );
    assert!(f.is_some(), "kein Accessor fuer abziehbarer_betrag");
}

/// `intervall::intervall` — der Ring, den `api.stand` fuer `intervall` aufruft.
#[test]
fn intervall_ist_erreichbar() {
    use domain::{Cent, Feldtyp};
    use intervall::{intervall, AchsenBindung, Spanne};
    let b = AchsenBindung {
        feld_id: "tage".into(),
        typ: Feldtyp::Int,
        askable: true,
        enum_werte: vec![],
        bereich: Some((0, 10)),
        signatur_slot: Some("tage".into()),
        slot_beitrag: bindung::SlotBeitrag::Exakt,
    };
    let r = intervall(
        &std::collections::BTreeMap::new(),
        &[b],
        |w| Ok::<_, std::convert::Infallible>(Cent::new(
            w.get("tage").and_then(|v| v.int().ok()).unwrap_or(0) * 100,
        )),
        intervall::CAP_DEFAULT,
        None,
    )
    .expect("Ring");
    assert!(matches!(r.intervall.spanne, Spanne::Zahl { .. }));
}
