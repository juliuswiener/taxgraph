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
    // Rust-eigen (B Option 1, 2026-10-06): die Antragszeile zu § 34 Abs. 3 fuer den Ehegatten. Python kennt sie nicht
    // (`partner_verzweigung` der Fixture fuehrt sie nicht); `tabellen_gleich_fixture` nimmt sie aus dem Vergleich und
    // `die_antragszeile_des_partners_traegt_die_kz_von_person_a` haelt ihren Inhalt fest.
    Verzweigung {
        feld: "p34_abs3_antragsbetrag_partner",
        art_feld: "rentner_veraeusserungs_betriebsart_partner",
        kz: ArtKz::Text(ANTRAG_ABS3),
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
    use crate::regal::ist_kz_form;
    use serde_json::{json, Map, Value};

    /// Ein Muster als Ja/Nein-Urteil ueber einen Wert.
    type Urteil = fn(&str) -> bool;

    /// Abbild der Kz-Tabellen aus `est_mapping.py`, eingefroren (der Erzeuger ist geloescht; Verlauf:
    /// `git show 2dd056a6:tools/parity/dump_kz_tabellen.py`). Die Tests laufen ohne `PARITY=1`:
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
        crate::regal::menge(crate::regal::zone(datei, anker, offnen, ende).unwrap_or_else(|e| panic!("{datei} {e}")))
    }

    fn zone_liste(datei: &str, anker: &str, offnen: &str, ende: &str) -> Value {
        crate::regal::folge(crate::regal::zone(datei, anker, offnen, ende).unwrap_or_else(|e| panic!("{datei} {e}")))
    }

    fn zone_paare(datei: &str, anker: &str, offnen: &str, ende: &str) -> Value {
        crate::regal::paar_objekt(crate::regal::zone(datei, anker, offnen, ende).unwrap_or_else(|e| panic!("{datei} {e}")))
    }

    /// Das n-te Literal einer einzeiligen Zone.
    fn zone_einzel(datei: &str, anker: &str, n: usize) -> Value {
        json!(crate::regal::einzel(crate::regal::zone(datei, anker, "", "").unwrap_or_else(|e| panic!("{datei} {e}")), n))
    }

    /// Das erste Literal ab der Ankerzeile bis `ende`: ein Text, der mit `\` ueber Zeilen laeuft.
    fn zone_text(datei: &str, anker: &str, ende: &str) -> Value {
        json!(crate::regal::einzel(
            crate::regal::zone_bis(datei, anker, ende).unwrap_or_else(|e| panic!("{datei} {e}")),
            0
        ))
    }

    /// `(Feld, &[(Kz, Text), …])` aus `ABSENDER_HERKUNFT`, in Lesereihenfolge: ein Literal ohne
    /// Kz-Form beginnt ein neues Feld, ein Kz-Literal gehoert mit dem naechsten Literal zum Feld davor.
    fn absender_herkunft(literale: Vec<String>) -> Value {
        let mut m: Map<String, Value> = Map::new();
        let mut feld = String::new();
        let mut es = literale.into_iter();
        while let Some(l) = es.next() {
            if ist_kz_form(&l) {
                let text = es.next().unwrap_or_default();
                if let Some(a) = m.entry(feld.clone()).or_insert_with(|| json!([])).as_array_mut() {
                    a.push(json!([l, text]));
                }
            } else {
                feld = l;
            }
        }
        Value::Object(m)
    }

    /// Das erste Literal der Zeile mit `anker`, als Text (nicht als `Value`).
    fn einzel_der(datei: &str, anker: &str) -> String {
        crate::regal::einzel(crate::regal::zone(datei, anker, "", "").unwrap_or_else(|e| panic!("{datei} {e}")), 0)
    }

    /// Die Ueberlebenden der Inventur: Regeln, die nicht in `tabellen.rs` stehen. Jede ist
    /// eine Zone in der Regal-Datei, kein handgeschriebener Wert.
    #[allow(clippy::too_many_lines, reason = "flache Tabelle: eine Zeile je Fixture-Schluessel")]
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
        // `let kz = if norm.starts_with("DE") { "E…" } else { "E…" };` — Vorwahl, Inland-Kz, Ausland-Kz.
        let weiche = crate::regal::wortliche(
            crate::regal::zone_bis(d, "let kz = if norm.starts_with(", ";").unwrap_or_else(|e| panic!("{d} {e}")),
        );
        // Genau diese drei: ein viertes Literal (neuer Zweig der Weiche) bliebe sonst ungelesen.
        assert_eq!(weiche.len(), 3, "iban_weiche: Vorwahl, Inland-Kz, Ausland-Kz erwartet, gefunden {weiche:?}");
        // Die Kz-Literale von `bankverbindung` in Lesereihenfolge: IBAN DE, IBAN Ausland, keine, Kontoinhaber.
        let bank_kz: Vec<String> = crate::regal::wortliche(
            crate::regal::zone(d, "fn bankverbindung(&mut self)", "{", "}").unwrap_or_else(|e| panic!("{d} {e}")),
        )
        .into_iter()
        .filter(|s| ist_kz_form(s))
        .collect();
        // Genau diese vier: ein fuenftes Kz in `bankverbindung` bliebe sonst ungelesen.
        assert_eq!(bank_kz.len(), 4, "bankverbindung: vier Kz-Literale erwartet, gefunden {bank_kz:?}");
        // Die Reihenfolge der Sanierungsarten (Python: `P35C_REIHENFOLGE`): Schluessel der ersten Zeilen
        // von `VERZWEIGUNG[p35c]`, in Lesereihenfolge. Eine `Map` sortiert, daher eine Liste.
        let art_reihenfolge: Vec<Value> = crate::regal::wortliche(
            crate::regal::zone_bis("tabellen.rs", "(\"waende\", \"", "]").unwrap_or_else(|e| panic!("tabellen.rs {e}")),
        )
        .chunks(2)
        .map(|paar| json!(paar[0]))
        .collect();
        let andere_felder: Vec<&str> = WERTEKODIERUNG.iter().map(|w| w.feld).collect();
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
                // Ein Alias ohne eigenes Literal: gemessen wird der Wert, nicht der Quelltext.
                "vereinigung": sortiert(crate::NULL_UNZULAESSIG_KZ_VEREINIGUNG),
            },
            "multiplikation": zone_liste("tabellen.rs", "pub(crate) const MULTIPLIKATION", "[", "]"),
            "p35a_summe_aus_posten": Value::Array(
                crate::regal::wortliche(crate::regal::zone(d, "const P35A_SUMME_AUS_POSTEN", "[", "]").unwrap())
                    .chunks(2)
                    .map(|c| json!([c[0], c[1]]))
                    .collect(),
            ),
            "iban_weiche": {
                "praefix": weiche[0],
                "inland": weiche[1],
                "ausland": weiche[2],
            },
            "bankverbindung": {
                "iban": [bank_kz[0], bank_kz[1]],
                "keine_bankverbindung": bank_kz[2],
                "kontoinhaber": bank_kz[3],
            },
            "p35c_massnahme_art_reihenfolge": art_reihenfolge,
            "kap_felder_a": zone_liste("tabellen.rs", "pub(crate) const KAP_FELDER_A", "[", "]"),
            "kap_felder_b": zone_liste("tabellen.rs", "pub(crate) const KAP_FELDER_B", "[", "]"),
            "kap_null_grund": zone_text("tabellen.rs", "pub(crate) const KAP_NULL_GRUND", ";"),
            "hinweise": {
                "kist_konfession": zone_text("tabellen.rs", "Ihre Konfession laesst sich", "\","),
                "kist_konfession_partner": zone_text("tabellen.rs", "Die Konfession Ihres Ehegatten", "\","),
            },
            "wertekodierung_andere_ohne_code": {
                "felder": sortiert(&andere_felder),
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
                "ns_e10_format": zone_einzel(x, "elstererklaerung/est/e10/v{vz}", 0),
                "testmerker_eric": zone_einzel(x, "const TESTMERKER_ERIC", 0),
                "pflicht_default": zone_paare(x, "const PFLICHT_DEFAULT", "[", "]"),
                "instanz_nummer_felder": zone_menge(x, "const INSTANZ_NUMMER_FELDER", "[", "]"),
                "e10_ausschluss_datenart": zone_menge(x, "const E10_AUSSCHLUSS_DATENART", "[", "]"),
                "instanz_container_tiefer": zone_paare(x, "const INSTANZ_CONTAINER_TIEFER", "[", "]"),
                "absender_strasse_zusatz_kz": zone_einzel(x, "const ABSENDER_STRASSE_ZUSATZ_KZ", 0),
                "absender_herkunft": absender_herkunft(crate::regal::wortliche(
                    crate::regal::zone(x, "const ABSENDER_HERKUNFT", "[", "]").unwrap_or_else(|e| panic!("{x} {e}")),
                )),
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

    /// Die Tabellen in der Form der Fixture.
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
            "pflichtfelder": Value::Array(
                PFLICHTFELDER
                    .iter()
                    .map(|(b, v, felder)| json!({"bedingung": b.als_str(), "eric_version": v, "felder": sortiert(felder)}))
                    .collect(),
            ),
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

    /// Die Fixture ohne `proben`: die vergleicht `proben_gleich_fixture` getrennt, denn Rust und
    /// Python haben verschiedene Regex-Engines — vergleichbar ist nur das Urteil ueber dieselben Werte.
    fn fixture_ohne_proben() -> Value {
        let mut f = fixture();
        if let Some(o) = f.as_object_mut() {
            o.remove("proben");
        }
        f
    }

    /// Tabellen aus `tabellen.rs` und Regeln aus den anderen Dateien, in der Form der ganzen Fixture.
    fn aus_allem() -> Value {
        let mut alles = aus_tabellen();
        if let (Some(a), Value::Object(r)) = (alles.as_object_mut(), aus_regal()) {
            a.extend(r);
        }
        alles
    }

    #[test]
    fn proben_gleich_fixture() {
        let fix = fixture();
        let regeln: [(&str, Urteil); 5] = [
            ("regex/iban", crate::deklaration::iban_muster),
            ("regex/instanz", |s| crate::instanz::instanz_re().is_some_and(|r| r.is_match(s))),
            ("regex/steuernummer", crate::xml::stnr_muster),
            ("xsd_verify/kz_pattern", crate::xsd::ist_kz),
            ("xsd_verify/ja_typ_pattern", crate::ist_ja_typ),
        ];
        let mut aus: Vec<String> = Vec::new();
        for (pfad, rust) in regeln {
            aus.extend(crate::regal::proben(pfad, &fix, rust));
        }
        let mut in_fixture: Vec<&str> = fix["proben"]
            .as_object()
            .map(|m| m.keys().map(String::as_str).collect())
            .unwrap_or_default();
        let mut im_regal: Vec<&str> = crate::regal::PROBE_PAARE.iter().map(|(n, _)| *n).collect();
        let mut hier: Vec<&str> = regeln.iter().map(|(n, _)| *n).collect();
        in_fixture.sort_unstable();
        im_regal.sort_unstable();
        hier.sort_unstable();
        assert_eq!(in_fixture, im_regal, "Proben-Pfade: Fixture gegen PROBE_PAARE");
        assert_eq!(im_regal, hier, "Proben-Pfade: PROBE_PAARE gegen diesen Test");
        assert!(aus.is_empty(), "Rust urteilt anders als Python ueber dieselben Werte:\n  {}", aus.join("\n  "));
    }

    /// Eine neue Top-Level-Konstante oder -Funktion in einer als vollstaendig gefuehrten Datei, die
    /// niemandem zugeordnet ist, macht den Standardlauf rot (und umgekehrt ein Regal-Eintrag, dessen
    /// Konstante es nicht mehr gibt).
    #[test]
    fn jede_tabelle_ist_eingetragen() {
        let (fehlt, veraltet) = crate::regal::regal_fehler();
        assert!(
            fehlt.is_empty(),
            "ohne Regal-Eintrag in regal.rs (neue Tabelle? Die Fixture ist eingefroren; \
             eine Rust-eigene Tabelle steht hier als Ausnahme mit Grund):\n  {}",
            fehlt.join("\n  ")
        );
        assert!(
            veraltet.is_empty(),
            "Regal-Eintrag ohne Gegenstueck im Quelltext (umbenannt oder gestrichen):\n  {}",
            veraltet.join("\n  ")
        );
    }

    /// Jedes exakte Kz-Literal im Produktionstext von `rust/elster/src` hat einen Grund: `aus_regal()`
    /// liest es aus dem Quelltext und vergleicht es mit der Fixture, es steht in einer Tabelle von
    /// `tabellen.rs`, oder das Regal fuehrt seine Funktion als `Verhalten` mit benannten Tests
    /// (`kz_wache.rs`). Eine neue Kz in einem Funktionskoerper, die nichts davon liest, macht den
    /// Standardlauf rot — die Klasse von Nr 59.
    #[test]
    fn jedes_kz_literal_ist_von_der_fixture_gelesen() {
        let (_, gelesen) = crate::regal::aufzeichnen(aus_allem);
        assert!(!gelesen.is_empty(), "aus_regal() hat nichts aus dem Quelltext gelesen");
        let befund = crate::kz_wache::kz_literal_befund(&gelesen);
        // Ohne Funde in den Dateien mit Kz waere „kein Fehler“ nichts wert: die Abtastung sah dann nichts.
        for datei in ["tabellen.rs", "kz_format.rs", "deklaration.rs", "xml.rs"] {
            assert!(
                befund.je_datei.iter().any(|(d, n)| d == datei && *n > 0),
                "kz_wache sieht in {datei} kein Kz-Literal: {:?}",
                befund.je_datei
            );
        }
        assert!(
            befund.fehler.is_empty(),
            "Kz-Literal ohne Leser. Entweder in eine Tabelle von tabellen.rs, oder die Zone \
             in aus_regal() lesen, oder — wenn nur Verhaltenstests \
             sie decken — `Zuordnung::Verhalten` im Regal eintragen:\n  {}",
            befund.fehler.join("\n  ")
        );
    }

    /// Jeder Pfad des Regals steht in der Fixture, und jeder Top-Level-Schluessel der Fixture gehoert
    /// einem Pfad des Regals: kein Schluessel ohne Tabelle, keine Tabelle ohne Schluessel.
    #[test]
    fn kein_schluessel_ohne_tabelle() {
        let fix = fixture();
        let pfade = crate::regal::gemeldete_schluessel();
        let fehlt_in_fixture: Vec<&str> = pfade
            .iter()
            .copied()
            .filter(|p| {
                crate::regal::hole(&fix, p).is_none() && fix.get("proben").and_then(|x| x.get(*p)).is_none()
            })
            .collect();
        let ohne_regal: Vec<&str> = fix
            .as_object()
            .map(|m| {
                m.keys()
                    .map(String::as_str)
                    .filter(|k| *k != "proben")
                    .filter(|k| !pfade.iter().any(|p| p == k || p.starts_with(&format!("{k}/"))))
                    .collect()
            })
            .unwrap_or_default();
        assert!(fehlt_in_fixture.is_empty(), "Regal-Pfad ohne Schluessel in der Fixture: {fehlt_in_fixture:?}");
        assert!(ohne_regal.is_empty(), "Fixture-Schluessel ohne Regal-Eintrag: {ohne_regal:?}");
    }

    /// Die Sperre gegen eine verschobene Klassifikation (Weg B voll, Stufe 3, W10 d). Die Fixture haelt,
    /// welche Kz Abzug sind (aufrunden) und welche Einnahme (abrunden), die Kz-Paare der Verzweigungen
    /// (§ 35c) und die Transform-Quellen. Wandert ein Kz mit lebendem Feld aus der Abzugsliste
    /// (Sonde `QX1b`) oder aendert sich eine Zeile der Verzweigung (`QP1` bis `QP3`), wird NUR dieser Test rot:
    /// der Proptest `rundung_zugunsten_je_bindung` in `tests/eigenschaften.rs` leitet seine Erwartung aus
    /// `kz_format` ab, derselben Tabelle, die er pruefen soll. Die Fixture ist deshalb eine Sperre, keine
    /// Python-Antwort, die bei Abweichung neu erzeugt wird: eine Abweichung ist ein Entscheid mit Grund
    /// im Commit. Sie steht in `RUST_EIGENE_ZEILEN` oder `RUST_EIGENE_KZ` und in der Abweichungsliste
    /// (README `rust/fixtures/README.md`, Regel 2); die Fixture bleibt unberuehrt.
    #[test]
    fn tabellen_gleich_fixture() {
        let mut rust = aus_allem();
        let fix = fixture_ohne_proben();
        // Die Rust-eigenen Zeilen (README `rust/fixtures/README.md`, Abweichung Nr. 23) stehen nur in Rust: sie
        // fallen aus dem Vergleich, und die Fixture darf sie nicht fuehren (sonst ist die Abweichung keine mehr).
        for (tabelle, feld) in RUST_EIGENE_ZEILEN {
            let weg = rust.get_mut(tabelle).and_then(Value::as_object_mut);
            assert!(weg.is_some_and(|m| m.remove(feld).is_some()), "{tabelle}/{feld} fehlt in tabellen.rs");
            assert!(fix[tabelle].get(feld).is_none(), "Python kennt {tabelle}/{feld} jetzt: Eintrag streichen");
        }
        // Dasselbe fuer ein Kz, das nur in Rust steht (Abweichung Nr. 31): genau ein Treffer je Liste in Rust, keiner in der Fixture.
        for (pfad, kz) in RUST_EIGENE_KZ {
            let liste = rust.pointer_mut(pfad).and_then(Value::as_array_mut);
            let entfernt = liste.map(|l| {
                let vorher = l.len();
                l.retain(|x| x != kz);
                vorher - l.len()
            });
            assert_eq!(entfernt, Some(1), "{kz} steht nicht genau einmal in {pfad} von tabellen.rs");
            let in_fixture = fix.pointer(pfad).and_then(Value::as_array).is_some_and(|l| l.iter().any(|x| x == kz));
            assert!(!in_fixture, "Python kennt {kz} in {pfad} jetzt: Eintrag streichen");
        }
        // Und fuer ein Schluessel-Wert-Paar, das nur in Rust steht (Abweichung Nr. 49): genau dieser Wert in Rust, kein Schluessel in der Fixture.
        for (pfad, schluessel, wert) in RUST_EIGENE_PAARE {
            let paar = rust.pointer_mut(pfad).and_then(Value::as_object_mut).and_then(|m| m.remove(schluessel));
            assert_eq!(paar, Some(json!(wert)), "{schluessel} steht nicht mit `{wert}` in {pfad} von tabellen.rs");
            let in_fixture = fix.pointer(pfad).and_then(Value::as_object).is_some_and(|m| m.contains_key(schluessel));
            assert!(!in_fixture, "Python kennt {schluessel} in {pfad} jetzt: Eintrag streichen");
        }
        let mut aus = Vec::new();
        abweichungen("", &rust, &fix, &mut aus);
        assert!(
            aus.is_empty(),
            "tabellen.rs weicht von rust/fixtures/kz_tabellen.json ab (Stand von est_mapping.py, \
             eingefroren; eine gewollte Abweichung steht als Eintrag mit Grund in der Abweichungsliste, \
             `rust/fixtures/README.md`, nicht als Aenderung der Fixture):\n  {}",
            aus.join("\n  ")
        );
    }

    /// Zeilen der Tabellen, die es nur in Rust gibt: `(Tabelle der Fixture, Feld)`. Jede steht mit Grund im README
    /// (`rust/fixtures/README.md`) und hat einen eigenen Test.
    const RUST_EIGENE_ZEILEN: [(&str, &str); 1] =
        [("partner_verzweigung", "p34_abs3_antragsbetrag_partner")];

    /// Kz, die es nur in Rust gibt: `(JSON-Pfad der Liste, Kz)`. E0108701 (Spenden an Parteien, Zeile 7 der Anlage
    /// Sonderausgaben) steht in den vier Listen, die `kz_format.rs` fuehrt; Python (`est_mapping.py`) kennt das Kz nicht.
    /// Grund und Test: README `rust/fixtures/README.md`, Abweichung Nr. 31 (`bescheid/tests/parteispenden_einreichung_hermetisch.rs`).
    /// E0108801 (Spenden an Waehlervereinigungen, Zeile 8, gleicher Schema-Typ wie Zeile 7) steht in denselben vier Listen. Grund und
    /// Test: Abweichung Nr. 44 (`bescheid/tests/waehlervereinigungen_einreichung_hermetisch.rs`).
    /// E0600920 (abgezogene auslaendische Steuer, § 34c Abs. 2, Anlage AUS Zeile 10) steht nur in `ABZUGS_KZ`; sein Schema-Typ
    /// erlaubt die 0. Grund und Test: Abweichung Nr. 41 (`bescheid/tests/p34c_abzug_einreichung_hermetisch.rs`).
    /// E0205406 (Unfallkosten, Zeile "Sonstiges" der Anlage N, `Weitere_Wk/Sonst`) steht nur in `ABZUGS_KZ`. Grund und Test:
    /// Abweichung Nr. 48 (`bescheid/tests/unfallkosten_weitere_wk_xml.rs`).
    const RUST_EIGENE_KZ: [(&str, &str); 10] = [
        ("/abzugs_kz", "E0108701"),
        ("/abzugs_kz", "E0108801"),
        ("/abzugs_kz", "E0600920"),
        ("/abzugs_kz", "E0205406"),
        ("/null_unzulaessig/je_vz/2024", "E0108701"),
        ("/null_unzulaessig/je_vz/2024", "E0108801"),
        ("/null_unzulaessig/je_vz/2025", "E0108701"),
        ("/null_unzulaessig/je_vz/2025", "E0108801"),
        ("/null_unzulaessig/vereinigung", "E0108701"),
        ("/null_unzulaessig/vereinigung", "E0108801"),
    ];

    /// Paare `(JSON-Pfad der Tabelle, Schluessel, Wert)`, die es nur in Rust gibt. `weitere_wk` -> `Sonst` in
    /// `INSTANZ_CONTAINER_TIEFER` (`xml.rs`): die zweite Zeile "Sonstiges" der Anlage N, der Abzug nach § 34c Abs. 2 `EStG`.
    /// Python kennt die Zeile nicht. Grund und Test: Abweichung Nr. 49 (`bescheid/tests/p34c_abzug_weitere_wk_xml.rs`, Test
    /// `das_xml_traegt_beide_zeilen_unter_einem_weitere_wk_und_das_schema_nimmt_es`).
    const RUST_EIGENE_PAARE: [(&str, &str, &str); 1] =
        [("/elster_xml/instanz_container_tiefer", "weitere_wk", "Sonst")];

    /// Abweichung Nr. 23: die Antragszeile zu § 34 Abs. 3 fuer den Ehegatten ist Person As Zeile mit der Weiche des
    /// Partners: dieselben drei Kz je Betriebsart (E0801602 / E0805003 / E0901704), gelenkt von
    /// `rentner_veraeusserungs_betriebsart_partner`. Ein vertauschtes oder fehlendes Kz faellt hier auf, nicht erst im Schema.
    #[test]
    fn die_antragszeile_des_partners_traegt_die_kz_von_person_a() {
        let rust = verzweigung(PARTNER_VERZWEIGUNG);
        let partner = &rust["p34_abs3_antragsbetrag_partner"];
        let person_a = &fixture()["verzweigung"]["p34_abs3_antragsbetrag"];
        assert_eq!(partner["art_feld"], "rentner_veraeusserungs_betriebsart_partner");
        assert_eq!(partner["kz"], person_a["kz"]);
        assert_eq!(
            partner["kz"],
            json!({"gewerbe": "E0801602", "selbstaendig": "E0805003", "land_forst": "E0901704"})
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
