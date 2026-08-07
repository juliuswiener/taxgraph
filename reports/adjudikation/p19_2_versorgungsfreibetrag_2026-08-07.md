# § 19 Abs. 2 Versorgungsfreibetrag — `test_n`-Blindstelle, Rest-Kandidat aus BACKLOG

Datum: 2026-08-07. Auftrag: `p19_2_versorgungsfreibetrag` (BACKLOG `test-n-blindstelle-ohne-ground-truth`,
Feld `offene_frage_p19_2`) prüfen — Befund verifizieren, toten Code messen, Geldfrage klären,
Anschlusskosten beziffern. Reiner Messauftrag, KEIN Code geändert.

---

## 1. Stimmt der dokumentierte Befund noch?

**Ja, wörtlich bestätigt.** Feld-IDs vs. Slot-Namen verbatim gegenübergestellt:

**Bindung** (`produkt/bindung/bindung_an_gesamt.yaml:33,51`):
```yaml
- feld_id: versorgung_jahresrente
  quelle: {regel_id: p19_2_versorgungsfreibetrag, signatur_slot: jahresrente}
...
- feld_id: versorgung_bemessungsgrundlage
  quelle: {regel_id: p19_2_versorgungsfreibetrag, signatur_slot: bemessungsgrundlage}
```
Slot-Namen: `jahresrente`, `bemessungsgrundlage` (ohne `versorgung_`-Präfix).

**Accessor** (`golden/runner.py:916`, AST-gelesen mit derselben `_runner_dict_inputs`-Funktion,
die `test_n` für `RUNNER_ACCESSOR_FUER_REGEL`-Einträge nutzt):
```python
def catala_p19_2_versorgungsfreibetrag(s: dict) -> int:
    ...
    bg = int(s.get("versorgung_bemessungsgrundlage") or s.get("versorgungsbezuege_bemessungsgrundlage", 0))
    beginn = int(s.get("versorgung_beginn_jahr") or s.get("versorgungsbeginn_jahr", 0))
```
Gelesene Dict-Keys: `{versorgung_beginn_jahr, versorgung_bemessungsgrundlage, versorgungsbeginn_jahr,
versorgungsbezuege_bemessungsgrundlage}` — Feld-IDs (mit `versorgung_`-Präfix, plus zwei
Altnamen-Fallbacks), NICHT die Bindungs-Slot-Namen `jahresrente`/`bemessungsgrundlage`.

**Ring-Aufruf-Frage** (drittes Detail im Befund: "Ring ruft `catala_einkuenfte_versorgung`, nicht
den Freibetrag-Accessor"): teilweise ungenau — der Ring ruft `catala_einkuenfte_versorgung`
DIREKT (`api.py:868`), aber diese Funktion ruft `catala_p19_2_versorgungsfreibetrag` INTERN
(`golden/runner.py:992`). Der Freibetrag-Accessor läuft also im Produktionspfad — nur nicht als
direkter Ring-Aufruf, sondern eine Ebene tiefer. Für die `test_n`-Frage ist das irrelevant
(`RUNNER_ACCESSOR_FUER_REGEL` würde ohnehin auf `catala_p19_2_versorgungsfreibetrag` zeigen, nicht
auf den Ring-Call), aber für Punkt 2 (toter Code?) ist es der entscheidende Unterschied — s.u.

## 2. Ist der Freibetrag-Accessor toter Code?

**Nein — gemessen, nicht geraten.** Grep + Ring-Pfad-Nachvollzug:

```
$ grep -rn "catala_p19_2_versorgungsfreibetrag(" --include="*.py" .
./tests/test_versorgung_p19_2.py:26   (Unit-Tests, direkter Aufruf)
./tests/test_versorgung_p19_2.py:115  (Unit-Test, fail-closed)
./golden/runner.py:916                (Definition)
./golden/runner.py:992                (Aufruf INNERHALB catala_einkuenfte_versorgung)
```
Kein Aufrufer in `produkt/haut/api.py` — der Ring ruft nur `catala_einkuenfte_versorgung`
(`api.py:868`). Aber `catala_einkuenfte_versorgung` ruft `catala_p19_2_versorgungsfreibetrag`
selbst auf (`golden/runner.py:992`, `try: vfb_plus_zuschlag =
catala_p19_2_versorgungsfreibetrag(s) except VersorgungsfreibetragOffen: return 0`) — mit
GENAU demselben `s`-Dict, das der Ring an `catala_einkuenfte_versorgung` übergibt (Feld-ID-Namen).

Empirisch bestätigt per Spy-Wrapper um `catala_p19_2_versorgungsfreibetrag` während eines echten
`/ergebnis`-Durchlaufs (`gesamt`-Scheibe, `versorgung_jahresrente=3000000`,
`versorgung_bemessungsgrundlage=3000000`, `versorgung_beginn_jahr=2005`):
```
catala_p19_2_versorgungsfreibetrag wurde 1 mal aufgerufen waehrend /ergebnis
    input= {'versorgung_jahresrente': 30000, 'versorgung_bemessungsgrundlage': 30000,
            'versorgung_beginn_jahr': 2005} -> VFB+Zuschlag= 3900
```

**Antwort: Der Accessor ist NICHT toter Code. Er läuft im Produktionspfad, jeden Aufruf von
`catala_einkuenfte_versorgung` — nur nicht als direkter Ring-Aufruf, sondern intern
weitergereicht.** Die Beschreibung im BACKLOG ("Ring ruft `catala_einkuenfte_versorgung`, nicht
den Freibetrag-Accessor") ist für den DIREKTEN Ring→Accessor-Call korrekt, aber übersieht die
interne Weiterleitung — der Accessor wird nicht umgangen, sondern eine Ebene tiefer aufgerufen,
mit denselben Feld-ID-Keys.

## 3. Rechnet der Ring den Versorgungsfreibetrag trotzdem korrekt? (Geldfrage)

**Ja — gemessen, Zahl stimmt.**

Handrechnung § 19 Abs. 2 (Kohorte 2005: 40 %, Höchstbetrag 3.000 €, Zuschlag 900 €,
`params/kohorten/versorgungsfreibetrag_p19_2.yaml:14`):
```
BG = 30.000 €
VFB_roh = 40 % × 30.000 = 12.000 € → gedeckelt auf Höchstbetrag 3.000 €
Zuschlag-Deckel = 30.000 − 3.000 = 27.000 € >> 900 € → Zuschlag voll 900 €
VFB + Zuschlag = 3.000 + 900 = 3.900 €
```
Gemessene Ring-Ausgabe (Spy auf `catala_p19_2_versorgungsfreibetrag`, s.o.): `VFB+Zuschlag = 3900`
— exakt die Handrechnung.

Differential-Beweis, dass der Betrag auch numerisch in die festgesetzte Steuer einfließt (nicht
nur berechnet und verworfen wird): dieselben 30.000 € einmal als Versorgung mit VFB, einmal als
normaler Bruttoarbeitslohn ohne VFB-Weg (kein Freibetrag, nur der allgemeine WK-Pauschbetrag,
viel kleiner):
```
Referenz (kein Einkommen):                          bestaetigt        0 ct
MIT Versorgung + VFB (2005-Kohorte, BG=30k):         bestaetigt   318500 ct
Vergleich: dieselben 30k als Bruttolohn (kein VFB):  bestaetigt   394600 ct
```
`318500 < 394600` — die Versorgung mit VFB erzeugt eine NIEDRIGERE Steuer als derselbe Betrag
ohne Freibetragsweg, obwohl beide Fälle dieselben 30.000 € Bruttoeinnahmen haben. Das ist nur
möglich, wenn der VFB tatsächlich von der Bemessungsgrundlage abgezogen wird, bevor die Steuer
berechnet wird — der Freibetrag wirkt korrekt.

(Ergänzend, bereits VOR diesem Auftrag bestehend und hier nur bestätigend nachvollzogen:
`tests/test_paket_b_e2e_http.py::test_gesamt_versorgungsfreibetrag_kohortenvergleich_2005_vs_2040`
und drei weitere `test_gesamt_versorgungsfreibetrag_*`-Tests prüfen denselben Ring-Pfad bereits
seit Existenz der Datei, alle grün — `timeout 60 python3 -m pytest -q
tests/test_paket_b_e2e_http.py -k versorgungsfreibetrag` → `5 passed`.)

**Kernaussage: Der Namens-Mismatch (`jahresrente`/`bemessungsgrundlage` in der Bindung vs.
`versorgung_jahresrente`/`versorgung_bemessungsgrundlage` im Accessor) ist eine
`test_n`-Rückrichtungs-Blindstelle, KEIN Geldfehler.** `api.py` liest die Feld-IDs direkt aus dem
Store (`f.get("versorgung_bemessungsgrundlage", ...)`, `api.py:832`) und übergibt sie unter
denselben Feld-ID-Namen an den Accessor (`api.py:870`) — die Bindungs-`signatur_slot`-Namen
(`jahresrente`, `bemessungsgrundlage`) werden im Ring-Pfad nirgends benutzt, sie sind nur eine
Doku-Annotation in der YAML, die `test_n` (Rückrichtung: Bindung → Regel-Signatur) prüfen würde,
wenn die Regel eine Ground Truth hätte. Da `p19_2_versorgungsfreibetrag` bislang KEINE Ground
Truth in `test_n`s Sinn hat, wird der Mismatch dort nicht geprüft — aber er wird auch nirgendwo
im Rechenweg selbst wirksam, weil dort durchgehend Feld-ID-Namen verwendet werden.

## 4. Anschlusskosten an `test_n` — Mechanik oder Entscheidung?

Gemessen (nicht gebaut): `RUNNER_ACCESSOR_FUER_REGEL["p19_2_versorgungsfreibetrag"] =
"catala_p19_2_versorgungsfreibetrag"` probeweise gesetzt und `_n_gefundene_verstoesse` (dieselbe
Funktion, die `test_n` nutzt) direkt gegen `bindung_an_gesamt.yaml` laufen lassen:

```
neue geltungsbedingung-Verstoesse: 3
  ('an_gesamt', 'versorgung_alter_bei_beginn', 'p19_2_versorgungsfreibetrag', 'altersgrenze_sonstige_alter')
  ('an_gesamt', 'versorgung_art', 'p19_2_versorgungsfreibetrag', 'art_beamtenrechtlich_oder_nicht')
  ('an_gesamt', 'versorgung_beginn_jahr', 'p19_2_versorgungsfreibetrag', 'versorgungsbeginn_kohorte')
neue signatur_slot-Verstoesse: 2
  ('an_gesamt', 'versorgung_bemessungsgrundlage', 'p19_2_versorgungsfreibetrag', 'bemessungsgrundlage')
  ('an_gesamt', 'versorgung_jahresrente', 'p19_2_versorgungsfreibetrag', 'jahresrente')
```
Identisches Ergebnis, ob `RUNNER_ACCESSOR_FUER_REGEL` auf `catala_p19_2_versorgungsfreibetrag`
ODER auf `catala_einkuenfte_versorgung` zeigt (beide Varianten gemessen, beide liefern dieselben
5 Verstöße) — weil `_n_gefundene_verstoesse` für `RUNNER_ACCESSOR_FUER_REGEL`-Einträge IMMER
`gbs = set()` setzt (keine `geltungsbedingungen` aus `rules.yaml` verfügbar, `test_bindungstabelle.py:1140`),
und `catala_einkuenfte_versorgung` selbst nur `versorgung_jahresrente` als Dict-Key liest (die
anderen 4 Bindungsfelder gehen nur indirekt über den internen Call an
`catala_p19_2_versorgungsfreibetrag`, den `_runner_dict_inputs` nicht mitverfolgt).

**Ursachen der 5 Verstöße:**
- 3× `geltungsbedingung` (`versorgungsbeginn_kohorte`, `art_beamtenrechtlich_oder_nicht`,
  `altersgrenze_sonstige_alter`): `RUNNER_ACCESSOR_FUER_REGEL`-Pfad kennt strukturell KEINE
  `geltungsbedingungen` (kein `rules.yaml`-Eintrag) — dasträfe JEDE Regel mit
  `geltungsbedingung`-Bindung, die über diesen Mechanismus angeschlossen wird, nicht nur p19_2
  (vgl. die bereits akzeptierten `[Lücke]`-Einträge für `p10_1_9_schulgeld`/
  `p33_2a_fahrtkostenpauschale` in `SIGNATUR_SLOT_ZEIGT_INS_LEERE`/
  `GELTUNGSBEDINGUNG_ZEIGT_INS_LEERE`, `test_bindungstabelle.py:976-978`).
- 2× `signatur_slot` (`jahresrente`, `bemessungsgrundlage`): der eigentliche
  Feld-ID-vs.-Slot-Name-Mismatch aus Punkt 1.

**Mechanik oder Entscheidung?** Beides, aufgeteilt:
- Die 2 `signatur_slot`-Verstöße sind MECHANISCH lösbar — entweder Bindung auf die echten
  Dict-Keys (`versorgung_jahresrente`/`versorgung_bemessungsgrundlage`) umstellen, oder (näher am
  bestehenden Präzedenzfall `p10_1_9_schulgeld`/`p33_2a_fahrtkostenpauschale`) die 2 Verstöße in
  `SIGNATUR_SLOT_ZEIGT_INS_LEERE` als dokumentierte Ausnahme aufnehmen. Keine Ermessensfrage,
  reine Nomenklatur.
- Die 3 `geltungsbedingung`-Verstöße sind strukturell — genau wie beim `p10_1_9_schulgeld`/
  `p33_2a_fahrtkostenpauschale`-Präzedenzfall braucht jeder `RUNNER_ACCESSOR_FUER_REGEL`-Anschluss
  eine `geltungsbedingung`-Ausnahmeliste, weil das Pattern strukturell keine `rules.yaml`-Bedingungen
  kennt. Das ist eine EINMALIGE Design-Entscheidung (schon getroffen, s. `SIGNATUR_SLOT_ZEIGT_INS_LEERE`/
  `GELTUNGSBEDINGUNG_ZEIGT_INS_LEERE`-Kommentar `test_bindungstabelle.py:973-975`: "kein loses Ende, nur
  nicht rules.yaml-prüfbar"), keine neue Entscheidung für p19_2 — nur deren Anwendung.

**Aufwand geschätzt (Mechanik, keine neue Design-Entscheidung nötig):** 1 Zeile
`RUNNER_ACCESSOR_FUER_REGEL["p19_2_versorgungsfreibetrag"] = "catala_p19_2_versorgungsfreibetrag"`
+ 5 Zeilen in den beiden Ausnahmelisten (analog zu den 3 bestehenden `[Lücke]`-Einträgen für die
zwei bereits angeschlossenen Regeln) + 1 Zeile `REGELN_OHNE_GROUND_TRUTH` streichen. ~15 Minuten,
kein Rot-Grün-Zyklus im Sinne eines Bugfixes nötig (kein Bug), nur Sichtbarkeits-Buchhaltung.
Nicht gebaut — Julius hat noch nicht entschieden.

## Zusammenfassung, wenn gefragt

**Der Punkt ist kleiner als er im BACKLOG aussieht.** Die Geldfrage (Punkt 3, die wichtigste) ist
geklärt: der Ring rechnet den Versorgungsfreibetrag korrekt, gemessen mit Handrechnung UND
Differential-Beweis. Der Freibetrag-Accessor ist kein toter Code — er läuft, nur indirekt über
`catala_einkuenfte_versorgung`. Was übrig bleibt, ist ausschließlich der `test_n`-Anschluss selbst
(Sichtbarkeits-Buchhaltung, kein Bug), und der ist zu ~90 % Mechanik (identisch zum bereits
gelösten `p10_1_9_schulgeld`/`p33_2a_fahrtkostenpauschale`-Muster) — nur die Wahl, OB man die 2
`signatur_slot`-Verstöße per Bindungs-Umbenennung oder per Ausnahmeliste schließt, ist eine kleine
Entscheidung (keine Geld- oder Korrektheitsfrage).

## GATE

Befehl: `timeout 500 python3 -m pytest -q`
```
1655 passed, 4 skipped, 1 warning in 224.06s (0:03:44)
```
Exit-Code: 0. Identisch zur Referenz (1655/4) — kein Code geändert, keine Verschiebung erwartet
und keine gemessen.

## Aufräumen

`git status --short` vor/nach Messung: nur dieser Report neu (`?? reports/adjudikation/
p19_2_versorgungsfreibetrag_2026-08-07.md`). Temp-Skripte `/tmp/p19_2_measure.py`,
`/tmp/p19_2_measure2.py`, `/tmp/p19_2_hookup_cost.py` gelöscht.

## Nicht gemessen

1. Ob eine dritte Variante (Bindung umbenennen statt Ausnahmeliste) irgendwelche anderen
   Verbraucher der `signatur_slot`-Namen `jahresrente`/`bemessungsgrundlage` bricht — nicht
   gegrept, weil reine Messaufgabe zur Ist-Lage, kein Bauvergleich verlangt.
2. Die zwei Altnamen-Fallbacks im Accessor (`versorgungsbezuege_bemessungsgrundlage`,
   `versorgungsbeginn_jahr`) — woher/wozu sie kommen, nicht recherchiert (git blame), für die
   drei gestellten Fragen nicht relevant.

## Status

Kein Code geändert. Nichts committed.
