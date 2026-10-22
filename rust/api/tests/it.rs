//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.

#[path = "abhaengigkeiten.rs"]
mod abhaengigkeiten;
#[path = "arbeitsmittel_satzfrage_bedingung.rs"]
mod arbeitsmittel_satzfrage_bedingung;
#[path = "backup_restore_rundlauf.rs"]
mod backup_restore_rundlauf;
#[path = "beleg_konfession_haushaltshilfen.rs"]
mod beleg_konfession_haushaltshilfen;
#[path = "bereich_rand_hermetisch.rs"]
mod bereich_rand_hermetisch;
#[path = "berufsverband_hermetisch.rs"]
mod berufsverband_hermetisch;
#[path = "bundesland_frage_ehegatte.rs"]
mod bundesland_frage_ehegatte;
#[path = "c2_satz3_fragen_hermetisch.rs"]
mod c2_satz3_fragen_hermetisch;
#[path = "chat_stderr_prozess_hermetisch.rs"]
mod chat_stderr_prozess_hermetisch;
#[path = "computer_software_nr89.rs"]
mod computer_software_nr89;
#[path = "daten_im_dienst.rs"]
mod daten_im_dienst;
#[path = "dienstwohnung_gestrichen_hermetisch.rs"]
mod dienstwohnung_gestrichen_hermetisch;
#[path = "domain_enums_exhaustiv.rs"]
mod domain_enums_exhaustiv;
#[path = "enum_labels_vollstaendig.rs"]
mod enum_labels_vollstaendig;
#[path = "ergebnis_scheibe_naht.rs"]
mod ergebnis_scheibe_naht;
#[path = "event_naht.rs"]
mod event_naht;
#[path = "flag_und_schalter_hermetisch.rs"]
mod flag_und_schalter_hermetisch;
#[path = "fragen_naht.rs"]
mod fragen_naht;
#[path = "gesamt_ehegatte_versorgung_hermetisch.rs"]
mod gesamt_ehegatte_versorgung_hermetisch;
#[path = "gwg_folgefrage_hermetisch.rs"]
mod gwg_folgefrage_hermetisch;
#[path = "http_grenzen_hermetisch.rs"]
mod http_grenzen_hermetisch;
#[path = "jahr_ausserhalb_hermetisch.rs"]
mod jahr_ausserhalb_hermetisch;
#[path = "kein_kind_sperre.rs"]
mod kein_kind_sperre;
#[path = "kind_idnr_schreibweg_hermetisch.rs"]
mod kind_idnr_schreibweg_hermetisch;
#[path = "kontoauszug_naht.rs"]
mod kontoauszug_naht;
#[path = "kopfnamen.rs"]
mod kopfnamen;
#[path = "korrektur_sweep.rs"]
mod korrektur_sweep;
#[path = "kvpv_zuschuss_hermetisch.rs"]
mod kvpv_zuschuss_hermetisch;
#[path = "lesen_naht.rs"]
mod lesen_naht;
#[path = "lohn_rueckfragen_hermetisch.rs"]
mod lohn_rueckfragen_hermetisch;
#[path = "lstb_beleg_hermetisch.rs"]
mod lstb_beleg_hermetisch;
#[path = "null_arbeitstage.rs"]
mod null_arbeitstage;
#[path = "p23_waechter_hermetisch.rs"]
mod p23_waechter_hermetisch;
#[path = "p34_abs3_55_geburtstag_hermetisch.rs"]
mod p34_abs3_55_geburtstag_hermetisch;
#[path = "p34_partner_fragen_hermetisch.rs"]
mod p34_partner_fragen_hermetisch;
#[path = "p34c_fiktive_steuer_frage_hermetisch.rs"]
mod p34c_fiktive_steuer_frage_hermetisch;
#[path = "p34c_staat_folgefrage_hermetisch.rs"]
mod p34c_staat_folgefrage_hermetisch;
#[path = "parteispenden_hermetisch.rs"]
mod parteispenden_hermetisch;
#[path = "partner_sonstige_waechter_hermetisch.rs"]
mod partner_sonstige_waechter_hermetisch;
#[path = "preflight_pflichtfelder.rs"]
mod preflight_pflichtfelder;
#[path = "rav_bav_waechter_hermetisch.rs"]
mod rav_bav_waechter_hermetisch;
#[path = "renten_leistungsmitteilung_texte.rs"]
mod renten_leistungsmitteilung_texte;
#[path = "rentner_ehegatten_kreuze.rs"]
mod rentner_ehegatten_kreuze;
#[path = "rentner_eingangsfrage.rs"]
mod rentner_eingangsfrage;
#[path = "rentner_lohn_versorgung_hermetisch.rs"]
mod rentner_lohn_versorgung_hermetisch;
#[path = "scheiben_cfg.rs"]
mod scheiben_cfg;
#[path = "scheiben_paritaet.rs"]
mod scheiben_paritaet;
#[path = "schulgeld_anteil_hermetisch.rs"]
mod schulgeld_anteil_hermetisch;
#[path = "schulgeld_anteilsfrage_bedingung.rs"]
mod schulgeld_anteilsfrage_bedingung;
#[path = "stand_naht.rs"]
mod stand_naht;
#[path = "ueberlauf_klassen_hermetisch.rs"]
mod ueberlauf_klassen_hermetisch;
#[path = "ueberlauf_profil_waechter.rs"]
mod ueberlauf_profil_waechter;
#[path = "unfallkosten_hermetisch.rs"]
mod unfallkosten_hermetisch;
#[path = "vast_naht_bindung.rs"]
mod vast_naht_bindung;
#[path = "versorgung_abgabe_sperre_hermetisch.rs"]
mod versorgung_abgabe_sperre_hermetisch;
#[path = "versorgung_rueckfragen_hermetisch.rs"]
mod versorgung_rueckfragen_hermetisch;
#[path = "vorjahr_naht.rs"]
mod vorjahr_naht;
#[path = "vz_params_jahre.rs"]
mod vz_params_jahre;
