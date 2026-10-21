//! Fluss J: weitere Nutzer-Durchlaeufe im Browser und die Beobachtung der Fragen (Julius 2026-10-20: „sind die Fragen in einer
//! sinnvollen Reihenfolge? kommen Fragen, die eigentlich nicht gestellt werden muessten?“).
//!
//! Wie Fluss I tippen und klicken die Tests, was ein Nutzer tippt und klickt; die Schnittstelle dient nur zum LESEN. Neu ist
//! die Beobachtung: mit `TAXGRAPH_FRAGENLOG=<datei>` haelt jeder Durchlauf die Folge der Fragen fest (`fluss_i::fragenlog`),
//! mit `TAXGRAPH_WIRKUNGSLOG=<datei>` laeuft danach die Wirkungsprobe ([`wirkungsprobe`]). Ohne die Variablen geschieht nichts
//! davon; die Tests pruefen dann nur ihre Pruefpunkte (Zahl, Sperre, Befunde) und die Konsole.
//!
//! Die Beobachtung AENDERT NICHTS an der Fragenfolge. Was sie findet, steht im Bericht des Auftrags; eine Aenderung entscheidet
//! Julius.

use std::collections::BTreeSet;
use std::io::Write;

use serde_json::{json, Value};

use crate::dienst::Start;
use crate::fluss_i::{
    aktuell, bis_zur_ersten_frage, durchlauf, ehepaar, ergebnis, euro_text, kirche, person_a, person_b, pos, pruefe_konsole, wie_oft, Gib, PlatzhalterErlaubt,
    Schritt, Tabelle,
};
use crate::seite::Umgebung;

/// Pfade (JSON-Zeiger-aehnlich), an denen sich `a` und `b` unterscheiden.
fn pfade(a: &Value, b: &Value, p: &str, aus: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()).collect::<BTreeSet<_>>() {
                pfade(x.get(k).unwrap_or(&Value::Null), y.get(k).unwrap_or(&Value::Null), &format!("{p}/{k}"), aus);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (u, v)) in x.iter().zip(y).enumerate() {
                pfade(u, v, &format!("{p}[{i}]"), aus);
            }
        }
        _ if a != b => aus.push(p.to_owned()),
        _ => {}
    }
}

/// Eine Korrektur, wie die Oberflaeche sie schreibt (`korrigiereBestaetigt`): das bestehende Ereignis des Feldes wird mit
/// `ersetzt` und `signal_1` genannt, sonst weist der Store sie ab (fail-closed).
fn schreibe_ersetzt(u: &Umgebung, fall: &str, feld: &str, wert: &Value) -> (u16, Value) {
    let alt = u.stand_felder(fall)[feld]["event_id"].clone();
    u.dienst.post(
        &format!("/fall/{fall}/event"),
        &json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": alt, "signal_2": "ok"}, "ersetzt": alt,
        }),
    )
}

/// Was der Dienst zu einem Fall sagt: Grund, Steuer, Kirchensteuer und die Erklaerung (`GET /deklaration`).
struct Messung {
    grund: String,
    zahl: Option<i64>,
    kist: Option<i64>,
    erklaerung: Value,
}

fn messe(u: &Umgebung, fall: &str) -> Messung {
    let (status, e) = u.dienst.get(&format!("/fall/{fall}/ergebnis"));
    assert_eq!(status, 200, "ergebnis: {e}");
    let (dstatus, d) = u.dienst.get(&format!("/fall/{fall}/deklaration"));
    Messung {
        grund: e["grund"].as_str().unwrap_or("").to_owned(),
        zahl: e["zahl_cent"].as_i64(),
        kist: e["kist_cent"].as_i64(),
        erklaerung: if dstatus == 200 { d } else { json!({"status": dstatus}) },
    }
}

/// Die Wirkungsprobe (U2 des Auftrags): fuer jedes Feld, das der Lauf gefragt hat, setzt sie bis zu vier andere Werte, liest
/// Steuer, Kirchensteuer und Erklaerung und stellt den Ausgangswert wieder her. Schreibt eine Zeile je Versuch in die Datei,
/// die `TAXGRAPH_WIRKUNGSLOG` nennt; ohne die Variable geschieht nichts.
///
/// Spalten: Lauf, Feld, Typ, Ausgangswert, Versuch, Status, Grund, Steuer-Aenderung in Cent, Kirchensteuer-Aenderung in Cent,
/// Zahl der geaenderten Kennzeichen des Formulars (`/deklaration/…`, `/person_b/…`), die ersten vier davon.
///
/// GRENZE: ein Versuch trifft nur die Zweige, die seine Werte erreichen. „Keine Wirkung gesehen“ heisst NICHT „wirkungslos“.
/// Text und Datum variiert die Probe nicht (das Muster der Antwort kann sie abweisen); `veranlagung` auch nicht (es baut den
/// ganzen Fall um). Die Probe liest dieselbe Schnittstelle wie die Oberflaeche und aendert den Fall nur voruebergehend.
pub(crate) fn wirkungsprobe(u: &Umgebung, lauf: &str, verlauf: &[Schritt]) {
    let Ok(pfad) = std::env::var("TAXGRAPH_WIRKUNGSLOG") else { return };
    let mut log = std::fs::OpenOptions::new().create(true).append(true).open(pfad).expect("TAXGRAPH_WIRKUNGSLOG nicht beschreibbar");
    let fall = u.fall();
    let felder = u.stand_felder(&fall);
    let basis = messe(u, &fall);
    // Stellen der Erklaerung, die sich ohne jede Aenderung unterscheiden (Zeitstempel o. ae.), zaehlen nicht.
    let mut fluechtig = Vec::new();
    pfade(&basis.erklaerung, &messe(u, &fall).erklaerung, "", &mut fluechtig);
    for s in verlauf {
        let ausgang = felder.get(&s.feld).map_or(Value::Null, |f| f["wert"].clone());
        let versuche: Vec<Value> = if ausgang.is_null() || s.feld == "veranlagung" {
            Vec::new()
        } else {
            match s.typ.as_str() {
                "bool" => vec![json!(!ausgang.as_bool().unwrap_or(false))],
                "cent" => [0_i64, 100_000, 5_000_000].iter().filter(|c| ausgang != json!(**c)).map(|c| json!(c)).collect(),
                "int" => [0_i64, 1, 10, 200].iter().filter(|c| ausgang != json!(**c)).map(|c| json!(c)).collect(),
                "enum" => s.optionen.split(',').filter(|o| ausgang != json!(o)).map(|o| json!(o)).collect(),
                _ => Vec::new(),
            }
        };
        if versuche.is_empty() {
            writeln!(log, "{lauf}\t{}\t{}\t{ausgang}\t-\t-\t-\t-\t-\t-\t(nicht variiert: Text, Datum, Gate oder ohne Ausgangswert)", s.feld, s.typ).unwrap();
            continue;
        }
        for v in &versuche {
            let (status, antwort) = schreibe_ersetzt(u, &fall, &s.feld, v);
            if status != 200 && status != 201 {
                let grund = antwort["detail"].as_str().or_else(|| antwort["fehler"].as_str()).unwrap_or("").replace(['\t', '\n'], " ");
                writeln!(log, "{lauf}\t{}\t{}\t{ausgang}\t{v}\t{status}\tabgewiesen: {grund}\t-\t-\t-\t", s.feld, s.typ).unwrap();
                continue;
            }
            let m = messe(u, &fall);
            let mut stellen = Vec::new();
            pfade(&basis.erklaerung, &m.erklaerung, "", &mut stellen);
            stellen.retain(|p| !fluechtig.contains(p));
            let delta = |a: Option<i64>, b: Option<i64>| match (a, b) {
                (Some(x), Some(y)) => (y - x).to_string(),
                (None, None) => "0".to_owned(),
                _ => "Zahl<->Sperre".to_owned(),
            };
            // Die Kennzeichen der Erklaerung (`/deklaration/…`, `/person_b/…`) sind die Wirkung auf das Formular; `/basis_snapshot`
            // und die Merker `/eingaben_konsistent`, `/unvollstaendig`, `/vollstaendig` aendern sich bei jeder Eingabe.
            let formular: Vec<&String> = stellen.iter().filter(|p| p.starts_with("/deklaration/") || p.starts_with("/person_b/")).collect();
            writeln!(
                log,
                "{lauf}\t{}\t{}\t{ausgang}\t{v}\t{status}\t{}\t{}\t{}\t{}\t{}",
                s.feld,
                s.typ,
                m.grund,
                delta(basis.zahl, m.zahl),
                delta(basis.kist, m.kist),
                formular.len(),
                formular.iter().take(4).map(|p| p.as_str()).collect::<Vec<_>>().join(" ")
            )
            .unwrap();
        }
        let (status, antwort) = schreibe_ersetzt(u, &fall, &s.feld, &ausgang);
        assert!(status == 200 || status == 201, "die Probe konnte {} nicht auf {ausgang} zurueckstellen: {status} {antwort}", s.feld);
    }
    let ende = messe(u, &fall);
    assert!(
        ende.zahl == basis.zahl && ende.kist == basis.kist && ende.grund == basis.grund,
        "nach der Wirkungsprobe ist der Fall nicht mehr der Ausgangsfall: Steuer {:?} -> {:?}, Kirchensteuer {:?} -> {:?}, Grund {} -> {}",
        basis.zahl,
        ende.zahl,
        basis.kist,
        ende.kist,
        basis.grund,
        ende.grund
    );
}

/// Antworten fuer einen Einzelnutzer mit Lohn 60.000 EUR (wie `fluss_i`, ohne Kirche, wenn `konf` „keine“ ist).
fn einzel_tabelle(konf: &'static str) -> Tabelle {
    let mut t = person_a();
    t.push(("veranlagung", Gib::Wahl("einzel")));
    t.extend(kirche(konf, None, "bayern"));
    t
}

/// Eine gesetzliche Rente: Art, Beginn und Jahresbetrag (Euro, wie getippt). `partner` haengt `_partner` an die Feld-IDs.
fn rente(partner: bool, beginn: &'static str, euro: &'static str) -> Tabelle {
    if partner {
        vec![("rentner_renten_art_partner", Gib::Wahl("gesetzliche_rente")), ("rentner_renten_beginn_jahr_partner", Gib::Euro(beginn)), ("rentner_jahresrente_partner", Gib::Euro(euro))]
    } else {
        vec![("rentner_renten_art", Gib::Wahl("gesetzliche_rente")), ("rentner_renten_beginn_jahr", Gib::Euro(beginn)), ("rentner_jahresrente", Gib::Euro(euro))]
    }
}

/// Das Ergebnis, ohne Handrechnung: der Dienst meldet eine Zahl (kein Sperrgrund), und die Oberflaeche zeigt genau sie.
/// Liefert die Antwort des Dienstes. GRENZE: die Hoehe der Zahl prueft diese Funktion nicht.
fn pruefe_zahl_gezeigt(u: &Umgebung, kennung: &str) -> Value {
    let (gezeigt, _rechenweg, api) = ergebnis(u);
    pp!("J.der_lauf_endet_mit_einer_zahl_und_die_oberflaeche_zeigt_sie", api["grund"] == json!("bestaetigt"), "{kennung}: der Dienst sperrt: {api}");
    pp!(
        "J.der_lauf_endet_mit_einer_zahl_und_die_oberflaeche_zeigt_sie",
        api["zahl_cent"].as_i64().is_some_and(|z| gezeigt == euro_text(z)),
        "{kennung}: die Oberflaeche zeigt {gezeigt:?}, der Dienst meldet {}",
        api["zahl_cent"]
    );
    api
}

/// Wie viele Fragen des Verlaufs beginnen mit einem der Wortanfaenge (fuer die Kennzahlen im Bericht).
fn zaehle(verlauf: &[Schritt], anfaenge: &[&str]) -> usize {
    verlauf.iter().filter(|s| anfaenge.iter().any(|a| s.feld.starts_with(a))).count()
}

/// Rentner, ein Nutzer, gesetzliche Rente (Kreuz „sonstige Einkuenfte, z. B. eine Rente“). Die Folge der Fragen wird beobachtet
/// (`TAXGRAPH_FRAGENLOG`); die Fragen, die die Tabelle nicht kennt, beantwortet ein Platzhalter (`PlatzhalterErlaubt`), die
/// Hoehe der Zahl ist darum nicht belegt.
#[test]
fn fluss_j_rentner_einzel_mit_gesetzlicher_rente_erreicht_die_steuerzahl() {
    let _sperre = crate::browser::seriell();
    let _p = PlatzhalterErlaubt::neu();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    bis_zur_ersten_frage(&u, "rentner_gesamt", &["kein_sonstige"]);
    let mut t = einzel_tabelle("keine");
    t.extend(rente(false, "2015", "18000"));
    let verlauf = durchlauf(&u, "rentner_einzel_rente", &t, &[]);
    eprintln!("KENNZAHL rentner_einzel: Fragen {}, davon kind_ {}, dba_ {}, p33a_ {}", verlauf.len(), zaehle(&verlauf, &["kind_", "fam_"]), zaehle(&verlauf, &["dba_"]), zaehle(&verlauf, &["p33a_"]));
    pruefe_zahl_gezeigt(&u, "Rentner, ein Nutzer, gesetzliche Rente");
    wirkungsprobe(&u, "rentner_einzel_rente", &verlauf);
    pruefe_konsole(&u, "Rentner einzel");
}

/// Rentner-Ehepaar, beide mit gesetzlicher Rente. Die Ankreuzliste der Rentner hat keine Zeile zum Partner; was der Lauf
/// deshalb zusaetzlich fragt, steht im Bericht (`TAXGRAPH_FRAGENLOG`).
#[test]
fn fluss_j_rentner_ehepaar_mit_zwei_gesetzlichen_renten_erreicht_die_steuerzahl() {
    let _sperre = crate::browser::seriell();
    let _p = PlatzhalterErlaubt::neu();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    bis_zur_ersten_frage(&u, "rentner_gesamt", &["kein_sonstige"]);
    let mut t = person_a();
    t.push(("veranlagung", Gib::Wahl("zusammen")));
    t.extend(kirche("keine", Some("keine"), "bayern"));
    t.extend(rente(false, "2015", "18000"));
    t.extend(rente(true, "2018", "12000"));
    let verlauf = durchlauf(&u, "rentner_ehepaar_rente", &t, &[]);
    eprintln!("KENNZAHL rentner_ehepaar: Fragen {}, davon Partner {}, kind_ {}", verlauf.len(), verlauf.iter().filter(|s| s.feld.ends_with("_partner")).count(), zaehle(&verlauf, &["kind_", "fam_"]));
    pruefe_zahl_gezeigt(&u, "Rentner-Ehepaar, zwei gesetzliche Renten");
    wirkungsprobe(&u, "rentner_ehepaar_rente", &verlauf);
    pruefe_konsole(&u, "Rentner Ehepaar");
}

/// Alleinverdiener: Zusammenveranlagung, der Partner hat keinen Lohn (0 EUR). Der Lauf fragt den Partner weiter nach Steuerklasse,
/// Lohnsteuer und Vorsorge; die Kirchensteuer ist 0, weil niemand Mitglied ist. GRENZE: die Hoehe der Steuer ist nicht von Hand
/// gerechnet.
#[test]
fn fluss_j_alleinverdiener_ehepaar_ohne_lohn_des_partners_erreicht_die_steuerzahl() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    let mut t = ehepaar("keine", "keine", "bayern");
    t.retain(|(f, _)| *f != "bruttoarbeitslohn_partner");
    let verlauf = durchlauf(&u, "alleinverdiener", &t, &[]);
    let api = pruefe_zahl_gezeigt(&u, "Alleinverdiener");
    pp!("J.der_alleinverdiener_zahlt_keine_kirchensteuer", api["kist_cent"] == json!(0), "die Kirchensteuer ist {}, niemand ist Mitglied", api["kist_cent"]);
    eprintln!("KENNZAHL alleinverdiener: Fragen {}, davon Partner {}, Zahl {}", verlauf.len(), verlauf.iter().filter(|s| s.feld.ends_with("_partner")).count(), api["zahl_cent"]);
    wirkungsprobe(&u, "alleinverdiener", &verlauf);
    pruefe_konsole(&u, "Alleinverdiener");
}

/// Ein Einzelnutzer ohne Konfession wird trotzdem nach dem Bundesland gefragt: die Frage kommt vor der Konfession (Befund B,
/// Fluss I), und die Antwort „keine“ nimmt sie nicht zurueck. Die Zahl ist die von Fluss I (Handrechnung dort), die Kirchensteuer 0.
#[test]
fn fluss_j_einzelnutzer_ohne_konfession_wird_trotzdem_nach_dem_bundesland_gefragt() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    let verlauf = durchlauf(&u, "einzel_keine_konfession", &einzel_tabelle("keine"), &[]);
    let (land, konf) = (pos(&verlauf, "kist_bundesland"), pos(&verlauf, "kist_konfession"));
    pp!(
        "J.ohne_konfession_wird_das_bundesland_trotzdem_vor_der_konfession_gefragt",
        wie_oft(&verlauf, "kist_bundesland") == 1 && land.is_some() && konf.is_some() && land < konf,
        "Bundesland {}-mal, Position {land:?}, Konfession {konf:?}",
        wie_oft(&verlauf, "kist_bundesland")
    );
    let api = pruefe_zahl_gezeigt(&u, "Einzelnutzer ohne Konfession");
    pp!("J.ohne_konfession_wird_das_bundesland_trotzdem_vor_der_konfession_gefragt", api["zahl_cent"] == json!(1_392_400) && api["kist_cent"] == json!(0), "Steuer {} (Handrechnung Fluss I: 1392400), Kirchensteuer {}", api["zahl_cent"], api["kist_cent"]);
    eprintln!("KENNZAHL einzel_keine_konfession: Fragen {}", verlauf.len());
    wirkungsprobe(&u, "einzel_keine_konfession", &verlauf);
    pruefe_konsole(&u, "Einzelnutzer ohne Konfession");
}

/// Versucht eine Antwort auf die aktuelle Frage `feld`. Liefert `(Oberflaeche geht weiter, Text des Banners)`.
fn versuche_antwort(u: &Umgebung, feld: &str, text: &str) -> (bool, String) {
    assert_eq!(aktuell(u)["feld_id"], json!(feld), "die aktuelle Frage ist nicht {feld}");
    u.s.fuelle("#feld-input", text);
    u.s.klick("#bestaetigen");
    let weiter = u.s.try_warte_bis(&format!("AKTUELL && AKTUELL.feld_id !== '{feld}'"), 5).is_some();
    (weiter, u.s.textinhalt("#netz-banner"))
}

/// Fluss J F4 (behoben 2026-10-21, d-51). Vorher kam eine leere Antwort auf den Hausnummernzusatz und auf den BIC nicht durch: die
/// Fragen hatten keinen Ausweg „keine“, und die meisten Nutzer haben weder einen Zusatz noch eine ausländische Bank (Beobachtung
/// U4). Jetzt geht die leere Antwort auf den Zusatz durch („kein Zusatz“), der BIC wird nur nach einer IBAN ohne DE gefragt und
/// darf dort leer bleiben; ein Strich statt eines BIC kommt weiter nicht durch.
#[test]
fn fluss_j_kein_hausnummernzusatz_und_kein_bic_kommen_durch() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    // Ein deutsches Konto: der Zusatz bleibt leer, nach dem BIC fragt die Oberflaeche nicht.
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    let t = einzel_tabelle("keine");
    durchlauf(&u, "einzel_ohne_zusatz", &t, &["stammdaten_hausnummerzusatz"]);
    let (weiter, banner) = versuche_antwort(&u, "stammdaten_hausnummerzusatz", "");
    eprintln!("MESSUNG Zusatz leer: weiter {weiter}, Banner {banner:?}");
    pp!("J.ein_leerer_hausnummernzusatz_geht_durch", weiter, "die leere Antwort auf den Zusatz kam nicht durch; Banner {banner:?}");
    let stand = u.stand_felder(&u.fall());
    let zusatz = &stand["stammdaten_hausnummerzusatz"];
    pp!(
        "J.ein_leerer_hausnummernzusatz_geht_durch",
        zusatz["wert"] == json!("") && zusatz["zustand"] == json!("bestaetigt"),
        "gespeichert ist {}, Zustand {}",
        zusatz["wert"],
        zusatz["zustand"]
    );
    let rest = durchlauf(&u, "einzel_ohne_zusatz", &t, &[]);
    pp!(
        "J.ein_deutsches_konto_fragt_nicht_nach_dem_bic",
        wie_oft(&rest, "stammdaten_iban") == 1 && wie_oft(&rest, "stammdaten_bic") == 0,
        "IBAN {}-mal, BIC {}-mal gefragt",
        wie_oft(&rest, "stammdaten_iban"),
        wie_oft(&rest, "stammdaten_bic")
    );
    let api = pruefe_zahl_gezeigt(&u, "Einzelnutzer ohne Zusatz, deutsches Konto");
    pp!("J.kein_zusatz_und_kein_bic_aendern_die_steuer_nicht", api["zahl_cent"] == json!(1_392_400), "Steuer {} (Fluss I: 1392400)", api["zahl_cent"]);
    // Ein auslaendisches Konto: der BIC kommt nach der IBAN; leer geht durch, ein Strich nicht.
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    let mut t = einzel_tabelle("keine");
    t.retain(|(f, _)| *f != "stammdaten_iban");
    t.push(("stammdaten_iban", Gib::Text("GB82WEST12345698765432")));
    let davor = durchlauf(&u, "einzel_auslands_iban", &t, &["stammdaten_bic"]);
    pp!(
        "J.ein_auslaendisches_konto_fragt_den_bic_nach_der_iban",
        pos(&davor, "stammdaten_iban").is_some() && aktuell(&u)["feld_id"] == json!("stammdaten_bic"),
        "IBAN an Stelle {:?}, aktuelle Frage {}",
        pos(&davor, "stammdaten_iban"),
        aktuell(&u)["feld_id"]
    );
    let (weiter, banner) = versuche_antwort(&u, "stammdaten_bic", "-");
    eprintln!("MESSUNG BIC Strich: weiter {weiter}, Banner {banner:?}");
    pp!("J.ein_strich_statt_eines_bic_kommt_nicht_durch", !weiter, "ein Strich statt des BIC ging durch; Banner {banner:?}");
    let (weiter, banner) = versuche_antwort(&u, "stammdaten_bic", "");
    eprintln!("MESSUNG BIC leer: weiter {weiter}, Banner {banner:?}");
    pp!("J.ein_leerer_bic_geht_durch", weiter, "die leere Antwort auf den BIC kam nicht durch; Banner {banner:?}");
    durchlauf(&u, "einzel_auslands_iban", &t, &[]);
    pruefe_zahl_gezeigt(&u, "Einzelnutzer, ausländisches Konto ohne BIC");
    pruefe_konsole(&u, "kein Zusatz, kein BIC");
}

/// Die Uebernahme aus dem Vorjahr: ein zweiter Fall bekommt die Antworten des ersten ueber „Vorjahr-Daten uebernehmen“. Beobachtet,
/// welche Fragen danach noch kommen und welche schon beantwortet sind (U3 des Auftrags: fragt die Oberflaeche doppelt?).
#[test]
fn fluss_j_vorjahr_uebernahme_beantwortet_fragen_vor() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let mut t = person_a();
    t.push(("veranlagung", Gib::Wahl("einzel")));
    t.extend(kirche("evangelisch", None, "nordrhein_westfalen"));
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    let quelle = durchlauf(&u, "vorjahr_quelle", &t, &[]);
    let fall1 = u.fall();
    let quelle_api = pruefe_zahl_gezeigt(&u, "Quelle des Vorjahres");

    bis_zur_ersten_frage(&u, "gesamt", &[]);
    u.s.klick("#vorjahr-toggle");
    u.s.fuelle("#vorjahr-fid", &fall1);
    u.s.klick("#vorjahr-go");
    u.s.warte_bis("document.getElementById('vorjahr-status').textContent.includes('übernommen')", 15, "die Uebernahme meldet ihr Ergebnis");
    let status = u.s.text("#vorjahr-status");
    eprintln!("MESSUNG Vorjahr: {status}");
    let felder = u.stand_felder(&u.fall());
    for f in ["veranlagung", "stammdaten_nachname", "bruttoarbeitslohn", "kist_konfession", "kist_bundesland", "steuerklasse"] {
        eprintln!("MESSUNG Vorjahr-Stand {f}: wert {} zustand {} herkunft {}", felder[f]["wert"], felder[f]["zustand"], felder[f]["herkunft_badge"]);
    }
    // Die Tabelle des zweiten Falls kennt auch die Fragen zum Ehegatten, falls die Oberflaeche sie stellt (Beobachtung).
    let mut t2 = t.clone();
    t2.extend(person_b().into_iter().filter(|(f, _)| *f != "veranlagung" && *f != "bruttoarbeitslohn_partner"));
    let _p = PlatzhalterErlaubt::neu();
    let ziel = durchlauf(&u, "vorjahr_ziel", &t2, &[]);
    eprintln!("KENNZAHL vorjahr: Fragen im ersten Fall {}, im zweiten nach der Uebernahme {}", quelle.len(), ziel.len());
    // Fluss J F1 (behoben 2026-10-21, d-51): vorher legte die Uebernahme `veranlagung` als Vorschlag ab und nahm die Frage aus der
    // Warteschlange; die Fragen zum Ehegatten verlangen aber eine BESTAETIGTE Antwort, blieben also offen (64 statt 51 Fragen, 26
    // zum Ehegatten). Jetzt bleiben ein Gate (`veranlagung`) und der Ausloeser einer Ableitung (`stammdaten_geburtsdatum`) eine
    // Frage, vorbelegt; alles andere gilt ohne Rueckfrage.
    pp!(
        "J.nach_der_vorjahr_uebernahme_fragt_die_oberflaeche_den_einzelnutzer_nicht_nach_dem_ehegatten",
        wie_oft(&ziel, "veranlagung") == 1 && wie_oft(&ziel, "stammdaten_vorname_partner") == 0 && ziel.len() < quelle.len(),
        "Frage veranlagung {}-mal, Frage nach dem Vornamen des Ehegatten {}-mal, Fragen {} gegen {} im Vorjahresfall",
        wie_oft(&ziel, "veranlagung"),
        wie_oft(&ziel, "stammdaten_vorname_partner"),
        ziel.len(),
        quelle.len()
    );
    // Eine Frage bleiben: das Gate (`veranlagung`), der Ausloeser der Ableitung (`stammdaten_geburtsdatum`) und die Felder des
    // Pflicht-Kegels, die der Vorjahresfall mitbrachte (`versicherungsart`, `mit_anspruch_auf_zuschuss`, `ep_eigenes_kfz`,
    // `ep_entfernung_km`): ohne ihre bestaetigte Antwort kommt keine Zahl (`input_kegel_nicht_bestaetigt`). Ohne Rueckfrage
    // gelten Name, Anschrift und Steuerklasse.
    pp!(
        "J.nach_der_vorjahr_uebernahme_bleiben_gate_ausloeser_und_kegel_eine_frage_alles_andere_gilt_ohne_rueckfrage",
        ["stammdaten_geburtsdatum", "versicherungsart", "mit_anspruch_auf_zuschuss", "ep_eigenes_kfz", "ep_entfernung_km"].iter().all(|f| wie_oft(&ziel, f) == 1)
            && ["stammdaten_nachname", "stammdaten_vorname", "stammdaten_plz", "stammdaten_steuernummer", "steuerklasse"].iter().all(|f| wie_oft(&ziel, f) == 0),
        "Geburtsdatum {}-mal gefragt, Versicherungsart {}-mal, Entfernung {}-mal, Nachname {}-mal, Steuerklasse {}-mal",
        wie_oft(&ziel, "stammdaten_geburtsdatum"),
        wie_oft(&ziel, "versicherungsart"),
        wie_oft(&ziel, "ep_entfernung_km"),
        wie_oft(&ziel, "stammdaten_nachname"),
        wie_oft(&ziel, "steuerklasse")
    );
    // Vorher (2026-10-20): alle 64 Fragen waren beantwortet und keine Zahl stand da. Das Geburtsjahr aus dem Vorjahr blieb
    // „vorlaeufig“, auch nach dem bestaetigten Geburtsdatum, und die Sperre nannte als offen nur `geburtsjahr`.
    let (_, _, api) = ergebnis(&u);
    pp!(
        "J.nach_der_vorjahr_uebernahme_endet_der_fragebogen_mit_einer_zahl",
        api["grund"] == json!("bestaetigt") && api["zahl_cent"].is_i64(),
        "Grund {}, offen {}, Zahl {}",
        api["grund"],
        api["offen"],
        api["zahl_cent"]
    );
    // Dieselben Antworten, dieselbe Steuer: die Uebernahme aendert keine Zahl (Euro-Wirkung 0).
    pp!(
        "J.die_zahl_nach_der_vorjahr_uebernahme_ist_die_zahl_des_vorjahresfalls",
        api["zahl_cent"] == quelle_api["zahl_cent"] && api["kist_cent"] == quelle_api["kist_cent"],
        "Steuer {} gegen {}, Kirchensteuer {} gegen {}",
        api["zahl_cent"],
        quelle_api["zahl_cent"],
        api["kist_cent"],
        quelle_api["kist_cent"]
    );
    let felder = u.stand_felder(&u.fall());
    pp!(
        "J.das_bestaetigte_geburtsdatum_macht_das_geburtsjahr_aus_dem_vorjahr_zur_berechneten_antwort",
        felder["geburtsjahr"]["zustand"] == json!("bestaetigt") && felder["geburtsjahr"]["herkunft_badge"] == json!("berechnet"),
        "geburtsjahr {} / {} / {}",
        felder["geburtsjahr"]["wert"],
        felder["geburtsjahr"]["zustand"],
        felder["geburtsjahr"]["herkunft_badge"]
    );
    let vorlaeufig: Vec<String> = felder.as_object().unwrap().iter().filter(|(_, f)| f["zustand"] == json!("vorlaeufig")).map(|(k, _)| k.clone()).collect();
    eprintln!("MESSUNG noch vorlaeufig: {} Felder: {vorlaeufig:?}", vorlaeufig.len());
    pruefe_konsole(&u, "Vorjahr");
}

/// Fluss J F1 (2026-10-21), nicht geprueft im Audit vom 2026-10-20: Ein Ehepaar als Quelle des Vorjahres. Beide Geburtsdaten
/// bleiben eine Frage (Ausloeser der Ableitungen `geburtsjahr` und `geburtsjahr_partner`), `veranlagung` auch; Name, Anschrift und
/// Steuerklasse beider gelten ohne Rueckfrage. Dieselben Antworten ergeben dieselbe Steuer wie im Vorjahresfall.
#[test]
fn fluss_j_vorjahr_uebernahme_mit_einem_ehepaar_als_quelle() {
    let _sperre = crate::browser::seriell();
    let _p = PlatzhalterErlaubt::neu();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let t = ehepaar("keine", "keine", "bayern");
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    let quelle = durchlauf(&u, "vorjahr_ehepaar_quelle", &t, &[]);
    let fall1 = u.fall();
    let quelle_api = pruefe_zahl_gezeigt(&u, "Ehepaar als Quelle des Vorjahres");

    bis_zur_ersten_frage(&u, "gesamt", &[]);
    u.s.klick("#vorjahr-toggle");
    u.s.fuelle("#vorjahr-fid", &fall1);
    u.s.klick("#vorjahr-go");
    u.s.warte_bis("document.getElementById('vorjahr-status').textContent.includes('übernommen')", 15, "die Uebernahme meldet ihr Ergebnis");
    eprintln!("MESSUNG Vorjahr Ehepaar: {}", u.s.text("#vorjahr-status"));
    let ziel = durchlauf(&u, "vorjahr_ehepaar_ziel", &t, &[]);
    eprintln!("KENNZAHL vorjahr_ehepaar: Fragen im ersten Fall {}, im zweiten nach der Uebernahme {}", quelle.len(), ziel.len());
    let api = pruefe_zahl_gezeigt(&u, "Ehepaar nach der Vorjahr-Uebernahme");
    pp!(
        "J.nach_der_vorjahr_uebernahme_eines_ehepaars_fragt_die_oberflaeche_gate_und_beide_geburtsdaten",
        wie_oft(&ziel, "veranlagung") == 1 && wie_oft(&ziel, "stammdaten_geburtsdatum") == 1 && wie_oft(&ziel, "stammdaten_geburtsdatum_partner") == 1 && ziel.len() < quelle.len(),
        "veranlagung {}-mal, Geburtsdatum {}-mal, Geburtsdatum des Partners {}-mal, Fragen {} gegen {}",
        wie_oft(&ziel, "veranlagung"),
        wie_oft(&ziel, "stammdaten_geburtsdatum"),
        wie_oft(&ziel, "stammdaten_geburtsdatum_partner"),
        ziel.len(),
        quelle.len()
    );
    pp!(
        "J.die_zahl_nach_der_vorjahr_uebernahme_eines_ehepaars_ist_die_zahl_des_vorjahresfalls",
        api["zahl_cent"] == quelle_api["zahl_cent"] && api["kist_cent"] == quelle_api["kist_cent"],
        "Steuer {} gegen {}, Kirchensteuer {} gegen {}",
        api["zahl_cent"],
        quelle_api["zahl_cent"],
        api["kist_cent"],
        quelle_api["kist_cent"]
    );
    let felder = u.stand_felder(&u.fall());
    pp!(
        "J.beide_geburtsjahre_sind_nach_der_vorjahr_uebernahme_berechnet_und_bestaetigt",
        ["geburtsjahr", "geburtsjahr_partner"].iter().all(|f| felder[*f]["zustand"] == json!("bestaetigt") && felder[*f]["herkunft_badge"] == json!("berechnet")),
        "geburtsjahr {} / {}, geburtsjahr_partner {} / {}",
        felder["geburtsjahr"]["zustand"],
        felder["geburtsjahr"]["herkunft_badge"],
        felder["geburtsjahr_partner"]["zustand"],
        felder["geburtsjahr_partner"]["herkunft_badge"]
    );
    pruefe_konsole(&u, "Vorjahr Ehepaar");
}

/// Fluss J F1, Nachtrag (2026-10-21), Euro-Wirkung: Wer sein Geburtsdatum nach der Eingabe korrigiert, bekommt den
/// Altersentlastungsbetrag (§ 24a EStG) des neuen Datums. Vorher blieb das Geburtsjahr des ersten Datums bestaetigt in Kraft (die
/// Ableitung liess jedes Ziel mit aktivem Event aus), und der Dienst rechnete einen Fall, der 1985 geboren ist, mit dem
/// Altersentlastungsbetrag eines 1955 Geborenen. Gemessen am Dienst: Lohn 60.000 EUR, ein Nutzer, einmal 1955 (Kohorte 2020: 16,0 %,
/// Hoechstbetrag 760 EUR), korrigiert auf 1985 (kein Altersentlastungsbetrag), und ein frischer Fall mit 1985 als Gegenprobe.
#[test]
fn fluss_j_ein_korrigiertes_geburtsdatum_gibt_die_steuer_eines_frischen_falls() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let mit_datum = |datum: &'static str| -> Tabelle { einzel_tabelle("keine").into_iter().map(|(f, g)| if f == "stammdaten_geburtsdatum" { (f, Gib::Text(datum)) } else { (f, g) }).collect() };
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    durchlauf(&u, "geboren_1955", &mit_datum("15.03.1955"), &[]);
    let fall = u.fall();
    let alt = messe(&u, &fall);
    let (status, antwort) = schreibe_ersetzt(&u, &fall, "stammdaten_geburtsdatum", &json!("15.03.1985"));
    assert!(status == 200 || status == 201, "die Korrektur des Geburtsdatums ging nicht durch: {status} {antwort}");
    let korrigiert = messe(&u, &fall);
    let felder = u.stand_felder(&fall);
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    durchlauf(&u, "geboren_1985", &mit_datum("15.03.1985"), &[]);
    let frisch = messe(&u, &u.fall());
    eprintln!(
        "MESSUNG Euro § 24a: Steuer {:?} Cent bei Geburtsdatum 1955, {:?} Cent nach der Korrektur auf 1985, {:?} Cent im frischen Fall 1985; geburtsjahr nach der Korrektur {} / {}",
        alt.zahl, korrigiert.zahl, frisch.zahl, felder["geburtsjahr"]["wert"], felder["geburtsjahr"]["zustand"]
    );
    pp!(
        "J.das_korrigierte_geburtsdatum_erneuert_das_geburtsjahr",
        felder["geburtsjahr"]["wert"] == json!(1985) && felder["geburtsjahr"]["zustand"] == json!("bestaetigt") && felder["geburtsjahr"]["herkunft_badge"] == json!("berechnet"),
        "geburtsjahr {} / {} / {}",
        felder["geburtsjahr"]["wert"],
        felder["geburtsjahr"]["zustand"],
        felder["geburtsjahr"]["herkunft_badge"]
    );
    // Der Altersentlastungsbetrag mindert die Einkuenfte um bis zu 760 EUR: die Steuer des 1955 Geborenen ist niedriger als die des
    // 1985 Geborenen, und die Korrektur gibt genau die Steuer des frischen Falls (nicht die alte).
    pp!(
        "J.das_korrigierte_geburtsdatum_gibt_die_steuer_eines_frischen_falls",
        alt.zahl.is_some() && alt.zahl < frisch.zahl && korrigiert.zahl == frisch.zahl && korrigiert.kist == frisch.kist,
        "Steuer {:?} (1955), {:?} (korrigiert), {:?} (frisch 1985); Kirchensteuer {:?} gegen {:?}",
        alt.zahl,
        korrigiert.zahl,
        frisch.zahl,
        korrigiert.kist,
        frisch.kist
    );
    pruefe_konsole(&u, "Geburtsdatum korrigiert");
}

/// Fluss J F1 (2026-10-21): Die Sperre `ring_betrag_vorlaeufig` sagte nur „Ein Betrag, den du genannt hast, ist noch nicht
/// bestaetigt“, ohne das Feld zu nennen; der Dienst liefert es in `offen`. Die Oberflaeche macht jetzt aus jedem offenen Feld eine
/// Schaltflaeche mit seiner Frage (`zeigeOffeneFelder`). Der Test ruft die Funktion mit der Antwort einer solchen Sperre; der Dienst
/// kann sie in der Oberflaeche nicht mehr erzeugen, weil die Vorjahres-Uebernahme das Geburtsjahr jetzt ableitet.
#[test]
fn fluss_j_die_sperre_ring_betrag_vorlaeufig_nennt_das_offene_feld() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    let t = einzel_tabelle("keine");
    durchlauf(&u, "sperre_nennt_feld", &t, &[]);
    pruefe_zahl_gezeigt(&u, "Einzelnutzer");
    let baue = |r: Value| -> Value {
        u.s.skript_async(
            "const fertig = arguments[arguments.length - 1]; const el = document.createElement('div');
             zeigeOffeneFelder(el, arguments[0]).then(() => fertig(
               Array.from(el.querySelectorAll('[data-sperr-feld]')).map(b => [b.dataset.sperrFeld, b.textContent])));",
            &[r],
        )
    };
    let wege = baue(json!({"grund": "ring_betrag_vorlaeufig", "offen": ["geburtsjahr"], "sperr_felder": []}));
    eprintln!("MESSUNG Schaltflaechen der Sperre: {wege}");
    pp!(
        "J.die_sperre_nennt_das_offene_feld_mit_seiner_frage",
        wege == json!([["geburtsjahr", "In welchem Jahr bist du geboren?"]]),
        "Schaltflaechen: {wege}"
    );
    // Die Konsole vorher pruefen: die Kontrolle unten fragt ein Feld ab, das es nicht gibt (404 im Konsolenprotokoll).
    pruefe_konsole(&u, "Sperre nennt das Feld");
    // KONTROLLEN: ein Feld ohne Frage bekommt keine erfundene Schaltflaeche; ein anderer Grund und die Auslöser des Servers
    // (`sperr_felder`) bleiben bei `zeigeSperrFelder`.
    let ohne_frage = baue(json!({"grund": "ring_betrag_vorlaeufig", "offen": ["gibt_es_nicht"], "sperr_felder": []}));
    let anderer_grund = baue(json!({"grund": "input_kegel_nicht_bestaetigt", "offen": ["geburtsjahr"], "sperr_felder": []}));
    let mit_ausloesern = baue(json!({"grund": "ring_betrag_vorlaeufig", "offen": ["geburtsjahr"], "sperr_felder": [{"feld_id": "x", "grund": "y"}]}));
    pp!(
        "J.die_sperre_erfindet_keine_schaltflaeche",
        ohne_frage == json!([]) && anderer_grund == json!([]) && mit_ausloesern == json!([]),
        "ohne Frage {ohne_frage}, anderer Grund {anderer_grund}, mit Ausloesern {mit_ausloesern}"
    );
}
