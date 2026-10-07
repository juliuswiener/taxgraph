//! Die Scheibenlisten in `scheiben_tabellen.rs` (Felder und Kegel je Scheibe) sind seit Weg B leicht
//! (Entscheidung 2026-10-05) **von Hand gepflegte Rust-Quelle**, nicht mehr aus Python erzeugt.
//! Vorher hielt nur der Parity-Test `konstanten_gleich` (`PARITY=1`, nie in der CI) sie gegen
//! `SCHEIBEN` in Python. Diese Datei haelt die Eigenschaften fest, die ohne Python noch gelten
//! muessen:
//!
//! - Jedes Feld einer Scheibe steht in der Registry (`rust/bindung/daten`).
//!   Ein Feld, das dort fehlt, laesst `scheibe_bindung` die ganze Scheibe mit 500 abweisen.
//! - Der Kegel (die Felder, die ueber Freigabe oder Sperre des Scheibenbetrags entscheiden) liegt in
//!   den Feldern, ist ohne Dopplung und fragbar. Fehlt ein Kegel-Feld oder steht eines doppelt, zaehlt
//!   `feste_zahl` ein Pflichtfeld nicht oder zweimal -- ohne Fehlermeldung, nur im Betrag.
//! - Die Laengen stehen fest. Eine Aenderung ist gewollt, dann aendert sie auch die Zahl hier.
//! - Erreichbarkeit (Stufe 3 von B voll, W4): Jedes fragbare Feld steht in einer Scheibe, die der Nutzer
//!   waehlen kann (`gesamt`, `rentner_gesamt`); sonst wird es nie gefragt (tote Bindung). Jedes
//!   Screening-Flag steht in `gesamt`. Jedes Betragsfeld (`cent`) dort ohne Kz steht in einer benannten
//!   Liste; ein neues faellt auf, und ein Eintrag, dessen Feld inzwischen ein Kz traegt, auch.
//!
//! Mutationsprobe (Instruktor, 2026-10-05): in `SCHEIBEN_GESAMT_KEGEL` `agb_notwendig_angemessen`
//! durch ein Duplikat von `agb_zwangslaeufig` ersetzen -- vorher in keinem Rust-Test rot, jetzt
//! `kegel_ist_dopplungsfrei_und_liegt_in_den_feldern`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

use bescheid::deklaration::Cfg;
use bindung::{Bindung, Registry};
use domain::{Feldtyp, Scheibe};

/// Die vier Scheiben, die ihre Feldliste selbst tragen. `NVorGwg` liest sie aus ihrer YAML.
const MIT_LISTE: [Scheibe; 4] = [
    Scheibe::Ep,
    Scheibe::AnGesamt,
    Scheibe::Gesamt,
    Scheibe::RentnerGesamt,
];

/// `(Scheibe, Felder, Kegel)`. Die Zahlen sind der Stand vom 2026-10-06 (C2: die beiden Felder
/// `kind_anderer_elternteil_tod_am` und `kind_anderer_elternteil_ausland_zeitraum` neu in `gesamt`
/// und `rentner_gesamt`, 352 -> 354 und 250 -> 252; B Option 1 am 2026-10-06: die vier Partner-Felder zu § 34 Abs. 3
/// `antrag_ermaessigter_satz_partner`, `dauernd_berufsunfaehig_partner`, `ermaessigung_einmal_genutzt_partner` und
/// `p34_abs3_antragsbetrag_partner` in beiden, 354 -> 358 und 252 -> 256; danach am 2026-10-06 das doppelte
/// `geburtsjahr` in `rentner_gesamt` einmal, 256 -> 255; danach am 2026-10-06 `bruttoarbeitslohn`, `steuerklasse` und die
/// fuenf `versorgung_*`-Felder neu in `rentner_gesamt`, 255 -> 262, der Kegel bleibt bei 28; danach am 2026-10-07
/// `kind_schulgeld_aufteilung_prozent` in beiden (Abweichung Nr. 26), 358 -> 359 und 262 -> 263; danach am 2026-10-07 `gwg_ohne_vorsteuerabzug` in beiden (Abweichung Nr. 27),
/// 359 -> 360 und 263 -> 264; danach am 2026-10-07 `ep_unfallkosten` in `an_gesamt` und `gesamt` (Abweichung Nr. 28),
/// 84 -> 85 und 360 -> 361, die Kegel bleiben bei 33 und 35, `rentner_gesamt` bei 264; danach am 2026-10-07
/// `parteispenden_betrag` in `gesamt` und `rentner_gesamt` (Abweichung Nr. 31), 361 -> 362 und 264 -> 265, die Kegel bleiben;
/// danach am 2026-10-07 `alter_55_vor_verkauf` und `alter_55_vor_verkauf_partner` in beiden (Abweichung Nr. 32), 362 -> 364 und 265 -> 267, die Kegel bleiben;
/// danach am 2026-10-07 `bruttoarbeitslohn_partner`, `steuerklasse_partner` und die fuenf `versorgung_*_partner`-Felder in
/// `rentner_gesamt` (Abweichung Nr. 33), 267 -> 274, der Kegel bleibt bei 28, `gesamt` bei 364;
/// vorher 2026-10-05, `9c07d98e`).
const LAENGEN: [(Scheibe, usize, usize); 4] = [
    (Scheibe::Ep, 6, 4),
    (Scheibe::AnGesamt, 85, 33),
    (Scheibe::Gesamt, 364, 35),
    (Scheibe::RentnerGesamt, 274, 28),
];

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel).expect("Registry (Python und Rust) laedt")
    })
}

fn bindungen() -> BTreeMap<&'static str, &'static Bindung> {
    registry()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .map(|b| (b.feld_id.as_str(), b))
        .collect()
}

fn felder(s: Scheibe) -> &'static [&'static str] {
    Cfg::fuer(s)
        .felder_roh()
        .unwrap_or_else(|| panic!("{s:?} traegt keine eigene Feldliste"))
}

fn kegel(s: Scheibe) -> &'static [&'static str] {
    Cfg::fuer(s)
        .kegel_roh()
        .unwrap_or_else(|| panic!("{s:?} traegt keinen eigenen Kegel"))
}

/// Felder, die in `liste` mehr als einmal stehen, sortiert.
fn doppelte(liste: &[&'static str]) -> Vec<&'static str> {
    let mut gesehen = BTreeSet::new();
    let mut doppelt = BTreeSet::new();
    for f in liste {
        if !gesehen.insert(*f) {
            doppelt.insert(*f);
        }
    }
    doppelt.into_iter().collect()
}

#[test]
fn jedes_feld_jeder_scheibe_steht_in_der_registry() {
    let alle = bindungen();
    for s in MIT_LISTE {
        let fehlt: Vec<&str> = felder(s)
            .iter()
            .copied()
            .filter(|f| !alle.contains_key(f))
            .collect();
        assert!(
            fehlt.is_empty(),
            "{s:?}: Felder ohne Bindung in rust/bindung/daten: {fehlt:?}"
        );
    }
}

/// `n_vor_gwg` hat keine eigene Liste: ihre Felder stehen in einer YAML, ihre Teil-Ringe in der
/// Tabelle. Beides muss in der Registry stehen.
#[test]
fn n_vor_gwg_datei_und_teil_ringe_stehen_in_der_registry() {
    let cfg = Cfg::fuer(Scheibe::NVorGwg);
    assert!(cfg.felder_roh().is_none() && cfg.kegel_roh().is_none());
    let datei = cfg.felder_datei().expect("n_vor_gwg nennt ihre Datei");
    let (_, inhalt) = registry()
        .dateien
        .iter()
        .find(|(p, _)| p.file_name().is_some_and(|n| n == datei))
        .unwrap_or_else(|| panic!("{datei} fehlt in der Registry"));
    assert!(!inhalt.bindungen.is_empty(), "{datei} bindet kein Feld");
    let alle = bindungen();
    assert!(!cfg.teil_ringe().is_empty());
    for (familie, _, felder) in cfg.teil_ringe() {
        let fehlt: Vec<&&str> = felder.iter().filter(|f| !alle.contains_key(**f)).collect();
        assert!(
            fehlt.is_empty(),
            "Teil-Ring {familie}: Felder ohne Bindung: {fehlt:?}"
        );
    }
}

/// Jede Scheibe führt jedes Feld einmal. Ein Feld doppelt in der Liste bricht die Vorjahr-Übernahme
/// am zweiten Eintrag mit 422 ab (`rust/api/tests/vorjahr_naht.rs`).
#[test]
fn felder_sind_dopplungsfrei() {
    for s in MIT_LISTE {
        assert_eq!(
            doppelte(felder(s)),
            Vec::<&str>::new(),
            "{s:?}: doppelte Felder"
        );
    }
}

#[test]
fn kegel_ist_dopplungsfrei_und_liegt_in_den_feldern() {
    for s in MIT_LISTE {
        let k = kegel(s);
        assert_eq!(
            doppelte(k),
            Vec::<&str>::new(),
            "{s:?}: doppelter Kegel-Eintrag"
        );
        let in_felder: BTreeSet<&str> = felder(s).iter().copied().collect();
        let draussen: Vec<&&str> = k.iter().filter(|f| !in_felder.contains(**f)).collect();
        assert!(
            draussen.is_empty(),
            "{s:?}: Kegel-Felder ausserhalb der Felder: {draussen:?}"
        );
    }
}

/// Ein Kegel-Feld, das der Nutzer nicht gefragt wird, kann nie bestaetigt werden: die Scheibe bliebe
/// fuer immer gesperrt. Heute sind alle 100 Kegel-Felder fragbar.
#[test]
fn jedes_kegel_feld_ist_fragbar() {
    let alle = bindungen();
    for s in MIT_LISTE {
        let nicht: Vec<&&str> = kegel(s)
            .iter()
            .filter(|f| alle.get(**f).is_none_or(|b| !b.askable))
            .collect();
        assert!(
            nicht.is_empty(),
            "{s:?}: Kegel-Felder, die nicht askable sind: {nicht:?}"
        );
    }
}

#[test]
fn laengen_aendern_sich_nur_mit_absicht() {
    for (s, n_felder, n_kegel) in LAENGEN {
        assert_eq!(felder(s).len(), n_felder, "{s:?}: Anzahl Felder");
        assert_eq!(kegel(s).len(), n_kegel, "{s:?}: Anzahl Kegel-Felder");
    }
}

/// Die Scheiben, die die Oberflaeche anbietet (`produkt/haut/static/index.html`). Ein Feld, das in keiner steht,
/// kann der Nutzer nicht setzen: ein POST endet mit "`feld_id` nicht in dieser Scheibe".
const NUTZERWAEHLBAR: [Scheibe; 2] = [Scheibe::Gesamt, Scheibe::RentnerGesamt];

fn erreichbar() -> BTreeSet<&'static str> {
    NUTZERWAEHLBAR
        .iter()
        .flat_map(|s| felder(*s).iter().copied())
        .collect()
}

/// Felder aus `gefragt`, die in `erreichbar` fehlen.
fn nicht_erreichbar<'a>(gefragt: &BTreeSet<&'a str>, erreichbar: &BTreeSet<&str>) -> Vec<&'a str> {
    gefragt
        .iter()
        .copied()
        .filter(|f| !erreichbar.contains(f))
        .collect()
}

/// Ein fragbares Feld ohne Scheibe war mehrfach die Fehlerursache: Bindung geschrieben, Auslese gebaut, Tests
/// gruen, und das Feld ueber die Oberflaeche nie setzbar, weil der Eintrag in der Scheibenliste fehlte.
/// Die Listen sind von Hand gepflegt (`scheiben_tabellen.rs`); dies ist der Zeuge, dass sie zur Registry passen.
#[test]
fn jedes_fragbare_feld_steht_in_einer_nutzerwaehlbaren_scheibe() {
    let fragbar: BTreeSet<&str> = bindungen()
        .into_iter()
        .filter(|(_, b)| b.askable)
        .map(|(id, _)| id)
        .collect();
    assert!(
        fragbar.len() >= 300,
        "nur {} fragbare Felder",
        fragbar.len()
    );
    let tot = nicht_erreichbar(&fragbar, &erreichbar());
    assert!(
        tot.is_empty(),
        "fragbar gebunden, aber in keiner nutzerwaehlbaren Scheibe (tote Bindung): {tot:?}; \
         entweder in die Scheibenliste in scheiben_tabellen.rs aufnehmen oder das Feld nicht fragbar machen"
    );
}

/// Screening-Flags ("Hast du ...?" mit dem Kreuz "kein ...") schalten ganze Regeln ab, aber nur, wenn der
/// Nutzer sie sieht. Ein Flag ausserhalb von `gesamt` wird nie gefragt und schaltet nie etwas ab.
#[test]
fn jedes_screening_flag_steht_in_der_scheibe_gesamt() {
    let flags: BTreeSet<&str> = bindungen()
        .into_iter()
        .filter(|(_, b)| b.screening == Some(true))
        .map(|(id, _)| id)
        .collect();
    assert!(flags.len() >= 15, "nur {} Screening-Flags", flags.len());
    let in_gesamt: BTreeSet<&str> = felder(Scheibe::Gesamt).iter().copied().collect();
    let fehlt: Vec<&&str> = flags.iter().filter(|f| !in_gesamt.contains(**f)).collect();
    assert!(
        fehlt.is_empty(),
        "Screening-Flags nicht in `gesamt`: {fehlt:?}"
    );
}

/// Betragsfelder (`cent`), die ein Nutzer ausfuellen kann und die kein Kz tragen. Der Lader verlangt fuer jedes
/// Feld ohne Kz einen `elster_kz_grund`; diese Liste ist die zweite Unterschrift: ein neues Betragsfeld ohne Kz
/// faellt hier auf, statt in der grossen Menge zu verschwinden. Warum das einzelne Feld kein Kz hat, steht in
/// seinem `elster_kz_grund` in `rust/bindung/daten`.
///
/// Herkunft: `BETRAGSFELDER_OHNE_KZ` aus `tests/test_bindungstabelle.py` (78 Namen), geschnitten mit den Daten vom
/// 2026-10-05 (`6fe9971b`): 63 Felder. 15 Namen der Python-Liste tragen inzwischen ein Kz (`berufsausbildung_aufwendungen`,
/// `dba_*`, `gewst_messbetrag`, `kist_*`, `p22_nr3_einkuenfte`, `p32b_*`, `p33a_unterhalt_*`, `p35c_*`, `p36_lohnsteuer`,
/// `realsplitting_*`); Python prueft die Gegenrichtung nicht und fuehrte sie weiter. Hier prueft sie
/// `die_liste_der_betragsfelder_ohne_kz_enthaelt_nur_noch_luecken`.
///
/// Gewaehlt ist eine Liste im Test und keine Marke in den Daten: von 63 Feldern tragen 22 `kz_status`, alle
/// `endgueltig`; fuer die uebrigen 41 muesste jemand je Feld entscheiden, ob "offen" oder "endgueltig" stimmt. Das
/// ist eine rechtliche Einordnung und gehoert nicht in einen Waechtertest. `kz_status` liest sonst nur der Lader.
// ponytail: von Hand gepflegte Liste; Upgrade waere `kz_status` an allen 63 Feldern, wenn die Einordnung entschieden ist.
const BETRAGSFELDER_OHNE_KZ: &[&str] = &[
    "afa_jahresbetrag",
    "am_anschaffungskosten",
    "basis_kv",
    "basis_kv_partner",
    "basis_pv",
    "basis_pv_partner",
    "behinderungsbedingte_aufwendungen",
    "behinderungsbedingte_aufwendungen_partner",
    "betriebseinnahmen",
    "bruttoarbeitslohn_partner",
    "einkuenfte_gewinn",
    "einkuenfte_gewinn_partner",
    // Seit 2026-10-07 (Abweichung Nr. 28): nicht in der Python-Liste. Kz offen, die Abgabe sperrt bei einem Betrag ueber 0.
    "ep_unfallkosten",
    "gewinnanteil",
    "gewinnanteil_partner",
    "gewst_messbetrag_partner",
    "gewst_zu_zahlen_partner",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige",
    "kap_gewinn_sonstige_partner",
    "kap_kapitalertraege_partner",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner",
    "kirchensteuer_arbeitgeber_partner",
    "p23_anschaffung_herstellungskosten",
    "p23_veraeusserungspreis",
    "p23_werbungskosten",
    "p33a_andere_einkuenfte_bezuege",
    "p34_abs3_antragsbetrag",
    "p34_abs3_antragsbetrag_partner",
    "p35c_massnahme_einzelbetrag",
    "p36_lohnsteuer_partner",
    "p36_vorauszahlungen",
    "pv_einnahmen",
    "rentner_jahresrente",
    "rentner_jahresrente_partner",
    "rentner_rentenfreibetrag",
    "rentner_rentenfreibetrag_partner",
    "rentner_veraeusserungsgewinn",
    "rentner_veraeusserungsgewinn_partner",
    "spenden_vermoegensstock",
    "uebernachtung_kosten_monat",
    "verguetung_darlehen",
    "verguetung_darlehen_partner",
    "verguetung_taetigkeit",
    "verguetung_taetigkeit_partner",
    "verguetung_ueberlassung",
    "verguetung_ueberlassung_partner",
    "verlustvortrag_bestand",
    "versorgung_bemessungsgrundlage",
    // Seit 2026-10-07 (Abweichung Nr. 33): nicht in der Python-Liste. Wie `versorgung_bemessungsgrundlage` und
    // `versorgung_jahresrente` von Person A: kein Kz, der Betrag steht mit Grund in `nicht_deklariert`.
    "versorgung_bemessungsgrundlage_partner",
    "versorgung_jahresrente",
    "versorgung_jahresrente_partner",
    "vor_ag_anteil_rv_partner",
    "vor_an_anteil_rv_partner",
    "vor_rv_ausserhalb_lstb_partner",
    "vorsorge_arbeitslosenversicherung_partner",
    "vorsorge_erwerbsunfaehigkeit_partner",
    "vorsorge_rv_alt_mit_ueberschuss_partner",
    "vorsorge_rv_alt_ohne_ueberschuss_partner",
    "vorsorge_unfall_haftpflicht_partner",
    "vpf_mahlzeiten_gezahltes_entgelt",
    "vpf_steuerfreie_erstattung_betrag",
    "vv_erhaltungsaufwand",
    "vv_gebaeude_afa",
    "vv_schuldzinsen",
    "vv_sonstige_wk",
];

/// `cent`-Felder in den nutzerwaehlbaren Scheiben ohne Kz.
fn betragsfelder_ohne_kz() -> BTreeSet<&'static str> {
    let in_scheiben = erreichbar();
    bindungen()
        .into_iter()
        .filter(|(id, b)| {
            matches!(b.typ, Feldtyp::Cent) && b.elster_kz.is_none() && in_scheiben.contains(*id)
        })
        .map(|(id, _)| id)
        .collect()
}

#[test]
fn jedes_betragsfeld_ohne_kz_steht_in_der_benannten_liste() {
    let benannt: BTreeSet<&str> = BETRAGSFELDER_OHNE_KZ.iter().copied().collect();
    let neu: Vec<&str> = betragsfelder_ohne_kz()
        .difference(&benannt)
        .copied()
        .collect();
    assert!(
        neu.is_empty(),
        "Betragsfeld ohne Kz, das nicht in BETRAGSFELDER_OHNE_KZ steht: {neu:?}; entweder ein Kz binden \
         oder das Feld hier mit Blick auf seinen elster_kz_grund eintragen"
    );
}

/// Die zweite Richtung, und die leichter zu uebersehende: Ein Feld bekommt sein Kz, der Eintrag bleibt stehen.
/// Dann behauptet die Liste weiter "kein Weg in die Erklaerung", und eine Liste, die mehr verdeckt als noetig,
/// deckt irgendwann ein echtes Loch mit zu.
#[test]
fn die_liste_der_betragsfelder_ohne_kz_enthaelt_nur_noch_luecken() {
    let echt = betragsfelder_ohne_kz();
    assert!(echt.len() >= 50, "nur {} Betragsfelder ohne Kz", echt.len());
    let veraltet: Vec<&&str> = BETRAGSFELDER_OHNE_KZ
        .iter()
        .filter(|f| !echt.contains(**f))
        .collect();
    assert!(
        veraltet.is_empty(),
        "in BETRAGSFELDER_OHNE_KZ, aber kein Betragsfeld ohne Kz mehr (Kz gebunden oder Feld entfernt): {veraltet:?}"
    );
    let doppelt =
        BETRAGSFELDER_OHNE_KZ.len() - BETRAGSFELDER_OHNE_KZ.iter().collect::<BTreeSet<_>>().len();
    assert_eq!(
        doppelt, 0,
        "BETRAGSFELDER_OHNE_KZ fuehrt einen Namen doppelt"
    );
}
