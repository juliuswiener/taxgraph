# elster/ - ELSTER-Feldmodell und Feldmapping (M1.3)

**Status: Stub, pending ELSTER-Entwicklerzugang (Julius).**

Das eigentliche ELSTER-Feldmodell (Felder, Typen, Hierarchie, Pflichtstatus)
wird aus dem amtlichen ELSTER-Schema deterministisch geparst, sobald der
Entwicklerzugang vorliegt. Bis dahin ist hier nur das **Format der
Mapping-Tabelle** (Regel-Output -> ELSTER-Feld-ID) als reviewbares Artefakt
definiert, mit einem Stub-Beispiel und Platzhalter-Feld-IDs.

Kein LLM: das Feldmodell und das Mapping sind deterministische Datenbasis. Die
Mapping-Tabelle ist ein eigenes, reviewbares Artefakt (Roadmap M1.3).

## Inhalt

- `feldmapping_schema.md` - Format der Mapping-Tabelle.
- `feldmapping.stub.yaml` - Beispielzeilen mit Platzhalter-Feld-IDs
  (`status: stub`), zeigt Struktur und Anschluss an die Regel-Outputs.
- `validate_mapping.py` - prueft eine Mapping-Datei gegen das Format
  (`make elster-check`).

## Offene Punkte (pending Julius)

1. ELSTER-Entwicklerportal-Zugang / ERiC-Registrierung (laeuft separat).
2. Sobald verfuegbar: ELSTER-Schema fuer die MVP-Anlagen (Mantelbogen, Anlage N,
   Anlage Vorsorgeaufwand, Anlage Kind) parsen -> `elster/feldmodell/<vz>.yaml`
   (Felder, Typen, Hierarchie, Pflichtstatus).
3. Platzhalter-Feld-IDs im Mapping durch die amtlichen IDs ersetzen, `status`
   auf `mapped` bzw. `verified` heben.
