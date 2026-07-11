# Charge 2 Teil 2 — Zuschnitt-Report: § 10 Vorsorgeaufwendungen (v1/v2/v4 + v3 Backlog)

Stufe A, $0. Zuschnitte 6-8/8. Vier Regeln aus § 10 (Instructor-Dekomposition msg 1189),
gemeinsame eingefrorene Quelle estg_p10_2026-07-11. Zitatanker per `grep -oF|wc -l` in `[n]`.

---

## v1 — `p10_1_2_altersvorsorge` (§ 10 Abs. 1 Nr. 2 i.V.m. Abs. 3)

```yaml
- rule_id: p10_1_2_altersvorsorge
  norm: § 10 Abs. 1 Nr. 2, Abs. 3 EStG
  quellen:
  - typ: gesetz
    datei: sources/gesetze-im-internet/estg_p10_2026-07-11.txt
    zitatanker: "Beiträge zu den gesetzlichen Rentenversicherungen"                 # [1]  Nr. 2
    auszug: "bis zu dem Höchstbeitrag zur knappschaftlichen Rentenversicherung, aufgerundet auf einen vollen Betrag in Euro"  # [beim Eintrag zaehlen]  Abs. 3
  signature:
    scope: Altersvorsorge
    inputs:
      altersvorsorgebeitraege: money    # AN-Anteil + weitere Nr.-2-Beitraege
      steuerfreier_ag_anteil: money      # steuerfreier AG-Anteil zur RV (Abs. 3, mindernd)
      hoechstbeitrag_knappschaft: money  # Parameter (§ 2-Integration/params), aufgerundet auf volle Euro
    output: abziehbare_altersvorsorge     # min(altersvorsorgebeitraege, hoechstbeitrag) - steuerfreier_ag_anteil, >= 0
  geltungsbedingungen:
  - bedingung: hoechstbeitrag_ist_parameter
    deckt_ab: "bis zu dem Höchstbeitrag zur knappschaftlichen Rentenversicherung"
    quelle: "§ 10 Abs. 3 Satz 1 EStG"
    beschreibung: "Der Höchstbetrag ist der (dynamische) Höchstbeitrag zur knappschaftlichen RV, aufgerundet auf volle Euro - kommt als Parameter, nicht hier hergeleitet."
  - bedingung: voller_abzug_100_prozent
    deckt_ab: "Beiträge zu den gesetzlichen Rentenversicherungen"
    quelle: "§ 10 Abs. 3 EStG (Auslauf der prozentualen Kürzung)"
    beschreibung: "MVP 2026: 100 % Abzug (der historische Prozentsatz-Aufwuchs ist ausgelaufen). Falls Rechtsstand einen Prozentsatz traegt, ist er Parameter."
  rundung:
  - deckt_ab: "aufgerundet auf einen vollen Betrag in Euro"
    zitatanker: "aufgerundet auf einen vollen Betrag in Euro"                        # [beim Eintrag zaehlen]
    quelle: "§ 10 Abs. 3 Satz 1 EStG"
  # OFFEN fuer dich: die exakte Abs.-3-Mechanik (Reihenfolge Kürzung/Kappung, Behandlung
  # des steuerfreien AG-Anteils) ist subtil - bitte die Formel bestaetigen/korrigieren.
```

---

## v2 — `p10_1_3_3a_kv_pv` (§ 10 Abs. 1 Nr. 3 UND Nr. 3a GEMEINSAM mit Abs. 4)

EIN Mechanismus (Instructor msg 1189): Basis-KV/PV (Nr. 3) IMMER voll abziehbar, auch
über dem Höchstbetrag; Nr.-3a-Raum nur, wenn Basis unter dem Höchstbetrag (2.800/1.900).

```yaml
- rule_id: p10_1_3_3a_kv_pv
  norm: § 10 Abs. 1 Nr. 3 und 3a, Abs. 4 EStG
  quellen:
  - typ: gesetz
    datei: sources/gesetze-im-internet/estg_p10_2026-07-11.txt
    zitatanker: "Beiträge zu Kranken- und Pflegeversicherungen, soweit diese nicht nach Nummer 3 zu berücksichtigen sind"  # [1]  Nr. 3a
    auszug: "2 800 Euro abgezogen werden"                                            # [1]  Abs. 4
  signature:
    scope: KrankenPflegeVorsorge
    inputs:
      basis_kv_pv: money                 # Basis-Kranken + Pflege (Nr. 3) - immer voll
      weitere_vorsorgeaufwendungen: money # Nr. 3a (weitere KV/PV, Haftpflicht etc.)
      mit_anspruch_auf_zuschuss: bool      # true -> Höchstbetrag 1.900, sonst 2.800
    output: abziehbare_kv_pv_vorsorge      # basis + min(weitere, max(0, Höchstbetrag - basis))
  geltungsbedingungen:
  - bedingung: basis_kv_pv_ist_basisabsicherung
    deckt_ab: "Beiträge zu Kranken- und Pflegeversicherungen, soweit diese nicht nach Nummer 3 zu berücksichtigen sind"
    quelle: "§ 10 Abs. 1 Nr. 3 EStG"
    beschreibung: "basis_kv_pv = Basisabsicherung (Nr. 3, sozialhilfegleiches Niveau). Input-Semantik."
  rundung: []
  raster:
  - {basis_kv_pv: 4000, weitere_vorsorgeaufwendungen: 500, mit_anspruch_auf_zuschuss: false}  # Basis>2800 -> 4000+0=4000
  - {basis_kv_pv: 2000, weitere_vorsorgeaufwendungen: 1500, mit_anspruch_auf_zuschuss: false}  # 2000+min(1500,800)=2800
  - {basis_kv_pv: 1500, weitere_vorsorgeaufwendungen: 1000, mit_anspruch_auf_zuschuss: true}   # 1500+min(1000,400)=1900
```

---

## v4 — `p10_1_4_kirchensteuer` (§ 10 Abs. 1 Nr. 4)

```yaml
- rule_id: p10_1_4_kirchensteuer
  norm: § 10 Abs. 1 Nr. 4 EStG
  quellen:
  - typ: gesetz
    datei: sources/gesetze-im-internet/estg_p10_2026-07-11.txt
    zitatanker: "gezahlte Kirchensteuer"                                              # [1]
    auszug: "gezahlte Kirchensteuer"
  signature:
    scope: Kirchensteuerabzug
    inputs:
      gezahlte_kirchensteuer: money
      erstattete_kirchensteuer: money
    output: abziehbare_kirchensteuer      # gezahlte - erstattete, >= 0
  geltungsbedingungen:
  - bedingung: keine_zuschlagsteuer_kappung
    deckt_ab: "gezahlte Kirchensteuer"
    quelle: "§ 10 Abs. 1 Nr. 4 EStG"
    beschreibung: "Die als Zuschlag zur Kapitalertragsteuer (Abgeltung) gezahlte KiSt ist ausgenommen (MVP-AN ohne KapErtr). Kein Cap sonst."
  rundung: []
```

---

## v3 — § 10 Abs. 4a (Günstigerprüfung): BACKLOG-EMPFEHLUNG

`status: zuschnitt_offen` / Backlog. Abs. 4a ist ein Rechtsstand-2004/2019-Vergleich
(Mindestabzug nach altem Recht). Für den MVP-Arbeitnehmerfall 2026 praktisch immer
irrelevant (die aktuellen Höchstbeträge sind günstiger). Empfehlung: NICHT in Charge 2
formalisieren, als Backlog notieren, bei Bedarf spaeter.

---

## Frage an dich

Reihenfolge v1 → v2 → v4, v3 Backlog. Kernfragen: (a) v1 Abs.-3-Formel (Kürzung
AG-Anteil vor/nach Kappung, 100 %-Annahme) — bitte bestaetigen. (b) v2-Mechanik
`basis + min(weitere, max(0, Höchstbetrag − basis))` normgetreu? (c) hoechstbeitrag_
knappschaft als Input/Parameter ok, oder soll v1 ihn aus params ziehen?
