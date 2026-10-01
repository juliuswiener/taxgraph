//! `_feste_zahl` (`api.py:194-223`) mit benanntem Grund statt stillem `None`.
//!
//! Python liefert `None` fuer **vier verschiedene Lagen**, und `_ergebnis_roh`
//! (`api.py:587-619`) rekonstruiert sie muehsam wieder auseinander -- es ruft
//! `_vorlaeufige_ring_betraege` ein zweites Mal, baut die offene Liste neu und ruft
//! `_bescheid_fn` ein zweites Mal. Der Aufrufer rechnet also die Bedingungen des
//! Aufgerufenen nach. Ein Port, der nur `None` weitergibt, verliert drei Gruende und
//! meldet viermal dasselbe.
//!
//! Hier traegt das Ergebnis den Grund selbst: [`KeineZahl`].
//!
//! Die vier Lagen, in der Reihenfolge der Python-Zeilen:
//!
//! | # | Bedingung | Grund |
//! |---|---|---|
//! | 1 | `cfg["gesamt_ring"] is None` | [`KeineZahl::KeinScheibenGesamtbescheid`] |
//! | 2 | Kegel unvollstaendig oder nicht `bestaetigt` | [`KeineZahl::InputKegelNichtBestaetigt`] |
//! | 3 | `_vorlaeufige_ring_betraege(...)` nicht leer | [`KeineZahl::RingBetragVorlaeufig`] |
//! | 4 | `_bescheid_fn(...) is None` | [`KeineZahl::EngineUnavailable`] |
//!
//! **Reihenfolge ist Semantik.** Python prueft 1 vor 3, und `_ergebnis_roh` prueft 1
//! VOR allem anderen (`api.py:588`). Wer die Reihenfolge aendert, aendert den Grund.

use std::cell::{Cell, RefCell};

use domain::{Cent, Vz, Zustand};
use interview::Graph;
use store::Store;

use crate::BescheidFehler;

use super::Cfg;
use crate::zweige::{bescheid_fn, Extras, Umgebung};
use crate::Felder;

/// Warum es keine festgesetzte Zahl gibt. Vier Lagen, die in Python alle `None` hiessen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeineZahl {
    /// Lage 1: die Scheibe hat keinen Gesamt-Accessor (`cfg["gesamt_ring"] is None`).
    /// Nur `n_vor_gwg`. Python-Grund: `kein_scheiben_gesamtbescheid`.
    KeinScheibenGesamtbescheid,
    /// Lage 2: der Kegel ist unvollstaendig oder nicht durchgehend `bestaetigt`.
    /// Python-Grund: `input_kegel_nicht_bestaetigt`.
    InputKegelNichtBestaetigt,
    /// Lage 3: ein Ring-Betrag ausserhalb des Kegels ist genannt, aber nicht bestaetigt
    /// (Klasse C, [[klasse-c-vorlaeufiger-betrag-sperrt]]). Python-Grund:
    /// `ring_betrag_vorlaeufig`. Die betroffenen Feld-Ids stehen in [`KeineZahlGrund::felder`].
    RingBetragVorlaeufig { betraege: Vec<&'static str> },
    /// Lage 4: `_bescheid_fn` kennt die Quantitaet nicht.
    /// Python-Grund: `engine_unavailable`.
    EngineUnavailable,
}

/// [`KeineZahl`] samt den Feld-Ids, die ihn ausgeloest haben.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeineZahlGrund {
    pub grund: KeineZahl,
    /// Nur bei [`KeineZahl::RingBetragVorlaeufig`] gefuellt: die offenen Betragsfelder.
    pub felder: Vec<&'static str>,
}

/// Die festgesetzte Zahl einer Scheibe, oder der benannte Grund dagegen.
///
/// Python `_feste_zahl` gibt `(zahl_euro, solz_cent, extras)` zurueck. `solz_cent` ist
/// `Option`, weil die `slot_fn` den Slot nur bei manchen Zweigen fuellt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FesteZahl {
    /// Die festgesetzte Zahl in CENT. Python nennt sie im Docstring `zahl_euro`, die API
    /// schreibt sie aber als `zahl_cent` (`api.py:619`), und [`crate::zweige::BescheidFn`]
    /// liefert `Cent`. Der Docstring ist veraltet; der Typ folgt dem Aufrufer.
    pub zahl: Cent,
    pub solz_cent: Option<Cent>,
    /// Post-Engine-Zuschlaege (§ 51a `kist_cent`, § 101 `mobilitaetspraemie_cent`).
    /// Ein **fehlender** Schluessel heisst "nicht rechenbar", nicht "null Euro".
    pub extras: Extras,
}

/// Der Kegel nach [`interview::relevante_kegel_felder`] -- Python `BR.relevante_kegel_felder`
/// (`bindung_rollen.py:41`). Ohne `store` bleibt der volle Kegel (Alt-Aufrufer, Teil-Ringe).
///
/// Die Sicht ist die VOLLE (`graph.alle()`), nicht eine aus dem Kegel gebaute: `relevanz`
/// sammelt die Gates je Regel aus der Sicht, und die Gate-Felder stehen neben dem Kegel,
/// nicht in ihm. Eine Kegel-Sicht liesse jedes Gate fehlen und schloesse nichts aus.
fn relevanter_kegel<'k>(
    kegel: &[&'k str],
    store: Option<&Store>,
    graph: &Graph<'k>,
) -> Vec<&'k str> {
    interview::relevante_kegel_felder(kegel, graph.alle(), store, graph)
}

/// Python `_feste_zahl` (`api.py:194`). Fail-closed: die Zahl NUR bei Scheiben-Gesamt-Accessor
/// UND vollstaendig bestaetigtem Input-Kegel.
///
/// # Errors
/// Nur echte Rechenfehler ([`crate::BescheidFehler`]). Die vier Lagen ohne Zahl sind **kein**
/// Fehler, sondern [`Ok(Err(KeineZahlGrund))`] -- sie sind eine Auskunft, keine Stoerung.
///
/// ```
/// use bescheid::deklaration::{feste_zahl, Cfg, KeineZahl};
/// use bescheid::testhilfe::{felder, index, params, store};
/// use bescheid::zweige::Umgebung;
/// use domain::{Scheibe, Vz};
/// let umg = Umgebung { achsen: &[], index: index(), params: params() };
/// // Lage 1: `n_vor_gwg` hat keinen Gesamt-Accessor.
/// let leer = felder(&store(&[]));
/// let r = feste_zahl(&leer, &Cfg::fuer(Scheibe::NVorGwg), Vz::Vz2025, &[], &umg, None, None);
/// assert!(matches!(r, Ok(Err(g)) if g.grund == KeineZahl::KeinScheibenGesamtbescheid));
/// ```
#[allow(clippy::too_many_arguments)] // Python-Signatur 1:1
pub fn feste_zahl(
    felder: &Felder,
    cfg: &Cfg,
    vz: Vz,
    scheibe_felder: &[&str],
    umgebung: &Umgebung<'_>,
    store: Option<&Store>,
    graph: Option<&Graph<'_>>,
) -> Result<Result<FesteZahl, KeineZahlGrund>, crate::BescheidFehler> {
    // Lage 1 -- VOR allem anderen (Python `api.py:588` prueft sie zuerst).
    let Some(q) = cfg.gesamt_ring() else {
        return Ok(Err(KeineZahlGrund {
            grund: KeineZahl::KeinScheibenGesamtbescheid,
            felder: Vec::new(),
        }));
    };
    // Der Kegel, wie ihn `_feste_zahl` sieht. Ohne Graph bleibt er voll -- der Graph ist
    // nur die Quelle der Regel-Zuordnung, und ohne ihn schliesst `relevanz` nichts aus.
    let kegel: Vec<&str> = match graph {
        Some(g) => relevanter_kegel(scheibe_felder, store, g),
        None => scheibe_felder.to_vec(),
    };
    // Lage 2: unvollstaendig ODER nicht durchgehend bestaetigt.
    //
    // Der Laengenvergleich ist der Riegel, nicht `meet_zustand`. `meet_zustand([])` ist
    // `Bestaetigt` ("leeres Aggregat = neutral", `store.py:56`), und `0 < 0` ist falsch --
    // ohne den Vergleich gaebe ein LEERER Kegel eine Zahl ohne jede Eingabe frei.
    let zustaende: Vec<Zustand> = kegel
        .iter()
        .filter_map(|f| felder.get(*f))
        .map(|ev| ev.zustand)
        .collect();
    if zustaende.len() < kegel.len() || domain::meet_zustand(zustaende) != Zustand::Bestaetigt {
        return Ok(Err(KeineZahlGrund {
            grund: KeineZahl::InputKegelNichtBestaetigt,
            felder: Vec::new(),
        }));
    }
    // Lage 3: Klasse C -- ein vorlaeufiger Betrag AUSSERHALB des Kegels faellt in
    // `_bescheid_fn` still aus der Zahl (over-tax-safe), die Zahl hiesse trotzdem
    // "bestaetigt". Gemessen: 100.000 EUR vorlaeufiger Veraeusserungsgewinn = 23.100 EUR
    // zu wenig, ohne Signal. Dieselbe Schwelle wie fuer den Kegel.
    let betraege = super::vorlaeufige_ring_betraege(felder, cfg, umgebung.index);
    if !betraege.is_empty() {
        return Ok(Err(KeineZahlGrund {
            grund: KeineZahl::RingBetragVorlaeufig {
                betraege: betraege.clone(),
            },
            felder: betraege,
        }));
    }
    let solz_out: Cell<Option<Cent>> = Cell::new(None);
    let extras: RefCell<Extras> = RefCell::new(Extras::default());
    // Lage 4.
    let Some(bf) = bescheid_fn(
        q,
        vz,
        umgebung,
        Some(felder),
        store,
        true,
        Some(&solz_out),
        Some(&extras),
    ) else {
        return Ok(Err(KeineZahlGrund {
            grund: KeineZahl::EngineUnavailable,
            felder: Vec::new(),
        }));
    };
    let mut werte = intervall::Werte::neu();
    for f in &kegel {
        if let Some(ev) = felder.get(*f) {
            werte.setze(f, ev.wert.clone());
        }
    }
    let zahl = bf(&werte).map_err(slot_fehler)?;
    // `bf` leiht `extras` (es schreibt § 51a/§ 101 hinein). Der Borrow endet hier, nicht
    // am Ende des Blocks -- sonst laesst sich `extras` nicht auslesen.
    drop(bf);
    Ok(Ok(FesteZahl {
        zahl,
        solz_cent: solz_out.get(),
        extras: extras.into_inner(),
    }))
}

/// [`intervall::SlotFehler`] auf [`BescheidFehler`]. Der innere Fehler ist schon
/// [`BescheidFehler`] (der `BescheidFn`-Typ traegt ihn), die drei Hüllen tragen den
/// Python-Grund: `KeyError`, `TypeError`, `OverflowError`.
fn slot_fehler(f: intervall::SlotFehler<BescheidFehler>) -> BescheidFehler {
    use intervall::SlotFehler;
    match f {
        SlotFehler::UnbekanntesFeld(f) => BescheidFehler::SlotFehlt(f),
        SlotFehler::SummandNichtGanzzahl(_) => BescheidFehler::Python {
            klasse: "TypeError",
            was: "Summand ist keine Ganzzahl (Slot-Addition)",
        },
        SlotFehler::Ueberlauf(_) => BescheidFehler::Ueberlauf("feste_zahl"),
        SlotFehler::Slot(e) => e,
    }
}
