# Charge 13 — Nachtrags-Sammelcharge (Zuschnitt, Stufe A, 2026-07-14)

Schließt die benannten **Rechen-Nachträge** des Vollabdeckungs-Programms (Abschluss­bericht
2026-07-14, Entscheidungsblock Punkt 1). Der AN-nahe Nenner ist mit Charge 12 auf 40/40 = 100 %.
Charge 13 fügt keinem neuen Regelungsbereich etwas hinzu, sondern rundet drei bereits benannte,
**formalisierbare** Nachträge innerhalb bestehender Bereiche ab und bestätigt einen vierten als
Nicht-Gegenstand.

Messbasis: `reports/review/2026-07-13-coverage-landkarte.md` (Abschnitt „Nicht-Gegenstand" +
„Offene Ebene"). Kein Nenner-Zuwachs — Charge 13 verdichtet den Reifegrad.

## Umfang (Instructor-Freigabe 2026-07-14: „Abschlussbericht folgen", § 34 Abs 3 = rein)

| # | Nachtrag | § | Disposition | Regel |
|---|---|---|---|---|
| 1 | Privater Veräußerungs-Verlusttopf (gleichjährig) | § 23 Abs. 3 S. 7 | **formalisieren** | `p23_3_verlusttopf` |
| 2 | KiSt-ermäßigte Abgeltung (Formel e−4q / 4+k) | § 32d Abs. 1 S. 3–5 | **formalisieren** | `p32d_1_kirchensteuer` |
| 3 | Ermäßigter Durchschnittssatz 56 % | § 34 Abs. 3 | **formalisieren** | `p34_3_ermaessigter_durchschnittssatz` |
| 4 | Prognosekorridor verbilligte Vermietung 50–<66 % | § 21 Abs. 2 | **Nicht-Gegenstand (bestätigt)** | — (Bedingung an `p21_2`) |

## 1 — § 23 Abs. 3 S. 7: privater Veräußerungs-Verlusttopf

**Wortlaut (Zitatanker `Verluste dürfen nur bis zur Höhe des Gewinns`):** „Verluste dürfen nur
bis zur Höhe des Gewinns, den der Steuerpflichtige im gleichen Kalenderjahr aus privaten
Veräußerungsgeschäften erzielt hat, ausgeglichen werden; sie dürfen nicht nach § 10d abgezogen
werden."

- **Präzedenz:** `p20_6_verlustverrechnung` (Topf-Trennung § 20 Abs. 6). Gleiche Mechanik: ein
  Netto-Verlust ist quarantänisiert, wirkt nicht gegen andere Einkunftsarten.
- **Signatur** `PvgVerlusttopf`: `gewinn_pvg: money`, `verlust_pvg: money` (beide nicht-negativ,
  Verlust als positiver Betrag) → `anzusetzende_einkuenfte: money`.
- **Rechenkern:** `anzusetzende_einkuenfte = max(0; gewinn_pvg − verlust_pvg)`. Keine Division,
  keine Rundung.
- **Geltungsbedingungen:** `verlust_nur_gleichjaehriger_pvg_gewinn` (S7 Hs1),
  `kein_ausgleich_mit_anderen_einkunftsarten` (S7 Hs2, kein § 10d),
  `kein_verlustvor_ruecktrag_s8` (S8 Vor-/Rücktrag = Mehrjahres-State, ausgeklammert wie § 10d
  Abs. 1).
- **Abgrenzung:** die 1.000-€-Freigrenze auf den *Gesamtgewinn* (S5) ist `p23_freigrenze` —
  nachgelagert, eigene Regel. S7 liefert den gejahrsnetteten Betrag, auf den die Freigrenze wirkt.
- **Seeds (synthetisch, hier gerechnet, Julius-Review):** (2000/500)→1500 · (500/800)→0
  Verlust quarantänisiert · (1000/1000)→0 · (0/0)→0.

## 2 — § 32d Abs. 1 S. 3–5: KiSt-ermäßigte Abgeltungsteuer

**Wortlaut (Zitatanker `e – 4 q`):** „Im Fall der Kirchensteuerpflicht ermäßigt sich die Steuer …
um 25 Prozent der auf die Kapitalerträge entfallenden Kirchensteuer. Die Einkommensteuer beträgt
damit **(e – 4q) / (4 + k)**. Dabei sind „e" die nach … § 20 ermittelten Einkünfte, „q" die …
anrechenbare ausländische Steuer und „k" der … geltende Kirchensteuersatz."

- **Präzedenz/Andockung:** `p32d_1_abgeltung` benennt genau diesen KiSt-Fall als „Nachtrag b"
  (dort Bedingung `keine_kirchensteuer_auf_kapitalertraege`). Charge 13 liefert den Nachtrag —
  komplementär.
- **Signatur** `KapitalAbgeltungKirchensteuer`: `kapitaleinkuenfte: money` (e),
  `anrechenbare_auslaendische_steuer: money` (q), `kirchensteuersatz: decimal` (k) →
  `kapital_steuer: money`.
- **Rechenkern:** `kapital_steuer = (kapitaleinkuenfte − 4·q) / (4 + kirchensteuersatz)`. k ist
  bereits Dezimalsatz (0,09 bzw. 0,08 — Landessatz), nicht /100. Die beiden 4er sind exakte
  Strukturkonstanten. Cent-Schnitt zuletzt.
- **Geltungsbedingungen:** `kirchensteuerpflicht_besteht` (S3),
  `keine_guenstigerpruefung_beantragt` (Abs. 6, `p32d_1`-Präzedenz),
  `kapitaleinkuenfte_nicht_negativ` (negative KAP → `p20_6`).
- **Seeds (exakt, ohne Rundungsabhängigkeit):** e=4090,q=0,k=0,09 → 1000,00 · e=8180 → 2000,00 ·
  e=4490,q=100,k=0,09 → (4090)/4,09 = 1000,00 (q-Wächter) · e=4080,k=0,08 → 1000,00 (Landessatz) ·
  e=0 → 0,00. Alle Seeds sind absichtlich glatt gewählt, damit die (steuerfachlich offene)
  Cent-Rundungsrichtung sie nicht beeinflusst.

## 3 — § 34 Abs. 3: ermäßigter Durchschnittssatz (56 %)

**Wortlaut (Zitatanker `56 Prozent des durchschnittlichen Steuersatzes`):** „… auf Antrag …
die auf den Teil dieser außerordentlichen Einkünfte, der den Betrag von insgesamt 5 Millionen
Euro nicht übersteigt, entfallende Einkommensteuer nach einem ermäßigten Steuersatz … wenn der
Steuerpflichtige das 55. Lebensjahr vollendet hat oder … dauernd berufsunfähig ist. Der ermäßigte
Steuersatz beträgt **56 Prozent des durchschnittlichen Steuersatzes**, der sich ergäbe, wenn die
tarifliche Einkommensteuer nach dem gesamten zu versteuernden Einkommen zuzüglich der dem
Progressionsvorbehalt unterliegenden Einkünfte zu bemessen wäre, **mindestens jedoch 14 Prozent**."

- **Landkarten-Vorbehalt:** § 34 Abs. 3 ist dort als „AN-fern → benannter Nachtrag" markiert
  (antragsabhängig, einmal im Leben, i. d. R. Betriebsveräußerung). Der Abschlussbericht empfiehlt
  ihn dennoch für Charge 13; Instructor-Freigabe 2026-07-14 = **rein**. Er hat klaren Wortlaut und
  dockt strukturgleich zu § 34 Abs. 1 an — die AN-Ferne betrifft die Häufigkeit, nicht die
  Formalisierbarkeit.
- **Präzedenz/Andockung:** `p34_fuenftel_ao_est` und `p32d_1` (Tarifwerte als Inputs, kein
  Selbst-Tarif).
- **Signatur** `ErmaessigterDurchschnittssatz`: `ao_einkuenfte: money` (Abs. 2 Nr. 1),
  `est_gesamt_zzgl_progression: money` (p32a-Ergebnis, Input),
  `bemessungsgrundlage_durchschnitt: money` (zvE gesamt inkl. ao zzgl. Progressionseinkünfte,
  Input) → `est_ao: money`.
- **Rechenkern:** `durchschnittssatz = est_gesamt_zzgl_progression / bemessungsgrundlage_durchschnitt`;
  `ermaessigter_satz = max(0,56 · durchschnittssatz; 0,14)`;
  `est_ao = ermaessigter_satz · min(ao_einkuenfte; 5.000.000 €)`.
- **Geltungsbedingungen:** `antrag_gestellt` (S1), `persoenliche_voraussetzung_erfuellt`
  (55. LJ / Berufsunfähigkeit, S1), `ao_einkuenfte_abs2_nr1` (nur Veräußerungsgewinne, NICHT
  Nr. 2–4 → die laufen Fünftel), `einmal_im_leben` (S4, Einmaligkeits-State wie § 35c-40k),
  `tarifwerte_sind_p2_ergebnis`, `bemessungsgrundlage_positiv`.
- **Seeds (exakt):** DS 0,30 → 0,168 · ao 100.000 → 16.800,00 · 14-%-Boden (0,112<0,14) ao 50.000
  → 7.000,00 · 5-Mio-Cap ao 6 Mio → 840.000,00 · ao 0 → 0,00.
- **Benannt offen (weiter Nachtrag):** S3 (Rest-zvE regulär besteuern) ist § 2-Integration; die
  Interaktion mit dem Progressionsvorbehalt-Input ist als Bemessungsgröße deklariert, nicht hier
  gerechnet.

## 4 — § 21 Abs. 2: Prognosekorridor 50–<66 % (Nicht-Gegenstand, bestätigt)

Der Abschlussbericht listet „§ 21-Abs2-Korridor-Prüfung" für Charge 13. Prüfung ergibt: **kein
Norm-Wortlaut zu formalisieren.** § 21 Abs. 2 regelt nur die beiden Ränder (S1: < 50 % → anteilige
Aufteilung; S2: ≥ 66 % → voll entgeltlich). Das Zwischenband 50–< 66 % hängt an der
**Totalüberschussprognose** (BMF-Verwaltungsanweisung, Rechtsprechung) — eine externe
Ermessens-/Datenquelle, kein Gesetzeswortlaut. Genau das ist bereits als Geltungsbedingung
`entgelt_quote_ausserhalb_prognosekorridor` an `p21_2_verbilligte_vermietung_wk` (Charge 10)
benannt und dort ausdrücklich als „kein Wortlaut, BMF-Prognose" markiert.

**Disposition (AINA):** keine Regel. Etwas ohne Norm-Wortlaut zu „formalisieren" hieße, eine
BMF-Prognose-Heuristik zu erfinden — das verbietet das Leitprinzip. Der Korridor bleibt benannter
Nicht-Gegenstand; er wird erst mit einer eingefrorenen BMF-Quelle als eigener Nachtrag
wiedervorgelegt. Charge 13 bestätigt und dokumentiert diese Grenze, sie erzeugt keinen Code.

## Stufe B — Verifikation über die eingefrorene Kaskade

Besetzung eingefroren (`pipeline/models.yaml`): A = Sonnet 4.6, B = GLM-5.2, Judge =
mistral-medium-3-5, Worker = DeepSeek V4 Flash. DoD je Regel unverändert (Manifest-Kopf):
Norm-Freeze + Zitatanker, Doppelformalisierung A/B mit extensionaler Äquivalenz auf dem Raster,
Round-Trip ohne stille Zusatzannahme, `scope_gap` ohne `wirkt_hinein`-Norm-Teil, **Clerk-Test-Gate
Pflicht** (Seeds oben, Herkunft synthetisch → Julius-Review dieses Reports), Human-Review.

Quellen sind eingefroren und sha256-geprüft (`make sources-check`): § 23, § 32d, § 34, § 21 alle
im Bestand (`sources/gesetze-im-internet/estg_p{23,32d,34,21}_2026-07-13.txt`). Das Quellen-Gate
(Zitatanker + Auszug wörtlich) läuft VOR jedem Modell und ist grün.

Discovery-Funde der Kaskade (stille Zusatzannahmen, Abweichungskandidaten) landen in der
Discovery-Queue und warten auf Instructor-Triage in die Item-Registry
(`pipeline/item_registry/`) — nur die Triage kippt Verdikte (Registry-Ratsche).

## Stufe-B-Ergebnis (Lauf 2026-07-14, $0,3793, wall 311s)

Alle drei Regeln: **jedes deterministische Gate grün** — syntax/typecheck A+B, `rundungs_lint`,
`praezisions_lint`, **`equivalence` (A≡B auf dem Raster)**, `roundtrip`, `scope_gap`,
`geltungsbereich`, `grenzfall`, `defekt`, **`clerk` (Seeds oben)**. Keine offenen Gates.

| Regel | Kosten | Gates | Status | Discovery-Items |
|---|---|---|---|---|
| `p23_3_verlusttopf` | $0,0508 | alle grün | **verified_bedingt** (3 Bed.) | 4 triagiert (2 nicht_material, 2 bedingung_neu) |
| `p32d_1_kirchensteuer` | $0,0973 | alle grün | **verified_bedingt** (3 Bed.) | 8 triagiert (5 nicht_material/backlog + 3 Detektor-Whitelist) |
| `p34_3_ermaessigter_durchschnittssatz` | $0,2311 | alle grün | **verified_bedingt** (6 Bed.) | 16 triagiert (inkl. 3 abweichung → nicht_echt) |

Alle drei **`verified_bedingt`, keine offenen Gates.** `p23_3` war reine Whitelist (Dev-Delegation
seit Charge 10). Bei `p32d`/`p34` wurden die `offen`-Items — insbesondere die 3 `abweichung` bei
§ 34 — erst nach unabhängiger Zweitmeinung adjudiziert (s. u.).

### Zweitmeinung (unabhängiges Modell, Instructor-Anweisung 2026-07-14)

Entscheidungsregel Julius: „lass ein anderes Modell drüberschauen; stimmt Claudes Einschätzung
mit ihm überein → grün." Gewählt: **`openai/gpt-5.5`** (Provider `openai` gepinnt, `allow_fallbacks`
false, temp 0) — eine vierte Modellfamilie, weder Claude (Analyse) noch mistral (Detektor, der die
Flags setzte) noch die Formalisierer A/B. Neutraler Prompt (Norm + generierter Catala + Roh-Flags,
**ohne** Vorurteil). Rohdaten: `pipeline/item_registry/discovery/charge13/adjudikation-gpt-5.5-crosscheck.json`.

**Ergebnis: volle Übereinstimmung.** gpt-5.5 = `alle_fehlalarm` (§ 34) / `alle_nicht_material`
(§ 32d) — identisch zu meiner Einschätzung, Item für Item, mit derselben Begründung. Daher gemäß
Julius' Regel grün gezogen.

### Adjudikations-Grundlage (bestätigt durch beide Modelle)

**`p32d_1_kirchensteuer` — 5 `offen`, keine `abweichung`.** Alle fünf sind normkonforme
Restatements der Formel-Zuordnung (e = § 20-Einkünfte, q = anrechenbare ausl. Steuer, k =
KiSt-Satz; Formel (e−4q)/(4+k)) bzw. der Norm-Teil § 32d Abs 1 S 4 selbst. Kandidat: alle
`nicht_material` (Input-Etikett/normkonforme Interpretation, `konv:`-Muster wie p23_freigrenze) —
der Detektor hat sie nur nicht mit `konv:` getaggt. Empfehlung: `nicht_material`.

**`p34_3` — 3 `abweichung` + 7 `offen`-Annahmen/Norm-Teile.** Generierter Catala (A) verifiziert:
`cap = min(ao; 5.000.000)`, `durchschnittssatz = est_gesamt / bemessungsgrundlage`,
`ermaessigter_satz = max(0,56·ds; 0,14)`, `est_ao = cap · ermaessigter_satz` — **wortgetreu**.
Die 3 Abweichungen sind das dokumentierte mistral-Über-Flag-Muster (reife Designs / Sondersatz-
Überlese, models.yaml):

1. „Satz auf Bemessungsgrundlage (cap) statt auf ao-Einkünfte" → `cap = min(ao; 5 Mio)` IST
   „der Teil der ao-Einkünfte, der 5 Mio nicht übersteigt" (S1 wörtlich). **Kandidat nicht_echt.**
2. „Durchschnittssatz = est/bemessung nicht normkonform" → das IST die Definition des
   durchschnittlichen Steuersatzes; Andockung nimmt est als p32a-Input (wie p34_1/p32d).
   **Kandidat nicht_echt.**
3. „fehlende Multiplikation mit der tariflichen ESt" → die Norm rechnet Satz × ao-Betrag, NICHT
   × tarifliche ESt; der Judge verwechselt. **Kandidat nicht_echt.**

`equivalence` (A≡B) und `clerk` (Wächter 5-Mio-Cap 840.000,00 und 14-%-Boden 7.000,00) sind grün —
starke Gegenbelege zu allen drei. Durch die unabhängige gpt-5.5-Zweitmeinung bestätigt (s. o.) und
gemäß Julius' Entscheidungsregel als `nicht_echt` triagiert. Drafts + Roh-Verdikt unter
`pipeline/item_registry/discovery/charge13/`.

## Kosten

Stufe A: $0. Stufe B: **$0,3793** (Schätzung ~$0,3 getroffen). OpenRouter-Restbudget des
Tageskeys danach ausreichend. Kein Wert aus dem Gedächtnis: alle Erwartungswerte sind aus dem
formalisierten Rechenkern abgeleitet (Herkunft `synthetisch`, Rechenweg im Manifest, dieser
Report als `quelle` für den Julius-Review der Seeds).
