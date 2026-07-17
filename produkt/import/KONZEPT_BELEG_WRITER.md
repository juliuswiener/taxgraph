# Beleg-Upload-Writer — Konzept-Skizze (Stufe 1, NUR OCR, kein Bank, kein LLM)

**Status:** Konzept-Skizze zur Instructor-Abnahme. KEIN Code. Verortung + 4 Kern-Fragen + Leitplanken.
Basiert auf `produkt/haut/KONZEPT_BELEGQUELLEN.md` (04c8804). LLM-frei, lokal-first, fail-closed.

**Doktrin-Passung:** ein neuer **Writer** über dem EINEN Store — keine Engine-/Registry-/Store-Kern-
Änderung außer EINER Guard-Zeile (siehe 2). `herkunft=beleg_import` ist im Store-Schema bereits gültig
(Enum). Der Beleg-Wert ist Signal 1 (Vorschlag), er bewegt keine Steuer-Zahl bis der Mensch ihn neben
dem Beleg bestätigt (Signal 2). K2 (fail-closed) unverändert.

## 1. Extraktions-Pfad: Beleg (PDF/Bild) → Kandidatenwerte je feld_id (deterministisch, LLM-frei)

```
Beleg → [Textschicht?] → Text → Beleg-Typ-Erkennung → Anker-/Label-Match → typ-Parser → Kandidaten
         │pdftotext (digital)                          │aus bindung.herkunft_slots
         └tesseract-deu (Bildscan, wie DBA-Route)
```

- **Textschicht zuerst:** digitale Rechnungen/LStB-PDF haben einen Textlayer → `pdftotext` (exakt,
  kein OCR-Rauschen). Nur echte Bild-Scans → **tesseract-deu** (lokal vorhanden, wie die DBA-Bildscan-
  Route, `[[bgbl-scan-textlayer-ocr-route]]`). Kein externer Dienst → Gerät wird nie verlassen.
- **Anker aus der Bindungstabelle — schon da:** das Feld `herkunft_slots` trägt bereits die Beleg-
  Positions-Anker: `bruttoarbeitslohn → "Lohnsteuerbescheinigung Nr. 3"`, `vor_an_anteil_rv → "Nr. 23
  a/b"`, `vor_ag_anteil_rv → "Nr. 22 a/b"`. Der Writer sucht diese Label/Nummern-Anker im Text
  (Label-Match) und liest den benachbarten Wert. Der Anker IST die Verankerungs-Doktrin auf der
  Beleg-Seite (kein Rate-Match).
- **Beleg-Typ-Erkennung** über charakteristische Kopf-Marker (z. B. „Lohnsteuerbescheinigung",
  „Zuwendungsbestätigung/Spende", „Rechnung … Arbeitskosten") → wählt das Feld-Set (die feld_ids mit
  passenden herkunft_slots) für diesen Beleg-Typ. Kein Typ erkannt → keine Extraktion (Lücke).
- **typ-Parser je Ziel-typ:** `cent` = EUR-Betrag-Regex (`\d{1,3}(\.\d{3})*,\d{2}` → Cent-Int),
  `datum` = ISO-Normalisierung, `int` = Ganzzahl. **`bool`/`enum` werden NICHT OCR-extrahiert** — das
  sind Ja/Nein-/Auswahl-Fragen, keine Beleg-Werte (Feld-Verteilung: cent 28 / int 11 / bool 37 /
  enum 1 → Stufe 1 zielt auf die cent/datum/int-Betragsfelder mit Anker, bool/enum bleiben manuell).
- **Ausgabe je Feld:** `{feld_id, wert, confidence, beleg_position (Seite/BBox), roh_text}`. Nichts
  Extrahierbares/Mehrdeutiges → KEIN Wert, benannte Lücke (siehe 4).

## 2. Store-Integration + Guard-Erweiterung (der EINZIGE Kern-Zuwachs)

Der Writer schreibt über den EINEN Pfad:
```
append_event(feld_id, wert, zustand="vorlaeufig",
             herkunft={herkunft: "beleg_import", pruef_tiefe: "ungeprueft", haftung: "nutzer"},
             schreiber="import:beleg", signal={signal_1: <beleg-ref>, signal_2: null})
```
- `herkunft=beleg_import` ist im Schema-Enum bereits gültig → **keine neue Kategorie nötig**.
- **GUARD-ERWEITERUNG (symmetrisch zum `llm:`-Guard):** heute erzwingt `append_event` nur für
  `schreiber ^llm:` die Kopplung (herkunft=llm_vorschlag, vorlaeufig, signal_2=null). Für Belege
  fehlt das Gegenstück. Vorschlag: `schreiber ^import:beleg` (bzw. `herkunft=beleg_import`) ⇒ **zwingt
  zustand=vorlaeufig + signal_2=null** — ein Beleg kann strukturell NIE direkt bestätigt werden.
  Symmetrisch im Schema: `allOf` `herkunft=beleg_import ⇒ vorlaeufig` (parallel zur bestehenden
  `llm_vorschlag ⇒ vorlaeufig`-Regel). Das ist die einzige Code-Zeile am Store-Kern.

## 3. Beleg als signal_2-Objekt (K3, spätere Zwei-Signal-Bestätigung)

- Der vorlaeufig-Wert wird später vom Menschen **neben dem angezeigten Beleg** bestätigt. `signal_2`
  wird dann ein **Beleg-Referenz-Objekt** statt eines bloßen Klick-Strings, z. B.
  `"beleg:rechnung_2025_042#seite2#bbox=…"` — der hochgeladene Beleg IST der Bestätigungs-Nachweis.
- Der Beleg-Anhang liegt **lokal-first** (`produkt/haut/faelle/`-Politik: gitignored, nie ins Repo,
  127.0.0.1). Die Justification-Kante („folge der Kante") führt danach vom Euro nicht nur zum
  Paragraphen-Anker, sondern auch zum Beleg — genau die Herkunfts-Invariante des Produkts.
- `signal_1` (der Beleg-Vorschlag) trägt die Beleg-Referenz + confidence schon beim Import; `signal_2`
  ist der menschliche Bestätigungs-Akt darauf (Event mit `ersetzt=<import-event_id>`).

## 4. Ehrlichkeit: Confidence, nie Auto-Bestätigung, unlesbar = Lücke

- tesseract liefert **per-Wort-Confidence** (`--tsv`, `conf`-Spalte) → je Kandidat eine OCR-Confidence.
  Sie wandert in den Vorschlag (signal_1-Metadatum), **ändert aber nie den Zustand**: vorlaeufig bleibt
  vorlaeufig, egal wie hoch. **Keine Auto-Bestätigung** — das ist die K2-Invariante, nicht verhandelbar
  (`[[falsches-gruen]]`). Confidence dient NUR zur Anzeige/Sortierung, NIE zum Setzen.
- **Unlesbar/mehrdeutig → KEIN geratener Wert**, benannte Lücke: das Feld bleibt offen, die UI fragt
  es manuell. Mehrere Betrags-Kandidaten für ein Feld → ALLE als Vorschläge zeigen, Mensch wählt; nie
  still den „besten" nehmen.

## Abgrenzung (streng Stufe 1)

- **NUR** OCR/Beleg-Upload strukturierter Belege. **KEIN** Bank/Kontoauszug (eigener Julius-Cap,
  PSD2-Leitplanken im Belegquellen-Konzept). **KEIN** LLM (Freitext-Rechnungen = spätere LLM-Vorschlags-
  Schicht, eigener Cap).
- **Lokales tesseract verlässt das Gerät NICHT** → passt zur Lokal-first-Leitplanke, kein ausgehender
  Dienst. Ein EXTERNER OCR-Dienst wäre eine ausgehende Integration → Julius-Wort (Leitplanke 5); Stufe
  1 nutzt bewusst nur den lokalen Pfad.
- Writer-Modul (später): `produkt/import/beleg_writer.py` — konsumiert Bindungstabelle (Anker/typ) +
  schreibt Store. Kollisionsfrei mit dev-1 (an_gesamt-Ring, produkt/haut).

## Offene Entscheide (zur Abnahme)

1. **Guard-Ort:** `append_event` schreiber-Prefix `^import:beleg` (wie `llm:`) UND schema-`allOf`
   `beleg_import ⇒ vorlaeufig` — beide (Empfehlung, Code + Gate)?
2. **Confidence-Ablage:** in `signal_1` (Beleg-Referenz-Objekt trägt confidence) oder ein optionales
   Event-Metadatum? Vorschlag: signal_1-Objekt.
3. **signal_2-Beleg-Objekt-Format** + Beleg-Speicher-Ort (`produkt/haut/faelle/<fall>/belege/`?).
4. **Beleg-Typ-Umfang Stufe 1:** nur Lohnsteuerbescheinigung (die herkunft_slots-Felder sind LStB-
   zentriert) oder auch Spendenquittung / Handwerker-Rechnung (§35a-Kz existieren)?
5. **Gate:** deterministischer Extraktions-Test (bekannter Test-Beleg → erwartete Kandidatenwerte +
   confidence; Negativ: unlesbar → Lücke, nie Auto-bestätigt; Guard: import:beleg kann nicht
   bestätigen).

Nach Abnahme: Guard-Zeile + Schema-Regel (Store-Kern) + `produkt/import/beleg_writer.py` (Extraktions-
Pfad) + Extraktions-Gate. Alles LLM-frei, lokal, fail-closed.
