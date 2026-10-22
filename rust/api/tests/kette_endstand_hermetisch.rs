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
    ergebnis_vorlaeufig(scheibe, paare, &[]).await
}

/// Wie `ergebnis`, aber die Felder in `vorlaeufig` schreibt der Fall ohne `signal_2` als `vorlaeufig`.
async fn ergebnis_vorlaeufig(
    scheibe: &str,
    paare: &[(&'static str, Value)],
    vorlaeufig: &[&str],
) -> Value {
    let d = dienst();
    let kopf = json!({"fall_id": "kette", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (zustand, signal_2) = if vorlaeufig.contains(feld) {
            ("vorlaeufig", Value::Null)
        } else {
            ("bestaetigt", json!(format!("ok@{feld}")))
        };
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": zustand, "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": signal_2},
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
//
// AUSNAHME (Abweichung Nr. 78): a1 bis a4 tragen die von Hand gerechnete Zahl von Rust (der Pauschbetrag steht neben der Vorsorge), nicht
// die von Python; a6 hat kein Python-Gegenstueck. Die Rechnung steht am jeweiligen Fall.

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

/// Abweichung Nr. 78, § 101 `EStG`: das zvE der Mobilitaetspraemie kommt aus demselben Abzug wie die Steuer, also Vorsorge PLUS Pauschbetrag.
/// 14.000 EUR Lohn, 30 km an 220 Tagen ohne Kfz: Entfernungspauschale 220 x (20 x 0,30 + 10 x 0,38) = 2.156 EUR (§ 9 Abs. 1 Nr. 4), ab dem
/// 21. km 220 x 10 x 0,38 = 836 EUR; sie uebersteigt den Arbeitnehmer-Pauschbetrag (1.230 EUR) um 926 EUR, die Grenze aus Satz 3 greift nicht.
/// Einkuenfte 14.000 - 2.156 = 11.844, Abzug 36 + 100 (Rentenbeitrag) = 136, zvE 11.708, Grundfreibetrag 2025 12.096: Unterschreitung 388 EUR
/// (kleiner als 836, also bestimmt sie die Bemessung), Praemie 14 % = 5.432 ct; Steuer 0. Die alte Lesart max(100; 36) gaebe zvE 11.744,
/// Unterschreitung 352, Praemie 4.928 ct.
fn a6_praemie_nach_abzug_mit_pauschbetrag_aenderungen() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(1_400_000)),
        ("ep_arbeitstage", json!(220)),
        ("ep_entfernung_km", json!(30)),
        ("vor_an_anteil_rv", json!(10_000)),
    ]
}

#[tokio::test]
async fn a6_mobilitaetspraemie_rechnet_mit_dem_abzug_inklusive_pauschbetrag() {
    let a = ergebnis(
        "an_gesamt",
        &mit(kegel_an_gesamt(), a6_praemie_nach_abzug_mit_pauschbetrag_aenderungen()),
    )
    .await;
    erwarte_zahl("a6", &a, 0);
    assert_eq!(a["mobilitaetspraemie_cent"].as_i64(), Some(5_432), "{a}");
}

/// Abweichung Nr. 88, Scheibe `an_gesamt`: 900,00 EUR brutto bei 19 % (netto 756,30) liegen unter der Netto-Grenze, der volle Bruttopreis
/// ist Sofortabzug. 60.000 EUR Lohn, 30 km an 220 Tagen (Entfernungspauschale 2.156 EUR, ueber dem Pauschbetrag): Einkuenfte
/// 60.000 - 2.156 - 900 = 56.944, zvE 56.944 - 36 = 56.908, Steuer 13.226 EUR = 1.322.600 ct (wie `g29` in `gesamt`). Von Hand gerechnet
/// (Skript `handrechnung.py`, Eichung an `a5`: zvE 58.734 ergibt 1.392.400 ct). Der Abzug kommt NACH dem Catala-Scope dazu
/// (`mit_sofortabzug_im_band`): der Scope zaehlt 900 EUR gegen 800 EUR und lieferte 0.
#[tokio::test]
async fn a7_arbeitsmittel_900_eur_brutto_bei_19_prozent_sind_sofortabzug() {
    let a = ergebnis(
        "an_gesamt",
        &mit(
            kegel_an_gesamt(),
            mit(
                vec![("bruttoarbeitslohn", json!(6_000_000))],
                arbeitsmittel_brutto_aenderungen(90_000, "satz_19"),
            ),
        ),
    )
    .await;
    erwarte_zahl("a7", &a, 1_322_600);
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

/// Abweichung Nr. 38: Bei Einzelveranlagung zaehlt ein bestaetigter Wert im Feld des Ehegatten (`p36_lohnsteuer_partner`, hier
/// 3.000 EUR) nicht. Wie `g9` mit demselben Python-Wert: Steuer 1.392.400 ct, Abschlusszahlung 458.000 ct. Wuerde die Rechnung den
/// Wert des Ehegatten ohne Zusammenveranlagung abziehen, kaeme 158.000 ct heraus.
#[tokio::test]
async fn g9_abschlusszahlung_einzeln_ignoriert_die_lohnsteuer_des_ehegatten() {
    let mit_ehegatte = mit(
        g9_abschlusszahlung_aenderungen(),
        vec![("p36_lohnsteuer_partner", json!(300_000))],
    );
    let a = ergebnis("gesamt", &mit(kegel_gesamt(), mit_ehegatte)).await;
    erwarte_kette("g9e", &a, 1_392_400, [58_770, 58_734, 13_924, 13_924]);
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
///
/// ABWEICHUNG Nr. 78 (a1 bis a4): der Sonderausgaben-Pauschbetrag (36 EUR, zusammen 72 EUR; § 10c Satz 1 und 2 `EStG`) steht NEBEN der
/// Vorsorge; Python rechnet `max(Vorsorge; 36)`. Die Zahlen unten sind die von Rust, von Hand gerechnet (Tarif § 32a, VZ 2025):
/// a1: Einkuenfte 60.000 - 1.230 = 58.770, Vorsorge min(9.021; 29.344) - 4.510 (AG-Anteil 451.050 ct, abgerundet) = 4.511, zvE
/// 58.770 - 36 - 4.511 = 54.223, Steuer 12.220 EUR = 1.222.000 ct (Python: zvE 54.259, 12.234 EUR).
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
    erwarte_zahl("a1", &a, 1_222_000);
}

/// Wie `a1`, aber 45.000 EUR Beitraege: der Abzug wird auf den Hoechstbeitrag gedeckelt. Python: 559.500 ct.
/// Rust (Abweichung Nr. 78): Vorsorge min(45.000; 29.344) - 5.000 = 24.344, zvE 58.770 - 36 - 24.344 = 34.390, Steuer 5.584 EUR =
/// 558.400 ct (Python: zvE 34.426, 5.595 EUR).
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
    erwarte_zahl("a2", &a, 558_400);
}

/// Der steuerfreie AG-Anteil (35.000 EUR) ist groesser als der Hoechstbeitrag: der Altersvorsorge-Abzug ist 0, nicht negativ.
/// Mit KV/PV-Beitraegen (2.400 EUR) dazu, denn ohne sie bleibt ein negativer Abzug folgenlos (gemessen: der Mutant ohne
/// `max(0)` blieb gruen): er wuerde die Basis-Beitraege auffressen. Python: 1.302.200 ct, gleich der Gegenprobe nur mit den
/// KV/PV-Beitraegen.
/// Rust (Abweichung Nr. 78): Altersvorsorge 0, KV/PV 2.000 + 400 = 2.400, zvE 58.770 - 36 - 2.400 = 56.334, Steuer 13.009 EUR =
/// 1.300.900 ct (Python: zvE 56.370, 13.022 EUR).
/// Seit d-46 (4-Prozent-Kuerzung, `gesetzlich_an`): KV 2.000 · 0,96 + PV 400 = 2.320, zvE 58.770 - 36 - 2.320 = 56.414, Steuer 13.039 EUR =
/// 1.303.900 ct (Handrechnung `rechnung_altfaelle.py`, geeicht an der alten Zahl 56.334 -> 13.009). Der negative Altersvorsorge-Abzug waere
/// weiter sichtbar: er fraesse 2.320 statt 2.400 auf.
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
    erwarte_zahl("a3", &a, 1_303_900);
}

/// Zusammenveranlagung: die KV/PV-Beitraege des Ehegatten (1.800 EUR) gehen in die gemeinsamen Sonderausgaben ein.
/// Python: 1.650.400 ct; ohne die Partner-Beitraege 1.707.600 ct.
/// Rust (Abweichung Nr. 78, Pauschbetrag zusammen 72 EUR neben der Vorsorge): Einkuenfte (60.000 - 1.230) + (30.000 - 1.230) = 87.540,
/// zvE 87.540 - 72 - 1.800 = 85.668, Splitting 2 x Tarif(42.834) = 16.480 EUR = 1.648.000 ct (Python: zvE 85.740, 16.504 EUR).
#[tokio::test]
async fn a4_partner_kv_pv_geht_in_die_gemeinsamen_sonderausgaben() {
    let a = ergebnis(
        "an_gesamt",
        &mit(kegel_an_gesamt(), a4_partner_kv_pv_aenderungen()),
    )
    .await;
    erwarte_zahl("a4", &a, 1_648_000);
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

// ---------------------------------------------------------------------------------------------------------------------
// Zahlen-Faelle der Zweige, Folge (k9, Auftrag 5): sechs Mutanten, die main auf 11e05f50 gemessen hat und die diese Datei
// gruen liess. Herkunft der Erwartung wie oben: `python3 tools/parity/kette_erwartung.py SCHEIBE BASIS_FN AENDERUNGS_FN`.
// ---------------------------------------------------------------------------------------------------------------------

/// Zusammenveranlagung, nur der Ehegatte hat Einkuenfte: 200.000 EUR Gewerbegewinn, Messbetrag 5.000 EUR, Hebesatz 450.
/// § 35 Abs. 1: 4 x Messbetrag = 20.000 EUR ist der KLEINSTE der drei Werte (gezahlte `GewSt` 22.500 EUR, Deckel 3 ist die
/// ganze tarifliche Steuer).
fn g18_p35_vierfacher_messbetrag_bindet_aenderungen() -> Paare {
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
        ("einkuenfte_gewinn_partner", json!(20_000_000)),
        ("gewst_messbetrag_partner", json!(500_000)),
        ("gewst_hebesatz_partner", json!(450)),
    ]
}

/// Wie `g18`, aber 80.000 EUR Lohn der Person A neben 20.000 EUR Gewerbegewinn des Ehegatten (Messbetrag 3.000 EUR,
/// Hebesatz 500): 4 x Messbetrag = 12.000 EUR und gezahlte `GewSt` 15.000 EUR liegen UEBER dem Deckel 3 (Anteil der
/// Gewerbeeinkuenfte an der Summe der positiven Einkuenfte x tarifliche Steuer); der Deckel 3 bindet.
fn g19_p35_deckel_drei_bindet_aenderungen() -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(8_000_000)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(0)),
        ("gewinn_betriebsart_partner", json!("gewerbe")),
        ("einkuenfte_gewinn_partner", json!(2_000_000)),
        ("gewst_messbetrag_partner", json!(300_000)),
        ("gewst_hebesatz_partner", json!(500)),
    ]
}

/// Wie `g12`, aber der Ehegatte erfuellt nur EINE der beiden Voraussetzungen des Freibetrags (§ 16 Abs. 4: Alter/
/// Berufsunfaehigkeit ja, erstmalige Inanspruchnahme nein): kein Freibetrag, `netto_vg_partner` ist der volle Gewinn
/// 20.000 EUR.
fn g20_partner_vg_nur_ein_haken_aenderungen() -> Paare {
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
        ("rentner_freibetrag_erstmalig_partner", json!(false)),
    ]
}

/// Wie `g5` (120.000 EUR Veraeusserungsgewinn), aber MIT Antrag auf den ermaessigten Satz und OHNE Berechtigung (kein
/// Geburtsjahr, nicht dauernd berufsunfaehig): § 34 Abs. 3 Satz 1 verlangt beides, es gilt die Fuenftelung des Abs. 1.
fn g21_p34_antrag_ohne_berechtigung_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(12_000_000)),
        ("antrag_ermaessigter_satz", json!(true)),
        ("dauernd_berufsunfaehig", json!(false)),
    ]
}

/// Zusammenveranlagung, Person A ist berechtigt (dauernd berufsunfaehig) und stellt den Antrag, hat aber KEINEN
/// Veraeusserungsgewinn; der Ehegatte hat 120.000 EUR: § 34 Abs. 3 gilt nur fuer die ausserordentlichen Einkuenfte der
/// Person A (`netto_vg` > 0), hier gilt die Fuenftelung ueber den Gewinn des Ehegatten.
fn g22_p34_antrag_a_ohne_vg_partner_mit_vg_aenderungen() -> Paare {
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
        ("rentner_veraeusserungsgewinn_partner", json!(12_000_000)),
        ("rentner_alter_55_oder_berufsunfaehig_partner", json!(true)),
        ("rentner_freibetrag_erstmalig_partner", json!(true)),
        ("antrag_ermaessigter_satz", json!(true)),
        ("dauernd_berufsunfaehig", json!(true)),
    ]
}

/// § 9 Abs. 1 Nr. 6 / § 7: Arbeitsmittel zu 800,01 EUR (80.001 ct), Nutzungsdauer 3 Jahre, KEIN Sofortabzug gewaehlt: die
/// Grenze von 800,00 EUR ist ueberschritten, es gilt die lineare `AfA`. Dazu 30 km Pendelstrecke an 220 Tagen (wie `g14`).
/// Abweichung Nr. 88: die Grenze gilt netto; der Kauf OHNE Umsatzsteuer (`keine_ust`) haelt sie bei 800,00 EUR, 800,01 EUR sind darueber.
/// (Ohne Antwort zum Satz sperrt der Fall im Band bis 952,00 EUR; das haelt `sperre_scheiben_hermetisch.rs`.)
fn g23_arbeitsmittel_800_01_eur_afa_aenderungen() -> Paare {
    vec![
        ("ep_arbeitstage", json!(220)),
        ("ep_entfernung_km", json!(30)),
        ("am_anschaffungskosten", json!(80_001)),
        ("am_umsatzsteuersatz", json!("keine_ust")),
        ("arbeitsmittel_nutzungsdauer", json!(3)),
        ("am_afa_ist_anschaffungsjahr", json!(false)),
        ("am_gwg_sofortabzug_gewaehlt", json!(false)),
    ]
}

/// Wie `g18`, dazu 30.000 EUR Progressionseinkuenfte (§ 32b): die Anrechnung laeuft im nachgezogenen Wrapper `p32b_wrapper`,
/// nicht in `p35_credit`. 4 x Messbetrag = 20.000 EUR bleibt der kleinste der drei Werte. (Person A hat keine `GewSt`: die
/// Kombination aus Progressionseinkuenfte und `GewSt` der Person A sperrt `p32b_kombi_offen`.)
fn g24_p35_vierfacher_messbetrag_bindet_nach_p32b_aenderungen() -> Paare {
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
        ("einkuenfte_gewinn_partner", json!(20_000_000)),
        ("gewst_messbetrag_partner", json!(500_000)),
        ("p32b_progressionseinkuenfte", json!(3_000_000)),
        ("gewst_hebesatz_partner", json!(450)),
    ]
}

/// Wie `g19`, dazu 30.000 EUR Progressionseinkuenfte (§ 32b): der Deckel 3 rechnet im Wrapper mit der Steuer nach § 32b
/// (`t_32b`) und bindet wieder.
fn g25_p35_deckel_drei_bindet_nach_p32b_aenderungen() -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(8_000_000)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(0)),
        ("gewinn_betriebsart_partner", json!("gewerbe")),
        ("einkuenfte_gewinn_partner", json!(2_000_000)),
        ("gewst_messbetrag_partner", json!(300_000)),
        ("p32b_progressionseinkuenfte", json!(3_000_000)),
        ("gewst_hebesatz_partner", json!(500)),
    ]
}

/// § 35 Abs. 1 `EStG`, Anrechnung ohne § 32b (`p35_credit`): die tarifliche Steuer 62.144 EUR sinkt um genau 20.000 EUR =
/// 4 x Messbetrag (5.000 EUR); gezahlte `GewSt` (22.500 EUR) und Deckel 3 (die ganze tarifliche Steuer) sind groesser.
/// Python: festzusetzende Steuer 42.144 EUR.
#[tokio::test]
async fn g18_p35_vierfacher_messbetrag_ist_der_kleinste_wert() {
    let a = ergebnis(
        "gesamt",
        &mit(
            kegel_gesamt(),
            g18_p35_vierfacher_messbetrag_bindet_aenderungen(),
        ),
    )
    .await;
    erwarte_kette("g18", &a, 4_214_400, [200_000, 199_928, 62_144, 42_144]);
}

/// § 35 Abs. 1 `EStG`, Anrechnung ohne § 32b: der Deckel 3 (20.000 / 98.770 EUR der Einkuenfte x tarifliche Steuer 20.922 EUR)
/// ist der kleinste Wert; die Anrechnung betraegt 4.236 EUR. Python: festzusetzende Steuer 16.686 EUR.
#[tokio::test]
async fn g19_p35_deckel_drei_ist_der_kleinste_wert() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g19_p35_deckel_drei_bindet_aenderungen()),
    )
    .await;
    erwarte_kette("g19", &a, 1_668_600, [98_770, 98_698, 20_922, 16_686]);
}

/// § 16 Abs. 4 `EStG`: der Freibetrag verlangt BEIDE Voraussetzungen. Der Ehegatte ist berufsunfaehig, nimmt den Freibetrag
/// aber nicht erstmalig in Anspruch: kein Freibetrag, die 20.000 EUR gehen voll in die `GdE` (88.770 EUR). Python: 17.204 EUR.
#[tokio::test]
async fn g20_partner_vg_braucht_beide_voraussetzungen_fuer_den_freibetrag() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g20_partner_vg_nur_ein_haken_aenderungen()),
    )
    .await;
    erwarte_kette("g20", &a, 1_720_400, [88_770, 88_698, 17_204, 17_204]);
}

/// § 34 Abs. 3 Satz 1 `EStG`: der Antrag allein reicht nicht, die Berechtigung (ab 55 oder dauernd berufsunfaehig) muss
/// dazukommen. Ohne sie gilt die Fuenftelung (Python: dieselbe Zahl wie `g5` und `g8`, 4.458.400 ct).
#[tokio::test]
async fn g21_p34_antrag_ohne_berechtigung_bleibt_fuenftelung() {
    let a = ergebnis(
        "gesamt",
        &mit(
            kegel_gesamt(),
            g21_p34_antrag_ohne_berechtigung_aenderungen(),
        ),
    )
    .await;
    erwarte_kette("g21", &a, 4_458_400, [133_770, 133_734, 44_584, 44_584]);
}

/// § 34 Abs. 3 `EStG` gilt fuer die ausserordentlichen Einkuenfte der Person A: hat nur der Ehegatte einen
/// Veraeusserungsgewinn, bleibt es bei der Fuenftelung, auch wenn A berechtigt ist und den Antrag stellt. Der Test pinnt nur
/// das Verhalten des Orakels; ob der Ehegatte mit eigener Berechtigung den Antrag fuer SEINEN Gewinn stellen duerfte
/// (Abs. 3 Satz 1 spricht vom Steuerpflichtigen), ist hier nicht geprueft und bleibt offen.
#[tokio::test]
async fn g22_p34_antrag_von_a_gilt_nicht_fuer_den_gewinn_des_ehegatten() {
    let a = ergebnis(
        "gesamt",
        &mit(
            kegel_gesamt(),
            g22_p34_antrag_a_ohne_vg_partner_mit_vg_aenderungen(),
        ),
    )
    .await;
    erwarte_kette("g22", &a, 3_459_400, [143_770, 143_698, 34_594, 34_594]);
}

/// § 9 Abs. 1 Nr. 6 / § 7 Abs. 1 `EStG`: ein Arbeitsmittel zu 800,01 EUR liegt ueber der Grenze; es zaehlt die lineare `AfA`
/// (266 EUR im Folgejahr, `GdE` 57.578 EUR = 57.844 - 266), nicht der Sofortabzug. Python: 1.346.700 ct.
#[tokio::test]
async fn g23_arbeitsmittel_zu_800_01_eur_sind_afa_pflichtig() {
    let a = ergebnis(
        "gesamt",
        &mit(
            kegel_gesamt(),
            g23_arbeitsmittel_800_01_eur_afa_aenderungen(),
        ),
    )
    .await;
    erwarte_kette("g23", &a, 1_346_700, [57_578, 57_542, 13_467, 13_467]);
}

/// Abweichung Nr. 88 (Regel von Julius, R 9.12 `LStR`): ein Arbeitsmittel mit `preis` Cent BRUTTO und dem Umsatzsteuersatz der
/// Rechnung, Sofortabzug gewaehlt, Nutzungsdauer 3 (gilt nur ueber der Netto-Grenze). Sonst wie `g14`.
fn arbeitsmittel_brutto_aenderungen(preis: i64, satz: &'static str) -> Paare {
    vec![
        ("ep_arbeitstage", json!(220)),
        ("ep_entfernung_km", json!(30)),
        ("am_anschaffungskosten", json!(preis)),
        ("am_umsatzsteuersatz", json!(satz)),
        ("arbeitsmittel_nutzungsdauer", json!(3)),
        ("am_afa_ist_anschaffungsjahr", json!(false)),
        ("am_gwg_sofortabzug_gewaehlt", json!(true)),
    ]
}

/// Abweichung Nr. 88: 900,00 EUR brutto bei 19 % (netto 756,30) liegen unter der Netto-Grenze, der volle Bruttopreis ist
/// Sofortabzug (`GdE` 56.944 EUR = 57.844 ohne Arbeitsmittel - 900). Vor Nr. 88 galt 900 EUR als `AfA`-pflichtig. Die Steuer
/// ist von Hand gerechnet (Skript `handrechnung.py`, Eichung an g14 und g23), nicht aus dem Rust-Code gelesen.
#[tokio::test]
async fn g29_arbeitsmittel_900_eur_brutto_bei_19_prozent_sind_sofortabzug() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), arbeitsmittel_brutto_aenderungen(90_000, "satz_19")),
    )
    .await;
    erwarte_kette("g29", &a, 1_322_600, [56_944, 56_908, 13_226, 13_226]);
}

/// Abweichung Nr. 88: 952,00 EUR brutto bei 19 % sind netto genau 800,00 EUR, noch Sofortabzug (`GdE` 56.892 EUR = 57.844 - 952).
#[tokio::test]
async fn g30_arbeitsmittel_952_eur_brutto_bei_19_prozent_sind_noch_sofortabzug() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), arbeitsmittel_brutto_aenderungen(95_200, "satz_19")),
    )
    .await;
    erwarte_kette("g30", &a, 1_320_600, [56_892, 56_856, 13_206, 13_206]);
}

/// Abweichung Nr. 88: 856,00 EUR brutto bei 7 % sind netto genau 800,00 EUR (`GdE` 56.988 EUR = 57.844 - 856).
#[tokio::test]
async fn g31_arbeitsmittel_856_eur_brutto_bei_7_prozent_sind_noch_sofortabzug() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), arbeitsmittel_brutto_aenderungen(85_600, "satz_7")),
    )
    .await;
    erwarte_kette("g31", &a, 1_324_200, [56_988, 56_952, 13_242, 13_242]);
}

/// Abweichung Nr. 89 (BMF-Schreiben vom 22.02.2022, `BStBl` I S. 187): ein Notebook, ein Geraet der Liste in Rz. 2 und 3, mit `preis` Cent
/// BRUTTO, der Antwort auf „steht auf der Liste“ (`liste`), eigener Nutzungsdauer (`nd`), der Antwort „in diesem Jahr gekauft“
/// (`kaufjahr`) und dem Kaufmonat. 30 km an 220 Tagen wie `g14`. Weder Umsatzsteuersatz noch Wahl des Sofortabzugs.
fn notebook_aenderungen(
    preis: i64,
    liste: Option<bool>,
    nd: Option<i64>,
    kaufjahr: bool,
    monat: Option<i64>,
) -> Paare {
    let mut p = vec![
        ("ep_arbeitstage", json!(220)),
        ("ep_entfernung_km", json!(30)),
        ("am_anschaffungskosten", json!(preis)),
        ("am_afa_ist_anschaffungsjahr", json!(kaufjahr)),
    ];
    if let Some(l) = liste {
        p.push(("am_computer_software", json!(l)));
    }
    if let Some(n) = nd {
        p.push(("arbeitsmittel_nutzungsdauer", json!(n)));
    }
    if let Some(m) = monat {
        p.push(("am_anschaffung_monat", json!(m)));
    }
    p
}

/// Abweichung Nr. 89, Scheibe `gesamt`: 900,00 EUR brutto, Antwort „ja“, weder Satz noch Wahl noch Nutzungsdauer. Das Geraet ist kein
/// `GWG` (die 800-Euro-Grenze gilt nicht), die Nutzungsdauer ist ein Jahr, der Abzug im Kaufjahr voll (Rz. 1.4): `GdE` 56.944 EUR wie `g29`,
/// aber ohne die Satzfrage. Von Hand gerechnet (Skript `handrechnung.py`, Eichung an g14, g23, g29).
#[tokio::test]
async fn g32_notebook_900_eur_ja_ist_voller_abzug_ohne_satz_und_wahl() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), notebook_aenderungen(90_000, Some(true), None, true, None)),
    )
    .await;
    erwarte_kette("g32", &a, 1_322_600, [56_944, 56_908, 13_226, 13_226]);
}

/// Abweichung Nr. 89, Scheibe `gesamt`: 1.500,00 EUR brutto, „ja“, keine eigene Nutzungsdauer, gekauft im Oktober: voller Abzug, kein
/// Zwoelftel nach dem Kaufmonat (Rz. 1.4). `GdE` 57.844 - 1.500 = 56.344, zvE 56.308, `ESt` 12.999 EUR.
#[tokio::test]
async fn g33_notebook_1500_eur_ja_im_kaufjahr_ist_voller_abzug_ohne_zwoelftel() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), notebook_aenderungen(150_000, Some(true), None, true, Some(10))),
    )
    .await;
    erwarte_kette("g33", &a, 1_299_900, [56_344, 56_308, 12_999, 12_999]);
}

/// Abweichung Nr. 89, Scheibe `gesamt`: dasselbe Geraet im Folgejahr (nicht in diesem Jahr gekauft): mit einem Jahr Nutzungsdauer im
/// Kaufjahr aufgebraucht, der Abzug ist 0. `GdE` 57.844, zvE 57.808, `ESt` 13.568 EUR.
#[tokio::test]
async fn g34_notebook_1500_eur_ja_im_folgejahr_hat_keinen_abzug_mehr() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), notebook_aenderungen(150_000, Some(true), None, false, None)),
    )
    .await;
    erwarte_kette("g34", &a, 1_356_800, [57_844, 57_808, 13_568, 13_568]);
}

/// Abweichung Nr. 89, Scheibe `gesamt`: „ja“ mit eigener Nutzungsdauer 3 (Rz. 1.2: abweichen ist erlaubt): die Verteilung wie vor Nr. 89,
/// 1.500 / 3 = 500, Oktober (13 - 10) / 12 = 3/12, also 125. `GdE` 57.719, zvE 57.683, `ESt` 13.520 EUR.
#[tokio::test]
async fn g35_notebook_1500_eur_ja_mit_eigener_nutzungsdauer_3_verteilt_wie_bisher() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), notebook_aenderungen(150_000, Some(true), Some(3), true, Some(10))),
    )
    .await;
    erwarte_kette("g35", &a, 1_352_000, [57_719, 57_683, 13_520, 13_520]);
}

/// Abweichung Nr. 89, Scheibe `gesamt`: „nein“ mit Nutzungsdauer 1 und Oktober: der Weg vor Nr. 89, ein Zwoelftel-Anteil
/// (1.500 x 3/12 = 375). Ohne „ja“ gilt die Sonderregel nicht. `GdE` 57.469, zvE 57.433, `ESt` 13.425 EUR.
#[tokio::test]
async fn g36_notebook_1500_eur_nein_mit_nutzungsdauer_1_hat_das_zwoelftel_nach_dem_kaufmonat() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), notebook_aenderungen(150_000, Some(false), Some(1), true, Some(10))),
    )
    .await;
    erwarte_kette("g36", &a, 1_342_500, [57_469, 57_433, 13_425, 13_425]);
}

/// Abweichung Nr. 89, Scheibe `an_gesamt` (60.000 EUR Lohn, 30 km an 220 Tagen, Pauschale 2.156 EUR): 1.500,00 EUR brutto, „ja“,
/// Kaufjahr Oktober. Einkuenfte 60.000 - 2.156 - 1.500 = 56.344, zvE 56.308, `ESt` 12.999 EUR = 1.299.900 ct (wie `g33` in `gesamt`).
/// Der Abzug kommt NACH dem Catala-Scope dazu (`mit_computer_im_kaufjahr`): der Scope zaehlt 1.500 EUR gegen 800 EUR und lieferte 0.
#[tokio::test]
async fn a8_notebook_1500_eur_ja_ist_in_an_gesamt_voller_abzug() {
    let a = ergebnis(
        "an_gesamt",
        &mit(kegel_an_gesamt(), mit(vec![("bruttoarbeitslohn", json!(6_000_000))], notebook_aenderungen(150_000, Some(true), None, true, Some(10)))),
    )
    .await;
    erwarte_zahl("a8", &a, 1_299_900);
}

/// Abweichung Nr. 89, Scheibe `an_gesamt`: 900,00 EUR brutto im Band, „ja“, ohne Satz: voller Abzug, 1.322.600 ct (wie `a7`).
#[tokio::test]
async fn a9_notebook_900_eur_ja_ist_in_an_gesamt_voller_abzug_ohne_satz() {
    let a = ergebnis(
        "an_gesamt",
        &mit(kegel_an_gesamt(), mit(vec![("bruttoarbeitslohn", json!(6_000_000))], notebook_aenderungen(90_000, Some(true), None, true, None))),
    )
    .await;
    erwarte_zahl("a9", &a, 1_322_600);
}

/// Abweichung Nr. 89, Scheibe `an_gesamt`: dasselbe Geraet im Folgejahr, Abzug 0: 57.844 - 36 = 57.808, 1.356.800 ct.
#[tokio::test]
async fn a10_notebook_1500_eur_ja_im_folgejahr_hat_in_an_gesamt_keinen_abzug() {
    let a = ergebnis(
        "an_gesamt",
        &mit(kegel_an_gesamt(), mit(vec![("bruttoarbeitslohn", json!(6_000_000))], notebook_aenderungen(150_000, Some(true), None, false, None))),
    )
    .await;
    erwarte_zahl("a10", &a, 1_356_800);
}

/// Wie `g18` mit 30.000 EUR Progressionseinkuenfte: die Anrechnung rechnet `p32b_wrapper` nach. 4 x Messbetrag (20.000 EUR)
/// ist wieder der kleinste Wert. Python: tarifliche 62.144 EUR, festzusetzende 44.991 EUR (= Steuer nach § 32b 64.991 - 20.000).
#[tokio::test]
async fn g24_p35_nach_p32b_vierfacher_messbetrag_ist_der_kleinste_wert() {
    let a = ergebnis(
        "gesamt",
        &mit(
            kegel_gesamt(),
            g24_p35_vierfacher_messbetrag_bindet_nach_p32b_aenderungen(),
        ),
    )
    .await;
    erwarte_kette("g24", &a, 4_499_100, [200_000, 199_928, 62_144, 44_991]);
}

/// Wie `g19` mit 30.000 EUR Progressionseinkuenfte: im Wrapper bindet der Deckel 3 (Anteil x Steuer nach § 32b). Python:
/// festzusetzende Steuer 19.748 EUR.
#[tokio::test]
async fn g25_p35_nach_p32b_deckel_drei_ist_der_kleinste_wert() {
    let a = ergebnis(
        "gesamt",
        &mit(
            kegel_gesamt(),
            g25_p35_deckel_drei_bindet_nach_p32b_aenderungen(),
        ),
    )
    .await;
    erwarte_kette("g25", &a, 1_974_800, [98_770, 98_698, 20_922, 19_748]);
}

/// § 34 Abs. 3 Satz 1 `EStG`: Alter GENAU 55 (Geburtsjahr 1970, Veranlagungsjahr 2025) berechtigt, auch ohne dauernde
/// Berufsunfaehigkeit; dazu der Antrag und 120.000 EUR Veraeusserungsgewinn. Python `vz - geburtsjahr >= 55`.
fn g26_p34_abs3_alter_genau_55_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(12_000_000)),
        ("antrag_ermaessigter_satz", json!(true)),
        ("geburtsjahr", json!(1970)),
        ("dauernd_berufsunfaehig", json!(false)),
    ]
}

/// § 33a Abs. 2 `EStG`: ein Kind in Berufsausbildung, auswaerts untergebracht (Ausbildungsfreibetrag 1.200 EUR).
fn g27_ausbildungsfreibetrag_aenderungen() -> Paare {
    vec![("p33a_ausbildung_anzahl_kinder", json!(1))]
}

/// § 34 Abs. 3 Satz 1 `EStG`: wer GENAU 55 Jahre alt wird (2025 - 1970), ist berechtigt. Mit Antrag gilt der ermaessigte
/// Satz: Python 2.813.600 ct (die Fuenftelung desselben Falls waere 4.458.400 ct, siehe `g21`).
#[tokio::test]
async fn g26_p34_abs3_gilt_ab_genau_55_jahren() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g26_p34_abs3_alter_genau_55_aenderungen()),
    )
    .await;
    erwarte_kette("g26", &a, 2_813_600, [133_770, 133_734, 28_136, 28_136]);
}

/// § 33a Abs. 2 `EStG`: der Ausbildungsfreibetrag mindert das zu versteuernde Einkommen um 1.200 EUR (zvE 57.534 statt
/// 58.734 EUR). Python: 1.346.400 ct statt 1.392.400 ct.
#[tokio::test]
async fn g27_ausbildungsfreibetrag_mindert_das_zu_versteuernde_einkommen() {
    let a = ergebnis(
        "gesamt",
        &mit(kegel_gesamt(), g27_ausbildungsfreibetrag_aenderungen()),
    )
    .await;
    erwarte_kette("g27", &a, 1_346_400, [58_770, 57_534, 13_464, 13_464]);
}

/// Wie `g26` (Alter genau 55, Antrag auf den ermaessigten Satz), aber der ANTRAG ist nur VORLAEUFIG (kein `signal_2`): er
/// zaehlt nicht, die Fuenftelung gilt. Das Feld liegt ausserhalb des Pflicht-Kegels und ist kein Betrag, darum sperrt es die
/// Zahl nicht (`ring_betrag_vorlaeufig` nennt nur Betragsfelder).
fn g28_p34_antrag_nur_vorlaeufig_aenderungen() -> Paare {
    vec![
        ("kein_gewinn", json!(false)),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
        ("rentner_freibetrag_erstmalig", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(12_000_000)),
        ("antrag_ermaessigter_satz", json!(true)),
        ("geburtsjahr", json!(1970)),
        ("dauernd_berufsunfaehig", json!(false)),
    ]
}

/// Python: der nur vorlaeufige Antrag zaehlt nicht, die Zahl ist die Fuenftelung (4.458.400 ct, wie `g21`), nicht der
/// ermaessigte Satz (2.813.600 ct, `g26`). Der Bescheid liest nur Bestaetigtes (`nur_bestaetigt`): ein Vorschlag bewegt die
/// festgesetzte Steuer nie, bevor der Mensch ihn bestaetigt hat.
#[tokio::test]
async fn g28_vorlaeufiger_antrag_aendert_die_festgesetzte_steuer_nicht() {
    let a = ergebnis_vorlaeufig(
        "gesamt",
        &mit(kegel_gesamt(), g28_p34_antrag_nur_vorlaeufig_aenderungen()),
        &["antrag_ermaessigter_satz"],
    )
    .await;
    erwarte_kette("g28", &a, 4_458_400, [133_770, 133_734, 44_584, 44_584]);
}

// ---- Beide Zweige rechnen dieselbe Rechenstelle gleich (Nr. 70, `test_zweig_duplikation_differential.py`) ---------------------------

/// Was eine Aenderung an der Basis in der Kette bewirkt: (`GdE`, zvE) in Euro, nachher minus vorher, im SELBEN Zweig.
///
/// Die Python-Datei schnitt die Aufrufe der Rechenstellen mit und verglich je Accessor Argumente und Ergebnis beider Zweige. Rust ruft
/// jede dieser Stellen nur noch aus EINER Hilfsfunktion in `zweige/gesamt.rs`, die der Rentner-Zweig mitbenutzt; die Kopie, die dort
/// auseinanderlaufen konnte, gibt es nicht mehr. Es kann aber ein Zweig die Hilfsfunktion nicht rufen oder ihr Ergebnis weglassen
/// (Nachmessung Nr. 70, Mutant DU1: der Rentner-Zweig laesst den Pauschbetrag der Person A weg, alle 1.673 Tests der Pakete blieben
/// gruen). Verglichen wird darum die WIRKUNG derselben Eingabe in beiden Zweigen, nie die Endsumme: dieselbe Formel greift in
/// verschiedene Progressionsstufen (`WAHLRECHT_TRUE_DELTA_GESAMT` 135000 gegen `_RENTNER` 126000), `GdE` und zvE aber sind Abzuege und
/// Zurechnungen in Euro ohne Tarif.
async fn kette_unterschied(scheibe: &str, basis: Paare, aenderungen: Paare) -> (i64, i64) {
    let vorher = ergebnis(scheibe, &basis).await;
    let nachher = ergebnis(scheibe, &mit(basis, aenderungen)).await;
    for (name, a) in [("vorher", &vorher), ("nachher", &nachher)] {
        assert_eq!(
            a["grund"], "bestaetigt",
            "KONTROLLE: {scheibe} {name}: Kegel vollstaendig? {a}"
        );
        assert!(
            !a["kette"].is_null(),
            "KONTROLLE: {scheibe} {name}: keine Kette -- {a}"
        );
    }
    let (v, n) = (stufen(&vorher), stufen(&nachher));
    (n[0] - v[0], n[1] - v[1])
}

/// Dieselbe Eingabe, beide Zweige: gleiche Wirkung auf `GdE` und zvE, und die Wirkung ist nicht 0 (sonst misst der Fall nichts).
async fn beide_zweige_gleich(name: &str, aenderungen: Paare) {
    let gesamt = kette_unterschied("gesamt", kegel_gesamt(), aenderungen.clone()).await;
    let rentner = kette_unterschied("rentner_gesamt", kegel_rentner(), aenderungen).await;
    assert_ne!(
        gesamt,
        (0, 0),
        "KONTROLLE: {name} bewirkt im Zweig gesamt nichts -- der Fall erreicht die Rechenstelle nicht"
    );
    assert_eq!(
        gesamt, rentner,
        "DEFEKT: {name}: dieselbe Eingabe wirkt in den Zweigen verschieden (GdE, zvE in Euro): gesamt {gesamt:?}, rentner {rentner:?}"
    );
}

#[tokio::test]
async fn behinderten_pauschbetrag_wirkt_in_beiden_zweigen_gleich() {
    beide_zweige_gleich(
        "§ 33b Behinderten-Pauschbetrag (GdB 50)",
        vec![("rentner_grad_der_behinderung", json!(50))],
    )
    .await;
}

#[tokio::test]
async fn pflege_pauschbetrag_wirkt_in_beiden_zweigen_gleich() {
    beide_zweige_gleich(
        "§ 33b Pflege-Pauschbetrag (Pflegegrad 2)",
        vec![("rentner_pflegegrad", json!(2))],
    )
    .await;
}

#[tokio::test]
async fn hinterbliebenen_pauschbetrag_wirkt_in_beiden_zweigen_gleich() {
    beide_zweige_gleich(
        "§ 33b Hinterbliebenen-Pauschbetrag",
        vec![("rentner_hinterbliebenenbezuege", json!(true))],
    )
    .await;
}

#[tokio::test]
async fn kirchensteuer_erstattungsueberhang_wirkt_in_beiden_zweigen_gleich() {
    beide_zweige_gleich(
        "§ 10 Abs. 4b Erstattungsueberhang (1.000 gezahlt, 2.500 erstattet)",
        vec![("kist_gezahlt", json!(100_000)), ("kist_erstattet", json!(250_000))],
    )
    .await;
}

#[tokio::test]
async fn entlastungsbetrag_fuer_alleinerziehende_wirkt_in_beiden_zweigen_gleich() {
    beide_zweige_gleich(
        "§ 24b Entlastungsbetrag (allein stehend, ein Kind)",
        vec![
            ("fam_alleinstehend", json!(true)),
            ("fam_anzahl_kinder", json!(1)),
            ("fam_monate_ohne_voraussetzung", json!(0)),
        ],
    )
    .await;
}

#[tokio::test]
async fn sonstige_einkuenfte_nach_p22_nr3_wirken_in_beiden_zweigen_gleich() {
    beide_zweige_gleich(
        "§ 22 Nr. 3 (500 EUR, ueber der Freigrenze)",
        vec![("p22_nr3_einkuenfte", json!(50_000))],
    )
    .await;
}

/// Der Ueberhang geht auch in die `GdE`, die der Spenden-Hoechstbetrag (20 % der `GdE`) liest. Nachmessung Nr. 70, Mutant MR4: der
/// Rentner-Zweig liess den Ueberhang aus der `GdE` fuer die Folgestellen weg, alle alten Tests und die sieben ersten neuen blieben
/// gruen, weil kein Fall den Ueberhang mit einer Spende ueber dem Hoechstbetrag verband. Die Spende (20.000 EUR) steht in der
/// BASIS und liegt in beiden Zweigen ueber dem Deckel; gemessen wird nur, was der Ueberhang dazu bewirkt.
#[tokio::test]
async fn erstattungsueberhang_hebt_den_spendendeckel_in_beiden_zweigen_gleich() {
    let spende = vec![("spenden_betrag", json!(2_000_000))];
    let aenderung = vec![("kist_gezahlt", json!(100_000)), ("kist_erstattet", json!(250_000))];
    let gesamt = kette_unterschied("gesamt", mit(kegel_gesamt(), spende.clone()), aenderung.clone()).await;
    let rentner = kette_unterschied("rentner_gesamt", mit(kegel_rentner(), spende), aenderung).await;
    assert_ne!(gesamt, (0, 0), "KONTROLLE: der Ueberhang wirkt im Zweig gesamt nicht");
    assert_eq!(
        gesamt, rentner,
        "DEFEKT: der Ueberhang wirkt mit einer Spende ueber dem Deckel in den Zweigen verschieden (GdE, zvE): gesamt {gesamt:?}, rentner {rentner:?}"
    );
}

/// Der Ehegatte: Zusammenveranlagung, Person B bestaetigt Lohn und Kapital mit 0 (der Kegel von `gesamt`) -- Gleiches fuer den Rentner.
fn ehepaar() -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ]
}

/// Bei Einzelveranlagung sperrt eine Angabe zum Ehegatten das Ergebnis mit `partner_konsistenz_offen`, in beiden Zweigen. Nachmessung
/// Nr. 70, Mutanten MG12 und MR6 (der Pauschbetrag des Ehegatten zaehlt auch bei Einzelveranlagung): kein Fall erreicht diese Stelle mit
/// einer Zahl, weil die Sperre davorsteht -- die Grenze der Zweig-Tests. Faellt die Sperre, faerbt dieser Test und nennt die Stelle.
#[tokio::test]
async fn angabe_zum_ehegatten_bei_einzelveranlagung_sperrt_in_beiden_zweigen() {
    let aenderung = vec![("rentner_grad_der_behinderung_partner", json!(50))];
    for (scheibe, basis) in [("gesamt", kegel_gesamt()), ("rentner_gesamt", kegel_rentner())] {
        let a = ergebnis(scheibe, &mit(basis, aenderung.clone())).await;
        assert_eq!(
            a["grund"], "partner_konsistenz_offen",
            "DEFEKT: {scheibe}: bei Einzelveranlagung sperrt eine Angabe zum Ehegatten nicht mehr -- MG12/MR6 (Pauschbetrag des Ehegatten) sind dann erreichbar: {a}"
        );
    }
}

/// Der Behinderten-Pauschbetrag des Ehegatten (nur bei Zusammenveranlagung): beide Zweige tragen ihn gleich ein.
#[tokio::test]
async fn behinderten_pauschbetrag_des_ehegatten_wirkt_in_beiden_zweigen_gleich() {
    let aenderung = vec![("rentner_grad_der_behinderung_partner", json!(50))];
    let gesamt = kette_unterschied("gesamt", mit(kegel_gesamt(), ehepaar()), aenderung.clone()).await;
    let rentner = kette_unterschied("rentner_gesamt", mit(kegel_rentner(), ehepaar()), aenderung).await;
    assert_ne!(gesamt, (0, 0), "KONTROLLE: der Pauschbetrag des Ehegatten wirkt im Zweig gesamt nicht");
    assert_eq!(
        gesamt, rentner,
        "DEFEKT: der Pauschbetrag des Ehegatten wirkt in den Zweigen verschieden (GdE, zvE): gesamt {gesamt:?}, rentner {rentner:?}"
    );
}

/// Das Ehepaar ohne Kapitalertraege des Partners (Fall X): Lohn A 60.000 EUR, Lohn B 20.000 EUR, `kein_kap_partner` bestaetigt `true`,
/// die fuenf Kapital-Felder des Ehegatten fehlen.
fn ehepaar_ohne_kapitalertraege() -> Paare {
    let fuenf = ["kap_kapitalertraege_partner", "kap_gewinn_aktien_partner", "kap_gewinn_sonstige_partner", "kap_verlust_aktien_partner", "kap_verlust_sonstige_partner"];
    let lohn = vec![("bruttoarbeitslohn", json!(6_000_000)), ("bruttoarbeitslohn_partner", json!(2_000_000))];
    let mut paare = mit(mit(kegel_gesamt(), ehepaar()), lohn);
    paare.retain(|(f, _)| !fuenf.contains(f));
    mit(paare, vec![("kein_kap_partner", json!(true))])
}

/// Abweichung Nr. 96 (Julius d-47): ein Ehepaar, das „keine Kapitalertraege des Partners“ bestaetigt und die fuenf Kapital-Felder des
/// Ehegatten nicht beantwortet, bekommt dieselbe Zahl wie das Paar, das die Kapitalertraege ankreuzt und die fuenf Felder mit 0 beantwortet
/// (Gegenprobe). Vor der Aenderung sperrte der erste Fall mit `partner_kegel_offen`, ohne offene Frage. Lohn A 60.000 EUR, Lohn B 20.000 EUR:
/// gemeinsame `ESt` 13.838 EUR (Handrechnung der Kirchensteuer-Faelle unten, Tarif aus `params/2025/`).
#[tokio::test]
async fn ehepaar_ohne_kapitalertraege_des_partners_hat_dieselbe_zahl_wie_mit_kreuz_und_fuenf_nullen() {
    let lohn = vec![("bruttoarbeitslohn", json!(6_000_000)), ("bruttoarbeitslohn_partner", json!(2_000_000))];
    // Y: Kreuz gesetzt („ja, das gab es“ = kein_kap_partner false), die fuenf Felder mit 0 beantwortet.
    let mit_kreuz = mit(mit(mit(kegel_gesamt(), ehepaar()), lohn), vec![("kein_kap_partner", json!(false))]);
    // X: kein Kreuz (kein_kap_partner true), die fuenf Felder fehlen.
    let ohne_kreuz = ehepaar_ohne_kapitalertraege();

    let y = ergebnis("gesamt", &mit_kreuz).await;
    erwarte_zahl("Y mit Kreuz und fuenf Nullen", &y, 1_383_800);
    let x = ergebnis("gesamt", &ohne_kreuz).await;
    assert_ne!(
        x["grund"], "partner_kegel_offen",
        "DEFEKT: das Ehepaar ohne Kapitalertraege des Partners wird mit `partner_kegel_offen` gesperrt: {x}"
    );
    erwarte_zahl("X ohne Kreuz, ohne die fuenf Felder", &x, 1_383_800);
    assert_eq!(x["zahl_cent"], y["zahl_cent"], "X und Y muessen dieselbe Zahl geben");
    // Schweigen ist kein „keine“: fehlt auch das Kreuz, bleibt die Sperre.
    let stumm: Paare = ohne_kreuz.iter().filter(|(f, _)| *f != "kein_kap_partner").cloned().collect();
    let s = ergebnis("gesamt", &stumm).await;
    assert_eq!(s["grund"], "partner_kegel_offen", "DEFEKT: ohne das Kreuz muss die Sperre bleiben: {s}");
}

/// Grenze der Aenderung (Nr. 96): bestaetigt das Paar „keine Kapitalertraege des Partners“ und steht trotzdem ein Kapitalbetrag des Ehegatten
/// nur VORLAEUFIG im Fall, verlangt die Sperre `partner_kegel_offen` die fuenf Felder nicht mehr, aber die Zahl bleibt aus: ein vorlaeufiger
/// Betrag faellt nie still aus der Rechnung (`feste_zahl`, Lage 3 `ring_betrag_vorlaeufig`). Die Frage dazu entfaellt mit dem Kreuz.
#[tokio::test]
async fn ein_nur_vorlaeufiger_kapitalbetrag_des_partners_gibt_auch_mit_dem_kreuz_keine_zahl() {
    let mut paare = ehepaar_ohne_kapitalertraege();
    paare.push(("kap_kapitalertraege_partner", json!(100_000)));
    let a = ergebnis_vorlaeufig("gesamt", &paare, &["kap_kapitalertraege_partner"]).await;
    eprintln!("MESSUNG vorlaeufig: grund={} zahl_cent={} sperr_felder={}", a["grund"], a["zahl_cent"], a["sperr_felder"]);
    assert_ne!(a["grund"], "bestaetigt", "DEFEKT: ein vorlaeufiger Kapitalbetrag des Partners faellt still aus der Zahl: {a}");
    assert!(a["zahl_cent"].is_null(), "DEFEKT: trotz vorlaeufigem Betrag steht eine Zahl da: {a}");
}


// ---- Abweichung Nr. 94: Kirchensteuer bei Zusammenveranlagung, wenn nur ein Ehegatte Mitglied ist ---------------------------
//
// Die Landes-Kirchensteuergesetze erheben bei Zusammenveranlagung nur von dem Teil der gemeinsamen ESt, der auf das Mitglied
// entfaellt (NRW § 7 Abs. 2, BY Art. 9 Abs. 2 Nr. 2, BW § 19 Abs. 4; die uebrigen Laender gleich). Der Teil folgt dem Verhaeltnis der
// Grundtarif-Betraege (§ 32a Abs. 1) auf die Summe der Einkuenfte jeder Person; ein Ehegatte mit Lohn zaehlt Lohn minus
// Arbeitnehmer-Pauschbetrag (§ 9a Satz 2: nur bis zur Hoehe der Einnahmen), gesondert besteuerte Kapitaleinkuenfte zaehlen nicht (NRW § 7
// Abs. 2 Satz 3). Vor Nr. 94 rechnete die App Satz mal VOLLE gemeinsame ESt, wenn Person A Mitglied war, und 0, wenn nur Person B es war.
//
// HERKUNFT DER ERWARTUNGSWERTE: Handrechnung nach Gesetz, Skript `handrechnung.py` im Cache-Ordner des Auftrags (Tarif und Pauschbetraege aus
// `params/2025/`, nicht aus der App; geeicht an `a4` (zvE 85.668 -> 16.480 EUR) und `a5` (zvE 58.734 -> 13.924 EUR)). Lohn A 60.000 EUR, Lohn B
// 20.000 EUR: Einkuenfte 58.770 und 18.770, zvE 77.468 (Pauschbetrag 2 x 36), gemeinsame ESt 2 x Tarif(38.734) = 13.838 EUR, Grundtarif auf die
// Einkuenfte 13.938 (A) und 1.336 EUR (B), Anteil A 13.938 / 15.274, Anteil B 1.336 / 15.274. Kirchensteuer Cent = ESt in Euro x Satz x Anteil,
// auf den Cent abgerundet. Die Rechnung gilt in allen drei Gesamtlauf-Scheiben gleich (nur Lohn: kein Unterschied im Zweig).

/// Ein Ehepaar mit Lohn; `konf_b` `None` laesst die Frage nach der Konfession des Ehegatten unbeantwortet.
fn kist_ehepaar(
    lohn_b: i64,
    konf_a: &'static str,
    konf_b: Option<&'static str>,
    land: &'static str,
) -> Paare {
    let mut p = vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("bruttoarbeitslohn_partner", json!(lohn_b)),
        ("kist_konfession", json!(konf_a)),
        ("kist_bundesland", json!(land)),
        // Die Sperre `kirchensteuer_betrag_offen` (Person A Mitglied) verlangt beide Betraege; sie gehoert nicht zu Nr. 94.
        ("kist_gezahlt", json!(0)),
        ("kist_erstattet", json!(0)),
    ];
    if let Some(k) = konf_b {
        p.push(("kist_konfession_partner", json!(k)));
    }
    p
}

/// `zahl_cent` ist die gemeinsame `ESt` (Positivkontrolle der Handrechnung), `kist_cent` die Schaetzung der Kirchensteuer.
fn erwarte_kist(name: &str, a: &Value, zahl_cent: i64, kist_cent: i64) {
    erwarte_zahl(name, a, zahl_cent);
    assert_eq!(
        a["kist_cent"].as_i64(),
        Some(kist_cent),
        "{name}: kist_cent -- {a}"
    );
}

/// (Name, Lohn B in Cent, Konfession A, Konfession B, Land, gemeinsame `ESt` in Cent, Kirchensteuer in Cent) -- Zahlen aus `handrechnung.out`.
type KistFall = (
    &'static str,
    i64,
    &'static str,
    Option<&'static str>,
    &'static str,
    i64,
    i64,
);

// Bayern, nur B Mitglied (Entscheidung d-43; die Frage nach dem Bundesland steht dafuer erst seit der ODER-Bedingung in `kist_bundesland`,
// `bundesland_frage_ehegatte.rs`): gleiche gemeinsame ESt 13.838 EUR, Satz 8 % statt 9 %, Anteil B 1.336 / 15.274 -> 110.704 x 1.336 / 15.274 =
// 9.683 Cent (abgerundet). Handrechnung im Skript `handrechnung_bayern_b.py` des Auftrags, geeicht an derselben Zeile mit 9 % (= 10.893 Cent).
const KIST_FAELLE: [KistFall; 8] = [
    ("A ev, B keine: Anteil von A (91,25 %)", 2_000_000, "evangelisch", Some("keine"), "nordrhein_westfalen", 1_383_800, 113_648),
    ("A keine, B ev: Anteil von B (8,75 %), nicht 0", 2_000_000, "keine", Some("evangelisch"), "nordrhein_westfalen", 1_383_800, 10_893),
    ("A ev, B rk: die Summe bleibt Satz mal volle ESt", 2_000_000, "evangelisch", Some("roemisch-katholisch"), "nordrhein_westfalen", 1_383_800, 124_542),
    ("Bayern 8 %, A ev, B keine", 2_000_000, "evangelisch", Some("keine"), "bayern", 1_383_800, 101_020),
    ("Bayern 8 %, A keine, B ev: Anteil von B", 2_000_000, "keine", Some("evangelisch"), "bayern", 1_383_800, 9_683),
    ("Alleinverdiener A ev, B keine: Anteil 1", 0, "evangelisch", Some("keine"), "nordrhein_westfalen", 823_800, 74_142),
    ("Alleinverdiener A keine, B ev: Anteil 0", 0, "keine", Some("evangelisch"), "nordrhein_westfalen", 823_800, 0),
    ("Konfession von B unbeantwortet: wie bisher, volle ESt", 2_000_000, "evangelisch", None, "nordrhein_westfalen", 1_383_800, 124_542),
];

/// Scheibe `gesamt`.
#[tokio::test]
async fn kist_bei_zusammenveranlagung_gesamt_nach_landesrecht() {
    for (name, lohn_b, konf_a, konf_b, land, zahl, kist) in KIST_FAELLE {
        let basis = mit(mit(kegel_gesamt(), ehepaar()), kist_ehepaar(lohn_b, konf_a, konf_b, land));
        erwarte_kist(&format!("gesamt, {name}"), &ergebnis("gesamt", &basis).await, zahl, kist);
    }
}

/// Scheibe `rentner_gesamt` (Rente 0, Lohn beider Ehegatten: derselbe Zweig fuer Person B wie in `gesamt`).
#[tokio::test]
async fn kist_bei_zusammenveranlagung_rentner_nach_landesrecht() {
    for (name, lohn_b, konf_a, konf_b, land, zahl, kist) in KIST_FAELLE {
        let basis = mit(
            mit(kegel_rentner(), vec![("rentner_jahresrente", json!(0))]),
            mit(ehepaar(), kist_ehepaar(lohn_b, konf_a, konf_b, land)),
        );
        erwarte_kist(
            &format!("rentner_gesamt, {name}"),
            &ergebnis("rentner_gesamt", &basis).await,
            zahl,
            kist,
        );
    }
}

/// Einzelveranlagung bleibt wie bisher: Satz mal `ESt` (58.734 EUR zvE -> 13.924 EUR, 9 % = 125.316 ct), Person B zaehlt nicht.
#[tokio::test]
async fn kist_bei_einzelveranlagung_bleibt_satz_mal_est() {
    let basis = mit(
        kegel_gesamt(),
        vec![
            ("kist_konfession", json!("evangelisch")),
            ("kist_bundesland", json!("nordrhein_westfalen")),
            ("kist_gezahlt", json!(0)),
            ("kist_erstattet", json!(0)),
        ],
    );
    erwarte_kist("gesamt, einzel", &ergebnis("gesamt", &basis).await, 1_392_400, 125_316);
}

/// (Name, Scheibe, Lohn B in Cent, Aenderungen gegenueber `kist_ehepaar`, gemeinsame `ESt` in Cent, Kirchensteuer in Cent).
type KistEinkunftFall = (&'static str, &'static str, i64, Paare, i64, i64);

/// Einkunftsarten ausser Lohn gehen in die Summe der Einkuenfte der Person ein, der sie gehoeren. NRW, A ev, B keine, Zusammenveranlagung;
/// die Zahlen stehen im Skript unter `GG1`, `GG2/GG3`, `RR1` und `RR2`. Je Zeile faellt ohne die Einkunftsart der Anteil von A anders aus.
#[tokio::test]
async fn kist_anteil_zaehlt_gewinn_vermietung_und_rente_der_person_zu() {
    let gewinn_a = [("kein_gewinn", json!(false)), ("einkuenfte_gewinn", json!(2_000_000))];
    let gewinn_b = [
        ("kein_gewinn", json!(false)),
        ("einkuenfte_gewinn", json!(0)),
        ("gewinn_betriebsart_partner", json!("selbstaendig")),
        ("einkuenfte_gewinn_partner", json!(2_000_000)),
    ];
    // d-53 (2026-10-22): die Zeile `kein_sonstige_partner` steht in der Rentner-Liste; "ja, der Ehegatte hat eine Rente" gehoert zur Rente von B.
    let rente_b = [
        ("kein_sonstige_partner", json!(false)),
        ("rentner_jahresrente", json!(0)),
        ("rentner_renten_art_partner", json!("gesetzliche_rente")),
        ("rentner_jahresrente_partner", json!(2_000_000)),
        ("rentner_renten_beginn_jahr_partner", json!(2_025)),
        ("rentner_alter_bei_rentenbeginn_partner", json!(65)),
    ];
    let faelle: [KistEinkunftFall; 5] = [
        ("GG1 gesamt: Gewinn von B", "gesamt", 0, gewinn_b.to_vec(), 1_422_600, 114_562),
        ("GG2 gesamt: Vermietung von A", "gesamt", 2_000_000, vec![("bruttoarbeitslohn", json!(4_000_000)), ("kein_vuv", json!(false)), ("vv_einnahmen", json!(2_000_000))], 1_383_800, 113_648),
        ("GG3 gesamt: Gewinn von A", "gesamt", 2_000_000, [("bruttoarbeitslohn", json!(4_000_000))].into_iter().chain(gewinn_a).collect(), 1_383_800, 113_648),
        ("RR1 rentner_gesamt: Rente von B", "rentner_gesamt", 0, [("bruttoarbeitslohn", json!(4_000_000))].into_iter().chain(rente_b).collect(), 729_000, 58_675),
        ("RR2 rentner_gesamt: Rente von A", "rentner_gesamt", 4_000_000, vec![("bruttoarbeitslohn", json!(0)), ("rentner_jahresrente", json!(2_000_000))], 729_000, 6_934),
    ];
    for (name, scheibe, lohn_b, aenderung, zahl, kist) in faelle {
        let kegel = if scheibe == "gesamt" { kegel_gesamt() } else { kegel_rentner() };
        let basis = mit(
            mit(kegel, ehepaar()),
            mit(kist_ehepaar(lohn_b, "evangelisch", Some("keine"), "nordrhein_westfalen"), aenderung),
        );
        erwarte_kist(name, &ergebnis(scheibe, &basis).await, zahl, kist);
    }
}

// ---- Scheibe `an_gesamt` (Nr. 94, zuletzt aufgenommen): nur Lohn je Person; Person B ohne eigene Werbungskosten, mit dem Pauschbetrag
// 1.230 EUR wie in `est_zusammen` (Einkuenfte 58.770 und 18.770 EUR, gleiche Handrechnung wie in `gesamt`). `kist_gezahlt` und
// `kist_erstattet` kennt diese Scheibe nicht, die Sperre `kirchensteuer_betrag_offen` entfaellt hier.

/// Ein Ehepaar in `an_gesamt`; `konf_b` `None` laesst die Frage nach der Konfession des Ehegatten unbeantwortet.
fn kist_an_gesamt(lohn_b: i64, konf_a: &'static str, konf_b: Option<&'static str>, land: &'static str) -> Paare {
    let mut p = vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("bruttoarbeitslohn_partner", json!(lohn_b)),
        ("kist_konfession", json!(konf_a)),
        ("kist_bundesland", json!(land)),
    ];
    if let Some(k) = konf_b {
        p.push(("kist_konfession_partner", json!(k)));
    }
    p
}

async fn erwarte_kist_an_gesamt(faelle: &[KistFall]) {
    for &(name, lohn_b, konf_a, konf_b, land, zahl, kist) in faelle {
        let basis = mit(kegel_an_gesamt(), kist_an_gesamt(lohn_b, konf_a, konf_b, land));
        erwarte_kist(&format!("an_gesamt, {name}"), &ergebnis("an_gesamt", &basis).await, zahl, kist);
    }
}

/// Fall 1: beide Ehegatten Mitglied (gleiche oder verschiedene Kirche): die Summe bleibt Satz mal volle gemeinsame `ESt`.
#[tokio::test]
async fn kist_an_gesamt_fall_1_beide_mitglied_bleibt_satz_mal_volle_est() {
    erwarte_kist_an_gesamt(&[
        ("A ev, B rk (konfessionsverschieden)", 2_000_000, "evangelisch", Some("roemisch-katholisch"), "nordrhein_westfalen", 1_383_800, 124_542),
        ("A ev, B ev (konfessionsgleich)", 2_000_000, "evangelisch", Some("evangelisch"), "nordrhein_westfalen", 1_383_800, 124_542),
    ])
    .await;
}

/// Fall 3: nur ein Ehegatte Mitglied: nur der Anteil des Mitglieds, nach dem Grundtarif auf die Summe der Einkuenfte.
#[tokio::test]
async fn kist_an_gesamt_fall_3_nur_ein_mitglied_zahlt_seinen_anteil() {
    erwarte_kist_an_gesamt(&[
        ("A ev, B keine: Anteil von A (91,25 %)", 2_000_000, "evangelisch", Some("keine"), "nordrhein_westfalen", 1_383_800, 113_648),
        ("A keine, B ev: Anteil von B (8,75 %), nicht 0", 2_000_000, "keine", Some("evangelisch"), "nordrhein_westfalen", 1_383_800, 10_893),
        ("Bayern 8 %, A ev, B keine", 2_000_000, "evangelisch", Some("keine"), "bayern", 1_383_800, 101_020),
        ("Bayern 8 %, A keine, B ev: Anteil von B", 2_000_000, "keine", Some("evangelisch"), "bayern", 1_383_800, 9_683),
        ("Alleinverdiener A ev, B keine: Anteil 1", 0, "evangelisch", Some("keine"), "nordrhein_westfalen", 823_800, 74_142),
        ("Alleinverdiener A keine, B ev: Anteil 0", 0, "keine", Some("evangelisch"), "nordrhein_westfalen", 823_800, 0),
    ])
    .await;
}

/// Die Werbungskosten von Person A mindern nur ihre Einkuenfte, nicht die von Person B (die hat hier nur den Pauschbetrag): A hat 220 Tage
/// mal 30 km Entfernungspauschale (220 x (20 x 0,30 + 10 x 0,38) = 2.156 EUR, Saetze aus `params/2025/`), Einkuenfte 57.844 und 18.770 EUR,
/// zvE 76.542, gemeinsame `ESt` 13.546 EUR, Grundtarif 13.582 und 1.336 EUR (Skript, Zeile `AN-EP`).
#[tokio::test]
async fn kist_an_gesamt_werbungskosten_von_person_a_mindern_nicht_die_einkuenfte_von_person_b() {
    let basis = mit(
        mit(kegel_an_gesamt(), kist_an_gesamt(2_000_000, "evangelisch", Some("keine"), "nordrhein_westfalen")),
        vec![("ep_arbeitstage", json!(220)), ("ep_entfernung_km", json!(30)), ("ep_eigenes_kfz", json!(true))],
    );
    erwarte_kist("an_gesamt, EP von A", &ergebnis("an_gesamt", &basis).await, 1_354_600, 110_995);
}

/// Der Beitrag von Person B zu einem Berufsverband (Abweichung Nr. 93) mindert ihre Einkuenfte erst ueber dem Pauschbetrag und damit ihren
/// Anteil an der gemeinsamen `ESt`. Fall wie Fall 3 (Lohn A 60.000, Lohn B 20.000 EUR, nur B Mitglied, NRW 9 %), von Hand nach Gesetz und
/// `params/2025/`: ohne Beitrag und mit 178 EUR (unter 1.230) Einkuenfte 58.770 und 18.770, `ESt` 13.838 EUR, Grundtarif 13.938 und 1.336 EUR,
/// Kirchensteuer von B 108,93 EUR. Mit 2.000 EUR Einkuenfte 58.770 und 18.000, zvE 76.698, `ESt` 13.596 EUR, Grundtarif 13.938 und 1.149 EUR,
/// Kirchensteuer von B 122.364 Cent mal 1.149 / 15.087 = 93,19 EUR.
#[tokio::test]
async fn kist_an_gesamt_beitrag_von_person_b_mindert_nur_ihre_einkuenfte() {
    for (name, beitrag_b, zahl, kist) in [
        ("178 EUR unter dem Pauschbetrag", 17_800, 1_383_800, 10_893),
        ("2.000 EUR darueber", 200_000, 1_359_600, 9_319),
    ] {
        let basis = mit(
            mit(kegel_an_gesamt(), kist_an_gesamt(2_000_000, "keine", Some("evangelisch"), "nordrhein_westfalen")),
            vec![("berufsverband_beitraege_partner", json!(beitrag_b))],
        );
        erwarte_kist(&format!("an_gesamt, B ev., {name}"), &ergebnis("an_gesamt", &basis).await, zahl, kist);
    }
}

/// Ohne Antwort zu Person B bleibt es wie bisher (Satz mal volle `ESt` nach Person A): die Antwort ist kein Kegel-Feld, die Zahl kommt auch ohne sie.
#[tokio::test]
async fn kist_an_gesamt_ohne_antwort_zu_person_b_rechnet_wie_bisher() {
    erwarte_kist_an_gesamt(&[("B unbeantwortet", 2_000_000, "evangelisch", None, "nordrhein_westfalen", 1_383_800, 124_542)]).await;
}
