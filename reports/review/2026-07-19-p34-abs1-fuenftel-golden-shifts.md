# §34 Abs.1 Fünftelregelung auf §16-vg — Golden-Shift-Provenance (dev-1, 2026-07-19)

**Kontext:** §34 Abs.1 EStG ist MANDATORY (S.1 „Sind in dem zvE außerordentliche Einkünfte enthalten, so IST die …
ESt nach den Sätzen 2 bis 4 zu berechnen" — kein Antrag; nur Abs.3-56%-Satz ist „auf Antrag"). §16-Betriebs­ver­äußerungs­gewinn
ist außerordentlich per §34 Abs.2 Nr.1. → Der Ring wandte den §16-vg (netto nach §16 Abs.4-FB) bisher VOLL PROGRESSIV an
= systematischer **Over-tax** jeder §16-vg-Fall. Die §34-Naht (`catala_gesamt_zve` + `tarif_modifiziert`/`catala_fuenftel`
in `_festzusetzende` gesamt + im rentner-Ring) korrigiert das → **10 committed Goldens schiften nach unten**.

**Formel (§34 Abs.1 S.2/S.3, catala_fuenftel):** ao = netto_vg (NUR die Veräußerungsgewinn-Komponente; laufender §15/§18-Gewinn
bleibt progressiv). zvE_rest = zvE − ao. S.2 (zvE_rest ≥ 0): est = Tarif(zvE_rest) + 5×[Tarif(zvE_rest + ao/5) − Tarif(zvE_rest)].
S.3 (zvE_rest < 0 ∧ zvE > 0): est = 5×Tarif(zvE/5). Guard `zve2>0` (sonst catala_fuenftel-ValueError). Kein Under-tax
(Fünftel ≤ progressiv IMMER, Tarif monoton). Werte = ECHTER Produktions-Ring (catala_gesamt via HTTP-slot_fn), nicht Neben-Rechnung.

## GESHIFTETE COMMITTED GOLDENS (alt → neu, in Cent)
| Fall | einkuenfte_gewinn (netto_vg) | Branch | alt (progressiv) | neu (Fünftel) | Hand-Mathe |
|---|---|---|---|---|---|
| vg 100000 (gesamt+rentner p16_4) | 55000 | S.3 | 1249500 | **0** | zvE 54964; verbleibendes 54964−55000=−36<0 → 5×Tarif(54964//5=10992); 10992 < GfB 12096 → Tarif 0 → 5×0=0 |
| vg 150000 (gesamt+rentner p16_4) | 119000 | S.3 | 3905200 | **1304000** | zvE 118964; verbleibendes −36 → 5×Tarif(118964//5=23792)=5×260800/... =13040 |
| vg 181000 (gesamt+rentner p16_4) | 181000 | S.3 | 6509200 | **3065000** | zvE 180964; verbleibendes −36 → 5×Tarif(180964//5=36192)=30650 |
| laufender 30000 + vg 100000 (gesamt gvl + rentner rgva) | 85000 (ao 55000) | S.2 | 2477200 | **2097800** | zvE 84964; verbleibendes 84964−55000=29964>0 → Tarif(29964)+5×[Tarif(40964)−Tarif(29964)]; laufender 30000 PROGRESSIV (nur vg geglättet) |
| rente 20000 + vg 100000 (rentner rrv) | sonstige 16598 + 55000 (ao 55000) | S.2 | 1914400 | **1486100** | zvE 71562; verbleibendes 16562>0 → S.2; Rente progressiv |
| EÜR 50000 + vg 100000 (rentner reuv) | 105000 (ao 55000) | S.2 | 3317200 | **3124800** | zvE 104964; verbleibendes 49964>0 → S.2; EÜR-Gewinn progressiv |
| vg 40000 (gesamt+rentner p16_4) | 0 (FB 45000 > vg) | — | 0 | **0** (unverändert) | netto_vg 0 → Guard `netto_vg>0` → kein Fünftel |

**⚠ vg 100000 → 0 ist KEIN Under-tax-Bug:** §34 Abs.1 S.3 + Grundfreibetrag machen moderate Einmal-Veräußerungsgewinne
steuerfrei (1/5 des zvE unter GfB → 5×0). Der 150000-Fall (gleicher S.3-Pfad, nonzero 1304000) beweist: S.3 zeroed NICHT blanket.

## NEUE BOUNDARY-GOLDENS (raster-Vollständigkeit, Auflage D)
| Test | Fall | Branch | neu | Zweck |
|---|---|---|---|---|
| test_gesamt_fuenftel_zve_null_skip (f34z) | vg 200000 + §10d-Verlustvortrag 200000 | Guard | **0** | zve2≤0 → Naht überspringt (kein catala_fuenftel-ValueError) |
| test_gesamt_fuenftel_p35_interaktion (f34p) | laufender gewerbe 50000 + vg 200000 + GewSt-MB 3000/Hebesatz 400% | S.2 | **7964800** | §35-Deckel-3 nutzt POST-Fünftel-tarifliche (geminderte tarifliche Steuer §35 Abs.1 S.4) |
| test_gesamt_fuenftel_per_kind (f34k) | laufender gewerbe 80000 + vg 200000 + 2 Kinder | S.2 per-§31-Zweig | **10667200** | Freibetrag-Zweig gewinnt → per-Zweig-Fünftel (zve2 je Zweig verschieden); non-vakuum (< no-Kind-Wert) |

Raster deckt: S.3→0 (100k) · S.3→nonzero (150k/181k) · S.2-mit-laufender (gvl 2097800) · netto_vg=0-Guard (40k) ·
zve2≤0-Skip (f34z) · §35-Interaktion (f34p) · per-§31-Kind (f34k). Vollständig.

## SCOPE-LIMITS (MVP, dokumentiert)
- **§34 Abs.1 S.4 (§6b/§6c-Opt-out)**: „Die Sätze 1 bis 3 gelten nicht … wenn der Steuerpflichtige auf diese Einkünfte
  ganz oder teilweise §6b oder §6c anwendet." → NICHT modelliert (kein §6b/§6c-Feld im Ring); da §6b eine seltene
  Rücklagen-Wahl ist und ihre Abwesenheit die Fünftel-Anwendung nur bestätigt → automatische Fünftel-Anwendung ok.
- **§34 Abs.3 (ermäßigter 56%-Durchschnittssatz, 55+/einmal/Antrag, Wahlrecht statt Abs.1)**: Stufe-2 (Snapshot p34_3
  verified_bedingt, kein Accessor; braucht 55+/einmal/Antrags-Felder). dev-2 hat den Deklarations-Recon (…-p34-abs3-…md).
- ao = NUR netto_vg (laufender §15/§18-Gewinn progressiv; §35-Zähler unberührt). §34 Abs.2 Nr.2-4 (Entschädigungen/
  mehrjährige Vergütungen) nicht im Ring (kein Feld).
