//! Zuordnung Accessor-Feld -> YAML-Schluessel in `bindung::Params`, hermetisch (ohne `PARITY=1`, ohne Python). Messung N4
//! (`berichte/mutation-bindung-domain.md`): Mutanten, die in `params_zugriff.rs` den Schluessel oder die Datei eines Feldes gegen den
//! eines Nachbarfeldes tauschen, ueberlebten die Doctests (je Accessor EIN Feld) und die Bestandstests. Das ist die Gefahr "falscher
//! Wert in der Steuererklaerung".
//!
//! * `felder_gegen_unabhaengige_lesung`: jedes Feld jedes Accessors gegen eine zweite Lesung derselben YAML-Datei (mit festen
//!   Schluesselnamen im Test), fuer alle drei Veranlagungszeitraeume und alle Kohortenzeilen.
//! * `felder_bei_lauter_verschiedenen_werten`: dieselbe Pruefung auf einer Temp-Kopie von `params/`, in der jede Zahl einer Datei
//!   einen eigenen Wert hat. Die echten Dateien haben gleiche Werte in Nachbarfeldern (`pauschale_an_abreise` == `pauschale_ab_8h` ==
//!   14, `jahrespauschale` == `tagespauschale_hoechstbetrag` == 1260); ein Tausch fiele dort nicht auf.
//! * Fehlerpfade: Typ- und Schluesselfehler tragen Datei und Schluessel im Text; eine gepatchte Temp-Kopie erreicht sie.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    clippy::panic,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use bindung::Params;
use domain::{Euro, Vz};
use rust_decimal::Decimal;
use serde_yaml_ng::Value;

const VZS: [Vz; 3] = [Vz::Vz2024, Vz::Vz2025, Vz::Vz2026];

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
            .join(format!("bindung-zuordnung-{name}-{}", std::process::id()));
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

fn yaml(wurzel: &Path, rel: &str) -> Value {
    let text = std::fs::read_to_string(wurzel.join("params").join(rel)).unwrap();
    serde_yaml_ng::from_str(&text).unwrap()
}

fn zahl(v: &Value) -> Decimal {
    v.as_i64().map_or_else(
        || Decimal::from_str(&v.as_f64().unwrap().to_string()).unwrap(),
        Decimal::from,
    )
}

/// `y[schluessel].wert`, oder `y[schluessel]`, wo die Datei keine `wert`-Huelle fuehrt.
fn gelesen(y: &Value, schluessel: &str) -> Decimal {
    let v = &y[schluessel];
    assert!(!v.is_null(), "Schluessel {schluessel} fehlt in der Datei");
    zahl(v.get("wert").unwrap_or(v))
}

fn euro(x: Euro) -> Decimal {
    Decimal::from(x.get())
}

fn staffel(y: &Value, schluessel: &str) -> BTreeMap<i64, Decimal> {
    y[schluessel]
        .as_mapping()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.as_i64().unwrap(), zahl(v)))
        .collect()
}

fn euro_staffel(m: &BTreeMap<i64, Euro>) -> BTreeMap<i64, Decimal> {
    m.iter().map(|(k, v)| (*k, euro(*v))).collect()
}

fn pruefe_jahresfelder(wurzel: &Path) {
    let p = Params::lade(wurzel).unwrap();
    for vz in VZS {
        let j = vz.jahr();
        let y = |datei: &str| yaml(wurzel, &format!("{j}/{datei}"));
        let m = |feld: &str| format!("VZ {j}: {feld}");

        let t = y("einkommensteuertarif_p32a.yaml");
        assert_eq!(
            euro(p.grundfreibetrag(vz).unwrap()),
            gelesen(&t, "grundfreibetrag"),
            "{}",
            m("grundfreibetrag")
        );
        let a = y("arbeitnehmerpauschbetrag.yaml");
        assert_eq!(
            euro(p.arbeitnehmer_pauschbetrag(vz).unwrap()),
            gelesen(&a, "wert"),
            "{}",
            m("arbeitnehmer_pauschbetrag")
        );

        let z = y("arbeitszimmer_homeoffice.yaml");
        let az = p.arbeitszimmer(vz).unwrap();
        assert_eq!(
            euro(az.jahrespauschale),
            gelesen(&z, "jahrespauschale"),
            "{}",
            m("jahrespauschale")
        );
        assert_eq!(
            euro(az.tagespauschale_pro_tag),
            gelesen(&z, "tagespauschale_pro_tag"),
            "{}",
            m("tagespauschale_pro_tag")
        );
        assert_eq!(
            euro(az.tagespauschale_hoechstbetrag),
            gelesen(&z, "tagespauschale_hoechstbetrag"),
            "{}",
            m("tagespauschale_hoechstbetrag")
        );

        let e = y("entfernungspauschale.yaml");
        let ep = p.entfernungspauschale(vz).unwrap();
        assert_eq!(
            ep.satz_bis_20_km.get(),
            gelesen(&e, "satz_bis_20_km"),
            "{}",
            m("satz_bis_20_km")
        );
        assert_eq!(
            ep.satz_ab_21_km.get(),
            gelesen(&e, "satz_ab_21_km"),
            "{}",
            m("satz_ab_21_km")
        );
        assert_eq!(
            Decimal::from(ep.staffelgrenze_km),
            gelesen(&e, "staffelgrenze_km"),
            "{}",
            m("staffelgrenze_km")
        );
        assert_eq!(
            euro(ep.hoechstbetrag_ohne_kfz),
            gelesen(&e, "hoechstbetrag_ohne_kfz"),
            "{}",
            m("hoechstbetrag_ohne_kfz")
        );

        let d = y("dhf_p9_1_nr5.yaml");
        let dhf = p.dhf(vz).unwrap();
        assert_eq!(
            euro(dhf.cap_monat_inland),
            gelesen(&d, "cap_monat_inland"),
            "{}",
            m("cap_monat_inland")
        );
        assert_eq!(
            dhf.cap_monat_ausland.map(euro),
            (!d["cap_monat_ausland"].is_null()).then(|| gelesen(&d, "cap_monat_ausland")),
            "{}",
            m("cap_monat_ausland")
        );

        let v = y("verpflegung_p9_4a.yaml");
        let vp = p.verpflegung(vz).unwrap();
        assert_eq!(
            euro(vp.pauschale_24h),
            gelesen(&v, "pauschale_24h"),
            "{}",
            m("pauschale_24h")
        );
        assert_eq!(
            euro(vp.pauschale_an_abreise),
            gelesen(&v, "pauschale_an_abreise"),
            "{}",
            m("pauschale_an_abreise")
        );
        assert_eq!(
            euro(vp.pauschale_ab_8h),
            gelesen(&v, "pauschale_ab_8h"),
            "{}",
            m("pauschale_ab_8h")
        );
        assert_eq!(
            Decimal::from(vp.kuerzung_fruehstueck_prozent),
            gelesen(&v, "kuerzung_fruehstueck_prozent"),
            "{}",
            m("kuerzung_fruehstueck_prozent")
        );
        assert_eq!(
            Decimal::from(vp.kuerzung_mittag_abend_prozent),
            gelesen(&v, "kuerzung_mittag_abend_prozent"),
            "{}",
            m("kuerzung_mittag_abend_prozent")
        );

        let s = y("sparer_pauschbetrag_p20_9.yaml");
        assert_eq!(
            euro(p.sparer_pauschbetrag(vz).unwrap()),
            gelesen(&s, "wert"),
            "{}",
            m("sparer_pauschbetrag")
        );
        let g = y("abgeltungssatz_p32d.yaml");
        assert_eq!(
            Decimal::from(p.abgeltungssatz_prozent(vz).unwrap()),
            gelesen(&g, "wert"),
            "{}",
            m("abgeltungssatz_prozent")
        );
        let r = y("renten_werbungskostenpauschbetrag_p9a.yaml");
        assert_eq!(
            euro(p.renten_wk_pauschbetrag(vz).unwrap()),
            gelesen(&r, "wert"),
            "{}",
            m("renten_wk_pauschbetrag")
        );

        let b = y("behinderten_pauschbetrag_p33b.yaml");
        let pb = p.p33b_pauschbetraege(vz).unwrap();
        assert_eq!(
            euro_staffel(&pb.gdb_staffel),
            staffel(&b, "gdb_staffel"),
            "{}",
            m("gdb_staffel")
        );
        assert_eq!(
            euro(pb.blind_hilflos_taubblind),
            gelesen(&b, "blind_hilflos_taubblind"),
            "{}",
            m("blind_hilflos_taubblind")
        );
        assert_eq!(
            euro(pb.hinterbliebenen),
            gelesen(&b, "hinterbliebenen"),
            "{}",
            m("hinterbliebenen")
        );
        assert_eq!(
            euro_staffel(&pb.pflege_staffel),
            staffel(&b, "pflege_staffel"),
            "{}",
            m("pflege_staffel")
        );
        assert_eq!(
            euro(pb.pflege_hilflos),
            gelesen(&b, "pflege_hilflos"),
            "{}",
            m("pflege_hilflos")
        );

        let f = y("fahrtkostenpauschale_p33_2a.yaml");
        let fp = p.fahrtkostenpauschale_p33_2a(vz).unwrap();
        assert_eq!(
            euro(fp.pauschale_900),
            gelesen(&f, "pauschale_900"),
            "{}",
            m("pauschale_900")
        );
        assert_eq!(
            euro(fp.pauschale_4500),
            gelesen(&f, "pauschale_4500"),
            "{}",
            m("pauschale_4500")
        );

        for (name, datei, sh) in [
            (
                "kinderbetreuung",
                "kinderbetreuung_p10.yaml",
                p.kinderbetreuung(vz).unwrap(),
            ),
            ("schulgeld", "schulgeld_p10.yaml", p.schulgeld(vz).unwrap()),
        ] {
            let k = y(datei);
            assert_eq!(
                sh.abzugssatz.get(),
                gelesen(&k, "abzugssatz"),
                "{}",
                m(&format!("{name}.abzugssatz"))
            );
            assert_eq!(
                euro(sh.hoechstbetrag_je_kind),
                gelesen(&k, "hoechstbetrag_je_kind"),
                "{}",
                m(&format!("{name}.hoechstbetrag_je_kind"))
            );
        }

        let kg = y("kindergeld_p66.yaml");
        assert_eq!(
            euro(p.kindergeld_monatlich_je_kind(vz).unwrap()),
            gelesen(&kg, "kindergeld_monatlich_je_kind"),
            "{}",
            m("kindergeld")
        );
        let kf = y("kinderfreibetrag_p32.yaml");
        assert_eq!(
            euro(p.kinderfreibetrag_je_elternteil(vz).unwrap()),
            gelesen(&kf, "kinderfreibetrag_je_elternteil")
                + gelesen(&kf, "bea_freibetrag_je_elternteil"),
            "{}",
            m("kinderfreibetrag_je_elternteil")
        );
        let vh = y("vorsorge_hoechstbetrag_p10.yaml");
        assert_eq!(
            euro(p.vorsorge_hoechstbeitrag(vz).unwrap()),
            gelesen(&vh, "hoechstbeitrag"),
            "{}",
            m("vorsorge_hoechstbeitrag")
        );
        let sa = y("sonderausgabenpauschbetrag.yaml");
        assert_eq!(
            euro(p.sonderausgaben_pauschbetrag(vz).unwrap()),
            gelesen(&sa, "wert"),
            "{}",
            m("sonderausgaben_pauschbetrag")
        );
    }
}

fn pruefe_kohorten(wurzel: &Path) {
    let p = Params::lade(wurzel).unwrap();
    let bt = yaml(wurzel, "kohorten/rente_besteuerungsanteil_p22.yaml");
    let ea = yaml(wurzel, "kohorten/rente_ertragsanteil_p22.yaml");
    let vf = yaml(wurzel, "kohorten/versorgungsfreibetrag_p19_2.yaml");
    let ae = yaml(wurzel, "kohorten/altersentlastungsbetrag_p24a.yaml");
    let zeile = |y: &Value, j: i64| {
        y["kohorten"]
            .as_mapping()
            .unwrap()
            .get(Value::from(j))
            .cloned()
    };
    let feld = |z: &Value, k: &str| zahl(&z[k]);
    for jahr in 2000..=2070 {
        let erwartet = zeile(&bt, jahr).map(|z| feld(&z, "besteuerungsanteil_prozent"));
        assert_eq!(
            p.rente_besteuerungsanteil(jahr)
                .unwrap()
                .map(domain::Satz::get),
            erwartet,
            "Besteuerungsanteil {jahr}"
        );
    }
    for alter in -2..=105 {
        let erwartet = zeile(&ea, alter).map(|z| feld(&z, "ertragsanteil_prozent"));
        assert_eq!(
            p.rente_ertragsanteil(alter).unwrap().map(domain::Satz::get),
            erwartet,
            "Ertragsanteil {alter}"
        );
    }
    let (v_min, v_max) = (2005, 2058);
    for beginn in 1990..=2070 {
        let j = beginn.clamp(v_min, v_max);
        let z = zeile(&vf, j).unwrap();
        let k = p.versorgungsfreibetrag_kohorte(beginn).unwrap();
        assert_eq!(
            k.prozentsatz.get(),
            feld(&z, "prozentsatz"),
            "Versorgung {beginn} prozentsatz"
        );
        assert_eq!(
            euro(k.hoechstbetrag),
            feld(&z, "hoechstbetrag"),
            "Versorgung {beginn} hoechstbetrag"
        );
        assert_eq!(
            euro(k.zuschlag),
            feld(&z, "zuschlag"),
            "Versorgung {beginn} zuschlag"
        );
        let z = zeile(&ae, j).unwrap();
        let k = p.altersentlastung_kohorte(beginn).unwrap();
        assert_eq!(
            k.prozentsatz.get(),
            feld(&z, "prozentsatz"),
            "Altersentlastung {beginn} prozentsatz"
        );
        assert_eq!(
            euro(k.hoechstbetrag),
            feld(&z, "hoechstbetrag"),
            "Altersentlastung {beginn} hoechstbetrag"
        );
    }
}

#[test]
fn felder_gegen_unabhaengige_lesung() {
    pruefe_jahresfelder(&repo());
    pruefe_kohorten(&repo());
}

/// Jede Zahl einer Jahresdatei bekommt einen eigenen Wert (7001, 7002, ...): ein Tausch zweier Schluessel ist dann an jedem Feld
/// sichtbar, auch wo die echten Dateien gleiche Werte fuehren.
#[test]
fn felder_bei_lauter_verschiedenen_werten() {
    let w = Wurzel::neu("eindeutig");
    let mut zaehler = 7000;
    for vz in VZS {
        let dir = w.pfad(&vz.jahr().to_string());
        let mut dateien: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        dateien.sort();
        for pfad in dateien {
            let text = std::fs::read_to_string(&pfad).unwrap();
            let neu: Vec<String> = text
                .lines()
                .map(|zeile| {
                    let t = zeile.trim_start();
                    let einzug = &zeile[..zeile.len() - t.len()];
                    let top = !einzug.is_empty() || t.contains(':');
                    if let Some(rest) = t.strip_prefix("wert: ") {
                        if rest.trim().parse::<f64>().is_ok() {
                            zaehler += 1;
                            return format!("{einzug}wert: {zaehler}");
                        }
                    }
                    for k in [
                        "pauschale_900",
                        "pauschale_4500",
                        "blind_hilflos_taubblind",
                        "hinterbliebenen",
                        "pflege_hilflos",
                    ] {
                        if einzug.is_empty() && top {
                            if let Some(rest) = t.strip_prefix(&format!("{k}: ")) {
                                if rest.trim().parse::<f64>().is_ok() {
                                    zaehler += 1;
                                    return format!("{k}: {zaehler}");
                                }
                            }
                        }
                    }
                    zeile.to_owned()
                })
                .collect();
            std::fs::write(&pfad, neu.join("\n") + "\n").unwrap();
        }
    }
    pruefe_jahresfelder(&w.0);
}

#[test]
fn dhf_auslandsgrenze_ohne_block_ist_ein_typfehler() {
    let w = Wurzel::neu("dhf");
    // Der Block `cap_monat_ausland` steht am Dateiende: durch einen Skalar ersetzen.
    let p = w.pfad("2026/dhf_p9_1_nr5.yaml");
    let text = std::fs::read_to_string(&p).unwrap();
    let ab = text.find("cap_monat_ausland:").unwrap();
    std::fs::write(&p, format!("{}cap_monat_ausland: 2000\n", &text[..ab])).unwrap();
    let fehler = w.lade().dhf(Vz::Vz2026).unwrap_err();
    assert_eq!(
        fehler.to_string(),
        "2026/dhf_p9_1_nr5.yaml: Schluessel cap_monat_ausland.wert ist keine Block"
    );
}

#[test]
fn verpflegung_fehlende_kuerzung_ist_null_und_ein_unlesbarer_wert_ein_fehler() {
    let w = Wurzel::neu("verpflegung");
    let p = w.pfad("2025/verpflegung_p9_4a.yaml");
    let text = std::fs::read_to_string(&p).unwrap();
    let ab = text.find("kuerzung_fruehstueck_prozent:").unwrap();
    std::fs::write(&p, &text[..ab]).unwrap();
    let v = w.lade().verpflegung(Vz::Vz2025).unwrap();
    assert_eq!(
        (
            v.kuerzung_fruehstueck_prozent,
            v.kuerzung_mittag_abend_prozent
        ),
        (0, 0)
    );
    assert_eq!(
        (v.pauschale_24h, v.pauschale_an_abreise, v.pauschale_ab_8h),
        (Euro::new(28), Euro::new(14), Euro::new(14))
    );

    let w = Wurzel::neu("verpflegung-typ");
    w.ersetze(
        "2025/verpflegung_p9_4a.yaml",
        "kuerzung_fruehstueck_prozent:\n  wert: 20",
        "kuerzung_fruehstueck_prozent:\n  wert: abc",
    );
    let fehler = w.lade().verpflegung(Vz::Vz2025).unwrap_err();
    assert_eq!(
        fehler.to_string(),
        "2025/verpflegung_p9_4a.yaml: Schluessel kuerzung_fruehstueck_prozent.wert ist keine ganze Zahl"
    );
}

#[test]
fn altersentlastung_fehler_nennen_datei_und_schluessel() {
    let w = Wurzel::neu("alter-tabelle");
    w.ersetze(
        "kohorten/altersentlastungsbetrag_p24a.yaml",
        "\nkohorten:\n",
        "\nkohorten_x:\n",
    );
    let fehler = w.lade().altersentlastung_kohorte(2025).unwrap_err();
    assert_eq!(
        fehler.to_string(),
        "kohorten/altersentlastungsbetrag_p24a.yaml: Schluessel kohorten fehlt"
    );

    let w = Wurzel::neu("alter-prozent");
    w.ersetze(
        "kohorten/altersentlastungsbetrag_p24a.yaml",
        "2025: {prozentsatz: 13.2,",
        "2025: {prozentsatz: abc,",
    );
    let fehler = w.lade().altersentlastung_kohorte(2025).unwrap_err();
    assert_eq!(
        fehler.to_string(),
        "kohorten/altersentlastungsbetrag_p24a.yaml: Schluessel kohorten.2025.prozentsatz ist keine Dezimalzahl"
    );

    let w = Wurzel::neu("alter-hoechstbetrag");
    w.ersetze(
        "kohorten/altersentlastungsbetrag_p24a.yaml",
        "hoechstbetrag: 627}",
        "hoechstbetrag: abc}",
    );
    let fehler = w.lade().altersentlastung_kohorte(2025).unwrap_err();
    assert_eq!(
        fehler.to_string(),
        "kohorten/altersentlastungsbetrag_p24a.yaml: Schluessel kohorten.2025.hoechstbetrag ist keine ganze Zahl"
    );
}

#[test]
fn versorgungsfreibetrag_fehler_nennen_datei_und_schluessel() {
    for (name, alt, neu, schluessel, erwartet) in [
        (
            "prozent",
            "2025: {prozentsatz: 13.2,",
            "2025: {prozentsatz: abc,",
            "prozentsatz",
            "Dezimalzahl",
        ),
        (
            "hoechst",
            "hoechstbetrag: 990,",
            "hoechstbetrag: abc,",
            "hoechstbetrag",
            "ganze Zahl",
        ),
        (
            "zuschlag",
            "zuschlag: 297}",
            "zuschlag: abc}",
            "zuschlag",
            "ganze Zahl",
        ),
    ] {
        let w = Wurzel::neu(&format!("vfb-{name}"));
        w.ersetze("kohorten/versorgungsfreibetrag_p19_2.yaml", alt, neu);
        let fehler = w.lade().versorgungsfreibetrag_kohorte(2025).unwrap_err();
        assert_eq!(
            fehler.to_string(),
            format!("kohorten/versorgungsfreibetrag_p19_2.yaml: Schluessel kohorten.2025.{schluessel} ist keine {erwartet}")
        );
    }
}

#[test]
fn kinderfreibetrag_summe_ausserhalb_i64_nennt_den_schluessel() {
    let w = Wurzel::neu("kfb");
    w.ersetze(
        "2025/kinderfreibetrag_p32.yaml",
        "wert: 3336",
        "wert: 9223372036854775807",
    );
    let fehler = w
        .lade()
        .kinderfreibetrag_je_elternteil(Vz::Vz2025)
        .unwrap_err();
    assert_eq!(
        fehler.to_string(),
        "2025/kinderfreibetrag_p32.yaml: Schluessel kinderfreibetrag_je_elternteil.wert ist keine Summe in i64"
    );
}

/// `yaml_dateien` nimmt nur `*.yaml`: eine Datei ohne Endung im Jahresverzeichnis wird uebergangen, auch wenn sie kein YAML ist.
#[test]
fn datei_ohne_endung_im_jahresverzeichnis_wird_uebergangen() {
    let w = Wurzel::neu("notiz");
    std::fs::write(w.pfad("2025/NOTIZ"), "kaputt: [\n").unwrap();
    std::fs::write(w.pfad("kohorten/NOTIZ"), "kaputt: [\n").unwrap();
    let p = w.lade();
    assert_eq!(p.grundfreibetrag(Vz::Vz2025).unwrap(), Euro::new(12_096));
}
