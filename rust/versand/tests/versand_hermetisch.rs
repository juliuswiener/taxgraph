//! Der Versandpfad gegen die ATTRAPPE von `libericapi.so` — nie die echte Bibliothek, nie ein
//! Zertifikat, nie eine echte PIN, nie das Netz.
//!
//! Die Attrappe (`eric_attrappe.c`, dieselbe wie im Vergleichslauf; Orte siehe `ATTRAPPE_QUELLEN`) wird mit
//! `cc -shared -fPIC` gebaut und antwortet nach Steuerdateien in `$ERIC_ATTRAPPE_DIR`; sie schreibt
//! auf, was sie bekam: Flags, Crypto-Parameter (die PIN nur als Laenge), den Pfad des Zertifikats, die
//! Zahl der Aufrufe. Jeder Test liest daraus, WAS bei ERiC ankam — und bei jeder Sperre, dass NICHTS
//! ankam (`init_zaehler` fehlt: ERiC wurde nicht einmal geladen).
//!
//! `ERIC_ATTRAPPE_DIR` ist Prozess-Zustand; darum laufen alle Tests, die ERiC rufen, hintereinander
//! (`SPERRE`). Braucht `cc`. Fehlt es, scheitert der Test — ausser mit `TAXGRAPH_OHNE_CC=1`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

use elster::EricFehler;
use versand::{
    lauf, sende, Antwort, Modus, Umgebung, VersandFehler, Zertifikat, ECHTVERSAND_FREIGABE,
};

const XML_TEST: &[u8] = b"<?xml version=\"1.0\"?><Elster><TransferHeader>\
<Testmerker>700000004</Testmerker></TransferHeader></Elster>";
const XML_ECHT: &[u8] = b"<?xml version=\"1.0\"?><Elster><TransferHeader></TransferHeader></Elster>";
const ANTWORT_ERFOLG: &str =
    "<Elster><Erfolg><Telenummer>N552026081012345</Telenummer></Erfolg></Elster>";
const PIN: &str = "PIN-SENTINEL-4711";
const ZERT_NAME: &str = "zertifikat-SENTINEL-9f3a.pfx";

/// Wo die Attrappe liegt. Der Ort ausserhalb von `parity/` ist der kuenftige (die Loeschung von
/// `rust/parity` verschiebt die Datei dorthin); solange er fehlt, gilt der heutige. Der Rueckfall
/// wird mit dem Verschieben gegenstandslos und kann dann entfallen.
const ATTRAPPE_QUELLEN: [&str; 2] = [
    "../elster/tests/eric_attrappe/eric_attrappe.c",
    "../parity/tests/eric_attrappe/eric_attrappe.c",
];

static SPERRE: Mutex<()> = Mutex::new(());
static BIBLIOTHEK: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Die gebaute Attrappe (`libericapi.so`); `None` nur mit `TAXGRAPH_OHNE_CC=1` und ohne `cc`.
fn bibliothek() -> Option<&'static Path> {
    BIBLIOTHEK
        .get_or_init(|| {
            let lib = Path::new(env!("CARGO_TARGET_TMPDIR")).join("versand_attrappe");
            std::fs::create_dir_all(&lib).unwrap();
            let ziel = lib.join("libericapi.so");
            let quelle = ATTRAPPE_QUELLEN
                .iter()
                .map(|p| Path::new(env!("CARGO_MANIFEST_DIR")).join(p))
                .find(|p| p.is_file())
                .expect("die Quelle der Attrappe liegt an keinem der bekannten Orte");
            let gebaut = Command::new("cc")
                .args(["-shared", "-fPIC", "-O0", "-o"])
                .arg(&ziel)
                .arg(&quelle)
                .status();
            if gebaut.as_ref().is_ok_and(std::process::ExitStatus::success) {
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
        (rc, String::from_utf8(aus).unwrap())
    }

    /// Schreibt ein XML ins Wegwerf-Verzeichnis (je Inhalt eine eigene Datei) und gibt den Pfad.
    fn xml_datei(&self, xml: &[u8]) -> String {
        let p = self.wurzel.path().join(format!("fall_{}.xml", xml.len()));
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
