//! Transform-Tabellen der Deklaration (`est_mapping.py:214-517`). Die Begruendung je Eintrag
//! steht im Python-Original an derselben Stelle; hier nur, was den Port betrifft.

use domain::{Konfession, Rentenart};

/// Kz, die jede Deklaration unbedingt setzt (`Art_Erkl` E0100001, Julius-Entscheidung
/// 2026-08-10: das Produkt erzeugt nur Einkommensteuererklaerungen).
pub const KONSTANTE_KZ: &[&str] = &["E0100001"];

/// Kz, die aus `stammdaten_iban` abgeleitet werden (Laenderpraefix-Weiche + Kontoinhaber).
pub const IBAN_TRANSFORM_ZIEL_KZ: &[&str] = &["E0102102", "E0102603", "E0101601"];

/// Klasse d — Negation: Store-Feld → Kz mit invertiertem Wert.
pub(crate) const NEGATION: &[(&str, &str)] = &[("fam_alleinstehend", "E0503701")];

/// Klasse e — Multiplikation: Zaehlfeld → N Anlage-Kind-Instanzen.
pub(crate) const MULTIPLIKATION: &[&str] = &["fam_anzahl_kinder"];

/// Klasse a — dokumentierte Aggregation (Summe dokumentiert, NICHT deklariert).
pub(crate) const DOKUMENTIERT_AGGREGAT: &[(&str, &[&str])] = &[(
    "E0703838",
    &[
        "vv_gebaeude_afa",
        "vv_schuldzinsen",
        "vv_erhaltungsaufwand",
        "vv_sonstige_wk",
    ],
)];

/// § 23 Rohdaten-Felder je Instanz; der Gewinn wird in der Deklaration berechnet.
pub(crate) const P23_BETRAGSFELDER: &[&str] = &[
    "p23_veraeusserungspreis",
    "p23_anschaffung_herstellungskosten",
    "p23_werbungskosten",
];
pub(crate) const P23_ART_FELD: &str = "p23_veraeusserungs_typ";
pub(crate) const P23_GEWINN_KZ: &[(&str, &str)] =
    &[("grundstueck", "E0306801"), ("anderes_wg", "E0307701")];

/// Ein Wert-Feld, dessen Kz vom Wert eines Art-Felds abhaengt (Klasse f / g×f).
pub(crate) struct Verzweigung {
    pub feld: &'static str,
    pub art_feld: &'static str,
    pub kz: ArtKz,
}

/// Die Kz je Art-Wert. Die Rentenart ist ein Domain-Enum; die uebrigen Art-Felder bleiben
/// Text-Tabellen (`REWRITE_PLAN` §7, 9b-B: `Lage<T>` nur fuer Veranlagung, Konfession, Bundesland
/// und Rentenart).
#[derive(Clone, Copy)]
pub(crate) enum ArtKz {
    Rente(fn(Rentenart) -> &'static str),
    Text(&'static [(&'static str, &'static str)]),
}

impl ArtKz {
    /// Alle `(Art-Wert, Kz)`-Paare; die Rentenart in der Reihenfolge der Bindung.
    pub(crate) fn paare(self) -> Vec<(&'static str, &'static str)> {
        match self {
            Self::Rente(kz) => Rentenart::ALLE
                .into_iter()
                .map(|r| (r.als_str(), kz(r)))
                .collect(),
            Self::Text(tabelle) => tabelle.to_vec(),
        }
    }
}

const fn rente_wert(art: Rentenart) -> &'static str {
    match art {
        Rentenart::GesetzlicheRente
        | Rentenart::BerufsstaendischeVersorgung
        | Rentenart::PrivateBasisrente => "E1800301",
        Rentenart::PrivateLeibrente => "E1801601",
        Rentenart::SonstigeLeibrente => "E1803102",
    }
}
const fn rente_beginn(art: Rentenart) -> &'static str {
    match art {
        Rentenart::GesetzlicheRente
        | Rentenart::BerufsstaendischeVersorgung
        | Rentenart::PrivateBasisrente => "E1800501",
        Rentenart::PrivateLeibrente => "E1801701",
        Rentenart::SonstigeLeibrente => "E1803202",
    }
}
const VERAEUSSERUNG: &[(&str, &str)] = &[
    ("gewerbe", "E0801301"),
    ("selbstaendig", "E0804501"),
    ("land_forst", "E0901201"),
];
/// § 34 Abs. 3 Antragszeile im Container der Basiszeile darueber (XSD E10-2024/E10-2025:
/// `VAe_G_FB_Antr` / `Vor_FB` / `VAe_G_FB_Antr`).
const ANTRAG_ABS3: &[(&str, &str)] = &[
    ("gewerbe", "E0801602"),
    ("selbstaendig", "E0805003"),
    ("land_forst", "E0901704"),
];
/// §§ 13-18: `land_forst` bewusst ohne Kz (Anlage L hat vier Kandidaten, Auswahl haengt an zwei
/// fehlenden Feldern) → fail-closed.
const GEWINN: &[(&str, &str)] = &[("gewerbe", "E0800302"), ("selbstaendig", "E0803202")];
const GEWINN_BEZEICHNUNG: &[(&str, &str)] =
    &[("gewerbe", "E0800301"), ("selbstaendig", "E0803101")];
const BASIS_KV: &[(&str, &str)] = &[
    ("gesetzlich_an", "E2001203"),
    ("gesetzlich_freiwillig", "E2001805"),
    ("privat", "E2003104"),
];
const BASIS_PV: &[(&str, &str)] = &[
    ("gesetzlich_an", "E2001505"),
    ("gesetzlich_freiwillig", "E2002105"),
    ("privat", "E2003202"),
];

/// Klasse f — Verzweigung (`est_mapping.py:261-326`).
pub(crate) const VERZWEIGUNG: &[Verzweigung] = &[
    Verzweigung {
        feld: "rentner_jahresrente",
        art_feld: "rentner_renten_art",
        kz: ArtKz::Rente(rente_wert),
    },
    Verzweigung {
        feld: "rentner_renten_beginn_jahr",
        art_feld: "rentner_renten_art",
        kz: ArtKz::Rente(rente_beginn),
    },
    Verzweigung {
        feld: "rentner_veraeusserungsgewinn",
        art_feld: "rentner_veraeusserungs_betriebsart",
        kz: ArtKz::Text(VERAEUSSERUNG),
    },
    Verzweigung {
        feld: "p34_abs3_antragsbetrag",
        art_feld: "rentner_veraeusserungs_betriebsart",
        kz: ArtKz::Text(ANTRAG_ABS3),
    },
    Verzweigung {
        feld: "p35c_massnahme_einzelbetrag",
        art_feld: "p35c_massnahme_art",
        kz: ArtKz::Text(&[
            ("waende", "E0241001"),
            ("dach", "E0241101"),
            ("geschossdecken", "E0241201"),
            ("fenster_tueren", "E0241301"),
            ("sommerlicher_waermeschutz", "E0241302"),
            ("lueftung", "E0241401"),
            ("heizung", "E0241501"),
            ("digital", "E0241601"),
            ("heizung_optimierung", "E0241701"),
        ]),
    },
    Verzweigung {
        feld: "einkuenfte_gewinn",
        art_feld: "gewinn_betriebsart",
        kz: ArtKz::Text(GEWINN),
    },
    Verzweigung {
        feld: "gewinn_bezeichnung",
        art_feld: "gewinn_betriebsart",
        kz: ArtKz::Text(GEWINN_BEZEICHNUNG),
    },
    Verzweigung {
        feld: "basis_kv",
        art_feld: "versicherungsart",
        kz: ArtKz::Text(BASIS_KV),
    },
    Verzweigung {
        feld: "basis_pv",
        art_feld: "versicherungsart",
        kz: ArtKz::Text(BASIS_PV),
    },
];

/// Klasse g×f — Verzweigung Person B (`est_mapping.py:368-396`); Wert geht nach `person_b`.
pub(crate) const PARTNER_VERZWEIGUNG: &[Verzweigung] = &[
    Verzweigung {
        feld: "rentner_jahresrente_partner",
        art_feld: "rentner_renten_art_partner",
        kz: ArtKz::Rente(rente_wert),
    },
    Verzweigung {
        feld: "rentner_renten_beginn_jahr_partner",
        art_feld: "rentner_renten_art_partner",
        kz: ArtKz::Rente(rente_beginn),
    },
    Verzweigung {
        feld: "einkuenfte_gewinn_partner",
        art_feld: "gewinn_betriebsart_partner",
        kz: ArtKz::Text(GEWINN),
    },
    Verzweigung {
        feld: "gewinn_bezeichnung_partner",
        art_feld: "gewinn_betriebsart_partner",
        kz: ArtKz::Text(GEWINN_BEZEICHNUNG),
    },
    Verzweigung {
        feld: "rentner_veraeusserungsgewinn_partner",
        art_feld: "rentner_veraeusserungs_betriebsart_partner",
        kz: ArtKz::Text(VERAEUSSERUNG),
    },
    Verzweigung {
        feld: "basis_kv_partner",
        art_feld: "versicherungsart_partner",
        kz: ArtKz::Text(BASIS_KV),
    },
    Verzweigung {
        feld: "basis_pv_partner",
        art_feld: "versicherungsart_partner",
        kz: ArtKz::Text(BASIS_PV),
    },
];

/// Klasse g — Person-B-Felder mit denselben Kz wie Person A (`est_mapping.py:332-364`).
pub(crate) const PARTNER_INSTANZ: &[(&str, &str)] = &[
    ("bruttoarbeitslohn_partner", "E0200201"),
    ("vor_an_anteil_rv_partner", "E2000401"),
    ("vor_ag_anteil_rv_partner", "E2000801"),
    ("vor_rv_ausserhalb_lstb_partner", "E2000601"),
    ("kap_kapitalertraege_partner", "E1900701"),
    ("kap_gewinn_aktien_partner", "E1900901"),
    ("kap_verlust_aktien_partner", "E1901301"),
    ("kap_verlust_sonstige_partner", "E1901201"),
    ("rentner_grad_der_behinderung_partner", "E0109708"),
    ("rentner_hilflos_blind_taubblind_partner", "E0109706"),
    ("steuerklasse_partner", "E0200002"),
    ("p36_lohnsteuer_partner", "E0200301"),
    ("kirchensteuer_arbeitgeber_partner", "E0200501"),
    ("gewst_hebesatz_partner", "E0801705"),
    ("gewst_messbetrag_partner", "E0801606"),
    ("gewst_zu_zahlen_partner", "E0801704"),
];

/// Der Pflege-Block § 33b Abs. 6 EStG (Gruppe `AgB/Pflege_PB/Einz`) als eingefrorene Kz-Menge.
/// Fundstelle: `produkt/bindung/bindung_rentner.yaml:293-437` (sieben gebundene Felder), XSD-Pfade
/// gegen E10-2025.xsd. E0161901 („weitere an der Pflege beteiligte Personen") liegt in derselben
/// Gruppe und fehlt hier BEWUSST: kein Bindungsfeld, der Mapper schreibt es nie.
/// ponytail: feste Siebener-Menge, jahresunabhaengig, damit `deklariere()` ohne XSD laeuft und in
/// jeder Umgebung dieselbe Deklaration ergibt. Upgrade: aus dem XSD ableiten, sobald weitere
/// Felder des Blocks in die Bindung kommen. Zwilling von `PFLEGE_KZ` (`est_mapping.py`).
pub(crate) const PFLEGE_KZ: &[&str] = &[
    "E0161606", "E0161808", "E0161607", "E0161506", "E0110601", "E0106507", "E0106603",
];

/// Option A (Julius-Entscheidung 2026-08-10): KAP-Felder, deren gemeinsame 0 nicht deklariert wird.
pub(crate) const KAP_FELDER_A: &[&str] = &[
    "kap_kapitalertraege",
    "kap_gewinn_aktien",
    "kap_verlust_aktien",
    "kap_gewinn_sonstige",
    "kap_verlust_sonstige",
];
pub(crate) const KAP_FELDER_B: &[&str] = &[
    "kap_kapitalertraege_partner",
    "kap_gewinn_aktien_partner",
    "kap_verlust_aktien_partner",
    "kap_gewinn_sonstige_partner",
    "kap_verlust_sonstige_partner",
];
pub(crate) const KAP_NULL_GRUND: &str = "Option A (Julius-Entscheidung 2026-08-10): alle KAP-Felder beider \
Personen bestaetigt 0 -- als Kz deklariert wuerde ELSTER 'Kapitalertraege erklaert' lesen und einen \
Angabegrund verlangen, den wir nicht liefern.";

/// Wann eine Pflichtfeld-Gruppe greift (`est_mapping.py:435-449`).
#[derive(Clone, Copy)]
pub(crate) enum PflichtBedingung {
    /// Unbedingt Pflicht.
    Immer,
    /// Gruppe ganz unberuehrt ODER ganz gesetzt (gemessen per ERiC-Gegenprobe).
    AlleOderKeins,
}

impl PflichtBedingung {
    pub(crate) const fn als_str(self) -> &'static str {
        match self {
            Self::Immer => "immer",
            Self::AlleOderKeins => "alle_oder_keins",
        }
    }
}

/// Klasse PFLICHT — gepflegte Liste aus checkESt-Sweeps gegen ERiC 44.2.4.0 (`est_mapping.py:454-462`).
pub(crate) const PFLICHTFELDER: &[(PflichtBedingung, &str, &[&str])] = &[
    (
        PflichtBedingung::Immer,
        "44.2.4.0",
        &[
            "stammdaten_nachname",
            "stammdaten_vorname",
            "stammdaten_geburtsdatum",
            "stammdaten_strasse",
            "stammdaten_plz",
            "stammdaten_wohnort",
            "kist_konfession",
        ],
    ),
    (
        PflichtBedingung::AlleOderKeins,
        "44.2.4.0",
        &["bruttoarbeitslohn", "steuerklasse", "p36_lohnsteuer"],
    ),
    (
        PflichtBedingung::AlleOderKeins,
        "44.2.4.0",
        &["vor_an_anteil_rv", "vor_ag_anteil_rv"],
    ),
];

/// Klasse i — Laien-Enum → amtlicher Religionsschluessel; „andere" bewusst ohne Code.
pub(crate) struct Wertekodierung {
    pub feld: &'static str,
    pub kz: &'static str,
    pub code: fn(Konfession) -> Option<&'static str>,
    pub hinweis_unbekannt: &'static str,
}

const fn konfession_code(k: Konfession) -> Option<&'static str> {
    match k {
        Konfession::Keine => Some("11"),
        Konfession::Evangelisch => Some("02"),
        Konfession::RoemischKatholisch => Some("03"),
        Konfession::Andere => None,
    }
}

pub(crate) const WERTEKODIERUNG: &[Wertekodierung] = &[
    Wertekodierung {
        feld: "kist_konfession",
        kz: "E0100402",
        code: konfession_code,
        hinweis_unbekannt: "Ihre Konfession laesst sich nicht automatisch dem amtlichen \
Religionsschluessel zuordnen. Der amtliche Schluessel unterscheidet rund zwanzig einzelne \
Koerperschaften, viele davon regional (etwa juedische Gemeinden je nach Bundesland). Bitte tragen Sie \
die Konfession in Mein ELSTER nach oder waehlen Sie eine der angebotenen, falls sie zutrifft. Alles \
Uebrige Ihrer Erklaerung bleibt davon unberuehrt.",
    },
    Wertekodierung {
        feld: "kist_konfession_partner",
        kz: "E0101002",
        code: konfession_code,
        hinweis_unbekannt: "Die Konfession Ihres Ehegatten laesst sich nicht automatisch dem amtlichen \
Religionsschluessel zuordnen (rund zwanzig Koerperschaften, viele regional). Bitte in Mein ELSTER \
nachtragen oder eine der angebotenen waehlen, falls sie zutrifft. Alles Uebrige Ihrer Erklaerung bleibt \
davon unberuehrt.",
    },
];

/// Nachschlag in einer `(schluessel, wert)`-Tabelle.
pub(crate) fn suche<'a>(tabelle: &'a [(&'a str, &'a str)], schluessel: &str) -> Option<&'a str> {
    tabelle
        .iter()
        .find(|(k, _)| *k == schluessel)
        .map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map, Value};

    /// Abbild der Kz-Tabellen aus `est_mapping.py`, erzeugt von `tools/parity/dump_kz_tabellen.py`
    /// (Python-Gegenstueck: `tests/test_kz_tabellen_fixture.py`). Beide Tests laufen ohne `PARITY=1`:
    /// ein Tausch oder Tippfehler in `tabellen.rs` ist sonst nur mit dem Python-Orakel sichtbar
    /// (`elster_paritaet`); `Kz::ist_gueltig` prueft nur die Form, nicht, ob die Kz die richtige ist.
    const FIXTURE: &str = include_str!("../../fixtures/kz_tabellen.json");

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).unwrap()
    }

    fn sortiert(liste: &[&str]) -> Value {
        let mut l = liste.to_vec();
        l.sort_unstable();
        json!(l)
    }

    /// Objekt aus `(Schluessel, Wert)`-Paaren; ein doppelter Schluessel ist ein Fehler der Tabelle.
    fn objekt(paare: &[(&str, &str)]) -> Value {
        let m: Map<String, Value> = paare
            .iter()
            .map(|(k, v)| ((*k).to_owned(), json!(v)))
            .collect();
        assert_eq!(m.len(), paare.len(), "doppelter Schluessel in {paare:?}");
        Value::Object(m)
    }

    fn verzweigung(tabelle: &[Verzweigung]) -> Value {
        let m: Map<String, Value> = tabelle
            .iter()
            .map(|v| {
                let eintrag = json!({"art_feld": v.art_feld, "kz": objekt(&v.kz.paare())});
                (v.feld.to_owned(), eintrag)
            })
            .collect();
        assert_eq!(m.len(), tabelle.len(), "doppeltes Feld in der Verzweigung");
        Value::Object(m)
    }

    fn dokumentiert_aggregat() -> Value {
        let m: Map<String, Value> = DOKUMENTIERT_AGGREGAT
            .iter()
            .map(|(kz, felder)| ((*kz).to_owned(), sortiert(felder)))
            .collect();
        Value::Object(m)
    }

    fn wertekodierung() -> Value {
        let m: Map<String, Value> = WERTEKODIERUNG
            .iter()
            .map(|w| {
                let code: Map<String, Value> = Konfession::ALLE
                    .into_iter()
                    .filter_map(|k| (w.code)(k).map(|c| (k.als_str().to_owned(), json!(c))))
                    .collect();
                (w.feld.to_owned(), json!({"kz": w.kz, "code": code}))
            })
            .collect();
        Value::Object(m)
    }

    /// Der Wert einer Regal-Zone: Literale als Menge, Liste oder Paarobjekt. Ein fehlender
    /// Anker ist ein Fehler der Zone, nie ein Skip — sonst waere „nichts gefunden" gruen.
    fn zone_menge(datei: &str, anker: &str, offnen: &str, ende: &str) -> Value {
        crate::regal::menge(&crate::regal::zone(datei, anker, offnen, ende).unwrap_or_else(|e| panic!("{datei} {e}")))
    }

    fn zone_liste(datei: &str, anker: &str, offnen: &str, ende: &str) -> Value {
        crate::regal::folge(&crate::regal::zone(datei, anker, offnen, ende).unwrap_or_else(|e| panic!("{datei} {e}")))
    }

    fn zone_paare(datei: &str, anker: &str, offnen: &str, ende: &str) -> Value {
        crate::regal::paar_objekt(&crate::regal::zone(datei, anker, offnen, ende).unwrap_or_else(|e| panic!("{datei} {e}")))
    }

    /// Das n-te Literal einer einzeiligen Zone.
    fn zone_einzel(datei: &str, anker: &str, n: usize) -> Value {
        json!(crate::regal::einzel(&crate::regal::zone(datei, anker, "", "").unwrap_or_else(|e| panic!("{datei} {e}")), n))
    }

    /// Das erste Literal der Zeile mit `anker`, als Text (nicht als `Value`).
    fn einzel_der(datei: &str, anker: &str) -> String {
        crate::regal::einzel(&crate::regal::zone(datei, anker, "", "").unwrap_or_else(|e| panic!("{datei} {e}")), 0)
    }

    /// Die Ueberlebenden der Inventur: Regeln, die nicht in `tabellen.rs` stehen. Jede ist
    /// eine Zone in der Regal-Datei, kein handgeschriebener Wert.
    fn aus_regal() -> Value {
        let kf = "kz_format.rs";
        let d = "deklaration.rs";
        let x = "xml.rs";
        // Wertbasiert, nicht ueber Quelltext: die Klasse, die Rust einem rc gibt, gegen die von Python.
        let rcs = [
            crate::RC_OK,
            crate::RC_PLAUSIBILITAET,
            crate::RC_IO_SCHEMA_VALIDIERUNGSFEHLER,
            crate::RC_HERSTELLER_GESPERRT,
            crate::RC_DATENARTVERSION_UNBEKANNT,
            crate::RC_IO_UNERWARTETE_ELEMENTE,
        ];
        let klassen: Map<String, Value> = rcs
            .iter()
            .map(|rc| (crate::klasse_name(crate::klassifiziere_rc(*rc)).to_owned(), json!(rc)))
            .collect();
        let mut nicht_geprueft_klassen: Vec<&str> = rcs
            .iter()
            .map(|rc| crate::klassifiziere_rc(*rc))
            .filter(|k| crate::nicht_geprueft(*k))
            .map(crate::klasse_name)
            .collect();
        nicht_geprueft_klassen.sort_unstable();
        nicht_geprueft_klassen.dedup();
        let vorwahl_e77 = zone_einzel("xsd.rs", "if kz.get(1..3) == Some(", 0)
            .as_str()
            .map(str::to_owned)
            .unwrap_or_default();
        let schluessel_e10_v = crate::regal::wortliche(
            crate::regal::zone("xsd.rs", ".entry(vec![\"E10\"", "", "").unwrap_or_else(|e| panic!("xsd.rs {e}")),
        )
        .join("/");
        let r = json!({
            "abzugs_kz": zone_menge(kf, "pub const ABZUGS_KZ", "[", "]"),
            "komma_ohne_e60_kz": zone_menge(kf, "pub const KOMMA_OHNE_E60_KZ", "[", "]"),
            "datums_kz": zone_menge(kf, "pub const DATUMS_KZ", "[", "]"),
            "null_unzulaessig": {
                "je_vz": {
                    "2024": zone_menge(kf, "const NULL_UNZULAESSIG_KZ_2024", "[", "]"),
                    "2025": zone_menge(kf, "const NULL_UNZULAESSIG_KZ_2025", "[", "]"),
                },
                "vereinigung": zone_einzel(kf, "pub const NULL_UNZULAESSIG_KZ_VEREINIGUNG", 0),
            },
            "multiplikation": zone_liste("tabellen.rs", "pub(crate) const MULTIPLIKATION", "[", "]"),
            "p35a_summe_aus_posten": Value::Array(
                crate::regal::wortliche(&crate::regal::zone(d, "const P35A_SUMME_AUS_POSTEN", "[", "]").unwrap())
                    .chunks(2)
                    .map(|c| json!([c[0], c[1]]))
                    .collect(),
            ),
            "iban_weiche": {
                "praefix": zone_einzel(d, "let kz = if norm.starts_with(", 0),
                "inland": zone_einzel(d, "let kz = if norm.starts_with(", 1),
                "ausland": zone_einzel(d, "let kz = if norm.starts_with(", 2),
            },
            "bankverbindung": {
                "iban": [zone_einzel(d, "fn bankverbindung(&mut self)", 0),
                         zone_einzel(d, "fn bankverbindung(&mut self)", 1)],
                "keine_bankverbindung": zone_einzel(d, "fn bankverbindung(&mut self)", 2),
                "kontoinhaber": zone_einzel(d, "fn bankverbindung(&mut self)", 3),
            },
            "p35c_massnahme_art_reihenfolge": Value::Array(
                crate::regal::paar_objekt(&crate::regal::zone("tabellen.rs", "p35c_massnahme_einzelbetrag", "[", "]").unwrap())
                    .as_object()
                    .map(|m| m.keys().map(|k| json!(k.clone())).collect())
                    .unwrap_or_default(),
            ),
            "kap_felder_a": zone_liste("tabellen.rs", "pub(crate) const KAP_FELDER_A", "[", "]"),
            "kap_felder_b": zone_liste("tabellen.rs", "pub(crate) const KAP_FELDER_B", "[", "]"),
            "kap_null_grund": json!(crate::regal::text_der("tabellen.rs", "pub(crate) const KAP_NULL_GRUND", 0)),
            "hinweise": {
                "kist_konfession": json!(crate::regal::text_der("tabellen.rs", "Ihre Konfession laesst sich", 0)),
                "kist_konfession_partner":
                    json!(crate::regal::text_der("tabellen.rs", "Die Konfession Ihres Ehegatten", 0)),
            },
            "wertekodierung_andere_ohne_code": {
                "felder": zone_menge("tabellen.rs", "pub(crate) const WERTEKODIERUNG", "[", "]"),
                "ohne_code": Value::Array(
                    Konfession::ALLE
                        .into_iter()
                        .filter(|k| (WERTEKODIERUNG[0].code)(*k).is_none())
                        .map(|k| json!(k.als_str().replace('\u{f6}', "oe")))
                        .collect(),
                ),
            },
            "pflege": {
                "grad_kz": zone_einzel(d, "let grad_kz =", 0),
                "h_kz": zone_einzel(d, "let h_kz =", 0),
                "block": zone_liste("tabellen.rs", "pub(crate) const PFLEGE_KZ", "[", "]"),
            },
            "xsd_verify": {
                "max_depth": json!(crate::MAX_DEPTH),
                // Python haelt den Namensraum in Clark-Notation: `{uri}`.
                "xs_namespace": json!(format!("{{{}}}", einzel_der("xsd.rs", "const XS: &str"))),
                "datenart": {
                    "default": zone_liste("xsd.rs", "(\"E10-{jahr}.xsd\"", "", ""),
                    "routing": { vorwahl_e77: zone_liste("xsd.rs", "(\"E77-{jahr}.xsd\"", "", "") },
                },
            },
            "eric": {
                "extern_schema_muster": zone_einzel("xmllint.rs", "finde_datei(&format!(\"elster11_E10_", 0),
            },
            "elster_xml": {
                "ns_elster": json!(crate::NS_ELSTER),
                "ns_e10_format": zone_einzel(x, "let mut e10 = Knoten::neu", 0),
                "testmerker_eric": zone_einzel(x, "const TESTMERKER_ERIC", 0),
                "pflicht_default": zone_paare(x, "const PFLICHT_DEFAULT", "[", "]"),
                "instanz_nummer_felder": zone_menge(x, "const INSTANZ_NUMMER_FELDER", "[", "]"),
                "e10_ausschluss_datenart": zone_menge(x, "const E10_AUSSCHLUSS_DATENART", "[", "]"),
                "instanz_container_tiefer": zone_paare(x, "const INSTANZ_CONTAINER_TIEFER", "[", "]"),
                "absender_strasse_zusatz_kz": zone_einzel(x, "const ABSENDER_STRASSE_ZUSATZ_KZ", 0),
                "absender_herkunft": Value::Object(
                    crate::regal::wortliche(&crate::regal::zone(x, "const ABSENDER_HERKUNFT", "[", "]").unwrap())
                        .chunks(2)
                        .map(|c| (c[0].clone(), json!([c[1].clone()])))
                        .fold(Map::new(), |mut m, (k, v)| {
                            m.entry(k).or_insert(json!([])).as_array_mut().map(|a| {
                                if let Value::Array(x) = v { a.push(x[0].clone()) }
                            });
                            m
                        }),
                ),
                "eric_pflicht_trotz_optional": {
                    schluessel_e10_v: zone_menge("xsd.rs", "if !v.iter().any", "{", "}"),
                },
            },
            "eric_rc": {
                "rc": {
                    "RC_OK": json!(crate::RC_OK),
                    "RC_PLAUSIBILITAET": json!(crate::RC_PLAUSIBILITAET),
                    "RC_IO_SCHEMA_VALIDIERUNGSFEHLER": json!(crate::RC_IO_SCHEMA_VALIDIERUNGSFEHLER),
                    "RC_HERSTELLER_GESPERRT": json!(crate::RC_HERSTELLER_GESPERRT),
                    "RC_DATENARTVERSION_UNBEKANNT": json!(crate::RC_DATENARTVERSION_UNBEKANNT),
                    "RC_IO_UNERWARTETE_ELEMENTE": json!(crate::RC_IO_UNERWARTETE_ELEMENTE),
                },
                "klassen": klassen,
                "sonstig": {"rc": 7, "klasse": crate::klasse_name(crate::klassifiziere_rc(7))},
                "validiere": json!(crate::ERIC_VALIDIERE),
                "meldungen_max": json!(crate::VALIDIERE_MELDUNGEN_MAX),
                "nicht_geprueft_klassen": nicht_geprueft_klassen,
            },
        });
        r
    }

    /// Die Tabellen in der Form von `dump_kz_tabellen.tabellen()`.
    fn aus_tabellen() -> Value {
        json!({
            "konstante_kz": sortiert(KONSTANTE_KZ),
            "iban_transform_ziel_kz": sortiert(IBAN_TRANSFORM_ZIEL_KZ),
            "negation": objekt(NEGATION),
            "dokumentiert_aggregat": dokumentiert_aggregat(),
            "p23": {
                "betragsfelder": sortiert(P23_BETRAGSFELDER),
                "art_feld": P23_ART_FELD,
                "gewinn_kz": objekt(P23_GEWINN_KZ),
            },
            "verzweigung": verzweigung(VERZWEIGUNG),
            "partner_verzweigung": verzweigung(PARTNER_VERZWEIGUNG),
            "partner_instanz": objekt(PARTNER_INSTANZ),
            "pflege_kz": sortiert(PFLEGE_KZ),
            "wertekodierung": wertekodierung(),
        })
    }

    /// Jeder Eintrag, der in `tabellen.rs` und in der Fixture verschieden steht (Pfad + beide Werte).
    fn abweichungen(pfad: &str, rust: &Value, fix: &Value, aus: &mut Vec<String>) {
        match (rust, fix) {
            (Value::Object(r), Value::Object(f)) => {
                let schluessel = r.keys().chain(f.keys().filter(|k| !r.contains_key(*k)));
                for k in schluessel {
                    let weg = format!("{pfad}/{k}");
                    match (r.get(k), f.get(k)) {
                        (Some(a), Some(b)) => abweichungen(&weg, a, b, aus),
                        (a, b) => aus.push(format!("{weg}: Rust {a:?}, Fixture {b:?}")),
                    }
                }
            }
            _ if rust != fix => aus.push(format!("{pfad}: Rust {rust}, Fixture {fix}")),
            _ => {}
        }
    }

    #[test]
    fn tabellen_gleich_fixture() {
        let mut aus = Vec::new();
        abweichungen("", &aus_tabellen(), &fixture(), &mut aus);
        assert!(
            aus.is_empty(),
            "tabellen.rs weicht von rust/fixtures/kz_tabellen.json ab (est_mapping.py ist die Quelle: \
             erst `python3 tools/parity/dump_kz_tabellen.py`, dann tabellen.rs nachziehen):\n  {}",
            aus.join("\n  ")
        );
    }

    /// Gegenprobe gegen „beide Seiten leer“ oder einen Vergleich, der nie etwas findet (nur Fixture und
    /// Helfer, nicht die Tabellen: eine Abweichung dort macht diesen Test nicht zusätzlich rot).
    #[test]
    fn der_vergleich_sieht_einen_unterschied_und_die_fixture_ist_nicht_leer() {
        let fix = fixture();
        let p35c = &fix["verzweigung"]["p35c_massnahme_einzelbetrag"]["kz"];
        assert_eq!(
            p35c.as_object().map(Map::len),
            Some(9),
            "neun Sanierungsarten"
        );
        assert!(p35c.get("heizung").is_some());
        let nicht_leer = |v: &Value| v.as_object().is_some_and(|m| !m.is_empty());
        assert!(nicht_leer(&fix["negation"]) && nicht_leer(&fix["p23"]["gewinn_kz"]));

        let mut geaendert = fix.clone();
        geaendert["verzweigung"]["p35c_massnahme_einzelbetrag"]["kz"]["heizung"] =
            json!("E0241599");
        let mut aus = Vec::new();
        abweichungen("", &geaendert, &fix, &mut aus);
        assert!(aus.len() == 1 && aus[0].contains("heizung"), "{aus:?}");
    }
}
