# Multi-VZ-Programm (VZ 2024–2026) — M2-Scoping: Rechtsänderungs-Diff (Instructor, 2026-07-15)

Julius-Wort 2026-07-15: "ab steuererklärung 2024". Ziel: System rechnet VZ 2024, 2025, 2026.

## Befund vorab: das Fundament EXISTIERT schon

`params/2024|2025|2026/` liegen mit je VZ-Datei (Tarif § 32a für ALLE drei Jahre,
Entfernungspauschale, AN-Pauschbetrag, Arbeitszimmer, Sonderausgaben-PB) — das MVP war
multi-VZ-gebaut. Die MVP-Regeln nehmen Sätze als params-Inputs (Beispiel
Entfernungspauschale: VZ-2024/25-Staffel 0,30/0,38 vs. VZ-2026 einheitlich 0,38 ab km 1
nach StÄndG 2025 ist BEREITS korrekt parametrisiert). **Die Lücke sind die ~60 Regeln
der Chargen 4–25**, die 2026-Werte im auszug/Code binden.

## Rechtsänderungs-Diff 2024→2026 (recherchiert + quellen-verifiziert, Stand heute)

### A. Nur params-Jahreswerte (kein Regel-Umbau)
| Wert | 2024 | 2025 | 2026 | Gesetz |
|---|---|---|---|---|
| Grundfreibetrag | 11.784 | 12.096 | 12.348 | G. z. Freistellung Existenzminimum 2024 + SteFeG (BGBl 2024 I Nr. 449) |
| Tarif-Eckwerte | eigene | +2,6 % verschoben | +2 % | SteFeG (params/­<vz>/p32a ✓ liegen) |
| Soli-Freigrenze | 18.130/36.260 | 19.950/39.900 | 20.350/40.700 | SteFeG |
| Kinderfreibetrag (je Elternteil) | 3.306 | 3.336 | 3.414 | SteFeG |
| Kindergeld | 250 | 255 | 259 | SteFeG |
| Unterhaltshöchstbetrag § 33a (=GFB-gekoppelt) | 11.784 | 12.096 | 12.348 | — |
| Kohorten (§ 24a/§ 22/Kinderzulage/BLP) | — | — | — | params/kohorten ✓ tragen schon |

### B. Struktur-Änderungen (eigene Regel-Version oder neue Regel)
1. **Entfernungspauschale (§ 9 Abs 1 Nr 4):** VZ ≤ 2025 Staffel 0,30/0,38 ab km 21;
   VZ 2026 einheitlich 0,38 ab km 1 (StÄndG 2025, verkündet 23.12.2025). ✅ BEREITS
   parametrisiert (MVP). Mobilitätsprämie entfristet; Prämien-Mechanik 14 % unverändert,
   aber Bemessung folgt neuer Pauschale → p101-Geltungsbedingungen gegen VZ prüfen.
2. **Degressive AfA § 7 Abs 2 — FEHLENDE REGEL (auch für unser Ziel-VZ!):**
   (a) WachstumschancenG-Fenster: Anschaffung 1.4.2024–31.12.2024, max. 2× linear/20 %*
   (*Fassung prüfen: teils 2,5×/25 % — BGBl-Abgleich nötig); (b) Investitionsbooster
   (InvestitionssofortprogrammG, verkündet 18.7.2025): Anschaffung 1.7.2025–31.12.2027,
   max. 3× linear/30 %. → NEUE Regel mit Anschaffungsfenster-Kohorten-params.
3. **E-Fahrzeug-AfA 75 % im Anschaffungsjahr** (Investitionsbooster, betriebliche
   E-Fahrzeuge) → neue kleine Regel (Anschaffung ab 1.7.2025).
4. **Verbindlichkeiten-Abzinsung § 6 Nr 3 a. F.:** für WJ-Ende ≤ 31.12.2022 galt 5,5 % —
   VOR unserem Fenster (ab VZ 2024 irrelevant), nur Doku. ✓ erledigt.
5. **DBA CH:** VZ ≤ 2025 = Konsolidierung 2010; VZ 2026 = Protokoll 2023. ✅ Overlay-
   Struktur trägt exakt das — Katalog braucht VZ-Spalte.
6. **Sonder-AfA § 7g Abs 5:** 20 % (Anschaffung < 2024) / 40 % (ab 2024) — als Kohorte
   geplant (C27) ✓ deckt Multi-VZ mit.
7. **§ 10d-Mindestbesteuerung:** 70 % gilt 2024–2027 einheitlich ✓ keine Änderung im Fenster.
8. **Prüfliste offen (M3):** Versorgungsfreibetrag-Kohorten (laufen ✓), Vorsorge-
   Höchstrechengrößen (§ 10: Beitragsbemessungsgrenzen 2024/25/26!), Sachbezugswerte,
   Verpflegungspauschalen (unverändert 14/28), Behinderten-Pauschbeträge (unverändert),
   Sparer-PB (unverändert seit 2023), GWG 800 € (unverändert).

### C. ELSTER
Datenarten sind VZ-spezifisch (E10-2024/-2025/-2026, E77-…): Feldmapping je VZ prüfen
(Kz-Drift). checkESt offline validiert VZ ≤ 2025 → Multi-VZ macht den amtlichen
Offline-Vollbeweis für 2024/2025 MÖGLICH, sobald Hersteller-ID da.

## Programm-Zuschnitt (Vorschlag)
- **M1** (dev, läuft): Bestands-Befund VZ-Dimension.
- **M3**: params-Nachzug 2024/2025 für alle Chargen-4-25-Regeln mit Jahreswerten
  (Inventar aus M1×M2; GETTSIM-Import-Skript params/import_gettsim.py als Zubringer prüfen).
- **M4**: Struktur-Regeln (degressive AfA a+b, E-Kfz-75 %) als Charge 28 — Freezes:
  geltende Fassung § 7 liegt (Abs-2-Fenster prüfen); WachstumschancenG-Altfassung
  besorge ich falls nötig (BGBl).
- **M5**: VZ-Dispatch-Konvention (params/<vz>-Ladung in Integration; Regel-Versionen via
  gueltig_ab/bis NUR wo B-Punkte es verlangen) + E2E-Goldens je VZ (Agent-Fanout:
  ein Golden-Satz pro VZ).
Budget: M3/M5 überwiegend $0; M4 ~$0,2–0,3.
