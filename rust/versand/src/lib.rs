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

mod abruf;
mod cli;
mod ffi;
mod merker;

use std::ffi::CString;
use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub use abruf::{dekodiere_datenpaket, AbrufFehler};
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

/// Der Text, wenn ERiC das Zertifikat ablehnt (`EricGetHandleToCertificate`): nennt den Code, nie
/// Pfad oder PIN.
fn zertifikat_abgelehnt(rc: i32) -> String {
    format!(
        "ERiC lehnt das Zertifikat ab (EricGetHandleToCertificate rc={rc}) — Pfad/Format/PIN \
         pruefen, siehe elster/VERSAND.md. Wert nicht geloggt."
    )
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

/// `ERIC_TRANSFER_ERR_XML_THEADER` (`eric_fehlercodes.h`): der Annahmeserver meldet einen Fehler im
/// Transferheader; die Fehlermeldungen stehen im Serverantwortpuffer.
pub const ERIC_TRANSFER_ERR_XML_THEADER: i32 = 610_101_210;

/// `ERIC_TRANSFER_ERR_XML_NHEADER` (`eric_fehlercodes.h`): dasselbe fuer den Nutzdatenheader.
pub const ERIC_TRANSFER_ERR_XML_NHEADER: i32 = 610_101_292;

/// Laenge, auf die Code und Text einer Servermeldung gekuerzt werden.
const MELDUNG_CODE_MAX: usize = 20;
const MELDUNG_TEXT_MAX: usize = 300;

/// Eine Fehlermeldung des ELSTER-Servers: Code und Text aus einem `RC`-Block der Serverantwort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Servermeldung {
    pub code: String,
    pub text: String,
}

/// Steuerzeichen werden zu Leerzeichen, Leerraum zusammengezogen, auf `max` Zeichen gekuerzt: die
/// Serverantwort ist fremder Text, der auf ein Terminal geht.
fn bereinigt(text: &str, max: usize) -> String {
    let ohne_steuerzeichen: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let eng = ohne_steuerzeichen.split_whitespace().collect::<Vec<_>>().join(" ");
    eng.chars().take(max).collect()
}

/// Die Fehlermeldungen des Servers aus der Serverantwort (`EricBearbeiteVorgang`, bei
/// [`ERIC_TRANSFER_ERR_XML_THEADER`] und [`ERIC_TRANSFER_ERR_XML_NHEADER`]): Code und Text der
/// `Rueckgabe`- und `Stack`-Eintraege im `RC`-Block von `TransferHeader` und `NutzdatenHeader`
/// (Aufbau: ERiC-Dokumentation, `Datenarten/.../Beispiele/*_Response.xml`). Code 0 heisst „OK" und
/// zaehlt nicht; gleiche Eintraege stehen einmal. Aus dem Datenteil kommt nichts. Eine leere,
/// unlesbare oder fremd aufgebaute Antwort liefert eine leere Liste.
///
/// ```
/// let xml = "<Elster><TransferHeader><RC><Rueckgabe><Code>42</Code><Text>Nein</Text></Rueckgabe>\
///            </RC></TransferHeader></Elster>";
/// let m = versand::servermeldungen(xml);
/// assert_eq!(m.len(), 1);
/// assert_eq!(m.first().map(|x| (x.code.as_str(), x.text.as_str())), Some(("42", "Nein")));
/// assert!(versand::servermeldungen("kein xml").is_empty());
/// ```
#[must_use]
// ponytail: Aufbau nur aus der ERiC-Dokumentation, an einer echten Ablehnung ungemessen; bei einer
// Abweichung hier anpassen oder `EricGetErrormessagesFromXMLAnswer` ueber FFI rufen (ericapi.h).
pub fn servermeldungen(serverantwort_xml: &str) -> Vec<Servermeldung> {
    let Ok(dok) = roxmltree::Document::parse(serverantwort_xml) else {
        return Vec::new();
    };
    let heisst = |n: &roxmltree::Node<'_, '_>, namen: &[&str]| {
        n.is_element() && namen.contains(&n.tag_name().name())
    };
    let mut aus: Vec<Servermeldung> = Vec::new();
    for rc in dok.descendants().filter(|n| {
        heisst(n, &["RC"])
            && n.parent_element()
                .is_some_and(|p| heisst(&p, &["TransferHeader", "NutzdatenHeader"]))
    }) {
        for eintrag in rc.children().filter(|e| heisst(e, &["Rueckgabe", "Stack"])) {
            let kind = |name: &str| {
                eintrag
                    .children()
                    .find(|k| heisst(k, &[name]))
                    .and_then(|k| k.text())
                    .unwrap_or_default()
            };
            let code = bereinigt(kind("Code"), MELDUNG_CODE_MAX);
            if code.chars().all(|c| c == '0') {
                continue;
            }
            let meldung = Servermeldung {
                code,
                text: bereinigt(kind("Text"), MELDUNG_TEXT_MAX),
            };
            if !aus.contains(&meldung) {
                aus.push(meldung);
            }
        }
    }
    aus
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
    let handle = eric
        .zertifikat_oeffnen(&pfad_c)
        .map_err(|rc| VersandFehler::ZertifikatFehlt(zertifikat_abgelehnt(rc)))?;
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

    // Aufbau der Serverantwort: ERiC-Dokumentation (`ericapi.h`, „Fehler in der Serverantwort") und
    // ihre Beispiele unter `Dokumentation/Datenarten/`: `ElsterSignatur/Kontoinformation/Beispiele/
    // Kontoinformation_Response.xml` (`TransferHeader/RC`, Code 0) und `ElsterDatenabholung/
    // PostfachBestaetigung/Beispiele/2_b1_PostfachBestaetigung_32_1NDB_Response.xml`
    // (`NutzdatenHeader/RC`, Code 67890; im Datenteil ein weiteres `Rueckgabe` mit Code 194346573).
    const DOC_TRANSFERHEADER_OK: &str = concat!(
        r#"<Elster xmlns="http://www.elster.de/elsterxml/schema/v11"><TransferHeader version="11">"#,
        r"<RC><Rueckgabe><Code>0</Code><Text>OK</Text></Rueckgabe>",
        r"<Stack><Code>0</Code><Text /></Stack></RC></TransferHeader></Elster>"
    );
    const DOC_NUTZDATENHEADER_FEHLER: &str = concat!(
        r#"<Elster xmlns="http://www.elster.de/elsterxml/schema/v11">"#,
        r#"<TransferHeader version="11"><RC><Rueckgabe><Code>0</Code><Text>OK</Text></Rueckgabe>"#,
        r"</RC></TransferHeader><DatenTeil><Nutzdatenblock>",
        r#"<NutzdatenHeader version="11"><RC>"#,
        r"<Rueckgabe><Code>67890</Code><Text>Der Empfang von mindestens einer der Daten wurde nicht ",
        r"erfolgreich bestaetigt.</Text></Rueckgabe>",
        r"<Stack><Code>67890</Code><Text>Der Empfang von mindestens einer der Daten wurde nicht ",
        r"erfolgreich bestaetigt.</Text></Stack></RC></NutzdatenHeader>",
        r"<Nutzdaten><Bereitstellung><Rueckgabe><Code>194346573</Code>",
        r"<Text>Datensatz nicht gefunden.</Text></Rueckgabe></Bereitstellung></Nutzdaten>",
        r"</Nutzdatenblock></DatenTeil></Elster>"
    );

    fn meldung(code: &str, text: &str) -> Servermeldung {
        Servermeldung {
            code: code.to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn servermeldungen_liest_den_rc_block_des_nutzdatenheaders_und_nur_ihn() {
        assert_eq!(
            servermeldungen(DOC_NUTZDATENHEADER_FEHLER),
            vec![meldung(
                "67890",
                "Der Empfang von mindestens einer der Daten wurde nicht erfolgreich bestaetigt."
            )],
            "Rueckgabe und Stack gleichen Inhalts stehen einmal; der Datenteil zaehlt nicht"
        );
    }

    #[test]
    fn servermeldungen_kennt_code_null_als_ok() {
        assert_eq!(servermeldungen(DOC_TRANSFERHEADER_OK), Vec::new());
    }

    #[test]
    fn servermeldungen_nennt_verschiedene_eintraege_in_dokumentreihenfolge() {
        let xml = "<Elster><TransferHeader><RC>\
                   <Rueckgabe><Code>080015009</Code><Text>Zertifikat gesperrt</Text></Rueckgabe>\
                   <Stack><Code>000000042</Code><Text>Weiter unten</Text></Stack>\
                   <Stack><Code>080015009</Code><Text>Zertifikat gesperrt</Text></Stack>\
                   </RC></TransferHeader></Elster>";
        assert_eq!(
            servermeldungen(xml),
            vec![
                meldung("080015009", "Zertifikat gesperrt"),
                meldung("000000042", "Weiter unten")
            ]
        );
    }

    #[test]
    fn servermeldungen_bereinigt_steuerzeichen_und_kuerzt() {
        // ESC ist in XML 1.0 verboten (die Antwort wuerde nicht gelesen); U+009B (CSI, ein
        // Steuerzeichen der C1-Gruppe) ist erlaubt und wirkt auf manchen Terminals wie ESC `[`.
        let lang = "a".repeat(400);
        let xml = format!(
            "<Elster><TransferHeader><RC><Rueckgabe><Code>7</Code>\
             <Text>\u{9b}31mrot\n\tZeile &amp; mehr</Text></Rueckgabe>\
             <Stack><Code>8</Code><Text>{lang}</Text></Stack></RC></TransferHeader></Elster>"
        );
        let m = servermeldungen(&xml);
        assert_eq!(m.first(), Some(&meldung("7", "31mrot Zeile & mehr")));
        assert_eq!(m.get(1).map(|x| x.text.chars().count()), Some(MELDUNG_TEXT_MAX));
        assert!(!m.iter().any(|x| x.text.chars().any(char::is_control)));
    }

    #[test]
    fn servermeldungen_liefert_bei_unbrauchbarer_antwort_nichts() {
        for t in [
            "",
            "kein xml",
            "<Elster/>",
            "<Elster><TransferHeader/></Elster>",
            "<Elster><RC><Rueckgabe><Code>5</Code><Text>nicht im Header</Text></Rueckgabe></RC></Elster>",
            "<Elster><TransferHeader><RC><Rueckgabe><Text>ohne Code</Text></Rueckgabe></RC></TransferHeader></Elster>",
            "<!DOCTYPE Elster [<!ENTITY a \"b\">]><Elster/>",
        ] {
            assert_eq!(servermeldungen(t), Vec::new(), "{t}");
        }
    }
}
