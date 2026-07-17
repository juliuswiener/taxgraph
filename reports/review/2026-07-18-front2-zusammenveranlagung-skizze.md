# Front 2 — Splitting/Zusammenveranlagung: Deklarations-/Bindungs-Skizze (Task #11)

**Status:** Konzept-Skizze zur Instructor-Abnahme, concept-first, KEIN Code. Reine Deklarations-/
Bindungs-Seite (Ring-slot_fn zusammen = dev-1-Haut). LLM-frei. Mit KERN-FRAGE (Store-Personen-Modell)
+ einem widerlegten Vorbefund.

## Zwei Recon-Befunde, die den Zuschnitt bestimmen

1. **Die zusammen-Engine nimmt JOINT-Aggregate, NICHT 2× Person.** `festzusetzende_est_gesamt_zusammen`
   (runner.py) bekommt `einkuenfte_nichtselbststaendig` (kombiniert), `einkuenfte_kapitalvermoegen` …
   als bereits GEMEINSAME Summen + wendet den Splittingtarif an. Der **Person-Split matert nur für § 9a**
   (jeder Ehegatte hat seinen EIGENEN Arbeitnehmer-Pauschbetrag 1.230 €): joint einkuenfte_ns =
   (bruttolohn_A − max(WK_A, 1230)) + (bruttolohn_B − max(WK_B, 1230)). Diese Per-Person-Rechnung + Summe
   ist der **Ring/slot_fn (dev-1)**; die Engine kriegt nur die Summe. **Kein neuer Engine-Input meinerseits.**

2. **WIDERLEGT: es gibt KEINE distinkten A/B-Bruttolohn-Kz.** E0220201 / E2200401 existieren NICHT im
   E10. Person-A und Person-B tragen ihren Bruttolohn beide in **E0200201** — aber in ZWEI SEPARATEN
   Anlage-N-Instanzen (wie die Kind-Anlagen). Distinkte A/B-Kz gibt es NUR im Mantelbogen für die
   Identität: **E0100081 (Person A) / E0100082 (Person B)** (Identifikationsnummer). Also: Person-A/B-
   Einkommen = **Instanz-Multiplikation**, nicht Kz-Paare.

## KERN-FRAGE (nicht still gelöst): Store-Personen-Modell

Zusammenveranlagung = ZWEI Anlage-N-Instanzen; die Einkommensfelder existieren je Person. Zwei Wege:

| | Option A — flache `_partner`-Zwillinge | Option B — Store-Personen-Dimension |
|---|---|---|
| Modell | `bruttoarbeitslohn` (A) + `bruttoarbeitslohn_partner` (B), `vor_*_partner` … | `feld_id` + `person` (A/B) im Store |
| Store-Kern | **unverändert** (nur neue feld_ids) | append_event/materialisiere/snapshot bekommen eine Person-Achse (Umbau) |
| est_mapping | neue Klasse „Person-Multiplikation" (Instanz B), analog Kind (Klasse e) | Person-Achse im Mapping |
| Kosten | leicht; nur ~4-5 Einkommensfelder doppeln (bruttolohn, VOR) | schwerer, aber skaliert für viel-Feld-Haushalte |

**EMPFEHLUNG: Option A (MVP).** Nur die per-Person-Einkommensfelder doppeln (`_partner`); globale Felder
(veranlagung, Kinder, agB, Spende, §35a-Haushalt) bleiben EINFACH (geteilt). Option B (echtes Personen-
Modell) als benannter Struktur-Nachtrag, falls der Haushalts-Umfang wächst. **Dein Entscheid — das ist
der größere Struktur-Zuwachs, den du gemeint hast.**

## Die 4 Punkte (unter Option A)

### 1. Bindungs-Bedarf (neue Felder)
- `bruttoarbeitslohn_partner` (§ 19, Kz E0200201 in Anlage-N-Instanz B), `vor_an_anteil_rv_partner` /
  `vor_ag_anteil_rv_partner` / `vor_rv_ausserhalb_lstb_partner` (VOR je Person, eigene LStB).
- `person_b_idnr` (Mantelbogen, E0100082) + ggf. `person_a_idnr` (E0100081) — 1:1-Kz (distinkt A/B).
- Anker: § 26b EStG (Zusammenveranlagung) + § 19/§ 10 je Person; § 26-Freeze vorhanden.
- **Offen:** die 4 Abwesenheits-Flags (kein_gewinn/kap/vuv/sonstige) — Haushalt-geteilt (MVP) oder je
  Person? § 2 ist personenbezogen; für den reinen-AN-zusammen-Fall reicht Haushalt-geteilt → als
  benannte Vereinfachung führen.

### 2. est_mapping: Person-Multiplikation (neue Klasse, analog Kind Klasse e)
- `_partner`-Felder → Anlage-N-INSTANZ B (dieselben Kz E0200201/E2000401, zweites Sub-Dokument), NICHT
  neue Kz. Nur `person_b_idnr` → distinktes E0100082.
- Drift-Wächter zieht mit (Instanz-Kz = keine Kollision, Transform-Quelle die `_partner`-Felder).

### 3. Splitting-Tarif-Ehrlichkeit (fail-closed über BEIDE Personen)
- Der zusammen-Bescheid rechnet NUR bei **vollständig bestätigtem Kegel BEIDER Personen** (alle A- UND
  B-Einkommensfelder `bestaetigt`). Hängt EIN Person-B-Feld `vorlaeufig`/offen → **unvollständig, kein
  Splitting-Teilergebnis** (meet_zustand über die vereinigte A∪B-Feldmenge). Kein halber Splitting-Bescheid.

### 4. Zone-Schnitt
- **Meine Zone:** die `_partner`-Bindungen + Person-Multiplikations-Mapping + Drift-Wächter + Gate.
- **dev-1-Haut:** der zusammen-slot_fn (per-Person §9a-Rechnung → joint einkuenfte_ns → gesamt_zusammen-
  Scope). Ich liefere die Deklarations-/Input-Seite.

## Zur Abnahme
(a) Store-Modell A (flach _partner, Empfehlung) vs B (Personen-Dimension)? (b) Abwesenheits-Flags
Haushalt-geteilt (MVP) vs je Person? (c) person_b_idnr-Feld + E0100082 direkt oder Kz-Review? (d)
Person-Multiplikations-Klasse in est_mapping OK (analog Kind)? Nach Abnahme: `_partner`-Bindungen +
est_mapping-Klasse + fail-closed-über-beide + Gate.
