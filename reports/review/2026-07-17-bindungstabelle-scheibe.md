# Bindungstabelle — vertikale Erst-Scheibe N+VOR+GWG (Task #11)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Zone:** `produkt/` (neu, additiv, LLM-frei). Kein Touch an rules.yaml / item_registry / elster.

## Dateien

- `produkt/bindung/schema.json` — JSON-Schema 2020-12 (Auflagen A/B/C eingearbeitet).
- `produkt/bindung/SCHEMA.md` — Doku, Summen-Konvention, Gate-Vertrag, Entscheidungen.
- `produkt/bindung/bindung_n_vor_gwg.yaml` — die Scheibe: **31 Bindungen + 8 benannte Lücken**.
- `tests/test_bindungstabelle.py` — Gate (11 Tests, inkl. 5 Negativtests). **11/11 grün.**

## Abdeckung (6 Regeln)

| Regel | askable Slots | Geltungsbed. | Bindungen | Lücken |
|---|---|---|---|---|
| EP `p09_entfernungspauschale` (Catala) | 4 | – | 4 | – (4 Parameter auto-exempt) |
| dHf `p9_1_3_nr5_...` | 3 | 4 | 7 | – |
| Verpflegung `p9_4a_...` | 4 | 6 | 8 | 2 (Ausland; Mitternachtsregel) |
| Arbeitsmittel `p9_1_3_nr6_7_...` | 4 | 2 | 3 | 3 (AfA-Zweig §7) |
| VOR `p10_1_2_altersvorsorge` | 3 | 3 | 5 | 3 (Parameter + ag-Doppel-Slot) |
| GWG `p6_2_gwg_sofortabzug` | 1 | 3 | 4 | – |

**Vollständigkeit deterministisch belegt** (Gate b): jeder askable Signatur-Slot (aus rules.yaml
`signature.inputs`, für EP aus der Catala-Signatur) und jede Geltungsbedingung hat eine Bindung ODER
eine benannte Lücke. Parameter-Slots sind ausgenommen über **Datei-Abgleich** mit den Value-Keys von
`params/<vz>/` (keine Namens-Heuristik). 9 Felder tragen eine amtliche `elster_kz` (alle gegen
E10-2025 verifiziert); 22 sind Ja/Nein-Bedingungen oder modell-abweichende Slots ohne eigenes ESt-Kz.

## Gate (`tests/test_bindungstabelle.py`)

- (a) Schema-Validierung + feld_id-Eindeutigkeit.
- (b) Vollständigkeit je Regel (Slots ∪ Geltungsbedingungen − Parameter).
- (c) `elster_kz` existiert in E10-2025 (`kz_extract`; skip wenn Schemadok fehlt).
- (d) `anker_ref.zitatanker` voll-Länge via `pipeline/gates._normalize` gegen die Quelldatei — **alle
  31 Anker verifiziert.**
- (e) Summen-Konvention (je Slot: ein `exakt` ODER nur `summand`, typ-homogen).
- **Negativtests (5):** § im Fragetext, erfundene Kz, verfälschter Anker, unbelegter Slot, gemischte
  Summanden — jeder färbt das Gate rot. Zusätzlich extern verifiziert (echter Anker-Tamper → ROT,
  restauriert → GRÜN), gegen Falsch-Grün.

## Entscheidungen / eingearbeitete Auflagen

- **A Summen-Konvention:** der VOR-Slot `gesamtbeitraege_inkl_ag` wird aus **drei** Summanden gespeist
  (AN Nr. 23 / AG Nr. 22 / außerhalb LStB), jeder mit eigener `herkunft_slots`-Provenance — die
  materialisierte Provenance-Frage des UI-Kerns (mein LStB-Split-Nachtrag).
- **B Anker-Gate:** neues Feld `anker_ref.datei` (explizit gesetzt, da VOR/GWG kein rules.yaml
  `norm_source` tragen); voll-Längen-`_normalize`-Prüfung wie bei den Freezes.
- **C elster_kz einschemig:** `elster_kz` bindet nur ESt-Kz (E10-2025). Das GWG-Betragsfeld
  (`E6002301`, Anlage EÜR / eigene Datenart) wird NICHT als `elster_kz` gebunden, nur in
  `elster_kz_grund` referenziert — Gate (c) bleibt einschemig, kein Datenart-Misch.

## Surfacte Mapping-Befunde (benannte Lücken, kein Rate-Mapping)

1. **Verpflegung stunden↔tage:** unsere Signatur ist per-Reise (`abwesenheit_stunden`), die ELSTER-
   Deklaration zählt Tage je Kategorie (E0205201/E0205409). Modell-Umrechnung → betroffene Slots
   `elster_kz: null` + Grund.
2. **Arbeitsmittel AfA-Zweig:** `nutzungsdauer_jahre`/`anschaffungsmonat`/`jahre_seit_anschaffung`
   gehören zum §7-AfA-Zweig (> 800 €); Anker im §7-Freeze → 3 benannte Lücken (AfA-Scheiben-Nachtrag).
3. **VOR ag_anteil_steuerfrei:** identisch mit dem AG-Summanden-Feld (E2000801/Nr. 22); im UI eine
   Abfrage, intern auf zwei Slots verteilt → Lücke mit Verweis.
4. **Name-Mismatch Parameter:** VOR-Input `hoechstbeitrag_knappschaft` ≠ params-Key `hoechstbeitrag`
   → explizite Lücke (Grund nennt den echten Key), statt stiller Auto-Exemption.

## Offener Produkt-Punkt (kein Blocker)

- Anrede `du`/`Sie`: aktuell einheitlich `du`; Umschalter ist Render-Schicht-Feinschliff, nicht Binding.

## Reproduktion

```bash
ERIC_DIR=~/02_Software/eric python3 -m pytest tests/test_bindungstabelle.py -q
```
