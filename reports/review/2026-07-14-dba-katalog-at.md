# DBA-Geltungsbedingungs-Katalog — Österreich (W4, Stufe-A-artig, Instructor-Review)

**Kein Kaskaden-Lauf, $0.** Der Katalog ordnet je Einkunftsart-Artikel des DBA-AT die
**Methode** (Freistellung / Anrechnung) zu und dockt sie an das **`dba_methode`/`dba_staat`-
Interface** aus Charge 20 an. Die RECHENwirkung läuft vollständig durch die zwei bereits
verified_bedingt-Kanäle: **Freistellung → § 32b** (`p32b_progressionsvorbehalt`, Charge 4),
**Anrechnung → § 34c** (`p34c_1`/`p34c_2`, Charge 20). Der Katalog formalisiert KEINE neue
Rechenregel — er ist die staatsvertragliche Methoden-Zuordnung als Geltungsbedingung.

Quelle: `sources/dba/dba_at_neufassung_2024.txt` (amtliche Neufassung BGBl, ab VZ 2024,
inkl. Protokolle 2010+2023). **Methodenartikel Art. 23** (Deutschland-Teil, Abs. 1). Alle
Anker via `quellen._normalize` verifiziert (NBSP-Standard).

## Methodenartikel Art. 23 Abs. 1 (Deutschland als Ansässigkeitsstaat)

- **Buchst. a — FREISTELLUNG (Default) → § 32b-Kanal.** Zitatanker `werden die Einkünfte aus der
  Republik Österreich sowie`: „Von der Bemessungsgrundlage der deutschen Steuer werden die
  Einkünfte aus der Republik Österreich … ausgenommen, die nach diesem Abkommen in der Republik
  Österreich besteuert werden dürfen und nicht unter Buchstabe b fallen." + **Progressionsvorbehalt**
  (Anker `bei der Festsetzung des Steuersatzes`): „behält aber das Recht, die so ausgenommenen
  Einkünfte … bei der Festsetzung des Steuersatzes … zu berücksichtigen." → `dba_methode = freistellung`,
  Andockung `p32b_progressionsvorbehalt`.
- **Buchst. b — ANRECHNUNG (Ausnahmeliste) → § 34c-Kanal.** Zitatanker `wird unter Beachtung der
  Vorschriften des deutschen Steuerrechts über die Anrechnung`. Betrifft: aa) Dividenden, bb) Zinsen,
  cc) Lizenzgebühren, dd) Art. 13 Abs. 2, ee) Art. 15 Abs. 5, ff) Art. 16 Abs. 1, gg) Art. 17. →
  `dba_methode = anrechnung`, Andockung `p34c_1_anrechnung_hoechstbetrag`.
- **Buchst. c** — Progressionsvorbehalt-Klausel (Rückfall) — als Bedingung dokumentiert.

## Katalog: Einkunftsart-Artikel → Methode → Kanal

| DBA-Artikel (Einkunftsart) | Methode (Art. 23) | `dba_methode` | Kanal (verified_bedingt) | Anker |
|---|---|---|---|---|
| Art. 6 unbewegliches Vermögen (Immobilien) | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a |
| Art. 7 Unternehmensgewinne (Betriebsstätte) | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a |
| Art. 10 Dividenden (nicht Schachtel/Buchst. a) | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 1 b aa |
| Art. 11 Zinsen | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 1 b bb |
| Art. 12 Lizenzgebühren | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 1 b cc |
| Art. 13 Abs. 2 Veräußerungsgewinne (bestimmte) | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 1 b dd |
| Art. 13 (übrige Abs.) Veräußerung | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a |
| Art. 14 selbständige Arbeit (feste Einrichtung) | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a |
| Art. 15 Abs. 1–4 nichtselbständige Arbeit | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a |
| Art. 15 Abs. 5 (bestimmte n.s. Arbeit) | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 1 b ee |
| Art. 16 Abs. 1 Aufsichts-/Verwaltungsrat | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 1 b ff |
| Art. 17 Künstler und Sportler | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 1 b gg |
| Art. 18 Ruhegehälter (privat) | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a (Kassenstaat-Ausnahmen s. Art. 18) |
| Art. 19 öffentlicher Dienst | i. d. R. Kassenstaat → Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a |

## Andockung ans C20-Interface

Der Katalog liefert je (`dba_staat = AT`, Einkunftsart) den Wert `dba_methode ∈ {freistellung,
anrechnung}`. Die § 2-Integration wählt danach den Kanal:
- `dba_methode = freistellung` → Einkünfte NICHT in die Bemessungsgrundlage, ABER in den
  Progressionsvorbehalt (`p32b_progressionsvorbehalt`, § 32b Abs. 1 Nr. 3).
- `dba_methode = anrechnung` → Einkünfte in der Bemessungsgrundlage, ausländische Steuer via
  `p34c_1_anrechnung_hoechstbetrag` gedeckelt (per-country: `dba_staat = AT`).

Als Geltungsbedingungs-Paket `dba_methode_at_katalog` an der Integration deklarierbar (kein
Rechenkern — die Methoden-Zuordnung ist Sachverhalt/Abkommen, die Rechnung läuft durch die
bestehenden Kanäle).

## Nicht-Gegenstand / Nachträge

- Schachteldividenden-Freistellung (Art. 23 Abs. 1 a i. V. m. Art. 10) — Beteiligungsschwelle als
  Sonderbedingung, Nachtrag.
- Rückfallklauseln (Art. 23 Abs. 1 c / Art. 28 subject-to-tax) — als Bedingung benannt, nicht gerechnet.
- Grenzgänger-Sonderregeln (soweit im AT-DBA) — Nachtrag.
- Protokolle 2010/2023 sind in der Neufassung enthalten; ein künftiges Änderungsprotokoll wäre ein
  neuer Freeze.

## Nächste Kataloge

US (Art. 23, Neufassung 2008 + Protokoll 2006), dann CH (Art. 24) — CH erst NACH Abgleich des
2025-Änderungsprotokolls (ratifiziert 27.11.2025, im Freeze noch nicht enthalten, Instructor-Caveat).
