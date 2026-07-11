# Auszug-Leitlinien-Sweep — Teil-2-Rest-Regeln (Stufe-A, vor den Rest-Batches)

Leitlinie (Instructor 2026-07-12, [[formalisierer-kontext-kanal]]): der `auszug` ist der
EINZIGE Norm-Kanal zum Formalisierer (Geltungsbedingungen → nur Judge). Er muss **jede
Klausel enthalten, die den OUTPUT variiert** (Trigger, Schwellen, Beträge, Formelbestandteile);
reine Anwendbarkeits-Klauseln dürfen draußen bleiben. Vier Regeln (nr5a-Cap, nr6_7-Verteilung,
solzg-Milderung, p36-Aufrundung) belegten: enge auszüge → das Modell rät oder droppt den
fehlenden Zweig. Dieser Sweep prüft die 6 noch nicht gelaufenen Teil-2-Regeln PRÄVENTIV.

Befund: **alle 6 sind gehungert** (auszug 22–115 Zeichen, jeweils ohne mindestens eine
output-variierende Klausel). Ohne Weitung würden sie dieselben Fehler wie Batch 1 produzieren.

| Regel | auszug jetzt | fehlende OUTPUT-Klausel | Schwere |
|---|---|---|---|
| p33_1_2 | „den Umständen nach notwendig… angemessenen Betrag nicht übersteigen" [81] | die **Subtraktion der zumutbaren Belastung** (Abs. 1: „…der die zumutbare Belastung übersteigt, vom Gesamtbetrag abgezogen") | hoch |
| p32_6 | „für jedes… Kind ein Freibetrag" [72] | die **Beträge 3.414 € + 1.464 €** und die **Verdopplung** bei Zusammenveranlagung | hoch |
| p10_1_3_3a | „2 800 Euro abgezogen werden" [27] | die **Höchstbetrags-Mechanik** (Basis-KV/PV immer voll; weitere bis 2.800/1.900) | sehr hoch |
| p10_1_4 | „gezahlte Kirchensteuer" [22] | die **Erstattungs-Verrechnung** (gezahlte − erstattete, Null-Boden) | mittel |
| p31 | „erhöht sich… um den Anspruch auf Kindergeld" [115] | die **Günstiger-Struktur** (Satz 1: „entweder durch die Freibeträge… oder durch Kindergeld… bewirkt") | mittel |
| p10_1_2 | „bis zu dem Höchstbeitrag… aufgerundet…" [110] | die **AG-Anteil-Kürzung** („vermindert um den nach § 3 Nr. 62 steuerfreien Arbeitgeberanteil") | hoch |

## Proposed Widenings (verbatim aus frozen source, geprüft wo [✓])

**p33_1_2** → auszug erweitern um den Abs.-1-Kern (Subtraktions-Mechanik): den Satz, der
„…der die dem Steuerpflichtigen zumutbare Belastung (Absatz 3) übersteigt, vom Gesamtbetrag der
Einkünfte abgezogen" enthält. (zumutbare_belastung ist Input; die Regel muss subtrahieren + Null-Boden.)

**p32_6** [✓ verbatim, 455 Zeichen] → auszug = Satz 1+2:
„Bei der Veranlagung zur Einkommensteuer wird für jedes zu berücksichtigende Kind des
Steuerpflichtigen ein Freibetrag von 3 414 Euro für das sächliche Existenzminimum des Kindes
(Kinderfreibetrag) sowie ein Freibetrag von 1 464 Euro für den Betreuungs- und Erziehungs- oder
Ausbildungsbedarf des Kindes vom Einkommen abgezogen. 2 Bei Ehegatten, die nach den §§ 26, 26b
zusammen zur Einkommensteuer veranlagt werden, verdoppeln sich die Beträge nach Satz 1…"
(3.414+1.464 = 4.878 pro Kind/Elternteil; verdoppelt 9.756 — deckt die Seeds exakt.)

**p10_1_3_3a** [✓ verbatim] → auszug = „…2 800 Euro abgezogen werden. 2 Der Höchstbetrag beträgt
1 900 Euro bei Steuerpflichtigen, die … Anspruch auf … Erstattung …" PLUS der Nr-3-Satz (Basis-
KV/PV immer voll abziehbar, auch über dem Höchstbetrag). Zwei auszüge: Basis-Durchbruch + Höchstbetrag.

**p10_1_4** → auszug um die Erstattungs-Verrechnung ergänzen (gezahlte minus erstattete). Prüfen ob
§ 10 Abs. 4b (Erstattungsüberhang) den Wortlaut trägt; sonst ist die Netto-Logik Input-Semantik
(erstattete_kirchensteuer) + Null-Boden — grenzwertig, evtl. reicht die bestehende Bedingung.

**p31** → auszug um Satz 1 ergänzen: „…entweder durch die Freibeträge nach § 32 Absatz 6 oder durch
Kindergeld nach Abschnitt X bewirkt." (die Günstiger-Wahl min(est_ohne, est_mit+kindergeld)).

**p10_1_2** [Teil-verbatim] → ZWEI gezielte auszüge (der Volltext-Block hat 1.540 Zeichen
Sonderfall-Irrelevanz dazwischen — NICHT am Stück nehmen): (1) Höchstbeitrag+Verdopplung+aufrunden;
(2) „…vermindert um den nach § 3 Nummer 62 steuerfreien Arbeitgeberanteil zur gesetzlichen
Rentenversicherung…" (die AG-Kürzung nach dem Cap). Sonst rechnet das Modell min(gesamt,HB) ohne AG-Abzug.

## Empfehlung

Vor jedem Rest-Batch die auszüge der beiden Regeln gemäß obiger Liste weiten (verbatim
verifizieren wie gehabt), DANN laufen lassen. Erwartung nach Batch 1/nr5a/nr6_7: die
strukturell einfachen Regeln (p32, p33, p10_4, p31) werden dann A+B grün; die mit Cap/Basis-
Durchbruch (p10_3a, p10_2) evtl. mit Rest-Subtilität (analog solzg-Milderung) — dann per-seed
messen, ggf. Split-Fallback. Kein Rest-Batch mit den aktuellen Fragment-auszügen.

## Frage an Instructor

Widenings so freigegeben (ich finalisiere die exakten verbatim-Strings + verifiziere je Regel
beim Anwenden)? Batch-Reihenfolge § 33+§ 32 zuerst? p10_1_4-Erstattung: Wortlaut-Weitung oder
als Input-Semantik akzeptieren?
