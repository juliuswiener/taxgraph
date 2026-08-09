# Dialog-Lücke — fragt das Produkt alles, was die Abgabe braucht?

Auftrag von team-lead, 2026-08-09/10: reine Messung, keine Implementierung. Frage: kann ein
Nutzer über die tatsächliche Oberfläche (`/fragen`, `produkt/traverser/`) alles eingeben, was
für eine abgabefähige Erklärung nötig ist — ohne den Store wie in bisherigen Messungen direkt
im Testcode zu bauen? Regeln: keine Datei außer diesem Bericht geändert, nichts committet,
jede Zahl mit erzeugendem Befehl, leere Suchen als "0 Treffer für ...", Hersteller-ID nie
ausgegeben (überall `<ID>`).

Umgebung: HEAD zu Beginn der Messung `04f51db`, Repo unverändert während der Messung außer
diesem Bericht. Scheibe `gesamt`, VZ 2025.

## Ergebnis vorweg

- **Abgabe braucht:** 78 Positionen — 73 Feld-IDs mit `elster_kz` in der `gesamt`-Bindung,
  plus 5 `absender_*`-Parameter, die `erzeuge_xml(abgabefaehig=True)` fail-closed verlangt.
- **Dialog bietet:** 73 von 78 — alle 73 Kz-tragenden Feld-IDs sind erreichbar. 0 davon fallen
  in Fall (b) oder (c).
- **Differenz:** 5 — ausschließlich die `absender_*`-Parameter, ausschließlich Fall (a).
- Zwei Nebenbefunde jenseits der a/b/c-Tabelle (unten begründet): der Live-Endpunkt
  `einreichen()` fährt den `abgabefaehig=True`-Pfad nie (bereits bekannt, hier unabhängig
  bestätigt), und ein struktureller Gate-Defekt lässt Partner-Felder auch bei
  Einzelveranlagung im Angebot stehen.

## Schritt 1 — Was fragt das Produkt?

`fragen()` (`produkt/haut/api.py:2121`) baut die Kandidatenmenge über
`_scheibe_bindung(store)` → `TR.naechste_fragen(store, bindung, beitrag)`. `_scheibe_bindung`
schneidet die Bindungstabelle exakt auf `SCHEIBEN[store["scheibe"]]["felder"]` zu
(`produkt/haut/api.py:117-129`).

```python
import api_constants as C
len(C.SCHEIBEN["gesamt"]["felder"])
```
→ **193** Feld-IDs im Universum der `gesamt`-Scheibe.

`naechste_fragen()` (`produkt/traverser/traverser.py:106-124`) gibt alle askable, unbeantworteten
Felder zurück, deren Regel laut `relevanz()` nicht `"ausgeschlossen"` ist. `relevanz()`
(`traverser.py:73-103`) schließt eine Regel NUR aus, wenn ein Gate (askable Feld mit
`geltungsbedingung` in der eigenen `quelle`) einen bestätigten Wert `False` trägt. Vor dem
ersten Event ist kein Gate beantwortet — also ist zu Beginn keine Regel ausgeschlossen.

Live-Probe, leerer Store, kein einziges Event:

```python
s0 = ST.leerer_store(2025, fall_id="probe_t0"); s0["scheibe"] = "gesamt"
len(TR.naechste_fragen(s0, {f: TR.lade_bindung()[f] for f in C.SCHEIBEN["gesamt"]["felder"]}, None))
```
→ **193** — die Angebotsmenge bei t=0 deckt sich exakt mit dem vollen Feld-Universum der
Scheibe. Das ist keine Näherung, sondern eine strukturelle Konsequenz: da Ausschluss nur über
ein bestätigtes `False` läuft und bei t=0 nichts bestätigt ist, kann zu Beginn nichts
ausgeschlossen sein.

Nach `veranlagung="einzel"` gesetzt:

```python
b(s1, "veranlagung", "einzel")   # ST.append_event, wert="einzel"
len(TR.naechste_fragen(s1, bindung_gesamt, None))
```
→ **192** (193 minus `veranlagung` selbst, das jetzt beantwortet ist).

Nach `veranlagung="zusammen"` gesetzt (eigener Store, gleiche Bindung): ebenfalls **192**, und
als Menge **identisch** zur Einzel-Queue minus `veranlagung`
(`set(q_zusammen) == set(q_einzel)` → `True`, live geprüft). Für die Feld-IDs mit `elster_kz`
macht Einzel- vs. Zusammenveranlagung damit **keinen Unterschied im Angebot** — beide Zweige
bieten dieselben 192 Felder an, bevor der Nutzer weiter antwortet.

**Warum das kein Zufall ist:** die Ausschluss-Semantik hängt an genau EINEM Mechanismus (Gate
= askable Feld, eigener bestätigter Wert `False`). Ein Blick auf alle 74 askable Felder mit
`geltungsbedingung` in der eigenen `quelle` zeigt: 40 sind `bool` (können den Mechanismus
tatsächlich auslösen), 34 sind es NICHT (`enum`, `int`, `text`, `cent`, `datum` — deren
bestätigter Wert ist nie das Python-Objekt `False`, der Ausschluss kann für diese Regeln
strukturell nie greifen). Darunter fallen u.a. `stammdaten_nachname_partner`,
`stammdaten_vorname_partner`, `stammdaten_geburtsdatum_partner`, `kist_konfession_partner`,
`person_b_idnr` — alle fünf mit `geltungsbedingung: beide_ehegatten_zusammen_veranlagt`, aber
keins davon `bool`. Live bestätigt:

```python
[f for f in TR.naechste_fragen(s1, bindung_gesamt, None) if f.endswith("_partner") or f == "person_b_idnr"]
```
→ **26** Partner-Felder stehen im Angebot, obwohl `veranlagung="einzel"` bereits bestätigt ist.
`TR.relevanz(s1, bindung_gesamt)["p2_festzusetzung_zusammen"]` → `{"status": "unentschieden", ...}`
— nie `"ausgeschlossen"`.

Das ist kein Fall-(a)/(b)/(c)-Befund im Sinn des Auftrags (die Abgabe braucht diese
Partner-Felder bei Einzelveranlagung ja gerade NICHT), sondern die Kehrseite: der Dialog bietet
mehr an, als bei Einzelveranlagung nötig wäre, weil das Gate, das diese Regel bei
Einzelveranlagung ausschließen soll, mechanisch nie greifen kann (nicht-`bool`-Feld als Gate).
Wird hier nur gemessen, nicht bewertet — die Entscheidung, ob das ein Fix-Kandidat ist, liegt
außerhalb dieses Auftrags.

## Schritt 2 — Was braucht die Abgabe?

**A) Feld-seitig — Feld-IDs mit `elster_kz` in der `gesamt`-Bindung:**

```python
kz_felder = [f for f in C.SCHEIBEN["gesamt"]["felder"]
             if TR.lade_bindung()[f].get("elster_kz") not in (None, "null")]
len(kz_felder)
```
→ **73**.

Kontrollrechnung — fehlen Bindungseinträge oder ist irgendeines der 193 Felder nicht askable?

```python
b_all = TR.lade_bindung()
[f for f in C.SCHEIBEN["gesamt"]["felder"] if f not in b_all]           # Fall-(a)-Kandidaten
[f for f in C.SCHEIBEN["gesamt"]["felder"] if not b_all[f].get("askable")]  # Fall-(b)-Kandidaten
```
→ beide **0 Treffer**. Innerhalb der 193 Felder der `gesamt`-Scheibe existiert also weder
Fall (a) noch Fall (b) — jedes gelistete Feld hat eine Bindung und ist askable.

**B) Absender-seitig — die fünf `erzeuge_xml(abgabefaehig=True)`-Pflichtparameter**
(`produkt/import/elster_xml.py:321-324`, fail-closed geprüft `:467-471`, Präfix-Check
`:484-493`, Commit `e365a37`): `absender_name`, `absender_strasse`, `absender_plz`,
`absender_ort`, `absender_steuernummer`.

**Abgabe braucht gesamt: 73 + 5 = 78.**

## Schritt 3 — Die Differenz

**A) Die 73 Kz-Feld-IDs — 0 Differenz.**

```python
q0 = set(TR.naechste_fragen(s0, bindung_gesamt, None))   # t=0-Angebot, s.o.
[f for f in kz_felder if f not in q0]
```
→ **0 Treffer** — alle 73 Kz-tragenden Feld-IDs stehen im t=0-Angebot. Kein Fall (c): keins
der Kz-Felder ist askable+vorhanden, aber vom Fragegraph unerreicht.

**B) Die 5 `absender_*`-Parameter — vollständige Differenz, alle Fall (a).**

```bash
grep -rn "feld_id.*absender\|absender_name\|absender_strasse\|absender_plz\|absender_ort\|absender_steuernummer" \
     produkt/bindung/*.yaml produkt/haut/api_constants.py
```
→ **0 Treffer**. Kein `absender_*` existiert irgendwo als `feld_id`.

```bash
grep -rln "absender_name\|absender_strasse\|absender_plz\|absender_ort\|absender_steuernummer" \
     --include="*.py" --include="*.yaml" . | grep -v /.git/ | grep -v elster_xml.py \
     | grep -v ^./tests/ | grep -v BACKLOG.yaml
```
→ **0 Treffer**. Die einzigen Fundstellen im gesamten Repo außerhalb von `elster_xml.py`
selbst (wo die Parameter definiert sind) und einer BACKLOG-Prosa-Zeile (`BACKLOG.yaml:131`)
sind in `tests/test_checkest_durchstich.py` — die dort hart codierte Konstante:

```python
# tests/test_checkest_durchstich.py:145-147
_ABSENDER = dict(absender_name="Maier Hans", absender_strasse="Musterstr. 55",
                 absender_plz="55555", absender_ort="Musterort",
                 absender_steuernummer="9181081508155")
```

Der Kommentar direkt darüber (Zeilen 140-144) sagt es selbst: "Sie liegen noch nicht als
Fall-Felder vor (Bau läuft)". Alle fünf `absender_*` sind reine Fall (a) — gar nicht als
Feld vorhanden, weder askable noch nicht-askable, weil es sie als `feld_id` schlicht nicht
gibt.

**Feinere Auflösung innerhalb von Fall (a) — vier der fünf haben einen unverdrahteten
Zwilling, einer nicht:**

Die Stammdaten-Arbeit eines anderen Workers (parallel im Gang, vgl. `produkt/haut/
api_constants.py:334-339` `STAMMDATEN_FELDER`) hat für vier der fünf Absender-Werte bereits
ein semantisch passendes, askable, Kz-tragendes Gegenstück gebaut — nur für die
Erklärung, nicht für den Vorsatz-Block:

| `absender_*` | Analoges Dialog-Feld | elster_kz | askable |
|---|---|---|---|
| `absender_name` | `stammdaten_nachname` + `stammdaten_vorname` | E0100201 / E0100301 | ja |
| `absender_strasse` | `stammdaten_strasse` + `stammdaten_hausnummer` | E0101104 / E0101206 | ja |
| `absender_plz` | `stammdaten_plz` | E0100601 | ja |
| `absender_ort` | `stammdaten_wohnort` | E0100602 | ja |
| `absender_steuernummer` | — kein Analog gefunden | — | — |

```bash
grep -rn "feld_id:.*steuernummer\|feld_id: steuernummer" produkt/bindung/*.yaml
```
→ **0 Treffer für "feld_id: steuernummer" in produkt/bindung/\*.yaml** — für
`absender_steuernummer` existiert nicht einmal ein unverdrahteter Kandidat, im Unterschied zu
den anderen vier.

Zwischen den vier `stammdaten_*`-Feldern und den vier `absender_*`-Parametern gibt es **keine
Verbindung im Code** — kein Umbenennungs-, Concat- oder Übergabeglied. Beide Repräsentationen
derselben realen Angabe (Name/Adresse des Erklärenden) existieren nebeneinander, unverbunden:
genau die Naht-Klasse, auf die team-lead hingewiesen hat. Formal bleibt es Fall (a) — es gibt
kein Feld namens `absender_name` — aber der Fix-Aufwand für vier der fünf Parameter ist kleiner
als für `absender_steuernummer`, wo auch kein Rohstoff existiert.

## Nebenbefund: Live-Endpunkt fährt den `abgabefaehig`-Pfad nicht

Unabhängig von Feld-Verfügbarkeit besteht eine zweite, in `reports/adjudikation/
einreichen_mock_naht_2026-08-09.md` bereits dokumentierte Lücke, hier gegengeprüft am
aktuellen Stand (`produkt/haut/api.py:2394-2396`):

```python
xml = EX.erzeuge_xml(result, vz=vz,
                     empfaenger_land=str(body.get("empfaenger_land") or "BY"),
                     testmerker=EX.TESTMERKER_ERIC)
```

Kein `abgabefaehig=True`, kein `absender_*`. Selbst wenn alle fünf Absender-Werte als Felder
existierten, askable wären und beantwortet würden, hinge der `<Vorsatz>`-Block nicht an —
`einreichen()` ruft die abgabefähige Variante schlicht nicht auf. Diese Lücke ist von der
Feld-Lücke oben unabhängig und würde durch das Schließen der Feld-Lücke allein nicht behoben.

## Bilanz für team-lead

Abgabe braucht 78 Positionen (73 Kz-Feld-IDs + 5 `absender_*`). Der Dialog bietet 73 von 78 —
alle Kz-Feld-IDs, unabhängig von Einzel- oder Zusammenveranlagung, bereits ab dem ersten
Aufruf von `/fragen`. Die volle Differenz sind die 5 `absender_*`-Parameter, ausnahmslos
Fall (a) — gar nicht als Feld vorhanden. Vier davon (`absender_name`, `absender_strasse`,
`absender_plz`, `absender_ort`) haben ein unverdrahtetes Gegenstück in den neuen
Stammdaten-Feldern; `absender_steuernummer` hat keins. Fall (b) und (c) kommen in dieser
Messung nicht vor. Zusätzlich, aber getrennt zu werten: `einreichen()` fährt den
`abgabefaehig=True`-Pfad aktuell gar nicht (bekannt, hier bestätigt), und ein nicht-`bool`
Gate-Defekt lässt 26 Partner-Felder auch bei Einzelveranlagung im Angebot stehen (Über-, nicht
Unterangebot — kein a/b/c-Fall, aber dieselbe strukturelle Fehlerklasse: eine
Geltungsbedingung, die nicht an ein tatsächlich prüfbares Gate gekoppelt ist).
