//! Provider-agnostischer Chat-Completions-Client (`produkt/haut/llm_client.py`): EIN Aufruf,
//! Wiederholung bei voruebergehenden Stoerungen, Wanduhr-Frist, Schluessel-Maskierung.
//!
//! Wiederholungsregel wortgleich zu `_call` (`llm_client.py:215-230`): drei Versuche; 429/500/
//! 502/503/504, Netzstoerung, Socket-Timeout und leerer Inhalt sind voruebergehend; eine
//! abgeschnittene Antwort (`finish_reason == "length"`) bekommt genau EINE Wiederholung, mit der
//! kleineren Frist; alles andere ist endgueltig. Backoff 1 s, 2 s.
use std::cell::RefCell;
use std::fmt;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::http::{self, Transport};
use crate::pii::{Gefiltert, Maskiert};

thread_local! {
    /// `_letzte.meta["provider"]` (`llm_client.py:81`): wer in DIESEM Thread zuletzt geantwortet hat.
    static ANBIETER: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Der Anbieter der letzten gelesenen Antwort in diesem Thread (`letzte_meta().get("provider", "")`).
/// Leer, solange keine Antwort gelesen wurde. Der Ausfall-Eintrag im Protokoll nennt ihn auch dann,
/// wenn die Antwort leer oder abgeschnitten war und der Aufruf scheiterte.
#[must_use]
pub fn letzter_anbieter() -> String {
    ANBIETER.with(|a| a.borrow().clone())
}

/// `_merke("", "")`: nie die Angabe eines früheren Aufrufs stehen lassen. `HttpChat::complete` ruft es
/// selbst; wer vor dem Aufruf scheitern kann (fehlende Umgebung), ruft es zuerst.
pub fn vergiss_anbieter() {
    ANBIETER.with(|a| a.borrow_mut().clear());
}

fn merke_anbieter(provider: &str) {
    ANBIETER.with(|a| provider.clone_into(&mut a.borrow_mut()));
}

/// `_VORUEBERGEHEND` (`llm_client.py:100`).
const VORUEBERGEHEND: [u16; 5] = [429, 500, 502, 503, 504];
/// `_VERSUCHE`.
const VERSUCHE: u32 = 3;
/// `_ABGESCHNITTEN_MAX`: ein regulaerer Versuch plus EINE Wiederholung.
const ABGESCHNITTEN_MAX: u32 = 2;

/// Kontrolliertes Vokabular fuer das Protokoll (`GRUND_*`, `llm_client.py:73-75`) — nie
/// Anbietertext.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grund {
    Leer,
    Abgeschnitten,
    Frist,
    Sonstig,
}

impl Grund {
    /// Das Protokoll-Wort; `Sonstig` ist leer wie Pythons `getattr(e, "grund", "")`.
    ///
    /// ```
    /// assert_eq!(llm::Grund::Frist.als_str(), "frist_ueberschritten");
    /// assert_eq!(llm::Grund::Sonstig.als_str(), "");
    /// ```
    #[must_use]
    pub fn als_str(self) -> &'static str {
        match self {
            Self::Leer => "leere_antwort",
            Self::Abgeschnitten => "abgeschnitten",
            Self::Frist => "frist_ueberschritten",
            Self::Sonstig => "",
        }
    }
}

/// Endgueltiger Fehlschlag eines Aufrufs (Pythons `LlmNichtVerfuegbar` mit `grund`/`versuche`).
/// `detail` ist gekuerzte Diagnose; der Schluessel ist darin IMMER durch `<KEY>` ersetzt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmFehler {
    /// Nach allen Versuchen voruebergehend gescheitert (`_aufgegeben(e, 3)`).
    Voruebergehend {
        versuche: u32,
        grund: Grund,
        detail: String,
    },
    /// Abgeschnitten, und die eine Wiederholung auch (`_aufgegeben(e, abgeschnitten)`).
    Abgeschnitten { versuche: u32 },
    /// Nicht heilbar: nicht konfiguriert, anderer Status, kaputte Antwort, Frist.
    Endgueltig { grund: Grund, detail: String },
}

impl LlmFehler {
    /// Das Protokoll-Wort (`getattr(e, "grund", "")`).
    ///
    /// ```
    /// assert_eq!(llm::LlmFehler::Abgeschnitten { versuche: 2 }.grund(), "abgeschnitten");
    /// ```
    #[must_use]
    pub fn grund(&self) -> &'static str {
        match self {
            Self::Voruebergehend { grund, .. } | Self::Endgueltig { grund, .. } => grund.als_str(),
            Self::Abgeschnitten { .. } => Grund::Abgeschnitten.als_str(),
        }
    }

    /// `getattr(e, "versuche", 1)`.
    ///
    /// ```
    /// let e = llm::LlmFehler::Endgueltig { grund: llm::Grund::Sonstig, detail: String::new() };
    /// assert_eq!(e.versuche(), 1);
    /// ```
    #[must_use]
    pub fn versuche(&self) -> u32 {
        match self {
            Self::Voruebergehend { versuche, .. } | Self::Abgeschnitten { versuche } => *versuche,
            Self::Endgueltig { .. } => 1,
        }
    }
}

impl fmt::Display for LlmFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Voruebergehend { versuche, detail, .. } => {
                write!(f, "LLM-Aufruf nach {versuche} Versuch(en) fehlgeschlagen: {detail}")
            }
            Self::Abgeschnitten { versuche } => write!(
                f,
                "LLM-Aufruf nach {versuche} Versuch(en) fehlgeschlagen: LLM-Antwort bei {MAX_TOKENS} Tokens abgeschnitten — unvollständiges JSON."
            ),
            Self::Endgueltig { detail, .. } => f.write_str(detail),
        }
    }
}

impl std::error::Error for LlmFehler {}

/// `_MAX_TOKENS` (`llm_client.py:67`).
pub const MAX_TOKENS: u32 = 8192;

/// Der API-Schluessel. `Debug` zeigt ihn nie.
#[derive(Clone)]
pub struct Schluessel(String);

impl fmt::Debug for Schluessel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Schluessel(<KEY>)")
    }
}

impl Schluessel {
    /// Fuer `ors`: derselbe Typ, dieselbe Regel — der Wert steht nie in `Debug`.
    pub(crate) fn neu(wert: String) -> Self {
        Self(wert)
    }

    pub(crate) fn als_str(&self) -> &str {
        &self.0
    }
}

/// Endpunkt, Modell, Schluessel und die Zeitgrenzen.
#[derive(Debug, Clone)]
pub struct Konfiguration {
    pub basis: String,
    pub modell: String,
    pub schluessel: Schluessel,
    /// `_FRIST_S` 150 s.
    pub frist: Duration,
    /// `_FRIST_WIEDERHOLUNG_S` 90 s, nach einer abgeschnittenen Antwort.
    pub frist_wiederholung: Duration,
    /// `_TIMEOUT` 30 s je Verbindungsaufbau/Leseoperation.
    pub socket: Duration,
    /// `_BACKOFF_S` (1 s, 2 s).
    pub backoff: [Duration; 2],
}

impl Konfiguration {
    /// Aus `$LLM_API_BASE`, `$LLM_MODEL`, `$LLM_API_KEY` (`llm_client.py:162-174`), mit den
    /// Python-Zeitgrenzen. Reihenfolge der Pruefung wie `_call`: erst Basis/Modell, dann Schluessel.
    ///
    /// ```
    /// // In der Test-Umgebung ist nichts gesetzt: endgueltig, ohne Netz.
    /// if std::env::var("LLM_API_BASE").is_err() {
    ///     assert!(llm::Konfiguration::aus_env().is_err());
    /// }
    /// ```
    ///
    /// # Errors
    /// [`LlmFehler::Endgueltig`], wenn eine der drei Angaben fehlt.
    pub fn aus_env() -> Result<Self, LlmFehler> {
        let env = |k: &str| {
            std::env::var(k)
                .map(|v| v.trim().to_owned())
                .unwrap_or_default()
        };
        let (basis, modell) = (
            env("LLM_API_BASE").trim_end_matches('/').to_owned(),
            env("LLM_MODEL"),
        );
        if basis.is_empty() || modell.is_empty() {
            return Err(endgueltig(
                "LLM_API_BASE/LLM_MODEL nicht gesetzt — Provider nicht konfiguriert.",
            ));
        }
        let schluessel = env("LLM_API_KEY");
        if schluessel.is_empty() {
            return Err(endgueltig(
                "kein LLM_API_KEY in der Umgebung — Chat bleibt reine Erklär-Grenze ($0).",
            ));
        }
        Ok(Self::neu(basis, modell, schluessel))
    }

    /// Feste Werte mit den Python-Zeitgrenzen.
    ///
    /// ```
    /// let k = llm::Konfiguration::neu("http://127.0.0.1:9".into(), "m".into(), "sk-geheim".into());
    /// assert!(!format!("{k:?}").contains("sk-geheim"));
    /// ```
    #[must_use]
    pub fn neu(basis: String, modell: String, schluessel: String) -> Self {
        Self {
            basis,
            modell,
            schluessel: Schluessel(schluessel),
            frist: Duration::from_secs(150),
            frist_wiederholung: Duration::from_secs(90),
            socket: Duration::from_secs(30),
            backoff: [Duration::from_secs(1), Duration::from_secs(2)],
        }
    }
}

fn endgueltig(detail: &str) -> LlmFehler {
    LlmFehler::Endgueltig {
        grund: Grund::Sonstig,
        detail: detail.to_owned(),
    }
}

/// Rolle einer Nachricht.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rolle {
    System,
    User,
}

/// Eine Chat-Nachricht. Nutzertext kommt nur aus [`Gefiltert`] oder [`Maskiert`]; System-Texte
/// baut nur dieses Crate (Prompt-Bausteine).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Nachricht {
    #[serde(rename = "role")]
    rolle: Rolle,
    #[serde(rename = "content")]
    inhalt: String,
}

impl Nachricht {
    pub(crate) fn system(inhalt: String) -> Self {
        Self {
            rolle: Rolle::System,
            inhalt,
        }
    }

    /// Nutzertext — nur gefiltert.
    ///
    /// ```
    /// let (g, _) = llm::pii::filtere("hallo");
    /// assert_eq!(llm::Nachricht::nutzer(&g).inhalt(), "hallo");
    /// ```
    #[must_use]
    pub fn nutzer(text: &Gefiltert) -> Self {
        Self {
            rolle: Rolle::User,
            inhalt: text.as_str().to_owned(),
        }
    }

    /// Kontoauszug-Buchung fuer den Klassifikator (`kontoauszug_writer.py:471-472`).
    pub(crate) fn buchung(zweck: &Maskiert, betrag_cent: i64) -> Self {
        // `f"{betrag / 100:.2f}"`: beide Seiten runden korrekt vom selben f64.
        #[allow(clippy::cast_precision_loss)] // Buchungsbetraege liegen weit unter 2^53 Cent
        let euro = betrag_cent as f64 / 100.0;
        Self {
            rolle: Rolle::User,
            inhalt: format!("Zweck: {zweck}\nBetrag: {euro:.2} EUR"),
        }
    }

    /// Rolle der Nachricht.
    ///
    /// ```
    /// let (g, _) = llm::pii::filtere("x");
    /// assert_eq!(llm::Nachricht::nutzer(&g).rolle(), llm::Rolle::User);
    /// ```
    #[must_use]
    pub fn rolle(&self) -> Rolle {
        self.rolle
    }

    /// Inhalt der Nachricht.
    ///
    /// ```
    /// let (g, _) = llm::pii::filtere("x");
    /// assert_eq!(llm::Nachricht::nutzer(&g).inhalt(), "x");
    /// ```
    #[must_use]
    pub fn inhalt(&self) -> &str {
        &self.inhalt
    }
}

/// Rohe Antwort (`Completion`, `llm_client.py:375-383`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Completion {
    pub text: String,
    pub provider: String,
    pub finish: String,
}

/// Der eine niedrig-level Aufruf (`complete`). Tests setzen eine Fixture-Implementierung ein.
pub trait Chat {
    /// Ein Aufruf mit optionalem strikten JSON-Schema (`{"name", "strict", "schema"}`).
    ///
    /// # Errors
    /// [`LlmFehler`] nach der Wiederholungsregel.
    fn complete(
        &self,
        nachrichten: &[Nachricht],
        schema: Option<&Value>,
    ) -> Result<Completion, LlmFehler>;

    /// Der Anbieter der letzten Antwort (`llm_client.letzte_meta()["provider"]`), fuer den
    /// Ausfall-Eintrag im Protokoll. Ein Chat ohne Anbieter lässt ihn leer.
    fn letzter_anbieter(&self) -> String {
        String::new()
    }
}

/// Der HTTP-Client.
#[derive(Debug, Clone)]
pub struct HttpChat {
    pub konfiguration: Konfiguration,
}

/// Ein einzelner Versuch ist so ausgegangen.
enum Versuch {
    Ok(Completion),
    Voruebergehend(Grund, String),
    Abgeschnitten,
    Endgueltig(Grund, String),
}

impl Chat for HttpChat {
    fn letzter_anbieter(&self) -> String {
        letzter_anbieter()
    }

    fn complete(
        &self,
        nachrichten: &[Nachricht],
        schema: Option<&Value>,
    ) -> Result<Completion, LlmFehler> {
        let k = &self.konfiguration;
        let mut nutzlast = json!({"model": k.modell, "messages": nachrichten, "temperature": 0,
            "response_format": schema.map_or_else(|| json!({"type": "json_object"}),
                |s| json!({"type": "json_schema", "json_schema": s})),
            "max_tokens": MAX_TOKENS});
        if schema.is_some() {
            if let Value::Object(m) = &mut nutzlast {
                m.insert("provider".into(), json!({"require_parameters": true}));
            }
        }
        let koerper = serde_json::to_vec(&nutzlast).map_err(|e| endgueltig(&e.to_string()))?;
        vergiss_anbieter();
        let mut abgeschnitten = 0;
        for versuch in 0..VERSUCHE {
            let letzter = versuch == VERSUCHE - 1;
            let frist = if abgeschnitten > 0 {
                k.frist_wiederholung
            } else {
                k.frist
            };
            match ein_versuch(k, &koerper, frist) {
                Versuch::Ok(c) => return Ok(c),
                Versuch::Endgueltig(grund, detail) => {
                    return Err(LlmFehler::Endgueltig { grund, detail })
                }
                Versuch::Abgeschnitten => {
                    abgeschnitten += 1;
                    if abgeschnitten >= ABGESCHNITTEN_MAX || letzter {
                        return Err(LlmFehler::Abgeschnitten {
                            versuche: abgeschnitten,
                        });
                    }
                }
                Versuch::Voruebergehend(grund, detail) => {
                    if letzter {
                        return Err(LlmFehler::Voruebergehend {
                            versuche: VERSUCHE,
                            grund,
                            detail,
                        });
                    }
                }
            }
            let pause = usize::try_from(versuch)
                .ok()
                .and_then(|i| k.backoff.get(i))
                .copied()
                .unwrap_or_default();
            std::thread::sleep(pause);
        }
        Err(endgueltig("unerreichbar"))
    }
}

/// `_ein_versuch` + `_inhalt` (`llm_client.py:276-372`).
fn ein_versuch(k: &Konfiguration, koerper: &[u8], frist: Duration) -> Versuch {
    let ende = Instant::now() + frist;
    let socket = k.socket.min(frist);
    let schluessel = k.schluessel.0.as_str();
    let antwort = match http::post(&k.basis, schluessel, koerper, socket, ende) {
        Ok(a) => a,
        Err(Transport::Netz(d)) => {
            return Versuch::Voruebergehend(Grund::Sonstig, maskiere(&d, schluessel))
        }
        Err(Transport::Zeit) => {
            return Versuch::Voruebergehend(
                Grund::Sonstig,
                format!(
                    "TimeoutError: Zeitüberschreitung nach {}s",
                    k.socket.as_secs()
                ),
            )
        }
        Err(Transport::Frist) => {
            return Versuch::Endgueltig(Grund::Frist, "LLM-Antwort überschritt die Frist".into())
        }
        Err(Transport::Kaputt(d)) => {
            return Versuch::Endgueltig(
                Grund::Sonstig,
                format!("LLM-Aufruf fehlgeschlagen: {}", maskiere(&d, schluessel)),
            )
        }
    };
    if !(200..300).contains(&antwort.status) {
        // PARITAET-Abweichung (Sicherheit): Python maskiert erst NACH dem Kuerzen auf 300
        // Zeichen — ein Schluessel, der ueber die Schnittkante ragt, bliebe als Praefix stehen.
        // Hier wird vorher maskiert.
        let text = maskiere(&String::from_utf8_lossy(&antwort.koerper), schluessel);
        let detail = crate::py::vorne(&text, 300);
        return if VORUEBERGEHEND.contains(&antwort.status) {
            Versuch::Voruebergehend(Grund::Sonstig, format!("HTTP {} {detail}", antwort.status))
        } else {
            Versuch::Endgueltig(
                Grund::Sonstig,
                format!(
                    "LLM-Aufruf fehlgeschlagen: HTTP {} {detail}",
                    antwort.status
                ),
            )
        };
    }
    let Ok(j) = serde_json::from_slice::<Value>(&antwort.koerper) else {
        return Versuch::Endgueltig(
            Grund::Sonstig,
            "LLM-Aufruf fehlgeschlagen: JSONDecodeError".into(),
        );
    };
    inhalt(&j)
}

/// `_inhalt`: Struktur pruefen, dann „abgeschnitten" VOR „leer" (die genauere Diagnose).
fn inhalt(j: &Value) -> Versuch {
    let Some(wahl) = j.get("choices").and_then(|c| c.get(0)) else {
        return Versuch::Endgueltig(Grund::Sonstig, "LLM-Aufruf fehlgeschlagen: KeyError".into());
    };
    let Some(nachricht) = wahl.get("message").filter(|m| m.is_object()) else {
        return Versuch::Endgueltig(Grund::Sonstig, "LLM-Aufruf fehlgeschlagen: KeyError".into());
    };
    let ende = wahl
        .get("finish_reason")
        .filter(|v| crate::py::wahr(v))
        .map(crate::py::py_str)
        .unwrap_or_default();
    let provider = j
        .get("provider")
        .filter(|v| crate::py::wahr(v))
        .map(crate::py::py_str)
        .unwrap_or_default();
    // `_merke(provider, ende)` steht VOR den Pruefungen: auch eine abgeschnittene oder leere Antwort
    // nennt im Ausfall-Eintrag, wer sie geschickt hat.
    merke_anbieter(&provider);
    if ende == "length" {
        return Versuch::Abgeschnitten;
    }
    let text = match nachricht.get("content") {
        // `(inhalt or "")`: jeder falsche Wert (None, 0, false, []) zaehlt als leer.
        None => String::new(),
        Some(v) if !crate::py::wahr(v) => String::new(),
        Some(Value::String(s)) => s.clone(),
        // PARITAET-Abweichung: Python ruft `.strip()` auf einer Nicht-Zeichenkette und stuerzt
        // mit `AttributeError` ausserhalb jeder Behandlung (HTTP 500). Hier endgueltig.
        Some(_) => {
            return Versuch::Endgueltig(
                Grund::Sonstig,
                "LLM-Aufruf fehlgeschlagen: AttributeError".into(),
            )
        }
    };
    if crate::py::strip(&text).is_empty() {
        return Versuch::Voruebergehend(Grund::Leer, "leerer Inhalt vom Anbieter".into());
    }
    Versuch::Ok(Completion {
        text,
        provider,
        finish: ende,
    })
}

/// Schluessel in einer Diagnose durch `<KEY>` ersetzen (`llm_client.py:305-306`).
fn maskiere(text: &str, schluessel: &str) -> String {
    if schluessel.is_empty() {
        text.to_owned()
    } else {
        text.replace(schluessel, "<KEY>")
    }
}

#[cfg(test)]
mod tests {
    use super::{inhalt, maskiere, Versuch};
    use serde_json::json;

    #[test]
    fn abgeschnitten_vor_leer() {
        let j = json!({"choices": [{"message": {"content": ""}, "finish_reason": "length"}]});
        assert!(matches!(inhalt(&j), Versuch::Abgeschnitten));
    }

    #[test]
    fn leer_ist_voruebergehend() {
        let j = json!({"choices": [{"message": {"content": "  "}, "finish_reason": "stop"}]});
        assert!(matches!(
            inhalt(&j),
            Versuch::Voruebergehend(super::Grund::Leer, _)
        ));
    }

    #[test]
    fn schluessel_wird_maskiert() {
        assert_eq!(
            maskiere("invalid key sk-123", "sk-123"),
            "invalid key <KEY>"
        );
    }
}
