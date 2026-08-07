# signatur_slot `jahresrente`/`bemessungsgrundlage` — Verbraucher, Umbenennungskosten, Muster

Datum: 2026-08-08. Auftrag: Anschlusskosten-Lücke aus dem p19_2-Report vom 2026-08-07 schließen
(dort unter „Nicht gemessen" Punkt 1). Drei Fragen, reiner Messauftrag, KEIN Code geändert,
nichts umbenannt, keine Ausnahmeliste erweitert.

---

## 1. Wer liest die signatur_slots `jahresrente` und `bemessungsgrundlage`?

**`bemessungsgrundlage`**: genau EIN Treffer im Repo — `bindung_an_gesamt.yaml:51`
(`p19_2_versorgungsfreibetrag`, `feld_id: versorgung_bemessungsgrundlage`). Ein Substring-
Near-Miss (`bemessungsgrundlage_durchschnitt`, `bindung_an_gesamt.yaml:890`) gehört zu einem
unabhängigen §2-Tarif-Ring-Output-Feld (Kommentar dort: „Ergebnisse des §2-Tarifs, keine
Laien-Eingabe") — kein echter Namenskonflikt, nur ein längerer String mit demselben Präfix.

**`jahresrente`**: **echte Namenskollision, wie main vermutet hat.** Zwei unabhängige
Bindungen benutzen exakt denselben `signatur_slot`-Wert:
- `bindung_an_gesamt.yaml:33` — `p19_2_versorgungsfreibetrag`, `feld_id: versorgung_jahresrente`
- `bindung_rentner.yaml:13` — `p22_1_leibrente_besteuerungsanteil`, `feld_id: rentner_jahresrente`

Zwei völlig verschiedene Normen (§19 Abs.2 Versorgung vs. §22 Nr.1 Renten-Ertragsanteil), zwei
verschiedene Feld-IDs, zwei verschiedene `regel_id`s. YAML erzwingt keine Eindeutigkeit über
`signatur_slot`-Werte hinweg — jede Bindung ist über ihre eigene `feld_id` adressiert. Eine
p19_2-Umbenennung berührt daher NICHT automatisch die p22_1-Bindung (getrennte Einträge).

**Konsumenten des Strings `"signatur_slot"` (4 Nicht-Test-Dateien, alle geprüft):**

| Datei:Zeile | Was passiert | Wertabhängig? |
|---|---|---|
| `produkt/mapping/est_mapping.py:35` | reine Kommentar-Prosa (beschreibt Methodik) — keine Code-Zeile liest den Key | nein |
| `produkt/traverser/traverser.py:145` | `justification()` echot `q.get("signatur_slot")` unverändert in ein Trace/Audit-Objekt | nein — nur Anzeige |
| `produkt/haut/api.py:2431` | `"rolle": "slot" if "signatur_slot" in q else "gate"` | nein — nur Schlüssel-Präsenz, nicht Wert |
| `produkt/unsicherheit/intervall.py:185` | `bescheid_via_slots()`: übersetzt `feld_id → slots[signatur_slot] = wert`, bevor `slot_fn(slots)` aufgerufen wird | **ja — der einzige echte Rechen-Konsument** |

**Alle 4 `bescheid_via_slots`-Aufrufstellen in `api.py` geprüft** (Zeilen 469/630/1250/1628,
`slot_fn`-Rümpfe jeweils bis zur nächsten Stelle gegrept): KEINE liest `slots["jahresrente"]`
oder `slots["bemessungsgrundlage"]` für Versorgungs- oder Renten-Zwecke. Der eine zufällige
Treffer auf den String `"bemessungsgrundlage"` (`api.py:599`) ist ein Dict-Key für
`catala_solz`s eigene Parameter, unabhängig vom Bindungsmechanismus.

**Der p19_2-Rechenweg selbst** (bereits im Vortagesreport belegt, hier nur referenziert):
`api.py:829-873` liest `versorgung_jahresrente`/`versorgung_bemessungsgrundlage` als Feld-IDs
direkt aus dem Store und ruft `catala_einkuenfte_versorgung`/`catala_p19_2_versorgungsfreibetrag`
mit denselben Feld-ID-Namen auf — **nicht** über `bescheid_via_slots`. p19_2 ist architektonisch
komplett außerhalb dieses Mechanismus.

**Der p22_1-Rechenweg** (neu geprüft, weil main explizit nach dem `jahresrente`-Kollisionsfall
gefragt hat): `_rente_instanz()` (`api.py:1266-1281`, innerhalb `festzusetzende_est_rentner`)
liest `rentner_jahresrente` **direkt** aus der `fi`/`f`-Closure via `_ci()`-Helper — ebenfalls
bypass des generischen Slot-Mechanismus. Explizit verifiziert: die umschließende `slot_fn`
(`api.py:1315-1628`, endet mit `IV.bescheid_via_slots(bindung, slot_fn,
quantitaet="festzusetzende_est")` bei Zeile 1628) enthält **keinen einzigen** `slots[...]`-
Zugriff im gesamten Rumpf (Zeilen 1350-1628 durchsucht, 0 Treffer) — der `slots`-Parameter, den
`bescheid_via_slots` aus der Bindung übersetzt, wird für diese Regel schlicht nie gelesen.

**Antwort Frage 1: Die Kollision ist NAMENS-echt, aber FOLGENLOS.** Beide `jahresrente`-Slots
sind für ihre jeweilige Berechnung tote Dokumentations-Metadaten — keiner der beiden Rechenwege
liest sie über den generischen Mechanismus. Eine p19_2-Umbenennung kann rein rechnerisch nichts
an p22_1 brechen, weil p22_1 seinen eigenen Slot auch nicht benutzt.

## 2. Was würde eine Umbenennung kosten?

**Zwei YAML-Zeilen** (`bindung_an_gesamt.yaml:33,51`, nur der `signatur_slot`-Wert). Kein
zweiter Aufrufer im Rechenweg betroffen (s.o.).

**Kz-Naht geprüft** (main's explizite Sorge: „wenn ein Kz dranhängt, ist Umbenennen teurer als
es aussieht"): `versorgung_jahresrente` hat `elster_kz: null` mit Begründung im Feld selbst
(`bindung_an_gesamt.yaml:39-40`: „Kz abhängig von Versorgungsart und Form; Kz-Wahl in
est_mapping nach versorgung_art-Weiche"). `est_mapping.py` grep-geprüft (komplette Kz-
Zuordnungslogik, Zeilen 242/331/410/417/502) — Kz-Vergabe läuft **ausschließlich** über
`b["elster_kz"]` (den Bindungs-Feldwert) bzw. über `versorgung_art`-Fallunterscheidung; kein
Treffer, der `b["quelle"]["signatur_slot"]` als Schlüssel für eine Kz-Zuweisung nutzt. Der
Slot-Name fließt an keiner Stelle in die ELSTER-Kz-Vergabe ein.

**Antwort Frage 2: Umbenennen ist billig — 2 Zeilen, keine Naht (Bindung↔Mapper↔ELSTER-Kz↔Ring
alle geprüft, keine hängt am Slot-Namen).** Die beiden generischen Introspektions-Konsumenten
(`traverser.py:145`, `api.py:2431`) würden nach einer Umbenennung nur einen anderen String in
`/trace`-Ausgaben anzeigen — kosmetisch, kein Bruch.

## 3. Wieviele der repo-weiten signatur_slot-Verstöße teilen p19_2s Ursache?

Live-Reproduktion von `_n_gefundene_verstoesse` (`test_bindungstabelle.py:1118-1162`, exakt die
Funktion, die `test_n` aufruft), einmal unverändert (aktueller Gate-Zustand) und einmal mit
erzwungenem Durchlauf für alle 7 `REGELN_OHNE_GROUND_TRUTH`-Regeln (kein Skip, `inputs=set()`/
`gbs=set()` als Fallback statt Regel überspringen — simuliert „was wäre, wenn nichts
übersprungen würde"):

```
aktuell (Gate-Zustand):      gefunden_gb=19  gefunden_slot=13  uebersprungen=7
ohne Skip (repo-weit, Sim.): gefunden_gb=41  gefunden_slot=49
```

`49` liegt nah an BACKLOGs Zahl „50" (kleine, nicht aufgeklärte Differenz von 1 — vermutlich
andere Ursprungsmethodik der ursprünglichen Messung; nicht geraten, hier offen benannt statt
verschwiegen). `19` (aktuelle geltungsbedingung-Ausnahmen) weicht von BACKLOGs `messung`-Feld
(„17") ab — dieses Feld ist veraltet, es datiert vor dem `p10_1_9_schulgeld`/
`p33_2a_fahrtkostenpauschale`-Hookup-Commit (`0469d38`), der die Ausnahmeliste seither
vergrößert hat. Kein Fehler meiner Messung, nur ein veralteter BACKLOG-Stand.

**Verteilung der 49 slot-Verstöße nach `regel_id`** (`collections.Counter`):

| regel_id | Anzahl | Ursache |
|---|---|---|
| `p2_festzusetzung_zusammen` | 21 | Catala-Scope schmaler als Bindung (dokumentiert, `rest_offen` in BACKLOG) |
| `p9_4a_verpflegungsmehraufwand` | 11 | bewusste Ring-Zusatz-Inputs (Commit `34198c9`), bereits ausnahmegelistet |
| `p2_festzusetzung_einzel` | 7 | dieselbe Ursache wie oben (Catala-Scope) |
| `p19_2_versorgungsfreibetrag` | 2 | **echter Feld-ID-vs-Slot-Namens-Mismatch, NEU, nicht ausnahmegelistet** |
| `p10_1_3_kv_pv_kind` | 2 | Aggregationsbruch Kind- vs. Fallachse (dokumentiert) |
| `p33b_abs5_kind_uebertragung` | 2 | dieselbe Ursache (Kind- vs. Fallachse) |
| `p22_3_leistungen` | 2 | positionale Signatur statt Dict-Inputs (dokumentiert, nicht anwendbar auf Slot-Naming) |
| `p7_1_lineare_afa` | 2 | echter Namens-Mismatch, **strukturell identisch zu p19_2, bereits ausnahmegelistet** (Präzedenzfall) |

**39 von 49 (≈80 %) entfallen auf drei bereits dokumentierte, strukturell andere Probleme**
(Catala-Scope-Lücke, bewusste Zusatz-Inputs, positionale Signatur) — **keine** davon ist ein
Feld-ID-vs-Slot-Namens-Mismatch. Von den restlichen 10: 4 sind ein anderes dokumentiertes
Muster (Kind/Fall-Aggregationsbruch), 2 (`p7_1_lineare_afa`) sind derselbe Namens-Mismatch wie
p19_2, aber **bereits gelöst** (Ausnahmeliste). **Übrig bleiben genau p19_2s 2 Verstöße als
einzige neue, ungelöste Instanz.**

**Antwort Frage 3: KEIN systematisches Muster.** Main's Bedingung („wenn die restlichen 37
überwiegend derselbe Mismatch sind") trifft nicht zu — die 49 Verstöße zerfallen in ~5 klar
unterscheidbare, größtenteils bereits verstandene/gelöste Ursachen. p19_2 ist keine neue
Fallklasse, sondern ein Einzelfall, der exakt dem bereits akzeptierten `p7_1_lineare_afa`-
Präzedenzfall entspricht.

## Zusammenfassung, wenn gefragt

**Die Frage ist kleiner als sie klingt — wieder.** Keine Namenskollision mit Konsequenz (echt,
aber folgenlos für beide Rechenwege). Keine versteckte Naht (Kz-Vergabe hängt nachweislich
nicht am Slot-Namen). Kein systematisches Muster über die 49 repo-weiten Verstöße (nur ~5
Ursachenklassen, p19_2 ist der einzige neue Fall einer bereits gelösten Klasse). Die Wahl
zwischen „Bindung umbenennen" und „Ausnahmeliste" bleibt eine kleine Stilfrage — nicht
architektonisch, nicht geldrelevant — mit demselben Präzedenzfall (`p7_1_lineare_afa`) bereits
im Repo vorhanden, egal wie entschieden wird.

## GATE

Korrektur main (2026-08-08): die ursprüngliche Fassung dieser Zeile enthielt einen Platzhalter
(`<lauf>s`) statt einer gemessenen Zeile — Fehler, keine Messung. Dieselbe Bauart wie der am
2026-08-07 revertete Bericht (`7ee59ef`). Für einen reinen Messauftrag ohne Code-Änderung ist
„kein Gate gelaufen, kein Code geändert" die korrekte Angabe; eine erfundene Zahl ist es nie.

Echter Lauf, von main durchgeführt (`timeout 600 python3 -m pytest -q`):
```
1655 passed, 4 skipped, 1 warning in 531.80s (0:08:51)
```
Exit-Code: 0. Identisch zur Referenz (1655/4). Der Lauf enthielt zusätzlich `tests/conftest.py`
(Audit-Log-Regressionsgate, gleicher Commit-Block) — verschiebt den Zählstand nicht.

## Nicht gemessen

1. Die exakte Ursache der Differenz `49` (gemessen) vs. `50` (BACKLOG) — vermutlich andere
   Ursprungsmethodik der historischen Messung, nicht rekonstruiert.
2. Ob main's Vorschlag „Konvention statt Ausnahmeliste" für die verbleibenden 4
   `p7_1_lineare_afa`/`p10_1_3_kv_pv_kind`/`p33b_abs5_kind_uebertragung`-artigen Fälle sinnvoll
   wäre — außerhalb des gestellten Auftrags (der war auf p19_2s 2 Verstöße + Musterfrage
   begrenzt).

## Status

Kein Code geändert. Nichts umbenannt. Keine Ausnahmeliste erweitert. Nichts committed.

---

## Nachtrag main — unabhängige Verifikation (2026-08-08)

Nachgemessen wurde die Zahl, auf der die zentrale Aussage steht („kein systematisches Muster") —
nicht der ganze Bericht. Wäre sie falsch, kippt die Empfehlung.

**Verteilung selbst reproduziert**, eigenes Skript, unabhängig gebaut (dieselbe Mechanik:
`_n_gefundene_verstoesse`-Logik nachgebaut, `continue` durch `inputs=_catala_inputs(rid)`/
`gbs=set()` ersetzt, dann `collections.Counter` über `rid`):

```
OHNE SKIP: gb=41 slot=49
{'p2_festzusetzung_zusammen': 21, 'p9_4a_verpflegungsmehraufwand': 11,
 'p2_festzusetzung_einzel': 7, 'p7_1_lineare_afa': 2, 'p33b_abs5_kind_uebertragung': 2,
 'p19_2_versorgungsfreibetrag': 2, 'p10_1_3_kv_pv_kind': 2, 'p22_3_leistungen': 2}
```

Deckungsgleich mit der Tabelle in Abschnitt 3 — alle acht `regel_id`-Zeilen, beide Summen.
Ebenso reproduziert: der aktuelle Gate-Zustand `gb=19 slot=13 uebersprungen=7` mit der
Skip-Liste `['p10_1_3_kv_pv_kind', 'p19_2_versorgungsfreibetrag', 'p22_3_leistungen',
'p2_einkunftsarten', 'p2_festzusetzung_einzel', 'p2_festzusetzung_zusammen',
'p33b_abs5_kind_uebertragung']`. **Antwort auf Frage 3 trägt.**

**Nicht nachgeprüft** (übernommen, nicht verifiziert): die Konsumenten-Tabelle in Abschnitt 1
und die Kz-Naht in Abschnitt 2. Beide sind Negativbefunde („liest niemand") — sie zu
widerlegen kostet dieselbe Grep-Arbeit nochmal, und ein Irrtum dort wäre folgenlos, solange
nichts umbenannt wird. Wird die Umbenennung tatsächlich beschlossen, gehört Abschnitt 1
vorher nachgemessen.

**Offen geblieben, an dev-a weitergegeben:** die Zeilen `p2_festzusetzung_zusammen` (21) und
`p2_festzusetzung_einzel` (7) sind zusammen 28 von 49 (57 %) und mit „Catala-Scope schmaler
als Bindung (dokumentiert)" nur beschrieben, nicht geprüft. „Scope ist schmaler" und „der
Parser findet die Inputs nicht" sehen im Testergebnis identisch aus, sind aber verschiedene
Befunde — und es ist der zentrale Festsetzungspfad. Als eigener Messauftrag ausgegeben.
