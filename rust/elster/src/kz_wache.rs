//! Wache gegen Kz-Literale, die keine Fixture liest.
//!
//! Das Regal (`regal.rs`) deckt Tabellen und Zonen ab, die jemand dort eingetragen hat. Eine Kz,
//! die jemand in einen Funktionskoerper schreibt (eine zweite Kopie, eine neue Bedingung), sieht es
//! nicht — so entstand Nr 59: `abgabe_pruefen` fragte die IBAN-Ausland-Kz mit einer eigenen Kopie
//! des Literals ab, die keine Fixture las (Vault `backlog/taxgraph/kz-tabellen-rest`). Diese Wache
//! dreht die Richtung um:
//! sie sucht jedes exakte Kz-Literal (`"E"` + 7 Ziffern) im Produktionstext von `rust/elster/src`
//! und verlangt fuer jedes einen von drei Gruenden:
//!
//! 1. **Gelesen:** `aus_regal()` hat genau dieses Literal aus dem Quelltext gelesen und gegen die
//!    Fixture verglichen (`regal::aufzeichnen`, `wortliche`, `einzel`).
//! 2. **Tabelle:** es steht in einem Top-Level-Eintrag von `tabellen.rs`, den das Regal einer
//!    Fixture-Stelle zuordnet (nicht als `Ausnahme`); `tabellen_gleich_fixture` vergleicht dessen Wert.
//! 3. **Verhalten:** es steht in einer Funktion mit `Zuordnung::Verhalten`; das Regal nennt ihre
//!    Kz-Literale vollstaendig und die Tests, die bei jedem davon rot werden.
//!
//! Alles andere ist ein Fehler. Der Test braucht kein Netz und kein Python: die Quelltexte kommen
//! aus `include_str!` (Regal) und, fuer Dateien, die das Regal nicht kennt, vom Verzeichnis
//! `src/` selbst — eine neue Datei entgeht der Wache damit nicht.
//!
//! Die Abtastung ist ein Wortzerleger, kein Parser: Zeichenketten, Raw-Strings, Zeichenliterale,
//! Lebensdauern, Kommentare, `#[cfg(test)]`-Items und die Klammertiefe genuegen, um zu sagen, in
//! welchem obersten `fn`/`const`/`static` ein Literal steht. Eine Form, die sie nicht kennt
//! (`cfg(not(test))`, `#[cfg(test)]` vor etwas anderem als einem Item), bricht laut ab.
//!
//! ponytail: nur `rust/elster/src` und nur exakte Literale. Eine Kz, die zur Laufzeit entsteht
//! (`format!("E{…}")`), eine in einer Datendatei und jede in einer anderen Crate sieht die Wache
//! nicht. Upgrade: dieselbe Abtastung ueber `rust/*/src` (Dateien ausserhalb des Regals haben keine
//! Zone: jedes Literal dort braeuchte eine eigene Zuordnung).
//!
//! ponytail: in `tabellen.rs` gilt ein Item als Ganzes (Grund 2), nicht jedes Literal darin: ein
//! Feld, das der Baustein der Fixture nicht liest, bliebe still. Gemessen ist eine neue Zeile in
//! einer Tabelle (rot an `tabellen_gleich_fixture`). Upgrade: die Literale dort wie Zonen aufzeichnen.
#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::doc_markdown
)]

use std::borrow::Cow;
use std::fs;
use std::path::Path;

use crate::regal::{datei_von, ist_kz_form, Zuordnung};

/// Ein exaktes Kz-Literal (`"E"` + 7 Ziffern) im Produktionstext.
pub(crate) struct KzFund {
    /// Byte-Bereich des Literals samt Anfuehrungszeichen.
    pub von: usize,
    pub bis: usize,
    pub kz: String,
    /// Das oberste `fn`/`const`/`static`, in dem es steht; `None` in `impl`, `mod`, Makro und Co.
    pub item: Option<String>,
}

/// Was die Abtastung eines Quelltextes ergibt.
#[derive(Default)]
pub(crate) struct Abtastung {
    pub funde: Vec<KzFund>,
    /// `#[cfg(test)] mod NAME;`: Dateien, die ganz Testcode sind.
    pub test_module: Vec<String>,
}

/// Index hinter der Zeichenkette, die bei `i` (dem `"`) beginnt.
fn ende_zeichenkette(b: &[u8], i: usize) -> usize {
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'"' => return j + 1,
            _ => j += 1,
        }
    }
    b.len()
}

/// Raw-String ab `i` (dem `r`): `(Inhalt von, Inhalt bis, Ende)`; `None`, wenn dort keiner beginnt.
fn ende_roh(b: &[u8], i: usize) -> Option<(usize, usize, usize)> {
    let mut j = i + 1;
    while b.get(j) == Some(&b'#') {
        j += 1;
    }
    let hashes = j - (i + 1);
    if b.get(j) != Some(&b'"') {
        return None;
    }
    let inhalt_von = j + 1;
    let mut schluss = vec![b'#'; hashes];
    schluss.insert(0, b'"');
    let p = b[inhalt_von..]
        .windows(schluss.len())
        .position(|w| w == schluss.as_slice())?;
    let inhalt_bis = inhalt_von + p;
    Some((inhalt_von, inhalt_bis, inhalt_bis + schluss.len()))
}

/// Index hinter dem Blockkommentar, der bei `i` beginnt (er darf sich verschachteln).
fn ende_blockkommentar(b: &[u8], i: usize) -> usize {
    let (mut tiefe, mut j) = (0usize, i);
    while j + 1 < b.len() {
        if b[j] == b'/' && b[j + 1] == b'*' {
            tiefe += 1;
            j += 2;
        } else if b[j] == b'*' && b[j + 1] == b'/' {
            tiefe -= 1;
            j += 2;
            if tiefe == 0 {
                return j;
            }
        } else {
            j += 1;
        }
    }
    b.len()
}

/// Index hinter `'x'`/`'\n'`, wenn bei `i` ein Zeichenliteral beginnt; sonst hinter der Lebensdauer
/// oder Schleifenmarke (`'static`, `'a`, `'outer`), damit ihr Name nicht als Wort zaehlt.
fn ende_zeichenliteral(text: &str, i: usize) -> usize {
    let b = text.as_bytes();
    if b.get(i + 1) == Some(&b'\\') {
        let mut j = i + 3;
        while j < b.len() && b[j] != b'\'' {
            j += 1;
        }
        return (j + 1).min(b.len());
    }
    if let Some(c) = text[i + 1..].chars().next() {
        let n = i + 1 + c.len_utf8();
        if b.get(n) == Some(&b'\'') {
            return n + 1;
        }
    }
    let mut j = i + 1;
    while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
        j += 1;
    }
    j
}

/// Text eines Attributs `#[…]`/`#![…]` ab `i` (dem `#`) ohne Leerraum, dazu der Index dahinter.
fn attribut_text(b: &[u8], i: usize) -> Option<(String, usize)> {
    let mut j = i + 1;
    if b.get(j) == Some(&b'!') {
        j += 1;
    }
    if b.get(j) != Some(&b'[') {
        return None;
    }
    let mut tiefe = 0usize;
    while j < b.len() {
        match b[j] {
            b'"' => {
                j = ende_zeichenkette(b, j);
                continue;
            }
            b'[' => tiefe += 1,
            b']' => {
                tiefe = tiefe.saturating_sub(1);
                if tiefe == 0 {
                    let text = b[i..=j]
                        .iter()
                        .filter(|c| c.is_ascii() && !c.is_ascii_whitespace())
                        .map(|c| char::from(*c))
                        .collect();
                    return Some((text, j + 1));
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// Das Wort ab `ab` (nach Leerraum) und der Index dahinter.
fn naechstes_wort(text: &str, ab: usize) -> Option<(&str, usize)> {
    let b = text.as_bytes();
    let mut von = ab;
    while von < b.len() && b[von].is_ascii_whitespace() {
        von += 1;
    }
    let mut bis = von;
    while bis < b.len() && (b[bis].is_ascii_alphanumeric() || b[bis] == b'_') {
        bis += 1;
    }
    (bis > von).then(|| (&text[von..bis], bis))
}

/// Womit ein Item nach `#[cfg(test)]` beginnen darf; alles andere bricht ab (siehe Kopf).
const ITEM_ANFANG: &[&str] = &[
    "mod", "fn", "pub", "const", "static", "impl", "use", "struct", "enum", "trait", "type",
    "unsafe", "async", "extern",
];

struct Abtaster<'a> {
    text: &'a str,
    b: &'a [u8],
    /// Welche Literale ein Fund sind (die Wache: Kz-Form; `abdeckung.rs`: Bezeichner-Form).
    treffer: fn(&str) -> bool,
    i: usize,
    /// Tiefe der Klammern `(`, `[`, `{` zusammen.
    tiefe: usize,
    item: Option<String>,
    /// `Some(t)`: ein `#[cfg(test)]`-Item laeuft, das bei Klammertiefe `t` begann; Funde zaehlen nicht.
    test_ab: Option<usize>,
    /// Das erste Wort nach `#[cfg(test)]` ist noch nicht geprueft.
    test_pruefen: bool,
    vorwort: String,
    /// Name aus `mod NAME`, solange unklar ist, ob das Modul einen Rumpf hat.
    test_mod: Option<String>,
    aus: Abtastung,
}

impl Abtaster<'_> {
    fn lauf(mut self) -> Abtastung {
        while self.i < self.b.len() {
            self.schritt();
        }
        self.aus
    }

    fn schritt(&mut self) {
        let (c, n) = (self.b[self.i], self.b.get(self.i + 1).copied());
        match (c, n) {
            (b'/', Some(b'/')) => {
                self.i = self.text[self.i..]
                    .find('\n')
                    .map_or(self.b.len(), |j| self.i + j);
            }
            (b'/', Some(b'*')) => self.i = ende_blockkommentar(self.b, self.i),
            (b'"', _) => {
                let ende = ende_zeichenkette(self.b, self.i);
                self.fund(self.i, ende, self.i + 1, ende.saturating_sub(1));
                self.i = ende;
            }
            (b'\'', _) => self.i = ende_zeichenliteral(self.text, self.i),
            (b'#', _) => self.attribut(),
            (b'(' | b'[' | b'{', _) => {
                self.tiefe += 1;
                if c == b'{' {
                    self.test_mod = None; // `mod NAME {` hat einen Rumpf, keine eigene Datei
                }
                self.i += 1;
            }
            (b')' | b']' | b'}', _) => {
                self.schliesse(c);
                self.i += 1;
            }
            (b';', _) => {
                self.semikolon();
                self.i += 1;
            }
            (c, _) if c.is_ascii_alphabetic() || c == b'_' => self.wort(),
            _ => self.i += 1,
        }
    }

    /// Haelt `text[inhalt_von..inhalt_bis]` als Fund fest, wenn `treffer` es annimmt und kein Testcode laeuft.
    fn fund(&mut self, von: usize, bis: usize, inhalt_von: usize, inhalt_bis: usize) {
        if self.test_ab.is_some() {
            return;
        }
        if let Some(kz) = self
            .text
            .get(inhalt_von..inhalt_bis)
            .filter(|s| (self.treffer)(s))
        {
            self.aus.funde.push(KzFund {
                von,
                bis,
                kz: kz.to_owned(),
                item: self.item.clone(),
            });
        }
    }

    fn attribut(&mut self) {
        let Some((attr, ende)) = attribut_text(self.b, self.i) else {
            self.i += 1;
            return;
        };
        self.i = ende;
        if attr == "#[cfg(test)]" {
            if self.test_ab.is_none() {
                self.test_ab = Some(self.tiefe);
                self.test_pruefen = true;
                self.vorwort.clear();
            }
        } else {
            // `test` als Wort (`not(test)`, `all(test, …)`); `feature = "testhilfe"` ist keins.
            assert!(
                !(attr.starts_with("#[cfg(") && (attr.contains("(test") || attr.contains(",test"))),
                "kz_wache kennt `{attr}` nicht: cfg(test) nur als `#[cfg(test)]` vor einem Item; Abtastung erweitern"
            );
        }
    }

    fn schliesse(&mut self, c: u8) {
        self.tiefe = self.tiefe.saturating_sub(1);
        if c == b'}' {
            if self.test_ab == Some(self.tiefe) {
                self.test_ab = None;
            }
            if self.tiefe == 0 {
                self.item = None;
            }
        }
    }

    fn semikolon(&mut self) {
        if self.test_ab == Some(self.tiefe) {
            if let Some(name) = self.test_mod.take() {
                self.aus.test_module.push(name);
            }
            self.test_ab = None;
        }
        if self.tiefe == 0 {
            self.item = None;
        }
    }

    /// Ein Wort (Bezeichner oder Schluesselwort); `r"…"`/`br#"…"#` sind Raw-Strings, kein Wort.
    fn wort(&mut self) {
        let (text, von) = (self.text, self.i);
        let mut ende = von;
        while ende < self.b.len() && (self.b[ende].is_ascii_alphanumeric() || self.b[ende] == b'_')
        {
            ende += 1;
        }
        let w = &text[von..ende];
        if (w == "r" || w == "br") && matches!(self.b.get(ende), Some(b'"' | b'#')) {
            if let Some((iv, ib, schluss)) = ende_roh(self.b, ende - 1) {
                self.fund(von, schluss, iv, ib);
                self.i = schluss;
                return;
            }
        }
        self.i = ende;
        if self.test_pruefen {
            self.test_pruefen = false;
            assert!(
                ITEM_ANFANG.contains(&w),
                "kz_wache: `#[cfg(test)]` steht vor `{w}`, keinem Item; Abtastung erweitern"
            );
        }
        if self.test_ab.is_some() {
            if self.vorwort == "mod" {
                self.test_mod = Some(w.to_owned());
            }
            w.clone_into(&mut self.vorwort);
        } else if self.tiefe == 0 {
            self.item_wort(w, ende);
        }
    }

    /// Ein Wort auf oberster Ebene: `fn NAME`, `const NAME`, `const fn NAME`, `static [mut] NAME`
    /// setzen das Item; `impl`, `mod`, `struct` und Verwandte loeschen es.
    fn item_wort(&mut self, w: &str, ende: usize) {
        match w {
            "fn" => {
                if let Some((name, _)) = naechstes_wort(self.text, ende) {
                    self.item = Some(name.to_owned());
                }
            }
            "const" | "static" => {
                let mut name = naechstes_wort(self.text, ende);
                if let Some(("fn" | "mut", nach)) = name {
                    name = naechstes_wort(self.text, nach);
                }
                if let Some((name, _)) = name {
                    self.item = Some(name.to_owned());
                }
            }
            "impl" | "mod" | "struct" | "enum" | "trait" | "use" | "type" | "union"
            | "macro_rules" => {
                self.item = None;
            }
            _ => {}
        }
    }
}

/// Alle exakten Kz-Literale ausserhalb von `#[cfg(test)]` in `text`, dazu die Testmodul-Dateien.
pub(crate) fn abtasten(text: &str) -> Abtastung {
    abtasten_mit(text, ist_kz_form)
}

/// Wie [`abtasten`], aber mit eigener Form: jedes Literal ausserhalb von `#[cfg(test)]`, das
/// `treffer` annimmt. `KzFund::kz` heisst dann „der Literal-Text".
pub(crate) fn abtasten_mit(text: &str, treffer: fn(&str) -> bool) -> Abtastung {
    Abtaster {
        text,
        b: text.as_bytes(),
        treffer,
        i: 0,
        tiefe: 0,
        item: None,
        test_ab: None,
        test_pruefen: false,
        vorwort: String::new(),
        test_mod: None,
        aus: Abtastung::default(),
    }
    .lauf()
}

/// Alle `.rs`-Dateien unter `dir` als Pfad relativ zu `wurzel` (mit `/`), sortiert.
pub(crate) fn rs_dateien(wurzel: &Path, dir: &Path, aus: &mut Vec<String>) {
    let mut pfade: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    pfade.sort();
    for p in pfade {
        if p.is_dir() {
            rs_dateien(wurzel, &p, aus);
        } else if p.extension().is_some_and(|e| e == "rs") {
            aus.push(
                p.strip_prefix(wurzel)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

/// Der Text einer Datei: aus dem Regal, wenn es sie kennt (`include_str!`, dieselben Adressen wie in
/// der Aufzeichnung), sonst von der Platte.
fn text_von(wurzel: &Path, rel: &str) -> Cow<'static, str> {
    datei_von(rel).map_or_else(
        || Cow::Owned(fs::read_to_string(wurzel.join(rel)).unwrap()),
        |d| Cow::Borrowed(d.text),
    )
}

/// Die Dateien, die `#[cfg(test)] mod NAME;` in `rel` meint.
pub(crate) fn modul_dateien(rel: &str, name: &str) -> [String; 2] {
    let (dir, datei) = rel.rsplit_once('/').map_or(("", rel), |(d, f)| (d, f));
    let vorn = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    let basis = if matches!(datei, "lib.rs" | "main.rs" | "mod.rs") {
        vorn
    } else {
        format!("{vorn}{}/", datei.trim_end_matches(".rs"))
    };
    [format!("{basis}{name}.rs"), format!("{basis}{name}/mod.rs")]
}

/// Ergebnis der Wache.
pub(crate) struct Befund {
    pub fehler: Vec<String>,
    /// Gepruefte Kz-Literale je Datei (nur Dateien mit mindestens einem): der Beweis, dass die
    /// Abtastung etwas gesehen hat.
    pub je_datei: Vec<(String, usize)>,
}

/// Prueft jedes Kz-Literal im Produktionstext von `src/` gegen die drei Gruende des Kopfes;
/// `gelesen` sind die Bereiche aus `regal::aufzeichnen`.
pub(crate) fn kz_literal_befund(gelesen: &[(usize, usize)]) -> Befund {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut pfade = Vec::new();
    rs_dateien(&wurzel, &wurzel, &mut pfade);
    let dateien: Vec<(String, Cow<'static, str>, Abtastung)> = pfade
        .into_iter()
        .map(|rel| {
            let text = text_von(&wurzel, &rel);
            let abt = abtasten(&text);
            (rel, text, abt)
        })
        .collect();
    let test_dateien: Vec<String> = dateien
        .iter()
        .flat_map(|(rel, _, abt)| abt.test_module.iter().flat_map(|n| modul_dateien(rel, n)))
        .collect();
    let mut befund = Befund {
        fehler: Vec::new(),
        je_datei: Vec::new(),
    };
    for (rel, text, abt) in dateien
        .iter()
        .filter(|(rel, _, _)| !test_dateien.contains(rel))
    {
        if !abt.funde.is_empty() {
            befund.je_datei.push((rel.clone(), abt.funde.len()));
        }
        befund
            .fehler
            .extend(datei_fehler(rel, text, &abt.funde, gelesen));
    }
    befund
}

/// Die Zuordnungen, die das Regal einem obersten Item gibt (es kann mehrere haben).
fn zuordnungen<'a>(
    eintraege: &'a [(&'static str, Zuordnung)],
    item: &'a str,
) -> impl Iterator<Item = &'a Zuordnung> {
    eintraege
        .iter()
        .filter(move |(n, _)| *n == item)
        .map(|(_, z)| z)
}

/// Die Fehler einer Datei: Literale ohne Grund, und Regal-Eintraege `Verhalten`, die nicht mehr stimmen.
fn datei_fehler(
    rel: &str,
    text: &str,
    funde: &[KzFund],
    gelesen: &[(usize, usize)],
) -> Vec<String> {
    let basis = text.as_ptr().addr();
    let eintraege = datei_von(rel).map_or(&[][..], |d| d.eintraege);
    let mut fehler = Vec::new();
    for f in funde {
        let item = f.item.as_deref();
        let von_fixture_gelesen = gelesen
            .iter()
            .any(|&(v, b)| v <= basis + f.von && basis + f.bis <= b);
        let in_tabelle = rel == "tabellen.rs"
            && item.is_some_and(|i| {
                zuordnungen(eintraege, i).any(|z| !matches!(z, Zuordnung::Ausnahme(_)))
            });
        let im_verhalten = item.is_some_and(|i| {
            zuordnungen(eintraege, i).any(|z| matches!(z, Zuordnung::Verhalten { .. }))
        });
        if !(von_fixture_gelesen || in_tabelle || im_verhalten) {
            let zeile = text.get(..f.von).map_or(0, |v| v.matches('\n').count() + 1);
            fehler.push(format!(
                "{rel}:{zeile} \"{}\" in {}: von keiner Fixture-Zone gelesen und in keiner Tabelle",
                f.kz,
                item.map_or_else(
                    || "einem impl-Block, Modul oder Makro".to_owned(),
                    |i| format!("`{i}`")
                ),
            ));
        }
    }
    for (name, z) in eintraege {
        if let Zuordnung::Verhalten { kz, tests } = z {
            let gefunden: Vec<&str> = funde
                .iter()
                .filter(|f| f.item.as_deref() == Some(*name))
                .map(|f| f.kz.as_str())
                .collect();
            if gefunden != *kz {
                fehler.push(format!(
                    "{rel}: `{name}` fuehrt die Kz {gefunden:?}, das Regal nennt {kz:?}"
                ));
            }
            for t in *tests {
                if !text.contains(&format!("fn {t}(")) {
                    fehler.push(format!("{rel}: Test `{t}` zu `{name}` fehlt"));
                }
            }
        }
    }
    fehler
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ein Quelltext, der jede Falle der Abtastung enthaelt. Erwartet: 6 Funde, 1 Testdatei.
    const PROBE: &str = r##"
const A: &[&str] = &["E0100001"];
// "E0100002" im Kommentar
/* "E0100003" und /* verschachtelt */ "E0100004" */
fn f<'a>(x: &'a str) -> &'static str {
    let c = '"';
    let d = '\'';
    let _ = "E01000051";
    let _ = r#"E0100006 "mit" Anfuehrung"#;
    let _ = r#"E0100006"#;
    "E0100007"
}
#[cfg(test)]
mod tests { fn t() { let _ = "E0100008"; } }
#[cfg(test)]
mod extern_datei;
#[cfg(test)]
pub(crate) fn nur_test() -> &'static str { "E0100009" }
struct X;
impl X { fn m() { let _ = "E0100010"; } }
const fn g() -> &'static str { "E0100011" }
const T: fn(u8) -> u8 = h; const U: &str = "E0100012";
"##;

    #[test]
    fn die_abtastung_liest_quelltext_wie_der_compiler() {
        let abt = abtasten(PROBE);
        let gefunden: Vec<(&str, Option<&str>)> = abt
            .funde
            .iter()
            .map(|f| (f.kz.as_str(), f.item.as_deref()))
            .collect();
        assert_eq!(
            gefunden,
            [
                ("E0100001", Some("A")),
                ("E0100006", Some("f")),
                ("E0100007", Some("f")),
                ("E0100010", None),
                ("E0100011", Some("g")),
                ("E0100012", Some("U")),
            ]
        );
        assert_eq!(abt.test_module, ["extern_datei"]);
        for f in &abt.funde {
            assert!(
                PROBE[f.von..f.bis].contains(&f.kz),
                "Lage von {} stimmt nicht",
                f.kz
            );
        }
    }

    #[test]
    #[should_panic(expected = "kz_wache kennt")]
    fn eine_unbekannte_cfg_form_bricht_ab() {
        let _ = abtasten("#[cfg(not(test))]\nfn x() {}\n");
    }

    #[test]
    fn eine_cfg_mit_feature_testhilfe_ist_kein_cfg_test() {
        // `domain/src/lib.rs` hat `#[cfg(feature = "testhilfe")]`; die Abtastung ueber alle Crates darf daran nicht abbrechen.
        let abt = abtasten("#[cfg(feature = \"testhilfe\")]\nfn x() { let _ = \"E0100001\"; }\n");
        assert_eq!(abt.funde.len(), 1);
    }

    #[test]
    fn abtasten_mit_nimmt_die_form_des_aufrufers() {
        let abt = abtasten_mit(
            "fn f() { let _ = (\"veranlagung\", \"E0100001\", 'x', \"A\"); }\n#[cfg(test)]\nfn t() { let _ = \"nur_test\"; }\n",
            |s| s.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
        );
        let texte: Vec<&str> = abt.funde.iter().map(|f| f.kz.as_str()).collect();
        assert_eq!(texte, ["veranlagung"]);
    }

    #[test]
    #[should_panic(expected = "keinem Item")]
    fn cfg_test_vor_etwas_anderem_als_einem_item_bricht_ab() {
        let _ = abtasten("#[cfg(test)]\nfoo! { \"E0100001\" }\n");
    }

    #[test]
    fn modul_dateien_folgen_der_modulordnung() {
        assert_eq!(
            modul_dateien("lib.rs", "regal"),
            ["regal.rs", "regal/mod.rs"]
        );
        assert_eq!(
            modul_dateien("eric/mod.rs", "t"),
            ["eric/t.rs", "eric/t/mod.rs"]
        );
        assert_eq!(modul_dateien("xml.rs", "t"), ["xml/t.rs", "xml/t/mod.rs"]);
    }
}
