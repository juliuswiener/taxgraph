# Paket B — Haut (Privat-Oberfläche) — Konzept-Skizze

**Zone:** `produkt/haut/` (neu, additiv). **Status:** SKIZZE zur Instructor-Abnahme VOR dem Bau
(Schema-first = teuerste Fehlerquelle zuerst). **LLM-frei, $0** in dieser Stufe.
**Naht:** ausschließlich `produkt/traverser/API.md` — die Haut fasst Engine/Registry/rules.yaml/
params/sources NIE an, konsumiert nur Traverser (read) + `store.append_event` (der einzige Schreibpfad)
+ Bindungstabelle + `intervall.py` + `est_mapping.deklariere` + ELSTER-Ampel.

## 0. Zonen-Grenze (mechanisch, aus API.md)

Die Haut ist eine **dünne HTTP-Hülle** über die Paket-A-Naht. Sie enthält **keine** Steuerlogik,
**keine** zweite Wahrheit, **keinen** zweiten Schreibpfad. Jede Zahl kommt aus dem Kern (Engine über
`intervall.py`/Golden-Runner). Fail-closed ist im Store erzwungen, nicht in der Haut — die Haut kann
die Garantien strukturell nicht verletzen (ein `llm:`-Schreiber ist an `vorlaeufig` gekoppelt, ein
`bestaetigt` erfordert `signal_2`, ein KI-Wert kann nicht in die Summe fließen).

## 1. Backend-Zuschnitt

**Empfehlung: stdlib `http.server.ThreadingHTTPServer` + eigener Mini-Router, NULL Web-Framework.**

Begründung (Ponytail: erst stdlib, bevor Dependency):
- Der Backend-Auftrag ist eine **dünne JSON-Hülle** über bereits fertige Python-Funktionen — wenige
  Endpunkte, JSON rein/raus, ein Schreibpfad. Kein ORM, keine Sessions, keine Templates, kein Auth-Stack.
- **Schema-first liefere ich selbst** über JSON-Schema-Dateien (`produkt/haut/api_schema/*.json`, Muster
  `produkt/bindung/schema.json`) + Handvalidierung am Endpunkt-Rand — das braucht kein Framework, und
  ein Framework (FastAPI/Pydantic) zöge starlette+pydantic+uvicorn als Zoo herein, den Julius explizit
  ausgeschlossen hat.
- **Concurrency-Bedarf ist gering:** ein Nutzer je Fall. Der einzige echte Parallelismus ist der
  ELSTER-Check (UI-Lab: „seriell je Slot, Worker-Pool für Parallelität, warm p95 76 ms") — das deckt
  ein `concurrent.futures.ThreadPoolExecutor`, kein async-Server.
- **Upgrade-Pfad offen (YAGNI):** falls Last/Streaming es je verlangt, ist der Wechsel auf
  `uvicorn`+`starlette` (ohne Pydantic-Zwang) ein isolierter Austausch der Transport-Schicht — die
  Endpunkt-Funktionen bleiben, weil sie reine `(request_json) -> response_json`-Hüllen sind. Erst bauen,
  wenn gemessen nötig.

### Endpunkt-Landkarte (jede Zeile = dünne Hülle über genau eine Naht-Funktion)

| Methode + Pfad | Naht-Aufruf | liefert / bewirkt |
|---|---|---|
| `GET /fall/{id}/fragen` | `traverser.naechste_fragen` + Bindungs-Metadaten je Feld | Mobile-Wegpunkt-Queue: `fragetext_laie, typ, einheit, bereich, beispielwert, enum_werte, herkunft-badge, anker_ref` |
| `GET /fall/{id}/stand` | `store.materialisiere` + `traverser.relevanz` + `intervall.intervall` | Snapshot + Regel-Status + `[min,max]`-Ring + Steuer-at-Risk |
| `POST /fall/{id}/event` | `store.append_event` — **EINZIGER Schreib-Endpunkt** | schreibt `{feld_id,wert,zustand,herkunft,schreiber,signal,ersetzt}`; Zwei-Signal-Bestätigung UND llm-vorläufig laufen beide hier durch, fail-closed im Store |
| `GET /fall/{id}/feld/{fid}/warum` | `traverser.justification` | Herkunfts-Kette Beleg→Extraktion→Vorschlag→Paragraph (antippbares Badge) bis `anker_ref` |
| `GET /fall/{id}/ergebnis` | fail-closed feste Zahl + `traverser.trace_ergebnis` | Zahl NUR wenn Input-Kegel bestätigt (`store.meet_zustand`), sonst `null`; + Vorwärts-Trace je Regel |
| `GET /fall/{id}/deklaration` | `est_mapping.deklariere` | ELSTER-Deklarationsvorschau (Store→E-Nr), lossy-transparent |
| `POST /fall/{id}/elster-ampel` | `elster/checkest_gate` über warmen Daemon (ThreadPool) | Abschnitts-Ampel; Befund an `snapshot_id` gebunden; `gekappt_verdacht=true` ist **nie grün** (API.md-Garantie 5) |
| `POST /fall/{id}/chat` | **Platzhalter (Stufe später)** — schreibt qua Naht nur `vorlaeufig`/`llm_vorschlag` | Explain-Panel-Kontrakt; **KEIN LLM-Call in dieser Stufe** |

Persistenz: `store.lade`/`store.speichere` (ein JSON je `fall_id` unter `produkt/haut/faelle/`). Kein DB.

## 2. Mobile-first Wegpunkt-Fluss (Primärpfad — UI-Lab 6b0b165 als Design-Input)

Ein Bildschirm = **ein Wegpunkt** (`naechste_fragen[0]`) + was er am Ergebnis ändert. Die fünf
UI-Lab-Dimensionen binden 1:1 an die Naht:

1. **Herkunft zum Anfassen** — Badge je Feld aus `herkunft` (solide = Beleg/`laie`, schimmernd =
   `llm_vorschlag`); Antippen ruft `/warum` → volle Kette bis `anker_ref`. Kette erscheint IM
   Bestätigungsmoment.
2. **Bestätigen mit Unsicherheits-Gefühl** — Geste nach Konfidenz: sicherer Beleg = ein Tipp; mittlere
   KI = Halten (Fortschritts-Ring); niedrige = Doppel-Bestätigung. Jede Geste ist EIN `bestaetigt`-Event
   mit `signal_2` über `/event` (Zwei-Signal). Schwellen (85/60) = Design-Entscheid im Bau.
3. **Navigation ohne 200-Fragen-Wand** — immer nur der nächste freigeschaltete Wegpunkt; die Queue
   kommt aus `naechste_fragen` (Gating zuerst, dann Unsicherheits-Beitrag aus `intervall.py`).
4. **Bescheid als schrumpfender Ring** — Ergebnis startet als `[min…max]` aus `intervall.intervall`;
   jede Bestätigung zieht die Spanne monoton enger (konzentrische Ringe). Fortschritt = Form, nicht Zahl.
5. **Chat als Berater daneben** — Explain-Slot gleich groß neben dem Bestätigen-Knopf, situativ (ELSTER-
   Widerspruch / großer Vorjahres-Sprung). Erklärt + verlinkt Paragraph+Beleg, **setzt nie Werte**
   (Stufe später, siehe §4).

**Fragetexte kommen AUSSCHLIESSLICH aus `bindung.fragetext_laie`** — die Haut erfindet nie eigene Texte
(das Schema verbietet `§`/`EStG`/`Abs.` im Fragetext bereits mechanisch). **Gesten-Richtung = Herkunft**
(wisch links = Beleg, oben = Gesetz, unten = vorläufig parken) wandert als `signal` mit ins `/event` →
Audit-Log (Beweis bewusster Entscheidung).

## 3. Desktop-Graph (Zusatzansicht, nachrangig)

Desktop bekommt zusätzlich den Abhängigkeits-Graphen: `traverser.relevanz` liefert je Regel
`{status, gates_offen, annahmen_offen}` → Karte zum Überblick + gezieltem Springen. Nach dem
Mobile-Fluss gebaut, nicht davor.

## 4. LLM-Chat-Slot (nur Architektur-Platzhalter in dieser Stufe)

Reserviert wird **nur die Position + der Vertrag**, nicht die Anbindung:
- **Position:** Explain-Panel neben dem Bestätigen-Knopf (gleiche Größe, UI-Lab Dim 5).
- **Vertrag:** Chat schreibt qua Store-Auflage A **ausschließlich** `vorlaeufig`-Events mit
  `schreiber="llm:…"`, `herkunft.herkunft="llm_vorschlag"`, `signal_2=null`. Eine gefälschte Herkunft
  wird im Store hart abgewiesen (`ValueError`). Die Bestätigung bleibt der menschliche Zwei-Signal-Klick.
- **KEIN LLM-Call in dieser Stufe.** Die tatsächliche Chat-Anbindung ist eine **spätere Stufe mit
  eigenem Julius-Cap**. Der Endpunkt `POST /chat` existiert jetzt nur als leere Hülle, die den Vertrag
  fixiert (damit die Architektur nicht später umgebaut werden muss).

## 5. e2e-Durchstich-Plan (die Integrationsprobe über HTTP nachgefahren)

`tests/test_paket_b_e2e_http.py` fährt `tests/test_paket_a_e2e.py` Schritt für Schritt über die
Endpunkte nach (gleiche Asserts, gleiche EP-Familie, gleiche 2156 €), gegen den laufenden
ThreadingHTTPServer (Test-Fixture startet/stoppt ihn; Catala-Toolchain-Skip wie im A-Test):

1. leerer Fall → `GET /fragen` == die 4 EP-Felder
2. `POST /event` ×3 laie-bestätigt (entfernung/kfz/oepnv) + ×1 llm-vorläufig (arbeitstage)
3. `GET /stand` → Spanne `> 0`, `arbeitstage` bounded (0..366), Queue == `[ep_arbeitstage]`
4. `GET /ergebnis` → `null` (Kegel enthält `vorlaeufig`) — **fail-closed vorher**
5. `POST /event` Zwei-Signal-Bestätigung (`ersetzt=llm-event`, `signal_2`)
6. `GET /stand` → Spanne `== 0` (Punkt, monoton geschrumpft)
7. `GET /ergebnis` → `2156 €`; `GET /warum` → Justification bis `anker_ref`

**Erste Scheibe = EP-Familie** (Bindung + Engine + Goldens vollständig vorhanden). Weitere Scheiben
(N+VOR+GWG, Rentner, KAP/VV) danach, jede über denselben Durchstich verifiziert.

## Schema-first-Artefakte, die VOR dem Bau festliegen (mit dieser Skizze / direkt danach)

- `produkt/haut/api_schema/*.json` — Request/Response-JSON-Schema je Endpunkt (2020-12), Muster
  `bindung/schema.json`; Gate validiert Beispiel-Payloads.
- Fehler-Kontrakt: fail-closed-Antworten (`ergebnis=null`, ELSTER `gekappt_verdacht`) sind explizite
  Response-Formen, kein HTTP-500.

## Offene Entscheide für den Instructor (Abnahme-Punkte)

1. **Backend-Stack:** stdlib `ThreadingHTTPServer` (Empfehlung, null Framework) — oder soll ich
   `uvicorn`+`starlette` als Fundament setzen (async, aber Dependency-Zuwachs)?
2. **Fall-Persistenz-Ort:** `produkt/haut/faelle/*.json` (gitignored) ok, oder eigener State-Pfad?
3. **Frontend-Lieferform dieser Stufe:** Nur JSON-API + ein minimaler statischer Mobile-Prototyp
   (HTML/CSS/Vanilla-JS, kein Build-Step) — oder API-only, Frontend als eigene Folge-Scheibe?
4. **Scheiben-Reihenfolge nach EP:** N+VOR+GWG als zweite (deckt Summen-Konvention + bool-Bedingung)?

**Kein Code in dieser Stufe** — nach Abnahme: `produkt/haut/` Server + Endpunkt-Hüllen + `api_schema/`
+ `tests/test_paket_b_e2e_http.py`, EP-Scheibe zuerst.
