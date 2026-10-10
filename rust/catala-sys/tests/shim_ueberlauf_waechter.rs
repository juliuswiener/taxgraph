//! `mpz_get_si` steht im C-Shim nur im Makro `TG_AUS`.
//!
//! Warum: GMP rechnet exakt. `mpz_get_si` gibt bei einem Wert ausserhalb von `long` still dessen
//! untere Bits zurueck, ein Betrag wuerde falsch statt als Ueberlauf gemeldet. `TG_AUS` haelt
//! daneben fest, ob der Wert passte (`mpz_fits_slong_p`), und Rust liest erst dann
//! (`Ausgabe::cent`). Ein `mpz_get_si` im Klartext waere ein Rueckfall. Das Verhalten prueft
//! `ausgabe_ueberlauf_hermetisch.rs` je Ausgabe; dieser Text-Waechter faengt auch eine Stelle,
//! die dort keinen Fall hat, und jede neu hinzukommende (der Shim hat 45 `TG_AUS`-Aufrufe).
//!
//! Der Waechter prueft Text, keinen Lauf: kein Catala, kein Orakel. Ersetzt
//! `tests/test_ueberlauf_waechter_text.py` (Waechter 1, Python-Test, entfaellt mit Python).
//! Der Waechter ist eine reine Funktion (`probleme`); die Tests fuettern sie mit dem echten Text
//! und mit Fassungen, in denen genau der Rueckfall steckt. Ein Waechter, der nur den echten Text
//! sieht, kaeme auch gruen, wenn er nie anschluege.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

/// C nach Zeilenverkettung (`\` am Zeilenende), ohne Kommentare, Inhalt von Zeichenketten und
/// -literalen leer. Zeilenenden bleiben, damit eine Direktive mit Fortsetzungszeilen EINE Zeile ist.
fn bereinigt(text: &str) -> Result<String, String> {
    let text = text.replace("\r\n", "\n");
    let mut verkettet = String::with_capacity(text.len());
    for zeile in text.split('\n') {
        let ohne_rand = zeile.trim_end_matches([' ', '\t']);
        // Der Praeprozessor klebt die Zeilen OHNE Leerzeichen: `mpz_get_\<NL>si` ist `mpz_get_si`.
        if let Some(rest) = ohne_rand.strip_suffix('\\') {
            verkettet.push_str(rest);
        } else {
            verkettet.push_str(zeile);
            verkettet.push('\n');
        }
    }
    let mut aus = String::with_capacity(verkettet.len());
    let mut zeichen = verkettet.chars().peekable();
    while let Some(c) = zeichen.next() {
        match (c, zeichen.peek().copied()) {
            ('/', Some('*')) => {
                zeichen.next();
                let mut vorher = ' ';
                let mut zu = false;
                for d in zeichen.by_ref() {
                    if vorher == '*' && d == '/' {
                        zu = true;
                        break;
                    }
                    vorher = d;
                }
                if !zu {
                    return Err("Blockkommentar ohne Ende".to_owned());
                }
                aus.push(' ');
            }
            ('/', Some('/')) => {
                while zeichen.peek().is_some_and(|&d| d != '\n') {
                    zeichen.next();
                }
                aus.push(' ');
            }
            ('"' | '\'', _) => {
                aus.push(c);
                aus.push(c);
                while let Some(&d) = zeichen.peek() {
                    if d == c || d == '\n' {
                        break;
                    }
                    zeichen.next();
                    if d == '\\' {
                        zeichen.next();
                    }
                }
                if zeichen.peek() == Some(&c) {
                    zeichen.next();
                }
            }
            _ => aus.push(c),
        }
    }
    Ok(aus)
}

fn ist_kennzeichen(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `wort` als ganzes Wort in `zeile` (`\bwort\b`), so oft es vorkommt, mit dem Rest hinter jedem Treffer.
fn treffer<'a>(zeile: &'a str, wort: &str) -> Vec<&'a str> {
    zeile
        .match_indices(wort)
        .filter(|(i, _)| {
            let vor = zeile.get(..*i).and_then(|s| s.chars().next_back());
            let nach = zeile.get(i + wort.len()..).unwrap_or("");
            !vor.is_some_and(ist_kennzeichen) && !nach.chars().next().is_some_and(ist_kennzeichen)
        })
        .map(|(i, _)| zeile.get(i + wort.len()..).unwrap_or(""))
        .collect()
}

/// `#define TG_AUS` (mit beliebigem Leerraum um `#` und `define`).
fn ist_tg_aus_define(zeile: &str) -> bool {
    zeile
        .trim_start()
        .strip_prefix('#')
        .map(str::trim_start)
        .and_then(|r| r.strip_prefix("define"))
        .filter(|r| r.starts_with([' ', '\t']))
        .is_some_and(|r| r.trim_start().split(|c| !ist_kennzeichen(c)).next() == Some("TG_AUS"))
}

/// Leer = in Ordnung. `quellen`: Dateiname und C-Text.
fn probleme(quellen: &[(String, String)]) -> Vec<String> {
    let mut p = Vec::new();
    if quellen.is_empty() {
        return vec![
            "keine C-Datei im Shim-Verzeichnis gefunden: der Waechter prueft nichts".into(),
        ];
    }
    let mut koepfe: Vec<String> = Vec::new();
    let mut aufrufe = 0usize;
    for (name, roh) in quellen {
        let text = match bereinigt(roh) {
            Ok(t) => t,
            Err(e) => {
                p.push(format!("{name}: {e}"));
                continue;
            }
        };
        for zeile in text.split('\n') {
            if ist_tg_aus_define(zeile) {
                koepfe.push(zeile.to_owned());
                continue;
            }
            aufrufe += treffer(zeile, "TG_AUS")
                .iter()
                .filter(|rest| rest.trim_start().starts_with('('))
                .count();
            if !treffer(zeile, "mpz_get_si").is_empty() {
                let kurz: String = zeile.split_whitespace().collect::<Vec<_>>().join(" ");
                p.push(format!("{name}: mpz_get_si ausserhalb von TG_AUS: {kurz}"));
            }
        }
    }
    match koepfe.as_slice() {
        [kopf] => {
            if treffer(kopf, "mpz_get_si").is_empty() || !kopf.contains("mpz_fits_slong_p") {
                p.push(
                    "TG_AUS wandelt nicht mit mpz_get_si UND prueft mit mpz_fits_slong_p".into(),
                );
            }
        }
        andere => p.push(format!(
            "das Makro TG_AUS ist {}-mal definiert, erwartet genau einmal",
            andere.len()
        )),
    }
    if aufrufe == 0 {
        p.push("TG_AUS wird nirgends aufgerufen: der Waechter prueft nichts".into());
    }
    p
}

fn echte_quellen() -> Vec<(String, String)> {
    let csrc = Path::new(env!("CARGO_MANIFEST_DIR")).join("csrc");
    let mut dateien: Vec<_> = std::fs::read_dir(&csrc)
        .expect("csrc lesbar")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "c" || x == "h"))
        .collect();
    dateien.sort();
    dateien
        .iter()
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(p).expect("C-Datei lesbar"),
            )
        })
        .collect()
}

const MAKRO: &str = "#define TG_AUS(dst, r, feld)                          \\\n  do {                                                \\\n    (dst).wert = mpz_get_si((r)->feld);               \\\n    (dst).passt = mpz_fits_slong_p((r)->feld) != 0;   \\\n    (dst).name = #feld;                               \\\n  } while (0)\n";
const NUTZER: &str = "void f(Out *out, R *r) {\n  TG_AUS(*out, r, A__b);\n}\n";

fn mit(rest: &str) -> Vec<String> {
    probleme(&[("shim.c".to_owned(), format!("{MAKRO}{rest}"))])
}

/// Der echte Shim: `mpz_get_si` steht nur im Makro, das Makro prueft mit `mpz_fits_slong_p`.
#[test]
fn mpz_get_si_steht_nur_im_makro_tg_aus() {
    let quellen = echte_quellen();
    assert!(
        quellen.iter().any(|(n, _)| n == "shim.c"),
        "shim.c fehlt in csrc: {:?}",
        quellen.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    assert_eq!(probleme(&quellen), Vec::<String>::new());
}

/// Gegenprobe am echten Text: ein `TG_AUS(...)` des echten shim.c wird im Speicher durch das
/// Klartext-Aequivalent ersetzt, in drei Schreibweisen; der Waechter meldet genau diese Stelle.
#[test]
fn waechter_schlaegt_an_wenn_ein_echter_aufruf_zu_klartext_wird() {
    let echt = echte_quellen()
        .into_iter()
        .find(|(n, _)| n == "shim.c")
        .unwrap()
        .1;
    let aufruf = "  TG_AUS(*out, r, EuerGewinn__gewinn);";
    assert_eq!(echt.matches(aufruf).count(), 1, "Aufruf nicht eindeutig");
    for schreibweise in ["mpz_get_si", "mpz_get_\\\nsi", "mpz_\\ \nget_\\\r\nsi"] {
        let klartext = format!("  (*out).wert = {schreibweise}(r->EuerGewinn__gewinn);");
        let p = probleme(&[("shim.c".to_owned(), echt.replacen(aufruf, &klartext, 1))]);
        assert!(
            p.len() == 1 && p.iter().all(|m| m.contains("ausserhalb von TG_AUS")),
            "{schreibweise:?}: {p:?}"
        );
    }
}

/// Der Waechter selbst: sauberer Text geht durch (auch mit dem Namen in Kommentar und Zeichenkette),
/// jede Form des Rueckfalls schlaegt an.
#[test]
fn waechter_trennt_sauberen_text_vom_rueckfall() {
    assert_eq!(mit(NUTZER).len(), 0);
    let kommentare = "/* mpz_get_si */ // mpz_get_si\nconst char *s = \"mpz_get_si\";\n";
    assert_eq!(mit(&format!("{kommentare}{NUTZER}")).len(), 0);

    let rueckfaelle = [
        (
            "Klartext-Aufruf",
            "long x(R *r) { return mpz_get_si(r->a); }\n",
        ),
        (
            "neben einem Makro-Aufruf",
            "void f(R *r) { TG_AUS(o, r, a); long y = mpz_get_si(r->b); }\n",
        ),
        ("zweites Makro", "#define NOCH_EINS(r) mpz_get_si(r)\n"),
        ("Alias-Makro", "#define GET mpz_get_si\n"),
        (
            "hinter einem Kommentarende",
            "/* ok */ long x(R *r) { return mpz_get_si(r->a); }\n",
        ),
        (
            "Muster `/*` in einer Zeichenkette schuetzt nicht",
            "const char *s = \"/*\"; long x(R *r) { return mpz_get_si(r->a); }\n",
        ),
    ];
    for (name, text) in rueckfaelle {
        let p = mit(&format!("{text}{NUTZER}"));
        assert!(
            p.iter().any(|m| m.contains("ausserhalb von TG_AUS")),
            "{name}: Waechter schlug nicht an: {p:?}"
        );
    }

    // Das Makro verliert die Pruefung, wird doppelt definiert, oder niemand ruft es.
    let ohne_pruefung = MAKRO.replace("mpz_fits_slong_p", "mpz_sgn");
    assert_ne!(probleme(&[("shim.c".to_owned(), format!("{ohne_pruefung}{NUTZER}"))]).len(), 0);
    assert_ne!(probleme(&[("shim.c".to_owned(), format!("{MAKRO}{MAKRO}{NUTZER}"))]).len(), 0);
    assert_ne!(mit("void f(void) {}\n").len(), 0);
    assert_ne!(probleme(&[]).len(), 0);
}
