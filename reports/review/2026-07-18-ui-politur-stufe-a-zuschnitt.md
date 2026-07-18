# UI-Politur — Stufe-A-Recon (dev-1 → Instructor → Julius)

**Concept-first, KEIN Bau.** Basis für Julius' Optik-Steuerung. IST-Zustand der Haut ehrlich +
Lab-Gestaltung (6b0b165 / uiux-lab-haiku.md) auf konkrete Haut-Änderungen übersetzt + Prioritäten.

## 1. IST-Zustand der Haut (ehrlich — bewusst roher Prototyp)
Vanilla-JS, kein Build, ~460 Zeilen (app.js 211 / index.html 37 / style.css 60 + Desktop-Graph).
Redet NUR über die HTTP-API, keine Steuerlogik im Frontend. Was DA ist:
- **Wegpunkt-Fluss** (mobil): eine Frage nach der anderen (fragetext_laie + hilfe + Eingabe +
  Bestätigen + „Warum?" → Anker). Matcht Lab-Dim 3 (keine 200-Fragen-Wand) — SOLIDE Basis.
- **Herkunfts-Badge**: „KI"/„✓" (schimmernd/solide) in der „schon beantwortet"-Liste — Basis-Stufe.
- **Ring** (#ring): zeigt den ANTEIL bestätigter Felder (Fortschritt), NICHT die Bescheid-Spanne.
- **„Warum?"** → Zitatanker aus der Bindung. Gut (Provenance-Ansatz da).
- **Desktop-Graph** (graph.html): Abhängigkeitsgraph als Zusatzansicht — matcht Lab-Dim 3 Desktop.
- **Ergebnis**: Zahl ODER ehrlicher Guard-Text (13 grund-Texte, inkl. der neuen K2-Sperren).

**Die 3 kritischen Lücken für „benutzbar":**
- ⚠ **KEINE SCHEIBEN-WAHL: app.js:28 hardcodet `scheibe:"ep"`.** Der ganze Rechen-Ring (gesamt =
  §19/§21/§20, rentner_gesamt = §22/§33b) ist UI-UNERREICHBAR — die App startet immer nur den
  Entfernungspauschale-Fall. Das ist die #1-Lücke: ohne Einstieg sieht der Nutzer den fertigen
  Rechen-Ring nie.
- **bool-Prefill** (app.js:104-108): fix „Ja"/„Nein", liest `beispielwert` NICHT → alle bool-Fragen
  defaulten „Ja" (dev-2s #4; die 4 kein_-Flags haben jetzt beispielwert=False, greift erst mit dem Fix).
- **Ring zeigt Feld-Anteil, nicht die schrumpfende Bescheid-Spanne** (Lab-Dim 4, die Signatur-Optik).

## 2. Lab (5 Dimensionen) → konkrete Haut-Änderungen
| Lab-Dimension | Haut-Änderung | Server liefert schon? |
|---|---|---|
| 1 Herkunft zum Anfassen | Badge antippbar → Kette Beleg→Extraktion→Vorschlag→§ im Bestätigungsmoment | herkunft_vektor/justification da (Teil) |
| 2 Bestätigen mit Unsicherheits-Gefühl | Hold-to-confirm bei KI-Konfidenz (schimmernd), 1-Tipp bei Beleg | herkunft_badge da; Konfidenz-Schwelle = Design |
| 3 Navigation ohne Wand | STEHT (Wegpunkt mobil + Desktop-Graph) | ja |
| 4 Bescheid als schrumpfender Ring | Ring-Geometrie = [min,max]→Punkt statt Feld-Anteil | intervall.min_cent/max_cent da |
| 5 Chat als Berater daneben | Chat-Slot neben Bestätigen (gleiche Größe), situativ, erklärt nie füllt | POST /chat→501 (KI-Sperre fest) da |

## 3. Scheiben-Wahl-UX (Instructor-Punkt 3) — Julius-Entscheid
Der Nutzer muss seinen Typ wählen. Verfügbare user-facing Scheiben:
- **gesamt** — Arbeitnehmer / Vermieter / Kapital (§19+§21+§20, EIN Ring).
- **rentner_gesamt** — Rentner (§22+§33b).
- (an_gesamt = schmaler AN-only-MVP, wohl von gesamt abgelöst; ep/n_vor_gwg = WK-Unterscheiben, kein Einstieg.)

**Optionen für den Einstieg (Start-Screen):**
- (A) 2 Kacheln: „Ich arbeite / vermiete / habe Kapital" → gesamt · „Ich bin Rentner" → rentner_gesamt.
- (B) 3 Kacheln Angestellter/Vermieter/Rentner (Angestellter+Vermieter beide → gesamt, nur Label-Diff).
- (C) Eine Frage „Was trifft zu?" (Mehrfach) → Scheibe abgeleitet.
Empfehlung: **(A)** — kleinster ehrlicher Einstieg, matcht die 2 echten Ring-Typen. Label-Verfeinerung später.

## 4. bool-Prefill-Fix (dev-2s #4, meine Zone)
app.js bool-select: `option.selected = (String(o.value) === String(q.beispielwert))` beim Rendern →
beispielwert wird Vorauswahl (statt fix „Ja"). Gilt für ALLE bool-Fragen. dev-2 hat die 4 kein_-Flags
beispielwert True→False gesetzt (800dca0) → greifen automatisch = Normalfall „Nein". Klein, isoliert, P0.

## 5. Prioritäten + Häppchen-Reihenfolge (Vorschlag)
- **P0 — „überhaupt benutzbar"**: (a) Scheiben-Wahl-Start-Screen (Rechen-Ring erreichbar!) + (b) bool-Prefill.
  Ohne (a) ist der ganze committete Rechen-Ring UI-tot. Kleinstes Häppchen, größter Hebel.
- **P1 — Signatur-Optik**: (c) schrumpfender Bescheid-Ring (Lab-Dim 4) + (d) Herkunft-Kette antippbar (Dim 1).
- **P2 — Vertrauen/Erklärung**: (e) Chat-Slot-Platzhalter (Dim 5, KI-Sperre fest) + (f) Hold-to-confirm (Dim 2).
- **P3 — später**: Gesten-Grammatik, ELSTER-Fehler-Pädagogik, Ambient-Signale (Lab-Spannungen, Design-Raum).

## Julius-Entscheide (die Optik-Steuerung)
1. **Mobile-first vs Desktop-first** (der eine Lab-Julius-Entscheid): Lab-Empfehlung = responsive mit
   Mobile-Wegpunkt-Primärpfad + Desktop-Graph als Zusatz. Bestätigen?
2. **Scheiben-Wahl-Einstieg**: Option A/B/C (Empfehlung A)?
3. **Prioritäts-Reihenfolge**: P0→P1→P2 wie vorgeschlagen, oder andere Gewichtung (z.B. Ring-Optik vor Chat)?
4. Umfang: „benutzbar-MVP" (P0+P1) jetzt, Rest Backlog — oder volle Lab-Vision in einem Rutsch?

Zuschnitt zu Instructor → Julius steuert Optik → dann Bau (P0 zuerst, kleinste ehrliche Häppchen).
