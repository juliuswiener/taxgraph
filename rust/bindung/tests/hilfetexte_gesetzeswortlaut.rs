//! Hilfetexte und Fundstellen sagen nicht mehr und nichts anderes als der Gesetzeswortlaut.
//!
//! Entscheidung: `hilfetexte-werden-auf-den-gesetzeswortlaut-berichtigt-ohne-rueckfrage-2026-10-24`.
//! Backlog: `hilfetexte-aus-den-claims-audits-p35c-p10-sagen-mehr-als-das-gesetz` (AK1 bis AK5) und
//! `hilfetexte-rest-stufe-1-jargon-und-35a-fundstelle-ohne-pruefung` (AK1, AK2).
//!
//! ANLASS: Die Claims-Audits vom 2026-10-24 legten Erklaertexte neben den eingefrorenen Wortlaut
//! (`sources/gesetze-im-internet/`). Ein Text, der mehr verspricht als die Rechnung haelt, verleitet zu einem
//! falschen Eintrag: Energieberater-Kosten im 7-%-Feld (7 statt 50 %), ein Vertrag nach 2004 in Nr. 3a, ein
//! Kirchensteuer-Zuschlag auf Kapitalertraege im Abzug, eine Erstattung, die niemand abzieht. Kein Rechenfehler,
//! aber der Nutzer tippt, was der Text ihm sagt.
//!
//! Das Netz ist EINE Liste von Faellen (Datei, Eintrag, Textfeld, Mussteile, Verbotsteile) und keine allgemeine
//! Pruefung: ob ein Satz "stimmt", entscheidet der Wortlaut, und den liest ein Mensch. Der Test haelt fest, was
//! gelesen und berichtigt wurde, damit ein spaeterer Textwechsel es nicht still zurueckdreht. Die Mussteile sind
//! kurze Stuecke des Wortlauts (Datum, Paragraph, Satzzahl), keine ganzen Saetze.
//!
//! Nicht gesehen (Grenzen): ob ein Text VERSTAENDLICH ist (Ermessen); Hilfetexte, die niemand gegen den Wortlaut
//! gelegt hat (der Bestand hat rund 700 Anzeigetexte; diese Liste deckt die gelesenen); Texte, die der Rust-Code
//! selbst baut. Gibt es zwei Lesarten (Kirchensteuer-Zuschlag, Ausbildung im Dienstverhaeltnis), steht im Text nur
//! der Wortlaut.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use serde_yaml_ng::{Mapping, Value};

/// Was der Nutzer liest (wie in `texte_lesbar.rs`).
const ANZEIGE: [&str; 5] = [
    "fragetext_laie",
    "hilfe_kurz",
    "frage_laie",
    "titel",
    "erklaerung",
];

fn daten() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("daten")
}

/// Jede `*.yaml` unter `rust/bindung/daten`, geparst und nach Dateiname sortiert.
fn geparst() -> &'static Vec<(String, Value)> {
    static CELL: OnceLock<Vec<(String, Value)>> = OnceLock::new();
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
                let wert: Value = serde_yaml_ng::from_str(&std::fs::read_to_string(&p).unwrap())
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                (name, wert)
            })
            .collect()
    })
}

/// Alle Eintraege (Mappings) im Baum, deren `kennung` den Wert `id` hat.
fn sammle<'a>(wert: &'a Value, kennung: &str, id: &str, aus: &mut Vec<&'a Mapping>) {
    match wert {
        Value::Mapping(m) => {
            if m.get(kennung).and_then(Value::as_str) == Some(id) {
                aus.push(m);
            }
            for (_, v) in m {
                sammle(v, kennung, id, aus);
            }
        }
        Value::Sequence(s) => {
            for v in s {
                sammle(v, kennung, id, aus);
            }
        }
        _ => {}
    }
}

/// Der EINE Eintrag mit `kennung: id` in `datei`. Fehlt er oder gibt es zwei, ist das ein Fehler: ein Netz, das
/// einen umbenannten Eintrag nicht mehr findet, waere gruen.
fn eintrag(datei: &'static str, kennung: &str, id: &str) -> &'static Mapping {
    let (_, wert) = geparst()
        .iter()
        .find(|(n, _)| n == datei)
        .unwrap_or_else(|| panic!("{datei} liegt nicht unter rust/bindung/daten"));
    let mut treffer = Vec::new();
    sammle(wert, kennung, id, &mut treffer);
    assert_eq!(
        treffer.len(),
        1,
        "{datei}: {kennung} {id} muss genau einmal vorkommen, nicht {}",
        treffer.len()
    );
    treffer[0]
}

/// Der Text `feld` des Eintrags `kennung: id` in `datei`. `feld` darf ein Pfad mit `/` sein
/// (`feld_bedingung/grund`: der `grund` steht im Unterabschnitt).
fn text(datei: &'static str, kennung: &str, id: &str, feld: &str) -> String {
    let mut teile = feld.split('/');
    let erst = teile.next().unwrap_or(feld);
    let mut v = eintrag(datei, kennung, id)
        .get(erst)
        .unwrap_or_else(|| panic!("{datei}: {id} hat kein Feld {erst}"));
    for t in teile {
        v = v
            .get(t)
            .unwrap_or_else(|| panic!("{datei}: {id} hat kein Feld {t} unter {feld}"));
    }
    v.as_str()
        .unwrap_or_else(|| panic!("{datei}: {id}/{feld} ist kein Text"))
        .to_owned()
}

/// `Err` mit allen Verstoessen, wenn ein Mussteil fehlt oder ein Verbotsteil steht.
fn pruefe_text(text: &str, muss: &[&str], verboten: &[&str]) -> Result<(), String> {
    let mut fehler = Vec::new();
    for m in muss {
        if !text.contains(m) {
            fehler.push(format!("fehlt: {m:?}"));
        }
    }
    for v in verboten {
        if text.contains(v) {
            fehler.push(format!("steht noch: {v:?}"));
        }
    }
    if fehler.is_empty() {
        Ok(())
    } else {
        Err(fehler.join("; "))
    }
}

/// Ein gelesener Text.
struct Fall {
    datei: &'static str,
    kennung: &'static str,
    id: &'static str,
    feld: &'static str,
    muss: &'static [&'static str],
    verboten: &'static [&'static str],
}

fn pruefe_faelle(faelle: &[Fall]) {
    assert!(!faelle.is_empty(), "eine leere Liste waere gruen");
    let mut rot = Vec::new();
    for f in faelle {
        let t = text(f.datei, f.kennung, f.id, f.feld);
        if let Err(e) = pruefe_text(&t, f.muss, f.verboten) {
            rot.push(format!(
                "{} {} {}: {e}\n    Text: {t}",
                f.datei, f.id, f.feld
            ));
        }
    }
    assert!(
        rot.is_empty(),
        "{} von {} Texten weichen vom Wortlaut ab:\n{}",
        rot.len(),
        faelle.len(),
        rot.join("\n")
    );
}

const FELD_ID: &str = "feld_id";

// ------------------------------------------------------------------------------- Kontrolle des Netzes

/// Das Netz schlaegt an: ein fehlender Mussteil und ein stehengebliebener Verbotsteil werden beide gemeldet.
#[test]
fn die_pruefung_meldet_fehlende_und_verbotene_teile() {
    assert!(pruefe_text("a b c", &["a", "c"], &["x"]).is_ok());
    let fehlt = pruefe_text("a b", &["a", "c"], &[]).unwrap_err();
    assert!(fehlt.contains("fehlt: \"c\""), "{fehlt}");
    let steht = pruefe_text("a b", &[], &["b"]).unwrap_err();
    assert!(steht.contains("steht noch: \"b\""), "{steht}");
}

// ------------------------------------------------------------------------------- § 35c

/// § 35c Abs. 1 S. 4: Die Kosten des Energieberaters haben ein EIGENES Feld (50 %); im Sanierungsfeld (7-%-Staffel)
/// zaehlen sie nicht. Der Text darf sie dort nicht als Teil der Kosten nennen.
#[test]
fn das_sanierungsfeld_verweist_die_energieberater_kosten_an_ihr_eigenes_feld() {
    pruefe_faelle(&[Fall {
        datei: "bindung_p35c_sanierung.yaml",
        kennung: FELD_ID,
        id: "p35c_sanierungsaufwendungen",
        feld: "hilfe_kurz",
        muss: &[
            "Energieberater",
            "trägst du nicht hier ein",
            "50 Prozent",
            "§ 35c Abs. 1 S. 4 EStG",
        ],
        verboten: &["ggfs. BAFA-Energieberater-Kosten"],
    }]);
}

/// Die Dateikoepfe nennen "Abs.1a" (so einen Absatz hat § 35c nicht); die Stelle ist Abs. 1 Satz 4.
#[test]
fn die_kopfkommentare_der_35c_dateien_nennen_abs_1_satz_4() {
    for datei in [
        "bindung_p35c_sanierung.yaml",
        "bindung_p35c_energieberater.yaml",
    ] {
        let roh = std::fs::read_to_string(daten().join(datei)).unwrap();
        assert!(!roh.contains("Abs.1a"), "{datei} nennt noch Abs.1a");
        assert!(
            roh.contains("Abs. 1 S. 4"),
            "{datei} nennt Abs. 1 S. 4 nicht"
        );
    }
}

// ------------------------------------------------------------------------------- § 10 Abs. 1 Nr. 3a

/// § 10 Abs. 1 Nr. 3a `EStG`: Beitraege zu den Altvertraegen zaehlen nur, wenn die Laufzeit vor dem 1. Januar 2005
/// begonnen hat und bis zum 31. Dezember 2004 ein Beitrag gezahlt wurde. Frage und Hilfetext nennen die Bedingung.
#[test]
fn nr_3a_altvertraege_nennen_die_bedingung_des_gesetzes() {
    let mut faelle = Vec::new();
    for id in [
        "vorsorge_rv_alt_mit_ueberschuss",
        "vorsorge_rv_alt_ohne_ueberschuss",
    ] {
        faelle.push(Fall {
            datei: "bindung_n_vor_gwg.yaml",
            kennung: FELD_ID,
            id,
            feld: "fragetext_laie",
            muss: &["vor dem 1. Januar 2005"],
            verboten: &[],
        });
        faelle.push(Fall {
            datei: "bindung_n_vor_gwg.yaml",
            kennung: FELD_ID,
            id,
            feld: "hilfe_kurz",
            muss: &[
                "vor dem 1. Januar 2005",
                "31. Dezember 2004",
                "§ 10 Abs. 1 Nr. 3a EStG",
            ],
            verboten: &["Riester-ähnliche"],
        });
    }
    pruefe_faelle(&faelle);
}

// ------------------------------------------------------------------------------- § 10 Abs. 1 Nr. 4, Abs. 4b

/// § 10 Abs. 1 Nr. 4 `EStG`: "dies gilt nicht, soweit die Kirchensteuer als Zuschlag zur Kapitalertragsteuer oder als
/// Zuschlag auf die nach dem gesonderten Tarif des § 32d Absatz 1 ermittelte Einkommensteuer gezahlt wurde".
/// Der Text nennt nur den Wortlaut, keine Lesart, aus welchem Beleg das Feld gespeist wird.
#[test]
fn nr_4_kirchensteuer_nennt_den_zuschlag_zur_kapitalertragsteuer() {
    pruefe_faelle(&[Fall {
        datei: "bindung_sonder_agb_35a.yaml",
        kennung: FELD_ID,
        id: "kist_gezahlt",
        feld: "hilfe_kurz",
        muss: &[
            "Zuschlag zur Kapitalertragsteuer",
            "§ 32d Abs. 1",
            "§ 10 Abs. 1 Nr. 4 EStG",
        ],
        verboten: &[],
    }]);
}

/// Die erstattete Kirchensteuer (Nr. 4) steht in § 10 Abs. 4b SATZ 3 ("Nummer 3 und 4"); Satz 2 nennt Nr. 2 bis 3a.
#[test]
fn kist_erstattet_nennt_abs_4b_satz_3_als_fundstelle() {
    let m = eintrag("bindung_sonder_agb_35a.yaml", FELD_ID, "kist_erstattet");
    let anker = m.get("anker_ref").expect("anker_ref");
    let quelle = anker.get("quelle").and_then(Value::as_str).unwrap();
    let zitat = anker.get("zitatanker").and_then(Value::as_str).unwrap();
    assert_eq!(quelle, "§ 10 Abs. 4b Satz 3 EStG");
    assert!(
        zitat.contains("Nummer 3 und 4"),
        "der Zitatanker muss die Nummer 3 und 4 aus Satz 3 nennen: {zitat}"
    );
}

/// § 10 Abs. 4b `EStG`: erstattete Beitraege werden mit den gezahlten verrechnet. "Trage den Beitrag so ein, wie
/// dort steht." allein liess die Rueckerstattung stehen; die vier Beitragsfelder (Person und Partner, KV und PV)
/// nennen sie jetzt.
#[test]
fn die_beitragsfelder_nennen_die_erstattung_nach_abs_4b() {
    let mut faelle = Vec::new();
    for (datei, id) in [
        ("bindung_n_vor_gwg.yaml", "basis_kv"),
        ("bindung_n_vor_gwg.yaml", "basis_pv"),
        ("bindung_an_gesamt.yaml", "basis_kv_partner"),
        ("bindung_an_gesamt.yaml", "basis_pv_partner"),
    ] {
        faelle.push(Fall {
            datei,
            kennung: FELD_ID,
            id,
            feld: "hilfe_kurz",
            muss: &["erstatteter Beiträge", "§ 10 Abs. 4b EStG"],
            verboten: &["wie dort steht."],
        });
    }
    pruefe_faelle(&faelle);
}

// ------------------------------------------------------------------------------- § 10 Abs. 1 Nr. 7, § 9 Abs. 6

/// § 9 Abs. 6 S. 1 `EStG`: Kosten einer Berufsausbildung oder eines Studiums sind nur Werbungskosten, wenn schon eine
/// Erstausbildung abgeschlossen ist oder die Ausbildung im Rahmen eines Dienstverhaeltnisses stattfindet. Beide
/// Texte zur Berufsausbildung nennen den Wortlaut und sagen, dass dieselben Kosten nicht zweimal angesetzt werden.
#[test]
fn die_ausbildungstexte_nennen_paragraph_9_absatz_6_und_das_dienstverhaeltnis() {
    pruefe_faelle(&[
        Fall {
            datei: "bindung_screening_ausgaben.yaml",
            kennung: FELD_ID,
            id: "keine_berufsausbildung",
            feld: "hilfe_kurz",
            muss: &[
                "Dienstverhältnis",
                "§ 9 Abs. 6 EStG",
                "nicht noch einmal ein",
            ],
            verboten: &[],
        },
        Fall {
            datei: "bindung_sonder_agb_35a.yaml",
            kennung: FELD_ID,
            id: "berufsausbildung_aufwendungen",
            feld: "hilfe_kurz",
            muss: &[
                "Dienstverhältnis",
                "§ 9 Abs. 6 EStG",
                "nicht noch einmal ein",
            ],
            verboten: &[],
        },
    ]);
}

// ------------------------------------------------------------------------------- Projektjargon, § 35a

/// Kein Projektjargon aus der Bauzeit in einem Text, den der Nutzer liest. Kommentare und `grund`-Zeilen
/// (Entwicklersprache) bleiben aussen vor.
#[test]
fn kein_projektjargon_im_nutzertext() {
    fn durchsuche(wert: &Value, re: &Regex, stelle: &str, funde: &mut Vec<String>) {
        match wert {
            Value::Mapping(m) => {
                let hier = m
                    .get(FELD_ID)
                    .and_then(Value::as_str)
                    .map_or_else(|| stelle.to_owned(), str::to_owned);
                for (k, v) in m {
                    let Some(schluessel) = k.as_str() else {
                        continue;
                    };
                    match v.as_str() {
                        Some(t) if ANZEIGE.contains(&schluessel) => {
                            if let Some(treffer) = re.find(t) {
                                funde.push(format!("{hier}/{schluessel}: {:?}", treffer.as_str()));
                            }
                        }
                        _ => durchsuche(v, re, &format!("{hier}/{schluessel}"), funde),
                    }
                }
            }
            Value::Sequence(s) => {
                for (i, v) in s.iter().enumerate() {
                    durchsuche(v, re, &format!("{stelle}[{i}]"), funde);
                }
            }
            _ => {}
        }
    }
    let re = Regex::new(r"Stufe-?[12]|\bMVP\b|fail-closed").unwrap();
    let mut funde = Vec::new();
    for (datei, wert) in geparst() {
        durchsuche(wert, &re, datei, &mut funde);
    }
    assert!(
        funde.is_empty(),
        "Projektjargon im Nutzertext:\n{}",
        funde.join("\n")
    );
}

/// Die Kontrolle zum Jargon-Netz: der Nutzertext der DBA-Frage wurde berichtigt, er sagt "Staat" statt "Stufe-1".
#[test]
fn die_dba_hilfe_nennt_den_einen_staat_in_alltagssprache() {
    pruefe_faelle(&[Fall {
        datei: "bindung_p34c_gesamt.yaml",
        kennung: FELD_ID,
        id: "dba_mehrere_staaten",
        feld: "hilfe_kurz",
        muss: &["nur für EINEN Staat", "Steuerberatung"],
        verboten: &["Stufe-1"],
    }]);
}

/// § 35a Abs. 2 und 3 `EStG` geben 20 % der AUFWENDUNGEN; auf Arbeitskosten beschraenkt erst Abs. 5 Satz 2. Die
/// Begruendung der beiden Zaehlgruppen nennt beide Stellen.
#[test]
fn die_35a_gruppen_nennen_abs_5_satz_2_fuer_die_arbeitskosten() {
    let mut faelle = Vec::new();
    for id in ["hh_handwerker", "hh_dienstleistung"] {
        faelle.push(Fall {
            datei: "bindung_regel_bedingungen.yaml",
            kennung: "gruppe",
            id,
            feld: "grund",
            muss: &["Abs. 5 S. 2", "Arbeitskosten"],
            verboten: &["um 20 % der Arbeitskosten"],
        });
    }
    pruefe_faelle(&faelle);
}

// ------------------------------------------------------------------------------- Satz der Fundstelle

/// § 33 Abs. 2a `EStG`: S. 1 ordnet die Pauschale an, S. 2 nennt, wer sie erhaelt (Grad der Behinderung von
/// mindestens 80 oder mindestens 70 mit Merkzeichen G; Merkzeichen aG, Bl, `TBl` oder H).
#[test]
fn der_grund_der_fahrtkostenpauschale_nennt_satz_2_und_die_grenze_70() {
    pruefe_faelle(&[Fall {
        datei: "bindung_regel_bedingungen.yaml",
        kennung: "regel_id",
        id: "p33_2a_fahrtkostenpauschale",
        feld: "grund",
        muss: &["Abs. 2a S. 2", "mindestens 70"],
        verboten: &["Abs. 2a S. 1", "ab 80"],
    }]);
}

/// § 33b Abs. 3 `EStG`: S. 3 gewaehrt den erhoehten Pauschbetrag (Hilflose, Blinde, Taubblinde), S. 4 erklaert nur
/// "hilflos". Die Merkzeichen H, Bl, `TBl` stehen nicht im Wortlaut und gehoeren nicht in den Grund.
#[test]
fn der_grund_der_hilflos_frage_nennt_satz_3_fuer_den_erhoehten_pauschbetrag() {
    pruefe_faelle(&[Fall {
        datei: "bindung_rentner.yaml",
        kennung: FELD_ID,
        id: "rentner_hilflos_blind_taubblind_partner",
        feld: "feld_bedingung/grund",
        muss: &["Abs. 3 S. 3", "im Sinne von S. 4"],
        verboten: &["Abs. 3 S. 4", "Merkzeichen"],
    }]);
}
