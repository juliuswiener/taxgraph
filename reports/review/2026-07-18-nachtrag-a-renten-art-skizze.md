# Nachtrag A — `rentner_renten_art`-Enum: Konzept-Skizze (Task #11)

**Status:** Konzept-Skizze zur Instructor-Abnahme, concept-first, KEIN Code. Klärt die 4 Punkte +
den Zonen-Schnitt. Schließt die Anlage-R-Kz-GAP (rentner_jahresrente/renten_beginn_jahr). LLM-frei.

## Kernbefund: die Art-Achse ist eine STEUERLICHE Zwei-Wege-Weiche (§ 22 Nr. 1), nicht Vordruck-Logik

§ 22 Nr. 1 S. 3 Buchst. a unterscheidet ZWEI Rechenpfade — genau die muss die Art-Achse treffen:

| Rechenpfad | Norm | deckt | Besteuerung | Status |
|---|---|---|---|---|
| **aa — Basisversorgung** | § 22 Nr. 1 S. 3 a **aa** | gesetzliche RV, landw. Alterskasse, berufsständische Versorgung, private Basisrente (Rürup, § 10 Abs. 1 Nr. 2 b) | **Kohorten-%** nach Rentenbeginn-Jahr | **p22_1 EXISTIERT** |
| **bb — Ertragsanteil** | § 22 Nr. 1 S. 3 a **bb** | private Leibrenten (nicht-Basis), sonstige Verpflichtungsgründe | **Ertragsanteil-Tabelle** (nach Alter/Laufzeit) | **NICHT in p22_1 → Registry-Nachtrag (dev-1)** |

Anker: aa „aus den gesetzlichen Rentenversicherungen, der landwirtschaftlichen Alterskasse, den
berufsständischen Versorgungseinrichtungen" · bb „die nicht solche im Sinne des Doppelbuchstaben aa
sind" (beide estg_p22_2026-07-13.txt, verifiziert).

## 1. Enum-Werte (steuerlich, laientauglich gruppiert)

`rentner_renten_art` (typ enum, askable), fragetext „Um welche Art von Rente geht es?":
- `gesetzliche_rente` ┐
- `berufsstaendische_versorgung` ├─ **Rechenpfad aa** (Kohorten-%, p22_1)
- `private_basisrente` (Rürup)   ┘
- `private_leibrente` ┐
- `sonstige_leibrente` ┴─ **Rechenpfad bb** (Ertragsanteil, Registry-Nachtrag)

Laientauglich (der Rentner erkennt „gesetzliche Rente" vs. „private Rentenversicherung"), intern auf die
2 Rechenpfade + N Anlage-R-Kz abgebildet. **Offene Frage:** 5 Werte (fein) vs. 2 Werte
(basisversorgung/private_leibrente, grob)? Empfehlung: 5 erkennbare Werte, Rechenpfad-Gruppierung intern
(sonst muss der Laie „Basisversorgung" verstehen).

## 2. Kz-Verzweigung = neue est_mapping-Transform-Klasse f (1 Wert-Slot → N-Kz je Enum-Wert)

`rentner_jahresrente` / `rentner_renten_beginn_jahr` verzweigen per `rentner_renten_art` auf das
Art-spezifische Anlage-R-Kz:

| Art (Rechenpfad) | jahresrente-Kz | beginn-Kz | Anlage-R-Zeile / Sektionspfad |
|---|---|---|---|
| gesetzl/berufsst/basisrente (aa) | **E1800301** | **E1800501** | Z4/Z6, /R/Leibr_gesetzl |
| private_leibrente (bb) | **E1801601** | **E1801701** | Z13/Z14, /R/Leibr_priv |
| sonstige_leibrente (bb) | **E1803102** | **E1803202** | Z19/Z20, /R/Leibr_sonst |

Config `VERZWEIGUNG[(wert_feld, art_feld)] = {enum_wert -> Kz}`; deckt_ab-Anker je Kz-Zweig = das
Anlage-R-Zeilen-Label im jeweiligen Sektionspfad. est_mapping: Feld mit VERZWEIGUNG-Config → liest
`rentner_renten_art` aus dem Store → wählt das Ziel-Kz → `deklaration[Kz] = wert`. Fehlt die Art
(unbeantwortet) → keine Deklaration (Lücke), nie Default-Kz.

## 3. Jahr↔Datum-Brücke (deterministisch, kein Rate)

`rentner_renten_beginn_jahr` = int (Jahr); Anlage-R „Beginn der Rente" (E1800501…) = Datum-Feld. Das
**Jahr** ist die steuerlich relevante Größe (Kohorten-% keyed auf Rentenbeginn-JAHR). Bridge: int-Jahr →
Jahr-Komponente des Vordruck-Datums. Verlangt der Vordruck volle Datum-Granularität (Monat/Tag), wird
Monat/Tag NICHT erfunden → separates optionales askable ODER benannte Lücke. Empfehlung Stufe 1:
Jahr-Granularität (deckt die Besteuerung); Monat/Tag = Nachtrag nur falls die ERiC-Validierung es hart
verlangt.

## 4. Drift-Wächter-Passung

Die verzweigten Kz (E1800301/E1801601/…) sind KEINE 1:1-Kz (rentner_jahresrente hat elster_kz=null).
Damit sie nicht als Phantom gelten: im Drift-Wächter (`test_deklarations_abdeckung`) `_erlaubte_kz` +
`_transform_quellen` um die VERZWEIGUNG-Ziel-Kz + Quellfelder erweitern (analog NEGATION). Assertion 4:
VERZWEIGUNG-Ziele kollidieren nicht mit 1:1-Kz; Assertion 2: die Wert-/Art-Felder sind Transform-Quellen.
= Test-Update beim Bau (meine Zone), kein Loch.

## 5. REGISTRY-GRENZE — Cross-Zone-Nachtrag (dev-1)

Die Art-Achse berührt den **Catala-Rechenpfad**: aa (Kohorten) rechnet p22_1; bb (Ertragsanteil) ist ein
ANDERER Rechenpfad (Ertragsanteil-Tabelle nach Alter/Laufzeit), **nicht in p22_1**.

**Zonen-Schnitt (Vorschlag):**
- **Meine Zone (Deklarations-Seite):** `rentner_renten_art`-Enum (bindung) + Kz-Verzweigung (est_mapping)
  + Jahr-Datum-Bridge + Drift-Wächter. Das ist reines **Input→Kz-Mapping**, kein Rechnen — funktioniert
  für ALLE Arten.
- **dev-1-Zone (Rechenpfad, rules.yaml):** die Besteuerungs-BERECHNUNG des bb-Zweigs (Ertragsanteil) —
  **benannter Registry-Nachtrag `p22_1_ertragsanteil` (§ 22 Nr. 1 S. 3 a bb)** + Ertragsanteil-Tabelle-
  params. Ich materialisiere das NICHT.
- **Ehrlichkeits-Sperre bis dahin:** die aa-Werte (gesetzl/berufsst/basisrente) sind voll nutzbar
  (p22_1 rechnet). Die bb-Werte (private_leibrente/sonstige) sind **deklarations-fähig** (Kz-Verzweigung),
  aber die Steuer-Berechnung wartet auf den Registry-Nachtrag → bis dahin als **benannte Rechenpfad-Lücke**
  führen (NIE still mit dem Kohorten-% falsch rechnen — das wäre ein steuerlicher Fehler).

## Zur Abnahme

Entscheide: (a) Enum-Granularität 5 vs. 2; (b) Verzweigungs-Klasse f in est_mapping OK; (c) Jahr-
Granularität Stufe 1 OK; (d) Zonen-Schnitt — baue ich die Deklarations-Seite jetzt (aa voll nutzbar,
bb deklarations-fähig + Rechenpfad-Lücke), und `p22_1_ertragsanteil` geht als Cross-Zone-Order an dev-1?
Nach Abnahme: bindung_rentner.yaml (rentner_renten_art + jahresrente/beginn von GAP auf Verzweigung) +
est_mapping VERZWEIGUNG + Drift-Wächter-Update + Gate.
