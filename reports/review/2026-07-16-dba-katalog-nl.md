# DBA-Geltungsbedingungs-Katalog — Niederlande (W4, VZ-Split-Overlay, Instructor-Review)

**Kein Kaskaden-Lauf, $0.** Fünfter DBA-Methoden-Katalog (Paket 3). **ECHTER VZ-SPLIT** wie CH:
zwei Freeze-Fassungen.
- **VZ 2024 / 2025:** „2012 i.d.F. 24.03.2021" — Base `dba_nl_abkommen_2012` (BGBl 2012 II S. 1414)
  + Wort-Tweaks aus `dba_nl_protokoll_2021`. **Keine** Bagatellregelung.
- **VZ 2026:** „2012 i.d.F. 14.04.2025" — Overlay `dba_nl_protokoll_2025` (Umsetzungsges. BGBl 2025 II
  Nr. 270, Inkrafttr.-Bekanntm. BGBl 2025 II Nr. 307, anwendbar 01.01.2026). **Neu: Bagatell
  Art. 14 Abs. 1a (35-Tage)** + Art. 22 Abs. 1 a neu gefasst.

Methodenartikel **Art. 22 Abs. 1** (Deutschland als Ansässigkeitsstaat). Andockung wie AT/US/CH:
**Freistellung → § 32b** (`p32b_progressionsvorbehalt`), **Anrechnung → § 34c** (`p34c_1`/`p34c_2`).
**Kein neuer Rechenkern.**

## ⚠ Interleave-Disziplin (Auflage 2) — NL-Freezes DE/NL zweisprachig

Der NL-Freeze ist bis auf Buchstaben-Ebene DE/NL-interleaved; BGBl-Kopfzeilen + Seitenzahlen stehen
im Textstrom **versetzt zwischen** deutschen Blöcken. Beispiel Base Art. 22 Abs. 1 a: der Satz „…
werden die Einkünfte aus den Niederlanden ausgenommen, die" (Z. 1791) und die Fortsetzung „nach
diesem Abkommen tatsächlich in den Niederlanden besteuert werden…" (Z. 1800) sind durch die
Kopfzeile „Bundesgesetzblatt Jahrgang 2012 Teil II Nr. 38…" (Z. 1796) + Seitenzahl „1429" (Z. 1798)
GETRENNT. **Alle Anker dieses Katalogs stammen aus zusammenhängenden deutschen Blöcken** und wurden
voll-Länge `_normalize`-verifiziert — ein Anker über die Kopfzeilen-Grenze würde nach `_normalize`
FEHLEN (Test fängt das). Die a-Freistellung ist daher in **zwei** contig-Anker zerlegt (Satz 1 +
Subject-to-tax-Teil).

## Methodenartikel Art. 22 Abs. 1

- **Buchst. a — FREISTELLUNG (Default) → § 32b-Kanal**, mit **ausdrücklichem Subject-to-tax**
  („tatsächlich besteuert"):
  - **VZ ≤ 2025** (Quelle **Base 2012**, contig-Stücke): Anker 1 `Von der Bemessungsgrundlage der
    deutschen Steuer werden die Einkünfte aus den Niederlanden ausgenommen, die` + Anker 2 `nach
    diesem Abkommen tatsächlich in den Niederlanden besteuert werden und nicht unter Buchstabe b fallen`.
  - **VZ 2026** (Quelle **Protokoll 2025**, neu gefasst, contig): `Von der Bemessungsgrundlage der
    deutschen Steuer werden die Einkünfte aus den Niederlanden ausgenommen, die nach diesem Abkommen
    tatsächlich in den Niederlanden besteuert werden und nicht unter Buchstabe b fallen` (im 2025-
    Protokoll ohne Seitenumbruch — sauberer Anker als Base). Wortlaut inhaltsgleich, Fassung neu.
  → `dba_methode = freistellung`, Andockung `p32b` (Progressionsvorbehalt über Buchst. d, Base).
- **Buchst. b — ANRECHNUNG (Ausnahmeliste) → § 34c-Kanal** (Quelle **Base 2012**; Kern unverändert).
  Anker: `wird unter Beachtung der Vorschriften des deutschen Steuerrechts über die Anrechnung
  ausländischer Steuern die niederländische Steuer angerechnet`. Betrifft aa) Dividenden (nicht a),
  bb) Art. 13 Abs. 2, cc) Aufsichts-/Verwaltungsrat, dd) Art. 16, ee) Art. 17 Abs. 2–4. →
  `dba_methode = anrechnung`, Andockung `p34c_1`. **2021-Tweak** (`dba_nl_protokoll_2021`, Anker
  `In Artikel 22 Absatz 1 Buchstabe b des Abkommens`): ee)-Verweis „Art. 17 Abs. 2 und 3" → „Abs. 2
  bis 4" — reine Verweis-Änderung, Methoden-Kern unberührt.
- **Buchst. c** — Aktivitätsvorbehalt (§ 8 AStG) für Art. 7/10 + zugehör. Immobilien/Veräußerung (Base).
- **Buchst. d** — Progressionsvorbehalt (Base): `… von der deutschen Steuer ausgenommenen Einkünfte
  bei der Festsetzung ihres Steuersatzes zu berücksichtigen`.

## Katalog: Einkunftsart → Methode → Kanal → VZ-Quelle

| DBA-Artikel (Einkunftsart) | Methode | `dba_methode` | Kanal | Quelle (VZ-Split) | Anker |
|---|---|---|---|---|---|
| Art. 6 unbewegliches Vermögen | Freistellung + Prog | freistellung | § 32b | Base a / 2025 a | Freistellungs-Anker a |
| Art. 7 Unternehmensgewinne (Betriebsstätte) | Freistellung + Prog¹ | freistellung | § 32b | Base a / 2025 a | Freistellungs-Anker a |
| Art. 10 Dividenden (nicht Schachtel) | **Anrechnung** | anrechnung | § 34c | Base b aa | Anrechnungs-Anker b |
| Art. 10 Schachteldividende (≥ 10 %) | Freistellung | freistellung | § 32b | Base/2025 a S. 2 | „…mindestens 10 Prozent unmittelbar…" |
| Art. 13 Abs. 2 Veräußerungsgewinne | **Anrechnung** | anrechnung | § 34c | Base b bb | Anrechnungs-Anker b |
| Art. 13 Abs. 1/3 (Immobilien/BV-Veräußerung) | Freistellung + Prog | freistellung | § 32b | Base a | Freistellungs-Anker a |
| Art. 14 Abs. 1 nichtselbst. Arbeit | Freistellung + Prog | freistellung | § 32b | Base a / 2025 a | Freistellungs-Anker a |
| **Art. 14 Abs. 1a Bagatell (nur VZ 2026)** | Freistellung + Prog | freistellung | § 32b | **2025-Protokoll** | „…weniger als 35 Arbeitstagen…" |
| Aufsichts-/Verwaltungsratsvergütungen | **Anrechnung** | anrechnung | § 34c | Base b cc | Anrechnungs-Anker b |
| Art. 16 Künstler/Sportler | **Anrechnung** | anrechnung | § 34c | Base b dd | Anrechnungs-Anker b |
| Art. 17 Abs. 2–4 Ruhegeh./Renten/SV | **Anrechnung** | anrechnung | § 34c | Base b ee (2021-Verweis) | Anrechnungs-Anker b |
| übrige NL-Quellen-Einkünfte (Default) | Freistellung + Prog | freistellung | § 32b | Base a / 2025 a | Freistellungs-Anker a |

¹ Vorbehaltlich Aktivitätsvorbehalt Buchst. c (§ 8 AStG) → sonst Anrechnung.

## Andockung + Nachträge

Andockung: (`dba_staat = NL`, Einkunftsart, **VZ-abhängige Fassung**) → `dba_methode` →
`p32b` / `p34c_1` (per-country `dba_staat = NL`). Geltungsbedingungs-Paket `dba_methode_nl_katalog`
mit **VZ-Weiche** (≤ 2025 Base / 2026 Overlay) — analog CH-2026-Overlay. Kein Rechenkern.

**Nachträge / Nicht-Gegenstand:** Bagatell Art. 14 Abs. 1a **erst ab VZ 2026** (35-Tage, mit
Drittstaats-Rückausnahme S. 2; Auslegung Prot.-Ziffer XII, Schwellenwert-Tageszählung) als
Geltungsbedingung; keine echte Grenzgängerregelung (Arbeitsortprinzip, ca. 45.000 DE→NL-Pendler);
starker Subject-to-tax-/Aktivitätsvorbehalt (Freistellung nur bei tatsächlicher NL-Besteuerung —
BFH bestätigt Art. 22 Abs. 1 a/b/d); öff. Dienst Art. 18 mit 35-Tage (2025-Protokoll); § 1a KStG /
Investmentsteuer / REIT-Anpassungen (2025-Protokoll — nicht methoden-katalog-relevant, benannt).

## Voll-Länge-Anker-Verifikation

Skript `reports/review/2026-07-16-dba-katalog-frlunl-anker-verify.py` (`gates._normalize`): NL-Anker über `dba_nl_abkommen_2012`
(a-contig-Stücke, b, c, d), `dba_nl_protokoll_2021` (Enum-Tweak) und `dba_nl_protokoll_2025`
(a-Freistellung VZ 2026 + Bagatell) sämtlich `OK` voll-Länge. Alle Anker aus zusammenhängenden
deutschen Blöcken, nie über Spalten-/Seitengrenze (Auflage 2 erfüllt).

## Offener Punkt (Auflage 4)

Die **konsolidierte Neubekanntmachung 11.05.2026** aus dem Recherche-Report (`2026-07-15-dba-
fassung-fr-lu-nl.md`) ist NICHT Grundlage dieses Katalogs — er ankert ausschließlich am
**Änderungsprotokoll-Freeze** `dba_nl_protokoll_2025` (primärverifiziert). Die Neubekanntmachung
bleibt **UNVERIFIZIERT** und wird hier nur als künftiger konsolidierter Lesetext genannt, nicht
still übernommen. Sollte ein konsolidierter VZ-2026-Freeze gewünscht sein: separater
Instructor-Freeze + Anker-Re-Verifikation.
