# Kapital-Deklaration Finalisierung — Fortschritt + Golden-Drafts (dev-2)

**Status:** Deklarations-Seite. LLM-frei. Golden-Werte hand-gerechnet aus den formalisierten Regeln
(p20_9/p20_6/p32d_1 catala_a-Snapshots). **DRAFTS — Commit erst gegen dev-1s §32d-Accessor-Kontrakt.**

## Item 1 — est_mapping-Vollständigkeit kap_*: KOMPLETT (keine Härtung nötig)
- **4 Kz 1:1-gemappt + getestet** (test_scheibe3_kapital_und_vv_1zu1_roundtrip, Zeile 190):
  kap_kapitalertraege→E0121709, kap_gewinn_aktien→E1900901, kap_verlust_aktien→E1901301,
  kap_verlust_sonstige→E1901201 — je 1:1 + exakter Round-Trip (Zeilen 196-203). Aktien-Subset-Semantik
  separat getestet (test_aktien_subset_semantik).
- **2 bewusst-null (dokumentiert, Audit-belegt):** kap_gewinn_sonstige (MODELL-MISMATCH p20_6-4-Töpfe vs
  Vordruck), kap_zusammenveranlagung (global aus Mantelbogen).
- **K2 kein_kap-Guard: existiert** (flag_check FLAG_NEGIERT kein_kap → [kap_kapitalertraege, kap_gewinn_aktien]).
- **Gültigkeit:** §20 + §32d „geltende Fassung 2026" ✓.

## Regel-Formeln (aus catala_a-Snapshots, exakt)
- **p20_9 SparerPauschbetrag:** pauschbetrag = 2000 (zusammen) / 1000 (einzeln); einkuenfte_nach_sparer_pb
  = max(0, kapitalertraege − pauschbetrag).
- **p20_6 Verlustverrechnung:** saldo_aktien = max(0, gewinn_aktien − verlust_aktien); saldo_sonstige =
  max(0, gewinn_sonstige − verlust_sonstige); verrechnete = saldo_aktien + saldo_sonstige. **Topf-Trennung:
  sonstige-Verlust reduziert NICHT den Aktien-Gewinn** (§20 Abs.6). (Verlustvortrag §10d = benannte GAP.)
- **p32d_1 KapitalAbgeltung:** abgeltung = kapitaleinkuenfte × 0.25; guenstiger_delta =
  est_regulaer_mit_kap − est_regulaer_ohne_kap; kapital_steuer = abgeltung, AUSNAHME wenn
  guenstiger_delta < abgeltung → kapital_steuer = guenstiger_delta (Günstigerprüfung § 32d Abs. 6).

## Item 2 — 4 Golden-Drafts (hand-gerechnet)

**Schlüssel-Einsicht:** Ohne Anderseinkommen greift IMMER die Günstigerprüfung (Grundtarif auf kleines
Kapital = 0 < 25%-Abgeltung) → kapital_steuer = 0. Für eine ECHTE 25%-Abgeltung (≠0) braucht es
Anderseinkommen, das den Grenzsteuersatz über 25% hebt (→ Accessor-abhängig).

**Zielwerte = dev-1s FULL-Pipeline (brutto → §9a → Accessor → steuer), Instructor-fixiert (msg 2752):**

| Draft | Sachverhalt (einzel VZ2025, BRUTTOLOHN-Ebene) | Zwischenwerte (dev-1 nachgerechnet) | festzusetzende_est | 
|---|---|---|---|
| **K1** Sparer-PB absorbiert | kap_kapitalertraege 1000€ | nach_sparer_pb=max(0,1000−1000)=0 → abgeltung 0 | **0** |
| **K2** Günstiger greift | kap_kapitalertraege 5000€ | nach_sparer_pb=4000; abgeltung=1000; grundtarif(4000)=0 < 1000 → 0 | **0** |
| **K3** Verlusttopf §20 Abs.6 | gewinn_aktien 5000 / verlust_aktien 2000 / gewinn_sonstige 1000 / verlust_sonstige 3000 | saldo_aktien=3000, saldo_sonstige=0 (Topf-Trennung!), verrechnet=3000; nach_sparer_pb=2000; abgeltung=500; Günstiger→0 | **0** |
| **K4** Reine Abgeltung | **bruttoarbeitslohn 60000** + kap_kapitalertraege 10000 | §9a → einkünfte 58770; est_ohne=13924; nach_sparer_pb=9000; abgeltung=2250; delta=est_mit−est_ohne=3613 > 2250 → kapital_steuer=2250 → 13924+2250 | **16174** |

K1–K3 = 0 (jeder testet einen anderen Zweig: Sparer-PB-Absorption, Günstiger-Vorrang, Verlusttopf-Trennung).
K4 = **16174** (dev-1s brutto-basierter Zielwert, NICHT der Layer-I-Wert 16651 der einkünfte-direkt füttert).
Diese Werte brauchen dev-1s Accessor (kap_*→steuer_kapital_gesondert); Commit erst gegen den finalen
EURO-Accessor-Kontrakt (sonst vakuöser 0-Pass für K1–K3, weil ohne Accessor die kap-Felder nicht in den
Tarif fließen).

## FINDING — Integration ist SCHON da; 2 Goldens JETZT fertig (kein Accessor nötig)

catala_gesamt hat bereits die Slots `steuer_kapital_gesondert_in` + `einkuenfte_kapitalvermoegen_in`
(engine p32a Zeile 461/477, runner 464/476). Empirisch bestätigt (catala_gesamt-Probe):
- **steuer_kapital_gesondert = 2250 → festzusetzende_est +2250 exakt** (§ 32d Abs. 3 S. 2 Addition).
- **einkuenfte_kapitalvermoegen = 9000 tariflich → +3651** (= Grenzsteuer auf 9000; 3651 > 2250 bestätigt,
  dass bei § 19-60000 die Abgeltung gewinnt — validiert K4-Hand-Rechnung).

→ Das teilt die Goldens in **2 LAYER**:
- **Layer I — Engine-Integration (MEINE Zone, JETZT fertig + grün):** Sachverhalt setzt die hand-gerechnete
  gesonderte Steuer / tarifliche Kap-Einkünfte DIREKT; testet die catala_gesamt-§32d-Integration. Braucht
  KEINEN Accessor. **2 Goldens geschrieben + golden-runner 116/116 grün:**
  - `kapital_abgeltung_2025_einzel`: § 19 60000 + steuer_kapital_gesondert 2250 → **festzusetzende_est 16651**
    (§ 32d Abs. 1/3, Abgeltung addiert; Anker „erhöht sich die tarifliche Einkommensteuer um den nach
    Absatz 1 ermittelten Betrag").
  - `kapital_guenstiger_2025_einzel`: § 19 12000 + einkuenfte_kapitalvermoegen 5000 tariflich → **902**
    (§ 32d Abs. 6 Günstiger: tariflich 902 < Abgeltung-Alt 1000; Anker „…wenn dies zu einer niedrigeren
    Einkommensteuer … führt (Günstigerprüfung)").
  - `kapital_abgeltung_2025_zusammen`: § 19 80000 + steuer_kapital_gesondert 2500 → **17118**
    (Zusammenveranlagung/Splitting + GEMEINSAMER Sparer-PB 2000, § 20 Abs. 9 S. 2; deckt den 2000-Zweig).
  → **golden-runner 117/117 grün** (3 Layer-I-Goldens: Abgeltung einzel/zusammen + Günstiger einzel).
- **Layer II — Full-Pipeline (braucht dev-1s Accessor):** Sachverhalt setzt kap_kapitalertraege etc., der
  Accessor rechnet steuer_kapital_gesondert via p32d_1. Das sind die K1–K4-Drafts oben — Commit erst gegen
  den Accessor (sonst vakuöser 0-Pass für K1–K3, weil ohne Accessor die kap-Felder nicht in den Tarif fließen).

**Empfehlung:** Layer I (2 Goldens) ist commit-fertig JETZT (echte §32d-Integration, kein Vakuum). Layer II
docke ich an sobald dev-1s Accessor-Kontrakt steht.

## Item 3 — K2 + Gültigkeit: bestätigt (oben).

## OFFENE ACCESSOR-KONTRAKT-FRAGE an dev-1 (blockiert Golden-Finalisierung)
Die Regeln p20_9 (Sparer-PB auf `kapitalertraege`) und p20_6 (Töpfe auf gewinn/verlust) sind SEPARAT.
kap_kapitalertraege ist der GESAMT-Kapitalertrag (E0121709), gewinn/verlust sind Subsets (Aktien-Topf).
**Wie assembliert der Accessor `kapitaleinkuenfte` (p32d_1-Input)?**
- (a) kapitaleinkuenfte = einkuenfte_nach_sparer_pb (Sparer-PB auf kapitalertraege), Töpfe nur Deklaration?
- (b) kapitaleinkuenfte = max(0, verrechnete_kapitaleinkuenfte − sparer_pb) (Töpfe THEN Sparer-PB)?
- Und: welcher Wert speist `einkuenfte_kapitalvermoegen` (Günstiger-Grundtarif-Pfad) in catala_gesamt?
- Einheit-Kontrakt: kapital_steuer in Cent oder Euro? (est_mapping-Naht = Cent.)
**Meine Drafts nehmen (a) für K1/K2/K4 (nur kapitalertraege) + p20_6→p20_9-Kette für K3** — bestätige/korrigiere,
dann finalisiere ich die 4 Goldens + K4s §19-Teil gegen den Accessor.
