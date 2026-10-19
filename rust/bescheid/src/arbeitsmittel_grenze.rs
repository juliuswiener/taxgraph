//! Die 800-EUR-Grenze des Arbeitsmittel-Sofortabzugs beim Arbeitnehmer (Abweichung Nr. 88).
//!
//! **Regel** (Julius, 2026-10-19, R 9.12 LStR; im Korpus `source_unavailable`, die Anleitung zur Anlage N 2025 sagt
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
/// Regelsatz der Umsatzsteuer in Prozent (§ 12 Abs. 1 UStG).
const REGELSATZ_PROZENT: i64 = 19;
/// Ermaessigter Satz in Prozent (§ 12 Abs. 2 UStG), zum Beispiel Fachbuecher.
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

#[cfg(test)]
mod tests {
    use super::*;

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
