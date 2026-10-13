//! Fluss C: eine Antwort schreiben (Euro zu Cent, Datum, Text mit Muster, leer, Umkehr invertierter Fragen).
//!
//! Gegenstueck zu `tests/test_ui_feldtypen.py` (10 Funktionen), `tests/test_ui_leerwert_stille_null.py` (5) und
//! `tests/test_ui_frage_polaritaet.py` (3). GELD: ein Euro-Feld, das nicht in Cent umrechnet, schreibt den
//! 100-fachen Betrag; eine nicht umgekehrte Frage speichert das Gegenteil der Antwort (3.486 EUR zu viel Steuer
//! beim Dienstwohnungs-Fall von 2026-08-20). Die Dienstwohnungs-Frage `dhf_keine_pflicht_dienstwohnung` ist seit
//! 2026-10-06 gestrichen (Rust: `dienstwohnung_gestrichen_hermetisch.rs`); die Faelle nehmen Felder, die es gibt.

use serde_json::{json, Value};

use crate::dienst::Start;
use crate::seite::Umgebung;

const ZEITRAUM_FELD: &str = "kind_wohnsitz_inland_zeitraum";
const MUSTER: &str = r"^\d{2}\.\d{2}-\d{2}\.\d{2}$";

/// `baueEingabe` + Eingabe + `leseWert` fuer eine Frage `q`: `{art, angekommen, gelesen}`.
fn baue_und_lies(u: &Umgebung, q: &Value, eingabe: &str) -> Value {
    u.s.auswerten(
        "(q, eingabe) => { const box = document.getElementById('eingabe'); \
         const el = baueEingabe(q, box, 'feld-input', 'frage', null); el.value = eingabe; \
         return {art: el.type || el.tagName.toLowerCase(), angekommen: el.value, gelesen: leseWert(q, el)}; }",
        &[q.clone(), json!(eingabe)],
    )
}

/// Das erste Feld des Falls mit `typ` (ohne Muster) in der Warteschlange.
fn erstes_feld(u: &Umgebung, fall: &str, typ: &str) -> String {
    let (_, antwort) = u.dienst.get(&format!("/fall/{fall}/fragen"));
    antwort["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["typ"] == typ && q["muster"].is_null() && q["frage_invertiert"] == json!(false))
        .unwrap_or_else(|| panic!("kein Feld vom Typ {typ} in der Warteschlange"))["feld_id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn fluss_c_antwort_schreiben() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let s = &u.s;
    u.zum_fragebogen_neu();
    let _ = u.konsolenfehler();

    // Jeder Bindungstyp nimmt seinen Wert an, `leseWert` liest ihn richtig, und der Dienst nimmt ihn an (kein 422).
    let probe = u.dienst.fall("gesamt", "typen-probe");
    let faelle = [
        ("cent", json!({"typ": "cent"}), "620.50", json!(62050)),
        ("int", json!({"typ": "int"}), "220", json!(220)),
        ("text", json!({"typ": "text", "beispielwert": "Anna"}), "Anna", json!("Anna")),
        ("datum", json!({"typ": "datum", "beispielwert": "05.05.1955"}), "1955-05-05", json!("05.05.1955")),
    ];
    for (name, q, eingabe, erwartet) in faelle {
        let mut q = q;
        q["feld_id"] = json!(format!("pruef_{name}"));
        q["frage_laie"] = json!("Prüffrage");
        let r = baue_und_lies(&u, &q, eingabe);
        pp!(
            "C.jeder_typ_nimmt_seinen_wert_an_und_der_store_akzeptiert_ihn",
            r["angekommen"] == json!(eingabe),
            "das Feld fuer {name} hat die Eingabe {eingabe:?} nicht angenommen (Feldart {}, drin steht {})",
            r["art"],
            r["angekommen"]
        );
        pp!(
            "C.jeder_typ_nimmt_seinen_wert_an_und_der_store_akzeptiert_ihn",
            r["gelesen"] == erwartet,
            "leseWert() liest {} statt {erwartet}",
            r["gelesen"]
        );
        let feld = erstes_feld(&u, &probe, name);
        let (status, antwort) = u.dienst.schreibe(&probe, &feld, &r["gelesen"]);
        pp!(
            "C.jeder_typ_nimmt_seinen_wert_an_und_der_store_akzeptiert_ihn",
            status == 200 || status == 201,
            "der Dienst weist {} fuer typ={name} (Feld {feld}) ab: {status} {antwort}",
            r["gelesen"]
        );
    }

    // `text` ist kein Zahlenfeld; ein Datum geht ins amtliche Format (und zurueck); leer und Unsinn schreiben nichts.
    let art = s.auswerten(
        "() => baueEingabe({feld_id: 'x', typ: 'text', beispielwert: 'Anna'}, document.getElementById('eingabe'), \
         'feld-input', 'frage', null).type",
        &[],
    );
    pp!("C.text_ist_kein_zahlenfeld", art != json!("number"), "`text` wird wieder als Zahlenfeld gebaut");
    pp!("C.text_ist_kein_zahlenfeld", art == json!("text"), "unerwartete Feldart fuer `text`: {art}");
    let r = s.auswerten(
        "() => { const q = {feld_id: 'x', typ: 'datum'}; \
         const el = baueEingabe(q, document.getElementById('eingabe'), 'feld-input', 'frage', null); \
         el.value = '2025-07-15'; const iso = leseWert(q, el); \
         const el2 = document.createElement('input'); el2.value = '15.07.2025'; \
         return {aus_kalender: iso, aus_text: leseWert(q, el2), \
         leer: leseWert(q, Object.assign(document.createElement('input'), {value: ''})), \
         muell: leseWert(q, Object.assign(document.createElement('input'), {value: 'morgen'}))}; }",
        &[],
    );
    pp!("C.datum_wird_in_das_amtliche_format_uebersetzt", r["aus_kalender"] == json!("15.07.2025"), "ISO nicht uebersetzt: {}", r["aus_kalender"]);
    pp!("C.datum_wird_in_das_amtliche_format_uebersetzt", r["aus_text"] == json!("15.07.2025"), "deutsches Datum kaputtgemacht: {}", r["aus_text"]);
    pp!("C.datum_wird_in_das_amtliche_format_uebersetzt", r["leer"].is_null(), "leeres Datumsfeld liefert {} statt nichts", r["leer"]);
    pp!("C.datum_wird_in_das_amtliche_format_uebersetzt", r["muell"].is_null(), "„morgen“ liefert {} statt nichts", r["muell"]);
    let r = s.auswerten(
        "() => { const box = document.getElementById('eingabe'); \
         const t = baueEingabe({feld_id: 'x', typ: 'text'}, box, 'feld-input', 'frage', 'Anna'); const wert_text = t.value; \
         const d = baueEingabe({feld_id: 'y', typ: 'datum'}, box, 'feld-input', 'frage', '05.05.1955'); \
         return {text: wert_text, datum_im_feld: d.value, datum_gelesen: leseWert({feld_id: 'y', typ: 'datum'}, d)}; }",
        &[],
    );
    pp!("C.beim_korrigieren_steht_der_alte_wert_wieder_da", r["text"] == json!("Anna"), "Textfeld verliert die Vorbelegung: {}", r["text"]);
    pp!("C.beim_korrigieren_steht_der_alte_wert_wieder_da", r["datum_im_feld"] == json!("1955-05-05"), "Kalenderfeld zeigt die Vorbelegung nicht: {}", r["datum_im_feld"]);
    pp!("C.beim_korrigieren_steht_der_alte_wert_wieder_da", r["datum_gelesen"] == json!("05.05.1955"), "Rundweg kaputt: {}", r["datum_gelesen"]);

    // Ein Wert gegen das Muster wird nicht geschrieben (Oberflaeche), und der Dienst weist ihn ab (jeder Schreiber).
    let q = json!({"feld_id": ZEITRAUM_FELD, "typ": "text", "muster": MUSTER, "standardwert": "01.01-31.12"});
    let r = s.auswerten(
        "(q) => { const box = document.getElementById('eingabe'); const el = baueEingabe(q, box, 'feld-input', 'frage', null); \
         const lies = v => { el.value = v; return leseWert(q, el); }; \
         return {art: el.type, pattern: el.getAttribute('pattern'), gut: lies('01.01-31.12'), julius: lies('01.01-31.122'), \
         leer: lies(''), quatsch: lies('immer')}; }",
        std::slice::from_ref(&q),
    );
    pp!("C.ein_wert_gegen_das_muster_wird_nicht_geschrieben", r["gut"] == json!("01.01-31.12"), "der gueltige Wert kommt nicht durch: {}", r["gut"]);
    pp!("C.ein_wert_gegen_das_muster_wird_nicht_geschrieben", r["julius"].is_null(), "„01.01-31.122“ wird angenommen: {}", r["julius"]);
    pp!("C.ein_wert_gegen_das_muster_wird_nicht_geschrieben", r["quatsch"].is_null() && r["leer"].is_null(), "leer/Unsinn werden angenommen: {r}");
    pp!("C.ein_wert_gegen_das_muster_wird_nicht_geschrieben", !r["pattern"].is_null() && r["pattern"] != json!(""), "kein HTML-pattern am Feld");
    let (_, fragen) = u.dienst.get(&format!("/fall/{probe}/fragen"));
    let zeitraum = fragen["fragen"].as_array().unwrap().iter().find(|f| f["feld_id"] == ZEITRAUM_FELD);
    pp!("C.der_store_weist_einen_wert_gegen_das_muster_ab", zeitraum.is_some_and(|f| !f["muster"].is_null()), "{ZEITRAUM_FELD} hat kein Muster in der Bindung: der Test misst nichts");
    let neu = u.dienst.fall("gesamt", "muster-probe");
    for (wert, soll_gehen) in [("01.01-31.122", false), ("immer", false), ("01.01-31.12", true)] {
        let (status, antwort) = u.dienst.schreibe(&neu, ZEITRAUM_FELD, &json!(wert));
        let ging = status == 200 || status == 201;
        pp!(
            "C.der_store_weist_einen_wert_gegen_das_muster_ab",
            ging == soll_gehen,
            "{wert:?}: erwartet {}, war das Gegenteil ({status} {antwort})",
            if soll_gehen { "angenommen" } else { "abgewiesen" }
        );
    }

    // Der uebliche Wert (Standardwert) laesst sich mit einem Klick uebernehmen; ohne Standardwert gibt es keinen Knopf;
    // der Beispielwert wird nie zur Vorbelegung.
    let r = s.auswerten(
        "(q) => { const box = document.getElementById('eingabe'); const el = baueEingabe(q, box, 'feld-input', 'frage', null); \
         const b = box.querySelector('.standardwert'); if (!b) return {knopf: false}; b.click(); \
         return {knopf: true, text: b.textContent, wert: el.value, gelesen: leseWert(q, el)}; }",
        &[q],
    );
    pp!("C.der_uebliche_wert_laesst_sich_mit_einem_klick_uebernehmen", r["knopf"] == json!(true), "kein Knopf fuer den ueblichen Wert");
    pp!("C.der_uebliche_wert_laesst_sich_mit_einem_klick_uebernehmen", r["text"].as_str().is_some_and(|t| t.contains("01.01-31.12")), "der Knopf sagt nicht, was er eintraegt: {}", r["text"]);
    pp!("C.der_uebliche_wert_laesst_sich_mit_einem_klick_uebernehmen", r["gelesen"] == json!("01.01-31.12"), "der uebernommene Wert kommt nicht durch die Pruefung: {r}");
    let da = s.auswerten(
        "() => { const box = document.getElementById('eingabe'); \
         baueEingabe({feld_id: 'bruttoarbeitslohn', typ: 'cent', beispielwert: 6200000}, box, 'feld-input', 'frage', null); \
         return !!box.querySelector('.standardwert'); }",
        &[],
    );
    pp!("C.ohne_standardwert_kein_knopf", da == json!(false), "ein Geldfeld bekommt einen „ueblichen Wert“ angeboten, der aus dem Beispielwert stammt");
    let q = json!({"feld_id": ZEITRAUM_FELD, "typ": "text", "muster": MUSTER, "beispielwert": "01.01-31.12"});
    let r = s.auswerten(
        "(q) => { const box = document.getElementById('eingabe'); const el = baueEingabe(q, box, 'feld-input', 'frage', null); \
         return {wert: el.value, platzhalter: el.placeholder, hinweis: formatHinweis(q)}; }",
        &[q],
    );
    pp!("C.beispielwert_wird_nie_zur_vorbelegung", r["wert"] == json!(""), "der Beispielwert landet als Vorbelegung im Feld: {}", r["wert"]);
    pp!("C.beispielwert_wird_nie_zur_vorbelegung", r["platzhalter"].as_str().is_some_and(|t| t.contains("01.01-31.12")), "der Platzhalter zeigt den Beispielwert nicht mehr");
    pp!("C.beispielwert_wird_nie_zur_vorbelegung", r["hinweis"].as_str().is_some_and(|t| t.contains("01.01-31.12")), "der Formathinweis nennt den Beispielwert nicht mehr");
    let (_, fragen) = u.dienst.get(&format!("/fall/{probe}/fragen"));
    let typen = |t: &str| fragen["fragen"].as_array().unwrap().iter().filter(|f| f["typ"] == t).count();
    pp!("C.die_bindung_enthaelt_diese_typen_wirklich", typen("text") >= 1 && typen("datum") >= 1, "text/datum kommen in der Warteschlange nicht vor");

    // Leerwerte: leer und ungueltiger Text lesen `undefined` (kein Event), eine echte 0 bleibt eine 0.
    s.sichtflaeche(360, 780);
    pp!("C.leeres_feld_liest_undefined_nicht_null", u.rendere_frage("bruttoarbeitslohn"), "bruttoarbeitslohn nicht erreichbar");
    s.fuelle("#feld-input", "");
    let wert = s.ausdruck("leseWert(AKTUELL)");
    pp!("C.leeres_feld_liest_undefined_nicht_null", wert.is_null(), "leseWert() bei leerem Feld muss undefined liefern, ist {wert}");
    s.skript("document.getElementById('feld-input').value = '12,5';", &[]);
    pp!("C.komma_statt_punkt_liest_undefined", s.wert("#feld-input").is_empty(), "der Browser laesst ein Komma im Zahlenfeld doch durch");
    let wert = s.ausdruck("leseWert(AKTUELL)");
    pp!("C.komma_statt_punkt_liest_undefined", wert.is_null(), "leseWert() bei browser-ungueltigem Text muss undefined liefern, ist {wert}");
    s.fuelle("#feld-input", "0");
    let wert = s.ausdruck("leseWert(AKTUELL)");
    pp!("C.explizite_null_bleibt_eine_echte_null", wert == json!(0), "leseWert() bei expliziter 0 muss 0 liefern, ist {wert}");
    // Eine ungueltige Zahl in einem Ganzzahlfeld schreibt ebenfalls nichts (Mutant F6: stille Null).
    let n = s.auswerten(
        "() => { const el = document.createElement('input'); el.value = 'abc'; return leseWert({feld_id: 'x', typ: 'int'}, el); }",
        &[],
    );
    pp!("C.ungueltige_ganzzahl_liest_undefined_nicht_null", n.is_null(), "leseWert() bei ungueltiger Ganzzahl muss undefined liefern, ist {n}");

    // „Bestaetigen“ auf einem leeren Feld: kein Event, kein Fragenwechsel, Fehlermeldung im Banner.
    pp!("C.bestaetigen_schreibt_bei_leerem_feld_kein_event_und_zeigt_fehler", u.rendere_frage("bruttoarbeitslohn"), "bruttoarbeitslohn nicht erreichbar");
    s.fuelle("#feld-input", "");
    s.skript("versteckeNetzFehler();", &[]);
    s.klick("#bestaetigen");
    std::thread::sleep(std::time::Duration::from_millis(300));
    pp!(
        "C.bestaetigen_schreibt_bei_leerem_feld_kein_event_und_zeigt_fehler",
        u.aktuelle_frage() == json!("bruttoarbeitslohn"),
        "die Frage hat gewechselt: es wurde doch ein Event geschrieben"
    );
    pp!(
        "C.bestaetigen_schreibt_bei_leerem_feld_kein_event_und_zeigt_fehler",
        s.ausdruck("document.getElementById('netz-banner').hidden") == json!(false),
        "kein Fehlerbanner: der Nutzer erfaehrt nichts von der Ablehnung"
    );
    pp!(
        "C.bestaetigen_schreibt_bei_leerem_feld_kein_event_und_zeigt_fehler",
        s.textinhalt("#netz-banner").contains("gültigen Wert"),
        "unerwarteter Banner-Text: {}",
        s.textinhalt("#netz-banner")
    );
    pp!(
        "C.bestaetigen_schreibt_bei_leerem_feld_kein_event_und_zeigt_fehler",
        s.ausdruck("!document.getElementById('bestaetigen').disabled") == json!(true),
        "Bestaetigen bleibt nach der Ablehnung gesperrt"
    );
    let fall = u.fall();
    pp!(
        "C.bestaetigen_schreibt_bei_leerem_feld_kein_event_und_zeigt_fehler",
        u.dienst.ereignisse(&fall, "bruttoarbeitslohn").is_empty(),
        "im Store steht ein Ereignis zu bruttoarbeitslohn"
    );

    // Eine echte 0 wird bestaetigt und steht danach im Store; kein Fehlerbanner.
    s.fuelle("#feld-input", "0");
    s.skript("versteckeNetzFehler();", &[]);
    s.klick("#bestaetigen");
    s.warte_bis("!document.getElementById('bestaetigen').disabled", 5, "Bestaetigen wird wieder frei");
    s.warte_bis("STAND && STAND.felder && STAND.felder['bruttoarbeitslohn'] !== undefined", 8, "Stand kennt bruttoarbeitslohn");
    pp!("C.bestaetigen_schreibt_explizite_null_erfolgreich", s.ausdruck("document.getElementById('netz-banner').hidden") == json!(true), "Fehlerbanner bei einer gueltigen 0");
    let felder = u.stand_felder(&fall);
    pp!(
        "C.bestaetigen_schreibt_explizite_null_erfolgreich",
        felder["bruttoarbeitslohn"]["wert"] == json!(0),
        "der Store traegt nicht wert=0: {}",
        felder["bruttoarbeitslohn"]
    );

    // Ohne Python-Vorbild (Mutant U20 aus der Nachmessung in main): Euro mit Cent wird GERUNDET, nicht abgeschnitten.
    // In Gleitkomma liegt 4,35 * 100 bei 434,99999999999994 (Math.floor gaebe 434, der Nutzer verlore einen Cent je Feld).
    let r = s.auswerten(
        "() => { const q = {feld_id: 'x', typ: 'cent'}; const el = document.createElement('input'); \
         const lies = v => { el.value = v; return leseWert(q, el); }; \
         return {a: lies('4.35'), b: lies('19.99'), c: lies('1.15'), d: lies('0.57'), e: lies('620.5')}; }",
        &[],
    );
    pp!(
        "C.euro_mit_cent_wird_gerundet_nicht_abgeschnitten",
        r == json!({"a": 435, "b": 1999, "c": 115, "d": 57, "e": 62050}),
        "leseWert() liest Euro mit Cent falsch: {r}"
    );
    let feld = erstes_feld(&u, &fall, "cent");
    pp!("C.euro_mit_cent_wird_gerundet_nicht_abgeschnitten", u.rendere_frage(&feld), "{feld} nicht erreichbar");
    s.fuelle("#feld-input", "19.99");
    s.klick("#bestaetigen");
    s.warte_bis(&format!("STAND && STAND.felder && STAND.felder['{feld}'] !== undefined"), 8, "Stand kennt das Euro-Feld");
    let wert = u.dienst.letzter_wert(&fall, &feld);
    pp!("C.euro_mit_cent_wird_gerundet_nicht_abgeschnitten", wert == json!(1999), "19,99 EUR stehen als {wert} Cent in der Akte ({feld})");
}

#[test]
fn fluss_c_umkehr_invertierter_fragen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 780);
    let s = &u.s;
    u.zum_fragebogen_neu();

    // Klick des Nutzers -> gelesener Wert. Die Umkehr gilt fuer jedes invertierte Feld (nicht nur fuer `kein_*`).
    let invertiert = [
        ("vpf_keine_mahlzeitengestellung", "true", false),
        ("vpf_keine_mahlzeitengestellung", "false", true),
        ("kein_kap", "true", false),
        ("kein_kap", "false", true),
        ("kein_gewinn", "true", false),
        ("kein_vuv", "true", false),
        ("kein_sonstige", "true", false),
        ("kein_kind", "true", false),
        ("vv_nebenkosten_nicht_vereinbart", "true", false),
        ("hh_handwerker_keine_foerderung", "true", false),
    ];
    for (fid, klick, erwartet) in invertiert {
        pp!("C.invertierte_frage_speichert_die_bedeutung_der_antwort", u.rendere_frage(fid), "{fid} ist in der Scheibe „gesamt“ nicht erreichbar");
        s.klick(&format!(".wahl-opt[data-wert='{klick}']"));
        let wert = s.ausdruck("leseWert(AKTUELL)");
        pp!(
            "C.invertierte_frage_speichert_die_bedeutung_der_antwort",
            wert == json!(erwartet),
            "Klick {klick} -> {fid} muss {erwartet} sein, ist {wert}: die Umkehr fehlt, der Store hielte das Gegenteil der Antwort"
        );
    }
    // Die Kehrseite: eine Frage, die die Verneinung selbst traegt, wird nicht umgekehrt.
    pp!("C.selbstverneinende_frage_wird_nicht_umgekehrt", u.rendere_frage("uebernachtung_keine_lange_unterbrechung"), "uebernachtung_keine_lange_unterbrechung nicht erreichbar");
    s.klick(".wahl-opt[data-wert='false']");
    let wert = s.ausdruck("leseWert(AKTUELL)");
    pp!("C.selbstverneinende_frage_wird_nicht_umgekehrt", wert == json!(false), "uebernachtung_keine_lange_unterbrechung muss false sein, ist {wert}");

    // Die Verstanden-Seite zeigt die Antwort, nicht den Rohwert.
    let faelle = [
        ("vpf_keine_mahlzeitengestellung", true, "Nein"),
        ("vpf_keine_mahlzeitengestellung", false, "Ja"),
        ("kein_kap", true, "Nein"),
        ("vv_nebenkosten_nicht_vereinbart", true, "Nein"),
        ("vv_nebenkosten_nicht_vereinbart", false, "Ja"),
    ];
    for (fid, gespeichert, text) in faelle {
        let meta = s.auswerten(
            "async (fid) => { const r = await jget(`/fall/${FALL}/fragen`); const q = r.body.fragen.find(f => f.feld_id === fid); \
             return q ? {feld_id: q.feld_id, typ: q.typ, frage_invertiert: q.frage_invertiert} : null; }",
            &[json!(fid)],
        );
        pp!("C.verstanden_seite_zeigt_die_antwort_nicht_den_rohwert", !meta.is_null(), "{fid} nicht in der Scheibe „gesamt“ erreichbar");
        let mut v = meta;
        v["wert"] = json!(gespeichert);
        let angezeigt = s.auswerten("(v) => verstandenWertText(v)", &[v]);
        pp!(
            "C.verstanden_seite_zeigt_die_antwort_nicht_den_rohwert",
            angezeigt == json!(text),
            "{fid}={gespeichert} muss als „{text}“ erscheinen, steht als {angezeigt}"
        );
    }
}
