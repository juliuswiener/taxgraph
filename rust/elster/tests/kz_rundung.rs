//! Rundungsklasse der Betrags-Kz (Abnahme-Folgeauftrag 1, Luecke QX1/QX1b/N4): wie `kz_format` einen
//! Cent-Betrag in ein Kz schreibt — aufrunden (Abzug), abrunden (Einnahme) oder exakt als `"N,NN"`.
//!
//! Warum es diese Datei gibt: Faellt ein Kz aus `ABZUGS_KZ` (Mutant `QX1b`: `E2000401`; QX1: `E0703838`;
//! N4: `E0505607`), wird der Abzug abgerundet statt aufgerundet, und der Steuerpflichtige verliert bis zu
//! 99 Cent je Zeile — bei Summe und Posten sogar eine Erklaerung, die ERiC ablehnt (`rc=610001002`).
//! Gemessen an `52a4dd16`: in allen drei Faellen wurde nur die eingefrorene Python-Fixture
//! (`tabellen_gleich_fixture`, `einreichung_e2e`) rot, kein Rust-eigener Test.
//!
//! **Das Schema entscheidet die Klasse nicht.** Gemessen (M, `E10-2025.xsd`): alle 1762 Typen enden auf
//! `_CType_RABE`; `E0705701` (Summe der Werbungskosten, Abzug) und `E0200201` (Bruttoarbeitslohn,
//! Einnahme) tragen denselben Typ `GanzzahlOhneFuehrNull_MaxVK12_Muster1442570515_CType_RABE`. Auch die
//! Schema-Umgebung traegt keine Richtung: Geschwister im selben Kontext tragen sie nicht (4 von 13
//! Kontexten mit mehr als einem gebundenen Betrags-Kz sind gemischt), und die Summe-/Posten-Regeln stehen
//! nur in der Jahresdokumentation (`.ods`), nicht im Schema. Deshalb zwei Teile:
//!
//! - **Sperre** (`jedes_betrags_kz_hat_eine_bewusste_rundungsklasse`, hermetisch): `kz_rundung.tsv` haelt
//!   je Kz die Klasse fest. Eine Zeile aendern, ergaenzen oder streichen ist ein Entscheid mit Grund (im
//!   Commit). Die Menge der Zeilen ist die Registry-Menge (jedes Kz eines `cent`-Felds) plus `ABZUGS_KZ`
//!   plus `KOMMA_OHNE_E60_KZ`; ein neues Betragsfeld mit Kz ohne Zeile ist rot.
//! - **Schema** (`rundungsklasse_passt_zum_schema_typ`, braucht `E10-2025.xsd`/`E77-2025.xsd`): aus dem
//!   Schema ableitbar sind `komma <=> Dezimalzahl` und `auf/ab <=> Ganzzahl`; dazu steht der Wortlaut des
//!   Schemas neben jeder Zeile und wird verglichen, damit eine Zeile nicht erfunden sein kann. Ohne Schema
//!   rot, ausser `TAXGRAPH_OHNE_XSD=1` (`elster::testhilfe::schemas_da`).
//!
//! Grenze (L): Die Sperre schuetzt vor dem versehentlichen Wechsel, nicht vor einer urspruenglich falschen
//! Wahl zwischen auf und ab; die Wahl belegt die Anleitung ESt 1 A 2025 (`anl_est1a_2025.txt:269-274`) und
//! der Wortlaut in der dritten Spalte, den eine Person lesen muss. Kz, die nur eine Transformation oder ein
//! Aggregat schreibt und die weder ein `cent`-Feld noch `ABZUGS_KZ` noch `KOMMA_OHNE_E60_KZ` nennt, stehen
//! nicht in der Tabelle (sie runden ab, der Standard); ein Aggregat-Kz, das aufrunden muesste und fehlt,
//! faengt diese Datei nicht.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Feldtyp, Kz};
use elster::testhilfe::schemas_da;
use elster::{finde_schema, kz_format, kz_meta, KzFormat, ABZUGS_KZ, KOMMA_OHNE_E60_KZ};

/// Das Jahr, gegen dessen Schema geprueft wird (wie `bindungs_typ_vs_xsd_typ.rs`, `kz_sperre.rs`).
const VZ: i64 = 2025;

/// Wie viele Zeichen des Schema-Wortlauts in der dritten Spalte stehen.
const WORTLAUT_ZEICHEN: usize = 70;

/// Die Sperre, neben dem Test.
const TABELLE: &str = include_str!("kz_rundung.tsv");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Klasse {
    Auf,
    Ab,
    Komma,
}

impl Klasse {
    fn aus_text(s: &str) -> Option<Self> {
        match s {
            "auf" => Some(Self::Auf),
            "ab" => Some(Self::Ab),
            "komma" => Some(Self::Komma),
            _ => None,
        }
    }

    fn von_format(f: KzFormat) -> Self {
        match f {
            KzFormat::EuroAufgerundet => Self::Auf,
            KzFormat::EuroAbgerundet => Self::Ab,
            KzFormat::KommaCent => Self::Komma,
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::Auf => "auf",
            Self::Ab => "ab",
            Self::Komma => "komma",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Zeile {
    klasse: Klasse,
    wortlaut: String,
}

/// `Kz <Tab> klasse <Tab> Wortlaut`; `#`-Zeilen und Leerzeilen zaehlen nicht. Ein Fehler der Datei (falsche
/// Spaltenzahl, ungueltiges Kz, unbekannte Klasse, doppeltes Kz, leerer Wortlaut) ist kein Befund der Bindung.
fn lies_tabelle(text: &str) -> BTreeMap<String, Zeile> {
    let mut m = BTreeMap::new();
    for (i, zeile) in text.lines().enumerate() {
        if zeile.is_empty() || zeile.starts_with('#') {
            continue;
        }
        let s: Vec<&str> = zeile.split('\t').collect();
        assert!(
            s.len() == 3,
            "kz_rundung.tsv Zeile {}: drei Spalten (Kz, klasse, Wortlaut) erwartet, gefunden {}",
            i + 1,
            s.len()
        );
        assert!(
            Kz::ist_gueltig(s[0]),
            "kz_rundung.tsv Zeile {}: {:?} ist kein Kz",
            i + 1,
            s[0]
        );
        let klasse = Klasse::aus_text(s[1]).unwrap_or_else(|| {
            panic!(
                "kz_rundung.tsv Zeile {}: klasse {:?} ist weder auf, ab noch komma",
                i + 1,
                s[1]
            )
        });
        assert!(
            !s[2].trim().is_empty(),
            "kz_rundung.tsv Zeile {}: Wortlaut fehlt",
            i + 1
        );
        let alt = m.insert(
            s[0].to_owned(),
            Zeile {
                klasse,
                wortlaut: s[2].to_owned(),
            },
        );
        assert!(
            alt.is_none(),
            "kz_rundung.tsv Zeile {}: {} steht doppelt",
            i + 1,
            s[0]
        );
    }
    m
}

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        // Die Bindung des Dienstes (`rust/bindung/daten`), nie ein fester Pfad im Test.
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Bindung laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

/// Die Menge, die die Tabelle tragen muss: jedes Kz eines `cent`-Felds der Registry, `ABZUGS_KZ`,
/// `KOMMA_OHNE_E60_KZ`.
fn soll_menge() -> BTreeSet<String> {
    let mut s: BTreeSet<String> = bindungen()
        .iter()
        .filter(|b| b.typ == Feldtyp::Cent)
        .filter_map(|b| b.elster_kz.as_ref().map(|k| k.as_str().to_owned()))
        .collect();
    s.extend(ABZUGS_KZ.iter().map(|k| (*k).to_owned()));
    s.extend(KOMMA_OHNE_E60_KZ.iter().map(|k| (*k).to_owned()));
    s
}

fn klasse_im_code(kz: &str) -> Klasse {
    Klasse::von_format(kz_format(&Kz::new(kz).unwrap()))
}

/// Die Befunde der Sperre als Textzeilen, leer wenn Tabelle, Registry und Code zusammenpassen.
fn befunde_der_sperre(tabelle: &BTreeMap<String, Zeile>, soll: &BTreeSet<String>) -> Vec<String> {
    let mut b = Vec::new();
    let fehlt: Vec<&String> = soll.iter().filter(|k| !tabelle.contains_key(*k)).collect();
    if !fehlt.is_empty() {
        b.push(format!(
            "Betrags-Kz ohne Zeile in kz_rundung.tsv (neues cent-Feld mit Kz, oder ein Kz neu in ABZUGS_KZ/KOMMA_OHNE_E60_KZ; \
             die Klasse auf, ab oder komma ist ein Entscheid mit Grund): {fehlt:?}"
        ));
    }
    let zuviel: Vec<&String> = tabelle.keys().filter(|k| !soll.contains(*k)).collect();
    if !zuviel.is_empty() {
        b.push(format!(
            "Zeilen in kz_rundung.tsv, die weder ein cent-Feld der Registry noch ABZUGS_KZ noch KOMMA_OHNE_E60_KZ nennt \
             (das Feld oder der Eintrag ist verschwunden): {zuviel:?}"
        ));
    }
    for (kz, z) in tabelle {
        let im_code = klasse_im_code(kz);
        if im_code != z.klasse {
            b.push(format!(
                "{kz}: die Tabelle sagt {}, kz_format sagt {} (Kz aus ABZUGS_KZ gefallen oder dazugekommen, oder \
                 KOMMA_OHNE_E60_KZ geaendert): {}",
                z.klasse.text(),
                im_code.text(),
                z.wortlaut
            ));
        }
    }
    b
}

#[test]
fn jedes_betrags_kz_hat_eine_bewusste_rundungsklasse() {
    let tabelle = lies_tabelle(TABELLE);
    let soll = soll_menge();
    // Positivkontrolle: eine leere Tabelle gegen eine leere Menge waere gruen.
    assert!(
        soll.len() >= 80,
        "die Soll-Menge ist verdaechtig klein: {}",
        soll.len()
    );
    assert!(
        tabelle.values().filter(|z| z.klasse == Klasse::Auf).count() >= 50
            && tabelle
                .values()
                .filter(|z| z.klasse == Klasse::Komma)
                .count()
                >= 6,
        "die Tabelle traegt zu wenige auf-/komma-Zeilen"
    );
    let befunde = befunde_der_sperre(&tabelle, &soll);
    assert!(befunde.is_empty(), "{}", befunde.join("\n"));
}

/// Die Sperre erkennt ihre eigenen Fehlerfaelle (sonst waere sie gruen, weil sie nichts sieht).
#[test]
fn die_sperre_erkennt_ihre_eigenen_fehlerfaelle() {
    let tabelle = lies_tabelle(TABELLE);
    let soll = soll_menge();
    assert_eq!(befunde_der_sperre(&tabelle, &soll).len(), 0);

    // 1. Ein Abzugs-Kz kippt nach ab (Tabelle sagt auf, der Code sagte ab): erkennt der Vergleich.
    let mut t = tabelle.clone();
    t.get_mut("E2000401").unwrap().klasse = Klasse::Ab;
    assert!(befunde_der_sperre(&t, &soll)
        .iter()
        .any(|b| b.starts_with("E2000401:")));

    // 2. Ein Kz faellt aus der Tabelle: fehlende Zeile.
    let mut t = tabelle.clone();
    t.remove("E0505607");
    assert!(befunde_der_sperre(&t, &soll)
        .iter()
        .any(|b| b.contains("ohne Zeile") && b.contains("E0505607")));

    // 3. Eine Zeile ohne Gegenstueck (Feld entfernt): ueberzaehlige Zeile.
    let mut t = tabelle.clone();
    t.insert(
        "E0999999".to_owned(),
        Zeile {
            klasse: Klasse::Ab,
            wortlaut: "erfunden".to_owned(),
        },
    );
    assert!(befunde_der_sperre(&t, &soll)
        .iter()
        .any(|b| b.contains("E0999999")));

    // 4. Ein Einnahme-Kz (Bruttoarbeitslohn) steht in der Tabelle als auf, der Code rundet ab: erkennt der Vergleich.
    let mut t = tabelle.clone();
    t.get_mut("E0200201")
        .expect("E0200201 (Bruttoarbeitslohn) steht in der Tabelle")
        .klasse = Klasse::Auf;
    assert!(befunde_der_sperre(&t, &soll)
        .iter()
        .any(|b| b.starts_with("E0200201:")));
}

// ---------------------------------------------------------------- gegen das Schema

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Der Wortlaut (`xs:annotation/xs:documentation`) je Kz-Element des Schemas, Leerraum zusammengezogen.
/// Ein Kz kann in mehreren Kontexten stehen; alle Wortlaute zaehlen.
fn wortlaute(pfad: &Path) -> HashMap<String, Vec<String>> {
    let text = std::fs::read_to_string(pfad).unwrap_or_else(|e| panic!("{}: {e}", pfad.display()));
    let doc = roxmltree::Document::parse(&text).unwrap();
    let mut m: HashMap<String, Vec<String>> = HashMap::new();
    for n in doc.descendants().filter(roxmltree::Node::is_element) {
        if n.tag_name().name() != "element" {
            continue;
        }
        let Some(name) = n.attribute("name").filter(|s| Kz::ist_gueltig(s)) else {
            continue;
        };
        let mut w = String::new();
        for a in n.children().filter(|c| c.tag_name().name() == "annotation") {
            if let Some(d) = a
                .children()
                .find(|c| c.tag_name().name() == "documentation")
            {
                w = norm(d.text().unwrap_or(""));
            }
        }
        m.entry(name.to_owned()).or_default().push(w);
    }
    m
}

fn datenart(kz: &str) -> (&'static str, &'static str) {
    // Wie `xsd::datenart`: `E60...` (Anlage EUeR) steht in E77, alles andere in E10.
    if kz.get(1..3) == Some("60") {
        ("E77-{jahr}.xsd", "E77")
    } else {
        ("E10-{jahr}.xsd", "E10")
    }
}

/// Was der Test je Datenart (E10, E77) aus dem Schema liest: Typ je Kz und Wortlaut je Kz.
type SchemaAuszug = (
    HashMap<String, elster::KzMeta>,
    HashMap<String, Vec<String>>,
);

#[test]
fn rundungsklasse_passt_zum_schema_typ() {
    if !schemas_da(VZ) {
        return;
    }
    let tabelle = lies_tabelle(TABELLE);
    let mut je_datenart: HashMap<&str, SchemaAuszug> = HashMap::new();
    let mut geprueft = 0usize;
    let mut befunde = Vec::new();
    for (kz, z) in &tabelle {
        let (muster, start) = datenart(kz);
        if !je_datenart.contains_key(start) {
            let pfad = finde_schema(VZ, muster).unwrap_or_else(|| panic!("{muster} fehlt"));
            je_datenart.insert(start, (kz_meta(&pfad, start).unwrap(), wortlaute(&pfad)));
        }
        let (meta, worte) = &je_datenart[start];
        let Some(m) = meta.get(kz) else {
            befunde.push(format!("{kz}: nicht im Schema {muster} (Jahr {VZ})"));
            continue;
        };
        let dezimal = m.type_name.starts_with("Dezimalzahl");
        let ganz = m.type_name.starts_with("Ganzzahl");
        match z.klasse {
            Klasse::Komma if !dezimal => befunde.push(format!(
                "{kz}: Klasse komma, aber der Schema-Typ {} ist keine Dezimalzahl",
                m.type_name
            )),
            Klasse::Auf | Klasse::Ab if !ganz => befunde.push(format!(
                "{kz}: Klasse {}, aber der Schema-Typ {} ist keine Ganzzahl (eine Dezimalzahl gehoert nach komma)",
                z.klasse.text(),
                m.type_name
            )),
            _ => {}
        }
        let kurz = |s: &str| s.chars().take(WORTLAUT_ZEICHEN).collect::<String>();
        let passt = worte
            .get(kz)
            .is_some_and(|ws| ws.iter().any(|w| kurz(w) == z.wortlaut));
        if !passt {
            befunde.push(format!(
                "{kz}: der Wortlaut der Tabelle {:?} steht nicht im Schema (dort: {:?})",
                z.wortlaut,
                worte
                    .get(kz)
                    .map(|ws| ws.iter().map(|w| kurz(w)).collect::<Vec<_>>())
            ));
        }
        geprueft += 1;
    }
    assert!(befunde.is_empty(), "{}", befunde.join("\n"));
    // Positivkontrolle: die Schleife hat die Zeilen wirklich gegen das Schema gelesen.
    assert!(
        geprueft >= 80 && geprueft == tabelle.len(),
        "gegen das Schema gepruefte Zeilen: {geprueft}"
    );
}
