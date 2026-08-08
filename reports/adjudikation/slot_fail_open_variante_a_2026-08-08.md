# Variante (a) entscheidungsreif — Miss-Messung der 6 gelesenen Slot-Namen

Datum: 2026-08-08. Auftrag (main): BACKLOG `slot-fail-open-13568-eur`, Variante (a) — Frage
„welche Aufrufer dürfen einen Slot legitim weglassen?" beantworten, damit `slots.get(name,
default)` → fail-closed (`slots[name]`) empfehlbar wird oder nicht. Reiner Messauftrag: KEIN
Code im Live-Baum geändert. Alle Instrumentierung + alle Testläufe liefen ausschließlich auf
einer isolierten `/tmp`-Kopie (`git archive HEAD | tar -x`, `oracle/gettsim/_catala` per Symlink
zurück auf den Live-Checkout), HEAD `e1be52d`.

---

## Methodik

`produkt/unsicherheit/intervall.py::bescheid_via_slots` auf der `/tmp`-Kopie so erweitert, dass
der `slots`-dict, den `_fn()` baut, VOR der Übergabe an `slot_fn(slots)` in eine
Mess-Dict-Subklasse (`_MessSlots`) gewickelt wird. Diese überschreibt `.get()` und `__contains__`
und protokolliert JEDEN Fall, in dem der angefragte Key fehlt (`(quantitaet, slot, modus)` nach
`/tmp/slot_probe_devb_copy_misslog.jsonl`) — deckt sowohl Klasse 1 (`slots.get(name, default)`)
als auch Klasse 3 (`{k: slots[k] for k in (...) if k in slots}`, nutzt `__contains__`) ab.
Nur aktiv auf der Kopie (`_MESS_AKTIV=True`), Produktionscode im Live-Baum unverändert.

Positivkontrolle vor dem Suite-Lauf: ein synthetischer Aufruf mit einem garantiert fehlenden Key
erzeugt zuverlässig einen Log-Eintrag — die Instrumentierung feuert nachweislich, ein leeres Log
ist also ein echtes Null-Ergebnis und kein stiller Blindgang der Messung selbst.

Lauf: volle pytest-Suite (`1666 passed, 4 skipped` — die 4 Skips sind GETTSIM-venv/Recorded-
Fixture/SQL-Backend-bedingt, nicht slot-relevant) inkl. `tests/test_paket_b_e2e_http.py` (195
Tests, echter HTTP-Request/Response-Zyklus über `server.make_server`, alle vier Scheiben `ep`,
`an_gesamt`, `gesamt`, `rentner_gesamt` real angelegt und beschieden). Ein Test
(`test_k_alle_python_dateien_parsen`) schlägt NUR wegen `git ls-files` fehl, weil die
`git archive`-Kopie kein `.git`-Verzeichnis hat — Artefakt der Kopiermethode, kein Produktbefund,
per `--deselect` bestätigt irrelevant für diese Messung.

## Ergebnis: Miss-Log ist LEER — kein einziger der 6 Slots fiel je auf seinen Default zurück

| Slot | Lesestelle(n) | Miss kommt real vor? | Warum nicht |
|---|---|---|---|
| `arbeitstage` | api.py:469 (ep), :592/:611-628 (an_gesamt/festzusetzende_est), :743 (gesamt) | **nein** | `ep_arbeitstage` steht in `SCHEIBEN["ep"]["felder"]` UND in `SCHEIBEN["an_gesamt"/"gesamt"]["kegel"]` — `_feste_zahl`/`ergebnis()` liefert nie eine Zahl, bevor dieses Feld `bestaetigt` ist (Meet-Gate) |
| `entfernung_km_roh` | api.py:469, :592, :743 | **nein** | dieselbe Kegel-Pflicht (`ep_entfernung_km`) wie oben |
| `oepnv_kosten_jahr` | api.py:469, :592 (via `_oepnv_eur`), :743 | **nein** | dieselbe Kegel-Pflicht (`ep_oepnv_kosten`) wie oben |
| `eigenes_oder_ueberlassenes_kfz` | api.py:469, :592, :743 | **nein** | dieselbe Kegel-Pflicht (`ep_eigenes_kfz`) wie oben |
| `bruttoarbeitslohn` | api.py:574, :592, :846 | **nein** | Pflichtfeld in `SCHEIBEN["an_gesamt"/"gesamt"]["kegel"]` (nicht in `ep`, aber dort auch nicht gelesen — `ep` hat `kegel=None` und liest `bruttoarbeitslohn` gar nicht) |
| `veranlagung` | api.py:590 (+ mittelbar überall, wo `zusammen`-Zweig verzweigt) | **nein** | Pflichtfeld in ALLEN VIER `kegel`/`felder`-Tupeln, inkl. `rentner_gesamt` |

## Warum das kein Zufall ist — ZWEI getrennte Mechanismen, nicht einer

**Fester Pfad (`/ergebnis`) — Kegel-Meet-Gate.** `ergebnis()` (api.py:2237) ruft `_feste_zahl()`
(api.py:1635), und die liefert NUR eine Zahl, wenn `ST.meet_zustand([felder[f]["zustand"] for f
in scheibe_felder]) == "bestaetigt"` — also wenn JEDES Feld in `scheibe_felder` (=
`cfg["kegel"]` bzw. `cfg["felder"]`) bestätigt vorliegt. Alle 6 Slot-Namen sind über ihre
`feld_id` (`ep_arbeitstage`, `ep_entfernung_km`, `ep_oepnv_kosten`, `ep_eigenes_kfz`,
`bruttoarbeitslohn`, `veranlagung`) Mitglied genau dieses Kegels in JEDER Scheibe, die sie
tatsächlich liest (verifiziert per `api_constants.SCHEIBEN[...]` auf der Kopie). Ein Slot kann
also gar nicht fehlen, WENN der Ring überhaupt eine Zahl ausgibt — er fehlt nur, wenn der Ring
vorher schon (aus einem anderen Grund) `None`/`offen` zurückgibt, und dann liest die betroffene
`slot_fn` gar nicht erst (der Aufruf passiert NACH dem Gate).

Das ist derselbe Mechanismus, der main's ursprünglichen 13.568-€-Fund erst ermöglicht hat: nicht
ein FEHLENDER Slot im Normalbetrieb, sondern ein UMBENANNTER `signatur_slot` in der
Bindungstabelle (`bruttoarbeitslohn` → `bruttoarbeitslohn_x`) — der Kegel-Check auf Feld-Ebene
(`felder[f]["zustand"]`) sieht die Umbenennung nicht, weil er auf `feld_id`, nicht auf
`signatur_slot` prüft. Der Slot fehlt dann nicht, WEIL das Feld nicht erhoben wurde, sondern WEIL
die Bindung ihn unter einem falschen Namen einträgt. Für diesen Fehlerkanal ist `test_slot_fn_
reader_existiert.py` (Variante b) das richtige Gate — das ist eine statische Bindungs-
/Signatur-Prüfung, kein Laufzeit-Miss.

**Estimate-Pfad (`/stand`, `/fragen`) — askable-Flag, KEIN Kegel-Gate.** `_bescheid_fn` wird
nicht nur von `_feste_zahl` (api.py:1650) aufgerufen, sondern auch von `_gesamt_beitrag`
(api.py:2102/2108) und `stand` (api.py:2164/2171) — den Estimate-Pfaden, die BEWUSST mit
`nur_bestaetigt=False` laufen, damit vorläufige Werte im `/stand`-Intervall Wirkung zeigen
dürfen. Diese Pfade laufen NICHT über das Kegel-Meet-Gate, sondern über `IV.intervall()`, das
sein `base`-Dict aus `{fid: b for fid, b in bindung.items() if b.get("askable")}` baut
(intervall.py:89) — NUR askable Felder. Der Schutz für den Estimate-Pfad ist also nicht der
Kegel, sondern das `askable`-Flag in der YAML — experimentell bestätigt (Abschnitt unten). Heute
sind alle 6 Bindungseinträge `askable: true` (geprüft), deshalb bleibt auch dieser Pfad ohne
Miss — aber aus einem ANDEREN Grund als der feste Pfad.

### Nachgemessen: `askable: false` am Estimate-Pfad — drei Lesemodi für vier zusammengehörige Slots, nicht einer

`ep_arbeitstage` auf der `/tmp`-Kopie auf `askable: false` gesetzt, realer EP-Fall über
HTTP-Server, `/stand` aufgerufen. Miss-Log feuert zuverlässig (`{"quantitaet":
"festzusetzende_est", "slot": "arbeitstage", "modus": "in-check-false"}`, 2×) — main's
Mechanismus (askable filtert `intervall()`s base) ist bestätigt. Die ERWARTETE Konsequenz — eine
stille Verschiebung des Intervalls — trat aber NICHT ein. Stattdessen: `/stand` wirft HTTP 500
mit `KeyError: 'arbeitstage'`, Traceback:

```
api.py:554 slot_fn -> runner.catala_werbungskosten_n(wk_input)
golden/runner.py:193 wk += catala_entfernungspauschale(s)
golden/runner.py:110 arbeitstage_in=int(s["arbeitstage"])   <- bare Subscript, ungated
```

Grund: `catala_entfernungspauschale` (`golden/runner.py:106-118`) liest die vier EP-Slots, die
api.py als EINE zusammengehörige Einheit übergibt, in DREI verschiedenen Modi:

| Zeile | Slot | Lesemodus | Ausgang bei Fehlen |
|---|---|---|---|
| `runner.py:110` | `entfernung_km_roh` | bare `s["..."]` | **gegated** durch `if "entfernung_km_roh" in s` (api.py:485/743) — Feld fehlt komplett aus `wk_input`, EP-Komponente entfällt lautlos (over-tax) |
| `runner.py:111` | `arbeitstage` | bare `s["..."]` | **ungegated** — KeyError, HTTP 500 |
| `runner.py:112` | `eigenes_oder_ueberlassenes_kfz` | `s.get(..., False)` | **stiller Default** — falsche EP-Berechnung ohne Fehler |
| `runner.py:113` | `oepnv_kosten_jahr` | `s.get(..., 0)` | **stiller Default** — falsche EP-Berechnung ohne Fehler |

Vier Werte desselben Sachverhalts (alle vier EP-Eingaben derselben Berechnung), drei Ausgänge:
einer gated, einer crasht, zwei schweigen. Das äußere Klasse-3-Gate in api.py (`if
"entfernung_km_roh" in s:`) liest sich wie „ist eine EP im Spiel?", trägt aber faktisch die
Presence-Prüfung für alle vier — nirgends deklariert, nirgends getestet, dass die vier Slots
gemeinsam vorhanden sein müssen. Diese Kopplung existiert nur implizit, weil im Normalbetrieb
immer alle vier `askable: true` sind und deshalb immer gemeinsam ankommen.

**Rangfolge der drei Ausgänge, von harmlos zu gefährlich: der Crash ist der BESTE der drei, nicht
das Problem.** Ein `KeyError` ist laut, reproduzierbar, bricht sichtbar mit HTTP 500 — kostet
niemanden Geld, weil `/ergebnis` (der feste Pfad, der tatsächlich eine Steuer festsetzt) davon
unberührt bleibt (blieb bei 662900 unverändert, over-tax-safe im Ausgang). Die zwei stillen
Defaults (`eigenes_oder_ueberlassenes_kfz`, `oepnv_kosten_jahr`) sind das eigentliche Risiko:
sie kürzen die Werbungskosten lautlos, der Nutzer zahlt zu viel, und nichts im System merkt es
an. Wer diesen Abschnitt liest und den Crash für den Befund hält, hat die Prioritäten verdreht —
die zwei Stillen sind der Befund, der Crash ist nur der, der zufällig am lautesten ist.

Ergebnis: ein Ausfall/eine Verzerrung des Estimate-Pfads, sobald GENAU EINER der vier EP-Slots
`askable: false` wird, während die anderen askable bleiben — eine asymmetrische Teil-Umstellung,
kein Alles-oder-Nichts-Fall. Heute unerreichbar, weil alle vier `askable: true` sind — aber ein
realistischer Auslöser: ein Feld, das automatisch aus eDaten übernommen wird (Richtung, in die
`edaten/import:elster` das Produkt bereits bewegt), würde genau diese Konstellation erzeugen,
kein exotisches Szenario.

Für `bruttoarbeitslohn`/`veranlagung` nicht separat nachgemessen (kein Klasse-3-
Dict-Comprehension-Gate an dieser Stelle, nur direkte `slots.get(...)`-Aufrufe in `api.py`) —
dort wäre bei `askable: false` eher der stille Default (dritte Zeile der Tabelle oben, gleicher
Mechanismus wie `oepnv_kosten_jahr`/`eigenes_oder_ueberlassenes_kfz`) zu erwarten, aber ungemessen.

## Antwort auf die blockierende Frage

**Alle 6 Slots dürfen NICHT legitim weggelassen werden — für jeden ist der Default fachlich
falsch, wenn er je griffe.** Aber: keiner von ihnen greift je im Normalbetrieb, weil das
Kegel-Gate (`_feste_zahl`) den Ring vorher sperrt. Der Default ist also heute eine **tote
Falle**: erreichbar nur über einen Programmierfehler (Bindungstabelle mit falschem/fehlendem
`signatur_slot`), nie über einen legitimen fachlichen Weglass-Fall.

| Slot | Miss real (ja/nein) | Default fachlich korrekt (ja/nein/unklar) | Folge bei Umstellung auf `slots[name]` |
|---|---|---|---|
| `arbeitstage` | nein | nein (0 Tage ist nie ein korrekter Pendler-Fall) | keine Verhaltensänderung im Normalbetrieb; toter Bindungsfehler wird zu KeyError statt stiller 0 |
| `entfernung_km_roh` | nein | nein | wie oben |
| `oepnv_kosten_jahr` | nein | nein | wie oben |
| `eigenes_oder_ueberlassenes_kfz` | nein | nein (False ist ein aktiver fachlicher Wert, aber der Default-Pfad wird nie fachlich, sondern nur über einen Bindungsfehler erreicht) | wie oben |
| `bruttoarbeitslohn` | nein | nein (0 € Lohn ist der Klasse-Bug selbst) | wie oben |
| `veranlagung` | nein | unklar in der Richtung (Splitting kann kippen), aber der Miss-Pfad selbst ist nie fachlich erreichbar | wie oben |

„Miss real" = heute, mit allen 6 Bindungseinträgen auf `askable: true`. Das ist keine
Invariante, sondern ein YAML-Flag — sobald eines der 6 Felder (plausibel z. B. bei
automatischer eDaten-Übernahme) auf `askable: false` gesetzt wird, ändert sich der Ausgang: für
den festen Pfad (`/ergebnis`) nichts (Kegel-Gate bleibt unabhängig von `askable` scharf), für den
Estimate-Pfad (`/stand`) einer von drei Ausgängen je nach Slot — s. Abschnitt oben (drei
Lesemodi bei den vier EP-Slots, zwei davon still und gefährlicher als der gemessene Crash).
`bruttoarbeitslohn`/`veranlagung` nicht separat nachgemessen (kein
Klasse-3-Dict-Comprehension-Gate an dieser Stelle, nur direkte `slots.get(...)`-Aufrufe in
`api.py`) — dort wäre bei `askable: false` der stille Default zu erwarten (gleicher Mechanismus
wie die zwei stillen EP-Fälle), aber ungemessen.

## Empfehlung (main/Julius entscheiden)

Alle 6 Slots können auf `slots[name]` (fail-closed) umgestellt werden, OHNE dass sich das
Verhalten im Normalbetrieb ändert — kein Test bricht, kein realer Kegel-Pfad verändert sich,
weil kein einziger Miss je auftritt. Die einzige Verhaltensänderung: ein zukünftiger
Bindungsfehler (umbenannter/fehlender `signatur_slot`) wirft `KeyError` statt still den Default
zu liefern und die Steuer auf 0/falsch fallen zu lassen. Das ist exakt die Verhaltensänderung,
die Variante (a) beheben sollte — kein Zielkonflikt mit Variante (b) (Gate bleibt zusätzlich
sinnvoll, weil es den Fehler VOR dem Release fängt statt erst zur Laufzeit).

Kein individueller Fall mit realem Miss übrig — main's befürchtetes „Einzelfallentscheidung
nötig" Szenario tritt für keinen der 6 Slots ein.

## Gate

Kein Gate gelaufen als Produktveränderung — kein Code im Live-Baum geändert, Empfehlung ist
Messung + Vorschlag, keine Umsetzung. Falls main/Julius die Umstellung auf `slots[name]`
freigeben: dann gehört dazu ein grüner Volllauf (die 1666 grün + 195 e2e-HTTP oben sind bereits
der Beweis, dass KEIN bestehender Test durch die Umstellung rot würde, aber ein echter Commit
braucht den Lauf auf dem Live-Baum, nicht auf der Kopie).

## Status

Kein Code im Live-Repo geändert. Instrumentierung + alle Testläufe ausschließlich auf
`/tmp/slot_probe_devb_copy` (git-archive-Kopie von HEAD `e1be52d`). `tests/test_bindungstabelle.py`
(dev-a's Datei) nicht angefasst — der separat aufgetretene `[orch-gate]`-Pre-Commit-Fehler auf
dieser Datei stammt aus dev-a's/main's paralleler, laufender Änderung dort (main: „dev-a arbeitet
gerade in tests/test_bindungstabelle.py, und ich committe gleich") und ist nicht Teil dieses
Auftrags.
