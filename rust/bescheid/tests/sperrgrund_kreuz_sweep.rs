//! Abweichung Nr. 71: kein Kreuz darf ein Feld aus der Warteschlange nehmen, das der Guard `an_gesamt_sperrgrund`
//! weiter verlangt (Gegenstueck zu `tests/test_feld_bedingung_sperrgrund_widerspruch.py`, 4 Tests). Standardlauf, ohne
//! Python, ohne `ERiC`, ohne Netz.
//!
//! **Worum es geht.** Manche Fragen entfallen, wenn der Nutzer ein Kreuz setzt ("ich habe keine Einkuenfte dieser Art").
//! Der Guard, der vor der Abgabe prueft, ob der Bescheid gerechnet werden darf, kann zugleich verlangen, dass genau diese
//! Frage beantwortet ist. Dann sperrt er fuer immer: die Frage kommt nie wieder, und der Nutzer kann sie nicht
//! beantworten (eine Sackgasse). Die Python-Fassung las den Quelltext des Guards und verglich die Feldnamen. Der Guard
//! ist in Rust eine Folge von Pruefungen, keine Menge; ein Quelltext-Vergleich waere eine vierte, fragile Darstellung
//! derselben Verzweigung. Dieser Test misst das Verhalten: er beantwortet den Fragebogen wie ein Nutzer, setzt Kreuze
//! und fragt den Guard.
//!
//! **Warum es zaehlt.** Geld- und Datenbezug. Ein Pflichtfeld, das still aus der Warteschlange faellt, laesst die
//! Abgabe fuer immer gesperrt, oder der Nutzer gibt einen Wert an, den die Oberflaeche nie erfragt hat. Bis zu dieser
//! Datei haengte an der Rust-Seite nur der Zaehl-Pin `die_fragenzahl_vor_und_nach_den_antworten` und der Golden-Master:
//! ein Kreuz an einem vom Guard verlangten Feld (Mutanten S01 bis S03 der Nachmessung Nr. 68) faellt dort nur auf, weil
//! sich eine Zahl aendert.
//!
//! **Wo es sitzt.** Der Guard ist `deklaration/sperre.rs` samt `sperre/*.rs`; die Warteschlange ist
//! `interview::naechste_fragen`; die Kreuze sind die `feld_bedingung`en und `regel_bedingungen` der Bindung
//! (`rust/bindung/daten`). Beide Seiten werden hier so verdrahtet, wie der Dienst sie verdrahtet (`api::Zustand::
//! scheibe_bindung`): Sicht der Scheibe, Graph aller Dateien, Index der Scheibe fuer den Guard.
//!
//! **Das Verfahren.** Ein Lauf beantwortet die Warteschlange, bis sie nichts Neues mehr fragt (nach jedem Steuerfeld wird
//! sie neu gelesen). Die Antwort ist der Abwesenheitswert ("zu": Kreuze "nein", Betraege 0) oder, fuer die Felder, die
//! eine geoeffnete Gruppe neu fragt, der Beispielwert ("offen"). Daraus entstehen je Scheibe (`gesamt`, `rentner_gesamt`,
//! `an_gesamt`) und Veranlagung (einzeln, zusammen) mehrere KONTEXTE: der Grundzustand und je Steuerfeld und Wert, der
//! eine Gruppe oeffnet, einer. Dann geht jedes Kreuz auf jeden Wert, den eine Bedingung nennt: (a) vor der ersten Frage,
//! (b) nachdem das erste der entfallenden Felder beantwortet ist, (c) nachdem alle bis auf eines beantwortet sind. Der
//! Endstand ist eine SACKGASSE, wenn der Guard sperrt und Antworten auf Felder, die die Scheibe fragen koennte, die
//! Sperre aufloesen (kleinste solche Menge: [`Treffer::felder`]). Eine harte Sperre der geoeffneten Gruppe (ring-
//! unfaehige Einkunftsart) bleibt ohne Wirkung: sie loest keine Antwort auf.
//!
//! **Bekannte Sackgassen** ([`BEKANNT`]) sind die offenen Defekte; die Tabelle ist leer. Drei Klassen standen hier und sind
//! behoben: `kein_sonstige_partner` (Python, nur ueber einen Seitenkanal, in `rentner_gesamt`) und `kein_lohn_pension_partner`
//! (Rust-eigen seit Abweichung Nr. 46, Seitenkanal in `gesamt` und `an_gesamt`) hatten eine Ursache: eine Bedingung auf ein Kreuz,
//! das die Scheibe nicht fragt (`interview::fragen::bedingung_schliesst_aus`). `kein_kap_partner` (Python `BEKANNTE_TREFFER`, in
//! `gesamt`) sass im Guard: Abweichung Nr. 96 (Julius d-47) laesst die Sperre `partner_kegel_offen` die fuenf Kapital-Felder des
//! Ehegatten nicht mehr verlangen, wenn das Kreuz bestaetigt „keine“ sagt (`sperre/gesamt.rs`). Eine neue Klasse macht den Test rot;
//! ein behobener Defekt macht `die_bekannten_sackgassen_werden_noch_gesehen` rot, der Eintrag in [`BEKANNT`] faellt dann weg.
//!
//! **Grenzen** (jede mit einem Mutanten gemessen, siehe `REWRITE_PLAN.md` Nr. 71). Der Guard-Zweig braucht den passenden
//! Zustand; der Sweep setzt Beispielwerte und sieht nur, was diese Werte erreichen. Er sieht nicht: (1) Zweige, die eine
//! bestimmte Wertekombination brauchen (mehr als 3 Monate am Ort UND keine Tage nach der Frist, Arbeitsmittel ueber
//! 800 EUR UND Anschaffungsjahr ja); (2) Felder in Kind-Instanzen (`kind_betreuung_*`): der Sweep legt keine Instanzen an;
//! (3) den Wechsel des Kontextfelds selbst (erst `kein_gewinn` = nein und die Gruppe ausfuellen, dann auf ja zurueck): der
//! Sweep setzt das Kreuz nur auf Felder, die der Kontext nicht vorgibt; (4) Gruppen, die erst zusammen sperren (er oeffnet
//! jede einzeln); (5) alles, was der Guard erst HINTER einer harten Sperre der geoeffneten Gruppe prueft. Er prueft den
//! Guard `an_gesamt_sperrgrund`, nicht `unvollstaendig` der Deklaration.
//!
//! **S03 (`tage_24h`).** Der Guard verlangt das Feld selbst nie. Das Kreuz verschiebt aber die Reihenfolge der Fragen: die
//! Zu-Antwort `false` auf das Gate `vpf_frist_nicht_unterbrochen` kommt dann vor `vpf_monate_am_ort` und nimmt die Regel
//! `p9_4a_verpflegungsmehraufwand` weg, waehrend die schon gegebenen Tage den Guard weiter ausloesen. Der Sweep meldet das
//! (Klasse `AnGesamt verpflegung_dreimonatsfrist_aufteilung_offen`). Die Ursache liegt in der Reihenfolge, nicht im Kreuz:
//! eine Gate-Antwort nimmt Folgefelder weg, frueher gegebene Antworten loesen den Guard weiter aus. Der Sweep prueft diese
//! Klasse nicht gezielt, er sieht sie nur, wo die Reihenfolge der Warteschlange sie erzeugt. Der Mechanismus ist aus der
//! Antwortfolge des Treffers gelesen, nicht an einer Nutzer-Sitzung nachgestellt.
//!
//! **Abweichung Nr. 74 an allen Eintraegen.** Die Regel in `bedingung_schliesst_aus` (eine Bedingung auf ein Feld, das die Scheibe
//! nicht fragt, schliesst nichts aus) steht allgemein im Code und wirkt an jedem solchen Eintrag der Bindung, nicht nur an den
//! zwei Lohn-Kreuzen. Der Abschnitt "Abweichung Nr. 74 an allen Eintraegen" unten zaehlt diese Eintraege aus der Bindung (37:
//! 18 in `gesamt` und 5 in `an_gesamt`, alle an den zwei Lohn-Kreuzen, 14 in `rentner_gesamt`; vorher 48 und 25, bis `rentner_gesamt` am
//! 2026-10-21 `kein_gewinn_partner` fragte), haelt sie gegen die Liste [`GEPRUEFT`] und misst
//! an jedem: die Frage bleibt stehen, auch wenn die Akte das Bedingungsfeld mit einem schliessenden Wert tragen wuerde.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::disallowed_types
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use bescheid::deklaration::{an_gesamt_sperrgrund, feste_zahl, Cfg};
use bescheid::zweige::Umgebung;
use bescheid::{BindungIndex, Instanzquelle};
use bindung::{Bindung, FeldBedingung, Registry};
use domain::{Achsenwert, Feldtyp, Herkunft, HerkunftVektor, PruefTiefe, PyWert, Scheibe, Vz, Zustand};
use intervall::AchsenBindung;
use interview::{Graph, Sicht};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const SCHEIBEN: [Scheibe; 3] = [Scheibe::Gesamt, Scheibe::RentnerGesamt, Scheibe::AnGesamt];
const VERANLAGUNGEN: [&str; 2] = ["einzel", "zusammen"];

/// Die bekannten Sackgassen: (Scheibe, nur ueber Seitenkanal, Sperrgrund, Felder, deren Antwort die Sperre aufloest).
/// Die Felder sind der Kern; der Sweep darf weitere nennen (die kleinste Menge ist nicht eindeutig).
const BEKANNT: &[(&str, bool, &str, &[&str])] = &[];

// ---------------------------------------------------------------- die Welt: Bindung + Graph

struct Welt {
    registry: &'static Registry,
    graph: Graph<'static>,
    nachschlag: HashMap<String, &'static Bindung>,
    /// Felder, an denen eine Bedingung haengt (`feld_bedingung`, `regel_bedingungen`, `ableitung`), und die Screening-Fragen.
    steuer: BTreeSet<&'static str>,
    /// Feld -> die Felder, deren Fragen oder Antworten an ihm haengen (Bedingung, Regelbedingung, Ableitung, Instanzzahl).
    abhaengig: BTreeMap<String, BTreeSet<String>>,
}

fn lade() -> Registry {
    bindung::lade_registry_der_wurzel(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn baue_welt(registry: &'static Registry) -> Welt {
    let alle: Vec<&Bindung> = registry
        .dateien
        .iter()
        .flat_map(|(_, d)| &d.bindungen)
        .collect();
    let mut steuer = BTreeSet::new();
    let mut abhaengig: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for b in &alle {
        // Eine Screening-Frage ("kein_*" ja/nein) ist auch dann ein Kontext, wenn heute keine Bedingung sie nennt: der Guard
        // liest sie (`verneint("kein_unterhalt")`) und verlangt dann die Felder dahinter.
        if b.askable && b.typ == Feldtyp::Bool && (b.feld_id.starts_with("kein_") || b.feld_id.starts_with("keine_")) {
            steuer.insert(b.feld_id.as_str());
        }
        if let Some(fb) = &b.feld_bedingung {
            for c in fb.kette() {
                steuer.insert(c.feld.as_str());
                abhaengig.entry(c.feld.clone()).or_default().insert(b.feld_id.clone());
            }
        }
        if let Some(a) = &b.ableitung {
            for aus in std::iter::once(&a.aus).chain(&a.und_feld) {
                steuer.insert(aus.as_str());
                abhaengig.entry(aus.clone()).or_default().insert(b.feld_id.clone());
            }
        }
    }
    for (_, d) in &registry.dateien {
        for r in &d.regel_bedingungen {
            steuer.insert(r.feld.as_str());
            for b in alle.iter().filter(|b| b.quelle.regel_id == r.regel_id) {
                abhaengig.entry(r.feld.clone()).or_default().insert(b.feld_id.clone());
            }
        }
        for g in &d.instanz_gruppen {
            for b in alle.iter().filter(|b| b.instanz_gruppe.as_deref() == Some(g.gruppe.as_str())) {
                abhaengig.entry(g.anzahl_feld.clone()).or_default().insert(b.feld_id.clone());
            }
        }
    }
    Welt {
        registry,
        graph: Graph::aus_registry(registry),
        nachschlag: alle.iter().map(|b| (b.feld_id.clone(), *b)).collect(),
        steuer,
        abhaengig,
    }
}

fn welt() -> &'static Welt {
    static W: OnceLock<Welt> = OnceLock::new();
    W.get_or_init(|| baue_welt(Box::leak(Box::new(lade()))))
}

/// Eine Scheibe, verdrahtet wie `api::Zustand::scheibe_bindung`.
struct Lage {
    welt: &'static Welt,
    scheibe: Scheibe,
    cfg: Cfg,
    sicht: Sicht<'static>,
    index: BindungIndex<'static>,
}

fn lage(welt: &'static Welt, scheibe: Scheibe) -> &'static Lage {
    let cfg = Cfg::fuer(scheibe);
    let ids = cfg.felder(|_| Vec::new()).unwrap();
    let sicht = welt.graph.sicht(ids.iter().map(String::as_str)).unwrap();
    let index = ids
        .iter()
        .map(|f| (f.clone(), welt.nachschlag[f.as_str()]))
        .collect();
    Box::leak(Box::new(Lage {
        welt,
        scheibe,
        cfg,
        sicht,
        index,
    }))
}

// ---------------------------------------------------------------- Antworten und Laeufe

/// Die Antwort auf ein Feld: der Abwesenheitswert ("zu": Kreuze "nein", Betraege 0) oder der Beispielwert ("offen").
fn wert(b: &Bindung, offen: bool) -> Value {
    let kein = b.feld_id.starts_with("kein_") || b.feld_id.starts_with("keine_");
    if offen {
        return match b.typ {
            Feldtyp::Bool if kein => json!(false),
            _ => b.beispielwert.clone(),
        };
    }
    if let Some(w) = &b.abwesenheitswert {
        return w.clone();
    }
    match b.typ {
        Feldtyp::Bool => json!(kein),
        Feldtyp::Cent | Feldtyp::Int if b.bereich.as_ref().is_none_or(|r| r.min <= 0) => json!(0),
        _ => b.beispielwert.clone(),
    }
}

fn roh(feld_id: &str, w: Value, ersetzt: Option<String>) -> NeuesEventRoh {
    NeuesEventRoh {
        feld_id: feld_id.to_owned(),
        wert: w.into(),
        zustand: Zustand::Bestaetigt,
        herkunft: HerkunftVektor::Voll(Herkunft {
            herkunft: Achsenwert::new("laie").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        }),
        schreiber: "ui:laie".to_owned(),
        signal: Signal {
            signal_1: Some(None),
            signal_2: Some(format!("ok@{feld_id}")),
            signal_2_fehlt: false,
        },
        signal_2_fremd: None,
        ersetzt,
        ts: None,
    }
}

type Antworten = Vec<(String, Value)>;

#[derive(Clone)]
struct Lauf {
    lage: &'static Lage,
    store: Store,
    /// Jede angenommene Antwort in Reihenfolge (zum Nachspielen).
    antworten: Antworten,
    /// Alles, was schon beantwortet oder vorgegeben wurde.
    versucht: BTreeSet<String>,
    /// Was die Warteschlange gefragt hat (ohne die Vorgaben).
    gefragt: BTreeSet<String>,
    /// Antworten der Warteschlange, die der Store abgewiesen hat.
    abgelehnt: Vec<String>,
}

impl Lauf {
    fn neu(lage: &'static Lage) -> Self {
        Self {
            lage,
            store: Store::leer(2025, Some("sweep".into())),
            antworten: Vec::new(),
            versucht: BTreeSet::new(),
            gefragt: BTreeSet::new(),
            abgelehnt: Vec::new(),
        }
    }

    fn setze(&mut self, feld_id: &str, w: Value) -> Result<(), String> {
        let alt = self
            .store
            .aktive()
            .find(|(f, _)| *f == feld_id)
            .map(|(_, e)| e.event_id.to_string());
        self.store
            .append_roh(
                &roh(feld_id, w.clone(), alt),
                None,
                BindungNachschlag::neu(&self.lage.welt.nachschlag),
            )
            .map_err(|e| format!("{e:?}"))?;
        self.antworten.push((feld_id.to_owned(), w));
        Ok(())
    }

    fn fragen(&self) -> Vec<String> {
        interview::naechste_fragen(
            &self.store,
            &self.lage.sicht,
            &self.lage.welt.graph,
            None::<&HashMap<String, i64>>,
        )
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    /// Setzt die Vorgaben, dann beantwortet es die Warteschlange, bis sie nichts Neues mehr enthaelt. Nach der Antwort auf
    /// ein Steuerfeld wird die Warteschlange neu gelesen, die uebrigen Antworten einer Runde gehen am Stueck.
    /// `Err`: eine Vorgabe wurde abgewiesen oder die Warteschlange leert sich nicht.
    fn fuelle(&mut self, vorgaben: &[(String, Value)], offen: &BTreeSet<String>) -> Result<(), String> {
        for (f, w) in vorgaben {
            self.versucht.insert(f.clone());
            self.setze(f, w.clone()).map_err(|e| format!("{f}: {e}"))?;
        }
        for _ in 0..400 {
            let todo: Vec<String> = self
                .fragen()
                .into_iter()
                .filter(|f| !self.versucht.contains(f))
                .collect();
            if todo.is_empty() {
                return Ok(());
            }
            for f in todo {
                self.versucht.insert(f.clone());
                self.gefragt.insert(f.clone());
                let Some(b) = BindungNachschlag::neu(&self.lage.welt.nachschlag).get(&f) else {
                    self.abgelehnt.push(format!("{f}: keine Bindung"));
                    continue;
                };
                if let Err(e) = self.setze(&f, wert(b, offen.contains(&f))) {
                    self.abgelehnt.push(format!("{f}: {e}"));
                }
                if self.lage.welt.steuer.contains(f.as_str()) {
                    break;
                }
            }
        }
        Err("Warteschlange wird nicht leer".into())
    }

    /// Spielt `praefix` ein (als gefragt, ausser den Vorgaben) und stellt dann `kreuz` auf `v`.
    fn nach_praefix(
        lage: &'static Lage,
        praefix: &[(String, Value)],
        vorgabenfelder: &BTreeSet<String>,
        kreuz: &str,
        v: &Value,
    ) -> Result<Self, String> {
        let mut w = Self::neu(lage);
        for (f, x) in praefix {
            w.versucht.insert(f.clone());
            if !vorgabenfelder.contains(f) {
                w.gefragt.insert(f.clone());
            }
            w.setze(f, x.clone())?;
        }
        w.versucht.insert(kreuz.to_owned());
        w.setze(kreuz, v.clone())?;
        Ok(w)
    }

    /// Der Sperrgrund des Guards auf dem Endstand (VZ 2025; der Guard sieht die Roh-Felder).
    fn grund(&self) -> String {
        let f = self.store.materialisiere(None).unwrap().0;
        let q = Instanzquelle {
            store: Some(&self.store),
            bindung: Some(&self.lage.index),
            nur_bestaetigt: false,
        };
        match an_gesamt_sperrgrund(&f, Some(&self.lage.cfg), Some(Vz::Vz2025), &q) {
            Ok(None) => "keiner".into(),
            Ok(Some(g)) => g.als_str().into(),
            Err(e) => format!("FEHLER {e:?}"),
        }
    }
}

// ---------------------------------------------------------------- die Werte, die ein Steuerfeld annehmen kann

/// Die Werte, die eine Bedingung, eine Regelbedingung oder die Aufzaehlung fuer `feld` nennt, dazu das Gegenteil eines
/// Wahrheitswerts. Ein Wert, den der Store abweist, faellt beim Setzen heraus.
fn werte_fuer(w: &Welt, feld: &str) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let mut add = |x: Value| {
        if out.contains(&x) {
            return;
        }
        let gegenteil = if let Value::Bool(b) = x { Some(json!(!b)) } else { None };
        out.push(x);
        if let Some(g) = gegenteil.filter(|g| !out.contains(g)) {
            out.push(g);
        }
    };
    for (_, d) in &w.registry.dateien {
        for b in &d.bindungen {
            if let Some(fb) = &b.feld_bedingung {
                for c in fb.kette().filter(|c| c.feld == feld) {
                    c.wert.iter().chain(&c.wert_nicht).for_each(|x| add(x.clone()));
                    if let Some(a) = c.alter_im_vz {
                        add(json!(2025 - a));
                        add(json!(1900));
                    }
                    if let Some(g) = c.groesser_als {
                        add(json!(g + 1));
                        add(json!(0));
                    }
                }
            }
        }
        for r in d.regel_bedingungen.iter().filter(|r| r.feld == feld) {
            add(r.wert.clone());
        }
    }
    if let Some(b) = w.nachschlag.get(feld) {
        for e in b.enum_werte.iter().flatten() {
            add(json!(e));
        }
        if b.typ == Feldtyp::Bool {
            add(json!(true));
        }
    }
    out
}

// ---------------------------------------------------------------- Kontexte

struct Kontext {
    name: String,
    vorgaben: Antworten,
    offen: BTreeSet<String>,
    lauf: Lauf,
    grund: String,
}

fn kontext(lage: &'static Lage, name: String, vorgaben: Antworten, offen: BTreeSet<String>) -> Result<Kontext, String> {
    let mut lauf = Lauf::neu(lage);
    lauf.fuelle(&vorgaben, &offen)?;
    let grund = lauf.grund();
    Ok(Kontext {
        name,
        vorgaben,
        offen,
        lauf,
        grund,
    })
}

/// Der Grundzustand ("zu") und je Steuerfeld, das die Warteschlange gefragt hat, und je Wert, der es oeffnet: ein Kontext, in
/// dem die dabei neu gefragten Felder mit dem Beispielwert beantwortet sind. `Err`: ein Kontext liess sich nicht bauen.
fn kontexte(lage: &'static Lage, ver: &str) -> Result<Vec<Kontext>, String> {
    let basis_vorgaben: Antworten = vec![("veranlagung".into(), json!(ver))];
    let basis = kontext(lage, "zu".into(), basis_vorgaben.clone(), BTreeSet::new())?;
    let gefragt0 = basis.lauf.gefragt.clone();
    let mut out = vec![basis];
    for s in &gefragt0 {
        if !lage.welt.steuer.contains(s.as_str()) || s == "veranlagung" {
            continue;
        }
        let zu = wert(lage.welt.nachschlag[s.as_str()], false);
        for v in werte_fuer(lage.welt, s).into_iter().filter(|v| *v != zu) {
            let mut vorgaben = basis_vorgaben.clone();
            vorgaben.push((s.clone(), v.clone()));
            let mut offen = BTreeSet::new();
            // Die neu gefragten Felder oeffnen weitere; bis zum Fixpunkt, hoechstens viermal.
            let mut letzter = None;
            for _ in 0..4 {
                let c = kontext(lage, format!("{s}={v}"), vorgaben.clone(), offen.clone())?;
                let neu: BTreeSet<String> = c.lauf.gefragt.difference(&gefragt0).cloned().collect();
                let fertig = neu.is_subset(&offen);
                offen.extend(neu);
                letzter = Some(c);
                if fertig {
                    break;
                }
            }
            out.extend(letzter);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- Sackgassen

/// Eine Sackgasse: der Guard sperrt, die Warteschlange ist leer, und Antworten auf Felder, die die Scheibe fragen
/// koennte, es aber nicht (mehr) tut, loesen die Sperre auf.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Treffer {
    scheibe: String,
    /// Ein Kreuz stand nur ueber einen Seitenkanal im Fall (es steht nicht im Fragebogen der Scheibe).
    seitenkanal: bool,
    grund: String,
    /// Die Felder, deren Antwort die Sperre aufloest (eine kleinste Menge, nicht die einzige), und die Felder, an
    /// denen ihr Ausschluss haengt.
    felder: Vec<String>,
    ausgeschlossen_durch: BTreeSet<String>,
}

#[derive(Default)]
struct Zaehler {
    kontexte: AtomicUsize,
    kontexte_mit_sperre: AtomicUsize,
    szenarien: AtomicUsize,
    abgewiesen: AtomicUsize,
    abgelehnt: AtomicUsize,
    mit_ausschluss: AtomicUsize,
    mit_sperre: AtomicUsize,
    guard_fehler: AtomicUsize,
}

/// Die Felder, die der Kontext gefragt hat und der Lauf `w` nicht mehr fragt: sie sind durch das Kreuz entfallen.
fn entfallen(c: &Kontext, w: &Lauf, kreuz: &str) -> BTreeSet<String> {
    c.lauf
        .gefragt
        .difference(&w.gefragt)
        .filter(|f| f.as_str() != kreuz && !c.vorgaben.iter().any(|(v, _)| v == *f))
        .cloned()
        .collect()
}

/// Die Felder, an denen der Ausschluss von `b` haengt: seine `feld_bedingung` samt Kette und die Bedingungen seiner Regel.
fn bedingungsfelder(w: &Welt, b: &Bindung) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Some(fb) = &b.feld_bedingung {
        out.extend(fb.kette().map(|c| c.feld.clone()));
    }
    out.extend(
        w.graph
            .regel_bedingungen(&b.quelle.regel_id)
            .iter()
            .map(|r| r.feld.clone()),
    );
    out
}

/// Ist der Endstand `w` eine Sackgasse? `erlaubt` ist die Sperre des Kontexts, die bleiben darf (eine harte Sperre der
/// geoeffneten Gruppe), wenn die Sackgasse verschwindet.
fn sackgasse(w: &Lauf, erlaubt: &str, seitenkanal: bool, z: &Zaehler) -> Option<Treffer> {
    let lage = w.lage;
    let g = w.grund();
    if g.starts_with("FEHLER") {
        z.guard_fehler.fetch_add(1, Ordering::Relaxed);
    }
    // Dieselbe Sperre wie im Kontext: die harte Sperre der geoeffneten Gruppe, das Kreuz aendert daran nichts.
    if g == "keiner" || (g == erlaubt && erlaubt != "keiner") {
        return None;
    }
    z.mit_sperre.fetch_add(1, Ordering::Relaxed);
    let beantwortet: BTreeSet<&str> = w.antworten.iter().map(|(f, _)| f.as_str()).collect();
    let u: Vec<&Bindung> = lage
        .sicht
        .iter()
        .filter(|b| b.askable && b.ableitung.is_none() && !beantwortet.contains(b.feld_id.as_str()))
        .collect();
    let ohne_sperre = |menge: &[&Bindung]| -> bool {
        let mut l = w.clone();
        for b in menge {
            // Eine Antwort, die der Store abweist, fehlt in der Menge.
            let _ = l.setze(&b.feld_id, wert(b, false));
        }
        let g2 = l.grund();
        g2 == "keiner" || (g2 == erlaubt && g2 != g)
    };
    if !ohne_sperre(&u) {
        return None;
    }
    // Kleinste Menge, die die Sperre aufloest: erst grosse Stuecke weglassen, dann einzelne Felder.
    let mut m = u;
    let mut stueck = m.len().div_ceil(2);
    while stueck >= 1 {
        let mut i = 0;
        while i < m.len() {
            let ende = (i + stueck).min(m.len());
            let rest: Vec<&Bindung> = m[..i].iter().chain(&m[ende..]).copied().collect();
            if rest.len() < m.len() && ohne_sperre(&rest) {
                m = rest;
            } else {
                i = ende;
            }
        }
        if stueck == 1 {
            break;
        }
        stueck = stueck.div_ceil(2);
    }
    Some(Treffer {
        scheibe: format!("{:?}", lage.scheibe),
        seitenkanal,
        grund: g,
        felder: m.iter().map(|b| b.feld_id.clone()).collect(),
        ausgeschlossen_durch: m.iter().flat_map(|b| bedingungsfelder(lage.welt, b)).collect(),
    })
}

/// Szenarien im Kontext `c`: `kreuz` wird auf `v` gestellt (a) bevor irgendetwas gefragt wird, (b) nachdem das erste der
/// entfallenden Felder beantwortet ist, (c) nachdem alle bis auf eines beantwortet sind.
fn szenarien(c: &Kontext, kreuz: &str, v: &Value, z: &Zaehler) -> Vec<(Treffer, String)> {
    if c.vorgaben.iter().any(|(f, _)| f == kreuz) {
        return Vec::new();
    }
    let lage = c.lauf.lage;
    let seitenkanal = lage.sicht.get(kreuz).is_none();
    z.szenarien.fetch_add(1, Ordering::Relaxed);
    let mut vorgaben = c.vorgaben.clone();
    vorgaben.push((kreuz.to_owned(), v.clone()));
    let mut w = Lauf::neu(lage);
    if w.fuelle(&vorgaben, &c.offen).is_err() {
        z.abgewiesen.fetch_add(1, Ordering::Relaxed);
        return Vec::new();
    }
    z.abgelehnt.fetch_add(w.abgelehnt.len(), Ordering::Relaxed);
    let e = entfallen(c, &w, kreuz);
    if e.is_empty() {
        return Vec::new();
    }
    z.mit_ausschluss.fetch_add(1, Ordering::Relaxed);
    let wo = |wann: &str| format!("{:?}/{} {kreuz}={v} {wann}", lage.scheibe, c.name);
    let mut out: Vec<_> = sackgasse(&w, &c.grund, seitenkanal, z)
        .map(|t| (t, wo("vorab")))
        .into_iter()
        .collect();
    let vorgabenfelder: BTreeSet<String> = c.vorgaben.iter().map(|(f, _)| f.clone()).collect();
    let pos: Vec<usize> = c
        .lauf
        .antworten
        .iter()
        .enumerate()
        .filter(|(_, (f, _))| e.contains(f))
        .map(|(i, _)| i)
        .collect();
    let mut ks = vec![1];
    if pos.len() > 2 {
        ks.push(pos.len() - 1);
    }
    for k in ks {
        let Some(&schnitt) = pos.get(k - 1) else { continue };
        let Ok(mut w2) = Lauf::nach_praefix(lage, &c.lauf.antworten[..=schnitt], &vorgabenfelder, kreuz, v) else {
            continue;
        };
        if w2.fuelle(&[], &c.offen).is_err() || entfallen(c, &w2, kreuz).is_empty() {
            continue;
        }
        z.mit_ausschluss.fetch_add(1, Ordering::Relaxed);
        out.extend(sackgasse(&w2, &c.grund, seitenkanal, z).map(|t| (t, wo(&format!("nach {k} Feldern")))));
    }
    out
}

type Funde = BTreeMap<Treffer, Vec<String>>;

/// Alle Kontexte einer Scheibe und Veranlagung, jeder mit jedem Kreuz. Die Kontexte laufen auf mehreren Faeden.
fn sweep(welt: &'static Welt, scheibe: Scheibe, ver: &str, z: &Zaehler) -> Funde {
    let lage = lage(welt, scheibe);
    let ctxs = kontexte(lage, ver).unwrap_or_else(|e| panic!("{scheibe:?} {ver}: {e}"));
    z.kontexte.fetch_add(ctxs.len(), Ordering::Relaxed);
    z.kontexte_mit_sperre
        .fetch_add(ctxs.iter().filter(|c| c.grund != "keiner").count(), Ordering::Relaxed);
    z.abgelehnt
        .fetch_add(ctxs.iter().map(|c| c.lauf.abgelehnt.len()).sum(), Ordering::Relaxed);
    let kreuze: Vec<(String, Value)> = welt
        .steuer
        .iter()
        .filter(|k| welt.nachschlag.contains_key(**k))
        .flat_map(|k| werte_fuer(welt, k).into_iter().map(move |v| ((*k).to_owned(), v)))
        .collect();
    let naechster = AtomicUsize::new(0);
    let funde = Mutex::new(Funde::new());
    let merke = |t: Treffer, wo: String| {
        let mut f = funde.lock().unwrap();
        let e = f.entry(t).or_default();
        if e.len() < 3 {
            e.push(wo);
        }
    };
    let faeden = std::thread::available_parallelism().map_or(2, usize::from).min(8);
    std::thread::scope(|s| {
        for _ in 0..faeden {
            s.spawn(|| loop {
                let i = naechster.fetch_add(1, Ordering::Relaxed);
                let Some(c) = ctxs.get(i) else { return };
                // Der Kontext selbst: sein Endstand kann schon eine Sackgasse sein.
                if let Some(t) = sackgasse(&c.lauf, "keiner", false, z) {
                    merke(t, format!("{scheibe:?}/{} (Kontext)", c.name));
                }
                for (k, v) in &kreuze {
                    // Ein Kreuz, das der Kontext nie gefragt hat, setzt nur der Grundzustand (Seitenkanal).
                    if c.name != "zu" && !c.lauf.gefragt.contains(k) {
                        continue;
                    }
                    // Haengt keines der gefragten Felder am Kreuz, aendert es hier nichts.
                    if !welt.abhaengig.get(k).is_some_and(|d| d.iter().any(|f| c.lauf.gefragt.contains(f))) {
                        continue;
                    }
                    for (t, wo) in szenarien(c, k, v, z) {
                        merke(t, wo);
                    }
                }
            });
        }
    });
    funde.into_inner().unwrap()
}

/// Das Ergebnis eines Sweeps ueber die angegebenen Scheiben und beide Veranlagungen.
struct Beobachtung {
    funde: Funde,
    kontexte: usize,
    kontexte_mit_sperre: usize,
    szenarien: usize,
    abgewiesen: usize,
    abgelehnt: usize,
    mit_ausschluss: usize,
    mit_sperre: usize,
    guard_fehler: usize,
}

fn beobachte(welt: &'static Welt, scheiben: &[Scheibe], veranlagungen: &[&str]) -> Beobachtung {
    let z = Zaehler::default();
    let mut funde = Funde::new();
    for &scheibe in scheiben {
        for &ver in veranlagungen {
            for (t, wo) in sweep(welt, scheibe, ver, &z) {
                funde.entry(t).or_default().extend(wo);
            }
        }
    }
    let n = |a: &AtomicUsize| a.load(Ordering::Relaxed);
    Beobachtung {
        funde,
        kontexte: n(&z.kontexte),
        kontexte_mit_sperre: n(&z.kontexte_mit_sperre),
        szenarien: n(&z.szenarien),
        abgewiesen: n(&z.abgewiesen),
        abgelehnt: n(&z.abgelehnt),
        mit_ausschluss: n(&z.mit_ausschluss),
        mit_sperre: n(&z.mit_sperre),
        guard_fehler: n(&z.guard_fehler),
    }
}

/// Der Sweep ueber die echte Bindung, einmal je Testprozess.
fn beobachtet() -> &'static Beobachtung {
    static B: OnceLock<Beobachtung> = OnceLock::new();
    B.get_or_init(|| beobachte(welt(), &SCHEIBEN, &VERANLAGUNGEN))
}

/// Die Klasse einer Sackgasse: Scheibe, Seitenkanal, Sperrgrund.
type Klasse = (String, bool, String);

/// Je Klasse die Vereinigung der genannten Felder; die Felder, an denen ihr Ausschluss haengt, stehen mit "<- " davor.
fn klassen(funde: &Funde) -> BTreeMap<Klasse, BTreeSet<String>> {
    let mut out: BTreeMap<Klasse, BTreeSet<String>> = BTreeMap::new();
    for t in funde.keys() {
        let e = out.entry((t.scheibe.clone(), t.seitenkanal, t.grund.clone())).or_default();
        e.extend(t.felder.iter().cloned());
        e.extend(t.ausgeschlossen_durch.iter().map(|d| format!("<- {d}")));
    }
    out
}

fn beschreibe(klassen: &BTreeMap<Klasse, BTreeSet<String>>) -> String {
    klassen
        .iter()
        .map(|((s, k, g), f)| format!("  {s} seitenkanal={k} {g}: {f:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------- Tests

/// Das eigentliche Tor: der Sweep ueber die echte Bindung findet nur die bekannten Klassen. Eine neue Klasse heisst: ein Kreuz
/// (oder eine Regelbedingung) nimmt ein Feld aus der Warteschlange, das ein Sperrgrund weiter verlangt.
#[test]
fn kein_kreuz_macht_ein_vom_guard_verlangtes_feld_unbeantwortbar() {
    let b = beobachtet();
    let bekannt: BTreeSet<Klasse> = BEKANNT
        .iter()
        .map(|(s, k, g, _)| ((*s).to_owned(), *k, (*g).to_owned()))
        .collect();
    let gesehen = klassen(&b.funde);
    let neu: BTreeMap<_, _> = gesehen
        .iter()
        .filter(|(k, _)| !bekannt.contains(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    assert!(
        neu.is_empty(),
        "NEUE Sackgasse: ein Kreuz nimmt ein Feld aus der Warteschlange, das der Guard weiter verlangt.\n{}\n\
         Beispiele: {:?}",
        beschreibe(&neu),
        b.funde
            .iter()
            .filter(|(t, _)| neu.contains_key(&(t.scheibe.clone(), t.seitenkanal, t.grund.clone())))
            .map(|(_, wo)| wo)
            .collect::<Vec<_>>()
    );
}

/// Die bekannten Sackgassen sind noch da, mit ihrem Kern an Feldern. Das belegt an der echten Bindung, dass der Sweep etwas
/// sieht. Behebt jemand einen Defekt, wird dieser Test rot: der Eintrag in `BEKANNT` faellt dann weg.
#[test]
fn die_bekannten_sackgassen_werden_noch_gesehen() {
    let gesehen = klassen(&beobachtet().funde);
    for (scheibe, seitenkanal, grund, kern) in BEKANNT {
        let schluessel = ((*scheibe).to_owned(), *seitenkanal, (*grund).to_owned());
        let Some(felder) = gesehen.get(&schluessel) else {
            panic!(
                "{scheibe} seitenkanal={seitenkanal} {grund}: nicht mehr gesehen. Ist der Defekt behoben, den Eintrag in \
                 BEKANNT streichen. Gesehen:\n{}",
                beschreibe(&gesehen)
            );
        };
        let fehlt: Vec<&&str> = kern.iter().filter(|f| !felder.contains(**f)).collect();
        assert!(
            fehlt.is_empty(),
            "{scheibe} {grund}: der Sweep nennt {fehlt:?} nicht mehr (gesehen: {felder:?})"
        );
    }
}

/// Die offenen Defekte als Aussage: es gibt keine Sackgasse.
#[test]
fn es_gibt_keine_sackgasse() {
    let gesehen = klassen(&beobachtet().funde);
    assert!(gesehen.is_empty(), "DEFEKT: Sackgassen\n{}", beschreibe(&gesehen));
}

/// Der Sweep liest genug: eine Extraktion, die mangels Kontexten oder Kreuzen still leer wird, machte das Tor oben sinnlos
/// gruen. Die Schwellen liegen unter dem Messwert (54 Steuerfelder, 167 Kontexte, davon 24 mit harter Sperre, 2.214 Szenarien,
/// 1.628 mit Ausschluss, 24 Endstaende mit Sperre; `--nocapture` zeigt sie). Seit Abweichung Nr. 96 (Julius d-47) hat der echte
/// Bestand keine Sackgasse mehr: die Endstaende mit Sperre sind nur noch die der harten Kontexte, und ein Treffer ist keine
/// Voraussetzung mehr. Dass der Sweep nicht blind ist, belegt `ein_eingeschleustes_kreuz_wird_als_sackgasse_gesehen`.
#[test]
fn der_sweep_liest_genug() {
    let b = beobachtet();
    let w = welt();
    eprintln!(
        "Messwerte: {} Steuerfelder, {} Kontexte ({} mit harter Sperre), {} Szenarien, {} mit Ausschluss, {} Endstaende mit Sperre",
        w.steuer.len(),
        b.kontexte,
        b.kontexte_mit_sperre,
        b.szenarien,
        b.mit_ausschluss,
        b.mit_sperre
    );
    assert!(w.steuer.len() >= 40, "Steuerfelder: {}", w.steuer.len());
    assert!(
        w.nachschlag.values().filter(|x| x.feld_bedingung.is_some()).count() >= 60,
        "Felder mit feld_bedingung"
    );
    assert!(b.kontexte >= 100, "Kontexte: {}", b.kontexte);
    assert!(b.szenarien >= 1_000, "Szenarien: {}", b.szenarien);
    assert!(b.mit_ausschluss >= 1_000, "Szenarien mit entfallenden Feldern: {}", b.mit_ausschluss);
    assert!(b.mit_sperre >= 10, "Endstaende mit Sperre: {}", b.mit_sperre);
    assert!(b.kontexte_mit_sperre >= 5, "Kontexte mit harter Sperre: {}", b.kontexte_mit_sperre);
    assert_eq!(b.abgewiesen, 0, "Szenarien, deren Vorgabe der Store abwies");
    assert_eq!(b.abgelehnt, 0, "Antworten der Warteschlange, die der Store abwies");
    assert_eq!(b.guard_fehler, 0, "Endstaende, in denen der Guard einen Fehler lieferte");
}

/// Der Grundzustand ist in jeder Scheibe und Veranlagung frei von Sperren: sonst verdeckte eine Sperre alles, was der Guard
/// dahinter prueft, und jedes Szenario waere blind. Dazu fragt jeder Grundzustand etwas (kein leerer Fragebogen).
#[test]
fn der_grundzustand_sperrt_in_keiner_scheibe() {
    let w = welt();
    for scheibe in SCHEIBEN {
        for ver in VERANLAGUNGEN {
            let l = lage(w, scheibe);
            let c = kontext(
                l,
                "zu".into(),
                vec![("veranlagung".into(), json!(ver))],
                BTreeSet::new(),
            )
            .unwrap();
            assert_eq!(c.grund, "keiner", "{scheibe:?} {ver}: der Grundzustand sperrt");
            assert!(c.lauf.gefragt.len() >= 60, "{scheibe:?} {ver}: nur {} Fragen", c.lauf.gefragt.len());
            assert!(c.lauf.fragen().is_empty(), "{scheibe:?} {ver}: die Warteschlange ist nicht leer");
        }
    }
}

/// Die Welt mit einem eingeschleusten Kreuz: `feld` entfaellt, wenn `kein_gewinn` bestaetigt `true` ist. Das ist der Fehler
/// der Mutanten S01 bis S03 der Nachmessung Nr. 68.
fn welt_mit_kreuz(feld: &str) -> &'static Welt {
    let mut reg = lade();
    let mut gefunden = false;
    for (_, d) in &mut reg.dateien {
        for b in d.bindungen.iter_mut().filter(|b| b.feld_id == feld) {
            b.feld_bedingung = Some(FeldBedingung {
                feld: "kein_gewinn".into(),
                wert: Some(json!(false)),
                grund: "Kontrolle des Sweeps".into(),
                wert_nicht: None,
                alter_im_vz: None,
                praefix_nicht: None,
                groesser_als: None,
                beleg: false,
                und: None,
                oder: None,
            });
            gefunden = true;
        }
    }
    assert!(gefunden, "{feld} steht nicht in der Bindung");
    Box::leak(Box::new(baue_welt(Box::leak(Box::new(reg)))))
}

/// Kontrolle des Sweeps: schleust man das Kreuz an einem Feld ein, das der Guard verlangt, meldet er die Sackgasse. Ohne
/// diese Probe koennte das Tor oben gruen bleiben, weil der Sweep nichts sieht.
#[test]
fn ein_eingeschleustes_kreuz_wird_als_sackgasse_gesehen() {
    // (Feld, Scheibe, Sperrgrund): `hh_rechnung_unbar` verlangt `gesamt.rs` (haushaltsnahe Leistung ohne Ueberweisung),
    // `dhf_eigener_hausstand` verlangt `werbungskosten.rs` (doppelte Haushaltsfuehrung).
    for (feld, scheibe, grund) in [
        ("hh_rechnung_unbar", Scheibe::Gesamt, "rechnung_unbar_offen"),
        ("dhf_eigener_hausstand", Scheibe::Gesamt, "dhf_tatbestand_offen"),
    ] {
        assert!(
            !beobachtet().funde.keys().any(|t| t.felder.iter().any(|f| f == feld)),
            "{feld}: schon in der echten Bindung eine Sackgasse"
        );
        let b = beobachte(welt_mit_kreuz(feld), &[scheibe], &VERANLAGUNGEN);
        let t = b
            .funde
            .keys()
            .find(|t| t.felder.iter().any(|f| f == feld))
            .unwrap_or_else(|| panic!("{feld}: der eingeschleuste Fehler bleibt unbemerkt"));
        assert_eq!(t.grund, grund, "{feld}");
        assert!(!t.seitenkanal, "{feld}: das Kreuz steht im Fragebogen");
        assert!(t.ausgeschlossen_durch.contains("kein_gewinn"), "{feld}: {:?}", t.ausgeschlossen_durch);
    }
}

// ---------------------------------------------------------------- Ticket kein_lohn_pension_partner

/// `api::ergebnis::ergebnis` ohne HTTP: erst der Guard, dann `feste_zahl` mit Graph. `Ok(Cent)` oder der Grund als Text.
fn zahl_oder_grund(w: &Lauf) -> Result<i64, String> {
    let g = w.grund();
    if g != "keiner" {
        return Err(g);
    }
    let lage = w.lage;
    let felder = w.store.materialisiere(None).unwrap().0;
    let kegel: Vec<&str> = lage.cfg.kegel_roh().unwrap().to_vec();
    let achsen: Vec<AchsenBindung> = lage.sicht.iter().map(AchsenBindung::from).collect();
    let umg = Umgebung {
        achsen: &achsen,
        index: &lage.index,
        params: bescheid::testhilfe::params(),
    };
    match feste_zahl(&felder, &lage.cfg, Vz::Vz2025, &kegel, &umg, Some(&w.store), Some(&lage.welt.graph)).unwrap() {
        Ok(z) => Ok(z.zahl.get()),
        Err(k) => Err(format!("{:?}", k.grund)),
    }
}

/// Ein Lauf der Scheibe mit den Vorgaben `vorgaben`; die Warteschlange beantwortet `fuelle` bis zum Ende.
fn lauf_mit(scheibe: Scheibe, vorgaben: &[(&str, Value)]) -> Lauf {
    let vorgaben: Antworten = vorgaben.iter().map(|(f, w)| ((*f).to_owned(), w.clone())).collect();
    let mut w = Lauf::neu(lage(welt(), scheibe));
    w.fuelle(&vorgaben, &BTreeSet::new()).unwrap();
    w
}

/// Das zusammenveranlagte Ehepaar mit den Vorgaben `mehr`.
fn ehepaar(scheibe: Scheibe, mehr: &[(&str, Value)]) -> Lauf {
    let mut v = vec![("veranlagung", json!("zusammen"))];
    v.extend(mehr.iter().cloned());
    lauf_mit(scheibe, &v)
}

/// Das Kreuz "der Ehegatte hat weder Lohn noch Pension" (Abweichung Nr. 46) baut in keiner Scheibe eine Sackgasse: wer es setzt
/// und jede gestellte Frage beantwortet, bekommt eine Zahl. Nur `rentner_gesamt` fragt das Kreuz; in `gesamt` und `an_gesamt`
/// steht es nicht im Fragebogen (der Dienst weist es mit 400 ab), und eine Akte, die es trotzdem traegt, darf dort nicht haengen.
/// Kontrolle: in `rentner_gesamt` verbirgt das Kreuz die Lohnfrage des Ehegatten.
#[test]
fn das_kreuz_kein_lohn_pension_partner_baut_in_keiner_scheibe_eine_sackgasse() {
    for scheibe in SCHEIBEN {
        let w = ehepaar(scheibe, &[("kein_lohn_pension_partner", json!(true))]);
        if scheibe == Scheibe::RentnerGesamt {
            assert!(
                !w.gefragt.contains("bruttoarbeitslohn_partner"),
                "KONTROLLE: in rentner_gesamt verbirgt das Kreuz die Lohnfrage des Ehegatten"
            );
        }
        let erg = zahl_oder_grund(&w);
        assert!(erg.is_ok(), "DEFEKT {scheibe:?}: Kreuz gesetzt, alle Fragen beantwortet, keine Zahl: {erg:?}");
    }
}

/// Dasselbe Kreuz fuer die Hauptperson (`kein_lohn_pension`) verbirgt `bruttoarbeitslohn`. Das Feld steht im Kegel von `gesamt`
/// und `an_gesamt` (nicht in dem von `rentner_gesamt`). Der Guard verlangt es nicht, `feste_zahl` schon
/// (`InputKegelNichtBestaetigt`): der Sweep, der nur den Guard fragt, sieht diese Sackgasse nicht.
#[test]
fn das_kreuz_kein_lohn_pension_baut_in_keiner_scheibe_eine_sackgasse() {
    for scheibe in SCHEIBEN {
        let w = lauf_mit(scheibe, &[("veranlagung", json!("einzel")), ("kein_lohn_pension", json!(true))]);
        let erg = zahl_oder_grund(&w);
        assert!(erg.is_ok(), "DEFEKT {scheibe:?}: Kreuz der Hauptperson gesetzt, alle Fragen beantwortet, keine Zahl: {erg:?}");
    }
}

/// Kontrolle: ein Ehepaar MIT Lohnangabe des Ehegatten bekommt in jeder Scheibe eine Zahl ueber 0, und ohne das Kreuz fragt jede
/// Scheibe die Lohnfrage (in `rentner_gesamt` mit ausdruecklich verneintem Kreuz). Das Fehlen der Frage im Kreuz-Fall ist also
/// die Wirkung des Kreuzes, nicht des Messaufbaus.
#[test]
fn ein_ehepaar_mit_lohn_des_ehegatten_bekommt_in_jeder_scheibe_eine_zahl() {
    for scheibe in SCHEIBEN {
        let mit_lohn = ehepaar(scheibe, &[("bruttoarbeitslohn_partner", json!(3_800_000))]);
        assert!(zahl_oder_grund(&mit_lohn).is_ok_and(|z| z > 0), "{scheibe:?}: {:?}", zahl_oder_grund(&mit_lohn));
        // `fuelle` beantwortet ein gefragtes Kreuz mit dem Abwesenheitswert `true`; darum hier ausdruecklich `false`.
        let ohne_kreuz = ehepaar(scheibe, &[("kein_lohn_pension_partner", json!(false))]);
        assert!(
            ohne_kreuz.gefragt.contains("bruttoarbeitslohn_partner"),
            "KONTROLLE: {scheibe:?} fragt die Lohnfrage des Ehegatten nicht"
        );
        assert!(zahl_oder_grund(&ohne_kreuz).is_ok(), "{scheibe:?}: {:?}", zahl_oder_grund(&ohne_kreuz));
    }
}

/// Widerspruch: das Kreuz sagt "kein Lohn", im Fall steht aber ein BESTAETIGTER Lohn des Ehegatten. Der Lohn zaehlt mit
/// (Abweichung Nr. 46, Entscheidung 3): die Zahl ist die ohne Kreuz. Gilt in jeder Scheibe; kein Weg darf daran etwas aendern.
#[test]
fn bei_kreuz_und_bestaetigtem_lohn_zaehlt_der_lohn_mit() {
    for scheibe in SCHEIBEN {
        let lohn = ("bruttoarbeitslohn_partner", json!(3_800_000));
        let ohne = zahl_oder_grund(&ehepaar(scheibe, std::slice::from_ref(&lohn)));
        let mit = zahl_oder_grund(&ehepaar(scheibe, &[lohn, ("kein_lohn_pension_partner", json!(true))]));
        assert!(ohne.as_ref().is_ok_and(|z| *z > 0), "{scheibe:?}: {ohne:?}");
        assert_eq!(ohne, mit, "{scheibe:?}: das Kreuz aendert die Zahl");
    }
}

/// Widerspruch, zweite Form: das Kreuz sagt "kein Lohn", im Fall steht aber ein nur VORLAEUFIGER Lohn des Ehegatten (Vorschlag der
/// KI oder des Vorjahrs). In `gesamt` und `an_gesamt` bleibt die Lohnfrage in der Warteschlange; die Bestaetigung gibt die Zahl frei.
/// Grenze: in `rentner_gesamt` (Kreuz in der Scheibe) bleibt die Frage verborgen und die Zahl gesperrt (`RingBetragVorlaeufig`),
/// bis der Nutzer das Kreuz zuruecknimmt. Das ist ein eigener Befund und hier nicht gebaut.
#[test]
fn bei_kreuz_und_nur_vorlaeufigem_lohn_bleibt_die_lohnfrage_in_gesamt_und_an_gesamt_beantwortbar() {
    for scheibe in [Scheibe::Gesamt, Scheibe::AnGesamt] {
        let mut w = Lauf::neu(lage(welt(), scheibe));
        for (f, v) in [("veranlagung", json!("zusammen")), ("kein_lohn_pension_partner", json!(true))] {
            w.versucht.insert(f.to_owned());
            w.setze(f, v).unwrap();
        }
        let mut e = roh("bruttoarbeitslohn_partner", json!(3_800_000), None);
        e.zustand = Zustand::Vorlaeufig;
        e.signal = Signal { signal_1: Some(None), signal_2: None, signal_2_fehlt: false };
        w.store
            .append_roh(&e, None, BindungNachschlag::neu(&w.lage.welt.nachschlag))
            .unwrap();
        // Der Vorschlag bleibt unbeantwortet; die uebrigen Fragen der Scheibe beantwortet `fuelle`.
        w.versucht.insert("bruttoarbeitslohn_partner".to_owned());
        w.fuelle(&[], &BTreeSet::new()).unwrap();
        assert!(
            w.fragen().iter().any(|f| f == "bruttoarbeitslohn_partner"),
            "DEFEKT {scheibe:?}: der vorlaeufige Lohn liegt hinter dem Kreuz, die Lohnfrage fehlt in der Warteschlange"
        );
        w.setze("bruttoarbeitslohn_partner", json!(3_800_000)).unwrap();
        let erg = zahl_oder_grund(&w);
        assert!(erg.is_ok_and(|z| z > 0), "{scheibe:?}: nach der Bestaetigung keine Zahl");
    }
}

// ---------------------------------------------------------------- d-54: "keine Kinder" neben gespeicherten Kind-Angaben

/// d-54 (Julius, 2026-10-22): `kein_kind` = ja neben einem bestaetigten Kind-Betrag sperrt in `gesamt` mit `flag_konsistenz_offen` und in keiner
/// anderen Scheibe (`rentner_gesamt`: Entscheidung 3 vom 2026-10-09, ein gespeicherter Wert zaehlt weiter; `an_gesamt` hat das Kreuz nicht).
/// Der Sweep oben sieht diese Klasse nie: er beantwortet leere Faelle, und die Sperre loest keine Antwort auf ein ungefragtes Feld, sondern
/// das Kreuz selbst. Darum steht sie hier einzeln. Kontrolle: dieselbe Angabe mit `kein_kind` = nein sperrt nicht. Die Gegenprobe zum Weg hinaus
/// (Kreuz erreichbar, Zahl nach der Antwort) steht in `rust/api/tests/kein_kind_sperre.rs` und `fluss_e`.
#[test]
fn kein_kind_neben_gespeicherten_kind_angaben_sperrt_nur_in_gesamt() {
    for (feld, wert) in [("schulgeld", json!(600_000)), ("p33a_ausbildung_anzahl_kinder", json!(1)), ("fam_anzahl_kinder", json!(2))] {
        let veranlagung = ("veranlagung", json!("einzel"));
        let ja = lauf_mit(Scheibe::Gesamt, &[veranlagung.clone(), (feld, wert.clone()), ("kein_kind", json!(true))]);
        assert_eq!(ja.grund(), "flag_konsistenz_offen", "gesamt mit {feld} und kein_kind = ja");
        let nein = lauf_mit(Scheibe::Gesamt, &[veranlagung.clone(), (feld, wert.clone()), ("kein_kind", json!(false))]);
        assert_eq!(nein.grund(), "keiner", "KONTROLLE gesamt mit {feld} und kein_kind = nein");
        let rentner = lauf_mit(Scheibe::RentnerGesamt, &[veranlagung, (feld, wert), ("kein_kind", json!(true))]);
        assert_eq!(rentner.grund(), "keiner", "rentner_gesamt mit {feld} und kein_kind = ja");
    }
}

// ---------------------------------------------------------------- Abweichung Nr. 74 an allen Eintraegen

/// Wie eine Bedingung eines Felds auf ein anderes Feld (`kreuz`) in einer Scheibe wirkt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Art {
    /// `kreuz` steht nicht in der Scheibe: die Regel Nr. 74 laesst die Frage stehen.
    Regel,
    /// Dasselbe, aber die Bedingung ist `groesser_als`: ein Beleg, den ohne `kreuz` in der Scheibe nichts liefert. Die Regel
    /// greift nicht; die Frage steht nicht.
    Beleg,
    /// `kreuz` steht in der Scheibe: die Bedingung wirkt wie immer (Kontrolle, dass der Sweep ein Entfallen sieht).
    InSicht,
}

/// Ein Eintrag: das Feld `feld` der Scheibe traegt eine Bedingung auf `kreuz`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Eintrag {
    scheibe: String,
    kreuz: String,
    feld: String,
    art: Art,
}

/// Alle Eintraege der echten Bindung: je Scheibe, je askable Feld mit `feld_bedingung`, je Glied der Kette. Die Zahl zaehlt der
/// Test hier selbst; kein Handwert.
fn eintraege(welt: &'static Welt) -> BTreeSet<Eintrag> {
    let mut out = BTreeSet::new();
    for scheibe in SCHEIBEN {
        let l = lage(welt, scheibe);
        for b in l.sicht.iter().filter(|b| b.askable) {
            let Some(fb) = &b.feld_bedingung else { continue };
            for g in fb.kette() {
                let art = if l.sicht.get(&g.feld).is_some() {
                    Art::InSicht
                } else if g.groesser_als.is_some() {
                    Art::Beleg
                } else {
                    Art::Regel
                };
                out.insert(Eintrag {
                    scheibe: format!("{scheibe:?}"),
                    kreuz: g.feld.clone(),
                    feld: b.feld_id.clone(),
                    art,
                });
            }
        }
    }
    out
}

/// Schliesst der Wert `v` des Bedingungsfelds das Feld des Eintrags aus, wenn man die Regel Nr. 74 wegdenkt? So rechnet
/// `interview::fragen::bedingung_schliesst_aus` ohne die Regel (`alter_im_vz` zaehlt hier immer als schliessend).
fn schliesst_nach_der_bedingung(welt: &Welt, e: &Eintrag, v: &Value) -> bool {
    let Some(fb) = welt.nachschlag.get(&e.feld).and_then(|b| b.feld_bedingung.as_ref()) else {
        return false;
    };
    let x = PyWert::from(v.clone());
    fb.kette().filter(|g| g.feld == e.kreuz && g.groesser_als.is_none()).any(|g| {
        if g.alter_im_vz.is_some() {
            true
        } else if let Some(nicht) = &g.wert_nicht {
            x.py_eq(&PyWert::from(nicht.clone()))
        } else {
            !x.py_eq(&g.wert.clone().map_or(PyWert::Null, PyWert::from))
        }
    })
}

/// Was der Sweep an den Eintraegen sieht. Schluessel: (Eintrag, ob der Wert nach der Bedingung ausschliesst).
#[derive(Default)]
struct Messung {
    /// Laeufe, in denen die Frage des Eintrags stand, obwohl ihr Kontext sie schon gefragt hatte.
    stand: BTreeMap<(Eintrag, bool), usize>,
    /// Laeufe, in denen sie fehlte (je Schluessel hoechstens drei Beispiele).
    fehlte: BTreeMap<(Eintrag, bool), Vec<String>>,
    /// Alle Laeufe, an denen ein Eintrag beteiligt war.
    laeufe: usize,
}

/// Kontexte fuer die Eintraege, deren Frage noch eine zweite Bedingung der Kette verlangt (`und`): `kontexte` bildet nur Werte, die
/// eine Bedingung WOERTLICH nennt, und `wert_nicht: 0` nennt nur die 0, also nie einen Lohn ueber 0. Je zweitem Bedingungsfeld
/// der Scheibe ein Kontext mit dem Beispielwert des Felds.
fn zusatzkontexte(l: &'static Lage, ver: &str, eintraege: &BTreeSet<Eintrag>) -> Vec<Kontext> {
    let name = format!("{:?}", l.scheibe);
    let felder: BTreeSet<&str> = eintraege
        .iter()
        .filter(|e| e.scheibe == name && e.art == Art::Regel)
        .filter_map(|e| l.welt.nachschlag.get(&e.feld).and_then(|b| b.feld_bedingung.as_ref()))
        .flat_map(FeldBedingung::kette)
        .map(|g| g.feld.as_str())
        .filter(|f| l.sicht.get(f).is_some())
        .collect();
    felder
        .into_iter()
        .map(|f| {
            let vorgaben = vec![("veranlagung".to_owned(), json!(ver)), (f.to_owned(), wert(l.welt.nachschlag[f], true))];
            kontext(l, format!("+{f}"), vorgaben, BTreeSet::new()).unwrap_or_else(|e| panic!("{name} {ver} +{f}: {e}"))
        })
        .collect()
}

/// Je Scheibe, Veranlagung und Kontext, der die Frage eines Eintrags schon gestellt hat, und je Wert des Bedingungsfelds: das
/// Bedingungsfeld wird gesetzt, BEVOR irgendetwas gefragt wird (so steht es in einer von Hand geaenderten Akte), die Warteschlange
/// wird beantwortet, und der Lauf sagt, ob die Frage noch gestellt wird. Der Store nimmt das Feld an, auch wenn die Scheibe es
/// nicht fragt (er kennt die ganze Bindung).
fn miss_eintraege(welt: &'static Welt, eintraege: &BTreeSet<Eintrag>, arten: &[Art]) -> Messung {
    let m = Mutex::new(Messung::default());
    for scheibe in SCHEIBEN {
        for ver in VERANLAGUNGEN {
            let l = lage(welt, scheibe);
            let mut ctxs = kontexte(l, ver).unwrap_or_else(|e| panic!("{scheibe:?} {ver}: {e}"));
            ctxs.extend(zusatzkontexte(l, ver, eintraege));
            let name = format!("{scheibe:?}");
            let kreuze: BTreeSet<&str> = eintraege
                .iter()
                .filter(|e| e.scheibe == name && arten.contains(&e.art))
                .map(|e| e.kreuz.as_str())
                .collect();
            let naechster = AtomicUsize::new(0);
            let faeden = std::thread::available_parallelism().map_or(2, usize::from).min(8);
            std::thread::scope(|s| {
                for _ in 0..faeden {
                    s.spawn(|| loop {
                        let i = naechster.fetch_add(1, Ordering::Relaxed);
                        let Some(c) = ctxs.get(i) else { return };
                        for &kreuz in &kreuze {
                            let ziele: Vec<&Eintrag> = eintraege
                                .iter()
                                .filter(|e| e.scheibe == name && e.kreuz == kreuz && arten.contains(&e.art))
                                .filter(|e| c.lauf.gefragt.contains(&e.feld))
                                .collect();
                            if ziele.is_empty() || c.vorgaben.iter().any(|(f, _)| f == kreuz) {
                                continue;
                            }
                            for v in werte_fuer(welt, kreuz) {
                                let mut vorgaben = c.vorgaben.clone();
                                vorgaben.push((kreuz.to_owned(), v.clone()));
                                let mut w = Lauf::neu(l);
                                w.fuelle(&vorgaben, &c.offen)
                                    .unwrap_or_else(|e| panic!("{name}/{} {kreuz}={v}: {e}", c.name));
                                let mut m = m.lock().unwrap();
                                m.laeufe += 1;
                                for e in ziele.iter().copied() {
                                    let k = (e.clone(), schliesst_nach_der_bedingung(welt, e, &v));
                                    if w.gefragt.contains(&e.feld) {
                                        *m.stand.entry(k).or_default() += 1;
                                    } else {
                                        let f = m.fehlte.entry(k).or_default();
                                        if f.len() < 3 {
                                            f.push(format!("{name}/{ver}/{} {kreuz}={v}", c.name));
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            });
        }
    }
    m.into_inner().unwrap()
}

/// Die Eintraege, an denen die Regel Nr. 74 heute greift: (Scheibe, Kreuz, Felder). Die Bindung ist die Quelle; diese Liste
/// haelt fest, welche Eintraege geprueft sind. Waechst die Bindung um einen Eintrag oder faellt einer weg, wird
/// `die_regel_nr_74_erfasst_genau_die_gepruefte_menge_an_eintraegen` rot, bis der Eintrag hier steht (oder in [`AUSNAHMEN`]).
const GEPRUEFT: &[(&str, &str, &[&str])] = &[
    ("AnGesamt", "kein_lohn_pension", &["bruttoarbeitslohn", "p36_lohnsteuer"]),
    ("AnGesamt", "kein_lohn_pension_partner", &["bruttoarbeitslohn_partner", "vor_ag_anteil_rv_partner", "vor_an_anteil_rv_partner"]),
    (
        "Gesamt",
        "kein_lohn_pension",
        &[
            "bruttoarbeitslohn",
            "p36_lohnsteuer",
            "steuerklasse",
            "versorgung_alter_bei_beginn",
            "versorgung_art",
            "versorgung_beginn_jahr",
            "versorgung_bemessungsgrundlage",
            "versorgung_jahresrente",
        ],
    ),
    (
        "Gesamt",
        "kein_lohn_pension_partner",
        &[
            "bruttoarbeitslohn_partner",
            "p36_lohnsteuer_partner",
            "steuerklasse_partner",
            // Abweichung Nr. 87: die fuenf Versorgungsfelder des Ehegatten stehen seit Nr. 87 in `gesamt`.
            "versorgung_alter_bei_beginn_partner",
            "versorgung_art_partner",
            "versorgung_beginn_jahr_partner",
            "versorgung_bemessungsgrundlage_partner",
            "versorgung_jahresrente_partner",
            "vor_ag_anteil_rv_partner",
            "vor_an_anteil_rv_partner",
        ],
    ),
    // Fluss J F2/F3/F8: `kein_gewinn_partner` (11 Felder) steht seit 2026-10-21 in `rentner_gesamt`, sein Eintrag hier ist weggefallen
    // (das Kreuz wird gefragt, die Bedingung schliesst dort aus); er steht in [`KONTROLLE`].
    (
        "RentnerGesamt",
        "kein_kap_partner",
        &["kap_gewinn_aktien_partner", "kap_gewinn_sonstige_partner", "kap_kapitalertraege_partner", "kap_verlust_aktien_partner", "kap_verlust_sonstige_partner"],
    ),
    (
        "RentnerGesamt",
        "kein_sonstige_partner",
        &["rentner_alter_bei_rentenbeginn_partner", "rentner_jahresrente_partner", "rentner_renten_art_partner", "rentner_renten_beginn_jahr_partner", "rentner_rentenfreibetrag_partner"],
    ),
    // Fragen-Reduktion Teil 2, Phase 2: `keine_behinderung_pflege` (zwei Felder) steht seit 2026-10-22 in `rentner_gesamt`, sein Eintrag hier ist
    // weggefallen (das Kreuz wird gefragt, die Bedingung schliesst dort aus). Er steht NICHT in [`KONTROLLE`]: im Grundzustand ("zu") schliesst
    // das Kreuz die Felder aus, und der Kontext, der sie stellt, gibt das Kreuz selbst vor (`= ja`); `miss_eintraege` ueberspringt den Eintrag
    // dann (gemessen: "stand 0, fehlte []"). Die Kontrolle der Messung bleibt bei den vier Eintraegen unten.
    ("RentnerGesamt", "keine_behinderung_pflege_partner", &["rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner"]),
];

/// Eintraege mit `groesser_als` auf ein Feld, das die Scheibe nicht fragt: die Regel greift dort nicht, denn die Betragsbedingung
/// verlangt einen Beleg, und ohne das Feld in der Scheibe gibt es keinen. Die Frage steht nicht.
///
/// Leer seit Abweichung Nr. 87: die vier Rueckfragen des Ehegatten zum Versorgungsbezug (`versorgung_zweiter_bezug_partner`,
/// `versorgung_sterbegeld_partner`, `versorgung_mehrere_jahre_partner`, `versorgung_nicht_ganzjaehrig_partner`) hingen in `gesamt`
/// an `versorgung_jahresrente_partner > 0`, das die Scheibe nicht fragte; mit den fuenf Feldern des Ehegatten in `gesamt` hat die
/// Bedingung ihren Beleg. Ein neuer solcher Eintrag macht `die_regel_nr_74_erfasst_genau_die_gepruefte_menge_an_eintraegen` rot,
/// bis er hier steht; dann misst `die_betragsbedingung_ausserhalb_der_scheibe_bleibt_ein_beleg` ihn wieder.
const BELEG: &[(&str, &str, &[&str])] = &[];

/// Eintraege, an denen die Frage trotz der Regel verschwindet: (Scheibe, Kreuz, Feld, Grund). Leer: der Sweep fand keinen. Ein
/// Eintrag hier ist ein Befund fuer ein eigenes Ticket, kein Freibrief; die Regel selbst aendert dieser Test nicht.
const AUSNAHMEN: &[(&str, &str, &str, &str)] = &[];

/// Kontrolle des Sweeps: Eintraege, bei denen das Kreuz in der Scheibe steht. Dort nimmt ein schliessender Wert die Frage heraus;
/// derselbe Messaufbau muss das sehen, sonst sagt sein "die Frage bleibt" bei den Regel-Eintraegen nichts.
const KONTROLLE: &[(&str, &str, &str)] = &[
    ("Gesamt", "kein_gewinn_partner", "einkuenfte_gewinn_partner"),
    ("Gesamt", "kein_kap_partner", "kap_kapitalertraege_partner"),
    ("RentnerGesamt", "kein_gewinn", "gewinn_betriebsart"),
    // Fluss J F2/F3/F8: das Kreuz des Ehegatten, das `rentner_gesamt` seit 2026-10-21 fragt (vorher ein Eintrag der Regel).
    ("RentnerGesamt", "kein_gewinn_partner", "einkuenfte_gewinn_partner"),
];

fn als_menge(liste: &[(&str, &str, &[&str])], art: Art) -> BTreeSet<Eintrag> {
    liste
        .iter()
        .flat_map(|(s, k, fs)| {
            fs.iter().map(move |f| Eintrag { scheibe: (*s).to_owned(), kreuz: (*k).to_owned(), feld: (*f).to_owned(), art })
        })
        .collect()
}

fn zeige(e: &Eintrag) -> String {
    format!("{} {} -> {}", e.scheibe, e.kreuz, e.feld)
}

/// Die Zahl der Eintraege zaehlt der Test aus der Bindung (je Scheibe, je askable Feld mit Bedingung, je Glied der Kette, dessen
/// Bedingungsfeld die Scheibe nicht fragt) und haelt sie gegen die Liste der geprueften. Ein neuer Eintrag macht ihn rot.
#[test]
fn die_regel_nr_74_erfasst_genau_die_gepruefte_menge_an_eintraegen() {
    let ist = eintraege(welt());
    for (art, soll) in [(Art::Regel, GEPRUEFT), (Art::Beleg, BELEG)] {
        let ist_art: BTreeSet<Eintrag> = ist.iter().filter(|e| e.art == art).cloned().collect();
        let soll_art = als_menge(soll, art);
        let neu: Vec<String> = ist_art.difference(&soll_art).map(zeige).collect();
        let weg: Vec<String> = soll_art.difference(&ist_art).map(zeige).collect();
        assert!(
            neu.is_empty() && weg.is_empty(),
            "{art:?}: die Bindung hat andere Eintraege als die Liste der geprueften.\nNEU in der Bindung (erst pruefen, dann in GEPRUEFT/BELEG \
             aufnehmen oder in AUSNAHMEN benennen): {neu:#?}\nNICHT MEHR in der Bindung (aus der Liste streichen): {weg:#?}"
        );
    }
    for (s, k, f, _) in AUSNAHMEN {
        assert!(
            ist.iter().any(|e| e.art == Art::Regel && (e.scheibe.as_str(), e.kreuz.as_str(), e.feld.as_str()) == (*s, *k, *f)),
            "AUSNAHMEN nennt {s} {k} -> {f}, der kein Regel-Eintrag mehr ist: streichen"
        );
    }
}

/// An jedem Eintrag bleibt die Frage stehen, auch wenn das Bedingungsfeld in der Akte steht: das Kreuz wird gesetzt, BEVOR
/// irgendetwas gefragt wird (so steht es in einer von Hand geaenderten Akte), in jedem Kontext der Scheibe, der die Frage sonst
/// stellt, mit jedem Wert, den eine Bedingung nennt. Ohne die Regel faellt die Frage bei einem schliessenden Wert weg
/// (Sackgasse, wenn der Guard sie verlangt). Zuerst die Kontrolle: derselbe Aufbau sieht das Entfallen, wenn das Kreuz in der
/// Scheibe steht.
#[test]
fn an_jedem_eintrag_der_regel_nr_74_bleibt_die_frage_stehen() {
    let w = welt();
    let alle = eintraege(w);
    let regel: BTreeSet<Eintrag> = alle.iter().filter(|e| e.art == Art::Regel).cloned().collect();
    let kontrolle: BTreeSet<Eintrag> = alle
        .iter()
        .filter(|e| e.art == Art::InSicht && KONTROLLE.contains(&(e.scheibe.as_str(), e.kreuz.as_str(), e.feld.as_str())))
        .cloned()
        .collect();
    assert_eq!(kontrolle.len(), KONTROLLE.len(), "KONTROLLE: ein Eintrag der Kontrolle steht nicht mehr in der Bindung");
    let ziele: BTreeSet<Eintrag> = regel.union(&kontrolle).cloned().collect();
    let m = miss_eintraege(w, &ziele, &[Art::Regel, Art::InSicht]);
    let stand = |e: &Eintrag, schliessend| m.stand.get(&(e.clone(), schliessend)).copied().unwrap_or(0);
    let fehlte = |e: &Eintrag, schliessend| m.fehlte.get(&(e.clone(), schliessend)).cloned().unwrap_or_default();
    eprintln!("Messwerte: {} Eintraege, {} Laeufe", regel.len(), m.laeufe);
    assert!(m.laeufe >= 500, "Laeufe: {} (der Sweep ist blind)", m.laeufe);

    for e in &kontrolle {
        assert!(
            stand(e, true) == 0 && !fehlte(e, true).is_empty(),
            "KONTROLLE {}: ein schliessender Wert nimmt die Frage nicht heraus (stand {}, fehlte {:?})",
            zeige(e),
            stand(e, true),
            fehlte(e, true)
        );
        assert!(
            stand(e, false) >= 1 && fehlte(e, false).is_empty(),
            "KONTROLLE {}: ein nicht schliessender Wert nimmt die Frage heraus",
            zeige(e)
        );
    }

    let mut befunde: Vec<String> = Vec::new();
    for e in &regel {
        if AUSNAHMEN.iter().any(|(s, k, f, _)| (*s, *k, *f) == (e.scheibe.as_str(), e.kreuz.as_str(), e.feld.as_str())) {
            continue;
        }
        if stand(e, true) == 0 {
            befunde.push(format!("{}: nicht belegt, kein Lauf mit schliessendem Wert, in dem die Frage sonst steht", zeige(e)));
        }
        for schliessend in [true, false] {
            let f = fehlte(e, schliessend);
            if !f.is_empty() {
                befunde.push(format!("{}: die Frage verschwindet (schliessender Wert: {schliessend}), z. B. {f:?}", zeige(e)));
            }
        }
    }
    assert!(befunde.is_empty(), "DEFEKT an Eintraegen der Regel Nr. 74:\n{}", befunde.join("\n"));
}

/// Die Betragsbedingung (`groesser_als`) auf ein Feld, das die Scheibe nicht fragt, bleibt ein Beleg, den es nicht gibt: die
/// Regel Nr. 74 greift dort nicht. Die Folgefragen stehen weder im Grundzustand noch mit einem Betrag von 0; ein bestaetigter
/// Betrag ueber der Schwelle in der Akte oeffnet sie.
#[test]
fn die_betragsbedingung_ausserhalb_der_scheibe_bleibt_ein_beleg() {
    let w = welt();
    let belege: Vec<Eintrag> = eintraege(w).into_iter().filter(|e| e.art == Art::Beleg).collect();
    // Leer ist erlaubt (BELEG ist seit Nr. 87 leer), solange die Bindung und die Liste dieselbe Zahl nennen: sonst lief die Schleife blind.
    assert_eq!(belege.len(), BELEG.iter().map(|(_, _, felder)| felder.len()).sum::<usize>(), "KONTROLLE: Beleg-Eintraege der Bindung und BELEG weichen ab");
    for e in &belege {
        let scheibe = *SCHEIBEN.iter().find(|s| format!("{s:?}") == e.scheibe).unwrap();
        let schwelle = w.nachschlag[&e.feld]
            .feld_bedingung
            .as_ref()
            .unwrap()
            .kette()
            .find(|g| g.feld == e.kreuz)
            .and_then(|g| g.groesser_als)
            .unwrap();
        let zusammen = ("veranlagung", json!("zusammen"));
        let ohne = lauf_mit(scheibe, std::slice::from_ref(&zusammen));
        assert!(!ohne.gefragt.contains(&e.feld), "{}: ohne Betrag steht die Frage", zeige(e));
        let null = lauf_mit(scheibe, &[zusammen.clone(), (e.kreuz.as_str(), json!(0))]);
        assert!(!null.gefragt.contains(&e.feld), "{}: mit Betrag 0 steht die Frage", zeige(e));
        let viel = lauf_mit(scheibe, &[zusammen, (e.kreuz.as_str(), json!(schwelle + 1))]);
        assert!(viel.gefragt.contains(&e.feld), "{}: mit Betrag ueber der Schwelle steht die Frage nicht", zeige(e));
    }
}
