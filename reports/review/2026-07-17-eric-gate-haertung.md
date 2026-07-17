# ERiC-Gate-Härtung gegen Falsch-Grün (Fehler-Kappung)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Auslöser:** P9-R3-Fund — checkESt kappt die FehlerRegelpruefung-Liste per Default bei 20
(„Weitere Fehlermeldungen werden abgeschnitten", ERiC-Entwicklerhandbuch Kap. 4.1.2.1). >20 Fehler →
Fehler 21+ gehen still verloren.

## Wichtige Korrektur zum Wertebereich

Einstellungsnamen bestätigt: **`validieren.fehler_max`** / **`validieren.hinweise_max`** (Int, via
`EricEinstellungSetzen`). **Aber:** erlaubter Wertebereich lt. Handbuch = **1–1000** (Default 20).
Der vorgeschlagene Wert 10000 ist außerhalb → `EricEinstellungSetzen` liefert rc=610001861
(`ERIC_GLOBAL_EINSTELLUNG_WERT_UNGUELTIG`), Cap bleibt bei 20. **Verwendet: 1000 (das Maximum).**
1000 ≫ jede realistische Fehlerzahl einer Erklärung → praktisch keine Kappung mehr.

## Umgesetzt (Ruling-Punkte a–d)

**(a) Cap-Anhebung VOR jedem Lauf** — `checkest_gate._load_and_init()` setzt nach `EricInitialisiere`
beide Caps auf `VALIDIERE_MELDUNGEN_MAX = 1000`; schlägt das Setzen fehl → `RuntimeError` (Abbruch
statt stiller Kappung). Einmal pro Prozess, gilt für ALLE Aufrufer (Gate, Fuzz, künftiger UI-Worker).

**(b) Fixpunkt-/Rest-Trunkierungs-Doktrin** — `checkest_gate.gekappt_verdacht(antwort)`: erreicht die
Fehlerzahl den (angehobenen) Cap, KANN weiter gekappt sein → nie als vollständig behandeln, Fehler
beheben und RE-validieren bis rc==0. Als Consumer-Helfer bereitgestellt + dokumentiert.

**(c) rc-Klassifizierung** — `checkest_gate.klassifiziere_rc()` + Konstanten `RC_OK`,
`RC_PLAUSIBILITAET` (610001002), `RC_IO_KEIN_TICKET` (610301200), `RC_HERSTELLER_GESPERRT` (610301202).
`eric_gate` Stufe B wertet **610301200 explizit als RED** („0 Fehler ≠ fehlerfrei", short-circuit VOR
der Plausibilität).

**(d) Negativtest / Tamper-Beweis** — neue **`eric_gate` Stufe C** (red-fähiger Regressions-Guard):
- Produktionspfad (ohne manuelles Setzen) auf einem >20-Fehler-Fixture MUSS **>20** liefern → beweist,
  dass `_load_and_init` die Anhebung wirklich vornimmt. Entfernt jemand die Anhebung → **26 fällt auf
  20 → Stufe C ROT** (empirisch verifiziert).
- Tamper: Cap manuell zurück auf 20 → **genau 20** (beweist: Fixture hat >20 Fehler UND Default kappt).
- Ohne `$ELSTER_HERSTELLER_ID` (credential-freier CI): Stufe C **übersprungen** (Plausibilitätspfad
  nicht erreichbar, analog Stufe-B-GESPERRT-Grenze) → CI bleibt grün.

## Geänderte Dateien

- `elster/checkest_gate.py` — Cap-Anhebung in `_load_and_init`, `VALIDIERE_MELDUNGEN_MAX`, RC-Konstanten,
  `klassifiziere_rc`, `gekappt_verdacht`.
- `elster/eric_gate.py` — Stufe B rc-Klassifizierung (610301200 → RED), neue Stufe C (Trunkierungs-Guard).
- `elster/fuzz/checkest_fuzz.py` — P3 als Tamper-Beweis (Cap 20↔1000).

## Verifikation

| Prüfung | Ergebnis |
|---|---|
| `checkest_gate --prove` (PIPE-PROOF) | BESTANDEN (plausibel rc=0, implausibel rc=610001002) — Contract intakt |
| `eric_gate` MIT HID | GRÜN, exit=0 (A ok, B plausibel, C: Produktion 26>20 & Tamper 20) |
| `eric_gate` OHNE HID (credential-frei) | GRÜN (A ok, B GESPERRT-Grenze, C übersprungen) |
| Negativtest: Anhebung deaktiviert | Stufe C ROT (Produktion 20, nicht >20) → GATE ROT — Guard rot-fähig |
| `fuzz` P3 Tamper | BESTANDEN (Cap=20→20 gekappt, Cap=1000→26 vollständig) |

## Offen / Übergabe

- Bei 36 korrumpierten Feldern werden 26 Fehler gemeldet (nicht 36): die restlichen ~10 Felder sind
  String-typisiert, wo der Ersatzwert `ZZ99XX` formal gültig ist → kein Fehler. 26 = vollständige
  Fehlermenge (26 < 1000 = kein Cap-Effekt).
- Fixpunkt-Revalidierungs-SCHLEIFE (aktives Beheben+Re-Validieren) gehört in den künftigen UI-Worker,
  nicht ins CI-Gate (das ein bekannt-gutes Fixture prüft). Helfer `gekappt_verdacht` liegt bereit.

Reproduktion: `ERIC_DIR=~/02_Software/eric ELSTER_HERSTELLER_ID=<id> python3 elster/eric_gate.py`
