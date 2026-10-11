//! Spielt `orakel/orakel.txt` gegen die Rust-Huelle `catala_sys::*` ab. Die Datei enthaelt erzeugte
//! Eingaben und die Ergebnisse, die der Catala-INTERPRETER dazu rechnet (`examples/orakel_erzeugen.rs`,
//! `make catala-orakel`); `catala_sys` ruft dagegen das eingecheckte C-Backend. Gleiche Regeln, zwei
//! Wege: ein Fehler in der Uebergabe (Shim, Feldreihenfolge, FFI-Struktur, Rundung, Backend) zeigt sich
//! als Abweichung. Die Datei ist kein Vergleich mit Python und braucht zum Abspielen kein opam.
//!
//! Fehlt die Datei, ist eine Funktion zu duenn belegt oder gehoert die Datei zu anderen Regeln als
//! `generated/`, wird der Test rot, nie uebersprungen. Alle 26 oeffentlichen Scope-Huellen von
//! `catala_sys` stehen in `HANDLER`; eine neue Huelle ohne Eintrag dort macht
//! `jede_huelle_hat_einen_abspieler` rot.

#![allow(clippy::panic, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use catala_sys::{
    agb_abzug, altersentlastungsbetrag, berufsausbildung, betriebs_freibetrag, entfernungspauschale,
    entlastungsbetrag, ermaessigter_durchschnittssatz, euer_gewinn, familienleistungsausgleich,
    festzusetzende_est_einzel, festzusetzende_est_einzel_voll, festzusetzende_est_gesamt,
    festzusetzende_est_gesamt_zusammen, festzusetzende_est_zusammen, grundtarif, gwg_sofortabzug, haushaltsnahe,
    kirchensteuerabzug, kranken_pflege_vorsorge, mitunternehmer_einkuenfte, raumkostenabzug, spenden_abzug,
    splittingtarif, verbilligte_vermietung_wk, verlustvortrag_abzug, zumutbare_belastung, CatalaFehler,
    EntfernungspauschaleEingabe, FestzusetzendeEstErgebnis, GesamtEingabe, RaumkostenabzugEingabe, Vz,
};

/// So viele Faelle muss die Datei je Funktion mindestens enthalten.
const MINDESTFAELLE: usize = 100;

fn orakel_pfad() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("orakel/orakel.txt")
}

/// Eine Zeile der Datei: `funktion | name=wert ... | name=wert ...` oder `... | FEHLER`.
struct Zeile {
    nr: usize,
    funktion: String,
    ein: BTreeMap<String, String>,
    /// `None` = der Interpreter meldete einen Fehler.
    aus: Option<Vec<(String, i128)>>,
}

fn paare(text: &str) -> Result<Vec<(String, String)>, String> {
    text.split_whitespace()
        .map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .ok_or_else(|| format!("kein name=wert: {p:?}"))
        })
        .collect()
}

fn parse_zeile(nr: usize, zeile: &str) -> Result<Zeile, String> {
    let teile: Vec<&str> = zeile.split(" | ").collect();
    let &[funktion, ein, aus] = teile.as_slice() else {
        return Err(format!("Zeile {nr}: nicht drei Teile"));
    };
    let aus = if aus.trim() == "FEHLER" {
        None
    } else {
        let mut werte = Vec::new();
        for (k, v) in paare(aus)? {
            werte.push((k, v.parse::<i128>().map_err(|_| format!("Zeile {nr}: kein Wert: {v:?}"))?));
        }
        Some(werte)
    };
    Ok(Zeile { nr, funktion: funktion.trim().to_owned(), ein: paare(ein)?.into_iter().collect(), aus })
}

/// Liest die Datei: Kopf-Hash und alle Zeilen. Eine fehlende Datei ist ein Fehler, kein leerer Lauf.
fn lies() -> (String, Vec<Zeile>) {
    let pfad = orakel_pfad();
    let text = std::fs::read_to_string(&pfad)
        .unwrap_or_else(|e| panic!("{} fehlt oder ist unlesbar ({e}) -- `make catala-orakel`", pfad.display()));
    let mut hash = String::new();
    let mut zeilen = Vec::new();
    for (i, zeile) in text.lines().enumerate() {
        if let Some(h) = zeile.strip_prefix("# regeln-hash: ") {
            h.trim().clone_into(&mut hash);
        } else if !zeile.starts_with('#') && !zeile.trim().is_empty() {
            zeilen.push(parse_zeile(i + 1, zeile).unwrap_or_else(|f| panic!("{f}")));
        }
    }
    (hash, zeilen)
}

/// Die Eingaben einer Zeile, jede genau einmal verbraucht: eine fehlende oder uebrige Eingabe
/// (Generator und Abspieler sind auseinandergelaufen) ist ein Fehler, kein stilles Weglassen.
struct Ein(BTreeMap<String, String>);

impl Ein {
    fn text(&mut self, name: &str) -> Result<String, String> {
        self.0.remove(name).ok_or_else(|| format!("Eingabe {name} fehlt"))
    }

    fn zahl(&mut self, name: &str) -> Result<i64, String> {
        self.text(name)?.parse().map_err(|_| format!("Eingabe {name} ist keine Zahl"))
    }

    fn wahr(&mut self, name: &str) -> Result<bool, String> {
        self.text(name)?.parse().map_err(|_| format!("Eingabe {name} ist kein Wahrheitswert"))
    }

    fn vz(&mut self, name: &str) -> Result<Vz, String> {
        let jahr = u16::try_from(self.zahl(name)?).map_err(|_| format!("Eingabe {name} ist kein Jahr"))?;
        Vz::try_from(jahr).map_err(|e| e.to_string())
    }

    /// `zaehler/nenner`.
    fn bruch(&mut self, name: &str) -> Result<(i64, u64), String> {
        let text = self.text(name)?;
        let (z, n) = text.split_once('/').ok_or_else(|| format!("Eingabe {name} ist kein Bruch"))?;
        let fehler = || format!("Eingabe {name} ist kein Bruch");
        Ok((z.parse().map_err(|_| fehler())?, n.parse().map_err(|_| fehler())?))
    }

    fn leer(self) -> Result<(), String> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(format!("unbenutzte Eingaben: {:?}", self.0.keys().collect::<Vec<_>>()))
        }
    }
}

/// Aeussere `Err` = der Aufruf scheiterte; innere `Err` = eine gelesene Ausgabe passt nicht in `i64`.
type Ergebnis = Result<Vec<(&'static str, Result<i64, CatalaFehler>)>, CatalaFehler>;
type Handler = fn(&mut Ein) -> Result<Ergebnis, String>;

fn einzel(name: &'static str, r: Result<i64, CatalaFehler>) -> Ergebnis {
    r.map(|w| vec![(name, Ok(w))])
}

fn est_ausgaben(r: Result<FestzusetzendeEstErgebnis, CatalaFehler>) -> Ergebnis {
    r.map(|e| {
        vec![
            ("summe_der_einkuenfte", e.summe_der_einkuenfte_cent()),
            ("gesamtbetrag_der_einkuenfte", e.gesamtbetrag_der_einkuenfte_cent()),
            ("einkommen", e.einkommen_cent()),
            ("zu_versteuerndes_einkommen", e.zu_versteuerndes_einkommen_cent()),
            ("tarifliche_est", e.tarifliche_est_cent()),
            ("festzusetzende_est", e.festzusetzende_est_cent()),
        ]
    })
}

fn gesamt_eingabe(e: &mut Ein) -> Result<GesamtEingabe, String> {
    Ok(GesamtEingabe {
        einkuenfte_nichtselbststaendig_cent: e.zahl("einkuenfte_nichtselbststaendig")?,
        einkuenfte_kapitalvermoegen_cent: e.zahl("einkuenfte_kapitalvermoegen")?,
        einkuenfte_vermietung_cent: e.zahl("einkuenfte_vermietung")?,
        einkuenfte_sonstige_cent: e.zahl("einkuenfte_sonstige")?,
        einkuenfte_gewinn_cent: e.zahl("einkuenfte_gewinn")?,
        altersentlastungsbetrag_cent: e.zahl("altersentlastungsbetrag")?,
        entlastungsbetrag_alleinerziehende_cent: e.zahl("entlastungsbetrag_alleinerziehende")?,
        sonderausgaben_cent: e.zahl("sonderausgaben")?,
        aussergewoehnliche_belastungen_cent: e.zahl("aussergewoehnliche_belastungen")?,
        freibetraege_kinder_cent: e.zahl("freibetraege_kinder")?,
        sonstige_abzuege_vom_einkommen_cent: e.zahl("sonstige_abzuege_vom_einkommen")?,
        anzurechnende_auslaendische_steuern_cent: e.zahl("anzurechnende_auslaendische_steuern")?,
        steuerermaessigungen_cent: e.zahl("steuerermaessigungen")?,
        steuer_kapital_gesondert_cent: e.zahl("steuer_kapital_gesondert")?,
        hinzurechnung_kindergeld_cent: e.zahl("hinzurechnung_kindergeld")?,
        hinzurechnung_zulage_cent: e.zahl("hinzurechnung_zulage")?,
        tarif_modifiziert: e.wahr("tarif_modifiziert")?,
        tarifliche_est_modifiziert_cent: e.zahl("tarifliche_est_modifiziert")?,
    })
}

/// Je Funktion von `catala_sys` ein Abspieler: liest die Eingaben der Zeile nach Namen und ruft die
/// Rust-Huelle. Die Namen der Eingaben sind die der Catala-Scopes.
const HANDLER: &[(&str, Handler)] = &[
    ("grundtarif", |e| {
        let zve = e.zahl("zu_versteuerndes_einkommen")?;
        Ok(einzel("tarifliche_steuer", grundtarif(zve, e.vz("veranlagungszeitraum")?)))
    }),
    ("splittingtarif", |e| {
        let zve = e.zahl("zu_versteuerndes_einkommen_gemeinsam")?;
        Ok(einzel("tarifliche_steuer", splittingtarif(zve, e.vz("veranlagungszeitraum")?)))
    }),
    ("zumutbare_belastung", |e| {
        let gde = e.zahl("gesamtbetrag_der_einkuenfte")?;
        let kinder = e.zahl("anzahl_kinder")?;
        Ok(einzel("zumutbare_belastung", zumutbare_belastung(gde, kinder, e.wahr("splitting")?)))
    }),
    ("festzusetzende_est_gesamt", |e| {
        let eingabe = gesamt_eingabe(e)?;
        Ok(est_ausgaben(festzusetzende_est_gesamt(eingabe, e.vz("veranlagungszeitraum")?)))
    }),
    ("festzusetzende_est_gesamt_zusammen", |e| {
        let eingabe = gesamt_eingabe(e)?;
        Ok(est_ausgaben(festzusetzende_est_gesamt_zusammen(eingabe, e.vz("veranlagungszeitraum")?)))
    }),
    ("festzusetzende_est_einzel", |e| {
        let (brutto, werbung, sonder) =
            (e.zahl("bruttoarbeitslohn")?, e.zahl("werbungskosten")?, e.zahl("sonderausgaben")?);
        Ok(einzel(
            "festzusetzende_est",
            festzusetzende_est_einzel(brutto, werbung, sonder, e.vz("veranlagungszeitraum")?),
        ))
    }),
    ("festzusetzende_est_einzel_voll", |e| {
        let (brutto, werbung, sonder) =
            (e.zahl("bruttoarbeitslohn")?, e.zahl("werbungskosten")?, e.zahl("sonderausgaben")?);
        Ok(est_ausgaben(festzusetzende_est_einzel_voll(brutto, werbung, sonder, e.vz("veranlagungszeitraum")?)))
    }),
    ("festzusetzende_est_zusammen", |e| {
        let erster = (e.zahl("bruttoarbeitslohn_a")?, e.zahl("werbungskosten_a")?);
        let zweiter = (e.zahl("bruttoarbeitslohn_b")?, e.zahl("werbungskosten_b")?);
        let gemeinsam = e.zahl("sonderausgaben_gemeinsam")?;
        Ok(einzel(
            "festzusetzende_est",
            festzusetzende_est_zusammen(
                erster.0,
                erster.1,
                zweiter.0,
                zweiter.1,
                gemeinsam,
                e.vz("veranlagungszeitraum")?,
            ),
        ))
    }),
    ("spenden_abzug", |e| {
        let zuwendungen = e.zahl("zuwendungen")?;
        Ok(einzel("spenden_abzug", spenden_abzug(zuwendungen, e.zahl("gesamtbetrag_der_einkuenfte")?)))
    }),
    ("agb_abzug", |e| {
        let agb = e.zahl("aussergewoehnliche_belastungen")?;
        Ok(einzel("abzug_agb", agb_abzug(agb, e.zahl("zumutbare_belastung")?)))
    }),
    ("kirchensteuerabzug", |e| {
        let gezahlt = e.zahl("gezahlte_kirchensteuer")?;
        Ok(einzel("abziehbare_kirchensteuer", kirchensteuerabzug(gezahlt, e.zahl("erstattete_kirchensteuer")?)))
    }),
    ("altersentlastungsbetrag", |e| {
        let lohn = e.zahl("arbeitslohn")?;
        let andere = e.zahl("positive_andere_einkuenfte")?;
        let (zaehler, nenner) = e.bruch("prozentsatz")?;
        Ok(einzel(
            "altersentlastungsbetrag",
            altersentlastungsbetrag(lohn, andere, zaehler, nenner, e.zahl("hoechstbetrag")?),
        ))
    }),
    ("entlastungsbetrag", |e| {
        let alleinstehend = e.wahr("alleinstehend")?;
        let kinder = e.zahl("anzahl_kinder")?;
        Ok(einzel("entlastungsbetrag", entlastungsbetrag(alleinstehend, kinder, e.zahl("monate_ohne_voraussetzung")?)))
    }),
    ("familienleistungsausgleich", |e| {
        let ohne = e.zahl("est_ohne_freibetraege")?;
        let mit = e.zahl("est_mit_freibetraegen")?;
        Ok(einzel("est_nach_familienausgleich", familienleistungsausgleich(ohne, mit, e.zahl("kindergeld")?)))
    }),
    ("verbilligte_vermietung_wk", |e| {
        let wk = e.zahl("werbungskosten")?;
        let (zaehler, nenner) = e.bruch("entgelt_quote_prozent")?;
        Ok(einzel("abziehbare_werbungskosten", verbilligte_vermietung_wk(wk, zaehler, nenner)))
    }),
    ("kranken_pflege_vorsorge", |e| {
        let basis = e.zahl("basis_kv_pv")?;
        let weitere = e.zahl("weitere_vorsorgeaufwendungen")?;
        Ok(einzel(
            "abziehbare_kv_pv_vorsorge",
            kranken_pflege_vorsorge(basis, weitere, e.wahr("mit_anspruch_auf_zuschuss")?),
        ))
    }),
    ("berufsausbildung", |e| {
        Ok(einzel("abziehbare_sonderausgaben", berufsausbildung(e.zahl("aufwendungen")?)))
    }),
    ("betriebs_freibetrag", |e| Ok(einzel("freibetrag", betriebs_freibetrag(e.zahl("veraeusserungsgewinn")?)))),
    ("euer_gewinn", |e| {
        let einnahmen = e.zahl("betriebseinnahmen")?;
        Ok(einzel("gewinn", euer_gewinn(einnahmen, e.zahl("betriebsausgaben")?)))
    }),
    ("mitunternehmer_einkuenfte", |e| {
        let anteil = e.zahl("gewinnanteil")?;
        let taetigkeit = e.zahl("verguetung_taetigkeit")?;
        let darlehen = e.zahl("verguetung_darlehen")?;
        Ok(einzel(
            "einkuenfte_mitunternehmer",
            mitunternehmer_einkuenfte(anteil, taetigkeit, darlehen, e.zahl("verguetung_ueberlassung")?),
        ))
    }),
    ("gwg_sofortabzug", |e| Ok(einzel("sofortabzug", gwg_sofortabzug(e.zahl("anschaffungskosten_netto")?)))),
    ("haushaltsnahe", |e| {
        let minijob = e.zahl("minijob_aufwendungen")?;
        let dienst = e.zahl("haushaltsnahe_dienstleistungen")?;
        Ok(einzel("steuerermaessigung", haushaltsnahe(minijob, dienst, e.zahl("handwerker_arbeitskosten")?)))
    }),
    ("verlustvortrag_abzug", |e| {
        let gde = e.zahl("gesamtbetrag_einkuenfte")?;
        let bestand = e.zahl("verlustvortrag_bestand")?;
        Ok(einzel("verlustabzug", verlustvortrag_abzug(gde, bestand, e.wahr("zusammenveranlagung")?)))
    }),
    ("ermaessigter_durchschnittssatz", |e| {
        let ao = e.zahl("ao_einkuenfte")?;
        let est = e.zahl("est_gesamt_zzgl_progression")?;
        Ok(einzel("est_ao", ermaessigter_durchschnittssatz(ao, est, e.zahl("bemessungsgrundlage_durchschnitt")?)))
    }),
    ("entfernungspauschale", |e| {
        let (zaehler, nenner) = e.bruch("entfernung_km_roh")?;
        let eingabe = EntfernungspauschaleEingabe {
            entfernung_km_roh_num: zaehler,
            entfernung_km_roh_den: nenner,
            arbeitstage: e.zahl("arbeitstage")?,
            eigenes_oder_ueberlassenes_kfz: e.wahr("eigenes_oder_ueberlassenes_kfz")?,
            oepnv_kosten_jahr_cent: e.zahl("oepnv_kosten_jahr")?,
            satz_bis_20_km_cent: e.zahl("satz_bis_20_km")?,
            satz_ab_21_km_cent: e.zahl("satz_ab_21_km")?,
            staffelgrenze_km: e.zahl("staffelgrenze_km")?,
            hoechstbetrag_cent: e.zahl("hoechstbetrag")?,
        };
        Ok(entfernungspauschale(eingabe).map(|r| {
            vec![
                ("entfernungspauschale", r.entfernungspauschale_cent()),
                ("abziehbarer_betrag", r.abziehbarer_betrag_cent()),
            ]
        }))
    }),
    ("raumkostenabzug", |e| {
        let eingabe = RaumkostenabzugEingabe {
            arbeitszimmer_vorhanden: e.wahr("arbeitszimmer_vorhanden")?,
            ist_mittelpunkt: e.wahr("ist_mittelpunkt")?,
            tatsaechliche_aufwendungen_cent: e.zahl("tatsaechliche_aufwendungen")?,
            jahrespauschale_gewaehlt: e.wahr("jahrespauschale_gewaehlt")?,
            monate_ohne_mittelpunkt: e.zahl("monate_ohne_mittelpunkt")?,
            homeoffice_tage: e.zahl("homeoffice_tage")?,
            jahrespauschale_cent: e.zahl("jahrespauschale")?,
            tagespauschale_pro_tag_cent: e.zahl("tagespauschale_pro_tag")?,
            tagespauschale_hoechstbetrag_cent: e.zahl("tagespauschale_hoechstbetrag")?,
        };
        Ok(raumkostenabzug(eingabe).map(|r| {
            vec![
                ("abzug_arbeitszimmer", r.abzug_arbeitszimmer_cent()),
                ("abzug_homeoffice", r.abzug_homeoffice_cent()),
                ("abzug_gesamt", r.abzug_gesamt_cent()),
            ]
        }))
    }),
];

/// Ruft die Rust-Funktion, die zur Zeile gehoert. `Err(String)` = die Zeile passt nicht zum Abspieler.
fn rechne(z: &Zeile) -> Result<Ergebnis, String> {
    let (_, handler) = HANDLER
        .iter()
        .find(|(name, _)| *name == z.funktion)
        .ok_or_else(|| format!("Funktion {:?} kennt der Abspieler nicht", z.funktion))?;
    let mut e = Ein(z.ein.clone());
    let ergebnis = handler(&mut e)?;
    e.leer()?;
    Ok(ergebnis)
}

/// `None` = Rust und Interpreter stimmen ueberein. Ein Rechenfehler des Interpreters (zum Beispiel
/// Teilung durch null) muss in Rust als `Assertion` ankommen. Passt ein Wert des Interpreters nicht in
/// `i64`, muss Rust genau das melden (`Ueberlauf`), statt eine falsche Zahl zu liefern.
fn abweichung(z: &Zeile, rust: &Ergebnis) -> Option<String> {
    match (&z.aus, rust) {
        (None, Err(CatalaFehler::Assertion)) => None,
        (None, Err(f)) => Some(format!("Catala meldet einen Fehler, Rust meldet {f:?} statt Assertion")),
        (None, Ok(_)) => Some("Catala meldet einen Fehler, Rust liefert Werte".to_owned()),
        (Some(_), Err(f)) => Some(format!("Catala liefert Werte, Rust scheitert: {f}")),
        (Some(erwartet), Ok(ist)) => {
            if erwartet.len() != ist.len() {
                return Some(format!("{} Ausgaben erwartet, Rust liefert {}", erwartet.len(), ist.len()));
            }
            erwartet.iter().find_map(|(name, soll)| {
                let Some((_, got)) = ist.iter().find(|(n, _)| n == name) else {
                    return Some(format!("Ausgabe {name} fehlt in Rust"));
                };
                let gleich = match i64::try_from(*soll) {
                    Ok(w) => *got == Ok(w),
                    Err(_) => matches!(got, Err(CatalaFehler::Ueberlauf(_))),
                };
                (!gleich).then(|| format!("{name}: Catala {soll}, Rust {got:?}"))
            })
        }
    }
}

#[test]
fn orakel_gehoert_zu_den_regeln_von_generated() {
    let (hash, _) = lies();
    let stand = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("generated/SOURCE_HASH"))
        .expect("generated/SOURCE_HASH fehlt");
    assert_eq!(
        hash,
        stand.trim(),
        "orakel.txt stammt aus anderen Regeln als generated/ -- `make catala-c`, dann `make catala-orakel`"
    );
}

#[test]
fn orakel_belegt_jede_funktion_ausreichend() {
    let (_, zeilen) = lies();
    for (name, _) in HANDLER {
        let n = zeilen.iter().filter(|z| z.funktion == *name).count();
        assert!(n >= MINDESTFAELLE, "{name}: {n} Faelle in orakel.txt, verlangt sind {MINDESTFAELLE}");
    }
}

/// Jede oeffentliche Scope-Huelle von `catala_sys` hat einen Abspieler. Die Liste der Huellen steht hier
/// von Hand, gegen `pub fn` in `src/lib.rs` gezaehlt: eine neue Huelle ohne Eintrag macht den Test rot.
#[test]
fn jede_huelle_hat_einen_abspieler() {
    // Alle `pub fn` ganz links in lib.rs (Methoden sind eingerueckt), ausser den Hilfsfunktionen.
    const KEINE_HUELLE: &[&str] = &["recompute_source_hash"];
    let lib = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).expect("lib.rs");
    let huellen: Vec<&str> = lib
        .lines()
        .filter_map(|z| z.strip_prefix("pub fn "))
        .filter_map(|z| z.split(['(', '<']).next())
        .filter(|n| !KEINE_HUELLE.contains(n))
        .collect();
    assert!(huellen.len() >= 26, "weniger als 26 Huellen gefunden ({}), Suchmuster kaputt?", huellen.len());
    for huelle in &huellen {
        assert!(HANDLER.iter().any(|(n, _)| n == huelle), "catala_sys::{huelle} hat keinen Abspieler in HANDLER");
    }
    assert_eq!(huellen.len(), HANDLER.len(), "HANDLER hat Eintraege ohne Huelle");
}

/// Die Datei muss die Faelle enthalten, an denen frueher alle Rust-Tests vorbeiliefen (Mutanten M06 und
/// M10 der Messung vom 2026-10-10), dazu Rechenfehler des Interpreters und Werte ausserhalb von `i64`.
#[test]
fn orakel_beruehrt_die_schwierigen_faelle() {
    let (_, zeilen) = lies();
    let zaehle = |funktion: &str, feld: &str, ab: i64| {
        zeilen
            .iter()
            .filter(|z| z.funktion == funktion)
            .filter(|z| z.ein.get(feld).and_then(|v| v.parse::<i64>().ok()).is_some_and(|v| v >= ab))
            .count()
    };
    assert!(zaehle("zumutbare_belastung", "anzahl_kinder", 3) >= 20, "zu wenige Faelle mit drei Kindern");
    for gesamt in ["festzusetzende_est_gesamt", "festzusetzende_est_gesamt_zusammen"] {
        assert!(zaehle(gesamt, "hinzurechnung_zulage", 1) >= 20, "{gesamt}: zu wenige Faelle mit Zulage");
    }
    let fehler = zeilen.iter().filter(|z| z.aus.is_none()).count();
    assert!(fehler >= 5, "nur {fehler} Rechenfehler des Interpreters in der Datei, verlangt sind 5");
    let gross = zeilen
        .iter()
        .filter(|z| z.aus.iter().flatten().any(|(_, w)| i64::try_from(*w).is_err()))
        .count();
    assert!(gross >= 1, "kein Fall mit Werten ausserhalb i64 in der Datei, der Ueberlauf-Weg bliebe ungeprueft");
}

#[test]
fn rust_rechnet_wie_der_catala_interpreter() {
    let (_, zeilen) = lies();
    let mut abweichungen = Vec::new();
    for z in &zeilen {
        let msg = match rechne(z) {
            Ok(rust) => abweichung(z, &rust),
            Err(f) => Some(f),
        };
        if let Some(m) = msg {
            abweichungen.push(format!("Zeile {} ({}): {m}", z.nr, z.funktion));
        }
    }
    assert!(
        abweichungen.is_empty(),
        "{} von {} Faellen weichen ab, die ersten:\n{}",
        abweichungen.len(),
        zeilen.len(),
        abweichungen.iter().take(10).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// Der Vergleich selbst: ein absichtlich falscher Sollwert muss auffallen, sonst waere jeder Lauf gruen.
#[test]
fn der_vergleich_erkennt_eine_abweichung() {
    let z = Zeile {
        nr: 1,
        funktion: "grundtarif".to_owned(),
        ein: BTreeMap::new(),
        aus: Some(vec![("tarifliche_steuer".to_owned(), 100)]),
    };
    assert!(abweichung(&z, &Ok(vec![("tarifliche_steuer", Ok(100))])).is_none());
    assert!(abweichung(&z, &Ok(vec![("tarifliche_steuer", Ok(101))])).is_some());
    assert!(abweichung(&z, &Err(CatalaFehler::Assertion)).is_some());
    let huge = Zeile { aus: Some(vec![("tarifliche_steuer".to_owned(), i128::from(i64::MAX) + 1)]), ..z };
    assert!(abweichung(&huge, &Ok(vec![("tarifliche_steuer", Err(CatalaFehler::Ueberlauf("x")))])).is_none());
    assert!(abweichung(&huge, &Ok(vec![("tarifliche_steuer", Ok(i64::MAX))])).is_some());
    let fehler = Zeile { aus: None, ..huge };
    assert!(abweichung(&fehler, &Err(CatalaFehler::Assertion)).is_none());
    assert!(abweichung(&fehler, &Ok(vec![("tarifliche_steuer", Ok(0))])).is_some());
    assert!(abweichung(&fehler, &Err(CatalaFehler::Ueberlauf("x"))).is_some());
}
