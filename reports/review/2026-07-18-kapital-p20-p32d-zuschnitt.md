# Kapital §20/§32d Declaration — Stufe-A-Zuschnitt (Recon, KEIN Bau)

**Status:** Recon zur Instructor-/Julius-Entscheid-Information, concept-first. LLM-frei.
**Kern-Befund: Kapital ist KLEIN auf der Declaration-Seite — Felder + Abgeltungsteuer-Rechnung +
Günstigerprüfung existieren bereits; die Lücke ist (wie V+V) nur die Accessor-Verdrahtung (dev-1).**

## 1. Deklarations-Felder: EXISTIEREN (Scheibe 3, bindung_kap_vv_familie)
`kap_kapitalertraege` (Kz **E0121709**, p20_9), `kap_gewinn_aktien` (E1900901), `kap_verlust_aktien`
(E1901301), `kap_verlust_sonstige` (E1901201) — je 1:1. `kap_gewinn_sonstige` (deriviert, null) +
`kap_zusammenveranlagung` (global) = benannte GAP. **Kein neues Feld nötig für den Kern.**

## 2. Abgeltungsteuer-Struktur = eigener Tarif-Zweig, ABER schon modelliert + dockt an
§ 20/§ 32d: Kapital wird mit **25 % FLAT** besteuert, SEPARAT vom Grundtarif (nicht im zvE). Der Engine-
Slot existiert: `catala_gesamt` nimmt **`steuer_kapital_gesondert`** (die gesonderte Kapitalsteuer wird
zur festzusetzenden ESt addiert) UND `einkuenfte_kapitalvermoegen` (falls Günstigerprüfung → Grundtarif).
Beide Pfade sind da. Die **Rechnung existiert als Regel**: `p32d_1_abgeltung` (scope KapitalAbgeltung,
inputs `{kapitaleinkuenfte, est_regulaer_mit_kap, est_regulaer_ohne_kap}`, output `kapital_steuer`).

## 3. Günstigerprüfung § 32d Abs. 6: SCHON IN p32d_1_abgeltung modelliert
Die Regel-Inputs `est_regulaer_mit_kap` / `est_regulaer_ohne_kap` SIND der Günstiger-Vergleich (25 %-
Abgeltung vs. Kapital im Grundtarif, das Günstigere). geltungsbedingung `guenstigerpruefung_beantragt`.
Norm der Regel: „§ 32d Abs. 1, 6 EStG (25-%-Abgeltung + Guenstigerpruefung)". **Kein separater
Nachtrag** — die Günstigerprüfung ist Teil der bestehenden Regel.

## 4. Sparer-Pauschbetrag § 20 Abs. 9 (1.000 €): EXISTIERT
`p20_9_sparer_pauschbetrag` (kap_kapitalertraege bindet ihn) + `p20_6_verlustverrechnung`
(Aktien-/Sonstige-Töpfe). Beides formalisiert.

## 5. Gültigkeit
estg_p20 + estg_p32d beide **„geltende Fassung 2026"** — gültig, kein neuer Freeze.

## Was FEHLT (die einzige echte Arbeit)
- **§ 32d-Accessor (dev-1-Zone):** wie bei V+V ist die Regel NICHT im golden-Runner verdrahtet (kein
  p32d-Call, nur der Input-Slot `steuer_kapital_gesondert`). dev-1 baut den Accessor (p32d_1_abgeltung →
  kapital_steuer → steuer_kapital_gesondert), inkl. Günstiger-Vergleich (est mit/ohne Kap). Engine-Call
  (KapitalAbgeltung compiled?) ODER Python-Accessor + Konsistenz-Gate gegen die Regel-test_seeds.
- **K2 kein_kap-Guard: EXISTIERT schon** (`flag_check`: kein_kap=true + kap_kapitalertraege>0 →
  Widerspruch). Keine neue Guard-Arbeit.
- **Kapital-Scheibe (dev-1-Haut):** ein SCHEIBEN-Eintrag (kap_* + veranlagung + flags) analog vv_gesamt.
- **MEINE Zone (minimal):** ggf. ein Kapital-Golden (Abgeltung 25 % + Günstiger-Fall) — kann ich gegen
  die p32d-Rechnung schreiben, sobald dev-1 den Accessor hat; die Bindungen/Kz/K2-Guard sind komplett.

## STRUKTUR-GRÖSSE (für Julius' Entscheid)
**KLEIN–MITTEL.** Kapital ist KEIN großer neuer Ring-Neubau: die Declaration-Felder, die Abgeltungsteuer-
+ Günstigerprüfungs-REGEL (p32d_1_abgeltung), der Sparer-Pauschbetrag und der K2-Guard existieren alle.
Der eigene 25-%-Tarif-Zweig **dockt an** (steuer_kapital_gesondert-Slot), wie V+V andockt. Die einzige
substanzielle Arbeit ist dev-1s §32d-Accessor-Verdrahtung (+ Kapital-Scheibe-Haut). Meine Declaration-
Seite ist praktisch fertig (Felder + K2-Guard) — bräuchte nur ein Golden + ggf. GAP-Schärfung.

**Empfehlung:** wenn Julius „weitere Einkunftsart" freigibt, ist Kapital ein günstiger nächster Schritt
(viel schon da). Der einzige Aufwand liegt bei dev-1 (Accessor+Haut); meine Seite ist inkrementell.
