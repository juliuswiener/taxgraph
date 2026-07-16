# DEV-2 Gate-Paket (msg 2085) — Sammel-Report

taxgraph-dev-2, 2026-07-16. Instructor-Order 2085: drei deterministische, LLM-freie
Gate-Bausteine, additiv im Worktree `feat/multivz`, Merge über Instructor. Basis
d4d045b (Fast-Forward, keine Divergenz). Je Teilaufgabe eigener Commit + Gate-Beweis
(pytest/make-Exit-Code), keine Selbst-Verifikation als Ersatz. Zonen: nur additiv
(golden/cases, tests/, elster/, Makefile-Target); kein Eingriff in rules.yaml /
pipeline / gates.py / quellen.py / sources (TABU, dev-1).

Commits: **771a601** (T1) · **ce5e897** (T2) · **ceeb059** (T3).
Final-Sweep: `make unit` 174 passed · `python golden/runner.py` 78/78 · `make eric-gate`
GRÜN — alle exit 0. Worktree sauber.

---

## T1 — GewSt § 8 Nr. 1 Buchst. d E-Kfz-Negativtest (Commit 771a601)

Additiver golden-Fall `golden/cases/gewst_ekfz_beweglich_halbe_nachtrag.yaml` (10.
GewSt-Case, Schema wie die 9 bestehenden). Grenzt die E-Kfz-Quote gegen die Regel-
1/5-Quote ab.

**Wortlaut-Klärung (Gültigkeits-Direktive):** § 8 Nr. 1 d **Satz 1** = „einem Fünftel
der Miet- und Pachtzinsen … beweglicher WG" (1/5). **Satz 2** = „Eine Hinzurechnung
nach Satz 1 ist nur **zur Hälfte** vorzunehmen" bei aa) Elektrofahrzeugen bb) Hybrid
(CO₂ ≤ 50 g/km ODER Reichweite ≥ 80 km) cc) Fahrrädern. ⇒ E-Kfz-Quote = **1/10**
(Halbierung der 1/5), NICHT ein Drittel. **„Drittel" kommt in § 8 Nr. 1 d nicht vor**
— die Instructor-Formulierung „Drittel-/Halbierungs-Quote" verwechselt vermutlich mit
der § 6 Abs. 1 Nr. 4-BLP-Privatnutzung (Viertel 0,25 % / Halb 0,5 %); das ist eine
andere Norm (ESt-Privatnutzung), nicht die GewSt-Hinzurechnung. Hier maßgeblich:
**Halbierung**.

**Stufe-A-Zuschnitt bestätigt:** die produktive Regel `p8_1_hinzurechnung` bildet die
E-Kfz-Halbierung NICHT ab — ihr eigener `hinweis` sagt wörtlich: „E-/Hybrid-/Fahrrad-
Halbierung von Buchst. d (1/10 statt 1/5) + KSA-Ausnahme (§ 25 KSVG) = benannter
Nachtrag, hier Standardfall." Der Runner (`_gewst_hinzurechnung_p8`) rechnet flach
d = /5. **An der Regel wurde nichts geändert** (Instructor-Auflage).

**Hand-Kette (Cent), 1/5-Standardpfad = gepinnte Erwartung, EZ 2026:**
- gewst_miet_beweglich 3.000.000 (E-Kfz-Leasing) → Summe d = 3.000.000/5 = 600.000
- Hinzurechnung = (600.000 − 200.000)/4 = **100.000**
- Gewerbeertrag = Gewinn 500.000 + 100.000 = 600.000
- § 11: (600.000 − 24.500) × 3,5 % = 20.142,50 = **2.014.250 ct** ← `erwartung.gewst_cent`

**Rechtlich-richtiger (zurückgestellter) Halbierungspfad, NUR dokumentiert:**
d = 3.000.000/10 = 300.000 → (300.000−200.000)/4 = 25.000 → GewErtrag 525.000 →
(525.000−24.500)×3,5 % = 17.517,50 = 1.751.750 ct. **Δ Messbetrag 2.625,00 €.**

**Tripwire-Charakter:** sobald die E-Kfz-Halbierung modelliert wird, liefert der Runner
1.751.750 ≠ gepinnte 2.014.250 → Fall ROT, erzwingt Erwartungs-Anpassung. Bis dahin
dokumentiert der Fall den Cut.

**Micro-Drift 60/80 km (nur vermerkt, nicht ausgerechnet):** Hybrid-Eignung Satz 2 bb
= Reichweite ≥ 80 km; § 36 Abs. 4 S. 2: „bei Verträgen, die vor dem 1. Januar 2025
abgeschlossen werden, statt … 80 Kilometern eine Reichweite von 60 Kilometern
ausreichend." **Vertragsabschluss-Kohorte, KEINE EZ-Schwelle** (analog BLP-Dossier).

**Gate-Beweis:** `python golden/runner.py` → 78/78, exit 0; Fall-Zeile
`OK gewst_ekfz_beweglich_halbe_nachtrag (est=2014250)`; Zitatanker verbatim im Freeze
(gewstg_p8_2026-07-16.txt). Tripwire-Nachweis: Erwartung testweise auf 1.751.750 →
FAIL 77/78. (Runner via Main-`_catala`-Symlink, da im Worktree kein Catala-Assembly;
d4d045b..50cb3f8 fasst golden/oracle/rules nicht an → Build-Artefakt bit-identisch.)

---

## T2 — Anker-Gate-Tail: golden/cases-Loader ins Freeze-Gate (Commit ce5e897)

Neues standalone-pytest `tests/test_golden_anker_freeze.py`, Catala-frei, läuft in
`make unit` (< 1 s).

**Lücke:** der Zitatanker-Freeze-Check der golden-Fälle lag NUR in `golden/runner.py`
Schritt 1 — und der braucht das schwere Catala-Assembly (`from pkg import …` beim
Modul-Import). Ein Quell-Umbau, der einen golden-Anker bricht, wurde vom billigen
Dauergate `make unit` NICHT gefangen; er fiel erst im vollen `make golden` auf.
Gleiche Ratsche-Familie wie D0 (deckt_ab).

**Loader:** zieht `quelle.zitatanker` + `quelle.datei` jedes golden-Falls in dieselbe
`_normalize`-Freeze-Prüfung wie `tests/test_deckt_ab_freeze.py`. `gates._normalize` ist
**zeichengleich** mit `golden/runner.py:normalize` (beide `_UMLAUT`-Transliteration +
lower + Whitespace-Kollaps) → Verdikt deckt sich exakt mit Runner-Schritt-1, keine
Divergenz. Loader-Funktion lokal im Testmodul (kein Eingriff in TABU-Zonen).

**Gate-Beweis:** `make unit` 174 passed (170 + 4), exit 0. Tamper: echter Anker in
`gewst_basis_ez2026.yaml` auf Platte verbogen → `test_alle_golden_anker_im_freeze`
FAILED mit exakter Verletzung auf gewstg_p11-Freeze; nach Restore grün. Zwei
In-Test-Negativtests (synthetisch + realer Anker + Fremdtext) ebenfalls rot.

**⚠ Doku-Abweichung geflaggt (Instructor-Auflage „melden statt improvisieren"):**
Der Design-Report `2026-07-15-degenerate-anker-design.md` nennt diesen golden/cases-
Tail **nicht** — dort sind D2/D3 zurückgestellt, kein golden-Loader dokumentiert. Der
memory-Vermerk „Tail: golden/cases-Loader" (anker-gate-paket.md) bezog sich auf den
**bereits erledigten Runner-Accessor-Loader b00bf1f** (`_kindergeld`/`_vorsorge_hb` +
Kinder-Goldens), NICHT auf ein Anker-Gate. Ich habe den Tail nach dem Instructor-
Einzeiler aus msg 2085 gebaut („der Loader, der golden/cases in das deckt_ab-Gate
einbezieht") — dieser Satz spezifiziert ihn eindeutig, unabhängig vom Report. Der
gebaute Loader ist die konsistente, hochwertige, additive Umsetzung dieser Beschreibung.
**Bitte gegenprüfen, ob das der gemeinte Tail ist.**

---

## T3 — ERiC-Offline-CI-Gate `make eric-gate` (Commit ceeb059)

Neues Target `eric-gate` + Orchestrator `elster/eric_gate.py`. VZ 2025, zweistufig,
rein lokal (kein Netz, KEIN Versand, KEINE Datei-Credentials).

- **Stufe A (credential-frei, gate-tragend):** minimales ESt-2025-XML gegen amtliches
  ELSTER-Schema `elster11_E10_2025_extern.xsd` (ERiC-Auslieferung) via xmllint. Tamper-
  Selbstcheck: schema-fremd mutiert MUSS fallen → Gate ist RED-fähig, kein Vakuum-Grün.
- **Stufe B (checkESt):** `EricBearbeiteVorgang(ESt_2025, ERIC_VALIDIERE)` — OHNE
  ERIC_SENDE, lokal im Plugin-.so. rc-Klassifikation: `rc==0` PLAUSIBEL (nur mit
  registrierter Hersteller-ID) → grün; `rc==610301202` GESPERRT → Hersteller-ID-Grenze
  → erwartete credential-freie Grenze, kein Gate-Fehler; sonst UNERWARTET → RED.

**⚠ Grenze/Befund (Instructor-Meldung, „melden statt improvisieren"):** Das reine
„checkESt-rc als Gate-Exit" (Order-Wortlaut) lässt sich **NICHT credential-frei**
bauen. Die Inhalts-Plausibilität ist ohne registrierte **Hersteller-ID unerreichbar**
— das ID-Gate feuert VOR der Inhaltsprüfung (empirisch belegt: valides + implausibles
XML liefern beide `rc=610301202`; VZ2025-XML trägt die seit ERiC 39.4.x gesperrte
Test-ID 74931). Das ist **Julius-Territorium**, bereits im Smoke-Befund 2026-07-12
dokumentiert (analog Versand-Zertifikat). Konsequenz: die credential-freie XSD-Struktur
(offiziell, deterministisch, RED-fähig) trägt den Gate-Exit; checkESt wird korrekt via
ERIC_VALIDIERE aufgerufen und **vervollständigt sich zum vollen rc==0/rc!=0-Differenzgate
automatisch, sobald `$ELSTER_HERSTELLER_ID` gesetzt ist — ohne Code-Änderung**. Kein
gefälschtes checkESt-Grün (falsches-grün-Doktrin).

Hersteller-ID NUR aus `$ELSTER_HERSTELLER_ID` falls exportiert — der Gate liest
**niemals** `.env.elster`. Ohne die Variable läuft er voll durch (Login-freier CI).
ERiC-Pfad aus `$ERIC_DIR` (Default ~/02_Software/eric).

**Weitere dokumentierte Grenze:** ein golden-Fall-abgeleitetes ESt-XML (statt des
bestehenden minimalen Testfalls) braucht das amtliche Feldmapping (Regel-Output →
ELSTER-Kz), das weiter Stub ist (pending ELSTER-Schema-Zugang, Julius). Nicht geraten;
Gate nutzt das vorhandene valide `testfall_est2025_minimal.xml`.

**Gate-Beweis:** `make eric-gate` → GRÜN exit 0 (Stufe A PASS + Tamper-FAIL, Stufe B
GESPERRT-Grenze korrekt klassifiziert). Valides XML schema-fremd manipuliert → `make`
exit 2 (ROT); nach Restore exit 0.

---

## Offene Rückfragen an Instructor
1. **T2:** Ist der gebaute golden/cases-Anker-Freeze-Loader der gemeinte „zurückgestellte
   Tail"? (Design-Report dokumentiert ihn nicht; memory-„golden/cases-Loader" = b00bf1f,
   schon erledigt.)
2. **T1:** „Drittel"-Quote — bestätigt als Verwechslung mit § 6-BLP? § 8 Nr. 1 d kennt
   nur die **Halbierung** (1/10).
3. **T3:** Voller checkESt-rc==0-Differenzbeweis bleibt an `$ELSTER_HERSTELLER_ID`
   (Julius) gebunden — soll ich, sobald die ID vorliegt, den Vollbeweis nachziehen?
