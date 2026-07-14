# DBA-Geltungsbedingungs-Katalog — USA (W4, Stufe-A-artig, Instructor-Review)

**Kein Kaskaden-Lauf, $0.** Zweiter DBA-Methoden-Katalog. Gleiche Architektur wie AT: Methoden-
Zuordnung je Einkunftsart → `dba_methode`/`dba_staat`-Interface (Charge 20) → die zwei
verified_bedingt-Kanäle (Freistellung → § 32b, Anrechnung → § 34c). Keine neue Rechenregel.

Quelle: `sources/dba/dba_us_neufassung_2008.txt` (amtliche Neufassung BGBl 2008, inkl. Protokoll
2006). **Methodenartikel Art. 23** (zweisprachig; Deutschland-Teil = Abs. 2). Anker via
`quellen._normalize` verifiziert.

## ⚠ Struktur-Unterschied zu AT: USA ist ANRECHNUNGS-Default (invers)

AT: Freistellung-Default, Anrechnung-Ausnahmeliste. **USA umgekehrt:** Deutschland rechnet die
US-Steuer grundsätzlich AN (Buchst. b), und stellt nur die ENUMERIERTEN Einkünfte FREI (Buchst. a:
Betriebsstätte, Immobilien, Schachteldividenden ≥ 10 %). Das ist der Kern-Unterschied im Katalog.

## Methodenartikel Art. 23 Abs. 2 (Deutschland als Ansässigkeitsstaat)

- **Buchst. a — FREISTELLUNG (enumeriert) → § 32b-Kanal.** Für Einkünfte, die nach dem Abkommen in
  den USA besteuert werden können (Betriebsstätte Art. 7, unbewegliches Vermögen Art. 6) + Schachtel-
  dividenden. Zitatanker `werden von der Bemessungsgrundlage der deutschen Steuer ebenfalls
  Beteiligungen ausgenommen` (Schachtel ≥ 10 %). + Progressionsvorbehalt. → `dba_methode = freistellung`,
  Andockung `p32b_progressionsvorbehalt`.
- **Buchst. b — ANRECHNUNG (Default für den Rest) → § 34c-Kanal.** Zitatanker `unter Beachtung der
  Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die Steuer der
  Vereinigten Staaten angerechnet`. → `dba_methode = anrechnung`, Andockung `p34c_1_anrechnung_hoechstbetrag`.

## Katalog: Einkunftsart-Artikel → Methode → Kanal

| DBA-Artikel (Einkunftsart) | Methode (Art. 23 Abs. 2) | `dba_methode` | Kanal (verified_bedingt) | Anker |
|---|---|---|---|---|
| Art. 7 Unternehmensgewinne (Betriebsstätte) | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 2 a |
| Art. 6 unbewegliches Vermögen (Immobilien) | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 2 a |
| Art. 10 Schachteldividenden (Beteiligung ≥ 10 %) | Freistellung | freistellung | § 32b | Art. 23 Abs. 2 a (Beteiligungen) |
| Art. 10 Streubesitz-Dividenden (< 10 %) | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 2 b |
| Art. 11 Zinsen | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 2 b |
| Art. 12 Lizenzgebühren | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 2 b |
| Art. 13 Veräußerungsgewinne | **Anrechnung** (Default b; Immobilien-Anteile ggf. a) | anrechnung | § 34c | Art. 23 Abs. 2 b |
| Art. 15 nichtselbständige Arbeit (US-Tätigkeit) | Freistellung + Prog (183-Tage/Betriebsstätten-Bezug) | freistellung | § 32b | Art. 23 Abs. 2 a |
| Art. 17 Künstler/Sportler | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 2 b |
| Art. 18/19 Ruhegehälter/öffentl. Dienst | i. d. R. Kassenstaat; DE-ansässig → Anrechnung | anrechnung | § 34c | Art. 23 Abs. 2 b |
| übrige US-Quellen-Einkünfte (Default) | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 2 b |

## Andockung + Nicht-Gegenstand

Identisch zu AT: Katalog liefert je (`dba_staat = US`, Einkunftsart) → `dba_methode`; Integration
wählt den Kanal (`p32b` / `p34c_1` per-country `dba_staat = US`). Geltungsbedingungs-Paket
`dba_methode_us_katalog`, kein Rechenkern.

**Nachträge:** Schachtel-Beteiligungsschwelle 10 % (Art. 10/Art. 23 a — exakte Prozent-/Halte-
Bedingung); US-spezifische Klauseln (Limitation-on-Benefits Art. 28, saving clause Abs. 1 US-Seite,
REIT-Sonderfälle Art. 10); Quellensteuer-Sätze (Art. 10/11/12 = Erstattungs-/Freistellungsverfahren,
kein §-34c-Rechenkern). Die US-Anrechnungs-SEITE (Abs. 1, USA rechnen DE-Steuer an) ist Nicht-
Gegenstand (US-Recht, nicht deutsche ESt).

## Nächster Katalog: CH — ERST NACH PROTOKOLL-ABGLEICH

CH (`dba_ch_konsolidiert_2010`, Methodenartikel Art. 24) **erst nach Abgleich des 2025-Änderungs-
protokolls** (ratifiziert 27.11.2025, BGBl 2025 II Nr. 275 — im Freeze NOCH NICHT enthalten,
Instructor-Caveat). Vor CH-Katalog: prüfen, welche Artikel das Protokoll ändert (Bekanntmachung
2025-12-23 + fedlex-Gegencheck) — sonst Katalog auf veralteter Methoden-Zuordnung.
