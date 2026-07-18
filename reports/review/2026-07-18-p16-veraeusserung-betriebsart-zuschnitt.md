# §16 Abs.4 Betriebsveräußerungs-Freibetrag — Stufe-A-Zuschnitt (rentner_veraeusserungsgewinn)

**Status:** Brücken-Aufgabe (concept-first, KEIN Bau vor OK; pausiert sobald dev-1s Kapital-Accessor kommt).
LLM-frei. Vertieft den Audit-Kandidat rentner_veraeusserungsgewinn zur abnahmefähigen Klasse-f-Weiche.
**GRÖSSE: SMALL** — 1 neues Art-Feld + 1 Klasse-f-Config + Kz-Verzweigung. Nur Deklaration, KEIN Ring.

## Bestehendes (verifiziert)
- `rentner_veraeusserungsgewinn` (cent, kz=null, quelle p16_4_freibetrag/veraeusserungsgewinn) — EXISTIERT.
- `rentner_alter_55_oder_berufsunfaehig` (bool, geltungsbedingung alter_55_oder_berufsunfaehig) — EXISTIERT.
- `rentner_freibetrag_erstmalig` (bool, geltungsbedingung freibetrag_einmal_im_leben) — EXISTIERT.
- `p16_4_freibetrag` (scope BetriebsFreibetrag, input veraeusserungsgewinn:money → output freibetrag) —
  **ANLAGE-AGNOSTISCH** (rechnet den 45k-Freibetrag mit 136k-Abschmelzung unabhängig von der Anlage).
- §16 Abs.4-Anker (estg_p16_2026-07-14, alle _normalize-verifiziert): „…so wird der Veräußerungsgewinn
  auf Antrag zur Einkommensteuer nur herangezogen, soweit er 45 000 Euro übersteigt" / „Der Freibetrag ist
  dem Steuerpflichtigen nur einmal zu gewähren" / „Er ermäßigt sich um den Betrag, um den der
  Veräußerungsgewinn 136 000 Euro übersteigt".

## (1) Kz-Matrix je Betriebsart (Hash/Vordruck-belegt, NICHT E-Präfix)
Der Freibetrag ist anlage-agnostisch, aber der DEKLARATIONS-Kz hängt an der Betriebsart (Anlage):

| Betriebsart (Enum) | Kz | Anlage / Beleg | Konfidenz |
|---|---|---|---|
| **gewerbe** (§ 16 Betrieb / Mitunternehmeranteil) | **E0801301** [VAe_G_FB_Antr] | Anlage G (E08, Hash 592812681); Block-Nachbarn belegen Kontext: E0801401 „Veräußerungsgewinn(e)", E0801608 „Freibetrag", E0801605 „des Betriebs / des Mitunternehmeranteils" | STRONG |
| **selbstaendig** (§ 18) | **E0901201** [VAe_G_FB_Antr] | Anlage S (E09, Hash 163190041); identische Label-Semantik „Veräußerungsgewinn vor Abzug des Freibetrags nach § 16 Abs. 4 EStG" | STRONG |
| **land_forst** (§ 14) | — **KEIN Kz** (proven-absence) | Anlage L; schema-weite Suche fand KEINEN E13-/L+F-Kz mit §16-Abs.4-FB-Label | GAP (benannt, wie Person-B-Hinterbliebenen) |

**Sub-Kontext (nachgelagert, NICHT MVP):** E0804501 [Vor_FB] (Anlage G, Block E0804xx mit E0804005
„Beteiligungsquote", E0804210 „Betriebsaufgabe über mehr als ein Kalenderjahr") = die
Anteils-/Beteiligungsquoten-Veräußerung — feinerer Gewerbe-Sub-Zweig. MVP mappt gewerbe→E0801301
(Haupt-Betriebsveräußerung); der Beteiligungs-Sub-Fall bleibt benannter Folge-Nachtrag.

## (2) Neues Art-Feld + est_mapping Klasse-f (wie renten_art)
- **NEU:** `rentner_veraeusserungs_betriebsart`, Enum **{gewerbe, selbstaendig, land_forst}**, askable,
  null-Kz (Art-Weiche, kein eigenes Deklarations-Kz — exakt wie rentner_renten_art).
- **est_mapping Klasse-f VERZWEIGUNG** (Ergänzung produkt/mapping/est_mapping.py):
  ```
  "rentner_veraeusserungsgewinn": {"art_feld": "rentner_veraeusserungs_betriebsart", "kz": {
      "gewerbe": "E0801301", "selbstaendig": "E0901201"}}   # land_forst: kein Kz-Zweig → nicht_deklariert
  ```
  Der bestehende Klasse-f-Mechanismus behandelt land_forst fail-closed (Art ohne Kz-Zweig →
  nicht_deklariert mit Grund), analog dem renten_art-„ohne Kz-Zweig"-Pfad.

## (3) Anker + Regel (nur Deklaration, KEIN Ring)
p16_4 ist anlage-agnostisch → est_mapping wählt nur den Kz je Betriebsart, die Freibetrags-RECHNUNG macht
die Regel. Voraussetzungs-Flags (alter_55, freibetrag_erstmalig) sind da. **Der §16-RING (Veräußerungs-
gewinn − Freibetrag → einkuenfte_gewinn im Tarif) wäre ein EIGENER Nachtrag (dev-1)** — diese Runde ist
reine Deklaration (Kz-Zuordnung + Art-Feld).

## Bau-Umfang nach OK
1. bindung_rentner.yaml: neues Feld rentner_veraeusserungs_betriebsart (Enum, null-Kz, §16-Abs.4-Anker).
2. est_mapping.py: VERZWEIGUNG-Eintrag rentner_veraeusserungsgewinn (gewerbe/selbstaendig, land_forst=GAP).
3. Drift-Wächter + Klasse-f-Test (Verzweigung gewerbe→E0801301/selbstaendig→E0901201 + land_forst-GAP + Roundtrip).

## Zur Abnahme
(1) Kz-Matrix E0801301/E0901201 OK? (2) Enum {gewerbe, selbstaendig, land_forst} OK, land_forst=benannte GAP?
(3) Beteiligungs-Sub-Fall (E0804501) als Folge-Nachtrag OK? (4) §16-Ring bleibt separater Nachtrag (dev-1)?
→ dann Bau. KEIN Kz-Eintrag/Bau vor OK.
