//! `engine` — typisierte Rust-Fassade ueber die `catala-sys`-Wrapper: ein Modul je
//! Paragraphen-Gruppe, mit `Cent`/`Euro`-typisierten Ein-/Ausgaben statt roher `i64`.
//!
//! **Teil A/Teil B-Grenze** (Schritt 4a): jede Funktion hier nimmt bereits FERTIG berechnete
//! Eingaben entgegen, die 1:1 dem `_in`-Struct des jeweiligen Catala-Scopes entsprechen -- NICHT
//! rohe Sachverhalt-Dicts wie in `produkt/engine/runner.py`. Die Python-Vorstufen, die diese
//! Eingaben aus rohen Feldwerten zusammensetzen (z. B. `_sonderausgaben_final`,
//! `_vorsorge_abzug`, `_kindergeld` fuer `catala_gesamt*`), sind Teil B und noch nicht portiert;
//! sie werden spaeter durch REINES HINZUFUEGEN weiterer Module ergaenzt, ohne die hier
//! definierten Signaturen zu aendern. Deshalb tragen die Eingabe-Structs hier auch KEIN
//! `PARITAET`-Kommentar zu Python-`.get(k, 0)`-Fail-Open-Defaults: diese Defaults entstehen erst
//! beim Zusammenbau der rohen Sachverhalt-Dicts (Teil B), nicht hier. Was hier gilt: kein
//! Eingabe-Struct leitet `Default` ab -- jedes Feld muss der Aufrufer explizit setzen.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

pub mod agb;
pub mod altersentlastungsbetrag;
pub mod arbeitszimmer;
pub mod berufsausbildung;
pub mod betriebsfreibetrag;
pub mod dezimal;
pub mod entfernungspauschale;
pub mod entlastungsbetrag;
pub mod euer;
pub mod familienleistungsausgleich;
pub mod fuenftelregelung;
pub mod gwg;
pub mod kirchensteuer;
pub mod mitunternehmer;
pub mod sachverhalt;
pub mod spenden;
pub mod tarif;
pub mod verbilligte_vermietung;
pub mod verlustvortrag;
pub mod vorsorgeaufwendungen;
pub mod zumutbare_belastung;
pub mod zugriff;

pub use catala_sys::CatalaFehler;
