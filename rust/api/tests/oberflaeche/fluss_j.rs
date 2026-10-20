//! Fluss J: weitere Nutzer-Durchlaeufe im Browser und die Beobachtung der Fragen (Julius 2026-10-20: „sind die Fragen in einer
//! sinnvollen Reihenfolge? kommen Fragen, die eigentlich nicht gestellt werden muessten?“).
//!
//! Wie Fluss I tippen und klicken die Tests, was ein Nutzer tippt und klickt; die Schnittstelle dient nur zum LESEN. Neu ist
//! die Beobachtung: mit `TAXGRAPH_FRAGENLOG=<datei>` haelt jeder Durchlauf die Folge der Fragen fest (`fluss_i::fragenlog`),
//! mit `TAXGRAPH_WIRKUNGSLOG=<datei>` laeuft danach die Wirkungsprobe ([`wirkungsprobe`]). Ohne die Variablen geschieht nichts
//! davon; die Tests pruefen dann nur Zahl, Fragenfolge-Kennzahlen und Konsole.
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

/// Die Wirkungsprobe (U2 des Auftrags): fuer jedes Feld, das der Lauf gefragt hat, setzt sie zwei bis drei andere Werte, liest
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
                "int" => [0_i64, 10, 200].iter().filter(|c| ausgang != json!(**c)).map(|c| json!(c)).collect(),
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

/// Eine leere Antwort auf den Hausnummernzusatz und den BIC kommt nicht durch: die Fragen haben keinen Ausweg „keine“, und die
/// meisten Nutzer haben weder einen Zusatz noch eine ausländische Bank (Beobachtung U4). Was der Nutzer stattdessen tippt, steht
/// im Bericht (Messung „Strich“ unten).
#[test]
fn fluss_j_leerer_hausnummernzusatz_und_leerer_bic_kommen_nicht_durch() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    bis_zur_ersten_frage(&u, "gesamt", &[]);
    let t = einzel_tabelle("keine");
    durchlauf(&u, "einzel_leere_antworten", &t, &["stammdaten_hausnummerzusatz"]);
    let (weiter, banner) = versuche_antwort(&u, "stammdaten_hausnummerzusatz", "");
    eprintln!("MESSUNG Zusatz leer: weiter {weiter}, Banner {banner:?}");
    pp!("J.ein_leerer_hausnummernzusatz_kommt_nicht_durch", !weiter, "die leere Antwort auf den Zusatz ging durch; Banner {banner:?}");
    let (weiter, banner) = versuche_antwort(&u, "stammdaten_hausnummerzusatz", "-");
    eprintln!("MESSUNG Zusatz Strich: weiter {weiter}, Banner {banner:?}");
    durchlauf(&u, "einzel_leere_antworten", &t, &["stammdaten_bic"]);
    let (weiter, banner) = versuche_antwort(&u, "stammdaten_bic", "");
    eprintln!("MESSUNG BIC leer: weiter {weiter}, Banner {banner:?}");
    pp!("J.ein_leerer_bic_kommt_nicht_durch", !weiter, "die leere Antwort auf den BIC ging durch; Banner {banner:?}");
    let (weiter, banner) = versuche_antwort(&u, "stammdaten_bic", "-");
    eprintln!("MESSUNG BIC Strich: weiter {weiter}, Banner {banner:?}");
    pruefe_konsole(&u, "leere Antworten");
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
    pruefe_zahl_gezeigt(&u, "Quelle des Vorjahres");

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
    // BEFUND (gemessen 2026-10-20): die Uebernahme legt `veranlagung` als Vorschlag ab („vorlaeufig“) und nimmt die Frage aus der
    // Warteschlange; die Fragen zum Ehegatten verlangen aber eine BESTAETIGTE Antwort, bleiben also offen. Der Einzelnutzer wird
    // nach dem Ehegatten gefragt, und der zweite Fall stellt mehr Fragen als der erste. Wer die Uebernahme aendert, dreht diesen
    // Pruefpunkt um.
    pp!(
        "J.nach_der_vorjahr_uebernahme_fragt_die_oberflaeche_den_einzelnutzer_nach_dem_ehegatten",
        felder["veranlagung"]["zustand"] == json!("vorlaeufig") && wie_oft(&ziel, "veranlagung") == 0 && wie_oft(&ziel, "stammdaten_vorname_partner") == 1,
        "veranlagung {} / {}, Frage veranlagung {}-mal, Frage nach dem Vornamen des Ehegatten {}-mal",
        felder["veranlagung"]["wert"],
        felder["veranlagung"]["zustand"],
        wie_oft(&ziel, "veranlagung"),
        wie_oft(&ziel, "stammdaten_vorname_partner")
    );
    // BEFUND: alle 64 Fragen sind beantwortet, trotzdem steht keine Zahl da. Das Geburtsdatum aus dem Vorjahr ist „vorlaeufig“, die
    // Frage danach fehlt in der Warteschlange (uebernehmbar), und die Sperre nennt als offen nur `geburtsjahr`.
    let (_, _, api) = ergebnis(&u);
    pp!(
        "J.nach_der_vorjahr_uebernahme_endet_der_fragebogen_ohne_zahl",
        api["grund"] == json!("ring_betrag_vorlaeufig") && api["offen"] == json!(["geburtsjahr"]) && api["zahl_cent"].is_null(),
        "Grund {}, offen {}, Zahl {}",
        api["grund"],
        api["offen"],
        api["zahl_cent"]
    );
    eprintln!("MESSUNG Ergebnisseite ohne Zahl: {:?}", u.s.text("#ergebnis"));
    let zeile = "#belegt-liste li:has(.z-name[title='stammdaten_geburtsdatum'])";
    let da = u.s.finde(zeile).is_some();
    pp!("J.die_korrektur_des_geburtsdatums_oeffnet_die_sperre_nicht", da, "die Zeile des Geburtsdatums fehlt in „Schon beantwortet“");
    crate::fluss_i::korrigiere(&u, "stammdaten_geburtsdatum", Gib::Text("15.03.1985"));
    let (gezeigt, _, api) = ergebnis(&u);
    eprintln!("MESSUNG nach der Korrektur des Geburtsdatums: Grund {}, offen {}, Zahl {}, gezeigt {gezeigt:?}", api["grund"], api["offen"], api["zahl_cent"]);
    let vorlaeufig: Vec<String> = u.stand_felder(&u.fall()).as_object().unwrap().iter().filter(|(_, f)| f["zustand"] == json!("vorlaeufig")).map(|(k, _)| k.clone()).collect();
    eprintln!("MESSUNG noch vorlaeufig: {} Felder: {vorlaeufig:?}", vorlaeufig.len());
    pp!(
        "J.die_korrektur_des_geburtsdatums_oeffnet_die_sperre_nicht",
        api["grund"] == json!("ring_betrag_vorlaeufig") && api["offen"] == json!(["geburtsjahr"]) && vorlaeufig.iter().any(|f| f == "geburtsjahr"),
        "Grund {}, offen {}, vorlaeufig {vorlaeufig:?}",
        api["grund"],
        api["offen"]
    );
    pruefe_konsole(&u, "Vorjahr");
}
