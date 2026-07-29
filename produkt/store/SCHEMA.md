# Sachverhalts-Store — Schema (UI-Kern, Task #11)

**Zone:** `produkt/` (neu, additiv, LLM-frei im Kern). **Status:** SCHEMA zur Instructor-Abnahme
VOR dem Bau des Store-Mechanismus + Gate (Schema-first, wie bei der Bindungstabelle).
**Julius-Entscheide:** #2 (KI-Sperre fest ins Datenmodell: `Vorlaeufig<T>`/`Bestaetigt<T>` als Typ,
Herkunfts-Vektor als Payload, Meet über Input-Kegel), #4 (Store = Protokoll + Schnappschüsse; ERiC-Befund
bindet an Snapshot-Hash).

## Was der Store ist

Der EINE Wahrheits-Store eines Steuerfalls (Lab: „Store ist Wahrheit, viele Schreiber"). Zwei
Strukturen, beide in einer Datei je Fall/VZ:

1. **Event-Log** (`events`) — **append-only**. Jedes Event schreibt EINEN Feldwert mit Zustand +
   Herkunft. Nichts wird gelöscht; ein Korrektur-Event trägt `ersetzt: <event_id>` und gewinnt bei der
   Materialisierung. Der Log ist die Wahrheit.
2. **Content-adressierte Snapshots** (`snapshots`) — deterministische Materialisierung eines
   Log-Präfixes (`feld_id → aktueller Wert`), adressiert per `snapshot_id = sha256(felder)`. Der
   ERiC-Befund bindet an genau diesen Hash → eine Prüfung gilt nachweislich für EINEN Zustand.

## Der Wert-Typ: `zustand` (Julius #2)

`zustand ∈ {vorlaeufig, bestaetigt}` ist der materialisierte `Vorlaeufig<T>`/`Bestaetigt<T>`-Typ.
Ordnung (Verband):

```
vorlaeufig  ⊑  bestaetigt        (vorlaeufig ist das schwächere/kleinere)
Meet(a, b) = das schwächere von beiden.
```

**Fail-closed / Meet über Input-Kegel:** ein berechneter Wert (Aggregat) ist nur `bestaetigt`, wenn
**alle** seine Eingaben `bestaetigt` sind. Hängt irgendwo ein `vorlaeufig` im Input-Kegel, ist das
Ergebnis `vorlaeufig` — und eine festzusetzende Steuerzahl wird strukturell NICHT ausgegeben
(K2: „Summe ist strukturell keine Zahl"). Das ist der Typ als Enforcement, nicht als Bitte.
(Die Meet-Rechnung ist Engine-Logik; das Schema definiert nur den Typ + die Ordnung.)

## Der Herkunfts-Vektor (Payload, dev-2-Fund)

Vertrauen ist ein **Vektor**, keine Leiter — drei unabhängige Achsen, Meet läuft **pro Achse** getrennt
(so kann ein Wert amtlich-geprüft, aber vom Nutzer verantwortet sein, ohne dass eine Achse die andere
verdeckt):

| Achse | Werte (aufsteigend) | Bedeutung |
|---|---|---|
| `herkunft` | laie · llm_vorschlag · beleg_import · vorjahr · kontoauszug · edaten · berechnet · orakel | woher der Wert stammt (keine totale Ordnung — Kategorie; Meet = „unspezifischer"/Konflikt-Markierung) |
| `pruef_tiefe` | ungeprueft ⊑ plausibilisiert ⊑ orakel_bestaetigt ⊑ amtlich | wie tief geprüft |
| `haftung` | nutzer · berater · system · amt | wer haftet (AO § 150 Abs. 7 / § 153 / § 93c) |

`pruef_tiefe` ist eine Kette (Meet = Minimum). `herkunft`/`haftung` sind Kategorien; Meet zweier
verschiedener Werte markiert einen **Herkunfts-/Haftungs-Konflikt** (Oberfläche, nie auto-versöhnt —
dev-2 Split-Annunciator). Die genaue Meet-Regel je Achse steht als offener Punkt unten.

## Zwei-Signal-Bestätigung (K3)

- `signal.signal_1` = Vorschlag (z.B. LLM-Extraktion), **allein wirkungslos**.
- `signal.signal_2` = menschliche/deterministische Bestätigung neben dem Beleg.
- **Schema-Zwang:** `zustand: bestaetigt` erfordert `signal.signal_2` (nicht-leer). Ein Event mit
  `herkunft.herkunft = llm_vorschlag` MUSS `zustand: vorlaeufig` tragen (LLM darf nie direkt bestätigen —
  Julius #1: „Chat schreibt nur Vorlaeufig-Patches"). Beides mechanisiert im Schema.

## Content-Adressierung

- `event_id = sha256(canonical_json(event OHNE das Feld event_id))`.
- `snapshot_id = sha256(canonical_json(felder))`.
- `canonical_json` = `json.dumps(obj, sort_keys=True, ensure_ascii=False, separators=(",", ":"))`.
Deterministisch, vom Gate nachgerechnet (Tamper-fest: ein verändertes Event bekommt einen anderen
Hash → fällt auf). `ts` ist Teil des Event-Inhalts (geht in den Hash ein).

## Bindung an die Bindungstabelle

`event.feld_id` und die Snapshot-Feld-Keys binden an `produkt/bindung/` (feld_id). Der Gate prüft:
jede im Store verwendete `feld_id` existiert in einer Bindungstabelle. So bleibt der Store das
Gegenüber der Bindungstabelle (was gefragt werden darf ↔ was geantwortet wurde).

## Worked Example

```yaml
version: 1
fall_id: "fall-muster-2025"
veranlagungszeitraum: 2025
events:
  # 1) Laie tippt Arbeitstage, bestätigt selbst (Zwei-Signal: kein Vorschlag, direkter Beleg)
  - event_id: "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90"
    ts: "2026-07-17T14:00:00+00:00"
    feld_id: ep_arbeitstage
    wert: 220
    zustand: bestaetigt
    herkunft: {herkunft: laie, pruef_tiefe: ungeprueft, haftung: nutzer}
    schreiber: "ui:laie"
    signal: {signal_1: null, signal_2: "eingabe-bestaetigt@ep_arbeitstage"}
    ersetzt: null
  # 2) LLM schlägt Entfernung aus hochgeladenem Beleg vor -> NUR vorlaeufig
  - event_id: "b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1"
    ts: "2026-07-17T14:01:00+00:00"
    feld_id: ep_entfernung_km
    wert: 30
    zustand: vorlaeufig
    herkunft: {herkunft: llm_vorschlag, pruef_tiefe: ungeprueft, haftung: nutzer}
    schreiber: "llm:chat"
    signal: {signal_1: null, signal_2: null}
    ersetzt: null
snapshots:
  - snapshot_id: "0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0"
    ts: "2026-07-17T14:01:05+00:00"
    bis_event: "b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1"
    felder:
      ep_arbeitstage: {wert: 220, zustand: bestaetigt, herkunft: {herkunft: laie, pruef_tiefe: ungeprueft, haftung: nutzer}}
      ep_entfernung_km: {wert: 30, zustand: vorlaeufig, herkunft: {herkunft: llm_vorschlag, pruef_tiefe: ungeprueft, haftung: nutzer}}
    eric_befund:
      gebunden_an: "0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0"
      rc: 0
      klasse: plausibel
      gekappt_verdacht: false
      fehler_anzahl: 0
```
(Die `event_id`/`snapshot_id` im Beispiel sind Platzhalter-Hashes; der Gate rechnet die echten nach.)

## Gate-Vertrag (geplant, nach Schema-Abnahme)

`tests/test_store.py`: (a) Schema-Validierung; (b) `event_id`/`snapshot_id` content-adressiert korrekt
(nachgerechnet); (c) `snapshot.felder` = deterministische Materialisierung des Log-Präfixes bis
`bis_event` (append-only + `ersetzt`-Auflösung); (d) `eric_befund.gebunden_an == snapshot_id`;
(e) `feld_id` existiert in der Bindungstabelle; (f) Typ-Zwänge (bestaetigt→signal_2, llm→vorlaeufig).
Negativtests pflicht (manipuliertes Event → Hash bricht; llm+bestaetigt → rot; ERiC an falschem Hash → rot).

## Auflagen (Instructor abgenommen, msg 2377)

- **A Schreiber↔Herkunft-Kopplung (Schlupfloch geschlossen):** ein Schreiber `^llm:` MUSS sich ehrlich
  als `herkunft.herkunft=llm_vorschlag`, `zustand=vorlaeufig`, `signal_2=null` deklarieren — sonst
  könnte ein LLM über eine gefälschte Herkunft (`beleg_import`) einen `bestaetigt`-Wert schmuggeln.
  Mechanisiert im Schema (Conditional) UND hart im Schreibpfad `append_event`.
- **B Ein aktives Event je feld_id:** zwei Events auf dasselbe `feld_id` ohne `ersetzt`-Kette sind ROT.
  Neuschreiben erfordert `ersetzt` auf das aktuell aktive Event (Ziel muss existieren, selbes `feld_id`,
  nicht bereits ersetzt). Verhindert stille Divergenz bei der Materialisierung. (Gate.)
- **C `eric_befund.gekappt_verdacht` (Pflicht):** die Trunkierungs-Sperre reicht bis in den Store.
  **`klasse=plausibel` MIT `gekappt_verdacht=true` darf downstream NIE als grün gewertet werden** — die
  ELSTER-Lampe bleibt dann gelb, nicht grün.
- **D Typ-Konformität (Gate):** `wert` wird gegen den Bindungstabellen-`typ` des `feld_id` geprüft
  (cent/int → Ganzzahl, bool → boolean, enum → ∈ `enum_werte`, datum → ISO-8601). Sonst wäre die
  Typbindung nur Doku.

## Entscheidungen (Instructor abgenommen, msg 2377 — alle wie vorgeschlagen)

1. **Meet-Regel je Herkunfts-Achse:** `pruef_tiefe` = Minimum (Kette, klar). Für `herkunft`/`haftung`
   (Kategorien): Meet zweier verschiedener Werte → eigener Wert `konflikt` je Achse (Split-Annunciator),
   ODER ein Vorrang (z.B. haftung: amt > berater > nutzer > system)? Vorschlag: **Konflikt-Markierung**
   (nie auto-versöhnt), Auflösung ist Oberfläche/menschlich.
2. **Ein Store-File je Fall/VZ oder ein File mit VZ-Achse?** Vorschlag: **eine Datei je Fall/VZ**
   (klarer Hash-Raum, ERiC ist VZ-spezifisch).
3. **Snapshots materialisiert speichern oder nur `snapshot_id` + Neuberechnung?** Julius #4 sagt
   „beides". Vorschlag: `felder` materialisiert MIT speichern (ERiC-Bindung + Audit brauchen den
   konkreten Zustand), Gate rechnet zur Kontrolle nach.
4. **`wert` für `cent`:** als JSON-`number` (Ganzzahl Cent) — bestätigen (deckt sich mit Bindungstabelle).
5. **Unsicherheits-Anzeige (Julius #6):** das `[min,max]`-Intervall ist ein DERIVAT über dem Store
   (Engine-Rerun mit/ohne vorlaeufige Werte), kein Store-Feld. Bestätigen, dass es NICHT ins Store-Schema
   gehört (Consumer, nicht Speicher).

Nach Abnahme: Store-Mechanismus (der EINE Schreibpfad `append_event` + `materialisiere`) + Gate.
