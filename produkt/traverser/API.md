# Paket-B-Naht — API-Vertrag (Kern ↔ Haut)

**Task #11.** Definiert die EINZIGE Schnittstelle, die Paket B (Privat-Oberfläche + LLM-Chat) gegen den
Kern nutzen darf. Ziel: kollisionsfrei (Paket A/B berühren sich nur hier) und fail-closed (kein zweiter
Schreibpfad, keine KI-Werte in der Summe).

## Was die Haut LESEN darf (read-only)

**Traverser** (`produkt/traverser/traverser.py`) — der Regel-Graph in zwei Richtungen:

| Funktion | liefert |
|---|---|
| `relevanz(store, bindung)` | je Regel: `{status: ausgeschlossen\|relevant\|unentschieden, gates_offen, annahmen_offen}` |
| `naechste_fragen(store, bindung, beitrag=None)` | geordnete Interview-Queue (Gating zuerst, dann Unsicherheits-Beitrag aus `intervall.py`) |
| `justification(store, feld_id, bindung)` | Vorwärts-Trace eines Felds: `wert/zustand/herkunft/event_id/signal/regel_id/slot\|bedingung/anker_ref` |
| `trace_ergebnis(store, bindung, snapshot_id)` | beteiligte Regeln → je Regel die Feld-Justifications (snapshot-gebunden) |

**Bindungstabelle** (`produkt/bindung/`) — Metadaten je `feld_id`: `typ, einheit, fragetext_laie,
hilfe_kurz, beispielwert, enum_werte, bereich, anker_ref`. Reine Anzeige-/Validierungs-Metadaten.

**Store-Snapshot** (`produkt/store/store.materialisiere`) — der aktuelle Antwortstand + `snapshot_id`.

**Unsicherheits-Derivat** (`produkt/unsicherheit/intervall.py`) — `[min,max]`-Bescheid + Beitrag je Feld
(für die Frage-Reihenfolge und die Steuer-at-Risk-Anzeige).

## Was die Haut SCHREIBEN darf — genau EIN Pfad

**`store.append_event(store, feld_id=…, wert=…, zustand=…, herkunft=…, schreiber=…, signal=…, ersetzt=…)`**
ist der EINZIGE Schreibpfad. Es gibt keine zweite Schreib-Implementierung.

- Der **LLM-Chat** schreibt qua Store-Auflage A ausschließlich **`vorlaeufig`**-Events mit
  `schreiber="llm:…"`, `herkunft.herkunft="llm_vorschlag"`, `signal_2=null`. Jeder Versuch, über eine
  gefälschte Herkunft einen `bestaetigt`-Wert zu setzen, wird hart abgewiesen (`ValueError`).
- **Bestätigen** (Zwei-Signal) ist ein `bestaetigt`-Event mit `signal_2` (der menschliche Klick neben
  dem Beleg) — geschrieben von der UI (`schreiber="ui:…"`), NICHT vom Chat.
- **Korrektur** eines Felds: neues Event mit `ersetzt=<aktives event_id>` (Auflage B).

## Was die Haut NICHT tut

- **Kein** direkter Schreibzugriff auf `rules.yaml`, `item_registry`, `params`, `sources` — der Kern ist
  für die Haut vollständig read-only außer über `append_event`.
- **Keine** eigene Steuerberechnung / Regel-Interpretation — die Zahl kommt ausschließlich aus dem Kern
  (Engine über `intervall.py`/Golden-Runner), nie aus der Haut oder dem LLM.
- **Keine** zweite Wahrheits-Quelle — der Store ist die Wahrheit; alles andere ist Ableitung.

## Fail-closed-Garantien (mechanisch, nicht per Bitte)

1. Ein KI-Wert (`vorlaeufig`) kann strukturell nicht in eine festzusetzende Summe fließen
   (Meet über den Input-Kegel, `store.meet_zustand`).
2. `bestaetigt` erfordert `signal_2` (Zwei-Signal) — Schema + `append_event`.
3. Ein `llm:`-Schreiber ist an `llm_vorschlag`/`vorlaeufig` gekoppelt (Store-Auflage A).
4. Höchstens ein aktives Event je `feld_id`; Überschreiben nur via gültiges `ersetzt` (Auflage B).
5. ELSTER-Befund bindet an den `snapshot_id`; `plausibel` mit `gekappt_verdacht=true` ist nie grün
   (Trunkierungs-Sperre, Auflage C).

## Grenzen (ehrlich benannt)

- **Erreichbarkeit** nur aus deklarierten Geltungsbedingungen — interne Catala-Zweige sind unsichtbar
  (Traverser fragt dann konservativ, schneidet nicht).
- **Per-Cent-Attribution** im Vorwärts-Trace ist Regel/Slot/Feld/Event-genau; der Durchgriff „welcher
  Cent aus welcher Regel" braucht den Catala-Herleitungs-Baum (eigener Baustein, noch nicht verdrahtet).
