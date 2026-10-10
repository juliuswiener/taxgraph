//! Die Texte, die der Nutzer LIEST, muessen lesbar sein (Loeschplan V4, Stapel 1c, Zeilen Z11, Z7, Z12).
//! Ersatz fuer drei Python-Tests ueber `rust/bindung/daten`:
//!
//! - `tests/test_bindung_texte_lesbar.py`: keine ASCII-Umschrift (`fuer`, `Massnahme`), keine
//!   Feld-Kennung mitten im Satz an den Laien;
//! - `tests/test_fragetext_behauptet_nichts.py`: ein Fragetext faengt nicht mit "Du hast/warst/bist ..." an;
//! - `tests/test_fragetexte_anrede.py`: der Dialog duzt, kein "Sie".
//!
//! ANLASS (Python, gemessen 2026-08-14 bis 2026-08-24, Julius' echter Durchgang): "erhoehen aber den Steuersatz
//! auf dein uebriges Einkommen" im Hilfetext, den der Nutzer lesen MUSS; "Wie `kist_konfession`, aber fuer den
//! Ehe-/Lebenspartner" mitten im Satz; 10 von 219 Texten siezten; "Du warst mehr als 3 Monate am selben Ort"
//! als Frage, und der Nutzer bestaetigte mit "ja" einen Sachverhalt, den es nicht gab (`vpf_frist_nicht_
//! unterbrochen = True`). Kein Rechenfehler, aber die Software wirkt unfertig an der Stelle, an der sie Recht
//! erklaert, und eine Frage, die etwas unterstellt, schreibt eine falsche Bestaetigung in die Akte.
//!
//! Anders als in Python: die Texte kommen aus dem geparsten YAML (jeder Schluessel `fragetext_laie`,
//! `hilfe_kurz`, `frage_laie`, `titel`, `erklaerung`, an jeder Stelle des Baums), nicht aus einem Zeilen-Regex.
//! Der Zeilenscan zaehlte 25 `null`-Zeilen als Text (736 statt 711) und saehe eine Zeichenkette ueber mehrere
//! Zeilen nur in der ersten Zeile. Die Fundstelle ist die `feld_id` des umschliessenden Eintrags statt der Zeile.
//!
//! WARUM EINE LISTE UND KEIN MUSTER (Python): von 132 Woertern mit `ae/oe/ue/ss` in den Anzeigetexten sind nur 15
//! echte Umschrift; ein Muster machte aus "Steuer" "Staeuer". Darum die geprueft Liste [`VERBOTEN`] und daneben
//! das datengetriebene Netz [`umlaut_variante_im_korpus`], das Faelle faengt, die noch niemand aufgeschrieben hat
//! (steht "beruecksichtigen" neben "berücksichtigen", ist das erste Umschrift). Zwei Netze, weil keines allein
//! reicht.
//!
//! Nicht portiert: `BEHAUPTUNG_ERLAUBT` (leer; jede Ausnahme waere eine Schuld, hier ist jeder Treffer rot).
//!
//! Nicht gesehen (Grenzen): ob die Sichtbarkeit einer Frage stimmt (der Dialog stellt jede fragbare Frage, auch
//! dem Nutzer, auf den ein Satz nicht zutrifft); ob ein Text verstaendlich ist (Ermessen); die Texte, die der
//! Rust-Code selbst baut (Hinweise, Fehlermeldungen: `rust/*/src`), und ein Siezen in Texten nicht fragbarer
//! Felder. "Du hast" mitten im Satz ist erlaubt (legitime Rueckbindung an eine Antwort).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::many_single_char_names
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use serde_yaml_ng::Value;

/// Nur was der Nutzer liest. `feld_id`, `enum_werte`, `beispielwert`, `regel_id`, `anker_ref` sind WERTE: dort
/// waere eine geaenderte Schreibweise ein Datenfehler, kein Textfehler.
const ANZEIGE: [&str; 5] = [
    "fragetext_laie",
    "hilfe_kurz",
    "frage_laie",
    "titel",
    "erklaerung",
];

/// Die am 2026-08-24 gefundenen und behobenen Umschriften (Python-Liste), mit der richtigen Form. Rueckfall-Schutz.
const VERBOTEN: [(&str, &str); 16] = [
    ("Haelftelung", "Hälftelung"),
    ("Massnahme", "Maßnahme"),
    ("Sozialversicherungstraeger", "Sozialversicherungsträger"),
    ("Verhaeltnis", "Verhältnis"),
    ("Zwoelftelung", "Zwölftelung"),
    ("auswaertiger", "auswärtiger"),
    ("erhoehen", "erhöhen"),
    ("fuer", "für"),
    ("ganzjaehriger", "ganzjähriger"),
    ("heisst", "heißt"),
    ("noetig", "nötig"),
    ("ueber", "über"),
    ("uebriges", "übriges"),
    ("volljaehrigen", "volljährigen"),
    ("zustaendig", "zuständig"),
    ("zaahlen", "zählen"),
];

/// Ein Fragetext, der so BEGINNT, erzaehlt dem Nutzer, was er getan hat, statt ihn zu fragen. Bewusst auf den
/// Satzanfang beschraenkt: mitten im Satz ist "du hast" oft eine legitime Rueckbindung an eine gegebene Antwort.
const BEHAUPTUNG: &str = r"(?i)^\s*Du\s+(hast|hattest|warst|bist|wurdest|hast\s+dir|zahlst|zahltest|nutzt|nutztest|gibst|gabst|besitzt|erhieltst|bekamst)\b";

/// Hoeflichkeitsform. "Sie" nur als eigenstaendiges Wort mit grossem S: "sie" (Pronomen) und Wortteile wie
/// "diese" bleiben aussen vor.
const SIEZEN: &str = r"\b(Sie|Ihnen|Ihre|Ihren|Ihrem|Ihrer|Ihres|Ihr)\b";

fn re(muster: &str) -> Regex {
    Regex::new(muster).unwrap()
}

/// `\w+`: ein Wort (Unicode, wie Pythons `\b\w+\b`).
fn wort() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| re(r"\w+"))
}

/// `ae|oe|ue|ss` samt Grossschreibung: die Stellen, an denen eine Umschrift stehen kann.
fn umlaut_stelle() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| re(r"ae|oe|ue|ss|Ae|Oe|Ue"))
}

/// [`VERBOTEN`], je Wort als ganzes Wort.
fn verboten() -> &'static Vec<(Regex, &'static str, &'static str)> {
    static CELL: OnceLock<Vec<(Regex, &'static str, &'static str)>> = OnceLock::new();
    CELL.get_or_init(|| {
        VERBOTEN
            .iter()
            .map(|(f, r)| (re(&format!(r"\b{}\b", regex::escape(f))), *f, *r))
            .collect()
    })
}

// ------------------------------------------------------------------------------------- Sammler

/// Ein Text, den der Nutzer zu sehen bekommt.
struct Text {
    datei: String,
    /// `feld_id` des umschliessenden Eintrags, sonst der Pfad im Baum.
    stelle: String,
    schluessel: String,
    inhalt: String,
}

fn daten() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("daten")
}

/// Jede `*.yaml` unter `rust/bindung/daten`, sortiert: `(Dateiname, Inhalt)`.
fn quellen() -> &'static Vec<(String, String)> {
    static CELL: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CELL.get_or_init(|| {
        let mut pfade: Vec<PathBuf> = std::fs::read_dir(daten())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "yaml"))
            .collect();
        pfade.sort();
        pfade
            .into_iter()
            .map(|p| {
                let name = p.file_name().unwrap().to_string_lossy().into_owned();
                (name, std::fs::read_to_string(&p).unwrap())
            })
            .collect()
    })
}

fn sammle(
    wert: &Value,
    stelle: &str,
    datei: &str,
    texte: &mut Vec<Text>,
    ids: &mut BTreeSet<String>,
) {
    match wert {
        Value::Mapping(m) => {
            let hier = m
                .get("feld_id")
                .and_then(Value::as_str)
                .inspect(|id| {
                    ids.insert((*id).to_owned());
                })
                .map_or_else(|| stelle.to_owned(), str::to_owned);
            for (k, v) in m {
                let Some(schluessel) = k.as_str() else {
                    continue;
                };
                match v.as_str() {
                    Some(text) if ANZEIGE.contains(&schluessel) && !text.trim().is_empty() => {
                        texte.push(Text {
                            datei: datei.to_owned(),
                            stelle: hier.clone(),
                            schluessel: schluessel.to_owned(),
                            inhalt: text.to_owned(),
                        });
                    }
                    _ => sammle(v, &format!("{hier}/{schluessel}"), datei, texte, ids),
                }
            }
        }
        Value::Sequence(s) => {
            for (i, v) in s.iter().enumerate() {
                sammle(v, &format!("{stelle}[{i}]"), datei, texte, ids);
            }
        }
        _ => {}
    }
}

/// Alle Anzeigetexte und alle `feld_id` aus den YAML-Dateien.
fn korpus() -> &'static (Vec<Text>, BTreeSet<String>) {
    static CELL: OnceLock<(Vec<Text>, BTreeSet<String>)> = OnceLock::new();
    CELL.get_or_init(|| {
        let (mut texte, mut ids) = (Vec::new(), BTreeSet::new());
        for (datei, inhalt) in quellen() {
            let wert: Value =
                serde_yaml_ng::from_str(inhalt).unwrap_or_else(|e| panic!("{datei}: {e}"));
            sammle(&wert, datei, datei, &mut texte, &mut ids);
        }
        (texte, ids)
    })
}

fn anzeigetexte() -> &'static [Text] {
    &korpus().0
}

/// `(feld_id, Schluessel, Text)` der fragbaren Felder, Fragetext und Kurzhilfe.
fn askable_texte(schluessel: &[&str]) -> Vec<(String, String, String)> {
    let registry = bindung::lade_registry_der_wurzel(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".."),
    )
    .expect("Registry laedt");
    let mut aus = Vec::new();
    for (_, datei) in &registry.dateien {
        for b in datei.bindungen.iter().filter(|b| b.askable) {
            if schluessel.contains(&"fragetext_laie") {
                if let Some(t) = b.fragetext_laie.as_deref().filter(|t| !t.trim().is_empty()) {
                    aus.push((b.feld_id.clone(), "fragetext_laie".to_owned(), t.to_owned()));
                }
            }
            if schluessel.contains(&"hilfe_kurz") && !b.hilfe_kurz.trim().is_empty() {
                aus.push((
                    b.feld_id.clone(),
                    "hilfe_kurz".to_owned(),
                    b.hilfe_kurz.clone(),
                ));
            }
        }
    }
    aus
}

// ------------------------------------------------------------------------------------- Pruefungen

/// Verbotene Schreibweisen in `text`: `(falsch, richtig)`.
fn umschriften(text: &str) -> Vec<(&'static str, &'static str)> {
    verboten()
        .iter()
        .filter(|(muster, _, _)| muster.is_match(text))
        .map(|(_, falsch, richtig)| (*falsch, *richtig))
        .collect()
}

/// Woerter in `text`, die genau eine `feld_id` sind (Gross-/Kleinschreibung zaehlt: "Bruttoarbeitslohn" ist ein
/// Substantiv, `bruttoarbeitslohn` mitten im Satz die Kennung).
fn kennungen(text: &str, ids: &BTreeSet<String>) -> Vec<String> {
    wort()
        .find_iter(text)
        .map(|m| m.as_str())
        .filter(|w| ids.contains(*w))
        .map(str::to_owned)
        .collect()
}

fn umlaut(paar: &str) -> &'static str {
    match paar {
        "ae" => "ä",
        "oe" => "ö",
        "ue" => "ü",
        "ss" => "ß",
        "Ae" => "Ä",
        "Oe" => "Ö",
        "Ue" => "Ü",
        anders => panic!("kein Umlaut-Paar: {anders}"),
    }
}

/// Jede Schreibweise von `wort`, in der mindestens eine der hoechstens vier Stellen `ae/oe/ue/ss` einen Umlaut
/// traegt (Pythons `itertools.product`, ohne den Fall "keine ersetzt").
fn umlaut_varianten(wort: &str) -> Vec<String> {
    let stellen: Vec<usize> = umlaut_stelle().find_iter(wort).map(|m| m.start()).collect();
    if stellen.is_empty() || stellen.len() > 4 {
        return Vec::new();
    }
    (1_u32..(1 << stellen.len()))
        .map(|maske| {
            let (mut s, mut letzte) = (String::new(), 0);
            for (i, st) in stellen.iter().enumerate() {
                s.push_str(&wort[letzte..*st]);
                let paar = &wort[*st..st + 2];
                s.push_str(if (maske >> i) & 1 == 1 {
                    umlaut(paar)
                } else {
                    paar
                });
                letzte = st + 2;
            }
            s.push_str(&wort[letzte..]);
            s
        })
        .collect()
}

/// Das zweite Netz: ein Wort aus `text`, dessen Variante mit Umlaut im Korpus vorkommt: `(Wort, Variante)`.
fn umlaut_variante_im_korpus(text: &str, bekannt: &BTreeSet<&str>) -> Vec<(String, String)> {
    let mut treffer = Vec::new();
    for w in wort().find_iter(text).map(|m| m.as_str()) {
        if let Some(v) = umlaut_varianten(w)
            .into_iter()
            .find(|v| bekannt.contains(v.as_str()) && v != w)
        {
            treffer.push((w.to_owned(), v));
        }
    }
    treffer
}

// ------------------------------------------------------------------------------------- Tests

/// Untergrenze gegen den stillen Leerlauf: griffe der Sammler ins Leere (anderer Pfad, umbenannte Schluessel),
/// waeren alle Pruefungen gruen, ohne etwas gemessen zu haben. Gemessen 2026-10-06: 711 Texte (343 Fragetexte,
/// 368 Kurzhilfen), 368 `feld_id`, 26 Dateien.
#[test]
fn der_sammler_findet_texte() {
    let (texte, ids) = korpus();
    assert!(
        texte.len() > 600,
        "nur {} Anzeigetexte gefunden: Sammler pruefen",
        texte.len()
    );
    assert!(
        ids.len() > 300,
        "nur {} feld_ids gefunden: Sammler pruefen",
        ids.len()
    );
    assert!(
        quellen().len() >= 25,
        "nur {} YAML-Dateien",
        quellen().len()
    );
    for k in ["fragetext_laie", "hilfe_kurz"] {
        assert!(
            texte.iter().any(|t| t.schluessel == k),
            "kein Text unter dem Schluessel {k}"
        );
    }
}

#[test]
fn keine_ascii_umschrift_in_anzeigetexten() {
    let treffer: Vec<String> = anzeigetexte()
        .iter()
        .flat_map(|t| {
            umschriften(&t.inhalt).into_iter().map(move |(f, r)| {
                format!(
                    "{} {} ({}): {f:?} statt {r:?}",
                    t.datei, t.stelle, t.schluessel
                )
            })
        })
        .collect();
    assert!(
        treffer.is_empty(),
        "ASCII-Umschrift in Texten, die der Nutzer liest:\n  {}",
        treffer.join("\n  ")
    );
}

/// Das zweite Netz braucht keine Pflege: steht irgendwo "beruecksichtigen" und woanders "berücksichtigen", ist
/// das erste Umschrift, belegt durch das zweite. Faengt NICHT alles: "übriges" kam nur falsch vor und rutschte
/// durch. Darum zwei Netze.
#[test]
fn keine_neue_umschrift_deren_richtige_form_es_schon_gibt() {
    let alles: String = quellen()
        .iter()
        .map(|(_, s)| s.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let bekannt: BTreeSet<&str> = wort().find_iter(&alles).map(|m| m.as_str()).collect();
    assert!(
        bekannt.len() > 3000,
        "nur {} Woerter im Korpus",
        bekannt.len()
    );
    let treffer: Vec<String> = anzeigetexte()
        .iter()
        .flat_map(|t| {
            umlaut_variante_im_korpus(&t.inhalt, &bekannt)
                .into_iter()
                .map(move |(w, v)| {
                    format!(
                        "{} {} ({}): {w:?}, {v:?} steht anderswo",
                        t.datei, t.stelle, t.schluessel
                    )
                })
        })
        .collect();
    assert!(
        treffer.is_empty(),
        "Woerter, die anderswo mit Umlaut stehen:\n  {}",
        treffer.join("\n  ")
    );
}

/// "Wie `kist_konfession`, aber fuer den Ehe-/Lebenspartner": die Kennung ist ein internes Wort, der Laie hat sie
/// nie gesehen. Geprueft wird gegen die TATSAECHLICHEN `feld_id`, nicht gegen ein Unterstrich-Muster:
/// `bruttoarbeitslohn` hat keinen Unterstrich, und Pythons erster Anlauf blieb bei "Siehe bruttoarbeitslohn."
/// gruen.
#[test]
fn keine_feld_kennung_im_laientext() {
    let ids = &korpus().1;
    let treffer: Vec<String> = anzeigetexte()
        .iter()
        .flat_map(|t| {
            kennungen(&t.inhalt, ids).into_iter().map(move |k| {
                format!(
                    "{} {} ({}): Feld-Kennung {k:?}",
                    t.datei, t.stelle, t.schluessel
                )
            })
        })
        .collect();
    assert!(
        treffer.is_empty(),
        "interne Feld-Kennungen in Texten fuer den Laien:\n  {}",
        treffer.join("\n  ")
    );
}

/// Der Sweep deckt jedes kuenftige Feld mit ab. Der Dialog stellt JEDE fragbare Frage, auch dem Nutzer, auf den
/// die Behauptung nicht zutrifft; wer sie bejaht, bestaetigt etwas, das es nicht gibt. Die Bedingung gehoert in
/// `hilfe_kurz`, die Frage selbst muss fuer jeden beantwortbar sein.
#[test]
fn kein_fragetext_unterstellt_dem_nutzer_etwas() {
    let muster = re(BEHAUPTUNG);
    let texte = askable_texte(&["fragetext_laie"]);
    // Boden auf dem Objekt, das der Sweep verbraucht (Python: gemessen 340, gesetzt 300). Hier gemessen 341.
    assert!(
        texte.len() >= 300,
        "nur {} askable Fragetexte: der Sweep prueft nichts",
        texte.len()
    );
    let schaden: Vec<String> = texte
        .iter()
        .filter(|(_, _, t)| muster.is_match(t))
        .map(|(f, _, t)| format!("{f}: {}", t.chars().take(110).collect::<String>()))
        .collect();
    assert!(
        schaden.is_empty(),
        "Fragetexte behaupten einen Sachverhalt, statt ihn zu erfragen:\n  {}",
        schaden.join("\n  ")
    );
}

/// Der gefundene Fall, namentlich: ein Rueckbau faellt auf, und nicht nur der Sweep bleibt gruen.
#[test]
fn der_gefundene_fall_ist_repariert() {
    let registry = bindung::lade_registry_der_wurzel(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".."),
    )
    .expect("Registry laedt");
    let b = registry
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .find(|b| b.feld_id == "vpf_frist_nicht_unterbrochen")
        .expect("Feld verschwunden: dann gehoert dieser Test angepasst, nicht geloescht");
    let text = b.fragetext_laie.as_deref().unwrap_or("");
    assert!(
        !re(BEHAUPTUNG).is_match(text),
        "der Fragetext behauptet wieder etwas: {text:?}"
    );
    assert!(
        b.hilfe_kurz.contains("3 Monate") || b.hilfe_kurz.contains("drei Monate"),
        "die Bedingung, unter der die Frage zaehlt, muss in hilfe_kurz stehen, sonst ist sie ehrlich, aber unbeantwortbar"
    );
}

/// Der Dialog duzt durchgehend: ein Wechsel mitten im Fragebogen ist ein Bruch, den man nicht begruenden kann.
#[test]
fn kein_fragetext_siezt() {
    let muster = re(SIEZEN);
    let texte = askable_texte(&["fragetext_laie", "hilfe_kurz"]);
    // Python: >= 500 (gemessen 682). Hier gemessen 682.
    assert!(
        texte.len() >= 500,
        "nur {} askable Texte: die Anrede-Pruefung liest nichts",
        texte.len()
    );
    let treffer: Vec<String> = texte
        .iter()
        .filter_map(|(f, k, t)| {
            muster.find(t).map(|m| {
                format!(
                    "{f} ({k}): {:?} — {}",
                    m.as_str(),
                    t.chars().take(70).collect::<String>()
                )
            })
        })
        .collect();
    assert!(
        treffer.is_empty(),
        "diese Texte siezen, der Rest duzt:\n  {}",
        treffer.join("\n  ")
    );
}

// --------------------------------------------------------------------- Selbstproben der Pruefungen

/// Ohne diese Proben waeren die Tests oben vakuum-gruen, sobald jemand ein Muster zerschiesst.
#[test]
fn die_muster_greifen_ueberhaupt() {
    let behauptung = re(BEHAUPTUNG);
    assert!(behauptung.is_match("Du warst mehr als 3 Monate am selben Ort, hast aber ..."));
    assert!(behauptung.is_match("Du hast Kinder unter 14 Jahren?"));
    assert!(
        behauptung.is_match("  DU BIST verheiratet?"),
        "Gross-/Kleinschreibung und Einrueckung zaehlen nicht"
    );
    assert!(!behauptung.is_match("Lief deine Taetigkeit am selben Ort durchgehend?"));
    assert!(!behauptung.is_match("Wie viele Kilometer sind es zur Arbeit?"));
    assert!(!behauptung.is_match("Wurde dir ein Fruehstueck gestellt?"));
    assert!(
        !behauptung.is_match("Welchen Betrag hast du gezahlt?"),
        "mitten im Satz ist erlaubt"
    );

    let sie = re(SIEZEN);
    assert!(sie.is_match("Wie viel haben Sie gezahlt?"));
    assert!(sie.is_match("Steht auf der Bescheinigung Ihrer Bank"));
    assert!(!sie.is_match("Wenn sie hoeher ist, gilt der Betrag"));
    assert!(!sie.is_match("Diese Angabe steht im Bescheid"));
}

#[test]
fn die_umschrift_und_kennungspruefung_findet_ihre_faelle_und_laesst_die_richtigen_in_ruhe() {
    assert_eq!(
        umschriften("steuerfrei, erhoehen aber den Satz auf dein uebriges Einkommen"),
        [("erhoehen", "erhöhen"), ("uebriges", "übriges")]
    );
    assert_eq!(umschriften("Steuer, dass, Zuschuss, Kasse, für, über, heißt").len(), 0);
    assert_eq!(
        umschriften("Das gilt fuer dich"),
        [("fuer", "für")],
        "ganzes Wort: 'fuer' ja"
    );
    assert!(
        umschriften("Feuerwehr, Abenteuer").is_empty(),
        "Wortteil ist keine Umschrift"
    );

    let ids: BTreeSet<String> = ["bruttoarbeitslohn", "kist_konfession"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert_eq!(
        kennungen("Wie kist_konfession, aber fuer den Partner.", &ids),
        ["kist_konfession"]
    );
    assert_eq!(
        kennungen("Siehe bruttoarbeitslohn.", &ids),
        ["bruttoarbeitslohn"],
        "auch ohne Unterstrich"
    );
    assert!(
        kennungen("Wie hoch war dein Bruttoarbeitslohn?", &ids).is_empty(),
        "Substantiv, keine Kennung"
    );
    assert_eq!(kennungen("TT.MM-TT.MM und Anlage_N", &ids).len(), 0);
}

#[test]
fn das_zweite_netz_findet_eine_umschrift_deren_richtige_form_es_gibt() {
    let bekannt: BTreeSet<&str> = ["berücksichtigen", "für", "Steuer"].into_iter().collect();
    assert_eq!(
        umlaut_variante_im_korpus("Das ist zu beruecksichtigen, nicht fuer jeden", &bekannt),
        [
            ("beruecksichtigen".to_owned(), "berücksichtigen".to_owned()),
            ("fuer".to_owned(), "für".to_owned())
        ]
    );
    assert!(
        umlaut_variante_im_korpus("Die Steuer ist hoch", &bekannt).is_empty(),
        "'Steuer' bleibt Steuer"
    );
    assert!(
        umlaut_variante_im_korpus("berücksichtigen für", &bekannt).is_empty(),
        "die richtige Form ist keine Umschrift"
    );
    assert_eq!(
        umlaut_varianten("fuer"),
        ["für"],
        "eine Stelle, eine Variante"
    );
    assert_eq!(umlaut_varianten("Haelftelung").len(), 1);
    assert_eq!(
        umlaut_varianten("Masse").len(),
        1,
        "'ss' wird zu 'ß' (der Korpus entscheidet)"
    );
    assert_eq!(
        umlaut_varianten("aeoeueaess").len(),
        0,
        "mehr als vier Stellen: nicht gepruft"
    );
    assert_eq!(umlaut_varianten("Kind").len(), 0);
}
