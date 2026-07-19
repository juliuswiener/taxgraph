# Nachträge-Register — Gap-Überblick nach charge29/charge30/Weg-ii — dev-2, 2026-07-18

**Zweck:** ALLE benannten Gaps/Nachträge aus charge29 (Materialisierung §35a/§10b/§33/§10-KiSt/§33-Abs.3),
charge30 (§31/§24a/§24b) und Weg-ii (Abzug-Faltung in den gesamt-Ring) in EINEM priorisierten Register für
Julius. Systematisch aus Scheibe-Kommentaren (api.py), rules/estg-Modul-Kommentaren, den Materialisierungs-
Reports, dem inerte-Deklaration-Audit und commit-messages gesammelt (nicht nur die Instructor-Liste).

**K2-Richtung**: bei jedem Gap ist die heutige Wirkung fail-SAFE (über-besteuert / guard-gesperrt / bestätigte
Null) — KEIN stiller Unter-Abzug. Das Register ordnet nach (Häufigkeit × Impact), dann Aufwand.

## TIER 1 — HÄUFIG + niedriger Aufwand (verified Snapshot = billige Promotion, wie §35a)
| Gap | Norm | Auswirkung / Häufigkeit | Aufwand | Snapshot? | K2-Richtung |
|---|---|---|---|---|---|
| Altersvorsorge-Höchstbetrag/Günstiger | § 10 Abs. 1 Nr. 2 (p10_1_2) | HÄUFIG (jeder mit Basis-/Rürup-Vorsorge); heute nur roh in SA | niedrig (materialisieren + Accessor) | ✓ verified (Altersvorsorge) | über-besteuert (voller Abzug fehlt) = safe |
| GWG-Sofortabzug-Schwelle 800/1000 | § 6 Abs. 2 (p6_2) | MITTEL-HÄUFIG (AN/Betrieb mit GWG); Feld fließt, Formel inert | niedrig | ✓ verified (GwgSofortabzug) | Feld live, Sofort-vs-AfA-Wahl inert |
| Berufsausbildungskosten | § 10 Abs. 1 Nr. 7 (p10_1_7) | MITTEL (Erststudium/-ausbildung) | niedrig | ✓ verified (Berufsausbildungsaufwendungen) | über-besteuert = safe |
| Verbilligte Vermietung 66/50%-WK-Kürzung | § 21 Abs. 2 (p21_2) | MITTEL (Vermietung an Angehörige) | niedrig | ✓ verified (VerbilligteVermietungWk) | WK ungekürzt = unter-besteuert-RISIKO → Guard/Materialisierung nötig |

## TIER 2 — HÄUFIG, mittlerer Aufwand (Ring-/Kompositions-Erweiterung)
| Gap | Norm | Auswirkung / Häufigkeit | Aufwand | Snapshot? | K2-Richtung |
|---|---|---|---|---|---|
| Rentner MIT Arbeitslohn/Nebeneinkommen | § 22 + § 19/§ 21 (Kompositions-Lücke) | HÄUFIG (Rentner mit Minijob/V+V/Kapital); von keinem Ring VOLL modelliert | mittel-hoch (§22+§19-Ring-Komposition) | teils (§22-Module da) | fail-closed (kein voller Ring) = safe |
| Person-B-WK bei § 19-B-Zusammenveranlagung | § 19/§ 9 (Person-B) | HÄUFIG (Ehepaar beide AN mit WK); MVP Person-B-WK = 0 | mittel | — | unter-Abzug = über-besteuert = safe |
| Person-B § 35a/§ 10b/§ 33 bei zusammen (GdE=A+B) | § 35a/§ 10b/§ 33 | HÄUFIG (Ehepaar mit Abzügen); charge29-Nachtrag | mittel | Module da | Abzug fehlt Person-B = über-besteuert = safe |
| dHf/Verpflegung/Arbeitsmittel im Person-B-§19 | § 9 (Person-B) | MITTEL; keine Slots in der zusammen-Scheibe | mittel | — | über-besteuert = safe |

## TIER 3 — SELTENER oder konservativ-bindend (niedriger Aufwand)
| Gap | Norm | Auswirkung / Häufigkeit | Aufwand | Snapshot? | K2-Richtung |
|---|---|---|---|---|---|
| § 20-Günstiger-Kapital NICHT in § 10b/§ 33-GdE (GdE-Zwilling) | § 2 Abs. 3 + § 10b/§ 33 | SELTEN bindend (Kapital tariflich + Spende/agB am Deckel); Stage-1-Nachtrag | niedrig (GdE-Erweiterung) | n/a | GdE ohne Kapital → §10b/§33-Deckel etwas niedriger = über-besteuert = safe |
| § 24a-Bemessung ohne Kapitaleinkünfte | § 24a S. 2 (Stage-2) | SELTEN (Senior mit Kapital, §24a-relevant) | niedrig | Modul da | §24a etwas niedriger = über-besteuert = safe |
| § 16 Abs. 4 Betriebsveräußerungs-Freibetrag | § 16 Abs. 4 (p16_4) | SELTEN (Betriebsaufgabe/-verkauf) | niedrig | ✓ verified (BetriebsFreibetrag) | Freibetrag fehlt = über-besteuert = safe |
| § 16-Betriebsart land_forst / § 14 (kein Kz) | § 14/§ 16 | SELTEN; proven-absent Kz-Zweig | niedrig | — | GAP-Zweig → nicht_deklariert (kein Phantom) = safe |

## TIER 4 — NACHTRÄGE ohne Regel/Slot (Guard-gesperrt oder Multi-Jahres-State)
| Gap | Norm | Auswirkung / Häufigkeit | Aufwand | Snapshot? | K2-Richtung |
|---|---|---|---|---|---|
| Arbeitsmittel-AfA | § 9 Abs. 1 Nr. 6/7 | HÄUFIG (jeder AN mit Arbeitsmitteln); heute nur K2-Guard | HOCH (Voll-Formalisierung + Pipeline-Lauf = Julius-Cap) | KEINER | Guard sperrt (am_nicht_ring_faehig) = fail-closed safe |
| § 10 Abs. 4b KiSt-Erstattungsüberhang-Hinzurechnung | § 10 Abs. 4b S. 3 | SELTEN (Erstattung > Zahlung); kein p32a-Slot | mittel (Regel + p32a-hinzurechnung-Slot) | nein | Guard sperrt (erstattungsueberhang_offen) = fail-closed safe |
| § 35a Abs. 5 S. 4 haushaltsbezogener Höchstbetrag (zwei Alleinstehende) + Abs. 2 S. 2 Pflege/Heim | § 35a Abs. 5/2 | MITTEL (WG/Heim-Fälle) | mittel | nein | Höchstbetrag ungeteilt = unter-besteuert-RISIKO bei zwei Alleinstehenden → prüfen |
| § 10b 4-‰-Umsatz-Alternative + Großspenden-Vortrag | § 10b Abs. 1/1a | SELTEN (Großspender/Unternehmer) | mittel | nein | nur 20%-GdE-Deckel → alternativer Deckel fehlt = über-besteuert = safe |
| Kapital-Co-Okkurrenz E0121709 + Aktien/Sonstige-Töpfe | § 20/§ 32d | SELTEN (Aggregat + Töpfe zugleich) | mittel | — | Guard sperrt (kapital_semantik_offen) = fail-closed safe |
| § 10d Verlustvortrag (Multi-Jahres-State) | § 10d Abs. 4 | MITTEL (Verlustjahr → Folgejahr) | HOCH (VZ-übergreifender State) | — | kein Vortrag = über-besteuert im Folgejahr = safe |
| § 35c 40 000-Objekt-Lebensdauer-Deckel (Multi-Jahres) | § 35c (§ 2/VZ-State) | SELTEN (energetische Sanierung mehrjährig) | HOCH (Multi-Jahres-State) | — | Objektdeckel fehlt = unter-besteuert-RISIKO bei Mehrjahres-Sanierung → prüfen |

## TIER 5 — Test-Coverage / UI-Politur / Feature (kein Rechen-Gap)
| Gap | Art | Auswirkung | Aufwand | K2-Richtung |
|---|---|---|---|---|
| § 31-2-Tarif-Günstiger als golden-Case-Type | Test-Coverage (dev-1-Runner) | Günstiger-ENTSCHEID nicht als Golden (Accessor verifiziert, Komposition-Zweig getestet) | niedrig | — |
| Opt-in-Gate-UX (bestätigte-Null statt absent→0) | UI-Politur (dev-1-Haut) | Laie-Ja/Nein-Frage statt still-optional | niedrig | fail-safe (absent→0) bleibt korrekt |
| Kontoauszug-PDF-Import (nur CSV/CAMT heute) | Feature (Beleg-Writer) | PDF-Auszug nicht parsbar | mittel (LLM-Fallback) | — |
| Kontoauszug-LLM-Recorded-Fixture | Test-Gate (Julius-Cap) | 1 Live-Call zum Aufzeichnen offen | niedrig (Julius-Cap) | — |

## TIER 6 — dev-1-Bughunt 2026-07-19 (nach §10-Vorsorge-Faltung, 4 neue Ring/Haut-Gaps)
Von dev-1s Bughunt im gesamt-Ring gefunden. Richtungs-Tag = K2-Wirkung heute.
| Gap | Norm | Richtung | Auswirkung / Häufigkeit | Aufwand | Owner / Status |
|---|---|---|---|---|---|
| **A** Person-B-Vorsorge nicht abgezogen bei Zusammenveranlagung (VOR §10 Abs.1 Nr.2 / KV-PV Nr.3/3a / §24a sind einzel-only im gesamt-Kegel) | § 10 Abs. 1 Nr. 2/3/3a, § 24a | **ÜBER-tax** (still, ungesperrt) | HÄUFIG (Ehepaar zusammen, beide mit Vorsorge → Person-B-Abzug fehlt) | A.1 niedrig (Guard-Sperre) · A.2 mittel (Person-B-Vorsorge-Kegel, Klasse g) | dev-1 baut A.1-Guard JETZT; voller Fix A.2 = eigene Front |
| **B** Ehegatte-sonstige-Kapitalgewinn nicht erfassbar bei zusammen (runner-Accessor Z.533 hart 0) | § 20 Abs. 2 | **UNTER-tax** (Modell-Mismatch) | MITTEL (Ehepaar, Ehegatte-Veräußerungsgewinn) | mittel (Person-B-§20-Abs.2-Erfassung: dev-2-Binding + dev-1-runner) | offen |
| **C** § 21-Verbilligt-Accessor: entgelt_quote=0-`or 100`-Falle + fehlende Einkünfteerzielungsabsicht | § 21 Abs. 2 / Abs. 1 | **UNTER-tax** (§21-Fix-DEFEKT) | quote=0 (voll unentgeltlich) → als 100 behandelt → WK voll statt 0; EEA nicht geprüft | niedrig (`or 100`→None-Check) + mittel (EEA-Gate) | dev-1-runner (Accessor-Defekt im §21-Fix) |
| **D** § 24b bei veranlagung=zusammen + fam_alleinstehend=True ohne Konsistenz-Sperre | § 24b Abs. 1 | **UNTER-tax** (Widerspruch ungesperrt) | SELTEN (widersprüchliche Eingabe zusammen+alleinstehend) | niedrig (Konsistenz-Guard) | dev-2-Zone (produkt/konsistenz partner_check) |
| **E** § 10 Abs. 3 S. 3 RV-Höchstbetrag-Verdopplung bei zusammen (~27.566 → 55.132 €) nicht abgebildet (A+B-RV in EINEN Slot) | § 10 Abs. 3 S. 2 | **ÜBER-tax** (fail-safe) | SELTEN (Hoch-RV-Paare > 27.566 € gemeinsam) → leicht unter-abgezogen | mittel (per-Person-RV-HB) | runner.py-_vorsorge_abzug-Zone (dev-1), shared/golden-Impact = später |

**STATUS TIER 6 (2026-07-19):** A GESCHLOSSEN (A.1-Guard + A.2 Person-B-Vorsorge live d4ceaa1). C+D GESCHLOSSEN (§21-quote0-Fix + §24b-Konsistenz-Sperre, committet addce7c). E = neuer A.2-Residual (über-tax, selten, später). B (§20-Abs.2-Ehegatte-Kapitalgewinn) = offen, mittlere Front.
**Prio-Historie:** C+D zuerst (UNTER-tax) → erledigt; A.2 (über-tax häufig) → erledigt; E (über-tax selten) + B (unter-tax mittel) verbleibend.

## ⚠ K2-PRÜFAUFTRÄGE (die 2 unter-besteuert-RISIKO-Kandidaten)
Die meisten Gaps sind fail-safe (über-besteuert/guard-gesperrt). ZWEI verdienen K2-Prüfung, ob heute still
unter-besteuert wird:
1. **§ 21 Abs. 2 verbilligte Vermietung**: wird die WK-Kürzung (bei < 66/50% Entgelt) heute angewandt oder
   fließen die WK ungekürzt? Falls ungekürzt → unter-besteuert. (verified Snapshot vorhanden → Promotion.)
2. **§ 35a Abs. 5 S. 4** (zwei Alleinstehende, ein Haushalt): wird der Höchstbetrag geteilt oder doppelt
   gewährt? Falls doppelt → unter-besteuert. (kein Snapshot → prüfen/formalisieren.)

**STATUS 2026-07-19 (beide GEKLÄRT):** (1) § 21 Abs. 2 = Under-Tax bestätigt + FIX materialisiert/committet
(797fd60, WK-Kürzung greift) — ABER neuer Folge-Defekt TIER-6-C (entgelt_quote=0-`or 100`-Falle) offen.
(2) § 35a Abs. 5 S. 4 = dokumentierter NICHT-Gap (Cross-Erklärungs-Koordination, im Einzel-Modell strukturell
nicht erreichbar; Befund 2026-07-18-k2-35a-abs5-s4-befund.md). NEUE UNTER-tax-Kandidaten = TIER-6 B/C/D.

## EMPFEHLUNG (Reihenfolge)
1. **Tier-1-Promotionen** (billig, verified): §10-Vorsorge (häufig) + GWG + Berufsausbildung + §21-verbilligt.
2. **K2-Prüfaufträge** (§21 Abs.2 + §35a Abs.5): erst prüfen ob unter-besteuert, dann priorisieren.
3. **Tier-2 Person-B/Rentner-Komposition** (häufig, mittel): Ehepaar-WK + Rentner-mit-Nebeneinkommen.
4. **Tier-4-Nachträge** nach Bedarf (die meisten selten/fail-safe).
Repeated-Instance-Trias (Multi-Objekt/Per-Kind/Multi-Rente) = ERLEDIGT, nicht mehr Gap.
