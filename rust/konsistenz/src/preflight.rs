//! Preflight: sammelt alle Konsistenzprüfungen vor der Abgabe und bildet die Ampel
//! (`produkt/konsistenz/preflight.py`).
//!
//! Die Plausibilitätsprüfungen (Betrag ↔ Bezugsgröße) MELDEN, sie sperren nicht; ohne Bezugsgröße
//! bleiben sie still; ihre Schwellen sind Größenordnungen gegen Tippfehler um Faktor 10
//! (`preflight.py:40-57`).
//!
//! Traverser-Naht: welche angekündigten Instanzen fehlen, rechnet
//! [`interview::fehlende_instanzen`] (`traverser.py:191-234`); hier steht nur der Klartext dazu.
//! Die volle Bindung (`TR.lade_bindung()`) ist der [`Graph`].
use std::collections::HashSet;
use std::hash::BuildHasher;

use domain::PyWert;
use interview::Graph;

use crate::flag::{flag_widersprueche, FlagWiderspruch};
use crate::lesung::{lies, Felder};
use crate::nicht_gerechnet::{nicht_gerechnete_angaben, NichtGerechnet};
use crate::partner::{alleinerziehend_mit_zusammen, partner_ohne_zusammen, PartnerWiderspruch};
use crate::pauschalen::{pauschal_hinweise, PauschalHinweis};
use crate::zahl::{als_text, eur, ganzzahl, leer_nach_strip};

/// Die Betragsfelder, die der Ring liest (`produkt/haut/api_constants.py:748`,
/// `RING_BETRAGSFELDER`). Dieselbe Menge wie die Klasse-C-Sperre in `_feste_zahl`, damit Sperre
/// und Hinweis nicht auseinanderlaufen.
pub const RING_BETRAGSFELDER: [&str; 120] = [
    "afa_jahresbetrag",
    "agb_aufwendungen",
    "am_anschaffung_monat",
    "am_anschaffungskosten",
    "arbeitsmittel_nutzungsdauer",
    "basis_kv",
    "basis_kv_partner",
    "basis_pv",
    "basis_pv_partner",
    "behinderungsbedingte_aufwendungen",
    "behinderungsbedingte_aufwendungen_partner",
    "berufsausbildung_aufwendungen",
    "betriebseinnahmen",
    "bruttoarbeitslohn",
    "bruttoarbeitslohn_partner",
    "dba_auslaendische_einkuenfte",
    "dba_gezahlte_auslaendische_steuer",
    "dhf_monate",
    "dhf_unterkunftskosten_monat",
    "einkuenfte_gewinn",
    "einkuenfte_gewinn_partner",
    "ep_unfallkosten",
    "fam_anzahl_kinder",
    "fam_monate_ohne_voraussetzung",
    "geburtsjahr",
    "geburtsjahr_partner",
    "gewinnanteil",
    "gewinnanteil_partner",
    "gewst_hebesatz",
    "gewst_hebesatz_partner",
    "gewst_messbetrag",
    "gewst_messbetrag_partner",
    "hh_dienstleistungen",
    "hh_handwerker_arbeitskosten",
    "hh_minijob_aufwendungen",
    "kap_gewinn_aktien",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige",
    "kap_gewinn_sonstige_partner",
    "kap_kapitalertraege",
    "kap_kapitalertraege_partner",
    "kap_q_auslaendische_steuer",
    "kap_verlust_aktien",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige",
    "kap_verlust_sonstige_partner",
    "kist_erstattet",
    "kist_gezahlt",
    "p22_nr3_einkuenfte",
    "p32b_progressionseinkuenfte",
    "p33a_andere_einkuenfte_bezuege",
    "p33a_ausbildung_anzahl_kinder",
    "p33a_unterhalt_aufwendungen",
    "p33a_unterhalt_kv_pv",
    "p35c_energieberater_aufwendungen",
    "p35c_sanierungsaufwendungen",
    "p36_kapitalertragsteuer",
    "p36_kapitalertragsteuer_kist",
    "p36_kapitalertragsteuer_solz",
    "p36_lohnsteuer",
    "p36_vorauszahlungen",
    "pv_anzahl_einheiten",
    "pv_bruttoleistung_kwp",
    "pv_einnahmen",
    "realsplitting_empfaenger_kv_krankengeld",
    "realsplitting_empfaenger_kv_pv",
    "realsplitting_unterhaltsleistungen",
    "rentner_alter_bei_rentenbeginn_partner",
    "rentner_grad_der_behinderung",
    "rentner_grad_der_behinderung_partner",
    "rentner_jahresrente_partner",
    "rentner_pflegegrad",
    "rentner_renten_beginn_jahr_partner",
    "rentner_rentenfreibetrag_partner",
    "rentner_veraeusserungsgewinn",
    "rentner_veraeusserungsgewinn_partner",
    "sonstige_betriebsausgaben",
    "spenden_betrag",
    "tage_24h",
    "tage_an_abreise",
    "tage_ueber_8h_eintaegig",
    "uebernachtung_kosten_monat",
    "uebernachtung_monate",
    "uebernachtung_monate_bisher",
    "verguetung_darlehen",
    "verguetung_darlehen_partner",
    "verguetung_taetigkeit",
    "verguetung_taetigkeit_partner",
    "verguetung_ueberlassung",
    "verguetung_ueberlassung_partner",
    "verlustvortrag_bestand",
    "versorgung_alter_bei_beginn",
    "versorgung_beginn_jahr",
    "versorgung_bemessungsgrundlage",
    "versorgung_jahresrente",
    "vor_ag_anteil_rv",
    "vor_ag_anteil_rv_partner",
    "vor_an_anteil_rv",
    "vor_an_anteil_rv_partner",
    "vor_rv_ausserhalb_lstb",
    "vor_rv_ausserhalb_lstb_partner",
    "vorsorge_arbeitslosenversicherung",
    "vorsorge_arbeitslosenversicherung_partner",
    "vorsorge_erwerbsunfaehigkeit",
    "vorsorge_erwerbsunfaehigkeit_partner",
    "vorsorge_rv_alt_mit_ueberschuss",
    "vorsorge_rv_alt_mit_ueberschuss_partner",
    "vorsorge_rv_alt_ohne_ueberschuss",
    "vorsorge_rv_alt_ohne_ueberschuss_partner",
    "vorsorge_unfall_haftpflicht",
    "vorsorge_unfall_haftpflicht_partner",
    "vpf_abendessen_gestellt_anzahl",
    "vpf_fruehstuecke_gestellt_anzahl",
    "vpf_mahlzeiten_gezahltes_entgelt",
    "vpf_mittagessen_gestellt_anzahl",
    "vpf_monate_am_ort",
    "vpf_steuerfreie_erstattung_betrag",
    "vpf_tage_24h_nach_drei_monaten",
    "vpf_tage_an_abreise_nach_drei_monaten",
    "vpf_tage_ueber_8h_nach_drei_monaten",
];

/// Obergrenze des Bruttolohns (Cent), bis zu der [`kist_ueber_brutto_anteil`] beweisbar dasselbe
/// ergibt wie Pythons `kist > brutto * 0.30`: `floor(2^50 · 10 / 3)`.
pub const BRUTTO_FLOAT_EXAKT_MAX: i64 = 3_752_999_689_475_413;

/// § 10 Abs. 1 Nr. 9 S. 1 `EStG`: „30 Prozent des Entgelts, höchstens 5 000 Euro"; Schwelle =
/// zehnfaches Schulgeld, ab dem der Höchstbetrag erreicht ist. Python:
/// `int(5_000_00 / 0.30) * 10` = `int(1666666.6666666667) * 10` = 16 666 660 — gleich der exakten
/// Ganzzahlrechnung `500_000 * 10 / 3 * 10` (Nachkommateil 0,67 liegt weit von einer Ganzzahl).
/// Der Satz 30 % ist in `params/{2024,2025,2026}/schulgeld_p10.yaml` über alle VZ gleich.
pub const SCHULGELD_SCHWELLE_CENT: i64 = 16_666_660;

/// Kirchensteuer ist ein Zuschlag von 8 bzw. 9 v. H. auf die `ESt`, die `ESt` ist kleiner als der
/// Lohn; normal also unter 10 % des Lohns, Faktor 3 Luft (`preflight.py:59-64`). Python:
/// `kist > brutto * 0.30` in Float.
///
/// Ganzzahlig `kist * 10 > brutto * 3`. Gleich zu Python für `1 ≤ brutto ≤`
/// [`BRUTTO_FLOAT_EXAKT_MAX`], jedes `kist`:
///
/// 1. Für ganzzahliges `kist` und endliches `f` gilt `kist > f ⇔ kist > floor(f)`; ebenso
///    `kist·10 > 3b ⇔ kist > floor(3b/10)`. Zu zeigen: `floor(fl(b · fl(0.3))) = floor(3b/10)`.
/// 2. `fl(0.3) = 0.3 − 0.2·2^-54` (Mantisse 5404319552844595 = `0.3·2^54 − 0.2`); `float(b)` ist
///    exakt (b < 2^53). Exaktes Produkt `x = K + r/10 − ε`, `K = floor(3b/10)`, `r ∈ 0..=9`,
///    `ε = (b/5)·2^-54 < 1/24`.
/// 3. `r = 0`: der halbe Abstand unter `K` ist ≥ `K·2^-54 = (3b/10)·2^-54 > ε`, also `fl(x) = K`.
/// 4. `r ≥ 1`: `x ∈ (K, K+1)`, beide Nachbarn darstellbar, Rundung monoton ⇒ `fl(x) ∈ [K, K+1]`.
///    Unter 2^50 ist der halbe Abstand ≤ 1/16 < 1/10 ≤ `K+1−x` ⇒ `fl(x) < K+1` ⇒ Boden `K`.
///
/// Brute force 1..=10^10 bestätigt, erste gemessene Abweichung bei 10^16+3.
/// ponytail: oberhalb der Grenze rechnet Rust exakt, Python mit Float-Fehler — ein Lohn über
/// 37 Billionen Euro. Upgrade-Pfad: IEEE-Produkt mit i128 nachbilden, falls je nötig.
///
/// ```
/// use konsistenz::kist_ueber_brutto_anteil;
/// assert!(!kist_ueber_brutto_anteil(3, 10));
/// assert!(kist_ueber_brutto_anteil(4, 10));
/// ```
#[must_use]
pub fn kist_ueber_brutto_anteil(kist: i64, brutto: i64) -> bool {
    i128::from(kist) * 10 > i128::from(brutto) * 3
}

/// Einbehaltene und gezahlte `KiSt` dürfen auseinanderliegen, aber nicht um eine Größenordnung.
const KIST_ABGLEICH_FAKTOR: i128 = 10;

/// Ein Plausibilitäts-Widerspruch. `bezug` fehlt bei Schulgeld, IBAN und Konfession, wie in
/// Python (dort fehlt der Schlüssel).
#[derive(Debug, Clone, PartialEq)]
pub struct PlausiWiderspruch {
    pub feld_id: String,
    /// Der geprüfte Wert (Betrag, IBAN, Anzahl — bei der Konfession `null`).
    pub wert: PyWert,
    pub bezug: Option<i64>,
    pub grund: String,
}

/// Ein genannter, aber noch nicht bestätigter Ring-Betrag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VorlaeufigerBetrag {
    pub feld_id: &'static str,
    pub wert: i64,
    pub hinweis: String,
}

/// Preflight-Ampel (`preflight.py:402-410`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ampel {
    /// Harte Widersprüche.
    Rot,
    /// Nur weiche Hinweise: die Erklärung ist in Ordnung, die angezeigte Zahl bildet aber nicht
    /// alles ab.
    Gelb,
    /// Nichts zu wissen.
    Gruen,
}

impl Ampel {
    /// Der Wire-Wert (`"RED"`/`"AMBER"`/`"GREEN"`).
    ///
    /// ```
    /// assert_eq!(konsistenz::Ampel::Gelb.als_str(), "AMBER");
    /// ```
    #[must_use]
    pub const fn als_str(self) -> &'static str {
        match self {
            Self::Rot => "RED",
            Self::Gelb => "AMBER",
            Self::Gruen => "GREEN",
        }
    }
}

/// Ergebnis von [`preflight`] (`preflight.py:412-421`).
#[derive(Debug, Clone, PartialEq)]
pub struct PreflightErgebnis {
    pub widersprueche_flag: Vec<FlagWiderspruch>,
    pub widersprueche_partner: Vec<PartnerWiderspruch>,
    pub widersprueche_alleinerziehend: Vec<PartnerWiderspruch>,
    pub widersprueche_plausibilitaet: Vec<PlausiWiderspruch>,
    pub hinweise_pauschalen: Vec<PauschalHinweis>,
    pub hinweise_nicht_gerechnet: Vec<NichtGerechnet>,
    pub hinweise_betrag_vorlaeufig: Vec<VorlaeufigerBetrag>,
    pub status: Ampel,
}

/// `_bestaetigter_betrag` (`preflight.py:89-98`): bestätigter Betrag > 0, sonst `None`. `true`
/// ist kein Betrag. PARITÄT: Floats zählen nicht (Wertebereich, s. Crate-Doku).
fn bestaetigter_betrag(felder: &Felder, feld_id: &str) -> Option<i64> {
    lies(felder, feld_id)
        .bestaetigt()
        .and_then(ganzzahl)
        .filter(|n| *n > 0)
}

/// `_frage_kurz` (`preflight.py:128-133`): Fragetext bis einschließlich des ersten „?".
fn frage_kurz(feld_id: &str, graph: &Graph<'_>) -> String {
    let text = graph
        .alle()
        .get(feld_id)
        .and_then(|b| b.fragetext_laie.as_deref())
        .unwrap_or("");
    match text.split_once('?') {
        Some((kopf, _)) => format!("{kopf}?"),
        None => text.to_owned(),
    }
}

/// `_aufzaehlung` (`preflight.py:118-125`): „Kind 3" / „Kind 2 und Kind 3" / „Kind 2, Kind 3 und
/// Kind 4". Das Etikett bleibt Singular — einen Plural zu raten ginge schief.
fn aufzaehlung(etikett: &str, nummern: &[u16]) -> String {
    let teile: Vec<String> = nummern.iter().map(|n| format!("{etikett} {n}")).collect();
    match teile.split_last() {
        None => String::new(),
        Some((letztes, [])) => letztes.clone(),
        Some((letztes, davor)) => format!("{} und {letztes}", davor.join(", ")),
    }
}

/// „3 Kinder angegeben, 2 Namen eingetragen" als Klartext (`preflight.py:136-166`). Die Frage
/// kommt im Fragebogen nicht wieder — ohne Meldung ginge die dritte Instanz lautlos verloren.
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// assert!(konsistenz::unvollstaendige_instanzen(&konsistenz::Felder::new(), &g).is_empty());
/// ```
#[must_use]
pub fn unvollstaendige_instanzen(felder: &Felder, graph: &Graph<'_>) -> Vec<PlausiWiderspruch> {
    interview::fehlende_instanzen(felder, graph.alle(), graph)
        .into_iter()
        .map(|l| {
            // Der Satz braucht eine Luecke: fehlend nicht leer, vorhanden + fehlend = anzahl.
            debug_assert!(
                !l.fehlend.is_empty() && l.vorhanden.len() + l.fehlend.len() == usize::from(l.anzahl)
            );
            // Python `len()`, also `int` (`preflight.py:157`). Höchstens `anzahl` (u16) Einträge:
            // der Ersatzwert ist unerreichbar.
            let vorhanden = i64::try_from(l.vorhanden.len()).unwrap_or(i64::MAX);
            let fehlt = aufzaehlung(l.etikett, &l.fehlend);
            PlausiWiderspruch {
                feld_id: l.feld_id.to_owned(),
                wert: PyWert::Ganz(vorhanden),
                bezug: Some(i64::from(l.anzahl)),
                grund: format!(
                    "Angegeben hast du {}, ausgefüllt sind {vorhanden}: auf die Frage »{}« fehlt die \
                     Antwort für {fehlt}. Diese Frage kommt im Fragebogen nicht noch einmal — ohne \
                     die Antwort steht {fehlt} nicht in deiner Steuererklärung. Bitte trage die \
                     fehlende Angabe nach, oder gib die Zahl an, die wirklich in die Erklärung soll.",
                    l.anzahl,
                    frage_kurz(l.feld_id, graph)
                ),
            }
        })
        .collect()
}

/// Genannte, nicht bestätigte Ring-Beträge (`preflight.py:169-219`). `/ergebnis` sperrt dann
/// die Zahl; diese Meldung sagt WARUM und WELCHE Angabe. Nur `typ: cent` ohne Instanzgruppe —
/// der Satz nennt einen Euro-Betrag.
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// assert!(konsistenz::vorlaeufige_ring_betraege(&konsistenz::Felder::new(), &g).is_empty());
/// ```
#[must_use]
pub fn vorlaeufige_ring_betraege(felder: &Felder, graph: &Graph<'_>) -> Vec<VorlaeufigerBetrag> {
    let mut treffer = Vec::new();
    for feld_id in RING_BETRAGSFELDER {
        let Some(eintrag) = felder.get(feld_id) else {
            continue;
        };
        if eintrag.zustand == domain::Zustand::Bestaetigt {
            continue;
        }
        let Some(b) = graph.alle().get(feld_id) else {
            continue;
        };
        if b.typ != domain::Feldtyp::Cent
            || b.instanz_gruppe.as_deref().is_some_and(|g| !g.is_empty())
        {
            continue;
        }
        // PARITÄT: Floats zählen nicht (Wertebereich, s. Crate-Doku).
        let Some(wert) = ganzzahl(&eintrag.wert).filter(|n| *n > 0) else {
            continue;
        };
        // Ohne fragetext_laie fällt der Ortshinweis weg — ein Füllwort läse sich als Frage.
        let frage = frage_kurz(feld_id, graph);
        let ort = if frage.is_empty() {
            String::new()
        } else {
            format!(" bei der Frage »{frage}«")
        };
        treffer.push(VorlaeufigerBetrag {
            feld_id,
            wert,
            // Keine Richtungsaussage: gilt für einen Abzug wie für eine Anrechnung.
            hinweis: format!(
                "Du hast{ort} {} eingetragen, diesen Betrag aber noch nicht bestätigt. Solange das \
                 so ist, zeigt die Software keine Steuer an; sie könnte den Betrag sonst nicht \
                 mitrechnen. Bitte prüfe und bestätige ihn; danach rechnet die Software die Zahl.",
                eur(wert)
            ),
        });
    }
    treffer
}

/// `schulgeld` (Kind 1) und `schulgeld__<Ziffern>` (Kind 2..N), feld_id-sortiert, mit bestätigtem
/// Betrag > 0 (`preflight.py:101-115`). PARITÄT: `str.isdigit()` kennt auch Unicode-Ziffern; das
/// Store-Schema (`^[a-z][a-z0-9_]*$`) lässt nur ASCII zu. `schulgeld__0`/`__02` zählen wie in Python.
fn schulgeld_felder(felder: &Felder) -> Vec<(&str, i64)> {
    felder
        .keys()
        .filter(|fid| {
            *fid == "schulgeld"
                || fid
                    .strip_prefix("schulgeld__")
                    .is_some_and(|r| !r.is_empty() && r.bytes().all(|c| c.is_ascii_digit()))
        })
        .filter_map(|fid| bestaetigter_betrag(felder, fid).map(|b| (fid.as_str(), b)))
        .collect()
}

fn widerspruch(
    feld_id: &str,
    wert: PyWert,
    bezug: Option<i64>,
    grund: String,
) -> PlausiWiderspruch {
    PlausiWiderspruch {
        feld_id: feld_id.to_owned(),
        wert,
        bezug,
        grund,
    }
}

/// Lohnsteuer, RV-Beiträge und `KiSt` gegen den Bruttolohn (`preflight.py:235-274`).
fn gegen_brutto(felder: &Felder, brutto: i64, out: &mut Vec<PlausiWiderspruch>) {
    // § 32a Abs. 1 S. 2 Nr. 5: Spitzensatz 0,45 — Lohnsteuer über dem Lohn ist unmöglich.
    if let Some(lohnsteuer) = bestaetigter_betrag(felder, "p36_lohnsteuer").filter(|l| *l > brutto)
    {
        out.push(widerspruch(
            "p36_lohnsteuer",
            PyWert::Ganz(lohnsteuer),
            Some(brutto),
            format!(
                "Bei einem Bruttoarbeitslohn von {} kann dein Arbeitgeber nicht {} Lohnsteuer \
             einbehalten haben — die Lohnsteuer wird vom Lohn abgezogen und ist deshalb immer \
             kleiner als der Lohn. Bitte prüfe, welche der beiden Zahlen stimmt.",
                eur(brutto),
                eur(lohnsteuer)
            ),
        ));
    }
    // RV-Anteile sind ein Prozentsatz des Lohns — ohne Beitragssatz und BBG prüfbar.
    for (feld_id, name) in [
        ("vor_an_anteil_rv", "dein eigener Anteil"),
        ("vor_ag_anteil_rv", "der Anteil deines Arbeitgebers"),
    ] {
        if let Some(beitrag) = bestaetigter_betrag(felder, feld_id).filter(|b| *b > brutto) {
            out.push(widerspruch(feld_id, PyWert::Ganz(beitrag), Some(brutto), format!(
                "Bei einem Bruttoarbeitslohn von {} können die Rentenversicherungsbeiträge nicht {} \
                 betragen ({name}). Die Beiträge sind ein Anteil des Lohns und damit immer kleiner \
                 als der Lohn. Bitte prüfe, welche der beiden Zahlen stimmt.",
                eur(brutto), eur(beitrag))));
        }
    }
    for (feld_id, name) in [
        ("kist_gezahlt", "gezahlte"),
        ("kirchensteuer_arbeitgeber", "vom Arbeitgeber einbehaltene"),
    ] {
        if let Some(kist) =
            bestaetigter_betrag(felder, feld_id).filter(|k| kist_ueber_brutto_anteil(*k, brutto))
        {
            out.push(widerspruch(
                feld_id,
                PyWert::Ganz(kist),
                Some(brutto),
                format!(
                    "Bei einem Bruttoarbeitslohn von {} sind {} {name} Kirchensteuer sehr \
                 unwahrscheinlich. Die Kirchensteuer beträgt 8 bis 9 Prozent der Einkommensteuer \
                 und liegt damit üblicherweise im Bereich einiger hundert Euro. Bitte prüfe, \
                 welche der beiden Zahlen stimmt.",
                    eur(brutto),
                    eur(kist)
                ),
            ));
        }
    }
}

/// Kirchensteuer erklärt, Kirchenzugehörigkeit offen (`preflight.py:311-343`). Gemessen
/// 2026-08-28: 0 € `KiSt` statt 1.053,36 € bei richtiger `ESt` — deshalb melden, nicht sperren.
/// Ein Betrag von 0 € zählt nicht als Angabe; das Bundesland schon.
fn konfession_offen(felder: &Felder, out: &mut Vec<PlausiWiderspruch>) {
    // `isinstance(konfession, str) and konfession` — die leere Zeichenkette zaehlt nicht.
    let konfession = lies(felder, "kist_konfession").bestaetigt();
    if konfession.and_then(als_text).is_some_and(|s| !s.is_empty()) {
        return;
    }
    let bundesland = lies(felder, "kist_bundesland").bestaetigt();
    let beleg = [
        "kist_gezahlt",
        "kirchensteuer_arbeitgeber",
        "kist_erstattet",
    ]
    .into_iter()
    .find_map(|f| bestaetigter_betrag(felder, f).map(|b| (f, b)));
    let (feld_id, betrag) = match beleg {
        Some((f, b)) => (f, Some(b)),
        None if bundesland.and_then(als_text).is_some_and(|s| !s.is_empty()) => {
            ("kist_bundesland", None)
        }
        None => return,
    };
    let womit = match betrag {
        Some(b) => format!("Kirchensteuer gezahlt oder einbehalten ({})", eur(b)),
        None => "angegeben, in welchem Bundesland du Kirchensteuer zahlst".to_owned(),
    };
    out.push(widerspruch(
        feld_id,
        betrag.map_or(PyWert::Null, PyWert::Ganz),
        None,
        format!(
        "Du hast {womit} — ob du einer Kirche angehörst, die Kirchensteuer erhebt, ist aber noch \
         offen. Ohne diese Angabe wird deine Kirchensteuer nicht berechnet und fehlt in deinem \
         Ergebnis. Bitte gib an, ob du einer solchen Kirche angehörst — und wenn nicht, prüfe die \
         Kirchensteuer-Angaben noch einmal."),
    ));
}

/// Betrag ↔ Bezugsgröße (`preflight.py:222-370`). Nur bestätigte Werte; ohne Bezugsgröße bleibt
/// die jeweilige Prüfung still. `vorjahr_verlustvortrag` ist der bestätigte Bestand des letzten
/// verknüpften Vorjahres (`store["vorjahr_referenz"]["verlustvortrag_bestand"]["wert"]`, nur wenn
/// Ganzzahl); `graph` ist die VOLLE Bindung.
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// assert!(konsistenz::plausibilitaets_widersprueche(&konsistenz::Felder::new(), None, &g).is_empty());
/// ```
#[must_use]
pub fn plausibilitaets_widersprueche(
    felder: &Felder,
    vorjahr_verlustvortrag: Option<i64>,
    graph: &Graph<'_>,
) -> Vec<PlausiWiderspruch> {
    let mut out = Vec::new();
    if let Some(brutto) = bestaetigter_betrag(felder, "bruttoarbeitslohn") {
        gegen_brutto(felder, brutto, &mut out);
    }
    for (feld_id, betrag) in schulgeld_felder(felder) {
        if betrag > SCHULGELD_SCHWELLE_CENT {
            out.push(widerspruch(
                feld_id,
                PyWert::Ganz(betrag),
                None,
                format!(
                "{} Schulgeld für ein Kind in einem Jahr ist ungewöhnlich hoch. Absetzbar sind 30 \
                 Prozent des Schulgelds, höchstens 5.000 € je Kind — dieser Höchstbetrag ist \
                 bereits ab rund 16.700 € Schulgeld erreicht. Bitte prüfe den Betrag.",
                eur(betrag)),
            ));
        }
    }
    if let (Some(gezahlt), Some(einbehalten)) = (
        bestaetigter_betrag(felder, "kist_gezahlt"),
        bestaetigter_betrag(felder, "kirchensteuer_arbeitgeber"),
    ) {
        if i128::from(gezahlt.max(einbehalten))
            > i128::from(gezahlt.min(einbehalten)) * KIST_ABGLEICH_FAKTOR
        {
            out.push(widerspruch(
                "kist_gezahlt",
                PyWert::Ganz(gezahlt),
                Some(einbehalten),
                format!(
                "Dein Arbeitgeber hat {} Kirchensteuer einbehalten, gezahlt hast du nach deiner \
                 Angabe {}. Beide Angaben beschreiben normalerweise dieselbe Zahlung und liegen \
                 hier weit auseinander. Bitte prüfe, welche der beiden Angaben stimmt.",
                eur(einbehalten), eur(gezahlt)),
            ));
        }
    }
    // PARITAET: `preflight.py` prueft `keine_bank is True` (IDENTITAET) und
    // `isinstance(iban, str)`. Der strukturelle Vergleich bzw. `als_text` bilden beides ab.
    let keine_bank = lies(felder, "stammdaten_keine_bankverbindung").bestaetigt();
    if let (Some(PyWert::Bool(true)), Some(iban)) = (
        keine_bank,
        lies(felder, "stammdaten_iban")
            .bestaetigt()
            .and_then(als_text),
    ) {
        if !leer_nach_strip(iban) {
            out.push(widerspruch(
                "stammdaten_iban",
                PyWert::Text(iban.to_owned()),
                None,
                "Du hast angegeben, keine Bankverbindung für eine Erstattung angeben zu wollen — \
                 trotzdem ist eine Kontonummer (IBAN) erfasst. Ohne Konto kann das Finanzamt eine \
                 Erstattung nicht überweisen. Bitte prüfe, welche der beiden Angaben stimmt."
                    .to_owned(),
            ));
        }
    }
    konfession_offen(felder, &mut out);
    // Der Bestand wird von Jahr zu Jahr weniger; ein Anstieg braucht einen neuen Verlust im Vorjahr.
    if let (Some(alt), Some(neu)) = (
        vorjahr_verlustvortrag,
        bestaetigter_betrag(felder, "verlustvortrag_bestand"),
    ) {
        if neu > alt {
            out.push(widerspruch("verlustvortrag_bestand", PyWert::Ganz(neu), Some(alt), format!(
                "Im letzten verknüpften Vorjahr stand dein Verlustvortrag bei {}, jetzt gibst du {} \
                 an. Der Vortrag wird normalerweise von Jahr zu Jahr weniger, weil ein Teil davon \
                 verrechnet wird — ein Anstieg ist nur richtig, wenn im Vorjahr ein neuer Verlust \
                 hinzukam. Bitte prüfe den Betrag gegen deinen aktuellen Verlustfeststellungsbescheid.",
                eur(alt), eur(neu))));
        }
    }
    // Bewusst im selben Schlüssel: api.preflight_check liefert eine feste Schlüsselliste aus.
    out.extend(unvollstaendige_instanzen(felder, graph));
    out
}

/// Snapshot → Preflight-Ergebnis mit Ampel (`preflight.py:373-421`). `scheibe_felder` wie bei
/// [`flag_widersprueche`]; `graph` ist die VOLLE Bindung (`TR.lade_bindung()`).
///
/// ```
/// use konsistenz::{preflight, Ampel, Felder};
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// assert_eq!(preflight::<std::hash::RandomState>(&Felder::new(), None, None, &g).status, Ampel::Gruen);
/// ```
#[must_use]
pub fn preflight<S: BuildHasher>(
    felder: &Felder,
    scheibe_felder: Option<&HashSet<String, S>>,
    vorjahr_verlustvortrag: Option<i64>,
    graph: &Graph<'_>,
) -> PreflightErgebnis {
    let widersprueche_flag = flag_widersprueche(felder, scheibe_felder);
    let widersprueche_partner = partner_ohne_zusammen(felder);
    let widersprueche_alleinerziehend = alleinerziehend_mit_zusammen(felder);
    let widersprueche_plausibilitaet =
        plausibilitaets_widersprueche(felder, vorjahr_verlustvortrag, graph);
    let hinweise_pauschalen = pauschal_hinweise(felder);
    let hinweise_nicht_gerechnet = nicht_gerechnete_angaben(felder);
    let hinweise_betrag_vorlaeufig = vorlaeufige_ring_betraege(felder, graph);
    let status = if !widersprueche_flag.is_empty()
        || !widersprueche_partner.is_empty()
        || !widersprueche_alleinerziehend.is_empty()
        || !widersprueche_plausibilitaet.is_empty()
    {
        Ampel::Rot
    } else if !hinweise_pauschalen.is_empty()
        || !hinweise_nicht_gerechnet.is_empty()
        || !hinweise_betrag_vorlaeufig.is_empty()
    {
        Ampel::Gelb
    } else {
        Ampel::Gruen
    };
    PreflightErgebnis {
        widersprueche_flag,
        widersprueche_partner,
        widersprueche_alleinerziehend,
        widersprueche_plausibilitaet,
        hinweise_pauschalen,
        hinweise_nicht_gerechnet,
        hinweise_betrag_vorlaeufig,
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Pythons Bedingung wörtlich in IEEE-f64: `kist > brutto * 0.30` mit exaktem int/float-
    /// Vergleich (`kist > floor(f)`).
    fn python_kist(kist: i64, brutto: i64) -> bool {
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let boden = ((brutto as f64) * 0.30).floor() as i128;
        i128::from(kist) > boden
    }

    #[test]
    fn schulgeld_schwelle_ist_python_wert() {
        assert_eq!(SCHULGELD_SCHWELLE_CENT, 500_000 * 10 / 3 * 10);
    }

    #[test]
    fn grenze_ist_floor_2_hoch_50_mal_10_drittel() {
        assert_eq!(i128::from(BRUTTO_FLOAT_EXAKT_MAX), (1_i128 << 50) * 10 / 3);
    }

    #[test]
    fn kist_schwelle_gleich_python_an_raendern() {
        let mut brutto_werte: Vec<i64> = (1..=100_000).collect();
        brutto_werte.extend((0..10_000).map(|d| BRUTTO_FLOAT_EXAKT_MAX - d));
        for b in brutto_werte {
            let k = i64::try_from(i128::from(b) * 3 / 10).unwrap();
            for kist in [k - 1, k, k + 1] {
                assert_eq!(
                    kist_ueber_brutto_anteil(kist, b),
                    python_kist(kist, b),
                    "b={b} kist={kist}"
                );
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20_000))]
        #[test]
        fn kist_schwelle_gleich_python_im_bereich(b in 1..=BRUTTO_FLOAT_EXAKT_MAX, d in -2_i64..=2) {
            let kist = i64::try_from(i128::from(b) * 3 / 10).unwrap() + d;
            prop_assert_eq!(kist_ueber_brutto_anteil(kist, b), python_kist(kist, b));
        }
    }

    /// GEWOLLTE ABWEICHUNG von Python (Wertebereich, Crate-Doku `lib.rs`): ein Float ist in den Betragspruefungen
    /// KEIN Betrag. Python nimmt `isinstance(wert, (int, float))` (`preflight.py:89-98`), `1500.5` gilt dort als Betrag und
    /// loest die Pruefung aus. Der Store laesst auf `cent`-Feldern nur Ganzzahlen zu (Auflage T; 0 Floats in 192 echten
    /// Faellen). Wer Floats hier zaehlen laesst, aendert den Wertebereich und traegt die Abweichung neu ein.
    #[test]
    fn float_ist_kein_betrag_abweichung_von_python() {
        use crate::lesung::test_snap;
        use domain::Zustand::{Bestaetigt, Vorlaeufig};

        let bestaetigt = |w: PyWert| test_snap(&[("bruttoarbeitslohn", w, Bestaetigt)]);
        for (wert, erwartet) in [
            (PyWert::Ganz(1500), Some(1500)),
            (PyWert::Gleit(1500.5), None), // Python: 1500.5
            (PyWert::Gleit(1500.0), None), // Python: 1500.0
            (PyWert::Bool(true), None),    // Python: ebenfalls kein Betrag
        ] {
            assert_eq!(
                bestaetigter_betrag(&bestaetigt(wert.clone()), "bruttoarbeitslohn"),
                erwartet,
                "{wert:?}"
            );
        }

        let reg = interview::doctest_registry().unwrap();
        let g = Graph::aus_registry(&reg);

        // Lohnsteuer ueber dem Lohn: mit Ganzzahlen ein Widerspruch (Kontrolle), mit Floats keiner (Python: einer).
        let lohn = |brutto: PyWert, steuer: PyWert| {
            plausibilitaets_widersprueche(
                &test_snap(&[
                    ("bruttoarbeitslohn", brutto, Bestaetigt),
                    ("p36_lohnsteuer", steuer, Bestaetigt),
                ]),
                None,
                &g,
            )
        };
        let w = lohn(PyWert::Ganz(100_000), PyWert::Ganz(200_000));
        assert_eq!(w.len(), 1, "Kontrolle mit Ganzzahlen");
        assert_eq!(w[0].feld_id, "p36_lohnsteuer");
        for (brutto, steuer) in [
            (PyWert::Gleit(100_000.0), PyWert::Gleit(200_000.0)),
            (PyWert::Ganz(100_000), PyWert::Gleit(200_000.0)),
            (PyWert::Gleit(100_000.0), PyWert::Ganz(200_000)),
        ] {
            assert!(
                lohn(brutto.clone(), steuer.clone()).is_empty(),
                "{brutto:?} / {steuer:?}"
            );
        }

        // Vorlaeufiger Ring-Betrag: eine Ganzzahl wird gemeldet (Kontrolle), ein Float nicht (Python: ja).
        let vorlaeufig = |w: PyWert| {
            vorlaeufige_ring_betraege(&test_snap(&[("bruttoarbeitslohn", w, Vorlaeufig)]), &g)
        };
        let v = vorlaeufig(PyWert::Ganz(150_000));
        assert_eq!(
            v.iter().map(|b| (b.feld_id, b.wert)).collect::<Vec<_>>(),
            [("bruttoarbeitslohn", 150_000)],
            "Kontrolle mit Ganzzahl"
        );
        assert!(vorlaeufig(PyWert::Gleit(1500.0)).is_empty());
    }

    #[test]
    fn aufzaehlung_wie_python() {
        assert_eq!(aufzaehlung("Kind", &[3]), "Kind 3");
        assert_eq!(aufzaehlung("Kind", &[2, 3]), "Kind 2 und Kind 3");
        assert_eq!(aufzaehlung("Kind", &[2, 3, 4]), "Kind 2, Kind 3 und Kind 4");
    }
}
