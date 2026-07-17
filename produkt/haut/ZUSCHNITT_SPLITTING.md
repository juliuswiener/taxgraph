# Stufe-A-Zuschnitt — Front 2: Splitting-Berechnung (zusammen-Ring)

**Auftrag:** Instructor — Recon + Zuschnitt VOR Bau (kein Blind-Bau). Zusammenveranlagung (§ 26b)
im festzusetzende_est-Ring. dev-2 baut die Person-B-Deklaration (committet 3ffb2a1): feld_ids
`bruttoarbeitslohn_partner`, `vor_gesamtbeitraege_partner` (3 Summanden), `person_b_idnr`,
`veranlagung=zusammen`. dev-1 baut die Ring-Seite (golden/runner + Haut). **Kein Code — Recon.**

## Recon-Befund: der Scope nimmt Roh-Werte PRO PERSON, §9a intern

`festzusetzende_est_zusammen` (Catala, `FestzusetzendeEstZusammenIn`) nimmt:
`bruttoarbeitslohn_a`, `bruttoarbeitslohn_b`, `werbungskosten_a`, `werbungskosten_b`,
`sonderausgaben_gemeinsam`, `veranlagungszeitraum`. Also **Roh-Bruttolohn + Roh-WK PRO Person**
(nicht joint-bereinigt).

**KORREKTUR zum dev-2-Befund („§9a per Person = slot_fn-Sache"):** der Arbeitnehmer-Pauschbetrag
(§ 9a, 1230 €) wird per Person **IM Scope** angewandt — handverifiziert: `zus(40000,40000, WK 500/500)
== zus(…, WK 0/0) = 13838` (WK 500 < 1230 → Pauschbetrag greift intern, wie bei `festzusetzende_est_einzel`).
⇒ **KEIN § 9a in der slot_fn.** Die slot_fn liefert nur die Roh-WK pro Person (via `catala_werbungskosten_n`);
Pauschbetrag-Günstiger + Splitting macht der amtliche Catala-Tarif. Doktrin gewahrt (wie EP/dHf/einzel).

**Splitting-Vorteil handverifiziert (VZ2025):** ungleiche Einkommen 60000+20000 → zusammen 13838 €
gegenüber einzel(60000)+einzel(20000) = 15251 € = **Vorteil 1413 €**. Gleiche Einkommen 40000+40000 →
zusammen 13838 = 2× einzel(40000) (Vorteil 0, Progressionsvorteil nur bei Ungleichheit). Rechnung korrekt.

## Zuschnitt (Ring-Seite, dev-1)

**Engine (golden/runner):** neuer Accessor `catala_est_zusammen(s)` → `festzusetzende_est_zusammen`
(analog `catala_est`-Dispatcher, Trigger `bruttoarbeitslohn_a` bzw. `veranlagung==zusammen` + Partner-Feld).
Ausgabe EURO → Naht-Cent via `nach_cent(quantitaet="festzusetzende_est")` (Key belegt).

**Haut (an_gesamt, veranlagung-Weiche):** die `festzusetzende_est`-slot_fn verzweigt bei
`veranlagung==zusammen`:
```
wk_a = catala_werbungskosten_n(person-A-slots: EP + dHf)   # roh, § 9a im Scope
wk_b = catala_werbungskosten_n(person-B-slots)             # MVP: keine Partner-WK-Felder → wk_b = 0
so   = vorsorge_a + vorsorge_b                             # sonderausgaben_gemeinsam
-> catala_est_zusammen(bruttoarbeitslohn_a, bruttoarbeitslohn_b, wk_a, wk_b, so, vz)
```
`bruttoarbeitslohn_a` = bestehendes `bruttoarbeitslohn` (cent→euro), `bruttoarbeitslohn_b` =
`bruttoarbeitslohn_partner`. Scheibe: veranlagung-Weiche IN `an_gesamt` (kein zweiter Scheiben-Typ) —
einzel → `festzusetzende_est_einzel`, zusammen → `festzusetzende_est_zusammen`.

## Goldens (handverifiziert, VZ2025)

| Golden | Fall | erwartet |
|---|---|---|
| G-ZUS-A | gleich 40000+40000, WK 0, so 0 | **festzusetzende_est_zusammen 13838** (= 2× einzel, Kontrolle) |
| G-ZUS-B | ungleich 60000+20000, WK 0, so 0 | **13838** (vs einzel-Summe 15251 → Splitting-Vorteil 1413) |
| G-ZUS-C | § 9a-Beleg 40000+40000, WK 500/500 | **13838** (Pauschbetrag per Person intern) |

## K2-Guard (beide Personen)

Der zusammen-Ring rechnet NUR bei vollständig bestätigtem Kegel BEIDER Personen: ein Person-B-Feld
(`bruttoarbeitslohn_partner`, `person_b_idnr`, Partner-Vorsorge) vorläufig/offen → Ring unavailable
(`grund=partner_kegel_offen`), kein halber Splitting-Bescheid. Spiegelt dev-2s fail-closed A∪B
(deklarations-seitig) im Ring.

## Offene Entscheide (zur Abnahme)

1. **§9a-Korrektur bestätigen:** slot_fn liefert Roh-WK pro Person, Pauschbetrag im Scope (nicht Haut). OK?
2. **Person-B-WK:** MVP = wk_b 0 (keine Partner-EP/dHf-Felder in dev-2s Deklaration) → Pauschbetrag per
   Person intern. Partner-WK (EP/dHf für B) als eigene Erweiterung später — ok?
3. **Vorsorge-Deckelung bei Zusammenveranlagung (§ 10 Abs. 3):** `sonderausgaben_gemeinsam` = per Person
   gedeckelt (`_vorsorge_abzug(a) + _vorsorge_abzug(b)`, je einzel-HB) ODER gemeinsamer HB (2× HB)? Für
   MVP-Goldens so=0 irrelevant; Klärung vor VOR-im-zusammen-Ring. Dein Wort.
4. **Accessor vs Dispatcher:** eigener `catala_est_zusammen` (Empfehlung) oder `catala_est`-Zweig?
5. **veranlagung-Weiche IN an_gesamt** (Empfehlung) oder eigene Scheibe `an_gesamt_zusammen`?

Nach Abnahme: `catala_est_zusammen` + Goldens (dev-1), veranlagung-Weiche in der slot_fn + Partner-
Kegel-Guard + e2e (Splitting-Vorteil sichtbar). Kollisionsfrei mit dev-2s Verpflegungs-Tage-Bindung.
