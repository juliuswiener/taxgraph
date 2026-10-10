//! Spike AK0 des Privatverkaufs (Backlog `ehrlich-erklaerter-privatverkauf-verhindert-die-abgabe`): welche Kz verlangt die amtliche
//! Pruefung (`checkESt`) fuer die Anlage SO, Block Grundstueck (`Grdst`) und Block anderes Wirtschaftsgut (`And_WG`)?
//!
//! Der Code kennt heute nur den Gewinn je Verkauf (`E0306801`, `E0307701`). Bis zu zehn weitere Kz je Verkauf (Daten, Preis, Kosten,
//! `AfA`, Eigennutzung) schreibt er nicht. Dieser Test setzt die Kz je Verkauf von Hand in eine saubere Deklaration (Basis der
//! Angestellten, wie in `checkest_blockmatrix.rs`), baut das XML ueber `erzeuge_xml` (abgabefaehig, Snapshot) und laesst `ERiC`
//! urteilen (`validiere`, nur `ERIC_VALIDIERE`, kein Versand). Die Verkaeufe gehen als Instanzen der Gruppe `p23_veraeusserung` hinein:
//! der Schreiber legt den Pfad aus dem amtlichen Schema, nicht aus einer Tabelle des Produkts. Kein Produktcode wird geaendert.
//!
//! WAS GEMESSEN WIRD (nichts davon steht in der XSD als Pflicht, `minOccurs` ist dort ueberall 0):
//! - Gewinn allein, Gewinn mit Daten/Preis/Kosten, Eigennutzung, `AfA`, andere Zwecke (Grundstueck); Gewinn allein und voll (`And_WG`).
//! - Auslassung: je Block jedes Kz des vollen Satzes einzeln weggelassen (`L1_*`).
//! - Abhaengigkeit: der Gewinn allein plus je ein weiteres Kz (`Z1_*`).
//! - Inhalt: Frist abgelaufen, Veraeusserung im Vorjahr, Veraeusserung vor Anschaffung, Gewinn rechnet nicht, Verlust, Gewinn 0.
//! - Mehrere Verkaeufe in einer Erklaerung (zwei Grundstuecke, ein Wirtschaftsgut).
//!
//! KONTROLLEN. Ein gruener Lauf ohne `ERiC` saehe gleich aus. Darum: die Basis allein muss `rc=0` sein, und zwei Akten mit einem
//! Formfehler im SO-Block (Datum als `2025-06-15`, Gewinn als Text) muessen von `ERiC` beanstandet werden. Nur so ist bewiesen, dass
//! der SO-Block ueberhaupt gelesen wird. Jedes eingesetzte Kz muss ausserdem im XML stehen, sonst haette der Schreiber es verschluckt
//! und der Fall mass etwas anderes. Alle anderen Faelle werden NICHT bewertet, nur gedruckt: sie sind das Messergebnis.
//!
//! KEIN SKIP. Der Test ist `#[ignore]` (`ERiC` und die Hersteller-ID gehoeren nicht auf einen Runner, Entscheid Julius 2026-09-12).
//! Mit `--ignored` ist jede fehlende Voraussetzung ein `panic`. Aufruf (die ID kommt aus dem Prozess-Env, `mit_id.sh` liest sie aus
//! der gitignorierten `.env`): `mit_id.sh cargo test -p bescheid --test checkest_privatverkauf_spike -- --ignored --nocapture`.
//!
//! KEIN GEHEIMNIS IN DER AUSGABE. Gedruckt werden `rc`, Klasse und die `<Text>`-Meldungen, die ID darin ersetzt durch `<ID>`; nie das
//! XML, nie der Rumpf der `ERiC`-Antwort, nie die ID.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types,
    clippy::too_many_lines
)]

use std::collections::{BTreeMap, HashMap};

use bescheid::deklaration::mit_ring_werten;
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Vz, Zustand};
use elster::testhilfe::schemas_da;
use elster::{
    deklariere, erzeuge_xml, klassifiziere_rc, validiere, AnlageInstanz, Deklaration, Felder, XmlOptionen,
};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

/// `ERiC` antwortet auf die Platzhalter-ID mit `rc=610301202` und prueft nichts.
const PLATZHALTER_ID: &str = "74931";
/// Die Gruppe, in der der Schreiber die Verkaeufe der Anlage SO ablegt (`INSTANZ_CONTAINER_TIEFER` in `elster/src/xml.rs`).
const GRUPPE: &str = "p23_veraeusserung";

/// Ein Kz mit Wert (Kz-Werte der Deklaration sind EURO, Daten `TT.MM.JJJJ`, Ankreuzfelder `true`).
type Kz = (&'static str, Value);

fn hersteller_id() -> String {
    let id = std::env::var("ELSTER_HERSTELLER_ID").unwrap_or_default();
    let id = id.trim();
    assert!(!id.is_empty(), "ID fehlt: ELSTER_HERSTELLER_ID ist nicht gesetzt (mit_id.sh liest sie aus der .env)");
    assert!(
        id != PLATZHALTER_ID && !id.chars().all(|c| c == '0'),
        "ID ist der gesperrte Platzhalter: ERiC gaebe rc=610301202 und prueft nichts"
    );
    id.to_owned()
}

fn voraussetzungen() -> String {
    assert!(
        elster::find_eric_lib().is_some(),
        "Bibliothek fehlt: libericapi.so liegt weder unter $ERIC_DIR noch unter ~/02_Software/eric"
    );
    assert!(schemas_da(2025), "ERiC-Schema fuer VZ 2025 fehlt: ohne es baut der Writer kein XML");
    hersteller_id()
}

fn setze(s: &mut Store, feld: &str, wert: Value) {
    let leer = HashMap::new();
    s.append_roh(
        &NeuesEventRoh {
            feld_id: feld.to_owned(),
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: HerkunftVektor::Voll(Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: Some(format!("ok@{feld}")),
                signal_2_fehlt: false,
            },
            signal_2_fremd: None,
            ersetzt: None,
            ts: Some("2026-09-01T00:00:00+00:00".to_owned()),
        },
        None,
        BindungNachschlag::neu(&leer),
    )
    .unwrap();
}

/// Kegel einer vollstaendigen `gesamt`-Erklaerung der Angestellten wie `basis` in `checkest_blockmatrix.rs`, Cent. Allein ist sie `rc=0`.
fn basis() -> Vec<(&'static str, Value)> {
    vec![
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("vor_an_anteil_rv", json!(4_200_000)),
        ("vor_ag_anteil_rv", json!(1_200_000)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
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
        ("veranlagung", json!("einzel")),
    ]
}

/// Die Deklaration der Basis, so wie die Produktion sie baut: Ring, dann `deklariere`. Sie sperrt nicht und hat keinen Verkauf.
fn sauber() -> (Deklaration, Felder) {
    let mut s = Store::aus_datei(
        serde_json::from_value(json!({"version": 1, "veranlagungszeitraum": 2025, "scheibe": "gesamt", "events": []})).unwrap(),
    );
    for (f, w) in basis() {
        setze(&mut s, f, w);
    }
    let (mut felder, _) = s.materialisiere(None).unwrap();
    mit_ring_werten(&mut felder, Some(Vz::Vz2025), params()).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    assert_eq!(d.unvollstaendig().len(), 0, "die Basis sperrt schon: {:?}", d.unvollstaendig());
    (d, felder)
}

/// Die `<Text>`-Meldungen aus der `ERiC`-Antwort, Leerraum zusammengezogen, die ID ersetzt.
fn meldungen(antwort: &str, id: &str) -> Vec<String> {
    let mut aus = Vec::new();
    let mut rest = antwort;
    while let Some(von) = rest.find("<Text>") {
        let nach = &rest[von + "<Text>".len()..];
        let Some(bis) = nach.find("</Text>") else { break };
        let t: String = nach[..bis].split_whitespace().collect::<Vec<_>>().join(" ");
        aus.push(t.replace(id, "<ID>"));
        rest = &nach[bis + "</Text>".len()..];
    }
    aus
}

struct Urteil {
    rc: i32,
    meldungen: Vec<String>,
}

const RC_PLAUSIBILITAET: i32 = 610_001_002;

impl Urteil {
    fn plausibel(&self) -> bool {
        self.rc == 0 && self.meldungen.is_empty()
    }
}

/// Alles, was ein Fall braucht: die saubere Deklaration, ihr Snapshot, die ID und die gesammelten Ergebniszeilen.
struct Messplatz {
    d: Deklaration,
    felder: Felder,
    id: String,
    zeilen: Vec<String>,
}

impl Messplatz {
    /// Die Verkaeufe als Instanzen 1, 2, ... der Gruppe `p23_veraeusserung`, dann `erzeuge_xml` (abgabefaehig) und `ERiC`.
    /// Jedes eingesetzte Kz muss im XML stehen; sonst panikt der Fall (der Schreiber haette es verschluckt).
    fn messe(&mut self, name: &str, verkaeufe: &[Vec<Kz>]) -> Urteil {
        let mut d = self.d.clone();
        d.anlage_instanzen = vec![(
            GRUPPE.to_owned(),
            verkaeufe
                .iter()
                .enumerate()
                .map(|(i, kz)| AnlageInstanz {
                    index: u64::try_from(i + 1).unwrap(),
                    felder: kz.iter().map(|(k, w)| ((*k).to_owned(), w.clone())).collect::<BTreeMap<_, _>>(),
                    dokumentiert: BTreeMap::new(),
                })
                .collect(),
        )];
        let xml = erzeuge_xml(
            &d,
            &XmlOptionen {
                hersteller_id: Some(self.id.clone()),
                abgabefaehig: true,
                snapshot: Some(&self.felder),
                ..XmlOptionen::default()
            },
        )
        .unwrap_or_else(|e| panic!("{name}: erzeuge_xml scheitert: {}", e.to_string().replace(&self.id, "<ID>")));
        for kz in verkaeufe.iter().flatten().map(|(k, _)| k) {
            assert!(xml.contains(&format!("<{kz}>")), "{name}: {kz} steht nicht im XML (der Schreiber hat es verschluckt)");
        }
        let (rc, antwort) = validiere(xml.as_bytes(), "ESt_2025")
            .unwrap_or_else(|e| panic!("{name}: ERiC laedt nicht oder bricht ab: {e:?}"));
        let u = Urteil { rc, meldungen: meldungen(&antwort, &self.id) };
        let kz: Vec<String> = verkaeufe
            .iter()
            .map(|v| v.iter().map(|(k, _)| k.get(2..).unwrap_or("?")).collect::<Vec<_>>().join(","))
            .collect();
        let texte = u.meldungen.iter().take(4).map(|m| m.chars().take(600).collect::<String>()).collect::<Vec<_>>().join(" / ");
        let zeile = format!(
            "{name:<44} rc={:<10} {:?} n={} grdst={} and_wg={} einz={} kz={} | {texte}",
            u.rc,
            klassifiziere_rc(i64::from(u.rc)),
            u.meldungen.len(),
            xml.matches("<Grdst>").count(),
            xml.matches("<And_WG>").count(),
            xml.matches("<Einz>").count(),
            kz.join(" ; ")
        );
        eprintln!("{zeile}");
        self.zeilen.push(zeile);
        u
    }
}

/// Die Kz aus `muster`, deren Name in `namen` steht, in der Reihenfolge des Musters.
fn waehle(muster: &[Kz], namen: &[&str]) -> Vec<Kz> {
    muster.iter().filter(|(k, _)| namen.contains(k)).cloned().collect()
}

fn ohne(v: &[Kz], kz: &str) -> Vec<Kz> {
    v.iter().filter(|(k, _)| *k != kz).cloned().collect()
}

/// Wie `v`, aber `kz` hat `wert` (steht `kz` nicht darin, kommt es dazu).
fn mit(v: &[Kz], kz: &'static str, wert: Value) -> Vec<Kz> {
    let mut aus = ohne(v, kz);
    aus.push((kz, wert));
    aus
}

/// Jedes Kz des Blocks `Grdst/Einz` mit einem Beispielwert. Preis 200.000, Anschaffung 150.000, Werbungskosten 5.000, Gewinn 45.000.
fn grdst_muster() -> Vec<Kz> {
    vec![
        ("E0306111", json!("Musterstr. 1, 55555 Musterort")),
        ("E0306201", json!("01.03.2020")),
        ("E0306202", json!("15.06.2025")),
        ("E0306301", json!(true)),
        ("E0306302", json!("01.03.2020-15.06.2025")),
        ("E0306303", json!(120)),
        ("E0306304", json!(true)),
        ("E0306305", json!("01.03.2020-15.06.2025")),
        ("E0306306", json!(80)),
        ("E0306401", json!(200_000)),
        ("E0306501", json!(150_000)),
        ("E0306601", json!(10_000)),
        ("E0306701", json!(5_000)),
        ("E0306801", json!(45_000)),
    ]
}

/// Jedes Kz des Blocks `And_WG/Einz` mit einem Beispielwert. Preis 12.000, Anschaffung 10.000, Werbungskosten 100, Gewinn 1.900.
fn wg_muster() -> Vec<Kz> {
    vec![
        ("E0307101", json!("Goldmuenzen")),
        ("E0307201", json!("01.03.2025")),
        ("E0307202", json!("15.06.2025")),
        ("E0307401", json!(12_000)),
        ("E0307501", json!(10_000)),
        ("E0307601", json!(100)),
        ("E0307701", json!(1_900)),
    ]
}

const GRDST_GEWINN: &str = "E0306801";
const GRDST_VOLL: [&str; 7] = ["E0306111", "E0306201", "E0306202", "E0306401", "E0306501", "E0306701", GRDST_GEWINN];
const WG_GEWINN: &str = "E0307701";
const WG_VOLL: [&str; 7] = ["E0307101", "E0307201", "E0307202", "E0307401", "E0307501", "E0307601", WG_GEWINN];

/// Auslassung und Abhaengigkeit eines Blocks: jedes Kz des vollen Satzes einzeln weg (`L1_`), der Gewinn allein plus je ein weiteres
/// Kz des Musters (`Z1_`).
fn auslassung_und_abhaengigkeit(m: &mut Messplatz, praefix: &str, muster: &[Kz], voll: &[&str], gewinn: &str) {
    let basis = waehle(muster, voll);
    for kz in voll {
        m.messe(&format!("L1_{praefix}_ohne_{kz}"), &[ohne(&basis, kz)]);
    }
    let nur_gewinn = waehle(muster, &[gewinn]);
    for (kz, wert) in muster.iter().filter(|(k, _)| *k != gewinn) {
        let mut v = nur_gewinn.clone();
        v.push((*kz, wert.clone()));
        m.messe(&format!("Z1_{praefix}_gewinn_plus_{kz}"), &[v]);
    }
}

/// Die Anlage SO gegen das echte `checkESt`: Pflicht-Kz-Liste fuer Grundstueck und anderes Wirtschaftsgut.
#[test]
#[ignore = "braucht die ERiC-Bibliothek und die registrierte Hersteller-ID (Umgebungs-Gate, Entscheid Julius 2026-09-12): lokal mit mit_id.sh (Spike AK0)"]
fn privatverkauf_kz_pflichtliste_gegen_echtes_checkest() {
    let id = voraussetzungen();
    let (dekl, felder) = sauber();
    let mut platz = Messplatz { d: dekl, felder, id, zeilen: Vec::new() };
    let gm = grdst_muster();
    let wm = wg_muster();

    // --- Kontrollen: ERiC wurde erreicht, liest den SO-Block und beanstandet, was es beanstanden muss.
    let k0 = platz.messe("K0_nur_basis", &[]);
    assert!(k0.plausibel(), "K0: die Basis allein ist nicht rc=0 (ID gesperrt oder ERiC defekt?): rc={} {:?}", k0.rc, k0.meldungen);
    for (name, v) in [
        ("K1_grdst_datum_als_iso", mit(&waehle(&gm, &GRDST_VOLL), "E0306202", json!("2025-06-15"))),
        ("K2_grdst_gewinn_als_text", mit(&waehle(&gm, &GRDST_VOLL), GRDST_GEWINN, json!("abc"))),
        ("K3_wg_datum_als_iso", mit(&waehle(&wm, &WG_VOLL), "E0307202", json!("2025-06-15"))),
        ("K4_grdst_datum_fehlt", ohne(&waehle(&gm, &GRDST_VOLL), "E0306201")),
    ] {
        let u = platz.messe(name, &[v]);
        assert!(
            u.rc == RC_PLAUSIBILITAET && !u.meldungen.is_empty(),
            "{name}: ERiC muss den Fehler im SO-Block mit rc={RC_PLAUSIBILITAET} beanstanden, gab rc={} n={}",
            u.rc,
            u.meldungen.len()
        );
    }

    // --- Grundstueck: die Stufen aus dem Backlog.
    let voll = waehle(&gm, &GRDST_VOLL);
    platz.messe("Ga_gewinn_allein", &[waehle(&gm, &[GRDST_GEWINN])]);
    platz.messe("Gb_voll_daten_preis_kosten", std::slice::from_ref(&voll));
    platz.messe("Gb2_voll_ohne_bezeichnung", &[ohne(&voll, "E0306111")]);
    platz.messe("Gc1_eigennutzung_mit_gewinn", &[mit(&voll, "E0306301", json!(true))]);
    platz.messe("Gc2_eigennutzung_mit_zeitraum", &[waehle(&gm, &[&GRDST_VOLL[..], &["E0306301", "E0306302"]].concat())]);
    platz.messe("Gc3_eigennutzung_zeitraum_und_qm", &[waehle(&gm, &[&GRDST_VOLL[..], &["E0306301", "E0306302", "E0306303"]].concat())]);
    platz.messe("Gc4_eigennutzung_ohne_gewinn", &[mit(&ohne(&voll, GRDST_GEWINN), "E0306301", json!(true))]);
    platz.messe("Gc5_eigennutzung_gewinn_null", &[mit(&mit(&voll, "E0306301", json!(true)), GRDST_GEWINN, json!(0))]);
    platz.messe("Gc6_eigennutzung_nur_haken_und_daten", &[waehle(&gm, &["E0306201", "E0306202", "E0306301"])]);
    platz.messe(
        "Gd_teilnutzung_eigen_und_andere",
        &[waehle(&gm, &[&GRDST_VOLL[..], &["E0306301", "E0306302", "E0306303", "E0306304", "E0306305", "E0306306"]].concat())],
    );
    platz.messe("Ge1_afa_gewinn_preis_minus_ak_plus_afa", &[mit(&mit(&voll, "E0306601", json!(10_000)), GRDST_GEWINN, json!(55_000))]);
    platz.messe("Ge2_afa_gewinn_wie_ohne_afa", &[mit(&voll, "E0306601", json!(10_000))]);
    platz.messe("Gf_frist_abgelaufen_15_jahre", &[mit(&voll, "E0306201", json!("01.03.2010"))]);
    platz.messe("Gg_veraeusserung_im_vorjahr", &[mit(&voll, "E0306202", json!("15.06.2024"))]);
    platz.messe("Gh_veraeusserung_vor_anschaffung", &[mit(&voll, "E0306202", json!("01.01.2019"))]);
    platz.messe("Gi_gewinn_rechnet_nicht", &[mit(&voll, GRDST_GEWINN, json!(99_999))]);
    platz.messe("Gj_verlust", &[mit(&mit(&voll, "E0306401", json!(100_000)), GRDST_GEWINN, json!(-55_000))]);
    platz.messe("Gk_gewinn_null", &[mit(&voll, GRDST_GEWINN, json!(0))]);
    // Die Einzelbetraege (Preis, Anschaffungskosten, AfA, Werbungskosten) sind nicht jeder fuer sich Pflicht, aber sobald einer da ist,
    // rechnet checkESt den Gewinn nach. Darum je Fall der Gewinn neu gerechnet: Preis - Anschaffungskosten + AfA - Werbungskosten.
    let pflicht4 = waehle(&gm, &["E0306111", "E0306201", "E0306202"]);
    let plus = |extra: &[Kz]| -> Vec<Kz> { pflicht4.iter().cloned().chain(extra.iter().cloned()).collect() };
    platz.messe("Gx1_vier_pflicht_ohne_betraege", &[plus(&[("E0306801", json!(45_000))])]);
    platz.messe("Gx2_ohne_werbungskosten_gewinn_neu", &[plus(&[("E0306401", json!(200_000)), ("E0306501", json!(150_000)), ("E0306801", json!(50_000))])]);
    platz.messe("Gx3_ohne_anschaffungskosten_gewinn_neu", &[plus(&[("E0306401", json!(200_000)), ("E0306701", json!(5_000)), ("E0306801", json!(195_000))])]);
    platz.messe("Gx4_ohne_preis_gewinn_neu", &[plus(&[("E0306501", json!(150_000)), ("E0306701", json!(5_000)), ("E0306801", json!(-155_000))])]);
    platz.messe("Gx5_nur_preis_und_gewinn", &[plus(&[("E0306401", json!(200_000)), ("E0306801", json!(200_000))])]);
    platz.messe("Gx6_gewinn_null_ohne_betraege", &[plus(&[("E0306801", json!(0))])]);
    let eigen = [("E0306301", json!(true)), ("E0306302", json!("01.03.2020-15.06.2025")), ("E0306303", json!(120))];
    platz.messe("Gx7_eigennutzung_gewinn_null_ohne_betraege", &[plus(&[eigen[0].clone(), eigen[1].clone(), eigen[2].clone(), ("E0306801", json!(0))])]);
    platz.messe("Gx8_eigennutzung_gewinn_ohne_betraege", &[plus(&[eigen[0].clone(), eigen[1].clone(), eigen[2].clone(), ("E0306801", json!(45_000))])]);
    platz.messe("Gx9_eigennutzung_ohne_gewinn_ohne_betraege", &[plus(&[eigen[0].clone(), eigen[1].clone(), eigen[2].clone()])]);
    platz.messe(
        "Gx10_andere_zwecke_voll",
        &[waehle(&gm, &[&GRDST_VOLL[..], &["E0306304", "E0306305", "E0306306"]].concat())],
    );
    platz.messe(
        "Gx11_afa_ohne_preis_und_ak",
        &[plus(&[("E0306601", json!(10_000)), ("E0306701", json!(5_000)), ("E0306801", json!(5_000))])],
    );
    auslassung_und_abhaengigkeit(&mut platz, "G", &gm, &GRDST_VOLL, GRDST_GEWINN);

    // --- Anderes Wirtschaftsgut. Das Schema kennt hier kein AfA-Kz: die AfA steckt in E0307501 ("ggf. gemindert um AfA").
    let voll = waehle(&wm, &WG_VOLL);
    platz.messe("Wa_gewinn_allein", &[waehle(&wm, &[WG_GEWINN])]);
    platz.messe("Wb_voll_art_daten_preis_kosten", std::slice::from_ref(&voll));
    platz.messe("Wc_ohne_art", &[ohne(&voll, "E0307101")]);
    platz.messe("Wd_frist_ueber_ein_jahr", &[mit(&voll, "E0307201", json!("01.03.2023"))]);
    platz.messe("We_veraeusserung_im_vorjahr", &[mit(&voll, "E0307202", json!("15.06.2024"))]);
    platz.messe("Wf_veraeusserung_vor_anschaffung", &[mit(&voll, "E0307202", json!("01.01.2025"))]);
    platz.messe("Wg_gewinn_rechnet_nicht", &[mit(&voll, WG_GEWINN, json!(99_999))]);
    platz.messe("Wh_verlust", &[mit(&mit(&voll, "E0307401", json!(8_000)), WG_GEWINN, json!(-2_100))]);
    platz.messe("Wi_gewinn_null", &[mit(&voll, WG_GEWINN, json!(0))]);
    platz.messe("Wx1_art_und_gewinn", &[waehle(&wm, &["E0307101", WG_GEWINN])]);
    platz.messe("Wx2_art_daten_und_gewinn", &[waehle(&wm, &["E0307101", "E0307201", "E0307202", WG_GEWINN])]);
    platz.messe("Wx3_ohne_werbungskosten_gewinn_neu", &[mit(&ohne(&voll, "E0307601"), WG_GEWINN, json!(2_000))]);
    platz.messe("Wx4_art_gewinn_null_ohne_betraege", &[mit(&waehle(&wm, &["E0307101"]), WG_GEWINN, json!(0))]);
    platz.messe("Wx5_nur_preis_und_gewinn", &[mit(&waehle(&wm, &["E0307101", "E0307401"]), WG_GEWINN, json!(12_000))]);
    auslassung_und_abhaengigkeit(&mut platz, "W", &wm, &WG_VOLL, WG_GEWINN);

    // --- Mehrere Verkaeufe in einer Erklaerung: zwei Grundstuecke und ein Wirtschaftsgut (Instanzen 1 bis 3 der Gruppe).
    let g_voll = waehle(&gm, &GRDST_VOLL);
    let g_zwei = mit(&mit(&g_voll, "E0306401", json!(300_000)), GRDST_GEWINN, json!(145_000));
    platz.messe("M1_zwei_grundstuecke", &[g_voll.clone(), g_zwei.clone()]);
    platz.messe("M2_zwei_grundstuecke_ein_wg", &[g_voll, g_zwei, waehle(&wm, &WG_VOLL)]);
    let wg_zwei = mit(&mit(&waehle(&wm, &WG_VOLL), "E0307401", json!(15_000)), WG_GEWINN, json!(4_900));
    platz.messe("M3_zwei_wg", &[waehle(&wm, &WG_VOLL), wg_zwei]);

    eprintln!("--- {} Faelle gemessen ---", platz.zeilen.len());
}
