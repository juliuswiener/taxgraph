# Unsicherheits-Derivat — Konzept-Skizze (Task #11, Paket A, Julius #6)

**Zone:** `produkt/unsicherheit/` (neu, additiv, **NULL LLM**, rein deterministisch). **Status:**
Konzept-Skizze zur Instructor-Abnahme VOR dem Bau (wie Schema-first). Kein Store-Feld — der
`[min,max]`-Bescheid ist ein **DERIVAT** über dem Store (Julius #5: Store speichert Fakten, keine
Ableitungen).

## Was das Derivat liefert

1. **`[min,max]`-Bescheid-Intervall:** die Spanne, in der die festzusetzende Steuer liegt, solange
   unsichere Eingaben offen sind. Verengt sich, wenn Felder bestätigt werden (Julius #6:
   „schrumpfender Bescheid").
2. **Beitrag je Feld zur Spannen-Verengung:** wie stark jedes unsichere Feld die Spanne aufspannt →
   Grundlage für **Frage-Reihenfolge** (das spannungsstärkste zuerst) und **Steuer-at-Risk**.

## Unsicherheits-Quelle (aus dem Store)

Ein askable Feld ist **unsicher**, wenn sein aktives Event `zustand=vorlaeufig` trägt ODER es
**offen** ist (kein Event). `zustand=bestaetigt`-Felder sind **fix** (gehen als Konstante in jeden
Rerun). Der Store-Snapshot (materialisiert) ist der Eingangszustand.

## Der Rechenkern als reine Funktion

`bescheid(werte) → steuer_cent` existiert bereits: `golden/runner.py` liefert reine, deterministische
`catala_*(sachverhalt) → int` (z.B. `catala_est`, `catala_gesamt`, `catala_entfernungspauschale`).
Brücke **Store→Engine:** Store hält Werte je `feld_id`; die Engine erwartet `sachverhalt`-Keys
(= `signatur_slot`). Die **Bindungstabelle** (`quelle.signatur_slot`) ist genau diese Übersetzung
`feld_id → slot`. Summanden-Slots werden vor dem Rerun addiert (Summen-Konvention).

## Extremwert-Bestimmung je Typ

| typ | Extremwerte | Anzahl |
|---|---|---|
| bool | `{false, true}` | 2 |
| enum | alle `enum_werte` | \|enum\| |
| cent / int | `[bereich.min, bereich.max]` | 2 (Grenzen) |
| datum / text | **keine numerische Achse** → als Nicht-Achse ausgenommen (benannt, nie still) | 0 |

**cent/int-Grenzen — Vorschlag:** ein **neues optionales Bindungstabellen-Feld `bereich: {min, max}`**
(deterministisch, amtlich verankerbar; z.B. `arbeitstage 0..366`, `monate 0..12`, `entfernung_km 0..`).
Fehlt `bereich` bei einem cent/int-Feld, ist die Achse **unbounded** → die betroffene Intervallseite
wird ehrlich als **offen markiert** (`intervall.min_offen`/`max_offen = true`), NIE still auf 0 gesetzt
(Falsch-Grün-Analog).

## Kombinatorik-Deckel (der Kern der Skizze)

Voller kartesischer Raum = Π |Extremwerte_i| → explodiert. Keine Monotonie-Annahme möglich
(Günstigerprüfung/Caps/Freigrenzen sind nicht-monoton), also kann One-at-a-time das wahre Intervall
NICHT garantieren. Vorschlag in zwei Stufen:

1. **Beitrag je Feld (One-at-a-time), O(Σ|Extremwerte_i|) Reruns:** halte alle anderen unsicheren
   Felder auf ihrem aktuellen Wert (vorlaeufig-Vorschlag, sonst `bereich`-Mittelpunkt), variiere EIN
   Feld über seine Extremwerte, miss die Steuer-Spanne = **Beitrag** des Felds. Liefert die Rangfolge
   (Frage-Reihenfolge) billig. **Ehrlich notiert:** One-at-a-time-Ranking kann Interaktions-Effekte
   (nicht-monotone Wechselwirkungen zweier Felder) **unterschätzen** — als Ranking-Heuristik akzeptiert;
   das gemeldete Intervall selbst bleibt durch das `gedeckelt`-Flag ehrlich (Stufe 2).

   **Nicht-fixierbare Felder:** ein unbounded Feld (cent/int ohne `bereich`) OHNE vorlaeufig-Vorschlag
   hat keinen Fixierwert → es ist **nicht fixierbar** und zählt in BEIDEN Stufen automatisch als
   **offene Achse** (die betroffene Intervallseite wird offen markiert). Es wird NIE ein Ersatzwert
   (0 o.ä.) erfunden.
2. **Gesamt-Intervall (gedeckelter Kartesischer über die Top-K-Treiber):** sortiere die Felder nach
   Beitrag; nimm die **Top-K**, deren kombinierter Raum ≤ **Cap** (Vorschlag 256 Reruns); rechne das
   EXAKTE `[min,max]` über deren Kreuzprodukt (die restlichen unsicheren Felder auf ihrem aktuellen
   Wert fixiert). Das gemeldete Intervall ist damit **exakt bzgl. der K stärksten Treiber** und trägt
   ein **`gedeckelt`-Flag** + Liste der nicht berücksichtigten Rest-Felder, wenn K < n. Nie als
   „das volle, exakte Intervall" ausgeben, wenn gedeckelt (Falsch-Grün-Sperre, analog zur checkESt-
   Trunkierung).

Ergebnis-Skizze:
```
{ intervall: {min_cent, max_cent, min_offen, max_offen, gedeckelt, exakt_bzgl_top_k: K,
              rest_felder: [feld_id, ...]},
  beitraege: [ {feld_id, spanne_cent, min_cent, max_cent}, ... ]  # absteigend sortiert
  basis_snapshot: <snapshot_id> }
```
Das Derivat bindet an den `snapshot_id` (reproduzierbar; gleicher Zustand → gleiches Intervall).

## Determinismus / NULL LLM

Feste Extremwert-Mengen, feste Rerun-Reihenfolge (feld_id-sortiert), content-adressierbar über den
Snapshot. Keine Heuristik, kein LLM. Reruns sind reine `catala_*`-Aufrufe (Hinweis: brauchen das
`_catala`-Symlink + opam-Env; Rerun-Kosten real → der Cap ist auch ein Laufzeit-Deckel).

## Offene Punkte zur Abnahme (Instructor-Entscheid)

1. **`bereich:{min,max}` als neues optionales Bindungstabellen-Feld** (Prärequisit für cent/int-Achsen)
   — OK? Grenzen amtlich verankert (arbeitstage 0..366 etc.); Gate prüft min≤max. Ich würde die
   Bindungstabellen-Scheibe N+VOR+GWG um `bereich` ergänzen (separater kleiner Nachzug).
2. **Cap = 256 Reruns + Top-K-Strategie** (exakt über die stärksten Treiber, Rest geflaggt) — OK, oder
   anderer Cap/andere Strategie?
3. **Engine = `golden/runner.py` `catala_*`** (reine cent-Funktionen), feld_id→slot via Bindungstabelle
   — bestätigen, dass das die vorgesehene Rechen-Engine ist (kein neuer Rechenpfad).
4. **Fixierung der „anderen" Felder beim One-at-a-time:** vorlaeufig-Vorschlagswert, sonst
   `bereich`-Mittelpunkt (bei offenem Feld ohne Vorschlag). OK? (Alternative: 0 — verzerrt aber die
   Beiträge.)
5. **datum/text als Nicht-Achse ausgenommen** — OK?

Nach Abnahme: `bereich`-Nachzug in der Bindungstabelle (+ Gate) → dann `produkt/unsicherheit/intervall.py`
(One-at-a-time + gedeckelter Top-K-Kartesischer) + `tests/test_unsicherheit.py` (deterministisch,
Negativtests: gedeckelt-Flag greift, unbounded-Achse markiert, Beitrag-Monotonie beim Bestätigen).
