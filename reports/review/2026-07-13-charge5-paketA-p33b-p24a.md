# Charge-5 Paket A — Zuschnitte § 33b + § 24a

Vollabdeckung Charge 5 (Pauschbeträge/Familie-Rest, schließt 🟡-Lücken). Stufe A, $0.
Freezes vom Instructor (sha verifiziert): `estg_p33b_2026-07-13.txt` (22bd08a9),
`estg_p24a_2026-07-13.txt` (6e426c9f). Zwei neue/bestätigte Muster: **Tabellen-Lookup**
(§ 33b, p33_3-Präzedenz) und **Kohorten-versionierte Parameter** (§ 24a, NEU).

---

## § 33b — Behinderten-/Hinterbliebenen-/Pflege-Pauschbeträge

Der Paragraph trägt DREI Pauschbeträge mit eigenen Tabellen. Vorschlag: **drei Teilregeln**
(sauberer als ein Multi-Output-Monolith; jede Tabelle ein eigener deterministischer clerk-Test).

### (A) p33b_behinderten_pauschbetrag — GdB-Tabelle (primär)

Wortlaut Abs. 3 S. 2: „Als Pauschbetrag werden gewährt bei einem Grad der Behinderung von
mindestens:" + Staffel. Abs. 3 S. 3: hilflos/blind/taubblind → 7.400 € (statt der Staffel).

Tabelle (GdB ≥ Schwelle → Pauschbetrag):
| GdB | € | GdB | € |
|---|---|---|---|
| 20 | 384 | 70 | 1.780 |
| 30 | 620 | 80 | 2.120 |
| 40 | 860 | 90 | 2.460 |
| 50 | 1.140 | 100 | 2.840 |
| 60 | 1.440 | hilflos/blind/taubblind | **7.400** |

Signatur:
```
inputs:  grad_der_behinderung  integer   # festgestellter GdB (0..100)
         ist_hilflos_blind_taubblind  boolean  # Abs. 3 S. 3 (Override)
output:  behinderten_pauschbetrag  money
logik:   if ist_hilflos_blind_taubblind then 7.400
         else Tabellen-Lookup (größte Schwelle <= GdB; < 20 -> 0)
```
Muster: **tabellen_lookup** wie p33_3 (zumutbare Belastung). Die Schwellen-Halb­offenheit
(„von mindestens") = Stufenfunktion, GdB unter 20 → 0.

Wächter-Seeds (Rechenweg = Wortlaut-Tabelle): GdB 20 → 384 (untere Grenze); GdB 45 → 860
(Stufe 40, nicht 50 — „von mindestens", nächstniedrigere Schwelle); GdB 100 → 2.840;
hilflos (GdB egal) → 7.400; GdB 19 → 0 (unter Mindest-GdB).

### (B) p33b_pflege_pauschbetrag — Pflegegrad-Tabelle

Wortlaut Abs. 6 S. 3: „Als Pflege-Pauschbetrag wird gewährt: 1. bei Pflegegrad 2 600 Euro,
2. bei Pflegegrad 3 1 100 Euro, 3. bei Pflegegrad 4 oder 5 1 800 Euro." + S. 4: hilflos → 1.800.
Signatur: `pflegegrad integer + ist_hilflos boolean → pflege_pauschbetrag money`. Lookup
(PG2→600, PG3→1100, PG4/5→1800; hilflos→1800; PG<2→0). Seeds: PG2→600, PG3→1100, PG5→1800,
PG1→0, hilflos→1800.

### (C) p33b_hinterbliebenen_pauschbetrag — Festbetrag

Abs. 4 S. 1: „einen Pauschbetrag von 370 Euro (Hinterbliebenen-Pauschbetrag)". Trivial-Festbetrag
bei Anspruch. Signatur: `hat_hinterbliebenenbezuege boolean → 370 sonst 0`. Ein Seed je Zweig.

Scope-Grenzen (Geltungsbedingungen, dokumentiert): die Übertragung auf Eltern (Abs. 5,
Kind-Fall) + die Aufteilung bei mehreren Pflegepersonen (Abs. 6 S. 9) sind § 2-Integration/
Sonderfälle — nicht Teil des Grundbetrags-Lookups (wie bei p32_6/p31 die Aufteilungslogik).

---

## § 24a — Altersentlastungsbetrag (NEUES Muster: Kohorten-versionierte Parameter)

Wortlaut S. 1: „Der Altersentlastungsbetrag ist bis zu einem Höchstbetrag im Kalenderjahr ein
nach einem Prozentsatz ermittelter Betrag des Arbeitslohns und der positiven Summe der
Einkünfte, die nicht solche aus nichtselbständiger Arbeit sind." S. 5: „Der maßgebende
Prozentsatz und der Höchstbetrag … sind der nachstehenden Tabelle zu entnehmen".

Rechenkern: `min(prozentsatz × (arbeitslohn + positive_andere_einkuenfte), hoechstbetrag)`.

### Das neue Muster — Parameter je KOHORTE, nicht je VZ
Prozentsatz + Höchstbetrag hängen am **„auf die Vollendung des 64. Lebensjahres folgenden
Kalenderjahr"** (S. 5-Tabelle) — der Kohorte. Diese ist **lebenslang fixiert** (der Wert von
2026 gilt für diese Kohorte in JEDEM künftigen VZ), anders als der § 32a-Tarif (VZ-versioniert).
→ Parameter-Struktur wie p32a-`tariff_parameters`, aber Schlüssel = **massgebendes_folgejahr**
(Kohorte), NICHT veranlagungszeitraum. Sauber trennen, sonst Falschzuordnung.

Tabellen-Auszug (Folgejahr → % / Höchst): 2005 → 40,0 / 1.900 · 2006 → 38,4 / 1.824 · … ·
2025 → 13,2 / 627 · 2026 → 12,8 / 608 · … · 2040 → 7,2 / 342 · … · 2058 → 0,0 / 0. (Ab 2058
ist der Betrag 0 — die Kohorte läuft aus.)

Signatur:
```
inputs:  arbeitslohn                   money    # Bruttolohn (§ 19), OHNE Versorgungsbezüge §19(2)
         positive_andere_einkuenfte    money    # positive Summe Nicht-nsA-Einkünfte NACH den
                                                 # Ausschlüssen S. 2 (Leibrenten §22 Nr.1a etc.)
         massgebendes_folgejahr        integer  # Kohorte (Jahr nach 64.-Lebensjahr-Vollendung)
output:  altersentlastungsbetrag       money
logik:   bemessung = arbeitslohn + positive_andere_einkuenfte
         (satz, hoechst) = kohorten_tabelle[massgebendes_folgejahr]
         ergebnis = min(satz/100 × bemessung, hoechst)
```

hinweis-Kandidat (Klasse 1, wie § 32b/§ 24a-nsA): `positive_andere_einkuenfte` kommt bereits
**nach den Ausschlüssen S. 2** (Versorgungsbezüge, Leibrenten) herein — die Filterung braucht den
Einkunftsarten-Kontext der § 2-Integration; der auszug nennt die Ausschlüsse, aber die Regel
bekommt den bereits gefilterten Wert. Geltungsbedingung + hinweis pinnen das (Präzedenz § 32b).

Geltungsbedingungen: (1) `alter_64_vor_bezugsjahr_vollendet` (S. 3: Anspruch nur, wenn das 64.
Lebensjahr VOR Beginn des VZ vollendet war); (2) `bemessung_netto_nach_ausschluessen` (S. 2);
(3) `parameter_je_kohorte_nicht_vz` (das neue Muster).

Wächter-Seeds (Rechenweg = Wortlaut-Tabelle):
- Kohorte 2026 (12,8 % / 608), bemessung 3.000 → 0,128 × 3.000 = 384,00 < 608 → **384,00**.
- Kappung: Kohorte 2026, bemessung 10.000 → 1.280 > 608 → **608,00** (Höchstbetrag).
- Alt-Kohorte 2005 (40,0 % / 1.900), bemessung 3.000 → 1.200,00 < 1.900 → **1.200,00**.
- Auslauf-Kohorte 2058 (0,0 % / 0) → **0,00** (unabhängig von der Bemessung).
- Präzisions-Hinweis: `satz/100 × bemessung` in decimal, Cent-Schnitt zuletzt (praezisions_lint).

---

## Nächste Schritte
1. Instructor-Review dieses Pakets (Signaturen, Teilregel-Split § 33b, das Kohorten-Muster § 24a).
2. Nach Freigabe: Signaturen + Tabellen-Parameter + Seeds in `rules.yaml` (§ 24a-Kohorten-Tabelle
   als versionierte Parameter-Struktur — ggf. eigene Konvention in `signatur_konventionen.yaml`
   analog p32a-`tariff_parameters`, Schlüssel Kohorte).
3. Stufe B (Doppelformalisierung + Gates), Freigabe je Lauf. Für § 24a evtl. hinweis-Kanal
   (netto-Bemessung) vorbereitet.
4. Landkarte: § 33b + § 24a von ⬜/🟡 auf ✅; agB 2/4 → 4/4, Tarif-Mechanismen 1/4 → 2/4.
   Danach Paket B (§ 33a + § 10b), Paket C (§ 10 Nr. 5 + Abs. 1a).
