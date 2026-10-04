//! Entscheidungsstellen der drei Prompts und der Regelzeilen (`api_llm.py`: `_aussagen_prompt`, `_regel_zeilen`,
//! `_themen_prompt`, `_dialog_prompt`, `_aussagen_block`), am Aufrufort von `llm::prompt` geprueft (N4,
//! Mutationsmessung `rust/llm`, Teil `prompt.rs`/`texte.rs`). Jede Erwartung stammt aus dem Python-Aufruf mit
//! denselben Eingaben; sie steht hier als Literal, kein Test ruft Python. Die festen Textteile (`texte.rs`) sind
//! lang; sie stehen als Kennung (Zeichenzahl und FNV-1a-64 des ganzen Systemtextes), die wechselnden Teile
//! zusaetzlich im Klartext.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use llm::gates::{felder_je_regel, KatalogFeld};
use llm::parse::{Aussage, AussageStatus};
use llm::pii::{filtere, Gefiltert};
use llm::prompt::{aussagen_prompt, dialog_prompt, regel_zeilen, themen_prompt};
use llm::{Nachricht, Rolle};

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// (Zeichenzahl, FNV-1a-64 ueber die UTF-8-Bytes): die Kennung eines langen Textes aus Python.
fn kennung(s: &str) -> (usize, u64) {
    (s.chars().count(), fnv(s))
}

fn gef(text: &str) -> Gefiltert {
    let (g, k) = filtere(text);
    assert_eq!((g.as_str(), k.len()), (text, 0), "Eingabe ohne PII bleibt");
    g
}

fn katalog(json: &str) -> Vec<KatalogFeld> {
    serde_json::from_str(json).unwrap()
}

fn aussagen(texte: &[&str]) -> Vec<Aussage> {
    texte
        .iter()
        .map(|t| Aussage {
            text: (*t).into(),
            beleg: String::new(),
            status: AussageStatus::Offen,
            regeln: Vec::new(),
        })
        .collect()
}

/// System- und Nutzernachricht eines Prompts: Systemtext als Kennung, Nutzertext im Klartext.
fn pruefe(name: &str, m: &[Nachricht], system: (usize, u64), user: &str) {
    assert_eq!(m.len(), 2, "{name}");
    assert_eq!(
        (m[0].rolle(), m[1].rolle()),
        (Rolle::System, Rolle::User),
        "{name}"
    );
    assert_eq!(
        kennung(m[0].inhalt()),
        system,
        "{name}: Systemtext\n{}",
        m[0].inhalt()
    );
    assert_eq!(m[1].inhalt(), user, "{name}: Nutzertext");
}

/// Stufe 1: fester Systemtext, der Freitext als Nutzernachricht.
#[test]
fn aussagen_prompt_wie_python() {
    let m = aussagen_prompt(&gef("Ich bin ledig und habe 50000 Euro verdient"));
    pruefe(
        "aussagen",
        &m,
        (994, 0xfa44_32b0_5350_f697),
        "Ich bin ledig und habe 50000 Euro verdient",
    );
}

const REGEL_KATALOG: &str = r#"[{"feld_id": "f1", "regel_id": "r_kurz", "fragetext_laie": "Wie hoch war der Lohn?"}, {"feld_id": "f2", "regel_id": "r_kurz", "fragetext_laie": "Wann hast du angefangen"}, {"feld_id": "f3", "regel_id": "r_kurz", "fragetext_laie": "Was zahlst du? "}, {"feld_id": "f4", "regel_id": "r_fuenf", "fragetext_laie": "Frage eins?"}, {"feld_id": "f5", "regel_id": "r_fuenf", "fragetext_laie": "Frage zwei?"}, {"feld_id": "f6", "regel_id": "r_fuenf", "fragetext_laie": "Frage drei?"}, {"feld_id": "f7", "regel_id": "r_fuenf", "fragetext_laie": "Frage vier?"}, {"feld_id": "f8", "regel_id": "r_fuenf", "fragetext_laie": "Frage fuenf?"}, {"feld_id": "f9", "regel_id": "r_vier", "fragetext_laie": "Eins"}, {"feld_id": "f10", "regel_id": "r_vier", "fragetext_laie": "Zwei"}, {"feld_id": "f11", "regel_id": "r_vier", "fragetext_laie": "Drei"}, {"feld_id": "f12", "regel_id": "r_vier", "fragetext_laie": "Vier"}, {"feld_id": "f13", "regel_id": "r_43", "fragetext_laie": "wort wort wort wort wort wort wort wort wor"}, {"feld_id": "f14", "regel_id": "r_44", "fragetext_laie": "wort wort wort wort wort wort wort wort wort"}, {"feld_id": "f15", "regel_id": "r_45", "fragetext_laie": "wort wort wort wort wort wort wort wort wortx"}, {"feld_id": "f16", "regel_id": "r_46", "fragetext_laie": "wort wort wort wort wort wort wort wort wort w"}, {"feld_id": "f17", "regel_id": "r_50", "fragetext_laie": "Wie viele Kilometer faehrst du taeglich zur Arbeitsstaette hin und zurueck"}, {"feld_id": "f18", "regel_id": "r_ohne_leer", "fragetext_laie": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}, {"feld_id": "f19", "regel_id": "r_umlaut", "fragetext_laie": "Überpruefung der Kirchensteuer fuer Änderungen im Monat"}, {"feld_id": "f20", "regel_id": "r_leer_text", "fragetext_laie": ""}, {"feld_id": "f21", "regel_id": "r_leer_text", "fragetext_laie": null}, {"feld_id": "f22", "regel_id": "r_leer_text", "fragetext_laie": "Mit Text?"}, {"feld_id": "f23", "regel_id": "r_nur_fragezeichen", "fragetext_laie": "?"}, {"feld_id": "f24", "regel_id": "r_nur_fragezeichen", "fragetext_laie": "  "}, {"feld_id": "f25", "regel_id": "r_nur_fragezeichen", "fragetext_laie": " ? ?"}, {"feld_id": "f26", "regel_id": "r_trenner", "fragetext_laie": "Wie alt?? ?"}, {"feld_id": "f27", "regel_id": "r_trenner", "fragetext_laie": "Wie alt ?"}, {"feld_id": "f28", "regel_id": "r_trenner", "fragetext_laie": "Antwort?  "}, {"feld_id": "f29", "regel_id": "r_ein_leerer", "fragetext_laie": ""}, {"feld_id": "f30", "regel_id": "r_ein_leerer", "fragetext_laie": "Zweite Frage"}, {"feld_id": "f31", "regel_id": "r_erste_leer", "fragetext_laie": "?"}, {"feld_id": "f32", "regel_id": "r_erste_leer", "fragetext_laie": "Zweite Frage"}, {"feld_id": "f33", "regel_id": "r_erste_leer", "fragetext_laie": "Dritte Frage"}, {"feld_id": "f34", "regel_id": "r_erste_leer", "fragetext_laie": "Vierte Frage"}]"#;

/// Regelzeilen: erste drei Feldfragen, `? ` am Ende weg, bei mehr als 44 Zeichen an der Wortgrenze gekuerzt, leere Fragen fallen weg.
#[test]
#[rustfmt::skip]
fn regel_zeilen_wie_python() {
    let kat = katalog(REGEL_KATALOG);
    let je = felder_je_regel(&kat);
    let faelle: &[(&[&str], &str)] = &[
        (&["r_kurz"], "- r_kurz: Wie hoch war der Lohn; Wann hast du angefangen; Was zahlst du"),
        (&["r_fuenf"], "- r_fuenf: Frage eins; Frage zwei; Frage drei"),
        (&["r_vier"], "- r_vier: Eins; Zwei; Drei"),
        (&["r_43"], "- r_43: wort wort wort wort wort wort wort wort wor"),
        (&["r_44"], "- r_44: wort wort wort wort wort wort wort wort wort"),
        (&["r_45"], "- r_45: wort wort wort wort wort wort wort wort…"),
        (&["r_46"], "- r_46: wort wort wort wort wort wort wort wort…"),
        (&["r_50"], "- r_50: Wie viele Kilometer faehrst du taeglich zur…"),
        (&["r_ohne_leer"], "- r_ohne_leer: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA…"),
        (&["r_umlaut"], "- r_umlaut: Überpruefung der Kirchensteuer fuer…"),
        (&["r_leer_text"], "- r_leer_text: f20; f21; Mit Text"),
        (&["r_nur_fragezeichen"], "- r_nur_fragezeichen"),
        (&["r_trenner"], "- r_trenner: Wie alt; Wie alt; Antwort"),
        (&["r_ein_leerer"], "- r_ein_leerer: f29; Zweite Frage"),
        (&["r_erste_leer"], "- r_erste_leer: Zweite Frage; Dritte Frage"),
        (&["r_kurz", "r_fuenf", "r_vier", "r_43"], "- r_kurz: Wie hoch war der Lohn; Wann hast du angefangen; Was zahlst du\n- r_fuenf: Frage eins; Frage zwei; Frage drei\n- r_vier: Eins; Zwei; Drei\n- r_43: wort wort wort wort wort wort wort wort wor"),
        (&["r_43", "r_44", "r_45"], "- r_43: wort wort wort wort wort wort wort wort wor\n- r_44: wort wort wort wort wort wort wort wort wort\n- r_45: wort wort wort wort wort wort wort wort…"),
        (&["r_kurz", "r_fuenf", "r_vier", "r_43", "r_44", "r_45", "r_46", "r_50", "r_ohne_leer", "r_umlaut", "r_leer_text", "r_nur_fragezeichen", "r_trenner", "r_ein_leerer", "r_erste_leer"], "- r_kurz: Wie hoch war der Lohn; Wann hast du angefangen; Was zahlst du\n- r_fuenf: Frage eins; Frage zwei; Frage drei\n- r_vier: Eins; Zwei; Drei\n- r_43: wort wort wort wort wort wort wort wort wor\n- r_44: wort wort wort wort wort wort wort wort wort\n- r_45: wort wort wort wort wort wort wort wort…\n- r_46: wort wort wort wort wort wort wort wort…\n- r_50: Wie viele Kilometer faehrst du taeglich zur…\n- r_ohne_leer: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA…\n- r_umlaut: Überpruefung der Kirchensteuer fuer…\n- r_leer_text: f20; f21; Mit Text\n- r_nur_fragezeichen\n- r_trenner: Wie alt; Wie alt; Antwort\n- r_ein_leerer: f29; Zweite Frage\n- r_erste_leer: Zweite Frage; Dritte Frage"),
    ];
    for (regeln, erwartet) in faelle {
        let regeln: Vec<String> = regeln.iter().map(|r| (*r).to_owned()).collect();
        assert_eq!(regel_zeilen(&je, &regeln), *erwartet, "regel_zeilen({regeln:?})");
    }
}

/// Stufe 2: Aussagen nummeriert (ab 0), ohne Aussagen "(keine)", danach die Regelzeilen.
#[test]
fn themen_prompt_wie_python() {
    let m = themen_prompt(
        &gef("Ich bin ledig und habe 50000 Euro verdient"),
        &aussagen(&[]),
        "- r1: Wie hoch war der Lohn",
    );
    pruefe(
        "ohne Aussagen",
        &m,
        (912, 0xa033_2cc5_ecf6_c786),
        "Ich bin ledig und habe 50000 Euro verdient",
    );
    assert!(
        m[0].inhalt()
            .contains("AUSSAGEN:\n(keine)\n\nREGELN:\n- r1: Wie hoch war der Lohn\n\nAntworte"),
        "ohne Aussagen"
    );
    let m = themen_prompt(
        &gef("Ich bin ledig und habe 50000 Euro verdient"),
        &aussagen(&["Der Nutzer ist ledig", "Der Nutzer hat 50000 Euro verdient"]),
        "- r1: a\n- r2: b",
    );
    pruefe(
        "zwei Aussagen",
        &m,
        (956, 0x8a7f_1d0d_25ba_9b6c),
        "Ich bin ledig und habe 50000 Euro verdient",
    );
    assert!(m[0].inhalt().contains("AUSSAGEN:\n[0] Der Nutzer ist ledig\n[1] Der Nutzer hat 50000 Euro verdient\n\nREGELN:\n- r1: a\n- r2: b\n\nAntworte"), "zwei Aussagen");
    let m = themen_prompt(&gef("Frage?"), &aussagen(&["Der Nutzer ist ledig"]), "- r");
    pruefe("eine Aussage", &m, (905, 0x07f9_07e6_1924_b0eb), "Frage?");
    assert!(
        m[0].inhalt()
            .contains("AUSSAGEN:\n[0] Der Nutzer ist ledig\n\nREGELN:\n- r\n\nAntworte"),
        "eine Aussage"
    );
}

/// Stufe 3: Feldzeilen (Typ, Bereich, Werte, Hilfe), Aussagenblock, Kontext; jeder Zweig einmal an und aus.
#[test]
#[rustfmt::skip]
fn dialog_prompt_wie_python() {
    let kat_json = r#"[{"feld_id": "bruttoarbeitslohn", "fragetext_laie": "Wie hoch war dein Bruttoarbeitslohn?", "hilfe_kurz": "Steht auf der Lohnsteuerbescheinigung Nr. 3", "typ": "geld", "bereich": {"min": 0, "max": 10000000}, "enum_werte": null}, {"feld_id": "kirchensteuer_art", "fragetext_laie": null, "hilfe_kurz": "", "typ": null, "bereich": {}, "enum_werte": ["ev", "rk", "keine"]}, {"feld_id": "anzahl_kinder", "fragetext_laie": "", "hilfe_kurz": null, "typ": "ganzzahl", "bereich": null, "enum_werte": []}, {"feld_id": "anteil", "fragetext_laie": "Welcher Anteil?", "hilfe_kurz": "Kurz", "typ": "zahl", "bereich": {"min": 0.5, "max": 1000000.0}, "enum_werte": ["it's", "ja"]}]"#;
    let alle = katalog(kat_json);
    {
        let katalog: Vec<&KatalogFeld> = vec![];
        let m = dialog_prompt(&gef("Ich bin ledig und habe 50000 Euro verdient"), &katalog, &gef(""), &aussagen(&[]));
        pruefe("ohne alles", &m, (4476, 0x6b79_0850_a365_cb71), "Ich bin ledig und habe 50000 Euro verdient");
        assert!(m[0].inhalt().contains("(keine anderen):\n\n\nGeld-Beträge"), "ohne alles: Feldliste");
        assert!(m[0].inhalt().contains("vor.\n\nFür die ANTWORT"), "ohne alles: Aussagen und Kontext");
    }
    {
        let katalog: Vec<&KatalogFeld> = vec![&alle[0], &alle[1], &alle[2], &alle[3]];
        let m = dialog_prompt(&gef("Ich bin ledig und habe 50000 Euro verdient"), &katalog, &gef(""), &aussagen(&[]));
        pruefe("vier Felder", &m, (4862, 0x4737_4bac_4665_ecd3), "Ich bin ledig und habe 50000 Euro verdient");
        assert!(m[0].inhalt().contains("(keine anderen):\n- bruttoarbeitslohn: Wie hoch war dein Bruttoarbeitslohn? (Typ geld, Bereich {'min': 0, 'max': 10000000})\n    dazu gehört: Steht auf der Lohnsteuerbescheinigung Nr. 3\n- kirchensteuer_art: None (Typ None, Werte ['ev', 'rk', 'keine'])\n- anzahl_kinder:  (Typ ganzzahl)\n- anteil: Welcher Anteil? (Typ zahl, Bereich {'min': 0.5, 'max': 1000000.0}, Werte [\"it's\", 'ja'])\n    dazu gehört: Kurz\n\nGeld-Beträge"), "vier Felder: Feldliste");
        assert!(m[0].inhalt().contains("vor.\n\nFür die ANTWORT"), "vier Felder: Aussagen und Kontext");
    }
    {
        let katalog: Vec<&KatalogFeld> = vec![&alle[0]];
        let m = dialog_prompt(&gef("Ich bin ledig und habe 50000 Euro verdient"), &katalog, &gef(""), &aussagen(&["Der Nutzer ist ledig", "Der Nutzer hat 50000 Euro verdient"]));
        pruefe("Aussagen", &m, (5689, 0x084b_456d_0aef_8557), "Ich bin ledig und habe 50000 Euro verdient");
        assert!(m[0].inhalt().contains("(keine anderen):\n- bruttoarbeitslohn: Wie hoch war dein Bruttoarbeitslohn? (Typ geld, Bereich {'min': 0, 'max': 10000000})\n    dazu gehört: Steht auf der Lohnsteuerbescheinigung Nr. 3\n\nGeld-Beträge"), "Aussagen: Feldliste");
        assert!(m[0].inhalt().contains("vor.\n\nDAS HABEN WIR AUS SEINER NACHRICHT HERAUSGELESEN — jede Zeile ist eine Tatsache, die er über sich gesagt hat:\n[0] Der Nutzer ist ledig\n[1] Der Nutzer hat 50000 Euro verdient\nARBEITE DIESE LISTE ZEILE FÜR ZEILE AB. Zu JEDER Zeile gehört ein Eintrag: ein Vorschlag, wenn der Wert im Text steht — sonst eine Rückfrage. Übergehen darfst du eine Zeile nur, wenn die Feldliste oben zu ihrem Thema wirklich nichts enthält; Unsicherheit ist KEIN Grund zum Übergehen, dafür ist die Rückfrage da. Gib in `aussage` die Nummer der Zeile an, auf die sich der Eintrag stützt.\nHÖCHSTENS EINE RÜCKFRAGE JE ZEILE, und sie fragt nach GENAU EINER Sache. Eine Zeile lässt eine Unklarheit offen — die klärst du. Frag NICHT alle Felder ab, deren Thema die Zeile berührt: „zwei Kinder\" berührt Vornamen, Geburtsdaten und Elternteile, aber die stehen im Fragebogen und sind dort nicht verloren. Und bündle nie zwei Fragen in einen Satz („Wie heissen sie und wann sind sie geboren?\") — der Nutzer hat genau EIN Eingabefeld dafür und kann nur eine der beiden beantworten.\n\nFür die ANTWORT"), "Aussagen: Aussagen und Kontext");
    }
    {
        let katalog: Vec<&KatalogFeld> = vec![&alle[0], &alle[1]];
        let m = dialog_prompt(&gef("Ich bin ledig und habe 50000 Euro verdient"), &katalog, &gef("Offenes Feld: bruttoarbeitslohn"), &aussagen(&[]));
        pruefe("Kontext", &m, (4741, 0xa775_cc53_f890_f808), "Ich bin ledig und habe 50000 Euro verdient");
        assert!(m[0].inhalt().contains("(keine anderen):\n- bruttoarbeitslohn: Wie hoch war dein Bruttoarbeitslohn? (Typ geld, Bereich {'min': 0, 'max': 10000000})\n    dazu gehört: Steht auf der Lohnsteuerbescheinigung Nr. 3\n- kirchensteuer_art: None (Typ None, Werte ['ev', 'rk', 'keine'])\n\nGeld-Beträge"), "Kontext: Feldliste");
        assert!(m[0].inhalt().contains("vor.\n\nOffenes Feld: bruttoarbeitslohn\n\nFür die ANTWORT"), "Kontext: Aussagen und Kontext");
    }
    {
        let katalog: Vec<&KatalogFeld> = vec![&alle[0], &alle[2]];
        let m = dialog_prompt(&gef("Ich bin ledig und habe 50000 Euro verdient"), &katalog, &gef("Kontext zum Feld"), &aussagen(&["Der Nutzer ist ledig"]));
        pruefe("Aussage und Kontext", &m, (5701, 0x6d4c_4a50_a21f_d873), "Ich bin ledig und habe 50000 Euro verdient");
        assert!(m[0].inhalt().contains("(keine anderen):\n- bruttoarbeitslohn: Wie hoch war dein Bruttoarbeitslohn? (Typ geld, Bereich {'min': 0, 'max': 10000000})\n    dazu gehört: Steht auf der Lohnsteuerbescheinigung Nr. 3\n- anzahl_kinder:  (Typ ganzzahl)\n\nGeld-Beträge"), "Aussage und Kontext: Feldliste");
        assert!(m[0].inhalt().contains("vor.\n\nDAS HABEN WIR AUS SEINER NACHRICHT HERAUSGELESEN — jede Zeile ist eine Tatsache, die er über sich gesagt hat:\n[0] Der Nutzer ist ledig\nARBEITE DIESE LISTE ZEILE FÜR ZEILE AB. Zu JEDER Zeile gehört ein Eintrag: ein Vorschlag, wenn der Wert im Text steht — sonst eine Rückfrage. Übergehen darfst du eine Zeile nur, wenn die Feldliste oben zu ihrem Thema wirklich nichts enthält; Unsicherheit ist KEIN Grund zum Übergehen, dafür ist die Rückfrage da. Gib in `aussage` die Nummer der Zeile an, auf die sich der Eintrag stützt.\nHÖCHSTENS EINE RÜCKFRAGE JE ZEILE, und sie fragt nach GENAU EINER Sache. Eine Zeile lässt eine Unklarheit offen — die klärst du. Frag NICHT alle Felder ab, deren Thema die Zeile berührt: „zwei Kinder\" berührt Vornamen, Geburtsdaten und Elternteile, aber die stehen im Fragebogen und sind dort nicht verloren. Und bündle nie zwei Fragen in einen Satz („Wie heissen sie und wann sind sie geboren?\") — der Nutzer hat genau EIN Eingabefeld dafür und kann nur eine der beiden beantworten.\n\nKontext zum Feld\n\nFür die ANTWORT"), "Aussage und Kontext: Aussagen und Kontext");
    }
    {
        let katalog: Vec<&KatalogFeld> = vec![&alle[0]];
        let m = dialog_prompt(&gef("Ich bin ledig und habe 50000 Euro verdient"), &katalog, &gef(""), &aussagen(&[]));
        pruefe("ein Feld", &m, (4642, 0xbd52_f104_372e_10b0), "Ich bin ledig und habe 50000 Euro verdient");
        assert!(m[0].inhalt().contains("(keine anderen):\n- bruttoarbeitslohn: Wie hoch war dein Bruttoarbeitslohn? (Typ geld, Bereich {'min': 0, 'max': 10000000})\n    dazu gehört: Steht auf der Lohnsteuerbescheinigung Nr. 3\n\nGeld-Beträge"), "ein Feld: Feldliste");
        assert!(m[0].inhalt().contains("vor.\n\nFür die ANTWORT"), "ein Feld: Aussagen und Kontext");
    }
}
