//! `catala-sys` — safe Rust wrapper over the Catala-generated C backend for the
//! `Einkommensteuertarif` scopes. `unsafe` lives ONLY in this crate (`REWRITE_PLAN.md` §2.2);
//! every FFI call is wrapped by a safe function with a `// SAFETY:` comment.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

use std::sync::Mutex;

// `Vz` lebt seit Schritt 2 in `domain` (jede hoehere Schicht braucht sie, keine davon braucht
// die C-FFI dieser Crate) -- hier nur re-exportiert, damit bestehende `catala_sys::Vz`-Aufrufer
// unveraendert bleiben.
pub use domain::{UngueltigeVz, Vz};

/// Ein Catala-Scope-Aufruf ist gescheitert (z. B. eine `assert`-Verletzung im Regeltext von
/// `rules/estg/p32a/einkommensteuertarif.catala_en`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Catala-Scope-Aufruf fehlgeschlagen (Laufzeitfehler im Regeltext)")]
pub struct CatalaFehler;

extern "C" {
    // ponytail: `long`/`int` auf der C-Seite werden hier als `i64`/`i32` deklariert, weil
    // diese Crate nur Linux x86_64 (LP64, `c_long == i64`) als Ziel hat; ein Windows-Target
    // braeuchte stattdessen `std::os::raw::c_long`.
    fn tg_grundtarif(zve_cents: i64, vz_code: i32, out_cents: *mut i64) -> i32;
    fn tg_splittingtarif(zve_gemeinsam_cents: i64, vz_code: i32, out_cents: *mut i64) -> i32;
    fn tg_festzusetzende_est_einzel(
        bruttoarbeitslohn_cents: i64,
        werbungskosten_cents: i64,
        sonderausgaben_cents: i64,
        vz_code: i32,
        out_cents: *mut i64,
    ) -> i32;
}

// `catala_init()` biegt GMPs Speicherverwaltung PROZESSWEIT um (`mp_set_memory_functions`);
// die Heap-Arena ist zwar `__thread`, aber ein zweiter gleichzeitiger Aufruf ist nicht
// vermessen. Serialisieren ist die einfache, nachweislich korrekte Loesung.
// ponytail: globaler Lock, Per-Thread-Arena erst wenn Lastmessung ihn als Engpass zeigt
// (REWRITE_PLAN.md §3 "GMP").
static CATALA_LOCK: Mutex<()> = Mutex::new(());

fn locked<T>(f: impl FnOnce() -> T) -> T {
    let _guard = CATALA_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f()
}

/// § 32a Abs. 1 `EStG` Grundtarif: `zve_cent` auf die tarifliche Einkommensteuer, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn grundtarif(zve_cent: i64, vz: Vz) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: `out` ist ein gueltiger, alignierter `*mut i64` fuer die Dauer des Aufrufs;
        // `tg_grundtarif` schreibt nur hindurch, wenn es 0 zurueckgibt. Kein Zeiger ueberlebt
        // den Aufruf.
        let rc = unsafe { tg_grundtarif(zve_cent, vz as i32, &raw mut out) };
        if rc == 0 {
            Ok(out)
        } else {
            Err(CatalaFehler)
        }
    })
}

/// § 32a Abs. 5 `EStG` Splittingtarif: gemeinsames zvE in Cent auf die tarifliche
/// Einkommensteuer, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn splittingtarif(zve_gemeinsam_cent: i64, vz: Vz) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `grundtarif`.
        let rc = unsafe { tg_splittingtarif(zve_gemeinsam_cent, vz as i32, &raw mut out) };
        if rc == 0 {
            Ok(out)
        } else {
            Err(CatalaFehler)
        }
    })
}

/// End-to-end Arbeitnehmerfall (`FestzusetzendeEstEinzel`): Bruttoarbeitslohn, Werbungskosten
/// und Sonderausgaben in Cent auf die festzusetzende Einkommensteuer, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
pub fn festzusetzende_est_einzel(
    bruttoarbeitslohn_cent: i64,
    werbungskosten_cent: i64,
    sonderausgaben_cent: i64,
    vz: Vz,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `grundtarif`.
        let rc = unsafe {
            tg_festzusetzende_est_einzel(
                bruttoarbeitslohn_cent,
                werbungskosten_cent,
                sonderausgaben_cent,
                vz as i32,
                &raw mut out,
            )
        };
        if rc == 0 {
            Ok(out)
        } else {
            Err(CatalaFehler)
        }
    })
}

/// Sha256 ueber die sortierten `rules/**/*.catala_en`-Inhalte unterhalb von `repo_root`,
/// exakt wie `gen.sh` sie berechnet (`find ... -print0 | sort -z | xargs -0 cat | sha256sum`).
/// Shellt bewusst zu `sha256sum` statt eine zweite Implementierung zu pflegen, die von
/// `gen.sh` abweichen koennte.
///
/// # Errors
/// Reicht I/O-Fehler von `sh`/`find`/`sort`/`sha256sum` durch.
pub fn recompute_source_hash(repo_root: &std::path::Path) -> std::io::Result<String> {
    let script = format!(
        "find {root} -name '*.catala_en' -print0 | sort -z | xargs -0 cat | sha256sum | cut -d' ' -f1",
        root = shell_quote(&repo_root.join("rules")),
    );
    let output = std::process::Command::new("sh").arg("-c").arg(script).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn shell_quote(path: &std::path::Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::{festzusetzende_est_einzel, grundtarif, recompute_source_hash, splittingtarif, Vz};

    #[test]
    fn grundtarif_am_grundfreibetrag_ist_steuerfrei() {
        // VZ 2025 Grundfreibetrag 12.096 EUR (§ 32a Abs. 1 Nr. 1) -> 0 Cent Steuer.
        let cent = grundtarif(12_096 * 100, Vz::Vz2025).unwrap();
        assert_eq!(cent, 0);
    }

    #[test]
    fn splittingtarif_verdoppelt_die_haelfte() {
        let cent = splittingtarif(200_000 * 100, Vz::Vz2025).unwrap();
        assert!(cent > 0);
    }

    #[test]
    fn festzusetzende_est_einzel_laeuft_durch() {
        let cent =
            festzusetzende_est_einzel(50_000 * 100, 0, 0, Vz::Vz2025).unwrap();
        assert!(cent > 0);
    }

    #[test]
    fn ungueltige_vz_wird_nicht_zu_vz() {
        assert!(Vz::try_from(2023u16).is_err());
        assert_eq!(Vz::try_from(2025u16).unwrap(), Vz::Vz2025);
    }

    #[test]
    fn generated_c_stimmt_mit_source_hash_ueberein() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.join("../..");
        let want = recompute_source_hash(&repo_root).expect("sha256sum verfuegbar");
        let got = std::fs::read_to_string(manifest_dir.join("generated/SOURCE_HASH"))
            .expect("generated/SOURCE_HASH fehlt -- `make catala-c` gelaufen?");
        assert_eq!(
            want,
            got.trim(),
            "generated/ ist veraltet gegen rules/ -- `make catala-c` erneut laufen lassen"
        );
    }

    /// Beweist die Mechanik selbst, ohne rules/ anzufassen: zwei verschiedene Baeume liefern
    /// verschiedene Hashes, derselbe Baum zweimal denselben.
    #[test]
    fn hash_aendert_sich_mit_dem_inhalt() {
        let tmp = std::env::temp_dir().join(format!("catala-sys-hash-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("rules/estg/p_test")).unwrap();
        let file = tmp.join("rules/estg/p_test/a.catala_en");

        std::fs::write(&file, "declaration scope A:\n  output x content money\n").unwrap();
        let hash_a = recompute_source_hash(&tmp).unwrap();
        let hash_a_again = recompute_source_hash(&tmp).unwrap();
        assert_eq!(hash_a, hash_a_again, "gleicher Baum muss gleichen Hash liefern");

        std::fs::write(&file, "declaration scope A:\n  output x content money\n  # geaendert\n")
            .unwrap();
        let hash_b = recompute_source_hash(&tmp).unwrap();
        assert_ne!(hash_a, hash_b, "geaenderter Inhalt muss einen anderen Hash liefern");

        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
