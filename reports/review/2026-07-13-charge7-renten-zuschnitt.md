# Charge-7-Zuschnitt: Renten (§ 22 Nr. 1 Leibrenten, Anlage R)

Zweite neue Einkunftsart. Stufe A, $0, via skip-judge (Judge geparkt bis Modell-Entscheidung).
Freeze vom Instructor (sha 064ba262, sources-check 55/55). § 22 beginnt „Sonstige Einkünfte sind".

## Kern: p22_1_leibrente_besteuerungsanteil

Wortlaut § 22 Nr. 1 S. 3 Buchst. a aa): der steuerpflichtige Anteil der Leibrente ergibt sich aus
dem **Jahr des Rentenbeginns und dem in diesem Jahr maßgebenden Prozentsatz** (Besteuerungsanteil)
aus der nachstehenden Tabelle. Der Prozentsatz ist am Rentenbeginn FIXIERT (Kohorte, lebenslang).

Rechenkern (Erstjahr): `steuerpflichtiger_rentenanteil = (besteuerungsanteil_prozent / 100) ×
jahresrente`. Signatur: `jahresrente money, besteuerungsanteil_prozent decimal ->
steuerpflichtiger_rentenanteil money`. **Andockung/Kohorten-Muster wie § 24a**: der
`besteuerungsanteil_prozent` kommt als Input aus der Kohorten-Tabelle (§ 2-Integration,
params/kohorten/rente_besteuerungsanteil_p22.yaml, Schlüssel = Rentenbeginn-Jahr). Präzision:
Prozent in decimal, Cent-Schnitt zuletzt (praezisions_lint).

Kohorten-Tabelle (§ 22 Nr. 1 S. 3 a aa, Rentenbeginn → Besteuerungsanteil %): 2005 und früher → 50 %,
dann steigend (~2 %/Jahr bis 2020 → 80 %, danach langsamer nach dem Wachstumschancengesetz), bis
100 %. Aus dem Freeze parsen + gegen Anker validieren (wie § 24a; exakte Werte im Stufe-B-params).
→ params/kohorten/ (nicht vz-gebunden, lebenslang fix).

Seeds (Beispiel-Prozentsätze, exakt nach Freeze-Parse): Rentenbeginn 2005 (50 %), Jahresrente
12.000 → 6.000; Rentenbeginn 2020 (~80 %), 12.000 → 9.600; Rentenbeginn 2040 (100 %), 12.000 →
12.000; Jahresrente 0 → 0.

## § 9a Nr. 3 WK-Pauschbetrag 102 € — ANTWORT: § 2-Integration, KEINE eigene Charge-7-Regel

Der WK-Pauschbetrag für sonstige Einkünfte (§ 9a S. 1 Nr. 3, 102 €; im bestehenden § 9a-Freeze
`estg_p9a_2026-07-09.txt`) ist analog zum Arbeitnehmer-Pauschbetrag (§ 9a Nr. 1), der bereits in der
handgeschriebenen § 2-Integration (arbeitnehmerfall) lebt. Der 102-€-Abzug ist ein
Subtraktionsschritt in der Einkünfte-Ermittlung (Renten-Einkünfte = steuerpflichtiger_rentenanteil −
102), NICHT eine eigenständige Rechtsfolge-Regel. **Entscheidung: als § 2-Integrations-Konstante
führen (wie AN-PB), nicht als separate Pipeline-Regel** — Uniformität mit dem AN-PB-Muster, kein
Mikro-Zuschnitt. (Falls du eine eigene Mini-Regel bevorzugst — p9a_3_wk_pauschbetrag_renten, 102
Festbetrag — ist das ein Einzeiler; aber die §-2-Konstante ist konsistenter.)

## Scope-Grenzen (dokumentiert)
- **€-Rentenfreibetrag-Fixierung** (Folgejahre): der steuerfreie €-Betrag wird am Ende des zweiten
  Rentenjahres FIXIERT und gilt lebenslang unverändert; die Folgejahre rechnen Rente − fixierter
  €-Freibetrag, NICHT erneut den %-Satz. Das ist mehrjähriger State (§ 2-Integration), nicht
  Erstjahr-Rechnung → Scope-Grenze/Backlog. Die Erstjahr-Regel liefert den Besteuerungsanteil, aus
  dem die Integration den €-Freibetrag ableitet.
- **Öffnungsklausel** (§ 22 Nr. 1 S. 3 a bb): Teil-Ertragsanteilsbesteuerung bei hohen Beiträgen über
  dem Höchstbeitrag — Sonderfall, eigener Zuschnitt.
- **Ertragsanteils-Renten** (§ 22 Nr. 1 S. 3 a bb, private Renten/Zeitrenten): eigene Ertragsanteil-
  Tabelle nach Alter bei Rentenbeginn — separater Zuschnitt (andere Tabelle).
- **§ 22 Nr. 1a (Unterhalt/Realsplitting-Empfänger), Nr. 2 (§ 23), Nr. 3, Nr. 5 (Altersvorsorge-
  verträge)** = eigene Regelungsbereiche, nicht Leibrenten-Kern.

## Nächste Schritte
1. Instructor-Review (Signatur, Kohorten-params-Ansatz, WK-PB-Entscheidung, €-Freibetrag-Scope).
2. Nach Freigabe: params/kohorten/rente_besteuerungsanteil_p22.yaml (aus Freeze geparst + validiert),
   Signatur + Seeds in rules.yaml. Stufe B skip-judge (/endpoints-Log entfällt — Judge geparkt bis
   Modell-Entscheidung).
3. Landkarte: Renten § 22 → andere Einkunftsarten 2/4.
