# Mitunternehmer §15 Abs.1 Nr.2 — Deklarations-Design-Lock (#2-Front) — dev-2, 2026-07-19

Read-only Materialisierungs-Plan für p15_1_2 MitunternehmerEinkuenfte (Instructor-Follow-on nach #2-Wahl).
Enthält catala_a-Formel + BOUNDARY-ANALYSE (p10d_2-Lehre: faithful=True ≠ Correctness) + Byte-gleich-Plan +
4-Feld-Binding-Spec. Alle Source-Werte gegen gefrorene estg_p15_2026-07-14.txt / estg_p15a_2026-07-14.txt
verankert (Gedächtnis-Verbot gelebt). KEIN Write appliziert.

## 1. catala_a-Formel (Snapshot verified_bedingt, faithful=True)

```catala
declaration scope MitunternehmerEinkuenfte:
  input gewinnanteil content money
  input verguetung_taetigkeit content money
  input verguetung_darlehen content money
  input verguetung_ueberlassung content money
  output einkuenfte_mitunternehmer content money

scope MitunternehmerEinkuenfte:
  definition einkuenfte_mitunternehmer equals
    gewinnanteil + verguetung_taetigkeit + verguetung_darlehen + verguetung_ueberlassung
```
catala_a_sha256 = `598768bc1518ad8c38999f098f4f92965c3787700ab51fa1eae344a396b1bd44` (byte-gleich-Ziel).
Struktur: **reine 4-Summanden-Addition**, KEINE Caps/Floors/Schwellen/Verzweigungen.

## 2. BOUNDARY-ANALYSE ⚠

### 2.1 Struktur-Risiko: NIEDRIG (aber nicht null)
Die Formel kann strukturell nur „falsch summieren" (falsche/fehlende Komponente, Vorzeichen) — NICHT
„falsch cappen" wie p10d_2 (dort war der min(GdE)-Cap der Fehler). Kein min/max/if → keine p10d_2-Klasse-Falle.

### 2.2 Tatbestand-Vollständigkeit: ✓ EXAKT
§15 Abs.1 S.1 Nr.2 S.1 (Source): „die **Gewinnanteile** der Gesellschafter einer OHG, KG und anderen
Gesellschaft, bei der der Gesellschafter als Mitunternehmer anzusehen ist, und die **Vergütungen**, die der
Gesellschafter von der Gesellschaft für seine **Tätigkeit im Dienst** der Gesellschaft oder für die **Hingabe
von Darlehen** oder für die **Überlassung von Wirtschaftsgütern** bezogen hat." → die 4 Formel-Komponenten
mappen 1:1, additiv korrekt (Sondervergütungen sind Teil der gewerblichen Einkünfte, hinzugerechnet). Keine
fehlende/überschüssige Komponente.

### 2.3 ⚠⚠ HAUPT-RISIKO: negativer gewinnanteil + §15a-Wechselwirkung = LATENTER UNDER-TAX
- §15 Abs.3 S.2 (Source) bestätigt: gewerbliche Nr.2-Einkünfte können „**positiv oder negativ**" sein →
  gewinnanteil KANN negativ sein (Verlust-Mitunternehmeranteil).
- §15a Abs.1 S.1 (Source): „Der einem **Kommanditisten** zuzurechnende Anteil am **Verlust** der KG darf
  weder mit anderen Einkünften aus Gewerbebetrieb noch mit Einkünften aus anderen Einkunftsarten
  **ausgeglichen** werden, soweit ein **negatives Kapitalkonto** entsteht oder sich erhöht; er darf insoweit
  auch nicht nach §10d abgezogen werden."
- **DIE FORMEL SUMMIERT gewinnanteil ROH — KEIN §15a-Guard.** Für einen Kommanditisten mit Verlust >
  Kapitalkonto zöge die Formel den vollen Verlust in einkuenfte_mitunternehmer → via einkuenfte_gewinn in die
  GdE → über-verrechneter Verlust → **UNDER-TAX**. Exakt die p10d_2-Analogie: fehlende Beschränkung macht die
  Formel zu großzügig. `faithful=True` heißt nur „bildet den Nr.2-Grund­tatbestand korrekt ab", NICHT „behandelt §15a".
- **MITIGATION (Instructor-Entscheidung, Empfehlung A):**
  - **(A) Input-Semantik-Definition** [EMPFOHLEN]: `gewinnanteil` = der bereits **§15a-beschränkte,
    ausgleichsfähige** Anteil (aus dem gesonderten Feststellungsbescheid — §15a Abs.4 stellt den verrechenbaren
    Verlust jährlich fest; ausgleichsfähig = Anteil − verrechenbar). Nutzer/Bescheid liefert den
    ausgleichsfähigen Betrag. §15a bleibt separate Zukunfts-Regel. → konsistent mit gewst_messbetrag/
    verlustvortrag_bestand (FA-festgestellte Inputs). Pflicht: expliziter Feld-Hilfe-Text + Boundary-Seed.
  - **(B) Gewinn-only-MVP**: gewinnanteil ≥ 0 per Geltungsbedingung; Verlust = benannte Lücke (§15a später).
    Over-tax-safe, schneidet aber Verlust-Mitunternehmer aus.

### 2.4 Sondervergütungen ≥ 0: kein Guard nötig
Tätigkeitsvergütung/Zinsen/Miete sind Zuflüsse (≥0 naturgemäß). Negative Eingabe = Fehl-Eingabe (Trust-
Boundary: bezogene Vergütungen). Falls positiv → over-tax-safe.

### 2.5 Out-of-MVP-Scope (dokumentieren, kein Formel-Bestandteil)
- §15 Abs.1 Nr.2 **S.2 mittelbare Beteiligung** (Doppelstock-PersG): Zurechnungs-Regel; gewinnanteil-Input
  enthält bereits die mittelbare Zurechnung (Feststellungsbescheid).
- §15 **Abs.4** Verlust-Ausgleichsverbote (Tierzucht/Termingeschäfte/stille Innengesellschaft): wie §15a
  spezielle Verrechnungs-Verbote; Mitigation-(A) (ausgleichsfähiger-Anteil-Semantik) deckt sie mit ab.

## 3. Materialisierungs-Plan (byte-gleich)
1. **Datei**: `rules/estg/p15_1_2_mitunternehmer/mitunternehmereinkuenfte.catala_en` (filename = lowercased
   Modulname). Erster ```catala```-Block byte-gleich Snapshot → sha256 == `598768bc…`.
2. **clerk.toml**: `include_dirs += "rules/estg/p15_1_2_mitunternehmer"`; `[[target]] modules += "MitunternehmerEinkuenfte"`.
3. **§-Anker** (PFLICHT vor Materialisierung, Anker-Gate): §15 Abs.1 S.1 Nr.2 S.1 — Zitatanker aus
   estg_p15_2026-07-14.txt (Wortlaut oben 2.2, source-verifiziert). deckt_ab: gewinnanteil + 3 Sondervergütungen.
4. **#[test]-Seeds** (in catala + rules.yaml-candidate): (i) reiner Gewinn (gewinnanteil 50000, Rest 0 → 50000);
   (ii) Gewinn+alle 3 Sondervergütungen (10000+12000+3000+5000 → 30000); (iii) **BOUNDARY**: negativer
   gewinnanteil −20000 + Tätigkeit 12000 → −8000 (dokumentiert §15a-Input-Semantik = ausgleichsfähiger Anteil);
   (iv) alles 0 → 0.
5. Verify: `clerk build p32a-python` → `assemble_catala.sh` → `clerk test` / pytest.

## 4. 4-Feld-Binding-Spec (bindung_an_gesamt.yaml + bindung_rentner.yaml)
| feld_id | Herkunft §15 Abs.1 Nr.2 | Typ | Kz | Pflicht |
|---|---|---|---|---|
| gewinnanteil | „Gewinnanteile der Gesellschafter" (ausgleichsfähig n. §15a, Feststellungsbescheid) | money/cent | null-MVP (Anlage G Mitunternehmer-Zeile; eigene Kz-Runde) | optional (absent→0) |
| verguetung_taetigkeit | „Tätigkeit im Dienst der Gesellschaft" | money/cent ≥0 | null-MVP | optional |
| verguetung_darlehen | „Hingabe von Darlehen" (Zinsen) | money/cent ≥0 | null-MVP | optional |
| verguetung_ueberlassung | „Überlassung von Wirtschaftsgütern" (Miete/Pacht) | money/cent ≥0 | null-MVP | optional |

- quelle.regel_id = `p15_1_2_mitunternehmer` (NEU). Alle 4 → gesamt-scheibe (nicht n_vor_gwg → test_graph_uebersicht
  n_vor_gwg==7 immun; ⚠ JOINT-Build: verify keine gesamt-scheibe-Count-Assertion bricht).
- **est_mapping**: alle 4 = null-Kz-MVP Aggregat (Instructor-Adjudikation: KEIN VERZWEIGUNG jetzt, per-Art-Kz =
  eigene Kz-Runde). 4 SEPARATE Modul-Inputs (die Formel summiert sie) — NICHT slot_beitrag-Summanden.
- **flag_check (Pflichtfix)**: FLAG_NEGIERT["kein_gewinn"] += die 4 Mitunternehmer-Felder (sie speisen
  einkuenfte_gewinn → kein_gewinn=true muss ihre Absenz erzwingen, wie §§13-18-Stufe-1).
- **Ring-Naht (dev-1, JOINT)**: Accessor catala_mitunternehmer_einkuenfte(4 Inputs) → einkuenfte_mitunternehmer
  → faltet in einkuenfte_gewinn (betriebsart=gewerbe; §15 = gewerblich, Anlage G).

## 5. §35-Nr.2-Naht (a2, nach a1)
§35 Abs.1 S.1 Nr.2 (Source estg_p35): Mitunternehmer → 4× **anteiliger** GewSt-Messbetrag (§35 Abs.2 gesondert+
einheitlich festgestellt). 1 Feld `anteiliger_gewst_messbetrag` (FA-Grundlagenbescheid, null-Kz-MVP wie
gewst_messbetrag; Kz E0802104 vorgemerkt), reuse §35-4×-Accessor (runner.py:665) als Summand. Braucht a1 zuerst.

## Fazit
p15_1_2 ist **byte-gleich materialisierbar** (Formel trivial, faithful-Snapshot). Der EINE ernste
Boundary-Punkt = **negativer gewinnanteil ohne §15a-Guard (latenter Under-Tax)** → gelöst via Input-Semantik
„ausgleichsfähiger Anteil" (Empfehlung A) + Boundary-Seed + Feld-Hilfe-Text. Kein p10d_2-Cap-Bug (keine Caps
in der Formel). Anker §15 Abs.1 S.1 Nr.2 S.1 source-verifiziert, deckt alle 4 Komponenten. Bereit für
Instructor-Boundary-Review → JOINT-Materialisierung nach dev-1s §24a-Bundle.
