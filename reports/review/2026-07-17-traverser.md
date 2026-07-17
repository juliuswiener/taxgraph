# Traverser (K1) + Paket-B-API-Vertrag — Task #11, Paket A

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Zone:** `produkt/traverser/` (neu, additiv, **NULL LLM**, read-only).

## Dateien

- `produkt/traverser/KONZEPT.md` — abgenommene Skizze.
- `produkt/traverser/traverser.py` — relevanz / naechste_fragen (rückwärts) + justification / trace_ergebnis (vorwärts).
- `produkt/traverser/guenstiger_liste.yaml` — source-verankerte Günstiger-Knoten + benannte Ausnahmen.
- `produkt/traverser/API.md` — die Paket-B-Naht (Kern↔Haut).
- `tests/test_traverser.py` — Gate, **11 Tests grün** (inkl. 3 Negativtest-Klassen + Sweep-Netz).

## Was der Traverser leistet (K1)

**Rückwärts = Interview:** `relevanz(store, bindung)` klassifiziert je Regel
`ausgeschlossen|relevant|unentschieden` aus den **Gating-Bedingungen** (askable bool-Geltungsbedingung
= `false` → Regel ausgeschlossen). `naechste_fragen()` liefert die geordnete Queue: **Gating zuerst**
(streicht ganze Regeln), dann Slots nach **Unsicherheits-Beitrag** (`intervall.py`), sonst
feld_id-deterministisch. Ausgeschlossene Regeln werden nicht gefragt.

**Vorwärts = Beweis:** `justification(store, feld_id, bindung)` = das rekursive Justification-Blatt
(wert/zustand/herkunft/event_id/signal → regel_id/slot|bedingung → **anker_ref**); `trace_ergebnis()`
gruppiert je beteiligter Regel, snapshot-gebunden.

## Auflagen umgesetzt

- **Günstiger = Liste** (`guenstiger_liste.yaml`, source-verankert): der Scheiben-Günstiger-Knoten
  **EP-ÖPNV** (`p09_entfernungspauschale`, Zweige `ep_entfernung_km` + `ep_oepnv_kosten`, Anker
  § 9 Abs. 2 EStG verifiziert). Interview-Garantie **by construction**: `naechste_fragen` nimmt ALLE
  unbeantworteten askable Felder nicht-ausgeschlossener Regeln → kein Zweig wird anhand eines
  vorläufigen Siegers weggeschnitten (Test `test_guenstiger_beide_zweige_nicht_geschnitten`).
- **Sweep-Netz** (kein stilles Vergessen): jede rules.yaml-Regel mit `günstiger`-Erwähnung
  (p31, p32d_1_abgeltung, p10a_guenstigerpruefung, p32d_1_kirchensteuer — alle außerhalb der Scheibe)
  steht als **benannte Ausnahme** mit Grund. Gate `test_guenstiger_sweep_netz` erzwingt das; extern
  tamper-verifiziert (Ausnahme entfernt → ROT, restauriert → grün).
- **Annahmen nie still:** nicht-askable (berechnete) Geltungsbedingungen sind kein Gate, werden aber
  als `annahmen_offen` geführt (Test `test_relevanz_annahmen_nie_still`).
- **Per-Cent-Grenze ehrlich benannt:** Trace ist Regel/Slot/Feld/Event-genau; per-Cent via
  Catala-Herleitungs-Baum ist ein benannter Nachtrag (KONZEPT.md/API.md), kein still gelöstes Problem.

## API-Vertrag (Paket-B-Naht, `API.md`)

Die Haut LIEST nur Traverser + Bindungstabelle + Store-Snapshot + Unsicherheits-Derivat und SCHREIBT
ausschließlich über **`store.append_event`** (der EINE Schreibpfad). Kein zweiter Schreibpfad, keine
Regel-/Registry-/Engine-Berührung durch die Haut, keine zweite Wahrheits-Quelle. Fünf fail-closed-
Garantien (Meet-Sperre, Zwei-Signal, llm→vorlaeufig, ein-aktives-Event, ERiC-Trunkierungs-Sperre) sind
mechanisch, nicht per Bitte.

## Gesamtstand Paket A (UI-Kern)

Bindungstabelle + Store + Unsicherheits-Derivat + Traverser + API-Vertrag — **zusammen 53 Gate-Tests
grün** (13 + 19 + 10 + 11). Der KI-freie, deterministische UI-Kern-Unterbau steht vollständig; die
Paket-B-Naht ist definiert. Bereit für Paket B (Privat-Haut + LLM-Chat) oder Scheiben-Ausbau.

## Reproduktion

```bash
python3 -m pytest tests/test_traverser.py -q
```
