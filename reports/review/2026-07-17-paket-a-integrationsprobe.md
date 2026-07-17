# Paket-A-Integrationsprobe — die 4 Säulen als System (Task #11)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor · NULL LLM.

## Datei

- `tests/test_paket_a_e2e.py` — ein durchgehendes End-to-End-Szenario (EP-Familie), **grün, nicht
  geskippt** (Catala-Toolchain vorhanden). Alle Paket-A-Gates zusammen: **54/54.**

## Was bewiesen wird (System, nicht Einzelteile)

Ein einziger Fall wandert durch alle vier Säulen:

1. **Traverser (Interview):** leerer Store → `naechste_fragen` liefert genau die 4 EP-Felder.
2. **Store (Schreibpfad):** `append_event` mischt 3× laie-bestätigt (Entfernung/Kfz/ÖPNV) + 1×
   **llm-VORLÄUFIG** (Arbeitstage, `schreiber=llm:chat`). Danach steht NUR das offene Feld in der Queue.
3. **Unsicherheit:** das Intervall spannt auf der bounded-Achse Arbeitstage (0..366) — Spanne > 0,
   keine offene Achse (die anderen sind bestätigt/fix).
4. **Fail-closed VORHER:** `meet_zustand` über den Input-Kegel = `vorlaeufig` → es gibt **keine** feste
   Zahl (`_feste_zahl` = None). Der Typ als Enforcement.
5. **Zwei-Signal-Bestätigung:** der LLM-Wert wird per `append_event` mit `ersetzt=<llm-event_id>` +
   `signal_2` bestätigt (die menschliche Bestätigung neben dem Beleg).
6. **Unsicherheit schrumpft:** nach der Bestätigung ist das Intervall ein **Punkt** (Spanne 0) —
   monoton verengt.
7. **Fail-closed NACHHER:** Kegel = `bestaetigt` → die echte festzusetzende Zahl fällt: die
   Entfernungspauschale **2.156 €** (220 Tage × 30 km, VZ 2025, via echtem Catala über
   `bescheid_via_slots`), gleich der Intervall-Untergrenze.
8. **Vorwärts-Trace:** `trace_ergebnis` liefert die Justification bis zum `anker_ref` (§ 9-Anker,
   deckungsgleich mit der Bindungstabelle), Zustand `bestaetigt`, `signal_2` gesetzt, Herkunft `laie`
   (nach der Bestätigung, nicht mehr llm).

## Bedeutung

Das ist der Beweis, dass Bindungstabelle + Store + Unsicherheits-Derivat + Traverser **als ein System**
zusammenspielen — und zugleich die **lebende Spezifikation für Paket B**: die Haut folgt exakt diesem
Ablauf (fragen → schreiben → Intervall → bestätigen → Zahl → Trace), ausschließlich über den
API-Vertrag (`produkt/traverser/API.md`), mit dem EINEN Schreibpfad und der mechanischen KI-Sperre.

## Reproduktion

```bash
python3 -m pytest tests/test_paket_a_e2e.py -v   # grün mit Toolchain, sonst sauberer Skip
```
