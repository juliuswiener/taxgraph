# Bindungstabelle — Schema (UI-Kern, Task #11)

**Zone:** `produkt/` (neu, additiv, LLM-frei). **Kein** Touch an rules.yaml / item_registry / elster.
**Status:** SCHEMA zur Instructor-Abnahme VOR dem Scheiben-Bau (Schema-first = teuerste Fehlerquelle).

## Zweck

Die Bindungstabelle ist die Brücke zwischen **Regel-Fachschicht** (Signatur-Inputs +
Geltungsbedingungen der Catala-Regeln) und der **Laien-Eingabe-Schicht** des UI. Jeder Eintrag bindet
genau **einen** Regel-Slot an genau **ein** Abfrage-Feld, deterministisch und amtlich verankert. Sie
ist die materialisierte Form des Lab-Kernbefunds „Store ist Wahrheit + jede Kante trägt Herkunft".

Maschinenlesbar validiert gegen `produkt/bindung/schema.json` (JSON Schema 2020-12). Daten liegen als
YAML (`produkt/bindung/bindung_*.yaml`), werden geladen und gegen das Schema geprüft (Gate).

## Felder je Eintrag (`bindungen[]`)

| Feld | Typ | Pflicht | Bedeutung |
|---|---|---|---|
| `feld_id` | string (snake_case) | ✓ | Eindeutiger stabiler Schlüssel des Abfrage-Felds. |
| `quelle` | object | ✓ | `{regel_id, signatur_slot}` **oder** `{regel_id, geltungsbedingung}` (genau eins). |
| `typ` | enum | ✓ | `cent` (Geld in Cent, Ganzzahl) / `int` / `bool` / `enum` / `datum` (ISO-8601) / `text`. |
| `slot_beitrag` | enum | – (Default `exakt`) | `exakt` = Feld IST der Slot-Wert; `summand` = Feld ist ein Summand (s. Summen-Konvention). |
| `einheit` | string\|null | ✓ | z.B. `km`, `Tage`, `EUR`; `null` bei bool/enum/datum. |
| `askable` | bool | ✓ | `true` = der Laie gibt es ein; `false` = **abgeleitete/berechnete/Parameter**-Größe (nie abfragen). |
| `fragetext_laie` | string\|null | ✓ falls askable | **Einfaches Deutsch, KEINE Paragraphen/Amtssprache** (s. Doktrin). Bei `askable=false` `null`. |
| `hilfe_kurz` | string | ✓ | Kurzer Hilfetext, ebenfalls laienverständlich. |
| `beispielwert` | passend zu typ | ✓ | z.B. bei `cent` Ganzzahl-Cent (132000 = 1.320,00 €). |
| `herkunft_slots` | array\|null | ✓ | Provenance je Teilbetrag, z.B. `["Lohnsteuerbescheinigung Nr. 23"]` (VOR-Musterfall). |
| `elster_kz` | `E#######`\|null | ✓ | Amtliche XSD-E-Nummer; `null` → `elster_kz_grund` Pflicht (kein Rate-Mapping). |
| `elster_kz_grund` | string | falls Kz null | Warum die E-Nr (noch) nicht belegt ist. |
| `vz_gueltigkeit` | int[] | ✓ | Veranlagungszeiträume, z.B. `[2024, 2025, 2026]`. |
| `anker_ref` | `{quelle, zitatanker, datei?}` | ✓ | Norm-Fundstelle + wörtlicher Zitatanker (das „warum"). `datei` optional (Pflicht bei Catala-Quellen wie EP). |
| `enum_werte` | string[] | falls typ=enum | Zulässige Werte. |
| `bereich` | `{min, max, grund?}` | – (nur cent/int) | Wertebereichsgrenzen fürs Unsicherheits-Derivat. Fehlt = unbounded (offene Intervallseite). Gate: min≤max, ganzzahlig, cent<0 nur mit `grund` (Verluste). |

## Summen-Konvention (Auflage A)

Ein Regel-`signatur_slot` kann fachlich aus **mehreren** Laien-Feldern zusammengesetzt sein (der
VOR-Musterfall: `gesamtbeitraege_inkl_ag` = AN-Anteil ∪ AG-Anteil ∪ Beiträge außerhalb LStB). Regel:

- Mehrere Bindungen dürfen **denselben** `signatur_slot` referenzieren, wenn alle `slot_beitrag: summand`
  tragen. Der Slot-Wert ist dann die **Summe** der Felder.
- Nur `typ` **cent** oder **int** (addierbar), und **typ-homogen** über alle Summanden eines Slots.
- Der Gate prüft: pro Slot höchstens EIN `exakt`-Feld **oder** ausschließlich `summand`-Felder (kein
  Mischen), plus Typ-Homogenität. Jeder Summand trägt seine eigene `herkunft_slots`-Provenance
  (z.B. LStB-Zeile) — genau die Provenance-Frage des UI-Kerns.

## `datei` / Anker-Verifikation (Auflage B — Gate d)

Jeder `anker_ref.zitatanker` wird **voll-Länge** via `pipeline/gates._normalize` gegen die Quelldatei
geprüft. Die Datei löst der Gate über `regel_id → rules.yaml norm_source`; bei Catala-Scope-Quellen
(EP hat keinen rules.yaml-Eintrag) MUSS `anker_ref.datei` explizit gesetzt sein. Ohne diese Prüfung
driften UI-Anker von den Freezes ab — die Zitatanker-Doktrin gilt auch hier.

## fragetext_laie-Doktrin (mechanisiert im Schema)

Der Fragetext ist für einen steuerlichen **Laien**. **Verboten** (per `not`-Pattern im Schema
abgelehnt): `§`, `EStG`/`GewStG`/`KStG`, `Abs.`, `Satz N`, `i.S.d.`, „Aufwendungen i[.S.d.]".
Die Paragraphen hängen ausschließlich als `anker_ref` dran (das aufklappbare „warum"), NIE im Fragetext.

- ❌ schlecht: „Aufwendungen i.S.d. § 9 Abs. 1 S. 3 Nr. 4a EStG"
- ✅ gut: „Wie viele Tage bist du zur Arbeit gefahren?"

## `luecken[]` — benannte Nicht-Abdeckung

Ein Regel-Slot oder eine Geltungsbedingung, die die Scheibe bewusst NICHT bindet, wird als
`luecken`-Eintrag mit `grund` geführt. Der Gate akzeptiert **Bindung ODER benannte Lücke** — stille
Nicht-Abdeckung ist ROT.

## `regel_bedingungen[]` — Ob-Bedingung regel-weit (nicht je Gate-Feld)

`relevanz()` (`produkt/traverser/traverser.py`) schließt eine Regel bisher nur über EIGENE
`askable`-bool-Geltungsbedingungen aus (`false` bestätigt → ausgeschlossen). Trägt eine Regel
ausschließlich nicht-bool Gates (enum/text/datum), kann sie so strukturell **nie** ausgeschlossen
werden, egal was der Nutzer antwortet (Dialog-Überangebot).

`regel_bedingungen[]` (Top-Level, additiv, `$defs/regel_bedingung`) schließt diese Lücke: EIN
`{regel_id, feld, wert, grund}`-Eintrag pro betroffener Regel, ausgewertet regel-weit — `feld` muss
kein eigenes Feld der Regel sein. Bestätigt UND abweichend → ausgeschlossen; unbeantwortet/vorläufig
schließt NICHT aus (fail-closed, wie die bool-Gates). Beispiel: `p2_festzusetzung_zusammen` gilt nur
bei `veranlagung == "zusammen"` (`bindung_regel_bedingungen.yaml`).

## Gate-Vertrag (`tests/test_bindungstabelle.py`)

1. **(a) Schema-Validierung** — jede `bindung_*.yaml` validiert gegen `schema.json`.
2. **(b) Vollständigkeit je erfasster Regel** — für jede in der Scheibe geführte Regel: **jeder**
   askable Signatur-Slot **und jede** Geltungsbedingung hat entweder eine Bindung **oder** eine
   benannte Lücke. (Für Regeln in rules.yaml aus deren `geltungsbedingungen`; askable Slots aus der
   Catala-Signatur minus Parameter.)
3. **(c) elster_kz-Existenz** — jede nicht-null `elster_kz` existiert im XSD E10-2025 (Abgleich via
   `elster/kz_extract.py`). **Auflage C (Entscheid):** `elster_kz` bindet **nur ESt-Kz** (E10-2025).
   EÜR-Betrags-Kz (`E60…`, eigene Datenart) werden NICHT als `elster_kz` gebunden, sondern nur in
   `elster_kz_grund` referenziert (wie das GWG-Beispiel). Begründung: Gate (c) bleibt einschemig
   (E10-2025); EÜR ist eine andere Datenart mit eigenem XSD (später, kein Datenart-Misch).
4. **(d) Anker-Verifikation** — jeder `anker_ref.zitatanker` voll-Länge via `pipeline/gates._normalize`
   gegen die Quelldatei (`anker_ref.datei` oder `regel_id → rules.yaml norm_source`).
5. **Negativtest (Pflicht):** absichtlich manipulierte Einträge (`§` im Fragetext, erfundene E-Nr,
   unbelegter Slot, verfälschter Zitatanker, gemischte `exakt`/`summand` auf einem Slot) MÜSSEN das
   Gate ROT färben.

## Worked Examples (illustrativ — Scheibe folgt nach Abnahme)

```yaml
version: 1
scheibe: "N (EP/Arbeitsmittel/dHf/Verpflegung) + VOR + GWG"
bindungen:
  # --- EP: Signatur-Slot (askable Input) ---
  - feld_id: ep_arbeitstage
    quelle: {regel_id: p09_entfernungspauschale, signatur_slot: arbeitstage}
    typ: int
    einheit: Tage
    askable: true
    fragetext_laie: "An wie vielen Tagen bist du im Jahr zur Arbeit gefahren?"
    hilfe_kurz: "Zähl nur die Tage, an denen du wirklich zur Arbeitsstelle gefahren bist."
    beispielwert: 220
    herkunft_slots: null
    elster_kz: E0203503
    vz_gueltigkeit: [2024, 2025, 2026]
    anker_ref:
      quelle: "§ 9 Abs. 1 S. 3 Nr. 4 EStG"
      zitatanker: "für jeden Arbeitstag, an dem der Arbeitnehmer die erste Tätigkeitsstätte aufsucht"

  # --- VOR: EIN Slot aus DREI Summanden-Feldern (Summen-Konvention, LStB-Split-Musterfall) ---
  - feld_id: vor_an_anteil_rv
    quelle: {regel_id: p10_1_2_altersvorsorge, signatur_slot: gesamtbeitraege_inkl_ag}
    typ: cent
    slot_beitrag: summand
    einheit: EUR
    askable: true
    fragetext_laie: "Wie viel hast du selbst in die gesetzliche Rentenversicherung eingezahlt (dein Anteil vom Lohn)?"
    hilfe_kurz: "Steht auf deiner Lohnsteuerbescheinigung in Zeile 23."
    beispielwert: 3500000
    herkunft_slots: ["Lohnsteuerbescheinigung Nr. 23 a/b"]
    elster_kz: E2000401
    vz_gueltigkeit: [2024, 2025, 2026]
    anker_ref:
      quelle: "§ 10 Abs. 1 Nr. 2, Abs. 3 EStG"
      zitatanker: "Beiträge zu den gesetzlichen Rentenversicherungen"
      datei: "sources/gesetze-im-internet/estg_p10_2026-07-11.txt"
  - feld_id: vor_ag_anteil_rv
    quelle: {regel_id: p10_1_2_altersvorsorge, signatur_slot: gesamtbeitraege_inkl_ag}
    typ: cent
    slot_beitrag: summand
    einheit: EUR
    askable: true
    fragetext_laie: "Wie viel hat dein Arbeitgeber steuerfrei dazugegeben (Zuschuss zur Rente)?"
    hilfe_kurz: "Steht auf deiner Lohnsteuerbescheinigung in Zeile 22."
    beispielwert: 3500000
    herkunft_slots: ["Lohnsteuerbescheinigung Nr. 22 a/b"]
    elster_kz: E2000801
    vz_gueltigkeit: [2024, 2025, 2026]
    anker_ref:
      quelle: "§ 10 Abs. 1 Nr. 2, Abs. 3 EStG"
      zitatanker: "Beiträge zu den gesetzlichen Rentenversicherungen"
      datei: "sources/gesetze-im-internet/estg_p10_2026-07-11.txt"
  - feld_id: vor_rv_ausserhalb_lstb
    quelle: {regel_id: p10_1_2_altersvorsorge, signatur_slot: gesamtbeitraege_inkl_ag}
    typ: cent
    slot_beitrag: summand
    einheit: EUR
    askable: true
    fragetext_laie: "Hast du außerhalb vom Lohn noch selbst in eine gesetzliche Rente eingezahlt? Wenn ja, wie viel?"
    hilfe_kurz: "Zum Beispiel freiwillige Beiträge, die nicht auf der Lohnsteuerbescheinigung stehen."
    beispielwert: 0
    herkunft_slots: ["außerhalb Lohnsteuerbescheinigung"]
    elster_kz: E2000601
    vz_gueltigkeit: [2024, 2025, 2026]
    anker_ref:
      quelle: "§ 10 Abs. 1 Nr. 2, Abs. 3 EStG"
      zitatanker: "Beiträge zu den gesetzlichen Rentenversicherungen"
      datei: "sources/gesetze-im-internet/estg_p10_2026-07-11.txt"

  # --- GWG: Geltungsbedingung (bool-Abfrage) ---
  - feld_id: gwg_netto_ohne_vorsteuer
    quelle: {regel_id: p6_2_gwg_sofortabzug, geltungsbedingung: netto_ist_9b_bereinigt}
    typ: bool
    einheit: null
    askable: true
    fragetext_laie: "Ist der Preis, den du eingibst, ohne Mehrwertsteuer (netto)?"
    hilfe_kurz: "Wenn du vorsteuerabzugsberechtigt bist, rechnest du netto."
    beispielwert: true
    herkunft_slots: null
    elster_kz: null
    elster_kz_grund: "Reine Geltungsbedingung ohne eigenes Deklarationsfeld; Betrag geht in E6002301 (Anlage EÜR)."
    vz_gueltigkeit: [2024, 2025, 2026]
    anker_ref:
      quelle: "§ 6 Abs. 2 S. 1 EStG"
      zitatanker: "vermindert um einen darin enthaltenen Vorsteuerbetrag (§ 9b Absatz 1)"

luecken:
  - regel_id: p10_1_2_altersvorsorge
    geltungsbedingung: hoechstbeitrag_ist_parameter
    grund: "Parameter (Höchstbeitrag knappschaftl. RV) aus params/, keine Laien-Abfrage."
```

## Entscheidungen (Instructor abgenommen, msg 2366)

1. **EP nicht in rules.yaml:** EP zieht die **Catala-Input-Signatur** (`p09_entfernungspauschale`),
   nicht rules.yaml-Bedingungen. Bestätigt.
2. **Parameter deterministisch ausgenommen:** Der Gate leitet die Parameter-Menge aus den **Keys von
   `params/<vz>/`** ab (Datei-Abgleich), NICHT aus Namens-Heuristik. Ein Catala-Input, dessen Name ein
   Parameter-Key ist, ist `askable:false` und braucht keine Bindung.
3. **cent = Ganzzahl Cent:** bestätigt (deckt sich mit `*_cent`-Registry-Konvention).
4. **YAML-Daten + JSON-Schema:** bestätigt.
5. **Auflage A/B/C** oben eingearbeitet (Summen-Konvention, Anker-Gate d, elster_kz einschemig E10-2025).

## Offener Produkt-Punkt (kein Blocker)

- **Anrede `du`/`Sie`:** aktuell einheitlich `du` als Arbeitsstand in `fragetext_laie`. Ein
  du/Sie-Umschalter ist späterer Produkt-Feinschliff (nicht im Binding, sondern in der Render-Schicht).

Nach Abnahme dieser Fassung: Scheibe N+VOR+GWG (`bindung_n_vor_gwg.yaml`) + Gate
`tests/test_bindungstabelle.py` (Schema/Vollständigkeit/elster_kz/Anker + Negativtests).
