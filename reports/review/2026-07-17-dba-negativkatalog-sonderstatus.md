# W3 — Sonderstatus-Negativkatalog (RU / BR / VAE / HK)

**Datum:** 2026-07-17 · **Modus:** LLM-frei, $0, keine neuen Freezes (alles aus Bestand)
**Quelle (Primär-Anker):** `sources/bmf/bmf_stand_dba_2026.txt` — sauberer Text-Layer,
kein OCR-Bildscan (echtes `§`, keine `$`-Artefakte).
**Verify:** `reports/review/2026-07-17-dba-negativkatalog-sonderstatus-anker-verify.py` — **22 OK / 0 FEHLER**
(13 Positiv-Anker voll-Länge · 5 Negativtests · 4 Klassen-Kontrollen).

## Zweck

Der `dba_vorhanden = false`-Pfad als **eigene Katalog-Klasse** — das **Komplement** der
11 W1-Positiv-Kataloge (AT/US/CH/FR/LU/NL + ES/TR/GB/DK/PL). Für jeden Staat hier gilt:
**`kein_dba_mit_quellenstaat = true` → unilateraler § 34c REGIERT** (kein Methodenartikel,
keine Freistellung, keine Anrechnung nach DBA). Die Bedingung wurde in Paket 10c Block 3
(`p34c_1`/`p34c_2`, Commit 32c74e7) eingeführt: ein DBA-Staat sperrt den unilateralen § 34c
— der Negativkatalog liefert die **Gegenprobe**, für welche Staaten die Sperre NICHT greift.

Die vier Staaten teilen dasselbe Ergebnis (§ 34c), aber über **vier verschiedene
Rechtswege** — die Statusklasse ist die Pointe, nicht das Ergebnis:

| Staat | Statusklasse | Rechtsgrund | Rest-Abkommen |
|---|---|---|---|
| BR  | **GEKÜNDIGT**    | Voll-DBA (1975) von DE gekündigt, BStBl 2006 I S. 216 | nur Schiff/Luft (S,L) + § 49-Befreiung |
| VAE | **AUSGELAUFEN**  | Voll-DBA befristet, Geltung bis 31.12.2021 | ab 2022 nur S,L, BStBl 2022 S. 640 |
| RU  | **SUSPENDIERT**  | Art. 5–22+24 einseitig ausgesetzt + § 1 Abs. 3 S. 2 StAbwG | Abkommen besteht formal weiter |
| HK  | **KEIN VOLL-DBA** | China-DBA (28.03.2014) in HK nicht anwendbar | nur Luftfahrt (1997) / Schifffahrt (2003) |

---

## 1. RU — SUSPENDIERT (die Sonderklasse, feinste Mechanik)

**Kein Auslauf, keine Kündigung** — RU ist der subtile Fall. Zweistufig:

1. **Einseitige Suspendierung durch RU** (Verbalnote 08.08.2023, „ohne konkrete Angabe
   einer Rechtsgrundlage", „mit sofortiger Wirkung und bis auf Weiteres"): ausgesetzt sind
   **Artikel 5 bis 22 und 24** des DBA vom 29.05.1996 (BGBl. 1996 II S. 2711) i.d.F. des
   Änderungsprotokolls vom 15.10.2007 (BGBl. 2008 II S. 1399) sowie Nummern 2–7 des
   Protokolls. Betrifft sämtliche Einkunftsarten + Suspendierung des Diskriminierungs­verbots
   (Art. 24). **Völkerrechtlich KEINE Aufhebung** — das Abkommen *besteht weiterhin*.
2. **Deutsche Überlagerung seit 01.01.2024**: deutsche Besteuerungsrechte werden durch das
   DBA aufgrund **§ 1 Abs. 3 Satz 2 Steueroasen-Abwehrgesetz** (StAbwG, BGBl. 2021 I S. 2056)
   i.V.m. Durchführungs-VO (20.12.2021) + Zweiter ÄndVO (BGBl. 2023 I Nr. 375) **nicht mehr
   berührt** (zuletzt Dritte ÄndVO vom 20.12.2024, BGBl. 2024 I Nr. 444). RU ist
   nicht-kooperatives Steuerhoheitsgebiet.

**Konsequenz:** Für die deutsche Besteuerung entfaltet das DBA keine Schrankenwirkung mehr
→ `kein_dba_mit_quellenstaat = true`, § 34c regiert. **Die Begründung ist NICHT
„gekündigt/ausgelaufen" (wie BR/VAE), sondern „suspendiert + StAbwG-neutralisiert"** — bei
späterer Wiederanwendung des DBA (Ende der Suspendierung + StAbwG-Entlistung) fällt der
Staat zurück in den Positiv-Pfad. Das ist der Grund für die Sonderbehandlung.

Anker (voll-Länge, Unicode-Quotes gemieden): `ru_verbalnote_2023`,
`ru_artikel_5_bis_22_24`, `ru_aenderungsprotokoll_2007`, `ru_kein_voelkerrechtl_wegfall`,
`ru_stabwg_seit_2024`. — Alle Details im Freeze präsent, kein Melde-Fall.

## 2. BR — GEKÜNDIGT

Das Voll-DBA von 1975 (`corpus/dba_text_mineru/1975-12-23-Brasilien-Abkommen-DBA-Gesetz.md`
= historischer Zweitbeleg) wurde von deutscher Seite gekündigt; seither nur noch das
**Schifffahrt/Luftfahrt-Sonderabkommen** (I.4) und die § 49 Abs. 4 EStG-Steuerbefreiung
(**BStBl 2006 I S. 216**). In der Liste künftiger Abkommen (II.1) steht Brasilien nur mit
„A / V" (Aufnahme / in Verhandlung) — **kein aktives Einkommen-DBA**.

Anker: `br_sonderabkommen_sl`, `br_49_befreiung_2006`. Negativtest `neg_br_altes_vollabkommen`
(„Brasilien 23.12.1975" FEHLT in der aktiven Liste) belegt: das gekündigte Voll-DBA ist raus.

## 3. VAE — AUSGELAUFEN

Das Voll-DBA (Unterzeichnung 01.07.2010) war **befristet** — Geltung **bis 31.12.2021**
(I.1-Einkommenstabelle). Seit 2022 nur noch S/L-Befreiung nach § 49 (**BStBl 2022 S. 640**);
die 2022-Gegenseitigkeitsfeststellung
(`corpus/dba_text_mineru/2022-04-08-Vereinigte-Arabische-Emirate-...-Gegenseitigkeitsfeststellung.md`,
1996er-Abkommen als Kontext) bezieht sich auf diese S/L-Restlage, nicht auf ein Voll-DBA.

Anker: `vae_befristung_2021` (`bis 31.12.2021`), `vae_49_befreiung_2022`. Negativtest
`neg_vae_falschdatum_2031` schützt gegen Datums-Verwechslung.

## 4. HK — KEIN VOLL-DBA (Prüffall, bestätigt)

Hongkong ist seit 01.07.1997 besonderer Teil der VR China (SAR); das allgemeine Steuerrecht
der VR China gilt dort **nicht** → das **DBA DE–VR China vom 28.03.2014 ist in Hongkong
nicht anwendbar**. Es existieren nur Sonderabkommen für **Luftfahrt** (08.05.1997) und
**Schifffahrt** (13.01.2003) — für ESt-Zwecke **kein DBA**. (Analog Macau ab 20.12.1999.)

Anker: `hk_sar_status_1997`, `hk_china_steuerrecht_nicht`, `hk_china_dba_nicht_anwendbar`,
`hk_luftfahrt_ausnahme`. Negativtest `neg_hk_anwendbar` (positive Fassung ohne „nicht" FEHLT).

---

## Negativtest-Logik (Falsch-Grün-resistent)

Die stärksten Diskriminatoren sind die **„ohne-nicht"-Paare**: der Freeze trägt
`...führt völkerrechtlich **nicht** zu einer Aufhebung...` und `...DBA vom 28. März 2014 in
Hongkong **nicht** anwendbar` — die positiven Fassungen (`neg_ru_aufhebung_positiv`,
`neg_hk_anwendbar`) FEHLEN. Das beweist, dass die Anker die **Negation** tragen, nicht die
Affirmation — ein 1-Wort-Fehlgriff würde die Statusklasse invertieren und wird abgefangen.
`neg_ru_gekuendigt` trennt zusätzlich die RU-Suspendierungs-Klasse sauber von der
BR-Kündigungs-Klasse.

## Andockung

Alle vier Staaten: `kein_dba_mit_quellenstaat = true` → `p34c_1`/`p34c_2` (§ 34c Abs. 1/2,
unilaterale Anrechnung) greifen ohne DBA-Sperre; **kein** § 32b-Progressionsvorbehalt aus
DBA-Freistellung (es gibt keine). Komplement zu den 11 Positiv-Katalogen.

**Kein neuer Freeze angelegt** — sämtliche Anker aus `bmf_stand_dba_2026`; die MinerU-Texte
(1975 BR, 1996 VAE) dienen nur als historischer Zweitbeleg, nie als Primär-Anker.
