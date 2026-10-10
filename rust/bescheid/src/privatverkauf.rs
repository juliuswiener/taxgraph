//! § 23 `EStG` im Bescheid: private Veraeusserungsgeschaefte der Hauptperson (Abweichung Nr. 54 in `rust/fixtures/README.md`).
//!
//! Drei Aufgaben an einer Stelle, damit Rechnung, Sperre und Erklaerung dieselbe Lesart der Instanz teilen:
//!
//! * [`p23_privatverkauf`]: die Einkuenfte aus privaten Veraeusserungsgeschaeften fuer den Ring. Steuerfreie Verkaeufe (Frist
//!   abgelaufen, Eigennutzung, taeglicher Gebrauch) tragen nichts bei; ueber die Freigrenze des Veranlagungsjahrs entscheidet der
//!   Gesamtgewinn in Cent. Python rechnet nichts davon und sperrt jeden Verkauf; [`crate::einkuenfte::p23_ansonsten_einkuenfte`]
//!   bleibt als Parity-Gegenstueck unveraendert bestehen.
//! * [`p23_sperrgrund`]: der Guard. Ein bejahter Verkauf, den die Software nicht rechnet (Kryptowert, Beteiligung, Ehegatte,
//!   Teilnutzung, Gebaeude der letzten zehn Jahre auf einem Grundstueck mit abgelaufener Frist), sperrt mit [`Sperrgrund::P23VerkaufNichtGerechnet`]; fehlende oder unstimmige Angaben sperren mit
//!   [`Sperrgrund::P23AngabenOffen`], bei einem selbst bewohnten Verkauf innerhalb der Frist auch fehlende Wohnflaeche und Zeitraum
//!   (Zeile 36). Beides VOR jeder Zahl: ein Verkauf wird nie still weggelassen.
//! * [`p23_ohne_verkaeufe_ausserhalb_der_erklaerung`]: Frist abgelaufen (auch bei Eigennutzung) und Gegenstand des taeglichen Gebrauchs
//!   stehen NICHT in der Erklaerung (Anleitung Anlage SO, `sources/bfinv/anleitungen/anl_so_2025.txt`: die Zeilen 34 bis 57 tragen nur
//!   Verkaeufe innerhalb der Frist; Gegenstaende des taeglichen Gebrauchs "muessen Sie hier nicht angeben"). Der Schreiber in `elster`
//!   kennt die Frist nicht und truege jede Instanz ein, darum nimmt diese Funktion sie vorher aus dem Snapshot. Ein selbst bewohnter
//!   Verkauf innerhalb der Frist bleibt und steht mit Haken, Zeitraum, Flaeche und Gewinn 0 in der Erklaerung (`elster`,
//!   `P23_EIGENNUTZUNG_KZ`, `checkESt`-Fall Gx7).
use bindung::Params;
use domain::{Cent, Euro, PyWert, Sperrgrund, Vz, Zustand};
use elster::{parse_instanz, Instanz};
use engine::zugriff::teil2::privatverkauf::{
    p23_befreiung, p23_freigrenze_je_vz, p23_gewinn_nach_afa, Art, Befreiung, Eigennutzung, Tag,
    VerkaufEingabe,
};

use std::collections::BTreeSet;

use crate::{
    ist_false, ist_zusammen, positive_zahl, wert, zahl_oder_null, BescheidFehler, Felder,
    Instanzquelle,
};

/// Die Instanz-Gruppe der Verkaeufe in der Bindung (`bindung_p23_gesamt.yaml`).
const GRUPPE: &str = "p23_veraeusserung";

/// Ein gelesener, in sich stimmiger Verkauf.
struct Verkauf {
    befreiung: Befreiung,
    /// Ob die Anlage SO den Verkauf traegt: innerhalb der Frist und kein Gegenstand des taeglichen Gebrauchs.
    in_erklaerung: bool,
    /// Gewinn (positiv) oder Verlust (negativ) in Cent, nach `AfA`-Minderung der Kosten.
    gewinn: Cent,
}

fn text<'a>(inst: &'a Instanz, feld: &str) -> Option<&'a str> {
    match wert(&inst.felder, feld) {
        Some(PyWert::Text(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn bool_antwort(inst: &Instanz, feld: &str) -> Option<bool> {
    match wert(&inst.felder, feld) {
        Some(PyWert::Bool(b)) => Some(*b),
        _ => None,
    }
}

fn cent(inst: &Instanz, feld: &str) -> Result<Cent, BescheidFehler> {
    Ok(Cent::new(zahl_oder_null(wert(&inst.felder, feld))?))
}

/// Liest eine Instanz. `Ok(Err(grund))` heisst: der Verkauf ist nicht rechenbar, der Grund sagt warum. Die Pruefungen stehen in
/// der Reihenfolge, in der der Nutzer etwas tun kann: zuerst die Faelle, die die Software bewusst nicht rechnet, dann fehlende
/// oder unstimmige Angaben. `vz` `None` (Alt-Aufrufer ohne Jahr) lasst die Pruefung des Verkaufsjahres aus.
fn lese(inst: &Instanz, vz: Option<Vz>) -> Result<Result<Verkauf, Sperrgrund>, BescheidFehler> {
    use Sperrgrund::{P23AngabenOffen as offen, P23VerkaufNichtGerechnet as nicht_gerechnet};
    let art = match text(inst, "p23_veraeusserungs_typ") {
        Some("grundstueck") => Art::Grundstueck,
        Some("anderes_wg") => Art::AnderesWg,
        Some("kryptowert" | "beteiligung") => return Ok(Err(nicht_gerechnet)),
        _ => return Ok(Err(offen)),
    };
    let eigennutzung = if art == Art::Grundstueck {
        match text(inst, "p23_eigennutzung") {
            Some("nein") => Eigennutzung::Nein,
            Some("ausschliesslich") => Eigennutzung::Ausschliesslich,
            Some("letzte_drei_jahre") => Eigennutzung::LetzteDreiJahre,
            Some("teilweise") => return Ok(Err(nicht_gerechnet)),
            _ => return Ok(Err(offen)),
        }
    } else {
        Eigennutzung::Nein
    };
    // Nur beim Grundstueck gelesen (Abs. 1 Nr. 1 Satz 2): ein Gebaeude, das in den letzten zehn Jahren entstand, macht den Verkauf auch
    // nach Ablauf der Frist teilweise steuerpflichtig. Ohne Antwort ist die Angabe offen, nicht "nein".
    let gebaeude_neu = match (art, bool_antwort(inst, "p23_gebaeude_in_frist")) {
        (Art::Grundstueck, None) => return Ok(Err(offen)),
        (Art::Grundstueck, Some(b)) => b,
        (Art::AnderesWg, _) => false,
    };
    let (taeglicher_gebrauch, einkunftsquelle) = if art == Art::AnderesWg {
        match (
            bool_antwort(inst, "p23_taeglicher_gebrauch"),
            bool_antwort(inst, "p23_einkunftsquelle"),
        ) {
            (Some(t), Some(e)) => (t, e),
            _ => return Ok(Err(offen)),
        }
    } else {
        (false, false)
    };
    let (Some(anschaffung), Some(veraeusserung)) = (
        text(inst, "p23_anschaffungsdatum").and_then(Tag::aus_tt_mm_jjjj),
        text(inst, "p23_veraeusserungsdatum").and_then(Tag::aus_tt_mm_jjjj),
    ) else {
        return Ok(Err(offen));
    };
    let falsches_jahr = vz.is_some_and(|v| veraeusserung.jahr != i32::from(v.jahr()));
    if falsches_jahr || veraeusserung < anschaffung {
        return Ok(Err(offen));
    }
    let eingabe = VerkaufEingabe {
        art,
        anschaffung,
        veraeusserung,
        eigennutzung,
        taeglicher_gebrauch,
        einkunftsquelle,
    };
    let befreiung = p23_befreiung(&eingabe);
    // Die Anlage SO traegt nur Verkaeufe innerhalb der Frist, auch die selbst bewohnten (Zeilen 34 bis 57). `p23_befreiung` prueft die
    // Eigennutzung VOR der Frist; ohne sie beantwortet dieselbe Funktion, ob die Frist laeuft und der Gegenstand nicht des taeglichen
    // Gebrauchs ist.
    let in_erklaerung = p23_befreiung(&VerkaufEingabe { eigennutzung: Eigennutzung::Nein, ..eingabe })
        == Befreiung::Steuerpflichtig;
    // Ein Grundstueck mit abgelaufener Frist, ohne Eigennutzung und mit einem Gebaeude der letzten zehn Jahre: der Gebaeudeteil ist
    // steuerpflichtig. Die Aufteilung zwischen Boden und Gebaeude rechnet die Software nicht. Laeuft die Frist noch, zaehlt das ganze
    // Grundstueck; bei Eigennutzung sind Gebaeude und Boden frei: dort aendert die Antwort nichts.
    if gebaeude_neu && befreiung == Befreiung::FristAbgelaufen {
        return Ok(Err(nicht_gerechnet));
    }
    // Der Preis muss genannt sein (auch 0): ein fehlender Preis wuerde als Verlust in die Rechnung laufen und den Gesamtgewinn
    // druecken. Kosten und Werbungskosten duerfen fehlen (0): das macht den Gewinn hoeher, nie niedriger.
    if wert(&inst.felder, "p23_veraeusserungspreis")
        .and_then(PyWert::zahl_ohne_bool)
        .is_none()
    {
        return Ok(Err(offen));
    }
    let (preis, kosten, afa, wk) = (
        cent(inst, "p23_veraeusserungspreis")?,
        cent(inst, "p23_anschaffung_herstellungskosten")?,
        cent(inst, "p23_afa")?,
        cent(inst, "p23_werbungskosten")?,
    );
    // Preis und Werbungskosten sind nie negativ: ein Minus kehrte ihre Wirkung auf den Gewinn um (ein negativer Preis senkte ihn, negative
    // Werbungskosten hoben ihn). Fuer die Kosten genuegt der Guard unten.
    if preis.get() < 0 || wk.get() < 0 {
        return Ok(Err(offen));
    }
    // Die AfA mindert die Kosten (Abs. 3 Satz 4); sie ist nie negativ und nie groesser als die Kosten, die sie mindert. Negative Kosten
    // faengt `afa > kosten` mit ab: die AfA ist dann mindestens 0 und liegt darueber.
    if afa.get() < 0 || afa > kosten {
        return Ok(Err(offen));
    }
    // Ein selbst bewohnter Verkauf innerhalb der Frist steht mit Haken, Zeitraum und Flaeche in der Zeile 36 (Gewinn 0): ohne sie
    // ist die Zeile unvollstaendig und `checkESt` weist sie ab (Fall Gc1, rc=610001002). Nach Ablauf der Frist steht er nicht in der
    // Erklaerung und braucht beides nicht. Ob der Zeitraum zur Dreijahresregel passt, prueft die Software nicht.
    if befreiung == Befreiung::Eigennutzung && in_erklaerung {
        let zeitraum = (
            text(inst, "p23_eigennutzung_von").and_then(Tag::aus_tt_mm_jjjj),
            text(inst, "p23_eigennutzung_bis").and_then(Tag::aus_tt_mm_jjjj),
        );
        let flaeche = positive_zahl(wert(&inst.felder, "p23_wohnflaeche"))?;
        if flaeche.is_none() || !matches!(zeitraum, (Some(von), Some(bis)) if von <= bis) {
            return Ok(Err(offen));
        }
    }
    let gewinn = p23_gewinn_nach_afa(preis, kosten, afa, wk)?;
    Ok(Ok(Verkauf { befreiung, in_erklaerung, gewinn }))
}

/// § 23 fuer den Ring, EURO: Σ ueber alle bestaetigten Verkaeufe, die steuerpflichtig sind; dann die Freigrenze des Jahres auf den
/// Gesamtgewinn (Gewinne minus Verluste, in Cent). Ein Gesamtverlust bringt 0 (der Verlustruecktrag/-vortrag nach Satz 8 ist nicht
/// gebaut, das Finanzamt stellt ihn fest). Ohne Store oder Bindung 0. Ein Verkauf, den der Guard ([`p23_sperrgrund`]) sperrt, traegt
/// hier nichts bei; der Guard laeuft vor jeder Zahl, die einen Nutzer erreicht.
///
/// **Sicherheit:** bei `nur_bestaetigt` zaehlt eine vorlaeufige Instanz nicht.
///
/// # Errors
/// Instanz-, Accessor- und Ueberlauf-Fehler.
pub fn p23_privatverkauf(
    q: &Instanzquelle<'_>,
    vz: Vz,
    p: &Params,
) -> Result<Euro, BescheidFehler> {
    if q.beide().is_none() {
        return Ok(Euro::new(0));
    }
    let mut gesamt = Cent::new(0);
    for inst in q.instanzen(GRUPPE)? {
        if !q.zaehlt(&inst) {
            continue;
        }
        let Ok(v) = lese(&inst, Some(vz))? else {
            continue;
        };
        if v.befreiung != Befreiung::Steuerpflichtig {
            continue;
        }
        gesamt = gesamt
            .checked_add(v.gewinn)
            .ok_or(BescheidFehler::Ueberlauf("Summe der Verkaeufe"))?;
    }
    Ok(p23_freigrenze_je_vz(gesamt, vz, p)?.floor_euro())
}

/// Der Guard (K2): sperrt, solange ein bejahter Verkauf nicht rechenbar ist. `kein_p23_verkauf` = nein heisst "ich habe verkauft".
/// `None`, wenn das Kreuz nicht auf "nein" steht (nie gefragt oder "kein Verkauf" mit Betraegen faengt `FLAG_NEGIERT` in
/// `konsistenz`). Nicht nur bestaetigte Instanzen: eine vorlaeufige Instanz ist eine offene Angabe.
///
/// # Errors
/// Instanz- und Ueberlauf-Fehler.
pub(crate) fn p23_sperrgrund(
    f: &Felder,
    q: &Instanzquelle<'_>,
    vz: Option<Vz>,
) -> Result<Option<Sperrgrund>, BescheidFehler> {
    if !ist_false(wert(f, "kein_p23_verkauf")) {
        return Ok(None);
    }
    // Ehepaare mit Verkauf bleiben gesperrt: ob Freigrenze und Verlustausgleich je Ehegatte gelten, sagt der eingefrorene Text nicht.
    if ist_zusammen(f) {
        return Ok(Some(Sperrgrund::P23VerkaufNichtGerechnet));
    }
    if q.beide().is_none() {
        return Ok(Some(Sperrgrund::P23AngabenOffen));
    }
    let instanzen = q.instanzen(GRUPPE)?;
    // Ein bejahter Verkauf ohne eine einzige Instanz ist eine offene Angabe, kein "kein Verkauf".
    let mut offen = instanzen.is_empty();
    for inst in &instanzen {
        if inst.zustand != Zustand::Bestaetigt {
            offen = true;
            continue;
        }
        match lese(inst, vz)? {
            Ok(_) => {}
            // Was die Software nie rechnet, geht vor dem, was der Nutzer nachtragen kann.
            Err(Sperrgrund::P23VerkaufNichtGerechnet) => {
                return Ok(Some(Sperrgrund::P23VerkaufNichtGerechnet));
            }
            Err(_) => offen = true,
        }
    }
    Ok(offen.then_some(Sperrgrund::P23AngabenOffen))
}

/// Nimmt die bestaetigten Verkaeufe, die nicht in die Erklaerung gehoeren (Frist abgelaufen, auch bei Eigennutzung; Gegenstand des
/// taeglichen Gebrauchs), aus dem Snapshot, ehe `deklariere` ihn liest. Die Anleitung zur Anlage SO: private Veraeusserungsgeschaefte
/// sind nur Verkaeufe innerhalb der Frist (Zeilen 34 bis 42 "nicht mehr als 10 Jahre", Zeilen 52 bis 57 "nicht mehr als ein Jahr");
/// Gegenstaende des taeglichen Gebrauchs "muessen Sie hier nicht angeben". Bescheid und Ring zaehlen diese Verkaeufe schon nicht mit.
///
/// Es faellt jedes Feld der Instanz heraus (Basisname fuer Index 1, `basis__n` sonst), und nur die Felder der Gruppe
/// `p23_veraeusserung`: eine andere Gruppe mit demselben Index bleibt. Ohne Store oder Bindung geschieht nichts.
///
/// # Errors
/// Instanz- und Ueberlauf-Fehler.
pub fn p23_ohne_verkaeufe_ausserhalb_der_erklaerung(
    felder: &mut Felder,
    q: &Instanzquelle<'_>,
    vz: Vz,
) -> Result<(), BescheidFehler> {
    let Some((_, bindung)) = q.beide() else {
        return Ok(());
    };
    let mut weg = BTreeSet::new();
    for inst in q.instanzen(GRUPPE)? {
        if inst.zustand != Zustand::Bestaetigt {
            continue;
        }
        if let Ok(v) = lese(&inst, Some(vz))? {
            if !v.in_erklaerung {
                weg.insert(inst.index);
            }
        }
    }
    felder.retain(|feld_id, _| {
        let (basis, index) = parse_instanz(feld_id).unwrap_or((feld_id.as_str(), 1));
        let in_gruppe = bindung
            .get(basis)
            .is_some_and(|b| b.instanz_gruppe.as_deref() == Some(GRUPPE));
        !(in_gruppe && weg.contains(&index))
    });
    Ok(())
}
