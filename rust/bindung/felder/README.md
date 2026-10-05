# Felder, die nur Rust kennt

Hier liegen `bindung_*.yaml`-Dateien fuer Felder, die es in Python nicht gibt (Weg B leicht,
Entscheidung 2026-10-05, Vault `decisions/weg-b-leicht-und-art9-schutz-als-test.md`).

- **Gleiches Format** wie `produkt/bindung/bindung_*.yaml` (`produkt/bindung/schema.json`).
- **Python liest dieses Verzeichnis nicht.** `make serve-python` kennt diese Felder nicht und
  rechnet sie weiter falsch; das ist eine bewusste Abweichung (Liste, Test, Commit-Begruendung).
- **Der Dienst laedt beide Verzeichnisse** (`bindung::lade_registry_der_wurzel`). `feld_id` und
  Dateiname sind ueber beide eindeutig.
- **Die Python-Orakel-Tests lesen weiter nur `produkt/bindung`.** Ein neues Feld hier macht
  `make unit` und die Orakel-Tests (`interview`, `intervall`) nicht rot.
- **Eine Scheibe nimmt ein Feld nur auf, wenn es in `rust/bescheid/src/deklaration/scheiben_tabellen.rs`
  steht** (von Hand gepflegt). `rust/bescheid/tests/scheiben_tabellen_konsistenz.rs` prueft, dass jedes
  dort genannte Feld in der Registry steht und der Kegel in den Feldern liegt.
- **Neue Dateien:** `bindung_<name>.yaml`; `FELD_BESTAND.yaml` und andere Dateien ohne Praefix
  `bindung_` ignoriert der Lader.
