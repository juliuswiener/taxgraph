# Rentner-Scheibe §22/§33b — Stufe-A-Zuschnitt (dev-1 → Instructor)

**Vor Bau.** Recon der §22/§33b/§24a-Regeln + rentner-Bindung + catala_gesamt-Slots. Parallel zu dev-2s
§22-Ertragsanteil-Tabellen-Zuschnitt. **Kein Bau bis Abnahme + dev-2-Signatur.**

## Fakten (Recon)
- pkg hat KEIN §22/§33b-Modul → **Weg A** (Python-Transkription + Konsistenz-Gate, wie §21/§32d).
- catala_gesamt-Slots vorhanden: `einkuenfte_sonstige` (§22), `aussergewoehnliche_belastungen` (§33b),
  `altersentlastungsbetrag` (§24a) — alle bereits verdrahtet.
- rentner-Bindung führt: rentner_jahresrente, _renten_beginn_jahr, _renten_art, _alter_64_erfuellt,
  §33b: _grad_der_behinderung, _hilflos_blind_taubblind, _hinterbliebenenbezuege, _pflegegrad,
  _gepflegter_hilflos; Partner: _grad_der_behinderung_partner, _hilflos_blind_taubblind_partner.
  (rentner_veraeusserungs* = dev-2s §16, NICHT diese Scheibe.)
- renten_art-Enum: gesetzliche_rente, berufsstaendische_versorgung, private_basisrente, private_leibrente,
  sonstige_leibrente.
- Regeln: p22_1_leibrente_besteuerungsanteil(jahresrente, besteuerungsanteil_prozent) → stpfl_rentenanteil
  = prozent/100 × jahresrente. p33b_behinderten_pauschbetrag(gdb int, hilflos bool) → Pauschbetrag
  (GdB-Staffel 20→384/45→860/100→2840, hilflos→7400). p33b_pflege(pflegegrad, hilflos) →; p33b_hinterbliebenen
  (bool 370). p24a_altersentlastungsbetrag(arbeitslohn, andere, prozentsatz, hoechstbetrag).

## Vorgeschlagene Architektur
### Q1 — Scheibe: NEUE `rentner_gesamt` (Empfehlung) statt Integration in `gesamt`
Der Rentner-Fall hat ~10 spezifische Felder (renten_art/beginn/§33b/pflege/hinterbliebenen). In `gesamt`
integriert müsste JEDER Angestellte sie als bestätigte-Null durchklicken (Kegel-Bloat, schlechte UX). →
**eigene Scheibe rentner_gesamt** (Instructor-Wortlaut), teilt aber die est-Rechenlogik mit `gesamt`
(beide catala_gesamt). Ich faktorisiere die gemeinsame slot_fn-Kernrechnung, kein zweiter Rechenpfad.
Felder: renten_art/beginn/jahresrente + §33b-Block + Partner-Behinderung + veranlagung + Abwesenheits-Flags.

### Q2 — §22-Accessor (Weg A)
`catala_renten_einkuenfte(jahresrente, anteil_prozent)` = p22_1-Transkription − WK-PB §9a S.3 (102) →
`einkuenfte_sonstige`. Beispiel: gesetzl. Rente 12000 @ 50 % Kohorte = 6000 − 102 = **5898**. Der
`anteil_prozent` kommt aus **dev-2s Tabelle** (Besteuerungsanteil je Kohorte-Beginnjahr für gesetzlich/
Basis; Ertragsanteil je Alter für private Leibrente) — das ist die Naht-Frage an dev-2 (s.u.).

### Q3 — §33b-Integration
Behinderten- + Pflege- + Hinterbliebenen-Pauschbetrag (drei Weg-A-Accessoren, Konsistenz-Gate gegen die
p33b_*-test_seeds) → Summe in `aussergewoehnliche_belastungen`. catala_gesamt zieht sie im zvE ab.

### Q4 — partner_check aktiviert sich
rentner_gesamt exponiert rentner_*_partner → der (schon universell verdrahtete) partner_check-Guard feuert
LIVE: Partner-Behinderungsfeld gesetzt + veranlagung≠zusammen → partner_konsistenz_offen. Kein neues Wiring,
nur die Felder in der Scheibe. K2-Guard damit end-to-end scharf.

## OFFENE INSTRUCTOR-ENTSCHEIDE
1. **Scheibe:** eigene rentner_gesamt (Empfehlung) vs Integration in `gesamt`?
2. **renten_art-Scope MVP:** nur gesetzlich/Basis (Besteuerungsanteil-Kohorte) — private Leibrente
   (Ertragsanteil-nach-Alter) als Nachtrag? Oder beide Zweige sofort (hängt an dev-2s Tabelle)?
3. **§24a Altersentlastungsbetrag:** greift NICHT für Leibrenten (§24a S.2). Reiner-Rentner-MVP → §24a=0
   (kein Slot-Input)? Oder Rentner-MIT-Arbeitslohn/anderen-Einkünften (dann §24a auf die)?
4. **§33b-Pauschbetrag-Werte** (384/860/…/7400, Pflege-Staffel, Hinterbliebenen 370): als params/<vz> mit
   §33b-Anker (Anker-Disziplin, wie §10c/§20) — oder Weg-A-Transkription der in-Regel-Staffel? Empfehlung:
   params (Anker vorab, ich prüfe estg_p33b-Freeze). WK-PB Renten 102 (§9a S.1 Nr.3) ebenso als param.

## NAHT-FRAGEN AN DEV-2 (Accessor-Signatur)
- Deine Anteil-Tabelle: Signatur `anteil_prozent(renten_art, renten_beginn_jahr [, alter])`? Getrennte
  Tabellen Besteuerungsanteil (Kohorte-Jahr) vs Ertragsanteil (Alter)? Einheit Prozentwert (50.0 = 50 %)?
- Liefert die Tabelle den Prozentsatz, und mein Accessor macht p22_1 + WK-PB? (So plane ich — bestätige.)
- §33b/WK-PB-Params: baust du die (mit §33b/§9a-Anker) oder ich? (Ich kann, Anker vorab zu Instructor.)

## Nächster Schritt
Instructor-Abnahme (Q1-Q4) + dev-2 Tabellen-Signatur → dann Bau: §22/§33b-Accessoren + Konsistenz-Gate +
rentner_gesamt-Scheibe + Goldens (gesetzl. Rente Basisfall, §33b-Abzug, partner_check-Live-Fall). Freeze zu dir.

---
## BUILD-SPEC (nach Abnahme 2026-07-18 — festgezurrt, wartet nur dev-2s Param-Werte + 2 Felder)

**Entscheide (Instructor):** Q1 eigene rentner_gesamt-Scheibe (est-Kern via catala_gesamt geteilt). Q2 aa+bb.
Q3 §24a=0 (Renten-only). Q4 §33b/WK-PB als params/<vz> — dev-2 baut, ich lese. K2: aa-Rentenfreibetrag-Fixierung.

**catala_renten_einkuenfte(s) — 4 Zweige (EURO → einkuenfte_sonstige):**
- bb (private_leibrente/sonstige_leibrente): jahresrente × ertragsanteil%(alter_bei_rentenbeginn)/100 − 102.
- aa Jahr 1 (renten_beginn_jahr == VZ): jahresrente × besteuerungsanteil%(renten_beginn_jahr)/100 − 102.
- aa Folgejahr MIT rentner_rentenfreibetrag: (jahresrente − rentenfreibetrag) − 102.
- aa Folgejahr OHNE rentenfreibetrag: FAIL-CLOSED → grund `rentenfreibetrag_fixierung_offen` (K2: %×erhöhte
  Rente unterbesteuert; ab Jahr 2 ist der Freibetrag EUR-fix). WK-PB 102 nie unter 0 (max(0, …)).

**§33b-Accessoren (3, Konsistenz-Gate gegen p33b-seeds, Werte aus dev-2-params):**
- catala_behinderten_pb(gdb, hilflos): hilflos → 7400; gdb<20 → 0; sonst tabelle[(gdb//10)*10] (Tier-Floor).
- catala_pflege_pb(pflegegrad, hilflos): hilflos → 1800 (Vorrang); sonst PG2→600/PG3→1100/PG4-5→1800/sonst 0.
- catala_hinterbliebenen_pb(bool): True → 370, sonst 0.
- Summe → aussergewoehnliche_belastungen (catala_gesamt-Slot). Pflege+behinderten NEBENEINANDER (beide abziehbar).

**rentner_gesamt-Kegel (Feldliste):** rentner_renten_art, _renten_beginn_jahr, _jahresrente,
_alter_bei_rentenbeginn (NEU dev-2), _rentenfreibetrag (NEU dev-2), _grad_der_behinderung, _hilflos_blind_taubblind,
_pflegegrad, _gepflegter_hilflos, _hinterbliebenenbezuege, _grad_der_behinderung_partner (Partner),
_hilflos_blind_taubblind_partner (Partner), veranlagung, kein_gewinn/kein_kap/kein_vuv (Abwesenheit der
NICHT-modellierten Arten). **kein_sonstige NICHT im Sperr-Guard** (s.u.).

**⚠ GUARD-VERFEINERUNG (Fund, K2):** der gesamt_guard sperrt aktuell `kein_sonstige==False`. Für rentner_gesamt
IST die Rente §22-sonstige → kein_sonstige=False ist KORREKT (Rente vorhanden), darf NICHT sperren. Lösung: der
Sperr-Guard wird scheibe-abhängig — cfg-Feld `fremd_arten` = die Abwesenheits-Flags, die diese Scheibe erzwingt
(gesamt: kein_gewinn/kein_sonstige; rentner_gesamt: kein_gewinn/kein_kap/kein_vuv). flag_check (kein_sonstige=true
+ jahresrente>0 → Widerspruch) bleibt aktiv (Konsistenz). Kapital-Semantik-Guard nur bei gesamt (kein kap-Feld in rentner).

**Goldens (Layer-II, ich, brutto-Ebene):** gesetzl Rente 12000@50% Jahr1 → einkuenfte_sonstige 5898 → est(zvE); §33b
GdB50 → aussergew_belastungen 1140; partner_check-Live (Partner-GdB + einzel → partner_konsistenz_offen); aa-Folgejahr-
ohne-Freibetrag → rentenfreibetrag_fixierung_offen. dev-2 baut §22-Tabellen-Layer-I.

**Blockiert auf:** dev-2 (a) rente_besteuerungsanteil_p22 (aa, existiert) + rente_ertragsanteil_p22 (bb, neu), (b)
params/<vz> §33b-Pauschbeträge + WK-PB 102 (Anker vorab an Instructor), (c) 2 Bindungsfelder alter_bei_rentenbeginn +
rentner_rentenfreibetrag. Sobald da: Accessoren + Scheibe + Guard-Verfeinerung + Goldens, Freeze.
