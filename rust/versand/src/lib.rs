//! `versand` — der Echtversand an ELSTER (`EricBearbeiteVorgang` mit `ERIC_VALIDIERE | ERIC_SENDE`),
//! das Rust-Gegenstueck zu `elster/versand.py` (Julius hat den Rust-Weg am 2026-10-06 freigegeben).
//!
//! NIE automatisch ausgeloest. Der Versand ist eine ausgehende, unwiderrufliche Aktion an eine
//! Behoerde. Darum liegt er in einer eigenen Crate, an der kein anderer Crate haengt (der Dienst
//! `api` kann ihn nicht erreichen), und nur das Programm `taxgraph-versand` ruft [`sende`] auf. Auch
//! kein Test sendet real: alle laufen gegen die Attrappe von `libericapi.so`. Ausloesen ist Julius'
//! Aufgabe, siehe `elster/VERSAND.md`.
//!
//! Sicherheitsmodell, jede Stufe bricht VOR dem naechsten ERiC-Aufruf ab:
//! 1. Echtversand verlangt die woertliche Freigabe [`ECHTVERSAND_FREIGABE`] (kein Bool, kein
//!    Default): [`VersandFehler::EchtversandOhneFreigabe`].
//! 2. Das XML wird gegen den behaupteten Modus geprueft ([`pruefe_merker_konsistenz`]):
//!    [`VersandFehler::XmlMerkerMismatch`].
//! 3. Ohne Zertifikat (Pfad, Datei, PIN) bricht [`sende`] ab, BEVOR ERiC geladen wird:
//!    [`VersandFehler::ZertifikatFehlt`].
//! 4. Zertifikatspfad und PIN stehen nie in einem Fehlertext, einer `Debug`-Ausgabe oder einer
//!    Zusammenfassung.
//!
//! Erfolg heisst `rc == 0` UND eine Telenummer ([`Antwort::erfolg`]).
#![deny(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

mod cli;
mod ffi;
mod merker;

use std::ffi::CString;
use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub use cli::{lauf, Umgebung};
pub use elster::{EricFehler, ERIC_VALIDIERE};
pub use merker::{merker_im_xml, pruefe_merker_konsistenz};

/// `ERIC_SENDE` (`eric_types.h`, `eric_bearbeitung_flag_t`): der Datensatz geht an den
/// ELSTER-Annahmeserver. Das einzige Vorkommen im Repo (ein Waechter prueft das).
pub const ERIC_SENDE: u32 = 1 << 2;

/// Amtlich `ERIC_TESTMERKER_CLEARINGSTELLE` (`ericdef.h`): die Faelle werden in der Clearingstelle
/// aussortiert und verworfen, es findet keine Verarbeitung im Finanzamt statt.
pub const TESTMERKER: &str = elster::TESTMERKER_ERIC;

/// Woertliche Freigabe-Phrase fuer den Echtversand. Bewusst lang, bewusst keine Kurzform: wer sie
/// nicht ausdruecklich tippt, bekommt nie einen Echtversand.
pub const ECHTVERSAND_FREIGABE: &str = "JA ICH SENDE ECHT AN DAS FINANZAMT";

/// Warum [`sende`] abbricht (alle ausser [`VersandFehler::Eric`] VOR jedem Sendeaufruf).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersandFehler {
    /// Echtversand ohne (oder mit falscher) Freigabe.
    #[error(
        "Echtversand verlangt die Freigabe woertlich gleich {:?} — abgebrochen VOR jedem ERiC-Aufruf.",
        ECHTVERSAND_FREIGABE
    )]
    EchtversandOhneFreigabe,
    /// Das XML traegt nicht den Merker, den der Aufrufer behauptet.
    #[error("{0}")]
    XmlMerkerMismatch(String),
    /// Das XML ist kein lesbares UTF-8-XML; den Merker darin kann niemand pruefen.
    #[error("Das XML ist nicht lesbar (kein wohlgeformtes UTF-8-XML) — der Merker laesst sich nicht pruefen. Abgebrochen VOR jedem ERiC-Aufruf.")]
    XmlUnlesbar,
    /// Zertifikatspfad, -datei oder PIN fehlt, oder ERiC lehnt das Zertifikat ab.
    #[error("{0}")]
    ZertifikatFehlt(String),
    /// XML, Datenart, Pfad oder PIN enthaelt ein NUL-Byte (ERiC bekommt C-Strings).
    #[error("XML, Datenart, Zertifikatspfad oder PIN enthaelt ein NUL-Byte")]
    NulByte,
    /// ERiC liess sich nicht finden, laden oder initialisieren, oder ein Puffer fehlte.
    #[error(transparent)]
    Eric(#[from] EricFehler),
}

/// Test- oder Echtversand. Ein anderer Testmerker als [`TESTMERKER`] ist nicht darstellbar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modus {
    /// Testversand an die Clearingstelle (der Default des Programms).
    Testversand,
    /// ECHTVERSAND; `freigabe` muss [`ECHTVERSAND_FREIGABE`] woertlich sein.
    Echtversand { freigabe: String },
}

/// Zertifikat und PIN. `Debug` zeigt keins von beiden.
// ponytail: die PIN bleibt bis Prozessende im Speicher; `zeroize`, wenn das Programm je laenger lebt.
pub struct Zertifikat {
    pfad: PathBuf,
    pin: String,
}

impl Zertifikat {
    #[must_use]
    pub fn neu(pfad: impl Into<PathBuf>, pin: impl Into<String>) -> Self {
        Self {
            pfad: pfad.into(),
            pin: pin.into(),
        }
    }

    /// Gibt es einen Pfad und eine Datei dort? (Fuer die Zusammenfassung; nie der Pfad selbst.)
    #[must_use]
    pub fn vorhanden(&self) -> bool {
        !self.pfad.as_os_str().is_empty() && self.pfad.is_file()
    }

    /// Bricht mit `ZertifikatFehlt` ab, bevor irgendein ERiC-Aufruf stattfindet. Die Texte nennen,
    /// WAS fehlt, nie WO.
    fn pruefe(&self) -> Result<(), VersandFehler> {
        if self.pfad.as_os_str().is_empty() {
            return Err(VersandFehler::ZertifikatFehlt(
                "Kein Zertifikatspfad gesetzt. $ELSTER_ZERTIFIKAT_PFAD exportieren oder --zertifikat \
                 uebergeben — siehe elster/VERSAND.md."
                    .to_owned(),
            ));
        }
        if !self.pfad.is_file() {
            return Err(VersandFehler::ZertifikatFehlt(
                "Zertifikatsdatei nicht gefunden (Pfad gesetzt, Datei existiert nicht) — siehe \
                 elster/VERSAND.md."
                    .to_owned(),
            ));
        }
        if self.pin.is_empty() {
            return Err(VersandFehler::ZertifikatFehlt(
                "Keine PIN gesetzt. $ELSTER_ZERTIFIKAT_PIN exportieren — siehe elster/VERSAND.md."
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

impl fmt::Debug for Zertifikat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Zertifikat { pfad: <nicht gezeigt>, pin: <nicht gezeigt> }")
    }
}

/// Was ERiC zurueckgab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Antwort {
    pub rc: i32,
    pub rueckgabe_xml: String,
    pub serverantwort_xml: String,
    /// `eric.log` dieses Laufs (Besitzer-only-Verzeichnis); sein Inhalt nennt Auszuege der Erklaerung.
    pub log_pfad: PathBuf,
}

impl Antwort {
    /// Die Telenummer, der amtliche Nachweis der Uebermittlung.
    #[must_use]
    pub fn telenummer(&self) -> Option<String> {
        telenummer(&self.rueckgabe_xml)
    }

    /// Erfolg heisst `rc == 0` UND eine Telenummer; `rc == 0` allein reicht nicht.
    #[must_use]
    pub fn erfolg(&self) -> bool {
        self.rc == 0 && self.telenummer().is_some()
    }
}

/// Die Telenummer aus dem Rueckgabepuffer (`<Telenummer>…</Telenummer>`, mindestens ein Zeichen,
/// kein `<` darin) — der Erfolgsnachweis (`ericapi.h`: der Puffer „enthaelt beim Versand XML-Daten
/// mit generierter Telenummer").
///
/// ```
/// assert_eq!(versand::telenummer("<E><Telenummer>N55</Telenummer></E>").as_deref(), Some("N55"));
/// assert_eq!(versand::telenummer("<E><Telenummer></Telenummer></E>"), None);
/// ```
#[must_use]
pub fn telenummer(rueckgabe_xml: &str) -> Option<String> {
    const AUF: &str = "<Telenummer>";
    const ZU: &str = "</Telenummer>";
    let mut rest = rueckgabe_xml;
    while let Some(i) = rest.find(AUF) {
        let nach = rest.get(i + AUF.len()..)?;
        let (wert, danach) = nach.split_at(nach.find('<').unwrap_or(nach.len()));
        if !wert.is_empty() && danach.starts_with(ZU) {
            return Some(wert.to_owned());
        }
        rest = nach;
    }
    None
}

/// Nur unbedenkliche Metadaten fuers Logging und `--dry-run`: nie Zertifikatspfad, PIN,
/// Steuernummer oder IBAN (die stehen im XML-Inhalt, der hier nie ausgegeben wird).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zusammenfassung {
    pub datenart_version: String,
    pub modus: &'static str,
    pub merker_im_xml: Option<String>,
    pub merker_konsistent: bool,
    pub xml_bytes: usize,
    pub zertifikat_vorhanden: bool,
}

/// Die Zusammenfassung zu einem geplanten Versand; ruft ERiC nie.
#[must_use]
pub fn zusammenfassung(
    xml: &[u8],
    datenart_version: &str,
    modus: &Modus,
    zertifikat: &Zertifikat,
) -> Zusammenfassung {
    Zusammenfassung {
        datenart_version: datenart_version.to_owned(),
        modus: match modus {
            Modus::Testversand => "TESTVERSAND (Clearingstelle, wird verworfen)",
            Modus::Echtversand { .. } => "ECHTVERSAND",
        },
        merker_im_xml: merker_im_xml(xml),
        merker_konsistent: pruefe_merker_konsistenz(xml, modus).is_ok(),
        xml_bytes: xml.len(),
        zertifikat_vorhanden: zertifikat.vorhanden(),
    }
}

fn pruefe_freigabe(modus: &Modus) -> Result<(), VersandFehler> {
    match modus {
        Modus::Echtversand { freigabe } if freigabe != ECHTVERSAND_FREIGABE => {
            Err(VersandFehler::EchtversandOhneFreigabe)
        }
        Modus::Echtversand { .. } | Modus::Testversand => Ok(()),
    }
}

/// `EricBearbeiteVorgang(ERIC_VALIDIERE | ERIC_SENDE)` — der eigentliche Versand. `lib` ist die
/// `libericapi.so` (`elster::find_eric_lib()`); `None` ist ein Fehler, aber erst nach allen Sperren.
///
/// Reihenfolge, jede Stufe bricht fail-closed ab: Freigabe, XML-Merker, Zertifikat (Pfad, Datei,
/// PIN), dann erst ERiC laden, das Zertifikat oeffnen und senden. Der Aufruf kommt aus dem
/// aufrufenden Thread (ERiC ist Singlethread; ein Prozess macht einen Versand).
///
/// # Errors
/// [`VersandFehler`]; jeder ausser [`VersandFehler::Eric`] heisst: ERiC wurde nicht aufgerufen.
/// `rc != 0` von ERiC ist KEIN Fehler dieser Funktion, sondern steht in der [`Antwort`].
pub fn sende(
    lib: Option<&Path>,
    xml: &[u8],
    datenart_version: &str,
    zertifikat: &Zertifikat,
    modus: &Modus,
) -> Result<Antwort, VersandFehler> {
    pruefe_freigabe(modus)?;
    pruefe_merker_konsistenz(xml, modus)?;
    zertifikat.pruefe()?;
    let xml_c = CString::new(xml).map_err(|_| VersandFehler::NulByte)?;
    let datenart_c = CString::new(datenart_version).map_err(|_| VersandFehler::NulByte)?;
    let pfad_c = CString::new(zertifikat.pfad.as_os_str().as_bytes())
        .map_err(|_| VersandFehler::NulByte)?;
    let pin_c = CString::new(zertifikat.pin.as_bytes()).map_err(|_| VersandFehler::NulByte)?;

    let lib = lib.ok_or(EricFehler::NichtGefunden)?;
    let log_dir = ffi::neues_log_dir()?;
    let eric = ffi::Eric::laden(lib, &log_dir)?;
    let handle = eric.zertifikat_oeffnen(&pfad_c).map_err(|rc| {
        VersandFehler::ZertifikatFehlt(format!(
            "ERiC lehnt das Zertifikat ab (EricGetHandleToCertificate rc={rc}) — Pfad/Format/PIN \
             pruefen, siehe elster/VERSAND.md. Wert nicht geloggt."
        ))
    })?;
    let (rc, rueckgabe_xml, serverantwort_xml) =
        eric.bearbeite(&xml_c, &datenart_c, ERIC_VALIDIERE | ERIC_SENDE, &handle, &pin_c)?;
    Ok(Antwort {
        rc,
        rueckgabe_xml,
        serverantwort_xml,
        log_pfad: log_dir.join("eric.log"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANTWORT_ERFOLG: &str =
        "<Elster><Erfolg><Telenummer>N552026081012345</Telenummer></Erfolg></Elster>";

    #[test]
    fn telenummer_extrahiert_den_wert() {
        assert_eq!(telenummer(ANTWORT_ERFOLG).as_deref(), Some("N552026081012345"));
    }

    #[test]
    fn telenummer_fehlt_liefert_none() {
        for t in [
            "<Elster><Fehler/></Elster>",
            "",
            "<Telenummer></Telenummer>",
            "<Telenummer>offen",
            "<Telenummer>a<b></Telenummer>",
        ] {
            assert_eq!(telenummer(t), None, "{t}");
        }
    }

    #[test]
    fn telenummer_ueberspringt_einen_leeren_treffer_vor_dem_echten() {
        let t = "<Telenummer></Telenummer><Telenummer>N1</Telenummer>";
        assert_eq!(telenummer(t).as_deref(), Some("N1"));
    }

    fn antwort(rc: i32, text: &str) -> Antwort {
        Antwort {
            rc,
            rueckgabe_xml: text.to_owned(),
            serverantwort_xml: String::new(),
            log_pfad: PathBuf::new(),
        }
    }

    #[test]
    fn erfolg_braucht_rc_null_und_eine_telenummer() {
        assert!(antwort(0, ANTWORT_ERFOLG).erfolg());
        assert!(!antwort(0, "<Elster/>").erfolg(), "rc 0 allein ist kein Erfolg");
        assert!(!antwort(610_101_271, ANTWORT_ERFOLG).erfolg(), "Telenummer allein ist kein Erfolg");
    }

    #[test]
    fn die_zusammenfassung_zeigt_weder_pfad_noch_pin() {
        let z = Zertifikat::neu("/geheim/mein_zertifikat.pfx", "PIN-SENTINEL-4711");
        let info = zusammenfassung(b"<E/>", "ESt_2025", &Modus::Testversand, &z);
        let dump = format!("{info:?} {z:?}");
        assert!(!dump.contains("geheim") && !dump.contains("mein_zertifikat"), "{dump}");
        assert!(!dump.contains("PIN-SENTINEL-4711"), "{dump}");
        assert!(!info.zertifikat_vorhanden);
    }

    #[test]
    fn die_zusammenfassung_nennt_modus_und_merker() {
        let z = Zertifikat::neu("", "");
        let xml = b"<E><TransferHeader><Testmerker>700000004</Testmerker></TransferHeader></E>";
        let info = zusammenfassung(xml, "ESt_2025", &Modus::Testversand, &z);
        assert_eq!(info.merker_im_xml.as_deref(), Some("700000004"));
        assert!(info.merker_konsistent);
        assert!(info.modus.starts_with("TESTVERSAND"));
        let echt = Modus::Echtversand {
            freigabe: ECHTVERSAND_FREIGABE.to_owned(),
        };
        assert!(!zusammenfassung(xml, "ESt_2025", &echt, &z).merker_konsistent);
    }
}
