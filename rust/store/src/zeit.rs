//! Ein einziger Zeitstempel-Helfer fuer `store`/`audit`/`fehler_log` — alle drei brauchen
//! `_now()` (`store.py:41-42`) nur als DEFAULT, wenn der Aufrufer keinen festen Zeitstempel
//! uebergibt (Tests/Parity liefern immer einen festen `ts`, damit Content-Adressen und
//! Log-Zeilen deterministisch bleiben).
#[must_use]
pub(crate) fn jetzt_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}
