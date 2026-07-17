# E-Nr-Nachträge — Kandidaten-Tabellen zur Review (Task #11)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** Kandidaten ZUR INSTRUCTOR-REVIEW — **kein Eintrag
in die Bindungstabelle vor OK** (Zitatanker-Doktrin auf Kz-Ebene). LLM-frei.
**Methode:** STRUKTUR-BEWUSSTER XSD-Parse (`kz_extract`, Sektions-Pfad + wörtliches Label), kein
Flat-Label-Grep. E10-2025 (2242 Kz). Konfidenz: STRONG = Sektion+Label decken das Konzept verbatim;
MITTEL = plausibel, aber Vordruck-Zeilen-Cross-Check empfohlen; GAP = keine saubere E-Nr (bleibt Lücke).

## STRONG — Favorit, nach OK direkt eintragbar

| feld_id | Favorit E-Nr | Sektion | wörtliches Label | Begründung |
|---|---|---|---|---|
| kap_kapitalertraege | **E0121709** | KapV | „Kapitalerträge (Abgeltungsteuer)" | direktes Konzept-Match (§ 20 laufende KapErträge). |
| kap_gewinn_aktien | **E1900901** | Betr_lt_StBesch | „In Zeile … enthaltene Gewinne aus Aktienveräußerungen" | verbatim Aktien-Gewinn-Topf. |
| kap_verlust_aktien | **E1901301** | Betr_lt_StBesch | „Nicht ausgeglichene Verluste aus der Veräußerung von Aktien" | verbatim Aktien-Verlust-Topf. |
| kap_verlust_sonstige | **E1901201** | Betr_lt_StBesch | „Nicht ausgeglichene Verluste ohne Verluste aus der Veräußerung von Aktien" | verbatim sonstiger Verlust-Topf (= ohne Aktien). |
| vv_einnahmen | **E0700201** | Einz (Anlage V) | „Mieteinnahmen" | direktes Konzept-Match. |

## MITTEL — Kandidat, Vordruck-Zeilen-Cross-Check empfohlen

| feld_id | Kandidat E-Nr | Sektion | wörtliches Label | Vorbehalt |
|---|---|---|---|---|
| vv_werbungskosten | E0703838 | Einz (Anlage V) | „Abzugsfähige Werbungskosten" | mehrere WK-Summenfelder je Objekt-Kontext (E0703838/E0703911) — Zeilen-Zuordnung prüfen. |
| vv_gebaeude_afa | E0703304 | Direkt | „Absetzung für Abnutzung wie Vorjahr / laut Erläuterung" | „wie Vorjahr"-Variante; der AfA-Betrags-Kz kann abweichen (mehrere AfA-Zeilen). |
| fam_alleinstehend | E0503701 / E0503821 | EfA | „…war(en) … volljährige Person(en) gemeldet, für die … kein Anspruch auf Kindergeld …" | **INVERSE Logik**: das Feld markiert NICHT-alleinstehend (schädliche Haushaltsgemeinschaft). Unser Slot `alleinstehend` ist die Negation → Mapping-Richtung im OK klären. |
| fam_kinder_im_haushalt | E0500702 | Allg | „Anspruch auf Kindergeld oder vergleichbare Leistungen für VZ" | je Kind-Anlage; Konzept passt, Zeilen-Zuordnung prüfen. |

## GAP bleibt — E-Nr nicht sauber resolvierbar (geschärfter Grund)

| feld_id | Grund (geschärft) |
|---|---|
| kist_gezahlt / kist_erstattet | Anlage Sonderausgaben KiSt: „gezahlt"/„erstattet" ist **Spalten-Kontext** (Form-Kz 103/104), nicht Teil des XSD-Feld-Labels — Flat-Grep findet nur Lohnsteuer-KiSt (E0200501) und KiSt-auf-KapErträge (E0100009). **Vordruck-Zeilen↔XSD-Sequenz-Cross-Check nötig** (wie Anlage-N-€-Summen), kein Rate-Mapping. |
| berufsausbildung_aufwendungen | Kein XSD-Label „Aufwendungen für die eigene Berufsausbildung"; die Anlage-Sonderausgaben-Zeile trägt disambiguierende Vordruck-Wörter nicht im Feld-Label → Vordruck-Cross-Check nötig. |
| kap_gewinn_sonstige | **Modell-Mismatch:** unser p20_6-Zuschnitt trennt 4 Töpfe (Aktien/Sonstige × Gewinn/Verlust); der Vordruck deklariert die **Summe** der Kapitalerträge (E0121709) + „darin enthaltene Aktiengewinne" (E1900901). Der sonstige Gewinn ist DERIVIERT (Summe − Aktien), kein eigenes Deklarationsfeld. |
| kap_zusammenveranlagung | Globale Veranlagungsart (Mantelbogen), kein KAP-spezifisches Kz. |
| fam_anzahl_kinder | Zahl = **Anzahl der Kind-Anlagen** (je Kind eine Anlage), kein Einzel-Zähl-Kz im E10. |
| fam_monate_ohne_voraussetzung | Aus dem EfA-**Zeitraum** (E0503801 „im Zeitraum") abgeleitet; die Regel rechnet die vollen Kalendermonate, kein direkter Monats-Kz. |
| vv_schuldzinsen / vv_erhaltungsaufwand | Anlage V ist zeilen-spezifisch (viele Kz); „Schuldzinsen"/„Erhaltungsaufwand" tauchen nicht als eigenes Feld-Label auf → Vordruck-Zeilen-Cross-Check nötig, sonst Rauschen. |
| vv_entgelt_quote_prozent | Keine Deklarations-Kz; die Entgelt-/Marktmiete-Quote wird gerechnet (66/50-%-Prüfung), nicht als Prozent-Feld deklariert. |

## Vorschlag zum Vorgehen (nach deinem Review)

1. **STRONG (5):** nach deinem OK trage ich die Favoriten ein; Gate (c) beweist sie gegen E10-2025.
2. **MITTEL (4):** deine Richtungsentscheidung (v.a. fam_alleinstehend Inverse-Logik + vv-AfA/WK-Zeile);
   optional ziehe ich den Anlage-V-/Kind-Vordruck (falls exportiert) für den Zeilen-Cross-Check.
3. **GAP (8):** ich schärfe die `elster_kz_grund`-Texte in der Bindungstabelle entsprechend (kein Kz),
   damit die Lücken benannt-präzise bleiben — das ist additiv und braucht kein Kz-Review.

Kein Eintrag erfolgt vor deinem Wort. Rohbefehl: `ERIC_DIR=~/02_Software/eric python3 elster/kz_extract.py --sektion KapV` etc.

## Umsetzung (nach Instructor-Review msg 2437)

**EINGETRAGEN (5 STRONG, Gate c beweist — alle in E10-2025 verifiziert):**
kap_kapitalertraege=**E0121709**, kap_gewinn_aktien=**E1900901** (Subset-Semantik in hilfe notiert:
Aktiengewinne sind als Teilmenge der Summen-Zeile deklariert), kap_verlust_aktien=**E1901301**,
kap_verlust_sonstige=**E1901201**, vv_einnahmen=**E0700201**. `elster_kz_grund` bei diesen entfernt.

**GAP mit geschärftem Grund (statt Eintrag):**
- vv_werbungskosten / vv_gebaeude_afa → Anlage-V-Vordruck-Freeze abwarten (Zeilen-Cross-Check).
- fam_alleinstehend → Vordruck kodiert die Negation (schädliche Haushaltsgemeinschaft), Übersetzung in
  die est_mapping-Schicht, kein Direkt-Kz.
- fam_kinder_im_haushalt → „Anspruch auf Kindergeld" (E0500702) ≠ „Kind im Haushalt".
- kap_gewinn_sonstige (Modell-Mismatch, deriviert), kap_zusammenveranlagung (global), fam_anzahl_kinder
  (=Anzahl Kind-Anlagen), fam_monate_ohne_voraussetzung (aus EfA-Zeitraum), vv_schuldzinsen/
  vv_erhaltungsaufwand/vv_sonstige_wk (Anlage-V zeilen-spezifisch), vv_entgelt_quote_prozent (berechnet),
  kist_gezahlt/erstattet + berufsausbildung_aufwendungen (Spalten-Kontext / kein XSD-Label).

Gates: 54/54 grün (inkl. Gate c über die 5 neuen Kz). Offener Folge-Posten: Anlage-V-Zeilen-Cross-Check
für vv_werbungskosten + vv_gebaeude_afa (Freezes seit 94ded2a da) → separater Kandidaten-Nachtrag.
