# Recon kombiniert §19+§21 — Decision-Gate §10c SA-Pauschbetrag

**Datum:** 2026-07-18 · **Session:** dev-1 · **Stand:** VOR Bau, GO empfangen (msg 2716), Bau angehalten wegen Fund

## TL;DR
Der GO-Plan ist bau-fähig, ABER die Recon fand eine **pfad-abhängige §10c-Divergenz**, die den
Golden-Zielwert bestimmt UND committete Arbeit (vv_gesamt) berührt. Entscheidung nötig vor Bau.

## Der zitierte mehrarten-Golden belegt NICHT die Ring-Summe (Semantik-Unterschied)
GO-Plan zitierte `gesamt_2026_einzel_mehrarten` (est 13747) als Beleg. Der Golden füttert aber:
- `einkuenfte_nichtselbststaendig: 40000` **roh** (= schon §9a-netto §19-Einkünfte), `sonderausgaben: 36`, **VZ 2026**.

Mein kombinierter Ring nimmt **bruttoarbeitslohn** (brutto) und re-nettet via §9a:
`est_einzel(40000).summe_der_einkuenfte = 38770` → `catala_gesamt(ns=38770, vv=18770)` (VZ 2025).
Der mehrarten-Golden gehört zu einer **anderen Eingabe-Konvention**; sein Wert ist kein Ring-Zielwert.

## Fund: §10c Sonderausgaben-Pauschbetrag (36€) — pfad-abhängig
Verifiziert (VZ 2025, opam-Env):

| Weg | §19-Lohn 40000, sonst nichts | §10c 36€? |
|---|---|---|
| `est_einzel(40000).festzusetzende_est` (= an_gesamt-Ring) | **6919** | ja (intern) |
| `catala_gesamt(ns=38770, SA=0)` (= vv_gesamt/kombiniert) | **6930** | nein |
| `catala_gesamt(ns=38770, SA=36)` | 6919 | = einzel |

`6930 − 6919 = 11€ = §10c-Pauschbetrag (36€ zvE × Grenzsteuer)`. Bestätigt: `catala_gesamt` wendet
§10c NICHT automatisch an (nimmt `sonderausgaben` explizit), `est_einzel` schon.

**Betrifft auch committet:** `vermieter_only_2025_einzel` (vv=30000, SA=0 → 4303, d013d5a) fällt §10c
ebenfalls. Reiner Vermieter vv=18770: gesamt(SA=0)=1336 vs gesamt(SA=36)=1327 → committeter Ring
9€ zu hoch ggü. §10c-korrekt.

**§2 Abs.3-Struktur:** SA wird EINMAL auf Personen-Ebene nach Summe aller Einkunftsarten abgezogen.
→ §10c-korrekter Weg = `catala_gesamt(ns+vv, sonderausgaben=36)`.

## K2-Konsequenz
Gleicher §19-Lohn wird pfad-abhängig verschieden besteuert: 6919 (an_gesamt, Job-only) vs 6930
(kombiniert, Job+Vermietung). Das ist ein user-sichtbarer Fidelity-Bruch (falscher Bescheid um 11€
im kombinierten Fall, 9€ im reinen-Vermieter-Fall).

## Verlust-Verrechnung (K2-Kern, unabhängig vom §10c-Gate — funktioniert)
- nur §19: gesamt(38770, 0) = 6930
- §21-Verlust −5000: gesamt(38770, −5000) = **5399** (< 6930 → Verlust mindert Lohn ✓)
- großer Verlust −50000: gesamt(38770, −50000) = **0** (K2: Floor, keine Negativsteuer ✓)

## Optionen
**A** — Ring folgt committeter vv_gesamt-Konvention (SA=0, kein §10c) → Golden 13466.
Konsistent mit vv_gesamt, ABER an_gesamt bleibt Ausreißer (§19-Lohn 11€ niedriger). Kein neuer Param.

**B (empfohlen, §2-Abs.3-korrekt)** — alle gesamt-Ringe gewähren §10c: `catala_gesamt(..., sonderausgaben=36)`
→ Golden 13452. Konsistent mit an_gesamt (§19 gleich besteuert ob allein/kombiniert). Braucht:
(1) Param `sa_pauschbetrag_p10c.yaml` je VZ mit Anker (§10c EStG), Anker VORAB zu Instructor;
(2) Nachzieh-Fix für committeten vv_gesamt-Ring (+ vermieter_only/vermieter_verlust Goldens).

**C** — §10c als est_einzel-Artefakt behandeln, SA=0 überall Modell-Grenze, an_gesamt-Ring angleichen.
Widerspricht §10c EStG (Pauschbetrag steht jedem zu) → K2-schwach.

## Frage an Instructor
Welche Option? Bei B: §10c-Anker (§10c EStG, 36€ je VZ 2024/25/26 — Wert-Konstanz prüfen) vorab
freigeben, und ob der vv_gesamt-Nachzieh-Fix in denselben Freeze oder separat.
