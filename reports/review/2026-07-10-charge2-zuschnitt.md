# Charge 2: Zuschnitt-Vorschläge — zur Entscheidung

Drei Regeln stehen auf `status: zuschnitt_offen` und laufen nicht mit. Der
Zuschnitt ist eine Entscheidung, keine Ausführung — deshalb hier ein Vorschlag,
kein Manifest-Eintrag.

Quellen sind bereits eingefroren, nach der neuen Konvention auf Paragraphen-Ebene:
`estg_p9_2026-07-10.txt` (§ 9 Abs. 1–6, 15.967 Zeichen) und
`estg_p6_2026-07-10.txt` (§ 6 Abs. 1–7, 24.000 Zeichen). Beide sha256-verifiziert;
die Passagen „Nummer 7 bleibt unberührt" und „800 Euro nicht übersteigen" sind
wörtlich enthalten.

---

## 1. Neue Regel: § 9 Abs. 1 S. 3 Nr. 6 + Nr. 7 gemeinsam

Ersetzt `p9_1_3_nr6_arbeitsmittel` und `p9_1_3_nr7_afa`.

**Warum zusammen.** Nr. 6 verweist auf Nr. 7 („Nummer 7 bleibt unberührt"), was der
Judge korrekt als `wirkt_hinein` gemeldet hat. Isoliert ist Nr. 6 eine
Identitätsfunktion — der Normtext ist buchstäblich 324 Bytes lang, davon der halbe
Metadaten-Kopf. Ein Test darauf prüft nichts.

**Warum Multi-Source.** Die 800-Euro-Grenze steht nicht in § 9, sondern folgt aus
dem Verweis in Nr. 7 Satz 2 auf § 6 Abs. 2. Dasselbe Muster wie § 33 Abs. 3 mit
dem BFH-Leitsatz: der Normtext bleibt unangetastet, die zweite Quelle kommt
etikettiert daneben.

```yaml
- rule_id: p9_1_3_nr6_7_arbeitsmittel_afa
  norm: § 9 Abs. 1 S. 3 Nr. 6 und Nr. 7 EStG
  quellen:
  - typ: gesetz
    label: "§ 9 Abs. 1 S. 3 Nr. 6 und Nr. 7 EStG"
    datei: sources/gesetze-im-internet/estg_p9_2026-07-10.txt
    # `auszug` folgt beim Manifest-Eintrag; er wird woertlich gegen die Quelle geprueft
  - typ: gesetz
    label: "§ 6 Abs. 2 EStG (geringwertige Wirtschaftsgueter)"
    datei: sources/gesetze-im-internet/estg_p6_2026-07-10.txt
  signature:
    scope: ArbeitsmittelUndAfa
    inputs:
      anschaffungskosten: money          # netto, ohne Vorsteuer
      nutzungsdauer_jahre: int
      anschaffungsmonat: int             # 1..12
    output: abziehbar
```

**Der Anschaffungsmonat gehört in die Signatur**, nicht in eine Geltungsbedingung.
Nach der Abgrenzungsregel: er *variiert den Betrag* (zeitanteilige AfA im
Anschaffungsjahr), er schaltet die Regel nicht an oder aus. Ohne ihn bräuchte die
Regel eine stille Volljahr-Annahme — genau das, was der Round-Trip-Judge zu Recht
meldet.

Vorgeschlagenes Raster (prüft beide Zweige und die Grenze):

```yaml
  raster:
  - {anschaffungskosten: 500,  nutzungsdauer_jahre: 3, anschaffungsmonat: 1}   # GWG
  - {anschaffungskosten: 800,  nutzungsdauer_jahre: 3, anschaffungsmonat: 1}   # genau an der Grenze
  - {anschaffungskosten: 801,  nutzungsdauer_jahre: 3, anschaffungsmonat: 1}   # knapp darueber -> AfA
  - {anschaffungskosten: 1200, nutzungsdauer_jahre: 3, anschaffungsmonat: 1}   # volles Jahr
  - {anschaffungskosten: 1200, nutzungsdauer_jahre: 3, anschaffungsmonat: 7}   # 6/12
  - {anschaffungskosten: 5000, nutzungsdauer_jahre: 5, anschaffungsmonat: 12}  # 1/12
```

**Die 800-Euro-Grenze ist eine Entscheidung, die ich dir nicht abnehme.** Der
Wortlaut des § 6 Abs. 2 sagt „800 Euro nicht übersteigen", also greift der
Sofortabzug bei *genau* 800 Euro noch. Das Raster prüft 800 und 801. Wenn die
Modelle hier abweichen, ist es ein echter Divergenzbefund und kein Rundungsartefakt.

Testfälle: kein amtliches Rechenbeispiel bekannt. Nach der Absegnung schlage ich
synthetische Fälle mit `rechenweg` vor, wie bei den fünf Regeln zuvor.

---

## 2. Neuschnitt: § 9 Abs. 1 S. 3 Nr. 5a — Übernachtungskosten

Der alte Zuschnitt hatte **fünf `wirkt_hinein` bei null unabhängigen** Norm-Teilen.
Das heißt nicht, dass das Modell schlecht formalisiert hat, sondern dass die
Signatur den Ausschnitt verfehlt: sie kannte nur `uebernachtungskosten` und
`monate_taetigkeit`, während die Norm nach 48 Monaten auf 1.000 Euro **pro Monat**
kappt und eine Neubeginn-Regel bei Unterbrechung kennt.

```yaml
- rule_id: p9_1_3_nr5a_uebernachtung
  norm: § 9 Abs. 1 S. 3 Nr. 5a EStG
  quellen:
  - typ: gesetz
    datei: sources/gesetze-im-internet/estg_p9_2026-07-10.txt
  signature:
    scope: Uebernachtungskosten
    inputs:
      uebernachtungskosten_monat: money   # statt Jahresbetrag
      monate: int                         # Monate der Auswaertstaetigkeit im VZ
      monate_bisher_am_ort: int           # fuer die 48-Monats-Grenze
    output: abziehbare_uebernachtungskosten
  geltungsbedingungen:
  - bedingung: keine_unterbrechung_mit_neubeginn
    quelle: "§ 9 Abs. 4a Satz 7 EStG (entsprechend)"
  - bedingung: erste_taetigkeitsstaette_liegt_nicht_am_ort
```

Die Parallele zu § 9 Abs. 4a ist beabsichtigt: dort trägt dieselbe Neubeginn-Regel
bereits eine Geltungsbedingung.

---

## 3. Was ich brauche

Pro Regel: **Zuschnitt so**, **Zuschnitt anders** (dann sag wie), oder **weiter
zurückstellen**.

Nach der Absegnung ist der Rest mechanisch: Manifest-Eintrag, ein Lauf pro Regel
(je rund 0,06 USD), dann Testfall-Vorschlag mit `rechenweg`.
