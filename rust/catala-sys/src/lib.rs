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

/// Ein Catala-Scope-Aufruf ist gescheitert.
///
/// `Assertion` bildet `TG_ERR_CATALA` ab (eine `assert`-Verletzung im Regeltext, z. B.
/// `rules/estg/p32a/einkommensteuertarif.catala_en`). `UngueltigerVzCode` bildet `TG_ERR_VZ` ab
/// -- praktisch unerreichbar ueber diese Crate, weil jeder Aufrufer bereits ein gueltiges
/// `Vz`-Enum-Mitglied uebergeben muss, aber `tg_vz()` auf der C-Seite bleibt als zweite,
/// unabhaengige Bounds-Pruefung bestehen (Verteidigung in der Tiefe, kein Vertrauen auf eine
/// einzige Schicht).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CatalaFehler {
    #[error("Catala-Scope-Aufruf fehlgeschlagen (Laufzeitfehler im Regeltext)")]
    Assertion,
    #[error("ungueltiger Veranlagungszeitraum-Code: {0}")]
    UngueltigerVzCode(i32),
}

/// Wertet den Rueckgabecode eines `tg_*`-Wrappers aus: `0` -> Ok, `2` -> ungueltiger VZ-Code
/// (siehe [`CatalaFehler`]), alles andere -> Catala-Assertion.
fn ergebnis<T>(rc: i32, wert: T, vz_code: i32) -> Result<T, CatalaFehler> {
    match rc {
        0 => Ok(wert),
        2 => Err(CatalaFehler::UngueltigerVzCode(vz_code)),
        _ => Err(CatalaFehler::Assertion),
    }
}

/// Wie [`ergebnis`], fuer Scopes ohne `Veranlagungszeitraum`-Parameter (der `TG_ERR_VZ`-Zweig
/// ist fuer diese Scopes unerreichbar, weil ihr `tg_*`-Wrapper `tg_vz()` nie aufruft).
fn ergebnis_ohne_vz<T>(rc: i32, wert: T) -> Result<T, CatalaFehler> {
    if rc == 0 {
        Ok(wert)
    } else {
        Err(CatalaFehler::Assertion)
    }
}

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

    fn tg_spenden_abzug(zuwendungen_cents: i64, gde_cents: i64, out_cents: *mut i64) -> i32;
    fn tg_zumutbare_belastung(
        gde_cents: i64,
        anzahl_kinder: i64,
        splitting: i32,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_agb_abzug(agb_cents: i64, zumutbare_belastung_cents: i64, out_cents: *mut i64) -> i32;
    fn tg_kirchensteuerabzug(gezahlt_cents: i64, erstattet_cents: i64, out_cents: *mut i64) -> i32;
    fn tg_altersentlastungsbetrag(
        arbeitslohn_cents: i64,
        positive_andere_cents: i64,
        prozentsatz_num: i64,
        prozentsatz_den: u64,
        hoechstbetrag_cents: i64,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_entlastungsbetrag(
        alleinstehend: i32,
        anzahl_kinder: i64,
        monate_ohne_voraussetzung: i64,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_familienleistungsausgleich(
        est_ohne_cents: i64,
        est_mit_cents: i64,
        kindergeld_cents: i64,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_verbilligte_vermietung_wk(
        werbungskosten_cents: i64,
        entgelt_quote_num: i64,
        entgelt_quote_den: u64,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_kranken_pflege_vorsorge(
        basis_cents: i64,
        weitere_cents: i64,
        mit_zuschuss: i32,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_berufsausbildung(aufwendungen_cents: i64, out_cents: *mut i64) -> i32;
    fn tg_betriebs_freibetrag(veraeusserungsgewinn_cents: i64, out_cents: *mut i64) -> i32;
    fn tg_euer_gewinn(
        betriebseinnahmen_cents: i64,
        betriebsausgaben_cents: i64,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_mitunternehmer_einkuenfte(
        gewinnanteil_cents: i64,
        verguetung_taetigkeit_cents: i64,
        verguetung_darlehen_cents: i64,
        verguetung_ueberlassung_cents: i64,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_gwg_sofortabzug(anschaffungskosten_netto_cents: i64, out_cents: *mut i64) -> i32;
    fn tg_verlustvortrag_abzug(
        gde_cents: i64,
        bestand_cents: i64,
        zusammenveranlagung: i32,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_ermaessigter_durchschnittssatz(
        ao_cents: i64,
        est_gesamt_zzgl_progression_cents: i64,
        bemessungsgrundlage_durchschnitt_cents: i64,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_entfernungspauschale(
        eingabe: *const TgEntfernungspauschaleInFfi,
        ausgabe: *mut TgEntfernungspauschaleOutFfi,
    ) -> i32;
    fn tg_raumkostenabzug(
        eingabe: *const TgRaumkostenabzugInFfi,
        ausgabe: *mut TgRaumkostenabzugOutFfi,
    ) -> i32;
    fn tg_festzusetzende_est_einzel_voll(
        bruttoarbeitslohn_cents: i64,
        werbungskosten_cents: i64,
        sonderausgaben_cents: i64,
        vz_code: i32,
        ausgabe: *mut TgEstOutFfi,
    ) -> i32;
    fn tg_festzusetzende_est_zusammen(
        bruttoarbeitslohn_a_cents: i64,
        werbungskosten_a_cents: i64,
        bruttoarbeitslohn_b_cents: i64,
        werbungskosten_b_cents: i64,
        sonderausgaben_gemeinsam_cents: i64,
        vz_code: i32,
        out_cents: *mut i64,
    ) -> i32;
    fn tg_festzusetzende_est_gesamt(
        eingabe: *const TgGesamtInFfi,
        vz_code: i32,
        ausgabe: *mut TgEstOutFfi,
    ) -> i32;
    fn tg_festzusetzende_est_gesamt_zusammen(
        eingabe: *const TgGesamtInFfi,
        vz_code: i32,
        ausgabe: *mut TgEstOutFfi,
    ) -> i32;
}

// -- repr(C)-Spiegel der `shim.c`-Structs. Feldreihenfolge und -typen MUESSEN mit `shim.c`
// uebereinstimmen (`long` -> `i64`, `unsigned long` -> `u64`, `int` -> `i32`); `repr(C)`
// wendet dann dieselben Layout-/Padding-Regeln wie der C-Compiler an. Rein mechanische
// FFI-Buendelung, keine Fachlogik -- die Fachtypisierung (`Cent`/`Euro`, PARITAET-Kommentare
// zu Python-Fail-Open-Defaults) lebt in der `engine`-Crate, nicht hier.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct TgEntfernungspauschaleInFfi {
    entfernung_km_roh_num: i64,
    entfernung_km_roh_den: u64,
    arbeitstage: i64,
    eigenes_oder_ueberlassenes_kfz: i32,
    oepnv_kosten_jahr_cents: i64,
    satz_bis_20_km_cents: i64,
    satz_ab_21_km_cents: i64,
    staffelgrenze_km: i64,
    hoechstbetrag_cents: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct TgEntfernungspauschaleOutFfi {
    entfernungspauschale_cents: i64,
    abziehbarer_betrag_cents: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct TgRaumkostenabzugInFfi {
    arbeitszimmer_vorhanden: i32,
    ist_mittelpunkt: i32,
    tatsaechliche_aufwendungen_cents: i64,
    jahrespauschale_gewaehlt: i32,
    monate_ohne_mittelpunkt: i64,
    homeoffice_tage: i64,
    jahrespauschale_cents: i64,
    tagespauschale_pro_tag_cents: i64,
    tagespauschale_hoechstbetrag_cents: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
// Feldnamen spiegeln shim.c 1:1; das gemeinsame Praefix ist die C-Struct-Konvention.
#[allow(clippy::struct_field_names)]
struct TgRaumkostenabzugOutFfi {
    abzug_arbeitszimmer_cents: i64,
    abzug_homeoffice_cents: i64,
    abzug_gesamt_cents: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
// Feldnamen spiegeln shim.c 1:1; das gemeinsame Suffix ist die C-Struct-Konvention.
#[allow(clippy::struct_field_names)]
struct TgEstOutFfi {
    summe_der_einkuenfte_cents: i64,
    gesamtbetrag_der_einkuenfte_cents: i64,
    einkommen_cents: i64,
    zu_versteuerndes_einkommen_cents: i64,
    tarifliche_est_cents: i64,
    festzusetzende_est_cents: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct TgGesamtInFfi {
    einkuenfte_nichtselbststaendig_cents: i64,
    einkuenfte_kapitalvermoegen_cents: i64,
    einkuenfte_vermietung_cents: i64,
    einkuenfte_sonstige_cents: i64,
    einkuenfte_gewinn_cents: i64,
    altersentlastungsbetrag_cents: i64,
    entlastungsbetrag_alleinerziehende_cents: i64,
    sonderausgaben_cents: i64,
    aussergewoehnliche_belastungen_cents: i64,
    freibetraege_kinder_cents: i64,
    sonstige_abzuege_vom_einkommen_cents: i64,
    anzurechnende_auslaendische_steuern_cents: i64,
    steuerermaessigungen_cents: i64,
    steuer_kapital_gesondert_cents: i64,
    hinzurechnung_kindergeld_cents: i64,
    hinzurechnung_zulage_cents: i64,
    tarif_modifiziert: i32,
    tarifliche_est_modifiziert_cents: i64,
}

/// Eingabe fuer [`entfernungspauschale`] (§ 9 Abs. 1 S. 3 Nr. 4/4a `EStG`). `entfernung_km_roh`
/// ist im Catala-Scope `CATALA_DEC`, keine ganze Zahl -- deshalb Zaehler/Nenner statt eines
/// einzelnen Felds (Python uebergibt eine `Decimal`, `Nenner` ist bei ganzen km meist `1`).
#[derive(Debug, Clone, Copy)]
pub struct EntfernungspauschaleEingabe {
    pub entfernung_km_roh_num: i64,
    pub entfernung_km_roh_den: u64,
    pub arbeitstage: i64,
    pub eigenes_oder_ueberlassenes_kfz: bool,
    pub oepnv_kosten_jahr_cent: i64,
    pub satz_bis_20_km_cent: i64,
    pub satz_ab_21_km_cent: i64,
    pub staffelgrenze_km: i64,
    pub hoechstbetrag_cent: i64,
}

/// Ergebnis von [`entfernungspauschale`]: `entfernungspauschale_cent` VOR dem
/// Hoechstbetragsdeckel, `abziehbarer_betrag_cent` NACH dem Deckel (Python liest nur
/// Letzteres in `catala_entfernungspauschale`/`catala_ep_ab_21km`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntfernungspauschaleErgebnis {
    pub entfernungspauschale_cent: i64,
    pub abziehbarer_betrag_cent: i64,
}

/// Eingabe fuer [`raumkostenabzug`] (§ 4 Abs. 5 Nr. 6b/6c `EStG`, haeusliches Arbeitszimmer und
/// Homeoffice-Tagespauschale).
#[derive(Debug, Clone, Copy)]
pub struct RaumkostenabzugEingabe {
    pub arbeitszimmer_vorhanden: bool,
    pub ist_mittelpunkt: bool,
    pub tatsaechliche_aufwendungen_cent: i64,
    pub jahrespauschale_gewaehlt: bool,
    pub monate_ohne_mittelpunkt: i64,
    pub homeoffice_tage: i64,
    pub jahrespauschale_cent: i64,
    pub tagespauschale_pro_tag_cent: i64,
    pub tagespauschale_hoechstbetrag_cent: i64,
}

/// Ergebnis von [`raumkostenabzug`]: Arbeitszimmer- und Homeoffice-Abzug getrennt, plus Summe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaumkostenabzugErgebnis {
    pub abzug_arbeitszimmer_cent: i64,
    pub abzug_homeoffice_cent: i64,
    pub abzug_gesamt_cent: i64,
}

/// Eingabe fuer [`festzusetzende_est_gesamt`]/[`festzusetzende_est_gesamt_zusammen`]: die 16
/// Money-/Bool-Felder, die `FestzusetzendeEstGesamt_in` und `FestzusetzendeEstGesamtZusammen_in`
/// TEILEN (identische Feldliste in `Einkommensteuertarif.h`) -- welcher der beiden Catala-Scopes
/// laeuft, entscheidet einzig, welche der beiden Funktionen aufgerufen wird.
#[derive(Debug, Clone, Copy)]
pub struct GesamtEingabe {
    pub einkuenfte_nichtselbststaendig_cent: i64,
    pub einkuenfte_kapitalvermoegen_cent: i64,
    pub einkuenfte_vermietung_cent: i64,
    pub einkuenfte_sonstige_cent: i64,
    pub einkuenfte_gewinn_cent: i64,
    pub altersentlastungsbetrag_cent: i64,
    pub entlastungsbetrag_alleinerziehende_cent: i64,
    pub sonderausgaben_cent: i64,
    pub aussergewoehnliche_belastungen_cent: i64,
    pub freibetraege_kinder_cent: i64,
    pub sonstige_abzuege_vom_einkommen_cent: i64,
    pub anzurechnende_auslaendische_steuern_cent: i64,
    pub steuerermaessigungen_cent: i64,
    pub steuer_kapital_gesondert_cent: i64,
    pub hinzurechnung_kindergeld_cent: i64,
    pub hinzurechnung_zulage_cent: i64,
    pub tarif_modifiziert: bool,
    pub tarifliche_est_modifiziert_cent: i64,
}

impl GesamtEingabe {
    fn zu_ffi(self) -> TgGesamtInFfi {
        TgGesamtInFfi {
            einkuenfte_nichtselbststaendig_cents: self.einkuenfte_nichtselbststaendig_cent,
            einkuenfte_kapitalvermoegen_cents: self.einkuenfte_kapitalvermoegen_cent,
            einkuenfte_vermietung_cents: self.einkuenfte_vermietung_cent,
            einkuenfte_sonstige_cents: self.einkuenfte_sonstige_cent,
            einkuenfte_gewinn_cents: self.einkuenfte_gewinn_cent,
            altersentlastungsbetrag_cents: self.altersentlastungsbetrag_cent,
            entlastungsbetrag_alleinerziehende_cents: self.entlastungsbetrag_alleinerziehende_cent,
            sonderausgaben_cents: self.sonderausgaben_cent,
            aussergewoehnliche_belastungen_cents: self.aussergewoehnliche_belastungen_cent,
            freibetraege_kinder_cents: self.freibetraege_kinder_cent,
            sonstige_abzuege_vom_einkommen_cents: self.sonstige_abzuege_vom_einkommen_cent,
            anzurechnende_auslaendische_steuern_cents: self
                .anzurechnende_auslaendische_steuern_cent,
            steuerermaessigungen_cents: self.steuerermaessigungen_cent,
            steuer_kapital_gesondert_cents: self.steuer_kapital_gesondert_cent,
            hinzurechnung_kindergeld_cents: self.hinzurechnung_kindergeld_cent,
            hinzurechnung_zulage_cents: self.hinzurechnung_zulage_cent,
            tarif_modifiziert: i32::from(self.tarif_modifiziert),
            tarifliche_est_modifiziert_cents: self.tarifliche_est_modifiziert_cent,
        }
    }
}

/// Ergebnis der Einkommensteuertarif-Scopes mit mehreren Ausgabefeldern
/// (`FestzusetzendeEstEinzel`/`FestzusetzendeEstGesamt`/`FestzusetzendeEstGesamtZusammen`
/// teilen dieselben sechs Feldnamen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FestzusetzendeEstErgebnis {
    pub summe_der_einkuenfte_cent: i64,
    pub gesamtbetrag_der_einkuenfte_cent: i64,
    pub einkommen_cent: i64,
    pub zu_versteuerndes_einkommen_cent: i64,
    pub tarifliche_est_cent: i64,
    pub festzusetzende_est_cent: i64,
}

impl From<TgEstOutFfi> for FestzusetzendeEstErgebnis {
    fn from(ffi: TgEstOutFfi) -> Self {
        Self {
            summe_der_einkuenfte_cent: ffi.summe_der_einkuenfte_cents,
            gesamtbetrag_der_einkuenfte_cent: ffi.gesamtbetrag_der_einkuenfte_cents,
            einkommen_cent: ffi.einkommen_cents,
            zu_versteuerndes_einkommen_cent: ffi.zu_versteuerndes_einkommen_cents,
            tarifliche_est_cent: ffi.tarifliche_est_cents,
            festzusetzende_est_cent: ffi.festzusetzende_est_cents,
        }
    }
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
///
/// ```
/// use catala_sys::Vz;
/// use catala_sys::grundtarif;
/// assert_eq!(grundtarif(0, Vz::Vz2025).unwrap(), 0); // unter dem Grundfreibetrag
/// assert!(grundtarif(6_000_000, Vz::Vz2025).unwrap() > grundtarif(5_000_000, Vz::Vz2025).unwrap());
/// ```
pub fn grundtarif(zve_cent: i64, vz: Vz) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: `out` ist ein gueltiger, alignierter `*mut i64` fuer die Dauer des Aufrufs;
        // `tg_grundtarif` schreibt nur hindurch, wenn es 0 zurueckgibt. Kein Zeiger ueberlebt
        // den Aufruf.
        let rc = unsafe { tg_grundtarif(zve_cent, vz as i32, &raw mut out) };
        ergebnis(rc, out, vz as i32)
    })
}

/// § 32a Abs. 5 `EStG` Splittingtarif: gemeinsames zvE in Cent auf die tarifliche
/// Einkommensteuer, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::Vz;
/// use catala_sys::{grundtarif, splittingtarif};
/// assert_eq!(splittingtarif(10_000_000, Vz::Vz2025).unwrap(), 2 * grundtarif(5_000_000, Vz::Vz2025).unwrap());
/// ```
pub fn splittingtarif(zve_gemeinsam_cent: i64, vz: Vz) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `grundtarif`.
        let rc = unsafe { tg_splittingtarif(zve_gemeinsam_cent, vz as i32, &raw mut out) };
        ergebnis(rc, out, vz as i32)
    })
}

/// End-to-end Arbeitnehmerfall (`FestzusetzendeEstEinzel`): Bruttoarbeitslohn, Werbungskosten
/// und Sonderausgaben in Cent auf die festzusetzende Einkommensteuer, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::Vz;
/// use catala_sys::festzusetzende_est_einzel;
/// assert!(festzusetzende_est_einzel(5_000_000, 0, 0, Vz::Vz2025).unwrap() > 0);
/// ```
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
        ergebnis(rc, out, vz as i32)
    })
}

/// § 10b `EStG` Spendenabzug: Zuwendungen und Gesamtbetrag der Einkuenfte in Cent auf den
/// abziehbaren Spendenabzug, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::spenden_abzug;
/// assert_eq!(spenden_abzug(10_000, 1_000_000).unwrap(), 10_000); // unter dem Deckel voll abziehbar
/// ```
pub fn spenden_abzug(zuwendungen_cent: i64, gde_cent: i64) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: `out` ist ein gueltiger, alignierter `*mut i64` fuer die Dauer des Aufrufs.
        let rc = unsafe { tg_spenden_abzug(zuwendungen_cent, gde_cent, &raw mut out) };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 33 Abs. 3 `EStG` zumutbare Belastung: Gesamtbetrag der Einkuenfte in Cent, Kinderzahl und
/// Splitting-Flag auf die zumutbare Belastung, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::zumutbare_belastung;
/// assert!(zumutbare_belastung(6_000_000, 0, false).unwrap() > zumutbare_belastung(2_000_000, 0, false).unwrap());
/// ```
pub fn zumutbare_belastung(
    gde_cent: i64,
    anzahl_kinder: i64,
    splitting: bool,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe {
            tg_zumutbare_belastung(gde_cent, anzahl_kinder, i32::from(splitting), &raw mut out)
        };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 33 Abs. 1 `EStG` aussergewoehnliche Belastungen abzueglich zumutbarer Belastung, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::agb_abzug;
/// assert_eq!(agb_abzug(100, 500).unwrap(), 0); // unter der zumutbaren Belastung nichts abziehbar
/// ```
pub fn agb_abzug(agb_cent: i64, zumutbare_belastung_cent: i64) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe { tg_agb_abzug(agb_cent, zumutbare_belastung_cent, &raw mut out) };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 10 Abs. 1 Nr. 4 `EStG` abziehbare Kirchensteuer: gezahlte abzueglich erstatteter
/// Kirchensteuer, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::kirchensteuerabzug;
/// assert_eq!(kirchensteuerabzug(1_000, 300).unwrap(), 700); // Erstattung mindert den Abzug
/// ```
pub fn kirchensteuerabzug(gezahlt_cent: i64, erstattet_cent: i64) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe { tg_kirchensteuerabzug(gezahlt_cent, erstattet_cent, &raw mut out) };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 24a `EStG` Altersentlastungsbetrag: Arbeitslohn, positive andere Einkuenfte (jeweils Cent),
/// Prozentsatz als Bruch (Zaehler/Nenner) und Hoechstbetrag (Cent) auf den
/// Altersentlastungsbetrag, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::altersentlastungsbetrag;
/// // Bemessung 3.000 EUR, 20,0 % (200/10), Hoechstbetrag 760 EUR → 600 EUR
/// assert_eq!(altersentlastungsbetrag(200_000, 100_000, 200, 10, 76_000).unwrap(), 60_000);
/// ```
pub fn altersentlastungsbetrag(
    arbeitslohn_cent: i64,
    positive_andere_cent: i64,
    prozentsatz_num: i64,
    prozentsatz_den: u64,
    hoechstbetrag_cent: i64,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe {
            tg_altersentlastungsbetrag(
                arbeitslohn_cent,
                positive_andere_cent,
                prozentsatz_num,
                prozentsatz_den,
                hoechstbetrag_cent,
                &raw mut out,
            )
        };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 24b `EStG` Entlastungsbetrag fuer Alleinerziehende: Alleinstehend-Flag, Kinderzahl und Monate
/// ohne Voraussetzung auf den Entlastungsbetrag, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::entlastungsbetrag;
/// assert_eq!(entlastungsbetrag(false, 2, 0).unwrap(), 0); // nicht alleinstehend: kein Anspruch
/// ```
pub fn entlastungsbetrag(
    alleinstehend: bool,
    anzahl_kinder: i64,
    monate_ohne_voraussetzung: i64,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe {
            tg_entlastungsbetrag(
                i32::from(alleinstehend),
                anzahl_kinder,
                monate_ohne_voraussetzung,
                &raw mut out,
            )
        };
        ergebnis_ohne_vz(rc, out)
    })
}

/// §§ 31, 32 Abs. 6 `EStG` Familienleistungsausgleich (Guenstigerpruefung Kindergeld vs.
/// Kinderfreibetrag): `ESt` ohne/mit Freibetraegen und Kindergeld (jeweils Cent) auf die `ESt` nach
/// Familienleistungsausgleich, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::familienleistungsausgleich;
/// // Freibetrag guenstiger: mit-Steuer plus Kindergeld
/// assert_eq!(familienleistungsausgleich(1_000_000, 900_000, 30_000).unwrap(), 930_000);
/// ```
pub fn familienleistungsausgleich(
    est_ohne_cent: i64,
    est_mit_cent: i64,
    kindergeld_cent: i64,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe {
            tg_familienleistungsausgleich(
                est_ohne_cent,
                est_mit_cent,
                kindergeld_cent,
                &raw mut out,
            )
        };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 21 Abs. 2 `EStG` verbilligte Vermietung: Werbungskosten (Cent) und Entgeltquote (Bruch
/// Zaehler/Nenner in Prozent) auf die abziehbaren Werbungskosten, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::verbilligte_vermietung_wk;
/// assert_eq!(verbilligte_vermietung_wk(100_000, 100, 1).unwrap(), 100_000); // 100 % Entgelt: voll abziehbar
/// ```
pub fn verbilligte_vermietung_wk(
    werbungskosten_cent: i64,
    entgelt_quote_num: i64,
    entgelt_quote_den: u64,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe {
            tg_verbilligte_vermietung_wk(
                werbungskosten_cent,
                entgelt_quote_num,
                entgelt_quote_den,
                &raw mut out,
            )
        };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 10 Abs. 1 Nr. 3 `EStG` Basiskranken- und Pflegeversicherung: Basisbeitrag, weitere
/// Vorsorgeaufwendungen (jeweils Cent) und Zuschuss-Anspruch-Flag auf die abziehbare
/// Kranken-/Pflegevorsorge, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::kranken_pflege_vorsorge;
/// assert_eq!(kranken_pflege_vorsorge(50_000, 0, false).unwrap(), 50_000);
/// ```
pub fn kranken_pflege_vorsorge(
    basis_cent: i64,
    weitere_cent: i64,
    mit_zuschuss: bool,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe {
            tg_kranken_pflege_vorsorge(
                basis_cent,
                weitere_cent,
                i32::from(mit_zuschuss),
                &raw mut out,
            )
        };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 10 Abs. 1 Nr. 7 `EStG` Berufsausbildungsaufwendungen: Aufwendungen in Cent auf die
/// abziehbaren Sonderausgaben, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::berufsausbildung;
/// assert_eq!(berufsausbildung(10_000).unwrap(), 10_000); // unter dem Hoechstbetrag voll abziehbar
/// ```
pub fn berufsausbildung(aufwendungen_cent: i64) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe { tg_berufsausbildung(aufwendungen_cent, &raw mut out) };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 16 Abs. 4 `EStG` Freibetrag fuer Betriebsveraeusserung: Veraeusserungsgewinn in Cent auf den
/// Freibetrag, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::betriebs_freibetrag;
/// assert_eq!(betriebs_freibetrag(16_000_000).unwrap(), 2_100_000); // Teilabschmelzung ueber 136.000 EUR
/// ```
pub fn betriebs_freibetrag(veraeusserungsgewinn_cent: i64) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe { tg_betriebs_freibetrag(veraeusserungsgewinn_cent, &raw mut out) };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 4 Abs. 3 `EStG` Einnahmenueberschussrechnung: Betriebseinnahmen abzueglich
/// Betriebsausgaben (jeweils Cent) auf den Gewinn, in Cent. Kann negativ sein (Verlust).
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::euer_gewinn;
/// assert_eq!(euer_gewinn(3_000_000, 5_000_000).unwrap(), -2_000_000); // Ausgaben ueber Einnahmen: Verlust
/// ```
pub fn euer_gewinn(
    betriebseinnahmen_cent: i64,
    betriebsausgaben_cent: i64,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc =
            unsafe { tg_euer_gewinn(betriebseinnahmen_cent, betriebsausgaben_cent, &raw mut out) };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 15 Abs. 1 S. 1 Nr. 2 `EStG` Mitunternehmereinkuenfte: Gewinnanteil und die drei
/// Sondervergueltungen (Taetigkeit, Darlehen, Ueberlassung; jeweils Cent) auf die
/// Mitunternehmereinkuenfte, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::mitunternehmer_einkuenfte;
/// assert_eq!(mitunternehmer_einkuenfte(1_000_000, 1_200_000, 300_000, 500_000).unwrap(), 3_000_000);
/// ```
pub fn mitunternehmer_einkuenfte(
    gewinnanteil_cent: i64,
    verguetung_taetigkeit_cent: i64,
    verguetung_darlehen_cent: i64,
    verguetung_ueberlassung_cent: i64,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe {
            tg_mitunternehmer_einkuenfte(
                gewinnanteil_cent,
                verguetung_taetigkeit_cent,
                verguetung_darlehen_cent,
                verguetung_ueberlassung_cent,
                &raw mut out,
            )
        };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 6 Abs. 2 `EStG` GWG-Sofortabzug: Anschaffungskosten netto in Cent auf den Sofortabzug, in
/// Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::gwg_sofortabzug;
/// assert_eq!(gwg_sofortabzug(80_100).unwrap(), 0); // ueber 800 EUR kein Sofortabzug
/// ```
pub fn gwg_sofortabzug(anschaffungskosten_netto_cent: i64) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe { tg_gwg_sofortabzug(anschaffungskosten_netto_cent, &raw mut out) };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 10d `EStG` Verlustvortrag: Gesamtbetrag der Einkuenfte, Verlustvortragsbestand (jeweils
/// Cent) und Zusammenveranlagungs-Flag auf den Verlustabzug, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::verlustvortrag_abzug;
/// assert_eq!(verlustvortrag_abzug(5_000_000, 6_000_000, false).unwrap(), 5_000_000); // auf das GdE gekappt
/// ```
pub fn verlustvortrag_abzug(
    gde_cent: i64,
    bestand_cent: i64,
    zusammenveranlagung: bool,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe {
            tg_verlustvortrag_abzug(
                gde_cent,
                bestand_cent,
                i32::from(zusammenveranlagung),
                &raw mut out,
            )
        };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 34 Abs. 1 `EStG` ermaessigter Durchschnittssatz (Fuenftelregelung fuer aussergewoehnliche
/// Einkuenfte): ausserordentliche Einkuenfte, `ESt` gesamt zzgl. Progression und
/// Bemessungsgrundlage Durchschnitt (jeweils Cent) auf die `ESt` auf die ausserordentlichen
/// Einkuenfte, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::ermaessigter_durchschnittssatz;
/// assert_eq!(ermaessigter_durchschnittssatz(10_000_000, 20_000_000, 100_000_000).unwrap(), 1_400_000);
/// ```
pub fn ermaessigter_durchschnittssatz(
    ao_cent: i64,
    est_gesamt_zzgl_progression_cent: i64,
    bemessungsgrundlage_durchschnitt_cent: i64,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `spenden_abzug`.
        let rc = unsafe {
            tg_ermaessigter_durchschnittssatz(
                ao_cent,
                est_gesamt_zzgl_progression_cent,
                bemessungsgrundlage_durchschnitt_cent,
                &raw mut out,
            )
        };
        ergebnis_ohne_vz(rc, out)
    })
}

/// § 9 Abs. 1 S. 3 Nr. 4/4a `EStG` Entfernungspauschale.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::{entfernungspauschale, EntfernungspauschaleEingabe};
/// let e = entfernungspauschale(EntfernungspauschaleEingabe {
///     entfernung_km_roh_num: 106, entfernung_km_roh_den: 10, arbeitstage: 200,
///     eigenes_oder_ueberlassenes_kfz: false, oepnv_kosten_jahr_cent: 0,
///     satz_bis_20_km_cent: 30, satz_ab_21_km_cent: 38, staffelgrenze_km: 20, hoechstbetrag_cent: 450_000,
/// }).unwrap();
/// assert_eq!(e.entfernungspauschale_cent, 60_000); // 10 volle km × 0,30 EUR × 200 Tage
/// ```
pub fn entfernungspauschale(
    eingabe: EntfernungspauschaleEingabe,
) -> Result<EntfernungspauschaleErgebnis, CatalaFehler> {
    locked(|| {
        let ffi_in = TgEntfernungspauschaleInFfi {
            entfernung_km_roh_num: eingabe.entfernung_km_roh_num,
            entfernung_km_roh_den: eingabe.entfernung_km_roh_den,
            arbeitstage: eingabe.arbeitstage,
            eigenes_oder_ueberlassenes_kfz: i32::from(eingabe.eigenes_oder_ueberlassenes_kfz),
            oepnv_kosten_jahr_cents: eingabe.oepnv_kosten_jahr_cent,
            satz_bis_20_km_cents: eingabe.satz_bis_20_km_cent,
            satz_ab_21_km_cents: eingabe.satz_ab_21_km_cent,
            staffelgrenze_km: eingabe.staffelgrenze_km,
            hoechstbetrag_cents: eingabe.hoechstbetrag_cent,
        };
        let mut ffi_out = TgEntfernungspauschaleOutFfi::default();
        // SAFETY: `&ffi_in`/`&mut ffi_out` sind gueltige, alignierte Zeiger auf `repr(C)`-Structs
        // fuer die Dauer des Aufrufs; keiner ueberlebt ihn.
        let rc = unsafe { tg_entfernungspauschale(&raw const ffi_in, &raw mut ffi_out) };
        ergebnis_ohne_vz(
            rc,
            EntfernungspauschaleErgebnis {
                entfernungspauschale_cent: ffi_out.entfernungspauschale_cents,
                abziehbarer_betrag_cent: ffi_out.abziehbarer_betrag_cents,
            },
        )
    })
}

/// § 4 Abs. 5 Nr. 6b/6c `EStG` haeusliches Arbeitszimmer und Homeoffice-Tagespauschale.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::{raumkostenabzug, RaumkostenabzugEingabe};
/// let r = raumkostenabzug(RaumkostenabzugEingabe {
///     arbeitszimmer_vorhanden: false, ist_mittelpunkt: false, tatsaechliche_aufwendungen_cent: 0,
///     jahrespauschale_gewaehlt: false, monate_ohne_mittelpunkt: 0, homeoffice_tage: 120,
///     jahrespauschale_cent: 126_000, tagespauschale_pro_tag_cent: 600, tagespauschale_hoechstbetrag_cent: 126_000,
/// }).unwrap();
/// assert_eq!(r.abzug_gesamt_cent, 72_000); // 120 Tage × 6 EUR
/// ```
pub fn raumkostenabzug(
    eingabe: RaumkostenabzugEingabe,
) -> Result<RaumkostenabzugErgebnis, CatalaFehler> {
    locked(|| {
        let ffi_in = TgRaumkostenabzugInFfi {
            arbeitszimmer_vorhanden: i32::from(eingabe.arbeitszimmer_vorhanden),
            ist_mittelpunkt: i32::from(eingabe.ist_mittelpunkt),
            tatsaechliche_aufwendungen_cents: eingabe.tatsaechliche_aufwendungen_cent,
            jahrespauschale_gewaehlt: i32::from(eingabe.jahrespauschale_gewaehlt),
            monate_ohne_mittelpunkt: eingabe.monate_ohne_mittelpunkt,
            homeoffice_tage: eingabe.homeoffice_tage,
            jahrespauschale_cents: eingabe.jahrespauschale_cent,
            tagespauschale_pro_tag_cents: eingabe.tagespauschale_pro_tag_cent,
            tagespauschale_hoechstbetrag_cents: eingabe.tagespauschale_hoechstbetrag_cent,
        };
        let mut ffi_out = TgRaumkostenabzugOutFfi::default();
        // SAFETY: siehe `entfernungspauschale`.
        let rc = unsafe { tg_raumkostenabzug(&raw const ffi_in, &raw mut ffi_out) };
        ergebnis_ohne_vz(
            rc,
            RaumkostenabzugErgebnis {
                abzug_arbeitszimmer_cent: ffi_out.abzug_arbeitszimmer_cents,
                abzug_homeoffice_cent: ffi_out.abzug_homeoffice_cents,
                abzug_gesamt_cent: ffi_out.abzug_gesamt_cents,
            },
        )
    })
}

/// Wie [`festzusetzende_est_einzel`], gibt aber alle sechs Zwischenergebnisse zurueck (drei
/// verschiedene Python-Aufrufer lesen drei verschiedene Felder desselben Scope-Aufrufs).
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::Vz;
/// use catala_sys::festzusetzende_est_einzel_voll;
/// let v = festzusetzende_est_einzel_voll(5_000_000, 0, 0, Vz::Vz2025).unwrap();
/// assert!(v.zu_versteuerndes_einkommen_cent < 5_000_000 && v.festzusetzende_est_cent > 0);
/// ```
pub fn festzusetzende_est_einzel_voll(
    bruttoarbeitslohn_cent: i64,
    werbungskosten_cent: i64,
    sonderausgaben_cent: i64,
    vz: Vz,
) -> Result<FestzusetzendeEstErgebnis, CatalaFehler> {
    locked(|| {
        let mut ffi_out = TgEstOutFfi::default();
        // SAFETY: siehe `grundtarif`; `&mut ffi_out` ist ein gueltiger, alignierter Zeiger auf
        // ein `repr(C)`-Struct fuer die Dauer des Aufrufs.
        let rc = unsafe {
            tg_festzusetzende_est_einzel_voll(
                bruttoarbeitslohn_cent,
                werbungskosten_cent,
                sonderausgaben_cent,
                vz as i32,
                &raw mut ffi_out,
            )
        };
        ergebnis(rc, FestzusetzendeEstErgebnis::from(ffi_out), vz as i32)
    })
}

/// End-to-end Ehegattenfall (`FestzusetzendeEstZusammen`): Bruttoarbeitslohn und
/// Werbungskosten je Partner sowie gemeinsame Sonderausgaben (jeweils Cent) auf die
/// festzusetzende Einkommensteuer, in Cent.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::Vz;
/// use catala_sys::festzusetzende_est_zusammen;
/// assert!(festzusetzende_est_zusammen(6_000_000, 0, 0, 0, 0, Vz::Vz2025).unwrap() > 0);
/// ```
// Die a/b-Paarung bildet die zwei Ehepartner ab, nicht austauschbare Varianten eines Namens.
#[allow(clippy::similar_names)]
pub fn festzusetzende_est_zusammen(
    bruttoarbeitslohn_a_cent: i64,
    werbungskosten_a_cent: i64,
    bruttoarbeitslohn_b_cent: i64,
    werbungskosten_b_cent: i64,
    sonderausgaben_gemeinsam_cent: i64,
    vz: Vz,
) -> Result<i64, CatalaFehler> {
    locked(|| {
        let mut out: i64 = 0;
        // SAFETY: siehe `grundtarif`.
        let rc = unsafe {
            tg_festzusetzende_est_zusammen(
                bruttoarbeitslohn_a_cent,
                werbungskosten_a_cent,
                bruttoarbeitslohn_b_cent,
                werbungskosten_b_cent,
                sonderausgaben_gemeinsam_cent,
                vz as i32,
                &raw mut out,
            )
        };
        ergebnis(rc, out, vz as i32)
    })
}

/// Veranlagter Gesamtfall, Einzelveranlagung (`FestzusetzendeEstGesamt`): nimmt bereits final
/// berechnete Einkuenfte/Abzuege (siehe [`GesamtEingabe`]) entgegen -- die Vorstufen
/// (Sonderausgaben-Endbetrag, Vorsorgeabzug, Kindergeld-Vergleich) sind Teil-B-Python-Glue und
/// nicht Teil dieser Funktion.
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::festzusetzende_est_gesamt;
/// use catala_sys::{GesamtEingabe, Vz};
/// let eingabe = GesamtEingabe {
///     einkuenfte_nichtselbststaendig_cent: 6_000_000, einkuenfte_kapitalvermoegen_cent: 0,
///     einkuenfte_vermietung_cent: 0, einkuenfte_sonstige_cent: 0, einkuenfte_gewinn_cent: 0,
///     altersentlastungsbetrag_cent: 0, entlastungsbetrag_alleinerziehende_cent: 0, sonderausgaben_cent: 0,
///     aussergewoehnliche_belastungen_cent: 0, freibetraege_kinder_cent: 0,
///     sonstige_abzuege_vom_einkommen_cent: 0, anzurechnende_auslaendische_steuern_cent: 0,
///     steuerermaessigungen_cent: 0, steuer_kapital_gesondert_cent: 0, hinzurechnung_kindergeld_cent: 0,
///     hinzurechnung_zulage_cent: 0, tarif_modifiziert: false, tarifliche_est_modifiziert_cent: 0,
/// };
/// assert!(festzusetzende_est_gesamt(eingabe, Vz::Vz2025).unwrap().festzusetzende_est_cent > 0);
/// ```
pub fn festzusetzende_est_gesamt(
    eingabe: GesamtEingabe,
    vz: Vz,
) -> Result<FestzusetzendeEstErgebnis, CatalaFehler> {
    locked(|| {
        let ffi_in = eingabe.zu_ffi();
        let mut ffi_out = TgEstOutFfi::default();
        // SAFETY: siehe `entfernungspauschale`.
        let rc =
            unsafe { tg_festzusetzende_est_gesamt(&raw const ffi_in, vz as i32, &raw mut ffi_out) };
        ergebnis(rc, FestzusetzendeEstErgebnis::from(ffi_out), vz as i32)
    })
}

/// Wie [`festzusetzende_est_gesamt`], fuer die Zusammenveranlagung
/// (`FestzusetzendeEstGesamtZusammen`) -- identische Eingabefelder, siehe [`GesamtEingabe`].
///
/// # Errors
/// Gibt [`CatalaFehler`] zurueck, wenn der Catala-Scope eine Laufzeit-Assertion verletzt.
///
/// ```
/// use catala_sys::{festzusetzende_est_gesamt, festzusetzende_est_gesamt_zusammen};
/// use catala_sys::{GesamtEingabe, Vz};
/// let eingabe = GesamtEingabe {
///     einkuenfte_nichtselbststaendig_cent: 6_000_000, einkuenfte_kapitalvermoegen_cent: 0,
///     einkuenfte_vermietung_cent: 0, einkuenfte_sonstige_cent: 0, einkuenfte_gewinn_cent: 0,
///     altersentlastungsbetrag_cent: 0, entlastungsbetrag_alleinerziehende_cent: 0, sonderausgaben_cent: 0,
///     aussergewoehnliche_belastungen_cent: 0, freibetraege_kinder_cent: 0,
///     sonstige_abzuege_vom_einkommen_cent: 0, anzurechnende_auslaendische_steuern_cent: 0,
///     steuerermaessigungen_cent: 0, steuer_kapital_gesondert_cent: 0, hinzurechnung_kindergeld_cent: 0,
///     hinzurechnung_zulage_cent: 0, tarif_modifiziert: false, tarifliche_est_modifiziert_cent: 0,
/// };
/// let einzel = festzusetzende_est_gesamt(eingabe, Vz::Vz2025).unwrap();
/// let zusammen = festzusetzende_est_gesamt_zusammen(eingabe, Vz::Vz2025).unwrap();
/// assert!(zusammen.festzusetzende_est_cent <= einzel.festzusetzende_est_cent); // Splittingvorteil
/// ```
pub fn festzusetzende_est_gesamt_zusammen(
    eingabe: GesamtEingabe,
    vz: Vz,
) -> Result<FestzusetzendeEstErgebnis, CatalaFehler> {
    locked(|| {
        let ffi_in = eingabe.zu_ffi();
        let mut ffi_out = TgEstOutFfi::default();
        // SAFETY: siehe `entfernungspauschale`.
        let rc = unsafe {
            tg_festzusetzende_est_gesamt_zusammen(&raw const ffi_in, vz as i32, &raw mut ffi_out)
        };
        ergebnis(rc, FestzusetzendeEstErgebnis::from(ffi_out), vz as i32)
    })
}

/// Sha256 ueber die sortierten `rules/**/*.catala_en`-Inhalte unterhalb von `repo_root`,
/// exakt wie `gen.sh` sie berechnet (`find ... -print0 | sort -z | xargs -0 cat | sha256sum`).
/// Shellt bewusst zu `sha256sum` statt eine zweite Implementierung zu pflegen, die von
/// `gen.sh` abweichen koennte.
///
/// # Errors
/// Reicht I/O-Fehler von `sh`/`find`/`sort`/`sha256sum` durch.
///
/// ```
/// use catala_sys::recompute_source_hash;
/// let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
/// let h = recompute_source_hash(&repo).unwrap();
/// assert_eq!(h.len(), 64); // sha256 in Hex
/// assert!(h.bytes().all(|b| b.is_ascii_hexdigit()));
/// ```
pub fn recompute_source_hash(repo_root: &std::path::Path) -> std::io::Result<String> {
    let script = format!(
        "find {root} -name '*.catala_en' -print0 | sort -z | xargs -0 cat | sha256sum | cut -d' ' -f1",
        root = shell_quote(&repo_root.join("rules")),
    );
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .output()?;
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
        let cent = festzusetzende_est_einzel(50_000 * 100, 0, 0, Vz::Vz2025).unwrap();
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
        assert_eq!(
            hash_a, hash_a_again,
            "gleicher Baum muss gleichen Hash liefern"
        );

        std::fs::write(
            &file,
            "declaration scope A:\n  output x content money\n  # geaendert\n",
        )
        .unwrap();
        let hash_b = recompute_source_hash(&tmp).unwrap();
        assert_ne!(
            hash_a, hash_b,
            "geaenderter Inhalt muss einen anderen Hash liefern"
        );

        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
