//! Fluss E: eine Sperre ist sichtbar und behebbar.
//!
//! Gegenstueck zu `tests/test_sperre_bietet_ihr_feld_an.py` (7 Funktionen). GELD und DATEN: eine Sperre ohne Weg
//! hinaus ist eine Sackgasse (kein Ergebnis, keine Abgabe); eine aufgeweichte Sperre rechnete mit einem Widerspruch.
//! Der Fall: gemeinsam veranlagen, eine Partnerangabe bestaetigen, dann auf „einzel“ umstellen.

use serde_json::{json, Value};

use crate::dienst::Start;
use crate::seite::Umgebung;

/// Die acht Partner-Felder (`partner_check.PARTNER_FELDER`), mit typkonformen Werten.
const WERT: [(&str, i64); 8] = [
    ("rentner_grad_der_behinderung_partner", 60),
    ("rentner_hilflos_blind_taubblind_partner", 1),
    ("kap_kapitalertraege_partner", 500_000),
    ("kap_gewinn_aktien_partner", 100_000),
    ("kap_gewinn_sonstige_partner", 100_000),
    ("kap_verlust_aktien_partner", 100_000),
    ("kap_verlust_sonstige_partner", 100_000),
    ("rentner_jahresrente_partner", 2_000_000),
];

fn laie(fid: &str, wert: &Value) -> Value {
    json!({"feld_id": fid, "wert": wert, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": format!("ok@{fid}")}})
}

/// Gemeinsam veranlagen, Partnerangabe bestaetigen, DANN auf einzel umstellen. Liefert `(ergebnis, event_id der Angabe)`.
fn sperrfall(u: &Umgebung, fall: &str, feld: &str, wert: &Value, scheibe: &str) -> (Value, String) {
    let d = &u.dienst;
    let (st, antwort) = d.post("/fall", &json!({"scheibe": scheibe, "veranlagungszeitraum": 2025, "fall_id": fall}));
    assert_eq!(st, 201, "{scheibe}: {antwort}");
    let (st, r) = d.post(&format!("/fall/{fall}/event"), &laie("veranlagung", &json!("zusammen")));
    assert_eq!(st, 201, "{r}");
    let veranlagung_ev = r["event_id"].as_str().unwrap().to_owned();
    let (st, r) = d.post(&format!("/fall/{fall}/event"), &laie(feld, wert));
    assert_eq!(st, 201, "{feld}: {r}");
    let feld_ev = r["event_id"].as_str().unwrap().to_owned();
    let mut umstellen = laie("veranlagung", &json!("einzel"));
    umstellen["ersetzt"] = json!(veranlagung_ev);
    let (st, r) = d.post(&format!("/fall/{fall}/event"), &umstellen);
    assert_eq!(st, 201, "Umstellung abgelehnt: {r}");
    let (st, erg) = d.get(&format!("/fall/{fall}/ergebnis"));
    assert_eq!(st, 200, "{erg}");
    (erg, feld_ev)
}

fn zum_sperrfall(u: &Umgebung, fall: &str) {
    u.oeffne_start();
    u.s.skript(
        &format!(
            "FALL = '{fall}'; document.getElementById('login').hidden = true; document.getElementById('start').hidden = true; \
             document.getElementById('flow').hidden = false; (async () => {{ await refresh(); }})();"
        ),
        &[],
    );
    std::thread::sleep(std::time::Duration::from_millis(500));
}

#[test]
fn fluss_e_sperre_sichtbar_und_behebbar() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1100, 1400);
    let s = &u.s;

    // Der Sperrgrund traegt die ausloesende Angabe samt Satz.
    let (erg, _) = sperrfall(&u, "sf1", "kap_kapitalertraege_partner", &json!(500_000), "gesamt");
    pp!("E.sperrgrund_traegt_die_ausloesende_feld_id", erg["grund"] == json!("partner_konsistenz_offen"), "grund: {erg}");
    let erwartet = json!([{
        "feld_id": "kap_kapitalertraege_partner", "wert": 500_000, "veranlagung": "einzel",
        "grund": "Du hast etwas bei „Kapitaleinkünfte des Partners“ eingetragen, aber keine Zusammenveranlagung gewählt. \
Partnerbezogene Angaben sind nur bei gemeinsamer Veranlagung möglich. Bitte prüfe deine Angaben.",
    }]);
    pp!("E.sperrgrund_traegt_die_ausloesende_feld_id", erg["sperr_felder"] == erwartet, "der Sperrgrund nennt die ausloesende Angabe nicht. Gemessen: {}", erg["sperr_felder"]);

    // Jedes der acht Partnerfelder verhaelt sich gleich.
    for (i, (feld, wert)) in WERT.into_iter().enumerate() {
        let scheibe = if feld == "rentner_jahresrente_partner" { "rentner_gesamt" } else { "gesamt" };
        let wert = if feld == "rentner_hilflos_blind_taubblind_partner" { json!(true) } else { json!(wert) };
        let (erg, _) = sperrfall(&u, &format!("sfp{i}"), feld, &wert, scheibe);
        pp!("E.jedes_partnerfeld_wird_genauso_gemeldet", erg["grund"] == json!("partner_konsistenz_offen"), "{feld} ({scheibe}): {erg}");
        let ids: Vec<&Value> = erg["sperr_felder"].as_array().unwrap().iter().map(|w| &w["feld_id"]).collect();
        pp!("E.jedes_partnerfeld_wird_genauso_gemeldet", ids == vec![&json!(feld)], "{feld}: {ids:?}");
    }

    // Die gemeldete Kennung ergibt eine Frage ueber den Einzelfeld-Weg.
    let fid = erg["sperr_felder"][0]["feld_id"].as_str().unwrap().to_owned();
    let (st, r) = u.dienst.get(&format!("/fall/sf1/feld/{fid}/frage"));
    pp!("E.sperrfeld_ist_ueber_den_einzelfeld_weg_erreichbar", st == 200, "{st} {r}");
    pp!("E.sperrfeld_ist_ueber_den_einzelfeld_weg_erreichbar", r["frage"]["feld_id"] == json!(fid), "{}", r["frage"]);
    pp!("E.sperrfeld_ist_ueber_den_einzelfeld_weg_erreichbar", r["frage"]["typ"] == json!("cent"), "{}", r["frage"]);

    // Die Korrektur der gemeldeten Angabe loest die Sperre.
    let (_, feld_ev) = sperrfall(&u, "sf3", "kap_kapitalertraege_partner", &json!(500_000), "gesamt");
    let mut korrektur = laie("kap_kapitalertraege_partner", &json!(0));
    korrektur["ersetzt"] = json!(feld_ev);
    let (st, r) = u.dienst.post("/fall/sf3/event", &korrektur);
    pp!("E.korrektur_hebt_die_sperre_auf", st == 201, "Korrektur abgelehnt: {st} {r}");
    let (st, erg2) = u.dienst.get("/fall/sf3/ergebnis");
    pp!("E.korrektur_hebt_die_sperre_auf", st == 200 && erg2["grund"] == json!("input_kegel_nicht_bestaetigt"), "die Sperre steht nach der Korrektur immer noch: grund={}", erg2["grund"]);

    // Ohne Widerspruch (Zusammenveranlagung) meldet die Antwort keine ausloesenden Felder.
    let (st, _) = u.dienst.post("/fall", &json!({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "sf4"}));
    assert_eq!(st, 201);
    for (fid, w) in [("veranlagung", json!("zusammen")), ("kap_kapitalertraege_partner", json!(500_000))] {
        let (st, r) = u.dienst.post("/fall/sf4/event", &laie(fid, &w));
        assert_eq!(st, 201, "{fid}: {r}");
    }
    let (st, erg) = u.dienst.get("/fall/sf4/ergebnis");
    assert_eq!(st, 200);
    pp!("E.ohne_sperre_keine_sperr_felder", erg["grund"] == json!("flag_konsistenz_offen"), "{erg}");
    pp!("E.ohne_sperre_keine_sperr_felder", erg["grund"] != json!("partner_konsistenz_offen"), "{erg}");
    pp!("E.ohne_sperre_keine_sperr_felder", erg["sperr_felder"] == json!([]), "ohne Widerspruch darf kein ausloesendes Feld gemeldet werden: {}", erg["sperr_felder"]);

    // Im Browser: die Sperranzeige bietet die Angabe an und oeffnet ihre Frage, mit `KORREKTUR_FID` fuer das `ersetzt`.
    sperrfall(&u, "sf5", "kap_kapitalertraege_partner", &json!(500_000), "gesamt");
    zum_sperrfall(&u, "sf5");
    s.skript("(async () => { await zeigeErgebnis(); })();", &[]);
    let vorn = s.try_warte_bis("(() => { const e = document.getElementById('fertig'); return !!e && !e.hidden; })()", 10);
    pp!("E.sperranzeige_oeffnet_die_frage_zu_ihrem_feld", vorn.is_some(), "der Sperr-Screen ist nicht vorn: der Test misst sonst einen anderen Zustand");
    // Ohne Python-Vorbild (Mutant U7 aus der Nachmessung in main): die Sperre zeigt den Klartext des Servers, nicht den
    // (anderen) Satz aus der Tabelle der Oberflaeche.
    let (_, erg5) = u.dienst.get("/fall/sf5/ergebnis");
    let klartext = erg5["klartext"].as_str().unwrap_or_default().to_owned();
    let gezeigt = s.ausdruck("(document.getElementById('ergebnis').firstChild || {}).textContent || ''");
    pp!("E.sperranzeige_zeigt_den_klartext_des_servers", !klartext.is_empty(), "der Dienst schickt keinen Klartext: {erg5}");
    pp!("E.sperranzeige_zeigt_den_klartext_des_servers", gezeigt == json!(klartext), "die Sperre zeigt {gezeigt} statt des Klartexts des Servers {klartext:?}");
    // Ohne Python-Vorbild (Mutanten U8, U9): die Vorab-Pruefung zeigt Widersprueche und Hinweise getrennt, Art und Text wie der
    // Dienst sie liefert, und die Liste ist sichtbar. Dieser Fall liefert beides (der Widerspruch ist die Sperre selbst).
    let (st, pf) = u.dienst.get("/fall/sf5/preflight");
    let items = pf["items"].as_array().cloned().unwrap_or_default();
    let hat = |typ: &str| items.iter().any(|i| i["typ"] == typ);
    pp!("E.vorab_pruefung_zeigt_widerspruch_und_hinweis_getrennt", st == 200 && hat("widerspruch") && hat("hinweis"), "Vorbedingung: der Fall liefert nicht beide Arten: {pf}");
    s.skript("(async () => { await zeigePreflight(); })();", &[]);
    let da = s.try_warte_bis("(() => { const e = document.getElementById('preflight'); return !!e && !e.hidden; })()", 10);
    pp!("E.vorab_pruefung_zeigt_widerspruch_und_hinweis_getrennt", da.is_some() && s.sichtbar("#preflight"), "die Vorab-Pruefung bleibt verborgen, obwohl sie {} Eintraege hat", items.len());
    let gezeigt = s.ausdruck(
        "[...document.querySelectorAll('#preflight-liste .preflight-eintrag')].map(li => ({klasse: li.className, \
         badge: li.querySelector('.preflight-badge').textContent, text: li.querySelector('.preflight-text').textContent}))",
    );
    let erwartet: Vec<Value> = items
        .iter()
        .map(|i| {
            let widerspruch = i["typ"] == "widerspruch";
            json!({"klasse": format!("preflight-eintrag preflight-{}", if widerspruch { "widerspruch" } else { "hinweis" }),
                   "badge": if widerspruch { "Widerspruch" } else { "Hinweis" }, "text": i["text"]})
        })
        .collect();
    pp!("E.vorab_pruefung_zeigt_widerspruch_und_hinweis_getrennt", gezeigt == json!(erwartet), "Anzeige {gezeigt} statt {}", json!(erwartet));
    let knoepfe = s.ausdruck("[...document.querySelectorAll('#ergebnis [data-sperr-feld]')].map(e => e.getAttribute('data-sperr-feld'))");
    pp!("E.sperranzeige_oeffnet_die_frage_zu_ihrem_feld", knoepfe == json!(["kap_kapitalertraege_partner"]), "die Sperranzeige bietet die ausloesende Angabe nicht an: {knoepfe}");
    s.klick("#ergebnis [data-sperr-feld]");
    std::thread::sleep(std::time::Duration::from_millis(800));
    let nach = s.ausdruck(
        "({wegpunkt_hidden: document.getElementById('wegpunkt').hidden, \
         aktuell: (typeof AKTUELL !== 'undefined' && AKTUELL) ? AKTUELL.feld_id : null, \
         korrektur_fid: (typeof KORREKTUR_FID !== 'undefined') ? KORREKTUR_FID : null, \
         banner: (document.getElementById('netz-banner') || {}).textContent || ''})",
    );
    pp!("E.sperranzeige_oeffnet_die_frage_zu_ihrem_feld", nach["wegpunkt_hidden"] == json!(false), "{nach}");
    pp!("E.sperranzeige_oeffnet_die_frage_zu_ihrem_feld", nach["aktuell"] == json!("kap_kapitalertraege_partner"), "{nach}");
    pp!("E.sperranzeige_oeffnet_die_frage_zu_ihrem_feld", nach["korrektur_fid"] == json!("kap_kapitalertraege_partner"), "ohne KORREKTUR_FID schickt bestaetigen() kein `ersetzt` (Auflage B): {nach}");
    pp!("E.sperranzeige_oeffnet_die_frage_zu_ihrem_feld", nach["banner"].as_str().unwrap_or("").trim().is_empty(), "{nach}");
    let fehler = u.konsolenfehler();
    pp!("E.sperranzeige_oeffnet_die_frage_zu_ihrem_feld", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Die Gegenprobe: die normale Korrektur ueber die Belegt-Liste bleibt im Sperrzustand gesperrt.
    sperrfall(&u, "sf6", "kap_kapitalertraege_partner", &json!(500_000), "gesamt");
    zum_sperrfall(&u, "sf6");
    let rel = s.ausdruck("(STAND.relevanz || {})['p2_festzusetzung_zusammen']");
    pp!("E.normale_korrektur_bleibt_im_sperrzustand_gesperrt", rel["status"] == json!("ausgeschlossen"), "{rel}");
    let geklickt = s.auswerten(
        "() => { const li = [...document.querySelectorAll('#belegt-liste li')].find(x => x.querySelector('.z-name') \
         && x.querySelector('.z-name').title === 'kap_kapitalertraege_partner'); if (!li) return false; li.click(); return true; }",
        &[],
    );
    pp!("E.normale_korrektur_bleibt_im_sperrzustand_gesperrt", geklickt == json!(true), "die Belegt-Zeile des Feldes fehlt: der Test misst sonst nichts");
    std::thread::sleep(std::time::Duration::from_millis(800));
    let nach = s.ausdruck(
        "({aktuell: (typeof AKTUELL !== 'undefined' && AKTUELL) ? AKTUELL.feld_id : null, \
         korrektur_fid: (typeof KORREKTUR_FID !== 'undefined') ? KORREKTUR_FID : null, \
         banner: (document.getElementById('netz-banner') || {}).textContent || ''})",
    );
    pp!("E.normale_korrektur_bleibt_im_sperrzustand_gesperrt", nach["aktuell"] != json!("kap_kapitalertraege_partner"), "korrigiereBestaetigt ist im Sperrzustand durchgelassen worden: {nach}");
    pp!("E.normale_korrektur_bleibt_im_sperrzustand_gesperrt", nach["korrektur_fid"].is_null(), "{nach}");
    pp!(
        "E.normale_korrektur_bleibt_im_sperrzustand_gesperrt",
        nach["banner"].as_str().unwrap_or("").trim()
            == "Diese Frage ist durch eine andere Antwort entfallen und lässt sich nicht mehr ändern. Willst du sie zurückholen, ändere die Antwort, die sie abgeschaltet hat.",
        "die Abweisung klingt anders als gemessen: {}",
        nach["banner"]
    );
}
