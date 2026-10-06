//! Der Rand jedes Bereichsfelds an der HTTP-Naht (Weg B voll, Stufe 3, Waechter W9): fuer JEDES Feld, das in
//! der Registry (`rust/bindung/daten`) einen `bereich: {min, max}` traegt, weist `POST /fall/{id}/event`
//! `min - 1` und `max + 1` mit 422 ab, nimmt `min`, `max` und die 0 mit 201 an, und danach antworten
//! `GET /ergebnis` und `GET /stand` nie mit einem 500. Standardlauf, ohne Python, ohne `PARITY=1`.
//!
//! Gegenstueck zu `tests/test_bindung_bereich_serverseitig.py` (Python, `test_wert_ausserhalb_bereich_wird_abgewiesen`,
//! `test_null_unter_minimum_wird_angenommen`, `test_wert_im_bereich_endet_nicht_in_500`). Die Felder kommen LIVE aus der
//! Registry, keine feste Zahl: ein neues Bereichsfeld ist ab dem ersten Lauf dabei, ein Feld ohne Bereich faellt weg.
//! Der Python-Test pinnt "42 Felder, davon 3 Enum"; das tut dieser Test nicht (`C1` des Berichts, ENTFAELLT).
//!
//! Was welcher Teil bewacht:
//! - Schreibweg (422/201): Store (`bereich_verletzt`) UND die Verdrahtung in `api/src/event.rs`. Die Unit-Tests im Store
//!   sehen nur den Store; fehlte der Aufruf an der Naht (leerer `BindungNachschlag`), wuerden sie gruen bleiben.
//! - Ring (`< 500`): ein Randwert, der der Eingabe nach gueltig ist, darf den Rechenkern nicht mit einem Fehler beenden.
//!   Python hatte zwei solche Faelle (`rentner_renten_beginn_jahr` nach dem VZ, `rentner_alter_bei_rentenbeginn` 98..100,
//!   Schluessel 0..97 in `rente_ertragsanteil_p22.yaml`). Ein Rentenfeld laeuft deshalb unter JEDER Rentenart aus der
//!   Registry (`rentner_renten_art`), sonst erreichte der Randwert den bb-Zweig (Leibrente) nie.
//! - Die 0 bleibt zulaessig, auch unter einem Minimum > 0 ("nichts anzugeben", Vault
//!   `decisions/speichern-lehnt-nullwerte-nicht-ab`); fuer die Felder mit `enum_werte` gilt das nicht, sie sind die Kontrolle.
//!
//! Reichere Kegel (`reichere_kegel_machen_den_rand_im_ergebnis_wirksam`): Der Basis-Kegel (`gesamt`/`rentner_gesamt`, einzel
//! bzw. zusammen) setzt jedes Pflichtfeld, aber die Rechnung LIEST nicht jedes Bereichsfeld (z. B. `gewst_hebesatz` nur mit
//! `kein_gewinn = false` und `gewst_messbetrag`). Gemessen (Bericht `w-rand-auftrag2-messung.md`): im Basis-Kegel aendern nur
//! 12 von 42 Feldern die Antwort von `/ergebnis`. Acht weitere machen reine Fixture-Kegel wirksam (Ueberlagerung unveraendert
//! aus einem vorhandenen Test, Quelle je Gruppe unten); fuer sie prueft der zweite Test min und max bis in den Ring und haelt
//! fest, dass sich die Zahl zwischen min und max aendert (Positivkontrolle: sonst liest der Ring das Feld nicht mehr, und das
//! `< 500` waere leer wahr).
//!
//! Grenzen: 22 der 42 Felder pruefen hier nur den Schreibweg (422/201) und `< 500` bei gleicher Rechnung: 12 liest der Ring
//! nie (Zaehler und Flaechen fuer Formularfragen und die ELSTER-Erklaerung), 8 liest er, aber kein vorhandener Kegel macht sie
//! wirksam (`fam_monate_ohne_voraussetzung`, `versorgung_alter_bei_beginn`, `geburtsjahr_partner`, `am_anschaffung_monat`,
//! `uebernachtung_monate_bisher`, `vpf_tage_*_nach_drei_monaten`), 2 haengen an einem aus Fixture-Feldern zusammengesetzten
//! Kegel (`dhf_monate`, `arbeitsmittel_nutzungsdauer`). Die Positivkontrolle `basis_kegel_rechnen_und_tragen_eine_zahl` haelt
//! fest, dass jeder Basis-Kegel rechnet.
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
use bindung::{Bereich, Bindung};
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

// ---- Basis-Kegel (aus tests/_kegel.py::kegel_fuer, wie flag_und_schalter_hermetisch.rs) ---------------------

const GESAMT: &str = "gesamt";
const RENTNER: &str = "rentner_gesamt";
const ART: &str = "rentner_renten_art";
const ART_PARTNER: &str = "rentner_renten_art_partner";

/// Pflicht-Kegel `gesamt` (36 Felder), einzel, alle Kreuze "nein".
fn gesamt() -> Paare {
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
        ("kein_p23_verkauf", json!(true)),
    ]
}

/// Pflicht-Kegel `rentner_gesamt` (30 Felder), einzel: gesetzliche Rente von 200.000 EUR ab dem VZ (so hoch, dass Rentenart und
/// Alter bei Rentenbeginn die Steuer veraendern), mit Rentenfreibetrag 0, damit auch ein
/// frueherer Rentenbeginn rechnet (aa vor dem VZ braucht die Euro-Fixierung).
fn rentner() -> Paare {
    vec![
        (ART, json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(20_000_000)),
        ("rentner_renten_beginn_jahr", json!(2025)),
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
        ("kein_p23_verkauf", json!(true)),
        ("rentner_rentenfreibetrag", json!(0)),
    ]
}

/// Ein Feld ersetzen (gleiche Stelle) oder anhaengen.
fn mit(mut basis: Paare, feld: &'static str, wert: Value) -> Paare {
    match basis.iter_mut().find(|(f, _)| *f == feld) {
        Some(p) => p.1 = wert,
        None => basis.push((feld, wert)),
    }
    basis
}

/// Der Partner-Kegel: die Scheibe `gesamt` kennt `bruttoarbeitslohn_partner` und die fuenf KAP-Felder, `rentner_gesamt`
/// nur die KAP-Felder und die Partner-Rente (`flag_und_schalter_hermetisch.rs`: `partner_kap`, `partner_rente_voll`).
fn partner(scheibe: &str) -> Paare {
    let mut p: Paare = vec![
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ];
    if scheibe == GESAMT {
        p.push(("bruttoarbeitslohn_partner", json!(0)));
    } else {
        p.extend([
            ("rentner_jahresrente_partner", json!(15_000_000)),
            (ART_PARTNER, json!("gesetzliche_rente")),
            ("rentner_renten_beginn_jahr_partner", json!(2025)),
            ("rentner_alter_bei_rentenbeginn_partner", json!(65)),
            ("rentner_rentenfreibetrag_partner", json!(0)),
        ]);
    }
    p
}

/// Der Basis-Kegel der Scheibe; `zusammen` setzt die Zusammenveranlagung und den Partner-Kegel.
fn basis(scheibe: &str, zusammen: bool) -> Paare {
    let k = if scheibe == GESAMT {
        gesamt()
    } else {
        rentner()
    };
    if !zusammen {
        return k;
    }
    let mut k = mit(k, "veranlagung", json!("zusammen"));
    k.extend(partner(scheibe));
    k
}

// ---- Fall fahren ---------------------------------------------------------------------------------------------

struct Lauf {
    /// Status des Events, das das gepruefte Feld schreibt.
    feld: u16,
    /// Status und Meldung von `GET /ergebnis` und `GET /stand`.
    ergebnis: (u16, Value),
    stand: (u16, Value),
}

/// Legt Fall `id` der Scheibe an, schreibt jedes Paar der Basis ueber `POST /event` (bestaetigt, Nutzer-Klick) und das
/// gepruefte Feld mit `wert` an die Stelle, die es in der Basis hat (sonst am Ende); danach `/ergebnis` und `/stand`.
async fn fahre(d: &Dienst, id: &str, scheibe: &str, kegel: Paare, feld: &str, wert: i64) -> Lauf {
    let kopf = json!({"fall_id": id, "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall {id} {scheibe}: {antwort}");
    let mut paare: Vec<(String, Value)> =
        kegel.into_iter().map(|(f, w)| (f.to_owned(), w)).collect();
    match paare.iter_mut().find(|(f, _)| f == feld) {
        Some(p) => p.1 = json!(wert),
        None => paare.push((feld.to_owned(), json!(wert))),
    }
    let mut status_feld = 0;
    for (f, w) in &paare {
        let rumpf = json!({
            "feld_id": f, "wert": w, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{f}")},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, antwort) = sende(d, "POST", &format!("/fall/{id}/event"), Some(&rumpf)).await;
        if f == feld {
            status_feld = status;
        } else {
            assert_eq!(status, 201, "{id}: Basis-Kegel {f}: {status} {antwort}");
        }
    }
    Lauf {
        feld: status_feld,
        ergebnis: sende(d, "GET", &format!("/fall/{id}/ergebnis"), None).await,
        stand: sende(d, "GET", &format!("/fall/{id}/stand"), None).await,
    }
}

// ---- Die Bereichsfelder, live aus der Registry ---------------------------------------------------------------

struct Feld {
    id: String,
    bereich: Bereich,
    /// Die Felder mit `enum_werte`: die 0 steht nicht darin, sie sind die Kontrolle.
    mit_enum: bool,
}

fn registry() -> Vec<Bindung> {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    bindung::lade_registry_der_wurzel(&wurzel)
        .expect("Bindung laedt")
        .dateien
        .into_iter()
        .flat_map(|(_, d)| d.bindungen)
        .collect()
}

fn bereichsfelder(alle: &[Bindung]) -> Vec<Feld> {
    alle.iter()
        .filter_map(|b| {
            Some(Feld {
                id: b.feld_id.clone(),
                bereich: b.bereich.clone()?,
                mit_enum: b.enum_werte.is_some(),
            })
        })
        .collect()
}

/// Die Werte von `enum_werte` des Felds `id`; ohne das Feld oder ohne Liste ein leerer Vec.
fn enum_werte(alle: &[Bindung], id: &str) -> Vec<String> {
    alle.iter()
        .find(|b| b.feld_id == id)
        .and_then(|b| b.enum_werte.clone())
        .unwrap_or_default()
}

/// In welcher Scheibe wird das Feld geprueft? Rentenfelder zuerst in `rentner_gesamt` (dort liest sie der Ring), alle anderen
/// in `gesamt`; schreibt die Scheibe das Feld nicht (400), die andere. `Err`, wenn keine es schreibt: ein Bereichsfeld, das
/// niemand fragen kann, ist hier ein Befund.
async fn scheibe_fuer(d: &Dienst, f: &Feld, zaehler: &mut u32) -> Result<&'static str, String> {
    let reihe = if f.id.starts_with("rentner_") {
        [RENTNER, GESAMT]
    } else {
        [GESAMT, RENTNER]
    };
    for scheibe in reihe {
        *zaehler += 1;
        let zusammen = f.id.ends_with("_partner");
        let l = fahre(
            d,
            &format!("w9-sonde-{zaehler}"),
            scheibe,
            basis(scheibe, zusammen),
            &f.id,
            f.bereich.min,
        )
        .await;
        if l.feld != 400 {
            return Ok(scheibe);
        }
    }
    Err(format!(
        "{}: in keiner der Scheiben {reihe:?} schreibbar (beide 400)",
        f.id
    ))
}

fn unter500(l: &Lauf) -> Result<(), String> {
    for (route, (s, a)) in [("ergebnis", &l.ergebnis), ("stand", &l.stand)] {
        if *s >= 500 {
            return Err(format!("GET /{route}: {s} {a}"));
        }
    }
    Ok(())
}

/// Der Rand jedes Bereichsfelds, live aus der Registry. Siehe Moduldoku.
#[tokio::test]
async fn jedes_bereichsfeld_nimmt_den_rand_an_weist_den_nachbarn_ab_und_rechnet_ohne_500() {
    let alle = registry();
    let felder = bereichsfelder(&alle);
    // Positivkontrolle: ohne Bereichsfelder waere jede Aussage unten leer wahr. Die Zahl steht in der Meldung.
    assert!(
        !felder.is_empty(),
        "Registry hat 0 Bereichsfelder: die Rand-Pruefung liefe leer (Positivkontrolle)"
    );
    assert!(
        felder.iter().any(|f| f.mit_enum) && felder.iter().any(|f| !f.mit_enum && f.bereich.min > 0),
        "kein Enum-Bereichsfeld oder keines mit min > 0 unter {} Bereichsfeldern: die Zweige 'Enum lehnt 0 ab' und \
         'die 0 unter min ist zulaessig' liefen leer",
        felder.len()
    );
    let arten = enum_werte(&alle, ART);
    assert!(
        arten.len() >= 2 && enum_werte(&alle, ART_PARTNER).len() >= 2,
        "Rentenarten aus der Registry: {arten:?}: die Rentenart-Schleife liefe leer"
    );

    let d = dienst();
    let mut falsch: Vec<String> = Vec::new();
    let (mut n, mut faelle, mut sonden) = (0_u32, 0_u32, 0_u32);
    for f in &felder {
        let scheibe = match scheibe_fuer(&d, f, &mut sonden).await {
            Ok(s) => s,
            Err(e) => {
                falsch.push(e);
                continue;
            }
        };
        let zusammen = f.id.ends_with("_partner");
        let (min, max) = (f.bereich.min, f.bereich.max);
        // (Name, Wert, erwartetes Event, Rentenarten durchlaufen?). Die Abweisung haengt nicht von der Art ab.
        let mut plan: Vec<(&str, i64, u16, bool)> = Vec::new();
        for (name, w) in [("min-1", min - 1), ("max+1", max + 1)] {
            // Die 0 unter min bleibt zulaessig und wird weiter unten als Annahme geprueft.
            if w != 0 {
                plan.push((name, w, 422, false));
            }
        }
        plan.push(("min", min, 201, true));
        plan.push(("max", max, 201, true));
        if !f.mit_enum && min != 0 {
            plan.push(("0", 0, 201, true));
        }
        // Rentenfelder laufen unter jeder Rentenart der Registry (der Ring liest sie je Art anders).
        let art_feld = if zusammen { ART_PARTNER } else { ART };
        let rentenarten: Vec<Option<String>> = if scheibe == RENTNER {
            enum_werte(&alle, art_feld).into_iter().map(Some).collect()
        } else {
            vec![None]
        };
        for (name, wert, erwartet, je_art) in plan {
            let varianten: Vec<Option<String>> = if je_art {
                rentenarten.clone()
            } else {
                vec![None]
            };
            for art in varianten {
                n += 1;
                faelle += 1;
                let mut kegel = basis(scheibe, zusammen);
                if let Some(a) = &art {
                    kegel = mit(kegel, art_feld, json!(a));
                }
                let id = format!("w9-{n}");
                let l = fahre(&d, &id, scheibe, kegel, &f.id, wert).await;
                let kopf = format!("{} {name}={wert} (Scheibe {scheibe}, Art {art:?})", f.id);
                if l.feld != erwartet {
                    falsch.push(format!("{kopf}: Event {} statt {erwartet}", l.feld));
                }
                if let Err(e) = unter500(&l) {
                    falsch.push(format!("{kopf}: {e}"));
                }
            }
        }
    }
    assert!(
        falsch.is_empty(),
        "{} von {faelle} Faellen ueber {} Bereichsfelder verletzt:\n{falsch:#?}",
        falsch.len(),
        felder.len()
    );
}

/// Ein reicherer Kegel: die Basis der Scheibe `gesamt` plus eine Ueberlagerung, die EIN Bereichsfeld in `/ergebnis` wirksam macht.
struct Gruppe {
    feld: &'static str,
    zusammen: bool,
    ueberlagerung: Paare,
    /// Woher die Ueberlagerung stammt (nur fuer die Meldung).
    quelle: &'static str,
}

/// Die acht Gruppen mit reinem Fixture-Kegel (Bericht `w-rand-auftrag2-messung.md`). Jede Ueberlagerung steht so in der Quelle.
fn gruppen() -> Vec<Gruppe> {
    vec![
        Gruppe {
            feld: "gewst_hebesatz",
            zusammen: false,
            ueberlagerung: vec![
                ("kein_gewinn", json!(false)),
                ("einkuenfte_gewinn", json!(5_000_000)),
                ("gewinn_betriebsart", json!("gewerbe")),
                ("gewst_messbetrag", json!(150_000)),
                ("gewst_hebesatz", json!(400)),
            ],
            quelle: "kette_endstand_hermetisch.rs::g2_p35_gewerbesteuer_aenderungen",
        },
        Gruppe {
            feld: "gewst_hebesatz_partner",
            zusammen: true,
            ueberlagerung: vec![
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
            ],
            quelle: "kette_endstand_hermetisch.rs::g17_p35_credit_uebersteigt_die_steuer_aenderungen",
        },
        Gruppe {
            feld: "fam_anzahl_kinder",
            zusammen: true,
            ueberlagerung: vec![
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
            ],
            quelle: "kette_endstand_hermetisch.rs::g4_kind_freibetrag_p32d_aenderungen (der Kindergeld-Kegel g3 laesst das Feld stumm: Kindergeld siegt, die Steuer bleibt gleich)",
        },
        Gruppe {
            feld: "kind_grad_der_behinderung",
            zusammen: false,
            ueberlagerung: vec![
                ("fam_anzahl_kinder", json!(1)),
                ("kind_idnr", json!("12345678901")),
                ("kind_behinderten_pb_antrag", json!(true)),
                ("kind_pb_nicht_selbst_genutzt", json!(true)),
            ],
            quelle: "bescheid/src/abzuege.rs, Doctest von kind_behinderten_pb_daten",
        },
        Gruppe {
            feld: "ep_arbeitstage",
            zusammen: false,
            ueberlagerung: vec![
                ("ep_arbeitstage", json!(220)),
                ("ep_entfernung_km", json!(30)),
                ("am_anschaffungskosten", json!(80_000)),
                ("arbeitsmittel_nutzungsdauer", json!(3)),
                ("am_afa_ist_anschaffungsjahr", json!(false)),
                ("am_gwg_sofortabzug_gewaehlt", json!(true)),
            ],
            quelle: "kette_endstand_hermetisch.rs::g14_arbeitsmittel_genau_800_aenderungen",
        },
        Gruppe {
            feld: "vv_entgelt_quote_prozent",
            zusammen: false,
            ueberlagerung: vec![
                ("vv_einnahmen", json!(960_000)),
                ("vv_gebaeude_afa", json!(300_000)),
                ("vv_schuldzinsen", json!(250_000)),
                ("vv_erhaltungsaufwand", json!(80_000)),
                ("vv_sonstige_wk", json!(40_000)),
                ("vv_nebenkosten_umgelegt", json!(180_000)),
                ("kein_vuv", json!(false)),
            ],
            quelle: "fixtures/e2e/gesamt.json (die VV-Events; kein_vuv = false, sonst sperrt die Flag-Konsistenz)",
        },
        Gruppe {
            feld: "versorgung_beginn_jahr",
            zusammen: false,
            ueberlagerung: vec![
                ("versorgung_jahresrente", json!(1_800_000)),
                ("versorgung_bemessungsgrundlage", json!(1_500_000)),
                ("versorgung_beginn_jahr", json!(2020)),
            ],
            quelle: "bescheid/tests/sperre_scheiben_hermetisch.rs, Gruppe versorgung, Fall 'beides bestaetigt'",
        },
        Gruppe {
            feld: "uebernachtung_monate",
            zusammen: false,
            ueberlagerung: vec![
                ("uebernachtung_kosten_monat", json!(60_000)),
                ("uebernachtung_im_inland", json!(true)),
                ("uebernachtung_monate_bisher", json!(0)),
                ("uebernachtung_monate", json!(6)),
                ("uebernachtung_auswaerts", json!(true)),
                ("uebernachtung_alleinnutzung", json!(true)),
                ("uebernachtung_keine_lange_unterbrechung", json!(true)),
            ],
            quelle: "bescheid/tests/sperre_scheiben_hermetisch.rs, Gruppe wk, Fall 'Uebernachtung, Ort, Bedingungen, Zeitraum'",
        },
    ]
}

fn signatur(l: &Lauf) -> String {
    let (status, antwort) = &l.ergebnis;
    format!("{status}/{}/{}", antwort["grund"], antwort["zahl_cent"])
}

/// Die acht reichen Kegel: min und max werden angenommen, rechnen bis in den Ring ("bestaetigt", nie 500), und die Zahl
/// aendert sich zwischen min und max. Die 0 unter min > 0 (ausser Enum) bleibt zulaessig und rechnet ohne 500.
#[tokio::test]
async fn reichere_kegel_machen_den_rand_im_ergebnis_wirksam() {
    let alle = registry();
    let felder = bereichsfelder(&alle);
    let d = dienst();
    let mut falsch: Vec<String> = Vec::new();
    let mut n = 0_u32;
    let gruppen = gruppen();
    // Positivkontrolle: ohne Gruppen liefe die Schleife leer.
    assert!(
        gruppen.len() >= 8,
        "nur {} Gruppen: die reichen Kegel liefen leer",
        gruppen.len()
    );
    for g in &gruppen {
        let Some(f) = felder.iter().find(|f| f.id == g.feld) else {
            falsch.push(format!(
                "{}: kein Bereichsfeld mehr (Quelle {}): Gruppe pruefen oder streichen",
                g.feld, g.quelle
            ));
            continue;
        };
        let (min, max) = (f.bereich.min, f.bereich.max);
        let mut werte = vec![("min", min, true), ("max", max, true)];
        if !f.mit_enum && min > 0 {
            werte.push(("0", 0, false));
        }
        let mut signaturen: Vec<String> = Vec::new();
        for (name, wert, muss_rechnen) in werte {
            n += 1;
            let kegel = g
                .ueberlagerung
                .iter()
                .fold(basis(GESAMT, g.zusammen), |k, (feld, w)| {
                    mit(k, feld, w.clone())
                });
            let l = fahre(&d, &format!("w9-reich-{n}"), GESAMT, kegel, g.feld, wert).await;
            let kopf = format!("{} {name}={wert} (Kegel {})", g.feld, g.quelle);
            if l.feld != 201 {
                falsch.push(format!("{kopf}: Event {} statt 201", l.feld));
            }
            if let Err(e) = unter500(&l) {
                falsch.push(format!("{kopf}: {e}"));
            }
            if muss_rechnen {
                let (status, antwort) = &l.ergebnis;
                if *status != 200 || antwort["grund"] != "bestaetigt" {
                    falsch.push(format!(
                        "{kopf}: /ergebnis {status} {antwort}, erwartet 200 und grund bestaetigt"
                    ));
                }
                signaturen.push(signatur(&l));
            }
        }
        // Positivkontrolle je Gruppe: der Rand aendert die Zahl ueberhaupt.
        if signaturen.len() == 2 && signaturen[0] == signaturen[1] {
            falsch.push(format!(
                "{}: min und max geben dieselbe Antwort {} (Kegel {}): der Ring liest das Feld unter diesem Kegel nicht, \
                 das Rand-Ergebnis belegt nichts",
                g.feld, signaturen[0], g.quelle
            ));
        }
    }
    assert!(
        falsch.is_empty(),
        "{} Verstoesse in {} Gruppen:\n{falsch:#?}",
        falsch.len(),
        gruppen.len()
    );
}

/// Positivkontrolle: jeder Basis-Kegel rechnet ("bestaetigt", eine Zahl). Wuerde ein neues Pflichtfeld den Kegel unvollstaendig
/// machen, sperrte der Ring vor der Rechnung, und jedes `< 500` oben waere leer wahr.
#[tokio::test]
async fn basis_kegel_rechnen_und_tragen_eine_zahl() {
    let d = dienst();
    let mut falsch = Vec::new();
    let mut n = 0;
    for (scheibe, zusammen) in [
        (GESAMT, false),
        (GESAMT, true),
        (RENTNER, false),
        (RENTNER, true),
    ] {
        n += 1;
        // Das gepruefte Feld ist hier ein Basis-Feld mit seinem Basis-Wert.
        let (feld, wert) = if scheibe == GESAMT {
            ("ep_arbeitstage", 0)
        } else {
            ("rentner_pflegegrad", 0)
        };
        let lauf = fahre(
            &d,
            &format!("w9-basis-{n}"),
            scheibe,
            basis(scheibe, zusammen),
            feld,
            wert,
        )
        .await;
        let (status, antwort) = &lauf.ergebnis;
        if *status != 200 || antwort["grund"] != "bestaetigt" || !antwort["zahl_cent"].is_i64() {
            falsch.push(format!(
                "{scheibe} zusammen={zusammen}: /ergebnis {status} {antwort}"
            ));
        }
        if lauf.stand.0 != 200 {
            falsch.push(format!(
                "{scheibe} zusammen={zusammen}: /stand {} {}",
                lauf.stand.0, lauf.stand.1
            ));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}
