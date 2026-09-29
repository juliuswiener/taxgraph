//! Proptest-Strategien: je Teil-2-Funktion ein Sachverhalt-dict mit realistischen Betraegen,
//! Verlusten (negativ), Zonengrenzen, fehlenden Schluesseln und gelegentlich fremden VZ.
use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use serde_json::{json, Map, Value};

type Feld = BoxedStrategy<Option<(String, Value)>>;

/// Euro-Betrag: meist 0..200.000, dazu Verluste, grosse Werte, Grenzwerte.
fn betrag() -> BoxedStrategy<i64> {
    prop_oneof![
        6 => 0i64..200_000,
        2 => -100_000i64..0,
        1 => 0i64..20_000_000,
        1 => prop::sample::select(vec![
            0, 1, -1, 99, 100, 101, 102, 255, 256, 624, 999, 1000, 1001, 12_096, 12_348,
            13_805, 18_130, 19_950, 20_350, 24_500, 36_260, 39_900, 40_700, 200_000,
            1_000_000, 1_000_001, 3_000_000, 5_000_000,
        ]),
    ]
    .boxed()
}

/// Entfernung in km: ganze Zahlen und halbe km (JSON-Zahl, Python liest `Decimal(str(x))`).
fn km() -> BoxedStrategy<Value> {
    prop_oneof![(0i64..150).prop_map(Value::from), (0i32..300).prop_map(|h| Value::from(f64::from(h) / 2.0))].boxed()
}

fn vz() -> BoxedStrategy<i64> {
    prop_oneof![12 => 2024i64..=2026, 1 => Just(2023i64), 1 => Just(2027i64)].boxed()
}

fn nur_gueltige_vz() -> BoxedStrategy<i64> {
    (2024i64..=2026).boxed()
}

fn feld(k: &'static str, s: BoxedStrategy<Value>, p_da: f64) -> Feld {
    prop::option::weighted(p_da, s).prop_map(move |o| o.map(|v| (k.to_string(), v))).boxed()
}

/// Optionales Feld (`s.get`): 80 % vorhanden.
fn opt<T: Into<Value> + std::fmt::Debug + Clone + 'static>(k: &'static str, s: BoxedStrategy<T>) -> Feld {
    feld(k, s.prop_map(Into::into).boxed(), 0.8)
}

/// Pflichtfeld (`s[k]`): 97 % vorhanden, damit `KeyError` selten, aber vorkommt.
fn req<T: Into<Value> + std::fmt::Debug + Clone + 'static>(k: &'static str, s: BoxedStrategy<T>) -> Feld {
    feld(k, s.prop_map(Into::into).boxed(), 0.97)
}

/// Immer vorhandenes Feld mit zufaelligem Wert (Dispatch-Schluessel).
fn immer<T: Into<Value> + std::fmt::Debug + Clone + 'static>(k: &'static str, s: BoxedStrategy<T>) -> Feld {
    s.prop_map(move |v| Some((k.to_string(), v.into()))).boxed()
}

/// Fest vorhandenes Feld mit festem Wert.
fn fix(k: &'static str, v: Value) -> Feld {
    Just(Some((k.to_string(), v))).boxed()
}

fn bool_() -> BoxedStrategy<bool> {
    any::<bool>().boxed()
}

fn klein(max: i64) -> BoxedStrategy<i64> {
    prop_oneof![8 => 0..=max, 1 => -3i64..0, 1 => (max + 1)..(max + 30)].boxed()
}

fn dict(felder: Vec<Feld>) -> BoxedStrategy<Vec<Value>> {
    felder
        .prop_map(|v| vec![Value::Object(v.into_iter().flatten().collect::<Map<_, _>>())])
        .boxed()
}

fn veranlagung() -> BoxedStrategy<Value> {
    prop_oneof![5 => Just(json!("einzel")), 5 => Just(json!("zusammen")), 1 => Just(json!("getrennt"))].boxed()
}

fn gesamt_felder() -> Vec<Feld> {
    let mut f = vec![
        req("veranlagungszeitraum", vz()),
        fix("gesamtfall", json!(true)),
        feld("veranlagung", veranlagung(), 0.95),
    ];
    for k in [
        "einkuenfte_nichtselbststaendig", "einkuenfte_kapitalvermoegen", "einkuenfte_vermietung",
        "einkuenfte_sonstige", "einkuenfte_gewinn", "altersentlastungsbetrag",
        "entlastungsbetrag_alleinerziehende", "sonderausgaben", "vorsorge_gesamtbeitraege_inkl_ag",
        "vorsorge_ag_anteil_steuerfrei", "aussergewoehnliche_belastungen", "freibetraege_kinder",
        "sonstige_abzuege_vom_einkommen", "anzurechnende_auslaendische_steuern",
        "steuerermaessigungen", "steuer_kapital_gesondert", "hinzurechnung_kindergeld",
        "hinzurechnung_zulage", "tarifliche_est_modifiziert", "versorgung_jahresrente",
        "versorgung_bemessungsgrundlage",
    ] {
        f.push(feld(k, betrag().prop_map(Value::from).boxed(), 0.4));
    }
    f.push(feld("tarif_modifiziert", bool_().prop_map(Value::from).boxed(), 0.15));
    f.push(feld("kinder_ganzjaehrig", klein(4).prop_map(Value::from).boxed(), 0.15));
    f.push(feld("versorgung_beginn_jahr", (1995i64..2065).prop_map(Value::from).boxed(), 0.4));
    f
}

fn gewst_felder() -> Vec<Feld> {
    let mut f = vec![
        req("veranlagungszeitraum", nur_gueltige_vz()),
        feld("gewst_output", prop_oneof![Just(json!("p35_anrechnung")), Just(json!("messbetrag"))].boxed(), 0.7),
        req("gewst_hebesatz", prop_oneof![200i64..=900, Just(0i64)].boxed()),
    ];
    for k in [
        "gewinn_gewerbebetrieb", "gewst_entgelte_schulden", "gewst_renten", "gewst_stille",
        "gewst_miet_beweglich", "gewst_miet_unbeweglich", "gewst_rechte", "gewst_einheitswert",
        "gewst_grundsteuer", "gewst_gewinnanteile_mitunternehmer", "gewst_schachteldividenden",
        "fehlbetrag_bestand",
    ] {
        f.push(feld(k, betrag().prop_map(Value::from).boxed(), 0.5));
    }
    f
}

fn kst_felder() -> Vec<Feld> {
    let mut f = vec![req("gewst_hebesatz", (200i64..=900).boxed())];
    for k in [
        "gewinn_estg", "verdeckte_gewinnausschuettung", "verdeckte_einlage", "personensteuern",
        "geldstrafen", "dividende_bezuege", "veraeusserungsgewinn", "zinsaufwand", "zinsertrag",
        "abschreibungen", "zins_vortrag_bestand", "ebitda_vortrag_bestand",
        "verlustvortrag_bestand", "umsaetze", "loehne_gehaelter", "zuwendungen",
    ] {
        f.push(feld(k, betrag().prop_map(Value::from).boxed(), 0.5));
    }
    f.push(feld("beteiligung_prozent", klein(100).prop_map(Value::from).boxed(), 0.5));
    for k in [
        "keine_konzern_oder_nahestehende_b", "eigenkapital_escape_c", "schaedlicher_erwerb",
        "antrag_8d", "fortfuehrungs_voraussetzungen",
    ] {
        f.push(feld(k, bool_().prop_map(Value::from).boxed(), 0.4));
    }
    f
}

fn rentenart() -> BoxedStrategy<Value> {
    prop::sample::select(vec![
        "gesetzliche_rente", "berufsstaendische_versorgung", "private_basisrente",
        "private_leibrente", "sonstige_leibrente", "betriebsrente_direktzusage",
    ])
    .prop_map(Value::from)
    .boxed()
}

fn fuenftel_felder() -> Vec<Feld> {
    vec![
        req("veranlagungszeitraum", vz()),
        feld("veranlagung", veranlagung(), 0.97),
        req("zu_versteuerndes_einkommen", betrag()),
        immer("ausserordentliche_einkuenfte", betrag()),
    ]
}

/// Strategie fuer die Funktion `name` (Name ohne `catala_`).
pub fn fuer(name: &str) -> BoxedStrategy<Vec<Value>> {
    let b = betrag;
    let felder: Vec<Feld> = match name {
        "gesamt" | "gesamt_gde" | "gesamt_tarifliche" | "gesamt_zve" | "gesamt_kette" => gesamt_felder(),
        "est" => {
            return prop_oneof![
                3 => dict(gesamt_felder()),
                1 => dict(vec![immer("sanierungsaufwendungen", b()), opt("ist_uebernaechstes_foerderjahr", bool_())]),
                1 => dict(vec![immer("bruttolistenpreis", b()), opt("bruchteils_teiler", prop_oneof![Just(1i64), Just(2), Just(4), Just(0), Just(-3)].boxed())]),
                1 => dict({ let mut f = gewst_felder(); f.push(fix("gewerbesteuer", json!(true))); f }),
                1 => dict({ let mut f = kst_felder(); f.push(fix("koerperschaft", json!(true))); f.push(req("veranlagungszeitraum", vz())); f }),
                1 => dict(vec![req("veranlagungszeitraum", vz()), immer("bruttoarbeitslohn_a", b()), opt("bruttoarbeitslohn_b", b()), opt("werbungskosten_a", b()), opt("werbungskosten_b", b()), opt("sonderausgaben_gemeinsam", b())]),
                2 => dict(vec![req("veranlagungszeitraum", vz()), immer("bruttoarbeitslohn", b()), opt("werbungskosten", b()), opt("sonderausgaben", b())]),
                2 => dict(fuenftel_felder()),
                3 => dict(vec![req("veranlagungszeitraum", vz()), feld("veranlagung", veranlagung(), 0.97), req("zu_versteuerndes_einkommen", b())]),
                1 => dict(vec![req("veranlagungszeitraum", vz()), immer("entfernung_km_roh", km()), req("arbeitstage", klein(250)), opt("eigenes_oder_ueberlassenes_kfz", bool_()), opt("oepnv_kosten_jahr", b())]),
                1 => dict(vec![req("veranlagungszeitraum", vz()), immer("arbeitszimmer_vorhanden", bool_()), opt("ist_mittelpunkt", bool_()), opt("tatsaechliche_aufwendungen", b()), opt("jahrespauschale_gewaehlt", bool_()), opt("monate_ohne_mittelpunkt", klein(12)), opt("homeoffice_tage", klein(260))]),
            ]
            .boxed();
        }
        "est_einzel_zve" => vec![req("veranlagungszeitraum", vz()), req("bruttoarbeitslohn", b()), opt("werbungskosten", b()), opt("sonderausgaben", b())],
        "est_zusammen" => vec![req("veranlagungszeitraum", vz()), opt("bruttoarbeitslohn_a", b()), opt("bruttoarbeitslohn_b", b()), opt("werbungskosten_a", b()), opt("werbungskosten_b", b()), opt("sonderausgaben_gemeinsam", b())],
        "fuenftel" => fuenftel_felder(),
        "solz" => vec![req("veranlagungszeitraum", vz()), req("bemessungsgrundlage", b()), opt("kapital_steuer", b()), req("splitting", bool_())],
        "sparer_pb" => vec![req("veranlagungszeitraum", vz()), opt("kapitalertraege", b()), opt("zusammenveranlagung", bool_())],
        "kapital_verrechnung" => vec![opt("gewinn_aktien", b()), opt("verlust_aktien", b()), opt("gewinn_sonstige", b()), opt("verlust_sonstige", b())],
        "kapital_steuer" => vec![req("veranlagungszeitraum", vz()), opt("kapitaleinkuenfte", b()), opt("est_regulaer_mit_kap", b()), opt("est_regulaer_ohne_kap", b())],
        "renten_einkuenfte" => vec![
            req("veranlagungszeitraum", vz()), feld("renten_art", rentenart(), 0.97), opt("jahresrente", b()),
            req("alter_bei_rentenbeginn", klein(97)), req("renten_beginn_jahr", (2020i64..=2028).boxed()),
            feld("rentenfreibetrag", prop_oneof![b().prop_map(Value::from), Just(Value::Null)].boxed(), 0.7),
        ],
        "p19_2_versorgungsfreibetrag" | "einkuenfte_versorgung" => vec![
            opt("versorgung_jahresrente", b()), opt("versorgung_bemessungsgrundlage", b()),
            opt("versorgungsbezuege_bemessungsgrundlage", b()),
            opt("versorgung_beginn_jahr", prop_oneof![1995i64..2065, Just(0i64)].boxed()),
            opt("versorgungsbeginn_jahr", (1995i64..2065).boxed()),
        ],
        "behinderten_pb" => vec![req("veranlagungszeitraum", vz()), opt("ist_hilflos_blind_taubblind", bool_()), opt("grad_der_behinderung", klein(110))],
        "pflege_pb" => vec![req("veranlagungszeitraum", vz()), opt("ist_hilflos", bool_()), opt("pflegegrad", klein(6))],
        "hinterbliebenen_pb" => vec![req("veranlagungszeitraum", vz()), opt("hat_hinterbliebenenbezuege", bool_())],
        "p33_2a_fahrtkostenpauschale" => vec![req("veranlagungszeitraum", vz()), opt("hat_ag_bl_tbl_h", bool_()), opt("hat_gdb80_oder_70g", bool_())],
        "p33a_unterhalt" => vec![req("veranlagungszeitraum", vz()), req("aufwendungen", b()), opt("kv_pv_beitraege", b()), opt("andere_einkuenfte_bezuege", b())],
        "p33a_ausbildungsfreibetrag" => vec![opt("anzahl_kinder", klein(5))],
        "p10_1_5_kinderbetreuung" => vec![opt("veranlagungszeitraum", vz()), opt("aufwendungen", b())],
        "p10_1_9_schulgeld" => vec![opt("veranlagungszeitraum", vz()), opt("aufwendungen", b()), opt("splitting", bool_())],
        "p10_1a_realsplitting" => vec![opt("unterhaltsleistungen", b()), opt("kv_pv_beitraege", b()), opt("kv_krankengeld", b())],
        "p32b_1" => vec![req("zu_versteuerndes_einkommen", b()), req("progressionseinkuenfte", b()), req("est_auf_erhoehte_bemessung", b())],
        "gewst" => gewst_felder(),
        "kst_nenner_b" => kst_felder(),
        "p10d_2" => vec![opt("gesamtbetrag_einkuenfte", b()), opt("verlustvortrag_bestand", b()), opt("zusammenveranlagung", bool_())],
        "p23_veraeusserungsgewinn" => vec![req("veraeusserungspreis", b()), req("anschaffungs_herstellungskosten", b()), req("werbungskosten", b())],
        "p23_freigrenze" => vec![req("gesamtgewinn", b())],
        "p23_verlusttopf" => vec![req("gewinn_pvg", b()), req("verlust_pvg", b())],
        "p34c_1" => vec![req("gezahlte_auslaendische_steuer", b()), req("deutsche_est_inkl_ausl", b()), req("zu_versteuerndes_einkommen", b()), req("auslaendische_einkuenfte_staat", b())],
        "ermaessigter_durchschnittssatz" => vec![opt("ao_einkuenfte", b()), opt("est_gesamt_zzgl_progression", b()), opt("bemessungsgrundlage_durchschnitt", b())],
        "p35c_sanierung" => vec![opt("sanierungsaufwendungen", b()), opt("ist_uebernaechstes_foerderjahr", bool_())],
        "p35c_energieberater" => vec![opt("energieberater_aufwendungen", b())],
        "p35c_jahresdeckel" => vec![opt("sanierung_ermaessigung", b()), opt("energieberater_ermaessigung", b()), opt("ist_uebernaechstes_foerderjahr", bool_())],
        "p22_nr3_einkuenfte" => return prop_oneof![b().prop_map(|x| x * 100), -1_000i64..60_000].prop_map(|x| vec![json!(x)]).boxed(),
        andere => panic!("keine Strategie fuer {andere}"),
    };
    dict(felder)
}
