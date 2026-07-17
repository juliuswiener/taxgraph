# Stufe-A-Zuschnitt — voller Werbungskosten-Ring (Stufe 1b)

**Auftrag:** Instructor — Recon + Zuschnitt VOR Bau (kein Blind-Bau). Aggregiere dHf/Verpflegung/
Arbeitsmittel in `catala_werbungskosten_n` → echter Bescheid für die meisten Angestellten.
**WK bleibt ROH** (§9a im est-Tarif, wie EP). **Kein Code — Recon-Ergebnis + Weg-Vorschlag.**

## Kernbefund: Fall NEIN (keine aufrufbaren Catala-Scopes)

`rules/estg/` hat eigenständige Catala-Module für EP, Arbeitszimmer, §35c, Kinderfreibetrag,
§33a, Tarif, SolZ, AfA-Überhang — **NICHT** für dHf (§9 Abs.1 Nr.5), Verpflegung (§9 Abs.4a),
Arbeitsmittel (§9 Abs.1 Nr.6/7). Die drei sind in `pipeline/produktion/rules.yaml` **registry-
formalisiert** (mit Rechenlogik), aber **nicht als golden/pkg-Scope kompiliert** wie EP →
`catala_werbungskosten_n` kann sie heute nicht aufrufen.

## Je Familie

| Familie | Registry-Rechenlogik? | Komplexität | Accessor-Weg |
|---|---|---|---|
| **dHf** (p9_1_3_nr5) | JA: `unterkunftskosten_monat × monate`, Cap **1000 €/Monat Inland** (2000 € Ausland-Grenze, Ausnahme Dienstwohnung) | niedrig | Python-Andockung trivial |
| **Verpflegung** (p9_4a) | JA: Pauschalen **14 €** (>8 h) / **28 €** (24 h/Zwischentag) / 14 € (An-/Abreise), 3-Monats-Frist, Mahlzeitenkürzung, nur Inland | mittel | Python-Andockung machbar |
| **Arbeitsmittel** (p9_1_3_nr6_7) | **`status: zuschnitt_ersetzt`** → aufgeteilt in `p9_1_3_nr6_7_afa_laufend(_nb)`; GWG-Sofortabzug (≤800 €) = eigene Regel p6_2 (EÜR). AfA **mehrjährig** über Nutzungsdauer (braucht Anschaffungsjahr-Kontext) | hoch | eigenes Paket |

Golden-Handrechnung (Registry-Rechenwege verifiziert): dHf 800 €×12 = **9600 €** (unter Cap);
1400 € → gekappt 1000 €×12 = **12000 €**. Verpflegung 10 Tage >8 h = 10×14 = **140 €**.

WK-Ring-Zielwerte (brutto 40000, VZ2025, handgerechnet über catala_est):
- EP allein (WK 2156) → 6629 € (= MVP).
- **EP + dHf 12000 (WK 14156) → 3143 €.**
- **EP + Verpflegung 140 (WK 2296) → 6585 €.**

## Weg-Optionen (dein Entscheid)

**(A) Python-Andockung in golden/runner** — das etablierte, von dir abgesegnete Muster
(`_vorsorge_abzug`, `_p35c_ermaessigung_cent`, `_kfz_nutzungswert_monat_cent`): `_dhf_abzug` /
`_verpflegung_abzug` als reine Formeln, `catala_werbungskosten_n` summiert EP + diese. Golden gegen
die Registry-Rechenwege (9600/12000) = Konsistenz-Nachweis. **Kein Pipeline-Lauf**, meine Zone,
schnell. Risiko: Zweit-Berechnung neben der Registry-Catala (durch Golden abgedeckt).

**(B) Catala-Modul (rules/estg + Kompilat)** — dHf/Verpflegung als eigenständige Scopes wie EP,
Registry-treu, aber Neu-Formalisierung + Pipeline-Lauf + dein Cap-Wort vorab.

## Vorschlag

1. **dHf JETZT** (Weg A, trivial): `_dhf_abzug = min(unterkunftskosten_monat, 100000ct) × monate`
   (Inland; Ausland-2000 €-Grenze als benannte Lücke bis eigener Schnitt). Golden 9600/12000 +
   WK-Ring-Fall (EP+dHf → 3143 €).
2. **Verpflegung JETZT oder direkt danach** (Weg A): Pauschalen-Tabelle + 3-Monats-Frist +
   Mahlzeitenkürzung. Mehr Logik als dHf — sauber machbar, eigene Goldens.
3. **Arbeitsmittel SPÄTER** (eigenes Paket): AfA ist mehrjährig (zuschnitt_ersetzt → afa_laufend);
   GWG-Sofortabzug ist §4/EÜR (nicht Anlage-N-WK, wie GWG schon geklärt). Bleibt vorerst im Guard.

## Guard-Konsequenz

Die integrierten Familien (dHf, ggf. Verpflegung) fallen aus `GUARD_WERBUNGSKOSTEN` (jetzt
ring-fähig). Nur die noch-nicht-integrierten Felder gaten weiter (AM bleibt, bis Paket AfA).
Der Kegel wächst um die integrierten Felder (bestätigte 0 für den reinen Pendler).

## Offene Entscheide (zur Abnahme)

1. **Weg A (Python-Andockung, Empfehlung) oder B (Catala-Modul)?**
2. **Cap-/Pauschal-Wort:** dHf 1000 €/Monat Inland (2000 € Ausland als Lücke)? Verpflegung 14/28 €?
3. **Familien-Schnitt:** dHf + Verpflegung jetzt, AM als eigenes AfA-Paket später — ok?
4. **Ausland-dHf (2000 €) + Mahlzeitenkürzung Verpflegung:** benannte Lücken in Stufe 1b, oder gleich mit?

Nach Abnahme (Weg + Cap-Wort): Accessor-Ausbau `catala_werbungskosten_n` + Goldens (handverifiziert),
Guard-Anpassung, WK-Ring-e2e. Falls Weg B: Registry-Bedarf + Pipeline-Lauf zuerst.
