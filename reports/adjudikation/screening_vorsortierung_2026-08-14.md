# Screening-Modell: Vorsortierung aller Regeln ohne Ob-Bedingung

Stand 2026-08-14, HEAD 2064f9d. Gemessen, nicht geschätzt: `produkt/bindung/bindung_*.yaml`
gegen `bindung_regel_bedingungen.yaml` und die bool-Gates aus `traverser.relevanz()`.

**Ausgangslage: 35 Regeln ohne jede Ob-Bedingung, zusammen 126 askable Felder.**
Diese Felder kann der Dialog heute nicht abschalten — sie werden jedem gestellt.

## Warum die Zuordnung NICHT automatisch geht

Erster Versuch war eine Heuristik über die §-Nummer der `regel_id`. Sie liefert 10 Treffer und
**drei davon sind falsch**, weil § 32 drei verschiedene Dinge enthält:

| Regel | § | Heuristik sagt | richtig ist |
|---|---|---|---|
| `p32_6_kinderfreibetraege` | 32 Abs. 6 | `kein_kap` | Kinder |
| `p32b_progressionsvorbehalt` | 32b | `kein_kap` | Lohnersatzleistungen |
| `p32d_1_kirchensteuer` | 32d | `kein_kap` | ✓ zufällig richtig |

Dieselbe Falle bei § 33 (agB allgemein / Behinderten-PB / Pflege-PB) und § 10 (Vorsorge /
Kirchensteuer / Schulgeld / Berufsausbildung). Die Paragraphennummer ist ein Ordnungsmerkmal des
Gesetzes, keine Aussage über die Lebenssituation — und Screening fragt nach der Lebenssituation.
Deshalb unten von Hand sortiert.

---

## A — Immer relevant, kein Screening möglich (39 Felder)

Diese Regeln treffen jeden Steuerpflichtigen. Ein Gate hier wäre ein Loch, kein Filter.

| Regel | Felder | warum ohne Screening |
|---|---|---|
| `p2_festzusetzung_einzel` | 20 | Stammdaten, IBAN, Steuernummer, Bruttolohn, Veranlagungsart |
| `p10_1_3_3a_kv_pv` | 9 | KV/PV-Beiträge hat jeder |
| `p36_2_anrechnung` | 5 | Lohnsteuer/Vorauszahlungen — jeder Arbeitnehmer |
| `p10_1_2_altersvorsorge` | 3 | RV-Beiträge — jeder Arbeitnehmer |
| `p51a_kirchensteuer` | 2 | Konfession/Bundesland; „keine" ist auch eine Antwort |

**Aufräum-Befund in `p2_festzusetzung_einzel`:** die Regel trägt neben den Stammdaten auch
`einkuenfte_gewinn`, `gewinn_betriebsart`, `gewst_hebesatz`, `gewst_messbetrag` und
`pv_einnahmen`. Die gehören fachlich zu Gewinneinkünften bzw. Photovoltaik, nicht zur
Festsetzung. Das ist derselbe Sammel-Scope-Fehler, der am 2026-08-12 den Dialog-Killer
verursacht hat (`pv_auf_gebaeude` an `p2_festzusetzung_einzel` riss 24 Felder aus dem Dialog).
Diese fünf Felder umzuhängen ist Voraussetzung dafür, dass Gruppe B unten sauber greift —
sonst hängen Gewinn-Felder an einer Regel, die nie ausgeschlossen werden darf.

## B — Sofort abschaltbar, KEINE neue Frage nötig (28 Felder)

Die vier `kein_*`-Screeningfragen existieren bereits (`p2_einkunftsarten`). Es fehlt nur der
Eintrag in `bindung_regel_bedingungen.yaml`, der die Regel daran hängt.

| Screening-Feld (existiert) | Regel | Felder |
|---|---|---|
| `kein_vuv` | `p21_vermietung_einkuenfte` | 5 |
| `kein_kap` | `p20_6_verlustverrechnung` | 4 |
| `kein_kap` | `p20_9_sparer_pauschbetrag` | 1 |
| `kein_kap` | `p32d_1_kirchensteuer` | 1 |
| `kein_gewinn` | `p15_1_2_mitunternehmer` | 4 |
| `kein_gewinn` | `p4_3_gewinn` | 3 |
| `kein_sonstige` | `p22_1_leibrente_besteuerungsanteil` | 5 |
| `kein_sonstige` | `p23_veraeusserungsgewinn` | 4 |
| `kein_sonstige` | `p22_3_leistungen` | 1 |

**Das ist reine Konfiguration** — neun YAML-Einträge, kein neues Feld, kein Code.

Achtung bei `p22_1_leibrente`: die Regel steht auch im rentner-Kegel. `relevanz()` schließt nur
bei bestätigtem `kein_sonstige=True` aus, ein Rentner antwortet dort `False` — der Kegel bleibt
also intakt. Vor dem Scharfschalten trotzdem mit dem Ehrlich-Lauf gegenprüfen.

## C — Braucht je eine NEUE Screening-Frage (59 Felder)

Nach Hebel sortiert. Eine Frage, ein `regel_bedingungen`-Eintrag je Regel.

### C1 — „Hast du Kinder?" → 22 Felder (größter Einzelhebel)

| Regel | Felder |
|---|---|
| `p32_6_kinderfreibetraege` | 13 |
| `p24b_entlastungsbetrag` | 3 |
| `p10_1_3_kv_pv_kind` | 2 |
| `p33b_abs5_kind_uebertragung` | 2 |
| `p10_1_9_schulgeld` | 1 |
| `p33a_ausbildungsfreibetrag` | 1 |

Ein einziges bool-Feld nimmt 22 Fragen aus dem Dialog jedes Kinderlosen. `p24b` ist streng
genommen „alleinerziehend", was Kinder voraussetzt — hängt also korrekt darunter.

### C2 — „Behinderung oder Pflegebedürftigkeit im Haushalt?" → 10 Felder

`p33b_behinderten_pauschbetrag` (4), `p33_2a_fahrtkostenpauschale` (2),
`p33b_hinterbliebenen_pauschbetrag` (2), `p33b_pflege_pauschbetrag` (2)

### C3 — „Einkünfte oder Steuern aus dem Ausland?" → 6 Felder

`p34c_1_anrechnung_hoechstbetrag` (6)

### C4 — „Betriebsrente, Pension oder Versorgungsbezüge?" → 5 Felder

`p19_2_versorgungsfreibetrag` (5)

### C5 — „Fährst du zur Arbeit?" → 4 Felder

`p09_entfernungspauschale` (4). Trifft die meisten, lohnt aber für reine Homeoffice-Fälle.

### C6 — Einzelfragen mit 1–3 Feldern

| Frage | Regel | Felder |
|---|---|---|
| Unterhalt an bedürftige Person gezahlt? | `p33a_unterhalt` | 3 |
| Energetisch saniert? | `p35c_sanierung_ermaessigung` + `p35c_energieberater_ermaessigung` | 3 |
| Kirchensteuer gezahlt/erstattet? | `p10_1_4_kirchensteuer` | 2 |
| Lohnersatz bezogen (Kranken-/Eltern-/Arbeitslosengeld)? | `p32b_progressionsvorbehalt` | 1 |
| Verlustvortrag aus Vorjahren? | `p10d_2_verlustvortrag_abzug` | 1 |
| Gespendet? | `p10b_spenden` | 1 |
| Berufsausbildung/Erststudium? | `p10_1_7_berufsausbildung` | 1 |

---

## Bilanz

| Gruppe | Felder | Aufwand |
|---|---|---|
| A — bleibt immer | 39 | — (aber 5 Felder umhängen, s. o.) |
| B — sofort abschaltbar | 28 | 9 YAML-Einträge |
| C — neue Screening-Frage | 59 | ~13 neue bool-Felder + 20 YAML-Einträge |
| **abschaltbar gesamt** | **87 von 126 (69 %)** | |

Zum Vergleich: der Ehrlich-Lauf stellt heute 194 Fragen in der Scheibe `gesamt`
(`tests/test_dialog_ehrlich_lauf.py`). Die 87 Felder sind nicht 1:1 Fragen — manche Regeln
werden ohnehin nicht erreicht —, aber die Richtung ist gemessen und nicht geschätzt.

## Empfohlene Reihenfolge

1. **Die 5 fehlplatzierten Felder aus `p2_festzusetzung_einzel` umhängen.** Ohne das greift
   Gruppe B für Gewinn/PV nicht sauber. Gleiche Fehlerklasse wie der Dialog-Killer vom 12.08.
2. **Gruppe B** — neun YAML-Einträge, kein neues Feld. Danach Ehrlich-Lauf gegenmessen.
3. **C1 (Kinder)** — 22 Felder mit einem bool.
4. Rest von C nach Hebel.

Nach jedem Schritt `tests/test_dialog_ehrlich_lauf.py` — der misst genau das, was das Screening
verbessern soll, und ist gegen fail-open abgesichert (Mutationsprobe im Modul-Docstring).
