# Ehegatte-Behinderung (Partner-GAP §33b) — Stufe-A-Zuschnitt

**Status:** Recon + Kz-Satz-Beleg zur Instructor-Review, concept-first. **KEIN Bau vor OK.** LLM-frei.
Erweitert die p33b-Person-A-Arbeit (b39daff) auf den Ehegatten (E05058-Block).
**GRÖSSE: SMALL** — 2 Felder + 1:1-est_mapping + K2-Guard + Drift.

## 1. Person-B-Kz-Satz — belegt via Hash/Dok-Order/Vordruck (NICHT E-Präfix)
Block-Hash m1740292092 (E05058), Dok-Order NACH E01097, Vordruck agb_2025 zweiter Block „Ehefrau /
Person B" (kurz-Kz 155/153/154):

| Konzept | Person-B-Kz | Sektion (Tag) | wörtliches Label | Vordruck-Kz |
|---|---|---|---|---|
| GdB Partner | **E0505809** | [Ausw_Rentb_Besch] | „Grad der Behinderung" | 155 |
| hilflos/blind/taubblind Partner | **E0505807** | [Blind_Hilfl] | „blind / taubblind / ständig hilflos (Merkzeichen Bl, TBl und/oder H)" | 153 |
| (gehbehindert G/aG Partner) | E0505808 | [Geh_Steh] | „erheblich gehbehindert (G/aG)" | 154 → **bewusstes Nicht-Feld** (§33 Abs.2a Fahrtkostenpauschale, wie E0109707 bei Person A) |

## 2. MELDE-STATT-RATEN: est_mapping-Klasse ist **1 (1:1), NICHT g**
Deine Zuschnitt-Vorgabe nannte Klasse g (PARTNER_INSTANZ, Kz-Reuse wie Splitting). **Hier trifft das
NICHT zu:** Klasse g war nötig, weil Person B beim Bruttoarbeitslohn KEINE eigenen Kz hat (E0220201
existiert nicht → Person-A-Kz in Instanz-B-Bucket). Bei der Behinderung hat Person B **eigene distinkte
Kz** (E0505809/E0505807). Also plain **Klasse 1**: `rentner_*_partner → eigenes E05058-Kz`, 1:1.
Kein person_b-Bucket, keine neue est_mapping-Maschinerie. (Flache Zwillingsfelder = nur die Store-/Feld-
Seite folgt dem Splitting-Modell-A-flach-Muster; das Kz-Mapping selbst ist 1:1.)

## 3. Person-B-Hinterbliebenen — KEIN eigener Kz (belegt, dass keiner existiert)
Instructor-Auftrag „finden oder belegen dass es keinen gibt": **Es gibt keinen.** Schema-weit existieren
NUR zwei Hinterbliebenen-Claim-Kz:
- **E0109704** [Hinterbl] „Ich beantrage den Hinterbliebenen-Pauschbetrag" — Person-A-eigen (erstes
  Dok-Vorkommen @1131347, gebunden b39daff).
- **E0505805** [Hbl] „Die Übertragung des Hinterbliebenen-Pauschbetrags wird beantragt" — Kind-Übertragung
  § 33b Abs. 5, NICHT Person B.

Der Papier-Vordruck agb_2025 zeigt zwar Hinterbliebenen A/B (kurz-Kz **380** Person A / **381** Person B),
aber der **E10-Datensatz führt KEINEN getrennten Person-B-Hinterbliebenen-E-Nr**. (E0109704 hat genau
1 CType-Instanz [Hinterbl_2106281], keine maxOccurs-Wiederholung belegbar → NICHT als Klasse-g-Reuse
behandeln, das wäre geraten.)
→ **rentner_hinterbliebenenbezuege_partner = benannte GAP** (kein Feld bauen, oder null-Kz mit Grund).
Fail-closed: kein erfundener Kz.

## 4. Pflege-Pauschbetrag: N/A für Partner-GAP
Der Pflege-Block [Ang_pflegebeduerft_Pers] beschreibt die **gepflegte Person** (Name/IdNr/Pflegegrad/
Wohnsitz/Merkzeichen H), NICHT stpfl./Ehegatte. Kein A/B-Träger-Split auf Deklarations-Kz-Ebene
(„Angaben zur pflegenden Person" ist ein separater Vordruck-Teil). → **Partner-Pflege ist kein Bestand-
teil der Ehegatte-Behinderung**, kein Feld.

## 5. K2-Guard (dein Flag bestätigt)
Partner-Behinderungs-Felder sind nur bei **veranlagung == zusammen** sinnvoll. → K2-Guard „kein
Partner-Feld ohne Zusammenveranlagung" (fail-closed, Muster partner_kegel_offen). Baue ich in
produkt/konsistenz/ analog flag_check.

## 6. Regel-Andockung + Gültigkeit
- § 33b gilt für den Ehegatten **identisch** (jeder Ehegatte hat eigenen Behinderten-Pauschbetrag). ✓
- Regel-Seite: p33b_behinderten_pauschbetrag ist Single-Person-Scope. Der Partner braucht eine
  **zweite Instanz** (Person-B-Auswertung). Wie schon bei Person A ist die **Tarif-Integration** des
  Pauschbetrags NICHT in dieser Deklarations-Runde (agB-Ring = separat/dev-1). Deklarations-Seite
  (meine Zone) ist vollständig mit 1:1-Kz + K2-Guard; die Regel-Zweitinstanz/Tarif-Wirkung ist gleicher
  Status wie Person-A (kein Ring in dieser Runde).

## Bau-Umfang (nach OK)
1. bindung_rentner.yaml: 2 Felder `rentner_grad_der_behinderung_partner` (int 20..100, E0505809),
   `rentner_hilflos_blind_taubblind_partner` (bool, E0505807) — je Kz-Zitatanker + §33b-Anker + K2-Hinweis.
2. produkt/konsistenz/: K2-Guard partner-Behinderung ⇒ veranlagung==zusammen.
3. Drift-Wächter: feld_id global eindeutig, Kz-Eindeutigkeit über Person-A UND Person-B (E0505809/807
   dürfen mit keinem A-Kz kollidieren — geprüft: kollisionsfrei).
4. Test: 1:1-Deklaration _partner + K2-Guard + Roundtrip.

## Zur Abnahme
(1) Person-B-Kz-Satz E0505809/E0505807 OK? (2) Klasse-1-statt-g bestätigt? (3) Hinterbliebenen_partner
= GAP (kein Feld) OK? (4) Pflege_partner = N/A OK? (5) K2-Guard bauen? → dann Bau, Freeze zu dir.
