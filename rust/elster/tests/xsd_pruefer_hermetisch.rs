//! Der Schema-Pruefer (`elster::pruefe_bindung_mit`, Gegenstueck zu `xsd_verify.pruefe_bindung`) gegen SELBST GEBAUTE Schemas, ohne
//! amtliches XSD und ohne ERiC: der Standardlauf und die CI sehen ihn. Gegenstueck zu `tests/test_xsd_verify.py` (die Tests ohne
//! `requires_real_schema`) und zu `tests/test_kz_ausschluss_vollstaendig.py` (E60-Kz laufen gegen E77).
//!
//! **Worum es geht.** `vz_gueltigkeit_schema.rs` ruft den Pruefer nur gegen das echte Schema und liest dort `status`; ob der Pruefer
//! ein Kz mit zwei Pfaden als `Mehrdeutig` meldet, ob der Exit-Code bei einem Abbruch der Tiefe anschlaegt und ob ein Jahr ohne Schema
//! gemeldet statt uebersprungen wird, sah kein Test der CI (Gegenversuche Nr. 69: `X4`, `X5`, `X6`, `X9`, `X10` blieben gruen).
//!
//! Die Schemas sind klein und stehen unten im Text; das Wurzelelement heisst `E10` bzw. `E77`, weil der Pruefer genau dort startet.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use bindung::Bindung;
use elster::{
    ernte_est_mapping_kz, pruefe_bindung_mit, xsd_walk, KzPruefling, PruefStatus, XsdFehler, MAX_DEPTH,
};

/// Das Jahr, das die selbst gebauten Schemas tragen (ein Jahr, das es als Steuerjahr nicht gibt).
const JAHR: i64 = 2099;

/// Ein Wegwerf-Verzeichnis mit Schema-Dateien; weg, wenn der Test endet.
struct Probe(PathBuf);

impl Probe {
    fn neu(name: &str) -> Self {
        let d = std::env::temp_dir().join(format!("taxgraph-xsdpruefer-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        Self(d)
    }

    fn schreibe(&self, datei: &str, inhalt: &str) -> PathBuf {
        let p = self.0.join(datei);
        std::fs::write(&p, inhalt).unwrap();
        p
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn kopf(wurzel: &str, typ: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\n  <xs:element name=\"{wurzel}\" type=\"{typ}\"/>\n"
    )
}

/// §1-Fixture: ein `complexType`, der unter zwei Namen vorkommt (Absender, Empfaenger) und das Kz `E9999001` fuehrt.
fn geteilt_xsd() -> String {
    kopf("E10", "EnvelopeType")
        + r#"  <xs:complexType name="EnvelopeType">
    <xs:sequence>
      <xs:element name="Absender" type="AdresseType"/>
      <xs:element name="Empfaenger" type="AdresseType"/>
    </xs:sequence>
  </xs:complexType>
  <xs:complexType name="AdresseType">
    <xs:sequence><xs:element name="E9999001" type="xs:string"/></xs:sequence>
  </xs:complexType>
</xs:schema>"#
}

/// Ein Kz (`E1111111`) in genau einem Block.
fn einfach_xsd() -> String {
    kopf("E10", "RootType")
        + r#"  <xs:complexType name="RootType">
    <xs:sequence><xs:element name="Block" type="BlockType"/></xs:sequence>
  </xs:complexType>
  <xs:complexType name="BlockType">
    <xs:sequence><xs:element name="E1111111" type="xs:string"/></xs:sequence>
  </xs:complexType>
</xs:schema>"#
}

/// `E10 -> W1 -> W2 -> ... -> W<tiefe> -> Blatt`; `fruehe_kz` liegt in `W1`, also VOR dem Abbruch der Tiefe.
fn tiefe_kette_xsd(tiefe: usize, fruehe_kz: &str) -> String {
    let typen = (1..=tiefe)
        .map(|i| {
            let frueh = if i == 1 { format!("<xs:element name=\"{fruehe_kz}\" type=\"xs:string\"/>") } else { String::new() };
            let weiter = if i < tiefe {
                format!("<xs:element name=\"W{}\" type=\"W{}Type\"/>", i + 1, i + 1)
            } else {
                "<xs:element name=\"Blatt\" type=\"xs:string\"/>".to_owned()
            };
            format!("<xs:complexType name=\"W{i}Type\"><xs:sequence>{frueh}{weiter}</xs:sequence></xs:complexType>")
        })
        .collect::<Vec<String>>()
        .concat();
    kopf("E10", "W1Type") + &typen + "</xs:schema>"
}

fn pruefling(feld: &str, kz: &str, jahre: &[i64]) -> KzPruefling {
    KzPruefling {
        feld_id: feld.to_owned(),
        elster_kz: kz.to_owned(),
        vz_gueltigkeit: jahre.to_vec(),
    }
}

/// Die Suche, die fuer `JAHR` das E10-Schema `pfad` liefert und fuer jedes andere Jahr nichts.
fn nur_e10(pfad: &Path) -> impl Fn(i64, &str) -> Option<PathBuf> + '_ {
    move |jahr, muster| (jahr == JAHR && muster == "E10-{jahr}.xsd").then(|| pfad.to_path_buf())
}

#[test]
fn geteilter_typ_ist_mehrdeutig_und_der_exit_code_schlaegt_an() {
    let probe = Probe::neu("geteilt");
    let pfad = probe.schreibe("geteilt.xsd", &geteilt_xsd());
    // Der Walker liefert zwei unterscheidbare Fundstellen (die Primitive) ...
    let (index, abbrueche) = xsd_walk(&pfad, "E10").unwrap();
    assert_eq!(abbrueche, 0);
    let pfade: Vec<String> = index.iter().find(|(k, _)| k == "E9999001").unwrap().1.iter().map(|p| p.join("/")).collect();
    assert_eq!(pfade, ["E10/Absender/E9999001", "E10/Empfaenger/E9999001"]);
    // ... und der Pruefer (der Konsument) klassifiziert das als Mehrdeutig und schlaegt an.
    let b = pruefe_bindung_mit(&[pruefling("feld_x", "E9999001", &[JAHR])], nur_e10(&pfad)).unwrap();
    assert_eq!(b.felder["feld_x"].status, PruefStatus::Mehrdeutig);
    assert_eq!(b.felder["feld_x"].jahre[&JAHR].status, PruefStatus::Mehrdeutig);
    assert_eq!(b.felder["feld_x"].jahre[&JAHR].pfade.len(), 2);
    assert_eq!(b.exit_code, 1);
}

#[test]
fn eindeutiger_pfad_ist_ok_und_der_exit_code_bleibt_null() {
    let probe = Probe::neu("einfach");
    let pfad = probe.schreibe("einfach.xsd", &einfach_xsd());
    let b = pruefe_bindung_mit(&[pruefling("feld_ok", "E1111111", &[JAHR])], nur_e10(&pfad)).unwrap();
    assert_eq!(b.felder["feld_ok"].status, PruefStatus::Ok);
    assert_eq!(b.felder["feld_ok"].jahre[&JAHR].pfade, ["E10/Block/E1111111"]);
    assert_eq!(b.max_depth_hits_gesamt, 0);
    assert_eq!(b.exit_code, 0);
}

#[test]
fn ein_kz_das_nicht_im_schema_steht_ist_nicht_gefunden() {
    let probe = Probe::neu("fehlt");
    let pfad = probe.schreibe("einfach.xsd", &einfach_xsd());
    let b = pruefe_bindung_mit(&[pruefling("feld_fehlt", "E9999999", &[JAHR])], nur_e10(&pfad)).unwrap();
    assert_eq!(b.felder["feld_fehlt"].status, PruefStatus::NichtGefunden);
    assert_eq!(b.felder["feld_fehlt"].jahre[&JAHR].pfade, Vec::<String>::new());
    assert_eq!(b.exit_code, 1);
}

#[test]
fn ein_feld_ohne_kz_wird_nie_nachgeschlagen() {
    let probe = Probe::neu("ohne-kz");
    let pfad = probe.schreibe("einfach.xsd", &einfach_xsd());
    let b = pruefe_bindung_mit(&[pruefling("feld_ohne_kz", "", &[JAHR])], nur_e10(&pfad)).unwrap();
    assert!(b.felder.is_empty(), "{:?}", b.felder);
    assert_eq!(b.exit_code, 0);
}

#[test]
fn abbrueche_der_tiefe_zaehlen_und_schlagen_auch_bei_einem_sonst_ok_feld_an() {
    let probe = Probe::neu("tief");
    // Feste Tiefen, nicht `MAX_DEPTH +- 20`: sonst folgte der Test jeder Aenderung der Grenze (Gegenversuch X7: 80 -> 8000).
    assert_eq!(MAX_DEPTH, 80, "Die Tiefen 100 und 60 unten setzen die Grenze 80 voraus");
    let pfad = probe.schreibe("tief.xsd", &tiefe_kette_xsd(100, "E2222222"));
    let b = pruefe_bindung_mit(&[pruefling("feld_frueh", "E2222222", &[JAHR])], nur_e10(&pfad)).unwrap();
    // Das geprueft Feld liegt VOR dem Abbruch und ist eindeutig OK ...
    assert_eq!(b.felder["feld_frueh"].status, PruefStatus::Ok);
    // ... trotzdem schlaegt der Pruefer an, weil hinter ihm ein Teilbaum fehlt.
    assert!(b.max_depth_hits_gesamt > 0, "{}", b.max_depth_hits_gesamt);
    assert_eq!(b.exit_code, 1);
    // Gegenprobe: dieselbe Kette, aber flach genug, dass nichts abbricht.
    let flach = probe.schreibe("flach.xsd", &tiefe_kette_xsd(60, "E2222222"));
    let b = pruefe_bindung_mit(&[pruefling("feld_frueh", "E2222222", &[JAHR])], nur_e10(&flach)).unwrap();
    assert_eq!((b.max_depth_hits_gesamt, b.exit_code), (0, 0));
    // Die Grenze selbst (Nachmessung Nr. 69, RX5: `>=` statt `>`): das Blatt einer Kette mit 80 Typen liegt noch in MAX_DEPTH, das einer
    // Kette mit 81 Typen dahinter.
    for (n, erwartet) in [(80, 0), (81, 1)] {
        let datei = probe.schreibe(&format!("grenze{n}.xsd"), &tiefe_kette_xsd(n, "E2222222"));
        let b = pruefe_bindung_mit(&[pruefling("feld_frueh", "E2222222", &[JAHR])], nur_e10(&datei)).unwrap();
        assert_eq!(b.max_depth_hits_gesamt, erwartet, "Kette mit {n} Typen");
    }
}

/// Die Suche, die fuer jedes Jahr in `schemas` dessen E10-Schema liefert und fuer jedes andere Jahr nichts.
fn je_jahr(schemas: &[(i64, PathBuf)]) -> impl Fn(i64, &str) -> Option<PathBuf> + '_ {
    move |jahr, muster| {
        schemas
            .iter()
            .find(|(j, _)| *j == jahr && muster == "E10-{jahr}.xsd")
            .map(|(_, p)| p.clone())
    }
}

/// Hat ein Feld mehrere Jahre, gilt das schlechteste: Mehrdeutig vor `NichtGefunden` vor Ok (Nachmessung Nr. 69, RX1 und RX2). Ohne diese
/// Reihenfolge meldet der Pruefer ein Kz, das in EINEM Steuerjahr fehlt, als OK, solange es im anderen steht.
#[test]
fn die_jahre_eines_felds_rollen_nach_dem_schlechtesten_auf() {
    let probe = Probe::neu("rollup");
    // E9999001 steht im ersten Schema zweimal (Mehrdeutig), im zweiten einmal (Ok), im dritten gar nicht (NichtGefunden).
    let doppelt = probe.schreibe("doppelt.xsd", &geteilt_xsd());
    let einmal = probe.schreibe("einmal.xsd", &einfach_xsd().replace("E1111111", "E9999001"));
    let fehlt = probe.schreibe("fehlt.xsd", &einfach_xsd());
    let rollup_aus = |schemas: [(i64, &PathBuf); 2]| {
        let schemas: Vec<(i64, PathBuf)> = schemas.iter().map(|(j, p)| (*j, (*p).clone())).collect();
        let jahre: Vec<i64> = schemas.iter().map(|(j, _)| *j).collect();
        let b = pruefe_bindung_mit(&[pruefling("feld_x", "E9999001", &jahre)], je_jahr(&schemas)).unwrap();
        (b.felder["feld_x"].status, b.exit_code)
    };
    assert_eq!(rollup_aus([(2098, &einmal), (2099, &einmal)]), (PruefStatus::Ok, 0));
    assert_eq!(rollup_aus([(2098, &einmal), (2099, &fehlt)]), (PruefStatus::NichtGefunden, 1));
    assert_eq!(rollup_aus([(2098, &fehlt), (2099, &einmal)]), (PruefStatus::NichtGefunden, 1));
    assert_eq!(rollup_aus([(2098, &einmal), (2099, &doppelt)]), (PruefStatus::Mehrdeutig, 1));
    assert_eq!(rollup_aus([(2098, &fehlt), (2099, &doppelt)]), (PruefStatus::Mehrdeutig, 1));
}

#[test]
fn ein_jahr_ohne_schema_wird_gemeldet_nicht_uebersprungen() {
    let probe = Probe::neu("unverfuegbar");
    let pfad = probe.schreibe("einfach.xsd", &einfach_xsd());
    // 2099 verifiziert OK, 2026 hat kein Schema: der Feld-Rollup bleibt OK, der Exit-Code 0, das Jahr steht im Bericht.
    let b = pruefe_bindung_mit(&[pruefling("feld_mixed", "E1111111", &[JAHR, 2026])], nur_e10(&pfad)).unwrap();
    assert_eq!(b.felder["feld_mixed"].jahre[&2026].status, PruefStatus::SchemaUnverfuegbar);
    assert_eq!(b.unverfuegbare_jahre, [2026]);
    assert_eq!(b.felder["feld_mixed"].status, PruefStatus::Ok);
    assert_eq!(b.exit_code, 0);
    // Gibt es NUR das Jahr ohne Schema, ist das kein OK.
    let b = pruefe_bindung_mit(&[pruefling("feld_nur_2026", "E1111111", &[2026])], nur_e10(&pfad)).unwrap();
    assert_eq!(b.felder["feld_nur_2026"].status, PruefStatus::SchemaUnverfuegbar);
    assert_eq!(b.exit_code, 1);
}

#[test]
fn e60_kz_laufen_gegen_e77_und_alle_anderen_gegen_e10() {
    // Die Datenart-Weiche (`E6004901` steht nicht im E10-Schema, sondern in E77): die Suche sieht, welches Schema gefragt wird.
    let gefragt: RefCell<Vec<(i64, String)>> = RefCell::new(Vec::new());
    let b = pruefe_bindung_mit(
        &[pruefling("feld_euer", "E6004901", &[JAHR]), pruefling("feld_est", "E0100201", &[JAHR])],
        |jahr, muster| {
            gefragt.borrow_mut().push((jahr, muster.to_owned()));
            None
        },
    )
    .unwrap();
    let gefragt = gefragt.into_inner();
    assert!(gefragt.contains(&(JAHR, "E77-{jahr}.xsd".to_owned())), "{gefragt:?}");
    assert!(gefragt.contains(&(JAHR, "E10-{jahr}.xsd".to_owned())), "{gefragt:?}");
    assert_eq!(gefragt.len(), 2, "{gefragt:?}");
    assert_eq!(b.unverfuegbare_jahre, [JAHR]);
    // Und gegen ein echtes E77-Schema findet er das Kz dort, nicht in E10.
    let probe = Probe::neu("e77");
    let e77 = probe.schreibe(
        "e77.xsd",
        &(kopf("E77", "RootType")
            + r#"  <xs:complexType name="RootType"><xs:sequence><xs:element name="E6004901" type="xs:string"/></xs:sequence></xs:complexType>
</xs:schema>"#),
    );
    let b = pruefe_bindung_mit(&[pruefling("feld_euer", "E6004901", &[JAHR])], |jahr, muster| {
        (jahr == JAHR && muster == "E77-{jahr}.xsd").then(|| e77.clone())
    })
    .unwrap();
    assert_eq!(b.felder["feld_euer"].jahre[&JAHR].pfade, ["E77/E6004901"]);
    assert_eq!(b.exit_code, 0);
}

#[test]
fn ein_einbinden_weiterer_schemas_bricht_laut_ab_statt_weniger_zu_finden() {
    // Python loest `xs:include` auf; der Port nicht (E10/E77 2020-2025 haben keins, `XsdFehler::Include`). Er soll dann abbrechen,
    // statt still Kz zu uebersehen, die in der eingebundenen Datei stehen.
    let probe = Probe::neu("include");
    probe.schreibe(
        "typen.xsd",
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="UngenutztType"><xs:sequence/></xs:complexType></xs:schema>"#,
    );
    for einbinden in ["include", "import"] {
        let haupt = probe.schreibe(
            &format!("haupt_{einbinden}.xsd"),
            &(kopf("E10", "RootType").replace(
                "<xs:element name=\"E10\"",
                &format!("<xs:{einbinden} schemaLocation=\"typen.xsd\"/>\n  <xs:element name=\"E10\""),
            ) + r#"  <xs:complexType name="RootType"><xs:sequence><xs:element name="E3333333" type="xs:string"/></xs:sequence></xs:complexType>
</xs:schema>"#),
        );
        let fehler = xsd_walk(&haupt, "E10").unwrap_err();
        assert!(matches!(fehler, XsdFehler::Include(_)), "{einbinden}: {fehler:?}");
        let fehler = pruefe_bindung_mit(&[pruefling("feld_x", "E3333333", &[JAHR])], nur_e10(&haupt)).unwrap_err();
        assert!(matches!(fehler, XsdFehler::Include(_)), "{einbinden}: {fehler:?}");
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Die Kz der Transform-Tabellen als Prueflinge (`ernte_est_mapping_kz`), gegen die echte Bindung, ohne Schema.

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Bindung laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

#[test]
fn die_ernte_der_transform_tabellen_traegt_kz_und_jahre_der_echten_bindung() {
    let ernte = ernte_est_mapping_kz(index()).expect("jedes Wert-Feld der Tabellen steht in der Bindung");
    let nach_id: HashMap<&str, &KzPruefling> = ernte.iter().map(|k| (k.feld_id.as_str(), k)).collect();
    assert_eq!(nach_id.len(), ernte.len(), "eine feld_id kommt doppelt vor");
    println!("ernte_est_mapping_kz: {} Prueflinge", ernte.len());
    // Floor: gemessen 63 am 2026-10-13 auf a8ee3be1; eine leere Ernte waere gruen, ohne etwas zu pruefen.
    assert!(ernte.len() >= 60, "nur {} Prueflinge aus den Tabellen", ernte.len());
    let jahre = |feld: &str| index()[feld].vz_gueltigkeit.clone();
    let erwartet = [
        ("negation:fam_alleinstehend", "E0503701", "fam_alleinstehend"),
        ("verzweigung:rentner_jahresrente:sonstige_leibrente", "E1803102", "rentner_jahresrente"),
        // Der EIGENE Betrieb: gewerbe = G/Gew/Einz_U/Betr_1_2, selbstaendig = S/Gewinn/Freiber_T, NICHT die Ges_Fest-Kz des
        // gesondert festgestellten Beteiligungsanteils (E0800502/E0803402, Container-Korrektur 2026-08-20).
        ("verzweigung:einkuenfte_gewinn:gewerbe", "E0800302", "einkuenfte_gewinn"),
        ("verzweigung:einkuenfte_gewinn:selbstaendig", "E0803202", "einkuenfte_gewinn"),
        // Die Bezeichnung steht im selben Block wie der Betrag (checkESt verlangt sie gemeinsam).
        ("verzweigung:gewinn_bezeichnung:gewerbe", "E0800301", "gewinn_bezeichnung"),
        ("verzweigung:gewinn_bezeichnung:selbstaendig", "E0803101", "gewinn_bezeichnung"),
    ];
    for (id, kz, feld) in erwartet {
        let k = nach_id.get(id).unwrap_or_else(|| panic!("{id} fehlt in der Ernte"));
        assert_eq!(k.elster_kz, kz, "{id}");
        assert_eq!(k.vz_gueltigkeit, jahre(feld), "{id}: die Jahre sind die des Wert-Felds {feld}");
    }
    // land_forst steht BEWUSST ohne Kz (Anlage L: zwei Kz, die Wahl haengt an zwei Feldern, die es nicht gibt; fail-closed).
    assert!(!nach_id.contains_key("verzweigung:einkuenfte_gewinn:land_forst"));
}
