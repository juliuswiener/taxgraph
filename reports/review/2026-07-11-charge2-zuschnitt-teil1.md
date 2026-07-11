# Charge 2 — Zuschnitts-Report Teil 1 (Neuschnitte), KORRIGIERT — zur Prüfung

Stufe A, $0. Überarbeitet nach Instructor-Review (msg 1161): 3 echte Fehler behoben,
davon 2 Rechtsfehler in Regel 2. **Alle Treffer-Zahlen per Skript gezählt**
(`grep -oF <phrase> <datei> | wc -l` = Vorkommen; die frozen Files sind lange
Einzelzeilen, `grep -c` unterzählt — genau der Fehler in v1). Zählung steht je Anker
in `[n]`.

Neu eingefroren: `estg_p7_2026-07-11.txt` (§ 7 EStG, 11.314 Zeichen, sha256
7f4745a6…, Passagen „Absetzung für Abnutzung" + „Zwölftel" verifiziert) — die
zeitanteilige AfA-Zwölftelung steht in § 7 Abs. 1 S. 4, nicht in § 9/§ 6.

---

## 1. `p9_1_3_nr6_7_arbeitsmittel_afa` — ersetzt nr6 + nr7 (Multi-Source, DREI Quellen)

```yaml
- rule_id: p9_1_3_nr6_7_arbeitsmittel_afa
  norm: § 9 Abs. 1 S. 3 Nr. 6 und Nr. 7 EStG
  quellen:
  - typ: gesetz
    label: "§ 9 Abs. 1 S. 3 Nr. 6 und Nr. 7 EStG"
    datei: sources/gesetze-im-internet/estg_p9_2026-07-10.txt
    zitatanker: "Nummer 7 bleibt unberührt"                 # [1]
    auszug: "Aufwendungen für Arbeitsmittel"                # [1]  (Nr. 6)
  - typ: gesetz
    label: "§ 6 Abs. 2 EStG (geringwertige Wirtschaftsgüter, 800-€-Grenze)"
    datei: sources/gesetze-im-internet/estg_p6_2026-07-10.txt
    zitatanker: "800 Euro nicht übersteigen"                # [1]
    auszug: "an deren Stelle tretende Wert für das einzelne Wirtschaftsgut 800 Euro nicht übersteigen"  # [1]  (ersetzt den 3-Treffer-Auszug aus v1)
  - typ: gesetz
    label: "§ 7 Abs. 1 S. 4 EStG (zeitanteilige AfA im Anschaffungsjahr)"
    datei: sources/gesetze-im-internet/estg_p7_2026-07-11.txt
    zitatanker: "ein Zwölftel für jeden vollen Monat"       # [1]  (Abs. 1 S. 4)
  signature:
    scope: ArbeitsmittelUndAfa
    inputs:
      anschaffungskosten: money          # massgebliche AK i.S.d. § 6 Abs. 2 S. 1 (s. Bedingung)
      nutzungsdauer_jahre: int
      anschaffungsmonat: int             # 1..12
      jahre_seit_anschaffung: int        # NEU: 0 = Anschaffungsjahr, >0 = Folgejahr
    output: abziehbar
  geltungsbedingungen:
  - bedingung: gwg_sofortabzug_gewaehlt
    deckt_ab: "in voller Höhe als Betriebsausgaben abgezogen werden"  # [2 - beim Eintrag auf eindeutiges Fenster verlängern] (§ 6 Abs. 2 „können"-Wahlrecht)
    quelle: "§ 6 Abs. 2 Satz 1 EStG"
    beschreibung: "GWG-Sofortabzug ist ein Wahlrecht (können); der Scope nimmt an, dass es ausgeuebt wird."
  - bedingung: anschaffungskosten_sind_massgebliche_ak
    deckt_ab: "an deren Stelle tretende Wert für das einzelne Wirtschaftsgut 800 Euro nicht übersteigen"  # [1]
    quelle: "§ 6 Abs. 2 Satz 1 EStG (Verweis § 9b Abs. 1)"
    beschreibung: "Input = massgebliche AK i.S.d. § 6 Abs. 2 S. 1. Netto/Brutto NICHT still festgelegt: § 6 Abs. 2 verweist auf § 9b Abs. 1, der Vorsteuer nur rausrechnet, SOWEIT bei der USt abziehbar (beim AN i.d.R. nicht). Grenzfall -> Morgen-Paket Julius (BMF/LStR)."
  raster:
  - {anschaffungskosten: 500,  nutzungsdauer_jahre: 3, anschaffungsmonat: 1,  jahre_seit_anschaffung: 0}   # GWG
  - {anschaffungskosten: 800,  nutzungsdauer_jahre: 3, anschaffungsmonat: 1,  jahre_seit_anschaffung: 0}   # Grenze (<=)
  - {anschaffungskosten: 801,  nutzungsdauer_jahre: 3, anschaffungsmonat: 1,  jahre_seit_anschaffung: 0}   # knapp drüber -> AfA volles Jahr
  - {anschaffungskosten: 1200, nutzungsdauer_jahre: 3, anschaffungsmonat: 7,  jahre_seit_anschaffung: 0}   # Anschaffungsjahr 6/12
  - {anschaffungskosten: 1200, nutzungsdauer_jahre: 3, anschaffungsmonat: 7,  jahre_seit_anschaffung: 1}   # Folgejahr -> volle Jahres-AfA
  - {anschaffungskosten: 1200, nutzungsdauer_jahre: 3, anschaffungsmonat: 7,  jahre_seit_anschaffung: 3}   # Letztjahr -> Rest (Gegenstück zur 6/12)
  - {anschaffungskosten: 5000, nutzungsdauer_jahre: 5, anschaffungsmonat: 12, jahre_seit_anschaffung: 0}   # 1/12
```

**Korrekturen ggü. v1:** (a) §6-Auszug auf eindeutiges Fenster [1] statt „einer
selbständigen Nutzung fähig" [3]. (b) dritte Quelle § 7 Abs. 1 S. 4 für die
Zwölftelung (sonst quellenlose stille Annahme). (c) `jahre_seit_anschaffung` in die
Signatur — Folgejahre sind für den MVP-Arbeitnehmerfall material (2024er Laptop hat
2026 laufende AfA); Raster um Folge-/Letztjahr ergänzt. (d) GWG-Wahlrecht als
Bedingung `gwg_sofortabzug_gewaehlt`. (e) Netto/Brutto nicht still festgelegt →
Bedingung + Grenzfall-Doku (Morgen-Paket).

---

## 2. `p9_1_3_nr5a_uebernachtung` — Neuschnitt (Monatsbetrag)

```yaml
- rule_id: p9_1_3_nr5a_uebernachtung
  norm: § 9 Abs. 1 S. 3 Nr. 5a EStG
  quellen:
  - typ: gesetz
    label: "§ 9 Abs. 1 S. 3 Nr. 5a EStG"
    datei: sources/gesetze-im-internet/estg_p9_2026-07-10.txt
    zitatanker: "bis zur Höhe des Betrags nach Nummer 5"   # [1]  Nr. 5a Satz 4 (PRIMÄR-Anker, Kappung)
    auszug: "notwendige Mehraufwendungen eines Arbeitnehmers für beruflich veranlasste Übernachtungen"  # [1]  Nr. 5a Satz 1
    # VERWEISZIEL (Manifest-Kommentar): der 1.000-€/Monat-Betrag steht in Nr. 5
    #   ("1 000 Euro im Monat" [1], dHf), Nr. 5a Satz 4 verweist nur darauf.
  signature:
    scope: Uebernachtungskosten
    inputs:
      uebernachtungskosten_monat: money   # bei alleiniger Nutzung (s. Bedingung)
      monate: int                         # Monate der Auswärtstätigkeit im VZ
      monate_bisher_am_ort: int           # für die 48-Monats-Grenze
    output: abziehbare_uebernachtungskosten
  geltungsbedingungen:
  - bedingung: keine_unterbrechung_mit_neubeginn
    deckt_ab: "wenn die Unterbrechung mindestens sechs Monate dauert"   # [1]  Nr. 5a Satz 5
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 5 EStG"
    beschreibung: "EIGENE Neubeginn-Regel des Nr. 5a: sechs Monate (NICHT die vier Wochen aus § 9 Abs. 4a S. 7). Der Scope kennt die Unterbrechungshistorie nicht."
  - bedingung: unterkunft_im_inland
    deckt_ab: "2 000 Euro im Monat bei einer Unterkunft im Ausland"     # [1]  (Nr. 5, via Verweis mitgezogen)
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5 Satz 4 EStG (via Verweis Nr. 5a Satz 4)"
    beschreibung: "MVP-Linie Inland. Der Verweis auf Nr. 5 zieht die 2.000-€-Auslandsgrenze mit; Auslandsfall zurückgestellt."
  - bedingung: kosten_bei_alleiniger_nutzung
    deckt_ab: "die bei alleiniger Nutzung durch den Arbeitnehmer angefallen wären"  # [1]  Nr. 5a Satz 3
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 3 EStG"
    beschreibung: "Bei gemeinsamer Nutzung sind nur die Kosten bei alleiniger Nutzung ansetzbar. Input-Semantik von uebernachtungskosten_monat."
  - bedingung: erste_taetigkeitsstaette_liegt_nicht_am_ort
    deckt_ab: "an einer Tätigkeitsstätte, die nicht erste Tätigkeitsstätte ist"     # [beim Eintrag zählen]
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 1 EStG"
  raster:
  - {uebernachtungskosten_monat: 800,  monate: 12, monate_bisher_am_ort: 10}   # unter Kappung, <48
  - {uebernachtungskosten_monat: 1400, monate: 12, monate_bisher_am_ort: 10}   # >1000 aber <48 -> keine Kappung
  - {uebernachtungskosten_monat: 1400, monate: 12, monate_bisher_am_ort: 48}   # genau 48 -> noch keine Kappung
  - {uebernachtungskosten_monat: 1400, monate: 12, monate_bisher_am_ort: 49}   # ab 49 -> Kappung auf 1000/Monat
```

**Korrekturen ggü. v1:** (a) RECHTSFEHLER Neubeginn behoben — Nr. 5a Satz 5 (sechs
Monate), nicht § 9 Abs. 4a S. 7 (vier Wochen). (b) Primär-Anker auf Nr. 5a Satz 4
(„bis zur Höhe des Betrags nach Nummer 5" [1]) — der 1.000er steht in Nr. 5, kommt
per Verweis (Verweisziel im Kommentar deklariert; sonst dHf-Lektion rückwärts).
(c) Bedingung `unterkunft_im_inland` (Verweis auf Nr. 5 zieht die 2.000-€-Ausland-
grenze mit). (d) Bedingung `kosten_bei_alleiniger_nutzung` (Nr. 5a Satz 3).
(e) 48er-Grenze: Kappung ab Monat 49; Raster prüft 48 (aus) vs 49 (an).

**Seeds beide Regeln:** kein amtliches Beispiel → synthetisch mit `rechenweg` nach
deiner Abgrenzungs-Freigabe (bis dahin „fehlt bewusst", Clerk-Gate fällt korrekt).

---

## Was ich von dir brauche

Pro Regel: **so** / **anders (wie)** / **zurückstellen**. Nach Freigabe: `deckt_ab`-
Fenster final auf Eindeutigkeit gepinnt, Seeds mit rechenweg, Manifest-Eintrag →
Stufe B (~0,12 USD beide). Teil-2-Quellen friere ich parallel ein ($0).
