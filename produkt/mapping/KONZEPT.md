# est_mapping-Schicht — Konzept-Skizze (Task #11, Paket A)

**Zone:** `produkt/mapping/` (neu, additiv, **NULL LLM**, rein deterministisch). **Status:** Konzept-
Skizze zur Instructor-Abnahme VOR dem Bau (Schema/Konzept-first). Die deterministische Übersetzung
**Store-Snapshot → ELSTER-Deklaration** — genau die Fälle, die die Bindungstabelle ehrlich als GAP
ausgelagert hat (kein 1:1-Kz).

## Andock-Punkt (nicht neu erfinden)

- **`elster/feldmapping.stub.yaml`** — die vorhandene Mapping-Tabelle (`regel_output` dotted →
  `elster_feld_id` + `anlage` + `typ` + `status`), validiert von `elster/validate_mapping.py`.
- **`produkt/bindung/`** — `feld_id → {signatur_slot, elster_kz (1:1), typ, herkunft_slots}`.
- **`produkt/store/`** — der materialisierte Snapshot (`feld_id → {wert, zustand, herkunft}`).
- `elster/est_mapping.py` existiert NOCH NICHT → **neu** ist nur die **Transform-Logik** (die 5
  Fall-Klassen), nicht die Kz-Tabelle. (Ort offen, s. Punkt 5 — Vorschlag `produkt/mapping/est_mapping.py`.)

## Signatur

`deklariere(snapshot, bindung) → deklaration` mit `deklaration = {E-Nr → wert, kind_anlagen: [...]}`.
**Rein deterministisch**, Funktion des Store-Snapshots. **Fail-closed:** nur `zustand=bestaetigt`-Werte
werden deklariert; hängt ein `vorlaeufig` im Pflicht-Kegel, ist die Deklaration UNVOLLSTÄNDIG (kein
Versand — analog zur festzusetzenden Zahl, Meet aus dem Store-Typ). **Snapshot-gebunden:** die
Deklaration bindet an den `snapshot_id` (reproduzierbar).

## Die 5 Fall-Klassen (die Transform-Regeln)

| # | Klasse | Beispiel | Transform | Round-Trip |
|---|---|---|---|---|
| 1 | **1:1** | kap_kapitalertraege→E1900701, vv_einnahmen→E0700201, VOR-Summanden→E2000401/801/601 | Wert direkt an `elster_kz` (aus Bindungstabelle) | **exakt** (Wert == zurückgelesen) |
| a | **Aggregation** (§21-WK) | {vv_gebaeude_afa, vv_schuldzinsen, vv_erhaltungsaufwand, vv_sonstige_wk} | SUMME → E0703838, **Zuordnungsart-Fallunterscheidung** ([Einz]/[Sum]/[Direkt]/[Verhaelt]) | **verlustbehaftet**: Round-Trip auf AGGREGAT-Ebene (Σ Detail-Slots == E0703838); Details bleiben Store-Wahrheit |
| b | **Split** (VOR) | gesamtbeitraege_inkl_ag (Regel-Slot) ↔ 3 Deklarations-Kz | die Regel aggregiert die Summanden; die Deklaration deklariert die 3 Summanden EINZELN (E2000401/801/601) — kein Deklarieren der Summe | **exakt** je Summand |
| c | **Berechnete Größe** | zumutbare_belastung, entgelt_quote_prozent | **NICHT deklariert** (das FA rechnet sie); im est_mapping ausgeschlossen | keiner (nicht in der Deklaration) |
| d | **Negation** | fam_alleinstehend | Store `alleinstehend=true` → EfA-Feld E0503701/E0503821 = false/leer (keine schädliche Haushaltsgemeinschaft); `false` → true | **invertierbar** (Doppel-Negation == Store) |
| e | **Multiplikation** | anzahl_kinder = N | → N Anlage-Kind-Instanzen (je Kind ein Sub-Dokument mit Per-Kind-Kz) | **Zähl-Round-Trip** (Zahl der Kind-Anlagen == anzahl_kinder) |

## Round-Trip-Gate (dev-2-Lab N3 „Vertrauen ist die Kante")

`zuruecklesen(deklaration) → felder`, dann `felder == Store-Werte` (je Klasse):
- **1:1 / Split / Negation:** invertierbar → exakte Gleichheit je Feld.
- **Aggregation:** **einweg-verlustbehaftet** — der Round-Trip prüft NUR die Summe (Σ Store-Details ==
  deklarierte Summe), NIE die Details (die sind aus der Deklaration nicht rekonstruierbar; der Store
  bleibt ihre Wahrheit). Ehrlich benannt, kein stiller Detail-Verlust.
- **Berechnete:** nicht deklariert → nicht round-getripped (benannte Ausnahme).
Das ist die mechanische Garantie „was der Store hält, steht so (oder als benannte Aggregation) in der
Deklaration" — die Kehrseite der Bindungstabellen-Anker-Doktrin auf der Deklarationsseite.

## Gate (`tests/test_est_mapping.py`, geplant)

(a) Determinismus; (b) fail-closed (vorlaeufig-Feld → nicht deklariert / Deklaration unvollständig-Flag);
(c) Round-Trip je Klasse (1:1 exakt, Aggregation aggregat-genau, Negation Doppel-Neg, Multiplikation
Zähl); (d) 1:1-Kz-Konsistenz mit der Bindungstabelle `elster_kz`; (e) berechnete Größen NICHT in der
Deklaration. Negativtests (verfälschte Summe → Round-Trip rot; vorlaeufig durchgereicht → rot).

## Offene Punkte zur Abnahme (Instructor-Entscheid)

1. **§21-WK-Zuordnungsart-Fallunterscheidung:** die Wahl [Einz]/[Sum]/[Direkt]/[Verhaelt] hängt an der
   **Zahl der Vermietungsobjekte** + AfA-Zuordnung. **Vorschlag:** MVP = **Einzel-Objekt → E0703838
   [Einz]**; Multi-Objekt/[Direkt]/[Verhaelt] als benannte Lücke (braucht ein Store-Feld
   `anzahl_vermietungsobjekte` + Zuordnungs-Modell). OK?
2. **Kind-Anlagen-Multiplikation:** die Per-Kind-Kz sind noch GAP (Kind-Anlage nicht voll gemappt).
   **Vorschlag:** MVP = Anzahl + Anspruchs-Flags; Voll-Kz-Ausbau der Anlage Kind später. OK?
3. **Fail-closed-Granularität:** Deklaration wird als „unvollständig" markiert, sobald EIN Pflicht-Feld
   vorlaeufig/offen ist (kein Teil-Versand). OK, oder feiner (pro Anlage)?
4. **Round-Trip bei Aggregation** = aggregat-genau (Summe), Details bleiben Store-Wahrheit. Bestätigen?
5. **Ort:** `produkt/mapping/est_mapping.py` (UI-Kern-nah, konsumiert Store+Bindung) statt `elster/`
   (das bleibt die ERiC-Seite). OK, oder in elster/?

Nach Abnahme: `est_mapping.py` (die 5 Fall-Klassen + fail-closed + snapshot-Bindung) + Round-Trip-Gate.
