//! Fluss B: vom Fallstart bis zur ersten Frage, Ankreuzliste und Auswahl-Buttons.
//!
//! Gegenstueck zu `tests/test_ui_screening_liste.py` (7 Browser-Funktionen) und `tests/test_ui_wahl_buttons.py` (7).
//! Die achte Funktion der Liste (`die_bindung_erklaert_welche_fragen_dazugehoeren`) ist ein Bindungstest ohne
//! Browser (siehe `ZUORDNUNG.tsv`). Geld: nein, aber Daten: ein Kreuz schreibt `kein_*` mit Umkehr in den Store.

use serde_json::json;

use crate::dienst::Start;
use crate::seite::Umgebung;

/// Eine frische Seite im Weg `weg` (Kachel „Gesamt“, Wegwahl) mit Sichtflaeche `breite` mal `hoehe`.
fn neuer_fall(u: &Umgebung, weg: &str, breite: u32, hoehe: u32) {
    u.s.sichtflaeche(breite, hoehe);
    u.oeffne_start();
    u.fall_beginnen(weg);
    let _ = u.konsolenfehler(); // Ausgangslage: Meldungen frueherer Seiten zaehlen nicht
}

fn aktive_buttons(u: &Umgebung) -> serde_json::Value {
    u.s.ausdruck("[...document.querySelectorAll('.wahl-opt.aktiv')].map(e => e.dataset.wert)")
}

#[test]
fn fluss_b_fallstart_und_ankreuzliste() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let s = &u.s;

    // Die Liste steht am Anfang des Fragebogens, mit allen Themen, ohne Konsolenfehler.
    neuer_fall(&u, "fragebogen", 1280, 900);
    s.warte_sichtbar("#screening", 8);
    pp!("B.die_liste_steht_am_anfang_des_fragebogens", !s.sichtbar("#wegpunkt"), "die Frage-Karte steht neben der Liste: zwei Aufforderungen");
    let n = s.anzahl("#screening-liste .sc-box");
    pp!("B.die_liste_steht_am_anfang_des_fragebogens", n >= 10, "nur {n} Kaestchen in der Liste");
    let text = s.textinhalt("#screening");
    for wort in ["Vermietung", "Kapitalerträge", "Behinderung"] {
        pp!("B.die_liste_steht_am_anfang_des_fragebogens", text.contains(wort), "„{wort}“ kommt in der Liste nicht vor");
    }
    let fehler = u.konsolenfehler();
    pp!("B.die_liste_steht_am_anfang_des_fragebogens", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Ein Kreuz heisst „ja“ (Umkehr bei `kein_kap`), ein leeres Kaestchen „nein“; gemessen wird der Store.
    let fall = u.fall();
    s.skript("document.querySelector('#screening-liste .sc-box[data-feld=\"kein_kap\"]').checked = true;", &[]);
    s.klick("#screening-weiter");
    s.warte_unsichtbar("#screening", 8);
    let felder = u.stand_felder(&fall);
    pp!(
        "B.ein_kreuz_heisst_ja_und_ein_leeres_kaestchen_nein",
        felder["kein_kap"]["wert"] == json!(false),
        "angekreuzt wurde als {} gespeichert: der Nutzer verliert seine Kapitalertraege aus der Erklaerung",
        felder["kein_kap"]["wert"]
    );
    pp!(
        "B.ein_kreuz_heisst_ja_und_ein_leeres_kaestchen_nein",
        felder["kein_vuv"]["wert"] == json!(true),
        "nicht angekreuzt wurde als {} gespeichert",
        felder["kein_vuv"]["wert"]
    );
    pp!("B.ein_kreuz_heisst_ja_und_ein_leeres_kaestchen_nein", felder["kein_kap"]["zustand"] == json!("bestaetigt"), "Zustand nicht bestaetigt");

    // Nichts angekreuzt = alles verneint: die Liste nimmt Folgefragen weg, danach kommt die erste echte Frage, und sie kommt nicht wieder.
    neuer_fall(&u, "fragebogen", 1280, 900);
    s.warte_sichtbar("#screening", 8);
    let fall = u.fall();
    let vorher = u.offene(&fall);
    s.klick("#screening-weiter");
    s.warte_unsichtbar("#screening", 8);
    let nachher = u.offene(&fall);
    pp!(
        "B.die_liste_nimmt_die_folgefragen_weg",
        nachher.len() + 100 < vorher.len(),
        "die Liste nimmt nur {} Fragen weg (gemessen: 147)",
        vorher.len() - nachher.len().min(vorher.len())
    );
    for f in ["kap_kapitalertraege", "vv_einnahmen", "kind_vorname"] {
        pp!("B.die_liste_nimmt_die_folgefragen_weg", !nachher.iter().any(|x| x == f), "{f} steht noch offen, obwohl das Thema verneint ist");
    }
    s.warte_sichtbar("#wegpunkt", 8);
    pp!("B.nach_der_liste_kommt_der_fragebogen", !s.sichtbar("#screening"), "die Liste bleibt sichtbar");
    pp!("B.nach_der_liste_kommt_der_fragebogen", !s.textinhalt("#frage").trim().is_empty(), "der Fragebogen zeigt keine Frage");
    let da = s.auswerten("async () => await zeigeScreening()", &[]);
    pp!("B.beim_zweiten_mal_kommt_sie_nicht_wieder", da == json!(false), "die Liste erscheint ein zweites Mal, obwohl alles beantwortet ist");

    // Auf dem KI-Weg steht die Liste nicht im Weg; sie kommt erst beim Wechsel in den Fragebogen.
    neuer_fall(&u, "ki", 1280, 900);
    s.warte_sichtbar("#chat-body .chat-erklaer", 8);
    pp!("B.auf_dem_ki_weg_kommt_sie_nicht_sofort", !s.sichtbar("#screening"), "die Ankreuzliste steht auf dem KI-Weg im Weg");
    s.klick("#zum-fragebogen");
    s.warte_sichtbar("#screening", 8);
    let fehler = u.konsolenfehler();
    pp!("B.auf_dem_ki_weg_kommt_sie_nicht_sofort", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Schmaler Schirm (360 px): kein seitliches Scrollen, kein auslaufender Text, Tap-Ziele ab 44 px.
    neuer_fall(&u, "fragebogen", 360, 780);
    s.warte_sichtbar("#screening", 8);
    let m = s.ausdruck(
        "(() => { const zeilen = [...document.querySelectorAll('#screening-liste .sc-label')]; \
         const ueber = ['#screening', '#screening-liste'].map(sel => { const e = document.querySelector(sel); \
         return {sel, ueber: e ? e.scrollWidth - e.clientWidth : -1}; }).filter(x => x.ueber > 0); \
         return {breite: document.documentElement.scrollWidth, \
         kleinste: Math.min(...zeilen.map(e => e.getBoundingClientRect().height)), ueberlauf: ueber}; })()",
    );
    pp!("B.tap_ziele_und_schmaler_schirm", m["breite"].as_f64().unwrap() <= 360.0, "Seitwaerts-Scrollen bei 360 px: {}", m["breite"]);
    pp!("B.tap_ziele_und_schmaler_schirm", m["ueberlauf"] == json!([]), "Text laeuft aus seinem Kasten: {}", m["ueberlauf"]);
    pp!("B.tap_ziele_und_schmaler_schirm", m["kleinste"].as_f64().unwrap() >= 44.0, "kleinstes Tap-Ziel nur {} px", m["kleinste"]);

    // Ohne Python-Vorbild (Mutant U21 aus der Nachmessung in main): ein doppelter Aufruf legt genau EINEN Fall an. Die
    // Kachel sperrt sich nach dem ersten Klick (`disabled`), ein zweiter `click()` im selben Tick erreicht den Handler
    // darum nie; den Schutz in `waehleScheibe` selbst misst nur der zweite, direkte Aufruf im selben Tick. Gezaehlt
    // werden die POST /fall, die der Browser abschickt (der Aufruf kommt synchron bis zum `fetch`).
    const ZAEHLE_POST_FALL: &str = "window.__fall_posts = 0; const echt = window.fetch; \
         window.fetch = (u, o) => { if (o && o.method === 'POST' && new URL(String(u), location.href).pathname === '/fall') { window.__fall_posts++; } return echt(u, o); };";
    u.oeffne_start();
    let klicks = s.ausdruck(&format!(
        "(() => {{ {ZAEHLE_POST_FALL} const k = document.querySelector(\".kachel[data-scheibe='gesamt']\"); k.click(); k.click(); return window.__fall_posts; }})()"
    ));
    pp!("B.doppelklick_auf_die_kachel_legt_einen_fall_an", klicks == json!(1), "zwei Klicks im selben Tick schicken {klicks} POST /fall");
    u.oeffne_start();
    let aufrufe = s.ausdruck(&format!("(() => {{ {ZAEHLE_POST_FALL} waehleScheibe('gesamt'); waehleScheibe('gesamt'); return window.__fall_posts; }})()"));
    pp!("B.doppelklick_auf_die_kachel_legt_einen_fall_an", aufrufe == json!(1), "zwei Aufrufe im selben Tick schicken {aufrufe} POST /fall: der Schutz in waehleScheibe fehlt");
}

#[test]
fn fluss_b_auswahl_buttons() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 780);
    let s = &u.s;
    u.zum_fragebogen_neu();

    // Die erste Frage ist ein Button-Wahl-Feld mit Anzeigetexten statt Rohwerten.
    pp!("B.erste_frage_ist_eine_button_wahl", u.aktuelle_frage() == json!("veranlagung"), "erste Frage ist {}, erwartet veranlagung", u.aktuelle_frage());
    pp!("B.erste_frage_ist_eine_button_wahl", s.anzahl(".wahl .wahl-opt") > 0, "keine Auswahl-Buttons gerendert");
    pp!("B.erste_frage_ist_eine_button_wahl", s.anzahl("#eingabe select") == 0, "es ist weiter ein Dropdown");
    let texte = s.ausdruck("[...document.querySelectorAll('.wahl-opt')].map(e => e.textContent)");
    let texte: Vec<String> = texte.as_array().unwrap().iter().map(|t| t.as_str().unwrap().to_owned()).collect();
    pp!("B.buttons_tragen_den_anzeigetext_nicht_den_rohwert", texte.iter().any(|t| t.contains("Zusammenveranlagung")), "Anzeigetexte fehlen auf den Buttons: {texte:?}");
    pp!(
        "B.buttons_tragen_den_anzeigetext_nicht_den_rohwert",
        !texte.iter().any(|t| matches!(t.trim(), "einzel" | "zusammen")),
        "Rohwert steht auf einem Button: {texte:?}"
    );

    // Kein seitliches Scrollen bei 360 px, auch mit langen Button-Texten.
    let breite = s.ausdruck("document.documentElement.scrollWidth");
    pp!("B.keine_horizontale_scrollbar_bei_360px", breite.as_f64().unwrap() <= 360.0, "scrollWidth={breite} > 360");

    // Ein Klick setzt den Wert und markiert genau diesen Button.
    s.klick(".wahl-opt[data-wert='zusammen']");
    pp!("B.klick_setzt_den_wert_und_markiert_den_button", s.wert("#feld-input") == "zusammen", "der Klick landet nicht im versteckten Feld");
    pp!("B.klick_setzt_den_wert_und_markiert_den_button", aktive_buttons(&u) == json!(["zusammen"]), "genau ein Button darf aktiv sein: {}", aktive_buttons(&u));
    pp!(
        "B.klick_setzt_den_wert_und_markiert_den_button",
        s.ausdruck("document.querySelector(\".wahl-opt[data-wert='zusammen']\").getAttribute('aria-pressed')") == json!("true"),
        "aria-pressed fehlt am aktiven Button"
    );

    // Die Vorauswahl folgt dem Beispielwert (kein_kap: false = zweite Option), nicht der ersten Option.
    pp!("B.vorauswahl_folgt_dem_beispielwert_nicht_der_ersten_option", u.rendere_frage("kein_kap"), "kein_kap nicht erreichbar");
    pp!("B.vorauswahl_folgt_dem_beispielwert_nicht_der_ersten_option", s.wert("#feld-input") == "false", "die Vorauswahl folgt nicht dem Beispielwert (false): {}", s.wert("#feld-input"));
    pp!("B.vorauswahl_folgt_dem_beispielwert_nicht_der_ersten_option", aktive_buttons(&u) == json!(["false"]), "markiert sein muss der Beispielwert-Button: {}", aktive_buttons(&u));

    // „Ja“ auf „Hattest du Kapitalertraege?“ ergibt `kein_kap = false` (Umkehr bleibt beim Button-Umbau erhalten).
    s.klick(".wahl-opt[data-wert='true']");
    let wert = s.ausdruck("leseWert(AKTUELL)");
    pp!("B.bool_inversion_bleibt_erhalten", wert == json!(false), "„Ja“ auf „Hattest du Kapitalertraege?“ muss kein_kap=false ergeben, ist {wert}");

    // Eine lange Auswahl (dba_staat: 16 Werte) bleibt ein Dropdown.
    pp!("B.lange_auswahl_bleibt_ein_dropdown", u.rendere_frage("dba_staat"), "dba_staat ist in der Scheibe nicht offen");
    pp!("B.lange_auswahl_bleibt_ein_dropdown", s.anzahl("#eingabe select") > 0, "16 Staaten werden als Button-Reihe gerendert");
}
