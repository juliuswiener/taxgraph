//! Deklaration → ELSTER-Submission-XML (E10) (`produkt/eingang/elster_xml.py`).
//!
//! Die Verschachtelung kommt aus dem amtlichen XSD ([`crate::schema_info`]), nie aus einer
//! Handtabelle. Fail-closed: eine Deklaration mit offenen Eingaben wird nicht serialisiert, und
//! eine Kz ohne Schema-Pfad ist ein harter Fehler statt eines stillen Weglassens.
//!
//! Namespaces: `<Elster>` traegt den Rahmen-Namespace als Default, `<E10>` den E10-Namespace
//! LOKAL als Default. Eine Praefix-Deklaration am Root lehnt ERiC ab (rc=610301200, gemessen
//! 2026-08-09), obwohl das XSD beide Fassungen annimmt. Die Ausgabe ist byte-gleich zu
//! `ET.indent(space="\t")` + `ET.tostring` + der Praefix-Nachbearbeitung des Originals.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use serde_json::Value;

use crate::deklaration::{Deklaration, Felder};
use crate::py;
use crate::xsd::{finde_schema, schema_info, KzMeta, PflichtKinder, SchemaInfo};

/// Namespace des ELSTER-Rahmens.
pub const NS_ELSTER: &str = "http://www.elster.de/elsterxml/schema/v11";
/// ERiC-Testfall-Merker: wird nie ans Finanzamt zugestellt.
pub const TESTMERKER_ERIC: &str = "700000004";

/// Kz aus `anlage_instanzen`/`deklaration`, die in eine andere Datenart gehoeren (E60xx → E77).
const E10_AUSSCHLUSS_DATENART: &[&str] = &["E6002301", "E6004901"];
/// Gruppen, deren Wiederholung tiefer als am E10-Direktkind haengt (`<SO>` maxOccurs=1).
const INSTANZ_CONTAINER_TIEFER: &[(&str, &str)] = &[("p23_veraeusserung", "Einz")];
/// Skalare Pflicht-Diskriminatoren ohne Kz.
const PFLICHT_DEFAULT: &[(&str, &str)] = &[("Person", "PersonA"), ("Laufende_Nummer_V", "1")];
/// Pflicht-Kinder mit XSD-unique-Bedingung: je Instanz 1, 2, 3 …
const INSTANZ_NUMMER_FELDER: &[&str] = &["Laufende_Nummer_V"];
/// Vorsatz-Absender aus Stammdaten-Kz (Person A); dieselben Angaben stehen im Hauptvordruck.
const ABSENDER_HERKUNFT: &[(&str, &[(&str, &str)])] = &[
    (
        "absender_name",
        &[("E0100201", "Nachname"), ("E0100301", "Vorname")],
    ),
    (
        "absender_strasse",
        &[("E0101104", "Straße"), ("E0101206", "Hausnummer")],
    ),
    ("absender_plz", &[("E0100601", "PLZ")]),
    ("absender_ort", &[("E0100602", "Wohnort")]),
];
const ABSENDER_STRASSE_ZUSATZ_KZ: &str = "E0101207";

/// Die Deklaration laesst sich nicht schema-konform serialisieren. Der Text ist
/// Paritaetskriterium: er geht als `detail` einer 422-Antwort an den Nutzer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct XmlFehler(pub String);

/// Parameter von [`erzeuge_xml`]; `Default` entspricht den Python-Vorgaben.
#[derive(Clone)]
pub struct XmlOptionen<'a> {
    pub vz: i64,
    pub empfaenger_land: String,
    pub empfaenger_finanzamt: String,
    /// `None` → `$ELSTER_HERSTELLER_ID`. Nie loggen, nie ins Repo.
    pub hersteller_id: Option<String>,
    pub datenlieferant: String,
    /// `None`/leer → kein `<Testmerker>` (Echtfall-XML; ein Versand ist hier NICHT gebaut).
    pub testmerker: Option<String>,
    pub nutzdaten_ticket: String,
    /// Haengt den `<Vorsatz>`-Block an und erzwingt Absender + Bankverbindungs-Entscheidung.
    pub abgabefaehig: bool,
    pub absender_name: Option<String>,
    pub absender_strasse: Option<String>,
    pub absender_plz: Option<String>,
    pub absender_ort: Option<String>,
    pub absender_steuernummer: Option<String>,
    /// Felder-Ebene, aus der die Steuernummer abgeleitet wird (kein Kz-Spiegel).
    pub snapshot: Option<&'a Felder>,
}

impl Default for XmlOptionen<'_> {
    fn default() -> Self {
        Self {
            vz: 2025,
            empfaenger_land: "BY".to_owned(),
            empfaenger_finanzamt: "9181".to_owned(),
            hersteller_id: None,
            datenlieferant: "TaxGraph".to_owned(),
            testmerker: Some(TESTMERKER_ERIC.to_owned()),
            nutzdaten_ticket: "taxgraph-0001".to_owned(),
            abgabefaehig: false,
            absender_name: None,
            absender_strasse: None,
            absender_plz: None,
            absender_ort: None,
            absender_steuernummer: None,
            snapshot: None,
        }
    }
}

impl fmt::Debug for XmlOptionen<'_> {
    /// Die Hersteller-ID erscheint nie im Debug-Text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XmlOptionen")
            .field("vz", &self.vz)
            .field(
                "hersteller_id",
                &self.hersteller_id.as_ref().map(|_| "<gesetzt>"),
            )
            .field("abgabefaehig", &self.abgabefaehig)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------- Baum

#[derive(Debug, Default)]
struct Knoten {
    name: String,
    attrs: Vec<(&'static str, String)>,
    text: Option<String>,
    kinder: Vec<Knoten>,
}

impl Knoten {
    fn neu(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..Self::default()
        }
    }

    fn blatt(name: &str, text: impl Into<String>) -> Self {
        Self {
            name: name.to_owned(),
            text: Some(text.into()),
            ..Self::default()
        }
    }

    fn mit(mut self, kinder: Vec<Self>) -> Self {
        self.kinder = kinder;
        self
    }

    fn anzahl(&self, name: &str) -> usize {
        self.kinder.iter().filter(|k| k.name == name).count()
    }

    /// Das n-te Kind (0-basiert) mit diesem Namen.
    fn nth_mut(&mut self, name: &str, n: usize) -> Result<&mut Self, XmlFehler> {
        self.kinder
            .iter_mut()
            .filter(|k| k.name == name)
            .nth(n)
            .ok_or_else(|| XmlFehler(format!("interner Fehler: Container {name}[{n}] fehlt")))
    }
}

fn vorgabe(name: &str) -> Option<&'static str> {
    PFLICHT_DEFAULT
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| *v)
}

fn fehlende_vorgabe(container: &[String], name: &str) -> XmlFehler {
    XmlFehler(format!(
        "Container {} verlangt Pflichtfeld '{name}', für das keine Vorgabe hinterlegt ist (PFLICHT_DEFAULT erweitern).",
        container.join("/")
    ))
}

/// `_wert_text`: Bool ist in ELSTER `X`, sonst Python `str(wert)`.
fn wert_text(wert: &Value) -> String {
    if wert.is_boolean() {
        "X".to_owned()
    } else {
        py::str_von(wert)
    }
}

struct Instanzwahl<'p> {
    container: &'p [String],
    index: usize,
    person_b: bool,
}

/// `_einhaengen` (`elster_xml.py:201-306`): Pfad anlegen (idempotent), Blatt setzen.
///
/// Die Entscheidung "Blatt oder nicht" faellt VOR dem Anlegen des Pfades — zweistufig, weil
/// `knoten` die `&mut`-Referenz bis zum Ende haelt und ein nachtraegliches Abraeumen sonst
/// doppelt borrowt (E0382/E0499). Ein Ankreuzfeld (Ja1/JaX) mit "Nein" ergibt kein Element;
/// wuerde der Pfad trotzdem gebaut, bliebe sein Container leer stehen und checkESt weist die
/// ganze Abgabe ab (gemessen 2026-10-01, ERiC 44.2.4.0):
/// „Der Kontext `/AgB[1]/Beh[1]/Geh_Steh_Blind_Hilfl[1]` ist leer."
fn einhaengen(
    e10: &mut Knoten,
    pfad: &[String],
    wert: &Value,
    pflicht: &PflichtKinder,
    kz_meta: &HashMap<String, KzMeta>,
    instanz: Option<&Instanzwahl<'_>>,
) -> Result<(), XmlFehler> {
    // Stufe 1: entscheiden. `Ok(None)` heisst "kein Blatt" — der Pfad wird gar nicht erst gebaut.
    let Some(text) = blatt_text(pfad.last(), wert, kz_meta) else {
        return Ok(());
    };

    // Stufe 2: anlegen und einhaengen.
    let mut knoten = e10;
    let innen = pfad.len().saturating_sub(1);
    for i in 1..innen {
        let (Some(name), Some(container)) = (pfad.get(i), pfad.get(..=i)) else {
            break;
        };
        let leer = Vec::new();
        let pflichtkinder = pflicht.get(container).unwrap_or(&leer);
        let wahl = instanz.filter(|w| w.container == container);
        let n = wahl.map_or(0, |w| w.index);
        let vorhanden = knoten.anzahl(name);
        if vorhanden <= n && (wahl.is_some() || vorhanden == 0) {
            let bis = if wahl.is_some() { n } else { 0 };
            for j in vorhanden..=bis {
                let mut neu = Knoten::neu(name);
                for p in pflichtkinder {
                    let text = match (wahl, p.as_str()) {
                        (Some(w), "Person") => if j == w.index && w.person_b || j >= 1 {
                            "PersonB"
                        } else {
                            "PersonA"
                        }
                        .to_owned(),
                        (Some(_), p) if INSTANZ_NUMMER_FELDER.contains(&p) => (j + 1).to_string(),
                        (_, p) => vorgabe(p)
                            .ok_or_else(|| fehlende_vorgabe(container, p))?
                            .to_owned(),
                    };
                    neu.kinder.push(Knoten::blatt(p, text));
                }
                knoten.kinder.push(neu);
            }
        }
        knoten = knoten.nth_mut(name, n)?;
    }
    let Some(kz_name) = pfad.last() else {
        return Ok(());
    };
    knoten.kinder.push(Knoten::blatt(kz_name, text));
    Ok(())
}

/// Stufe 1 von [`einhaengen`]: den Blatt-Text bestimmen, oder `None`, wenn kein Element
/// entstehen darf.
///
/// Ja-Typ: True → erster enum-Wert; False → bei `JaNein12` der zweite ("2" = Nein, eine echte
/// Antwort), bei Ankreuzfeldern (Ja1/JaX) weglassen (gemessen 2026-08-16). `None` ist damit
/// die einzige Stelle, an der ein Kz bewusst nichts in die Deklaration eintraegt.
fn blatt_text(
    kz_name: Option<&String>,
    wert: &Value,
    kz_meta: &HashMap<String, KzMeta>,
) -> Option<String> {
    let kz_name = kz_name?;
    if let (Some(km), Value::Bool(b)) = (kz_meta.get(kz_name), wert) {
        if km.is_ja {
            return if *b {
                Some(km.enums.first().map_or("X", String::as_str).to_owned())
            } else {
                km.enums.get(1).cloned()
            };
        }
    }
    Some(wert_text(wert))
}

// ---------------------------------------------------------------- Serialisierung

fn schreibe(w: &mut Writer<Vec<u8>>, k: &Knoten, tiefe: usize) -> std::io::Result<()> {
    let mut start = BytesStart::new(k.name.as_str());
    for (a, v) in &k.attrs {
        start.push_attribute((*a, v.as_str()));
    }
    if !k.kinder.is_empty() {
        w.write_event(Event::Start(start))?;
        let einzug = format!("\n{}", "\t".repeat(tiefe + 1));
        for kind in &k.kinder {
            w.write_event(Event::Text(BytesText::from_escaped(einzug.as_str())))?;
            schreibe(w, kind, tiefe + 1)?;
        }
        let zurueck = format!("\n{}", "\t".repeat(tiefe));
        w.write_event(Event::Text(BytesText::from_escaped(zurueck)))?;
        w.write_event(Event::End(BytesEnd::new(k.name.as_str())))
    } else if let Some(t) = k.text.as_deref().filter(|t| !t.is_empty()) {
        w.write_event(Event::Start(start))?;
        // ElementTree escaped im Text nur &, <, > — genau `partial_escape`.
        w.write_event(Event::Text(BytesText::from_escaped(
            quick_xml::escape::partial_escape(t),
        )))?;
        w.write_event(Event::End(BytesEnd::new(k.name.as_str())))
    } else {
        w.write_event(Event::Empty(start))
    }
}

/// Zweite Linie hinter Auflage T (Alt-Stores, Importe, Kz JEDEN Typs): `partial_escape` maskiert
/// nur &, < und > — ein Zeichen ausserhalb der XML-1.0-Char-Produktion landete roh im XML, und
/// ELSTER wiese die ganze Abgabe ab. Dokumentreihenfolge wie `ElementTree.iter()`; die Meldung
/// nennt das Element, nie den Wert (PII), wortgleich mit `elster_xml.py::erzeuge_xml`.
fn pruefe_zeichen(k: &Knoten) -> Result<(), XmlFehler> {
    if k.text
        .as_deref()
        .is_some_and(|t| !domain::nur_xml_zeichen(t))
    {
        return Err(XmlFehler(format!(
            "Element {} enthält ein Steuerzeichen, das im XML nicht zulässig ist — ELSTER wiese die ganze Abgabe ab. Wert nicht geloggt.",
            k.name
        )));
    }
    k.kinder.iter().try_for_each(pruefe_zeichen)
}

fn serialisiere(wurzel: &Knoten) -> Result<String, XmlFehler> {
    pruefe_zeichen(wurzel)?;
    let mut w = Writer::new(Vec::new());
    w.config_mut().add_space_before_slash_in_empty_elements = true;
    schreibe(&mut w, wurzel, 0).map_err(|e| XmlFehler(format!("Serialisierung: {e}")))?;
    let body =
        String::from_utf8(w.into_inner()).map_err(|e| XmlFehler(format!("Serialisierung: {e}")))?;
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{body}"
    ))
}

// ---------------------------------------------------------------- Kopf, Vorsatz

fn transfer_header(opt: &XmlOptionen<'_>, hid: &str) -> Knoten {
    let mut kinder = vec![
        Knoten::blatt("Verfahren", "ElsterErklaerung"),
        Knoten::blatt("DatenArt", "ESt"),
        Knoten::blatt("Vorgang", "send-Auth"),
    ];
    if let Some(tm) = opt.testmerker.as_deref().filter(|t| !t.is_empty()) {
        kinder.push(Knoten::blatt("Testmerker", tm));
    }
    let mut empf =
        Knoten::neu("Empfaenger").mit(vec![Knoten::blatt("Ziel", opt.empfaenger_land.as_str())]);
    empf.attrs.push(("id", "L".to_owned()));
    kinder.push(empf);
    kinder.push(Knoten::blatt("HerstellerID", hid));
    kinder.push(Knoten::blatt("DatenLieferant", opt.datenlieferant.as_str()));
    kinder.push(Knoten::neu("Datei").mit(vec![
        Knoten::blatt("Verschluesselung", "CMSEncryptedData"),
        Knoten::blatt("Kompression", "GZIP"),
        Knoten::neu("TransportSchluessel"),
    ]));
    let mut th = Knoten::neu("TransferHeader").mit(kinder);
    th.attrs.push(("version", "11".to_owned()));
    th
}

struct Absender {
    name: String,
    strasse: String,
    plz: String,
    ort: String,
    steuernummer: String,
}

/// `<Vorsatz>` (`Vorsatz_67907_CType`, E10-2025.xsd:25286-25360): kein Kz-Element, letztes
/// Kind von `<E10>`. Unterfallart "10", Vorgang "01" (Veranlagung), `OrdNrArt` "S" (verlangt eine
/// `StNr` mit Finanzamts-Praefix), Bescheid "2" (keine elektronische Abholung).
fn vorsatz(vz: i64, a: Absender, datenlieferant: &str) -> Knoten {
    Knoten::neu("Vorsatz").mit(vec![
        Knoten::blatt("Unterfallart", "10"),
        Knoten::blatt("Vorgang", "01"),
        Knoten::blatt("StNr", a.steuernummer),
        Knoten::blatt("Zeitraum", vz.to_string()),
        Knoten::blatt("AbsName", a.name),
        Knoten::blatt("AbsStr", a.strasse),
        Knoten::blatt("AbsPlz", a.plz),
        Knoten::blatt("AbsOrt", a.ort),
        Knoten::blatt("Copyright", datenlieferant),
        Knoten::blatt("OrdNrArt", "S"),
        Knoten::neu("Rueckuebermittlung").mit(vec![Knoten::blatt("Bescheid", "2")]),
    ])
}

/// `_leite_absender_ab`: je Feld alle Kz vorhanden (truthy) → mit Leerzeichen verbunden.
fn leite_absender_ab(
    deklaration: &std::collections::BTreeMap<String, Value>,
    feld: &str,
) -> Option<String> {
    let (_, quellen) = ABSENDER_HERKUNFT.iter().find(|(f, _)| *f == feld)?;
    let teile: Vec<&Value> = quellen
        .iter()
        .map(|(kz, _)| deklaration.get(*kz).unwrap_or(&Value::Null))
        .collect();
    if !teile.iter().all(|t| py::truthy(t)) {
        return None;
    }
    let mut s = teile
        .iter()
        .map(|t| py::str_von(t))
        .collect::<Vec<_>>()
        .join(" ");
    if feld == "absender_strasse" {
        if let Some(z) = deklaration
            .get(ABSENDER_STRASSE_ZUSATZ_KZ)
            .filter(|z| py::truthy(z))
        {
            s.push_str(&py::str_von(z));
        }
    }
    Some(s)
}

/// `_leite_steuernummer_ab`: nur ein BESTAETIGTER Wert zaehlt.
fn leite_steuernummer_ab(snapshot: &Felder) -> Option<String> {
    let feld = snapshot.get("stammdaten_steuernummer")?;
    if feld.zustand != domain::Zustand::Bestaetigt || !py::truthy(&feld.wert) {
        return None;
    }
    Some(py::str_von(&feld.wert))
}

/// `^[0-9]{4}0[0-9]{8}$` (Python-`$`: ein abschliessendes `\n` passt mit).
fn stnr_muster(s: &str) -> bool {
    let b = s.strip_suffix('\n').unwrap_or(s).as_bytes();
    b.len() == 13 && b.iter().all(u8::is_ascii_digit) && b.get(4) == Some(&b'0')
}

fn abgabe_pruefen(result: &Deklaration, opt: &XmlOptionen<'_>) -> Result<Absender, XmlFehler> {
    let dekl = &result.deklaration;
    let keine = dekl.get("E0102002") == Some(&Value::Bool(true));
    if !(keine || dekl.contains_key("E0102102") || dekl.contains_key("E0102603")) {
        return Err(XmlFehler(
            "abgabefaehig=True verlangt eine Bankverbindungs-Entscheidung: weder IBAN (stammdaten_iban) noch \
             'keine Bankverbindung vorhanden' (stammdaten_keine_bankverbindung) ist bestaetigt. checkESt \
             akzeptiert kein Schweigen (rc=610001002)."
                .to_owned(),
        ));
    }
    let explizit = |p: &Option<String>| p.clone().filter(|s| !s.is_empty());
    let werte: Vec<(&str, Option<String>)> = [
        ("absender_name", &opt.absender_name),
        ("absender_strasse", &opt.absender_strasse),
        ("absender_plz", &opt.absender_plz),
        ("absender_ort", &opt.absender_ort),
    ]
    .into_iter()
    .map(|(feld, p)| (feld, explizit(p).or_else(|| leite_absender_ab(dekl, feld))))
    .collect();
    let mut stnr = explizit(&opt.absender_steuernummer);
    if stnr.is_none() {
        stnr = opt.snapshot.and_then(leite_steuernummer_ab);
    }
    let mut fehlend = Vec::new();
    for (feld, wert) in &werte {
        if wert.as_deref().is_some_and(|w| !w.is_empty()) {
            continue;
        }
        let quellen = ABSENDER_HERKUNFT
            .iter()
            .find(|(f, _)| f == feld)
            .map_or(&[][..], |(_, q)| *q);
        let kz: Vec<String> = quellen
            .iter()
            .filter(|(kz, _)| !dekl.get(*kz).is_some_and(py::truthy))
            .map(|(kz, label)| format!("{kz} ({label})"))
            .collect();
        fehlend.push(format!("{feld} (fehlendes Kz: {})", kz.join(", ")));
    }
    if stnr.as_deref().is_none_or(str::is_empty) {
        fehlend.push(
            "absender_steuernummer (kein Kz-Spiegel — weder als Parameter übergeben noch aus \
             snapshot['stammdaten_steuernummer'] ableitbar, s. _leite_steuernummer_ab())"
                .to_owned(),
        );
    }
    if !fehlend.is_empty() {
        let liste: Vec<String> = fehlend.iter().map(|f| format!("  - {f}")).collect();
        return Err(XmlFehler(format!(
            "abgabefaehig=True verlangt Absender-Stammdaten, es fehlen:\n{}\nsonst lehnt checkESt das XML wegen \
             fehlender Vorsatz-Pflichtfelder ab (still fuer den Nutzer; gemessen 2026-08-09: 9 \
             Vorsatz-Fehlermeldungen, gleich bei Einzel- und Zusammenveranlagung — die uebrigen, \
             fallabhaengigen Fehler bleiben davon unberuehrt).",
            liste.join("\n")
        )));
    }
    let steuernummer = stnr.unwrap_or_default();
    if !stnr_muster(&steuernummer) {
        return Err(XmlFehler(
            "absender_steuernummer entspricht nicht dem amtlichen Muster ([0-9]{4}0[0-9]{8}, \
             SteuernummerBaseCType, E10-2025.xsd:1779-1786) — 13 Ziffern, 5. Ziffer '0'. Wert nicht geloggt."
                .to_owned(),
        ));
    }
    let praefix: String = steuernummer.chars().take(4).collect();
    if praefix != opt.empfaenger_finanzamt {
        return Err(XmlFehler(format!(
            "absender_steuernummer beginnt nicht mit der Finanzamtsnummer {} (Praefix {}) — checkESt lehnt das \
             als 'Bundesfinanzamtsnummer ... unterscheiden sich' ab.",
            py::repr_str(&opt.empfaenger_finanzamt),
            py::repr_str(&praefix)
        )));
    }
    let mut it = werte.into_iter().map(|(_, w)| w.unwrap_or_default());
    Ok(Absender {
        name: it.next().unwrap_or_default(),
        strasse: it.next().unwrap_or_default(),
        plz: it.next().unwrap_or_default(),
        ort: it.next().unwrap_or_default(),
        steuernummer,
    })
}

// ---------------------------------------------------------------- Schema-Cache

fn schema_fuer(vz: i64) -> Result<Arc<SchemaInfo>, XmlFehler> {
    static CACHE: OnceLock<Mutex<HashMap<i64, Arc<SchemaInfo>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(info) = guard.get(&vz) {
        return Ok(Arc::clone(info));
    }
    let pfad = finde_schema(vz, "E10-{jahr}.xsd").ok_or_else(|| {
        XmlFehler(format!(
            "E10-{vz}.xsd nicht gefunden — $ERIC_DIR setzen / ERiC-Doku entpacken."
        ))
    })?;
    let info = Arc::new(schema_info(&pfad).map_err(|e| XmlFehler(e.to_string()))?);
    guard.insert(vz, Arc::clone(&info));
    Ok(info)
}

/// Python `repr` eines Tupels von Pfad-Segmenten.
fn tupel_repr(t: &[String]) -> String {
    let teile: Vec<String> = t.iter().map(|s| py::repr_str(s)).collect();
    if teile.len() == 1 {
        format!("({},)", teile.join(", "))
    } else {
        format!("({})", teile.join(", "))
    }
}

fn liste_repr(l: &[String]) -> String {
    let teile: Vec<String> = l.iter().map(|s| py::repr_str(s)).collect();
    format!("[{}]", teile.join(", "))
}

type InstanzMap<'d> = HashMap<&'d str, Vec<(Vec<String>, usize, &'d Value)>>;

fn baue_instanz_map<'d>(
    result: &'d Deklaration,
    pfade: &HashMap<&str, &[String]>,
    vz: i64,
) -> Result<InstanzMap<'d>, XmlFehler> {
    let mut map: InstanzMap<'d> = HashMap::new();
    for (gruppe, instanzen) in &result.anlage_instanzen {
        for inst in instanzen {
            let idx0 = usize::try_from(inst.index.saturating_sub(1)).unwrap_or(usize::MAX);
            for (kz, wert) in &inst.felder {
                let Some(kz_pfad) = pfade.get(kz.as_str()) else {
                    if E10_AUSSCHLUSS_DATENART.contains(&kz.as_str()) {
                        continue;
                    }
                    return Err(XmlFehler(format!(
                        "Gruppe '{gruppe}': Kz {kz} nicht im E10-{vz}-Schema — kann nicht deklariert werden \
                         (weder E10-Kz noch expliziter Ausschluss in E10_AUSSCHLUSS_DATENART)."
                    )));
                };
                if kz_pfad.len() < 3 {
                    return Err(XmlFehler(format!(
                        "Gruppe '{gruppe}': Kz {kz} Pfad {} zu kurz — kein E10-Container ableitbar.",
                        tupel_repr(kz_pfad)
                    )));
                }
                let tiefer = INSTANZ_CONTAINER_TIEFER
                    .iter()
                    .find(|(g, _)| g == gruppe)
                    .map(|(_, s)| *s);
                let ende = tiefer
                    .and_then(|s| kz_pfad.iter().position(|p| p == s))
                    .map_or(2, |i| i + 1);
                let container = kz_pfad.get(..ende).unwrap_or(kz_pfad).to_vec();
                map.entry(kz.as_str())
                    .or_default()
                    .push((container, idx0, wert));
            }
        }
    }
    Ok(map)
}

/// Offene Eingaben, dann — nur auf dem Abgabe-Pfad — fehlende Pflichtfelder. Konsistent heisst
/// nur "nichts widerspricht sich", ein leerer Store ist das auch; ob das Noetige da ist, sagen
/// die Pflichtluecken (Python: `pflichtfelder_vollstaendig`, `elster_xml.py::erzeuge_xml`).
fn eingaben_pruefen(result: &Deklaration, opt: &XmlOptionen<'_>) -> Result<(), XmlFehler> {
    if !result.eingaben_konsistent() {
        let offen = result.unvollstaendig();
        let erste = serde_json::to_value(offen.get(..3).unwrap_or(offen)).unwrap_or(Value::Null);
        return Err(XmlFehler(format!(
            "Deklaration unvollständig ({} offene Pflichtfelder) — kein Submission-XML. Erste: {}",
            offen.len(),
            py::repr(&erste)
        )));
    }
    if !opt.abgabefaehig || result.pflichtfelder_luecken().is_empty() {
        return Ok(());
    }
    let fehlend: Vec<&str> = result
        .pflichtfelder_luecken()
        .iter()
        .map(|e| e.feld_id.as_str())
        .collect();
    let fehlend = serde_json::to_value(fehlend).unwrap_or(Value::Null);
    Err(XmlFehler(format!(
        "abgabefaehig=True verlangt pflichtfelder_vollstaendig=True — fehlend: {}. checkESt lehnt das XML sonst ab (rc=610001002).",
        py::repr(&fehlend)
    )))
}

/// Deklaration → ELSTER-Submission-XML (`elster_xml.py::erzeuge_xml`).
///
/// Buckets: `deklaration` (Person A, Instanz 0), `person_b` (Person-Container, Instanz 1 — oder
/// 0, wenn Person A dort nichts erklaert), `anlage_instanzen` (Instanz index−1). Aeussere
/// Schleife in Schema-Reihenfolge, innere ueber die Instanzen.
///
/// # Errors
/// [`XmlFehler`] bei offenen Eingaben, fehlender Hersteller-ID, fehlendem Schema, Kz ohne
/// Schema-Pfad, inkonsistenter Kinderzahl und — mit `abgabefaehig` — fehlenden Pflichtfeldern,
/// fehlender Bankverbindungs-Entscheidung, fehlendem Absender oder ungueltiger Steuernummer.
///
/// ```
/// use std::collections::HashMap;
/// use elster::{deklariere, erzeuge_xml, Felder, XmlOptionen};
/// let d = deklariere(&Felder::new(), &HashMap::new(), None).unwrap();
/// let opt = XmlOptionen { hersteller_id: Some("00000".into()), ..XmlOptionen::default() };
/// if elster::finde_schema(2025, "E10-{jahr}.xsd").is_some() {
///     let xml = erzeuge_xml(&d, &opt).unwrap();
///     assert!(xml.contains("<E0100001>X</E0100001>"));
/// }
/// ```
pub fn erzeuge_xml(result: &Deklaration, opt: &XmlOptionen<'_>) -> Result<String, XmlFehler> {
    eingaben_pruefen(result, opt)?;
    let dekl = &result.deklaration;
    if dekl.is_empty() {
        return Err(XmlFehler(
            "leere Deklaration — nichts zu übermitteln.".to_owned(),
        ));
    }
    let hid = opt
        .hersteller_id
        .clone()
        .filter(|h| !h.is_empty())
        .or_else(|| std::env::var("ELSTER_HERSTELLER_ID").ok())
        .filter(|h| !h.is_empty())
        .ok_or_else(|| {
            XmlFehler(
                "keine Hersteller-ID — $ELSTER_HERSTELLER_ID setzen (nie im Repo, nie im Code)."
                    .to_owned(),
            )
        })?;
    let vz = opt.vz;
    let info = schema_fuer(vz)?;
    let pfade: HashMap<&str, &[String]> = info
        .pfade
        .iter()
        .map(|(k, p)| (k.as_str(), p.as_slice()))
        .collect();
    let mut fehlend: Vec<String> = dekl
        .keys()
        .filter(|k| {
            !pfade.contains_key(k.as_str()) && !E10_AUSSCHLUSS_DATENART.contains(&k.as_str())
        })
        .cloned()
        .collect();
    fehlend.sort();
    if !fehlend.is_empty() {
        return Err(XmlFehler(format!(
            "{} Kz ohne Pfad im E10-{vz}-Schema: {}",
            fehlend.len(),
            liste_repr(fehlend.get(..5).unwrap_or(&fehlend))
        )));
    }
    if !result.kind_anlagen.is_empty() {
        let behauptet = result.kind_anlagen.len();
        let vorhanden = 1 + result.instanzen_der_gruppe("kind").len();
        if behauptet != vorhanden {
            return Err(XmlFehler(format!(
                "Kinderzahl behauptet {behauptet}, Kind-Daten für {vorhanden} vorhanden — kind_anlagen vs \
                 anlage_instanzen[kind] inkonsistent."
            )));
        }
    }
    let instanz_map = baue_instanz_map(result, &pfade, vz)?;
    let mut fehlend_b: Vec<String> = result
        .person_b
        .keys()
        .filter(|k| !pfade.contains_key(k.as_str()))
        .cloned()
        .collect();
    fehlend_b.sort();
    if !fehlend_b.is_empty() {
        return Err(XmlFehler(format!(
            "{} Kz in person_b ohne Pfad: {}",
            fehlend_b.len(),
            liste_repr(fehlend_b.get(..5).unwrap_or(&fehlend_b))
        )));
    }
    let mut e10 = Knoten::neu("E10");
    e10.attrs.push((
        "xmlns",
        format!("http://finkonsens.de/elster/elstererklaerung/est/e10/v{vz}"),
    ));
    e10.attrs.push(("version", vz.to_string()));
    kz_schleife(&mut e10, result, &info, &pfade, &instanz_map)?;
    if opt.abgabefaehig {
        let absender = abgabe_pruefen(result, opt)?;
        e10.kinder.push(vorsatz(vz, absender, &opt.datenlieferant));
    }
    let mut empf_f = Knoten::blatt("Empfaenger", opt.empfaenger_finanzamt.as_str());
    empf_f.attrs.push(("id", "F".to_owned()));
    let mut nh = Knoten::neu("NutzdatenHeader").mit(vec![
        Knoten::blatt("NutzdatenTicket", opt.nutzdaten_ticket.as_str()),
        empf_f,
    ]);
    nh.attrs.push(("version", "11".to_owned()));
    let block =
        Knoten::neu("Nutzdatenblock").mit(vec![nh, Knoten::neu("Nutzdaten").mit(vec![e10])]);
    let mut wurzel = Knoten::neu("Elster").mit(vec![
        transfer_header(opt, &hid),
        Knoten::neu("DatenTeil").mit(vec![block]),
    ]);
    wurzel.attrs.push(("xmlns", NS_ELSTER.to_owned()));
    serialisiere(&wurzel)
}

/// Die Kz-Schleife in Schema-Reihenfolge: Person A, Person B, Anlage-Instanzen.
fn kz_schleife(
    e10: &mut Knoten,
    result: &Deklaration,
    info: &SchemaInfo,
    pfade: &HashMap<&str, &[String]>,
    instanz_map: &InstanzMap<'_>,
) -> Result<(), XmlFehler> {
    let pflicht = &info.pflicht;
    let dekl = &result.deklaration;
    // Welche Container belegt Instanz 0 (Person A, und index=1-Instanzen)?
    let mut instanz_null: Vec<&[String]> = dekl
        .keys()
        .filter_map(|k| pfade.get(k.as_str()).copied())
        .collect();
    for (kz, eintraege) in instanz_map {
        if eintraege.iter().any(|(_, i, _)| *i == 0) {
            instanz_null.extend(pfade.get(kz).copied());
        }
    }
    let person_container = |pfad: &[String]| -> Option<Vec<String>> {
        (2..=pfad.len())
            .rev()
            .filter_map(|i| pfad.get(..i))
            .find(|c| {
                pflicht
                    .get(*c)
                    .is_some_and(|p| p.iter().any(|k| k == "Person"))
            })
            .map(<[String]>::to_vec)
    };
    let mut person_b_index: HashMap<Vec<String>, usize> = HashMap::new();
    for (kz, pfad) in &info.pfade {
        if let Some(w) = dekl.get(kz) {
            einhaengen(e10, pfad, w, pflicht, &info.kz_meta, None)?;
        }
        if let Some(w) = result.person_b.get(kz) {
            let cp = person_container(pfad).ok_or_else(|| {
                XmlFehler(format!(
                    "person_b-Kz {kz} liegt in Container ohne Person-Diskriminator (Pfad: {}) — kann nicht als \
                     PersonB-Instanz geschrieben werden.",
                    pfad.join("/")
                ))
            })?;
            // Person B rueckt auf Instanz 0, wenn Person A in diesem Container nichts erklaert —
            // sonst entstuende eine leere PersonA-Huelle, die checkESt beanstandet (2026-08-20).
            let index = *person_b_index
                .entry(cp.clone())
                .or_insert_with(|| usize::from(instanz_null.iter().any(|p| p.starts_with(&cp))));
            let wahl = Instanzwahl {
                container: &cp,
                index,
                person_b: true,
            };
            einhaengen(e10, pfad, w, pflicht, &info.kz_meta, Some(&wahl))?;
        }
        for (container, idx0, w) in instanz_map.get(kz.as_str()).map_or(&[][..], Vec::as_slice) {
            let wahl = Instanzwahl {
                container,
                index: *idx0,
                person_b: false,
            };
            einhaengen(e10, pfad, w, pflicht, &info.kz_meta, Some(&wahl))?;
        }
    }
    Ok(())
}
