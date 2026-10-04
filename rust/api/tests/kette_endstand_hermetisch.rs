//! Rechenweg-Kette aus dem ENDSTAND (p24a): die letzte Stufe von `GET /ergebnis` -> `kette` ist die
//! gezahlte Steuer, auch dort, wo eine Korrektur AUSSERHALB der Engine sitzt (§ 32d-Abgeltungsteuer,
//! § 32b-Zuschlag) -- im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! Vor p24a speiste `rahmen` die Kette aus dem Basis-`g` (Vor-Korrektur-Stand); `setze_kette` verwarf sie
//! dann still, `kette` war `null` (bei 30.000 EUR Kapitalertraegen endete sie bei 13.924 statt 21.174 EUR,
//! -7.250 EUR). Die Paritaets-Suiten hielten das nur mit `PARITY=1`; die CI faehrt Parity nicht.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl unten ist die Ausgabe des Python-Servers (HEAD 0aa91677 +
//! p24a-Fix, 2026-10-04, `GET /fall/{id}/ergebnis` mit denselben Events ueber `POST /event`; die
//! Ereignislisten stammen aus `tests/_kegel.py::kegel_fuer`, erzeugt von `rechenweg/gen_kette_rs.py`).
//! Kein Wert ist aus dem Rust-Code abgelesen.
//!
//! ponytail: die Erwartungswerte sind eingefroren. Aendert sich der Tarif oder der Kegel, rechnet man
//! sie am Python-Server nach und zieht die Konstanten nach.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

type Paare = Vec<(&'static str, Value)>;

struct Dienst {
    zustand: Zustand,
    token: String,
    _tmp: tempfile::TempDir,
}

fn dienst() -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let konfig = Konfig {
        wurzel: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        faelle: tmp.path().join("faelle"),
        audit_dir: tmp.path().join("faelle"),
    };
    let auth = Auth::neu(
        "testgeheimnis".into(),
        tmp.path().join("users.json"),
        Some(konfig.audit_pfad()),
    );
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    Dienst {
        zustand,
        token,
        _tmp: tmp,
    }
}

async fn sende(d: &Dienst, methode: &str, pfad: &str, body: Option<&Value>) -> (u16, Value) {
    let text = body.map(ToString::to_string);
    let mut b = Request::builder()
        .method(methode)
        .uri(pfad)
        .header("authorization", format!("Bearer {}", d.token));
    if let Some(t) = &text {
        b = b
            .header("content-type", "application/json")
            .header("content-length", t.len().to_string());
    }
    let req = b.body(text.map_or_else(Body::empty, Body::from)).unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (
        teile.status.as_u16(),
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Legt den Fall der Scheibe an, schreibt jedes Paar ueber die echte Route `POST /event` (als Nutzer-Klick,
/// bestaetigt) und liefert `GET /ergebnis`.
async fn ergebnis(scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    let d = dienst();
    let kopf = json!({"fall_id": "kette", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, antwort) = sende(&d, "POST", "/fall/kette/event", Some(&rumpf)).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, antwort) = sende(&d, "GET", "/fall/kette/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Basis plus Aenderungen: ein Feld, das die Basis schon traegt, wird ersetzt (EIN Event je Feld, kein
/// Ueberschreiben), ein neues angehaengt -- wie `tests/_kegel.py::kegel_fuer` mit dem gesetzt-Dict.
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

/// Die vier Stufen einer Kette, EURO, in der Reihenfolge der Oberflaeche.
fn stufen(a: &Value) -> [i64; 4] {
    let k = &a["kette"];
    [
        "gesamtbetrag_der_einkuenfte",
        "zu_versteuerndes_einkommen",
        "tarifliche_est",
        "festzusetzende_est",
    ]
    .map(|s| k[s].as_i64().unwrap_or(i64::MIN))
}

/// Die Kette steht da, ihre letzte Stufe IST die gezahlte Steuer, und die Stufen stimmen mit dem Python-Lauf.
fn erwarte_kette(name: &str, a: &Value, zahl_cent: i64, soll: [i64; 4]) {
    assert_eq!(
        a["grund"], "bestaetigt",
        "{name}: Positivkontrolle -- Kegel vollstaendig? {a}"
    );
    assert_eq!(
        a["zahl_cent"].as_i64(),
        Some(zahl_cent),
        "{name}: Zahl -- {a}"
    );
    assert!(
        !a["kette"].is_null(),
        "{name}: kette fehlt (null) -- `setze_kette` hat die Vor-Korrektur-Kette verworfen; die Kette \
         muss aus dem Endstand kommen und bei {zahl_cent} ct enden. Antwort: {a}"
    );
    assert_eq!(
        stufen(a),
        soll,
        "{name}: Stufen GdE/zvE/tariflich/festzusetzende -- {a}"
    );
    assert_eq!(
        soll[3] * 100,
        zahl_cent,
        "{name}: letzte Stufe muss die gezahlte Steuer sein"
    );
}

// ---- Basisfaelle (volle Kegel, aus tests/_kegel.py::kegel_fuer) ------------------------------------

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

fn g1_p32d_kapital_aenderungen() -> Paare {
    vec![
        ("kap_kapitalertraege", json!(3_000_000)),
        ("kein_kap", json!(false)),
    ]
}

fn g2_p35_gewerbesteuer_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(5_000_000)),
        ("gewinn_betriebsart", json!("gewerbe")),
        ("gewst_messbetrag", json!(150_000)),
        ("gewst_hebesatz", json!(400)),
    ]
}

fn g3_kind_kindergeld_aenderungen() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(2_000_000)),
        ("fam_anzahl_kinder", json!(1)),
    ]
}

fn g4_kind_freibetrag_p32d_aenderungen() -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(30_000_000)),
        ("kap_kapitalertraege", json!(3_000_000)),
        ("kein_kap", json!(false)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("fam_anzahl_kinder", json!(1)),
    ]
}

fn g5_vg_einzel_p34_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(12_000_000)),
    ]
}

/// § 34 Abs. 3 auf den Cent genau an der Obergrenze: 5.000.000 EUR Veraeusserungsgewinn (der Freibetrag § 16 Abs. 4 ist
/// ab 181.000 EUR 0, `netto_vg` ist der rohe Gewinn), Antrag auf den ermaessigten Satz, dauernd berufsunfaehig.
fn g6_p34_abs3_genau_5_mio_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(500_000_000)),
        ("antrag_ermaessigter_satz", json!(true)),
        ("dauernd_berufsunfaehig", json!(true)),
    ]
}

/// § 31 mit einem Kind, Zusammenveranlagung, 84.418 EUR Gewinn, sonst nichts: die Steuer mit Kinderfreibetrag plus
/// Kindergeld ist genau so hoch wie die ohne (`est_ohne` 16.046 EUR, `est_mit` 12.986 EUR, Kindergeld 3.060 EUR).
fn g7_kind_gleichstand_aenderungen() -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(0)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("fam_anzahl_kinder", json!(1)),
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(8_441_800)),
    ]
}

fn r3_rentner_p34_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(12_000_000)),
    ]
}

fn r4_rentner_p35_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(5_000_000)),
        ("gewinn_betriebsart", json!("gewerbe")),
        ("gewst_messbetrag", json!(150_000)),
        ("gewst_hebesatz", json!(400)),
    ]
}

fn r5_rentner_kind_freibetrag_aenderungen() -> Paare {
    vec![
        ("rentner_jahresrente", json!(30_000_000)),
        ("veranlagung", json!("zusammen")),
        ("fam_anzahl_kinder", json!(1)),
    ]
}

fn r1_rentner_p32d_aenderungen() -> Paare {
    vec![
        ("kein_kap", json!(false)),
        ("kap_kapitalertraege", json!(3_000_000)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
    ]
}

fn r2_rentner_kind_p32d_aenderungen() -> Paare {
    vec![
        ("kein_kap", json!(false)),
        ("fam_anzahl_kinder", json!(1)),
        ("kap_kapitalertraege", json!(3_000_000)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
    ]
}

// ---- Tests -----------------------------------------------------------------------------------------

/// Gegenprobe ohne Sonderregel: die Kette lief schon vor p24a und muss unveraendert laufen.
#[tokio::test]
async fn g0_gegenprobe_lohn_60k_kette_endet_bei_der_zahl() {
    let a = ergebnis("gesamt", &kegel_gesamt()).await;
    erwarte_kette("g0", &a, 1_392_400, [58_770, 58_734, 13_924, 13_924]);
    assert!(a["kette"]["p31"].is_null(), "kinderlos: kein p31");
}

/// § 32d, 30.000 EUR Kapitalertraege: die Abgeltungsteuer (+7.250 EUR) sitzt AUSSERHALB der Engine
/// (`result = est_raw + kap_st_k`). Tarifliche Stufe bleibt 13.924 EUR (Kapital laeuft nicht in den
/// tariflichen zvE, § 2 Abs. 5b), nur die letzte Stufe traegt den Zuschlag: 21.174 statt 13.924 EUR.
#[tokio::test]
async fn g1_p32d_kapital_kette_traegt_die_abgeltungsteuer() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g1_p32d_kapital_aenderungen()),
    )
    .await;
    erwarte_kette("g1", &a, 2_117_400, [58_770, 58_734, 13_924, 21_174]);
}

/// § 35 `EStG`: 50.000 EUR Gewerbegewinn, Messbetrag 1.500, Hebesatz 400 -- die Anrechnung steckt in
/// `steuerermaessigungen` des finalen `g2`; die Kette muss sie sehen (Stufe `festzusetzende` 28.756 EUR).
#[tokio::test]
async fn g2_p35_gewerbesteuer_kette_endet_bei_der_zahl() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g2_p35_gewerbesteuer_aenderungen()),
    )
    .await;
    erwarte_kette("g2", &a, 2_875_600, [108_770, 108_734, 34_756, 28_756]);
}

/// § 31 mit einem Kind, Kindergeld gewinnt: der fb=0-Lauf bestimmt die Steuer und muss seinen eigenen
/// Endstand hinterlassen -- `SolzInfo` traegt nur den Kinderfreibetrag-Lauf. `p31` nennt den Sieger.
#[tokio::test]
async fn g3_kind_kindergeld_siegt_kette_kommt_aus_dem_ohne_freibetrag_lauf() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g3_kind_kindergeld_aenderungen()),
    )
    .await;
    erwarte_kette("g3", &a, 132_700, [18_770, 18_734, 1_327, 1_327]);
    assert_eq!(a["kette"]["p31"]["guenstiger"], "kindergeld", "{a}");
    assert_eq!(a["kette"]["p31"]["kindergeld"].as_i64(), Some(3060), "{a}");
}

/// § 31 mit einem Kind, der Kinderfreibetrag gewinnt (300.000 EUR, Zusammenveranlagung) UND § 32d: der
/// Kinderfreibetrag-Lauf hinterlaesst seinen Endstand mit der Abgeltungsteuer (+7.000 EUR), das Kindergeld
/// ist darin hinzugerechnet. Vor p24a verwarf `setze_kette` die Kette hier still (`null`).
#[tokio::test]
async fn g4_kind_freibetrag_p32d_kette_kommt_aus_dem_freibetrag_lauf() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g4_kind_freibetrag_p32d_aenderungen()),
    )
    .await;
    erwarte_kette("g4", &a, 10_965_600, [298_770, 289_098, 99_596, 109_656]);
    assert_eq!(a["kette"]["p31"]["guenstiger"], "freibetraege", "{a}");
    assert_eq!(a["kette"]["p31"]["kindergeld"].as_i64(), Some(3060), "{a}");
}

/// Rentner-Zweig, 20.000 EUR Rente + 30.000 EUR Kapitalertraege: 1:1 gesamt-Praezedenz, eigener
/// Schliesser in `rentner_tarif::festzusetzende` (Kapital VOR § 32b, ein einziger Rueckgabepunkt).
#[tokio::test]
async fn r1_rentner_p32d_kette_traegt_die_abgeltungsteuer() {
    let a = ergebnis(
        "rentner_gesamt",
        &mit(kegel_rentner(), r1_rentner_p32d_aenderungen()),
    )
    .await;
    erwarte_kette("r1", &a, 806_100, [16_598, 16_562, 811, 8_061]);
}

/// Rentner-Zweig mit Kind (Kindergeld gewinnt) UND § 32d: der Kinderzweig des Rentner-Zweigs speist die
/// Kette ebenfalls aus dem Endstand des fb=0-Laufs, nicht aus dem Rentner-`g`. `p31` nennt den Sieger.
#[tokio::test]
async fn r2_rentner_kind_p32d_kette_kommt_aus_dem_endstand() {
    let a = ergebnis(
        "rentner_gesamt",
        &mit(kegel_rentner(), r2_rentner_kind_p32d_aenderungen()),
    )
    .await;
    erwarte_kette("r2", &a, 806_100, [16_598, 16_562, 811, 8_061]);
    assert_eq!(a["kette"]["p31"]["guenstiger"], "kindergeld", "{a}");
    assert_eq!(a["kette"]["p31"]["kindergeld"].as_i64(), Some(3060), "{a}");
}

/// Rentner-Gegenprobe ohne Sonderregel (20.000 EUR Rente): die Kette lief schon vor p24a und endet unveraendert
/// bei der Zahl. Basis der Rentner-Faelle dieser Datei.
#[tokio::test]
async fn r0_rentner_gegenprobe_kette_endet_bei_der_zahl() {
    let a = ergebnis("rentner_gesamt", &kegel_rentner()).await;
    erwarte_kette("r0", &a, 81_100, [16_598, 16_562, 811, 811]);
    assert!(a["kette"]["p31"].is_null(), "kinderlos: kein p31");
}

/// § 34 `EStG` im Gesamt-Zweig (120.000 EUR Veraeusserungsgewinn, einzeln): der modifizierte Tarif steckt im
/// finalen `g2` (`tarif_modifiziert`). Aus dem Basis-`g` endete die Kette um die § 34-Ermaessigung zu hoch
/// (34.338 statt 30.358 EUR). Die Stufe `tarifliche` haelt das `g2` fest.
#[tokio::test]
async fn g5_vg_einzel_p34_kette_traegt_die_tarifermaessigung() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g5_vg_einzel_p34_aenderungen()),
    )
    .await;
    erwarte_kette("g5", &a, 4_458_400, [133_770, 133_734, 44_584, 44_584]);
}

/// § 34 Abs. 3 `EStG` gilt bis EINSCHLIESSLICH 5.000.000 EUR (`netto_vg <= 5_000_000` in `p34_chooser`; Python
/// `0 < netto_vg <= 5_000_000`, `bescheid_zweige.py:873`): genau 5 Mio EUR mit Antrag und Berufsunfaehigkeit rechnen den
/// ermaessigten Durchschnittssatz (1.263.270 EUR), nicht die Fuenftelung (Python ohne Antrag: 2.230.219 EUR). Ueber 5 Mio
/// sperrt `abs3_ueber_5mio_offen` (gemessen bei 5.000.001 EUR); die Grenze selbst ist ueber `GET /ergebnis` also nur auf
/// genau 5.000.000 EUR erreichbar.
///
/// Die Mutationsmessung (`bescheid-elster-mutation`, T04) liess `<=` -> `<` in jedem Lauf gruen, auch mit `PARITY=1`; der
/// Witness (`rentner_veraeusserungsgewinn` 500.000.000 Cent) wich bei `festzusetzende_est_gesamt` und `_rentner` ab.
/// Erwartung aus dem Python-Server (`api.ergebnis` im selben Prozess, 2026-10-04, `0197bf76`).
#[tokio::test]
async fn g6_p34_abs3_gilt_bis_einschliesslich_5_mio() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g6_p34_abs3_genau_5_mio_aenderungen()),
    )
    .await;
    erwarte_kette(
        "g6",
        &a,
        126_327_000,
        [5_058_770, 5_058_734, 1_263_270, 1_263_270],
    );
}

/// § 31 `EStG` am Gleichstand: kostet der Kinderfreibetrag-Lauf plus Kindergeld GENAU so viel wie der Lauf ohne (hier
/// 12.986 + 3.060 = 16.046 EUR), bleibt es beim Kindergeld (`est_mit + kg < est_ohne` ist strikt; Python
/// `_fb_guenstiger = _est_mit_fb + _kg_kind < _est_ohne_fb`, `bescheid_zweige.py:1014`). Die Kette zeigt dann den Lauf
/// ohne Freibetrag (zvE 84.346 EUR, nicht 74.746 EUR) und `p31` nennt das Kindergeld.
///
/// Gepinnt ist das VERHALTEN DES ORAKELS am Gleichstand, nicht seine gesetzliche Richtigkeit: ob § 31 `EStG` bei
/// Gleichstand das Kindergeld oder den Freibetrag meint, ist nicht geprueft und bleibt offen. Faellt die Entscheidung
/// anders aus, aendert sich diese Erwartung zusammen mit Python.
///
/// Die Mutationsmessung (T15) liess `<` -> `<=` in jedem Lauf gruen, auch mit `PARITY=1`: die Zufallsfaelle treffen den
/// Gleichstand nicht (118 von 3.000 untersuchten zvE-Werten, Zusammenveranlagung, ein Kind). Die Zahl selbst ist an dieser
/// Stelle gleich; die Mutation verschiebt `p31.guenstiger` auf "freibetraege" und die Stufen zvE/tarifliche der Kette
/// (74.746 und 12.986 statt 84.346 und 16.046 EUR).
/// Erwartung aus dem Python-Server (`api.ergebnis` im selben Prozess, 2026-10-04, `0197bf76`).
#[tokio::test]
async fn g7_kind_gleichstand_haelt_das_kindergeld() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g7_kind_gleichstand_aenderungen()),
    )
    .await;
    erwarte_kette("g7", &a, 1_604_600, [84_418, 84_346, 16_046, 16_046]);
    assert_eq!(a["kette"]["p31"]["guenstiger"], "kindergeld", "{a}");
    assert_eq!(a["kette"]["p31"]["kindergeld"].as_i64(), Some(3060), "{a}");
}

/// § 34 `EStG` im Rentner-Zweig (20.000 EUR Rente + 120.000 EUR Veraeusserungsgewinn): derselbe Fall mit dem
/// `g2` des Rentner-Laufs. Ohne den Endstand stuende hier die `tarifliche` Stufe um die Ermaessigung zu hoch.
#[tokio::test]
async fn r3_rentner_p34_kette_traegt_die_tarifermaessigung() {
    let a = ergebnis(
        "rentner_gesamt",
        &mit(kegel_rentner(), r3_rentner_p34_aenderungen()),
    )
    .await;
    erwarte_kette("r3", &a, 2_051_100, [91_598, 91_562, 20_511, 20_511]);
}

/// § 35 `EStG` im Rentner-Zweig (50.000 EUR Gewerbegewinn, Messbetrag 1.500, Hebesatz 400): die Anrechnung
/// steckt in `steuerermaessigungen` des finalen `g2`; die Stufe `festzusetzende` kommt aus dem Endwert.
#[tokio::test]
async fn r4_rentner_p35_kette_endet_bei_der_zahl() {
    let a = ergebnis(
        "rentner_gesamt",
        &mit(kegel_rentner(), r4_rentner_p35_aenderungen()),
    )
    .await;
    erwarte_kette("r4", &a, 1_105_000, [66_598, 66_562, 17_050, 11_050]);
}

/// § 31 im Rentner-Zweig, der Kinderfreibetrag gewinnt (300.000 EUR Rente, Zusammenveranlagung, 1 Kind): die
/// Kette kommt aus dem Freibetrag-Lauf, das Kindergeld ist hinzugerechnet. Der Kindergeld-Sieg (r2) laesst
/// den Freibetrag-Lauf unbenutzt; erst dieser Fall haelt ihn.
#[tokio::test]
async fn r5_rentner_kind_freibetrag_kette_kommt_aus_dem_freibetrag_lauf() {
    let a = ergebnis(
        "rentner_gesamt",
        &mit(kegel_rentner(), r5_rentner_kind_freibetrag_aenderungen()),
    )
    .await;
    erwarte_kette("r5", &a, 8_234_000, [250_398, 240_726, 79_280, 82_340]);
    assert_eq!(a["kette"]["p31"]["guenstiger"], "freibetraege", "{a}");
    assert_eq!(a["kette"]["p31"]["kindergeld"].as_i64(), Some(3060), "{a}");
}

// ---- Zahlen-Faelle der Zweige (k9, Auftrag 4): ohne PARITY=1, ohne Python ---------------------------------------
//
// Jede Zahl unten ist die Ausgabe des Python-Servers (`api.ergebnis` im selben Prozess, Wegwerf-Wurzel, 2026-10-04,
// main 2e4cc91d). Die Ereignislisten sind die Feldlisten dieser Datei (`kegel_*()` plus Aenderungen). HERKUNFT:
// `python3 tools/parity/kette_erwartung.py SCHEIBE BASIS_FN AENDERUNGS_FN` liest sie aus diesem Quelltext, meldet, wenn
// `tests/_kegel.py::kegel_fuer` etwas hinzufuegt oder aendert, und druckt die Werte unten (Beispiel:
// `kette_erwartung.py an_gesamt kegel_an_gesamt a4_partner_kv_pv_aenderungen`). Das Skript reproduziert auch die aelteren
// Konstanten (g4, g5, r3).

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

/// Wie `g5`, aber der Verkaeufer ist dauernd berufsunfaehig (§ 34 Abs. 3 Satz 1: berechtigt) und stellt KEINEN Antrag.
fn g8_p34_berechtigt_ohne_antrag_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(12_000_000)),
        ("dauernd_berufsunfaehig", json!(true)),
    ]
}

/// § 36: Lohnsteuer, Vorauszahlungen und die drei Kapitalertragsteuer-Posten, alle verschieden hoch.
fn g9_abschlusszahlung_aenderungen() -> Paare {
    vec![
        ("p36_lohnsteuer", json!(800_000)),
        ("p36_vorauszahlungen", json!(100_000)),
        ("p36_kapitalertragsteuer", json!(30_000)),
        ("p36_kapitalertragsteuer_solz", json!(1_650)),
        ("p36_kapitalertragsteuer_kist", json!(2_700)),
    ]
}

/// § 16 Abs. 4: 20.000 EUR Veraeusserungsgewinn bleiben unter dem Freibetrag (45.000 EUR), `netto_vg` ist 0, kein Verlust.
fn g10_vg_unter_freibetrag_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(2_000_000)),
    ]
}

/// § 3 Nr. 72: 10.000 EUR steuerfreie PV-Einnahmen mindern nur einen POSITIVEN Gewinn; hier ist der Gewinn -10.000 EUR.
fn g11_pv_freistellung_bei_verlust_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(-1_000_000)),
        ("pv_einnahmen", json!(1_000_000)),
        ("pv_bruttoleistung_kwp", json!(10)),
        ("pv_anzahl_einheiten", json!(1)),
        ("pv_auf_gebaeude", json!(true)),
    ]
}

/// Zusammenveranlagung, 20.000 EUR Veraeusserungsgewinn des Ehegatten unter dessen Freibetrag: `netto_vg_partner` ist 0.
fn g12_partner_vg_unter_freibetrag_aenderungen() -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(1_000_000)),
        ("rentner_veraeusserungsgewinn_partner", json!(2_000_000)),
        ("rentner_alter_55_oder_berufsunfaehig_partner", json!(true)),
        ("rentner_freibetrag_erstmalig_partner", json!(true)),
    ]
}

/// § 32d Abs. 6: 10.000 EUR Lohn und 30.000 EUR Kapitalertraege, die tarifliche Steuer ist niedriger als die Abgeltung
/// (Guenstigerpruefung gewinnt); evangelisch, Bayern.
fn g13_guenstigerpruefung_gewinnt_aenderungen() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(1_000_000)),
        ("kein_kap", json!(false)),
        ("kap_kapitalertraege", json!(3_000_000)),
        ("kist_konfession", json!("evangelisch")),
        ("kist_bundesland", json!("bayern")),
        ("kist_gezahlt", json!(0)),
        ("kist_erstattet", json!(0)),
    ]
}

/// § 9 Abs. 1 Nr. 6: Arbeitsmittel zu GENAU 800,00 EUR mit gewaehltem Sofortabzug, dazu 30 km Pendelstrecke an 220 Tagen
/// (damit die Werbungskosten ueber dem Pauschbetrag liegen).
fn g14_arbeitsmittel_genau_800_aenderungen() -> Paare {
    vec![
        ("ep_arbeitstage", json!(220)),
        ("ep_entfernung_km", json!(30)),
        ("am_anschaffungskosten", json!(80_000)),
        ("arbeitsmittel_nutzungsdauer", json!(3)),
        ("am_afa_ist_anschaffungsjahr", json!(false)),
        ("am_gwg_sofortabzug_gewaehlt", json!(true)),
    ]
}

/// § 6 Abs. 2 `EStG`: ein GWG zu GENAU 800,00 EUR netto (80.000 ct) zaehlt noch als Sofortabzug (EUeR-Weg, Verzeichnis gefuehrt).
fn g15_gwg_genau_800_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("betriebseinnahmen", json!(5_000_000)),
        ("sonstige_betriebsausgaben", json!(0)),
        ("afa_jahresbetrag", json!(0)),
        ("gwg_anschaffungskosten_netto", json!(80_000)),
        ("gwg_verzeichnis_ab_250", json!(true)),
        ("gwg_bewegliches_selbstaendig_nutzbar", json!(true)),
        ("gwg_netto_ohne_vorsteuer", json!(true)),
    ]
}

/// § 35 darf die Steuer nicht unter 0 druecken, auch nicht im nachgezogenen Deckel 3 nach § 32b: Zusammenveranlagung, der
/// Ehegatte hat 50.000 EUR Gewerbegewinn (Messbetrag 10.000 EUR, Hebesatz 400), dazu 30.000 EUR Progressionseinkuenfte. Die
/// Anrechnung (Messbetrag x 4, gezahlte `GewSt`, Deckel 3) und die Steuerermaessigung § 35a (20.000 EUR haushaltsnahe Dienstleistung)
/// uebersteigen zusammen die Steuer; die festzusetzende Steuer ist 0.
/// (Der Weg ueber die `GewSt` der Person A sperrt `p32b_kombi_offen`; die des Ehegatten nicht.)
fn g17_p35_credit_uebersteigt_die_steuer_aenderungen() -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(0)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(0)),
        ("gewinn_betriebsart_partner", json!("gewerbe")),
        ("einkuenfte_gewinn_partner", json!(5_000_000)),
        ("gewst_messbetrag_partner", json!(1_000_000)),
        ("gewst_hebesatz_partner", json!(400)),
        ("p32b_progressionseinkuenfte", json!(3_000_000)),
        ("hh_dienstleistungen", json!(2_000_000)),
        ("hh_in_eu_ewr", json!(true)),
        ("hh_rechnung_unbar", json!(true)),
    ]
}

/// § 35 Abs. 1 `EStG`: Nenner 0. Beide Ehegatten haben keine positiven Einkuenfte ausser dem Gewerbegewinn des Partners, den
/// der Verlust der Person A (-20.000 EUR) aufhebt; Messbetrag und Zaehler sind positiv, die Summe der positiven Einkuenfte (der
/// Nenner) ist 0. Python (`nenner > 0`): keine Anrechnung, keine Division; Steuer 0, zvE -72 EUR.
fn g16_p35_nenner_null_aenderungen() -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(0)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(-2_000_000)),
        ("gewinn_betriebsart_partner", json!("gewerbe")),
        ("einkuenfte_gewinn_partner", json!(2_000_000)),
        ("gewst_messbetrag_partner", json!(50_000)),
        ("gewst_hebesatz_partner", json!(400)),
    ]
}

fn a1_vorsorge_unter_hoechstbetrag_aenderungen() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("vor_an_anteil_rv", json!(451_050)),
        ("vor_ag_anteil_rv", json!(451_050)),
    ]
}

fn a2_vorsorge_ueber_hoechstbetrag_aenderungen() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("vor_an_anteil_rv", json!(4_000_000)),
        ("vor_ag_anteil_rv", json!(500_000)),
    ]
}

/// Der steuerfreie AG-Anteil (35.000 EUR) liegt ueber dem Hoechstbeitrag: der Abzug ist 0, nie negativ.
fn a3_ag_anteil_ueber_hoechstbetrag_aenderungen() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("vor_ag_anteil_rv", json!(3_500_000)),
        ("basis_kv", json!(200_000)),
        ("basis_pv", json!(40_000)),
    ]
}

/// Zusammenveranlagung, KV/PV-Beitraege nur beim Ehegatten.
fn a4_partner_kv_pv_aenderungen() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn_partner", json!(3_000_000)),
        ("basis_kv_partner", json!(150_000)),
        ("basis_pv_partner", json!(30_000)),
    ]
}

/// Keine Pendelstrecke: die § 101-Mobilitaetspraemie wird nicht gerechnet (null, nicht 0).
fn a5_ohne_pendelstrecke_aenderungen() -> Paare {
    vec![("bruttoarbeitslohn", json!(6_000_000))]
}

/// Grund `bestaetigt` und die gezahlte Steuer in CENT (Scheiben ohne Kette, z. B. `an_gesamt`).
fn erwarte_zahl(name: &str, a: &Value, zahl_cent: i64) {
    assert_eq!(
        a["grund"], "bestaetigt",
        "{name}: Positivkontrolle -- Kegel vollstaendig? {a}"
    );
    assert_eq!(
        a["zahl_cent"].as_i64(),
        Some(zahl_cent),
        "{name}: Zahl -- {a}"
    );
}

/// § 34 Abs. 3 verlangt den ANTRAG (Satz 1 "auf Antrag"): wer berechtigt ist (dauernd berufsunfaehig), aber keinen
/// Antrag stellt, bekommt die Fuenftelung nach Abs. 1 (Python: dieselbe Zahl wie `g5`, 4.458.400 ct). Mit gestelltem Antrag
/// waeren es 2.813.600 ct (gemessen mit denselben Feldern plus `antrag_ermaessigter_satz` = true).
///
/// Schliesst `p34_chooser`: `ist_true(wert(l.f, "antrag_ermaessigter_satz"))` durch `true` ersetzt -- ein von main
/// gefundener Ueberlebender der 14 Tests (T04e); `g6` hat den Antrag immer gesetzt.
#[tokio::test]
async fn g8_p34_berechtigt_ohne_antrag_bleibt_fuenftelung() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g8_p34_berechtigt_ohne_antrag_aenderungen()),
    )
    .await;
    erwarte_kette("g8", &a, 4_458_400, [133_770, 133_734, 44_584, 44_584]);
}

/// § 36 Abs. 2 `EStG`: Abschlusszahlung = Steuer minus Lohnsteuer, Vorauszahlungen und die drei
/// Kapitalertragsteuer-Posten. Python: 1.392.400 - 800.000 - 100.000 - 30.000 - 1.650 - 2.700 = 458.050, auf ganze Euro
/// 458.000 ct. Die Posten sind verschieden hoch; vertauscht `abschlusszahlung_cent` zwei von ihnen (z. B. nimmt die
/// Kapitalertragsteuer als Vorauszahlung), aendert sich die Zahl.
#[tokio::test]
async fn g9_abschlusszahlung_zieht_alle_fuenf_anrechnungen_ab() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g9_abschlusszahlung_aenderungen()),
    )
    .await;
    erwarte_kette("g9", &a, 1_392_400, [58_770, 58_734, 13_924, 13_924]);
    assert_eq!(a["abschlusszahlung_cent"].as_i64(), Some(458_000), "{a}");
}

/// § 16 Abs. 4 `EStG`: ein Veraeusserungsgewinn UNTER dem Freibetrag (20.000 EUR gegen 45.000 EUR) laesst `netto_vg` bei 0,
/// er erzeugt keinen Verlust. Python: dieselbe Zahl wie die Gegenprobe `g0` (1.392.400 ct, `GdE` 58.770 EUR).
#[tokio::test]
async fn g10_vg_unter_dem_freibetrag_erzeugt_keinen_verlust() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g10_vg_unter_freibetrag_aenderungen()),
    )
    .await;
    erwarte_kette("g10", &a, 1_392_400, [58_770, 58_734, 13_924, 13_924]);
}

/// § 3 Nr. 72 `EStG`: die steuerfreie PV-Einnahme (10.000 EUR) mindert nur einen positiven Gewinn. Bei einem Gewinn von
/// -10.000 EUR bleibt der Verlust stehen (`GdE` 48.770 EUR = 60.000 - 1.230 - 10.000); ohne die Untergrenze 0 wuerde der
/// Verlust um die PV-Einnahme "geheilt".
#[tokio::test]
async fn g11_pv_freistellung_mindert_keinen_verlust() {
    let a = ergebnis(
        "gesamt",
        &mit(
            kegel_gesamt(),
            g11_pv_freistellung_bei_verlust_aenderungen(),
        ),
    )
    .await;
    erwarte_kette("g11", &a, 1_024_500, [48_770, 48_734, 10_245, 10_245]);
}

/// § 16 Abs. 4 `EStG` je Person: bleibt der Veraeusserungsgewinn des EHEGATTEN unter seinem Freibetrag, ist sein
/// `netto_vg_partner` 0 und kein Phantom-Verlust. Python: dieselbe Zahl wie mit Gewinn 0 (1.114.400 ct).
#[tokio::test]
async fn g12_partner_vg_unter_dem_freibetrag_erzeugt_keinen_verlust() {
    let a = ergebnis(
        "gesamt",
        &mit(
            kegel_gesamt(),
            g12_partner_vg_unter_freibetrag_aenderungen(),
        ),
    )
    .await;
    erwarte_kette("g12", &a, 1_114_400, [68_770, 68_698, 11_144, 11_144]);
}

/// § 32d Abs. 6 `EStG`: gewinnt die Guenstigerpruefung (tarifliche Steuer 0 gegen 7.250 EUR Abgeltung), rechnet die Kette
/// mit 6.606 EUR (Kapitalsteuer nach Sparer-Pauschbetrag) und die Kapital-KiSt-Ermaessigung (Abs. 1 Satz 3-5) greift NICHT:
/// sie gilt nur, wenn die Abgeltungsteuer anfaellt. Die Kirchensteuer bleibt 0.
#[tokio::test]
async fn g13_guenstigerpruefung_gewinnt_ohne_kist_ermaessigung() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g13_guenstigerpruefung_gewinnt_aenderungen()),
    )
    .await;
    erwarte_kette("g13", &a, 660_600, [8_770, 8_734, 0, 6_606]);
    assert_eq!(a["solz_cent"].as_i64(), Some(36_333), "{a}");
    assert_eq!(a["kist_cent"].as_i64(), Some(0), "{a}");
}

/// § 9 Abs. 1 Nr. 6 `EStG`, Grenze 800,00 EUR GENAU: bei gewaehltem Sofortabzug zaehlen die 800 EUR voll als Werbungskosten
/// (`GdE` 57.044 EUR = 57.844 ohne Arbeitsmittel - 800), die `AfA` ist dann aus. Python `am_anschaffungskosten <= 80000`.
/// Ein Cent mehr (800,01 EUR) waere `AfA`-pflichtig; die Sperre `arbeitsmittel_afa_ueber_gwg_offen` stuende ohne Wahl davor.
#[tokio::test]
async fn g14_arbeitsmittel_zu_genau_800_eur_sind_sofortabzug_ohne_afa() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g14_arbeitsmittel_genau_800_aenderungen()),
    )
    .await;
    erwarte_kette("g14", &a, 1_326_300, [57_044, 57_008, 13_263, 13_263]);
}

/// § 6 Abs. 2 `EStG`, Grenze 800,00 EUR GENAU: ein GWG zu 80.000 ct netto ist noch Sofortabzug (`GdE` 107.970 EUR = 108.770
/// ohne GWG - 800); Python `netto > 80000` gibt erst darueber 0. Gegenprobe ohne GWG: 3.475.600 ct, mit GWG 3.442.000 ct.
#[tokio::test]
async fn g15_gwg_zu_genau_800_eur_ist_noch_sofortabzug() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g15_gwg_genau_800_aenderungen()),
    )
    .await;
    erwarte_kette("g15", &a, 3_442_000, [107_970, 107_934, 34_420, 34_420]);
}

/// Siehe `g16_p35_nenner_null_aenderungen`: Nenner 0, Python rechnet ohne Anrechnung (Steuer 0, `GdE` 0, zvE -72 EUR). Ein
/// `nenner >= 0` liefe in die Division durch 0.
#[tokio::test]
async fn g16_p35_mit_nenner_null_rechnet_ohne_anrechnung() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g16_p35_nenner_null_aenderungen()),
    )
    .await;
    erwarte_kette("g16", &a, 0, [0, -72, 0, 0]);
}

/// Siehe `g17_p35_credit_uebersteigt_die_steuer_aenderungen`: Python: festzusetzende Steuer 0 (`GdE` 50.000, zvE 49.928,
/// tarifliche 5.834 EUR). Ohne die Untergrenze 0 nach der Anrechnung stuende hier eine negative Steuer (gemessen am Mutanten:
/// -4.000 EUR, das ist die Steuerermaessigung § 35a).
#[tokio::test]
async fn g17_p35_credit_macht_die_steuer_nie_negativ() {
    let a = ergebnis(
        "gesamt",
        &mit(
            kegel_gesamt(),
            g17_p35_credit_uebersteigt_die_steuer_aenderungen(),
        ),
    )
    .await;
    erwarte_kette("g17", &a, 0, [50_000, 49_928, 5_834, 0]);
}

/// § 10 Abs. 3 `EStG` (Scheibe `an_gesamt`, nur Arbeitnehmer): der Abzug ist `min(Beitraege, Hoechstbeitrag) - AG-Anteil`;
/// 9.021 EUR Beitraege liegen UNTER dem Hoechstbeitrag. Python: 1.223.400 ct (statt 1.392.400 ct ohne Vorsorge).
#[tokio::test]
async fn a1_vorsorge_unter_dem_hoechstbetrag() {
    let a = ergebnis(
        "an_gesamt",
        &mit(
            kegel_an_gesamt(),
            a1_vorsorge_unter_hoechstbetrag_aenderungen(),
        ),
    )
    .await;
    erwarte_zahl("a1", &a, 1_223_400);
}

/// Wie `a1`, aber 45.000 EUR Beitraege: der Abzug wird auf den Hoechstbeitrag gedeckelt. Python: 559.500 ct.
#[tokio::test]
async fn a2_vorsorge_ueber_dem_hoechstbetrag_wird_gedeckelt() {
    let a = ergebnis(
        "an_gesamt",
        &mit(
            kegel_an_gesamt(),
            a2_vorsorge_ueber_hoechstbetrag_aenderungen(),
        ),
    )
    .await;
    erwarte_zahl("a2", &a, 559_500);
}

/// Der steuerfreie AG-Anteil (35.000 EUR) ist groesser als der Hoechstbeitrag: der Altersvorsorge-Abzug ist 0, nicht negativ.
/// Mit KV/PV-Beitraegen (2.400 EUR) dazu, denn ohne sie bleibt ein negativer Abzug folgenlos (gemessen: der Mutant ohne
/// `max(0)` blieb gruen): er wuerde die Basis-Beitraege auffressen. Python: 1.302.200 ct, gleich der Gegenprobe nur mit den
/// KV/PV-Beitraegen.
#[tokio::test]
async fn a3_ag_anteil_ueber_dem_hoechstbetrag_gibt_keinen_negativen_abzug() {
    let a = ergebnis(
        "an_gesamt",
        &mit(
            kegel_an_gesamt(),
            a3_ag_anteil_ueber_hoechstbetrag_aenderungen(),
        ),
    )
    .await;
    erwarte_zahl("a3", &a, 1_302_200);
}

/// Zusammenveranlagung: die KV/PV-Beitraege des Ehegatten (1.800 EUR) gehen in die gemeinsamen Sonderausgaben ein.
/// Python: 1.650.400 ct; ohne die Partner-Beitraege 1.707.600 ct.
#[tokio::test]
async fn a4_partner_kv_pv_geht_in_die_gemeinsamen_sonderausgaben() {
    let a = ergebnis(
        "an_gesamt",
        &mit(kegel_an_gesamt(), a4_partner_kv_pv_aenderungen()),
    )
    .await;
    erwarte_zahl("a4", &a, 1_650_400);
}

/// § 101 `EStG`: ohne Pendelstrecke (0 km) wird keine Mobilitaetspraemie gerechnet; die Antwort traegt `null`, nicht 0.
/// Python: `mobilitaetspraemie_cent` null (mit 30 km waere es eine Zahl).
#[tokio::test]
async fn a5_ohne_pendelstrecke_keine_mobilitaetspraemie() {
    let a = ergebnis(
        "an_gesamt",
        &mit(kegel_an_gesamt(), a5_ohne_pendelstrecke_aenderungen()),
    )
    .await;
    erwarte_zahl("a5", &a, 1_392_400);
    assert!(a["mobilitaetspraemie_cent"].is_null(), "{a}");
}

/// `g6` im Rentner-Zweig: 20.000 EUR Rente, dazu genau 5.000.000 EUR Veraeusserungsgewinn, Antrag, berufsunfaehig.
fn r6_rentner_p34_abs3_genau_5_mio_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(500_000_000)),
        ("antrag_ermaessigter_satz", json!(true)),
        ("dauernd_berufsunfaehig", json!(true)),
    ]
}

/// `g7` im Rentner-Zweig: Zusammenveranlagung, ein Kind, 101.300 EUR Rente (sonst nichts): Kinderfreibetrag-Lauf plus
/// Kindergeld und Lauf ohne Freibetrag stehen gleich.
fn r7_rentner_kind_gleichstand_aenderungen() -> Paare {
    vec![
        ("rentner_jahresrente", json!(10_130_000)),
        ("veranlagung", json!("zusammen")),
        ("fam_anzahl_kinder", json!(1)),
    ]
}

/// § 34 Abs. 3 `EStG` bis EINSCHLIESSLICH 5 Mio EUR auch im Rentner-Zweig (`g6` mit 20.000 EUR Rente): derselbe
/// `p34_chooser`, Python 125.006.800 ct. Der Mutant `<=` -> `<` rechnet wie bei `g6` die Fuenftelung (gemessen: 218.778.600 ct).
#[tokio::test]
async fn r6_rentner_p34_abs3_gilt_bis_einschliesslich_5_mio() {
    let a = ergebnis(
        "rentner_gesamt",
        &mit(
            kegel_rentner(),
            r6_rentner_p34_abs3_genau_5_mio_aenderungen(),
        ),
    )
    .await;
    erwarte_kette(
        "r6",
        &a,
        125_006_800,
        [5_016_598, 5_016_562, 1_250_068, 1_250_068],
    );
}

/// § 31 `EStG` am Gleichstand im Rentner-Zweig (`g7` mit Rente statt Gewinn): `est_ohne` 16.066 EUR = `est_mit` 13.006 EUR +
/// Kindergeld 3.060 EUR, Python: Kindergeld, Kette ohne Freibetrag (zvE 84.411 EUR). Wie bei `g7` pinnt der Test nur das
/// Verhalten des Orakels; die gesetzliche Richtigkeit am Gleichstand ist offen.
#[tokio::test]
async fn r7_rentner_kind_gleichstand_haelt_das_kindergeld() {
    let a = ergebnis(
        "rentner_gesamt",
        &mit(kegel_rentner(), r7_rentner_kind_gleichstand_aenderungen()),
    )
    .await;
    erwarte_kette("r7", &a, 1_606_600, [84_483, 84_411, 16_066, 16_066]);
    assert_eq!(a["kette"]["p31"]["guenstiger"], "kindergeld", "{a}");
    assert_eq!(a["kette"]["p31"]["kindergeld"].as_i64(), Some(3060), "{a}");
}
