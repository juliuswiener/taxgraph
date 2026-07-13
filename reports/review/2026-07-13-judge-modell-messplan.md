# Judge-Modell-Messplan (Vorregistrierung) — Neubesetzung der Judge-Rolle

Anlass: `deepseek/deepseek-v4-pro` ist über westliche Provider chronisch schlecht bedient
(2026-07-13: deepinfra timeout + together hang, beide an Regel 1; /endpoints-Status als Go-Signal
falsifiziert). Provider-Roulette gestoppt (Eskalations-Regel). Damit Judge-MODELL-Frage. Muster:
B-Rollen-Bake-off. **Vorregistriert VOR dem Lauf** (Kandidaten, Messgrößen, Abbruch-/Wahlkriterien
fest), damit die Wahl nicht nachträglich an ein Modell angepasst wird.

## Randbedingungen (hart)
- **Dritte Modellfamilie** ≠ Anthropic (A) ≠ Z.AI/GLM (B) ≠ DeepSeek — dekorrelierte Fehler, Soundness
  des Äquivalenz-/Detektor-Netzes (models.yaml-Prinzip).
- **Westlich/EU-gehostet**, keine chinesischen Endpoints (Legal-/Steuermaterial-Ausschlussliste).
- **Provider-gepinnt**, ein Provider, allow_fallbacks=false (Doktrin unverändert).
- Output-Budget **24k** (V4-Pro-Bedarf: Reasoning + JSON-Verdikt + Annahme-Mapping; die Zahl der
  Items treibt die Länge, nicht die Prosa).

## Kandidaten (Instructor-Rahmen)
| Kandidat | Familie | Host (Kandidat) | Notiz |
|---|---|---|---|
| `mistralai/mistral-large` | Mistral (EU) | Mistral/EU | Sovereignty-Plus; JSON-Reliabilität prüfen |
| `google/gemini-2.5-pro` | Google | Google/Vertex | Trunkierungs-Historie bei 8k → MIT 24k+ messen |
| `meta-llama/llama-4-...` | Meta | westl. Hoster | billig; Detektor-Recall prüfen |
(Finale Slugs + Provider-Tags gegen /models + /endpoints verifizieren; je Kandidat der zuverlässigste
westliche Provider, echter Probe-Call — NICHT /endpoints-Status, s. Lehrstück.)

## Referenzkorpus — der Trumpf: wir HABEN Judge-Ground-Truth
Aus 20+ produktiven Regeln liegen `dekomponiert@2`-Verdikte des bisherigen Judges (deepseek-v4-pro)
vor (item_registry + report.json-Historie). Diese sind die REFERENZ. Jeder Kandidat läuft als Judge
auf demselben Regel-Set (Kontext + Signatur + auszug identisch), sein Verdikt wird gegen die
Referenz-Items verglichen. Kein neues Labeling nötig — der teure Teil ist geschenkt.

Auswahl des Korpus: die Regeln mit den GEHALTVOLLSTEN Verdikten (viele stille Zusatzannahmen /
scope_gaps), z.B. § 9 Abs. 4a (6 Bedingungen / 12 scope_gaps — der Stresstest), § 35a (12
Bedingungen), § 32b, das GWG-_nb, ein Tabellen-Fall (§ 33b). ~8-10 Regeln, disjunkt gestreut über
die Fehlerklassen.

## Messgrößen (vorregistriert)
1. **annahme_verpasst** (G2-Leitmaß): fängt der Kandidat dieselben stillen Zusatzannahmen wie die
   Referenz? (deepseek-v4-pro hatte 0.000 in G2; GLM 0.143, Sonnet 0.286.) NIEDRIGER = besser.
2. **Inventar-Recall**: deckt sein Item-Inventar die Referenz-Items (Anker-Schlüssel-Union)?
3. **JSON-Parse-Quote**: Anteil schema-valider Antworten bei 24k-Budget (Trunkierung/finish_reason).
4. **Provider-Zuverlässigkeit**: echte Call-Completion-Rate + Latenz über 3 Läufe (NICHT /endpoints).
   Ein Kandidat, der wie deepseek-v4-pro hängt/timeoutet, fällt raus — egal wie gut die Verdikte.
5. **Kosten** je Verdikt.
6. **Quant-Arm (fp4-Ausnahmen-Frage)**: falls ein Kandidat quantisiert antritt, fp4/fp8 vs unquant
   als Arm mitmessen (klärt, ob der bisherige „auch"-Vorrang-Blindfleck + Streuung Quant-Artefakte
   waren, siehe judge-provider-fp4-Notiz).

## Abbruch-/Wahlkriterien (vor dem Lauf fest)
- **Zuverlässigkeit ist Vorbedingung, nicht Trade:** ein Kandidat mit Call-Completion < 100 % über
  die 3 Zuverlässigkeits-Läufe scheidet aus, unabhängig von den Verdikt-Metriken (der ganze Anlass
  ist Unzuverlässigkeit).
- Unter den zuverlässigen: niedrigstes annahme_verpasst gewinnt; bei Gleichstand höherer
  Inventar-Recall, dann Kosten.
- **Regelfall bei Sieger:** der Bake-off-Sieger (dritte Familie) wird Judge, **A bleibt Sonnet, B
  bleibt GLM** — Dekorrelation A(Anthropic)/B(Z.AI)/Judge(dritte Familie) unveraendert intakt.
- **Kein Sieger** (alle unzuverlässig): Rückfall = **GLM-5.2 als Judge, dann muss B aus GLM WEG in
  eine dritte Familie** (Gemini war im B-Bake-off der beste Rest, mit dokumentiertem Trunkierungs-
  Caveat → mit 24k-Budget messen). Also A(Sonnet)/B(Gemini)/Judge(GLM). ACHTUNG: „GLM-als-Judge +
  Sonnet-als-B" waere FALSCH — dann sind A UND B beide Anthropic, gleiche Familie teilt Modell-
  Idiosynkrasien, equivalence verliert ihre Beweiskraft. Die dritte Familie MUSS ueber alle drei
  Rollen erhalten bleiben. Alternativ, falls B-Umbesetzung zu teuer: Judge bleibt deepseek-v4-pro
  geparkt, Queue wartet, bis ein zuverlaessiger Kandidat verfuegbar wird.

## Ablauf
1. Julius-Freigabe (Kandidaten + Budget ~$1-2).
2. Slugs/Provider verifizieren (echter Minimal-Probe-Call je Kandidat/Provider).
3. Referenzkorpus fixieren (die ~8-10 Regeln + ihre Referenz-Verdikte einfrieren).
4. Je Kandidat: Judge auf dem Korpus, Metriken 1-6 erheben.
5. Scored-Vergleich → Julius wählt → models.yaml judge-slug+provider, Hash-Bump.
6. Dann: der geparkte 9er-Judge-Nachzug läuft auf dem neuen Judge (Batch-Disziplin).

Bis zur Entscheidung: KEIN --redo-judge; die 9 (+ künftige) strukturgeprueft-Regeln warten ehrlich.
