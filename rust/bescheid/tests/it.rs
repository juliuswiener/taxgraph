//! Alle Testdateien dieses Crates, die kein eigenes Ziel brauchen, in EINEM Test-Binary (weniger Links). Eigene Ziele und der Grund:
//! `Cargo.toml`. Neue Testdatei: hier eintragen; `rust/bindung/tests/testbinaries_vollstaendig.rs` schlaegt sonst an.
//! Aufruf einer Datei: `cargo test -p <crate> --test it <datei>::`.

#[path = "abs3_partner_gewinn.rs"]
mod abs3_partner_gewinn;
#[path = "abzuege_instanzen_hermetisch.rs"]
mod abzuege_instanzen_hermetisch;
#[path = "altersentlastung_je_ehegatte_hermetisch.rs"]
mod altersentlastung_je_ehegatte_hermetisch;
#[path = "altersentlastung_sonstige_hermetisch.rs"]
mod altersentlastung_sonstige_hermetisch;
#[path = "an_gesamt_luecken_sperren.rs"]
mod an_gesamt_luecken_sperren;
#[path = "ausland_bstbk_2022_23_hermetisch.rs"]
mod ausland_bstbk_2022_23_hermetisch;
#[path = "berufsverband_zeile_53_xml.rs"]
mod berufsverband_zeile_53_xml;
#[path = "c2_weit_ang_xml_hermetisch.rs"]
mod c2_weit_ang_xml_hermetisch;
#[path = "checkest_verdrahtung.rs"]
mod checkest_verdrahtung;
#[path = "dba_freistellung_einreichung_hermetisch.rs"]
mod dba_freistellung_einreichung_hermetisch;
#[path = "dba_freistellung_progression.rs"]
mod dba_freistellung_progression;
#[path = "dba_freistellung_sperre.rs"]
mod dba_freistellung_sperre;
#[path = "dba_staat_kz.rs"]
mod dba_staat_kz;
#[path = "dienstwohnung_einreichung_hermetisch.rs"]
mod dienstwohnung_einreichung_hermetisch;
#[path = "eigenschaften.rs"]
mod eigenschaften;
#[path = "einreichung_e2e.rs"]
mod einreichung_e2e;
#[path = "feld_kennung_gate.rs"]
mod feld_kennung_gate;
#[path = "feste_zahl.rs"]
mod feste_zahl;
#[path = "gesamt_ehegatte_versorgung_hermetisch.rs"]
mod gesamt_ehegatte_versorgung_hermetisch;
#[path = "gesamt_mischfaelle_hermetisch.rs"]
mod gesamt_mischfaelle_hermetisch;
#[path = "gewinn_und_vorsorge_einreichung_hermetisch.rs"]
mod gewinn_und_vorsorge_einreichung_hermetisch;
#[path = "gewst_zu_zahlen_wert.rs"]
mod gewst_zu_zahlen_wert;
#[path = "golden_kopf.rs"]
mod golden_kopf;
#[path = "gwg_einzel_afa.rs"]
mod gwg_einzel_afa;
#[path = "gwg_grenze_hermetisch.rs"]
mod gwg_grenze_hermetisch;
#[path = "hausnummer_zusatz.rs"]
mod hausnummer_zusatz;
#[path = "kap_partner_kreuz_erklaerung.rs"]
mod kap_partner_kreuz_erklaerung;
#[path = "kapital_guenstiger_hermetisch.rs"]
mod kapital_guenstiger_hermetisch;
#[path = "kapital_topf_hermetisch.rs"]
mod kapital_topf_hermetisch;
#[path = "kind_idnr_ist_zustand_hermetisch.rs"]
mod kind_idnr_ist_zustand_hermetisch;
#[path = "kinderfreibetrag_je_kind_hermetisch.rs"]
mod kinderfreibetrag_je_kind_hermetisch;
#[path = "meet_zustand_riegel.rs"]
mod meet_zustand_riegel;
#[path = "p34_abs3_55_geburtstag_hermetisch.rs"]
mod p34_abs3_55_geburtstag_hermetisch;
#[path = "p34_partner_antragszeile_hermetisch.rs"]
mod p34_partner_antragszeile_hermetisch;
#[path = "p34_partner_hermetisch.rs"]
mod p34_partner_hermetisch;
#[path = "p34c_abzug_einkuenfte.rs"]
mod p34c_abzug_einkuenfte;
#[path = "p34c_abzug_einreichung_hermetisch.rs"]
mod p34c_abzug_einreichung_hermetisch;
#[path = "p34c_abzug_rechnung.rs"]
mod p34c_abzug_rechnung;
#[path = "p34c_abzug_sperre.rs"]
mod p34c_abzug_sperre;
#[path = "p34c_abzug_weitere_wk_xml.rs"]
mod p34c_abzug_weitere_wk_xml;
#[path = "p35a_p35c_gates_hermetisch.rs"]
mod p35a_p35c_gates_hermetisch;
#[path = "parteispenden_einreichung_hermetisch.rs"]
mod parteispenden_einreichung_hermetisch;
#[path = "parteispenden_ermaessigung.rs"]
mod parteispenden_ermaessigung;
#[path = "parteispenden_sonderausgaben.rs"]
mod parteispenden_sonderausgaben;
#[path = "privatverkauf_einreichung_hermetisch.rs"]
mod privatverkauf_einreichung_hermetisch;
#[path = "rav_bav_einreichung_hermetisch.rs"]
mod rav_bav_einreichung_hermetisch;
#[path = "rav_bav_ring_hermetisch.rs"]
mod rav_bav_ring_hermetisch;
#[path = "rente_bmf_uebersicht_hermetisch.rs"]
mod rente_bmf_uebersicht_hermetisch;
#[path = "rente_pauschbetrag_einmal.rs"]
mod rente_pauschbetrag_einmal;
#[path = "rentenbeginn_jahr.rs"]
mod rentenbeginn_jahr;
#[path = "rentner_lohn_versorgung_hermetisch.rs"]
mod rentner_lohn_versorgung_hermetisch;
#[path = "sa_pauschbetrag_neben_vorsorge_hermetisch.rs"]
mod sa_pauschbetrag_neben_vorsorge_hermetisch;
#[path = "scheiben_tabellen_konsistenz.rs"]
mod scheiben_tabellen_konsistenz;
#[path = "slot_waechter.rs"]
mod slot_waechter;
#[path = "sonderausgaben_agb_abbildung_hermetisch.rs"]
mod sonderausgaben_agb_abbildung_hermetisch;
#[path = "sperre_mutanten_luecken.rs"]
mod sperre_mutanten_luecken;
#[path = "sperre_scheiben_hermetisch.rs"]
mod sperre_scheiben_hermetisch;
#[path = "sperrgrund_kreuz_sweep.rs"]
mod sperrgrund_kreuz_sweep;
#[path = "steuerermaessigungen_abbildung_hermetisch.rs"]
mod steuerermaessigungen_abbildung_hermetisch;
#[path = "ueberlauf_hermetisch.rs"]
mod ueberlauf_hermetisch;
#[path = "ueberlauf_luecke_hermetisch.rs"]
mod ueberlauf_luecke_hermetisch;
#[path = "ueberlauf_zweige_hermetisch.rs"]
mod ueberlauf_zweige_hermetisch;
#[path = "unfallkosten_einreichung_hermetisch.rs"]
mod unfallkosten_einreichung_hermetisch;
#[path = "unfallkosten_neben_entfernungspauschale.rs"]
mod unfallkosten_neben_entfernungspauschale;
#[path = "unfallkosten_weitere_wk_xml.rs"]
mod unfallkosten_weitere_wk_xml;
#[path = "versorgung_abgabe_sperre.rs"]
mod versorgung_abgabe_sperre;
#[path = "versorgung_altersgrenze_ring.rs"]
mod versorgung_altersgrenze_ring;
#[path = "versorgung_im_lohn_enthalten.rs"]
mod versorgung_im_lohn_enthalten;
#[path = "versorgung_sperre_faelle.rs"]
mod versorgung_sperre_faelle;
#[path = "waehlervereinigungen.rs"]
mod waehlervereinigungen;
#[path = "waehlervereinigungen_einreichung_hermetisch.rs"]
mod waehlervereinigungen_einreichung_hermetisch;
#[path = "znull_kegel.rs"]
mod znull_kegel;
