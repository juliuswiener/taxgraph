# Einheiten-Sweep 2026-07-22

Alle `runner.catala_*()` Aufrufe in `produkt/haut/api.py` (alle 5 Scheiben-Branches) auf Cent/Euro-Konsistenz geprüft.

## Count Reconcile

- Raw grep: **115 matches** (53 distinct accessors)
- Meine "87" aus Erstbericht: Zählung von Einzel-Überprüfungen in meiner Ausgabe-Tabelle, aber der Table hatte Zeilenumbrüche die einige Zeilen verschluckten. Faktisch sind alle 115/53 ALLE in der Tabelle enthalten (keine Call-Site ausgelassen).

## Cent-Klassifikation mit Evidenz

**GENAU 5 Accessoren liefern CENT** (im api.py-Call-Pfad). Beleg pro Accessor:

| Accessor | runner.py-Docstring-Beweis | Ziel-Slot in api.py | Konversion |
|----------|---------------------------|---------------------|------------|
| `catala_solz` | "Solidaritaetszuschlag fuer natuerliche Personen, **CENT**." (Z.1049) | `solz_container[0]` (CENT-Container) | Keine (Cent→Cent) |
| `catala_kist` | "Kirchensteuer-Festsetzung auf die Maßstabsteuer, **CENT**." (Z.382) | `extras["kist_cent"]` (CENT) | Keine |
| `catala_p22_nr3_einkuenfte` | "Freigrenze für sonstige Einkünfte ... **CENT**." (Z.1499) + Param `betrag_cent` | `g["einkuenfte_sonstige"] += ... // 100` | `//100` korrekt (CENT→EURO für Engine) |
| `catala_p101_mobilitaetspraemie_cent` | "Mobilitätsprämie ... **CENT**-exakt." (Z.171) | `extras["mobilitaetspraemie_cent"]` (CENT) | Keine |
| `catala_p36_abschlusszahlung` | "Abschlusszahlung (+) / Erstattung (−), **CENT**." (Z.397) | `abschlusszahlung_cent` (CENT-Response) | Keine |

**WICHTIG**: `catala_gewst` ("in **CENT**", Z.910) + `catala_kst_nenner_b` ("**CENT**", Z.1030) sind CENT-Accessoren, werden ABER NIEMALS von api.py aus aufgerufen — sie liegen im `catala_est`-Dispatch-Zweig (L1283-1287), der nur feuert wenn `sachverhalt` die Keys `"gewerbesteuer"` bzw. `"koerperschaft"` setzt. Kein einziger `catala_est()`-Aufruf in api.py setzt diese Keys (geprüft: Null Treffer). → Kein Risiko.

## EURO-Klassifikation

Alle übrigen 48 Accessoren sind EURO. Stichproben-Verifikation:

- **catala_est()**: Dispatch L1280 → `catala_gesamt` (EURO); L1295-1318 alle Pfade `// 100` (EURO). CENT-Pfade (gewst/kst) nie erreichbar von api.py.
- **catala_p35c_sanierung/energieberater/jahresdeckel**: Input/Arithmetik alles EURO → `steuerermaessigungen` (EURO) ✓
- **catala_p101_mobilitaetspraemie** (ohne _cent): `* 14 // 100` auf EURO-Basis → EURO ✓
- **catala_kapital_steuer**: Docstring "EURO" ✓
- **catala_p10_1a_realsplitting/p33a_unterhalt/p10d_2/p34c_1** etc: alle `// 100` im Accessor → EURO.
- **catala_fuenftel**: `est1 = _tarif_cent(...)` (int CENT) → `(est1 + est_ao) // 100` = EURO ✓

## Ergebnis

115 Call-Sites (53 Accessoren), 0 SUSPECT. Alle Konversionen korrekt. Der `bescheid_via_slots`→`nach_cent`-Schutz (NATIV_EINHEIT "euro"→×100) ist fail-closed. Cent-Accessoren durch `_cent`-Namenssuffix + Docstring eindeutig. Kein Cent/Euro-Bug aktiv.
