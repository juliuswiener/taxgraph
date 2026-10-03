//! Rust (`engine`, Teil A der ~30 Catala-Aufrufe aus Schritt 4a) gegen Python
//! (`tools/parity/oracle.py`, ueber `produkt/engine/runner.py`). `REWRITE_PLAN.md` §5.
//!
//! Zwei Nachweiswege je Funktion:
//! 1. Corpus-Replay: echte `(fn, args)`-Faelle aus einem Testlauf, aufgezeichnet von
//!    `tools/parity/record.py` nach `rust/fixtures/corpus/engine_calls*.jsonl`.
//! 2. Proptest: 1000 generierte Faelle je Funktion gegen dasselbe Orakel.
//!
//! `altersentlastungsbetrag` (§ 24a) faellt aus dem Corpus-Replay: der runner.py-Wrapper leitet
//! `prozentsatz`/`hoechstbetrag` selbst aus einer Kohorten-YAML ab (Teil B, kein 1:1-Pass-through
//! der rohen Catala-Scope-Eingaben, siehe `tools/parity/record.py`s Doc-Kommentar) -- Paritaet
//! deckt hier allein der Proptest gegen den rohen Scope ab.
//!
//! Braucht den Catala-Opam-Switch + `python3` mit dem Repo-Umfeld -- in CI standardmaessig SKIP:
//!
//!   `PARITY`=1 `cargo` test -p parity --test `engine_catala_paritaet` -- --nocapture
#![allow(
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use domain::{Cent, Satz};
use engine::agb::{self, AgbAbzugEingabe};
use engine::altersentlastungsbetrag::{self, AltersentlastungsbetragEingabe};
use engine::berufsausbildung::{self, BerufsausbildungEingabe};
use engine::betriebsfreibetrag::{self, BetriebsFreibetragEingabe};
use engine::entlastungsbetrag::{self, EntlastungsbetragEingabe};
use engine::euer::{self, EuerEingabe};
use engine::familienleistungsausgleich::{self, FamilienleistungsausgleichEingabe};
use engine::fuenftelregelung::{self, FuenftelregelungEingabe};
use engine::gwg::{self, GwgEingabe};
use engine::kirchensteuer::{self, KirchensteuerabzugEingabe};
use engine::mitunternehmer::{self, MitunternehmerEingabe};
use engine::spenden::{self, SpendenAbzugEingabe};
use engine::verbilligte_vermietung::{self, VerbilligteVermietungEingabe};
use engine::verlustvortrag::{self, VerlustvortragEingabe};
use engine::vorsorgeaufwendungen::{self, VorsorgeaufwendungenEingabe};
use engine::zumutbare_belastung::{self, ZumutbareBelastungEingabe};
use parity::{diff_engine, EngineAbweichung, Oracle};
use proptest::prelude::*;
use rust_decimal::Decimal;
use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip_ohne_parity_env() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

/// EIN Orakel-Prozess fuer die gesamte Testdatei statt eines Neustarts je Fall. `oracle.py`
/// importiert bei jedem Start das komplette Catala-Laufzeitsystem (Sekunden); bei 15 Funktionen
/// x 1000 Proptest-Faellen waere ein Neustart je Fall (wie `tarif_paritaet.rs`s 2 Funktionen es
/// sich leisten) unbrauchbar langsam. Der `Mutex` serialisiert Testthreads auf diesen einen
/// Prozess -- dieselbe Rolle wie `parity::ORACLE_LOCK`, nur um eine tatsaechlich geteilte
/// Instanz statt nur um sich ueberschneidende Spawns.
fn oracle() -> MutexGuard<'static, Oracle> {
    static ORACLE: OnceLock<Mutex<Oracle>> = OnceLock::new();
    ORACLE
        .get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn diff(funktion: &str, args: Value, rust_cent: i64) -> Option<EngineAbweichung> {
    diff_engine(&mut oracle(), funktion, args, rust_cent).expect("Orakel-Aufruf laeuft durch")
}

fn cent(args: &Value, key: &str) -> Cent {
    Cent::new(
        args[key]
            .as_i64()
            .unwrap_or_else(|| panic!("{key} fehlt oder ist kein i64")),
    )
}

fn i64f(args: &Value, key: &str) -> i64 {
    args[key]
        .as_i64()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist kein i64"))
}

fn boolf(args: &Value, key: &str) -> bool {
    args[key]
        .as_bool()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist kein bool"))
}

/// § 21 Abs. 2 `EStG` und § 24a `EStG` kodieren ihren Prozentsatz im Corpus als `_num`/`_den`
/// (siehe `tools/parity/record.py`/`oracle.py`), symmetrisch zu [`engine::dezimal::zu_bruch`].
fn prozent_von(args: &Value, feld: &str) -> Satz {
    let num = i64f(args, &format!("{feld}_num"));
    let den = args[format!("{feld}_den")]
        .as_u64()
        .unwrap_or_else(|| panic!("{feld}_den fehlt"));
    Satz::new(Decimal::from(num) / Decimal::from(den))
}

/// Rechnet EINEN Corpus-/Proptest-Fall ueber die passende `engine`-Funktion, in Cent.
///
/// # Panics
/// Wenn `funktion` unbekannt ist oder der zugehoerige Catala-Scope eine Assertion verletzt --
/// beides waere im Corpus-Replay (reale, bereits einmal erfolgreich gelaufene Faelle) ein Befund,
/// kein erwarteter Pfad.
fn rust_cent_fuer(funktion: &str, args: &Value) -> i64 {
    // Jeder Arm ruft `.unwrap_or_else(...).get()` selbst auf: die `engine`-Module haben je einen
    // EIGENEN Fehlertyp (`CatalaFehler` bzw. ein Verbund mit `DezimalUeberlauf`), ein `match` mit
    // Ergebnissen erst NACH dem `match` aufzuloesen bräuchte also einen gemeinsamen Fehlertyp, den
    // es nicht gibt.
    match funktion {
        "spenden_abzug" => spenden::berechnen(SpendenAbzugEingabe {
            zuwendungen: cent(args, "zuwendungen_cent"),
            gesamtbetrag_der_einkuenfte: cent(args, "gesamtbetrag_der_einkuenfte_cent"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "zumutbare_belastung" => zumutbare_belastung::berechnen(ZumutbareBelastungEingabe {
            gesamtbetrag_der_einkuenfte: cent(args, "gesamtbetrag_der_einkuenfte_cent"),
            anzahl_kinder: i64f(args, "anzahl_kinder"),
            splitting: boolf(args, "splitting"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "agb_abzug" => agb::berechnen(AgbAbzugEingabe {
            aussergewoehnliche_belastungen: cent(args, "aussergewoehnliche_belastungen_cent"),
            zumutbare_belastung: cent(args, "zumutbare_belastung_cent"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "kirchensteuerabzug" => kirchensteuer::berechnen(KirchensteuerabzugEingabe {
            gezahlte_kirchensteuer: cent(args, "gezahlte_kirchensteuer_cent"),
            erstattete_kirchensteuer: cent(args, "erstattete_kirchensteuer_cent"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "entlastungsbetrag" => entlastungsbetrag::berechnen(EntlastungsbetragEingabe {
            alleinstehend: boolf(args, "alleinstehend"),
            anzahl_kinder: i64f(args, "anzahl_kinder"),
            monate_ohne_voraussetzung: i64f(args, "monate_ohne_voraussetzung"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "familienleistungsausgleich" => {
            familienleistungsausgleich::berechnen(FamilienleistungsausgleichEingabe {
                est_ohne_freibetraege: cent(args, "est_ohne_freibetraege_cent"),
                est_mit_freibetraegen: cent(args, "est_mit_freibetraegen_cent"),
                kindergeld: cent(args, "kindergeld_cent"),
            })
            .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
            .get()
        }
        "verbilligte_vermietung_wk" => {
            verbilligte_vermietung::berechnen(VerbilligteVermietungEingabe {
                werbungskosten: cent(args, "werbungskosten_cent"),
                entgelt_quote_prozent: prozent_von(args, "entgelt_quote_prozent"),
            })
            .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
            .get()
        }
        "kranken_pflege_vorsorge" => vorsorgeaufwendungen::berechnen(VorsorgeaufwendungenEingabe {
            basis: cent(args, "basis_cent"),
            weitere: cent(args, "weitere_cent"),
            mit_zuschuss: boolf(args, "mit_zuschuss"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "berufsausbildung" => berufsausbildung::berechnen(BerufsausbildungEingabe {
            aufwendungen: cent(args, "aufwendungen_cent"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "betriebs_freibetrag" => betriebsfreibetrag::berechnen(BetriebsFreibetragEingabe {
            veraeusserungsgewinn: cent(args, "veraeusserungsgewinn_cent"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "euer_gewinn" => euer::berechnen(EuerEingabe {
            betriebseinnahmen: cent(args, "betriebseinnahmen_cent"),
            betriebsausgaben: cent(args, "betriebsausgaben_cent"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "verlustvortrag_abzug" => verlustvortrag::berechnen(VerlustvortragEingabe {
            gesamtbetrag_einkuenfte: cent(args, "gesamtbetrag_einkuenfte_cent"),
            verlustvortrag_bestand: cent(args, "verlustvortrag_bestand_cent"),
            zusammenveranlagung: boolf(args, "zusammenveranlagung"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "mitunternehmer_einkuenfte" => mitunternehmer::berechnen(MitunternehmerEingabe {
            gewinnanteil: cent(args, "gewinnanteil_cent"),
            verguetung_taetigkeit: cent(args, "verguetung_taetigkeit_cent"),
            verguetung_darlehen: cent(args, "verguetung_darlehen_cent"),
            verguetung_ueberlassung: cent(args, "verguetung_ueberlassung_cent"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "gwg_sofortabzug" => gwg::berechnen(GwgEingabe {
            anschaffungskosten_netto: cent(args, "anschaffungskosten_netto_cent"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        "ermaessigter_durchschnittssatz" => fuenftelregelung::berechnen(FuenftelregelungEingabe {
            ao_einkuenfte: cent(args, "ao_einkuenfte_cent"),
            est_gesamt_zzgl_progression: cent(args, "est_gesamt_zzgl_progression_cent"),
            bemessungsgrundlage_durchschnitt: cent(args, "bemessungsgrundlage_durchschnitt_cent"),
        })
        .unwrap_or_else(|e| panic!("{funktion}: {e:?}"))
        .get(),
        andere => panic!("Corpus-Replay: unbekannte oder Teil-B-Funktion {andere}"),
    }
}

fn corpus_dateien() -> Vec<PathBuf> {
    let dir = repo_root().join("rust/fixtures/corpus");
    let mut treffer = Vec::new();
    let Ok(eintraege) = std::fs::read_dir(&dir) else {
        return treffer;
    };
    for eintrag in eintraege.flatten() {
        let name = eintrag.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("engine_calls") && name.ends_with(".jsonl") {
            treffer.push(eintrag.path());
        }
    }
    treffer
}

#[derive(serde::Deserialize)]
struct Fall {
    #[serde(rename = "fn")]
    funktion: String,
    args: Value,
}

/// Ersetzt `record.py`s Aufzeichnung eines echten Testlaufs durch einen erneuten Durchlauf durch
/// beide Implementierungen. Ein leeres Corpus waere ein stilles GRUEN ohne Beweis (vgl.
/// CLAUDE.md "Backlog-Eintrag aelter als sein eigener Fix" / "0 Dateien geprueft") -- deshalb die
/// explizite `faelle_gesamt > 0`-Assertion statt nur ueber eine leere Liste zu iterieren.
#[test]
fn engine_corpus_replay() {
    if skip_ohne_parity_env() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht Catala-Toolchain + Python-Umfeld)");
        return;
    }
    let dateien = corpus_dateien();
    let mut faelle_gesamt = 0u64;
    let mut abweichungen = Vec::new();

    for datei in &dateien {
        let inhalt = std::fs::read_to_string(datei).expect("Corpus-Datei lesbar");
        for zeile in inhalt.lines() {
            if zeile.trim().is_empty() {
                continue;
            }
            let fall: Fall = serde_json::from_str(zeile).expect("Corpus-Zeile ist JSON");
            let rust_cent = rust_cent_fuer(&fall.funktion, &fall.args);
            if let Some(a) = diff(&fall.funktion, fall.args.clone(), rust_cent) {
                abweichungen.push(a);
            }
            faelle_gesamt += 1;
        }
    }

    eprintln!(
        "engine_corpus_replay: {faelle_gesamt} Faelle aus {} Datei(en), {} Abweichungen",
        dateien.len(),
        abweichungen.len()
    );
    assert!(
        faelle_gesamt > 0,
        "Corpus leer ({} Datei(en) unter rust/fixtures/corpus/engine_calls*.jsonl) -- \
         `python3 -m pytest tests/ -p tools.parity.record -q` zuerst laufen lassen",
        dateien.len()
    );
    assert!(abweichungen.is_empty(), "Abweichungen: {abweichungen:?}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(parity::fallzahl::holen_u32(
        "engine_catala_paritaet proptest",
        1000,
    )))]

    #[test]
    fn spenden_abzug_paritaet(zuwendungen in 0i64..=10_000_000i64, gesamtbetrag in 0i64..=50_000_000i64) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = spenden::berechnen(SpendenAbzugEingabe {
            zuwendungen: Cent::new(zuwendungen),
            gesamtbetrag_der_einkuenfte: Cent::new(gesamtbetrag),
        }).expect("Spenden-Scope laeuft durch").get();
        let args = serde_json::json!({
            "zuwendungen_cent": zuwendungen,
            "gesamtbetrag_der_einkuenfte_cent": gesamtbetrag,
        });
        prop_assert_eq!(diff("spenden_abzug", args, rust_cent), None);
    }

    #[test]
    fn zumutbare_belastung_paritaet(
        gesamtbetrag in 0i64..=50_000_000i64,
        anzahl_kinder in 0i64..=8i64,
        splitting in any::<bool>(),
    ) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = zumutbare_belastung::berechnen(ZumutbareBelastungEingabe {
            gesamtbetrag_der_einkuenfte: Cent::new(gesamtbetrag),
            anzahl_kinder,
            splitting,
        }).expect("ZumutbareBelastung-Scope laeuft durch").get();
        let args = serde_json::json!({
            "gesamtbetrag_der_einkuenfte_cent": gesamtbetrag,
            "anzahl_kinder": anzahl_kinder,
            "splitting": splitting,
        });
        prop_assert_eq!(diff("zumutbare_belastung", args, rust_cent), None);
    }

    #[test]
    fn agb_abzug_paritaet(agb in 0i64..=10_000_000i64, zumutbar in 0i64..=5_000_000i64) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = agb::berechnen(AgbAbzugEingabe {
            aussergewoehnliche_belastungen: Cent::new(agb),
            zumutbare_belastung: Cent::new(zumutbar),
        }).expect("AgbAbzug-Scope laeuft durch").get();
        let args = serde_json::json!({
            "aussergewoehnliche_belastungen_cent": agb,
            "zumutbare_belastung_cent": zumutbar,
        });
        prop_assert_eq!(diff("agb_abzug", args, rust_cent), None);
    }

    #[test]
    fn kirchensteuerabzug_paritaet(gezahlt in 0i64..=5_000_000i64, erstattet in 0i64..=5_000_000i64) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = kirchensteuer::berechnen(KirchensteuerabzugEingabe {
            gezahlte_kirchensteuer: Cent::new(gezahlt),
            erstattete_kirchensteuer: Cent::new(erstattet),
        }).expect("Kirchensteuerabzug-Scope laeuft durch").get();
        let args = serde_json::json!({
            "gezahlte_kirchensteuer_cent": gezahlt,
            "erstattete_kirchensteuer_cent": erstattet,
        });
        prop_assert_eq!(diff("kirchensteuerabzug", args, rust_cent), None);
    }

    #[test]
    fn entlastungsbetrag_paritaet(
        alleinstehend in any::<bool>(),
        anzahl_kinder in 0i64..=8i64,
        monate_ohne_voraussetzung in 0i64..=12i64,
    ) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = entlastungsbetrag::berechnen(EntlastungsbetragEingabe {
            alleinstehend,
            anzahl_kinder,
            monate_ohne_voraussetzung,
        }).expect("Entlastungsbetrag-Scope laeuft durch").get();
        let args = serde_json::json!({
            "alleinstehend": alleinstehend,
            "anzahl_kinder": anzahl_kinder,
            "monate_ohne_voraussetzung": monate_ohne_voraussetzung,
        });
        prop_assert_eq!(diff("entlastungsbetrag", args, rust_cent), None);
    }

    #[test]
    fn familienleistungsausgleich_paritaet(
        est_ohne in 0i64..=20_000_000i64,
        est_mit in 0i64..=20_000_000i64,
        kindergeld in 0i64..=1_000_000i64,
    ) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = familienleistungsausgleich::berechnen(FamilienleistungsausgleichEingabe {
            est_ohne_freibetraege: Cent::new(est_ohne),
            est_mit_freibetraegen: Cent::new(est_mit),
            kindergeld: Cent::new(kindergeld),
        }).expect("Familienleistungsausgleich-Scope laeuft durch").get();
        let args = serde_json::json!({
            "est_ohne_freibetraege_cent": est_ohne,
            "est_mit_freibetraegen_cent": est_mit,
            "kindergeld_cent": kindergeld,
        });
        prop_assert_eq!(diff("familienleistungsausgleich", args, rust_cent), None);
    }

    #[test]
    fn verbilligte_vermietung_paritaet(werbungskosten in 0i64..=5_000_000i64, prozent in 0i64..=100i64) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = verbilligte_vermietung::berechnen(VerbilligteVermietungEingabe {
            werbungskosten: Cent::new(werbungskosten),
            entgelt_quote_prozent: Satz::new(Decimal::new(prozent, 0)),
        }).expect("VerbilligteVermietungWk-Scope laeuft durch").get();
        let args = serde_json::json!({
            "werbungskosten_cent": werbungskosten,
            "entgelt_quote_prozent_num": prozent,
            "entgelt_quote_prozent_den": 1,
        });
        prop_assert_eq!(diff("verbilligte_vermietung_wk", args, rust_cent), None);
    }

    #[test]
    fn kranken_pflege_vorsorge_paritaet(
        basis in 0i64..=5_000_000i64,
        weitere in 0i64..=5_000_000i64,
        mit_zuschuss in any::<bool>(),
    ) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = vorsorgeaufwendungen::berechnen(VorsorgeaufwendungenEingabe {
            basis: Cent::new(basis),
            weitere: Cent::new(weitere),
            mit_zuschuss,
        }).expect("KrankenPflegeVorsorge-Scope laeuft durch").get();
        let args = serde_json::json!({
            "basis_cent": basis,
            "weitere_cent": weitere,
            "mit_zuschuss": mit_zuschuss,
        });
        prop_assert_eq!(diff("kranken_pflege_vorsorge", args, rust_cent), None);
    }

    #[test]
    fn berufsausbildung_paritaet(aufwendungen in 0i64..=2_000_000i64) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = berufsausbildung::berechnen(BerufsausbildungEingabe {
            aufwendungen: Cent::new(aufwendungen),
        }).expect("Berufsausbildung-Scope laeuft durch").get();
        let args = serde_json::json!({ "aufwendungen_cent": aufwendungen });
        prop_assert_eq!(diff("berufsausbildung", args, rust_cent), None);
    }

    #[test]
    fn betriebs_freibetrag_paritaet(veraeusserungsgewinn in 0i64..=30_000_000i64) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = betriebsfreibetrag::berechnen(BetriebsFreibetragEingabe {
            veraeusserungsgewinn: Cent::new(veraeusserungsgewinn),
        }).expect("BetriebsFreibetrag-Scope laeuft durch").get();
        let args = serde_json::json!({ "veraeusserungsgewinn_cent": veraeusserungsgewinn });
        prop_assert_eq!(diff("betriebs_freibetrag", args, rust_cent), None);
    }

    #[test]
    fn euer_gewinn_paritaet(einnahmen in 0i64..=20_000_000i64, ausgaben in 0i64..=20_000_000i64) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = euer::berechnen(EuerEingabe {
            betriebseinnahmen: Cent::new(einnahmen),
            betriebsausgaben: Cent::new(ausgaben),
        }).expect("EuerGewinn-Scope laeuft durch").get();
        let args = serde_json::json!({
            "betriebseinnahmen_cent": einnahmen,
            "betriebsausgaben_cent": ausgaben,
        });
        prop_assert_eq!(diff("euer_gewinn", args, rust_cent), None);
    }

    #[test]
    fn verlustvortrag_abzug_paritaet(
        gde in 0i64..=20_000_000i64,
        bestand in 0i64..=20_000_000i64,
        zusammenveranlagung in any::<bool>(),
    ) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = verlustvortrag::berechnen(VerlustvortragEingabe {
            gesamtbetrag_einkuenfte: Cent::new(gde),
            verlustvortrag_bestand: Cent::new(bestand),
            zusammenveranlagung,
        }).expect("VerlustvortragAbzug-Scope laeuft durch").get();
        let args = serde_json::json!({
            "gesamtbetrag_einkuenfte_cent": gde,
            "verlustvortrag_bestand_cent": bestand,
            "zusammenveranlagung": zusammenveranlagung,
        });
        prop_assert_eq!(diff("verlustvortrag_abzug", args, rust_cent), None);
    }

    #[test]
    fn mitunternehmer_einkuenfte_paritaet(
        gewinnanteil in -10_000_000i64..=10_000_000i64,
        taetigkeit in 0i64..=5_000_000i64,
        darlehen in 0i64..=5_000_000i64,
        ueberlassung in 0i64..=5_000_000i64,
    ) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = mitunternehmer::berechnen(MitunternehmerEingabe {
            gewinnanteil: Cent::new(gewinnanteil),
            verguetung_taetigkeit: Cent::new(taetigkeit),
            verguetung_darlehen: Cent::new(darlehen),
            verguetung_ueberlassung: Cent::new(ueberlassung),
        }).expect("MitunternehmerEinkuenfte-Scope laeuft durch").get();
        let args = serde_json::json!({
            "gewinnanteil_cent": gewinnanteil,
            "verguetung_taetigkeit_cent": taetigkeit,
            "verguetung_darlehen_cent": darlehen,
            "verguetung_ueberlassung_cent": ueberlassung,
        });
        prop_assert_eq!(diff("mitunternehmer_einkuenfte", args, rust_cent), None);
    }

    #[test]
    fn gwg_sofortabzug_paritaet(anschaffungskosten in 0i64..=200_000i64) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = gwg::berechnen(GwgEingabe {
            anschaffungskosten_netto: Cent::new(anschaffungskosten),
        }).expect("GwgSofortabzug-Scope laeuft durch").get();
        let args = serde_json::json!({ "anschaffungskosten_netto_cent": anschaffungskosten });
        prop_assert_eq!(diff("gwg_sofortabzug", args, rust_cent), None);
    }

    #[test]
    fn ermaessigter_durchschnittssatz_paritaet(
        ao_einkuenfte in 0i64..=20_000_000i64,
        est_gesamt in 0i64..=20_000_000i64,
        bemessungsgrundlage in 100i64..=20_000_000i64,
    ) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = fuenftelregelung::berechnen(FuenftelregelungEingabe {
            ao_einkuenfte: Cent::new(ao_einkuenfte),
            est_gesamt_zzgl_progression: Cent::new(est_gesamt),
            bemessungsgrundlage_durchschnitt: Cent::new(bemessungsgrundlage),
        }).expect("ErmaessigterDurchschnittssatz-Scope laeuft durch").get();
        let args = serde_json::json!({
            "ao_einkuenfte_cent": ao_einkuenfte,
            "est_gesamt_zzgl_progression_cent": est_gesamt,
            "bemessungsgrundlage_durchschnitt_cent": bemessungsgrundlage,
        });
        prop_assert_eq!(diff("ermaessigter_durchschnittssatz", args, rust_cent), None);
    }

    /// Kein Corpus-Eintrag (Teil-B-Ableitung in runner.py, s. Modul-Doc) -- Paritaet nur hier.
    #[test]
    fn altersentlastungsbetrag_paritaet(
        arbeitslohn in 0i64..=20_000_000i64,
        andere_einkuenfte in 0i64..=20_000_000i64,
        prozentsatz in 0i64..=100i64,
        hoechstbetrag in 0i64..=200_000i64,
    ) {
        if skip_ohne_parity_env() { return Ok(()); }
        let rust_cent = altersentlastungsbetrag::berechnen(AltersentlastungsbetragEingabe {
            arbeitslohn: Cent::new(arbeitslohn),
            positive_andere_einkuenfte: Cent::new(andere_einkuenfte),
            prozentsatz: Satz::new(Decimal::new(prozentsatz, 0)),
            hoechstbetrag: Cent::new(hoechstbetrag),
        }).expect("Altersentlastungsbetrag-Scope laeuft durch").get();
        let args = serde_json::json!({
            "arbeitslohn_cent": arbeitslohn,
            "positive_andere_einkuenfte_cent": andere_einkuenfte,
            "prozentsatz_num": prozentsatz,
            "prozentsatz_den": 1,
            "hoechstbetrag_cent": hoechstbetrag,
        });
        prop_assert_eq!(diff("altersentlastungsbetrag", args, rust_cent), None);
    }
}

/// Beweist, dass der Diff-Mechanismus selbst eine Abweichung findet: EIN Rust-Ergebnis wird um
/// 1 Cent verschoben, der Rest bleibt echt. Ohne diesen Test koennte `diff_engine` kaputt sein
/// (z. B. immer `None` liefern) und alle Tests oben liefen trotzdem gruen durch.
#[test]
fn negativkontrolle_erkennt_genau_eine_abweichung() {
    if skip_ohne_parity_env() {
        eprintln!("PARITY!=1 -- uebersprungen (braucht Catala-Toolchain + Python-Umfeld)");
        return;
    }
    let mut abweichungen = Vec::new();

    for i in 0..20i64 {
        let zuwendungen = i * 10_000;
        let gesamtbetrag = 3_000_000;
        let mut rust_cent = spenden::berechnen(SpendenAbzugEingabe {
            zuwendungen: Cent::new(zuwendungen),
            gesamtbetrag_der_einkuenfte: Cent::new(gesamtbetrag),
        })
        .expect("Spenden-Scope laeuft durch")
        .get();
        if i == 10 {
            rust_cent += 1; // die eine, absichtliche Abweichung
        }
        let args = serde_json::json!({
            "zuwendungen_cent": zuwendungen,
            "gesamtbetrag_der_einkuenfte_cent": gesamtbetrag,
        });
        if let Some(a) = diff("spenden_abzug", args, rust_cent) {
            abweichungen.push(a);
        }
    }

    eprintln!(
        "negativkontrolle: {} Abweichungen gefunden (erwartet: 1)",
        abweichungen.len()
    );
    assert_eq!(
        abweichungen.len(),
        1,
        "Kontrollprobe muss GENAU eine Abweichung finden"
    );
}
