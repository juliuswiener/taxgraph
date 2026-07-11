# Charge 2 Stufe B — Konsolidiertes Triage-Paket (nr6_7 + nr5a) nach Redos

Instructor msg 1170, Step 4. Nach Provider-Pin (DeepInfra) + je einem Redo-Lauf
(--force, redoed A+B+judge; Prompts unverändert). Nacht-Summe key-usage: **$0,35**.

## Status nach Redo

| Regel | equiv | clerk | Rest-Fail (seed-gepinnt) |
|---|---|---|---|
| nr6_7 | FAIL | 5/6 | **Letztjahr-Rest** j=3: A modelliert den Endjahr-AfA-Rest nicht |
| nr5a | **PASS** (A==B) | 2/3 | **48-Monats-Cap** {49}→12000: A/B wenden die 1000-€-Kappung via Nr.-5-Verweis nicht an |

Beide massiv verbessert ggü. Batch 1:
- nr6_7: B liefert jetzt Catala (war leer); A's **Zwölftelung ist GEFIXT** — year0 M=7 → 200,00 korrekt (per-Seed verifiziert), 5/6 Seeds bestehen.
- nr5a: **equivalence PASS** (B-Redo fixte B, A==B auf allen 4 Rasterpunkten).

## WICHTIG — Judge-Falschpositive bei nr6_7

Die 3 nr6_7-`abweichung`-Items flaggen A's Anschaffungsjahr-Formel `(12-(Monat-1))/12`
als „nicht gesetzlich". `(12-(M-1))/12 = (13-M)/12` ist aber KORREKT (§7 Abs.1 S.4:
ein Zwölftel je vollem Monat der VORANGEHT = M-1 Zwölftel weg). Per-Seed-Verifikation:
year0 M=7 = 200,00 = mein Oracle. **Diese 3 Abweichungen sind Judge-Falschpositive**
auf nun-korrektem Code → Triage-Vorschlag `nicht_echt`, NICHT als Defekt registrieren.

Der ECHTE nr6_7-Rest (Letztjahr-Rest j=3) taucht NICHT sauber in den Judge-Items auf —
er ist seed-gefangen (clerk 5/6). Entscheidung nötig: ist der Endjahr-Rest (Jahr 4 bei
mid-year-Anschaffung, 1200−200−400−400=200) modellierungspflichtig (dann A-Defekt/Redo)
oder MVP-außerhalb (dann Seed j=3 streichen + Doku)?

## nr5a — echter Gap (kein Falschpositiv)

Der 48-Monats-Cap ({49}→12000) fehlt: A==B geben den ungekappten Wert, obwohl Nr.5a
Satz 4 nach 48 Monaten auf den Nr.-5-Betrag (1000/Monat) kappt. Das ist der
wirkt_hinein-Verweis, den die Signatur trägt, den A/B aber nicht formalisiert haben.
Mein Seed {49}=12000 ist korrekt. Entscheidung: A-Redo mit Fokus (Prompts unverändert →
nur erneuter Lauf, unsicher) oder als Discovery/Bedingung registrieren?

## Discovery-Queue (Judge, zur Triage)

### nr6_7 (24 Items)
- **abweichung** (3, offen) → Vorschlag `nicht_echt` (Judge-Falschpositive auf year0, s.o.).
- **annahme nicht_material** (5, Detektor-vorbelegt): interpretation/rundung → bestätigen.
- **annahme offen** (12): u.a. netto/USt (mappt auf meine Bedingung
  `anschaffungskosten_sind_massgebliche_ak`), nutzungsdauer-Interpretation,
  jahre_seit_anschaffung=0=Anschaffungsjahr, anschaffungsmonat 1..12, Rundung.
- **norm_teil bedingung_neu** (1, Detektor), **norm_teil offen** (1): „ein Zwölftel für
  jeden vollen Monat…" (§7 Abs.1 S.4, wirkt_hinein — deckt sich mit meiner §7-Quelle).
- **norm_teil nicht_material_backlog** (2).

### nr5a (4 Items)
- **annahme bedingung_neu** (1, Detektor), **annahme nicht_material** (1, Detektor).
- **annahme offen** (1): monate_bisher_am_ort-Interpretation.
- **norm_teil offen** (1): „notwendige Mehraufwendungen… Übernachtungen" (Nr.5a Satz 1).

## Meine Fragen an dich

1. nr6_7-Abweichungen als `nicht_echt` (Judge-Falschpositive) bestätigt?
2. Letztjahr-Rest (nr6_7 j=3): modellierungspflichtig (Redo/Defekt) oder MVP-außerhalb (Seed streichen)?
3. 48-Monats-Cap (nr5a): erneuter A-Lauf oder als Bedingung/Discovery registrieren?
4. Rest der offenen Items: schick mir deine Triage-Zeilen (Format wie nichtsaettigung),
   dann discovery-YAMLs editieren → aufnehmen → Regate (wie Charge 1).

Kein weiterer Redo ohne dein Go (Prompts unverändert per Dekret). Budget: $0,35 / 10.
