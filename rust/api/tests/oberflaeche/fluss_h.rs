//! Fluss H: der KI-Weg. Die KI setzt nichts; sie schlaegt vor und fragt nach, und was der Nutzer dazu eintippt, geht
//! ueber den normalen Schreibweg in den Store.
//!
//! Sechs Tests, einer je Gruppe der Python-Tests (`ki_flow`, `ki_fokus`, `chat_aussagen`, `chat_wartezustand`,
//! `verstanden_seite`, `dialog_kanal`). `/chat` wird im Browser gespielt (`chat.rs`), wie Python es mit `page.route`
//! tat: der Dienst sieht den Aufruf nie, es fliesst kein Schluessel und kein Cent. Gemessen wird, wo es zaehlt, der
//! STORE ueber `GET /stand`, nicht der Browser.
//!
//! GELD und DATEN: ein Eingabefeld, das Euro nicht in Cent wandelt, schriebe den 100-fachen Betrag (bestaetigt, also
//! zaehlend); eine nicht umgekehrte Ja/Nein-Frage schaltet einen ganzen Frageblock ab; ein leeres Feld darf keine 0
//! werden; „spaeter“ schreibt nichts; zwei Kinder brauchen zwei Ereignisse.

use serde_json::{json, Value};

use crate::chat::{antwort, gib_frei, halte_an, rf, rf_instanzen, rf_zustand, senden, setze_kinderzahl, stub, vorschlag, ModellErsatz};
use crate::dienst::{Dienst, Start};
use crate::seite::{zum_fragebogen, Umgebung};

/// Zwei echte Felder aus der Bindung, absichtlich die beiden heiklen Typen: `cent` (Euro in, Cent im Store) und `bool`
/// mit `frage_invertiert` (`vpf_keine_mahlzeitengestellung` benennt die Abwesenheit, die Frage fragt nach der Anwesenheit).
const CENT_FELD: &str = "bruttoarbeitslohn";
const BOOL_FELD_INVERTIERT: &str = "vpf_keine_mahlzeitengestellung";

/// Eine frische Seite mit neuem Fall auf dem Fragebogen-Weg; `ankreuzen` bedient die Ankreuzliste (None: weggeraeumt).
fn frisch(u: &Umgebung, ankreuzen: Option<&[&str]>) {
    u.oeffne_start();
    u.fall_beginnen("fragebogen");
    zum_fragebogen(&u.s, ankreuzen);
}

fn wert_von(u: &Umgebung, fall: &str, feld: &str) -> Option<Value> {
    u.stand_felder(fall).get(feld).cloned()
}

#[test]
fn fluss_h_ki_flow() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 780);
    let s = &u.s;

    // Teil 1: die Wegwahl steht zwischen Fallart und Fluss, genau zwei Wege.
    u.oeffne_start();
    u.s.skript("document.querySelector(\".kachel[data-scheibe='gesamt']\").click();", &[]);
    s.warte_sichtbar("#wegwahl", 10);
    pp!("H.die_wegwahl_steht_zwischen_fallart_und_fluss", !s.sichtbar("#start") && !s.sichtbar("#flow"), "der Fluss ist schon offen: die Wahl waere eine Zwischenseite");
    let knoepfe = s.ausdruck("[...document.querySelectorAll('#wegwahl .kachel')].map(b => b.id)");
    pp!("H.die_wegwahl_steht_zwischen_fallart_und_fluss", knoepfe == json!(["weg-fragebogen", "weg-ki"]), "nicht genau zwei Wege: {knoepfe}");
    let text = s.textinhalt("#weg-fragebogen");
    pp!("H.die_wegwahl_steht_zwischen_fallart_und_fluss", text.contains("KI") && text.contains("jederzeit"), "kein Hinweis, dass die KI zwischendurch gefuettert werden kann: {text:?}");
    pp!("H.die_wegwahl_steht_zwischen_fallart_und_fluss", s.ausdruck("document.documentElement.scrollWidth").as_i64().unwrap_or(-1) <= 360, "die Wegwahl scrollt bei 360 px seitwaerts");
    let fehler = u.konsolenfehler();
    pp!("H.die_wegwahl_steht_zwischen_fallart_und_fluss", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Der KI-Weg zeigt NUR das KI-Fenster; der Fragebogen steht nicht daneben, der Fokus liegt im Eingabefeld.
    u.oeffne_start();
    u.fall_beginnen("ki");
    s.warte_sichtbar("#chat-body .chat-erklaer", 8);
    pp!("H.der_ki_weg_zeigt_nur_das_ki_fenster", !s.sichtbar("#wegpunkt"), "der Fragebogen steht neben dem KI-Fenster: der Nutzer hat ihn gerade abgewaehlt");
    pp!("H.der_ki_weg_zeigt_nur_das_ki_fenster", s.ausdruck("document.activeElement.id") == json!("chat-text"), "der Fokus liegt nicht im Eingabefeld der KI");
    pp!("H.der_ki_weg_zeigt_nur_das_ki_fenster", s.textinhalt("#chat-body").contains("Schreib einfach los"), "der KI-Weg sagt nicht, was der Nutzer jetzt tun soll");
    pp!("H.der_ki_weg_zeigt_nur_das_ki_fenster", s.sichtbar("#zum-fragebogen"), "kein sichtbarer Weg zurueck in den Fragebogen");

    // Der Fragebogen-Weg stellt die erste Frage, mit Fokus auf ihr.
    frisch(&u, None);
    pp!("H.der_fragebogen_weg_stellt_die_erste_frage", s.sichtbar("#wegpunkt"), "keine erste Frage");
    pp!("H.der_fragebogen_weg_stellt_die_erste_frage", s.ausdruck("document.activeElement.id") == json!("wegpunkt"), "der Fokus liegt nicht auf der Frage");

    // Teil 2: zwei Rueckfragen, aber nur EINE auf dem Schirm; „Weiter“ fuehrt zur naechsten.
    let fall = u.fall();
    pp!("H.rueckfragen_kommen_einzeln", u.offene(&fall).contains(&CENT_FELD.to_owned()), "{CENT_FELD} steht nicht in den offenen Fragen: der Test misst sonst den Rueckfall");
    stub(s, &[antwort(&[], &[rf("Meinst du 100.000 für das ganze Jahr oder anteilig bis Juni?", CENT_FELD), rf("Unterstützt du jemanden finanziell?", BOOL_FELD_INVERTIERT)], &[])]);
    pp!("H.rueckfragen_kommen_einzeln", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    let z = rf_zustand(s);
    pp!("H.rueckfragen_kommen_einzeln", z["offen"] == json!(true), "die Rueckfragen-Seite ist gar nicht erschienen");
    let frage = z["frage"].as_str().unwrap_or_default();
    pp!("H.rueckfragen_kommen_einzeln", frage.contains("anteilig bis Juni") && !frage.contains("Unterstützt du jemanden"), "beide Fragen stehen zugleich auf dem Schirm: {z}");
    pp!("H.rueckfragen_kommen_einzeln", z["zaehler"].as_str().unwrap_or_default().contains("1 von 2"), "kein Zaehler: {z}");
    pp!("H.rueckfragen_kommen_einzeln", z["eingaben"] == json!(1), "nicht genau ein Eingabefeld: {z}");
    s.fuelle("#rf-input", "62000");
    s.klick("#rf-weiter");
    let zweite = s.try_warte_bis("document.getElementById('rf-zaehler').textContent.includes('2 von 2')", 5);
    pp!("H.weiter_fuehrt_zur_naechsten_rueckfrage", zweite.is_some(), "die zweite Frage kam nicht");
    let z = rf_zustand(s);
    pp!("H.weiter_fuehrt_zur_naechsten_rueckfrage", z["frage"].as_str().unwrap_or_default().contains("Unterstützt du jemanden") && !z["frage"].as_str().unwrap_or_default().contains("Ganzes Jahr"), "{z}");

    // Ein Cent-Feld landet als Cent im Store (62.000 EUR sind 6.200.000 ct), bestaetigt, vom Laien.
    frisch(&u, None);
    let fall = u.fall();
    stub(s, &[antwort(&[], &[rf("Wie hoch war dein Bruttoarbeitslohn?", CENT_FELD)], &[])]);
    pp!("H.ein_cent_feld_landet_als_cent_im_store", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    s.fuelle("#rf-input", "62000");
    s.klick("#rf-weiter");
    s.warte_unsichtbar("#rueckfragen", 5);
    let f = wert_von(&u, &fall, CENT_FELD);
    pp!("H.ein_cent_feld_landet_als_cent_im_store", f.is_some(), "die Antwort ist gar nicht im Store angekommen: {}", u.stand_felder(&fall));
    let f = f.unwrap_or(Value::Null);
    pp!("H.ein_cent_feld_landet_als_cent_im_store", f["wert"] == json!(6_200_000), "62.000 EUR wurden als {} gespeichert statt als 6200000 Cent", f["wert"]);
    pp!("H.ein_cent_feld_landet_als_cent_im_store", f["zustand"] == json!("bestaetigt"), "die Antwort liegt nur vorlaeufig vor ({}): sie zaehlt in keiner Summe", f["zustand"]);
    pp!("H.ein_cent_feld_landet_als_cent_im_store", f["herkunft_badge"] == json!("laie"), "Herkunft {} statt 'laie'", f["herkunft_badge"]);

    // Die Ja/Nein-Umkehr gilt auch im Rueckfragen-Schritt: „Ja, ich unterstuetze jemanden“ ist `false` im Store.
    frisch(&u, None);
    let fall = u.fall();
    pp!("H.bool_umkehr_gilt_auch_im_rueckfragen_schritt", u.offene(&fall).contains(&BOOL_FELD_INVERTIERT.to_owned()), "{BOOL_FELD_INVERTIERT} steht nicht in den offenen Fragen");
    stub(s, &[antwort(&[], &[rf("Unterstützt du jemanden finanziell?", BOOL_FELD_INVERTIERT)], &[])]);
    pp!("H.bool_umkehr_gilt_auch_im_rueckfragen_schritt", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    s.skript("[...document.querySelectorAll('#rf-eingabe .wahl-opt')].find(b => b.textContent.trim() === 'Ja').click();", &[]);
    s.klick("#rf-weiter");
    s.warte_unsichtbar("#rueckfragen", 5);
    let f = wert_von(&u, &fall, BOOL_FELD_INVERTIERT);
    pp!("H.bool_umkehr_gilt_auch_im_rueckfragen_schritt", f.is_some(), "die Antwort ist nicht im Store angekommen");
    pp!(
        "H.bool_umkehr_gilt_auch_im_rueckfragen_schritt",
        f.as_ref().is_some_and(|f| f["wert"] == json!(false)),
        "„Ja, ich unterstuetze jemanden“ wurde als {:?} gespeichert: gespeichert gehoert das Gegenteil der Antwort",
        f.as_ref().map(|f| &f["wert"])
    );

    // Ein leeres Feld schreibt nichts (keine stille 0) und die Seite springt nicht weiter.
    frisch(&u, None);
    let fall = u.fall();
    stub(s, &[antwort(&[], &[rf("Wie hoch war dein Bruttoarbeitslohn?", CENT_FELD)], &[])]);
    pp!("H.ein_leeres_feld_schreibt_nichts", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    s.klick("#rf-weiter");
    let banner = s.try_warte_bis("!document.getElementById('netz-banner').hidden", 5);
    pp!("H.ein_leeres_feld_schreibt_nichts", banner.is_some(), "keine Meldung zum leeren Feld");
    pp!("H.ein_leeres_feld_schreibt_nichts", s.sichtbar("#rueckfragen"), "die Seite ist weitergesprungen, obwohl nichts eingegeben wurde");
    pp!("H.ein_leeres_feld_schreibt_nichts", wert_von(&u, &fall, CENT_FELD).is_none(), "ein leeres Feld hat einen Wert erzeugt: ununterscheidbar von einer echten 0");

    // „Spaeter“ schreibt nichts, legt keinen Merker an, und die Frage steht von selbst wieder im Fragebogen.
    frisch(&u, None);
    let fall = u.fall();
    stub(s, &[antwort(&[], &[rf("Wie hoch war dein Bruttoarbeitslohn?", CENT_FELD)], &[])]);
    pp!("H.spaeter_schreibt_nichts_und_die_frage_kommt_im_fragebogen_wieder", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    s.klick("#rf-spaeter");
    s.warte_unsichtbar("#rueckfragen", 5);
    pp!("H.spaeter_schreibt_nichts_und_die_frage_kommt_im_fragebogen_wieder", wert_von(&u, &fall, CENT_FELD).is_none(), "„Spaeter“ hat etwas geschrieben: das ist dann eine Antwort");
    pp!("H.spaeter_schreibt_nichts_und_die_frage_kommt_im_fragebogen_wieder", u.offene(&fall).contains(&CENT_FELD.to_owned()), "das Feld steht nicht mehr in den offenen Fragen");
    pp!("H.spaeter_schreibt_nichts_und_die_frage_kommt_im_fragebogen_wieder", s.try_warte_bis("(() => { const e = document.getElementById('wegpunkt'); return !!e && !e.hidden; })()", 5).is_some(), "der Fragebogen kommt nicht zurueck");

    // Teil 2b: eine Rueckfrage, die N Felder meint (zwei Kinder, `kind_vorname`, `kind_vorname__2`).
    let mit_kindern = || {
        frisch(&u, Some(&["kein_kind"]));
        let fall = u.fall();
        assert!(matches!(setze_kinderzahl(s, 2).as_i64(), Some(200 | 201)), "die Kinderzahl liess sich nicht setzen");
        assert!(u.offene(&fall).contains(&"kind_vorname".to_owned()), "kind_vorname steht nicht in den offenen Fragen: die Rueckfrage fiele weg");
        stub(s, &[antwort(&[], &[rf("Wie heißen deine Kinder mit Vornamen?", "kind_vorname")], &[])]);
        assert!(senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
        fall
    };
    let _ = mit_kindern();
    let m = rf_instanzen(s);
    pp!("H.zwei_kinder_geben_zwei_eingabefelder_in_der_nachfrage", m["zeilen"] == json!(2), "die Nachfrage bietet {} Instanz-Zeilen fuer zwei Kinder an: {m}", m["zeilen"]);
    pp!("H.zwei_kinder_geben_zwei_eingabefelder_in_der_nachfrage", m["marken"] == json!(["Kind 1", "Kind 2"]), "ohne Nummer weiss der Nutzer nicht, welches Kind gemeint ist: {}", m["marken"]);
    pp!("H.zwei_kinder_geben_zwei_eingabefelder_in_der_nachfrage", m["ids"].as_array().is_some_and(|a| a.iter().all(|i| i.as_str().is_some_and(|i| !i.is_empty()))), "ein Feld ohne id: der Schreibpfad findet es nicht: {}", m["ids"]);
    pp!("H.zwei_kinder_geben_zwei_eingabefelder_in_der_nachfrage", m["eingaben"] == json!(2), "nicht genau zwei Eingabefelder: {m}");

    let fall = mit_kindern();
    s.fuelle("#rf-input__1", "Anna");
    s.fuelle("#rf-input__2", "Ben");
    s.klick("#rf-weiter");
    s.warte_unsichtbar("#rueckfragen", 8);
    let f = u.stand_felder(&fall);
    pp!("H.zwei_kinder_geben_zwei_ereignisse_im_store", f["kind_vorname"]["wert"] == json!("Anna"), "Instanz 1 fehlt oder steht falsch: {}", f["kind_vorname"]);
    pp!("H.zwei_kinder_geben_zwei_ereignisse_im_store", f["kind_vorname__2"]["wert"] == json!("Ben"), "das zweite Kind hat keinen Vornamen: {f}");
    for fid in ["kind_vorname", "kind_vorname__2"] {
        pp!("H.zwei_kinder_geben_zwei_ereignisse_im_store", f[fid]["zustand"] == json!("bestaetigt") && f[fid]["herkunft_badge"] == json!("laie"), "{fid}: {}", f[fid]);
    }

    let fall = mit_kindern();
    s.fuelle("#rf-input__1", "Anna");
    s.klick("#rf-weiter");
    s.warte_unsichtbar("#rueckfragen", 8);
    let f = u.stand_felder(&fall);
    pp!("H.eine_leer_gelassene_instanz_schreibt_nichts", f["kind_vorname"]["wert"] == json!("Anna"), "die gefuellte Instanz ging mit der leeren verloren: {}", f["kind_vorname"]);
    pp!("H.eine_leer_gelassene_instanz_schreibt_nichts", f.get("kind_vorname__2").is_none(), "die leere Instanz wurde geschrieben: {}", f["kind_vorname__2"]);
    // BEWUSST festgeschrieben (Python, 2026-08-27): die uebersprungene Instanz kommt NICHT von selbst wieder.
    pp!("H.eine_leer_gelassene_instanz_schreibt_nichts", !u.offene(&fall).contains(&"kind_vorname__2".to_owned()), "die uebersprungene Instanz steht jetzt als eigene Frage in der Queue: diese Zusicherung umschreiben");

    let fall = mit_kindern();
    s.klick("#rf-spaeter");
    s.warte_unsichtbar("#rueckfragen", 8);
    let f = u.stand_felder(&fall);
    pp!("H.spaeter_beantworten_gilt_auch_auf_der_achse", f.as_object().is_some_and(|o| !o.keys().any(|k| k.starts_with("kind_vorname"))), "„Spaeter“ hat etwas geschrieben: {f}");
    pp!("H.spaeter_beantworten_gilt_auch_auf_der_achse", u.offene(&fall).contains(&"kind_vorname".to_owned()), "das Feld steht nicht mehr in den offenen Fragen");

    // Die Gegenrichtung: ein Feld OHNE Achse bleibt bei einem Feld und einem Ereignis, auch bei Kinderzahl 2.
    let fall = {
        frisch(&u, Some(&["kein_kind"]));
        let fall = u.fall();
        assert!(matches!(setze_kinderzahl(s, 2).as_i64(), Some(200 | 201)));
        stub(s, &[antwort(&[], &[rf("Wie hoch war dein Bruttoarbeitslohn?", CENT_FELD)], &[])]);
        assert!(senden(s, "ich habe bis juni gearbeitet", true));
        fall
    };
    let z = rf_instanzen(s);
    pp!("H.ein_feld_ohne_achse_bleibt_bei_einem_feld_und_einem_ereignis", z["zeilen"] == json!(0) && z["eingaben"] == json!(1), "ein Feld ohne Achse zeigt Instanz-Zeilen: {z}");
    s.fuelle("#rf-input", "62000");
    s.klick("#rf-weiter");
    s.warte_unsichtbar("#rueckfragen", 8);
    let f = u.stand_felder(&fall);
    pp!("H.ein_feld_ohne_achse_bleibt_bei_einem_feld_und_einem_ereignis", f[CENT_FELD]["wert"] == json!(6_200_000), "der Wert steht falsch im Store: {}", f[CENT_FELD]);
    pp!("H.ein_feld_ohne_achse_bleibt_bei_einem_feld_und_einem_ereignis", f.get(format!("{CENT_FELD}__2")).is_none(), "ein Feld ohne Achse hat eine zweite Instanz erzeugt");

    // Ohne `feld_id` gibt es keinen Typ und kein Eingabefeld: der Chat bleibt der Weg, der Knopf sagt es.
    frisch(&u, None);
    stub(s, &[antwort(&[], &[rf("Wie war das Jahr für dich insgesamt?", "")], &[])]);
    pp!("H.eine_rueckfrage_ohne_feld_bleibt_beim_chat", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    let z = rf_zustand(s);
    pp!("H.eine_rueckfrage_ohne_feld_bleibt_beim_chat", z["offen"] == json!(true) && z["eingaben"] == json!(0), "ohne Feld darf kein Eingabefeld dastehen: {z}");
    pp!("H.eine_rueckfrage_ohne_feld_bleibt_beim_chat", s.textinhalt("#rf-weiter").contains("Berater"), "der Knopf verspricht etwas anderes: {:?}", s.textinhalt("#rf-weiter"));
    s.klick("#rf-weiter");
    pp!("H.eine_rueckfrage_ohne_feld_bleibt_beim_chat", s.wert("#chat-text").starts_with("Zu deiner Rückfrage"), "der Knopf fuehrt nicht in den Chat: die Frage ist unbeantwortbar");

    // Teil 3: nie zwei Aufforderungen zugleich; erst die Bestaetigungen, dann die Rueckfragen.
    let beides = antwort(
        &[vorschlag("aussergewoehnliche_belastungen", &json!(62_000), "620 euro arztkosten")],
        &[rf("Ganzes Jahr oder anteilig?", CENT_FELD)],
        &[],
    );
    frisch(&u, None);
    stub(s, std::slice::from_ref(&beides));
    pp!("H.nie_zwei_aufforderungen_gleichzeitig", senden(s, "ich habe bis juni gearbeitet", false), "die Anzeige kommt nicht");
    let v = s.try_warte_bis("(() => { const e = document.getElementById('verstanden'); return !!e && !e.hidden; })()", 5);
    pp!("H.nie_zwei_aufforderungen_gleichzeitig", v.is_some(), "die Verstanden-Seite kommt nicht");
    pp!("H.nie_zwei_aufforderungen_gleichzeitig", !s.sichtbar("#rueckfragen"), "Bestaetigungen und Rueckfrage stehen gleichzeitig auf dem Schirm");
    pp!("H.nie_zwei_aufforderungen_gleichzeitig", !s.sichtbar("#wegpunkt"), "der Fragebogen steht daneben: wieder zwei Aufforderungen");
    pp!("H.nie_zwei_aufforderungen_gleichzeitig", s.textinhalt("#chat-body").contains("danach frage ich das Offene nach"), "die zurueckgestellte Rueckfrage wird nirgends erwaehnt");
    pp!("H.nie_zwei_aufforderungen_gleichzeitig", s.textinhalt("#verstanden-weiter").contains("Nachfragen"), "der Knopf verspricht den Fragebogen, fuehrt aber zu einer Nachfrage: {:?}", s.textinhalt("#verstanden-weiter"));

    frisch(&u, None);
    stub(s, &[beides]);
    pp!("H.nach_den_bestaetigungen_kommen_die_rueckfragen", senden(s, "ich habe bis juni gearbeitet", false), "die Anzeige kommt nicht");
    s.try_warte_bis("(() => { const e = document.getElementById('verstanden'); return !!e && !e.hidden; })()", 5);
    pp!("H.nach_den_bestaetigungen_kommen_die_rueckfragen", s.anzahl("#verstanden-liste .v-ok") > 0, "die Bestaetigung ist nicht anklickbar angekommen");
    s.klick("#verstanden-weiter");
    let rf_da = s.try_warte_bis("(() => { const e = document.getElementById('rueckfragen'); return !!e && !e.hidden; })()", 5);
    pp!("H.nach_den_bestaetigungen_kommen_die_rueckfragen", rf_da.is_some(), "nach den Bestaetigungen kommt keine Rueckfrage");
    pp!("H.nach_den_bestaetigungen_kommen_die_rueckfragen", !s.sichtbar("#verstanden") && s.textinhalt("#rf-frage").contains("anteilig"), "falsche oder keine Rueckfrage");
    let fehler = u.konsolenfehler();
    pp!("H.nach_den_bestaetigungen_kommen_die_rueckfragen", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Eine Rueckfrage zu einem Feld, das schon beantwortet ist, wird nicht gestellt, und der Verlauf sagt es.
    frisch(&u, Some(&["kein_kap"]));
    let fall = u.fall();
    pp!("H.eine_rueckfrage_zu_einem_erledigten_feld_wird_nicht_gestellt", u.offene(&fall).contains(&"kap_kapitalertraege".to_owned()), "Vorbedingung: kap_kapitalertraege muss offen sein");
    let (st, r) = u.dienst.schreibe(&fall, "kap_kapitalertraege", &json!(0));
    assert_eq!(st, 201, "{r}");
    pp!("H.eine_rueckfrage_zu_einem_erledigten_feld_wird_nicht_gestellt", !u.offene(&fall).contains(&"kap_kapitalertraege".to_owned()), "Vorbedingung 2: das beantwortete Feld steht immer noch in den offenen Fragen");
    s.auswerten("async () => { await refresh(); }", &[]);
    stub(s, &[antwort(&[], &[rf("Wie hoch waren deine Kapitalerträge?", "kap_kapitalertraege")], &[])]);
    pp!("H.eine_rueckfrage_zu_einem_erledigten_feld_wird_nicht_gestellt", senden(s, "ich hatte gar keine kapitalertraege", false), "chatSenden endet nicht");
    pp!("H.eine_rueckfrage_zu_einem_erledigten_feld_wird_nicht_gestellt", !s.sichtbar("#rueckfragen"), "die Nachfrage wird gestellt, obwohl das Feld gar nicht mehr offen ist");
    pp!("H.eine_rueckfrage_zu_einem_erledigten_feld_wird_nicht_gestellt", s.textinhalt("#chat-body").contains("erledigt"), "die weggefallene Nachfrage wird nirgends erwaehnt");

    // Ohne Bestaetigungen geht es nach „spaeter“ zurueck in den Fragebogen.
    frisch(&u, None);
    stub(s, &[antwort(&[], &[rf("Ganzes Jahr oder anteilig?", CENT_FELD)], &[])]);
    pp!("H.ohne_bestaetigungen_geht_es_zurueck_in_den_fragebogen", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    s.klick("#rf-spaeter");
    let wp = s.try_warte_bis("(() => { const e = document.getElementById('wegpunkt'); return !!e && !e.hidden; })()", 5);
    pp!("H.ohne_bestaetigungen_geht_es_zurueck_in_den_fragebogen", wp.is_some() && !s.sichtbar("#verstanden") && !s.sichtbar("#rueckfragen"), "der Fragebogen kommt nicht zurueck");
}

// ---- chat_aussagen: was die KI gelesen hat, steht mit Beleg da ------------------------------------------------------

const FRAGE: &str = "Meinst du 100.000 für das ganze Jahr oder anteilig bis Juni?";

fn aussage(text: &str, beleg: &str, status: &str) -> Value {
    json!({"text": text, "beleg": beleg, "status": status})
}

/// Die Antwort mit einer Rueckfrage zu `bruttoarbeitslohn` (`_rueckfrage_antwort`), optional mit Vorschlaegen.
fn rueckfrage_antwort(vorschlaege: &[Value]) -> Value {
    antwort(
        vorschlaege,
        &[json!({"frage": FRAGE, "aussage": 0, "feld_id": "bruttoarbeitslohn"})],
        &[aussage("Jahresbrutto 100.000 EUR", "bis Juni 100k p.a.", "rueckfrage")],
    )
}

fn zeilen(s: &crate::browser::Sitzung) -> Vec<Value> {
    s.ausdruck(
        "[...document.querySelectorAll('#chat-body .a-zeile')].map(li => ({status: li.dataset.status, klasse: li.className, \
         text: (li.querySelector('.a-text') || {}).textContent || '', marke: (li.querySelector('.a-marke') || {}).textContent || '', \
         beleg: (li.querySelector('.a-beleg') || {}).textContent || '', frage: (li.querySelector('.a-frage') || {}).textContent || ''}))",
    )
    .as_array()
    .cloned()
    .unwrap_or_default()
}

/// Wie oft steht dieselbe Frage GLEICHZEITIG als sichtbarer Text auf dem Schirm (nur der eigene Text, nicht der der Kinder).
fn wie_oft_steht_die_frage_da(s: &crate::browser::Sitzung, frage: &str) -> i64 {
    s.auswerten(
        "(frage) => { const sichtbar = e => { for (let n = e; n && n !== document.body; n = n.parentElement) { \
           if (n.hidden || getComputedStyle(n).display === 'none') return false; } return true; }; \
         let n = 0; for (const e of document.querySelectorAll('body *')) { \
           const eigen = [...e.childNodes].filter(k => k.nodeType === 3).map(k => k.textContent).join(''); \
           if (eigen.includes(frage) && sichtbar(e)) n++; } return n; }",
        &[json!(frage)],
    )
    .as_i64()
    .unwrap_or(-1)
}

fn sperren(s: &crate::browser::Sitzung) -> Value {
    s.ausdruck(
        "({wegpunkt: document.getElementById('wegpunkt').hasAttribute('inert'), verstanden: document.getElementById('verstanden').hasAttribute('inert'), \
          rueckfragen: document.getElementById('rueckfragen').hasAttribute('inert'), belegt: document.getElementById('belegt-liste').hasAttribute('inert'), \
          importbox: document.querySelector('.vorjahr-box').hasAttribute('inert'), chattext: document.getElementById('chat-text').disabled, \
          sendknopf: document.getElementById('chat-send').disabled})",
    )
}

#[test]
fn fluss_h_chat_aussagen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 780);
    let s = &u.s;
    let senden_frei = |text: &str| senden(s, text, false);

    // Punkt 1: jede Aussage mit dem woertlichen Satzteil, auf den sie sich stuetzt.
    frisch(&u, None);
    stub(
        s,
        &[antwort(
            &[json!({"feld_id": "aussergewoehnliche_belastungen", "wert": 62_000, "beleg": "620 euro arztkosten", "aussage": 1})],
            &[],
            &[aussage("Nutzer ist ledig", "ich bin ledig", "vorschlag"), aussage("Arztkosten 620 EUR", "620 euro arztkosten", "vorschlag")],
        )],
    );
    pp!("H.aussagen_erscheinen_mit_ihrem_beleg", senden_frei("ich bin ledig und hatte 620 euro arztkosten"), "chatSenden endet nicht");
    let z = zeilen(s);
    pp!("H.aussagen_erscheinen_mit_ihrem_beleg", z.len() == 2, "nicht beide Aussagen angezeigt: {z:?}");
    pp!("H.aussagen_erscheinen_mit_ihrem_beleg", z[0]["text"] == json!("Nutzer ist ledig") && z[1]["text"] == json!("Arztkosten 620 EUR"), "{z:?}");
    pp!("H.aussagen_erscheinen_mit_ihrem_beleg", z.iter().all(|x| x["beleg"].as_str().unwrap_or_default().contains("weil du sagtest")), "Aussage ohne Beleg-Zitat: eine Behauptung der KI, die der Nutzer nur glauben kann: {z:?}");
    pp!(
        "H.aussagen_erscheinen_mit_ihrem_beleg",
        z[0]["beleg"].as_str().unwrap_or_default().contains("ich bin ledig") && z[1]["beleg"].as_str().unwrap_or_default().contains("620 euro arztkosten"),
        "der Beleg gehoert zur falschen Aussage: {z:?}"
    );
    let fehler = u.konsolenfehler();
    pp!("H.aussagen_erscheinen_mit_ihrem_beleg", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Die Aussagen stehen VOR der Antwort (erst was gelesen wurde, dann was daraus folgt).
    frisch(&u, None);
    let mut a = antwort(&[], &[], &[aussage("Arztkosten 620 EUR", "620 euro", "kein_feld")]);
    a["antwort"] = json!("Arztkosten zählen zu den außergewöhnlichen Belastungen.");
    stub(s, &[a]);
    senden_frei("ich bin ledig und hatte 620 euro arztkosten");
    let vor = s.ausdruck(
        "(() => { const box = document.querySelector('#chat-body .chat-aussagen'); const ant = document.querySelector('#chat-body .chat-antwort'); \
          return (box && ant) ? (box.compareDocumentPosition(ant) & Node.DOCUMENT_POSITION_FOLLOWING) > 0 : null; })()",
    );
    pp!("H.aussagen_stehen_vor_der_antwort", vor == json!(true), "die gelesenen Aussagen stehen nicht vor der Antwort: {vor}");

    // Punkt 2: eine Aussage ohne Feld ist als solche sichtbar (Rahmenform UND Farbe, nicht nur die Klasse).
    frisch(&u, None);
    stub(
        s,
        &[antwort(
            &[json!({"feld_id": "veranlagungsart", "wert": "einzel", "typ": "enum", "beleg": "ich bin ledig", "aussage": 0})],
            &[],
            &[aussage("Nutzer ist ledig", "ich bin ledig", "vorschlag"), aussage("Seit Juli arbeitslos", "seit juli arbeitslos", "kein_feld")],
        )],
    );
    senden_frei("ich bin ledig und seit juli arbeitslos");
    let z = zeilen(s);
    pp!("H.eine_aussage_ohne_feld_ist_sichtbar_als_solche", z.len() == 2, "die nicht zugeordnete Aussage fehlt ganz: {z:?}");
    let ohne: Vec<&Value> = z.iter().filter(|x| x["status"] == json!("kein_feld")).collect();
    pp!("H.eine_aussage_ohne_feld_ist_sichtbar_als_solche", ohne.len() == 1 && ohne[0]["text"] == json!("Seit Juli arbeitslos"), "kein_feld-Zeile nicht als solche ausgezeichnet: {z:?}");
    pp!("H.eine_aussage_ohne_feld_ist_sichtbar_als_solche", ohne[0]["marke"].as_str().is_some_and(|m| !m.is_empty()) && ohne[0]["marke"] != z[0]["marke"], "die Marke unterscheidet die Zeilen nicht: {z:?}");
    let stile = s.ausdruck(
        "[...document.querySelectorAll('#chat-body .a-zeile')].map(li => { const c = getComputedStyle(li); \
         return {status: li.dataset.status, art: c.borderLeftStyle, farbe: c.borderLeftColor}; })",
    );
    let stil = |status: &str| stile.as_array().and_then(|a| a.iter().find(|x| x["status"] == json!(status)).cloned()).unwrap_or(Value::Null);
    let (ohne_stil, mit_stil) = (stil("kein_feld"), stil("vorschlag"));
    pp!("H.eine_aussage_ohne_feld_ist_sichtbar_als_solche", ohne_stil["art"] == json!("dashed") && mit_stil["art"] == json!("solid"), "die Rahmenform unterscheidet die beiden nicht ({ohne_stil} gegen {mit_stil}): Farbe allein traegt die Unterscheidung nicht");
    pp!("H.eine_aussage_ohne_feld_ist_sichtbar_als_solche", ohne_stil["farbe"] != mit_stil["farbe"], "auch die Farbe ist gleich: {stile}");
    pp!("H.eine_aussage_ohne_feld_ist_sichtbar_als_solche", s.textinhalt("#chat-body .a-kein_feld .a-hinweis").contains("kein passendes Feld"), "kein Hinweis, was der Nutzer jetzt tun kann");

    // Punkt 3: die Rueckfrage steht nur an EINEM Ort, die Aussage dazu bleibt im Verlauf.
    frisch(&u, None);
    stub(s, &[rueckfrage_antwort(&[])]);
    senden_frei("bis Juni 100k p.a.");
    let n = wie_oft_steht_die_frage_da(s, FRAGE);
    pp!("H.die_rueckfrage_steht_nur_an_EINEM_ort", n == 1, "die Frage steht {n}x gleichzeitig auf dem Schirm: genau eine Aufforderung, genau ein Antwortweg");
    pp!("H.die_rueckfrage_steht_nur_an_EINEM_ort", s.anzahl("#chat-body .a-antworten") == 0, "im Verlauf steht wieder ein eigener „Antworten“-Knopf neben dem Wizard");
    pp!("H.die_rueckfrage_steht_nur_an_EINEM_ort", s.textinhalt("#rf-frage").contains(FRAGE), "und der eine Ort ist die Rueckfragen-Seite: dort steht sie nicht");
    let z = zeilen(s);
    pp!("H.die_aussage_zur_rueckfrage_bleibt_im_verlauf", z.len() == 1 && z[0]["status"] == json!("rueckfrage"), "genau eine Aussage erwartet: {z:?}");
    pp!("H.die_aussage_zur_rueckfrage_bleibt_im_verlauf", z[0]["beleg"].as_str().unwrap_or_default().contains("bis Juni 100k p.a."), "auch die Rueckfrage-Aussage braucht ihren Beleg");
    pp!("H.die_aussage_zur_rueckfrage_bleibt_im_verlauf", z[0]["marke"].as_str().is_some_and(|m| !m.is_empty()), "die Aussage traegt keine Marke: der Bezug zur Frage fehlt");
    let drin = format!("{}{}", z[0]["frage"].as_str().unwrap_or_default(), z[0]["text"].as_str().unwrap_or_default());
    pp!("H.die_aussage_zur_rueckfrage_bleibt_im_verlauf", !drin.contains(FRAGE), "die Frage selbst steht wieder in der Verlaufszeile: {z:?}");

    // Die Rueckfrage ist beantwortbar, und danach steht keine Sperre mehr.
    frisch(&u, None);
    stub(s, &[rueckfrage_antwort(&[])]);
    senden(s, "bis Juni 100k p.a.", true);
    pp!("H.rueckfrage_ist_beantwortbar_und_der_ablauf_laeuft_weiter", s.finde("#rf-input").is_some(), "eine Rueckfrage ohne Eingabefeld ist keine Rueckfrage");
    s.fuelle("#rf-input", "50000");
    s.klick("#rf-weiter");
    s.warte_unsichtbar("#rueckfragen", 5);
    let g = sperren(s);
    pp!("H.rueckfrage_ist_beantwortbar_und_der_ablauf_laeuft_weiter", g.as_object().is_some_and(|o| o.values().all(|v| *v == json!(false))), "nach der Rueckfrage-Runde sind noch Sperren gesetzt: {g}");
    let fehler = u.konsolenfehler();
    pp!("H.rueckfrage_ist_beantwortbar_und_der_ablauf_laeuft_weiter", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Ohne `feld_id` fuehrt der Wizard in den Chat, ohne Getipptes zu verlieren.
    frisch(&u, None);
    stub(s, &[antwort(&[], &[json!({"frage": "Wie war das Jahr für dich insgesamt?", "aussage": 0, "feld_id": ""})], &[aussage("Unklare Angabe", "bis Juni", "rueckfrage")])]);
    senden(s, "bis Juni 100k p.a.", true);
    pp!("H.ohne_feld_fuehrt_der_wizard_in_den_chat_ohne_getipptes_zu_verlieren", s.finde(".rf-ohne-feld").is_some(), "ohne Feld muss der Wizard sagen, dass hier im Berater geantwortet wird");
    s.fuelle("#chat-text", "ausserdem habe ich eine Brille gekauft");
    s.klick("#rf-weiter");
    let wert = s.wert("#chat-text");
    pp!("H.ohne_feld_fuehrt_der_wizard_in_den_chat_ohne_getipptes_zu_verlieren", wert.contains("Brille"), "getippter Text ueberschrieben: {wert:?}");
    pp!("H.ohne_feld_fuehrt_der_wizard_in_den_chat_ohne_getipptes_zu_verlieren", wert.contains("Zu deiner Rückfrage"), "Vorspann fehlt: {wert:?}");
    pp!("H.ohne_feld_fuehrt_der_wizard_in_den_chat_ohne_getipptes_zu_verlieren", wert.contains("Wie war das Jahr"), "der Bezug fehlt im abgeschickten Text: {wert:?}");

    // Eine Rueckfrage, deren Aussage-Index ins Leere zeigt, verschwindet nicht und steht nur einmal da.
    frisch(&u, None);
    stub(s, &[antwort(&[], &[json!({"frage": "Ganzes Jahr oder anteilig?", "aussage": 7, "feld_id": "bruttoarbeitslohn"})], &[aussage("Nutzer ist ledig", "ich bin ledig", "kein_feld")])]);
    senden(s, "ich bin ledig und hatte 620 euro arztkosten", true);
    pp!("H.eine_rueckfrage_ohne_zuordnung_verschwindet_nicht", s.textinhalt("#rf-frage").contains("Ganzes Jahr oder anteilig?"), "die heimatlose Rueckfrage ist verschwunden");
    pp!("H.eine_rueckfrage_ohne_zuordnung_verschwindet_nicht", wie_oft_steht_die_frage_da(s, "Ganzes Jahr oder anteilig?") == 1, "sie steht mehrfach da");

    // Rueckfrage UND Vorschlag am selben Feld: die Rueckfrage gewinnt, der Wert wird zurueckgehalten (am 2026-08-21
    // kostete das fast 70.000 EUR zu viel Bruttolohn).
    frisch(&u, None);
    stub(s, &[rueckfrage_antwort(&[json!({"feld_id": "bruttoarbeitslohn", "wert": 10_000_000, "beleg": "100k p.a.", "aussage": 0})])]);
    senden_frei("bis Juni 100k p.a.");
    pp!("H.rueckfrage_und_vorschlag_am_selben_feld_haelt_den_wert_zurueck", !s.sichtbar("#verstanden"), "der Vorschlag wurde trotz offener Rueckfrage zur Bestaetigung angeboten: ein Klick schriebe den Wert, nach dem die KI noch fragt");
    pp!("H.rueckfrage_und_vorschlag_am_selben_feld_haelt_den_wert_zurueck", s.try_warte_bis("(() => { const e = document.getElementById('rueckfragen'); return !!e && !e.hidden; })()", 5).is_some(), "die Rueckfrage fehlt");
    pp!("H.rueckfrage_und_vorschlag_am_selben_feld_haelt_den_wert_zurueck", s.textinhalt("#chat-body").contains("zurückgehalten"), "der Wert verschwand still: derselbe Fehler in die andere Richtung");

    // Punkt 4: ohne die neuen Felder verhaelt sich alles wie vorher.
    frisch(&u, None);
    let mut alt = antwort(&[json!({"feld_id": "aussergewoehnliche_belastungen", "wert": 62_000, "beleg": "620 euro"})], &[], &[]);
    alt["antwort"] = json!("Das kommt in Zeile 61 der Anlage.");
    alt.as_object_mut().unwrap().remove("aussagen");
    alt.as_object_mut().unwrap().remove("rueckfragen");
    stub(s, &[alt]);
    senden_frei("ich bin ledig und hatte 620 euro arztkosten");
    pp!("H.ohne_aussagen_verhaelt_sich_alles_wie_heute", s.anzahl("#chat-body .chat-aussagen") == 0, "ohne `aussagen` steht trotzdem ein (leerer) Aussagen-Kasten da");
    pp!("H.ohne_aussagen_verhaelt_sich_alles_wie_heute", s.textinhalt("#chat-body .chat-antwort").contains("Das kommt in Zeile 61"), "die Antwort fehlt");
    pp!("H.ohne_aussagen_verhaelt_sich_alles_wie_heute", s.try_warte_bis("(() => { const e = document.getElementById('verstanden'); return !!e && !e.hidden; })()", 5).is_some() && s.anzahl("#verstanden-liste .v-ok") > 0, "der Vorschlag kam nicht mehr zur Bestaetigung durch");
    let fehler = u.konsolenfehler();
    pp!("H.ohne_aussagen_verhaelt_sich_alles_wie_heute", fehler.is_empty(), "Konsolenfehler ohne die neuen Felder: {fehler:?}");

    frisch(&u, None);
    let mut leer = antwort(&[], &[], &[]);
    leer.as_object_mut().unwrap().remove("aussagen");
    leer.as_object_mut().unwrap().remove("rueckfragen");
    stub(s, &[leer]);
    senden_frei("ich bin ledig und hatte 620 euro arztkosten");
    pp!("H.ohne_aussagen_bleibt_die_leermeldung_stehen", s.textinhalt("#chat-body").contains("weder einen Wert"), "die Leermeldung ist mit der neuen Bedingung ganz verschwunden");

    frisch(&u, None);
    stub(s, &[rueckfrage_antwort(&[])]);
    senden_frei("bis Juni 100k p.a.");
    pp!("H.bei_offener_rueckfrage_keine_leermeldung", !s.textinhalt("#chat-body").contains("weder einen Wert"), "neben der Rueckfrage steht, die KI habe keine Frage erkannt");

    // Der abgeschickte Satz steht WAEHREND des Aufrufs nur einmal da (im Verlauf, nicht noch im Feld).
    frisch(&u, None);
    halte_an(s);
    let satz = "ich bin verheiratet und habe zwei kinder";
    s.fuelle("#chat-text", satz);
    s.klick("#chat-send");
    s.try_warte_bis("document.getElementById('chat-send').disabled", 5);
    let z = s.auswerten(
        "(satz) => ({im_feld: document.getElementById('chat-text').value, \
          im_verlauf: [...document.querySelectorAll('#chat-body > *')].filter(e => (e.textContent || '').includes(satz)).length, \
          warte_text: (document.querySelector('.chat-warte') || {}).textContent || ''})",
        &[json!(satz)],
    );
    pp!("H.der_abgeschickte_satz_steht_waehrend_des_aufrufs_nur_einmal", z["im_verlauf"] == json!(1), "der Satz steht {}x im Verlauf", z["im_verlauf"]);
    pp!("H.der_abgeschickte_satz_steht_waehrend_des_aufrufs_nur_einmal", z["im_feld"] == json!(""), "der Satz steht waehrend des ganzen Aufrufs noch im Eingabefeld: {}", z["im_feld"]);
    pp!("H.der_abgeschickte_satz_steht_waehrend_des_aufrufs_nur_einmal", !z["warte_text"].as_str().unwrap_or_default().contains("liest mit"), "„Die KI liest mit“ klingt nach jemandem, der ueber die Schulter schaut: {}", z["warte_text"]);
    let mut a = antwort(&[], &[], &[]);
    a["antwort"] = json!("Verstanden.");
    gib_frei(s, 200, &a);
    s.try_warte_bis("!document.getElementById('chat-send').disabled", 5);
    pp!("H.der_abgeschickte_satz_steht_waehrend_des_aufrufs_nur_einmal", s.wert("#chat-text").is_empty(), "nach dem Aufruf steht wieder etwas im Feld");

    // Bei einem Fehlschlag kommt der Satz zurueck ins Feld (Nutzerarbeit geht nicht verloren).
    frisch(&u, None);
    halte_an(s);
    let satz = "ich hatte 620 euro arztkosten";
    s.fuelle("#chat-text", satz);
    s.klick("#chat-send");
    s.try_warte_bis("document.getElementById('chat-send').disabled", 5);
    gib_frei(s, 500, &json!({"fehler": "kaputt"}));
    s.try_warte_bis("!document.getElementById('chat-send').disabled", 5);
    pp!("H.bei_einem_fehlschlag_kommt_der_satz_zurueck", s.wert("#chat-text") == satz, "nach dem Fehlschlag ist der Satz weg: {:?}", s.wert("#chat-text"));

    // Der Wizard ist waehrend eines laufenden Aufrufs gesperrt, der Berater nicht; danach ist die Sperre weg.
    frisch(&u, None);
    halte_an(s);
    s.fuelle("#chat-text", "bis Juni 100k p.a.");
    s.klick("#chat-send");
    s.try_warte_bis("document.getElementById('chat-send').disabled", 5);
    gib_frei(s, 200, &rueckfrage_antwort(&[]));
    s.try_warte_bis("!document.getElementById('chat-send').disabled", 5);
    s.try_warte_bis("!document.getElementById('rueckfragen').hidden", 5);
    halte_an(s);
    s.fuelle("#chat-text", "und eine Brille");
    s.klick("#chat-send");
    s.try_warte_bis("document.getElementById('chat-send').disabled", 5);
    let z = s.ausdruck(
        "({wizard_gesperrt: document.getElementById('rueckfragen').hasAttribute('inert'), wizard_sichtbar: !document.getElementById('rueckfragen').hidden, \
          berater_gesperrt: !!document.getElementById('berater').closest('[inert]'), verlauf_lesbar: !document.getElementById('chat-body').closest('[inert]')})",
    );
    pp!("H.der_wizard_ist_waehrend_eines_laufenden_aufrufs_gesperrt", z["wizard_sichtbar"] == json!(true), "Vorbedingung: der Wizard muss stehen: {z}");
    pp!("H.der_wizard_ist_waehrend_eines_laufenden_aufrufs_gesperrt", z["wizard_gesperrt"] == json!(true), "der Wizard nimmt waehrend des laufenden Aufrufs Klicks an: {z}");
    pp!("H.der_wizard_ist_waehrend_eines_laufenden_aufrufs_gesperrt", z["berater_gesperrt"] == json!(false) && z["verlauf_lesbar"] == json!(true), "mit dem Wizard wurde der ganze Berater gesperrt: {z}");
    let mut a = antwort(&[], &[], &[]);
    a["antwort"] = json!("Verstanden.");
    gib_frei(s, 200, &a);
    s.try_warte_bis("!document.getElementById('chat-send').disabled", 5);
    pp!("H.der_wizard_ist_waehrend_eines_laufenden_aufrufs_gesperrt", s.wert("#chat-text").is_empty(), "das Feld wurde nach dem Aufruf nicht geleert");
    pp!("H.der_wizard_ist_waehrend_eines_laufenden_aufrufs_gesperrt", s.ausdruck("document.getElementById('rueckfragen').hasAttribute('inert')") == json!(false), "die Sperre blieb nach dem Aufruf stehen");

    // Die Aussagen-Anzeige liegt im Berater-Panel, nicht in einem Bereich, den der laufende Aufruf sperrt.
    frisch(&u, None);
    stub(s, &[rueckfrage_antwort(&[])]);
    senden_frei("bis Juni 100k p.a.");
    let drin = s.ausdruck(
        "(() => { const box = document.querySelector('#chat-body .chat-aussagen'); if (!box) return 'kein Kasten'; \
          for (const id of ['wegpunkt', 'verstanden', 'belegt-liste']) { if (document.getElementById(id).contains(box)) return id; } \
          if (document.querySelector('.vorjahr-box').contains(box)) return 'vorjahr-box'; \
          return document.getElementById('berater').contains(box) ? 'berater' : 'sonstwo'; })()",
    );
    pp!("H.die_chat_sperre_greift_auch_mit_der_neuen_anzeige", drin == json!("berater"), "die Aussagen-Anzeige liegt in {drin} statt im Berater-Panel");
    pp!("H.die_chat_sperre_greift_auch_mit_der_neuen_anzeige", s.ausdruck("!document.getElementById('berater').closest('[inert]')") == json!(true), "das Berater-Panel liegt in einem gesperrten Bereich");

    // 360 px: lange Modelltexte (ein unteilbares Wort) schieben die Anzeige nicht seitwaerts, auch nicht IM Kasten.
    s.sichtflaeche(360, 640);
    frisch(&u, None);
    let lang = "Der Nutzer erzielte Einkuenfte aus nichtselbstaendiger Arbeit, gebucht auf DE00500105175407324931unterkontoLohnUndGehaltJanuarBisJuni2025Sammelbuchung";
    stub(s, &[antwort(&[], &[json!({"frage": lang, "aussage": 0, "feld_id": "bruttoarbeitslohn"})], &[aussage(lang, lang, "kein_feld")])]);
    senden_frei("ich bin ledig und hatte 620 euro arztkosten");
    s.try_warte_bis("!document.getElementById('rueckfragen').hidden", 5);
    pp!("H.die_anzeige_passt_auf_den_schmalen_schirm", s.ausdruck("document.documentElement.scrollWidth").as_i64().unwrap_or(-1) <= 360, "horizontales Scrollen bei 360 px durch die neue Anzeige");
    let ueberlauf = s.ausdruck(
        "['#chat-body', '#chat-body .chat-aussagen', '#chat-body .a-zeile', '#rueckfragen', '#rf-frage'].map(q => { const e = document.querySelector(q); \
         return {sel: q, ueber: e ? e.scrollWidth - e.clientWidth : null}; }).filter(x => x.ueber === null || x.ueber > 0)",
    );
    pp!("H.die_anzeige_passt_auf_den_schmalen_schirm", ueberlauf == json!([]), "der Text laeuft aus seinem Kasten heraus (null heisst: das Element war gar nicht da): {ueberlauf}");
    let (x, _, b, _) = s.box_von("#chat-body .chat-aussagen");
    pp!("H.die_anzeige_passt_auf_den_schmalen_schirm", x >= 0.0 && x + b <= 360.0, "der Aussagen-Kasten ragt aus dem Bild: x={x} breite={b}");
    for knopf in ["#rf-weiter", "#rf-spaeter"] {
        let (_, _, _, h) = s.box_von(knopf);
        pp!("H.die_anzeige_passt_auf_den_schmalen_schirm", h >= 44.0, "Tap-Ziel {knopf} unter 44 px: {h}");
    }
}

// ---- verstanden_seite: „Das habe ich verstanden“, jeder KI-Vorschlag mit seinem Zitat, einzeln zu bestaetigen --------
//
// Hier laeuft der ECHTE `/chat` des Dienstes gegen einen lokalen Ersatz fuer das Sprachmodell (`ModellErsatz`): geprueft
// wird alles NACH dem Modellaufruf (Anzeige, Bestaetigung, Schreibpfad, Katalog-Check, Auflage A/B). Es fliesst kein Cent.

const VERSTANDEN_TEXT: &str = "Ich bin verheiratet, fahre an 220 Tagen zur Arbeit, habe 62000 Euro brutto verdient und hatte keine Kapitalerträge.";
const KONFLIKT_TEXT: &str = "Ich verdiene 70000 Euro brutto.";

/// Was das Modell aus `VERSTANDEN_TEXT` gemacht haette: enum, Zahl ohne Einheit, cent und das invertierte bool.
fn verstanden_vorschlaege() -> Vec<Value> {
    vec![
        json!({"feld_id": "veranlagung", "wert": "zusammen", "beleg": "verheiratet", "begruendung": "Ehe → Zusammenveranlagung möglich"}),
        json!({"feld_id": "ep_arbeitstage", "wert": 220, "beleg": "an 220 Tagen", "begruendung": "Arbeitstage direkt genannt"}),
        json!({"feld_id": "bruttoarbeitslohn", "wert": 6_200_000, "beleg": "62000 Euro brutto", "begruendung": "Bruttolohn in Euro, als Cent übernommen"}),
        json!({"feld_id": "kein_kap", "wert": true, "beleg": "keine Kapitalerträge", "begruendung": "Abwesenheit ausdrücklich genannt"}),
    ]
}

fn konflikt_vorschlaege() -> Vec<Value> {
    vec![json!({"feld_id": "bruttoarbeitslohn", "wert": 7_000_000, "beleg": "70000 Euro brutto", "begruendung": "genannt"})]
}

fn verstanden_offen(s: &crate::browser::Sitzung) -> bool {
    s.try_warte_bis("(() => { const e = document.getElementById('verstanden'); return !!e && !e.hidden; })()", 8).is_some()
}

#[test]
fn fluss_h_verstanden_seite() {
    let _sperre = crate::browser::seriell();
    let modell = ModellErsatz::starte(json!({}));
    let u = umgebung_oder_ende!(Start { llm_basis: Some(modell.basis.clone()), ..Start::default() }, 360, 780);
    let (s, d) = (&u.s, &u.dienst);
    let feld_ids: Vec<String> = verstanden_vorschlaege().iter().map(|v| v["feld_id"].as_str().unwrap_or_default().to_owned()).collect();

    // Der Dienst (ohne Browser): die Anzeige-Metadaten stehen in der Antwort, die Vorschlaege bleiben vorlaeufig.
    modell.setze_vorschlaege(&verstanden_vorschlaege());
    d.fall("gesamt", "v1");
    let (st, body) = d.post("/fall/v1/chat", &json!({"text": VERSTANDEN_TEXT}));
    pp!("H.chat_liefert_anzeige_metadaten", st == 200, "{st} {body}");
    let nach_feld = |f: &str| body["vorschlaege"].as_array().and_then(|a| a.iter().find(|v| v["feld_id"] == json!(f)).cloned()).unwrap_or(Value::Null);
    pp!("H.chat_liefert_anzeige_metadaten", body["vorschlaege"].as_array().is_some_and(|a| a.len() == 4), "nicht alle Vorschlaege wurden geschrieben: {}", body["vorschlaege"]);
    let lohn = nach_feld("bruttoarbeitslohn");
    pp!("H.chat_liefert_anzeige_metadaten", lohn["typ"] == json!("cent"), "ohne typ weiss die Oberflaeche nicht, dass 6200000 Cent sind: {lohn}");
    pp!("H.chat_liefert_anzeige_metadaten", lohn["frage"].as_str().is_some_and(|f| !f.is_empty()), "fragetext_laie fehlt: die Zeile hätte nur die feld_id als Ueberschrift");
    pp!("H.chat_liefert_anzeige_metadaten", lohn["beleg"] == json!("62000 Euro brutto"), "{lohn}");
    pp!("H.chat_liefert_anzeige_metadaten", lohn["event_id"].as_str().is_some_and(|f| !f.is_empty()), "ohne event_id kann die Seite den Vorschlag nicht ersetzen (Auflage B)");
    pp!("H.chat_liefert_anzeige_metadaten", nach_feld("veranlagung")["enum_labels"]["zusammen"].as_str().is_some_and(|f| !f.is_empty()), "enum_labels fehlen: der Nutzer laese den Rohwert 'zusammen': {}", nach_feld("veranlagung"));
    for f in &feld_ids {
        let ev = d.ereignisse("v1", f);
        let letztes = ev.last().cloned().unwrap_or(Value::Null);
        pp!("H.vorschlaege_bleiben_vorlaeufig", letztes["zustand"] == json!("vorlaeufig"), "{f} ist nicht vorlaeufig: {letztes}");
        pp!("H.vorschlaege_bleiben_vorlaeufig", letztes["signal"]["signal_2"].is_null(), "{f} kam mit zweitem Signal durch: {letztes}");
    }

    // Konflikt: ein belegtes Feld wird nicht ueberschrieben; die Antwort traegt beide Werte samt Anzeige-Metadaten.
    modell.setze_vorschlaege(&konflikt_vorschlaege());
    d.fall("gesamt", "k1");
    let (st, r) = d.schreibe("k1", "bruttoarbeitslohn", &json!(6_200_000));
    assert_eq!(st, 201, "{r}");
    let (st, body) = d.post("/fall/k1/chat", &json!({"text": KONFLIKT_TEXT}));
    pp!("H.konflikt_traegt_anzeige_metadaten", st == 200, "{st} {body}");
    pp!("H.konflikt_traegt_anzeige_metadaten", body["vorschlaege"] == json!([]), "ein belegtes Feld darf nicht ueberschrieben werden: {}", body["vorschlaege"]);
    let k = body["konflikte"].as_array().and_then(|a| a.first().cloned()).unwrap_or(Value::Null);
    pp!("H.konflikt_traegt_anzeige_metadaten", body["konflikte"].as_array().is_some_and(|a| a.len() == 1), "{}", body["konflikte"]);
    pp!("H.konflikt_traegt_anzeige_metadaten", k["aktueller_wert"] == json!(6_200_000) && k["vorschlag_wert"] == json!(7_000_000), "{k}");
    pp!("H.konflikt_traegt_anzeige_metadaten", k["typ"] == json!("cent"), "ohne typ zeigt die Oberflaeche zwei Cent-Rohwerte: {k}");
    pp!("H.konflikt_traegt_anzeige_metadaten", k["frage"].as_str().is_some_and(|f| !f.is_empty()), "ohne Fragetext weiss der Nutzer nicht, worum gestritten wird");
    pp!("H.konflikt_traegt_anzeige_metadaten", k["aktuelles_event_id"].as_str().is_some_and(|f| !f.is_empty()), "ohne event_id kann die Uebernahme nichts ersetzen");

    // Browser: Fall angelegt, Chat abgeschickt, Verstanden-Seite offen.
    let seite = || {
        modell.setze_vorschlaege(&verstanden_vorschlaege());
        frisch(&u, None);
        s.fuelle("#chat-text", VERSTANDEN_TEXT);
        s.klick("#chat-send");
        assert!(verstanden_offen(s), "die Verstanden-Seite kommt nicht");
        u.fall()
    };
    let stand = |fall: &str| u.stand_felder(fall);

    seite();
    let felder = s.ausdruck("[...document.querySelectorAll('.v-zeile')].map(e => e.dataset.feld)");
    let mut felder: Vec<String> = felder.as_array().map(|a| a.iter().filter_map(|f| f.as_str().map(str::to_owned)).collect()).unwrap_or_default();
    felder.sort();
    let mut soll = feld_ids.clone();
    soll.sort();
    pp!("H.seite_zeigt_jede_zeile_mit_ihrem_zitat", felder == soll, "Zeilen: {felder:?}");
    let belege = s.ausdruck("[...document.querySelectorAll('.v-beleg')].map(e => e.textContent)");
    pp!("H.seite_zeigt_jede_zeile_mit_ihrem_zitat", belege.as_array().is_some_and(|b| b.len() == 4), "nicht jede Zeile traegt ihren Beleg: {belege}");
    pp!("H.seite_zeigt_jede_zeile_mit_ihrem_zitat", belege.as_array().is_some_and(|b| b.iter().any(|t| t.as_str().is_some_and(|t| t.contains("62000 Euro brutto")))), "Zitat fehlt: {belege}");
    let wert = s.textinhalt(".v-zeile[data-feld='bruttoarbeitslohn'] .v-wert");
    pp!("H.cent_wird_als_euro_gezeigt", wert.contains("62.000,00"), "Cent-Rohwert steht auf der Seite: {wert:?}");
    let wert = s.textinhalt(".v-zeile[data-feld='veranlagung'] .v-wert");
    pp!("H.enum_zeigt_den_anzeigetext", wert.contains("Zusammenveranlagung"), "Rohwert statt Anzeigetext: {wert:?}");
    let frage = s.textinhalt(".v-zeile[data-feld='kein_kap'] .v-frage");
    let wert = s.textinhalt(".v-zeile[data-feld='kein_kap'] .v-wert");
    pp!("H.kein_feld_wird_invertiert_angezeigt", frage.contains("Kapitalerträge"), "unerwarteter Fragetext: {frage:?}");
    pp!("H.kein_feld_wird_invertiert_angezeigt", wert.trim() == "Nein", "'keine Kapitalertraege' steht als {wert:?} unter der Frage {frage:?}: die kein_-Inversion fehlt, die Seite sagt das Gegenteil des gespeicherten Werts");
    let breite = s.ausdruck("document.documentElement.scrollWidth").as_i64().unwrap_or(-1);
    pp!("H.keine_horizontale_scrollbar_bei_360px", breite <= 360, "scrollWidth={breite} > 360: die Seite scrollt seitlich");

    // Ein Klick auf „Stimmt“ bestaetigt genau EIN Feld; die Seite schiebt sich nicht weg.
    let fall = seite();
    s.klick(".v-zeile[data-feld='bruttoarbeitslohn'] .v-ok");
    s.try_warte_bis("!!document.querySelector(\".v-zeile[data-feld='bruttoarbeitslohn'].v-fertig\")", 5);
    let f = stand(&fall);
    pp!("H.stimmt_bestaetigt_genau_ein_feld", f["bruttoarbeitslohn"]["zustand"] == json!("bestaetigt"), "{}", f["bruttoarbeitslohn"]);
    for fid in ["veranlagung", "ep_arbeitstage", "kein_kap"] {
        pp!("H.stimmt_bestaetigt_genau_ein_feld", f[fid]["zustand"] == json!("vorlaeufig"), "{fid} wurde mitbestaetigt: ein Klick hat mehr als eine Zeile bewegt");
    }
    let fall = seite();
    let _ = fall;
    s.klick(".v-zeile[data-feld='veranlagung'] .v-ok");
    s.try_warte_bis("!!document.querySelector(\".v-zeile[data-feld='veranlagung'].v-fertig\")", 5);
    pp!("H.bestaetigen_schiebt_die_seite_nicht_weg", s.sichtbar("#verstanden"), "die Verstanden-Seite ist nach einer Bestaetigung weg");
    pp!("H.bestaetigen_schiebt_die_seite_nicht_weg", !s.sichtbar("#wegpunkt"), "der Fragefluss hat sich davorgeschoben");
    pp!("H.bestaetigen_schiebt_die_seite_nicht_weg", s.anzahl(".v-zeile") == 4, "die restlichen Zeilen sind verschwunden");

    // „Weiter“ ist keine stille Zustimmung und kein stilles Verwerfen.
    let fall = seite();
    s.klick("#verstanden-weiter");
    crate::seite::zum_fragebogen(s, None);
    pp!("H.weiter_fuehrt_in_den_fragefluss_und_verliert_nichts", !s.sichtbar("#verstanden"), "die Verstanden-Seite steht noch");
    let f = stand(&fall);
    for fid in &feld_ids {
        pp!("H.weiter_fuehrt_in_den_fragefluss_und_verliert_nichts", f[fid.as_str()]["zustand"] == json!("vorlaeufig"), "{fid} hat sich durch 'Weiter' veraendert: das waere stille Zustimmung");
    }
    pp!("H.weiter_fuehrt_in_den_fragefluss_und_verliert_nichts", u.offene(&fall).contains(&"bruttoarbeitslohn".to_owned()), "der unbestaetigte Vorschlag steht nicht mehr im Fragefluss: er waere lautlos verloren");

    // „Aendern“ legt die Frage vor, verwirft nichts.
    let fall = seite();
    s.klick(".v-zeile[data-feld='ep_arbeitstage'] .v-aendern");
    s.try_warte_bis("document.getElementById('verstanden').hidden", 8);
    s.try_warte_bis("(() => { const e = document.getElementById('wegpunkt'); return !!e && !e.hidden; })()", 5);
    pp!("H.aendern_oeffnet_das_feld_im_fragefluss", s.ausdruck("AKTUELL && AKTUELL.feld_id") == json!("ep_arbeitstage"), "falsches Feld im Fragefluss: {}", s.ausdruck("AKTUELL && AKTUELL.feld_id"));
    pp!("H.aendern_oeffnet_das_feld_im_fragefluss", !s.sichtbar("#verstanden"), "die Liste steht neben dem Fragefluss");
    pp!("H.aendern_oeffnet_das_feld_im_fragefluss", stand(&fall)["ep_arbeitstage"]["zustand"] == json!("vorlaeufig"), "„Aendern“ hat den Vorschlag veraendert");

    // „Aendern“ kehrt in die Liste zurueck: MIT dem vorher gesetzten Haekchen, die korrigierte Zeile als erledigt.
    let fall = seite();
    s.klick(".v-zeile[data-feld='veranlagung'] .v-ok");
    s.try_warte_bis("!!document.querySelector(\".v-zeile[data-feld='veranlagung'].v-fertig\")", 5);
    let offen_vorher = s.anzahl("#verstanden-liste .v-zeile:not(.v-fertig)");
    pp!("H.aendern_kehrt_in_die_liste_zurueck", offen_vorher >= 2, "Vorbedingung: mindestens zwei Zeilen muessen offen bleiben, offen sind {offen_vorher}");
    s.klick(".v-zeile[data-feld='ep_arbeitstage'] .v-aendern");
    s.try_warte_bis("document.getElementById('verstanden').hidden", 8);
    s.try_warte_bis("(() => { const e = document.getElementById('wegpunkt'); return !!e && !e.hidden; })()", 5);
    pp!("H.aendern_kehrt_in_die_liste_zurueck", s.ausdruck("AKTUELL && AKTUELL.feld_id") == json!("ep_arbeitstage"), "falsches Feld im Fragefluss");
    s.fuelle("#feld-input", "210");
    s.halte("#bestaetigen", 800);
    pp!("H.aendern_kehrt_in_die_liste_zurueck", verstanden_offen(s), "die Liste kommt nicht zurueck");
    pp!("H.aendern_kehrt_in_die_liste_zurueck", stand(&fall)["ep_arbeitstage"]["wert"] == json!(210), "der korrigierte Wert steht nicht im Store: {}", stand(&fall)["ep_arbeitstage"]);
    let zeilen = s.ausdruck("[...document.querySelectorAll('#verstanden-liste .v-zeile')].map(li => ({feld: li.dataset.feld, fertig: li.classList.contains('v-fertig')}))");
    let fertig = |f: &str| zeilen.as_array().is_some_and(|a| a.iter().any(|z| z["feld"] == json!(f) && z["fertig"] == json!(true)));
    pp!("H.aendern_kehrt_in_die_liste_zurueck", fertig("veranlagung"), "das vorher gesetzte Haekchen ist weg: die Liste wurde neu gebaut statt zurueckgeholt: {zeilen}");
    pp!("H.aendern_kehrt_in_die_liste_zurueck", fertig("ep_arbeitstage"), "die korrigierte Zeile bietet weiter ein Bestaetigen an: {zeilen}");
    pp!("H.aendern_kehrt_in_die_liste_zurueck", s.anzahl("#verstanden-liste .v-zeile:not(.v-fertig)") == offen_vorher - 1, "die uebrigen Zeilen stimmen nicht: {zeilen}");

    // Die letzte Aenderung laesst die Liste nicht leer stehen.
    seite();
    for feld in ["veranlagung", "bruttoarbeitslohn", "kein_kap"] {
        s.klick(&format!(".v-zeile[data-feld='{feld}'] .v-ok"));
        s.try_warte_bis(&format!("!!document.querySelector(\".v-zeile[data-feld='{feld}'].v-fertig\")"), 5);
    }
    pp!("H.die_letzte_aenderung_laesst_die_liste_nicht_leer_stehen", s.anzahl("#verstanden-liste .v-zeile:not(.v-fertig)") == 1, "Vorbedingung: genau eine Zeile darf offen sein");
    s.klick(".v-zeile[data-feld='ep_arbeitstage'] .v-aendern");
    s.try_warte_bis("document.getElementById('verstanden').hidden", 8);
    s.fuelle("#feld-input", "210");
    s.halte("#bestaetigen", 800);
    let entschieden = s.try_warte_bis("!!document.querySelector(\"#verstanden-liste .v-zeile[data-feld='ep_arbeitstage'].v-fertig\")", 8);
    pp!("H.die_letzte_aenderung_laesst_die_liste_nicht_leer_stehen", entschieden.is_some(), "die korrigierte Zeile wird nicht als erledigt markiert");
    pp!("H.die_letzte_aenderung_laesst_die_liste_nicht_leer_stehen", !s.sichtbar("#verstanden"), "die Liste steht wieder da, obwohl in ihr nichts mehr offen ist");

    // Konflikte: beide Werte sichtbar, ohne Klick aendert sich nichts; „Uebernehmen“ ersetzt, „Meins behalten“ schreibt nichts.
    let konflikt_seite = || {
        modell.setze_vorschlaege(&konflikt_vorschlaege());
        frisch(&u, None);
        let fall = u.fall();
        let (st, r) = d.schreibe(&fall, "bruttoarbeitslohn", &json!(6_200_000));
        assert_eq!(st, 201, "{r}");
        s.fuelle("#chat-text", KONFLIKT_TEXT);
        s.klick("#chat-send");
        assert!(s.try_warte_bis("!!document.querySelector('.v-konflikt')", 8).is_some(), "der Konflikt wird nicht angezeigt");
        fall
    };
    let fall = konflikt_seite();
    let werte = s.ausdruck("[...document.querySelectorAll('.v-konflikt .v-seite-wert')].map(e => e.textContent)");
    let hat = |t: &str| werte.as_array().is_some_and(|a| a.iter().any(|w| w.as_str().is_some_and(|w| w.contains(t))));
    pp!("H.konflikt_erscheint_und_keine_seite_gewinnt_von_allein", hat("62.000,00"), "bisheriger Wert fehlt: {werte}");
    pp!("H.konflikt_erscheint_und_keine_seite_gewinnt_von_allein", hat("70.000,00"), "Vorschlag fehlt: {werte}");
    pp!("H.konflikt_erscheint_und_keine_seite_gewinnt_von_allein", stand(&fall)["bruttoarbeitslohn"]["wert"] == json!(6_200_000), "der Wert hat sich ohne Klick veraendert: ein Konflikt darf sich nicht selbst aufloesen");
    s.klick(".v-konflikt .v-uebernehmen");
    s.try_warte_bis("!!document.querySelector('.v-konflikt.v-fertig')", 5);
    let f = stand(&fall);
    pp!("H.konflikt_erscheint_und_keine_seite_gewinnt_von_allein", f["bruttoarbeitslohn"]["wert"] == json!(7_000_000) && f["bruttoarbeitslohn"]["zustand"] == json!("bestaetigt"), "die Uebernahme hat nicht gewirkt: {}", f["bruttoarbeitslohn"]);

    let fall = konflikt_seite();
    let vorher = stand(&fall);
    s.klick(".v-konflikt .v-ok");
    s.try_warte_bis("!!document.querySelector('.v-konflikt.v-fertig')", 5);
    pp!("H.meins_behalten_schreibt_nichts", stand(&fall) == vorher, "„Meins behalten“ hat etwas geschrieben");
}

// ---- chat_wartezustand: waehrend die KI antwortet, bleibt die Oberflaeche ehrlich -----------------------------------

/// Absenden und warten, bis der Aufruf wirklich offen ist (nicht: eine Zeit lang schlafen).
fn absenden_offen(s: &crate::browser::Sitzung, text: &str) -> bool {
    s.skript("window.__CHAT_OFFEN = null;", &[]);
    s.fuelle("#chat-text", text);
    s.klick("#chat-send");
    s.try_warte_bis("window.__CHAT_OFFEN !== null", 5).is_some()
}

fn gesperrt(s: &crate::browser::Sitzung) -> Value {
    s.ausdruck(
        "({wegpunkt: document.getElementById('wegpunkt').hasAttribute('inert'), verstanden: document.getElementById('verstanden').hasAttribute('inert'), \
          belegt: document.getElementById('belegt-liste').hasAttribute('inert'), importbox: document.querySelector('.vorjahr-box').hasAttribute('inert'), \
          chattext: document.getElementById('chat-text').disabled, sendknopf: document.getElementById('chat-send').disabled})",
    )
}

fn nichts_gesperrt(g: &Value) -> bool {
    g.as_object().is_some_and(|o| o.values().all(|v| *v == json!(false)))
}

fn erstes_cent_feld(s: &crate::browser::Sitzung, n: usize) -> String {
    let felder = s.auswerten(
        "async () => { const r = await jget(`/fall/${FALL}/fragen`); return r.body.fragen.filter(f => f.typ === 'cent').map(f => f.feld_id); }",
        &[],
    );
    felder[n].as_str().unwrap_or_default().to_owned()
}

#[test]
fn fluss_h_chat_wartezustand() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 780);
    let s = &u.s;
    let bereit = |feld_n: usize| {
        frisch(&u, None);
        halte_an(s);
        let feld = erstes_cent_feld(s, feld_n);
        assert!(u.rendere_frage(&feld), "{feld} in dieser Scheibe nicht erreichbar");
        feld
    };
    let antwort_text = |t: &str| {
        let mut a = antwort(&[], &[], &[]);
        a["antwort"] = json!(t);
        a
    };
    let warte_da = || s.ausdruck("!!document.querySelector('#chat-body .chat-warte')") == json!(true);

    // Punkt 1: waehrend des Aufrufs steht eine BEWEGTE Anzeige, als Zustand ausgezeichnet.
    frisch(&u, None);
    halte_an(s);
    pp!("H.waehrend_des_aufrufs_laeuft_eine_sichtbare_anzeige", absenden_offen(s, "ich hatte 620 euro solche ausgaben"), "der Aufruf wird nicht offen");
    pp!("H.waehrend_des_aufrufs_laeuft_eine_sichtbare_anzeige", warte_da(), "keine Laufanzeige im Chat-Verlauf, waehrend der Aufruf laeuft");
    let animation = s.ausdruck("getComputedStyle(document.querySelector('.chat-warte-punkte i')).animationName");
    pp!("H.waehrend_des_aufrufs_laeuft_eine_sichtbare_anzeige", animation == json!("chat-warte-puls"), "die Laufanzeige bewegt sich nicht (animation-name={animation}): ein Zustand, der wie eine Aussage aussieht");
    pp!("H.waehrend_des_aufrufs_laeuft_eine_sichtbare_anzeige", s.ausdruck("document.querySelector('.chat-warte').getAttribute('role')") == json!("status"), "die Laufanzeige ist fuer Screen-Reader nicht als Zustand ausgezeichnet");

    // Unter prefers-reduced-motion tritt eine ruhige Kennzeichnung an die Stelle der Animation.
    s.reduzierte_bewegung(true);
    frisch(&u, None);
    halte_an(s);
    pp!("H.reduzierte_bewegung_zeigt_statt_animation_eine_ruhige_kennzeichnung", absenden_offen(s, "ich hatte 620 euro solche ausgaben"), "der Aufruf wird nicht offen");
    pp!("H.reduzierte_bewegung_zeigt_statt_animation_eine_ruhige_kennzeichnung", warte_da(), "Laufanzeige fehlt unter prefers-reduced-motion ganz");
    let animation = s.ausdruck("getComputedStyle(document.querySelector('.chat-warte-punkte i')).animationName");
    pp!("H.reduzierte_bewegung_zeigt_statt_animation_eine_ruhige_kennzeichnung", animation == json!("none"), "unter prefers-reduced-motion laeuft trotzdem eine Animation ({animation})");
    let marke = s.ausdruck("getComputedStyle(document.querySelector('.chat-warte-still')).display");
    pp!("H.reduzierte_bewegung_zeigt_statt_animation_eine_ruhige_kennzeichnung", marke != json!("none"), "ohne Bewegung UND ohne Ersatz-Kennzeichnung ist der Wartezustand unsichtbar");
    pp!("H.reduzierte_bewegung_zeigt_statt_animation_eine_ruhige_kennzeichnung", s.ausdruck("document.querySelector('.chat-warte-still').textContent") == json!("läuft"), "die Kennzeichnung sagt nicht „läuft“");
    s.reduzierte_bewegung(false);

    // Punkt 2: gesperrt sind genau die Bereiche, aus denen sich `AKTUELL` aendern laesst.
    let feld = bereit(0);
    pp!("H.waehrend_des_aufrufs_ist_der_fragenbereich_gesperrt", absenden_offen(s, "ich hatte 620 euro solche ausgaben"), "der Aufruf wird nicht offen");
    let g = gesperrt(s);
    pp!("H.waehrend_des_aufrufs_ist_der_fragenbereich_gesperrt", g["sendknopf"] == json!(true), "Absendeknopf nicht gesperrt: {g}");
    pp!("H.waehrend_des_aufrufs_ist_der_fragenbereich_gesperrt", g["wegpunkt"] == json!(true), "Fragebereich nicht gesperrt: der Nutzer kann die Frage wechseln: {g}");
    pp!("H.waehrend_des_aufrufs_ist_der_fragenbereich_gesperrt", g["verstanden"] == json!(true), "Verstanden-Liste nicht gesperrt: {g}");
    pp!("H.waehrend_des_aufrufs_ist_der_fragenbereich_gesperrt", g["belegt"] == json!(true), "Belegt-Liste nicht gesperrt: ein Klick dort ruft korrigiereBestaetigt(): {g}");
    pp!("H.waehrend_des_aufrufs_ist_der_fragenbereich_gesperrt", g["importbox"] == json!(true), "Import-Box nicht gesperrt: Vorjahr/Kontoauszug rufen refresh(): {g}");
    pp!("H.waehrend_des_aufrufs_ist_der_fragenbereich_gesperrt", g["chattext"] == json!(true), "Chat-Eingabefeld nicht gesperrt (sein Inhalt wird am Ende geleert): {g}");

    // Nutzerpfad: ein echter Klick auf „Bestaetigen“ und „Erklaer mir“ kommt nicht durch, `AKTUELL` bleibt.
    let feld2 = bereit(0);
    pp!("H.waehrend_des_aufrufs_veraendert_ein_klick_die_aktuelle_frage_nicht", absenden_offen(s, "ich hatte 620 euro solche ausgaben"), "der Aufruf wird nicht offen");
    pp!("H.waehrend_des_aufrufs_veraendert_ein_klick_die_aktuelle_frage_nicht", !s.klick_versuch("#bestaetigen"), "der Klick auf „Bestaetigen“ ging waehrend des laufenden KI-Aufrufs durch");
    pp!("H.waehrend_des_aufrufs_veraendert_ein_klick_die_aktuelle_frage_nicht", !s.klick_versuch("#chat"), "der Klick auf „Erklaer mir“ ging durch: er leert den Chat-Verlauf und raeumt die ausstehende Antwort mit weg");
    pp!("H.waehrend_des_aufrufs_veraendert_ein_klick_die_aktuelle_frage_nicht", s.ausdruck("AKTUELL && AKTUELL.feld_id") == json!(feld2) && feld2 == feld, "die aktuelle Frage hat sich waehrend des Aufrufs geaendert");

    // Nach der Antwort ist alles wieder frei, die Laufanzeige weg, „Bestaetigen“ wieder anklickbar.
    bereit(0);
    absenden_offen(s, "ich hatte 620 euro solche ausgaben");
    gib_frei(s, 200, &antwort_text("Das kommt in Zeile 61 der Anlage."));
    s.try_warte_bis("!document.getElementById('chat-send').disabled", 5);
    pp!("H.nach_der_antwort_ist_alles_wieder_frei", !warte_da(), "die Laufanzeige steht nach der Antwort noch da");
    let g = gesperrt(s);
    pp!("H.nach_der_antwort_ist_alles_wieder_frei", nichts_gesperrt(&g), "nach der Antwort sind noch Sperren gesetzt: {g}");
    pp!("H.nach_der_antwort_ist_alles_wieder_frei", s.klick_versuch("#bestaetigen"), "„Bestaetigen“ ist nach der Antwort nicht wieder anklickbar");

    // Punkt 3: bricht der Aufruf ab (Server weg, WLAN aus), faellt JEDE Sperre, und der Nutzer erfaehrt es.
    bereit(0);
    absenden_offen(s, "ich hatte 620 euro solche ausgaben");
    gib_frei(s, 0, &Value::Null);
    s.try_warte_bis("!document.getElementById('chat-send').disabled", 5);
    pp!("H.nach_einem_fehlgeschlagenen_aufruf_ist_alles_wieder_frei", !warte_da(), "die Laufanzeige steht nach dem Abbruch noch da");
    let g = gesperrt(s);
    pp!("H.nach_einem_fehlgeschlagenen_aufruf_ist_alles_wieder_frei", nichts_gesperrt(&g), "nach dem Abbruch sind noch Sperren gesetzt: {g}");
    pp!("H.nach_einem_fehlgeschlagenen_aufruf_ist_alles_wieder_frei", s.klick_versuch("#bestaetigen"), "nach einem abgebrochenen KI-Aufruf ist die Oberflaeche dauerhaft gesperrt");
    pp!(
        "H.nach_einem_fehlgeschlagenen_aufruf_ist_alles_wieder_frei",
        s.ausdruck("[...document.querySelectorAll('#chat-body p')].some(e => e.textContent.includes('unerwartet'))") == json!(true),
        "der Nutzer erfaehrt nichts ueber den fehlgeschlagenen Aufruf"
    );

    // Eine Ausnahme in der Auswertung der Antwort laesst nichts gesperrt zurueck.
    let feld = bereit(0);
    s.skript("window.zeigeVerstanden = () => { throw new Error('Testfehler in der Auswertung'); };", &[]);
    absenden_offen(s, "ich hatte 620 euro solche ausgaben");
    gib_frei(s, 200, &antwort(&[json!({"feld_id": feld, "wert": 62_000})], &[], &[]));
    s.try_warte_bis("!document.getElementById('chat-send').disabled", 5);
    std::thread::sleep(std::time::Duration::from_millis(500));
    pp!("H.eine_ausnahme_in_der_auswertung_sperrt_die_oberflaeche_nicht", !warte_da(), "die Laufanzeige steht nach der Ausnahme noch da");
    let g = gesperrt(s);
    pp!("H.eine_ausnahme_in_der_auswertung_sperrt_die_oberflaeche_nicht", nichts_gesperrt(&g), "nach der Ausnahme sind noch Sperren gesetzt: {g}");
    pp!("H.eine_ausnahme_in_der_auswertung_sperrt_die_oberflaeche_nicht", s.klick_versuch("#bestaetigen"), "eine Ausnahme in der Auswertung laesst die Oberflaeche gesperrt zurueck");
    u.konsolenfehler(); // die absichtliche Ausnahme steht im Protokoll; die folgenden Faelle messen frisch

    // Kontextwechsel: eine Antwort, die zu einer anderen Frage kommt, wird gekennzeichnet (und zwar VOR der Antwort).
    let feld = bereit(0);
    absenden_offen(s, "zählt homeoffice als arbeitstag?");
    s.skript("refresh();", &[]);
    let gewechselt = s.try_warte_bis(&format!("AKTUELL && AKTUELL.feld_id !== {}", json!(feld)), 5);
    pp!("H.antwort_zu_einer_anderen_frage_wird_gekennzeichnet", gewechselt.is_some(), "der Kontext wechselt nicht: der Test misst sonst nichts");
    gib_frei(s, 200, &antwort_text("Homeoffice-Tage zählen nicht als Fahrten."));
    let da = s.try_warte_bis("!!document.querySelector('.chat-antwort')", 5);
    pp!("H.antwort_zu_einer_anderen_frage_wird_gekennzeichnet", da.is_some(), "die Antwort kommt nicht");
    let klassen = s.ausdruck("[...document.querySelectorAll('#chat-body p')].map(e => e.className)");
    let pos = |k: &str| klassen.as_array().and_then(|a| a.iter().position(|x| x == k));
    pp!("H.antwort_zu_einer_anderen_frage_wird_gekennzeichnet", pos("chat-kontext-hinweis").is_some(), "die Antwort wurde still unter eine andere Frage gehaengt: {klassen}");
    pp!("H.antwort_zu_einer_anderen_frage_wird_gekennzeichnet", pos("chat-kontext-hinweis") < pos("chat-antwort"), "der Hinweis steht hinter der Antwort: {klassen}");

    // Gegenprobe: bleibt der Nutzer bei seiner Frage, erscheint keine Einschraenkung.
    bereit(0);
    absenden_offen(s, "zählt homeoffice als arbeitstag?");
    gib_frei(s, 200, &antwort_text("Homeoffice-Tage zählen nicht als Fahrten."));
    s.try_warte_bis("!!document.querySelector('.chat-antwort')", 5);
    let klassen = s.ausdruck("[...document.querySelectorAll('#chat-body p')].map(e => e.className)");
    pp!("H.ohne_kontextwechsel_kein_hinweis", klassen.as_array().is_some_and(|a| !a.is_empty() && !a.iter().any(|x| x == "chat-kontext-hinweis")), "Hinweis erscheint, obwohl die Frage dieselbe geblieben ist (oder gar keine Antwort steht): {klassen}");

    // Der Schreibpfad haengt NICHT an `AKTUELL`: ein Vorschlag fuer Feld A, bestaetigt waehrend die Seite bei Feld B steht,
    // wird unter A geschrieben (sonst waere aus einem Anzeige- ein Datenfehler geworden).
    let feld_a = bereit(0);
    let feld_b = erstes_cent_feld(s, 1);
    s.skript(
        "window.__EVENTS = []; const vorher = window.fetch; window.fetch = (u, o) => { \
           if (String(u).endsWith('/event') && o && o.method === 'POST') { window.__EVENTS.push(JSON.parse(o.body || '{}')); \
             return Promise.resolve(new Response(JSON.stringify({fehler: 'im Test abgefangen'}), {status: 400, headers: {'Content-Type': 'application/json'}})); } \
           return vorher(u, o); };",
        &[],
    );
    absenden_offen(s, "620 euro dafür");
    gib_frei(s, 200, &antwort(&[json!({"feld_id": feld_a, "wert": 62_000, "beleg": "620 euro"})], &[], &[]));
    let v = s.try_warte_bis("(() => { const e = document.getElementById('verstanden'); return !!e && !e.hidden; })()", 5);
    pp!("H.vorschlag_landet_unter_seiner_eigenen_feld_id", v.is_some(), "die Verstanden-Seite kommt nicht");
    u.rendere_frage(&feld_b);
    s.skript("document.querySelector('#verstanden-liste .v-ok').click();", &[]);
    let gepostet = s.try_warte_bis("window.__EVENTS.length > 0", 5);
    pp!("H.vorschlag_landet_unter_seiner_eigenen_feld_id", gepostet.is_some(), "es wurde gar kein Event abgeschickt");
    let letztes = s.ausdruck("window.__EVENTS[window.__EVENTS.length - 1].feld_id");
    pp!("H.vorschlag_landet_unter_seiner_eigenen_feld_id", letztes == json!(feld_a), "der Vorschlag fuer {feld_a} wurde unter {letztes} geschrieben (AKTUELL stand auf {feld_b}): aus dem Anzeigefehler wird ein Datenfehler");
}

// ---- ki_fokus: wer die KI waehlt, bekommt NUR das KI-Fenster --------------------------------------------------------

/// Was vom Fluss tatsaechlich im Bild steht: die direkten Kinder von `#flow`, gefiltert nach Box UND `display` (das
/// Ausblenden geschieht per CSS, das `hidden`-Attribut bliebe unveraendert und die Messung waere blind).
const SICHT_JS: &str = "(() => { const da = e => !!(e && e.getBoundingClientRect().width > 0 && e.getBoundingClientRect().height > 0 \
    && getComputedStyle(e).display !== 'none'); const flow = document.getElementById('flow'); \
    return { kinder: [...flow.children].filter(da).map(e => e.id || String(e.className).split(' ')[0] || e.tagName.toLowerCase()), \
      berater: Math.round(document.getElementById('berater').getBoundingClientRect().width), fenster: window.innerWidth, \
      fragebogen: da(document.getElementById('wegpunkt')), ring: da(document.getElementById('ring')), belegt: da(document.querySelector('.belegt')) }; })()";

/// Die Spaltenbreiten des GRIDS von `#flow` (links der Fragebogen, rechts der Berater), nicht die eines Kindes: ein
/// Kind, das gerade ausgeblendet ist, ergaebe 0 px gegen die Beraterbreite und waere von selbst gruen.
const BREITEN_JS: &str = "(() => { const s = getComputedStyle(document.getElementById('flow')).gridTemplateColumns.split(' ') \
    .map(parseFloat).filter(x => !Number.isNaN(x)); \
    return {links: s[0] || 0, rechts: s.length > 1 ? s[1] : 0, spalten: s.length}; })()";

/// Wo `#<id>` im Dokument liegt, gemessen an Reihenfolge und Lage, nicht an einer Klasse.
fn lage(s: &crate::browser::Sitzung, id: &str) -> Value {
    s.ausdruck(&format!(
        "(() => {{ const x = document.getElementById({id:?}); const berater = document.getElementById('berater'); \
          const verlauf = document.getElementById('chat-body'); const eingabe = document.querySelector('.chat-eingabe'); \
          const nach = (a, b) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) > 0; \
          return {{im_berater: berater.contains(x), nach_verlauf: nach(verlauf, x), vor_eingabe: nach(x, eingabe), \
                   oben: x.getBoundingClientRect().top, eingabe_oben: eingabe.getBoundingClientRect().top}}; }})()"
    ))
}

#[test]
fn fluss_h_ki_fokus() {
    let _sperre = crate::browser::seriell();
    // Breit genug, dass die Zweispalten-Regel (min-width: 900px) ueberhaupt greift.
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let s = &u.s;
    let ki_seite = || {
        u.oeffne_start();
        u.fall_beginnen("ki");
        s.warte_sichtbar("#chat-body .chat-erklaer", 8);
        s.warte_bis("document.getElementById('berater').getBoundingClientRect().width > 0", 5, "das KI-Fenster hat eine Box");
    };

    // Der KI-Weg: NUR das Panel, kein Stueck Fragebogen, und es ist der groesste Bereich.
    ki_seite();
    let fehler = u.konsolenfehler();
    pp!("H.kein_konsolenfehler", fehler.is_empty(), "Konsolenfehler auf dem KI-Weg: {fehler:?}");
    let sicht = s.ausdruck(SICHT_JS);
    pp!("H.auf_dem_ki_weg_steht_NUR_das_ki_fenster", sicht["kinder"] == json!(["berater"]), "neben dem KI-Fenster steht noch etwas: {} (auf diesem Weg gibt es nur das Panel)", sicht["kinder"]);
    pp!("H.auf_dem_ki_weg_steht_NUR_das_ki_fenster", sicht["fragebogen"] == json!(false), "die Frage-Karte des Fragebogens steht im Bild");
    pp!("H.auf_dem_ki_weg_steht_NUR_das_ki_fenster", sicht["ring"] == json!(false), "der Ring des Fragebogens steht im Bild");
    pp!("H.auf_dem_ki_weg_steht_NUR_das_ki_fenster", sicht["belegt"] == json!(false), "die Liste der beantworteten Felder steht im Bild");
    pp!(
        "H.auf_dem_ki_weg_steht_NUR_das_ki_fenster",
        sicht["berater"].as_f64().unwrap_or(0.0) > sicht["fenster"].as_f64().unwrap_or(0.0) * 0.5,
        "berater={} px von {} px: das ist nicht der groesste Bereich",
        sicht["berater"],
        sicht["fenster"]
    );
    let teile = s.ausdruck(
        "(() => { const da = q => { const e = document.querySelector(q); return !!(e && e.getBoundingClientRect().width > 0 && getComputedStyle(e).display !== 'none'); }; \
          return {balken: da('.fortschritt'), balkentext: da('.fortschritt-text'), import: da('.vorjahr-box'), ring: da('#ring'), spanne: da('#spanne'), \
                  frage: da('#wegpunkt'), bestaetigen: da('#bestaetigen'), belegt: da('.belegt'), panel: da('#berater')}; })()",
    );
    let steht_da: Vec<&String> = teile.as_object().map(|o| o.iter().filter(|(k, v)| **v == json!(true) && k.as_str() != "panel").map(|(k, _)| k).collect()).unwrap_or_default();
    pp!("H.im_ki_weg_ist_kein_stueck_fragebogen_im_bild", steht_da.is_empty(), "vom Fragebogen steht noch im Bild: {steht_da:?}");
    pp!("H.im_ki_weg_ist_kein_stueck_fragebogen_im_bild", teile["panel"] == json!(true), "das Panel selbst ist auch weg: dann ist gar nichts mehr da");

    // Der Rueckweg ist die einzige Tuer: er fuehrt in den Fragebogen, und der Berater bleibt als schmaler Begleiter.
    pp!("H.es_gibt_einen_weg_zurueck_in_den_fragebogen", s.sichtbar("#zum-fragebogen"), "kein sichtbarer Weg zurueck in den Fragebogen: der Nutzer sitzt im Panel fest");
    s.klick("#zum-fragebogen");
    zum_fragebogen(s, None);
    let sicht = s.ausdruck(SICHT_JS);
    pp!("H.es_gibt_einen_weg_zurueck_in_den_fragebogen", sicht["fragebogen"] == json!(true), "nach dem Klick ist der Fragebogen nicht da: {sicht}");
    pp!("H.es_gibt_einen_weg_zurueck_in_den_fragebogen", sicht["ring"] == json!(true), "der Ring fehlt auch: {sicht}");
    pp!("H.es_gibt_einen_weg_zurueck_in_den_fragebogen", sicht["kinder"].as_array().is_some_and(|k| k.contains(&json!("berater"))), "der Berater ist verschwunden: {} (er soll Begleiter werden, nicht weg sein)", sicht["kinder"]);
    let b = s.ausdruck(BREITEN_JS);
    pp!("H.es_gibt_einen_weg_zurueck_in_den_fragebogen", b["rechts"].as_f64() < b["links"].as_f64(), "der Chat ist im Fragebogen immer noch der Hauptbereich: {b}");

    // Der Fragebogen-Weg: die Leiste bleibt schmal, kein Rueckweg-Knopf, der Kopf (Fortschritt, Import) vollstaendig.
    frisch(&u, None);
    let b = s.ausdruck(BREITEN_JS);
    pp!("H.auf_dem_fragebogen_weg_bleibt_die_leiste_schmal", b["spalten"].as_i64().unwrap_or(0) > 1 && b["rechts"].as_f64() < b["links"].as_f64(), "auch ohne KI-Weg ist der Chat der groesste Bereich: {b}");
    pp!("H.der_rueckweg_ist_nur_auf_dem_ki_weg_da", !s.sichtbar("#zum-fragebogen"), "der Rueckweg-Knopf steht auch im Fragebogen, wo er nichts bedeutet");
    let kopf = s.ausdruck(
        "(() => { const da = q => { const e = document.querySelector(q); return !!(e && e.getBoundingClientRect().width > 0 && getComputedStyle(e).display !== 'none'); }; \
          return {balken: da('.fortschritt'), import: da('.vorjahr-box')}; })()",
    );
    pp!("H.auf_dem_fragebogen_weg_bleibt_der_kopf_vollstaendig", kopf["balken"] == json!(true) && kopf["import"] == json!(true), "auf dem Fragebogen-Weg fehlt der Kopf: {kopf}");

    // Rueckfragen sind noch der KI-Weg (der Nutzer wartet auf sie), und der Wizard steht IM KI-Fenster zwischen Verlauf
    // und Eingabe.
    ki_seite();
    stub(s, &[antwort(&[], &[rf("Wie viele Tage?", "ep_arbeitstage")], &[])]);
    pp!("H.bei_rueckfragen_bleibt_der_ki_fokus", senden(s, "ich hatte 62000 euro brutto", true), "die Rueckfragen-Seite kommt nicht");
    let sicht = s.ausdruck(SICHT_JS);
    pp!("H.bei_rueckfragen_bleibt_der_ki_fokus", sicht["kinder"] == json!(["berater"]), "bei den Rueckfragen taucht der Fragebogen wieder auf: {} (der Nutzer ist noch auf dem KI-Weg)", sicht["kinder"]);
    let l = lage(s, "rueckfragen");
    pp!("H.der_wizard_steht_im_ki_fenster", l["im_berater"] == json!(true), "der Wizard haengt nicht im KI-Fenster: er ist wieder eine eigene Karte");
    pp!("H.der_wizard_steht_im_ki_fenster", l["nach_verlauf"] == json!(true), "der Wizard steht UEBER dem Verlauf: das Gespraech liest sich rueckwaerts");
    pp!("H.der_wizard_steht_im_ki_fenster", l["vor_eingabe"] == json!(true), "der Wizard steht unter dem Eingabefeld");
    pp!("H.der_wizard_steht_im_ki_fenster", l["oben"].as_f64() < l["eingabe_oben"].as_f64(), "auch im Bild steht der Wizard nicht ueber der Eingabe: {l}");

    // Die Bestaetigungen sind die letzte Stufe desselben Wegs: gleicher Ort, Fragebogen bleibt draussen.
    ki_seite();
    stub(s, &[antwort(&[vorschlag(CENT_FELD, &json!(6_200_000), "62000 euro brutto")], &[], &[])]);
    pp!("H.bei_bestaetigungen_bleibt_der_ki_fokus", senden(s, "ich hatte 62000 euro brutto", false), "die Antwort kommt nicht");
    s.warte_bis("!document.getElementById('verstanden').hidden", 8, "die Verstanden-Seite steht");
    let sicht = s.ausdruck(SICHT_JS);
    pp!("H.bei_bestaetigungen_bleibt_der_ki_fokus", sicht["kinder"] == json!(["berater"]), "beim Bestaetigen taucht der Fragebogen wieder auf: {} (auch das ist noch der KI-Weg)", sicht["kinder"]);
    let l = lage(s, "verstanden");
    pp!("H.die_bestaetigungen_stehen_auch_im_ki_fenster", l["im_berater"] == json!(true), "die Bestaetigungen stehen wieder ausserhalb des KI-Fensters");
    pp!("H.die_bestaetigungen_stehen_auch_im_ki_fenster", l["nach_verlauf"] == json!(true) && l["vor_eingabe"] == json!(true), "sie stehen nicht zwischen Verlauf und Eingabefeld: {l}");

    // Der Verlauf tritt zurueck, solange der Wizard steht: sonst laege die Frage unterhalb des Fensterrands. Der Verlauf
    // muss LANG sein, sonst misst der Test nichts (ein leerer ist 65 px hoch, ob der Deckel bei 68vh oder 30vh steht).
    ki_seite();
    s.skript(
        "const b = document.getElementById('chat-body'); for (let i = 0; i < 40; i++) { const p = document.createElement('p'); \
         p.textContent = 'Verlaufszeile ' + i + ' - Fülltext, damit der Verlauf seinen Deckel erreicht.'; b.appendChild(p); }",
        &[],
    );
    let vorher = s.ausdruck("parseFloat(getComputedStyle(document.getElementById('chat-body')).maxHeight)").as_f64().unwrap_or(0.0);
    stub(s, &[antwort(&[], &[rf("Wie viele Tage?", "ep_arbeitstage")], &[])]);
    pp!("H.der_verlauf_tritt_zurueck_solange_der_wizard_steht", senden(s, "ich hatte 62000 euro brutto", true), "die Rueckfragen-Seite kommt nicht");
    let m = s.ausdruck(
        "(() => { const cb = document.getElementById('chat-body'); const rf = document.getElementById('rueckfragen').getBoundingClientRect(); \
          return {deckel: parseFloat(getComputedStyle(cb).maxHeight), verlauf: cb.getBoundingClientRect().height, rf_unten: rf.bottom, fenster: window.innerHeight}; })()",
    );
    let zahl = |k: &str| m[k].as_f64().unwrap_or(f64::NAN);
    pp!("H.der_verlauf_tritt_zurueck_solange_der_wizard_steht", zahl("deckel") < vorher, "der Deckel des Verlaufs ist beim Wizard nicht gesunken ({vorher} -> {} px): die Regel .berater.rf-laeuft greift nicht", zahl("deckel"));
    pp!("H.der_verlauf_tritt_zurueck_solange_der_wizard_steht", zahl("verlauf") <= zahl("deckel") + 1.0, "der Verlauf ist hoeher als sein eigener Deckel: {m}");
    pp!("H.der_verlauf_tritt_zurueck_solange_der_wizard_steht", zahl("rf_unten") <= zahl("fenster"), "der Wizard endet unterhalb des Fensterrands: {m} (der Nutzer sieht die Knoepfe nicht, ohne zu scrollen)");
}

// ---- ki_flow, zweiter Teil: Auflagen und der Rechenweg unter dem Wert -----------------------------------------------

fn verstanden_steht(s: &crate::browser::Sitzung) -> bool {
    s.try_warte_bis("(() => { const e = document.getElementById('verstanden'); return !!e && !e.hidden; })()", 8).is_some()
}

/// Wo der Rechenweg in der ersten Zeile der Verstanden-Liste liegt (`_rechenweg_lage`).
fn rechenweg_lage(s: &crate::browser::Sitzung) -> Value {
    s.ausdruck(
        "(() => { const li = document.querySelector('#verstanden-liste .v-zeile'); if (!li) return null; \
          const rw = li.querySelector('.v-rechenweg'); const wert = li.querySelector('.v-wert') || li.querySelector('.v-paar'); \
          const beleg = li.querySelector('.v-beleg'); if (!rw) return {da: false}; \
          const nach = (a, b) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) > 0; \
          return {da: true, text: rw.textContent, unter_dem_wert: nach(wert, rw), ueber_dem_beleg: beleg ? nach(rw, beleg) : null}; })()",
    )
}

const RECHENWEG_SATZ: &str = "50.000 € pro Jahr ÷ 12 × 6 Monate";

fn halber_rechenweg(erklaerung: &str) -> Value {
    json!({"basis": 5_000_000, "faktor": 0.5, "erklaerung": erklaerung})
}

#[test]
fn fluss_h_ki_flow_auflagen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 360, 780);
    let s = &u.s;
    let sperren_ids = "['wegpunkt','verstanden','belegt-liste','rueckfragen'].filter(id => document.getElementById(id).hasAttribute('inert'))";

    // Eine neue Antwort ueberholt die laufende Runde: die alten Fragen sind durch den neuen Satz laengst beantwortet.
    frisch(&u, None);
    let mut danach = antwort(&[], &[], &[]);
    danach["antwort"] = json!("Danke — dann rechne ich mit dem anteiligen Betrag.");
    stub(s, &[antwort(&[], &[rf("Ganzes Jahr oder anteilig?", CENT_FELD), rf("Unterstützt du jemanden?", BOOL_FELD_INVERTIERT)], &[]), danach]);
    pp!("H.eine_neue_antwort_ueberholt_die_laufende_runde", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    pp!("H.eine_neue_antwort_ueberholt_die_laufende_runde", senden(s, "anteilig, ich habe nur bis Juni gearbeitet", false), "die zweite Antwort kommt nicht");
    s.warte_sichtbar("#wegpunkt", 5);
    pp!("H.eine_neue_antwort_ueberholt_die_laufende_runde", !s.sichtbar("#rueckfragen"), "die alte Rueckfrage steht noch da, obwohl der Nutzer inzwischen geantwortet hat");
    pp!("H.eine_neue_antwort_ueberholt_die_laufende_runde", s.textinhalt("#chat-body .chat-antwort").contains("anteiligen Betrag"), "die neue Antwort steht nicht im Verlauf");
    let fehler = u.konsolenfehler();
    pp!("H.eine_neue_antwort_ueberholt_die_laufende_runde", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Abwaertskompatibel: fehlt `rueckfragen` in der Antwort, gehen Vorschlaege direkt zur Bestaetigung.
    frisch(&u, None);
    let mut alt = antwort(&[json!({"feld_id": "aussergewoehnliche_belastungen", "wert": 62_000, "beleg": "620 euro"})], &[], &[]);
    alt["antwort"] = json!("Das kommt in Zeile 61 der Anlage.");
    if let Some(o) = alt.as_object_mut() {
        o.remove("aussagen");
        o.remove("rueckfragen");
    }
    stub(s, &[alt]);
    pp!("H.ohne_rueckfragen_bleibt_alles_wie_bisher", senden(s, "ich habe bis juni gearbeitet", false), "die Antwort kommt nicht");
    pp!("H.ohne_rueckfragen_bleibt_alles_wie_bisher", verstanden_steht(s), "ohne Rueckfragen kommen die Bestaetigungen nicht");
    pp!("H.ohne_rueckfragen_bleibt_alles_wie_bisher", !s.sichtbar("#rueckfragen"), "ohne Rueckfragen steht trotzdem die neue Seite da");
    pp!("H.ohne_rueckfragen_bleibt_alles_wie_bisher", s.textinhalt("#chat-body .chat-antwort").contains("Das kommt in Zeile 61"), "die Antwort steht nicht im Verlauf");
    pp!("H.ohne_rueckfragen_bleibt_alles_wie_bisher", s.textinhalt("#chat-body").contains("— oben."), "die Meldung verweist nicht mehr auf die Bestaetigungen oben");
    let fehler = u.konsolenfehler();
    pp!("H.ohne_rueckfragen_bleibt_alles_wie_bisher", fehler.is_empty(), "Konsolenfehler ohne die neuen Felder: {fehler:?}");

    // Die Chat-Sperre fasst auch die Rueckfragen-Seite: „Weiter“ dort schriebe ein Event und zoege `AKTUELL` weg.
    frisch(&u, None);
    stub(s, &[antwort(&[], &[rf("Ganzes Jahr oder anteilig?", CENT_FELD)], &[])]);
    pp!("H.die_chat_sperre_fasst_auch_die_rueckfragen_seite", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    halte_an(s);
    pp!("H.die_chat_sperre_fasst_auch_die_rueckfragen_seite", absenden_offen(s, "und eine Brille"), "der zweite Aufruf wird nicht offen");
    pp!("H.die_chat_sperre_fasst_auch_die_rueckfragen_seite", s.ausdruck("document.getElementById('rueckfragen').hasAttribute('inert')") == json!(true), "die Rueckfragen-Seite bleibt waehrend des Aufrufs bedienbar: ein Klick auf „Weiter“ schriebe dort ein Event");
    let mut verstanden = antwort(&[], &[], &[]);
    verstanden["antwort"] = json!("Verstanden.");
    gib_frei(s, 200, &verstanden);
    s.try_warte_bis("!document.getElementById('chat-send').disabled", 5);
    let offen = s.ausdruck(sperren_ids);
    pp!("H.die_chat_sperre_fasst_auch_die_rueckfragen_seite", offen == json!([]), "nach dem Aufruf sind noch Sperren gesetzt: {offen}");

    // Die Rueckfragen-Seite nimmt den Fokus, bei JEDER Frage; „Spaeter“ ist der Ausweg.
    frisch(&u, None);
    stub(s, &[antwort(&[], &[rf("Ganzes Jahr oder anteilig?", CENT_FELD), rf("Unterstützt du jemanden?", BOOL_FELD_INVERTIERT)], &[])]);
    pp!("H.die_rueckfragen_seite_nimmt_den_fokus", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    pp!("H.die_rueckfragen_seite_nimmt_den_fokus", s.ausdruck("document.activeElement.id") == json!("rueckfragen"), "der Fokus bleibt im Chat: der Screenreader sagt den Wechsel nicht an");
    s.fuelle("#rf-input", "62000");
    s.klick("#rf-weiter");
    s.try_warte_bis("document.getElementById('rf-zaehler').textContent.includes('2 von 2')", 5);
    pp!("H.die_rueckfragen_seite_nimmt_den_fokus", s.ausdruck("document.activeElement.id") == json!("rueckfragen"), "bei der zweiten Frage wandert der Fokus nicht mit");
    pp!("H.die_rueckfragen_seite_nimmt_den_fokus", s.sichtbar("#rf-spaeter"), "ohne Ausweg sitzt fest, wer die Frage nicht beantworten kann");

    // 360 px: die Frage ist Modelltext beliebiger Laenge; das unteilbare Wort haengt an `overflow-wrap:anywhere`.
    s.sichtflaeche(360, 640);
    frisch(&u, None);
    let lang = "Meintest du den Betrag auf der Buchung DE00500105175407324931unterkontoLohnUndGehaltJanuarBisJuni2025Sammelbuchung?";
    stub(s, &[antwort(&[], &[rf(lang, CENT_FELD)], &[])]);
    pp!("H.die_seite_passt_auf_den_schmalen_schirm", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    pp!("H.die_seite_passt_auf_den_schmalen_schirm", s.ausdruck("document.documentElement.scrollWidth").as_i64().unwrap_or(-1) <= 360, "horizontales Scrollen bei 360 px durch die Rueckfragen-Seite");
    let (x, _, b, _) = s.box_von("#rueckfragen");
    pp!("H.die_seite_passt_auf_den_schmalen_schirm", x >= 0.0 && x + b <= 360.0, "die Karte ragt aus dem Bild: x={x} breite={b}");
    for sel in ["#rf-weiter", "#rf-spaeter"] {
        let (x, _, b, h) = s.box_von(sel);
        pp!("H.die_seite_passt_auf_den_schmalen_schirm", h >= 44.0, "Tap-Ziel {sel} unter 44 px: {h}");
        pp!("H.die_seite_passt_auf_den_schmalen_schirm", x >= 0.0 && x + b <= 360.0, "{sel} ragt aus dem Bild: x={x} breite={b}");
    }
    let (ax, ay, aw, ah) = s.box_von("#rf-weiter");
    let (cx, cy, _, _) = s.box_von("#rf-spaeter");
    pp!("H.die_seite_passt_auf_den_schmalen_schirm", ax + aw <= cx + 1.0 || ay + ah <= cy + 1.0, "die beiden Knoepfe ueberlappen");
    s.sichtflaeche(360, 780);

    // Der Rechenweg steht UNTER dem Wert und UEBER dem Zitat: Ergebnis, dann wie es entstand, dann worauf es sich stuetzt.
    frisch(&u, None);
    stub(s, &[antwort(&[json!({"feld_id": CENT_FELD, "wert": 2_500_000, "beleg": "50.000 im jahr, bis juni gearbeitet", "rechenweg": halber_rechenweg(RECHENWEG_SATZ)})], &[], &[])]);
    pp!("H.der_rechenweg_steht_unter_dem_wert", senden(s, "ich habe bis juni gearbeitet", false), "die Antwort kommt nicht");
    pp!("H.der_rechenweg_steht_unter_dem_wert", verstanden_steht(s), "die Verstanden-Seite kommt nicht");
    let l = rechenweg_lage(s);
    pp!("H.der_rechenweg_steht_unter_dem_wert", l["da"] == json!(true), "der ausgerechnete Wert steht ohne seine Rechnung da: der Nutzer bestaetigt eine Zahl, deren Zustandekommen er nicht sieht ({l})");
    pp!("H.der_rechenweg_steht_unter_dem_wert", l["text"].as_str().unwrap_or_default().contains(RECHENWEG_SATZ), "Rechenweg-Text fehlt: {l}");
    pp!("H.der_rechenweg_steht_unter_dem_wert", l["unter_dem_wert"] == json!(true), "der Rechenweg steht nicht unter dem Wert: {l}");
    pp!("H.der_rechenweg_steht_unter_dem_wert", l["ueber_dem_beleg"] == json!(true), "der Rechenweg steht unter dem Zitat statt darueber: {l}");
    let fehler = u.konsolenfehler();
    pp!("H.der_rechenweg_steht_unter_dem_wert", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Wo nichts gerechnet wurde (`rechenweg` null oder ganz fehlend), steht keine leere Zeile da.
    frisch(&u, None);
    stub(s, &[antwort(
        &[
            json!({"feld_id": CENT_FELD, "wert": 6_200_000, "beleg": "62000 brutto", "rechenweg": null}),
            json!({"feld_id": "aussergewoehnliche_belastungen", "wert": 62_000, "beleg": "620 euro"}),
        ],
        &[],
        &[],
    )]);
    pp!("H.ohne_rechenweg_steht_dort_nichts", senden(s, "ich habe bis juni gearbeitet", false), "die Antwort kommt nicht");
    pp!("H.ohne_rechenweg_steht_dort_nichts", verstanden_steht(s), "die Verstanden-Seite kommt nicht");
    pp!("H.ohne_rechenweg_steht_dort_nichts", s.anzahl("#verstanden-liste .v-rechenweg") == 0, "ohne Rechnung steht trotzdem eine Rechenweg-Zeile da");
    pp!("H.ohne_rechenweg_steht_dort_nichts", s.anzahl("#verstanden-liste .v-zeile") == 2, "die Vorschlaege selbst kamen nicht mehr durch: {}", s.anzahl("#verstanden-liste .v-zeile"));
    let fehler = u.konsolenfehler();
    pp!("H.ohne_rechenweg_steht_dort_nichts", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Ein Rechenweg ohne Satz wird aus den Zahlen zusammengesetzt; die Basis (Cent) erscheint als Euro.
    frisch(&u, None);
    stub(s, &[antwort(&[json!({"feld_id": CENT_FELD, "wert": 2_500_000, "beleg": "50.000 im jahr", "rechenweg": halber_rechenweg("")})], &[], &[])]);
    pp!("H.ein_rechenweg_ohne_satz_verschwindet_nicht", senden(s, "ich habe bis juni gearbeitet", false), "die Antwort kommt nicht");
    pp!("H.ein_rechenweg_ohne_satz_verschwindet_nicht", verstanden_steht(s), "die Verstanden-Seite kommt nicht");
    let l = rechenweg_lage(s);
    pp!("H.ein_rechenweg_ohne_satz_verschwindet_nicht", l["da"] == json!(true), "ohne Satz ist die Rechnung ganz verschwunden");
    let text = l["text"].as_str().unwrap_or_default();
    pp!("H.ein_rechenweg_ohne_satz_verschwindet_nicht", text.contains("50.000,00") && text.contains("0.5"), "die Basis erscheint nicht als Euro-Betrag: {text:?} (5000000 Cent sind 50.000 EUR, nicht fuenf Millionen)");

    // Am Widerspruch wiegt die Rechnung am schwersten: dort vergleicht der Nutzer die eigene Zahl mit einer ausgerechneten.
    frisch(&u, None);
    let mut mit_konflikt = antwort(&[], &[], &[]);
    mit_konflikt["konflikte"] = json!([{
        "feld_id": CENT_FELD, "wert": 2_500_000, "beleg": "bis juni gearbeitet", "frage": format!("Frage zu {CENT_FELD}"), "typ": "cent",
        "frage_invertiert": false, "einheit": null, "enum_labels": null, "aussage": 0,
        "aktueller_wert": 5_000_000, "vorschlag_wert": 2_500_000, "aktuelles_event_id": "EV-ALT", "gross": false, "begruendung": "",
        "rechenweg": halber_rechenweg(RECHENWEG_SATZ),
    }]);
    stub(s, &[mit_konflikt]);
    pp!("H.der_rechenweg_steht_auch_am_widerspruch", senden(s, "ich habe bis juni gearbeitet", false), "die Antwort kommt nicht");
    pp!("H.der_rechenweg_steht_auch_am_widerspruch", verstanden_steht(s), "die Verstanden-Seite kommt nicht");
    let l = rechenweg_lage(s);
    pp!("H.der_rechenweg_steht_auch_am_widerspruch", l["da"] == json!(true), "am Widerspruch fehlt die Rechnung");
    pp!("H.der_rechenweg_steht_auch_am_widerspruch", l["text"].as_str().unwrap_or_default().contains("÷ 12 × 6 Monate"), "der Rechenweg-Text fehlt: {l}");
    pp!("H.der_rechenweg_steht_auch_am_widerspruch", l["unter_dem_wert"] == json!(true), "der Rechenweg steht nicht unter dem Wertepaar: {l}");
    pp!("H.der_rechenweg_steht_auch_am_widerspruch", s.anzahl("#verstanden-liste .v-uebernehmen") > 0, "der Konflikt ist nicht mehr aufloesbar");
    let fehler = u.konsolenfehler();
    pp!("H.der_rechenweg_steht_auch_am_widerspruch", fehler.is_empty(), "Konsolenfehler: {fehler:?}");

    // Die Seite liegt im KI-Panel und fuellt es aus (breiter Schirm).
    s.sichtflaeche(1280, 800);
    frisch(&u, None);
    stub(s, &[antwort(&[], &[rf("Ganzes Jahr oder anteilig?", CENT_FELD)], &[])]);
    pp!("H.die_seite_liegt_im_ki_panel", senden(s, "ich habe bis juni gearbeitet", true), "die Rueckfragen-Seite kommt nicht");
    let lage = s.ausdruck(
        "(() => { const rf = document.getElementById('rueckfragen'); const b = document.getElementById('berater'); \
          const r = rf.getBoundingClientRect(), bb = b.getBoundingClientRect(); \
          return {im_panel: b.contains(rf), rf_links: r.left, rf_rechts: r.right, berater_links: bb.left, berater_rechts: bb.right, \
                  berater_sichtbar: bb.width > 0 && bb.height > 0}; })()",
    );
    let z = |k: &str| lage[k].as_f64().unwrap_or(f64::NAN);
    pp!("H.die_seite_liegt_im_ki_panel", lage["berater_sichtbar"] == json!(true), "das KI-Panel ist verschwunden");
    pp!("H.die_seite_liegt_im_ki_panel", lage["im_panel"] == json!(true), "der Wizard liegt wieder ausserhalb des KI-Panels: {lage}");
    pp!("H.die_seite_liegt_im_ki_panel", z("rf_links") >= z("berater_links") - 1.0 && z("rf_rechts") <= z("berater_rechts") + 1.0, "der Wizard ragt aus dem Panel heraus: {lage}");
    pp!("H.die_seite_liegt_im_ki_panel", s.finde("#rf-input").is_some(), "das Eingabefeld fehlt in der Breite");
}

// ---- dialog_kanal: ein Eingabefeld, ein Knopf, ein Aufruf -----------------------------------------------------------

const ERKLAERUNG: &str = "Arbeitstage sind die Tage, an denen du tatsächlich zu deiner ersten Tätigkeitsstätte gefahren bist. Urlaub, Krankheit und Homeoffice zählen nicht mit.";

/// Der Dialog-Kanal ohne Browser, durch den ECHTEN `/chat` des Dienstes gegen den Ersatz fuer das Sprachmodell: beide
/// Haelften (Werte und Antwort) kommen an, jede darf leer sein, und die Antwort wird nirgends gespeichert.
#[test]
fn fluss_h_dialog_kanal_http() {
    let _sperre = crate::browser::seriell();
    let modell = ModellErsatz::starte(json!({}));
    let d = Dienst::starte(&Start { llm_basis: Some(modell.basis.clone()), ..Start::default() });
    let fall = d.fall("gesamt", "dk1");
    let dialog = |vs: &[Value], antwort: &str, unsicher: bool| {
        modell.setze_dialog(json!({
            "vorschlaege": vs.iter().map(|v| json!({"feld_id": v["feld_id"], "wert": v["wert"], "beleg": v["beleg"], "begruendung": "egal", "aussage": 0, "rechenweg": null})).collect::<Vec<_>>(),
            "rueckfragen": [], "antwort": antwort, "unsicher": unsicher,
        }));
    };
    let chat = |text: &str, feld: Option<&str>| {
        let mut rumpf = json!({"text": text});
        if let Some(f) = feld {
            rumpf["feld_id"] = json!(f);
        }
        let (st, body) = d.post(&format!("/fall/{fall}/chat"), &rumpf);
        assert_eq!(st, 200, "chat: {body}");
        body
    };
    let ereignisse = || d.akte(&fall)["events"].as_array().map_or(0, Vec::len);
    let feld_ids = |b: &Value| b["vorschlaege"].as_array().map(|a| a.iter().map(|v| v["feld_id"].clone()).collect::<Vec<_>>()).unwrap_or_default();

    // Eine reine Frage liefert die Antwort und keinen Wert; sie schreibt nichts.
    dialog(&[], ERKLAERUNG, false);
    let vorher = ereignisse();
    let b = chat("Zählt ein Tag im Homeoffice als Arbeitstag?", Some("ep_arbeitstage"));
    pp!("H.reine_frage_liefert_antwort_und_keinen_wert", b["antwort"] == json!(ERKLAERUNG), "die Antwort kommt nicht an: {b}");
    pp!("H.reine_frage_liefert_antwort_und_keinen_wert", feld_ids(&b).is_empty(), "eine Frage hat einen Vorschlag erzeugt: {b}");
    pp!("H.reine_frage_liefert_antwort_und_keinen_wert", ereignisse() == vorher, "eine Frage hat etwas geschrieben");

    // Die Gegenrichtung: eine reine Angabe liefert Werte und keinen erfundenen Belehrungstext.
    dialog(&[json!({"feld_id": "ep_arbeitstage", "wert": 220, "beleg": "220 Tagen"})], "", false);
    let b = chat("Ich fahre an 220 Tagen zur Arbeit.", Some("ep_arbeitstage"));
    pp!("H.reine_angabe_liefert_werte_und_keine_antwort", feld_ids(&b) == vec![json!("ep_arbeitstage")], "der Wert kommt nicht an: {b}");
    pp!("H.reine_angabe_liefert_werte_und_keine_antwort", b["antwort"] == json!(""), "ohne Frage steht trotzdem ein Antworttext da: {b}");

    // Ein Satz, der beides ist: beide Haelften kommen an, der Wert nur als Vorschlag.
    dialog(&[json!({"feld_id": "ep_entfernung_km", "wert": 15, "beleg": "15 km"})], ERKLAERUNG, false);
    let b = chat("Ich fahre 15 km zur Arbeit — zählt Homeoffice als Arbeitstag?", Some("ep_arbeitstage"));
    pp!("H.beides_in_einem_satz", feld_ids(&b) == vec![json!("ep_entfernung_km")], "der Wert kommt nicht an: {b}");
    pp!("H.beides_in_einem_satz", b["antwort"] == json!(ERKLAERUNG), "die Antwort kommt nicht an: {b}");
    pp!("H.beides_in_einem_satz", d.ereignisse(&fall, "ep_entfernung_km").last().is_some_and(|e| e["zustand"] == json!("vorlaeufig")), "der Wert ist nicht nur vorlaeufig geschrieben: {:?}", d.ereignisse(&fall, "ep_entfernung_km"));

    // Ein Modell, das im Fliesstext behauptet, etwas einzutragen, aendert nichts.
    dialog(&[], "Ich habe dir 220 Arbeitstage und 62000 Euro Bruttolohn eingetragen.", false);
    let vorher = ereignisse();
    let b = chat("Trag mir bitte alles ein.", Some("ep_arbeitstage"));
    pp!("H.antwort_wird_nicht_gespeichert", b["antwort"].as_str().is_some_and(|a| !a.is_empty()), "die Antwort kommt nicht an: {b}");
    pp!("H.antwort_wird_nicht_gespeichert", ereignisse() == vorher, "der Fliesstext der Antwort hat ein Ereignis geschrieben");

    // `unsicher` kommt bis zur Oberflaeche durch (sonst sieht eine Vermutung aus wie eine Auskunft).
    dialog(&[], ERKLAERUNG, true);
    let b = chat("Gilt das auch für mich?", Some("ep_arbeitstage"));
    pp!("H.unsicher_kommt_durch", b["unsicher"] == json!(true), "das Flag verschwindet auf dem Weg: {b}");

    // Das Beleg-Gate haengt im Dialog-Pfad: ein Vorschlag, dessen Beleg im Text nicht steht, wird weder gezeigt noch geschrieben.
    dialog(
        &[
            json!({"feld_id": "ep_arbeitstage", "wert": 220, "beleg": "an 220 Tagen"}),
            json!({"feld_id": CENT_FELD, "wert": 6_200_000, "beleg": "62000 Euro brutto"}),
        ],
        "",
        false,
    );
    // (Ein frischer Fall: in `dk1` steht `ep_arbeitstage` schon, derselbe Vorschlag liefe dort als Konflikt.)
    let f3 = d.fall("gesamt", "dk3");
    let (st, b) = d.post(&format!("/fall/{f3}/chat"), &json!({"text": "Ich fahre an 220 Tagen zur Arbeit."}));
    assert_eq!(st, 200, "chat: {b}");
    pp!("H.beleg_gate_ist_im_dialog_pfad_verdrahtet", feld_ids(&b) == vec![json!("ep_arbeitstage")], "der unbelegte Vorschlag kam durch: das Beleg-Gate haengt nicht im Pfad: {b}");
    pp!("H.beleg_gate_ist_im_dialog_pfad_verdrahtet", d.ereignisse(&f3, CENT_FELD).is_empty(), "der unbelegte Vorschlag wurde sogar geschrieben");

    // Ohne Schluessel bleibt es bei der Erklaer-Grenze: kein Netz, kein erfundener Text, 501.
    let ohne = Dienst::starte(&Start::default());
    let f2 = ohne.fall("gesamt", "dk2");
    let (st, b) = ohne.post(&format!("/fall/{f2}/chat"), &json!({"text": "Was sind Arbeitstage?", "feld_id": "ep_arbeitstage"}));
    pp!("H.ohne_key_bleibt_es_bei_der_erklaer_grenze", st == 501 && b["fehler"] == json!("not_implemented"), "ohne Schluessel kam {st} {b}");
}

#[test]
fn fluss_h_dialog_kanal() {
    let _sperre = crate::browser::seriell();
    // Der ECHTE `/chat` des Dienstes gegen den lokalen Ersatz; er antwortet mit einer Erklaerung, `unsicher: true`, ohne Werte.
    let modell = ModellErsatz::starte(json!({"vorschlaege": [], "rueckfragen": [], "antwort": ERKLAERUNG, "unsicher": true}));
    let u = umgebung_oder_ende!(Start { llm_basis: Some(modell.basis.clone()), ..Start::default() }, 360, 780);
    let (s, d) = (&u.s, &u.dienst);

    // Der Berater ist ohne Klick da, mit genau einem Absende-Knopf; das Chat-Modal gibt es nicht mehr.
    frisch(&u, None);
    pp!("H.berater_ist_ohne_klick_da_mit_genau_einem_knopf", s.sichtbar("#berater"), "der Berater ist nicht sichtbar");
    pp!("H.berater_ist_ohne_klick_da_mit_genau_einem_knopf", s.sichtbar("#chat-text"), "das Eingabefeld ist nicht erreichbar");
    pp!("H.berater_ist_ohne_klick_da_mit_genau_einem_knopf", s.finde("#chat-overlay").is_none(), "das Chat-Modal existiert noch: es lag zuletzt ueber der ganzen Oberflaeche");
    pp!("H.berater_ist_ohne_klick_da_mit_genau_einem_knopf", s.finde("#chat-frage").is_none(), "es gibt noch einen zweiten Knopf");
    pp!("H.berater_ist_ohne_klick_da_mit_genau_einem_knopf", s.sichtbar("#wegpunkt"), "der Fragefluss ist verdeckt");

    // „Erklaer mir“ erklaert sofort: Fragetext, Kurzhilfe, wortlicher Gesetzestext. Es ruft das Modell NICHT auf (Geld).
    frisch(&u, None);
    let vorher = modell.aufrufe();
    s.klick("#chat");
    s.warte_sichtbar(".chat-frage-titel", 5);
    pp!("H.erklaer_mir_erklaert_sofort_und_ohne_modell", !s.text(".chat-frage-titel").trim().is_empty(), "kein Fragetext in der Erklaerung");
    pp!("H.erklaer_mir_erklaert_sofort_und_ohne_modell", s.finde(".chat-gesetz").is_some(), "kein Gesetzestext in der Erklaerung: der Zitatanker fehlt");
    pp!("H.erklaer_mir_erklaert_sofort_und_ohne_modell", s.textinhalt(".chat-gesetz").contains('§'), "der Gesetzestext nennt keinen Paragrafen: {:?}", s.textinhalt(".chat-gesetz"));
    pp!("H.erklaer_mir_erklaert_sofort_und_ohne_modell", modell.aufrufe() == vorher, "„Erklaer mir“ hat das Sprachmodell aufgerufen ({} Anfragen): jeder Aufruf kostet", modell.aufrufe() - vorher);

    // Eine Rueckfrage danach bleibt im selben Verlauf: die Erklaerung bleibt stehen, Frage und Antwort kommen darunter.
    s.fuelle("#chat-text", "Und zählt Homeoffice dazu?");
    s.klick("#chat-send");
    s.warte_sichtbar(".chat-antwort", 10);
    pp!("H.rueckfrage_nach_erklaer_mir_bleibt_im_selben_verlauf", s.finde(".chat-frage-titel").is_some(), "die Erklaerung wurde von der Rueckfrage ueberschrieben: der Bezug ist weg");
    pp!("H.rueckfrage_nach_erklaer_mir_bleibt_im_selben_verlauf", s.finde(".chat-du").is_some(), "die eigene Frage steht nicht im Verlauf");
    pp!("H.rueckfrage_nach_erklaer_mir_bleibt_im_selben_verlauf", s.textinhalt(".chat-antwort").contains("Arbeitstage sind"), "die Antwort des Modells steht nicht da: {:?}", s.textinhalt(".chat-antwort"));
    pp!("H.rueckfrage_nach_erklaer_mir_bleibt_im_selben_verlauf", s.finde(".chat-unsicher").is_some(), "unsicher=true wird dem Nutzer nicht angezeigt: eine Vermutung sieht aus wie eine Auskunft");
    pp!("H.rueckfrage_nach_erklaer_mir_bleibt_im_selben_verlauf", s.wert("#chat-text").is_empty(), "das Eingabefeld wurde nicht geleert");

    // Eine Frage veraendert den Fall nicht: derselbe Satz „Trag mir 220 Tage ein“ geht durch die ganze Kette und
    // schreibt nichts (weder neue Felder noch neue Ereignisse).
    frisch(&u, None);
    let fall = u.fall();
    let felder_vorher = u.stand_felder(&fall);
    let ereignisse_vorher = d.akte(&fall)["events"].as_array().map_or(0, Vec::len);
    let anfragen_vorher = modell.aufrufe();
    s.fuelle("#chat-text", "Trag mir bitte 220 Arbeitstage und 62000 Euro Bruttolohn ein.");
    s.klick("#chat-send");
    s.warte_sichtbar(".chat-antwort", 10);
    pp!("H.frage_veraendert_den_fall_nicht", modell.aufrufe() > anfragen_vorher, "die Frage hat das Modell gar nicht erreicht: der Test misst sonst nichts");
    let felder_nachher = u.stand_felder(&fall);
    let schluessel = |v: &Value| v.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default();
    pp!("H.frage_veraendert_den_fall_nicht", schluessel(&felder_nachher) == schluessel(&felder_vorher), "der Fall hat sich veraendert: {:?}", schluessel(&felder_nachher));
    pp!("H.frage_veraendert_den_fall_nicht", d.akte(&fall)["events"].as_array().map_or(0, Vec::len) == ereignisse_vorher, "eine Frage hat ein Ereignis geschrieben");
}
