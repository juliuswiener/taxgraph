//! Gemeinsame Wege durch die Oberflaeche (Gegenstueck zu `tests/ui_hilfen.py`) und die Umgebung eines Flusses:
//! ein Dienst auf Wegwerf-Daten plus eine Browser-Sitzung.

use serde_json::{json, Value};

use crate::browser::Sitzung;
use crate::dienst::{Dienst, Start};

pub struct Umgebung {
    pub dienst: Dienst,
    pub s: Sitzung,
}

impl Umgebung {
    /// `None`, wenn kein Browser da ist und `TAXGRAPH_OHNE_BROWSER=1` das Ueberspringen erlaubt.
    pub fn neu(opt: &Start, breite: u32, hoehe: u32) -> Option<Self> {
        let s = Sitzung::neu(breite, hoehe)?;
        let dienst = Dienst::starte(opt);
        Some(Self { dienst, s })
    }

    /// Die Startseite oeffnen und warten, bis `initAuth()` entschieden hat: Start-Bildschirm oder Anmeldemaske.
    pub fn oeffne_start(&self) {
        oeffne_startseite(&self.s, &self.dienst.basis);
    }

    /// Von der Startseite bis zur Wegwahl: Kachel „Gesamt“ (wie `.click()` im Python-Test), dann Weg `fragebogen` oder `ki`.
    pub fn fall_beginnen(&self, weg: &str) {
        self.s.skript("document.querySelector(\".kachel[data-scheibe='gesamt']\").click();", &[]);
        self.s.warte_sichtbar("#wegwahl", 10);
        self.s.skript(&format!("document.getElementById('weg-{weg}').click();"), &[]);
    }

    /// Startseite, Kachel „Gesamt“, Weg „Fragebogen“, Ankreuzliste weggeraeumt (Gegenstueck zu `zum_fragebogen(page)`).
    pub fn zum_fragebogen_neu(&self) {
        self.oeffne_start();
        self.fall_beginnen("fragebogen");
        zum_fragebogen(&self.s, None);
    }

    /// Die Fall-ID der Seite (`FALL`).
    pub fn fall(&self) -> String {
        self.s.ausdruck("FALL").as_str().unwrap_or_default().to_owned()
    }

    /// Fehler der Browser-Konsole seit dem letzten Aufruf; die Sonde `auth-sonde-taxgraph` ist beabsichtigt (404/401).
    /// `favicon.ico` fehlt dem Dienst; Playwright-`pageerror` (die Messlatte der Python-Tests) sieht nur Skriptfehler.
    pub fn konsolenfehler(&self) -> Vec<String> {
        self.s
            .konsole()
            .into_iter()
            .filter(|(stufe, text)| stufe == "SEVERE" && !text.contains("auth-sonde-taxgraph") && !text.contains("/favicon.ico"))
            .map(|(_, text)| text)
            .collect()
    }

    /// Die offenen Fragen des Falls (`GET /fall/{id}/fragen`), nur die Feld-IDs.
    pub fn offene(&self, fall: &str) -> Vec<String> {
        let (status, antwort) = self.dienst.get(&format!("/fall/{fall}/fragen"));
        assert_eq!(status, 200, "fragen: {antwort}");
        antwort["fragen"].as_array().unwrap().iter().filter_map(|q| q["feld_id"].as_str().map(str::to_owned)).collect()
    }

    /// Der Stand (`GET /fall/{id}/stand`), Feld `felder`.
    pub fn stand_felder(&self, fall: &str) -> Value {
        let (status, antwort) = self.dienst.get(&format!("/fall/{fall}/stand"));
        assert_eq!(status, 200, "stand: {antwort}");
        antwort["felder"].clone()
    }

    /// Die Frage direkt als aktuelle Frage rendern (Muster der Python-Tests): Quelle ist `/fragen` der Seite.
    /// `true`, wenn das Feld danach `AKTUELL` ist.
    pub fn rendere_frage(&self, feld: &str) -> bool {
        self.s.auswerten(
            "async (fid) => { const r = await jget(`/fall/${FALL}/fragen`); \
             const q = r.body.fragen.find(f => f.feld_id === fid); if (q) { AKTUELL = q; zeigeFrage(q, STAND); } \
             return !!(AKTUELL && AKTUELL.feld_id === fid); }",
            &[json!(feld)],
        ) == json!(true)
    }

    pub fn aktuelle_frage(&self) -> Value {
        self.s.ausdruck("AKTUELL && AKTUELL.feld_id")
    }
}

/// Die Startseite des Dienstes unter `basis` oeffnen, bis Start-Bildschirm oder Anmeldemaske steht.
pub fn oeffne_startseite(s: &Sitzung, basis: &str) {
    s.oeffne(basis);
    s.warte_bis(
        "(() => { const sicht = (id) => { const e = document.getElementById(id); return !!e && !e.hidden; }; \
          return sicht('start') || sicht('login'); })()",
        10,
        "Start-Bildschirm oder Anmeldemaske",
    );
}

/// Von der Wegwahl bis zur ersten echten Frage. `ankreuzen = None`: die Ankreuzliste wird weggeraeumt, OHNE zu
/// antworten (die zehn Screening-Felder bleiben offen). `Some(felder)`: die Liste wird bedient (genannte Felder
/// angekreuzt, die uebrigen verneint, „Weiter“).
pub fn zum_fragebogen(s: &Sitzung, ankreuzen: Option<&[&str]>) {
    let mut da = s.sichtbar("#screening");
    if !da {
        // Sie kann noch im Kommen sein (zeigeScreening holt erst /fragen).
        da = s
            .try_warte_bis("(() => { const e = document.getElementById('screening'); return !!e && !e.hidden; })()", 8)
            .is_some();
    }
    match (da, ankreuzen) {
        (true, None) => {
            s.skript(
                "SCREENING_OFFEN = false; document.getElementById('screening').hidden = true; \
                 zeigeScreening = async () => false; refresh();",
                &[],
            );
        }
        (true, Some(felder)) => {
            for feld in felder {
                let sel = format!("#screening-liste .sc-box[data-feld=\"{feld}\"]");
                assert!(s.finde(&sel).is_some(), "{feld} steht nicht in der Ankreuzliste");
                s.skript(
                    "const b = document.querySelector(arguments[0]); b.checked = true; \
                     b.dispatchEvent(new Event('change', {bubbles: true}));",
                    &[json!(sel)],
                );
            }
            s.klick("#screening-weiter");
            s.warte_unsichtbar("#screening", 8);
        }
        (false, _) => {}
    }
    s.warte_sichtbar("#wegpunkt", 8);
}
