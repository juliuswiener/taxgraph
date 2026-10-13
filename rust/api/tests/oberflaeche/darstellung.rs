//! Die Darstellung: Tap-Ziele und Breite bei 360 px, lesbare Werte, Fortschritts-Nenner, Panel, Grundlagen der
//! Barrierefreiheit (24 Python-Funktionen aus `test_ui_responsive`, `test_ui_lesbare_werte`,
//! `test_ui_fortschritt_und_panel`, `test_ui_a11y`). Kennungen `R.` (responsive), `L.` (lesbare Werte), `P.`
//! (Fortschritt und Panel), `Y.` (Barrierefreiheit).
//!
//! Ein Fehler hier kostet kein Geld und schreibt keine Daten falsch; er macht die Oberflaeche unbenutzbar oder
//! irrefuehrend (Fortschritt, der „fertig“ sagt, ein Wert, der als Rohzahl dasteht). Gemessen wird mit den Mitteln der
//! Fluesse (WebDriver, 360 px per CDP), ohne neue Bibliothek.
//!
//! BENANNTE GRENZE zu `test_ui_a11y.py`: Python laedt `axe-core` aus dem Netz (CDN) und ueberspringt den Test still,
//! wenn das Netz fehlt. Hier gibt es kein `axe-core` und kein Netz; `Y.*` misst die Regeln selbst nach, die axe als
//! kritisch oder ernst fuehrt und die sich ohne Bibliothek messen lassen (Sprache, Titel, Name von Knoepfen, Beschriftung von
//! Eingabefeldern, `alt`, Linktext, Farbkontrast ueber einfarbigen Grund). Was axe darueber hinaus kennt, fehlt.

use serde_json::{json, Value};

use crate::browser::{seriell, Sitzung};
use crate::dienst::{Dienst, Start};

fn ereignis(feld: &str, wert: &Value, ersetzt: Option<&str>) -> Value {
    let mut ev = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
    });
    if let Some(e) = ersetzt {
        ev["ersetzt"] = json!(e);
        ev["signal"]["signal_1"] = json!(e);
    }
    ev
}

fn schreibe(d: &Dienst, fall: &str, feld: &str, wert: &Value, ersetzt: Option<&str>) -> u16 {
    d.post(&format!("/fall/{fall}/event"), &ereignis(feld, wert, ersetzt)).0
}

/// Die offenen Fragen eines Falls, ganz (`GET /fragen`).
fn fragen(d: &Dienst, fall: &str) -> Vec<Value> {
    let (status, antwort) = d.get(&format!("/fall/{fall}/fragen"));
    assert_eq!(status, 200, "fragen: {antwort}");
    antwort["fragen"].as_array().cloned().unwrap_or_default()
}

fn refresh(s: &Sitzung) {
    s.auswerten("async () => { await refresh(); }", &[]);
}

/// Boxen: `(links, oben, breite, hoehe)`, oder `None`, wenn das Element fehlt oder keine Box hat.
fn kasten(s: &Sitzung, css: &str) -> Option<(f64, f64, f64, f64)> {
    let r = s.ausdruck(&format!(
        "(() => {{ const e = document.querySelector({css:?}); if (!e) return null; const r = e.getBoundingClientRect(); \
          return (r.width > 0 || r.height > 0) ? [r.left, r.top, r.width, r.height] : null; }})()"
    ));
    let z = |i: usize| r[i].as_f64();
    Some((z(0)?, z(1)?, z(2)?, z(3)?))
}

fn scroll_breite(s: &Sitzung) -> i64 {
    s.ausdruck("document.documentElement.scrollWidth").as_i64().unwrap_or(-1)
}

// ---- test_ui_responsive: 360 x 640 ----------------------------------------------------------------------------------

#[test]
fn darstellung_responsive() {
    let _sperre = seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 640);
    let s = &u.s;

    // Start-Kacheln: 44 x 44 Tap-Ziele, kein seitwaerts Scrollen. `#start .kachel`: die Wegwahl benutzt dieselbe Bauart,
    // liegt aber noch versteckt im Dokument (ein verstecktes Element hat keine Box).
    u.oeffne_start();
    pp!("R.start_screen_kacheln_responsive", s.ausdruck("[window.innerWidth, window.innerHeight]") == json!([360, 640]), "die Sichtflaeche ist nicht 360 x 640");
    pp!("R.start_screen_kacheln_responsive", scroll_breite(s) <= 360, "seitwaerts Scrollen: scrollWidth={}", scroll_breite(s));
    let kacheln = s.ausdruck(
        "[...document.querySelectorAll('#start .kachel')].map(k => { const r = k.getBoundingClientRect(); return [r.width, r.height]; })",
    );
    let kacheln = kacheln.as_array().cloned().unwrap_or_default();
    pp!("R.start_screen_kacheln_responsive", kacheln.len() >= 2, "keine Kacheln gefunden: {kacheln:?}");
    for (i, k) in kacheln.iter().enumerate() {
        let (b, h) = (k[0].as_f64().unwrap_or(0.0), k[1].as_f64().unwrap_or(0.0));
        pp!("R.start_screen_kacheln_responsive", h >= 44.0 && b >= 44.0, "Kachel {i}: {b} x {h} unter 44");
    }
    pp!("R.start_screen_no_horizontal_scroll", scroll_breite(s) <= 360, "scrollWidth={} > 360", scroll_breite(s));

    // Wegpunkt: zwei Knoepfe nebeneinander oder gestapelt, 44 x 44, ohne Ueberlappung.
    u.zum_fragebogen_neu();
    let best = kasten(s, "#bestaetigen");
    let chat = kasten(s, "#chat");
    pp!("R.wegpunkt_buttons_responsive", best.is_some() && chat.is_some(), "Knopf fehlt oder hat keine Box: {best:?} {chat:?}");
    let ((bx, by, bb, bh), (cx, cy, cb, ch)) = (best.unwrap_or_default(), chat.unwrap_or_default());
    pp!("R.wegpunkt_buttons_responsive", bh >= 44.0 && bb >= 44.0, "bestaetigen: {bb} x {bh}");
    pp!("R.wegpunkt_buttons_responsive", ch >= 44.0 && cb >= 44.0, "chat: {cb} x {ch}");
    pp!("R.wegpunkt_buttons_responsive", bx + bb <= cx + 1.0 || by + bh <= cy + 1.0, "die Knoepfe ueberlappen: ({bx},{by},{bb},{bh}) / ({cx},{cy},{cb},{ch})");
    pp!("R.wegpunkt_buttons_responsive", scroll_breite(s) <= 360, "scrollWidth={}", scroll_breite(s));

    // Eingabefeld der Frage: mindestens 44 px hoch (die Karte muss ein Eingabefeld tragen).
    let fall = u.fall();
    let cent = fragen(&u.dienst, &fall).into_iter().find(|q| q["typ"] == "cent").and_then(|q| q["feld_id"].as_str().map(str::to_owned));
    pp!("R.input_fields_tap_targets", cent.is_some(), "keine Cent-Frage im Fall: der Test misst nichts");
    pp!("R.input_fields_tap_targets", u.rendere_frage(&cent.unwrap_or_default()), "die Frage laesst sich nicht rendern");
    let eingabe = kasten(s, "#feld-input");
    pp!("R.input_fields_tap_targets", eingabe.is_some(), "#feld-input hat keine Box");
    pp!("R.input_fields_tap_targets", eingabe.is_some_and(|(_, _, _, h)| h >= 44.0), "Eingabefeld unter 44 px: {eingabe:?}");

    // Fortschrittsleiste: sichtbar, nicht breiter als 360 px.
    u.oeffne_start();
    u.fall_beginnen("fragebogen");
    s.warte_sichtbar("#flow", 8);
    pp!("R.fortschrittsleiste_responsive", s.finde("#fortschritt").is_some(), "#fortschritt fehlt");
    pp!("R.fortschrittsleiste_responsive", s.sichtbar("#fortschritt"), "die Fortschrittsleiste ist versteckt");
    let leiste = kasten(s, "#fortschritt");
    pp!("R.fortschrittsleiste_responsive", leiste.is_some_and(|(_, _, b, _)| b <= 360.0), "Leiste breiter als 360 px: {leiste:?}");

    // Belegt-Liste: jede Zeile mindestens 44 px hoch. (Der Python-Test lud die Seite NEU und fand keine Zeile, also
    // keine Pruefung; hier steht die Antwort im Fall der Seite, und es muss mindestens eine Zeile da sein.)
    u.zum_fragebogen_neu();
    let fall = u.fall();
    pp!("R.belegt_liste_tap_targets", schreibe(&u.dienst, &fall, "bruttoarbeitslohn", &json!(2_500_000), None) == 201, "die Antwort wird nicht geschrieben");
    refresh(s);
    s.warte_sichtbar("#belegt-liste .zeile", 8);
    let hoehen = s.ausdruck("[...document.querySelectorAll('#belegt-liste .zeile')].map(z => z.getBoundingClientRect().height)");
    let hoehen = hoehen.as_array().cloned().unwrap_or_default();
    pp!("R.belegt_liste_tap_targets", !hoehen.is_empty(), "keine Zeile in der Belegt-Liste");
    for (i, h) in hoehen.iter().enumerate() {
        pp!("R.belegt_liste_tap_targets", h.as_f64().unwrap_or(0.0) >= 44.0, "Zeile {i}: Hoehe {h} unter 44");
    }

    // Herkunft-Overlay (ein Modal) und KI-Berater (ein dauerhaftes Panel): bei 360 px nicht breiter als der Schirm.
    let berater = kasten(s, "#berater");
    pp!("R.overlay_und_berater_responsive", berater.is_some(), "der Berater ist nicht sichtbar: er soll dauerhaft offen sein");
    pp!("R.overlay_und_berater_responsive", berater.is_some_and(|(_, _, b, _)| b <= 360.0), "Berater breiter als 360 px: {berater:?}");
    pp!("R.overlay_und_berater_responsive", s.finde("#chat-text").is_some() && s.finde("#chat-send").is_some(), "Eingabefeld oder Absende-Knopf fehlt");
    s.skript("document.getElementById('kette-overlay').hidden = false;", &[]);
    let karte = kasten(s, "#kette-overlay .overlay-card");
    pp!("R.overlay_und_berater_responsive", karte.is_some(), "die Karte des Herkunft-Overlays hat keine Box: nicht offen");
    pp!("R.overlay_und_berater_responsive", karte.is_some_and(|(_, _, b, _)| b <= 360.0), "Overlay-Karte breiter als 360 px: {karte:?}");
    s.skript("document.getElementById('kette-zu').click();", &[]);
    s.warte_unsichtbar("#kette-overlay", 5);
    pp!("R.overlay_und_berater_responsive", !s.sichtbar("#kette-overlay"), "das Overlay laesst sich nicht schliessen");
}

// ---- test_ui_lesbare_werte: was der Nutzer ueber seine Angaben liest ------------------------------------------------

#[test]
fn darstellung_lesbare_werte() {
    let _sperre = seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let s = &u.s;
    let zeilen = || {
        s.ausdruck(
            "[...document.querySelectorAll('#belegt-liste .zeile')].map(li => ({name: (li.querySelector('.z-name') || {}).textContent || '', \
              kennung: (li.querySelector('.z-name') || {}).title || '', wert: (li.querySelector('.z-wert') || {}).textContent || ''}))",
        )
    };

    // Ein Cent-Betrag als Euro, davor die FRAGE statt der Feld-Kennung.
    u.zum_fragebogen_neu();
    let fall = u.fall();
    pp!("L.die_liste_zeigt_frage_und_lesbaren_wert", schreibe(&u.dienst, &fall, "bruttoarbeitslohn", &json!(2_500_000), None) == 201, "die Antwort wird nicht geschrieben");
    refresh(s);
    s.warte_sichtbar("#belegt-liste .zeile", 8);
    let z = zeilen();
    pp!("L.die_liste_zeigt_frage_und_lesbaren_wert", z.as_array().is_some_and(|a| a.len() == 1), "genau eine Zeile erwartet: {z}");
    let (name, wert, kennung) = (z[0]["name"].as_str().unwrap_or_default(), z[0]["wert"].as_str().unwrap_or_default(), z[0]["kennung"].as_str().unwrap_or_default());
    pp!("L.die_liste_zeigt_frage_und_lesbaren_wert", wert != "2500000", "der Cent-Rohwert steht wieder da: der Nutzer liest 2500000 als seinen Betrag");
    pp!("L.die_liste_zeigt_frage_und_lesbaren_wert", wert.contains("25.000") && wert.contains('€'), "kein lesbarer Euro-Betrag: {wert:?}");
    pp!("L.die_liste_zeigt_frage_und_lesbaren_wert", name != "bruttoarbeitslohn", "die Feld-Kennung steht wieder als Name da");
    pp!("L.die_liste_zeigt_frage_und_lesbaren_wert", name.chars().count() > 15 && name.contains('?'), "der Name ist keine Frage: {name:?}");
    pp!("L.die_liste_zeigt_frage_und_lesbaren_wert", kennung == "bruttoarbeitslohn", "die Feld-Kennung ist ganz verschwunden: {kennung:?}");

    // Ein Bool als Ja/Nein, und bei den `kein_`-Feldern mit der Umkehr des Schreibpfads.
    u.zum_fragebogen_neu();
    let fall = u.fall();
    pp!("L.ein_bool_steht_als_ja_oder_nein_da", schreibe(&u.dienst, &fall, "kein_kap", &json!(true), None) == 201, "die Antwort wird nicht geschrieben");
    refresh(s);
    s.warte_sichtbar("#belegt-liste .zeile", 8);
    let z = zeilen();
    let wert = z[0]["wert"].as_str().unwrap_or_default();
    pp!("L.ein_bool_steht_als_ja_oder_nein_da", wert == "Ja" || wert == "Nein", "Rohwert statt Ja/Nein: {wert:?}");
    pp!("L.ein_bool_steht_als_ja_oder_nein_da", wert == "Nein", "die Umkehr der kein_-Felder fehlt in der Anzeige: {wert:?}: der Nutzer liest das Gegenteil seiner Antwort");

    // „Schon beantwortet“ steht nicht ueber einer leeren Liste.
    u.zum_fragebogen_neu();
    let fall = u.fall();
    pp!("L.ohne_antworten_steht_die_ueberschrift_nicht_da", !s.sichtbar(".belegt"), "die Ueberschrift „Schon beantwortet“ steht da, obwohl nichts beantwortet ist");
    schreibe(&u.dienst, &fall, "bruttoarbeitslohn", &json!(2_500_000), None);
    refresh(s);
    s.warte_sichtbar("#belegt-liste .zeile", 8);
    pp!("L.ohne_antworten_steht_die_ueberschrift_nicht_da", s.sichtbar(".belegt"), "und jetzt fehlt sie, obwohl etwas beantwortet ist");

    // Der Kopf: ohne Zahl behauptet er nicht, ein Wert stehe fest.
    u.zum_fragebogen_neu();
    s.try_warte_bis("!document.getElementById('spanne').textContent.includes('berechnet')", 8);
    let kopf = s.ausdruck("({spanne: document.getElementById('spanne').textContent, hint: document.getElementById('spanne-hint').textContent})");
    pp!("L.ohne_zahl_behauptet_der_kopf_nicht_dass_etwas_feststeht", !kopf["spanne"].as_str().unwrap_or_default().contains('—'), "ein Gedankenstrich als Wert: {kopf}");
    pp!("L.ohne_zahl_behauptet_der_kopf_nicht_dass_etwas_feststeht", kopf["hint"] != json!("steht"), "„steht“ ohne Zahl daneben: {kopf}");
    pp!("L.ohne_zahl_behauptet_der_kopf_nicht_dass_etwas_feststeht", !kopf["spanne"].as_str().unwrap_or_default().trim().is_empty(), "der Kopf ist leer: {kopf}");

    // Ein langer Wert quetscht die Frage nicht zu einer Buchstabensaeule (360 px): Breite UND Hoehe messen.
    s.sichtflaeche(360, 780);
    u.zum_fragebogen_neu();
    let fall = u.fall();
    schreibe(&u.dienst, &fall, "veranlagung", &json!("zusammen"), None);
    refresh(s);
    s.warte_sichtbar("#belegt-liste .zeile", 8);
    let m = s.ausdruck(
        "(() => { const li = document.querySelector('#belegt-liste .zeile'); const n = li.querySelector('.z-name'), w = li.querySelector('.z-wert'); \
          const zh = parseFloat(getComputedStyle(n).lineHeight) || 20; \
          return {name_breite: n.getBoundingClientRect().width, name_hoehe: n.getBoundingClientRect().height, \
                  zeilen: Math.round(n.getBoundingClientRect().height / zh), name_text: n.textContent, wert_breite: w.getBoundingClientRect().width}; })()",
    );
    pp!("L.ein_langer_wert_quetscht_die_frage_nicht_zu_tode", m["name_text"].as_str().is_some_and(|t| t.chars().count() > 20), "Vorbedingung: der Fragetext muss lang sein: {m}");
    pp!("L.ein_langer_wert_quetscht_die_frage_nicht_zu_tode", m["name_breite"].as_f64().unwrap_or(0.0) >= 80.0, "der Fragetext hat nur {} px: er bricht buchstabenweise um (der Wert daneben nimmt {} px)", m["name_breite"], m["wert_breite"]);
    pp!("L.ein_langer_wert_quetscht_die_frage_nicht_zu_tode", m["zeilen"].as_f64().unwrap_or(99.0) <= 6.0, "der Fragetext laeuft ueber {} Zeilen: eine Buchstabensaeule", m["zeilen"]);
}

// ---- test_ui_fortschritt_und_panel, Teil 1: der Nenner ---------------------------------------------------------------

struct Anzeige {
    mitte: String,
    text: String,
    max: f64,
    wert: f64,
    anteil: f64,
    fest: i64,
    gesamt: i64,
}

fn anzeige(s: &Sitzung) -> Anzeige {
    s.warte_bis("document.getElementById('ring-mitte').textContent.includes('/')", 8, "der Ring zeigt Zaehler/Nenner");
    let r = s.ausdruck(
        "({mitte: document.getElementById('ring-mitte').textContent, text: document.getElementById('fortschritt-text').textContent, \
          max: document.getElementById('fortschritt').max, wert: document.getElementById('fortschritt').value, \
          anteil: document.getElementById('ring').style.getPropertyValue('--anteil')})",
    );
    let mitte = r["mitte"].as_str().unwrap_or_default().to_owned();
    let (z, n) = mitte.split_once('/').unwrap_or((mitte.as_str(), ""));
    Anzeige {
        text: r["text"].as_str().unwrap_or_default().to_owned(),
        max: r["max"].as_f64().unwrap_or(f64::NAN),
        wert: r["wert"].as_f64().unwrap_or(f64::NAN),
        anteil: r["anteil"].as_str().and_then(|a| a.trim().parse().ok()).unwrap_or(f64::NAN),
        fest: z.trim().parse().unwrap_or(-1),
        gesamt: n.trim().parse().unwrap_or(-1),
        mitte,
    }
}

fn kandidatenwerte(q: &Value) -> Vec<Value> {
    match q["typ"].as_str() {
        Some("bool") => vec![json!(false), json!(true)],
        Some("enum") => q["enum_werte"].as_array().cloned().unwrap_or_default(),
        _ => {
            if q["beispielwert"].is_null() {
                Vec::new()
            } else {
                vec![q["beispielwert"].clone()]
            }
        }
    }
}

/// Erstes `(feld, wert, offen_vorher, offen_nachher)`, fuer das `treffer(vorher, nachher)` gilt. Jede Probe auf einem
/// eigenen Fall; die Entscheidung faellt ueber HTTP, nie an der Anzeige, die hier geprueft wird.
fn sonde(d: &Dienst, zaehler: &mut u32, kandidaten: &[Value], treffer: impl Fn(usize, usize) -> bool) -> Option<(String, Value, usize, usize)> {
    for q in kandidaten {
        let feld = q["feld_id"].as_str()?;
        for wert in kandidatenwerte(q) {
            *zaehler += 1;
            let fall = d.fall("gesamt", &format!("sonde-{zaehler}"));
            let vorher = fragen(d, &fall).len();
            let status = schreibe(d, &fall, feld, &wert, None);
            if !(status == 200 || status == 201) {
                continue;
            }
            let nachher = fragen(d, &fall).len();
            if treffer(vorher, nachher) {
                return Some((feld.to_owned(), wert, vorher, nachher));
            }
        }
    }
    None
}

/// Erster Gegenwert, der die abgeschalteten Fragen wieder in die Queue holt, und wie viele.
fn sonde_korrektur(d: &Dienst, zaehler: &mut u32, feld: &str, wert: &Value, gegenwerte: &[Value]) -> Option<(Value, usize)> {
    for gegen in gegenwerte {
        *zaehler += 1;
        let fall = d.fall("gesamt", &format!("sonde-{zaehler}"));
        schreibe(d, &fall, feld, wert, None);
        let eng = fragen(d, &fall).len();
        let (st, warum) = d.get(&format!("/fall/{fall}/feld/{feld}/warum"));
        if st != 200 {
            continue;
        }
        let alt = warum["justification"]["event_id"].as_str()?.to_owned();
        let st = schreibe(d, &fall, feld, gegen, Some(&alt));
        if !(st == 200 || st == 201) {
            continue;
        }
        let weit = fragen(d, &fall).len();
        if weit > eng {
            return Some((gegen.clone(), weit - eng));
        }
    }
    None
}

#[test]
fn darstellung_nenner() {
    let _sperre = seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 780);
    let (s, d) = (&u.s, &u.dienst);
    let mut zaehler = 0u32;
    let basis_fall = d.fall("gesamt", "nenner-basis");
    let basis = fragen(d, &basis_fall);

    // Der Ausgangspunkt: noch nichts beantwortet, und trotzdem steht ein voller Nenner da.
    u.zum_fragebogen_neu();
    let a = anzeige(s);
    let offen = fragen(d, &u.fall()).len();
    pp!("P.nenner_ist_die_zahl_der_noch_offenen_fragen", offen > 1, "diese Scheibe stellt nur eine Frage: der Test misst nichts");
    pp!("P.nenner_ist_die_zahl_der_noch_offenen_fragen", a.fest == 0, "der Zaehler steht nicht auf 0: {:?}", a.mitte);
    pp!("P.nenner_ist_die_zahl_der_noch_offenen_fragen", a.gesamt == offen as i64, "Nenner {} != offene Fragen {offen}", a.gesamt);
    pp!("P.nenner_ist_die_zahl_der_noch_offenen_fragen", a.max == offen as f64 && a.wert == 0.0, "die Leiste laeuft gegen max={}, value={}", a.max, a.wert);
    pp!("P.nenner_ist_die_zahl_der_noch_offenen_fragen", a.text.contains(&format!("0 von {offen} Fragen beantwortet")), "Text: {:?}", a.text);

    // Eine Antwort, die weder einen Block auf- noch zumacht: der Zaehler steigt um 1, der Nenner bleibt STEHEN.
    let mut rueckwaerts = basis.clone();
    rueckwaerts.reverse();
    let t = sonde(d, &mut zaehler, &rueckwaerts, |v, n| n + 1 == v);
    pp!("P.nenner_waechst_nicht_mit_dem_zaehler_mit", t.is_some(), "keine Frage gefunden, deren Antwort genau sich selbst aus der Queue nimmt: der Test misst nichts");
    let (feld, wert, _, _) = t.unwrap_or_default();
    u.zum_fragebogen_neu();
    let vor = anzeige(s);
    schreibe(d, &u.fall(), &feld, &wert, None);
    refresh(s);
    let nach = anzeige(s);
    pp!("P.nenner_waechst_nicht_mit_dem_zaehler_mit", nach.fest == vor.fest + 1, "der Zaehler ist nicht gestiegen: {:?} -> {:?}", vor.mitte, nach.mitte);
    pp!("P.nenner_waechst_nicht_mit_dem_zaehler_mit", nach.gesamt == vor.gesamt, "der Nenner ist mitgewachsen: {:?} -> {:?}", vor.mitte, nach.mitte);
    pp!("P.nenner_waechst_nicht_mit_dem_zaehler_mit", nach.gesamt > nach.fest && nach.anteil < 1.0, "{} liest sich als „fertig“ (anteil {})", nach.mitte, nach.anteil);
    pp!("P.nenner_waechst_nicht_mit_dem_zaehler_mit", !nach.text.contains("entfallen") && !nach.text.contains("dazugekommen"), "die Gesamtzahl blieb, die Anzeige behauptet eine Aenderung: {:?}", nach.text);

    // Eine Antwort, die Bloecke abschaltet, senkt die Gesamtzahl, und die Anzeige sagt es.
    let t = sonde(d, &mut zaehler, &basis, |v, n| n + 1 < v);
    pp!("P.eine_antwort_die_bloecke_abschaltet_senkt_die_gesamtzahl_und_sagt_es", t.is_some(), "kein Feld gefunden, dessen Antwort mehr als sich selbst aus der Queue nimmt");
    let (feld, wert, sonde_vor, sonde_nach) = t.unwrap_or_default();
    u.zum_fragebogen_neu();
    let vor = anzeige(s);
    schreibe(d, &u.fall(), &feld, &wert, None);
    refresh(s);
    let nach = anzeige(s);
    pp!("P.eine_antwort_die_bloecke_abschaltet_senkt_die_gesamtzahl_und_sagt_es", nach.gesamt < vor.gesamt, "{feld}={wert} nimmt laut Sonde {} Fragen aus der Queue, die Anzeige bleibt bei {:?} -> {:?}", sonde_vor - sonde_nach, vor.mitte, nach.mitte);
    let weg = vor.gesamt - nach.gesamt;
    pp!("P.eine_antwort_die_bloecke_abschaltet_senkt_die_gesamtzahl_und_sagt_es", nach.text.contains(&format!("{weg} Fragen entfallen")), "die Gesamtzahl fiel um {weg}, die Anzeige sagt nichts dazu: {:?}", nach.text);
    pp!("P.eine_antwort_die_bloecke_abschaltet_senkt_die_gesamtzahl_und_sagt_es", nach.max == nach.gesamt as f64, "die Leiste laeuft weiter gegen den alten Nenner (max={}, angezeigt {})", nach.max, nach.gesamt);

    // Die Gegenrichtung: nimmt der Nutzer sein „nein“ zurueck, kommen die Fragen zurueck, und der Balken springt zurueck.
    let q = basis.iter().find(|q| q["feld_id"] == json!(feld));
    let gegenwerte: Vec<Value> = q.map(kandidatenwerte).unwrap_or_default().into_iter().filter(|w| *w != wert).collect();
    let k = sonde_korrektur(d, &mut zaehler, &feld, &wert, &gegenwerte);
    pp!("P.eine_korrektur_die_bloecke_aufmacht_hebt_die_gesamtzahl_und_sagt_es", k.is_some(), "keine Korrektur von {feld} bringt Fragen zurueck in die Queue: der Test misst nichts");
    let (gegenwert, erwartet_dazu) = k.unwrap_or_default();
    u.zum_fragebogen_neu();
    let fall = u.fall();
    schreibe(d, &fall, &feld, &wert, None);
    refresh(s);
    let eng = anzeige(s);
    let alt = d.get(&format!("/fall/{fall}/feld/{feld}/warum")).1["justification"]["event_id"].as_str().unwrap_or_default().to_owned();
    schreibe(d, &fall, &feld, &gegenwert, Some(&alt));
    refresh(s);
    let weit = anzeige(s);
    pp!("P.eine_korrektur_die_bloecke_aufmacht_hebt_die_gesamtzahl_und_sagt_es", weit.gesamt > eng.gesamt, "die Korrektur bringt laut Sonde {erwartet_dazu} Fragen zurueck, die Anzeige bleibt bei {:?} -> {:?}", eng.mitte, weit.mitte);
    let dazu = weit.gesamt - eng.gesamt;
    pp!("P.eine_korrektur_die_bloecke_aufmacht_hebt_die_gesamtzahl_und_sagt_es", weit.text.contains(&format!("{dazu} Fragen dazugekommen")), "die Gesamtzahl stieg um {dazu}, die Anzeige sagt nichts dazu: {:?}", weit.text);
    pp!("P.eine_korrektur_die_bloecke_aufmacht_hebt_die_gesamtzahl_und_sagt_es", weit.max == weit.gesamt as f64 && weit.wert == weit.fest as f64, "Leiste und Zahl gehen auseinander: max={}, wert={}, angezeigt {}", weit.max, weit.wert, weit.mitte);
    pp!("P.eine_korrektur_die_bloecke_aufmacht_hebt_die_gesamtzahl_und_sagt_es", weit.anteil < eng.anteil, "der Ring ist nicht zurueckgegangen, obwohl Fragen dazugekommen sind");

    // Ein vorlaeufiger Wert zaehlt genau einmal: ein Vorschlag steht in `stand.felder` UND in der Queue.
    u.zum_fragebogen_neu();
    let fall = u.fall();
    let vor = anzeige(s);
    let quelle = d.fall("gesamt", "nenner-quelle");
    let cent: Vec<Value> = fragen(d, &quelle).into_iter().filter(|q| q["typ"] == "cent" && !q["beispielwert"].is_null()).take(5).collect();
    pp!("P.ein_vorlaeufiger_wert_zaehlt_genau_einmal", !cent.is_empty(), "kein Cent-Feld mit Beispielwert fuer die Vorjahr-Uebernahme");
    for q in &cent {
        schreibe(d, &quelle, q["feld_id"].as_str().unwrap_or_default(), &q["beispielwert"], None);
    }
    let (st, uebernahme) = d.post(&format!("/fall/{fall}/vorjahr"), &json!({"vorjahr_fall_id": quelle}));
    let n = uebernahme["uebernommen"].as_i64().unwrap_or(0);
    pp!("P.ein_vorlaeufiger_wert_zaehlt_genau_einmal", st == 200 && n > 0, "die Vorjahr-Uebernahme hat kein Feld uebernommen ({st} {uebernahme}): der Test misst nichts");
    refresh(s);
    let nach = anzeige(s);
    let zustaende = s.ausdruck("Object.values(STAND.felder).map(f => f.zustand)");
    let vorlaeufig = zustaende.as_array().map_or(0, |a| a.iter().filter(|z| **z == json!("vorlaeufig")).count()) as i64;
    pp!("P.ein_vorlaeufiger_wert_zaehlt_genau_einmal", vorlaeufig == n, "erwartet {n} vorlaeufige Felder im Stand, gefunden {zustaende}");
    pp!("P.ein_vorlaeufiger_wert_zaehlt_genau_einmal", nach.fest == vor.fest, "ein Vorschlag ist keine Bestaetigung, der Zaehler stieg trotzdem: {:?} -> {:?}", vor.mitte, nach.mitte);
    pp!("P.ein_vorlaeufiger_wert_zaehlt_genau_einmal", nach.gesamt == vor.gesamt, "die Gesamtzahl stieg um {}, obwohl {n} Vorschlaege nur bestehende Fragen vorbelegen: {:?} -> {:?}", nach.gesamt - vor.gesamt, vor.mitte, nach.mitte);
    pp!("P.ein_vorlaeufiger_wert_zaehlt_genau_einmal", !nach.text.contains("dazugekommen"), "die Anzeige meldet neue Fragen, obwohl nur vorbelegt wurde: {:?}", nach.text);

    // Ein Netzfehler beim Holen der Fragen meldet keinen Fortschritt: die Zahl ist UNBEKANNT, nicht null.
    u.zum_fragebogen_neu();
    let vor = anzeige(s);
    pp!("P.ein_netzfehler_meldet_keinen_fortschritt", vor.gesamt > 1, "Vorbedingung: ein Nenner groesser 1: {:?}", vor.mitte);
    s.skript(
        "const echt = window.fetch; window.fetch = (u, o) => String(u).includes('/fragen') ? Promise.reject(new TypeError('Netz weg')) : echt(u, o);",
        &[],
    );
    refresh(s);
    let nach = anzeige(s);
    pp!("P.ein_netzfehler_meldet_keinen_fortschritt", nach.gesamt == vor.gesamt, "nach dem Netzfehler steht {:?} statt {:?}: eine gescheiterte Abfrage darf keine Gesamtzahl setzen", nach.mitte, vor.mitte);
    pp!("P.ein_netzfehler_meldet_keinen_fortschritt", nach.anteil < 1.0, "der Ring ist durch einen Netzfehler vollgelaufen (--anteil={})", nach.anteil);
    pp!("P.ein_netzfehler_meldet_keinen_fortschritt", nach.max == vor.max, "die Leiste laeuft jetzt gegen max={}", nach.max);
    pp!("P.ein_netzfehler_meldet_keinen_fortschritt", s.ausdruck("!document.getElementById('netz-banner').hidden") == json!(true), "der Nutzer erfaehrt nichts vom Netzfehler");
}

// ---- test_ui_fortschritt_und_panel, Teil 2: das Panel -----------------------------------------------------------------

#[test]
fn darstellung_panel() {
    let _sperre = seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let (s, d) = (&u.s, &u.dienst);

    // Breit: der Berater liegt rechts NEBEN der Frage-Karte, ueberlappt sie nicht, die Belegt-Liste bleibt links.
    u.zum_fragebogen_neu();
    let fall = u.fall();
    let frage = kasten(s, "#wegpunkt");
    let berater = kasten(s, "#berater");
    pp!("P.panel_liegt_auf_breitem_schirm_rechts_neben_dem_inhalt", frage.is_some() && berater.is_some(), "Frage-Karte oder Berater ohne Box: {frage:?} {berater:?}");
    let q = fragen(d, &fall).into_iter().find(|q| !q["beispielwert"].is_null());
    pp!("P.panel_liegt_auf_breitem_schirm_rechts_neben_dem_inhalt", q.is_some(), "keine Frage mit Beispielwert");
    let q = q.unwrap_or_default();
    schreibe(d, &fall, q["feld_id"].as_str().unwrap_or_default(), &q["beispielwert"], None);
    refresh(s);
    s.warte_sichtbar("#belegt-liste li", 8);
    let ((fx, fy, fb, fh), (bx, by, bb, _)) = (frage.unwrap_or_default(), berater.unwrap_or_default());
    pp!("P.panel_liegt_auf_breitem_schirm_rechts_neben_dem_inhalt", bx >= fx + fb - 1.0, "der Berater beginnt bei x={bx}, die Frage-Karte endet bei {}: er ueberlappt sie", fx + fb);
    pp!("P.panel_liegt_auf_breitem_schirm_rechts_neben_dem_inhalt", by < fy + fh, "der Berater beginnt bei y={by}, unterhalb der Frage-Karte (endet bei {}): er haengt darunter", fy + fh);
    pp!("P.panel_liegt_auf_breitem_schirm_rechts_neben_dem_inhalt", bb >= 280.0, "Panel zu schmal zum Lesen: {bb}");
    pp!("P.panel_liegt_auf_breitem_schirm_rechts_neben_dem_inhalt", scroll_breite(s) <= 1280, "seitwaerts Scrollen");
    let belegt = kasten(s, "#belegt-liste");
    pp!("P.panel_liegt_auf_breitem_schirm_rechts_neben_dem_inhalt", belegt.is_some_and(|(x, _, _, _)| x < bx), "die Belegt-Liste ist in die Berater-Spalte gerutscht: {belegt:?}");

    // Das Panel bleibt beim Scrollen stehen (sticky): sonst waere die KI nicht „immer da“.
    s.sichtflaeche(1280, 480);
    u.zum_fragebogen_neu();
    let fall = u.fall();
    for q in fragen(d, &fall).into_iter().filter(|q| !q["beispielwert"].is_null()).take(12) {
        schreibe(d, &fall, q["feld_id"].as_str().unwrap_or_default(), &q["beispielwert"], None);
    }
    refresh(s);
    let hoehe = s.ausdruck("document.documentElement.scrollHeight").as_i64().unwrap_or(0);
    pp!("P.panel_bleibt_beim_scrollen_stehen", hoehe > 780, "die Seite ist mit {hoehe} px gar nicht scrollbar: der Test misst nichts");
    let oben = kasten(s, "#berater").map_or(f64::NAN, |k| k.1);
    s.skript("window.scrollTo(0, 300);", &[]);
    let unten = kasten(s, "#berater").map_or(f64::NAN, |k| k.1);
    pp!("P.panel_bleibt_beim_scrollen_stehen", unten > oben - 300.0 + 1.0, "der Berater ist beim Scrollen um {} px mitgewandert: er klebt nicht", oben - unten);
    pp!("P.panel_bleibt_beim_scrollen_stehen", unten >= 0.0, "der Berater ist aus dem Bild gescrollt (y={unten})");

    // Schmal: das Panel rutscht UNTER den Inhalt, nicht abgeschnitten.
    s.sichtflaeche(360, 640);
    u.zum_fragebogen_neu();
    let frage = kasten(s, "#wegpunkt").unwrap_or_default();
    let berater = kasten(s, "#berater").unwrap_or_default();
    pp!("P.panel_rutscht_auf_schmalem_schirm_unter_den_inhalt", berater.1 >= frage.1 + frage.3 - 1.0, "der Berater (y={}) liegt nicht unter der Frage-Karte (endet bei {})", berater.1, frage.1 + frage.3);
    pp!("P.panel_rutscht_auf_schmalem_schirm_unter_den_inhalt", berater.0 >= 0.0 && berater.0 + berater.2 <= 360.0, "der Berater ragt aus dem Bild: x={}, Breite={}", berater.0, berater.2);
    pp!("P.panel_rutscht_auf_schmalem_schirm_unter_den_inhalt", scroll_breite(s) <= 360, "seitwaerts Scrollen bei 360 px");
    pp!("P.panel_rutscht_auf_schmalem_schirm_unter_den_inhalt", kasten(s, "#chat-text").is_some_and(|k| k.3 >= 44.0), "Tap-Ziel Eingabefeld unter 44 px");

    // Die Chat-Sperre greift im zweispaltigen Bild: `inert` ueber feste IDs, und der Berater haengt unter keinem gesperrten Vorfahren.
    s.sichtflaeche(1280, 900);
    u.zum_fragebogen_neu();
    s.skript("chatSperren(true);", &[]);
    let gesperrt = s.ausdruck(
        "({wegpunkt: document.getElementById('wegpunkt').hasAttribute('inert'), verstanden: document.getElementById('verstanden').hasAttribute('inert'), \
          belegt: document.getElementById('belegt-liste').hasAttribute('inert'), importbox: document.querySelector('.vorjahr-box').hasAttribute('inert'), \
          chattext: document.getElementById('chat-text').disabled})",
    );
    pp!("P.die_chat_sperre_greift_auch_im_zweispaltigen_bild", gesperrt.as_object().is_some_and(|o| o.values().all(|v| *v == json!(true))), "die Sperre greift ins Leere: {gesperrt}");
    pp!("P.die_chat_sperre_greift_auch_im_zweispaltigen_bild", s.ausdruck("!document.getElementById('berater').closest('[inert]')") == json!(true), "der Berater haengt unter einem gesperrten Bereich: der KI-Aufruf wuerde sein eigenes Panel mitsperren");
    s.skript("chatSperren(false);", &[]);
    pp!("P.die_chat_sperre_greift_auch_im_zweispaltigen_bild", s.ausdruck("!document.querySelector('#wegpunkt[inert], #verstanden[inert]')") == json!(true), "nicht freigegeben");
}

// ---- test_ui_a11y: die Grundlagen der Barrierefreiheit ----------------------------------------------------------------

/// Die Regeln, die axe als kritisch oder ernst fuehrt und die ohne Bibliothek messbar sind. Liefert die Verstoesse als
/// Liste von Texten (leer = keiner). Farbkontrast nur ueber Text mit einfarbigem Grund (kein Verlauf, kein Bild).
const A11Y_JS: &str = "(() => { \
  const sichtbar = e => { const r = e.getBoundingClientRect(); const s = getComputedStyle(e); \
    return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none' && !e.closest('[hidden],[inert],[aria-hidden=\"true\"]'); }; \
  const name = e => { const l = e.getAttribute('aria-labelledby'); \
    return (e.getAttribute('aria-label') || '').trim() || (l ? l.split(/\\s+/).map(i => (document.getElementById(i) || {}).textContent || '').join(' ').trim() : '') \
      || (e.textContent || '').trim() || (e.getAttribute('title') || '').trim(); }; \
  const v = []; \
  if (!document.documentElement.lang) v.push('html-has-lang'); \
  if (!document.title.trim()) v.push('document-title'); \
  for (const b of document.querySelectorAll('button, [role=button]')) if (sichtbar(b) && !name(b)) v.push('button-name: ' + (b.id || b.className)); \
  for (const a of document.querySelectorAll('a[href]')) if (sichtbar(a) && !name(a)) v.push('link-name: ' + (a.id || a.href)); \
  for (const i of document.querySelectorAll('input:not([type=hidden]):not([type=submit]):not([type=button]), select, textarea')) { \
    if (!sichtbar(i)) continue; \
    const l = (i.id && document.querySelector('label[for=\"' + i.id + '\"]')) || i.closest('label'); \
    if (!(l && l.textContent.trim()) && !i.getAttribute('aria-label') && !i.getAttribute('aria-labelledby') && !i.title) v.push('label: ' + (i.id || i.name || i.type)); } \
  for (const m of document.querySelectorAll('img')) if (sichtbar(m) && !m.hasAttribute('alt')) v.push('image-alt: ' + m.src); \
  const rgba = s => { const m = s.match(/rgba?\\(([^)]+)\\)/); if (!m) return null; const p = m[1].split(/[,\\s\\/]+/).filter(Boolean).map(Number); return {r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1}; }; \
  const misch = (f, h) => ({r: f.r * f.a + h.r * (1 - f.a), g: f.g * f.a + h.g * (1 - f.a), b: f.b * f.a + h.b * (1 - f.a), a: 1}); \
  const grund = e => { const ebenen = []; for (let x = e; x; x = x.parentElement) { const s = getComputedStyle(x); \
      if (s.backgroundImage !== 'none') return null; const c = rgba(s.backgroundColor); if (c && c.a > 0) { ebenen.push(c); if (c.a >= 1) break; } } \
    let g = {r: 255, g: 255, b: 255, a: 1}; for (const c of ebenen.reverse()) g = misch(c, g); return g; }; \
  const lum = c => { const f = u => { u /= 255; return u <= 0.03928 ? u / 12.92 : Math.pow((u + 0.055) / 1.055, 2.4); }; return 0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b); }; \
  let geprueft = 0; \
  for (const e of document.querySelectorAll('body *')) { \
    if (!sichtbar(e) || e.disabled) continue; \
    if (![...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim())) continue; \
    const s = getComputedStyle(e); const fg = rgba(s.color); const bg = grund(e); if (!fg || !bg) continue; \
    const f = misch(fg, bg); const l1 = lum(f), l2 = lum(bg); const k = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05); geprueft++; \
    const gross = parseFloat(s.fontSize) >= 24 || (parseFloat(s.fontSize) >= 18.66 && parseInt(s.fontWeight) >= 700); \
    if (k < (gross ? 3 : 4.5)) v.push('color-contrast: ' + (e.id || String(e.className) || e.tagName) + ' ' + k.toFixed(2) + ' (' + s.color + ' auf ' + s.backgroundColor + ')'); } \
  return {verstoesse: v, kontrast_geprueft: geprueft}; })()";

#[test]
fn darstellung_a11y() {
    let _sperre = seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 640);
    let s = &u.s;

    u.oeffne_start();
    let r = s.ausdruck(A11Y_JS);
    pp!("Y.a11y_startscreen", r["kontrast_geprueft"].as_i64().unwrap_or(0) >= 5, "der Kontrast wurde fuer {} Elemente geprueft: die Messung ist leer", r["kontrast_geprueft"]);
    pp!("Y.a11y_startscreen", r["verstoesse"] == json!([]), "Verstoesse auf dem Start-Screen: {}", r["verstoesse"]);

    u.zum_fragebogen_neu();
    let r = s.ausdruck(A11Y_JS);
    pp!("Y.a11y_wegpunkt", r["kontrast_geprueft"].as_i64().unwrap_or(0) >= 5, "der Kontrast wurde fuer {} Elemente geprueft: die Messung ist leer", r["kontrast_geprueft"]);
    pp!("Y.a11y_wegpunkt", r["verstoesse"] == json!([]), "Verstoesse im Wegpunkt-Fluss: {}", r["verstoesse"]);
}
