//! Die Gegenprobe VOR jedem ERiC-Aufruf: das XML muss den Merker tragen, den der Aufrufer
//! behauptet (`versand.py::_pruefe_merker_konsistenz`).
//!
//! Abweichung zu Python, nur strenger: der Merker wird aus dem XML-Baum gelesen, nicht mit einer
//! Regex gesucht. Python laesst `<!-- <Testmerker>700000004</Testmerker> -->` oder einen Merker
//! ausserhalb von `TransferHeader` als Testmerker durch; ERiC sieht dann KEINEN Testmerker und
//! sendet echt, obwohl der Aufrufer Testversand behauptet.

use roxmltree::Document;

use crate::{Modus, VersandFehler, TESTMERKER};

/// Ein `Testmerker`-Element im XML.
struct Fund {
    text: String,
    /// Kind von `TransferHeader` (nur dort wertet ERiC den Merker).
    im_kopf: bool,
}

fn funde(xml: &[u8]) -> Result<Vec<Fund>, VersandFehler> {
    let text = std::str::from_utf8(xml).map_err(|_| VersandFehler::XmlUnlesbar)?;
    let dok = Document::parse(text).map_err(|_| VersandFehler::XmlUnlesbar)?;
    Ok(dok
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Testmerker")
        .map(|n| Fund {
            text: n.text().unwrap_or_default().to_owned(),
            im_kopf: n
                .parent_element()
                .is_some_and(|p| p.tag_name().name() == "TransferHeader"),
        })
        .collect())
}

/// Der Text des ersten `Testmerker`-Elements; `None`, wenn es keins gibt oder das XML nicht lesbar ist.
#[must_use]
pub fn merker_im_xml(xml: &[u8]) -> Option<String> {
    funde(xml).ok()?.into_iter().next().map(|f| f.text)
}

/// Passt das XML zum behaupteten Modus? Sonst `XmlMerkerMismatch` (oder `XmlUnlesbar`).
///
/// # Errors
/// [`VersandFehler::XmlMerkerMismatch`], wenn Testversand ohne genau einen passenden Merker in
/// `TransferHeader` oder Echtversand mit irgendeinem `Testmerker`-Element angefordert wird.
pub fn pruefe_merker_konsistenz(xml: &[u8], modus: &Modus) -> Result<(), VersandFehler> {
    let funde = funde(xml)?;
    match modus {
        Modus::Testversand => {
            let beschreibung = match funde.as_slice() {
                [einziger] if einziger.im_kopf && einziger.text == TESTMERKER => return Ok(()),
                [] => "keinen Testmerker".to_owned(),
                [einziger] if !einziger.im_kopf => {
                    "einen Testmerker ausserhalb von TransferHeader".to_owned()
                }
                [einziger] => format!("den Testmerker {:?}", einziger.text),
                _ => "mehrere Testmerker".to_owned(),
            };
            Err(VersandFehler::XmlMerkerMismatch(format!(
                "Testversand angefordert (Merker {TESTMERKER:?}), aber das XML traegt {beschreibung}. \
                 Abgebrochen VOR jedem ERiC-Aufruf — kein Versand."
            )))
        }
        Modus::Echtversand { .. } => match funde.first() {
            None => Ok(()),
            Some(f) => Err(VersandFehler::XmlMerkerMismatch(format!(
                "Echtversand angefordert (kein Testmerker), aber das XML traegt noch ein \
                 Testmerker-Element ({:?}). Abgebrochen VOR jedem ERiC-Aufruf — kein Versand. \
                 Das waere ohnehin nur in der Clearingstelle verworfen worden, aber der Aufrufer \
                 hat offensichtlich das falsche XML gebaut.",
                f.text
            ))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KOPF_TEST: &[u8] = b"<Elster><TransferHeader><Testmerker>700000004</Testmerker></TransferHeader></Elster>";
    const KOPF_ECHT: &[u8] = b"<Elster><TransferHeader></TransferHeader></Elster>";

    fn echt() -> Modus {
        Modus::Echtversand {
            freigabe: crate::ECHTVERSAND_FREIGABE.to_owned(),
        }
    }

    #[test]
    fn testversand_nimmt_genau_einen_merker_im_kopf() {
        assert_eq!(pruefe_merker_konsistenz(KOPF_TEST, &Modus::Testversand), Ok(()));
    }

    #[test]
    fn testversand_verwirft_falsche_formen() {
        for xml in [
            &b"<Elster><TransferHeader><Testmerker>700000001</Testmerker></TransferHeader></Elster>"[..],
            KOPF_ECHT,
            b"<Elster><TransferHeader><Testmerker> 700000004</Testmerker></TransferHeader></Elster>",
            b"<Elster><TransferHeader><Testmerker/></TransferHeader></Elster>",
            b"<Elster><TransferHeader><!-- <Testmerker>700000004</Testmerker> --></TransferHeader></Elster>",
            b"<Elster><Nutzdaten><Testmerker>700000004</Testmerker></Nutzdaten></Elster>",
            b"<Elster><TransferHeader><Testmerker>700000004</Testmerker><Testmerker>700000004</Testmerker></TransferHeader></Elster>",
        ] {
            assert!(
                matches!(
                    pruefe_merker_konsistenz(xml, &Modus::Testversand),
                    Err(VersandFehler::XmlMerkerMismatch(_))
                ),
                "{}",
                String::from_utf8_lossy(xml)
            );
        }
    }

    #[test]
    fn echtversand_verlangt_ein_xml_ohne_merker_element() {
        assert_eq!(pruefe_merker_konsistenz(KOPF_ECHT, &echt()), Ok(()));
        for xml in [
            KOPF_TEST,
            &b"<Elster><TransferHeader><Testmerker/></TransferHeader></Elster>"[..],
            b"<Elster><Nutzdaten><Testmerker>x</Testmerker></Nutzdaten></Elster>",
        ] {
            assert!(
                matches!(
                    pruefe_merker_konsistenz(xml, &echt()),
                    Err(VersandFehler::XmlMerkerMismatch(_))
                ),
                "{}",
                String::from_utf8_lossy(xml)
            );
        }
    }

    #[test]
    fn unlesbares_xml_ist_ein_abbruch_in_beiden_modi() {
        for xml in [&b"<Elster><TransferHeader>"[..], b"kein xml", &[0xff, 0xfe, 0x00]] {
            assert_eq!(
                pruefe_merker_konsistenz(xml, &Modus::Testversand),
                Err(VersandFehler::XmlUnlesbar)
            );
            assert_eq!(pruefe_merker_konsistenz(xml, &echt()), Err(VersandFehler::XmlUnlesbar));
        }
    }

    #[test]
    fn merker_im_xml_nennt_den_ersten_text() {
        assert_eq!(merker_im_xml(KOPF_TEST).as_deref(), Some("700000004"));
        assert_eq!(merker_im_xml(KOPF_ECHT), None);
        assert_eq!(merker_im_xml(b"kein xml"), None);
    }
}
