# Vervollständigung p24b_entlastungsbetrag — zur Absegnung (2026-07-11)

Akkumulierte Item-Union aus 1 gespeicherten Verdikt(en). Vorschläge für
fehlende Geltungsbedingungen; nichts davon ist im Manifest aktiv.

**Vorbehalt:** Diese Union stammt aus den vorhandenen Verdikten. Für ein
belastbares Paket sollte sie aus einem frischen Union-until-Saturation-Lauf
je Regel erzeugt werden (die Inventarstufe streut). Zitatanker sind
Platzhalter und gegen den eingefrorenen Normtext zu prüfen.

Bereits deklariert: 3 Bedingung(en).

## A) Ungedeckte `wirkt_hinein`-Norm-Teile → Bedingungs-Kandidaten

Keine.


## B) Undeklarierte Annahmen → Input-Semantik-Kandidaten

- **None** / `None` (undeklariert in 2/6 Verdikten)
  - Annahme: Die Eingabe 'alleinstehend' wird als korrekt gemäß der Definition in § 24b Abs. 3 EStG vorausgesetzt.
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