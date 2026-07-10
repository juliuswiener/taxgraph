# Vervollständigung p9_4a_verpflegungsmehraufwand — zur Absegnung (2026-07-11)

Akkumulierte Item-Union aus 1 gespeicherten Verdikt(en). Vorschläge für
fehlende Geltungsbedingungen; nichts davon ist im Manifest aktiv.

**Vorbehalt:** Diese Union stammt aus den vorhandenen Verdikten. Für ein
belastbares Paket sollte sie aus einem frischen Union-until-Saturation-Lauf
je Regel erzeugt werden (die Inventarstufe streut). Zitatanker sind
Platzhalter und gegen den eingefrorenen Normtext zu prüfen.

Bereits deklariert: 6 Bedingung(en).

## A) Ungedeckte `wirkt_hinein`-Norm-Teile → Bedingungs-Kandidaten

- **?** (wirkt_hinein in 6/9 Verdikten)
  - Zitat: Satz 4: Hat der Arbeitnehmer keine erste Tätigkeitsstätte, gelten die Sätze 2 und 3 entsprechend.
  - Vorschlag:
    ```yaml
    - bedingung: <name>
      deckt_ab: "<wörtliche Passage aus ?>"
      quelle: "?"
      beschreibung: "<was die Bedingung an/ausschaltet>"
    ```

## B) Undeklarierte Annahmen → Input-Semantik-Kandidaten

- **None** / `None` (undeklariert in 1/6 Verdikten)
  - Annahme: Die Formalisierung nimmt an, dass 'monate_am_ort' die ununterbrochene Dauer ohne vierwöchige Unterbrechung erfasst.
  - Vorschlag:
    ```yaml
    - bedingung: None_None
      deckt_ab: "<wörtliche Passage, die diese Lesart festlegt>"
      quelle: "<§ ...>"
      beschreibung: "Input-Semantik: None (None) ..."
    ```

## Absegnung

Pro Kandidat: übernehmen (dann trage ich die Bedingung mit geprüftem
Zitatanker ein), ändern, oder als Grenzfall in die Registry statt in
eine Bedingung.