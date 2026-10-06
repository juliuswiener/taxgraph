# Die Bindung, die der Rust-Dienst liest

Hier liegen die `bindung_*.yaml`-Dateien, die `bindung::lade_registry_der_wurzel` laedt, dazu
`FELD_BESTAND.yaml`, `schema.json` und `SCHEMA.md`. Weg B voll, Entscheidung 2026-10-05
(Vault `decisions/weg-b-leicht-und-art9-schutz-als-test.md`, Folgeentscheidung B voll).

- **Rust-Besitz.** Das Verzeichnis begann am 2026-10-05 als byteweise Kopie von `produkt/bindung`
  (sha256 je Datei gleich, 28 Dateien) und waechst ohne Python weiter. Wer ein Feld anlegt oder
  aendert, aendert nur hier.
- **Python liest dieses Verzeichnis nicht.** `produkt/bindung` bleibt eingefroren fuer
  `make serve-python` und die Python-Tests. `make serve-python` kennt neue Felder nicht und rechnet
  sie weiter falsch; das ist eine bewusste Abweichung (Liste, Test, Commit-Begruendung).
- **Kein Rust-Code nennt `produkt/bindung`.** Jeder Lesezugriff geht ueber
  `bindung::lade_registry_der_wurzel`. Das haelt `rust/bindung/tests/keine_python_bindung_im_rust_code.rs`
  fest; die Ausnahmen (Parity-Suiten und Tests, die eine Python-Antwort vergleichen) stehen dort
  mit Grund.
- **Eine `feld_id` kommt in allen Dateien zusammen nur einmal vor** (der Lader prueft das beim Bauen).
- **Eine Scheibe nimmt ein Feld nur auf, wenn es in `rust/bescheid/src/deklaration/scheiben_tabellen.rs`
  steht** (von Hand gepflegt). `rust/bescheid/tests/scheiben_tabellen_konsistenz.rs` prueft, dass jedes
  dort genannte Feld in der Registry steht und der Kegel in den Feldern liegt.
- **Ein neues Feld braucht eine Zeile in `FELD_BESTAND.yaml`** (unter `felder:`, alphabetisch). Sonst
  schuetzt die Ratsche es nicht vor stillem Loeschen. `rust/bindung/tests/feld_bestand.rs::jedes_heutige_feld_steht_im_bestand`
  wird rot und nennt die Zeilen, die einzutragen sind.
- **Neue Dateien:** `bindung_<name>.yaml`; Dateien ohne Praefix `bindung_` ignoriert der Lader.
- **Zaehlende Tests pinnen keine Feldzahl.** Ein neues Feld darf `cargo test --workspace` nicht
  rot machen, ausser dem einen Test, der die Zeile in `FELD_BESTAND.yaml` verlangt (siehe oben), und bei einem fragbaren Feld dem Test, der den Eintrag in `scheiben_tabellen.rs` verlangt (gemessen 2026-10-06: ein Feld mit `askable: false` und eigenem Slot macht nur den Bestand-Test rot); die Orakel-Tests lesen weiter Pythons Eingabe (`interview::python_orakel_registry`).
