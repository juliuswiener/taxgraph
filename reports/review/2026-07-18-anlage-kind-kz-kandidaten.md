# Anlage-Kind Kz-Kandidaten-Matrix (Per-Kind-Konsument, read-only Recon) — dev-2 2026-07-18

**Zweck:** Kz-Review-Material für den Per-Kind-Instanz-Konsumenten. KEIN Blind-Bind — Instructor
adjudiziert je Kz (Anker VORAB, Zitatanker-Doktrin auf Kz-Ebene, [[kz-block-disambiguierung-personA-B]]).
Quelle: E10-2025.html Schemadok (ERiC 44.2.4.0) + amtlicher Vordruck Anlage Kind 2025 (sources/bfinv/kind_2025.txt).

## Kandidaten-Matrix
| # | Kz | XSD-Label | Sektions-Pfad (Hash) | Vordruck-Anlage-Kind-Zeile | geplantes Feld (typ) | Konfidenz |
|---|---|---|---|---|---|---|
| 1 | E0500406 | "Identifikationsnummer" | `Allg` (m1285501656) | Zeile 4, Kennziffer 01 | kind_idnr (text) | STARK |
| 2 | E0500807 | "Art des Kindschaftsverhältnisses" | `K_Verh_A` (m1388229944) | Zeile 56 (Elternteil A) | kind_kindschaftsverhaeltnis_a (text/enum) | STARK |
| 3 | E0500808 | "Art des Kindschaftsverhältnisses" | `K_Verh_B` (m1388229944) | Zeile 56 (Elternteil B) | kind_kindschaftsverhaeltnis_b (text/enum) | STARK |
| 4 | E0500601 | "Kindschaftsverhältnis bestand vom - bis" | `K_Verh_A` (m1388229944) | Zeile 53 (Zeitraum vom–bis, Elternteil A) | kind_kindschaftsverh_zeitraum_a (text/datum) | STARK |
| 5 | E0500805 | "Kindschaftsverhältnis bestand vom - bis" | `K_Verh_B` (m1388229944) | Zeile 53 (Elternteil B) | kind_kindschaftsverh_zeitraum_b (text/datum) | STARK |
| — | E0500702 | "Anspruch auf Kindergeld oder vergleichbare Leistungen für $VZ$" | `Allg` (m1285501656) | — | **ABGELEHNT, bleibt** (≠Haushaltszugehörigkeit, [[vier-pakete-sequenz-2026-07]]) | n/a |

## Disambiguierungs-Beleg (Sektions-Pfad, NICHT E-Präfix)
- **E0500807 vs E0500808** tragen IDENTISCHES XSD-Label "Art des Kindschaftsverhältnisses" — getrennt AUSSCHLIESSLICH
  durch Sektion `K_Verh_A` vs `K_Verh_B` (Elternteil A/B, gleicher Hash m1388229944 = das Zwei-Elternteil-Spalten-
  Konstrukt der Anlage Kind). Textbook der kz-block-Lehre.
- **E0500601 vs E0500805** analog (identisches Label "…bestand vom - bis", K_Verh_A/B).
- **Abgrenzung Zeitraum:** E0500601/805 (K_Verh, Vordruck-Zeile 53 = Kindschaftsverhältnis-Zeitraum) sind DISTINKT
  von den Berücksichtigungs-Zeiträumen (Vordruck-Zeilen 16/18/20, Kz 80/81/82) — der Sektions-Pfad K_Verh grenzt ab,
  das bloße Label "Zeitraum vom-bis" wäre vordruck-mehrdeutig.
- E0500406 (IdNr) + E0500702 (Kindergeld) beide in `Allg` (Kind-Allgemein-Block, m1285501656).

## Struktur-Hinweis für den Bau (nach Adjudikation)
Per-Kind trägt ZWEI Achsen: (a) Kind-Instanz (instanz_gruppe:kind, Kind 1=Basis / Kind 2..N=__n) × (b) Elternteil
A/B (zwei distinkte Basis-Kz je Konzept: _a→E0500807/E0500601, _b→E0500808/E0500805). Die A/B-Achse = zwei separate
1:1-Felder (kein neuer Mechanismus); die Kind-Achse = der Instanz-Kern. Je Kind also 5 Felder (IdNr + 2×Kindschafts-
verh.-Art + 2×Zeitraum), je Kind-Instanz Kz-Reuse. Tarif-neutral (count-MVP bleibt), Ring-neutral.

## Anker-Quellen VERIFIZIERT (build-ready, alle in sources/gesetze-im-internet/estg_p32_2026-07-11.txt)
- **kind_idnr (E0500406)** → § 32 Abs. 6 S. 12 EStG, zitatanker: "Identifizierung des Kindes durch die an dieses
  Kind vergebene Identifikationsnummer" (referenziert § 139b AO im Text — keine separate AO-Quelldatei nötig).
- **kind_kindschaftsverhaeltnis_a/b (E0500807/808)** → § 32 Abs. 1 EStG, zitatanker: "im ersten Grad mit dem
  Steuerpflichtigen verwandte Kinder" (Pflegekind-Fall im selben Absatz).
- **kind_kindschaftsverh_zeitraum_a/b (E0500601/805)** → § 32 Abs. 2 EStG, zitatanker: "Besteht bei einem
  angenommenen Kind das Kindschaftsverhältnis zu den leiblichen Eltern weiter" (Bestehen des Kindschaftsverh.).
Alle vier Zitatanker voll-Länge gegen die Quelldatei prüfbar ([[anker-verifikation-volllaenge]]); im Bau via
_normalize verifiziert.

## Zur Adjudikation
Je Kz: bestätigen (STARK, Anker-vorab) oder ablehnen. Nach GO: taggen instanz_gruppe:kind + 5 Felder binden +
est_mapping-Instanz (reuse Multi-Objekt-Muster, reines 1:1 je Instanz) + 2-Kind-Roundtrip/Drift/fail-closed-Tests.
