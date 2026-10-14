//! Waechter der Belege: jeder Verweis in Spalte 7 von `rust/TESTMAP.tsv` und im Text von `rust/fixtures/README.md`
//! loest auf. Die Karte und die Abweichungsliste sagen, warum eine Abweichung gilt und was einen Python-Test ersetzt;
//! ein Beleg, den es nicht mehr gibt (Bericht im Cache, geloeschte Parity, Testname von gestern), macht beides unpruefbar.
//!
//! Was als Verweis zaehlt (Muster, keine Grammatik: was der Pruefer nicht erkennt, prueft er nicht):
//! - Datei: ein Pfad mit Endung. Gueltig ist der Pfad im Baum (ab Wurzel, ab `rust/` oder als eindeutiges Suffix) oder die
//!   Verlaufsform `<commit>:<pfad>` / `<commit>^:<pfad>`, wenn Git den Pfad in diesem Commit kennt (Weg fuer geloeschte Dateien).
//!   Pfade ab `decisions/`, `audits/`, `tickets/`, `backlog/`, `research/` liegen im Vault: der Test zaehlt sie und prueft sie nicht.
//! - Test: `Pfad::name`, `Pfad::tests::name`, `Pfad::a, ::b` und `Pfad: `name`, `name``; der Name steht als `fn` in der Datei.
//! - Typ: `Name<..>` steht als `struct`, `enum`, `trait`, `type` oder `union` im Baum; ein Typ, dem der Satz selbst
//!   abspricht zu existieren (`.. den gibt es nicht`, `.. den es nie gab`), zaehlt nicht.
//! - Cache: `~/.cache/..` ist nie ein Beleg (fluechtig).
//! - Hash: eine Commit-Kurzform ist Vorfahre von HEAD (ein Hash vom Nebenzweig loest nur dort auf).
//!
//! Verlauf: Hash- und Verlaufspruefung brauchen den ganzen Verlauf. Ein flacher Klon (oder ein Baum ohne `.git`) meldet das laut und
//! wird rot, ausser `TAXGRAPH_OHNE_VERLAUF=1`; dann steht die Zahl der ungeprueften Verweise auf stderr (`UEBERSPRUNGEN: Verlauf fehlt`),
//! und `scripts/ci-nicht-geprueft.sh` zaehlt sie. Der Job `rust` der CI holt den Verlauf (`fetch-depth: 0`).
//!
//! Der Pruefer liest nur die Karte, die Abweichungsliste, den Baum und Git; nichts aus `tests/`, `tools/` oder Python
//! (er ueberlebt L2). Das Tor des Repos (`.orch-gate.json`) fuehrt keine Rust-Tests aus (Loeschplan E7): lokal laeuft er
//! mit `cd rust && cargo test -p bindung --test belege_aufloesbar`.
//!
//! Grenzen: Namen nach einer Verlaufsform werden nicht gegen die alte Datei geprueft. Vault-Pfade prueft niemand
//! (ponytail: eine Umgebungsvariable mit dem Vault-Ordner liesse sie aufloesen). `BELEGE_LISTE=1 cargo test .. -- --nocapture`
//! druckt den ganzen Befund, auch wenn der Test gruen ist.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use regex::Regex;
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::LazyLock;

const ENDUNGEN: &str = "rs|py|md|sh|tsv|yaml|yml|json|c|toml|txt|js|html|css|xml|xsd|lock";
const VAULT: [&str; 5] = [
    "decisions/",
    "audits/",
    "tickets/",
    "backlog/",
    "research/",
];
/// Pfade der Parity, die L1 (`9f4442b1`, `437451ee`) geloescht hat: sie gelten nur noch in der Verlaufsform.
const GELOESCHT: [&str; 3] = ["rust/parity/", "parity/", "tools/parity/"];
/// Typen aus fremden Crates und der Standardbibliothek: ihre Deklaration steht nicht im Baum.
const FREMD: &[&str] = &[
    "Option",
    "Result",
    "Vec",
    "String",
    "Box",
    "HashMap",
    "BTreeMap",
    "BTreeSet",
    "HashSet",
    "Arc",
    "Mutex",
    "Cow",
    "Path",
    "PathBuf",
    "Value",
    "Json",
    "Decimal",
    "Router",
    "Rc",
    "RefCell",
    "Cell",
    "Ordering",
    "Regex",
    "Display",
    "Debug",
    "Clone",
    "Copy",
    "Fn",
    "FnMut",
    "FnOnce",
    "Deserialize",
    "Serialize",
    "Error",
    "Infallible",
];

fn re(muster: &str) -> Regex {
    Regex::new(muster).unwrap()
}

// Das Rust-`regex` kennt kein Lookaround: Zeichen vor und nach einem Treffer prueft der Code von Hand.
/// Pfad mit Endung; Gruppe 1 ist der Pfad. Das Zeichen dahinter steht im Muster, damit `.c` nicht `.css` verdeckt.
static PFAD: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"((?:[\w.-]+/)+[\w.-]+\.(?:{ENDUNGEN}))(?:\W|$)"
    ))
});
static KETTE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^((?:::[A-Za-z_]\w*)+)(\{[\w,]+\}\w*)?"));
static DOPPELPUNKT: LazyLock<Regex> = LazyLock::new(|| {
    re(r"^`?:\s+((?:`[a-z][a-z0-9_]{8,}`(?:\s*\([^)]*\))?(?:,\s*(?:und\s+)?)?)+)")
});
static NAME_IN_TICKS: LazyLock<Regex> = LazyLock::new(|| re(r"`([a-z][a-z0-9_]{8,})`"));
/// Das, was zwischen zwei Testnamen einer Kette steht; danach muss `::` folgen (das prueft der Code).
static FOLGE: LazyLock<Regex> = LazyLock::new(|| re(r"^`?,?\s*(?:und\s+)?`?"));
static GRUPPE: LazyLock<Regex> = LazyLock::new(|| re(r"^(\w*)\{([\w,]+)\}(\w*)$"));
static CACHE: LazyLock<Regex> = LazyLock::new(|| re(r"~/\.cache/[\w./*-]+"));
static TYP: LazyLock<Regex> = LazyLock::new(|| {
    re(r"([A-Z][A-Za-z0-9]+)<([A-Za-z0-9_, ]+(?:<[A-Za-z0-9_, ]+>)?)>")
});
static VERNEINT: LazyLock<Regex> =
    LazyLock::new(|| re(r"^[,\s(]*den (?:gibt es nicht|es nie gab)"));
static DEKLARATION: LazyLock<Regex> =
    LazyLock::new(|| re(r"\b(?:struct|enum|trait|type|union)\s+(\w+)"));
/// Commit-Kurzform; 16 Hex-Zeichen sind Inhalts-Hashes und gehoeren nicht dazu.
static HASH: LazyLock<Regex> = LazyLock::new(|| re(r"[0-9a-f]{7,12}"));
/// Das Ende eines Texts, der eine Revision nennt: `9f4442b1^` oder `9f4442b1` (vor dem Doppelpunkt).
static REVISION: LazyLock<Regex> = LazyLock::new(|| re(r"([0-9a-f]{7,40}(?:\^\d*|~\d+)*)$"));
static NR_IN_TABELLE: LazyLock<Regex> = LazyLock::new(|| re(r"^\| (\d+) \|"));

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Art {
    Datei,
    Test,
    Cache,
    Typ,
    Hash,
}

impl Art {
    fn name(self) -> &'static str {
        match self {
            Art::Datei => "DATEI",
            Art::Test => "TEST",
            Art::Cache => "CACHE",
            Art::Typ => "TYP",
            Art::Hash => "HASH",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lage {
    Vorfahre,
    KeinVorfahre,
    KeinCommit,
}

/// Was Git ueber den Verlauf weiss. Die Attrappe in den Tests ersetzt es.
trait Verlauf {
    /// Der ganze Verlauf liegt vor (kein flacher Klon, ein `.git` ist da).
    fn da(&self) -> bool;
    /// `<rev>:<pfad>` steht in Git (`git cat-file -e`).
    fn kennt(&self, rev_pfad: &str) -> bool;
    fn lage(&self, hash: &str) -> Lage;
}

fn git(wurzel: &Path, args: &[&str]) -> Option<Output> {
    Command::new("git")
        .arg("-C")
        .arg(wurzel)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .ok()
}

fn gelingt(wurzel: &Path, args: &[&str]) -> bool {
    git(wurzel, args).is_some_and(|o| o.status.success())
}

struct EchtesGit(PathBuf);

impl Verlauf for EchtesGit {
    fn da(&self) -> bool {
        self.0.join(".git").exists()
            && git(&self.0, &["rev-parse", "--is-shallow-repository"]).is_some_and(|o| {
                o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "false"
            })
    }

    fn kennt(&self, rev_pfad: &str) -> bool {
        gelingt(&self.0, &["cat-file", "-e", rev_pfad])
    }

    fn lage(&self, hash: &str) -> Lage {
        if !gelingt(&self.0, &["cat-file", "-e", &format!("{hash}^{{commit}}")]) {
            Lage::KeinCommit
        } else if gelingt(&self.0, &["merge-base", "--is-ancestor", hash, "HEAD"]) {
            Lage::Vorfahre
        } else {
            Lage::KeinVorfahre
        }
    }
}

/// Dateien, die im Baum liegen. Mit `.git`: was Git verfolgt oder nicht ignoriert (eine ignorierte Laufzeitdatei ist
/// kein Beleg: in der CI fehlt sie). Ohne `.git` (Abzug, Test-Baum): ein Gang durch den Ordner.
fn dateien_im_baum(wurzel: &Path) -> BTreeSet<String> {
    if wurzel.join(".git").exists() {
        let args = ["ls-files", "-z", "--cached", "--others", "--exclude-standard"];
        if let Some(o) = git(wurzel, &args).filter(|o| o.status.success()) {
            return o
                .stdout
                .split(|b| *b == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .filter(|p| wurzel.join(p).is_file())
                .collect();
        }
    }
    let mut aus = BTreeSet::new();
    gehe(wurzel, wurzel, &mut aus);
    aus
}

/// ponytail: die Ausnahmen spiegeln `.gitignore` (Bauordner); ein neuer ignorierter Ordner ohne `.git` im Baum faellt hier auf.
fn gehe(wurzel: &Path, dir: &Path, aus: &mut BTreeSet<String>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if p.is_dir() {
            let bau = name.starts_with('.')
                || name.starts_with("target")
                || matches!(
                    name.as_str(),
                    "_build" | "_target" | "_targets" | "_catala" | "__pycache__" | "node_modules"
                );
            if !bau {
                gehe(wurzel, &p, aus);
            }
        } else {
            aus.insert(
                p.strip_prefix(wurzel)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

enum Fund {
    Da(String),
    Mehrdeutig,
    Fehlt,
}

struct Baum {
    wurzel: PathBuf,
    dateien: BTreeSet<String>,
    nach_name: HashMap<String, Vec<String>>,
    texte: RefCell<HashMap<String, String>>,
    typen: OnceCell<BTreeSet<String>>,
}

impl Baum {
    fn lade(wurzel: &Path) -> Self {
        let dateien = dateien_im_baum(wurzel);
        let mut nach_name: HashMap<String, Vec<String>> = HashMap::new();
        for d in &dateien {
            let name = d.rsplit('/').next().unwrap_or(d);
            nach_name.entry(name.to_owned()).or_default().push(d.clone());
        }
        Baum {
            wurzel: wurzel.to_owned(),
            dateien,
            nach_name,
            texte: RefCell::new(HashMap::new()),
            typen: OnceCell::new(),
        }
    }

    fn lies(&self, rel: &str) -> String {
        String::from_utf8_lossy(&std::fs::read(self.wurzel.join(rel)).unwrap_or_default())
            .into_owned()
    }

    fn mit_text<R>(&self, rel: &str, f: impl FnOnce(&str) -> R) -> R {
        let mut c = self.texte.borrow_mut();
        f(c.entry(rel.to_owned()).or_insert_with(|| self.lies(rel)))
    }

    /// Ab Wurzel, ab `rust/`, sonst als Suffix: eindeutig oder nicht.
    fn aufloesen(&self, tok: &str) -> Fund {
        for kopf in ["", "rust/"] {
            let p = format!("{kopf}{tok}");
            if self.dateien.contains(&p) {
                return Fund::Da(p);
            }
        }
        let name = tok.rsplit('/').next().unwrap_or(tok);
        let ende = format!("/{tok}");
        let treffer: Vec<&String> = self
            .nach_name
            .get(name)
            .map(|v| v.iter().filter(|p| p.ends_with(&ende)).collect())
            .unwrap_or_default();
        match treffer.as_slice() {
            [] => Fund::Fehlt,
            [p] => Fund::Da((*p).clone()),
            _ => Fund::Mehrdeutig,
        }
    }

    fn hat_name(&self, datei: &str, name: &str) -> bool {
        let namen: Vec<String> = match GRUPPE.captures(name) {
            Some(g) => g[2]
                .split(',')
                .map(|a| format!("{}{a}{}", &g[1], &g[3]))
                .collect(),
            None => vec![name.to_owned()],
        };
        let ist_quelltext = datei.ends_with(".rs") || datei.ends_with(".py");
        self.mit_text(datei, |t| {
            namen.iter().all(|n| {
                let esc = regex::escape(n);
                if let Some(praefix) = esc.strip_suffix('_') {
                    // `rente_` steht fuer eine Gruppe: jeder Name mit diesem Anfang genuegt.
                    return re(&format!(r"\b(?:fn|def)\s+{praefix}_\w*")).is_match(t);
                }
                re(&format!(r"\b(?:fn|def|mod|class|struct|enum)\s+{esc}\b")).is_match(t)
                    || (!ist_quelltext && t.contains(n.as_str()))
            })
        })
    }

    fn hat_modul_tests(&self, datei: &str) -> bool {
        self.mit_text(datei, |t| re(r"\bmod\s+tests\b").is_match(t))
    }

    fn typ_da(&self, name: &str) -> bool {
        self.typen
            .get_or_init(|| {
                let mut s = BTreeSet::new();
                for d in self
                    .dateien
                    .iter()
                    .filter(|d| d.starts_with("rust/") && d.ends_with(".rs"))
                {
                    for m in DEKLARATION.captures_iter(&self.lies(d)) {
                        s.insert(m[1].to_owned());
                    }
                }
                s
            })
            .contains(name)
    }
}

#[derive(Debug)]
struct Befund {
    art: Art,
    klasse: String,
    ort: String,
    tok: String,
}

#[derive(Default)]
struct Ergebnis {
    zeilen: usize,
    geprueft: BTreeMap<Art, usize>,
    befunde: Vec<Befund>,
    /// Vault-Pfade: gezaehlt, nicht geprueft.
    vault: Vec<(String, String)>,
    /// Verweise, die ohne Verlauf nicht zu pruefen waren (Hash, Verlaufsform).
    ohne_verlauf: usize,
}

impl Ergebnis {
    fn zeilen_mit_befund(&self) -> usize {
        self.befunde
            .iter()
            .map(|b| b.ort.split_whitespace().next().unwrap_or(""))
            .collect::<BTreeSet<_>>()
            .len()
    }

    fn bericht(&self, name: &str, liste: bool) -> String {
        let mut s = format!(
            "== {name}: {} Zeilen; {} Zeilen mit mindestens einem nicht aufloesbaren Verweis\n",
            self.zeilen,
            self.zeilen_mit_befund()
        );
        for art in [Art::Datei, Art::Cache, Art::Test, Art::Typ, Art::Hash] {
            let mut klassen: BTreeMap<&str, usize> = BTreeMap::new();
            for b in self.befunde.iter().filter(|b| b.art == art) {
                *klassen.entry(b.klasse.as_str()).or_default() += 1;
            }
            let davon: Vec<String> = klassen.iter().map(|(k, n)| format!("{k}={n}")).collect();
            let _ = writeln!(
                s,
                "   {:6} geprueft {:4}   nicht aufloesbar {:3}{}",
                art.name(),
                self.geprueft.get(&art).copied().unwrap_or(0),
                klassen.values().sum::<usize>(),
                if davon.is_empty() {
                    String::new()
                } else {
                    format!("   davon {}", davon.join(", "))
                }
            );
        }
        let _ = writeln!(
            s,
            "   VAULT  gezaehlt {:4}   nicht geprueft;  ohne Verlauf nicht geprueft: {}",
            self.vault.len(),
            self.ohne_verlauf
        );
        let mut alle: Vec<String> = self
            .befunde
            .iter()
            .map(|b| format!("[{}/{}] {} -> {}", b.art.name(), b.klasse, b.ort, b.tok))
            .collect();
        if liste {
            alle.extend(self.vault.iter().map(|(o, t)| format!("[VAULT] {o} -> {t}")));
        }
        for z in alle {
            let _ = writeln!(s, "     {z}");
        }
        s
    }
}

fn ist_wort(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Das Zeichen vor `start` ist keines, das den Treffer zum Teil eines laengeren Worts oder Pfads macht.
fn frei_vor(text: &str, start: usize, nicht: &str) -> bool {
    !text[..start]
        .chars()
        .next_back()
        .is_some_and(|c| ist_wort(c) || nicht.contains(c))
}

fn frei_nach(text: &str, ende: usize) -> bool {
    !text[ende..].chars().next().is_some_and(ist_wort)
}

/// Steht vor dem Pfad `<rev>:` oder `<rev>^:`, ist das die Verlaufsform: die Revision.
fn verlaufsform(vor: &str) -> Option<String> {
    let kopf = vor.strip_suffix(':')?;
    let m = REVISION.find(kopf)?;
    frei_vor(kopf, m.start(), "").then(|| m.as_str().to_owned())
}

/// Der Hash steht hinter dem Wort `Vault`: er gehoert dem Vault-Repo, nicht diesem.
fn steht_im_vault(text: &str, start: usize) -> bool {
    let rueck: Vec<char> = text[..start].chars().rev().take(8).collect();
    let fenster: String = rueck.into_iter().rev().filter(|c| *c != '`').collect();
    fenster.trim_end().ends_with("Vault")
}

fn klasse_der_datei(tok: &str) -> &'static str {
    if GELOESCHT.iter().any(|g| tok.starts_with(g)) {
        "L1"
    } else if tok.starts_with("berichte/") {
        "BERICHT"
    } else {
        "SONST"
    }
}

struct Pruefer<'a> {
    baum: &'a Baum,
    git: &'a dyn Verlauf,
    home: &'a Path,
    verlauf_pruefen: bool,
    lagen: HashMap<String, Lage>,
    erg: Ergebnis,
}

impl<'a> Pruefer<'a> {
    fn neu(baum: &'a Baum, git: &'a dyn Verlauf, home: &'a Path, verlauf_pruefen: bool) -> Self {
        Pruefer {
            baum,
            git,
            home,
            verlauf_pruefen,
            lagen: HashMap::new(),
            erg: Ergebnis::default(),
        }
    }

    fn buche(&mut self, art: Art, gut: bool, ort: &str, tok: &str, klasse: &str) {
        *self.erg.geprueft.entry(art).or_default() += 1;
        if !gut {
            self.erg.befunde.push(Befund {
                art,
                klasse: klasse.to_owned(),
                ort: ort.to_owned(),
                tok: tok.to_owned(),
            });
        }
    }

    fn text(&mut self, ort: &str, text: &str) {
        self.pfade(ort, text);
        self.cache(ort, text);
        self.typen(ort, text);
        self.hashes(ort, text);
    }

    fn pfade(&mut self, ort: &str, text: &str) {
        for m in PFAD.captures_iter(text) {
            let g = m.get(1).unwrap();
            if !frei_vor(text, g.start(), "/.~-") {
                continue;
            }
            let tok = g.as_str().trim_end_matches(['.', ',', ';', ':', ')']);
            if VAULT.iter().any(|v| tok.starts_with(v)) {
                self.erg.vault.push((ort.to_owned(), tok.to_owned()));
            } else if let Some(rev) = verlaufsform(&text[..g.start()]) {
                self.verlaufspfad(ort, &rev, tok);
            } else {
                match self.baum.aufloesen(tok) {
                    Fund::Da(datei) => {
                        self.buche(Art::Datei, true, ort, tok, "");
                        self.namen(ort, &datei, tok, &text[g.end()..]);
                    }
                    Fund::Mehrdeutig => {
                        let k = format!("{}/mehrdeutig", klasse_der_datei(tok));
                        self.buche(Art::Datei, false, ort, tok, &k);
                    }
                    Fund::Fehlt => self.buche(Art::Datei, false, ort, tok, klasse_der_datei(tok)),
                }
            }
        }
    }

    fn verlaufspfad(&mut self, ort: &str, rev: &str, tok: &str) {
        if !self.verlauf_pruefen {
            self.erg.ohne_verlauf += 1;
            return;
        }
        let rev_pfad = format!("{rev}:{tok}");
        let gut = self.git.kennt(&rev_pfad);
        self.buche(Art::Datei, gut, ort, &rev_pfad, "VERLAUF");
    }

    /// Testnamen hinter einem aufgeloesten Pfad: `Pfad::a::b, ::c` und `Pfad`: `name`, `name`.
    fn namen(&mut self, ort: &str, datei: &str, tok: &str, hinten: &str) {
        if let Some(dp) = DOPPELPUNKT.captures(hinten) {
            for n in NAME_IN_TICKS.captures_iter(&dp[1]) {
                let gut = self.baum.hat_name(datei, &n[1]);
                self.buche(Art::Test, gut, ort, &format!("{tok}: {}", &n[1]), "");
            }
        }
        let mut weiter = hinten;
        while let Some(k) = KETTE.captures(weiter) {
            let mut segs: Vec<&str> = k[1].split("::").skip(1).collect();
            if segs[0] == "tests" {
                segs.remove(0);
                if segs.is_empty() {
                    segs.push("tests");
                }
            }
            let gruppe = k.get(2).map_or("", |x| x.as_str());
            let ziel = format!("{}{gruppe}", segs[segs.len() - 1]);
            let gut = if ziel == "tests" {
                self.baum.hat_modul_tests(datei)
            } else {
                self.baum.hat_name(datei, &ziel)
            };
            self.buche(Art::Test, gut, ort, &format!("{tok}::{}{gruppe}", segs.join("::")), "");
            weiter = &weiter[k.get(0).unwrap().end()..];
            match FOLGE.find(weiter) {
                Some(f) if weiter[f.end()..].starts_with("::") => weiter = &weiter[f.end()..],
                _ => break,
            }
        }
    }

    fn cache(&mut self, ort: &str, text: &str) {
        for m in CACHE.find_iter(text) {
            let tok = m.as_str().trim_end_matches(['.', ',', ';', ':', ')', '`']);
            let da = self.home.join(tok.trim_start_matches("~/")).exists();
            self.buche(Art::Cache, false, ort, tok, if da { "existiert" } else { "weg" });
        }
    }

    fn typen(&mut self, ort: &str, text: &str) {
        for m in TYP.captures_iter(text) {
            let ganz = m.get(0).unwrap();
            let name = &m[1];
            if !frei_vor(text, ganz.start(), ":") || FREMD.contains(&name) {
                continue;
            }
            let gut = VERNEINT.is_match(&text[ganz.end()..]) || self.baum.typ_da(name);
            self.buche(Art::Typ, gut, ort, ganz.as_str(), "");
        }
    }

    fn lage(&mut self, hash: &str) -> Lage {
        let git = self.git;
        *self
            .lagen
            .entry(hash.to_owned())
            .or_insert_with(|| git.lage(hash))
    }

    fn hashes(&mut self, ort: &str, text: &str) {
        for m in HASH.find_iter(text) {
            let h = m.as_str();
            let hex_mit_buchstabe_und_ziffer = h.chars().any(|c| matches!(c, 'a'..='f'))
                && h.chars().any(|c| c.is_ascii_digit());
            if !hex_mit_buchstabe_und_ziffer
                || !frei_vor(text, m.start(), "/.-")
                || !frei_nach(text, m.end())
                || steht_im_vault(text, m.start())
            {
                continue;
            }
            if !self.verlauf_pruefen {
                self.erg.ohne_verlauf += 1;
                continue;
            }
            let lage = self.lage(h);
            let klasse = match lage {
                Lage::Vorfahre => "",
                Lage::KeinVorfahre => "kein_vorfahre_von_head",
                Lage::KeinCommit => "kein_commit",
            };
            self.buche(Art::Hash, lage == Lage::Vorfahre, ort, h, klasse);
        }
    }
}

struct Lauf {
    testmap: Ergebnis,
    readme: Ergebnis,
}

impl Lauf {
    fn befunde(&self) -> usize {
        self.testmap.befunde.len() + self.readme.befunde.len()
    }

    fn bericht(&self, liste: bool) -> String {
        format!(
            "{}\n{}",
            self.testmap.bericht("TESTMAP (Spalte 7)", liste),
            self.readme.bericht("README (rust/fixtures)", liste)
        )
    }
}

/// Liest die Karte, die Abweichungsliste und den Baum unter `wurzel` und loest jeden Verweis auf.
/// `ohne_verlauf`: ein fehlender Verlauf ist erlaubt (dann zaehlt `ohne_verlauf` die ungeprueften Verweise).
fn pruefe_baum(wurzel: &Path, git: &dyn Verlauf, ohne_verlauf: bool, home: &Path) -> Lauf {
    let baum = Baum::lade(wurzel);
    let verlauf_da = git.da();
    let karte = std::fs::read_to_string(wurzel.join("rust/TESTMAP.tsv")).unwrap();
    let mut p = Pruefer::neu(&baum, git, home, verlauf_da);
    let zeilen: Vec<&str> = karte.lines().collect();
    for (i, z) in zeilen.iter().enumerate().skip(1) {
        let f: Vec<&str> = z.split('\t').collect();
        if f.len() == 7 {
            p.text(&format!("TESTMAP:{} {}", i + 1, f[0]), f[6]);
        }
    }
    let mut testmap = p.erg;
    testmap.zeilen = zeilen.len() - 1;
    if !verlauf_da && !ohne_verlauf {
        testmap.befunde.push(Befund {
            art: Art::Hash,
            klasse: "VERLAUF FEHLT".to_owned(),
            ort: "-".to_owned(),
            tok: "kein ganzer Verlauf (flacher Klon oder kein .git): der Job braucht fetch-depth: 0; \
                  ohne Verlauf nur mit TAXGRAPH_OHNE_VERLAUF=1"
                .to_owned(),
        });
    }

    let liste = std::fs::read_to_string(wurzel.join("rust/fixtures/README.md")).unwrap();
    let mut p = Pruefer::neu(&baum, git, home, verlauf_da);
    let zeilen: Vec<&str> = liste.lines().collect();
    for (i, z) in zeilen.iter().enumerate() {
        let nr = NR_IN_TABELLE
            .captures(z)
            .map_or(String::new(), |c| format!(" Nr.{}", &c[1]));
        p.text(&format!("README:{}{nr}", i + 1), z);
    }
    let mut readme = p.erg;
    readme.zeilen = zeilen.len();
    Lauf { testmap, readme }
}

fn repo_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

/// Der Ernstfall: der echte Baum, der echte Verlauf.
#[test]
fn jeder_beleg_der_testkarte_und_der_abweichungsliste_loest_auf() {
    let wurzel = repo_wurzel();
    let ohne_verlauf = std::env::var("TAXGRAPH_OHNE_VERLAUF").is_ok_and(|v| v == "1");
    let git = EchtesGit(wurzel.clone());
    let lauf = pruefe_baum(&wurzel, &git, ohne_verlauf, &home());
    let ungeprueft = lauf.testmap.ohne_verlauf + lauf.readme.ohne_verlauf;
    if !git.da() && ohne_verlauf {
        // Direktschrift an stderr: `eprintln!` faengt libtest in einem bestandenen Test ein, die CI saehe die Zeile nie.
        let _ = writeln!(
            std::io::stderr(),
            "UEBERSPRUNGEN: Verlauf fehlt, {ungeprueft} Verweise nicht geprueft, TAXGRAPH_OHNE_VERLAUF=1"
        );
    }
    if std::env::var_os("BELEGE_LISTE").is_some() {
        let _ = writeln!(std::io::stdout(), "{}", lauf.bericht(true));
    }
    assert!(
        lauf.befunde() == 0,
        "{} Verweise in rust/TESTMAP.tsv (Spalte 7) und rust/fixtures/README.md loesen nicht auf. Gueltig: ein Pfad im Baum, \
         `<commit>^:<pfad>` fuer eine geloeschte Datei, ein Vault-Pfad, ein vorhandener Testname, ein Hash auf main; nie ein Pfad im Cache.\n{}",
        lauf.befunde(),
        lauf.bericht(false)
    );
}

/// Der Job `rust` holt den ganzen Verlauf: ohne ihn waere jeder Hash und jede Verlaufsform auf dem Runner unpruefbar,
/// und der Test oben wuerde dort rot (statt vorher hier, wo der Klon voll ist).
#[test]
fn der_rust_job_holt_den_ganzen_verlauf() {
    let text = std::fs::read_to_string(repo_wurzel().join(".github/workflows/ci.yml")).unwrap();
    let wf: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
    let schritte = wf["jobs"]["rust"]["steps"]
        .as_sequence()
        .expect("Job rust ohne Schritte");
    let checkout = schritte
        .iter()
        .find(|s| s["uses"].as_str().is_some_and(|u| u.starts_with("actions/checkout")))
        .expect("Job rust ohne Checkout");
    assert_eq!(
        checkout["with"]["fetch-depth"].as_u64(),
        Some(0),
        "der Checkout im Job rust braucht `fetch-depth: 0`"
    );
}

// ---- Positivkontrollen: ein gebauter Mini-Baum und eine Attrappe fuer Git ----

struct Attrappe {
    da: bool,
    bekannt: Vec<&'static str>,
    lagen: Vec<(&'static str, Lage)>,
}

impl Verlauf for Attrappe {
    fn da(&self) -> bool {
        self.da
    }

    fn kennt(&self, rev_pfad: &str) -> bool {
        self.bekannt.contains(&rev_pfad)
    }

    fn lage(&self, hash: &str) -> Lage {
        self.lagen
            .iter()
            .find(|(k, _)| *k == hash)
            .map_or(Lage::KeinCommit, |(_, l)| *l)
    }
}

fn ohne_git() -> Attrappe {
    Attrappe {
        da: true,
        bekannt: vec![],
        lagen: vec![],
    }
}

/// Baut einen Ordner mit den Dateien (Pfad, Inhalt); der Aufrufer raeumt ihn weg.
fn probebaum(name: &str, dateien: &[(&str, &str)]) -> PathBuf {
    let wurzel = std::env::temp_dir().join(format!("belege-aufloesbar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wurzel);
    for (pfad, inhalt) in dateien {
        let p = wurzel.join(pfad);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, inhalt).unwrap();
    }
    wurzel
}

/// Prueft einen Text gegen einen Mini-Baum; liefert die Befunde als `ART/klasse tok`.
fn befunde_von(wurzel: &Path, git: &dyn Verlauf, home: &Path, text: &str) -> (Ergebnis, Vec<String>) {
    let baum = Baum::lade(wurzel);
    let mut p = Pruefer::neu(&baum, git, home, git.da());
    p.text("t", text);
    let rot = p
        .erg
        .befunde
        .iter()
        .map(|b| format!("{}/{} {}", b.art.name(), b.klasse, b.tok))
        .collect();
    (p.erg, rot)
}

fn weg(wurzel: &Path) {
    let _ = std::fs::remove_dir_all(wurzel);
}

#[test]
fn dateiverweise_loesen_ab_wurzel_ab_rust_und_als_eindeutiges_suffix_auf() {
    let w = probebaum(
        "datei",
        &[
            ("a/x.rs", ""),
            ("m/a/x.rs", ""),
            ("rust/b/y.rs", ""),
            ("docs/rust/b/y.rs", ""),
            ("rust/m/n/p.rs", ""),
            ("rust/m/o/p.rs", ""),
            ("rust/m/x/w.rs", ""),
            ("rust/n/x/w.rs", ""),
        ],
    );
    // a/x.rs: ab Wurzel (das Suffix waere zweideutig); b/y.rs: ab rust/ (das Suffix waere zweideutig);
    // n/p.rs: eindeutiges Suffix; rust/m/o/p.rs: voller Pfad; x/w.rs: zweideutig; q/weg.rs, berichte/b.md, rust/parity/t.rs: fehlen.
    let text = "a/x.rs b/y.rs n/p.rs rust/m/o/p.rs x/w.rs und q/weg.rs, berichte/b.md sowie rust/parity/t.rs";
    let (e, rot) = befunde_von(&w, &ohne_git(), &w, text);
    weg(&w);
    assert_eq!(
        rot,
        [
            "DATEI/SONST/mehrdeutig x/w.rs",
            "DATEI/SONST q/weg.rs",
            "DATEI/BERICHT berichte/b.md",
            "DATEI/L1 rust/parity/t.rs"
        ]
    );
    assert_eq!(e.geprueft[&Art::Datei], 8, "acht Dateiverweise gezaehlt, vier davon rot");
}

#[test]
fn eine_pfadangabe_hinter_einem_slash_oder_wort_ist_kein_verweis() {
    let w = probebaum("rand", &[("a/x.rs", "")]);
    // Ein Pfad hinter `~/` oder `/` oder mitten in einem Wort ist kein Verweis dieses Baums; `.css` wird nicht zu `.c` gekuerzt.
    let (e, rot) = befunde_von(&w, &ohne_git(), &w, "~/a/fremd.rs /a/fremd2.rs a/x.rss a/stil.css");
    weg(&w);
    assert_eq!(rot, ["DATEI/SONST a/stil.css"]);
    assert_eq!(e.geprueft[&Art::Datei], 1);
}

#[test]
fn die_verlaufsform_loest_nur_ueber_git_auf() {
    let w = probebaum("verlauf", &[("rust/x.rs", "")]);
    let git = Attrappe {
        da: true,
        bekannt: vec!["9f4442b1^:rust/parity/x.rs"],
        lagen: vec![("9f4442b1", Lage::Vorfahre)],
    };
    let text = "9f4442b1^:rust/parity/x.rs und 9f4442b1^:rust/parity/weg.rs und rust/parity/x.rs";
    let (e, rot) = befunde_von(&w, &git, &w, text);
    weg(&w);
    assert_eq!(
        rot,
        [
            "DATEI/VERLAUF 9f4442b1^:rust/parity/weg.rs",
            "DATEI/L1 rust/parity/x.rs"
        ]
    );
    assert_eq!(e.geprueft[&Art::Hash], 2, "der Hash in der Verlaufsform wird mitgeprueft");
}

#[test]
fn ein_hash_muss_vorfahre_von_head_sein() {
    let w = probebaum("hash", &[("rust/x.rs", "")]);
    let git = Attrappe {
        da: true,
        bekannt: vec![],
        lagen: vec![("abc1234", Lage::Vorfahre), ("def5678", Lage::KeinVorfahre)],
    };
    // abc1234 gut, def5678 nur auf einem Nebenzweig, 0123abc unbekannt; deadbeef (keine Ziffer), 12345678 (kein Buchstabe),
    // 0123456789abcdef (16 Hex: ein Inhalts-Hash), a/b-abc1234 (Teil eines Pfads) und Vault `fedcba9` (Hash des Vault-Repos) zaehlen nicht.
    let text = "abc1234 def5678 0123abc deadbeef 12345678 0123456789abcdef a/b-abc1234 Vault `fedcba9`";
    let (e, rot) = befunde_von(&w, &git, &w, text);
    weg(&w);
    assert_eq!(rot, ["HASH/kein_vorfahre_von_head def5678", "HASH/kein_commit 0123abc"]);
    assert_eq!(e.geprueft[&Art::Hash], 3);
}

#[test]
fn der_cache_ist_nie_ein_beleg_auch_wenn_die_datei_da_ist() {
    let w = probebaum("cache", &[(".cache/da.py", "")]);
    let (e, rot) = befunde_von(&w, &ohne_git(), &w, "~/.cache/da.py und ~/.cache/weg.py.");
    weg(&w);
    assert_eq!(rot, ["CACHE/existiert ~/.cache/da.py", "CACHE/weg ~/.cache/weg.py"]);
    assert_eq!(e.geprueft[&Art::Cache], 2);
}

#[test]
fn ein_testname_muss_als_fn_in_seiner_datei_stehen() {
    let w = probebaum(
        "test",
        &[
            ("rust/a/tests/t.rs", "mod tests {}\nfn eins_zwei_drei() {}\nfn rente_a() {}\nfn rente_b() {}\nfn grp_x_ende() {}\nfn grp_y_ende() {}\n"),
            ("rust/a/tests/leer.rs", "// nichts\n"),
        ],
    );
    let text = "rust/a/tests/t.rs::eins_zwei_drei, ::gibt_es_nicht und rust/a/tests/t.rs::tests::eins_zwei_drei, \
                rust/a/tests/t.rs: `eins_zwei_drei`, `fehlt_hier_leider` (Grund), \
                rust/a/tests/t.rs::rente_ und rust/a/tests/t.rs::grp_{x,y}_ende und rust/a/tests/t.rs::grp_{x,z}_ende, \
                rust/a/tests/leer.rs::tests";
    let (e, rot) = befunde_von(&w, &ohne_git(), &w, text);
    weg(&w);
    assert_eq!(
        rot,
        [
            "TEST/ rust/a/tests/t.rs::gibt_es_nicht",
            "TEST/ rust/a/tests/t.rs: fehlt_hier_leider",
            "TEST/ rust/a/tests/t.rs::grp_{x,z}_ende",
            "TEST/ rust/a/tests/leer.rs::tests"
        ]
    );
    assert_eq!(e.geprueft[&Art::Test], 9);
}

#[test]
fn ein_typ_muss_im_baum_stehen_ausser_der_satz_verneint_ihn() {
    let w = probebaum("typ", &[("rust/a/src/l.rs", "pub struct Echt {}\npub trait Eigen {}\n")]);
    let text = "Echt<T> und Eigen<Dyn> gut, Erfunden<X> schlecht, Kein Typ Art9<T> (den gibt es nicht), \
                Ersetzt den Newtype Frei<T>, den es nie gab, Option<u8> ist fremd, Erfunden2<X>.";
    let (e, rot) = befunde_von(&w, &ohne_git(), &w, text);
    weg(&w);
    assert_eq!(rot, ["TYP/ Erfunden<X>", "TYP/ Erfunden2<X>"]);
    assert_eq!(e.geprueft[&Art::Typ], 6);
}

#[test]
fn ohne_verlauf_wird_der_pruefer_laut_rot_oder_zaehlt_was_er_nicht_pruefte() {
    let w = probebaum(
        "flach",
        &[
            ("rust/TESTMAP.tsv", "file\tl\tn\tx\tc\tt\tnotiz\nrust/a.rs\t1\t1\t0\tA\tB\tAlt: 9f4442b1^:rust/parity/x.rs, Stand abc1234\n"),
            ("rust/fixtures/README.md", "| 7 | Hash abc1234 und 9f4442b1^:rust/parity/y.rs |\n"),
        ],
    );
    let flach = Attrappe { da: false, bekannt: vec![], lagen: vec![] };
    let streng = pruefe_baum(&w, &flach, false, &w);
    let nachsichtig = pruefe_baum(&w, &flach, true, &w);
    weg(&w);
    assert_eq!(streng.befunde(), 1, "flacher Klon ohne Schalter: genau ein Befund");
    assert_eq!(streng.testmap.befunde[0].klasse, "VERLAUF FEHLT");
    assert_eq!(nachsichtig.befunde(), 0);
    assert_eq!(
        nachsichtig.testmap.ohne_verlauf + nachsichtig.readme.ohne_verlauf,
        6,
        "je Hash und je Verlaufsform ein ungeprueftes Stueck: drei in der Karte, drei in der Liste"
    );
}

/// Der Pruefer liest nur die Karte, die Liste, den Baum und Git: ein Baum ohne `tests/` und `tools/` (wie nach L2) laeuft durch.
#[test]
fn der_pruefer_laeuft_auf_einem_baum_ohne_tests_und_tools() {
    let w = probebaum(
        "l2",
        &[
            ("rust/TESTMAP.tsv", "file\tl\tn\tx\tc\tt\tnotiz\nrust/a/tests/t.rs\t1\t1\t0\tA\tB\tSiehe rust/a/tests/t.rs::eins_zwei_drei und audits/x.md und 9f4442b1^:tests/test_alt.py\n"),
            ("rust/fixtures/README.md", "| 7 | Beleg rust/a/tests/t.rs und tests/test_alt.py |\n"),
            ("rust/a/tests/t.rs", "fn eins_zwei_drei() {}\n"),
        ],
    );
    assert!(!w.join("tests").exists() && !w.join("tools").exists());
    let git = Attrappe { da: true, bekannt: vec!["9f4442b1^:tests/test_alt.py"], lagen: vec![("9f4442b1", Lage::Vorfahre)] };
    let lauf = pruefe_baum(&w, &git, false, &w);
    weg(&w);
    // Rot ist nur der Verweis auf die Python-Datei, die es im L2-Baum nicht mehr gibt.
    assert_eq!(lauf.befunde(), 1, "{}", lauf.bericht(true));
    assert_eq!(lauf.readme.befunde[0].tok, "tests/test_alt.py");
    assert_eq!(lauf.testmap.vault.len(), 1);
}
