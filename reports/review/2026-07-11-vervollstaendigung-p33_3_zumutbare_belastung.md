# Vervollständigung p33_3_zumutbare_belastung — zur Absegnung (2026-07-11)

Akkumulierte Item-Union aus 1 gespeicherten Verdikt(en). Vorschläge für
fehlende Geltungsbedingungen; nichts davon ist im Manifest aktiv.

**Vorbehalt:** Diese Union stammt aus den vorhandenen Verdikten. Für ein
belastbares Paket sollte sie aus einem frischen Union-until-Saturation-Lauf
je Regel erzeugt werden (die Inventarstufe streut). Zitatanker sind
Platzhalter und gegen den eingefrorenen Normtext zu prüfen.

Bereits deklariert: 0 Bedingung(en).

## A) Ungedeckte `wirkt_hinein`-Norm-Teile → Bedingungs-Kandidaten

- **§ 33 Abs. 3 Satz 2** (wirkt_hinein in 1/1 Verdikten)
  - Zitat: Als Kinder des Steuerpflichtigen zählen die, für die er Anspruch auf einen Freibetrag nach § 32 Absatz 6 oder auf Kindergeld hat.
  - Vorschlag:
    ```yaml
    - bedingung: <name>
      deckt_ab: "<wörtliche Passage aus § 33 Abs. 3 Satz 2>"
      quelle: "§ 33 Abs. 3 Satz 2"
      beschreibung: "<was die Bedingung an/ausschaltet>"
    ```
- **§ 33 Abs. 3 Satz 1 Nr. 1 Buchst. b** (wirkt_hinein in 1/1 Verdikten)
  - Zitat: nach § 32a Absatz 5 oder 6 (Splitting-Verfahren) zu berechnen ist
  - Vorschlag:
    ```yaml
    - bedingung: <name>
      deckt_ab: "<wörtliche Passage aus § 33 Abs. 3 Satz 1 Nr. 1 Buchst. b>"
      quelle: "§ 33 Abs. 3 Satz 1 Nr. 1 Buchst. b"
      beschreibung: "<was die Bedingung an/ausschaltet>"
    ```
- **§ 33 Abs. 3 Satz 2 EStG** (wirkt_hinein in 1/1 Verdikten)
  - Zitat: Als Kinder des Steuerpflichtigen zählen die, für die er Anspruch auf einen Freibetrag nach § 32 Absatz 6 oder auf Kindergeld hat.
  - Vorschlag:
    ```yaml
    - bedingung: <name>
      deckt_ab: "<wörtliche Passage aus § 33 Abs. 3 Satz 2 EStG>"
      quelle: "§ 33 Abs. 3 Satz 2 EStG"
      beschreibung: "<was die Bedingung an/ausschaltet>"
    ```
- **§ 33 Abs. 3 Satz 1** (wirkt_hinein in 1/1 Verdikten)
  - Zitat: bei denen die Einkommensteuer nach § 32a Absatz 5 oder 6 (Splitting-Verfahren) zu berechnen ist
  - Vorschlag:
    ```yaml
    - bedingung: <name>
      deckt_ab: "<wörtliche Passage aus § 33 Abs. 3 Satz 1>"
      quelle: "§ 33 Abs. 3 Satz 1"
      beschreibung: "<was die Bedingung an/ausschaltet>"
    ```

## B) Undeklarierte Annahmen → Input-Semantik-Kandidaten

- **anzahl_kinder** / `interpretation` (undeklariert in 4/4 Verdikten)
  - Annahme: Die Eingabe 'anzahl_kinder' wird als Anzahl der Kinder im Sinne des § 33 Abs. 3 Satz 2 EStG interpretiert, ohne dass die Anspruchsvoraussetzungen geprüft werden.
  - Vorschlag:
    ```yaml
    - bedingung: anzahl_kinder_interpretation
      deckt_ab: "<wörtliche Passage, die diese Lesart festlegt>"
      quelle: "<§ ...>"
      beschreibung: "Input-Semantik: anzahl_kinder (interpretation) ..."
    ```
- **splitting** / `interpretation` (undeklariert in 2/2 Verdikten)
  - Annahme: Die boolesche Eingabe 'splitting' wird als Indikator dafür interpretiert, dass die Einkommensteuer nach dem Splitting-Verfahren berechnet wird.
  - Vorschlag:
    ```yaml
    - bedingung: splitting_interpretation
      deckt_ab: "<wörtliche Passage, die diese Lesart festlegt>"
      quelle: "<§ ...>"
      beschreibung: "Input-Semantik: splitting (interpretation) ..."
    ```
- **ergebnis** / `rundung` (undeklariert in 2/2 Verdikten)
  - Annahme: Das Ergebnis wird durch Abschneiden der Nachkommastellen auf ganze Euro abgerundet, obwohl die Norm keine Rundung vorschreibt.
  - Vorschlag:
    ```yaml
    - bedingung: ergebnis_rundung
      deckt_ab: "<wörtliche Passage, die diese Lesart festlegt>"
      quelle: "<§ ...>"
      beschreibung: "Input-Semantik: ergebnis (rundung) ..."
    ```

## Absegnung

Pro Kandidat: übernehmen (dann trage ich die Bedingung mit geprüftem
Zitatanker ein), ändern, oder als Grenzfall in die Registry statt in
eine Bedingung.