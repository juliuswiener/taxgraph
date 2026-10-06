//! Anker-Waechter (Weg B voll, Stufe 3, W7): jeder `zitatanker` einer Bindung steht im Quelltext des
//! Gesetzes, auf das er zeigt. Ein Anker ist das woertliche Zitat, das sagt, WARUM ein Feld so gebunden
//! ist. Steht er nicht im Text, zeigt die Bindung im Rechtsbeleg auf eine falsche Fundstelle: kein
//! Steuereffekt, aber ein Beleg, der nichts belegt.
//!
//! Gegenstueck zu `tests/test_bindungstabelle.py::test_d_anker_normalize` (Python-Werkzeug
//! `pipeline.gates._normalize`). Die Funktion [`normalisiere`] ist dessen Regel, in Rust nachgebaut:
//! Umlaute zu ae/oe/ue/ss, alles klein, jede Folge von Leerraum zu einem Leerzeichen. Beide Seiten
//! (Anker und Quelltext) laufen durch sie; der Anker muss dann ein Stueck des Quelltexts sein.
//!
//! Die Quelldatei ist `anker_ref.datei`. Fehlt sie oder ist sie leer, ist das ein Fehler. Das ist strenger
//! als das Python-Werkzeug, das dann auf `norm_source` der Regel in `pipeline/produktion/rules.yaml`
//! zurueckfaellt: die Bindung gehoert seit Stufe 1 Rust, und kein Rust-Test soll den Python-Baum lesen.
//! Die Datei muss unter `sources/` liegen (kein `..`, kein Symlink hinaus): ein Anker gegen eine Datei
//! ausserhalb belegt nichts, sondern liest, was gerade dort liegt.
//!
//! Jede Bindung der Registry wird gezaehlt: `verglichen` plus `ohne_quelle` ist die Zahl der Bindungen,
//! unabhaengig gezaehlt aus `Registry::dateien`. Eine Untergrenze allein liesse Bindungen unbemerkt fallen.
//!
//! Was dieser Waechter NICHT sieht: ob der Anker zur Fundstelle (`anker_ref.quelle`) passt, ob die
//! Quelldatei die richtige Fassung des Gesetzes ist und ob der Anker das Feld wirklich traegt. Er prueft nur,
//! dass der Text irgendwo in der Datei steht.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use bindung::Registry;

/// Untergrenzen: ohne sie wuerde der Waechter gruen, weil seine Menge leer ist (heute 368 Anker, 40 Dateien).
const MINDESTENS_ANKER: usize = 300;
const MINDESTENS_DATEIEN: usize = 30;

/// Pythons `\s` fuer `str`: Unicode-Leerraum (Rust: `White_Space`) plus U+001C bis U+001F, die Python als
/// Leerraum zaehlt und Rust nicht.
fn ist_leerraum(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `pipeline.gates._normalize`: Umlaute und ss (auch grosse Umlaute), klein, Leerraum zusammen, Rand weg.
fn normalisiere(text: &str) -> String {
    let mut umgesetzt = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            'ä' | 'Ä' => umgesetzt.push_str("ae"),
            'ö' | 'Ö' => umgesetzt.push_str("oe"),
            'ü' | 'Ü' => umgesetzt.push_str("ue"),
            'ß' => umgesetzt.push_str("ss"),
            _ => umgesetzt.push(c),
        }
    }
    umgesetzt
        .to_lowercase()
        .split(ist_leerraum)
        .filter(|teil| !teil.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Was die Pruefung von einer Bindung braucht.
struct Anker<'a> {
    feld_id: &'a str,
    datei: Option<&'a str>,
    zitatanker: &'a str,
}

/// Die Quelldatei eines Ankers: seine eigene `datei`. Fehlt sie oder ist sie leer, ist das ein Fehler.
fn quelldatei<'a>(anker: &Anker<'a>) -> Result<&'a str, String> {
    anker
        .datei
        .filter(|d| !d.is_empty())
        .ok_or_else(|| format!("{}: anker_ref.datei fehlt oder ist leer", anker.feld_id))
}

/// Nur relative Pfade `sources/...` ohne `..`.
fn pruefe_pfad(datei: &str) -> Result<(), String> {
    let mut teile = Path::new(datei).components();
    if teile.next() != Some(Component::Normal("sources".as_ref())) {
        return Err(format!("{datei} liegt nicht unter sources/"));
    }
    if teile.any(|c| !matches!(c, Component::Normal(_))) {
        return Err(format!("{datei} verlaesst sources/ ueber .. oder einen absoluten Teil"));
    }
    Ok(())
}

/// Der Text der Quelldatei, nachdem Pfad und Symlinks geprueft sind.
fn lies_quelle(wurzel: &Path, datei: &str) -> Result<String, String> {
    pruefe_pfad(datei)?;
    let voll = wurzel.join(datei);
    let echt = voll
        .canonicalize()
        .map_err(|e| format!("Quelldatei {datei} nicht lesbar: {e}"))?;
    let erlaubt = wurzel
        .join("sources")
        .canonicalize()
        .map_err(|e| format!("{}/sources nicht lesbar: {e}", wurzel.display()))?;
    if !echt.starts_with(&erlaubt) {
        return Err(format!("{datei} zeigt ueber einen Symlink aus sources/ hinaus"));
    }
    std::fs::read_to_string(&echt).map_err(|e| format!("Quelldatei {datei} nicht lesbar: {e}"))
}

struct Befund {
    fehler: Vec<String>,
    /// Anker, deren Quelltext gelesen und mit dem Anker verglichen wurde (auch bei Fehlschlag).
    verglichen: usize,
    /// Anker, bei denen es keinen Quelltext zum Vergleichen gab (keine Datei, nicht lesbar, ausserhalb).
    /// `verglichen + ohne_quelle` ist die Zahl der Anker, die `pruefe` gesehen hat.
    ohne_quelle: usize,
    dateien: BTreeSet<String>,
}

fn pruefe(wurzel: &Path, anker: &[Anker]) -> Befund {
    let mut befund = Befund {
        fehler: Vec::new(),
        verglichen: 0,
        ohne_quelle: 0,
        dateien: BTreeSet::new(),
    };
    let mut quellen: BTreeMap<&str, Result<String, String>> = BTreeMap::new();
    for a in anker {
        let datei = match quelldatei(a) {
            Ok(d) => d,
            Err(e) => {
                befund.fehler.push(e);
                befund.ohne_quelle += 1;
                continue;
            }
        };
        let quelle = quellen
            .entry(datei)
            .or_insert_with(|| lies_quelle(wurzel, datei).map(|t| normalisiere(&t)));
        let norm_quelle = match quelle {
            Ok(q) => q,
            Err(e) => {
                befund.fehler.push(format!("{}: {e}", a.feld_id));
                befund.ohne_quelle += 1;
                continue;
            }
        };
        befund.verglichen += 1;
        befund.dateien.insert(datei.to_owned());
        let norm_anker = normalisiere(a.zitatanker);
        // Ein Anker, der nach dem Normalisieren leer ist, stuende in jedem Text.
        if norm_anker.is_empty() {
            befund.fehler.push(format!("{}: Zitatanker ist nur Leerraum", a.feld_id));
        } else if !norm_quelle.contains(&norm_anker) {
            let vorn: String = a.zitatanker.chars().take(60).collect();
            befund
                .fehler
                .push(format!("{}: Zitatanker nicht in {datei}: '{vorn}'", a.feld_id));
        }
    }
    befund
}

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Die Registry, die der Dienst liest.
fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| bindung::lade_registry_der_wurzel(&wurzel()).expect("Registry laedt"))
}

/// Die Pruefung auf der Registry, die der Dienst liest.
fn echter_befund() -> &'static Befund {
    static CELL: OnceLock<Befund> = OnceLock::new();
    CELL.get_or_init(|| {
        let anker: Vec<Anker> = registry()
            .dateien
            .iter()
            .flat_map(|(_, d)| d.bindungen.iter())
            .map(|b| Anker {
                feld_id: &b.feld_id,
                datei: b.anker_ref.datei.as_deref(),
                zitatanker: &b.anker_ref.zitatanker,
            })
            .collect();
        pruefe(&wurzel(), &anker)
    })
}

#[test]
fn jeder_zitatanker_steht_im_quelltext_seiner_datei() {
    let befund = echter_befund();
    assert!(
        befund.fehler.is_empty(),
        "{} Anker ohne Beleg im Quelltext:\n{}",
        befund.fehler.len(),
        befund.fehler.join("\n")
    );
}

/// Gegenprobe gegen eine leere Pruefung: ein Waechter ohne Anker waere gruen.
#[test]
fn es_gibt_anker_und_quelldateien_zu_pruefen() {
    let befund = echter_befund();
    assert!(
        befund.verglichen >= MINDESTENS_ANKER,
        "nur {} Anker verglichen",
        befund.verglichen
    );
    assert!(
        befund.dateien.len() >= MINDESTENS_DATEIEN,
        "nur {} Quelldateien gelesen",
        befund.dateien.len()
    );
    assert!(
        befund.dateien.iter().all(|d| d.starts_with("sources/")),
        "Quelldatei ausserhalb von sources/: {:?}",
        befund.dateien
    );
}

/// Jede Bindung der Registry wird geprueft, keine faellt still heraus. Die Zahl kommt auf einem anderen Weg
/// als die Anker von `echter_befund` (Summe der Laengen je Datei, keine Iteratorkette): ein Filter oder ein
/// `skip` in der Kette liesse eine Untergrenze wie `MINDESTENS_ANKER` gruen, diese Gleichung nicht.
#[test]
fn jede_bindung_der_registry_wird_geprueft() {
    let bindungen: usize = registry().dateien.iter().map(|(_, d)| d.bindungen.len()).sum();
    let befund = echter_befund();
    assert!(bindungen >= MINDESTENS_ANKER, "nur {bindungen} Bindungen in der Registry");
    assert_eq!(
        befund.verglichen + befund.ohne_quelle,
        bindungen,
        "{} verglichen + {} ohne Quelltext, aber {bindungen} Bindungen in der Registry",
        befund.verglichen,
        befund.ohne_quelle
    );
}

#[test]
fn normalisiere_wie_das_python_werkzeug() {
    // Erwartungen aus pipeline.gates._normalize (gemessen 2026-10-06), nicht aus dieser Datei abgeleitet.
    let faelle: &[(&str, &str)] = &[
        ("Äpfel Öl Übung ß", "aepfel oel uebung ss"),
        ("  Fünf \t\n Straßen  ", "fuenf strassen"),
        ("STRASSE", "strasse"),
        ("§ 9 Abs. 1 Satz 3 Nr. 4", "§ 9 abs. 1 satz 3 nr. 4"),
        // Grosses Eszett wird klein zu ß, nicht zu ss (die Tabelle kennt nur das kleine).
        ("\u{1e9e}", "\u{df}"),
        ("ΟΔΥΣΣΕΥΣ", "οδυσσευς"),
        (" \t ", ""),
        ("", ""),
    ];
    for (eingabe, soll) in faelle {
        assert_eq!(normalisiere(eingabe), *soll, "{eingabe:?}");
    }
}

#[test]
fn leerraum_ist_wie_pythons_s() {
    // Zusammengefasst, wie in Python: geschuetztes Leerzeichen, Gedankenraum, ideographisch, NEL, C0-Trenner.
    for c in ['\u{a0}', '\u{2003}', '\u{3000}', '\u{85}', '\u{1c}', '\u{1f}', '\u{b}', '\u{c}'] {
        assert_eq!(normalisiere(&format!("a{c}{c}b")), "a b", "U+{:04X}", u32::from(c));
    }
    // Nicht zusammengefasst: Nullbreiten-Leerzeichen ist in Python kein Leerraum.
    assert_eq!(normalisiere("a\u{200b}b"), "a\u{200b}b");
}

#[test]
fn pfad_nur_unter_sources_und_ohne_ausbruch() {
    for gut in [
        "sources/gesetze-im-internet/estg_p9_2026-07-10.txt",
        "sources/dba/x.txt",
        "sources/./a.txt",
    ] {
        assert!(pruefe_pfad(gut).is_ok(), "{gut}");
    }
    for schlecht in [
        "rules/estg/p9/p9.catala_en",
        "../sources/a.txt",
        "sources/../rules/a.txt",
        "sources/a/../../b.txt",
        "/sources/a.txt",
        "/etc/passwd",
        "Sources/a.txt",
        "sourcesx/a.txt",
        "",
    ] {
        assert!(pruefe_pfad(schlecht).is_err(), "{schlecht:?}");
    }
}

#[test]
fn quelldatei_ist_die_eigene_sonst_ein_fehler() {
    let anker = |datei: Option<&'static str>| Anker {
        feld_id: "f",
        datei,
        zitatanker: "z",
    };
    assert_eq!(quelldatei(&anker(Some("sources/eigene.txt"))), Ok("sources/eigene.txt"));
    // Keine Datei und eine leere Datei sind dasselbe: ein Fehler, der das Feld nennt.
    for ohne in [None, Some("")] {
        let fehler = quelldatei(&anker(ohne)).unwrap_err();
        assert!(fehler.starts_with("f: ") && fehler.contains("datei fehlt"), "{ohne:?}: {fehler}");
    }
}

fn hermetische_wurzel(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("anker-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("sources")).unwrap();
    std::fs::write(
        d.join("sources/gesetz.txt"),
        "Die Aufwendungen für Kinder-\nbetreuung sind  abziehbar.\n§ 10 Abs. 1 Nr. 5 Größe",
    )
    .unwrap();
    std::fs::write(d.join("geheim.txt"), "Nur ausserhalb von sources").unwrap();
    d
}

#[test]
fn die_pruefung_erkennt_ihre_fehlerfaelle() {
    let wurzel = hermetische_wurzel("fehlerfaelle");
    let anker = |zitat: &'static str, datei: Option<&'static str>| Anker {
        feld_id: "f",
        datei,
        zitatanker: zitat,
    };
    let befund = |a: &Anker| pruefe(&wurzel, std::slice::from_ref(a));
    let gesetz = Some("sources/gesetz.txt");
    // Gefunden: Gross-/Kleinschreibung, Umlaute, Leerraum und Zeilenumbruch sind egal.
    for gut in [
        "aufwendungen FUER kinder-\n betreuung",
        "§ 10  Abs. 1 Nr. 5 Groesse",
        "ABZIEHBAR.",
    ] {
        let b = befund(&anker(gut, gesetz));
        assert!(b.fehler.is_empty(), "{gut}: {:?}", b.fehler);
        assert_eq!(b.verglichen, 1);
    }
    // Nicht gefunden: ein Wort ersetzt, ein Zeichen zu viel.
    for schlecht in ["Aufwendungen für Pflege", "abziehbar!", "dieser text steht nirgends zzzq"] {
        let b = befund(&anker(schlecht, gesetz));
        assert_eq!(b.fehler.len(), 1, "{schlecht}");
        assert!(b.fehler[0].contains("nicht in"), "{:?}", b.fehler);
    }
    // Nur Leerraum stuende in jedem Text.
    let b = befund(&anker(" \t\n ", gesetz));
    assert_eq!(b.fehler.len(), 1);
    assert!(b.fehler[0].contains("nur Leerraum"), "{:?}", b.fehler);
    // Datei fehlt, liegt ausserhalb von sources/ (obwohl sie existiert), keine Datei genannt.
    for (datei, teil) in [
        (Some("sources/gibt_es_nicht.txt"), "nicht lesbar"),
        (Some("../geheim.txt"), "liegt nicht unter sources/"),
        (Some("geheim.txt"), "liegt nicht unter sources/"),
        (Some("sources/../geheim.txt"), "verlaesst sources/"),
        (None, "datei fehlt"),
        (Some(""), "datei fehlt"),
    ] {
        let b = befund(&anker("ausserhalb", datei));
        assert_eq!(b.fehler.len(), 1, "{datei:?}: {:?}", b.fehler);
        assert!(b.fehler[0].contains(teil), "{datei:?}: {:?}", b.fehler);
        assert_eq!((b.verglichen, b.ohne_quelle), (0, 1), "{datei:?}");
    }
}

/// Die Bindung gehoert Rust: ein Anker ohne `datei` (oder mit leerer) ist ein Fehler, kein Rueckfall auf
/// eine Regel im Python-Baum. Der Anker steht im Quelltext (der erste der drei besteht); ein Fehler der
/// beiden anderen muss die fehlende Datei nennen, sonst haette etwas anderes die Datei ersetzt.
#[test]
fn anker_ohne_datei_ist_ein_fehler() {
    let wurzel = hermetische_wurzel("ohne_datei");
    let zitat = "Die Aufwendungen für Kinder-\nbetreuung";
    let mit = |datei| Anker {
        feld_id: "mit",
        datei,
        zitatanker: zitat,
    };
    let anker = [
        mit(Some("sources/gesetz.txt")),
        Anker {
            feld_id: "keine",
            ..mit(None)
        },
        Anker {
            feld_id: "leer",
            ..mit(Some(""))
        },
    ];
    let b = pruefe(&wurzel, &anker);
    assert_eq!(b.fehler.len(), 2, "{:?}", b.fehler);
    // Der Fehler nennt die fehlende Datei, nicht einen Lesefehler einer Ersatzdatei.
    for (fehler, feld) in b.fehler.iter().zip(["keine", "leer"]) {
        assert!(
            fehler.starts_with(&format!("{feld}: ")) && fehler.contains("datei fehlt"),
            "{fehler}"
        );
    }
    // Keiner der drei faellt aus der Zaehlung: einer verglichen, zwei ohne Quelltext.
    assert_eq!((b.verglichen, b.ohne_quelle), (1, 2));
}

#[cfg(unix)]
#[test]
fn ein_symlink_aus_sources_hinaus_wird_abgewiesen() {
    let wurzel = hermetische_wurzel("symlink");
    std::os::unix::fs::symlink(wurzel.join("geheim.txt"), wurzel.join("sources/hinaus.txt")).unwrap();
    let anker = Anker {
        feld_id: "f",
        datei: Some("sources/hinaus.txt"),
        zitatanker: "Nur ausserhalb von sources",
    };
    let b = pruefe(&wurzel, std::slice::from_ref(&anker));
    assert_eq!(b.fehler.len(), 1, "{:?}", b.fehler);
    assert!(b.fehler[0].contains("Symlink"), "{:?}", b.fehler);
}
