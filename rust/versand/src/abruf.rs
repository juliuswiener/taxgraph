//! Der Abruf der Lohnsteuerbescheinigung bei ELSTER (`ElsterVaStDaten`), Abweichung Nr. 91.
//!
//! Ablauf nach dem ERiC-Handbuch 44.2.4.0, Kap. 9.2.4.2 und 9.9.6: Zertifikat oeffnen, Anfrage,
//! Wahl der `<Id>`, Abholung, `EricDekodiereDaten`. Jeder Schritt vor dem ersten ERiC-Aufruf bricht
//! ab, ohne ERiC zu laden. Steuer-ID, Datenpaket und PIN stehen nie in einem Fehlertext.

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use elster::EricFehler;

use crate::{ffi, zertifikat_abgelehnt, Zertifikat};

/// Warum der Abruf abbricht.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AbrufFehler {
    /// Das Datenpaket ist leer.
    #[error("Das Datenpaket ist leer.")]
    DatenpaketLeer,
    /// Das Datenpaket enthaelt ein NUL-Byte.
    #[error("Das Datenpaket enthaelt ein NUL-Byte.")]
    DatenpaketNulByte,
    /// Zertifikatspfad, -datei oder PIN fehlt, oder ERiC lehnt das Zertifikat ab.
    #[error("{0}")]
    Zertifikat(String),
    /// ERiC kann das Datenpaket nicht entschluesseln.
    #[error("ERiC konnte das Datenpaket nicht entschluesseln (EricDekodiereDaten rc={rc}).")]
    Dekodieren { rc: i32 },
    /// ERiC liess sich nicht finden, laden oder initialisieren, oder ein Puffer fehlte.
    #[error(transparent)]
    Eric(#[from] EricFehler),
    /// Eine Eingabe ist ungueltig; der Text nennt das Feld, nie den Wert.
    #[error("Eingabe {feld}: {grund}.")]
    Eingabe {
        feld: &'static str,
        grund: &'static str,
    },
    /// Die Antwort auf die Anfrage nennt keinen Beleg `VaSt_LStB` in der Schemaversion 202501.
    #[error(
        "Die Antwort nennt keinen Beleg VaSt_LStB der Schemaversion 202501 (VZ 2025); gefunden: {gefunden}."
    )]
    KeinBeleg { gefunden: String },
    /// Mehrere Belege `VaSt_LStB` (mehrere Arbeitgeber): die Summenregel ist unbekannt.
    #[error("Die Antwort nennt {0} Belege VaSt_LStB; mehrere Arbeitgeber werden nicht summiert.")]
    MehrereBelege(usize),
    /// Ein Teil der Antwort von ELSTER fehlt oder ist unlesbar.
    #[error("Die Antwort von ELSTER ist nicht lesbar ({0}).")]
    AntwortUnlesbar(&'static str),
}

/// Die Eingaben eines Abrufs, einmal geprueft. STUMPF.
pub struct AbrufAuftrag {
    jahr: i32,
}

impl std::fmt::Debug for AbrufAuftrag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AbrufAuftrag {{ jahr: {} }}", self.jahr)
    }
}

impl AbrufAuftrag {
    /// STUMPF.
    ///
    /// # Errors
    /// [`AbrufFehler::Eingabe`].
    pub fn neu(
        _idnr: &str,
        _jahr: &str,
        _hersteller_id: &str,
        _heute: i32,
    ) -> Result<Self, AbrufFehler> {
        Ok(Self { jahr: 0 })
    }

    /// Das Veranlagungsjahr.
    #[must_use]
    pub fn veranlagungsjahr(&self) -> i32 {
        self.jahr
    }

    /// STUMPF.
    #[must_use]
    pub fn anfrage_xml(&self) -> String {
        String::new()
    }

    /// STUMPF.
    ///
    /// # Errors
    /// [`AbrufFehler::AntwortUnlesbar`].
    pub fn abholung_xml(&self, _id: &str) -> Result<String, AbrufFehler> {
        Ok(String::new())
    }
}

/// STUMPF.
///
/// # Errors
/// [`AbrufFehler`].
pub fn waehle_id(_serverantwort_xml: &str) -> Result<String, AbrufFehler> {
    Ok(String::new())
}

/// STUMPF.
///
/// # Errors
/// [`AbrufFehler`].
pub fn datenpaket(_serverantwort_xml: &str) -> Result<String, AbrufFehler> {
    Ok(String::new())
}

/// Zertifikatspfad und PIN als C-Strings; bricht VOR jedem ERiC-Aufruf ab, wenn eins fehlt.
fn zertifikat_als_c(zertifikat: &Zertifikat) -> Result<(CString, CString), AbrufFehler> {
    zertifikat
        .pruefe()
        .map_err(|e| AbrufFehler::Zertifikat(e.to_string()))?;
    let nul = || AbrufFehler::Zertifikat("Zertifikatspfad oder PIN enthaelt ein NUL-Byte".to_owned());
    let pfad = CString::new(zertifikat.pfad.as_os_str().as_bytes()).map_err(|_| nul())?;
    let pin = CString::new(zertifikat.pin.as_bytes()).map_err(|_| nul())?;
    Ok((pfad, pin))
}

/// `EricDekodiereDaten` auf einer geladenen Instanz mit offenem Zertifikat.
fn dekodiere_paket(
    eric: &ffi::Eric,
    handle: &ffi::ZertifikatWaechter<'_>,
    pin: &CStr,
    base64: &CStr,
) -> Result<Vec<u8>, AbrufFehler> {
    let (rc, klartext) = eric.dekodiere(handle, pin, base64)?;
    if rc != 0 {
        return Err(AbrufFehler::Dekodieren { rc });
    }
    Ok(klartext)
}

/// Entschluesselt ein abgeholtes Datenpaket (Base64, wie es in `<Datenpaket>` steht) mit dem
/// Zertifikat: `EricDekodiereDaten`. Ruft das Netz nie. Ein Fehler von ERiC ist ein benannter
/// Fehler, nie ein Panic; der Puffer von ERiC wird dann nicht gelesen.
///
/// # Errors
/// [`AbrufFehler`]; jeder ausser [`AbrufFehler::Eric`] und [`AbrufFehler::Dekodieren`] heisst: ERiC
/// wurde nicht geladen.
pub fn dekodiere_datenpaket(
    lib: Option<&Path>,
    zertifikat: &Zertifikat,
    base64: &str,
) -> Result<Vec<u8>, AbrufFehler> {
    if base64.is_empty() {
        return Err(AbrufFehler::DatenpaketLeer);
    }
    let base64_c = CString::new(base64).map_err(|_| AbrufFehler::DatenpaketNulByte)?;
    let (pfad_c, pin_c) = zertifikat_als_c(zertifikat)?;
    let lib = lib.ok_or(EricFehler::NichtGefunden)?;
    let log_dir = ffi::neues_log_dir()?;
    let eric = ffi::Eric::laden(lib, &log_dir)?;
    let handle = eric
        .zertifikat_oeffnen(&pfad_c)
        .map_err(|rc| AbrufFehler::Zertifikat(zertifikat_abgelehnt(rc)))?;
    dekodiere_paket(&eric, &handle, &pin_c, &base64_c)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Erfundene Werte: keine echte Steuer-ID, keine echte Hersteller-ID (74931 ist der Platzhalter der
    // ERiC-Beispiele).
    const IDNR: &str = "12345678903";
    const HID: &str = "74931";
    const ID_A: &str = "xy123abcdefghijkmnopqrstuvwxyz12";
    const ID_B: &str = "zz999abcdefghijkmnopqrstuvwxyz12";
    const ID_C: &str = "ab001abcdefghijkmnopqrstuvwxyz12";
    const LSTB: &str = "VaSt_LStB";

    fn auftrag() -> AbrufAuftrag {
        AbrufAuftrag::neu(IDNR, "2025", HID, 2026).unwrap()
    }

    /// Die Antwort auf eine Anfrage nach dem Aufbau der amtlichen Beispiele (`4_b_...Response.xml`).
    fn antwort(ids: &[(&str, &str, &str)]) -> String {
        let eintraege: String = ids
            .iter()
            .map(|(id, art, version)| {
                format!(
                    r#"<Id id="{id}" groesse="1500" belegart="{art}" hashwert="680f" schemaversion="{version}"/>"#
                )
            })
            .collect();
        format!(
            r#"<Elster xmlns="http://www.elster.de/elsterxml/schema/v11"><DatenTeil><Nutzdatenblock><Nutzdaten><Datenabholung xmlns="http://finkonsens.de/elster/elsterdatenabholung/v3" version="32"><Anfrage idnr="{IDNR}" veranlagungsjahr="2025">{eintraege}</Anfrage></Datenabholung></Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"#
        )
    }

    fn feld_und_wert(f: &AbrufFehler) -> (&'static str, String) {
        match f {
            AbrufFehler::Eingabe { feld, .. } => (*feld, f.to_string()),
            andere => panic!("erwartet Eingabe, bekam {andere:?}"),
        }
    }

    #[test]
    fn abruf_pruefung_steuer_id_verlangt_genau_elf_ziffern_und_nennt_das_feld_nie_den_wert() {
        assert!(AbrufAuftrag::neu(IDNR, "2025", HID, 2026).is_ok());
        let faelle = [
            "1234567890",   // zehn
            "123456789012", // zwoelf
            "1234567890a",  // Buchstabe
            "12345 678903", // Leerzeichen
            " 12345678903", // Leerzeichen vorn
            "١٢٣٤٥٦٧٨٩٠١",  // elf Ziffern einer anderen Schrift
            "",
        ];
        for idnr in faelle {
            let f = AbrufAuftrag::neu(idnr, "2025", HID, 2026).unwrap_err();
            let (feld, text) = feld_und_wert(&f);
            assert_eq!(feld, "idnr", "{idnr:?}");
            assert!(idnr.is_empty() || !text.contains(idnr.trim()), "der Wert steht im Text: {text}");
        }
    }

    #[test]
    fn abruf_pruefung_jahr_laesst_nur_die_letzten_vier_veranlagungsjahre_zu() {
        for ok in ["2023", "2024", "2025", "2026"] {
            let a = AbrufAuftrag::neu(IDNR, ok, HID, 2026).unwrap();
            assert_eq!(a.veranlagungsjahr().to_string(), ok);
        }
        for schlecht in ["2022", "2027", "1999", "25", "20250", "abcd", "2O25", "+2025", " 2025", ""] {
            let f = AbrufAuftrag::neu(IDNR, schlecht, HID, 2026).unwrap_err();
            assert_eq!(feld_und_wert(&f).0, "jahr", "{schlecht:?}");
        }
        // Die Grenze wandert mit dem laufenden Jahr.
        assert!(AbrufAuftrag::neu(IDNR, "2022", HID, 2025).is_ok());
        assert!(AbrufAuftrag::neu(IDNR, "2021", HID, 2025).is_err());
    }

    #[test]
    fn abruf_pruefung_hersteller_id_verlangt_genau_fuenf_ziffern() {
        for schlecht in ["7493", "749311", "7493a", "", " 74931"] {
            let f = AbrufAuftrag::neu(IDNR, "2025", schlecht, 2026).unwrap_err();
            let (feld, text) = feld_und_wert(&f);
            assert_eq!(feld, "hersteller_id", "{schlecht:?}");
            assert!(schlecht.is_empty() || !text.contains(schlecht.trim()), "{text}");
        }
    }

    #[test]
    fn abruf_debug_zeigt_weder_steuer_id_noch_hersteller_id() {
        let text = format!("{:?}", auftrag());
        assert!(!text.contains(IDNR) && !text.contains(HID), "{text}");
        assert!(text.contains("2025"), "das Jahr ist unbedenklich: {text}");
    }

    #[test]
    fn abruf_anfrage_xml_traegt_datenart_vorgang_idnr_und_jahr() {
        let xml = auftrag().anfrage_xml();
        let dok = roxmltree::Document::parse(&xml).unwrap();
        let text = |name: &str| {
            dok.descendants()
                .find(|n| n.has_tag_name(name))
                .and_then(|n| n.text())
                .unwrap_or_default()
                .to_owned()
        };
        assert_eq!(text("Verfahren"), "ElsterDatenabholung");
        assert_eq!(text("DatenArt"), "ElsterVaStDaten");
        assert_eq!(text("Vorgang"), "send-Auth");
        assert_eq!(text("HerstellerID"), HID);
        let anfrage = dok.descendants().find(|n| n.has_tag_name("Anfrage")).unwrap();
        assert_eq!(anfrage.attribute("idnr"), Some(IDNR));
        assert_eq!(anfrage.attribute("veranlagungsjahr"), Some("2025"));
        assert_eq!(
            anfrage.tag_name().namespace(),
            Some("http://finkonsens.de/elster/elsterdatenabholung/v3")
        );
        assert_eq!(anfrage.parent_element().and_then(|p| p.attribute("version")), Some("32"));
        assert!(dok.descendants().all(|n| !n.has_tag_name("Abholung")), "eine Anfrage holt nichts ab");
    }

    #[test]
    fn abruf_abholung_xml_traegt_id_idnr_und_jahr() {
        let xml = auftrag().abholung_xml(ID_A).unwrap();
        let dok = roxmltree::Document::parse(&xml).unwrap();
        let abholung = dok.descendants().find(|n| n.has_tag_name("Abholung")).unwrap();
        assert_eq!(abholung.attribute("id"), Some(ID_A));
        assert_eq!(abholung.attribute("idnr"), Some(IDNR));
        assert_eq!(abholung.attribute("veranlagungsjahr"), Some("2025"));
        assert!(dok.descendants().all(|n| !n.has_tag_name("Anfrage")), "eine Abholung fragt nichts an");
        assert_ne!(xml, auftrag().anfrage_xml());
    }

    #[test]
    fn abruf_abholung_weist_eine_id_ab_die_nicht_dem_muster_der_amtlichen_id_entspricht() {
        let ids = [
            String::new(),
            "kurz".to_owned(),
            "x\"/><Evil/><y a=\"".to_owned(),
            ID_A.to_uppercase(),
            format!("{ID_A}x"),
            "xy123abcdefghijklmnopqrstuvwxyz1".to_owned(), // 32 Zeichen, aber mit `l`
        ];
        for id in &ids {
            assert_eq!(
                auftrag().abholung_xml(id),
                Err(AbrufFehler::AntwortUnlesbar("Id")),
                "{id:?}"
            );
        }
    }

    #[test]
    fn abruf_waehle_id_nimmt_den_lstb_beleg_der_schemaversion_202501_und_keinen_anderen() {
        let xml = antwort(&[
            (ID_A, "VaSt_RBM", "202501"),
            (ID_B, LSTB, "202401"),
            (ID_C, LSTB, "202501"),
        ]);
        assert_eq!(waehle_id(&xml).as_deref(), Ok(ID_C));
        // Die Reihenfolge entscheidet nicht.
        let xml = antwort(&[(ID_C, LSTB, "202501"), (ID_A, "VaSt_RBM", "202501")]);
        assert_eq!(waehle_id(&xml).as_deref(), Ok(ID_C));
    }

    #[test]
    fn abruf_waehle_id_lehnt_mehrere_lstb_belege_ab_statt_einen_zu_nehmen() {
        let xml = antwort(&[(ID_A, LSTB, "202501"), (ID_C, LSTB, "202501")]);
        assert_eq!(waehle_id(&xml), Err(AbrufFehler::MehrereBelege(2)));
    }

    #[test]
    fn abruf_waehle_id_ohne_passenden_beleg_nennt_belegart_und_version_nie_die_id() {
        let xml = antwort(&[(ID_A, "VaSt_RBM", "202501"), (ID_B, LSTB, "202401")]);
        let Err(AbrufFehler::KeinBeleg { gefunden }) = waehle_id(&xml) else {
            panic!("erwartet KeinBeleg");
        };
        assert!(gefunden.contains("VaSt_RBM 202501") && gefunden.contains("VaSt_LStB 202401"), "{gefunden}");
        assert!(!gefunden.contains(ID_A) && !gefunden.contains(ID_B), "{gefunden}");
        let leer = antwort(&[]);
        assert!(matches!(waehle_id(&leer), Err(AbrufFehler::KeinBeleg { .. })));
    }

    #[test]
    fn abruf_waehle_id_weist_eine_unlesbare_antwort_und_eine_ungueltige_id_ab() {
        for kaputt in ["", "kein xml", "<Elster><Anfrage>", "<!DOCTYPE a [<!ENTITY b \"c\">]><a/>"] {
            assert_eq!(waehle_id(kaputt), Err(AbrufFehler::AntwortUnlesbar("Serverantwort")), "{kaputt:?}");
        }
        let xml = antwort(&[("keine-id", LSTB, "202501")]);
        assert_eq!(waehle_id(&xml), Err(AbrufFehler::AntwortUnlesbar("Id")));
    }

    #[test]
    fn abruf_datenpaket_zieht_den_base64_text_ohne_zeilenumbrueche() {
        let xml = "<Elster><Nutzdaten><Datenabholung><Abholung id=\"x\"><Datenpaket>MIAGCSqG\n   SIb3DQEH\n\t A6CAMIAC\n  </Datenpaket></Abholung></Datenabholung></Nutzdaten></Elster>";
        assert_eq!(datenpaket(xml).as_deref(), Ok("MIAGCSqGSIb3DQEHA6CAMIAC"));
    }

    #[test]
    fn abruf_datenpaket_fehlt_oder_ist_leer_ist_eine_unlesbare_antwort() {
        for xml in [
            "<Elster><Abholung id=\"x\"/></Elster>",
            "<Elster><Abholung id=\"x\"><Datenpaket/></Abholung></Elster>",
            "<Elster><Abholung id=\"x\"><Datenpaket>  \n </Datenpaket></Abholung></Elster>",
        ] {
            assert_eq!(datenpaket(xml), Err(AbrufFehler::AntwortUnlesbar("Datenpaket")), "{xml}");
        }
        assert_eq!(datenpaket("kein xml"), Err(AbrufFehler::AntwortUnlesbar("Serverantwort")));
    }
}
