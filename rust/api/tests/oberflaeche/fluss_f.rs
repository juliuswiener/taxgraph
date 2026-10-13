//! Fluss F: der Rechenweg (die vier Stufen unter der Steuerzahl) erscheint nur, wenn er stimmt.
//!
//! Gegenstueck zu `tests/test_ui_rechenweg.py` (11 Funktionen). GELD: eine falsche Kette neben der richtigen Zahl
//! zeigt dem Nutzer zwei Steuern unter einem Namen; eine Kette, die nicht bei der Zahl endet, darf nie erscheinen.
//! Sechs Funktionen messen das DOM (360 px breit), fuenf die Kette selbst (`GET /ergebnis`, ohne Browser).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Zahlen der fuenf HTTP-Funktionen und des DOM stehen so im Python-Test (dort am
//! Python-Dienst gemessen). Die Pflicht-Kegel (`kegel_*`) sind Kopien aus `rust/api/tests/kette_endstand_hermetisch.rs`,
//! dort aus `tests/_kegel.py::kegel_fuer` erzeugt. ponytail: zwei Kopien, ein gemeinsames Modul, wenn ein dritter Test
//! sie braucht. Fehlt dem Kegel ein Feld, ist `grund` nicht `bestaetigt`, und der Pruefpunkt sagt es.

use serde_json::{json, Value};

use crate::dienst::{Dienst, Start};
use crate::seite::Umgebung;

type Paare = Vec<(&'static str, Value)>;

/// Basis plus Aenderungen: ein Feld, das die Basis schon traegt, wird ersetzt, ein neues angehaengt
/// (wie `tests/_kegel.py::kegel_fuer` mit dem `gesetzt`-Dict).
fn mit(basis: Paare, aenderungen: Paare) -> Paare {
    let mut raus = basis;
    for (feld, wert) in aenderungen {
        match raus.iter_mut().find(|(f, _)| *f == feld) {
            Some(p) => p.1 = wert,
            None => raus.push((feld, wert)),
        }
    }
    raus
}

fn kegel_gesamt() -> Paare {
    vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(0)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
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
        ("kap_kapitalertraege", json!(0)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ]
}

fn kegel_rentner() -> Paare {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(2_000_000)),
        ("rentner_renten_beginn_jahr", json!(2_025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
        ("veranlagung", json!("einzel")),
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
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("rentner_rentenfreibetrag", json!(0)),
    ]
}

fn kegel_an_gesamt() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(0)),
        ("veranlagung", json!("einzel")),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
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
        ("dhf_unterkunftskosten_monat", json!(0)),
        ("dhf_monate", json!(0)),
        ("dhf_im_inland", json!(false)),
        ("dhf_beruflich_veranlasst", json!(false)),
        ("dhf_eigener_hausstand", json!(false)),
        ("dhf_finanzielle_beteiligung", json!(false)),
        ("tage_24h", json!(0)),
        ("tage_an_abreise", json!(0)),
        ("tage_ueber_8h_eintaegig", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("fam_anzahl_kinder", json!(0)),
        ("verlustvortrag_bestand", json!(0)),
    ]
}

/// Person B bei Zusammenveranlagung (`_LOHN_60K_ZUSAMMEN`): sonst sperrt `partner_kegel_offen` vor jeder Zahl.
fn partner_null() -> Paare {
    vec![
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ]
}

fn lohn_zusammen(brutto: i64) -> Paare {
    mit(vec![("veranlagung", json!("zusammen")), ("bruttoarbeitslohn", json!(brutto))], partner_null())
}

fn vg_a() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
    ]
}

fn vg_b() -> Paare {
    vec![
        ("rentner_alter_55_oder_berufsunfaehig_partner", json!(true)),
        ("rentner_freibetrag_erstmalig_partner", json!(true)),
    ]
}

/// Ein Ereignis wie die Oberflaeche es schreibt (`_laie` im Python-Test).
fn laie(feld: &str, wert: &Value) -> Value {
    json!({"feld_id": feld, "wert": wert, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")}})
}

/// Fall anlegen, jedes Paar als bestaetigte Antwort schreiben, `GET /ergebnis` lesen (`_fall_mit`, ohne Behauptung).
fn fall_mit(d: &Dienst, fid: &str, scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    d.fall(scheibe, fid);
    for (feld, wert) in paare {
        let (st, r) = d.post(&format!("/fall/{fid}/event"), &laie(feld, wert));
        assert_eq!(st, 201, "{fid}: {feld} -> {r}");
    }
    let (st, erg) = d.get(&format!("/fall/{fid}/ergebnis"));
    assert_eq!(st, 200, "{fid}: {erg}");
    erg
}

/// `fall_mit`, dazu die Vorbedingung des Python-Tests: der Kegel ist voll, der Fall ist `bestaetigt`.
macro_rules! fall_bestaetigt {
    ($id:literal, $d:expr, $fid:expr, $scheibe:expr, $paare:expr) => {{
        let erg = fall_mit($d, $fid, $scheibe, &$paare);
        pp!($id, erg["grund"] == json!("bestaetigt"), "{}: Kegel unvollstaendig oder Sperre: {}", $fid, erg);
        erg
    }};
}

const STUFEN: [&str; 4] = ["gesamtbetrag_der_einkuenfte", "zu_versteuerndes_einkommen", "tarifliche_est", "festzusetzende_est"];

fn vier_stufen(k: &Value) -> Vec<i64> {
    STUFEN.iter().map(|s| k[*s].as_i64().unwrap_or(i64::MIN)).collect()
}

/// Die `p31`-Stufe hat genau die Schluessel `guenstiger`, `kindergeld`, `text`.
fn p31_form(k: &Value) -> bool {
    let mut schluessel: Vec<&String> = k["p31"].as_object().map(|o| o.keys().collect()).unwrap_or_default();
    schluessel.sort();
    schluessel == ["guenstiger", "kindergeld", "text"]
}

// ---- Die Kette selbst, ohne Browser --------------------------------------------------------------------------

#[test]
fn fluss_f_kette_ueber_http() {
    let d = Dienst::starte(&Start::default());

    // `test_kette_endet_bei_der_zahl`: steht eine Kette da, endet sie bei der Zahl; die Gegenproben ohne Sonderregel
    // MUESSEN eine Kette haben (sonst waere „nie eine Kette“ auch gruen).
    let lohn_einzel = vec![("veranlagung", json!("einzel")), ("bruttoarbeitslohn", json!(6_000_000))];
    let rente_20k = kegel_rentner();
    let faelle: Vec<(&str, &str, Paare, bool)> = vec![
        ("lohn60k-gegenprobe", "gesamt", mit(mit(kegel_gesamt(), lohn_zusammen(6_000_000)), vec![("kein_gewinn", json!(true))]), true),
        (
            "vg-a120-allein",
            "gesamt",
            mit(mit(mit(kegel_gesamt(), lohn_zusammen(6_000_000)), vg_a()), vec![("rentner_veraeusserungsgewinn", json!(12_000_000))]),
            false,
        ),
        (
            "vg-a60-b60",
            "gesamt",
            mit(
                mit(mit(mit(kegel_gesamt(), lohn_zusammen(6_000_000)), vg_a()), vg_b()),
                vec![("rentner_veraeusserungsgewinn", json!(6_000_000)), ("rentner_veraeusserungsgewinn_partner", json!(6_000_000))],
            ),
            false,
        ),
        ("rente20k-gegenprobe", "rentner_gesamt", mit(rente_20k.clone(), vec![("kein_gewinn", json!(true))]), true),
        (
            "rente20k-vg120",
            "rentner_gesamt",
            mit(mit(rente_20k, vg_a()), vec![("rentner_veraeusserungsgewinn", json!(12_000_000))]),
            false,
        ),
        (
            "lohn60k-einzel-gegenprobe",
            "gesamt",
            mit(mit(kegel_gesamt(), lohn_einzel.clone()), vec![("kein_kap", json!(true)), ("kein_gewinn", json!(true))]),
            true,
        ),
        (
            "p32d-kap30k",
            "gesamt",
            mit(
                mit(kegel_gesamt(), lohn_einzel.clone()),
                vec![("kein_gewinn", json!(true)), ("kein_kap", json!(false)), ("kap_kapitalertraege", json!(3_000_000))],
            ),
            false,
        ),
        (
            "p35-gewst1500",
            "gesamt",
            mit(
                mit(kegel_gesamt(), lohn_einzel.clone()),
                vec![
                    ("kein_kap", json!(true)),
                    ("kein_gewinn", json!(false)),
                    ("einkuenfte_gewinn", json!(5_000_000)),
                    ("gewinn_betriebsart", json!("gewerbe")),
                    ("gewst_messbetrag", json!(150_000)),
                    ("gewst_hebesatz", json!(400)),
                ],
            ),
            false,
        ),
        (
            "p32b-pe20k",
            "gesamt",
            mit(
                mit(kegel_gesamt(), lohn_einzel.clone()),
                vec![("kein_kap", json!(true)), ("kein_gewinn", json!(true)), ("p32b_progressionseinkuenfte", json!(2_000_000))],
            ),
            false,
        ),
        (
            "vg-einzel120",
            "gesamt",
            mit(
                mit(mit(kegel_gesamt(), lohn_einzel), vec![("kein_kap", json!(true))]),
                mit(vg_a(), vec![("rentner_veraeusserungsgewinn", json!(12_000_000))]),
            ),
            false,
        ),
    ];
    for (name, scheibe, paare, kette_pflicht) in faelle {
        let erg = fall_bestaetigt!("F.kette_endet_bei_der_zahl", &d, name, scheibe, paare);
        let k = &erg["kette"];
        if kette_pflicht {
            pp!("F.kette_endet_bei_der_zahl", !k.is_null(), "{name}: die Gegenprobe ohne Sonderregel verliert die Kette: {erg}");
        }
        if !k.is_null() {
            pp!(
                "F.kette_endet_bei_der_zahl",
                k["festzusetzende_est"].as_i64().map(|e| e * 100) == erg["zahl_cent"].as_i64(),
                "{name}: Rechenweg endet bei {} EUR, die Zahl darueber ist {} ct: zwei Steuern unter demselben Label",
                k["festzusetzende_est"],
                erg["zahl_cent"]
            );
        }
    }

    // `test_p31_stufe_nennt_die_guenstigerpruefung`: der Freibetrag gewinnt, das Kindergeld wird hinzugerechnet.
    let lohn_300k = mit(mit(kegel_gesamt(), lohn_zusammen(30_000_000)), vec![("kein_gewinn", json!(true)), ("fam_anzahl_kinder", json!(1))]);
    let e = fall_bestaetigt!("F.p31_stufe_nennt_die_guenstigerpruefung", &d, "p31-freibetrag-gewinnt", "gesamt", lohn_300k.clone());
    let k = &e["kette"];
    pp!("F.p31_stufe_nennt_die_guenstigerpruefung", e["zahl_cent"] == json!(10_265_600), "{e}");
    pp!("F.p31_stufe_nennt_die_guenstigerpruefung", !k.is_null(), "Kette fehlt bei 1 Kind trotz bestaetigtem Fall: {e}");
    pp!("F.p31_stufe_nennt_die_guenstigerpruefung", k["festzusetzende_est"] == json!(102_656), "{k}");
    pp!("F.p31_stufe_nennt_die_guenstigerpruefung", k["festzusetzende_est"].as_i64().map(|x| x * 100) == e["zahl_cent"].as_i64(), "{k}");
    pp!("F.p31_stufe_nennt_die_guenstigerpruefung", k["tarifliche_est"] == json!(99_596), "{k}");
    pp!("F.p31_stufe_nennt_die_guenstigerpruefung", k["zu_versteuerndes_einkommen"] == json!(289_098), "{k}");
    pp!(
        "F.p31_stufe_nennt_die_guenstigerpruefung",
        k["p31"]["guenstiger"] == json!("freibetraege") && k["p31"]["kindergeld"] == json!(3_060) && p31_form(k),
        "{}",
        k["p31"]
    );
    let text = k["p31"]["text"].as_str().unwrap_or_default();
    pp!("F.p31_stufe_nennt_die_guenstigerpruefung", text.contains("hinzugerechnet") && text.contains("3.060"), "{text}");

    // `test_p31_kindergeld_gewinnt_kette_bleibt_vollstaendig` und die Kontrolle ohne Kind.
    let lohn_20k = |kinder: i64| {
        mit(
            mit(kegel_gesamt(), vec![("veranlagung", json!("einzel")), ("bruttoarbeitslohn", json!(2_000_000))]),
            vec![("kein_kap", json!(true)), ("kein_gewinn", json!(true)), ("fam_anzahl_kinder", json!(kinder))],
        )
    };
    let e = fall_bestaetigt!("F.p31_kindergeld_gewinnt_kette_bleibt_vollstaendig", &d, "p31-kindergeld-gewinnt", "gesamt", lohn_20k(1));
    let k = &e["kette"];
    pp!("F.p31_kindergeld_gewinnt_kette_bleibt_vollstaendig", e["zahl_cent"] == json!(132_700), "{e}");
    pp!("F.p31_kindergeld_gewinnt_kette_bleibt_vollstaendig", !k.is_null(), "Kette fehlt bei 1 Kind trotz bestaetigtem Fall: {e}");
    pp!("F.p31_kindergeld_gewinnt_kette_bleibt_vollstaendig", k["festzusetzende_est"] == json!(1_327) && k["tarifliche_est"] == json!(1_327), "{k}");
    pp!("F.p31_kindergeld_gewinnt_kette_bleibt_vollstaendig", k["festzusetzende_est"].as_i64().map(|x| x * 100) == e["zahl_cent"].as_i64(), "{k}");
    pp!(
        "F.p31_kindergeld_gewinnt_kette_bleibt_vollstaendig",
        k["p31"]["guenstiger"] == json!("kindergeld") && k["p31"]["kindergeld"] == json!(3_060),
        "{}",
        k["p31"]
    );

    let e = fall_bestaetigt!("F.p31_kontrolle_kinderlos_hat_dieselbe_kette_wie_vorher", &d, "p31-kontrolle", "gesamt", lohn_20k(0));
    let k = &e["kette"];
    pp!("F.p31_kontrolle_kinderlos_hat_dieselbe_kette_wie_vorher", e["zahl_cent"] == json!(132_700), "{e}");
    pp!("F.p31_kontrolle_kinderlos_hat_dieselbe_kette_wie_vorher", !k.is_null(), "Kette fehlt im kinderlosen Fall: {e}");
    pp!("F.p31_kontrolle_kinderlos_hat_dieselbe_kette_wie_vorher", vier_stufen(k) == [18_770, 18_734, 1_327, 1_327], "{k}");
    pp!("F.p31_kontrolle_kinderlos_hat_dieselbe_kette_wie_vorher", k.get("p31").is_none(), "Ohne Kinder gibt es keine § 31-Entscheidung zu nennen: {k}");

    // `test_rentner_kette_mit_kind_nennt_p31`: 200.000 EUR Rente, ein Kind, das Kindergeld gewinnt.
    let rente_200k = mit(kegel_rentner(), vec![("rentner_jahresrente", json!(20_000_000))]);
    let ohne = fall_bestaetigt!("F.rentner_kette_mit_kind_nennt_p31", &d, "p31-rentner-ohne", "rentner_gesamt", rente_200k.clone());
    let e = fall_bestaetigt!("F.rentner_kette_mit_kind_nennt_p31", &d, "p31-rentner", "rentner_gesamt", mit(rente_200k, vec![("fam_anzahl_kinder", json!(1))]));
    let k = &e["kette"];
    pp!("F.rentner_kette_mit_kind_nennt_p31", e["zahl_cent"] == ohne["zahl_cent"], "das Kindergeld gewinnt, die Zahl bleibt wie ohne Kind: {} gegen {}", e["zahl_cent"], ohne["zahl_cent"]);
    pp!("F.rentner_kette_mit_kind_nennt_p31", !k.is_null(), "Kette fehlt bei rentner_gesamt mit Kind: {e}");
    pp!("F.rentner_kette_mit_kind_nennt_p31", k["festzusetzende_est"].as_i64().map(|x| x * 100) == e["zahl_cent"].as_i64(), "{k}");
    pp!(
        "F.rentner_kette_mit_kind_nennt_p31",
        k["p31"]["guenstiger"] == json!("kindergeld") && k["p31"]["kindergeld"] == json!(3_060),
        "{}",
        k["p31"]
    );
    pp!("F.rentner_kette_mit_kind_nennt_p31", vier_stufen(k) == [166_898, 166_862, 59_170, 59_170], "{k}");
    pp!("F.rentner_kette_mit_kind_nennt_p31", vier_stufen(&ohne["kette"]) == [166_898, 166_862, 59_170, 59_170], "{}", ohne["kette"]);
}

// ---- Der Rechenweg im Bild, 360 px breit ---------------------------------------------------------------------

/// Die Seite laden, `FALL` setzen, den Start-Bildschirm weg und `zeigeErgebnis()` rufen (Muster der Python-Tests).
fn zeige_ergebnis(u: &Umgebung, fall: &str) {
    u.oeffne_start();
    u.s.skript(
        &format!(
            "FALL = '{fall}'; document.getElementById('login').hidden = true; document.getElementById('start').hidden = true; \
             document.getElementById('flow').hidden = false; (async () => {{ await zeigeErgebnis(); }})();"
        ),
        &[],
    );
}

/// Wartet, bis `#rechenweg` sichtbar ist (die Kette oder der Hinweis stehen).
fn rechenweg_da(u: &Umgebung) -> bool {
    u.s.try_warte_bis("(() => { const e = document.getElementById('rechenweg'); return !!e && !e.hidden; })()", 10).is_some()
}

/// Die Zeilen von `#rechenweg-body`: Klasse, Beschriftung, Wert, Unterzeile.
fn reihen(u: &Umgebung) -> Vec<Value> {
    u.s.ausdruck(
        "[...document.querySelectorAll('#rechenweg-body .rw-reihe')].map(r => ({klasse: r.className, \
         label: (r.querySelector('.rw-label') || {}).textContent || '', \
         wert: ((r.querySelector('.rw-wert') || {}).textContent || '').trim(), \
         sub: ((r.querySelector('.rw-label .rw-sub') || {}).textContent || '')}))",
    )
    .as_array()
    .cloned()
    .unwrap_or_default()
}

fn breiter_als_der_schirm(u: &Umgebung) -> i64 {
    u.s.ausdruck("document.documentElement.scrollWidth").as_i64().unwrap_or(-1)
}

#[test]
fn fluss_f_rechenweg_im_bild() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 640);
    let d = &u.dienst;

    // positiv: eine Kette, vier Stufen, der Betrag der API steht im Bild.
    let basis = mit(
        kegel_gesamt(),
        vec![
            ("bruttoarbeitslohn", json!(5_000_000)),
            ("ep_entfernung_km", json!(20)),
            ("ep_eigenes_kfz", json!(true)),
            ("ep_arbeitstage", json!(220)),
        ],
    );
    let erg = fall_bestaetigt!("F.rechenweg_positiv", d, "rw-pos", "gesamt", basis.clone());
    let k = &erg["kette"];
    pp!("F.rechenweg_positiv", !k.is_null(), "kette fehlt im Positivfall (gesamt, kinderlos): {erg}");
    pp!("F.rechenweg_positiv", k.get("gesamtbetrag_der_einkuenfte").is_some() && k.get("festzusetzende_est").is_some(), "{k}");
    pp!(
        "F.rechenweg_positiv",
        k["festzusetzende_est"].as_i64().map(|x| x * 100) == erg["zahl_cent"].as_i64(),
        "kette[festzusetzende_est]={} * 100 != zahl_cent={}",
        k["festzusetzende_est"],
        erg["zahl_cent"]
    );
    zeige_ergebnis(&u, "rw-pos");
    pp!("F.rechenweg_positiv", rechenweg_da(&u), "#rechenweg bleibt verborgen, obwohl die Kette da ist");
    let zeilen = reihen(&u);
    let stufen = zeilen.iter().filter(|r| !r["klasse"].as_str().unwrap_or_default().contains("rw-delta")).count();
    pp!("F.rechenweg_positiv", stufen >= 4, "Erwarte 4 Stufen, gefunden: {stufen}");
    let letzte = zeilen.last().cloned().unwrap_or(Value::Null);
    pp!("F.rechenweg_positiv", letzte["wert"].as_str().unwrap_or_default().ends_with('€'), "Letzte Stufe kein Euro-Wert: {letzte}");
    let gesamt_text = zeilen
        .iter()
        .filter(|r| r["label"].as_str().unwrap_or_default().contains("Gesamtbetrag der Einkünfte"))
        .map(|r| r["wert"].as_str().unwrap_or_default().to_owned())
        .next_back();
    pp!("F.rechenweg_positiv", gesamt_text.is_some(), "Gesamtbetrag der Einkünfte nicht im DOM: {zeilen:?}");
    let gesamt_text = gesamt_text.unwrap_or_default();
    pp!(
        "F.rechenweg_positiv",
        gesamt_text.replace('.', "").contains(&k["gesamtbetrag_der_einkuenfte"].to_string()),
        "API-Wert {} nicht in '{gesamt_text}'",
        k["gesamtbetrag_der_einkuenfte"]
    );
    let breit = breiter_als_der_schirm(&u);
    pp!("F.rechenweg_positiv", breit <= 360, "scrollWidth={breit} > 360");

    // negativ: eine Scheibe ohne Kette zeigt keinen Rechenweg (an_gesamt hat gar keinen).
    d.fall("an_gesamt", "rw-neg");
    for (feld, wert) in [
        ("bruttoarbeitslohn", json!(50_000)),
        ("veranlagung", json!("einzel")),
        ("fam_anzahl_kinder", json!(0)),
        ("ep_entfernung_km", json!(20)),
        ("ep_eigenes_kfz", json!(true)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_arbeitstage", json!(220)),
        ("verlustvortrag_bestand", json!(0)),
        ("p36_lohnsteuer", json!(0)),
        ("p36_vorauszahlungen", json!(0)),
        ("kist_konfession", json!("keine")),
        ("p35a_mitveranlagung", json!(false)),
    ] {
        let (st, r) = d.post("/fall/rw-neg/event", &laie(feld, &wert));
        assert_eq!(st, 201, "{feld}: {r}");
    }
    zeige_ergebnis(&u, "rw-neg");
    // Gewartet wird auf die Ergebnisanzeige selbst (`#fertig`), sonst misst „verborgen“ nur die Zeit vor dem Aufbau.
    let fertig = u.s.try_warte_bis("(() => { const e = document.getElementById('fertig'); return !!e && !e.hidden; })()", 10);
    pp!("F.rechenweg_negativ", fertig.is_some(), "die Ergebnisanzeige kommt nicht: der Test misst sonst nichts");
    pp!("F.rechenweg_negativ", u.s.ausdruck("!!document.getElementById('rechenweg')") == json!(true), "#rechenweg nicht gefunden");
    pp!(
        "F.rechenweg_negativ",
        u.s.ausdruck("document.getElementById('rechenweg').hidden") == json!(true),
        "rechenweg sollte hidden sein, wenn die Kette fehlt (Guard-Fall)"
    );
    let breit = breiter_als_der_schirm(&u);
    pp!("F.rechenweg_negativ", breit <= 360, "scrollWidth={breit} > 360");

    // bestaetigt ohne Kette: der Rechenweg zeigt den Hinweis, die Tabelle bleibt verborgen.
    let an_gesamt = mit(
        kegel_an_gesamt(),
        vec![
            ("bruttoarbeitslohn", json!(5_000_000)),
            ("ep_entfernung_km", json!(20)),
            ("ep_eigenes_kfz", json!(true)),
            ("ep_arbeitstage", json!(220)),
            ("dhf_im_inland", json!(true)),
            ("p36_lohnsteuer", json!(0)),
            ("p36_vorauszahlungen", json!(0)),
            ("kist_konfession", json!("keine")),
            ("p35a_mitveranlagung", json!(false)),
        ],
    );
    let erg = fall_bestaetigt!("F.rechenweg_bestaetigt_ohne_kette", d, "rw-hinweis", "an_gesamt", an_gesamt);
    pp!("F.rechenweg_bestaetigt_ohne_kette", erg["kette"].is_null(), "an_gesamt sollte keine kette liefern: {}", erg["kette"]);
    zeige_ergebnis(&u, "rw-hinweis");
    pp!("F.rechenweg_bestaetigt_ohne_kette", rechenweg_da(&u), "#rechenweg bleibt verborgen: der Hinweis fehlt");
    pp!("F.rechenweg_bestaetigt_ohne_kette", u.s.ausdruck("document.getElementById('rechenweg-tabelle').hidden") == json!(true), "Tabelle sollte hidden sein, wenn keine Kette da ist");
    pp!("F.rechenweg_bestaetigt_ohne_kette", u.s.ausdruck("document.getElementById('rechenweg-hinweis').hidden") == json!(false), "Hinweis sollte sichtbar sein, wenn die Kette fehlt");
    let hinweis = u.s.ausdruck("document.getElementById('rechenweg-hinweis').textContent");
    pp!("F.rechenweg_bestaetigt_ohne_kette", hinweis.as_str().is_some_and(|t| !t.is_empty()), "Hinweis sollte Text enthalten");
    let breit = breiter_als_der_schirm(&u);
    pp!("F.rechenweg_bestaetigt_ohne_kette", breit <= 360, "scrollWidth={breit} > 360");

    // stille Null: die Zahl gilt, eine vorlaeufige p23-Instanz steht in `offen`, die Anzeige sagt es im Erfolgs-Zweig.
    let erg = fall_mit(d, "rw-hinweis-offen", "gesamt", &basis);
    for (feld, wert) in [
        ("p23_veraeusserungspreis", json!(20_000_000)),
        ("p23_anschaffung_herstellungskosten", json!(15_000_000)),
        ("p23_werbungskosten", json!(500_000)),
        ("p23_veraeusserungs_typ", json!("grundstueck")),
    ] {
        let mut ev = laie(feld, &wert);
        ev["zustand"] = json!("vorlaeufig");
        ev["signal"] = json!({"signal_1": null, "signal_2": null});
        let (st, r) = d.post("/fall/rw-hinweis-offen/event", &ev);
        assert_eq!(st, 201, "{feld}: {r}");
    }
    let (st, erg2) = d.get("/fall/rw-hinweis-offen/ergebnis");
    pp!("F.ergebnis_hinweis_offen_bei_stiller_null", erg["grund"] == json!("bestaetigt") && st == 200, "{erg}");
    pp!("F.ergebnis_hinweis_offen_bei_stiller_null", erg2["grund"] == json!("bestaetigt"), "{erg2}");
    pp!("F.ergebnis_hinweis_offen_bei_stiller_null", !erg2["zahl_cent"].is_null(), "Zahl muss trotz stiller Null stehen: {erg2}");
    pp!(
        "F.ergebnis_hinweis_offen_bei_stiller_null",
        erg2["offen"].as_array().is_some_and(|o| o.contains(&json!("p23_veraeusserungspreis"))),
        "{}",
        erg2["offen"]
    );
    zeige_ergebnis(&u, "rw-hinweis-offen");
    let da = u.s.try_warte_bis("!!document.querySelector('#ergebnis .ergebnis-hinweis-offen')", 10);
    pp!("F.ergebnis_hinweis_offen_bei_stiller_null", da.is_some(), "der Hinweis auf offene Angaben fehlt im Erfolgs-Zweig");
    let klasse = u.s.ausdruck("document.getElementById('ergebnis').className");
    pp!("F.ergebnis_hinweis_offen_bei_stiller_null", klasse == json!("ergebnis"), "Erfolgs-Zweig erwartet, war: {klasse}");
    let zahl = u.s.ausdruck("((document.querySelector('#ergebnis .erg-zahl') || {}).textContent || '').trim()");
    pp!("F.ergebnis_hinweis_offen_bei_stiller_null", zahl.as_str().is_some_and(|t| !t.is_empty()), "Zahl fehlt trotz stiller Null");
    let hinweis = u.s.ausdruck("document.querySelector('#ergebnis .ergebnis-hinweis-offen').textContent");
    let hinweis = hinweis.as_str().unwrap_or_default().to_owned();
    pp!("F.ergebnis_hinweis_offen_bei_stiller_null", hinweis.contains("p23_veraeusserungspreis"), "{hinweis}");
    pp!("F.ergebnis_hinweis_offen_bei_stiller_null", hinweis.to_lowercase().contains("nicht"), "{hinweis}");
    let breit = breiter_als_der_schirm(&u);
    pp!("F.ergebnis_hinweis_offen_bei_stiller_null", breit <= 360, "scrollWidth={breit} > 360");

    // der § 31-Satz steht als Unterzeile an der letzten Zeile.
    let lohn_300k = mit(mit(kegel_gesamt(), lohn_zusammen(30_000_000)), vec![("kein_gewinn", json!(true)), ("fam_anzahl_kinder", json!(1))]);
    fall_bestaetigt!("F.p31_stufe_ist_im_dom_sichtbar", d, "p31-dom", "gesamt", lohn_300k);
    zeige_ergebnis(&u, "p31-dom");
    pp!("F.p31_stufe_ist_im_dom_sichtbar", rechenweg_da(&u), "#rechenweg bleibt verborgen");
    let zeilen = reihen(&u);
    let letzte = zeilen.last().cloned().unwrap_or(Value::Null);
    pp!("F.p31_stufe_ist_im_dom_sichtbar", letzte["label"].as_str().unwrap_or_default().contains("festzusetzende Einkommensteuer"), "{letzte}");
    let sub = letzte["sub"].as_str().unwrap_or_default();
    pp!("F.p31_stufe_ist_im_dom_sichtbar", !sub.is_empty(), "Keine Unterzeile an der letzten Stufe: der § 31-Satz erreicht die Oberflaeche nicht. Zeilen: {zeilen:?}");
    pp!("F.p31_stufe_ist_im_dom_sichtbar", sub.contains("Kindergeld") && sub.contains("3.060"), "{sub}");
    pp!("F.p31_stufe_ist_im_dom_sichtbar", sub.contains("hinzugerechnet"), "Der Sprung ueber die tarifliche Steuer fehlt: {sub}");
    pp!("F.p31_stufe_ist_im_dom_sichtbar", letzte["wert"] == json!("102.656 €"), "Letzte Stufe zeigt {} statt 102.656 €", letzte["wert"]);

    // Endstand: der § 32d-Fall zeigt die TABELLE, und ihre letzte Zeile ist die gezahlte Steuer.
    let p32d = mit(
        mit(kegel_gesamt(), vec![("veranlagung", json!("einzel")), ("bruttoarbeitslohn", json!(6_000_000))]),
        vec![("kein_gewinn", json!(true)), ("kein_kap", json!(false)), ("kap_kapitalertraege", json!(3_000_000))],
    );
    let erg = fall_bestaetigt!("F.endstandskette_zeigt_tabelle_mit_der_gezahlten_steuer", d, "rw-p32d", "gesamt", p32d);
    pp!(
        "F.endstandskette_zeigt_tabelle_mit_der_gezahlten_steuer",
        !erg["kette"].is_null(),
        "Der § 32d-Fall verliert die Kette, obwohl beide Seiten aus dem Endstand speisen sollten: zahl_cent={}",
        erg["zahl_cent"]
    );
    zeige_ergebnis(&u, "rw-p32d");
    pp!("F.endstandskette_zeigt_tabelle_mit_der_gezahlten_steuer", rechenweg_da(&u), "#rechenweg bleibt verborgen");
    pp!(
        "F.endstandskette_zeigt_tabelle_mit_der_gezahlten_steuer",
        u.s.ausdruck("document.getElementById('rechenweg-tabelle').hidden") == json!(false),
        "Tabelle hidden, obwohl eine Kette da ist"
    );
    pp!("F.endstandskette_zeigt_tabelle_mit_der_gezahlten_steuer", u.s.ausdruck("document.getElementById('rechenweg-hinweis').hidden") == json!(true), "der Hinweis steht neben der Tabelle");
    let zeilen = reihen(&u);
    let letzte = zeilen.last().cloned().unwrap_or(Value::Null);
    pp!(
        "F.endstandskette_zeigt_tabelle_mit_der_gezahlten_steuer",
        letzte["wert"] == json!("21.174 €"),
        "Letzte Stufe zeigt {} statt 21.174 €: dann sind Zahl und Kette wieder zwei verschiedene Steuern",
        letzte["wert"]
    );
}
