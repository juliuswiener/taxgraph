# Kapital §32d — Stufe-A-Zuschnitt (dev-1 → Instructor)

**Vor Bau.** Recon der Regeln p20_6/p20_9/p32d_1 + dev-2s Golden-Drafts (K1-K4) + catala_gesamt-Naht.
Ziel: Accessor-Kontrakt für dev-2s Golden-Finalisierung + Scheiben-Plan. **Kein Bau bis Abnahme.**

## 1. p32d_1-Kontrakt (aufgelöst — Instructor-Relay bestätigt)
Die Regel rechnet die Günstigerprüfung NICHT selbst (hinweis: "est_regulaer_mit/ohne_kap … hier NICHT
selbst rechnen"). Inputs: `kapitaleinkuenfte`, `est_regulaer_mit_kap`, `est_regulaer_ohne_kap`.
Rechenkern: `kapital_steuer = min(0.25 × kapitaleinkuenfte, est_regulaer_mit_kap − est_regulaer_ohne_kap)`.
→ **Ich bilde die zwei est-Werte extern über catala_gesamt (zweimal)** — dockt genau an die kombiniert-Infra:
- `est_regulaer_ohne_kap` = catala_gesamt(einkuenfte_ns=§19, einkuenfte_kapitalvermoegen=0)
- `est_regulaer_mit_kap`  = catala_gesamt(einkuenfte_ns=§19, einkuenfte_kapitalvermoegen=kapitaleinkuenfte)

## 2. Integration in den §2-Bescheid (elegante Identität)
`festzusetzende_est = est_regulaer_ohne_kap + kapital_steuer`, weil:
- Günstiger (delta ≤ abgeltung): est_ohne + delta = est_mit (Kapital im Grundtarif, zvE).
- Abgeltung (abgeltung < delta): est_ohne + abgeltung (gesondert, §32d Abs.6 hinzugerechnet).
catala_gesamt hat BEIDE Naht-Inputs schon: `einkuenfte_kapitalvermoegen` (Günstiger-Pfad, zvE) UND
`steuer_kapital_gesondert` (Abgeltung-Pfad, Abs.6-Hinzurechnung). MVP-Accessor nutzt die Identität direkt
(est_ohne + kapital_steuer), kein dritter gesamt-Call nötig.

**Verifiziert an dev-2s Zielwerten (VZ2025 einzel):**
- K4 (§19 60000 + kap 10000): einkuenfte_ns=58770; est_ohne=**13924**; est_mit(kap 9000)=17537;
  delta=**3613** (dev-2 schätzte ≈3780); abgeltung=0.25×9000=**2250**; kapital_steuer=min=2250 →
  **festzusetzende_est = 13924 + 2250 = 16174**. (Abgeltung gewinnt: Grenzsteuer 41 % > 25 %.)
- K2 (kap 5000, kein §19): est_ohne=0, est_mit(4000)=0, delta=0 → kapital_steuer=0 → **est 0** ✓.
- K1/K3: est 0 (Sparer-PB-Absorption / Günstiger-Vorrang bei 0-Grundtarif) ✓.

## 3. Accessor-Mechanik — Weg A (kein pkg-Modul)
pkg hat KEIN kapital/p32d-Modul → Python-Transkription + Konsistenz-Gate runner↔registry (wie §21):
- `catala_sparer_pb(kapitalertraege, zusammen)` = max(0, kapitalertraege − (2000 zusammen / 1000 einzel)) [p20_9]
- `catala_kapital_verrechnung(gewinn/verlust_aktien/sonstige)` = Σ max(0, Topf) [p20_6, Topf-Trennung]
- `catala_kapital_steuer(kapitaleinkuenfte, est_mit, est_ohne)` = min(0.25×kap, est_mit−est_ohne) [p32d_1]
Konsistenz-Gate gegen die test_seeds jeder Regel (rot bei Divergenz), Sätze/Beträge aus params.
**Param-Frage:** Sparer-PB (1000/2000) + 25%-Satz — gibt es dafür params/<vz>/? Falls nicht, Anker VORAB zu dir
(analog sonderausgabenpauschbetrag). Ich prüfe das vor dem Bau; melde Fund.

## 4. Assembly `kapitaleinkuenfte` — die subtile Frage (dev-2 Q1) + OFFENE Vordruck-Semantik
Legaler Ablauf: §20 Abs.6 Verlustverrechnung (Töpfe, per-Topf-Floor) ZUERST, dann §20 Abs.9 Sparer-PB.
→ **kapitaleinkuenfte = max(0, verrechnete − sparer_pb)** (Option b, Töpfe THEN Sparer-PB). Deckt K3.
Für K1/K2/K4 (nur kap_kapitalertraege, keine Töpfe) fällt es auf p20_9(kapitalertraege) zusammen (= Option a).
**Beide Draft-Wege sind derselbe Baum, wenn** `verrechnete_vor_sparer_pb` so definiert ist:

> **verrechnete_vor_sparer_pb = kap_kapitalertraege + Σ max(0, Topf)** (additiv) — ODER —
> **= Σ max(0, Topf)**, mit kap_kapitalertraege als GESAMT der die Töpfe schon enthält (subset).

Das ist eine **Vordruck-Semantik-Frage**: Ist E0121709 (kap_kapitalertraege) inklusive der Aktien-
Veräußerungsgewinne (E1900901) oder additiv daneben? In dev-2s Drafts co-okkurieren sie NIE (K1/2/4 nur
E0121709, K3 nur Töpfe) → beide Modelle geben dieselben Goldens. Für den MVP unkritisch, aber der Accessor
muss sich für EINEN Weg entscheiden. **Meine Empfehlung: additiv** (E0121709 = übrige Erträge Zins/Dividende,
Töpfe = Veräußerungsgewinne, getrennte Vordruck-Zeilen) — falls die Vordruck-Semantik das stützt. Deine
Entscheidung (du hast die Vordruck-/Kz-Autorität). Co-Okkurrenz sonst = benannter GAP.

## 5. Einheit (dev-2 Q3)
Accessoren rechnen EURO (wie catala_vermietung_einkuenfte / catala_einkuenfte_nichtselbststaendig); die
Haut/est_mapping-Naht konvertiert an der Store-Grenze zu Cent. **Accessor-Kontrakt an dev-2: EURO;
kapital_steuer geht als steuer_kapital_gesondert (EURO) in catala_gesamt bzw. direkt in die Identität.**

## 6. Scheiben-Plan
Wert liegt im KOMBINIERTEN Fall (Kapital + §19) — reiner Kapital-MVP ist fast immer est 0 (Günstiger).
→ Erweiterung der bestehenden Scheibe (analog kombiniert §19+§21): kap_* im Kegel (kap_kapitalertraege +
Aktien/sonstige-Töpfe + kap_zusammenveranlagung), `kein_kap=false` (bestätigte Anwesenheit), flag_check-Guard
(kein_kap=true + kap>0 → flag_konsistenz_offen, existiert schon). slot_fn: sparer_pb+verrechnung → kapital-
einkuenfte, dann est_ohne/est_mit → kapital_steuer → est_ohne+kapital_steuer.
**Offene Scheiben-Frage:** eigene `kap_gesamt`-Scheibe ODER in vv_gesamt integrieren (dann rechnet EIN Ring
§19+§21+§20)? Ich neige zu vv_gesamt-Integration (der End-Zustand EIN-catala_gesamt-Ring, den wir bei
kombiniert benannt haben) — aber das macht die Scheibe breit. Deine Richtung.

## Fragen an Instructor (Abnahme-Gate)
1. Assembly-Semantik §4: additiv (Empfehlung) oder subset? (MVP-unkritisch, aber Accessor braucht Festlegung.)
2. Sparer-PB/25%-Satz als params/<vz> mit Anker — falls noch nicht da, Anker-Freigabe VORAB.
3. Scheibe: eigene kap_gesamt oder vv_gesamt-Integration (EIN-Ring)?
4. K4-Zielwert 16174 (est_ohne 13924 + 2250) bestätigt als Golden? Dann melde ich dev-2 den Kontrakt.
