//! Sperrgrund: die abschliessende Menge der Gruende, aus denen `ergebnis()` KEINE
//! Zahl liefert (`produkt/bescheid/bescheid_deklaration.py: SPERRGRUND_KLARTEXT`,
//! `produkt/haut/api.py: ergebnis()`). Der Klartext jeder Variante ist byte-identisch
//! aus der Python-Quelle uebernommen (`tools/parity/dump_sperrgruende.py` ->
//! `rust/fixtures/sperrgrund_klartext.json`); ein Rust-Test vergleicht beide (unten).
//!
//! `grund` in `api.py: ergebnis()` ist entweder `None` (kein Sperrgrund), das Literal
//! `"bestaetigt"` (Erfolg, kein Klartext-Lookup) oder einer der 56 Schluessel unten. Die
//! dritte Moeglichkeit bildet [`Sperrgrund::Bestaetigt`] ab; sie hat bewusst KEINEN
//! eigenen Klartext (die Python-Quelle hat auch keinen fuer sie).
use std::fmt;
use std::str::FromStr;

/// Text fuer einen unbekannten/nicht gelisteten Sperrgrund-String (`UNBEKANNTER_SPERRGRUND`).
pub const UNBEKANNTER_SPERRGRUND: &str = "Die Berechnung kann an dieser Stelle nicht fortgesetzt werden, und woran genau es liegt, lässt sich hier nicht in Worte fassen. Das liegt an der Software, nicht an deinen Angaben. Bitte melde diesen Fall — damit lässt sich nachvollziehen, was gefehlt hat.";

/// Ein Sperrgrund-String, der zu keinem der 56 bekannten Schluessel und nicht zu
/// `"bestaetigt"` passt.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unbekannter Sperrgrund {0:?}")]
pub struct UnbekannterSperrgrund(pub String);

/// Alle Gruende, aus denen `ergebnis()` keine Zahl liefert, plus der Erfolgsfall
/// `"bestaetigt"` (`api.py: ergebnis()`, `grund in (None, "bestaetigt")`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sperrgrund {
    /// `"bestaetigt"`: kein Sperrgrund, das Ergebnis steht. Kein Klartext (Python hat
    /// auch keinen).
    Bestaetigt,
    Abs3Ueber5mioOffen,
    AlleinerziehendKonsistenzOffen,
    ArbeitsmittelAfaUeberGwgOffen,
    AuslandDhfNichtRingFaehig,
    BehinderungsbedingteAufwendungenWahlrechtOffen,
    BehinderungsbedingteAufwendungenWahlrechtPartnerOffen,
    BerufsunfaehigkeitOffen,
    DbaKapitalOffen,
    DbaMultiCountryOffen,
    DhfTatbestandOffen,
    EinkunftsartNichtRingFaehig,
    EngineUnavailable,
    FahrtkostenpauschaleOffen,
    FlagKonsistenzOffen,
    GewinnAngabenOffen,
    GewinnQuelleOffen,
    GewstHebesatzOffen,
    GwgAbschreibungOffen,
    GwgMehrwertsteuerOffen,
    GwgTatbestandOffen,
    HandwerkerFoerderungOffen,
    HaushaltEuEwrOffen,
    InputKegelNichtBestaetigt,
    KapitalSemantikOffen,
    KeinScheibenGesamtbescheid,
    KinderGehoerenInGesamt,
    KinderbetreuungReineBetreuungOffen,
    KinderbetreuungZahlungOffen,
    KirchensteuerBetragOffen,
    LohnersatzBetragOffen,
    LufEuerOffen,
    P164GateOffen,
    P32bKombiOffen,
    P35cDoppelfoerderungOffen,
    PartnerKegelOffen,
    PartnerKonsistenzOffen,
    PartnerVorOffen,
    ProgressionGehoertInGesamt,
    RealsplittingAngabenOffen,
    RechnungUnbarOffen,
    RenteInstanzOffen,
    RentenbeginnJahrUngueltig,
    RentenbeginnNachVz,
    RentenbeginnOffen,
    RentenfreibetragFixierungOffen,
    RingBetragVorlaeufig,
    UebernachtungTatbestandOffen,
    UebernachtungZeitraumOffen,
    UnterhaltBetragOffen,
    VerlustvortragBetragOffen,
    VerlustvortragGehoertInGesamt,
    VerpflegungDreimonatsfristAufteilungOffen,
    VerpflegungDreimonatsfristUnterbrechungOffen,
    VerpflegungReduktionOffen,
    VersorgungsfreibetragOffen,
    VvInstanzOffen,
}

impl Sperrgrund {
    /// Der Wire-String, wie er in `grund` steht (`api.py`, `bescheid_deklaration.py`).
    #[must_use]
    pub const fn als_str(self) -> &'static str {
        match self {
            Self::Bestaetigt => "bestaetigt",
            Self::Abs3Ueber5mioOffen => "abs3_ueber_5mio_offen",
            Self::AlleinerziehendKonsistenzOffen => "alleinerziehend_konsistenz_offen",
            Self::ArbeitsmittelAfaUeberGwgOffen => "arbeitsmittel_afa_ueber_gwg_offen",
            Self::AuslandDhfNichtRingFaehig => "ausland_dhf_nicht_ring_faehig",
            Self::BehinderungsbedingteAufwendungenWahlrechtOffen => {
                "behinderungsbedingte_aufwendungen_wahlrecht_offen"
            }
            Self::BehinderungsbedingteAufwendungenWahlrechtPartnerOffen => {
                "behinderungsbedingte_aufwendungen_wahlrecht_partner_offen"
            }
            Self::BerufsunfaehigkeitOffen => "berufsunfaehigkeit_offen",
            Self::DbaKapitalOffen => "dba_kapital_offen",
            Self::DbaMultiCountryOffen => "dba_multi_country_offen",
            Self::DhfTatbestandOffen => "dhf_tatbestand_offen",
            Self::EinkunftsartNichtRingFaehig => "einkunftsart_nicht_ring_faehig",
            Self::EngineUnavailable => "engine_unavailable",
            Self::FahrtkostenpauschaleOffen => "fahrtkostenpauschale_offen",
            Self::FlagKonsistenzOffen => "flag_konsistenz_offen",
            Self::GewinnAngabenOffen => "gewinn_angaben_offen",
            Self::GewinnQuelleOffen => "gewinn_quelle_offen",
            Self::GewstHebesatzOffen => "gewst_hebesatz_offen",
            Self::GwgAbschreibungOffen => "gwg_abschreibung_offen",
            Self::GwgMehrwertsteuerOffen => "gwg_mehrwertsteuer_offen",
            Self::GwgTatbestandOffen => "gwg_tatbestand_offen",
            Self::HandwerkerFoerderungOffen => "handwerker_foerderung_offen",
            Self::HaushaltEuEwrOffen => "haushalt_eu_ewr_offen",
            Self::InputKegelNichtBestaetigt => "input_kegel_nicht_bestaetigt",
            Self::KapitalSemantikOffen => "kapital_semantik_offen",
            Self::KeinScheibenGesamtbescheid => "kein_scheiben_gesamtbescheid",
            Self::KinderGehoerenInGesamt => "kinder_gehoeren_in_gesamt",
            Self::KinderbetreuungReineBetreuungOffen => "kinderbetreuung_reine_betreuung_offen",
            Self::KinderbetreuungZahlungOffen => "kinderbetreuung_zahlung_offen",
            Self::KirchensteuerBetragOffen => "kirchensteuer_betrag_offen",
            Self::LohnersatzBetragOffen => "lohnersatz_betrag_offen",
            Self::LufEuerOffen => "luf_euer_offen",
            Self::P164GateOffen => "p16_4_gate_offen",
            Self::P32bKombiOffen => "p32b_kombi_offen",
            Self::P35cDoppelfoerderungOffen => "p35c_doppelfoerderung_offen",
            Self::PartnerKegelOffen => "partner_kegel_offen",
            Self::PartnerKonsistenzOffen => "partner_konsistenz_offen",
            Self::PartnerVorOffen => "partner_vor_offen",
            Self::ProgressionGehoertInGesamt => "progression_gehoert_in_gesamt",
            Self::RealsplittingAngabenOffen => "realsplitting_angaben_offen",
            Self::RechnungUnbarOffen => "rechnung_unbar_offen",
            Self::RenteInstanzOffen => "rente_instanz_offen",
            Self::RentenbeginnJahrUngueltig => "rentenbeginn_jahr_ungueltig",
            Self::RentenbeginnNachVz => "rentenbeginn_nach_vz",
            Self::RentenbeginnOffen => "rentenbeginn_offen",
            Self::RentenfreibetragFixierungOffen => "rentenfreibetrag_fixierung_offen",
            Self::RingBetragVorlaeufig => "ring_betrag_vorlaeufig",
            Self::UebernachtungTatbestandOffen => "uebernachtung_tatbestand_offen",
            Self::UebernachtungZeitraumOffen => "uebernachtung_zeitraum_offen",
            Self::UnterhaltBetragOffen => "unterhalt_betrag_offen",
            Self::VerlustvortragBetragOffen => "verlustvortrag_betrag_offen",
            Self::VerlustvortragGehoertInGesamt => "verlustvortrag_gehoert_in_gesamt",
            Self::VerpflegungDreimonatsfristAufteilungOffen => {
                "verpflegung_dreimonatsfrist_aufteilung_offen"
            }
            Self::VerpflegungDreimonatsfristUnterbrechungOffen => {
                "verpflegung_dreimonatsfrist_unterbrechung_offen"
            }
            Self::VerpflegungReduktionOffen => "verpflegung_reduktion_offen",
            Self::VersorgungsfreibetragOffen => "versorgungsfreibetrag_offen",
            Self::VvInstanzOffen => "vv_instanz_offen",
        }
    }

    /// Die dem Nutzer angezeigte Erklaerung (`SPERRGRUND_KLARTEXT`/`sperrgrund_klartext()`).
    /// `Bestaetigt` hat keinen Klartext -- dafuer zeigt die Haut ein Ergebnis, keinen Text.
    #[must_use]
    pub const fn klartext(self) -> Option<&'static str> {
        match self {
            Self::Bestaetigt => None,
            Self::Abs3Ueber5mioOffen => Some("Du hast den ermäßigten Steuersatz für den Verkauf oder die Aufgabe deines Betriebs beantragt, und der Gewinn liegt über fünf Millionen Euro. Der ermäßigte Satz gilt nur bis zu dieser Grenze; wie der Teil darüber zu versteuern ist, rechnet die Software noch nicht. Dieser Fall braucht steuerliche Beratung."),
            Self::AlleinerziehendKonsistenzOffen => Some("Zwei Angaben passen nicht zusammen: Du hast angegeben, allein stehend zu sein, und zugleich eine gemeinsame Veranlagung mit Ehe- oder Lebenspartner gewählt. Den Entlastungsbetrag für Alleinerziehende gibt es nur, wenn du nicht gemeinsam veranlagt wirst. Bitte sieh dir beide Angaben noch einmal an."),
            Self::ArbeitsmittelAfaUeberGwgOffen => Some("Zu deinen angeschafften Arbeitsmitteln fehlt noch, wie die Kosten abgesetzt werden sollen. Bei Anschaffungen bis 800 Euro ist das die Frage, ob du den Betrag sofort in voller Höhe absetzen willst; bei teureren Geräten die Nutzungsdauer und — wenn du sie in diesem Jahr gekauft hast — der Anschaffungsmonat. Bitte beantworte die Rückfragen zu deinen Arbeitsmitteln."),
            Self::AuslandDhfNichtRingFaehig => Some("Deine zweite Wohnung am Arbeitsort liegt im Ausland. Dafür gelten eigene Obergrenzen, die die Software noch nicht rechnet. Dieser Fall braucht steuerliche Beratung."),
            Self::BehinderungsbedingteAufwendungenWahlrechtOffen => Some("Du hast eine Behinderung angegeben und zusätzlich Kosten, die dadurch entstanden sind. Hier hast du die Wahl: entweder der Pauschbetrag ohne Nachweis oder deine tatsächlichen Kosten mit Belegen. Welcher Weg günstiger ist, hängt an der Höhe deiner Kosten — deshalb kann die Software das nicht für dich entscheiden. Bitte beantworte die Frage nach dem Pauschbetrag."),
            Self::BehinderungsbedingteAufwendungenWahlrechtPartnerOffen => Some("Für deinen Ehe- oder Lebenspartner ist eine Behinderung angegeben und zusätzlich Kosten, die dadurch entstanden sind. Auch hier gibt es die Wahl zwischen dem Pauschbetrag ohne Nachweis und den tatsächlichen Kosten mit Belegen. Welcher Weg günstiger ist, hängt an der Höhe der Kosten — deshalb kann die Software das nicht entscheiden. Bitte beantworte die Frage nach dem Pauschbetrag für deinen Partner."),
            Self::BerufsunfaehigkeitOffen => Some("Du hast den ermäßigten Steuersatz für den Verkauf oder die Aufgabe deines Betriebs beantragt. Vor dem 55. Geburtstag steht er dir nur zu, wenn du dauernd berufsunfähig bist. Bitte beantworte diese Frage, auch wenn die Antwort „nein“ ist."),
            Self::DbaKapitalOffen => Some("Du hast Kapitalerträge angegeben und zugleich ausländische Einkünfte. Ob und wie eine im Ausland gezahlte Steuer auf deine Kapitalerträge angerechnet wird, rechnet die Software noch nicht. Dieser Fall braucht steuerliche Beratung."),
            Self::DbaMultiCountryOffen => Some("Du hast Einkünfte aus mehr als einem ausländischen Staat. Jedes Land hat ein eigenes Abkommen mit Deutschland darüber, wo besteuert wird; mehrere Länder zugleich rechnet die Software noch nicht. Dieser Fall braucht steuerliche Beratung."),
            Self::DhfTatbestandOffen => Some("Du hast Kosten für eine zweite Wohnung am Arbeitsort angegeben. Ob sie absetzbar sind, hängt an drei Voraussetzungen: dass die zweite Wohnung beruflich veranlasst ist, dass du an deinem Hauptwohnsitz einen eigenen Hausstand führst und dass du dich dort finanziell an den Kosten beteiligst. Bitte beantworte diese drei Fragen."),
            Self::EinkunftsartNichtRingFaehig => Some("Du hast angegeben, dass du eine Einkunftsart hast, die in dieser Berechnung noch nicht mitgerechnet werden kann — je nach Fall Renten und andere sonstige Einkünfte, Einnahmen aus Vermietung, Kapitalerträge, Gewinn aus einem Betrieb oder der Verkauf eines Grundstücks oder eines anderen Vermögensgegenstands. Ein Ergebnis ohne diese Einkünfte wäre zu niedrig, deshalb rechnet die Software hier nicht weiter."),
            Self::EngineUnavailable => Some("Alle nötigen Angaben liegen vor, aber der Rechenkern liefert für diesen Fall gerade kein Ergebnis. Das liegt an der Software, nicht an deinen Angaben. Bitte versuche es später noch einmal, und melde den Fall, wenn er bestehen bleibt."),
            Self::FahrtkostenpauschaleOffen => Some("Für die Fahrtkostenpauschale bei Behinderung fehlt noch eine Antwort: ob du eines der Merkzeichen aG, Bl, TBl oder H hast, oder ob dein Grad der Behinderung mindestens 80 beträgt (oder mindestens 70 mit Merkzeichen G). Bitte beantworte die Fragen, auch wenn die Antwort „nein“ ist."),
            Self::FlagKonsistenzOffen => Some("Zwei Angaben passen nicht zusammen: Bei einer Einkunftsart hast du angegeben, dass du sie nicht hast, und an anderer Stelle trotzdem einen Betrag dazu eingetragen. Es geht um eine der vier Fragen, ob du Gewinneinkünfte, Kapitalerträge, Einnahmen aus Vermietung oder sonstige Einkünfte wie Renten hast. Bitte sieh dir an, welche der beiden Angaben stimmt."),
            Self::GewinnAngabenOffen => Some("Du hast angegeben, dass du Gewinneinkünfte hast. Es fehlen noch Angaben zu deiner Einnahmen-Überschuss-Rechnung: Betriebseinnahmen, sonstige Betriebsausgaben oder Abschreibungen. Trag bei jeder dieser Angaben einen Betrag ein, auch wenn er 0 € ist. Oder gib deinen Gewinn direkt als Gesamtbetrag an."),
            Self::GewinnQuelleOffen => Some("Deinen Gewinn hast du auf zwei Wegen angegeben: einmal als fertigen Betrag und einmal aufgeteilt in Betriebseinnahmen, Betriebsausgaben und Abschreibungen. Welcher der beiden gilt, kann die Software nicht raten. Bitte lass einen der beiden Wege stehen."),
            Self::GewstHebesatzOffen => Some("Zu deinem Gewerbebetrieb fehlt der Hebesatz deiner Gemeinde, oder er steht auf 0 oder darunter. Ein Hebesatz von 0 oder darunter ist nicht möglich, jede Gemeinde muss einen Mindestsatz erheben. Ohne ihn lässt sich nicht berechnen, wie viel Gewerbesteuer auf deine Einkommensteuer angerechnet wird. Den Hebesatz findest du auf deinem Gewerbesteuerbescheid oder auf der Internetseite deiner Gemeinde."),
            Self::GwgAbschreibungOffen => Some("Ein Gerät, das du als Sofortabzug erfasst hast, kommt dafür nicht in Frage: Es kostet mehr als 800 Euro ohne Mehrwertsteuer, du kannst es nicht allein benutzen, oder du hast es ab 250 Euro weder in einer Liste noch in deiner Buchführung festgehalten. Dann verteilt sich der Abzug über mehrere Jahre (Abschreibung). Diese Abschreibung rechnet die Software hier noch nicht, und das Gerät still wegzulassen wäre falsch. Das Ergebnis bleibt deshalb offen. Trage das Gerät bitte nicht hier ein, sondern bei der Abschreibung."),
            Self::GwgMehrwertsteuerOffen => Some("Bei einem als Sofortabzug erfassten Gerät hast du angegeben, dass der Preis die Mehrwertsteuer enthält. Was du dann absetzen darfst, hängt davon ab, ob du die Mehrwertsteuer vom Finanzamt zurückbekommst: als Kleinunternehmer zählt der Preis mit Mehrwertsteuer, sonst der Preis ohne. Diese Unterscheidung kann die Software noch nicht treffen. Einen Abzug von null Euro will sie dir nicht zeigen, deshalb bleibt das Ergebnis offen. Bekommst du die Mehrwertsteuer zurück, gib den Preis ohne sie an und beantworte die Frage nach dem Preis ohne Mehrwertsteuer mit Ja."),
            Self::GwgTatbestandOffen => Some("Zu einem als Sofortabzug erfassten Gerät fehlt noch eine Antwort zu einer der Voraussetzungen — ob es allein benutzbar ist, ob der Betrag den Vorsteuerabzug schon abgezogen hat, oder (ab 250 Euro) ob du dazu eine Liste geführt hast oder es aus deiner Buchführung ersichtlich ist. Bitte beantworte die offene Frage zu diesem Gerät."),
            Self::HandwerkerFoerderungOffen => Some("Zu deinen Handwerkerkosten fehlt noch die Antwort, ob du dafür öffentliche Fördermittel bekommen hast — etwa einen zinsverbilligten Kredit oder einen steuerfreien Zuschuss. Für geförderte Maßnahmen gibt es die Steuerermäßigung nicht. Bitte beantworte diese Frage, auch wenn du keine Förderung bekommen hast."),
            Self::HaushaltEuEwrOffen => Some("Für deine Kosten für Handwerker, Haushaltshilfe oder haushaltsnahe Dienstleistungen fehlt noch die Antwort, ob der Haushalt in der Europäischen Union oder im Europäischen Wirtschaftsraum liegt. Nur dann gibt es die Steuerermäßigung. Bitte beantworte diese Frage — bei einem Haushalt in Deutschland ist sie automatisch mit Ja beantwortet."),
            Self::InputKegelNichtBestaetigt => Some("Für ein Ergebnis fehlen noch Angaben. Welche das sind, ist hier aufgeführt — sobald sie beantwortet sind, geht es weiter. Es ist nichts schiefgegangen: du bist noch mitten in der Erklärung."),
            Self::KapitalSemantikOffen => Some("Deine Kapitalerträge hast du auf zwei Wegen angegeben: einmal als Gesamtsumme und einmal aufgeteilt in einzelne Gewinne und Verluste. Ob die Einzelbeträge in der Summe schon enthalten sind oder dazukommen, kann die Software nicht raten. Bitte lass einen der beiden Wege stehen."),
            Self::KeinScheibenGesamtbescheid => Some("Für diesen Ausschnitt deiner Erklärung gibt es bewusst keine Gesamtsumme. Die einzelnen Regeln werden hier gerechnet, aber eine belastbare Gesamtsteuer daraus zu bilden kann die Software an dieser Stelle noch nicht — und sie zeigt lieber keine Zahl als eine falsche."),
            Self::KinderGehoerenInGesamt => Some("Du hast Kinder angegeben. Ob Kindergeld oder die Kinderfreibeträge günstiger sind, wird gegeneinander abgewogen, und diese Abwägung ist in der gerade laufenden Berechnung nicht enthalten. Ohne sie wäre deine Steuer zu hoch, deshalb rechnet die Software hier nicht weiter."),
            Self::KinderbetreuungReineBetreuungOffen => Some("Zu deinen Betreuungskosten fehlt noch die Antwort, ob der Betrag reine Betreuung ist. Nachhilfe, Musik- oder Sportunterricht und Freizeitkurse sind keine Betreuung und werden nicht abgezogen. Hast du beides in einem Betrag gezahlt, trage bitte nur den Betreuungsteil ein und antworte dann mit Ja."),
            Self::KinderbetreuungZahlungOffen => Some("Zu deinen Betreuungskosten fehlt noch die Antwort, ob du eine Rechnung erhalten und per Überweisung bezahlt hast. Das Finanzamt erkennt nur Betreuungskosten an, die auf das Konto des Betreuers überwiesen wurden. Bar bezahlte Beträge zählen nicht. Bitte beantworte diese Frage oder trage nur den überwiesenen Teil ein."),
            Self::KirchensteuerBetragOffen => Some("Du bist Mitglied einer Kirche. Es fehlt noch, wie viel Kirchensteuer du im Jahr gezahlt hast oder erstattet bekommen hast. Die gezahlte Kirchensteuer steht auf deiner Lohnsteuerbescheinigung. Trag beide Beträge ein, auch wenn einer davon 0 € ist."),
            Self::LohnersatzBetragOffen => Some("Du hast angegeben, dass du Lohnersatzleistungen bekommen hast, etwa Elterngeld, Krankengeld oder Arbeitslosengeld. Es fehlt noch der Betrag. Er ist steuerfrei, erhöht aber den Steuersatz auf dein übriges Einkommen. Trag ihn ein, auch wenn er 0 € ist."),
            Self::LufEuerOffen => Some("Du hast einen land- oder forstwirtschaftlichen Betrieb angegeben und dazu Einnahmen und Ausgaben einzeln erfasst. Für die Land- und Forstwirtschaft gelten eigene Arten der Gewinnermittlung, die die Software noch nicht rechnet. Dieser Fall braucht steuerliche Beratung."),
            Self::P164GateOffen => Some("Es ist ein Gewinn aus dem Verkauf oder der Aufgabe eines Betriebs angegeben — bei dir oder bei deinem Partner. Dafür gibt es einen Freibetrag, aber nur unter zwei Bedingungen: Die betreffende Person ist mindestens 55 Jahre alt oder dauernd berufsunfähig, und sie hat diesen Freibetrag noch nie in Anspruch genommen. Bitte beantworte beide Fragen."),
            Self::P32bKombiOffen => Some("Du hast Lohnersatzleistungen wie Eltern-, Kranken- oder Arbeitslosengeld angegeben und zusätzlich einen Betriebsverkauf, Gewerbesteuer oder ausländische Einkünfte. Diese Kombination rechnet die Software noch nicht: Lohnersatzleistungen erhöhen den Steuersatz, und wie sich das mit den anderen Ermäßigungen verzahnt, ist offen. Dieser Fall braucht steuerliche Beratung."),
            Self::P35cDoppelfoerderungOffen => Some("Zu deiner energetischen Sanierung fehlt noch die Antwort, ob du dafür schon anderweitig gefördert wurdest — etwa durch öffentliche Zuschüsse oder weil du dieselben Kosten bereits als Handwerkerleistung geltend machst. In diesen Fällen entfällt die Steuerermäßigung ganz. Bitte beantworte diese Frage, auch wenn keine andere Förderung vorliegt."),
            Self::PartnerKegelOffen => Some("Zu deinem Ehe- oder Lebenspartner fehlen noch Angaben, die die gemeinsame Berechnung braucht — je nach Fall der Bruttoarbeitslohn, die Kapitalerträge oder die Art der Krankenversicherung. Ein Ergebnis für nur eine der beiden Personen wäre falsch. Bitte ergänze die offenen Angaben zu deinem Partner."),
            Self::PartnerKonsistenzOffen => Some("Zwei Angaben passen nicht zusammen: Du hast etwas zu deinem Ehe- oder Lebenspartner eingetragen — etwa dessen Behinderung, Kapitalerträge oder Rente — aber keine gemeinsame Veranlagung gewählt. Angaben zum Partner zählen nur in einer gemeinsamen Erklärung. Bitte sieh dir beide Angaben noch einmal an."),
            Self::PartnerVorOffen => Some("Du hast eine gemeinsame Veranlagung gewählt und Beiträge zur Rentenversicherung angegeben. Die Altersvorsorgebeiträge beider Partner rechnet die Software in dieser Zusammenstellung noch nicht. Dieser Fall wird derzeit nicht berechnet."),
            Self::ProgressionGehoertInGesamt => Some("Du hast Lohnersatzleistungen wie Eltern-, Kranken- oder Arbeitslosengeld angegeben. Diese Leistungen sind steuerfrei, erhöhen aber den Steuersatz auf dein übriges Einkommen. Dieser Effekt ist in der gerade laufenden Berechnung nicht enthalten, deshalb rechnet die Software hier nicht weiter."),
            Self::RealsplittingAngabenOffen => Some("Du zahlst Unterhalt an deinen geschiedenen oder getrennt lebenden Ehepartner. Es fehlt noch der Betrag oder die Antwort, ob der Empfänger dem Abzug zugestimmt hat (Anlage U). Ohne Zustimmung gibt es den Abzug nicht. Bitte beantworte beide Fragen."),
            Self::RechnungUnbarOffen => Some("Zu deinen Handwerker- oder Haushaltsdienstleistungen fehlt noch die Antwort, ob du eine Rechnung erhalten und sie überwiesen hast. Barzahlungen erkennt das Finanzamt hier nicht an. Bitte beantworte diese Frage."),
            Self::RenteInstanzOffen => Some("Zu einer deiner Renten oder zu einer Rente deines Partners sind die Angaben unvollständig. Für jede einzelne Rente braucht die Berechnung vier Dinge: die Art der Rente, den Jahresbetrag, das Jahr des Rentenbeginns und das Alter der beziehenden Person zu diesem Zeitpunkt. Bitte ergänze die fehlenden Angaben."),
            Self::RentenbeginnJahrUngueltig => Some("Das Jahr des Rentenbeginns ist keine gültige Jahreszahl. Eine Rente kann nicht im Jahr 0 oder davor begonnen haben. Bitte trage das Jahr ein, in dem deine Rente zum ersten Mal gezahlt wurde."),
            Self::RentenbeginnNachVz => Some("Das Jahr des Rentenbeginns liegt nach dem Jahr dieser Steuererklärung. Eine Rente, die erst später beginnt, gehört nicht in diese Erklärung. Bitte prüfe das Jahr des Rentenbeginns."),
            Self::RentenbeginnOffen => Some("Zu deiner Rente fehlt das Jahr, in dem die Rentenzahlung begonnen hat. Die Berechnung braucht dieses Jahr, um den steuerfreien Teil der Rente richtig festzulegen. Bitte trage das Jahr des Rentenbeginns ein."),
            Self::RentenfreibetragFixierungOffen => Some("Die Rente hat vor diesem Jahr begonnen. Dann ist der steuerfreie Teil der Rente ein fester Eurobetrag, der im Jahr nach dem Rentenbeginn einmal festgelegt wurde und sich seither nicht mehr ändert. Diesen Betrag findest du in einem früheren Steuerbescheid. Bitte trage ihn ein."),
            Self::RingBetragVorlaeufig => Some("Ein Betrag, den du genannt hast, ist noch nicht bestätigt. Solange das so ist, zeigt die Software keine Steuer an; sie könnte einen genannten Betrag sonst nicht mitrechnen. Bitte sieh dir die Angabe noch einmal an und bestätige sie; danach rechnet die Software die Zahl."),
            Self::UebernachtungTatbestandOffen => Some("Du hast Übernachtungskosten auf Auswärtstätigkeit angegeben. Ob sie absetzbar sind, hängt an mehreren Fragen: ob die Unterkunft im Inland liegt, ob die Übernachtung wirklich auswärts stattfand, ob du die Unterkunft allein genutzt hast und ob die Tätigkeit ohne lange Unterbrechung lief. Bitte beantworte diese Fragen."),
            Self::UebernachtungZeitraumOffen => Some("Du hast Übernachtungskosten angegeben, aber die Angabe fehlt, in wie vielen Monaten dieses Jahres du auswärts übernachtet hast und seit wie vielen Monaten du schon an diesem Ort arbeitest. Davon hängt ab, ob deine Kosten nach 48 Monaten noch begrenzt sind. Bitte beantworte diese Fragen."),
            Self::UnterhaltBetragOffen => Some("Du hast angegeben, dass du eine unterhaltsberechtigte Person unterstützt hast. Es fehlt noch, wie viel du dafür im Jahr ausgegeben hast. Trag den Betrag ein, auch wenn er 0 € ist."),
            Self::VerlustvortragBetragOffen => Some("Du hast angegeben, dass für dich ein Verlustvortrag festgestellt wurde. Es fehlt noch der Betrag. Du findest ihn im letzten Bescheid über die gesonderte Feststellung des verbleibenden Verlustvortrags. Trag ihn ein, auch wenn er 0 € ist."),
            Self::VerlustvortragGehoertInGesamt => Some("Du hast einen Verlustvortrag aus einem früheren Jahr angegeben. Seine Verrechnung mit dem Einkommen dieses Jahres ist in der gerade laufenden Berechnung nicht enthalten. Ohne sie wäre deine Steuer zu hoch, deshalb rechnet die Software hier nicht weiter."),
            Self::VerpflegungDreimonatsfristAufteilungOffen => Some("Du warst länger als drei Monate am selben auswärtigen Ort tätig. Die Verpflegungspauschale gibt es nur für die ersten drei Monate, danach entfällt sie. Deshalb braucht die Berechnung zu jeder Art von Abwesenheitstag zusätzlich die Zahl der Tage, die nach diesen drei Monaten lagen. Bitte ergänze diese Angabe."),
            Self::VerpflegungDreimonatsfristUnterbrechungOffen => Some("Du warst länger als drei Monate am selben auswärtigen Ort tätig, hast aber für die Zeit nach Ablauf der drei Monate keine Abwesenheitstage angegeben. Das ist möglich, wenn du die Tätigkeit dort mindestens vier Wochen unterbrochen hast — dann beginnt die Frist neu. Bitte beantworte die Frage, ob es eine solche Unterbrechung gab."),
            Self::VerpflegungReduktionOffen => Some("Zu deinen Auswärtstätigkeiten fehlt noch die Antwort, ob dir dabei Mahlzeiten gestellt wurden — also Frühstück, Mittag- oder Abendessen von deinem Arbeitgeber oder auf dessen Veranlassung. Jede gestellte Mahlzeit kürzt die Verpflegungspauschale. Bitte beantworte diese Frage, auch wenn keine Mahlzeiten gestellt wurden."),
            Self::VersorgungsfreibetragOffen => Some("Du hast Versorgungsbezüge angegeben — etwa eine Betriebsrente oder eine Beamtenpension. Für den Freibetrag darauf braucht die Berechnung zwei Angaben: das Jahr, in dem die Versorgung begann, und den Betrag, aus dem der Freibetrag berechnet wird. Beides findest du in deiner Lohnsteuerbescheinigung oder in der Mitteilung deiner Versorgungsstelle."),
            Self::VvInstanzOffen => Some("Zu einer deiner vermieteten Immobilien sind die Angaben unvollständig. Jedes weitere Objekt braucht dieselben Angaben wie das erste: Mieteinnahmen, Gebäudeabschreibung, Schuldzinsen, Erhaltungsaufwand, sonstige Werbungskosten und den Anteil, der entgeltlich vermietet ist. Bitte ergänze die fehlenden Angaben."),
        }
    }
}

impl fmt::Display for Sperrgrund {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.als_str())
    }
}

impl FromStr for Sperrgrund {
    type Err = UnbekannterSperrgrund;

    /// ```
    /// use domain::Sperrgrund;
    /// assert_eq!("bestaetigt".parse::<Sperrgrund>().unwrap(), Sperrgrund::Bestaetigt);
    /// assert!(Sperrgrund::Bestaetigt.klartext().is_none());
    /// assert!("kirchensteuer_betrag_offen".parse::<Sperrgrund>().unwrap().klartext().is_some());
    /// assert!("unbekannt_xyz".parse::<Sperrgrund>().is_err());
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bestaetigt" => Ok(Self::Bestaetigt),
            "abs3_ueber_5mio_offen" => Ok(Self::Abs3Ueber5mioOffen),
            "alleinerziehend_konsistenz_offen" => Ok(Self::AlleinerziehendKonsistenzOffen),
            "arbeitsmittel_afa_ueber_gwg_offen" => Ok(Self::ArbeitsmittelAfaUeberGwgOffen),
            "ausland_dhf_nicht_ring_faehig" => Ok(Self::AuslandDhfNichtRingFaehig),
            "behinderungsbedingte_aufwendungen_wahlrecht_offen" => {
                Ok(Self::BehinderungsbedingteAufwendungenWahlrechtOffen)
            }
            "behinderungsbedingte_aufwendungen_wahlrecht_partner_offen" => {
                Ok(Self::BehinderungsbedingteAufwendungenWahlrechtPartnerOffen)
            }
            "berufsunfaehigkeit_offen" => Ok(Self::BerufsunfaehigkeitOffen),
            "dba_kapital_offen" => Ok(Self::DbaKapitalOffen),
            "dba_multi_country_offen" => Ok(Self::DbaMultiCountryOffen),
            "dhf_tatbestand_offen" => Ok(Self::DhfTatbestandOffen),
            "einkunftsart_nicht_ring_faehig" => Ok(Self::EinkunftsartNichtRingFaehig),
            "engine_unavailable" => Ok(Self::EngineUnavailable),
            "fahrtkostenpauschale_offen" => Ok(Self::FahrtkostenpauschaleOffen),
            "flag_konsistenz_offen" => Ok(Self::FlagKonsistenzOffen),
            "gewinn_angaben_offen" => Ok(Self::GewinnAngabenOffen),
            "gewinn_quelle_offen" => Ok(Self::GewinnQuelleOffen),
            "gewst_hebesatz_offen" => Ok(Self::GewstHebesatzOffen),
            "gwg_abschreibung_offen" => Ok(Self::GwgAbschreibungOffen),
            "gwg_mehrwertsteuer_offen" => Ok(Self::GwgMehrwertsteuerOffen),
            "gwg_tatbestand_offen" => Ok(Self::GwgTatbestandOffen),
            "handwerker_foerderung_offen" => Ok(Self::HandwerkerFoerderungOffen),
            "haushalt_eu_ewr_offen" => Ok(Self::HaushaltEuEwrOffen),
            "input_kegel_nicht_bestaetigt" => Ok(Self::InputKegelNichtBestaetigt),
            "kapital_semantik_offen" => Ok(Self::KapitalSemantikOffen),
            "kein_scheiben_gesamtbescheid" => Ok(Self::KeinScheibenGesamtbescheid),
            "kinder_gehoeren_in_gesamt" => Ok(Self::KinderGehoerenInGesamt),
            "kinderbetreuung_reine_betreuung_offen" => Ok(Self::KinderbetreuungReineBetreuungOffen),
            "kinderbetreuung_zahlung_offen" => Ok(Self::KinderbetreuungZahlungOffen),
            "kirchensteuer_betrag_offen" => Ok(Self::KirchensteuerBetragOffen),
            "lohnersatz_betrag_offen" => Ok(Self::LohnersatzBetragOffen),
            "luf_euer_offen" => Ok(Self::LufEuerOffen),
            "p16_4_gate_offen" => Ok(Self::P164GateOffen),
            "p32b_kombi_offen" => Ok(Self::P32bKombiOffen),
            "p35c_doppelfoerderung_offen" => Ok(Self::P35cDoppelfoerderungOffen),
            "partner_kegel_offen" => Ok(Self::PartnerKegelOffen),
            "partner_konsistenz_offen" => Ok(Self::PartnerKonsistenzOffen),
            "partner_vor_offen" => Ok(Self::PartnerVorOffen),
            "progression_gehoert_in_gesamt" => Ok(Self::ProgressionGehoertInGesamt),
            "realsplitting_angaben_offen" => Ok(Self::RealsplittingAngabenOffen),
            "rechnung_unbar_offen" => Ok(Self::RechnungUnbarOffen),
            "rente_instanz_offen" => Ok(Self::RenteInstanzOffen),
            "rentenbeginn_jahr_ungueltig" => Ok(Self::RentenbeginnJahrUngueltig),
            "rentenbeginn_nach_vz" => Ok(Self::RentenbeginnNachVz),
            "rentenbeginn_offen" => Ok(Self::RentenbeginnOffen),
            "rentenfreibetrag_fixierung_offen" => Ok(Self::RentenfreibetragFixierungOffen),
            "ring_betrag_vorlaeufig" => Ok(Self::RingBetragVorlaeufig),
            "uebernachtung_tatbestand_offen" => Ok(Self::UebernachtungTatbestandOffen),
            "uebernachtung_zeitraum_offen" => Ok(Self::UebernachtungZeitraumOffen),
            "unterhalt_betrag_offen" => Ok(Self::UnterhaltBetragOffen),
            "verlustvortrag_betrag_offen" => Ok(Self::VerlustvortragBetragOffen),
            "verlustvortrag_gehoert_in_gesamt" => Ok(Self::VerlustvortragGehoertInGesamt),
            "verpflegung_dreimonatsfrist_aufteilung_offen" => {
                Ok(Self::VerpflegungDreimonatsfristAufteilungOffen)
            }
            "verpflegung_dreimonatsfrist_unterbrechung_offen" => {
                Ok(Self::VerpflegungDreimonatsfristUnterbrechungOffen)
            }
            "verpflegung_reduktion_offen" => Ok(Self::VerpflegungReduktionOffen),
            "versorgungsfreibetrag_offen" => Ok(Self::VersorgungsfreibetragOffen),
            "vv_instanz_offen" => Ok(Self::VvInstanzOffen),
            other => Err(UnbekannterSperrgrund(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Sperrgrund, UNBEKANNTER_SPERRGRUND};

    /// Parity-Fixture, per Build-Script-losem `include_str!` eingebettet -- direkt aus
    /// `tools/parity/dump_sperrgruende.py` erzeugt, kein manuell abgetippter Text.
    const FIXTURE: &str = include_str!("../../fixtures/sperrgrund_klartext.json");

    #[test]
    fn klartext_ist_byte_identisch_zur_python_quelle() {
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        let klartext = fixture["klartext"].as_object().unwrap();
        assert_eq!(
            klartext.len(),
            56,
            "Fixture-Groesse hat sich veraendert -- Enum nachziehen"
        );
        for (schluessel, erwartet) in klartext {
            let grund: Sperrgrund = schluessel.parse().unwrap();
            assert_eq!(
                grund.klartext().unwrap(),
                erwartet.as_str().unwrap(),
                "{schluessel}"
            );
        }
        assert_eq!(
            UNBEKANNTER_SPERRGRUND,
            fixture["unbekannt"].as_str().unwrap()
        );
    }

    #[test]
    fn bestaetigt_hat_keinen_klartext() {
        assert_eq!(Sperrgrund::Bestaetigt.als_str(), "bestaetigt");
        assert!(Sperrgrund::Bestaetigt.klartext().is_none());
    }
}
