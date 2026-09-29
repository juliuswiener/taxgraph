//! Die Typ-Algebra ueber Zustand und Herkunft (`store.py:45-72`, "Julius #2": *"Der Typ ist
//! Enforcement, nicht Bitte"*). Ein Aggregat ist nur `bestaetigt`, wenn ALLE Eingaben
//! `bestaetigt` sind (Meet ueber den Input-Kegel).
use super::herkunft::{Achsenwert, Herkunft, HerkunftVektor};
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

/// `meet_herkunft` scheitert, wenn einer der Eingabe-Vektoren die Alt-Form traegt (`herkunft`
/// ohne `pruef_tiefe`/`haftung`, s. [`HerkunftVektor`]). Python liest dort zuerst
/// `v["pruef_tiefe"]` (`store.py:71`, VOR jeder Achsen-Kategorisierung) — auf einem Alt-Dict
/// wirft das ein `KeyError`, bevor `herkunft`/`haftung` ueberhaupt gelesen werden. Rust bricht an
/// derselben Stelle ab, statt eine erfundene `pruef_tiefe`/`haftung` einzusetzen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Herkunfts-Vektor in Alt-Form (ohne pruef_tiefe/haftung) kann nicht gemeinsam gemeint werden")]
pub struct MeetFehler;

/// Meet pro Achse: `herkunft`/`haftung` sind bei Uneinigkeit `"konflikt"` (nie automatisch
/// versoehnt), `pruef_tiefe` ist das Minimum (die schwaechste Kette bestimmt die Staerke der
/// Summe). Leerer Input liefert die Identitaet `{herkunft: "berechnet", pruef_tiefe: Amtlich,
/// haftung: "system"}` (`store.py:64-65`).
///
/// # Errors
/// [`MeetFehler`], wenn ein Vektor die Alt-Form traegt (s. [`HerkunftVektor`]/`store.py:71`).
pub fn meet_herkunft(vektoren: impl IntoIterator<Item = HerkunftVektor>) -> Result<Herkunft, MeetFehler> {
    let vs: Vec<HerkunftVektor> = vektoren.into_iter().collect();
    if vs.is_empty() {
        return Ok(Herkunft {
            herkunft: achsenwert_bekannt_nicht_leer("berechnet"),
            pruef_tiefe: PruefTiefe::Amtlich,
            haftung: achsenwert_bekannt_nicht_leer("system"),
        });
    }
    let volle: Vec<&Herkunft> = vs.iter().map(HerkunftVektor::als_voll).collect::<Option<_>>().ok_or(MeetFehler)?;
    // `vs` (und damit `volle`) ist hier nachweislich nicht leer (frueher return oben).
    let Some((erster, rest)) = volle.split_first() else { return Err(MeetFehler) };

    let herkunft = meet_achse(erster.herkunft.as_str(), rest.iter().map(|v| v.herkunft.as_str()));
    let haftung = meet_achse(erster.haftung.as_str(), rest.iter().map(|v| v.haftung.as_str()));
    let pruef_tiefe = volle
        .iter()
        .map(|v| v.pruef_tiefe)
        .fold(PruefTiefe::Amtlich, PruefTiefe::min);

    Ok(Herkunft { herkunft, pruef_tiefe, haftung })
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

    mod herkunft {
        use super::super::{achsenwert_bekannt_nicht_leer, meet_herkunft, MeetFehler};
        use crate::{Herkunft, HerkunftAlt, HerkunftVektor, PruefTiefe};

        fn voll(herkunft: &str, tiefe: PruefTiefe, haftung: &str) -> HerkunftVektor {
            Herkunft {
                herkunft: achsenwert_bekannt_nicht_leer(herkunft),
                pruef_tiefe: tiefe,
                haftung: achsenwert_bekannt_nicht_leer(haftung),
            }
            .into()
        }

        fn alt(herkunft: &str) -> HerkunftVektor {
            HerkunftVektor::Alt(HerkunftAlt { herkunft: achsenwert_bekannt_nicht_leer(herkunft) })
        }

        #[test]
        fn leerer_input_liefert_identitaet() {
            let h = meet_herkunft(std::iter::empty()).unwrap();
            assert_eq!(h.herkunft.as_str(), "berechnet");
            assert_eq!(h.pruef_tiefe, PruefTiefe::Amtlich);
            assert_eq!(h.haftung.as_str(), "system");
        }

        #[test]
        fn einigkeit_bleibt_erhalten_minimum_der_pruef_tiefe() {
            let h = meet_herkunft([
                voll("mensch", PruefTiefe::Amtlich, "nutzer"),
                voll("mensch", PruefTiefe::Ungeprueft, "nutzer"),
            ])
            .unwrap();
            assert_eq!(h.herkunft.as_str(), "mensch");
            assert_eq!(h.pruef_tiefe, PruefTiefe::Ungeprueft);
            assert_eq!(h.haftung.as_str(), "nutzer");
        }

        #[test]
        fn uneinigkeit_wird_konflikt() {
            let h = meet_herkunft([
                voll("mensch", PruefTiefe::Amtlich, "nutzer"),
                voll("edaten", PruefTiefe::Amtlich, "amt"),
            ])
            .unwrap();
            assert!(h.herkunft.ist_konflikt());
            assert!(h.haftung.ist_konflikt());
        }

        /// Roter Beweis fuer die Alt-Form: ein einzelner Alt-Vektor im Input muss scheitern —
        /// genau wie Pythons `KeyError` auf `v["pruef_tiefe"]` (`store.py:71`). Wer diesen Zweig
        /// entfernt (z. B. `als_voll` durch ein `unwrap_or`-Fallback ersetzt), macht diesen Test
        /// rot.
        #[test]
        fn alt_vektor_im_gemisch_scheitert() {
            let fehler = meet_herkunft([voll("mensch", PruefTiefe::Amtlich, "nutzer"), alt("vorjahr")]).unwrap_err();
            assert_eq!(fehler, MeetFehler);
        }

        #[test]
        fn einzelner_alt_vektor_scheitert() {
            assert_eq!(meet_herkunft([alt("vorjahr")]).unwrap_err(), MeetFehler);
        }
    }
}
