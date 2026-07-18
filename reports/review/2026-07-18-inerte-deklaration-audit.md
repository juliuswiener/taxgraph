# Inerte-Deklaration-Audit — dev-2, 2026-07-18 (read-only, KEINE Materialisierung)

**Zweck:** echter Rechen-Ring-Backlog für Julius' UI-vs-weiter-Fork. Bindungs-zentrische Methode (wie §35a
gefunden wurde): für jede in produkt/bindung/*.yaml referenzierte regel_id → prüfen ob die Engine den Output
tatsächlich BERECHNET (nicht nur Snapshot, nicht nur Bindung).

**Live-Signal (2 Kriterien, ODER):** (a) golden/runner.py-Accessor `catala_*` berechnet den Output für Golden-
Fälle; ODER (b) Modul ist im assemblierten Engine-Paket `oracle/gettsim/_catala/pkg/` exportiert + verdrahtet.
**Inert = bound, aber KEINES von beiden** (Wert wird roh gefüttert / ist nur Deklarations-Feld).

## KATEGORIE 1 — INERTE FORMEL, verified_bedingt-Snapshot = BILLIGE PROMOTION (kein Re-Run, wie §35a war)
| regel_id | Bindung | Snapshot (module) | Live? | Aufwand |
|---|---|---|---|---|
| p10_1_2_altersvorsorge | n_vor_gwg | ✓ verified_bedingt (Altersvorsorge) | NEIN | Materialisieren + Accessor (§10 Vorsorge-Höchstbetrag/Günstiger) |
| p10_1_7_berufsausbildung | sonder_agb_35a | ✓ verified_bedingt (Berufsausbildungsaufwendungen) | NEIN | Materialisieren + Accessor |
| p16_4_freibetrag | rentner | ✓ verified_bedingt (BetriebsFreibetrag) | NEIN | Materialisieren + Accessor (§16 Abs.4 Veräußerungs-FB) |
| p21_2_verbilligte_vermietung_wk | kap_vv_familie | ✓ verified_bedingt (VerbilligteVermietungWk) | NEIN | Materialisieren + Accessor (§21 Abs.2 66/50%-WK-Kürzung) |
| p24a_altersentlastungsbetrag | rentner | ✓ verified_bedingt (Altersentlastungsbetrag) | ⚠ SLOT live, FORMEL NEIN | Materialisieren (s. Konflikt-Flag) |
| p24b_entlastungsbetrag | kap_vv_familie | ✓ verified_bedingt (Entlastungsbetrag) | NEIN | Materialisieren + Accessor (§24b) |
| p6_2_gwg_sofortabzug | n_vor_gwg | ✓ verified_bedingt (GwgSofortabzug) | ⚠ FELD live, FORMEL NEIN | Materialisieren (s. Konflikt-Flag) |
| p31_familienleistungsausgleich | kap_vv_familie:LUECKE | ✓ verified_bedingt (Familienleistungsausgleich) | NEIN | Materialisieren (§31 Günstigerprüfung Kindergeld↔FB) |

→ **8 billige Promotionen** (verified_bedingt, byte-gleich materialisierbar wie §35a/§10b/agB/KiSt — je ~1 dev-2-
Materialisierung + 1 dev-1-Accessor, kein Pipeline-Re-Run).

## KATEGORIE 2 — ECHTER FORMALISIERUNGS-BEDARF (kein Snapshot = teuer, Pipeline-Lauf nötig)
| regel_id | Bindung | Snapshot | Live? | Aufwand |
|---|---|---|---|---|
| p9_1_3_nr6_7_arbeitsmittel_afa | n_vor_gwg | KEINER | NEIN (nur K2-Guard, e2e am_guard fail-closed) | Voll-Formalisierung (§9 Abs.1 Nr.6/7 AfA) + Snapshot + Materialisierung |

→ **1 echte Lücke** (Arbeitsmittel-AfA: heute nur guard-gesperrt, kein Rechenmodell).

## KATEGORIE 3 — SCHON MATERIALISIERT (charge29), Verdrahtung dev-1 #7/#8 WIP
p10_1_4_kirchensteuer (Kirchensteuerabzug), p10b_spenden (SpendenAbzug), p33_1_2_agb_abzug (AgbAbzug),
p35a_2_3_haushaltsnahe (Haushaltsnahe) — Module engine-exportiert, Accessor/Scheibe in Arbeit.

## KATEGORIE 4 — LIVE (Engine berechnet, kein Backlog)
p09_entfernungspauschale (catala_entfernungspauschale), p20_6_verlustverrechnung (catala_kapital_verrechnung),
p20_9_sparer_pauschbetrag (catala_sparer_pb), p21_vermietung_einkuenfte (catala_vermietung_einkuenfte),
p22_1_leibrente_besteuerungsanteil (catala_renten_einkuenfte), p33b_* ×3 (catala_behinderten/pflege/
hinterbliebenen_pb), p34_fuenftel_ao_est (catala_fuenftel), p32_6_kinderfreibetraege (module Kinderfreibetraege
+ freibetraege_kinder wired), p9_1_3_nr5_doppelte_haushaltsfuehrung + p9_4a_verpflegungsmehraufwand (in §19-WK,
e2e dhf_ring/verpflegung_ring grün), p2_festzusetzung_einzel/zusammen + p2_einkunftsarten (Integrations-/Pseudo-Scopes).

## ⚠ KONFLIKT-FLAGS mit dem naiven Scan ("p24a/GWG sind live")
Dein naiver Scan nannte p24a + GWG live. Meine bindungs-zentrische Methode findet eine FEINERE Wahrheit:
- **p24a altersentlastungsbetrag**: der p32a-INPUT-SLOT ist live (Wert fließt in FestzusetzendeEstGesamt), ABER
  in `golden/cases/gesamt_2026_einzel_abzuege_ermaessigung.yaml` wird `altersentlastungsbetrag: 2000` als ROHER
  sachverhalt-Input gefüttert — die §24a-FORMEL (Kohorte × %, gedeckelt; module Altersentlastungsbetrag) ist
  NICHT materialisiert/verdrahtet. Slot-live ≠ Formel-live. = billige Promotion.
- **p6_2 GWG**: GWG-Felder fließen in der n_vor_gwg-Deklarations-Scheibe, aber die §6-Abs.2-Sofortabzug-Schwelle
  (800/1000-Grenze, Sofort vs AfA; module GwgSofortabzug) ist NICHT im Engine-Paket → Formel inert.
→ Kein Widerspruch, nur Präzisierung: „Slot/Feld erreichbar" ≠ „Formel berechnet". Beide sind billige Promotionen.

## FORK-INPUT-ZUSAMMENFASSUNG für Julius
- **8 billige Promotionen** (verified Snapshot vorhanden): §10-Vorsorge, §10-Berufsausbildung, §16-Freibetrag,
  §21-verbilligt-WK, §24a-Altersentlastung, §24b-Entlastung, §6-GWG, §31-Familienleistungsausgleich.
- **1 echte Formalisierung** (kein Snapshot): §9-Arbeitsmittel-AfA.
- **4 in Verdrahtung** (charge29, dev-1 WIP): §35a, §10b, §33-agB, §10-KiSt.
- Alle übrigen bound regel_ids sind live.
Methode + Belege reproduzierbar (grep bindungen / _catala/pkg / runner-Accessoren / Snapshot-queue_status).
