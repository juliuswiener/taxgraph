//! Spielt `rust/catala-sys/orakel/orakel.txt` gegen die MODULE von `engine` ab (nicht gegen `catala_sys`
//! wie `catala-sys/tests/catala_orakel.rs`). Die Datei haelt erzeugte Eingaben und das Ergebnis, das der
//! Catala-INTERPRETER dazu rechnet; hier laeuft jede Zeile durch einen Adapter (Orakel-Zeile -> `Eingabe`
//! des Moduls -> `berechnen`) und wird mit dem Wert des Interpreters verglichen. Ein Fehler in der
//! Abbildung "Engine-Eingabe -> Scope-Eingabe" (Feld vertauscht, nicht durchgereicht, falsche Einheit, Nenner
//! verloren) zeigt sich als Abweichung. Kein Python, kein opam, kein `PARITY=1`.
//!
//! Warum es den Test gibt: die Messung vom 2026-10-11 (`research/engine-schicht-mutantenmessung-sechs-
//! luecken-2026-10-11`) setzte 92 Fehler in diese Abbildung; sechs ueberlebten alle Rust-Tests ohne
//! `rust/parity` (ZB2, VL2, AZ1, ZA05, TA6, VV3). Dieser Test ersetzt `engine_catala_paritaet` fuer die
//! 19 Module; die Schicht `zugriff` (Sachverhalt -> Engine-Eingabe) deckt er nicht.
//!
//! Fehlt die Datei, hat ein Modul oder eine Scope-Huelle keinen Adapter, wird eine Funktion zu duenn
//! abgespielt oder vergleicht ein Adapter keinen Wert, wird der Test rot, nie uebersprungen.
//!
//! Die drei Module mit Dezimal-Eingabe (`Satz`, `Km`) fuehren den Wert als `Decimal`; eine Zeile mit
//! einem Bruch wie `6928/78` hat dort keine Entsprechung. Der Adapter bildet nur Nenner ab, die 100 teilen,
//! und zaehlt die uebrigen als uebersprungen; fuer sie traegt die Datei eigene Zeilen mit Nenner 1, 10, 100.

#![allow(clippy::panic, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use catala_sys::CatalaFehler;
use domain::{Cent, Km, Satz};
use engine::{
    agb, altersentlastungsbetrag, arbeitszimmer, berufsausbildung, betriebsfreibetrag, entfernungspauschale,
    entlastungsbetrag, euer, familienleistungsausgleich, fuenftelregelung, gwg, kirchensteuer, mitunternehmer,
    spenden, tarif, verbilligte_vermietung, verlustvortrag, vorsorgeaufwendungen, zumutbare_belastung,
};
use rust_decimal::Decimal;

// Leser und Vergleich teilt sich dieser Test mit `catala-sys/tests/catala_orakel.rs`.
#[path = "../../catala-sys/orakel/leser.rs"]
mod leser;
use leser::{abweichung, einzel, est_ausgaben, lies_aus, Ein, Ergebnis, Zeile};

/// So viele abgespielte Faelle muss die Datei je Funktion mindestens enthalten.
const MINDESTFAELLE: usize = 100;
/// So viele Zeilen mit Nachkommastellen muss die Datei je Dezimal-Funktion mindestens enthalten.
const MINDEST_NACHKOMMA: usize = 50;

/// Scope-Huellen von `catala_sys` ohne Modul in `engine`; `engine::zugriff` ruft sie ohne Zwischenschicht.
const OHNE_MODUL: &[&str] = &["haushaltsnahe"];
/// Module in `engine`, die keinen Catala-Scope hinter sich haben, mit dem Grund.
const KEIN_ADAPTER: &[(&str, &str)] = &[
    ("dezimal", "Helfer Decimal -> Bruch, ruft keinen Scope"),
    ("sachverhalt", "Verzweigung ueber die Module, kein eigener Scope"),
    ("zugriff", "Sachverhalt -> Engine-Eingabe (Teil B), eigene Parity-Suiten"),
];

/// `Ok(None)` = die Zeile hat im Modul keine Entsprechung (Bruch ohne endliche Dezimaldarstellung).
type Adapter = fn(&mut Ein) -> Result<Option<Ergebnis>, String>;

fn orakel_pfad() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../catala-sys/orakel/orakel.txt")
}

fn lies() -> Vec<Zeile> {
    lies_aus(&orakel_pfad()).1
}

fn geld(e: &mut Ein, name: &str) -> Result<Cent, String> {
    e.zahl(name).map(Cent::new)
}

fn cent(name: &'static str, r: Result<Cent, CatalaFehler>) -> Result<Option<Ergebnis>, String> {
    Ok(Some(einzel(name, r.map(Cent::get))))
}

/// `zaehler/nenner` als `Decimal`, wenn der Nenner 100 teilt (dann ist der Wert eine Dezimalzahl mit hoechstens
/// zwei Nachkommastellen); sonst `None`. `normalize` streicht Nullen am Ende: `505/10` wird `50.5` mit
/// Skala 1, wie ein Modul den Wert aus den Parametern bekommt.
fn als_dezimal((zaehler, nenner): (i64, u64)) -> Option<Decimal> {
    if !100_u64.is_multiple_of(nenner) {
        return None;
    }
    let faktor = i64::try_from(100 / nenner).ok()?;
    Some(Decimal::new(zaehler.checked_mul(faktor)?, 2).normalize())
}

fn einzel_eingabe(e: &mut Ein) -> Result<tarif::FestzusetzendeEstEinzelEingabe, String> {
    Ok(tarif::FestzusetzendeEstEinzelEingabe {
        bruttoarbeitslohn: geld(e, "bruttoarbeitslohn")?,
        werbungskosten: geld(e, "werbungskosten")?,
        sonderausgaben: geld(e, "sonderausgaben")?,
    })
}

fn gesamt_eingabe(e: &mut Ein) -> Result<tarif::GesamtEingabe, String> {
    Ok(tarif::GesamtEingabe {
        einkuenfte_nichtselbststaendig: geld(e, "einkuenfte_nichtselbststaendig")?,
        einkuenfte_kapitalvermoegen: geld(e, "einkuenfte_kapitalvermoegen")?,
        einkuenfte_vermietung: geld(e, "einkuenfte_vermietung")?,
        einkuenfte_sonstige: geld(e, "einkuenfte_sonstige")?,
        einkuenfte_gewinn: geld(e, "einkuenfte_gewinn")?,
        altersentlastungsbetrag: geld(e, "altersentlastungsbetrag")?,
        entlastungsbetrag_alleinerziehende: geld(e, "entlastungsbetrag_alleinerziehende")?,
        sonderausgaben: geld(e, "sonderausgaben")?,
        aussergewoehnliche_belastungen: geld(e, "aussergewoehnliche_belastungen")?,
        freibetraege_kinder: geld(e, "freibetraege_kinder")?,
        sonstige_abzuege_vom_einkommen: geld(e, "sonstige_abzuege_vom_einkommen")?,
        anzurechnende_auslaendische_steuern: geld(e, "anzurechnende_auslaendische_steuern")?,
        steuerermaessigungen: geld(e, "steuerermaessigungen")?,
        steuer_kapital_gesondert: geld(e, "steuer_kapital_gesondert")?,
        hinzurechnung_kindergeld: geld(e, "hinzurechnung_kindergeld")?,
        hinzurechnung_zulage: geld(e, "hinzurechnung_zulage")?,
        tarif_modifiziert: e.wahr("tarif_modifiziert")?,
        tarifliche_est_modifiziert: geld(e, "tarifliche_est_modifiziert")?,
    })
}

/// (Modul in `engine`, Funktion in `orakel.txt`, Adapter). Die Namen der Eingaben sind die der Catala-Scopes.
/// Die Reihenfolge der Eingaben im Adapter ist gleichgueltig: sie werden nach Namen gelesen.
const ADAPTER: &[(&str, &str, Adapter)] = &[
    ("agb", "agb_abzug", |e| {
        let eingabe = agb::AgbAbzugEingabe {
            aussergewoehnliche_belastungen: geld(e, "aussergewoehnliche_belastungen")?,
            zumutbare_belastung: geld(e, "zumutbare_belastung")?,
        };
        cent("abzug_agb", agb::berechnen(eingabe))
    }),
    ("altersentlastungsbetrag", "altersentlastungsbetrag", |e| {
        let arbeitslohn = geld(e, "arbeitslohn")?;
        let positive_andere_einkuenfte = geld(e, "positive_andere_einkuenfte")?;
        let prozentsatz = als_dezimal(e.bruch("prozentsatz")?);
        let hoechstbetrag = geld(e, "hoechstbetrag")?;
        let Some(prozentsatz) = prozentsatz else { return Ok(None) };
        let r = altersentlastungsbetrag::berechnen(altersentlastungsbetrag::AltersentlastungsbetragEingabe {
            arbeitslohn,
            positive_andere_einkuenfte,
            prozentsatz: Satz::new(prozentsatz),
            hoechstbetrag,
        });
        match r {
            Ok(c) => Ok(Some(einzel("altersentlastungsbetrag", Ok(c.get())))),
            Err(altersentlastungsbetrag::AltersentlastungsbetragFehler::Catala(f)) => {
                Ok(Some(einzel("altersentlastungsbetrag", Err(f))))
            }
            Err(altersentlastungsbetrag::AltersentlastungsbetragFehler::Dezimal(d)) => Err(d.to_string()),
        }
    }),
    ("arbeitszimmer", "raumkostenabzug", |e| {
        let eingabe = arbeitszimmer::ArbeitszimmerEingabe {
            arbeitszimmer_vorhanden: e.wahr("arbeitszimmer_vorhanden")?,
            ist_mittelpunkt: e.wahr("ist_mittelpunkt")?,
            tatsaechliche_aufwendungen: geld(e, "tatsaechliche_aufwendungen")?,
            jahrespauschale_gewaehlt: e.wahr("jahrespauschale_gewaehlt")?,
            monate_ohne_mittelpunkt: e.zahl("monate_ohne_mittelpunkt")?,
            homeoffice_tage: e.zahl("homeoffice_tage")?,
            jahrespauschale: geld(e, "jahrespauschale")?,
            tagespauschale_pro_tag: geld(e, "tagespauschale_pro_tag")?,
            tagespauschale_hoechstbetrag: geld(e, "tagespauschale_hoechstbetrag")?,
        };
        Ok(Some(arbeitszimmer::berechnen(eingabe).map(|r| {
            vec![
                ("abzug_arbeitszimmer", r.abzug_arbeitszimmer_cent()),
                ("abzug_homeoffice", r.abzug_homeoffice_cent()),
                ("abzug_gesamt", r.abzug_gesamt_cent()),
            ]
        })))
    }),
    ("berufsausbildung", "berufsausbildung", |e| {
        let eingabe = berufsausbildung::BerufsausbildungEingabe { aufwendungen: geld(e, "aufwendungen")? };
        cent("abziehbare_sonderausgaben", berufsausbildung::berechnen(eingabe))
    }),
    ("betriebsfreibetrag", "betriebs_freibetrag", |e| {
        let eingabe = betriebsfreibetrag::BetriebsFreibetragEingabe {
            veraeusserungsgewinn: geld(e, "veraeusserungsgewinn")?,
        };
        cent("freibetrag", betriebsfreibetrag::berechnen(eingabe))
    }),
    ("entfernungspauschale", "entfernungspauschale", |e| {
        let km = als_dezimal(e.bruch("entfernung_km_roh")?);
        let arbeitstage = e.zahl("arbeitstage")?;
        let eigenes_oder_ueberlassenes_kfz = e.wahr("eigenes_oder_ueberlassenes_kfz")?;
        let oepnv_kosten_jahr = geld(e, "oepnv_kosten_jahr")?;
        let satz_bis_20_km = geld(e, "satz_bis_20_km")?;
        let satz_ab_21_km = geld(e, "satz_ab_21_km")?;
        let staffelgrenze_km = e.zahl("staffelgrenze_km")?;
        let hoechstbetrag = geld(e, "hoechstbetrag")?;
        let Some(km) = km else { return Ok(None) };
        let r = entfernungspauschale::berechnen(entfernungspauschale::EntfernungspauschaleEingabe {
            entfernung_km_roh: Km::new(km),
            arbeitstage,
            eigenes_oder_ueberlassenes_kfz,
            oepnv_kosten_jahr,
            satz_bis_20_km,
            satz_ab_21_km,
            staffelgrenze_km,
            hoechstbetrag,
        });
        match r {
            Ok(r) => Ok(Some(Ok(vec![
                ("entfernungspauschale", r.entfernungspauschale_cent()),
                ("abziehbarer_betrag", r.abziehbarer_betrag_cent()),
            ]))),
            Err(entfernungspauschale::EntfernungspauschaleFehler::Catala(f)) => Ok(Some(Err(f))),
            Err(entfernungspauschale::EntfernungspauschaleFehler::Dezimal(d)) => Err(d.to_string()),
        }
    }),
    ("entlastungsbetrag", "entlastungsbetrag", |e| {
        let eingabe = entlastungsbetrag::EntlastungsbetragEingabe {
            alleinstehend: e.wahr("alleinstehend")?,
            anzahl_kinder: e.zahl("anzahl_kinder")?,
            monate_ohne_voraussetzung: e.zahl("monate_ohne_voraussetzung")?,
        };
        cent("entlastungsbetrag", entlastungsbetrag::berechnen(eingabe))
    }),
    ("euer", "euer_gewinn", |e| {
        let eingabe = euer::EuerEingabe {
            betriebseinnahmen: geld(e, "betriebseinnahmen")?,
            betriebsausgaben: geld(e, "betriebsausgaben")?,
        };
        cent("gewinn", euer::berechnen(eingabe))
    }),
    ("familienleistungsausgleich", "familienleistungsausgleich", |e| {
        let eingabe = familienleistungsausgleich::FamilienleistungsausgleichEingabe {
            est_ohne_freibetraege: geld(e, "est_ohne_freibetraege")?,
            est_mit_freibetraegen: geld(e, "est_mit_freibetraegen")?,
            kindergeld: geld(e, "kindergeld")?,
        };
        cent("est_nach_familienausgleich", familienleistungsausgleich::berechnen(eingabe))
    }),
    ("fuenftelregelung", "ermaessigter_durchschnittssatz", |e| {
        let eingabe = fuenftelregelung::FuenftelregelungEingabe {
            ao_einkuenfte: geld(e, "ao_einkuenfte")?,
            est_gesamt_zzgl_progression: geld(e, "est_gesamt_zzgl_progression")?,
            bemessungsgrundlage_durchschnitt: geld(e, "bemessungsgrundlage_durchschnitt")?,
        };
        cent("est_ao", fuenftelregelung::berechnen(eingabe))
    }),
    ("gwg", "gwg_sofortabzug", |e| {
        let eingabe = gwg::GwgEingabe { anschaffungskosten_netto: geld(e, "anschaffungskosten_netto")? };
        cent("sofortabzug", gwg::berechnen(eingabe))
    }),
    ("kirchensteuer", "kirchensteuerabzug", |e| {
        let eingabe = kirchensteuer::KirchensteuerabzugEingabe {
            gezahlte_kirchensteuer: geld(e, "gezahlte_kirchensteuer")?,
            erstattete_kirchensteuer: geld(e, "erstattete_kirchensteuer")?,
        };
        cent("abziehbare_kirchensteuer", kirchensteuer::berechnen(eingabe))
    }),
    ("mitunternehmer", "mitunternehmer_einkuenfte", |e| {
        let eingabe = mitunternehmer::MitunternehmerEingabe {
            gewinnanteil: geld(e, "gewinnanteil")?,
            verguetung_taetigkeit: geld(e, "verguetung_taetigkeit")?,
            verguetung_darlehen: geld(e, "verguetung_darlehen")?,
            verguetung_ueberlassung: geld(e, "verguetung_ueberlassung")?,
        };
        cent("einkuenfte_mitunternehmer", mitunternehmer::berechnen(eingabe))
    }),
    ("spenden", "spenden_abzug", |e| {
        let eingabe = spenden::SpendenAbzugEingabe {
            zuwendungen: geld(e, "zuwendungen")?,
            gesamtbetrag_der_einkuenfte: geld(e, "gesamtbetrag_der_einkuenfte")?,
        };
        cent("spenden_abzug", spenden::berechnen(eingabe))
    }),
    ("tarif", "grundtarif", |e| {
        let zve = geld(e, "zu_versteuerndes_einkommen")?;
        cent("tarifliche_steuer", tarif::grundtarif(zve, e.vz("veranlagungszeitraum")?))
    }),
    ("tarif", "splittingtarif", |e| {
        let zve = geld(e, "zu_versteuerndes_einkommen_gemeinsam")?;
        cent("tarifliche_steuer", tarif::splittingtarif(zve, e.vz("veranlagungszeitraum")?))
    }),
    ("tarif", "festzusetzende_est_einzel", |e| {
        let eingabe = einzel_eingabe(e)?;
        cent("festzusetzende_est", tarif::festzusetzende_est_einzel(eingabe, e.vz("veranlagungszeitraum")?))
    }),
    ("tarif", "festzusetzende_est_einzel_voll", |e| {
        let eingabe = einzel_eingabe(e)?;
        Ok(Some(est_ausgaben(tarif::festzusetzende_est_einzel_voll(eingabe, e.vz("veranlagungszeitraum")?))))
    }),
    ("tarif", "festzusetzende_est_zusammen", |e| {
        let eingabe = tarif::FestzusetzendeEstZusammenEingabe {
            bruttoarbeitslohn_a: geld(e, "bruttoarbeitslohn_a")?,
            werbungskosten_a: geld(e, "werbungskosten_a")?,
            bruttoarbeitslohn_b: geld(e, "bruttoarbeitslohn_b")?,
            werbungskosten_b: geld(e, "werbungskosten_b")?,
            sonderausgaben_gemeinsam: geld(e, "sonderausgaben_gemeinsam")?,
        };
        cent("festzusetzende_est", tarif::festzusetzende_est_zusammen(eingabe, e.vz("veranlagungszeitraum")?))
    }),
    ("tarif", "festzusetzende_est_gesamt", |e| {
        let eingabe = gesamt_eingabe(e)?;
        Ok(Some(est_ausgaben(tarif::festzusetzende_est_gesamt(eingabe, e.vz("veranlagungszeitraum")?))))
    }),
    ("tarif", "festzusetzende_est_gesamt_zusammen", |e| {
        let eingabe = gesamt_eingabe(e)?;
        Ok(Some(est_ausgaben(tarif::festzusetzende_est_gesamt_zusammen(eingabe, e.vz("veranlagungszeitraum")?))))
    }),
    ("verbilligte_vermietung", "verbilligte_vermietung_wk", |e| {
        let werbungskosten = geld(e, "werbungskosten")?;
        let quote = als_dezimal(e.bruch("entgelt_quote_prozent")?);
        let Some(quote) = quote else { return Ok(None) };
        let r = verbilligte_vermietung::berechnen(verbilligte_vermietung::VerbilligteVermietungEingabe {
            werbungskosten,
            entgelt_quote_prozent: Satz::new(quote),
        });
        match r {
            Ok(c) => Ok(Some(einzel("abziehbare_werbungskosten", Ok(c.get())))),
            Err(verbilligte_vermietung::VerbilligteVermietungFehler::Catala(f)) => {
                Ok(Some(einzel("abziehbare_werbungskosten", Err(f))))
            }
            Err(verbilligte_vermietung::VerbilligteVermietungFehler::Dezimal(d)) => Err(d.to_string()),
        }
    }),
    ("verlustvortrag", "verlustvortrag_abzug", |e| {
        let eingabe = verlustvortrag::VerlustvortragEingabe {
            gesamtbetrag_einkuenfte: geld(e, "gesamtbetrag_einkuenfte")?,
            verlustvortrag_bestand: geld(e, "verlustvortrag_bestand")?,
            zusammenveranlagung: e.wahr("zusammenveranlagung")?,
        };
        cent("verlustabzug", verlustvortrag::berechnen(eingabe))
    }),
    ("vorsorgeaufwendungen", "kranken_pflege_vorsorge", |e| {
        let eingabe = vorsorgeaufwendungen::VorsorgeaufwendungenEingabe {
            basis: geld(e, "basis_kv_pv")?,
            weitere: geld(e, "weitere_vorsorgeaufwendungen")?,
            mit_zuschuss: e.wahr("mit_anspruch_auf_zuschuss")?,
        };
        cent("abziehbare_kv_pv_vorsorge", vorsorgeaufwendungen::berechnen(eingabe))
    }),
    ("zumutbare_belastung", "zumutbare_belastung", |e| {
        let eingabe = zumutbare_belastung::ZumutbareBelastungEingabe {
            gesamtbetrag_der_einkuenfte: geld(e, "gesamtbetrag_der_einkuenfte")?,
            anzahl_kinder: e.zahl("anzahl_kinder")?,
            splitting: e.wahr("splitting")?,
        };
        cent("zumutbare_belastung", zumutbare_belastung::berechnen(eingabe))
    }),
];

/// Ruft den Adapter der Zeile; `Ok(None)` = ohne Entsprechung im Modul, `Err` = die Zeile passt nicht zum
/// Adapter (fehlende oder uebrige Eingabe).
fn rechne(adapter: Adapter, z: &Zeile) -> Result<Option<Ergebnis>, String> {
    let mut e = Ein(z.ein.clone());
    let ergebnis = adapter(&mut e)?;
    e.leer()?;
    Ok(ergebnis)
}

#[derive(Default)]
struct Auswertung {
    abgespielt: BTreeMap<&'static str, usize>,
    uebersprungen: BTreeMap<&'static str, usize>,
    abweichungen: Vec<String>,
}

fn spiele_ab(zeilen: &[Zeile]) -> Auswertung {
    let mut a = Auswertung::default();
    for z in zeilen {
        let Some(&(_, funktion, adapter)) = ADAPTER.iter().find(|(_, f, _)| *f == z.funktion) else {
            if !OHNE_MODUL.contains(&z.funktion.as_str()) {
                a.abweichungen.push(format!("Zeile {} ({}): kein Adapter, Funktion nicht in OHNE_MODUL", z.nr, z.funktion));
            }
            continue;
        };
        match rechne(adapter, z) {
            Ok(Some(ergebnis)) => {
                *a.abgespielt.entry(funktion).or_default() += 1;
                if let Some(m) = abweichung(z, &ergebnis) {
                    a.abweichungen.push(format!("Zeile {} ({funktion}): {m}", z.nr));
                }
            }
            Ok(None) => *a.uebersprungen.entry(funktion).or_default() += 1,
            Err(f) => a.abweichungen.push(format!("Zeile {} ({funktion}): {f}", z.nr)),
        }
    }
    a
}

#[test]
fn engine_rechnet_wie_der_catala_interpreter() {
    let zeilen = lies();
    let a = spiele_ab(&zeilen);
    for (_, funktion, _) in ADAPTER {
        println!(
            "[replay] {funktion}: {} abgespielt, {} uebersprungen",
            a.abgespielt.get(funktion).unwrap_or(&0),
            a.uebersprungen.get(funktion).unwrap_or(&0)
        );
    }
    assert!(
        a.abweichungen.is_empty(),
        "{} Abweichungen bei {} Zeilen, die ersten:\n{}",
        a.abweichungen.len(),
        zeilen.len(),
        a.abweichungen.iter().take(10).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn orakel_belegt_jeden_adapter_ausreichend() {
    let a = spiele_ab(&lies());
    for (_, funktion, _) in ADAPTER {
        let n = a.abgespielt.get(funktion).copied().unwrap_or(0);
        assert!(n >= MINDESTFAELLE, "{funktion}: {n} abgespielte Faelle in orakel.txt, verlangt sind {MINDESTFAELLE}");
    }
}

/// Die drei Dezimal-Scopes brauchen Zeilen, die der Adapter abbilden kann UND die Nachkommastellen haben:
/// ohne sie liefe der Nenner (VV3: `num / den` -> `num / 1`) an jeder Zeile vorbei.
#[test]
fn die_dezimal_scopes_haben_zeilen_mit_nachkommastellen() {
    let zeilen = lies();
    for (funktion, feld) in [
        ("altersentlastungsbetrag", "prozentsatz"),
        ("verbilligte_vermietung_wk", "entgelt_quote_prozent"),
        ("entfernungspauschale", "entfernung_km_roh"),
    ] {
        let n = zeilen
            .iter()
            .filter(|z| z.funktion == funktion)
            .filter_map(|z| Ein(z.ein.clone()).bruch(feld).ok())
            .filter(|&(zaehler, nenner)| {
                matches!(nenner, 10 | 100) && u64::try_from(zaehler).is_ok_and(|zl| zl % nenner != 0)
            })
            .count();
        assert!(n >= MINDEST_NACHKOMMA, "{funktion}: {n} Zeilen mit Nachkommastellen, verlangt sind {MINDEST_NACHKOMMA}");
    }
}

/// Jedes Modul in `engine` hat einen Adapter (oder steht mit Grund in `KEIN_ADAPTER`), und jede Scope-Huelle
/// von `catala_sys` hat einen (oder steht in `OHNE_MODUL`). Die Listen von Hand sind absichtlich: ein neues
/// Modul oder eine neue Huelle ohne Eintrag macht den Test rot, eine veraltete Ausnahme auch.
#[test]
fn jedes_engine_modul_und_jede_huelle_hat_einen_adapter() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine_lib = std::fs::read_to_string(manifest.join("src/lib.rs")).expect("engine/src/lib.rs");
    let module: Vec<&str> =
        engine_lib.lines().filter_map(|z| z.strip_prefix("pub mod ")).filter_map(|z| z.strip_suffix(';')).collect();
    assert!(module.len() >= 22, "weniger als 22 Module gefunden ({}), Suchmuster kaputt?", module.len());
    for m in &module {
        let hat = KEIN_ADAPTER.iter().any(|(n, _)| n == m) || ADAPTER.iter().any(|(n, _, _)| n == m);
        assert!(hat, "engine::{m} hat keinen Adapter und steht nicht in KEIN_ADAPTER");
    }
    for (n, _) in KEIN_ADAPTER {
        assert!(module.contains(n), "KEIN_ADAPTER nennt {n}, das es in engine nicht mehr gibt");
    }
    for (n, _, _) in ADAPTER {
        assert!(module.contains(n), "ADAPTER nennt engine::{n}, das es nicht gibt");
    }

    let sys_lib = std::fs::read_to_string(manifest.join("../catala-sys/src/lib.rs")).expect("catala-sys/src/lib.rs");
    // Alle `pub fn` ganz links in lib.rs (Methoden sind eingerueckt), ausser der Hilfsfunktion.
    let huellen: Vec<&str> = sys_lib
        .lines()
        .filter_map(|z| z.strip_prefix("pub fn "))
        .filter_map(|z| z.split(['(', '<']).next())
        .filter(|n| *n != "recompute_source_hash")
        .collect();
    assert!(huellen.len() >= 26, "weniger als 26 Huellen gefunden ({}), Suchmuster kaputt?", huellen.len());
    for h in &huellen {
        let hat = OHNE_MODUL.contains(h) || ADAPTER.iter().any(|(_, f, _)| f == h);
        assert!(hat, "catala_sys::{h} hat keinen Adapter und steht nicht in OHNE_MODUL");
    }
    for n in OHNE_MODUL {
        assert!(huellen.contains(n), "OHNE_MODUL nennt {n}, das catala_sys nicht mehr hat");
    }
}

/// Der Vergleich ist scharf: je Adapter faellt ein um einen Cent verfaelschter Sollwert auf. Das verhindert
/// einen Adapter, der nichts oder eine leere Ausgabeliste vergleicht (dann waere jeder Lauf gruen).
#[test]
fn der_vergleich_sieht_einen_verfaelschten_sollwert() {
    let zeilen = lies();
    for &(_, funktion, adapter) in ADAPTER {
        let probe = zeilen
            .iter()
            .filter(|z| z.funktion == funktion)
            .filter(|z| z.aus.as_ref().is_some_and(|aus| !aus.is_empty()))
            .find_map(|z| rechne(adapter, z).ok().flatten().map(|ergebnis| (z, ergebnis)));
        let (z, ergebnis) = probe.unwrap_or_else(|| panic!("{funktion}: keine abspielbare Zeile mit Ausgabe"));
        assert!(abweichung(z, &ergebnis).is_none(), "{funktion}: Zeile {} weicht schon ohne Eingriff ab", z.nr);
        let mut aus = z.aus.clone().unwrap_or_default();
        if let Some((_, wert)) = aus.first_mut() {
            *wert += 1;
        }
        let falsch = Zeile { nr: z.nr, funktion: z.funktion.clone(), ein: z.ein.clone(), aus: Some(aus) };
        assert!(abweichung(&falsch, &ergebnis).is_some(), "{funktion}: ein verfaelschter Sollwert faellt nicht auf");
    }
}
