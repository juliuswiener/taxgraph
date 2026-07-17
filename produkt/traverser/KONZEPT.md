# Traverser — Konzept-Skizze (K1, Task #11, Paket A)

**Zone:** `produkt/traverser/` (neu, additiv, **NULL LLM**, rein deterministisch). **Status:**
Konzept-Skizze zur Instructor-Abnahme VOR dem Bau. Das dritte Kern-Stück: der Regel-Graph in **zwei
Leserichtungen** — rückwärts = Interview, vorwärts = Beweis. Bindet die drei vorhandenen Säulen
(Bindungstabelle, Store, Unsicherheits-Derivat) zusammen und ist die **einzige** Schnittstelle, die
Paket B (Haut) liest.

## Datengrundlage (KEINE Catala-Introspektion)

Der Traverser ist reine Ableitung über: `pipeline/produktion/rules.yaml` (geltungsbedingungen,
signature, Günstiger-Marker), `produkt/bindung/` (feld_id ↔ slot ↔ geltungsbedingung, typ, anker_ref,
bereich), `produkt/store/` (aktueller Antwortstand), `produkt/unsicherheit/intervall.py` (Beiträge für
die Frage-Reihenfolge). Er introspiziert **nicht** die Catala-Scopes — das ist die zentrale Grenze
(s.u.), macht ihn aber deterministisch und leichtgewichtig.

## (a) RÜCKWÄRTS — Interview

**Relevanz je Regel** (aus den Geltungsbedingungen):
- Eine Geltungsbedingung, die als **askable bool** gebunden ist (z.B. `dhf_beruflich_veranlasst`), ist
  eine **Gating-Frage**. Antwort `false` (bestätigt) → die Regel ist **ausgeschlossen**, ihre Slots
  werden irrelevant.
- Regel-Status: `ausgeschlossen` (eine Gating-Bedingung ist bestätigt-false) · `relevant` (alle Gates
  bestätigt-true) · `unentschieden` (mind. ein Gate offen/vorlaeufig).
- `askable:false`-Geltungsbedingungen (reine Annahmen, z.B. `voller_abzug_100_prozent`) sind **kein**
  Gate (immer angenommen).

**Nächste Frage(n):** unbeantwortete askable Felder relevanter/unentschiedener Regeln. Reihenfolge:
1. **Gating-Bedingungen zuerst** (sie können ganze Regeln streichen → billigste Information).
2. dann die Slot-Felder, **sortiert nach Unsicherheits-Beitrag** aus `intervall.py` (das spannungs-
   stärkste zuerst = Steuer-at-Risk). Fällt die Engine aus, stabiler Fallback (feld_id-sortiert).

**Günstigerprüfungs-Zweige (Lab-Fund: naives Lazy bricht):** bei einem Günstiger-Knoten
(z.B. § 31 Kindergeld↔Freibeträge, § 33a Unterhalt-Schonbetrag, EP-ÖPNV-max, KAP-Topf) hängt das
Ergebnis von `max`/`min` **beider** Zweige ab → es werden **ALLE Zweig-Inputs** gefragt, NIE ein Zweig
anhand eines vorläufigen Siegers weggeschnitten. Erkennung aus dem **deklarierten Günstiger-Marker**
in rules.yaml (nicht aus Catala).

**Grenze (benannt):** Erreichbarkeit stammt NUR aus den deklarierten Geltungsbedingungen. Interne
Catala-Verzweigungen, die nicht als Geltungsbedingung materialisiert sind, sind unsichtbar — der
Traverser fragt dann konservativ (nicht wegschneiden). Das ist der Preis für „keine Catala-
Introspektion".

## (b) VORWÄRTS — Trace / Justification

Rekursives **Justification-Objekt** je Feld, deterministisch aus Store + Bindungstabelle:

```
Justification(feld_id) = {
  feld_id, wert, zustand, herkunft,                     # aus dem Store-Event
  event_id, signal,                                     # Zwei-Signal-Beleg
  regel_id, signatur_slot | geltungsbedingung,          # aus der Bindungstabelle
  anker_ref: {quelle, zitatanker, datei},               # der Gesetzes-Anker (das "warum")
  basis_snapshot: <snapshot_id>                          # snapshot-gebunden
}
```

**Ergebnis-Trace** (die „folge der Kante"-Geste als Datenstruktur): `Ergebnis → beteiligte Regeln
(deren Slots belegt sind) → je Slot die Feld-Justifications → anker_ref`. Auf **Regel/Slot/Feld/Event-
Ebene exakt** und ohne Engine ableitbar.

**Grenze (benannt):** die **per-Cent-Attribution** („welcher Cent kommt aus welcher Regel") braucht den
**Catala-Herleitungs-Baum** — der ist NICHT als Modul vorhanden (`pipeline/provenance.py` ist
LLM-Rollen-Provenance, nicht Steuer-Attribution; ein Catala-Trace-Prototyp existiert, unverdrahtet).
Der Traverser-Trace ist damit Regel/Slot/Feld-genau; der per-Cent-Durchgriff ist ein benannter
Nachtrag (Catala-Herleitungs-Baum-Anbindung), kein still gelöstes Problem.

## (c) API-VERTRAG — Paket-B-Schnittstelle

Der Traverser ist **read-only** und die **einzige** Sicht, die die Haut braucht:
```
naechste_fragen(store) -> [feld_id, ...]           # geordnete Interview-Queue
relevanz(store)        -> {regel_id: status}       # ausgeschlossen|relevant|unentschieden
justification(store, feld_id) -> Justification     # Vorwärts-Trace je Feld
trace_ergebnis(store)  -> {regel_id: [Justification, ...]}   # beteiligte Regeln
```
**Vertrag:** Paket B (Haut + LLM-Chat) konsumiert **ausschließlich** Traverser (read) +
Bindungstabelle (Metadaten) + Store — und schreibt **NUR** über `store.append_event` (der EINE
Schreibpfad; LLM-Chat schreibt qua Auflage A nur `vorlaeufig`). **Kein zweiter Schreibpfad**, keine
Regel-/Registry-Berührung durch die Haut. Das ist die kollisionsfreie Naht Kern↔Haut.

## Determinismus / NULL LLM

Reine Ableitung über rules.yaml + Bindungstabelle + Store; feste Sortierung; snapshot-gebunden. Keine
Heuristik, kein LLM. Das Interview ist berechnet, nicht kuratiert (Lab-K1).

## Offene Punkte zur Abnahme (Instructor-Entscheid)

1. **Relevanz-Semantik:** Gating = askable bool-Geltungsbedingungen; `false`→ausgeschlossen,
   offen/vorlaeufig→unentschieden; askable:false-Bedingungen sind kein Gate. OK?
2. **Günstiger-Erkennung:** aus einem rules.yaml-Marker. Falls kein sauberes Einzelfeld existiert,
   führe ich eine **explizite, source-verankerte Günstiger-Regel-Liste** im Traverser (benannt, kein
   Raten). Welche Variante bevorzugst du?
3. **Per-Cent-Grenze:** Trace auf Regel/Slot/Feld-Ebene jetzt; per-Cent via Catala-Herleitungs-Baum als
   benannter Nachtrag. OK?
4. **Frage-Reihenfolge:** Gating zuerst, dann Slots nach `intervall.py`-Beitrag (Engine-Aufruf); ohne
   Engine stabiler feld_id-Fallback. OK?
5. **Scope Erst-Ausbau:** Traverser gegen die vorhandene Scheibe N+VOR+GWG (die 6 Regeln); Gate mit
   Golden-artigen Interview-/Trace-Assertions + Negativtests (Günstiger-nicht-geschnitten,
   ausgeschlossene-Regel-fragt-nicht, Trace-Anker = Bindungstabelle-Anker). OK?

Nach Abnahme: `produkt/traverser/traverser.py` (relevanz + naechste_fragen + justification/trace) +
`tests/test_traverser.py` (deterministisch, Negativtests) + API-Vertrag-Doku.
