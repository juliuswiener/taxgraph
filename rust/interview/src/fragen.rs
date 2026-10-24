//! Rueckwaerts, Teil 2: die geordnete Interview-Queue (`naechste_fragen`,
//! `traverser.py:380-733`).
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};

use bindung::{Bindung, Bindungspunkt, FeldBedingung, Vorjahr};
use domain::PyWert;
use store::{Event, Store};

use crate::antwort::{Aktiv, Antwort};
use crate::graph::{Graph, Sicht};
use crate::instanz::{instanz_antworten, instanz_unvollstaendig};
use crate::relevanz::{
    bedingung_je_instanz, gate_gewicht, relevanz_mit, Bedingungsstand, RegelRelevanz, Regelstatus,
};

/// Geordnete Interview-Queue: unbeantwortete askable Felder nicht-ausgeschlossener Regeln
/// (`naechste_fragen`, `traverser.py:395-444`).
///
/// Ordnung: Gates zuerst — nach [`gate_gewicht`] absteigend, bei Gleichstand die
/// `eingangsfrage` vor den Merkmalsfragen (§ 35a, 2026-08-21: "NICHT oeffentlich gefoerdert?"
/// kam vor "hattest du Handwerkerkosten?"), dann alphabetisch. Dann die Slots nach
/// Unsicherheits-`beitrag` (Cent, aus dem Intervall), ohne Beitrag alphabetisch. Danach Themen
/// am Stueck (`_nach_themen`), Vordruck-Ordnung innerhalb des Themas, Themen hinter ihre
/// Voraussetzung.
///
/// Guenstiger-sicher by construction: ALLE offenen askable Felder nicht-ausgeschlossener Regeln
/// stehen in der Queue — kein Zweig faellt wegen eines vorlaeufigen Siegers weg (einzige Ausnahme:
/// `feld_bedingung.groesser_als`, siehe `feld_ausgeschlossen`). Ein leerer
/// `beitrag` gilt wie keiner (Python: `if beitrag:`).
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let ohne_beitrag: Option<&std::collections::HashMap<String, i64>> = None;
/// let q = interview::naechste_fragen(&store::Store::leer(2025, None), g.alle(), &g, ohne_beitrag);
/// assert!(q.len() > 100);
/// assert!(q.iter().all(|f| g.alle().get(f).unwrap().askable));
/// ```
#[must_use]
pub fn naechste_fragen<'r, S: std::hash::BuildHasher>(
    store: &Store,
    sicht: &Sicht<'r>,
    graph: &Graph<'r>,
    beitrag: Option<&HashMap<String, i64, S>>,
) -> Vec<&'r str> {
    naechste_fragen_mit_pflicht(store, sicht, graph, beitrag, &[])
}

/// [`naechste_fragen`] mit den Pflichtfeldern der Scheibe: den Feldern, deren BESTAETIGTE Antwort die Steuerzahl verlangt (der
/// Pflicht-Kegel `cfg["kegel"]` und die Ring-Betraege, ohne sie `input_kegel_nicht_bestaetigt` und `ring_betrag_vorlaeufig`).
///
/// Die Pflichtfelder entscheiden nur, welche Felder aus der Vorjahres-Uebernahme eine Frage bleiben ([`vorjahr_uebernommen`]).
/// Ohne Vorjahres-Werte im Fall aendert sich die Queue nicht; `&[]` ist [`naechste_fragen`].
#[must_use]
pub fn naechste_fragen_mit_pflicht<'r, S: std::hash::BuildHasher>(
    store: &Store,
    sicht: &Sicht<'r>,
    graph: &Graph<'r>,
    beitrag: Option<&HashMap<String, i64, S>>,
    pflicht: &[&str],
) -> Vec<&'r str> {
    let aktiv = Aktiv::aus(store);
    let rel = relevanz_mit(&aktiv, sicht, graph);
    let vz = store.datei().veranlagungszeitraum.als_i64_saettigend();
    let braucht_bestaetigung = braucht_bestaetigung(sicht, graph, pflicht);
    let kand: Vec<&'r Bindung> = sicht
        .iter()
        .filter(|b| {
            b.askable
                && (aktiv.antwort(&b.feld_id).ist_offen()
                    || instanz_unvollstaendig(&aktiv, sicht, graph, &b.feld_id))
                && !vorjahr_uebernommen(b, aktiv.get(&b.feld_id), &braucht_bestaetigung)
                && rel
                    .get(b.quelle.regel_id.as_str())
                    .is_none_or(|r| r.status != Regelstatus::Ausgeschlossen)
                && !feld_ausgeschlossen(b, &aktiv, sicht, graph, vz, &rel)
        })
        .collect();
    let gw = gate_gewicht(sicht, graph);
    let gewicht = |b: &Bindung| gw.get(b.feld_id.as_str()).copied().unwrap_or(0);
    // "Gate" heisst hier: die Antwort streicht andere Fragen. `veranlagung` hat KEINE
    // geltungsbedingung, wirkt aber ueber regel_bedingungen auf 38 Partner-Felder.
    let ist_gate = |b: &Bindung| {
        matches!(b.quelle.bindungspunkt, Bindungspunkt::Geltungsbedingung(_)) || gewicht(b) > 0
    };
    let (mut gates, mut slots): (Vec<&'r Bindung>, Vec<&'r Bindung>) =
        kand.into_iter().partition(|b| ist_gate(b));
    gates.sort_by(|a, b| {
        (Reverse(gewicht(a)), !a.eingangsfrage, a.feld_id.as_str()).cmp(&(
            Reverse(gewicht(b)),
            !b.eingangsfrage,
            b.feld_id.as_str(),
        ))
    });
    let beitrag = beitrag.filter(|m| !m.is_empty());
    match beitrag {
        Some(m) => slots.sort_by_key(|b| {
            (
                Reverse(m.get(&b.feld_id).copied().unwrap_or(0)),
                b.feld_id.as_str(),
            )
        }),
        None => slots.sort_by_key(|b| b.feld_id.as_str()),
    }
    gates.extend(slots);
    let angefangen = angefangene_themen(store, sicht);
    nach_themen(&gates, sicht, graph, &gw, &angefangen, beitrag.is_some())
}

/// `vorjahr: uebernehmbar` mit Vorjahres-Wert faellt aus der Queue, bleibt aber korrigierbar
/// (`_vorjahr_uebernommen`, `traverser.py:380-392`). `vorschlag` bleibt eine Frage.
///
/// Rust-eigen (Fluss J F1, 2026-10-21; weicht vom Entscheid 2026-09-05 `vorjahr-kategorie-ohne-verhalten` ab): Ein Feld, an dessen
/// BESTAETIGTER Antwort etwas haengt, bleibt eine Frage, vorbelegt mit dem Wert des Vorjahres ([`braucht_bestaetigung`]). Der Wert
/// aus dem Vorjahr ist nur `vorlaeufig`: ohne Bestaetigung blieben die Fragen hinter einem Gate offen (der Einzelnutzer wurde
/// nach dem Ehegatten gefragt), und eine Ableitung rechnete nicht (das Geburtsjahr blieb vorlaeufig, die Sperre
/// `ring_betrag_vorlaeufig` hielt die Steuerzahl zurueck).
fn vorjahr_uebernommen(b: &Bindung, ev: Option<&Event>, braucht_bestaetigung: &HashSet<&str>) -> bool {
    b.vorjahr == Some(Vorjahr::Uebernehmbar)
        && ev.is_some_and(|e| e.herkunft.herkunft_achse().as_str() == "vorjahr")
        && !braucht_bestaetigung.contains(b.feld_id.as_str())
}

/// Die Felder, an deren BESTAETIGTER Antwort etwas haengt, das ein Wert aus dem Vorjahr nicht ersetzt. Keine feste Liste:
/// - Ein Gate ueber `regel_bedingungen` (`veranlagung`): erst eine bestaetigte Antwort schliesst die Regeln des Ehegatten aus.
/// - Der Ausloeser einer `ableitung` (`aus`, `und_feld`; `stammdaten_geburtsdatum`): die Ableitung rechnet nur aus einem
///   bestaetigten Wert.
/// - Ein Pflichtfeld der Scheibe (`pflicht`): die Steuerzahl kommt erst, wenn jedes relevante Kegel-Feld und jeder Ring-Betrag
///   bestaetigt ist. Ein Vorjahreswert ist nur `vorlaeufig`; ohne Frage blieb die Sperre `input_kegel_nicht_bestaetigt`
///   (Fluss J: `versicherungsart`, `mit_anspruch_auf_zuschuss`, `ep_eigenes_kfz`, `ep_entfernung_km`) oder
///   `ring_betrag_vorlaeufig` (`versorgung_beginn_jahr_partner`).
///
/// Ein Gate, dessen Folgefragen auch ohne Bestaetigung gestellt werden (Eingangsfrage einer Regel, `feld_bedingung`), gehoert
/// nicht dazu: seine Folgefragen bleiben offen, und nichts wartet auf die Bestaetigung.
fn braucht_bestaetigung<'r>(sicht: &Sicht<'r>, graph: &Graph<'r>, pflicht: &[&str]) -> HashSet<&'r str> {
    let gate = sicht
        .iter()
        .filter(|b| !graph.bedingte_regeln(&b.feld_id).is_empty())
        .map(|b| b.feld_id.as_str());
    let ausloeser = sicht
        .iter()
        .filter_map(|b| b.ableitung.as_ref())
        .flat_map(|a| [Some(a.aus.as_str()), a.und_feld.as_deref()])
        .flatten();
    let pflichtfeld = sicht
        .iter()
        .filter(|b| pflicht.contains(&b.feld_id.as_str()))
        .map(|b| b.feld_id.as_str());
    gate.chain(ausloeser).chain(pflichtfeld).collect()
}

/// Faellt DIESES Feld weg, obwohl seine Regel gilt (`_feld_ausgeschlossen`,
/// `traverser.py:523-558`)? Nur bei BESTAETIGT abweichender Antwort jeder Instanz; `wert_nicht`
/// fuer Auswahlfelder (`kist_konfession == "keine"`).
///
/// PARITAET: Python unterscheidet "Schluessel `wert_nicht` vorhanden" von "fehlt"; die Bindung
/// kennt nur `Option`. Ein ausdrueckliches `wert_nicht: null` hiesse in Python "gleich None";
/// in keiner `bindung_*.yaml` belegt (gemessen: 42× `wert: false`, 8× `"zusammen"`, 5×
/// `wert_nicht: "keine"`, 4× `wert_nicht: 0`, 3× `wert: true`).
///
/// Rust-eigen (Abweichung Nr. 32): `alter_im_vz` meint ein Geburtsjahr in `feld`. Das Feld bleibt nur im Jahr, in dem
/// Veranlagungsjahr `vz` minus Geburtsjahr genau diese Zahl ergibt; jedes BESTAETIGTE andere Geburtsjahr schliesst es aus,
/// ein fehlendes oder vorlaeufiges nicht (fail-closed wie `wert`). Ein bestaetigter Wert, der kein Ganzzahl-Geburtsjahr ist,
/// schliesst ebenfalls nicht aus: die Frage bleibt.
///
/// Rust-eigen (Abweichung Nr. 34): `und` haengt weitere Bedingungen an; das Feld faellt weg, sobald EINE Bedingung der Kette
/// es ausschliesst. `groesser_als` meint einen Betrag in `feld` und gilt als POSITIVER BEWEIS, der einzige Fall, in dem diese
/// Funktion bei Schweigen ausschliesst: das Feld bleibt nur, wenn mindestens eine Instanz von `feld` bestaetigt ueber der
/// Schwelle liegt. Das bricht den Satz "alle offenen Felder stehen in der Queue" mit Absicht und nur fuer Folgefragen, deren
/// Fehlen nichts kostet (der Anteil am Schulgeld-Hoechstbetrag: keine Antwort heisst "je zur Haelfte"); die Eingangsfrage nach
/// dem Betrag selbst steht weiter in der Queue und schaltet sie frei.
///
/// Rust-eigen (Entscheidung d-43): `oder` haengt eine Alternative an ein Glied. Schliesst das Glied das Feld aus, bleibt es
/// trotzdem, sobald eine Alternative gilt ([`alternative_gilt`]). Die Frage nach dem Bundesland haengt so an Person A ODER
/// Person B; vorher fehlte sie, wenn nur der Ehegatte einer Kirche angehoerte, und die Kirchensteuer rechnete mit 9 %.
///
/// Rust-eigen (Fragen-Reduktion Teil 2): `beleg: true` macht ein Glied der Kette zum POSITIVEN BEWEIS ([`beleg_fehlt`]). Das Feld
/// bleibt nur, wenn `feld` bestaetigt beantwortet ist und die Bedingung erfuellt; Schweigen und ein vorlaeufiger Wert schliessen
/// aus. Eine `oder`-Alternative, die gilt, haelt es weiter offen.
fn feld_ausgeschlossen(
    b: &Bindung,
    aktiv: &Aktiv<'_>,
    sicht: &Sicht<'_>,
    graph: &Graph<'_>,
    vz: i64,
    rel: &BTreeMap<&str, RegelRelevanz<'_>>,
) -> bool {
    let Some(bed) = &b.feld_bedingung else {
        return false;
    };
    bed.und_glieder().any(|glied| {
        (bedingung_schliesst_aus(glied, aktiv, sicht, graph, vz) || beleg_fehlt(glied, aktiv, sicht, graph, vz))
            && !glied
                .alternativen()
                .any(|alt| alternative_gilt(alt, aktiv, sicht, graph, vz, rel))
    })
}

/// Fehlt dem Glied mit `beleg: true` der Beleg? Ja, solange `feld` nicht in jeder Instanz bestaetigt beantwortet ist und eine
/// Instanz die Bedingung erfuellt ([`Bedingungsstand::Erfuellt`]). Das Feld der Bedingung muss die Scheibe fragen: sonst gaebe es
/// nie einen Beleg, und die Frage fiele fuer immer weg (Abweichung Nr. 74, wie bei [`bedingung_schliesst_aus`]).
fn beleg_fehlt(bed: &FeldBedingung, aktiv: &Aktiv<'_>, sicht: &Sicht<'_>, graph: &Graph<'_>, vz: i64) -> bool {
    bed.beleg
        && sicht.get(&bed.feld).is_some()
        && bedingung_stand(bed, aktiv, sicht, graph, vz) != Bedingungsstand::Erfuellt
}

/// Liegt der Betrag `w` echt ueber `schwelle`? Nur eine Ganzzahl belegt einen Betrag: `cent` und `int` sind ganzzahlig (kein
/// Float im Rechenpfad, Tor 2b); Text, Wahrheitswert, Null und eine Kommazahl belegen nichts.
fn ueber_schwelle(w: &PyWert, schwelle: i64) -> bool {
    matches!(w, PyWert::Ganz(n) if *n > schwelle)
}

/// Schliesst GENAU DIESE Bedingung (ein Glied der Kette) das Feld aus?
///
/// Rust-eigen (Abweichung Nr. 74): eine Bedingung auf ein Feld, das die Scheibe nicht fragt, schliesst nichts aus.
/// Ausnahme `groesser_als`, der Beleg.
fn bedingung_schliesst_aus(
    bed: &FeldBedingung,
    aktiv: &Aktiv<'_>,
    sicht: &Sicht<'_>,
    graph: &Graph<'_>,
    vz: i64,
) -> bool {
    if let Some(schwelle) = bed.groesser_als {
        return !instanz_antworten(aktiv, sicht, graph, &bed.feld).iter().any(
            |a| matches!(a, Antwort::Bestaetigt(w) if ueber_schwelle(w, schwelle)),
        );
    }
    // Der Nutzer kann ein Feld, das die Scheibe nicht fragt, hier nie setzen oder zuruecknehmen. Stuende eine Antwort darauf
    // trotzdem im Store (von Hand geaenderte Akte), liesse sie eine Frage entfallen, die der Guard oder der Kegel weiter
    // verlangt: eine Sackgasse. Hinter `groesser_als`, denn dort gilt ein Beleg, und ohne beantwortbares Feld bleibt die
    // Folgefrage weg.
    if sicht.get(&bed.feld).is_none() {
        return false;
    }
    bedingung_stand(bed, aktiv, sicht, graph, vz) == Bedingungsstand::Ausgeschlossen
}

/// Beginnt der Text mit dem Praefix, ohne Leerzeichen und ohne Gross-/Kleinschreibung? Die IBAN steht so im Store, wie der
/// Nutzer sie getippt hat ("de89 3704 ..."); die Erklaerung normalisiert sie genauso (`Bau::iban`).
fn beginnt_mit_praefix(text: &str, praefix: &str) -> bool {
    let norm: String = text.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_uppercase).collect();
    norm.starts_with(&praefix.to_uppercase())
}

/// Der Stand einer Bedingung der Arten `wert`, `wert_nicht`, `alter_im_vz` und `praefix_nicht` ueber alle Instanzen ihres Felds: ausgeschlossen,
/// wenn JEDE Instanz bestaetigt abweicht, erfuellt, wenn alle bestaetigt sind und eine nicht abweicht, sonst offen.
fn bedingung_stand(
    bed: &FeldBedingung,
    aktiv: &Aktiv<'_>,
    sicht: &Sicht<'_>,
    graph: &Graph<'_>,
    vz: i64,
) -> Bedingungsstand {
    if let Some(alter) = bed.alter_im_vz {
        bedingung_je_instanz(aktiv, sicht, graph, &bed.feld, |w| {
            matches!(w, PyWert::Ganz(gj) if vz.checked_sub(*gj) != Some(alter))
        })
    } else if let Some(praefix) = &bed.praefix_nicht {
        bedingung_je_instanz(aktiv, sicht, graph, &bed.feld, |w| {
            matches!(w, PyWert::Text(t) if beginnt_mit_praefix(t, praefix))
        })
    } else if let Some(nicht) = &bed.wert_nicht {
        let nicht = PyWert::from(nicht.clone());
        bedingung_je_instanz(aktiv, sicht, graph, &bed.feld, |w| w.py_eq(&nicht))
    } else {
        let soll = bed.wert.clone().map_or(PyWert::Null, PyWert::from);
        bedingung_je_instanz(aktiv, sicht, graph, &bed.feld, |w| !w.py_eq(&soll))
    }
}

/// Haelt DIESE Alternative (`oder`) das Feld offen, obwohl das Glied, an dem sie haengt, es ausschliesst?
///
/// POSITIVER BEWEIS (wie `groesser_als`, anders als `wert`): ja nur, wenn die Scheibe das Feld der Alternative fragt, seine Regel
/// nicht ausgeschlossen ist (bei Einzelveranlagung gilt die Frage nach dem Ehegatten nicht, eine alte Antwort darauf zaehlt dort
/// nicht) und eine BESTAETIGTE Antwort die Bedingung erfuellt. Schweigen, ein vorlaeufiger Wert und "keine" halten die Frage nicht
/// offen. Das kostet nichts, wo die Alternative eine Folgefrage freischaltet: ohne bestaetigtes Mitglied gibt es keinen Hebesatz.
/// Ohne diese Strenge fragte jede Scheibe ohne Ehegatten-Feld und jede Einzelveranlagung das Bundesland bei jedem Fall.
fn alternative_gilt(
    alt: &FeldBedingung,
    aktiv: &Aktiv<'_>,
    sicht: &Sicht<'_>,
    graph: &Graph<'_>,
    vz: i64,
    rel: &BTreeMap<&str, RegelRelevanz<'_>>,
) -> bool {
    let Some(feld) = sicht.get(&alt.feld) else {
        return false;
    };
    if rel
        .get(feld.quelle.regel_id.as_str())
        .is_some_and(|r| r.status == Regelstatus::Ausgeschlossen)
    {
        return false;
    }
    if alt.groesser_als.is_some() {
        return !bedingung_schliesst_aus(alt, aktiv, sicht, graph, vz);
    }
    bedingung_stand(alt, aktiv, sicht, graph, vz) == Bedingungsstand::Erfuellt
}

/// Themen mit mindestens einer BESTAETIGTEN Antwort, das zuletzt angefasste zuerst
/// (`_angefangene_themen`, `traverser.py:447-464`). Liest ALLE Events (auch ersetzte) — die
/// Log-Reihenfolge ist die Antwort-Reihenfolge.
fn angefangene_themen<'r>(store: &Store, sicht: &Sicht<'r>) -> Vec<&'r str> {
    let mut zuletzt: HashMap<&'r str, usize> = HashMap::new();
    for (i, e) in store.events().iter().enumerate() {
        if Antwort::aus(Some(e)).ist_offen() {
            continue;
        }
        if let Some(b) = sicht.get(&e.feld_id) {
            if !b.quelle.regel_id.is_empty() {
                zuletzt.insert(b.quelle.regel_id.as_str(), i);
            }
        }
    }
    let mut themen: Vec<(&'r str, usize)> = zuletzt.into_iter().collect();
    themen.sort_by_key(|&(_, i)| Reverse(i));
    themen.into_iter().map(|(t, _)| t).collect()
}

/// Themen am Stueck, ohne die Rangfolge umzuwerfen (`_nach_themen`, `traverser.py:467-520`):
/// ein Thema steht dort, wo sein BESTES Feld stand; innerhalb gilt die bisherige Ordnung, die
/// Eingangsfrage zuerst. Gemessen vorher: 175 Themenwechsel bei 62 Themen.
fn nach_themen<'r>(
    felder: &[&'r Bindung],
    sicht: &Sicht<'r>,
    graph: &Graph<'r>,
    gw: &HashMap<&'r str, usize>,
    angefangen: &[&'r str],
    gewicht_aktiv: bool,
) -> Vec<&'r str> {
    let mut themen: Vec<(&'r str, Vec<&'r Bindung>)> = Vec::new();
    let mut pos: HashMap<&'r str, usize> = HashMap::new();
    for &b in felder {
        let t = b.quelle.regel_id.as_str();
        let i = *pos.entry(t).or_insert_with(|| {
            themen.push((t, Vec::new()));
            themen.len() - 1
        });
        if let Some((_, gruppe)) = themen.get_mut(i) {
            gruppe.push(b);
        }
    }
    for (_, gruppe) in &mut themen {
        let (eingang, rest): (Vec<&Bindung>, Vec<&Bindung>) =
            gruppe.iter().partition(|b| b.eingangsfrage);
        let geordnet: Vec<&Bindung> = eingang.into_iter().chain(rest).collect();
        *gruppe = nach_ausloesern(nach_vordruck(&geordnet, gw, gewicht_aktiv));
    }
    let folge = themen_folge(&themen, sicht, graph, angefangen);
    let mut out = Vec::with_capacity(felder.len());
    for t in folge {
        if let Some((_, gruppe)) = pos.get(t).and_then(|&i| themen.get(i)) {
            out.extend(gruppe.iter().map(|b| b.feld_id.as_str()));
        }
    }
    // Guenstiger-sicher: jedes Feld genau einmal, kein Thema verloren oder doppelt gesetzt.
    debug_assert_eq!(
        out.len(),
        felder.len(),
        "Themenfolge verliert oder verdoppelt Felder"
    );
    out
}

/// Ein Feld mit `ableitung` steht hinter seinen Ausloesern (`aus`, `und_feld`), auch im selben
/// Thema (`_nach_ausloesern`, `traverser.py`; Vault: `decisions/ableitung-feuert-je-instanz-und-
/// frage-nach-beiden-ausloesern`, Punkt 2). `themen_folge` ordnet nur THEMEN; innerhalb eines
/// Themas stand das Ziel `kind_unter_14_haushaltszugehoerig` (Gate) vor seinem `und_feld` (Slot),
/// und wer der Queue folgte, beantwortete es, bevor die Ableitung feuern konnte.
///
/// Das Ziel bleibt eine Frage; nur seine STELLE aendert sich: direkt hinter den letzten Ausloeser,
/// der noch weiter hinten steht. Mehrere Nachzuegler hinter demselben Ausloeser behalten ihre
/// Reihenfolge, die Menge bleibt gleich. Ein Ausloeser mit eigener `ableitung` zaehlt nicht (keine
/// Ketten; er koennte selbst nachruecken und seinen Nachzuegler verlieren), ebenso wenig einer, der
/// nicht in der Gruppe steht (anderes Thema, schon beantwortet).
// ponytail: nur Ausloeser derselben Gruppe. Liegt ein `und_feld` in einem anderen Thema, ordnet
// `themen_folge` es nicht vor (dort gilt nur `aus`); heute gibt es keinen solchen Fall.
fn nach_ausloesern(gruppe: Vec<&Bindung>) -> Vec<&Bindung> {
    let ort: HashMap<&str, usize> = gruppe
        .iter()
        .enumerate()
        .map(|(i, b)| (b.feld_id.as_str(), i))
        .collect();
    // Index des Ausloesers -> Indizes seiner Nachzuegler, in Gruppenordnung.
    let mut hinter: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, b) in gruppe.iter().enumerate() {
        // Ausloeser: Quelle und `und_feld` der Ableitung, dazu das Feld einer `praefix_nicht`-Bedingung: die Antwort auf die IBAN
        // entscheidet, ob der BIC noch gefragt wird, also steht er dahinter.
        // ponytail: nur `praefix_nicht`. Die Folge der uebrigen `feld_bedingung` ist gewollt (das Bundesland stand vor der
        // Konfession, an der seine Bedingung haengt, Fluss I Befund B; seit B1 ordnet die Eingangsfrage die Konfession davor, nicht
        // die Bedingung); wer sie auch ordnen will, entscheidet das je Feld.
        let mut ausloeser: Vec<&str> = b
            .ableitung
            .iter()
            .flat_map(|regel| [Some(regel.aus.as_str()), regel.und_feld.as_deref()])
            .flatten()
            .collect();
        ausloeser.extend(
            b.feld_bedingung
                .iter()
                .flat_map(FeldBedingung::und_glieder)
                .filter(|g| g.praefix_nicht.is_some())
                .map(|g| g.feld.as_str()),
        );
        let weiter_hinten = ausloeser
            .into_iter()
            .filter_map(|x| ort.get(x).copied())
            .filter(|&j| j > i && gruppe.get(j).is_some_and(|a| a.ableitung.is_none()))
            .max();
        if let Some(j) = weiter_hinten {
            hinter.entry(j).or_default().push(i);
        }
    }
    if hinter.is_empty() {
        return gruppe;
    }
    let nachgezogen: HashSet<usize> = hinter.values().flatten().copied().collect();
    let mut aus = Vec::with_capacity(gruppe.len());
    for (i, &b) in gruppe.iter().enumerate() {
        if nachgezogen.contains(&i) {
            continue;
        }
        aus.push(b);
        for k in hinter.get(&i).into_iter().flatten() {
            aus.extend(gruppe.get(*k).copied());
        }
    }
    aus
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Klasse {
    Formal,
    Wert,
}

/// Innerhalb eines Themas: Felder mit ELSTER-Kennzahl in Vordruck-Reihenfolge, nur innerhalb
/// ihrer Klasse (`_nach_vordruck`, `traverser.py:561-623`). Eingangsfragen und echte Gates
/// (Gewicht > 0) bleiben stehen; "formal" = traegt eine Geltungsbedingung, streicht nichts.
/// Mit Unsicherheits-Beitrag bleibt die Klasse "wert" in Beitrags-Ordnung.
fn nach_vordruck<'r>(
    gruppe: &[&'r Bindung],
    gw: &HashMap<&'r str, usize>,
    gewicht_aktiv: bool,
) -> Vec<&'r Bindung> {
    let klasse = |b: &Bindung| -> Option<Klasse> {
        if b.eingangsfrage || gw.get(b.feld_id.as_str()).copied().unwrap_or(0) > 0 {
            return None;
        }
        Some(match b.quelle.bindungspunkt {
            Bindungspunkt::Geltungsbedingung(_) => Klasse::Formal,
            Bindungspunkt::SignaturSlot(_) => Klasse::Wert,
        })
    };
    let mut aus = gruppe.to_vec();
    for k in [Klasse::Formal, Klasse::Wert] {
        if k == Klasse::Wert && gewicht_aktiv {
            continue;
        }
        let mit: Vec<&Bindung> = gruppe
            .iter()
            .copied()
            .filter(|b| b.elster_kz.is_some() && klasse(b) == Some(k))
            .collect();
        if mit.len() < 2 {
            continue;
        }
        let kandidaten: HashSet<&str> = mit.iter().map(|b| b.feld_id.as_str()).collect();
        let mut sortiert = mit.clone();
        sortiert.sort_by(|a, b| a.elster_kz.cmp(&b.elster_kz));
        let mut naechster = sortiert.into_iter();
        for platz in &mut aus {
            if kandidaten.contains(platz.feld_id.as_str()) {
                if let Some(b) = naechster.next() {
                    *platz = b;
                }
            }
        }
    }
    aus
}

/// Ein Thema folgt ALLEN Themen, die seine Voraussetzung erheben (`_themen_folge`,
/// `traverser.py:626-733`): `regel_bedingungen`-Feld, `ableitung.aus`, Zaehlfeld der
/// Instanz-Gruppe. Einstieg fest (`themen_zuerst`), dann angefangene Themen, dann der Rest.
/// KEINE Topologie-Sortierung: runden-weise, bei einem Ring bleibt der Rest in
/// Wunsch-Reihenfolge — nie Endlosschleife, nie ein verlorenes Thema.
fn themen_folge<'r>(
    themen: &[(&'r str, Vec<&'r Bindung>)],
    sicht: &Sicht<'r>,
    graph: &Graph<'r>,
    angefangen: &[&'r str],
) -> Vec<&'r str> {
    let vorhanden: HashSet<&str> = themen.iter().map(|(t, _)| *t).collect();
    let mut quelle: HashMap<&'r str, HashSet<&'r str>> = HashMap::new();
    for (thema, felder) in themen {
        let mut merke = |feld: &str| {
            if let Some(qt) = sicht.get(feld).map(|b| b.quelle.regel_id.as_str()) {
                if !qt.is_empty() && qt != *thema && vorhanden.contains(qt) {
                    quelle.entry(thema).or_default().insert(qt);
                }
            }
        };
        for c in graph.regel_bedingungen(thema) {
            merke(&c.feld);
        }
        for b in felder {
            if let Some(abl) = &b.ableitung {
                merke(&abl.aus);
            }
            let zaehl = b
                .instanz_gruppe
                .as_deref()
                .filter(|g| !g.is_empty())
                .and_then(|g| graph.instanz_gruppe(g))
                .map(|g| g.anzahl_feld.as_str())
                .filter(|z| !z.is_empty());
            if let Some(z) = zaehl {
                if !felder.iter().any(|f| f.feld_id == z) {
                    merke(z);
                }
            }
        }
    }
    let vorne: Vec<&'r str> = graph
        .themen_zuerst()
        .iter()
        .copied()
        .filter(|t| vorhanden.contains(t))
        .collect();
    let laufend: Vec<&'r str> = angefangen
        .iter()
        .copied()
        .filter(|t| vorhanden.contains(t) && !vorne.contains(t))
        .collect();
    let wunsch: Vec<&'r str> = vorne
        .iter()
        .chain(&laufend)
        .copied()
        .chain(
            themen
                .iter()
                .map(|(t, _)| *t)
                .filter(|t| !vorne.contains(t) && !laufend.contains(t)),
        )
        .collect();
    let mut folge: Vec<&'r str> = Vec::new();
    let mut gesetzt_menge: HashSet<&'r str> = HashSet::new();
    let mut offen = wunsch.clone();
    for _runde in 0..=wunsch.len() {
        if offen.is_empty() {
            break;
        }
        let gesetzt: Vec<&'r str> = offen
            .iter()
            .copied()
            .filter(|t| quelle.get(t).is_none_or(|q| q.is_subset(&gesetzt_menge)))
            .collect();
        if gesetzt.is_empty() {
            break;
        }
        gesetzt_menge.extend(gesetzt.iter().copied());
        folge.extend(gesetzt.iter().copied());
        offen.retain(|t| !gesetzt.contains(t));
    }
    folge.extend(offen);
    folge
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Eine Bindung der echten Registry, umbenannt; `ableitung` je nach `regel` gesetzt oder leer.
    fn feld(id: &str, regel: Option<(&str, Option<&str>)>) -> Bindung {
        let reg = crate::doctest_registry().expect("registry");
        let vorlage = reg
            .dateien
            .iter()
            .flat_map(|(_, d)| &d.bindungen)
            .find(|b| b.feld_id == "kind_unter_14_haushaltszugehoerig")
            .expect("Vorlage")
            .clone();
        let mut b = vorlage.clone();
        b.feld_id = id.to_owned();
        b.ableitung = regel.map(|(aus, und)| {
            let mut a = vorlage
                .ableitung
                .clone()
                .expect("Vorlage traegt eine ableitung");
            a.aus = aus.to_owned();
            a.und_feld = und.map(str::to_owned);
            a
        });
        b
    }

    fn ids(v: &[&Bindung]) -> Vec<String> {
        v.iter().map(|b| b.feld_id.clone()).collect()
    }

    /// Fluss J F1 (2026-10-21): Welche `uebernehmbar`-Felder des Bestands nach der Vorjahres-Uebernahme eine Frage bleiben, folgt
    /// der Bindung und steht in keiner Liste von Hand. Der Test haelt den Bestand fest, OHNE den Pflicht-Kegel einer Scheibe
    /// (den prueft `rust/api/tests/oberflaeche/fluss_j.rs` am Browser): Faengt eine neue Bindung ein Gate oder einen Ausloeser
    /// ein, faellt er rot, und die Liste im Bericht von Fluss J ist zu ergaenzen. `steuerklasse` und `versicherungsart` gehoeren
    /// nicht dazu: an ihrer bestaetigten Antwort haengt weder eine Regel noch eine Ableitung.
    #[test]
    fn diese_uebernehmbaren_felder_bleiben_im_bestand_eine_frage() {
        let reg = crate::doctest_registry().expect("registry");
        let g = Graph::aus_registry(&reg);
        let sicht = g.alle();
        let noetig = braucht_bestaetigung(sicht, &g, &[]);
        let mut liste: Vec<&str> = sicht
            .iter()
            .filter(|b| b.askable && b.vorjahr == Some(Vorjahr::Uebernehmbar) && noetig.contains(b.feld_id.as_str()))
            .map(|b| b.feld_id.as_str())
            .collect();
        liste.sort_unstable();
        assert_eq!(
            liste,
            [
                "kind_geburtsdatum",
                "rentner_hinterbliebenenbezuege",
                "stammdaten_geburtsdatum",
                "stammdaten_geburtsdatum_partner",
                "veranlagung",
                "vpf_auswaertige_taetigkeit",
            ]
        );
        for hat_nichts_festzuhalten in ["steuerklasse", "versicherungsart"] {
            assert!(!noetig.contains(hat_nichts_festzuhalten), "{hat_nichts_festzuhalten}");
        }
    }

    /// Gegenstueck zu `test_nachzuegler_behalten_ihre_reihenfolge_und_gehen_nicht_verloren`.
    #[test]
    fn nachzuegler_behalten_ihre_reihenfolge_und_gehen_nicht_verloren() {
        let regel = Some(("q", Some("u")));
        let (z1, z2) = (feld("z1", regel), feld("z2", regel));
        let (v, q, u, w) = (
            feld("v", None),
            feld("q", None),
            feld("u", None),
            feld("w", None),
        );
        let ordnung = |gruppe: Vec<&Bindung>| ids(&nach_ausloesern(gruppe));
        assert_eq!(
            ordnung(vec![&z1, &z2, &v, &q, &u, &w]),
            ["v", "q", "u", "z1", "z2", "w"]
        );
        // Steht schon alles richtig, aendert sich nichts.
        assert_eq!(
            ordnung(vec![&q, &u, &z1, &z2, &v, &w]),
            ["q", "u", "z1", "z2", "v", "w"]
        );
        // Fehlt ein Ausloeser in der Gruppe, bleibt das Ziel stehen oder folgt dem anderen.
        assert_eq!(ordnung(vec![&z1, &v, &q]), ["v", "q", "z1"]);
        assert_eq!(ordnung(vec![&z1, &v, &w]), ["z1", "v", "w"]);
    }

    /// Ein Ausloeser mit eigener `ableitung` zaehlt nicht: keine Kette, nichts geht verloren.
    #[test]
    fn ausloeser_mit_eigener_ableitung_zaehlt_nicht() {
        let (a, b) = (feld("a", Some(("b", None))), feld("b", Some(("a", None))));
        assert_eq!(ids(&nach_ausloesern(vec![&a, &b])), ["a", "b"]);
    }

    /// Eine `feld_bedingung` mit `feld` und dieser einen Art (die uebrigen Arten `None`).
    fn bedingung(feld: &str, praefix_nicht: Option<&str>, wert: Option<serde_json::Value>) -> FeldBedingung {
        FeldBedingung {
            feld: feld.to_owned(),
            wert,
            grund: "Begruendung fuer den Test mit mehr als vierzig Zeichen.".to_owned(),
            wert_nicht: None,
            alter_im_vz: None,
            praefix_nicht: praefix_nicht.map(str::to_owned),
            groesser_als: None,
            beleg: false,
            und: None,
            oder: None,
        }
    }

    /// Fluss J F4: Das Feld einer `praefix_nicht`-Bedingung steht vor dem Feld, das an ihr haengt (die IBAN vor dem BIC). In der
    /// Vordruck-Ordnung stand der BIC (Kz `E0102201`) vor der IBAN (kein Kz), und die Frage nach ihm kam, bevor die IBAN sie
    /// erledigen konnte. Dasselbe gilt fuer das Feld einer `wert`-Bedingung (Vault: `decisions/eine-folgefrage-mit-wert-bedingung-
    /// steht-hinter-ihrem-ausloeser-im-selben-thema`): das Alter bei Versorgungsbeginn steht hinter der Art. KONTROLLE: jede andere
    /// Art von `feld_bedingung` ordnet nichts um (das Bundesland stand vor der Konfession, an der seine `wert_nicht`-Bedingung
    /// haengt, Fluss I Befund B; seit B1 steht die Konfession als Eingangsfrage davor).
    #[test]
    fn das_feld_einer_praefix_bedingung_steht_vor_dem_feld_das_an_ihr_haengt() {
        let mut bic = feld("bic", None);
        bic.feld_bedingung = Some(bedingung("iban", Some("DE"), None));
        let (iban, v) = (feld("iban", None), feld("v", None));
        assert_eq!(ids(&nach_ausloesern(vec![&bic, &v, &iban])), ["v", "iban", "bic"]);
        // Steht schon alles richtig, aendert sich nichts. Fehlt das Feld der Bedingung in der Gruppe, bleibt der BIC stehen.
        assert_eq!(ids(&nach_ausloesern(vec![&iban, &bic, &v])), ["iban", "bic", "v"]);
        assert_eq!(ids(&nach_ausloesern(vec![&bic, &v])), ["bic", "v"]);
        // `wert`: das Ziel rueckt hinter den Ausloeser; steht schon alles richtig oder fehlt der Ausloeser, bleibt es stehen.
        let mut land = feld("land", None);
        land.feld_bedingung = Some(bedingung("iban", None, Some(serde_json::json!("x"))));
        assert_eq!(ids(&nach_ausloesern(vec![&land, &v, &iban])), ["v", "iban", "land"]);
        assert_eq!(ids(&nach_ausloesern(vec![&iban, &land, &v])), ["iban", "land", "v"]);
        assert_eq!(ids(&nach_ausloesern(vec![&land, &v])), ["land", "v"]);
        // KONTROLLE `wert_nicht`: ordnet nichts um.
        let mut nicht = feld("nicht", None);
        let mut bed = bedingung("iban", None, None);
        bed.wert_nicht = Some(serde_json::json!("x"));
        nicht.feld_bedingung = Some(bed);
        assert_eq!(ids(&nach_ausloesern(vec![&nicht, &v, &iban])), ["nicht", "v", "iban"]);
    }

    /// Keine Ketten (Entscheidung Punkt 3): Ein Ausloeser, der selbst hinter einen anderen rueckt, zaehlt nicht. Sonst stuende sein
    /// Nachzuegler hinter einem Platz, den der Ausloeser verlaesst, und ginge verloren. `a` haengt per `wert` an `b`, `b` an `c`:
    /// nur `b` rueckt hinter `c`, `a` bleibt stehen, alle drei Felder bleiben in der Gruppe.
    #[test]
    fn ein_wert_ausloeser_der_selbst_nachrueckt_zaehlt_nicht() {
        let (mut a, mut b, c) = (feld("a", None), feld("b", None), feld("c", None));
        a.feld_bedingung = Some(bedingung("b", None, Some(serde_json::json!("x"))));
        b.feld_bedingung = Some(bedingung("c", None, Some(serde_json::json!("x"))));
        assert_eq!(ids(&nach_ausloesern(vec![&a, &b, &c])), ["a", "c", "b"]);
    }

    /// Ein Ereignis fuer `feld_id` mit dem rohen Wert `wert`, am Store vorbei geschrieben (die Bindung prueft hier nichts).
    fn ereignis(feld_id: &str, wert: PyWert, bestaetigt: bool) -> store::Event {
        use domain::{Achsenwert, Herkunft, PruefTiefe, Schreiber, Zustand};
        let mut e = store::Event {
            event_id: store::EventId::aus_bytes([0; 32]),
            ts: "2026-10-07T10:00:00Z".to_owned(),
            feld_id: feld_id.to_owned(),
            wert,
            zustand: if bestaetigt { Zustand::Bestaetigt } else { Zustand::Vorlaeufig },
            herkunft: Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }
            .into(),
            schreiber: Schreiber::Mensch("julius".to_owned()),
            signal: None,
            ersetzt: None,
        };
        e.event_id = e.berechne_event_id().unwrap();
        e
    }

    /// Steht die Frage `feld` in der Queue eines Falls (VZ 2025), der nur `geburtsjahr` kennt?
    fn frage_steht(feld: &str, geburtsjahr: Option<(PyWert, bool)>) -> bool {
        let antworten = geburtsjahr
            .into_iter()
            .map(|(w, b)| ("geburtsjahr", w, b))
            .collect();
        frage_steht_bei(feld, antworten)
    }

    /// Steht die Frage `feld` in der Queue eines Falls (VZ 2025), der nur diese `antworten` `(Feld, Wert, bestaetigt)` kennt?
    fn frage_steht_bei(feld: &str, antworten: Vec<(&str, PyWert, bool)>) -> bool {
        frage_steht_in_sicht(feld, None, antworten)
    }

    /// Wie [`frage_steht_bei`], aber die Scheibe fragt das Feld `ohne` nicht: ihre Sicht laesst es weg. Eine Antwort darauf kann
    /// trotzdem im Store stehen (Seitenkanal, von Hand geaenderte Akte).
    fn frage_steht_in_sicht(feld: &str, ohne: Option<&str>, antworten: Vec<(&str, PyWert, bool)>) -> bool {
        queue_in_sicht(ohne, antworten).iter().any(|f| f == feld)
    }

    /// Die ganze Queue (VZ 2025, in der Reihenfolge von [`naechste_fragen`]) eines Falls, der nur diese `antworten` kennt; die
    /// Scheibe fragt das Feld `ohne` nicht.
    fn queue_in_sicht(ohne: Option<&str>, antworten: Vec<(&str, PyWert, bool)>) -> Vec<String> {
        let reg = crate::doctest_registry().expect("registry");
        let g = Graph::aus_registry(&reg);
        let sicht = Sicht::aus(g.alle().iter().filter(|b| Some(b.feld_id.as_str()) != ohne));
        let events = antworten
            .into_iter()
            .map(|(f, w, b)| ereignis(f, w, b))
            .collect();
        let s = Store::aus_datei(store::StoreDatei {
            version: 1,
            veranlagungszeitraum: store::Veranlagungsjahr(2025),
            fall_id: None,
            scheibe: None,
            user_id: None,
            events,
            snapshots: Vec::new(),
            vorjahr_referenz: None,
        });
        let ohne_beitrag: Option<&HashMap<String, i64>> = None;
        naechste_fragen(&s, &sicht, &g, ohne_beitrag)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// Die Frage nach dem Alter bei Versorgungsbeginn des Partners hing an der Art (`wert`: `altersgrenze_sonstige`), stand aber
    /// DAVOR (Fragen-Golden Fall 3 und 8): wer der Liste folgte, beantwortete sie zu frueh und bei den drei anderen Arten umsonst.
    /// Am Aufrufort: `naechste_fragen` mit der echten Registry, Zusammenveranlagung bestaetigt (Entscheidung `eine-folgefrage-mit-
    /// wert-bedingung-steht-hinter-ihrem-ausloeser-im-selben-thema`, Punkt 5). Die Menge der Fragen aendert sich nicht.
    #[test]
    fn folgefrage_steht_hinter_ihrem_ausloeser() {
        const ART: &str = "versorgung_art_partner";
        const ALTER: &str = "versorgung_alter_bei_beginn_partner";
        let zusammen = || ("veranlagung", PyWert::Text("zusammen".to_owned()), true);
        let art = |w: &str| (ART, PyWert::Text(w.to_owned()), true);
        let ort = |q: &[String], f: &str| q.iter().position(|x| x == f).unwrap_or_else(|| panic!("{f} fehlt in der Queue"));

        let q = queue_in_sicht(None, vec![zusammen()]);
        assert!(ort(&q, ART) < ort(&q, ALTER), "die Art steht vor dem Alter, das an ihr haengt");
        assert_eq!(q.iter().filter(|f| *f == ALTER).count(), 1, "das Alter steht genau einmal");
        // KONTROLLE: die Art ist eine Frage und schaltet das Alter nur bei `altersgrenze_sonstige` frei.
        let q = queue_in_sicht(None, vec![zusammen(), art("altersgrenze_sonstige")]);
        assert!(q.iter().any(|f| f == ALTER), "bei altersgrenze_sonstige bleibt das Alter eine Frage");
        let q = queue_in_sicht(None, vec![zusammen(), art("beamtenrechtlich")]);
        assert!(!q.iter().any(|f| f == ALTER), "bei einer anderen Art entfaellt das Alter");
    }

    /// `feld_bedingung.alter_im_vz` (Abweichung Nr. 32): die Frage nach dem 55. Geburtstag steht nur, wenn das BESTAETIGTE
    /// Geburtsjahr im Veranlagungsjahr genau 55 Jahre ergibt (2025 − 1970). Jedes andere bestaetigte Ganzzahl-Geburtsjahr
    /// schliesst sie aus, auch am Rand von `i64` (kein Ueberlauf). Fail-closed: kein, ein vorlaeufiges oder ein bestaetigtes
    /// Geburtsjahr, das keine Ganzzahl ist, schliesst sie nicht aus.
    #[test]
    fn die_altersbedingung_schliesst_nur_ein_bestaetigtes_anderes_geburtsjahr_aus() {
        let frage = |gj| frage_steht("alter_55_vor_verkauf", gj);
        let bestaetigt = |w: PyWert| Some((w, true));
        let vorlaeufig = |w: PyWert| Some((w, false));
        assert!(frage(bestaetigt(PyWert::Ganz(1970))), "2025 minus 1970 ist 55");
        for anders in [1969, 1971, 1900, 2010, 0, -1] {
            assert!(!frage(bestaetigt(PyWert::Ganz(anders))), "geboren {anders}");
        }
        for rand in [i64::MIN, i64::MAX] {
            assert!(!frage(bestaetigt(PyWert::Ganz(rand))), "geboren {rand}: kein Ueberlauf, kein 55. Jahr");
        }
        assert!(frage(None), "ohne Geburtsjahr bleibt die Frage");
        assert!(frage(vorlaeufig(PyWert::Ganz(1971))), "ein vorlaeufiges Geburtsjahr schliesst nicht aus");
        assert!(frage(vorlaeufig(PyWert::Ganz(1970))), "ein vorlaeufiges Geburtsjahr im Jahr 55 laesst die Frage stehen");
        for kein_jahr in [PyWert::Text("1971".to_owned()), PyWert::Gleit(1971.0), PyWert::Bool(false), PyWert::Null] {
            assert!(
                frage(bestaetigt(kein_jahr.clone())),
                "ein bestaetigter Wert, der keine Ganzzahl ist ({kein_jahr:?}), schliesst die Frage nicht aus"
            );
        }
    }

    /// `feld_bedingung.groesser_als` mit `und` (Abweichung Nr. 34): die Anteilsfrage des Schulgelds steht nur bei
    /// Einzelveranlagung UND bestaetigtem Schulgeld ueber 0. Anders als die uebrigen Arten gilt hier POSITIVER BEWEIS: Schweigen,
    /// ein vorlaeufiger Wert und jeder Wert, der keine Zahl ueber 0 ist, lassen die Frage nicht stehen. Mit zwei Kindern genuegt
    /// eines; die Queue fuehrt Felder, nicht Kinder.
    #[test]
    fn die_betragsbedingung_verlangt_bestaetigtes_schulgeld_ueber_0_und_einzelveranlagung() {
        const ANTEIL: &str = "kind_schulgeld_aufteilung_prozent";
        let einzel = || ("veranlagung", PyWert::Text("einzel".to_owned()), true);
        let kinder = |n| ("fam_anzahl_kinder", PyWert::Ganz(n), true);
        let steht = |antworten: Vec<(&str, PyWert, bool)>| frage_steht_bei(ANTEIL, antworten);
        let kind1 = |w: PyWert, b: bool| ("schulgeld", w, b);
        let kind2 = |w: PyWert, b: bool| ("schulgeld__2", w, b);

        // Der Schnitt bei 0, Rand von i64 ohne Ueberlauf.
        for (wert, erwartet) in [
            (PyWert::Ganz(1), true),
            (PyWert::Ganz(300_000), true),
            (PyWert::Ganz(i64::MAX), true),
            (PyWert::Ganz(0), false),
            (PyWert::Ganz(-1), false),
            (PyWert::Ganz(i64::MIN), false),
        ] {
            assert_eq!(
                steht(vec![kinder(1), einzel(), kind1(wert.clone(), true)]),
                erwartet,
                "bestaetigtes Schulgeld {wert:?}"
            );
        }
        // Kein Beleg: Schweigen, ein vorlaeufiger Wert, ein Wert, der keine Ganzzahl ist (auch eine Kommazahl ueber 0).
        assert!(!steht(vec![kinder(1), einzel()]), "ohne Schulgeld");
        assert!(!steht(vec![kinder(1), einzel(), kind1(PyWert::Ganz(300_000), false)]), "vorlaeufig");
        for kein_betrag in [
            PyWert::Text("300000".to_owned()),
            PyWert::Bool(true),
            PyWert::Null,
            PyWert::Gleit(300_000.5),
        ] {
            assert!(
                !steht(vec![kinder(1), einzel(), kind1(kein_betrag.clone(), true)]),
                "ein bestaetigter Wert, der keine Ganzzahl ist ({kein_betrag:?}), belegt kein Schulgeld"
            );
        }
        // Die erste Bedingung (`und`): Zusammenveranlagung schliesst aus, Schweigen auf die Veranlagung nicht.
        let zusammen = ("veranlagung", PyWert::Text("zusammen".to_owned()), true);
        let schulgeld = || kind1(PyWert::Ganz(300_000), true);
        assert!(!steht(vec![kinder(1), zusammen, schulgeld()]), "Zusammenveranlagung");
        assert!(steht(vec![kinder(1), schulgeld()]), "Veranlagung unbeantwortet");
        assert!(steht(vec![kinder(1), einzel(), schulgeld()]), "Einzelveranlagung");
        // Zwei Kinder: ein bestaetigtes Schulgeld ueber 0 genuegt, gleich bei welchem Kind; ein offenes Kind ohne Beleg nicht.
        let (null, viel) = (PyWert::Ganz(0), PyWert::Ganz(300_000));
        assert!(steht(vec![kinder(2), einzel(), kind1(viel.clone(), true), kind2(null.clone(), true)]), "Kind 1");
        assert!(steht(vec![kinder(2), einzel(), kind1(null.clone(), true), kind2(viel.clone(), true)]), "Kind 2");
        assert!(steht(vec![kinder(2), einzel(), kind2(viel.clone(), true)]), "Kind 2, Kind 1 offen");
        assert!(!steht(vec![kinder(2), einzel(), kind1(null.clone(), true), kind2(null.clone(), true)]), "beide 0");
        assert!(!steht(vec![kinder(2), einzel(), kind1(null.clone(), true)]), "Kind 1 0, Kind 2 offen");
        assert!(!steht(vec![kinder(2), einzel(), kind1(viel, false), kind2(null, true)]), "Kind 1 nur vorlaeufig");
    }

    /// Eine Bedingung auf ein Feld, das die Scheibe nicht fragt, schliesst nichts aus (Rust-eigen, Abweichung Nr. 74). Das Kreuz
    /// `kein_lohn_pension_partner` nimmt `bruttoarbeitslohn_partner` aus der Queue, wenn die Scheibe das Kreuz fragt. Fragt sie es
    /// nicht (`gesamt`, `an_gesamt`), kann der Nutzer es dort nie setzen oder zuruecknehmen, und der Guard verlangt die Lohnfrage:
    /// sie bleibt. Die Betragsbedingung (`groesser_als`) bleibt ein Beleg: ohne das Feld in der Scheibe gibt es keinen, die
    /// Folgefrage steht nicht.
    #[test]
    fn eine_bedingung_auf_ein_feld_ausserhalb_der_scheibe_schliesst_nichts_aus() {
        const LOHN: &str = "bruttoarbeitslohn_partner";
        const KREUZ: &str = "kein_lohn_pension_partner";
        const ANTEIL: &str = "kind_schulgeld_aufteilung_prozent";
        let kreuz = || (KREUZ, PyWert::Bool(true), true);
        assert!(frage_steht_in_sicht(LOHN, None, vec![]), "KONTROLLE: ohne Antwort steht die Lohnfrage");
        assert!(!frage_steht_in_sicht(LOHN, None, vec![kreuz()]), "KONTROLLE: das Kreuz in der Scheibe verbirgt die Lohnfrage");
        assert!(frage_steht_in_sicht(LOHN, Some(KREUZ), vec![kreuz()]), "das Kreuz gehoert nicht zur Scheibe: die Lohnfrage bleibt");
        // Ein vorlaeufiges Kreuz schloss auch vorher nichts aus; es bleibt so.
        assert!(frage_steht_in_sicht(LOHN, None, vec![(KREUZ, PyWert::Bool(true), false)]), "vorlaeufiges Kreuz");

        // Die Betragsbedingung: Beleg ueber das Feld, nicht ueber seine Sicht.
        let basis = || {
            vec![
                ("fam_anzahl_kinder", PyWert::Ganz(1), true),
                ("veranlagung", PyWert::Text("einzel".to_owned()), true),
            ]
        };
        let mut mit_schulgeld = basis();
        mit_schulgeld.push(("schulgeld", PyWert::Ganz(300_000), true));
        assert!(frage_steht_in_sicht(ANTEIL, None, mit_schulgeld), "KONTROLLE: mit bestaetigtem Schulgeld steht die Anteilsfrage");
        assert!(
            !frage_steht_in_sicht(ANTEIL, Some("schulgeld"), basis()),
            "ohne das Feld in der Scheibe und ohne Beleg steht die Folgefrage nicht"
        );
    }

    /// `beleg: true` (Fragen-Reduktion Teil 2): Die Frage steht nur bei BESTAETIGTER Antwort, die die Bedingung erfuellt. Schweigen,
    /// ein vorlaeufiger Wert und der ausschliessende Wert lassen sie weg, bei `wert_nicht` (Konfession) wie bei `wert` ("ja"). Fragt
    /// die Scheibe das Feld der Bedingung nicht, gibt es nie einen Beleg: die Frage bleibt (Abweichung Nr. 74), auch gegen eine
    /// Antwort darauf im Store.
    #[test]
    fn beleg_verlangt_eine_bestaetigte_erfuellte_antwort_und_ein_gefragtes_feld() {
        const ARBEITGEBER: &str = "kirchensteuer_arbeitgeber";
        const ZAEHLER: &str = "hh_anzahl_handwerker";
        const KONFESSION: &str = "kist_konfession";
        const JA_NEIN: &str = "hh_hat_aufwendungen";
        let konfession = |w: &str, bestaetigt: bool| vec![(KONFESSION, PyWert::Text(w.to_owned()), bestaetigt)];
        let ja_nein = |w: bool, bestaetigt: bool| vec![(JA_NEIN, PyWert::Bool(w), bestaetigt)];

        // wert_nicht: keine
        assert!(!frage_steht_bei(ARBEITGEBER, vec![]), "Schweigen");
        assert!(!frage_steht_bei(ARBEITGEBER, konfession("evangelisch", false)), "vorlaeufige Konfession");
        assert!(!frage_steht_bei(ARBEITGEBER, konfession("keine", true)), "keine");
        assert!(frage_steht_bei(ARBEITGEBER, konfession("evangelisch", true)), "bestaetigt evangelisch");
        assert!(frage_steht_bei(ARBEITGEBER, konfession("roemisch-katholisch", true)), "bestaetigt katholisch");
        // wert: true
        assert!(!frage_steht_bei(ZAEHLER, vec![]), "Schweigen");
        assert!(!frage_steht_bei(ZAEHLER, ja_nein(true, false)), "vorlaeufiges ja");
        assert!(!frage_steht_bei(ZAEHLER, ja_nein(false, true)), "nein");
        assert!(frage_steht_bei(ZAEHLER, ja_nein(true, true)), "bestaetigtes ja");
        // Die Eingangsfragen selbst bleiben in der Queue und schalten die Folgefrage frei.
        assert!(frage_steht_bei(KONFESSION, vec![]), "die Konfession ist eine Frage");
        assert!(frage_steht_bei(JA_NEIN, vec![]), "die Ja/Nein-Frage ist eine Frage");
        // Das Feld der Bedingung gehoert nicht zur Scheibe: kein Beleg moeglich, die Frage bleibt.
        assert!(frage_steht_in_sicht(ARBEITGEBER, Some(KONFESSION), vec![]), "Konfession nicht in der Scheibe");
        assert!(frage_steht_in_sicht(ZAEHLER, Some(JA_NEIN), vec![]), "Ja/Nein nicht in der Scheibe");
        assert!(
            frage_steht_in_sicht(ARBEITGEBER, Some(KONFESSION), konfession("keine", true)),
            "eine Antwort auf ein Feld ausserhalb der Scheibe schliesst nichts aus"
        );
    }
}
