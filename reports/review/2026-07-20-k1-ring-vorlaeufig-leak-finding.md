# K1 SECURITY-Fund + Fix: vorläufiger Vorschlag bewog die festgesetzte Steuer am Ring (dev-1, 2026-07-20)

## Kurz
Ein **vorläufiger** Vorschlag (llm:chat / import:beleg / import:kontoauszug / import:vorjahr / berechnet:maps)
für ein **optionales** (nicht-Kegel-)Feld bewegte die **festgesetzte Steuer** in `/ergebnis` — OHNE menschliches
signal_2. Das verletzt den Zwei-Signal-Kern-Invariant ("ein Vorschlag bewegt die Summe nie ohne Confirm").
**Pre-existing** (betraf alle vorläufig-Schreiber schon vor K1); K1 (LLM-Chat schreibt vorläufige optionale
Vorschläge) macht es scharf ausnutzbar. **Gefunden** durch die von dev-2 an dev-1 delegierte Ring-e2e in
`tests/test_haut_chat.py`. **Gefixt** (ein Punkt, `_bescheid_fn`), **verifiziert** (test_haut_chat 5/5).

## Reproduktion (test_ring_e2e_vorlaeufig_bewegt_steuer_nicht)
1. Voller AN-Kegel bestätigt → `festzusetzende_est` = **691900** ct.
2. llm:chat schlägt `agb_aufwendungen` = 5000 € **vorläufig** vor (signal_2=null) → `/ergebnis` = **604300** ct.
   → Der unbestätigte agB-Abzug hatte die Steuer um 87400 ct gesenkt. **DEFEKT.**
3. (Nach Fix) vorläufig → 691900 unverändert; erst der Confirm (ersetzt das llm-Event) senkt sie.

## Ursache
`produkt/haut/api.py::_bescheid_fn` baut die Wert-Closures (`_c`/`_cent`/`_b`) über den **rohen**
materialisierten `felder`-Snapshot: `f.get(fid, {}).get("wert")` — **ohne** `zustand`-Prüfung. Die Kegel-Felder
sind durch das Meet-Gate in `_feste_zahl` (`meet_zustand != "bestaetigt" → None`) geschützt; **optionale**
Felder (agB §33, Spenden §10b, Berufsausbildung §10, Mitunternehmer §15, §16-vg, §35a, …) sind **nicht** im
Kegel → das Gate greift für sie nicht → ihr vorläufiger Wert floss ungefiltert in den Bescheid.

Warum die bestehenden Grüns das nicht fingen: dev-2s S1/S6 prüfen den **Mapping**-Pfad (`est_mapping.deklariere`,
der vorläufig korrekt ausschließt). Der **Ring** (`/ergebnis` via `_feste_zahl`→`_bescheid_fn`) ist ein
**anderer** Pfad, der die Deklaration NICHT liest, sondern die Roh-`felder`. (dev-2s S6-Kommentar "die der Ring
als Eingabe liest" traf die Design-ABSICHT, nicht den Ist-Zustand.) Die Golden-e2e bestätigen optionale Felder
immer via `_laie` (bestätigt) → der vorläufig-Fall wurde nie am Ring getestet.

## Fix (chirurgisch, ein Punkt, over-tax-safe)
`_bescheid_fn` filtert den `felder`-Snapshot am Eingang auf **bestätigt-only**:
```python
if felder:
    felder = {fid: ev for fid, ev in felder.items() if ev.get("zustand") == "bestaetigt"}
```
Deckt alle 5 wert-rechnenden Aufrufer (gesamt/rentner/an_gesamt/teil-ringe). vorläufig → absent → 0 → kein
Abzug bis Confirm (über-Steuer-sicher, nie Unter-Steuer). Kegel-Felder ohnehin alle bestätigt (Gate). Der
**Sperr-Guard** `_an_gesamt_sperrgrund` liest die Roh-`felder` bewusst SEPARAT weiter (ein vorläufiges
nicht-ring-fähiges Feld muss den Ring weiter sperren) — vom Fix unberührt.

> ⚠ **v2-Endstand:** dieser Filter wird PARAMETERISIERT (`if nur_bestaetigt and felder:`) — der Instructor-
> Sweep fand, dass der UNBEDINGTE Filter auch die /stand-Range kollabierte. Endgültige Form + Begründung in
> **Nachtrag 2 (v2)** unten; der obige unbedingte Block ist der Zwischenstand, NICHT der committete Code.

## Verifikation (dev-1, isoliert, kein Port)
- `tests/test_haut_chat.py` **5/5 grün** (Cap-Gate 501, Prompt/Check-Katalog-Split, Happy-Path vorläufig,
  Graceful-Skip human-only, **Ring-e2e vorläufig-bewegt-Steuer-nicht**).
- Regressions-Analyse: die einzigen zwei vorläufig+zahl_cent-Tests in der Suite sind unberührt —
  `test_paket_b_e2e_http:369` (ep_arbeitstage = **Kegel**-Feld → `input_kegel_nicht_bestaetigt`, Kegel-Gate),
  `:605 test_an_gesamt_am_guard_vorlaeufig` (AM → **Sperr**-Guard, liest vorläufig separat). Kein Golden
  kodiert "vorläufiges optionales Feld IN /ergebnis". **Autoritativer Beweis = dev-2s integrierte Voll-Suite.**

## Nachtrag: volle Leak-Surface (Instructor-Adjudikation 4371 — kein Teilfix)
Instructor APPROVED die Fix-Semantik (bestätigt-only im Ring = Zwei-Signal-Vertrag), verlangte aber die
GANZE Surface zu schließen (ein Teilfix = falsche Sicherheit). Wichtige Abgrenzung: der Invariant gilt NUR
für die **festgesetzte Steuer** (`/ergebnis`, `_bescheid_fn`); der **/stand-[min,max]-Range zeigt vorläufig
ABSICHTLICH** (Steuer-at-Risk) — das ist KEIN Leak, wird NICHT gefiltert.

### Zweiter Leck-Pfad: Instanz-Enumeration (dev-2 empirisch bestätigt: vorläufige gwg-Instanz = 600€ am Ring)
`est_mapping.instanzen(store,…)` liest den **store** separat vom bestätigt-gefilterten `_bescheid_fn`-Snapshot
→ der flat-Filter greift dort nicht. Alle EM.instanzen-Wert-Konsumenten in api.py behandelt:
- **:386 gwg-Σ** — REAL LEAK (gwg optional, KEIN Kegel-/Sperr-Gate) → Filter `if inst["zustand"]=="bestaetigt"` PFLICHT.
- **:579 vv-Σ / :887 rente-Σ** — Filter als defense-in-depth (der `vv_instanz_offen`/`rente_instanz_offen`-Guard
  in `_an_gesamt_sperrgrund` sperrt index≥2-vorläufig schon VOR dem Σ, Basis im Kegel-Meet → no-op, aber
  konsistent + refactor-sicher).
- **:1092/:1110** = die Kegel-Gates SELBST (die Schutz-Mechanik) → unangetastet.
- `_laufender_gewinn` liest gefiltertes `f` + delegiert GWG an die gefilterte Σ → gedeckt. KEINE weiteren
  Roh-store-Reader im Wert-Pfad (grep EM.instanzen + `store` in `_bescheid_fn` 419-983).

### kontoauszug-Handler: Check-Katalog GLOBAL (Konsistenz/defense-in-depth)
`KW.uebernehme_kontoauszug` nutzte `bindung` für Check-Katalog UND Targeting. War KEIN Live-Leak (Targeting
Zeile 169 constraint auf per-Scheibe schon), aber der Enforcement-Katalog gehört global (dev-2-Kontrakt 4365,
decoupled). Additiver `katalog=`-Param (rückwärts-kompatibel: None → `lade_katalog(bindung)`-Fallback); Handler
reicht `ST.lade_katalog(TR.lade_bindung())`; Targeting bleibt per-Scheibe. dev-2 bestätigte sauberen Merge.

## Nachtrag 2 (v2): /stand-Range-Kollaps → Parameterisierung (Instructor-Sweep-Nebenbefund 4379)
Der unbedingte `_bescheid_fn`-Filter griff AUCH, wenn `/stand` den Bescheid für die [min,max]-Spanne rief
(api.py:1294) → die Live-Preview vorläufiger suggestible-Felder (agB/§35a/gwg) kollabierte in /stand (min=max,
verlor „bestätige agB → −876€"). KEIN Security-Regress (/ergebnis blieb bestätigt-only, beide /stand-Verhalten
über-tax-safe), aber UX-Regress der K1-Range-Kern-UX.

FIX (Instructor-approved, fail-safe): `_bescheid_fn(nur_bestaetigt: bool = True)` DEFAULT, durchgefädelt durch
`_gwg_sofortabzug_summe` + `_laufender_gewinn` + vv/rente-Σ inline. `=False` NUR an den 4 ESTIMATE-Calls (fragen
`_gesamt_beitrag` 1238/1244, `/stand` 1294/1301) — die NIE die festgesetzte Steuer emittieren. Festgesetzt-Pfade
(`_feste_zahl` 1030 = /ergebnis, availability 1390) bleiben default True (Auflage-1: kein festgesetzt-Pfad nutzt
False). Netto: **/stand-v2 == /stand-v1-ORIGINAL** (vorläufig-inklusiv); der unbedingte Filter war die einzige
Abweichung, v2 revertet /stand aufs Original → keine /stand-Regression.

DUAL-GOLDEN (Instructor-Auflage-2): `test_haut_chat` ring-e2e prüft am vorläufigen agB BEIDES — /ergebnis
UNVERÄNDERT (Security-Endpunkt) + /stand-intervall min<est_basis (UX-Preview). Fängt beide Regressionen
(Filter-weg=Security, Filter-zu=UX). dev-2s S10 = gwg-Instanz-Pendant.

### Fixture-Regression (K1-Katalog-Fallout, KEIN Security-Loch)
Der K1-Feld-Katalog wies Bestandstests ab, die `llm:chat` als GENERISCHEN vorläufig-Writer für human-only-Felder
(ep_arbeitstage/am) nutzten (dev-2s bnzfkylpu: 3/697, alle Katalog-Klasse). Fix (generisch-vorläufig → ui:laie,
katalog-frei): test_paket_b `_llm`→`_vorl`, test_paket_a `_llm_vorschlag`→`_vorlaeufig`, `_store_append`-vorläufig-
Zweig. ⚠ test_durchstich_http badge-Assert `"llm_vorschlag"`→`"laie"`: der Test demonstrierte KI-Vorschlag von
ep_arbeitstage — human-only, K1 verbietet das KORREKT → KI-Badge-Sub-Demo retired (in test_haut_chat abgedeckt).
Spezifische llm:chat-Auflage-A-Tests (test_paket_b:334 „gefaelscht", S5) UNANGETASTET.

## Verifikation (final, dev-1)
8 targeted GRÜN mit opam/catala: `test_haut_chat` 5/5 (inkl agB-Dual-Golden) + `test_paket_a_e2e` +
`test_durchstich_http` + `test_an_gesamt_am_guard_vorlaeufig` (die 3 formerly-failing). Volle `test_paket_b`
NICHT selbst gefahren (Port-Race mit dev-2s autoritativer Suite; regressionsfrei by construction s.o.).
**Autoritativer Gate = dev-2s finale integrierte Suite S1-S10 + test_haut_chat + volle Golden-Regression.**
KEIN Commit bis Instructor-Sign-off + grünes Suite-Verdikt.

## Geänderte Dateien
- `produkt/haut/api.py` — `_bescheid_fn(nur_bestaetigt=True)` bestätigt-Filter (flat-Leak) + 3 EM.instanzen-Σ-Filter
  (Instanz-Leak: gwg real + vv/rente defense-in-depth), alle via `nur_bestaetigt` parameterisiert (/stand=False) +
  K1-Handler final (chat prompt/check-Katalog-Split, event/entfernung/kontoauszug GLOBAL check-Katalog).
- `produkt/import/kontoauszug_writer.py` — additiver `katalog=`-Param (GLOBAL-Enforcement decoupled vom Targeting).
- `tests/test_haut_chat.py` — NEU (5 Tests, HAUT-Handler + agB-Dual-Ring-e2e).
- `tests/test_paket_b_e2e_http.py`, `tests/test_paket_a_e2e.py` — Fixture-Fix (generisch-vorläufig → ui:laie).
- (dev-2: `test_ui_zwei_signal_sicherheit.py` S9+S10 gwg-Instanz-Dual; est_mapping._b-Fix; finale autoritative Suite.)
