//! Der Versandpfad gegen die ATTRAPPE von `libericapi.so` — nie die echte Bibliothek, nie ein
//! Zertifikat, nie eine echte PIN, nie das Netz.
//!
//! Die Attrappe (`eric_attrappe.c`, dieselbe wie im Vergleichslauf; Ort siehe `ATTRAPPE_QUELLE`) wird mit
//! `cc -shared -fPIC` gebaut und antwortet nach Steuerdateien in `$ERIC_ATTRAPPE_DIR`; sie schreibt
//! auf, was sie bekam: Flags, Crypto-Parameter (die PIN nur als Laenge), den Pfad des Zertifikats, die
//! Zahl der Aufrufe. Jeder Test liest daraus, WAS bei ERiC ankam — und bei jeder Sperre, dass NICHTS
//! ankam (`init_zaehler` fehlt: ERiC wurde nicht einmal geladen).
//!
//! `ERIC_ATTRAPPE_DIR` ist Prozess-Zustand; darum laufen alle Tests, die ERiC rufen, hintereinander
//! (`SPERRE`; unter `cargo nextest` hat jeder Test seinen eigenen Prozess, siehe `bibliothek`).
//! Braucht `cc`. Fehlt es, scheitert der Test — ausser mit `TAXGRAPH_OHNE_CC=1`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

use elster::EricFehler;
use versand::{
    dekodiere_datenpaket, hole_beleg, lauf, lauf_abruf, lege_beleg_ab, sende, AbrufAuftrag,
    AbrufFehler, AbrufUmgebung, Antwort, Modus, Servermeldung, Umgebung, VersandFehler, Zertifikat,
    ABRUF_FREIGABE, ECHTVERSAND_FREIGABE,
};

const XML_TEST: &[u8] = b"<?xml version=\"1.0\"?><Elster><TransferHeader>\
<Testmerker>700000004</Testmerker></TransferHeader></Elster>";
const XML_ECHT: &[u8] = b"<?xml version=\"1.0\"?><Elster><TransferHeader></TransferHeader></Elster>";
const ANTWORT_ERFOLG: &str =
    "<Elster><Erfolg><Telenummer>N552026081012345</Telenummer></Erfolg></Elster>";
const PIN: &str = "PIN-SENTINEL-4711";
const ZERT_NAME: &str = "zertifikat-SENTINEL-9f3a.pfx";

/// Serverantworten fuer die Ablehnung durch den ELSTER-Server. Aufbau nach der ERiC-Dokumentation
/// (`ericapi.h`, „Fehler in der Serverantwort"; Beispiele `Dokumentation/Datenarten/ElsterSignatur/
/// Kontoinformation/Beispiele/Kontoinformation_Response.xml` und `ElsterDatenabholung/
/// PostfachBestaetigung/Beispiele/2_b1_PostfachBestaetigung_32_1NDB_Response.xml`): Meldungen stehen
/// in `TransferHeader/RC` und `NutzdatenHeader/RC`. Wortlaut und Code 080015009 der ersten nach dem
/// Bericht vom 11.10.2026 (gesperrtes Zertifikat), NICHT aus einem Puffer gemessen. Die `*-MARKER`
/// stehen dort, wo ERiC Inhalt der Erklaerung oder Kopfdaten fuehrt; sie duerfen nie in der Ausgabe
/// stehen.
const SERVER_TRANSFERHEADER: &str = concat!(
    r#"<Elster xmlns="http://www.elster.de/elsterxml/schema/v11"><TransferHeader version="11">"#,
    r"<DatenLieferant>DATENLIEFERANT-MARKER</DatenLieferant>",
    r"<RC><Rueckgabe><Code>080015009</Code><Text>Zertifikat vom Herausgeber gesperrt</Text></Rueckgabe>",
    r"<Stack><Code>080015009</Code><Text>Zertifikat vom Herausgeber gesperrt</Text></Stack></RC>",
    r"</TransferHeader><DatenTeil><Nutzdatenblock><Nutzdaten><Steuernummer>STNR-MARKER-4711</Steuernummer>",
    r"<Bereitstellung><Rueckgabe><Code>194346573</Code><Text>NUTZDATEN-TEXT-MARKER</Text></Rueckgabe>",
    r"</Bereitstellung></Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"
);
const SERVER_NUTZDATENHEADER: &str = concat!(
    r#"<Elster xmlns="http://www.elster.de/elsterxml/schema/v11"><TransferHeader version="11">"#,
    r"<DatenLieferant>DATENLIEFERANT-MARKER</DatenLieferant>",
    r"<RC><Rueckgabe><Code>0</Code><Text>OK</Text></Rueckgabe></RC></TransferHeader>",
    r#"<DatenTeil><Nutzdatenblock><NutzdatenHeader version="11"><RC>"#,
    r"<Rueckgabe><Code>67890</Code><Text>Der Empfang von mindestens einer der Daten wurde nicht erfolgreich bestaetigt.</Text></Rueckgabe>",
    r"</RC></NutzdatenHeader><Nutzdaten><Steuernummer>STNR-MARKER-4711</Steuernummer>",
    r"<Bereitstellung><Rueckgabe><Code>194346573</Code><Text>NUTZDATEN-TEXT-MARKER</Text></Rueckgabe>",
    r"</Bereitstellung></Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"
);
const MARKER: [&str; 3] = ["STNR-MARKER-4711", "NUTZDATEN-TEXT-MARKER", "DATENLIEFERANT-MARKER"];

/// Wo die Attrappe liegt: ausserhalb von `parity/`, das nach der Abnahme geloescht wird.
const ATTRAPPE_QUELLE: &str = "../elster/tests/eric_attrappe/eric_attrappe.c";

static SPERRE: Mutex<()> = Mutex::new(());
static BIBLIOTHEK: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Die gebaute Attrappe (`libericapi.so`); `None` nur mit `TAXGRAPH_OHNE_CC=1` und ohne `cc`.
fn bibliothek() -> Option<&'static Path> {
    BIBLIOTHEK
        .get_or_init(|| {
            let lib = Path::new(env!("CARGO_TARGET_TMPDIR")).join("versand_attrappe");
            std::fs::create_dir_all(&lib).unwrap();
            let ziel = lib.join("libericapi.so");
            // Unter `cargo nextest` ist jeder Test ein eigener Prozess und baut die Attrappe selbst: jeder baut in eine
            // eigene Datei und benennt sie dann atomar auf `ziel` um. Ein Prozess, der `ziel` gerade laedt, sieht so nie
            // eine halbe Datei ("file too short").
            let eigene = lib.join(format!("libericapi.{}.tmp", std::process::id()));
            let quelle = Path::new(env!("CARGO_MANIFEST_DIR")).join(ATTRAPPE_QUELLE);
            assert!(quelle.is_file(), "die Quelle der Attrappe fehlt: {}", quelle.display());
            let gebaut = Command::new("cc")
                .args(["-shared", "-fPIC", "-O0", "-o"])
                .arg(&eigene)
                .arg(&quelle)
                .status();
            if gebaut.as_ref().is_ok_and(std::process::ExitStatus::success) {
                std::fs::rename(&eigene, &ziel).unwrap();
                return Some(ziel);
            }
            assert!(
                std::env::var("TAXGRAPH_OHNE_CC").as_deref() == Ok("1"),
                "cc baut die Attrappe nicht ({gebaut:?}); nur TAXGRAPH_OHNE_CC=1 erlaubt das Fehlen"
            );
            eprintln!("[versand_hermetisch] UEBERSPRUNGEN: kein cc, TAXGRAPH_OHNE_CC=1");
            None
        })
        .as_deref()
}

/// Ein Lauf gegen die Attrappe: eigenes Steuerverzeichnis, ein Platzhalter-Zertifikat, die Sperre.
struct Lauf {
    _sperre: MutexGuard<'static, ()>,
    wurzel: tempfile::TempDir,
    lib: PathBuf,
}

impl Lauf {
    /// `skript`: der Inhalt der Datei `skript` der Attrappe (ohne abschliessenden Zeilenumbruch).
    fn neu(skript: &str) -> Option<Self> {
        let lib = bibliothek()?.to_owned();
        let sperre = SPERRE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let wurzel = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(wurzel.path().join("steuer")).unwrap();
        std::fs::write(wurzel.path().join("steuer/skript"), skript).unwrap();
        std::fs::write(wurzel.path().join(ZERT_NAME), b"kein echtes Zertifikat").unwrap();
        std::env::set_var("ERIC_ATTRAPPE_DIR", wurzel.path().join("steuer"));
        Some(Self {
            _sperre: sperre,
            wurzel,
            lib,
        })
    }

    fn steuer(&self, name: &str) -> PathBuf {
        self.wurzel.path().join("steuer").join(name)
    }

    /// Der Platzhalter-Pfad des Zertifikats (eine Datei mit Fuelltext, nie ein echtes Zertifikat).
    fn zert_pfad(&self) -> String {
        self.wurzel.path().join(ZERT_NAME).to_string_lossy().into_owned()
    }

    fn zertifikat(&self) -> Zertifikat {
        Zertifikat::neu(self.zert_pfad(), PIN)
    }

    /// Eine Steuer- oder Aufzeichnungsdatei der Attrappe; `None`, wenn es sie nicht gibt.
    fn datei(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.steuer(name)).ok()
    }

    /// Ein Zaehler der Attrappe; `None` heisst: nie aufgerufen.
    fn zaehler(&self, name: &str) -> Option<u32> {
        self.datei(name).map(|t| t.trim().parse().unwrap())
    }

    fn sende(&self, xml: &[u8], modus: &Modus) -> Result<Antwort, VersandFehler> {
        sende(Some(&self.lib), xml, "ESt_2025", &self.zertifikat(), modus)
    }

    /// Jeder erzeugte Rueckgabepuffer wurde freigegeben (und es gab welche).
    fn puffer_ausgeglichen(&self) -> bool {
        let erzeugt = self.zaehler("puffer_erzeugt");
        erzeugt.is_some() && erzeugt == self.zaehler("puffer_freigegeben")
    }

    /// ERiC wurde nicht einmal geladen.
    fn eric_unberuehrt(&self) -> bool {
        self.zaehler("init_zaehler").is_none()
            && self.zaehler("zaehler").is_none()
            && self.zaehler("zertifikat_zaehler").is_none()
    }

    /// Das Programm: `args`, die Eingabe, ist stdin ein Terminal? Rueckgabe: Exit-Code und Ausgabe.
    fn programm(&self, args: &[&str], stdin: &str, terminal: bool) -> (i32, String) {
        Self::programm_mit(args, stdin, terminal, &self.zert_pfad(), Some(self.lib.clone()))
    }

    fn programm_mit(
        args: &[&str],
        stdin: &str,
        terminal: bool,
        zert_umgebung: &str,
        eric_lib: Option<PathBuf>,
    ) -> (i32, String) {
        let env = Umgebung {
            zertifikat_pfad: zert_umgebung.to_owned(),
            pin: PIN.to_owned(),
            eric_lib,
        };
        let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        let mut aus = Vec::new();
        let rc = lauf(&args, &env, &mut stdin.as_bytes(), terminal, &mut aus);
        let aus = String::from_utf8(aus).unwrap();
        // S8 fuer JEDEN Programmlauf: nie die PIN, nie der Zertifikatspfad in der Ausgabe.
        assert!(
            !aus.contains(PIN) && !aus.contains(ZERT_NAME),
            "PIN oder Zertifikatspfad in der Ausgabe von {args:?}:\n{aus}"
        );
        (rc, aus)
    }

    /// Schreibt ein XML ins Wegwerf-Verzeichnis (je Inhalt eine eigene Datei) und gibt den Pfad.
    fn xml_datei(&self, xml: &[u8]) -> String {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        xml.hash(&mut h);
        let p = self.wurzel.path().join(format!("fall_{:x}.xml", h.finish()));
        std::fs::write(&p, xml).unwrap();
        p.to_string_lossy().into_owned()
    }
}

fn echt(freigabe: &str) -> Modus {
    Modus::Echtversand {
        freigabe: freigabe.to_owned(),
    }
}

fn echt_frei() -> Modus {
    echt(ECHTVERSAND_FREIGABE)
}

fn erfolgs_skript() -> String {
    format!("0\n{ANTWORT_ERFOLG}\n--SERVER--\n<Annahme>ok</Annahme>")
}

// ------------------------------------------------------------------ S1: Freigabe

#[test]
fn echtversand_ohne_die_woertliche_freigabe_ruft_eric_nie() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let lower = ECHTVERSAND_FREIGABE.to_lowercase();
    let mit_blank = format!("{ECHTVERSAND_FREIGABE} ");
    let ohne_ende = &ECHTVERSAND_FREIGABE[..ECHTVERSAND_FREIGABE.len() - 1];
    for falsch in ["", "ja klar", &lower, &mit_blank, ohne_ende] {
        assert_eq!(
            l.sende(XML_ECHT, &echt(falsch)),
            Err(VersandFehler::EchtversandOhneFreigabe),
            "Freigabe {falsch:?}"
        );
    }
    assert!(l.eric_unberuehrt(), "ERiC wurde trotz fehlender Freigabe gerufen");
}

// ------------------------------------------------------------------ S2/S3: Merker

#[test]
fn ein_xml_das_nicht_zum_modus_passt_ruft_eric_nie() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let falsche_test: [&[u8]; 6] = [
        b"<Elster><TransferHeader><Testmerker>700000001</Testmerker></TransferHeader></Elster>",
        XML_ECHT,
        b"<Elster><TransferHeader><!-- <Testmerker>700000004</Testmerker> --></TransferHeader></Elster>",
        b"<Elster><Nutzdaten><Testmerker>700000004</Testmerker></Nutzdaten></Elster>",
        b"<Elster><TransferHeader><Testmerker>700000004</Testmerker><Testmerker>700000004</Testmerker></TransferHeader></Elster>",
        b"<Elster><TransferHeader><Testmerker/></TransferHeader></Elster>",
    ];
    for xml in falsche_test {
        assert!(
            matches!(
                l.sende(xml, &Modus::Testversand),
                Err(VersandFehler::XmlMerkerMismatch(_))
            ),
            "Testversand mit {}",
            String::from_utf8_lossy(xml)
        );
    }
    // Selbst mit der richtigen Freigabe: der Echtversand eines XML, das noch einen Merker traegt.
    for xml in [XML_TEST, &b"<Elster><TransferHeader><Testmerker/></TransferHeader></Elster>"[..]] {
        assert!(
            matches!(
                l.sende(xml, &echt_frei()),
                Err(VersandFehler::XmlMerkerMismatch(_))
            ),
            "Echtversand mit {}",
            String::from_utf8_lossy(xml)
        );
    }
    assert!(l.eric_unberuehrt(), "ERiC wurde trotz Merker-Mismatch gerufen");
}

#[test]
fn ein_unlesbares_xml_ruft_eric_nie() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    for xml in [&b"<Elster><TransferHeader>"[..], b"kein xml", &[0xff, 0xfe]] {
        for modus in [Modus::Testversand, echt_frei()] {
            assert_eq!(l.sende(xml, &modus), Err(VersandFehler::XmlUnlesbar));
        }
    }
    assert!(l.eric_unberuehrt());
}

// ------------------------------------------------------------------ S4/S5: Zertifikat

#[test]
fn ohne_zertifikat_oder_pin_ruft_eric_nie() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let fehlt = l.wurzel.path().join("gibt-es-nicht.pfx").to_string_lossy().into_owned();
    let faelle = [
        (Zertifikat::neu("", PIN), "Zertifikatspfad"),
        (Zertifikat::neu(fehlt, PIN), "nicht gefunden"),
        (Zertifikat::neu(l.zert_pfad(), ""), "PIN"),
    ];
    for (z, wort) in &faelle {
        let r = sende(Some(&l.lib), XML_TEST, "ESt_2025", z, &Modus::Testversand);
        match r {
            Err(VersandFehler::ZertifikatFehlt(text)) => assert!(text.contains(wort), "{text}"),
            andere => panic!("erwartet ZertifikatFehlt({wort}), bekam {andere:?}"),
        }
    }
    assert!(l.eric_unberuehrt(), "ERiC wurde trotz fehlendem Zertifikat geladen");
}

#[test]
fn lehnt_eric_das_zertifikat_ab_wird_nie_gesendet() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    std::fs::write(l.steuer("zertifikat_rc"), "610201106").unwrap(); // ERIC_CRYPT_E_PIN_WRONG
    match l.sende(XML_TEST, &Modus::Testversand) {
        Err(VersandFehler::ZertifikatFehlt(text)) => {
            assert!(text.contains("ERiC lehnt") && text.contains("610201106"), "{text}");
        }
        andere => panic!("erwartet ZertifikatFehlt, bekam {andere:?}"),
    }
    assert_eq!(l.zaehler("zertifikat_zaehler"), Some(1));
    assert_eq!(l.zaehler("zaehler"), None, "kein Sendeaufruf, wenn das Zertifikat scheitert");
    assert_eq!(l.datei("zertifikat_geschlossen"), None, "es gab nichts zu schliessen");
}

// ------------------------------------------------------------------ S9/S10: der Aufruf selbst

#[test]
fn der_testversand_kommt_mit_flags_crypto_und_xml_bei_eric_an() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let a = l.sende(XML_TEST, &Modus::Testversand).unwrap();
    assert_eq!(a.rc, 0);
    assert_eq!(a.telenummer().as_deref(), Some("N552026081012345"));
    assert!(a.erfolg());
    assert_eq!(a.rueckgabe_xml, ANTWORT_ERFOLG);
    assert_eq!(a.serverantwort_xml, "<Annahme>ok</Annahme>");

    let meta = l.datei("gesehen/1.meta").unwrap();
    // ERIC_VALIDIERE (1 << 1) | ERIC_SENDE (1 << 2) = 6; kein Druck; Crypto und Serverantwort da.
    for zeile in [
        "datenart=ESt_2025",
        "flags=6",
        "druck=NULL",
        "crypto=gesetzt",
        "serverantwort=gesetzt",
    ] {
        assert!(meta.lines().any(|z| z == zeile), "{zeile} fehlt in:\n{meta}");
    }
    // eric_verschluesselungs_parameter_t: Version 3, das Handle der Attrappe, die PIN nur als Laenge.
    assert_eq!(
        l.datei("gesehen/1.crypto").unwrap(),
        format!("version=3\nhandle=77\npin_laenge={}\n", PIN.len())
    );
    assert_eq!(std::fs::read(l.steuer("gesehen/1.xml")).unwrap(), XML_TEST, "das XML Byte fuer Byte");
    assert_eq!(l.datei("zertifikat_pfad").unwrap(), l.zert_pfad());
    assert_eq!(l.datei("zertifikat_geschlossen").unwrap(), "77\n", "das Handle wird geschlossen");
    assert_eq!(l.zaehler("zaehler"), Some(1));
    assert_eq!(l.zaehler("init_zaehler"), Some(1));
    assert_eq!(l.zaehler("beende_zaehler"), Some(1), "EricBeende am Ende");
    // eric.log nennt Auszuege der Erklaerung: das Verzeichnis gehoert nur dem Besitzer (0700).
    assert!(a.log_pfad.is_file(), "eric.log fehlt: {:?}", a.log_pfad);
    let modus = std::fs::metadata(a.log_pfad.parent().unwrap()).unwrap().permissions().mode() & 0o777;
    assert_eq!(modus, 0o700, "Log-Verzeichnis {:?}", a.log_pfad.parent());
    assert_eq!(l.zaehler("puffer_erzeugt"), Some(2), "Rueckgabe- und Serverantwort-Puffer");
    assert!(l.puffer_ausgeglichen(), "ein Puffer wurde nicht freigegeben");
}

#[test]
fn der_echtversand_mit_freigabe_und_sauberem_xml_geht_durch() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let a = l.sende(XML_ECHT, &echt_frei()).unwrap();
    assert!(a.erfolg());
    assert!(l.datei("gesehen/1.meta").unwrap().lines().any(|z| z == "flags=6"));
    assert_eq!(std::fs::read(l.steuer("gesehen/1.xml")).unwrap(), XML_ECHT);
}

#[test]
fn ein_misserfolg_bleibt_ein_misserfolg_und_raeumt_auf() {
    let Some(l) = Lauf::neu("610101271\n<Elster><Fehler/></Elster>") else { return };
    let a = l.sende(XML_TEST, &Modus::Testversand).unwrap();
    assert_eq!(a.rc, 610_101_271);
    assert_eq!(a.telenummer(), None);
    assert!(!a.erfolg());
    assert_eq!(l.datei("zertifikat_geschlossen").unwrap(), "77\n");
    assert_eq!(l.zaehler("beende_zaehler"), Some(1));
    assert!(l.puffer_ausgeglichen(), "ein Puffer wurde nicht freigegeben");
}

#[test]
fn rc_null_ohne_telenummer_ist_kein_erfolg() {
    let Some(l) = Lauf::neu("0\n<Elster><Erfolg/></Elster>") else { return };
    let a = l.sende(XML_TEST, &Modus::Testversand).unwrap();
    assert_eq!(a.rc, 0);
    assert!(!a.erfolg());
}

#[test]
fn ein_ladefehler_von_eric_ruft_weder_zertifikat_noch_senden() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    std::fs::write(l.steuer("init_rc"), "5").unwrap();
    assert_eq!(
        l.sende(XML_TEST, &Modus::Testversand),
        Err(VersandFehler::Eric(EricFehler::Init(5)))
    );
    assert_eq!(l.zaehler("zertifikat_zaehler"), None);
    assert_eq!(l.zaehler("zaehler"), None);
}

#[test]
fn scheitert_das_anheben_der_meldungsgrenze_wird_nie_gesendet() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    std::fs::write(l.steuer("einstellung_rc"), "3").unwrap();
    assert!(matches!(
        l.sende(XML_TEST, &Modus::Testversand),
        Err(VersandFehler::Eric(EricFehler::Einstellung { rc: 3, .. }))
    ));
    assert_eq!(l.zaehler("zertifikat_zaehler"), None);
    assert_eq!(l.zaehler("zaehler"), None);
}

#[test]
fn die_sperren_gehen_der_suche_nach_eric_voraus() {
    // Ohne Bibliothek: mit guten Eingaben `NichtGefunden`, mit schlechter Freigabe die Freigabe.
    let wurzel = tempfile::tempdir().unwrap();
    let zert = wurzel.path().join(ZERT_NAME);
    std::fs::write(&zert, b"x").unwrap();
    let z = Zertifikat::neu(&zert, PIN);
    assert_eq!(
        sende(None, XML_TEST, "ESt_2025", &z, &Modus::Testversand),
        Err(VersandFehler::Eric(EricFehler::NichtGefunden))
    );
    assert_eq!(
        sende(None, XML_ECHT, "ESt_2025", &z, &echt("ja")),
        Err(VersandFehler::EchtversandOhneFreigabe)
    );
}

#[test]
fn ein_nul_byte_in_datenart_oder_pin_ruft_eric_nie() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let r = sende(Some(&l.lib), XML_TEST, "ESt\0_2025", &l.zertifikat(), &Modus::Testversand);
    assert_eq!(r, Err(VersandFehler::NulByte));
    let z = Zertifikat::neu(l.zert_pfad(), "pin\0x");
    let r = sende(Some(&l.lib), XML_TEST, "ESt_2025", &z, &Modus::Testversand);
    assert_eq!(r, Err(VersandFehler::NulByte));
    assert!(l.eric_unberuehrt());
}

// ------------------------------------------------------------------ S8: nie Pfad, nie PIN

#[test]
fn weder_pfad_noch_pin_stehen_je_in_einer_ausgabe() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let z = l.zertifikat();
    let mut texte: Vec<String> = vec![format!("{z:?}")];
    let fehler_faelle: Vec<Result<Antwort, VersandFehler>> = vec![
        l.sende(XML_ECHT, &echt("falsch")),
        l.sende(XML_ECHT, &Modus::Testversand),
        l.sende(b"kein xml", &Modus::Testversand),
        sende(Some(&l.lib), XML_TEST, "ESt_2025", &Zertifikat::neu("", PIN), &Modus::Testversand),
        sende(
            Some(&l.lib),
            XML_TEST,
            "ESt_2025",
            &Zertifikat::neu(format!("{}.fehlt", l.zert_pfad()), PIN),
            &Modus::Testversand,
        ),
        sende(Some(&l.lib), XML_TEST, "ESt_2025", &Zertifikat::neu(l.zert_pfad(), ""), &Modus::Testversand),
        sende(None, XML_TEST, "ESt_2025", &z, &Modus::Testversand),
    ];
    for r in fehler_faelle {
        let e = r.unwrap_err();
        texte.push(format!("{e} | {e:?}"));
    }
    std::fs::write(l.steuer("zertifikat_rc"), "610201106").unwrap();
    texte.push(format!("{:?}", l.sende(XML_TEST, &Modus::Testversand).unwrap_err()));
    std::fs::remove_file(l.steuer("zertifikat_rc")).unwrap();
    let a = l.sende(XML_TEST, &Modus::Testversand).unwrap();
    texte.push(format!("{a:?}"));
    texte.push(format!("{:?}", versand::zusammenfassung(XML_TEST, "ESt_2025", &Modus::Testversand, &z)));
    let umgebung = Umgebung {
        zertifikat_pfad: l.zert_pfad(),
        pin: PIN.to_owned(),
        eric_lib: None,
    };
    texte.push(format!("{umgebung:?}"));
    assert!(texte.len() >= 10);
    for t in &texte {
        assert!(!t.contains(PIN), "die PIN steht in: {t}");
        assert!(!t.contains(ZERT_NAME), "der Zertifikatspfad steht in: {t}");
    }
}

// ------------------------------------------------------------------ S6: das Programm

#[test]
fn das_programm_im_dry_run_laedt_eric_nie_und_zeigt_weder_pfad_noch_pin() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let datei = l.xml_datei(XML_TEST);
    let (rc, aus) = l.programm(&["--xml", &datei, "--datenart", "ESt_2025", "--dry-run"], "", false);
    assert_eq!(rc, 0);
    assert!(aus.contains("DRY-RUN") && aus.contains("NICHTS gesendet"), "{aus}");
    assert!(aus.contains("merker_konsistent: true") && aus.contains("zertifikat_vorhanden: true"), "{aus}");
    assert!(!aus.contains(PIN) && !aus.contains(ZERT_NAME), "{aus}");
    assert!(l.eric_unberuehrt(), "der Dry-run hat ERiC geladen");

    let datei = l.xml_datei(XML_ECHT);
    let (rc, aus) = l.programm(&["--xml", &datei, "--datenart", "ESt_2025", "--dry-run"], "", false);
    assert_eq!(rc, 0);
    assert!(aus.contains("merker_konsistent: false") && aus.contains("WARNUNG"), "{aus}");
    assert!(l.eric_unberuehrt());

    // Zwei Testmerker: die Anzeige nennt den ERSTEN (`merker_im_xml`), der Modus ist inkonsistent.
    let zwei = b"<Elster><TransferHeader><Testmerker>700000004</Testmerker>\
<Testmerker>700000009</Testmerker></TransferHeader></Elster>";
    let datei = l.xml_datei(zwei);
    let (rc, aus) = l.programm(&["--xml", &datei, "--datenart", "ESt_2025", "--dry-run"], "", false);
    assert_eq!(rc, 0);
    assert!(aus.contains("merker_im_xml: 700000004"), "{aus}");
    assert!(!aus.contains("700000009"), "die Anzeige nennt nicht den ersten Merker: {aus}");
    assert!(aus.contains("merker_konsistent: false") && aus.contains("WARNUNG"), "{aus}");
    assert!(l.eric_unberuehrt());
}

#[test]
fn das_programm_verweigert_den_echtversand_ohne_die_freigabe_option() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let datei = l.xml_datei(XML_ECHT);
    let phrase = format!("{ECHTVERSAND_FREIGABE}\n");
    let klein = ECHTVERSAND_FREIGABE.to_lowercase();
    for freigabe in [None, Some("ja klar"), Some(klein.as_str())] {
        let mut args = vec!["--xml", &datei, "--datenart", "ESt_2025", "--echtversand"];
        if let Some(f) = freigabe {
            args.extend(["--freigabe", f]);
        }
        let (rc, aus) = l.programm(&args, &phrase, true);
        assert_eq!(rc, 2, "{freigabe:?}: {aus}");
        assert!(aus.contains("VERWEIGERT"), "{aus}");
    }
    assert!(l.eric_unberuehrt());
}

#[test]
fn das_programm_verlangt_ein_terminal_und_die_zweite_eingabe() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let datei = l.xml_datei(XML_ECHT);
    let args = [
        "--xml", &datei, "--datenart", "ESt_2025", "--echtversand", "--freigabe", ECHTVERSAND_FREIGABE,
    ];
    let richtig = format!("{ECHTVERSAND_FREIGABE}\n");
    // Kein Terminal: auch die richtige Phrase auf stdin gilt nicht (strenger als Python).
    let (rc, aus) = l.programm(&args, &richtig, false);
    assert_eq!(rc, 2);
    assert!(aus.contains("kein Terminal"), "{aus}");
    // Terminal, aber nichts eingegeben (EOF).
    let (rc, aus) = l.programm(&args, "", true);
    assert_eq!(rc, 2);
    assert!(aus.contains("VERWEIGERT"), "{aus}");
    // Terminal, falsche Eingabe — auch die Phrase mit Anhang oder in Kleinschrift.
    for falsch in ["ja\n", &format!("{ECHTVERSAND_FREIGABE} \n"), &richtig.to_lowercase()] {
        let (rc, aus) = l.programm(&args, falsch, true);
        assert_eq!(rc, 2, "{falsch:?}");
        assert!(aus.contains("stimmte nicht ueberein"), "{aus}");
    }
    assert!(l.eric_unberuehrt(), "ERiC wurde trotz verweigerter Huerde gerufen");
}

#[test]
fn das_programm_sendet_den_echtversand_nach_beiden_huerden() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let datei = l.xml_datei(XML_ECHT);
    let args = [
        "--xml", &datei, "--datenart", "ESt_2025", "--echtversand", "--freigabe", ECHTVERSAND_FREIGABE,
    ];
    let (rc, aus) = l.programm(&args, &format!("{ECHTVERSAND_FREIGABE}\n"), true);
    assert_eq!(rc, 0, "{aus}");
    assert!(aus.contains("[versand] rc=0") && aus.contains("Telenummer: N552026081012345"), "{aus}");
    assert!(!aus.contains(PIN) && !aus.contains(ZERT_NAME), "{aus}");
    assert_eq!(l.zaehler("zaehler"), Some(1));
    assert!(l.datei("gesehen/1.meta").unwrap().lines().any(|z| z == "flags=6"));
}

#[test]
fn das_programm_testversand_braucht_keine_eingabe_und_kein_terminal() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let datei = l.xml_datei(XML_TEST);
    let (rc, aus) = l.programm(&["--xml", &datei, "--datenart", "ESt_2025", "--testversand"], "", false);
    assert_eq!(rc, 0, "{aus}");
    assert!(aus.contains("Telenummer: N552026081012345"), "{aus}");
    assert_eq!(l.zaehler("zaehler"), Some(1));
    // `--name=wert` und die Option `--zertifikat` (sie hat Vorrang vor der Umgebung).
    let xml_option = format!("--xml={datei}");
    let zert_option = format!("--zertifikat={}", l.zert_pfad());
    let (rc, aus) = Lauf::programm_mit(
        &[&xml_option, "--datenart=ESt_2025", &zert_option, "--testversand"],
        "",
        false,
        "/nicht/da.pfx",
        Some(l.lib.clone()),
    );
    assert_eq!(rc, 0, "{aus}");
    assert_eq!(l.datei("zertifikat_pfad").unwrap(), l.zert_pfad());
}

#[test]
fn das_programm_meldet_einen_fehlschlag_mit_exit_eins_und_dem_rueckgabetext() {
    let Some(l) = Lauf::neu("610101271\n<Elster><Fehler>TEXT-VON-ERIC</Fehler></Elster>") else { return };
    let datei = l.xml_datei(XML_TEST);
    let (rc, aus) = l.programm(&["--xml", &datei, "--datenart", "ESt_2025", "--testversand"], "", false);
    assert_eq!(rc, 1, "{aus}");
    assert!(aus.contains("KEIN ERFOLG") && aus.contains("NICHT blind wiederholen"), "{aus}");
    assert!(aus.contains("TEXT-VON-ERIC") && aus.contains("eric.log"), "{aus}");
    assert!(aus.contains("Telenummer: (keine"), "{aus}");
    assert!(!aus.contains(PIN) && !aus.contains(ZERT_NAME), "{aus}");
}

/// Der Testversand gegen die Attrappe, die `rc` und die Serverantwort liefert.
fn ablehnung(rc: i32, rueckgabe: &str, server: &str) -> Option<(i32, String)> {
    let l = Lauf::neu(&format!("{rc}\n{rueckgabe}\n--SERVER--\n{server}"))?;
    let datei = l.xml_datei(XML_TEST);
    Some(l.programm(&["--xml", &datei, "--datenart", "ESt_2025", "--testversand"], "", false))
}

#[test]
fn das_programm_nennt_bei_einer_serverablehnung_code_und_text_aber_nie_den_datenteil() {
    for (rc, server, code, text) in [
        (610_101_210, SERVER_TRANSFERHEADER, "080015009", "Zertifikat vom Herausgeber gesperrt"),
        (
            610_101_292,
            SERVER_NUTZDATENHEADER,
            "67890",
            "Der Empfang von mindestens einer der Daten wurde nicht erfolgreich bestaetigt.",
        ),
    ] {
        let Some((exit, aus)) = ablehnung(rc, "", server) else { return };
        assert_eq!(exit, 1, "{aus}");
        assert!(aus.contains(&format!("[versand] rc={rc}")), "{aus}");
        let zeile = format!("[versand]   Code {code}: {text}");
        assert_eq!(aus.matches(&zeile).count(), 1, "Code und Text genau einmal erwartet:\n{aus}");
        assert!(aus.contains("Antwort des ELSTER-Servers"), "{aus}");
        for marker in MARKER {
            assert!(!aus.contains(marker), "{marker} steht in der Ausgabe:\n{aus}");
        }
    }
}

#[test]
fn eine_serverablehnung_ohne_auswertbare_meldung_verweist_auf_das_log() {
    for server in ["", "kein xml", "<Elster><TransferHeader/></Elster>"] {
        let Some((exit, aus)) = ablehnung(610_101_210, "", server) else { return };
        assert_eq!(exit, 1, "{aus}");
        assert!(aus.contains("nennt keinen auswertbaren Fehler") && aus.contains("eric.log"), "{aus}");
        assert!(!aus.contains("Antwort des ELSTER-Servers"), "{aus}");
    }
}

#[test]
fn jeder_andere_rc_behaelt_seine_ausgabe_zeichengleich_auch_mit_serverantwort() {
    // 610101271 ist kein Fehler des Servers im Header: die Serverantwort wird nicht ausgewertet.
    let Some((exit, aus)) =
        ablehnung(610_101_271, "<Elster><Fehler/></Elster>", SERVER_TRANSFERHEADER)
    else {
        return;
    };
    assert_eq!(exit, 1, "{aus}");
    let (kopf, log) = aus.split_once("[versand] eric.log: ").expect("Zeile mit eric.log fehlt");
    assert_eq!(
        kopf,
        "[versand] rc=610101271\n\
         [versand] Telenummer: (keine — kein Erfolg)\n\
         [versand] KEIN ERFOLG. Siehe elster/VERSAND.md, Abschnitt 'Wenn etwas schiefgeht' — NICHT blind wiederholen.\n\
         [versand] Rueckgabe von ERiC:\n\
         <Elster><Fehler/></Elster>\n"
    );
    assert!(log.trim_end().ends_with("eric.log"), "{log}");
}

#[test]
fn das_programm_meldet_rc_null_ohne_telenummer_als_kein_erfolg() {
    // rc 0 allein ist kein Erfolg: ohne Telenummer Exit 1 und "KEIN ERFOLG" (nicht Exit 0).
    let Some(l) = Lauf::neu("0\n<Elster><Erfolg/></Elster>") else { return };
    let datei = l.xml_datei(XML_TEST);
    let (rc, aus) = l.programm(&["--xml", &datei, "--datenart", "ESt_2025", "--testversand"], "", false);
    assert_eq!(rc, 1, "{aus}");
    assert!(aus.contains("[versand] rc=0") && aus.contains("KEIN ERFOLG"), "{aus}");
    assert!(aus.contains("Telenummer: (keine"), "{aus}");
    assert_eq!(l.zaehler("zaehler"), Some(1), "ERiC wurde gerufen, der Versand war nur kein Erfolg");
}

#[test]
fn das_programm_bricht_bei_einer_sperre_ab_und_ruft_eric_nie() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let test_xml = l.xml_datei(XML_TEST);
    let echt_xml = l.xml_datei(XML_ECHT);
    // Testversand mit einem XML ohne Merker; Echtversand mit beiden Huerden, aber einem XML mit Merker.
    let (rc, aus) = l.programm(&["--xml", &echt_xml, "--datenart", "ESt_2025", "--testversand"], "", false);
    assert_eq!(rc, 2, "{aus}");
    assert!(aus.contains("ABGEBROCHEN"), "{aus}");
    let phrase = format!("{ECHTVERSAND_FREIGABE}\n");
    let (rc, aus) = l.programm(
        &["--xml", &test_xml, "--datenart", "ESt_2025", "--echtversand", "--freigabe", ECHTVERSAND_FREIGABE],
        &phrase,
        true,
    );
    assert_eq!(rc, 2, "{aus}");
    assert!(aus.contains("ABGEBROCHEN"), "{aus}");
    // Ohne Zertifikat in der Umgebung.
    let (rc, aus) = Lauf::programm_mit(
        &["--xml", &test_xml, "--datenart", "ESt_2025", "--testversand"],
        "",
        false,
        "",
        Some(l.lib.clone()),
    );
    assert_eq!(rc, 2, "{aus}");
    assert!(aus.contains("ABGEBROCHEN") && aus.contains("Zertifikatspfad"), "{aus}");
    assert!(l.eric_unberuehrt());
}

#[test]
fn das_programm_ohne_bibliothek_bricht_erst_nach_den_sperren_ab() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let datei = l.xml_datei(XML_TEST);
    let (rc, aus) = Lauf::programm_mit(
        &["--xml", &datei, "--datenart", "ESt_2025", "--testversand"],
        "",
        false,
        &l.zert_pfad(),
        None,
    );
    assert_eq!(rc, 2, "{aus}");
    assert!(aus.contains("libericapi.so nicht gefunden"), "{aus}");
}

#[test]
fn das_programm_meldet_nutzungsfehler_mit_exit_zwei_und_liest_nichts() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let datei = l.xml_datei(XML_TEST);
    let faelle: Vec<Vec<&str>> = vec![
        vec![],
        vec!["--xml", &datei],
        vec!["--xml", &datei, "--datenart", "ESt_2025"],
        vec!["--xml", &datei, "--datenart", "ESt_2025", "--dry-run", "--testversand"],
        vec!["--xml", &datei, "--datenart", "ESt_2025", "--echtversand", "--testversand"],
        vec!["--xml", &datei, "--datenart", "ESt_2025", "--unbekannt"],
        vec!["--datenart", "ESt_2025", "--testversand", "--xml"],
        vec!["--xml", "/gibt/es/nicht.xml", "--datenart", "ESt_2025", "--testversand"],
    ];
    for args in faelle {
        let (rc, aus) = l.programm(&args, "", false);
        assert_eq!(rc, 2, "{args:?}: {aus}");
    }
    assert!(l.eric_unberuehrt());
}

// ------------------------------------------------------------------ main.rs: der echte Prozess

impl Lauf {
    /// Das gebaute Programm als Prozess: eigene Umgebung (nur diese Variablen, `HOME` im
    /// Wegwerf-Verzeichnis, damit ERiC NIE unter `~/02_Software` gesucht wird), stdin ist /dev/null.
    fn prozess(&self, args: &[&str], mit_pin: bool) -> (Option<i32>, String) {
        let lib_dir = self.lib.parent().unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_taxgraph-versand"));
        cmd.args(args)
            .env_clear()
            .env("HOME", self.wurzel.path())
            .env("ERIC_DIR", lib_dir)
            .env("ERIC_ATTRAPPE_DIR", self.wurzel.path().join("steuer"))
            .env("ELSTER_ZERTIFIKAT_PFAD", self.zert_pfad())
            .stdin(std::process::Stdio::null());
        if mit_pin {
            cmd.env("ELSTER_ZERTIFIKAT_PIN", PIN);
        }
        let out = cmd.output().unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(!text.contains(PIN) && !text.contains(ZERT_NAME), "{text}");
        (out.status.code(), text)
    }
}

#[test]
fn das_programm_liest_die_umgebung_und_setzt_den_exit_code() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let datei = l.xml_datei(XML_TEST);
    // Mit Pfad und PIN aus der Umgebung und ERiC ueber $ERIC_DIR: ein Testversand gegen die Attrappe.
    let (code, aus) = l.prozess(&["--xml", &datei, "--datenart", "ESt_2025", "--testversand"], true);
    assert_eq!(code, Some(0), "{aus}");
    assert!(aus.contains("Telenummer: N552026081012345"), "{aus}");
    assert_eq!(l.datei("gesehen/1.crypto").unwrap(), format!("version=3\nhandle=77\npin_laenge={}\n", PIN.len()));
    // Ohne PIN in der Umgebung bricht es vor ERiC ab.
    let (code, aus) = l.prozess(&["--xml", &datei, "--datenart", "ESt_2025", "--testversand"], false);
    assert_eq!(code, Some(2), "{aus}");
    assert!(aus.contains("Keine PIN gesetzt"), "{aus}");
    assert_eq!(l.zaehler("zaehler"), Some(1), "der zweite Lauf hat ERiC nicht mehr gerufen");
}

#[test]
fn das_programm_als_prozess_erkennt_kein_terminal() {
    let Some(l) = Lauf::neu(&erfolgs_skript()) else { return };
    let datei = l.xml_datei(XML_ECHT);
    let args = ["--xml", &datei, "--datenart", "ESt_2025", "--echtversand", "--freigabe", ECHTVERSAND_FREIGABE];
    let (code, aus) = l.prozess(&args, true);
    assert_eq!(code, Some(2), "{aus}");
    assert!(aus.contains("kein Terminal"), "{aus}");
    assert!(l.eric_unberuehrt());
}

// ------------------------------------------------------------------ Nr. 91, AK1: EricDekodiereDaten

/// Ein erfundenes Datenpaket: die Attrappe entschluesselt nichts, sie notiert die Eingabe.
const PAKET: &str = "TUlBR0NTcUdTSWIzRFFFSEE2Q0FNSUFDQVFBeA==";
const KLARTEXT: &[u8] = b"<VaSt_LStB version=\"202501\"><Besteuerungsgrund/></VaSt_LStB>";

impl Lauf {
    fn dekodiere(&self, paket: &str) -> Result<Vec<u8>, AbrufFehler> {
        dekodiere_datenpaket(Some(&self.lib), &self.zertifikat(), paket)
    }
}

#[test]
fn die_dekodierung_gibt_den_klartext_des_datenpakets_zurueck() {
    let Some(l) = Lauf::neu("") else { return };
    std::fs::write(l.steuer("dekodiert"), KLARTEXT).unwrap();
    assert_eq!(l.dekodiere(PAKET).unwrap(), KLARTEXT);
    // ERiC bekam das Datenpaket Byte fuer Byte, das Handle der Attrappe und die PIN nur als Laenge.
    assert_eq!(l.datei("dekodiere_eingabe").unwrap(), PAKET);
    assert_eq!(
        l.datei("dekodiere_crypto").unwrap(),
        format!("handle=77\npin_laenge={}\n", PIN.len())
    );
    assert_eq!(l.zaehler("dekodiere_zaehler"), Some(1));
    assert_eq!(l.zaehler("zaehler"), None, "Dekodieren sendet nichts");
    assert_eq!(l.datei("zertifikat_geschlossen").unwrap(), "77\n");
    assert_eq!(l.zaehler("beende_zaehler"), Some(1));
    assert_eq!(l.zaehler("puffer_erzeugt"), Some(1));
    assert!(l.puffer_ausgeglichen(), "ein Puffer wurde nicht freigegeben");
}

#[test]
fn die_dekodierung_liest_die_ganze_laenge_auch_hinter_einem_nul_byte() {
    let Some(l) = Lauf::neu("") else { return };
    let klar: &[u8] = b"\x1f\x8b\x08\x00ABC\x00D";
    std::fs::write(l.steuer("dekodiert"), klar).unwrap();
    assert_eq!(l.dekodiere(PAKET).unwrap(), klar);
}

#[test]
fn ein_fehlercode_von_eric_ist_ein_benannter_fehler_ohne_paket_und_pin() {
    let Some(l) = Lauf::neu("") else { return };
    std::fs::write(l.steuer("dekodiere_rc"), "610001862").unwrap(); // ERIC_GLOBAL_ERR_DEKODIEREN
    // Die Attrappe schreibt den Puffer auch im Fehlerfall (ERiC: Inhalt undefiniert); er darf nie durchkommen.
    std::fs::write(l.steuer("dekodiert"), KLARTEXT).unwrap();
    let fehler = l.dekodiere(PAKET).unwrap_err();
    assert_eq!(fehler, AbrufFehler::Dekodieren { rc: 610_001_862 });
    let text = fehler.to_string();
    assert!(text.contains("610001862") && !text.contains(PAKET) && !text.contains(PIN), "{text}");
    assert_eq!(l.datei("zertifikat_geschlossen").unwrap(), "77\n", "auch im Fehlerfall zu");
    assert_eq!(l.zaehler("beende_zaehler"), Some(1));
    assert!(l.puffer_ausgeglichen(), "ein Puffer wurde nicht freigegeben");
}

#[test]
fn ohne_zertifikat_oder_pin_ruft_die_dekodierung_eric_nie() {
    let Some(l) = Lauf::neu("") else { return };
    let faelle = [
        (Zertifikat::neu("", PIN), "Zertifikatspfad"),
        (Zertifikat::neu(l.zert_pfad(), ""), "PIN"),
    ];
    for (z, wort) in &faelle {
        match dekodiere_datenpaket(Some(&l.lib), z, PAKET) {
            Err(AbrufFehler::Zertifikat(text)) => assert!(text.contains(wort), "{text}"),
            andere => panic!("erwartet Zertifikat({wort}), bekam {andere:?}"),
        }
    }
    assert!(l.eric_unberuehrt(), "ERiC wurde trotz fehlendem Zertifikat geladen");
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
}

#[test]
fn ein_leeres_datenpaket_oder_ein_nul_byte_ruft_eric_nie() {
    let Some(l) = Lauf::neu("") else { return };
    assert_eq!(l.dekodiere(""), Err(AbrufFehler::DatenpaketLeer));
    assert_eq!(l.dekodiere("AB\0CD"), Err(AbrufFehler::DatenpaketNulByte));
    assert!(l.eric_unberuehrt());
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
}

#[test]
fn ladefehler_und_zertifikatsfehler_von_eric_kommen_benannt_und_ohne_dekodierung() {
    let Some(l) = Lauf::neu("") else { return };
    std::fs::write(l.steuer("init_rc"), "5").unwrap();
    assert_eq!(l.dekodiere(PAKET), Err(AbrufFehler::Eric(EricFehler::Init(5))));
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
    std::fs::write(l.steuer("init_rc"), "0").unwrap();
    std::fs::write(l.steuer("zertifikat_rc"), "610201106").unwrap(); // ERIC_CRYPT_E_PIN_WRONG
    match l.dekodiere(PAKET) {
        Err(AbrufFehler::Zertifikat(text)) => {
            assert!(text.contains("ERiC lehnt") && text.contains("610201106"), "{text}");
        }
        andere => panic!("erwartet Zertifikat, bekam {andere:?}"),
    }
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
}

// ------------------------------------------------------------------ Nr. 91, AK3: der Abruf und das Programm

// Erfundene Werte: keine echte Steuer-ID; 74931 ist der Platzhalter der ERiC-Beispiele, keine echte Hersteller-ID.
const IDNR: &str = "12345678903";
const HERSTELLER: &str = "74931";
const ID_A: &str = "xy123abcdefghijkmnopqrstuvwxyz12";
const ID_C: &str = "ab001abcdefghijkmnopqrstuvwxyz12";
const HEUTE: i32 = 2026;

fn auftrag() -> AbrufAuftrag {
    AbrufAuftrag::neu(IDNR, "2025", HERSTELLER, HEUTE).unwrap()
}

/// Die Antwort auf die Anfrage (Aufbau: amtliches Beispiel `4_b_ElsterVaStDaten_32_Anfrage_Response.xml`).
fn anfrage_antwort(ids: &[(&str, &str, &str)]) -> String {
    use std::fmt::Write as _;
    let mut eintraege = String::new();
    for (id, art, version) in ids {
        write!(
            eintraege,
            r#"<Id id="{id}" groesse="1500" belegart="{art}" hashwert="680f" schemaversion="{version}"/>"#
        )
        .unwrap();
    }
    format!(
        r#"<Elster xmlns="http://www.elster.de/elsterxml/schema/v11"><DatenTeil><Nutzdatenblock><Nutzdaten><Datenabholung xmlns="http://finkonsens.de/elster/elsterdatenabholung/v3" version="32"><Anfrage idnr="{IDNR}" veranlagungsjahr="2025">{eintraege}</Anfrage></Datenabholung></Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"#
    )
}

/// Die Antwort auf die Abholung (Aufbau: `5_b1_Einzelbescheid_ElsterVaStDaten_32_Abholung_Response.xml`).
fn abholung_antwort(paket: &str) -> String {
    format!(
        r#"<Elster xmlns="http://www.elster.de/elsterxml/schema/v11"><DatenTeil><Nutzdatenblock><Nutzdaten><Datenabholung xmlns="http://finkonsens.de/elster/elsterdatenabholung/v3" version="32"><Abholung id="{ID_C}" idnr="{IDNR}" veranlagungsjahr="2025"><Datenpaket>{paket}</Datenpaket></Abholung></Datenabholung></Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"#
    )
}

/// Das Skript der Attrappe: ein Block je Aufruf von `EricBearbeiteVorgang` (Anfrage, dann Abholung).
fn abruf_skript(anfrage: &str, abholung: &str) -> String {
    format!("0\n<Erfolg/>\n--SERVER--\n{anfrage}\n=====\n0\n<Erfolg/>\n--SERVER--\n{abholung}")
}

fn gute_anfrage() -> String {
    anfrage_antwort(&[(ID_A, "VaSt_RBM", "202501"), (ID_C, "VaSt_LStB", "202501")])
}

/// Das Datenpaket in der Antwort steht in Zeilen (wie im amtlichen Beispiel); ERiC bekommt es ohne Leerraum.
fn gutes_skript() -> String {
    let umbrochen = format!("{}\n    {}\n  ", &PAKET[..10], &PAKET[10..]);
    abruf_skript(&gute_anfrage(), &abholung_antwort(&umbrochen))
}

fn modus_von(pfad: &Path) -> u32 {
    std::fs::metadata(pfad).unwrap().permissions().mode() & 0o777
}

impl Lauf {
    fn log_dir(&self) -> PathBuf {
        let d = self.wurzel.path().join("log");
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn hole(&self) -> Result<Vec<u8>, AbrufFehler> {
        hole_beleg(Some(&self.lib), &self.zertifikat(), &auftrag(), &self.log_dir())
    }

    /// Das Programm `taxgraph-abruf` mit der Hersteller-ID aus der Umgebung `hersteller`; die
    /// Wegwerf-Verzeichnisse fuer ERiC-Logs liegen unter `wurzel/temp`.
    fn abruf_programm_mit(
        &self,
        hersteller: &str,
        args: &[&str],
        stdin: &str,
        terminal: bool,
    ) -> (i32, String) {
        let temp = self.wurzel.path().join("temp");
        std::fs::create_dir_all(&temp).unwrap();
        let env = AbrufUmgebung {
            zertifikat_pfad: self.zert_pfad(),
            pin: PIN.to_owned(),
            hersteller_id: hersteller.to_owned(),
            eric_lib: Some(self.lib.clone()),
            heute_jahr: HEUTE,
            temp,
        };
        let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        let mut aus = Vec::new();
        let rc = lauf_abruf(&args, &env, &mut stdin.as_bytes(), terminal, &mut aus);
        let aus = String::from_utf8(aus).unwrap();
        // Fuer JEDEN Programmlauf: weder PIN, Zertifikatspfad, Steuer-ID, Hersteller-ID noch Datenpaket in der Ausgabe.
        for geheim in [PIN, ZERT_NAME, IDNR, HERSTELLER, PAKET] {
            assert!(!aus.contains(geheim), "{geheim:?} in der Ausgabe von {args:?}:\n{aus}");
        }
        // Der Pfad des Belegs steht da, sein Inhalt nie.
        let inhalt = String::from_utf8_lossy(KLARTEXT);
        assert!(!aus.contains(&*inhalt), "der Beleg steht in der Ausgabe von {args:?}:\n{aus}");
        (rc, aus)
    }

    fn abruf_programm(&self, args: &[&str], stdin: &str, terminal: bool) -> (i32, String) {
        self.abruf_programm_mit(HERSTELLER, args, stdin, terminal)
    }

    /// Der echte Lauf mit beiden Huerden, Ziel `ziel`.
    fn abruf_echt(&self, ziel: &str) -> (i32, String) {
        let tippen = format!("{ABRUF_FREIGABE}\n");
        self.abruf_programm(
            &["--idnr", IDNR, "--jahr", "2025", "--ziel", ziel, "--abruf", "--freigabe", ABRUF_FREIGABE],
            &tippen,
            true,
        )
    }
}

#[test]
fn der_abruf_geht_durch_anfrage_abholung_und_dekodierung() {
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    std::fs::write(l.steuer("dekodiert"), KLARTEXT).unwrap();
    assert_eq!(l.hole().unwrap(), KLARTEXT);
    assert_eq!(l.zaehler("zaehler"), Some(2), "Anfrage und Abholung");
    let a = auftrag();
    assert_eq!(std::fs::read_to_string(l.steuer("gesehen/1.xml")).unwrap(), a.anfrage_xml());
    // Die Abholung nennt die Id des Belegs VaSt_LStB, nicht die des anderen Eintrags.
    assert_eq!(
        std::fs::read_to_string(l.steuer("gesehen/2.xml")).unwrap(),
        a.abholung_xml(ID_C).unwrap()
    );
    for n in [1, 2] {
        let meta = l.datei(&format!("gesehen/{n}.meta")).unwrap();
        // ERIC_VALIDIERE (1 << 1) | ERIC_SENDE (1 << 2) = 6; send-Auth; kein Druck.
        for zeile in [
            "datenart=ElsterVaStDaten_32",
            "flags=6",
            "druck=NULL",
            "crypto=gesetzt",
            "serverantwort=gesetzt",
        ] {
            assert!(meta.lines().any(|z| z == zeile), "{zeile} fehlt in Aufruf {n}:\n{meta}");
        }
        assert_eq!(
            l.datei(&format!("gesehen/{n}.crypto")).unwrap(),
            format!("version=3\nhandle=77\npin_laenge={}\n", PIN.len())
        );
    }
    assert_eq!(l.datei("dekodiere_eingabe").unwrap(), PAKET, "das Datenpaket ohne Leerraum");
    assert_eq!(
        l.datei("dekodiere_crypto").unwrap(),
        format!("handle=77\npin_laenge={}\n", PIN.len()),
        "die Dekodierung bekommt dasselbe Zertifikat und dieselbe PIN wie die Aufrufe davor"
    );
    assert_eq!(l.zaehler("dekodiere_zaehler"), Some(1));
    assert_eq!(l.zaehler("zertifikat_zaehler"), Some(1), "ein Zertifikat, einmal geoeffnet");
    assert_eq!(l.datei("zertifikat_geschlossen").unwrap(), "77\n");
    assert_eq!(l.zaehler("init_zaehler"), Some(1));
    assert_eq!(l.zaehler("beende_zaehler"), Some(1));
    assert_eq!(l.zaehler("puffer_erzeugt"), Some(5), "zwei Aufrufe mit je zwei Puffern und die Dekodierung");
    assert!(l.puffer_ausgeglichen(), "ein Puffer wurde nicht freigegeben");
}

#[test]
fn ohne_passenden_beleg_oder_mit_mehreren_wird_nie_abgeholt() {
    let ohne = anfrage_antwort(&[(ID_A, "VaSt_RBM", "202501"), (ID_C, "VaSt_LStB", "202401")]);
    let Some(l) = Lauf::neu(&abruf_skript(&ohne, &abholung_antwort(PAKET))) else { return };
    assert!(matches!(l.hole(), Err(AbrufFehler::KeinBeleg { .. })));
    assert_eq!(l.zaehler("zaehler"), Some(1), "nur die Anfrage ging raus");
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
    assert_eq!(l.datei("zertifikat_geschlossen").unwrap(), "77\n");
    assert!(l.puffer_ausgeglichen());
    drop(l); // SPERRE ist nicht wiedereintrittsfaehig: ein zweiter Lauf erst nach dem ersten

    let zwei = anfrage_antwort(&[(ID_A, "VaSt_LStB", "202501"), (ID_C, "VaSt_LStB", "202501")]);
    let Some(l) = Lauf::neu(&abruf_skript(&zwei, &abholung_antwort(PAKET))) else { return };
    assert_eq!(l.hole(), Err(AbrufFehler::MehrereBelege(2)));
    assert_eq!(l.zaehler("zaehler"), Some(1));
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
}

#[test]
fn eine_ablehnung_nennt_code_und_text_der_servermeldung_nie_den_datenteil() {
    // Anfrage abgelehnt (Transferheader).
    let skript = format!("610101210\n<Fehler/>\n--SERVER--\n{SERVER_TRANSFERHEADER}");
    let Some(l) = Lauf::neu(&skript) else { return };
    let fehler = l.hole().unwrap_err();
    let gesperrt = Servermeldung {
        code: "080015009".to_owned(),
        text: "Zertifikat vom Herausgeber gesperrt".to_owned(),
    };
    assert_eq!(
        fehler,
        AbrufFehler::Abgelehnt { schritt: "Anfrage", rc: 610_101_210, meldungen: vec![gesperrt] }
    );
    let text = fehler.to_string();
    assert!(text.contains("080015009") && text.contains("Zertifikat vom Herausgeber gesperrt"), "{text}");
    for m in MARKER.iter().chain([&IDNR, &PIN, &PAKET]) {
        assert!(!text.contains(m), "{m} im Fehlertext: {text}");
    }
    assert_eq!(l.zaehler("zaehler"), Some(1));
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
    assert_eq!(l.datei("zertifikat_geschlossen").unwrap(), "77\n");
    assert!(l.puffer_ausgeglichen());
    drop(l);

    // Abholung abgelehnt (Nutzdatenheader): die Anfrage ging durch.
    let skript = format!(
        "0\n<Erfolg/>\n--SERVER--\n{}\n=====\n610101292\n<Fehler/>\n--SERVER--\n{SERVER_NUTZDATENHEADER}",
        gute_anfrage()
    );
    let Some(l) = Lauf::neu(&skript) else { return };
    match l.hole() {
        Err(AbrufFehler::Abgelehnt { schritt: "Abholung", rc: 610_101_292, meldungen }) => {
            assert_eq!(meldungen.len(), 1, "{meldungen:?}");
            assert_eq!(meldungen.first().map(|m| m.code.as_str()), Some("67890"));
        }
        andere => panic!("erwartet Abgelehnt(Abholung), bekam {andere:?}"),
    }
    assert_eq!(l.zaehler("zaehler"), Some(2));
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
}

#[test]
fn eine_abholung_ohne_datenpaket_oder_ohne_klartext_ist_ein_benannter_fehler() {
    let Some(l) = Lauf::neu(&abruf_skript(&gute_anfrage(), &abholung_antwort(""))) else { return };
    assert_eq!(l.hole(), Err(AbrufFehler::AntwortUnlesbar("Datenpaket")));
    assert_eq!(l.zaehler("zaehler"), Some(2));
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
    drop(l);

    // ERiC entschluesselt, liefert aber nichts: kein Beleg.
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    assert_eq!(l.hole(), Err(AbrufFehler::AntwortUnlesbar("Klartext")));
    assert_eq!(l.zaehler("dekodiere_zaehler"), Some(1));
    drop(l);

    // ERiC kann nicht entschluesseln: der Code, kein Klartext.
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    std::fs::write(l.steuer("dekodiere_rc"), "610001862").unwrap();
    std::fs::write(l.steuer("dekodiert"), KLARTEXT).unwrap();
    assert_eq!(l.hole(), Err(AbrufFehler::Dekodieren { rc: 610_001_862 }));
    assert_eq!(l.datei("zertifikat_geschlossen").unwrap(), "77\n");
    assert!(l.puffer_ausgeglichen());
}

#[test]
fn die_sperren_des_abrufs_gehen_jedem_eric_aufruf_voraus() {
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    let faelle = [
        (Zertifikat::neu("", PIN), "Zertifikatspfad"),
        (Zertifikat::neu(l.zert_pfad(), ""), "PIN"),
    ];
    for (z, wort) in &faelle {
        match hole_beleg(Some(&l.lib), z, &auftrag(), &l.log_dir()) {
            Err(AbrufFehler::Zertifikat(text)) => assert!(text.contains(wort), "{text}"),
            andere => panic!("erwartet Zertifikat({wort}), bekam {andere:?}"),
        }
    }
    assert!(l.eric_unberuehrt(), "ERiC wurde trotz fehlendem Zertifikat geladen");
    // Ohne Bibliothek: erst nach den Sperren.
    assert_eq!(
        hole_beleg(None, &l.zertifikat(), &auftrag(), &l.log_dir()),
        Err(AbrufFehler::Eric(EricFehler::NichtGefunden))
    );
    // ERiC lehnt das Zertifikat ab: keine Anfrage.
    std::fs::write(l.steuer("zertifikat_rc"), "610201106").unwrap();
    assert!(matches!(l.hole(), Err(AbrufFehler::Zertifikat(t)) if t.contains("610201106")));
    assert_eq!(l.zaehler("zaehler"), None);
}

#[test]
fn der_beleg_liegt_in_einem_verzeichnis_0700_als_datei_0600() {
    let wurzel = tempfile::tempdir().unwrap();
    let pfad = lege_beleg_ab(wurzel.path(), KLARTEXT).unwrap();
    assert_eq!(std::fs::read(&pfad).unwrap(), KLARTEXT);
    let dir = pfad.parent().unwrap();
    assert_eq!(dir.parent().unwrap(), wurzel.path());
    assert!(dir.file_name().unwrap().to_string_lossy().starts_with("taxgraph_beleg_"), "{dir:?}");
    assert_eq!(pfad.file_name().unwrap(), "beleg.xml");
    assert_eq!(modus_von(dir), 0o700, "Verzeichnis {dir:?}");
    assert_eq!(modus_von(&pfad), 0o600, "Datei {pfad:?}");
    let zweiter = lege_beleg_ab(wurzel.path(), KLARTEXT).unwrap();
    assert_ne!(zweiter.parent(), pfad.parent(), "jeder Beleg bekommt ein eigenes Verzeichnis");
    let fehlt = wurzel.path().join("gibt-es-nicht");
    assert_eq!(lege_beleg_ab(&fehlt, KLARTEXT), Err(AbrufFehler::Ablage("Verzeichnis")));
}

#[test]
fn der_dry_run_laedt_eric_nie_und_zeigt_weder_steuer_id_noch_pin() {
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    let (rc, aus) = l.abruf_programm(&["--idnr", IDNR, "--jahr", "2025", "--dry-run"], "", false);
    assert_eq!(rc, 0, "{aus}");
    for zeile in ["DRY-RUN", "ElsterVaStDaten_32", "veranlagungsjahr: 2025", "steuer_id: 11 Ziffern"] {
        assert!(aus.contains(zeile), "{zeile} fehlt in:\n{aus}");
    }
    assert!(aus.contains("zertifikat_vorhanden: true"), "{aus}");
    assert!(l.eric_unberuehrt(), "der Dry-run hat ERiC beruehrt");
    assert_eq!(l.zaehler("dekodiere_zaehler"), None);
}

#[test]
fn das_programm_verweigert_den_echten_abruf_ohne_die_freigabe_option() {
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    let tippen = format!("{ABRUF_FREIGABE}\n");
    for args in [
        vec!["--idnr", IDNR, "--jahr", "2025", "--abruf"],
        vec!["--idnr", IDNR, "--jahr", "2025", "--abruf", "--freigabe", "ja"],
        vec!["--idnr", IDNR, "--jahr", "2025", "--abruf", "--freigabe", ECHTVERSAND_FREIGABE],
    ] {
        let (rc, aus) = l.abruf_programm(&args, &tippen, true);
        assert_eq!(rc, 2, "{args:?}: {aus}");
        assert!(aus.contains("ABRUF VERWEIGERT") && aus.contains(ABRUF_FREIGABE), "{aus}");
    }
    assert!(l.eric_unberuehrt());
}

#[test]
fn das_abruf_programm_verlangt_ein_terminal_und_die_zweite_eingabe() {
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    let args = ["--idnr", IDNR, "--jahr", "2025", "--abruf", "--freigabe", ABRUF_FREIGABE];
    let tippen = format!("{ABRUF_FREIGABE}\n");
    let (rc, aus) = l.abruf_programm(&args, &tippen, false);
    assert_eq!(rc, 2, "{aus}");
    assert!(aus.contains("kein Terminal"), "{aus}");
    for (eingabe, wort) in [("", "keine interaktive"), ("ja\n", "stimmte nicht ueberein")] {
        let (rc, aus) = l.abruf_programm(&args, eingabe, true);
        assert_eq!(rc, 2, "{eingabe:?}: {aus}");
        assert!(aus.contains("ABRUF VERWEIGERT") && aus.contains(wort), "{aus}");
    }
    assert!(l.eric_unberuehrt(), "ERiC wurde trotz verweigerter Huerde beruehrt");
}

#[test]
fn das_programm_ruft_nach_beiden_huerden_ab_und_legt_den_beleg_in_0700_0600_ab() {
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    std::fs::write(l.steuer("dekodiert"), KLARTEXT).unwrap();
    let ziel = l.wurzel.path().join("ziel");
    std::fs::create_dir_all(&ziel).unwrap();
    let (rc, aus) = l.abruf_echt(&ziel.to_string_lossy());
    assert_eq!(rc, 0, "{aus}");
    // Die Aufforderung `> ` endet ohne Zeilenumbruch (am Terminal setzt ihn die Eingabetaste); der Test tippt nicht.
    let pfad = aus.lines().find_map(|z| z.split_once("[abruf] Beleg abgelegt: ")).map(|(_, p)| p).unwrap_or_default();
    let pfad = PathBuf::from(pfad);
    assert!(pfad.starts_with(&ziel), "{pfad:?} liegt nicht unter {ziel:?}:\n{aus}");
    assert_eq!(std::fs::read(&pfad).unwrap(), KLARTEXT);
    assert_eq!(modus_von(pfad.parent().unwrap()), 0o700);
    assert_eq!(modus_von(&pfad), 0o600);
    assert!(aus.contains("beginnt_mit_xml_kopf: ja"), "{aus}");
    assert_eq!(l.zaehler("zaehler"), Some(2));
    // Nach einem Erfolg bleibt kein ERiC-Log liegen (es koennte die Anfrage nennen).
    let uebrig: Vec<_> = std::fs::read_dir(l.wurzel.path().join("temp")).unwrap().collect();
    assert_eq!(uebrig.len(), 0, "{uebrig:?}");
}

#[test]
fn das_programm_meldet_eine_ablehnung_mit_exit_eins_und_behaelt_das_log() {
    let skript = format!("610101210\n<Fehler/>\n--SERVER--\n{SERVER_TRANSFERHEADER}");
    let Some(l) = Lauf::neu(&skript) else { return };
    let ziel = l.wurzel.path().join("ziel");
    std::fs::create_dir_all(&ziel).unwrap();
    let (rc, aus) = l.abruf_echt(&ziel.to_string_lossy());
    assert_eq!(rc, 1, "{aus}");
    assert!(aus.contains("Code 080015009: Zertifikat vom Herausgeber gesperrt"), "{aus}");
    for m in MARKER {
        assert!(!aus.contains(m), "{m} in der Ausgabe:\n{aus}");
    }
    let log = aus.lines().find_map(|z| z.strip_prefix("[abruf] eric.log: ")).unwrap_or_default();
    assert!(Path::new(log).is_file(), "eric.log fehlt: {log:?}\n{aus}");
    // eric.log nennt Auszuege der Anfrage: sein Verzeichnis gehoert nur dem Besitzer.
    assert_eq!(modus_von(Path::new(log).parent().unwrap()), 0o700, "Log-Verzeichnis von {log:?}");
    assert_eq!(std::fs::read_dir(&ziel).unwrap().count(), 0, "bei einem Fehler wird kein Beleg abgelegt");
    drop(l);
    // Ein Fehler aus der Antwort (kein Beleg) ist ebenfalls Exit 1.
    let ohne = anfrage_antwort(&[(ID_A, "VaSt_RBM", "202501")]);
    let Some(l) = Lauf::neu(&abruf_skript(&ohne, &abholung_antwort(PAKET))) else { return };
    let ziel = l.wurzel.path().join("ziel");
    std::fs::create_dir_all(&ziel).unwrap();
    let (rc, aus) = l.abruf_echt(&ziel.to_string_lossy());
    assert_eq!(rc, 1, "{aus}");
    assert!(aus.contains("VaSt_RBM 202501"), "{aus}");
}

#[test]
fn das_programm_bricht_bei_ungueltiger_eingabe_vor_eric_ab() {
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    let tippen = format!("{ABRUF_FREIGABE}\n");
    for (args, feld) in [
        (vec!["--idnr", "1234567890", "--jahr", "2025", "--dry-run"], "idnr"),
        (vec!["--idnr", IDNR, "--jahr", "2021", "--dry-run"], "jahr"),
        (vec!["--idnr", "x", "--jahr", "2025", "--abruf", "--freigabe", ABRUF_FREIGABE], "idnr"),
    ] {
        let (rc, aus) = l.abruf_programm(&args, &tippen, true);
        assert_eq!(rc, 2, "{args:?}: {aus}");
        assert!(aus.contains(feld), "{feld} fehlt in:\n{aus}");
    }
    // Ein Ziel, das kein Verzeichnis ist, bricht VOR den Huerden und VOR ERiC ab: ein abgeholter Beleg ginge sonst verloren.
    let fehlt = l.wurzel.path().join("gibt-es-nicht").to_string_lossy().into_owned();
    let (rc, aus) = l.abruf_programm(
        &["--idnr", IDNR, "--jahr", "2025", "--ziel", &fehlt, "--abruf", "--freigabe", ABRUF_FREIGABE],
        &tippen,
        true,
    );
    assert_eq!(rc, 2, "{aus}");
    assert!(aus.contains("Ziel ist kein Verzeichnis") && !aus.contains("tippe"), "{aus}");
    // Ohne Hersteller-ID in der Umgebung.
    let (rc, aus) = l.abruf_programm_mit("", &["--idnr", IDNR, "--jahr", "2025", "--dry-run"], "", false);
    assert_eq!(rc, 2, "{aus}");
    assert!(aus.contains("ELSTER_HERSTELLER_ID"), "{aus}");
    // Nutzungsfehler: unbekannte Option, fehlende Pflichtangaben, zwei Modi.
    for args in [
        vec!["--idnr", IDNR, "--jahr", "2025", "--dry-run", "--unbekannt"],
        vec!["--jahr", "2025", "--dry-run"],
        vec!["--idnr", IDNR, "--dry-run"],
        vec!["--idnr", IDNR, "--jahr", "2025"],
        vec!["--idnr", IDNR, "--jahr", "2025", "--dry-run", "--abruf"],
    ] {
        let (rc, aus) = l.abruf_programm(&args, "", false);
        assert_eq!(rc, 2, "{args:?}: {aus}");
        assert!(aus.contains("Nutzung: taxgraph-abruf"), "{aus}");
    }
    assert!(l.eric_unberuehrt());
}

#[test]
fn die_abruf_umgebung_zeigt_weder_pin_noch_zertifikat_noch_hersteller_id_im_debug() {
    let env = AbrufUmgebung {
        zertifikat_pfad: ZERT_NAME.to_owned(),
        pin: PIN.to_owned(),
        hersteller_id: HERSTELLER.to_owned(),
        eric_lib: None,
        heute_jahr: HEUTE,
        temp: PathBuf::from("/nirgends"),
    };
    let text = format!("{env:?}");
    for geheim in [PIN, ZERT_NAME, HERSTELLER] {
        assert!(!text.contains(geheim), "{geheim:?} im Debug: {text}");
    }
}

#[test]
fn das_abruf_programm_als_prozess_liest_die_umgebung_und_erkennt_kein_terminal() {
    use chrono::Datelike as _;
    let Some(l) = Lauf::neu(&gutes_skript()) else { return };
    let vorjahr = (chrono::Utc::now().year() - 1).to_string();
    let lauf = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_taxgraph-abruf"))
            .args(args)
            .env_clear()
            .env("HOME", l.wurzel.path())
            .env("ERIC_DIR", l.lib.parent().unwrap())
            .env("ERIC_ATTRAPPE_DIR", l.wurzel.path().join("steuer"))
            .env("ELSTER_ZERTIFIKAT_PFAD", l.zert_pfad())
            .env("ELSTER_ZERTIFIKAT_PIN", PIN)
            .env("ELSTER_HERSTELLER_ID", HERSTELLER)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        for geheim in [PIN, ZERT_NAME, IDNR, HERSTELLER] {
            assert!(!text.contains(geheim), "{geheim:?} in der Ausgabe:\n{text}");
        }
        (out.status.code(), text)
    };
    let (code, aus) = lauf(&["--idnr", IDNR, "--jahr", &vorjahr, "--dry-run"]);
    assert_eq!(code, Some(0), "{aus}");
    assert!(aus.contains("DRY-RUN") && aus.contains("zertifikat_vorhanden: true"), "{aus}");
    let (code, aus) = lauf(&["--idnr", IDNR, "--jahr", &vorjahr, "--abruf", "--freigabe", ABRUF_FREIGABE]);
    assert_eq!(code, Some(2), "{aus}");
    assert!(aus.contains("kein Terminal"), "{aus}");
    assert!(l.eric_unberuehrt());
}
