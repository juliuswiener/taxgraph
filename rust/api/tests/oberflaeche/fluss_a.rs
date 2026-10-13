//! Fluss A: Anmelden, Abmelden, 401 und die Richtlinie (CSP).
//!
//! Gegenstueck zu `tests/test_ui_login.py` (6 Funktionen) und dem Browser-Teil von `tests/test_idor_und_csp.py`
//! (`die_oberflaeche_laeuft_unter_der_csp`). Die uebrigen Funktionen von `test_idor_und_csp.py` ruhen auf bestehenden
//! Rust-Tests oder auf `csp_dateien` (siehe `ZUORDNUNG.tsv`). Geld: nein. Daten: Konten und Token.

use serde_json::json;

use crate::dienst::Start;
use crate::seite::{zum_fragebogen, Umgebung};

const NUTZER: &str = "testuser1";
const PASSWORT: &str = "geheim1234";

fn registrieren_und_anmelden(u: &Umgebung) {
    let s = &u.s;
    s.warte_sichtbar("#login", 5);
    s.klick("#login-umschalten");
    s.fuelle("#login-user", NUTZER);
    s.fuelle("#login-pw", PASSWORT);
    s.klick("#login-go");
    s.warte_sichtbar("#start", 5);
}

fn anmelden(u: &Umgebung) {
    let s = &u.s;
    s.warte_sichtbar("#login", 5);
    s.klick("#login-umschalten");
    s.fuelle("#login-user", NUTZER);
    s.fuelle("#login-pw", PASSWORT);
    s.klick("#login-go");
    s.warte_sichtbar("#start", 5);
}

#[test]
fn fluss_a_anmelden_und_abmelden() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start { auth: true, ..Start::default() }, 360, 780);
    let s = &u.s;

    // Auth ist Pflicht: die Maske steht VOR dem Start-Bildschirm, kein technischer Fehlerbanner.
    s.oeffne(&u.dienst.basis);
    s.warte_sichtbar("#login", 5);
    pp!("A.anmeldemaske_erscheint_wenn_auth_pflicht_ist", !s.sichtbar("#start"), "Start-Bildschirm sichtbar, obwohl Auth Pflicht ist");
    pp!(
        "A.anmeldemaske_erscheint_wenn_auth_pflicht_ist",
        s.ausdruck("document.getElementById('netz-banner').hidden") == json!(true),
        "die Sonde loest einen technischen Fehlerbanner aus"
    );

    // Registrieren und anmelden fuehrt in die Anwendung; das Token ueberlebt in der Sitzung einen Neuladen.
    registrieren_und_anmelden(&u);
    let token = s.ausdruck("TOKEN");
    pp!("A.registrierung_und_anmeldung_fuehrt_in_die_anwendung", token.as_str().is_some_and(|t| !t.is_empty()), "kein TOKEN nach dem Login");
    pp!(
        "A.registrierung_und_anmeldung_fuehrt_in_die_anwendung",
        s.ausdruck("sessionStorage.getItem('taxgraph_token')") == token,
        "das Token steht nicht in sessionStorage"
    );
    pp!("A.registrierung_und_anmeldung_fuehrt_in_die_anwendung", !s.sichtbar("#login"), "Anmeldemaske bleibt sichtbar");
    pp!("A.registrierung_und_anmeldung_fuehrt_in_die_anwendung", s.sichtbar("#konto-leiste"), "Konto-Leiste fehlt nach dem Login");

    // Jeder Aufruf nach dem Login traegt `Authorization: Bearer …` und wird akzeptiert.
    s.skript(
        "window.__anfragen = []; const f = window.fetch; \
         window.fetch = (u, o) => { window.__anfragen.push({url: String(u), auth: o && o.headers && o.headers.Authorization}); return f(u, o); };",
        &[],
    );
    u.fall_beginnen("fragebogen");
    s.warte_bis("window.__anfragen.some(a => a.url.includes('/fall/') && a.url.endsWith('/stand'))", 10, "GET /fall/<id>/stand");
    let auth = s.ausdruck("(window.__anfragen.find(a => a.url.includes('/fall/') && a.url.endsWith('/stand')) || {}).auth");
    pp!(
        "A.request_nach_login_traegt_authorization_header",
        auth.as_str().is_some_and(|a| a.starts_with("Bearer ")),
        "kein Authorization-Kopf am Aufruf nach dem Login: {auth}"
    );
    zum_fragebogen(s, None); // der Kopf wird auch akzeptiert: /fragen kommt durch, die Ankreuzliste erscheint

    // Abmelden raeumt Token, sessionStorage und Konto-Leiste und zeigt die Maske.
    s.klick("#logout-btn");
    s.warte_sichtbar("#login", 5);
    pp!("A.abmelden_entfernt_das_token", s.ausdruck("TOKEN").is_null(), "TOKEN-Variable nach dem Abmelden nicht geleert");
    pp!(
        "A.abmelden_entfernt_das_token",
        s.ausdruck("sessionStorage.getItem('taxgraph_token')").is_null(),
        "sessionStorage traegt nach dem Abmelden noch das Token"
    );
    pp!("A.abmelden_entfernt_das_token", !s.sichtbar("#konto-leiste"), "Konto-Leiste bleibt nach dem Abmelden sichtbar");

    // Ein abgelaufenes Token zeigt die Maske, nicht den Netz-Fehlerbanner.
    anmelden(&u);
    u.fall_beginnen("fragebogen");
    zum_fragebogen(s, None);
    s.skript("versteckeNetzFehler(); TOKEN = 'kaputt.und.abgelaufen'; refresh();", &[]);
    s.warte_sichtbar("#login", 5);
    pp!(
        "A.ungueltiges_token_zeigt_maske_nicht_fehlerbanner",
        s.ausdruck("document.getElementById('netz-banner').hidden") == json!(true),
        "ein 401 zeigt den technischen Fehlerbanner statt der Anmeldemaske"
    );
}

#[test]
fn fluss_a_einzelnutzer_und_richtlinie() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 780);
    let s = &u.s;

    // Einzelnutzer-Modus: die Maske erscheint nie, es gibt nie ein Token; die Richtlinie blockiert die Oberflaeche nicht.
    s.oeffne(&u.dienst.basis);
    s.warte_sichtbar("#start", 10);
    pp!("A.einzelnutzer_modus_zeigt_nie_die_anmeldemaske", !s.sichtbar("#login"), "Anmeldemaske im Einzelnutzer-Modus");
    pp!("A.einzelnutzer_modus_zeigt_nie_die_anmeldemaske", s.sichtbar("#start"), "Start-Bildschirm bleibt versteckt");
    pp!("A.einzelnutzer_modus_zeigt_nie_die_anmeldemaske", !s.sichtbar("#konto-leiste"), "Konto-Leiste ohne Login");
    u.fall_beginnen("fragebogen");
    zum_fragebogen(s, None);
    pp!("A.einzelnutzer_modus_zeigt_nie_die_anmeldemaske", !s.sichtbar("#login"), "Anmeldemaske mitten im Einzelnutzer-Fluss");
    pp!("A.einzelnutzer_modus_zeigt_nie_die_anmeldemaske", s.ausdruck("TOKEN").is_null(), "im Einzelnutzer-Modus existiert ein Token");

    let verletzungen: Vec<String> =
        s.konsole().into_iter().filter(|(_, t)| t.contains("Content Security Policy")).map(|(_, t)| t).collect();
    pp!(
        "A.die_oberflaeche_laeuft_unter_der_csp",
        verletzungen.is_empty(),
        "die eigene Richtlinie blockiert die eigene Oberflaeche: {verletzungen:?}"
    );
    pp!("A.die_oberflaeche_laeuft_unter_der_csp", s.sichtbar("#wegpunkt"), "der Wegpunkt wurde unter der Richtlinie nicht sichtbar");
    pp!(
        "A.die_oberflaeche_laeuft_unter_der_csp",
        !s.textinhalt("#frage").trim().is_empty(),
        "keine Frage gerendert: die Seite ist unter der Richtlinie leer"
    );
}
