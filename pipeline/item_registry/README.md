# Item-Registry (Protokolldekret Stufe 4)

Je Regel eine `<rule_id>.yaml`: die monoton wachsende Menge aller triagierten
Prüf-Items. Die Gates prüfen deterministisch gegen diese Dateien, nicht gegen
frische Judge-Läufe.

**Nur Julius' Triage schreibt hier hinein.** Ablauf:

```
python pipeline/item_registry.py discover <rule_id> > draft.yaml   # offene Funde
# draft.yaml editieren: je Item triage setzen
python pipeline/item_registry.py aufnehmen draft.yaml              # in die Registry
```

Triage-Status: `bedingung_neu` (abgedeckt durch eine Geltungsbedingung),
`grenzfall` (objektiv ambig → Review), `nicht_material` (generisch, global gemappt),
`nicht_echt` (kein echter Befund).

Noch leer: die Erstbefüllung erfolgt über die Absegnung der
Vervollständigungs-Pakete (`reports/review/2026-07-11-vervollstaendigung-*.md`).
