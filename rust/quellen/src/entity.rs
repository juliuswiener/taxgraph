//! HTML-Entities dekodieren, wie `html.unescape` aus Python, aber streng.
//!
//! Gleich wie Python: `&name;` für die 253 Namen der Tabelle (`entity_tabelle`), `&#N;` und `&#xH;` für erlaubte Zeichen, in
//! EINEM Durchgang (`&amp;nbsp;` wird `&nbsp;`). Ein `&` ohne Buchstabe oder `#Ziffer` dahinter bleibt stehen.
//!
//! Anders als Python, mit Absicht: Python lässt `&foo;` stehen, deutet `&sect` ohne Semikolon als `§` (Altlast von HTML 4)
//! und ersetzt eine unzulässige Zahl durch U+FFFD, durch das cp1252-Zeichen oder durch nichts. Das verändert einen Rechtstext
//! unbemerkt. Hier ist jeder dieser Fälle ein Fehler mit Namen, und nichts wird geschrieben.

use crate::entity_tabelle::BENANNT;
use crate::Fehler;

/// Ersetzt `&name;`, `&#N;` und `&#xH;` durch ihr Zeichen.
///
/// ```
/// assert_eq!(quellen::entity::dekodiere("&sect;&nbsp;9 &amp; &#8211; &#xE4;").unwrap(), "§\u{a0}9 & – ä");
/// assert!(quellen::entity::dekodiere("&gibtesnicht;").is_err());
/// ```
///
/// # Errors
/// [`Fehler::UnbekannteEntity`], [`Fehler::EntityOhneSemikolon`], [`Fehler::UnzulaessigeZeichenreferenz`].
pub fn dekodiere(text: &str) -> Result<String, Fehler> {
    let mut aus = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((davor, danach)) = rest.split_once('&') {
        aus.push_str(davor);
        if let Some((zeichen, verbraucht)) = am_anfang(danach)? {
            aus.push(zeichen);
            rest = danach.get(verbraucht..).unwrap_or_default();
        } else {
            aus.push('&');
            rest = danach;
        }
    }
    aus.push_str(rest);
    Ok(aus)
}

/// Liest die Entity, die direkt hinter einem `&` steht: Zeichen und Zahl der verbrauchten Bytes, oder `None`, wenn dort
/// keine Entity beginnt. Alle Namen und Ziffern sind ASCII, Bytes und Zeichen sind hier gleich.
fn am_anfang(s: &str) -> Result<Option<(char, usize)>, Fehler> {
    if let Some(rest) = s.strip_prefix('#') {
        return zahlenreferenz(rest);
    }
    if !s.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Ok(None);
    }
    let name: String = s.chars().take_while(char::is_ascii_alphanumeric).collect();
    if !s.get(name.len()..).is_some_and(|r| r.starts_with(';')) {
        return Err(Fehler::EntityOhneSemikolon { name });
    }
    match BENANNT.binary_search_by(|(n, _)| n.as_bytes().cmp(name.as_bytes())) {
        Ok(i) => Ok(BENANNT.get(i).map(|(_, c)| (*c, name.len() + 1))),
        Err(_) => Err(Fehler::UnbekannteEntity { name }),
    }
}

/// Der Teil hinter `&#`: `N;` oder `xH;` (auch `X`).
fn zahlenreferenz(rest: &str) -> Result<Option<(char, usize)>, Fehler> {
    let (hex, ziffern_ab) = match rest.strip_prefix(['x', 'X']) {
        Some(h) => (true, h),
        None => (false, rest),
    };
    let ist_ziffer = |c: &char| if hex { c.is_ascii_hexdigit() } else { c.is_ascii_digit() };
    let ziffern: String = ziffern_ab.chars().take_while(ist_ziffer).collect();
    if ziffern.is_empty() {
        return Ok(None);
    }
    let bis = usize::from(hex) + ziffern.len();
    let vorn = if hex { rest.get(..1).unwrap_or_default() } else { "" };
    if !rest.get(bis..).is_some_and(|r| r.starts_with(';')) {
        return Err(Fehler::EntityOhneSemikolon { name: format!("#{vorn}{ziffern}") });
    }
    let schreibweise = format!("&#{vorn}{ziffern};");
    let zeichen = u32::from_str_radix(&ziffern, if hex { 16 } else { 10 }).ok().and_then(zulaessig);
    match zeichen {
        Some(c) => Ok(Some((c, 1 + bis + 1))),
        None => Err(Fehler::UnzulaessigeZeichenreferenz { text: schreibweise }),
    }
}

/// Die Zeichen, die Python für `&#N;` unverändert liefert: nicht 0, keine Steuerzeichen außer Tab, LF, FF, CR (U+000B, U+007F
/// bis U+009F ausgenommen), keine Surrogate, nichts über U+10FFFF, keine Nichtzeichen (U+FDD0 bis U+FDEF, U+xFFFE, U+xFFFF).
fn zulaessig(code: u32) -> Option<char> {
    let steuer = matches!(code, 0x0..=0x8 | 0xb | 0xe..=0x1f | 0x7f..=0x9f);
    let nichtzeichen = (0xfdd0..=0xfdef).contains(&code) || (code & 0xfffe) == 0xfffe;
    if steuer || nichtzeichen {
        None
    } else {
        char::from_u32(code)
    }
}
