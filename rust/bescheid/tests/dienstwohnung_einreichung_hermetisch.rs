//! Eine Akte, die `dhf_keine_pflicht_dienstwohnung` vor der Streichung beantwortet hat (2026-10-06,
//! Abweichung Nr. 21 in `rust/fixtures/README.md`), bleibt einreichbar: `einreichungs_xml` bricht nicht mit
//! `DeklarationUnvollstaendig` ab und liefert dasselbe wie die Akte ohne die alte Antwort.
//!
//! Die Frage wird nicht mehr gestellt, das Feld bleibt mit `askable: false` in der Bindung. Fehlte das Feld,
//! meldete `deklariere` "Feld nicht in der Bindungstabelle" unter `unvollstaendig`, und `einreichungs_xml`
//! wiese die Akte ab (`api::einreichen`: 409 `deklaration_unvollstaendig`). Der Test geht ueber `einreichungs_xml`
//! und nicht ueber `POST /einreichen`: der HTTP-Weg ruft danach `ERiC`.
//!
//! Die Akte ist `rust/fixtures/e2e/gesamt.json` (vollstaendig, mit Stammdaten). Fehlt das ERiC-Schema, endet
//! `einreichungs_xml` am Writer mit `XmlNichtBaubar`, nachdem Guard und Deklaration liefen (wie in
//! `einreichung_e2e.rs`); mit Schema ist das XML Byte fuer Byte gleich.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::deklaration::{einreichungs_xml, EinreichFehler};
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Zustand};
use elster::testhilfe::schemas_da;
use serde_json::json;
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const FELD: &str = "dhf_keine_pflicht_dienstwohnung";

fn akte() -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    Store::aus_datei(store::lade(&pfad).unwrap())
}

fn mit_alter_antwort(mut s: Store) -> Store {
    let leer = HashMap::new();
    s.append_roh(
        &NeuesEventRoh {
            feld_id: FELD.to_owned(),
            wert: json!(true).into(),
            zustand: Zustand::Bestaetigt,
            herkunft: HerkunftVektor::Voll(Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: Some(format!("ok@{FELD}")),
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
    s
}

fn lauf(s: &Store) -> Result<String, EinreichFehler> {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned())).map(|e| e.xml)
}

#[test]
fn eine_alte_antwort_auf_die_dienstwohnung_sperrt_das_einreichen_nicht() {
    let ohne = lauf(&akte());
    // Mit Schema baut die Akte ein XML; ohne endet sie am Writer.
    assert_eq!(ohne.is_ok(), schemas_da(2025), "{ohne:?}");
    let mit = lauf(&mit_alter_antwort(akte()));
    assert!(
        !matches!(mit, Err(EinreichFehler::DeklarationUnvollstaendig(_))),
        "alte Antwort sperrt die Akte: {mit:?}"
    );
    // Gleiche Antwort wie ohne die alte Antwort: dasselbe XML, oder (ohne Schema) derselbe Fehler am Writer.
    match (ohne, mit) {
        (Ok(a), Ok(b)) => assert_eq!(a, b, "das XML aendert sich durch die alte Antwort"),
        (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string()),
        (a, b) => panic!("ohne alte Antwort {a:?}, mit {b:?}"),
    }
}
