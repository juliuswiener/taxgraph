# Import-Writer Security-Sweep (dev-1, 2026-07-20, Task #7)

Read-only Cross-cutting-Sweep. Frage: bewegt IRGENDEIN vorläufig-Event der neuen
Import-Kanäle (`import:beleg`, `import:kontoauszug`, `llm:chat`) die FESTGESETZTE Steuer
ohne Mensch-signal_2 — die [[ring-liest-vorlaeufig-parallel-pfad-luecke]]-Falle, auf Import
angewandt. **VERDIKT: GRÜN.** Keine neue parallele Roh-Pfad-Lücke gefunden.

## Gate 1 — Store-Katalog (Choke-Point)

`produkt/store/store.py` `append_event()` (149-243) ist der einzige Schreibpfad:
- Auflage A (Writer-Präfix-Block): `^llm:`/`^import:beleg`/`^import:kontoauszug`/
  `^import:vorjahr`/`^berechnet:` erzwingen `zustand="vorlaeufig"` + `signal_2=None` +
  korrekte `herkunft` — ValueError bei Verstoß.
- Auflage K1 (204-217): Katalog-Check gegen `vorschlagbar_von` (`lade_katalog()`, 137-146).
  Fehlt der Eintrag → Default human-only, fail-closed.

Feld-Enumeration: 21 `vorschlagbar_von`-Felder über 4 Bindungs-YAMLs
(n_vor_gwg/an_gesamt/sonder_agb_35a/kap_vv_familie; bindung_rentner: keine). ALLE
`typ: cent` oder `typ: int` (nur `ep_entfernung_km`, Distanz). Kein bool/enum/datum/text
trägt `vorschlagbar_von` — Klassifikation/Wahlrecht/Status/Identität/Abwesenheit/Allokation
bleiben strukturell human-only.

Zusatz-Härtung `beleg_writer.py` `beleg_felder()` (84-95): filtert
`if b.get("typ") != "cent": continue` — beleg_writer kann strukturell GAR NICHT auf
non-cent-Felder schreiben, unabhängig von der Whitelist.

Test-Coverage bereits vorhanden: `tests/test_ui_zwei_signal_sicherheit.py:85-86,118-123`
(kontoauszug/beleg → vorlaeufig + Gegenprobe), `:127` (Katalog mandatory), `:158-160`
(beleg_import-Zwang an `^import:beleg`-Präfix, nicht herkunft).

## Gate 2 — Ring-Bescheid

`produkt/haut/api.py` `_bescheid_fn()` (428): Default `nur_bestaetigt=True`, Snapshot-Filter
452-453 (`felder = {... if ev.get("zustand")=="bestaetigt"}`). Alle 6 Call-Sites geprüft:
- **1031** (`_feste_zahl`, EINZIGER festgesetzte-Zahl-Emitter): Default True ✓
- 1239/1245/1295/1302: explizit `nur_bestaetigt=False`, kommentiert Estimate-/`/stand`-Pfad
  — nie der festgesetzte Pfad, korrekt.
- 1391: Default True, aber nur `grund`-Klassifikation bei `zahl=None`, nicht die emittierte
  Zahl (die kommt aus `_feste_zahl` @1383).

Instanz-Σ (gwg/vv/rente) alle `nur_bestaetigt`-thread
(`if not nur_bestaetigt or inst["zustand"]=="bestaetigt"`). `EM.instanzen()`
(`est_mapping.py:347-372`) liefert bewusst RAW+meet-zustand pro Instanz; Filterung passiert
downstream in api.py, by design.

`_an_gesamt_sperrgrund` liest bewusst RAW (K2-Sperr-Guard) — safe-by-design, nur Lock, nie
Tax-Senkung. Kein `nur_bestaetigt`-Nutzer außerhalb `api.py` (grep bestätigt).

## Gate 3 — Elster-Export (Bonus-Fund, dritte unabhängige Schicht)

`produkt/mapping/est_mapping.py`:
- `deklariere()` **212**: `if sfeld.get("zustand") != "bestaetigt"` → `unvollstaendig`-Liste,
  NIE in `deklaration`.
- `_deklariere_instanz()` **142**: identisches Gate für Instanz-Felder (Multi-GWG/VV/Rente).

Selbst ein hypothetischer Ring-Bug könnte vorläufige OCR/LLM-Werte nicht bis in die
tatsächliche ELSTER-Übermittlung durchreichen.

## Fazit

Guess-Writer-Containment ist **dreifach gegated**: Store-Katalog → Ring-Bescheid →
Elster-Export. Kein ROT. Kein neuer paralleler Roh-Pfad durch die Import-Writer.
