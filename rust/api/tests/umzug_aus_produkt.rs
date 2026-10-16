//! E2 bis E4 des Loeschplans (Vault `audits/python-loeschplan-2026-10-06`): Oberflaeche, Nutzerdatei und
//! das eingefrorene Register liegen in Rust-Pfaden; `produkt/` ist seit L3 weg.
//!
//! Was hier schiefgehen kann und wen es trifft:
//!
//! - Eine Datei der Oberflaeche oder des Registers aendert sich, ohne dass es auffaellt: der
//!   Dienst zeigt dem Nutzer eine andere Seite, die Orakel-Tests vergleichen gegen ein anderes Register.
//!   Die Tabellen unten halten den Stand als SHA-256 fest (aus `produkt/` am 2026-10-12 gemessen, `cmp` byte-gleich).
//!   Einen Vergleich mit `produkt/` gibt es seit L3 nicht mehr; die Hashes sind der Beleg.
//! - Die Nutzerdatei: der Standardpfad ist `users.json` neben `faelle/`. Ein Rueckgriff auf `produkt/auth/users.json`
//!   (bis L3) gilt nicht mehr: gaelte er wieder, wuerde eine alte Datei im Repo die Konten still ersetzen.
//!   Das Makefile (`backup`, `serve`) muss dieselbe Wahl treffen wie `Konfig::nutzerdatei`.
//! - Nichts hier liest die echte Nutzerdatei; die Pfade liegen in Wegwerf-Verzeichnissen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use api::konfig::Konfig;
use sha2::{Digest, Sha256};

/// `rust/api/static`: Name, SHA-256.
const OBERFLAECHE: &[(&str, &str)] = &[
    ("app.js", "3fbecbd62932c7094249cb4045c6789f4a4dc94c4fded9804907e2c033012d8d"),
    ("graph.css", "19db5b44291a8a6d22a93bc325b711b68f2adf181f0c63d091ac44f5fb71b60e"),
    ("graph.html", "234fc3db33fb9e036daee3c6a4a8cec959b0f3c4a5061419d7a02fba32929f5f"),
    ("graph.js", "a305763b18fc3bd907f6cf937279e2cf0587867d67c574518f85454c4c498bc9"),
    // index.html seit 2026-10-15 (S8, Julius) mit dem Hinweis auf die Namen-Grenze des PII-Filters neben dem KI-Eingabefeld:
    // weicht bewusst von `produkt/haut/static/index.html` ab (Stand 12c0ef31...); Wächter `http.rs::die_startseite_warnt_im_chat_vor_namen`.
    ("index.html", "9d923eed2beb2fe9ad7b59ba98aa8d60c8966ac66185ca62f10ed45fc3f8f856"),
    ("style.css", "f29ca8554f00ba97b396210c4391a497f07a3b3ba8d28848b67bcbd70d7ffd95"),
];

/// `rust/fixtures/python_bindung_2dd056a6`: der eingefrorene Stand von `produkt/bindung` (alle 28 Dateien).
const REGISTER: &[(&str, &str)] = &[
    ("bindung_an_gesamt.yaml", "c0ee815ee857eb2b993c0d70cd3073e62aeda3a2748aca9dff47a51963b95ce2"),
    ("bindung_kap_vv_familie.yaml", "eccf29925bbb1b8ff9f79c85f32482057f9d087839fe9a088296ce19894d20af"),
    ("bindung_n_vor_gwg.yaml", "0029e3ea18f1f4eec89ff9b208bd0f33760afd894a465363e52dda449738dc49"),
    ("bindung_p10_1_3_kind_kv_pv.yaml", "8c731629c17d51ed5450732bbdcd0f4530d82ea0ea8da42b88dfde66fcd7ddfd"),
    ("bindung_p10_1_5_gesamt.yaml", "4d7a088611fb0a6928dc174e5d0b0e14e57a4b34041b8684e242ea00a00dcfab"),
    ("bindung_p10_1_9_schulgeld_gesamt.yaml", "e7f7d811ee38dcbdeca4531dfd42305009d183dd07801f5104b1fa1c864c5ba1"),
    ("bindung_p10_1a_realsplitting_gesamt.yaml", "50a3e500caf4fdadacf89ec464dba92eca648919b7f79bc1486d4234dab3c9fb"),
    ("bindung_p101_mobilitaetspraemie.yaml", "85b729cc3a06e49f65e0bfbfb0aa4373a3e4bb8234766a0512a77e527e02d7d5"),
    ("bindung_p22_nr3.yaml", "fafdd5295ad5583f7f4a039abfc159b1c340c6191a2ca240813182c6cd3dd4e6"),
    ("bindung_p23_gesamt.yaml", "52445973f9ecbedf231f24d54d8b01bccd4f800acc147103cef19f921af96732"),
    ("bindung_p32b_gesamt.yaml", "277949acd146eda3503028b038663da1a11270b39d7ae3de78f24c7a987f9ece"),
    ("bindung_p33_2a_fahrtkostenpauschale.yaml", "9d5c2d458cce60b01a951dcaf5f1de515ee89e177237a02a846cd83f5f5dfb22"),
    ("bindung_p33a_gesamt.yaml", "521ed065723d11fedea413862d25ffa1d5d41504af51f1688d7e089f12fc5968"),
    ("bindung_p33b_abs5_kind.yaml", "901065acd9fa9410e53f57cb258b190c44ce81b57a054be030d2874b15910990"),
    ("bindung_p34c_gesamt.yaml", "01df7f019e51277cf34849ac247a26ed1147080444a99c365d7841b89b05d12c"),
    ("bindung_p35c_energieberater.yaml", "b27f9964ae0b8ce4cdcf897322d790e7327f35dded37fe924a619d3675a9e6ba"),
    ("bindung_p35c_sanierung.yaml", "d3af9532d69fdf1504263fd1106f3c3339a8732a5873e8b77e8fbfc9584ab8a4"),
    ("bindung_p36_abschlusszahlung.yaml", "81607a20eca9cd036c2abde5cf6ddba0ae84e21d3463c33176496f6bef23efcc"),
    ("bindung_p3_nr72_pv.yaml", "2e7c2cf831de9350a6403d8c41e7dd8fbcd80424b5d68573d3d0528022139df3"),
    ("bindung_p51a_kirchensteuer.yaml", "7c453857db2524c54a33bf90dfc3136bc4ae6ec9def3aca5961a9260fc5f41ef"),
    ("bindung_regel_bedingungen.yaml", "ca4f2844255a19538780ba5f19f0ca46cc37f4c24c88a84e889ad12d375b8a04"),
    ("bindung_rentner.yaml", "11df8c74a9be9f0e2aa2eca3c51fb8689d49d40eaec70c2375b7ea67d504d0d2"),
    ("bindung_screening_ausgaben.yaml", "1eae206fc676fd11b7f4e411b8071ce693100feacaf67ab5193368b8affc63f0"),
    ("bindung_screening_partner.yaml", "4bbc1efd619328ebb3b807cf0ccbd408612912a9c2663cfdee495b4bc68743c5"),
    ("bindung_sonder_agb_35a.yaml", "d76d83a2176714773bf4a3ae4689a3121452d04dde63a302146dafabfb8762f5"),
    ("FELD_BESTAND.yaml", "518029ec88d680771e12d46ba8316082edd3dac9502f17c24fad530502dc0021"),
    ("schema.json", "71e87770a33d3064391613183b3c943d428ed21ccf321e7d8cab80c9c3d58670"),
    ("SCHEMA.md", "b63c347fac3d84039f15c9530706c52afb108593f256282be7989feaf3c4e04a"),
];

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            write!(s, "{b:02x}").unwrap();
            s
        })
}

/// `dir` hat genau die Dateien der Tabelle (keine fehlt, keine ist dazugekommen) und jede traegt ihren Hash.
fn pruefe_stand(dir: &Path, soll: &[(&str, &str)]) {
    let mut gefunden: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    gefunden.sort();
    let mut erwartet: Vec<String> = soll.iter().map(|(n, _)| (*n).to_string()).collect();
    erwartet.sort();
    assert_eq!(gefunden, erwartet, "Dateien in {}", dir.display());
    for (name, hash) in soll {
        let bytes = std::fs::read(dir.join(name)).unwrap();
        assert_eq!(sha256_hex(&bytes), *hash, "{name} in {}", dir.display());
    }
}

#[test]
fn die_oberflaeche_liegt_byte_gleich_in_rust_api_static() {
    pruefe_stand(&wurzel().join("rust/api/static"), OBERFLAECHE);
}

#[test]
fn das_eingefrorene_register_liegt_byte_gleich_in_rust_fixtures() {
    pruefe_stand(
        &wurzel().join("rust/fixtures/python_bindung_2dd056a6"),
        REGISTER,
    );
}

#[test]
fn der_dienst_liefert_die_oberflaeche_aus_rust_api_static() {
    let konfig = Konfig {
        wurzel: wurzel(),
        faelle: PathBuf::from("/nie/gelesen/faelle"),
        audit_dir: PathBuf::from("/nie/gelesen/faelle"),
    };
    assert_eq!(konfig.static_dir(), wurzel().join("rust/api/static"));
    assert!(konfig.static_dir().join("index.html").is_file());
}

/// Eine Wurzel (mit oder ohne Datei an der alten Stelle `produkt/auth/users.json`) und ein Datenverzeichnis (mit oder ohne
/// `users.json`).
struct Umgebung {
    tmp: tempfile::TempDir,
}

impl Umgebung {
    fn neu(alt: bool, neu: bool) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let u = Self { tmp };
        std::fs::create_dir_all(u.daten().join("faelle")).unwrap();
        if alt {
            std::fs::create_dir_all(u.wurzel().join("produkt/auth")).unwrap();
            std::fs::write(u.alt(), "{}").unwrap();
        }
        if neu {
            std::fs::write(u.neu_pfad(), "{}").unwrap();
        }
        u
    }

    fn wurzel(&self) -> PathBuf {
        self.tmp.path().join("wurzel")
    }

    fn daten(&self) -> PathBuf {
        self.tmp.path().join("daten")
    }

    fn alt(&self) -> PathBuf {
        self.wurzel().join("produkt/auth/users.json")
    }

    fn neu_pfad(&self) -> PathBuf {
        self.daten().join("users.json")
    }

    fn konfig(&self) -> Konfig {
        Konfig {
            wurzel: self.wurzel(),
            faelle: self.daten().join("faelle"),
            audit_dir: self.daten().join("faelle"),
        }
    }
}

#[test]
fn ohne_datei_gilt_users_json_neben_faelle() {
    let u = Umgebung::neu(false, false);
    assert_eq!(u.konfig().nutzerdatei(), u.neu_pfad());
}

#[test]
fn eine_datei_an_der_alten_stelle_zaehlt_nicht_mehr() {
    // Nur die alte Datei da: kein Rueckgriff, der Pfad bleibt `users.json` neben `faelle/` (auch wenn es sie dort nicht gibt).
    let u = Umgebung::neu(true, false);
    assert!(u.alt().is_file());
    assert_eq!(u.konfig().nutzerdatei(), u.neu_pfad());
    // Beide da: ebenfalls die neue.
    let u = Umgebung::neu(true, true);
    assert_eq!(u.konfig().nutzerdatei(), u.neu_pfad());
}

/// `make -n backup` im Wegwerf-Verzeichnis `cwd`: der `tar`-Befehl, in dem `-C <dir> <datei>` die Nutzerdatei nennt.
fn makefile_tar_zeile(u: &Umgebung, alt_im_cwd: bool) -> String {
    let cwd = u.tmp.path().join("cwd");
    if alt_im_cwd {
        std::fs::create_dir_all(cwd.join("produkt/auth")).unwrap();
        std::fs::write(cwd.join("produkt/auth/users.json"), "{}").unwrap();
    } else {
        std::fs::create_dir_all(&cwd).unwrap();
    }
    let aus = Command::new("make")
        .args(["-n", "backup"])
        .arg("-C")
        .arg(&cwd)
        .arg("-f")
        .arg(wurzel().join("Makefile").canonicalize().unwrap())
        .arg(format!("FAELLE_ROOT={}", u.daten().display()))
        .arg(format!("BACKUP_DIR={}", u.tmp.path().join("sicherung").display()))
        .output()
        .unwrap();
    assert!(aus.status.success(), "{}", String::from_utf8_lossy(&aus.stderr));
    let text = String::from_utf8(aus.stdout).unwrap();
    // Der Befehl geht ueber drei Zeilen (`\`-Fortsetzung); ab `tar czf` bis zum Ende.
    let ab = text.find("tar czf").unwrap_or_else(|| panic!("kein tar: {text}"));
    text[ab..].replace("\\\n", " ")
}

#[test]
fn das_makefile_waehlt_die_nutzerdatei_wie_konfig() {
    let daten = |u: &Umgebung| u.daten().display().to_string();
    // users.json im Datenverzeichnis da, alte Stelle im Arbeitsverzeichnis auch: die im Datenverzeichnis.
    let u = Umgebung::neu(false, true);
    let z = makefile_tar_zeile(&u, true);
    assert!(z.contains(&format!("-C {}/ users.json", daten(&u))), "{z}");
    // nur die alte Stelle da: kein Rueckgriff, die Datei im Datenverzeichnis (die es nicht gibt, `backup` bricht dann ab).
    let u = Umgebung::neu(false, false);
    let z = makefile_tar_zeile(&u, true);
    assert!(z.contains(&format!("-C {}/ users.json", daten(&u))), "{z}");
    assert!(!z.contains("produkt/auth"), "{z}");
    // keine: dieselbe.
    let u = Umgebung::neu(false, false);
    let z = makefile_tar_zeile(&u, false);
    assert!(z.contains(&format!("-C {}/ users.json", daten(&u))), "{z}");
}
