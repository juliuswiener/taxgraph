# GewSt-Ketten — Hand-Golden für G1–G5 (taxgraph-dev-2, 2026-07-16)

Instructor-Auftrag (Paket 4): 6–8 GewSt-Hand-Ketten NUR aus Freeze-Wortlaut, EZ 2024/2025/2026
wo relevant, Triangulations-Muster wie M5-VZ-Goldens. **Unabhängig** von dev-1 (baut G1/G2 aus
denselben Freezes) — Abweichung später = Eskalation an Instructor, nicht an dev-1. Read-only +
Handrechnung, $0, LLM-frei. Freeze-Basis **b84161d**. Rechen-Skript `scratchpad/gewst_ketten.py`
(Decimal, exakt). Jede Zahl mit Zitatstelle (Freeze-Datei:Zeile).

## Rechtskette + Reihenfolge (Wortlaut-belegt)
1. **§6** — „Besteuerungsgrundlage für die Gewerbesteuer ist der Gewerbeertrag." (`gewstg_p6:10`)
2. **§7 S.1** — Gewerbeertrag = Gewinn aus Gewerbebetrieb (EStG/KStG) „vermehrt und vermindert um
   die in den §§ 8 und 9 bezeichneten Beträge." (`gewstg_p7:10`)
3. **§8 Nr.1** — „**Ein Viertel der Summe aus** a) Entgelten für Schulden … d) **einem Fünftel** der
   Miet-/Pachtzinsen … beweglicher WG … e) **der Hälfte** der Miet-/Pachtzinsen … unbeweglicher WG …
   f) **einem Viertel** der Aufwendungen für … Rechte … **soweit die Summe den Betrag von 200 000
   Euro übersteigt**" (`gewstg_p8:10`). ⇒ Hinzu = 0,25 × max(0, Summe(a…f) − 200.000); Gewichte
   a/b/c = 100 %, d = ⅕, e = ½, f = ¼. **Reihenfolge:** erst gewichten & summieren, dann 200k-Schwelle,
   dann ¼ des übersteigenden Betrags.
4. **§9 Nr.1** — VZ-Split (s. G2): EZ 2024 Alt = 1,2 % Einheitswert; EZ 2025+ Neu = Grundsteuer-BA.
5. **§10a S.1/2** — „bis zu einem Betrag in Höhe von **1 Million Euro** … Der 1 Million Euro
   übersteigende maßgebende Gewerbeertrag ist **bis zu 60 Prozent** … zu kürzen." (`gewstg_p10a:10`).
6. **§11 Abs.1 S.3 / Abs.2** — „auf **volle 100 Euro nach unten abzurunden** und 1. bei natürlichen
   Personen sowie bei Personengesellschaften um einen **Freibetrag … 24 500 Euro** … zu kürzen";
   „Steuermesszahl … beträgt **3,5 Prozent**." (`gewstg_p11:10`). **Reihenfolge:** abrunden → Freibetrag
   → 3,5 %.
7. **§35 EStG (Bestand p35_1)** — Ermäßigung = min der drei Deckel: (1) **4,0× Messbetrag**,
   (2) tatsächlich zu zahlende GewSt (= Messbetrag × Hebesatz), (3) Ermäßigungshöchstbetrag
   (anteilige tarifliche ESt auf gewerbliche Einkünfte). Deckel 3 = EStG-/ESt-Kontext (Cross-Ref
   p35_1), außerhalb reiner GewSt-Kette.

*Modellierungs-Hinweis:* alle Ketten mit Gewerbeerträgen als Vielfache von 100 gewählt → die
§10a-vs-§11-Abrundungs-Reihenfolge ist konstruktiv irrelevant (kein Streitpunkt in diesen Goldens).

---

## G1 — BASIS-KETTE (Einzelunternehmer, EZ 2026)
| Schritt | Rechnung | Wert |
|---|---|---|
| Gewinn Gewerbebetrieb | (frei gewählt) | 500.000,00 |
| §8 Summe(a…f) | a 300.000 + d 200.000/5=40.000 + e 600.000/2=300.000 + f 80.000/4=20.000 | 660.000,00 |
| §8 Nr.1 Hinzurechnung | 0,25 × (660.000 − 200.000) | **115.000,00** |
| §9 Nr.1 (EZ2026=Neu) | Grundsteuer-BA (frei) | −12.000,00 |
| §7 Gewerbeertrag | 500.000 + 115.000 − 12.000 | **603.000,00** |
| §11 abgerundet − FB | 603.000 (÷100 glatt) − 24.500 | 578.500,00 |
| §11 Steuermessbetrag | 578.500 × 3,5 % | **20.247,50** |
| §35 Hebesatz 380 % | 4×MB = 80.990,00 · GewSt = 76.940,50 → **Deckel 2 bindet** | 76.940,50 |
| §35 Hebesatz 400 % | 4×MB = GewSt = 80.990,00 (Grenzfall, volle Neutralisierung) | 80.990,00 |
| §35 Hebesatz 450 % | 4×MB = 80.990,00 < GewSt 91.113,75 → **Deckel 1 bindet** | 80.990,00 |

Deckel 3 (Ermäßigungshöchstbetrag): bei Einzelunternehmer mit nur gewerblichen Einkünften ist der
Anteil = 1; sofern die geballte tarifliche ESt ≥ Deckel 1/2, **bindet Deckel 3 nicht** (Annahme
Hochverdiener). Andernfalls kappt er zusätzlich — ESt-Kontext, p35_1.

## G2 — §9-VZ-SPLIT-PAAR (identischer Fall EZ 2024 vs EZ 2025) → Negativtest
Gemeinsam: Gewinn 300.000; §8 Summe 150.000 (a) + 150.000 (e 300.000/2) = 300.000 → Hinzu
0,25 × (300.000 − 200.000) = **25.000,00**.

| | §9 Nr.1 Fassung (Zitat) | Kürzung | §7 Gewerbeertrag | Messbetrag |
|---|---|---|---|---|
| **EZ 2024** | „1,2 Prozent des **Einheitswerts** … Grundbesitzes" (`gewstg_p9_altfassung_ez2024:12`); EW 400.000 × 1,2 % | 4.800,00 | 320.200,00 | **10.349,50** |
| **EZ 2025** | „die im Erhebungszeitraum als **Betriebsausgabe erfasste Grundsteuer**" (`gewstg_p9:10`, JStG 2024 Art.9 Nr.3 `jstg2024_gewstg_aenderungen:15`); Grundsteuer-BA 9.000 | 9.000,00 | 316.000,00 | **10.202,50** |
| **Δ** | Fassungswechsel | +4.200,00 | −4.200,00 | **−147,00** |

**Anwendungs-Anker §36 Abs.4b S.1:** „§ 9 Nummer 1 Satz 1 in der Fassung des Artikels 9 des Gesetzes
vom 2. Dezember 2024 … ist **erstmals für den Erhebungszeitraum 2025** anzuwenden." (`gewstg_p36:10`).
⇒ EZ 2024 zwingend Altfassung, EZ 2025+ Neufassung. Das Δ Messbetrag −147,00 (bzw. GewSt −147 ×
Hebesatz) ist der **Negativtest**: naiver 1:1-Klon EZ2024→EZ2025 ohne Fassungswechsel wäre still
falsch. Betragsrichtung fall-abhängig (hier EW-Kürzung > Grundsteuer-BA; bei anderen Betrieben umgekehrt).

## G3 — §10a GEWERBEVERLUST (Sockel 1 Mio + 60 %; **kein** 2-Mio-Splitting, **kein** 70 %)
Kapazität = 1.000.000 + 0,60 × max(0, maßgeb. Gewerbeertrag − 1.000.000); Verlustabzug =
min(Fehlbetrag-Bestand, Kapazität).

| Fall | maßgeb. GewErtrag | Fehlbetrag-Bestand | Kapazität | Verlustabzug | verbleib. GewErtrag | Rest-Fehlbetrag |
|---|---|---|---|---|---|---|
| (a) unter Sockel | 900.000 | 700.000 | 1.000.000 | 700.000 | 200.000 | 0 |
| (b) >1 Mio, Deckel **bindet nicht** | 5.000.000 | 3.000.000 | 3.400.000 | 3.000.000 | 2.000.000 | 0 |
| (c) >1 Mio, Deckel **BINDET** | 5.000.000 | 5.000.000 | 3.400.000 | **3.400.000** | 1.600.000 | 1.600.000 |

Fall (c) = Mindestbesteuerung: trotz 5 Mio Verlustbestand bleiben 1,6 Mio Gewerbeertrag steuerpflichtig,
Rest-Fehlbetrag 1,6 Mio ist vortragsfähig (§10a S.6/7). Fußnote-Freeze bestätigt: **60 %** in §10a,
das WtChancenG-**70 %** gilt **nur § 10d EStG** (`gewstg_p10a` Kopf/Meta) — Kontrast zur ESt-
Mindestbesteuerung p10d_2.

## G4 — KANTEN
- **(a) §8-Summe < 200k → Hinzurechnung 0:** Summe 140.000 (a 120.000 + f 80.000/4=20.000) < 200.000
  → 0,25 × max(0, 140.000 − 200.000) = **0,00**. Freibetrag wirkt als **Schwelle nach oben**, floored 0.
- **(b) Gewerbeertrag < 24.500 → Messbetrag 0:** GewErtrag 20.000 → abger. 20.000 − 24.500, Kürzung
  „höchstens jedoch in Höhe des abgerundeten Gewerbeertrags" (`gewstg_p11:10`) → Bemessung 0 →
  **Messbetrag 0,00**.
- **(c) „negative Summe §8" / Nicht-Spiegelbildlichkeit:** die §8-Nr.1-Komponenten a–f sind
  konstruktiv **≥ 0** (Add-backs von Entgelten/Mieten/Lizenzen) → eine echte negative Summe ist aus
  dem Wortlaut nicht erreichbar. Der 200k-Freibetrag ist **nicht spiegelbildlich**: „soweit die Summe
  den Betrag von 200 000 Euro übersteigt" (`gewstg_p8:10`) erzeugt nie eine negative Hinzurechnung
  (kein Abzug bei Summe < 200k). **Wortlaut eindeutig** → kein echter Auslegungsspielraum; als
  bestätigte Nicht-Spiegelbildlichkeit dokumentiert (nicht als offene Frage).

---

## VZ-/EZ-Drift-Befund (Kernresultat für die Golden-Wartung)
- **EINZIGE gebundene Drift EZ 2024↔2025: § 9 Nr. 1 Satz 1** (Einheitswert 1,2 % → Grundsteuer-BA),
  Anker §36 Abs.4b (`gewstg_p36:10`). EZ 2026 = EZ 2025 (Neufassung).
- **§8-Kern (¼ / 200k), §11 (3,5 % / 24.500), §10a (1 Mio / 60 %) = EZ-stabil** über 2024/25/26.
  §35 = **4,0×** stabil (p35_1). Sauberes Muster wie EStG-VZ-Goldens: nur eine benannte Norm driftet.
- **Micro-Drift (nicht in Basis-Ketten):** § 8 Nr.1 Buchst. d Satz 2 (E-Fahrzeug-Hälfte-Regel):
  bei Verträgen vor 1.1.2025 Reichweite 60 km statt 80 km (`gewstg_p36:10`, §36 Abs.4 S.2). Nur bei
  E-Fahrzeug-Leasing relevant; für eigene Golden-Fälle bewusst ausgelassen — als Kandidat für einen
  Spezial-Negativtest vermerkt.

## Übergabe
- Diese Ketten = **Golden-Erwartungen G1–G5**. Für dev-1s parallelen Bau als unabhängige Zweitrechnung.
  **Bei Abweichung dev-1↔dev-2 → Eskalation an Instructor** (Auftragslage), nicht direkte Korrektur.
- Alle Zwischenwerte reproduzierbar (`scratchpad/gewst_ketten.py`). Zitatstellen durchgängig
  Freeze-Datei:Zeile (b84161d).
- Offen für Bau: §35-Deckel-3 (Ermäßigungshöchstbetrag) braucht ESt-Kontext (zvE, Anteil gewerbl.
  Einkünfte) — Cross-Ref auf p35_1/EStG-Tarif, nicht in reiner GewSt-Kette abbildbar.
