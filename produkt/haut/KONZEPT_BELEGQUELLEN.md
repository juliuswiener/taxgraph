# Belegquellen — Beleg-Upload + Kontoauszug-Anschluss (Produkt-Richtung Julius 2026-07-18)

**Status:** Instructor-Vermerk (Anforderung Julius). Verortung + Architektur-Passung + Leitplanken.
KEIN Bau in dieser Stufe — spätere Paket-B-Stufen, eigener Julius-Cap für LLM-/Bank-Anbindung.

## Die Anforderung

Die UI soll (a) **Rechnungen/Belege hochladen** lassen (PDF/Foto → Werte extrahieren) und
(b) idealerweise einen **Kontoauszug-Anschluss** bieten, der Ausgaben aus den Bank-Transaktionen
erkennt und als mögliche steuerlich relevante Posten vorschlägt.

## Architektur-Passung: das ist KEIN Umbau, das ist ein neuer WRITER

Der Lab-Kernbefund war „**Der Store ist die Wahrheit, Häute/Eingabewege sind austauschbare
Schreiber**". Beleg-Upload und Kontoauszug sind exakt das: zwei neue Writer über dem EINEN
Sachverhalts-Store, ohne neue Rechen-/Registry-Maschinerie. Sie fügen sich in den bestehenden
`store.append_event`-Vertrag ein:

| Kanal | herkunft (Vektor-Achse) | zustand | signal_2 |
|---|---|---|---|
| manuell (Laie tippt) | `laie` | frei | Eingabe-Bestätigung |
| **Beleg-Upload (OCR/LLM-Extraktion)** | `beleg_import` | **immer `vorlaeufig`** | null bis Mensch bestätigt |
| **Kontoauszug (Transaktions-Klassifikation)** | `kontoauszug` (neue Achse-Kategorie) | **immer `vorlaeufig`** | null bis Mensch bestätigt |
| Vorjahr | `vorjahr` | `vorlaeufig` | null |

**Fail-closed gilt unverändert (K2):** ein Beleg-/Kontoauszug-Wert ist ein Vorschlag (Signal 1),
er bewegt keine Steuer-Zahl, bis der Mensch ihn neben dem Beleg bestätigt (Signal 2). Der
`herkunft_vektor` bekommt genau eine neue `herkunft`-Kategorie (`kontoauszug`) — sonst nichts.
Der Beleg-Anhang selbst wird das `signal_2`-Beleg-Objekt („Bestätigung neben dem Beleg", K3).

## Stufen-Verortung (nach dem aktuellen Gesamtsteuer-MVP)

1. **Beleg-Upload / OCR** — Beleg → Kandidatenwerte je feld_id (Betrag, Datum, Kategorie).
   Reiner OCR-Pfad (tesseract, LLM-frei) für strukturierte Belege; LLM-Extraktion für Freitext-
   Rechnungen = die LLM-Vorschlags-Schicht (eigener Julius-Cap, Chat-Slot-Vertrag).
2. **Kontoauszug-Anschluss** — eine Stufe darüber (Bank-API + Datenschutz, s. Leitplanken).
   Transaktion → Vorschlag „könnte Werbungskosten/Handwerker/Spende sein" → `vorlaeufig`.

Beide docken an denselben Store-Writer + dieselbe Zwei-Signal-Bestätigung wie die aktuelle Haut.
Der Beleg wird zum Herkunfts-Nachweis: „folge der Kante" führt vom Euro nicht nur zum Paragraphen,
sondern auch zum hochgeladenen Beleg/zur Transaktion.

## SICHERHEITS-LEITPLANKEN (nicht verhandelbar — Kontoauszug ist hochsensibel)

1. **Bank-Credentials NIE durch die App/KI.** Kontoauszug-Anschluss = Open-Banking (PSD2), der
   Nutzer authentifiziert **direkt bei seiner Bank** (OAuth-Redirect zur Bank-Oberfläche); die App
   sieht Zugangsdaten nie, erhält nur ein read-only-Token. Kein Passwort-/PIN-/TAN-Feld in der App.
2. **Read-only.** Nur Kontoumsätze lesen, nie Überweisung/Verfügung. Keine Zahlungsauslösung.
3. **Lokal-first.** Transaktionsdaten sind Nutzerdaten — dieselbe `produkt/haut/faelle/`-Politik
   (gitignored, nie ins Repo, 127.0.0.1-Bind). Kein Cloud-Upload ohne ausdrückliche Nutzer-Wahl.
4. **Klassifikation ist Vorschlag, nie Fakt.** „Diese Zahlung ist eine Werbungskosten-Ausgabe" ist
   eine Heuristik (Regel oder LLM) → `vorlaeufig`, Zwei-Signal-Pflicht. Der Nutzer entscheidet je
   Transaktion; nichts wird still übernommen.
5. **Ausgehende Bank-/OCR-Dienste = Julius-Wort.** Anbindung eines externen Kontoauszug-/OCR-
   Dienstes ist eine ausgehende Integration (Daten verlassen das Gerät) → eigener Julius-Cap,
   nicht autonom.

## Warum das die Doktrin bestätigt statt sie zu dehnen

Das Produkt hat von Anfang an „Herkunft je Wert" als Kern-Invariante (Herkunfts-Vektor, Zwei-Signal,
Justification-Kante). Beleg-Upload und Kontoauszug sind der Beweis, dass diese Invariante die
richtige war: neue Eingabequellen kosten eine neue `herkunft`-Kategorie + einen Extraktions-Writer,
KEINE Änderung an Engine, Registry, Store-Kern oder Bescheid-Ehrlichkeit.
