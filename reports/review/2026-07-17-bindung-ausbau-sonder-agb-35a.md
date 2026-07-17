# Bindungstabelle Scheiben-Ausbau — Sonderausgaben/KiSt + agB §33 + §35a (Task #11)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Zone:** `produkt/bindung/` (additiv). **Kein neues Schema** — gleiches Muster wie die Erst-Scheibe.

## Datei

- `produkt/bindung/bindung_sonder_agb_35a.yaml` — **12 Bindungen + 16 benannte Lücken** über 4 Regeln.
  Der Gate `tests/test_bindungstabelle.py` globt alle `bindung_*.yaml` → deckt die neue Scheibe ohne
  Änderung ab (**13/13 grün**, mit den anderen Paket-A-Gates zusammen 53/53).

## Abdeckung

| Regel | Bindungen | Lücken | verifizierte elster_kz |
|---|---|---|---|
| p10_1_4_kirchensteuer (KiSt gezahlt/erstattet) | 2 | 2 | – (Form-Kz 103/104, E-Nr-Nachtrag) |
| p10_1_7_berufsausbildung | 1 | 1 | – (E-Nr-Nachtrag) |
| p33_1_2_agb_abzug (§33) | 3 | 3 | E0104109 |
| p35a_2_3_haushaltsnahe (§35a) | 6 | 8 | E0161404 / E0161504 / E0161804 |

Alle 13 Zitatanker voll-Länge via `_normalize` gegen die Freezes verifiziert (Gate d). elster_kz gegen
E10-2025 (Gate c). Vollständigkeit deterministisch (Gate b): jeder askable Slot + jede Geltungsbedingung
→ Bindung ODER benannte Lücke.

## Bewusste Lücken (benannt, kein Rate-Mapping)

- **KiSt-E-Nr:** Form-Kz 103 (gezahlt) / 104 (erstattet) sind belegt (Vordruck); die XSD-E-Nr ist per
  Flat-Grep nicht erreichbar (Spalten-Kontext im Label) → `elster_kz: null` + Grund, Sektions-Lookup-
  Nachtrag (wie im kz-kandidaten-Report 2026-07-12 dokumentiert).
- **zumutbare Belastung (§33):** berechnete Zwischengröße = Output von `p33_3_zumutbare_belastung`
  (aus Gesamtbetrag der Einkünfte/Kinder/Splitting) → Lücke, nicht deklariert.
- **§35a-Qualifikationen:** von den 12 Geltungsbedingungen sind 3 echte Laien-Gates (Rechnung+unbare
  Zahlung, Haushalt in EU/EWR, Handwerker ohne öffentliche Förderung); die übrigen 9 sind
  Annahmen/Abgrenzungen, die entweder im Feld-Fragetext materialisiert sind (nur Arbeitskosten, Minijob-
  Basis, Renovierung) oder eigene Sonderfälle außerhalb der Scheibe (Pflege/Heim, gemeinsamer Haushalt) —
  je als benannte Lücke mit Grund.

## fragetext_laie

Alle Fragen laienverständlich, keine Paragraphen im Fragetext (Schema erzwingt es); die §-Referenz hängt
als `anker_ref`. Beispiele: „Wie viel Kirchensteuer hast du dieses Jahr gezahlt?" · „Hattest du größere
außergewöhnliche Ausgaben, die du zwangsläufig tragen musstest?" · „Hast du eine Rechnung bekommen und
per Überweisung bezahlt?"

## Hinweis geteilter Tree

`pipeline/produktion/rules.yaml` zeigt beim Bau `M` (dev-1 mid-edit, nicht meine Änderung); parst
sauber, Gate grün. Nur `produkt/bindung/bindung_sonder_agb_35a.yaml` ist mein neuer, untracked Beitrag.

## Reproduktion

```bash
python3 -m pytest tests/test_bindungstabelle.py -q
```
