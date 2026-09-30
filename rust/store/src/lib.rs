//! `store` — der Sachverhalts-Store (`produkt/store/store.py`): append-only Event-Log,
//! Auflagen A/K1/F2/T/F/B, Ableitungen (`_leite_ab`/`_rechne_ab`), Materialisierung/Snapshot,
//! Persistenz, Audit-Log ([`audit`]), PII-sicheres Fehler-Log ([`fehler_log`]).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod ableitung;
mod abweisung;
mod anhaenge_datei;
pub mod audit;
mod canonical;
mod event;
pub mod fehler_log;
mod katalog;
mod nachschlag;
mod persistenz;
mod store;
mod zeit;

pub use abweisung::Abweisung;
pub use canonical::{canonical_json, sha256_hex, EventId, EventIdFehler};
pub use event::{Event, NeuesEvent, Signal};
pub use katalog::Katalog;
pub use nachschlag::{baue_nachschlag, instanz_basis, BindungNachschlag};
pub use persistenz::{lade, speichere, PersistenzFehler};
pub use store::{
    EricBefund, EricBefundEingabe, EricKlasse, Snapshot, SnapshotFehler, SnapshotFeld, Store,
    StoreDatei, Veranlagungsjahr,
};
