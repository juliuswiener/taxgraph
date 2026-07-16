# DBA-Methoden-Katalog — Spanien (W4-Standard, Art. 22, Instructor-Review)

**Kein Kaskaden-Lauf, $0, LLM-frei.** DBA-Methoden-Katalog Spanien (Paket 6, W1). Standard-W4
(OECD-konforme Artikelnummerierung, amtlicher DE-Text im gefreezten Grundabkommen). Geltend:
**Abkommen Deutschland–Spanien vom 03.02.2011** (BGBl. 2012 II S. 18), einfassig für **VZ 2024–2026**
+ **BEPS-MLI-Randnotiz ab VZ 2025** (siehe MLI-Split unten). Methodenartikel **Art. 22 Abs. 2**
(deutsche Seite). Andockung wie AT/US/CH/LU/NL: **Freistellung → § 32b**, **Anrechnung → § 34c_1**.
Kein Rechenkern.

## ⚠ Interleave-Disziplin — Grundabkommen DE/ES zweisprachig

Der Freeze `dba_es_abkommen_2011` ist DE/ES-interleaved (deutscher Block, dann spanische Übersetzung)
mit BGBl-Kopfzeilen zwischen den Blöcken („Das Bundesgesetzblatt im Internet…", „Bundesgesetzblatt
Jahrgang 2012 Teil II Nr. 2…"). **Alle Zitatanker stammen aus zusammenhängenden deutschen Blöcken**,
voll-Länge `_normalize`-verifiziert (W1-Skript). Kein Anker aus einem ES-Block oder über eine Kopfzeile.

## Methodenartikel Art. 22 Abs. 2 (deutsche Methode)

- **Buchst. a — FREISTELLUNG (Default) → § 32b.** Anker: `Von der Bemessungsgrundlage der deutschen
  Steuer werden die Einkünfte aus dem Königreich Spanien sowie die im Königreich Spanien gelegenen
  Vermögenswerte ausgenommen, die nach diesem Abkommen im Königreich Spanien tatsächlich besteuert
  werden und nicht unter Buchstabe b fallen`. → `dba_methode = freistellung`. „tatsächlich besteuert"
  = subject-to-tax-Vorbehalt (Rückfall in Anrechnung bei Nichtbesteuerung — benannter Nachtrag).
- **Schachteldividenden-Freistellung (a) — Beteiligungsschwelle 10 %.** Anker: `Kapital zu mindestens
  10 vom Hundert unmittelbar der deutschen Gesellschaft gehört`. Dividenden nur freigestellt, wenn an
  eine DE-Kapitalgesellschaft (nicht PersGes) von einer ES-Gesellschaft mit ≥ 10 % Direktbeteiligung
  gezahlt und beim Ausschütter nicht abgezogen. Sonst → Anrechnung (b i).
- **Buchst. b — ANRECHNUNG (enumerierte Ausnahmen) → § 34c.** Anker (Intro): `wird unter Beachtung der
  Vorschriften des deutschen Steuerrechts über die Anrechnung ausländischer Steuern die spanische
  Steuer angerechnet`. Enum i–vii direkt aus dem Freeze gepinnt (jeder Punkt voll-Länge verifiziert):
  - i) `Dividenden, die nicht unter Buchstabe a fallen` (Streubesitz < 10 %);
  - ii) `Einkünfte, die nach Artikel 13 Absätze 2 und 3 im Königreich Spanien besteuert werden können`
    (Veräußerungsgewinne beweglich / Betriebsstätte);
  - iii) `Einkünfte, die nach Artikel 14 Absatz 3 im Königreich Spanien besteuert werden können`
    (unselbständige Arbeit Abs. 3 — Bordpersonal Schiff/Luftfahrzeug);
  - iv) `Einkünfte, die nach Artikel 15 im Königreich Spanien besteuert werden können`
    (Aufsichts-/Verwaltungsratsvergütungen);
  - v) `Einkünfte, die nach Artikel 16 im Königreich Spanien besteuert werden können` (Künstler/Sportler);
  - vi) `Einkünfte, die nach Artikel 17 Absätze 2 und 3 im Königreich Spanien besteuert werden können`
    (Ruhegehälter/Renten Abs. 2/3 — Sozialversicherungs-/öffentliche Renten);
  - vii) `Einkünfte aus unbeweglichem Vermögen (einschließlich Einkünften aus der Veräußerung dieses
    Vermögens)`, soweit nicht tatsächlich zu einer ES-Betriebsstätte gehörend.
- **Buchst. c — AKTIVITÄTSKLAUSEL (Umschaltung a→b) → Anrechnung.** Anker: `Statt der Bestimmungen des
  Buchstabens a sind die` [Bestimmungen des Buchstabens b anzuwenden]. Für Einkünfte nach **Art. 7
  (Unternehmensgewinne) und Art. 10 (Dividenden)** gilt statt Freistellung die Anrechnung, wenn kein
  Nachweis aktiver Tätigkeit (§ 8 Abs. 1 AStG) erbracht wird. → Geltungsbedingung.
- **Buchst. d — PROGRESSIONSVORBEHALT.** Anker: `von der deutschen Steuer ausgenommenen Einkünfte und
  Vermögenswerte bei der Festsetzung ihres Steuersatzes zu berücksichtigen`. DE behält den
  Progressionsvorbehalt auf freigestellte Einkünfte → materialisiert über `p32b_progressionsvorbehalt`
  (§ 32b Abs. 1 Nr. 3 EStG). Anders als FR (Fassungsloch) ist der ES-Prog-Satz amtlich verankert.

## Katalog: Einkunftsart → Methode → Kanal → Quelle

| Einkunftsart (ES-Zählung) | Methode | `dba_methode` | Kanal | Quelle | Anker |
|---|---|---|---|---|---|
| Immobilien (Art. 6) | Freistellung + Prog | freistellung | § 32b | Art. 22 Abs. 2 a | ES-frei-a |
| Unternehmensgewinne / Betriebsstätte (Art. 7) — mit Aktivitätsnachweis | Freistellung + Prog | freistellung | § 32b | Art. 22 Abs. 2 a | ES-frei-a |
| **Art. 7 / Art. 10 OHNE Aktivitätsnachweis (§ 8 AStG)** | **Anrechnung** | anrechnung | § 34c | Art. 22 Abs. 2 c | ES-umschalt-c |
| Schachteldividenden (≥ 10 %, Art. 10) | Freistellung | freistellung | § 32b | Art. 22 Abs. 2 a | ES-schachtel10 |
| Streubesitzdividenden (< 10 %, Art. 10) | **Anrechnung** | anrechnung | § 34c | Art. 22 Abs. 2 b i | ES-enum-i |
| Veräußerungsgewinne (Art. 13 Abs. 2/3) | **Anrechnung** | anrechnung | § 34c | b ii | ES-enum-ii |
| unselbst. Arbeit Bordpersonal (Art. 14 Abs. 3) | **Anrechnung** | anrechnung | § 34c | b iii | ES-enum-iii |
| Aufsichts-/Verwaltungsrat (Art. 15) | **Anrechnung** | anrechnung | § 34c | b iv | ES-enum-iv |
| Künstler/Sportler (Art. 16) | **Anrechnung** | anrechnung | § 34c | b v | ES-enum-v |
| Ruhegehälter/Renten (Art. 17 Abs. 2/3) | **Anrechnung** | anrechnung | § 34c | b vi | ES-enum-vi |
| unbewegl. Vermögen nicht-Betriebsstätte | **Anrechnung** | anrechnung | § 34c | b vii | ES-enum-vii |
| nichtselbst. Arbeit (kein Bordpersonal, Art. 14 Abs. 1/2) | Freistellung + Prog | freistellung | § 32b | Art. 22 Abs. 2 a | ES-frei-a |
| übrige ES-Quellen-Einkünfte (Default) | Freistellung + Prog | freistellung | § 32b | Art. 22 Abs. 2 a | ES-frei-a |

## ⚠ MLI-Randnotiz-Split (Auflage 1) — VZ 2024 vs. VZ 2025/26

**Bilateral einfassig** (keine Methoden-Änderung durch das BEPS-MLI), ABER das BEPS-MLI ist für das
DBA-Spanien amtlich **ab 01.01.2025** anwendbar. Amtlicher Beleg = `bmf_stand_dba_2026`:

- **Regel** (warum erst ab Anwendungsgesetz): Anker `erfassten Steuerabkommens aus Gründen der
  Rechtssicherheit und -klarheit jedoch erst nach Abschluss eines nachfolgenden
  Anwendungsgesetzgebungsverfahrens` (DE-Auswahlentscheidung zu Art. 35 Abs. 7 BEPS-MLI).
- **Anwendungsgesetz** Anker: `Das Gesetz zur Anwendung des Mehrseitigen Übereinkommens vom
  24. November 2016 und zu weiteren` (BGBl. 2024 I Nr. 205).
- **Spanien-Zeile** Anker (bmf_stand I.2, „Abkommen, auf die das BEPS-MLI-Anwendungsgesetz anzuwenden
  ist"): `Spanien 2024 205 2025 5 01.01.2025`.

→ **VZ 2024:** DBA-Wortlaut ohne MLI. **VZ 2025/26:** MLI-Missbrauchsklauseln (PPT/Präambel) überlagern
das Abkommen — **keine Methoden-Änderung** (Freistellung/Anrechnung bleiben), nur Missbrauchs-Vorbehalt.
Randnotiz, kein Rechenkern; als benannter Nachtrag geführt (deckt_ab am Stand-Schreiben verankerbar).

## Andockung + Nachträge

Andockung: (`dba_staat = ES`, Einkunftsart) → `dba_methode` → `p32b` / `p34c_1` (per-country
`dba_staat = ES`). Geltungsbedingungs-Paket `dba_methode_es_katalog`, kein Rechenkern.

**Nachträge / Nicht-Gegenstand:** BEPS-MLI-Overlay ab VZ 2025 (Missbrauch/PPT — Randnotiz);
subject-to-tax-Rückfall (a „tatsächlich besteuert") + Rückfallklauseln Art. 22 Abs. 1 e; Aktivitäts-
Tatbestand § 8 AStG (Sachverhalts-Vorfrage); Vermögensteuer-Freistellung (a Satz 2/3 — DE erhebt keine
VSt); Sonderfälle Art. 17 Abs. 1 (private Ruhegehälter → Ansässigkeitsstaat, Freistellung-nah).
Zweitbelege (NWB/Kommentar) nur Gegenprobe, nie Primäranker (siehe [[dba-anker-nur-amtlich]]).

## Voll-Länge-Anker-Verifikation

Skript `reports/review/2026-07-16-dba-katalog-estr-anker-verify.py` (`gates._normalize`): alle
ES-Kernanker (Freistellung a, Schachtel 10 %, Anrechnung b + Enum i–vii, Umschalt c, Prog d) + die drei
bmf_stand-MLI-Anker (Regel, Anwendungsgesetz, Spanien-Zeile) **voll-Länge OK**. Negativtests greifen
(z. B. „25 vom Hundert" statt „10 vom Hundert" FEHLT in ES; „Türkei …01.01.2025" FEHLT im bmf_stand).
Gesamt 33/33.
