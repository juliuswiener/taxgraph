# elster/ - ELSTER-Feldmodell und Feldmapping (M1.3) + ERiC-Anbindung (Phase 4)

**Status: ERiC 44.2.4.0 vorhanden + Smoke-Test READY (2026-07-12). Feldmapping weiter Stub.**

## ERiC-Bibliothek (as-is-Software, AUSSERHALB des Repos)

Die ERiC-Distribution (offizielle ELSTER Rich Client Bibliothek, ~682 MB nativ)
liegt nach Julius-Konvention unter `~/02_Software/eric/` und wird NICHT ins Repo
eingecheckt. Der Pfad kommt aus der Umgebung, nicht hart aus dem Code:

```sh
export ERIC_DIR=~/02_Software/eric          # entpackte Linux-x86_64-Distribution
ERIC_DIR=~/02_Software/eric python elster/smoke_test.py
```

Ohne `ERIC_DIR` sucht der Smoke-Test den Default `~/02_Software/eric/` ab. Ablage:
`~/02_Software/eric/extracted/ERiC-44.2.4.0/Linux-x86_64/` (`lib/libericapi.so`,
`lib/plugins/libcheckESt_*.so`, `include/`, `Beispiel/`).

- **Version:** ERiC 44.2.4.0 (API 44.2.4), libericapi Produktversion 44,2,4,0.
- **Ladbar:** ja — `ctypes.CDLL` ok, `EricInitialisiere`/`EricVersion`/`EricBeende` -> `ERIC_OK`.
- **ESt-Jahresmodule (checkESt) offline:** VZ 2015-2025. **Kein VZ 2026** (Modul folgt
  mit spaeterer ERiC-Auslieferung; VZ-2026-Veranlagung oeffnet erst 2027).
- **Offline-CI-Gate:** machbar. `EricBearbeiteVorgang(xml, "ESt_<vz>", ERIC_VALIDIERE)`
  (Flag `1<<1`, ohne `ERIC_SENDE`=`1<<2`) prueft lokal per Plugin-`.so`, ohne Netz,
  ohne Credentials, ohne Versand. `ERIC_VALIDIERE_OHNE_FREIGABEDATUM` (`1<<8`) fuer
  Rand-/Vor-Freigabe-Faelle.

Befund im Detail: `reports/review/2026-07-12-eric-smoke-befund.md`.

---

## Feldmapping (M1.3) — weiter Stub, pending Feldmodell

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
