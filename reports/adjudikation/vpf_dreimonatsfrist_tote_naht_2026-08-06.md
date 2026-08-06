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

## Verdrahtungs-Paket — Analyse (2026-08-06, Ergänzung, NUR gelesen, api.py nicht verändert)

### 1. Namens-Match: `catala_werbungskosten_n` erwartet exakt die Konstanten-Namen

Kein Enum/Lookup-Mismatch. `_verpflegung_roh_cent` (`golden/runner.py:288-291`) liest:
```python
s.get("vpf_tage_24h_nach_drei_monaten", 0)
s.get("vpf_tage_an_abreise_nach_drei_monaten", 0)
s.get("vpf_tage_ueber_8h_nach_drei_monaten", 0)
```
— Zeichen für Zeichen identisch mit `VERPFLEGUNG_TAGE_NACH_FRIST` in
`api_constants.py:47-48`. Der Fix ist eine reine Weiterreichungs-Lücke (Feld kommt
in `f` an, geht nie in `wk_input`/`gesamt_wk_input`), kein Namens-Drift wie beim
DBA-Enum-Fall. Analog zum bestehenden `VERPFLEGUNG_TAGE`-Verdrahtungsmuster
(`gesamt_wk_input[t] = _c(t)` für `t in VERPFLEGUNG_TAGE`,
`produkt/haut/api.py:736`) — derselbe Loop-Bauplan, nur mit der
NACH_FRIST-Tupel und Ziel-Keys unter denselben Namen.

### 2. Betrifft nur `gesamt`, NICHT `an_gesamt` — geprüft, nicht nur vermutet

- `SCHEIBEN["an_gesamt"]["felder"]` (`api_constants.py:351-358`): enthält
  `VERPFLEGUNG_TAGE` und `VERPFLEGUNG_GUARD`, **NICHT** `VERPFLEGUNG_TAGE_NACH_FRIST`.
- Live-Probe gegen den echten Server (an_gesamt-Scheibe, Feld gesetzt):
  ```
  POST /fall/{id}/event {"feld_id":"vpf_tage_24h_nach_drei_monaten", ...}
  → 400 {"fehler": "feld_id 'vpf_tage_24h_nach_drei_monaten' nicht in dieser Scheibe"}
  ```
  Das Feld ist in `an_gesamt` also **hart nicht setzbar** — das Schema weist es
  zurück, bevor überhaupt ein Ring-Aufruf stattfindet. Das ist keine Lücke im
  selben Sinn wie Loch 1 (dort *kommt* das Feld an und wird verworfen; hier kommt
  es gar nicht erst durch).
- Einschätzung: `an_gesamt` rechnet Verpflegung grundsätzlich (`VERPFLEGUNG_TAGE`
  ist verdrahtet, S.3/S.8-11 laufen dort), nur S.6 fehlt komplett — konsistent
  mit "reiner AN-Fall, MVP" (Docstring `api.py:452`, "§ 2 Gesamtsteuer MVP").
  Ich halte es für richtig, `VERPFLEGUNG_TAGE_NACH_FRIST` auch dort zu ergänzen
  (Scheiben-Deklaration + Verdrahtung), weil sonst zwei Scheiben, die dieselbe
  Vorschrift (§9 Abs.4a) ansonsten identisch abbilden, bei Dienstreisen > 3 Monate
  unterschiedlich (und beide falsch: `gesamt` über-abzieht, `an_gesamt` lässt den
  Fall gar nicht zu) reagieren — aber das ist eine Produktentscheidung, kein
  Muss aus der Messung. Entscheidung liegt bei main.

### 3. Guard-Korrektheit — GEPRÜFT, Ergebnis: Guard selbst korrekt, aber ein Nebenfund dabei

Guard-Frage: sperrt `_an_gesamt_sperrgrund` auch dann, wenn der Nutzer
"keine Mahlzeiten gestellt" (=`vpf_keine_mahlzeitengestellung=True`, bestätigt)
geantwortet hat? Live gemessen (gesamt-Scheibe, `tage_24h=60`,
`vpf_keine_mahlzeitengestellung=True` bestätigt, alle Pflichtfelder gesetzt):
```
grund=bestaetigt, zahl_cent=1024500   (kein Sperren — Guard lässt legitime "nein"-Antwort durch)
```
→ Guard-Antwort ist korrekt: "keine Mahlzeiten gestellt" bestätigt-true
entsperrt den Fall wie in `keine_mahlz_wert_true` (`api.py:1651`) vorgesehen.
Kein zu strenger Guard, keine Korrektur nötig.

**Nebenfund (nicht Teil der ursprünglichen Frage, aber bei derselben Messung
aufgefallen):** derselbe Fall mit `vpf_monate_am_ort=4` (>3, "Frist überschritten")
lieferte **exakt denselben Wert** (`zahl_cent=1024500`) wie ein Fall mit
`tage_24h=0` (keine Verpflegung überhaupt). D.h. der bestehende `_mon > 3`-Zweig
(`api.py:735`, `if not (isinstance(_mon, int) ... and _mon > 3):`) lässt bei
Monat > 3 **gar keine** Verpflegungstage mehr in `gesamt_wk_input` — nicht nur
die Tage nach der Frist, sondern alle. Zum Vergleich `vpf_monate_am_ort=3`
(genau an der Grenze, Frist nicht überschritten): `zahl_cent=1008700`, also mit
voller Pauschale gerechnet (60×28€ = 1680€ WK-Wirkung sichtbar in der Differenz
zu 1024500). Das ist over-tax-safe (0 Abzug statt Fake-voller-Abzug bei fehlender
Information), aber es bedeutet: **selbst nach dem Verdrahten von
VERPFLEGUNG_TAGE_NACH_FRIST bleibt der `_mon > 3`-Alles-oder-Nichts-Zweig aktiv**
und muss mit-angefasst werden, sonst bleibt jeder Fall mit `vpf_monate_am_ort > 3`
weiterhin bei 0€ Verpflegungs-WK statt bei der korrekt reduzierten Pauschale
(S.6 rechnet ja genau DAS: nicht "alles oder nichts", sondern "Tage in Frist ja,
Tage danach nein"). Diese Bedingung (`api.py:735` und `api.py:485` im an_gesamt-
Zweig) müsste beim Verdrahten so umgebaut werden, dass sie bei `_mon > 3` NICHT
mehr komplett sperrt, sondern die NACH_FRIST-Felder zusätzlich verlangt/verdrahtet
— sonst bleibt der Alles-oder-Nichts-Pfad ein dritter, bisher unentdeckter
Fehlerzweig neben Loch 1 und Loch 2.

### Aufwandsschätzung

- Verdrahtung `gesamt`-Zweig (NACH_FRIST-Felder in `gesamt_wk_input`,
  analog VERPFLEGUNG_TAGE-Loop): ~15 Min.
- Umbau `_mon > 3`-Bedingung (Alles-oder-Nichts → Tage-in-Frist-vs-danach,
  `gesamt`-Zweig, betrifft `api.py:733-735`): ~30-45 Min. inkl. Nachdenken über
  Wechselwirkung mit dem bestehenden Guard (`_an_gesamt_sperrgrund` sperrt
  aktuell NICHT bei `_mon > 3` — das ist reine Datenfrage, kein Fail-Closed;
  zu klären, ob das so bleiben soll oder ob >3 Monate ohne NACH_FRIST-Angabe
  neu sperren sollte, fail-closed statt 0-Abzug).
- Falls `an_gesamt` mit-verdrahtet wird (Produktentscheidung, s.o.): +Scheiben-
  Deklaration (`api_constants.py`) + identische Verdrahtung im `festzusetzende_est`-
  Zweig (~452-530) + derselbe `_mon > 3`-Umbau dort: weitere ~30 Min.
- `xfail` entfernen, echten Assert wiederherstellen, ggf. auf exakten Wert
  schärfen (Kat-B-Muster): ~15 Min.
- Test-Ergänzung für den `_mon > 3`-Alles-oder-Nichts-Zweig selbst (bisher
  ungetestet, siehe Nebenfund): ~20 Min.
- **Summe: gesamt-only ≈ 1,5h; inkl. an_gesamt ≈ 2h.** Kein Catala-Änderung nötig
  (Ring ist bereits vollständig und korrekt, isoliert verifiziert).
