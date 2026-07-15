# Degenerate-Anker — Design-Studie (taxgraph-dev-2, 2026-07-15)

Instructor-Auftrag: Design-Scoping (KEIN Code) für Geltungsbedingungen, deren Rechtsgrund kein
zusammenhängendes wörtliches Zitat hat und die heute mit maschinell-unprüfbaren Freitext-Keys
behelfen (= Ratsche-Loch). Read-only. Bestandsaufnahme deterministisch aus `pipeline/produktion/
rules.yaml` (260 `deckt_ab`-Einträge, 85 Regeln) gegen den `sources/`-Freeze, `_normalize`
exakt aus `gates.py:828` repliziert. Skripte: `scratchpad/degenerate_anker.py` + `_anker2.py` ($0).

## 0. STRUKTUR-BEFUND VORAB — das eigentliche Ratsche-Loch ist nicht die Anker-Qualität

Zwei getrennte Anker-Ebenen im System, nur EINE ist heute freeze-geprüft:
- **`zitatanker`** (Quellen-Ebene `quellen[].zitatanker`, `test_seed[].zitatanker`,
  `rundung.zitatanker`): HART freeze-geprüft — `_normalize(anker) in _normalize(quelltext)`,
  Abbruch bei Verstoß (`quellen.py:87`, `gates.py:910`, `gates.py:303`). Volllängen-Substring.
- **`deckt_ab`** (Bedingungs-Ebene `geltungsbedingungen[].deckt_ab`): **KEIN Freeze-Check.** Das
  einzige Gate, das `deckt_ab` liest (`geltungsbereich_gate`, `gates.py:754-767`), matcht es gegen
  die **Judge-Ausgabe** (`norm_teil`), nicht gegen die eingefrorene Quelle. `deckt_ab` trägt auch
  keinen `datei`-Zeiger — `quelle` ist ein Zitations-STRING (`"§ 9 Abs. 1 S. 3 Nr. 5 Satz 1 EStG"`),
  keine Datei.

→ Die „Volllängen-Pflicht" für `deckt_ab` ist heute **manuelle Disziplin** (mein/Instructor-
Anker-Check, [[anker-verifikation-volllaenge]]), NICHT maschinell erzwungen. Das ist das Loch:
nicht dass die Anker schlecht wären, sondern dass nichts sie beim Freeze-Drift fängt. Ein Refactoring
der Quelle, das eine Passage umformuliert, kippt kein Gate — die Bedingung zeigt still ins Leere.

## 1. BESTANDSAUFNAHME (260 deckt_ab, verbatim-Test gegen Freeze)

| Klasse | Zahl | Kriterium |
|---|---|---|
| (a) echter Volllängen-Anker | **252** | `_normalize(deckt_ab)` kommt verbatim in einer REGEL-EIGENEN Quelle vor |
| (b) degeneriert (Freitext) | **1** | nirgends verbatim (auch nicht in fremder Quelle) |
| (c) Grenzfall — Cross-Source | **7** | verbatim, aber in ANDERER frozen Datei als die Regel deklariert |
| (c) Grenzfall — ohne Freeze-Quelle | 0 | — |

**Erfreulicher Kern-Befund: 97 % (252/260) sind bereits echte contiguous Volllängen-Anker** — der
Korpus ist NICHT voller Freitext. Das eigentliche Problem ist klein-aber-real und liegt im fehlenden
Gate (0.) plus 8 harten + ~17 schwachen Fällen.

### Qualitäts-Sub-Befunde der 252 „echten" Anker
- **17 schwache Anker** (<25 Zeichen normalisiert): matchen verbatim, beweisen aber wenig —
  `"auf Antrag"` (10), `"40 700 Euro"` (11), `"beträgt 25 Prozent"` (19), `"der Übergangsgewinn"` (20).
  Ein 10-Zeichen-Anker pinnt keinen spezifischen Rechtsgrund; er kann bei Quell-Umbau versehentlich
  weitermatchen (falsch-grün).
- **3 Bedingungen nutzen `deckt_ab` bereits als Liste** (Multi-Fragment) — das Schema erlaubt es,
  aber ohne per-Fragment-`datei` und ohne Gate.
- Längen-Verteilung: median 60, p10 27, p90 107, max 197 Zeichen — die Mehrheit sind solide Sätze.

### Die 8 harten Fälle (würden ein naives deckt_ab-Freeze-Gate FALSE-FAILen)
| Fall | rule_id | Bedingung | Grund kein Match in Regel-Quelle |
|---|---|---|---|
| Cross | p4_3_gewinn | keine_lebensfuehrungskosten_p12_nr1 | Rechtsgrund in **§ 12** (nicht §4-Datei) |
| Cross | p4_3_gewinn | keine_repraesentation_gemischt_p12_nr1s2 | § 12 Nr. 1 S. 2 |
| Cross | p4_3_gewinn | keine_personensteuern_p12_nr3 | § 12 Nr. 3 |
| Cross | p4_3_gewinn | einkunftsart_gewerbe_p15_abs2 | § 15 Abs. 2 |
| Cross | p4_3_gewinn | einkunftsart_selbstaendige_arbeit_p18_abs1 | § 18 Abs. 1 |
| Cross | p34c_1_anrechnung_hoechstbetrag | auslaendische_einkuenfte_p34d_katalog | § 34d-Katalog |
| Cross | p4_1_bv_vergleich | massgeblichkeit_handelsbilanz_gob | § 5-GoB (Maßgeblichkeit) |
| Freitext | p35_1_gewst_anrechnung | messbetrag_gewst_sachverhalt | `"Gewerbesteuermessbetrags"` — Quelle hat nur `"Steuermessbetrags"` (zusammengesetztes/flektiertes Wort, nicht contiguous) |

## 2. TYPOLOGIE der degenerierten/schwachen Fälle

- **T1 Cross-Source / mehrquellig** (7): Rechtsgrund liegt in einer REFERENZIERTEN Norm, nicht der
  Signatur-Norm der Regel. Anker IST verbatim vorhanden — nur in der falschen (undeklarierten) Datei.
- **T2 Paraphrase / Flexion / verstreut** (1 belegt): der Autor schrieb ein zusammengesetztes oder
  gebeugtes Wort, das so nicht contiguous im Text steht (`Gewerbesteuermessbetrags`). Konzept da,
  Token nicht.
- **T3 Tabellen-Konstante**: Anker ist ein nackter Zahlwert (`"40 700 Euro"`, solzg-Splitting-
  Freigrenze). Kurz, nicht diskriminierend, driftet mit den params.
- **T4 Kurz/generisch — schwacher Beweis** (17): `"auf Antrag"`, `"beträgt 25 Prozent"`. Verbatim,
  aber pinnt die konkrete Bedingung nicht eindeutig.
- **T5 Implizit / Umkehrschluss** (latent, vom Instructor benannt): eine Bedingung, deren Grund die
  ABWESENHEIT einer Ausnahme oder eine systematische Herleitung ist — es EXISTIERT kein positives
  Zitat. Vom Verbatim-Scan nicht positiv auffindbar; solche Fälle borgen sich heute einen schwachen
  Proxy-Anker (versteckt in den 252/17).
- **T6 Änderungsbefehl-Altfassung** (latent, aus BLP-Dossier): Grund ist eine ABGELÖSTE Fassung, die
  im aktuellen Freeze nicht mehr steht (BLP-60k-Kohorte: der 60.000-Wert ist aus dem aktuellen § 6
  raus, nur die 70k/100k-Änderungstexte überleben). Positiv-Zitat unmöglich, weil bewusst weg.

## 3. DESIGN-VORSCHLÄGE (maschinell prüfbar gegen Freeze, je Aufwand)

**D0 — deckt_ab-Freeze-Gate (Fundament, löst das Loch aus 0.).** Neues deterministisches Gate:
je Bedingung `_normalize(deckt_ab) in _normalize(quelle)`, Abbruch bei Verstoß — exakt die
`quellen.py`-Mechanik, nur auf die Bedingungs-Ebene gezogen. Braucht D1 (datei-Zeiger), weil `quelle`
heute nur ein Zitationsstring ist. **Aufwand: klein** (~25 LOC, `_normalize` wiederverwendet).
HÖCHSTER Wert — schließt die Ratsche für alle 252 auf einen Schlag (sie passen bereits), die 8 werden
sichtbare FAILs statt stiller Löcher.

**D1 — Multi-Fragment-Anker mit per-Fragment-datei (löst T1 + T2).** `deckt_ab` wird (optional) Liste
von `{datei, fragment}`; Gate verlangt, dass JEDES Fragment in seiner benannten frozen Datei matcht.
Rückwärtskompatibel: ein blanker String = 1 Fragment, `datei` default = `norm_source`/erste `quellen[].datei`.
Löst T1 (Fragment zeigt auf §34d-Datei) und T2 (verstreute Tokens als Liste, alle müssen matchen).
**Aufwand: klein** (Schema + Gate-Schleife ~30 LOC; Liste ist im Schema schon erlaubt).

**D2 — Abwesenheits-Anker (löst T5 + T6).** `deckt_ab_absent: {datei, fragment}`: Gate assertiert,
dass der Text im aktuellen Freeze NICHT vorkommt (`_normalize(frag) not in _normalize(quelle)`) —
beweist einen Umkehrschluss („keine Ausnahmeklausel vorhanden") oder eine Altfassungs-Grenze („der
alte 60k-Wert ist raus"). **Aufwand: klein.** RISIKO: beweist Abwesenheit, nicht den positiven Grund
— MUSS mit `begruendung` + einem positiven Anker auf den überlebenden Änderungstext gepaart werden
(sonst beweist er nur „Text fehlt", was auch bei Tippfehler wahr ist). Disziplin-, kein Technik-Problem.

**D3 — Berechneter/Herleitungs-Anker (löst T3, mehrquellige Konstanten-Kette).** Statt den nackten
Zahlwert zu zitieren, den DERIVATIONS-Grund ankern: `deckt_ab_params: {sha, pfad}` = sha über den
params-Abschnitt, der die Konstante materialisiert (z. B. solzg-Freigrenze aus `params/<vz>/`). Gate
rechnet den sha über die frozen params neu und vergleicht — bindet die Konstante an die (bereits
freeze-geprüfte) params-Schicht. **Aufwand: mittel** (kanonische Serialisierung des params-Abschnitts
+ sha). BILLIGERE Alternative: T3 in D1 einfalten — den Konstanten-Anker um den umgebenden Satz
erweitern (`"40 700 Euro"` → `"…40 700 Euro bei Zusammenveranlagung"`). Empfehlung: erst einfalten,
D3 nur wenn eine Konstante wirklich nirgends als Satz zitierbar ist.

**D4 — Mindest-Diskriminanz-Lint (löst T4).** Kein hartes Gate, sondern Lint (Stufe-1-INFO wie der
Präzisions-Lint-Präzedenz, `gates.py:396`): flaggt `deckt_ab` < N Zeichen ODER Mehrfach-Match in der
Quelle als „schwacher Anker → Review", erzwingt entweder längeres contiguous Zitat oder expliziten
`schwach_ok: true`-Waiver mit Begründung. **Aufwand: winzig** (Längen- + count-Check, ~10 LOC).

## 4. MIGRATIONS-KOSTEN

| Bewegung | Zahl | Aufwand |
|---|---|---|
| **Unverändert kompatibel** (a-Anker, String = 1-Fragment-Liste, datei-default = norm_source) | 252 | 0 — passen sofort unter D0+D1 |
| **T1 Cross-Source** → `datei`-Zeiger ergänzen (oder D1-Liste) | 7 | klein, mechanisch |
| **T2 Paraphrase** → D1-Fragment fixen oder contiguous-Token nachziehen | 1 | trivial |
| **T4 schwach** → D4-Lint-Review (verlängern oder Waiver) | ~17 | niedrige Prio, INFO |
| **T5/T6 latent** → D2, erst wenn solche Bedingung entsteht (BLP-Kohorte naheliegend) | 0 heute | pro Fall klein |

**Netto: ~8 harte Migrationen + ~17 Lint-Reviews von 260.** Die 252-Mehrheit ist schon konform. Voll
rückwärtskompatibel, wenn `deckt_ab` String-oder-Liste bleibt und `datei` auf `norm_source` defaultet.

**Der große Gewinn ist D0:** heute sind NULL deckt_ab maschinell gegen den Freeze geprüft. Das Gate
einzuführen schließt die Ratsche für alle 252 auf einen Schlag (bestehen bereits) und macht die 8
harten Fälle zu sichtbaren FAILs, die D1/D2 sauber lösen.

## Empfehlung (für Julius-Entscheid)
1. **D0 + D1 + D4 zuerst** (alle klein/winzig, schließen 260/260 mit 8 harten + 17 Lint-Touches).
2. **D2** wenn die erste Umkehrschluss-/Altfassungs-Bedingung landet (BLP-Kohorte aus meinem Dossier
   ist der nächste Kandidat — [[versprochene-bedingung-materialisieren]]).
3. **D3 zurückstellen** — T3-Konstanten vorerst in D1 einfalten (umgebenden Satz ankern).

Kein Eingriff in rules.yaml/pipeline. Reines Scoping. Zonen-Zuweisung (gates.py/quellen.py = TABU,
dev-1) durch Instructor nötig, falls gebaut wird.
