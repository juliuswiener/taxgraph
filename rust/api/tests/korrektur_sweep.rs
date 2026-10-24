//! Korrektur-Sweep (Auftrag `korrektur-sweep`, 2026-10-21, Anlass Fluss J F1 Nachtrag): rechnet ein Fall, dessen Antwort nachtraeglich
//! KORRIGIERT wurde, wie ein frischer Fall mit demselben Endwert?
//!
//! **Worum es geht.** Wer ein Feld korrigiert, schreibt ein neues Ereignis, das das alte ersetzt (`ersetzt`). Alles, was der Dienst aus
//! dem alten Wert abgeleitet hat (Geburtsjahr, Alters-Merker), muss mitziehen. Bis `7c23f673` blieb das abgeleitete Geburtsjahr
//! stehen: 292,00 EUR zu wenig Steuer (§ 24a). Der Sweep sucht diese Klasse mit dem Dienst, nicht mit einer Liste von Feldern.
//!
//! **Messebene.** In-process wie `kette_endstand_hermetisch.rs`: `POST /fall`, `POST /fall/{id}/event` (mit `ersetzt` fuer die
//! Korrektur, wie die Oberflaeche), `GET /fall/{id}/ergebnis` und `GET /fall/{id}/stand`. Kein echter Bestand, kein Browser.
//!
//! **Aufbau eines Versuchs.** Ausgangsfall `B` (Antworten in Schreibreihenfolge, aus einem Fragenlauf mit den Antworttabellen der
//! Browser-Fluesse I/J). K = `B` mit `f = v0`, dann `f` auf `v2` korrigiert. F = frischer Fall mit `B`, aber `f = v2`. Jede Abweichung
//! zwischen K und F (Ergebnis, Steuer, Sperre, Feldwerte und -zustaende) ist ein Fund. Art 2: `f` steht nicht in `B`; K schreibt
//! erst `v1`, korrigiert auf `v2`, F schreibt gleich `v2`. Art 3 (Nutzerpfad, nur an Gate-Feldern): K wird nach der Korrektur zu Ende
//! befragt, F ist ein frischer Fragenlauf mit dem Endwert; so zeigt sich, ob ein verborgener Block weiter zaehlt (Klasse K2). Art 4
//! (nur an Gate-Feldern): zweimal korrigiert (`v0 -> v1 -> v2`) gegen frischen Fall mit `v2`.
//!
//! **Aufruf** (nie im Gate, darum `#[ignore]`): `TAXGRAPH_KORREKTUR_LOG=<datei> cargo test -p api --test it korrektur_sweep:: -- --ignored
//! --nocapture`. Weitere Schalter: `TAXGRAPH_KORREKTUR_BASEN=a,b` (Namen der Ausgangsfaelle), `TAXGRAPH_KORREKTUR_FELDER=f1,f2`,
//! `TAXGRAPH_KORREKTUR_ART=1|2|3|4|123|…` (Standard 1234), `TAXGRAPH_KORREKTUR_GRENZE=<n>` (hoechstens n Felder je Ausgangsfall und Art).
//!
//! GRENZE: ein Versuch trifft nur die Werte und Ausgangsfaelle, die er probiert. Text-Felder variiert der Sweep nicht; Datumsfelder nur
//! mit sieben Geburtsdaten bzw. zwei Daten im Veranlagungsjahr. „Keine Abweichung gesehen“ heisst nicht „kein Fehler“.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::print_stderr
)]

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

/// Eine Antwort der Tabelle: Euro (wie getippt, bei `int` die Zahl selbst), Auswahl oder Text/Datum.
#[derive(Clone, Copy)]
enum Gib {
    Euro(&'static str),
    Wahl(&'static str),
    Text(&'static str),
}

type Tabelle = Vec<(&'static str, Gib)>;

/// Wie ein Wert in den Fall kam: als Antwort des Nutzers, als Bestaetigung eines Vorschlags (die Ableitung hatte schon einen Wert
/// vorlaeufig gesetzt, der Nutzer ersetzt ihn) oder gar nicht (der Dienst hat ihn selbst abgeleitet und bestaetigt).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Art {
    Antwort,
    Vorschlag,
    Abgeleitet,
}

/// Ein Ereignis des Ausgangsfalls.
#[derive(Clone)]
struct Ereignis {
    feld: String,
    wert: Value,
    signal_2: String,
    art: Art,
}

/// Ein Ausgangsfall: Scheibe, Veranlagungsjahr und die Antworten in Schreibreihenfolge.
struct Basis {
    name: &'static str,
    scheibe: &'static str,
    vz: i64,
    ereignisse: Vec<Ereignis>,
}

struct Dienst {
    zustand: Zustand,
    token: String,
    zaehler: AtomicU64,
    _tmp: tempfile::TempDir,
}

fn wurzel() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

impl Dienst {
    fn neu() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let konfig = Konfig { wurzel: wurzel(), faelle: tmp.path().join("faelle"), audit_dir: tmp.path().join("faelle") };
        let auth = Auth::neu("testgeheimnis".into(), tmp.path().join("users.json"), Some(konfig.audit_pfad()));
        let zustand = Zustand::neu(konfig, auth);
        let token = zustand.auth.stelle_aus("alice").unwrap();
        Self { zustand, token, zaehler: AtomicU64::new(0), _tmp: tmp }
    }

    async fn sende(&self, methode: &str, pfad: &str, body: Option<&Value>) -> (u16, Value) {
        let text = body.map(ToString::to_string);
        let mut b = Request::builder().method(methode).uri(pfad).header("authorization", format!("Bearer {}", self.token));
        if let Some(t) = &text {
            b = b.header("content-type", "application/json").header("content-length", t.len().to_string());
        }
        let req = b.body(text.map_or_else(Body::empty, Body::from)).unwrap();
        let r = app(self.zustand.clone()).oneshot(req).await.unwrap();
        let (teile, rumpf) = r.into_parts();
        let bytes = rumpf.collect().await.unwrap().to_bytes();
        (teile.status.as_u16(), serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn neuer_fall(&self, scheibe: &str, vz: i64) -> String {
        let id = format!("s{}", self.zaehler.fetch_add(1, Ordering::SeqCst));
        let (status, antwort) = self.sende("POST", "/fall", Some(&json!({"fall_id": id, "scheibe": scheibe, "veranlagungszeitraum": vz}))).await;
        assert_eq!(status, 201, "POST /fall: {antwort}");
        id
    }

    /// Schreibt eine bestaetigte Nutzerantwort (wie `ergebnis_vorlaeufig` in `kette_endstand_hermetisch.rs`).
    async fn schreibe(&self, fall: &str, e: &Ereignis) -> (u16, Value) {
        let rumpf = json!({
            "feld_id": e.feld, "wert": e.wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": e.signal_2},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        self.sende("POST", &format!("/fall/{fall}/event"), Some(&rumpf)).await
    }

    /// Die Korrektur wie in der Oberflaeche: ein neues Ereignis mit `ersetzt` auf das aktive Ereignis des Feldes.
    async fn korrigiere(&self, fall: &str, feld: &str, wert: &Value) -> (u16, Value) {
        let (_, stand) = self.sende("GET", &format!("/fall/{fall}/stand"), None).await;
        let alt = stand["felder"][feld]["event_id"].clone();
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": alt, "signal_2": "ok"}, "ersetzt": alt,
        });
        self.sende("POST", &format!("/fall/{fall}/event"), Some(&rumpf)).await
    }

    /// Was der Dienst zu einem Fall sagt: das Ergebnis und je Feld Wert und Zustand.
    async fn messe(&self, fall: &str) -> Messung {
        let (s1, mut ergebnis) = self.sende("GET", &format!("/fall/{fall}/ergebnis"), None).await;
        assert_eq!(s1, 200, "ergebnis: {ergebnis}");
        if let Value::Object(o) = &mut ergebnis {
            o.remove("fall_id");
            o.remove("snapshot_id");
        }
        let (s2, stand) = self.sende("GET", &format!("/fall/{fall}/stand"), None).await;
        assert_eq!(s2, 200, "stand: {stand}");
        let felder = stand["felder"]
            .as_object()
            .map(|o| o.iter().map(|(k, v)| (k.clone(), (v["wert"].clone(), v["zustand"].as_str().unwrap_or("").to_owned()))).collect())
            .unwrap_or_default();
        Messung { ergebnis, felder }
    }

    /// Schreibt eine Antwort. Hat das Feld schon ein Ereignis, hat der Dienst es entweder abgeleitet und bestaetigt (dann bleibt es,
    /// `Art::Abgeleitet`) oder als Vorschlag gesetzt (dann bestaetigt der Nutzer ihn: ersetzt mit `e.wert`, bei `uebernimm_vorschlag` mit
    /// dem vorgeschlagenen Wert, `Art::Vorschlag`). So macht es die Oberflaeche.
    async fn schreibe_robust(&self, fall: &str, e: &mut Ereignis, uebernimm_vorschlag: bool) -> (u16, Value) {
        let (s, a) = self.schreibe(fall, e).await;
        if !(s == 422 && a["fehler"].as_str().is_some_and(|t| t.contains("aktives Event"))) {
            return (s, a);
        }
        let (_, stand) = self.sende("GET", &format!("/fall/{fall}/stand"), None).await;
        let f = &stand["felder"][&e.feld];
        if f["zustand"] == json!("bestaetigt") {
            e.art = Art::Abgeleitet;
            e.wert = f["wert"].clone();
            return (201, Value::Null);
        }
        e.art = Art::Vorschlag;
        if uebernimm_vorschlag && !f["wert"].is_null() {
            e.wert = f["wert"].clone();
        }
        let (s, a) = self.korrigiere(fall, &e.feld, &e.wert).await;
        (if s == 200 { 201 } else { s }, a)
    }

    /// Ein Fall aus den Ereignissen; `ersetze` tauscht den Wert eines Feldes (Art 1), `haenge_an` schreibt am Ende (Art 2).
    async fn baue(&self, b: &Basis, ersetze: Option<(&str, &Value)>) -> String {
        let fall = self.neuer_fall(b.scheibe, b.vz).await;
        for e in &b.ereignisse {
            let mut e = e.clone();
            if let Some((f, w)) = ersetze {
                if e.feld == f {
                    e.wert = w.clone();
                }
            }
            if e.art == Art::Abgeleitet {
                continue;
            }
            let (status, antwort) = self.schreibe_robust(&fall, &mut e, false).await;
            assert!(status == 201, "{}: {} = {} wurde abgewiesen: {status} {antwort}", b.name, e.feld, e.wert);
        }
        fall
    }
}

struct Messung {
    ergebnis: Value,
    felder: BTreeMap<String, (Value, String)>,
}

impl Messung {
    fn zahl(&self) -> Option<i64> {
        self.ergebnis["zahl_cent"].as_i64()
    }
    fn kist(&self) -> Option<i64> {
        self.ergebnis["kist_cent"].as_i64()
    }
    fn grund(&self) -> String {
        self.ergebnis["grund"].as_str().unwrap_or("").to_owned()
    }
}

/// Die Stellen, an denen zwei Ergebnisse (JSON) auseinandergehen.
fn pfade(a: &Value, b: &Value, p: &str, aus: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()).collect::<std::collections::BTreeSet<_>>() {
                pfade(x.get(k).unwrap_or(&Value::Null), y.get(k).unwrap_or(&Value::Null), &format!("{p}/{k}"), aus);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (u, v)) in x.iter().zip(y).enumerate() {
                pfade(u, v, &format!("{p}/{i}"), aus);
            }
        }
        _ if a != b => aus.push(p.to_owned()),
        _ => {}
    }
}

/// Alle Abweichungen K gegen F: Stellen des Ergebnisses und Felder mit anderem Wert oder Zustand.
fn vergleiche(k: &Messung, f: &Messung) -> (Vec<String>, Vec<String>) {
    let mut ergebnis = Vec::new();
    pfade(&k.ergebnis, &f.ergebnis, "", &mut ergebnis);
    // Ereignis-Kennungen und Signale der Rechenspur sind je Fall verschieden, auch bei gleicher Rechnung: sie sind kein Fund.
    ergebnis.retain(|p| !(p.ends_with("/event_id") || p.contains("/signal/") || p.ends_with("/ts")));
    let mut felder = Vec::new();
    for name in k.felder.keys().chain(f.felder.keys()).collect::<std::collections::BTreeSet<_>>() {
        let (a, b) = (k.felder.get(name), f.felder.get(name));
        if a != b {
            let zeige = |x: Option<&(Value, String)>| x.map_or_else(|| "-".to_owned(), |(w, z)| format!("{w}/{z}"));
            felder.push(format!("{name}:{} gegen {}", zeige(a), zeige(b)));
        }
    }
    (ergebnis, felder)
}

// ---------------------------------------------------------------------------------------------------------------------------
// Ausgangsfaelle: die Antworttabellen der Browser-Fluesse I/J (`oberflaeche/fluss_i.rs`, `fluss_j.rs`), als Daten uebernommen.
// ---------------------------------------------------------------------------------------------------------------------------

fn person_a(geburtsdatum: &'static str) -> Tabelle {
    vec![
        ("stammdaten_nachname", Gib::Text("Mustermann")),
        ("stammdaten_vorname", Gib::Text("Max")),
        ("stammdaten_geburtsdatum", Gib::Text(geburtsdatum)),
        ("stammdaten_plz", Gib::Text("80331")),
        ("stammdaten_wohnort", Gib::Text("München")),
        ("stammdaten_strasse", Gib::Text("Marienplatz")),
        ("stammdaten_hausnummer", Gib::Text("1")),
        ("stammdaten_hausnummerzusatz", Gib::Text("a")),
        ("stammdaten_bic", Gib::Text("PBNKDEFF370")),
        ("stammdaten_iban", Gib::Text("DE02120300000000202051")),
        ("stammdaten_steuernummer", Gib::Text("9181081508155")),
        ("steuerklasse", Gib::Wahl("4")),
        ("bruttoarbeitslohn", Gib::Euro("60000")),
        ("versicherungsart", Gib::Wahl("gesetzlich_an")),
        ("ep_ziel_des_weges", Gib::Wahl("1")),
        ("ep_ziel_adresse", Gib::Text("80331 München, Marienplatz 1")),
    ]
}

fn person_b() -> Tabelle {
    // Die Zeilen von `person_b()` in `fluss_i.rs` (ohne `veranlagung`, die setzt der Aufrufer).
    vec![
        ("bruttoarbeitslohn_partner", Gib::Euro("20000")),
        ("stammdaten_vorname_partner", Gib::Text("Erika")),
        ("stammdaten_nachname_partner", Gib::Text("Mustermann")),
        ("stammdaten_geburtsdatum_partner", Gib::Text("09.07.1988")),
    ]
}

fn kirche(konf_a: &'static str, konf_b: Option<&'static str>) -> Tabelle {
    let mut t = vec![("kist_konfession", Gib::Wahl(konf_a)), ("kist_bundesland", Gib::Wahl("bayern"))];
    if let Some(b) = konf_b {
        t.push(("kist_konfession_partner", Gib::Wahl(b)));
    }
    t
}

fn rente(partner: bool, beginn: &'static str, euro: &'static str) -> Tabelle {
    if partner {
        vec![
            ("rentner_renten_art_partner", Gib::Wahl("gesetzliche_rente")),
            ("rentner_renten_beginn_jahr_partner", Gib::Euro(beginn)),
            ("rentner_jahresrente_partner", Gib::Euro(euro)),
        ]
    } else {
        vec![
            ("rentner_renten_art", Gib::Wahl("gesetzliche_rente")),
            ("rentner_renten_beginn_jahr", Gib::Euro(beginn)),
            ("rentner_jahresrente", Gib::Euro(euro)),
        ]
    }
}

/// (Name, Scheibe, Antworten, Kreuze der Ankreuzliste, Beschreibung). Kreuz heisst „ja, das gab es“.
fn ausgangsfaelle() -> Vec<(&'static str, &'static str, Tabelle, &'static [&'static str])> {
    let einzel = |gd: &'static str, konf: &'static str| {
        let mut t = person_a(gd);
        t.push(("veranlagung", Gib::Wahl("einzel")));
        t.extend(kirche(konf, None));
        t
    };
    let paar = |konf_a: &'static str, konf_b: &'static str| {
        let mut t = person_a("15.03.1985");
        t.push(("veranlagung", Gib::Wahl("zusammen")));
        t.extend(person_b());
        t.extend(kirche(konf_a, Some(konf_b)));
        t
    };
    let mut alleinverdiener = paar("keine", "keine");
    alleinverdiener.retain(|(f, _)| *f != "bruttoarbeitslohn_partner");
    let mut rentner_einzel = einzel("15.03.1955", "keine");
    rentner_einzel.extend(rente(false, "2015", "18000"));
    let mut rentner_paar = person_a("15.03.1955");
    rentner_paar.push(("veranlagung", Gib::Wahl("zusammen")));
    rentner_paar.extend(kirche("keine", Some("keine")));
    rentner_paar.extend(rente(false, "2015", "18000"));
    rentner_paar.extend(rente(true, "2018", "12000"));
    // Zwei Kinder (unter 14): der Zaehler und je Kind das Geburtsdatum; die uebrigen Kind-Felder beantwortet der neutrale Standard.
    let kinder = |mut t: Tabelle| {
        t.push(("fam_anzahl_kinder", Gib::Euro("2")));
        t.push(("kind_geburtsdatum", Gib::Text("15.03.2018")));
        t.push(("kind_geburtsdatum__2", Gib::Text("20.06.2021")));
        // Betreuungskosten (§ 10 Abs. 1 Nr. 5 EStG): nur dann wirkt das Alter des Kindes (unter 14) auf die Steuer.
        t.push(("kinderbetreuungskosten", Gib::Euro("4000")));
        t.push(("kinderbetreuungskosten__2", Gib::Euro("3000")));
        for f in ["kind_betreuung_reine_betreuung", "kind_betreuung_reine_betreuung__2", "kind_betreuung_rechnung_ueberweisung", "kind_betreuung_rechnung_ueberweisung__2"] {
            t.push((f, Gib::Wahl("true")));
        }
        t
    };
    vec![
        ("einzel_zwei_kinder", "gesamt", kinder(einzel("15.03.1985", "keine")), &["kein_kind"]),
        ("ehepaar_zwei_kinder", "gesamt", kinder(paar("keine", "keine")), &["kein_kind"]),
        ("einzel_1955", "gesamt", einzel("15.03.1955", "keine"), &[]),
        ("einzel_1985", "gesamt", einzel("15.03.1985", "keine"), &[]),
        ("einzel_evangelisch", "gesamt", einzel("15.03.1985", "evangelisch"), &[]),
        ("ehepaar", "gesamt", paar("keine", "keine"), &[]),
        ("ehepaar_evangelisch", "gesamt", paar("evangelisch", "keine"), &[]),
        ("alleinverdiener", "gesamt", alleinverdiener, &[]),
        ("rentner_einzel", "rentner_gesamt", rentner_einzel, &["kein_sonstige"]),
        ("rentner_ehepaar", "rentner_gesamt", rentner_paar, &["kein_sonstige"]),
    ]
}

/// Der Betrag einer Euro-Angabe in Cent („60000“, „19.99“); bei `int` die Zahl selbst.
fn euro_wert(t: &str, typ: &str) -> Value {
    if typ == "int" {
        return json!(t.parse::<i64>().unwrap());
    }
    let (e, c) = t.split_once('.').unwrap_or((t, "0"));
    let c = format!("{c:0<2}");
    json!(e.parse::<i64>().unwrap() * 100 + c[..2].parse::<i64>().unwrap())
}

/// Der Wert zu einer Frage: Tabelle, sonst der neutrale Standard der Oberflaeche (Geld und Zahl 0, Ja/Nein „Nein“, bei
/// `frage_invertiert` und der Ankreuzliste das Gegenteil davon).
fn antwort(q: &Value, instanz: &str, tabelle: &Tabelle, kreuze: &[&str], ueberschreibe: &[(&str, Value)]) -> Value {
    let basis = q["feld_id"].as_str().unwrap();
    // Die Tabelle darf eine Instanz (`feld__2`) eigens belegen; sonst gilt der Eintrag des Basisfeldes.
    let feld = if tabelle.iter().any(|(f, _)| *f == instanz) { instanz } else { basis };
    let typ = q["typ"].as_str().unwrap();
    let invertiert = q["frage_invertiert"] == json!(true);
    // Der Endwert einer Korrektur gilt nur fuer genau die Instanz, die korrigiert wurde.
    if let Some((_, w)) = ueberschreibe.iter().find(|(f, _)| *f == instanz) {
        return w.clone();
    }
    if typ == "bool" {
        // Eine Tabelle darf einen Ja/Nein-Wert vorgeben (Wahl „true“/„false“, der gespeicherte Wert); sonst gilt der neutrale Standard.
        if let Some((_, Gib::Wahl(w))) = tabelle.iter().find(|(f, _)| *f == feld) {
            return json!(*w == "true");
        }
        let ja = kreuze.contains(&feld);
        return json!(if invertiert { !ja } else { ja });
    }
    match (tabelle.iter().find(|(f, _)| *f == feld).map(|(_, g)| *g), typ) {
        (Some(Gib::Euro(t)), "cent" | "int") => euro_wert(t, typ),
        (Some(Gib::Wahl(w)), "enum") => json!(w),
        (Some(Gib::Text(t)), "text" | "datum") => json!(t),
        (None, "cent" | "int") => json!(0),
        (None, "enum") => q["enum_werte"][0].clone(),
        (None, "text") => json!(q["beispielwert"].as_str().unwrap_or("Muster")),
        (None, "datum") => json!("01.01.1960"),
        (g, t) => panic!("{feld}: Typ {t} passt nicht zur Tabelle ({})", g.is_some()),
    }
}

/// Beantwortet Frage fuer Frage, wie es die Oberflaeche tut, und gibt die Antworten in Schreibreihenfolge zurueck.
async fn fragenlauf(d: &Dienst, name: &'static str, scheibe: &'static str, vz: i64, tabelle: &Tabelle, kreuze: &[&str]) -> Basis {
    let fall = d.neuer_fall(scheibe, vz).await;
    let ereignisse = fragenlauf_ab(d, &fall, name, tabelle, kreuze, &[], Vec::new()).await;
    Basis { name, scheibe, vz, ereignisse }
}

/// Beantwortet die offenen Fragen eines Falls, der schon `ereignisse` traegt, bis `/fragen` leer ist. `ueberschreibe` setzt Werte
/// vor die Tabelle (der Endwert einer Korrektur).
async fn fragenlauf_ab(
    d: &Dienst,
    fall: &str,
    name: &str,
    tabelle: &Tabelle,
    kreuze: &[&str],
    ueberschreibe: &[(&str, Value)],
    mut ereignisse: Vec<Ereignis>,
) -> Vec<Ereignis> {
    for schritt in 0..700 {
        let (status, fr) = d.sende("GET", &format!("/fall/{fall}/fragen"), None).await;
        assert_eq!(status, 200, "{name}: fragen: {fr}");
        let Some(q) = fr["fragen"].as_array().and_then(|a| a.first()).cloned() else {
            return ereignisse;
        };
        let fid = q["feld_id"].as_str().unwrap().to_owned();
        let anzahl = q["instanz_anzahl"].as_u64().unwrap_or(1).max(1);
        let mut neu = false;
        for k in 1..=anzahl {
            let feld = if k == 1 { fid.clone() } else { format!("{fid}__{k}") };
            if ereignisse.iter().any(|e| e.feld == feld) {
                continue;
            }
            let wert = antwort(&q, &feld, tabelle, kreuze, ueberschreibe);
            let signal_2 = if q["screening"] == json!(true) { format!("screening@{feld}") } else { format!("ok@{feld}") };
            let mut e = Ereignis { feld: feld.clone(), wert, signal_2, art: Art::Antwort };
            let (s, a) = d.schreibe_robust(fall, &mut e, true).await;
            assert!(s == 201, "{name}: Schritt {schritt}: {feld} = {} abgewiesen: {s} {a}", e.wert);
            ereignisse.push(e);
            neu = true;
        }
        assert!(neu, "{name}: die Frage {fid} kommt nach der Antwort wieder (Schritt {schritt})");
    }
    panic!("{name}: ueber 700 Fragen");
}

// ---------------------------------------------------------------------------------------------------------------------------
// Wertepaare
// ---------------------------------------------------------------------------------------------------------------------------

fn basisname(feld: &str) -> &str {
    match feld.rsplit_once("__") {
        Some((b, n)) if n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty() => b,
        _ => feld,
    }
}

/// Andere Werte zu einem Feld: Ja/Nein das Gegenteil, Geld und Zahl wenige Werte im Bereich, Auswahl jede andere Option, Datum je
/// nach Art. Text gar nicht.
fn kandidaten(b: &bindung::Bindung, v0: &Value, vz: i64) -> Vec<Value> {
    use domain::Feldtyp;
    let mut v: Vec<Value> = match b.typ {
        Feldtyp::Bool => vec![json!(true), json!(false)],
        Feldtyp::Enum => b.enum_werte.iter().flatten().map(|w| json!(w)).collect(),
        Feldtyp::Cent | Feldtyp::Int => {
            let roh: Vec<i64> = match (&b.bereich, b.typ) {
                (Some(r), _) => vec![r.min, r.min + (r.max - r.min) / 2, r.max],
                (None, Feldtyp::Cent) => vec![0, 100_000, 5_000_000],
                (None, _) => vec![0, 1, 2, 10],
            };
            roh.into_iter().filter(|x| !b.nicht_negativ || *x >= 0).map(|x| json!(x)).collect()
        }
        Feldtyp::Datum if b.feld_id.contains("geburt") => {
            ["15.03.1955", "31.12.1960", "01.01.1961", "15.03.1985", "15.03.2008", "15.03.2012", "15.03.2018"].iter().map(|d| json!(d)).collect()
        }
        Feldtyp::Datum => vec![json!(format!("01.03.{vz}")), json!(format!("01.09.{vz}"))],
        Feldtyp::Text => Vec::new(),
    };
    v.retain(|x| x != v0);
    v
}

// ---------------------------------------------------------------------------------------------------------------------------
// Der Sweep
// ---------------------------------------------------------------------------------------------------------------------------

#[derive(Default)]
struct Zaehler {
    versuche: usize,
    abgewiesen: usize,
    gleich: usize,
    stand_nur: usize,
    euro: usize,
}

struct Log(Option<std::fs::File>);

impl Log {
    fn zeile(&mut self, z: &str) {
        eprintln!("{z}");
        if let Some(f) = &mut self.0 {
            writeln!(f, "{z}").unwrap();
        }
    }
}

fn euro_delta(k: Option<i64>, f: Option<i64>) -> String {
    match (k, f) {
        (Some(a), Some(b)) => (a - b).to_string(),
        (None, None) => "0".to_owned(),
        _ => "Zahl<->Sperre".to_owned(),
    }
}

/// Was Art 3 braucht, um die offenen Fragen wie der Nutzer zu beantworten.
struct Kontext<'a> {
    tabelle: &'a Tabelle,
    kreuze: &'a [&'a str],
}

/// Ein Versuch: K gegen F. Schreibt eine Zeile ins Log, wenn sie abweichen; zaehlt in jedem Fall.
///
/// Art 1: F schreibt die Antworten des Ausgangsfalls mit dem Endwert. Art 2: das Feld fehlt im Ausgangsfall. Art 3 (Nutzerpfad): K
/// wird nach der Korrektur zu Ende befragt (neutrale Antworten auf alles, was neu gefragt wird), F ist ein frischer Fragenlauf mit
/// dem Endwert; Antworten in einem Block, den die Korrektur verborgen hat, stehen nur in K. Art 4 (zweimal korrigiert): K wird auf
/// `neu` und danach auf `dritte` korrigiert, F ist der frische Fall mit `dritte`; so zeigt sich, ob eine zweite Korrektur den Wert der
/// ersten zurueckholt.
#[allow(clippy::too_many_arguments)]
async fn versuche(d: &Dienst, b: &Basis, ctx: &Kontext<'_>, art: u8, feld: &str, alt: &Value, neu: &Value, dritte: Option<&Value>, z: &mut Zaehler, log: &mut Log) {
    z.versuche += 1;
    // K: Ausgangsfall mit dem alten Wert (Art 1/3: der Wert der Basis; Art 2: `alt` am Ende), dann korrigiert auf `neu`.
    let k_fall = d.baue(b, None).await;
    if art == 2 {
        let e = Ereignis { feld: feld.to_owned(), wert: alt.clone(), signal_2: format!("ok@{feld}"), art: Art::Antwort };
        let (s, _) = d.schreibe(&k_fall, &e).await;
        if s != 201 {
            z.abgewiesen += 1;
            return;
        }
    }
    let (s, a) = d.korrigiere(&k_fall, feld, neu).await;
    if s != 200 && s != 201 {
        z.abgewiesen += 1;
        let _ = a;
        return;
    }
    // Art 4: eine zweite Korrektur. Der Endwert ist dann `dritte`.
    let endwert = dritte.unwrap_or(neu);
    if let Some(w) = dritte {
        let (s, _) = d.korrigiere(&k_fall, feld, w).await;
        if s != 200 && s != 201 {
            z.abgewiesen += 1;
            return;
        }
    }
    // Art 3: K zu Ende befragen, F frisch befragen.
    let f_fall = if art == 3 {
        let mut ev = b.ereignisse.clone();
        if let Some(e) = ev.iter_mut().find(|e| e.feld == feld) {
            e.wert = neu.clone();
        }
        fragenlauf_ab(d, &k_fall, b.name, ctx.tabelle, ctx.kreuze, &[], ev).await;
        let fall = d.neuer_fall(b.scheibe, b.vz).await;
        fragenlauf_ab(d, &fall, b.name, ctx.tabelle, ctx.kreuze, &[(feld, neu.clone())], Vec::new()).await;
        fall
    } else if art == 1 || art == 4 {
        d.baue(b, Some((feld, endwert))).await
    } else {
        let fall = d.baue(b, None).await;
        let e = Ereignis { feld: feld.to_owned(), wert: neu.clone(), signal_2: format!("ok@{feld}"), art: Art::Antwort };
        let (s, _) = d.schreibe(&fall, &e).await;
        assert_eq!(s, 201, "F-Fall: {feld} = {neu} abgewiesen, K hat es angenommen");
        fall
    };
    let (mk, mf) = (d.messe(&k_fall).await, d.messe(&f_fall).await);
    let (ergebnis, felder) = vergleiche(&mk, &mf);
    if ergebnis.is_empty() && felder.is_empty() {
        z.gleich += 1;
        return;
    }
    let klasse = if mk.zahl() != mf.zahl() || mk.kist() != mf.kist() || mk.grund() != mf.grund() {
        z.euro += 1;
        "EUR"
    } else {
        z.stand_nur += 1;
        "STAND"
    };
    log.zeile(&format!(
        "FUND\t{}\t{art}\t{feld}\t{alt}\t{neu}{}\t{klasse}\tK {:?}\tF {:?}\tdelta {}\tKist K {:?} F {:?}\tgrund {} / {}\tErgebnis: {}\tFelder: {}",
        b.name,
        dritte.map_or_else(String::new, |w| format!(" > {w}")),
        mk.zahl(),
        mf.zahl(),
        euro_delta(mk.zahl(), mf.zahl()),
        mk.kist(),
        mf.kist(),
        mk.grund(),
        mf.grund(),
        ergebnis.iter().take(6).cloned().collect::<Vec<_>>().join(" "),
        felder.iter().take(6).cloned().collect::<Vec<_>>().join(" | ")
    ));
}

fn liste_aus_env(name: &str) -> Option<Vec<String>> {
    std::env::var(name).ok().filter(|v| !v.is_empty()).map(|v| v.split(',').map(str::to_owned).collect())
}

async fn sweep_basis(
    name: &'static str,
    scheibe: &'static str,
    tabelle: Tabelle,
    kreuze: &'static [&'static str],
    bindungen: &BTreeMap<String, bindung::Bindung>,
    log_pfad: Option<String>,
) -> Zaehler {
    let d = Dienst::neu();
    let vz = 2025;
    let b = fragenlauf(&d, name, scheibe, vz, &tabelle, kreuze).await;
    let mut log = Log(log_pfad.map(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap()));
    let basis_messung = d.messe(&d.baue(&b, None).await).await;
    log.zeile(&format!(
        "BASIS\t{name}\t{scheibe}\tEreignisse {}\tSteuer {:?}\tKist {:?}\tgrund {}\tSchluessel {}",
        b.ereignisse.len(),
        basis_messung.zahl(),
        basis_messung.kist(),
        basis_messung.grund(),
        basis_messung.ergebnis.as_object().map_or_else(String::new, |o| o.keys().cloned().collect::<Vec<_>>().join(","))
    ));
    let arten = std::env::var("TAXGRAPH_KORREKTUR_ART").unwrap_or_else(|_| "1234".to_owned());
    let nur = liste_aus_env("TAXGRAPH_KORREKTUR_FELDER");
    let grenze: usize = std::env::var("TAXGRAPH_KORREKTUR_GRENZE").ok().and_then(|g| g.parse().ok()).unwrap_or(usize::MAX);
    let mut z = Zaehler::default();
    let mut nicht_variiert: Vec<String> = Vec::new();
    let ctx = Kontext { tabelle: &tabelle, kreuze };
    let gates = gate_felder(bindungen);
    // Art 1, 3 und 4: jedes beantwortete Feld des Ausgangsfalls, jeder andere Wert. Art 3 und 4 nur an Feldern, an denen andere Fragen
    // haengen oder aus denen etwas abgeleitet wird.
    for art in [1_u8, 3, 4] {
        if !arten.contains(char::from(b'0' + art)) {
            continue;
        }
        let mut n = 0;
        for e in &b.ereignisse {
            if e.art == Art::Abgeleitet || nur.as_ref().is_some_and(|l| !l.contains(&e.feld)) || (art >= 3 && !gates.contains(basisname(&e.feld))) {
                continue;
            }
            let Some(bd) = bindungen.get(basisname(&e.feld)) else {
                if art == 1 {
                    nicht_variiert.push(format!("{} (keine Bindung)", e.feld));
                }
                continue;
            };
            let werte = kandidaten(bd, &e.wert, vz);
            if werte.is_empty() || !bd.askable {
                if art == 1 {
                    nicht_variiert.push(format!("{} ({})", e.feld, if bd.askable { "Text" } else { "nicht beantwortbar" }));
                }
                continue;
            }
            n += 1;
            if n > grenze {
                break;
            }
            for w in &werte {
                // Art 4 braucht einen dritten Wert, der weder der alte noch der zweite ist.
                let dritte = if art == 4 { werte.iter().find(|x| *x != w && **x != e.wert) } else { None };
                if art == 4 && dritte.is_none() {
                    continue;
                }
                versuche(&d, &b, &ctx, art, &e.feld, &e.wert, w, dritte, &mut z, &mut log).await;
            }
        }
    }
    // Art 2: beantwortbare Felder, die der Ausgangsfall nicht hat.
    if arten.contains('2') {
        let vorhanden: std::collections::BTreeSet<&str> = b.ereignisse.iter().map(|e| e.feld.as_str()).collect();
        let mut n = 0;
        for (fid, bd) in bindungen {
            if vorhanden.contains(fid.as_str()) || !bd.askable || bd.ableitung.is_some() {
                continue;
            }
            if nur.as_ref().is_some_and(|l| !l.contains(fid)) {
                continue;
            }
            let werte = kandidaten(bd, &Value::Null, vz);
            if werte.len() < 2 {
                continue;
            }
            n += 1;
            if n > grenze {
                break;
            }
            // Zwei Paare: erster -> zweiter Wert und zurueck.
            versuche(&d, &b, &ctx, 2, fid, &werte[0], &werte[1], None, &mut z, &mut log).await;
            versuche(&d, &b, &ctx, 2, fid, &werte[1], &werte[0], None, &mut z, &mut log).await;
        }
    }
    log.zeile(&format!(
        "SUMME\t{name}\tVersuche {}\tabgewiesen {}\tgleich {}\tnur Stand {}\tEUR/Sperre {}\tnicht variiert {}",
        z.versuche,
        z.abgewiesen,
        z.gleich,
        z.stand_nur,
        z.euro,
        nicht_variiert.len()
    ));
    for f in &nicht_variiert {
        log.zeile(&format!("NICHT_VARIIERT\t{name}\t{f}"));
    }
    z
}

/// Felder, an denen andere Fragen haengen: Gates, Kreuze der Ankreuzliste, Quellen einer Ableitung und jedes Feld, das eine
/// `feld_bedingung` liest.
fn gate_felder(bindungen: &BTreeMap<String, bindung::Bindung>) -> std::collections::BTreeSet<String> {
    let mut g = std::collections::BTreeSet::new();
    for b in bindungen.values() {
        if b.gate == Some(true) || b.screening == Some(true) {
            g.insert(b.feld_id.clone());
        }
        if let Some(a) = &b.ableitung {
            g.insert(a.aus.clone());
            g.extend(a.und_feld.clone());
        }
        if let Some(fb) = &b.feld_bedingung {
            g.extend(fb.kette().map(|k| k.feld.clone()));
        }
    }
    g
}

fn lade_bindungen() -> BTreeMap<String, bindung::Bindung> {
    let reg = bindung::lade_registry_der_wurzel(&wurzel()).expect("Registry");
    reg.dateien.into_iter().flat_map(|(_, d)| d.bindungen).map(|b| (b.feld_id.clone(), b)).collect()
}

/// Der Sweep ueber alle Ausgangsfaelle (oder die, die `TAXGRAPH_KORREKTUR_BASEN` nennt), je Ausgangsfall ein Thread.
#[test]
#[ignore = "Messung: TAXGRAPH_KORREKTUR_LOG=<datei> setzen und mit --ignored aufrufen"]
fn korrektur_sweep_ueber_alle_ausgangsfaelle() {
    let bindungen = lade_bindungen();
    let nur = liste_aus_env("TAXGRAPH_KORREKTUR_BASEN");
    let log_pfad = std::env::var("TAXGRAPH_KORREKTUR_LOG").ok();
    let faelle: Vec<_> = ausgangsfaelle().into_iter().filter(|(n, ..)| nur.as_ref().is_none_or(|l| l.iter().any(|x| x == n))).collect();
    assert!(!faelle.is_empty(), "kein Ausgangsfall gewaehlt");
    let summe: Vec<(&str, Zaehler)> = std::thread::scope(|s| {
        let hs: Vec<_> = faelle
            .into_iter()
            .map(|(name, scheibe, tabelle, kreuze)| {
                let (bindungen, log_pfad) = (&bindungen, log_pfad.clone());
                s.spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                    (name, rt.block_on(sweep_basis(name, scheibe, tabelle, kreuze, bindungen, log_pfad)))
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for (name, z) in &summe {
        eprintln!("ENDE {name}: Versuche {}, abgewiesen {}, gleich {}, nur Stand {}, EUR/Sperre {}", z.versuche, z.abgewiesen, z.gleich, z.stand_nur, z.euro);
    }
}

/// Die Positivkontrolle des Auftrags: Lohn 60.000 EUR, ein Nutzer, Geburtsdatum erst 15.03.1955, dann auf 15.03.1985 korrigiert. Der
/// frische Fall mit 1985 kostet 13.924,00 EUR (Handrechnung Fluss I: 1.392.400 Cent). Auf `c1665ce5` rechnet die Korrektur mit dem
/// Geburtsjahr 1955 weiter und liefert 13.632,00 EUR; ab `7c23f673` liefert sie 13.924,00 EUR. Der Test haelt den frischen Fall fest und
/// druckt, welche der zwei Zahlen die Korrektur liefert (`KONTROLLE`).
#[test]
#[ignore = "Messung: mit --ignored --nocapture aufrufen, die Zeile KONTROLLE nennt die Zahlen"]
fn kontrolle_geburtsdatum_1955_auf_1985() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let d = Dienst::neu();
        let (_, scheibe, tabelle, kreuze) = ausgangsfaelle().into_iter().find(|(n, ..)| *n == "einzel_1955").unwrap();
        let b = fragenlauf(&d, "einzel_1955", scheibe, 2025, &tabelle, kreuze).await;
        let k = d.baue(&b, None).await;
        let (s, a) = d.korrigiere(&k, "stammdaten_geburtsdatum", &json!("15.03.1985")).await;
        assert!(s == 200 || s == 201, "Korrektur abgewiesen: {s} {a}");
        let (mk, mf) = (d.messe(&k).await, d.messe(&d.baue(&b, Some(("stammdaten_geburtsdatum", &json!("15.03.1985")))).await).await);
        let alt = d.messe(&d.baue(&b, None).await).await;
        eprintln!(
            "KONTROLLE Steuer {:?} Cent bei 1955, {:?} Cent nach der Korrektur auf 1985, {:?} Cent im frischen Fall 1985; geburtsjahr K {:?} F {:?}",
            alt.zahl(),
            mk.zahl(),
            mf.zahl(),
            mk.felder.get("geburtsjahr"),
            mf.felder.get("geburtsjahr")
        );
        assert_eq!(mf.zahl(), Some(1_392_400), "der frische Fall 1985 ist die Handrechnung aus Fluss I");
    });
}

/// Blindheitskontrolle des Sweeps (AK1). Seit `7c23f673` gibt es keinen Defekt der Klasse mehr, den der Sweep sucht; die Kontrolle
/// oben prueft nur noch den frischen Fall und wuerde auch bei einem leeren Vergleich gruen bleiben. Hier schleppt der Test den
/// alten Fehler ueber die oeffentliche Schnittstelle ein: ein vom Nutzer bestaetigtes `geburtsjahr` 1955, VOR dem Geburtsdatum
/// geschrieben (danach liefe `schreibe_robust` in den Zweig „bestaetigt, bleibt“ und verwuerfe es). Die Antwort des Nutzers am
/// Ziel bleibt stehen, auch wenn das Geburtsdatum auf 1985 korrigiert wird; K rechnet mit 1955 (1.363.200 Cent), der frische Fall
/// F ohne dieses Ereignis mit 1985 (1.392.400 Cent). `versuche` (Art 3, Nutzerpfad) muss das als Euro-Fund zaehlen und loggen.
/// Die Mutante `mk.zahl() != mf.zahl() || …` -> `false` in `versuche` faerbt den Test rot.
#[test]
fn eingeschleppter_altwert_wird_als_euro_fund_gesehen() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let d = Dienst::neu();
        let (_, scheibe, tabelle, kreuze) = ausgangsfaelle().into_iter().find(|(n, ..)| *n == "einzel_1955").unwrap();
        let mut b = fragenlauf(&d, "einzel_1955", scheibe, 2025, &tabelle, kreuze).await;
        let stelle = b.ereignisse.iter().position(|e| e.feld == "stammdaten_geburtsdatum").unwrap();
        b.ereignisse.insert(stelle, Ereignis { feld: "geburtsjahr".to_owned(), wert: json!(1955), signal_2: "ok@geburtsjahr".to_owned(), art: Art::Antwort });
        let ctx = Kontext { tabelle: &tabelle, kreuze };
        let datei = tempfile::NamedTempFile::new().unwrap();
        let mut log = Log(Some(datei.reopen().unwrap()));
        let mut z = Zaehler::default();
        versuche(&d, &b, &ctx, 3, "stammdaten_geburtsdatum", &json!("15.03.1955"), &json!("15.03.1985"), None, &mut z, &mut log).await;
        let text = std::fs::read_to_string(datei.path()).unwrap();
        assert_eq!((z.versuche, z.euro, z.gleich, z.stand_nur), (1, 1, 0, 0), "der eingeschleppte Altwert wurde nicht als Euro-Fund gezaehlt: {text}");
        assert!(text.contains("\tEUR\tK Some(1363200)\tF Some(1392400)\tdelta -29200\t"), "die Fundzeile nennt nicht die 292,00 EUR: {text}");
    });
}

// ---------------------------------------------------------------------------------------------------------------------------
// Messung `fragen-reduktion-2` (Fragen-Reduktion Teil 2, 2026-10-21): wie viele Fragen stellt der Dienst, und rechnet er danach
// dieselben Cent? Aufruf: `TAXGRAPH_OHNE_XSD=1 cargo test -p api --test it korrektur_sweep::messung_fragenzahl_und_steuer -- --ignored
// --nocapture`. Eine Zeile `P0` je Fall: Zahl der beantworteten Fragen (ohne Ankreuzliste), Steuer, Kirchensteuer, `grund`. Zwei Staende
// vergleicht man, indem man den Test auf beiden aufruft und die `P0`-Zeilen gegeneinander liest.
// GRENZE: Der Fragenlauf beantwortet jede Frage, die der Dienst stellt, mit der Tabelle oder dem neutralen Standard. Eine Frage, die der
// Dienst nicht stellt, bleibt unbeantwortet; ob das Ergebnis dann gleich ist, zeigt die Zeile. Die Messung trifft nur diese Faelle.
// ---------------------------------------------------------------------------------------------------------------------------

/// Einzelnutzer wie in `ausgangsfaelle` (Geburtsdatum 1985), mit der gewaehlten Konfession; `ersetze` tauscht Zeilen der Tabelle, `plus`
/// haengt Antworten an.
fn einzel_mit(konfession: &'static str, ersetze: &[(&'static str, Gib)], plus: &[(&'static str, Gib)]) -> Tabelle {
    let mut t = person_a("15.03.1985");
    t.push(("veranlagung", Gib::Wahl("einzel")));
    t.extend(kirche(konfession, None));
    t.retain(|(f, _)| !ersetze.iter().any(|(g, _)| g == f));
    t.extend(ersetze.iter().copied());
    t.extend(plus.iter().copied());
    t
}

/// Fragenlauf, Basismessung, Zeile `P0`. Gibt die Zahl der beantworteten Fragen und die Steuer zurueck. `ohne` nennt Antworten, die ein
/// Nutzer nicht gegeben haette, weil die Frage entfaellt: ihre Ereignisse werden nicht geschrieben.
async fn p0_lauf(name: &'static str, scheibe: &'static str, tabelle: &Tabelle, kreuze: &[&str], ohne: &[&str]) -> (usize, Option<i64>) {
    let d = Dienst::neu();
    let mut b = fragenlauf(&d, name, scheibe, 2025, tabelle, kreuze).await;
    let folge: Vec<String> = b.ereignisse.iter().filter(|e| e.signal_2.starts_with("ok@")).map(|e| e.feld.clone()).collect();
    b.ereignisse.retain(|e| !ohne.contains(&e.feld.as_str()));
    let m = d.messe(&d.baue(&b, None).await).await;
    eprintln!("P0\t{name}\tfragen {}\tSteuer {:?}\tKist {:?}\tgrund {}\tohne {ohne:?}", folge.len(), m.zahl(), m.kist(), m.grund());
    eprintln!("P0FOLGE\t{name}\t{}", folge.join(" "));
    (folge.len(), m.zahl())
}

#[test]
#[ignore = "Messung: mit --ignored --nocapture aufrufen, die Zeilen P0 nennen die Zahlen"]
fn messung_fragenzahl_und_steuer() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        // Die zehn Ausgangsfaelle der Korrektur-Sweeps.
        for (name, scheibe, tabelle, kreuze) in ausgangsfaelle() {
            p0_lauf(name, scheibe, &tabelle, kreuze, &[]).await;
        }
        // F7: Konfession „keine“. Ein Wert fuer die Kirchensteuer des Arbeitgebers und ein anderes Bundesland aendern die Steuer nicht?
        let (_, a0) = p0_lauf("keine_ag0", "gesamt", &einzel_mit("keine", &[], &[]), &[], &[]).await;
        let (_, a1) = p0_lauf("keine_ag1000", "gesamt", &einzel_mit("keine", &[], &[("kirchensteuer_arbeitgeber", Gib::Euro("1000"))]), &[], &[]).await;
        let (_, a2) = p0_lauf("keine_nrw", "gesamt", &einzel_mit("keine", &[("kist_bundesland", Gib::Wahl("nordrhein_westfalen"))], &[]), &[], &[]).await;
        eprintln!("P0VERGLEICH F7 keine: ag0 {a0:?} ag1000 {a1:?} nrw {a2:?} gleich {}", a0 == a1 && a0 == a2);
        // F6 Haushaltshilfen: die Zaehler gehen nicht in die Rechnung ein, solange „hat Aufwendungen“ nein ist.
        let zaehler = [("hh_anzahl_dienstleistungen", Gib::Euro("3")), ("hh_anzahl_handwerker", Gib::Euro("3")), ("hh_anzahl_minijobs", Gib::Euro("3"))];
        let (_, h1) = p0_lauf("keine_hh_zaehler3", "gesamt", &einzel_mit("keine", &[], &zaehler), &[], &[]).await;
        eprintln!("P0VERGLEICH F6 hh: zaehler0 {a0:?} zaehler3 {h1:?} gleich {}", a0 == h1);
        // Der Ja-Weg: wer Haushaltshilfen hatte, bekommt die Zaehler und danach die Einzelfragen; nur die Stelle der Zaehler darf sich aendern.
        let ja = [("hh_hat_aufwendungen", Gib::Wahl("true")), ("hh_anzahl_dienstleistungen", Gib::Euro("1")), ("hh_anzahl_handwerker", Gib::Euro("1")), ("hh_anzahl_minijobs", Gib::Euro("1"))];
        p0_lauf("hh_ja", "gesamt", &einzel_mit("keine", &[], &ja), &[], &[]).await;
        // Das Ehepaar, dessen Ehegatte einer Kirche angehoert (Entscheidung d-43: das Bundesland bleibt eine Frage).
        let mut paar_b = person_a("15.03.1985");
        paar_b.push(("veranlagung", Gib::Wahl("zusammen")));
        paar_b.extend(person_b());
        paar_b.extend(kirche("keine", Some("evangelisch")));
        p0_lauf("ehepaar_b_evangelisch", "gesamt", &paar_b, &[], &[]).await;
        // F6 Arbeitsweg: Wirkung der Wegfelder bei 0 Tagen, und was der Dienst sagt, wenn der Nutzer sie nicht beantwortet.
        let (_, e1) = p0_lauf("ep_km30_tage0", "gesamt", &einzel_mit("keine", &[], &[("ep_entfernung_km", Gib::Euro("30"))]), &[], &[]).await;
        eprintln!("P0VERGLEICH F6 ep: km0 {a0:?} km30(tage0) {e1:?} gleich {}", a0 == e1);
        let weg = ["ep_eigenes_kfz", "ep_entfernung_km", "ep_oepnv_kosten", "ep_ziel_des_weges", "ep_ziel_adresse"];
        p0_lauf("ep_unbeantwortet", "gesamt", &einzel_mit("keine", &[], &[]), &[], &weg).await;
        p0_lauf("ep_nur_km_unbeantwortet", "gesamt", &einzel_mit("keine", &[], &[]), &[], &weg[..3]).await;
        // Aussergewoehnliche Belastungen: die Ja/Nein-Frage nach der Angemessenheit steht hinter dem Betrag 0.
        p0_lauf("agb_angemessen_unbeantwortet", "gesamt", &einzel_mit("keine", &[], &[]), &[], &["agb_notwendig_angemessen"]).await;
    });
}
