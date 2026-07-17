# Nachtrag: VOR-Vorsorge-Mapping LStB-zeilenscharf (Paket-A-Bindungstabelle)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** benannter Nachtrag (KEINE Golden-Schema-Änderung
jetzt — Instructor-Ruling Prio 3) · untracked, Commit über Instructor

## Fund (aus A2 P9-R3, P1 VOR-Injektion)

Unser Golden-Schema abstrahiert die Altersvorsorge-Beiträge als **eine** Größe
`vorsorge_gesamtbeitraege_inkl_ag` (+ `vorsorge_ag_anteil_steuerfrei`). Die ELSTER-Deklaration (Anlage
Vorsorgeaufwand, Sektion `VOR/AVor`) verlangt dieselbe Größe **LStB-zeilenscharf aufgeteilt**:

| ELSTER-Kz | Bedeutung (kz_extract / E10-2025.html) | Herkunft |
|---|---|---|
| E2000401 | Arbeitnehmeranteil laut **Nr. 23 a/b** der Lohnsteuerbescheinigung | LStB Zeile 23 |
| E2000801 | Arbeitgeberanteil/-zuschuss laut **Nr. 22 a/b** der Lohnsteuerbescheinigung | LStB Zeile 22 |
| E2000601 | Beiträge zu gesetzlichen Rentenversicherungen — **ohne** die in E2000401 erfassten | außerhalb LStB |

Der Fuzz-Scout injizierte einen Best-effort-Split `AN = gesamt − AG` → validiert `rc=0` (plausibel),
ABER die **Herkunft je LStB-Zeile** (welcher Teilbetrag kommt aus Nr. 22 vs. Nr. 23 vs. außerhalb) ist
in unserem Golden nicht abgebildet. Für eine ECHTE Einreichung ist der Split nicht rekonstruierbar,
ohne die LStB-Zeilenwerte als Input zu führen.

## Warum jetzt NICHT lösen

Golden-Schema-Änderung würde 6 Vorsorge-Goldens + Regel-Inputs berühren und ist entkoppelbar. Der
Best-effort-Split hält den Fuzz-Scout grün; die Präzisierung ist ein sauber isolierbarer Nachtrag.

## Wohin es gehört: Paket-A-Bindungstabelle (UI-Kern-Provenance)

Das ist **nicht** bloß ein Mapping-Detail, sondern exakt die **Provenance-Frage** des UI-Kerns aus dem
Ideation-Lab (Binding-Table-Artefakt-Lücke, „Vertrauen = Vektor: Herkunft × Prüftiefe × Haftung"): jeder
deklarierte Wert muss seine **Herkunft** tragen. E2000401/E2000801/E2000601 sind ein Musterfall — ein
Sachverhalts-Betrag zerfällt in mehrere Deklarations-Kz mit je EIGENER Quellzeile (LStB Nr. 22/23 /
außerhalb).

**Konkret für die Bindungstabelle:** die Zeile Vorsorge braucht eine Spalte **Herkunft/LStB-Zeile** je
Kz, damit der Store (Sachverhalts-YAML) den Aufteilungsschlüssel provenance-echt hält, statt ihn im
Mapping zu raten. Das validiert die Lab-These „Store ist Wahrheit + Kante trägt Herkunft" an einem
realen amtlichen Fall.

## Offene Punkte (für den Vollausbau, nicht jetzt)

1. Golden-Schema um LStB-Zeilenaufteilung (Nr. 22/23) erweitern → E2000401/E2000801 herkunftsecht.
2. E2000601 (ges. RV außerhalb LStB) als eigenen Input führen (heute im Golden nicht vorhanden).
3. VOR-`AVor`-Kz in `elster/feldmapping.stub.yaml` aufnehmen (dort aktuell nur EÜR/AUS gemappt) — mit
   Herkunfts-/LStB-Zeilen-Feld pro Eintrag.

## Querverweise

- A2-Scout: `reports/review/2026-07-17-p9r3-checkest-fuzzing.md` (P1 VOR-Unschärfe).
- Lab-Synthese: Binding-Table-Artefakt-Lücke + Provenance-Vektor (UI-Kern).
