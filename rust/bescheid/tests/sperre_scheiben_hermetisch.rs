//! Scheibe `rentner_gesamt` (und vier Gegenproben auf `gesamt`): die Sperren des Gesamt-Guards `an_gesamt_sperrgrund` im
//! Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! Der Guard verwandelt eine fehlende oder widerspruechliche Angabe in einen sichtbaren Grund (`rente_instanz_offen`,
//! `rentenfreibetrag_fixierung_offen`, `kinderbetreuung_zahlung_offen`, `dhf_tatbestand_offen`, ...), statt dass der
//! Bescheid still zu wenig abzieht oder zu viel anrechnet. Auf der Scheibe `rentner_gesamt` laeuft er mit eigener `Cfg`
//! (`rentner`, ohne `partner_19`, Fremd-Art `kein_vuv`). Was in `deklaration/sperre/*.rs` nicht schon die Tests aus
//! h8-hermetisch2/3 (`rentenbeginn_jahr.rs`, `gewst_hebesatz_hermetisch.rs`, `p35a_p35c_gates_hermetisch.rs`) bewachen,
//! pruefte bisher nur Parity (`rust/parity/tests/bescheid_deklaration_paritaet.rs`, nur mit `PARITY=1`; die CI faehrt
//! Parity nicht).
//!
//! Gemessen am 2026-10-03 auf 4a2f6ea4 (Bericht h8-hermetisch4): 13 Mutationen am Aufrufort in
//! `deklaration/sperre/{gesamt,einkunft,werbungskosten}.rs`. 11 lassen `cargo test -p bescheid` gruen (188 passed, 0 failed,
//! 10 ignored): R2 Rente Person B gilt schon mit einem Kernfeld als vollstaendig, R3 die KV/PV-Weiche der Person B ignoriert
//! `basis_pv_partner`, R4 weitere Rente-Instanzen erst ab Index 3, R5 Versorgungsfreibetrag verlangt nur eines von Beginnjahr
//! und Bemessungsgrundlage, R6 Schwelle `GdB` 20 -> 21, R7 kein Zahlungsnachweis bei der Kinderbetreuung, R8 Fremd-Art
//! `.any` -> `.all`, R10 die Unterhalt-Sperre entfaellt, R11 Ausland-dHf sperrt nicht, R12 Person B zaehlt bei § 16 Abs. 4
//! auch ohne Zusammenveranlagung, R13 Dreimonatsfrist der Verpflegung erst ab dem fuenften Monat. Zwei (R1 `rente()` laeuft nie,
//! R9 `partner_19` auch auf der Rentner-Scheibe) fangen schon die Bestandstests `jahr_null_und_minus_eins_sperren` und
//! `jahr_eins_sperrt_nicht_wegen_des_jahres` (R9 dazu `rentner_brauchbarer_hebesatz_messbetrag_null_und_einzelveranlagung_
//! sperren_nicht`). Mit diesen Tests werden alle 13 rot, jede mit dem Test, der ihre Stelle prueft (Tabelle im Bericht).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zeile der Tabelle unten ist die Ausgabe des Python-Orakels
//! `bescheid_deklaration._an_gesamt_sperrgrund` (ueber `tools/parity/bescheid_oracle`, Wegwerf-Datenwurzel) mit der Scheibe,
//! dem Store UND der Scheiben-Bindung, `nur_bestaetigt = false`, VZ 2025, auf denselben Events (Orakel-Skript und Lauf:
//! Anlagen zum Bericht). Kein Wert ist aus dem Rust-Code abgelesen. Eine Rechnung aus den Quellen gibt es fuer eine Sperre
//! nicht: DASS eine unbeantwortete Frage sperrt, ist Entwurf des Projekts, nicht Gesetzeswortlaut; dort stuetzt nur das
//! Python-Orakel. Aus dem Gesetz stammen nur die Tatbestaende, an denen einzelne Sperren haengen (§ 33b Abs. 2 `EStG`: Pauschbetrag
//! ab `GdB` 20, `estg_p33b_2026-07-13.txt`; § 10 Abs. 1 Nr. 5 S. 4 `EStG`: Rechnung und unbare Zahlung, `estg_p10_2026-07-11.txt`).
//!
//! 37 der 101 Faelle liefern keine Sperre (Kontrollfaelle: alles beantwortet, Betrag 0, Flag bejaht): ohne sie bestuende auch ein
//! Guard, der immer einen Grund liefert, jeden Fall. Die vier Gegenproben auf `gesamt` zeigen, dass die Rentner-Zweige dort
//! nicht laufen (`rente()`, Person-B-Kegel nur dort, Fremd-Art `kein_sonstige` statt `kein_vuv`).
//!
//! Abgrenzung (k9, Auftrag flag-negiert-alle): diese Datei mutiert weder `konsistenz/flag.rs` (`FLAG_NEGIERT`) noch die
//! Scheibenkonfiguration (`Cfg`-Schalter, `fremd_arten`) in `deklaration.rs`; sie bewacht nur, wie `sperre/*.rs` die Konfiguration
//! LIEST (R1, R8, R9). Ein unbeantwortetes `kein_*`-Flag gilt als "ja"; steht daneben ein Betrag der Art, loest das
//! `flag_konsistenz_offen` aus. Die Events der betroffenen Faelle (Rente, Gewinn, Kapital) beantworten das Flag darum
//! ausdruecklich mit `false` (`kein_sonstige`, `kein_gewinn`, `kein_kap`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::sync::OnceLock;

use bescheid::deklaration::{an_gesamt_sperrgrund, Cfg};
use bescheid::testhilfe::{felder, index, store};
use bescheid::{BindungIndex, Instanzquelle};
use domain::{Scheibe, Sperrgrund, Vz};
use serde_json::{json, Value};

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

/// Ein Ja/Nein-Feld; `bestaetigt = false` ist ein vorlaeufiger Wert.
fn b(fid: &'static str, wert: bool, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein Zahlfeld (Cent, Jahr oder Grad); `bestaetigt = false` ist ein vorlaeufiger Wert.
fn z(fid: &'static str, wert: i64, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein Textfeld.
fn t(fid: &'static str, wert: &'static str, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein handgebauter Fall: Scheibe, Events und der Grund, den das Python-Orakel liefert.
struct Fall {
    scheibe: Scheibe,
    gruppe: &'static str,
    name: &'static str,
    events: Vec<Ev>,
    erwartet: Option<&'static str>,
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn faelle() -> Vec<Fall> {
    vec![
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente",
            name: "Rente A vollstaendig, Beginn 2015, Freibetrag fixiert",
            events: vec![
                b("kein_sonstige", false, true),
                t("rentner_renten_art", "gesetzliche_rente", true),
                z("rentner_jahresrente", 1_200_000, true),
                z("rentner_renten_beginn_jahr", 2015, true),
                z("rentner_alter_bei_rentenbeginn", 65, true),
                z("rentner_rentenfreibetrag", 4000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente",
            name: "gesetzliche Rente vor dem VZ ohne Rentenfreibetrag",
            events: vec![
                b("kein_sonstige", false, true),
                t("rentner_renten_art", "gesetzliche_rente", true),
                z("rentner_jahresrente", 1_200_000, true),
                z("rentner_renten_beginn_jahr", 2015, true),
                z("rentner_alter_bei_rentenbeginn", 65, true),
            ],
            erwartet: Some("rentenfreibetrag_fixierung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente",
            name: "Rentenbeginn nach dem VZ (2026)",
            events: vec![
                t("rentner_renten_art", "gesetzliche_rente", true),
                z("rentner_renten_beginn_jahr", 2026, true),
            ],
            erwartet: Some("rentenbeginn_nach_vz"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente",
            name: "Rentenbeginn 0",
            events: vec![
                t("rentner_renten_art", "gesetzliche_rente", true),
                z("rentner_renten_beginn_jahr", 0, true),
            ],
            erwartet: Some("rentenbeginn_jahr_ungueltig"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente",
            name: "Rente 2 unvollstaendig (nur Art)",
            events: vec![
                b("kein_sonstige", false, true),
                t("rentner_renten_art", "gesetzliche_rente", true),
                z("rentner_jahresrente", 1_200_000, true),
                z("rentner_renten_beginn_jahr", 2015, true),
                z("rentner_alter_bei_rentenbeginn", 65, true),
                z("rentner_rentenfreibetrag", 4000, true),
                t("rentner_renten_art__2", "gesetzliche_rente", true),
            ],
            erwartet: Some("rente_instanz_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente",
            name: "Rente 2 vollstaendig und bestaetigt",
            events: vec![
                b("kein_sonstige", false, true),
                t("rentner_renten_art", "gesetzliche_rente", true),
                z("rentner_jahresrente", 1_200_000, true),
                z("rentner_renten_beginn_jahr", 2015, true),
                z("rentner_alter_bei_rentenbeginn", 65, true),
                z("rentner_rentenfreibetrag", 4000, true),
                t("rentner_renten_art__2", "gesetzliche_rente", true),
                z("rentner_jahresrente__2", 600_000, true),
                z("rentner_renten_beginn_jahr__2", 2016, true),
                z("rentner_alter_bei_rentenbeginn__2", 64, true),
                z("rentner_rentenfreibetrag__2", 3000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente",
            name: "Rente 2 vollstaendig, Art nur vorlaeufig",
            events: vec![
                b("kein_sonstige", false, true),
                t("rentner_renten_art", "gesetzliche_rente", true),
                z("rentner_jahresrente", 1_200_000, true),
                z("rentner_renten_beginn_jahr", 2015, true),
                z("rentner_alter_bei_rentenbeginn", 65, true),
                z("rentner_rentenfreibetrag", 4000, true),
                t("rentner_renten_art__2", "gesetzliche_rente", false),
                z("rentner_jahresrente__2", 600_000, true),
                z("rentner_renten_beginn_jahr__2", 2016, true),
                z("rentner_alter_bei_rentenbeginn__2", 64, true),
                z("rentner_rentenfreibetrag__2", 3000, true),
            ],
            erwartet: Some("rente_instanz_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente",
            name: "Rente 2 vollstaendig, Beginn nach dem VZ",
            events: vec![
                b("kein_sonstige", false, true),
                t("rentner_renten_art", "gesetzliche_rente", true),
                z("rentner_jahresrente", 1_200_000, true),
                z("rentner_renten_beginn_jahr", 2015, true),
                z("rentner_alter_bei_rentenbeginn", 65, true),
                z("rentner_rentenfreibetrag", 4000, true),
                t("rentner_renten_art__2", "gesetzliche_rente", true),
                z("rentner_jahresrente__2", 600_000, true),
                z("rentner_renten_beginn_jahr__2", 2026, true),
                z("rentner_alter_bei_rentenbeginn__2", 64, true),
                z("rentner_rentenfreibetrag__2", 3000, true),
            ],
            erwartet: Some("rentenbeginn_nach_vz"),
        },
        Fall {
            scheibe: Scheibe::Gesamt,
            gruppe: "rente",
            name: "Gegenprobe gesamt: Rentenbeginn nach dem VZ sperrt dort nicht",
            events: vec![
                t("rentner_renten_art", "gesetzliche_rente", true),
                z("rentner_renten_beginn_jahr", 2026, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "zusammen, Rente B: 1 von 4 Kernfeldern",
            events: vec![
                t("veranlagung", "zusammen", true),
                t("rentner_renten_art_partner", "gesetzliche_rente", true),
            ],
            erwartet: Some("rente_instanz_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "zusammen, Rente B: 3 von 4 Kernfeldern",
            events: vec![
                t("veranlagung", "zusammen", true),
                t("rentner_renten_art_partner", "gesetzliche_rente", true),
                z("rentner_jahresrente_partner", 900_000, true),
                z("rentner_renten_beginn_jahr_partner", 2018, true),
            ],
            erwartet: Some("rente_instanz_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "zusammen, Rente B: alle 4 Kernfelder, Freibetrag fixiert",
            events: vec![
                t("veranlagung", "zusammen", true),
                t("rentner_renten_art_partner", "gesetzliche_rente", true),
                z("rentner_jahresrente_partner", 900_000, true),
                z("rentner_renten_beginn_jahr_partner", 2018, true),
                z("rentner_alter_bei_rentenbeginn_partner", 66, true),
                z("rentner_rentenfreibetrag_partner", 3500, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "zusammen, Rente B: alle 4 Kernfelder, ohne Freibetrag (aa)",
            events: vec![
                t("veranlagung", "zusammen", true),
                t("rentner_renten_art_partner", "gesetzliche_rente", true),
                z("rentner_jahresrente_partner", 900_000, true),
                z("rentner_renten_beginn_jahr_partner", 2018, true),
                z("rentner_alter_bei_rentenbeginn_partner", 66, true),
            ],
            erwartet: Some("rentenfreibetrag_fixierung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "zusammen, keine Rente B",
            events: vec![t("veranlagung", "zusammen", true)],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "einzel, Rente B unvollstaendig: Person B zaehlt nicht",
            events: vec![
                t("veranlagung", "einzel", true),
                t("rentner_renten_art_partner", "gesetzliche_rente", true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "zusammen, Partner-KV ohne Versicherungsart",
            events: vec![
                t("veranlagung", "zusammen", true),
                z("basis_kv_partner", 300_000, true),
            ],
            erwartet: Some("partner_kegel_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "zusammen, Partner-PV ohne Versicherungsart",
            events: vec![
                t("veranlagung", "zusammen", true),
                z("basis_pv_partner", 80_000, true),
            ],
            erwartet: Some("partner_kegel_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "zusammen, Partner-KV mit Versicherungsart",
            events: vec![
                t("veranlagung", "zusammen", true),
                z("basis_kv_partner", 300_000, true),
                t("versicherungsart_partner", "gesetzlich", true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "rente_b",
            name: "zusammen, Partner-KV mit Versicherungsart nur vorlaeufig",
            events: vec![
                t("veranlagung", "zusammen", true),
                z("basis_kv_partner", 300_000, true),
                t("versicherungsart_partner", "gesetzlich", false),
            ],
            erwartet: Some("partner_kegel_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "versorgung",
            name: "Versorgungsbezug ohne Beginnjahr und Bemessungsgrundlage",
            events: vec![z("versorgung_jahresrente", 1_800_000, true)],
            erwartet: Some("versorgungsfreibetrag_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "versorgung",
            name: "nur Beginnjahr",
            events: vec![
                z("versorgung_jahresrente", 1_800_000, true),
                z("versorgung_beginn_jahr", 2020, true),
            ],
            erwartet: Some("versorgungsfreibetrag_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "versorgung",
            name: "nur Bemessungsgrundlage",
            events: vec![
                z("versorgung_jahresrente", 1_800_000, true),
                z("versorgung_bemessungsgrundlage", 1_500_000, true),
            ],
            erwartet: Some("versorgungsfreibetrag_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "versorgung",
            name: "Beginnjahr nur vorlaeufig, Bemessungsgrundlage bestaetigt",
            events: vec![
                z("versorgung_jahresrente", 1_800_000, true),
                z("versorgung_beginn_jahr", 2020, false),
                z("versorgung_bemessungsgrundlage", 1_500_000, true),
            ],
            erwartet: Some("versorgungsfreibetrag_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "versorgung",
            name: "beides bestaetigt",
            events: vec![
                z("versorgung_jahresrente", 1_800_000, true),
                z("versorgung_beginn_jahr", 2020, true),
                z("versorgung_bemessungsgrundlage", 1_500_000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "versorgung",
            name: "kein Versorgungsbezug",
            events: vec![],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "GdB 20, Aufwendungen, Wahlrecht unbeantwortet",
            events: vec![
                z("rentner_grad_der_behinderung", 20, true),
                z("behinderungsbedingte_aufwendungen", 100_000, true),
            ],
            erwartet: Some("behinderungsbedingte_aufwendungen_wahlrecht_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "GdB 19, Aufwendungen",
            events: vec![
                z("rentner_grad_der_behinderung", 19, true),
                z("behinderungsbedingte_aufwendungen", 100_000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "Merkzeichen H ohne GdB, Aufwendungen, Wahlrecht unbeantwortet",
            events: vec![
                b("rentner_hilflos_blind_taubblind", true, true),
                z("behinderungsbedingte_aufwendungen", 100_000, true),
            ],
            erwartet: Some("behinderungsbedingte_aufwendungen_wahlrecht_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "GdB 50, Aufwendungen, Wahlrecht bestaetigt",
            events: vec![
                z("rentner_grad_der_behinderung", 50, true),
                z("behinderungsbedingte_aufwendungen", 100_000, true),
                b("behinderungsbedingte_aufwendungen_wahlrecht_pb", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "GdB 50, Aufwendungen 0",
            events: vec![
                z("rentner_grad_der_behinderung", 50, true),
                z("behinderungsbedingte_aufwendungen", 0, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "GdB 50, Aufwendungen, Kind-Pauschbetrag uebertragen",
            events: vec![
                z("rentner_grad_der_behinderung", 50, true),
                z("behinderungsbedingte_aufwendungen", 100_000, true),
                t("kind_idnr", "12345678901", true),
                b("kind_behinderten_pb_antrag", true, true),
                b("kind_pb_nicht_selbst_genutzt", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "GdB 50, Aufwendungen, Kind-IdNr nur 10 Zeichen",
            events: vec![
                z("rentner_grad_der_behinderung", 50, true),
                z("behinderungsbedingte_aufwendungen", 100_000, true),
                t("kind_idnr", "1234567890", true),
                b("kind_behinderten_pb_antrag", true, true),
                b("kind_pb_nicht_selbst_genutzt", true, true),
            ],
            erwartet: Some("behinderungsbedingte_aufwendungen_wahlrecht_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "zusammen, Partner GdB 30, Aufwendungen Partner, Wahlrecht unbeantwortet",
            events: vec![
                t("veranlagung", "zusammen", true),
                z("rentner_grad_der_behinderung_partner", 30, true),
                z("behinderungsbedingte_aufwendungen_partner", 100_000, true),
            ],
            erwartet: Some("behinderungsbedingte_aufwendungen_wahlrecht_partner_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "zusammen, Partner GdB 19, Aufwendungen Partner",
            events: vec![
                t("veranlagung", "zusammen", true),
                z("rentner_grad_der_behinderung_partner", 19, true),
                z("behinderungsbedingte_aufwendungen_partner", 100_000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "behinderung",
            name: "einzel, Partner GdB 30: Konsistenz",
            events: vec![
                t("veranlagung", "einzel", true),
                z("rentner_grad_der_behinderung_partner", 30, true),
                z("behinderungsbedingte_aufwendungen_partner", 100_000, true),
            ],
            erwartet: Some("partner_konsistenz_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "kinderbetreuung",
            name: "Kind unter 14, 3.000 EUR, Betreuungsart unbeantwortet",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
            ],
            erwartet: Some("kinderbetreuung_reine_betreuung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "kinderbetreuung",
            name: "reine Betreuung ja, Zahlung unbeantwortet",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_betreuung_reine_betreuung", true, true),
            ],
            erwartet: Some("kinderbetreuung_zahlung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "kinderbetreuung",
            name: "reine Betreuung ja, Zahlung nein",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_betreuung_reine_betreuung", true, true),
                b("kind_betreuung_rechnung_ueberweisung", false, true),
            ],
            erwartet: Some("kinderbetreuung_zahlung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "kinderbetreuung",
            name: "reine Betreuung ja, Zahlung ja",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_betreuung_reine_betreuung", true, true),
                b("kind_betreuung_rechnung_ueberweisung", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "kinderbetreuung",
            name: "reine Betreuung nein (bestaetigt)",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_betreuung_reine_betreuung", false, true),
            ],
            erwartet: Some("kinderbetreuung_reine_betreuung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "kinderbetreuung",
            name: "Kind unter 14 nein: nicht qualifiziert",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", false, true),
                z("kinderbetreuungskosten", 300_000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "kinderbetreuung",
            name: "Kosten 0",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 0, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "kinderbetreuung",
            name: "zweites Kind ohne Betreuungsart",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_betreuung_reine_betreuung", true, true),
                b("kind_betreuung_rechnung_ueberweisung", true, true),
                b("kind_unter_14_haushaltszugehoerig__2", true, true),
                z("kinderbetreuungskosten__2", 300_000, true),
            ],
            erwartet: Some("kinderbetreuung_reine_betreuung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "fremd",
            name: "kein_vuv = nein (Vermietung liegt vor)",
            events: vec![b("kein_vuv", false, true)],
            erwartet: Some("einkunftsart_nicht_ring_faehig"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "fremd",
            name: "kein_vuv = ja",
            events: vec![b("kein_vuv", true, true)],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "fremd",
            name: "kein_sonstige = nein gilt auf der Rentner-Scheibe nicht als Fremd-Art",
            events: vec![b("kein_sonstige", false, true)],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::Gesamt,
            gruppe: "fremd",
            name: "Gegenprobe gesamt: kein_sonstige = nein",
            events: vec![b("kein_sonstige", false, true)],
            erwartet: Some("einkunftsart_nicht_ring_faehig"),
        },
        Fall {
            scheibe: Scheibe::Gesamt,
            gruppe: "fremd",
            name: "Gegenprobe gesamt: kein_vuv = nein gilt dort nicht als Fremd-Art",
            events: vec![b("kein_vuv", false, true)],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "partner19",
            name: "zusammen ohne Person-B-Kegel: die Rentner-Scheibe verlangt ihn nicht",
            events: vec![t("veranlagung", "zusammen", true)],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::Gesamt,
            gruppe: "partner19",
            name: "Gegenprobe gesamt: zusammen ohne Person-B-Kegel",
            events: vec![t("veranlagung", "zusammen", true)],
            erwartet: Some("partner_kegel_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "keine_lohnersatzleistungen = nein, Betrag fehlt",
            events: vec![b("keine_lohnersatzleistungen", false, true)],
            erwartet: Some("lohnersatz_betrag_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "keine_lohnersatzleistungen = nein, Betrag bestaetigt",
            events: vec![
                b("keine_lohnersatzleistungen", false, true),
                z("p32b_progressionseinkuenfte", 500_000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "kein_verlustvortrag = nein, Betrag fehlt",
            events: vec![b("kein_verlustvortrag", false, true)],
            erwartet: Some("verlustvortrag_betrag_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "kein_verlustvortrag = nein, Betrag bestaetigt",
            events: vec![
                b("kein_verlustvortrag", false, true),
                z("verlustvortrag_bestand", 200_000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "kein_unterhalt = nein, Betrag fehlt",
            events: vec![b("kein_unterhalt", false, true)],
            erwartet: Some("unterhalt_betrag_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "kein_unterhalt = nein, Betrag bestaetigt",
            events: vec![
                b("kein_unterhalt", false, true),
                z("p33a_unterhalt_aufwendungen", 600_000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "Kirchensteuer evangelisch, gezahlt und erstattet fehlen",
            events: vec![t("kist_konfession", "evangelisch", true)],
            erwartet: Some("kirchensteuer_betrag_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "Kirchensteuer evangelisch, nur gezahlt",
            events: vec![
                t("kist_konfession", "evangelisch", true),
                z("kist_gezahlt", 90_000, true),
            ],
            erwartet: Some("kirchensteuer_betrag_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "Kirchensteuer evangelisch, gezahlt und erstattet",
            events: vec![
                t("kist_konfession", "evangelisch", true),
                z("kist_gezahlt", 90_000, true),
                z("kist_erstattet", 0, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "keine Konfession",
            events: vec![t("kist_konfession", "keine", true)],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "kein_realsplitting = nein, Angaben fehlen",
            events: vec![b("kein_realsplitting", false, true)],
            erwartet: Some("realsplitting_angaben_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "kein_realsplitting = nein, Zustimmung verneint",
            events: vec![
                b("kein_realsplitting", false, true),
                b("realsplitting_zustimmung", false, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "kein_realsplitting = nein, Zustimmung ja, Betrag fehlt",
            events: vec![
                b("kein_realsplitting", false, true),
                b("realsplitting_zustimmung", true, true),
            ],
            erwartet: Some("realsplitting_angaben_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "keine_behinderung_pflege = nein, Fahrtkostenfragen fehlen",
            events: vec![b("keine_behinderung_pflege", false, true)],
            erwartet: Some("fahrtkostenpauschale_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "keine_behinderung_pflege = nein, aG/Bl/TBl/H ja",
            events: vec![
                b("keine_behinderung_pflege", false, true),
                b("fahrtkosten_pausch_ag_bl_tbl_h", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "betrag",
            name: "keine_behinderung_pflege = nein, aG/Bl/TBl/H nein, 900-EUR-Frage fehlt",
            events: vec![
                b("keine_behinderung_pflege", false, true),
                b("fahrtkosten_pausch_ag_bl_tbl_h", false, true),
            ],
            erwartet: Some("fahrtkostenpauschale_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "mehrere DBA-Staaten",
            events: vec![b("dba_mehrere_staaten", true, true)],
            erwartet: Some("dba_multi_country_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Kapital und auslaendische Einkuenfte",
            events: vec![
                z("kap_kapitalertraege", 100_000, true),
                z("dba_auslaendische_einkuenfte", 50_000, true),
            ],
            erwartet: Some("dba_kapital_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Progression § 32b und Veraeusserungsgewinn",
            events: vec![
                z("p32b_progressionseinkuenfte", 500_000, true),
                z("rentner_veraeusserungsgewinn", 1_000_000, true),
            ],
            erwartet: Some("p32b_kombi_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Progression § 32b und Gewerbesteuer-Messbetrag",
            events: vec![
                z("p32b_progressionseinkuenfte", 500_000, true),
                z("gewst_messbetrag", 10_000, true),
                z("gewst_hebesatz", 400, true),
            ],
            erwartet: Some("p32b_kombi_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Progression § 32b allein",
            events: vec![z("p32b_progressionseinkuenfte", 500_000, true)],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Veraeusserungsgewinn ohne § 16-Abs.-4-Angaben",
            events: vec![z("rentner_veraeusserungsgewinn", 1_000_000, true)],
            erwartet: Some("p16_4_gate_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Veraeusserungsgewinn mit beiden Angaben",
            events: vec![
                b("kein_gewinn", false, true),
                z("rentner_veraeusserungsgewinn", 1_000_000, true),
                b("rentner_alter_55_oder_berufsunfaehig", true, true),
                b("rentner_freibetrag_erstmalig", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "zusammen, Gewinn Person B ohne Angaben",
            events: vec![
                t("veranlagung", "zusammen", true),
                z("rentner_veraeusserungsgewinn_partner", 1_000_000, true),
            ],
            erwartet: Some("p16_4_gate_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "einzel, Gewinn Person B ohne Angaben: Person B zaehlt nicht",
            events: vec![
                t("veranlagung", "einzel", true),
                z("rentner_veraeusserungsgewinn_partner", 1_000_000, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "kein_vuv = ja, aber Mieteinnahmen",
            events: vec![
                b("kein_vuv", true, true),
                z("vv_einnahmen", 1_200_000, true),
            ],
            erwartet: Some("flag_konsistenz_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Kapital-Aggregat und Verlusttopf",
            events: vec![
                b("kein_kap", false, true),
                z("kap_kapitalertraege", 100_000, true),
                z("kap_gewinn_aktien", 20_000, true),
            ],
            erwartet: Some("kapital_semantik_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "zusammen, Kapital-Aggregat und Verlusttopf Person B",
            events: vec![
                t("veranlagung", "zusammen", true),
                b("kein_kap_partner", false, true),
                z("kap_kapitalertraege_partner", 100_000, true),
                z("kap_gewinn_aktien_partner", 20_000, true),
            ],
            erwartet: Some("kapital_semantik_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Gewinn: Direktwert und EUeR-Quelle",
            events: vec![
                b("kein_gewinn", false, true),
                z("einkuenfte_gewinn", 1_000_000, true),
                z("betriebseinnahmen", 2_000_000, true),
            ],
            erwartet: Some("gewinn_quelle_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Gewinn: Land-/Forstwirtschaft mit EUeR",
            events: vec![
                b("kein_gewinn", false, true),
                t("gewinn_betriebsart", "land_forst", true),
                z("betriebseinnahmen", 2_000_000, true),
            ],
            erwartet: Some("luf_euer_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "einkunft",
            name: "Gewinn: EUeR begonnen, Angaben fehlen",
            events: vec![
                b("kein_gewinn", false, true),
                z("betriebseinnahmen", 2_000_000, true),
            ],
            erwartet: Some("gewinn_angaben_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "dHf im Ausland",
            events: vec![
                z("dhf_unterkunftskosten_monat", 80_000, true),
                b("dhf_im_inland", false, true),
            ],
            erwartet: Some("ausland_dhf_nicht_ring_faehig"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "dHf, Inland unbeantwortet",
            events: vec![z("dhf_unterkunftskosten_monat", 80_000, true)],
            erwartet: Some("dhf_tatbestand_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "dHf, Inland ja, Bedingungen fehlen",
            events: vec![
                z("dhf_unterkunftskosten_monat", 80_000, true),
                b("dhf_im_inland", true, true),
            ],
            erwartet: Some("dhf_tatbestand_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "dHf, Inland ja, alle Bedingungen",
            events: vec![
                z("dhf_unterkunftskosten_monat", 80_000, true),
                b("dhf_im_inland", true, true),
                b("dhf_beruflich_veranlasst", true, true),
                b("dhf_eigener_hausstand", true, true),
                b("dhf_finanzielle_beteiligung", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Verpflegung ohne Monate am Ort",
            events: vec![z("tage_24h", 5, true)],
            erwartet: Some("verpflegung_dreimonatsfrist_aufteilung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Verpflegung, 2 Monate, Mahlzeiten unbeantwortet",
            events: vec![z("tage_24h", 5, true), z("vpf_monate_am_ort", 2, true)],
            erwartet: Some("verpflegung_reduktion_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Verpflegung, 2 Monate, keine Mahlzeitengestellung",
            events: vec![
                z("tage_24h", 5, true),
                z("vpf_monate_am_ort", 2, true),
                b("vpf_keine_mahlzeitengestellung", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name:
                "Verpflegung, 3 Monate, Mahlzeiten unbeantwortet: Frist noch nicht ueberschritten",
            events: vec![z("tage_24h", 5, true), z("vpf_monate_am_ort", 3, true)],
            erwartet: Some("verpflegung_reduktion_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Verpflegung, 4 Monate, Nach-Frist-Tage fehlen",
            events: vec![
                z("tage_24h", 5, true),
                z("vpf_monate_am_ort", 4, true),
                b("vpf_keine_mahlzeitengestellung", true, true),
            ],
            erwartet: Some("verpflegung_dreimonatsfrist_aufteilung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Verpflegung, 4 Monate, Nach-Frist-Tage 0, Unterbrechung unbeantwortet",
            events: vec![
                z("tage_24h", 5, true),
                z("vpf_monate_am_ort", 4, true),
                z("vpf_tage_24h_nach_drei_monaten", 0, true),
                b("vpf_keine_mahlzeitengestellung", true, true),
            ],
            erwartet: Some("verpflegung_dreimonatsfrist_unterbrechung_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Verpflegung, 4 Monate, Nach-Frist-Tage 0, Unterbrechung beantwortet",
            events: vec![
                z("tage_24h", 5, true),
                z("vpf_monate_am_ort", 4, true),
                z("vpf_tage_24h_nach_drei_monaten", 0, true),
                b("vpf_frist_nicht_unterbrochen", true, true),
                b("vpf_keine_mahlzeitengestellung", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Uebernachtung, Ort unbeantwortet",
            events: vec![z("uebernachtung_kosten_monat", 60_000, true)],
            erwartet: Some("uebernachtung_tatbestand_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Uebernachtung, Ort ja, Zeitraum unbeantwortet",
            events: vec![
                z("uebernachtung_kosten_monat", 60_000, true),
                b("uebernachtung_im_inland", true, true),
                b("uebernachtung_auswaerts", true, true),
                b("uebernachtung_alleinnutzung", true, true),
                b("uebernachtung_keine_lange_unterbrechung", true, true),
            ],
            erwartet: Some("uebernachtung_zeitraum_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Uebernachtung, Ort, Bedingungen, Zeitraum",
            events: vec![
                z("uebernachtung_kosten_monat", 60_000, true),
                b("uebernachtung_im_inland", true, true),
                z("uebernachtung_monate_bisher", 0, true),
                z("uebernachtung_monate", 6, true),
                b("uebernachtung_auswaerts", true, true),
                b("uebernachtung_alleinnutzung", true, true),
                b("uebernachtung_keine_lange_unterbrechung", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Arbeitsmittel 500 EUR ohne GWG-Wahl",
            events: vec![z("am_anschaffungskosten", 50_000, true)],
            erwartet: Some("arbeitsmittel_afa_ueber_gwg_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Arbeitsmittel 500 EUR mit GWG-Wahl",
            events: vec![
                z("am_anschaffungskosten", 50_000, true),
                b("am_gwg_sofortabzug_gewaehlt", true, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Arbeitsmittel 900 EUR ohne Nutzungsdauer",
            events: vec![z("am_anschaffungskosten", 90_000, true)],
            erwartet: Some("arbeitsmittel_afa_ueber_gwg_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Arbeitsmittel 900 EUR, Nutzungsdauer 5",
            events: vec![
                z("am_anschaffungskosten", 90_000, true),
                z("arbeitsmittel_nutzungsdauer", 5, true),
            ],
            erwartet: None,
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "wk",
            name: "Arbeitsmittel 900 EUR, Anschaffungsjahr ja ohne Monat",
            events: vec![
                z("am_anschaffungskosten", 90_000, true),
                z("arbeitsmittel_nutzungsdauer", 5, true),
                b("am_afa_ist_anschaffungsjahr", true, true),
            ],
            erwartet: Some("arbeitsmittel_afa_ueber_gwg_offen"),
        },
        Fall {
            scheibe: Scheibe::RentnerGesamt,
            gruppe: "konsistenz",
            name: "Alleinerziehend und zusammen",
            events: vec![
                t("veranlagung", "zusammen", true),
                b("fam_alleinstehend", true, true),
            ],
            erwartet: Some("alleinerziehend_konsistenz_offen"),
        },
    ]
}

/// Die Bindung der Scheibe: nur ihre Feld-Ids, wie `api._scheibe_bindung`. Der volle Index kennt mehr Flags und liesse
/// `flag_widersprueche` auf Felder ansprechen, die die Scheibe nie fragt.
fn scheiben_index(scheibe: Scheibe) -> &'static BindungIndex<'static> {
    static GESAMT: OnceLock<BindungIndex<'static>> = OnceLock::new();
    static RENTNER: OnceLock<BindungIndex<'static>> = OnceLock::new();
    static AN_GESAMT: OnceLock<BindungIndex<'static>> = OnceLock::new();
    let zelle = match scheibe {
        Scheibe::RentnerGesamt => &RENTNER,
        Scheibe::AnGesamt => &AN_GESAMT,
        _ => &GESAMT,
    };
    zelle.get_or_init(|| {
        let ids = Cfg::fuer(scheibe).felder(|_| Vec::new()).unwrap();
        index()
            .iter()
            .filter(|(k, _)| ids.contains(k))
            .map(|(k, b)| (k.clone(), *b))
            .collect()
    })
}

/// Der Sperrgrund der Scheibe fuer VZ 2025 auf einem Store aus den Events (Store UND Scheiben-Bindung, also Instanzen
/// wie im Betrieb; der Guard sieht Roh-Felder, `nur_bestaetigt` wird nicht gelesen).
fn grund(scheibe: Scheibe, events: &[Ev]) -> Option<&'static str> {
    let st = store(events);
    let f = felder(&st);
    let q = Instanzquelle {
        store: Some(&st),
        bindung: Some(scheiben_index(scheibe)),
        nur_bestaetigt: false,
    };
    let cfg = Cfg::fuer(scheibe);
    an_gesamt_sperrgrund(&f, Some(&cfg), Some(Vz::Vz2025), &q)
        .unwrap()
        .map(Sperrgrund::als_str)
}

/// Alle Faelle der Gruppe durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn pruefe(gruppe: &str) {
    let faelle: Vec<Fall> = faelle()
        .into_iter()
        .filter(|f| f.gruppe == gruppe)
        .collect();
    assert!(!faelle.is_empty(), "Gruppe {gruppe} ist leer");
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|f| {
            let got = grund(f.scheibe, &f.events);
            (got != f.erwartet).then(|| {
                format!(
                    "{} [{}]: Rust {got:?}, Orakel {:?}",
                    f.name, f.scheibe, f.erwartet
                )
            })
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len(),
        faelle.len()
    );
}

/// Rente Person A (`beginn_grund`: Beginn vor/nach dem VZ, 0, Freibetrag-Fixierung nur bei aa) und weitere Rente-Instanzen (`RenteInstanzOffen`: unvollstaendig oder nicht bestaetigt; der Beginn je Instanz). Gegenprobe: auf `gesamt` laeuft `rente()` nicht.
#[test]
fn rente_person_a_und_instanzen() {
    pruefe("rente");
}

/// Rente Person B nur bei Zusammenveranlagung: alle vier Kernfelder bestaetigt oder keins; die KV/PV-Weiche braucht eine bestaetigte `versicherungsart_partner`, fuer Kranken- wie fuer Pflegeversicherung.
#[test]
fn rente_person_b_und_kv_pv_weiche() {
    pruefe("rente_b");
}

/// § 19 Abs. 2: Beginnjahr UND Bemessungsgrundlage, beide bestaetigt (`VersorgungsfreibetragOffen`).
#[test]
fn versorgungsfreibetrag_braucht_beginn_und_bemessungsgrundlage() {
    pruefe("versorgung");
}

/// § 33b Abs. 1 S. 1: ein Pauschbetrag ab `GdB` 20 oder mit Merkzeichen, Aufwendungen > 0 und unbeantwortetes Wahlrecht sperren; das Kind-Pauschbetrag-Zeichen (`IdNr` mit 11 Zeichen, Antrag, nicht selbst genutzt) hebt die Sperre auf; der Partner spiegelt es nur bei Zusammenveranlagung.
#[test]
fn behinderung_wahlrecht_schwelle_merkzeichen_kind_und_partner() {
    pruefe("behinderung");
}

/// § 10 Abs. 1 Nr. 5: je qualifiziertem Kind mit Kosten > 0 eine bestaetigte reine Betreuung und ein bestaetigter Zahlungsnachweis; auch ein bestaetigtes Nein sperrt.
#[test]
fn kinderbetreuung_verlangt_betreuungsart_und_zahlung() {
    pruefe("kinderbetreuung");
}

/// Die Fremd-Arten der Scheibe (`kein_vuv` auf `rentner_gesamt`, `kein_sonstige` auf `gesamt`): ein bestaetigtes Nein sperrt, das Flag der jeweils anderen Scheibe nicht.
#[test]
fn fremd_arten_der_scheibe() {
    pruefe("fremd");
}

/// Der Person-B-Kegel (`partner_19`) gilt nur auf `gesamt`, nicht auf `rentner_gesamt`.
#[test]
fn partner_kegel_nur_auf_der_gesamt_scheibe() {
    pruefe("partner19");
}

/// Ein verneintes Screening-Flag ohne bestaetigten Betrag sperrt: Lohnersatz, Verlustvortrag, Unterhalt, Kirchensteuer, Realsplitting, Fahrtkostenpauschale.
#[test]
fn betrag_offen_je_screening_flag() {
    pruefe("betrag");
}

/// § 34c DBA, § 32b-Koinzidenz, § 16 Abs. 4 (Person A und B), Flag-Konsistenz, Kapital-Semantik und Gewinn-Quelle.
#[test]
fn dba_p32b_p16_flag_kapital_und_gewinn() {
    pruefe("einkunft");
}

/// § 9: doppelte Haushaltsfuehrung, Verpflegungspauschale (3-Monats-Frist), Uebernachtung, Arbeitsmittel.
#[test]
fn werbungskosten_dhf_verpflegung_uebernachtung_arbeitsmittel() {
    pruefe("wk");
}

/// § 24b: `fam_alleinstehend` bestaetigt und Zusammenveranlagung widersprechen sich.
#[test]
fn alleinerziehend_und_zusammen_sperrt() {
    pruefe("konsistenz");
}
