//! Luecken in den Sperren von `deklaration/sperre/*.rs`, die ein `cargo-mutants`-Lauf am 2026-10-24 (Basis `0cb6eccd`,
//! `arch-think`) gefunden hat: Mutanten, die auch alle Tests von `bescheid` und `api` (ohne Sweep und Browser) bestanden.
//! Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Der Guard `an_gesamt_sperrgrund` haelt eine Zahl zurueck, solange eine Angabe fehlt oder nur vorlaeufig
//! ist. Die Tabellen in `sperre_scheiben_hermetisch.rs` pruefen jede Sperre mit "alles fehlt" und "alles da". Die Mutanten
//! fielen durch die Mitte: nur EINE von zwei Angaben beantwortet, eine Angabe nur vorlaeufig, ein Wert auf der Grenze, ein
//! Bool oder eine Liste statt einer Zahl.
//!
//! **Warum es zaehlt.** Faellt eine dieser Sperren weg, rechnet die Scheibe still weiter: mit einem Freibetrag (§ 16 Abs. 4),
//! einer Verpflegungspauschale ohne Mahlzeitenkuerzung oder einer Uebernachtung ohne Zeitraum. Das Ergebnis waere zu
//! niedrig in der Steuer, und kein Test faende es.
//!
//! **Wo es sitzt.** `deklaration/sperre/werbungskosten.rs` (dHf, Verpflegung, Uebernachtung, Arbeitsmittel) und
//! `deklaration/sperre/einkunft.rs` (`p16_4`, `realsplitting_fahrtkosten`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: aus den Regeln, die die Guards selbst in ihren Kommentaren nennen ("BESTAETIGT", "ein
//! vorlaeufiger Wert urteilt nicht", "Naht-Fix: nicht nur is True auf dem Rohwert", Python `sum(wert or 0)` mit `TypeError`).
//! Kein Wert ist aus dem Rust-Code abgelesen; eine Rechnung aus dem Gesetz gibt es fuer eine Sperre nicht (DASS eine
//! unbeantwortete Frage sperrt, ist Entwurf des Projekts). Die Tabelle haelt nur den bestehenden Stand fest.
//!
//! Jede Zeile hat eine Gegenprobe, die NICHT sperrt (Kontrollfaelle): ohne sie bestuende auch ein Guard, der immer denselben
//! Grund liefert, jeden Fall.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::sync::OnceLock;

use bescheid::deklaration::{an_gesamt_sperrgrund, Cfg};
use bescheid::testhilfe::{felder, index, store};
use bescheid::{BescheidFehler, BindungIndex, Instanzquelle};
use domain::{Scheibe, Sperrgrund, Vz};
use serde_json::{json, Value};

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

/// Ein Ja/Nein-Feld; `bestaetigt = false` ist ein vorlaeufiger Wert.
fn b(fid: &'static str, wert: bool, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein Zahlfeld (Cent, Jahr oder Tage); `bestaetigt = false` ist ein vorlaeufiger Wert.
fn z(fid: &'static str, wert: i64, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein Feld mit beliebigem JSON-Wert.
fn w(fid: &'static str, wert: Value, bestaetigt: bool) -> Ev {
    (fid, wert, bestaetigt)
}

/// Ein handgebauter Fall: Events und der erwartete Grund.
struct Fall {
    name: &'static str,
    events: Vec<Ev>,
    erwartet: Option<&'static str>,
}

fn fall(name: &'static str, events: Vec<Ev>, erwartet: Option<&'static str>) -> Fall {
    Fall { name, events, erwartet }
}

/// Die Bindung der Scheibe: nur ihre Feld-Ids, wie `api._scheibe_bindung`.
fn scheiben_index(scheibe: Scheibe) -> &'static BindungIndex<'static> {
    static GESAMT: OnceLock<BindungIndex<'static>> = OnceLock::new();
    static RENTNER: OnceLock<BindungIndex<'static>> = OnceLock::new();
    let zelle = match scheibe {
        Scheibe::RentnerGesamt => &RENTNER,
        _ => &GESAMT,
    };
    zelle.get_or_init(|| {
        let ids = Cfg::fuer(scheibe).felder(|_| Vec::new()).unwrap();
        index()
            .iter()
            .filter(|(k, _)| ids.contains(k))
            .map(|(k, b)| (k.clone(), *b))
            .collect()
    })
}

/// Der Guard auf der Scheibe fuer VZ 2025 (Store UND Scheiben-Bindung, `nur_bestaetigt = false` wie in `einreichen`).
fn guard(scheibe: Scheibe, events: &[Ev]) -> Result<Option<Sperrgrund>, BescheidFehler> {
    let st = store(events);
    let f = felder(&st);
    let q = Instanzquelle { store: Some(&st), bindung: Some(scheiben_index(scheibe)), nur_bestaetigt: false };
    let cfg = Cfg::fuer(scheibe);
    an_gesamt_sperrgrund(&f, Some(&cfg), Some(Vz::Vz2025), &q)
}

/// Alle Faelle durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn pruefe(scheibe: Scheibe, faelle: Vec<Fall>) {
    assert!(!faelle.is_empty());
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|f| {
            let got = guard(scheibe, &f.events).unwrap().map(Sperrgrund::als_str);
            (got != f.erwartet).then(|| format!("{}: Rust {got:?}, erwartet {:?}", f.name, f.erwartet))
        })
        .collect();
    assert!(abweichend.is_empty(), "{} von {} Faellen weichen ab: {abweichend:#?}", abweichend.len(), faelle.len());
}

// ---------------------------------------------------------------- dHf, Verpflegung, Uebernachtung, Arbeitsmittel

/// Doppelte Haushaltsfuehrung: `dhf_im_inland` = ja zaehlt nur BESTAETIGT ("Naht-Fix: nicht nur is True auf dem Rohwert"). Ein
/// vorlaeufiges ja neben bestaetigten Bedingungen sperrt weiter; mit bestaetigtem ja ist die Gegenprobe frei.
#[test]
fn dhf_inland_ja_zaehlt_nur_bestaetigt() {
    let rest = |inland_bestaetigt: bool| {
        vec![
            z("dhf_unterkunftskosten_monat", 80_000, true),
            b("dhf_im_inland", true, inland_bestaetigt),
            b("dhf_beruflich_veranlasst", true, true),
            b("dhf_eigener_hausstand", true, true),
            b("dhf_finanzielle_beteiligung", true, true),
        ]
    };
    pruefe(
        Scheibe::RentnerGesamt,
        vec![
            fall("dHf, Inland ja nur vorlaeufig, Bedingungen bestaetigt", rest(false), Some("dhf_tatbestand_offen")),
            fall("dHf, Inland ja bestaetigt, Bedingungen bestaetigt (Gegenprobe)", rest(true), None),
        ],
    );
}

/// Verpflegung: die Tage einer Kategorie zaehlen nach Pythons `sum(wert or 0)`: auch vorlaeufige, `True` als 1. Ein bestaetigtes
/// `vpf_monate_am_ort` ist Pflicht (ein vorlaeufiges zaehlt nicht), und die Mahlzeitenfrage ist nur mit bestaetigtem "ja, keine
/// Mahlzeit gestellt" oder bestaetigten Anzahlen beantwortet.
#[test]
fn verpflegung_tage_monate_und_mahlzeiten() {
    let tage = || z("tage_24h", 5, true);
    pruefe(
        Scheibe::RentnerGesamt,
        vec![
            fall(
                "Tage nur vorlaeufig: die Summe zaehlt sie trotzdem, ohne Monate am Ort sperrt es",
                vec![z("tage_24h", 5, false)],
                Some("verpflegung_dreimonatsfrist_aufteilung_offen"),
            ),
            fall(
                "Tage als Bool true: Python zaehlt True als 1",
                vec![w("tage_24h", json!(true), true)],
                Some("verpflegung_dreimonatsfrist_aufteilung_offen"),
            ),
            fall("Tage als Bool false: Summe 0, keine Sperre (Gegenprobe)", vec![w("tage_24h", json!(false), true)], None),
            fall(
                "Monate am Ort 2 nur vorlaeufig",
                vec![tage(), z("vpf_monate_am_ort", 2, false), b("vpf_keine_mahlzeitengestellung", true, true)],
                Some("verpflegung_dreimonatsfrist_aufteilung_offen"),
            ),
            fall(
                "Monate am Ort 2 bestaetigt (Gegenprobe)",
                vec![tage(), z("vpf_monate_am_ort", 2, true), b("vpf_keine_mahlzeitengestellung", true, true)],
                None,
            ),
            fall(
                "keine Mahlzeitengestellung: nein bestaetigt (es wurden Mahlzeiten gestellt), keine Anzahl",
                vec![tage(), z("vpf_monate_am_ort", 2, true), b("vpf_keine_mahlzeitengestellung", false, true)],
                Some("verpflegung_reduktion_offen"),
            ),
            fall(
                "keine Mahlzeitengestellung: ja nur vorlaeufig",
                vec![tage(), z("vpf_monate_am_ort", 2, true), b("vpf_keine_mahlzeitengestellung", true, false)],
                Some("verpflegung_reduktion_offen"),
            ),
            fall(
                "Anzahl der gestellten Fruehstuecke bestaetigt statt des Kreuzes (Gegenprobe)",
                vec![tage(), z("vpf_monate_am_ort", 2, true), z("vpf_fruehstuecke_gestellt_anzahl", 2, true)],
                None,
            ),
        ],
    );
}

/// Mehr als drei Monate am Ort: nach der Frist zaehlen NUR bestaetigte Tage in der Summe fuer die Unterbrechungsfrage ("Nur
/// bestaetigte Tage zaehlen"). Vorlaeufige Tage bei bestaetigter Nach-Frist-Zahl 0 fragen die Unterbrechung nicht.
#[test]
fn verpflegung_nach_der_frist_zaehlt_nur_bestaetigte_tage() {
    let nach_frist_null = |tage_bestaetigt: bool| {
        vec![
            z("tage_24h", 5, tage_bestaetigt),
            z("vpf_monate_am_ort", 4, true),
            z("vpf_tage_24h_nach_drei_monaten", 0, true),
            b("vpf_keine_mahlzeitengestellung", true, true),
        ]
    };
    pruefe(
        Scheibe::RentnerGesamt,
        vec![
            fall("Tage vorlaeufig, Nach-Frist-Tage 0 bestaetigt: kein bestaetigter Tag, keine Unterbrechungsfrage", nach_frist_null(false), None),
            fall(
                "Tage bestaetigt, Nach-Frist-Tage 0 bestaetigt: die Unterbrechung ist offen (Gegenprobe)",
                nach_frist_null(true),
                Some("verpflegung_dreimonatsfrist_unterbrechung_offen"),
            ),
        ],
    );
}

/// Python `sum(wert or 0)`: eine nicht leere Liste oder ein nicht leeres Objekt als Tage ist `0 + [..]` = `TypeError` (die
/// Ausnahme steigt aus dem Guard auf, HTTP 500); eine leere ist 0. Die Gegenprobe zu `text_ist_ein_typfehler` in `werbungskosten.rs`.
#[test]
fn liste_und_objekt_als_tage_sind_ein_typfehler_nur_wenn_sie_nicht_leer_sind() {
    for (name, nicht_leer, leer) in [("Liste", json!([1]), json!([])), ("Objekt", json!({"a": 1}), json!({}))] {
        let fehler = guard(Scheibe::RentnerGesamt, &[w("tage_24h", nicht_leer, true)]);
        assert!(
            matches!(fehler, Err(BescheidFehler::Python { klasse: "TypeError", .. })),
            "{name} nicht leer: {fehler:?}"
        );
        let leer_ok = guard(Scheibe::RentnerGesamt, &[w("tage_24h", leer, true)]);
        assert!(matches!(leer_ok, Ok(None)), "{name} leer: {leer_ok:?}");
    }
}

/// Uebernachtung (§ 9 Abs. 1 Nr. 5a): Ort (ein Bool, bestaetigt), die drei Bedingungen und beide Zeitangaben (ganze Zahlen, bestaetigt)
/// muessen da sein; jede einzelne, die fehlt oder nur vorlaeufig ist, sperrt.
#[test]
fn uebernachtung_jede_einzelne_angabe_muss_bestaetigt_sein() {
    let bedingungen = || {
        vec![
            b("uebernachtung_auswaerts", true, true),
            b("uebernachtung_alleinnutzung", true, true),
            b("uebernachtung_keine_lange_unterbrechung", true, true),
        ]
    };
    let mit = |mut ort: Vec<Ev>, mut bed: Vec<Ev>, mut zeit: Vec<Ev>| {
        let mut e = vec![z("uebernachtung_kosten_monat", 60_000, true)];
        e.append(&mut ort);
        e.append(&mut bed);
        e.append(&mut zeit);
        e
    };
    let zeit = || vec![z("uebernachtung_monate_bisher", 0, true), z("uebernachtung_monate", 6, true)];
    let bedingungen_ohne_alleinnutzung = || {
        vec![
            b("uebernachtung_auswaerts", true, true),
            b("uebernachtung_alleinnutzung", true, false),
            b("uebernachtung_keine_lange_unterbrechung", true, true),
        ]
    };
    pruefe(
        Scheibe::RentnerGesamt,
        vec![
            fall("alles bestaetigt (Gegenprobe)", mit(vec![b("uebernachtung_im_inland", true, true)], bedingungen(), zeit()), None),
            fall(
                "Ort ja nur vorlaeufig",
                mit(vec![b("uebernachtung_im_inland", true, false)], bedingungen(), zeit()),
                Some("uebernachtung_tatbestand_offen"),
            ),
            fall(
                "Ort ist ein Text statt eines Bools",
                mit(vec![w("uebernachtung_im_inland", json!("ja"), true)], bedingungen(), zeit()),
                Some("uebernachtung_tatbestand_offen"),
            ),
            fall(
                "Ort bestaetigt, eine Bedingung nur vorlaeufig",
                mit(vec![b("uebernachtung_im_inland", true, true)], bedingungen_ohne_alleinnutzung(), zeit()),
                Some("uebernachtung_tatbestand_offen"),
            ),
            fall(
                "Monate bisher bestaetigt, Monate nur vorlaeufig",
                mit(
                    vec![b("uebernachtung_im_inland", true, true)],
                    bedingungen(),
                    vec![z("uebernachtung_monate_bisher", 0, true), z("uebernachtung_monate", 6, false)],
                ),
                Some("uebernachtung_zeitraum_offen"),
            ),
            fall(
                "Monate bestaetigt, Monate bisher nur vorlaeufig",
                mit(
                    vec![b("uebernachtung_im_inland", true, true)],
                    bedingungen(),
                    vec![z("uebernachtung_monate_bisher", 0, false), z("uebernachtung_monate", 6, true)],
                ),
                Some("uebernachtung_zeitraum_offen"),
            ),
            fall(
                "Monate bisher bestaetigt, aber ein Text statt einer ganzen Zahl",
                mit(
                    vec![b("uebernachtung_im_inland", true, true)],
                    bedingungen(),
                    vec![w("uebernachtung_monate_bisher", json!("0"), true), z("uebernachtung_monate", 6, true)],
                ),
                Some("uebernachtung_zeitraum_offen"),
            ),
            fall(
                "Monate bestaetigt, aber eine Kommazahl statt einer ganzen Zahl",
                mit(
                    vec![b("uebernachtung_im_inland", true, true)],
                    bedingungen(),
                    vec![z("uebernachtung_monate_bisher", 0, true), w("uebernachtung_monate", json!(6.5), true)],
                ),
                Some("uebernachtung_zeitraum_offen"),
            ),
        ],
    );
}

/// Arbeitsmittel bis 800 EUR netto: der Sofortabzug muss BESTAETIGT gewaehlt sein ("is not True" auf dem Rohwert liess ein
/// vorlaeufiges True durch, Naht-Fix). Ein bestaetigtes "nein" ist keine Wahl: die AfA-Antworten fehlen dann. Kosten 0 sind kein Arbeitsmittel.
#[test]
fn arbeitsmittel_sofortabzug_nur_bestaetigt_gewaehlt_und_null_ist_kein_arbeitsmittel() {
    let am = || z("am_anschaffungskosten", 50_000, true);
    pruefe(
        Scheibe::RentnerGesamt,
        vec![
            fall("500 EUR, Sofortabzug bestaetigt gewaehlt (Gegenprobe)", vec![am(), b("am_gwg_sofortabzug_gewaehlt", true, true)], None),
            fall(
                "500 EUR, Sofortabzug nur vorlaeufig gewaehlt",
                vec![am(), b("am_gwg_sofortabzug_gewaehlt", true, false)],
                Some("arbeitsmittel_afa_ueber_gwg_offen"),
            ),
            fall(
                "500 EUR, Sofortabzug bestaetigt abgewaehlt, keine Nutzungsdauer",
                vec![am(), b("am_gwg_sofortabzug_gewaehlt", false, true)],
                Some("arbeitsmittel_afa_ueber_gwg_offen"),
            ),
            fall("Kosten 0: kein Arbeitsmittel, keine Sperre", vec![z("am_anschaffungskosten", 0, true)], None),
        ],
    );
}

// ---------------------------------------------------------------- § 16 Abs. 4 und Realsplitting

/// § 16 Abs. 4: ein Veraeusserungsgewinn ueber 0 verlangt BEIDE Bedingungen bestaetigt (55 Jahre oder berufsunfaehig, erstmalig).
/// Eine allein genuegt nicht, auch nicht ein vorlaeufiges ja. Fuer den Ehegatten gilt es bei Zusammenveranlagung wie fuer Person A.
#[test]
fn p16_abs4_verlangt_beide_bedingungen_bestaetigt() {
    let a = |alter: Option<(bool, bool)>, erstmalig: Option<(bool, bool)>| {
        let mut e = vec![b("kein_gewinn", false, true), z("rentner_veraeusserungsgewinn", 1_000_000, true)];
        if let Some((wert, bestaetigt)) = alter {
            e.push(b("rentner_alter_55_oder_berufsunfaehig", wert, bestaetigt));
        }
        if let Some((wert, bestaetigt)) = erstmalig {
            e.push(b("rentner_freibetrag_erstmalig", wert, bestaetigt));
        }
        e
    };
    let partner = |alter: Option<(bool, bool)>, erstmalig: Option<(bool, bool)>| {
        let mut e = vec![
            ("veranlagung", json!("zusammen"), true),
            b("kein_gewinn", false, true),
            z("rentner_veraeusserungsgewinn_partner", 1_000_000, true),
        ];
        if let Some((wert, bestaetigt)) = alter {
            e.push(b("rentner_alter_55_oder_berufsunfaehig_partner", wert, bestaetigt));
        }
        if let Some((wert, bestaetigt)) = erstmalig {
            e.push(b("rentner_freibetrag_erstmalig_partner", wert, bestaetigt));
        }
        e
    };
    let ja = Some((true, true));
    let offen = Some("p16_4_gate_offen");
    pruefe(
        Scheibe::RentnerGesamt,
        vec![
            fall("Person A: nur das Alter bestaetigt", a(ja, None), offen),
            fall("Person A: nur erstmalig bestaetigt", a(None, ja), offen),
            fall("Person A: Alter bestaetigt, erstmalig nur vorlaeufig", a(ja, Some((true, false))), offen),
            fall("Person A: Alter nur vorlaeufig, erstmalig bestaetigt", a(Some((true, false)), ja), offen),
            fall("Person A: beide bestaetigt mit nein (eine Antwort ist eine Antwort)", a(Some((false, true)), Some((false, true))), None),
            fall("Person B: nur das Alter bestaetigt", partner(ja, None), offen),
            fall("Person B: nur erstmalig bestaetigt", partner(None, ja), offen),
            fall("Person B: Alter bestaetigt, erstmalig nur vorlaeufig", partner(ja, Some((true, false))), offen),
            fall("Person B: Alter nur vorlaeufig, erstmalig bestaetigt", partner(Some((true, false)), ja), offen),
            fall("Person B: beide bestaetigt (Gegenprobe)", partner(ja, ja), None),
        ],
    );
}

/// Realsplitting (§ 10 Abs. 1a Nr. 1): bei "kein Realsplitting = nein" verlangt der Guard Zustimmung UND Unterhaltsleistungen
/// bestaetigt. Eine verneinte Zustimmung schliesst die Regel aus; dann fragt niemand den Betrag.
#[test]
fn realsplitting_verlangt_zustimmung_und_betrag_bestaetigt() {
    let rs = |zustimmung: Option<(bool, bool)>, betrag: Option<bool>| {
        let mut e = vec![b("kein_realsplitting", false, true)];
        if let Some((wert, bestaetigt)) = zustimmung {
            e.push(b("realsplitting_zustimmung", wert, bestaetigt));
        }
        if let Some(bestaetigt) = betrag {
            e.push(z("realsplitting_unterhaltsleistungen", 500_000, bestaetigt));
        }
        e
    };
    let offen = Some("realsplitting_angaben_offen");
    pruefe(
        Scheibe::Gesamt,
        vec![
            fall("Zustimmung ja und Betrag bestaetigt (Gegenprobe)", rs(Some((true, true)), Some(true)), None),
            fall("Zustimmung fehlt, Betrag bestaetigt", rs(None, Some(true)), offen),
            fall("Zustimmung nur vorlaeufig, Betrag bestaetigt", rs(Some((true, false)), Some(true)), offen),
            fall("Zustimmung bestaetigt, Betrag fehlt", rs(Some((true, true)), None), offen),
            fall("Zustimmung bestaetigt, Betrag nur vorlaeufig", rs(Some((true, true)), Some(false)), offen),
            fall("Zustimmung nein bestaetigt: die Regel ist aus, der Betrag wird nicht verlangt", rs(Some((false, true)), None), None),
        ],
    );
}
