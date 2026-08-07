# Ring-Sperrgrund für unbestätigte Instanzen — "stille Null"-Klasse

Datum: 2026-08-07. Auftrag: prüfen ob eine vorläufige Instanz bei allen 9 gefilterten
Aggregat-Stellen still (ohne Sperre/Hinweis) durch die bestätigte Rechnung fällt, wie beim
p23-Bug (jetzt gefixt, aber `grund` blieb `"bestaetigt"`). Reiner Messauftrag, KEIN Code
geändert.

---

## 1. Messung — Tabelle

Basisfall wie in `test_p23_ueber_ring_accessor` (60k€ Brutto, ledig, sonst neutral). Für jede
Instanz-Gruppe: Zielfeld(er) vorläufig gesetzt statt bestätigt, `/ergebnis`, `/fragen`, `/stand`
verglichen mit derselben Referenz ohne das Feld.

| Gruppe | Aufrufer | `grund` bei vorläufiger Instanz | `zahl_cent` | gleich wie Referenz (Instanz fehlt)? | in `/fragen`? | `ring_gesperrt` (`/stand`) |
|---|---|---|---|---|---|---|
| gwg | `_gwg_sofortabzug_summe` | `bestaetigt` | 1392400 | **ja — stille Null** | ja | None |
| kind (kinderbetreuungskosten) | `_kinderbetreuung_summe` | `bestaetigt` | 1392400 | **ja — stille Null** | ja | None |
| p23 | `_p23_ansonsten_einkuenfte` | `bestaetigt` | 1392400 | **ja — stille Null** (nach dem Zahl-Fix von heute) | ja | None |
| vv_objekt (Basis-Instanz, index 1) | Kegel-Pflichtfeld selbst | `input_kegel_nicht_bestaetigt` | None | **nein — gesperrt** | ja | None (aber `grund` zeigt Blockade) |
| rente (Basis-Instanz, index 1, `rentner_gesamt`) | Kegel-Pflichtfeld selbst | `input_kegel_nicht_bestaetigt` | None | **nein — gesperrt** | (nicht separat geprüft, Muster identisch zu vv_objekt) | — |
| vv_objekt (Zusatz-Instanz, index ≥2) | `_an_gesamt_sperrgrund` → `vv_instanz_offen` | `vv_instanz_offen` | None | **nein — gesperrt** | ja | `vv_instanz_offen` |
| rente (Zusatz-Instanz, index ≥2) | `_an_gesamt_sperrgrund` → `rente_instanz_offen` | **gemessen 2026-08-07**: `rente_instanz_offen` | None | **nein — gesperrt** | (Muster identisch zu vv_objekt, bestätigt) | `rente_instanz_offen` |
| kind (KV/PV) | `_kind_kv_pv_summe` | **gemessen 2026-08-07**: `bestaetigt` | 1392400 | **ja — stille Null** | (nicht separat geprüft, `/fragen`-Mechanismus identisch zu kinderbetreuungskosten) | None |
| kind (Behinderten-PB-Übertragung) | `_kind_behinderten_pb_daten` | **gemessen 2026-08-07**: `bestaetigt` | 1392400 | **ja — stille Null** | (dito) | None |
| kind (schulgeld) | `_schulgeld_summe` | **gemessen 2026-08-07**: `bestaetigt` | 1392400 | **ja — stille Null** | (dito) | None |

**Kernbefund:** Es gibt ZWEI strukturell verschiedene Klassen unter den 9 Stellen, keine
einheitliche Lücke:

- **Klasse A — Kegel-Basisfelder / Basis-Instanz (index 1) von vv_objekt und rente**: liegen
  im Pflicht-`kegel` der Scheibe. `/ergebnis` prüft das VOR jeder Zahl (`offen = [f for f in
  scheibe_felder if ... zustand != "bestaetigt"]`) → `grund = "input_kegel_nicht_bestaetigt"`,
  `offen` listet das Feld. Fail-closed, gemeldet. Kein Bug.
- **Klasse B — Zusatz-Instanzen (index ≥2) von vv_objekt und rente**: geschützt durch den
  expliziten Guard in `_an_gesamt_sperrgrund` (`vv_instanz_offen`/`rente_instanz_offen`),
  gemessen und bestätigt (`grund` UND `ring_gesperrt` zeigen es). Fail-closed, gemeldet. Kein
  Bug.
- **Klasse C — reine Aggregat-Summen ohne Kegel-Zugehörigkeit und ohne Guard: gwg, kind
  (alle drei kind-Aufrufer), p23**: KEIN Sperrgrund-Mechanismus geprüft diese Gruppen. Die
  vorläufige Instanz wird von der Summe übersprungen (Filter tut, was er soll — Zahl bleibt
  korrekt), aber `grund` bleibt `"bestaetigt"` und nichts in der `/ergebnis`-Antwort zeigt an,
  dass eine Eingabe des Nutzers ignoriert wurde. **Das ist die "stille Null" — jetzt für ALLE
  6 Klasse-C-Funktionen einzeln gemessen und bestätigt: `_gwg_sofortabzug_summe`,
  `_kinderbetreuung_summe`, `_schulgeld_summe`, `_kind_kv_pv_summe`,
  `_kind_behinderten_pb_daten`, `_p23_ansonsten_einkuenfte` (letztere strukturell stille
  Null nach dem heutigen Zahl-Fix — keine dieser Gruppen hat ein Kegel- oder
  `_an_gesamt_sperrgrund`-Gate).**

Gemessen (2026-08-07, alle mit Referenz-ohne-Feld / vorläufig / bestätigt-Kontrolle
nebeneinander — Kontrolle beweist, dass das Feld bei Bestätigung tatsächlich Δ hat, damit
„gleich wie Referenz" nicht durch Wirkungslosigkeit vorgetäuscht wird):
- `_kind_kv_pv_summe` (60k€-Basisfall, `kind_idnr="99988877766"`, `kind_kv=100000`,
  `kind_pv=50000`): Referenz `1392400`, vorläufig `1392400` (**stille Null**), bestätigt
  `1336300` (Δ = 56100 ct — Kontrolle wirkt).
- `_kind_behinderten_pb_daten` (`kind_idnr`, `kind_behinderten_pb_antrag=True`,
  `kind_pb_nicht_selbst_genutzt=True`, `kind_grad_der_behinderung=80`): Referenz `1392400`,
  vorläufig `1392400` (**stille Null**), bestätigt `1311400` (Δ = 81000 ct — Kontrolle wirkt).
- `_schulgeld_summe` (`schulgeld=300000`, kein `kind_idnr` nötig): Referenz `1392400`,
  vorläufig `1392400` (**stille Null**), bestätigt `1359200` (Δ = 33200 ct — Kontrolle
  wirkt).
- rente-Zusatz-Instanz (`rentner_gesamt`, vollständige 2. Instanz mit allen 4 RENTNER_22-
  Feldern, `zustand="vorlaeufig"`): `grund="rente_instanz_offen"`, `zahl_cent=None`,
  `stand.ring_gesperrt="rente_instanz_offen"` — bestätigt Klasse B, fail-closed, symmetrisch
  zu vv_objekt.

Damit sind alle 4 zuvor offenen Zeilen aus der ursprünglichen Tabelle geschlossen. Keine
davon war ein neuer Bug derselben Klasse wie p23 — bei allen dreien (b)-(d) unterscheidet
sich `vorläufig` NICHT von der Referenz (stille Null bestätigt, kein Fließen), und die
Kontrolle (bestätigt) zeigt in allen drei Fällen einen echten, plausiblen Δ, also greift
der `nur_bestaetigt`-Filter tatsächlich numerisch korrekt — die einzige Lücke bleibt das
Schweigen der `/ergebnis`-Antwort (`grund`/`offen`), nicht die Zahl selbst.

## 2. Existiert bereits ein Mechanismus?

Ja, für Klasse A und B — geprüft und bestätigt:

- **Klasse A**: `api.py::ergebnis`, direkt vor der Zahl:
  ```python
  offen = [f for f in scheibe_felder
           if f not in felder or felder[f]["zustand"] != "bestaetigt"]
  ...
  grund = "engine_unavailable" if (bf is None and not offen) else "input_kegel_nicht_bestaetigt"
  ```
  greift für jedes Kegel-Pflichtfeld, dazu gehören die Basis-Instanz-Felder von vv_objekt/rente
  (sie SIND Kegel-Felder, index 1 = Basis, kein Suffix).

- **Klasse B**: `api.py::_an_gesamt_sperrgrund`, Zeile ~1930 (vv_objekt) und ~1948 (rente):
  ```python
  if inst["index"] >= 2 and (
          not pflicht <= set(inst["felder"]) or inst["zustand"] != "bestaetigt"):
      return "vv_instanz_offen"
  ```
  Deckt ausdrücklich `zustand != "bestaetigt"` für Zusatz-Instanzen ab — genau der hier
  gesuchte Mechanismus, nur beschränkt auf `multi_objekt`/`multi_rente` (aus `cfg`).

Für Klasse C existiert **kein** Mechanismus. `cfg` hat keine `multi_gwg`/`multi_kind`/
`multi_p23`-Konfiguration, die `_an_gesamt_sperrgrund` prüfen könnte — diese Gruppen laufen
NUR über die Aggregat-Summenfunktionen selbst, die (nach dem heutigen Fix) korrekt filtern,
aber nichts melden.

`/fragen` zeigt die betroffenen Felder zwar an (bestätigt für gwg/kind/p23 — alle drei
tauchten in der Fragen-Queue auf, weil `_unbeantwortet()` in `traverser.py` `vorlaeufig` als
offen behandelt: `return ev is None or ev.get("zustand") == "vorlaeufig"`). Das ist ein
SEPARATER Endpunkt, den der Nutzer aktiv abrufen muss — `/ergebnis` selbst, das die
festgesetzte Zahl liefert, enthält keinen Verweis darauf.

## 3. Bauvarianten (Klasse C: gwg, kind ×3, p23) — NICHT gebaut, zur Entscheidung

**(a) Neuer Sperrgrund `instanz_vorlaeufig_offen`** (analog `vv_instanz_offen`)
- Mechanik: `_an_gesamt_sperrgrund` (oder eine neue, gleich gebaute Prüfung) erweitert um
  `gwg`/`kind`/`p23_veraeusserung` — sobald IRGENDEINE Instanz einer dieser Gruppen
  `vorlaeufig` ist, sperrt `/ergebnis` komplett (`zahl_cent: None`).
- Für den Nutzer: **er kommt nicht weiter** — keine Zahl, bis er die Instanz bestätigt oder
  löscht. Radikalste, sicherste Variante (Parität zu vv_objekt/rente).
- Aufwand: mittel. Neue `cfg`-Einträge (`multi_gwg`/`multi_kind`/`multi_p23`), neue Guard-
  Zweige in `_an_gesamt_sperrgrund`, Tests für jede der 4 Gruppen (rot/fix/grün-Zyklus wie
  heute bei p23). Risiko: gwg/kind sind laut den Docstrings bewusst OPTIONAL/additiv gehalten
  (`_gwg_sofortabzug_summe`-Docstring: „gwg ist OPTIONAL → KEIN Kegel-/Sperr-Gate (anders als
  vv/rente)") — eine Sperre würde diese bewusste Entscheidung umkehren.

**(b) Hinweis im Ergebnis statt Sperre**
- Mechanik: `ergebnis()` sammelt zusätzlich alle vorläufigen Instanz-Felder der Klasse-C-
  Gruppen (dieselbe Filterlogik wie beim Fix, nur invertiert: `if inst["zustand"] !=
  "bestaetigt"`) und legt sie in das bereits vorhandene `"offen"`-Feld der Antwort (aktuell
  bei `grund="bestaetigt"` immer `[]` — s. `api.py::ergebnis`, letzter `return`).
- Für den Nutzer: **er sieht es, kann aber weitermachen** — `grund` bleibt `"bestaetigt"`,
  `zahl_cent` bleibt die (korrekte, weil gefilterte) Zahl, aber `offen` zeigt z.B.
  `["p23_veraeusserungspreis", ...]` mit einem erklärenden Zusatz (o.ä. wie „diese Angaben
  sind noch nicht bestätigt und NICHT in der Zahl enthalten").
- Aufwand: klein–mittel. Eine Sammelstelle in `ergebnis()`, kein neuer Sperrgrund, kein
  `_an_gesamt_sperrgrund`-Eingriff. Tests: 4 neue (einer je Gruppe), prüfen `offen`-Inhalt.

**(c) Pro Gruppe unterschiedlich**
- gwg/p23: Hinweis (b) — seltene, oft einmalige Ereignisse, Sperre wäre unverhältnismäßig
  aufdringlich für einen Betrag, der ohnehin per Konstruktion 0 bleibt, bis bestätigt.
- kind (v.a. `kinderbetreuungskosten`/`schulgeld`, laufende Kosten): ebenfalls (b), gleiche
  Begründung.
- Effektiv identisch zu (b) für alle vier Gruppen unter der aktuellen Faktenlage — es gibt
  hier keinen der vier Fälle, der eine sachliche Begründung für (a) hätte (anders als bei
  vv_objekt/rente, wo der Kegel selbst schon eine Sperre kennt).

**Empfehlung, wenn gefragt:** (b) — konsistent mit der bereits bestehenden Docstring-
Entscheidung „gwg ist OPTIONAL → KEIN Sperr-Gate", macht die stille Null sichtbar ohne die
bestehende Additiv-Philosophie der drei Klasse-C-Gruppen zu brechen. Aber: Entscheidung liegt
bei Julius, nicht bei mir — nur als Bewertungshilfe genannt.

## 3a. Fünfte Klasse-C-Stelle? `SCHEIBEN["gesamt"]` + rente-Felder — geprüft, verneint

Main's offene Frage: `gesamt` führt `multi_rente=None`, hat aber rente-artige Feldnamen im
`felder`-Superset (`versorgung_jahresrente` u.ä.); nur `rentner_gesamt` setzt
`multi_rente="rente"`. Kann in `gesamt` trotzdem eine rente-Zusatz-Instanz (`<feld>__2`)
angelegt werden, und greift dann ein Guard?

Gemessen (`API.event(fid, laie("rentner_jahresrente__2", 1200000, "vorlaeufig"))` gegen
einen `gesamt`-Fall):
```
api.ApiError: feld_id 'rentner_jahresrente__2' nicht in dieser Scheibe
```
`event()` (api.py:2183-2222) prüft `fid` gegen die SCHEIBEN-gebundene `_scheibe_bindung(store)`
— `rentner_jahresrente` (und alle 4 RENTNER_22-Kernfelder) sind in `gesamt`'s Bindung gar nicht
enthalten (bestätigt per Introspektion von `SCHEIBEN["gesamt"]["felder"]`). Der Schreib-
Endpunkt selbst blockt den Versuch, BEVOR irgendeine Ring-/Instanz-Logik läuft — es kann in
`gesamt` keine rente-Instanz je entstehen.

**Antwort: NEIN, keine fünfte Klasse-C-Stelle.** Die 12 rente-artigen Feldnamen, die main in
`gesamt`s Kegel/Felder gesehen hat, sind keine RENTNER_22-Kernfelder und tragen keinen
`instanz_gruppe="rente"`-Eintrag in dieser Scheibe — sie sind einzelne Skalarfelder, keine
Instanz-Gruppe. Die Klasse-C-Liste bleibt bei 6 Funktionen (s. Abschnitt 1).

## 4. Bestätigt: KEIN Problem bei vv_objekt und rente (Klasse A + B)

Wie in Abschnitt 1/2 gemessen: beide haben einen greifenden, gemeldeten Sperrgrund. Punkt 4
aus dem Auftrag ("wenn `/fragen` es ohnehin zeigt") ist für Klasse A/B nicht relevant, weil
dort `/ergebnis` SELBST schon sperrt und meldet (`grund`) — kein Rückgriff auf `/fragen`
nötig. Für Klasse C ist `/fragen` zwar korrekt (zeigt die Felder), aber `/ergebnis` selbst
schweigt — das ist genau die Lücke aus Abschnitt 1.

## 5. GATE (Kontrolle, keine Code-Änderung in diesem Auftrag)

Befehl: `timeout 500 python3 -m pytest -q`
```
1655 passed, 4 skipped, 1 warning in 186.75s (0:03:06)
```
Exit-Code: 0. Identisch zur Referenz (1655/4) — keine Code-Änderung in diesem Auftrag, also
keine Verschiebung erwartet und keine gemessen.

**Nachtrag-Gate (nach den Messungen (a)-(d) + 5.-Stelle-Frage, immer noch KEIN Code
geändert):**
```
1655 passed, 4 skipped, 1 warning in 226.13s (0:03:46)
```
Exit-Code: 0. Identisch — bestätigt, dass keine der Messungen (reine `/tmp`-Skripte gegen
`API.FAELLE`-Tempdir) Produktcode berührt hat.

## Nicht gemessen

Die 4 Punkte aus der ursprünglichen Liste (`_kind_kv_pv_summe`, `_kind_behinderten_pb_daten`,
`_schulgeld_summe`, rente-Zusatz-Instanz) sind mit diesem Nachtrag geschlossen — s. Abschnitt 1
und 3a. Verbleibend:

1. **Kombinierte Fälle** (z.B. zwei Klasse-C-Gruppen gleichzeitig vorläufig) — nicht geprüft,
   Auftrag verlangte Gruppe-für-Gruppe.
2. **Wie sich (a)/(b)/(c) auf bestehende Tests auswirken würden** — reine Messaufgabe, keine
   Implementierung versucht, also auch keine Kompatibilitätsprüfung mit den 9 bestehenden
   nur_bestaetigt-Tests.

## Status

Kein Code geändert. `git status --short` vor und nach der Messung identisch leer (nur Reports
hinzugefügt). Nichts committed.

---

## Nachtrag main — unabhängige Verifikation (2026-08-07)

Nicht alles nachgeprüft, sondern die Stellen, an denen ein falsches „kein Bug" teuer wäre.

**Bestätigt (selbst gemessen, nicht übernommen):**

1. **Klasse B, vv_objekt index 2** — die einzige Klasse-B-Zeile, die der Bericht als gemessen
   führt. Eigener Lauf: `grund = "vv_instanz_offen"`, `zahl_cent = None`. Fail-closed wie
   beschrieben, kein Bug.
2. **Klasse C, gwg** — vorläufige gwg-Instanz: `grund = "bestaetigt"`, `zahl_cent = 1392400`,
   `offen = []`. Identisch zur Referenz ohne das Feld. Stille Null bestätigt.
3. **`api.py:2277`** — der Erfolgspfad von `ergebnis()` setzt `"offen": []` hart:
   ```python
   return 200, {..., "grund": "bestaetigt", "offen": [], "trace": trace, ...}
   ```
   Der Beleg des Berichts trifft zu.

**Korrektur zugunsten von Variante (b):** Der Bericht schätzt (b) auf „klein–mittel". Sie ist
billiger als das. Das Feld `offen` steht bereits in JEDER `/ergebnis`-Antwort und ist in
`produkt/haut/api_schema/ergebnis.json` als `required` geführt — die Antwortstruktur ändert
sich also nicht, kein Schema-Eingriff, keine Client-Anpassung. Es bleibt eine Sammelstelle in
`ergebnis()` plus die vier Tests.

**Fehlschlag, der KEIN Befund ist:** main hat versucht, die rente-Zusatz-Instanz selbst
nachzumessen, und die `rentner_gesamt`-Kegelfelder dafür mit geratenen Defaults gefüllt. Der
Lauf starb an
```
ValueError: renten_art 0 nicht ring-fähig (MVP: aa+bb; sonstige = GAP)
```
(`golden/runner.py:899`). Das ist ein Artefakt des Messskripts — `renten_art=0` war ein
geratener Wert, kein zulässiger Eingabewert —, kein Produktbefund. Festgehalten, damit die
Zeile nicht später als Bug wiederauftaucht. Die Messung selbst steht weiterhin aus (s.
„Nicht gemessen" Punkt 2) und läuft bei dev-a mit echten Werten aus den bestehenden
`rentner_gesamt`-Testfällen.

**Zur Meldung über 28 fremd-modifizierte Testdateien** (`import audit` + `AUDIT_DIR`-
Monkeypatch): das ist dev-b's laufende Arbeit am BACKLOG-Punkt `audit-jsonl-wucherung`, kein
Fremdkörper. Richtig gemeldet und richtig nicht angefasst.

**Offene Frage, die der Bericht nicht stellt:** `SCHEIBEN["gesamt"]` hat `multi_rente = None`,
führt aber 12 rente-artige Felder im Kegel; nur `rentner_gesamt` setzt `multi_rente = "rente"`.
Ob in der Scheibe `gesamt` überhaupt eine rente-Zusatz-Instanz anlegbar ist und ob dann ein
Guard greift, ist ungemessen. Falls nein, wäre das eine fünfte Klasse-C-Stelle. Zur Messung an
dev-a gegeben, nicht geraten.
