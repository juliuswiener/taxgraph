# Charge 23 — §5 Maßgeblichkeit + RAP-Zeitanteiligkeit (Zuschnitt, Stufe A, 2026-07-15)

W2 Bilanzierung Rest, Zeilen B2 (Maßgeblichkeit) + B3 (Ansatz/RAP). Quelle: `estg_p5_2026-07-14`
(§ 5 Abs. 1 + Abs. 5). **1 Rechenregel + 2 Bedingungs-Pakete.** Kein Stufe-B ohne Cap-Wort. Alle
Zitatanker VOLL-Länge via `_normalize` verifiziert (je Anker `OK (n Zeichen)`).

**STRUKTUR (Instructor-Vorgabe):** § 5 Abs. 5 „Ausgaben vor dem Abschlussstichtag … Aufwand für eine
bestimmte Zeit nach diesem Tag" = **die EINZIGE Rechenmechanik** (aktiver RAP, Zeitanteiligkeit) → eine
money-Regel. § 5 Abs. 1 Maßgeblichkeit (B2) + § 5 Abs. 5 S. 1 Ansatzgebot (B3) = **reine Bedingungs-
Pakete** (keine Rechenkerne; docken an die C21-Bilanzregeln an, wie die DBA-Kataloge).

## Sondersatz-Sweep (verbatim Freeze-Grep)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | § 5 Abs. 1 S. 1 | „das Betriebsvermögen anzusetzen (§ 4 Absatz 1 Satz 1), das nach den handelsrechtlichen Grundsätzen ordnungsmäßiger Buchführung auszuweisen ist" | **Maßgeblichkeit**: Steuerbilanz-BV = Handelsbilanz nach GoB → Bedingung (keine Rechenmechanik), dockt an C21 §4/§6. |
| S2 | § 5 Abs. 1 S. 1 | **„es sei denn"** (steuerliches Wahlrecht/Vorschrift) | Durchbrechung der Maßgeblichkeit → bool-Bedingung (Nachtrag). |
| S3 | § 5 Abs. 5 S. 1 | **„sind nur anzusetzen"** | Abschließendes AnsatzGEBOT (RAP nur bei Zeitbezug) → Bedingung. |
| S4 | § 5 Abs. 5 S. 1 Nr. 1 | **„soweit sie Aufwand für eine bestimmte Zeit nach diesem Tag darstellen"** | Zeitanteiligkeit + Boundary „bestimmte Zeit" (abgrenzbar) → Rechenkern `ausgabe · monate_nach / monate_gesamt`. |

## Regel 1 — § 5 Abs. 5 S. 1 Nr. 1: aktiver Rechnungsabgrenzungsposten (`p5_5_aktiver_rap`)

**Wortlaut (Zitatanker `Ausgaben vor dem Abschlussstichtag, soweit sie Aufwand für eine bestimmte Zeit
nach diesem Tag darstellen`, 105 Zeichen voll-verifiziert):** Ausgabe VOR dem Bilanzstichtag, die
Aufwand für eine bestimmte Zeit NACH dem Stichtag ist, wird zeitanteilig als aktiver RAP abgegrenzt
(der auf die Zeit nach dem Stichtag entfallende Teil).

- **Signatur** `AktiverRap`: `ausgabe: money` (Gesamt-Ausgabe vor Stichtag), `monate_nach_stichtag: int`
  (Monate der Aufwandsperiode NACH dem Stichtag), `monate_gesamt: int` (Gesamt-Laufzeit in Monaten)
  → `aktiver_rap: money`.
- **Rechenkern:** `aktiver_rap = ausgabe · monate_nach_stichtag / monate_gesamt` (der auf die Zeit nach
  dem Stichtag entfallende, abzugrenzende Teil).
- **⚠ Klasse-5-Vermeidung (`/12`-Lehre):** money · int / int mit Division durch `monate_gesamt` ZULETZT,
  Cent-Schnitt am Ende — keine vorgerundete Monatsrate. Monate als int-Inputs (ganzzahlige Perioden),
  nicht als decimal-Quote (vermeidet Klasse-5-Rundungsdrift).
- **Geltungsbedingungen:** `ausgabe_vor_stichtag` (die Ausgabe fällt VOR dem Abschlussstichtag an,
  Sachverhalt), `aufwand_bestimmte_zeit_danach` („bestimmte Zeit" = abgrenzbare, bestimmte Periode;
  unbestimmte Zeit → kein RAP), `nur_aktivseite_ausgabe` (Aktivseite: Ausgabe→Aufwand; die Passivseite
  Einnahme→Ertrag = passiver RAP = Nachtrag), `zeitanteilig_monate_sachverhalt` (monate_nach/gesamt =
  Sachverhalts-Inputs).
- **Seeds (Boundary-Wächter):** (Ausgabe 1200, nach 9, gesamt 12) → 900,00 (Versicherung 1.10., 3 M
  laufender Aufwand, 9 M RAP) · (1200, 3, 12) → 300,00 · (2400, 6, 24) → 600,00 ·
  **(1200, 0, 12) → 0,00 (WÄCHTER: nichts nach Stichtag = kein RAP)** ·
  **(1200, 12, 12) → 1200,00 (GRENZFALL: alles nach Stichtag = ganze Ausgabe RAP)**.

## Bedingungs-Paket B2 — § 5 Abs. 1 S. 1: Maßgeblichkeit (kein Rechenkern)

**Wortlaut (Zitatanker `das nach den handelsrechtlichen Grundsätzen ordnungsmäßiger Buchführung
auszuweisen ist`, 87 Zeichen voll-verifiziert):** Das Steuerbilanz-Betriebsvermögen (§ 4 Abs. 1 S. 1)
ist das nach handelsrechtlichen GoB auszuweisende — Maßgeblichkeit der Handelsbilanz.

- **Reines Bedingungs-Paket** (wie DBA-Katalog): keine money-Regel. Die Bedingung `massgeblichkeit_
  handelsbilanz_gob` (deckt_ab „das nach den handelsrechtlichen Grundsätzen ordnungsmäßiger Buchführung
  auszuweisen ist") **dockt an C21** an: `p4_1_bv_vergleich` (bv_werte_sachverhalt_massgeblichkeit_p5,
  offener Anker aus C21) und die § 6-Bewertungsregeln (die BV-Werte kommen aus der Handelsbilanz nach GoB).
- **Durchbrechung** („es sei denn", steuerliches Wahlrecht/Vorschrift, § 5 Abs. 1 S. 1 HS. 2) = bool-
  Bedingung `massgeblichkeit_durchbrochen_steuervorschrift` = benannter Nachtrag.

## Bedingungs-Paket B3 — § 5 Abs. 5 S. 1: RAP-Ansatzgebot (kein Rechenkern)

**Wortlaut (Zitatanker `Als Rechnungsabgrenzungsposten sind nur anzusetzen`, 50 Zeichen
voll-verifiziert):** RAP sind NUR bei Zeitbezug (Ausgabe/Einnahme vor Stichtag für Zeit danach)
anzusetzen — abschließende Aufzählung.

- **Reines Bedingungs-Paket:** `rap_nur_bei_zeitbezug` (deckt_ab „Als Rechnungsabgrenzungsposten sind
  nur anzusetzen") — Ansatzvoraussetzung für Regel 1 (ohne Zeitbezug kein RAP). Dockt an `p5_5_aktiver_rap`.

## Benannte Nachträge Charge 23

- **§ 5 Abs. 5 S. 1 Nr. 2** passiver RAP (Einnahme vor Stichtag → Ertrag danach, Passivseite) =
  strukturgleiche Regel (`ertrag · monate_nach / monate_gesamt`), Nachtrag.
- **§ 5 Abs. 1 S. 1 HS. 2** Maßgeblichkeits-Durchbrechung (steuerliche Wahlrechte/Vorschriften) = Nachtrag.
- **§ 5 Abs. 1 S. 2 ff.** Wahlrechtsausübungs-Verzeichnis; **Abs. 2** immaterielle WG; **Abs. 2a**
  bedingte Verpflichtungen; **Abs. 3** Rückstellungs-Ansatz; **Abs. 4/4a/4b** Sonderrückstellungen;
  **Abs. 5 S. 2** Disagio; **Abs. 6** Bewertungsvorbehalt; **Abs. 7** Übernahme von Verpflichtungen = eigene Komplexe.
- RAP-Tagesgenauigkeit (statt Monatsraster) = Verfeinerung/Nachtrag.

## Offene Punkte für deine Review

1. **RAP** `ausgabe · monate_nach_stichtag / monate_gesamt` mit Division ZULETZT (`/12`-Lehre,
   Klasse-5-Vermeidung); Monate als **int**-Inputs statt decimal-Quote — bestätigen; Monats- vs.
   Tagesraster (Monat = Standard, Tag = Nachtrag)?
2. **Boundary** monate_nach_stichtag = 0 → 0 (kein RAP) als Wächter-Seed — bestätigen.
3. **B2 Maßgeblichkeit + B3 Ansatz als reine Bedingungs-Pakete** (keine money-Regel, docken an
   C21-Bilanzregeln / an Regel 1) — bestätigen; oder soll B2 als eigener Bedingungs-Katalog (DBA-Muster)
   materialisiert werden?
4. **Passiver RAP** (Nr. 2, Passivseite) als strukturgleicher Nachtrag — bestätigen.
5. Cap-Wort Stufe B: **1 Rechenregel** (RAP, 1-quellig) → Vorschlag `--cost-cap 0.10`. (B2/B3 = keine
   Kaskade.)
