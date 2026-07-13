# Charge 11 — Zuschnitt Riester § 10a + §§ 83–86 (Stufe A)

Datum: 2026-07-13 · 6 Freezes sha256 verifiziert (p10a `e76dd3b9`, p79 `1f35e564`, p83 `dfdedeca`,
p84 `88efadaa`, p85 `7184a582`, p86 `4fb64c8f`). Größter Zuschnitt bisher: 2 Mechanik-Blöcke, 5 Regeln.

---

## Sondersatz-Sweep (Leitplanke 7 — alle 6 Freezes, auch/abweichend/an-die-Stelle/erhöht/tritt)

| Fund | Norm | Behandlung |
|---|---|---|
| „erhöht sich die Grundzulage … um einmalig 200 Euro" (Berufseinsteiger < 25) | § 84 S 2 | bool-Input `ist_berufseinsteiger_erstjahr`; Einmaligkeit = State/Nicht-Gegenstand |
| „erhöht sich die Kinderzulage … auf 300 Euro" (Kind nach 31.12.2007) | § 85 S 2 | Geburtsjahr-Schwelle → 2 Zähl-Inputs + param-Sätze (siehe Params) |
| „Ist der Sockelbetrag höher als der Mindesteigenbeitrag … so ist der Sockelbetrag … zu leisten" | § 86 Abs 1 S 5 | **an-die-Stelle-Sondersatz** → `max(…, 60)` in Mindesteigenbeitrag-Formel |
| „bleibt die Erhöhung der Grundzulage nach § 84 Satz 2 außer Betracht" | § 10a Abs 1 S 5 | **abweichend-Sondersatz**: Günstigerprüfung nutzt Zulage OHNE die 200 € → eigener Input `zulageanspruch_guenstigerpruefung` |
| „erhöht sich … um 60 Euro" (Ehegatten-Höchstbetrag) | § 10a Abs 3 S 3 | Ehegatten-Übertragung = Nicht-Gegenstand/Nachtrag |

Alle fünf explizit adressiert, keiner stillschweigend weg.

---

## BLOCK I — Zulage §§ 83–86 (3 Regeln)

### Regel 1 — `p8x_zulage_anspruch` (§§ 83/84/85: Grund + Berufseinsteiger + Kinder, vor Kürzung)
| Feld | Wert |
|---|---|
| Inputs | `anzahl_kinder_185` decimal, `anzahl_kinder_300` decimal, `ist_berufseinsteiger_erstjahr` bool |
| Output | `zulage_anspruch` (vor § 86-Kürzung) |
| Formel | `175 + (ist_berufseinsteiger_erstjahr ? 200 : 0) + anzahl_kinder_185 × 185 + anzahl_kinder_300 × 300` |
| Konstanten | Grundzulage 175 (§ 84 S 1), Berufseinsteiger 200 (§ 84 S 2), Kinder 185/300 (§ 85 S 1/2) — alle **params** (§ 83-Zusammensetzung) |
| Zitatanker | § 84 „beträgt ab dem Beitragsjahr 2018 jährlich 175 Euro"; § 85 „jährlich 185 Euro" + „erhöht sich … auf 300 Euro" |

### Regel 2 — `p86_mindesteigenbeitrag` (§ 86 Abs 1 S 2/4/5)
| Feld | Wert |
|---|---|
| Inputs | `vorjahres_einnahmen` money, `zulage_anspruch` money |
| Output | `mindesteigenbeitrag` |
| Formel | `max( min((4/100) × vorjahres_einnahmen, 2100) − zulage_anspruch, 60 )` |
| Sondersatz | Sockel-höher-Regel S 5 = das äußere `max(…, 60)`; Deckel 2100 (§ 10a-Höchstbetrag) |
| Andockung | `vorjahres_einnahmen` = Sachverhalts-Input (Faktum wie Bruttolohn, KEIN State — Leitplanke 4) |
| Zitatanker | § 86 „4 Prozent der Summe der in dem dem Kalenderjahr vorangegangenen Kalenderjahr" + „Als Sockelbetrag sind ab dem Jahr 2005 jährlich 60 Euro zu leisten" |

### Regel 3 — `p86_zulage_kuerzung` (§ 86 Abs 1 S 6)
| Feld | Wert |
|---|---|
| Inputs | `zulage_anspruch` money, `altersvorsorgebeitraege` money, `mindesteigenbeitrag` money |
| Output | `gewaehrte_zulage` |
| Formel | `zulage_anspruch × min(1, altersvorsorgebeitraege / mindesteigenbeitrag)` |
| Anker | § 86 „Die Kürzung der Zulage ermittelt sich nach dem Verhältnis der Altersvorsorgebeiträge zum Mindesteigenbeitrag" |
| Präzisions-Hinweis | Klasse 5: `zulage × (beitrag/mindest)` = money × decimal → Cent-Schnitt zuletzt; Quote auf 1 gekappt (voller Eigenbeitrag → volle Zulage) |

## BLOCK II — § 10a SA-Abzug + Günstigerprüfung (2 Regeln)

### Regel 4 — `p10a_sonderausgabenabzug` (§ 10a Abs 1 S 1: Deckelung)
| Feld | Wert |
|---|---|
| Inputs | `altersvorsorgebeitraege` money, `zulage_anspruch` money |
| Output | `sonderausgabenabzug` |
| Formel | `min(altersvorsorgebeitraege + zulage_anspruch, 2100)` |
| Anker | § 10a „Altersvorsorgebeiträge (§ 82) zuzüglich der dafür … zustehenden Zulage jährlich bis zu 2 100 Euro als Sonderausgaben abziehen" |

### Regel 5 — `p10a_guenstigerpruefung` (§ 10a Abs 2 + Abs 4 S 1: Andockung)
| Feld | Wert |
|---|---|
| Inputs | `est_ohne_sonderausgabenabzug` money, `est_mit_sonderausgabenabzug` money, `zulageanspruch_guenstigerpruefung` money |
| Output | `zusaetzliche_steuerermaessigung` (über den Zulageanspruch hinausgehend, Abs 4 S 1) |
| Formel | `max(0, (est_ohne_sonderausgabenabzug − est_mit_sonderausgabenabzug) − zulageanspruch_guenstigerpruefung)` |
| Andockung | beide ESt-Werte = Inputs (§ 31/§ 34-Muster). Wenn SA-Vorteil > Zulage → Differenz gewährt (SA günstiger, Abs 2 S 1); sonst 0 (nur Zulage, Abs 2 S 2). Die Hinzurechnung des Zulageanspruchs zur tariflichen ESt = § 2-Integration. |
| ⚠ Sondersatz | `zulageanspruch_guenstigerpruefung` ist die Zulage **OHNE** die § 84-S 2-Erhöhung (§ 10a Abs 1 S 5) — die 200-€-Bereinigung passiert in der Integration, kommt bereinigt als Input |
| Anker | § 10a Abs 2 „Ist der Sonderausgabenabzug … günstiger als der Anspruch auf die Zulage … erhöht sich die … tarifliche Einkommensteuer um den Anspruch auf Zulage" |

---

## Params-Plan (Leitplanke 3 — mit begründetem Push-Back)

`params/riester_p8x.yaml`: grundzulage 175 (Beitragsjahr-versioniert ab 2018, § 32a-Muster),
berufseinsteiger_bonus 200, kinderzulage_vor_2008 185, kinderzulage_ab_2008 300, sockelbetrag 60,
hoechstbetrag 2100, mindesteigenbeitrag_prozent 4.

**Push-Back zur Kohorten-Vorgabe:** Kinderzulage 185/300 ist **keine 54-Zeilen-Kohortentabelle**
wie § 24a/§ 22, sondern **eine binäre Geburtsjahr-Schwelle** (Kind ≷ 31.12.2007). Vorschlag:
**2 Zähl-Inputs** (`anzahl_kinder_185`, `anzahl_kinder_300`) + Sätze als params, KEINE
kohorten/-Tabelle. Die Schwellen-Zuordnung (welches Kind zählt wohin) = § 2-Integration. Sauberer
als eine 2-Zeilen-Pseudo-Kohorte. Falls du auf kohorten/ bestehst, ziehe ich es nach.

## Nicht-Gegenstände / Nachträge (benannt)
- **Berufseinsteiger-Einmaligkeit** (§ 84 S 2/3 „einmalig", erstes Jahr) = Einmal-im-Leben-State analog 40k-Deckel; Anspruch als bool-Input, Tracking außerhalb.
- **Ehegatten-Übertragung / mittelbar Zulageberechtigte** (§ 10a Abs 3, § 79 S 2, § 86 Abs 1 S 2 Hs 2) = eigener Komplex, Nicht-Gegenstand/Nachtrag.
- **Ausländische Alterssicherungssysteme** (§ 10a Abs 6, § 86 Abs 5) = Auslandskomplex, außerhalb AN-Kern.
- **Landwirte** (§ 86 Abs 3), **§ 86 Abs 2 S 2/3** (Entgelt-Sonderfälle Pflege/tatsächliches Entgelt) = Nicht-Gegenstand.
- **Zulageberechtigung §§ 79/10a Abs 1** (Pflichtversicherten-Kreis) = Geltungsbedingung `unmittelbar_zulageberechtigt_p79s1`.
- **Kinderzulage-Zuordnung Elternteil** (§ 85 Abs 2), **Kindergeld-Rückforderung** (§ 85 Abs 1 S 3) = Geltungsbedingung/Integration.

---

## Zuschnittsfragen an Instructor
1. **Alle 5 Regeln in Charge 11**, oder Block I (3) zuerst, Block II (2) danach? (5 Regeln ≈ $0,25–0,35 Stufe B — über bisherigem Rahmen.)
2. **Kinderzulage:** 2 Zähl-Inputs + params (mein Vorschlag) oder doch kohorten/-Tabelle?
3. **Günstigerprüfung §84-S2-Bereinigung:** `zulageanspruch_guenstigerpruefung` als bereinigter Input (mein Vorschlag) ok, oder als eigene Mini-Regel?

Nach Freigabe + Params-Anlage → Stufe B. Kein $-Lauf ohne Wort.
