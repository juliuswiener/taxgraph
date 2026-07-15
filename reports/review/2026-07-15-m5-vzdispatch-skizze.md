# M5-Vorarbeit — Konzept-Skizze VZ-Dispatch (taxgraph-dev-2, 2026-07-15)

Read-only Skizze zur Instructor-Review. KEIN Bau. Frage: wie kommen die
`params/<vz>`-Werte in die Integrations-Scopes, und welche Regel-Versions-Konvention
(rule_id-Suffix `_vz2024` vs. `gueltig_ab`-Feld vs. Bestehendes)?

## 1. Bestandsaufnahme — der VZ-Dispatch EXISTIERT bereits in zwei Ausprägungen

Das MVP ist multi-VZ-gebaut. Zwei belegte Muster im Code:

**Muster A — embedded `match vz` (Tarif-Präzedenz).**
`rules/estg/p32a/einkommensteuertarif.catala_en:24` deklariert die Enum
`Veranlagungszeitraum: VZ2024 | VZ2025 | VZ2026`; die Jahreswerte stehen als
Literale in einem `depends on vz ... match vz with pattern -- VZ2024: {...}`-Block
IM Catala. `params/<vz>/einkommensteuertarif_p32a.yaml` ist Provenance, `params-check`
(`derive_coefficients.py`) verifiziert literal==abgeleitet. Werte im Catala, params
dokumentieren.

**Muster B — Runtime-Input-Injektion (MVP-Andock-Präzedenz).**
`rules/estg/p04_arbeitszimmer_homeoffice`, `p09_entfernungspauschale`: das Catala-Scope
deklariert die Sätze als `input` (z. B. `jahrespauschale_in`, `satz_bis_20_km_in`);
`golden/runner.py` liest sie per `_<topic>_params(year)` aus `params/<year>/<topic>.yaml`
und reicht sie als Scope-Inputs. **params/<vz>.yaml ist die LIVE-Quelle** (Runtime-Lesung).

**Enum ist schon durchgefädelt:** alle vier Integrations-Scopes (`familie1-4`) tragen
`input veranlagungszeitraum content Einkommensteuertarif.Veranlagungszeitraum`
(familie1:68, familie2:71, familie3:64, familie4:95). `runner.py` mappt
`VZ_ENUM = {2024: …VZ2024, 2025: …VZ2025, 2026: …VZ2026}`.

## 2. Wie params/<vz> in die Scopes kommen — Antwort: Muster B erweitern

Für die 6 M3-Themen ist der Andockpunkt `golden/runner.py`: je Thema ein Loader
analog `_ep_saetze(year)`, der `params/<year>/<topic>.yaml` liest und die `wert`-Felder
als Scope-Inputs übergibt. Meine M3-Dateien sind exakt in diesem Format — sie stecken
sich direkt ein, sobald das jeweilige Regel-Scope die Konstante als `input` führt statt
sie zu backen. Die Integrations-Scopes fädeln `veranlagungszeitraum` bereits durch; sie
gewinnen nur die neuen Param-Inputs als Weiterreichung an die Unter-Scopes.

## 3. Regel-Versions-Konvention — Bewertung aus Bestandssicht

| Option | Bewertung |
|---|---|
| **rule_id-Suffix `_vz2024`** | ✗ 3× Regel-Explosion (81→bis 3× je Drift-Regel), 3× Anker-Volllängen-Verifikation, duplizierte geltungsbedingungen, Registry-Ratschen-Last. Nur bei echtem Struktur-Divergenz gerechtfertigt — trifft auf KEINE der 6 zu. |
| **`gueltig_ab`-Feld als Regel-Dispatch** | ✗ neue Maschinerie ohne Codebase-Support; ein Dispatcher müsste je VZ die Regelversion wählen. `gueltig_ab` existiert nur als params-Datei-Feld (Provenance), nicht als Regel-Selektor. |
| **Muster B (Input-Injektion)** | ✓ EIN rule_id, params.yaml live, runner injiziert. Kein Fassungs-Drift möglich (Quelle = params). Für reine SKALAR-Drift. |
| **Muster A (embedded `match vz`)** | ✓ EIN rule_id, ein Scope, VZ als Runtime-Input, Werte in einem `match vz`-Block. Bewährt (Tarif). Für STRUKTUR-Drift (Formel ändert sich). Braucht params↔catala-Sync-Gate. |

**Empfehlung: Hybrid B (Skalar) + A (Struktur). KEIN rule_id-Suffix, KEIN neues
`gueltig_ab`-Dispatch.** `gueltig_ab` bleibt params-Provenance-Feld.

- Skalar-Drift (KFB, Soli-FG, Kindergeld, Unterhalt-HB, vorsorge-HB) → **Muster B**:
  Regel nimmt Wert als Input, runner liest params/<vz>.
- Struktur-Drift (Kinderbetreuung ⅔→80 %, Formel-Koeffizient ändert sich) → **Muster A**:
  `depends on vz`-Block für Satz+Deckel; ODER `abzugssatz` als Input (B) mit VZ-2024-⅔
  als eigenem params-Wert (liegt schon). B genügt sogar hier, da der Satz ein Faktor-Input
  ist, keine Formel-Verzweigung — Empfehlung: auch Kinderbetreuung via B, spart Muster A ganz.

## 4. HAUPTFUND — Bestands-Randbedingung: handgeschrieben vs. formalisiert

Die Mechanik-Wahl ist NICHT frei, sondern durch die Herkunft der Regel-Catala
eingeschränkt (Kosten- und Ownership-Grenze):

| Regel | Herkunft | Kosten M5 | Weg |
|---|---|---|---|
| `kindergeld` (p31-Input) | schon `input money` | **$0** | nur runner-Loader + Integration-Verdrahtung |
| `vorsorge_hoechstbetrag` (p10_1_2-Input) | schon `input money` | **$0** | nur runner-Loader + Ableitungs-Doku |
| `solzg` Freigrenze | **handgeschrieben** (`rules/estg/solzg/solzg.catala_en`, `[handgeschrieben]`) | **$0** | direkt Muster A/B, kein Formalisierer |
| `p32_6` KFB | **formalisiert** | Kosten | rules.yaml `signature.inputs` += KFB-Input (dev-1/TABU) + Re-Formalisierung ODER Hand-Patch des generierten Catala |
| `p33a` Unterhalt-HB | **formalisiert** | Kosten | wie p32_6 |
| `p10_1_5` Kinderbetreuung | **formalisiert** + STRUKTUR | Kosten | wie p32_6, zusätzlich VZ-2024-Fassungs-auszug (⅔/4000) als zweite Freeze-Quelle |

Belegt: p32_6/p33a/p10_1_5 backen ihre Konstanten, weil `signature.inputs` in
rules.yaml den Wert nicht als Input führt und der Formalisierer den auszug-Wert als
Catala-Literal materialisiert. Sie zu parametrisieren = rules.yaml-Signatur-Änderung
(**dev-1-Territorium, TABU für mich**) + neuer Formalisierer-Lauf (Kosten, nur unter
Instructor-Cap-Wort) ODER deterministischer Hand-Patch des generierten `catala_a/b`.

## 5. Empfohlene M5-Reihenfolge (billig→teuer, kostentransparent)

1. **Stufe 1 ($0):** `kindergeld` + `vorsorge_hoechstbetrag` — schon `input`, nur
   runner-Loader (`_kindergeld(year)`, `_vorsorge_hb(year)`) + familie1-4-Verdrahtung.
2. **Stufe 2 ($0):** `solzg` (handgeschrieben) → Muster A `match vz` für Freigrenze
   einzel/splitting; Satz 5,5 %/11,9 % bleiben Literale (fix). params als Sync-Quelle.
3. **Stufe 3 (Kosten, dev-1):** `p32_6` + `p33a` → rules.yaml `signature.inputs` +=
   Wert-Input; Re-Formalisierung ODER Hand-Patch. Anker-Verifikation nur der neuen Inputs.
4. **Stufe 4 (Kosten, dev-1, strukturell):** `p10_1_5` Kinderbetreuung → Satz+Deckel als
   Input (Muster B); braucht VZ-2024-Fassungs-Freeze (§ 10 I 5 a. F., ⅔/4000). Instructor
   besorgt Alt-Fassung analog WachstumschancenG-Muster.
5. **Quer (read-only, $0, Agent-Fanout):** VZ-Golden je Jahr — ein Golden-Satz pro VZ
   (2024/2025/2026), deterministisch aus params + GETTSIM-Orakel konstruiert.

## 6. Sync-Gate-Bedarf
- Muster B: params.yaml = Runtime-Quelle → kein Drift möglich, kein Sync-Gate nötig.
- Muster A (falls für solzg gewählt): braucht params↔catala-Literal-Konsistenz-Gate
  analog `derive_coefficients.py`/`params-check`, sonst Fassungs-Drift (gefährlichste
  Falsch-Grün-Quelle, siehe Gültigkeits-Direktive).

## Offene Punkte an Instructor
- OK für Hybrid B+A ohne rule_id-Suffix, `gueltig_ab` bleibt Provenance?
- Stufe 3/4 (formalisierte Regeln): Re-Formalisierung ODER deterministischer Hand-Patch
  des generierten Catala? Hand-Patch = $0 + Anker-Verifikation, aber berührt
  `pipeline/runs/produktion/<rule_id>/` (dev-1). Re-Formalisierung = sauberer, aber Kosten.
- rules.yaml-`signature.inputs`-Änderungen sind dev-1-exklusiv — Koordination über dich.
