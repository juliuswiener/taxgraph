//! Transform-Tabellen der Deklaration (`est_mapping.py:214-517`). Die Begruendung je Eintrag
//! steht im Python-Original an derselben Stelle; hier nur, was den Port betrifft.

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
    pub kz: &'static [(&'static str, &'static str)],
}

const RENTE_WERT: &[(&str, &str)] = &[
    ("gesetzliche_rente", "E1800301"),
    ("berufsstaendische_versorgung", "E1800301"),
    ("private_basisrente", "E1800301"),
    ("private_leibrente", "E1801601"),
    ("sonstige_leibrente", "E1803102"),
];
const RENTE_BEGINN: &[(&str, &str)] = &[
    ("gesetzliche_rente", "E1800501"),
    ("berufsstaendische_versorgung", "E1800501"),
    ("private_basisrente", "E1800501"),
    ("private_leibrente", "E1801701"),
    ("sonstige_leibrente", "E1803202"),
];
const VERAEUSSERUNG: &[(&str, &str)] = &[
    ("gewerbe", "E0801301"),
    ("selbstaendig", "E0804501"),
    ("land_forst", "E0901201"),
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
        kz: RENTE_WERT,
    },
    Verzweigung {
        feld: "rentner_renten_beginn_jahr",
        art_feld: "rentner_renten_art",
        kz: RENTE_BEGINN,
    },
    Verzweigung {
        feld: "rentner_veraeusserungsgewinn",
        art_feld: "rentner_veraeusserungs_betriebsart",
        kz: VERAEUSSERUNG,
    },
    Verzweigung {
        feld: "p35c_massnahme_einzelbetrag",
        art_feld: "p35c_massnahme_art",
        kz: &[
            ("waende", "E0241001"),
            ("dach", "E0241101"),
            ("geschossdecken", "E0241201"),
            ("fenster_tueren", "E0241301"),
            ("sommerlicher_waermeschutz", "E0241302"),
            ("lueftung", "E0241401"),
            ("heizung", "E0241501"),
            ("digital", "E0241601"),
            ("heizung_optimierung", "E0241701"),
        ],
    },
    Verzweigung {
        feld: "einkuenfte_gewinn",
        art_feld: "gewinn_betriebsart",
        kz: GEWINN,
    },
    Verzweigung {
        feld: "gewinn_bezeichnung",
        art_feld: "gewinn_betriebsart",
        kz: GEWINN_BEZEICHNUNG,
    },
    Verzweigung {
        feld: "basis_kv",
        art_feld: "versicherungsart",
        kz: BASIS_KV,
    },
    Verzweigung {
        feld: "basis_pv",
        art_feld: "versicherungsart",
        kz: BASIS_PV,
    },
];

/// Klasse g×f — Verzweigung Person B (`est_mapping.py:368-396`); Wert geht nach `person_b`.
pub(crate) const PARTNER_VERZWEIGUNG: &[Verzweigung] = &[
    Verzweigung {
        feld: "rentner_jahresrente_partner",
        art_feld: "rentner_renten_art_partner",
        kz: RENTE_WERT,
    },
    Verzweigung {
        feld: "rentner_renten_beginn_jahr_partner",
        art_feld: "rentner_renten_art_partner",
        kz: RENTE_BEGINN,
    },
    Verzweigung {
        feld: "einkuenfte_gewinn_partner",
        art_feld: "gewinn_betriebsart_partner",
        kz: GEWINN,
    },
    Verzweigung {
        feld: "gewinn_bezeichnung_partner",
        art_feld: "gewinn_betriebsart_partner",
        kz: GEWINN_BEZEICHNUNG,
    },
    Verzweigung {
        feld: "rentner_veraeusserungsgewinn_partner",
        art_feld: "rentner_veraeusserungs_betriebsart_partner",
        kz: VERAEUSSERUNG,
    },
    Verzweigung {
        feld: "basis_kv_partner",
        art_feld: "versicherungsart_partner",
        kz: BASIS_KV,
    },
    Verzweigung {
        feld: "basis_pv_partner",
        art_feld: "versicherungsart_partner",
        kz: BASIS_PV,
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
    pub code: &'static [(&'static str, &'static str)],
    pub hinweis_unbekannt: &'static str,
}

const KONFESSION_CODE: &[(&str, &str)] = &[
    ("keine", "11"),
    ("evangelisch", "02"),
    ("roemisch-katholisch", "03"),
];

pub(crate) const WERTEKODIERUNG: &[Wertekodierung] = &[
    Wertekodierung {
        feld: "kist_konfession",
        kz: "E0100402",
        code: KONFESSION_CODE,
        hinweis_unbekannt: "Ihre Konfession laesst sich nicht automatisch dem amtlichen \
Religionsschluessel zuordnen. Der amtliche Schluessel unterscheidet rund zwanzig einzelne \
Koerperschaften, viele davon regional (etwa juedische Gemeinden je nach Bundesland). Bitte tragen Sie \
die Konfession in Mein ELSTER nach oder waehlen Sie eine der angebotenen, falls sie zutrifft. Alles \
Uebrige Ihrer Erklaerung bleibt davon unberuehrt.",
    },
    Wertekodierung {
        feld: "kist_konfession_partner",
        kz: "E0101002",
        code: KONFESSION_CODE,
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
