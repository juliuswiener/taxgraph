//! Die BMF-Uebersicht zur Rentenbesteuerung gegen den Bescheid-Weg: 43 Zeilen (21 fuer 2025, 22 fuer 2026), je eine Zeile ein
//! alleinstehender Rentner, der genau an der Grenze zur Steuerfreiheit liegt. Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Das Bundesfinanzministerium nennt je Jahr des Rentenbeginns die hoechste Jahresbruttorente, bei der ein
//! Rentner ohne weitere Einkuenfte keine Einkommensteuer zahlt, und leitet sie her: Bruttorente, steuerfreier Teil, Anteil der
//! Besteuerung, Werbungskosten-Pauschbetrag (102 EUR), Sonderausgaben-Pauschbetrag (36 EUR), Vorsorgeaufwendungen und zu
//! versteuerndes Einkommen (= Grundfreibetrag). Die Zahlen stammen nicht aus unserem Code.
//!
//! **Warum es zaehlt.** Ein Fehler in einer Zeile der Besteuerungsanteil-Tabelle oder bei einem Pauschbetrag aendert die Steuer jedes
//! Rentners dieses Jahrgangs. Die Uebersicht liefert dafuer 43 unabhaengige Pruefpunkte. Der Fall prueft den Rentenkern
//! (Besteuerungsanteil, Pauschbetraege, Vorsorgeabzug, Grundfreibetrag), NICHT den Tarif: die Steuer ist an jedem Punkt 0.
//!
//! **Wo es sitzt.** Die Erwartung liest der Test zur Laufzeit aus `sources/bmf/bmf_rentenbesteuerung_uebersicht_*.txt` (PDF per
//! `pdftotext -layout`, Pruefsumme in der `.meta.yaml`, `make sources-check`); es gibt keine abgetippte Kopie. Der Bescheid-Weg ist
//! die Scheibe `RentnerGesamt` (`zweige/rentner.rs`, `engine::zugriff::teil2::rente`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Uebersicht des BMF (Stand Mai 2025 fuer 2025, Juni 2026 fuer 2026, Spiegel beim Finanzamt
//! Brandenburg, abgerufen 2026-10-15); Quelle und Lizenzlage in der `.meta.yaml`. Keine Zahl ist von Hand gerechnet oder dem Rust-Code
//! entnommen. Eingabe je Zeile: Jahr des Rentenbeginns, die hoechste Jahresbruttorente der Zeile, der "betragsmaessig
//! festgeschriebene steuerfreie Teil der Rente" der Zeile als Rentenfreibetrag (bei Rentenbeginn im Veranlagungsjahr gilt keiner,
//! dort rechnet die Engine den Prozentsatz), die Vorsorgeaufwendungen der Zeile als Basiskranken-/Pflegebeitrag. Die Aufteilung auf
//! Kranken- und Pflegeversicherung nennt die Uebersicht nicht; die Engine summiert beide (`kv_pv_sonderausgaben`), die Kontrolle
//! `die_aufteilung_auf_kv_und_pv_aendert_nichts` belegt das. Erwartung je Zeile: Gesamtbetrag der Einkuenfte = der Besteuerung
//! unterliegender Anteil minus Werbungskosten-Pauschbetrag, zu versteuerndes Einkommen und Steuer 0 EUR.
//!
//! Grenzen: nur Alleinstehende, nur gesetzliche Rente, nur Steuer 0 (Fussnote 1 der Uebersicht); Annahmen der Uebersicht zu den
//! Beitragssaetzen (allgemeiner Beitragssatz ohne Zusatzbeitrag) gehen nur ueber die Vorsorge-Spalte ein, nicht als Rechnung.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::deklaration::{feste_zahl, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use bescheid::Felder;
use bindung::Bindung;
use domain::{Feldtyp, Scheibe, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const SCHEIBE: Scheibe = Scheibe::RentnerGesamt;

/// Die zwei Quellen der Uebersicht: Veranlagungsjahr und Textdatei unter `sources/bmf/`.
const QUELLEN: [(Vz, &str); 2] = [
    (Vz::Vz2025, "bmf_rentenbesteuerung_uebersicht_2025-05.txt"),
    (Vz::Vz2026, "bmf_rentenbesteuerung_uebersicht_2026-06.txt"),
];

type Paare = Vec<(String, Value)>;

/// Eine Zeile der Uebersicht, alle Betraege in ganzen Euro.
#[derive(Debug, Clone, Copy)]
struct Zeile {
    vz: Vz,
    /// Jahr des Rentenbeginns; bei "2005 (oder frueher)" 2005.
    beginn: i64,
    brutto: i64,
    /// Besteuerungsanteil in Zehntelprozent: 83,5 % ist 835.
    zehntel: i64,
    steuerfrei: i64,
    steuerpflichtig: i64,
    wk: i64,
    sa: i64,
    vorsorge: i64,
    zve: i64,
}

/// "20.249" -> 20249 (Tausenderpunkt).
fn zahl(s: &str) -> Option<i64> {
    s.replace('.', "").parse().ok()
}

/// "83,5" -> 835.
fn zehntel_prozent(s: &str) -> Option<i64> {
    let (g, k) = s.split_once(',')?;
    (k.len() == 1).then_some(())?;
    Some(g.parse::<i64>().ok()? * 10 + k.parse::<i64>().ok()?)
}

/// Eine Zeile aus einer Textzeile der Uebersicht; alles andere (Kopf, Fussnoten) ist keine Zeile.
fn zeile(vz: Vz, zeile: &str) -> Option<Zeile> {
    let mut t = zeile.split_whitespace();
    let jahr = t.next().filter(|j| j.len() == 4)?;
    let beginn: i64 = jahr.parse().ok()?;
    let alle: Vec<&str> = t.collect();
    let rest = alle
        .strip_prefix(&["(oder", "früher)"][..])
        .unwrap_or(&alle);
    let [brutto, m1, m2, prozent, frei, pflichtig, wk, sa, vor, zve] = rest else {
        return None;
    };
    zahl(m1)?;
    zahl(m2)?;
    Some(Zeile {
        vz,
        beginn,
        brutto: zahl(brutto)?,
        zehntel: zehntel_prozent(prozent)?,
        steuerfrei: zahl(frei)?,
        steuerpflichtig: zahl(pflichtig)?,
        wk: zahl(wk)?,
        sa: zahl(sa)?,
        vorsorge: zahl(vor)?,
        zve: zahl(zve)?,
    })
}

/// Alle Zeilen einer Quelle in der Reihenfolge der Datei.
fn zeilen(vz: Vz, datei: &str) -> Vec<Zeile> {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sources/bmf")
        .join(datei);
    let text = std::fs::read_to_string(&pfad)
        .unwrap_or_else(|e| panic!("KONTROLLE: {} nicht lesbar: {e}", pfad.display()));
    text.lines().filter_map(|l| zeile(vz, l)).collect()
}

/// Alle 43 Zeilen, 2025 zuerst.
fn alle_zeilen() -> Vec<Zeile> {
    QUELLEN
        .iter()
        .flat_map(|(vz, datei)| zeilen(*vz, datei))
        .collect()
}

fn p(feld: &str, wert: Value) -> (String, Value) {
    (feld.to_owned(), wert)
}

/// Euro in Cent.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

/// Der Abwesenheitswert, nie der illustrative Beispielwert (wie `standardwert` in `rente_pauschbetrag_einmal.rs`).
fn standardwert(b: &Bindung) -> Value {
    if let Some(w) = &b.abwesenheitswert {
        return w.clone();
    }
    match b.typ {
        Feldtyp::Bool => json!(b.feld_id.starts_with("kein_") || b.feld_id.starts_with("keine_")),
        Feldtyp::Cent | Feldtyp::Int => json!(0),
        Feldtyp::Enum => b.beispielwert.clone(),
        _ => panic!(
            "KONTROLLE: Kegel-Feld {} hat keinen Abwesenheitswert",
            b.feld_id
        ),
    }
}

/// Der volle Pflicht-Kegel der Scheibe; was der Test setzt, gewinnt, der Rest folgt bestaetigt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(String, Value)]) -> Paare {
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Paare = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .rev()
                .find(|(g, _)| g == *f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            ((*f).to_owned(), w)
        })
        .collect();
    raus.extend(
        gesetzt
            .iter()
            .filter(|(f, _)| !kegel.contains(&f.as_str()))
            .cloned(),
    );
    raus
}

/// Das Ergebnis der Kette in ganzen Euro.
#[derive(Debug, PartialEq, Eq)]
struct Ergebnis {
    gde: i64,
    zve: i64,
    tarifliche_est: i64,
    festzusetzende_est: i64,
}

/// Die Kette der Scheibe `RentnerGesamt` fuer eine Akte im Veranlagungsjahr `vz`; ein Fall ohne Zahl ist ein Fehlschlag des Tests.
fn kette(vz: Vz, paare: &[(String, Value)]) -> Ergebnis {
    let cfg = Cfg::fuer(SCHEIBE);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let alle = kegel_fuer(&cfg, paare);
    let events: Vec<(&str, Value, bool)> = alle
        .iter()
        .map(|(f, w)| (f.as_str(), w.clone(), true))
        .collect();
    let store: Store = store(&events);
    let felder: Felder = felder(&store);
    let achsen: Vec<AchsenBindung> = teil.iter().map(|b| AchsenBindung::from(*b)).collect();
    let index: HashMap<String, &'static Bindung> =
        teil.iter().map(|b| (b.feld_id.clone(), *b)).collect();
    let kegel = cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &achsen,
        index: &index,
        params: params(),
    };
    match feste_zahl(&felder, &cfg, vz, &kegel, &umg, Some(&store), None) {
        Ok(Ok(z)) => {
            let k = z.extras.kette.expect("KONTROLLE: Kette fehlt");
            Ergebnis {
                gde: k.gesamtbetrag_der_einkuenfte.get(),
                zve: k.zu_versteuerndes_einkommen.get(),
                tarifliche_est: k.tarifliche_est.get(),
                festzusetzende_est: k.festzusetzende_est.get(),
            }
        }
        Ok(Err(k)) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {:?}", k.grund),
        Err(e) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {e:?}"),
    }
}

/// Die Akte einer Zeile: Einzelveranlagung, eine gesetzliche Rente der Anlage R mit der Bruttorente `brutto`, KV und PV in Cent.
/// Bei Rentenbeginn im Veranlagungsjahr gilt kein Rentenfreibetrag (er wird erst im Folgejahr festgeschrieben, Fussnote 3).
fn akte(z: &Zeile, brutto: i64, kv_cent: i64, pv_cent: i64) -> Paare {
    let freibetrag = if z.beginn < i64::from(z.vz.jahr()) {
        z.steuerfrei
    } else {
        0
    };
    vec![
        p("veranlagung", json!("einzel")),
        p("stammdaten_geburtsdatum", json!("05.05.1955")),
        p("kein_gewinn", json!(true)),
        p("kein_kap", json!(true)),
        p("kein_vuv", json!(true)),
        p("kein_sonstige", json!(false)),
        p("agb_zwangslaeufig", json!(true)),
        p("agb_notwendig_angemessen", json!(true)),
        p("rentner_renten_art", json!("gesetzliche_rente")),
        p("rentner_jahresrente", json!(cent(brutto))),
        p("rentner_renten_beginn_jahr", json!(z.beginn)),
        p("rentner_alter_bei_rentenbeginn", json!(65)),
        p("rentner_rentenfreibetrag", json!(cent(freibetrag))),
        p("versicherungsart", json!("gesetzlich_freiwillig")),
        p("basis_kv", json!(kv_cent)),
        p("basis_pv", json!(pv_cent)),
        p("mit_anspruch_auf_zuschuss", json!(false)),
    ]
}

/// Die Zeile an der Grenze: Bruttorente der Zeile, Vorsorgeaufwendungen der Zeile als Basiskrankenbeitrag.
fn grenzfall(z: &Zeile) -> Ergebnis {
    kette(z.vz, &akte(z, z.brutto, cent(z.vorsorge), 0))
}

/// Die Quelle ist vollstaendig und rechnet in sich auf: je Veranlagungsjahr genau die Jahrgaenge 2005 bis zum Veranlagungsjahr, 43
/// Zeilen zusammen; Bruttorente minus steuerfreier Teil ist der Anteil der Besteuerung, Anteil minus Pauschbetraege minus Vorsorge ist
/// das zu versteuernde Einkommen, und das ist der Grundfreibetrag der Parameter. Ohne diese Pruefung bewiese ein Lesefehler im Parser
/// (verrutschte Spalte, verlorene Zeile) nichts ueber den Bescheid-Weg.
#[test]
fn die_quelle_ist_vollstaendig_und_rechnet_in_sich_auf() {
    let params = params();
    let mut gesamt = 0;
    for (vz, datei) in QUELLEN {
        let zs = zeilen(vz, datei);
        let jahre: Vec<i64> = zs.iter().map(|z| z.beginn).collect();
        let soll: Vec<i64> = (2005..=i64::from(vz.jahr())).collect();
        assert_eq!(jahre, soll, "{vz}: Jahrgaenge der Zeilen in {datei}");
        gesamt += zs.len();
        let grundfreibetrag = params.grundfreibetrag(vz).unwrap().get();
        for z in &zs {
            let wo = format!("{vz}, Rentenbeginn {}", z.beginn);
            assert_eq!(
                z.brutto - z.steuerfrei,
                z.steuerpflichtig,
                "{wo}: Brutto - steuerfreier Teil"
            );
            assert_eq!(
                z.steuerpflichtig - z.wk - z.sa - z.vorsorge,
                z.zve,
                "{wo}: Anteil - Pauschbetraege - Vorsorge"
            );
            assert_eq!(z.zve, grundfreibetrag, "{wo}: zvE = Grundfreibetrag");
        }
    }
    assert_eq!(gesamt, 43, "Zeilen in beiden Quellen");
}

/// Die Prozentspalte der Uebersicht ist die Besteuerungsanteil-Tabelle der Parameter, Zeile fuer Zeile. Fuer Rentenbeginn vor dem
/// Veranlagungsjahr nimmt der Bescheid-Weg den Rentenfreibetrag als Eingabe und liest die Tabelle nicht; dieser Test ist darum die
/// einzige Pruefung der Prozente der 41 aelteren Jahrgaenge. Kontrolle: die Spalte ist nicht konstant (50,0 bis 84,0).
#[test]
fn die_prozentspalte_gleicht_der_besteuerungsanteil_tabelle() {
    let params = params();
    let zs = alle_zeilen();
    for z in &zs {
        let ist = params.rente_besteuerungsanteil(z.beginn).unwrap();
        let ist = ist.map(|s| s.get() * rust_decimal::Decimal::from(10));
        assert_eq!(
            ist,
            Some(rust_decimal::Decimal::from(z.zehntel)),
            "{}, Rentenbeginn {}: Besteuerungsanteil in Zehntelprozent",
            z.vz,
            z.beginn
        );
    }
    let kleinster = zs.iter().map(|z| z.zehntel).min();
    let groesster = zs.iter().map(|z| z.zehntel).max();
    assert_eq!((kleinster, groesster), (Some(500), Some(840)), "KONTROLLE");
}

/// Die Zeilen, fuer die `pruefe` eine Abweichung meldet; alle Zeilen durchlaufen, dann melden: unter einer Mutation zeigt die Meldung
/// jede rote Zeile.
fn abweichungen(pruefe: impl Fn(&Zeile) -> Option<String>) -> (Vec<String>, usize) {
    let zs = alle_zeilen();
    (zs.iter().filter_map(pruefe).collect(), zs.len())
}

fn melde(abweichend: &[String], von: usize, was: &str) {
    assert!(
        abweichend.is_empty(),
        "{was}: {} von {von} Zeilen weichen ab: {abweichend:#?}",
        abweichend.len()
    );
}

/// Der Rentenkern: der Gesamtbetrag der Einkuenfte ist der Anteil der Besteuerung minus Werbungskosten-Pauschbetrag, in allen 43
/// Zeilen. Fuer Rentenbeginn vor dem Veranlagungsjahr heisst das: Bruttorente minus Rentenfreibetrag minus 102 EUR; fuer Rentenbeginn
/// im Veranlagungsjahr rechnet die Engine den Prozentsatz (83,5 % bzw. 84,0 %, abgerundet auf volle Euro).
#[test]
fn jede_zeile_stimmt_im_gesamtbetrag_der_einkuenfte() {
    let (ab, von) = abweichungen(|z| {
        let ist = grenzfall(z).gde;
        let soll = z.steuerpflichtig - z.wk;
        (ist != soll).then(|| {
            format!(
                "{}, Rentenbeginn {}, brutto {}: Ist {ist}, Soll {soll}",
                z.vz, z.beginn, z.brutto
            )
        })
    });
    melde(&ab, von, "Gesamtbetrag der Einkuenfte");
}

/// Ohne Vorsorgeaufwendungen zieht der Bescheid-Weg Werbungskosten-Pauschbetrag (102 EUR) und Sonderausgaben-Pauschbetrag (36 EUR)
/// ab: zvE = Anteil der Besteuerung - wk - sa, in allen 43 Zeilen. Das ist die Spaltenrechnung der Uebersicht ohne die Spalte
/// "abzugsfaehige Vorsorgeaufwendungen" und zugleich die Kontrolle zum Defekttest unten: dort fehlt der Pauschbetrag nur, wenn
/// Vorsorge da ist.
#[test]
fn jede_zeile_ohne_vorsorge_endet_nach_den_zwei_pauschbetraegen() {
    let (ab, von) = abweichungen(|z| {
        let ist = kette(z.vz, &akte(z, z.brutto, 0, 0));
        let soll = (z.steuerpflichtig - z.wk, z.steuerpflichtig - z.wk - z.sa);
        ((ist.gde, ist.zve) != soll).then(|| {
            format!(
                "{}, Rentenbeginn {}, brutto {}: Ist (GdE, zvE) {:?}, Soll {soll:?}",
                z.vz,
                z.beginn,
                z.brutto,
                (ist.gde, ist.zve)
            )
        })
    });
    melde(&ab, von, "ohne Vorsorge");
}

/// DEFEKT: die Zeile der Uebersicht mit ihren Vorsorgeaufwendungen endet im Bescheid-Weg beim Grundfreibetrag und bei Steuer 0. Heute
/// ist das zu versteuernde Einkommen in 43 von 43 Zeilen um genau 36 EUR zu hoch (2025: 12.132 statt 12.096; 2026: 12.384 statt
/// 12.348) und die Steuer 5 EUR statt 0. Grund: `sonderausgaben_final` (`engine/src/zugriff/teil2/gesamt.rs`) bildet
/// `max(Sonderausgaben + Vorsorge, 36)`; § 10c Satz 1 `EStG` nennt die Vorsorge (§ 10 Abs. 1 Nr. 2, 3, 3a) nicht, der Pauschbetrag gilt
/// daneben (`sources/gesetze-im-internet/estg_p10c_2026-07-09.txt`), und die Uebersicht zieht beide ab.
#[test]
#[ignore = "DEFEKT: zvE um 36 EUR zu hoch, Steuer 5 statt 0 in 43 von 43 Zeilen (Sonderausgaben-Pauschbetrag entfaellt neben der Vorsorge, engine::zugriff::teil2::gesamt::sonderausgaben_final); Quelle BMF-Uebersicht und § 10c Satz 1 EStG; Befund an main (Rente-BMF), Vault-Notiz folgt"]
fn jede_zeile_der_uebersicht_endet_im_bescheid_weg_bei_steuer_null() {
    let (ab, von) = abweichungen(|z| {
        let soll = Ergebnis {
            gde: z.steuerpflichtig - z.wk,
            zve: z.zve,
            tarifliche_est: 0,
            festzusetzende_est: 0,
        };
        let ist = grenzfall(z);
        (ist != soll).then(|| {
            format!(
                "{}, Rentenbeginn {}, brutto {}: Ist {ist:?}, Soll {soll:?}",
                z.vz, z.beginn, z.brutto
            )
        })
    });
    melde(&ab, von, "DEFEKT: Steuer null");
}

/// Kontrolle: der Aufbau misst. Zwei Euro-Tausend mehr Bruttorente bei gleicher Vorsorge geben mehr Steuer als die Grenzzeile; ein
/// Aufbau, der immer dasselbe liefert, bestuende die Tests oben. Fuer Rentenbeginn vor dem Veranlagungsjahr ist der Rentenfreibetrag
/// fest, also steigt das zu versteuernde Einkommen um genau 2.000 EUR.
#[test]
fn kontrolle_mehr_rente_gibt_mehr_steuer() {
    for z in &alle_zeilen() {
        let grenze = grenzfall(z);
        let mehr = kette(z.vz, &akte(z, z.brutto + 2_000, cent(z.vorsorge), 0));
        let wo = format!("{}, Rentenbeginn {}", z.vz, z.beginn);
        assert!(
            mehr.festzusetzende_est > grenze.festzusetzende_est,
            "{wo}: Steuer {} bei +2.000 EUR Rente, Grenzzeile {}",
            mehr.festzusetzende_est,
            grenze.festzusetzende_est
        );
        if z.beginn < i64::from(z.vz.jahr()) {
            assert_eq!(mehr.zve - grenze.zve, 2_000, "{wo}: zvE bei +2.000 EUR Rente");
        }
    }
}

/// Die Aufteilung der Vorsorge auf Kranken- und Pflegeversicherung nennt die Uebersicht nicht. Der Bescheid-Weg summiert beide:
/// ganz auf `basis_kv`, ganz auf `basis_pv` und je zur Haelfte geben dasselbe Ergebnis (Rentenbeginn 2005, 2015 und das
/// Veranlagungsjahr selbst, je Quelle: 6 Zeilen).
#[test]
fn die_aufteilung_auf_kv_und_pv_aendert_nichts() {
    let zs = alle_zeilen();
    let proben: Vec<&Zeile> = zs
        .iter()
        .filter(|z| matches!(z.beginn, 2005 | 2015) || z.beginn == i64::from(z.vz.jahr()))
        .collect();
    assert_eq!(proben.len(), 6, "KONTROLLE: Proben");
    for z in proben {
        let nur_kranken = grenzfall(z);
        let nur_pflege = kette(z.vz, &akte(z, z.brutto, 0, cent(z.vorsorge)));
        let kv = cent(z.vorsorge / 2);
        let halb = kette(z.vz, &akte(z, z.brutto, kv, cent(z.vorsorge) - kv));
        assert_eq!(nur_kranken, nur_pflege, "{}, {}: ganz KV gegen ganz PV", z.vz, z.beginn);
        assert_eq!(nur_kranken, halb, "{}, {}: ganz KV gegen halb/halb", z.vz, z.beginn);
    }
}
