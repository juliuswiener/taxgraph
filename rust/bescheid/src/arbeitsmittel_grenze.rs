//! Die 800-EUR-Grenze des Arbeitsmittel-Sofortabzugs beim Arbeitnehmer (Abweichung Nr. 88).
//!
//! **Regel** (Julius, 2026-10-19, R 9.12 `LStR`; im Korpus `source_unavailable`, die Anleitung zur Anlage N 2025 sagt
//! „höchstens 800 € (ohne Umsatzsteuer)“): Geprueft wird der Preis OHNE Umsatzsteuer gegen 800,00 EUR, abgezogen wird der volle
//! Bruttopreis. Die Bruttogrenze folgt dem Satz der Rechnung: 19 % ergibt 952,00 EUR, 7 % ergibt 856,00 EUR, ohne Umsatzsteuer
//! bleibt es bei 800,00 EUR.
//!
//! **Rechnung in ganzen Cent, ohne Rundung.** Netto `<= 800,00` ist `brutto * 100 <= 80.000 * (100 + Satz)`; links wie rechts
//! stehen ganze Zahlen. Die Bruttogrenze ist darum `80.000 * (100 + Satz) / 100`, ganzzahlig und ohne Rest fuer 19, 7 und 0
//! (95.200, 85.600, 80.000). Es gibt keine Rundungsrichtung, die die Grenze verschieben koennte. Ein Satz mit Rest in der
//! Division liefe nach unten, und das waere richtig: `brutto` ist ganz, `brutto * 100 <= 80.000 * (100 + Satz)` gilt genau
//! bis zum abgerundeten Quotienten.
//!
//! Beide Stellen, die die Grenze brauchen (die Rechnung in `zweige::wk` und die Sperre in `deklaration::sperre`), lesen diese
//! Datei; eine zweite Fassung der Zahlen waere der Fehler, den Nr. 88 behebt.
use domain::PyWert;
use rust_decimal::Decimal;

/// § 6 Abs. 2 Satz 1 `EStG`: 800 EUR in CENT (`bescheid_zweige.py:293`).
pub(crate) const GWG_GRENZE_CENT: i64 = 80_000;

// Die Sätze stehen hier als benannte Konstanten, nicht in `params/`: § 12 UStG liegt nicht im Korpus (`source_unavailable`), und
// die zwei Bruttogrenzen 952,00 und 856,00 EUR hat Julius am 2026-10-19 genannt (explicit).
// ponytail: zwei feste Saetze; ein dritter Satz braeuchte einen Enum-Wert in `bindung_n_vor_gwg.yaml` und eine Zeile in
// `satz_prozent`. Aendert sich ein Satz, aendern sich die Konstante und der Text der Frage.
/// Regelsatz der Umsatzsteuer in Prozent (§ 12 Abs. 1 `UStG`).
const REGELSATZ_PROZENT: i64 = 19;
/// Ermaessigter Satz in Prozent (§ 12 Abs. 2 `UStG`), zum Beispiel Fachbuecher.
const ERMAESSIGT_PROZENT: i64 = 7;

/// Bruttogrenze in CENT, bis zu der der Preis netto nicht ueber 800,00 EUR liegt, bei einem Satz von `prozent` Prozent.
pub(crate) const fn bruttogrenze_cent(prozent: i64) -> i64 {
    GWG_GRENZE_CENT * (100 + prozent) / 100
}

/// Das obere Ende des Bandes: darueber ist die Grenze bei jedem Satz bis zum Regelsatz ueberschritten, der Satz aendert nichts.
const BAND_OBEN_CENT: i64 = bruttogrenze_cent(REGELSATZ_PROZENT);

/// Der Satz einer Antwort auf `am_umsatzsteuersatz`, in Prozent. `None`: unbeantwortet oder kein bekannter Wert. Nie ein
/// stiller Satz.
pub(crate) fn satz_prozent(antwort: Option<&PyWert>) -> Option<i64> {
    match antwort {
        Some(PyWert::Text(s)) => match s.as_str() {
            "satz_19" => Some(REGELSATZ_PROZENT),
            "satz_7" => Some(ERMAESSIGT_PROZENT),
            "keine_ust" => Some(0),
            _ => None,
        },
        _ => None,
    }
}

/// Darf ein Arbeitsmittel zum Bruttopreis `preis_cent` sofort abgezogen werden, soweit es die Grenze angeht?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sofortabzug {
    /// Netto hoechstens 800,00 EUR.
    Moeglich,
    /// Netto ueber 800,00 EUR: der Abzug laeuft ueber die Jahre (`AfA`).
    Nein,
    /// Der Satz entscheidet (Band ueber 800,00 EUR bis 952,00 EUR brutto) und ist nicht beantwortet.
    SatzOffen,
}

/// Die Grenze fuer einen Bruttopreis (CENT als `Decimal`: die Sperre liest den Rohwert, auch eine Kommazahl) und den Satz in
/// Prozent (`None` = unbeantwortet).
///
/// Ohne Satz gilt: bis 800,00 EUR brutto ist der Abzug bei jedem Satz moeglich, ueber 952,00 EUR bei keinem; dazwischen ist
/// die Antwort offen. Mit Satz entscheidet dessen Bruttogrenze.
pub(crate) fn sofortabzug(preis_cent: Decimal, satz: Option<i64>) -> Sofortabzug {
    let bis = |grenze: i64| preis_cent <= Decimal::from(grenze);
    match satz {
        Some(p) if bis(bruttogrenze_cent(p)) => Sofortabzug::Moeglich,
        Some(_) => Sofortabzug::Nein,
        None if bis(GWG_GRENZE_CENT) => Sofortabzug::Moeglich,
        None if !bis(BAND_OBEN_CENT) => Sofortabzug::Nein,
        None => Sofortabzug::SatzOffen,
    }
}

// ---------------------------------------------------------------- Computer und Software (Abweichung Nr. 89)

/// **Regel** (BMF-Schreiben vom 22.02.2022, `BStBl` I S. 187, `sources/bmf/bmf_nutzungsdauer_computer_software_2022-02-22.txt`): Computerhardware
/// und Software der Liste (Rz. 2 bis 5) haben eine betriebsgewoehnliche Nutzungsdauer von einem Jahr (Rz. 1). Das ist eine normale
/// Abschreibung nach § 7 Abs. 1 `EStG` (Rz. 1.1), kein `GWG` und keine Sofortabschreibung; der volle Abzug im Anschaffungsjahr wird nicht
/// beanstandet (Rz. 1.4), ein Zwoelftel nach dem Kaufmonat entfaellt. Wer davon abweicht, verteilt laenger (Rz. 1.2).
///
/// Ein Geraet der Liste durchlaeuft darum weder die Netto-Pruefung noch die Satzfrage (oben). Diese Funktion legt fest, was als „ja“ zaehlt: nur
/// ein `true`. Wer ein BESTAETIGTES „ja“ verlangt, uebergibt die Antwort nur, wenn sie bestaetigt ist (die Sperre prueft das vorher; die
/// Rechnung liest einen Stand mit bestaetigten Feldern).
pub(crate) fn computer_ja(antwort: Option<&PyWert>) -> bool {
    matches!(antwort, Some(PyWert::Bool(true)))
}

/// Wie ein Geraet der Liste abgeschrieben wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComputerWeg {
    /// Nutzungsdauer ein Jahr (Rz. 1): voller Abzug im Anschaffungsjahr (Rz. 1.4), in jedem anderen Jahr 0 (das Gut ist aufgebraucht).
    EinJahr,
    /// Eigene Nutzungsdauer ueber ein Jahr (Rz. 1.2): die Verteilung wie vor Nr. 89, mit dem Zwoelftel nach dem Kaufmonat.
    Verteilung,
}

/// Der Weg bei der eigenen Nutzungsdauer in Jahren (`0` oder weniger = keine Angabe, die Vorgabe ist ein Jahr).
pub(crate) fn computer_weg(nutzungsdauer: i64) -> ComputerWeg {
    if nutzungsdauer > 1 {
        ComputerWeg::Verteilung
    } else {
        ComputerWeg::EinJahr
    }
}

/// Bleibt bei einem bestaetigten „ja“ etwas offen, das die Zahl aendert? Eine Stelle fuer die Sperre; die Rechnung liest dieselben Antworten.
///
/// - `nutzungsdauer_vorlaeufig`: die eigene Nutzungsdauer steht da, ist aber nicht bestaetigt. Mit der Bestaetigung wechselte die Zahl
///   zwischen dem vollen Abzug und der Verteilung.
/// - `kaufjahr`: die BESTAETIGTE Antwort auf „in diesem Jahr gekauft“ (`None` = keine). Sie entscheidet bei einem Jahr zwischen dem
///   vollen Abzug und 0, darum gibt es hier keinen stillen Folgejahr-Wert.
/// - `monat`: der Kaufmonat; nur die Verteilung im Anschaffungsjahr braucht ihn, ein Jahr nicht.
pub(crate) fn computer_offen(
    nutzungsdauer_vorlaeufig: bool,
    nutzungsdauer: i64,
    kaufjahr: Option<bool>,
    monat: i64,
) -> bool {
    if nutzungsdauer_vorlaeufig {
        return true;
    }
    match kaufjahr {
        None => true,
        Some(im_kaufjahr) => {
            computer_weg(nutzungsdauer) == ComputerWeg::Verteilung
                && im_kaufjahr
                && !(1..=12).contains(&monat)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nur ein `true` ist „ja“; `false`, Text, Zahl und keine Antwort sind es nicht.
    #[test]
    fn nur_ein_true_ist_ja() {
        assert!(computer_ja(Some(&PyWert::Bool(true))));
        assert!(!computer_ja(Some(&PyWert::Bool(false))));
        assert!(!computer_ja(Some(&PyWert::Text("ja".to_owned()))));
        assert!(!computer_ja(Some(&PyWert::Ganz(1))));
        assert!(!computer_ja(None));
    }

    /// Ein Jahr ist der Normalfall (keine Angabe, 0 oder 1), jede laengere eigene Nutzungsdauer ist die Verteilung.
    #[test]
    fn ein_jahr_ist_die_vorgabe_und_mehr_ist_die_verteilung() {
        for (nd, soll) in [
            (-1, ComputerWeg::EinJahr),
            (0, ComputerWeg::EinJahr),
            (1, ComputerWeg::EinJahr),
            (2, ComputerWeg::Verteilung),
            (3, ComputerWeg::Verteilung),
            (30, ComputerWeg::Verteilung),
        ] {
            assert_eq!(computer_weg(nd), soll, "Nutzungsdauer {nd}");
        }
    }

    /// Was bei „ja“ offen bleibt: das Jahr der Anschaffung immer, die vorlaeufige Nutzungsdauer immer, der Monat nur bei einer
    /// Verteilung im Anschaffungsjahr.
    #[test]
    fn offen_bleiben_das_anschaffungsjahr_die_vorlaeufige_nutzungsdauer_und_der_monat_der_verteilung() {
        // (Name, Nutzungsdauer vorlaeufig, Nutzungsdauer, Anschaffungsjahr bestaetigt, Monat, offen)
        let faelle = [
            ("ein Jahr, Kaufjahr, ohne Monat", false, 0, Some(true), 0, false),
            ("ein Jahr, Folgejahr", false, 0, Some(false), 0, false),
            ("eigene 1, Kaufjahr, ohne Monat", false, 1, Some(true), 0, false),
            ("Anschaffungsjahr unbeantwortet", false, 0, None, 7, true),
            ("Verteilung 3, Kaufjahr, ohne Monat", false, 3, Some(true), 0, true),
            ("Verteilung 3, Kaufjahr, Monat 0", false, 3, Some(true), 0, true),
            ("Verteilung 3, Kaufjahr, Monat 13", false, 3, Some(true), 13, true),
            ("Verteilung 3, Kaufjahr, Monat 1", false, 3, Some(true), 1, false),
            ("Verteilung 3, Kaufjahr, Monat 12", false, 3, Some(true), 12, false),
            ("Verteilung 3, Folgejahr, ohne Monat", false, 3, Some(false), 0, false),
            ("Verteilung 3, Anschaffungsjahr unbeantwortet", false, 3, None, 7, true),
            ("Nutzungsdauer vorlaeufig", true, 0, Some(true), 7, true),
        ];
        for (name, vorlaeufig, nd, jahr, monat, soll) in faelle {
            assert_eq!(computer_offen(vorlaeufig, nd, jahr, monat), soll, "{name}");
        }
    }

    /// Die drei Bruttogrenzen von Julius, in Cent und ohne Rest.
    #[test]
    fn die_bruttogrenzen_sind_952_856_und_800_euro() {
        assert_eq!(bruttogrenze_cent(19), 95_200);
        assert_eq!(bruttogrenze_cent(7), 85_600);
        assert_eq!(bruttogrenze_cent(0), 80_000);
        for p in [19, 7, 0] {
            assert_eq!(GWG_GRENZE_CENT * (100 + p) % 100, 0, "Rest bei {p} %");
        }
    }

    /// Die Grenze selbst und ein Cent darueber, je Satz; ohne Satz zaehlt das Band.
    #[test]
    fn grenzfaelle_je_satz() {
        use Sofortabzug::{Moeglich, Nein, SatzOffen};
        let faelle = [
            (95_200, Some(19), Moeglich),
            (95_201, Some(19), Nein),
            (85_600, Some(7), Moeglich),
            (85_601, Some(7), Nein),
            (80_000, Some(0), Moeglich),
            (80_001, Some(0), Nein),
            (90_000, Some(7), Nein),
            (80_000, None, Moeglich),
            (80_001, None, SatzOffen),
            (95_200, None, SatzOffen),
            (95_201, None, Nein),
        ];
        for (preis, satz, soll) in faelle {
            assert_eq!(sofortabzug(Decimal::from(preis), satz), soll, "{preis} Cent, Satz {satz:?}");
        }
    }

    /// Auch eine Kommazahl im Rohwert liegt auf der richtigen Seite: ein halber Cent ueber der Grenze ist darueber.
    #[test]
    fn ein_halber_cent_ueber_der_grenze_ist_darueber() {
        let halb = |ganz: i64| Decimal::from(ganz) + Decimal::new(5, 1);
        assert_eq!(sofortabzug(halb(95_200), Some(19)), Sofortabzug::Nein);
        assert_eq!(sofortabzug(halb(80_000), None), Sofortabzug::SatzOffen);
        assert_eq!(sofortabzug(halb(95_200), None), Sofortabzug::Nein);
    }

    /// Nur die drei Werte der Bindung sind ein Satz; alles andere ist keine Antwort.
    #[test]
    fn nur_die_drei_enumwerte_sind_ein_satz() {
        let text = |s: &str| PyWert::Text(s.to_owned());
        assert_eq!(satz_prozent(Some(&text("satz_19"))), Some(19));
        assert_eq!(satz_prozent(Some(&text("satz_7"))), Some(7));
        assert_eq!(satz_prozent(Some(&text("keine_ust"))), Some(0));
        assert_eq!(satz_prozent(Some(&text("19"))), None);
        assert_eq!(satz_prozent(Some(&PyWert::Ganz(19))), None);
        assert_eq!(satz_prozent(None), None);
    }
}
