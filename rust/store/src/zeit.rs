//! Ein einziger Zeitstempel-Helfer fuer `store`/`audit`/`fehler_log` — alle drei brauchen
//! `_now()` (`store.py:41-42`) nur als DEFAULT, wenn der Aufrufer keinen festen Zeitstempel
//! uebergibt (Tests liefern immer einen festen `ts`, damit Content-Adressen und
//! Log-Zeilen deterministisch bleiben).
//!
//! Bis L1 (2026-10-12) stellte das Feature `festzeit` die Uhr auf `TAXGRAPH_JETZT`, damit der
//! Differenz-Harness `rust/parity` in beiden Servern gleiche Zeitstempel sah. Mit `rust/parity`
//! ist das Feature gefallen (Verlauf: `git show 37269912:rust/store/src/zeit.rs`).
#[must_use]
pub(crate) fn jetzt_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Der Zeitstempel eines neuen Eintrags: ein leerer Text heisst "fehlt" und wird zur Jetzt-Zeit, wie Pythons
/// `ts or _now()` (`store.py` `append_event` und `erzeuge_snapshot`; Entscheidung
/// leerer-zeitstempel-heisst-fehlt-und-wird-die-jetzt-zeit). Ob der Text ein Zeitstempel IST, prueft niemand:
/// Python nimmt `' '` und `'banane'` an, und das bleibt so.
#[must_use]
pub(crate) fn ts_oder_jetzt(ts: Option<String>) -> String {
    ts.filter(|t| !t.is_empty()).unwrap_or_else(jetzt_iso)
}
