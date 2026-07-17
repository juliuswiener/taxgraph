# Beleg-Writer Stufe 1b — Spendenquittung + Handwerker-Rechnung: Konzept-Skizze (Task #11)

**Status:** Konzept-Skizze zur Instructor-Abnahme, concept-first, KEIN Code. Additiv zum Beleg-Writer-
Rahmen (4c526bf). LLM-frei, lokal (pdftotext/tesseract-deu), fail-closed, $0. schreiber-scoped Guard
`^import:beleg` greift schon.

## Recon-Befunde (bestimmen den Zuschnitt)

| Beleg-Typ | Ziel-Feld(er) | Kz | herkunft_slots | Freeze/Regel |
|---|---|---|---|---|
| **Handwerker-Rechnung** | hh_handwerker_arbeitskosten (+ hh_dienstleistungen, hh_minijob_aufwendungen) | E0161804 / E0161504 / E0161404 (existieren) | **KEINE** | § 35a (p35a_2_3_haushaltsnahe), gefreezt |
| **Spendenquittung** | **spenden_betrag (NEU)** | E0108405 „Spenden Inland" (Kandidat) | — | § 10b (p10b_spenden), estg_p10b gefreezt |

Zwei Konsequenzen: (1) Handwerker-Felder haben **keine herkunft_slots** → Anker per Label-Match (nicht
LStB-Nr). (2) Für Spende gibt es **noch kein Bindungsfeld** → Struktur-Add `spenden_betrag` (§ 10b).

## 1. Beleg-Typ-Erkennung (Kopf-Marker, wie erkenne_lstb)

`erkenne_beleg_typ(text) -> "lstb"|"spende"|"handwerker"|None`:
- **Spende:** „Zuwendungsbestätigung" / „Bestätigung über Geldzuwendungen" (amtlich vorgeschriebenes
  Muster → stabiler, eindeutiger Marker).
- **Handwerker:** „Rechnung" + Präsenz von „Arbeitskosten"/„Lohnanteil"/„Handwerkerleistung" (weniger
  standardisiert → Marker + Feld-Präsenz, sonst None).

## 2. Feld-Extraktion → feld_ids

- **Handwerker → hh_handwerker_arbeitskosten (E0161804):** NUR der **Arbeitskosten-Anteil** — § 35a
  Abs. 5 S. 2 „Der Abzug ... gilt nur für Arbeitskosten" (Material ist NICHT begünstigt). Label-Match
  „Arbeitskosten"/„Lohnkosten"/„Lohnanteil"/„Anfahrt". (hh_dienstleistungen/hh_minijob analog, falls
  die Rechnung sie ausweist.)
- **Spende → spenden_betrag (NEU, E0108405):** amtliches Muster hat feste Zeile „Betrag der Zuwendung".
- **bool-Gates NICHT OCR-extrahiert** (Ja/Nein, kein Beleg-Betrag): hh_rechnung_unbar (§ 35a Abs. 5 S. 3
  Konto-Zahlung), hh_handwerker_keine_foerderung, agb_zwangslaeufig — bleiben manuell (wie Stufe 1).

## 3. Anker (kein herkunft_slots vorhanden — Entscheid nötig)

Zwei saubere Optionen:
- **(A) herkunft_slots an die Felder ergänzen** (Handwerker „Handwerkerrechnung: Arbeitskosten", Spende
  „Zuwendungsbestätigung: Betrag der Zuwendung") → der Writer generalisiert einen **Label-Anker-Modus**
  (LStB nutzt Nr-Anker, Stufe 1b Label-Anker). Anker bleibt in der Bindung (Doktrin-konsistent).
  **EMPFEHLUNG.**
- (B) Beleg-Typ-Label-Config im Writer (keine Bindungs-Änderung) — Anker doppelt gepflegt, weniger
  konsistent.
Der Writer braucht so oder so einen **zweiten Anker-Modus „Label-adjazenter Betrag"** (Fließtext-Rechnung
statt Formular-Nr). §10b/§35a-Norm-Anker sind gefreezt + werden bei Bau _normalize-verifiziert.

## 4. Ehrlichkeit (wie Stufe 1)

- OCR-Confidence → signal_1, **vorlaeufig bleibt vorlaeufig** (nie Auto-Bestätigung, K2).
- Unlesbar/mehrdeutig → **KEIN geratener Wert, benannte Lücke** (Feld bleibt offen, manuell).
- **Handwerker-Sonderfall (Missbrauchsschutz):** weist die Rechnung KEINEN separaten Arbeitskosten-
  Anteil aus (nur Gesamtbetrag inkl. Material) → **kein Wert** (Lücke), NIE den Gesamtbetrag als
  Arbeitskosten raten (§ 35a-Grenze). Mehrere Beträge → alle als Vorschläge zeigen, Mensch wählt.

## 5. Synthetische Test-Belege (fiktiv, Repo, nicht faelle/)

`tests/fixtures/muster_zuwendungsbestaetigung_2025.txt` (fiktiver Verein, Muster-Betrag) +
`muster_handwerkerrechnung_2025.txt` (fiktive Firma, ausgewiesener Arbeitskosten-Anteil + Material zur
Trennungs-Probe). Gate: Erkennung + Extraktion (Arbeitskosten getrennt vom Material!) + Spende-Betrag +
confidence→vorlaeufig + Material-ohne-Arbeitskosten→Lücke + Guard.

## Offene Entscheide (zur Abnahme)

- **(a) Anker-Modus A (herkunft_slots ergänzen, Empfehlung) vs B (Writer-Config)?**
- **(b) Spende-Feld `spenden_betrag` (§ 10b, p10b_spenden):** anlegen? Kz-Kandidat **E0108405** („geleistete
  Spenden an Empfänger im Inland") — STRONG, ABER Ausland-Variante E0105502 existiert → Kz wie Anlage-V-
  Muster kurz von dir bestätigen lassen (Kz-Doktrin) oder direkt eintragen?
- **(c) Handwerker-Umfang:** nur hh_handwerker_arbeitskosten oder auch dienstleistungen/minijob?
- **(d) Handwerker Material-vs-Arbeit:** nur ausgewiesener Arbeitskosten-Anteil, sonst Lücke — bestätigen?

Nach Abnahme: beleg_writer.py generalisieren (Beleg-Typ-Config + Label-Anker-Modus) + ggf. spenden_betrag-
Bindung + herkunft_slots + 2 synthetische Belege + Gate. Alles LLM-frei/lokal/fail-closed.
