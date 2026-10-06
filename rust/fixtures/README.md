# `rust/fixtures/`

`golden_cases.json` — Extrakt von `golden/cases/*.yaml` (135 Faelle), erzeugt von
`tools/parity/extract_golden.py`. Kein Rust-Runner konsumiert diese Datei in Schritt 1
(„Foundation") — sie liegt bereit fuer den Golden-Runner ab Schritt 4 (`REWRITE_PLAN.md` §7).

## Format

Ein JSON-Array, ein Objekt je Fall:

```json
{
  "id": "an_2025_einzel_ep",
  "beschreibung": "...",
  "sachverhalt": { "bruttoarbeitslohn": 40000, "veranlagungszeitraum": 2025, "...": "..." },
  "erwartung": { "festzusetzende_est": 6629 },
  "quelle": { "authority": "gesetz", "fundstelle": "...", "zitatanker": "..." }
}
```

- `sachverhalt`/`erwartung`: unveraendert aus dem YAML uebernommen, ein freies Dict je Fall
  (keine feste Feldliste — jeder Fall traegt nur die Schluessel, die er braucht).
- `quelle`: die Zitat-/Herkunftsangabe, fuer die Anker-Verifikation gegen `sources/`.

## Einheit: EURO, nicht Cent

Alle Zahlenwerte stehen wie im Quell-YAML in **vollen EURO** (`zu_versteuerndes_einkommen:
100000` = 100.000 €), nicht in Cent.

Ein blindes ×100 auf jeden Integer waere hier falsch: `sachverhalt`/`erwartung` sind freie
Dicts ohne Schema, das Geld- von Nicht-Geld-Schluesseln unterscheidet (`veranlagungszeitraum:
2025` ist ein Jahr, kein Betrag). Die Cent-Umrechnung braucht die `domain`-Typen (welches Feld
ist `Cent`, welches nicht) — die entstehen erst mit dem Golden-Runner in Schritt 4. Bis dahin
bleibt der Extrakt 1:1 zum YAML, und jeder Verbraucher rechnet selbst um, mit dem Wissen, was
Geld ist.

## Neu erzeugen

```
python3 tools/parity/extract_golden.py
```

## Eingefroren (Stufe 2)

Die Fixtures mit Antworten von Python — `interview_`, `konsistenz_`, `intervall_`, `wertwache_`, `eingang_`,
`zeichensatz_`, `kz_tabellen.json`, `sperrgrund_klartext.json` und `e2e/` — sind eingefroren. Ihre Erzeuger sind geloescht; der letzte Stand liegt im Verlauf (`git show 2dd056a6:tools/parity/<name>.py`).
Die Dateien werden nicht neu erzeugt und nicht von Hand geaendert. Eine gewollte Abweichung von Rust zu Python
steht als Eintrag mit Grund in einer Liste im Test; der Test verlangt, dass die Abweichung weiter besteht. Die
Abweichungsliste steht im Abschnitt „Abweichungsliste“ weiter unten.

Ausnahmen: `golden_cases.json` (Extrakt aus `golden/cases/*.yaml`, `tools/parity/extract_golden.py` bleibt) und
`begleitfelder_formen.json` (von Hand gepflegt). `api_stand_fragen_orakel.json` ist seit S2.2 ein Rust-eigener
Golden-Master (eigener Abschnitt unten) und wird mit `TAXGRAPH_GOLDEN_NEU=1` neu geschrieben.

## Abweichungsliste

Jede Stelle, an der Rust absichtlich von Python abweicht, mit Grund und dem Test, der sie haelt. Stand: Stufe 2,
Python-Stand `2dd056a6`.

Regeln:

1. Eine gewollte Abweichung hat einen Eintrag in dieser Liste UND einen Test ohne `PARITY=1`, der das heutige
   Rust-Verhalten festhaelt. Gleicht jemand Rust an Python an, wird der Test rot. Wer den Test aendert, aendert
   oder streicht den Eintrag im selben Commit.
2. Die eingefrorenen Fixtures (siehe oben) werden dafuer nie angefasst. Eine Abweichung steht in der Liste eines
   Tests oder in einem eigenen Test, nie als Aenderung in der Datei.
3. Spalte „Pruefung“: **M** = Mutant gemessen. Die Abweichung im Quelltext zurueckgebaut oder verschoben, der
   genannte Test wurde rot, die Basis war gruen. **T** = Test vorhanden, nicht mutiert (die Zusage steht im
   Kommentar des Tests).

| Nr | Ort | Python | Rust | Grund | Test (ohne `PARITY=1`) | Pruefung |
|----|-----|--------|------|-------|------------------------|----------|
| 1 | `eingang`, VaSt-Betrag, vier Eingaben `1__2`, `1_`, `_1`, `1e1_0` | `Decimal` streicht Unterstriche | `NichtLesbar` | VaSt-XML traegt keine Unterstriche | `rust/eingang/tests/orakel_werte.rs::vast_betraege_wie_orakel` (`UNTERSTRICH_ABWEICHUNG`, genau vier) | T |
| 2 | `domain`, Sperrgruende `KindFreibetragVerteilungOffen`, `KindZeitraumUnlesbar` und (seit Nr. 23) `Abs3PartnerAntragGewinnOffen`, `Abs3PartnerAntragUeber5mioOffen`, `BerufsunfaehigkeitPartnerOffen` | kennt sie nicht | Klartext und Kennung laufen rund | Entscheidung Julius 2026-10-04 | `rust/domain/src/sperrgrund.rs::tests::rust_eigene_gruende_haben_klartext_und_laufen_rund` | T |
| 3 | `api`, `elster`: Ganzzahl ausserhalb `i64` | rechnet mit beliebig grossen `int` weiter | HTTP 422, nie 500, nie Umbruch | Korrektheit vor Paritaet (`REWRITE_PLAN.md` §4) | `rust/api/tests/ueberlauf_klassen_hermetisch.rs` (elf Eingaben); `rust/elster/src/deklaration.rs::tests::p23_gewinn_ausserhalb_i64_ist_ein_fehler_statt_umbruch` | T |
| 4 | `domain`, Feld-Kennung `"x\n"` | `$` passt auch vor `\n`, das Schema nimmt es an | abgelehnt | fail-closed | `rust/domain/src/feld_id.rs::tests::basis_id_nimmt_genau_die_schema_regel_an` | M |
| 5 | `llm`, HTTP-Client, drei Eingaben (`Content-Length: abc`, URL ohne Schema, fremdes Schema); die vierte, IPv6 ohne Port, ist behoben (`http::tests::url_zerlegen_ipv6_literal`) | liest die Antwort bis zum Verbindungsende bzw. scheitert voruebergehend (drei Versuche) | endgueltiger Fehler nach einem Versuch | Befund der Messung, keine Absicht | `rust/llm/tests/client_netz.rs::weicht_von_python_ab_drei_eingaben_enden_in_rust_anders` | T |
| 6 | `llm`, `content` der Antwort ist Liste, Zahl oder Objekt | `.strip()` wirft `AttributeError` ungefangen (Dienst: 500) | endgueltiger Fehler `AttributeError` | der Aufruf bricht nicht ab | `rust/llm/tests/client_netz.rs::weicht_von_python_ab_inhalt_ohne_zeichenkette_ist_endgueltig` | M |
| 7 | `llm`, Fehlerkoerper mit Schluessel ueber der 300-Zeichen-Kante | kuerzt, dann maskiert; der Anfang des Schluessels bleibt stehen | maskiert, dann kuerzt | Sicherheit | `rust/llm/tests/client_netz.rs::weicht_von_python_ab_schluessel_an_der_schnittkante_wird_maskiert` | M |
| 8 | `llm`, Aussage-Nummer ausserhalb `i64` (`1e30`, `2^64-1`) | `int(...)` gibt die grosse Zahl; `int(inf)` wirft ungefangen | `None` | `inf` kommt ueber `serde_json` nicht an; nur die Zahl ausserhalb `i64` ist erreichbar | `rust/llm/tests/parse_entscheidungen.rs::weicht_von_python_ab_index_ausserhalb_i64_ist_none` | M |
| 9 | `llm`, `kategorie` ist Liste oder Objekt | `TypeError` (unhashable) ungefangen | `None`, Buchung bleibt unklassifiziert | der Aufruf bricht nicht ab | `rust/llm/tests/kontoauszug_entscheidungen.rs::weicht_von_python_ab_kategorie_liste_oder_objekt_ist_none` | M |
| 10 | `konsistenz`, Betragspruefungen (`bestaetigter_betrag`, `vorlaeufige_ring_betraege`): ein Float | zaehlt als Betrag (`isinstance(w, (int, float))`) | zaehlt nicht | Wertebereich: der Store laesst auf `cent`-Feldern nur Ganzzahlen zu (Auflage T); 0 Floats in 192 echten Faellen | `rust/konsistenz/src/preflight.rs::tests::float_ist_kein_betrag_abweichung_von_python` | M |
| 11 | `auth`, Audit scheitert | kein `try`, bricht ab | Anmelden, Abweisen und Abmelden laufen durch | das Audit ist ein Nebenkanal | `rust/auth/tests/entscheidungen.rs::ein_audit_fehler_kippt_keine_anmeldung` | T |
| 12 | `llm`, Dialogantwort: `NaN`, `Infinity`, `-Infinity`, `1e400` in `wert`, `rechenweg`, `aussage`; einzelnes Surrogat-Escape in `begruendung` | `json.loads` liest alle fuenf; die Antwort gilt | ganze Antwort `Antwort::Unlesbar` (Rueckfall des Dienstes), nie ein halb gelesener Vorschlag | `serde_json` ist strikt; die Fuenf sind kein gueltiges JSON | `rust/llm/tests/parse_entscheidungen.rs::weicht_von_python_ab_nan_unendlich_und_einzelnes_surrogat_machen_die_antwort_unlesbar` (neun Eingaben, eine Kontrolle); seit loeschplan-v4 Stapel 1e zusaetztlich ueber die Route: `rust/api/tests/chat_llm_attrappe_hermetisch.rs` (vier Eingaben mit NaN, `1e400`, `Infinity`, Surrogat, eine Kontrolle, dazu Stufe-3-Ausfall als `werte_ausgefallen`) | M |
| 13 | `llm`, ORS-Antwort mit UTF-8-BOM, in UTF-16, oder mit `NaN`, `Infinity`, `-Infinity`, `1e400` im Koerper | `json.loads(bytes)` liest sie | „nicht verfuegbar“ (Dienst: 503) | `serde_json` liest nichts davon | `rust/llm/tests/ors_netz.rs::weicht_von_python_ab_ors_antwort_mit_bom_utf16_nan_ist_nicht_verfuegbar` (sechs Eingaben, eine Kontrolle) | M (BOM, `NaN`); UTF-16 T |
| 14 | `llm`, Anbieterantwort mit `NaN`, `Infinity`, `-Infinity`, `1e400` irgendwo im Koerper | Python nimmt die Antwort an | endgueltiger Fehler `JSONDecodeError` nach einem Versuch | `serde_json` ist strikt | `rust/llm/tests/client_netz.rs::weicht_von_python_ab_anbieterantwort_mit_nan_ist_endgueltig` (vier Eingaben, eine Kontrolle) | M |
| 15 | `store`, `fehler_log`: Feld `quelle` | innerster Traceback-Rahmen (Datei:Zeile des Fehlers) | Aufrufstelle von `protokolliere` (`#[track_caller]`) | Rust hat keinen Traceback | `rust/store/src/fehler_log.rs::tests::quelle_ist_die_aufrufstelle_von_protokolliere` | M |
| 16 | `domain`, JSON-Lader von `PyWert`: 23 Skalare (`NaN`, `Infinity`, `1e400`, `-0`, Surrogat-Escapes) und 5 Strukturen | `json.loads` | Fehler, oder anderer Wert (je Eintrag in der Liste) | `serde_json` ist strikt, liest `-0` als `-0.0` und eine grosse Ganzzahl als Gleitkomma | `rust/domain/tests/lader_abweichungen_hermetisch.rs::json_lader_haelt_die_abweichungen_von_json_loads` (genau 23 + 5) | M |
| 17 | `domain`, YAML-Lader von `PyWert`: 114 Skalare und 113 Strukturen (`yes`/`no`/`on`/`off`, `007`, `1:30`, `1_000`, `0o17` ...) | YAML 1.1 (`yaml.safe_load`) | YAML 1.2 (`serde_yaml_ng`) | andere Bibliothek, anderer Standard | `rust/domain/tests/lader_abweichungen_hermetisch.rs::yaml_lader_haelt_die_abweichungen_von_safe_load` (genau 114 + 113) | M |
| 18 | `domain`, Ganzzahl ausserhalb `i64::MIN..=u64::MAX` (D1, vier Eintraege) | beliebig grosses `int` | `Gleit`: gleich der Nachbarzahl | `PyWert` fuehrt keine Ganzzahl ausserhalb des Bereichs | `rust/domain/tests/lader_abweichungen_hermetisch.rs::d1_ganzzahl_ausserhalb_des_bereichs_ist_gleich_der_nachbarzahl` | M |
| 19 | `domain`, `store` (typisierter Pfad `Store::append`): `bestaetigt` mit leerem `signal_2` zusammen mit typ-inkonformem Wert (D20) | meldet zuerst Auflage T (`TypInkonform`) | `Signal2::new` weist das leere Signal schon beim Bau ab (`ZweiSignalFehlend`); der Rohpfad `append_roh` (HTTP) haelt Pythons Reihenfolge | der Typ macht „bestaetigt ohne Signal“ unbaubar; Aufheben braeuchte einen Umbau des Typs | `rust/domain/tests/fehlertexte_hermetisch.rs::betrag_herkunft_und_signal_melden_ihren_text` (`Signal2::new("  ")` scheitert); Rohpfad: `rust/store/tests/append_roh.rs::bestaetigt_braucht_ein_signal_2_das_nicht_leer_ist` | M (Bau); Reihenfolge im typisierten Pfad T |
| 20 | `domain`, `repr_str`: Zeichen der Kategorien Cf (U+0600), Co (U+E000), Cn (U+0378) | `repr` escapet sie (`'\u0600'`) | bleiben stehen (`ponytail` an `repr_str`: Kategorientabelle fehlt) | Annaeherung an `str.isprintable()` | `rust/domain/src/py_text.rs::tests::repr_str_wie_cpython` | T |
| 21 | `bindung`, Feld `dhf_keine_pflicht_dienstwohnung` (Frage nach der Dienst- oder Werkswohnung im Ausland) | fragt sie (`askable: true`), im Inland wie im Ausland | stellt sie nicht mehr: `askable: false`, das Feld bleibt in der Bindung (Muster `spenden_vermoegensstock`). `/fragen` eine Frage weniger, `offen` um eins kleiner; die Regel nennt die Annahme weiter in `annahmen_offen`, weil sie in der Rechnung steckt; zwei Nachbarfragen tauschen den Platz (`keine_zweitwohnung`, `keine_energetische_sanierung`: das Gate-Gewicht sinkt um eins). Eine schon gegebene Antwort bleibt im Store, steht in `nicht_deklariert` (Grund: „Frage gestrichen 2026-10-06 ...“) und sperrt keine Akte; `POST /event` fuer das Feld antwortet weiter 201 | Entscheidung Julius 2026-10-06 („erstmal streichen“, VZ 2026 ist ein eigenes Paket). Die Antwort hat nie ein Ergebnis bewegt (bool, `gate: false`, kein Kz, kein Slot, der Catala-Scope kennt die Wohnungsart nicht). Gestrichen mit `entfernt:` in `FELD_BESTAND.yaml` waere jede Akte mit alter Antwort unvollstaendig und nicht einreichbar gewesen (gemessen: „Feld nicht in der Bindungstabelle“) | `rust/api/tests/dienstwohnung_gestrichen_hermetisch.rs::die_frage_wird_nicht_mehr_gestellt_und_das_ergebnis_bleibt` und `::eine_alte_antwort_bleibt_im_store_und_sperrt_die_akte_nicht`; `rust/bescheid/tests/dienstwohnung_einreichung_hermetisch.rs::eine_alte_antwort_auf_die_dienstwohnung_sperrt_das_einreichen_nicht`; der Golden-Master `rust/api/tests/stand_fragen_orakel_hermetisch.rs::stand_und_fragen_wie_python` | M |
| 22 | `bescheid` (`zweige/kinderfreibetrag.rs`), `bindung`, `elster`: Kinderfreibetrag bei Einzelveranlagung, wenn der andere Elternteil verstorben ist oder im Ausland lebte (zwei Felder je Kind: `kind_anderer_elternteil_tod_am`, Kz E0501102, Form TT.MM.JJJJ; `kind_anderer_elternteil_ausland_zeitraum`, Kz E0503903, Form TT.MM-TT.MM) | rechnet es nicht: bei Einzelveranlagung zaehlt je Kind nur die Haelfte (Satz 2), beide Felder gibt es nicht, `/fragen` stellt keine der beiden Fragen | zwei neue Fragen je Kind, nur bei Einzelveranlagung (bei bestaetigter Zusammenveranlagung fehlen sie). Je Monat, in dem die Voraussetzung des Kindschaftsverhaeltnisses besteht UND der Elternteil verstorben ist (ab dem Todesmonat, der Todesmonat zaehlt) oder im Ausland lebte (angefangene Monate), kommt der zweite Elternteil-Zwoelftel dazu (`personenmonate = monate(a) + monate(a & satz_3_monate)`); die Monate beider Felder zaehlen als Vereinigung, nie doppelt; das Kindergeld folgt weiter nur `monate(a)`. Zusammenveranlagung liest die Felder nicht. Ein Todestag oder Zeitraum, den der Rechner nicht lesen kann (Tag gibt es im Kalender nicht, falsche Form), sperrt mit `kind_zeitraum_unlesbar` statt zu raten. Die beiden Angaben gehen in den Abschnitt `Weit_Ang` der Anlage Kind. Bei 50.000, 60.000 und 80.000 Euro zvE (VZ 2025, ein Kind, Tod vor dem Jahr) sinkt die Steuer um 183, 522 und 972 Euro (von Hand aus `params/2025`) | Entscheidung Julius 2026-10-06 („bauen“): § 32 Abs. 6 Satz 3 Nr. 1 EStG. Nicht gebaut: das Kz E0501513 („Wohnsitz oder gewoehnlicher Aufenthalt des anderen Elternteils nicht zu ermitteln“; steht in der Anleitung, nicht im Satz 3) und Satz 3 Nr. 2 (Alleinadoption, Pflegekind nur zu einem Elternteil; die XSD kennt dafuer kein Kz). Das Gesetz sagt „nicht unbeschraenkt einkommensteuerpflichtig“, die amtliche Anlage fragt „lebte im Ausland“: Rust nimmt die Angabe der Anlage als Naeherung (Anleitung Anlage Kind 2025, Z. 110-112). Todesmonat zaehlt voll (Lesart wie bei Satz 5: ein Tag der Voraussetzung genuegt) | `rust/bescheid/tests/kinderfreibetrag_je_kind_hermetisch.rs::tod_im_vorjahr_gibt_den_vollen_betrag_bei_50_60_80_tausend`, `::ausland_ganzes_jahr_gibt_den_vollen_betrag_bei_50_60_80_tausend`, `::todesmonat_zaehlt_voll_und_die_monatsgrenze_verschiebt_um_einen_monat`, `::tod_und_ausland_mit_ueberschneidung_zaehlen_die_vereinigung`, `::zusammenveranlagung_liest_die_satz_3_felder_nicht`, `::unlesbarer_todestag_sperrt`; `rust/bescheid/tests/c2_weit_ang_xml_hermetisch.rs::die_beiden_angaben_gehen_ins_xml_und_bleiben_gueltig`; `rust/api/tests/c2_satz3_fragen_hermetisch.rs::die_fragen_stehen_nur_bei_einzelveranlagung_und_solange_sie_offen_ist`; der Golden-Master `rust/api/tests/stand_fragen_orakel_hermetisch.rs::stand_und_fragen_wie_python` | M (36 Mutanten, alle rot) |
| 23 | `bescheid` (`zweige/tarif.rs`, `abzuege.rs`, `deklaration/sperre.rs`, `deklaration/ring_werte.rs`, `einkuenfte.rs`), `bindung`, `elster` (`PARTNER_VERZWEIGUNG`), `domain`: der Antrag auf den ermaessigten Steuersatz nach § 34 Abs. 3 EStG fuer den EHEGATTEN (vier Felder: `antrag_ermaessigter_satz_partner`, `dauernd_berufsunfaehig_partner`, `ermaessigung_einmal_genutzt_partner`; Ring-Wert `p34_abs3_antragsbetrag_partner`, Kz E0801602 / E0805003 / E0901704 je Betriebsart, Abschnitt PersonB der Anlage G / S / L) | kennt den Antrag nur fuer Person A (`antrag_ermaessigter_satz`, `dauernd_berufsunfaehig`, `ermaessigung_einmal_genutzt`): der Veraeusserungsgewinn des Ehegatten laeuft immer in der Fuenftelregel (Abs. 1), auch wenn er berechtigt ist und den Antrag will; die drei Felder gibt es nicht, `/fragen` stellt keine der drei Fragen | drei neue Fragen, nur bei Zusammenveranlagung und solange die Veranlagung offen ist (bei bestaetigter Einzelveranlagung fehlen sie, weil die Regel `p2_festzusetzung_zusammen` dann ganz wegfaellt, `bindung_regel_bedingungen.yaml`; ein Ehepaar ohne Gewinn des Partners bekommt sie weiter). Hat NUR der Partner einen Veraeusserungsgewinn (0 < Gewinn nach § 16 Abs. 4 <= 5 Mio Euro) und stellt er den Antrag UND ist er berechtigt (geboren 55 Jahre vor dem Veranlagungsjahr oder frueher, oder dauernd berufsunfaehig, UND den Satz nie genutzt), rechnet der Chooser Abs. 3 fuer SEINEN Gewinn: Steuer auf zvE minus Gewinn plus 56 % des durchschnittlichen Steuersatzes (mind. 14 %) auf den Gewinn. Der Antrag von A gilt nur fuer A, der des Partners nur fuer den Partner. Die Antragszeile steht neben der Basiszeile im Abschnitt PersonB. Haben BEIDE einen Gewinn und beantragt der Partner, sperrt `abs3_partner_antrag_gewinn_offen`; beantragt A, bleibt `abs3_partner_gewinn_offen`. Ueber 5 Mio Euro Gewinn des Partners sperrt `abs3_partner_antrag_ueber_5mio_offen`, ein unbeantwortetes `dauernd_berufsunfaehig_partner` bei Antrag ohne Altersberechtigung `berufsunfaehigkeit_partner_offen` (drei Rust-eigene Sperrgruende, Klartext aus Sicht des Nutzers). Bei 200.000, 500.000 und 5.000.000 Euro Gewinn des Partners (VZ 2025, Zusammenveranlagung, A mit 60.000 Euro Lohn) sinkt die tarifliche Steuer von 71.658, 191.188 und 2.156.648 auf 45.829, 114.946 und 1.246.931 Euro (von Hand aus `params/2025` und dem Wortlaut, geeicht an zwei Python-Zahlen der Kette) | Entscheidung Julius 2026-10-06 („B Option 1 bauen“, Vault `backlog/taxgraph/p34-abs3-fuer-den-partner-fehlt-ganz`). Nicht gebaut: beide Gewinne zugleich. Fuer Ehegatten ist § 34 Abs. 3 S. 5 offen: Er laesst die Ermaessigung fuer EINEN Steuerpflichtigen mit mehreren Gewinnen nur fuer einen Gewinn zu. Ob er bei Zusammenveranlagung die Gewinne beider Ehegatten zusammen trifft (§ 26b EStG behandelt sie, soweit nichts anderes vorgeschrieben ist, gemeinsam als Steuerpflichtigen) oder je Ehegatten gilt, sagt der Wortlaut nicht. Die Sperre ist die vorsichtige Wahl, kein Gesetzesgebot; wie der Chooser zwei Gewinne gegeneinander setzt, ist ebenfalls offen. Nicht gebaut ist auch der Teil ueber 5 Mio Euro, mehrere Veraeusserungsgewinne desselben Partners. Das 55. Lebensjahr liest Rust wie bei Person A aus dem Geburtsjahr (`geburtsjahr_partner`, Jahr des Veranlagungszeitraums minus Geburtsjahr >= 55), nicht aus dem Geburtstag. Kz nur aus der XSD E10-2024 und E10-2025; fuer 2026 liegt kein Schema vor, die Antragszeile des Partners ist fuer 2026 ungeprueft (wie bei Person A). Die Tabellenzeile `PARTNER_VERZWEIGUNG[p34_abs3_antragsbetrag_partner]` steht nur in Rust (`tabellen_gleich_fixture` nimmt sie aus dem Vergleich) | `rust/bescheid/tests/p34_partner_hermetisch.rs::partner_mit_antrag_rechnet_den_ermaessigten_satz_bei_200k_500k_5mio`, `::ohne_antrag_gilt_die_fuenftelregel_fuer_den_partner_gewinn`, `::antrag_ohne_berechtigung_bleibt_fuenftelregel`, `::die_berechtigung_hat_die_grenze_bei_55_und_die_berufsunfaehigkeit_ersetzt_das_alter`, `::der_antrag_gilt_je_person_fuer_den_eigenen_gewinn`, `::beide_gewinn_mit_antrag_des_partners_sperrt`, `::der_rohe_gewinn_von_a_entscheidet`, `::beide_gewinn_mit_antrag_von_a_behaelt_den_bisherigen_grund`, `::einzelveranlagung_liest_die_partner_angaben_nicht`, `::ueber_fuenf_millionen_sperrt_und_die_grenze_selbst_nicht`, `::offene_berufsunfaehigkeit_des_partners_sperrt_bis_zur_antwort`, `::die_berufsunfaehigkeit_von_a_sperrt_wie_bisher_bis_zur_antwort`, `::ein_vorlaeufiger_antrag_zaehlt_nicht`, `::der_antrag_des_partners_ohne_eigenen_gewinn_aendert_die_rechnung_von_a_nicht`, `::die_rentner_scheibe_rechnet_den_partner_antrag_ebenso`, `::der_chooser_nimmt_abs3_fuer_den_partner_ueber_fuenf_millionen_nicht`, `::der_chooser_nimmt_bei_zwei_antraegen_den_gewinn_von_a`; `rust/bescheid/tests/ring_scheiben.rs` (Regelzeile `p34_antrag_partner`); `rust/bescheid/tests/p34_partner_antragszeile_hermetisch.rs` (6 Tests, darunter das schemagueltige XML je Betriebsart); `rust/api/tests/p34_partner_fragen_hermetisch.rs` (3); `rust/elster/src/tabellen.rs::tests::die_antragszeile_des_partners_traegt_die_kz_von_person_a`; `rust/domain/src/sperrgrund.rs::tests::die_gruende_zum_antrag_des_ehegatten_sprechen_aus_sicht_des_nutzers`; der Golden-Master `rust/api/tests/stand_fragen_orakel_hermetisch.rs::stand_und_fragen_wie_python` (nur der Fall mit Zusammenveranlagung: +3 Fragen, offen +3) | M (60 Mutanten an anderen Stellen des Rust-Codes, Daten und Tabellen, 59 rot, 1 Ueberlebender: der Vorrang von A vor dem Partner im Chooser; die Sperren fangen beide Gewinne mit beiden Antraegen im Produkt vorher ab, der Chooser ist ueber `feste_zahl` ohne Sperre aber erreichbar und seit der Nachpruefung gepinnt, ebenso die 5-Mio-Grenze im Chooser: `::der_chooser_nimmt_bei_zwei_antraegen_den_gewinn_von_a`, `::der_chooser_nimmt_abs3_fuer_den_partner_ueber_fuenf_millionen_nicht`; 4 Kommentar-Kontrollen gruen; Basis 337 Tests gruen) |

### Geprueft ohne eigene Zeile, und was offen bleibt

Stand S2.6: die Rust-Seite der Abweichungen aus `rust/parity/tests/extern_stub/fremd_abweichungen.rs` (A1, A3, C),
`wert_paritaet.rs` (Lader, D1, `repr_str`) und `store_append_paritaet.rs` (D20) steht jetzt in der Tabelle oben
(Nr. 12 bis 20). Faellt Python weg, haelt jeder dieser Tests die Rust-Seite weiter. Die zwei Listen der Lader tragen
je Eintrag die Spalte `CPython` als Herkunft; geprueft wird nur die Rust-Seite.

- `rust/elster/tests/eigenschaften.rs::luecke_im_instanzindex_zaehlt_dicht_ohne_leeres_einz`: keine Abweichung von
  Python (beide zaehlen dicht), nur vom alten XML der vier Bestandsgruppen (leeres `<Einz>`). Der Test haelt die
  Rust-Seite (M: Rang „spaerlich“ statt „dicht“ wird rot).
- `rust/store/src/fehler_log.rs`: von den drei Abweichungen der Bauart steht eine in der Tabelle (Nr. 15). `Meta` als
  Struct und die Fall-Kennung mit Muster als Parameter sind Sache des Typs; der Compiler haelt sie, kein Laufzeit-Test.
- `rust/elster/tests/offene_defekte.rs` und `rust/bescheid/tests/offene_defekte.rs`: die „gewollte Abweichung“ im
  Kommentar je eines Tests beschreibt die Gestaltung des Tests, keine Verhaltensabweichung von Python. Keine Zeile.

Nicht hermetisch uebernommen und offen:

- `rust/parity/tests/wert_paritaet.rs::lader_echte_dateien_gegen_cpython` und `YAML_KONTEXT_AUSNAHMEN`: die Lader gegen die echten
  YAML-Dateien des Repos. Die Ausnahmen stehen nur in der Parity-Suite.
- Dass KEINE weitere Abweichung dazukommt, misst nur der Vergleich mit `CPython` (`PARITY=1`). Die Tests oben halten
  die bekannten Abweichungen fest, sie suchen keine neuen. Fuer die Zeit nach Python braucht es einen Ersatz fuer
  diese Frage (zum Beispiel einen eingefrorenen Mitschnitt der Antworten von `CPython` je Eingabe).

## `interview_orakel.json`

Szenarien fuer die Crate `interview` samt den Antworten des Python-Orakels
(`produkt/traverser/traverser.py`, `produkt/haut/bindung_rollen.py`). Konsument:
`rust/interview/tests/orakel_werte.rs` (hermetisch, ohne Python). Je Szenario: Events (Reihenfolge ist
Semantik), eine Sicht (voll, Teil-Bindung oder selbstgebaute Felder) und die erwarteten Antworten.
Die Frageliste steht als Positionen in der Sicht-Reihenfolge, die Relevanz als Abweichung zur
Relevanz des leeren Stores. Der Unsicherheits-Beitrag steht nicht im Fixture, die Formel steht in
Test (der Erzeuger ist geloescht).

Eingefroren: Der Rust-Test zeigt, wo Rust vom Orakel abweicht.

## `konsistenz_orakel.json`

Snapshots (Feld -> `[wert, "b"|"v"]`, Scheibe, Vorjahr) fuer die Crate `konsistenz` samt den Antworten des
Python-Orakels (`produkt/konsistenz/*`, ueber `tools/parity/oracle_konsistenz.py`): `preflight(...)` und
`unvollstaendige_instanzen` je Snapshot, dazu `eur` und die fuenf Tabellen. Lange Texte (`grund`, `hinweis`)
stehen einmal in `texte` und im Szenario als `{"$t": index}`. Konsument:
`rust/konsistenz/tests/orakel_werte.rs` (hermetisch, ohne Python). Floats und Ganzzahlen ueber `i64` stehen nur
dort, wo Rust sie wie Python behandelt (`flag_check`, `check_pauschalen`, `partner_check`); die Betrags-Pruefungen
aus `preflight.py` zaehlen einen Float in Rust absichtlich nicht (`// PARITAET:` in der Crate-Doku).

## `intervall_orakel.json`

Szenarien fuer die Crate `intervall` samt den Antworten von `produkt/unsicherheit/intervall.py`: `sicht` (Pythons
Rohlesung aller echten Bindungen, Reihenfolge der Rust-Registry), `iv` (`intervall(...)` ueber synthetische Bindung,
Snapshot, Deckel) und `slots` (`bescheid_via_slots(...)`). Beide Seiten rechnen ueber dieselbe synthetische Engine
(`oracle_konsistenz._synth`: Summe ueber `gewicht(name) * zahl(wert)`). Konsument:
`rust/intervall/tests/orakel_werte.rs` (hermetisch, ohne Python).

Eingefroren: Der Rust-Test zeigt, wo Rust vom Orakel abweicht. Die Erzeuger waren deterministisch (fester Seed,
sortierte Ausgabe).

## `wertwache_orakel.json`

Feste Grenzfaelle fuer `engine::zugriff` samt den Antworten des laufenden Python-Orakels (`runner.catala_*` ueber
`tools/parity/oracle.py`): `solz`, `fuenftel`, `p32b_1`, `p34c_1`, `kst_nenner_b`, `behinderten_pb`, `p33a_unterhalt`,
`renten_einkuenfte`, `p3_nr72_photovoltaik`, `p101_mobilitaetspraemie` und `_cent` (3683 Faelle, 689 029 Bytes). Je Fall: `fn`, `args` (der rohe
Sachverhalt-dict, wie ihn die Parity-Suiten schicken) und `py` (`{"ok": wert}` oder `{"err": klasse, "catala": bool}`).
Die Faelle liegen an den Schwellen und Saetzen der Zugriffsfunktionen (0/0, -1/-1, genau an der Grenze, +-1), weil der
Zufallsgenerator von `zugriff_teil{1,2}_paritaet` sie nur selten und nie gemeinsam zieht. Die Gitter der Koerperschaftsteuer
(Zinsschranke, Verlustabzug, Spendenabzug), der Rente (Basisrente aa) und des Unterhalts-Bodens stammen aus einem
Operator-Sweep ueber die Rumpfe der elf Funktionen: je Mutant, der ohne `PARITY=1` gruen blieb, kam der kleinste Fall dazu,
der ihn faengt. Konsument:
`rust/engine/tests/wertwache_orakel_werte.rs` (hermetisch, ohne `PARITY=1`, ohne Python; benutzt die Adapter der
Parity-Suiten).

Eingefroren: Der Rust-Test zeigt, wo Rust vom Orakel abweicht. Der Erzeuger brauchte die gebaute Catala-Engine
(`make build-python`) und war deterministisch (feste Gitter, sortierte Ausgabe).

## `eingang_orakel.json`

Eingaben fuer die Crate `eingang` samt den Antworten des Python-Orakels (`produkt/eingang/*` ueber
`tools/parity/schritt8_oracle.py`). Konsumenten: `rust/eingang/tests/orakel_werte.rs` (Werte: CSV-
Zeilenmaschine, Betragsparser, Schluesselwoerter, Rundung, Vorjahr, eDaten, Beleg) und
`rust/eingang/tests/ocr_pfade.rs` (Abschnitt `ocr`: je Fall drei Shell-Skripte als `pdftotext`/`pdftoppm`/
`tesseract`, dieselben unter Python und Rust). Beide hermetisch, ohne Python zur Laufzeit. Der Erzeuger
legte `OMP_THREAD_LIMIT=7` in die Umgebung, damit sichtbar wird, dass nur `tesseract` ihn auf 1 setzt.

Eingefroren: Der Rust-Test zeigt, wo Rust vom Orakel abweicht. Das Fixture war deterministisch (Seed im Erzeuger).

## `zeichensatz_orakel.json`

Das Orakel `produkt/store/zeichensatz.py` fuer `domain::zeichensatz`, je Codepunkt (ohne Surrogate, bis U+10FFFF): `erlaubt`
(zusammenhaengende Bereiche, die `erstes_unerlaubtes_zeichen` durchlaesst) und `laeufe` (`[von, bis, name, rat]`: wie die
Meldung das Zeichen nennt, `null` = das Zeichen selbst, und der Rat), dazu volle Meldungen aus `feld_meldung` und
`element_meldung`. Konsument: `rust/domain/tests/zeichensatz_hermetisch.rs` (hermetisch, ohne `PARITY=1`, ohne Python).
Der Paritaetstest `rust/parity/tests/store_zeichensatz_paritaet.rs` prueft dasselbe live gegen Python.

Eingefroren; der Erzeuger war deterministisch.

## `api_stand_fragen_orakel.json`

Zwanzig Faelle (Scheiben `gesamt`, `an_gesamt`, `rentner_gesamt`, `ep`, `n_vor_gwg`; darunter zwei Vermietungsobjekte,
eine zweite Rente, ein vorlaeufiges Einzelfeld und ein Rentenbeginn im Folgejahr ohne Freibetrag, je bestaetigt und
vorlaeufig, damit die Faelle den Unterschied zwischen Ring mit und ohne Store, zwischen "nur bestaetigt" und
"auch vorlaeufig" und den Fehler `RentenfreibetragFixierungOffen` sehen; dazu zwei Faelle mit einer offenen Achse, damit die
Gewichte der Fragen-Reihenfolge sichtbar werden: Teil-Ring `ep_werbungskosten` und Gesamt-Ring ohne Gewichte wegen des Fehlers) samt den Antworten von GET /stand, GET /fragen und GET /feld/{fid}/frage. Bis `2dd056a6` waren das die Antworten des
Python-Servers (`api.stand`, `api.fragen`, `api.frage_einzeln` aus `produkt/haut/api.py`, im selben Prozess mit denselben
Ereignissen; `git show 2dd056a6:rust/fixtures/api_stand_fragen_orakel.json`). Der Inhalt blieb bei der Umstellung gleich
(Beleg im Commit: alle 20 Faelle gleich, auch `kopf`, den der alte Test nur fuer einen Fall verglich); nur `fall_id` heisst
in der Datei jetzt in jedem Fall `sf`. Aufbau eines Falls:
`events` (die Rumpfe von `POST /fall/<id>/event`, Reihenfolge ist Semantik), `stand` (die ganze Antwort), `fragen` (die
ganze Antwort, nur bei den grossen Faellen; sonst `fragen_ids` und der Sperrgrund), `kopf` (was der Mitschnitt fuer
`fragen` schreibt) und `einzeln` (Antwort je Probe-Feld). `event_id` jedes Felds in `stand` steht als `<event_id>` da:
der Server haengt die Uhrzeit an das Ereignis. Konsument: `rust/api/tests/stand_fragen_orakel_hermetisch.rs`
(hermetisch, ohne `PARITY=1`, ohne Python). Die Ereignislisten der Basisfaelle las der Erzeuger aus
`rust/api/tests/kette_endstand_hermetisch.rs`.

**Golden-Master (S2.2).** Die Datei gehoert Rust. Der Test spielt jeden Fall durch die echten Routen und vergleicht jede
Antwort Feld fuer Feld mit der Datei; die Meldung nennt je Fall bis zu zwoelf Pfade (`.stand.felder.<fid>.wert: ist …,
soll …`). Bei einer gewollten Aenderung (neues Bindungsfeld in einer Scheibe, neuer Fragetext, neuer Anzeigetext):

```
TAXGRAPH_GOLDEN_NEU=1 cargo test -p api --test stand_fragen_orakel_hermetisch
git diff rust/fixtures/api_stand_fragen_orakel.json    # LESEN
```

Das Neuschreiben nimmt jede heutige Antwort als richtig; der Diff ist die einzige Pruefung, und ein Mensch gibt ihn
frei. In der CI (`CI` gesetzt) ist es verboten. Die Datei steht zeilenweise (ein Feld, eine Frage, ein Ereignis je Zeile),
damit der Diff lesbar ist; das Format kommt vom Schreiber im Test, die Eingaben (`name`, `scheibe`, `events`) und die Huelle
eines Falls bleiben beim Neuschreiben stehen. Ein neuer Fall: `name`, `scheibe`, `events` und eine leere Huelle
(`"fragen": {}` oder `"fragen_ids": []`, dazu `"einzeln": {"<feld_id>": {}}`), dann `N_FAELLE` im Test anheben.
