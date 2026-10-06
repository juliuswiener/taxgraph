//! Meldungstexte von `bindung`, die kein Bestandstest festnagelt (Mutationsmessung N4d, Bericht `mutation-bindung-domain`). Die
//! Bestandstests pruefen bei Lade-, Validierungs- und Registry-Fehlern nur die Variante (`matches!`); in `params_zugriff.rs` nennen
//! sie Datei und Schluessel nur auf den Wegen `dhf`, `verpflegung`, `altersentlastung` (zwei von vier), `versorgungsfreibetrag` und
//! `kinderfreibetrag`. Ein `#[error("..")]` oder ein `format!`-Literal, das durch `x` ersetzt wird, blieb dort gruen.
//!
//! Eine Meldung ohne Datei und Schluessel laesst den Betrieb vor einer kaputten `params/`-Datei ohne Ort zurueck: der Fehler steht
//! im Log, die Zeile fehlt. Jeder erwartete Text ist ausgeschrieben; nur die Betriebssystem-Ursache (`No such file ...`) und die
//! YAML-Ursache werden aus einer zweiten, unabhaengigen Lesung derselben Datei abgeleitet.
//!
//! * Lader: `lade_bindung`, `lade_kohorten`, `lade_params` (Io, Yaml), `lade_registry` (Verzeichnis, doppelte `feld_id`).
//! * `Bindung::validieren`: alle sechs Regeln mit vollem Text.
//! * `Params`: `DateiFehlt`, `SchluesselFehlt` und `Typ` auf den Wegen `wert`/`euro`, `oben`/`oben_euro`/`oben_staffel`,
//!   `kohorten_tabelle`/`kohorten_dezimal`, `altersentlastung_kohorte` und `versorgungsfreibetrag_kohorte` (leere Tabelle, Luecke
//!   zwischen den Jahren, fehlendes Feld).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use bindung::{
    lade_bindung, lade_kohorten, lade_params, lade_registry, BindungDatei, KohortenDatei, Params,
    ParamsDatei, RegistryFehler,
};
use domain::Vz;

fn verzeichnis(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("bindung-fehlertexte-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Die Ursache, die das Betriebssystem fuer `lies(pfad)` meldet.
fn io_ursache(pfad: &Path) -> String {
    let ursache = std::fs::read_to_string(pfad).unwrap_err().to_string();
    assert!(
        ursache.contains("No such file or directory"),
        "Gegenprobe: die zweite Lesung nennt eine Ursache: {ursache}"
    );
    ursache
}

/// Die Ursache, die `serde_yaml_ng` fuer `text` als `T` meldet. `T` ist der Typ des Laders: ein Syntaxfehler am Ende der Datei
/// kommt je nach Zieltyp vor oder nach einem Typfehler (`unknown field`) zum Vorschein.
fn yaml_ursache<T: serde::de::DeserializeOwned + std::fmt::Debug>(text: &str) -> String {
    let ursache = serde_yaml_ng::from_str::<T>(text).unwrap_err().to_string();
    assert!(
        !ursache.is_empty(),
        "Gegenprobe: die zweite Lesung nennt eine Ursache"
    );
    ursache
}

// ---- Lader ----------------------------------------------------------------------------------------------------------

#[test]
fn lader_melden_pfad_und_ursache() {
    let d = verzeichnis("lader");
    let fehlt = d.join("fehlt.yaml");
    let kaputt = d.join("kaputt.yaml");
    let kaputt_text = "kaputt: [\n";
    std::fs::write(&kaputt, kaputt_text).unwrap();
    let io = format!(
        "konnte {} nicht lesen: {}",
        fehlt.display(),
        io_ursache(&fehlt)
    );
    let yaml = |ursache: String| format!("YAML-Fehler in {}: {ursache}", kaputt.display());

    assert_eq!(lade_bindung(&fehlt).unwrap_err().to_string(), io);
    assert_eq!(
        lade_bindung(&kaputt).unwrap_err().to_string(),
        yaml(yaml_ursache::<BindungDatei>(kaputt_text))
    );
    assert_eq!(lade_kohorten(&fehlt).unwrap_err().to_string(), io);
    assert_eq!(
        lade_kohorten(&kaputt).unwrap_err().to_string(),
        yaml(yaml_ursache::<KohortenDatei>(kaputt_text))
    );
    assert_eq!(lade_params(&fehlt).unwrap_err().to_string(), io);
    assert_eq!(
        lade_params(&kaputt).unwrap_err().to_string(),
        yaml(yaml_ursache::<ParamsDatei>(kaputt_text))
    );
    std::fs::remove_dir_all(&d).unwrap();
}

fn gueltig(feld_id: &str) -> String {
    format!(
        "version: 1\nscheibe: test\nbindungen:\n  - feld_id: {feld_id}\n    quelle: {{regel_id: r, signatur_slot: s}}\n    typ: bool\n    \
         askable: false\n    hilfe_kurz: Tipp\n    beispielwert: true\n    elster_kz: \"E0123456\"\n    vz_gueltigkeit: [2025]\n    \
         anker_ref: {{quelle: Q, zitatanker: Zit}}\n"
    )
}

#[test]
fn registry_meldet_verzeichnis_und_doppelte_feld_id_mit_text() {
    let d = verzeichnis("registry");
    let fehlt = d.join("gibt_es_nicht");
    let ursache = std::fs::read_dir(&fehlt).unwrap_err().to_string();
    assert!(ursache.contains("No such file or directory"), "{ursache}");
    let fehler = lade_registry(&fehlt).unwrap_err();
    assert!(matches!(fehler, RegistryFehler::Verzeichnis { .. }));
    assert_eq!(
        fehler.to_string(),
        format!(
            "konnte Verzeichnis {} nicht lesen: {ursache}",
            fehlt.display()
        )
    );

    std::fs::write(d.join("bindung_a.yaml"), gueltig("feld_x")).unwrap();
    std::fs::write(d.join("bindung_b.yaml"), gueltig("feld_x")).unwrap();
    assert_eq!(
        lade_registry(&d).unwrap_err().to_string(),
        format!(
            "feld_x: doppelt gebunden in {} und {}",
            d.join("bindung_a.yaml").display(),
            d.join("bindung_b.yaml").display()
        )
    );
    std::fs::remove_dir_all(&d).unwrap();
}

// ---- Bindung::validieren --------------------------------------------------------------------------------------------

/// Eine gueltige Bindung `testfeld` (bool, nicht askable, Kz, ein VZ); `aenderungen` ersetzt Teilstrings, `zusatz` haengt Felder an.
fn meldung(aenderungen: &[(&str, &str)], zusatz: &str) -> String {
    let mut yaml = gueltig("testfeld");
    for (alt, neu) in aenderungen {
        assert_eq!(yaml.matches(alt).count(), 1, "Anker nicht eindeutig: {alt}");
        yaml = yaml.replace(alt, neu);
    }
    yaml.push_str(zusatz);
    let datei: BindungDatei = serde_yaml_ng::from_str(&yaml).unwrap();
    datei.bindungen[0].validieren().unwrap_err().to_string()
}

#[test]
fn validierung_meldet_jede_regel_mit_feld_id_und_text() {
    assert_eq!(
        meldung(&[("feld_id: testfeld", "feld_id: Testfeld")], ""),
        "ungueltige feld_id \"Testfeld\" (erwartet ^[a-z][a-z0-9_]*$)"
    );
    assert_eq!(
        meldung(&[("[2025]", "[]")], ""),
        "testfeld: vz_gueltigkeit darf nicht leer sein"
    );
    assert_eq!(
        meldung(&[("askable: false", "askable: true")], ""),
        "testfeld: askable=true braucht fragetext_laie (min. 5 Zeichen)"
    );
    assert_eq!(
        meldung(&[("typ: bool", "typ: enum")], ""),
        "testfeld: typ=enum braucht enum_werte"
    );
    assert_eq!(
        meldung(&[("elster_kz: \"E0123456\"", "elster_kz: null")], ""),
        "testfeld: elster_kz=null braucht elster_kz_grund"
    );
    assert_eq!(
        meldung(&[], "    frage_invertiert: true\n"),
        "testfeld: frage_invertiert=true braucht typ=bool und askable=true"
    );
}

// ---- Params: Jahresdateien -------------------------------------------------------------------------------------------

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn kopiere(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for eintrag in std::fs::read_dir(von).unwrap() {
        let eintrag = eintrag.unwrap();
        let ziel = nach.join(eintrag.file_name());
        if eintrag.file_type().unwrap().is_dir() {
            kopiere(&eintrag.path(), &ziel);
        } else {
            std::fs::copy(eintrag.path(), &ziel).unwrap();
        }
    }
}

/// Temp-Kopie von `params/` (Wurzel mit `params/` darin); `Drop` raeumt auf.
struct Wurzel(PathBuf);

impl Wurzel {
    fn neu(name: &str) -> Self {
        let w = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("bindung-fehlertexte-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&w);
        kopiere(&repo().join("params"), &w.join("params"));
        Self(w)
    }

    fn pfad(&self, rel: &str) -> PathBuf {
        self.0.join("params").join(rel)
    }

    /// Ersetzt `alt` durch `neu`; `alt` muss in der Datei genau einmal vorkommen (sonst ist der Test taub).
    fn ersetze(&self, rel: &str, alt: &str, neu: &str) {
        let p = self.pfad(rel);
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(
            text.matches(alt).count(),
            1,
            "Anker nicht einwandfrei in {rel}: {alt:?}"
        );
        std::fs::write(&p, text.replacen(alt, neu, 1)).unwrap();
    }

    fn lade(&self) -> Params {
        Params::lade(&self.0).unwrap()
    }
}

impl Drop for Wurzel {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `wert()`, `euro()` und `datei()`: Datei fehlt, Schluessel `wert` fehlt, Wert ist keine ganze Zahl.
#[test]
fn jahresdatei_fehler_nennen_datei_und_schluessel() {
    let datei = "2025/arbeitnehmerpauschbetrag.yaml";

    let w = Wurzel::neu("jahr-datei");
    std::fs::remove_file(w.pfad(datei)).unwrap();
    assert_eq!(
        w.lade()
            .arbeitnehmer_pauschbetrag(Vz::Vz2025)
            .unwrap_err()
            .to_string(),
        "Parameterdatei arbeitnehmerpauschbetrag.yaml fuer VZ 2025 nicht geladen"
    );

    let w = Wurzel::neu("jahr-schluessel");
    w.ersetze(datei, "wert:\n  wert: 1230", "wert:\n  betrag: 1230");
    assert_eq!(
        w.lade()
            .arbeitnehmer_pauschbetrag(Vz::Vz2025)
            .unwrap_err()
            .to_string(),
        "2025/arbeitnehmerpauschbetrag.yaml: Schluessel wert.wert fehlt"
    );

    let w = Wurzel::neu("jahr-typ");
    w.ersetze(datei, "  wert: 1230", "  wert: abc");
    assert_eq!(
        w.lade()
            .arbeitnehmer_pauschbetrag(Vz::Vz2025)
            .unwrap_err()
            .to_string(),
        "2025/arbeitnehmerpauschbetrag.yaml: Schluessel wert.wert ist keine ganze Zahl"
    );
}

/// `oben()`, `oben_euro()` und `oben_staffel()` (Dateien ohne `wert`-Huelle): Schluessel fehlt, Betrag keine ganze Zahl, Staffel
/// ohne ganzzahligen Schluessel.
#[test]
fn oben_fehler_nennen_datei_und_schluessel() {
    let datei = "2025/behinderten_pauschbetrag_p33b.yaml";

    let w = Wurzel::neu("oben-fehlt");
    w.ersetze(datei, "\nblind_hilflos_taubblind: 7400", "\nblind_x: 7400");
    assert_eq!(
        w.lade()
            .p33b_pauschbetraege(Vz::Vz2025)
            .unwrap_err()
            .to_string(),
        "2025/behinderten_pauschbetrag_p33b.yaml: Schluessel blind_hilflos_taubblind fehlt"
    );

    let w = Wurzel::neu("oben-typ");
    w.ersetze(
        datei,
        "\nblind_hilflos_taubblind: 7400",
        "\nblind_hilflos_taubblind: abc",
    );
    assert_eq!(
        w.lade()
            .p33b_pauschbetraege(Vz::Vz2025)
            .unwrap_err()
            .to_string(),
        "2025/behinderten_pauschbetrag_p33b.yaml: Schluessel blind_hilflos_taubblind ist keine ganze Zahl"
    );

    let w = Wurzel::neu("oben-staffel");
    w.ersetze(
        datei,
        "gdb_staffel:\n  20: 384",
        "gdb_staffel:\n  zwanzig: 384",
    );
    assert_eq!(
        w.lade()
            .p33b_pauschbetraege(Vz::Vz2025)
            .unwrap_err()
            .to_string(),
        "2025/behinderten_pauschbetrag_p33b.yaml: Schluessel gdb_staffel ist keine Staffel {ganze Zahl: ganze Zahl}"
    );

    let w = Wurzel::neu("oben-fahrt");
    w.ersetze(
        "2025/fahrtkostenpauschale_p33_2a.yaml",
        "\npauschale_900: 900",
        "\npauschale_x: 900",
    );
    assert_eq!(
        w.lade()
            .fahrtkostenpauschale_p33_2a(Vz::Vz2025)
            .unwrap_err()
            .to_string(),
        "2025/fahrtkostenpauschale_p33_2a.yaml: Schluessel pauschale_900 fehlt"
    );
}

// ---- Params: Kohortentabellen ----------------------------------------------------------------------------------------

/// `kohorten_tabelle()` und `kohorten_dezimal()`: Tabelle fehlt (Schluessel oder ganze Datei), Zelle keine Dezimalzahl.
#[test]
fn kohortentabelle_fehler_nennen_datei_und_schluessel() {
    let w = Wurzel::neu("tab-schluessel");
    w.ersetze(
        "kohorten/rente_besteuerungsanteil_p22.yaml",
        "\nkohorten:\n",
        "\nkohorten_x:\n",
    );
    assert_eq!(
        w.lade()
            .rente_besteuerungsanteil(2025)
            .unwrap_err()
            .to_string(),
        "kohorten/rente_besteuerungsanteil_p22.yaml: Schluessel kohorten fehlt"
    );

    let w = Wurzel::neu("tab-datei");
    std::fs::remove_file(w.pfad("kohorten/rente_ertragsanteil_p22.yaml")).unwrap();
    assert_eq!(
        w.lade().rente_ertragsanteil(0).unwrap_err().to_string(),
        "kohorten/rente_ertragsanteil_p22.yaml: Schluessel kohorten fehlt"
    );

    let w = Wurzel::neu("tab-zelle");
    w.ersetze(
        "kohorten/rente_besteuerungsanteil_p22.yaml",
        "2025: {besteuerungsanteil_prozent: 83.5}",
        "2025: {besteuerungsanteil_prozent: abc}",
    );
    assert_eq!(
        w.lade().rente_besteuerungsanteil(2025).unwrap_err().to_string(),
        "kohorten/rente_besteuerungsanteil_p22.yaml: Schluessel kohorten.2025.besteuerungsanteil_prozent ist keine Dezimalzahl"
    );

    let w = Wurzel::neu("tab-zelle-alter");
    w.ersetze(
        "kohorten/rente_ertragsanteil_p22.yaml",
        "\n  0: {ertragsanteil_prozent: 59.0}",
        "\n  0: {ertragsanteil_prozent: abc}",
    );
    assert_eq!(
        w.lade().rente_ertragsanteil(0).unwrap_err().to_string(),
        "kohorten/rente_ertragsanteil_p22.yaml: Schluessel kohorten.0.ertragsanteil_prozent ist keine Dezimalzahl"
    );
}

/// `altersentlastung_kohorte()`: leere Tabelle, Luecke zwischen den Jahren (das geklemmte Jahr hat keine Zeile), fehlendes Feld.
#[test]
fn altersentlastung_kohorte_fehler_nennen_zeile_und_feld() {
    let d = "kohorten/altersentlastungsbetrag_p24a.yaml";

    let w = Wurzel::neu("alter-leer");
    w.ersetze(d, "\nkohorten:\n", "\nkohorten: {}\nkohorten_alt:\n");
    assert_eq!(
        w.lade()
            .altersentlastung_kohorte(2025)
            .unwrap_err()
            .to_string(),
        "kohorten/altersentlastungsbetrag_p24a.yaml: Schluessel kohorten.<jahr> fehlt"
    );

    let w = Wurzel::neu("alter-luecke");
    w.ersetze(
        d,
        "  2025: {prozentsatz: 13.2,",
        "  \"2025x\": {prozentsatz: 13.2,",
    );
    assert_eq!(
        w.lade()
            .altersentlastung_kohorte(2025)
            .unwrap_err()
            .to_string(),
        "kohorten/altersentlastungsbetrag_p24a.yaml: Schluessel kohorten.2025 fehlt"
    );

    for (name, alt, neu, feld) in [
        (
            "alter-prozent-fehlt",
            "2025: {prozentsatz: 13.2,",
            "2025: {prozentsatz_x: 13.2,",
            "prozentsatz",
        ),
        (
            "alter-hoechst-fehlt",
            "hoechstbetrag: 627}",
            "hoechstbetrag_x: 627}",
            "hoechstbetrag",
        ),
    ] {
        let w = Wurzel::neu(name);
        w.ersetze(d, alt, neu);
        assert_eq!(
            w.lade()
                .altersentlastung_kohorte(2025)
                .unwrap_err()
                .to_string(),
            format!(
                "kohorten/altersentlastungsbetrag_p24a.yaml: Schluessel kohorten.2025.{feld} fehlt"
            )
        );
    }
}

/// `versorgungsfreibetrag_kohorte()`: leere Tabelle und Luecke zwischen den Jahren.
#[test]
fn versorgungsfreibetrag_kohorte_fehler_nennen_zeile() {
    let d = "kohorten/versorgungsfreibetrag_p19_2.yaml";

    let w = Wurzel::neu("vfb-leer");
    w.ersetze(d, "\nkohorten:\n", "\nkohorten: {}\nkohorten_alt:\n");
    assert_eq!(
        w.lade()
            .versorgungsfreibetrag_kohorte(2025)
            .unwrap_err()
            .to_string(),
        "kohorten/versorgungsfreibetrag_p19_2.yaml: Schluessel kohorten.<jahr> fehlt"
    );

    let w = Wurzel::neu("vfb-luecke");
    w.ersetze(
        d,
        "  2025: {prozentsatz: 13.2,",
        "  \"2025x\": {prozentsatz: 13.2,",
    );
    assert_eq!(
        w.lade()
            .versorgungsfreibetrag_kohorte(2025)
            .unwrap_err()
            .to_string(),
        "kohorten/versorgungsfreibetrag_p19_2.yaml: Schluessel kohorten.2025 fehlt"
    );
}
