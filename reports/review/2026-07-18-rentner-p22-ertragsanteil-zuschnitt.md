# Rentner-Scheibe §22 Ertragsanteil — Stufe-A-Zuschnitt (LIVE Bescheid-Fall)

**Status:** concept-first, KEIN Bau vor OK. LLM-frei. Meine Zone = Tabellen/Params + Deklarations-Unterbau;
dev-1 = §22-Einkünfte-Accessor (einkuenfte_sonstige via gesamt-Ring) + Haut-Scheibe.

## (1) BESTEUERUNGSANTEIL gesetzliche Rente (aa) — EXISTIERT + GÜLTIGKEIT OK
- **Regel `p22_1_leibrente_besteuerungsanteil` EXISTIERT**: input jahresrente + besteuerungsanteil_prozent
  (decimal); steuerpflichtiger_rentenanteil = (besteuerungsanteil_prozent/100) × jahresrente. Der
  Prozentsatz kommt als Input (Lookup in der §2-Integration = dev-1-Accessor, Andockung wie §24a).
- **Kohorten-Tabelle EXISTIERT**: `params/kohorten/rente_besteuerungsanteil_p22.yaml` (Schlüssel = Jahr des
  Rentenbeginns, lebenslang fix).
- **⚠ GÜLTIGKEITS-ZEILE (kritisch geprüft, Zeitschwelle NICHT still):** die Tabelle ist die
  **Wachstumschancengesetz-2023-Fassung** (verlangsamter Anstieg 0,5 %/Jahr, 100 %-Punkt 2058 statt 2040) —
  belegt: 2022=82,0 · **2023=82,5** · 2024=83,0 · 2025=83,5 · 2032=87,0 · 2033=87,5 · **2040=91,0** (NICHT
  100) · 2041=91,5 · **2058=100,0**. Das ist die AKTUELLE Fassung, NICHT die alte 1 %/Jahr-mit-2040=100.
  §22-Quelle estg_p22_2026-07-13 „geltende Fassung 2026" deckt sich. **GÜLTIG.**
- Anker aa: „Jahr des Rentenbeginns" (§ 22 Nr. 1 S. 3 Buchst. a Doppelbuchst. aa), estg_p22_2026-07-13.txt ✓.

## (2) ERTRAGSANTEIL private Leibrente (bb) — GAP (Haupt-Unterbau dieser Runde)
- **KEINE Tabelle, KEINE Regel** (find params *ertragsanteil* = leer; kein p22_1_ertragsanteil in Registry).
  Das ist der benannte Nachtrag aus Nachtrag-A (renten_art-grund).
- **Volle bb-Tabelle aus §22-Quelle erfasst** (Alter bei Rentenbeginn → Ertragsanteil %):
  0–1→59, 2–3→58, … 60–61→22, 62→21, 63→20, 64→19, **65–66→18**, 67→17, 68→16, 69–70→15, … 80→8, … ab 97→1.
- Anker bb: „Bei Beginn der Rente vollendetes Lebensjahr" (§ 22 Nr. 1 S. 3 Buchst. a Doppelbuchst. bb) ✓.

## (3) Param-Struktur (konsistent zur aa-Tabelle)
- aa: `params/kohorten/rente_besteuerungsanteil_p22.yaml` (Jahr→%) — existiert.
- **bb (NEU, meine Zone):** `params/kohorten/rente_ertragsanteil_p22.yaml` (Alter-bei-Rentenbeginn→%,
  volle Tabelle oben). Selbe kohorten-Struktur; Lookup im Accessor (dev-1), wie aa.
- Regel bb: `p22_1_ertragsanteil` (Registry-Nachtrag, dev-1/Instructor) — parallel zu p22_1_leibrente_
  besteuerungsanteil: input jahresrente + ertragsanteil_prozent → steuerpflichtiger = %×jahresrente. Der
  renten_art-Klasse-f (Nachtrag A) routet aa/bb schon.

## (4) Deklarations-Vollständigkeit
- **Existiert:** rentner_jahresrente, rentner_renten_beginn_jahr (aa-Key), rentner_renten_art (aa/bb-Weiche).
- **FEHLT — rentner_alter_bei_rentenbeginn** (int, der bb-LOOKUP-KEY): die Ertragsanteil-Tabelle ist nach
  ALTER (nicht Jahr) geschlüsselt. Ohne dieses Feld ist bb nicht auflösbar. NEU (meine Zone), Anker §22 bb.
- **Rentenfreibetrag-Fixierung (§ 22 Nr. 1 S. 3 a aa S. 4/5):** steuerfreier Teil = jahresrente −
  steuerpflichtiger Anteil, FIXIERT ab dem Jahr NACH Rentenbeginn. Erst-Jahr = berechnet (Accessor);
  Folgejahre = fixierter Euro-Betrag → braucht ggf. Feld rentner_rentenfreibetrag_fixiert (Vorjahres-Wert).
  = **Accessor-/Folgejahr-Entscheid (dev-1)**; MVP Erst-Jahr rechnet der Accessor.
- **§ 9a S. 1 Nr. 3 WK-Pauschbetrag 102 €:** auto vom Accessor abgezogen (einkuenfte_sonstige =
  steuerpflichtiger − 102), KEIN Feld (höhere tatsächliche Renten-WK = seltener GAP).

## (5) Naht zu dev-1 — §22-Einkünfte-Accessor (Signatur-Vorschlag)
```
catala_renten_einkuenfte(s) -> int (EURO, wie Kapital-Accessor):
  art = s["renten_art"]
  if art in (gesetzliche_rente, berufsstaendische_versorgung, private_basisrente):   # aa
      prozent = lookup(params/kohorten/rente_besteuerungsanteil_p22.yaml, s["renten_beginn_jahr"])
  elif art in (private_leibrente, sonstige_leibrente):                               # bb
      prozent = lookup(params/kohorten/rente_ertragsanteil_p22.yaml, s["alter_bei_rentenbeginn"])
  steuerpflichtig = jahresrente * prozent / 100          # decimal, Cent-Schnitt zuletzt
  return max(0, steuerpflichtig - 102)                   # § 9a S.1 Nr.3 WK-Pauschbetrag
  # -> speist einkuenfte_sonstige in catala_gesamt
```
Einheit: EURO (Accessor) / Cent (Store-Naht). Rentenfreibetrag-Fixierung Folgejahr = separater Slot/Backlog.

## Bau-Umfang nach OK (meine Zone)
1. params/kohorten/rente_ertragsanteil_p22.yaml (NEU, volle bb-Tabelle, params-geankert).
2. bindung_rentner.yaml: NEU rentner_alter_bei_rentenbeginn (int, bb-Key, §22-bb-Anker).
3. est_mapping/Drift + Test (bb-Lookup-Tabelle vollständig + Grenzfälle 64→19/65→18, Randalter).
4. Naht-Meldung Signatur+Einheit an dev-1 (§22-Accessor) — analog Kapital-Kontrakt.

## INSTRUCTOR-RULING UMGESETZT + GEBAUT (2026-07-18, K2-Rentenfreibetrag-Fixierung)
Ruling: aa 3-Zweige gegen still-falschen Bescheid (bb ist exakt für alle Jahre, keine Fixierung):
1. renten_beginn_jahr == VZ (Erstjahr): steuerpflichtig = jahresrente × %(Kohorte). Exakt.
2. renten_beginn_jahr < VZ MIT rentner_rentenfreibetrag: steuerpflichtig = jahresrente − rentenfreibetrag. Exakt.
3. renten_beginn_jahr < VZ OHNE rentner_rentenfreibetrag: FAIL-CLOSED (grund rentenfreibetrag_fixierung_offen).
Diese 3-Zweig-Logik lebt im §22-Accessor (dev-1); meine Deklarations-Seite liefert die Felder + Params dafür.

**GEBAUT (meine Zone, alle Gates grün):**
- `params/kohorten/rente_ertragsanteil_p22.yaml` — bb-Tabelle, Alter 0..97 EXPANDIERT (Ranges aufgelöst),
  monoton fallend, Anker-Spots 0→59/64→19/65→18/97→1, §22 Nr.1 S.3 a bb.
- `bindung_rentner.yaml` — 2 NEU-Felder (null-Kz): rentner_alter_bei_rentenbeginn (int, bb-Lookup-Key,
  DERIVED aus E1801701-Datum − Geburtsdatum), rentner_rentenfreibetrag (cent, aa-Folgejahr-Laienfeld aus
  Vorjahres-Bescheid; ohne = Accessor-Zweig-3-fail-closed). §22-Anker beide voll-Länge.
- `params/{2024,2025,2026}/renten_werbungskostenpauschbetrag_p9a.yaml` — §9a S.1 Nr.3 WK-PB 102 €.
  **⚠ ANKER VORAB (deine Auflage): "von den Einnahmen im Sinne des § 22 Nummer 1, 1a und 5: ein Pauschbetrag
  von insgesamt 102 Euro" (estg_p9a_2026-07-09, _normalize-verifiziert) — bitte bestätigen.**
- `tests/test_renten_ertragsanteil_p22.py` — bb-Vollständigkeit + Monotonie + Anker-Spots + §9a-102 (4 Tests).
- Gates: bb-Test+Drift+est_mapping 40/40; breit 361 passed; Drift-Wächter grün (2 Felder feld_id-eindeutig).

**Offen (nicht meine Zone):** p22_1_ertragsanteil-Regel (Registry-Nachtrag dev-1) + §22-Accessor (dev-1,
3-Zweig-aa + bb) — finale Signatur an dev-1 gemeldet. §33b-Werte params: dev-1 bleibt bei inline
(Transkription + Konsistenz-Gate) außer du willst params.

## Zur Abnahme (Rest)
(1) §9a-102-Anker bestätigt? (2) rentner_rentenfreibetrag als Laienfeld (kein Anlage-R-Kz) OK, oder
möchtest du es anders (z.B. Pflicht-Berechnung im Erstjahr statt Folgejahr-Eingabe)? → dann Commit.
