# §33b Behinderten-/Hinterbliebenen-/Pflege-Pauschbetrag — Stufe-A-Zuschnitt + E-Nr-Kandidaten

**Status:** Recon + E-Nr-Kandidaten-Tabelle zur Instructor-Review, concept-first. **KEIN Direkt-Eintrag
in die Bindung vor OK** (Kz-Zitatanker-Doktrin). LLM-frei.
**Kern-Befund: SMALL — Felder + Regeln existieren; die eigentliche (deferred) Lücke ist die E-Nr-Zuweisung.**

## 1. §33b-Struktur: Regeln EXISTIEREN
- `p33b_behinderten_pauschbetrag` (scope BehindertenPauschbetrag, inputs `{grad_der_behinderung int,
  ist_hilflos_blind_taubblind bool}`, output behinderten_pauschbetrag) — die **GdB-Stufenfunktion**
  (§ 33b Abs. 3, 2021er-Reform-Beträge) ist IN der Regel (geltungsbedingung `gdb_stufenfunktion_von_
  mindestens` + `hilflos_blind_taubblind_override`). + `p33b_hinterbliebenen_pauschbetrag` +
  `p33b_pflege_pauschbetrag`. **Der Pauschbetrag wird von der Regel gerechnet — keine Pseudoregel nötig.**
- Fassung: estg_p33b **„geltende Fassung 2026"** — gültig, kein neuer Freeze. Vordruck **agb_2025**
  (Anlage Außergewöhnliche Belastungen) ist gefreezt (Vordruck-Zeilen-Cross-Check verfügbar).

## 2. Deklarations-Felder: EXISTIEREN (Scheibe 4, bindung_rentner) — alle null-Kz
`rentner_grad_der_behinderung` (int 20..100), `rentner_hilflos_blind_taubblind` (bool),
`rentner_hinterbliebenenbezuege` (bool), `rentner_pflegegrad` (int), `rentner_gepflegter_hilflos` (bool).
**Kein neues Feld nötig.** Die einzige Lücke = die E-Nr (die Anlage-R/Kind-Runde hatte p33b bewusst auf
„eigene Runde nach Behinderung-Freeze" vertagt — jetzt).

## 3. est_mapping-Klasse: KEINE neue
GdB/Pflegegrad/Merkmale sind **Input-Felder** (1:1 auf ihr Anlage-Kz, sobald resolved); der Pauschbetrag
ist **Regel-Output** (nicht deklariert). Also nur die 1:1-Klasse (nach Kz-Zuweisung). Keine Staffel-Logik
in est_mapping — die rechnet die Regel.

## KORREKTUR (Instructor-Refutation msg 2717 — erste Tabelle war blockgemischt)

Meine erste Auflösung („E05-Präfix = Anlage Kind") war **FALSCH** (widerlegt): E0505809 ist NICHT der
Kind-GdB, sondern identischer Tag+Label wie E0109708 — beide „Grad der Behinderung", nur andere Person.
Ich hatte die Blöcke gemischt (GdB von Person-A-Block, hilflos von Person-B-Block). Neu-Recon **über den
Sektions-Pfad (nicht den Tag)** — dreifach belegt.

### Sektions-Pfad-Beleg: welcher Block ist die stpfl. Person?
Die Behinderten-Kz kommen in ZWEI parallelen Person-Blöcken (CType-Hash gruppiert):
- **Block 66196332 (E01097…):** E0109708 GdB · E0109706 „blind/taubblind/ständig hilflos (Bl/TBl/H)" ·
  E0109707 „erheblich gehbehindert (G/aG)" · E0109101/102/103 Ausweis-Gültigkeit
- **Block m1740292092 (E05058…):** E0505809 GdB · E0505807 blind/hilflos · E0505808 gehbehindert · …

**Drei unabhängige Belege, dass Block 66196332 = stpfl. Person (Person A):**
1. **CType-Hash-Gruppierung:** GdB + beide Merkzeichen einer Person teilen denselben Hash → EIN Block
   pro Person (das ist der Sektions-Pfad, nicht der mehrdeutige Tag).
2. **Dokument-Reihenfolge:** E01097-Block rendert VOR E05058-Block (Offset 3 835 045 < 3 859 447).
3. **Amtlicher Vordruck agb_2025 (STARK):** erster GdB-Block-Header = „Steuerpflichtige Person / Ehemann /
   Person A" (kurz-Kz 105/103/104), zweiter = „Ehefrau / Person B" (155/153/154). Erster = stpfl. Person.
→ **Person A = E01097-Block.** Alle stpfl.-Kz konsistent aus DIESEM Block.

## E-Nr-Tabelle KORRIGIERT (stpfl. Person, alle aus E01097-Block; kein Direkt-Eintrag vor OK)

| feld_id | E-Nr | Sektion (Tag) | wörtliches Label | Beleg |
|---|---|---|---|---|
| rentner_grad_der_behinderung | **E0109708** | [Ausw_Rentb_Besch] | „Grad der Behinderung" | Block 66196332, Vordruck-Kz 105 |
| rentner_hilflos_blind_taubblind | **E0109706** | [Geh_Steh_Blind_Hilfl] | „blind / taubblind / ständig hilflos (Merkzeichen ‚Bl', ‚TBl' und/oder ‚H')" | **KORR. von E0505807 (=Person B)** → Block 66196332, Vordruck-Kz 103 |
| rentner_hinterbliebenenbezuege | **E0109704** | [Hinterbl] | „Ich beantrage den Hinterbliebenen-Pauschbetrag" | Person A eigen (rendert vor E0505805; E0505805 = „Übertragung"/Kind-Transfer §33b Abs.5, NICHT Person B) |
| rentner_pflegegrad | **E0161606** | [Ang_pflegebeduerft_Pers] | „Für die pflegebedürftige Person wurde folgender Pflegegrad festgestellt" | §33b Abs.6, 1 Instanz-Hash im Schema (single-MVP) |
| rentner_gepflegter_hilflos | **E0161808** | [Ang_pflegebeduerft_Pers] | „Für die pflegebedürftige Person wurde das Merkzeichen ‚H' festgestellt" | §33b Abs.6, selbe Instanz |

### Ehegatte-Behinderung = benannte PARTNER-GAP (nicht jetzt, Muster Splitting-Partner)
Block m1740292092 (E0505809 GdB · E0505807 blind/hilflos · E0505808 gehbehindert) = Person B / Ehefrau.
Kommt bei Zusammenveranlagung über das Partner-Instanz-Muster (est_mapping Klasse g), NICHT in diese Runde.

### Bewusste Nicht-Felder
- „erheblich gehbehindert (G/aG)" (E0109707, Vordruck-Kz 104) = **§ 33 Abs. 2a behinderungsbedingte
  Fahrtkostenpauschale**, ANDERER Pauschbetrag als § 33b — kein rentner-Feld, korrekt ausgelassen.
- E0505805 „Übertragung des Hinterbliebenen-Pauschbetrags" = Kind-Transfer (§ 33b Abs. 5), Familien-Runde.

**Pflege-Instanz bestätigt:** [Ang_pflegebeduerft_Pers] hat im Schema genau 1 Instanz-Hash (1534749469)
→ single-cared-for-person ist die Schema-eigene Modellierung; mehrere Gepflegte = späterer Repeated-GAP.

## Was FEHLT / GRÖSSE
- Nur die **E-Nr-Zuweisung** (nach deinem Review der Tabelle) — 4× STRONG direkt eintragbar,
  1× MITTEL (GdB-Doppel-Kz, A/B-Klärung). Danach 1:1-Bindung + Drift-Wächter + Gate.
- KEIN Ring/Accessor-Bedarf (der Pauschbetrag ist eine agB-Größe im Grundtarif, die Regel rechnet ihn;
  falls je ein Ring die agB einbezieht = dev-1, nicht diese Runde).
- **GRÖSSE: SMALL** — Felder + Regeln + Vordruck da; nur Kz-Zuweisung + Gate.

## Zur Abnahme (korrigiert)
Person-A-Satz jetzt konsistent aus EINEM Block (E01097), Sektions-Pfad-belegt (Hash + Dok-Order +
Vordruck-Header), Ehegatte als Partner-GAP ausgelagert.
(1) Diese 5 Kz eintragen? → rentner_grad_der_behinderung=**E0109708**, _hilflos_blind_taubblind=**E0109706**
(NICHT E0505807), _hinterbliebenenbezuege=**E0109704**, _pflegegrad=**E0161606**, _gepflegter_hilflos=**E0161808**.
(2) danach baue ich 1:1-Bindungen (Klasse 1) + Drift-Wächter-Update + Gate. Kein Ring/Accessor.
KEIN Kz-Eintrag bis dein OK auf DIESE korrigierte Tabelle.
