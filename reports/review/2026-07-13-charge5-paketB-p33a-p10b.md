# Charge-5 Paket B — Zuschnitte § 33a + § 10b

Vollabdeckung Charge 5 (schließt weitere 🟡-Lücken: agB-Unterhalt/Ausbildung, SA-Spenden).
Stufe A, $0. Freezes vom Instructor (sha verifiziert): `estg_p33a_2026-07-13.txt` (26790adc),
`estg_p10b_2026-07-13.txt` (230fac4c).

---

## § 33a — Unterhalt (Abs. 1) + Ausbildungsfreibetrag (Abs. 2)

Vorschlag: **zwei Teilregeln** (wie § 33b-Split; Abs. 1 gedeckelter Abzug, Abs. 2 Festbetrag).

### (A) p33a_unterhalt — Abs. 1 (gedeckelter Unterhaltsabzug)

Wortlaut Abs. 1: Aufwendungen für Unterhalt einer gesetzlich unterhaltsberechtigten Person,
abziehbar **bis zur Höhe des Grundfreibetrags (§ 32a Abs. 1 S. 2 Nr. 1)** (S. 1); Höchstbetrag
**erhöht um KV/PV-Beiträge** für die Person (S. 2); **gemindert um** (eigene Einkünfte + Bezüge
der unterhaltenen Person **− 624 €**), soweit positiv (S. 5).

Rechenkern:
```
hoechstbetrag = grundfreibetrag + kv_pv_beitraege - max(0, andere_einkuenfte_bezuege - 624)
unterhalt_abzug = min(aufwendungen, max(0, hoechstbetrag))
```
Signatur: `aufwendungen money, grundfreibetrag money, kv_pv_beitraege money,
andere_einkuenfte_bezuege money -> unterhalt_abzug money`.

Andockung: `grundfreibetrag` kommt als Input (§ 32a Abs. 1 S. 2 Nr. 1, VZ-versioniert 2026:
12.348; params/2026 oder § 2-Integration — wie § 32b-Tarifwert). Der 624-€-Anrechnungsfreibetrag
ist § 33a-eigener Wortlaut-Konstant (S. 5). `andere_einkuenfte_bezuege` kommt bereits als
Summe/netto herein (S. 5 zählt die Bezüge auf → § 2-Integration).

hinweis-Kandidat (Konditional-VORRANG/Netto, Leitlinie): (1) die Anrechnung nur soweit
Eink.+Bezüge > 624 (max(0, …)); (2) grundfreibetrag/andere_einkuenfte_bezuege kommen als Inputs,
nicht selbst nachschlagen/filtern. Vorbelegen.

Wächter-Seeds (Grundfreibetrag 2026 = 12.348):
- Standard: aufw. 10.000, gfb 12.348, kv_pv 0, andere 0 → hoechst 12.348, min(10.000; 12.348) = **10.000,00**.
- Deckelung: aufw. 15.000, gfb 12.348, kv_pv 0, andere 0 → **12.348,00**.
- Anrechnung: aufw. 15.000, gfb 12.348, kv_pv 0, andere 2.000 → hoechst 12.348 − (2.000−624)=10.972 → **10.972,00**.
- KV/PV-Erhöhung: aufw. 15.000, gfb 12.348, kv_pv 1.500, andere 0 → hoechst 13.848 → **13.848,00**.
- Anrechnung unter Freibetrag: andere 500 (< 624) → keine Minderung → hoechst 12.348.

### (B) p33a_ausbildungsfreibetrag — Abs. 2 (Festbetrag 1.200)

Wortlaut Abs. 2 S. 1: „einen Freibetrag in Höhe von 1 200 Euro je Kalenderjahr" für ein
„sich in Berufsausbildung befindenden, auswärtig untergebrachten, volljährigen Kindes" mit
Kindergeld-/Freibetrag-Anspruch. Signatur: `hat_anspruch bool -> 1.200 sonst 0` (Rechtsprädikat,
wie Hinterbliebenen). Seeds: true → 1.200, false → 0.

Scope-Grenzen (Geltungsbedingungen): Abs. 3 Zwölftelung (Monate ohne Voraussetzungen, +Aufrundung
auf vollen Euro) + Halbteilung Eltern (Abs. 2 S. 4) = § 2-Integration/Sonderfall, nicht Grundbetrag.
Abs. 4 (kein § 33 daneben) = Anwendungskonkurrenz, § 2-Integration.

---

## § 10b — Spenden/Mitgliedsbeiträge (Abs. 1, 20-%-GdE-Deckel)

Wortlaut Abs. 1 S. 1: „Zuwendungen (Spenden und Mitgliedsbeiträge) zur Förderung
steuerbegünstigter Zwecke … bis zu 1. **20 Prozent des Gesamtbetrags der Einkünfte** oder 2.
4 Promille der Summe der gesamten Umsätze und der … Löhne …".

Rechenkern (AN-nah): `spenden_abzug = min(zuwendungen, 0,20 × gesamtbetrag_der_einkuenfte)`.
Signatur: `zuwendungen money, gesamtbetrag_der_einkuenfte money -> spenden_abzug money`.
Präzision: `0,20 × GdE` in decimal, Cent-Schnitt zuletzt (praezisions_lint greift).

Scope-Grenzen (dokumentiert):
- **4-‰-Umsatz-Alternative** (Nr. 2) = Betriebs-/Unternehmerfall, außerhalb AN-nah → Scope-Note,
  nicht formalisiert (der AN-Fall nutzt die 20-%-GdE-Grenze).
- **Vortrag** (S. 9: Überhang in Folge-VZ) = mehrjährig/§ 10d-Verweis → § 2-Integration-Backlog.
- **Abs. 1a Stiftungs-Vermögensstock** (1 Mio/2 Mio, 10-Jahres-Sonderkontingent) = eigener
  Sonderzuschnitt (Backlog).
- **Abs. 2 Parteispenden** (3.300/6.600) = § 34g-Territorium (Steuerermäßigung, nicht SA-Abzug) →
  eigener Zuschnitt, nicht hier.
- **Abs. 3/4** (Sachzuwendungen-Bewertung, Vertrauensschutz/Haftung) = Verfahren/Bewertung,
  nicht der Grundabzug.

Wächter-Seeds: Standard zuw. 1.000, GdE 50.000 → 20 % = 10.000, min → **1.000,00**; Deckelung
zuw. 15.000, GdE 50.000 → **10.000,00** (20-%-Grenze); GdE 0 → 0.

hinweis-Bedarf: der Kern ist eine einfache min-Prozent-Deckelung (Formel, kein Konditional-
Vorrang) → nach der Leitlinie eher selbsttragend; ggf. Präzisions-Idiom-hinweis vorsorglich
(0,20 in decimal). Erst Lauf, dann bei Bedarf.

---

## Nächste Schritte
1. Instructor-Review (Signaturen, § 33a-2er-Split, Scope-Grenzen § 10b).
2. Nach Freigabe: Signaturen + Seeds in `rules.yaml`; grundfreibetrag als Input/params-Andockung.
3. Stufe B (Freigabe je Lauf). § 33a-Unterhalt hat Konditionale (Anrechnung, KV/PV) → hinweis
   vorbelegen; § 10b + Ausbildungsfreibetrag voraussichtlich selbsttragend.
4. Landkarte: § 33a Abs. 1 → agB 4/4; § 33a Abs. 2 → Kind (Ausbildungsfreibetrag) ✅; § 10b → SA.
   Danach Paket C (§ 10 Nr. 5 Kinderbetreuung + § 10 Abs. 1a Realsplitting, aus Bestands-§10-Freeze).
