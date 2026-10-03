//! Ein einziger Zeitstempel-Helfer fuer `store`/`audit`/`fehler_log` — alle drei brauchen
//! `_now()` (`store.py:41-42`) nur als DEFAULT, wenn der Aufrufer keinen festen Zeitstempel
//! uebergibt (Tests/Parity liefern immer einen festen `ts`, damit Content-Adressen und
//! Log-Zeilen deterministisch bleiben).
#[must_use]
pub(crate) fn jetzt_iso() -> String {
    // Nur im Testbau (`--features festzeit`): die Uhr steht auf `TAXGRAPH_JETZT`. Der
    // Differenz-Harness (`api_http_paritaet`) braucht gleiche Zeitstempel in beiden Servern, denn
    // jedes abgeleitete Event traegt `_now()` und damit eine Kennung, die an der Zeit haengt.
    #[cfg(feature = "festzeit")]
    if let Ok(fest) = std::env::var("TAXGRAPH_JETZT") {
        return fest;
    }
    chrono::Utc::now().to_rfc3339()
}
