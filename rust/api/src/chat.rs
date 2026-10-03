//! `api.chat` (`api.py:1095`): der Chat-Berater. EIN Kanal für Werte vorschlagen UND Fragen
//! beantworten. Die drei Modellstufen laufen in [`llm::dialog::llm_dialog`]; hier steht, was
//! `api.chat` darum tut — Katalog und Kontext bauen, jeden Vorschlag durch Scheiben-Gate und
//! Auflagen des Stores schicken, Abweisungen und Konflikte in die Antwort legen.
//!
//! Geschrieben wird nur vorläufig (`llm:chat`, `llm_vorschlag`, `signal_2 = null`); die Auflagen A, K1
//! und F2 erzwingt der Store. Eine Abweisung nennt in `abgelehnt_gruende` die Klasse und das Feld,
//! nie den Wert ([`abgelehnt_grund`]; Julius 2026-10-03, 6c).
//!
//! # Abweichungen von Python
//! `ponytail`: Zahlen über 2^53 teilt Python in `_wert_klartext` exakt durch 100, hier läuft die
//! Teilung über `f64`; so ein Wert steht nur im Kontext an das Modell. `NaN` druckt Rust als `NaN`,
//! Python als `nan`; der Store nimmt kein `NaN` an.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use bescheid::BindungIndex;
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, PyWert, Zustand};
use llm::dialog::{llm_dialog, AuditProtokoll, DialogErgebnis, Protokoll};
use llm::gates::KatalogFeld;
use llm::parse::Vorschlag;
use llm::py::GeordneteMap;
use llm::{Chat, Completion, HttpChat, Konfiguration, LlmFehler, Nachricht};
use serde_json::{json, Map, Value};
use store::fehler_log::{protokolliere, FallId, Meta, Stufe};
use store::{BindungNachschlag, Event, EventId, Katalog, NeuesEventRoh, Signal, Store};

use crate::antwort::Antwort;
use crate::anzeige::{anzeige_metadaten, enum_labels};
use crate::eigener_fall::EigenerFall;
use crate::fehler::ApiFehler;
use crate::flow;
use crate::python::{repr, typname, unhashbar, wahr};
use crate::zustand::{ScheibenBindung, Zustand as Dienst};

/// `_ERKLAER_KONTEXT_MAX` (`api.py:1276`): so viele bestätigte Angaben gehen höchstens an das Modell.
const KONTEXT_MAX: usize = 40;

/// `CHAT_501` (`api.py:862`).
fn chat_501() -> Value {
    json!({
        "fehler": "not_implemented",
        "vertrag": "LLM-Chat schreibt qua Store-Auflage A ausschliesslich vorlaeufig-Events \
(schreiber='llm:…', herkunft.herkunft='llm_vorschlag', signal_2=null); \
Bestätigung bleibt der menschliche Zwei-Signal-Klick.",
        "stufe": "spätere Stufe mit eigenem Julius-Cap — kein LLM-Call in dieser Stufe.",
    })
}

/// Der Grund einer Abweisung in `abgelehnt_gruende`: `<Klasse>: <feld_id>`, nie der Wert
/// (`api_llm._abgelehnt_grund`, Julius 2026-10-03, 6c „Feld und Typ, nicht der Wert").
///
/// Die Klasse ist der Tag bis zur schließenden Klammer (`fail-closed (Format)`, `(Typ)`, `(Bereich)`,
/// `(F2/Magnitude)`). Ein Tag mit anderen Zeichen als ASCII-Buchstaben, Ziffern, `_` und `/` zählt
/// nicht; dann steht nur `fail-closed`. Der Text des Stores selbst bleibt unberührt.
///
/// `ponytail`: eine Meldung ohne `fail-closed` nennt in Python die Klasse der Ausnahme
/// (`KeyError`, …). Die Abweisungen des Rust-Stores beginnen alle mit `fail-closed`; der Zweig
/// steht hier nur, damit der Wert nie durchrutscht.
///
/// ```
/// use api::chat::abgelehnt_grund;
/// assert_eq!(abgelehnt_grund("fail-closed (Format): kind_idnr='x' passt nicht", "kind_idnr"),
///            "fail-closed (Format): kind_idnr");
/// assert_eq!(abgelehnt_grund("fail-closed (GEHEIM-123): f=1", "f"), "fail-closed: f");
/// ```
#[must_use]
pub fn abgelehnt_grund(meldung: &str, feld_id: &str) -> String {
    let Some(rest) = meldung.strip_prefix("fail-closed") else {
        return format!("ValueError: {feld_id}");
    };
    let tag = rest
        .strip_prefix(" (")
        .and_then(|r| r.split_once(')'))
        .map(|(tag, _)| tag)
        .filter(|t| {
            !t.is_empty()
                && t.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'/')
        });
    match tag {
        Some(t) => format!("fail-closed ({t}): {feld_id}"),
        None => format!("fail-closed: {feld_id}"),
    }
}

/// Der Chat-Client, der `$LLM_API_BASE`, `$LLM_MODEL` und `$LLM_API_KEY` bei jedem Aufruf liest
/// (`llm_client._call`): fehlt eine, scheitert Stufe 1 mit dem Ausfall-Eintrag im Protokoll.
struct UmgebungsChat;

impl Chat for UmgebungsChat {
    fn complete(
        &self,
        nachrichten: &[Nachricht],
        schema: Option<&Value>,
    ) -> Result<Completion, LlmFehler> {
        // Python lässt bei fehlender Umgebung den Anbieter des vorigen Aufrufs im Thread stehen
        // (`_merke` kommt erst nach `_key()`); der Docstring von `letzte_meta` verspricht das Gegenteil.
        llm::vergiss_anbieter();
        HttpChat {
            konfiguration: Konfiguration::aus_env()?,
        }
        .complete(nachrichten, schema)
    }

    fn letzter_anbieter(&self) -> String {
        llm::letzter_anbieter()
    }
}

/// Audit (`llm_call`) und Fluss-Mitschnitt (`ki`) der drei Stufen.
struct ChatProtokoll<'a> {
    audit: AuditProtokoll,
    ablage: &'a Path,
    fall_id: &'a str,
}

impl Protokoll for ChatProtokoll<'_> {
    fn melde(&self, teil: &str) {
        self.audit.melde(teil);
    }

    fn mitschnitt(&self, stufe: u8, was: &str, inhalt: &PyWert) {
        flow::schreibe(
            self.ablage,
            Some(self.fall_id),
            "ki",
            &PyWert::Objekt(vec![
                ("stufe".to_owned(), PyWert::Ganz(i64::from(stufe))),
                ("was".to_owned(), PyWert::Text(was.to_owned())),
                ("inhalt".to_owned(), inhalt.clone()),
            ]),
        );
    }
}

/// `_wert_klartext(fid, wert, bindung)` (`api.py:1250`): ein Wert, wie ein Mensch ihn liest.
fn wert_klartext(fid: &str, wert: &PyWert, index: &BindungIndex<'_>) -> String {
    let b = index.get(fid);
    match b.map(|b| b.typ.als_str()) {
        Some("cent") => {
            // `isinstance(wert, (int, float))`, und `bool` ist ein `int`.
            #[allow(clippy::cast_precision_loss)] // über 2^53 teilt Python exakt, s. Moduldoku
            let zahl = match wert {
                PyWert::Bool(w) => Some(f64::from(u8::from(*w))),
                PyWert::Ganz(n) => Some(*n as f64),
                PyWert::GrossGanz(n) => Some(*n as f64),
                PyWert::Gleit(f) => Some(*f),
                _ => None,
            };
            if let Some(z) = zahl {
                return format!("{:.2} EUR", z / 100.0).replace('.', ",");
            }
        }
        Some("bool") => {
            let wahr = wert.truthy();
            let ja = if b.is_some_and(|b| b.frage_invertiert) {
                !wahr
            } else {
                wahr
            };
            return if ja { "ja" } else { "nein" }.to_owned();
        }
        Some("enum") => {
            if let PyWert::Text(w) = wert {
                if let Some(t) = enum_labels(fid).get(w.as_str()).and_then(Value::as_str) {
                    return t.to_owned();
                }
            }
            return wert.py_str();
        }
        _ => {}
    }
    match b.and_then(|b| b.einheit.as_deref()).filter(|e| !e.is_empty()) {
        Some(einheit) => format!("{} {einheit}", wert.py_str()),
        None => wert.py_str(),
    }
}

/// Die bestätigten Events in der Reihenfolge von `_aktives(store).items()` (`store.py:92`): das
/// `dict` führt jedes Feld an der Stelle seines ersten, nicht ersetzten Events und mit dem Wert
/// des letzten.
fn bestaetigte(store: &Store) -> Vec<&Event> {
    let ersetzt: HashSet<EventId> = store.events().iter().filter_map(|e| e.ersetzt).collect();
    let mut stellen: Vec<&Event> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for e in store.events().iter().filter(|e| !ersetzt.contains(&e.event_id)) {
        if let Some(&i) = index.get(e.feld_id.as_str()) {
            if let Some(s) = stellen.get_mut(i) {
                *s = e;
            }
        } else {
            index.insert(e.feld_id.as_str(), stellen.len());
            stellen.push(e);
        }
    }
    stellen
        .into_iter()
        .filter(|e| e.zustand == Zustand::Bestaetigt)
        .collect()
}

/// `(bindung.get(f) or {}).get("fragetext_laie", f)`: der Fragetext, `None` wo die Bindung ihn
/// ausdrücklich leer führt, die Feld-ID wo es keine Bindung gibt.
fn frage_von(f: &str, index: &BindungIndex<'_>) -> Option<String> {
    match index.get(f) {
        Some(b) => b.fragetext_laie.clone(),
        None => Some(f.to_owned()),
    }
}

/// `_erklaer_kontext(store, bindung, fid)` (`api.py:1279`): das offene Feld, seine Kurzhilfe, sein
/// Zitatanker — und was der Nutzer schon bestätigt hat. Der WERT besonderer Kategorien (Art. 9
/// DSGVO) bleibt draußen, die Zahl der ausgelassenen Angaben steht da.
fn erklaer_kontext(store: &Store, index: &BindungIndex<'_>, fid: Option<&str>) -> String {
    let mut teile: Vec<String> = Vec::new();
    if let Some(b) = fid.and_then(|f| index.get(f)) {
        teile.push(format!(
            "Die Frage, um die es geht: „{}“",
            b.fragetext_laie.as_deref().unwrap_or("None")
        ));
        if !b.hilfe_kurz.is_empty() {
            teile.push(format!(
                "Dazu gehört laut Feldbeschreibung: {}",
                b.hilfe_kurz
            ));
        }
        let a = &b.anker_ref;
        if !a.quelle.is_empty() || !a.zitatanker.is_empty() {
            teile.push(format!(
                "Wörtlicher Gesetzestext dazu — {}: „{}“",
                a.quelle, a.zitatanker
            ));
        }
    }
    let (mut offen, mut zurueckgehalten) = (Vec::new(), 0_usize);
    for e in bestaetigte(store) {
        let f = e.feld_id.as_str();
        let frage = frage_von(f, index);
        if llm::pii::ist_besondere_kategorie(f, frage.as_deref().unwrap_or("")) {
            zurueckgehalten += 1;
            continue;
        }
        offen.push(format!(
            "- {} → {}",
            frage.as_deref().unwrap_or("None"),
            wert_klartext(f, &e.wert, index)
        ));
        if offen.len() >= KONTEXT_MAX {
            break;
        }
    }
    if !offen.is_empty() {
        teile.push(format!(
            "Das hat der Nutzer bereits bestätigt:\n{}",
            offen.join("\n")
        ));
    }
    if zurueckgehalten > 0 {
        teile.push(format!(
            "({zurueckgehalten} weitere Angaben liegen vor, dürfen dir aber nicht übermittelt werden \
— es sind Gesundheits- oder Konfessionsangaben. Frage nicht danach und behandle sie als beantwortet.)"
        ));
    }
    teile.join("\n")
}

/// Der Katalog für den Prompt (`prompt_katalog`, `api.py:1147`): die LLM-vorschlagbaren Felder dieser
/// Scheibe in Scheibenreihenfolge.
fn prompt_katalog(sb: &ScheibenBindung<'_>, katalog: &Katalog) -> Vec<KatalogFeld> {
    sb.felder
        .iter()
        .filter(|f| katalog.erlaubt("llm", f))
        .filter_map(|f| sb.index.get(f.as_str()).map(|b| (f, *b)))
        .map(|(fid, b)| KatalogFeld {
            feld_id: fid.clone(),
            fragetext_laie: b.fragetext_laie.clone(),
            hilfe_kurz: Some(b.hilfe_kurz.clone()),
            typ: Some(b.typ.als_str().to_owned()),
            bereich: b.bereich.as_ref().map(|r| {
                GeordneteMap(vec![
                    ("min".to_owned(), json!(r.min)),
                    ("max".to_owned(), json!(r.max)),
                    ("grund".to_owned(), json!(r.grund)),
                ])
            }),
            enum_werte: b.enum_werte.clone(),
            regel_id: Some(b.quelle.regel_id.clone()),
            instanz_gruppe: b.instanz_gruppe.clone(),
        })
        .collect()
}

/// `(body.get("text") or "").strip()` und der Rumpf davor: ein Rumpf ohne `get` ist ein 500.
fn freitext_von(body: &Value) -> Result<String, ApiFehler> {
    let Value::Object(rumpf) = body else {
        return Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'get'", typname(body)),
        ));
    };
    match rumpf.get("text") {
        Some(Value::String(s)) => Ok(llm::py::strip(s).to_owned()),
        Some(andere) if wahr(andere) => Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'strip'", typname(andere)),
        )),
        _ => Ok(String::new()),
    }
}

/// `body.get("feld_id") or None` als Schlüssel in `bindung.get(...)`: Liste und Objekt sind dort nicht
/// hashbar (`TypeError`), jeder andere wahre Nicht-Text ist schlicht kein Feld.
fn offenes_feld(body: &Value) -> Result<Option<&str>, ApiFehler> {
    let fid = body.get("feld_id").unwrap_or(&Value::Null);
    if matches!(fid, Value::Array(a) if !a.is_empty())
        || matches!(fid, Value::Object(o) if !o.is_empty())
    {
        return Err(ApiFehler::unerwartet(
            "TypeError",
            unhashbar(fid, "dict key"),
        ));
    }
    Ok(fid.as_str().filter(|s| !s.is_empty()))
}

/// Ein Konflikt (Auflage B, Fall 2): das Feld ist LLM-vorschlagbar, hat aber schon ein aktives Event.
fn konflikt(sb: &ScheibenBindung<'_>, v: &Vorschlag, aktiv: &Event) -> Result<Value, ApiFehler> {
    let mut k = Map::new();
    k.insert("feld_id".into(), json!(v.feld_id));
    k.insert(
        "aktueller_wert".into(),
        aktiv
            .wert
            .zu_json()
            .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?,
    );
    k.insert("aktuelles_event_id".into(), json!(aktiv.event_id.to_string()));
    k.insert("vorschlag_wert".into(), v.wert.clone());
    k.insert("begruendung".into(), json!(v.begruendung));
    k.insert("beleg".into(), json!(v.beleg));
    // `gross`: das Feld steuert selbst eine andere Regel (`_ist_struktureller_konflikt`).
    k.insert(
        "gross".into(),
        json!(!sb.graph.bedingte_regeln(&v.feld_id).is_empty()),
    );
    k.insert("rechenweg".into(), v.rechenweg.clone());
    anzeige_metadaten(&v.feld_id, &sb.index, &mut k);
    Ok(Value::Object(k))
}

/// Was die Vorschläge ergeben haben.
#[derive(Default)]
struct Verarbeitet {
    geschrieben: Vec<Value>,
    abgelehnt: Vec<String>,
    gruende: Map<String, Value>,
    konflikte: Vec<Value>,
}

/// Der Vorschlag als `append_event`-Eingabe (`api.py:1206`): vorläufig, `llm:chat`, der Beleg im
/// `signal_1`.
fn neues_event(v: &Vorschlag) -> Result<NeuesEventRoh, ApiFehler> {
    let achse = |t: &str| {
        Achsenwert::new(t).map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))
    };
    Ok(NeuesEventRoh {
        feld_id: v.feld_id.clone(),
        wert: PyWert::from(v.wert.clone()),
        zustand: Zustand::Vorlaeufig,
        herkunft: HerkunftVektor::Voll(Herkunft {
            herkunft: achse("llm_vorschlag")?,
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: achse("nutzer")?,
        }),
        schreiber: "llm:chat".to_owned(),
        signal: Signal {
            signal_1: Some(Some(PyWert::Objekt(vec![
                ("typ".to_owned(), PyWert::Text("llm".to_owned())),
                (
                    "begruendung".to_owned(),
                    PyWert::Text(v.begruendung.clone()),
                ),
                ("beleg".to_owned(), PyWert::Text(v.beleg.clone())),
            ]))),
            signal_2: None,
            signal_2_fehlt: false,
        },
        signal_2_fremd: None,
        ersetzt: None,
        ts: None,
    })
}

/// Die Schleife über die Vorschläge (`api.py:1174-1229`): Scheiben-Gate, Konflikt, Store.
fn verarbeite(
    fall: &mut EigenerFall,
    sb: &ScheibenBindung<'_>,
    katalog: &Katalog,
    erg: &DialogErgebnis,
) -> Result<Verarbeitet, ApiFehler> {
    let mut aus = Verarbeitet::default();
    for beleg in &erg.vorschlaege {
        let v = beleg.vorschlag();
        let fid = v.feld_id.as_str();
        if !fid.is_empty() && !sb.index.contains_key(fid) {
            aus.abgelehnt.push(fid.to_owned());
            aus.gruende.insert(
                fid.to_owned(),
                json!("scheibenfremd: Feld gehört nicht zu dieser Scheibe"),
            );
            continue;
        }
        let bestehendes = if fid.is_empty() {
            None
        } else {
            fall.store().aktives(fid)
        };
        if let Some(aktiv) = bestehendes.filter(|_| katalog.erlaubt("llm", fid)) {
            aus.konflikte.push(konflikt(sb, v, aktiv)?);
            continue;
        }
        let neu = neues_event(v)?;
        match fall
            .store_mut()
            .append_roh(&neu, Some(katalog), BindungNachschlag::neu(&sb.index))
        {
            Ok(id) => {
                let mut g = Map::new();
                g.insert("feld_id".into(), json!(fid));
                g.insert("event_id".into(), json!(id.to_string()));
                g.insert("wert".into(), v.wert.clone());
                g.insert("beleg".into(), json!(v.beleg));
                g.insert("rechenweg".into(), v.rechenweg.clone());
                anzeige_metadaten(fid, &sb.index, &mut g);
                aus.geschrieben.push(Value::Object(g));
            }
            Err(e) => {
                aus.abgelehnt.push(fid.to_owned());
                if !fid.is_empty() {
                    // Klasse und Feld, nie der Wert (6c): nicht `e.to_string()`.
                    aus.gruende
                        .insert(fid.to_owned(), json!(abgelehnt_grund(&e.to_string(), fid)));
                }
            }
        }
    }
    Ok(aus)
}

/// Die Beobachtung auf stderr: nur Feld-IDs und Zahlen, nie ein Wert oder der Freitext. Gibt die
/// echten Feld-IDs zurück.
fn beobachte(abgelehnt: &[String]) -> Vec<String> {
    let echte: Vec<String> = abgelehnt.iter().filter(|a| !a.is_empty()).cloned().collect();
    if !echte.is_empty() {
        let sortiert: BTreeSet<&String> = echte.iter().collect();
        eprintln!(
            "[haut.chat] LLM-Vorschläge außerhalb Katalog abgelehnt: {}",
            repr(&json!(sortiert))
        );
    }
    let malformt = abgelehnt.len() - echte.len();
    if malformt > 0 {
        eprintln!(
            "[haut.chat] LLM-Vorschläge mit fehlender/leerer feld_id abgelehnt: {malformt}"
        );
    }
    echte
}

/// Die Instanz-Gruppen als `(gruppe, anzahl_feld)` für den Dialog (`TR.lade_instanz_gruppen()`).
fn instanz_gruppen(sb: &ScheibenBindung<'_>) -> Vec<(String, String)> {
    sb.graph
        .instanz_gruppen()
        .map(|g| (g.gruppe.clone(), g.anzahl_feld.clone()))
        .collect()
}

/// `api.chat(fall_id, body)` nach dem Owner-Check. Blockiert für die Dauer der drei Modellaufrufe.
///
/// `roh` ist der Rumpf als Bytes: der Mitschnitt hält `bei_feld` so, wie der Client es schickte.
///
/// # Errors
/// 500 mit der Python-Klasse, wo Python eine andere Ausnahme wirft; ein Ausfall von Stufe 1 ist
/// keine Ausnahme, sondern die Antwort `501`.
pub fn chat(
    z: &Dienst,
    fall: &mut EigenerFall,
    body: &Value,
    roh: &[u8],
) -> Result<Antwort, ApiFehler> {
    let fall_id = fall.id().as_str().to_owned();
    let sb = z.scheibe_bindung(fall.store())?;
    let freitext = freitext_von(body)?;
    let katalog = z.katalog()?;
    let prompt = prompt_katalog(&sb, &katalog);
    let kontext = erklaer_kontext(fall.store(), &sb.index, offenes_feld(body)?);
    let ablage = z.konfig.audit_dir.as_path();
    let ordnung: PyWert =
        serde_json::from_slice(roh).unwrap_or_else(|_| PyWert::from(body.clone()));
    flow::schreibe(
        ablage,
        Some(&fall_id),
        "nutzertext",
        &PyWert::Objekt(vec![
            ("text".to_owned(), PyWert::Text(freitext.clone())),
            (
                "bei_feld".to_owned(),
                flow::hole(&ordnung, "feld_id")
                    .cloned()
                    .unwrap_or(PyWert::Null),
            ),
        ]),
    );
    let protokoll = ChatProtokoll {
        audit: AuditProtokoll {
            pfad: z.konfig.audit_pfad(),
            user_id: fall.nutzer().map(|u| u.as_str().to_owned()),
        },
        ablage,
        fall_id: &fall_id,
    };
    let gruppen = instanz_gruppen(&sb);
    let erg = match llm_dialog(&UmgebungsChat, &freitext, &prompt, &kontext, &gruppen, &protokoll) {
        Ok(erg) => erg,
        Err(e) => {
            // WARNUNG, kein FEHLER: der Ausfall ist erwartet. ponytail: die PII-Muster liefert erst
            // `llm::pii`; bis dahin steht `<gesperrt:…>` als Fall-Kennung.
            let _ = protokolliere(
                &z.konfig.fehler_pfad(),
                "api.chat llm",
                &e,
                Stufe::Warnung,
                Some(FallId::pruefe(&fall_id, &[])),
                Meta::default(),
            );
            return Ok(Antwort::neu(501, chat_501()));
        }
    };
    let aus = verarbeite(fall, &sb, &katalog, &erg)?;
    store::speichere(fall.pfad(), fall.store().datei())?;
    let abgelehnt = beobachte(&aus.abgelehnt);
    Ok(Antwort::neu(
        200,
        json!({
            "vorschlaege": aus.geschrieben,
            "abgelehnt": abgelehnt,
            "abgelehnt_gruende": aus.gruende,
            "konflikte": aus.konflikte,
            "antwort": erg.antwort,
            "unsicher": erg.unsicher,
            "aussagen": erg.aussagen,
            "rueckfragen": erg.rueckfragen,
            "rueckfragen_zurueckgestellt": erg.rueckfragen_zurueckgestellt,
            "hinweis": "Vorschläge erfasst — bitte jeden einzeln bestätigen (die KI setzt nichts).",
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Derselbe Satz an Fällen wie `tests/test_haut_chat.py::test_abgelehnt_grund_klassen_und_randfaelle`.
    #[test]
    fn grund_wie_python() {
        for (meldung, soll) in [
            ("fail-closed (Format): kind_idnr='GEHEIM-123' passt nicht", "fail-closed (Format): f"),
            ("fail-closed (F2/Magnitude): a=1 von x", "fail-closed (F2/Magnitude): f"),
            ("fail-closed (Typ): f=GEHEIM-123", "fail-closed (Typ): f"),
            ("fail-closed: signal_2 muss Text oder null sein", "fail-closed: f"),
            ("fail-closed (GEHEIM-123): f=1", "fail-closed: f"),
            ("fail-closed (Typ", "fail-closed: f"),
            ("fail-closed ()", "fail-closed: f"),
            ("etwas anderes mit GEHEIM-123", "ValueError: f"),
        ] {
            assert_eq!(abgelehnt_grund(meldung, "f"), soll, "{meldung}");
        }
    }

    #[test]
    fn chat_501_hat_den_wortlaut_aus_python() {
        let v = chat_501();
        assert_eq!(v["fehler"], "not_implemented");
        let vertrag = v["vertrag"].as_str().unwrap();
        assert!(vertrag.contains("schreiber='llm:…'"));
        assert!(vertrag.ends_with("Zwei-Signal-Klick."));
    }
}
