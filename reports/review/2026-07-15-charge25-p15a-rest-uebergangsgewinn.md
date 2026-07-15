# Charge 25 — §15a-Rest (erweiterte Außenhaftung) + B8 Übergangsgewinn (Zuschnitt, Stufe A, 2026-07-15)

**LETZTE Charge des Großkomplex-Programms.** W3 P4 (§15a-Reste) + W2 B8 (Übergangsgewinn). Quellen:
`estg_p15a_2026-07-14` (§ 15a Abs. 1 S. 2/3), `estr_r4_6_2025` (R 4.6, **verwaltung/Mirror**). **2 Regeln.**
Kein Stufe-B ohne Cap-Wort. Alle Zitatanker VOLL-Länge via `_normalize` verifiziert; R-4.6-Anker
**hyphen-frei** gewählt (Mirror-Silbentrennung, Meta-Warnung).

## Sondersatz-Sweep (verbatim Freeze-Grep)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | § 15a Abs. 1 S. 2 | **„bis zur Höhe des Betrags, um den die im Handelsregister eingetragene Einlage … seine geleistete Einlage übersteigt"** | Die ausgleichsfähige Grenze aus P2 wird um den **Haftungsbetrag** (eingetragene − geleistete Einlage) ERWEITERT → `+haftungsbetrag` in der min-Grenze. |
| S2 | § 15a Abs. 1 S. 2 | **„soweit durch den Verlust ein negatives Kapitalkonto entsteht oder sich erhöht"** | Der erweiterte Betrag greift GERADE für den Teil, der negatives KK erzeugt (sonst deckt schon P2). |
| S3 | § 15a Abs. 1 S. 3 | „nur anzuwenden, wenn … im Handelsregister eingetragen … Haftung nachgewiesen … nicht durch Vertrag ausgeschlossen" | Drei Nachweis-Voraussetzungen → bool-Bedingungen (Nachtrag). |
| S4 | R 4.6 S. 2 | **„gleichmäßig entweder auf das Jahr des Übergangs und das folgende Jahr oder … die beiden folgenden Jahre"** | Übergangsgewinn-Verteilung auf **1, 2 oder 3 Jahre** (Antrag) → `uebergangsgewinn / verteilungsjahre`. |
| S5 | R 4.6 S. 3 | „Wird der Betrieb vor Ablauf des Verteilungszeitraums veräußert … erhöhen die noch nicht berücksichtigten Beträge den laufenden Gewinn des letzten Wirtschaftsjahres" | Veräußerungs-Klausel: Rest-Sofortversteuerung → Bedingung/Nachtrag. |

## Regel 1 — § 15a Abs. 1 S. 2: erweiterte Außenhaftung (`p15a_1_erweiterte_aussenhaftung`)

**Wortlaut (Zitatanker `bis zur Höhe des Betrags, um den die im Handelsregister eingetragene Einlage
des Kommanditisten seine geleistete Einlage übersteigt`, 131 Zeichen voll-verifiziert):** Haftet der
Kommanditist nach § 171 Abs. 1 HGB, sind Verluste ABWEICHEND von S. 1 auch bis zur Höhe des
Haftungsbetrags (eingetragene − geleistete Einlage) ausgleichsfähig, selbst wenn dadurch negatives
Kapitalkonto entsteht/sich erhöht.

- **Erweiterungs-Regel zu P2 (C24):** die ausgleichsfähige Grenze aus `p15a_1_ausgleichsfaehiger_verlust`
  (max(kapitalkonto, 0)) wird um den Haftungsbetrag erhöht.
- **Signatur** `ErweiterteAussenhaftung`: `verlustanteil: money`, `kapitalkonto: money` (signiert),
  `eingetragene_einlage: money`, `geleistete_einlage: money` → `ausgleichsfaehiger_verlust_erweitert: money`.
- **Rechenkern:** `haftungsbetrag = max(0; eingetragene_einlage − geleistete_einlage)`; `ausgleichsfaehiger_
  verlust_erweitert = min(verlustanteil; max(kapitalkonto; 0) + haftungsbetrag)`. Der Haftungsbetrag ist
  ≥ 0 (übersteigt-Konstruktion; geleistete ≥ eingetragene → 0, keine Erweiterung = P2).
- **⚠ Klasse-2/Boundary:** wie P2 doppeltes Floor (`max(…;0)` für KK UND für Haftungsbetrag), Cent-genau.
- **Geltungsbedingungen:** `haftung_171_hgb_abs1` (deckt_ab „auf Grund des § 171 Absatz 1 des
  Handelsgesetzbuchs" — Außenhaftung besteht), `einlage_differenz_haftungsbetrag` (Haftungsbetrag =
  eingetragene − geleistete Einlage, ≥ 0), `nachweis_voraussetzungen_abs1_s3` (deckt_ab „Satz 2 ist nur
  anzuwenden, wenn" — eingetragen + Haftung nachgewiesen + nicht vertraglich ausgeschlossen = S. 3, hier
  als Anwendbarkeits-Bedingung, Feinprüfung = Nachtrag), `erweitert_p2_ausgleichsfaehiger_verlust`
  (erweitert die P2-Grundregel; ohne erweiterte Haftung identisch zu P2).
- **Seeds (Wächter):** (Verlust 15000, KK 10000, eingetr 8000, geleist 5000) → Haftung 3000, Grenze
  13000 → **13000** · (15000, 10000, 5000, 5000) → Haftung 0 → **10000 (keine Erweiterung = P2)** ·
  **(5000, KK 0, eingetr 8000, geleist 5000) → Haftung 3000, Grenze 3000 → 3000 (WÄCHTER KK=0, nur Haftung)** ·
  **(15000, KK −3000, 8000, 5000) → max(−3000;0)=0 + 3000 → min(15000;3000)=3000 (WÄCHTER neg. KK)** ·
  (2000, 0, 8000, 5000) → min(2000; 3000) = 2000 (Verlust < Haftungsgrenze).

## Regel 2 — R 4.6 S. 2: Übergangsgewinn-Verteilung (`estr_4_6_uebergangsgewinn_verteilung`)

**Wortlaut (Zitatanker `auf das Jahr des Übergangs und das folgende Jahr oder auf das Jahr des Übergangs
und die beiden folgenden Jahre`, 111 Zeichen voll-verifiziert, hyphen-frei):** Beim Wechsel zum
Betriebsvermögensvergleich kann der Übergangsgewinn (Saldo aus Zu-/Abrechnungen) auf **Antrag** zur
Vermeidung von Härten gleichmäßig auf **1, 2 oder 3 Jahre** verteilt werden.

- **⚠ Quellen-Typ verwaltung (Billigkeit):** R 4.6 ist eine **Verwaltungs-Billigkeitsregelung** (EStR,
  „zur Vermeidung von Härten auf Antrag"), kein Gesetzeswortlaut. Der Übergang selbst (§ 4 Abs. 1/Abs. 3
  Wechsel) ist Gesetz; die 1-3-Jahres-Verteilung ist Verwaltung. Im auszug als `typ: verwaltung` etikettiert.
- **Signatur** `UebergangsgewinnVerteilung`: `uebergangsgewinn: money` (Saldo aus Zu-/Abrechnungen, kann
  negativ = Übergangsverlust sein), `verteilungsjahre: int` (1, 2 oder 3) → `jahresbetrag: money`.
- **Rechenkern:** `jahresbetrag = uebergangsgewinn / verteilungsjahre` (gleichmäßig). **Klasse-5-Vermeidung:**
  money / int, ein Cent-Schnitt (Catala rationale decimals).
- **Geltungsbedingungen:** `verteilung_auf_antrag_1_bis_3_jahre` (deckt_ab Verteilungs-Zitatanker; nur
  auf Antrag, Wahlrecht 1/2/3 Jahre), `gleichmaessige_verteilung` (deckt_ab „zur Vermeidung von Härten
  auf Antrag des Stpfl." — gleichmäßig = /verteilungsjahre), `uebergangsgewinn_saldo_input` (der
  Übergangsgewinn = Saldo aus Zu-/Abrechnungen kommt als Input, die Zu-/Abrechnungs-Ermittlung = eigener
  Komplex), `veraeusserung_rest_letzter_gewinn` (deckt_ab „erhöhen die noch nicht berücksichtigten
  Beträge den laufenden Gewinn des letzten Wirtschaftsjahres" — Veräußerung vor Verteilungsende → Rest-
  Sofortversteuerung = benannter Nachtrag, hier nur die Jahres-Ratenermittlung).
- **Seeds (Boundary):** (Übergangsgewinn 3000, Jahre 3) → 1000,00 · (3000, 2) → 1500,00 ·
  **(3000, 1) → 3000,00 (GRENZFALL: keine Verteilung = voll im Übergangsjahr)** · (9000, 3) → 3000,00 ·
  (−1800, 3) → −600,00 (ÜbergangsVERLUST, gleichmäßig verteilt).

## Benannte Nachträge Charge 25

- **§ 15a Abs. 1 S. 3** Feinprüfung der drei Nachweis-Voraussetzungen (Registereintrag, Haftungsnachweis,
  kein Vertragsausschluss) = eigene Prüf-Bedingungen.
- **§ 15a Abs. 1a** nachträgliche Einlagen; **Abs. 3** Einlageminderung/Haftungsminderung (Gewinn-
  hinzurechnung, 10-Jahres-Fenster); **Abs. 5** andere Unternehmer (stille Ges., GbR, ausl. PersGes,
  Mitreeder) = Bedingungen/Nachträge.
- **R 4.6 S. 3** Veräußerungs-Klausel (Rest → laufender Gewinn letztes WJ) = Bedingung/Nachtrag (oben).
- **R 4.6 S. 1/4** Gewinnberichtigungs-Pflicht + Eröffnungsbilanz-Ansatz; **Anlage zu R 4.6** Zu-/
  Abrechnungs-Tabelle (die Ermittlung des Übergangsgewinn-Saldos) = eigener Komplex.
- Passiver RAP (C23-Nachtrag) bleibt offen.

## Offene Punkte für deine Review

1. **R1 erweiterte Haftung** als eigene Erweiterungs-Regel (min mit `+haftungsbetrag`) vs. Parameter an
   P2 — Empfehlung: eigene Regel (P2 bleibt der Grundfall unberührt; R1 = Andockung mit vier Inputs).
   `haftungsbetrag` in-Regel aus eingetragene/geleistete berechnen (max(0, Differenz)) vs. als ein Input?
   Empfehlung: in-Regel (wortlautnah „um den … übersteigt", zwei Einlage-Inputs).
2. **R2 verwaltung-Quelle** (R 4.6 Billigkeit, kein Gesetz) — als `typ: verwaltung`-Regel ok, oder B8
   lieber komplett als Bedingungs-Paket/Nachtrag (da Billigkeit + Antrag)? Empfehlung: kleine Regel (die
   /verteilungsjahre-Mechanik ist echt), Antrag/Härte als Bedingung.
3. **R2 verteilungsjahre als int (1/2/3)**; Boundary Jahr=1 → voll — bestätigen.
4. Übergangsgewinn kann negativ (Übergangsverlust) — gleichmäßig verteilt, bestätigen.
5. Cap-Wort Stufe B: **2 Regeln** (R1 1-quellig §15a, R2 1-quellig verwaltung) → Vorschlag `--cost-cap 0.20`.
   Dann ist das Großkomplex-Programm KOMPLETT (W1+W2+W3+W4).
