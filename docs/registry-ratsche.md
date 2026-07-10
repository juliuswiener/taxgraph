# Registry-Ratsche — Judge als Detektor, Gates deterministisch

Protokolldekret 2026-07-10, Stufe 4. Antwort auf den Befund, dass das
Judge-Inventar nicht sättigt: die Nicht-Sättigung wird vom Problem zum
Mechanismus.

## Prinzip (AINA)

1. **Der Judge ist ein Detektor.** Jeder Lauf findet Prüf-Items (Annahmen,
   Norm-Teile, Abweichungskandidaten). Er *entscheidet* nichts über die Regel.
2. **Die Registry ist die Wahrheit.** Je Regel eine
   `pipeline/item_registry/<rule_id>.yaml`: die monoton wachsende Menge aller je
   **triagierten** Items.
3. **Die Gates prüfen gegen die Registry, nicht gegen den frischen Lauf.**
   Identischer Registry-Stand + identisches Manifest → identisches Verdikt, per
   Konstruktion. `geltungsbereich`, `roundtrip`, `grenzfall` sind reine Funktionen
   von (Registry, Manifest).
4. **Nur die Triage durch Julius ändert die Registry; nur die Registry ändert
   Verdikte.** Neue Funde eines Laufs kippen NIE ein Gate — sie landen in der
   Discovery-Queue (`queue_status: discovery_triage`) und warten auf Triage.

Damit ist die Nicht-Sättigung kein Blocker mehr, sondern der Mechanismus: jeder
künftige Re-Gate-Lauf ist billige inkrementelle Discovery über die Lebenszeit der
Regel — wie der Golden-Korpus wächst auch die Item-Registry monoton.

## Triage-Status

| Status | Bedeutung | Wirkung aufs Gate |
|---|---|---|
| `bedingung_neu` | abgedeckt durch eine Geltungsbedingung (`bedingung` gesetzt) | `geltungsbereich`/`roundtrip` PASS, sofern die Bedingung im Manifest deklariert ist |
| `grenzfall` | objektiv ambig, hängt von der Signatur-Interpretation ab | `grenzfall`-Gate FAIL → fester Review |
| `nicht_material` | generische Annahme, global in Signatur-Konventionen gemappt | blockiert nie |
| `nicht_echt` | kein echter Befund | blockiert nie |

## Ablauf der Triage

```
python pipeline/item_registry.py discover <rule_id> > draft.yaml
# draft.yaml editieren: je Item triage setzen (bei bedingung_neu die bedingung-ID)
python pipeline/item_registry.py aufnehmen draft.yaml
```

`discover` listet die neuen (noch nicht registrierten) Funde des letzten Verdikts.
Per Konvention gemappte Annahmen sind mit `nicht_material` vorbelegt. `aufnehmen`
schreibt die triagierten Items monoton in die Registry (Version +1).

## Signatur-Konventionen

`pipeline/signatur_konventionen.yaml` deklariert die generischen Annahmen EINMAL
global (Beträge in Euro, VZ-Bezug, ganzzahlige Monate, „Input enthält nur das
Etikettierte", …). Der Judge bekommt sie als Mapping-Ziel; eine Annahme, die eine
Konvention ausdrückt, mappt auf `konv:<id>` statt in jede Regel-Liste. Das senkt
die latente Item-Menge, ohne den Zuschnitt zu verengen.

## Freigabe-Semantik (ehrlich)

- `verified` — keine Geltungsbedingungen nötig, alle deterministischen Gates grün,
  keine offenen Discoveries.
- `verified_bedingt` — **alle REGISTRIERTEN Items abgedeckt** (jede Bedingung im
  Manifest, jede Konvention gemappt, keine offenen Grenzfälle) **und** die
  deterministischen Gates grün.
- `discovery_triage` — neue Funde warten auf Julius' Triage. Die Regel ist nicht
  fertig, aber kein Gate ist gekippt.
- `flagged_for_review` — ein deterministisches Gate (equivalence, clerk,
  geltungsbereich, roundtrip, grenzfall) ist rot.

**Restrisiko, wörtlich (Dekret Punkt 5):** Nach der Vervollständigung der
Bedingungslisten ist die verbleibende Aufgabe des Round-Trip-Gates die Erkennung
undeklarierter Annahmen, und dort bleibt Recall-Rauschen ein False-PASS-Risiko —
eine Annahme, die alle Inventarläufe verpassen. Gegenlager: Union-until-Saturation,
Golden-Tests, Human-Review. Das Gate garantiert keine Vollständigkeit.

## Was die Messung (Punkt 6) noch prüft

Bei fixem Registry-Stand gilt die Gate-Replikation per Konstruktion — der
Bestätigungslauf findet Bugs, kein Rauschen. Die neue Konvergenzgröße ist der
**Discovery-Yield**: neue materielle Items je Kampagne und Regel. Er darf langsam
auslaufen, muss nicht null werden. Die Judge-Splitrate ist nur noch informativ
(Rauschen der Detektorstufe, kippt nichts). Der Zweit-Judge lohnt sich, wenn er den
Discovery-Yield materiell erhöht — nicht um Verdikte zu stabilisieren; Entscheid
erst nach zwei, drei Kampagnen Yield-Daten.
