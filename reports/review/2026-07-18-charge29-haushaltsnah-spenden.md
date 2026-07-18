# Charge 29 — §35a Haushaltsnahe + §10b Spenden: PROMOTION + WIRING (Stufe A, 2026-07-18)

**KORREKTUR nach Registry-Prüfung:** die 2 Regeln sind KEINE Neu-Formalisierung — sie existieren
BEREITS als **verified_bedingt-Snapshots** (`pipeline/snapshots/p35a_2_3_haushaltsnahe.json` module
`Haushaltsnahe`, `pipeline/snapshots/p10b_spenden.json` module `SpendenAbzug`), voll judge-durchlaufen,
item_registry voll triagiert (offene Discoveries = 0). §35a/§10b sind **inert nur weil die verified_
bedingt-Kandidaten nie in die Engine promoted/verdrahtet wurden** — nicht weil sie fehlen. Charge 29 =
**Promotion (Snapshot → rules/estg live, p35c-Muster) + Wiring (Accessor + Scheibe)**. Kein Pipeline-
Re-Run, keine Cap-Kosten.

## Befund — die Regeln (verified_bedingt, geprüft)

### Regel 1 `p35a_2_3_haushaltsnahe` (module `Haushaltsnahe`), queue_status verified_bedingt
```
input minijob_aufwendungen / haushaltsnahe_dienstleistungen / handwerker_arbeitskosten : money
ermaessigung_abs1 = min(0.20·minijob;      $510)     # § 35a Abs. 1
ermaessigung_abs2 = min(0.20·dienstleist.; $4000)    # § 35a Abs. 2
ermaessigung_abs3 = min(0.20·handwerker;   $1200)    # § 35a Abs. 3
steuerermaessigung = abs1 + abs2 + abs3              # 3 additive Töpfe, eigene Deckel
```
- **Caps IN-RULE** (nicht params) — akzeptiert wie p35c-Präzedenz. Gegen frozen `estg_p35a_2026-07-09`
  verifiziert: 510/4000/1200 € + 20 % korrekt (Abs. 1/2/3). Fassungs-Check: stabil seit 2009, kein
  Änderungsgesetz 2024–2026 → VZ-unabhängig; §35a-Cap-Watch geht an Task #12 (Fassungs-Watch), NICHT
  params-Refactor (unbegründeter Re-Run).
- **judge faithful=false = getriaged Artefakt:** einzige Abweichung „20 % auf Gesamtbetrag statt nur
  Arbeitskosten" → Julius triagierte als Geltungsbedingung `*_enthaelt_nur_arbeitskosten` (Material
  wird UPSTREAM am Sachverhalt ausgeschlossen; Felder heißen `*_arbeitskosten`, Fragetext „ohne
  Material"). Kein Baufehler. Vgl. [[altfassung-aenderungsbefehl-judge-artefakt]]-Klasse.
- **Geltungsbedingungen (verified_bedingt-Auflagen, Wiring erzwingt):** `rechnung_und_unbare_zahlung`
  (Abs. 5 S. 3, NUR Abs. 2/3), `*_enthaelt_nur_arbeitskosten` (Abs. 5 S. 2), `haushalt_in_eu_ewr`
  (Abs. 4), `keine_beruecksichtigung_als_wk_sa_agb` (Abs. 5 S. 1), `handwerker_keine_oeffentliche_
  foerderung` (Abs. 3 S. 2), `kein_gemeinsamer_haushalt_zweier_alleinstehender` (Abs. 5 S. 4).

### Regel 2 `p10b_spenden` (module `SpendenAbzug`), queue_status verified_bedingt, abweichungen = []
```
input zuwendungen / gesamtbetrag_der_einkuenfte : money
spenden_abzug = min(zuwendungen; 0.20·gesamtbetrag_der_einkuenfte)   # § 10b Abs. 1 S. 1 Nr. 1
```
- Gegen frozen `estg_p10b_2026-07-13` verifiziert: 20 % GdE (Alt. 1). Sauber, keine Abweichung.
- **Geltungsbedingungen:** `gde_ist_p2_ergebnis` (Basis = Gesamtbetrag d. Einkünfte aus est-Rechnung,
  VOR Sonderausgaben → keine Zirkularität — bestätigt dev-1s GdE-Naht), `nur_zwanzig_prozent_gde_deckel`
  (4-‰-Umsatz-Alternative = Nachtrag).

## Promotions-/Wiring-Rezept (dev-1, p35c-Muster)

1. **Materialisieren** (Snapshot catala_a → live): `rules/estg/p35a/haushaltsnahe.catala_en` +
   `rules/estg/p10b/spenden.catala_en` aus dem geprüften catala_a, je mit `test`-Scope (Seeds unten,
   `assertion`-Form wie p35c `tests_*`). Module `Haushaltsnahe` / `SpendenAbzug`.
2. **Integration-Punkt existiert:** `p32a/einkommensteuertarif.catala_en:476 input steuerermaessigungen
   # § 35a/§ 35c/…` (Haushaltsnahe.steuerermaessigung dockt hier an) + `:469 input sonderausgaben
   # § 10b` (SpendenAbzug.spenden_abzug in den Sonderausgaben-Topf). Kein neuer Scope-Input nötig.
3. **Accessor** produkt/: `catala_p35a_haushaltsnahe` → Haushaltsnahe.steuerermaessigung;
   `catala_p10b_spenden` → SpendenAbzug.spenden_abzug (GdE aus est_einzel).
4. **Scheibe** `haushalt_gesamt` (§19-Basis + §35a + §10b), Auflagen:
   - **rechnung_unbar = conditional-mandatory Kegel-Feld** wenn dienstleistung/handwerker > 0 (NICHT
     Minijob); unbeantwortet → vorlaeufig; explizit false → Abs2/3-Ermäßigung 0 justiziert (Anker
     Abs. 5 S. 3, folgbar), Minijob unberührt. [Q3 aus voriger Runde]
   - **§35a-ESt-Deckelung (S6, „vermindert um sonstige Steuerermäßigungen", Überhang verfällt):** der
     festsetzung-scope muss steuerermaessigungen so verrechnen, dass festzusetzende ESt nicht < 0
     (§35a nicht erstattungsfähig). ⚠ VERIFY-AUFLAGE beim Wiring: prüfen ob der Tarif-Scope das schon
     floored (min(ermäßigung; verfügbare ESt)); falls nicht → Guard. K2.

## Seeds (Materialisierungs-Tests)

**Haushaltsnahe:** (minijob 2800,0,0)→510 · (0,0,handwerker 4500)→900 · (0,0,handwerker 10000)→1200 ·
(0,dienstl 3000,0)→600 · (minijob 2800,0,handwerker 10000)→1710 · (0,0,0)→0.
*(rechnung_unbar-Guard NICHT im Modul — Geltungsbedingung, Scheibe erzwingt; Modul rechnet Roh-Ermäßigung.)*

**SpendenAbzug:** (zuw 15000, GdE 50000)→10000 · (5000,50000)→5000 · (10000,50000)→10000 · (0,50000)→0.

## Benannte Nachträge Charge 29

§ 35a Abs. 2 S. 2 Pflege/Heim · Abs. 5 S. 4 haushaltsbezogener Höchstbetrag (zwei Alleinstehende) ·
§ 10b Abs. 1 S. 1 Nr. 2 (4-‰-Betrieb) · Abs. 1 S. 9 Spendenvortrag · Abs. 1a Stiftung (1/2 Mio) ·
Abs. 2 Parteispenden (3300/6600 + § 34g) · Abs. 3 Sachzuwendungen. Person-B (§35a/§10b bei zusammen,
GdE = A+B) = Folge wie §19-B.

## Offene Punkte

1. **Promotion blessed?** beide Snapshots verified_bedingt, offene Discoveries = 0, Caps quell-verifiziert
   → ich segne die Promotion. (Reviewer-Rolle: finale Ratsche-Freigabe Julius, aber autonom-Mandat +
   verified_bedingt-Zustand → dev-1 baut, ich verifiziere die Materialisierung 1:1 gegen catala_a.)
2. **regel_id-Altname** `p35a_2_3_haushaltsnahe` bleibt (Snapshot/Bindung/item_registry konsistent) —
   der Modul-Name `Haushaltsnahe` trägt die Semantik, kein Rename nötig (revidiert ggü. erster Fassung).
3. **§35a-ESt-Deckelung** Verify-Auflage beim Wiring (Punkt 4 oben) — K2-kritisch.
4. **Kachel-Scope** `haushalt_gesamt` = nur §35a+§10b; agB §33 (p33_1_2, auch Snapshot!) + KiSt
   (p10_1_4, auch Snapshot!) = separate spätere Kachel (dev-1-Rückfrage offen).
