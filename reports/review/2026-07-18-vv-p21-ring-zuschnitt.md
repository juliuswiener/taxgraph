# V+V §21 Bescheid-Ring — Stufe-A-Zuschnitt (Deklarations-Seite)

**Status:** Recon + Zuschnitt zur Instructor-Abnahme, concept-first, KEIN Bau. LLM-frei.
**Kern-Befund: die Deklarations-Seite existiert GRÖSSTENTEILS schon (Scheibe 3); die eigentliche Lücke
ist die Ring-Verdrahtung (dev-1), nicht neue Bindungen.**

## 1. §21-Freeze / Gültigkeit
`estg_p21_2026-07-13.txt`, **fassung „geltende Fassung 2026"** — gültig, kein neuer Freeze nötig. (Kein
Änderungsgesetz-Vorbehalt in der Meta; Stufe-A-Gültigkeits-Zeile: OK.)

## 2. Registry-Scope: p21 IST eine echte Regel (kein Pseudoregel) — aber NICHT verdrahtet
`p21_vermietung_einkuenfte` (rules.yaml): scope **VermietungEinkuenfte**, geltungsbedingung
`einkuenfte_ueberschuss_einnahmen_ueber_wk`, inputs `{einnahmen, gebaeude_afa, schuldzinsen,
erhaltungsaufwand, sonstige_werbungskosten}`, output `vermietung_einkuenfte`. **Aber:** der golden-Runner
ruft VermietungEinkuenfte NICHT — `catala_gesamt` nimmt `einkuenfte_vermietung` als bereits FERTIGEN
money-Input (§ 2-Integration). **→ Es fehlt der §21-Einkünfte-Accessor** (Einnahmen − Σ WK →
`einkuenfte_vermietung`). Das ist reine Arithmetik (Überschussrechnung) → **dev-1 baut einen Python-
Accessor `_vermietung_einkuenfte` (analog `_vorsorge_abzug`)**, kein Catala-Neubau/Judge-Runde nötig.
Kein Pseudoregel-Scope nötig — die Bindungen docken an die ECHTE Regel p21_vermietung_einkuenfte.

## 3. Deklarations-Seite: EXISTIERT (Scheibe 3, bindung_kap_vv_familie)
Die 5 §21-Inputs sind schon gebunden: `vv_einnahmen` (→ einnahmen, Kz **E0700201 „Mieteinnahmen"**, 1:1
verifiziert), `vv_gebaeude_afa` (→ gebaeude_afa), `vv_schuldzinsen`, `vv_erhaltungsaufwand`,
`vv_sonstige_wk`. **Kein neues Bindungs-Feld nötig** für den Kern-Ring.

**WK-Kz = weiterhin GAP/dokumentiert (mein Anlage-V-Ruling bestätigt sich):** die AfA hat KEIN sauberes
Einzel-Kz — sie ist Zuordnungsart-verzweigt (E0703302/E0703304 [Direkt], E0703417/E0703419 [Verhaelt],
je Art-Flag + Betrag, Mehrzeilen je Objekt). Schuldzinsen/Erhaltung analog. → est_mapping führt sie als
`DOKUMENTIERT_AGGREGAT` E0703838 (Σ, dokumentiert-nicht-deklariert). Die Zuordnungsart-Einzel-Kz +
Multi-Objekt = **benannter Nachtrag** (braucht `anzahl_vermietungsobjekte` + Zuordnungs-Modell).

## 4. est_mapping-Klasse: KEINE neue nötig
V+V-Einkünfte = Einnahmen − Σ WK ist die RING-Rechnung (dev-1-Accessor), NICHT est_mapping. est_mapping
mappt nur die Input-Felder: `vv_einnahmen` 1:1 (E0700201), die 4 WK → `DOKUMENTIERT_AGGREGAT` (E0703838).
Beides existiert. **Keine neue Transform-Klasse.**

## 5. K2-Guard: kein_vuv-Inversion (die eine echte neue Sache meiner Zone)
Heute: `kein_vuv=true` = Abwesenheits-Flag (reiner-AN-Fall, keine V+V). Für den V+V-Ring MUSS
`kein_vuv=false` (echte V+V-Einkünfte). **Ring-Architektur-Frage:** der reine-AN-`an_gesamt`-Ring sperrt
bei `kein_vuv=false` (aggregierte zvE). Ein V+V-Fall braucht also entweder (A) eine **neue Scheibe
`vv_gesamt`** (Vermieter: vv_* + optional Arbeitslohn → catala_gesamt mit einkuenfte_vermietung) oder
(B) eine **Erweiterung von an_gesamt**, die bei `kein_vuv=false` den V+V-Zweig statt der Sperre nimmt.
Empfehlung **(A) neue Scheibe vv_gesamt** — sauberer Zuschnitt, an_gesamt bleibt der reine-AN-MVP;
vv_gesamt reused die vv_*-Bindungen (feld_id global eindeutig, Drift-Wächter A5).

## Zusammenfassung / zur Abnahme
- **Meine Zone (Deklarations-Seite):** minimal — die Felder existieren. NEU nur: (a) ggf. eine Scheibe
  `vv_gesamt` (Scope-Binding an p21_vermietung_einkuenfte + catala_gesamt, Reuse vv_*), (b) die
  kein_vuv=false-Semantik dokumentieren, (c) Drift-Wächter/Gate.
- **dev-1-Zone (Ring):** der `_vermietung_einkuenfte`-Accessor (Einnahmen − Σ WK) + Verdrahtung in den
  Bescheid-Ring (einkuenfte_vermietung → catala_gesamt).
- **Nachtrag:** Zuordnungsart-Einzel-WK-Kz + Multi-Objekt (Anlage-V-Detail).

**Entscheide:** (1) Scheibe-Architektur A (neue vv_gesamt) vs B (an_gesamt-Erweiterung)? (2) baue ich
die vv_gesamt-Scheibe (Scope-Binding) + K2-kein_vuv, während dev-1 den §21-Accessor macht? (3)
Zuordnungsart-WK-Kz als benannter Nachtrag ok?
