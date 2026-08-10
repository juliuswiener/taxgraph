# Anlage KAP 2025 — Kz-Zuordnung für die drei offenen Bausteine

**Datum:** 2026-08-10
**Anlass:** `kap-antragsgrund-fehlt` (Abgabe-Blocker), `p32d-q-auslandssteuer`, sowie die
ungebundene Steuerabzugsbeträge-Seite. Alle drei brauchten dieselbe Kz-Recherche.
**Status:** Zuordnung für Stufe 1 und 3 belegt; eine Frage zur zweiten XSD-Serie offen.

## Warum dieser Report

`elster/kz_extract.py` nennt im Docstring die Beweislage-Regel: der XSD-Sektionspfad allein ist
**MITTEL**, erst der amtliche Vordruck-Kreuzcheck (Zeile ↔ Konzept) hebt auf **STARK**. Die
Anlage KAP fehlte in `sources/bfinv/` — die Recherche stand deshalb auf MITTEL und ich hatte
den Bau von Stufe 2/3 zurückgestellt, weil ein falsch gebundenes Kz einen Steuerbetrag an die
falsche Stelle der Erklärung schickt.

Der Vordruck lag in `sources/bfinv_raw/` (Unterstrich), das ich nicht abgesucht hatte:
`Anlage_KAP_2025.pdf`, dazu `040_Anleitung_Anlage_KAP_2025.pdf`, KAP-BET, KAP-INV und die
Jahrgänge 2023/2024.

## Belegte Zuordnung

Doppel-Kz im Vordruck = **Person A / Person B**.

### Seite 1 — Anträge (Stufe 1, der Abgabe-Blocker)

| Zeile | Wortlaut im Vordruck | Kz | XSD |
|---|---|---|---|
| 4 | „Ich beantrage die Günstigerprüfung für sämtliche Kapitalerträge." | 201/401 | E1900401 |
| 5 | „Ich beantrage eine Überprüfung des Steuereinbehalts für bestimmte Kapitalerträge." | 202/402 | E1900501 |
| 6 | Kirchensteuerpflicht, KapESt aber keine KiSt einbehalten | 203/403 | — |

Zeile 4 trägt im Vordruck den Zusatz: *„(Bei Zusammenveranlagung: Die Anlage KAP meines
Ehegatten / Lebenspartners ist beigefügt.)"* — das ist die Formular-Entsprechung zu
§ 32d Abs. 6 S. 4 (Antrag nur für sämtliche Kapitalerträge **beider** Ehegatten).

### Seite 3 — Steuerabzugsbeträge (Stufe 2 und 3)

Blocktitel: **„Steuerabzugsbeträge zu Erträgen in den Zeilen 7 bis 23 und zu Investmenterträgen
laut Anlage KAP-INV"**

| Zeile | Wortlaut | Kz | XSD |
|---|---|---|---|
| 37 | Kapitalertragsteuer | 280/480 | E1904701 |
| 38 | Solidaritätszuschlag | 281/481 | E1904901 |
| 39 | Kirchensteuer zur Kapitalertragsteuer | 282/482 | E1904801 |
| 40 | Angerechnete ausländische Steuern | 283/483 | E1905001 |
| **41** | **Anrechenbare noch nicht angerechnete ausländische Steuern** | **284/484** | **E1905101** |
| 42 | Fiktive ausländische Quellensteuer | 285/485 | E1905201 |

Der Kreuzcheck trägt, weil sich **Blocktitel und Sektionsname wörtlich decken**: „…zu Erträgen
in den Zeilen 7 bis 23 und zu Investmenterträgen" ↔ XSD-Sektion `St_Abz_Betr_Inl_u_Inv_Ert`
(Steuerabzugsbeträge Inländische **u**nd **Inv**estment-**Ert**räge). Zusammen mit der
identischen Bezeichnung je Zeile ist die Zuordnung STARK.

## `q` ist Zeile 41, nicht Zeile 40

§ 32d Abs. 1 S. 5 definiert „q" als *„die nach Maßgabe des Absatzes 5 anrechenbare ausländische
Steuer"*. Der Vordruck trennt zwei Dinge, die man leicht verwechselt:

- **Zeile 40 „Angerechnete"** — von der auszahlenden Stelle bereits verrechnet, kein Veranlagungs-`q`.
- **Zeile 41 „Anrechenbare noch nicht angerechnete"** — genau das, was in der Veranlagung nach
  Abs. 5 noch anzurechnen ist. **Das ist `q`.**

Zeile 42 („Fiktive ausländische Quellensteuer") ist ein dritter, eigener Tatbestand (DBA-Fiktion,
vgl. Abs. 5 S. 2) und ausdrücklich *„nicht in den Zeilen 40 und/oder 41 enthalten"*.

## Offene Frage (blockiert Stufe 3 nicht, aber vor dem Bau klären)

Das XSD führt eine **zweite, strukturgleiche Serie** in Sektion `St_Abz_Betr_Ert_m_o_inl_StAbz`:
E1904702 / E1904802 / E1904902 / E1905002 / E1905102 / E1905202 — mit **identischen**
Bezeichnungen (Kapitalertragsteuer, SolZ, KiSt, Angerechnete, Anrechenbare, Fiktive).

Der Vordruck hat an der entsprechenden Stelle aber nur **drei** Felder: Block „Anzurechnende
Steuern zu Erträgen in den Zeilen 28 bis 34 sowie aus anderen Einkunftsarten", Kz 286/486,
287/487, 288/488 — KapESt, SolZ, KiSt, **ohne** ausländische Steuern.

Sechs Kz im Schema, drei im Vordruck. Kandidaten: die zweite Serie gehört zur **Anlage KAP-BET**
(liegt als `Anlage_KAP_BET_2025.pdf` vor), oder `m_o` heißt „mit **o**der ohne" und schneidet
anders als der Vordruck-Block. **Nicht geraten** — vor dem Bau von Stufe 2/3 gegen
`040_Anleitung_Anlage_KAP_2025.pdf` und `Anlage_KAP_BET_2025.pdf` prüfen.

Für Stufe 1 (Anträge) ist die Frage irrelevant.

## Reihenfolge

1. **Antragsgrund** (E1900401/E1900501) — hebt den Abgabe-Blocker auf. Ohne ihn ist jede
   Erklärung mit Kapitalerträgen uneinreichbar (`rc=610001002`, gemessen).
2. **Steuerabzugsbeträge** (Zeilen 37–39) — die große Anrechnung: einbehaltene KapESt/SolZ/KiSt
   sind heute gar nicht erfassbar.
3. **`q`** (Zeile 41 / E1905101) — die ausländische Anrechnung, kleinster Betrag, over-tax-safe.

## Quellen

- `sources/bfinv_raw/Anlage_KAP_2025.pdf` (amtlicher Vordruck 2025, Seiten 1 und 3)
- `~/02_Software/eric/.../Schema/2025/E10-2025.xsd` (1771 Kz; die Nutzdaten-XSD daneben ist ein
  7-Zeilen-Wrapper ohne Kz — beim ersten Anlauf die falsche Datei gelesen)
- `sources/gesetze-im-internet/estg_p32d_2026-07-13.txt` (Abs. 1 S. 4–5, Abs. 5, Abs. 6)
