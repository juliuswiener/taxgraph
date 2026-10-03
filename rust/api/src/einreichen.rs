//! `api.einreichen` (`api.py:678`): Deklaration → ELSTER-XML → checkESt-Plausibilitätsprüfung.
//! KEIN Versand.
//!
//! Der Weg bis zum XML liegt in [`bescheid::deklaration::einreichungs_xml`]; hier stehen die
//! Antworten an jeder Abbruchstelle, der Aufruf von `ERiC` ([`elster::validiere`]: nur
//! `ERIC_VALIDIERE`, lokal, ohne Netz und ohne Zertifikat) und die Bindung des Befunds an den
//! geprüften Zustand (Snapshot mit `eric_befund`, Invariante 5 aus `store/SCHEMA.md`).
//! `ERIC_ENCRYPT_AND_SEND` ist nicht verdrahtet und bleibt es: diese Crate kennt kein Versandflag.
//!
//! # Abweichungen von Python
//! `ponytail`: ein Veranlagungszeitraum ausserhalb 2024–2026 in einer Fall-Datei meldet Rust als
//! 500 `ValueError`; Python rechnet weiter und scheitert erst in `deklariere` (0, unter 2024,
//! ueber 2100) oder fragt `ERiC` nach `ESt_<jahr>` (2027–2100). `POST /fall` legt solche Jahre nicht
//! an (nur Jahre mit `params/`), erreichbar ist es nur mit einer von Hand geschriebenen Akte.
//!
//! Die Bibliothek laden oder initialisieren scheitert in Python an `AttributeError` (fehlendes
//! Symbol) mit 500; Rust meldet es wie jeden Ladefehler als 503.
use bescheid::deklaration::{einreichungs_xml, sperrgrund_klartext, EinreichFehler, Einreichung};
use elster::{
    gekappt_verdacht, klasse_name, klassifiziere_rc, unerwarteter_rc_hinweis, validiere,
    EricKlasse, RC_DATENARTVERSION_UNBEKANNT, RC_OK,
};
use serde_json::{json, Map, Value};
use store::audit::AuditAktion;
use store::EricBefundEingabe;

use crate::antwort::Antwort;
use crate::deklaration::deklarations_fehler;
use crate::eigener_fall::EigenerFall;
use crate::fehler::ApiFehler;
use crate::python::{repr, typname, wahr};
use crate::stand::bescheid_fehler;
use crate::zustand::{Nutzer, Zustand as Dienst};

/// Der Text der 409 `scheibe_nicht_abgabefaehig`, mit der Scheibe als Pythons `repr` und der Zahl
/// der fehlenden Stammdatenfelder.
fn teilrechnung_hinweis(scheibe: &str, fehlende: usize) -> String {
    format!(
        "Die Scheibe {scheibe} ist eine Teilrechnung und kann keine Einkommensteuererklaerung \
         tragen: {fehlende} Stammdatenfelder (Name, Anschrift, Steuernummer) liegen nicht in \
         ihrem Fragenkegel und koennen dort auch nicht beantwortet werden. Legen Sie den Fall auf \
         einer abgabefaehigen Scheibe an ('gesamt' oder 'rentner_gesamt'). Die Berechnung auf \
         dieser Scheibe bleibt nutzbar."
    )
}

/// `str(body.get("empfaenger_land") or "BY")`. Ein Rumpf ohne `get` ist eine `AttributeError`
/// (500), ein falscher Wert heisst `"BY"`, ein wahrer Nicht-Text ist sein `str`.
fn empfaenger_land(body: &Value) -> Result<String, ApiFehler> {
    let Value::Object(rumpf) = body else {
        return Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'get'", typname(body)),
        ));
    };
    Ok(match rumpf.get("empfaenger_land") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(w) if wahr(w) => repr(w),
        _ => "BY".to_owned(),
    })
}

/// Der Anfang jeder Antwort ab dem Prüfen: `fall_id`, `eingereicht: false` und was `ERiC` sah.
fn mit(basis: &Map<String, Value>, rest: Value) -> Antwort {
    let mut m = basis.clone();
    if let Value::Object(r) = rest {
        m.extend(r);
    }
    Antwort::neu(422, Value::Object(m))
}

/// Die Antwort an der Stelle, an der `einreichungs_xml` abbricht — oder der Fehler, den Python dort
/// wirft.
fn abbruch(e: EinreichFehler, fall_id: &str, scheibe_roh: Option<&str>) -> Result<Antwort, ApiFehler> {
    match e {
        EinreichFehler::Scheibe(f) => Err(f.into()),
        EinreichFehler::Snapshot(f) => Err(ApiFehler::unerwartet("ValueError", f.to_string())),
        EinreichFehler::Veranlagungszeitraum(jahr) => Err(ApiFehler::unerwartet(
            "ValueError",
            format!("Veranlagungszeitraum {jahr} wird nicht unterstuetzt"),
        )),
        EinreichFehler::Bescheid(f) => Err(bescheid_fehler(&f)),
        EinreichFehler::Deklaration(f) => Err(deklarations_fehler(&f)),
        EinreichFehler::ScheibeNichtAbgabefaehig {
            fehlende_stammdatenfelder,
            ..
        } => {
            let scheibe = scheibe_roh.map_or_else(|| "None".to_owned(), |s| repr(&json!(s)));
            Ok(Antwort::neu(
                409,
                json!({
                    "fall_id": fall_id, "eingereicht": false,
                    "grund": "scheibe_nicht_abgabefaehig",
                    "scheibe": scheibe_roh,
                    "fehlende_stammdatenfelder": fehlende_stammdatenfelder,
                    "hinweis": teilrechnung_hinweis(&scheibe, fehlende_stammdatenfelder.len()),
                }),
            ))
        }
        EinreichFehler::Gesperrt(grund) => Ok(Antwort::neu(
            409,
            json!({
                "fall_id": fall_id, "eingereicht": false, "grund": grund.to_string(),
                "klartext": sperrgrund_klartext(Some(grund)),
                "hinweis": "Die Deklaration kann nicht erstellt werden, weil eine erforderliche \
                            Angabe fehlt.",
            }),
        )),
        EinreichFehler::DeklarationUnvollstaendig(eintraege) => {
            let unvollstaendig = serde_json::to_value(&eintraege)
                .map_err(|e| ApiFehler::unerwartet("TypeError", e.to_string()))?;
            Ok(Antwort::neu(
                409,
                json!({
                    "fall_id": fall_id, "eingereicht": false,
                    "grund": "deklaration_unvollstaendig", "unvollstaendig": unvollstaendig,
                }),
            ))
        }
        EinreichFehler::XmlNichtBaubar(f) => Ok(Antwort::neu(
            422,
            json!({
                "fall_id": fall_id, "eingereicht": false, "grund": "xml_nicht_baubar",
                "detail": f.to_string(),
            }),
        )),
    }
}

/// `api.einreichen(fall_id, body)` nach dem Owner-Check. Blockiert, solange `ERiC` prüft.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung, 500 mit der Python-Klasse, wo Python eine andere Ausnahme
/// wirft, 500 bei einem Snapshot, dessen Hash vom geprüften Zustand abweicht.
pub fn einreichen(
    z: &Dienst,
    nutzer: &Nutzer,
    fall: &mut EigenerFall,
    body: &Value,
) -> Result<Antwort, ApiFehler> {
    let fall_id = fall.id().as_str().to_owned();
    let land = empfaenger_land(body);
    let sb = z.scheibe_bindung(fall.store())?;
    let gebaut = einreichungs_xml(
        fall.store(),
        &sb.index,
        z.params()?,
        land.as_deref().unwrap_or("BY"),
        None,
    );
    let einr = match (gebaut, land) {
        // Python wirft `body.get` an der Stelle, an der es `erzeuge_xml` aufruft: vorher endet es
        // wie immer, an `erzeuge_xml` kommt es nicht mehr heran.
        (Ok(_) | Err(EinreichFehler::XmlNichtBaubar(_)), Err(f)) => return Err(f),
        (Err(e), _) => {
            return abbruch(e, &fall_id, fall.store().datei().scheibe.as_deref());
        }
        (Ok(einr), Ok(_)) => einr,
    };
    pruefe(z, nutzer, fall, &fall_id, &einr)
}

/// Invariante 5: der Befund bindet an genau den Zustand, den er prüfte — VOR den
/// Fallunterscheidungen, damit jedes Ergebnis gebunden wird. Ohne Events gibt es keinen Zustand,
/// an den zu binden wäre; die Antwort sagt es (`befund_gebunden`).
fn binde_befund(
    fall: &mut EigenerFall,
    einr: &Einreichung,
    rc: i64,
    klasse: EricKlasse,
    antwort: &str,
) -> Result<bool, ApiFehler> {
    if fall.store().events().is_empty() {
        return Ok(false);
    }
    let eingabe = EricBefundEingabe {
        rc,
        klasse,
        gekappt_verdacht: gekappt_verdacht(antwort),
        fehler_anzahl: None,
    };
    let snap = fall
        .store_mut()
        .erzeuge_snapshot(None, None, Some(eingabe))
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?
        .to_string();
    let sid = einr.deklaration.basis_snapshot.clone().unwrap_or_default();
    if snap != sid {
        return Err(ApiFehler::status(
            500,
            format!(
                "Snapshot-Hash weicht ab ({} != {}) — der Befund würde an einen anderen \
                 Zustand binden",
                snap.chars().take(12).collect::<String>(),
                sid.chars().take(12).collect::<String>()
            ),
        ));
    }
    store::speichere(fall.pfad(), fall.store().datei())?;
    Ok(true)
}

/// Ab dem XML: `ERiC` prüft, der Befund bindet an den Zustand, die Antwort nennt das Urteil.
fn pruefe(
    z: &Dienst,
    nutzer: &Nutzer,
    fall: &mut EigenerFall,
    fall_id: &str,
    einr: &Einreichung,
) -> Result<Antwort, ApiFehler> {
    let vz = i64::from(einr.vz.jahr());
    let (rc, antwort) = match validiere(einr.xml.as_bytes(), &format!("ESt_{vz}")) {
        Ok(x) => x,
        Err(e) => {
            return Ok(Antwort::neu(
                503,
                json!({
                    "fall_id": fall_id, "eingereicht": false, "grund": "eric_nicht_verfuegbar",
                    "detail": e.to_string(),
                }),
            ));
        }
    };
    let rc = i64::from(rc);
    let klasse = klassifiziere_rc(rc);

    let befund_gebunden = binde_befund(fall, einr, rc, klasse, &antwort)?;

    let mut basis = Map::new();
    for (k, v) in [
        ("fall_id", json!(fall_id)),
        ("eingereicht", json!(false)),
        ("basis_snapshot", json!(einr.deklaration.basis_snapshot)),
        ("befund_gebunden", json!(befund_gebunden)),
        ("vz", json!(vz)),
        ("rc", json!(rc)),
        ("klasse", json!(klasse_name(klasse))),
        ("xml_bytes", json!(einr.xml.len())),
    ] {
        basis.insert(k.to_owned(), v);
    }
    if rc == RC_DATENARTVERSION_UNBEKANNT {
        // Kein Pruefmodul fuer diesen VZ: die Erklaerung wurde NICHT geprueft.
        return Ok(mit(
            &basis,
            json!({
                "grund": "kein_pruefmodul_fuer_vz",
                "detail": format!(
                    "ERiC kennt die Datenartversion ESt_{vz} nicht — fuer diesen \
                     Veranlagungszeitraum liegt kein Pruefmodul vor. Die Erklaerung wurde \
                     nicht geprueft."
                ),
                "ericantwort": antwort,
            }),
        ));
    }
    if klasse == EricKlasse::PlausibilitaetFehler {
        return Ok(mit(
            &basis,
            json!({
                "grund": "plausibilitaet_verletzt", "ericantwort": antwort,
                "moeglicherweise_gekappt": gekappt_verdacht(&antwort),
            }),
        ));
    }
    if rc != RC_OK {
        // Fail-closed: jeder andere rc ist KEIN Plausibilitaetsverdikt.
        return Ok(mit(
            &basis,
            json!({
                "grund": "rc_kein_plausibilitaetsverdikt",
                "detail": unerwarteter_rc_hinweis(rc, &antwort), "ericantwort": antwort,
            }),
        ));
    }
    let uid = nutzer.0.as_deref().unwrap_or("dev");
    store::audit::anhaengen(
        &z.konfig.audit_pfad(),
        Some(uid),
        AuditAktion::FallValidiert,
        Some(fall_id),
        Some(&format!("vz={vz} rc=0")),
    )?;
    let nicht_deklariert = serde_json::to_value(&einr.deklaration.nicht_deklariert)
        .map_err(|e| ApiFehler::unerwartet("TypeError", e.to_string()))?;
    let mut ok = basis;
    ok.insert("plausibel".to_owned(), json!(true));
    ok.insert("nicht_deklariert".to_owned(), nicht_deklariert);
    ok.insert(
        "hinweis".to_owned(),
        json!(
            "checkESt bestanden. Versand ist nicht verdrahtet — ERIC_ENCRYPT_AND_SEND braucht \
             Zertifikat + explizite Freigabe."
        ),
    );
    Ok(Antwort::neu(200, Value::Object(ok)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn land_wie_python() {
        let l = |v: Value| empfaenger_land(&v).unwrap();
        assert_eq!(l(json!({})), "BY");
        assert_eq!(l(json!({"empfaenger_land": "NW"})), "NW");
        for leer in [json!(""), json!(0), json!(null), json!(false), json!([]), json!({}), json!(0.0)] {
            assert_eq!(l(json!({"empfaenger_land": leer})), "BY");
        }
        assert_eq!(l(json!({"empfaenger_land": 5})), "5");
        assert_eq!(l(json!({"empfaenger_land": true})), "True");
        assert_eq!(l(json!({"empfaenger_land": [1, "a"]})), "[1, 'a']");
        assert_eq!(l(json!({"empfaenger_land": {"a": null}})), "{'a': None}");
        let f = empfaenger_land(&json!([1])).unwrap_err();
        assert!(matches!(f, ApiFehler::Unerwartet { ref typ, ref meldung }
            if typ == "AttributeError" && meldung == "'list' object has no attribute 'get'"));
    }

    #[test]
    fn teilrechnung_wie_python() {
        assert_eq!(
            teilrechnung_hinweis("'an_gesamt'", 12),
            "Die Scheibe 'an_gesamt' ist eine Teilrechnung und kann keine \
             Einkommensteuererklaerung tragen: 12 Stammdatenfelder (Name, Anschrift, \
             Steuernummer) liegen nicht in ihrem Fragenkegel und koennen dort auch nicht \
             beantwortet werden. Legen Sie den Fall auf einer abgabefaehigen Scheibe an \
             ('gesamt' oder 'rentner_gesamt'). Die Berechnung auf dieser Scheibe bleibt nutzbar."
        );
    }
}
