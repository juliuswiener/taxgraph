//! `zugriff` — die Accessor-Schicht aus `produkt/engine/runner.py`: je `catala_<name>` eine
//! Funktion `<name>` mit typisierter Eingabe. Hier entstehen die Werte, die Teil A (Scope-Ebene)
//! fertig erwartet: Sachverhalt → Catala-Eingabe, Parameter aus `params/<vz>`, Cent → Euro per
//! Abrundung wie Pythons `//`.
//!
//! Aufteilung auf `teil1` (runner.py bis `catala_p7_linear_afa`) und `teil2` (ab
//! `catala_sparer_pb`) folgt nur der Arbeitsorganisation beim Port; öffentlich ist die flache
//! Sicht über die `pub use`.

pub mod teil1;
pub mod teil2;
