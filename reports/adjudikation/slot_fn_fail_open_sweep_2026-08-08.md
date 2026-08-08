# slot-fn fail-open Sweep — Discovery + 2 Mutationsproben (bruttoarbeitslohn, gesamt/an_gesamt)

Datum: 2026-08-08. Auftrag (dev-b): repo-weite Blast-Radius-Messung von `bescheid_via_slots()`s
`slots.get(name, default)`-Fail-Open (Präzedenz: main's 13.568-€-Fund,
`p2_festzusetzung_slot_verstoesse_2026-08-08.md`). Reiner Messauftrag: KEIN Code geändert,
nichts umbenannt, nichts committed. Alle Mutationen dieser Sitzung liefen ausschließlich auf
einer isolierten `/tmp`-Kopie (`git archive HEAD | tar -x`, dann `oracle/gettsim/_catala` per
Symlink zurück auf den Live-Checkout) — der Live-Baum wurde zu keinem Zeitpunkt mutiert.

---

## Teil 1 — Discovery: alle Aufrufer + alle Leser klassifiziert

### 1.1 Alle Aufrufer von `bescheid_via_slots` (repo-weit)

| Ort | `quantitaet` | Scheibe (`SCHEIBEN[...]["gesamt_ring"]`) |
|---|---|---|
| `produkt/haut/api.py:469` | `abziehbarer_betrag` | `ep` |
| `produkt/haut/api.py:630` | `festzusetzende_est` | `an_gesamt` |
| `produkt/haut/api.py:1250` | `festzusetzende_est_gesamt` | `gesamt` |
| `produkt/haut/api.py:1628` | `festzusetzende_est_rentner` | `rentner_gesamt` |
| `tests/test_paket_a_e2e.py:71` | `abziehbarer_betrag` | Test-Duplikat der EP-slot_fn, keine eigene Produktionsgefahr |
| `tests/test_unsicherheit.py:138` | (Modell-Lambda) | testet nur die Summanden-Konvention, kein realer Leser |
| `tests/test_einheiten.py:59` | (unbekannt) | testet nur den `ValueError`-Fail-Closed-Pfad selbst |

`api.py:469` (EP-Familie) war in keinem der bisherigen Reports untersucht — vierte, bisher
blinde Aufrufstelle.

### 1.2 Drei Klassen von `slots`-Lesarten (statt zwei)

- **Klasse 1 — `slots.get(name, default)`, fail-open**: `arbeitstage`→0, `entfernung_km_roh`→0,
  `oepnv_kosten_jahr`→0 (via `_oepnv_eur`-Helper, api.py:261), `eigenes_oder_ueberlassenes_kfz`→
  False, `bruttoarbeitslohn`→0 (drei Stellen: api.py:592, :574, :846), `veranlagung`→"einzel"
  (api.py:590).
- **Klasse 2 — bare `slots[name]`, fail-closed (KeyError)**: **0 Treffer** in api.py per AST.
- **Klasse 3 — NEU, bisher nicht auf main's Schirm**: `{k: slots[k] for k in (...) if k in
  slots}` (api.py:485, :743). Sieht wie Subscript-Zugriff (fail-closed) aus, ist es aber nicht —
  das `if k in slots` verhindert den KeyError, ein fehlender Slot lässt den Key im `wk_input`
  einfach WEG statt 0 einzusetzen oder zu werfen. `catala_werbungskosten_n` (`golden/runner.py:
  180-205`) gated jede WK-Komponente selbst wieder mit `if "X" in s:` — ein fehlender Key
  überspringt die ganze Komponente (z. B. Entfernungspauschale), reduziert die Werbungskosten →
  over-tax, aber über einen strukturell anderen Mechanismus als Klasse 1.

### 1.3 Richtungsklassifikation (unter-tax / over-tax / unklar)

| Slot | Default | Richtung | Begründung |
|---|---|---|---|
| `bruttoarbeitslohn` | 0 | **under-tax** | Einkommensfeld → 0 verliert steuerpflichtiges Einkommen |
| `veranlagung` | "einzel" | **unklar** | Splitting-Vorteil kann in beide Richtungen kippen — nicht pauschal zuordenbar, bewusst nicht geraten |
| `arbeitstage`, `entfernung_km_roh`, `eigenes_oder_ueberlassenes_kfz`, `oepnv_kosten_jahr` | 0/False | **over-tax** | alle vier sind §9-Entfernungspauschale-Inputs — 0/False-Default REDUZIERT den Abzug |

---

## Teil 2 — Mutationsproben: die 2 `bruttoarbeitslohn`-Lesestellen, die main's Probe NICHT abdeckte

Main's eigene Probe lief nur gegen Scheibe `an_gesamt`/einzel (api.py:592). Offen waren:
**api.py:846** (`festzusetzende_est_gesamt`, unconditional) und **api.py:574**
(`bruttoarbeitslohn_a` im `zusammen`-Zweig von `festzusetzende_est`, Scheibe `an_gesamt`).

### Design-Einschränkung: eine einzige Bindungszeile

`bruttoarbeitslohn` hat GENAU EINEN Bindungseintrag (`bindung_an_gesamt.yaml:15`), der über
`lade_bindung()` (merged alle `bindung_*.yaml`) von ALLEN drei Lesestellen konsumiert wird. Eine
Mutation trifft also automatisch alle drei gleichzeitig — Isolation kam über die
Eingabe-Szenarien, nicht über getrennte Mutationen:

1. **einzel/gesamt** (Scheibe `gesamt`, `veranlagung=einzel`) → trifft NUR api.py:846 (574 läuft
   nicht, das ist ein anderer `quantitaet`-Zweig).
2. **zusammen/gesamt** (Scheibe `gesamt`, `veranlagung=zusammen`) → trifft api.py:846 (Person A)
   PLUS die separate Person-B-Berechnung (api.py:861-864, direkter Feld-ID-Zugriff, unbetroffen).
3. **zusammen/an_gesamt** (Scheibe `an_gesamt`, `veranlagung=zusammen`) → trifft api.py:574
   (`bruttoarbeitslohn_a` in `catala_est_zusammen`).

Kegel-Werte NICHT geraten — reale, bereits testverifizierte Fixtures wiederverwendet:
`tests/test_stille_null_offen.py::_KEGEL`/`REFERENZ=1392400` für Szenario 1,
`tests/test_paket_b_e2e_http.py::test_gesamt_zusammen_beide_verdiener` (A=40000€, B=30000€ →
1077600 ct) für Szenario 2, `tests/test_paket_b_e2e_http.py::test_an_gesamt_zusammen`
(A=B=40000€ → 1383800 ct) für Szenario 3.

### Ergebnisse

Alle drei Baselines auf der isolierten Kopie VOR jeder Mutation gemessen und decken sich exakt
mit den zitierten Test-Referenzwerten (Bestätigung, dass die Kopie funktionsgleich zum Live-Baum
ist). Mutation: `bindung_an_gesamt.yaml:15` `signatur_slot: bruttoarbeitslohn` →
`bruttoarbeitslohn_x` (identisch zu main's Original-Mutation), via echtem HTTP-Server
(`server.make_server`) — `POST /fall` → `POST /fall/<id>/event` (Laien-Herkunft, bestätigt) →
`GET /fall/<id>/ergebnis`.

| Szenario | Aufrufstelle | Baseline (ct) | Nach Mutation (ct) | Delta (€) | grund |
|---|---|---|---|---|---|
| einzel/gesamt | api.py:846 | 1.392.400 | **0** | 13.924 € | bestaetigt (unverändert) |
| zusammen/gesamt | api.py:846 + Person-B (861-864) | 1.077.600 | **72.400** | 10.052 € | bestaetigt (unverändert) |
| zusammen/an_gesamt | api.py:574 | 1.383.800 | **295.600** | 10.882 € | bestaetigt (unverändert) |

**Erwartung (stille Null wie an_gesamt) für Szenario 1 bestätigt**: identisches Muster zu main's
Original-Fund — Steuer fällt exakt auf 0, `grund` bleibt `"bestaetigt"`, kein Fehler, kein
Statuswechsel.

**Szenarien 2+3 sind der interessantere Befund — kein Guard, aber auch keine volle Null.** Beide
Zusammenveranlagungs-Fälle fallen NICHT auf 0, obwohl derselbe fail-open Mechanismus greift.
Grund: bei `zusammen` wird Person A's Einkommen zwar auf 0 mutiert (der `bruttoarbeitslohn`-Slot
ist betroffen), aber Person B's Einkommen läuft über einen KOMPLETT ANDEREN, direkten
Feld-ID-Zugriff (`_c("bruttoarbeitslohn_partner")`, api.py:575 bzw. 864/908) — dieser Kanal ist
von der Slot-Mutation unberührt. Der Rest-Betrag ist also nicht Person A + Person B, sondern nur
noch die Steuerwirkung von Person B (bzw. bei Szenario 3 die Splitting-Tarif-Wirkung eines
Ehepaars mit fingiertem Einkommen 0/40.000 statt 40.000/40.000). Es feuert also KEIN Guard,
KEINE Sperre — der fail-open-Defekt wirkt exakt gleich (Person A's reales Einkommen verschwindet
lautlos aus der Steuerlast), nur ist der resultierende Betrag ungleich 0, weil die
Zusammenveranlagung strukturell zwei unabhängige Einkommens-Kanäle summiert und nur einer davon
den mutierten Slot durchläuft. Unter-tax bleibt bestätigt in allen drei Szenarien — der
Fehlbetrag (10.052 € bzw. 10.882 €) entspricht ungefähr der Steuerwirkung des jeweils
verschwundenen Person-A-Einkommens unter Splitting-Tarif, keine Zufallszahl.

---

## Bewusst nicht gemessen

1. **Die 4 EP-Slots (`arbeitstage`, `entfernung_km_roh`, `eigenes_oder_ueberlassenes_kfz`,
   `oepnv_kosten_jahr`) an allen drei unabhängigen Lesestellen (api.py:469, 611-628, 743)** —
   klassifiziert als over-tax (0/False-Default REDUZIERT den Abzug). Over-tax ist KEIN
   harmloser Fehler — ein Nutzer, der deshalb zu viel zahlt, hat vollen Schaden, und niemand
   reklamiert ihn (kein Finanzamt findet ihn). Nachrangig gemessen, weil under-tax-Kandidaten
   in dieser Reihenfolge zuerst dran waren (main's Priorisierung der Messreihenfolge, keine
   Aussage über Schwere).
2. **`veranlagung`→"einzel"-Fallback** — Richtung genuinely fallabhängig (Splitting-Vorteil kann
   in beide Richtungen kippen), keine pauschale Mutationsprobe ohne einen konkreten,
   asymmetrischen Einkommensfall sinnvoll; nicht Teil des unter-tax-Auftrags.
3. **Klasse-3-Lesestellen (api.py:485, :743) selbst gemessen** — Konsequenz (WK-Komponente
   entfällt) ist über `catala_werbungskosten_n`s eigenes `if "X" in s:`-Gating bereits am Code
   nachvollzogen, kein eigener `/ergebnis`-Differenzbeweis gefahren (over-tax-Richtung, gleiche
   Priorisierung wie Punkt 1).
4. **api.py:469 (EP-Familie, `abziehbarer_betrag`) selbst mutiert** — dieselben 4 Slot-Namen wie
   Punkt 1, over-tax, kein eigener Messlauf.

## Gate

Kein Gate gelaufen als Teil dieser Messung — keine pytest-Suite ausgeführt, kein Code im
Live-Baum geändert. Alle Zahlen oben sind reale, reproduzierbare HTTP-Messungen
(`server.make_server` → echter Request/Response-Zyklus) auf der isolierten `/tmp`-Kopie, keine
Unit-Test-Assertions.

## Status

Kein Code im Live-Repo geändert. Nichts umbenannt im Live-Baum. Nichts committed — main
committet. Alle Mutationen ausschließlich auf `/tmp/slot_probe_devb_copy` (git-archive-Kopie von
HEAD `5e9ad73`), vor Abschluss der Messung vollständig auf Baseline zurückgesetzt (`diff` gegen
Backup leer). `tests/test_slot_fn_reader_existiert.py` (dev-a's Datei) nicht angefasst.
