# VZ-Golden-Ketten F1 (Einzel-EÜR) + F2 (PersGes) 2024/2025 (taxgraph-dev-2, 2026-07-15)

Instructor-Auftrag: komplette Erwartungswert-Ketten F1/F2 für VZ 2024 + 2025 hand-rechnen,
trianguliert, als Referenz für dev-1s M5-Golden-Bau. Read-only, meine Zone (reports/).
Ketten repliziert aus den handgeschriebenen Integrations-Goldens
`rules/estg/integration/familie1_euer_unternehmer.catala_en` + `familie2_persges_
kommanditist.catala_en` (nur READ). tarif + Soli-Freigrenze aus M3-params
(GETTSIM-verifiziert); EÜR-/§35-/§15a-Mechanik VZ-stabil.

## TRIANGULATION (Falsch-Grün-Härte)
Meine Ketten-Replik reproduziert JEDE frozen assertion der bestehenden 2026-Goldens exakt:
F1a tarif 13747 / anr 8000 / fest 5747 / SolZ 0 ✓ · F1b tarif 30864 / fest 22864 /
SolZ 299,16 ✓ · F1c anr 3000 / fest 10747 ✓ · F2a tarif/fest 13747 ✓ · F2b ausgl 10000 /
eink_gewinn 58770 / tarif 13747 ✓. Damit ist die 2024/2025-Extrapolation (identischer Code,
nur tarif/soli-params driften) belegt. Tarif-Closed-Form = BMF-Corpus-Replik; Soli cent-genau.

## F1 — EuerUnternehmerKette (§4 III → Anlage G → §32a → §35-Anrechnung → SolZ)
Kette: gewinn = BE−BA · zvE = gewinn−SA · tarifl.ESt = tarif(zvE,VZ) · §35-Anrechnung =
min(4×Messbetrag; tarifl.ESt×gewinn/Σpos; gezahlte GewSt) · fest = tarifl.ESt−Anrechnung ·
SolZ = solzg(fest, Freigrenze[VZ]).

### F1a Basisfall (BE 100.000, BA 41.230, Messbetrag 2.000, gez. GewSt 8.400, Σpos 58.770, SA 36)
| VZ | gewinn | zvE | tarifl.ESt | §35-Anr | fest.ESt | SolZ |
|---|---|---|---|---|---|---|
| 2024 | 58.770 | 58.734 | 14.148 | 8.000 | 6.148 | 0 |
| 2025 | 58.770 | 58.734 | 13.924 | 8.000 | 5.924 | 0 |
| 2026 | 58.770 | 58.734 | 13.747 | 8.000 | 5.747 | 0 (Pin ✓) |

### F1b Soli-über-Freigrenze (BE 150.000, BA 49.964, Messbetrag 2.000, gez. GewSt 8.400, Σpos 100.036, SA 36)
| VZ | gewinn | zvE | tarifl.ESt | §35-Anr | fest.ESt | SolZ |
|---|---|---|---|---|---|---|
| 2024 | 100.036 | 100.000 | 31.363 | 8.000 | 23.363 | 622,72 |
| 2025 | 100.036 | 100.000 | 31.088 | 8.000 | 23.088 | 373,42 |
| 2026 | 100.036 | 100.000 | 30.864 | 8.000 | 22.864 | 299,16 (Pin ✓) |
SolZ-Rechenweg 2024: fest 23.363 > FG 18.130 → min(5,5 %×23.363=1.284,96; 11,9 %×(23.363−18.130)=622,72) → 622,72.

### F1c GewSt-Zahlung bindet Anrechnung (wie F1a, aber gez. GewSt 3.000)
| VZ | tarifl.ESt | §35-Anr | fest.ESt | SolZ |
|---|---|---|---|---|
| 2024 | 14.148 | 3.000 | 11.148 | 0 |
| 2025 | 13.924 | 3.000 | 10.924 | 0 |
| 2026 | 13.747 | 3.000 | 10.747 | 0 (Pin ✓) |

## F2 — KommanditistKette (§15 I Nr.2 + §15a → Anlage G → §32a → SolZ)
Kette: mitunt = gewinnanteil+3 SonderVG · ausgl = min(verlustanteil; max(kapitalkonto,0)) ·
eink_gewinn = mitunt−ausgl · zvE = eink_gewinn−SA · tarifl.ESt = tarif(zvE,VZ) · fest =
tarifl.ESt (keine §35-Anrechnung) · SolZ = solzg(fest, Freigrenze[VZ]).

### F2a Reiner-Gewinn-Kommanditist (gewinnanteil 40.000, VG-Tätigkeit 18.770, kein Verlust, SA 36)
| VZ | mitunt | ausgl | eink_gewinn | zvE | tarifl.=fest.ESt | SolZ |
|---|---|---|---|---|---|---|
| 2024 | 58.770 | 0 | 58.770 | 58.734 | 14.148 | 0 |
| 2025 | 58.770 | 0 | 58.770 | 58.734 | 13.924 | 0 |
| 2026 | 58.770 | 0 | 58.770 | 58.734 | 13.747 | 0 (Pin ✓) |

### F2b §15a-Begrenzung greift (gewinnanteil 0, verlustanteil 30.000, VG-Tätigkeit 68.770, kapitalkonto 10.000, SA 36)
| VZ | mitunt | ausgl | eink_gewinn | zvE | tarifl.=fest.ESt | SolZ |
|---|---|---|---|---|---|---|
| 2024 | 68.770 | 10.000 | 58.770 | 58.734 | 14.148 | 0 |
| 2025 | 68.770 | 10.000 | 58.770 | 58.734 | 13.924 | 0 |
| 2026 | 68.770 | 10.000 | 58.770 | 58.734 | 13.747 | 0 (Pin ✓) |
§15a: Rest 20.000 verrechenbar (§15a II, vorgetragen, NICHT in der Kette). ausgl/eink_gewinn VZ-stabil.

### F2c Wächter kapitalkonto=0 (gewinnanteil 1.000, verlustanteil 30.000, kapitalkonto 0, SA 36)
| VZ | ausgl | eink_gewinn | zvE | tarifl.=fest.ESt | SolZ |
|---|---|---|---|---|---|
| 2024 | 0 | 1.000 | 964 | 0 | 0 |
| 2025 | 0 | 1.000 | 964 | 0 | 0 |
| 2026 | 0 | 1.000 | 964 | 0 | 0 |
KK=0 → ganzer Verlust nicht ausgleichsfähig; zvE 964 < GFB alle VZ → ESt 0.

## Was driftet, was nicht (M5-Hinweis für dev-1)
- **Driftet je VZ:** nur tarifl.ESt (§32a-params) und SolZ (Freigrenze-params). Alle
  Zwischengrößen (gewinn, §35-Anrechnung, mitunt, ausgl, eink_gewinn, zvE) sind VZ-stabil —
  die EÜR-/§35-/§15a-Mechanik hat keine driftende Konstante.
- **Golden-Bau:** dev-1 kann die bestehenden 2026-Testfälle 1:1 auf VZ2024/VZ2025 klonen
  (nur `veranlagungszeitraum` + Erwartungswerte tauschen); Zwischenwert-Assertions
  (gewinn/ausgl/anr) bleiben identisch. Diese Tabellen sind die Ziel-Erwartungswerte.
- Compute-Skript (deterministisch, $0): scratchpad/e2e_chains.py — repliziert beide Scopes,
  Kontroll-Assertions gegen 2026-Pins bestehen.
