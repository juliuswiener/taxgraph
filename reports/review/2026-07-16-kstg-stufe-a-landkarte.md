# KStG Stufe-A-Landkarte (Paket 5, Nenner B: Kapitalgesellschaften)

Autor: taxgraph-instructor, 2026-07-16.
Freigabe: Julius direkt im Chat 2026-07-16 ("mach auch kstg") — hebt die
KStG-Sperre aus dem Paket-4-Abschluss auf.

## Gültigkeitslage (Gültigkeits-Check-Direktive)

- Alle 9 Freezes: GII-Stand 2026-07-16 (`sources/gesetze-im-internet/kstg_p*_2026-07-16.*`).
- § 34 Abs. 1: konsolidierte Fassung gilt "erstmals für den Veranlagungszeitraum 2025".
  ABER: alle §-34-Sonderregeln zu den Kern-§§ (§ 8 Abs. 1, § 8b, § 9, § 10, § 24) stammen
  aus Änderungen 2019/2021 und griffen lange vor VZ 2024 → **Kern einfassig VZ 2024–2026**,
  kein VZ-Split nötig (Kontrast zu § 9 Nr. 1 GewStG).
- § 23 Abs. 1 trägt die StInvSofortPG-Staffel im Wortlaut: "Veranlagungszeiträume bis 2027
  15 Prozent", dann 14/13/12/11/10 % ab VZ 2028–2032. Für den Scope VZ 2024–2026 ist der
  Satz **konstant 15 %**; die Staffel ist der Beleg, dass der Freeze aktuell ist. Regel
  kodiert 15 % + Bounds-Assertion VZ ∈ [2024, 2026] (int-VZ-Muster) — KEINE Staffel-Logik.
- JStG 2024 / WtChancenG berühren die Kern-§§ nicht (JStG-2024-Änderungsfreezes im Repo
  betreffen EStG/GewStG; § 34 nennt nichts Jüngeres für den Kern).

## Kern-Kette Nenner B

```
§ 1 Abs. 1 Nr. 1  Geltungsbedingung: rechtsform = Kapitalgesellschaft,
                  Geschäftsleitung/Sitz Inland (Katalog-Bedingung, keine Rechenregel)
§ 8 Abs. 1        Einkommen nach EStG-Vorschriften + KStG-Spezifika
                  → SLOT-Architektur wie § 7 GewStG: nimmt EStG-Gewinn (W2/§ 15-Pfad)
§ 8 Abs. 3 S. 2   + verdeckte Gewinnausschüttungen (mindern das Einkommen nicht)
      S. 3        − verdeckte Einlagen (erhöhen das Einkommen nicht)
                  → beide als INPUT-Beträge (Sachverhalt), keine Bewertungslogik
§ 8b Abs. 1+4     Dividenden außer Ansatz, AUSSER Streubesitz < 10 % (steuerpflichtig)
§ 8b Abs. 2+3     Veräußerungsgewinne außer Ansatz; 5 % pauschal nichtabziehbar
§ 8b Abs. 5       5 % der Bezüge Abs. 1 pauschal nichtabziehbar
                  → Netto-Effekt "95 % frei" NUR via Abs.-Kombination, nie als Konstante
§ 9 Abs. 1 Nr. 2  Spenden abziehbar bis max(20 % des Einkommens; 4 ‰ Umsätze+Löhne)
§ 10              Nichtabziehbare (Personensteuern Nr. 2, Geldstrafen Nr. 3) → Add-back-Input
§ 10d EStG        Verlustabzug via § 8 Abs. 1: 1-Mio-Sockel + 70 % (!!)
                  → p10d_2-Muster (bestand = Input). ACHTUNG: 70 % wie EStG,
                  NICHT 60 % wie § 10a GewStG — Golden muss die 60/70-Trennung
                  spiegelbildlich zum GewSt-Korpus verankern.
§ 23 Abs. 1 Nr. 1 KSt = 15 % des zvE (VZ 2024–2026)
SolZ              5,5 % auf KSt (solzg_1995_p3/p4 gefreezt)
GewSt             KapGes gewerbesteuerpflichtig kraft Rechtsform; § 7 GewStG dockt am
                  KSt-Einkommen an (§ 7 S. 4: "im Übrigen ist § 8b KStG anzuwenden")
                  → Nenner B gesamt = KSt + SolZ + GewSt
§ 24              NEGATIV-Beleg: Freibetrag 5.000 € gilt NICHT für Kapitalgesellschaften
                  (S. 2 Nr. 1) → im Nenner B KEIN Freibetrag; Freeze existiert, damit der
                  Ausschluss ankerbar ist
```

Nicht Kern (dokumentierte Nachtrag-Kandidaten): § 8a/§ 4h Zinsschranke, § 8c/§ 8d
Verlustuntergang, § 1a Optionsmodell, §§ 27 ff. Einlagekonto, Organschaft §§ 14 ff.

## Chargen-Schnitt (Vorschlag, Caps nach GewSt-Kalibrierung)

| Charge | Inhalt | Quellen | Cap |
|---|---|---|---|
| K1 | § 8 Abs. 1 Einkommens-Slot + Abs. 3 vGA/Einlagen (Inputs) | kstg_p8 | $0,15 |
| K2 | § 8b Abs. 1–5 Beteiligungserträge inkl. Streubesitz-Bedingung | kstg_p8b | $0,23 |
| K3 | § 9 Abs. 1 Nr. 2 Spenden-Höchstbetrag (20 %/4 ‰) | kstg_p9 | $0,15 |
| K4 | § 10 Add-back (Nr. 2 Personensteuern, Nr. 3 Geldstrafen) | kstg_p10 | $0,07 |
| K5 | § 10d-Verlust KSt-Variante (Sockel + 70 %, bestand=Input) | kstg_p8 + estg_p10d | $0,15 |
| K6 | § 23 Satz + Nenner-B-Endverdrahtung (KSt+SolZ+GewSt) + Goldens | kstg_p23 (+Runner) | $0,15 |

Summe geschätzt ~$0,90, **Cap-Vorschlag $1,00 gesamt**.

## Judge-Artefakt-Erwartungen (Vorab-Deklaration nach GewSt-Muster)

- K2/K3: Stufe-A-Zuschnitt lässt Rest-Absätze weg (§ 8b Abs. 6–12, § 9 Nr. 1/3) →
  erwartbar faithful=False mit leerer abweichungen-Liste (Zuschnitts-Artefakt).
- K1: Slot-Regel liefert BETRAG, Endkette zieht zusammen → Scope-Boundary-Artefakt möglich.
- Verfahren unverändert: Judge-Flag = STOPP + Meldung, Instructor-Adjudikation VORAB,
  dev committet erst nach Ruling.

## Geltungsbedingungen / Katalog

- `rechtsform = kapitalgesellschaft` als neue Geltungsbedingung (§ 1 Abs. 1 Nr. 1-Anker);
  Abgrenzung zum PersGes-Kern (Charge 24, § 15/§ 15a) muss explizit sein — keine Regel
  darf für beide Rechtsformen gleichzeitig feuern.
- VZ-Bounds 2024–2026 mit Assertion (int-VZ-Brücke, M5-Muster).
- § 8b-Streubesitz (< 10 %) ist BEDINGUNG, nicht Betrag — als Geltungsbedingung mit
  deckt_ab-Anker materialisieren (Lehre: versprochene-bedingung-materialisieren).

## Stufe-B-Vergabe

Bau-Order geht an den ersten frei werdenden dev (dev-1: DBA-Kataloge; dev-2:
Gate-Paket). Goldens: Nenner-B-Kette in Cent, Hand-Kette + dev-Triangulation wie GewSt.
