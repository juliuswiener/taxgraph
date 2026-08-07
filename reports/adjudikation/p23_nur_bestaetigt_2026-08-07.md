# p23 `nur_bestaetigt`-Lücke — Messung, roter Test, Fix

Datum: 2026-08-07. Auftrag: prüfen ob `_p23_ansonsten_einkuenfte` den Parameter `nur_bestaetigt`
tatsächlich auswertet (Verdacht aus dem vorigen Bericht unter `not_checked`).

---

## 1. Messung: fließt eine vorläufige p23-Instanz in die bestätigte Rechnung?

Skript (`API.fall_anlegen`/`API.event`/`API.ergebnis`, `API.FAELLE` auf Tempdir, 60k€ Brutto-
Basisfall wie in `test_p23_ueber_ring_accessor`, einmal ohne p23-Felder, einmal mit denselben
vier p23-Feldern aber `zustand="vorlaeufig"` statt `"bestaetigt"`):

```
OHNE p23:                     bestaetigt 1392400
MIT vorlaeufiger p23-Instanz: bestaetigt 3265600
```

Bug bestätigt: die vorläufige Instanz bewegt den Betrag EXAKT so, als wäre sie bestätigt
(3265600 ct = derselbe Wert wie im vorigen Bericht mit `zustand="bestaetigt"`), und `grund`
bleibt `"bestaetigt"` — keine Sperre, kein Hinweis, dass unbestätigte Daten eingeflossen sind.

## 2. Roter Test

Neuer Test `tests/test_p23_accessor.py::test_p23_vorlaeufige_instanz_nicht_in_bestaetigter_rechnung`:
identischer Kegel wie `test_p23_ueber_ring_accessor`, aber alle vier p23-Felder mit
`zustand="vorlaeufig"` gesetzt. Erwartung: Ergebnis muss identisch zum Fall OHNE p23-Instanz sein
(1392400 ct) — nicht nur "kein Crash".

Befehl: `python3 -m pytest -q tests/test_p23_accessor.py::test_p23_vorlaeufige_instanz_nicht_in_bestaetigter_rechnung`

Verbatim (ROT, vor dem Fix):
```
>       assert erg["zahl_cent"] == 1392400, (
            f"vorlaeufige p23-Instanz fliesst in die bestaetigte Rechnung ein: {erg['zahl_cent']} ct "
            "statt 1392400 ct (Basis ohne p23). K2-Zwei-Signal-Verletzung.")
E       AssertionError: vorlaeufige p23-Instanz fliesst in die bestaetigte Rechnung ein: 3265600 ct statt 1392400 ct (Basis ohne p23). K2-Zwei-Signal-Verletzung.
E       assert 3265600 == 1392400
1 failed in 1.05s
```

## 3. Fix

`produkt/haut/api.py::_p23_ansonsten_einkuenfte`, Schleifenkopf. Idiom identisch zu den 9
anderen `EM.instanzen()`-Stellen in derselben Datei (`_gwg_sofortabzug_summe`,
`_kind_kv_pv_summe`, `vv_objekt`-slot, `rente`-slot u.a.):

```python
for inst in instanzen:
    # Zwei-Signal-Filter (Instanz-Pfad, wie gwg/kind/vv_objekt/rente): eine vorläufige
    # p23-Instanz darf bei nur_bestaetigt=True nicht in die festgesetzte Summe.
    if nur_bestaetigt and inst["zustand"] != "bestaetigt":
        continue
    ...
```

## 4. Grüner Test

Befehl: `python3 -m pytest -q tests/test_p23_accessor.py -v`
```
collected 13 items
tests/test_p23_accessor.py .............                                 [100%]
============================== 13 passed in 1.10s ==============================
```

## 5. Vollständige Aufrufstellen-Liste — `EM.instanzen(`, wertet nur_bestaetigt aus?

11 echte Aufrufstellen im Repo (Tests und reine Kommentar-Erwähnungen ausgeschlossen). Main's
Schätzung "84" trifft nicht zu — 84 war die Zahl der Felder-mit-signatur_slot aus dem vorigen
kegel-Lücke-Bericht, eine andere Zählung, kein `EM.instanzen(`-Aufrufzähler.

| # | Zeile | Funktion | wertet nur_bestaetigt aus? |
|---|---|---|---|
| 1 | 166 | `_gwg_sofortabzug_summe` | ja |
| 2 | 229 | `_p23_ansonsten_einkuenfte` | **NEIN → Bug, jetzt gefixt** |
| 3 | 296 | `_kind_kv_pv_summe` | ja |
| 4 | 324 | `_kind_behinderten_pb_daten` | ja |
| 5 | 645 | `_kinderbetreuung_summe` (gesamt-Zweig) | ja |
| 6 | 663 | `_schulgeld_summe` (gesamt-Zweig) | ja |
| 7 | 725 | `vv_objekt` slot_fn (gesamt-Zweig) | ja |
| 8 | 1284 | `_kinderbetreuung_summe` (rentner-Zweig) | ja |
| 9 | 1300 | `_schulgeld_summe` (rentner-Zweig) | ja |
| 10 | 1323 | `rente` slot_fn (rentner-Zweig) | ja |
| 11 | 1926 + 1944 | `_an_gesamt_sperrgrund` (Guard: vv_objekt/rente Instanz-Vollständigkeit) | n/a — kein `nur_bestaetigt`-Parameter; die Funktion ist ein Gate (kein Accumulator) und prüft `inst["zustand"] != "bestaetigt"` UNBEDINGT (index≥2-Instanzen müssen immer vollständig+bestätigt sein, unabhängig vom Rechenmodus). Fail-closed, kein Bug. |

**Ergebnis: p23 war die einzige echte Lücke dieser Fehlerklasse.** Die 9 anderen Accumulator-
Stellen sind korrekt. Der Guard `_an_gesamt_sperrgrund` gehört strukturell nicht zur selben
Frage (kein Parameter, kein Auswertungsversäumnis) — dort ist der direkte `zustand`-Check
bereits die korrekte, striktere Form.

## 6. GATE

Befehl: `timeout 500 python3 -m pytest -q`
```
1655 passed, 4 skipped, 1 warning in 184.91s (0:03:04)
```
Exit-Code: 0. Main's Referenz nach ihrem letzten Commit: 1654 passed/4 skipped — Differenz = der
eine neue Test, wie erwartet.

## 7. Aufräumen

`git status --short`:
```
 M produkt/haut/api.py
 M tests/test_p23_accessor.py
?? reports/adjudikation/p23_bugfix_2026-08-07.md
```
(Letzterer ist der vorige, bereits akzeptierte Bericht — unverändert von mir in dieser Runde.)
`produkt/haut/faelle/*.json` ist vorbestehender, gitignorter Alt-Bestand (nicht von mir erzeugt,
`git check-ignore` bestätigt). Temp-Skript `/tmp/p23_vorlaeufig_repro.py` gelöscht.

## Status

Nichts committed (Regel: main committet).
