# DBA-Methoden-Katalog — Türkei (W4-Standard, Art. 22, Instructor-Review)

**Kein Kaskaden-Lauf, $0, LLM-frei.** DBA-Methoden-Katalog Türkei (Paket 6, W1). Standard-W4
(OECD-konforme Nummerierung, amtlicher DE-Text im gefreezten Grundabkommen). Geltend:
**Abkommen Deutschland–Türkei vom 19.09.2011** (BGBl. 2012 II S. 526), **einfassig VZ 2024–2026**
(MLI benannt, aber KEIN Anwendungsgesetz — siehe MLI-Negativ-Beleg unten). Methodenartikel
**Art. 22 Abs. 2** (deutsche Seite). Andockung: **Freistellung → § 32b**, **Anrechnung → § 34c_1**.
Kein Rechenkern.

## ⚠ Interleave-Disziplin — Grundabkommen DE/EN zweisprachig

Der Freeze `dba_tr_abkommen_2011` ist DE/EN-interleaved (deutscher Block, dann englische Übersetzung)
mit BGBl-Kopfzeilen zwischen den Blöcken. **Alle Zitatanker stammen aus zusammenhängenden deutschen
Blöcken**, voll-Länge `_normalize`-verifiziert (W1-Skript). Kein Anker aus einem EN-Block oder über eine
Kopfzeile. PDF-Silbentrennungen mit Leerzeichen (z. B. „Be stimmungen", „Doppel besteuerung") wurden
durch Anker gemieden, die vor dem Umbruch enden.

## Methodenartikel Art. 22 Abs. 2 (deutsche Methode)

- **Buchst. a — FREISTELLUNG (Default) → § 32b.** Anker: `Von der Bemessungsgrundlage der deutschen
  Steuer werden die Einkünfte aus der Türkei ausgenommen, die nach diesem Abkommen in der Türkei
  besteuert werden können und nicht unter Buchstabe b fallen`. → `dba_methode = freistellung`.
- **Schachteldividenden-Freistellung (a) — Beteiligungsschwelle 25 %** (höher als ES/OECD-üblich 10 %).
  Anker: `25 Prozent unmittelbar der deutschen Gesellschaft gehört`. Dividenden nur freigestellt, wenn
  an eine DE-Kapitalgesellschaft (nicht PersGes) von einer TR-Gesellschaft mit ≥ 25 % Direktbeteiligung
  gezahlt und beim Ausschütter nicht abgezogen. Sonst → Anrechnung (b aa).
- **Buchst. b — ANRECHNUNG (enumerierte Ausnahmen) → § 34c.** Anker (Intro): `wird unter Beachtung der
  Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die türkische
  Steuer angerechnet`. Enum aa–gg direkt aus dem Freeze gepinnt:
  - aa) `Dividenden, die nicht unter Buchstabe a fallen` (Streubesitz < 25 %);
  - bb) `Zinsen` (Art. 11);
  - cc) `Lizenzgebühren` (Art. 12);
  - dd) `Einkünfte, die nach Artikel 13 Absätze 2 und 5 in der` [Türkei besteuert werden können]
    (Veräußerungsgewinne);
  - ee) `Einkünfte, die nach Protokollziffer 6 zu Artikel 15 in der` [Türkei besteuert werden können]
    (Protokoll-Sonderregel zur nichtselbständigen Arbeit);
  - ff) `Aufsichtsrats- und Verwaltungsratsvergütungen` (Art. 16);
  - gg) `Einkünfte, die nach Artikel 17 besteuert werden können` (Künstler/Sportler).
- **Buchst. c — AKTIVITÄTSKLAUSEL (Umschaltung a→b) → Anrechnung.** Anker: `Statt der Bestimmungen des
  Buchstabens a sind die` [Bestimmungen des Buchstabens b anzuwenden]. Für **Art. 7 (Unternehmensgewinne)
  und Art. 10 (Dividenden)** gilt statt Freistellung die Anrechnung, wenn kein Nachweis aktiver Tätigkeit
  (§ 8 Abs. 1 Nr. 1 bis 6 AStG). → Geltungsbedingung.
- **Buchst. d — PROGRESSIONSVORBEHALT.** Anker: `Die Bundesrepublik Deutschland behält aber das Recht,
  die nach den Bestimmungen dieses Abkommens von der deutschen Steuer ausgenommenen Einkünfte bei der
  Festsetzung ihres Steuersatzes zu berücksichtigen`. → materialisiert über `p32b_progressionsvorbehalt`.

## Katalog: Einkunftsart → Methode → Kanal → Quelle

| Einkunftsart (TR-Zählung) | Methode | `dba_methode` | Kanal | Quelle | Anker |
|---|---|---|---|---|---|
| Immobilien (Art. 6) | Freistellung + Prog | freistellung | § 32b | Art. 22 Abs. 2 a | TR-frei-a |
| Unternehmensgewinne / Betriebsstätte (Art. 7) — mit Aktivitätsnachweis | Freistellung + Prog | freistellung | § 32b | Art. 22 Abs. 2 a | TR-frei-a |
| **Art. 7 / Art. 10 OHNE Aktivitätsnachweis (§ 8 Abs. 1 Nr. 1–6 AStG)** | **Anrechnung** | anrechnung | § 34c | Art. 22 Abs. 2 c | TR-umschalt-c |
| Schachteldividenden (≥ 25 %, Art. 10) | Freistellung | freistellung | § 32b | Art. 22 Abs. 2 a | TR-schachtel25 |
| Streubesitzdividenden (< 25 %, Art. 10) | **Anrechnung** | anrechnung | § 34c | b aa | TR-enum-aa |
| Zinsen (Art. 11) | **Anrechnung** | anrechnung | § 34c | b bb | TR-enum-bb |
| Lizenzgebühren (Art. 12) | **Anrechnung** | anrechnung | § 34c | b cc | TR-enum-cc |
| Veräußerungsgewinne (Art. 13 Abs. 2/5) | **Anrechnung** | anrechnung | § 34c | b dd | TR-enum-dd |
| nichtselbst. Arbeit — Protokollziffer 6 zu Art. 15 | **Anrechnung** | anrechnung | § 34c | b ee | TR-enum-ee |
| Aufsichts-/Verwaltungsrat (Art. 16) | **Anrechnung** | anrechnung | § 34c | b ff | TR-enum-ff |
| Künstler/Sportler (Art. 17) | **Anrechnung** | anrechnung | § 34c | b gg | TR-enum-gg |
| nichtselbst. Arbeit (Art. 15, kein Protokoll-6-Fall) | Freistellung + Prog | freistellung | § 32b | Art. 22 Abs. 2 a | TR-frei-a |
| Ruhegehälter/Renten (Art. 18) | Freistellung + Prog¹ | freistellung | § 32b | Art. 22 Abs. 2 a | TR-frei-a |
| übrige TR-Quellen-Einkünfte (Default) | Freistellung + Prog | freistellung | § 32b | Art. 22 Abs. 2 a | TR-frei-a |

¹ Ruhegehälter (Art. 18) ist NICHT in der Anrechnungs-Enum b aa–gg → Default-Freistellung. Grenzüber-
schreitende Renten DE↔TR (Diaspora) sind Gegenstand einer **Verständigungsvereinbarung 2014** (benannter
Zweitbeleg-Nachtrag, Auflage 4 — Kassenstaats-/Ansässigkeits-Zuordnung, Sozialversicherungsrenten);
Zweitbeleg, KEIN Primäranker.

## ⚠ MLI-Negativ-Beleg (Auflage 2) — TR bleibt VZ 2024–2026 unverändert

Das BEPS-MLI **benennt** die Türkei als Vertragspartei, aber es existiert **KEIN
BEPS-MLI-Anwendungsgesetz** für das DBA-Türkei. Nach der DE-Auswahlentscheidung zu Art. 35 Abs. 7
BEPS-MLI wird ein MLI-Modifikation erst nach einem Anwendungsgesetz wirksam (Anker `erfassten
Steuerabkommens aus Gründen der Rechtssicherheit und -klarheit jedoch erst nach Abschluss eines
nachfolgenden Anwendungsgesetzgebungsverfahrens`, bmf_stand_dba_2026).

**Abwesenheits-Argument (KEIN deckt_ab-Anker erzwungen):** In der bmf_stand-I.2-Tabelle „Abkommen, auf
die das BEPS-MLI-Anwendungsgesetz anzuwenden ist" ist die **Türkei NICHT gelistet** (dort stehen u. a.
Frankreich, Spanien, Kroatien, Malta, Slowakei, Ungarn — je ab 01.01.2025 —, aber nicht die Türkei).
Der Negativtest `Türkei 2024 205 2025 5 01.01.2025` FEHLT im bmf_stand (W1-Skript, greift). → **VZ
2024–2026 einfassig am unveränderten DBA-Wortlaut**, kein MLI-Overlay. Das Fehlen ist die Beweisform
(Abwesenheit in der amtlichen Positivliste), nicht ein zu pinnender Anker.

## Andockung + Nachträge

Andockung: (`dba_staat = TR`, Einkunftsart) → `dba_methode` → `p32b` / `p34c_1` (per-country
`dba_staat = TR`). Geltungsbedingungs-Paket `dba_methode_tr_katalog`, kein Rechenkern.

**Nachträge / Nicht-Gegenstand:** Renten-/Diaspora-Verständigungsvereinbarung 2014 (Zweitbeleg,
Auflage 4); Aktivitäts-Tatbestand § 8 Abs. 1 Nr. 1–6 AStG (Sachverhalts-Vorfrage); Rückfallklauseln
(subject-to-tax); öffentliche Kassen (Art. 19) Vorrang; Protokollziffer-Details (z. B. Ziffer 6 zu
Art. 15). Zweitbelege liegen in `corpus/kommentare/` bzw. `corpus/dba_text_nwb/` (gitignoriert, NIE
committen) — nur Gegenprobe, nie Primäranker (siehe [[dba-anker-nur-amtlich]]).

## Voll-Länge-Anker-Verifikation

Skript `reports/review/2026-07-16-dba-katalog-estr-anker-verify.py` (`gates._normalize`): alle
TR-Kernanker (Freistellung a, Schachtel 25 %, Anrechnung b + Enum aa–gg, Umschalt c, Prog d)
**voll-Länge OK**. Negativtest `10 Prozent … deutschen Gesellschaft` (statt 25 %) FEHLT in TR;
MLI-Negativtest `Türkei …01.01.2025` FEHLT im bmf_stand. Gesamt 33/33.
