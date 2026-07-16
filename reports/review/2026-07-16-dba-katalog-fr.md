# DBA-Geltungsbedingungs-Katalog — Frankreich (W4, Eigennummerierung, Instructor-Review)

**Kein Kaskaden-Lauf, $0.** Sechster DBA-Methoden-Katalog (Paket 3). FR ist der W4-Sonderfall:
**eigene, nicht-OECD-konforme Artikelnummerierung** und **kein amtlicher DE-Einzeltext** — geltend
ist „Abkommen 21.07.1959 i.d.F. Zusatzabkommen 31.03.2015" (BGBl 2015 II S. 1332), einfassig für
VZ 2024–2026 (+ BEPS-MLI-Randnotiz ab VZ 2025, nur Missbrauchsklauseln).

**Anker-Strategie (Auflage 3): so weit wie möglich am verifizierten Stream `dba_fr_zusabk_2015`
pinnen, NICHT am OCR-1959.** Glücksfall: das Zusatzabkommen 2015 fasst **den gesamten
Methodenartikel Art. 20 Abs. 1 (Buchst. a/c + neuer d) UND die Grenzgängerregelung Art. 13 Abs. 5 a
UND den Fiskalausgleich Art. 13a** neu — alle Katalog-Kernanker liegen damit im verifizierten
(nicht-OCR) Text. `dba_fr_abkommen_1959` (OCR ohne Strom-Beweis) wird nur als Grundtext referenziert;
bild-verifiziert sind dort ohnehin nur Titel (S. 398), Art. 13 Abs. 5 (S. 407), Art. 20-Anfang
(S. 409). Andockung wie AT/US/CH: **Freistellung → § 32b**, **Anrechnung → § 34c**. Kein Rechenkern.

## ⚠ Interleave-Disziplin — ZusAbk-Freeze DE/FR zweisprachig

Wie NL ist der FR-ZusAbk-Freeze DE/FR-interleaved mit versetzten BGBl-Kopfzeilen (z. B. „Das
Bundesgesetzblatt im Internet…" Z. 544/952 + Seitenzahlen zwischen den deutschen Blöcken). Alle
Anker stammen aus zusammenhängenden deutschen Blöcken, voll-Länge `_normalize`-verifiziert.

## Methodenartikel Art. 20 Abs. 1 (alle Buchstaben ZusAbk 2015, neu gefasst)

- **Buchst. a — FREISTELLUNG (Default) → § 32b-Kanal.** Anker: `die Einkünfte aus Frankreich sowie
  die in Frankreich gelegenen Vermögenswerte ausgenommen, die nach diesem Abkommen in Frankreich
  besteuert werden können` (ZusAbk Art. XII Nr. 1, „Satz 1 … ersetzt"). → `dba_methode = freistellung`.
- **Buchst. c — ANRECHNUNG (enumerierte Ausnahmen) → § 34c-Kanal.** Anker: `wird unter Beachtung der
  Vorschriften des deutschen Rechts über die Anrechnung ausländischer Steuern auf die deutsche Steuer
  angerechnet` (ZusAbk Art. XII Nr. 2). **Anrechnungs-Enum direkt aus dem Freeze gepinnt** (löst den
  Nummerierungs-Caveat des Recherche-Reports): `auf die unter Artikel 7 Absatz 4, Artikel 11,
  Artikel 13 Absatz 6 und Artikel 13 b fallenden Einkünfte` — d. h. **Dividenden** (nicht Schachtel-
  Buchst. b), **Art. 7 Abs. 4** (Sondervergütungen/Sonderfälle), **Art. 11** (Aufsichts-/Verwaltungs-
  rat in der FR-Zählung), **Art. 13 Abs. 6** (Arbeitnehmerüberlassung) und **Art. 13 b** (Künstler/
  Sportler). → `dba_methode = anrechnung`, Andockung `p34c_1`.
- **Buchst. d — UMSCHALTKLAUSEL (NEU, ZusAbk Art. XII Nr. 3) → Anrechnung.** Anker: `Doppelbesteuerung
  durch Steueranrechnung nach Buchstabe c vermieden, wenn die Bundesrepublik gegenüber Frankreich auf
  diplomatischem Weg andere Einkünfte notifiziert`. **Beschränkt auf Einkünfte nach Art. 4 und 12**
  („Der vorstehende Satz gilt nur für Einkünfte nach Artikel 4 und 12"). → Bedingung.

## Grenzgänger (W4-Abweichung, nur FR) — Art. 13 Abs. 5 a

**Echte Grenzgängerregelung** (ZusAbk Art. VI Nr. 1, „Abs. 5 Buchst. a … ersetzt", verifizierter
Stream). Anker: `können Einkünfte aus nichtselbständiger Arbeit von Personen, die im Grenzgebiet
eines Vertragsstaats arbeiten und ihre ständige Wohnstätte` + Zuweisung `im Grenzgebiet des anderen
Vertragsstaats haben („Grenzgänger"), nur in diesem anderen Staat besteuert werden`. **Wirkung: das
Besteuerungsrecht liegt allein beim ANSÄSSIGKEITSSTAAT** (nicht am Arbeitsort). Für den DE-ansässigen
Grenzgänger entfällt damit die französische Besteuerung dieses Arbeitslohns → **weder § 32b noch
§ 34c** auf diesen Lohn (reine DE-Besteuerung). Das kennen AT/US/CH/LU/NL so nicht.
→ Geltungsbedingung `grenzgaenger_fr_art13_5` (Sonderfall: `dba_methode` NICHT anwendbar, da kein
ausländisches Besteuerungsrecht), **kein Rechenkern**.

- **Fiskalausgleich Art. 13a** (ZusAbk Art. VII, NEU): Anker `Diese Entschädigung wird auf 1,5 vom
  Hundert der gesamten Bruttojahresvergütungen der Grenzgänger festgelegt`. Zahlungsstrom **zwischen
  den Staaten** — berührt die Steuer des Grenzgängers nicht → **benannt, nicht gerechnet**.
- Grenzzonen-/Nichtrückkehr-Details (45-Tage-Toleranz, Grenzgebiets-Städteliste BMF 16.11.2021,
  BStBl I S. 2230) — Sachverhalts-Bedingung, Nachtrag. Vorrang Art. 14 (öff. Kassen) vor Art. 13 Abs. 5.

## Katalog: Einkunftsart → Methode → Kanal → Quelle

| Einkunftsart (FR-Zählung) | Methode (Art. 20) | `dba_methode` | Kanal | Quelle | Anker |
|---|---|---|---|---|---|
| Immobilien / Unternehmensgewinne (Betriebsstätte) | Freistellung + Prog | freistellung | § 32b | ZusAbk Art. 20 a | Freistellungs-Anker a |
| nichtselbst. Arbeit (kein Grenzgänger) | Freistellung + Prog | freistellung | § 32b | ZusAbk Art. 20 a | Freistellungs-Anker a |
| **nichtselbst. Arbeit — Grenzgänger (Art. 13 Abs. 5)** | **nur Ansässigkeitsstaat** | — (n/a) | weder § 32b noch § 34c | ZusAbk Art. 13 Abs. 5 a | Grenzgänger-Anker |
| Dividenden (nicht Schachtel) | **Anrechnung** | anrechnung | § 34c | ZusAbk Art. 20 c | Anrechnungs-Anker + Enum |
| Art. 7 Abs. 4 (Sonderfälle) | **Anrechnung** | anrechnung | § 34c | ZusAbk Art. 20 c | Enum-Anker |
| Art. 11 (Aufsichts-/Verwaltungsrat) | **Anrechnung** | anrechnung | § 34c | ZusAbk Art. 20 c | Enum-Anker |
| Art. 13 Abs. 6 (Arbeitnehmerüberlassung) | **Anrechnung** | anrechnung | § 34c | ZusAbk Art. 20 c | Enum-Anker |
| Art. 13 b (Künstler/Sportler) | **Anrechnung** | anrechnung | § 34c | ZusAbk Art. 20 c | Enum-Anker |
| Ruhegehälter/Renten inkl. gesetzl. SV (Art. 13 Abs. 8) | Freistellung + Prog¹ | freistellung | § 32b | ZusAbk Art. VI Nr. 2 | „…nur in dem Staat…in dem der Begünstigte ansässig" |
| Art. 4/12 nach Notifikation (Umschaltklausel) | **Anrechnung** | anrechnung | § 34c | ZusAbk Art. 20 d | Umschalt-Anker |
| übrige FR-Quellen-Einkünfte (Default) | Freistellung + Prog | freistellung | § 32b | ZusAbk Art. 20 a | Freistellungs-Anker a |

¹ Art. 13 Abs. 8 (ZusAbk 2015 neu): Ruhegehälter/Renten nur im Ansässigkeitsstaat → für DE-Ansässige
reine DE-Besteuerung (kein Auslands-Besteuerungsrecht, `dba_methode` n/a). Für den Katalog als
Freistellungs-nahe Zuordnung geführt; Sonderfall benannt.

## ⚠ Progressionsvorbehalt — Anker-Lücke (Auflage 3; Herkunft korrigiert n. Instructor-Nachlauf)

Das Zusatzabkommen 2015 ersetzt in Art. 20 Abs. 1 a **nur Satz 1** (Freistellung). Der
**Progressionsvorbehalt-Satz** ist im Freeze-Bestand **derzeit amtlich NICHT belegbar** (nicht bloß
unbequem) — ein verbatim-Anker ist **unmöglich**, nicht optional:

- **Genealogie (korrigiert).** Der bildverifizierte **1961er-Druck** (`bgbl2_1961_ii_18.pdf`, S. 409)
  zeigt Art. 20 Abs. 1 **OHNE Buchstaben-Struktur** — er beginnt „(1) Dieses Abkommen beschränkt nicht
  das Recht des Vertragstaates …". Die heute geltende **a/c/d-Struktur inkl. Progressionsvorbehalt in
  Abs. 1 Buchst. a Satz 2** stammt **NICHT** aus dem Grundtext 1959, sondern aus dem
  **Revisionsprotokoll 09.06.1969** (BGBl 1970 II S. 717), fortgeschrieben durch **ZusAbk 1989**
  (BGBl 1990 II S. 770) und **ZusAbk 2001** (BGBl 2002 II S. 2370). Die frühere Report-Zeile
  „unverändert im Grundtext 1959" war **genealogisch falsch** und ist hiermit berichtigt.
- **Fassungsloch.** Rev-Prot 1969 + ZusAbk 1989 + ZusAbk 2001 sind **allesamt NICHT gefreezt**. Der
  Prog-Satz kann daher an KEINEM vorhandenen Freeze verbatim gepinnt werden — weder am OCR-1959 (falsche
  Fassung, kein Buchst. a Satz 2) noch am ZusAbk 2015 (ändert nur Satz 1).

**Entscheid (Instructor-Nachlauf msg 2089): OPTION 1 angenommen** — Progressionsvorbehalt bleibt
**ungepinnt**. Er läuft ohnehin über `p32b_progressionsvorbehalt` (§ 32b Abs. 1 Nr. 3 EStG, DE-seitig
materialisiert); die Methoden-Zuordnung `freistellung → § 32b` braucht keinen verbatim-DBA-Anker. Ein
künftiger verbatim-Anker setzt einen **Freeze des Rev-Prot 1969** (bzw. der ZusAbk 1989/2001) voraus —
Backlog, nicht Teil dieses Katalogs.

## Andockung + Nachträge

Andockung: (`dba_staat = FR`, Einkunftsart) → `dba_methode` → `p32b` / `p34c_1` (per-country
`dba_staat = FR`), Grenzgänger-Sonderfall separat. Geltungsbedingungs-Paket `dba_methode_fr_katalog`,
kein Rechenkern.

**Gültigkeits-Nachanker (Auflage 5, 2026-07-16):** Die MLI-Randnotiz „ab VZ 2025" ist jetzt **amtlich am
`bmf_stand_dba_2026` verankerbar** (bisher nur Recherche-Beleg): I.2-Tabelle „Abkommen, auf die das
BEPS-MLI-Anwendungsgesetz anzuwenden ist", Anker `Frankreich 2024 205 2025 5 01.01.2025`
(Anwendungsgesetz BGBl 2024 I Nr. 205, `Das Gesetz zur Anwendung des Mehrseitigen Übereinkommens vom
24. November 2016 und zu weiteren`). Beide voll-Länge im W1-Skript
`2026-07-16-dba-katalog-estr-anker-verify.py` verifiziert. Weiterhin **keine Methoden-Änderung**, nur
Missbrauchs-Vorbehalt.

**Nachträge / Nicht-Gegenstand:** BEPS-MLI-Overlay ab VZ 2025 (Missbrauchsklauseln/PPT — Randnotiz,
keine Methoden-Änderung); Grenzgänger-Grenzzone/45-Tage/Städteliste; Fiskalausgleich Art. 13a
(1,5 %) + Art. 13c (Rentenfiskalausgleich, zwischen-staatlich); Vorrang Art. 14 öff. Kassen;
Schachteldividenden-Freistellung Buchst. b (Beteiligungsschwelle — Base, Bild-Verifikation nötig
falls Anker gewünscht). Zusatz-Zweitbelege lokal vorhanden (BMF 16.11.2021 Grenzgebiet, BMF
Zweifelsfragen — nur referenziert, nicht committet).

## Voll-Länge-Anker-Verifikation

Skript `reports/review/2026-07-16-dba-katalog-frlunl-anker-verify.py` (`gates._normalize`): alle FR-Kernanker (Art. 20 a/c/c-Enum/d,
Art. 13 Abs. 5 a Grenzgänger + Zuweisung, Art. 13a Fiskalausgleich) über `dba_fr_zusabk_2015`
sämtlich `OK` voll-Länge. **Kein Anker am OCR-1959.** Progressionsvorbehalt bewusst ungepinnt
(Auflage-3-Meldung oben).
