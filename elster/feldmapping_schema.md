# Feldmapping-Tabelle - Format

Bildet Regel-Outputs (aus der Catala-Regelbibliothek) auf ELSTER-Feld-IDs ab.
Ein eigenes, reviewbares Artefakt, getrennt von den Regeln. Deterministisch,
kein LLM.

Datei: YAML mit einer Liste `mapping` von Zeilen. Eine Zeile je
Regel-Output/Feld-Paar.

```yaml
mapping:
  - regel_output: string       # kanonischer Bezeichner eines Regel-Outputs,
                               # z.B. "estg_p32a.Grundtarif.tarifliche_steuer"
    elster_feld_id: string     # amtliche ELSTER-Feld-ID (bis dahin Platzhalter)
    elster_feld_name: string   # Klartextbezeichnung des Feldes
    anlage: enum               # Mantelbogen | Anlage N | Anlage Vorsorgeaufwand |
                               #   Anlage Kind
    typ: enum                  # euro | integer | bool | string | prozent
    pflicht: bool              # Pflichtfeld laut ELSTER-Schema
    veranlagungszeitraum: int|"alle"
    status: enum               # stub | mapped | verified
    quelle: string             # ELSTER-Schema-Referenz (bei stub: "pending Zugang")
```

## Pflichtfelder je Zeile

`regel_output`, `elster_feld_id`, `anlage`, `typ`, `pflicht`, `status`.

## Statuswerte

- `stub`: Platzhalter-Feld-ID, ELSTER-Schema noch nicht angebunden.
- `mapped`: gegen das amtliche Schema gesetzte, aber noch nicht end-to-end
  verifizierte Feld-ID.
- `verified`: Mapping durch Testversand / checkESt bestaetigt (Phase 4).

## Invarianten (von validate_mapping.py geprueft)

- Pflichtfelder vorhanden, Enums gueltig.
- `elster_feld_id` innerhalb einer Anlage und eines VZ eindeutig.
- `regel_output` folgt dem Punktschema `<modul>.<scope>.<feld>` bzw.
  `<modul>.<feld>`.
