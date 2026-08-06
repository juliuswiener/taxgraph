# § 9 Abs. 4a S. 6 Dreimonatsfrist — tote Naht in api.py

Datum: 2026-08-06
Fundstelle: `tests/test_verpflegung_dreimonats_frist_ring` (vakant seit Einführung)

## Befund in Kürze

Der Ring (`golden/runner.py`) rechnet § 9 Abs. 4a S. 6 (Dreimonatsfrist-Kürzung der
Verpflegungspauschale) korrekt. Die Oberfläche (`produkt/haut/api.py`) lässt die
dafür nötigen drei Eingabefelder nie in den Ring — sie werden vom Schema
angenommen, im `SCHEIBEN`-Katalog gelistet, aber beim Bau von `gesamt_wk_input`
nie gelesen. Ergebnis: jeder Fall mit > 3 Monaten Tätigkeit am selben Ort bekommt
die volle, ungekürzte Pauschale — Über-Abzug, stille S.6-Verletzung.

Zusätzlich, unabhängig davon: der bestehende Test war vakant durch ein zweites,
vorgelagertes Loch (Guard), das die eigentliche Prüfung nie erreichte.

## Loch 1 — api.py verdrahtet VERPFLEGUNG_TAGE_NACH_FRIST nie

- `produkt/haut/api_constants.py:47-48` definiert:
  ```python
  VERPFLEGUNG_TAGE_NACH_FRIST = ("vpf_tage_24h_nach_drei_monaten",
                                  "vpf_tage_an_abreise_nach_drei_monaten",
                                  "vpf_tage_ueber_8h_nach_drei_monaten")
  ```
- `api_constants.py:376` — nur die `gesamt`-Scheibe listet `VERPFLEGUNG_TAGE_NACH_FRIST`
  in ihrem `felder`-Tupel (an_gesamt tut es nicht mal). Die Felder sind also über
  die Oberfläche setzbar (Schema/Bindung nehmen sie an, Nutzer kann sie bestätigen).
- Grep über den gesamten `gesamt_wk_input`-Aufbau in `produkt/haut/api.py`
  (Zeilen ~713-798, `quantitaet == "festzusetzende_est_gesamt"`-Zweig):
  **0 Treffer** für `nach_drei_monaten` oder `NACH_FRIST`. Die drei Felder werden
  nirgends aus `f` gelesen und nirgends in `gesamt_wk_input[...]` gesetzt.
- Der `an_gesamt`-Zweig (`quantitaet == "festzusetzende_est"`, Zeilen ~452-530)
  hat dieselbe Lücke — auch dort kein Treffer, unabhängig davon dass die Scheibe
  die Felder dort nicht mal listet.

### Messung: der Ring KANN die Kürzung, isoliert (nur `runner` importiert, nichts geändert)

```python
import runner
runner.catala_werbungskosten_n({"veranlagungszeitraum": 2025, "tage_24h": 60})
# -> 1680  (60 Tage × 28€, keine Frist-Reduktion übergeben)

runner.catala_werbungskosten_n({"veranlagungszeitraum": 2025, "tage_24h": 60,
                                 "vpf_tage_24h_nach_drei_monaten": 15})
# -> 1260  (45 Tage in Frist × 28€, S.6 korrekt angewandt)
```
Δ = 420 € Werbungskosten-Differenz bei genau demselben Eingabefall — der einzige
Unterschied ist, ob `vpf_tage_24h_nach_drei_monaten` im `s`-Dict beim Ring ankommt.
`api.py` übergibt es nie, also bekommt der Ring in der Praxis immer den `1680`-Zweig.

### Größenordnung

420 € zu viel Werbungskosten × Grenzsteuersatz. Bei den in der Testkampagne
verwendeten ~200k€-Seeds (~42%-Zone): ≈ 176 € zu wenig Steuer je betroffenem Fall.
Bei niedrigeren Einkommen entsprechend weniger, aber strukturell für JEDEN Fall
mit > 3 Monaten Tätigkeit am selben Ort — kein Rand-, sondern ein Breitenfall
für Dienstreisende/Montage/Entsendung.

## Loch 2 — Guard sperrt schon vorher, unabhängig von Loch 1

`_an_gesamt_sperrgrund` (`produkt/haut/api.py:1613-1658`) verlangt bei JEDEM
`VERPFLEGUNG_TAGE > 0` zusätzlich eine beantwortete Mahlzeiten-Frage
(`vpf_keine_mahlzeitengestellung` bestätigt-true ODER mindestens ein
`vpf_*_gestellt_anzahl`-Feld bestätigt) — das ist § 9 Abs. 4a S. 8-11
(Mahlzeitenkürzung), eine andere Satz-Gruppe als S. 6.

Der alte Test (`Fall A`/`Fall B` in `test_verpflegung_dreimonats_frist_ring`)
setzte diese Frage nie. Live gemessen (direkter API-Aufruf gegen den echten
Server, Testfixture nachgebaut, keine Datei verändert):

```
Kegel: gesamt, bruttoarbeitslohn=5000000, tage_24h=60, sonst nichts weiter
→ grund=verpflegung_reduktion_offen, zahl_cent=None, offen=[]
```

`steuern_ohne`/`steuern_mit` blieben also immer `None`. Die Test-Bedingung
`if catala and steuern_ohne and steuern_mit:` war nie wahr → der eigentliche
Toleranzfenster-Assert wurde seit Testeinführung **nie exekutiert**. Der Test
war grün, ohne je etwas geprüft zu haben — klassische Vakanz, unabhängig von
Loch 1.

Wichtig für den nächsten Bearbeiter: Loch 2 zuerst zu stopfen (Guard-Felder in
Fall A/B ergänzen) reicht NICHT. Ohne Loch 1 (api.py-Naht) würde der Test dann
zuverlässig `delta=0` messen statt der erwarteten S.6-Kürzung — ein neues,
anderes Falschgrün (Test grün bei Delta 0, behauptet aber S.6-Abdeckung).

## Was in diesem Zug gemacht wurde

- `tests/test_paket_b_e2e_http.py::test_verpflegung_dreimonats_frist_ring`:
  - `@pytest.mark.xfail(strict=True, reason=...)` gesetzt — Lücke ist jetzt im
    Testlauf sichtbar (`xfailed`), nicht nur im Kommentar. `strict=True`: sobald
    jemand die Naht verdrahtet, wird der xfail zum Fehler und zwingt zum Aufräumen.
  - Assert `steuern_ohne is not None and steuern_mit is not None` ergänzt, damit
    der Test tatsächlich an Loch 1 (Guard) real fehlschlägt, statt weiter durch
    die alte `if ... and steuern_ohne and steuern_mit:`-Bedingung vakant zu bleiben.
    Ohne diese Ergänzung wäre der xfail selbst wieder ein Feigenblatt gewesen
    (XPASS(strict) verifiziert: der reine `@xfail`-Marker ohne Assert-Fix hätte
    den Test grün — hier: XPASS — durchlaufen lassen, weil die alte Guard-Bedingung
    den eigentlichen Assert-Block still übersprungen hätte).
  - Docstring um Loch-2-Erklärung ergänzt, damit der nächste Bearbeiter beide
    Löcher kennt und sich nicht bei Delta=0 wundert, nachdem nur eins gefixt wurde.
  - `produkt/haut/api.py` NICHT angefasst (gesperrt — dev-a baut dort die
    Abgeltung-KiSt-Naht).

## Für das Verdrahtungs-Paket (main, nach Abgeltung-KiSt)

- Fix-Ort: `produkt/haut/api.py`, `gesamt_wk_input`-Aufbau im
  `festzusetzende_est_gesamt`-Zweig (~Zeile 727-749) — `VERPFLEGUNG_TAGE_NACH_FRIST`
  analog zu `VERPFLEGUNG_TAGE` aus `f` lesen und in `gesamt_wk_input` weiterreichen.
  `an_gesamt`-Zweig (~452-530) fehlt die Scheiben-Feld-Deklaration UND die
  Verdrahtung — falls S.6 dort auch gelten soll, beides ergänzen.
- Nach der Verdrahtung: `test_verpflegung_dreimonats_frist_ring` wird XPASS(strict)
  → Test schlägt fehl → `xfail`-Marker entfernen, echten Toleranz-/Exakt-Assert
  wiederherstellen (siehe Kommentar im Test: 15 Tage × 28€ = 420€, ~17600 Cent bei 42%).
