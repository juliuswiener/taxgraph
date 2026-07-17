# E-Nr-Kandidaten — Anlage R + Anlage Kind (Task #11)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** Kandidaten ZUR INSTRUCTOR-REVIEW — **kein Eintrag
in die Bindungstabelle vor OK** (Zitatanker-Doktrin auf Kz-Ebene). LLM-frei.
**Methode:** STRUKTUR-BEWUSSTER XSD-Parse (`elster/kz_extract.py`, E10-2025.html, 2242 Kz; Sektionspfad
aus den `$…/Vordruckzeile$`-Ankern), Vordruck-Zeile/Formularfeld aus dem gefreezten Vordruck
`sources/bfinv/anlage_r_2025.txt` / `anlage_kind_2025.txt` (4b32b9a). Kein Flat-Label-Grep.
**Fokus (Instructor):** p22_1 (Anlage R) + p24b/p31/p32 (Anlage Kind). Kohorten/berechnete Größen bleiben GAP.
**Konfidenz:** STRONG = Sektion+Label decken das Konzept verbatim, direkt eintragbar; MITTEL = Favorit
klar, aber Vordruck-Kontext (Renten-Art/Spalte/Format) klären; GAP = keine saubere E-Nr.

## Ergebnis (Instructor-Ruling 2026-07-17): KEIN Direkt-Eintrag — alle GAP/Nachtrag

Alle vier Kandidaten tragen echte Vorbehalte (Renten-Art-Dimension, Format-Brücke Jahr↔Datum,
Konzept-Mismatch) → **kein Rate-Eintrag** (ehrlicher als ein geratenes Default-Kz). Die verifizierten
E-Nr sind unten als Grundlage der zwei benannten Nachträge dokumentiert, NICHT eingetragen. Der
Deklarations-Drift-Wächter/Gate bleibt grün (keine neue non-null-Kz).

### Verifizierte Kandidaten (dokumentiert als Nachtrag-Grundlage, NICHT eingetragen)

| feld_id | Regel | Kandidat-E-Nr (je Renten-Art / Elternteil) | Sektion / Vordruck-Zeile | Ruling → Status |
|---|---|---|---|---|
| rentner_jahresrente | p22_1 | gesetzl **E1800301** / priv **E1801601** / sonst **E1803102** — „Rentenbetrag …" | /R/Leibr_{gesetzl,priv,sonst}/Einz, Anlage R Z4/13/19 (Feld 101/131/141) | **GAP** — Kz-Zuordnung hängt an `rentner_renten_art`-Enum (Nachtrag A); kein 1:1 ohne Art-Feld. |
| rentner_renten_beginn_jahr | p22_1 | gesetzl **E1800501** / priv **E1801701** / sonst **E1803202** — „Beginn der Rente" | /R/Leibr_*/Einz, Anlage R Z6/14/20 (Feld 103/132/142) | **GAP** — wie oben + Format-Brücke: Vordruck erwartet **Datum**, unser Feld ist **Jahr** (int). |
| fam_kinder_beruecksichtigt | p32_6 | **E0500807** „Art des Kindschaftsverhältnisses" (+ Zeitraum **E0500601**; A/B **E0500808/E0500805**) | [K_Verh_A/B] | **GAP** — strukturierte Per-Kind-Felder → Per-Kind-Kz-Modell (Nachtrag B). |
| fam_kinder_im_haushalt | p24b | **E0500702** „Anspruch auf Kindergeld …" | [Allg] | **GAP / ABGELEHNT** — Anspruch-Kindergeld ≠ Kind-im-Haushalt (Konzept-Mismatch, wie Scheibe 3). |

## Benannte Nachträge (Struktur-Zuwachs, eigene Runde)

- **Nachtrag A — `rentner_renten_art`-Enum:** neues askable Enum-Feld (gesetzlich / berufsständisch /
  private Basisrente …), das die Renten-Art führt; erst damit verzweigt die Kz-Zuordnung von
  rentner_jahresrente/renten_beginn_jahr auf das passende Kz-Trio (gesetzl E1800301/E1800501, priv
  E1801601/E1801701, sonst E1803102/E1803202). Bindungstabellen-Struktur-Zuwachs, kein Rate-Default.
  Plus Format-Brücke Jahr↔Datum für den Renten-Beginn.
- **Nachtrag B — Per-Kind-Kz-Modell:** die Anlage Kind ist je Kind instanziiert (Multiplikations-Klasse);
  Kindschaftsverhältnis (E0500807/E0500601), Kind-Identifikationsnummer u. a. sind Per-Kind-Felder.
  Unser Modell instanziiert (noch) nicht je Kind → fam_anzahl_kinder = Anzahl Anlagen Kind bleibt GAP,
  die Per-Kind-Kz warten auf dieses Modell.

## GAP bleibt — E-Nr nicht sauber resolvierbar (geschärfter Grund)

| feld_id / Konzept | Grund (geschärft) |
|---|---|
| besteuerungsanteil_prozent (p22_1) | **Kohorten-Parameter**, abgeleitet aus rentner_renten_beginn_jahr via § 22-Tabelle — kein Deklarations-Kz (Instructor-Vorgabe: bleibt GAP). |
| fam_anzahl_kinder (p24b/p32) | Zahl = **Anzahl der Anlagen Kind** (je Kind eine Anlage, Multiplikations-Klasse), kein Einzel-Zähl-Kz im E10. |
| fam_monate_ohne_voraussetzung (p24b) | Aus dem EfA-**Zeitraum** (E0503801 „im Zeitraum") abgeleitet; die Regel rechnet die vollen Kalendermonate, kein Monats-Kz. |
| Identifikationsnummer des Kindes | **Kein „Identifikationsnummer des Kindes"-Kz** in E10 gefunden (die IdNr-Felder betreffen Stpfl. A/B E0100081/82, empfangs-/unterstützte/pflegebedürftige Person). Per-Kind-Identität braucht das **Per-Kind-Kz-Modell** (Multiplikation) — unser Modell instanziiert (noch) nicht je Kind. |
| rentner_alter_64_erfuellt (p24a) | Altersentlastungsbetrag = **Mantelbogen** (Geburtsdatum), nicht Anlage R/Kind → außerhalb dieser Runde. |
| rentner_grad_der_behinderung / _hilflos_blind_taubblind / _hinterbliebenenbezuege / _pflegegrad / _gepflegter_hilflos (p33b) | **Anlage außergew. Belastungen**, nicht R/Kind. HINWEIS: die Kz existieren im E10 (Sektionen [Behind]/[Blind_Hilfl]/[Hinterbl]/[Ang_pflegebeduerft_Pers]) → sauber resolvierbar in einer **eigenen Kandidaten-Runde** nach dem Anlage-Behinderung-Freeze. |
| rentner_veraeusserungsgewinn (p16_4) | Betriebsveräußerung = **Anlage G/S**, nicht R/Kind → separate Runde. |
| fam_alleinstehend (p24b) | Bereits via est_mapping-NEGATION gemappt (E0503701/E0503821, EfA); hier nur Referenz, kein neuer Kandidat. |

## Ergebnis / Nächste Schritte (geruled)

- **Kein Eintrag in dieser Runde** — alle vier Kandidaten bleiben GAP mit geschärftem Grund; die
  Bindungstabelle wächst nicht (Gate/Drift-Wächter bleiben grün, keine neue non-null-Kz).
- **Zwei benannte Nachträge** (eigene Runde): (A) `rentner_renten_art`-Enum + Kz-Verzweigung + Jahr↔Datum-
  Brücke; (B) Per-Kind-Kz-Modell (Multiplikations-Instanziierung der Anlage Kind).
- **Weiter vorgemerkt:** p33b-Pauschbeträge (Sektionen [Behind]/[Blind_Hilfl]/[Hinterbl] existieren im
  E10) → eigene Kandidaten-Runde nach dem Anlage-Behinderung-Freeze.

Sauberes Ergebnis: die Anlage-R/Kind-Struktur ist reicher als unsere aktuellen Felder (Renten-Art-
Dimension, Per-Kind-Kz, Datum-vs-Jahr) — als benannte GAP/Nachträge geführt, kein Rate-Eintrag.
