# Redo-Judge 2026-07-11 — Statusmeldung (drei Listen)

Frische `dekomponiert@2`-Verdikte für 6 aktive Regeln. Kosten ~$2,64
(mehr als die geschätzten $1,5 — Nicht-Sättigung liefert viele Items:
p35a 57, p9_4a 38 Prüf-Items in einem Lauf).

## Kernbefund: mechanisches Seeding NICHT möglich

Die Triage vom 2026-07-11 war gegen die **alten** anker-losen Draft-Items
(`referenz: ?`). Die frischen Verdikte tragen **echte** Anker
(`betrifft_kat`/`ref`). Es gibt keine exakte Anker-Entsprechung zwischen
Prosa-Triage und frischen Ankern. Fuzzy ist verboten ("im Zweifel eskalieren")
→ es wurde für die 6 Regeln **nichts** automatisch geseedet.

Was der Lauf lieferte: echte Anker + Detektor-Vorschlag pro Item. Der Detektor
(`item_annahme@2`, 3-Stimmen-Mehrheit) hat selbst gemappt:
- **52 Items → bedingung_neu** (auf bereits deklarierte Bedingungen)
- **19 Items → nicht_material** (auf globale Konventionen `konv:*`)
- **102 Items → offen** (kein Mapping, brauchen Julius-Triage)

Diese Vorschläge sind **vorbelegt**, aber NICHT verbindlich — du bestätigst
oder korrigierst (AINA). Entwürfe: `pipeline/item_registry/discovery/*.yaml`.

## Liste 1 — verified_bedingt erreicht

**Keine.** Kein Rule advanced. Grund: Registries der 6 Regeln sind leer
(nur p33 war geseedet), und Seeding braucht deine Triage der frischen Items.

## Liste 2 — an Discovery-Triage wartend

| Regel | Items | offen | Detektor→bed | Detektor→nmat |
|---|---|---|---|---|
| p24b_entlastungsbetrag | 31 | 17 | 11 | 3 |
| p10_1_7_berufsausbildung | 12 | 6 | 1 | 5 |
| p9_6_erstausbildung_abgrenzung | 18 | 9 | 4 | 5 |
| p9_1_3_nr5_doppelte_haushaltsfuehrung | 17 | 14 | 2 | 1 |
| p35a_2_3_haushaltsnahe | 57 | 28 | 27 | 2 |
| **Summe (5 Regeln)** | **135** | **74** | **45** | **16** |

Die 74 offenen = **63 distinkte (betrifft, kategorie)-Gruppen**. Löwenanteil
(~60) ist `norm_teil` mit `abgedeckt_von: none` — Normteile, die der Judge als
`wirkt_hinein` klassifizierte, aber keiner Bedingung zuordnen konnte. Sie
blockieren nichts (leere Registry → kein Gap), sind aber zu triagieren:
echte Geltungsvoraussetzung (`bedingung_neu`) oder außerhalb (`nicht_material`,
§-33-Präzedenz: großer Ausschnitt ist die Norm, nicht der Fehler).

## Liste 3 — blockiert

| Regel | Grund | Status |
|---|---|---|
| p33_3_zumutbare_belastung | A-Rundungsdefekt (A rundet ab, B nicht). `defekt_formalisierer` + `freigabe:blockiert`. | Echt, bleibt bis A-Rerun. |
| p9_4a_verpflegungsmehraufwand | `freigabe:blockiert`, Grund: "Judge nicht reproduzierbar, 3 Läufe 3 Ergebnisse". | **Grund überholt** — genau das hat die Ratsche gelöst. Eskalation: Block aufheben? |

p9_4a hat zusätzlich 38 Items (28 offen) in der Discovery-Queue, unabhängig vom
Block.

## Entscheidung nötig (Julius)

Der Lauf legt die Nicht-Sättigungs-Steuer offen: ~100 offene Items **pro Lauf**,
größtenteils `norm_teil`-Rauschen. Bevor 102 Items einzeln durch die Triage
gehen:

1. **p9_4a-Block**: aufheben (Grund von Ratsche überholt) oder aus anderem
   Grund halten?
2. **Triage-Modus**: die 63 offenen Gruppen jetzt durchtriagieren (einmalige
   Kosten, danach Ratsche deterministisch), oder erst die `norm_teil`-Flut
   strukturell dämpfen (z.B. Konvention "großer Normausschnitt ist zulässig")?
3. **Detektor-Vorschläge** (52 bed + 19 nmat): pauschal bestätigen oder
   einzeln prüfen?
