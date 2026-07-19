# GewSt-Front Deklarations-Recon (§ 35 EStG-Konvergenz) — Read-only-Proposal dev-2, 2026-07-19

Parallel zu dev-1s Ring-Recon. Frage: welche Deklarations-Felder braucht die GewSt-§35-Front, damit der
§ 15-Gewerbe-Gewinn (neu via §§ 13-18-EÜR) die § 35-Anrechnung bekommt (statt over-tax)? Read-only, KEIN Write.

## Befund: GewSt-Rechnung EXISTIERT (runner), aber Deklarations-GAP TOTAL

`golden/runner.py` hat die volle GewSt-Kette — ABER alle Slots sind runner-INTERN (`s.get(...)`, Golden-Case-
Inputs), KEINER gebunden (grep bindung/*.yaml = 0 Treffer):
- `_gewst_messbetrag_cent` (Z.643): Gewerbeertrag = `gewinn_gewerbebetrieb` + § 8-Hinzurechnung − § 9-Kürzung
  − § 10a-Fehlbetrag; auf volle 100 €; − Freibetrag 24.500 (natürl. Person/PersG); × Messzahl 3,5 %.
- `_gewst_p35_anrechnung_cent` (Z.660): § 35 EStG = min(4× Messbetrag, tatsächl. GewSt = mb × `gewst_hebesatz`/100).
- `_gewst_hinzurechnung_p8` (§ 8): entgelte_schulden + renten + stille + miet_beweglich/5 + miet_unbeweglich/2 + rechte/4.
- `_gewst_kuerzung_p9` (§ 9): einheitswert·1,2 % (o. grundsteuer) + gewinnanteile_mitunternehmer + schachteldividenden.

## Deklarations-Felder-Proposal (was gebunden werden muss)

| Feld | Zweck | Bind-Art | Pflicht |
|---|---|---|---|
| **gewst_hebesatz** | Gemeinde-Hebesatz % (z. B. 400) — steuert § 35 min(4×mb, mb×hebesatz) | USER-Feld (int, %) | JA (ohne = keine § 35-Anrechnung) |
| **gewinn_gewerbebetrieb** | Gewerbeertrag-Basis | ⚠ KONVERGENZ: sollte aus einkuenfte_gewinn ABGELEITET werden wenn gewinn_betriebsart=gewerbe (NICHT separates User-Feld) — sonst Doppel-Eingabe + Divergenz | abgeleitet |
| gewst_entgelte_schulden / renten / stille / miet_beweglich / miet_unbeweglich / rechte | § 8-Hinzurechnungen (6) | USER-Felder cent, optional | optional (absent→0) |
| gewst_einheitswert (o. gewst_grundsteuer) / gewinnanteile_mitunternehmer / schachteldividenden | § 9-Kürzungen (4) | USER-Felder cent, optional | optional |
| fehlbetrag_bestand | § 10a Gewerbeverlust-Vortrag | USER-Feld cent, optional | optional |
| (Trigger) gewinn_betriebsart=gewerbe | GewSt NUR bei Gewerbe (nicht § 18/§ 13) | reuse bestehendes Enum | — |

## Snapshot-Promote-Status (die 3 GewSt-Regeln)

| Snapshot | queue | faithful | Promote-ready |
|---|---|---|---|
| p7_gewerbeertrag (Gewerbeertrag §7+§8−§9) | verified_bedingt | **TRUE** | **JA** (byte-ready, wie p16_4/p6_2-Muster) |
| p11_steuermessbetrag (§ 11 Messbetrag) | verified_bedingt | **FALSE** | NEIN — abweichungen=[] (KEIN Rechen-Defekt) → norm-teil/scope-Ursache, NÄHER promotable als Boundary-Defekt (vgl. p6_2a-auflösung); braucht Adjudikation |
| p35_1_gewst_anrechnung (§ 35 EStG) | verified_bedingt | **FALSE** | NEIN — idem (abweichungen=[]) |

## Fazit + Empfehlung

⚠ **KERN-BLOCKER**: die § 35-Anrechnung (p35_1) + der Messbetrag (p11) — genau die STEUER-relevanten Stücke —
sind faithful=FALSE → die GewSt-Front-KONVERGENZ (Gewerbe-Gewinn → § 35-Ermäßigung im gesamt-Ring) ist NICHT
promotbar ohne p11/p35-Re-Formalisierung. ABER: abweichungen=[] (kein Boundary-Rechen-Defekt wie Sammelposten-
zuführung) → wahrscheinlich norm-teil-Split/scope-gap → mit Adjudikation näher promotable. p7_gewerbeertrag ist
promote-ready.

**Front-Aufwand HOCH** (memory-bestätigt): (1) volle Bind-Scheibe (~12 Felder, meiste optional) + gewst_hebesatz
User-Feld; (2) KONVERGENZ-Wiring gewinn_gewerbebetrieb ← einkuenfte_gewinn(gewerbe); (3) p11+p35-Adjudikation/Re-
Formalisierung (Backlog-Vorbedingung); (4) Fold catala_gewst → steuerermaessigungen im gesamt-Ring (dev-1-Zone).
**Reihenfolge-Empfehlung**: erst p11/p35-Adjudikation (promotbar? sonst Python-Interim) KLÄREN — das ist der
Front-Gatekeeper; ohne § 35-Anrechnung ist die Bind-Scheibe nur Deklaration ohne Steuer-Wirkung (over-tax bleibt).
