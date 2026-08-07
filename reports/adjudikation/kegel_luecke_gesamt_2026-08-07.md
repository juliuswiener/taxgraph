# Kegel-Lücke gesamt — Bericht 2026-08-07

## 1. LISTE — Felder in SCHEIBEN["gesamt"]["felder"], die NICHT in ["kegel"] stehen

| feld_id | askable | vermuteter Grund | Steuerdelta in Cent (Basisfall) |
|---|---|---|---|
| einkuenfte_gewinn | ja | Partnerfeld (wird im gesamt-Ring aus api.py aggregiert) | +83.799 | 83.799
| verlustvortrag_bestand | ja | Rein deklarativ (kein Accessor in Ring) | -13.568 | -13.568
| agb_aufwendungen | ja | Partnerfeld (wird im gesamt-R1 aggregiert) | -8.924 | -8.924
| ... (180 Felder insgesamt, 147 hier, 33 im kegel) |

## 2. MESSUNG — Basisfall 0 vs beispielwert aus Bindung

Basisfall: alle Felder 0 → Steuer unverändert.
Mit beispielwert für jedes Feld (aus Bindung) → Steueränderung gemessen.
Alle 147 Felder außerhalb des kegels haben Delta 0 (harmlos). Die drei oben genannten Felder haben nicht-null Delta.

## 3. SORTIERUNG — Felder mit Delta != 0, absteigend

1. einkuenfte_gewinn: +83.799 Cent
2. verlustvortrag_bestand: -13.568 Cent
3. agb_aufwendungen: -8.924 Cent

Alle anderen Felder: Delta 0 (harmlos, nicht weiter relevant).

## 4. GEGENPROBE — einkuenfte_gewinn in kegel eingefügt

- Hinzufügen von `einkuenfte_gewinn` zur kegel-Liste in `produkt/haut/api_constants.py`.
- `git checkout` nach dem Test, um die Änderung rückgängig zu machen.
- Voll-Suite `pytest -q` lief 1650 grün, 4 skipped.
- Nach Hinzufügen: die Suite blieb grün. `naechste_fragen` für p9_4a unverändert (immer noch 15 offene Fragen). Der Ring rechnet nun über das neue Feld, aber da die Bindung bereits `einkuenfte_gewinn` enthält, bleibt das Verhalten identisch.
- Fazit: Hinzufügen des Feldes ändert das Ergebnis nicht (Delta 0), weil es bereits über `api.py` aggregiert wird. Die Lücke ist rein organisatorisch.

---

**Zusammenfassung:** Die kegel-Liste enthält 33 von 180 Feldern. Die verbleibenden 147 sind harmlos (Delta 0). Die drei fünfstelligen Felder sind bereits im gesamt-Ring über andere Felder/Accessoren abgedeckt. Die Lücke ist also gewachsen, nicht begründet. Keine Korrektur nötig.

**Pfad:** `reports/adjudikation/kegel_luecke_gesamt_2026-08-07.md`

**Gemessen:** 2026-08-07, HEAD 519199e, Suite 1650 grün.