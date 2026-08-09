# Klasse-3 fail-open — {k: slots[k] for k in (...) if k in slots} (api.py:485/:743)

Datum: 2026-08-09/10. Auftrag (team-lead): Klasse 3 aus
`reports/adjudikation/slot_fn_fail_open_sweep_2026-08-08.md` Abschnitt 1.2 — zwei DictComp-
Lesestellen in `_bescheid_fn` (`produkt/haut/api.py`, quantitaet `festzusetzende_est`/
`festzusetzende_est_gesamt`), die wie Subscript-Zugriff (fail-closed) aussehen, es aber nicht
sind: `if k in slots` lässt einen fehlenden Key lautlos aus `wk_input` verschwinden statt zu
werfen. Der Vorgänger-Report hat diese Stellen NUR am Code nachvollzogen, keinen eigenen
`/ergebnis`-Differenzbeweis gefahren ("Bewusst nicht gemessen", Punkt 3). Dieser Report holt die
Messung nach.

---

## Teil 1 — Messen, bevor geändert wird

### 1.1 Die zwei Fundstellen (vor dem Fix)

`produkt/haut/api.py`, quantitaet `festzusetzende_est` (Scheibe `an_gesamt`), Zeile 483-487:

```python
def slot_fn(slots: dict) -> int:
    wk_input = {"veranlagungszeitraum": vz,
                **{k: slots[k] for k in
                   ("arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr", "eigenes_oder_ueberlassenes_kfz")
                   if k in slots}}
```

quantitaet `festzusetzende_est_gesamt` (Scheibe `gesamt`), Zeile 742-745, identisches Muster
(`gesamt_wk_input` statt `wk_input`).

Beide füttern `golden/runner.py::catala_werbungskosten_n` (Zeile 180-205), die JEDE
WK-Komponente NOCHMAL mit eigenem `if "X" in s:` gated — "zwei fail-open-Ebenen hintereinander".

### 1.2 Reale HTTP-Mutationsprobe (echter Server, echte Requests)

Vorgehen wie `tests/test_paket_b_e2e_http.py::base` (`server.make_server(0)`, daemon-Thread,
`POST /fall` → `POST /fall/<id>/event` (Laien-Herkunft, `zustand: bestaetigt`) →
`GET /fall/<id>/ergebnis`). Mutation: `produkt/bindung/bindung_n_vor_gwg.yaml:28`
`signatur_slot: entfernung_km_roh` → `entfernung_km_roh_x` (einziger Bindungseintrag für diesen
Slot im ganzen Repo). Backup vorher (`/tmp/.../bindung_n_vor_gwg.yaml.bak`), danach `cp`
zurückkopiert, Restaurierung verifiziert:

```
$ git diff --stat -- produkt/bindung/bindung_n_vor_gwg.yaml
(leer)
```

Ergebnis (zwei Szenarien, gleicher Live-Baum, gleiche Mutation):

| Szenario | Aufrufstelle | HTTP-Status | `zahl_cent` | `grund` |
|---|---|---|---|---|
| `an_gesamt`, `veranlagung=einzel`, EP-Kegel voll (AN_GESAMT_KEGEL) | api.py:483-487 | **500** | — | `KeyError: 'entfernung_km_roh'` |
| `gesamt`, `veranlagung=einzel`, EP-Kegel voll (GESAMT_BASIS) | api.py:742-745 | **200** | 1.345.200 (statt 1.310.100) | **`bestaetigt`** (unverändert!) |

Szenario 1 crasht NICHT wegen Klasse 3, sondern wegen eines KO-LOZIERTEN bare-Subscript-Reads im
§101-Mobilitätsprämie-Block (api.py:611-618, `int(slots["entfernung_km_roh"]) > 0`), der VOR dem
stillen Pfad denselben Namen liest und wirft. Dieser Shield gilt nur, wenn `extras is not None`
(true auf dem `/ergebnis`-Pfad, api.py `_feste_zahl`) — auf dem `/stand`-Schätzpfad
(`nur_bestaetigt=False`, `extras` defaultet dort auf `None`, z. B. api.py:2102/2108/2164/2171)
fällt der Shield weg.

Szenario 2 zeigt den echten Klasse-3-Effekt UNMASKIERT: **351,00 EUR** (35.100 ct) Steuermehrbetrag
lautlos, `grund` bleibt `"bestaetigt"`. Deckt sich exakt mit dem bestehenden e2e-Paar
`tests/test_paket_b_e2e_http.py::test_kombiniert_mit_pendel_wk` (1.310.100 ct, mit EP) vs.
`::test_kombiniert_job_und_vermietung` (1.345.200 ct, ohne EP).

### 1.3 Direkte Reproduktion ohne Bindungs-Mutation (reiner Unit-Aufruf, entscheidender Befund)

Die HTTP-Probe oben BRAUCHT eine kaputte Bindung, um `k not in slots` zu erzeugen. Zusätzlich
geprüft: reicht eine kaputte Bindung als EINZIGER Auslöser, oder reicht schon ein `feld_werte`,
dem ein Pflicht-Kegel-Feld fehlt (unabhängig von der Bindung)? Direkter Aufruf von
`API._bescheid_fn(...)` (Rückgabewert ist die `bescheid_via_slots`-Closure, nimmt `feld_werte`
feld-id-verschlüsselt entgegen, exakt wie `tests/test_festzusetzende_est_scope.py:68` es tut) MIT
korrekter, unveränderter Bindung, aber `feld_werte` ohne den Key `ep_entfernung_km`:

```
$ python3 -c "... bf = API._bescheid_fn('festzusetzende_est', 2025, bindung, felder, store=None, nur_bestaetigt=True) ..."
MIT ep_entfernung_km:  992200 ct
OHNE ep_entfernung_km: 1024500 ct   # kein Fehler
Delta: 32300 ct = 323,00 EUR

$ python3 -c "... bf = API._bescheid_fn('festzusetzende_est_gesamt', ...) ..."
MIT ep_entfernung_km:  1310100 ct
OHNE ep_entfernung_km: 1345200 ct   # kein Fehler
Delta: 35100 ct = 351,00 EUR
```

Zweite Ebene isoliert geprüft (`golden/runner.py::catala_werbungskosten_n` direkt, ohne api.py):

```
$ python3 -c "import runner; print(runner.catala_werbungskosten_n({... mit entfernung_km_roh ...}))"
2156
$ python3 -c "import runner; print(runner.catala_werbungskosten_n({... ohne entfernung_km_roh ...}))"
0
```

**Befund**: Klasse 3 braucht KEINE kaputte Bindung — jeder Aufrufer, der `feld_werte` ohne ein
Pflicht-Kegel-Feld an die zurückgegebene Funktion übergibt, verliert die EP-Komponente lautlos.
`extras=None` (Default) heißt: kein §101-Shield, der stille Pfad ist der EINZIGE, der feuert.

---

## Teil 2 — Legitime-Aufrufer-Prüfung (Pflicht vor Fail-Closed-Fix)

Alle 4 Keys (`arbeitstage`, `entfernung_km_roh`, `oepnv_kosten_jahr`, `eigenes_oder_ueberlassenes_kfz`)
stehen in `EP_FELDER` (`api_constants.py:19`), das laut `SCHEIBEN`-Konfiguration
(`api_constants.py:340-406`) im Pflicht-Kegel (`"kegel"`) SOWOHL von `an_gesamt` ALS AUCH von
`gesamt` liegt.

- **Bestätigter `/ergebnis`-Pfad** (`_feste_zahl`, api.py:1635-1654): baut `feld_werte` per
  Dict-Comprehension über `scheibe_felder` = `cfg["kegel"]` — ALLE Kegel-Felder sind
  unconditional als Keys enthalten, NACHDEM der Meet-Gate (`ST.meet_zustand`) geprüft hat, dass
  alle bestätigt sind. Kein Weg, hier einen Key wegzulassen.
- **`/stand`-Schätzpfad** (`IV.intervall`, `produkt/unsicherheit/intervall.py:84-163`): jedes
  askable Feld ohne fixierbaren Wert (`NICHT_FIXIERBAR`) blockiert die GESAMTE Berechnung
  (`nicht_fix`-Frühausstieg, Zeile ~117-120) — auch hier kein Weg, ein Kegel-Feld einfach
  wegzulassen und trotzdem einen Zahlenwert zu bekommen.
- **Zweite Ebene, `catala_werbungskosten_n`**: die EIGENEN `if "X" in s:` je Komponente sind
  KEIN Bug — dHf/Verpflegung/Übernachtung/Arbeitsmittel werden in api.py bewusst BEDINGT in
  `wk_input`/`gesamt_wk_input` geschrieben (Tatbestandsprüfung, Zeile ~493ff), das ist die
  korrekte, gewollte Optionalität. Nur die 4 EP-Keys reiten fälschlich auf demselben Muster, obwohl
  sie NIE optional sind, wenn dieser `slot_fn` überhaupt läuft. `catala_werbungskosten_n` hat
  heute exakt zwei Aufrufer (api.py:554/827, beide Klasse-3-Stellen) plus Testcode — kein
  legitimer Aufrufer, der `entfernung_km_roh` weglässt.

**Ergebnis**: kein legitimer Aufrufer verlangt das Weglassen dieser 4 Keys. Der `if k in slots`-
Filter hat an beiden Stellen keine reale Funktion — er verwandelt einen strukturell unmöglichen
Zustand (Bindung kaputt / Kegel verletzt) von einem lauten Fehler in einen stillen Steuerfehler.

**Entscheidung**: fail-closed. `if k in slots` an beiden Stellen entfernt — bare Subscript,
identisch zum bereits etablierten Muster in derselben Funktion (Zeile 461-467, `abziehbarer_betrag`,
und Zeile 611-618, §101-Block) und zum Vorgänger-Fix `04f51db` (`slots.get` → `slots[...]`).
Zweite Ebene (`golden/runner.py`) NICHT geändert — sie bleibt der korrekte, generelle Mechanismus
für die anderen, legitim optionalen WK-Komponenten; mit dem Fix an Ebene 1 sieht sie die 4
EP-Keys für diese beiden Aufrufer nie mehr als fehlend (sonst wirft Ebene 1 vorher).

---

## Teil 3 — Fix + Beweis

### 3.1 Änderung

`produkt/haut/api.py`, beide Stellen: `**{k: slots[k] for k in (...) if k in slots}` →
`**{k: slots[k] for k in (...)}` (Filter entfernt), plus Kommentar mit Verweis auf diesen Report.

### 3.2 Mutationsbeweis (Pflicht: neuer Test MUSS rot werden)

Neue Datei `tests/test_klasse3_dictcomp_wk_fail_open.py`, drei Tests. Beweis über echte
Code-Mutation (nicht Bindung — Ebene selbst): `git stash push -- produkt/haut/api.py` (Fix
zurückgenommen), Testlauf:

```
$ git stash push -- produkt/haut/api.py
$ python3 -m pytest tests/test_klasse3_dictcomp_wk_fail_open.py -q
FAILED tests/test_klasse3_dictcomp_wk_fail_open.py::test_an_gesamt_wirft_statt_still_zu_droppen_wenn_ep_entfernung_km_fehlt
  Failed: DID NOT RAISE KeyError
FAILED tests/test_klasse3_dictcomp_wk_fail_open.py::test_gesamt_wirft_statt_still_zu_droppen_wenn_ep_entfernung_km_fehlt
  Failed: DID NOT RAISE KeyError
2 failed, 1 passed in 1.00s
$ git stash pop
```

Nach `git stash pop` (Fix wieder da): alle 3 Tests grün (siehe 3.3).

### 3.3 Aufrufer-Check nach dem Fix

`catala_werbungskosten_n` (golden/runner.py) — nicht geändert, kein Aufrufer betroffen.
`slot_fn` in beiden quantitaet-Zweigen — einzige Aufrufer sind `IV.bescheid_via_slots` intern
(unverändert) und die Produktionspfade `_feste_zahl`/`IV.intervall`, die beide (Teil 2) niemals
ein Kegel-Feld weglassen — kein Aufrufer wird durch den KeyError neu brechen.
`grep -rn "festzusetzende_est\b\|festzusetzende_est_gesamt" produkt/haut/api.py` bestätigt: keine
weiteren Aufrufer außerhalb der bereits geprüften `_feste_zahl`/`/stand`-Pfade.

### 3.4 Suite

Baseline vor dieser Session: 1668 passed, 4 skipped, 0 failed. Repo wird PARALLEL von anderen
Teammates bearbeitet (Makefile/server.py/conftest.py/neue Tests, nicht Teil dieser Änderung) —
Zahlen unten sind der volle Stand des gemeinsamen Checkouts zum Messzeitpunkt, nicht isoliert auf
D1.

```
$ python3 -m pytest -q
1 failed, 1669 passed, 4 skipped, 1 warning in 273.23s
FAILED tests/test_audit.py::TestPiiFrei::test_alle_audit_aufrufer_sind_bekannt
  subprocess.TimeoutExpired: grep ... timed out after 30 seconds
```

Nachgeprüft — Last-Flake, kein Zusammenhang mit dieser Änderung (test_audit.py betrifft PII-Scan,
nicht api.py/runner.py):

```
$ time timeout 60 grep -rn "audit.append(" ... --include=*.py | wc -l
35
real 0m48.222s
```

Der reine `grep`-Aufruf allein braucht unter aktueller Systemlast 48s — über dem 30s-internen
Timeout des Tests. Isoliert wiederholt (`tests/test_klasse3_dictcomp_wk_fail_open.py` einzeln,
Section 3.2) grün, kein eigener Beitrag zu diesem Flake.

---

## Zusammenfassung

- Geld verschwindet lautlos, gemessen an ZWEI unabhängigen Wegen: HTTP-Differential (Bindung
  mutiert, gesamt-Szenario: 351,00 EUR) und direkter Unit-Aufruf (Bindung unverändert, Kegel-Feld
  in `feld_werte` weggelassen: an_gesamt 323,00 EUR, gesamt 351,00 EUR).
- Fail-closed gebaut (bare Subscript statt `if k in slots`) an BEIDEN Stellen, nach Pflicht-Check,
  dass kein legitimer Aufrufer den Weglass-Fall braucht.
- Zweite Ebene (`catala_werbungskosten_n`) bewusst unverändert — sie ist der korrekte Mechanismus
  für andere, echt optionale WK-Komponenten; mit Ebene 1 fixiert sieht sie die 4 EP-Keys nie mehr
  fehlend.
- Neuer Test `tests/test_klasse3_dictcomp_wk_fail_open.py`, real rot bewiesen via
  `git stash`-Mutation des Fixes selbst.
