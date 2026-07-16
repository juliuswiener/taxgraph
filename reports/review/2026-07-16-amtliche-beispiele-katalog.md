# Amtliche Rechenbeispiele — Golden-Kandidaten-Katalog (Paket 9, dev-2)

taxgraph-dev-2, 2026-07-16. Verifikations-Härtung: gefreezte amtliche Quellen nach
konkreten Zahlen-Rechenbeispielen durchsucht (Inputs UND Ergebnis amtlich beziffert).
Je Kandidat: Zitatanker voll-Länge (_normalize-verifiziert gegen Freeze), Inputs,
amtlicher Erwartungswert, abgedeckt_von_regel, Hand-Abgleich. LLM-frei, $0, read-only.
Goldens baut Instructor+dev-1 nach Triage — dies ist der Katalog.

## STRUKTUR-BEFUND VORAB (melde statt improvisiere)
Die **sauber-quantifizierten UND rule-covered** amtlichen Beispiele sind zu großem Teil
**bereits als test_seed geerntet** (das Projekt hat die offensichtlichen Fälle schon
harvested). Verbleibende neue Kandidaten sind **dünner als das Quellvolumen suggeriert**
(bmf_reisekosten: 80 „Beispiel"e, 227 Euro-Beträge — aber ganz überwiegend Mahlzeiten-
Kürzungs-Mechanik, die unsere Stufe-A-Regel p9_4a NICHT abbildet). Die echten NEUEN
HITs konzentrieren sich auf **§ 35c** (bmf_35c_einzelfragen) und die **1 %-Kfz-Regelung**
(EÜR-Anleitung). Details unten.

## A. NEUE Golden-Kandidaten (rule-covered, Hand-Abgleich = HIT)

| # | Quelle | Zitatanker (voll-Länge, _normalize-OK) | Inputs | Amtlicher Wert | Regel | Abgleich |
|---|---|---|---|---|---|---|
| A1 | bmf/bmf_35c_einzelfragen_2025-08-21.txt | „Steuerermäßigung von 7 % (9.100 Euro) im Veranlagungszeitraum 2020" | sanierungsaufwendungen=130000, ist_uebernaechstes_foerderjahr=false | 9.100 € (=910000 ct) | p35c_sanierung_ermaessigung | **HIT** min(7%×130.000, 14.000)=9.100 |
| A2 | bmf/bmf_35c_einzelfragen_2025-08-21.txt | „die Jahreshöchstbeträge der Steuerermäßigung von 14.000 Euro, 14.000 Euro und 12.000 Euro" | sanierungsaufwendungen=200000, ist_uebernaechstes_foerderjahr=false | 14.000 € (Jahr 1/2, Cap) | p35c_sanierung_ermaessigung | **HIT** min(7%×200.000=14.000, 14.000)=14.000 |
| A3 | bmf/bmf_35c_einzelfragen_2025-08-21.txt | (wie A2) | sanierungsaufwendungen=200000, ist_uebernaechstes_foerderjahr=true | 12.000 € (Jahr 3, Cap) | p35c_sanierung_ermaessigung | **HIT** min(6%×200.000=12.000, 12.000)=12.000 |
| A4 | bfinv/euer_2025.txt | „Bruttolistenpreis x Kalendermonate x 1% = Nutzungswert" + „20.000 € x 12 x 1% = 2.400 €" | bruttolistenpreis=20000, bruchteils_teiler=1 | 200 €/Monat (Jahr 2.400 € = ×12) | p6_1_4_kfz_nutzungswert | **HIT** 20.000×1%=200/Monat; ×12=2.400 |

**Details A1/A2/A3 (§ 35c):** amtliche Beispiele 5 (130.000 €, kein Cap) + 6 (200.000 €,
Cap bindet exakt). Regel-Formel aus p35c-hinweis: satz 7 % (6 % im übernächsten Jahr),
hoechst 14.000 (12.000 übernächst), Ermäßigung=min(satz×Aufwand, hoechst). Alle Jahres-
werte cent-exakt getroffen. Gültigkeit: BMF 2025-08-21, § 35c-Sätze (7/7/6 %, Caps
14.000/14.000/12.000, Gesamt 40.000) unverändert → VZ-2020-Beispiel weiter gültig.
A1 impliziter Jahr-3-Wert 7.800 € (min(6%×130.000, 12.000)) = rule-derived, amtlich nicht
explizit beziffert → nur A1-J1 als amtlicher HIT geführt.

**Detail A4 (1 %-Kfz):** Regel-Output ist der MONATSwert (200 €); das amtliche Beispiel
nennt den Jahreswert (2.400 € = 200 × 12). HIT auf Monatsebene; Golden sollte den ×12-
Faktor dokumentieren (Regel = pro Monat by design).

## B. Bereits geerntete amtliche Beispiele (test_seed, kein neuer Kandidat)
| Quelle/Wert | Regel | Status |
|---|---|---|
| „zumutbare Belastung 1.408,70 €" (BFH VI R 75/14; GdE 51.835 stufenweise 2/3/4 %) | p33_3_zumutbare_belastung | test_seed vorhanden |
| „20 % von 10.000 € = 2.000 €" / „Steuerermäßigung 2015 1.410 €" | p35a_2_3_haushaltsnahe | test_seed vorhanden |
| „Zwischentag: 28,00 €" / „Anreisetag: 14,00 €" (Verpflegung Basis) | p9_4a_verpflegungsmehraufwand | test_seed vorhanden |
| „Entfernungspauschale 220 x 20 x 0,30 = 1.320 Euro" → ÖPNV 1.380 € ansetzbar | EP-Kette (Runner) | golden ep_2024_beispiel1_oepnv vorhanden |

## C. NICHT abgedeckt / TEILWEISE (Coverage-Hinweise)
| Bereich | Quelle | abgedeckt_von_regel | Grund |
|---|---|---|---|
| Riester-Förderung (Zulage/Sonderausgabe) | bmf_riester_foerderung_2023-10-05 (52 Beispiele, 56 €-Beträge) | **none** | keine Riester-Regel im Registry |
| Verpflegung Mahlzeiten-Kürzung (Frühstück −5,60 / Mittag-Abend −11,20; 22,40 € u.a.) | bmf_reisekosten_2020-11-25 (~80 Beispiele) | **teilweise** | p9_4a bildet nur Basis-Pauschale (28/14) ab, KEINE Mahlzeiten-Kürzung/Reisekostenerstattung |
| § 35c Gesamt-Höchstbetrag 40.000 € (mehrjährig/objektbezogen, Anrechnungsüberhang) | bmf_35c Beispiel 5/6 Rn. 27-31 | **teilweise** | p35c rechnet je Jahr; Gesamt-Höchstbetrag + Objektbezug + tarifl.-ESt-Deckelung nicht modelliert |
| EÜR § 4 Abs. 4a Schuldzinsen-Hinzurechnung (600 € → 500 €) | bfinv/euer_2025 | **none** | keine § 4 Abs. 4a-Schuldzinsen-Regel im Registry (selbst geprüft) |

## Abgleich-Ergebnis
- **4 neue HIT-Kandidaten** (A1-A4), alle cent-exakt gegen die produktive Regel-Logik
  hand-gerechnet, keine Abweichung → keine FUNDE (kein veralteter Rechtsstand entdeckt).
- Kein Golden hier gebaut (Auftrag: Katalog liefern; Golden-Bau = Instructor+dev-1 nach Triage).

## Repro
```
cd taxgraph-multivz
# Beispiel-Scan:
python3 - <<'EOF'
import re,glob,os
R="."; euro=re.compile(r'\d{1,3}(?:\.\d{3})*,\d{2}')
for f in glob.glob("sources/**/*.txt",recursive=True):
    t=open(f,encoding="utf-8").read()
    if t.count("Beispiel") or len(euro.findall(t))>3: print(f, t.count("Beispiel"))
EOF
# Anker-Verifikation: gates._normalize(anker) in _normalize(freeze) — alle 6 OK (s. Report).
```
Kein Code/Registry-Touch. Reiner Recherche-Katalog.

## Rückfrage an Instructor
1. Sollen die §-35c-Kandidaten (A1-A3) + der 1-%-Kfz-Kandidat (A4) als Goldens gebaut werden?
   § 35c ist heute Registry/clerk-verifiziert, hat aber KEINEN golden/runner.py-Pfad — ein
   Golden bräuchte entweder eine Runner-Andockung (analog GewSt) oder bleibt clerk-seitig.
   (EÜR-§-4-Abs-4a-Schuldzinsen selbst geprüft: keine Regel → als Coverage-Hinweis geführt.)
