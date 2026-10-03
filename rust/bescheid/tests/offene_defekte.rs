//! Offene Defekte der Rechen- und Sperrseite: je Python-`xfail(strict=True)`-Fall ein Gegenstueck,
//! das das RICHTIGE Verhalten verlangt. `#[ignore]` haelt `cargo test` gruen (ein dauerhaft roter
//! Test zerstoert das Signal, `73d4245`); `cargo test -p bescheid --test offene_defekte --
//! --ignored` zeigt jeden Fall rot. Der Ignore-Text nennt Grund, Python-Test und Vault-Notiz.
//!
//! Lesart des Rots: die Defekt-Zusicherung beginnt mit `DEFEKT:`. Eine Kontrolle (Messaufbau,
//! Normalfall) beginnt mit `KONTROLLE:`; scheitert sie, misst der Test nichts.
//!
//! In Rust liegen `GET /ergebnis` und `GET /deklaration` im Crate `api` (`api/src/ergebnis.rs`,
//! `api/src/deklaration.rs`), das von `bescheid` abhaengt. Die Tests komponieren darum hier, was die
//! Python-Handler komponieren: [`ergebnis`] wie `_ergebnis_roh`
//! (`api.py:568`), [`deklaration`] wie `deklaration` (`api.py:664`). Die Felder entstehen ueber
//! den Kegel-Bauer (`tests/_kegel.py`), alle bestaetigt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::{HashMap, HashSet};

use bescheid::deklaration::{an_gesamt_sperrgrund, feste_zahl, mit_ring_werten, Cfg, KeineZahl};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use bescheid::{Felder, Instanzquelle};
use bindung::Bindung;
use domain::{Feldtyp, Scheibe, Sperrgrund, Vz};
use intervall::AchsenBindung;
use konsistenz::FlagStand;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2025;

/// `dict(paare)`: der letzte Wert je Feld gewinnt, die Reihenfolge ist die des ersten Auftretens.
fn als_dict(paare: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut raus: Vec<(&'static str, Value)> = Vec::new();
    for (f, w) in paare {
        match raus.iter_mut().find(|(g, _)| g == f) {
            Some(e) => e.1 = w.clone(),
            None => raus.push((f, w.clone())),
        }
    }
    raus
}

/// `tests/_kegel.py::standardwert`: der Abwesenheitswert, nie der illustrative Beispielwert.
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

/// `tests/_kegel.py::kegel_fuer`: der volle Pflicht-Kegel; was der Test setzt, gewinnt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let gesetzt = als_dict(gesetzt);
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Vec<(&'static str, Value)> = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .find(|(g, _)| g == f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            (*f, w)
        })
        .collect();
    raus.extend(gesetzt.into_iter().filter(|(f, _)| !kegel.contains(f)));
    raus
}

/// Ein Fall einer Scheibe, mit der Scheiben-Bindung (`api._scheibe_bindung`) als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

/// `paare` kommen bestaetigt ueber den Kegel-Bauer; `vorlaeufig` danach als `zustand=vorlaeufig`,
/// und der Bauer belegt diese Felder nicht vor (`auslassen` in den Python-Fixturen).
fn fall(
    scheibe: Scheibe,
    paare: &[(&'static str, Value)],
    vorlaeufig: &[(&'static str, Value)],
) -> Fall {
    let cfg = Cfg::fuer(scheibe);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: {scheibe} liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let events: Vec<(&str, Value, bool)> = kegel_fuer(&cfg, paare)
        .into_iter()
        .filter(|(f, _)| !vorlaeufig.iter().any(|(v, _)| v == f))
        .map(|(f, w)| (f, w, true))
        .chain(vorlaeufig.iter().map(|(f, w)| (*f, w.clone(), false)))
        .collect();
    let store = store(&events);
    Fall {
        cfg,
        index: teil.iter().map(|b| (b.feld_id.clone(), *b)).collect(),
        achsen: teil.iter().map(|b| AchsenBindung::from(*b)).collect(),
        felder: felder(&store),
        store,
    }
}

/// `grund` aus `_ergebnis_roh`: bestaetigt, ein Sperrgrund des Guards oder eine Lage ohne Zahl.
#[derive(Debug, PartialEq)]
enum Grund {
    Bestaetigt,
    Sperre(Sperrgrund),
    KeineZahl(KeineZahl),
}

/// `_ergebnis_roh` (`api.py:568`) ohne HTTP: erst der K2-Guard, dann `feste_zahl`.
fn ergebnis(f: &Fall) -> (Grund, Option<i64>) {
    if f.cfg.guard() {
        let q = Instanzquelle {
            store: Some(&f.store),
            bindung: Some(&f.index),
            nur_bestaetigt: false,
        };
        if let Some(g) = an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q).unwrap() {
            return (Grund::Sperre(g), None);
        }
    }
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None).unwrap() {
        Ok(z) => (Grund::Bestaetigt, Some(z.zahl.get())),
        Err(k) => (Grund::KeineZahl(k.grund), None),
    }
}

/// `_mit_ring_werten`: der Snapshot, den `deklaration` und `einreichen` sehen.
fn ring_felder(f: &Fall) -> Felder {
    let mut felder = f.felder.clone();
    mit_ring_werten(&mut felder, Some(VZ), params()).unwrap();
    felder
}

/// `deklaration` (`api.py:664`) ohne HTTP: Ring-Werte, dann `elster::deklariere`. Ohne Guard.
fn deklaration(f: &Fall) -> elster::Deklaration {
    elster::deklariere(&ring_felder(f), &f.index, i64::from(VZ.jahr()), None).unwrap()
}

/// Die Feldmenge der Scheibe als `HashSet` — die Sicht, die `flag_widersprueche` als
/// `scheibe_felder` bekommt und die `api.py::event()` beim Scheiben-Zuschnitt anlegt.
fn scheiben_felder(scheibe: Scheibe) -> HashSet<String> {
    Cfg::fuer(scheibe)
        .felder(|d| panic!("KONTROLLE: {scheibe} liest Felder aus {d}"))
        .unwrap()
        .into_iter()
        .collect()
}

/// `einreichen` ab `api.py:745`: `erzeuge_xml(..., abgabefaehig=True, snapshot=felder)`.
fn xml(f: &Fall, d: &elster::Deklaration) -> String {
    let felder = ring_felder(f);
    let opt = elster::XmlOptionen {
        hersteller_id: Some("74931".to_owned()),
        snapshot: Some(&felder),
        abgabefaehig: true,
        ..elster::XmlOptionen::default()
    };
    elster::erzeuge_xml(d, &opt).unwrap_or_else(|e| panic!("KONTROLLE: erzeuge_xml scheitert: {e}"))
}

/// Kz-Wert als Zahl (`dek.get(kz)`).
fn kz(d: &elster::Deklaration, name: &str) -> Option<f64> {
    d.deklaration.get(name).and_then(Value::as_f64)
}

// ---------------------------------------------------------------- Gewinn-Quelle

/// `_STAMM` der Gewinn- und KAP-Topf-Tests (gleich): keine Kegel-Felder.
fn stamm_meier() -> Vec<(&'static str, Value)> {
    vec![
        ("stammdaten_nachname", json!("Meier")),
        ("stammdaten_vorname", json!("Klaus")),
        ("stammdaten_geburtsdatum", json!("01.01.1970")),
        ("stammdaten_strasse", json!("Teststr.")),
        ("stammdaten_hausnummer", json!("1")),
        ("stammdaten_plz", json!("10115")),
        ("stammdaten_wohnort", json!("Berlin")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("kist_konfession", json!("keine")),
    ]
}

/// `_BASIS + _PFLICHT_ZUSATZ + veranlagung + Betriebsart` beider Gewinn-Quellen-Tests (gleich).
fn gewinn_basis(betriebsart: &str) -> Vec<(&'static str, Value)> {
    let mut paare = stamm_meier();
    paare.extend([
        ("bruttoarbeitslohn", json!(0)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("kap_kapitalertraege", json!(0)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
        ("kein_gewinn", json!(false)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("basis_kv", json!(0)),
        ("basis_pv", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(false)),
        ("ep_arbeitstage", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("veranlagung", json!("einzel")),
        ("gewinn_betriebsart", json!(betriebsart)),
        ("gewinn_bezeichnung", json!("Testfall")),
    ]);
    paare
}

/// EUeR-Cluster beider Tests: 50.000 - 10.000 - 5.000 = 35.000 EUR laufender Gewinn.
fn euer() -> [(&'static str, Value); 3] {
    [
        ("betriebseinnahmen", json!(5_000_000)),
        ("sonstige_betriebsausgaben", json!(1_000_000)),
        ("afa_jahresbetrag", json!(500_000)),
    ]
}

/// `test_gewinn_quelle_offen_trifft_alle_betriebsarten.py`: Summe UND passende EUeR-Aufschluesselung
/// liefern einen Bescheid. Kontrolle: nur die Summe (35.000 EUR) liefert 575700 Cent.
fn gewinn_quelle(betriebsart: &str) {
    let lauf = |mit_euer: bool| {
        let mut paare = gewinn_basis(betriebsart);
        if mit_euer {
            paare.extend(euer());
        }
        paare.push(("einkuenfte_gewinn", json!(3_500_000)));
        ergebnis(&fall(Scheibe::Gesamt, &paare, &[]))
    };
    assert_eq!(
        lauf(false),
        (Grund::Bestaetigt, Some(575_700)),
        "KONTROLLE: nur die Summe, {betriebsart}"
    );
    let (grund, zahl) = lauf(true);
    assert!(
        zahl.is_some() && grund != Grund::Sperre(Sperrgrund::GewinnQuelleOffen),
        "DEFEKT: {betriebsart}, EUeR + passende Summe liefert keinen Bescheid (grund={grund:?}, zahl_cent={zahl:?})"
    );
}

#[test]
#[ignore = "gewinn_quelle_offen (sperre/einkunft.rs, fn gewinn) sperrt bei Direktwert > 0 UND EUeR-Komponente > 0 ohne Abfrage von gewinn_betriebsart, auch wenn die Summe zur EUeR passt. Python: test_gewinn_quelle_offen_trifft_alle_betriebsarten.py::test_euer_und_passende_summe_liefert_keinen_bescheid[land_forst]. Vault: audits/einreichen-sperrgruende-erreichen-den-nutzer-nie.md. Rot sehen: --ignored"]
fn gewinn_quelle_offen_land_forst() {
    gewinn_quelle("land_forst");
}

#[test]
#[ignore = "gewinn_quelle_offen (sperre/einkunft.rs, fn gewinn) sperrt bei Direktwert > 0 UND EUeR-Komponente > 0 ohne Abfrage von gewinn_betriebsart, auch wenn die Summe zur EUeR passt. Python: test_gewinn_quelle_offen_trifft_alle_betriebsarten.py::test_euer_und_passende_summe_liefert_keinen_bescheid[gewerbe]. Vault: audits/einreichen-sperrgruende-erreichen-den-nutzer-nie.md. Rot sehen: --ignored"]
fn gewinn_quelle_offen_gewerbe() {
    gewinn_quelle("gewerbe");
}

#[test]
#[ignore = "gewinn_quelle_offen (sperre/einkunft.rs, fn gewinn) sperrt bei Direktwert > 0 UND EUeR-Komponente > 0 ohne Abfrage von gewinn_betriebsart, auch wenn die Summe zur EUeR passt. Python: test_gewinn_quelle_offen_trifft_alle_betriebsarten.py::test_euer_und_passende_summe_liefert_keinen_bescheid[selbstaendig]. Vault: audits/einreichen-sperrgruende-erreichen-den-nutzer-nie.md. Rot sehen: --ignored"]
fn gewinn_quelle_offen_selbstaendig() {
    gewinn_quelle("selbstaendig");
}

/// Direktwert 0 neben positiver `EUeR`: `grund`, `zahl_cent` und der Kz-Wert der Deklaration.
fn luf_euer_lauf(betriebsart: &str, kz: &str) -> (Grund, Option<i64>, Option<Value>) {
    let mut paare = gewinn_basis(betriebsart);
    paare.extend(euer());
    paare.push(("einkuenfte_gewinn", json!(0)));
    let f = fall(Scheibe::Gesamt, &paare, &[]);
    let (grund, zahl) = ergebnis(&f);
    (grund, zahl, deklaration(&f).deklaration.get(kz).cloned())
}

/// `test_luf_euer_offen_asymmetrie.py`: nie zugleich `bestaetigt`, eine positive Zahl und eine
/// deklarierte 0. Kontrolle: `land_forst` sperrt mit `luf_euer_offen`.
fn luf_euer_symmetrie(betriebsart: &str, kz: &str) {
    let (grund, zahl, _) = luf_euer_lauf("land_forst", kz);
    assert_eq!(
        (grund, zahl),
        (Grund::Sperre(Sperrgrund::LufEuerOffen), None),
        "KONTROLLE: land_forst sperrt bei Doppelquelle"
    );
    let (grund, zahl, kz_wert) = luf_euer_lauf(betriebsart, kz);
    let deklariert_null = kz_wert.as_ref().and_then(Value::as_f64) == Some(0.0);
    assert!(
        !(grund == Grund::Bestaetigt && zahl.is_some_and(|z| z != 0) && deklariert_null),
        "DEFEKT: {betriebsart}, grund={grund:?}, zahl_cent={zahl:?}, {kz}={kz_wert:?}: Ring und Deklaration laufen auseinander"
    );
}

#[test]
#[ignore = "luf_euer_offen (sperre/einkunft.rs, fn gewinn) prueft woertlich gewinn_betriebsart == land_forst; gewerbe rechnet aus der EUeR und deklariert E0800302 = 0. Python: test_luf_euer_offen_asymmetrie.py::test_gewerbeartige_zweige_liefern_bescheid_trotz_doppelquelle[gewerbe]. Vault: audits/ring-gegen-deklaration-ueber-und-untererklaerung.md. Rot sehen: --ignored"]
fn luf_euer_offen_gewerbe() {
    luf_euer_symmetrie("gewerbe", "E0800302");
}

#[test]
#[ignore = "luf_euer_offen (sperre/einkunft.rs, fn gewinn) prueft woertlich gewinn_betriebsart == land_forst; selbstaendig rechnet aus der EUeR und deklariert E0803202 = 0. Python: test_luf_euer_offen_asymmetrie.py::test_gewerbeartige_zweige_liefern_bescheid_trotz_doppelquelle[selbstaendig]. Vault: audits/ring-gegen-deklaration-ueber-und-untererklaerung.md. Rot sehen: --ignored"]
fn luf_euer_offen_selbstaendig() {
    luf_euer_symmetrie("selbstaendig", "E0803202");
}

// ---------------------------------------------------------------- Vorlaeufige Werte in der Deklaration

/// `_GRUND_BASIS + _STAMM` beider Leck-Tests (KAP und Verpflegung, gleich); auch Kern von p20.
fn leck_basis() -> Vec<(&'static str, Value)> {
    vec![
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("vor_an_anteil_rv", json!(4_200_000)),
        ("vor_ag_anteil_rv", json!(1_200_000)),
        ("stammdaten_nachname", json!("Maier")),
        ("stammdaten_vorname", json!("Hans")),
        ("stammdaten_geburtsdatum", json!("05.05.1955")),
        ("stammdaten_strasse", json!("Musterstr.")),
        ("stammdaten_hausnummer", json!("55")),
        ("stammdaten_plz", json!("55555")),
        ("stammdaten_wohnort", json!("Musterort")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("kist_konfession", json!("keine")),
        ("stammdaten_steuernummer", json!("9181081508155")),
        ("steuerklasse", json!("1")),
        ("p36_lohnsteuer", json!(1_200_000)),
    ]
}

/// Ein Leck-Fall: `grund`, `zahl_cent` und die Deklaration.
fn leck_lauf(
    gesetzt: &[(&'static str, Value)],
    vorlaeufig: &[(&'static str, Value)],
) -> (Grund, Option<i64>, elster::Deklaration) {
    let mut paare = leck_basis();
    paare.extend_from_slice(gesetzt);
    let f = fall(Scheibe::Gesamt, &paare, vorlaeufig);
    let (grund, zahl) = ergebnis(&f);
    (grund, zahl, deklaration(&f))
}

/// Kz-Wert ist da und nicht 0/false (Pythons `bool(dek.get(kz))`).
fn gesetzt(d: &elster::Deklaration, kz: &str) -> bool {
    d.deklaration
        .get(kz)
        .is_some_and(|w| !matches!(w, Value::Null | Value::Bool(false)) && w.as_f64() != Some(0.0))
}

fn unvollstaendig(d: &elster::Deklaration) -> Vec<&str> {
    d.unvollstaendig()
        .iter()
        .map(|e| e.feld_id.as_str())
        .collect()
}

/// `test_kap_deklaration_vorlaeufig_leck_ohne_bestaetigung.py`. BEHOBEN 2026-10-03 (Entscheid
/// kap-vorschau-liest-nur-bestaetigte-werte): `kap_antrag` (`ring_werte.rs`) liest nur bestaetigte Werte,
/// der Test ist kein offener Defekt mehr und laeuft ohne `#[ignore]`. Gewollte Abweichung: Python
/// verlangt `zahl_cent(leck) == zahl_cent(baseline)` (Zeile 295) und scheitert dort seit dem
/// Kegel-Zuwachs (gemessen `None == 753400`, Lage 2) - vor der Kernaussage. Hier steht die Absicht
/// der Vorbedingung: der vorlaeufige Topf geht nicht in die Steuer ein.
#[test]
fn kap_vorlaeufiger_topf_leckt_in_deklaration() {
    let (_, basis, _) = leck_lauf(&[("kein_kap", json!(true))], &[]);
    let topf = [("kap_gewinn_sonstige", json!(175_000))];
    let (_, gruen_zahl, gruen) = leck_lauf(&[("kein_kap", json!(false)), topf[0].clone()], &[]);
    assert!(
        basis.is_some() && gruen_zahl > basis,
        "KONTROLLE: bestaetigter Topf erhoeht die Steuer nicht ({basis:?} -> {gruen_zahl:?})"
    );
    assert!(
        gruen.eingaben_konsistent() && gesetzt(&gruen, "E1900401") && gesetzt(&gruen, "E1901401"),
        "KONTROLLE: bestaetigter Topf erscheint nicht als E1900401/E1901401: {:?}",
        gruen.deklaration
    );
    let (grund, zahl, leck) = leck_lauf(&[("kein_kap", json!(false))], &topf);
    assert!(
        zahl.is_none() || zahl == basis,
        "KONTROLLE: der vorlaeufige Topf bewegt die Steuer ({basis:?} -> {zahl:?}, {grund:?})"
    );
    assert!(
        !leck.eingaben_konsistent() && unvollstaendig(&leck).contains(&"kap_gewinn_sonstige"),
        "KONTROLLE: die Deklaration meldet den vorlaeufigen Topf nicht als unvollstaendig"
    );
    assert!(
        !gesetzt(&leck, "E1900401") && !gesetzt(&leck, "E1901401"),
        "DEFEKT: eingaben_konsistent=false, aber E1900401={:?}, E1901401={:?} aus dem vorlaeufigen Topf",
        leck.deklaration.get("E1900401"),
        leck.deklaration.get("E1901401")
    );
}

/// AK4 der KAP-Vorschau: `unvollstaendig` nennt das Feld, aber weder Antrag noch genutzter Pauschbetrag
/// stehen in der Deklaration.
fn kein_antrag(d: &elster::Deklaration, feld: &str) {
    assert!(
        !d.eingaben_konsistent() && unvollstaendig(d).contains(&feld),
        "KONTROLLE: die Deklaration meldet {feld} nicht als unvollstaendig"
    );
    assert!(
        !gesetzt(d, "E1900401") && !gesetzt(d, "E1901401"),
        "DEFEKT: {feld} nur vorlaeufig, aber E1900401={:?}, E1901401={:?}",
        d.deklaration.get("E1900401"),
        d.deklaration.get("E1901401")
    );
}

/// `kap_kapitalertraege` (das Aggregat statt eines Topfs) nur vorlaeufig.
#[test]
fn kap_vorlaeufiges_aggregat_leckt_nicht_in_deklaration() {
    let (_, _, leck) = leck_lauf(
        &[("kein_kap", json!(false))],
        &[("kap_kapitalertraege", json!(175_000))],
    );
    kein_antrag(&leck, "kap_kapitalertraege");
}

/// Gemischter Zustand: Aktiengewinn 400 EUR bestaetigt, sonstiger Gewinn 300 EUR nur vorlaeufig. Der Antrag
/// steht, der genutzte Pauschbetrag zaehlt nur die 400 EUR (vorher 700), gleich der Kontrolle ohne den
/// vorlaeufigen Topf.
#[test]
fn kap_gemischter_zustand_zaehlt_nur_den_bestaetigten_topf() {
    let paare = [
        ("kein_kap", json!(false)),
        ("kap_gewinn_aktien", json!(40_000)),
    ];
    let (_, _, kontrolle) = leck_lauf(&paare, &[]);
    assert!(
        kontrolle.eingaben_konsistent()
            && gesetzt(&kontrolle, "E1900401")
            && kz(&kontrolle, "E1901401") == Some(400.0),
        "KONTROLLE: bestaetigter Aktiengewinn traegt nicht E1900401 und E1901401=400: {:?}",
        kontrolle.deklaration
    );
    let (_, _, gemischt) = leck_lauf(&paare, &[("kap_gewinn_sonstige", json!(30_000))]);
    assert!(
        !gemischt.eingaben_konsistent()
            && unvollstaendig(&gemischt).contains(&"kap_gewinn_sonstige")
            && gesetzt(&gemischt, "E1900401"),
        "KONTROLLE: gemischter Zustand ohne Antrag oder ohne unvollstaendig-Meldung: {:?}",
        gemischt.deklaration
    );
    assert_eq!(
        kz(&gemischt, "E1901401"),
        kz(&kontrolle, "E1901401"),
        "DEFEKT: der vorlaeufige Topf geht in den genutzten Sparer-Pauschbetrag ein"
    );
}

/// Partner-KAP in `gesamt` ist NICHT erreichbar: der Guard verlangt alle Partner-KAP-Felder bestaetigt und
/// sperrt mit `partner_kegel_offen`, noch bevor `mit_ring_werten` liest. Kontrolle: bestaetigt, keine Sperre.
#[test]
fn kap_partner_in_gesamt_sperrt_der_guard_vor_der_vorschau() {
    let mut paare = vec![
        ("veranlagung", json!("zusammen")),
        ("kein_kap_partner", json!(false)),
    ];
    paare.extend(
        [
            "bruttoarbeitslohn_partner",
            "kap_kapitalertraege_partner",
            "kap_gewinn_aktien_partner",
            "kap_verlust_aktien_partner",
            "kap_verlust_sonstige_partner",
        ]
        .map(|f| (f, json!(0))),
    );
    let topf = ("kap_gewinn_sonstige_partner", json!(175_000));
    let (grund, _, _) = leck_lauf(&paare, std::slice::from_ref(&topf));
    assert_eq!(
        grund,
        Grund::Sperre(Sperrgrund::PartnerKegelOffen),
        "vorlaeufiger Partner-Topf in gesamt sperrt nicht mit partner_kegel_offen"
    );
    paare.push(topf);
    let (grund, _, dek) = leck_lauf(&paare, &[]);
    assert!(
        !matches!(grund, Grund::Sperre(_))
            && dek.eingaben_konsistent()
            && gesetzt(&dek, "E1900401"),
        "KONTROLLE: bestaetigter Partner-Topf in gesamt: {grund:?}, {:?}",
        dek.deklaration
    );
}

/// `rentner_gesamt`, Zusammenveranlagung, eine Rente: kein Partner-KAP-Guard, keine KAP-Betraege im Kegel,
/// `/deklaration` laeuft ohne Guard-Treffer. Hier ist ein vorlaeufiger KAP-Wert erreichbar: eigener Topf,
/// Partner-Topf und Partner-Aggregat duerfen keinen Antrag ausloesen.
#[test]
fn kap_vorlaeufiger_wert_in_rentner_gesamt_leckt_nicht_in_deklaration() {
    let rentner = |gesetzt: &[(&'static str, Value)], vorlaeufig: &[(&'static str, Value)]| {
        let mut paare = vec![
            ("rentner_jahresrente", json!(2_000_000)),
            ("rentner_renten_beginn_jahr", json!(2025)),
            ("kein_sonstige", json!(false)),
            ("veranlagung", json!("zusammen")),
        ];
        paare.extend_from_slice(gesetzt);
        deklaration(&fall(Scheibe::RentnerGesamt, &paare, vorlaeufig))
    };
    let gruen = rentner(&[("kap_gewinn_sonstige_partner", json!(175_000))], &[]);
    assert!(
        gruen.eingaben_konsistent()
            && gesetzt(&gruen, "E1900401")
            && kz(&gruen, "E1901401") == Some(1750.0),
        "KONTROLLE: bestaetigter Partner-Topf in rentner_gesamt traegt keinen Antrag: {:?}",
        gruen.deklaration
    );
    for (feld, gesetzt_) in [
        ("kap_gewinn_sonstige", vec![("kein_kap", json!(false))]),
        ("kap_gewinn_sonstige_partner", vec![]),
        ("kap_kapitalertraege_partner", vec![]),
    ] {
        let leck = rentner(&gesetzt_, &[(feld, json!(175_000))]);
        kein_antrag(&leck, feld);
    }
}

/// `test_verpflegung_kuerzung_deklaration_vorlaeufig_widerspruch.py`. BEHOBEN 2026-10-03 (Entscheid
/// verpflegung-vorschau-liest-nur-bestaetigte-werte): `verpflegung` und `kuerzung_cent` (`ring_werte.rs`)
/// lesen nur bestaetigte Werte, der Test ist kein offener Defekt mehr und laeuft ohne `#[ignore]`. Der Leck-Fall
/// hat kein `vpf_monate_am_ort`, deshalb sperrt ein Waechter (Python: `verpflegung_dreimonatsfrist_aufteilung_offen`)
/// die Zahl; die Vorbedingung prueft nur, dass irgendein Waechter sperrt.
#[test]
fn verpflegung_vorlaeufige_tage_lecken_in_deklaration() {
    let basis = [
        ("veranlagung", json!("einzel")),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ];
    let tage = [
        ("tage_24h", json!(100)),
        ("vpf_fruehstuecke_gestellt_anzahl", json!(5)),
    ];
    let monate = [("vpf_monate_am_ort", json!(2))];
    let (_, _, gruen) = leck_lauf(&[&basis[..], &tage, &monate].concat(), &[]);
    assert!(
        gruen.eingaben_konsistent()
            && gruen.deklaration.get("E0205409") == Some(&json!(100))
            && gesetzt(&gruen, "E0205508"),
        "KONTROLLE: bestaetigte Tage erscheinen nicht als E0205409/E0205508: {:?}",
        gruen.deklaration
    );
    let (grund, zahl, leck) = leck_lauf(&basis, &tage);
    assert!(
        zahl.is_none() && matches!(grund, Grund::Sperre(_)),
        "KONTROLLE: kein Waechter sperrt die vorlaeufigen Tage ({grund:?}, {zahl:?})"
    );
    let u = unvollstaendig(&leck);
    assert!(
        !leck.eingaben_konsistent()
            && u.contains(&"tage_24h")
            && u.contains(&"vpf_fruehstuecke_gestellt_anzahl"),
        "KONTROLLE: die Deklaration meldet die vorlaeufigen Tage nicht als unvollstaendig: {u:?}"
    );
    assert!(
        !gesetzt(&leck, "E0205409") && !gesetzt(&leck, "E0205508"),
        "DEFEKT: eingaben_konsistent=false, aber E0205409={:?}, E0205508={:?} aus vorlaeufigen Tagen",
        leck.deklaration.get("E0205409"),
        leck.deklaration.get("E0205508")
    );
}

/// `tests/test_verpflegung_vorschau_liest_nur_bestaetigte_werte.py`: je Feld drei Faelle (bestaetigt / fehlt /
/// vorlaeufig), alles andere bestaetigt, `vpf_monate_am_ort = 2` (der Waechter sperrt nicht). Ein vorlaeufiges
/// Feld zaehlt wie ein fehlendes: dieselbe E0205508-Zeile. Basis M: 100 Tage `tage_24h`, die Mahlzeiten binden
/// die Kuerzung. Basis P: je 10 Mahlzeiten, die Tage binden sie.
#[test]
fn verpflegung_vorlaeufiges_feld_zaehlt_wie_fehlendes() {
    type Paare = Vec<(&'static str, Value)>;
    let ohne = |b: &Paare, feld: &str| -> Paare {
        b.iter().filter(|(f, _)| *f != feld).cloned().collect()
    };
    let m: Paare = vec![
        ("tage_24h", json!(100)),
        ("vpf_fruehstuecke_gestellt_anzahl", json!(5)),
        ("vpf_mittagessen_gestellt_anzahl", json!(2)),
        ("vpf_abendessen_gestellt_anzahl", json!(2)),
    ];
    let mahlzeiten_p: Paare = vec![
        ("vpf_fruehstuecke_gestellt_anzahl", json!(10)),
        ("vpf_mittagessen_gestellt_anzahl", json!(10)),
        ("vpf_abendessen_gestellt_anzahl", json!(10)),
    ];
    let tage_p: Paare = vec![
        ("tage_24h", json!(3)),
        ("tage_an_abreise", json!(2)),
        ("tage_ueber_8h_eintaegig", json!(2)),
    ];
    let p: Paare = [tage_p.clone(), mahlzeiten_p.clone()].concat();
    let nach: Paare = vec![
        ("vpf_tage_24h_nach_drei_monaten", json!(2)),
        ("vpf_tage_an_abreise_nach_drei_monaten", json!(1)),
        ("vpf_tage_ueber_8h_nach_drei_monaten", json!(1)),
    ];
    // (Name, Feld, Wert, bestaetigte Basis ohne das Feld)
    let mut faelle: Vec<(&str, &'static str, Value, Paare)> = vec![];
    for &(f, ref w) in &m[1..] {
        faelle.push((f, f, w.clone(), ohne(&m, f)));
    }
    faelle.push((
        "entgelt",
        "vpf_mahlzeiten_gezahltes_entgelt",
        json!(2000),
        m.clone(),
    ));
    for &(f, ref w) in &tage_p {
        faelle.push((f, f, w.clone(), ohne(&p, f)));
    }
    // Ein einziger vorlaeufiger Tage-Topf: ohne ihn keine Zeile, mit ihm (vorher) 84 EUR.
    faelle.push(("einziger_tage_topf", "tage_24h", json!(3), mahlzeiten_p));
    for &(f, ref w) in &nach {
        let basis = [
            p.clone(),
            nach.iter()
                .filter(|(g, _)| *g != f)
                .map(|(g, _)| (*g, json!(0)))
                .collect(),
        ]
        .concat();
        faelle.push((f, f, w.clone(), basis));
    }
    assert_eq!(faelle.len(), 11);
    let basis: Paare = vec![
        ("veranlagung", json!("einzel")),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("vpf_monate_am_ort", json!(2)),
    ];
    for (name, feld, wert, bestaetigt) in faelle {
        let gesetzt: Paare = [basis.clone(), bestaetigt].concat();
        let zeile = |g: &Paare, vorlaeufig: &[(&'static str, Value)]| {
            kz(&leck_lauf(g, vorlaeufig).2, "E0205508")
        };
        let fehlt = zeile(&gesetzt, &[]);
        let bestaetigt = zeile(&[gesetzt.clone(), vec![(feld, wert.clone())]].concat(), &[]);
        let vorlaeufig = zeile(&gesetzt, &[(feld, wert)]);
        assert_ne!(
            bestaetigt, fehlt,
            "KONTROLLE: {name}: {feld} bestaetigt aendert E0205508 nicht ({fehlt:?}), der Fall misst nichts"
        );
        assert_eq!(
            vorlaeufig, fehlt,
            "DEFEKT: {name}: {feld} nur vorlaeufig, E0205508 = {vorlaeufig:?}, ohne das Feld {fehlt:?} (bestaetigt {bestaetigt:?})"
        );
    }
}

// ---------------------------------------------------------------- KAP-Topf ohne Kennzahl

/// `_kap(...)` aus p20; ohne `kein_kap` sind es `_KAP_NULL`/`_KAP_NUR_SONSTIGE`.
fn kap(kein_kap: bool, ertraege: i64, sonstige: i64) -> [(&'static str, Value); 6] {
    [
        ("kein_kap", json!(kein_kap)),
        ("kap_kapitalertraege", json!(ertraege)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(sonstige)),
        ("kap_verlust_sonstige", json!(0)),
    ]
}

/// `_basis(...) + kap_events` aus `test_kap_deklaration_verschweigt_topf.py`.
fn topf_lauf(sonstige: i64) -> (Grund, Option<i64>, elster::Deklaration) {
    let mut paare = vec![
        ("bruttoarbeitslohn", json!(3_000_000)),
        ("veranlagung", json!("einzel")),
    ];
    paare.extend(stamm_meier());
    paare.extend(kap(false, 0, sonstige));
    let f = fall(Scheibe::Gesamt, &paare, &[]);
    let (grund, zahl) = ergebnis(&f);
    (grund, zahl, deklaration(&f))
}

/// `test_kap_deklaration_verschweigt_topf.py`. Kontrolle wie
/// `test_guard_bleibt_still_bei_reiner_topf_angabe`: `kapital_semantik_offen` schweigt zu Recht.
#[test]
#[ignore = "elster::deklariere (elster/src/deklaration.rs) schreibt E1900701 1:1 aus dem rohen kap_kapitalertraege; der Topf kap_gewinn_sonstige hat kein Kz. Bei reiner Topf-Angabe versteuert der Ring 1.750 EUR, die Anlage KAP behauptet E1900701 = 0. Python: test_kap_deklaration_verschweigt_topf.py::test_topf_wird_versteuert_aber_als_null_deklariert. Vault: audits/p20-kap-toepfe-xor-aggregat-steuer-ignoriert-deklaration.md. Rot sehen: --ignored"]
fn kap_topf_versteuert_aber_als_null_deklariert() {
    let (grund, basis, basis_d) = topf_lauf(0);
    assert_eq!(grund, Grund::Bestaetigt, "KONTROLLE: Basis ohne Kapital");
    let (grund, topf, topf_d) = topf_lauf(175_000);
    assert_eq!(
        grund,
        Grund::Bestaetigt,
        "KONTROLLE: Waechter feuert bei reiner Topf-Angabe"
    );
    let delta = topf.zip(basis).map(|(t, b)| t - b);
    assert!(
        delta.is_some_and(|n| n > 0),
        "KONTROLLE: 1.750 EUR im Topf erhoehen die Steuer nicht ({basis:?} -> {topf:?})"
    );
    let (e_basis, e_topf) = (kz(&basis_d, "E1900701"), kz(&topf_d, "E1900701"));
    assert!(
        e_topf != Some(0.0) || e_basis == e_topf,
        "DEFEKT: E1900701 = {e_topf:?} (Basis {e_basis:?}), waehrend die Steuer um {delta:?} Cent steigt"
    );
}

/// `_GRUND + _kap(...)` aus `test_p20_gewinn_sonstige_e1900701_widerspruch.py`: `grund`,
/// `zahl_cent`, Deklaration und das XML, das `einreichen` baut.
fn p20_lauf(
    kein_kap: bool,
    ertraege: i64,
    sonstige: i64,
) -> (Grund, Option<i64>, elster::Deklaration, String) {
    let mut paare = leck_basis();
    paare.extend([
        ("veranlagung", json!("einzel")),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ]);
    paare.extend(kap(kein_kap, ertraege, sonstige));
    let f = fall(Scheibe::Gesamt, &paare, &[]);
    let (grund, zahl) = ergebnis(&f);
    let d = deklaration(&f);
    let x = xml(&f, &d);
    (grund, zahl, d, x)
}

/// `test_p20_gewinn_sonstige_e1900701_widerspruch.py`: derselbe Bruch im XML. `einreichen` baut
/// es nur ohne Sperre und mit `eingaben_konsistent`; beides ist Kontrolle. Kontrolle wie
/// `test_gruenkontrolle_aggregat_konsistent`: das Aggregat allein steht als 1750 in Dict und XML.
#[test]
#[ignore = "elster::deklariere (elster/src/deklaration.rs) schreibt E1900701 1:1 aus dem rohen kap_kapitalertraege: bei reiner Topf-Angabe steht im XML aus einreichen ausdruecklich <E1900701>0</E1900701>, waehrend der Ring den Topf versteuert. Python: test_p20_gewinn_sonstige_e1900701_widerspruch.py::test_topf_only_steuer_und_deklaration_widersprechen_sich. Vault: audits/p20-kap-toepfe-xor-aggregat-steuer-ignoriert-deklaration.md. Rot sehen: --ignored"]
fn kap_topf_steht_im_xml_als_null() {
    let (grund, zero, _, _) = p20_lauf(true, 0, 0);
    assert_eq!(grund, Grund::Bestaetigt, "KONTROLLE: Baseline ohne Kapital");
    let (grund, green, d, x) = p20_lauf(false, 175_000, 0);
    assert!(
        grund == Grund::Bestaetigt && green.zip(zero).is_some_and(|(g, z)| g > z),
        "KONTROLLE: Aggregat 1.750 EUR erhoeht die Steuer nicht ({zero:?} -> {green:?}, {grund:?})"
    );
    assert!(
        d.eingaben_konsistent()
            && kz(&d, "E1900701") == Some(1750.0)
            && x.contains("<E1900701>1750</E1900701>"),
        "KONTROLLE: Aggregat steht nicht als E1900701 = 1750 in Dict und XML: {:?}",
        d.deklaration.get("E1900701")
    );
    let (grund, red, d, x) = p20_lauf(false, 0, 175_000);
    let delta = red.zip(zero).map(|(r, z)| r - z);
    assert!(
        grund == Grund::Bestaetigt && delta.is_some_and(|n| n > 0),
        "KONTROLLE: Topf-Lauf sperrt oder erhoeht die Steuer nicht ({grund:?}, delta={delta:?})"
    );
    assert!(
        d.eingaben_konsistent(),
        "KONTROLLE: einreichen braeche mit deklaration_unvollstaendig ab"
    );
    assert!(
        !(kz(&d, "E1900701") == Some(0.0) && x.contains("<E1900701>0</E1900701>")),
        "DEFEKT: Steuer +{delta:?} Cent aus kap_gewinn_sonstige, aber E1900701 steht in Dict und XML auf 0"
    );
}

// ---------------------------------------------------------------- Scheiben-Disjunktheit

/// Person-A-Basis-Kegel aus `test_kein_sonstige_partner_korrektur_ueberlebt_widerspruch.py`
/// (`_BASIS_ZUSAMMEN`, 25 Paare) — dort bereits gruen verifiziert fuer dieselbe Scheibe.
fn basis_zusammen() -> Vec<(&'static str, Value)> {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(2_000_000)),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(0)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
        ("veranlagung", json!("zusammen")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("basis_kv", json!(0)),
        ("basis_pv", json!(0)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(false)),
    ]
}

/// Die vier RENTNER_22_PARTNER-Kernfelder (Person B) aus `_RENTE_B_VOLL`.
fn rente_b_voll() -> [(&'static str, Value); 4] {
    [
        ("rentner_renten_art_partner", json!("gesetzliche_rente")),
        ("rentner_jahresrente_partner", json!(1_800_000)),
        ("rentner_renten_beginn_jahr_partner", json!(2015)),
        ("rentner_alter_bei_rentenbeginn_partner", json!(0)),
    ]
}

/// Die Scheiben-Zugehoerigkeit beider Felder — die Messung, an der beide Python-`xfail`s
/// scheitern (dort HTTP 400 „`feld_id` nicht in dieser Scheibe", `api.py:509`).
///
/// Der Bau ist `Cfg::fuer(scheibe).felder()` — dieselbe Liste, die `api.py::event()` gegen die
/// Feld-Id prueft und die `_scheibe_bindung` als `scheibe_felder` an `flag_widersprueche` gibt.
fn scheiben_zuschnitt() -> (bool, bool, bool) {
    let gesamt = scheiben_felder(Scheibe::Gesamt);
    let rentner = scheiben_felder(Scheibe::RentnerGesamt);
    (
        gesamt.contains("kein_sonstige_partner"),
        rentner.contains("rentner_jahresrente_partner"),
        rentner.contains("kein_sonstige_partner"),
    )
}

/// `test_kein_sonstige_partner_korrektur_ueberlebt_widerspruch.py`: die Korrektur-Kette laeuft auf
/// `rentner_gesamt`, das Flag lebt auf `gesamt`. Ein Fall ist an EINE Scheibe gebunden, Flag und
/// Ziel koennen also nie koexistieren — der `FLAG_NEGIERT`-Eintrag ist nicht falsch, sondern
/// unfeuerbar.
///
/// Der Waechter selbst ist in Rust richtig (KONTROLLE 1: erzwingt man die Koexistenz im Snapshot,
/// sperrt er). Der Defekt ist die Scheiben-Schranke davor: der Nutzer kann sie nicht herstellen.
#[test]
#[ignore = "kein_sonstige_partner liegt nur in SCHEIBEN_GESAMT_FELDER, rentner_jahresrente_partner nur in SCHEIBEN_RENTNER_GESAMT_FELDER (scheiben_tabellen.rs) — der erste Event-Post auf rentner_gesamt scheitert mit 400 'feld_id nicht in dieser Scheibe'. Python: test_kein_sonstige_partner_korrektur_ueberlebt_widerspruch.py::test_rentner_partner_korrektur_widerspruch_wird_gesperrt. Vault: keine eigene Notiz zur Disjunktheit; Bauart in tickets/zwei-vollstaendigkeitsbegriffe-einer-davon-gelesen.md. Rot sehen: --ignored"]
fn kein_sonstige_partner_korrektur_wird_gesperrt() {
    // KONTROLLE 1: der Waechter ist richtig. Die Koexistenz im Snapshot erzwungen (der Testbauer
    // kennt die Scheiben-Schranke nicht), sperrt er wie bei Person A.
    let mut paare = basis_zusammen();
    paare.push(("kein_sonstige_partner", json!(true)));
    paare.extend(rente_b_voll());
    let f = fall(Scheibe::RentnerGesamt, &paare, &[]);
    let (grund, zahl) = ergebnis(&f);
    assert!(
        grund == Grund::Sperre(Sperrgrund::FlagKonsistenzOffen) && zahl.is_none(),
        "KONTROLLE: kein_sonstige_partner=true neben rentner_jahresrente_partner sperrt nicht \
         ({grund:?}, {zahl:?}) — dann misst der Defekt unten den falschen Waechter"
    );
    // KONTROLLE 2: die Mechanik, die den Eintrag stilllegt (`FlagStand::NichtFragbar`).
    let ohne = fall(Scheibe::RentnerGesamt, &basis_zusammen(), &[]);
    assert_eq!(
        konsistenz::flag_stand(
            &ohne.felder,
            "kein_sonstige_partner",
            Some(&scheiben_felder(Scheibe::RentnerGesamt))
        ),
        FlagStand::NichtFragbar,
        "KONTROLLE: das auf dieser Scheibe fehlende Flag gilt nicht als unfragbar"
    );
    // DEFEKT: die Koexistenz ist ueber den normalen Weg nicht herstellbar.
    let (_, ziel_auf_rentner, flag_auf_rentner) = scheiben_zuschnitt();
    assert!(
        ziel_auf_rentner && flag_auf_rentner,
        "DEFEKT: rentner_jahresrente_partner auf rentner_gesamt={ziel_auf_rentner}, \
         kein_sonstige_partner dort={flag_auf_rentner} — die Korrektur 'mein Partner hat doch keine \
         Rente' ist auf dieser Scheibe nicht postbar, der FLAG_NEGIERT-Eintrag bleibt unfeuerbar"
    );
}

/// `test_rentner_partner_korrektur_richtung_offen_zu_gesetzt_bleibt_korrekt`: die Gegenrichtung,
/// gemessen an der Bindung statt an der Scheiben-Liste.
///
/// Die fuenf Partner-Rentenfelder tragen `feld_bedingung: {feld: kein_sonstige_partner, wert:
/// false}` (`bindung_rentner.yaml:123-198`). Diese Bedingung kann auf `rentner_gesamt` nie
/// BESTAETIGT abweichen, weil das Flag dort nicht postbar ist — die Abschaltung ist also tot.
#[test]
#[ignore = "selbe Ursache, andere Messstelle: die feld_bedingung der fuenf Partner-Rentenfelder verweist auf kein_sonstige_partner, das auf rentner_gesamt nicht liegt — die Bedingung ist dort nie erfuellbar. Python: test_kein_sonstige_partner_korrektur_ueberlebt_widerspruch.py::test_rentner_partner_korrektur_richtung_offen_zu_gesetzt_bleibt_korrekt. Vault: keine eigene Notiz. Rot sehen: --ignored"]
fn kein_sonstige_partner_korrektur_richtung_bleibt_korrekt() {
    let verweise: Vec<&str> = rente_b_voll()
        .iter()
        .map(|(f, _)| *f)
        .filter(|f| {
            index()[*f]
                .feld_bedingung
                .as_ref()
                .is_some_and(|b| b.feld == "kein_sonstige_partner")
        })
        .collect();
    assert_eq!(
        verweise.len(),
        rente_b_voll().len(),
        "KONTROLLE: nicht alle vier Partner-Rentenfelder haengen an kein_sonstige_partner: {verweise:?}"
    );
    let (_, _, flag_auf_rentner) = scheiben_zuschnitt();
    assert!(
        flag_auf_rentner,
        "DEFEKT: {verweise:?} haengen an kein_sonstige_partner, das Flag liegt aber nicht in \
         SCHEIBEN_RENTNER_GESAMT_FELDER — die Bedingung kann nie greifen, die Richtung ist nicht \
         korrekt, sondern unerreichbar"
    );
}

// ---------------------------------------------------------------- Rentner-Kegel und Pflichtluecken

/// `_VOLL` aus `test_pflichtfelder_luecken_ohne_leser.py` (17 Paare): alle 12 PFLICHTFELDER plus
/// die Stammdaten des Absender-Blocks.
fn pflicht_voll() -> Vec<(&'static str, Value)> {
    vec![
        ("stammdaten_nachname", json!("Maier")),
        ("stammdaten_vorname", json!("Hans")),
        ("stammdaten_geburtsdatum", json!("05.05.1955")),
        ("stammdaten_strasse", json!("Musterstr.")),
        ("stammdaten_hausnummer", json!("55")),
        ("stammdaten_plz", json!("55555")),
        ("stammdaten_wohnort", json!("Musterort")),
        ("stammdaten_steuernummer", json!("9181081508155")),
        ("kist_konfession", json!("keine")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("steuerklasse", json!("1")),
        ("p36_lohnsteuer", json!(1_200_000)),
        ("vor_an_anteil_rv", json!(4_200_000)),
        ("vor_ag_anteil_rv", json!(1_200_000)),
    ]
}

/// `test_rentner_gesamt_meldet_keine_felder_die_sein_kegel_nie_fragt`: eine Luecke, die die
/// laufende Scheibe nie fragt, ist keine Hilfe, sondern eine Sackgasse.
///
/// Gemessen wird die Invariante, nicht „die Meldung soll weg": jede gemeldete Pflichtluecke muss
/// ein Feld der Scheibe sein. Ein Kegel-Fix macht den Test gruen, ein Filter im Melder nicht.
#[test]
#[ignore = "der Kegel von rentner_gesamt (SCHEIBEN_RENTNER_GESAMT_FELDER) kennt aus der Gruppe 'alle_oder_keins' nur p36_lohnsteuer — nicht bruttoarbeitslohn und nicht steuerklasse. ERiC verlangt sie trotzdem (rc=610001002, gemessen 2026-10-01). Python: test_pflichtfelder_luecken_ohne_leser.py::test_rentner_gesamt_meldet_keine_felder_die_sein_kegel_nie_fragt. Vault: tickets/rentner-scheibe-fragt-lohnsteuer-ohne-die-zwei-pflichtfelder.md. Rot sehen: --ignored"]
fn rentner_gesamt_meldet_keine_felder_die_sein_kegel_nie_fragt() {
    // Der „Kegel" des Python-Tests ist `SCHEIBEN["rentner_gesamt"]["felder"]` (die Fragen der
    // Scheibe), nicht `cfg["kegel"]` (die Pflicht-Spannen-Achsen, `Cfg::kegel`).
    let kegel: HashSet<String> = Cfg::fuer(Scheibe::RentnerGesamt)
        .felder(|d| panic!("KONTROLLE: Feldliste aus {d}"))
        .unwrap()
        .into_iter()
        .collect();
    assert!(
        kegel.contains("p36_lohnsteuer") && !kegel.contains("bruttoarbeitslohn"),
        "KONTROLLE: der Kegel von rentner_gesamt traegt p36_lohnsteuer={} und bruttoarbeitslohn={}",
        kegel.contains("p36_lohnsteuer"),
        kegel.contains("bruttoarbeitslohn")
    );
    let paare: Vec<(&'static str, Value)> = pflicht_voll()
        .into_iter()
        .filter(|(f, _)| !matches!(*f, "bruttoarbeitslohn" | "steuerklasse"))
        .collect();
    let f = fall(Scheibe::RentnerGesamt, &paare, &[]);
    let d = deklaration(&f);
    let luecken: Vec<&str> = d
        .pflichtfelder_luecken()
        .iter()
        .map(|e| e.feld_id.as_str())
        .collect();
    assert!(
        !luecken.contains(&"p36_lohnsteuer") && !luecken.is_empty(),
        "KONTROLLE: p36_lohnsteuer ist beantwortet und liegt im Kegel; gemeldet wurde {luecken:?}"
    );
    let unerreichbar: Vec<&str> = luecken
        .iter()
        .copied()
        .filter(|f| !kegel.contains(*f))
        .collect();
    assert!(
        unerreichbar.is_empty(),
        "DEFEKT: als fehlend gemeldet, obwohl auf rentner_gesamt nicht beantwortbar: {unerreichbar:?} \
         — gemeldet {luecken:?}"
    );
}
