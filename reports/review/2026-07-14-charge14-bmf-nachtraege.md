# Charge 14 — BMF-Nachträge (Zuschnitt, Stufe A, 2026-07-14)

Schließt zwei **benannte, wortlaut-offene** Nachträge des Vollabdeckungs-Programms mit
eingefrorener BMF-Verwaltungsauffassung (neue Quellen-Klasse `verwaltung`). Kein
Nenner-Zuwachs — beide Fragen hingen bereits als „wortlaut-offen, BMF" an
Bestandsregeln; Charge 14 beantwortet sie BMF-gedeckt.

Quellen (Freeze 2026-07-14, committet 1b1ab9c, verify_sources 71/71, Anker grep-verifiziert):
- `sources/bmf/bmf_riester_foerderung_2023-10-05.txt` (BStBl I 2023, 1726)
- `sources/bmf/bmf_35c_einzelfragen_2025-08-21.txt` (Neufassung, ersetzt 14.01.2021)

**Leitprinzip (Instructor 2026-07-14, jetzt im Tooling verankert):** `verwaltung` ist
nachrangig zu Gesetz — der Prompt-Hinweis sagt seit dieser Charge wörtlich
„konkretisiert den Gesetzestext, **geht dem Gesetzeswortlaut aber NICHT vor**". BMF
deckt nur Fragen, die das Gesetz **offen** lässt, nie gegen den Wortlaut.

## Tooling-Vorarbeit (erledigt, LLM-frei)

1. **verwaltung-Prompt gehärtet** (`pipeline/quellen.py`, `_TYP_HINWEIS["verwaltung"]`):
   Nachrang-Satz ergänzt. **Byte-Identitäts-Beweis** (Auflage): build_norm_text über
   alle 54 Manifest-Regeln vor/nach dem Change — **53 byte-identisch**, nur die einzige
   verwaltung-nutzende Regel `p9_1_3_nr6_7_afa_laufend_nb` ändert sich (erwartet, Zweck
   der Härtung; betrifft nur deren nächsten Lauf, kein Bestand). Permanent gepinnt in
   `tests/test_quellen.py` (gesetz-Hinweis byte-genau, verwaltung-Nachrang, Per-Quelle-
   Injektion). pytest 140→144.
2. Quellen-Klasse `verwaltung` war tooling-seitig bereits verdrahtet (`_RANG` rank 2,
   `verify_sources` generisch über `sources/**`), kein weiterer Umbau nötig.

## 1 — Riester-Hinzurechnung: voll vs. bereinigt (§ 10a Abs. 2)

**Frage (bisher „wortlaut-offen, BMF" an `p10a_guenstigerpruefung`):** Erfolgt die
Hinzurechnung des Zulageanspruchs zur tariflichen ESt mit der **vollen** Zulage (inkl.
200 € Berufseinsteiger-Erhöhung § 84 S. 2) oder der **bereinigten** (ohne 200 €)?

**BMF-Antwort (Rn. 106, Zitatanker `Erfolgt aufgrund der Günstigerprüfung ein
Sonderausgabenabzug`):** „… erhöht sich die … tarifliche Einkommensteuer um den Anspruch
auf Zulage … **Der Erhöhungsbetrag nach § 84 Satz 2 EStG bleibt bei der Ermittlung der …
Zulage außer Betracht.**" (Rn. 103 deckt die Günstigerprüfungs-Seite.)

→ **BEREINIGT.** Bestätigt das existierende Design: `p10a_guenstigerpruefung` nimmt
`zulageanspruch_guenstigerpruefung` bereits als bereinigten Input (Regel-hinweis dort:
„Zulage OHNE § 84-S2-Erhöhung … kommt bereinigt als Input"). Auch die Hinzurechnung in
der § 2-Integration (Abs. 6) nutzt denselben bereinigten Wert.

**Disposition:** KEINE neue Regel, keine Signaturänderung, KEIN LLM-Lauf. Der Nachtrag
wird von „wortlaut-offen" zu **BMF-gedeckter Integrations-Festlegung**:
- `p10a_guenstigerpruefung`: hinweis-Nachtragssatz „ob Hinzurechnung voll/bereinigt —
  nicht in dieser Regel entschieden" → „**BMF Rn. 106: bereinigt (ohne § 84-S2-200 €)**".
- Neue Geltungsbedingung an der § 2-Integrations-Hinzurechnung (Abs. 6 § 10a-Zulage):
  `zulage_hinzurechnung_bereinigt_p84s2` mit deckt_ab-Anker BMF Rn. 106.
- Landkarte: „Riester-Hinzurechnung voll-vs-bereinigt (BMF)" → **erledigt (verwaltung)**.

## 2 — § 35c Energieberater: VZ-Zuordnung + Deckel-Interaktion (§ 35c Abs. 1 S. 4)

**Frage (bisher „wortlaut-offen, BMF" an `p35c_energieberater_ermaessigung`, hinweis a+b):**
(a) In welchem Förderjahr wirken die 50 % Energieberater-Kosten? (b) Zählen sie gegen die
Jahreshöchstbeträge (14 000/12 000) und den 40 000-€-Objektdeckel?

**BMF-Antwort (Rn. 56):**
- (a) VZ: „… **im Jahr des Abschlusses der energetischen Maßnahme** (siehe Rn. 80) zu
  berücksichtigen … und **nicht auf drei Jahre zu verteilen**." → **Abschlussjahr**, keine
  Drittelung (anders als die energetische Maßnahme selbst, die 7/7/6 % über drei Jahre läuft).
- (b) Deckel: „Die Kosten für den Energieberater … sind jeweils **– wie die Aufwendungen
  für die energetische Maßnahme selbst – vom (Gesamt-)Höchstbetrag und den
  Jahreshöchstbeträgen der Steuerermäßigung umfasst**." → **JA**, die Energieberater-50 %
  zählen gegen den 14 000/12 000-Jahresdeckel UND den 40 000-Objektdeckel, **gemeinsam**
  mit der energetischen-Maßnahme-Ermäßigung.

### Befund am Bestand (Handlungsbedarf)

Aktuell rechnen die beiden Teilregeln **unabhängig**:
- `p35c_sanierung_ermaessigung`: `min((7%|6%)·aufw, 14000|12000)` — deckelt sich **intern**.
- `p35c_energieberater_ermaessigung`: `(50%)·aufw` — **ungedeckelt**.

Damit kann die **Summe** beider (energetische Maßnahme + Energieberater) im selben Jahr den
14 000/12 000-Jahresdeckel überschreiten — genau das verbietet BMF Rn. 56. Der 40 000-
Objektdeckel liegt bei beiden nur als *Vorbedingung* (`objekt_hoechstbetrag_40k_nicht_
ausgeschoepft`), nicht als gemeinsame Rechnung.

### Zuschnitts-Vorschlag (mein Favorit = Instructor-Favorit: § 2-Integration)

**Der Jahresdeckel ist regelübergreifend** — § 35c Abs. 1 gewährt EINE Ermäßigung je
Objekt/Jahr, gespeist aus energetischer Maßnahme (7/6 %) UND Energieberater (50 %),
gedeckelt auf 14 000/12 000. Ein Deckel INNERHALB einer der beiden Teilregeln wäre falsch
(keine sieht den Zug der anderen). Er gehört an die **kombinierende Ebene**:

- **Neuer § 35c-Kombinations-Schritt in der § 2-Integration (Abs. 6, analog § 35a-Andock-
  Reihenfolge):** `p35c_gesamt = min(sanierung_ermaessigung + energieberater_ermaessigung,
  jahreshoechstbetrag)` mit `jahreshoechstbetrag = 14000` (bzw. 12000 im übernächsten
  Förderjahr). Die Teilregeln bleiben **rein** (liefern ihren Roh-/Eigen-Deckel-Betrag);
  die interne 14k-Deckelung von `p35c_sanierung` bleibt harmlos (der kombinierte
  min subsumiert sie: `min(min(san,14k)+eb, 14k) = min(san+eb, 14k)` für alle Fälle).
  → **Andockung wie § 31/§ 35a: kombinieren an der Integration, kein Teilregel-Umbau,
  kein LLM-Lauf** (Catala-Increment im Tarifmodul + clerk-Seeds).
- **VZ-Abschlussjahr (a):** ist § 2-Integrations-Zuordnung (welche Jahres-`energieberater_
  aufwendungen` in welchen VZ) — dieselbe Jahreswahl-Mechanik wie die energetische
  Maßnahme. Dokumentiert, BMF Rn. 56-verankert.
- **40 000-Objektdeckel:** bleibt **Mehrjahres-State** (§ 2/VZ-State, analog § 10d Abs. 4) —
  spannt alle Maßnahmen × alle Jahre. Bleibt benannter Nachtrag, aber jetzt **BMF-gedeckt**
  (Rn. 56 bestätigt: Energieberater zählt gegen 40k), nicht mehr wortlaut-offen.

**Alternative (verworfen):** Rest-Jahresdeckel als Input in `p35c_energieberater`
(`min(50%·aufw, rest_deckel)`). Nachteil: verlagert regelübergreifende Logik in eine
Teilregel, die den Sanierungs-Zug nur als vorberechneten Input „glauben" muss —
zerschneidet die Andockungs-Doktrin (Teilregeln rein, Kombination an der Integration).

### Disposition Charge 14 (nach deiner Review)

| Teil | Änderung | LLM? |
|---|---|---|
| Antwort 1 Riester | hinweis + § 2-Bedingung BMF-Rn.106-Anker; Landkarte | nein |
| Antwort 2 (a) VZ | hinweis energieberater: „BMF Rn. 56 Abschlussjahr"; Landkarte | nein |
| Antwort 2 (b) Jahresdeckel | § 2-Integration: `p35c_gesamt`-Kombinations-min + clerk-Seeds | nein (Catala/clerk) |
| Antwort 2 (b) 40k-Objekt | Bedingung-beschreibung „BMF Rn. 56-gedeckt" (bleibt Mehrjahres-Nachtrag) | nein |

**Keine neue Pipeline-Regel, kein Stufe-B-Modelllauf.** Charge 14 ist reine
Integrations-/Doku-Arbeit + ein Catala-Increment (Jahresdeckel-Kombination), alles
clerk-/pytest-verifizierbar. Kosten Stufe A: **$0**.

## Offene Punkte für deine Review

1. Deckel-Platzierung: § 2-Integration (mein/dein Favorit) bestätigen — dann baue ich das
   Catala-Increment + Seeds.
2. Sollen die BMF-Rn-Anker als `verwaltung`-**Quelle** in die betroffenen Regeln (ändert
   deren Formalisierer-Prompt → nächster Lauf) ODER nur als Bedingungs-/Landkarten-Text
   (prompt-neutral, mein Default für „keine neue Regel")? Für Antwort 1+2a empfehle ich
   prompt-neutral; für den Jahresdeckel-Catala-Schritt ist die BMF-Quelle der Zitatanker.
