//! Die Typ-Algebra ueber Zustand und Herkunft (`store.py:45-72`, "Julius #2": *"Der Typ ist
//! Enforcement, nicht Bitte"*). Ein Aggregat ist nur `bestaetigt`, wenn ALLE Eingaben
//! `bestaetigt` sind (Meet ueber den Input-Kegel).
use super::herkunft::{Achsenwert, Herkunft};
use super::zustand::{PruefTiefe, Zustand};

/// Meet ueber den Input-Kegel: `bestaetigt` nur, wenn ALLE Eingaben `bestaetigt` sind; ein leeres
/// Aggregat ist `bestaetigt` (neutrales Element, kein Input-Kegel vorhanden) — `store.py:52-57`.
///
/// Kommutativ, assoziativ und idempotent mit `Bestaetigt` als Identitaet (siehe Proptests).
///
/// ```
/// use domain::{meet_zustand, Zustand};
/// assert_eq!(meet_zustand([Zustand::Bestaetigt, Zustand::Bestaetigt]), Zustand::Bestaetigt);
/// assert_eq!(meet_zustand([Zustand::Bestaetigt, Zustand::Vorlaeufig]), Zustand::Vorlaeufig);
/// assert_eq!(meet_zustand(std::iter::empty()), Zustand::Bestaetigt);
/// ```
pub fn meet_zustand(zustaende: impl IntoIterator<Item = Zustand>) -> Zustand {
    let mut alle_bestaetigt = true;
    for z in zustaende {
        if z != Zustand::Bestaetigt {
            alle_bestaetigt = false;
        }
    }
    if alle_bestaetigt {
        Zustand::Bestaetigt
    } else {
        Zustand::Vorlaeufig
    }
}

/// Meet pro Achse: `herkunft`/`haftung` sind bei Uneinigkeit `"konflikt"` (nie automatisch
/// versoehnt), `pruef_tiefe` ist das Minimum (die schwaechste Kette bestimmt die Staerke der
/// Summe). Leerer Input liefert die Identitaet `{herkunft: "berechnet", pruef_tiefe: Amtlich,
/// haftung: "system"}` (`store.py:64-65`).
#[must_use]
pub fn meet_herkunft(vektoren: impl IntoIterator<Item = Herkunft>) -> Herkunft {
    let vs: Vec<Herkunft> = vektoren.into_iter().collect();
    let Some((erster, rest)) = vs.split_first() else {
        return Herkunft {
            herkunft: achsenwert_bekannt_nicht_leer("berechnet"),
            pruef_tiefe: PruefTiefe::Amtlich,
            haftung: achsenwert_bekannt_nicht_leer("system"),
        };
    };

    let herkunft = meet_achse(erster.herkunft.as_str(), rest.iter().map(|v| v.herkunft.as_str()));
    let haftung = meet_achse(erster.haftung.as_str(), rest.iter().map(|v| v.haftung.as_str()));
    let pruef_tiefe = vs
        .iter()
        .map(|v| v.pruef_tiefe)
        .fold(PruefTiefe::Amtlich, PruefTiefe::min);

    Herkunft { herkunft, pruef_tiefe, haftung }
}

/// `Achsenwert` aus einem bekannt nicht-leeren `&str` — fuer die Meet-Identitaet und die
/// Konflikt-Markierung, an denen die Leerheits-Pruefung nie greifen kann.
///
/// # Panics
/// Nie: jeder Aufrufer in diesem Modul uebergibt entweder ein nicht-leeres Literal oder einen
/// bereits validierten [`Achsenwert`], nie einen leeren String.
fn achsenwert_bekannt_nicht_leer(s: &str) -> Achsenwert {
    #[allow(clippy::unwrap_used)]
    Achsenwert::new(s.to_owned()).unwrap()
}

fn meet_achse<'a>(erster: &'a str, rest: impl Iterator<Item = &'a str>) -> Achsenwert {
    let einig = rest.into_iter().all(|w| w == erster);
    achsenwert_bekannt_nicht_leer(if einig { erster } else { "konflikt" })
}

#[cfg(test)]
mod tests {
    use super::meet_zustand;
    use crate::Zustand;
    use proptest::prelude::*;

    fn zustand_strategy() -> impl Strategy<Value = Zustand> {
        prop_oneof![Just(Zustand::Bestaetigt), Just(Zustand::Vorlaeufig)]
    }

    proptest! {
        #[test]
        fn kommutativ(a in zustand_strategy(), b in zustand_strategy()) {
            prop_assert_eq!(meet_zustand([a, b]), meet_zustand([b, a]));
        }

        #[test]
        fn assoziativ(a in zustand_strategy(), b in zustand_strategy(), c in zustand_strategy()) {
            let links = meet_zustand([meet_zustand([a, b]), c]);
            let rechts = meet_zustand([a, meet_zustand([b, c])]);
            prop_assert_eq!(links, rechts);
        }

        #[test]
        fn idempotent(a in zustand_strategy()) {
            prop_assert_eq!(meet_zustand([a, a]), a);
        }

        #[test]
        fn bestaetigt_ist_identitaet(a in zustand_strategy()) {
            prop_assert_eq!(meet_zustand([a, Zustand::Bestaetigt]), a);
        }
    }

    #[test]
    fn leeres_aggregat_ist_bestaetigt() {
        assert_eq!(meet_zustand(std::iter::empty()), Zustand::Bestaetigt);
    }
}
