# Charge 12 — Zuschnitt §23 + §22 Nr.3 + §101 + FW-Disposition (Stufe A)

Datum: 2026-07-13 · letzte 3 Landkarten-Zeilen in einem Schnitt · 4 Regeln + 1 Nicht-Gegenstand-Disposition
Freeze sha256 verifiziert (p23 `56d7d0c8`, p101 `fe8123dd`, p10e `e4ab2943`; §22 Nr3 im bestehenden p22).

## ⚠ FREIGRENZE ≠ FREIBETRAG (kritischer Encoding-Punkt, Klasse 2)

§23 (1.000 €) und §22 Nr.3 (256 €) sind **FREIGRENZEN**: unter der Schwelle ist der GANZE Betrag
steuerfrei; **ab der Schwelle ist der VOLLE Betrag steuerpflichtig** (nicht nur der übersteigende Teil,
kein `gewinn − 1000`). Der hinweis pinnt das + Grenzfall-Wächter am Schwellenwert (1000→1000, 256→256).

## Sondersatz-Sweep (§23/§101/§22 Nr.3)
| Fund | Norm | Behandlung |
|---|---|---|
| „erhöht sich der Zeitraum auf zehn Jahre" (Einkunftsquelle) | §23 Abs1 Nr2 S4 | Frist = Geltungsbedingung, ausgeklammert |
| „tritt an die Stelle des Veräußerungspreises / der AK" (Einlage-Sonderfälle) | §23 Abs3 S2/S3 | Einlage/verdeckte Einlage = Nicht-Gegenstand |
| „Nummer 3 ist nicht anzuwenden" (Subsidiarität) | §22 Nr3 | Geltungsbedingung (soweit nicht anderweitig zugeordnet) |
| Ehegatten: doppelter Grundfreibetrag | §101 S2 | Andockung (GFB als Input, Verdopplung in §2-Integration) |
Kein `auch`/`abweichend` mit eigener Rechtsfolge übrig.

---

## Regel 1 — `p23_veraeusserungsgewinn` (§23 Abs3 S1: Einzelgewinn)
| Feld | Wert |
|---|---|
| Inputs | `veraeusserungspreis` money, `anschaffungs_herstellungskosten` money, `werbungskosten` money |
| Output | `veraeusserungsgewinn` (kann NEGATIV sein = Verlust) |
| Formel | `veraeusserungspreis − anschaffungs_herstellungskosten − werbungskosten` |
| Anker | §23 Abs3 S1 „Unterschied zwischen Veräußerungspreis einerseits und den Anschaffungs- oder Herstellungskosten und den Werbungskosten andererseits" |
| hinweis | Ergebnis KANN NEGATIV sein (Verlust, kein max(0)); AfA-Minderung der AK/HK (Abs3 S4) erfolgt in der §2-Integration → `anschaffungs_herstellungskosten` kommt bereits AfA-gemindert |
| Geltungsbed. | `veraeusserung_innerhalb_frist` (Abs1 Nr1 10J Grundstücke / Nr2 1J andere WG), `nicht_ausschliesslich_eigene_wohnzwecke` (Abs1 Nr1 S3), `kein_gegenstand_taeglichen_gebrauchs` (Abs1 Nr2 S2) |
| Seeds | (200000,150000,5000)→45000 · (100000,120000,3000)→**−23000** (Verlust-Durchreichung) · (50000,50000,0)→0 |

## Regel 2 — `p23_freigrenze` (§23 Abs3 S5: 1.000-€-Freigrenze)
| Feld | Wert |
|---|---|
| Inputs | `gesamtgewinn` money (Summe aller §23-Gewinne im KJ, aggregiert in §2-Integration) |
| Output | `steuerpflichtiger_gewinn` |
| Formel | `gesamtgewinn >= 1000 ? gesamtgewinn : 0` |
| Anker | §23 Abs3 S5 „Gewinne bleiben steuerfrei, wenn der … Gesamtgewinn im Kalenderjahr weniger als 1 000 Euro betragen hat" |
| hinweis | **FREIGRENZE**: Schwelle 1000 INKLUSIV (≥1000 → voller Gewinn steuerpflichtig, NICHT gewinn−1000). Gilt nur für positiven Gesamtgewinn |
| Geltungsbed. | `gesamtgewinn_nicht_negativ` (Verlustverrechnung Abs3 S7-8 = §10d-artiger Mehrjahres-State, Nicht-Gegenstand) |
| Seeds | (5000)→5000 · (999)→0 (Freigrenze) · (1000)→**1000** (Schwelle-Wächter, voll) · (0)→0 |

## Regel 3 — `p22_3_sonstige_leistungen` (§22 Nr.3: 256-€-Freigrenze + WK)
| Feld | Wert |
|---|---|
| Inputs | `einnahmen` money, `werbungskosten` money |
| Output | `steuerpflichtige_einkuenfte` |
| Formel | `(einnahmen − werbungskosten) >= 256 ? (einnahmen − werbungskosten) : 0` |
| Anker | §22 Nr3 S2 „Solche Einkünfte sind nicht einkommensteuerpflichtig, wenn sie weniger als 256 Euro im Kalenderjahr betragen haben" |
| hinweis | **FREIGRENZE** 256 INKLUSIV (Einkünfte ≥256 → voll, NICHT −256). Einkünfte = Einnahmen − WK (S3-WK-Abzug) |
| Geltungsbed. | `leistung_subsidiaer` (Nr3 S1 soweit nicht anderweitig zugeordnet), `einkuenfte_nicht_negativ` (WK-Übersteigen S3 = Nicht-Gegenstand) |
| Seeds | (500,100)→400 · (300,50)→0 (250<256) · (256,0)→**256** (Schwelle-Wächter) · (255,0)→0 · (1000,800)→0 (200<256) |

## Regel 4 — `p101_mobilitaetspraemie` (§101: 14 % Mobilitätsprämie)
| Feld | Wert |
|---|---|
| Inputs | `entfernungspauschale_ab_21km` money, `zu_versteuerndes_einkommen` money, `grundfreibetrag` money |
| Output | `mobilitaetspraemie` |
| Formel | `(14/100) × min(entfernungspauschale_ab_21km, max(0, grundfreibetrag − zu_versteuerndes_einkommen))` |
| Anker | §101 S4 „Die Mobilitätsprämie beträgt 14 Prozent dieser Bemessungsgrundlage" + S2 (Begrenzung auf GFB-Unterschreitung) |
| hinweis | 14 % → /100 (Klasse 5 money×decimal, Cent-Schnitt zuletzt). Bemessung = min(EP ab 21km, GFB-Unterschreitungsbetrag). `grundfreibetrag` = Andockung (§32a, VZ-versioniert; Ehegatten = doppelter GFB + gemeinsames zvE in §2-Integration) |
| Geltungsbed. | `ep_ueber_an_pauschbetrag` (S3: nur soweit EP+WK den AN-Pauschbetrag übersteigen), `nur_ab_21_km` (S1: erst ab 21. vollem km) |
| Seeds | (600,10000,12096)→84 (14 %×600) · (3000,10000,12096)→**293,44** (14 %×2096, gedeckelt auf GFB-Unterschreitung) · (600,15000,12096)→0 (zvE>GFB) · (0,10000,12096)→0 |

---

## FW § 10e/10f/10g — NICHT-GEGENSTAND-Disposition (keine Formalisierung)

**Begründete Disposition** (Instructor-Vorgabe, Freeze estg_p10e liegt vor): § 10e (Förderung
selbstgenutztes Wohneigentum) greift nur für Objekte, die der Steuerpflichtige **„vor dem 1. Januar
1995"** angeschafft/hergestellt hat (Anker wörtlich im Freeze) → **ausgelaufener Altfall-Komplex**,
keine laufende AN-Relevanz mehr (Förderzeitraum 8 Jahre endete spätestens ~2002). § 10f (Baudenkmale
zu eigenen Wohnzwecken) + § 10g (schutzwürdige Kulturgüter) = Nischen-Sonderfälle, nicht AN-nah.
→ Landkarte **🚫 Nicht-Gegenstand mit Grund**, keine Regel.

---

## Zusammenfassung
| Regel | Kern | Freigrenze/Besonderheit |
|---|---|---|
| p23_veraeusserungsgewinn | VP − AK/HK − WK | Verlust-Durchreichung |
| p23_freigrenze | gewinn≥1000 ? gewinn : 0 | FREIGRENZE 1000 |
| p22_3_sonstige_leistungen | (einn−wk)≥256 ? (einn−wk) : 0 | FREIGRENZE 256 |
| p101_mobilitaetspraemie | 14 %×min(EP, GFB−zvE) | Andockung GFB |

4 Regeln, alle multi-quellig-arm (1-2 Quellen) → geschätzt ~$0,35-0,45, dein Cap $0,60 mit Puffer.
Nach deinem Review + Cap-Wort: Stufe B mit `--cost-cap 0.60`. Kein $-Lauf ohne Wort.
