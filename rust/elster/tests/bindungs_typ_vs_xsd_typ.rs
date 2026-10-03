//! Gate: Bindungs-`typ` gegen den Typ der Kz im ELSTER-Schema. Gegenstueck zu
//! `tests/test_bindungs_typ_vs_xsd_typ.py`; Vault: `decisions/typpruefung-bindung-gegen-schema-
//! bekommt-in-rust-einen-test-kein-build-skript`. Kein Build-Skript: die Bindung liegt als YAML vor
//! und wird zur Laufzeit gelesen, das Schema liegt nicht im Repo.
//!
//! Jede gebundene Kz faellt in GENAU EINEN Zweig (`Zweig`), und die Zweige zusammen sind alle
//! Bindungen mit `elster_kz`:
//!
//! - `Bool`: das Schema MUSS einen Ja-Typ fuehren (`Ja1` / `JaX` / `JaNein12` / `Ja2`).
//! - `Zahl` (cent, int): KEIN Ja-Typ; und JEDER Wert, den die Bindung durchlaesst (`enum_werte`,
//!   sonst der Bereich) und der in der Kz ankommt, steht in der Werteliste UND passt zum Muster.
//!   Ein Wert, den `deklariere` weglaesst, kann das Schema nicht verletzen. Ein Muster wie `.{1,3}`
//!   ist eine Stellenzahl, kein Wertevorrat: es urteilt ueber den Wert, nicht ueber seine Form.
//! - `Enum`: jeder `enum_werte`-Wert steht in der Werteliste des Schemas.
//! - `Wertekodierung`: ein `enum`, dessen Werte Laien-Vokabular sind (Konfession). Geprueft wird,
//!   was `deklariere` daraus schreibt, nicht der Laien-Wert. Die Tabelle `WERTEKODIERUNG` selbst
//!   prueft `wertekodierung_codes_stehen_in_der_werteliste` in `src/deklaration.rs`.
//! - `Datum`, `Text`: der `beispielwert` passt zu Werteliste bzw. Muster der Kz.
//!
//! Die Zweige sind ein `match` ueber [`Feldtyp`] ohne Auffangarm: ein neuer Typ bricht die
//! Uebersetzung. Eine Kz, die nicht im Schema steht, und ein `beispielwert`, der fehlt, wo ein Muster
//! gilt, stehen in einer sichtbaren Liste und machen den Test rot.
//!
//! **Das Schema liegt nur lokal** (`$ERIC_DIR`, sonst `~/02_Software/eric`). Fehlt es, ist der
//! Test rot; nur `TAXGRAPH_OHNE_XSD=1` laesst ihn aus (`elster::testhilfe::schemas_da`), und das
//! steht auf stderr („XML und XSD NICHT geprueft"). Die CI setzt die Variable und prueft damit die
//! Daten NICHT; sie fuehrt nur `pruefer_*` aus, die den Pruefer ohne Schema an der echten Bindung
//! und mit eingesetzten Schemawerten probieren.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Feldtyp, Herkunft, PruefTiefe, Zustand};
use elster::testhilfe::schemas_da;
use elster::{deklariere, finde_schema, kz_meta, BindungIndex, Felder, KzMeta};
use regex::Regex;
use serde_json::{json, Value};
use store::SnapshotFeld;

/// Das Jahr, gegen dessen Schema geprueft wird (wie das Python-Vorbild).
const VZ: i64 = 2025;

/// Ein Bereich, der mehr Werte fasst, bricht den Test statt ihn aufzuhalten: der Zahl-Zweig
/// fuehrt jeden Wert einzeln durch `deklariere`. Heute der groesste: 100 000 (`p35c_*_qm`).
const MAX_BEREICH: i64 = 1_000_000;

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
        bindung::lade_registry(&pfad)
            .expect("Bindung laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

/// Genau ein bestaetigtes Feld, Herkunft laie: die Python-Vorlage `{feld_id: {"wert": w,
/// "zustand": "bestaetigt"}}`.
fn einzeln(feld_id: &str, wert: Value) -> Felder {
    let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
    let herkunft = Herkunft {
        herkunft: a("laie"),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: a("nutzer"),
    };
    Felder::from([(
        feld_id.to_owned(),
        SnapshotFeld {
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: herkunft.into(),
        },
    )])
}

fn als_text(v: &Value) -> String {
    v.as_str().map_or_else(|| v.to_string(), str::to_owned)
}

/// Die sieben Kz des Pflegeblocks (§ 33b Abs. 6 EStG, Gruppe `AgB/Pflege_PB/Einz`): Zwilling von
/// `PFLEGE_KZ` in `src/tabellen.rs` (dort `pub(crate)`). Der Mapper schreibt den Block nur, wenn
/// `rentner_pflegegrad` 2..4 oder Merkzeichen H dasteht (`Bau::pflegeblock`); sonst faellt er
/// als Ganzes weg.
// ponytail: Abschrift der Kz-Menge. Upgrade: `PFLEGE_KZ` aus `elster` exportieren, sobald ein
// zweiter Test sie braucht.
const PFLEGEBLOCK_KZ: [&str; 7] = [
    "E0161606", "E0161808", "E0161607", "E0161506", "E0110601", "E0106507", "E0106603",
];
const PFLEGEGRAD: &str = "rentner_pflegegrad";

/// Was `deklariere` aus genau diesem einen Wert in die Kz schreibt. `None`: nichts, der Mapper
/// laesst den Wert weg. Wie das Python-Vorbild: nur `deklaration`, nicht `person_b`.
///
/// Ein Feld des Pflegeblocks bekommt den Pflegegrad 3 dazu. Allein kommt es nie an (der Block
/// faellt weg), der Pruefer sah 0 Werte — und `rentner_pflege_weitere_personen` (E0106603,
/// Schema `.{0,1}`) lief mit Bereich 0..20 unbemerkt durch.
fn ankunft(b: &Bindung, kz: &str, wert: Value, index: &BindungIndex<'_>) -> Option<String> {
    let mut felder = einzeln(&b.feld_id, wert);
    if b.feld_id != PFLEGEGRAD && PFLEGEBLOCK_KZ.contains(&kz) {
        felder.extend(einzeln(PFLEGEGRAD, json!(3)));
    }
    let d = deklariere(&felder, index, VZ, None).unwrap();
    d.deklaration.get(kz).map(als_text)
}

/// Die Muster einer Kz. Mehrere `pattern` in einem Ableitungsschritt sind im Schema Alternativen
/// (ODER); die ganze Zeichenkette muss passen.
struct Muster(Vec<Regex>);

impl Muster {
    fn neu(muster: &[String]) -> Result<Self, String> {
        muster
            .iter()
            .map(|p| Regex::new(&format!("^(?:{p})$")).map_err(|e| format!("Muster {p:?}: {e}")))
            .collect::<Result<_, _>>()
            .map(Self)
    }

    fn passt(&self, wert: &str) -> bool {
        self.0.iter().any(|re| re.is_match(wert))
    }
}

/// Die Zweige. Die Reihenfolge ist die der Ausgabe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Zweig {
    Bool,
    Zahl,
    Enum,
    Wertekodierung,
    Datum,
    Text,
}

/// Das Urteil ueber eine Bindung. `werte`: wie viele Werte tatsaechlich gegen das Schema gehalten
/// wurden. Ein Pruefer, der 0 Werte sieht, meldet trotzdem „bestanden".
struct Befund {
    zweig: Zweig,
    abweichungen: Vec<String>,
    ungeprueft: Vec<String>,
    werte: usize,
    /// Zahl-Kz, deren Werteliste oder Muster gegen mindestens einen angekommenen Wert stand.
    mit_facette: bool,
    /// Davon: das Muster ist eine Stellenzahl (`.{a,b}`).
    stellenzahl: bool,
}

/// Alle Werte, die die Bindung fuer ein Zahlfeld durchlaesst: `enum_werte`, sonst der Bereich.
fn durchgelassen(b: &Bindung) -> Vec<String> {
    if let Some(w) = &b.enum_werte {
        return w.clone();
    }
    let Some(r) = &b.bereich else {
        return Vec::new();
    };
    assert!(
        r.max - r.min <= MAX_BEREICH,
        "{}: Bereich {}..{} ist zu gross, um jeden Wert zu pruefen",
        b.feld_id,
        r.min,
        r.max
    );
    (r.min..=r.max).map(|w| w.to_string()).collect()
}

/// Ein Zahlwert, wie ihn der Store ablegt: Ganzzahl, wo der Text eine ist.
fn als_wert(w: &str) -> Value {
    w.parse::<i64>().map_or_else(|_| json!(w), |i| json!(i))
}

fn zahl_zweig(b: &Bindung, kz: &str, meta: &KzMeta, index: &BindungIndex<'_>, bef: &mut Befund) {
    let (feld, typ) = (&b.feld_id, b.typ.als_str());
    if meta.is_ja {
        bef.abweichungen.push(format!(
            "{feld}: typ={typ}, Kz {kz}, XSD type={} (Ja-Typ, aber Betrag)",
            meta.type_name
        ));
    }
    let muster = match Muster::neu(&meta.patterns) {
        Ok(m) => m,
        Err(e) => {
            bef.abweichungen
                .push(format!("{feld}: Kz {kz}, Schema-{e} nicht lesbar"));
            return;
        }
    };
    for w in durchgelassen(b) {
        let Some(v) = ankunft(b, kz, als_wert(&w), index) else {
            continue;
        };
        bef.werte += 1;
        if !meta.enums.is_empty() && !meta.enums.contains(&v) {
            bef.abweichungen.push(format!(
                "{feld}: typ={typ}, Kz {kz}, Bindung erlaubt {w}, XSD erlaubt nur {:?} — \
                 deklariert als {v:?}",
                meta.enums
            ));
        }
        if !meta.patterns.is_empty() && !muster.passt(&v) {
            bef.abweichungen.push(format!(
                "{feld}: typ={typ}, Kz {kz}, Bindung erlaubt {w}, XSD pattern={:?} — \
                 deklariert als {v:?}",
                meta.patterns
            ));
        }
    }
    bef.mit_facette = bef.werte > 0 && !(meta.enums.is_empty() && meta.patterns.is_empty());
    let stellen = Regex::new(r"^\.\{\d+,\d+\}$").unwrap();
    bef.stellenzahl = bef.mit_facette
        && !meta.patterns.is_empty()
        && meta.patterns.iter().all(|p| stellen.is_match(p));
}

fn enum_zweig(b: &Bindung, kz: &str, meta: &KzMeta, index: &BindungIndex<'_>, bef: &mut Befund) {
    let feld = &b.feld_id;
    let werte = b.enum_werte.clone().unwrap_or_default();
    if meta.enums.is_empty() {
        bef.abweichungen.push(format!(
            "{feld}: typ=enum, Kz {kz}, XSD keine enumeration — enum_werte {werte:?} koennen \
             nicht gemappt werden"
        ));
        return;
    }
    let geschrieben: Vec<(String, Option<String>)> = werte
        .into_iter()
        .map(|w| {
            let a = ankunft(b, kz, json!(w), index);
            (w, a)
        })
        .collect();
    // Laien-Vokabular: der Mapper schreibt einen anderen Text als den Wert.
    let kodiert = geschrieben
        .iter()
        .any(|(w, a)| a.as_ref().is_some_and(|a| a != w));
    if kodiert {
        bef.zweig = Zweig::Wertekodierung;
    }
    for (w, a) in geschrieben {
        // 1:1-Enum: der Wert selbst; Wertekodierung: der Code (kein Code: nichts zu pruefen).
        let Some(gepruefter) = (if kodiert { a } else { Some(w.clone()) }) else {
            continue;
        };
        bef.werte += 1;
        if !meta.enums.contains(&gepruefter) {
            bef.abweichungen.push(format!(
                "{feld}: typ=enum, Kz {kz}, enum_werte enthalten {w:?} (geschrieben: \
                 {gepruefter:?}), XSD erlaubt nur {:?}",
                meta.enums
            ));
        }
    }
}

/// `datum` und `text`: der `beispielwert` gegen Werteliste bzw. Muster der Kz.
fn beispiel_zweig(b: &Bindung, kz: &str, meta: &KzMeta, bef: &mut Befund) {
    let (feld, typ) = (&b.feld_id, b.typ.als_str());
    let facette = if meta.enums.is_empty() {
        format!("Muster {:?}", meta.patterns)
    } else {
        format!("Enum {:?}", meta.enums)
    };
    if meta.enums.is_empty() && meta.patterns.is_empty() {
        return; // weder Werteliste noch Muster: Freitext
    }
    if b.beispielwert.is_null() {
        bef.ungeprueft.push(format!(
            "{feld} (Kz {kz}): {typ} an {facette} gebunden, aber kein beispielwert"
        ));
        return;
    }
    let beispiel = als_text(&b.beispielwert);
    bef.werte += 1;
    let passt = if meta.enums.is_empty() {
        match Muster::neu(&meta.patterns) {
            Ok(m) => m.passt(&beispiel),
            Err(e) => {
                bef.abweichungen
                    .push(format!("{feld}: Kz {kz}, Schema-{e} nicht lesbar"));
                return;
            }
        }
    } else {
        meta.enums.contains(&beispiel)
    };
    if !passt {
        bef.abweichungen.push(format!(
            "{feld}: typ={typ}, Kz {kz}, XSD {facette}, beispielwert {beispiel:?} passt nicht"
        ));
    }
}

/// Die Pruefung EINER Bindung gegen die Typ-Angaben ihrer Kz. Genau ein Zweig je Bindung.
fn pruefe(b: &Bindung, kz: &str, meta: &KzMeta, index: &BindungIndex<'_>) -> Befund {
    let mut bef = Befund {
        zweig: match b.typ {
            Feldtyp::Bool => Zweig::Bool,
            Feldtyp::Cent | Feldtyp::Int => Zweig::Zahl,
            Feldtyp::Enum => Zweig::Enum,
            Feldtyp::Datum => Zweig::Datum,
            Feldtyp::Text => Zweig::Text,
        },
        abweichungen: Vec::new(),
        ungeprueft: Vec::new(),
        werte: 0,
        mit_facette: false,
        stellenzahl: false,
    };
    match b.typ {
        Feldtyp::Bool => {
            bef.werte += 1;
            if !meta.is_ja {
                bef.abweichungen.push(format!(
                    "{}: typ=bool, Kz {kz}, XSD type={} (kein Ja-Typ)",
                    b.feld_id, meta.type_name
                ));
            }
        }
        Feldtyp::Cent | Feldtyp::Int => zahl_zweig(b, kz, meta, index, &mut bef),
        Feldtyp::Enum => enum_zweig(b, kz, meta, index, &mut bef),
        Feldtyp::Datum | Feldtyp::Text => beispiel_zweig(b, kz, meta, &mut bef),
    }
    bef
}

/// Die Typ-Angaben der Kz aus E10 (und E77 fuer E60xx, wie das Python-Vorbild).
fn schema_meta() -> (HashMap<String, KzMeta>, HashMap<String, KzMeta>) {
    let lade = |datei: &str, wurzel: &str| {
        let pfad = finde_schema(VZ, datei).unwrap_or_else(|| panic!("{datei} fehlt"));
        kz_meta(&pfad, wurzel).unwrap()
    };
    (lade("E10-{jahr}.xsd", "E10"), lade("E77-{jahr}.xsd", "E77"))
}

#[test]
fn jede_gebundene_kz_steht_in_genau_einem_zweig() {
    if !schemas_da(VZ) {
        return;
    }
    let (e10, e77) = schema_meta();
    let mit_kz: Vec<&Bindung> = bindungen()
        .iter()
        .filter(|b| b.elster_kz.is_some())
        .collect();

    let mut zweige: BTreeMap<Zweig, usize> = BTreeMap::new();
    let (mut nicht_im_schema, mut abweichungen, mut ungeprueft) =
        (Vec::new(), Vec::new(), Vec::new());
    let (mut werte, mut mit_facette, mut stellenzahl) = (0, 0, 0);
    for b in &mit_kz {
        let kz = b.elster_kz.as_ref().unwrap().as_str();
        let schema = if kz[1..3] == *"60" { &e77 } else { &e10 };
        let Some(meta) = schema.get(kz) else {
            nicht_im_schema.push(format!("{} (Kz {kz}): Kz nicht im Schema", b.feld_id));
            continue;
        };
        let bef = pruefe(b, kz, meta, index());
        *zweige.entry(bef.zweig).or_default() += 1;
        werte += bef.werte;
        mit_facette += usize::from(bef.mit_facette);
        stellenzahl += usize::from(bef.stellenzahl);
        abweichungen.extend(bef.abweichungen);
        ungeprueft.extend(bef.ungeprueft);
    }

    println!(
        "bindungs_typ_vs_xsd_typ: {} Bindungen mit Kz, Zweige {zweige:?}, {werte} Werte gegen das \
         Schema gehalten, davon Zahl-Kz mit Werteliste oder Muster {mit_facette}, mit \
         Stellenzahl-Muster {stellenzahl}, nicht im Schema {}, ungeprueft {}",
        mit_kz.len(),
        nicht_im_schema.len(),
        ungeprueft.len()
    );
    for u in nicht_im_schema.iter().chain(&ungeprueft) {
        println!("  Nicht pruefbar (sichtbar, kein stiller Skip): {u}");
    }
    assert!(
        mit_kz.len() >= 100,
        "nur {} Bindungen mit Kz: die Bindung wurde nicht gelesen",
        mit_kz.len()
    );
    // KONTROLLE: der Zahl-Zweig hat Werte gegen Werteliste und Muster gehalten (gemessen 12 Kz, davon
    // 4 mit Stellenzahl-Muster; E0106603 ist die zwoelfte und kommt nur mit dem Pflegegrad an, s.
    // `ankunft`), nicht nur Kz ohne Facette durchgewunken.
    assert!(
        mit_facette >= 12 && stellenzahl >= 4,
        "Zahl-Zweig: nur {mit_facette} Kz mit Facette, {stellenzahl} mit Stellenzahl-Muster"
    );
    assert_eq!(
        zweige.values().sum::<usize>() + nicht_im_schema.len(),
        mit_kz.len(),
        "Zweige {zweige:?}: nicht jede Bindung mit Kz steht in genau einem Zweig"
    );
    assert!(
        nicht_im_schema.is_empty() && ungeprueft.is_empty(),
        "nicht pruefbar:\n{}",
        nicht_im_schema
            .iter()
            .chain(&ungeprueft)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        abweichungen.is_empty(),
        "Bindungs-Typ <-> XSD-Typ, {} Abweichungen:\n{}",
        abweichungen.len(),
        abweichungen.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// Der Pruefer ohne Schema: die echte Bindung, eingesetzte Schemawerte. Laeuft in der CI.

fn meta(enums: &[&str], patterns: &[&str]) -> KzMeta {
    let texte = |l: &[&str]| l.iter().map(|s| (*s).to_owned()).collect();
    KzMeta {
        type_name: "Probe".to_owned(),
        enums: texte(enums),
        patterns: texte(patterns),
        is_ja: false,
    }
}

fn eine(feld_id: &str) -> &'static Bindung {
    bindungen().iter().find(|b| b.feld_id == feld_id).unwrap()
}

/// Das Muster von `E0109708` (Grad der Behinderung) wie in `E10-2025.xsd`.
const GDB_MUSTER: &str = "20|25|30|35|40|45|50|55|60|65|70|75|80|85|90|95|100";

#[test]
fn pruefer_meldet_einen_wert_ausserhalb_des_musters() {
    let m = meta(&[], &[GDB_MUSTER]);
    // KONTROLLE: die echte Bindung besteht, und der Pruefer hat Werte gesehen (0 = kein Beweis).
    let gut = pruefe(
        eine("rentner_grad_der_behinderung"),
        "E0109708",
        &m,
        index(),
    );
    assert_eq!(gut.zweig, Zweig::Zahl);
    assert!(gut.abweichungen.is_empty(), "{:?}", gut.abweichungen);
    assert!(gut.werte >= 17, "nur {} Werte gesehen", gut.werte);

    // Ein Wert mehr in `enum_werte`: 33 laeuft durch den Mapper und verletzt das Muster.
    let mut schlecht = eine("rentner_grad_der_behinderung").clone();
    schlecht.enum_werte.as_mut().unwrap().push("33".to_owned());
    let mut idx: HashMap<String, &Bindung> = index().clone();
    idx.insert(schlecht.feld_id.clone(), &schlecht);
    let bef = pruefe(&schlecht, "E0109708", &m, &idx);
    assert_eq!(bef.abweichungen.len(), 1, "{:?}", bef.abweichungen);
    let meldung = &bef.abweichungen[0];
    for teil in ["rentner_grad_der_behinderung", "E0109708", "33", GDB_MUSTER] {
        assert!(meldung.contains(teil), "{teil:?} fehlt in: {meldung}");
    }
}

#[test]
fn pruefer_meldet_kein_stellenzahlmuster_als_widerspruch() {
    // `.{1,3}` gegen einen Bereich bis 366 ist eine Obergrenze, kein Widerspruch.
    let m = meta(&[], &[".{1,3}"]);
    let gut = pruefe(eine("tage_24h"), "E0205409", &m, index());
    assert!(gut.abweichungen.is_empty(), "{:?}", gut.abweichungen);
    assert!(gut.werte >= 300, "nur {} Werte gesehen", gut.werte);

    // Gegenprobe: ein Bereich bis 1000 laesst „1000" durch, vier Stellen, und das ist ein Befund.
    let mut weit = eine("tage_24h").clone();
    weit.bereich.as_mut().unwrap().max = 1000;
    let mut idx: HashMap<String, &Bindung> = index().clone();
    idx.insert(weit.feld_id.clone(), &weit);
    let bef = pruefe(&weit, "E0205409", &m, &idx);
    assert_eq!(bef.abweichungen.len(), 1, "{:?}", bef.abweichungen);
    assert!(
        bef.abweichungen[0].contains("1000"),
        "{:?}",
        bef.abweichungen
    );
}

/// Das Muster von `E0106603` (Anzahl weiterer Pflegepersonen) wie in `E10-2025.xsd:1380` und
/// `E10-2024.xsd:1444`: hoechstens EIN Zeichen, also 0..9. Abschrift, damit der Test in der CI
/// laeuft; `pflegepersonen_muster_steht_im_schema` haelt sie gegen das Schema.
const PFLEGEPERSONEN_MUSTER: &str = ".{0,1}";

#[test]
fn pruefer_meldet_pflegepersonen_ueber_dem_xsd_maximum() {
    let m = meta(&[], &[PFLEGEPERSONEN_MUSTER]);
    let b = eine("rentner_pflege_weitere_personen");
    let r = b.bereich.as_ref().unwrap();
    let bef = pruefe(b, "E0106603", &m, index());
    assert_eq!(bef.zweig, Zweig::Zahl);
    // KONTROLLE: jeder Wert des Bereichs kommt in der Kz an (sonst sah der Pruefer 0 Werte und
    // meldete trotzdem „bestanden") und wurde gegen das Muster gehalten.
    assert_eq!(
        bef.werte,
        usize::try_from(r.max - r.min + 1).unwrap(),
        "nicht jeder Wert des Bereichs {}..{} kam in E0106603 an",
        r.min,
        r.max
    );
    assert!(
        bef.abweichungen.is_empty(),
        "Bereich {}..{} laesst Werte durch, die das Schema (Muster {PFLEGEPERSONEN_MUSTER}) \
         ablehnt:\n{}",
        r.min,
        r.max,
        bef.abweichungen.join("\n")
    );
}

#[test]
fn pflegepersonen_muster_steht_im_schema() {
    if !schemas_da(VZ) {
        return;
    }
    let (e10, _) = schema_meta();
    assert_eq!(
        e10["E0106603"].patterns,
        [PFLEGEPERSONEN_MUSTER],
        "die Abschrift in diesem Test weicht vom Schema ab"
    );
}
