//! `POST /fall/{id}/einreichen` im Vergleichslauf `api_http_paritaet` (Folge 6, Stufe 2): Python
//! gegen Rust, beide gegen eine ATTRAPPE von `libericapi.so` (`eric_attrappe.c`).
//!
//! Warum eine Attrappe: die echte ERiC-Bibliothek liefert ohne registrierte Hersteller-ID immer
//! `rc = 610301202`; `rc = 0`, Plausibilitaetsfehler und die uebrigen Klassen sind ohne diese ID
//! nicht zu erreichen. Die Attrappe antwortet nach einem Skript und schreibt auf, was sie bekam.
//! Beide Server laden dieselbe Datei (`ERIC_DIR`), jeder mit eigenen Steuerdateien
//! (`ERIC_ATTRAPPE_DIR`). Der Lauf vergleicht Antwort, Audit, `fehler.log`, Fallakte mit Snapshot
//! UND das XML, das `ERiC` sah, samt Datenart und Flags.
//!
//! Nie die echte Bibliothek und nie eine echte Hersteller-ID: `ERIC_DIR` zeigt in ein Wegwerf-
//! Verzeichnis, `HOME` ebenso (sonst fiele die Suche auf `~/02_Software/eric` zurueck), die
//! Hersteller-ID ist die oeffentlich bekannte Platzhalter-ID `74931`. Die Attrappe kennt kein
//! Versandflag; der Lauf verlangt `flags == 2` (`ERIC_VALIDIERE`) und `NULL` fuer Druck-, Crypto-
//! und Serverantwort-Parameter bei JEDEM Aufruf.
//!
//! Das Schema `E10-2025.xsd` braucht der XML-Writer (beide Seiten); es kommt aus der lokalen
//! ERiC-Auslieferung und wird in das Wegwerf-Verzeichnis kopiert. Fehlt es, scheitert der Lauf —
//! ausser mit `TAXGRAPH_OHNE_XSD=1` (die CI hat keine Auslieferung).
use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

use super::{repo_root, schreibe_seed, skip, Anfrage, Modus, Paar};

/// Die Platzhalter-ID aus den amtlichen ERiC-Beispielen. Sie steht in `rust/fixtures/e2e/*.xml`.
const HERSTELLER_ID: &str = "74931";

/// Wie die Bibliothek im `ERIC_DIR` liegt, wenn ein Szenario anfaengt.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lib {
    /// Keine `libericapi.so`.
    Fehlt,
    /// Eine Datei dieses Namens, die keine Bibliothek ist.
    Muell,
    /// Die Attrappe.
    Gut,
}

struct E {
    name: &'static str,
    fall: &'static str,
    rumpf: Value,
    /// Skript der Attrappe, Bloecke durch `\n=====\n`; `None`: keine Datei.
    skript: Option<String>,
    lib: Lib,
    init_rc: Option<i32>,
    einstellung_rc: Option<i32>,
    /// Soll, an Python gemessen (`berichte/einreichen-bau.md`).
    status: u16,
    grund: Option<&'static str>,
    /// Anzahl der Aufrufe von `EricBearbeiteVorgang`.
    aufrufe: usize,
    /// Der Anfang von `detail`.
    detail_beginnt: Option<&'static str>,
    /// Das XML, das die Attrappe sah, gleich dieser Datei unter `rust/fixtures/e2e`.
    xml_wie: Option<&'static str>,
    /// Texte, die im XML an `ERiC` stehen muessen.
    xml_enthaelt: Vec<&'static str>,
}

impl E {
    fn neu(name: &'static str, fall: &'static str, status: u16, grund: Option<&'static str>) -> Self {
        Self {
            name,
            fall,
            rumpf: json!({}),
            skript: None,
            lib: Lib::Gut,
            init_rc: None,
            einstellung_rc: None,
            status,
            grund,
            aufrufe: 0,
            detail_beginnt: None,
            xml_wie: None,
            xml_enthaelt: vec![],
        }
    }
    fn rumpf(mut self, r: Value) -> Self {
        self.rumpf = r;
        self
    }
    /// `ERiC` antwortet mit diesem rc und diesem Puffer; `log` kommt in `eric.log`.
    fn eric(mut self, rc: i64, puffer: &str, log: &str) -> Self {
        let mut s = format!("{rc}\n{puffer}");
        if !log.is_empty() {
            s.push_str("\n--LOG--\n");
            s.push_str(log);
        }
        self.skript = Some(s);
        self.aufrufe = 1;
        self
    }
    fn lib(mut self, l: Lib) -> Self {
        self.lib = l;
        self
    }
    fn init(mut self, rc: i32) -> Self {
        self.init_rc = Some(rc);
        self
    }
    fn einstellung(mut self, rc: i32) -> Self {
        self.einstellung_rc = Some(rc);
        self
    }
    fn detail(mut self, t: &'static str) -> Self {
        self.detail_beginnt = Some(t);
        self
    }
    fn xml_wie(mut self, f: &'static str) -> Self {
        self.xml_wie = Some(f);
        self
    }
    fn xml_mit(mut self, t: &'static str) -> Self {
        self.xml_enthaelt.push(t);
        self
    }
}

// ---------------------------------------------------------------- Faelle

/// Die Faelle des Laufs: drei vollstaendige aus `rust/fixtures/e2e` und Abwandlungen davon.
fn faelle() -> Vec<(&'static str, Value)> {
    let lade = |n: &str| -> Value {
        let p = repo_root().join(format!("rust/fixtures/e2e/{n}.json"));
        serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
    };
    let mut v: Vec<(&'static str, Value)> = Vec::new();
    for (name, quelle) in [("e_ges", "gesamt"), ("e_arb", "arbeitnehmer"), ("e_ren", "rentner")] {
        v.push((name, lade(quelle)));
    }
    let aendere = |name: &'static str, f: &dyn Fn(&mut Value)| -> (&'static str, Value) {
        let mut d = lade("gesamt");
        f(&mut d);
        (name, d)
    };
    let setze = |d: &mut Value, feld: &str, wert: Value| {
        for e in d["events"].as_array_mut().unwrap() {
            if e["feld_id"] == feld {
                e["wert"] = wert.clone();
            }
        }
    };
    // Ein Wert, dem das zweite Signal fehlt: `deklaration_unvollstaendig`.
    v.push(aendere("e_vorl", &|d| {
        for e in d["events"].as_array_mut().unwrap() {
            if e["feld_id"] == "kap_kapitalertraege" {
                e["zustand"] = json!("vorlaeufig");
                e["signal"]["signal_2"] = Value::Null;
            }
        }
    }));
    // Aggregat und Topf der Kapitalertraege zugleich: Sperrgrund `kapital_semantik_offen`.
    v.push(aendere("e_sperre", &|d| {
        setze(d, "kap_kapitalertraege", json!(500_000));
        setze(d, "kap_gewinn_aktien", json!(500_000));
        setze(d, "kein_kap", json!(false));
    }));
    // Ohne Geburtsdatum: das Abgabe-Gate des Writers (`xml_nicht_baubar`).
    v.push(aendere("e_ohnegeb", &|d| {
        d["events"]
            .as_array_mut()
            .unwrap()
            .retain(|e| e["feld_id"] != "stammdaten_geburtsdatum");
    }));
    // 2026 hat kein Schema in ERiC 44.2.4.0.
    v.push(aendere("e_z26", &|d| d["veranlagungszeitraum"] = json!(2026)));
    v.push(aendere("e_scheibe_x", &|d| d["scheibe"] = json!("xyz")));
    let leer = |scheibe: &str| {
        json!({"version": 1, "veranlagungszeitraum": 2025, "scheibe": scheibe, "events": [],
               "snapshots": []})
    };
    v.push(("e_leer", leer("gesamt")));
    v.push(("e_an", leer("an_gesamt")));
    v.push(("e_ep", leer("ep")));
    for (name, d) in &mut v {
        d["fall_id"] = json!(name);
    }
    v
}

// ---------------------------------------------------------------- Szenarien

/// `n` Fehlermeldungen im Aufbau einer ERiC-Antwort.
fn fehlerliste(n: usize) -> String {
    let mut s = String::from("<EricAntwort><Rueckgabe>");
    for i in 0..n {
        let _ = write!(
            s,
            "<FehlerRegelpruefung><Nr>{i}</Nr><Text>Regel {i} verletzt</Text></FehlerRegelpruefung>"
        );
    }
    s.push_str("</Rueckgabe></EricAntwort>");
    s
}

fn szenarien() -> Vec<E> {
    let ok = |name, fall| E::neu(name, fall, 200, None).eric(0, "<Ok/>", "");
    let lib_fehlt = "libericapi.so nicht gefunden";
    let mut viele_zeilen = String::new();
    for i in 0..14 {
        let _ = write!(viele_zeilen, "EC(ab{i}) Fehler {i}\nINFO nichts {i}\nx ERROR {i}\r\n");
    }
    let v = vec![
        // --- Die Bibliothek laedt nicht (die Reihenfolge zaehlt: ERiC wird je Prozess einmal geladen)
        E::neu("A1 Bibliothek fehlt", "e_ges", 503, Some("eric_nicht_verfuegbar"))
            .lib(Lib::Fehlt)
            .detail(lib_fehlt),
        E::neu("A2 Datei ist keine Bibliothek", "e_ges", 503, Some("eric_nicht_verfuegbar"))
            .lib(Lib::Muell),
        E::neu("A3 Initialisierung scheitert", "e_ges", 503, Some("eric_nicht_verfuegbar"))
            .init(7)
            .detail("EricInitialisiere fehlgeschlagen (rc=7)."),
        E::neu("A4 Meldungs-Cap laesst sich nicht anheben", "e_ges", 503, Some("eric_nicht_verfuegbar"))
            .einstellung(5)
            .detail("EricEinstellungSetzen(validieren.fehler_max=1000) fehlgeschlagen (rc=5)"),
        // --- rc 0
        ok("B1 rc 0, gesamt, Standard-Land", "e_ges").xml_wie("gesamt.xml").xml_mit("<Ziel>BY</Ziel>"),
        ok("B2 rc 0, Arbeitnehmer", "e_arb").xml_wie("arbeitnehmer.xml"),
        ok("B3 rc 0, Rentner", "e_ren").xml_wie("rentner.xml"),
        ok("B4 rc 0, Land NW", "e_ges")
            .rumpf(json!({"empfaenger_land": "NW"}))
            .xml_mit("<Ziel>NW</Ziel>"),
        ok("B5 rc 0, Land Zahl", "e_ges")
            .rumpf(json!({"empfaenger_land": 5}))
            .xml_mit("<Ziel>5</Ziel>"),
        ok("B6 rc 0, Land Liste", "e_ges")
            .rumpf(json!({"empfaenger_land": [1, "a"]}))
            .xml_mit("<Ziel>[1, 'a']</Ziel>"),
        ok("B7 rc 0, Land wahr", "e_ges")
            .rumpf(json!({"empfaenger_land": true}))
            .xml_mit("<Ziel>True</Ziel>"),
        ok("B8 rc 0, Land leer ist BY", "e_ges")
            .rumpf(json!({"empfaenger_land": ""}))
            .xml_mit("<Ziel>BY</Ziel>"),
        ok("B9 rc 0, Land mit Sonderzeichen", "e_ges")
            .rumpf(json!({"empfaenger_land": "B<&>Y"}))
            .xml_mit("<Ziel>B&lt;&amp;&gt;Y</Ziel>"),
        ok("B10 rc 0, Land sehr lang", "e_ges").rumpf(json!({"empfaenger_land": "A".repeat(300)})),
        // --- Plausibilitaet
        E::neu("C1 Plausibilitaetsfehler", "e_ges", 422, Some("plausibilitaet_verletzt"))
            .eric(610_001_002, &fehlerliste(3), ""),
        E::neu("C2 Plausibilitaetsfehler, Cap erreicht", "e_ges", 422, Some("plausibilitaet_verletzt"))
            .eric(610_001_002, &fehlerliste(1000), ""),
        E::neu("C3 kein Pruefmodul", "e_ges", 422, Some("kein_pruefmodul_fuer_vz"))
            .eric(610_001_042, "<Fehler/>", ""),
        // --- Klassen ohne Urteil
        E::neu("D1 Schema-Sammelcode mit Text", "e_ges", 422, Some("rc_kein_plausibilitaetsverdikt"))
            .eric(610_301_200, "<Fehler><Text>Schema</Text></Fehler>", ""),
        E::neu("D2 Schema-Sammelcode, Puffer leer, Log ohne Treffer", "e_ges", 422, Some("rc_kein_plausibilitaetsverdikt"))
            .eric(610_301_200, "", "INFO nichts\nINFO noch nichts"),
        E::neu("D3 Schema-Sammelcode, Puffer leer, Log mit Treffern", "e_ges", 422, Some("rc_kein_plausibilitaetsverdikt"))
            .eric(610_301_200, "", "EC(1) kaputt\nx ERROR y\nnoch eine Zeile\nEC(cr) a\rEC(cr) b\n"),
        E::neu("D4 Hersteller-ID gesperrt, Puffer leer, viele Zeilen mit CRLF", "e_ges", 422, Some("rc_kein_plausibilitaetsverdikt"))
            .eric(610_301_202, "", &viele_zeilen),
        E::neu("D5 unerwartete Elemente, Puffer ohne Text", "e_ges", 422, Some("rc_kein_plausibilitaetsverdikt"))
            .eric(610_301_106, "<Foo/>", "ERROR ganz am Ende"),
        E::neu("D6 unbekannter rc", "e_ges", 422, Some("rc_kein_plausibilitaetsverdikt"))
            .eric(12_345, "", ""),
        E::neu("D7 negativer rc", "e_ges", 422, Some("rc_kein_plausibilitaetsverdikt"))
            .eric(-1, "<Text>x</Text>", ""),
        // --- Abbruch vor ERiC
        E::neu("E1 Teilrechnung an_gesamt", "e_an", 409, Some("scheibe_nicht_abgabefaehig")),
        E::neu("E2 Teilrechnung ep", "e_ep", 409, Some("scheibe_nicht_abgabefaehig")),
        E::neu("E3 unvollstaendig", "e_vorl", 409, Some("deklaration_unvollstaendig")),
        E::neu("E4 Sperrgrund", "e_sperre", 409, Some("kapital_semantik_offen")),
        E::neu("E5 Abgabe-Gate des Writers", "e_ohnegeb", 422, Some("xml_nicht_baubar"))
            .detail("abgabefaehig=True verlangt pflichtfelder_vollstaendig=True"),
        E::neu("E6 leerer Fall", "e_leer", 422, Some("xml_nicht_baubar")),
        E::neu("E7 Jahr ohne Schema", "e_z26", 422, Some("xml_nicht_baubar")).detail("E10-2026.xsd nicht gefunden"),
        E::neu("E8 unbekannte Scheibe", "e_scheibe_x", 400, None),
        // --- Steuerzeichen kommen nie bis ERiC
        E::neu("F1 NUL im Land", "e_ges", 422, Some("xml_nicht_baubar"))
            .rumpf(json!({"empfaenger_land": "B\u{0}Y"}))
            .detail("Element Ziel enth"),
        E::neu("F2 Steuerzeichen im Land", "e_ges", 422, Some("xml_nicht_baubar"))
            .rumpf(json!({"empfaenger_land": "B\u{1}Y"})),
        // --- Rumpf in falscher Gestalt: erst am Land fehlt Python das `get`
        E::neu("G1 Rumpf ist eine Liste", "e_ges", 500, None).rumpf(json!([1])),
        E::neu("G2 Rumpf ist Text", "e_ges", 500, None).rumpf(json!("text")),
        E::neu("G3 Rumpf ist null", "e_ges", 500, None).rumpf(Value::Null),
        E::neu("G4 Rumpf ist eine Zahl, Teilrechnung zuerst", "e_an", 409, Some("scheibe_nicht_abgabefaehig"))
            .rumpf(json!(5)),
        E::neu("G5 Rumpf ist eine Liste, Abgabe-Gate zuerst", "e_ohnegeb", 500, None).rumpf(json!([1])),
        // --- zweimal hintereinander: der zweite Befund haengt einen zweiten Snapshot an
        ok("H1 rc 0, noch einmal", "e_ges"),
    ];
    v
}

// ---------------------------------------------------------------- Aufbau

/// Baut die Attrappe nach `ziel`.
fn baue_attrappe(ziel: &Path) {
    let quelle = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/eric_attrappe/eric_attrappe.c");
    let st = Command::new("cc")
        .args(["-shared", "-fPIC", "-O0", "-Wall", "-Wextra", "-o"])
        .arg(ziel)
        .arg(&quelle)
        .status()
        .expect("cc fehlt: die Attrappe von libericapi.so laesst sich nicht bauen");
    assert!(st.success(), "cc scheiterte an {}", quelle.display());
}

/// Das Verzeichnis, das beide Server als `ERIC_DIR` sehen, und die Steuerdateien je Seite.
struct Eric {
    dir: PathBuf,
    home: PathBuf,
    attrappe: PathBuf,
    steuer_py: PathBuf,
    steuer_rs: PathBuf,
    /// Was gerade im `ERIC_DIR` liegt: eine geladene Datei wird nie ueberschrieben.
    lib: Cell<Lib>,
}

impl Eric {
    fn neu(wurzel: &Path) -> Option<Self> {
        let dir = wurzel.join("eric");
        let schema = dir.join("schema/2025");
        std::fs::create_dir_all(&schema).unwrap();
        let xsd = elster::finde_schema(2025, "E10-{jahr}.xsd")?;
        std::fs::copy(xsd, schema.join("E10-2025.xsd")).unwrap();
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        let attrappe = wurzel.join("libericapi.so");
        baue_attrappe(&attrappe);
        let home = wurzel.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let (steuer_py, steuer_rs) = (wurzel.join("steuer_py"), wurzel.join("steuer_rs"));
        std::fs::create_dir_all(&steuer_py).unwrap();
        std::fs::create_dir_all(&steuer_rs).unwrap();
        Some(Self {
            dir,
            home,
            attrappe,
            steuer_py,
            steuer_rs,
            lib: Cell::new(Lib::Fehlt),
        })
    }

    fn lib_pfad(&self) -> PathBuf {
        self.dir.join("lib/libericapi.so")
    }

    fn setze_lib(&self, l: Lib) {
        if self.lib.get() == l {
            return;
        }
        self.lib.set(l);
        let p = self.lib_pfad();
        // Erst entfernen, dann neu anlegen: eine geladene Datei behaelt ihren Inode, ein Ueberschreiben
        // liesse den laufenden Server abstuerzen.
        let _ = std::fs::remove_file(&p);
        match l {
            Lib::Fehlt => {}
            Lib::Muell => std::fs::write(&p, b"kein ELF").unwrap(),
            Lib::Gut => {
                std::fs::copy(&self.attrappe, &p).unwrap();
            }
        }
    }

    /// Steuerdateien beider Seiten fuer ein Szenario; leert Zaehler und Aufzeichnung.
    fn richte(&self, e: &E) {
        for s in [&self.steuer_py, &self.steuer_rs] {
            let _ = std::fs::remove_file(s.join("zaehler"));
            let _ = std::fs::remove_dir_all(s.join("gesehen"));
            for (name, wert) in [("init_rc", e.init_rc), ("einstellung_rc", e.einstellung_rc)] {
                match wert {
                    Some(w) => std::fs::write(s.join(name), w.to_string()).unwrap(),
                    None => {
                        let _ = std::fs::remove_file(s.join(name));
                    }
                }
            }
            match &e.skript {
                Some(t) => std::fs::write(s.join("skript"), t).unwrap(),
                None => {
                    let _ = std::fs::remove_file(s.join("skript"));
                }
            }
        }
    }

    fn umgebung(&self, steuer: &Path) -> Vec<(&'static str, String)> {
        vec![
            ("ERIC_DIR", self.dir.to_string_lossy().into_owned()),
            ("ERIC_ATTRAPPE_DIR", steuer.to_string_lossy().into_owned()),
            ("HOME", self.home.to_string_lossy().into_owned()),
            ("ELSTER_HERSTELLER_ID", HERSTELLER_ID.to_owned()),
        ]
    }
}

/// Die Zeile `thread=` einer Aufzeichnung; sie unterscheidet sich je Prozess und gehoert nicht in den
/// Vergleich der beiden Seiten, sondern in die Pruefung, dass jede Seite immer denselben Thread sieht.
fn thread_von(meta: &str) -> Option<String> {
    meta.lines().find(|z| z.starts_with("thread=")).map(str::to_owned)
}

fn ohne_thread(meta: &str) -> String {
    meta.lines().filter(|z| !z.starts_with("thread=")).fold(String::new(), |mut s, z| {
        let _ = writeln!(s, "{z}");
        s
    })
}

/// Was die Attrappe sah: `(meta, xml)` je Aufruf, in der Reihenfolge.
fn gesehen(steuer: &Path) -> Vec<(String, Vec<u8>)> {
    let mut n = 1;
    let mut v = Vec::new();
    while let Ok(xml) = std::fs::read(steuer.join(format!("gesehen/{n}.xml"))) {
        let meta = std::fs::read_to_string(steuer.join(format!("gesehen/{n}.meta"))).unwrap_or_default();
        v.push((meta, xml));
        n += 1;
    }
    v
}

fn pruefe(e: &E, body_py: Option<&Value>, status_rs: u16, py: &[(String, Vec<u8>)], rs: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut d = Vec::new();
    if status_rs != e.status {
        d.push(format!("Status {status_rs}, erwartet {}", e.status));
    }
    let grund = body_py.and_then(|b| b["grund"].as_str());
    if grund != e.grund {
        d.push(format!("grund {grund:?}, erwartet {:?}", e.grund));
    }
    if let Some(t) = e.detail_beginnt {
        let ist = body_py.and_then(|b| b["detail"].as_str()).unwrap_or_default();
        if !ist.starts_with(t) {
            d.push(format!("detail {ist:?} beginnt nicht mit {t:?}"));
        }
    }
    if py.len() != rs.len() {
        d.push(format!("ERiC-Aufrufe py={} rs={}", py.len(), rs.len()));
    }
    if rs.len() != e.aufrufe {
        d.push(format!("ERiC-Aufrufe {}, erwartet {}", rs.len(), e.aufrufe));
    }
    for (i, (p, r)) in py.iter().zip(rs).enumerate() {
        if ohne_thread(&p.0) != ohne_thread(&r.0) {
            d.push(format!("Aufruf {}: Kopf py={:?} rs={:?}", i + 1, p.0, r.0));
        }
        if p.1 != r.1 {
            d.push(format!("Aufruf {}: das XML an ERiC ist verschieden (py {} Bytes, rs {})", i + 1, p.1.len(), r.1.len()));
        }
    }
    // Nur pruefen, nie senden: Datenart, Flags und die drei Parameter.
    for (i, (meta, xml)) in rs.iter().enumerate() {
        for soll in ["datenart=ESt_2025\n", "flags=2\n", "druck=NULL\n", "crypto=NULL\n", "serverantwort=NULL\n"] {
            if !meta.contains(soll) {
                d.push(format!("Aufruf {}: {soll:?} fehlt in {meta:?}", i + 1));
            }
        }
        let text = String::from_utf8_lossy(xml);
        for t in &e.xml_enthaelt {
            if !text.contains(t) {
                d.push(format!("Aufruf {}: {t:?} steht nicht im XML", i + 1));
            }
        }
        if let Some(f) = e.xml_wie {
            let soll = std::fs::read(repo_root().join("rust/fixtures/e2e").join(f)).unwrap();
            if *xml != soll {
                d.push(format!("Aufruf {}: das XML an ERiC ist nicht {f} ({} gegen {} Bytes)", i + 1, xml.len(), soll.len()));
            }
        }
    }
    d
}

/// Der Lauf: dieselben Szenarien an Python und Rust, je mit eigener Attrappen-Steuerung.
#[test]
fn eric_paritaet_einreichen() {
    if skip() {
        return;
    }
    let wurzel = tempfile::tempdir().unwrap();
    let Some(eric) = Eric::neu(wurzel.path()) else {
        assert!(
            std::env::var("TAXGRAPH_OHNE_XSD").as_deref() == Ok("1"),
            "E10-2025.xsd fehlt in der lokalen ERiC-Auslieferung ($ERIC_DIR, ~/02_Software/eric); \
             nur TAXGRAPH_OHNE_XSD=1 erlaubt das Fehlen"
        );
        eprintln!("[eric_attrappe] UEBERSPRUNGEN: E10-2025.xsd fehlt, TAXGRAPH_OHNE_XSD=1");
        return;
    };
    let seed = wurzel.path().join("seed");
    schreibe_seed(&seed);
    for (name, d) in faelle() {
        std::fs::write(seed.join(format!("{name}.json")), serde_json::to_vec(&d).unwrap()).unwrap();
    }
    let (env_py, env_rs) = (eric.umgebung(&eric.steuer_py), eric.umgebung(&eric.steuer_rs));
    let extra_py: Vec<(&str, &str)> = env_py.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let extra_rs: Vec<(&str, &str)> = env_rs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    eric.setze_lib(Lib::Fehlt);
    let mut p = Paar::neu_mit(wurzel.path(), true, true, &seed, &extra_py, &extra_rs);
    let szen = szenarien();
    let mut tabelle: BTreeMap<String, String> = BTreeMap::new();
    // Der Thread, auf dem ERiC lief: je Seite immer derselbe (Pythons Hauptthread, Rusts ERiC-Thread).
    let mut threads: [std::collections::BTreeSet<String>; 2] = Default::default();
    for e in &szen {
        eric.setze_lib(e.lib);
        eric.richte(e);
        let a = Anfrage::neu(&format!("einreichen {}", e.name), "POST", &format!("/fall/{}/einreichen", e.fall))
            .json(&e.rumpf);
        let body_py = p.anfrage(&a, Modus::Voll);
        let status_rs = p.stat.letzter;
        let (g_py, g_rs) = (gesehen(&eric.steuer_py), gesehen(&eric.steuer_rs));
        for (seite, g) in threads.iter_mut().zip([&g_py, &g_rs]) {
            seite.extend(g.iter().filter_map(|(meta, _)| thread_von(meta)));
        }
        let d = pruefe(e, body_py.as_ref(), status_rs, &g_py, &g_rs);
        tabelle.insert(
            e.name.to_owned(),
            format!(
                "{status_rs} {} aufrufe={}",
                body_py.as_ref().and_then(|b| b["grund"].as_str()).unwrap_or("-"),
                g_rs.len()
            ),
        );
        p.stat.abweichungen.extend(d.into_iter().map(|x| format!("{}: {x}", e.name)));
    }
    for (seite, t) in ["Python", "Rust"].iter().zip(&threads) {
        if t.len() != 1 {
            p.stat.abweichungen.push(format!("{seite}: ERiC lief auf {} Threads: {t:?}", t.len()));
        }
    }
    p.zustand_vergleichen("nach den Szenarien");
    p.bericht("extern/einreichen");
    for (n, t) in &tabelle {
        println!("  {n}: {t}");
    }
    println!("EXTERN extern/einreichen: {} Szenarien, Attrappe statt ERiC", szen.len());
    assert!(p.stat.abweichungen.is_empty(), "{:#?}", p.stat.abweichungen);
}
