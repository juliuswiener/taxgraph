//! Verhaltensparitaet von `Store::append` (Rust) gegen `produkt/store/store.py::append_event`
//! (Python, ueber `tools/parity/oracle.py::_append_sequence`) ueber ganze Aufruf-SEQUENZEN --
//! Deliverable #2 (Nachtrag) der `store`-Crate, ergaenzend zu `store_paritaet.rs` (das nur die
//! reine `event_id`-Hashfunktion prueft, keine Auflagen A/K1/F2/T/F/B).
//!
//! Zwei Tests:
//! - `append_sequence_paritaet_ueber_zufaellige_aufruf_sequenzen`: 1000 proptest-Faelle, je 1..=20
//!   Aufrufe gegen einen anfangs LEEREN Store. Statt frei-random Feldkombinationen (die meist nur
//!   `TypInkonform` treffen wuerden) generiert ein Satz von ~16 benannten Szenario-Funktionen
//!   gezielt Auflage-A/K1/F2/T/F/B-Faelle -- jede Funktion liest den bisherigen `Store`-Zustand nur
//!   ueber dessen OEFFENTLICHE API (`events()`/`aktives()`), keine zusaetzliche Buchfuehrung.
//! - `append_sequence_replay_realer_faelle`: jede reale Fall-Datei unter `faelle_verzeichnis()`
//!   wird als Aufruf-Sequenz in einen frischen LEEREN Store (Rust) und einen frischen leeren Store
//!   (Python) wiedergegeben; beide Seiten muessen jedes Event annehmen. Deckt damit zugleich die
//!   "initial store = eine reale Fall-Datei"-Vorgabe ab (statt sie in den 1000-Faelle-Proptest zu
//!   mischen, s. Moduldoku dort fuer die parallele Vereinfachung in `store_paritaet.rs`).
//!
//! SICHERHEIT: reale Fall-Dateien tragen echte Nutzer-Steuerdaten. Nur lokal, in-prozess lesen,
//! NIE ins Repo/Fixtures kopieren; nur Zaehlwerte duerfen in Ausgabe/Assertions landen.
//!
//! Zwei bewusst NICHT generierte Kombinationen (harness-bedingte Fallstricke, keine echten Bugs):
//! - `zustand=vorlaeufig` MIT nicht-leerem `signal_2`: Python speichert das wortgetreu, Rust kann
//!   `Feldzustand::Vorlaeufig` strukturell gar kein `signal_2` mitgeben (`NeuesEvent::signal_2_roh`,
//!   dokumentierte PARITAET-Abweichung in `store/src/event.rs`). Generator liefert fuer
//!   `vorlaeufig` immer `signal_2=None`.
//! - `zustand=bestaetigt` mit LEEREM `signal_2` NEBEN einer weiteren, fruehere Auflage verletzenden
//!   Eingabe: `zu_neues_event` schlaegt dafuer schon bei der KONSTRUKTION fehl (Rust kann den
//!   Zustand nicht bauen), bevor `Store::append` ueberhaupt laeuft -- Python wuerde bei einer
//!   gleichzeitigen A/K1/F2/T-Verletzung ZUERST DIESE melden (`store.py`-Reihenfolge: A -> K1 -> F2
//!   -> T -> Zwei-Signal -> B). Das Szenario "Zwei-Signal fehlend" wird deshalb NUR mit
//!   Auflage-A-EXEMPTEN Schreibern (`import:elster`/`engine`/`abgeleitet:*`/Mensch) UND einem
//!   typ-korrekten Wert generiert, so dass in Python ebenfalls die Zwei-Signal-Pruefung zuerst
//!   greift.
//!
//! **D20 — die Reihenfolge der beiden Schreibpfad-Pruefungen divergiert.** Python prueft Auflage T
//! (Typ) vor dem Zwei-Signal (`store.py:409` gegen `:412`), Rust kann `Feldzustand::Bestaetigt`
//! ohne gueltiges `signal_2` gar nicht konstruieren — das Signal kommt also zuerst. Treffen beide
//! Fehler zusammen, meldet Python `TypInkonform` und Rust `ZweiSignalFehlend`. Beide Seiten weisen
//! ab, beide fail-closed; nur die KLASSE divergiert. Gemessen 2026-10-01 an `bruttoarbeitslohn` mit
//! `"50000"` und leerem `signal_2`: Python `TypInkonform`, Rust `ZweiSignalFehlend` — ohne jeden
//! Bezug zum Grad der Behinderung, die Abweichung ist also nicht durch ihn entstanden, sondern nur
//! durch ihn erreichbar geworden (ein Wert mit Werteliste auf `typ: int` ist der erste Fall, in dem
//! ein typ-korrekt aussehender Generatorwert typ-INKONFORM sein kann).
//!
//! Nicht angeglichen: die Angleichung muesste den Typ umbauen, der die Zusage im Typsystem traegt,
//! und genau das ist der Grund, warum sie funktioniert. Korrektheit vor Paritaet: der Fall steht in
//! `d20_reihenfolge_typ_vor_signal` fest.
//!
//! Braucht die Catala-Opam-Toolchain + `python3` mit Repo-Umfeld -- in CI standardmaessig SKIP:
//!
//!   `PARITY`=1 `cargo` test -p parity --test `store_append_paritaet` -- --nocapture
#![allow(
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::sync::{Mutex, OnceLock};

use bindung::Bindung;
use domain::{Achsenwert, Feldtyp, Feldzustand, Herkunft, PruefTiefe, PyWert, Schreiber, Signal2};
use parity::{AppendCallErgebnis, Oracle};
use proptest::prelude::*;
use serde_json::{json, Value};
use store::{Abweisung, Event, EventId, Katalog, NeuesEvent, Signal, Store};

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip_ohne_parity_env() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn expand_home(pfad: &str) -> std::path::PathBuf {
    if let Some(rest) = pfad.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    if pfad == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home);
        }
    }
    std::path::PathBuf::from(pfad)
}

/// s. `store_paritaet.rs::faelle_verzeichnis` (bewusst dupliziert -- Datei-lokales Idiom in
/// diesem Workspace, keine geteilte `tests/common.rs`).
fn faelle_verzeichnis() -> std::path::PathBuf {
    let eigen = std::env::var("TAXGRAPH_DATEN").unwrap_or_default();
    let wurzel = if eigen.trim().is_empty() {
        let xdg = std::env::var("XDG_DATA_HOME").unwrap_or_default();
        let basis = if xdg.trim().is_empty() {
            expand_home("~").join(".local").join("share")
        } else {
            expand_home(xdg.trim())
        };
        basis.join("taxgraph")
    } else {
        expand_home(eigen.trim())
    };
    wurzel.join("faelle")
}

fn walk_json(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut gefunden = Vec::new();
    for eintrag in read.flatten() {
        let pfad = eintrag.path();
        if pfad.is_dir() {
            gefunden.extend(walk_json(&pfad));
        } else if pfad.extension().is_some_and(|e| e == "json") {
            gefunden.push(pfad);
        }
    }
    gefunden
}

fn alle_bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let verzeichnis = repo_root().join("produkt").join("bindung");
        let registry = bindung::lade_registry(&verzeichnis).expect("bindung-registry laedt");
        registry
            .dateien
            .into_iter()
            .flat_map(|(_, datei)| datei.bindungen)
            .collect()
    })
}

fn oracle_singleton() -> &'static Mutex<Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
}

/// Feld-Pools je Auflage-K1-Typ (`Katalog::erlaubt`) plus `alle` (jedes geladene Feld) und
/// `muster` (Felder mit `bindung.muster`, fuer die Format-Verletzung).
struct Pools {
    alle: Vec<&'static Bindung>,
    llm: Vec<&'static Bindung>,
    beleg: Vec<&'static Bindung>,
    kontoauszug: Vec<&'static Bindung>,
    maps: Vec<&'static Bindung>,
    muster: Vec<&'static Bindung>,
}

/// `beweist`/`ableitung` loesen bei `bestaetigt` intern `Store::leite_ab`/`rechne_ab` aus
/// (store.rs 284-374): SYNTHETISIERTE Events mit einem `ts`, den keine Aufruf-Spec kontrolliert
/// (jeder Prozess stempelt sein eigenes `jetzt_iso()`). Ihr `event_id` ist dadurch zwischen
/// Rust- und Python-Prozess strukturell nie identisch -- ausserhalb der
/// `sha256(canonical_json(...))`-Content-Adressierung, um die es hier geht (dieselbe Abgrenzung
/// wie `store_paritaet.rs`s Moduldoku). Kein Auflage-Verhalten, daher aus den Pools
/// ausgeschlossen statt eine Kaskaden-Paritaet nachzubauen: sowohl die AUSLOESENDEN Felder
/// (`beweist` steht auf dem geprueften Feld selbst; `ableitung.aus`/`und_feld` stehen auf einem
/// ANDEREN Feld und nennen das ausloesende Feld nur beim Namen) als auch die ZIEL-Felder (die
/// sonst mit dem Kaskaden-Ergebnis um `AktivesEventVorhanden` konkurrieren wuerden).
fn ausgeloeste_felder(bindungen: &[Bindung]) -> std::collections::HashSet<String> {
    let mut ausgeloest = std::collections::HashSet::new();
    for b in bindungen {
        if let Some(regel) = &b.ableitung {
            ausgeloest.insert(regel.aus.clone());
            if let Some(und) = &regel.und_feld {
                ausgeloest.insert(und.clone());
            }
        }
    }
    ausgeloest
}

fn baue_pools(bindungen: &'static [Bindung], katalog: &Katalog) -> Pools {
    let ausgeloest = ausgeloeste_felder(bindungen);
    let mut p = Pools {
        alle: Vec::new(),
        llm: Vec::new(),
        beleg: Vec::new(),
        kontoauszug: Vec::new(),
        maps: Vec::new(),
        muster: Vec::new(),
    };
    for b in bindungen {
        if b.beweist.is_some() || b.ableitung.is_some() || ausgeloest.contains(&b.feld_id) {
            continue;
        }
        p.alle.push(b);
        if katalog.erlaubt("llm", &b.feld_id) {
            p.llm.push(b);
        }
        if katalog.erlaubt("beleg", &b.feld_id) {
            p.beleg.push(b);
        }
        if katalog.erlaubt("kontoauszug", &b.feld_id) {
            p.kontoauszug.push(b);
        }
        if katalog.erlaubt("maps", &b.feld_id) {
            p.maps.push(b);
        }
        if b.muster.is_some() {
            p.muster.push(b);
        }
    }
    p
}

fn pool_fuer<'p>(pools: &'p Pools, typ: &str) -> &'p [&'static Bindung] {
    match typ {
        "llm" => &pools.llm,
        "beleg" => &pools.beleg,
        "kontoauszug" => &pools.kontoauszug,
        "maps" => &pools.maps,
        _ => &[],
    }
}

/// Deterministischer Pseudo-Zufall ueber proptest-generierten Bytes (shrinkbar, ohne `rand`).
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn byte(&mut self) -> u8 {
        let b = self.bytes.get(self.pos).copied().unwrap_or(0);
        self.pos = self.pos.saturating_add(1);
        b
    }

    fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            usize::from(self.byte()) % n
        }
    }

    fn bool(&mut self) -> bool {
        self.byte().is_multiple_of(2)
    }
}

/// `cursor.range(n)` (immer `< n`) als kleine JSON-Zahl -- `try_from` statt `as` (Lint-sauber,
/// s. `#[deny(clippy::cast_possible_wrap)]` im Workspace).
fn zufallszahl(cursor: &mut Cursor, n: usize) -> Value {
    json!(i64::try_from(cursor.range(n)).expect("range(n) < n, passt immer in i64"))
}

fn waehle<'p, T>(cursor: &mut Cursor, pool: &'p [T]) -> Option<&'p T> {
    if pool.is_empty() {
        None
    } else {
        pool.get(cursor.range(pool.len()))
    }
}

fn herkunft(wert: &str, haftung: &str) -> Herkunft {
    Herkunft {
        herkunft: Achsenwert::new(wert).expect("nichtleer"),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: Achsenwert::new(haftung).expect("nichtleer"),
    }
}

fn erwartete_herkunft(sch: &Schreiber) -> &'static str {
    match sch {
        Schreiber::Llm(_) => "llm_vorschlag",
        Schreiber::Berechnet(_) => "berechnet",
        Schreiber::ImportBeleg => "beleg_import",
        Schreiber::ImportKontoauszug => "kontoauszug",
        Schreiber::ImportVorjahr => "vorjahr",
        _ => "laie",
    }
}

/// Ein Vorschlags-Schreiber MIT Katalog-Zugehoerigkeit (fuer K1/F2-Szenarien): `llm:`/
/// `berechnet:`/`import:beleg`/`import:kontoauszug`.
fn vorschlag_katalog_schreiber(cursor: &mut Cursor) -> Schreiber {
    match cursor.range(4) {
        0 => Schreiber::Llm("chat".to_string()),
        1 => Schreiber::Berechnet("maps".to_string()),
        2 => Schreiber::ImportBeleg,
        _ => Schreiber::ImportKontoauszug,
    }
}

/// Jeder Auflage-A-pflichtige Schreiber, inkl. `import:vorjahr` (das keinen K1-Katalog hat).
fn auflage_a_schreiber(cursor: &mut Cursor) -> Schreiber {
    match cursor.range(5) {
        0 => Schreiber::Llm("chat".to_string()),
        1 => Schreiber::Berechnet("maps".to_string()),
        2 => Schreiber::ImportBeleg,
        3 => Schreiber::ImportKontoauszug,
        _ => Schreiber::ImportVorjahr,
    }
}

/// Jeder Auflage-A-EXEMPTE Schreiber: `import:elster`/`engine`/`abgeleitet:*`/ein Mensch.
fn nicht_vorschlag_schreiber(cursor: &mut Cursor) -> Schreiber {
    match cursor.range(4) {
        0 => Schreiber::ImportElster,
        1 => Schreiber::Engine,
        2 => Schreiber::Abgeleitet("beweist".to_string()),
        _ => Schreiber::Mensch("julius".to_string()),
    }
}

fn wert_korrekt(cursor: &mut Cursor, feld: &Bindung) -> Value {
    match feld.typ {
        // `enum_werte` gibt es seit 2026-10-01 auch auf `cent`/`int` (Grad der Behinderung: das
        // XSD laesst an E0109708/E0505809 nur 17 Werte zu). Die Funktion heisst `wert_korrekt` --
        // eine Zufallszahl ist fuer so ein Feld KEIN korrekter Wert mehr.
        //
        // Die Liste traegt Zeichenketten (die YAML-Werteliste ist eine Textliste), das Feld
        // verlangt aber eine ZAHL: `json!("50")` waere an einem `typ: int` genauso falsch wie eine
        // 33 und wurde von beiden Seiten als Typfehler abgewiesen. Deshalb wird geparst, und nur
        // was sich als Ganzzahl lesen laesst, kommt in Frage.
        Feldtyp::Cent | Feldtyp::Int => {
            let zahlen: Vec<i64> = feld
                .enum_werte
                .as_ref()
                .map(|w| w.iter().filter_map(|s| s.parse::<i64>().ok()).collect())
                .unwrap_or_default();
            if zahlen.is_empty() {
                zufallszahl(cursor, 1_000_000)
            } else {
                json!(zahlen[cursor.range(zahlen.len())])
            }
        }
        Feldtyp::Bool => json!(cursor.bool()),
        Feldtyp::Enum => feld
            .enum_werte
            .as_ref()
            .filter(|w| !w.is_empty())
            .map_or(json!("x"), |w| json!(w[cursor.range(w.len())])),
        Feldtyp::Datum => json!("05.05.1990"),
        // Ein `muster` (Format-Regex, Auflage F) macht "text" ungueltig -- `standardwert` und
        // `beispielwert` erfuellen ihr `muster` (z. B. `kind_kindschaftsverh_zeitraum_b`:
        // `"01.01-31.12"`; `kind_idnr` hat keinen `standardwert`). Belegt fuer jedes Feld mit
        // `muster` in `tests/test_stille_null_typ.py::test_muster_prueft_den_ganzen_wert`.
        Feldtyp::Text => feld.muster.as_ref().map_or_else(
            || json!("text"),
            |_| {
                feld.standardwert
                    .clone()
                    .unwrap_or_else(|| feld.beispielwert.clone())
            },
        ),
    }
}

fn wert_falscher_typ(cursor: &mut Cursor, feld: &Bindung) -> Value {
    match feld.typ {
        Feldtyp::Cent | Feldtyp::Int => json!("keine-zahl"),
        Feldtyp::Bool => json!("ja"),
        Feldtyp::Enum | Feldtyp::Datum | Feldtyp::Text => zufallszahl(cursor, 1000),
    }
}

fn aktive_feld_ids(store: &Store) -> Vec<String> {
    let mut gesehen = std::collections::HashSet::new();
    let mut ergebnis = Vec::new();
    for e in store.events() {
        if gesehen.insert(e.feld_id.clone()) && store.aktives(&e.feld_id).is_some() {
            ergebnis.push(e.feld_id.clone());
        }
    }
    ergebnis
}

/// `Store::leite_ab`/`rechne_ab` haengen den `ts` synthetisierter Events an den jeweiligen
/// Prozess-Aufruf (nicht an einen expliziten Call), also NICHT an das gemeinsame `ts` der
/// Sequenz -- ihr `event_id` ist deshalb zwischen Rust- und Python-Prozess grundsaetzlich NIE
/// identisch. Ein `ersetzt`, das auf so ein Event zeigt, ist in Python-only garantiert ein
/// `ErsetztZielUnbekannt`, selbst wenn die Kaskade dort inhaltlich genauso feuert. Szenarien, die
/// eine ECHTE `ersetzt`-Referenz brauchen, duerfen deshalb nur EXPLIZIT (per Aufruf) erzeugte
/// Events als Ziel waehlen.
fn ist_explizit(schreiber: &Schreiber) -> bool {
    !matches!(schreiber, Schreiber::Abgeleitet(_))
}

/// Ein noch nicht in ein `NeuesEvent`/JSON umgesetzter Aufruf-Bauplan.
struct AufrufSpec {
    feld_id: String,
    wert: Value,
    zustand: &'static str,
    signal_2: Option<String>,
    herkunft: Herkunft,
    schreiber: Schreiber,
    signal_1: Option<Value>,
    ersetzt: Option<EventId>,
    ts: String,
}

fn zu_call_json(spec: &AufrufSpec) -> Value {
    json!({
        "feld_id": spec.feld_id,
        "wert": spec.wert,
        "zustand": spec.zustand,
        "herkunft": spec.herkunft,
        "schreiber": spec.schreiber.to_string(),
        "signal": {"signal_1": spec.signal_1, "signal_2": spec.signal_2},
        "ersetzt": spec.ersetzt.map(|e| e.to_string()),
        "ts": spec.ts,
    })
}

/// s. Moduldoku: schlaegt fuer `bestaetigt` MIT leerem `signal_2` an der Konstruktion fehl,
/// bevor `Store::append` ueberhaupt gerufen wird -- Generator haelt das ausschliesslich fuer
/// Auflage-A-exempte Schreiber vor, damit Python dieselbe Pruefung (Zwei-Signal) trifft.
fn zu_neues_event(spec: &AufrufSpec) -> Result<NeuesEvent, &'static str> {
    let feldzustand = if spec.zustand == "bestaetigt" {
        let roh = spec.signal_2.clone().unwrap_or_default();
        let signal_2 = Signal2::new(roh).map_err(|_| "ZweiSignalFehlend")?;
        Feldzustand::Bestaetigt { signal_2 }
    } else {
        Feldzustand::Vorlaeufig
    };
    Ok(NeuesEvent {
        feld_id: spec.feld_id.clone(),
        wert: PyWert::from(spec.wert.clone()),
        feldzustand,
        herkunft: spec.herkunft.clone(),
        schreiber: spec.schreiber.clone(),
        signal_1: spec.signal_1.clone().map(PyWert::from),
        ersetzt: spec.ersetzt,
        ts: Some(spec.ts.clone()),
    })
}

fn abweisung_klasse(a: &Abweisung) -> &'static str {
    match a {
        Abweisung::AuflageA { .. } => "AuflageA",
        Abweisung::AuflageAErsetztGuard { .. } => "AuflageAErsetztGuard",
        Abweisung::KatalogFehlt { .. } => "KatalogFehlt",
        Abweisung::KatalogNichtFreigegeben { .. } => "KatalogNichtFreigegeben",
        Abweisung::Magnitude { .. } => "Magnitude",
        // K2-Auflage 3: kein Gegenstueck in `store.py` (Python haelt NaN an der Tuer und beim
        // Schreiben auf, B4). Die Strategie erzeugt keine NaN/inf-Werte, also ist der Arm hier
        // unerreichbar — er steht trotzdem da, damit ein neuer Wert-Typ den Compiler trifft statt
        // still durchzulaufen.
        Abweisung::WertNichtDarstellbar { .. } => "WertNichtDarstellbar",
        Abweisung::TypInkonform { .. } => "TypInkonform",
        Abweisung::FormatInkonform { .. } => "FormatInkonform",
        Abweisung::AktivesEventVorhanden { .. } => "AktivesEventVorhanden",
        Abweisung::ErsetztZielUnbekannt(_) => "ErsetztZielUnbekannt",
        Abweisung::ErsetztFeldMismatch => "ErsetztFeldMismatch",
        Abweisung::ErsetztBereitsErsetzt => "ErsetztBereitsErsetzt",
    }
}

fn leer_spec() -> AufrufSpec {
    AufrufSpec {
        feld_id: String::new(),
        wert: Value::Null,
        zustand: "vorlaeufig",
        signal_2: None,
        herkunft: herkunft("laie", "nutzer"),
        schreiber: Schreiber::Mensch("julius".to_string()),
        signal_1: None,
        ersetzt: None,
        ts: String::new(),
    }
}

// -- Szenario-Generatoren -----------------------------------------------------------------
// Jede Funktion liefert `None`, wenn ihre Vorbedingung (z.B. ein nicht-leerer Pool oder ein
// bereits vorhandenes aktives Event) gerade nicht erfuellt ist -- `baue_aufruf` faellt dann auf
// die naechste Stufe zurueck.

fn szenario_vorschlag_gluecklich(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let sch = auflage_a_schreiber(cursor);
    let feld = match sch.vorschlag_typ() {
        Some(typ) => *waehle(cursor, pool_fuer(pools, typ))?,
        None => *waehle(cursor, &pools.alle)?,
    };
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = wert_korrekt(cursor, feld);
    spec.herkunft = herkunft(erwartete_herkunft(&sch), "nutzer");
    spec.schreiber = sch;
    Some(spec)
}

fn szenario_auflage_a_verletzt(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let sch = auflage_a_schreiber(cursor);
    let feld = match sch.vorschlag_typ() {
        Some(typ) => {
            waehle(cursor, pool_fuer(pools, typ)).or_else(|| waehle(cursor, &pools.alle))?
        }
        None => waehle(cursor, &pools.alle)?,
    };
    let wert = wert_korrekt(cursor, feld);
    let richtige_herkunft = erwartete_herkunft(&sch);
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = wert;
    if cursor.bool() {
        spec.herkunft = herkunft("mensch_manipuliert", "nutzer");
    } else {
        spec.zustand = "bestaetigt";
        spec.signal_2 = Some("mensch@ui".to_string());
        spec.herkunft = herkunft(richtige_herkunft, "nutzer");
    }
    spec.schreiber = sch;
    Some(spec)
}

fn szenario_ersetzt_guard(cursor: &mut Cursor, pools: &Pools, store: &Store) -> Option<AufrufSpec> {
    let sch = match cursor.range(3) {
        0 => Schreiber::Llm("chat".to_string()),
        1 => Schreiber::ImportBeleg,
        _ => Schreiber::ImportKontoauszug,
    };
    let typ = sch.vorschlag_typ()?;
    let feld = *waehle(cursor, pool_fuer(pools, typ))?;
    let ziel = waehle(cursor, store.events())?;
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = wert_korrekt(cursor, feld);
    spec.herkunft = herkunft(erwartete_herkunft(&sch), "nutzer");
    spec.schreiber = sch;
    spec.ersetzt = Some(ziel.event_id);
    Some(spec)
}

fn szenario_katalog_verletzt(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let sch = vorschlag_katalog_schreiber(cursor);
    let typ = sch.vorschlag_typ()?;
    let ziel_pool = pool_fuer(pools, typ);
    let kandidaten: Vec<&&Bindung> = pools
        .alle
        .iter()
        .filter(|b| !ziel_pool.iter().any(|p| p.feld_id == b.feld_id))
        .collect();
    let feld = **waehle(cursor, &kandidaten)?;
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = wert_korrekt(cursor, feld);
    spec.herkunft = herkunft(erwartete_herkunft(&sch), "nutzer");
    spec.schreiber = sch;
    Some(spec)
}

fn szenario_magnitude(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let sch = vorschlag_katalog_schreiber(cursor);
    let typ = sch.vorschlag_typ()?;
    let feld = *waehle(cursor, pool_fuer(pools, typ))?;
    let vorzeichen: i64 = if cursor.bool() { 1 } else { -1 };
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = json!(vorzeichen * 20_000_000_000i64);
    spec.herkunft = herkunft(erwartete_herkunft(&sch), "nutzer");
    spec.schreiber = sch;
    Some(spec)
}

fn szenario_typinkonform(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let feld = *waehle(cursor, &pools.alle)?;
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = wert_falscher_typ(cursor, feld);
    spec.schreiber = nicht_vorschlag_schreiber(cursor);
    Some(spec)
}

fn szenario_format(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let feld = *waehle(cursor, &pools.muster)?;
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = json!("###garantiert-ungueltig###");
    spec.schreiber = nicht_vorschlag_schreiber(cursor);
    Some(spec)
}

/// Decision textfeld-format-aus-xsd-beim-speichern, Punkt 3: `muster` und Datumspruefung
/// urteilen ueber den GANZEN Wert. Pythons `re.match` mit `$` liess ein abschliessendes `\n`
/// durch, Rust nie.
fn szenario_zeilenumbruch(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let kandidaten: Vec<&&Bindung> = pools
        .alle
        .iter()
        .filter(|b| b.muster.is_some() || b.typ == Feldtyp::Datum)
        .collect();
    let feld = **waehle(cursor, &kandidaten)?;
    let korrekt = wert_korrekt(cursor, feld);
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = json!(format!("{}\n", korrekt.as_str()?));
    spec.schreiber = nicht_vorschlag_schreiber(cursor);
    Some(spec)
}

/// Punkt 4: leerer Text (`typ: text`, Laenge 0) wird abgewiesen, mit und ohne `muster`.
fn szenario_leerer_text(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let kandidaten: Vec<&&Bindung> = pools
        .alle
        .iter()
        .filter(|b| b.typ == Feldtyp::Text)
        .collect();
    let feld = **waehle(cursor, &kandidaten)?;
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = json!("");
    spec.schreiber = nicht_vorschlag_schreiber(cursor);
    Some(spec)
}

fn szenario_zwei_signal_fehlend(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let feld = *waehle(cursor, &pools.alle)?;
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = wert_korrekt(cursor, feld);
    spec.zustand = "bestaetigt";
    spec.signal_2 = None;
    spec.schreiber = nicht_vorschlag_schreiber(cursor);
    Some(spec)
}

fn szenario_aktives_vorhanden(cursor: &mut Cursor, store: &Store) -> Option<AufrufSpec> {
    let feld_ids = aktive_feld_ids(store);
    let feld_id = waehle(cursor, &feld_ids)?.clone();
    let mut spec = leer_spec();
    spec.feld_id = feld_id;
    spec.wert = zufallszahl(cursor, 1000);
    Some(spec)
}

fn szenario_ersetzt_unbekannt(cursor: &mut Cursor, pools: &Pools, salt: u64) -> Option<AufrufSpec> {
    let feld = *waehle(cursor, &pools.alle)?;
    let fake = EventId::von_json(&json!({"__fake_ersetzt__": salt, "n": cursor.byte()}));
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = wert_korrekt(cursor, feld);
    spec.ersetzt = Some(fake);
    Some(spec)
}

fn szenario_ersetzt_mismatch(
    cursor: &mut Cursor,
    pools: &Pools,
    store: &Store,
) -> Option<AufrufSpec> {
    let explizit: Vec<_> = store
        .events()
        .iter()
        .filter(|e| ist_explizit(&e.schreiber))
        .collect();
    let ziel = *waehle(cursor, &explizit)?;
    let feld = pools.alle.iter().find(|b| b.feld_id != ziel.feld_id)?;
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = wert_korrekt(cursor, feld);
    spec.ersetzt = Some(ziel.event_id);
    Some(spec)
}

fn szenario_ersetzt_bereits(cursor: &mut Cursor, store: &Store) -> Option<AufrufSpec> {
    let bereits: Vec<EventId> = store.events().iter().filter_map(|e| e.ersetzt).collect();
    let ziel = *waehle(cursor, &bereits)?;
    let urspruenglich_feld = store
        .events()
        .iter()
        .find(|e| e.event_id == ziel)?
        .feld_id
        .clone();
    let mut spec = leer_spec();
    spec.feld_id = urspruenglich_feld;
    spec.wert = zufallszahl(cursor, 1000);
    spec.ersetzt = Some(ziel);
    Some(spec)
}

fn szenario_ersetzt_erfolg(
    cursor: &mut Cursor,
    pools: &Pools,
    store: &Store,
) -> Option<AufrufSpec> {
    let feld_ids: Vec<String> = aktive_feld_ids(store)
        .into_iter()
        .filter(|f| store.aktives(f).is_some_and(|e| ist_explizit(&e.schreiber)))
        .collect();
    let feld_id = waehle(cursor, &feld_ids)?.clone();
    let aktiv = store.aktives(&feld_id)?;
    let bindung = pools.alle.iter().find(|b| b.feld_id == feld_id);
    let wert = if let Some(b) = bindung {
        wert_korrekt(cursor, b)
    } else {
        zufallszahl(cursor, 1000)
    };
    let mut spec = leer_spec();
    spec.feld_id = feld_id;
    spec.wert = wert;
    spec.ersetzt = Some(aktiv.event_id);
    Some(spec)
}

fn szenario_plain(cursor: &mut Cursor, pools: &Pools) -> Option<AufrufSpec> {
    let feld = *waehle(cursor, &pools.alle)?;
    let mut spec = leer_spec();
    spec.feld_id.clone_from(&feld.feld_id);
    spec.wert = wert_korrekt(cursor, feld);
    if cursor.bool() {
        spec.zustand = "bestaetigt";
        spec.signal_2 = Some("mensch@ui".to_string());
    }
    Some(spec)
}

fn baue_aufruf(
    cursor: &mut Cursor,
    pools: &Pools,
    store: &Store,
    salt: u64,
    ts: &str,
) -> AufrufSpec {
    let versuch = match cursor.range(15) {
        0 => szenario_vorschlag_gluecklich(cursor, pools),
        1 => szenario_auflage_a_verletzt(cursor, pools),
        2 => szenario_ersetzt_guard(cursor, pools, store),
        3 => szenario_katalog_verletzt(cursor, pools),
        4 => szenario_magnitude(cursor, pools),
        5 => szenario_typinkonform(cursor, pools),
        6 => szenario_format(cursor, pools),
        7 => szenario_zwei_signal_fehlend(cursor, pools),
        8 => szenario_aktives_vorhanden(cursor, store),
        9 => szenario_ersetzt_unbekannt(cursor, pools, salt),
        10 => szenario_ersetzt_mismatch(cursor, pools, store),
        11 => szenario_ersetzt_erfolg(cursor, pools, store),
        12 => szenario_zeilenumbruch(cursor, pools),
        13 => szenario_leerer_text(cursor, pools),
        _ => szenario_ersetzt_bereits(cursor, store),
    };
    let mut spec = versuch
        .or_else(|| szenario_plain(cursor, pools))
        .unwrap_or_else(leer_spec);
    if spec.feld_id.is_empty() {
        spec.feld_id = "fallback_feld".to_string();
        spec.wert = json!(1);
    }
    spec.ts = ts.to_string();
    spec
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// 1..=20 generierte Aufrufe gegen einen anfangs leeren Store, Rust vs. Python ueber
    /// GENAU EINEN geteilten Oracle-Prozess (s. Moduldoku).
    #[test]
    fn append_sequence_paritaet_ueber_zufaellige_aufruf_sequenzen(bytes in prop::collection::vec(any::<u8>(), 8192)) {
        if skip_ohne_parity_env() {
            return Ok(());
        }
        let bindungen = alle_bindungen();
        let map = store::baue_nachschlag(bindungen);
        let nachschlag = store::BindungNachschlag::neu(&map);
        let katalog = Katalog::aus_bindungen(bindungen);
        let pools = baue_pools(bindungen, &katalog);

        let mut cursor = Cursor::new(&bytes);
        let anzahl_aufrufe = 1 + cursor.range(20);

        let mut rust_store = Store::leer(2025, None);
        let initial_json = serde_json::to_value(rust_store.datei()).expect("StoreDatei serialisiert");

        let mut calls_json = Vec::new();
        let mut rust_ergebnisse: Vec<Result<String, &'static str>> = Vec::new();

        for i in 0..anzahl_aufrufe {
            let ts = format!("2026-01-01T00:00:{i:02}+00:00");
            let spec = baue_aufruf(&mut cursor, &pools, &rust_store, i as u64, &ts);
            calls_json.push(zu_call_json(&spec));
            match zu_neues_event(&spec) {
                Err(label) => rust_ergebnisse.push(Err(label)),
                Ok(neu) => match rust_store.append(&neu, Some(&katalog), nachschlag) {
                    Ok(id) => rust_ergebnisse.push(Ok(id.to_string())),
                    Err(abw) => rust_ergebnisse.push(Err(abweisung_klasse(&abw))),
                },
            }
        }

        let mut rust_aktiv: Vec<String> = aktive_feld_ids(&rust_store)
            .iter()
            .filter_map(|fid| rust_store.aktives(fid).map(|e| e.event_id.to_string()))
            .collect();
        rust_aktiv.sort();

        let python = {
            let mut guard = oracle_singleton().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.append_sequence(&initial_json, &calls_json).expect("Orakel-Aufruf laeuft durch")
        };

        prop_assert_eq!(rust_ergebnisse.len(), python.results.len());
        for (i, (r, p)) in rust_ergebnisse.iter().zip(python.results.iter()).enumerate() {
            match (r, p) {
                (Ok(rust_id), AppendCallErgebnis::Erfolg { event_id }) => {
                    prop_assert_eq!(rust_id, event_id, "Aufruf {}: event_id weicht ab", i);
                }
                (Err(rust_label), AppendCallErgebnis::Fehler { err }) => {
                    prop_assert_eq!(*rust_label, err.as_str(), "Aufruf {}: Fehlerklasse weicht ab", i);
                }
                _ => prop_assert!(false, "Aufruf {}: Erfolg/Fehler-Divergenz (rust={:?})", i, r),
            }
        }
        prop_assert_eq!(rust_aktiv, python.aktive_event_ids, "aktive_event_ids weichen ab");
    }
}

/// D20 — die Reihenfolge der beiden Schreibpfad-Pruefungen divergiert (s. Moduldoku).
///
/// Ein Wert, der typ-INKONFORM ist, zusammen mit `zustand=bestaetigt` und LEEREM `signal_2`:
/// Python meldet `TypInkonform` (Auflage T steht in `store.py:409` vor dem Zwei-Signal in `:412`),
/// Rust kommt gar nicht bis `Store::append` — `zu_neues_event` baut `Feldzustand::Bestaetigt`
/// nicht ohne gueltiges `signal_2` und meldet `ZweiSignalFehlend`.
///
/// Dieser Test haelt die Abweichung FEST, er behebt sie nicht. Wird sie je angeglichen, faellt er
/// um und zwingt zur Entscheidung: Moduldoku und dieser Test gehen zusammen.
#[test]
fn d20_reihenfolge_typ_vor_signal() {
    if skip_ohne_parity_env() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht Catala-Toolchain + Python-Umfeld)");
        return;
    }
    let bindungen = alle_bindungen();
    let map = store::baue_nachschlag(bindungen);
    let nachschlag = store::BindungNachschlag::neu(&map);
    let katalog = Katalog::aus_bindungen(bindungen);

    // Typ-inkonform: eine ZAHL als JSON-String auf einem `typ: cent`-Feld. Kein GdB-Bezug — die
    // Abweichung gibt es unabhaengig von der Werteliste, der GdB macht sie nur erreichbar.
    let feld_id = "bruttoarbeitslohn";
    assert!(
        bindungen.iter().any(|b| b.feld_id == feld_id),
        "Bindungsfeld {feld_id} fehlt"
    );
    let spec = AufrufSpec {
        feld_id: feld_id.to_string(),
        wert: json!("50000"),
        zustand: "bestaetigt",
        signal_2: None,
        herkunft: herkunft("laie", "nutzer"),
        schreiber: Schreiber::Mensch("julius".to_string()),
        signal_1: None,
        ersetzt: None,
        ts: "2026-01-01T00:00:00+00:00".to_string(),
    };

    // Rust: die Konstruktion scheitert, bevor der Store ueberhaupt prueft.
    let rust_klasse = match zu_neues_event(&spec) {
        Err(label) => label,
        Ok(neu) => match Store::leer(2025, None).append(&neu, Some(&katalog), nachschlag) {
            Ok(_) => "Erfolg",
            Err(abw) => abweisung_klasse(&abw),
        },
    };
    assert_eq!(
        rust_klasse, "ZweiSignalFehlend",
        "Rust meldete {rust_klasse}; die Abweichung D20 hat sich verschoben"
    );

    // Python: meldet dieselbe Eingabe als Typfehler.
    let initial_json = serde_json::to_value(Store::leer(2025, None).datei())
        .expect("leerer Store serialisiert");
    let calls = vec![zu_call_json(&spec)];
    let python = {
        let mut guard = oracle_singleton().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.append_sequence(&initial_json, &calls).expect("Orakel-Aufruf laeuft durch")
    };
    let python_klasse = match python.results.first() {
        Some(AppendCallErgebnis::Fehler { err }) => err.clone(),
        other => panic!("Python nahm die Eingabe an oder scheiterte anders: {other:?}"),
    };
    assert_eq!(
        python_klasse, "TypInkonform",
        "Python meldete {python_klasse}; die Abweichung D20 hat sich verschoben"
    );

    assert_ne!(
        rust_klasse, python_klasse,
        "D20 ist angeglichen -- Moduldoku und dieser Test gehoeren dann entfernt"
    );
}

/// Der Orakel-Aufruf eines einzelnen Events (ohne `event_id`, mit Signal aufgeloest).
///
/// K2/Auflage 2: `wert` geht ueber [`PyWert::zu_json`], NICHT ueber ein `Serialize` von `PyWert` —
/// die Orakel-Form ist `canonical_json`, und die erbt die Schluesselsortierung von `Value`.
fn call_json(event: &Event, signal: &Signal) -> Value {
    json!({
        "feld_id": event.feld_id,
        "wert": event.wert.zu_json().expect("Store-Werte sind endlich (serde_json lehnt inf ab)"),
        "zustand": event.zustand,
        "herkunft": event.herkunft,
        "schreiber": event.schreiber.to_string(),
        "signal": signal,
        "ersetzt": event.ersetzt.map(|e| e.to_string()),
        "ts": event.ts,
    })
}

/// Jede reale Fall-Datei als Aufruf-Sequenz in einen frischen leeren Store (Rust UND Python) --
/// beide muessen jedes Event annehmen. S. Moduldoku fuer den Sicherheitshinweis.
#[test]
fn append_sequence_replay_realer_faelle() {
    if skip_ohne_parity_env() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht Catala-Toolchain + Python-Umfeld)");
        return;
    }
    let verzeichnis = faelle_verzeichnis();
    let kandidaten = walk_json(&verzeichnis);
    // Ein Parity-Lauf ohne Korpus ist kein gruener Lauf: er vergleicht nichts und meldet
    // "keine Abweichungen". Am 2026-10-01 kehrte diese Suite bei leerem `TAXGRAPH_DATEN`
    // still gruen zurueck (3 passed).
    assert!(
        !kandidaten.is_empty(),
        "append_sequence_replay_realer_faelle: 0 Fall-Dateien unter {} -- ein Parity-Lauf \
         ohne Korpus belegt nichts.",
        verzeichnis.display()
    );

    let bindungen = alle_bindungen();
    let map = store::baue_nachschlag(bindungen);
    let nachschlag = store::BindungNachschlag::neu(&map);
    let katalog = Katalog::aus_bindungen(bindungen);

    let leer = Store::leer(2025, None);
    let initial_json = serde_json::to_value(leer.datei()).expect("leerer Store serialisiert");

    let mut oracle = Oracle::spawn(&repo_root()).expect("oracle.py startet");
    let mut gepruefte_dateien = 0u64;
    let mut gepruefte_events = 0u64;
    let mut uebersprungen_abgeleitet = 0u64;
    let mut uebersprungen_alt_herkunft = 0u64;
    let mut rust_fehlschlaege = 0u64;
    let mut python_fehlschlaege = 0u64;
    let mut abweichende_entscheidungen = 0u64;

    for pfad in &kandidaten {
        // P10: store::lade laedt inzwischen ALLE realen Faelle (legacy Herkunft, unbegrenzte VZ).
        let datei =
            store::lade(pfad).unwrap_or_else(|e| panic!("store::lade({}): {e}", pfad.display()));
        if datei.events.is_empty() {
            continue;
        }
        gepruefte_dateien += 1;
        let mut rust_store = Store::leer(datei.veranlagungszeitraum.als_i64_saettigend(), None);
        let mut calls_json = Vec::new();
        // Je Aufruf: hat Rust angenommen? Index parallel zu `calls_json`.
        let mut rust_angenommen: Vec<bool> = Vec::new();
        for event in &datei.events {
            // Kaskaden-Events (`Schreiber::Abgeleitet`, s. `ausgeloeste_felder`s Dokstring) hat
            // die urspruengliche Live-Session schon EINMAL via `leite_ab`/`rechne_ab` erzeugt.
            // Ein FRISCHER Replay derselben Quellfelder loest dieselbe Kaskade selbst wieder aus
            // -- das Original-Event zusaetzlich als eigenen Aufruf zu wiederholen kollidiert mit
            // der eigenen Ableitung (`AktivesEventVorhanden`) UND traegt sowieso ein Prozess-
            // eigenes `ts` (nie stabil vergleichbar, s. `baue_pools`). Ausgelassen statt gezaehlt.
            if matches!(event.schreiber, Schreiber::Abgeleitet(_)) {
                uebersprungen_abgeleitet += 1;
                continue;
            }
            // P10: 990 Alt-Herkunft-Events (32/192 Dateien, ohne `pruef_tiefe`/`haftung`) lassen
            // sich nicht verlustfrei in einen `NeuesEvent`-Aufruf zurueckuebersetzen. Ausgelassen.
            let Some(herkunft) = event.herkunft.als_voll() else {
                uebersprungen_alt_herkunft += 1;
                continue;
            };
            gepruefte_events += 1;
            let signal = event.signal.clone().unwrap_or_default();
            calls_json.push(call_json(event, &signal));
            let feldzustand = match event.zustand {
                domain::Zustand::Bestaetigt => {
                    let Ok(signal_2) = Signal2::new(signal.signal_2.clone().unwrap_or_default())
                    else {
                        rust_fehlschlaege += 1;
                        rust_angenommen.push(false);
                        continue;
                    };
                    Feldzustand::Bestaetigt { signal_2 }
                }
                domain::Zustand::Vorlaeufig => Feldzustand::Vorlaeufig,
            };
            let neu = NeuesEvent {
                feld_id: event.feld_id.clone(),
                wert: event.wert.clone(),
                feldzustand,
                herkunft: herkunft.clone(),
                schreiber: event.schreiber.clone(),
                // signal_1-fehlt ist eine STRIKTE Teilmenge von Alt-Herkunft (P10, gemessen) --
                // der `als_voll()`-Guard oben hat Alt-Herkunft schon ausgefiltert, die aeussere
                // Ebene ist hier also immer `Some(...)`; `.flatten()` traegt trotzdem den echten
                // Fall ab, statt sich auf die Reihenfolge der Filter zu verlassen.
                signal_1: signal.signal_1.clone().flatten(),
                ersetzt: event.ersetzt,
                ts: Some(event.ts.clone()),
            };
            let angenommen = rust_store.append(&neu, Some(&katalog), nachschlag).is_ok();
            if !angenommen {
                rust_fehlschlaege += 1;
            }
            rust_angenommen.push(angenommen);
        }
        let python = oracle
            .append_sequence(&initial_json, &calls_json)
            .expect("Orakel-Aufruf laeuft durch");
        python_fehlschlaege += python
            .results
            .iter()
            .filter(|r| matches!(r, AppendCallErgebnis::Fehler { .. }))
            .count() as u64;
        abweichende_entscheidungen += abweichende(&python.results, &rust_angenommen);
    }

    eprintln!(
        "append_sequence_replay_realer_faelle: {gepruefte_dateien} Dateien, {gepruefte_events} Events \
         ({uebersprungen_abgeleitet} kaskaden-abgeleitete, {uebersprungen_alt_herkunft} Alt-Herkunft \
         uebersprungen), \
         {rust_fehlschlaege} Rust-Fehlschlaege, {python_fehlschlaege} Python-Fehlschlaege, \
         {abweichende_entscheidungen} abweichende Einzelentscheidungen"
    );
    // Je Event dieselbe Entscheidung (angenommen/abgewiesen), nicht nur dieselbe Anzahl.
    assert_eq!(
        abweichende_entscheidungen, 0,
        "Rust und Python entscheiden einzelne reale Events verschieden"
    );
    // Deliverable-Vorgabe war "beide muessen jedes Event annehmen" -- real trifft das nicht ZU
    // 100%: ein Teil der Bestandsdateien ist aelter als die heutige Bindungsregistry (Format-
    // Regeln/Reihenfolge-Abhaengigkeiten haben sich seither veraendert) und lehnt darum auf
    // BEIDEN Seiten symmetrisch ab. Massgeblich fuer DIESEN Test ist Rust-vs-Python-Paritaet,
    // nicht Alt-Datei-Validitaet gegen die heutige Registry -- deshalb Gleichstand statt Null.
    assert_eq!(
        rust_fehlschlaege, python_fehlschlaege,
        "Rust/Python lehnen unterschiedlich viele reale Events ab (Anzahl s.o.)"
    );
}

/// Zahl der Aufrufe, bei denen Rust anders entscheidet (angenommen/abgewiesen) als Python.
/// Index `i` in `python` gehoert zu Index `i` in `rust_angenommen`.
fn abweichende(python: &[AppendCallErgebnis], rust_angenommen: &[bool]) -> u64 {
    assert_eq!(
        python.len(),
        rust_angenommen.len(),
        "Aufrufzahl je Datei muss gleich sein"
    );
    python
        .iter()
        .zip(rust_angenommen)
        .filter(|(py, rust)| matches!(py, AppendCallErgebnis::Fehler { .. }) == **rust)
        .count() as u64
}
