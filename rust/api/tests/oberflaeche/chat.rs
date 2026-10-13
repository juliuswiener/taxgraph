//! Gemeinsame Hilfen der KI-Fluesse (Fluss H): `/chat` im Browser spielen (Ersatz fuer `page.route("**/chat")`),
//! absenden, die Rueckfragen-Seite lesen. Der Dienst sieht den Aufruf nie: es fliesst kein Schluessel und kein Cent,
//! und der Test haelt die Antwort so lange zurueck, wie er will (kein Schlafen, keine Zeitabhaengigkeit).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::browser::Sitzung;

/// Ein lokaler Ersatz fuer das Sprachmodell (`LLM_API_BASE` zeigt auf `127.0.0.1`; Modell und Schluessel sind
/// Platzhalter). Er erkennt die Stufe am Schemanamen der Anfrage (`aussagen`, `zuordnung`, `dialog`) und antwortet immer
/// gleich; die Antwort der Stufe `dialog` laesst sich zwischen zwei Gespraechen tauschen. Er sendet nichts ins Netz.
/// Wie `chat_llm_attrappe_hermetisch.rs`; der Thread laeuft bis zum Ende des Prozesses.
pub struct ModellErsatz {
    pub basis: String,
    dialog: Arc<Mutex<Value>>,
    /// Wie viele Anfragen den Ersatz erreicht haben (jede Stufe zaehlt einzeln): „ohne Modell“ heisst 0.
    aufrufe: Arc<AtomicUsize>,
}

impl ModellErsatz {
    pub fn starte(dialog: Value) -> Self {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let basis = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
        let dialog = Arc::new(Mutex::new(dialog));
        let geteilt = Arc::clone(&dialog);
        let aufrufe = Arc::new(AtomicUsize::new(0));
        let zaehler = Arc::clone(&aufrufe);
        std::thread::spawn(move || {
            for s in l.incoming() {
                let Ok(mut s) = s else { continue };
                zaehler.fetch_add(1, Ordering::SeqCst);
                let mut puffer = Vec::new();
                let mut b = [0u8; 8192];
                let (kopf_ende, laenge) = loop {
                    let n = s.read(&mut b).unwrap_or(0);
                    if n == 0 {
                        break (0, 0);
                    }
                    puffer.extend_from_slice(&b[..n]);
                    let text = String::from_utf8_lossy(&puffer).into_owned();
                    if let Some(i) = text.find("\r\n\r\n") {
                        let laenge = text[..i]
                            .lines()
                            .find_map(|z| z.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                            .unwrap_or(0);
                        if puffer.len() >= i + 4 + laenge {
                            break (i + 4, laenge);
                        }
                    }
                };
                let anfrage: Value = serde_json::from_slice(&puffer[kopf_ende..kopf_ende + laenge]).unwrap_or(Value::Null);
                let inhalt = match anfrage["response_format"]["json_schema"]["name"].as_str().unwrap_or("?") {
                    "aussagen" => json!({"aussagen": [{"text": "Der Nutzer nennt Angaben zu seinem Jahr", "beleg": "verheiratet"}]}),
                    "zuordnung" => json!({"zuordnungen": []}),
                    _ => geteilt.lock().unwrap().clone(),
                };
                let body = json!({"provider": "StubAnbieter", "choices": [{"finish_reason": "stop", "message": {"content": inhalt.to_string()}}]}).to_string();
                let antwort = format!("HTTP/1.1 200 X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = s.write_all(antwort.as_bytes());
            }
        });
        Self { basis, dialog, aufrufe }
    }

    pub fn aufrufe(&self) -> usize {
        self.aufrufe.load(Ordering::SeqCst)
    }

    /// Die Antwort der Stufe `dialog` im Ganzen tauschen (`vorschlaege`, `rueckfragen`, `antwort`, `unsicher`).
    pub fn setze_dialog(&self, dialog: Value) {
        *self.dialog.lock().unwrap() = dialog;
    }

    /// Die Antwort der Stufe `dialog`: `vorschlaege` (jeder mit `feld_id`, `wert`, `beleg`) und nichts weiter.
    pub fn setze_vorschlaege(&self, vorschlaege: &[Value]) {
        let v: Vec<Value> = vorschlaege
            .iter()
            .map(|v| json!({"feld_id": v["feld_id"], "wert": v["wert"], "beleg": v["beleg"], "begruendung": v.get("begruendung").cloned().unwrap_or(json!("b")), "aussage": 0, "rechenweg": null}))
            .collect();
        *self.dialog.lock().unwrap() = json!({"vorschlaege": v, "rueckfragen": [], "antwort": "", "unsicher": false});
    }
}

/// Eine Server-Antwort in der Form, die `/chat` liefert (`_antwort` in `tests/test_ui_ki_flow.py`).
pub fn antwort(vorschlaege: &[Value], rueckfragen: &[Value], aussagen: &[Value]) -> Value {
    let mach = |v: &Value, extra: Value| {
        let mut d = json!({
            "feld_id": v["feld_id"], "wert": v["wert"], "beleg": v.get("beleg").cloned().unwrap_or(json!("")),
            "frage": v.get("frage").cloned().unwrap_or_else(|| json!(format!("Frage zu {}", v["feld_id"].as_str().unwrap_or("")))),
            "typ": v.get("typ").cloned().unwrap_or(json!("cent")), "frage_invertiert": false,
            "einheit": v.get("einheit").cloned().unwrap_or(Value::Null), "enum_labels": Value::Null,
            "aussage": v.get("aussage").cloned().unwrap_or(json!(0)),
        });
        if let Some(rw) = v.get("rechenweg") {
            d["rechenweg"] = rw.clone();
        }
        for (k, w) in extra.as_object().unwrap() {
            d[k] = w.clone();
        }
        d
    };
    json!({
        "vorschlaege": vorschlaege.iter().map(|v| mach(v, json!({"event_id": v.get("event_id").cloned().unwrap_or(json!("EV-STUB"))}))).collect::<Vec<_>>(),
        "abgelehnt": [], "abgelehnt_gruende": {}, "konflikte": [], "antwort": "", "unsicher": false,
        "hinweis": "Vorschläge erfasst — bitte jeden einzeln bestätigen (die KI setzt nichts).",
        "aussagen": aussagen, "rueckfragen": rueckfragen,
    })
}

/// Eine Rueckfrage (`_rf`).
pub fn rf(frage: &str, feld_id: &str) -> Value {
    json!({"frage": frage, "feld_id": feld_id, "aussage": 0})
}

/// Ein Vorschlag der KI mit Beleg.
pub fn vorschlag(feld_id: &str, wert: &Value, beleg: &str) -> Value {
    json!({"feld_id": feld_id, "wert": wert, "beleg": beleg})
}

/// `/chat` abfangen. Mehrere Antworten gehen der Reihe nach hinaus, die letzte bleibt stehen. Die Seite laedt das
/// Skript neu, sobald sie neu geladen wird: der Aufrufer ruft das nach jedem `oeffne`.
pub fn stub(s: &Sitzung, antworten: &[Value]) {
    s.skript(
        "window.__CHAT = arguments[0]; if (!window.__ECHT_FETCH) { window.__ECHT_FETCH = window.fetch; } \
         window.fetch = (u, o) => String(u).endsWith('/chat') \
           ? Promise.resolve(new Response(JSON.stringify(window.__CHAT.length > 1 ? window.__CHAT.shift() : window.__CHAT[0]), \
               {status: 200, headers: {'Content-Type': 'application/json'}})) \
           : window.__ECHT_FETCH(u, o);",
        &[json!(antworten)],
    );
}

/// `/chat` anhalten: der Aufruf bleibt offen, bis `gib_frei` ihn mit `status`/`body` beendet (Ersatz fuer ein
/// Modell, das Sekunden braucht). `status` 0: das Netz bricht ab (`fetch` wirft).
pub fn halte_an(s: &Sitzung) {
    s.skript(
        "window.__CHAT_OFFEN = null; if (!window.__ECHT_FETCH) { window.__ECHT_FETCH = window.fetch; } \
         window.fetch = (u, o) => String(u).endsWith('/chat') \
           ? new Promise((res, rej) => { window.__CHAT_OFFEN = { res, rej }; }) : window.__ECHT_FETCH(u, o);",
        &[],
    );
}

pub fn gib_frei(s: &Sitzung, status: u16, body: &Value) {
    s.skript(
        "const offen = window.__CHAT_OFFEN; const st = arguments[0]; const body = arguments[1]; \
         setTimeout(() => { if (st === 0) { offen.rej(new TypeError('Netz weg')); } else { \
           offen.res(new Response(JSON.stringify(body), {status: st, headers: {'Content-Type': 'application/json'}})); } }, 0);",
        &[json!(status), body.clone()],
    );
}

/// Absenden und warten, bis die Anzeige steht (`_senden`): erst ist chatSenden ganz durch (`data-chat-laeuft` weg),
/// danach, falls erwartet, steht die Rueckfragen-Seite. `false`, wenn etwas ausbleibt.
pub fn senden(s: &Sitzung, text: &str, rueckfragen_erwartet: bool) -> bool {
    s.fuelle("#chat-text", text);
    s.klick("#chat-send");
    let frei = s.try_warte_bis("!document.getElementById('chat-send').disabled", 5).is_some()
        && s.try_warte_bis("!document.body.dataset.chatLaeuft", 10).is_some();
    frei && (!rueckfragen_erwartet || s.try_warte_bis("!document.getElementById('rueckfragen').hidden", 5).is_some())
}

/// Was die Rueckfragen-Seite zeigt (`_sichtbare_rueckfrage`).
pub fn rf_zustand(s: &Sitzung) -> Value {
    s.ausdruck(
        "({offen: !document.getElementById('rueckfragen').hidden, zaehler: document.getElementById('rf-zaehler').textContent, \
          frage: document.getElementById('rf-frage').textContent, \
          eingaben: document.querySelectorAll('#rf-eingabe input, #rf-eingabe select').length})",
    )
}

/// Die Instanz-Zeilen im Rueckfragen-Fenster (`_rf_instanzen`).
pub fn rf_instanzen(s: &Sitzung) -> Value {
    s.ausdruck(
        "(() => { const zeilen = [...document.querySelectorAll('#rf-eingabe .instanz-zeile')]; \
          return {zeilen: zeilen.length, marken: zeilen.map(z => z.querySelector('.instanz-marke').textContent), \
                  ids: zeilen.map(z => (z.querySelector('input, select') || {}).id), \
                  eingaben: document.querySelectorAll('#rf-eingabe input, #rf-eingabe select').length}; })()",
    )
}

/// `fam_anzahl_kinder` BESTAETIGT setzen (`_setze_kinderzahl`); der Status des Schreibens.
pub fn setze_kinderzahl(s: &Sitzung, n: i64) -> Value {
    s.auswerten(
        "async (n) => { const r = await jpost(`/fall/${FALL}/event`, { feld_id: 'fam_anzahl_kinder', wert: n, zustand: 'bestaetigt', \
           herkunft: {herkunft: 'laie', pruef_tiefe: 'ungeprueft', haftung: 'nutzer'}, schreiber: 'ui:laie', \
           signal: {signal_1: null, signal_2: 'klick@fam_anzahl_kinder'}}); \
         if (r.status < 200 || r.status >= 300) return r.status; await refresh(); return r.status; }",
        &[json!(n)],
    )
}
