# hinweis-Kanal — Erstbefüllungs-Vorschläge (Mini-Report an Instructor)

Nach Adoption (Dekret Morgenpaket §10, Julius 2026-07-12). Auflage (iii): jede Befüllung durch
deinen Review, bevor sie in `rules.yaml` landet. Dieser Report schlägt vor — er befüllt NICHT.

## Kernbefund: AKTUELL braucht KEINE Produktions-Regel einen hinweis

- Die **5 gemessenen Hunger-Fälle** (nr5a, nr6_7_afa_laufend, p10_1_3_3a, p33_1_2, +solzg-Milderung)
  sind bereits durch **auszug-Weitung** grün. Auflage (i): kein Rückbau, kein hinweis. Sie bleiben
  wie sie sind.
- Die **verbleibenden Residuen** sind NICHT hinweis-adressierbar:
  - **solzg Klasse-5-Präzision** (20351→0,11): B1 hat empirisch gezeigt, dass das Numeric-Idiom sie
    NICHT fixt (blieb $0,12). → Code-Aufgabe (Präzisions-Lint / decimal-Refactor), kein hinweis.
  - **GWG Netto/Brutto** (afa_laufend): Klasse-3 (fehlende Input-Trennung ak_netto/ak_brutto) →
    Neuschnitt Charge 3, kein hinweis.
  - **nr6_7 Überhangsjahr** (AK − Σ laufend): Klasse-3 §2-Integrations-Arithmetik → kein hinweis.

**Vorschlag: Erstbefüllung = LEER.** Kein `hinweis:` in die aktuelle `rules.yaml`. Der Kanal steht
bereit (Schema + Verdrahtung committed), wird aber erst gezogen, wenn eine KÜNFTIGE Regel Klasse-1-
Hunger zeigt, den Weitung nicht prominent genug schließt. Das hält Auflage (i) strikt ein.

## Wann künftig zum hinweis greifen (Entscheidungsprozedur)

Reihenfolge — hinweis ist die LETZTE Stufe, nicht die erste:

1. Zeigt der `auszug` den OUTPUT-variierenden Trigger überhaupt? Wenn nein → **auszug weiten** (Quelle
   hat den Text meist schon). Das ist der Normalfall und bleibt primär.
2. Steht der Trigger im auszug, wird aber in einer langen Passage vergraben nicht gewichtet? →
   **auszug in prominenten Eigen-Block** schneiden (p10_1_3_3a-Lehre). Immer noch Weitung, kein hinweis.
3. Erst wenn 1+2 nicht reichen — die Rechen-Mechanik ist nicht als zusammenhängender Gesetzes-Wortlaut
   verfügbar (z. B. eine min/max-Verschachtelung, die über mehrere Sätze verstreut ist) — dann
   **hinweis**: ein operativer Spec-Satz, der die Mechanik benennt, OHNE Gesetzestext zu paraphrasieren.

Wenn der Fall Klasse 3 (Arithmetik ohne Wortlaut), 4 (Rundungs-Richtung) oder 5 (Präzisionsordnung)
ist → NICHT hinweis, sondern der jeweilige strukturelle Fix (§2-Integration / rundungs_lint richtung /
Präzisions-Lint). Der hinweis-Kanal ist ausschließlich für Klasse-1-Kontext-Hunger belegt.

## Validierte hinweis-Bibliothek (B1-erprobt, als Vorlage für künftige Regeln)

Diese fünf Strings haben in B1 (Arm B) den Hunger mit engem auszug gelöst — als Muster für die FORM
(operativ, min/max/Bedingung explizit, kein Normzitat), falls eine künftige Regel dieselbe Mechanik
trägt. NICHT jetzt einsetzen (die Regeln sind über Weitung grün); als Referenz für den Reviewer.

| Regel | erprobter hinweis (Form-Muster) |
|---|---|
| nr5a Übernachtung | „…monate_bisher_am_ort >= 48 → Kappung min(kosten_monat,1000)×monate; davor tatsächliche Kosten voll." |
| nr6_7 AfA laufend | „AK>800 → linear AK/nutzungsdauer; Anschaffungsjahr anteilig ×(12−(monat−1))/12; AK≤800 Sofortabzug." |
| p10_1_3_3a KV/PV | „basis_kv_pv IMMER voll; weitere nur bis Rest-Raum min(weitere,max(0,Höchstbetrag−basis)); Höchstbetrag 2800 / 1900 mit Zuschuss." |
| p33_1_2 agB | „abziehbar = max(0, außergewöhnliche_belastungen − zumutbare_belastung)." |
| solzg (Milderung) | „SolZ = min(5,5%×BMG, 11,9%×(BMG−Freigrenze)); unter Freigrenze 0." (Milderungszone; Klasse-5-Cent NICHT damit lösbar) |

Volltext der erprobten Strings: `scratchpad/b1_run.py` HINWEIS-Dict / Rohdaten-JSON. Bei Bedarf hebe
ich sie in eine committete `hinweis_bibliothek.yaml`, sobald die erste künftige Regel eine braucht.

## Was ich committe (dieser Schritt)

- Schema: `rules.yaml` §4 HINWEIS + `schema_version: 2` (Doku).
- Verdrahtung: `run.py:build_candidate` `hinweis` → `formalisierer_zusatz` (Default leer, kein Regress,
  pytest 99/99).
- Dekret: Morgenpaket §10.
- **KEINE** `hinweis:`-Werte in `rules.yaml` (Erstbefüllung leer, wie oben begründet).

## Frage an dich

Bibliothek so lassen (Referenz im Report) oder gleich als committete `hinweis_bibliothek.yaml`
anlegen? Und: siehst du eine aktuelle Regel, die ich als hinweis-bedürftig übersehe (ich sehe keine)?

## Instructor-Entscheid (2026-07-12) — umgesetzt

- **(a) Kein Template-Bump** bestätigt. ZUSATZ: report.json schreibt je Lauf den verwendeten
  `hinweis` + `hinweis_sha256` mit (`run.py:hinweis_provenance`, Test `test_hinweis_provenance.py`,
  pytest 103/103) → auditierbar, falls der hinweis in rules.yaml später geändert wird.
- **(b) Erstbefüllung leer** bestätigt (Residuen Klasse 3/5, nicht hinweis-adressierbar).
- **(c) Bibliothek committed** als `pipeline/hinweis_bibliothek.yaml` (Review-Artefakt wie
  `signatur_konventionen.yaml`), Pflicht-Kopfkommentar: B1-validierte Vorlagen, jede Verwendung
  braucht Instructor-Review, kein Copy-Paste ohne Freigabe.
