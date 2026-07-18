# Repeated-Instance-KERN — gebaut, freeze-ready (dev-2, 2026-07-18)

**Status:** gebaut + grün (volle Suite 459 passed / 2 skipped). Der generische Instanz-MECHANISMUS
(Store-Modell A, Klasse INSTANZ) — unabhängig vom ersten Konsumenten. Freeze zuerst, dann Multi-Objekt-§21.

## Separator-ENTSCHEIDUNG: `base__<n>` statt `base#i` (Store-Invarianz-treu)
Der Zuschnitt/GO nannte `feld_id#i`. **`#` bricht aber das Store-Schema-Gate:** das Store-Event-`feld_id`-
Pattern ist `^[a-z][a-z0-9_]*$` (validiert von jsonschema in test_store.py/test_beleg_writer.py) — `#` ist
nicht drin. Die HARTE Auflage "Store-Kern bleibt UNVERÄNDERT" schlägt die illustrative `#`-Notation.
→ **Suffix `__<n>` (n≥2)**: liegt vollständig in `[a-z0-9_]` → das Store-Pattern bleibt WÖRTLICH unverändert,
`vv_einnahmen__2` ist ein valides feld_id, Store-Gate bleibt grün. Belegt in beide Richtungen:
`test_store_feld_id_pattern_unveraendert_akzeptiert_instanz` (\_\_2 grün) + `test_hash_separator_wuerde_store_gate_brechen`
(`#`-Gegenprobe rot). Kollisionsfrei: 0 bestehende `__`-feld_ids. **Wenn du doch `#` (+ Pattern-Aufweichung)
willst → sag Bescheid; 1-Zeilen-Regex-Swap.**

## Modell
- **Instanz 1 = Basis-feld_id** (ohne Suffix) — unverändert in `deklaration`/`dokumentiert` (NULL Regression
  für alle Einzel-Objekt-Goldens/Tests). **Instanz 2..N = `base__n`** → neuer `anlage_instanzen`-Bucket.
- **Kz-Reuse je Instanz** (kein neuer Kz — analog Person-B/Klasse g auf der Instanz-Achse; Recon 2026-07-18:
  alle drei Anlagen V/R/Kind = Reuse). 1:1-Basis → `instanz.felder[kz]`; Aggregat-Quelle (Klasse a) →
  `instanz.dokumentiert[ziel]` (Summe je Objekt).
- **Store lernt die Instanz GAR NICHT** — reine Konvention der Bindung (`instanz_gruppe`) + est_mapping.

## Geänderte Dateien (dev-2-Zone)
- `produkt/bindung/schema.json`: +`instanz_gruppe`-Property (optional, Pattern `^[a-z][a-z0-9_]*$`).
- `produkt/mapping/est_mapping.py`: `_INSTANZ_RE` + `_deklariere_instanz()`; `deklariere()` erkennt `base__n`
  am Schleifen-Kopf → `anlage_instanzen`-Bucket; `zuruecklesen()` Instanz-Round-Trip (`base__idx` + `E….__idx`).
  Result +`anlage_instanzen: {gruppe: [{index, felder{kz→wert}, dokumentiert?}]}`.
- **Store-Kern (`produkt/store/`): UNVERÄNDERT** (Schema + Code) — das war die Design-Auflage.
- `tests/test_instanz_kern.py` (NEU, 11 Tests, synthetische Bindung — kein reales Feld getaggt = Konsument-Schritt):
  1:1-Reuse über N, Aggregat je Objekt, Round-Trip, fail-closed je Instanz, nicht-instanzfähig-Guard,
  Kz-Reuse-kein-Phantom (Drift-Awareness), Store-Pattern-Invarianz + `#`-Gegenprobe, Determinismus.

## Drift-Awareness
Kein `__n` in der STATISCHEN Bindung (Instanzen sind Laufzeit) → Drift-Wächter unberührt, bleibt grün. Die
Instanz-Kz sind Basis-1:1-Kz (⊆ erlaubte Menge) → kein Phantom (bewiesen `test_instanz_kz_reuse_kein_phantom`).
Wenn der Konsument reale Felder taggt, aktiviert sich die Awareness ohne Wächter-Umbau.

## Nächster Schritt (nach Kern-Freeze)
Erster Konsument = **Multi-Objekt-§21**: die vv-Felder in `bindung_kap_vv_familie.yaml` mit
`instanz_gruppe: vv_objekt` taggen + Goldens (2-Objekt-Fall) + reale-Bindung-Tests. Ring-Verdrahtung
(Multi-Instanz-Summe in catala_gesamt) = dev-1-Nachtrag. Danach Per-Kind (ELSTER-Pflicht).

## Zur Abnahme
(1) `__<n>`-Separator statt `#` OK (Store-Invarianz)? (2) Instanz-1=Basis / 2..N=Bucket-Split OK?
(3) Multi-Objekt-§21 als erster Konsument OK? → dann Konsument-Bau.
