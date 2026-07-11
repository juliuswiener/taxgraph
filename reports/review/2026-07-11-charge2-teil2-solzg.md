# Charge 2 Teil 2 — Zuschnitt-Report: Solidaritätszuschlag (SolzG § 3 + § 4)

Stufe A, $0. Erste Teil-2-Regel (einfachste, Instructor-Reihenfolge). Multi-Source
(beide eingefrorenen SolzG-Files). Zitatanker per `grep -oF|wc -l` (Vorkommen) in `[n]`.

```yaml
- rule_id: solzg_solidaritaetszuschlag
  norm: § 3, § 4 SolzG 1995
  quellen:
  - typ: gesetz
    label: "§ 3 SolzG 1995 (Bemessungsgrundlage, Freigrenze)"
    datei: sources/gesetze-im-internet/solzg_1995_p3_2026-07-11.txt
    zitatanker: "unter Berücksichtigung von Freibeträgen nach § 32 Abs. 6 des Einkommensteuergesetzes"  # [1]  (Abs. 2, Bemessungsgrundlage)
    auszug: "40 700 Euro"                                    # [1]  Freigrenze Splitting (Abs. 3 Nr. 1)
  - typ: gesetz
    label: "§ 4 SolzG 1995 (Zuschlagssatz 5,5 %, Milderungszone)"
    datei: sources/gesetze-im-internet/solzg_1995_p4_2026-07-11.txt
    zitatanker: "Der Solidaritätszuschlag beträgt 5,5 Prozent der Bemessungsgrundlage"  # [1]
    auszug: "nicht mehr als 11,9 Prozent des Unterschiedsbetrages"  # [1]  Milderungszone
  signature:
    scope: Solidaritaetszuschlag
    inputs:
      bemessungsgrundlage: money   # ESt, die unter Beruecksichtigung der Kinderfreibetraege
                                   # (§ 32 Abs. 6) festzusetzen waere (§ 3 Abs. 2) - Input von upstream
      splitting: bool              # § 32a Abs. 5/6 -> Freigrenze 40.700 statt 20.350 (§ 3 Abs. 3)
    output: solidaritaetszuschlag
  geltungsbedingungen:
  - bedingung: veranlagung_zur_einkommensteuer
    deckt_ab: "nur zu erheben, wenn die Bemessungsgrundlage"
    quelle: "§ 3 Abs. 3 SolzG 1995"
    beschreibung: "MVP: Veranlagungsfall. Lohnsteuerabzug/Jahresausgleich/KapESt (Abs. 2a, 4, 4a, 5) nicht modelliert."
  - bedingung: bemessungsgrundlage_ist_est_mit_kinderfreibetraegen
    deckt_ab: "unter Berücksichtigung von Freibeträgen nach § 32 Abs. 6 des Einkommensteuergesetzes"
    quelle: "§ 3 Abs. 2 SolzG 1995"
    beschreibung: "Input = ESt abweichend von § 2 Abs. 6, mit § 32 Abs. 6-Kinderfreibetraegen (SolZ-eigene Bemessung). Kommt von der § 2-Integration, nicht hier gerechnet."
  - bedingung: keine_abgeltungsteuer_nach_32d
    deckt_ab: "vermindert um die Einkommensteuer nach § 32d Absatz 3 und 4"
    quelle: "§ 3 Abs. 3 / § 4 Satz 2 SolzG 1995"
    beschreibung: "MVP-AN ohne Kapitalertraege: die § 32d-Abs.-3/4-Sonderbehandlung (SolZ immer, ausserhalb Freigrenze/Milderung) entfaellt."
  raster:
  - {bemessungsgrundlage: 18000, splitting: false}   # < Freigrenze -> 0
  - {bemessungsgrundlage: 20350, splitting: false}   # genau Freigrenze -> 0 (uebersteigt = strikt groesser)
  - {bemessungsgrundlage: 25000, splitting: false}   # Milderungszone (11,9%-Kappung greift)
  - {bemessungsgrundlage: 45000, splitting: false}   # > Milderungszone -> volle 5,5%
  - {bemessungsgrundlage: 40700, splitting: true}    # Splitting-Freigrenze -> 0
  - {bemessungsgrundlage: 60000, splitting: true}    # Splitting, volle 5,5%
```

## Abgrenzung

- SIGNATUR: `bemessungsgrundlage` + `splitting` variieren den Betrag (Freigrenze,
  5,5 %, Milderungszone). Beides gehoert in die Signatur.
- BEDINGUNGEN: reiner Veranlagungsfall; Bemessungsgrundlage-Semantik (SolZ-eigene
  ESt mit Kinderfreibetraegen); keine Abgeltungsteuer. Alle binaer, MVP-Standardfall.
- KEIN eigener Tarif-Rechenfluss: die Bemessungsgrundlage (SolZ-ESt) ist Input; der
  Tarif steckt in p32a / der § 2-Integration.

## Rechenfluss (fuer Seeds nach Freigabe)

Freigrenze F = 40.700 (splitting) sonst 20.350.
- BMG <= F: SolZ = 0.
- BMG > F: SolZ = min(5,5 % x BMG, 11,9 % x (BMG - F)). Bruchteile eines Cents ausser Ansatz.
Milderungszone endet, wo 5,5 % BMG = 11,9 % (BMG-F), d.h. BMG = 1,859375 x F (~37.846 bzw. ~75.692).

## Seeds

Kein amtliches Rechenbeispiel bekannt -> synthetisch mit rechenweg nach deiner
Abgrenzungs-Freigabe (Randfaelle: Freigrenze exakt, Milderungszone, Milderungszonen-
Ende, Splitting). Bis dahin "fehlt bewusst".

## Nachtrag: Auflagen eingearbeitet (Instructor msg 1192, "SO mit Auflagen")

1. **bemessungsgrundlage_ist_est_mit_kinderfreibetraegen** bleibt BEDINGUNG (nicht
   konv): das Label traegt die § 3-Abs.-2-Fiktion nicht aus sich heraus (weicht bei
   Kindern von der festgesetzten ESt ab). deckt_ab auf die verifizierte Passage.
2. **splitting** zusaetzlich als Bedingung `splitting_ist_veranlagungsergebnis`
   (gleiche Ziel-ID wie p33 -> Praezedenz-Ratsche), deckt_ab Splitting-Passage.
3. **Cent-Regel als deklarierte Rundung**: `rundung:` mit zitatanker "Bruchteile
   eines Cents bleiben außer Ansatz" [§ 4 Satz 3] -> rundungs_lint zaehlt sie als
   angeordnet, nicht als stille Annahme.
4. **Boundary-Raster+Seeds**: BMG=20.350 -> 0 (nur wenn UEBERSTEIGT); 20.351 -> 0,11
   (11,9 % x 1 = 0,119, Cent abgeschnitten -> 0,11); ein Punkt in der Zone; Zonen-
   Ende-Umgebung (~37.838); ein Punkt klar darueber (5,5 % voll); Splitting analog.
5. **Rechenwege je Seed mit exakter Cent-Behandlung** (Abschneiden, nicht kaufmaennisch).

Damit ist der Zuschnitt final; Manifest-Eintrag + Seeds folgen vor Stufe B (die
selbst erst nach Julius' Morgen-Entscheid zur Formalisierer-Besetzung laeuft).
