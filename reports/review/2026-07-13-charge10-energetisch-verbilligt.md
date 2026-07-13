# Charge 10 — Zuschnitt § 35c energetische Sanierung + § 21 Abs 2 verbilligte Vermietung (Stufe A)

Datum: 2026-07-13 · 2 Hauptregeln (+ 1 Teilregel-Vorschlag) · Freeze sha256 verifiziert
(p35c `da8052c9…` ✓, §21 Abs2 in bestehendem estg_p21 `…` — kein Re-Freeze).
Instructor-Wortlaut-Korrekturen (msg 1662) eingearbeitet.

---

## Regel 1 — `p35c_sanierung_ermaessigung` (§ 35c Abs 1 S 1: 7/7/6-Staffel)

**Kein "20 %"-Satz** (meine Skizze war falsch, Instructor + Freeze korrigiert). Wortlaut Abs 1 S 1:
Abschlussjahr + Folgejahr **je 7 % (höchstens je 14 000 €)**, übernächstes Jahr **6 % (höchstens
12 000 €)**. Da Jahr 1 = Jahr 2 (7 %/14k), genügt **1 bool** für die Jahres-Verzweigung.

| Feld | Wert |
|---|---|
| Inputs | `sanierungsaufwendungen` money, `ist_uebernaechstes_foerderjahr` bool |
| Output | `sanierung_ermaessigung` (ESt-Ermäßigungsbetrag des VZ) |
| Formel | `satz = ist_uebernaechstes_foerderjahr ? 6 : 7`; `hoechst = ist_uebernaechstes_foerderjahr ? 12000 : 14000`; `sanierung_ermaessigung = min((satz/100) × sanierungsaufwendungen, hoechst)` |
| Zitatanker | Abs 1 S 1 „im Kalenderjahr des Abschlusses der energetischen Maßnahme und im nächsten Kalenderjahr um je 7 Prozent der Aufwendungen des Steuerpflichtigen, höchstens jedoch um je 14 000 Euro und im übernächsten Kalenderjahr um 6 Prozent … höchstens jedoch um 12 000 Euro" |

**hinweis (/100-Encoding + Jahres-Konditional + Andockung):**
> `satz = wenn ist_uebernaechstes_foerderjahr dann 6 sonst 7` (Prozentwert → /100). `hoechst = wenn
> ist_uebernaechstes_foerderjahr dann 12000€ sonst 14000€`. `sanierung_ermaessigung = min((satz/100)
> × sanierungsaufwendungen, hoechst)`. Cent-Schnitt zuletzt. Die Ermäßigung mindert die **tarifliche
> ESt, vermindert um die sonstigen Steuerermäßigungen** (Abs 1 S 1, § 35a-Andock-Reihenfolge) — die
> Verrechnungs-Reihenfolge ist § 2-Integration, NICHT diese Regel. Energieberater-Kosten gehören
> NICHT in `sanierungsaufwendungen` (eigener 50 %-Satz, siehe Regel 1b).

**Seeds:** `(200000, false)→14000` (7 %×200k=14k Deckel greift) · `(100000, false)→7000` (7 %×100k
< 14k) · `(100000, true)→6000` (6 %×100k) · `(250000, true)→12000` (6 %×250k=15k → 12k Deckel) ·
`(0, false)→0`.

### Zuschnittsfrage A: 40 000-€-Objekt-Deckel → **Nicht-Gegenstand (State), analog § 10d Abs 4**

Abs 1 S 5: „je begünstigtes Objekt beträgt der Höchstbetrag der Steuerermäßigung 40 000 Euro" =
**Objekt-Lebensdauer-Deckel über alle Maßnahmen + alle Jahre**, keine Jahresformel. Vorschlag:
**ausklammern als State/Nicht-Gegenstand**, gleiche Logik wie § 10d Abs 4 (Charge 9):
1. Kumuliert über 3 VZ **und** mehrere Einzelmaßnahmen/Objekt → Mehrjahres-/Mehrmaßnahmen-State,
   § 2-Integrations-Territorium.
2. **Rechnerisch redundant im Einzelmaßnahmen-pro-VZ-Fall:** Summe der Jahresdeckel 14k+14k+12k =
   40k ⇒ bei einer Maßnahme ist der 40k automatisch eingehalten. Bindend NUR kumulativ = genau der
   State-Fall.
3. Konsistenz mit der § 10d-Entscheidung (pro-VZ-Mechanik ja, Mehrjahres-Bestand nein).

### Zuschnittsfrage B: Energieberater-Sondersatz (50 %) → **eigene Teilregel 1b** (Empfehlung)

⚠ **Sondersatz-Grep-Regel greift** (frisch ins Judge-Memo). Abs 1 S 4: „die tarifliche
Einkommensteuer vermindert sich **abweichend von Satz 1 um 50 Prozent der Aufwendungen für den
Energieberater**". Darf nicht stillschweigend weg.

**Empfehlung: eigene Teilregel `p35c_energieberater_ermaessigung`:**

| Feld | Wert |
|---|---|
| Inputs | `energieberater_aufwendungen` money |
| Output | `energieberater_ermaessigung` |
| Formel | `(50/100) × energieberater_aufwendungen` |
| Zitatanker | „die tarifliche Einkommensteuer vermindert sich abweichend von Satz 1 um 50 Prozent der Aufwendungen für den Energieberater" |

Begründung: Satz (50 %) + Bemessungsgrundlage (Energieberaterkosten) sind **wortlautklar**. Die
**VZ-Zuordnung** (in welchem der Förderjahre die 50 % wirken) ist wortlaut-offen (BMF-Auslegung:
i.d.R. Abschlussjahr) → **Integration, wie bei Regel 1 die Jahreswahl außerhalb liegt**. Die
Teilregel bildet nur die Satz-Mechanik ab, konsistent zu Regel 1. Geltungsbedingung:
`energieberater_bafa_zugelassen_und_beauftragt` (Abs 1 S 4).

**Alternative (falls Satz↔Verteilung für untrennbar gehalten):** benannter Nachtrag statt Teilregel.
Empfehlung bleibt Teilregel (billig, schließt den Sondersatz sauber).

### Geltungsbedingungen Regel 1 (deklariert)
- `objekt_aelter_zehn_jahre` (Abs 1 S 2, Herstellungsbeginn)
- `ausschliesslich_eigene_wohnzwecke` (Abs 2)
- `kein_doppelabzug_ba_wk_sa_agb` (Abs 3 S 1)
- `keine_10f_35a_oeffentliche_foerderung` (Abs 3 S 2 — Doppelförderungs-Sperre)
- `rechnung_und_zahlung_aufs_konto` (Abs 4)
- `fachunternehmen_bescheinigung` (Abs 1 S 6/7, amtl. Muster)
- 40k-Objektdeckel + Miteigentum-Einmaligkeit (Abs 6) + Rechtsverordnung-Mindestanforderungen (Abs 7) = State/außerhalb.

---

## Regel 2 — `p21_2_verbilligte_vermietung_wk` (§ 21 Abs 2: anteilige WK-Kürzung)

**Zwei Schwellen, nicht eine** (Instructor korrekt). Wortlaut Abs 2:
- **S 1:** Entgelt **< 50 %** ortsüblich → Aufteilung entgeltlich/unentgeltlich **zwingend**.
- **S 2:** Entgelt **≥ 66 %** bei Dauervermietung → gilt als **voll entgeltlich**.
- **Korridor 50–< 66 %:** Totalüberschussprognose (**BMF-Territorium, kein Norm-Wortlaut**) →
  Geltungsbedingung/Nachtrag, **nicht formalisiert**.

| Feld | Wert |
|---|---|
| Inputs | `werbungskosten` money, `entgelt_quote_prozent` decimal (vereinbarte ÷ ortsübliche Miete × 100) |
| Output | `abziehbare_werbungskosten` |
| Formel | `entgelt_quote_prozent >= 66 ? werbungskosten : (entgelt_quote_prozent/100) × werbungskosten` |
| Zitatanker | Abs 2 „Beträgt das Entgelt … weniger als 50 Prozent der ortsüblichen Marktmiete, so ist die Nutzungsüberlassung in einen entgeltlichen und einen unentgeltlichen Teil aufzuteilen. … mindestens 66 Prozent der ortsüblichen Miete, gilt die Wohnungsvermietung als entgeltlich." |

**hinweis (/100 + 66-%-Schwelle-Verzweigung + Korridor-Ausschluss):**
> `entgelt_quote_prozent` ist ein PROZENTWERT (vereinbarte/ortsübliche Miete × 100), → /100.
> `abziehbare_werbungskosten = wenn entgelt_quote_prozent >= 66 dann werbungskosten sonst
> (entgelt_quote_prozent/100) × werbungskosten`. Schwelle 66 INKLUSIV (≥). Bei quote < 50 volle
> anteilige Kürzung. Der Korridor 50 ≤ quote < 66 ist NICHT Gegenstand dieser Regel
> (Totalüberschussprognose, BMF). Cent-Schnitt zuletzt.

**Geltungsbedingung:** `entgelt_quote_ausserhalb_prognosekorridor` — Regel gilt nur für
`quote < 50` ODER `quote ≥ 66`; der Korridor 50–< 66 ist prognoseabhängig, ausgeklammert (Muster
§ 34 `verbleibendes_zve_nicht_negativ`: Formel deckt Kernfall, Sonderfall = deklarierte Bedingung).

**Seeds:** `(10000, 100)→10000` (volle Miete) · `(10000, 70)→10000` (≥66 voll) · `(12000, 66)→12000`
(**Grenzfall-Wächter, Schwelle inklusiv**) · `(10000, 40)→4000` (< 50 anteilig) · `(9000, 30)→2700`.

---

## Zusammenfassung Stufe A

| Regel | Kern | Inputs | Offene Zuschnittsfrage |
|---|---|---|---|
| `p35c_sanierung_ermaessigung` | min(7/6 %×Aufw, 14k/12k) | Aufwendungen, ist_übernächstes_Jahr | 40k-Deckel = State (Vorschlag) |
| `p35c_energieberater_ermaessigung` (1b) | 50 %×Energieberater | energieberater_aufwendungen | Teilregel vs. Nachtrag (Empfehlung: Teilregel) |
| `p21_2_verbilligte_vermietung_wk` | quote≥66 ? WK : quote×WK | WK, entgelt_quote_prozent | Korridor 50–66 = Bedingung (ausgeklammert) |

**Entscheidungen erbeten:** (A) 40k-Deckel als State ok? (B) Energieberater Teilregel oder Nachtrag?
Danach Stufe B (~$0,1–0,15, 3 Regeln). Kein $-Lauf ohne Wort.
