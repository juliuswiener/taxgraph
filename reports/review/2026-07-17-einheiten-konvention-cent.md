# Einheiten-Konvention Naht Kern↔Engine — normalize-to-cent (Task #11, Paket A)

**Anlass:** EP-Durchstich (dev-1) zeigte 21,56 € statt 2156 €. Root cause: `haut/api.py._bescheid_fn`
ruft `catala_entfernungspauschale` (liefert **EURO** 2156), `intervall.py` labelt die Achse `min_cent`,
`app.js euro(2156)=/100=21,56`. Die `golden/runner.py`-Accessoren liefern **gemischt** euro/cent.

**Konvention (Instructor-freigegeben, Option B):** kanonische Naht-Einheit = **CENT** end-to-end (deckt
sich mit Store-Inputs `typ:cent`, den vorhandenen `_cent`-Labels, dev-1s `euro()=/100`). Ein Adapter
normalisiert die Engine-Nativ-Ausgabe je Quantität verlustfrei auf Cent — **kein Key-Rename**, kein
Contract-Bruch.

## Ist-Erhebung: Ausgabe-Einheit je Accessor (Beleg = `//100` + golden-Erwartungswert-Key)

| Accessor (golden/runner.py) | Sachverhalt-Marker | Beleg | Nativ-Einheit | golden-Key | # Goldens |
|---|---|---|---|---|---|
| `catala_entfernungspauschale` | `entfernung_km_roh` | `//100` (L87) | **EURO** | `abziehbarer_betrag` | 4 |
| `catala_raumkosten` | `arbeitszimmer_vorhanden` | `//100` (L64) | **EURO** | `abzug_gesamt` | 3 |
| `catala_gesamt` / AN-Endfall | `gesamtfall` / `bruttoarbeitslohn` | `//100` (L355/409) | **EURO** | `festzusetzende_est` | 11 |
| Tariflicher Fall / `catala_fuenftel` | `zu_versteuerndes_einkommen` / `ausserordentliche_einkuenfte` | `//100` (L423/468/473) | **EURO** | `tarifliche_est` | 55 |
| `catala_gewst` | `gewerbesteuer` | `_cent` (L178/179) | **CENT** | `gewst_cent` | 10 |
| `catala_kst_nenner_b` | `koerperschaft` | `_cent` (L299) | **CENT** | `nenner_b_cent` | 15 |
| `_p35c_ermaessigung_cent` | `sanierungsaufwendungen` | `_cent` (L382) | **CENT** | `sanierung_ermaessigung_cent` | 3 |
| `_kfz_nutzungswert_monat_cent` | `bruttolistenpreis` | `_cent` (L384) | **CENT** | `nutzungswert_monat_cent` | 1 |

**Einheiten-Wahrheit = golden-Erwartungswert-Key** (`*_cent` → cent, sonst euro). Alle 8 Quantitäten
sind golden-belegt → **keine Accessor-ohne-Golden-Lücke** (Auflage B erfüllt). Hinweis: `catala_est`
und `catala_gewst` dispatchen SELBST euro ODER cent je nach Sachverhalt — die Einheit hängt an der
**Quantität**, nicht am Top-Level-Funktionsnamen; darum ist `quantitaet` (golden-Key) der Map-Schlüssel.

## Umbau (nur `produkt/`-Naht, NULL Engine-/Golden-Änderung)

- `produkt/unsicherheit/intervall.py`: `NATIV_EINHEIT`-Map (8 Quantitäten) + `nach_cent(wert, quantitaet)`
  (euro→×100 verlustfrei, cent→identisch, **ungemappt → ValueError 'Einheit unbelegt'**, Auflage A).
  `bescheid_via_slots(bindung, slot_fn, *, quantitaet)` normalisiert die Ausgabe; früher Wurf bei
  unbelegter Quantität. Keine Ergebnis-Keys umbenannt (`min_cent/max_cent/spanne_cent` bleiben, sind
  jetzt WAHR).
- `tests/test_einheiten.py` (neu, 8 Tests): (A) Map deckt genau die golden-Quantitäten + Unbelegt-Wurf;
  (B) Map-Einheit == golden-Key-Konvention; (C) exakt gegen echten Accessor+Golden (EP 2156→215600 über
  alle 4 EP-Goldens, Nenner-B unverändert über alle 15). Map-Tamper-verifiziert (EP euro→cent ⇒ 4 rot).
- `tests/test_paket_a_e2e.py`: EP via `quantitaet="abziehbarer_betrag"`, Erwartung 2156 → **215600**.
- `tests/test_unsicherheit.py`: Summanden-Test mit cent-Quantität (identity, unit-neutral).
- `produkt/README.md`: Einheiten-Konvention + Gate-Liste **75/75**.

## Grün

Volle Paket-A-Suite + Einheiten-Gate mit Catala-Toolchain: **75/75 grün** (0 skips). Auflage C echt
verifiziert (nicht geskippt). Auflage A/B: Map vollständig, fail-closed, konventionskonsistent.

## Offen / geflaggt (nicht angefasst)

- **ELSTER-Kz-Format** (euro/cent je Vordruck-Feld) = Submission-Layer — nur geflaggt, `est_mapping`
  bleibt Store-Cent (konsistent mit der Naht).
- **dev-1-Koordinationspunkt (1 Zeile, Zonen-Micro-Order):** `produkt/haut/api.py:115`
  `IV.bescheid_via_slots(bindung, slot_fn)` → `IV.bescheid_via_slots(bindung, slot_fn, quantitaet="abziehbarer_betrag")`
  (EP-Scheibe; je Haut-Scheibe der passende golden-Key). Ohne die Zeile wirft der Adapter beim
  Verdrahten (fail-closed) — kein stiller Fehlbetrag.
