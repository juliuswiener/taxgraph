# DBA-Geltungsbedingungs-Katalog — Schweiz (W4, Overlay-Konstruktion, Instructor-Review)

**Kein Kaskaden-Lauf, $0.** Dritter DBA-Methoden-Katalog. **OVERLAY** (Instructor-Auflage):
unveränderte Artikel ankern in `dba_ch_konsolidiert_2010`, die vom **Änderungsprotokoll 2023**
(BGBl 2025 II Nr. 275, in Kraft 27.11.2025, **anwendbar ab 01.01.2026 = VZ 2026**) GEÄNDERTEN
Artikel ankern in `dba_ch_protokoll_2023`. Je Katalog-Zeile ist die Quelle ausgewiesen.

Methodenartikel **Art. 24** (Deutschland als Ansässigkeitsstaat). Alle Anker VOLL-Länge via
`quellen._normalize` verifiziert (Skript-Ausgabe unten, jede Zeile `OK (n Zeichen)`).

## ⚠ Protokoll-2023-Änderungen (VZ 2026), die den Katalog betreffen

- **Art. 14 (selbständige Arbeit) AUFGEHOBEN** → selbständige Arbeit läuft ab VZ 2026 über das
  **Art.-7-Regime** (Unternehmensgewinne, Betriebsstätte). Zitatanker (Protokoll): `Artikel 14 des
  Abkommens wird aufgehoben`.
- **Art. 24 Abs. 1 Nr. 1 Buchst. b NEU gefasst** (Schachteldividenden-Freistellung + Missbrauchs-
  Ausnahmen), **Buchst. c aufgehoben, d→c**. Zitatanker (Protokoll): `Absatz 1 Nummer 1 Buchstabe b
  wird wie folgt gefasst` + `Dividenden an eine in der Bundesrepublik Deutschland ansässige Gesellschaft`.
- **PPT (Principal-Purpose-Test, Art. 23 Abs. 3 neu)** = Missbrauchs-Generalklausel → **benannter
  Nachtrag**, KEIN Rechen-Katalog (keine Methoden-Zuordnung je Einkunftsart).

## Methodenartikel Art. 24 Abs. 1 Nr. 1 (Grundstruktur, konsolidiert)

- **FREISTELLUNG (enumeriert) → § 32b-Kanal.** Zitatanker (konsolidiert, hyphen-frei gewählt — der
  CH-Freeze trennt „Ein- künfte", daher NICHT das Wort einschließen): `die nach den vorstehenden
  Artikeln in der Schweiz besteuert werden können, ausgenommen`. → `dba_methode = freistellung`.
  (Voll-Länge-Verify fing den Silbentrennungs-Bruch am ursprünglich längeren Anker — Regel bewährt.)
- **ANRECHNUNG (Rest) → § 34c-Kanal.** Zitatanker (konsolidiert): `rechnet die Bundesrepublik
  Deutschland in entsprechender Anwendung der Vorschriften des deutschen Rechts über die Anrechnung
  ausländischer Steuern`. → `dba_methode = anrechnung`.

## Katalog: Einkunftsart → Methode → Kanal → Overlay-Quelle

| DBA-Artikel (Einkunftsart) | Methode | `dba_methode` | Kanal | Overlay-Quelle | Anker |
|---|---|---|---|---|---|
| Art. 7 Unternehmensgewinne (Betriebsstätte) | Freistellung + Prog | freistellung | § 32b | konsolidiert (Art. 24 Abs.1 Nr.1 a) | „…ausgenommen: a) Gewinne im Sinne des Artikels 7" |
| Art. 6 unbewegliches Vermögen | Freistellung + Prog | freistellung | § 32b | konsolidiert | Freistellungs-Enum |
| **Selbständige Arbeit (ab VZ 2026 via Art. 7)** | Freistellung + Prog | freistellung | § 32b | **PROTOKOLL** (Art. 14 aufgehoben) | „Artikel 14 des Abkommens wird aufgehoben" |
| **Schachteldividenden (Buchst. b neu)** | Freistellung | freistellung | § 32b | **PROTOKOLL** (Art. 24 Abs.1 Nr.1 b) | „Absatz 1 Nummer 1 Buchstabe b wird wie folgt gefasst" |
| Art. 10 Streubesitz-Dividenden | Anrechnung | anrechnung | § 34c | konsolidiert (Anrechnungs-Rest) | „…über die Anrechnung ausländischer Steuern" |
| Art. 11 Zinsen | Anrechnung | anrechnung | § 34c | konsolidiert | Anrechnungs-Rest |
| Art. 12 Lizenzgebühren | Anrechnung | anrechnung | § 34c | konsolidiert | Anrechnungs-Rest |
| Art. 15 nichtselbständige Arbeit | Freistellung + Prog | freistellung | § 32b | konsolidiert | Freistellungs-Enum |
| Art. 17 Künstler/Sportler | Anrechnung | anrechnung | § 34c | konsolidiert | Anrechnungs-Rest |
| übrige CH-Quellen-Einkünfte (Default) | Anrechnung | anrechnung | § 34c | konsolidiert | Anrechnungs-Rest |

## Andockung + Nachträge

Andockung wie AT/US: (`dba_staat = CH`, Einkunftsart) → `dba_methode` → `p32b` / `p34c_1` (per-country
`dba_staat = CH`). Geltungsbedingungs-Paket `dba_methode_ch_katalog`, kein Rechenkern.

**Nachträge / Nicht-Gegenstand:** PPT Art. 23 Abs. 3 (Missbrauchs-Generalklausel); Art. 24 Abs. 1 Nr. 1
Buchst.-Umnummerierung (c weg, d→c — Detailabgleich); §-4-Abs-3/4-Wegzug (überdachende Besteuerung,
Art. 4); Grenzgänger Art. 15a (CH-spezifisch, 4,5-%-Abzug — eigener Nachtrag); Schachtel-Missbrauchs-
Ausnahmen (Buchst. b neu, Detailbedingungen). Protokoll ändert weiter Art. 2/3/5/7neu/9neu/19/26 —
für die Methoden-Zuordnung der AN-nahen/EÜR-Einkunftsarten nicht katalog-relevant (benannt).

## Voll-Länge-Anker-Verifikation (beide Quellen)

Siehe Commit-Meldung: Skript druckt je Anker `OK/FEHLT (n Zeichen)`; alle CH-Katalog-Anker über
`dba_ch_konsolidiert_2010` (Freistellungs-/Anrechnungs-Anker) UND `dba_ch_protokoll_2023`
(Art.-14-/Art.-24-b-Anker) VOLL-Länge OK.
