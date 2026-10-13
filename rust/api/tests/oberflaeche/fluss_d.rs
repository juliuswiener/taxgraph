//! Fluss D: eine Antwort aendern, mehrere Instanzen (zweites Kind), Korrektur ueber die Belegt-Liste.
//!
//! Gegenstueck zu `tests/test_ui_instanzen.py` (5 Browser-Funktionen) und `tests/test_ui_instanz_korrektur.py` (14).
//! Die acht uebrigen Funktionen von `test_ui_instanzen.py` sind Python-intern (`instanz_anzahl`, Cache): siehe
//! `ZUORDNUNG.tsv`. Daten: die Werte gehen als `kind_vorname`, `kind_vorname__2`, … in den Store, und der ELSTER-Writer
//! findet nur diese Kennungen. Ein falsch getroffene Instanz oder eine stille Null waere ein stiller Datenverlust.

use serde_json::{json, Value};

use crate::dienst::Start;
use crate::seite::{zum_fragebogen, Umgebung};

/// Ein frischer Fall im Fragebogen, „Kinder“ bejaht (sonst nimmt die Ankreuzliste den Block aus dem Dialog).
fn szenario(u: &Umgebung) {
    u.oeffne_start();
    u.fall_beginnen("fragebogen");
    zum_fragebogen(&u.s, Some(&["kein_kind"]));
    let _ = u.konsolenfehler();
}

fn banner(u: &Umgebung) -> String {
    u.s.textinhalt("#netz-banner").trim().to_owned()
}

/// Ein Ereignis auf dem Weg der Oberflaeche (`POST /event`), danach `refresh()`. Liefert den Status.
fn schreibe(u: &Umgebung, fid: &str, wert: Value, zustand: &str) -> u64 {
    let r = u.s.auswerten(
        "async ([fid, wert, zustand]) => { const r = await jpost(`/fall/${FALL}/event`, { feld_id: fid, wert, zustand, \
         herkunft: {herkunft: zustand === 'vorlaeufig' ? 'llm_vorschlag' : 'laie', pruef_tiefe: 'ungeprueft', haftung: 'nutzer'}, \
         schreiber: zustand === 'vorlaeufig' ? 'llm:chat' : 'ui:laie', \
         signal: {signal_1: null, signal_2: zustand === 'vorlaeufig' ? null : 'klick@' + fid}}); await refresh(); return r.status; }",
        &[json!([fid, wert, zustand])],
    );
    r.as_u64().unwrap_or(0)
}

fn ok(status: u64) -> bool {
    status == 200 || status == 201
}

/// Die Frage `feld` zeigen, unabhaengig von der Queue; liefert `{anzahl, etikett}` oder Null.
fn zeige(u: &Umgebung, feld: &str) -> Value {
    u.s.auswerten(
        "async (fid) => { const r = await jget(`/fall/${FALL}/fragen`); const q = (r.body.fragen || []).find(x => x.feld_id === fid); \
         if (!q) return null; AKTUELL = q; zeigeFrage(q, STAND); return {anzahl: q.instanz_anzahl, etikett: q.instanz_etikett}; }",
        &[json!(feld)],
    )
}

fn setze_kinderzahl(u: &Umgebung, n: u64) -> u64 {
    let r = u.s.auswerten(
        "async (n) => { const r = await jpost(`/fall/${FALL}/event`, { feld_id: 'fam_anzahl_kinder', wert: n, zustand: 'bestaetigt', \
         herkunft: {herkunft: 'laie', pruef_tiefe: 'ungeprueft', haftung: 'nutzer'}, schreiber: 'ui:laie', \
         signal: {signal_1: null, signal_2: 'klick@fam_anzahl_kinder'}}); if (r.status < 200 || r.status >= 300) return r.status; \
         await refresh(); return r.status; }",
        &[json!(n)],
    );
    r.as_u64().unwrap_or(0)
}

fn klick_belegt(u: &Umgebung, fid: &str) -> bool {
    u.s.auswerten(
        "(fid) => { const li = [...document.querySelectorAll('#belegt-liste li')].find(x => x.querySelector('.z-name').title === fid); \
         if (!li) return false; li.click(); return true; }",
        &[json!(fid)],
    ) == json!(true)
}

fn zeilen_werte(u: &Umgebung) -> Value {
    u.s.ausdruck("[...document.querySelectorAll('#eingabe .instanz-zeile input')].map(el => el.value)")
}

fn setze_zeile(u: &Umgebung, i: usize, text: &str) {
    u.s.skript(
        "const els = [...document.querySelectorAll('#eingabe .instanz-zeile input')]; els[arguments[0]].value = arguments[1];",
        &[json!(i), json!(text)],
    );
}

fn warte_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

#[test]
fn fluss_d_mehrere_instanzen_anlegen() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let s = &u.s;

    // Drei Kinder = drei Eingabefelder mit Nummer; die Werte landen unter base, base__2, base__3.
    szenario(&u);
    let fall = u.fall();
    pp!("D.drei_kinder_geben_drei_eingabefelder", ok(setze_kinderzahl(&u, 3)), "fam_anzahl_kinder=3 wird abgewiesen");
    let q = zeige(&u, "kind_vorname");
    pp!("D.drei_kinder_geben_drei_eingabefelder", q["anzahl"] == json!(3), "der Dienst meldet {q} statt drei Instanzen");
    let m = s.ausdruck(
        "(() => { const f = [...document.querySelectorAll('#eingabe .instanz-zeile')]; return {zeilen: f.length, \
         marken: f.map(z => z.querySelector('.instanz-marke').textContent), ids: f.map(z => (z.querySelector('input, select') || {}).id)}; })()",
    );
    pp!("D.drei_kinder_geben_drei_eingabefelder", m["zeilen"] == json!(3), "nur {} Eingabefelder: {m}", m["zeilen"]);
    pp!("D.drei_kinder_geben_drei_eingabefelder", m["marken"] == json!(["Kind 1", "Kind 2", "Kind 3"]), "Marken: {}", m["marken"]);
    pp!("D.drei_kinder_geben_drei_eingabefelder", m["ids"].as_array().unwrap().iter().all(|i| i.as_str().is_some_and(|x| !x.is_empty())), "ein Feld ohne id: {}", m["ids"]);
    let fehler = u.konsolenfehler();
    pp!("D.drei_kinder_geben_drei_eingabefelder", fehler.is_empty(), "Konsolenfehler: {fehler:?}");
    s.skript(
        "const namen = ['Anna', 'Ben', 'Cem']; [...document.querySelectorAll('#eingabe .instanz-zeile input')].forEach((el, i) => { el.value = namen[i]; });",
        &[],
    );
    s.klick("#bestaetigen");
    s.warte_bis("STAND && STAND.felder && STAND.felder['kind_vorname__3'] !== undefined", 8, "Stand kennt kind_vorname__3");
    let felder = u.stand_felder(&fall);
    pp!("D.die_werte_landen_unter_base_und_base__n", felder["kind_vorname"]["wert"] == json!("Anna"), "Instanz 1 fehlt oder steht falsch: {}", felder["kind_vorname"]);
    pp!("D.die_werte_landen_unter_base_und_base__n", felder["kind_vorname__2"]["wert"] == json!("Ben"), "Instanz 2 fehlt: das war die Luecke");
    pp!("D.die_werte_landen_unter_base_und_base__n", felder["kind_vorname__3"]["wert"] == json!("Cem"), "Instanz 3 fehlt");
    for f in ["kind_vorname", "kind_vorname__2", "kind_vorname__3"] {
        pp!("D.die_werte_landen_unter_base_und_base__n", felder[f]["zustand"] == json!("bestaetigt"), "{f} nicht bestaetigt");
    }

    // Bei einem Kind bleibt es ein schlichtes Feld.
    szenario(&u);
    pp!("D.ein_kind_bleibt_ein_schlichtes_feld", ok(setze_kinderzahl(&u, 1)), "fam_anzahl_kinder=1 wird abgewiesen");
    let q = zeige(&u, "kind_vorname");
    pp!("D.ein_kind_bleibt_ein_schlichtes_feld", q["anzahl"] == json!(1), "Anzahl {}", q["anzahl"]);
    pp!("D.ein_kind_bleibt_ein_schlichtes_feld", s.anzahl("#eingabe .instanz-zeile") == 0, "bei einem Kind stehen Instanz-Zeilen da");

    // Eine leere Instanz wird uebersprungen, nicht als Null geschrieben; die dritte geht nicht verloren.
    szenario(&u);
    let fall = u.fall();
    pp!("D.eine_leere_instanz_wird_uebersprungen_nicht_verworfen", ok(setze_kinderzahl(&u, 3)), "fam_anzahl_kinder=3 wird abgewiesen");
    zeige(&u, "kind_vorname");
    setze_zeile(&u, 0, "Anna");
    setze_zeile(&u, 2, "Cem");
    s.klick("#bestaetigen");
    s.warte_bis("STAND && STAND.felder && STAND.felder['kind_vorname__3'] !== undefined", 8, "Stand kennt kind_vorname__3");
    let felder = u.stand_felder(&fall);
    pp!("D.eine_leere_instanz_wird_uebersprungen_nicht_verworfen", felder["kind_vorname"]["wert"] == json!("Anna"), "Instanz 1: {}", felder["kind_vorname"]);
    pp!("D.eine_leere_instanz_wird_uebersprungen_nicht_verworfen", felder.get("kind_vorname__2").is_none(), "die leere Instanz wurde geschrieben (stille Null)");
    pp!("D.eine_leere_instanz_wird_uebersprungen_nicht_verworfen", felder["kind_vorname__3"]["wert"] == json!("Cem"), "die dritte Instanz ging mit der leeren zweiten verloren");

    // Gar keine Eingabe schreibt nichts und sagt es.
    szenario(&u);
    let fall = u.fall();
    pp!("D.alle_leer_schreibt_nichts_und_sagt_es", ok(setze_kinderzahl(&u, 2)), "fam_anzahl_kinder=2 wird abgewiesen");
    zeige(&u, "kind_vorname");
    let vorher: Vec<String> = u.stand_felder(&fall).as_object().unwrap().keys().cloned().collect();
    s.klick("#bestaetigen");
    warte_ms(800);
    let nachher: Vec<String> = u.stand_felder(&fall).as_object().unwrap().keys().cloned().collect();
    pp!("D.alle_leer_schreibt_nichts_und_sagt_es", vorher == nachher, "leere Felder haben etwas geschrieben");
    pp!("D.alle_leer_schreibt_nichts_und_sagt_es", !banner(&u).is_empty(), "der Klick tat nichts und sagte nichts");
}

#[test]
fn fluss_d_korrektur_der_zweiten_instanz() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let s = &u.s;

    szenario(&u);
    // Die Zerlegung `basisFeldId` misst sich am Hinweg `instanzFeldId`.
    let r = s.ausdruck(
        "(() => { const f = ['kind_vorname', 'kind_vorname__2', 'kind_vorname__10', 'kind_vorname__1', 'kind_vorname__02', \
         'kind_vorname__0', 'kind_vorname__', 'kind_vorname__2x', 'kind_vorname__2__3']; \
         return Object.fromEntries(f.map(x => [x, basisFeldId(x)])); })()",
    );
    pp!("D.die_zerlegung_misst_sich_am_hinweg", r["kind_vorname"] == json!({"basis": "kind_vorname", "instanz": 1}), "{}", r["kind_vorname"]);
    pp!("D.die_zerlegung_misst_sich_am_hinweg", r["kind_vorname__2"] == json!({"basis": "kind_vorname", "instanz": 2}), "{}", r["kind_vorname__2"]);
    pp!("D.die_zerlegung_misst_sich_am_hinweg", r["kind_vorname__10"] == json!({"basis": "kind_vorname", "instanz": 10}), "{}", r["kind_vorname__10"]);
    pp!("D.die_zerlegung_misst_sich_am_hinweg", r["kind_vorname__1"] == json!({"basis": "kind_vorname__1", "instanz": 1}), "__1 ist keine Instanz: {}", r["kind_vorname__1"]);
    for kaputt in ["kind_vorname__02", "kind_vorname__0", "kind_vorname__", "kind_vorname__2x"] {
        pp!("D.die_zerlegung_misst_sich_am_hinweg", r[kaputt] == json!({"basis": kaputt, "instanz": 1}), "{kaputt} wurde als Instanz gelesen: {}", r[kaputt]);
    }
    pp!("D.die_zerlegung_misst_sich_am_hinweg", r["kind_vorname__2__3"] == json!({"basis": "kind_vorname__2", "instanz": 3}), "{}", r["kind_vorname__2__3"]);
    let rund = s.ausdruck(
        "(() => [1, 2, 3, 17].map(i => { const fid = instanzFeldId('kind_vorname', i); const z = basisFeldId(fid); return [i, fid, z.basis, z.instanz]; }))()",
    );
    for z in rund.as_array().unwrap() {
        pp!("D.die_zerlegung_misst_sich_am_hinweg", z[2] == json!("kind_vorname") && z[3] == z[0], "Rundweg kaputt bei Instanz {}: {z}", z[0]);
    }

    // Beide Kinder bestaetigt, dann die zweite Zeile korrigieren.
    let fall = u.fall();
    assert!(ok(schreibe(&u, "fam_anzahl_kinder", json!(2), "bestaetigt")));
    assert!(ok(schreibe(&u, "kind_vorname", json!("Anna"), "bestaetigt")));
    assert!(ok(schreibe(&u, "kind_vorname__2", json!("Ben"), "bestaetigt")));
    let vorher = u.stand_felder(&fall);
    pp!("D.klick_auf_die_zweite_instanz_oeffnet_die_frage_mit_beiden_zeilen", klick_belegt(&u, "kind_vorname__2"), "kind_vorname__2 steht nicht in der Belegt-Liste");
    s.warte_sichtbar("#eingabe .instanz-zeile", 8);
    pp!("D.klick_auf_die_zweite_instanz_oeffnet_die_frage_mit_beiden_zeilen", !banner(&u).contains("entfallen"), "die alte Fehlmeldung steht da: {}", banner(&u));
    let m = s.ausdruck(
        "({feld: AKTUELL.feld_id, werte: [...document.querySelectorAll('#eingabe .instanz-zeile input')].map(el => el.value), \
         marken: [...document.querySelectorAll('#eingabe .instanz-marke')].map(el => el.textContent)})",
    );
    pp!("D.klick_auf_die_zweite_instanz_oeffnet_die_frage_mit_beiden_zeilen", m["feld"] == json!("kind_vorname"), "die Korrektur laeuft auf {} statt auf dem Basisfeld", m["feld"]);
    pp!("D.klick_auf_die_zweite_instanz_oeffnet_die_frage_mit_beiden_zeilen", m["werte"] == json!(["Anna", "Ben"]), "die gegebenen Antworten stehen nicht in den Zeilen: {}", m["werte"]);
    pp!("D.klick_auf_die_zweite_instanz_oeffnet_die_frage_mit_beiden_zeilen", m["marken"] == json!(["Kind 1", "Kind 2"]), "Marken: {}", m["marken"]);
    let fehler = u.konsolenfehler();
    pp!("D.klick_auf_die_zweite_instanz_oeffnet_die_frage_mit_beiden_zeilen", fehler.is_empty(), "Konsolenfehler: {fehler:?}");
    // Nur die zweite Zeile aendern: sie trifft genau ihre Instanz, die erste bleibt; alle gefuellten Zeilen werden neu geschrieben.
    setze_zeile(&u, 1, "Bernd");
    s.klick("#bestaetigen");
    s.warte_bis("STAND && STAND.felder && STAND.felder['kind_vorname__2'] && STAND.felder['kind_vorname__2'].wert === 'Bernd'", 8, "Stand zeigt Bernd");
    let felder = u.stand_felder(&fall);
    pp!("D.die_geaenderte_zweite_zeile_trifft_genau_ihre_instanz", felder["kind_vorname__2"]["wert"] == json!("Bernd"), "die Korrektur kam nicht an: {}", felder["kind_vorname__2"]);
    pp!("D.die_geaenderte_zweite_zeile_trifft_genau_ihre_instanz", felder["kind_vorname__2"]["zustand"] == json!("bestaetigt"), "Zustand nicht bestaetigt");
    pp!("D.die_geaenderte_zweite_zeile_trifft_genau_ihre_instanz", felder["kind_vorname__2"]["event_id"] != vorher["kind_vorname__2"]["event_id"], "dasselbe Ereignis: es wurde nichts geschrieben");
    pp!("D.die_geaenderte_zweite_zeile_trifft_genau_ihre_instanz", felder["kind_vorname"]["wert"] == json!("Anna"), "Kind 1 hat den Wert von Kind 2 mitbekommen: {}", felder["kind_vorname"]);
    pp!("D.die_geaenderte_zweite_zeile_trifft_genau_ihre_instanz", felder["kind_vorname"]["zustand"] == json!("bestaetigt"), "Kind 1 nicht mehr bestaetigt");
    pp!("D.eine_korrektur_schreibt_auch_die_unveraenderten_zeilen_neu", felder["kind_vorname"]["wert"] == vorher["kind_vorname"]["wert"], "der Wert von Kind 1 hat sich geaendert");
    pp!(
        "D.eine_korrektur_schreibt_auch_die_unveraenderten_zeilen_neu",
        felder["kind_vorname"]["event_id"] != vorher["kind_vorname"]["event_id"],
        "Kind 1 traegt noch sein altes Ereignis: der Weg schreibt nur noch die geaenderte Zeile"
    );

    // Eine Regel mit offenen Gates sperrt die Korrektur nicht (`unentschieden` ist der Normalfall).
    let lage = s.auswerten(
        "async () => { const q = (await jget(`/fall/${FALL}/feld/kind_vorname__2/frage`)).body.frage || {}; \
         const st = (await jget(`/fall/${FALL}/stand`)).body; return {regel_id: q.regel_id, status: ((st.relevanz || {})[q.regel_id] || {}).status}; }",
        &[],
    );
    pp!("D.eine_regel_mit_offenen_gates_sperrt_die_korrektur_nicht", lage["status"] == json!("unentschieden"), "Status ist {} statt 'unentschieden': der Aufbau trifft den Fall nicht mehr", lage["status"]);
    let okk = s.auswerten("async () => await korrigiereBestaetigt('kind_vorname__2')", &[]);
    warte_ms(600);
    pp!("D.eine_regel_mit_offenen_gates_sperrt_die_korrektur_nicht", okk == json!(true), "eine Regel mit offenen Gates sperrt die Korrektur. Banner: {}", banner(&u));
    pp!("D.eine_regel_mit_offenen_gates_sperrt_die_korrektur_nicht", !banner(&u).contains("entfallen"), "Banner: {}", banner(&u));

    // Das Gate umdrehen („keine Kinder“): die Frage ist abgeschaltet und bekommt den Hinweis statt der Frage.
    let vor = s.auswerten("async () => (await jget(`/fall/${FALL}/stand`)).body.felder.kein_kind", &[]);
    let neu = !vor["wert"].as_bool().unwrap_or(false);
    let status = s.auswerten(
        "async (neu) => { const ev = {feld_id: 'kein_kind', wert: neu, zustand: 'bestaetigt', \
         herkunft: {herkunft: 'laie', pruef_tiefe: 'ungeprueft', haftung: 'nutzer'}, schreiber: 'ui:laie', signal: {signal_1: null, signal_2: 'klick@kein_kind'}}; \
         const j = (await jget(`/fall/${FALL}/feld/kein_kind/warum`)).body.justification || {}; \
         if (j.event_id) { ev.ersetzt = j.event_id; ev.signal.signal_1 = j.event_id; } \
         const r = await jpost(`/fall/${FALL}/event`, ev); await refresh(); return r.status; }",
        &[json!(neu)],
    );
    assert!(ok(status.as_u64().unwrap_or(0)), "kein_kind drehen: {status}");
    let lage = s.auswerten(
        "async () => { const f = (await jget(`/fall/${FALL}/fragen`)).body.fragen || []; const st = (await jget(`/fall/${FALL}/stand`)).body; \
         const q = (await jget(`/fall/${FALL}/feld/kind_vorname__2/frage`)).body.frage || {}; \
         return {in_fragen: f.some(x => x.feld_id === 'kind_vorname'), zustand: (st.felder.kind_vorname || {}).zustand, \
         regel_status: ((st.relevanz || {})[q.regel_id] || {}).status, endpunkt_liefert_frage: !!q.feld_id}; }",
        &[],
    );
    pp!(
        "D.eine_abgeschaltete_frage_bekommt_den_hinweis_statt_der_frage",
        lage["zustand"] == json!("bestaetigt") && lage["in_fragen"] == json!(false) && lage["endpunkt_liefert_frage"] == json!(true),
        "der Aufbau stimmt nicht: {lage}"
    );
    pp!("D.eine_abgeschaltete_frage_bekommt_den_hinweis_statt_der_frage", lage["regel_status"] == json!("ausgeschlossen"), "die Regel gilt nicht als ausgeschlossen: {lage}");
    let vorher_text = s.textinhalt("#frage");
    let okk = s.auswerten("async () => await korrigiereBestaetigt('kind_vorname__2')", &[]);
    warte_ms(600);
    pp!("D.eine_abgeschaltete_frage_bekommt_den_hinweis_statt_der_frage", okk == json!(false), "die abgeschaltete Frage wurde vorgelegt");
    pp!("D.eine_abgeschaltete_frage_bekommt_den_hinweis_statt_der_frage", banner(&u).contains("entfallen"), "der Hinweis nennt den Grund nicht: {}", banner(&u));
    pp!("D.eine_abgeschaltete_frage_bekommt_den_hinweis_statt_der_frage", banner(&u).contains("abgeschaltet"), "der Hinweis sagt nicht, wie man sie zurueckholt: {}", banner(&u));
    pp!("D.eine_abgeschaltete_frage_bekommt_den_hinweis_statt_der_frage", s.textinhalt("#frage") == vorher_text, "die Frage im Bild wurde ersetzt");

    // Die zweite Zeile, dann die Gegenrichtung (beide Zeilen aendern), mit einem KI-Vorschlag in Zeile 1 (Halte-Geste).
    szenario(&u);
    let fall = u.fall();
    assert!(ok(schreibe(&u, "fam_anzahl_kinder", json!(2), "bestaetigt")));
    assert!(ok(schreibe(&u, "kind_vorname", json!("Anna"), "vorlaeufig")));
    assert!(ok(schreibe(&u, "kind_vorname__2", json!("Ben"), "bestaetigt")));
    let z = s.auswerten(
        "() => { const li = [...document.querySelectorAll('#belegt-liste li')].find(x => x.querySelector('.z-name').title === 'kind_vorname__2'); \
         return li ? {name: li.querySelector('.z-name').textContent, wert: li.querySelector('.z-wert').textContent} : null; }",
        &[],
    );
    pp!("D.die_instanz_zeile_ist_lesbar_statt_kennung_und_rohwert", !z.is_null(), "die Zeile fehlt in der Belegt-Liste");
    let name = z["name"].as_str().unwrap_or_default().to_owned();
    pp!("D.die_instanz_zeile_ist_lesbar_statt_kennung_und_rohwert", name != "kind_vorname__2", "die Zeile zeigt weiter die Kennung");
    pp!("D.die_instanz_zeile_ist_lesbar_statt_kennung_und_rohwert", name.contains("Vornamen"), "das ist nicht die Frage des Feldes: {name}");
    pp!("D.die_instanz_zeile_ist_lesbar_statt_kennung_und_rohwert", name.contains('2'), "ohne Nummer stehen zwei gleich benannte Zeilen untereinander: {name}");
    pp!("D.die_instanz_zeile_ist_lesbar_statt_kennung_und_rohwert", z["wert"] == json!("Ben"), "der Wert steht als Rohwert da: {}", z["wert"]);
    let vorher = u.stand_felder(&fall);
    assert!(klick_belegt(&u, "kind_vorname__2"));
    s.warte_sichtbar("#eingabe .instanz-zeile", 8);
    setze_zeile(&u, 0, "Anne");
    setze_zeile(&u, 1, "Bernd");
    s.halte("#bestaetigen", 800);
    s.warte_bis("STAND && STAND.felder && STAND.felder['kind_vorname'] && STAND.felder['kind_vorname'].wert === 'Anne'", 8, "Stand zeigt Anne");
    let felder = u.stand_felder(&fall);
    pp!("D.die_erste_zeile_laesst_sich_ueber_dieselbe_korrektur_mitgeben", felder["kind_vorname"]["wert"] == json!("Anne"), "Kind 1: {}", felder["kind_vorname"]);
    pp!("D.die_erste_zeile_laesst_sich_ueber_dieselbe_korrektur_mitgeben", felder["kind_vorname"]["zustand"] == json!("bestaetigt"), "der Nutzer hat den Vorschlag selbst ueberschrieben: das ist eine Antwort");
    pp!("D.die_erste_zeile_laesst_sich_ueber_dieselbe_korrektur_mitgeben", felder["kind_vorname__2"]["wert"] == json!("Bernd"), "Kind 2: {}", felder["kind_vorname__2"]);
    let _ = vorher;

    // Eine geleerte vorbelegte Zeile loescht nichts und es wird kein Loeschen behauptet.
    szenario(&u);
    let fall = u.fall();
    assert!(ok(schreibe(&u, "fam_anzahl_kinder", json!(2), "bestaetigt")));
    assert!(ok(schreibe(&u, "kind_vorname", json!("Anna"), "vorlaeufig")));
    assert!(ok(schreibe(&u, "kind_vorname__2", json!("Ben"), "bestaetigt")));
    let vorher = u.stand_felder(&fall);
    assert!(klick_belegt(&u, "kind_vorname__2"));
    s.warte_sichtbar("#eingabe .instanz-zeile", 8);
    setze_zeile(&u, 1, "");
    s.halte("#bestaetigen", 800);
    warte_ms(1500);
    let nachher = u.stand_felder(&fall);
    pp!("D.eine_geleerte_vorbelegte_zeile_loescht_nichts_und_sagt_das_nicht", nachher["kind_vorname__2"]["wert"] == json!("Ben"), "der Wert ist verschwunden");
    pp!(
        "D.eine_geleerte_vorbelegte_zeile_loescht_nichts_und_sagt_das_nicht",
        nachher["kind_vorname__2"]["event_id"] == vorher["kind_vorname__2"]["event_id"],
        "es wurde ein neues Ereignis geschrieben, obwohl die Zeile leer war"
    );
    pp!("D.eine_geleerte_vorbelegte_zeile_loescht_nichts_und_sagt_das_nicht", !banner(&u).to_lowercase().contains("gelöscht"), "es wird ein Loeschen behauptet, das nicht stattfand");

    // Drei Kinder, nur Kind 3 beantwortet: die Korrektur materialisiert keine leeren Instanzen.
    szenario(&u);
    let fall = u.fall();
    assert!(ok(schreibe(&u, "fam_anzahl_kinder", json!(3), "bestaetigt")));
    assert!(ok(schreibe(&u, "kind_vorname__3", json!("Cem"), "bestaetigt")));
    pp!("D.die_korrektur_materialisiert_keine_leeren_instanzen", klick_belegt(&u, "kind_vorname__3"), "die Zeile fehlt in der Belegt-Liste");
    s.warte_sichtbar("#eingabe .instanz-zeile", 8);
    pp!("D.die_korrektur_materialisiert_keine_leeren_instanzen", zeilen_werte(&u) == json!(["", "", "Cem"]), "die Vorbelegung sitzt in der falschen Zeile: {}", zeilen_werte(&u));
    setze_zeile(&u, 2, "Cemal");
    s.klick("#bestaetigen");
    s.warte_bis("STAND && STAND.felder && STAND.felder['kind_vorname__3'] && STAND.felder['kind_vorname__3'].wert === 'Cemal'", 8, "Stand zeigt Cemal");
    let felder = u.stand_felder(&fall);
    pp!("D.die_korrektur_materialisiert_keine_leeren_instanzen", felder["kind_vorname__3"]["wert"] == json!("Cemal"), "die Korrektur kam nicht an");
    for leer in ["kind_vorname", "kind_vorname__2"] {
        pp!("D.die_korrektur_materialisiert_keine_leeren_instanzen", felder.get(leer).is_none(), "{leer} wurde angelegt, obwohl die Zeile leer war");
    }

    // Felder ohne Achse gehen denselben Weg wie bisher; ein bestaetigtes Feld ist korrigierbar; ein Feld ohne Ereignis legt nichts vor.
    szenario(&u);
    assert!(ok(schreibe(&u, "veranlagung", json!("einzel"), "vorlaeufig")));
    let okk = s.auswerten("async () => await korrigiereBestaetigt('veranlagung')", &[]);
    warte_ms(500);
    pp!("D.ein_feld_ohne_achse_geht_denselben_weg_wie_bisher", okk == json!(true), "die Korrektur eines Feldes ohne Achse schlug fehl. Banner: {}", banner(&u));
    let m = s.ausdruck("({feld: AKTUELL.feld_id, zeilen: document.querySelectorAll('#eingabe .instanz-zeile').length})");
    pp!("D.ein_feld_ohne_achse_geht_denselben_weg_wie_bisher", m["feld"] == json!("veranlagung"), "AKTUELL steht auf {}", m["feld"]);
    pp!("D.ein_feld_ohne_achse_geht_denselben_weg_wie_bisher", m["zeilen"] == json!(0), "ein Feld ohne Achse hat Instanz-Zeilen bekommen");
    let fehler = u.konsolenfehler();
    pp!("D.ein_feld_ohne_achse_geht_denselben_weg_wie_bisher", fehler.is_empty(), "Konsolenfehler: {fehler:?}");
    assert!(ok(schreibe(&u, "fam_anzahl_kinder", json!(2), "bestaetigt")));
    let okk = s.auswerten("async () => await korrigiereBestaetigt('fam_anzahl_kinder')", &[]);
    warte_ms(500);
    pp!("D.ein_bestaetigtes_feld_ohne_achse_laesst_sich_jetzt_korrigieren", okk == json!(true), "die Korrektur eines bestaetigten Feldes schlaegt fehl. Banner: {}", banner(&u));
    pp!("D.ein_bestaetigtes_feld_ohne_achse_laesst_sich_jetzt_korrigieren", !banner(&u).contains("entfallen"), "Banner: {}", banner(&u));
    let m = s.ausdruck("({feld: AKTUELL.feld_id, frage: document.getElementById('frage').textContent})");
    pp!("D.ein_bestaetigtes_feld_ohne_achse_laesst_sich_jetzt_korrigieren", m["feld"] == json!("fam_anzahl_kinder"), "AKTUELL steht auf {}", m["feld"]);
    pp!("D.ein_bestaetigtes_feld_ohne_achse_laesst_sich_jetzt_korrigieren", m["frage"].as_str().is_some_and(|f| f.contains("Kinder")), "die falsche Frage liegt vor: {}", m["frage"]);
    // Ein einzelner Wert wird nicht buchstabenweise auf die Zeilen verteilt (fam_anzahl_kinder = 2).
    let r = s.auswerten(
        "async () => { const rr = await jget(`/fall/${FALL}/fragen`); const q = (rr.body.fragen || []).find(x => x.feld_id === 'kind_vorname'); \
         if (!q) return {fehler: 'kind_vorname nicht in /fragen'}; const box = document.getElementById('eingabe'); \
         baueEingabe(q, box, 'probe-input', 'frage', 'Anna'); const skalar = [...box.querySelectorAll('input')].map(el => el.value); \
         baueEingabe(q, box, 'probe-input', 'frage', {1: 'Anna', 2: 'Ben'}); const je_instanz = [...box.querySelectorAll('input')].map(el => el.value); \
         return {anzahl: q.instanz_anzahl, skalar, je_instanz}; }",
        &[],
    );
    pp!("D.ein_einzelner_wert_wird_nicht_buchstabenweise_auf_die_zeilen_verteilt", r["anzahl"] == json!(2), "der Aufbau stimmt nicht: {r}");
    pp!("D.ein_einzelner_wert_wird_nicht_buchstabenweise_auf_die_zeilen_verteilt", r["skalar"] == json!(["", ""]), "ein einzelner Wert wurde buchstabenweise verteilt: {}", r["skalar"]);
    pp!("D.ein_einzelner_wert_wird_nicht_buchstabenweise_auf_die_zeilen_verteilt", r["je_instanz"] == json!(["Anna", "Ben"]), "die Zuordnung je Instanz kommt nicht an: {}", r["je_instanz"]);
    let vorher_text = s.textinhalt("#frage");
    let okk = s.auswerten("async () => await korrigiereBestaetigt('gibt_es_dieses_feld_nicht')", &[]);
    warte_ms(500);
    pp!("D.ein_feld_ohne_ereignis_legt_keine_frage_vor", okk == json!(false), "fuer ein Feld ohne Ereignis wurde trotzdem eine Frage vorgelegt");
    pp!("D.ein_feld_ohne_ereignis_legt_keine_frage_vor", !banner(&u).is_empty(), "der Klick tat nichts und sagte nichts");
    pp!("D.ein_feld_ohne_ereignis_legt_keine_frage_vor", !banner(&u).contains("gibt_es_dieses_feld_nicht"), "die Kennung steht in der Meldung: {}", banner(&u));
    pp!("D.ein_feld_ohne_ereignis_legt_keine_frage_vor", s.textinhalt("#frage") == vorher_text, "die vorher offene Frage wurde ersetzt");
}
