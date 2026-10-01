//! Der Fall-Korpus: finden, lesen, **zaehlen**.
//!
//! # Warum es dieses Modul gibt
//!
//! Drei Parity-Suiten lasen den Korpus mit demselben wortgleichen Block und uebersprangen
//! eine unlesbare Datei mit `continue` -- **ohne sie zu zaehlen**:
//!
//! ```text
//! let Ok(text) = std::fs::read_to_string(pfad) else { continue; };
//! let Ok(roh)  = serde_json::from_str::<Value>(&text) else { continue; };
//! ```
//!
//! Die Wache daneben prueft die Zahl der **gefundenen** Dateien. Gemessen am 2026-10-01:
//! 44 echte Dateien und 148 unlesbare `.json` ergaben einen GRUENEN Lauf, dessen Schlusszeile
//! `reale_faelle: 192 Dateien, 0 ohne Store übersprungen` meldete -- der Lauf behauptete den
//! vollen Korpus und hatte 77 % davon nie gelesen.
//!
//! Dieselbe Bauart wie der `nicht_leer`-Waechter, den [`crate::pin`] abfedert: gruen und leer
//! sehen identisch aus. Nur liegt sie eine Schicht tiefer -- der Pin kann nur so ehrlich sein
//! wie der Korpus, den der Leser ihm reicht.
//!
//! # Was hier gilt
//!
//! [`Korpus::lies`] zaehlt drei Zahlen: gefunden, gelesen, uebersprungen. [`Korpus::pflicht`]
//! prueft die **gelesene** Zahl. Der stille `continue` wird zur gezaehlten Zeile.

use serde_json::Value;
use std::path::{Path, PathBuf};

/// Die Mindestzahl gelesener Dateien, unter der ein Lauf nichts belegt.
///
/// **Ein Lauf unter dieser Zahl ist kein gruener Lauf.** Er vergleicht zu wenig, um die
/// Abwesenheit von Abweichungen zu belegen -- genau die Luecke, die dieses Modul schliesst.
///
/// Die Zahl ist gemessen, nicht geraten. Der bekannte Korpus traegt **192** Dateien
/// (`korpus-rt`), der Live-Korpus **207**; beide sind vollstaendig lesbar (192 von 192,
/// 207 von 207 -- Stand 2026-10-01). Die Schwelle liegt bei der Haelfte des bekannten
/// Korpus: hoch genug, dass ein Lauf auf einem Bruchteil rot wird, niedrig genug fuer eine
/// gezielte Untersuchung mit einer Handvoll Faelle.
///
/// **Warum nicht tiefer.** Gemessen am 2026-10-01: 44 echte Dateien und 148 unlesbare
/// `.json` ergaben einen GRUENEN Lauf, der den vollen Korpus behauptete. Eine Schwelle von
/// 40 haette genau diesen Fall durchgelassen -- sie stand hier zuerst und war falsch.
/// `korpus::tests::die_schwelle_laesst_den_gemessenen_bruch_nicht_durch` haelt das fest.
///
/// `ponytail:` eine feste Zahl, keine Quote. Eine Quote (etwa "mindestens 90 % der
/// gefundenen") waere gegen einen Korpus aus lauter unlesbaren Dateien wirkungslos: 0 von 0
/// erreicht keine 90 %, und der Nenner ist selbst schon unbrauchbar. Eine feste Zahl ist
/// ehrlicher -- sie sagt, wie viele Faelle ein Lauf mindestens gesehen haben muss, damit
/// "0 Abweichungen" etwas bedeutet. Ausbau, falls der Korpus waechst: die Zahl an den
/// Korpus binden statt an den Code.
pub const MINDESTENS_GELESEN: usize = 100;

/// Der Korpus, wie er auf der Platte liegt: gefundene Dateien, gelesene Werte, Schwund.
///
/// `gefunden` ist die Zahl der Dateien mit passender Endung, `gelesen` die der erfolgreich
/// geparsten, `uebersprungen` die Differenz mit Grund. Eine Datei, die geparst wurde, aber
/// kein `events` traegt, gilt als **gelesen** -- sie ist kein Schwund, sondern ein gueltiger
/// Fall ohne Store; die Suiten zaehlen das getrennt als `kein_store`.
pub struct Korpus {
    /// Dateien mit passender Endung, in Sortierreihenfolge.
    pub gefunden: Vec<PathBuf>,
    /// Erfolgreich gelesene und geparste Dateien: `(Pfad, Wert)`.
    pub gelesen: Vec<(PathBuf, Value)>,
    /// Uebersprungene Dateien mit Grund: `(Pfad, Klartext)`.
    pub uebersprungen: Vec<(PathBuf, String)>,
}

impl Korpus {
    /// Liest alle `.json` unter `dir` -- rekursiv, sortiert, mit Zaehlung.
    ///
    /// Anders als die drei abgeloesten Fassungen verschluckt diese nichts: jede unlesbare
    /// Datei landet mit Grund in [`Korpus::uebersprungen`].
    #[must_use]
    pub fn lies(dir: &Path) -> Self {
        let gefunden = finde_json(dir);
        let mut gelesen = Vec::new();
        let mut uebersprungen = Vec::new();
        for pfad in &gefunden {
            match std::fs::read_to_string(pfad) {
                Err(e) => uebersprungen.push((pfad.clone(), format!("nicht lesbar: {e}"))),
                Ok(text) => match serde_json::from_str::<Value>(&text) {
                    Ok(roh) => gelesen.push((pfad.clone(), roh)),
                    Err(e) => uebersprungen.push((pfad.clone(), format!("kein JSON: {e}"))),
                },
            }
        }
        Self {
            gefunden,
            gelesen,
            uebersprungen,
        }
    }

    /// Die Zahl der erfolgreich gelesenen Dateien. **Diese** Zahl belegt einen Lauf.
    #[must_use]
    pub fn gelesen_zahl(&self) -> usize {
        self.gelesen.len()
    }

    /// Bricht ab, wenn zu wenig gelesen wurde -- VOR jeder Auswertung.
    ///
    /// Muss **vor** `wache_rechnet` laufen. Bei einem unlesbaren Korpus meldet der Pin sonst
    /// "gelistete Zeile(n) gibt es nicht mehr -- Liste aktualisieren", und das ist eine
    /// Fehldiagnose: nicht die Liste ist veraltet, der Korpus ist es. Wer der Meldung folgt,
    /// streicht korrekte Eintraege.
    ///
    /// # Panics
    ///
    /// Panikt, wenn [`Korpus::gelesen_zahl`] unter [`MINDESTENS_GELESEN`] liegt. Die Meldung
    /// nennt beide Zahlen und die Gruende der uebersprungenen Dateien.
    pub fn pflicht(&self, verzeichnis: &Path, block: &str) {
        let mut meldung = String::new();
        if self.gelesen_zahl() < MINDESTENS_GELESEN {
            use std::fmt::Write as _;
            let _ = write!(
                meldung,
                "\n{block}: nur {} von {} Dateien unter {} GELESEN (mindestens {MINDESTENS_GELESEN} \
                 noetig) -- ein Parity-Lauf auf diesem Bruchteil belegt nichts. Er vergleicht zu \
                 wenig, um '0 Abweichungen' zu bedeuten.",
                self.gelesen_zahl(),
                self.gefunden.len(),
                verzeichnis.display()
            );
            if self.gelesen_zahl() == 0 {
                meldung.push_str(
                    "\n  KEINE Datei gelesen. Die 'gelistete Zeile(n) gibt es nicht mehr'-Meldung \
                     des Pins waere hier eine FEHLDIAGNOSE: nicht die Liste ist veraltet, der \
                     Korpus ist unlesbar.",
                );
            }
            for (pfad, grund) in self.uebersprungen.iter().take(5) {
                let _ = write!(meldung, "\n  übersprungen: {} ({grund})", pfad.display());
            }
            if self.uebersprungen.len() > 5 {
                let _ = write!(
                    meldung,
                    "\n  ... und {} weitere übersprungen",
                    self.uebersprungen.len() - 5
                );
            }
        }
        assert!(meldung.is_empty(), "{meldung}");
    }
}

/// Alle Dateien mit Endung `.json` unter `dir`, rekursiv, sortiert.
///
/// Ersetzt die Kopien von `walk_json` in den drei umgestellten Suiten. Eine unlesbare Datei
/// ist hier **kein** Fehler: [`Korpus::lies`] zaehlt sie als uebersprungen.
#[must_use]
pub fn finde_json(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in read.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(finde_json(&p));
        } else if p.extension().is_some_and(|x| x == "json") {
            out.push(p);
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Baut einen Korpus aus `gelesen` gueltigen und `unlesbar` kaputten `.json`-Dateien.
    fn korpus_aus(gelesen: usize, unlesbar: usize) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("Tempverzeichnis");
        let faelle = dir.path().join("faelle");
        std::fs::create_dir_all(&faelle).expect("faelle anlegen");
        for i in 0..gelesen {
            std::fs::write(
                faelle.join(format!("gut_{i:04}.json")),
                br#"{"events": [], "veranlagungszeitraum": 2025}"#,
            )
            .expect("schreiben");
        }
        for i in 0..unlesbar {
            std::fs::write(faelle.join(format!("kaputt_{i:04}.json")), b"/* kein JSON */\n")
                .expect("schreiben");
        }
        (dir, faelle)
    }

    /// Der Kern: eine unlesbare Datei wird GEZAEHLT, nicht verschluckt.
    #[test]
    fn eine_unlesbare_datei_wird_gezaehlt_statt_verschluckt() {
        let (_d, faelle) = korpus_aus(3, 2);
        let k = Korpus::lies(&faelle);
        assert_eq!(k.gefunden.len(), 5, "gefundene Dateien");
        assert_eq!(k.gelesen_zahl(), 3, "gelesene Dateien");
        assert_eq!(k.uebersprungen.len(), 2, "uebersprungene Dateien");
        assert!(
            k.uebersprungen.iter().all(|(_, g)| g.contains("kein JSON")),
            "jede uebersprungene Datei nennt ihren Grund: {:?}",
            k.uebersprungen
        );
    }

    /// Ein vollstaendig lesbarer Korpus geht durch -- die Schwelle ist kein Selbstzweck.
    #[test]
    fn ein_voller_korpus_geht_durch() {
        let (_d, faelle) = korpus_aus(MINDESTENS_GELESEN, 0);
        let k = Korpus::lies(&faelle);
        k.pflicht(&faelle, "test"); // panikt nicht
        assert_eq!(k.uebersprungen.len(), 0);
    }

    /// **Der gemessene Bruch.** Am 2026-10-01 war ein Lauf auf 44 von 192 Dateien gruen und
    /// behauptete den vollen Korpus. Diese Zahl muss rot werden -- sonst ist die Schwelle
    /// selbst die Luecke, die sie schliessen soll.
    ///
    /// Geprueft wird an einem Korpus mit genau dieser Zahl, nicht an der Konstanten: eine
    /// Pruefung `44 < MINDESTENS_GELESEN` waere zur Uebersetzungszeit konstant und wuerde
    /// nichts messen (clippy meldet das zu Recht).
    #[test]
    fn die_schwelle_laesst_den_gemessenen_bruch_nicht_durch() {
        let (_d, faelle) = korpus_aus(44, 148);
        let k = Korpus::lies(&faelle);
        assert_eq!(k.gelesen_zahl(), 44);
        assert_eq!(k.gefunden.len(), 192, "der gemessene Korpus hatte 192 Dateien");
        let panik = std::panic::catch_unwind(|| k.pflicht(&faelle, "reale_faelle"));
        assert!(
            panik.is_err(),
            "44 von 192 gelesenen Dateien waren am 2026-10-01 GRUEN und behaupteten den vollen \
             Korpus -- die Schwelle {MINDESTENS_GELESEN} laesst genau das durch"
        );
    }

    /// Die Diagnose: ein unlesbarer Korpus nennt sich so und nicht "Liste aktualisieren".
    #[test]
    fn ein_unlesbarer_korpus_bricht_vor_dem_pin_ab() {
        let (_d, faelle) = korpus_aus(0, 192);
        let k = Korpus::lies(&faelle);
        let panik = std::panic::catch_unwind(|| k.pflicht(&faelle, "reale_faelle"))
            .expect_err("0 gelesene Dateien muessen abbrechen");
        let text = panik
            .downcast_ref::<String>()
            .map_or_else(String::new, Clone::clone);
        assert!(
            text.contains("0 von 192") && text.contains("FEHLDIAGNOSE"),
            "die Meldung muss die Zahl nennen und die Pin-Fehldiagnose benennen: {text}"
        );
    }
}
