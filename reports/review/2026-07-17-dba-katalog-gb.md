# DBA-Methoden-Katalog — Großbritannien (W4-Standard, Art. 23, Instructor-Review)

**Kein Kaskaden-Lauf, $0, LLM-frei.** DBA-Methoden-Katalog Vereinigtes Königreich (Paket 6, W1).
Standard-W4 (OECD-konforme Buchstaben-Gliederung, amtlicher DE-Text im gefreezten Grundabkommen).
Geltend: **Abkommen Deutschland–Vereinigtes Königreich vom 30.03.2010** (BGBl. 2010 II S. 1333),
**einfassig VZ 2024–2026** für den Methodenartikel (Fassungskette 2014/2021, Art. 23 UNBERÜHRT —
MLI-Negativ-Beleg unten). Methodenartikel **Art. 23 Abs. 1** (deutsche Seite; Abs. 2 = UK-Seite,
nicht Gegenstand). Andockung: **Freistellung → § 32b**, **Anrechnung → § 34c_1**. Kein Rechenkern.

## ⚠ Interleave-Disziplin — Grundabkommen DE/EN zweisprachig + BGBl-Seitenköpfe
Der Freeze `dba_gb_abkommen_2010` ist DE/EN-interleaved (deutscher Block, dann englische Übersetzung),
zusätzlich durch BGBl-Seitenköpfe (`Das Bundesgesetzblatt im Internet …`, `Bundesgesetzblatt Jahrgang
2010 …`, Seitenzahlen) unterbrochen. **Alle Zitatanker stammen aus zusammenhängenden deutschen Blöcken**,
voll-Länge `_normalize`-verifiziert; kein Anker über einen EN-Block, eine Kopfzeile oder eine
hyphen-Break (z. B. „der deutschen Ge-\n[Kopf]\nsellschaft" — der Schachtel-10%-Anker endet vor dem
Umbruch bei „unmittelbar der deutschen"). Verify-Skript `2026-07-17-dba-katalog-gb-anker-verify.py`
(21 OK / 0 Fehler, inkl. 3 Negativtests).

## Methodenartikel Art. 23 Abs. 1 (deutsche Methode)

- **a — FREISTELLUNG (Default) → § 32b.** Anker: `Von der Bemessungsgrundlage der deutschen Steuer
  werden die Einkünfte aus dem Vereinigten Königreich sowie die im Vereinigten Königreich gelegenen
  Vermögenswerte ausgenommen, die nach diesem Abkommen im Vereinigten Königreich tatsächlich besteuert
  werden und nicht unter Buchstabe b fallen`. → `dba_methode = freistellung`.
- **Schachteldividenden-Freistellung (a) — Beteiligungsschwelle 10 %** (OECD-üblich; niedriger als
  TR 25 %). Anker: `wenn diese Dividenden an eine in Deutschland ansässige Gesellschaft (jedoch nicht
  an eine Personengesellschaft) von einer im Vereinigten Königreich ansässigen Gesellschaft gezahlt
  werden` + `deren Kapital zu mindestens 10 vom Hundert unmittelbar der deutschen` [Gesellschaft
  gehört]. Nur an DE-Kapitalgesellschaft (nicht PersGes), ≥ 10 % Direktbeteiligung, beim Ausschütter
  nicht abgezogen. Sonst → Anrechnung (b aa).
- **b — ANRECHNUNG (enumerierte Ausnahmen) → § 34c.** Anker (Intro): `wird unter Beachtung der
  Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die Steuer des
  Vereinigten Königreichs angerechnet`. Enum aa–dd direkt aus dem Freeze:
  - aa) `aa) Dividenden, die nicht unter Buchstabe a fallen` (Streubesitz < 10 %);
  - bb) `bb) Einkünfte, die nach Artikel 13 Absatz 2 (Veräußerungsgewinne) im Vereinigten Königreich
    besteuert werden` [können] (Veräußerungsgewinne Art. 13 Abs. 2);
  - cc) `cc) Aufsichtsrats- und Verwaltungsratsvergütungen` (Art. 15);
  - dd) `dd) Einkünfte, die nach Artikel 16 (Künstler und Sportler) im Vereinigten Königreich besteuert
    werden` [können] (Künstler/Sportler).
- **c — AKTIVITÄTSKLAUSEL (Umschaltung a→b) → Anrechnung.** Anker: `Statt der Bestimmungen des
  Buchstabens a sind die Bestimmungen des Buchstabens b anzuwenden auf Einkünfte im Sinne der Artikel
  7 und 10` + `aus unter § 8 Absatz 1 des deutschen Außensteuergesetzes fallenden Tätigkeiten bezogen
  hat`. Für **Art. 7 (Unternehmensgewinne) und Art. 10 (Dividenden)** gilt statt Freistellung die
  Anrechnung, wenn kein Nachweis aktiver Tätigkeit (§ 8 Abs. 1 AStG). → Geltungsbedingung.
- **d — PROGRESSIONSVORBEHALT.** Anker: `Deutschland behält aber das Recht, die nach den Bestimmungen
  dieses Abkommens von der deutschen Steuer ausgenommenen Einkünfte und Vermögenswerte bei der
  Festsetzung seines Steuersatzes zu berücksichtigen`. → materialisiert über `p32b_progressionsvorbehalt`.
- **e — SWITCH-OVER-KLAUSEL (Qualifikationskonflikt/Notifikation) → Anrechnung** *(benannter Randfall;
  GB ist der erste W1-Staat mit einem Buchstaben e — NICHT unterschlagen).* Anker (Intro): `Ungeachtet
  der Bestimmungen des Buchstabens a wird die Doppelbesteuerung durch Steueranrechnung nach Buchstabe b
  vermieden, wenn`. Zwei Auslöser:
  - aa) **Qualifikationskonflikt**: `in den Vertragsstaaten Einkünfte oder Vermögen unterschiedlichen
    Abkommensbestimmungen zugeordnet oder verschiedenen Personen zugerechnet werden` (außer Art. 9),
    der Konflikt sich nicht nach Art. 26 Abs. 3 lösen lässt und die Einkünfte dadurch unbesteuert/
    niedriger besteuert blieben;
  - bb) **Notifikation**: `Deutschland nach gehöriger Konsultation mit der zuständigen Behörde des
    Vereinigten Königreichs auf diplomatischem Weg dem Vereinigten Königreich andere Einkünfte
    notifiziert`, bei denen Deutschland die Anrechnung anzuwenden beabsichtigt (wirksam ab 1. Tag des
    Folge-Kalenderjahres).
  → Geltungsbedingung `switch_over_qualifikationskonflikt_notifikation` (Umschaltung Freistellung→
  Anrechnung); materialisiert über den Anrechnungskanal `p34c_1` bei Vorliegen. Kein eigener Rechenkern.

## Katalog: Einkunftsart → Methode → Kanal → Quelle

| Einkunftsart (GB-Zählung) | Methode | `dba_methode` | Kanal | Quelle | Anker |
|---|---|---|---|---|---|
| Immobilien (Art. 6) | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a | a-freistellung |
| Unternehmensgewinne / Betriebsstätte (Art. 7) — mit Aktivitätsnachweis | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a | a-freistellung |
| **Art. 7 / Art. 10 OHNE Aktivitätsnachweis (§ 8 Abs. 1 AStG)** | **Anrechnung** | anrechnung | § 34c | Art. 23 Abs. 1 c | c-aktivitaet |
| Schachteldividenden (≥ 10 %, Art. 10) | Freistellung | freistellung | § 32b | Art. 23 Abs. 1 a S. 2 | a-schachtel-10% |
| Streubesitzdividenden (< 10 %, Art. 10) | **Anrechnung** | anrechnung | § 34c | b aa | b-aa-dividenden |
| Zinsen (Art. 11) | Freistellung + Prog¹ | freistellung | § 32b | Art. 23 Abs. 1 a | a-freistellung |
| Lizenzgebühren (Art. 12) | Freistellung + Prog¹ | freistellung | § 32b | Art. 23 Abs. 1 a | a-freistellung |
| Veräußerungsgewinne (Art. 13 Abs. 2) | **Anrechnung** | anrechnung | § 34c | b bb | b-bb-veraeusserung |
| Aufsichts-/Verwaltungsrat (Art. 15) | **Anrechnung** | anrechnung | § 34c | b cc | b-cc-aufsichtsrat |
| Künstler/Sportler (Art. 16) | **Anrechnung** | anrechnung | § 34c | b dd | b-dd-kuenstler |
| nichtselbst. Arbeit (Art. 14) | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a | a-freistellung |
| Ruhegehälter/Renten (Art. 17) | Freistellung + Prog² | freistellung | § 32b | Art. 23 Abs. 1 a | a-freistellung |
| **Qualifikationskonflikt / Notifikation (jede Einkunftsart)** | **Anrechnung (Switch-over)** | anrechnung | § 34c | Art. 23 Abs. 1 e aa/bb | e-switchover |
| übrige UK-Quellen-Einkünfte (Default) | Freistellung + Prog | freistellung | § 32b | Art. 23 Abs. 1 a | a-freistellung |

¹ Zinsen/Lizenzgebühren: nach DBA-GB im Quellenstaat regelmäßig 0 % (Art. 11/12), DE stellt frei mit
Progressionsvorbehalt; Aktivitätsvorbehalt c betrifft nur Art. 7/10.
² Ruhegehälter (Art. 17) NICHT in der Anrechnungs-Enum b aa–dd → Default-Freistellung; Sozialversicherungs-
renten/öff. Kassen (Art. 18) Sonderzuordnung = benannter Nachtrag.

## ⚠ MLI-Randnotiz (Auflage 2) — GB bleibt VZ 2024–2026 einfassig

**Abwesenheits-Argument (KEIN deckt_ab-Anker erzwungen, TR-Muster):** In der bmf_stand-I.2-Positivliste
„Abkommen, auf die das BEPS-MLI-Anwendungsgesetz anzuwenden ist" (Staaten je ab 01.01.2025: Frankreich,
Griechenland, Kroatien, Malta, Slowakei, Spanien, Ungarn) ist das **Vereinigte Königreich NICHT
gelistet**. GB erscheint im bmf_stand nur in der allgemeinen DBA-Fundstellen-Tabelle (Zeile
`Vereinigtes Königreich 30.03.2010/… 01.01.2011`). Negativtest `Vereinigtes Königreich …01.01.2025`
FEHLT (Verify-Skript, greift). → **VZ 2024–2026 einfassig am unveränderten Art.-23-Wortlaut**, kein
MLI-Overlay. Das Fehlen ist die Beweisform (Abwesenheit in der amtlichen Positivliste).

**BEPS lief bilateral (Auflage 2, PPT):** Statt über das multilaterale MLI wurde die BEPS-
Mindeststandard-Missbrauchsklausel für GB **bilateral durch das Änderungsprotokoll vom 12.01.2021**
(Freeze `dba_gb_protokoll_2021`) eingefügt — **neuer Art. 30A „Verhinderung von Abkommensmissbrauch"
(Principal-Purpose-Test)**. Anker: `der Erhalt dieser Vergünstigung einer der Hauptzwecke einer
Gestaltung oder Transaktion war`. → **PPT als Missbrauchs-Randnotiz = Geltungsvoraussetzung** (eine
Abkommensvergünstigung — auch die Freistellung/Anrechnung nach Art. 23 — entfällt, wenn ihr Erhalt ein
Hauptzweck einer Gestaltung war), **KEINE Methoden-Änderung**. Materialisierbar als
Geltungsbedingung `keine_ppt_missbrauchsgestaltung_art30a` (Vorfrage), kein Rechenkern.

## ⚠ Fassungsketten-Vermerk (Auflage 3) — Art. 23 durchgehend UNBERÜHRT

Drei Quell-Stücke, Art. 23 in keiner Fassung im Wortlaut geändert (Verify-Skript-belegt):
- `dba_gb_abkommen_2010` — Grundtext, **Art. 23 anker-tragend** (BGBl 2010 II S. 1333).
- `dba_gb_protokoll_2014` (12.01.2015 i.d.F. Protokoll 17.03.2014) — ändert Art. 23 NICHT; erwähnt
  Art. 23 nur als **Cross-Reference** (Art. 23 Abs. 1 Buchst. d für Dienstbezüge von
  Konsulatsangehörigen), kein `wird wie folgt geändert`/`erhält folgende Fassung` am Methodenartikel.
- `dba_gb_protokoll_2021` (12.01.2021) — fügt **Art. 30A (PPT)** an; **Art. 23 = 0 Erwähnungen**, unberührt.
→ Für den Methodenartikel führt GB **einfassig** (2010er Wortlaut) für VZ 2024–2026. (Das 2014er
Protokoll war in dev-2s Routen-Matrix übersehen, jetzt gefreezt — Fassungskette vollständig.)

## Andockung + Nachträge

Andockung wie AT/US/CH/FR/LU/NL/ES/TR: (`dba_staat = GB`, Einkunftsart) → `dba_methode` → `p32b` /
`p34c_1` (per-country `dba_staat = GB`). Geltungsbedingungs-Paket `dba_methode_gb_katalog`, kein Rechenkern.

**Auflage 5 — Cross-Rule-Referenz `kein_dba_mit_quellenstaat` (Paket 10c Block 3, 32c74e7):**
GB ist ein **DBA-Staat** ⇒ die frisch materialisierte Geltungsbedingung `kein_dba_mit_quellenstaat`
(§ 34c Abs. 6 S. 1) **SPERRT den unilateralen § 34c** für GB-Einkünfte — der Katalog regiert:
Freistellungs-Einkünfte → § 32b, Anrechnungs-Einkünfte (b aa–dd / c / e) → § 34c_1 als DBA-
Anrechnungskanal (§ 34c Abs. 6 S. 2: DBA-Anrechnungsmethode gilt Abs. 1 S. 2–5 entsprechend, deshalb
docken die Anrechnungsfälle weiter an p34c_1 an). Der Katalog ist damit die Materialisierung des
§ 34c-Abs.-6-DBA-Vorrangs für GB.

**Nachträge / Nicht-Gegenstand:** Switch-over e (aa/bb) als Geltungsbedingung materialisiert (oben);
PPT/Art. 30A (2021) als Missbrauchs-Vorfrage (oben); Sozialversicherungsrenten/öff. Kassen (Art. 18)
Sonderzuordnung; Aktivitäts-Tatbestand § 8 Abs. 1 AStG (Sachverhalts-Vorfrage); Rückfall-/Subject-to-
tax-Details. Zweitbelege (NWB/Kommentare) NUR Gegenprobe, nie Primäranker (siehe [[dba-anker-nur-amtlich]]).

## Voll-Länge-Anker-Verifikation

Skript `reports/review/2026-07-17-dba-katalog-gb-anker-verify.py` (`gates._normalize`): alle GB-Kernanker
(Freistellung a, Schachtel 10 %, Anrechnung b + Enum aa–dd, Umschalt c, Prog d, Switch-over e aa/bb)
**voll-Länge OK** aus zusammenhängenden deutschen Blöcken; PPT-Anker (Art. 30A, 2021) + GB-DBA-
Fundstelle (bmf_stand) OK. Negativtests: `25 Prozent`-Schachtel (statt 10 %) FEHLT, `§ 9 AStG`
(statt § 8) FEHLT, erfundene Switch-over-Formel FEHLT; MLI-Negativtest `Vereinigtes Königreich …
01.01.2025` FEHLT (Frankreich präsent). **Gesamt 21 OK / 0 Fehler.** Fassungsketten-Check: Art. 23 in
2014/2021 unberührt.
